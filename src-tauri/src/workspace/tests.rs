use super::{delete, models::*, projects, tasks};
use serde_json::{json, Value};
use sqlx::SqlitePool;
pub async fn database() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    for migration in crate::db::migrations() {
        sqlx::raw_sql(migration.sql).execute(&pool).await.unwrap();
    }
    pool
}
pub async fn project(pool: &SqlitePool) -> String {
    projects::save(
        pool,
        None,
        ProjectInput {
            name: "Test project".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
pub fn input(project: &str, title: &str) -> Value {
    json!({"project_id":project,"title":title,"status":"backlog","priority":"medium","due_at":null,"estimate_h":null,"external_url":null,"external_provider":null,"external_id":null,"blocked_reason":null,"blocked_on":null,"notes_md":"","subtasks":[],"tags":[],"alerts":[1440,60],"attachments":[]})
}
#[tokio::test]
async fn status_order_revision_and_child_rollback() {
    let pool = database().await;
    let project = project(&pool).await;
    let clock = now(true);
    let a = tasks::create(&pool, input(&project, " A "), &clock)
        .await
        .unwrap();
    let id = a["task"]["id"].as_str().unwrap();
    assert_eq!(a["task"]["title"], "A");
    let b = tasks::create(&pool, input(&project, "B"), &clock)
        .await
        .unwrap();
    let bid = b["task"]["id"].as_str().unwrap();
    let moved = tasks::move_task(&pool, bid, "backlog", Some(id), 0, &clock)
        .await
        .unwrap();
    assert_eq!(moved["task"]["sort_order"], 0);
    assert_eq!(moved["task"]["revision"], 1);
    assert!(
        tasks::update(&pool, id, json!({"title":"stale"}), 0, &clock)
            .await
            .is_err()
    );
    let blocked = tasks::update(
        &pool,
        id,
        json!({"status":"blocked","blocked_reason":"Waiting"}),
        1,
        &clock,
    )
    .await
    .unwrap();
    assert_eq!(blocked["task"]["blocked_since"], clock);
    let unchanged = tasks::update(
        &pool,
        id,
        json!({"status":"blocked"}),
        2,
        "2025-09-12T00:00:00.000Z",
    )
    .await
    .unwrap();
    assert_eq!(unchanged["task"]["revision"], 2);
    assert_eq!(unchanged["task"]["blocked_since"], clock);
    let done = tasks::update(&pool, id, json!({"status":"done"}), 2, &clock)
        .await
        .unwrap();
    assert!(done["task"]["blocked_since"].is_null());
    assert_eq!(done["task"]["done_at"], clock);
    assert!(tasks::children(
        &pool,
        id,
        "subtasks",
        json!([{"id":"other-task-child","title":"bad","done":false}]),
        3,
        &clock
    )
    .await
    .is_err());
    assert_eq!(tasks::get(&pool, id).await.unwrap()["task"]["revision"], 3);
    assert!(
        tasks::update(&pool, id, json!({"hours_worked":100}), 3, &clock)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn deletion_fingerprint_and_related_graph() {
    let pool = database().await;
    let p = project(&pool).await;
    let q = project(&pool).await;
    let clock = now(true);
    let task = tasks::create(&pool, input(&p, "Delete me"), &clock)
        .await
        .unwrap();
    let id = task["task"]["id"].as_str().unwrap();
    let other = tasks::create(&pool, input(&q, "Retain me"), &clock)
        .await
        .unwrap();
    let other_id = other["task"]["id"].as_str().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    execute(&mut conn,"INSERT INTO blocks(id,date,start_min,end_min,kind,task_id) VALUES ('b','2025-09-11',540,600,'task',?)",vec![json!(id)]).await.unwrap();
    execute(&mut conn,"INSERT INTO blocks(id,date,start_min,end_min,kind,task_id,carried_from_block_id) VALUES ('carry','2025-09-12',540,600,'task',?,'b')",vec![json!(other_id)]).await.unwrap();
    execute(&mut conn,"INSERT INTO time_entries(id,task_id,block_id,started_at,ended_at,minutes) VALUES ('entry',?,'b','2025-09-11T13:00:00.000Z','2025-09-11T13:01:30.000Z',1.5)",vec![json!(id)]).await.unwrap();
    drop(conn);
    let target = DeletionTarget {
        kind: "project".into(),
        id: p.clone(),
    };
    let preview = delete::preview(&pool, &target).await.unwrap();
    tasks::update(&pool, id, json!({"title":"Changed"}), 0, &clock)
        .await
        .unwrap();
    assert!(delete::remove(
        &pool,
        &target,
        preview["fingerprint"].as_str().unwrap(),
        &clock
    )
    .await
    .is_err());
    let preview = delete::preview(&pool, &target).await.unwrap();
    delete::remove(
        &pool,
        &target,
        preview["fingerprint"].as_str().unwrap(),
        &clock,
    )
    .await
    .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    assert!(one(
        &mut conn,
        "SELECT carried_from_block_id FROM blocks WHERE id='carry'",
        vec![]
    )
    .await
    .unwrap()["carried_from_block_id"]
        .is_null());
    assert!(rows(&mut conn, "PRAGMA foreign_key_check", vec![])
        .await
        .unwrap()
        .is_empty());
    drop(conn);
    assert!(tasks::get(&pool, other_id).await.is_ok());
    assert!(tasks::get(&pool, id).await.is_err());
}
#[tokio::test]
async fn seed_after_upgrade_orders_once_and_rolls_back() {
    let pool = database().await;
    let data = json!([{"table":"projects","rows":[{"id":"p","name":"Fixture","color":"cyan"}]},{"table":"tasks","rows":[{"id":"a","project_id":"p","title":"A","created_at":"2025-09-11T00:00:00.000Z","updated_at":"2025-09-11T00:00:00.000Z"},{"id":"b","project_id":"p","title":"B","created_at":"2025-09-11T00:00:00.000Z","updated_at":"2025-09-11T00:00:00.000Z"}]}]);
    crate::db::seed_into(
        &pool,
        "september-2025-midday-v1",
        serde_json::from_value(data.clone()).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        tasks::get(&pool, "b").await.unwrap()["task"]["sort_order"],
        1
    );
    tasks::move_task(&pool, "b", "backlog", Some("a"), 0, &now(true))
        .await
        .unwrap();
    crate::db::seed_into(
        &pool,
        "september-2025-midday-v1",
        serde_json::from_value(data).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        tasks::get(&pool, "b").await.unwrap()["task"]["sort_order"],
        0
    );
    let fresh = database().await;
    let invalid = json!([{"table":"projects","rows":[{"id":"p","name":"Fixture","color":"cyan"}]},{"table":"tasks","rows":[{"id":"broken"}]}]);
    assert!(crate::db::seed_into(
        &fresh,
        "september-2025-midday-v1",
        serde_json::from_value(invalid).unwrap()
    )
    .await
    .is_err());
    assert!(projects::list(&fresh).await.unwrap().is_empty());
    assert!(rows(
        &mut *fresh.acquire().await.unwrap(),
        "SELECT * FROM settings WHERE key='fixture_version'",
        vec![]
    )
    .await
    .unwrap()
    .is_empty());
}

#[tokio::test]
async fn children_due_precision_and_delete_failure_are_transactional() {
    let pool = database().await;
    let p = project(&pool).await;
    let mut data = input(&p, "Collections");
    data["due_at"] = json!("2025-09-11T21:00:37.123Z");
    data["tags"] = json!([{"name":" Review ","color":"cyan"},{"name":"review","color":"lime"}]);
    data["subtasks"] = json!([{"title":"First","done":false},{"title":"Second","done":true}]);
    let created = tasks::create(&pool, data, &now(true)).await.unwrap();
    assert_eq!(created["tags"].as_array().unwrap().len(), 1);
    let id = created["task"]["id"].as_str().unwrap();
    let done = tasks::update(
        &pool,
        id,
        json!({"status":"done","estimate_h":0}),
        0,
        &now(true),
    )
    .await
    .unwrap();
    assert_eq!(done["task"]["due_at"], "2025-09-11T21:00:37.123Z");
    let left = tasks::update(&pool, id, json!({"status":"todo"}), 1, &now(true))
        .await
        .unwrap();
    assert!(left["task"]["done_at"].is_null());
    let children =
        json!([{"id":created["subtasks"][1]["id"],"title":"Second updated","done":false}]);
    let edited = tasks::children(&pool, id, "subtasks", children, 2, &now(true))
        .await
        .unwrap();
    assert_eq!(edited["subtasks"][0]["id"], created["subtasks"][1]["id"]);
    assert_eq!(edited["subtasks"][0]["sort_order"], 0);
    let alerts = tasks::children(&pool, id, "alerts", json!([60, 60, 5]), 3, &now(true))
        .await
        .unwrap();
    assert_eq!(alerts["alerts"].as_array().unwrap().len(), 2);
    let target = DeletionTarget {
        kind: "task".into(),
        id: id.into(),
    };
    let preview = delete::preview(&pool, &target).await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER fail_delete BEFORE DELETE ON tasks BEGIN SELECT RAISE(ABORT,'injected delete failure'); END;").execute(&pool).await.unwrap();
    assert!(delete::remove(
        &pool,
        &target,
        preview["fingerprint"].as_str().unwrap(),
        &now(true)
    )
    .await
    .is_err());
    assert_eq!(tasks::get(&pool, id).await.unwrap(), alerts);
    sqlx::raw_sql("DROP TRIGGER fail_delete")
        .execute(&pool)
        .await
        .unwrap();
    delete::remove(
        &pool,
        &target,
        preview["fingerprint"].as_str().unwrap(),
        &now(true),
    )
    .await
    .unwrap();
    assert_eq!(
        rows(
            &mut *pool.acquire().await.unwrap(),
            "SELECT * FROM tags",
            vec![]
        )
        .await
        .unwrap()
        .len(),
        1
    );
}
#[tokio::test]
async fn failed_reply_preparation_rolls_back_the_write() {
    let pool = database().await;
    let p = project(&pool).await;
    let created = tasks::create(&pool, input(&p, "Original"), &now(true))
        .await
        .unwrap();
    let id = created["task"]["id"].as_str().unwrap();
    sqlx::raw_sql("ALTER TABLE alerts RENAME TO hidden_alerts")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        tasks::update(&pool, id, json!({"title":"Must roll back"}), 0, &now(true))
            .await
            .is_err()
    );
    let stored = one(
        &mut *pool.acquire().await.unwrap(),
        "SELECT title,revision FROM tasks WHERE id=?",
        vec![json!(id)],
    )
    .await
    .unwrap();
    assert_eq!(stored["title"], "Original");
    assert_eq!(stored["revision"], 0);
}

#[tokio::test]
async fn filtered_anchors_keep_hidden_order_and_noop_keeps_revision() {
    let pool = database().await;
    let p = project(&pool).await;
    let clock = now(true);
    let mut ids = vec![];
    for title in ["Visible A", "Hidden B", "Visible C", "Hidden D"] {
        let d = tasks::create(&pool, input(&p, title), &clock)
            .await
            .unwrap();
        ids.push(d["task"]["id"].as_str().unwrap().to_string());
    }
    let unchanged = tasks::move_task(&pool, &ids[3], "backlog", None, 0, &clock)
        .await
        .unwrap();
    assert_eq!(unchanged["task"]["revision"], 0);
    tasks::move_task(&pool, &ids[0], "backlog", Some(&ids[2]), 0, &clock)
        .await
        .unwrap();
    let order = rows(
        &mut *pool.acquire().await.unwrap(),
        "SELECT id FROM tasks ORDER BY sort_order,id",
        vec![],
    )
    .await
    .unwrap();
    assert_eq!(
        order
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![&ids[1], &ids[0], &ids[2], &ids[3]]
    );
    tasks::move_task(&pool, &ids[0], "backlog", None, 1, &clock)
        .await
        .unwrap();
    let order = rows(
        &mut *pool.acquire().await.unwrap(),
        "SELECT id FROM tasks ORDER BY sort_order,id",
        vec![],
    )
    .await
    .unwrap();
    assert_eq!(
        order
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![&ids[1], &ids[2], &ids[3], &ids[0]]
    );
}

#[tokio::test]
async fn project_edit_preserves_summary_fields_and_reorders_at_boundaries() {
    let pool = database().await;
    let first = project(&pool).await;
    let second = project(&pool).await;
    execute(&mut *pool.acquire().await.unwrap(),"UPDATE projects SET manager_name='Owner',manager_email='owner@example.test',summary_tone='detailed',summary_send_at='17:00' WHERE id=?",vec![json!(first)]).await.unwrap();
    let edited = projects::save(
        &pool,
        Some(first.clone()),
        ProjectInput {
            name: " Renamed ".into(),
            color: "violet".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(edited["name"], "Renamed");
    assert_eq!(edited["manager_name"], "Owner");
    assert_eq!(edited["summary_tone"], "detailed");
    assert_eq!(edited["summary_send_at"], "17:00");
    let unchanged = projects::reorder(&pool, &first, "left").await.unwrap();
    assert_eq!(unchanged[0]["id"], first);
    let moved = projects::reorder(&pool, &second, "left").await.unwrap();
    assert_eq!(moved[0]["id"], second);
    assert_eq!(moved[1]["sort_order"], 1);
    assert!(projects::save(
        &pool,
        None,
        ProjectInput {
            name: " ".into(),
            color: "cyan".into()
        }
    )
    .await
    .is_err());
    assert!(projects::save(
        &pool,
        None,
        ProjectInput {
            name: "Bad color".into(),
            color: "orange".into()
        }
    )
    .await
    .is_err());
    assert_eq!(projects::list(&pool).await.unwrap().len(), 2);
}
