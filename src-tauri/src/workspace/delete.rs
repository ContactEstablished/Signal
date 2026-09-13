use super::models::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};
pub async fn graph(conn: &mut SqliteConnection, target: &DeletionTarget) -> Result<Value> {
    choice(&target.kind, &["task", "project"])?;
    let table = if target.kind == "task" {
        "tasks"
    } else {
        "projects"
    };
    let root = one(
        conn,
        &format!("SELECT * FROM {table} WHERE id=?"),
        vec![json!(target.id)],
    )
    .await?;
    let tasks = if target.kind == "task" {
        vec![root.clone()]
    } else {
        rows(
            conn,
            "SELECT * FROM tasks WHERE project_id=? ORDER BY id",
            vec![json!(target.id)],
        )
        .await?
    };
    let ids: Vec<Value> = tasks.iter().map(|v| v["id"].clone()).collect();
    let mut result = json!({"root":root,"tasks":tasks});
    for name in [
        "timer_sessions",
        "timer_requests",
        "planner_claims",
        "subtasks",
        "task_tags",
        "attachments",
        "blocks",
        "time_entries",
        "alerts",
    ] {
        result[name] = json!(rows(
            conn,
            &format!(
                "SELECT * FROM {name} ORDER BY {}",
                if name == "timer_requests" {
                    "request_id"
                } else if name == "task_tags" {
                    "task_id,tag_id"
                } else {
                    "id"
                }
            ),
            vec![]
        )
        .await?
        .into_iter()
        .filter(|v| ids.contains(&v["task_id"]))
        .collect::<Vec<_>>());
    }
    let meetings = if target.kind == "project" {
        rows(
            conn,
            "SELECT * FROM meetings WHERE project_id=? ORDER BY id",
            vec![json!(target.id)],
        )
        .await?
    } else {
        vec![]
    };
    let meeting_ids: Vec<Value> = meetings.iter().map(|v| v["id"].clone()).collect();
    result["meeting_tasks"] = json!(rows(
        conn,
        "SELECT * FROM meeting_tasks ORDER BY meeting_id,task_id",
        vec![]
    )
    .await?
    .into_iter()
    .filter(|v| ids.contains(&v["task_id"]) || meeting_ids.contains(&v["meeting_id"]))
    .collect::<Vec<_>>());
    result["meetings"] = json!(meetings);
    result["summaries"] = json!(if target.kind == "project" {
        rows(
            conn,
            "SELECT * FROM summaries WHERE project_id=? ORDER BY id",
            vec![json!(target.id)],
        )
        .await?
    } else {
        vec![]
    });
    let block_ids: Vec<Value> = result["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].clone())
        .collect();
    result["detached_blocks"] =
        json!(rows(conn, "SELECT * FROM blocks ORDER BY id", vec![])
            .await?
            .into_iter()
            .filter(|v| !block_ids.contains(&v["id"])
                && block_ids.contains(&v["carried_from_block_id"]))
            .collect::<Vec<_>>());
    result["detached_time_entries"] =
        json!(rows(conn, "SELECT * FROM time_entries ORDER BY id", vec![])
            .await?
            .into_iter()
            .filter(|v| !ids.contains(&v["task_id"]) && block_ids.contains(&v["block_id"]))
            .collect::<Vec<_>>());
    Ok(result)
}
fn fingerprint(graph: &Value) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(graph).expect("JSON graph"))
    )
}
pub async fn preview(pool: &SqlitePool, target: &DeletionTarget) -> Result<Value> {
    let mut tx = pool.begin().await?;
    let data = graph(&mut tx, target).await?;
    let mut counts: serde_json::Map<String, Value> = data
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| v.as_array().map(|a| (k.clone(), json!(a.len()))))
        .collect();
    counts.remove("timer_requests");
    counts.remove("planner_claims");
    let sessions = data["timer_sessions"].as_array().unwrap();
    counts.insert(
        "timer_sessions".into(),
        json!(sessions.iter().filter(|v| v["state"] != "stopped").count()),
    );
    tx.commit().await?;
    Ok(json!({"counts":counts,"fingerprint":fingerprint(&data)}))
}
pub async fn remove(
    pool: &SqlitePool,
    target: &DeletionTarget,
    expected: &str,
    now: &str,
) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let data = graph(&mut tx, target).await?;
    if fingerprint(&data) != expected {
        return Err(AppError::conflict());
    }
    for v in data["attachments"].as_array().unwrap() {
        crate::attachments::queue_copy(&mut tx, v, now).await?;
    }
    for v in data["detached_blocks"].as_array().unwrap() {
        execute(
            &mut tx,
            "UPDATE blocks SET carried_from_block_id=NULL,revision=revision+1 WHERE id=?",
            vec![v["id"].clone()],
        )
        .await?;
    }
    for v in data["detached_time_entries"].as_array().unwrap() {
        execute(
            &mut tx,
            "UPDATE time_entries SET block_id=NULL WHERE id=?",
            vec![v["id"].clone()],
        )
        .await?;
    }
    for v in data["planner_claims"].as_array().unwrap() {
        execute(
            &mut tx,
            "DELETE FROM planner_claims WHERE id=?",
            vec![v["id"].clone()],
        )
        .await?;
    }
    for v in data["timer_requests"].as_array().unwrap() {
        execute(
            &mut tx,
            "DELETE FROM timer_requests WHERE request_id=?",
            vec![v["request_id"].clone()],
        )
        .await?;
    }
    for v in data["timer_sessions"].as_array().unwrap() {
        execute(
            &mut tx,
            "DELETE FROM timer_sessions WHERE id=?",
            vec![v["id"].clone()],
        )
        .await?;
    }
    // Remove time entries while their task still exists for the M0 cache triggers.
    for name in [
        "time_entries",
        "meeting_tasks",
        "task_tags",
        "alerts",
        "subtasks",
        "attachments",
    ] {
        for v in data[name].as_array().unwrap() {
            match name {
                "meeting_tasks" => {
                    execute(
                        &mut tx,
                        "DELETE FROM meeting_tasks WHERE meeting_id=? AND task_id=?",
                        vec![v["meeting_id"].clone(), v["task_id"].clone()],
                    )
                    .await?
                }
                "task_tags" => {
                    execute(
                        &mut tx,
                        "DELETE FROM task_tags WHERE task_id=? AND tag_id=?",
                        vec![v["task_id"].clone(), v["tag_id"].clone()],
                    )
                    .await?
                }
                _ => {
                    execute(
                        &mut tx,
                        &format!("DELETE FROM {name} WHERE id=?"),
                        vec![v["id"].clone()],
                    )
                    .await?
                }
            }
        }
    }
    // Also detach links among the blocks being deleted, regardless of ID order.
    for v in data["blocks"].as_array().unwrap() {
        execute(
            &mut tx,
            "UPDATE blocks SET carried_from_block_id=NULL,revision=revision+1 WHERE id=?",
            vec![v["id"].clone()],
        )
        .await?;
    }
    for name in ["blocks", "tasks", "meetings", "summaries"] {
        for v in data[name].as_array().unwrap() {
            execute(
                &mut tx,
                &format!("DELETE FROM {name} WHERE id=?"),
                vec![v["id"].clone()],
            )
            .await?;
        }
    }
    if target.kind == "project" {
        execute(
            &mut tx,
            "DELETE FROM projects WHERE id=?",
            vec![json!(target.id)],
        )
        .await?;
        let remaining = rows(
            &mut tx,
            "SELECT id FROM projects ORDER BY sort_order,id",
            vec![],
        )
        .await?;
        for (i, v) in remaining.iter().enumerate() {
            execute(
                &mut tx,
                "UPDATE projects SET sort_order=? WHERE id=?",
                vec![json!(i), v["id"].clone()],
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
