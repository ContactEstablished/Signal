//! Google Calendar → Signal meetings. Read-only; see
//! docs/Features/Signal/Mail-Calendar-Integration-Plan.md.
pub mod google;
pub mod sync;
#[cfg(test)]
mod tests;

use crate::workspace::models::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// How far around today Signal follows the calendar.
pub const PAST_DAYS: i64 = 7;
pub const FUTURE_DAYS: i64 = 60;
const EVERY: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub color: String,
    pub primary: bool,
    /// Imported into this project; None means not followed.
    pub project_id: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub calendars: Vec<Calendar>,
    /// Google granted read-only mail access at the last sign-in.
    pub mail: bool,
    /// The person turned on task suggestions from this inbox, which sends
    /// new email to the configured AI provider.
    pub mail_tasks: bool,
    /// Sure tasks from this inbox are added without review.
    pub auto_add: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub client_id: String,
    pub accounts: Vec<Account>,
    /// Accounts read through classic Outlook on this computer.
    pub outlook: Vec<crate::outlook::Account>,
}

pub async fn load(conn: &mut SqliteConnection) -> Result<Config> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key='calendar'")
        .fetch_optional(&mut *conn)
        .await?;
    raw.map(|v| {
        serde_json::from_str(&v).map_err(|_| AppError::validation("Calendar settings are invalid."))
    })
    .transpose()
    .map(Option::unwrap_or_default)
}
pub async fn save(conn: &mut SqliteConnection, c: &Config) -> Result<()> {
    sqlx::query("INSERT INTO settings(key,value) VALUES('calendar',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(serde_json::to_string(c).unwrap())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Keeps followed calendars and their projects, adds new ones unfollowed.
pub fn merge_calendars(old: &[Calendar], fresh: Vec<google::CalendarInfo>) -> Vec<Calendar> {
    fresh
        .into_iter()
        .map(|c| Calendar {
            project_id: old
                .iter()
                .find(|o| o.id == c.id)
                .and_then(|o| o.project_id.clone()),
            id: c.id,
            name: c.name,
            color: c.color,
            primary: c.primary,
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Secrets: protected for this Windows user, in this installation's own folder.

fn folder(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::new("File", "Application storage is unavailable."))?
        .join(if app.state::<crate::db::RuntimeState>().seeded {
            "calendar-dev"
        } else {
            "calendar"
        });
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
pub(crate) fn safe_name(id: &str) -> Result<String> {
    if id.len() > 64 || id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(AppError::validation("Invalid account."));
    }
    Ok(format!("account-{id}.bin"))
}
fn write_secret(app: &AppHandle, name: &str, value: &str) -> Result<()> {
    std::fs::write(
        folder(app)?.join(name),
        crate::intake::settings::protect(value.as_bytes(), false)?,
    )?;
    Ok(())
}
pub(crate) fn read_secret(app: &AppHandle, name: &str) -> Result<Option<String>> {
    let path = folder(app)?.join(name);
    if !path.is_file() {
        return Ok(None);
    }
    String::from_utf8(crate::intake::settings::protect(
        &std::fs::read(path)?,
        true,
    )?)
    .map(Some)
    .map_err(|_| AppError::validation("A saved Google credential is unreadable. Connect again."))
}
fn delete_secret(app: &AppHandle, name: &str) -> Result<()> {
    let path = folder(app)?.join(name);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
pub(crate) fn client(app: &AppHandle, cfg: &Config) -> Result<google::Client> {
    let secret = read_secret(app, "client.bin")?
        .filter(|_| !cfg.client_id.is_empty())
        .ok_or_else(|| {
            AppError::validation("Load the Google client file in Settings → Calendar & email first.")
        })?;
    Ok(google::Client {
        id: cfg.client_id.clone(),
        secret,
    })
}

// ---------------------------------------------------------------------------------------------
// Runtime state.

#[derive(Clone, Debug, Default, Serialize)]
pub struct AccountStatus {
    pub last_sync: Option<String>,
    pub last_error: Option<String>,
    pub needs_sign_in: bool,
}
#[derive(Default)]
pub struct Calendars {
    time_zone: Mutex<Option<String>>,
    status: Mutex<HashMap<String, AccountStatus>>,
    worker: tokio::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// One sync at a time, from the timer or "Sync now".
    gate: tokio::sync::Mutex<()>,
    wake: tokio::sync::Notify,
    cancel_connect: tokio::sync::Notify,
    connecting: std::sync::atomic::AtomicBool,
}

/// The display time zone, once the workspace is open.
pub fn time_zone(app: &AppHandle) -> Option<String> {
    app.state::<Calendars>().time_zone.lock().unwrap().clone()
}
async fn pool(app: &AppHandle) -> Result<sqlx::SqlitePool> {
    crate::workspace::pool(app).await
}
fn now_for_records(app: &AppHandle) -> String {
    crate::clock::sample(app)
        .map(|v| v.0)
        .unwrap_or_else(|_| crate::agenda::models::utc(chrono::Utc::now()))
}

#[derive(Serialize)]
pub struct SyncSummary {
    pub created: usize,
    pub updated: usize,
    pub removed: usize,
    pub canceled: usize,
    pub errors: Vec<String>,
}

async fn sync_account(
    app: &AppHandle,
    http: &reqwest::Client,
    c: &google::Client,
    account: &Account,
    tz: &str,
) -> Result<(sync::Outcome, Vec<Calendar>, Vec<String>)> {
    let refresh = read_secret(app, &safe_name(&account.id)?)?
        .ok_or_else(|| AppError::new("CalendarAuth", "Connect this Google account again."))?;
    let access = google::refresh(http, c, &refresh).await?;
    let calendars = merge_calendars(&account.calendars, google::calendars(http, &access).await?);
    let real = chrono::Utc::now();
    let from = crate::agenda::models::utc(real - chrono::Duration::days(PAST_DAYS));
    let to = crate::agenda::models::utc(real + chrono::Duration::days(FUTURE_DAYS));
    let now = now_for_records(app);
    let mut total = sync::Outcome::default();
    let mut problems = Vec::new();
    let p = pool(app).await?;
    for cal in &calendars {
        let Some(project_id) = &cal.project_id else {
            continue;
        };
        let events = match google::events(http, &access, &cal.id, &from, &to).await {
            Ok(e) => e,
            Err(e) if e.code == "CalendarAuth" => return Err(e),
            Err(e) => {
                problems.push(format!("{}: {}", cal.name, e.message));
                continue;
            }
        };
        let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
        if rows(
            &mut tx,
            "SELECT id FROM projects WHERE id=?",
            vec![json!(project_id)],
        )
        .await?
        .is_empty()
        {
            problems.push(format!(
                "{}: its project was deleted. Choose another project.",
                cal.name
            ));
            continue;
        }
        let target = sync::Target {
            account_id: &account.id,
            calendar_id: &cal.id,
            project_id,
            time_zone: tz,
            window_start: &from,
            window_end: &to,
            now: &now,
        };
        match sync::reconcile(&mut tx, &target, &events).await {
            Ok(o) => {
                tx.commit().await?;
                total.add(o);
            }
            Err(e) => problems.push(format!("{}: {}", cal.name, e.message)),
        }
    }
    Ok((total, calendars, problems))
}

pub async fn sync_all(app: &AppHandle) -> Result<SyncSummary> {
    let state = app.state::<Calendars>();
    let _gate = state.gate.lock().await;
    let tz =
        state.time_zone.lock().unwrap().clone().ok_or_else(|| {
            AppError::validation("Signal is still starting. Try again in a moment.")
        })?;
    let p = pool(app).await?;
    let cfg = load(&mut *p.acquire().await?).await?;
    let mut summary = SyncSummary {
        created: 0,
        updated: 0,
        removed: 0,
        canceled: 0,
        errors: vec![],
    };
    let mut changed = false;
    let google_client = if cfg.accounts.is_empty() {
        None
    } else {
        Some((client(app, &cfg)?, google::http()?))
    };
    for account in &cfg.accounts {
        let Some((c, http)) = &google_client else {
            break;
        };
        let result = sync_account(app, http, c, account, &tz).await;
        let stamp = crate::agenda::models::utc(chrono::Utc::now());
        let mut status = state
            .status
            .lock()
            .unwrap()
            .get(&account.id)
            .cloned()
            .unwrap_or_default();
        match result {
            Ok((o, calendars, problems)) => {
                changed |= o.changed();
                summary.created += o.created;
                summary.updated += o.updated;
                summary.removed += o.removed;
                summary.canceled += o.canceled;
                status.last_sync = Some(stamp);
                status.needs_sign_in = false;
                status.last_error = (!problems.is_empty()).then(|| problems.join(" "));
                summary.errors.extend(
                    problems
                        .into_iter()
                        .map(|e| format!("{}: {e}", account.email)),
                );
                // Save fresh calendar names without losing a change made meanwhile.
                let mut conn = p.acquire().await?;
                let mut latest = load(&mut conn).await?;
                if let Some(a) = latest.accounts.iter_mut().find(|a| a.id == account.id) {
                    a.calendars = merge_calendars(
                        &a.calendars,
                        calendars
                            .into_iter()
                            .map(|c| google::CalendarInfo {
                                id: c.id,
                                name: c.name,
                                color: c.color,
                                primary: c.primary,
                            })
                            .collect(),
                    );
                    save(&mut conn, &latest).await?;
                }
            }
            Err(e) => {
                status.needs_sign_in = e.code == "CalendarAuth";
                status.last_error = Some(e.message.clone());
                summary
                    .errors
                    .push(format!("{}: {}", account.email, e.message));
            }
        }
        state
            .status
            .lock()
            .unwrap()
            .insert(account.id.clone(), status);
    }
    changed |= sync_outlook(app, &cfg, &tz, &mut summary).await?;
    if changed {
        let _ = app.emit("signal://data-changed", ());
    }
    Ok(summary)
}

/// Brings in the followed Outlook calendars. Outlook is only started when one is followed.
async fn sync_outlook(
    app: &AppHandle,
    cfg: &Config,
    tz: &str,
    summary: &mut SyncSummary,
) -> Result<bool> {
    let followed: Vec<_> = cfg.outlook.iter().filter(|a| a.project_id.is_some()).collect();
    if followed.is_empty() {
        return Ok(false);
    }
    let state = app.state::<Calendars>();
    let real = chrono::Utc::now();
    let from = crate::agenda::models::utc(real - chrono::Duration::days(PAST_DAYS));
    let to = crate::agenda::models::utc(real + chrono::Duration::days(FUTURE_DAYS));
    let now = now_for_records(app);
    let read = match crate::outlook::events(&followed, &from, &to).await {
        Ok(r) => r,
        Err(e) => followed
            .iter()
            .map(|_| Err(AppError::new(e.code, e.message.clone())))
            .collect(),
    };
    let p = pool(app).await?;
    let mut changed = false;
    for (account, events) in followed.iter().zip(read) {
        let project_id = account.project_id.as_deref().unwrap_or_default();
        let result = async {
            let events = events?;
            let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
            if rows(&mut tx, "SELECT id FROM projects WHERE id=?", vec![json!(project_id)])
                .await?
                .is_empty()
            {
                return Err(AppError::validation(
                    "The calendar's project was deleted. Choose another project.",
                ));
            }
            let target = sync::Target {
                account_id: &account.id,
                calendar_id: crate::outlook::CALENDAR,
                project_id,
                time_zone: tz,
                window_start: &from,
                window_end: &to,
                now: &now,
            };
            let o = sync::reconcile(&mut tx, &target, &events).await?;
            tx.commit().await?;
            Ok(o)
        }
        .await;
        let mut status = state
            .status
            .lock()
            .unwrap()
            .get(&account.id)
            .cloned()
            .unwrap_or_default();
        match result {
            Ok(o) => {
                changed |= o.changed();
                summary.created += o.created;
                summary.updated += o.updated;
                summary.removed += o.removed;
                summary.canceled += o.canceled;
                status.last_sync = Some(crate::agenda::models::utc(chrono::Utc::now()));
                status.last_error = None;
            }
            Err(e) => {
                status.last_error = Some(e.message.clone());
                summary.errors.push(format!("{}: {}", account.email, e.message));
            }
        }
        state.status.lock().unwrap().insert(account.id.clone(), status);
    }
    Ok(changed)
}

async fn worker(app: AppHandle) {
    loop {
        if let Err(e) = sync_all(&app).await {
            eprintln!("Calendar: {}", e.message);
        }
        let state = app.state::<Calendars>();
        tokio::select! {
            _ = tokio::time::sleep(EVERY) => {}
            _ = state.wake.notified() => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Commands.

#[derive(Serialize)]
pub struct AccountView {
    /// "google" or "outlook".
    pub kind: &'static str,
    pub id: String,
    pub email: String,
    pub calendars: Vec<Calendar>,
    pub mail: bool,
    pub mail_tasks: bool,
    pub auto_add: bool,
    pub status: AccountStatus,
}
#[derive(Serialize)]
pub struct StatusView {
    pub has_client: bool,
    pub connecting: bool,
    pub accounts: Vec<AccountView>,
    pub past_days: i64,
    pub future_days: i64,
}

#[tauri::command]
pub async fn calendar_status(app: AppHandle) -> Result<StatusView> {
    let cfg = load(&mut *pool(&app).await?.acquire().await?).await?;
    let state = app.state::<Calendars>();
    let status = state.status.lock().unwrap().clone();
    Ok(StatusView {
        has_client: !cfg.client_id.is_empty() && read_secret(&app, "client.bin")?.is_some(),
        connecting: state.connecting.load(std::sync::atomic::Ordering::SeqCst),
        accounts: cfg
            .accounts
            .into_iter()
            .map(|a| AccountView {
                kind: "google",
                status: status.get(&a.id).cloned().unwrap_or_default(),
                id: a.id,
                email: a.email,
                calendars: a.calendars,
                mail: a.mail,
                mail_tasks: a.mail && a.mail_tasks,
                auto_add: a.mail && a.mail_tasks && a.auto_add,
            })
            .chain(cfg.outlook.into_iter().map(|a| AccountView {
                kind: "outlook",
                status: status.get(&a.id).cloned().unwrap_or_default(),
                calendars: vec![Calendar {
                    id: crate::outlook::CALENDAR.into(),
                    name: "Calendar".into(),
                    color: "#0078d4".into(),
                    primary: true,
                    project_id: a.project_id,
                }],
                id: a.id,
                email: a.email,
                mail: true,
                mail_tasks: a.mail_tasks,
                auto_add: a.mail_tasks && a.auto_add,
            }))
            .collect(),
        past_days: PAST_DAYS,
        future_days: FUTURE_DAYS,
    })
}

/// Loads the Desktop-app client file; with no path, asks for it. False when cancelled.
#[tauri::command]
pub async fn calendar_import_client(app: AppHandle, path: Option<String>) -> Result<bool> {
    use tauri_plugin_dialog::DialogExt;
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => match app
            .dialog()
            .file()
            .set_title("Choose the Google client file")
            .add_filter("Google client file", &["json"])
            .blocking_pick_file()
        {
            Some(p) => p
                .into_path()
                .map_err(|e| AppError::new("File", e.to_string()))?,
            None => return Ok(false),
        },
    };
    let meta = std::fs::metadata(&path)
        .map_err(|_| AppError::validation("Choose the downloaded client file."))?;
    if meta.len() > 64 * 1024 {
        return Err(AppError::validation("This is not a Google client file."));
    }
    let c = google::parse_client(
        &std::fs::read_to_string(&path)
            .map_err(|_| AppError::validation("This is not a Google client file."))?,
    )?;
    write_secret(&app, "client.bin", &c.secret)?;
    let p = pool(&app).await?;
    let mut conn = p.acquire().await?;
    let mut cfg = load(&mut conn).await?;
    cfg.client_id = c.id;
    save(&mut conn, &cfg).await?;
    Ok(true)
}

#[tauri::command]
pub async fn calendar_connect(app: AppHandle) -> Result<String> {
    use std::sync::atomic::Ordering;
    let state = app.state::<Calendars>();
    if state.connecting.swap(true, Ordering::SeqCst) {
        return Err(AppError::validation(
            "A Google sign-in is already waiting in your browser.",
        ));
    }
    let result = connect(&app).await;
    state.connecting.store(false, Ordering::SeqCst);
    let email = result?;
    state.wake.notify_one();
    Ok(email)
}

async fn connect(app: &AppHandle) -> Result<String> {
    use tauri_plugin_opener::OpenerExt;
    let p = pool(app).await?;
    let cfg = load(&mut *p.acquire().await?).await?;
    let c = client(app, &cfg)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let verifier = google::verifier();
    let state_token = uid();
    let url = google::auth_url(
        &c.id,
        &redirect,
        &state_token,
        &google::challenge(&verifier),
        None,
    );
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|_| AppError::new("File", "Could not open your web browser."))?;
    let calendars_state = app.state::<Calendars>();
    let code = tokio::select! {
        r = google::wait_for_code(listener, &state_token) => r?,
        _ = tokio::time::sleep(Duration::from_secs(300)) => return Err(AppError::validation("Google sign-in timed out. Try again.")),
        _ = calendars_state.cancel_connect.notified() => return Err(AppError::validation("Google sign-in cancelled.")),
    };
    let http = google::http()?;
    let tokens = google::exchange(&http, &c, &code, &verifier, &redirect).await?;
    let fresh = google::calendars(&http, &tokens.access).await?;
    let mut conn = p.acquire().await?;
    let mut cfg = load(&mut conn).await?;
    let id = match cfg.accounts.iter().position(|a| a.email == tokens.email) {
        Some(i) => {
            let a = &mut cfg.accounts[i];
            a.calendars = merge_calendars(&a.calendars, fresh);
            a.mail = tokens.mail;
            a.id.clone()
        }
        None => {
            let id = uid();
            cfg.accounts.push(Account {
                id: id.clone(),
                email: tokens.email.clone(),
                calendars: merge_calendars(&[], fresh),
                mail: tokens.mail,
                mail_tasks: false,
                auto_add: false,
            });
            id
        }
    };
    write_secret(app, &safe_name(&id)?, &tokens.refresh)?;
    save(&mut conn, &cfg).await?;
    calendars_state
        .status
        .lock()
        .unwrap()
        .insert(id, AccountStatus::default());
    Ok(tokens.email)
}

#[tauri::command]
pub fn calendar_cancel_connect(app: AppHandle) {
    app.state::<Calendars>().cancel_connect.notify_waiters();
}

#[tauri::command]
pub async fn calendar_set_project(
    app: AppHandle,
    account_id: String,
    calendar_id: String,
    project_id: Option<String>,
) -> Result<()> {
    let p = pool(&app).await?;
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(id) = &project_id {
        one(
            &mut tx,
            "SELECT id FROM projects WHERE id=?",
            vec![json!(id)],
        )
        .await?;
    }
    let mut cfg = load(&mut tx).await?;
    let slot = match cfg.outlook.iter_mut().find(|a| a.id == account_id) {
        Some(a) if calendar_id == crate::outlook::CALENDAR => &mut a.project_id,
        Some(_) => return Err(AppError::missing()),
        None => {
            &mut cfg
                .accounts
                .iter_mut()
                .find(|a| a.id == account_id)
                .and_then(|a| a.calendars.iter_mut().find(|c| c.id == calendar_id))
                .ok_or_else(AppError::missing)?
                .project_id
        }
    };
    let stopped = slot.is_some() && project_id.is_none();
    *slot = project_id;
    save(&mut tx, &cfg).await?;
    if stopped {
        // Already imported meetings stay as ordinary meetings.
        sync::unlink(&mut tx, &account_id, Some(&calendar_id)).await?;
    }
    tx.commit().await?;
    app.state::<Calendars>().wake.notify_one();
    Ok(())
}

/// Turns task suggestions from one Google inbox on or off.
#[tauri::command]
pub async fn calendar_set_mail(app: AppHandle, account_id: String, on: bool) -> Result<()> {
    let p = pool(&app).await?;
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    let mut cfg = load(&mut tx).await?;
    if let Some(a) = cfg.outlook.iter_mut().find(|a| a.id == account_id) {
        a.mail_tasks = on;
        a.auto_add &= on;
        save(&mut tx, &cfg).await?;
        tx.commit().await?;
        crate::intake::email::wake(&app);
        return Ok(());
    }
    let a = cfg
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or_else(AppError::missing)?;
    if on && !a.mail {
        return Err(AppError::validation(
            "Connect this account again and allow Signal to read email on Google's screen.",
        ));
    }
    a.mail_tasks = on;
    a.auto_add &= on;
    save(&mut tx, &cfg).await?;
    tx.commit().await?;
    crate::intake::email::wake(&app);
    Ok(())
}

/// Turns automatic adding of sure tasks on or off for one inbox.
#[tauri::command]
pub async fn calendar_set_auto_add(app: AppHandle, account_id: String, on: bool) -> Result<()> {
    let p = pool(&app).await?;
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    let mut cfg = load(&mut tx).await?;
    let slot = match cfg.outlook.iter_mut().find(|a| a.id == account_id) {
        Some(a) => (a.mail_tasks, &mut a.auto_add),
        None => {
            let a = cfg
                .accounts
                .iter_mut()
                .find(|a| a.id == account_id)
                .ok_or_else(AppError::missing)?;
            (a.mail && a.mail_tasks, &mut a.auto_add)
        }
    };
    if on && !slot.0 {
        return Err(AppError::validation(
            "Turn on task suggestions for this inbox first.",
        ));
    }
    *slot.1 = on;
    save(&mut tx, &cfg).await?;
    tx.commit().await?;
    Ok(())
}

#[tauri::command]
pub async fn calendar_remove_account(app: AppHandle, account_id: String) -> Result<()> {
    let p = pool(&app).await?;
    let mut tx = p.begin_with("BEGIN IMMEDIATE").await?;
    let mut cfg = load(&mut tx).await?;
    cfg.accounts.retain(|a| a.id != account_id);
    cfg.outlook.retain(|a| a.id != account_id);
    save(&mut tx, &cfg).await?;
    sync::unlink(&mut tx, &account_id, None).await?;
    sqlx::query("DELETE FROM mail_seen WHERE account_id=?")
        .bind(&account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    delete_secret(&app, &safe_name(&account_id)?)?;
    app.state::<Calendars>()
        .status
        .lock()
        .unwrap()
        .remove(&account_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_sync_now(app: AppHandle) -> Result<SyncSummary> {
    sync_all(&app).await
}

/// Called once the workspace is open with the display time zone; starts the timer.
#[tauri::command]
pub async fn calendar_runtime(app: AppHandle, time_zone: String) -> Result<()> {
    crate::agenda::models::zone(&time_zone)?;
    let state = app.state::<Calendars>();
    *state.time_zone.lock().unwrap() = Some(time_zone);
    let mut worker_slot = state.worker.lock().await;
    if worker_slot.as_ref().is_none_or(|h| h.inner().is_finished()) {
        *worker_slot = Some(tauri::async_runtime::spawn(worker(app.clone())));
    }
    drop(worker_slot);
    crate::intake::email::start(&app).await;
    Ok(())
}
