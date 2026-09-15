use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::{Column, Row, SqliteConnection, TypeInfo, ValueRef};
pub type Result<T> = std::result::Result<T, AppError>;
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}
impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("Validation", message)
    }
    pub fn missing() -> Self {
        Self::new("NotFound", "This record no longer exists.")
    }
    pub fn conflict() -> Self {
        Self::new(
            "Conflict",
            "This record changed. Reload and review your edit before retrying.",
        )
    }
}
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::new("Database", e.to_string())
    }
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new("File", e.to_string())
    }
}
pub fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[cfg(test)]
pub fn now(seeded: bool) -> String {
    if seeded {
        "2025-09-11T17:42:00.000Z".into()
    } else {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}
pub fn instant(v: &str) -> Result<String> {
    chrono::DateTime::parse_from_rfc3339(v)
        .map(|d| {
            d.with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        })
        .map_err(|_| AppError::validation("Enter a valid date and time."))
}
pub fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation(format!("{key} must be text.")))
}
pub fn nonblank(v: &str) -> Result<String> {
    let s = v.trim();
    if s.is_empty() {
        Err(AppError::validation("A name or title is required."))
    } else {
        Ok(s.into())
    }
}
pub fn choice(v: &str, choices: &[&str]) -> Result<()> {
    if choices.contains(&v) {
        Ok(())
    } else {
        Err(AppError::validation(format!("Invalid value: {v}")))
    }
}
pub const STATUSES: &[&str] = &["backlog", "todo", "in_progress", "blocked", "done"];
pub const COLORS: &[&str] = &["cyan", "lime", "magenta", "violet"];
pub fn row_json(row: sqlx::sqlite::SqliteRow) -> Result<Value> {
    let mut out = Map::new();
    for c in row.columns() {
        let raw = row.try_get_raw(c.ordinal())?;
        let v = if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" | "BOOLEAN" => json!(row.try_get::<i64, _>(c.ordinal())?),
                "REAL" => json!(row.try_get::<f64, _>(c.ordinal())?),
                _ => json!(row.try_get::<String, _>(c.ordinal())?),
            }
        };
        out.insert(c.name().into(), v);
    }
    Ok(Value::Object(out))
}
pub async fn rows(conn: &mut SqliteConnection, sql: &str, args: Vec<Value>) -> Result<Vec<Value>> {
    let mut q = sqlx::query(sql);
    for a in args {
        q = match a {
            Value::Null => q.bind(Option::<String>::None),
            Value::String(s) => q.bind(s),
            Value::Bool(b) => q.bind(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    q.bind(i)
                } else {
                    q.bind(
                        n.as_f64()
                            .ok_or_else(|| AppError::validation("Invalid number"))?,
                    )
                }
            }
            _ => return Err(AppError::validation("Invalid SQL parameter")),
        };
    }
    q.fetch_all(conn).await?.into_iter().map(row_json).collect()
}
pub async fn one(conn: &mut SqliteConnection, sql: &str, args: Vec<Value>) -> Result<Value> {
    rows(conn, sql, args)
        .await?
        .into_iter()
        .next()
        .ok_or_else(AppError::missing)
}
pub async fn execute(conn: &mut SqliteConnection, sql: &str, args: Vec<Value>) -> Result<()> {
    rows(conn, sql, args).await?;
    Ok(())
}
pub async fn insert(conn: &mut SqliteConnection, table: &str, v: &Value) -> Result<()> {
    let map = v
        .as_object()
        .ok_or_else(|| AppError::validation("Expected record"))?;
    // Table/column names here originate exclusively in the native whitelist/builders.
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        map.keys().cloned().collect::<Vec<_>>().join(","),
        vec!["?"; map.len()].join(",")
    );
    execute(conn, &sql, map.values().cloned().collect()).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    pub name: String,
    pub color: String,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeletionTarget {
    pub kind: String,
    pub id: String,
}
pub async fn detail(conn: &mut SqliteConnection, id: &str) -> Result<Value> {
    let task = one(conn, "SELECT * FROM tasks WHERE id=?", vec![json!(id)]).await?;
    let project = one(
        conn,
        "SELECT * FROM projects WHERE id=?",
        vec![task["project_id"].clone()],
    )
    .await?;
    let subtasks = rows(
        conn,
        "SELECT * FROM subtasks WHERE task_id=? ORDER BY sort_order,id",
        vec![json!(id)],
    )
    .await?;
    let tags=rows(conn,"SELECT tags.* FROM tags JOIN task_tags ON tags.id=task_tags.tag_id WHERE task_id=? ORDER BY name,tags.id",vec![json!(id)]).await?;
    let attachments = rows(
        conn,
        "SELECT * FROM attachments WHERE task_id=? ORDER BY filename,id",
        vec![json!(id)],
    )
    .await?;
    let alerts = rows(
        conn,
        "SELECT * FROM alerts WHERE task_id=? ORDER BY offset_min DESC",
        vec![json!(id)],
    )
    .await?;
    let meetings: Vec<Value> = vec![]; // Calendar-linked meetings use the bounded agenda query.
    Ok(
        json!({"task":task,"project":project,"subtasks":subtasks,"tags":tags,"attachments":attachments,"alerts":alerts,"meetings":meetings}),
    )
}
pub async fn check_revision(conn: &mut SqliteConnection, id: &str, revision: i64) -> Result<Value> {
    let task = one(conn, "SELECT * FROM tasks WHERE id=?", vec![json!(id)]).await?;
    if task["revision"].as_i64() != Some(revision) {
        return Err(AppError::conflict());
    }
    Ok(task)
}
pub async fn bump(conn: &mut SqliteConnection, id: &str, now: &str) -> Result<()> {
    execute(
        conn,
        "UPDATE tasks SET revision=revision+1,updated_at=? WHERE id=?",
        vec![json!(now), json!(id)],
    )
    .await
}
