use super::models::*;
use serde_json::{json, Value};
use sqlx::SqlitePool;
pub async fn list(pool: &SqlitePool) -> Result<Vec<Value>> {
    rows(
        &mut *pool.acquire().await?,
        "SELECT * FROM projects ORDER BY sort_order,id",
        vec![],
    )
    .await
}
pub async fn save(pool: &SqlitePool, id: Option<String>, input: ProjectInput) -> Result<Value> {
    let name = nonblank(&input.name)?;
    choice(&input.color, COLORS)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = if let Some(id) = id {
        one(
            &mut tx,
            "SELECT id FROM projects WHERE id=?",
            vec![json!(id)],
        )
        .await?;
        execute(
            &mut tx,
            "UPDATE projects SET name=?,color=? WHERE id=?",
            vec![json!(name), json!(input.color), json!(id)],
        )
        .await?;
        id
    } else {
        let id = uid();
        execute(&mut tx,"INSERT INTO projects(id,name,color,sort_order) VALUES (?,?,?,(SELECT COALESCE(MAX(sort_order),-1)+1 FROM projects))",vec![json!(id),json!(name),json!(input.color)]).await?;
        id
    };
    let reply = one(
        &mut tx,
        "SELECT * FROM projects WHERE id=?",
        vec![json!(id)],
    )
    .await?;
    tx.commit().await?;
    Ok(reply)
}
pub async fn reorder(pool: &SqlitePool, id: &str, direction: &str) -> Result<Vec<Value>> {
    choice(direction, &["left", "right"])?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut records = rows(
        &mut tx,
        "SELECT * FROM projects ORDER BY sort_order,id",
        vec![],
    )
    .await?;
    let at = records
        .iter()
        .position(|p| p["id"] == id)
        .ok_or_else(AppError::missing)?;
    let to = if direction == "left" {
        at.saturating_sub(1)
    } else {
        (at + 1).min(records.len() - 1)
    };
    records.swap(at, to);
    for (n, p) in records.iter_mut().enumerate() {
        execute(
            &mut tx,
            "UPDATE projects SET sort_order=? WHERE id=?",
            vec![json!(n), p["id"].clone()],
        )
        .await?;
        p["sort_order"] = json!(n);
    }
    tx.commit().await?;
    Ok(records)
}
