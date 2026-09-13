use super::*;
use crate::workspace::{delete, projects, tasks};
const NOW: &str = "2025-09-11T17:42:00.000Z";
#[test]
fn meeting_fold_reserves_every_occupied_wall_minute() {
    let query = q("2026-11-01");
    let meeting = json!({"starts_at":"2026-11-01T05:45:00.000Z","duration_min":30});
    let range = meeting_range(&meeting, &query).unwrap().unwrap();
    assert_eq!(range, (60, 120));
    assert_eq!(next_free(&[range], 30, 60), Some((120, 150)));
    let ordinary = json!({"starts_at":"2026-11-01T15:00:00.000Z","duration_min":30});
    assert_eq!(meeting_range(&ordinary, &query).unwrap(), Some((600, 630)));
}
async fn db() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("planner.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for migration in crate::db::migrations() {
        sqlx::raw_sql(migration.sql).execute(&p).await.unwrap();
    }
    (dir, p)
}
async fn task(p: &SqlitePool) -> String {
    let project = projects::save(
        p,
        None,
        ProjectInput {
            name: "Planner".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap();
    tasks::create(p,json!({"project_id":project["id"],"title":"Schedule work","status":"todo","priority":"medium","due_at":"2025-09-12T14:00:00.000Z","estimate_h":1,"notes_md":"","subtasks":[],"tags":[],"alerts":[],"attachments":[]}),NOW).await.unwrap()["task"]["id"].as_str().unwrap().into()
}
fn q(date: &str) -> PlannerQuery {
    PlannerQuery {
        date: date.into(),
        time_zone: "America/New_York".into(),
    }
}
fn input(request: &str, action: &str, payload: Value) -> PlannerInput {
    PlannerInput {
        request_id: request.into(),
        query: q("2025-09-12"),
        action: action.into(),
        payload,
        expected_fingerprint: None,
    }
}
async fn read(p: &SqlitePool, date: &str) -> Value {
    let mut tx = p.begin().await.unwrap();
    let v = snapshot(&mut tx, &q(date), NOW, 0).await.unwrap();
    tx.commit().await.unwrap();
    v
}
async fn create(p: &SqlitePool, task: Option<&str>, request: &str, start: i64, end: i64) -> Value {
    apply_into(p,input(request,"create",json!({"draft":{"kind":if task.is_some(){"task"}else{"focus"},"task_id":task,"start_min":start,"end_min":end}})),NOW,0).await.unwrap()
}
async fn count(p: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(p)
        .await
        .unwrap()
}
#[test]
fn calendar_gap_fold_and_no_space() {
    let mut draft = BlockDraft {
        kind: "focus".into(),
        task_id: None,
        start_min: 120,
        end_min: 180,
        start_offset: None,
        end_offset: None,
        source_id: None,
    };
    assert!(validate_draft(&mut draft, &q("2026-03-08")).is_err());
    draft.start_min = 60;
    draft.end_min = 120;
    assert!(validate_draft(&mut draft, &q("2026-11-01")).is_err());
    draft.start_offset = Some("-05:00".into());
    validate_draft(&mut draft, &q("2026-11-01")).unwrap();
    assert_eq!(draft.end_offset.as_deref(), Some("-05:00"));
    assert!(calendar(&q("2026-02-30")).is_err());
    assert_eq!(floor(&q("2025-09-11"), NOW).unwrap(), 825);
    assert_eq!(
        next_free(&[(420, 460), (450, 500)], 30, 420),
        Some((510, 540))
    );
    assert_eq!(next_free(&[], 30, 1425), None);
}
#[tokio::test]
async fn revisions_replay_detachment_and_restart() {
    let (dir, p) = db().await;
    let task = task(&p).await;
    let original = input(
        "create",
        "create",
        json!({"draft":{"kind":"task","task_id":task,"start_min":540,"end_min":600}}),
    );
    let result = apply_into(&p, original.clone(), NOW, 0).await.unwrap();
    let id = result["outcome"]["block_ids"][0].clone();
    let session = timers::mutate(
        &p,
        "start",
        json!({"taskId":task,"requestId":"start","blockId":id}),
        NOW,
        0,
    )
    .await
    .unwrap();
    let sid = session["outcome"]["session_id"].clone();
    let mut bad = original.clone();
    bad.payload["draft"]["end_min"] = json!(615);
    assert_eq!(
        apply_into(&p, bad, NOW, 0).await.unwrap_err().code,
        "Conflict"
    );
    let mut remove = input("remove", "remove", json!({"id":id,"expectedRevision":0}));
    remove.expected_fingerprint = read(&p, "2025-09-12").await["fingerprint"]
        .as_str()
        .map(str::to_owned);
    let removed = apply_into(&p, remove, NOW, 0).await.unwrap();
    assert_eq!(removed["snapshot"]["timers"]["sessions"][0]["id"], sid);
    assert!(removed["snapshot"]["timers"]["sessions"][0]["block_id"].is_null());
    assert_eq!(count(&p, "time_entries").await, 0);
    p.close().await;
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("planner.db"))
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let replay = apply_into(&p, original, NOW, 0).await.unwrap();
    assert_eq!(replay["outcome"]["block_ids"][0], id);
    assert_eq!(count(&p, "blocks").await, 0);
    assert_eq!(count(&p, "timer_sessions").await, 1);
}
#[tokio::test]
async fn stop_complete_is_atomic_and_removal_preserves_accounting() {
    let (_dir, p) = db().await;
    let task = task(&p).await;
    let result = create(&p, Some(&task), "block", 540, 600).await;
    let id = result["outcome"]["block_ids"][0].clone();
    let start = timers::mutate(
        &p,
        "start",
        json!({"taskId":task,"requestId":"start","blockId":id}),
        NOW,
        0,
    )
    .await
    .unwrap();
    let sid = start["outcome"]["session_id"].clone();
    let no_stop = input(
        "missing",
        "done",
        json!({"id":id,"expectedRevision":0,"done":true}),
    );
    assert!(apply_into(&p, no_stop, NOW, 0).await.is_err());
    let done = input(
        "done",
        "done",
        json!({"id":id,"expectedRevision":0,"done":true,"stopSession":{"id":sid,"revision":0}}),
    );
    sqlx::raw_sql("CREATE TRIGGER fail_receipt BEFORE INSERT ON planner_requests BEGIN SELECT RAISE(ABORT,'late failure'); END;").execute(&p).await.unwrap();
    assert!(apply_into(&p, done.clone(), "2025-09-11T18:42:00.000Z", 0)
        .await
        .is_err());
    assert_eq!(count(&p, "time_entries").await, 0);
    assert_eq!(
        read(&p, "2025-09-12").await["timers"]["sessions"][0]["state"],
        "running"
    );
    sqlx::raw_sql("DROP TRIGGER fail_receipt")
        .execute(&p)
        .await
        .unwrap();
    let result = apply_into(&p, done.clone(), "2025-09-11T18:42:00.000Z", 0)
        .await
        .unwrap();
    assert_eq!(result["changed_details"][0]["task"]["hours_worked"], 1.0);
    assert_eq!(result["changed_entries"][&task][0]["minutes"], 60.0);
    assert_eq!(result["snapshot"]["blocks"][0]["done"], 1);
    assert_eq!(result["changed_details"][0]["task"]["status"], "todo");
    apply_into(&p, done, "2025-09-11T19:42:00.000Z", 0)
        .await
        .unwrap();
    assert_eq!(count(&p, "time_entries").await, 1);
    let mut remove = input("remove", "remove", json!({"id":id,"expectedRevision":1}));
    remove.expected_fingerprint = result["snapshot"]["fingerprint"]
        .as_str()
        .map(str::to_owned);
    let removed = apply_into(&p, remove, "2025-09-11T19:42:00.000Z", 0)
        .await
        .unwrap();
    assert_eq!(removed["changed_details"][0]["task"]["hours_worked"], 1.0);
    assert!(removed["changed_entries"][&task][0]["block_id"].is_null());
}
#[tokio::test]
async fn carry_claims_survive_result_removal_and_parent_delete_is_valid() {
    let (_dir, p) = db().await;
    let task = task(&p).await;
    let mut source = input(
        "source",
        "create",
        json!({"draft":{"kind":"task","task_id":task,"start_min":540,"end_min":630}}),
    );
    source.query = q("2025-09-11");
    let source = apply_into(&p, source, NOW, 0).await.unwrap()["outcome"]["block_ids"][0].clone();
    let mut carry = input(
        "carry",
        "batch",
        json!({"mode":"carry","blocks":[{"kind":"task","task_id":task,"start_min":420,"end_min":510,"source_id":source}]}),
    );
    carry.expected_fingerprint = read(&p, "2025-09-12").await["fingerprint"]
        .as_str()
        .map(str::to_owned);
    let carried = apply_into(&p, carry.clone(), NOW, 0).await.unwrap();
    let id = carried["outcome"]["block_ids"][0].clone();
    assert_eq!(
        carried["snapshot"]["blocks"][0]["carried_from_block_id"],
        source
    );
    let mut remove = input("remove", "remove", json!({"id":id,"expectedRevision":0}));
    remove.expected_fingerprint = carried["snapshot"]["fingerprint"]
        .as_str()
        .map(str::to_owned);
    apply_into(&p, remove, NOW, 0).await.unwrap();
    carry.request_id = "carry-again".into();
    carry.expected_fingerprint = read(&p, "2025-09-12").await["fingerprint"]
        .as_str()
        .map(str::to_owned);
    assert!(apply_into(&p, carry, NOW, 0).await.is_err());
    let target = DeletionTarget {
        kind: "task".into(),
        id: task,
    };
    let preview = delete::preview(&p, &target).await.unwrap();
    delete::remove(&p, &target, preview["fingerprint"].as_str().unwrap(), NOW)
        .await
        .unwrap();
    assert_eq!(count(&p, "planner_claims").await, 0);
    assert!(count(&p, "planner_requests").await > 0);
}
#[tokio::test]
async fn automatic_placement_checks_all_occupancy_and_stale_clock() {
    let (_dir, p) = db().await;
    create(&p, None, "occupied", 420, 480).await;
    let mut quick = input(
        "quick",
        "batch",
        json!({"mode":"quick","blocks":[{"kind":"break","task_id":null,"start_min":420,"end_min":450}]}),
    );
    quick.expected_fingerprint = read(&p, "2025-09-12").await["fingerprint"]
        .as_str()
        .map(str::to_owned);
    assert!(apply_into(&p, quick.clone(), NOW, 0).await.is_err());
    quick.payload["blocks"][0]["start_min"] = json!(480);
    quick.payload["blocks"][0]["end_min"] = json!(510);
    apply_into(&p, quick, NOW, 0).await.unwrap();
    let mut stale = input(
        "stale",
        "batch",
        json!({"mode":"quick","blocks":[{"kind":"break","task_id":null,"start_min":510,"end_min":540}]}),
    );
    stale.expected_fingerprint = read(&p, "2025-09-12").await["fingerprint"]
        .as_str()
        .map(str::to_owned);
    assert!(apply_into(&p, stale, "2025-09-12T20:00:00.000Z", 0)
        .await
        .is_err());
}
#[tokio::test]
async fn existing_task_timer_keeps_original_association() {
    let (_dir, p) = db().await;
    let task = task(&p).await;
    let a = create(&p, Some(&task), "a", 540, 600).await["outcome"]["block_ids"][0].clone();
    let b = create(&p, Some(&task), "b", 600, 660).await["outcome"]["block_ids"][0].clone();
    let first = timers::mutate(
        &p,
        "start",
        json!({"requestId":"one","taskId":task,"blockId":a}),
        NOW,
        0,
    )
    .await
    .unwrap();
    let second = timers::mutate(
        &p,
        "start",
        json!({"requestId":"two","taskId":task,"blockId":b}),
        NOW,
        0,
    )
    .await
    .unwrap();
    assert_eq!(first["outcome"], second["outcome"]);
    assert_eq!(second["snapshot"]["sessions"][0]["block_id"], a);
    let old: timers::models::StartInput =
        serde_json::from_value(json!({"requestId":"old","taskId":task})).unwrap();
    assert!(json!(old).get("blockId").is_none());
}

#[tokio::test]
async fn moving_to_another_date_preserves_identity_history_and_live_timer() {
    let (_dir, p) = db().await;
    let task = task(&p).await;
    let id =
        create(&p, Some(&task), "date-block", 540, 600).await["outcome"]["block_ids"][0].clone();
    let first = timers::mutate(
        &p,
        "start",
        json!({"taskId":task,"requestId":"first","blockId":id}),
        NOW,
        0,
    )
    .await
    .unwrap();
    timers::mutate(&p,"stop",json!({"taskId":task,"requestId":"stop","sessionId":first["outcome"]["session_id"],"expectedRevision":0}),"2025-09-11T17:43:00.000Z",0).await.unwrap();
    let live = timers::mutate(
        &p,
        "start",
        json!({"taskId":task,"requestId":"second","blockId":id}),
        "2025-09-11T17:44:00.000Z",
        0,
    )
    .await
    .unwrap();
    let before = live["snapshot"]["sessions"].clone();
    let change = input(
        "change-date",
        "move",
        json!({"id":id,"expectedRevision":0,"destinationDate":"2025-09-15","start_min":600,"end_min":690}),
    );
    let result = apply_into(&p, change.clone(), "2025-09-11T17:45:00.000Z", 0)
        .await
        .unwrap();
    assert!(result["snapshot"]["blocks"].as_array().unwrap().is_empty());
    assert_eq!(result["outcome"]["destination_date"], "2025-09-15");
    let destination = read(&p, "2025-09-15").await;
    assert_eq!(destination["blocks"][0]["id"], id);
    assert_eq!(destination["blocks"][0]["start_min"], 600);
    assert_eq!(destination["blocks"][0]["end_min"], 690);
    assert_eq!(destination["timers"]["sessions"], before);
    assert_eq!(result["changed_entries"][&task][0]["block_id"], id);
    assert_eq!(result["changed_entries"][&task][0]["minutes"], 1.0);
    assert_eq!(
        result["changed_details"][0]["task"]["due_at"],
        "2025-09-12T14:00:00.000Z"
    );
    apply_into(&p, change.clone(), NOW, 0).await.unwrap();
    assert_eq!(count(&p, "blocks").await, 1);
    assert_eq!(read(&p, "2025-09-15").await["blocks"][0]["revision"], 1);
    let mut mismatch = change;
    mismatch.payload["destinationDate"] = json!("2025-09-16");
    assert_eq!(
        apply_into(&p, mismatch, NOW, 0).await.unwrap_err().code,
        "Conflict"
    );
}
#[tokio::test]
async fn destination_date_validation_and_late_failure_leave_source_unchanged() {
    let (_dir, p) = db().await;
    let id = create(&p, None, "source", 120, 180).await["outcome"]["block_ids"][0].clone();
    for date in ["2026-02-30", "2026-03-08"] {
        let change = input(
            date,
            "move",
            json!({"id":id,"expectedRevision":0,"destinationDate":date,"start_min":120,"end_min":180}),
        );
        assert!(apply_into(&p, change, NOW, 0).await.is_err());
    }
    assert_eq!(read(&p, "2025-09-12").await["blocks"][0]["revision"], 0);
    let mut fold = input(
        "fold",
        "move",
        json!({"id":id,"expectedRevision":0,"destinationDate":"2026-11-01","start_min":60,"end_min":120}),
    );
    assert!(apply_into(&p, fold.clone(), NOW, 0).await.is_err());
    fold.payload["start_offset"] = json!("-05:00");
    sqlx::raw_sql("CREATE TRIGGER fail_date_receipt BEFORE INSERT ON planner_requests BEGIN SELECT RAISE(ABORT,'late failure'); END;").execute(&p).await.unwrap();
    assert!(apply_into(&p, fold.clone(), NOW, 0).await.is_err());
    assert_eq!(read(&p, "2025-09-12").await["blocks"][0]["id"], id);
    sqlx::raw_sql("DROP TRIGGER fail_date_receipt")
        .execute(&p)
        .await
        .unwrap();
    apply_into(&p, fold, NOW, 0).await.unwrap();
    let destination = read(&p, "2026-11-01").await;
    assert_eq!(destination["blocks"][0]["start_offset"], "-05:00");
    let mut new = input(
        "new-on-date",
        "create",
        json!({"destinationDate":"2025-09-16","draft":{"kind":"break","task_id":null,"start_min":1425,"end_min":1440}}),
    );
    apply_into(&p, new.clone(), NOW, 0).await.unwrap();
    assert_eq!(read(&p, "2025-09-16").await["blocks"][0]["end_min"], 1440);
    new.request_id = "invalid-create".into();
    new.payload["destinationDate"] = json!("invalid");
    assert!(apply_into(&p, new, NOW, 0).await.is_err());
}
