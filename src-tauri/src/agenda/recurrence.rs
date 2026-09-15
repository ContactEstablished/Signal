use super::models::*;
use crate::workspace::models::*;
use chrono::{Datelike, Duration, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::SqliteConnection;
use std::collections::BTreeMap;
pub async fn root(conn: &mut SqliteConnection, id: &str) -> Result<Value> {
    let m=one(conn,"SELECT m.*,p.name AS project_name,p.color AS project_color FROM meetings m JOIN projects p ON p.id=m.project_id WHERE m.id=?",vec![json!(id)]).await?;
    if m["active_version_id"].is_null() {
        return Err(AppError::validation(
            "Meeting migration is not initialized. Reload Signal.",
        ));
    }
    Ok(m)
}
pub async fn segments(conn: &mut SqliteConnection, m: &Value) -> Result<Vec<Value>> {
    rows(
        conn,
        "SELECT * FROM meeting_segments WHERE version_id=? ORDER BY from_ordinal",
        vec![m["active_version_id"].clone()],
    )
    .await
}
pub fn weekdays(s: &Value) -> Result<Vec<i64>> {
    if s["repeat_rule"] != "weekly" {
        return Ok(vec![]);
    }
    let anchor = anchor(s)?;
    if s["repeat_weekdays"].is_null() {
        return Ok(vec![anchor.weekday().number_from_monday() as i64]);
    }
    let mut days: Vec<i64> = serde_json::from_str(text(s, "repeat_weekdays")?)
        .map_err(|_| AppError::validation("Invalid stored repeat weekdays."))?;
    if days.is_empty() || days.iter().any(|d| !(1..=7).contains(d)) {
        return Err(AppError::validation("Invalid stored repeat weekdays."));
    }
    days.sort();
    days.dedup();
    Ok(days)
}
fn anchor(s: &Value) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text(s, "anchor_local")?, "%Y-%m-%dT%H:%M:%S%.f")
        .map_err(|_| AppError::validation("Invalid stored meeting anchor."))
}
// Ordinals count scheduled occurrences, including DST gaps. A missing local
// time therefore never shifts notes, cancellations, or later occurrence IDs.
fn cadence(s: &Value) -> Result<(i64, Vec<i64>)> {
    if s["repeat_rule"] != "weekly" {
        return Ok((1, vec![0]));
    }
    let weekday = anchor(s)?.weekday().number_from_monday() as i64;
    let mut offsets: Vec<_> = weekdays(s)?
        .iter()
        .map(|d| (d - weekday).rem_euclid(7))
        .collect();
    offsets.sort();
    Ok((7, offsets))
}
pub fn local_at(s: &Value, index: i64) -> Result<NaiveDateTime> {
    let (period, offsets) = cadence(s)?;
    let count = offsets.len() as i64;
    let days = index
        .checked_div(count)
        .and_then(|v| v.checked_mul(period))
        .and_then(|v| v.checked_add(offsets[index.rem_euclid(count) as usize]))
        .ok_or_else(|| AppError::validation("Occurrence outside supported calendar."))?;
    let wall = anchor(s)?;
    Duration::try_days(days)
        .and_then(|d| wall.checked_add_signed(d))
        .ok_or_else(|| AppError::validation("Occurrence outside supported calendar."))
}
// Conservative cycle bounds keep day/week queries bounded even decades after
// the anchor; callers resolve and filter actual timestamps and segment ends.
pub fn cycle_bounds(s: &Value, first_day: i64, last_day: i64) -> Result<(i64, i64)> {
    let (period, offsets) = cadence(s)?;
    let count = offsets.len() as i64;
    Ok((
        (first_day.div_euclid(period) * count).max(0),
        (last_day.div_euclid(period) + 1) * count - 1,
    ))
}
pub fn generated(s: &Value, n: i64) -> Result<Option<Value>> {
    let first = number(s, "from_ordinal")?;
    if n < first || s["to_ordinal"].as_i64().is_some_and(|end| n >= end) {
        return Ok(None);
    }
    let repeat = text(s, "repeat_rule")?;
    if repeat == "none" && n != first {
        return Ok(None);
    }
    let wall = local_at(s, n - first)?;
    if s["repeat_until"]
        .as_str()
        .is_some_and(|end| wall.date().to_string().as_str() > end)
    {
        return Ok(None);
    }
    let tz = zone(text(s, "time_zone")?)?;
    let at = if n == first {
        parse(text(s, "anchor_utc")?)?
    } else {
        match tz.from_local_datetime(&wall) {
            LocalResult::None => return Ok(None),
            LocalResult::Single(v) => v.with_timezone(&Utc),
            LocalResult::Ambiguous(a, b) => if s["fold_policy"] == "later" {
                a.max(b)
            } else {
                a.min(b)
            }
            .with_timezone(&Utc),
        }
    };
    let mut out = start_fields(&utc(at), text(s, "time_zone")?)?;
    for k in [
        "title",
        "duration_min",
        "link_url",
        "agenda_md",
        "notes_md",
        "reminder_min",
        "repeat_rule",
    ] {
        out[k] = s[k].clone();
    }
    out["show_in_day"] = json!(s["show_in_day"] == 1);
    Ok(Some(out))
}
pub async fn resolve(conn: &mut SqliteConnection, m: &Value, n: i64) -> Result<Option<Value>> {
    if one(
        conn,
        "SELECT MIN(from_ordinal) AS cutoff FROM meeting_cutoffs WHERE meeting_id=?",
        vec![m["id"].clone()],
    )
    .await?["cutoff"]
        .as_i64()
        .is_some_and(|end| n >= end)
    {
        return Ok(None);
    }
    let stored = rows(
        conn,
        "SELECT * FROM meeting_occurrences WHERE meeting_id=? AND ordinal=?",
        vec![m["id"].clone(), json!(n)],
    )
    .await?
    .pop();
    if stored.as_ref().is_some_and(|v| v["cancelled"] == 1) {
        return Ok(None);
    }
    let ss = segments(conn, m).await?;
    let segment = ss.iter().rev().find(|s| {
        s["from_ordinal"].as_i64().unwrap_or(i64::MAX) <= n
            && s["to_ordinal"].as_i64().is_none_or(|e| n < e)
    });
    let snapshot = stored.as_ref().and_then(|v| v["snapshot_json"].as_str());
    let mut out = if let Some(s) = snapshot {
        stored_snapshot(s)?
    } else if let Some(s) = segment {
        let Some(v) = generated(s, n)? else {
            return Ok(None);
        };
        v
    } else {
        return Ok(None);
    };
    let links = if snapshot.is_some() {
        rows(conn,"SELECT task_id FROM meeting_occurrence_tasks WHERE meeting_id=? AND occurrence_key=? ORDER BY task_id",vec![m["id"].clone(),json!(key(n))]).await?
    } else {
        rows(
            conn,
            "SELECT task_id FROM meeting_segment_tasks WHERE segment_id=? ORDER BY task_id",
            vec![segment.unwrap()["id"].clone()],
        )
        .await?
    };
    out["task_ids"] = json!(links
        .iter()
        .map(|v| v["task_id"].clone())
        .collect::<Vec<_>>());
    out["ref"] = json!({"meetingId":m["id"],"occurrenceKey":key(n)});
    out["id"] = json!(hash(&out["ref"]));
    for k in ["project_id", "project_name", "project_color", "revision"] {
        out[k] = m[k].clone();
    }
    if out["repeat_rule"].is_null() {
        out["repeat_rule"] = segment
            .map(|s| s["repeat_rule"].clone())
            .unwrap_or(json!("none"));
    }
    out["attendance"] = stored
        .as_ref()
        .map(|s| s["attendance"].clone())
        .unwrap_or(json!("unmarked"));
    out["occurrence_notes_md"] = stored
        .as_ref()
        .map(|s| s["occurrence_notes_md"].clone())
        .unwrap_or(json!(""));
    Ok(Some(out))
}
pub async fn project(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
    project: Option<&str>,
    visible: bool,
) -> Result<(Vec<Value>, Vec<String>)> {
    let begin = parse(start)?;
    let finish = parse(end)?;
    if finish <= begin || finish - begin > Duration::days(8) {
        return Err(AppError::validation(
            "Meeting queries require a bounded day or week.",
        ));
    }
    let roots = rows(
        conn,
        "SELECT id FROM meetings WHERE (? IS NULL OR project_id=?) ORDER BY id",
        vec![json!(project), json!(project)],
    )
    .await?;
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for r in roots {
        let m = root(conn, text(&r, "id")?).await?;
        let mut candidates = BTreeMap::<i64, ()>::new();
        for s in segments(conn, &m).await? {
            let anchor =
                NaiveDateTime::parse_from_str(text(&s, "anchor_local")?, "%Y-%m-%dT%H:%M:%S%.f")
                    .map_err(|_| AppError::validation("Invalid meeting anchor."))?;
            let tz = zone(text(&s, "time_zone")?)?;
            let from = number(&s, "from_ordinal")?;
            let (lo, hi) = cycle_bounds(
                &s,
                ((begin - Duration::days(1)).with_timezone(&tz).date_naive() - anchor.date())
                    .num_days(),
                ((finish + Duration::days(1)).with_timezone(&tz).date_naive() - anchor.date())
                    .num_days(),
            )?;
            for i in lo..=hi {
                let n = from + i;
                if s["repeat_rule"] == "none" && i != 0 {
                    continue;
                }
                if s["to_ordinal"].as_i64().is_some_and(|e| n >= e) {
                    continue;
                }
                candidates.insert(n, ());
                if generated(&s, n)?.is_none() {
                    let local = local_at(&s, i)?;
                    if s["repeat_until"]
                        .as_str()
                        .is_none_or(|e| local.date().to_string().as_str() <= e)
                        && local.date() >= begin.with_timezone(&tz).date_naive()
                        && local.date()
                            <= (finish - Duration::milliseconds(1))
                                .with_timezone(&tz)
                                .date_naive()
                        && tz.from_local_datetime(&local) == LocalResult::None
                    {
                        skipped.push(format!(
                            "{} · {} skipped: this time does not exist in {}",
                            text(&m, "title")?,
                            local.date(),
                            tz
                        ));
                    }
                }
            }
        }
        for s in rows(conn,"SELECT ordinal FROM meeting_occurrences WHERE meeting_id=? AND snapshot_json IS NOT NULL AND actual_starts_at<? AND julianday(actual_starts_at)+json_extract(snapshot_json,'$.duration_min')/1440.0>julianday(?)",vec![m["id"].clone(),json!(end),json!(start)]).await? {candidates.insert(number(&s,"ordinal")?,());}
        for n in candidates.keys() {
            if let Some(v) = resolve(conn, &m, *n).await? {
                let at = parse(text(&v, "starts_at")?)?;
                if at < finish
                    && at + Duration::minutes(number(&v, "duration_min")?) > begin
                    && (!visible || v["show_in_day"] == true)
                {
                    out.push(v);
                    if out.len() > 10_000 {
                        return Err(AppError::validation("More than 10,000 meeting occurrences in this range. Narrow the project scope."));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a["starts_at"]
            .as_str()
            .cmp(&b["starts_at"].as_str())
            .then(a["id"].as_str().cmp(&b["id"].as_str()))
    });
    Ok((out, skipped))
}
