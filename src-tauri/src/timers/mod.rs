pub mod models;
use crate::{clock, workspace::models::*};
use models::*;
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use tauri::{AppHandle, Manager};
pub const MAX_MS: i64 = 9_007_199_254_740_991;
pub const FIXTURE: &str = "2025-09-11T17:42:00.000Z";
async fn pool(app: &AppHandle) -> Result<SqlitePool> {
    crate::db::pool(app)
        .await
        .map_err(|e| AppError::new("Database", e))
}
pub async fn snapshot(conn: &mut SqliteConnection, now: &str, offset: i64) -> Result<Value> {
    Ok(
        json!({"now_utc":now,"offset_ms":offset,"sessions":rows(conn,"SELECT s.*, t.title AS task_title, t.project_id, p.name AS project_name FROM timer_sessions s JOIN tasks t ON t.id=s.task_id JOIN projects p ON p.id=t.project_id WHERE s.state!='stopped' ORDER BY p.sort_order,p.id,t.title,s.task_id,s.id",vec![]).await?}),
    )
}
async fn entries(conn: &mut SqliteConnection, task: &str) -> Result<Vec<Value>> {
    rows(
        conn,
        "SELECT * FROM time_entries WHERE task_id=? ORDER BY started_at DESC,id DESC",
        vec![json!(task)],
    )
    .await
}
async fn reply(
    conn: &mut SqliteConnection,
    task: &str,
    now: &str,
    offset: i64,
    session: Value,
    entry: Value,
) -> Result<Value> {
    Ok(
        json!({"snapshot":snapshot(conn,now,offset).await?,"detail":detail(conn,task).await?,"entries":entries(conn,task).await?,"outcome":{"session_id":session,"entry_id":entry}}),
    )
}
pub async fn initialize_into(
    pool: &SqlitePool,
    seeded: bool,
    real_now: &str,
) -> Result<(i64, Vec<String>)> {
    if !seeded {
        return Ok((0, vec![]));
    }
    if !cfg!(debug_assertions) {
        return Err(AppError::validation("Fixtures require a debug build."));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let marker = rows(
        &mut tx,
        "SELECT value FROM settings WHERE key='fixture_version'",
        vec![],
    )
    .await?;
    if marker.first().and_then(|v| v["value"].as_str()) != Some("september-2025-midday-v1") {
        return Err(AppError::validation("M0 fixture marker is missing."));
    }
    let old = rows(
        &mut tx,
        "SELECT value FROM settings WHERE key='fixture_timers_m2'",
        vec![],
    )
    .await?;
    let anchor = rows(
        &mut tx,
        "SELECT value FROM settings WHERE key='fixture_clock_m2'",
        vec![],
    )
    .await?;
    let (anchor, warnings) = if old.is_empty() {
        if !anchor.is_empty() {
            return Err(AppError::validation(
                "Incomplete fixture clock installation.",
            ));
        }
        let anchor = json!({"real_anchor_utc":instant(real_now)?,"fixture_anchor_utc":FIXTURE});
        let mut warnings = vec![];
        for (suffix, elapsed, label) in [
            ("00000000006c", 2537000, "ATL-477"),
            ("000000000074", 2529000, "Parse config flags"),
        ] {
            let task = format!("f870c681-27a4-4d65-87be-{suffix}");
            let exists = rows(
                &mut tx,
                "SELECT id FROM tasks WHERE id=?",
                vec![json!(task)],
            )
            .await?;
            let live = rows(
                &mut tx,
                "SELECT id FROM timer_sessions WHERE task_id=? AND state!='stopped'",
                vec![json!(task)],
            )
            .await?;
            if exists.is_empty() || !live.is_empty() {
                warnings.push(format!(
                    "Fixture timer for {label} skipped: task missing or session already present."
                ));
                continue;
            }
            let start = clock::iso(clock::millis(FIXTURE)? - elapsed)?;
            insert(&mut tx,"timer_sessions",&json!({"id":uid(),"task_id":task,"state":"running","started_at":start,"segment_started_at":start,"accumulated_ms":0,"revision":0})).await?;
        }
        insert(
            &mut tx,
            "settings",
            &json!({"key":"fixture_clock_m2","value":anchor.to_string()}),
        )
        .await?;
        insert(&mut tx,"settings",&json!({"key":"fixture_timers_m2","value":json!({"version":1,"warnings":warnings}).to_string()})).await?;
        (anchor, warnings)
    } else {
        let decode = |v: &Value| -> Result<Value> {
            serde_json::from_str(text(v, "value")?)
                .map_err(|_| AppError::validation("Corrupt fixture clock settings."))
        };
        let saved = decode(&old[0])?;
        if saved["version"] != 1 {
            return Err(AppError::validation("Unknown fixture timer version."));
        }
        let warnings: Vec<String> = serde_json::from_value(saved["warnings"].clone())
            .map_err(|_| AppError::validation("Corrupt fixture warnings."))?;
        (
            decode(
                anchor
                    .first()
                    .ok_or_else(|| AppError::validation("Fixture clock anchor is missing."))?,
            )?,
            warnings,
        )
    };
    if text(&anchor, "fixture_anchor_utc")? != FIXTURE {
        return Err(AppError::validation("Unexpected fixture anchor."));
    }
    let offset = clock::millis(text(&anchor, "fixture_anchor_utc")?)?
        .checked_sub(clock::millis(text(&anchor, "real_anchor_utc")?)?)
        .ok_or_else(|| AppError::validation("Clock overflow."))?;
    tx.commit().await?;
    Ok((offset, warnings))
}
#[tauri::command]
pub async fn initialize_timers(app: AppHandle) -> Result<Value> {
    let clock = app.state::<clock::Clock>();
    let _gate = clock.initialize_gate.lock().await;
    let pool = pool(&app).await?;
    let real = clock::iso(chrono::Utc::now().timestamp_millis())?;
    let (offset, warnings) =
        initialize_into(&pool, app.state::<crate::db::RuntimeState>().seeded, &real).await?;
    *clock
        .offset
        .write()
        .map_err(|_| AppError::new("Database", "Clock lock failed."))? = Some(offset);
    let (now, _) = clock::sample(&app)?;
    let mut tx = pool.begin().await?;
    let mut result = snapshot(&mut tx, &now, offset).await?;
    tx.commit().await?;
    result["warnings"] = json!(warnings);
    if let Err(e) = crate::tray::refresh(&app).await {
        eprintln!("Timer tooltip: {}", e.message);
    }
    Ok(result)
}
#[tauri::command]
pub async fn get_timers(app: AppHandle) -> Result<Value> {
    let (now, offset) = clock::sample(&app)?;
    let pool = pool(&app).await?;
    let mut tx = pool.begin().await?;
    let result = snapshot(&mut tx, &now, offset).await?;
    tx.commit().await?;
    Ok(result)
}
#[tauri::command]
pub async fn get_task_time(app: AppHandle, task_id: String) -> Result<Value> {
    let (now, offset) = clock::sample(&app)?;
    let pool = pool(&app).await?;
    let mut tx = pool.begin().await?;
    one(
        &mut tx,
        "SELECT id FROM tasks WHERE id=?",
        vec![json!(task_id)],
    )
    .await?;
    let result = json!({"snapshot":snapshot(&mut tx,&now,offset).await?,"entries":entries(&mut tx,&task_id).await?});
    tx.commit().await?;
    Ok(result)
}
pub async fn mutate(
    pool: &SqlitePool,
    kind: &str,
    input: Value,
    now: &str,
    offset: i64,
) -> Result<Value> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let result = mutate_in_transaction(&mut tx, kind, input, now, offset).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn mutate_in_transaction(
    conn: &mut SqliteConnection,
    kind: &str,
    mut input: Value,
    now: &str,
    offset: i64,
) -> Result<Value> {
    choice(kind, &["start", "pause", "resume", "stop", "log"])?;
    let request = text(&input, "requestId")?.to_owned();
    let task = text(&input, "taskId")?.to_owned();
    nonblank(&request)?;
    nonblank(&task)?;
    if kind == "log" {
        input["startedAt"] = json!(instant(text(&input, "startedAt")?)?);
    }
    let mut payload = input.clone();
    payload
        .as_object_mut()
        .ok_or_else(|| AppError::validation("Expected input."))?
        .remove("requestId");
    payload["kind"] = json!(kind);
    let payload = payload.to_string();
    let now = instant(now)?;
    one(conn, "SELECT id FROM tasks WHERE id=?", vec![json!(task)]).await?;
    let receipts = rows(
        conn,
        "SELECT * FROM timer_requests WHERE request_id=?",
        vec![json!(request)],
    )
    .await?;
    if let Some(old) = receipts.first() {
        if old["payload_json"] != payload {
            return Err(AppError::conflict());
        }
        let result = reply(
            conn,
            &task,
            &now,
            offset,
            old["session_id"].clone(),
            old["entry_id"].clone(),
        )
        .await?;
        return Ok(result);
    }
    let mut session_id = Value::Null;
    let mut entry_id = Value::Null;
    if kind == "start" {
        let block = input.get("blockId").cloned().unwrap_or(Value::Null);
        if !block.is_null() {
            one(
                conn,
                "SELECT id FROM blocks WHERE id=? AND task_id=?",
                vec![block.clone(), json!(task)],
            )
            .await?;
        }
        let live = rows(
            conn,
            "SELECT * FROM timer_sessions WHERE task_id=? AND state!='stopped'",
            vec![json!(task)],
        )
        .await?;
        if let Some(live) = live.first() {
            session_id = live["id"].clone();
        } else {
            session_id = json!(uid());
            insert(conn,"timer_sessions",&json!({"id":session_id,"task_id":task,"block_id":block,"state":"running","started_at":now,"segment_started_at":now,"accumulated_ms":0,"revision":0})).await?;
        }
    } else if kind == "log" {
        let duration = input["durationMs"]
            .as_i64()
            .filter(|n| *n > 0 && *n <= MAX_MS)
            .ok_or_else(|| AppError::validation("Enter a positive duration."))?;
        let start = text(&input, "startedAt")?;
        let end = clock::iso(
            clock::millis(start)?
                .checked_add(duration)
                .ok_or_else(|| AppError::validation("Duration overflow."))?,
        )?;
        if end > now {
            return Err(AppError::validation(
                "Logged time cannot end in the future.",
            ));
        }
        entry_id = json!(uid());
        insert(conn,"time_entries",&json!({"id":entry_id,"task_id":task,"block_id":null,"started_at":start,"ended_at":end,"minutes":duration as f64/60000.0})).await?;
        bump(conn, &task, &now).await?;
    } else {
        session_id = json!(text(&input, "sessionId")?);
        let session = one(
            conn,
            "SELECT * FROM timer_sessions WHERE id=? AND task_id=?",
            vec![session_id.clone(), json!(task)],
        )
        .await?;
        let state = text(&session, "state")?;
        let expected = input["expectedRevision"]
            .as_i64()
            .filter(|n| *n >= 0 && *n <= MAX_MS)
            .ok_or_else(|| AppError::validation("Invalid timer revision."))?;
        if state == "stopped" && kind == "stop" {
            entry_id = session["entry_id"].clone();
        } else {
            if state == "stopped" || session["revision"].as_i64() != Some(expected) {
                return Err(AppError::conflict());
            }
            let old = session["accumulated_ms"]
                .as_i64()
                .ok_or_else(|| AppError::validation("Invalid stored duration."))?;
            let segment = if state == "running" {
                (clock::millis(&now)? - clock::millis(text(&session, "segment_started_at")?)?)
                    .max(0)
            } else {
                0
            };
            let elapsed = old
                .checked_add(segment)
                .filter(|v| *v <= MAX_MS)
                .ok_or_else(|| AppError::validation("Timer duration overflow."))?;
            match kind {
    "pause" if state=="running" => execute(conn,"UPDATE timer_sessions SET state='paused',accumulated_ms=?,segment_started_at=NULL,revision=revision+1 WHERE id=?",vec![json!(elapsed),session_id.clone()]).await?,
    "resume" if state=="paused" => execute(conn,"UPDATE timer_sessions SET state='running',segment_started_at=?,revision=revision+1 WHERE id=?",vec![json!(now),session_id.clone()]).await?,
    "stop" => {
     let start=text(&session,"started_at")?;let end=now.as_str().max(start);entry_id=json!(uid());
     insert(conn,"time_entries",&json!({"id":entry_id,"task_id":task,"block_id":session["block_id"],"started_at":start,"ended_at":end,"minutes":elapsed as f64/60000.0})).await?;
     execute(conn,"UPDATE timer_sessions SET state='stopped',segment_started_at=NULL,accumulated_ms=?,ended_at=?,entry_id=?,revision=revision+1 WHERE id=?",vec![json!(elapsed),json!(end),entry_id.clone(),session_id.clone()]).await?;
     bump(conn,&task,&now).await?;
    },_=>()
   }
        }
    }
    insert(conn,"timer_requests",&json!({"request_id":request,"task_id":task,"kind":kind,"payload_json":payload,"session_id":session_id,"entry_id":entry_id})).await?;
    let result = reply(conn, &task, &now, offset, session_id, entry_id).await?;
    Ok(result)
}
async fn command(app: &AppHandle, kind: &str, input: Value) -> Result<Value> {
    let (now, offset) = clock::sample(app)?;
    let result = mutate(&pool(app).await?, kind, input, &now, offset).await?;
    if let Err(e) = crate::tray::refresh(app).await {
        eprintln!("Timer tooltip: {}", e.message);
    }
    Ok(result)
}
#[tauri::command]
pub async fn start_timer(app: AppHandle, input: StartInput) -> Result<Value> {
    command(&app, "start", json!(input)).await
}
#[tauri::command]
pub async fn pause_timer(app: AppHandle, input: SessionInput) -> Result<Value> {
    command(&app, "pause", json!(input)).await
}
#[tauri::command]
pub async fn resume_timer(app: AppHandle, input: SessionInput) -> Result<Value> {
    command(&app, "resume", json!(input)).await
}
#[tauri::command]
pub async fn stop_timer(app: AppHandle, input: SessionInput) -> Result<Value> {
    command(&app, "stop", json!(input)).await
}
#[tauri::command]
pub async fn log_time(app: AppHandle, input: LogInput) -> Result<Value> {
    command(&app, "log", json!(input)).await
}
#[cfg(test)]
mod tests;
