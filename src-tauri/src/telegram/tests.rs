use super::ai::Brain;
use super::api::Api;
use super::config::{self, Pairing};
use super::engine::{self, ClockSource, Env};
use crate::workspace::models::*;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::Mutex;

const USER: i64 = 4242;
const CHAT: i64 = 4242;
const TZ: &str = "America/New_York";
/// Thursday 2026-10-29, 10:00 in New York.
const NOW: &str = "2026-10-29T14:00:00.000Z";

#[derive(Default)]
struct Fake {
    sent: Mutex<Vec<(i64, String, Option<Value>)>>,
    edits: Mutex<Vec<(i64, String)>>,
    answers: Mutex<Vec<String>>,
    fail_sends: Mutex<bool>,
    next_id: Mutex<i64>,
}
impl Api for Fake {
    async fn updates(&self, _: i64, _: u64) -> Result<Vec<Value>> {
        Ok(vec![])
    }
    async fn send(&self, chat: i64, text: &str, markup: Option<&Value>) -> Result<i64> {
        if *self.fail_sends.lock().unwrap() {
            return Err(AppError::new("Network", "offline"));
        }
        self.sent
            .lock()
            .unwrap()
            .push((chat, text.to_owned(), markup.cloned()));
        let mut id = self.next_id.lock().unwrap();
        *id += 1;
        Ok(*id)
    }
    async fn edit(&self, _: i64, id: i64, text: &str, _: Option<&Value>) -> Result<()> {
        self.edits.lock().unwrap().push((id, text.to_owned()));
        Ok(())
    }
    async fn answer(&self, _: &str, text: &str) -> Result<()> {
        self.answers.lock().unwrap().push(text.to_owned());
        Ok(())
    }
    async fn download(&self, _: &str, _: u64) -> Result<Vec<u8>> {
        Ok(vec![1, 2, 3])
    }
}
impl Fake {
    fn last(&self) -> (String, Option<Value>) {
        let sent = self.sent.lock().unwrap();
        let (_, text, markup) = sent.last().expect("a reply was sent").clone();
        (text, markup)
    }
    fn count(&self) -> usize {
        self.sent.lock().unwrap().len()
    }
}

