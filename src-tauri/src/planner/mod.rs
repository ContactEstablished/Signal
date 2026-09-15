pub mod models;
use crate::{clock, timers, workspace::models::*};
use chrono::Duration;
use models::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};
use tauri::AppHandle;
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| AppError::validation("Expected a list."))
}
fn integer(v: &Value, key: &str) -> Result<i64> {
    v[key]
        .as_i64()
        .ok_or_else(|| AppError::validation("Expected a whole number."))
}
fn keys(v: &Value, allowed: &[&str]) -> Result<()> {
    if v.as_object()
        .is_none_or(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        return Err(AppError::validation("Unknown planner field."));
    }
    Ok(())
}
fn decode_draft(v: Value) -> Result<BlockDraft> {
    serde_json::from_value(v).map_err(|_| AppError::validation("Invalid block draft."))
}
pub async fn snapshot(
    conn: &mut SqliteConnection,
    q: &PlannerQuery,
    now: &str,
    offset: i64,
) -> Result<Value> {
    let (date, _) = calendar(q)?;
    let (start, end) = bounds(q)?;
    let mut data = json!({"date":q.date,"time_zone":q.time_zone,
     "blocks":rows(conn,"SELECT * FROM blocks WHERE date=? ORDER BY start_min,id",vec![json!(q.date)]).await?,
     "previous_blocks":rows(conn,"SELECT * FROM blocks WHERE date=? ORDER BY start_min,id",vec![json!((date-Duration::days(1)).to_string())]).await?,
     "copy_sources":rows(conn,"SELECT * FROM blocks WHERE date=? ORDER BY start_min,id",vec![json!(previous_monday(date).to_string())]).await?,
     "tasks":rows(conn,"SELECT t.*,p.name AS project_name,p.color AS project_color FROM tasks t JOIN projects p ON p.id=t.project_id ORDER BY t.id",vec![]).await?,
     "entries":rows(conn,"SELECT * FROM time_entries WHERE started_at>=? AND started_at<? ORDER BY id",vec![json!(utc(start)),json!(utc(end))]).await?,
     "claims":rows(conn,"SELECT * FROM planner_claims ORDER BY id",vec![]).await?,
     "dismissed":!rows(conn,"SELECT key FROM settings WHERE key=? AND value='true'",vec![json!(format!("planner_dismissed:{}",q.date))]).await?.is_empty()
    });
    let meetings = crate::agenda::occurrences(conn, &utc(start), &utc(end), None, true).await?;
    data["meetings"] = json!(meetings);
    let timer = timers::snapshot(conn, now, offset).await?;
    data["timers"] = json!({"sessions":timer["sessions"]});
    data["fingerprint"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&data).unwrap())
    ));
    data["timers"] = timer;
    Ok(data)
}
async fn reply(
    conn: &mut SqliteConnection,
    q: &PlannerQuery,
    now: &str,
    offset: i64,
    outcome: Value,
) -> Result<Value> {
    let mut details = vec![];
    let mut entries = serde_json::Map::new();
    if let Some(task) = outcome["task_id"].as_str() {
        if !rows(conn, "SELECT id FROM tasks WHERE id=?", vec![json!(task)])
            .await?
            .is_empty()
        {
            details.push(detail(conn, task).await?);
            entries.insert(task.into(),json!(rows(conn,"SELECT * FROM time_entries WHERE task_id=? ORDER BY started_at DESC,id DESC",vec![json!(task)]).await?));
        }
    }
    Ok(
        json!({"snapshot":snapshot(conn,q,now,offset).await?,"outcome":outcome,"changed_details":details,"changed_entries":entries}),
    )
}
async fn add(
    conn: &mut SqliteConnection,
    q: &PlannerQuery,
    mut draft: BlockDraft,
    carry: bool,
) -> Result<String> {
    validate_draft(&mut draft, q)?;
    if let Some(task) = &draft.task_id {
        one(conn, "SELECT id FROM tasks WHERE id=?", vec![json!(task)]).await?;
    }
    let id = uid();
    insert(conn,"blocks",&json!({"id":id,"date":q.date,"kind":draft.kind,"task_id":draft.task_id,"start_min":draft.start_min,"end_min":draft.end_min,"time_zone":q.time_zone,"start_offset":draft.start_offset,"end_offset":draft.end_offset,"carried_from_block_id":if carry {draft.source_id} else {None}})).await?;
    Ok(id)
}
fn claimed(data: &Value, key: &str) -> bool {
    data["claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == key)
}
fn duration(task: &Value) -> Option<i64> {
    if task["estimate_h"].is_null() {
        return Some(30);
    }
    let remaining = (task["estimate_h"].as_f64()? - task["hours_worked"].as_f64()?) * 60.0;
    if remaining <= 0.0 {
        None
    } else {
        Some(((remaining / 15.0).ceil() as i64 * 15).min(120))
    }
}
async fn batch(
    conn: &mut SqliteConnection,
    input: &PlannerInput,
    data: &Value,
    now: &str,
) -> Result<Vec<String>> {
    keys(&input.payload, &["mode", "blocks"])?;
    let mode = text(&input.payload, "mode")?;
    choice(mode, &["quick", "carry", "due", "copy"])?;
    let floor = floor(&input.query, now)?;
    let mut occupied: Vec<(i64, i64)> = array(&data["blocks"])?
        .iter()
        .map(|b| {
            (
                b["start_min"].as_i64().unwrap(),
                b["end_min"].as_i64().unwrap(),
            )
        })
        .collect();
    for m in array(&data["meetings"])? {
        if let Some(r) = meeting_range(m, &input.query)? {
            occupied.push(r);
        }
    }
    let tasks = array(&data["tasks"])?;
    let mut ids = vec![];
    let mut seen = std::collections::HashSet::new();
    let mut last_order: Option<(String, i64, String, String)> = None;
    let drafts = array(&input.payload["blocks"])?;
    if drafts.is_empty() || drafts.len() > 500 || (mode == "quick" && drafts.len() != 1) {
        return Err(AppError::validation("Choose between one and 500 blocks."));
    }
    for value in drafts {
        let mut d = decode_draft(value.clone())?;
        validate_draft(&mut d, &input.query)?;
        let (key, expected_duration) = match mode {
            "quick" => {
                if d.task_id.is_some() || d.source_id.is_some() || d.kind == "task" {
                    return Err(AppError::validation("Quick add requires a neutral block."));
                }
                (None, if d.kind == "break" { 30 } else { 60 })
            }
            "carry" | "copy" => {
                let source = d
                    .source_id
                    .as_deref()
                    .ok_or_else(|| AppError::validation("Choose a source block."))?;
                let b = array(
                    &data[if mode == "carry" {
                        "previous_blocks"
                    } else {
                        "copy_sources"
                    }],
                )?
                .iter()
                .find(|b| b["id"] == source)
                .ok_or_else(AppError::conflict)?;
                if b["kind"] != d.kind || b["task_id"] != json!(d.task_id) {
                    return Err(AppError::conflict());
                }
                if mode == "carry"
                    && (b["kind"] != "task"
                        || b["done"] != 0
                        || !tasks
                            .iter()
                            .any(|t| t["id"] == b["task_id"] && t["status"] != "done"))
                {
                    return Err(AppError::conflict());
                }
                if mode == "copy" && (b["start_min"] != d.start_min || b["end_min"] != d.end_min) {
                    return Err(AppError::conflict());
                }
                let order = (
                    String::new(),
                    integer(b, "start_min")?,
                    String::new(),
                    source.to_owned(),
                );
                if last_order.as_ref().is_some_and(|old| old > &order) {
                    return Err(AppError::validation(
                        "Keep source blocks in their preview order.",
                    ));
                }
                last_order = Some(order);
                (
                    Some(if mode == "carry" {
                        format!("carry:{source}")
                    } else {
                        format!("copy:{source}:{}", input.query.date)
                    }),
                    integer(b, "end_min")? - integer(b, "start_min")?,
                )
            }
            _ => {
                let task = d
                    .task_id
                    .as_deref()
                    .ok_or_else(|| AppError::validation("Choose a task."))?;
                let t = tasks
                    .iter()
                    .find(|t| t["id"] == task)
                    .ok_or_else(AppError::conflict)?;
                let (date, _) = calendar(&input.query)?;
                let mut future = input.query.clone();
                future.date = (date + Duration::days(7)).to_string();
                let (end, _) = bounds(&future)?;
                if d.kind != "task"
                    || d.source_id.as_deref() != Some(task)
                    || t["status"] == "done"
                    || t["due_at"]
                        .as_str()
                        .is_none_or(|due| due >= utc(end).as_str())
                    || array(&data["blocks"])?.iter().any(|b| b["task_id"] == task)
                {
                    return Err(AppError::conflict());
                }
                let order = (
                    text(t, "due_at")?.to_owned(),
                    match text(t, "priority")? {
                        "high" => 0,
                        "medium" => 1,
                        _ => 2,
                    },
                    text(t, "title")?.to_owned(),
                    task.to_owned(),
                );
                if last_order.as_ref().is_some_and(|old| old > &order) {
                    return Err(AppError::validation(
                        "Keep tasks in due-date preview order.",
                    ));
                }
                last_order = Some(order);
                (
                    Some(format!("due:{task}:{}", input.query.date)),
                    duration(t).ok_or_else(AppError::conflict)?,
                )
            }
        };
        if let Some(key) = &key {
            if claimed(data, key) || !seen.insert(key.clone()) {
                return Err(AppError::conflict());
            }
        }
        if mode != "copy"
            && next_free(&occupied, expected_duration, floor) != Some((d.start_min, d.end_min))
        {
            return Err(AppError::conflict());
        }
        occupied.push((d.start_min, d.end_min));
        let id = add(conn, &input.query, d.clone(), mode == "carry").await?;
        if let Some(key) = key {
            insert(conn,"planner_claims",&json!({"id":key,"mode":mode,"source_id":d.source_id,"task_id":d.task_id,"destination_date":input.query.date,"block_id":id})).await?;
        }
        ids.push(id);
    }
    Ok(ids)
}
pub async fn apply_into(
    pool: &SqlitePool,
    input: PlannerInput,
    now: &str,
    offset: i64,
) -> Result<Value> {
    calendar(&input.query)?;
    nonblank(&input.request_id)?;
    choice(
        &input.action,
        &["create", "move", "done", "remove", "dismiss", "batch"],
    )?;
    let now = instant(now)?;
    let mut payload = json!(input);
    payload.as_object_mut().unwrap().remove("requestId");
    let payload = payload.to_string();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(old) = rows(
        &mut tx,
        "SELECT * FROM planner_requests WHERE request_id=?",
        vec![json!(input.request_id)],
    )
    .await?
    .first()
    {
        if old["payload_json"] != payload {
            return Err(AppError::conflict());
        }
        let outcome = serde_json::from_str(text(old, "outcome_json")?)
            .map_err(|_| AppError::validation("Invalid saved planner receipt."))?;
        let result = reply(&mut tx, &input.query, &now, offset, outcome).await?;
        tx.commit().await?;
        return Ok(result);
    }
    let data = snapshot(&mut tx, &input.query, &now, offset).await?;
    if matches!(input.action.as_str(), "remove" | "batch")
        && input.expected_fingerprint.as_deref() != data["fingerprint"].as_str()
    {
        return Err(AppError::conflict());
    }
    let mut outcome = json!({"block_ids":[]});
    match input.action.as_str() {
        "create" => {
            keys(&input.payload, &["draft", "destinationDate"])?;
            let mut destination = input.query.clone();
            if input.payload.get("destinationDate").is_some() {
                destination.date = text(&input.payload, "destinationDate")?.to_owned();
            }
            let d = decode_draft(input.payload["draft"].clone())?;
            if d.source_id.is_some() {
                return Err(AppError::validation(
                    "Use a planning preview to copy a source.",
                ));
            }
            outcome["block_ids"] = json!([add(&mut tx, &destination, d, false).await?]);
            outcome["destination_date"] = json!(destination.date);
        }
        "batch" => outcome["block_ids"] = json!(batch(&mut tx, &input, &data, &now).await?),
        "dismiss" => {
            keys(&input.payload, &[])?;
            execute(&mut tx,"INSERT INTO settings(key,value) VALUES(?,'true') ON CONFLICT(key) DO UPDATE SET value='true'",vec![json!(format!("planner_dismissed:{}",input.query.date))]).await?;
        }
        action => {
            let id = text(&input.payload, "id")?;
            let b = one(
                &mut tx,
                "SELECT * FROM blocks WHERE id=? AND date=?",
                vec![json!(id), json!(input.query.date)],
            )
            .await?;
            if integer(&input.payload, "expectedRevision")? != integer(&b, "revision")? {
                return Err(AppError::conflict());
            }
            outcome["block_ids"] = json!([id]);
            if !b["task_id"].is_null() {
                outcome["task_id"] = b["task_id"].clone();
            }
            match action {
                "move" => {
                    keys(
                        &input.payload,
                        &[
                            "id",
                            "expectedRevision",
                            "start_min",
                            "end_min",
                            "start_offset",
                            "end_offset",
                            "destinationDate",
                        ],
                    )?;
                    let mut d = decode_draft(
                        json!({"kind":b["kind"],"task_id":b["task_id"],"start_min":input.payload["start_min"],"end_min":input.payload["end_min"],"start_offset":input.payload["start_offset"],"end_offset":input.payload["end_offset"]}),
                    )?;
                    let mut destination = input.query.clone();
                    if input.payload.get("destinationDate").is_some() {
                        destination.date = text(&input.payload, "destinationDate")?.to_owned();
                    }
                    validate_draft(&mut d, &destination)?;
                    execute(&mut tx,"UPDATE blocks SET date=?,start_min=?,end_min=?,time_zone=?,start_offset=?,end_offset=?,revision=revision+1 WHERE id=?",vec![json!(destination.date),json!(d.start_min),json!(d.end_min),json!(destination.time_zone),json!(d.start_offset),json!(d.end_offset),json!(id)]).await?;
                    outcome["destination_date"] = json!(destination.date);
                }
                "done" => {
                    keys(
                        &input.payload,
                        &["id", "expectedRevision", "done", "stopSession"],
                    )?;
                    let done = input.payload["done"]
                        .as_bool()
                        .ok_or_else(|| AppError::validation("Choose a completion state."))?;
                    let live = rows(
                        &mut tx,
                        "SELECT * FROM timer_sessions WHERE block_id=? AND state!='stopped'",
                        vec![json!(id)],
                    )
                    .await?;
                    if done && !live.is_empty() {
                        let stop = &input.payload["stopSession"];
                        keys(stop, &["id", "revision"])?;
                        let s = &live[0];
                        if stop["id"] != s["id"] || stop["revision"] != s["revision"] {
                            return Err(AppError::conflict());
                        }
                        timers::mutate_in_transaction(&mut tx,"stop",json!({"requestId":format!("planner-stop:{}",input.request_id),"taskId":s["task_id"],"sessionId":s["id"],"expectedRevision":s["revision"]}),&now,offset).await?;
                    } else if !input.payload["stopSession"].is_null() {
                        return Err(AppError::conflict());
                    }
                    execute(
                        &mut tx,
                        "UPDATE blocks SET done=?,done_at=?,revision=revision+1 WHERE id=?",
                        vec![
                            json!(if done { 1 } else { 0 }),
                            if done { json!(now) } else { Value::Null },
                            json!(id),
                        ],
                    )
                    .await?;
                }
                "remove" => {
                    keys(&input.payload, &["id", "expectedRevision"])?;
                    execute(&mut tx,"UPDATE timer_sessions SET block_id=NULL,revision=revision+1 WHERE block_id=?",vec![json!(id)]).await?;
                    execute(
                        &mut tx,
                        "UPDATE time_entries SET block_id=NULL WHERE block_id=?",
                        vec![json!(id)],
                    )
                    .await?;
                    execute(&mut tx,"UPDATE blocks SET carried_from_block_id=NULL,revision=revision+1 WHERE carried_from_block_id=?",vec![json!(id)]).await?;
                    execute(&mut tx, "DELETE FROM blocks WHERE id=?", vec![json!(id)]).await?;
                }
                _ => unreachable!(),
            }
        }
    }
    insert(&mut tx,"planner_requests",&json!({"request_id":input.request_id,"payload_json":payload,"outcome_json":outcome.to_string()})).await?;
    let result = reply(&mut tx, &input.query, &now, offset, outcome).await?;
    tx.commit().await?;
    Ok(result)
}
#[tauri::command]
pub async fn get_planner(app: AppHandle, query: PlannerQuery) -> Result<Value> {
    let (now, offset) = clock::sample(&app)?;
    let pool = crate::db::pool(&app)
        .await
        .map_err(|e| AppError::new("Database", e))?;
    let mut tx = pool.begin().await?;
    let data = snapshot(&mut tx, &query, &now, offset).await?;
    tx.commit().await?;
    Ok(data)
}
#[tauri::command]
pub async fn apply_planner(app: AppHandle, input: PlannerInput) -> Result<Value> {
    let (now, offset) = clock::sample(&app)?;
    let pool = crate::db::pool(&app)
        .await
        .map_err(|e| AppError::new("Database", e))?;
    let result = apply_into(&pool, input, &now, offset).await?;
    if let Err(e) = crate::tray::refresh(&app).await {
        eprintln!("Timer tooltip: {}", e.message);
    }
    Ok(result)
}
#[cfg(test)]
mod tests;
