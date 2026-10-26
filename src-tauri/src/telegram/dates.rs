use chrono::{Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

/// A local wall-clock time resolved against a timezone.
#[derive(Debug, PartialEq)]
pub enum Local {
    /// The wall time does not exist (spring-forward gap).
    Missing,
    Single(chrono::DateTime<Utc>),
    /// The wall time happens twice (fall-back fold): (earlier, later).
    Ambiguous(chrono::DateTime<Utc>, chrono::DateTime<Utc>),
}

pub fn resolve(tz: Tz, date: NaiveDate, time: NaiveTime) -> Local {
    match tz.from_local_datetime(&date.and_time(time)) {
        LocalResult::None => Local::Missing,
        LocalResult::Single(v) => Local::Single(v.with_timezone(&Utc)),
        LocalResult::Ambiguous(a, b) => {
            let (a, b) = (a.with_timezone(&Utc), b.with_timezone(&Utc));
            Local::Ambiguous(a.min(b), a.max(b))
        }
    }
}

pub fn parse_date_iso(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

pub fn parse_time_24(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
}

/// "3pm", "3:30 pm", "15:00", "noon". Returns None for anything else.
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let t = text.trim().to_lowercase().replace('.', "");
    match t.as_str() {
        "noon" => return NaiveTime::from_hms_opt(12, 0, 0),
        "midnight" => return NaiveTime::from_hms_opt(0, 0, 0),
        _ => {}
    }
    let (digits, meridiem) = if let Some(v) = t.strip_suffix("pm") {
        (v.trim(), Some(true))
    } else if let Some(v) = t.strip_suffix("am") {
        (v.trim(), Some(false))
    } else {
        (t.as_str(), None)
    };
    let (h, m) = match digits.split_once(':') {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        None => (digits.parse::<u32>().ok()?, 0),
    };
    if m > 59 {
        return None;
    }
    let h = match meridiem {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            (h % 12) + if pm { 12 } else { 0 }
        }
        None => {
            // A bare "3" is too ambiguous to guess; require a colon form.
            if !digits.contains(':') || h > 23 {
                return None;
            }
            h
        }
    };
    NaiveTime::from_hms_opt(h, m, 0)
}

fn weekday(name: &str) -> Option<chrono::Weekday> {
    use chrono::Weekday::*;
    Some(match name {
        "mon" | "monday" => Mon,
        "tue" | "tues" | "tuesday" => Tue,
        "wed" | "wednesday" => Wed,
        "thu" | "thur" | "thurs" | "thursday" => Thu,
        "fri" | "friday" => Fri,
        "sat" | "saturday" => Sat,
        "sun" | "sunday" => Sun,
        _ => return None,
    })
}

/// ISO dates, "today", "tomorrow", or a weekday name (the next one after today).
pub fn parse_date(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let t = text.trim().to_lowercase();
    if let Some(d) = parse_date_iso(&t) {
        return Some(d);
    }
    match t.as_str() {
        "today" => return Some(today),
        "tomorrow" => return Some(today + Duration::days(1)),
        _ => {}
    }
    let name = t.strip_prefix("next ").unwrap_or(&t);
    let target = weekday(name)?;
    let ahead = (target.num_days_from_monday() as i64
        - today.weekday().num_days_from_monday() as i64)
        .rem_euclid(7);
    Some(today + Duration::days(if ahead == 0 { 7 } else { ahead }))
}

pub fn hhmm(time: NaiveTime) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_parse_and_bare_numbers_are_rejected() {
        let t = |s: &str| parse_time(s).map(hhmm);
        assert_eq!(t("3pm").as_deref(), Some("15:00"));
        assert_eq!(t("3:30 PM").as_deref(), Some("15:30"));
        assert_eq!(t("12am").as_deref(), Some("00:00"));
        assert_eq!(t("12 pm").as_deref(), Some("12:00"));
        assert_eq!(t("15:45").as_deref(), Some("15:45"));
        assert_eq!(t("noon").as_deref(), Some("12:00"));
        assert_eq!(t("3"), None);
        assert_eq!(t("13pm"), None);
        assert_eq!(t("9:75"), None);
    }

    #[test]
    fn dates_resolve_relative_to_today() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(); // Sunday
        assert_eq!(
            parse_date("tomorrow", today),
            NaiveDate::from_ymd_opt(2026, 10, 26)
        );
        assert_eq!(
            parse_date("Friday", today),
            NaiveDate::from_ymd_opt(2026, 10, 30)
        );
        assert_eq!(
            parse_date("sunday", today),
            NaiveDate::from_ymd_opt(2026, 11, 1)
        );
        assert_eq!(
            parse_date("2027-01-02", today),
            NaiveDate::from_ymd_opt(2027, 1, 2)
        );
        assert_eq!(parse_date("someday", today), None);
    }

    #[test]
    fn daylight_saving_gaps_and_folds_are_detected() {
        let ny: Tz = "America/New_York".parse().unwrap();
        let t = NaiveTime::from_hms_opt(2, 30, 0).unwrap();
        assert_eq!(
            resolve(ny, NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(), t),
            Local::Missing
        );
        let t = NaiveTime::from_hms_opt(1, 30, 0).unwrap();
        assert!(matches!(
            resolve(ny, NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(), t),
            Local::Ambiguous(a, b) if a < b
        ));
        assert!(matches!(
            resolve(ny, NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(), t),
            Local::Single(_)
        ));
    }
}