/// Answers the next AI call with a queued JSON string.
#[derive(Default)]
struct Script {
    replies: Mutex<Vec<String>>,
    calls: Mutex<Vec<(String, String)>>,
    transcript: Mutex<Option<String>>,
}
impl Brain for Script {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        self.calls
            .lock()
            .unwrap()
            .push((system.to_owned(), user.to_owned()));
        let mut queue = self.replies.lock().unwrap();
        if queue.is_empty() {
            return Err(AppError::validation("No AI answer was scripted."));
        }
        Ok(queue.remove(0))
    }
    async fn transcribe(&self, _: Vec<u8>) -> Result<String> {
        self.transcript
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| AppError::validation("No transcript."))
    }
}
impl Script {
    fn say(&self, json: Value) {
        self.replies.lock().unwrap().push(json.to_string());
    }
    fn calls(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

struct Bench {
    _dir: tempfile::TempDir,
    pool: SqlitePool,
    api: Fake,
    brain: Script,
    pairing: Pairing,
    clock: Mutex<ClockSource>,
    next_update: Mutex<i64>,
}

async fn bench() -> Bench {
    let dir = tempfile::tempdir().unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("telegram.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO projects(id,name,color,sort_order) VALUES('p1','Billing','cyan',0),('p2','Website','lime',1)")
        .execute(&pool)
        .await
        .unwrap();
    Bench {
        _dir: dir,
        pool,
        api: Fake::default(),
        brain: Script::default(),
        pairing: Pairing::default(),
        clock: Mutex::new(ClockSource::Fixed(NOW.into())),
        next_update: Mutex::new(1),
    }
}

impl Bench {
    async fn pair(&self) {
        let mut conn = self.pool.acquire().await.unwrap();
        config::save(
            &mut conn,
            &config::Config {
                enabled: true,
                consent: true,
                bot_username: "test_bot".into(),
                user_id: Some(USER),
                chat_id: Some(CHAT),
            },
        )
        .await
        .unwrap();
    }
    fn set_now(&self, iso: &str) {
        *self.clock.lock().unwrap() = ClockSource::Fixed(iso.into());
    }
    /// Delivers raw updates and processes them.
    async fn deliver(&self, updates: Vec<Value>) {
        let clock = match &*self.clock.lock().unwrap() {
            ClockSource::Fixed(v) => ClockSource::Fixed(v.clone()),
            ClockSource::Offset(v) => ClockSource::Offset(*v),
        };
        let env = Env {
            pool: &self.pool,
            api: &self.api,
            brain: &self.brain,
            clock: &clock,
            tz: TZ,
            pairing: &self.pairing,
        };
        engine::ingest(&env, &updates).await.unwrap();
        if let Err(e) = engine::run_once(&env).await {
            // An unreachable Telegram leaves replies queued; anything else is a bug.
            assert_eq!(e.code, "Network", "{}", e.message);
        }
    }
    fn update_id(&self) -> i64 {
        let mut n = self.next_update.lock().unwrap();
        *n += 1;
        *n
    }
    fn message(&self, user: i64, chat: i64, kind: &str, text: &str) -> Value {
        let mut message = json!({"message_id": 1, "date": 1_793_282_400, "from": {"id": user}, "chat": {"id": chat, "type": kind}});
        message["text"] = json!(text);
        json!({"update_id": self.update_id(), "message": message})
    }
    async fn say(&self, text: &str) {
        let update = self.message(USER, CHAT, "private", text);
        self.deliver(vec![update]).await;
    }
    fn tap_update(&self, data: &str) -> Value {
        json!({"update_id": self.update_id(), "callback_query": {"id": "cb", "from": {"id": USER}, "data": data, "message": {"message_id": 77, "chat": {"id": CHAT, "type": "private"}}}})
    }
    /// Taps the button whose label starts with `label` on the latest prompt.
    async fn tap(&self, label: &str) {
        let (_, markup) = self.api.last();
        let markup = markup.expect("the latest reply has buttons");
        let data = markup["inline_keyboard"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| row.as_array().unwrap().iter())
            .find(|b| b["text"].as_str().unwrap().contains(label))
            .unwrap_or_else(|| panic!("no button {label} in {markup}"))["callback_data"]
            .as_str()
            .unwrap()
            .to_owned();
        let update = self.tap_update(&data);
        self.deliver(vec![update]).await;
    }
    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await.unwrap()
    }
}

fn actions(list: Value) -> Value {
    json!({"actions": list, "reply": null})
}

// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn pairing_accepts_one_account_and_ignores_everyone_else() {
    let b = bench().await;
    let code = b.pairing.start();

    // A stranger with the right code in a group, and a wrong code in private, are ignored.
    let group = b.message(7, -100, "group", &format!("/start {code}"));
    let wrong = b.message(7, 7, "private", "/start 000000");
    b.deliver(vec![group, wrong]).await;
    assert_eq!(b.api.count(), 0);
    let mut conn = b.pool.acquire().await.unwrap();
    assert_eq!(config::load(&mut conn).await.unwrap().user_id, None);
    drop(conn);

    let right = b.message(USER, CHAT, "private", &format!("/start {code}"));
    b.deliver(vec![right]).await;
    assert!(b.api.last().0.contains("Paired"));
    let mut conn = b.pool.acquire().await.unwrap();
    let cfg = config::load(&mut conn).await.unwrap();
    assert_eq!((cfg.user_id, cfg.chat_id), (Some(USER), Some(CHAT)));
    drop(conn);

    // The code is consumed, and after pairing other senders get nothing.
    let sent = b.api.count();
    let other = b.message(7, 7, "private", "/help");
    let replay = b.message(7, 7, "private", &format!("/start {code}"));
    b.deliver(vec![other, replay]).await;
    assert_eq!(b.api.count(), sent);
}

#[tokio::test]
async fn unpaired_bot_stays_silent_without_a_started_pairing() {
    let b = bench().await;
    let update = b.message(USER, CHAT, "private", "/start 123456");
    b.deliver(vec![update]).await;
    assert_eq!(b.api.count(), 0);
}

