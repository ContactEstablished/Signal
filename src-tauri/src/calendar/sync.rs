//! Brings one calendar's events into Signal as ordinary meetings.
//! Google owns title, time, link and description; Signal owns the project,
//! notes, attendance, reminders and linked tasks.
use crate::agenda::models::{hash, start_fields, AgendaInput, MeetingRef};
use crate::workspace::models::*;
use serde_json::{json, Value};
use sqlx::SqliteConnection;
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Event {
    pub id: String,
    /// Deleted or cancelled at the source.
    pub cancelled: bool,
    /// Present but not a meeting for Signal: all-day, declined, out of office…
    pub skip: bool,
    pub title: String,
    pub starts_at: String,
    pub duration_min: i64,
    pub link_url: Option<String>,
    pub agenda_md: String,
}
impl Event {
    pub fn fingerprint(&self) -> String {
        hash(&json!([
            self.title,
            self.starts_at,
            self.duration_min,
            self.link_url,
            self.agenda_md
        ]))
    }
}

pub struct Target<'a> {
    pub account_id: &'a str,
    pub calendar_id: &'a str,
    pub project_id: &'a str,
    pub time_zone: &'a str,
    /// Events were read for [window_start, window_end).
    pub window_start: &'a str,
    pub window_end: &'a str,
    pub now: &'a str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Outcome {
    pub created: usize,
    pub updated: usize,
    pub removed: usize,
    pub canceled: usize,
}
impl Outcome {
    pub fn changed(&self) -> bool {
        self.created + self.updated + self.removed + self.canceled > 0
    }
    pub fn add(&mut self, o: Outcome) {
        self.created += o.created;
        self.updated += o.updated;
        self.removed += o.removed;
        self.canceled += o.canceled;
    }
}

const CANCELED: &str = "canceled";
const DRAFT: [&str; 12] = [
    "title",
    "starts_at",
    "start_local",
    "start_offset",
    "time_zone",
    "duration_min",
    "link_url",
    "agenda_md",
    "notes_md",
    "reminder_min",
    "show_in_day",
    "task_ids",
];

async fn apply(conn: &mut SqliteConnection, change: Value, now: &str) -> Result<Value> {
    // Removals are checked against a preview, like the meeting dialog does.
    let expected_fingerprint = if change["action"] == "remove" {
        crate::agenda::preview_into(conn, &change, now).await?["fingerprint"]
            .as_str()
            .map(str::to_owned)
    } else {
        None
    };
    crate::agenda::apply_in(
        conn,
        AgendaInput {
            request_id: format!("calendar:{}", uid()),
            change,
            expected_fingerprint,
        },
        now,
    )
    .await
}

fn reference(meeting_id: &str) -> MeetingRef {
    MeetingRef {
        meeting_id: meeting_id.into(),
        occurrence_key: crate::agenda::models::key(0),
    }
}

/// The meeting's single occurrence, or None if it is gone from Signal.
async fn current(conn: &mut SqliteConnection, meeting_id: &str) -> Result<Option<(Value, i64)>> {
    let Some(m) = rows(
        conn,
        "SELECT id FROM meetings WHERE id=?",
        vec![json!(meeting_id)],
    )
    .await?
    .pop() else {
        return Ok(None);
    };
    let m = crate::agenda::recurrence::root(conn, text(&m, "id")?).await?;
    let revision = m["revision"].as_i64().unwrap_or(0);
    Ok(crate::agenda::recurrence::resolve(conn, &m, 0)
        .await?
        .map(|o| (o, revision)))
}

