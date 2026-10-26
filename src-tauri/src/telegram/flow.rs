use super::dates::{self, Local};
use super::engine::{token, Cx, FlowChange, Out, Pre, Turn};
use super::model::*;
use super::search::{self, Pick, Project};
use crate::agenda::models::{start_fields, utc, AgendaInput, MeetingRef};
use crate::workspace::models::*;
use chrono::{Duration, NaiveDate, NaiveTime};
use serde_json::{json, Value};
use sqlx::SqliteConnection;

pub const HELP: &str = "Here is what I can do:\n\
• Add: “add a task to send the invoice to Acme by Friday” or “schedule a design review Tuesday at 2pm for 45 minutes”.\n\
• Finish: “mark the invoice task done, sent it Monday” completes it and saves your note.\n\
• Cancel: “the Thursday standup is cancelled” marks it cancelled; say “delete” to remove it instead.\n\
• Look up: /today, /tomorrow, or “what's on Friday?”.\n\
• /cancel stops what I'm asking you, and /help shows this.\n\n\
I always show what I understood and wait for you to tap Confirm before saving anything.";

const PAGE: usize = 5;

pub async fn load(conn: &mut SqliteConnection, chat: i64) -> Result<Option<Flow>> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT state_json FROM telegram_flow WHERE chat_id=?")
            .bind(chat)
            .fetch_optional(conn)
            .await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()))
}

