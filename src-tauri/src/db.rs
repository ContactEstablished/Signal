use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_sql::{DbInstances, DbPool, Migration, MigrationKind};

pub const USER_DB: &str = "sqlite:signal.db";
pub const SEED_DB: &str = "sqlite:signal-dev-september-2025.db";
pub struct RuntimeState {
    pub seeded: bool,
    pub close_to_tray: AtomicBool,
}
impl RuntimeState {
    pub fn database(&self) -> &'static str {
        if self.seeded {
            SEED_DB
        } else {
            USER_DB
        }
    }
}
pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "initial_handoff_schema",
            sql: include_str!("../migrations/0001_initial.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "board_fields_and_files",
            sql: include_str!("../migrations/0002_board_fields.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "durable_timers",
            sql: include_str!("../migrations/0003_timers.sql"),
            kind: MigrationKind::Up,
        },
    ]
}
#[derive(Serialize)]
pub struct RuntimeConfig {
    database: &'static str,
    seeded: bool,
}
#[tauri::command]
pub fn runtime_config(state: State<'_, RuntimeState>) -> RuntimeConfig {
    RuntimeConfig {
        database: state.database(),
        seeded: state.seeded,
    }
}
pub(crate) async fn pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let instances = app.state::<DbInstances>();
    let databases = instances.0.read().await;
    let state = app.state::<RuntimeState>();
    match databases.get(state.database()) {
        Some(DbPool::Sqlite(pool)) => Ok(pool.clone()),
        _ => Err("The local database is not open yet.".into()),
    }
}
#[tauri::command]
pub async fn load_tray_preference(app: AppHandle) -> Result<bool, String> {
    let pool = pool(&app).await?;
    let value: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'close_to_tray'")
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string())?;
    let enabled = value == "true";
    app.state::<RuntimeState>()
        .close_to_tray
        .store(enabled, Ordering::SeqCst);
    Ok(enabled)
}
#[tauri::command]
pub async fn set_tray_preference(app: AppHandle, enabled: bool) -> Result<(), String> {
    // A failed tray setup prevents startup; this check also prevents hiding without recovery.
    if enabled && app.tray_by_id("signal-tray").is_none() {
        return Err("The system tray is unavailable.".into());
    }
    let pool = pool(&app).await?;
    sqlx::query("INSERT INTO settings(key,value) VALUES ('close_to_tray',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(if enabled { "true" } else { "false" }).execute(&pool).await.map_err(|e| e.to_string())?;
    app.state::<RuntimeState>()
        .close_to_tray
        .store(enabled, Ordering::SeqCst);
    Ok(())
}
#[derive(Deserialize)]
pub struct SeedTable {
    table: String,
    rows: Vec<BTreeMap<String, Value>>,
}
#[tauri::command]
pub async fn seed_database(
    app: AppHandle,
    version: String,
    tables: Vec<SeedTable>,
) -> Result<(), String> {
    if !cfg!(debug_assertions) || !app.state::<RuntimeState>().seeded {
        return Err("Fixture loading requires a debug build launched with --seed.".into());
    }
    if version != "september-2025-midday-v1" {
        return Err("Unknown fixture version.".into());
    }
    let pool = pool(&app).await?;
    seed_into(&pool, &version, tables).await
}
pub(crate) async fn seed_into(
    pool: &SqlitePool,
    version: &str,
    tables: Vec<SeedTable>,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // Acquire the write lock before checking the marker: concurrent loads cannot double seed.
    let inserted = sqlx::query(
        "INSERT INTO settings(key,value) VALUES ('fixture_version',?) ON CONFLICT(key) DO NOTHING",
    )
    .bind(&version)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?
    .rows_affected();
    if inserted == 0 {
        return Ok(());
    }
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if existing != 0 {
        return Err(
            "Fixture database is not empty and has no seed marker; refusing to overwrite it."
                .into(),
        );
    }
    let allowed = [
        "projects",
        "tasks",
        "subtasks",
        "tags",
        "task_tags",
        "meetings",
        "meeting_tasks",
        "blocks",
        "time_entries",
        "alerts",
    ];
    for table in tables {
        if !allowed.contains(&table.table.as_str()) {
            return Err("Unknown fixture table.".into());
        }
        // Columns are validated against the migration, never trusted SQL from the webview.
        let schema = sqlx::query(&format!("PRAGMA table_info(\"{}\")", table.table))
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        let columns: Vec<String> = schema.iter().map(|r| r.get("name")).collect();
        for row in table.rows {
            if row.is_empty() || row.keys().any(|k| !columns.contains(k)) {
                return Err("Unknown fixture column.".into());
            }
            let names = row
                .keys()
                .map(|k| format!("\"{k}\""))
                .collect::<Vec<_>>()
                .join(",");
            let marks = vec!["?"; row.len()].join(",");
            let statement = format!("INSERT INTO \"{}\" ({names}) VALUES ({marks})", table.table);
            let mut query = sqlx::query(&statement);
            for value in row.values() {
                query = match value {
                    Value::Null => query.bind(Option::<String>::None),
                    Value::String(v) => query.bind(v),
                    Value::Number(v) if v.is_i64() => query.bind(v.as_i64().unwrap()),
                    Value::Number(v) => query.bind(v.as_f64().ok_or("Invalid fixture number")?),
                    _ => return Err("Invalid fixture value.".into()),
                };
            }
            query.execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
    }
    sqlx::query("UPDATE tasks SET sort_order=(SELECT COUNT(*) FROM tasks p WHERE p.project_id=tasks.project_id AND p.status=tasks.status AND (p.created_at<tasks.created_at OR (p.created_at=tasks.created_at AND p.id<tasks.id)))")
        .execute(&mut *tx).await.map_err(|e|e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())
}