fn draft(e: &Event, tz: &str, base: Option<&Value>) -> Result<Value> {
    let mut d = start_fields(&e.starts_at, tz)?;
    match base {
        Some(o) => {
            for k in DRAFT {
                if d.get(k).is_none() {
                    d[k] = o[k].clone();
                }
            }
            // Keep the meeting's own zone so its local time reads the same way.
            if let Some(z) = o["time_zone"].as_str() {
                let f = start_fields(&e.starts_at, z)?;
                for k in ["start_local", "start_offset", "time_zone"] {
                    d[k] = f[k].clone();
                }
            }
        }
        None => {
            d["notes_md"] = json!("");
            d["reminder_min"] = json!(10);
            d["show_in_day"] = json!(true);
            d["task_ids"] = json!([]);
            d["repeat_rule"] = json!("none");
            d["repeat_until"] = Value::Null;
            d["fold_policy"] = json!("earlier");
        }
    }
    d["title"] = json!(e.title);
    d["duration_min"] = json!(e.duration_min);
    d["link_url"] = json!(e.link_url);
    d["agenda_md"] = json!(e.agenda_md);
    Ok(d)
}

async fn link(
    conn: &mut SqliteConnection,
    t: &Target<'_>,
    event_id: &str,
) -> Result<Option<Value>> {
    Ok(rows(
        conn,
        "SELECT * FROM calendar_links WHERE account_id=? AND calendar_id=? AND event_id=?",
        vec![json!(t.account_id), json!(t.calendar_id), json!(event_id)],
    )
    .await?
    .pop())
}

async fn forget(conn: &mut SqliteConnection, t: &Target<'_>, event_id: &str) -> Result<()> {
    execute(
        conn,
        "DELETE FROM calendar_links WHERE account_id=? AND calendar_id=? AND event_id=?",
        vec![json!(t.account_id), json!(t.calendar_id), json!(event_id)],
    )
    .await
}

/// The event is gone. An untouched meeting is removed; one with notes,
/// attendance or linked tasks stays, marked cancelled.
async fn drop_event(
    conn: &mut SqliteConnection,
    t: &Target<'_>,
    l: &Value,
    out: &mut Outcome,
) -> Result<()> {
    let event_id = text(l, "event_id")?;
    let Some(meeting_id) = l["meeting_id"].as_str() else {
        return forget(conn, t, event_id).await;
    };
    if l["fingerprint"] == CANCELED {
        return Ok(());
    }
    let Some((o, revision)) = current(conn, meeting_id).await? else {
        return forget(conn, t, event_id).await;
    };
    let touched = !text(&o, "notes_md")?.trim().is_empty()
        || !text(&o, "occurrence_notes_md")?.trim().is_empty()
        || o["attendance"] != "unmarked"
        || o["task_ids"].as_array().is_some_and(|a| !a.is_empty())
        || o["repeat_rule"] != "none";
    if touched {
        if o["attendance"] != "canceled" {
            apply(conn, json!({"action":"record","payload":{"ref":reference(meeting_id),"expectedRevision":revision,"attendance":"canceled","occurrence_notes_md":o["occurrence_notes_md"]}}), t.now).await?;
        }
        execute(
            conn,
            "UPDATE calendar_links SET fingerprint=? WHERE account_id=? AND calendar_id=? AND event_id=?",
            vec![json!(CANCELED), json!(t.account_id), json!(t.calendar_id), json!(event_id)],
        )
        .await?;
        out.canceled += 1;
    } else {
        forget(conn, t, event_id).await?;
        apply(conn, json!({"action":"remove","payload":{"ref":reference(meeting_id),"expectedRevision":revision,"scope":"one_off"}}), t.now).await?;
        out.removed += 1;
    }
    Ok(())
}

