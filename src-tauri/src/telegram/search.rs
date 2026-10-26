use crate::agenda::{bounds, occurrences};
use crate::workspace::models::*;
use chrono::{Duration, NaiveDate};
use serde_json::{json, Value};
use sqlx::SqliteConnection;

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
}

pub async fn projects(conn: &mut SqliteConnection) -> Result<Vec<Project>> {
    Ok(rows(
        conn,
        "SELECT id,name FROM projects ORDER BY sort_order,name,id",
        vec![],
    )
    .await?
    .into_iter()
    .map(|r| Project {
        id: r["id"].as_str().unwrap_or_default().to_owned(),
        name: r["name"].as_str().unwrap_or_default().to_owned(),
    })
    .collect())
}

#[derive(Debug, PartialEq)]
pub enum Pick {
    One(usize),
    None,
    Many,
}

/// Exact (case-insensitive) name first, then a unique substring match in either direction.
pub fn match_project(projects: &[Project], name: &str) -> Pick {
    let wanted = name.trim().to_lowercase();
    if wanted.is_empty() {
        return Pick::None;
    }
    if let Some(i) = projects
        .iter()
        .position(|p| p.name.to_lowercase() == wanted)
    {
        return Pick::One(i);
    }
    let hits: Vec<usize> = projects
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let have = p.name.to_lowercase();
            have.contains(&wanted) || wanted.contains(&have)
        })
        .map(|(i, _)| i)
        .collect();
    match hits.len() {
        0 => Pick::None,
        1 => Pick::One(hits[0]),
        _ => Pick::Many,
    }
}

const STOP: &[&str] = &[
    "the", "a", "an", "my", "our", "task", "tasks", "meeting", "meetings", "for", "to", "of",
    "with", "and", "on", "in", "call",
];

pub fn tokens(query: &str) -> Vec<String> {
    let all: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    let useful: Vec<String> = all
        .iter()
        .filter(|t| !STOP.contains(&t.as_str()))
        .cloned()
        .collect();
    if useful.is_empty() {
        all
    } else {
        useful
    }
}

fn score(title: &str, project: &str, query: &str, toks: &[String]) -> Option<i32> {
    if toks.is_empty() {
        return None;
    }
    let title_l = title.to_lowercase();
    let hay = format!("{title_l} {}", project.to_lowercase());
    if !toks.iter().all(|t| hay.contains(t.as_str())) {
        return None;
    }
    let q = query.trim().to_lowercase();
    Some(if title_l == q {
        3
    } else if title_l.contains(&q) {
        2
    } else {
        1
    })
}

/// Stored tasks whose title or project contains every meaningful word of the query.
pub async fn find_tasks(
    conn: &mut SqliteConnection,
    query: &str,
    include_done: bool,
) -> Result<Vec<Value>> {
    let toks = tokens(query);
    let all = rows(
        conn,
        "SELECT t.id,t.title,t.status,t.due_at,t.revision,t.notes_md,t.project_id,p.name AS project_name FROM tasks t JOIN projects p ON p.id=t.project_id WHERE (?=1 OR t.status!='done') ORDER BY p.name,t.title,t.id",
        vec![json!(include_done as i64)],
    )
    .await?;
    let mut hits: Vec<(i32, Value)> = all
        .into_iter()
        .filter_map(|t| {
            score(
                t["title"].as_str().unwrap_or(""),
                t["project_name"].as_str().unwrap_or(""),
                query,
                &toks,
            )
            .map(|s| (s, t))
        })
        .collect();
    hits.sort_by_key(|(s, _)| -*s);
    Ok(hits.into_iter().map(|(_, t)| t).take(50).collect())
}

pub async fn task(conn: &mut SqliteConnection, id: &str) -> Result<Value> {
    one(
        conn,
        "SELECT t.id,t.title,t.status,t.due_at,t.revision,t.notes_md,t.project_id,p.name AS project_name FROM tasks t JOIN projects p ON p.id=t.project_id WHERE t.id=?",
        vec![json!(id)],
    )
    .await
}

/// Meeting occurrences in `[from, from + days)` (local dates) matching the query.
pub async fn find_meetings(
    conn: &mut SqliteConnection,
    query: &str,
    from: NaiveDate,
    days: i64,
    tz: &str,
) -> Result<Vec<Value>> {
    let start = bounds(from, tz)?.0;
    let end = bounds(from + Duration::days(days - 1), tz)?.1;
    let toks = tokens(query);
    let mut hits: Vec<(i32, Value)> = occurrences(conn, &start, &end, None, false)
        .await?
        .into_iter()
        .filter_map(|m| {
            score(
                m["title"].as_str().unwrap_or(""),
                m["project_name"].as_str().unwrap_or(""),
                query,
                &toks,
            )
            .map(|s| (s, m))
        })
        .collect();
    hits.sort_by(|a, b| {
        a.1["starts_at"]
            .as_str()
            .cmp(&b.1["starts_at"].as_str())
            .then(b.0.cmp(&a.0))
    });
    Ok(hits.into_iter().map(|(_, m)| m).take(50).collect())
}

pub struct DayView {
    pub overdue: Vec<Value>,
    pub tasks: Vec<Value>,
    pub meetings: Vec<Value>,
}

/// Open tasks due on `date`, overdue open tasks when `date` is today, and that day's meetings.
pub async fn day(
    conn: &mut SqliteConnection,
    date: NaiveDate,
    today: NaiveDate,
    tz: &str,
) -> Result<DayView> {
    let (a, b) = bounds(date, tz)?;
    let select = "SELECT t.id,t.title,t.status,t.due_at,p.name AS project_name FROM tasks t JOIN projects p ON p.id=t.project_id";
    let tasks = rows(
        conn,
        &format!("{select} WHERE t.status!='done' AND t.due_at>=? AND t.due_at<? ORDER BY t.due_at,t.title,t.id"),
        vec![json!(a), json!(b)],
    )
    .await?;
    let overdue = if date == today {
        rows(
            conn,
            &format!(
                "{select} WHERE t.status!='done' AND t.due_at<? ORDER BY t.due_at,t.title,t.id"
            ),
            vec![json!(a)],
        )
        .await?
    } else {
        vec![]
    };
    let meetings = occurrences(conn, &a, &b, None, false).await?;
    Ok(DayView {
        overdue,
        tasks,
        meetings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, name: &str) -> Project {
        Project {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn project_names_match_exactly_then_uniquely() {
        let list = [p("1", "Billing"), p("2", "Billing Ops"), p("3", "Website")];
        assert_eq!(match_project(&list, "billing"), Pick::One(0));
        assert_eq!(match_project(&list, "web"), Pick::One(2));
        assert_eq!(match_project(&list, "bill"), Pick::Many);
        assert_eq!(match_project(&list, "garden"), Pick::None);
        assert_eq!(match_project(&list, "  "), Pick::None);
    }

    #[test]
    fn search_words_ignore_filler_and_need_every_meaningful_word() {
        assert_eq!(tokens("the standup meeting"), vec!["standup"]);
        assert_eq!(tokens("the"), vec!["the"]);
        assert!(score("Weekly standup", "Team", "standup", &tokens("standup")).is_some());
        assert!(score(
            "Weekly standup",
            "Team",
            "design review",
            &tokens("design review")
        )
        .is_none());
        assert_eq!(
            score("Standup", "Team", "standup", &tokens("standup")),
            Some(3)
        );
    }
}
