//! Classic Outlook on this computer → Signal meetings and email task suggestions.
//! Read-only: Signal asks the Outlook already signed in on this machine, through
//! its local automation interface, and never sends, moves, marks or changes mail
//! or calendar items. The new Outlook has no such interface; see
//! docs/Features/Signal/Mail-Calendar-Integration-Plan.md.
#[cfg(test)]
mod tests;

use crate::calendar::sync::Event;
use crate::intake::email::{trim_reply, Email};
use crate::workspace::models::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tauri::AppHandle;

/// Each Outlook account has one calendar, its default one; links use this calendar ID.
pub const CALENDAR: &str = "outlook";
const SCRIPT: &str = include_str!("bridge.ps1");

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Account {
    pub id: String,
    pub email: String,
    /// Outlook's store ID for this mailbox; refreshed by "Find Outlook accounts".
    pub store_id: String,
    /// The calendar is brought into this project; None means not followed.
    pub project_id: Option<String>,
    /// The person turned on task suggestions from this inbox, which sends new
    /// email to the configured AI provider.
    pub mail_tasks: bool,
    /// Sure tasks from this inbox are added without review.
    pub auto_add: bool,
}

// ---------------------------------------------------------------------------------------------
// The bridge: one short Windows PowerShell run per request.

fn friendly(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("80040154") || lower.contains("class not registered") {
        "Classic Outlook is not installed on this computer.".into()
    } else if lower.contains("80080005") || lower.contains("server execution failed") {
        "Classic Outlook could not start. Open Outlook (classic), finish any window it shows, and try again.".into()
    } else {
        format!("Outlook: {}", raw.trim().chars().take(300).collect::<String>())
    }
}

/// The script as PowerShell's -EncodedCommand expects it (UTF-16LE, base64).
pub fn encoded_script() -> String {
    use base64::Engine as _;
    let bytes: Vec<u8> = SCRIPT.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn answer(stdout: &str) -> Result<Value> {
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| AppError::new("Outlook", "Outlook did not answer. Try again."))?;
    let v: Value = serde_json::from_str(line.trim())
        .map_err(|_| AppError::new("Outlook", "Outlook's answer was not readable. Try again."))?;
    if let Some(e) = v["error"].as_str() {
        return Err(AppError::new("Outlook", friendly(e)));
    }
    Ok(v)
}

#[cfg(windows)]
async fn run(op: &str, args: Value) -> Result<Value> {
    let mut cmd = tokio::process::Command::new("powershell.exe");
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded_script(),
    ])
    .env("SIGNAL_OUTLOOK_OP", op)
    .env("SIGNAL_OUTLOOK_ARGS", args.to_string())
    .stdin(std::process::Stdio::null())
    .kill_on_drop(true)
    // No console window flashing up during a background sync.
    .creation_flags(0x0800_0000);
    let out = tokio::time::timeout(Duration::from_secs(120), cmd.output())
        .await
        .map_err(|_| {
            AppError::new(
                "Outlook",
                "Outlook did not answer within 2 minutes. If Outlook (classic) is showing a sign-in or other window, finish it and try again.",
            )
        })?
        .map_err(|_| AppError::new("Outlook", "Could not start Windows PowerShell to read Outlook."))?;
    answer(&String::from_utf8_lossy(&out.stdout))
}
#[cfg(not(windows))]
async fn run(_: &str, _: Value) -> Result<Value> {
    Err(AppError::validation("Outlook is available on Windows only."))
}

// ---------------------------------------------------------------------------------------------
// Calendar items.

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawEvent {
    pub gid: String,
    pub entry_id: String,
    pub recurring: bool,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    pub busy: i64,
    pub response: i64,
    pub meeting: i64,
    pub subject: String,
    pub location: String,
    pub body: String,
}

const MEETING_HOSTS: [&str; 9] = [
    "teams.microsoft.com",
    "teams.live.com",
    "zoom.us",
    "zoomgov.com",
    "meet.google.com",
    "webex.com",
    "gotomeeting.com",
    "chime.aws",
    "whereby.com",
];

