use crate::workspace::models::*;
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, Offset, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerQuery {
    pub date: String,
    pub time_zone: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerInput {
    pub request_id: String,
    pub query: PlannerQuery,
    pub action: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_fingerprint: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockDraft {
    pub kind: String,
    pub task_id: Option<String>,
    pub start_min: i64,
    pub end_min: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}
pub fn calendar(q: &PlannerQuery) -> Result<(NaiveDate, Tz)> {
    let date = NaiveDate::parse_from_str(&q.date, "%Y-%m-%d")
        .map_err(|_| AppError::validation("Enter a valid calendar date."))?;
    if date.format("%Y-%m-%d").to_string() != q.date {
        return Err(AppError::validation("Use YYYY-MM-DD."));
    }
    let zone = q
        .time_zone
        .parse::<Tz>()
        .map_err(|_| AppError::validation("Choose a valid IANA timezone."))?;
    Ok((date, zone))
}
pub fn endpoint(
    date: NaiveDate,
    minute: i64,
    zone: Tz,
    selected: Option<&str>,
) -> Result<(String, DateTime<Tz>)> {
    if !(0..=1440).contains(&minute) {
        return Err(AppError::validation("Time must be within this day."));
    }
    let wall = date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .checked_add_signed(Duration::minutes(minute))
        .ok_or_else(|| AppError::validation("Date overflow."))?;
    let choices = match zone.from_local_datetime(&wall) {
        LocalResult::None => {
            return Err(AppError::validation(
                "This local time does not exist because clocks move forward.",
            ))
        }
        LocalResult::Single(v) => vec![v],
        LocalResult::Ambiguous(a, b) => vec![a, b],
    };
    let found = choices
        .iter()
        .find(|v| Some(v.offset().fix().to_string().as_str()) == selected);
    let v = if choices.len() > 1 || selected.is_some() {
        found.ok_or_else(|| AppError::validation("Choose the UTC offset for this local time."))?
    } else {
        &choices[0]
    };
    Ok((v.offset().fix().to_string(), *v))
}
pub fn validate_draft(d: &mut BlockDraft, q: &PlannerQuery) -> Result<()> {
    choice(&d.kind, &["task", "break", "lunch", "focus"])?;
    if (d.kind == "task") != d.task_id.is_some()
        || d.start_min < 0
        || d.end_min > 1440
        || d.start_min >= d.end_min
        || d.start_min % 15 != 0
        || d.end_min % 15 != 0
    {
        return Err(AppError::validation(
            "Choose a task or neutral block and a range in 15-minute steps within one day.",
        ));
    }
    let (date, zone) = calendar(q)?;
    let (so, start) = endpoint(date, d.start_min, zone, d.start_offset.as_deref())?;
    let (eo, end) = endpoint(date, d.end_min, zone, d.end_offset.as_deref())?;
    if end <= start {
        return Err(AppError::validation(
            "End must follow start in the selected offsets.",
        ));
    }
    d.start_offset = Some(so);
    d.end_offset = Some(eo);
    Ok(())
}
pub fn instant_in_zone(value: &str, zone: Tz) -> Result<DateTime<Tz>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .map_err(|_| AppError::validation("Invalid stored instant."))?
        .with_timezone(&zone))
}
pub fn bounds(q: &PlannerQuery) -> Result<(DateTime<Tz>, DateTime<Tz>)> {
    let (date, zone) = calendar(q)?;
    // Local midnight can itself be skipped or repeated in some zones. Use the first
    // real minute of that calendar date, matching Temporal's start-of-day behavior.
    let start = |day: NaiveDate| -> Result<DateTime<Tz>> {
        for m in 0..1440 {
            let wall = day.and_hms_opt(0, 0, 0).unwrap() + Duration::minutes(m);
            if let Some(v) = zone.from_local_datetime(&wall).earliest() {
                return Ok(v);
            }
        }
        Err(AppError::validation(
            "This calendar date does not exist in the selected timezone.",
        ))
    };
    Ok((
        start(date)?,
        start(
            date.succ_opt()
                .ok_or_else(|| AppError::validation("Date overflow."))?,
        )?,
    ))
}
pub fn previous_monday(date: NaiveDate) -> NaiveDate {
    let days = date.weekday().num_days_from_monday();
    date - Duration::days(if days == 0 { 7 } else { days as i64 })
}
pub fn floor(q: &PlannerQuery, now: &str) -> Result<i64> {
    let (date, zone) = calendar(q)?;
    let now = instant_in_zone(now, zone)?;
    if date < now.date_naive() {
        return Err(AppError::validation(
            "Automatic planning is unavailable on past dates.",
        ));
    }
    if date > now.date_naive() {
        return Ok(420);
    }
    let ms = (now.hour() as i64 * 3600 + now.minute() as i64 * 60 + now.second() as i64) * 1000
        + now.timestamp_subsec_millis() as i64;
    Ok((ms + 899999) / 900000 * 15)
}
pub fn meeting_range(m: &Value, q: &PlannerQuery) -> Result<Option<(i64, i64)>> {
    let (start, end) = bounds(q)?;
    let (_, zone) = calendar(q)?;
    let from = instant_in_zone(text(m, "starts_at")?, zone)?;
    let to = from
        + Duration::minutes(
            m["duration_min"]
                .as_i64()
                .ok_or_else(|| AppError::validation("Invalid meeting duration."))?,
        );
    if to <= start || from >= end {
        return Ok(None);
    }
    // The fixed 24-hour grid collapses a DST fold. Reserve the complete
    // wall-minute envelope, including both sides of the repeated hour.
    let clipped_start = from.max(start);
    let clipped_end = to.min(end);
    let mut a = 1440;
    let mut b = 0;
    let mut cursor = clipped_start;
    while cursor < clipped_end {
        let minute = cursor.hour() as i64 * 60 + cursor.minute() as i64;
        a = a.min(minute);
        b = b.max(minute + 1);
        cursor += Duration::minutes(1);
    }
    let last = clipped_end - Duration::milliseconds(1);
    let minute = last.hour() as i64 * 60 + last.minute() as i64;
    Ok(Some((a.min(minute), b.max(minute + 1))))
}
pub fn next_free(occupied: &[(i64, i64)], duration: i64, floor: i64) -> Option<(i64, i64)> {
    let mut ranges = occupied.to_vec();
    ranges.sort();
    let mut start = (floor + 14) / 15 * 15;
    for (a, b) in ranges {
        if b <= start {
            continue;
        }
        if start + duration <= a {
            break;
        }
        start = (b + 14) / 15 * 15;
    }
    if duration > 0 && start + duration <= 1440 {
        Some((start, start + duration))
    } else {
        None
    }
}
pub fn utc(v: DateTime<Tz>) -> String {
    v.with_timezone(&Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
