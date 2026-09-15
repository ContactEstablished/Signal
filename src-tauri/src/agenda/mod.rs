pub mod models;
pub mod recurrence;
use crate::workspace::models::*;
use chrono::{Datelike, Duration, NaiveDateTime, TimeZone, Utc};
use models::*;
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use tauri::AppHandle;
const FIELDS: [&str; 12] = [
    "title",
    "starts_at",
    "start_local",
    "start_offset",
    "time_zone",
    "duration_min",
    "link_url",
    "agenda_md",
    "notes_md",
    "reminder_min",
    "show_in_day",
    "task_ids",
];
pub async fn occurrences(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
    p: Option<&str>,
    visible: bool,
) -> Result<Vec<Value>> {
    Ok(recurrence::project(conn, start, end, p, visible).await?.0)
}
async fn pool(app: &AppHandle) -> Result<SqlitePool> {
    crate::db::pool(app)
        .await
        .map_err(|e| AppError::new("Database", e))
}
async fn segment(
    conn: &mut SqliteConnection,
    version: &str,
    from: i64,
    to: Option<i64>,
    d: &Value,
) -> Result<String> {
    let id = uid();
    let mut s = json!({"id":id,"version_id":version,"from_ordinal":from,"to_ordinal":to,"anchor_local":d["start_local"],"anchor_utc":d["starts_at"]});
    for k in [
        "time_zone",
        "repeat_rule",
        "repeat_until",
        "fold_policy",
        "title",
        "duration_min",
        "link_url",
        "agenda_md",
        "notes_md",
        "reminder_min",
        "show_in_day",
    ] {
        s[k] = d[k].clone();
    }
    s["repeat_weekdays"] = d
        .get("repeat_weekdays")
        .map(|v| json!(v.to_string()))
        .unwrap_or(Value::Null);
    insert(conn, "meeting_segments", &s).await?;
    for t in d["task_ids"].as_array().unwrap() {
        insert(
            conn,
            "meeting_segment_tasks",
            &json!({"segment_id":id,"task_id":t}),
        )
        .await?;
    }
    Ok(id)
}
pub async fn initialize_into(p: &SqlitePool, tz: &str, now: &str) -> Result<()> {
    zone(tz)?;
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    for m in rows(
        &mut tx,
        "SELECT * FROM meetings WHERE active_version_id IS NULL ORDER BY id",
        vec![],
    )
    .await?
    {
        let mut d = start_fields(text(&m, "starts_at")?, tz)?;
        for k in [
            "title",
            "duration_min",
            "link_url",
            "agenda_md",
            "notes_md",
            "repeat_rule",
            "reminder_min",
        ] {
            d[k] = m[k].clone();
        }
        d["fold_policy"] = json!("earlier");
        d["repeat_until"] = Value::Null;
        d["show_in_day"] = json!(true);
        d["task_ids"] = json!(rows(
            &mut tx,
            "SELECT task_id FROM meeting_tasks WHERE meeting_id=? ORDER BY task_id",
            vec![m["id"].clone()]
        )
        .await?
        .iter()
        .map(|r| r["task_id"].clone())
        .collect::<Vec<_>>());
        let v = uid();
        insert(
            &mut tx,
            "meeting_versions",
            &json!({"id":v,"meeting_id":m["id"],"created_at":now}),
        )
        .await?;
        segment(&mut tx, &v, 0, None, &d).await?;
        execute(
            &mut tx,
            "UPDATE meetings SET active_version_id=?,time_zone=?,show_in_day=1 WHERE id=?",
            vec![json!(v), json!(tz), m["id"].clone()],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
async fn task_rows(conn: &mut SqliteConnection) -> Result<Vec<Value>> {
    let mut tasks=rows(conn,"SELECT t.*,p.name AS project_name,p.color AS project_color,(SELECT COUNT(*) FROM subtasks s WHERE s.task_id=t.id AND s.done=1) AS subtask_done,(SELECT COUNT(*) FROM subtasks s WHERE s.task_id=t.id) AS subtask_total FROM tasks t JOIN projects p ON p.id=t.project_id ORDER BY p.name,t.title,t.id",vec![]).await?;
    for t in &mut tasks {
        t["tags"]=json!(rows(conn,"SELECT g.* FROM tags g JOIN task_tags tt ON tt.tag_id=g.id WHERE tt.task_id=? ORDER BY g.name,g.id",vec![t["id"].clone()]).await?);
        t["planned_ranges"] = json!([]);
    }
    Ok(tasks)
}
// A series remains a series after its current tail stops repeating: old
// versions and retained exceptions must never become eligible for one-off deletion.
async fn is_series(conn: &mut SqliteConnection, m: &Value) -> Result<bool> {
    let row=one(conn,"SELECT EXISTS(SELECT 1 FROM meeting_segments s JOIN meeting_versions v ON v.id=s.version_id WHERE v.meeting_id=? AND (s.repeat_rule!='none' OR s.from_ordinal>0)) OR EXISTS(SELECT 1 FROM meeting_occurrences WHERE meeting_id=? AND ordinal>0) AS value",vec![m["id"].clone(),m["id"].clone()]).await?;
    Ok(row["value"] == 1)
}
pub async fn meeting_detail(conn: &mut SqliteConnection, r: &MeetingRef) -> Result<Value> {
    let m = recurrence::root(conn, &r.meeting_id).await?;
    let n = r.ordinal()?;
    let o = recurrence::resolve(conn, &m, n)
        .await?
        .ok_or_else(AppError::missing)?;
    let ss = recurrence::segments(conn, &m).await?;
    let s = ss
        .iter()
        .rev()
        .find(|s| s["from_ordinal"].as_i64().unwrap() <= n)
        .ok_or_else(AppError::missing)?;
    let links = o["task_ids"].as_array().unwrap();
    let tasks = task_rows(conn)
        .await?
        .into_iter()
        .filter(|t| links.contains(&t["id"]))
        .collect::<Vec<_>>();
    Ok(
        json!({"occurrence":o,"series":{"is_recurring":is_series(conn,&m).await?,"repeat_rule":s["repeat_rule"],"repeat_weekdays":recurrence::weekdays(s)?,"repeat_until":s["repeat_until"],"fold_policy":s["fold_policy"],"revision":m["revision"]},"linked_tasks":tasks}),
    )
}
fn bounds(d: chrono::NaiveDate, tz: &str) -> Result<(String, String)> {
    let z = zone(tz)?;
    // Some zones advance at midnight. The boundary is the first valid
    // local instant; an entirely skipped date shares the next day's boundary.
    let midnight = |d: chrono::NaiveDate| -> Result<String> {
        let start = d.and_hms_opt(0, 0, 0).unwrap();
        for minute in 0..=2880 {
            let wall = start + Duration::minutes(minute);
            if let Some(v) = z.from_local_datetime(&wall).earliest() {
                return Ok(utc(v.with_timezone(&Utc)));
            }
        }
        Err(AppError::validation(
            "Calendar boundary is outside the supported timezone range.",
        ))
    };
    Ok((midnight(d)?, midnight(d + Duration::days(1))?))
}
pub async fn snapshot(
    conn: &mut SqliteConnection,
    q: &AgendaQuery,
    now: &str,
    week: bool,
) -> Result<Value> {
    let tz = zone(&q.time_zone)?;
    let today = parse(now)?.with_timezone(&tz).date_naive();
    let selected = if week { date(&q.date)? } else { today };
    let monday = selected - Duration::days(selected.weekday().num_days_from_monday() as i64);
    let project = if week {
        Some(
            one(
                conn,
                "SELECT * FROM projects WHERE id=?",
                vec![json!(q
                    .project_id
                    .as_ref()
                    .ok_or_else(|| AppError::validation("Choose a project."))?)],
            )
            .await?,
        )
    } else {
        None
    };
    let start = bounds(monday, &q.time_zone)?.0;
    let end = bounds(monday + Duration::days(6), &q.time_zone)?.1;
    let (meetings, skipped) =
        recurrence::project(conn, &start, &end, q.project_id.as_deref(), false).await?;
    let mut tasks = task_rows(conn).await?;
    tasks.retain(|t| {
        q.project_id
            .as_ref()
            .is_none_or(|id| t["project_id"] == id.as_str())
    });
    for t in &mut tasks {
        t["planned_ranges"]=json!(rows(conn,"SELECT date,start_min,end_min FROM blocks WHERE task_id=? AND date>=? AND date<=? ORDER BY date,start_min,id",vec![t["id"].clone(),json!(monday.to_string()),json!((monday+Duration::days(7)).to_string())]).await?);
    }
    let due = |t: &Value| {
        t["due_at"]
            .as_str()
            .and_then(|s| parse(s).ok())
            .map(|at| at.with_timezone(&tz).date_naive())
    };
    let unfinished = |t: &Value| t["status"] != "done";
    let mut days = Vec::new();
    let mut counts = serde_json::Map::new();
    for i in 0..7 {
        let d = monday + Duration::days(i);
        let (a, b) = bounds(d, &q.time_zone)?;
        let a = parse(&a)?;
        let b = parse(&b)?;
        let dt = tasks
            .iter()
            .filter(|t| due(t) == Some(d))
            .cloned()
            .collect::<Vec<_>>();
        counts.insert(
            d.to_string(),
            json!(dt.iter().filter(|t| unfinished(t) && d < today).count()),
        );
        let ms = meetings
            .iter()
            .filter(|m| {
                let at = parse(m["starts_at"].as_str().unwrap()).unwrap();
                at < b && at + Duration::minutes(m["duration_min"].as_i64().unwrap()) > a
            })
            .cloned()
            .collect::<Vec<_>>();
        days.push(json!({"date":d.to_string(),"tasks":dt,"meetings":ms}));
    }
    let mut out = json!({"query":{"date":selected.to_string(),"timeZone":q.time_zone,"projectId":q.project_id},"today":today.to_string(),"week_start":monday.to_string(),"days":days,"skipped_dates":skipped});
    if week {
        out["project"] = json!(project.unwrap());
        out["later"] = json!(tasks
            .iter()
            .filter(|t| due(t).is_some_and(|d| d > monday + Duration::days(6)))
            .collect::<Vec<_>>());
        out["no_date"] = json!(tasks
            .iter()
            .filter(|t| t["due_at"].is_null())
            .collect::<Vec<_>>());
        out["overdue_before_week"] = json!(tasks
            .iter()
            .filter(|t| unfinished(t) && due(t).is_some_and(|d| d < monday && d < today))
            .collect::<Vec<_>>());
        out["overdue_counts"] = json!(counts);
    } else {
        for (k, day) in [
            ("overdue", None),
            ("due_today", Some(today)),
            ("tomorrow", Some(today + Duration::days(1))),
        ] {
            out[k] = json!(tasks
                .iter()
                .filter(|t| unfinished(t)
                    && due(t).is_some_and(|d| day.map(|x| d == x).unwrap_or(d < today)))
                .collect::<Vec<_>>());
        }
        let (a, b) = bounds(today, &q.time_zone)?;
        out["meetings"] = json!(occurrences(conn, &a, &b, q.project_id.as_deref(), false).await?);
        out["hours_this_week"]=json!(rows(conn,"SELECT p.id AS project_id,p.name AS project_name,p.color AS project_color,CAST(COALESCE(SUM(e.minutes),0)/60.0 AS REAL) AS hours FROM projects p LEFT JOIN tasks t ON t.project_id=p.id LEFT JOIN time_entries e ON e.task_id=t.id AND e.started_at>=? AND e.started_at<? WHERE (? IS NULL OR p.id=?) GROUP BY p.id ORDER BY p.sort_order,p.id",vec![json!(start),json!(end),json!(q.project_id),json!(q.project_id)]).await?);
        out["counts"] = json!({"overdue":out["overdue"].as_array().unwrap().len(),"due_today":out["due_today"].as_array().unwrap().len(),"tomorrow":out["tomorrow"].as_array().unwrap().len(),"meetings":out["meetings"].as_array().unwrap().len()});
    }
    out["fingerprint"] = json!(hash(&out));
    Ok(out)
}
pub async fn graph(conn: &mut SqliteConnection, id: &str) -> Result<Value> {
    let m = recurrence::root(conn, id).await?;
    let mut v = json!({"root":m});
    for (key,sql) in [("segments","SELECT s.* FROM meeting_segments s JOIN meeting_versions v ON v.id=s.version_id WHERE v.meeting_id=? ORDER BY s.id"),("segment_tasks","SELECT l.* FROM meeting_segment_tasks l JOIN meeting_segments s ON s.id=l.segment_id JOIN meeting_versions v ON v.id=s.version_id WHERE v.meeting_id=? ORDER BY l.segment_id,l.task_id"),("occurrences","SELECT * FROM meeting_occurrences WHERE meeting_id=? ORDER BY ordinal"),("occurrence_tasks","SELECT * FROM meeting_occurrence_tasks WHERE meeting_id=? ORDER BY occurrence_key,task_id"),("cutoffs","SELECT * FROM meeting_cutoffs WHERE meeting_id=? ORDER BY from_ordinal")]{v[key]=json!(rows(conn,sql,vec![json!(id)]).await?);}
    Ok(v)
}
async fn save_snapshot(
    conn: &mut SqliteConnection,
    m: &Value,
    n: i64,
    d: &Value,
    now: &str,
) -> Result<()> {
    let mut s = json!({});
    for k in FIELDS {
        if k != "task_ids" {
            s[k] = d[k].clone();
        }
    }
    execute(conn,"INSERT INTO meeting_occurrences(meeting_id,occurrence_key,ordinal,snapshot_json,actual_starts_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(meeting_id,occurrence_key) DO UPDATE SET snapshot_json=excluded.snapshot_json,actual_starts_at=excluded.actual_starts_at,revision=revision+1,updated_at=excluded.updated_at",vec![m["id"].clone(),json!(key(n)),json!(n),json!(s.to_string()),d["starts_at"].clone(),json!(now)]).await?;
    execute(
        conn,
        "DELETE FROM meeting_occurrence_tasks WHERE meeting_id=? AND occurrence_key=?",
        vec![m["id"].clone(), json!(key(n))],
    )
    .await?;
    for t in d["task_ids"].as_array().unwrap() {
        insert(
            conn,
            "meeting_occurrence_tasks",
            &json!({"meeting_id":m["id"],"occurrence_key":key(n),"task_id":t}),
        )
        .await?;
    }
    Ok(())
}
async fn protected(
    conn: &mut SqliteConnection,
    m: &Value,
    n: i64,
    now: &str,
) -> Result<Vec<(i64, Value)>> {
    let at = parse(now)?;
    let mut ns = std::collections::BTreeSet::new();
    for s in recurrence::segments(conn, m).await? {
        let a = NaiveDateTime::parse_from_str(text(&s, "anchor_local")?, "%Y-%m-%dT%H:%M:%S%.f")
            .map_err(|_| AppError::validation("Invalid anchor."))?;
        let from = number(&s, "from_ordinal")?;
        let elapsed = (at
            .with_timezone(&zone(text(&s, "time_zone")?)?)
            .date_naive()
            - a.date())
        .num_days();
        let max = from + recurrence::cycle_bounds(&s, 0, elapsed)?.1;
        let max = max.min(s["to_ordinal"].as_i64().unwrap_or(max));
        let max = if s["repeat_rule"] == "none" {
            max.min(from)
        } else {
            max
        };
        if max - n.max(from) > 10_000 {
            return Err(AppError::validation("This change would snapshot more than 10,000 historical meetings. Choose a later occurrence."));
        }
        for i in n.max(from)..=max {
            ns.insert(i);
        }
    }
    for v in rows(conn,"SELECT ordinal FROM meeting_occurrences WHERE meeting_id=? AND ordinal>=? AND actual_starts_at<?",vec![m["id"].clone(),json!(n),json!(now)]).await?{ns.insert(number(&v,"ordinal")?);}
    let mut out = vec![];
    for i in ns {
        if let Some(o) = recurrence::resolve(conn, m, i).await? {
            if parse(text(&o, "starts_at")?)? < at {
                out.push((i, o));
            }
        }
    }
    Ok(out)
}
fn reference(p: &Value) -> Result<MeetingRef> {
    serde_json::from_value(p["ref"].clone())
        .map_err(|_| AppError::validation("Invalid meeting reference."))
}
async fn validate_change(conn: &mut SqliteConnection, change: &Value) -> Result<Value> {
    let action = text(change, "action")?;
    let p = &change["payload"];
    let allowed = match action {
        "create" => vec!["projectId", "draft"],
        "edit_occurrence" | "edit_following" => {
            vec!["ref", "expectedRevision", "draft", "occurrence"]
        }
        "record" => vec![
            "ref",
            "expectedRevision",
            "attendance",
            "occurrence_notes_md",
        ],
        "remove" => vec!["ref", "expectedRevision", "scope"],
        "move_due" => vec!["taskId", "dueAt", "expectedRevision"],
        _ => return Err(AppError::validation("Unknown agenda action.")),
    };
    if change.as_object().is_none_or(|m| m.len() != 2)
        || p.as_object()
            .is_none_or(|m| m.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        return Err(AppError::validation("Unknown agenda field."));
    }
    let mut c = change.clone();
    if ["create", "edit_occurrence", "edit_following"].contains(&action) {
        c["payload"]["draft"] = normalize(&p["draft"], action != "edit_occurrence")?;
        for id in c["payload"]["draft"]["task_ids"].as_array().unwrap() {
            one(conn, "SELECT id FROM tasks WHERE id=?", vec![id.clone()]).await?;
        }
    }
    if let Some(record) = p.get("occurrence") {
        if record.as_object().is_none_or(|m| {
            m.len() != 2
                || m.keys()
                    .any(|k| !["attendance", "occurrence_notes_md"].contains(&k.as_str()))
        }) {
            return Err(AppError::validation(
                "Invalid occurrence attendance and notes.",
            ));
        }
        choice(
            text(record, "attendance")?,
            &["unmarked", "attended", "missed"],
        )?;
        text(record, "occurrence_notes_md")?;
    }
    if action == "create" {
        one(
            conn,
            "SELECT id FROM projects WHERE id=?",
            vec![json!(text(p, "projectId")?)],
        )
        .await?;
    } else if action == "move_due" {
        let id = text(p, "taskId")?;
        check_revision(conn, id, number(p, "expectedRevision")?).await?;
        c["payload"]["dueAt"] = if p["dueAt"].is_null() {
            Value::Null
        } else {
            json!(instant(text(p, "dueAt")?)?)
        };
    } else {
        let r = reference(p)?;
        let m = recurrence::root(conn, &r.meeting_id).await?;
        if m["revision"].as_i64() != Some(number(p, "expectedRevision")?) {
            return Err(AppError::conflict());
        }
        recurrence::resolve(conn, &m, r.ordinal()?)
            .await?
            .ok_or_else(AppError::missing)?;
        if action == "record" {
            choice(text(p, "attendance")?, &["unmarked", "attended", "missed"])?;
            text(p, "occurrence_notes_md")?;
        }
        if action == "remove" {
            choice(text(p, "scope")?, &["one_off", "occurrence", "following"])?;
            if p["scope"] == "one_off" {
                if is_series(conn, &m).await? {
                    return Err(AppError::validation(
                        "Choose an occurrence or following cancellation for a series.",
                    ));
                }
            }
        }
    }
    Ok(c)
}
pub async fn preview_into(conn: &mut SqliteConnection, change: &Value, now: &str) -> Result<Value> {
    let c = validate_change(conn, change).await?;
    let p = &c["payload"];
    let r = reference(p)?;
    let m = recurrence::root(conn, &r.meeting_id).await?;
    let n = r.ordinal()?;
    let past = protected(conn, &m, n, now).await?;
    if c["action"] == "remove" && p["scope"] == "following" && !past.is_empty() {
        return Err(AppError::validation("Following cancellation would affect past meetings. Cancel this occurrence or choose a later occurrence."));
    }
    let mut overrides = vec![];
    for s in rows(conn,"SELECT ordinal FROM meeting_occurrences WHERE meeting_id=? AND ordinal>? AND snapshot_json IS NOT NULL ORDER BY ordinal",vec![m["id"].clone(),json!(n)]).await?{if let Some(o)=recurrence::resolve(conn,&m,number(&s,"ordinal")?).await?{overrides.push(o);}}
    let g = graph(conn, &r.meeting_id).await?;
    let fingerprint =
        hash(&json!({"change":c,"graph":g,"past":past.iter().map(|v|v.0).collect::<Vec<_>>()}));
    let at = parse(text(
        &recurrence::resolve(conn, &m, n).await?.unwrap(),
        "starts_at",
    )?)?;
    let mut skipped = Vec::<String>::new();
    if c["action"] == "edit_following" && p["draft"]["repeat_rule"] != "none" {
        let d = &p["draft"];
        let anchor = NaiveDateTime::parse_from_str(text(d, "start_local")?, "%Y-%m-%dT%H:%M:%S%.f")
            .map_err(|_| AppError::validation("Invalid local time."))?;
        let tz = zone(text(d, "time_zone")?)?;
        for i in 1..7 {
            let local = anchor + Duration::days(i);
            if d["repeat_rule"] == "weekly"
                && !d["repeat_weekdays"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(local.weekday().number_from_monday()))
            {
                continue;
            }
            if d["repeat_until"]
                .as_str()
                .is_none_or(|e| local.date().to_string().as_str() <= e)
                && tz.from_local_datetime(&local) == chrono::LocalResult::None
            {
                skipped.push(format!(
                    "{} skipped: this time does not exist in {}",
                    local.date(),
                    tz
                ));
            }
        }
    }
    let diagnostic_start = if c["action"] == "edit_following" {
        parse(text(&p["draft"], "starts_at")?)?
    } else {
        at
    };
    Ok(
        json!({"fingerprint":fingerprint,"affected_counts":{"stored_occurrences":g["occurrences"].as_array().unwrap().len(),"linked_tasks":g["occurrence_tasks"].as_array().unwrap().len()+g["segment_tasks"].as_array().unwrap().len()},"preserved_overrides":overrides,"skipped_dates":skipped,"diagnostic_range":{"start":utc(diagnostic_start),"end":utc(diagnostic_start+Duration::days(7))},"warnings":if past.is_empty(){vec![]}else{vec!["Past occurrences keep their existing schedule and notes."]}}),
    )
}
async fn reply(conn: &mut SqliteConnection, outcome: &Value, replayed: bool) -> Result<Value> {
    let mut details = vec![];
    let mut projects = outcome["affected_project_ids"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut tasks = outcome["affected_task_ids"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut detail = Value::Null;
    if let Some(id) = outcome["task_id"].as_str() {
        match crate::workspace::models::detail(conn, id).await {
            Ok(d) => {
                projects.push(d["task"]["project_id"].clone());
                tasks.push(json!(id));
                details.push(d);
            }
            Err(e) if e.code == "NotFound" => {}
            Err(e) => return Err(e),
        }
    }
    if !outcome["ref"].is_null() {
        let r: MeetingRef = serde_json::from_value(outcome["ref"].clone())
            .map_err(|_| AppError::validation("Invalid receipt."))?;
        match meeting_detail(conn, &r).await {
            Ok(d) => {
                projects.push(d["occurrence"]["project_id"].clone());
                tasks.extend(
                    d["linked_tasks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v["id"].clone()),
                );
                detail = d;
            }
            Err(e) if e.code == "NotFound" => {}
            Err(e) => return Err(e),
        }
    }
    projects.sort_by_key(Value::to_string);
    projects.dedup();
    tasks.sort_by_key(Value::to_string);
    tasks.dedup();
    let revision = detail["occurrence"]["revision"].clone();
    Ok(
        json!({"detail":detail,"outcome":outcome,"changed_details":details,"affected_project_ids":projects,"affected_task_ids":tasks,"revision":revision,"replayed":replayed}),
    )
}
pub async fn apply_into(p: &SqlitePool, input: AgendaInput, now: &str) -> Result<Value> {
    if input.request_id.trim().is_empty() {
        return Err(AppError::validation("Missing request identity."));
    }
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    let digest =
        hash(&json!({"change":input.change,"expectedFingerprint":input.expected_fingerprint}));
    if let Some(r) = rows(
        &mut tx,
        "SELECT * FROM agenda_requests WHERE request_id=?",
        vec![json!(input.request_id)],
    )
    .await?
    .pop()
    {
        if r["payload_hash"] != digest {
            return Err(AppError::conflict());
        }
        let o = serde_json::from_str(text(&r, "outcome_json")?).map_err(|_| {
            AppError::new(
                "UnknownOutcome",
                "Cannot read the committed agenda receipt.",
            )
        })?;
        // This receipt proves an earlier write committed. A readback failure
        // must never unlock a replacement write with a new request identity.
        let result = reply(&mut tx, &o, true)
            .await
            .map_err(|e| AppError::new("UnknownOutcome", e.message))?;
        tx.commit()
            .await
            .map_err(|e| AppError::new("UnknownOutcome", e.to_string()))?;
        return Ok(result);
    }
    let c = validate_change(&mut tx, &input.change).await?;
    let action = text(&c, "action")?;
    let payload = &c["payload"];
    if action == "edit_following" || action == "remove" {
        let v = preview_into(&mut tx, &c, now).await?;
        if input.expected_fingerprint.as_deref() != v["fingerprint"].as_str() {
            return Err(AppError::conflict());
        }
    }
    let mut outcome = json!({"ref":null,"removed":false});
    if action == "move_due" {
        let id = text(payload, "taskId")?;
        let old = check_revision(&mut tx, id, number(payload, "expectedRevision")?).await?;
        if old["due_at"] != payload["dueAt"] {
            execute(
                &mut tx,
                "UPDATE tasks SET due_at=? WHERE id=?",
                vec![payload["dueAt"].clone(), json!(id)],
            )
            .await?;
            bump(&mut tx, id, now).await?;
        }
        outcome["task_id"] = json!(id);
    } else if action == "create" {
        let id = uid();
        let d = &payload["draft"];
        let mut m = json!({"id":id,"project_id":payload["projectId"]});
        for k in [
            "title",
            "starts_at",
            "duration_min",
            "link_url",
            "agenda_md",
            "notes_md",
            "repeat_rule",
            "reminder_min",
            "time_zone",
            "show_in_day",
        ] {
            m[k] = d[k].clone();
        }
        insert(&mut tx, "meetings", &m).await?;
        let v = uid();
        insert(
            &mut tx,
            "meeting_versions",
            &json!({"id":v,"meeting_id":id,"created_at":now}),
        )
        .await?;
        segment(&mut tx, &v, 0, None, d).await?;
        execute(
            &mut tx,
            "UPDATE meetings SET active_version_id=? WHERE id=?",
            vec![json!(v), json!(id)],
        )
        .await?;
        for t in d["task_ids"].as_array().unwrap() {
            insert(
                &mut tx,
                "meeting_tasks",
                &json!({"meeting_id":id,"task_id":t}),
            )
            .await?;
        }
        outcome["ref"] = json!({"meetingId":id,"occurrenceKey":key(0)});
    } else {
        let r = reference(payload)?;
        let m = recurrence::root(&mut tx, &r.meeting_id).await?;
        let n = r.ordinal()?;
        let old = recurrence::resolve(&mut tx, &m, n)
            .await?
            .ok_or_else(AppError::missing)?;
        outcome["ref"] = json!(r);
        let g = graph(&mut tx, &r.meeting_id).await?;
        let mut affected = std::collections::BTreeSet::<String>::new();
        for k in ["segment_tasks", "occurrence_tasks"] {
            for link in g[k].as_array().unwrap() {
                affected.insert(text(link, "task_id")?.into());
            }
        }
        if let Some(ids) = payload["draft"]["task_ids"].as_array() {
            for id in ids {
                affected.insert(id.as_str().unwrap().into());
            }
        }
        let mut owners =
            std::collections::BTreeSet::<String>::from([text(&m, "project_id")?.into()]);
        for id in &affected {
            owners.insert(
                text(
                    &one(
                        &mut tx,
                        "SELECT project_id FROM tasks WHERE id=?",
                        vec![json!(id)],
                    )
                    .await?,
                    "project_id",
                )?
                .into(),
            );
        }
        outcome["affected_task_ids"] = json!(affected);
        outcome["affected_project_ids"] = json!(owners);
        match action {
            "edit_occurrence" => save_snapshot(&mut tx, &m, n, &payload["draft"], now).await?,
            "record" => {
                save_snapshot(&mut tx, &m, n, &old, now).await?;
                execute(&mut tx,"UPDATE meeting_occurrences SET attendance=?,occurrence_notes_md=? WHERE meeting_id=? AND ordinal=?",vec![payload["attendance"].clone(),payload["occurrence_notes_md"].clone(),m["id"].clone(),json!(n)]).await?;
            }
            "edit_following" => {
                let past = protected(&mut tx, &m, n, now).await?;
                for (i, o) in &past {
                    if rows(&mut tx,"SELECT 1 FROM meeting_occurrences WHERE meeting_id=? AND ordinal=? AND snapshot_json IS NOT NULL",vec![m["id"].clone(),json!(i)]).await?.is_empty(){save_snapshot(&mut tx,&m,*i,o,now).await?;}
                }
                let v = uid();
                insert(
                    &mut tx,
                    "meeting_versions",
                    &json!({"id":v,"meeting_id":m["id"],"created_at":now}),
                )
                .await?;
                for mut s in recurrence::segments(&mut tx, &m).await? {
                    if number(&s, "from_ordinal")? >= n {
                        continue;
                    }
                    let sid = s["id"].clone();
                    s["id"] = json!(uid());
                    s["version_id"] = json!(v);
                    s["to_ordinal"] = json!(s["to_ordinal"].as_i64().unwrap_or(n).min(n));
                    insert(&mut tx, "meeting_segments", &s).await?;
                    for l in rows(
                        &mut tx,
                        "SELECT task_id FROM meeting_segment_tasks WHERE segment_id=?",
                        vec![sid],
                    )
                    .await?
                    {
                        insert(
                            &mut tx,
                            "meeting_segment_tasks",
                            &json!({"segment_id":s["id"],"task_id":l["task_id"]}),
                        )
                        .await?;
                    }
                }
                segment(&mut tx, &v, n, None, &payload["draft"]).await?;
                execute(
                    &mut tx,
                    "UPDATE meetings SET active_version_id=? WHERE id=?",
                    vec![json!(v), m["id"].clone()],
                )
                .await?;
                if !past.iter().any(|v| v.0 == n) {
                    save_snapshot(&mut tx, &m, n, &payload["draft"], now).await?;
                }
                execute(
                    &mut tx,
                    "DELETE FROM meeting_tasks WHERE meeting_id=?",
                    vec![m["id"].clone()],
                )
                .await?;
                for t in payload["draft"]["task_ids"].as_array().unwrap() {
                    insert(
                        &mut tx,
                        "meeting_tasks",
                        &json!({"meeting_id":m["id"],"task_id":t}),
                    )
                    .await?;
                }
            }
            "remove" => {
                outcome["removed"] = json!(true);
                if payload["scope"] == "one_off" {
                    execute(
                        &mut tx,
                        "DELETE FROM meeting_tasks WHERE meeting_id=?",
                        vec![m["id"].clone()],
                    )
                    .await?;
                    execute(
                        &mut tx,
                        "DELETE FROM meetings WHERE id=?",
                        vec![m["id"].clone()],
                    )
                    .await?;
                } else if payload["scope"] == "following" {
                    execute(&mut tx,"INSERT OR IGNORE INTO meeting_cutoffs(meeting_id,from_ordinal,created_at) VALUES(?,?,?)",vec![m["id"].clone(),json!(n),json!(now)]).await?;
                } else {
                    save_snapshot(&mut tx, &m, n, &old, now).await?;
                    execute(&mut tx,"UPDATE meeting_occurrences SET cancelled=1 WHERE meeting_id=? AND ordinal=?",vec![m["id"].clone(),json!(n)]).await?;
                }
            }
            _ => unreachable!(),
        }
        // The main Save meeting action commits the schedule and this occurrence's
        // attendance/notes in the same transaction and under one replay identity.
        if let Some(record) = payload.get("occurrence") {
            execute(&mut tx,
                "UPDATE meeting_occurrences SET attendance=?,occurrence_notes_md=? WHERE meeting_id=? AND ordinal=?",
                vec![record["attendance"].clone(), record["occurrence_notes_md"].clone(), m["id"].clone(), json!(n)]).await?;
        }
        execute(
            &mut tx,
            "UPDATE meetings SET revision=revision+1 WHERE id=?",
            vec![m["id"].clone()],
        )
        .await?;
    }
    let result = reply(&mut tx, &outcome, false).await?;
    insert(&mut tx,"agenda_requests",&json!({"request_id":input.request_id,"payload_hash":digest,"outcome_json":outcome.to_string()})).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::new("UnknownOutcome", e.to_string()))?;
    Ok(result)
}
#[tauri::command]
pub async fn initialize_agenda(app: AppHandle, time_zone: String) -> Result<()> {
    initialize_into(
        &pool(&app).await?,
        &time_zone,
        &crate::clock::sample(&app)?.0,
    )
    .await
}
#[tauri::command]
pub async fn get_today(app: AppHandle, query: AgendaQuery) -> Result<Value> {
    let mut tx = pool(&app).await?.begin().await?;
    let v = snapshot(&mut tx, &query, &crate::clock::sample(&app)?.0, false).await?;
    tx.commit().await?;
    Ok(v)
}
#[tauri::command]
pub async fn get_week(app: AppHandle, query: AgendaQuery) -> Result<Value> {
    let mut tx = pool(&app).await?.begin().await?;
    let v = snapshot(&mut tx, &query, &crate::clock::sample(&app)?.0, true).await?;
    tx.commit().await?;
    Ok(v)
}
#[tauri::command]
pub async fn get_meeting_detail(app: AppHandle, reference: MeetingRef) -> Result<Value> {
    let mut tx = pool(&app).await?.begin().await?;
    let v = meeting_detail(&mut tx, &reference).await?;
    tx.commit().await?;
    Ok(v)
}
#[tauri::command]
pub async fn preview_meeting_change(app: AppHandle, change: Value) -> Result<Value> {
    let mut tx = pool(&app).await?.begin().await?;
    let v = preview_into(&mut tx, &change, &crate::clock::sample(&app)?.0).await?;
    tx.commit().await?;
    Ok(v)
}
#[tauri::command]
pub async fn apply_meeting(app: AppHandle, input: AgendaInput) -> Result<Value> {
    apply_into(&pool(&app).await?, input, &crate::clock::sample(&app)?.0).await
}
#[tauri::command]
pub async fn get_task_meetings(
    app: AppHandle,
    task_id: String,
    query: AgendaQuery,
) -> Result<Value> {
    let mut tx = pool(&app).await?.begin().await?;
    one(
        &mut tx,
        "SELECT id FROM tasks WHERE id=?",
        vec![json!(task_id)],
    )
    .await?;
    let d = date(&query.date)?;
    let mon = d - Duration::days(d.weekday().num_days_from_monday() as i64);
    let start = bounds(mon, &query.time_zone)?.0;
    let end = bounds(mon + Duration::days(6), &query.time_zone)?.1;
    let os = occurrences(&mut tx, &start, &end, None, false)
        .await?
        .into_iter()
        .filter(|m| m["task_ids"].as_array().unwrap().contains(&json!(task_id)))
        .collect::<Vec<_>>();
    tx.commit().await?;
    Ok(
        json!({"task_id":task_id,"week_start":mon.to_string(),"range_start_utc":start,"range_end_utc":end,"occurrences":os,"previous_date":(mon-Duration::days(7)).to_string(),"next_date":(mon+Duration::days(7)).to_string()}),
    )
}
#[tauri::command]
pub async fn search_agenda_tasks(app: AppHandle, input: Value) -> Result<Value> {
    let limit = number(&input, "limit")?;
    if !(1..=50).contains(&limit) {
        return Err(AppError::validation("Search limit must be 1–50."));
    }
    let needle = text(&input, "text")?.to_lowercase();
    let mut tx = pool(&app).await?.begin().await?;
    let mut tasks = task_rows(&mut tx).await?;
    tasks.retain(|v| {
        format!(
            "{} {} {}",
            v["title"].as_str().unwrap_or(""),
            v["project_name"].as_str().unwrap_or(""),
            v["external_id"].as_str().unwrap_or("")
        )
        .to_lowercase()
        .contains(&needle)
    });
    let sort_key = |v: &Value| {
        [
            v["project_name"].as_str().unwrap_or("").to_owned(),
            v["title"].as_str().unwrap_or("").to_owned(),
            v["id"].as_str().unwrap_or("").to_owned(),
        ]
    };
    if let Some(cursor) = input["cursor"].as_str() {
        let c: Value = serde_json::from_str(cursor)
            .map_err(|_| AppError::validation("Invalid search cursor."))?;
        if c["query"] != needle {
            return Err(AppError::validation("Search changed; start a new search."));
        }
        let after: [String; 3] = serde_json::from_value(c["after"].clone())
            .map_err(|_| AppError::validation("Invalid search cursor."))?;
        tasks.retain(|v| sort_key(v) > after);
    }
    let page = tasks
        .iter()
        .take(limit as usize)
        .cloned()
        .collect::<Vec<_>>();
    let cursor = if page.len() < tasks.len() {
        json!(json!({"query":needle,"after":sort_key(page.last().unwrap())}).to_string())
    } else {
        Value::Null
    };
    tx.commit().await?;
    Ok(json!({"items":page,"nextCursor":cursor}))
}
#[cfg(test)]
mod tests;