fn urls(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '(' | ')' | '[' | ']'))
        .filter(|t| t.starts_with("https://") || t.starts_with("http://"))
        .map(|t| t.trim_end_matches(['.', ',', ';', ':']).to_owned())
        .filter(|u| crate::links::validate(u).is_ok())
}
fn meeting_host(u: &str) -> bool {
    url::Url::parse(u)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .is_some_and(|h| MEETING_HOSTS.iter().any(|m| h == *m || h.ends_with(&format!(".{m}"))))
}
/// The join link: a link in the location, else a known meeting service in the body.
pub fn join_link(location: &str, body: &str) -> Option<String> {
    urls(location)
        .next()
        .or_else(|| urls(body).find(|u| meeting_host(u)))
}
/// Outlook's plain-text body without the Teams/Zoom join block and inline link targets.
pub fn agenda_text(body: &str) -> String {
    let body = body.replace("\r\n", "\n");
    let mut kept = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        if t.len() >= 20 && t.chars().all(|c| c == '_' || c == '-') {
            break;
        }
        // "Agenda<https://…>": keep the words, drop the link target.
        let mut clean = String::new();
        let mut rest = line;
        while let Some(i) = rest.find("<http") {
            clean.push_str(&rest[..i]);
            match rest[i..].find('>') {
                Some(j) => rest = &rest[i + j + 1..],
                None => {
                    rest = "";
                }
            }
        }
        clean.push_str(rest);
        kept.push(clean.trim_end().to_owned());
    }
    let mut text = String::new();
    let mut blank = 0;
    for line in kept {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        text.push_str(&line);
        text.push('\n');
    }
    text.trim().to_owned()
}

const DECLINED: i64 = 4; // olResponseDeclined
const OUT_OF_OFFICE: i64 = 3; // olOutOfOffice
const CANCELED: [i64; 2] = [5, 7]; // olMeetingCanceled, olMeetingReceivedAndCanceled

pub fn parse_event(r: &RawEvent) -> Option<Event> {
    let start = chrono::DateTime::parse_from_rfc3339(&r.start).ok()?;
    let end = chrono::DateTime::parse_from_rfc3339(&r.end).ok()?;
    let starts_at = crate::agenda::models::utc(start.with_timezone(&chrono::Utc));
    let base = if r.gid.is_empty() { &r.entry_id } else { &r.gid };
    if base.is_empty() {
        return None;
    }
    // Occurrences of a series share one ID; their start tells them apart.
    let id = if r.recurring {
        format!("{base}|{starts_at}")
    } else {
        base.clone()
    };
    let mut e = Event {
        id,
        starts_at,
        ..Event::default()
    };
    if CANCELED.contains(&r.meeting) {
        e.cancelled = true;
        return Some(e);
    }
    let minutes = (end - start).num_minutes();
    e.skip = r.all_day || r.response == DECLINED || r.busy == OUT_OF_OFFICE || minutes > 1440;
    e.duration_min = minutes.clamp(1, 1440);
    let title: String = r.subject.trim().chars().take(300).collect();
    e.title = if title.is_empty() {
        "(No title)".into()
    } else {
        title
    };
    let location = r.location.trim();
    e.link_url = join_link(location, &r.body);
    let mut agenda = String::new();
    if !location.is_empty() && e.link_url.as_deref() != Some(location) {
        agenda.push_str(&format!("Location: {location}\n\n"));
    }
    agenda.push_str(&agenda_text(&r.body));
    e.agenda_md = agenda.trim().chars().take(5000).collect();
    Some(e)
}

