use super::google::*;
use super::sync::*;
use super::*;
use serde_json::{json, Value};
use sqlx::SqlitePool;

const NOW: &str = "2026-11-18T18:00:00.000Z";
const FROM: &str = "2026-11-11T18:00:00.000Z";
const TO: &str = "2027-01-17T18:00:00.000Z";

async fn db() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("calendar.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    sqlx::query("INSERT INTO projects(id,name,color,sort_order) VALUES('p','Work','cyan',0)")
        .execute(&p)
        .await
        .unwrap();
    (dir, p)
}

fn target() -> Target<'static> {
    Target {
        account_id: "a",
        calendar_id: "primary",
        project_id: "p",
        time_zone: "America/New_York",
        window_start: FROM,
        window_end: TO,
        now: NOW,
    }
}

fn event(id: &str, title: &str, at: &str) -> Event {
    Event {
        id: id.into(),
        title: title.into(),
        starts_at: at.into(),
        duration_min: 30,
        link_url: Some("https://meet.google.com/abc-defg-hij".into()),
        agenda_md: "Discuss the plan".into(),
        ..Event::default()
    }
}

async fn run(p: &SqlitePool, events: &[Event]) -> Outcome {
    let mut tx = p.begin().await.unwrap();
    let o = reconcile(&mut tx, &target(), events).await.unwrap();
    tx.commit().await.unwrap();
    o
}

async fn meeting_for(p: &SqlitePool, event_id: &str) -> Option<Value> {
    let mut c = p.acquire().await.unwrap();
    let l = rows(
        &mut c,
        "SELECT meeting_id FROM calendar_links WHERE event_id=?",
        vec![json!(event_id)],
    )
    .await
    .unwrap()
    .pop()?;
    let id = l["meeting_id"].as_str()?.to_string();
    let r = crate::agenda::models::MeetingRef {
        meeting_id: id,
        occurrence_key: "o:0".into(),
    };
    crate::agenda::meeting_detail(&mut c, &r).await.ok()
}

async fn count(p: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(p).await.unwrap()
}