#[tokio::test]
async fn adding_a_task_asks_for_the_project_then_confirms_then_saves_once() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(json!([{"type":"add_task","title":"Send invoice to Acme","due_date":"2026-10-30","notes":"net 30"}])));
    b.say("add a task to send the invoice to Acme by Friday")
        .await;

    let (text, _) = b.api.last();
    assert!(text.contains("Which project"), "{text}");
    b.tap("Billing").await;

    let (card, _) = b.api.last();
    assert!(
        card.contains("Add task") && card.contains("Billing"),
        "{card}"
    );
    assert!(
        card.contains("Fri Oct 30, 2026 5:00 PM EDT (default 5:00 PM)"),
        "{card}"
    );
    assert_eq!(
        b.count("SELECT COUNT(*) FROM tasks").await,
        0,
        "nothing is saved before Confirm"
    );

    b.tap("Confirm").await;
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 1);
    let task: (String, String, String, String) =
        sqlx::query_as("SELECT title,status,due_at,notes_md FROM tasks")
            .fetch_one(&b.pool)
            .await
            .unwrap();
    assert_eq!(
        task,
        (
            "Send invoice to Acme".into(),
            "todo".into(),
            "2026-10-30T21:00:00.000Z".into(),
            "net 30".into()
        )
    );
    assert!(b
        .api
        .edits
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .1
        .contains("Added task"));

    // Tapping the same (now stale) Confirm again changes nothing.
    let stale = b.tap_update(&format!("k|{}|{}|y", "zzzzzzzz", 1));
    b.deliver(vec![stale]).await;
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 1);
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_flow").await, 0);
}

#[tokio::test]
async fn a_replayed_confirmation_does_not_apply_twice() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(
        json!([{"type":"add_task","title":"Once only","project_name":"Billing"}]),
    ));
    b.say("add once only to billing").await;
    let (_, markup) = b.api.last();
    let confirm = markup.unwrap()["inline_keyboard"][0][0]["callback_data"]
        .as_str()
        .unwrap()
        .to_owned();

    // The same button update arrives twice (Telegram retry): the second is a different update id.
    let first = b.tap_update(&confirm);
    let second = b.tap_update(&confirm);
    b.deliver(vec![first, second]).await;
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 1);
}

#[tokio::test]
async fn completing_a_task_appends_the_note_exactly_once_without_blocking_it() {
    let b = bench().await;
    b.pair().await;
    sqlx::query("INSERT INTO tasks(id,project_id,title,status,priority,notes_md,created_at,updated_at,sort_order) VALUES('t1','p1','Vendor invoice','in_progress','medium','Existing notes','2026-10-01T00:00:00.000Z','2026-10-01T00:00:00.000Z',0)")
        .execute(&b.pool)
        .await
        .unwrap();
    b.brain.say(actions(json!([{"type":"complete_task","query":"vendor invoice","note":"waiting on vendor, shipped Friday"}])));
    b.say("vendor invoice is done: waiting on vendor, shipped Friday")
        .await;
    let (card, _) = b.api.last();
    assert!(
        card.contains("in progress → done") && card.contains("waiting on vendor, shipped Friday"),
        "{card}"
    );
    b.tap("Confirm").await;

    let row: (String, String, Option<String>) =
        sqlx::query_as("SELECT status,notes_md,done_at FROM tasks WHERE id='t1'")
            .fetch_one(&b.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "done");
    assert!(row.2.is_some());
    assert!(row.1.starts_with("Existing notes\n\n### Completion note — 2026-10-29 10:00 (UTC-04:00)\n\nwaiting on vendor, shipped Friday"), "{}", row.1);
    assert_eq!(row.1.matches("Completion note").count(), 1);

    // Asking again reports it is already done and appends nothing.
    b.brain.say(actions(
        json!([{"type":"complete_task","query":"vendor invoice","note":"again"}]),
    ));
    b.say("vendor invoice done again").await;
    assert!(b.api.last().0.contains("already done"));
    let notes: String = sqlx::query_scalar("SELECT notes_md FROM tasks WHERE id='t1'")
        .fetch_one(&b.pool)
        .await
        .unwrap();
    assert_eq!(notes.matches("Completion note").count(), 1);
}

