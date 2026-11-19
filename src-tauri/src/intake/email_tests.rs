use super::{data::*, email::*};
use base64::Engine;
use serde_json::{json, Value};

fn b64(s: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}
fn gmail_message(parts: Value) -> Value {
    json!({
        "id": "18c1", "threadId": "18c0", "internalDate": "1795449600000", "snippet": "snippet text",
        "payload": {
            "mimeType": "multipart/alternative",
            "headers": [
                {"name":"From","value":"Dana Lee <dana@acme.com>"},
                {"name":"To","value":"matt@mattwilson.dev"},
                {"name":"Subject","value":"Contract redlines"},
                {"name":"Message-Id","value":"<abc.123@mail.acme.com>"}
            ],
            "parts": parts
        }
    })
}
fn email(i: usize) -> Email {
    Email {
        account_id: "acct".into(),
        mailbox: "matt@mattwilson.dev".into(),
        id: format!("m{i}"),
        thread_id: format!("t{i}"),
        from: "Dana Lee <dana@acme.com>".into(),
        to: "matt@mattwilson.dev".into(),
        subject: format!("Subject {i}"),
        received_at: "2026-11-18T14:00:00.000Z".into(),
        message_id: format!("<id{i}@acme.com>"),
        body: "Please send the invoice by Friday.".into(),
        outlook: false,
    }
}
fn projects() -> Vec<Value> {
    vec![json!({"id":"p-acme","name":"Acme"}), json!({"id":"p-tax","name":"TaxApp"})]
}
fn draft() -> Draft {
    serde_json::from_value(json!({"id":"email","source_kind":"email","revision":0,"recorded_at":"2026-11-18T14:00:00.000Z","time_zone":"America/New_York","transcript":"","projects":[],"candidates":[]})).unwrap()
}

#[test]
fn reads_plain_text_part_and_headers() {
    let m = gmail_message(json!([
        {"mimeType":"text/plain","filename":"","body":{"data":b64("Hi Matt,\r\n\r\nPlease review the redlines by Thursday.\r\n\r\nOn Mon, Nov 16, 2026 Matt wrote:\r\n> earlier text")}},
        {"mimeType":"text/html","filename":"","body":{"data":b64("<p>html version</p>")}}
    ]));
    let e = parse_message(&m, "acct", "matt@mattwilson.dev").unwrap();
    assert_eq!(e.from, "Dana Lee <dana@acme.com>");
    assert_eq!(e.subject, "Contract redlines");
    assert_eq!(e.message_id, "<abc.123@mail.acme.com>");
    assert_eq!(e.thread_id, "18c0");
    assert_eq!(e.received_at, "2026-11-23T16:00:00.000Z");
    assert_eq!(e.body, "Hi Matt,\n\nPlease review the redlines by Thursday.");
}

#[test]
fn falls_back_to_html_then_snippet_and_skips_attachments() {
    let m = gmail_message(json!([
        {"mimeType":"text/plain","filename":"notes.txt","body":{"attachmentId":"a1"}},
        {"mimeType":"text/html","filename":"","body":{"data":b64("<div>Can you <b>sign</b> this?</div>")}}
    ]));
    assert_eq!(
        parse_message(&m, "a", "x@y.com").unwrap().body,
        "Can you sign this?"
    );
    let m = gmail_message(json!([]));
    assert_eq!(parse_message(&m, "a", "x@y.com").unwrap().body, "snippet text");
}

#[test]
fn trims_quoted_replies_and_long_bodies() {
    assert_eq!(
        trim_reply("New ask\n> old line\nmore\n-----Original Message-----\nold"),
        "New ask\nmore"
    );
    assert_eq!(trim_reply(&"x".repeat(9000)).len(), 4000);
}

#[test]
fn links_open_the_message_in_the_right_account() {
    let e = email(1);
    assert_eq!(
        link(&e),
        "https://mail.google.com/mail/?authuser=matt%40mattwilson.dev#search/rfc822msgid%3Aid1%40acme.com"
    );
    let mut e = email(2);
    e.message_id.clear();
    assert!(link(&e).ends_with("#all/t2"));
    assert!(crate::links::validate(&link(&e)).is_ok());
}

