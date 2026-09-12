use super::*;
use crate::workspace::{projects, tasks};
async fn database() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0001_initial.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0002_board_fields.sql"))
        .execute(&pool)
        .await
        .unwrap();
    pool
}
#[tokio::test]
async fn stage_failure_compensates_and_recovery_preserves_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let files = Files::new(root.clone()).unwrap();
    assert!(Files::new(root).is_err());
    let pool = database().await;
    let source = temp.path().join("source.txt");
    fs::write(&source, "attachment").unwrap();
    assert!(files
        .stage(
            &pool,
            vec![source.clone(), temp.path().join("missing.txt")],
            &now(true)
        )
        .await
        .is_err());
    assert!(source.exists());
    assert!(rows(
        &mut *pool.acquire().await.unwrap(),
        "SELECT * FROM attachment_file_ops",
        vec![]
    )
    .await
    .unwrap()
    .is_empty());
    let staged = files
        .stage(&pool, vec![source.clone()], &now(true))
        .await
        .unwrap();
    assert_eq!(staged[0]["size"], 10);
    assert!(files.path("../source.txt").is_err());
    assert!(files.path("C:\\outside").is_err());
    files.recover(&pool, &now(true)).await.unwrap();
    assert!(source.exists());
    assert!(rows(
        &mut *pool.acquire().await.unwrap(),
        "SELECT * FROM attachment_file_ops",
        vec![]
    )
    .await
    .unwrap()
    .is_empty());
}
#[tokio::test]
async fn adoption_rollback_keeps_ready_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let files = Files::new(temp.path().join("managed")).unwrap();
    let pool = database().await;
    let source = temp.path().join("source.txt");
    fs::write(&source, "copy").unwrap();
    let staged = files
        .stage(&pool, vec![source.clone()], &now(true))
        .await
        .unwrap();
    let token = staged[0]["token"].clone();
    let project = projects::save(
        &pool,
        None,
        ProjectInput {
            name: "Files".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap();
    let mut input = json!({"project_id":project["id"],"title":"Files","status":"backlog","priority":"medium","attachments":[token,"bad-token"]});
    assert!(tasks::create(&pool, input.clone(), &now(true))
        .await
        .is_err());
    assert_eq!(
        rows(
            &mut *pool.acquire().await.unwrap(),
            "SELECT * FROM attachment_file_ops WHERE state='ready'",
            vec![]
        )
        .await
        .unwrap()
        .len(),
        1
    );
    input["attachments"] = json!([token]);
    let reply = tasks::create(&pool, input, &now(true)).await.unwrap();
    assert_eq!(reply["attachments"].as_array().unwrap().len(), 1);
    files.recover(&pool, &now(true)).await.unwrap();
    let path = files
        .path(reply["attachments"][0]["path"].as_str().unwrap())
        .unwrap();
    assert!(path.exists());
    assert!(source.exists());
}

#[cfg(windows)]
#[tokio::test]
async fn locked_cleanup_is_durable_and_retries_without_touching_other_workspace() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = tempfile::tempdir().unwrap();
    let ordinary = Files::new(temp.path().join("ordinary")).unwrap();
    let fixture = Files::new(temp.path().join("fixture")).unwrap();
    let pool = database().await;
    let source = temp.path().join("original.txt");
    fs::write(&source, "preserve me").unwrap();
    fs::write(ordinary.root.join("unrelated.txt"), "ordinary").unwrap();
    let staged = fixture
        .stage(&pool, vec![source.clone()], &now(true))
        .await
        .unwrap();
    let token = staged[0]["token"].as_str().unwrap().to_string();
    let op = one(
        &mut *pool.acquire().await.unwrap(),
        "SELECT * FROM attachment_file_ops WHERE token=?",
        vec![json!(token)],
    )
    .await
    .unwrap();
    let copy = fixture.path(op["relative_path"].as_str().unwrap()).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&copy)
        .unwrap();
    assert!(fixture
        .discard(&pool, &[token.clone()], &now(true))
        .await
        .unwrap());
    let pending = one(
        &mut *pool.acquire().await.unwrap(),
        "SELECT * FROM attachment_file_ops WHERE token=?",
        vec![json!(token)],
    )
    .await
    .unwrap();
    assert_eq!(pending["state"], "delete_pending");
    assert!(pending["last_error"].is_string());
    drop(lock);
    assert!(!fixture.cleanup(&pool, &now(true)).await.unwrap());
    assert!(!copy.exists());
    assert!(source.exists());
    assert!(ordinary.root.join("unrelated.txt").exists());
}