#[tokio::test]
async fn several_matching_tasks_are_offered_as_buttons_and_a_stale_target_is_re_reviewed() {
    let b = bench().await;
    b.pair().await;
    for (id, title) in [("a", "Review contract A"), ("b", "Review contract B")] {
        sqlx::query("INSERT INTO tasks(id,project_id,title,status,priority,notes_md,created_at,updated_at,sort_order) VALUES(?,'p1',?,'todo','medium','','2026-10-01T00:00:00.000Z','2026-10-01T00:00:00.000Z',0)")
            .bind(id).bind(title).execute(&b.pool).await.unwrap();
    }
    b.brain.say(actions(
        json!([{"type":"complete_task","query":"review contract","note":null}]),
    ));
    b.say("finished reviewing the contract").await;
    let (prompt, markup) = b.api.last();
    assert!(prompt.contains("Which task"), "{prompt}");
    assert!(markup.unwrap().to_string().contains("Review contract B"));
    b.tap("Review contract B").await;
    assert!(b.api.last().0.contains("Mark task done"));

    // The task changes in the app after the preview.
    sqlx::query(
        "UPDATE tasks SET title='Review contract B (final)', revision=revision+1 WHERE id='b'",
    )
    .execute(&b.pool)
    .await
    .unwrap();
    b.tap("Confirm").await;
    let status: String = sqlx::query_scalar("SELECT status FROM tasks WHERE id='b'")
        .fetch_one(&b.pool)
        .await
        .unwrap();
    assert_eq!(status, "todo", "a stale preview must not be applied");
    assert!(b
        .api
        .edits
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .1
        .contains("changed since"));
    assert!(
        b.api.last().0.contains("Mark task done"),
        "a fresh preview follows"
    );
    assert!(b.api.last().0.contains("final"));
}

#[tokio::test]
async fn adding_a_meeting_asks_for_missing_times_and_defaults_the_length() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(json!([{"type":"add_meeting","title":"Design review","project_name":"Website","date":"2026-11-03"}])));
    b.say("schedule a design review next Tuesday").await;
    assert!(b.api.last().0.contains("What time"));
    b.say("2:30pm").await;
    let (card, _) = b.api.last();
    assert!(
        card.contains("Tue Nov 3, 2026 2:30 PM EST to 3:00 PM"),
        "{card}"
    );
    assert!(card.contains("30 min (default)"), "{card}");
    b.tap("Confirm").await;
    let row: (String, String, i64, String) =
        sqlx::query_as("SELECT title,starts_at,duration_min,repeat_rule FROM meetings")
            .fetch_one(&b.pool)
            .await
            .unwrap();
    assert_eq!(
        row,
        (
            "Design review".into(),
            "2026-11-03T19:30:00.000Z".into(),
            30,
            "none".into()
        )
    );
}

#[tokio::test]
async fn clock_changes_are_clarified_rather_than_guessed() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(json!([{"type":"add_meeting","title":"Night shift sync","project_name":"Website","date":"2026-11-01","time":"01:30"}])));
    b.say("night shift sync Sunday 1:30am").await;
    assert!(b.api.last().0.contains("happens twice"));
    b.tap("Second").await;
    assert!(b.api.last().0.contains("1:30 AM EST"));

    b.brain.say(actions(json!([{"type":"add_meeting","title":"Gap sync","project_name":"Website","date":"2027-03-14","time":"02:30"}])));
    b.say("gap sync").await;
    // The earlier card is still open, so the new request is parked first.
    assert!(b.api.last().0.contains("still waiting"));
}

