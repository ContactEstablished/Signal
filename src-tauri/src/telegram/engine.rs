use super::ai::{self, Brain, Context, Interpretation};
use super::api::Api;
use super::config::{self, Config, PairResult, Pairing};
use super::flow;
use super::model::*;
use super::search;
use crate::workspace::models::*;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use sqlx::{Row, SqliteConnection, SqlitePool};

pub const FLOW_MINUTES: i64 = 30;
pub const MAX_VOICE_BYTES: u64 = 2_000_000;

/// Where "now" comes from. The app clock can be offset (the seeded fixture runs in 2025).
pub enum ClockSource {
    Fixed(String),
    Offset(i64),
}
impl ClockSource {
    pub fn now(&self) -> String {
        match self {
            Self::Fixed(v) => v.clone(),
            Self::Offset(ms) => crate::clock::iso(Utc::now().timestamp_millis() + ms)
                .unwrap_or_else(|_| Utc::now().to_rfc3339()),
        }
    }
    /// The app-clock instant at which a Telegram message with this Unix time was sent.
    pub fn message_time(&self, unix_secs: i64) -> String {
        let offset = match self {
            Self::Fixed(_) => 0,
            Self::Offset(ms) => *ms,
        };
        crate::clock::iso(unix_secs * 1000 + offset).unwrap_or_else(|_| self.now())
    }
}

pub struct Env<'a, A: Api, B: Brain> {
    pub pool: &'a SqlitePool,
    pub api: &'a A,
    pub brain: &'a B,
    pub clock: &'a ClockSource,
    pub tz: &'a str,
    pub pairing: &'a Pairing,
}

