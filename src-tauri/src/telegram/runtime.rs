use super::ai::Brain;
use super::api::{Api, HttpApi, API_BASE};
use super::config::{self, Config, Pairing, PairingStatus};
use super::engine::{self, ClockSource, Env};
use crate::workspace::models::*;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default, Clone, Serialize)]
pub struct Status {
    pub running: bool,
    pub connected: bool,
    pub last_error: Option<String>,
    pub last_poll: Option<String>,
}

#[derive(Default)]
pub struct Telegram {
    pub pairing: Pairing,
    worker: tokio::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    time_zone: Mutex<Option<String>>,
    status: Mutex<Status>,
}
impl Telegram {
    fn update(&self, f: impl FnOnce(&mut Status)) {
        f(&mut self.status.lock().unwrap());
    }
}

// ---------------------------------------------------------------------------------------------
// The bot token: protected for this Windows user, in this installation's own folder.

fn folder(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::new("File", "Application storage is unavailable."))?
        .join(if app.state::<crate::db::RuntimeState>().seeded {
            "telegram-dev"
        } else {
            "telegram"
        });
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn read_token(app: &AppHandle) -> Result<Option<String>> {
    let path = folder(app)?.join("token.bin");
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    String::from_utf8(crate::intake::settings::protect(&bytes, true)?)
        .map(Some)
        .map_err(|_| AppError::validation("Save the bot token again."))
}

fn write_token(app: &AppHandle, token: &str) -> Result<()> {
    std::fs::write(
        folder(app)?.join("token.bin"),
        crate::intake::settings::protect(token.as_bytes(), false)?,
    )?;
    Ok(())
}