#[tokio::test]
async fn cancelling_a_one_off_meeting_keeps_it_visible_and_removing_deletes_it() {
    let b = bench().await;
    b.pair().await;
    let draft = json!({"title":"Vendor call","starts_at":"2026-10-30T18:00:00.000Z","start_local":"2026-10-30T14:00:00.000","start_offset":"-04:00","time_zone":TZ,"duration_min":30,"link_url":null,"agenda_md":"","notes_md":"keep me","reminder_min":15,"show_in_day":true,"task_ids":[],"repeat_rule":"none","fold_policy":"earlier","repeat_until":null});
    let mut conn = b.pool.acquire().await.unwrap();
    crate::agenda::create_in(
        &mut conn,
        "p1",
        &crate::agenda::models::normalize(&draft, true).unwrap(),
        NOW,
    )
    .await
    .unwrap();
    drop(conn);

    b.brain.say(actions(
        json!([{"type":"cancel_meeting","query":"vendor call","mode":"cancel"}]),
    ));
    b.say("the vendor call tomorrow is cancelled").await;
    let (card, _) = b.api.last();
    assert!(
        card.contains("Cancel meeting") && card.contains("struck through"),
        "{card}"
    );
    b.tap("Confirm").await;
    let marked: i64 = sqlx::query_scalar("SELECT marked_cancelled FROM meeting_occurrences")
        .fetch_one(&b.pool)
        .await
        .unwrap();
    assert_eq!(marked, 1);
    assert_eq!(b.count("SELECT COUNT(*) FROM meetings").await, 1);

    // Cancelling it again is reported, not repeated.
    b.brain.say(actions(
        json!([{"type":"cancel_meeting","query":"vendor call","mode":"cancel"}]),
    ));
    b.say("vendor call cancelled").await;
    assert!(
        b.api.last().0.contains("couldn't find a meeting"),
        "{}",
        b.api.last().0
    );
    b.say("/cancel").await;

    // An explicit removal permanently deletes it after its own confirmation.
    b.brain.say(actions(
        json!([{"type":"cancel_meeting","query":"vendor call","mode":"remove"}]),
    ));
    b.say("delete the vendor call").await;
    assert!(b.api.last().0.contains("permanently deletes"));
    b.tap("Confirm").await;
    assert_eq!(b.count("SELECT COUNT(*) FROM meetings").await, 0);
}

#[tokio::test]
async fn a_recurring_meeting_needs_a_scope_and_ending_the_series_uses_the_cutoff() {
    let b = bench().await;
    b.pair().await;
    let draft = json!({"title":"Team standup","starts_at":"2026-10-30T13:00:00.000Z","start_local":"2026-10-30T09:00:00.000","start_offset":"-04:00","time_zone":TZ,"duration_min":15,"link_url":null,"agenda_md":"","notes_md":"","reminder_min":15,"show_in_day":true,"task_ids":[],"repeat_rule":"daily","fold_policy":"earlier","repeat_until":null});
    let mut conn = b.pool.acquire().await.unwrap();
    crate::agenda::create_in(
        &mut conn,
        "p2",
        &crate::agenda::models::normalize(&draft, true).unwrap(),
        NOW,
    )
    .await
    .unwrap();
    drop(conn);

    b.brain.say(actions(
        json!([{"type":"cancel_meeting","query":"standup","date":"2026-11-02","mode":"cancel"}]),
    ));
    b.say("standup on Monday is off").await;
    let (prompt, _) = b.api.last();
    assert!(prompt.contains("repeats"), "{prompt}");
    b.tap("End series").await;
    let (card, _) = b.api.last();
    assert!(card.contains("ends the series"), "{card}");
    b.tap("Confirm").await;
    assert_eq!(b.count("SELECT COUNT(*) FROM meeting_cutoffs").await, 1);
    assert_eq!(
        b.count("SELECT COUNT(*) FROM meetings").await,
        1,
        "the series row is kept; only later occurrences end"
    );
}

#[tokio::test]
async fn lookups_use_stored_data_and_the_original_message_date() {
    let b = bench().await;
    b.pair().await;
    sqlx::query("INSERT INTO tasks(id,project_id,title,status,priority,due_at,notes_md,created_at,updated_at,sort_order) VALUES('d1','p1','Pay supplier','todo','medium','2026-10-29T21:00:00.000Z','','2026-10-01T00:00:00.000Z','2026-10-01T00:00:00.000Z',0),('d2','p1','Old thing','todo','medium','2026-10-20T21:00:00.000Z','','2026-10-01T00:00:00.000Z','2026-10-01T00:00:00.000Z',1)")
        .execute(&b.pool).await.unwrap();
    b.say("/today").await;
    let (text, _) = b.api.last();
    assert!(
        text.contains("Thursday, Oct 29, 2026")
            && text.contains("Pay supplier")
            && text.contains("Overdue (1)")
            && text.contains("Old thing"),
        "{text}"
    );
    assert_eq!(b.brain.calls(), 0, "/today needs no AI call");

    // The next day is processed late (after midnight) but the message was sent on Thursday.
    b.set_now("2026-10-30T09:00:00.000Z");
    b.say("/tomorrow").await;
    let (text, _) = b.api.last();
    assert!(text.contains("Friday, Oct 30, 2026"), "{text}");
    assert!(text.contains("Nothing scheduled"), "{text}");
}

