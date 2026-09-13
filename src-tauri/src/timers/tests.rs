use super::*;
use crate::workspace::{delete, projects, tasks};
async fn db() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("test.db"))
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(10));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    for migration in crate::db::migrations() {
        sqlx::raw_sql(migration.sql).execute(&pool).await.unwrap();
    }
    (dir, pool)
}
async fn task(pool: &SqlitePool, title: &str) -> String {
    let p = projects::save(
        pool,
        None,
        ProjectInput {
            name: title.into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap();
    tasks::create(pool,json!({"project_id":p["id"],"title":title,"status":"todo","priority":"medium","notes_md":"","subtasks":[],"tags":[],"alerts":[],"attachments":[]}),FIXTURE).await.unwrap()["task"]["id"].as_str().unwrap().into()
}
fn at(delta: i64) -> String {
    clock::iso(clock::millis(FIXTURE).unwrap() + delta).unwrap()
}
fn start(id: &str, request: &str) -> Value {
    json!({"requestId":request,"taskId":id})
}
fn action(id: &str, session: &Value, request: &str, revision: i64) -> Value {
    json!({"requestId":request,"taskId":id,"sessionId":session,"expectedRevision":revision})
}
async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}
#[tokio::test]
async fn concurrent_sessions_pause_restart_and_exact_once_receipts() {
    let (dir, p) = db().await;
    let a = task(&p, "A").await;
    let b = task(&p, "B").await;
    let (a1, b1) = tokio::join!(
        mutate(&p, "start", start(&a, "a1"), FIXTURE, 0),
        mutate(&p, "start", start(&b, "b1"), FIXTURE, 0)
    );
    let aid = a1.unwrap()["outcome"]["session_id"].clone();
    let bid = b1.unwrap()["outcome"]["session_id"].clone();
    assert_ne!(aid, bid);
    assert_eq!(count(&p, "timer_sessions").await, 2);
    let again = mutate(&p, "start", start(&a, "a2"), &at(1000), 0)
        .await
        .unwrap();
    assert_eq!(again["outcome"]["session_id"], aid);
    // Header discovery must include names from other projects, without loading their Boards.
    let live = again["snapshot"]["sessions"].as_array().unwrap();
    assert_eq!(live.len(), 2);
    let other = live.iter().find(|s| s["task_id"] == b).unwrap();
    assert_eq!(other["task_title"], "B");
    assert_eq!(other["project_name"], "B");
    assert!(other["project_id"].as_str().is_some());

    mutate(&p, "pause", action(&a, &aid, "pause", 0), &at(1500), 0)
        .await
        .unwrap();
    assert!(
        mutate(&p, "resume", action(&a, &aid, "stale", 0), &at(2000), 0)
            .await
            .is_err()
    );
    // Destroy the connection pool to model restart, then reload from the same disk database.
    p.close().await;
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("test.db"))
                .foreign_keys(true)
                .busy_timeout(std::time::Duration::from_secs(10)),
        )
        .await
        .unwrap();
    mutate(&p, "resume", action(&a, &aid, "resume", 1), &at(60000), 0)
        .await
        .unwrap();
    let stop = action(&a, &aid, "stop", 2);
    let r = mutate(&p, "stop", stop.clone(), &at(62500), 0)
        .await
        .unwrap();
    assert_eq!(r["entries"][0]["minutes"], json!(4000.0 / 60000.0));
    assert_eq!(r["detail"]["task"]["revision"], 1);
    assert!(
        (r["detail"]["task"]["hours_worked"].as_f64().unwrap() - 4000.0 / 3600000.0).abs() < 1e-10
    );
    // Simulate losing the committed reply, then replay it after a replacement session exists.
    let replacement = mutate(&p, "start", start(&a, "replacement"), &at(63000), 0)
        .await
        .unwrap()["outcome"]["session_id"]
        .clone();
    let replay = mutate(&p, "stop", stop, &at(64000), 0).await.unwrap();
    assert_eq!(replay["outcome"], r["outcome"]);
    mutate(
        &p,
        "stop",
        action(&a, &aid, "different-stop-id", 0),
        &at(65000),
        0,
    )
    .await
    .unwrap();
    let old_start = mutate(&p, "start", start(&a, "a1"), &at(65000), 0)
        .await
        .unwrap();
    assert_eq!(old_start["outcome"]["session_id"], aid);
    assert!(old_start["snapshot"]["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"] == replacement));
    assert_eq!(count(&p, "time_entries").await, 1);
    let bstop = mutate(&p, "stop", action(&b, &bid, "bstop", 0), &at(65000), 0)
        .await
        .unwrap();
    assert_eq!(bstop["entries"][0]["minutes"], json!(65000.0 / 60000.0));
    assert!(mutate(&p, "start", start(&b, "a1"), &at(65000), 0)
        .await
        .is_err());
}
#[tokio::test]
async fn wall_clock_corrections_zero_and_paused_stop() {
    let (_d, p) = db().await;
    let a = task(&p, "Clock").await;
    let sid = mutate(&p, "start", start(&a, "start"), FIXTURE, 0)
        .await
        .unwrap()["outcome"]["session_id"]
        .clone();
    mutate(&p, "pause", action(&a, &sid, "pause", 0), &at(1000), 0)
        .await
        .unwrap();
    mutate(&p, "resume", action(&a, &sid, "resume", 1), &at(5000), 0)
        .await
        .unwrap();
    let r = mutate(&p, "stop", action(&a, &sid, "stop", 2), &at(-10000), 0)
        .await
        .unwrap();
    assert_eq!(r["entries"][0]["minutes"], json!(1.0 / 60.0));
    assert_eq!(r["entries"][0]["ended_at"], FIXTURE);
    let sid = mutate(&p, "start", start(&a, "new"), FIXTURE, 0)
        .await
        .unwrap()["outcome"]["session_id"]
        .clone();
    mutate(&p, "pause", action(&a, &sid, "p2", 0), FIXTURE, 0)
        .await
        .unwrap();
    let r = mutate(&p, "stop", action(&a, &sid, "s2", 1), &at(100000), 0)
        .await
        .unwrap();
    assert!(r["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["minutes"].as_f64() == Some(0.0)));
}
#[tokio::test]
async fn manual_logging_overlap_validation_and_atomic_rollback() {
    let (_d, p) = db().await;
    let a = task(&p, "Log").await;
    let input = json!({"requestId":"log1","taskId":a,"startedAt":at(-100000),"durationMs":1250});
    let r = mutate(&p, "log", input.clone(), FIXTURE, 0).await.unwrap();
    assert_eq!(r["entries"][0]["minutes"], json!(1250.0 / 60000.0));
    mutate(&p, "log", input.clone(), FIXTURE, 0).await.unwrap();
    assert_eq!(count(&p, "time_entries").await, 1);
    let mut changed = input.clone();
    changed["durationMs"] = json!(2000);
    assert_eq!(
        mutate(&p, "log", changed, FIXTURE, 0)
            .await
            .unwrap_err()
            .code,
        "Conflict"
    );
    for duration in [0, -1, MAX_MS + 1, 200000] {
        let bad = json!({"requestId":format!("bad-{duration}"),"taskId":a,"startedAt":at(-100000),"durationMs":duration});
        assert!(mutate(&p, "log", bad, FIXTURE, 0).await.is_err());
    }
    let mut overlap = input.clone();
    overlap["requestId"] = json!("overlap");
    mutate(&p, "log", overlap, FIXTURE, 0).await.unwrap();
    assert_eq!(count(&p, "time_entries").await, 2);
    // Fail after entry/cache insertion, before durable receipt: all effects must roll back.
    sqlx::raw_sql("CREATE TRIGGER fail_receipt BEFORE INSERT ON timer_requests BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").execute(&p).await.unwrap();
    let mut fail = input.clone();
    fail["requestId"] = json!("fail");
    assert!(mutate(&p, "log", fail, FIXTURE, 0).await.is_err());
    assert_eq!(count(&p, "time_entries").await, 2);
    sqlx::raw_sql("DROP TRIGGER fail_receipt")
        .execute(&p)
        .await
        .unwrap();
    let sid = mutate(&p, "start", start(&a, "timer"), FIXTURE, 0)
        .await
        .unwrap()["outcome"]["session_id"]
        .clone();
    sqlx::raw_sql("CREATE TRIGGER fail_stop BEFORE UPDATE OF state ON timer_sessions WHEN NEW.state='stopped' BEGIN SELECT RAISE(ABORT,'injected finalize failure'); END;").execute(&p).await.unwrap();
    assert!(
        mutate(&p, "stop", action(&a, &sid, "failed-stop", 0), &at(1000), 0)
            .await
            .is_err()
    );
    assert_eq!(count(&p, "time_entries").await, 2);
    let state: String = sqlx::query_scalar("SELECT state FROM timer_sessions WHERE id=?")
        .bind(sid.as_str())
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(state, "running");
}
#[tokio::test]
async fn deletion_fingerprints_discard_sessions_and_preserve_others() {
    let (_d, p) = db().await;
    let a = task(&p, "Delete").await;
    let b = task(&p, "Keep").await;
    let aid = mutate(&p, "start", start(&a, "a"), FIXTURE, 0)
        .await
        .unwrap()["outcome"]["session_id"]
        .clone();
    mutate(&p, "start", start(&b, "b"), FIXTURE, 0)
        .await
        .unwrap();
    let target = DeletionTarget {
        kind: "task".into(),
        id: a.clone(),
    };
    let first = delete::preview(&p, &target).await.unwrap();
    assert_eq!(first["counts"]["timer_sessions"], 1);
    assert_eq!(first, delete::preview(&p, &target).await.unwrap());
    mutate(&p, "pause", action(&a, &aid, "pause", 0), &at(1000), 0)
        .await
        .unwrap();
    assert!(delete::remove(
        &p,
        &target,
        first["fingerprint"].as_str().unwrap(),
        &at(2000)
    )
    .await
    .is_err());
    let current = delete::preview(&p, &target).await.unwrap();
    delete::remove(
        &p,
        &target,
        current["fingerprint"].as_str().unwrap(),
        &at(3000),
    )
    .await
    .unwrap();
    assert_eq!(count(&p, "timer_sessions").await, 1);
    assert_eq!(count(&p, "time_entries").await, 0);
    assert!(mutate(&p, "start", start(&a, "a"), &at(4000), 0)
        .await
        .is_err());
    assert!(
        mutate(&p, "stop", action(&a, &aid, "late", 1), &at(4000), 0)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn fixture_installation_is_atomic_repeatable_and_preserves_deletions() {
    let (_d, p) = db().await;
    sqlx::query("INSERT INTO settings VALUES('fixture_version','september-2025-midday-v1')")
        .execute(&p)
        .await
        .unwrap();
    let a = task(&p, "Fixture").await;
    sqlx::query("UPDATE tasks SET id='f870c681-27a4-4d65-87be-00000000006c' WHERE id=?")
        .bind(a)
        .execute(&p)
        .await
        .unwrap();
    // Existing original task has no children; first install deliberately lacks CLI task.
    let real = "2026-09-12T12:00:00.000Z";
    let (offset, warnings) = initialize_into(&p, true, real).await.unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        clock::iso(clock::millis(real).unwrap() + offset).unwrap(),
        FIXTURE
    );
    let row: (String, String) = sqlx::query_as("SELECT id,started_at FROM timer_sessions")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(row.1, at(-2537000));
    sqlx::query("DELETE FROM timer_sessions WHERE id=?")
        .bind(row.0)
        .execute(&p)
        .await
        .unwrap();
    assert_eq!(
        initialize_into(&p, true, &at(999999)).await.unwrap().0,
        offset
    );
    assert_eq!(count(&p, "timer_sessions").await, 0);
    assert_eq!(initialize_into(&p, false, real).await.unwrap(), (0, vec![]));
    sqlx::query("UPDATE settings SET value='broken' WHERE key='fixture_clock_m2'")
        .execute(&p)
        .await
        .unwrap();
    assert!(initialize_into(&p, true, real).await.is_err());
}
#[tokio::test]
async fn failed_fixture_marker_rolls_back_anchor_and_sessions() {
    let (_d, p) = db().await;
    sqlx::query("INSERT INTO settings VALUES('fixture_version','september-2025-midday-v1')")
        .execute(&p)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TRIGGER fail_marker BEFORE INSERT ON settings WHEN NEW.key='fixture_timers_m2' BEGIN SELECT RAISE(ABORT,'injected marker failure'); END;").execute(&p).await.unwrap();
    assert!(initialize_into(&p, true, FIXTURE).await.is_err());
    assert_eq!(count(&p, "timer_sessions").await, 0);
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE key='fixture_clock_m2'")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
