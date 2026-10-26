use crate::workspace::models::*;
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Telegram settings stored in the installation's own SQLite `settings` table. The bot
/// token is not here; it is protected for the Windows user in a separate file.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    /// The user accepted the disclosure that Telegram text and voice transcripts go to the AI provider.
    pub consent: bool,
    pub bot_username: String,
    /// The one paired Telegram account and its private chat.
    pub user_id: Option<i64>,
    pub chat_id: Option<i64>,
}

pub async fn load(conn: &mut SqliteConnection) -> Result<Config> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key='telegram'")
        .fetch_optional(conn)
        .await?;
    raw.map(|v| {
        serde_json::from_str(&v).map_err(|_| AppError::validation("Telegram settings are invalid."))
    })
    .transpose()
    .map(|v| v.unwrap_or_default())
}

pub async fn save(conn: &mut SqliteConnection, config: &Config) -> Result<()> {
    sqlx::query("INSERT INTO settings(key,value) VALUES('telegram',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(serde_json::to_string(config).expect("config serializes"))
        .execute(conn)
        .await?;
    Ok(())
}

pub const CODE_MINUTES: u64 = 5;
pub const MAX_FAILURES: u32 = 5;

#[derive(Debug, PartialEq)]
pub enum PairResult {
    Paired,
    /// Wrong, expired, locked or never started. Callers stay silent for all of these.
    Rejected,
}

struct Active {
    code: String,
    expires: Instant,
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    failures: u32,
}

/// One-time pairing code, kept only in memory: restarting Signal cancels pairing.
#[derive(Default)]
pub struct Pairing(Mutex<State>);

#[derive(Debug, Serialize, PartialEq)]
pub struct PairingStatus {
    pub active: bool,
    pub seconds_left: u64,
    pub failures: u32,
    pub locked: bool,
}

fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0, |acc, (x, y)| acc | (x ^ y))
            == 0
}

impl Pairing {
    /// Starts pairing and returns the six-digit code to show in Settings.
    pub fn start(&self) -> String {
        let bytes = *uuid::Uuid::new_v4().as_bytes();
        let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
        let code = format!("{n:06}");
        let mut state = self.0.lock().unwrap();
        state.active = Some(Active {
            code: code.clone(),
            expires: Instant::now() + Duration::from_secs(CODE_MINUTES * 60),
        });
        state.failures = 0;
        code
    }

    pub fn cancel(&self) {
        let mut state = self.0.lock().unwrap();
        state.active = None;
        state.failures = 0;
    }

    pub fn status(&self) -> PairingStatus {
        let mut state = self.0.lock().unwrap();
        if state
            .active
            .as_ref()
            .is_some_and(|a| a.expires <= Instant::now())
        {
            state.active = None;
        }
        PairingStatus {
            active: state.active.is_some(),
            seconds_left: state
                .active
                .as_ref()
                .map(|a| {
                    a.expires
                        .saturating_duration_since(Instant::now())
                        .as_secs()
                })
                .unwrap_or(0),
            failures: state.failures,
            locked: state.active.is_none() && state.failures >= MAX_FAILURES,
        }
    }

    /// Checks a code from any sender. The code is consumed on success and invalidated after
    /// five failed attempts across all senders.
    pub fn attempt(&self, code: &str) -> PairResult {
        let mut state = self.0.lock().unwrap();
        let live = state
            .active
            .as_ref()
            .is_some_and(|a| a.expires > Instant::now());
        if !live {
            state.active = None;
            return PairResult::Rejected;
        }
        if same(&state.active.as_ref().unwrap().code, code.trim()) {
            state.active = None;
            state.failures = 0;
            return PairResult::Paired;
        }
        state.failures += 1;
        if state.failures >= MAX_FAILURES {
            state.active = None;
        }
        PairResult::Rejected
    }

    #[cfg(test)]
    pub fn expire_now(&self) {
        if let Some(a) = self.0.lock().unwrap().active.as_mut() {
            a.expires = Instant::now() - Duration::from_secs(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_pairs_once_and_five_failures_lock_it() {
        let pairing = Pairing::default();
        assert_eq!(pairing.attempt("123456"), PairResult::Rejected);
        let code = pairing.start();
        assert_eq!(code.len(), 6);
        assert_eq!(pairing.attempt("000000x"), PairResult::Rejected);
        assert_eq!(pairing.attempt(&format!(" {code} ")), PairResult::Paired);
        assert_eq!(pairing.attempt(&code), PairResult::Rejected);

        let code = pairing.start();
        let wrong = if code == "000000" { "000001" } else { "000000" };
        for _ in 0..MAX_FAILURES {
            assert_eq!(pairing.attempt(wrong), PairResult::Rejected);
        }
        assert!(pairing.status().locked);
        assert_eq!(pairing.attempt(&code), PairResult::Rejected);
    }

    #[test]
    fn an_expired_code_is_rejected() {
        let pairing = Pairing::default();
        let code = pairing.start();
        pairing.expire_now();
        assert_eq!(pairing.attempt(&code), PairResult::Rejected);
        assert!(!pairing.status().active);
    }
}