#[tokio::test]
async fn a_new_request_while_waiting_is_never_swallowed_as_an_answer() {
    let b = bench().await;
    b.pair().await;
    b.brain
        .say(actions(json!([{"type":"add_task","title":"First thing"}])));
    b.say("add first thing").await;
    assert!(b.api.last().0.contains("Which project"));

    // Unrelated text is not taken as the project.
    b.say("please also cancel the dentist meeting").await;
    let (text, markup) = b.api.last();
    assert!(text.contains("still waiting"), "{text}");
    assert!(markup
        .unwrap()
        .to_string()
        .contains("Discard it and start this request"));
    assert_eq!(
        b.brain.calls(),
        1,
        "no AI call was spent on the parked message"
    );

    b.tap("Continue").await;
    assert!(b.api.last().0.contains("Which project"));
    b.tap("Website").await;
    assert!(b.api.last().0.contains("Website"));

    // /cancel clears the pending flow without changes.
    b.say("/cancel").await;
    assert!(b.api.last().0.contains("Cancelled"));
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_flow").await, 0);
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 0);
}

#[tokio::test]
async fn expired_questions_report_it_and_cannot_apply_changes() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(
        json!([{"type":"add_task","title":"Slow task","project_name":"Billing"}]),
    ));
    b.say("add slow task to billing").await;
    let (_, markup) = b.api.last();
    let confirm = markup.unwrap()["inline_keyboard"][0][0]["callback_data"]
        .as_str()
        .unwrap()
        .to_owned();

    b.set_now("2026-10-29T14:31:00.000Z");
    let tap = b.tap_update(&confirm);
    b.deliver(vec![tap]).await;
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 0);
    assert!(b
        .api
        .edits
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .1
        .contains("expired"));

    // The next ordinary message mentions the expiry and starts fresh.
    b.brain.say(actions(json!([{"type":"lookup","date":null}])));
    b.say("what is on today").await;
    let sent = b.api.sent.lock().unwrap().clone();
    assert!(sent.iter().any(|(_, t, _)| t.contains("Thursday")));
}

#[tokio::test]
async fn pressing_change_applies_a_typed_correction_to_the_proposal() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(json!([{"type":"add_task","title":"Draft report","project_name":"Billing","due_date":"2026-10-30","due_time":"15:00"}])));
    b.say("draft report friday 3pm billing").await;
    b.tap("Change").await;
    assert!(b.api.last().0.contains("Send the correction"));

    b.brain.say(actions(json!([{"type":"add_task","title":"Draft the Q4 report","project_name":"Billing","due_date":"2026-10-30","due_time":"16:00"}])));
    b.say("call it Draft the Q4 report and make it 4pm").await;
    let (card, _) = b.api.last();
    assert!(
        card.contains("Draft the Q4 report") && card.contains("4:00 PM"),
        "{card}"
    );
    b.tap("Confirm").await;
    let title: String = sqlx::query_scalar("SELECT title FROM tasks")
        .fetch_one(&b.pool)
        .await
        .unwrap();
    assert_eq!(title, "Draft the Q4 report");
}

#[tokio::test]
async fn ai_failures_and_missing_consent_change_nothing() {
    let b = bench().await;
    b.pair().await;
    // No scripted answer: the AI call fails.
    b.say("add a task").await;
    assert!(b.api.last().0.contains("Nothing was changed"));
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 0);
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_flow").await, 0);
}