fn delete_token(app: &AppHandle) -> Result<()> {
    let path = folder(app)?.join("token.bin");
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn valid_token(token: &str) -> bool {
    // <bot id>:<secret>, e.g. 123456789:AA... (letters, digits, '_' and '-').
    token.len() <= 100
        && token.split_once(':').is_some_and(|(id, secret)| {
            !id.is_empty()
                && id.bytes().all(|b| b.is_ascii_digit())
                && secret.len() >= 20
                && secret
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

// ---------------------------------------------------------------------------------------------
// AI and speech, using the AI Inbox provider and the local engine.

struct AppBrain {
    app: AppHandle,
    consent: bool,
}
impl Brain for AppBrain {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        if !self.consent {
            return Err(AppError::validation(
                "Accept the Telegram AI disclosure in Settings → Telegram before I can interpret messages.",
            ));
        }
        crate::intake::chat_json(&self.app, system, user).await
    }
    async fn transcribe(&self, audio: Vec<u8>) -> Result<String> {
        if !self.consent {
            return Err(AppError::validation(
                "Accept the Telegram AI disclosure in Settings → Telegram first.",
            ));
        }
        super::voice::transcribe(&self.app, audio).await
    }
}

// ---------------------------------------------------------------------------------------------
// The polling worker.

async fn ready(app: &AppHandle) -> Option<(SqlitePool, String, i64)> {
    let pool = crate::db::pool(app).await.ok()?;
    let offset = app
        .state::<crate::clock::Clock>()
        .offset
        .read()
        .ok()
        .and_then(|g| *g)?;
    let tz = app.state::<Telegram>().time_zone.lock().unwrap().clone()?;
    Some((pool, tz, offset))
}

async fn applied(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM telegram_applied")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn session(app: &AppHandle) -> Result<()> {
    let state = app.state::<Telegram>();
    let (pool, _, _) = loop {
        if let Some(r) = ready(app).await {
            break r;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    let cfg = config::load(&mut *pool.acquire().await?).await?;
    let Some(token) = read_token(app)?.filter(|_| cfg.enabled) else {
        return Ok(());
    };
    let api = HttpApi::new(API_BASE, &token)?;
    api.clear_webhook().await?;
    // A menu that fails to register must not stop the bot.
    let _ = api.set_commands().await;
    state.update(|s| {
        s.connected = true;
        s.last_error = None;
    });
    loop {
        let Some((pool, tz, offset)) = ready(app).await else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        let cfg = config::load(&mut *pool.acquire().await?).await?;
        if !cfg.enabled {
            return Ok(());
        }
        let brain = AppBrain {
            app: app.clone(),
            consent: cfg.consent,
        };
        let clock = ClockSource::Offset(offset);
        let env = Env {
            pool: &pool,
            api: &api,
            brain: &brain,
            clock: &clock,
            tz: &tz,
            pairing: &state.pairing,
        };
        let before = applied(&pool).await;
        // Finish anything stored before a restart, then wait for new messages.
        engine::run_once(&env).await?;
        let next = engine::next_offset(&mut *pool.acquire().await?).await?;
        let updates = api.updates(next, 25).await?;
        if !updates.is_empty() {
            engine::ingest(&env, &updates).await?;
            engine::run_once(&env).await?;
        }
        state.update(|s| s.last_poll = Some(clock.now()));
        if applied(&pool).await != before {
            let _ = app.emit("signal://data-changed", ());
        }
    }
}

async fn run(app: AppHandle) {
    let state = app.state::<Telegram>();
    state.update(|s| s.running = true);
    let mut wait = 2u64;
    loop {
        match session(&app).await {
            Ok(()) => break,
            Err(e) => {
                eprintln!("Telegram: {}", e.message);
                state.update(|s| {
                    s.connected = false;
                    s.last_error = Some(e.message.clone());
                });
                match e.code {
                    // A bad token will not fix itself.
                    "TelegramAuth" => break,
                    "TelegramConflict" => wait = 30,
                    _ => wait = (wait * 2).min(60),
                }
                tokio::time::sleep(Duration::from_secs(wait)).await;
            }
        }
        if wait < 60 && state.status.lock().unwrap().connected {
            wait = 2;
        }
    }
    state.update(|s| {
        s.running = false;
        s.connected = false;
    });
}

async fn start_worker(app: &AppHandle) {
    let state = app.state::<Telegram>();
    let mut worker = state.worker.lock().await;
    if worker.as_ref().is_some_and(|h| !h.inner().is_finished()) {
        return;
    }
    *worker = Some(tauri::async_runtime::spawn(run(app.clone())));
}

async fn stop_worker(app: &AppHandle) {
    let state = app.state::<Telegram>();
    if let Some(handle) = state.worker.lock().await.take() {
        handle.abort();
    }
    state.update(|s| {
        s.running = false;
        s.connected = false;
    });
}

/// Starts the worker when the saved settings say it should be running.
async fn ensure_worker(app: &AppHandle) -> Result<()> {
    let pool = crate::workspace::pool(app).await?;
    let cfg = config::load(&mut *pool.acquire().await?).await?;
    if cfg.enabled && read_token(app)?.is_some() {
        start_worker(app).await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Commands for the Settings screen.

async fn load_config(app: &AppHandle) -> Result<(SqlitePool, Config)> {
    let pool = crate::workspace::pool(app).await?;
    let cfg = config::load(&mut *pool.acquire().await?).await?;
    Ok((pool, cfg))
}

#[derive(Serialize)]
struct StatusReply {
    has_token: bool,
    bot_username: String,
    enabled: bool,
    consent: bool,
    paired: bool,
    tray: bool,
    pairing: PairingStatus,
    worker: Status,
}

#[tauri::command]
pub async fn telegram_status(app: AppHandle) -> Result<Value> {
    let (_, cfg) = load_config(&app).await?;
    let state = app.state::<Telegram>();
    let worker = state.status.lock().unwrap().clone();
    Ok(json!(StatusReply {
        has_token: read_token(&app)?.is_some(),
        bot_username: cfg.bot_username,
        enabled: cfg.enabled,
        consent: cfg.consent,
        paired: cfg.user_id.is_some(),
        tray: app
            .state::<crate::db::RuntimeState>()
            .close_to_tray
            .load(Ordering::SeqCst),
        pairing: state.pairing.status(),
        worker,
    }))
}

/// Forgets the paired account and everything waiting on it.
async fn reset_conversation(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    for table in ["telegram_flow", "telegram_outbox", "telegram_applied"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE telegram_updates SET processed=1")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[tauri::command]
pub async fn telegram_save_token(app: AppHandle, token: String) -> Result<Value> {
    let token = token.trim().to_owned();
    if !valid_token(&token) {
        return Err(AppError::validation(
            "That doesn't look like a bot token. Copy the whole token from @BotFather (it looks like 123456789:AAH…).",
        ));
    }
    let me = HttpApi::new(API_BASE, &token)?.me().await?;
    let username = me["username"].as_str().unwrap_or_default().to_owned();
    stop_worker(&app).await;
    let (pool, mut cfg) = load_config(&app).await?;
    write_token(&app, &token)?;
    // A different bot is a different conversation: unpair and drop anything waiting.
    reset_conversation(&pool).await?;
    app.state::<Telegram>().pairing.cancel();
    cfg.bot_username = username.clone();
    cfg.user_id = None;
    cfg.chat_id = None;
    config::save(&mut *pool.acquire().await?, &cfg).await?;
    ensure_worker(&app).await?;
    Ok(json!({"bot_username": username}))
}

#[tauri::command]
pub async fn telegram_remove_bot(app: AppHandle) -> Result<()> {
    stop_worker(&app).await;
    let (pool, _) = load_config(&app).await?;
    delete_token(&app)?;
    reset_conversation(&pool).await?;
    app.state::<Telegram>().pairing.cancel();
    config::save(&mut *pool.acquire().await?, &Config::default()).await
}

#[tauri::command]
pub async fn telegram_set_consent(app: AppHandle, consent: bool) -> Result<()> {
    let (pool, mut cfg) = load_config(&app).await?;
    cfg.consent = consent;
    config::save(&mut *pool.acquire().await?, &cfg).await
}

#[tauri::command]
pub async fn telegram_set_enabled(app: AppHandle, enabled: bool) -> Result<()> {
    let (pool, mut cfg) = load_config(&app).await?;
    if enabled && read_token(&app)?.is_none() {
        return Err(AppError::validation("Save a bot token first."));
    }
    cfg.enabled = enabled;
    config::save(&mut *pool.acquire().await?, &cfg).await?;
    if enabled {
        start_worker(&app).await;
    } else {
        stop_worker(&app).await;
        app.state::<Telegram>().pairing.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn telegram_start_pairing(app: AppHandle) -> Result<Value> {
    let (_, cfg) = load_config(&app).await?;
    if read_token(&app)?.is_none() {
        return Err(AppError::validation("Save a bot token first."));
    }
    if !cfg.enabled {
        return Err(AppError::validation("Turn on receiving messages first."));
    }
    if cfg.user_id.is_some() {
        return Err(AppError::validation(
            "A Telegram account is already paired. Unpair it first.",
        ));
    }
    let code = app.state::<Telegram>().pairing.start();
    Ok(json!({"code": code, "minutes": config::CODE_MINUTES, "bot_username": cfg.bot_username}))
}

#[tauri::command]
pub async fn telegram_cancel_pairing(app: AppHandle) -> Result<()> {
    app.state::<Telegram>().pairing.cancel();
    Ok(())
}

#[tauri::command]
pub async fn telegram_unpair(app: AppHandle) -> Result<()> {
    let (pool, mut cfg) = load_config(&app).await?;
    app.state::<Telegram>().pairing.cancel();
    reset_conversation(&pool).await?;
    cfg.user_id = None;
    cfg.chat_id = None;
    config::save(&mut *pool.acquire().await?, &cfg).await
}

/// Called once the workspace is open: supplies the display time zone and starts the worker.
#[tauri::command]
pub async fn telegram_runtime(app: AppHandle, time_zone: String) -> Result<()> {
    let _: chrono_tz::Tz = time_zone
        .parse()
        .map_err(|_| AppError::validation("Choose a valid IANA timezone."))?;
    *app.state::<Telegram>().time_zone.lock().unwrap() = Some(time_zone);
    ensure_worker(&app).await
}

#[cfg(test)]
mod tests {
    use super::valid_token;

    #[test]
    fn bot_tokens_have_the_botfather_shape() {
        assert!(valid_token("123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"));
        assert!(!valid_token("123456789"));
        assert!(!valid_token("abc:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"));
        assert!(!valid_token("123:short"));
        assert!(!valid_token(
            "123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PAL DsaW"
        ));
    }
}
