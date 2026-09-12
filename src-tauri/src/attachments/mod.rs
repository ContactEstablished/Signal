use crate::workspace::models::*;
use fs2::FileExt;
use serde_json::{json, Value};
use sqlx::{SqliteConnection, SqlitePool};
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
    sync::Mutex,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
pub struct Files {
    pub root: PathBuf,
    _lock: File,
    pub gate: tokio::sync::Mutex<()>,
    initialized: Mutex<bool>,
}
impl Files {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let root = fs::canonicalize(root)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".workspace.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            AppError::new(
                "File",
                "This workspace is already open in another Signal process.",
            )
        })?;
        Ok(Self {
            root,
            _lock: lock,
            gate: tokio::sync::Mutex::new(()),
            initialized: Mutex::new(false),
        })
    }
    pub fn path(&self, relative: &str) -> Result<PathBuf> {
        if relative.is_empty()
            || relative.contains(['/', '\\', ':'])
            || relative == "."
            || relative == ".."
            || relative.starts_with('.')
        {
            return Err(AppError::new("File", "Invalid managed attachment path"));
        }
        let path = self.root.join(relative);
        if path.exists() {
            let canonical = fs::canonicalize(&path)?;
            if !canonical.starts_with(&self.root)
                || !fs::symlink_metadata(&path)?.file_type().is_file()
            {
                return Err(AppError::new(
                    "File",
                    "Attachment is outside managed storage",
                ));
            }
        }
        Ok(path)
    }
    pub async fn recover(&self, pool: &SqlitePool, now: &str) -> Result<()> {
        let initialized = *self.initialized.lock().unwrap();
        if !initialized {
            execute(&mut *pool.acquire().await?,"UPDATE attachment_file_ops SET state='delete_pending' WHERE state IN ('staging','ready')",vec![]).await?;
            *self.initialized.lock().unwrap() = true;
        }
        self.cleanup(pool, now).await?;
        Ok(())
    }
    pub async fn cleanup(&self, pool: &SqlitePool, _now: &str) -> Result<bool> {
        let mut conn = pool.acquire().await?;
        let pending = rows(
            &mut conn,
            "SELECT * FROM attachment_file_ops WHERE state='delete_pending' ORDER BY token",
            vec![],
        )
        .await?;
        for op in pending {
            let path = text(&op, "relative_path")?;
            if !rows(
                &mut conn,
                "SELECT id FROM attachments WHERE path=?",
                vec![json!(path)],
            )
            .await?
            .is_empty()
            {
                continue;
            }
            let result = self.path(path).and_then(|p| match fs::remove_file(p) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            });
            match result {
                Ok(()) => {
                    execute(
                        &mut conn,
                        "DELETE FROM attachment_file_ops WHERE token=?",
                        vec![op["token"].clone()],
                    )
                    .await?
                }
                Err(e) => {
                    execute(
                        &mut conn,
                        "UPDATE attachment_file_ops SET last_error=? WHERE token=?",
                        vec![json!(e.message), op["token"].clone()],
                    )
                    .await?
                }
            }
        }
        Ok(!rows(
            &mut conn,
            "SELECT token FROM attachment_file_ops WHERE state='delete_pending'",
            vec![],
        )
        .await?
        .is_empty())
    }
    pub async fn stage(&self, pool: &SqlitePool, paths: Vec<PathBuf>, now: &str) -> Result<Value> {
        let mut created = vec![];
        let mut descriptors = vec![];
        for source in paths {
            let result:Result<Value>=async {
    let source=fs::canonicalize(source)?;
    if !source.is_file()||source.starts_with(&self.root){return Err(AppError::new("File","Choose a regular source file outside managed storage"))}
    let filename=source.file_name().and_then(|s|s.to_str()).ok_or_else(||AppError::new("File","Unsupported filename"))?.to_string();
    let token=uid();let extension=source.extension().and_then(|s|s.to_str()).filter(|s|s.len()<=16 && s.chars().all(|c|c.is_ascii_alphanumeric())).unwrap_or("bin");
    let relative=format!("{token}.{extension}");let mime=mime_guess::from_path(&source).first_or_octet_stream().to_string();
    insert(&mut *pool.acquire().await?,"attachment_file_ops",&json!({"token":token,"relative_path":relative,"filename":filename,"size":0,"mime":mime,"state":"staging","created_at":now})).await?;
    created.push(token.clone());
    let destination=self.path(&relative)?;let mut input=File::open(source)?;let mut output=OpenOptions::new().write(true).create_new(true).open(destination)?;
    let size=std::io::copy(&mut input,&mut output)?;output.sync_all()?;
    execute(&mut *pool.acquire().await?,"UPDATE attachment_file_ops SET state='ready',size=? WHERE token=?",vec![json!(size),json!(token)]).await?;
    Ok(json!({"token":token,"filename":filename,"size":size,"mime":mime}))
   }.await;
            match result {
                Ok(v) => descriptors.push(v),
                Err(e) => {
                    self.discard(pool, &created, now).await?;
                    return Err(e);
                }
            }
        }
        Ok(json!(descriptors))
    }
    pub async fn discard(&self, pool: &SqlitePool, tokens: &[String], now: &str) -> Result<bool> {
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        for token in tokens {
            execute(
                &mut tx,
                "UPDATE attachment_file_ops SET state='delete_pending' WHERE token=?",
                vec![json!(token)],
            )
            .await?;
        }
        tx.commit().await?;
        // The discard is durable. Cleanup failures must not turn it into a rejected edit.
        Ok(self.cleanup(pool, now).await.unwrap_or(true))
    }
}
pub async fn adopt(conn: &mut SqliteConnection, task: &str, tokens: &Value) -> Result<()> {
    let tokens = tokens
        .as_array()
        .ok_or_else(|| AppError::validation("Invalid attachment tokens"))?;
    for token in tokens {
        let token = token
            .as_str()
            .ok_or_else(|| AppError::validation("Invalid attachment token"))?;
        let op = one(
            conn,
            "SELECT * FROM attachment_file_ops WHERE token=? AND state='ready'",
            vec![json!(token)],
        )
        .await?;
        insert(conn,"attachments",&json!({"id":uid(),"task_id":task,"filename":op["filename"],"path":op["relative_path"],"size":op["size"],"mime":op["mime"]})).await?;
        execute(
            conn,
            "DELETE FROM attachment_file_ops WHERE token=?",
            vec![json!(token)],
        )
        .await?;
    }
    Ok(())
}
pub async fn queue_copy(conn: &mut SqliteConnection, file: &Value, now: &str) -> Result<()> {
    insert(conn,"attachment_file_ops",&json!({"token":uid(),"relative_path":file["path"],"filename":file["filename"],"size":file["size"],"mime":file["mime"],"state":"delete_pending","created_at":now})).await
}
async fn pool(app: &AppHandle) -> Result<SqlitePool> {
    crate::db::pool(app)
        .await
        .map_err(|e| AppError::new("Database", e))
}
fn clock(app: &AppHandle) -> String {
    now(app.state::<crate::db::RuntimeState>().seeded)
}
#[tauri::command]
pub async fn initialize_workspace(app: AppHandle) -> Result<()> {
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    files.recover(&pool, &clock(&app)).await
}
#[tauri::command]
pub async fn stage_attachments(app: AppHandle, paths: Option<Vec<String>>) -> Result<Value> {
    let paths = if let Some(paths) = paths {
        paths.into_iter().map(PathBuf::from).collect()
    } else {
        app.dialog()
            .file()
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                p.into_path()
                    .map_err(|e| AppError::new("File", e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?
    };
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    files.stage(&pool, paths, &clock(&app)).await
}
#[tauri::command]
pub async fn discard_staged_attachments(app: AppHandle, tokens: Vec<String>) -> Result<Value> {
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    Ok(json!({"cleanupPending":files.discard(&pool,&tokens,&clock(&app)).await?}))
}
#[tauri::command]
pub async fn add_attachments(
    app: AppHandle,
    task_id: String,
    tokens: Value,
    expected_revision: i64,
) -> Result<Value> {
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    check_revision(&mut tx, &task_id, expected_revision).await?;
    adopt(&mut tx, &task_id, &tokens).await?;
    if tokens.as_array().is_some_and(|a| !a.is_empty()) {
        bump(&mut tx, &task_id, &clock(&app)).await?;
    }
    let reply = detail(&mut tx, &task_id).await?;
    tx.commit().await?;
    Ok(reply)
}
#[tauri::command]
pub async fn remove_attachment(
    app: AppHandle,
    id: String,
    expected_revision: i64,
) -> Result<Value> {
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    let now = clock(&app);
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let file = one(
        &mut tx,
        "SELECT * FROM attachments WHERE id=?",
        vec![json!(id)],
    )
    .await?;
    let task = text(&file, "task_id")?;
    check_revision(&mut tx, task, expected_revision).await?;
    queue_copy(&mut tx, &file, &now).await?;
    execute(
        &mut tx,
        "DELETE FROM attachments WHERE id=?",
        vec![json!(id)],
    )
    .await?;
    bump(&mut tx, task, &now).await?;
    let reply = detail(&mut tx, task).await?;
    tx.commit().await?;
    let pending = files.cleanup(&pool, &now).await.unwrap_or(true);
    Ok(json!({"detail":reply,"cleanupPending":pending}))
}
#[tauri::command]
pub async fn open_attachment(app: AppHandle, id: String) -> Result<()> {
    let pool = pool(&app).await?;
    let file = one(
        &mut *pool.acquire().await?,
        "SELECT * FROM attachments WHERE id=?",
        vec![json!(id)],
    )
    .await?;
    let path = app.state::<Files>().path(text(&file, "path")?)?;
    if !path.is_file() {
        return Err(AppError::new("File", "The attachment copy is missing."));
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::new("File", e.to_string()))
}
#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFile {
    id: String,
    filename: String,
    mime: String,
    bytes: Vec<u8>,
}
#[tauri::command]
pub async fn install_fixture_attachments(app: AppHandle, fixtures: Vec<FixtureFile>) -> Result<()> {
    if !cfg!(debug_assertions) || !app.state::<crate::db::RuntimeState>().seeded {
        return Err(AppError::validation(
            "Fixture attachments require a debug --seed launch",
        ));
    }
    let expected = [
        (
            "f870c681-27a4-4d65-87be-000000000d01",
            "gateway-arch.pdf",
            "application/pdf",
        ),
        (
            "f870c681-27a4-4d65-87be-000000000d02",
            "latency-p95.png",
            "image/png",
        ),
    ];
    if fixtures.len() != 2 {
        return Err(AppError::validation(
            "Expected the two approved fixture files",
        ));
    }
    for (file, (id, name, mime)) in fixtures.iter().zip(expected) {
        if file.id != id
            || file.filename != name
            || file.mime != mime
            || file.bytes.is_empty()
            || file.bytes.len() > 1_000_000
        {
            return Err(AppError::validation("Invalid fixture attachment"));
        }
    }
    let pool = pool(&app).await?;
    let files = app.state::<Files>();
    let _gate = files.gate.lock().await;
    let now = clock(&app);
    let mut conn = pool.acquire().await?;
    if !rows(
        &mut conn,
        "SELECT key FROM settings WHERE key='fixture_attachments_m1'",
        vec![],
    )
    .await?
    .is_empty()
    {
        return Ok(());
    }
    let marker = one(
        &mut conn,
        "SELECT value FROM settings WHERE key='fixture_version'",
        vec![],
    )
    .await?;
    if marker["value"] != "september-2025-midday-v1" {
        return Err(AppError::validation("M0 fixture marker is missing"));
    }
    let task = "f870c681-27a4-4d65-87be-00000000006b";
    one(
        &mut conn,
        "SELECT id FROM tasks WHERE id=? AND external_id='ATL-482'",
        vec![json!(task)],
    )
    .await?;
    drop(conn);
    let mut tokens = vec![];
    let outcome:Result<()>=async {
  for file in &fixtures{
   let token=uid();let relative=format!("{token}.{}",if file.mime=="image/png"{"png"}else{"pdf"});
   insert(&mut *pool.acquire().await?,"attachment_file_ops",&json!({"token":token,"relative_path":relative,"filename":file.filename,"size":file.bytes.len(),"mime":file.mime,"state":"staging","created_at":now})).await?;tokens.push(token.clone());
   let mut out=OpenOptions::new().create_new(true).write(true).open(files.path(&relative)?)?;std::io::Write::write_all(&mut out,&file.bytes)?;out.sync_all()?;
   execute(&mut *pool.acquire().await?,"UPDATE attachment_file_ops SET state='ready' WHERE token=?",vec![json!(token)]).await?;
  }
  let mut tx=pool.begin_with("BEGIN IMMEDIATE").await?;
  for (file,token) in fixtures.iter().zip(&tokens){let op=one(&mut tx,"SELECT * FROM attachment_file_ops WHERE token=? AND state='ready'",vec![json!(token)]).await?;insert(&mut tx,"attachments",&json!({"id":file.id,"task_id":task,"filename":file.filename,"path":op["relative_path"],"size":file.bytes.len(),"mime":file.mime})).await?;execute(&mut tx,"DELETE FROM attachment_file_ops WHERE token=?",vec![json!(token)]).await?;}
  execute(&mut tx,"INSERT INTO settings(key,value) VALUES ('fixture_attachments_m1','synthetic-v1')",vec![]).await?;tx.commit().await?;Ok(())
 }.await;
    if outcome.is_err() {
        let _ = files.discard(&pool, &tokens, &now).await;
    }
    outcome
}