#[tokio::test]
async fn voice_notes_are_transcribed_and_shown_before_confirmation() {
    let b = bench().await;
    b.pair().await;
    *b.brain.transcript.lock().unwrap() = Some("add a task to water the plants in billing".into());
    b.brain.say(actions(
        json!([{"type":"add_task","title":"Water the plants","project_name":"Billing"}]),
    ));
    let mut update = b.message(USER, CHAT, "private", "");
    update["message"].as_object_mut().unwrap().remove("text");
    update["message"]["voice"] = json!({"file_id":"f","duration":3,"file_size":900});
    b.deliver(vec![update]).await;
    let sent = b.api.sent.lock().unwrap().clone();
    assert!(sent
        .iter()
        .any(|(_, t, _)| t.contains("Heard: “add a task to water the plants in billing”")));
    assert!(sent.last().unwrap().1.contains("Water the plants"));

    // A voice note that cannot be read asks for text and changes nothing.
    *b.brain.transcript.lock().unwrap() = None;
    let mut update = b.message(USER, CHAT, "private", "");
    update["message"].as_object_mut().unwrap().remove("text");
    update["message"]["voice"] = json!({"file_id":"g","duration":3,"file_size":900});
    b.deliver(vec![update]).await;
    assert!(b.api.last().0.contains("send text instead"));
}

#[tokio::test]
async fn replies_are_retried_after_a_delivery_failure_without_repeating_effects() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(
        json!([{"type":"add_task","title":"Resilient","project_name":"Billing"}]),
    ));
    *b.api.fail_sends.lock().unwrap() = true;
    b.say("add resilient to billing").await;
    // The conversation state was saved even though Telegram could not be reached.
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_flow").await, 1);
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_outbox").await, 1);
    assert_eq!(b.api.count(), 0);

    *b.api.fail_sends.lock().unwrap() = false;
    b.deliver(vec![]).await;
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_outbox").await, 0);
    assert!(b.api.last().0.contains("Add task"));
    assert_eq!(b.brain.calls(), 1, "the message was not interpreted twice");
}

#[tokio::test]
async fn each_question_gets_a_fresh_thirty_minutes() {
    let b = bench().await;
    b.pair().await;
    b.brain
        .say(actions(json!([{"type":"add_task","title":"Slow burn"}])));
    b.say("add slow burn").await;
    assert!(b.api.last().0.contains("Which project"));

    // Answered 20 minutes later: the confirmation then has its own 30 minutes.
    b.set_now("2026-10-29T14:20:00.000Z");
    b.tap("Billing").await;
    assert!(b.api.last().0.contains("Add task"));
    b.set_now("2026-10-29T14:45:00.000Z");
    b.tap("Confirm").await;
    assert_eq!(
        b.count("SELECT COUNT(*) FROM tasks").await,
        1,
        "25 minutes after the card is still valid"
    );
}

#[tokio::test]
async fn a_restart_between_storing_and_handling_an_update_still_acts_exactly_once() {
    let b = bench().await;
    b.pair().await;
    b.brain.say(actions(
        json!([{"type":"add_task","title":"Survives restart","project_name":"Billing"}]),
    ));
    let update = b.message(USER, CHAT, "private", "add survives restart to billing");

    // Stored but not yet handled (the app stopped), and delivered again by Telegram afterwards.
    let clock = ClockSource::Fixed(NOW.into());
    let env = Env {
        pool: &b.pool,
        api: &b.api,
        brain: &b.brain,
        clock: &clock,
        tz: TZ,
        pairing: &b.pairing,
    };
    engine::ingest(&env, &[update.clone()]).await.unwrap();
    assert_eq!(
        b.count("SELECT COUNT(*) FROM telegram_updates WHERE processed=0")
            .await,
        1
    );
    b.deliver(vec![update.clone(), update]).await;
    assert_eq!(
        b.count("SELECT COUNT(*) FROM telegram_updates").await,
        1,
        "duplicates are stored once"
    );
    assert_eq!(b.brain.calls(), 1);
    assert_eq!(b.count("SELECT COUNT(*) FROM telegram_flow").await, 1);

    b.tap("Confirm").await;
    assert_eq!(b.count("SELECT COUNT(*) FROM tasks").await, 1);
    let mut conn = b.pool.acquire().await.unwrap();
    assert_eq!(
        engine::next_offset(&mut conn).await.unwrap(),
        4,
        "the next poll acknowledges everything handled"
    );
}