#[test]
fn takes_turns_between_inboxes_and_counts_the_rest() {
    let ids = |n: usize, p: &str| (0..n).map(|i| format!("{p}{i}")).collect::<Vec<_>>();
    let (picked, remaining) = pick(vec![(0, ids(20, "a")), (1, ids(2, "b"))]);
    assert_eq!(picked.len(), BATCH);
    assert_eq!(remaining, 22 - BATCH);
    assert_eq!(picked[0], (0, "a0".to_string()));
    assert_eq!(picked[1], (1, "b0".to_string()));
    assert_eq!(picked[3], (1, "b1".to_string()));
    let (picked, remaining) = pick(vec![(0, vec![])]);
    assert!(picked.is_empty());
    assert_eq!(remaining, 0);
}

#[test]
fn prompt_carries_emails_projects_and_time_but_no_ids_beyond_projects() {
    let body = suggestion_body(&[email(0)], &projects(), "model-x", "2026-11-18T19:00:00.000Z", "America/New_York", "Focus on Acme");
    assert_eq!(body["model"], "model-x");
    let user: Value = serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(user["emails"][0]["email_id"], 0);
    assert_eq!(user["emails"][0]["subject"], "Subject 0");
    assert_eq!(user["projects"][1]["name"], "TaxApp");
    assert_eq!(user["instructions"], "Focus on Acme");
    assert_eq!(user["time_zone"], "America/New_York");
    assert!(body["messages"][0]["content"].as_str().unwrap().contains("untrusted"));
}

