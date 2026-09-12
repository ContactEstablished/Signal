use super::models::*;
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashSet;
const FIELDS: &[&str] = &[
    "title",
    "status",
    "priority",
    "due_at",
    "estimate_h",
    "external_url",
    "external_provider",
    "external_id",
    "blocked_reason",
    "blocked_on",
    "notes_md",
];
pub fn normalize_patch(patch: &Value) -> Result<Value> {
    let mut fields = patch
        .as_object()
        .ok_or_else(|| AppError::validation("Expected task fields"))?
        .clone();
    for (key, value) in fields.iter_mut() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(AppError::validation(format!("{key} is not editable")));
        }
        match key.as_str() {
            "title" => {
                *value = json!(nonblank(
                    value
                        .as_str()
                        .ok_or_else(|| AppError::validation("Title is required"))?
                )?)
            }
            "status" => choice(value.as_str().unwrap_or(""), STATUSES)?,
            "priority" => choice(value.as_str().unwrap_or(""), &["low", "medium", "high"])?,
            "due_at" => {
                if !value.is_null() {
                    *value = json!(instant(
                        value
                            .as_str()
                            .ok_or_else(|| AppError::validation("Invalid deadline"))?
                    )?)
                }
            }
            "estimate_h" => {
                if !value.is_null() && !value.as_f64().is_some_and(|v| v.is_finite() && v >= 0.0) {
                    return Err(AppError::validation(
                        "Estimate must be a nonnegative number",
                    ));
                }
            }
            "notes_md" => {
                if !value.is_string() {
                    return Err(AppError::validation("Notes must be text"));
                }
            }
            _ => {
                if !value.is_null() {
                    let s = value
                        .as_str()
                        .ok_or_else(|| AppError::validation("Expected text"))?
                        .trim();
                    *value = if s.is_empty() { Value::Null } else { json!(s) };
                }
            }
        }
    }
    let external = ["external_url", "external_provider", "external_id"];
    if external.iter().any(|k| fields.contains_key(*k)) {
        if !external.iter().all(|k| fields.contains_key(*k)) {
            return Err(AppError::validation(
                "Update the link and provider fields together",
            ));
        }
        if let Some(url) = fields["external_url"].as_str() {
            crate::links::validate(url)?;
        }
        if let Some(provider) = fields["external_provider"].as_str() {
            choice(provider, &["jira", "asana", "clickup"])?;
            if fields["external_id"].as_str().is_none() {
                return Err(AppError::validation("Provider ID is required"));
            }
        } else if !fields["external_id"].is_null() {
            return Err(AppError::validation(
                "Provider is required for an external ID",
            ));
        }
    }
    Ok(Value::Object(fields))
}
pub async fn board(pool: &SqlitePool, project: &str, start: &str, end: &str) -> Result<Value> {
    let start = instant(start)?;
    let end = instant(end)?;
    if end <= start {
        return Err(AppError::validation("Invalid day interval"));
    }
    let mut tx = pool.begin().await?;
    let project = one(
        &mut tx,
        "SELECT * FROM projects WHERE id=?",
        vec![json!(project)],
    )
    .await?;
    let mut tasks=rows(&mut tx,"SELECT tasks.*, (SELECT COUNT(*) FROM subtasks WHERE task_id=tasks.id) subtask_total,(SELECT COUNT(*) FROM subtasks WHERE task_id=tasks.id AND done=1) subtask_done FROM tasks WHERE project_id=? ORDER BY sort_order,id",vec![project["id"].clone()]).await?;
    for task in &mut tasks {
        task["tags"]=json!(rows(&mut tx,"SELECT tags.* FROM tags JOIN task_tags ON tag_id=tags.id WHERE task_id=? ORDER BY name,tags.id",vec![task["id"].clone()]).await?);
    }
    let meetings=rows(&mut tx,"SELECT * FROM meetings WHERE project_id=? AND julianday(starts_at)<julianday(?) AND julianday(starts_at)+duration_min/1440.0>julianday(?) ORDER BY starts_at,id",vec![project["id"].clone(),json!(end),json!(start)]).await?;
    let tags = rows(&mut tx, "SELECT * FROM tags ORDER BY name,id", vec![]).await?;
    tx.commit().await?;
    Ok(json!({"project":project,"tasks":tasks,"tags":tags,"meetings":meetings}))
}
pub async fn get(pool: &SqlitePool, id: &str) -> Result<Value> {
    let mut tx = pool.begin().await?;
    let value = detail(&mut tx, id).await?;
    tx.commit().await?;
    Ok(value)
}
pub async fn normalize_order(
    conn: &mut SqliteConnection,
    project: &Value,
    status: &str,
    ids: Vec<Value>,
    now: &str,
) -> Result<()> {
    for (index, id) in ids.iter().enumerate() {
        execute(conn,"UPDATE tasks SET sort_order=?,revision=revision+1,updated_at=? WHERE id=? AND project_id=? AND status=? AND sort_order!=?",vec![json!(index),json!(now),id.clone(),project.clone(),json!(status),json!(index)]).await?;
    }
    Ok(())
}
async fn move_in(
    conn: &mut SqliteConnection,
    task: &Value,
    status: &str,
    before: Option<&str>,
    now: &str,
) -> Result<()> {
    choice(status, STATUSES)?;
    let id = text(task, "id")?;
    let old = text(task, "status")?;
    let project = &task["project_id"];
    if before == Some(id) {
        return Ok(());
    }
    let mut dest = rows(
        conn,
        "SELECT id FROM tasks WHERE project_id=? AND status=? AND id!=? ORDER BY sort_order,id",
        vec![project.clone(), json!(status), json!(id)],
    )
    .await?
    .into_iter()
    .map(|r| r["id"].clone())
    .collect::<Vec<_>>();
    let position = if let Some(target) = before {
        dest.iter()
            .position(|v| v == target)
            .ok_or_else(|| AppError::validation("Drop target is no longer in this column"))?
    } else {
        dest.len()
    };
    dest.insert(position, json!(id));
    if status != old {
        execute(
            conn,
            "UPDATE tasks SET status=?,done_at=?,blocked_since=? WHERE id=?",
            vec![
                json!(status),
                if status == "done" {
                    json!(now)
                } else {
                    Value::Null
                },
                if status == "blocked" {
                    json!(now)
                } else {
                    Value::Null
                },
                json!(id),
            ],
        )
        .await?;
        // The moved task is bumped by its caller exactly once, even if its ordinal changes.
        execute(
            conn,
            "UPDATE tasks SET sort_order=? WHERE id=?",
            vec![json!(position), json!(id)],
        )
        .await?;
        let source = rows(
            conn,
            "SELECT id FROM tasks WHERE project_id=? AND status=? ORDER BY sort_order,id",
            vec![project.clone(), json!(old)],
        )
        .await?
        .into_iter()
        .map(|r| r["id"].clone())
        .collect();
        normalize_order(conn, project, old, source, now).await?;
    } else {
        execute(
            conn,
            "UPDATE tasks SET sort_order=? WHERE id=?",
            vec![json!(position), json!(id)],
        )
        .await?;
    }
    normalize_order(conn, project, status, dest, now).await
}
pub async fn move_task(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    before: Option<&str>,
    revision: i64,
    now: &str,
) -> Result<Value> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = check_revision(&mut tx, id, revision).await?;
    move_in(&mut tx, &old, status, before, now).await?;
    let current = one(&mut tx, "SELECT * FROM tasks WHERE id=?", vec![json!(id)]).await?;
    if old["status"] != current["status"] || old["sort_order"] != current["sort_order"] {
        bump(&mut tx, id, now).await?;
    }
    let reply = detail(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply)
}
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    patch: Value,
    revision: i64,
    now: &str,
) -> Result<Value> {
    let patch = normalize_patch(&patch)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = check_revision(&mut tx, id, revision).await?;
    let mut changed = false;
    for (field, value) in patch.as_object().unwrap() {
        if old[field] != *value {
            changed = true;
            if field == "status" {
                move_in(&mut tx, &old, value.as_str().unwrap(), None, now).await?;
            } else {
                execute(
                    &mut tx,
                    &format!("UPDATE tasks SET {field}=? WHERE id=?"),
                    vec![value.clone(), json!(id)],
                )
                .await?;
            }
        }
    }
    if changed {
        bump(&mut tx, id, now).await?;
    }
    let reply = detail(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply)
}
pub async fn set_children(
    conn: &mut SqliteConnection,
    id: &str,
    kind: &str,
    inputs: &Value,
) -> Result<()> {
    let values = inputs
        .as_array()
        .ok_or_else(|| AppError::validation("Expected a list"))?;
    match kind {
        "subtasks" => {
            let existing = rows(
                conn,
                "SELECT id FROM subtasks WHERE task_id=?",
                vec![json!(id)],
            )
            .await?;
            let mut seen = HashSet::new();
            let mut normalized = vec![];
            for (order, v) in values.iter().enumerate() {
                let map = v
                    .as_object()
                    .ok_or_else(|| AppError::validation("Invalid subtask"))?;
                if map
                    .keys()
                    .any(|k| !["id", "title", "done"].contains(&k.as_str()))
                {
                    return Err(AppError::validation("Unknown subtask field"));
                }
                let subid = if let Some(s) = v.get("id") {
                    let s = s
                        .as_str()
                        .ok_or_else(|| AppError::validation("Invalid subtask ID"))?;
                    if !existing.iter().any(|r| r["id"] == s) {
                        return Err(AppError::validation("Subtask does not belong to this task"));
                    }
                    s.to_string()
                } else {
                    uid()
                };
                if !seen.insert(subid.clone()) {
                    return Err(AppError::validation("Duplicate subtask"));
                }
                normalized.push(json!({"id":subid,"task_id":id,"title":nonblank(text(v,"title")?)?,"done":v["done"].as_bool().ok_or_else(||AppError::validation("Invalid subtask state"))?,"sort_order":order}));
            }
            execute(
                conn,
                "DELETE FROM subtasks WHERE task_id=?",
                vec![json!(id)],
            )
            .await?;
            for v in normalized {
                insert(conn, "subtasks", &v).await?;
            }
        }
        "tags" => {
            let mut ids = HashSet::new();
            for v in values {
                let tag = if v.get("id").is_some() {
                    if v.as_object().map_or(true, |m| m.len() != 1) {
                        return Err(AppError::validation("Invalid tag reference"));
                    }
                    one(
                        conn,
                        "SELECT * FROM tags WHERE id=?",
                        vec![json!(text(v, "id")?)],
                    )
                    .await?
                } else {
                    let name = nonblank(text(v, "name")?)?;
                    let color = text(v, "color")?;
                    choice(color, &["cyan", "lime", "magenta", "violet", "orange"])?;
                    if v.as_object().map_or(true, |m| m.len() != 2) {
                        return Err(AppError::validation("Unknown tag field"));
                    }
                    let all = rows(conn, "SELECT * FROM tags ORDER BY id", vec![]).await?;
                    if let Some(tag) = all.into_iter().find(|t| {
                        t["name"].as_str().unwrap_or("").trim().to_lowercase()
                            == name.to_lowercase()
                    }) {
                        tag
                    } else {
                        let tag = json!({"id":uid(),"name":name,"color":color});
                        insert(conn, "tags", &tag).await?;
                        tag
                    }
                };
                ids.insert(text(&tag, "id")?.to_string());
            }
            execute(
                conn,
                "DELETE FROM task_tags WHERE task_id=?",
                vec![json!(id)],
            )
            .await?;
            for tag in ids {
                execute(
                    conn,
                    "INSERT INTO task_tags(task_id,tag_id) VALUES (?,?)",
                    vec![json!(id), json!(tag)],
                )
                .await?;
            }
        }
        "alerts" => {
            let mut offsets = HashSet::new();
            for v in values {
                let n = v.as_i64().filter(|n| *n >= 0).ok_or_else(|| {
                    AppError::validation("Alert offsets must be whole nonnegative minutes")
                })?;
                offsets.insert(n);
            }
            let current = rows(
                conn,
                "SELECT * FROM alerts WHERE task_id=?",
                vec![json!(id)],
            )
            .await?;
            for row in &current {
                if !offsets.contains(&row["offset_min"].as_i64().unwrap()) {
                    execute(
                        conn,
                        "DELETE FROM alerts WHERE id=?",
                        vec![row["id"].clone()],
                    )
                    .await?;
                }
            }
            for n in offsets {
                if !current.iter().any(|v| v["offset_min"] == n) {
                    execute(
                        conn,
                        "INSERT INTO alerts(id,task_id,offset_min) VALUES (?,?,?)",
                        vec![json!(uid()), json!(id), json!(n)],
                    )
                    .await?;
                }
            }
        }
        _ => return Err(AppError::validation("Unknown collection")),
    };
    Ok(())
}
pub async fn children(
    pool: &SqlitePool,
    id: &str,
    kind: &str,
    values: Value,
    revision: i64,
    now: &str,
) -> Result<Value> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    check_revision(&mut tx, id, revision).await?;
    let old = detail(&mut tx, id).await?;
    set_children(&mut tx, id, kind, &values).await?;
    let current = detail(&mut tx, id).await?;
    if old[kind] != current[kind] {
        bump(&mut tx, id, now).await?;
    }
    let reply = detail(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply)
}
pub async fn create(pool: &SqlitePool, input: Value, now: &str) -> Result<Value> {
    let mut fields = input
        .as_object()
        .ok_or_else(|| AppError::validation("Expected task"))?
        .clone();
    let project = fields
        .remove("project_id")
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or_else(|| AppError::validation("Choose a project"))?;
    let mut collections = serde_json::Map::new();
    for key in ["subtasks", "tags", "alerts", "attachments"] {
        collections.insert(key.into(), fields.remove(key).unwrap_or(json!([])));
    }
    let mut values = normalize_patch(&Value::Object(fields))?;
    for required in ["title", "status", "priority"] {
        text(&values, required)?;
    }
    let id = uid();
    let status = text(&values, "status")?.to_string();
    values["id"] = json!(id);
    values["project_id"] = json!(project);
    values["created_at"] = json!(now);
    values["updated_at"] = json!(now);
    values["done_at"] = if status == "done" {
        json!(now)
    } else {
        Value::Null
    };
    values["blocked_since"] = if status == "blocked" {
        json!(now)
    } else {
        Value::Null
    };
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    one(
        &mut tx,
        "SELECT id FROM projects WHERE id=?",
        vec![json!(project)],
    )
    .await?;
    values["sort_order"] = one(
        &mut tx,
        "SELECT COALESCE(MAX(sort_order),-1)+1 AS n FROM tasks WHERE project_id=? AND status=?",
        vec![json!(project), json!(status)],
    )
    .await?["n"]
        .clone();
    insert(&mut tx, "tasks", &values).await?;
    for kind in ["subtasks", "tags", "alerts"] {
        set_children(&mut tx, &id, kind, &collections[kind]).await?;
    }
    crate::attachments::adopt(&mut tx, &id, &collections["attachments"]).await?;
    let reply = detail(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(reply)
}