/// Facts about the turn being processed, shared by every handler.
pub struct Cx {
    pub chat_id: i64,
    pub tz: Tz,
    pub tz_name: String,
    /// Real "now" on the app clock: used for expiry and for timestamps written to data.
    pub now: String,
    /// When the user sent the message: relative dates ("today", "Friday") resolve against this.
    pub message_time: String,
    pub today: NaiveDate,
}
impl Cx {
    pub fn new(chat_id: i64, tz_name: &str, now: String, message_time: String) -> Result<Self> {
        let tz: Tz = tz_name
            .parse()
            .map_err(|_| AppError::validation("Choose a valid IANA timezone."))?;
        let today = DateTime::parse_from_rfc3339(&message_time)
            .map_err(|_| AppError::validation("Invalid message time."))?
            .with_timezone(&tz)
            .date_naive();
        Ok(Self {
            chat_id,
            tz,
            tz_name: tz_name.to_owned(),
            now,
            message_time,
            today,
        })
    }
    pub fn local(&self, iso: &str) -> String {
        DateTime::parse_from_rfc3339(iso)
            .map(|d| {
                d.with_timezone(&self.tz)
                    .format("%a %b %-d, %Y %-I:%M %p %Z")
                    .to_string()
            })
            .unwrap_or_else(|_| iso.to_owned())
    }
    pub fn local_clock(&self, iso: &str) -> String {
        DateTime::parse_from_rfc3339(iso)
            .map(|d| d.with_timezone(&self.tz).format("%-I:%M %p").to_string())
            .unwrap_or_else(|_| iso.to_owned())
    }
    pub fn message_local(&self) -> String {
        DateTime::parse_from_rfc3339(&self.message_time)
            .map(|d| {
                d.with_timezone(&self.tz)
                    .format("%Y-%m-%d %H:%M %A")
                    .to_string()
            })
            .unwrap_or_default()
    }
    pub fn expiry(&self) -> String {
        DateTime::parse_from_rfc3339(&self.now)
            .map(|d| {
                (d.with_timezone(&Utc) + Duration::minutes(FLOW_MINUTES))
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_default()
    }
    pub fn expired(&self, flow: &Flow) -> bool {
        match (
            DateTime::parse_from_rfc3339(&self.now),
            DateTime::parse_from_rfc3339(&flow.expires_at),
        ) {
            (Ok(now), Ok(end)) => now >= end,
            _ => true,
        }
    }
}

/// What a turn wants to send and how it changes the conversation state.
#[derive(Default)]
pub struct Out {
    pub msgs: Vec<OutMsg>,
    pub flow: FlowChange,
    pub config: Option<Config>,
}
#[derive(Default)]
pub enum FlowChange {
    #[default]
    Keep,
    Set(Box<Flow>),
    Clear,
}
pub struct OutMsg {
    pub kind: &'static str,
    pub message_id: Option<i64>,
    pub callback_id: Option<String>,
    pub body: String,
    pub markup: Option<Value>,
    pub flow_token: Option<String>,
}
impl Out {
    pub fn say(&mut self, text: impl Into<String>) {
        self.msgs.push(OutMsg {
            kind: "send",
            message_id: None,
            callback_id: None,
            body: clip(&text.into(), 3900),
            markup: None,
            flow_token: None,
        });
    }
    pub fn prompt(&mut self, text: String, markup: Value, flow_token: &str) {
        self.msgs.push(OutMsg {
            kind: "send",
            message_id: None,
            callback_id: None,
            body: clip(&text, 3900),
            markup: Some(markup),
            flow_token: Some(flow_token.to_owned()),
        });
    }
    pub fn edit(&mut self, message_id: i64, text: impl Into<String>) {
        self.msgs.push(OutMsg {
            kind: "edit",
            message_id: Some(message_id),
            callback_id: None,
            body: clip(&text.into(), 3900),
            markup: None,
            flow_token: None,
        });
    }
    pub fn answer(&mut self, callback_id: &str, text: &str) {
        self.msgs.push(OutMsg {
            kind: "answer",
            message_id: None,
            callback_id: Some(callback_id.to_owned()),
            body: clip(text, 190),
            markup: None,
            flow_token: None,
        });
    }
}

enum Kind {
    Text(String),
    Voice {
        file_id: String,
        size: u64,
    },
    Callback {
        id: String,
        data: String,
        message_id: i64,
    },
    Other,
}
struct Inbound {
    chat_id: i64,
    user_id: i64,
    private: bool,
    date: i64,
    kind: Kind,
}

fn parse_update(v: &Value) -> Option<Inbound> {
    if let Some(m) = v.get("message") {
        let kind = if let Some(t) = m["text"].as_str() {
            Kind::Text(t.to_owned())
        } else if let Some(file_id) = m["voice"]["file_id"].as_str() {
            Kind::Voice {
                file_id: file_id.to_owned(),
                size: m["voice"]["file_size"].as_u64().unwrap_or(0),
            }
        } else {
            Kind::Other
        };
        return Some(Inbound {
            chat_id: m["chat"]["id"].as_i64()?,
            user_id: m["from"]["id"].as_i64()?,
            private: m["chat"]["type"] == "private",
            date: m["date"].as_i64().unwrap_or(0),
            kind,
        });
    }
    let c = v.get("callback_query")?;
    Some(Inbound {
        chat_id: c["message"]["chat"]["id"].as_i64()?,
        user_id: c["from"]["id"].as_i64()?,
        private: c["message"]["chat"]["type"] == "private",
        date: 0,
        kind: Kind::Callback {
            id: c["id"].as_str()?.to_owned(),
            data: c["data"].as_str().unwrap_or("").to_owned(),
            message_id: c["message"]["message_id"].as_i64().unwrap_or(0),
        },
    })
}

/// Stores updates before Telegram is told to forget them. Returns the next offset to request.
pub async fn ingest<A: Api, B: Brain>(env: &Env<'_, A, B>, updates: &[Value]) -> Result<i64> {
    let mut tx = env.pool.begin_with("BEGIN IMMEDIATE").await?;
    let now = env.clock.now();
    for u in updates {
        if let Some(id) = u["update_id"].as_i64() {
            sqlx::query("INSERT OR IGNORE INTO telegram_updates(update_id,received_at,payload) VALUES(?,?,?)")
                .bind(id)
                .bind(&now)
                .bind(u.to_string())
                .execute(&mut *tx)
                .await?;
        }
    }
    let next = next_offset(&mut tx).await?;
    tx.commit().await?;
    Ok(next)
}

pub async fn next_offset(conn: &mut SqliteConnection) -> Result<i64> {
    let max: Option<i64> = sqlx::query_scalar("SELECT MAX(update_id) FROM telegram_updates")
        .fetch_one(conn)
        .await?;
    Ok(max.map_or(0, |m| m + 1))
}

/// Handles every stored update in order, then delivers queued replies.
pub async fn run_once<A: Api, B: Brain>(env: &Env<'_, A, B>) -> Result<()> {
    loop {
        let next = {
            let mut conn = env.pool.acquire().await?;
            sqlx::query("SELECT update_id,payload FROM telegram_updates WHERE processed=0 ORDER BY update_id LIMIT 1")
                .fetch_optional(&mut *conn)
                .await?
        };
        let Some(row) = next else { break };
        let id: i64 = row.get("update_id");
        let payload: String = row.get("payload");
        let value: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
        if let Err(e) = handle(env, id, &value).await {
            // Never loop on a failing update: tell the user and move on.
            eprintln!("Telegram update {id}: {}", e.message);
            let mut tx = env.pool.begin_with("BEGIN IMMEDIATE").await?;
            if let Some(chat) = parse_update(&value).map(|i| i.chat_id) {
                let cfg = config::load(&mut tx).await?;
                if cfg.chat_id == Some(chat) {
                    insert_out(
                        &mut tx,
                        chat,
                        &OutMsg {
                            kind: "send",
                            message_id: None,
                            callback_id: None,
                            body: "Something went wrong and nothing was changed. Please try again."
                                .into(),
                            markup: None,
                            flow_token: None,
                        },
                    )
                    .await?;
                }
            }
            sqlx::query("UPDATE telegram_updates SET processed=1 WHERE update_id=?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
    }
    flush_outbox(env).await
}

enum Auth {
    Ignore,
    Pair(String),
    Allowed,
}

fn authorize(cfg: &Config, inbound: &Inbound) -> Auth {
    if !inbound.private {
        return Auth::Ignore;
    }
    match (cfg.user_id, cfg.chat_id) {
        (Some(user), Some(chat)) => {
            if inbound.user_id == user && inbound.chat_id == chat {
                Auth::Allowed
            } else {
                Auth::Ignore
            }
        }
        _ => match &inbound.kind {
            Kind::Text(t) => t
                .trim()
                .strip_prefix("/start")
                .map(|rest| rest.trim().to_owned())
                .filter(|code| !code.is_empty())
                .map_or(Auth::Ignore, Auth::Pair),
            _ => Auth::Ignore,
        },
    }
}

/// Work that needs the network and must finish before the database transaction opens.
#[derive(Default)]
pub struct Pre {
    pub heard: Option<String>,
    pub interpretation: Option<Result<Interpretation>>,
    pub revision: Option<Result<Action>>,
    pub failure: Option<String>,
}

pub fn is_command(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

async fn ai_context(env_tz: &str, cx: &Cx, conn: &mut SqliteConnection) -> Result<Context> {
    Ok(Context {
        time_zone: env_tz.to_owned(),
        message_local: cx.message_local(),
        today: cx.today,
        projects: search::projects(conn)
            .await?
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect(),
    })
}

async fn handle<A: Api, B: Brain>(
    env: &Env<'_, A, B>,
    update_id: i64,
    value: &Value,
) -> Result<()> {
    let Some(inbound) = parse_update(value) else {
        return mark_only(env.pool, update_id).await;
    };
    let now = env.clock.now();
    let (cfg, existing) = {
        let mut conn = env.pool.acquire().await?;
        let cfg = config::load(&mut conn).await?;
        let existing = flow::load(&mut conn, inbound.chat_id).await?;
        (cfg, existing)
    };
    let auth = authorize(&cfg, &inbound);
    if matches!(auth, Auth::Ignore) {
        return mark_only(env.pool, update_id).await;
    }
    let message_time = match inbound.kind {
        Kind::Callback { .. } => existing
            .as_ref()
            .map(|f| f.message_time.clone())
            .unwrap_or_else(|| now.clone()),
        _ => env.clock.message_time(inbound.date),
    };
    let cx = Cx::new(inbound.chat_id, env.tz, now, message_time)?;

    if let Auth::Pair(code) = auth {
        let mut out = Out::default();
        if env.pairing.attempt(&code) == PairResult::Paired {
            out.config = Some(Config {
                user_id: Some(inbound.user_id),
                chat_id: Some(inbound.chat_id),
                ..cfg
            });
            out.say(format!(
                "✅ Paired. I'll only answer you.\n\n{}",
                flow::HELP
            ));
        }
        return commit(env.pool, update_id, &cx, out).await;
    }

    // Network work first: the transaction below only touches the local database.
    let mut pre = Pre::default();
    let live = existing.as_ref().filter(|f| !cx.expired(f));
    let stage = live.and_then(|f| f.stage.clone());
    let mut turn = Turn::Nothing;
    let mut text: Option<String> = None;
    match &inbound.kind {
        Kind::Text(t) if !is_command(t) => text = Some(t.trim().to_owned()),
        Kind::Text(t) => turn = Turn::Command(t.trim().to_owned()),
        Kind::Voice { file_id, size } => {
            if *size > MAX_VOICE_BYTES {
                pre.failure =
                    Some("That voice note is too large. Please send text instead.".into());
            } else {
                let heard = match env.api.download(file_id, MAX_VOICE_BYTES).await {
                    Ok(bytes) => env.brain.transcribe(bytes).await,
                    Err(e) => Err(e),
                };
                match heard {
                    Ok(t) if !t.trim().is_empty() => {
                        pre.heard = Some(t.trim().to_owned());
                        text = Some(t.trim().to_owned());
                    }
                    Ok(_) => pre.failure = Some(
                        "I couldn't hear anything in that voice note. Please send text instead."
                            .into(),
                    ),
                    Err(e) => {
                        pre.failure = Some(format!(
                            "I couldn't use that voice note ({}) Please send text instead.",
                            e.message
                        ))
                    }
                }
            }
        }
        Kind::Callback {
            id,
            data,
            message_id,
        } => {
            turn = Turn::Callback {
                id: id.clone(),
                data: data.clone(),
                message_id: *message_id,
            };
            // "Discard it and start this request" interprets the parked message.
            if let (Some(f), Some(Stage::Conflict { incoming })) = (live, &stage) {
                if data == &format!("k|{}|{}|d", f.token, f.step) {
                    text = Some(incoming.text.clone());
                }
            }
        }
        Kind::Other => {}
    }
    if let Some(body) = text {
        enum Need {
            Interpret,
            Revise,
            Nothing,
        }
        let is_callback = matches!(turn, Turn::Callback { .. });
        let need = match &stage {
            _ if is_callback => Need::Interpret,
            None => Need::Interpret,
            Some(Stage::Revise(_)) => Need::Revise,
            Some(_) => Need::Nothing,
        };
        if !is_callback {
            turn = Turn::Text(body.clone());
        }
        if !matches!(need, Need::Nothing) {
            let ctx = {
                let mut conn = env.pool.acquire().await?;
                ai_context(env.tz, &cx, &mut conn).await?
            };
            match (need, &stage) {
                (Need::Revise, Some(Stage::Revise(p))) => {
                    pre.revision = Some(ai::revise(env.brain, &ctx, &p.action, &body).await)
                }
                _ => pre.interpretation = Some(ai::interpret(env.brain, &ctx, &body).await),
            }
        }
    } else if matches!(inbound.kind, Kind::Voice { .. }) {
        turn = Turn::Nothing;
    }
    let mut tx = env.pool.begin_with("BEGIN IMMEDIATE").await?;
    let result = flow::turn(&mut tx, &cx, turn, existing, pre).await;
    let out = match result {
        Ok(out) => out,
        Err(e) => {
            tx.rollback().await.ok();
            return Err(e);
        }
    };
    finish(tx, update_id, &cx, out).await
}

/// The parts of an inbound update the conversation logic needs.
pub enum Turn {
    Command(String),
    Text(String),
    Callback {
        id: String,
        data: String,
        message_id: i64,
    },
    Nothing,
}

async fn mark_only(pool: &SqlitePool, update_id: i64) -> Result<()> {
    sqlx::query("UPDATE telegram_updates SET processed=1 WHERE update_id=?")
        .bind(update_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn commit(pool: &SqlitePool, update_id: i64, cx: &Cx, out: Out) -> Result<()> {
    let tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    finish(tx, update_id, cx, out).await
}

async fn finish(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    update_id: i64,
    cx: &Cx,
    out: Out,
) -> Result<()> {
    if let Some(config) = &out.config {
        config::save(&mut tx, config).await?;
    }
    match &out.flow {
        FlowChange::Keep => {}
        FlowChange::Set(flow) => flow::save(&mut tx, cx.chat_id, flow).await?,
        FlowChange::Clear => flow::clear(&mut tx, cx.chat_id).await?,
    }
    for msg in &out.msgs {
        insert_out(&mut tx, cx.chat_id, msg).await?;
    }
    sqlx::query("UPDATE telegram_updates SET processed=1 WHERE update_id=?")
        .bind(update_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn insert_out(conn: &mut SqliteConnection, chat: i64, m: &OutMsg) -> Result<()> {
    sqlx::query("INSERT INTO telegram_outbox(chat_id,kind,message_id,callback_id,body,markup_json,flow_token) VALUES(?,?,?,?,?,?,?)")
        .bind(chat)
        .bind(m.kind)
        .bind(m.message_id)
        .bind(&m.callback_id)
        .bind(&m.body)
        .bind(m.markup.as_ref().map(Value::to_string))
        .bind(&m.flow_token)
        .execute(conn)
        .await?;
    Ok(())
}

/// Sends queued replies in order. A network failure leaves the rest queued for the next pass;
/// a Telegram refusal (blocked chat, message too old to edit) drops just that reply.
pub async fn flush_outbox<A: Api, B: Brain>(env: &Env<'_, A, B>) -> Result<()> {
    loop {
        let row = {
            let mut conn = env.pool.acquire().await?;
            sqlx::query("SELECT * FROM telegram_outbox ORDER BY id LIMIT 1")
                .fetch_optional(&mut *conn)
                .await?
        };
        let Some(row) = row else { return Ok(()) };
        let id: i64 = row.get("id");
        let chat: i64 = row.get("chat_id");
        let kind: String = row.get("kind");
        let body: String = row.get("body");
        let markup: Option<Value> = row
            .get::<Option<String>, _>("markup_json")
            .and_then(|m| serde_json::from_str(&m).ok());
        let sent = match kind.as_str() {
            "send" => env.api.send(chat, &body, markup.as_ref()).await.map(Some),
            "edit" => env
                .api
                .edit(
                    chat,
                    row.get::<Option<i64>, _>("message_id").unwrap_or(0),
                    &body,
                    markup.as_ref(),
                )
                .await
                .map(|_| None),
            _ => env
                .api
                .answer(
                    &row.get::<Option<String>, _>("callback_id")
                        .unwrap_or_default(),
                    &body,
                )
                .await
                .map(|_| None),
        };
        match sent {
            Ok(message_id) => {
                let mut tx = env.pool.begin_with("BEGIN IMMEDIATE").await?;
                if let (Some(mid), Some(token)) =
                    (message_id, row.get::<Option<String>, _>("flow_token"))
                {
                    flow::remember_message(&mut tx, chat, &token, mid).await?;
                }
                sqlx::query("DELETE FROM telegram_outbox WHERE id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
            }
            Err(e) if matches!(e.code, "Network" | "TelegramRate" | "TelegramConflict") => {
                return Err(e)
            }
            Err(e) => {
                eprintln!("Telegram reply dropped: {}", e.message);
                sqlx::query("DELETE FROM telegram_outbox WHERE id=?")
                    .bind(id)
                    .execute(env.pool)
                    .await?;
            }
        }
    }
}

pub fn token() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..8].to_owned()
}