#[test]
fn ties_tasks_to_their_email_and_drops_unknown_ones() {
    let ai = r#"```json
{"tasks":[
 {"email_id":0,"project_id":"p-acme","project_name":"Acme","title":"Send invoice to Dana","notes":"Dana needs the October invoice.","subtasks":null,"due_at":"2026-11-20T22:00:00Z","priority":"high","estimate_h":0.5,"warning":""},
 {"email_id":1,"project_id":"made-up","project_name":"Brand New Co","title":"Reply to Sam","notes":"","subtasks":[],"due_at":null,"priority":"medium","estimate_h":null,"warning":"Due date inferred"},
 {"email_id":9,"project_id":null,"project_name":"","title":"Ghost","notes":"","subtasks":[],"due_at":null,"priority":"low","estimate_h":null,"warning":""}
]}
```"#;
    let out = attach_sources(ai, &[email(0), email(1)], &projects(), "America/New_York", &[]).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    let tasks = v["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["project_id"], "p-acme");
    assert_eq!(tasks[0]["subtasks"], json!([]));
    assert_eq!(tasks[0]["source"], "Dana Lee · \"Subject 0\" · Nov 18, 9:00 AM · matt@mattwilson.dev");
    assert!(tasks[0]["notes"].as_str().unwrap().starts_with("Dana needs the October invoice.\n\nFrom Dana Lee"));
    assert!(tasks[0]["link_url"].as_str().unwrap().contains("rfc822msgid%3Aid0%40acme.com"));
    // Unknown projects are never proposed from email; the person chooses.
    assert_eq!(tasks[1]["project_id"], Value::Null);
    assert_eq!(tasks[1]["project_name"], "");
}

#[test]
fn appends_to_the_review_and_keeps_existing_suggestions() {
    let mut d = draft();
    let first = attach_sources(r#"{"tasks":[{"email_id":0,"project_id":"p-acme","project_name":"","title":"Send invoice","notes":"","subtasks":[],"due_at":null,"priority":"high","estimate_h":null,"warning":""}]}"#, &[email(0)], &projects(), "America/New_York", &[]).unwrap();
    d = append(d, &projects(), &first).unwrap();
    d.candidates[0].created_id = Some("task-1".into());
    let second = attach_sources(r#"{"tasks":[{"email_id":0,"project_id":null,"project_name":"TaxApp","title":"Review return","notes":"","subtasks":[],"due_at":null,"priority":"medium","estimate_h":null,"warning":""}]}"#, &[email(1)], &projects(), "America/New_York", &[]).unwrap();
    d = append(d, &projects(), &second).unwrap();
    assert_eq!(d.candidates.len(), 2);
    assert_eq!(d.candidates[0].created_id.as_deref(), Some("task-1"));
    assert_eq!(d.candidates[1].project_id.as_deref(), Some("p-tax"));
    assert_eq!(d.candidates[1].kind, "task");
    assert!(d.candidates[1].link_url.as_deref().unwrap().contains("id1%40acme.com"));
    assert!(d.projects.is_empty());
}

#[test]
fn email_reviews_hold_tasks_only_with_web_links() {
    let mut d = draft();
    let t = attach_sources(r#"{"tasks":[{"email_id":0,"project_id":"p-acme","project_name":"","title":"Send invoice","notes":"","subtasks":[],"due_at":null,"priority":"high","estimate_h":null,"warning":""}]}"#, &[email(0)], &projects(), "America/New_York", &[]).unwrap();
    d = append(d, &projects(), &t).unwrap();
    let mut bad = d.clone();
    bad.candidates[0].kind = "meeting".into();
    assert!(bad.validate().is_err());
    let mut bad = d.clone();
    bad.candidates[0].link_url = Some("file:///C:/secret.txt".into());
    assert!(bad.validate().is_err());
}

#[tokio::test]
async fn created_tasks_keep_the_email_link() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&pool).await.unwrap();
    }
    let project = crate::workspace::projects::save(
        &pool,
        None,
        crate::workspace::models::ProjectInput { name: "Acme".into(), color: "cyan".into() },
    )
    .await
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let projects = vec![json!({"id":project,"name":"Acme"})];
    let t = attach_sources(&format!(r#"{{"tasks":[{{"email_id":0,"project_id":"{project}","project_name":"","title":"Send invoice","notes":"Dana asked.","subtasks":[],"due_at":"2026-11-20T22:00:00Z","priority":"high","estimate_h":null,"warning":""}}]}}"#), &[email(0)], &projects, "America/New_York", &[]).unwrap();
    let d = append(draft(), &projects, &t).unwrap();
    let d = save(&pool, d).await.unwrap();
    let ids = vec![d.candidates[0].id.clone()];
    accept(
        &pool,
        Accept { request_id: "r1".into(), draft_id: d.id.clone(), revision: d.revision, candidate_ids: ids },
        "2026-11-18T19:00:00.000Z",
    )
    .await
    .unwrap();
    let (url, priority, due): (String, String, String) =
        sqlx::query_as("SELECT external_url, priority, due_at FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(url.starts_with("https://mail.google.com/mail/?authuser="));
    assert_eq!(priority, "high");
    assert_eq!(due, "2026-11-20T22:00:00.000Z");
}

#[tokio::test]
async fn upgrade_keeps_saved_voice_and_image_reviews() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await.unwrap();
    let all = crate::db::migrations();
    for m in all.iter().filter(|m| m.version < 13) {
        sqlx::raw_sql(m.sql).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql("INSERT INTO intake_drafts(id,revision,payload,updated_at,source_kind) VALUES ('v',3,'{}','2026-11-18','voice'),('i',2,'{}','2026-11-18','image');
        INSERT INTO intake_images(id,draft_id,mime,width,height,bytes) VALUES ('img','i','image/png',10,20,x'89504E47');")
        .execute(&pool).await.unwrap();
    for m in all.iter().filter(|m| m.version >= 13) {
        sqlx::raw_sql(m.sql).execute(&pool).await.unwrap();
    }
    let drafts: Vec<(String, i64)> = sqlx::query_as("SELECT id,revision FROM intake_drafts ORDER BY id").fetch_all(&pool).await.unwrap();
    assert_eq!(drafts, vec![("i".into(), 2), ("v".into(), 3)]);
    let image: (String, i64, Vec<u8>) = sqlx::query_as("SELECT draft_id,height,bytes FROM intake_images").fetch_one(&pool).await.unwrap();
    assert_eq!(image, ("i".into(), 20, vec![0x89, 0x50, 0x4E, 0x47]));
    // The image still belongs to its review: discarding the review removes the picture.
    sqlx::query("DELETE FROM intake_drafts WHERE id='i'").execute(&pool).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM intake_images").fetch_one(&pool).await.unwrap();
    assert_eq!(left, 0);
    // One review per source still holds, and email is now allowed.
    sqlx::query("INSERT INTO intake_drafts(id,revision,payload,updated_at,source_kind) VALUES ('e',0,'{}','x','email')").execute(&pool).await.unwrap();
    assert!(sqlx::query("INSERT INTO intake_drafts(id,revision,payload,updated_at,source_kind) VALUES ('e2',0,'{}','x','email')").execute(&pool).await.is_err());
    assert!(sqlx::query("INSERT INTO intake_drafts(id,revision,payload,updated_at,source_kind) VALUES ('z',0,'{}','x','fax')").execute(&pool).await.is_err());
}

fn sure_task(email_id: usize, project: &str, title: &str) -> Value {
    json!({"email_id":email_id,"project_id":project,"project_name":"","title":title,"notes":"","subtasks":[],"due_at":null,"priority":"high","estimate_h":null,"warning":"","sure":true,"unsure_reason":""})
}

#[test]
fn prompt_asks_whether_each_task_is_sure_without_needing_a_deadline() {
    let body = suggestion_body(&[email(0)], &projects(), "m", "2026-11-18T19:00:00.000Z", "America/New_York", "");
    let system = body["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("\"sure\""));
    assert!(system.contains("A deadline is not required"));
}

#[test]
fn sure_needs_a_person_writing_to_you_about_a_project() {
    let e = email(0);
    assert_eq!(unsure(&sure_task(0, "p-acme", "Send invoice"), &e), None);
    let mut t = sure_task(0, "p-acme", "x");
    t["sure"] = json!(false);
    t["unsure_reason"] = json!("Just an update.");
    assert_eq!(unsure(&t, &e).as_deref(), Some("Just an update"));
    t["unsure_reason"] = json!("");
    assert_eq!(unsure(&t, &e).as_deref(), Some("not clearly a task for you"));
    let mut t = sure_task(0, "p-acme", "x");
    t["project_id"] = Value::Null;
    assert_eq!(unsure(&t, &e).as_deref(), Some("unclear project"));
    let mut robot = email(0);
    robot.from = "Acme <no-reply@acme.com>".into();
    assert_eq!(unsure(&sure_task(0, "p-acme", "x"), &robot).as_deref(), Some("automated sender"));
    let mut cc = email(0);
    cc.to = "team@acme.com".into();
    assert!(!direct(&cc));
    assert_eq!(unsure(&sure_task(0, "p-acme", "x"), &cc).as_deref(), Some("you were not in To"));
    // Outlook gives addresses in any case.
    cc.to = "Team@acme.com, MATT@mattwilson.dev".into();
    assert!(direct(&cc));
    assert!(automated("alerts@bank.com") && !automated("dana@acme.com"));
}

#[test]
fn only_inboxes_with_the_switch_on_mark_sure_tasks() {
    let mut cc = email(1);
    cc.to = "team@acme.com".into();
    let ai = json!({"tasks":[sure_task(0, "p-acme", "Send invoice"), sure_task(1, "p-acme", "Reply")]}).to_string();
    let off: Value = serde_json::from_str(&attach_sources(&ai, &[email(0), cc.clone()], &projects(), "America/New_York", &[]).unwrap()).unwrap();
    assert!(off["tasks"].as_array().unwrap().iter().all(|t| t.get("auto").is_none()));
    assert_eq!(off["tasks"][1]["warning"], "");
    let on: Value = serde_json::from_str(&attach_sources(&ai, &[email(0), cc], &projects(), "America/New_York", &["acct".into()]).unwrap()).unwrap();
    assert_eq!(on["tasks"][0]["auto"], true);
    assert!(on["tasks"][1].get("auto").is_none());
    assert_eq!(on["tasks"][1]["warning"], "Not added automatically: you were not in To.");
    // The flag survives into the review, and is refused on other sources.
    let d = append(draft(), &projects(), &on.to_string()).unwrap();
    assert!(d.candidates[0].auto && !d.candidates[1].auto);
    let mut voice = d.clone();
    voice.source_kind = "voice".into();
    voice.candidates.truncate(1);
    voice.candidates[0].link_url = None;
    assert!(voice.validate().is_err());
}

#[tokio::test]
async fn adds_sure_tasks_tagged_and_undo_removes_them() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&pool).await.unwrap();
    }
    let project = crate::workspace::projects::save(
        &pool,
        None,
        crate::workspace::models::ProjectInput { name: "Acme".into(), color: "cyan".into() },
    )
    .await
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let projects = vec![json!({"id":project,"name":"Acme"})];
    let now = "2026-11-18T19:00:00.000Z";
    // Two sure tasks with the same title: the second is a possible duplicate and stays for review.
    let mut plain = sure_task(2, &project, "Call Sam");
    plain["sure"] = json!(false);
    let ai = json!({"tasks":[sure_task(0, &project, "Send invoice"), sure_task(1, &project, "Send invoice"), plain]}).to_string();
    let t = attach_sources(&ai, &[email(0), email(1), email(2)], &projects, "America/New_York", &["acct".into()]).unwrap();
    let d = save(&pool, append(draft(), &projects, &t).unwrap()).await.unwrap();
    let (d, added) = add_sure(&pool, d, now).await.unwrap();
    assert_eq!(added, 1);
    assert!(d.candidates[0].created_id.is_some());
    assert!(d.candidates[1].created_id.is_none() && !d.candidates[1].auto);
    assert!(d.candidates[1].warning.contains("Not added automatically: Possible duplicate"));
    assert!(d.candidates[2].created_id.is_none());
    let tag: String = sqlx::query_scalar("SELECT g.name FROM task_tags t JOIN tags g ON g.id=t.tag_id")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tag, FROM_EMAIL_TAG);
    // Undo deletes the task and its entry, unless time was tracked on it.
    let task = d.candidates[0].created_id.clone().unwrap();
    sqlx::query("INSERT INTO time_entries(id,task_id,started_at,ended_at,minutes) VALUES('te',?,?,?,5)")
        .bind(&task)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    assert!(undo_task(&pool, &task, now).await.unwrap_err().message.contains("Time is already tracked"));
    sqlx::query("DELETE FROM time_entries").execute(&pool).await.unwrap();
    undo_task(&pool, &task, now).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id=?").bind(&task).fetch_one(&pool).await.unwrap();
    assert_eq!(left, 0);
    assert!(drop_created(&pool, &d.id, d.revision - 1, &d.candidates[0].id).await.is_err());
    let after = drop_created(&pool, &d.id, d.revision, &d.candidates[0].id).await.unwrap();
    assert_eq!(after.candidates.len(), 2);
    assert_eq!(after.revision, d.revision + 1);
}

#[test]
fn a_review_edited_moments_ago_is_left_alone() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-11-18T19:00:00Z").unwrap().with_timezone(&chrono::Utc);
    assert!(recently("2026-11-18T18:59:00+00:00", now));
    assert!(!recently("2026-11-18T18:57:00+00:00", now));
    assert!(!recently("garbage", now));
    let d = new_draft("2026-11-18T19:00:00.000Z", "America/New_York").unwrap();
    assert_eq!(d.source_kind, "email");
    assert!(d.candidates.is_empty());
}