/// Reads the default calendar of each account, in [from, to). One result per account.
pub async fn events(accounts: &[&Account], from: &str, to: &str) -> Result<Vec<Result<Vec<Event>>>> {
    let stores: Vec<_> = accounts.iter().map(|a| a.store_id.clone()).collect();
    let v = run("events", json!({"stores":stores,"from":from,"to":to})).await?;
    Ok(accounts
        .iter()
        .map(|a| {
            let r = v["results"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["store_id"] == a.store_id.as_str())
                .ok_or_else(|| AppError::new("Outlook", "Outlook did not read this calendar."))?;
            if let Some(e) = r["error"].as_str() {
                return Err(AppError::new("Outlook", friendly(e)));
            }
            Ok(r["events"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| serde_json::from_value::<RawEvent>(e.clone()).ok())
                .filter_map(|e| parse_event(&e))
                .collect())
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Mail.

/// "<account id>:<Outlook entry ID>", carried by a suggestion for Open email.
pub fn mail_ref(account_id: &str, entry_id: &str) -> String {
    format!("{account_id}:{entry_id}")
}
pub fn parse_ref(r: &str) -> Result<(&str, &str)> {
    let bad = || AppError::validation("Invalid Outlook message.");
    let (account, entry) = r.split_once(':').ok_or_else(bad)?;
    crate::calendar::safe_name(account).map_err(|_| bad())?;
    if entry.is_empty() || entry.len() > 1000 || !entry.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    Ok((account, entry))
}

/// Inbox mail received since `since`, newest first: entry IDs per account.
pub async fn mail_list(accounts: &[&Account], since: &str) -> Result<Vec<Result<Vec<String>>>> {
    let stores: Vec<_> = accounts.iter().map(|a| a.store_id.clone()).collect();
    let v = run("mail_list", json!({"stores":stores,"since":since})).await?;
    Ok(accounts
        .iter()
        .map(|a| {
            let r = v["results"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["store_id"] == a.store_id.as_str())
                .ok_or_else(|| AppError::new("Outlook", "Outlook did not read this inbox."))?;
            if let Some(e) = r["error"].as_str() {
                return Err(AppError::new("Outlook", friendly(e)));
            }
            Ok(r["ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|i| i.as_str())
                .filter(|i| !i.is_empty() && i.len() <= 1000 && i.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(str::to_owned)
                .collect())
        })
        .collect())
}

pub fn parse_mail(v: &Value, a: &Account) -> Option<Email> {
    let s = |k: &str| v[k].as_str().unwrap_or_default().trim().to_owned();
    let (name, address) = (s("from_name"), s("from_address"));
    let from = match (name.is_empty(), address.is_empty()) {
        (_, true) => name,
        (true, false) => address,
        (false, false) if name.eq_ignore_ascii_case(&address) => address,
        (false, false) => format!("{name} <{address}>"),
    };
    let subject = s("subject");
    Some(Email {
        account_id: a.id.clone(),
        mailbox: a.email.clone(),
        id: v["id"].as_str().filter(|i| !i.is_empty())?.to_owned(),
        thread_id: String::new(),
        from,
        // SMTP addresses when Outlook gave them, else the display names in To.
        to: Some(s("to_addresses")).filter(|t| !t.is_empty()).unwrap_or_else(|| s("to")),
        subject: if subject.is_empty() {
            "(no subject)".into()
        } else {
            subject
        },
        received_at: s("received"),
        message_id: s("message_id"),
        body: trim_reply(&s("body").replace("\r\n", "\n")),
        outlook: true,
    })
}

/// Reads the picked messages. One result per item, in order.
pub async fn mail_get(items: &[(&Account, String)]) -> Result<Vec<Result<Email>>> {
    let q: Vec<_> = items
        .iter()
        .map(|(a, id)| json!({"store_id":a.store_id,"id":id}))
        .collect();
    let v = run("mail_get", json!({ "items": q })).await?;
    let results = v["results"].as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .map(|(a, id)| {
            let r = results
                .iter()
                .find(|r| r["id"] == id.as_str() && r["store_id"] == a.store_id.as_str())
                .ok_or_else(|| AppError::new("Outlook", "Outlook did not read this email."))?;
            if let Some(e) = r["error"].as_str() {
                return Err(AppError::new("Outlook", friendly(e)));
            }
            parse_mail(r, a).ok_or_else(|| AppError::new("Outlook", "Outlook returned an unreadable email."))
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Commands.

#[derive(Serialize)]
pub struct Found {
    pub added: Vec<String>,
    pub found: usize,
}

/// Adds the accounts set up in classic Outlook; known ones keep their choices.
#[tauri::command]
pub async fn outlook_add_accounts(app: AppHandle) -> Result<Found> {
    let v = run("accounts", json!({})).await?;
    let found: Vec<(String, String)> = v["accounts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let email = a["email"].as_str()?.trim();
            let store = a["store_id"].as_str()?.trim();
            (!email.is_empty() && !store.is_empty()).then(|| (email.to_owned(), store.to_owned()))
        })
        .collect();
    if found.is_empty() {
        return Err(AppError::validation(
            "Classic Outlook has no email accounts yet. Add them in Outlook (classic) → File → Add Account, then try again.",
        ));
    }
    let pool = crate::workspace::pool(&app).await?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut cfg = crate::calendar::load(&mut tx).await?;
    let mut added = Vec::new();
    for (email, store_id) in &found {
        match cfg
            .outlook
            .iter_mut()
            .find(|a| a.email.eq_ignore_ascii_case(email))
        {
            Some(a) => a.store_id = store_id.clone(),
            None => {
                cfg.outlook.push(Account {
                    id: uid(),
                    email: email.clone(),
                    store_id: store_id.clone(),
                    project_id: None,
                    mail_tasks: false,
                    auto_add: false,
                });
                added.push(email.clone());
            }
        }
    }
    crate::calendar::save(&mut tx, &cfg).await?;
    tx.commit().await?;
    Ok(Found {
        added,
        found: found.len(),
    })
}

/// Shows one suggestion's email in Outlook (classic).
#[tauri::command]
pub async fn outlook_open(app: AppHandle, mail_ref: String) -> Result<()> {
    let (account_id, entry_id) = parse_ref(&mail_ref)?;
    let pool = crate::workspace::pool(&app).await?;
    let cfg = crate::calendar::load(&mut *pool.acquire().await?).await?;
    let a = cfg
        .outlook
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::validation("This Outlook account was removed from Signal."))?;
    run("open", json!({"store_id":a.store_id,"id":entry_id})).await?;
    Ok(())
}
