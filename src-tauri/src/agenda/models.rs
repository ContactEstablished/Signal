use crate::workspace::models::*;
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub fn hash(value: &Value) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("JSON"))
    )
}
pub fn utc(v: DateTime<Utc>) -> String {
    v.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub fn parse(v: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v)
        .map(|v| v.with_timezone(&Utc))
        .map_err(|_| AppError::validation("Enter a valid UTC instant."))
}
pub fn date(v: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(v, "%Y-%m-%d")
        .map_err(|_| AppError::validation("Enter a valid calendar date."))
}
pub fn zone(v: &str) -> Result<Tz> {
    v.parse()
        .map_err(|_| AppError::validation("Choose a valid IANA timezone."))
}
pub fn number(v: &Value, k: &str) -> Result<i64> {
    v[k].as_i64()
        .ok_or_else(|| AppError::validation(format!("Invalid {k}.")))
}
pub fn key(n: i64) -> String {
    format!("o:{n}")
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeetingRef {
    pub meeting_id: String,
    pub occurrence_key: String,
}
impl MeetingRef {
    pub fn ordinal(&self) -> Result<i64> {
        let n = self
            .occurrence_key
            .strip_prefix("o:")
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|n| *n >= 0)
            .ok_or_else(|| AppError::validation("Invalid occurrence reference."))?;
        if key(n) != self.occurrence_key {
            return Err(AppError::validation("Invalid occurrence reference."));
        }
        Ok(n)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgendaQuery {
    pub date: String,
    pub time_zone: String,
    pub project_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgendaInput {
    pub request_id: String,
    pub change: Value,
    pub expected_fingerprint: Option<String>,
}
pub fn start_fields(at: &str, tz: &str) -> Result<Value> {
    let z = parse(at)?.with_timezone(&zone(tz)?);
    Ok(
        json!({"starts_at":at,"start_local":z.format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),"start_offset":z.offset().fix().to_string(),"time_zone":tz}),
    )
}
pub fn normalize(d: &Value, series: bool) -> Result<Value> {
    let allowed = [
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
        "repeat_rule",
        "repeat_weekdays",
        "repeat_until",
        "fold_policy",
    ];
    let map = d
        .as_object()
        .ok_or_else(|| AppError::validation("Invalid meeting."))?;
    if map.keys().any(|k| {
        !allowed.contains(&k.as_str())
            || (!series
                && [
                    "repeat_rule",
                    "repeat_weekdays",
                    "repeat_until",
                    "fold_policy",
                ]
                .contains(&k.as_str()))
    }) {
        return Err(AppError::validation("Unknown meeting field."));
    }
    let mut out = d.clone();
    out["title"] = json!(nonblank(text(d, "title")?)?);
    let z = zone(text(d, "time_zone")?)?;
    let wall = NaiveDateTime::parse_from_str(text(d, "start_local")?, "%Y-%m-%dT%H:%M:%S%.f")
        .map_err(|_| AppError::validation("Enter a valid local start time."))?;
    let offset = text(d, "start_offset")?
        .parse::<FixedOffset>()
        .map_err(|_| AppError::validation("Choose the start UTC offset."))?;
    let at = parse(text(d, "starts_at")?)?;
    if ![
        z.from_local_datetime(&wall).earliest(),
        z.from_local_datetime(&wall).latest(),
    ]
    .into_iter()
    .flatten()
    .any(|v| v.offset().fix() == offset && v.with_timezone(&Utc) == at)
    {
        return Err(AppError::validation(
            "Start time, timezone and offset disagree, or this local time does not exist.",
        ));
    }
    for (k, min, max) in [
        ("duration_min", 1, 1440),
        ("reminder_min", 0, i32::MAX as i64),
    ] {
        let n = number(d, k)?;
        if n < min || n > max {
            return Err(AppError::validation(format!("Invalid {k}.")));
        }
    }
    if !d["show_in_day"].is_boolean() {
        return Err(AppError::validation("Choose planner visibility."));
    }
    for k in ["agenda_md", "notes_md"] {
        text(d, k)?;
    }
    if !d["link_url"].is_null() {
        let u = text(d, "link_url")?.trim();
        if u.is_empty() {
            out["link_url"] = Value::Null;
        } else {
            let u =
                url::Url::parse(u).map_err(|_| AppError::validation("Enter an HTTP(S) link."))?;
            if !["http", "https"].contains(&u.scheme())
                || u.host_str().is_none()
                || !u.username().is_empty()
                || u.password().is_some()
            {
                return Err(AppError::validation(
                    "Use an HTTP(S) link without credentials.",
                ));
            }
            out["link_url"] = json!(u.as_str());
        }
    }
    let ids = d["task_ids"]
        .as_array()
        .ok_or_else(|| AppError::validation("Invalid linked tasks."))?;
    let mut ids = ids
        .iter()
        .map(|id| {
            id.as_str()
                .map(str::to_owned)
                .ok_or_else(|| AppError::validation("Invalid task ID."))
        })
        .collect::<Result<Vec<_>>>()?;
    ids.sort();
    ids.dedup();
    out["task_ids"] = json!(ids);
    out["starts_at"] = json!(utc(at));
    if series {
        choice(text(d, "repeat_rule")?, &["none", "daily", "weekly"])?;
        let mut weekdays = if let Some(value) = d.get("repeat_weekdays") {
            value
                .as_array()
                .ok_or_else(|| AppError::validation("Choose valid repeat weekdays."))?
                .iter()
                .map(|v| {
                    v.as_i64()
                        .filter(|n| (1..=7).contains(n))
                        .ok_or_else(|| AppError::validation("Choose valid repeat weekdays."))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![wall.weekday().number_from_monday() as i64]
        };
        weekdays.sort();
        weekdays.dedup();
        if d["repeat_rule"] == "weekly" && weekdays.is_empty() {
            return Err(AppError::validation("Choose at least one repeat weekday."));
        }
        // The editor advances the first meeting to a selected weekday. Validate
        // that contract here so following edits cannot introduce an off-day snapshot.
        if d["repeat_rule"] == "weekly"
            && !weekdays.contains(&(wall.weekday().number_from_monday() as i64))
        {
            return Err(AppError::validation(
                "The first meeting date must be a selected repeat weekday.",
            ));
        }
        out["repeat_weekdays"] = if d["repeat_rule"] == "weekly" {
            json!(weekdays)
        } else {
            json!([])
        };
        choice(text(d, "fold_policy")?, &["earlier", "later"])?;
        if let Some(end) = d["repeat_until"].as_str() {
            if date(end)? < wall.date() {
                return Err(AppError::validation(
                    "Repeat end cannot precede the start date.",
                ));
            }
        } else if !d["repeat_until"].is_null() {
            return Err(AppError::validation("Invalid repeat end."));
        }
    }
    Ok(out)
}

// Snapshots deliberately exclude relational task IDs and reject unknown data.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrenceSnapshot {
    pub title: String,
    pub starts_at: String,
    pub start_local: String,
    pub start_offset: String,
    pub time_zone: String,
    pub duration_min: i64,
    pub link_url: Option<String>,
    pub agenda_md: String,
    pub notes_md: String,
    pub reminder_min: i64,
    pub show_in_day: bool,
}
pub fn stored_snapshot(source: &str) -> Result<Value> {
    let value: OccurrenceSnapshot = serde_json::from_str(source)
        .map_err(|_| AppError::validation("Invalid stored occurrence snapshot."))?;
    let mut v = serde_json::to_value(value)
        .map_err(|_| AppError::validation("Invalid stored occurrence snapshot."))?;
    v["task_ids"] = json!([]);
    normalize(&v, false)
}