async fn record_notes(p: &SqlitePool, detail: &Value, notes: &str) {
    crate::agenda::apply_into(
        p,
        crate::agenda::models::AgendaInput {
            request_id: uid(),
            change: json!({"action":"record","payload":{"ref":detail["occurrence"]["ref"],"expectedRevision":detail["series"]["revision"],"attendance":"unmarked","occurrence_notes_md":notes}}),
            expected_fingerprint: None,
        },
        NOW,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn new_events_become_meetings_and_a_repeat_sync_changes_nothing() {
    let (_d, p) = db().await;
    let events = [event("e1", "Client call", "2026-11-19T15:00:00.000Z")];
    assert_eq!(run(&p, &events).await.created, 1);
    let m = meeting_for(&p, "e1").await.unwrap();
    let o = &m["occurrence"];
    assert_eq!(o["title"], "Client call");
    assert_eq!(o["project_id"], "p");
    assert_eq!(o["starts_at"], "2026-11-19T15:00:00.000Z");
    assert_eq!(o["start_local"], "2026-11-19T10:00:00.000");
    assert_eq!(o["link_url"], "https://meet.google.com/abc-defg-hij");
    assert_eq!(o["agenda_md"], "Discuss the plan");
    assert_eq!(o["duration_min"], 30);
    assert_eq!(m["source"]["kind"], "google");
    assert_eq!(run(&p, &events).await, Outcome::default());
    assert_eq!(count(&p, "SELECT COUNT(*) FROM meetings").await, 1);
}

#[tokio::test]
async fn changes_at_google_update_the_meeting_and_keep_signal_notes() {
    let (_d, p) = db().await;
    run(
        &p,
        &[event("e1", "Client call", "2026-11-19T15:00:00.000Z")],
    )
    .await;
    let m = meeting_for(&p, "e1").await.unwrap();
    record_notes(&p, &m, "Bring the contract").await;
    let mut moved = event("e1", "Client call (moved)", "2026-11-20T16:30:00.000Z");
    moved.duration_min = 45;
    assert_eq!(run(&p, &[moved]).await.updated, 1);
    let o = meeting_for(&p, "e1").await.unwrap()["occurrence"].clone();
    assert_eq!(o["title"], "Client call (moved)");
    assert_eq!(o["starts_at"], "2026-11-20T16:30:00.000Z");
    assert_eq!(o["duration_min"], 45);
    assert_eq!(o["occurrence_notes_md"], "Bring the contract");
    // The moved meeting shows on its new day.
    let mut c = p.acquire().await.unwrap();
    let day = crate::agenda::occurrences(
        &mut c,
        "2026-11-20T05:00:00.000Z",
        "2026-11-21T05:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(day.len(), 1);
}

#[tokio::test]
async fn deleted_untouched_events_are_removed() {
    let (_d, p) = db().await;
    run(
        &p,
        &[
            event("e1", "A", "2026-11-19T15:00:00.000Z"),
            event("e2", "B", "2026-11-19T17:00:00.000Z"),
        ],
    )
    .await;
    // e1 is gone from the list; e2 is reported cancelled.
    let mut gone = event("e2", "", "");
    gone.cancelled = true;
    let o = run(&p, &[gone]).await;
    assert_eq!(o.removed, 2);
    assert_eq!(count(&p, "SELECT COUNT(*) FROM meetings").await, 0);
    assert_eq!(count(&p, "SELECT COUNT(*) FROM calendar_links").await, 0);
}

#[tokio::test]
async fn cancelled_meetings_with_notes_stay_marked_and_come_back_if_restored() {
    let (_d, p) = db().await;
    let e = event("e1", "Review", "2026-11-19T15:00:00.000Z");
    run(&p, &[e.clone()]).await;
    let m = meeting_for(&p, "e1").await.unwrap();
    record_notes(&p, &m, "Prep notes").await;
    assert_eq!(run(&p, &[]).await.canceled, 1);
    let o = meeting_for(&p, "e1").await.unwrap()["occurrence"].clone();
    assert_eq!(o["attendance"], "canceled");
    assert_eq!(o["occurrence_notes_md"], "Prep notes");
    // A second sync leaves it alone.
    assert_eq!(run(&p, &[]).await, Outcome::default());
    // Restored at Google: the cancellation is undone, notes kept.
    assert_eq!(run(&p, &[e]).await.updated, 1);
    let o = meeting_for(&p, "e1").await.unwrap()["occurrence"].clone();
    assert_eq!(o["attendance"], "unmarked");
    assert_eq!(o["occurrence_notes_md"], "Prep notes");
}

#[tokio::test]
async fn meetings_deleted_in_signal_are_not_imported_again() {
    let (_d, p) = db().await;
    let e = event("e1", "Standup", "2026-11-19T15:00:00.000Z");
    run(&p, &[e.clone()]).await;
    sqlx::query("DELETE FROM meetings")
        .execute(&p)
        .await
        .unwrap();
    assert_eq!(
        count(
            &p,
            "SELECT COUNT(*) FROM calendar_links WHERE meeting_id IS NULL"
        )
        .await,
        1
    );
    assert_eq!(run(&p, &[e]).await, Outcome::default());
    assert_eq!(count(&p, "SELECT COUNT(*) FROM meetings").await, 0);
}

#[tokio::test]
async fn declined_and_all_day_events_are_not_meetings() {
    let (_d, p) = db().await;
    let e = event("e1", "Offsite", "2026-11-19T15:00:00.000Z");
    run(&p, &[e.clone()]).await;
    let mut declined = e;
    declined.skip = true;
    assert_eq!(run(&p, &[declined]).await.removed, 1);
    assert_eq!(count(&p, "SELECT COUNT(*) FROM meetings").await, 0);
}

#[tokio::test]
async fn stopping_a_calendar_keeps_its_meetings() {
    let (_d, p) = db().await;
    run(&p, &[event("e1", "A", "2026-11-19T15:00:00.000Z")]).await;
    let mut c = p.acquire().await.unwrap();
    unlink(&mut c, "a", Some("primary")).await.unwrap();
    drop(c);
    assert_eq!(count(&p, "SELECT COUNT(*) FROM meetings").await, 1);
    assert_eq!(count(&p, "SELECT COUNT(*) FROM calendar_links").await, 0);
}

#[test]
fn reads_only_desktop_client_files() {
    let ok = r#"{"installed":{"client_id":"123-abc.apps.googleusercontent.com","client_secret":"s3cret","redirect_uris":["http://localhost"]}}"#;
    assert_eq!(
        parse_client(ok).unwrap(),
        Client {
            id: "123-abc.apps.googleusercontent.com".into(),
            secret: "s3cret".into()
        }
    );
    assert!(parse_client(
        r#"{"web":{"client_id":"x.apps.googleusercontent.com","client_secret":"s"}}"#
    )
    .is_err());
    assert!(parse_client(r#"{"installed":{"client_id":"x","client_secret":"s"}}"#).is_err());
    assert!(parse_client("not json").is_err());
}

#[test]
fn pkce_matches_the_rfc_example_and_url_carries_it() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let v = verifier();
    assert_eq!(v.len(), 64);
    let u = url::Url::parse(&auth_url("cid", "http://127.0.0.1:5000", "st", "ch", None)).unwrap();
    let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["code_challenge"], "ch");
    assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
    assert_eq!(q["access_type"], "offline");
    assert!(q["scope"].contains("calendar.readonly"));
    // Email access is read-only: Signal never sends, labels or deletes mail.
    assert!(q["scope"].contains("gmail.readonly"));
    assert!(!q["scope"].contains("gmail.modify") && !q["scope"].contains("gmail.send") && !q["scope"].contains("mail.google.com"));
}

#[test]
fn redirect_checks_state_and_reports_cancellation() {
    assert_eq!(
        redirect_result("GET /?state=s&code=abc HTTP/1.1", "s")
            .unwrap()
            .unwrap(),
        "abc"
    );
    assert!(redirect_result("GET /?state=other&code=abc HTTP/1.1", "s")
        .unwrap()
        .is_err());
    let denied = redirect_result("GET /?state=s&error=access_denied HTTP/1.1", "s")
        .unwrap()
        .unwrap_err();
    assert!(denied.message.contains("cancelled"));
    assert!(redirect_result("GET /favicon.ico HTTP/1.1", "s").is_none());
}

#[test]
fn id_token_names_the_account() {
    use base64::Engine;
    let body = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(br#"{"email":"Matt@Example.com","sub":"1"}"#);
    assert_eq!(
        email_from_id_token(&format!("h.{body}.sig")).unwrap(),
        "matt@example.com"
    );
    assert!(email_from_id_token("nonsense").is_err());
}

#[test]
fn google_events_are_read_into_meetings() {
    let e = parse_event(&json!({
        "id":"e1","status":"confirmed","summary":"  Design review ",
        "start":{"dateTime":"2026-11-19T10:00:00-05:00","timeZone":"America/New_York"},
        "end":{"dateTime":"2026-11-19T11:15:00-05:00"},
        "location":"Room 4",
        "hangoutLink":"https://meet.google.com/abc",
        "description":"<p>Agenda:</p><ul><li>Scope</li><li>Dates &amp; cost</li></ul>"
    }))
    .unwrap();
    assert!(!e.skip && !e.cancelled);
    assert_eq!(e.title, "Design review");
    assert_eq!(e.starts_at, "2026-11-19T15:00:00.000Z");
    assert_eq!(e.duration_min, 75);
    assert_eq!(e.link_url.as_deref(), Some("https://meet.google.com/abc"));
    assert_eq!(
        e.agenda_md,
        "Location: Room 4\n\nAgenda:\n\n- Scope\n- Dates & cost"
    );

    let all_day =
        parse_event(&json!({"id":"d","start":{"date":"2026-11-19"},"end":{"date":"2026-11-20"}}))
            .unwrap();
    assert!(all_day.skip);
    let declined = parse_event(&json!({"id":"x","start":{"dateTime":"2026-11-19T10:00:00Z"},"end":{"dateTime":"2026-11-19T11:00:00Z"},"attendees":[{"email":"me@x.com","self":true,"responseStatus":"declined"}]})).unwrap();
    assert!(declined.skip);
    let ooo = parse_event(&json!({"id":"o","eventType":"outOfOffice","start":{"dateTime":"2026-11-19T10:00:00Z"},"end":{"dateTime":"2026-11-19T11:00:00Z"}})).unwrap();
    assert!(ooo.skip);
    let gone = parse_event(&json!({"id":"g","status":"cancelled"})).unwrap();
    assert!(gone.cancelled);
    let untitled = parse_event(&json!({"id":"u","start":{"dateTime":"2026-11-19T10:00:00Z"},"end":{"dateTime":"2026-11-19T10:30:00Z"},"location":"https://zoom.us/j/1 (passcode in invite)"})).unwrap();
    assert_eq!(untitled.title, "(No title)");
    assert_eq!(untitled.link_url.as_deref(), Some("https://zoom.us/j/1"));
}

#[test]
fn calendar_list_refresh_keeps_chosen_projects() {
    let old = vec![Calendar {
        id: "work".into(),
        name: "Work".into(),
        color: "#000".into(),
        primary: false,
        project_id: Some("p".into()),
    }];
    let fresh = parse_calendars(&json!({"items":[
        {"id":"me@x.com","summary":"me@x.com","primary":true,"backgroundColor":"#9fc6e7"},
        {"id":"work","summary":"Work","summaryOverride":"Work (shared)"},
        {"id":"old","summary":"Old","deleted":true}
    ]}));
    let merged = merge_calendars(&old, fresh);
    assert_eq!(merged.len(), 2);
    assert!(merged[0].primary && merged[0].project_id.is_none());
    assert_eq!(merged[1].name, "Work (shared)");
    assert_eq!(merged[1].project_id.as_deref(), Some("p"));
}