pub async fn reconcile(
    conn: &mut SqliteConnection,
    t: &Target<'_>,
    events: &[Event],
) -> Result<Outcome> {
    let mut out = Outcome::default();
    let mut seen = HashSet::new();
    for e in events {
        if !seen.insert(e.id.clone()) {
            continue;
        }
        let existing = link(conn, t, &e.id).await?;
        if e.cancelled || e.skip {
            if let Some(l) = existing {
                drop_event(conn, t, &l, &mut out).await?;
            }
            continue;
        }
        let fingerprint = e.fingerprint();
        let Some(l) = existing else {
            let r = apply(conn, json!({"action":"create","payload":{"projectId":t.project_id,"draft":draft(e, t.time_zone, None)?}}), t.now).await?;
            let meeting_id = r["outcome"]["ref"]["meetingId"].as_str().ok_or_else(|| {
                AppError::new("Database", "The imported meeting was not created.")
            })?;
            insert(
                conn,
                "calendar_links",
                &json!({"account_id":t.account_id,"calendar_id":t.calendar_id,"event_id":e.id,"meeting_id":meeting_id,"starts_at":e.starts_at,"fingerprint":fingerprint}),
            )
            .await?;
            out.created += 1;
            continue;
        };
        if l["fingerprint"] == fingerprint.as_str() {
            continue;
        }
        let Some(meeting_id) = l["meeting_id"].as_str() else {
            // Deleted in Signal: stay deleted, but follow the time for window bookkeeping.
            execute(
                conn,
                "UPDATE calendar_links SET starts_at=? WHERE account_id=? AND calendar_id=? AND event_id=?",
                vec![json!(e.starts_at), json!(t.account_id), json!(t.calendar_id), json!(e.id)],
            )
            .await?;
            continue;
        };
        let Some((o, revision)) = current(conn, meeting_id).await? else {
            continue;
        };
        let mut payload = json!({"ref":reference(meeting_id),"expectedRevision":revision,"draft":draft(e, t.time_zone, Some(&o))?});
        if l["fingerprint"] == CANCELED && o["attendance"] == "canceled" {
            // The event is back: undo the cancellation, keep the notes.
            payload["occurrence"] =
                json!({"attendance":"unmarked","occurrence_notes_md":o["occurrence_notes_md"]});
        }
        apply(
            conn,
            json!({"action":"edit_occurrence","payload":payload}),
            t.now,
        )
        .await?;
        execute(
            conn,
            "UPDATE calendar_links SET starts_at=?,fingerprint=? WHERE account_id=? AND calendar_id=? AND event_id=?",
            vec![json!(e.starts_at), json!(fingerprint), json!(t.account_id), json!(t.calendar_id), json!(e.id)],
        )
        .await?;
        out.updated += 1;
    }
    // Events in the window that Google no longer lists were deleted.
    for l in rows(
        conn,
        "SELECT * FROM calendar_links WHERE account_id=? AND calendar_id=? AND starts_at>=? AND starts_at<? ORDER BY event_id",
        vec![json!(t.account_id), json!(t.calendar_id), json!(t.window_start), json!(t.window_end)],
    )
    .await?
    {
        if !seen.contains(text(&l, "event_id")?) {
            drop_event(conn, t, &l, &mut out).await?;
        }
    }
    // Deleted-in-Signal markers are only needed while the event can still be listed.
    execute(
        conn,
        "DELETE FROM calendar_links WHERE account_id=? AND calendar_id=? AND meeting_id IS NULL AND starts_at<?",
        vec![json!(t.account_id), json!(t.calendar_id), json!(t.window_start)],
    )
    .await?;
    Ok(out)
}

/// Stops following a calendar: its meetings stay as ordinary Signal meetings.
pub async fn unlink(
    conn: &mut SqliteConnection,
    account_id: &str,
    calendar_id: Option<&str>,
) -> Result<()> {
    execute(
        conn,
        "DELETE FROM calendar_links WHERE account_id=? AND (? IS NULL OR calendar_id=?)",
        vec![json!(account_id), json!(calendar_id), json!(calendar_id)],
    )
    .await
}

/// Where an imported meeting came from, for the meeting dialog.
pub async fn source(conn: &mut SqliteConnection, meeting_id: &str) -> Result<Value> {
    // Agenda migration tests run meeting code on schemas older than calendar_links.
    let linked = "SELECT calendar_id FROM calendar_links WHERE meeting_id=?";
    let sql = if rows(
        conn,
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='calendar_links'",
        vec![],
    )
    .await?
    .is_empty()
    {
        "SELECT NULL AS calendar_id WHERE 0 AND ?"
    } else {
        linked
    };
    Ok(rows(conn, sql, vec![json!(meeting_id)])
        .await?
        .pop()
        .map(|l| {
            json!({"kind": if l["calendar_id"] == crate::outlook::CALENDAR { "outlook" } else { "google" }})
        })
        .unwrap_or(Value::Null))
}