pub async fn save(conn: &mut SqliteConnection, chat: i64, flow: &Flow) -> Result<()> {
    sqlx::query("INSERT INTO telegram_flow(chat_id,state_json) VALUES(?,?) ON CONFLICT(chat_id) DO UPDATE SET state_json=excluded.state_json")
        .bind(chat)
        .bind(serde_json::to_string(flow).expect("flow serializes"))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn clear(conn: &mut SqliteConnection, chat: i64) -> Result<()> {
    sqlx::query("DELETE FROM telegram_flow WHERE chat_id=?")
        .bind(chat)
        .execute(conn)
        .await?;
    Ok(())
}

/// Records which Telegram message carries the current prompt, so it can later lose its buttons.
pub async fn remember_message(
    conn: &mut SqliteConnection,
    chat: i64,
    token: &str,
    message_id: i64,
) -> Result<()> {
    if let Some(mut flow) = load(conn, chat).await? {
        if flow.token == token {
            flow.message_id = Some(message_id);
            save(conn, chat, &flow).await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Entry point for one inbound update.

pub async fn turn(
    conn: &mut SqliteConnection,
    cx: &Cx,
    turn: Turn,
    existing: Option<Flow>,
    pre: Pre,
) -> Result<Out> {
    let mut out = Out::default();
    if let Some(failure) = &pre.failure {
        out.say(failure.clone());
        return Ok(out);
    }
    let expired = existing.as_ref().is_some_and(|f| cx.expired(f));
    let live = existing.clone().filter(|f| !cx.expired(f));
    if expired {
        out.flow = FlowChange::Clear;
    }
    match turn {
        Turn::Nothing => out.say("I can read text and voice notes. Try /help."),
        Turn::Command(cmd) => {
            if expired {
                out.say("Your earlier request expired, so nothing was changed.");
            }
            command(conn, cx, &mut out, &cmd, live).await?;
        }
        Turn::Text(text) => {
            if expired {
                out.say("Your earlier request expired, so nothing was changed.");
            }
            text_turn(conn, cx, &mut out, text, live, pre).await?;
        }
        Turn::Callback {
            id,
            data,
            message_id,
        } => {
            callback_turn(
                conn, cx, &mut out, &id, &data, message_id, existing, expired, pre,
            )
            .await?;
        }
    }
    Ok(out)
}

async fn command(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    cmd: &str,
    live: Option<Flow>,
) -> Result<()> {
    let name = cmd
        .split_whitespace()
        .next()
        .unwrap_or("")
        .split('@')
        .next()
        .unwrap_or("")
        .to_lowercase();
    match name.as_str() {
        "/start" | "/help" => out.say(HELP),
        "/cancel" => {
            if let Some(flow) = live {
                if let Some(id) = flow.message_id {
                    out.edit(id, "✖ Cancelled. Nothing was changed.");
                }
                out.flow = FlowChange::Clear;
                out.say("Cancelled. Nothing was changed.");
            } else {
                out.say("There's nothing to cancel.");
            }
        }
        "/today" => out.say(day_text(conn, cx, cx.today).await?),
        "/tomorrow" => out.say(day_text(conn, cx, cx.today + Duration::days(1)).await?),
        _ => out.say("I don't know that command. Try /help."),
    }
    Ok(())
}

async fn text_turn(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    text: String,
    live: Option<Flow>,
    pre: Pre,
) -> Result<()> {
    let Some(mut flow) = live else {
        return start_request(conn, cx, out, &text, pre).await;
    };
    match flow.stage.clone() {
        Some(Stage::Question(q)) => {
            if let Some(value) = typed_answer(&q, &text, cx) {
                answer(conn, cx, out, flow, &q, value).await
            } else {
                conflict(cx, out, flow, text);
                Ok(())
            }
        }
        Some(Stage::Revise(p)) => match pre.revision {
            Some(Ok(action)) => {
                flow.current = Some(action);
                flow.stage = None;
                drive(conn, cx, out, flow).await
            }
            Some(Err(e)) => {
                out.say(format!(
                    "{} Send another correction, or tap Cancel.",
                    plain(&e.message)
                ));
                let _ = p;
                Ok(())
            }
            None => {
                out.say("I couldn't apply that correction right now. Send another, or tap Cancel.");
                Ok(())
            }
        },
        Some(Stage::Confirm(_)) | Some(Stage::Conflict { .. }) | None => {
            conflict(cx, out, flow, text);
            Ok(())
        }
    }
}

async fn start_request(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    text: &str,
    pre: Pre,
) -> Result<()> {
    if let Some(heard) = &pre.heard {
        out.say(format!("🎙 Heard: “{}”", clip(heard, 600)));
    }
    let interp = match pre.interpretation {
        Some(Ok(i)) => i,
        Some(Err(e)) => {
            out.say(format!("{} Nothing was changed.", plain(&e.message)));
            return Ok(());
        }
        None => {
            out.say("I couldn't interpret that right now. Nothing was changed.");
            return Ok(());
        }
    };
    if interp.actions.is_empty() {
        out.say(interp.reply.unwrap_or_else(|| {
            "I couldn't find a task or meeting in that. Try /help for examples.".into()
        }));
        return Ok(());
    }
    let flow = Flow {
        token: token(),
        step: 0,
        message_time: cx.message_time.clone(),
        expires_at: cx.expiry(),
        text: text.to_owned(),
        current: None,
        queue: interp.actions,
        stage: None,
        parked: None,
        done: vec![],
        message_id: None,
    };
    drive(conn, cx, out, flow).await
}

/// A request arrived while another was waiting; ask which one to keep.
fn conflict(cx: &Cx, out: &mut Out, mut flow: Flow, text: String) {
    let incoming = Incoming {
        text,
        message_time: cx.message_time.clone(),
    };
    if let Some(Stage::Conflict { .. }) = flow.stage {
        flow.stage = Some(Stage::Conflict { incoming });
    } else {
        flow.parked = flow.stage.take().map(Box::new);
        flow.stage = Some(Stage::Conflict { incoming });
    }
    present(out, cx, &mut flow);
    out.flow = FlowChange::Set(Box::new(flow));
}

// ---------------------------------------------------------------------------------------------
// Buttons.

async fn callback_turn(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    id: &str,
    data: &str,
    message_id: i64,
    existing: Option<Flow>,
    expired: bool,
    pre: Pre,
) -> Result<()> {
    let parts: Vec<&str> = data.split('|').collect();
    let (Some(flow), ["k", tok, step, code]) = (existing, parts.as_slice()) else {
        out.answer(id, "That request is no longer active.");
        return Ok(());
    };
    if flow.token != *tok || flow.step.to_string() != *step {
        out.answer(id, "This button is out of date.");
        return Ok(());
    }
    if expired {
        out.answer(id, "This request expired. Nothing was changed.");
        out.edit(message_id, "⌛ This request expired. Nothing was changed.");
        out.flow = FlowChange::Clear;
        return Ok(());
    }
    out.answer(id, "");
    let code = *code;
    let stage = flow.stage.clone();
    match (code, stage) {
        ("y", Some(Stage::Confirm(p))) => confirm(conn, cx, out, flow, p, message_id).await,
        ("x", Some(Stage::Confirm(_) | Stage::Question(_) | Stage::Revise(_))) => {
            out.edit(message_id, "✖ Cancelled. Nothing was changed.");
            let mut flow = flow;
            flow.current = None;
            flow.stage = None;
            drive(conn, cx, out, flow).await
        }
        ("e", Some(Stage::Confirm(p))) => {
            out.edit(message_id, "✏️ Changing this request…");
            let mut flow = flow;
            flow.stage = Some(Stage::Revise(p));
            present(out, cx, &mut flow);
            out.flow = FlowChange::Set(Box::new(flow));
            Ok(())
        }
        ("k", Some(Stage::Conflict { .. })) => {
            out.edit(message_id, "👍 Continuing with your earlier request.");
            let mut flow = flow;
            flow.stage = flow.parked.take().map(|b| *b);
            present(out, cx, &mut flow);
            out.flow = FlowChange::Set(Box::new(flow));
            Ok(())
        }
        ("d", Some(Stage::Conflict { incoming })) => {
            out.edit(message_id, "🗑 Discarded your earlier request.");
            out.flow = FlowChange::Clear;
            let moved = Cx::new(
                cx.chat_id,
                &cx.tz_name,
                cx.now.clone(),
                incoming.message_time.clone(),
            )?;
            let mut pre = pre;
            pre.heard = None;
            start_request(conn, &moved, out, &incoming.text, pre).await
        }
        (c, Some(Stage::Question(q))) => question_button(conn, cx, out, flow, q, c).await,
        _ => {
            out.say("That button no longer applies.");
            Ok(())
        }
    }
}

async fn question_button(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    mut flow: Flow,
    mut q: Question,
    code: &str,
) -> Result<()> {
    if let Some(index) = code.strip_prefix('o').and_then(|n| n.parse::<usize>().ok()) {
        let Some(value) = q.options.get(index).map(|o| o.value.clone()) else {
            out.say("That choice isn't available any more.");
            return Ok(());
        };
        return answer(conn, cx, out, flow, &q, value).await;
    }
    if let Some(page) = code.strip_prefix('p').and_then(|n| n.parse::<usize>().ok()) {
        if page * PAGE < q.options.len().max(1) {
            q.page = page;
            flow.stage = Some(Stage::Question(q));
            present(out, cx, &mut flow);
            out.flow = FlowChange::Set(Box::new(flow));
        }
        return Ok(());
    }
    match (code, q.field.as_str()) {
        ("s", "task") => {
            flow.stage = Some(Stage::Question(search_question(
                "task_search",
                "Type part of the task's name to search again.",
            )));
        }
        ("s", "meeting") => {
            flow.stage = Some(Stage::Question(search_question(
                "meeting_search",
                "Type part of the meeting's name to search again.",
            )));
        }
        ("n" | "b", "meeting" | "meeting_search") => {
            if let Some(Action::CancelMeeting(c)) = flow.current.as_mut() {
                let step = if c.date.is_some() { 1 } else { 7 };
                let from = c
                    .window_start
                    .as_deref()
                    .and_then(dates::parse_date_iso)
                    .unwrap_or_else(|| window_origin(c, cx));
                let shifted = from + Duration::days(if code == "n" { step } else { -step });
                c.window_start = Some(shifted.to_string());
            }
            return drive(conn, cx, out, flow).await;
        }
        _ => {
            out.say("That button no longer applies.");
            return Ok(());
        }
    }
    present(out, cx, &mut flow);
    out.flow = FlowChange::Set(Box::new(flow));
    Ok(())
}

fn search_question(field: &str, prompt: &str) -> Question {
    Question {
        field: field.into(),
        prompt: prompt.into(),
        options: vec![],
        page: 0,
        text_ok: true,
    }
}

/// A typed message that clearly answers the open question, as the value to apply.
pub fn typed_answer(q: &Question, text: &str, cx: &Cx) -> Option<Value> {
    let t = text.trim();
    match q.field.as_str() {
        "project" => {
            let list: Vec<Project> = q
                .options
                .iter()
                .map(|o| Project {
                    id: o.value["id"].as_str().unwrap_or_default().to_owned(),
                    name: o.label.clone(),
                })
                .collect();
            match search::match_project(&list, t) {
                Pick::One(i) => Some(q.options[i].value.clone()),
                _ => None,
            }
        }
        "date" => dates::parse_date(t, cx.today).map(|d| json!(d.to_string())),
        "time" => dates::parse_time(t).map(|v| json!(dates::hhmm(v))),
        "task_search" | "meeting_search" => (!t.is_empty()).then(|| json!(t)),
        _ => None,
    }
}

async fn answer(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    mut flow: Flow,
    q: &Question,
    value: Value,
) -> Result<()> {
    let Some(action) = flow.current.as_mut() else {
        out.say("That request is no longer active.");
        return Ok(());
    };
    match (q.field.as_str(), action) {
        ("project", Action::AddTask(t)) => {
            t.project_id = value["id"].as_str().map(str::to_owned);
            t.project_name = q
                .options
                .iter()
                .find(|o| o.value == value)
                .map(|o| o.label.clone());
        }
        ("project", Action::AddMeeting(m)) => {
            m.project_id = value["id"].as_str().map(str::to_owned);
            m.project_name = q
                .options
                .iter()
                .find(|o| o.value == value)
                .map(|o| o.label.clone());
        }
        ("task", Action::CompleteTask(c)) => c.task_id = value["id"].as_str().map(str::to_owned),
        ("meeting", Action::CancelMeeting(c)) => {
            c.meeting_id = value["m"].as_str().map(str::to_owned);
            c.occurrence_key = value["k"].as_str().map(str::to_owned);
        }
        ("scope", Action::CancelMeeting(c)) => {
            c.mode = value["mode"].as_str().unwrap_or("cancel").to_owned();
            c.scope = value["scope"].as_str().map(str::to_owned);
        }
        ("date", Action::AddMeeting(m)) => m.date = value.as_str().map(str::to_owned),
        ("time", Action::AddMeeting(m)) => m.time = value.as_str().map(str::to_owned),
        ("time", Action::AddTask(t)) => {
            t.due_time = value.as_str().map(str::to_owned);
            t.time_defaulted = false;
        }
        ("fold", Action::AddMeeting(m)) => m.fold = value.as_str().map(str::to_owned),
        ("fold", Action::AddTask(t)) => t.fold = value.as_str().map(str::to_owned),
        ("task_search", Action::CompleteTask(c)) => {
            c.query = value.as_str().unwrap_or_default().to_owned();
            c.task_id = None;
        }
        ("meeting_search", Action::CancelMeeting(c)) => {
            c.query = value.as_str().unwrap_or_default().to_owned();
            c.meeting_id = None;
            c.occurrence_key = None;
            c.window_start = None;
        }
        _ => {
            out.say("That answer doesn't fit this request.");
            return Ok(());
        }
    }
    flow.stage = None;
    drive(conn, cx, out, flow).await
}

// ---------------------------------------------------------------------------------------------
// Moving through the queue of requests.

pub async fn drive(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    mut flow: Flow,
) -> Result<()> {
    loop {
        let action = match flow.current.take() {
            Some(a) => a,
            None => {
                if flow.queue.is_empty() {
                    break;
                }
                flow.queue.remove(0)
            }
        };
        match resolve(conn, cx, action).await? {
            Resolved::Ask(q, action) => {
                flow.current = Some(action);
                flow.stage = Some(Stage::Question(q));
                present(out, cx, &mut flow);
                out.flow = FlowChange::Set(Box::new(flow));
                return Ok(());
            }
            Resolved::Propose(p) => {
                flow.current = Some(p.action.clone());
                flow.stage = Some(Stage::Confirm(p));
                present(out, cx, &mut flow);
                out.flow = FlowChange::Set(Box::new(flow));
                return Ok(());
            }
            Resolved::Info(text) | Resolved::Fail(text) => out.say(text),
        }
    }
    out.flow = FlowChange::Clear;
    Ok(())
}

enum Resolved {
    Ask(Question, Action),
    Propose(Proposal),
    Info(String),
    Fail(String),
}

async fn resolve(conn: &mut SqliteConnection, cx: &Cx, action: Action) -> Result<Resolved> {
    match action {
        Action::AddTask(t) => resolve_task(conn, cx, t).await,
        Action::AddMeeting(m) => resolve_meeting(conn, cx, m).await,
        Action::CompleteTask(c) => resolve_complete(conn, cx, c).await,
        Action::CancelMeeting(c) => resolve_cancel(conn, cx, c).await,
        Action::Lookup(l) => {
            let date = l
                .date
                .as_deref()
                .and_then(dates::parse_date_iso)
                .unwrap_or(cx.today);
            Ok(Resolved::Info(day_text(conn, cx, date).await?))
        }
    }
}

enum Chosen {
    Project(Project),
    Ask(Question),
    Stop(String),
}

async fn choose_project(
    conn: &mut SqliteConnection,
    id: &Option<String>,
    name: &Option<String>,
) -> Result<Chosen> {
    let all = search::projects(conn).await?;
    if all.is_empty() {
        return Ok(Chosen::Stop(
            "You don't have any projects yet. Create one in Signal, then ask me again.".into(),
        ));
    }
    if let Some(p) = id.as_ref().and_then(|id| all.iter().find(|p| &p.id == id)) {
        return Ok(Chosen::Project(p.clone()));
    }
    let hint = name.as_deref().map(str::trim).filter(|n| !n.is_empty());
    if let Some(hint) = hint {
        if let Pick::One(i) = search::match_project(&all, hint) {
            return Ok(Chosen::Project(all[i].clone()));
        }
    }
    let prompt = match hint {
        Some(h) => format!(
            "I couldn't match “{}” to one of your projects. Which project is this for?",
            clip(h, 60)
        ),
        None => "Which project is this for?".to_owned(),
    };
    Ok(Chosen::Ask(Question {
        field: "project".into(),
        prompt,
        options: all
            .iter()
            .map(|p| Opt {
                label: p.name.clone(),
                value: json!({"id": p.id}),
            })
            .collect(),
        page: 0,
        text_ok: true,
    }))
}

fn ask(field: &str, prompt: impl Into<String>, options: Vec<Opt>, text_ok: bool) -> Question {
    Question {
        field: field.into(),
        prompt: prompt.into(),
        options,
        page: 0,
        text_ok,
    }
}

fn opt(label: impl Into<String>, value: Value) -> Opt {
    Opt {
        label: label.into(),
        value,
    }
}

fn common_times() -> Vec<Opt> {
    [
        ("9:00 AM", "09:00"),
        ("10:00 AM", "10:00"),
        ("1:00 PM", "13:00"),
        ("2:00 PM", "14:00"),
        ("3:00 PM", "15:00"),
    ]
    .iter()
    .map(|(l, v)| opt(*l, json!(v)))
    .collect()
}

/// A local date and time as a UTC instant, or the question that resolves a daylight-saving problem.
fn instant(
    cx: &Cx,
    date: NaiveDate,
    time: NaiveTime,
    fold: &Option<String>,
) -> std::result::Result<String, Question> {
    match dates::resolve(cx.tz, date, time) {
        Local::Single(v) => Ok(utc(v)),
        Local::Missing => Err(ask(
            "time",
            format!(
                "{} doesn't exist on {} in {} because the clocks change. What time should I use?",
                time.format("%-I:%M %p"),
                date,
                cx.tz_name
            ),
            common_times(),
            true,
        )),
        Local::Ambiguous(a, b) => match fold.as_deref() {
            Some("earlier") => Ok(utc(a)),
            Some("later") => Ok(utc(b)),
            _ => Err(ask(
                "fold",
                format!(
                    "{} happens twice on {} in {} because the clocks change. Which one?",
                    time.format("%-I:%M %p"),
                    date,
                    cx.tz_name
                ),
                vec![
                    opt(format!("First ({})", cx.local(&utc(a))), json!("earlier")),
                    opt(format!("Second ({})", cx.local(&utc(b))), json!("later")),
                ],
                false,
            )),
        },
    }
}

fn priority_label(p: &Option<String>) -> &'static str {
    match p.as_deref() {
        Some("high") => "High",
        Some("low") => "Low",
        _ => "Medium",
    }
}

async fn resolve_task(conn: &mut SqliteConnection, cx: &Cx, mut t: TaskDraft) -> Result<Resolved> {
    let project = match choose_project(conn, &t.project_id, &t.project_name).await? {
        Chosen::Project(p) => p,
        Chosen::Ask(q) => return Ok(Resolved::Ask(q, Action::AddTask(t))),
        Chosen::Stop(m) => return Ok(Resolved::Fail(m)),
    };
    t.project_id = Some(project.id.clone());
    t.project_name = Some(project.name.clone());
    let mut due_at = Value::Null;
    let mut due_line = "No deadline".to_owned();
    if let Some(date) = t.due_date.as_deref().and_then(dates::parse_date_iso) {
        let given = t.due_time.as_deref().and_then(dates::parse_time_24);
        let time = given.unwrap_or_else(|| NaiveTime::from_hms_opt(17, 0, 0).unwrap());
        t.time_defaulted = given.is_none();
        match instant(cx, date, time, &t.fold) {
            Ok(iso) => {
                due_line = format!(
                    "{}{}",
                    cx.local(&iso),
                    if t.time_defaulted {
                        " (default 5:00 PM)"
                    } else {
                        ""
                    }
                );
                due_at = json!(iso);
            }
            Err(q) => return Ok(Resolved::Ask(q, Action::AddTask(t))),
        }
    }
    let wanted = normalize(&t.title);
    let dupes: Vec<String> =
        sqlx::query_scalar("SELECT title FROM tasks WHERE project_id=? AND status!='done'")
            .bind(&project.id)
            .fetch_all(&mut *conn)
            .await?;
    let mut lines = vec![
        format!("Title: {}", t.title),
        format!("Project: {}", project.name),
        format!("Due: {due_line}"),
        format!("Priority: {}", priority_label(&t.priority)),
    ];
    if let Some(h) = t.estimate_h {
        lines.push(format!("Estimate: {h} h"));
    }
    if !t.notes.is_empty() {
        lines.push(format!("Notes: {}", clip(&t.notes, 300)));
    }
    if dupes.iter().any(|d| normalize(d) == wanted) {
        lines.push("⚠ An open task with this title already exists in this project.".into());
    }
    let input = json!({"project_id":project.id,"title":t.title,"status":"todo","priority":t.priority.clone().unwrap_or_else(|| "medium".into()),"notes_md":t.notes,"due_at":due_at,"estimate_h":t.estimate_h,"external_url":null,"external_provider":null,"external_id":null,"blocked_reason":null,"blocked_on":null,"subtasks":[],"tags":[],"alerts":[1440,60],"attachments":[]});
    Ok(Resolved::Propose(Proposal {
        id: uid(),
        title: "Add task".into(),
        lines,
        done_text: format!(
            "✅ Added task “{}” to {}.",
            clip(&t.title, 80),
            project.name
        ),
        op: Op::CreateTask { input },
        action: Action::AddTask(t),
    }))
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

async fn resolve_meeting(
    conn: &mut SqliteConnection,
    cx: &Cx,
    mut m: MeetingDraft,
) -> Result<Resolved> {
    let project = match choose_project(conn, &m.project_id, &m.project_name).await? {
        Chosen::Project(p) => p,
        Chosen::Ask(q) => return Ok(Resolved::Ask(q, Action::AddMeeting(m))),
        Chosen::Stop(t) => return Ok(Resolved::Fail(t)),
    };
    m.project_id = Some(project.id.clone());
    m.project_name = Some(project.name.clone());
    let Some(date) = m.date.as_deref().and_then(dates::parse_date_iso) else {
        let today = cx.today;
        return Ok(Resolved::Ask(
            ask(
                "date",
                format!(
                    "What day is “{}”? Tap one or type a date like 2026-10-30 or “Friday”.",
                    clip(&m.title, 60)
                ),
                vec![
                    opt(
                        format!("Today ({})", today.format("%a %b %-d")),
                        json!(today.to_string()),
                    ),
                    opt(
                        format!(
                            "Tomorrow ({})",
                            (today + Duration::days(1)).format("%a %b %-d")
                        ),
                        json!((today + Duration::days(1)).to_string()),
                    ),
                ],
                true,
            ),
            Action::AddMeeting(m),
        ));
    };
    let Some(time) = m.time.as_deref().and_then(dates::parse_time_24) else {
        return Ok(Resolved::Ask(
            ask(
                "time",
                format!(
                    "What time is “{}” on {}? Tap one or type a time like 2:30pm.",
                    clip(&m.title, 60),
                    date.format("%a %b %-d")
                ),
                common_times(),
                true,
            ),
            Action::AddMeeting(m),
        ));
    };
    let starts = match instant(cx, date, time, &m.fold) {
        Ok(v) => v,
        Err(q) => {
            if q.field == "time" {
                m.time = None;
            }
            return Ok(Resolved::Ask(q, Action::AddMeeting(m)));
        }
    };
    m.duration_defaulted = m.duration_min.is_none();
    let minutes = m.duration_min.unwrap_or(30);
    let mut draft = start_fields(&starts, &cx.tz_name)?;
    draft["title"] = json!(m.title);
    draft["duration_min"] = json!(minutes);
    draft["link_url"] = m.link_url.clone().map_or(Value::Null, |u| json!(u));
    draft["agenda_md"] = json!("");
    draft["notes_md"] = json!(m.notes);
    draft["reminder_min"] = json!(15);
    draft["show_in_day"] = json!(true);
    draft["task_ids"] = json!([]);
    draft["repeat_rule"] = json!("none");
    draft["fold_policy"] = json!("earlier");
    draft["repeat_until"] = Value::Null;
    let end = crate::agenda::models::parse(&starts)? + Duration::minutes(minutes);
    let mut lines = vec![
        format!("Title: {}", m.title),
        format!("Project: {}", project.name),
        format!(
            "When: {} to {}",
            cx.local(&starts),
            cx.local_clock(&utc(end))
        ),
        format!(
            "Length: {minutes} min{}",
            if m.duration_defaulted {
                " (default)"
            } else {
                ""
            }
        ),
    ];
    if let Some(link) = &m.link_url {
        lines.push(format!("Link: {link}"));
    }
    if !m.notes.is_empty() {
        lines.push(format!("Notes: {}", clip(&m.notes, 300)));
    }
    if crate::agenda::models::parse(&starts)? < crate::agenda::models::parse(&cx.now)? {
        lines.push("⚠ That time is already in the past.".into());
    }
    let existing = crate::agenda::occurrences(
        conn,
        &starts,
        &utc(crate::agenda::models::parse(&starts)? + Duration::seconds(1)),
        Some(&project.id),
        false,
    )
    .await?;
    if existing
        .iter()
        .any(|v| normalize(v["title"].as_str().unwrap_or("")) == normalize(&m.title))
    {
        lines.push("⚠ A meeting with this title already exists at that time.".into());
    }
    Ok(Resolved::Propose(Proposal {
        id: uid(),
        title: "Add meeting".into(),
        lines,
        done_text: format!(
            "✅ Added meeting “{}” on {}.",
            clip(&m.title, 80),
            cx.local(&starts)
        ),
        op: Op::CreateMeeting {
            project_id: project.id,
            draft,
        },
        action: Action::AddMeeting(m),
    }))
}

fn task_label(cx: &Cx, t: &Value) -> String {
    let due = t["due_at"]
        .as_str()
        .map(|d| format!(" · due {}", cx.local(d)))
        .unwrap_or_default();
    format!(
        "{} · {} · {}{}",
        t["title"].as_str().unwrap_or(""),
        t["project_name"].as_str().unwrap_or(""),
        t["status"].as_str().unwrap_or("").replace('_', " "),
        due
    )
}

async fn resolve_complete(
    conn: &mut SqliteConnection,
    cx: &Cx,
    c: CompleteDraft,
) -> Result<Resolved> {
    let task = if let Some(id) = &c.task_id {
        match search::task(conn, id).await {
            Ok(t) => t,
            Err(e) if e.code == "NotFound" => {
                return Ok(Resolved::Fail("That task no longer exists.".into()))
            }
            Err(e) => return Err(e),
        }
    } else {
        let open = search::find_tasks(conn, &c.query, false).await?;
        match open.len() {
            0 => {
                let done = search::find_tasks(conn, &c.query, true).await?;
                return Ok(if let Some(t) = done.first() {
                    Resolved::Info(format!(
                        "“{}” in {} is already done. Nothing to change.",
                        t["title"].as_str().unwrap_or(""),
                        t["project_name"].as_str().unwrap_or("")
                    ))
                } else {
                    Resolved::Ask(
                        search_question("task_search", &format!("I couldn't find an open task matching “{}”. Type another name to search again.", clip(&c.query, 60))),
                        Action::CompleteTask(c),
                    )
                });
            }
            1 => open[0].clone(),
            _ => {
                let options = open
                    .iter()
                    .map(|t| opt(task_label(cx, t), json!({"id": t["id"]})))
                    .collect();
                return Ok(Resolved::Ask(
                    ask(
                        "task",
                        format!(
                            "Which task did you finish? (matches for “{}”)",
                            clip(&c.query, 60)
                        ),
                        options,
                        false,
                    ),
                    Action::CompleteTask(c),
                ));
            }
        }
    };
    if task["status"] == "done" {
        return Ok(Resolved::Info(format!(
            "“{}” is already done. Nothing to change.",
            task["title"].as_str().unwrap_or("")
        )));
    }
    let mut c = c;
    c.task_id = task["id"].as_str().map(str::to_owned);
    let title = task["title"].as_str().unwrap_or("").to_owned();
    let mut lines = vec![
        format!("Task: {title}"),
        format!("Project: {}", task["project_name"].as_str().unwrap_or("")),
        format!(
            "Status: {} → done",
            task["status"].as_str().unwrap_or("").replace('_', " ")
        ),
    ];
    if let Some(due) = task["due_at"].as_str() {
        lines.push(format!("Due: {}", cx.local(due)));
    }
    if let Some(note) = &c.note {
        lines.push(format!(
            "Completion note (added to the task's notes, dated when you confirm): {}",
            clip(note, 500)
        ));
    }
    Ok(Resolved::Propose(Proposal {
        id: uid(),
        title: "Mark task done".into(),
        lines,
        done_text: format!("✅ Marked “{}” done.", clip(&title, 80)),
        op: Op::CompleteTask {
            task_id: c.task_id.clone().unwrap_or_default(),
            revision: task["revision"].as_i64().unwrap_or(0),
            notes_md: task["notes_md"].as_str().unwrap_or("").to_owned(),
            note: c.note.clone(),
        },
        action: Action::CompleteTask(c),
    }))
}

fn window_origin(c: &CancelDraft, cx: &Cx) -> NaiveDate {
    c.date
        .as_deref()
        .and_then(dates::parse_date_iso)
        .unwrap_or(cx.today)
}

fn meeting_label(cx: &Cx, m: &Value) -> String {
    let starts = m["starts_at"].as_str().unwrap_or("");
    let repeat = match m["repeat_rule"].as_str() {
        Some("daily") => " · repeats daily",
        Some("weekly") => " · repeats weekly",
        _ => "",
    };
    let cancelled = if m["attendance"] == "canceled" {
        " · cancelled"
    } else {
        ""
    };
    format!(
        "{} · {} · {}{}{}",
        m["title"].as_str().unwrap_or(""),
        cx.local(starts),
        m["project_name"].as_str().unwrap_or(""),
        repeat,
        cancelled
    )
}

async fn resolve_cancel(
    conn: &mut SqliteConnection,
    cx: &Cx,
    mut c: CancelDraft,
) -> Result<Resolved> {
    let remove = c.mode == "remove";
    // Find the occurrence.
    let (meeting_id, occurrence_key) = match (&c.meeting_id, &c.occurrence_key) {
        (Some(m), Some(k)) => (m.clone(), k.clone()),
        _ => {
            let from = c
                .window_start
                .as_deref()
                .and_then(dates::parse_date_iso)
                .unwrap_or_else(|| window_origin(&c, cx));
            let days = if c.date.is_some() { 1 } else { 7 };
            let all = search::find_meetings(conn, &c.query, from, days, &cx.tz_name).await?;
            let live: Vec<&Value> = all
                .iter()
                .filter(|m| remove || m["attendance"] != "canceled")
                .collect();
            let window = if days == 1 {
                from.format("%a %b %-d").to_string()
            } else {
                format!(
                    "{} – {}",
                    from.format("%a %b %-d"),
                    (from + Duration::days(6)).format("%a %b %-d")
                )
            };
            if live.is_empty() {
                let note = if !all.is_empty() {
                    " (the matching ones are already marked cancelled)"
                } else {
                    ""
                };
                let mut q = search_question(
                    "meeting_search",
                    &format!("I couldn't find a meeting matching “{}” for {window}{note}. Type another name, or look at a different period.", clip(&c.query, 60)),
                );
                q.text_ok = true;
                c.window_start = Some(from.to_string());
                return Ok(Resolved::Ask(q, Action::CancelMeeting(c)));
            }
            if live.len() > 1 {
                c.window_start = Some(from.to_string());
                let options = live
                    .iter()
                    .map(|m| {
                        opt(
                            meeting_label(cx, m),
                            json!({"m": m["ref"]["meetingId"], "k": m["ref"]["occurrenceKey"]}),
                        )
                    })
                    .collect();
                return Ok(Resolved::Ask(
                    ask(
                        "meeting",
                        format!(
                            "Which meeting? (matches for “{}”, {window})",
                            clip(&c.query, 60)
                        ),
                        options,
                        false,
                    ),
                    Action::CancelMeeting(c),
                ));
            }
            let m = live[0];
            (
                m["ref"]["meetingId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                m["ref"]["occurrenceKey"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        }
    };
    let reference = MeetingRef {
        meeting_id: meeting_id.clone(),
        occurrence_key: occurrence_key.clone(),
    };
    let detail = match crate::agenda::meeting_detail(conn, &reference).await {
        Ok(d) => d,
        Err(e) if e.code == "NotFound" => {
            return Ok(Resolved::Fail("That meeting no longer exists.".into()))
        }
        Err(e) => return Err(e),
    };
    c.meeting_id = Some(meeting_id.clone());
    c.occurrence_key = Some(occurrence_key.clone());
    let occurrence = &detail["occurrence"];
    let recurring = detail["series"]["is_recurring"] == true;
    let revision = detail["series"]["revision"].as_i64().unwrap_or(0);
    let title = occurrence["title"].as_str().unwrap_or("").to_owned();
    let when = cx.local(occurrence["starts_at"].as_str().unwrap_or(""));
    if !remove && occurrence["attendance"] == "canceled" {
        return Ok(Resolved::Info(format!(
            "“{title}” on {when} is already marked cancelled. Nothing to change."
        )));
    }
    // A recurring meeting needs an explicit scope.
    if recurring && c.scope.is_none() {
        let options = if remove {
            vec![
                opt(
                    "Remove this occurrence only",
                    json!({"mode":"remove","scope":"occurrence"}),
                ),
                opt(
                    "Remove this and all following",
                    json!({"mode":"remove","scope":"following"}),
                ),
            ]
        } else {
            vec![
                opt(
                    "Cancel this occurrence (keep visible)",
                    json!({"mode":"cancel","scope":"occurrence"}),
                ),
                opt(
                    "End series from this occurrence (remove this and following)",
                    json!({"mode":"remove","scope":"following"}),
                ),
            ]
        };
        return Ok(Resolved::Ask(
            ask(
                "scope",
                format!("“{title}” on {when} repeats. What should happen?"),
                options,
                false,
            ),
            Action::CancelMeeting(c),
        ));
    }
    let mut lines = vec![
        format!("Meeting: {title}"),
        format!("When: {when}"),
        format!(
            "Project: {}",
            occurrence["project_name"].as_str().unwrap_or("")
        ),
    ];
    if recurring {
        lines.push(format!(
            "Repeats: {}",
            occurrence["repeat_rule"].as_str().unwrap_or("")
        ));
    }
    let proposal = if !c.mode.eq("remove") && c.scope.as_deref() != Some("following") {
        lines.push(
            "Effect: marked cancelled. It stays on your calendar, struck through, with its notes."
                .into(),
        );
        Proposal {
            id: uid(),
            title: "Cancel meeting".into(),
            lines,
            done_text: format!("✅ Marked “{}” on {} as cancelled.", clip(&title, 80), when),
            op: Op::CancelMeeting {
                meeting_id,
                occurrence_key,
                revision,
                occurrence_notes: occurrence["occurrence_notes_md"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            },
            action: Action::CancelMeeting(c),
        }
    } else {
        let scope = if !recurring {
            "one_off"
        } else if c.scope.as_deref() == Some("following") {
            "following"
        } else {
            "occurrence"
        };
        lines.push(
            match scope {
                "one_off" => "Effect: permanently deletes this meeting and its notes.".to_owned(),
                "occurrence" => format!("Effect: removes only the {when} occurrence. Other occurrences stay."),
                _ => format!("Effect: ends the series from {when}. This and all later occurrences are removed; earlier ones stay."),
            },
        );
        let change = json!({"action":"remove","payload":{"ref":{"meetingId":meeting_id,"occurrenceKey":occurrence_key},"expectedRevision":revision,"scope":scope}});
        let fingerprint = match crate::agenda::preview_into(conn, &change, &cx.now).await {
            Ok(v) => v["fingerprint"].as_str().map(str::to_owned),
            Err(e) => {
                return Ok(Resolved::Fail(format!(
                    "I can't remove that: {}",
                    e.message
                )))
            }
        };
        Proposal {
            id: uid(),
            title: "Remove meeting".into(),
            lines,
            done_text: match scope {
                "one_off" => format!("✅ Deleted “{}”.", clip(&title, 80)),
                "occurrence" => format!("✅ Removed “{}” on {}.", clip(&title, 80), when),
                _ => format!("✅ Ended the “{}” series from {}.", clip(&title, 80), when),
            },
            op: Op::RemoveMeeting {
                meeting_id,
                occurrence_key,
                revision,
                scope: scope.into(),
                fingerprint,
            },
            action: Action::CancelMeeting(c),
        }
    };
    Ok(Resolved::Propose(proposal))
}

// ---------------------------------------------------------------------------------------------
// Applying a confirmed change.

fn stamp(cx: &Cx) -> String {
    chrono::DateTime::parse_from_rfc3339(&cx.now)
        .map(|d| {
            d.with_timezone(&cx.tz)
                .format("%Y-%m-%d %H:%M (UTC%:z)")
                .to_string()
        })
        .unwrap_or_else(|_| cx.now.clone())
}

async fn apply_op(conn: &mut SqliteConnection, cx: &Cx, p: &Proposal) -> Result<()> {
    match &p.op {
        Op::CreateTask { input } => {
            crate::workspace::tasks::create_in(conn, input.clone(), &cx.now).await?;
        }
        Op::CompleteTask {
            task_id,
            revision,
            notes_md,
            note,
        } => {
            let mut patch = json!({"status":"done"});
            if let Some(note) = note.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                let section = format!("### Completion note — {}\n\n{}\n", stamp(cx), note);
                patch["notes_md"] = json!(if notes_md.trim().is_empty() {
                    section
                } else {
                    format!("{}\n\n{}", notes_md.trim_end(), section)
                });
            }
            crate::workspace::tasks::update_in(conn, task_id, patch, *revision, &cx.now).await?;
        }
        Op::CreateMeeting { project_id, draft } => {
            let change =
                json!({"action":"create","payload":{"projectId":project_id,"draft":draft}});
            agenda(conn, cx, p, change, None).await?;
        }
        Op::CancelMeeting {
            meeting_id,
            occurrence_key,
            revision,
            occurrence_notes,
        } => {
            let change = json!({"action":"record","payload":{"ref":{"meetingId":meeting_id,"occurrenceKey":occurrence_key},"expectedRevision":revision,"attendance":"canceled","occurrence_notes_md":occurrence_notes}});
            agenda(conn, cx, p, change, None).await?;
        }
        Op::RemoveMeeting {
            meeting_id,
            occurrence_key,
            revision,
            scope,
            fingerprint,
        } => {
            let change = json!({"action":"remove","payload":{"ref":{"meetingId":meeting_id,"occurrenceKey":occurrence_key},"expectedRevision":revision,"scope":scope}});
            agenda(conn, cx, p, change, fingerprint.clone()).await?;
        }
    }
    Ok(())
}

async fn agenda(
    conn: &mut SqliteConnection,
    cx: &Cx,
    p: &Proposal,
    change: Value,
    fingerprint: Option<String>,
) -> Result<()> {
    crate::agenda::apply_in(
        conn,
        AgendaInput {
            request_id: format!("telegram:{}", p.id),
            change,
            expected_fingerprint: fingerprint,
        },
        &cx.now,
    )
    .await?;
    Ok(())
}

async fn confirm(
    conn: &mut SqliteConnection,
    cx: &Cx,
    out: &mut Out,
    mut flow: Flow,
    p: Proposal,
    message_id: i64,
) -> Result<()> {
    let seen: Option<String> =
        sqlx::query_scalar("SELECT summary FROM telegram_applied WHERE proposal_id=?")
            .bind(&p.id)
            .fetch_optional(&mut *conn)
            .await?;
    if let Some(summary) = seen {
        out.edit(
            message_id,
            format!("{summary}\n(Already saved; nothing was changed again.)"),
        );
        flow.current = None;
        flow.stage = None;
        return drive(conn, cx, out, flow).await;
    }
    sqlx::query("SAVEPOINT tg_apply")
        .execute(&mut *conn)
        .await?;
    match apply_op(conn, cx, &p).await {
        Ok(()) => {
            sqlx::query("RELEASE tg_apply").execute(&mut *conn).await?;
            sqlx::query(
                "INSERT INTO telegram_applied(proposal_id,applied_at,summary) VALUES(?,?,?)",
            )
            .bind(&p.id)
            .bind(&cx.now)
            .bind(&p.done_text)
            .execute(&mut *conn)
            .await?;
            out.edit(message_id, p.done_text.clone());
            flow.done.push(p.done_text);
            flow.current = None;
            flow.stage = None;
            drive(conn, cx, out, flow).await
        }
        Err(e) => {
            sqlx::query("ROLLBACK TO tg_apply")
                .execute(&mut *conn)
                .await?;
            sqlx::query("RELEASE tg_apply").execute(&mut *conn).await?;
            if matches!(e.code, "Conflict" | "NotFound") {
                out.edit(message_id, "⚠ That changed since I showed it to you, so nothing was saved. Here's a fresh look.");
                flow.current = Some(p.action);
                flow.stage = None;
            } else {
                out.edit(
                    message_id,
                    format!(
                        "❌ I couldn't save that: {} Nothing was changed.",
                        e.message
                    ),
                );
                flow.current = None;
                flow.stage = None;
            }
            drive(conn, cx, out, flow).await
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Rendering.

fn present(out: &mut Out, cx: &Cx, flow: &mut Flow) {
    flow.step += 1;
    let Some(stage) = flow.stage.clone() else {
        return;
    };
    // Every question and confirmation gets its own 30 minutes. A conflict prompt does not
    // extend the request it interrupted.
    if !matches!(stage, Stage::Conflict { .. }) {
        flow.expires_at = cx.expiry();
    }
    let key = |code: &str| format!("k|{}|{}|{}", flow.token, flow.step, code);
    let button = |label: &str, code: &str| json!({"text": label, "callback_data": key(code)});
    let (text, rows): (String, Vec<Vec<Value>>) = match &stage {
        Stage::Confirm(p) => (
            format!("{}\n\n{}\n\nTap Confirm to save, Change to fix something, or Cancel.", p.title, p.lines.iter().map(|l| format!("• {l}")).collect::<Vec<_>>().join("\n")),
            vec![vec![button("✅ Confirm", "y"), button("✏️ Change", "e"), button("✖️ Cancel", "x")]],
        ),
        Stage::Revise(p) => (
            format!("Change “{}”.\nSend the correction as a message, for example “make it Friday at 3pm” or “call it Budget review”.", p.title),
            vec![vec![button("✖️ Cancel", "x")]],
        ),
        Stage::Conflict { .. } => (
            "I'm still waiting on your last request. What would you like to do?".to_owned(),
            vec![
                vec![button("Continue current request", "k")],
                vec![button("Discard it and start this request", "d")],
            ],
        ),
        Stage::Question(q) => {
            let mut rows: Vec<Vec<Value>> = vec![];
            let start = q.page * PAGE;
            for (i, o) in q.options.iter().enumerate().skip(start).take(PAGE) {
                rows.push(vec![button(&clip(&o.label, 60), &format!("o{i}"))]);
            }
            let mut nav = vec![];
            if q.page > 0 {
                nav.push(button("◀ Previous", &format!("p{}", q.page - 1)));
            }
            if start + PAGE < q.options.len() {
                nav.push(button("More ▶", &format!("p{}", q.page + 1)));
            }
            if !nav.is_empty() {
                rows.push(nav);
            }
            match q.field.as_str() {
                "task" => rows.push(vec![button("🔎 Search again", "s")]),
                "meeting" => {
                    rows.push(vec![button("◀ Earlier", "b"), button("Later ▶", "n")]);
                    rows.push(vec![button("🔎 Search again", "s")]);
                }
                "meeting_search" => rows.push(vec![button("◀ Earlier", "b"), button("Later ▶", "n")]),
                _ => {}
            }
            rows.push(vec![button("✖️ Cancel", "x")]);
            let hint = if q.text_ok && !q.options.is_empty() { "\nYou can also type your answer." } else { "" };
            (format!("{}{hint}", q.prompt), rows)
        }
    };
    out.prompt(text, json!({"inline_keyboard": rows}), &flow.token);
}

fn bullet(items: &[Value], cx: &Cx, render: impl Fn(&Value) -> String) -> String {
    let mut lines: Vec<String> = items
        .iter()
        .take(15)
        .map(|i| format!("• {}", render(i)))
        .collect();
    if items.len() > 15 {
        lines.push(format!("…and {} more", items.len() - 15));
    }
    let _ = cx;
    lines.join("\n")
}

pub async fn day_text(conn: &mut SqliteConnection, cx: &Cx, date: NaiveDate) -> Result<String> {
    let view = search::day(conn, date, cx.today, &cx.tz_name).await?;
    let mut text = format!("📅 {} ({})", date.format("%A, %b %-d, %Y"), cx.tz_name);
    let due_time = |t: &Value| {
        t["due_at"]
            .as_str()
            .map(|d| cx.local_clock(d))
            .unwrap_or_default()
    };
    if !view.overdue.is_empty() {
        text += &format!(
            "\n\nOverdue ({})\n{}",
            view.overdue.len(),
            bullet(&view.overdue, cx, |t| format!(
                "{} — {} · was due {}",
                t["title"].as_str().unwrap_or(""),
                t["project_name"].as_str().unwrap_or(""),
                t["due_at"]
                    .as_str()
                    .map(|d| cx.local(d))
                    .unwrap_or_default()
            ))
        );
    }
    if !view.tasks.is_empty() {
        text += &format!(
            "\n\nTasks due ({})\n{}",
            view.tasks.len(),
            bullet(&view.tasks, cx, |t| format!(
                "{} — {} · {}",
                t["title"].as_str().unwrap_or(""),
                t["project_name"].as_str().unwrap_or(""),
                due_time(t)
            ))
        );
    }
    if !view.meetings.is_empty() {
        text += &format!(
            "\n\nMeetings ({})\n{}",
            view.meetings.len(),
            bullet(&view.meetings, cx, |m| {
                let start = m["starts_at"].as_str().unwrap_or("");
                let end = crate::agenda::models::parse(start)
                    .map(|s| utc(s + Duration::minutes(m["duration_min"].as_i64().unwrap_or(0))))
                    .unwrap_or_default();
                format!(
                    "{}–{} {} — {}{}",
                    cx.local_clock(start),
                    cx.local_clock(&end),
                    m["title"].as_str().unwrap_or(""),
                    m["project_name"].as_str().unwrap_or(""),
                    if m["attendance"] == "canceled" {
                        " (cancelled)"
                    } else {
                        ""
                    }
                )
            })
        );
    }
    if view.overdue.is_empty() && view.tasks.is_empty() && view.meetings.is_empty() {
        text += "\n\nNothing scheduled and no tasks due.";
    }
    Ok(text)
}

/// Provider errors are written for the AI Inbox screen ("Your review is saved…"); drop that part.
fn plain(message: &str) -> String {
    message
        .split(" Your review is saved")
        .next()
        .unwrap_or(message)
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn inbox_only_advice_is_removed_from_provider_errors() {
        assert_eq!(
            plain("The provider is temporarily unavailable. Retry later. (HTTP 503) Your review is saved; retry after resolving this."),
            "The provider is temporarily unavailable. Retry later. (HTTP 503)"
        );
        assert_eq!(plain("Nothing to trim."), "Nothing to trim.");
    }
}
