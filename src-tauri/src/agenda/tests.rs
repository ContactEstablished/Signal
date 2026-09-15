use super::*;
const NOW: &str = "2025-09-11T17:42:00.000Z";
async fn db() -> (tempfile::TempDir, SqlitePool, String) {
    let dir = tempfile::tempdir().unwrap();
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("agenda.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    sqlx::query("INSERT INTO projects(id,name,color,sort_order) VALUES('p','Agenda','cyan',0)")
        .execute(&p)
        .await
        .unwrap();
    (dir, p, "p".into())
}
fn draft() -> Value {
    json!({"title":"Weekly review","starts_at":"2025-09-12T13:00:12.345Z","start_local":"2025-09-12T09:00:12.345","start_offset":"-04:00","time_zone":"America/New_York","duration_min":45,"link_url":"https://example.com/meeting","agenda_md":"Discuss","notes_md":"Shared","reminder_min":15,"show_in_day":true,"task_ids":[],"repeat_rule":"weekly","repeat_until":null,"fold_policy":"earlier"})
}

fn occurrence_draft(mut d: Value) -> Value {
    for key in [
        "repeat_rule",
        "repeat_weekdays",
        "repeat_until",
        "fold_policy",
    ] {
        d.as_object_mut().unwrap().remove(key);
    }
    d
}

#[tokio::test]
async fn combined_meeting_save_is_atomic_and_replays_once() {
    let (dir, p, pid) = db().await;
    let mut d = draft();
    d["repeat_rule"] = json!("none");
    let created = apply(
        &p,
        "combined-create",
        "create",
        json!({"projectId":pid,"draft":d}),
    )
    .await
    .unwrap();
    let r = created["outcome"]["ref"].clone();
    d["title"] = json!("Saved title");
    let payload = json!({"ref":r,"expectedRevision":0,"draft":occurrence_draft(d),"occurrence":{"attendance":"attended","occurrence_notes_md":"Saved notes"}});
    sqlx::raw_sql("CREATE TRIGGER fail_attendance BEFORE UPDATE OF attendance ON meeting_occurrences BEGIN SELECT RAISE(ABORT,'injected record failure'); END;").execute(&p).await.unwrap();
    assert!(
        apply(&p, "combined-save", "edit_occurrence", payload.clone())
            .await
            .is_err()
    );
    let mut tx = p.begin().await.unwrap();
    let old = meeting_detail(&mut tx, &serde_json::from_value(r.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(old["occurrence"]["title"], "Weekly review");
    assert_eq!(old["occurrence"]["attendance"], "unmarked");
    assert_eq!(old["occurrence"]["revision"], 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agenda_requests WHERE request_id='combined-save'"
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap(),
        0
    );
    drop(tx);
    sqlx::raw_sql("DROP TRIGGER fail_attendance")
        .execute(&p)
        .await
        .unwrap();
    let saved = apply(&p, "combined-save", "edit_occurrence", payload.clone())
        .await
        .unwrap();
    assert_eq!(saved["detail"]["occurrence"]["title"], "Saved title");
    assert_eq!(saved["detail"]["occurrence"]["attendance"], "attended");
    assert_eq!(
        saved["detail"]["occurrence"]["occurrence_notes_md"],
        "Saved notes"
    );
    let replay = apply(&p, "combined-save", "edit_occurrence", payload)
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["revision"], 1);
    p.close().await;
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(dir.path().join("agenda.db")),
        )
        .await
        .unwrap();
    let mut tx = p.begin().await.unwrap();
    let reopened = meeting_detail(&mut tx, &serde_json::from_value(r).unwrap())
        .await
        .unwrap();
    assert_eq!(reopened["occurrence"]["attendance"], "attended");
    assert_eq!(reopened["occurrence"]["occurrence_notes_md"], "Saved notes");
}

#[tokio::test]
async fn combined_following_save_records_attendance_only_on_selected_occurrence() {
    let (_dir, p, pid) = db().await;
    let created = apply(
        &p,
        "following-record-create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    let r = created["outcome"]["ref"].clone();
    let second = json!({"meetingId":r["meetingId"],"occurrenceKey":"o:1"});
    apply(&p, "other-attendance", "record", json!({"ref":second,"expectedRevision":0,"attendance":"missed","occurrence_notes_md":"Other meeting notes"})).await.unwrap();
    let mut d = draft();
    d["title"] = json!("Updated series");
    let saved = apply(&p, "combined-following", "edit_following", json!({"ref":r,"expectedRevision":1,"draft":d,"occurrence":{"attendance":"attended","occurrence_notes_md":"Only this occurrence"}})).await.unwrap();
    assert_eq!(saved["detail"]["occurrence"]["attendance"], "attended");
    let mut tx = p.begin().await.unwrap();
    let other = meeting_detail(&mut tx, &serde_json::from_value(second).unwrap())
        .await
        .unwrap();
    assert_eq!(other["occurrence"]["attendance"], "missed");
    assert_eq!(
        other["occurrence"]["occurrence_notes_md"],
        "Other meeting notes"
    );
    let next = meeting_detail(
        &mut tx,
        &MeetingRef {
            meeting_id: r["meetingId"].as_str().unwrap().into(),
            occurrence_key: "o:2".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(next["occurrence"]["title"], "Updated series");
    assert_eq!(next["occurrence"]["attendance"], "unmarked");
    assert_eq!(next["occurrence"]["occurrence_notes_md"], "");
}

#[tokio::test]
async fn combined_save_rejects_invalid_attendance_before_any_schedule_change() {
    let (_dir, p, pid) = db().await;
    let created = apply(
        &p,
        "invalid-record-create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    for (index, record) in [
        json!({"attendance":"invalid","occurrence_notes_md":""}),
        json!({"attendance":"attended"}),
        json!({"attendance":"attended","occurrence_notes_md":12}),
    ]
    .into_iter()
    .enumerate()
    {
        let result=apply(&p,&format!("invalid-record-{index}"),"edit_occurrence",json!({"ref":created["outcome"]["ref"],"expectedRevision":0,"draft":occurrence_draft(draft()),"occurrence":record})).await;
        assert!(result.is_err());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT revision FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}
fn weekday_draft(days: Vec<i64>) -> Value {
    let mut d = draft();
    d["starts_at"] = json!("2025-09-15T13:00:00.000Z");
    d["start_local"] = json!("2025-09-15T09:00:00.000");
    d["repeat_weekdays"] = json!(days);
    d
}

#[test]
fn weekday_validation_rejects_empty_invalid_and_off_day_starts() {
    for days in [vec![], vec![0, 1], vec![1, 8], vec![2, 3]] {
        assert!(normalize(&weekday_draft(days), true).is_err());
    }
    assert_eq!(
        normalize(&weekday_draft(vec![3, 1, 3]), true).unwrap()["repeat_weekdays"],
        json!([1, 3])
    );
    assert_eq!(
        normalize(&draft(), true).unwrap()["repeat_weekdays"],
        json!([5])
    );
    let mut d = weekday_draft(vec![1, 3]);
    d["repeat_rule"] = json!("none");
    assert_eq!(normalize(&d, true).unwrap()["repeat_weekdays"], json!([]));
}

#[tokio::test]
async fn selected_weekdays_persist_and_project_bounded_weeks_across_dst() {
    for days in [vec![1, 2, 3, 4], vec![1, 3], vec![1, 2]] {
        let (dir, p, pid) = db().await;
        let created = apply(
            &p,
            "weekdays",
            "create",
            json!({"projectId":pid,"draft":weekday_draft(days.clone())}),
        )
        .await
        .unwrap();
        assert_eq!(created["detail"]["series"]["repeat_weekdays"], json!(days));
        p.close().await;
        let p = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(dir.path().join("agenda.db"))
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let mut tx = p.begin().await.unwrap();
        for (start, end, hour) in [
            (
                "2025-09-15T00:00:00.000Z",
                "2025-09-22T00:00:00.000Z",
                "13:00",
            ),
            (
                "2025-11-03T00:00:00.000Z",
                "2025-11-10T00:00:00.000Z",
                "14:00",
            ),
            (
                "2040-09-10T00:00:00.000Z",
                "2040-09-17T00:00:00.000Z",
                "13:00",
            ),
        ] {
            let week = occurrences(&mut tx, start, end, None, false).await.unwrap();
            assert_eq!(week.len(), days.len());
            for (index, o) in week.iter().enumerate() {
                let local = NaiveDateTime::parse_from_str(
                    o["start_local"].as_str().unwrap(),
                    "%Y-%m-%dT%H:%M:%S%.f",
                )
                .unwrap();
                assert_eq!(local.weekday().number_from_monday() as i64, days[index]);
                assert_eq!(&o["starts_at"].as_str().unwrap()[11..16], hour);
                assert_eq!(&o["start_local"].as_str().unwrap()[11..16], "09:00");
            }
        }
        let week = snapshot(
            &mut tx,
            &AgendaQuery {
                date: "2025-09-15".into(),
                time_zone: "America/New_York".into(),
                project_id: Some("p".into()),
            },
            "2025-09-15T12:00:00.000Z",
            true,
        )
        .await
        .unwrap();
        assert_eq!(
            week["days"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["meetings"].as_array().unwrap().len())
                .sum::<usize>(),
            days.len()
        );
        let today = snapshot(
            &mut tx,
            &AgendaQuery {
                date: "2025-09-15".into(),
                time_zone: "America/New_York".into(),
                project_id: None,
            },
            "2025-09-15T12:00:00.000Z",
            false,
        )
        .await
        .unwrap();
        assert_eq!(today["meetings"].as_array().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn weekday_changes_preserve_other_occurrences_notes_and_cancellation() {
    let (_dir, p, pid) = db().await;
    let created = apply(
        &p,
        "create-days",
        "create",
        json!({"projectId":pid,"draft":weekday_draft(vec![1, 2, 3, 4])}),
    )
    .await
    .unwrap();
    let id = created["outcome"]["ref"]["meetingId"].clone();
    let second = json!({"meetingId":id,"occurrenceKey":"o:1"});
    let fourth = json!({"meetingId":id,"occurrenceKey":"o:3"});
    apply(&p, "record-days", "record", json!({"ref":fourth,"expectedRevision":0,"attendance":"attended","occurrence_notes_md":"Preserve this decision"})).await.unwrap();
    let mut d = weekday_draft(vec![2, 4]);
    d["start_local"] = json!("2025-09-16T09:00:00.000");
    d["starts_at"] = json!("2025-09-16T13:00:00.000Z");
    let changed = apply(
        &p,
        "change-days",
        "edit_following",
        json!({"ref":second,"expectedRevision":1,"draft":d}),
    )
    .await
    .unwrap();
    assert_eq!(
        changed["detail"]["series"]["repeat_weekdays"],
        json!([2, 4])
    );
    let mut tx = p.begin().await.unwrap();
    let saved = meeting_detail(&mut tx, &serde_json::from_value(fourth.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        saved["occurrence"]["start_local"],
        "2025-09-18T09:00:00.000"
    );
    assert_eq!(
        saved["occurrence"]["occurrence_notes_md"],
        "Preserve this decision"
    );
    let first = meeting_detail(
        &mut tx,
        &serde_json::from_value(created["outcome"]["ref"].clone()).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        first["occurrence"]["start_local"],
        "2025-09-15T09:00:00.000"
    );
    drop(tx);
    apply(
        &p,
        "cancel-days",
        "remove",
        json!({"ref":fourth,"expectedRevision":2,"scope":"occurrence"}),
    )
    .await
    .unwrap();
    let mut tx = p.begin().await.unwrap();
    let root = recurrence::root(&mut tx, id.as_str().unwrap())
        .await
        .unwrap();
    assert!(recurrence::resolve(&mut tx, &root, 3)
        .await
        .unwrap()
        .is_none());
    assert!(recurrence::resolve(&mut tx, &root, 4)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn weekday_recurrence_end_and_dst_gap_keep_stable_ordinals() {
    let (_dir, p, pid) = db().await;
    let mut d = weekday_draft(vec![1, 7]);
    d["start_local"] = json!("2025-03-03T02:30:00.000");
    d["starts_at"] = json!("2025-03-03T07:30:00.000Z");
    d["start_offset"] = json!("-05:00");
    d["repeat_until"] = json!("2025-03-10");
    let created = apply(&p, "gap-days", "create", json!({"projectId":pid,"draft":d}))
        .await
        .unwrap();
    let mut tx = p.begin().await.unwrap();
    let root = recurrence::root(
        &mut tx,
        created["outcome"]["ref"]["meetingId"].as_str().unwrap(),
    )
    .await
    .unwrap();
    assert!(recurrence::resolve(&mut tx, &root, 1)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        recurrence::resolve(&mut tx, &root, 2)
            .await
            .unwrap()
            .unwrap()["starts_at"],
        "2025-03-10T06:30:00.000Z"
    );
    assert!(recurrence::resolve(&mut tx, &root, 3)
        .await
        .unwrap()
        .is_none());
    let (_, warnings) = recurrence::project(
        &mut tx,
        "2025-03-09T05:00:00.000Z",
        "2025-03-10T04:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(warnings.len(), 1);
}

#[tokio::test]
async fn following_weekday_edit_preserves_every_past_meeting_in_the_week() {
    let (_dir, p, pid) = db().await;
    let mut d = weekday_draft(vec![1, 2, 3, 4]);
    d["start_local"] = json!("2025-09-08T09:00:00.000");
    d["starts_at"] = json!("2025-09-08T13:00:00.000Z");
    let created = apply(
        &p,
        "past-days",
        "create",
        json!({"projectId":pid,"draft":d}),
    )
    .await
    .unwrap();
    let r = created["outcome"]["ref"].clone();
    d["start_local"] = json!("2025-09-08T10:00:00.000");
    d["starts_at"] = json!("2025-09-08T14:00:00.000Z");
    d["notes_md"] = json!("Future agenda");
    apply(
        &p,
        "past-days-edit",
        "edit_following",
        json!({"ref":r,"expectedRevision":0,"draft":d}),
    )
    .await
    .unwrap();
    let mut tx = p.begin().await.unwrap();
    let root = recurrence::root(&mut tx, r["meetingId"].as_str().unwrap())
        .await
        .unwrap();
    for n in 0..4 {
        let o = recurrence::resolve(&mut tx, &root, n)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&o["start_local"].as_str().unwrap()[11..16], "09:00");
        assert_eq!(o["notes_md"], "Shared");
    }
    let future = recurrence::resolve(&mut tx, &root, 4)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(future["start_local"], "2025-09-15T10:00:00.000");
    assert_eq!(future["notes_md"], "Future agenda");
}

#[tokio::test]
async fn weekday_migration_keeps_legacy_schedule_identity_and_precision() {
    let dir = tempfile::tempdir().unwrap();
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("old.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for m in crate::db::migrations()
        .into_iter()
        .filter(|m| m.version < 6)
    {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    sqlx::raw_sql("INSERT INTO projects(id,name,color) VALUES('p','Keep','cyan');
        INSERT INTO meetings(id,project_id,title,starts_at,duration_min,repeat_rule,time_zone,show_in_day,active_version_id) VALUES('m','p','Old weekly','2025-11-02T06:30:17.123Z',30,'weekly','America/New_York',1,'v');
        INSERT INTO meeting_versions VALUES('v','m','2025-09-11T00:00:00.000Z');
        INSERT INTO meeting_segments(id,version_id,from_ordinal,anchor_local,anchor_utc,time_zone,repeat_rule,fold_policy,title,duration_min,agenda_md,notes_md,reminder_min,show_in_day) VALUES('s','v',0,'2025-11-02T01:30:17.123','2025-11-02T06:30:17.123Z','America/New_York','weekly','later','Old weekly',30,'','Keep notes',15,1);")
        .execute(&p).await.unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0006_meeting_weekdays.sql"))
        .execute(&p)
        .await
        .unwrap();
    let mut tx = p.begin().await.unwrap();
    let root = recurrence::root(&mut tx, "m").await.unwrap();
    let first = recurrence::resolve(&mut tx, &root, 0)
        .await
        .unwrap()
        .unwrap();
    let second = recurrence::resolve(&mut tx, &root, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first["starts_at"], "2025-11-02T06:30:17.123Z");
    assert_eq!(second["starts_at"], "2025-11-09T06:30:17.123Z");
    assert_eq!(second["ref"]["occurrenceKey"], "o:1");
    assert_eq!(second["notes_md"], "Keep notes");
    assert_eq!(
        meeting_detail(
            &mut tx,
            &MeetingRef {
                meeting_id: "m".into(),
                occurrence_key: "o:1".into()
            }
        )
        .await
        .unwrap()["series"]["repeat_weekdays"],
        json!([7])
    );
}
async fn apply(p: &SqlitePool, id: &str, action: &str, payload: Value) -> Result<Value> {
    let change = json!({"action":action,"payload":payload});
    let fp = if ["edit_following", "remove"].contains(&action) {
        let mut tx = p.begin().await?;
        Some(
            preview_into(&mut tx, &change, NOW).await?["fingerprint"]
                .as_str()
                .unwrap()
                .into(),
        )
    } else {
        None
    };
    apply_into(
        p,
        AgendaInput {
            request_id: id.into(),
            change,
            expected_fingerprint: fp,
        },
        NOW,
    )
    .await
}
#[tokio::test]
async fn recurrence_override_cutoff_and_replay() {
    let (_dir, p, pid) = db().await;
    let c = apply(
        &p,
        "create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    let r = c["outcome"]["ref"].clone();
    let mut tx = p.begin().await.unwrap();
    let week = occurrences(
        &mut tx,
        "2025-09-19T00:00:00.000Z",
        "2025-09-20T00:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(week.len(), 1);
    assert_eq!(week[0]["starts_at"], "2025-09-19T13:00:12.345Z");
    let second = week[0]["ref"].clone();
    drop(tx);
    let marked=apply(&p,"mark","record",json!({"ref":second,"expectedRevision":0,"attendance":"attended","occurrence_notes_md":"Decisions"})).await.unwrap();
    assert_eq!(
        marked["detail"]["occurrence"]["occurrence_notes_md"],
        "Decisions"
    );
    let mut d = draft();
    d["repeat_rule"] = json!("daily");
    d["title"] = json!("Changed");
    apply(
        &p,
        "following",
        "edit_following",
        json!({"ref":r,"expectedRevision":1,"draft":d}),
    )
    .await
    .unwrap();
    let mut tx = p.begin().await.unwrap();
    let detail = meeting_detail(&mut tx, &serde_json::from_value(second.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        detail["occurrence"]["starts_at"],
        "2025-09-19T13:00:12.345Z"
    );
    assert_eq!(detail["occurrence"]["attendance"], "attended");
    drop(tx);
    apply(
        &p,
        "cancel",
        "remove",
        json!({"ref":second,"expectedRevision":2,"scope":"following"}),
    )
    .await
    .unwrap();
    let replay = apply(
        &p,
        "create",
        "create",
        json!({"projectId":"p","draft":draft()}),
    )
    .await
    .unwrap();
    assert!(replay["replayed"].as_bool().unwrap());
    let mut tx = p.begin().await.unwrap();
    assert!(occurrences(
        &mut tx,
        "2025-09-19T00:00:00.000Z",
        "2025-09-20T00:00:00.000Z",
        None,
        false
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn legacy_initialization_preserves_instant_and_zone() {
    let (_d, p, _) = db().await;
    sqlx::query("INSERT INTO meetings(id,project_id,title,starts_at,duration_min,repeat_rule) VALUES('old','p','Old','2025-11-02T06:30:17.123Z',30,'weekly')").execute(&p).await.unwrap();
    initialize_into(&p, "America/New_York", NOW).await.unwrap();
    initialize_into(&p, "Europe/London", NOW).await.unwrap();
    let mut tx = p.begin().await.unwrap();
    let m = recurrence::root(&mut tx, "old").await.unwrap();
    assert_eq!(m["starts_at"], "2025-11-02T06:30:17.123Z");
    assert_eq!(m["time_zone"], "America/New_York");
    let o = recurrence::resolve(&mut tx, &m, 0).await.unwrap().unwrap();
    assert_eq!(o["starts_at"], m["starts_at"]);
    assert_eq!(o["start_offset"], "-05:00");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meeting_versions")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        1
    );
}
#[test]
fn gap_fold_validation() {
    let mut d = draft();
    d["start_local"] = json!("2025-03-09T02:30:00.000");
    assert!(normalize(&d, true).is_err());
    d["start_local"] = json!("2025-11-02T01:30:00.000");
    d["starts_at"] = json!("2025-11-02T06:30:00.000Z");
    d["start_offset"] = json!("-05:00");
    assert!(normalize(&d, true).is_ok());
    d["start_offset"] = json!("-04:00");
    assert!(normalize(&d, true).is_err());
}
#[tokio::test]
async fn late_receipt_failure_rolls_back_and_exact_retry_does_not_duplicate() {
    let (_dir, p, pid) = db().await;
    sqlx::raw_sql("CREATE TRIGGER fail_receipt BEFORE INSERT ON agenda_requests BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").execute(&p).await.unwrap();
    let payload = json!({"projectId":pid,"draft":draft()});
    assert!(apply(&p, "same", "create", payload.clone()).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    sqlx::query("DROP TRIGGER fail_receipt")
        .execute(&p)
        .await
        .unwrap();
    let first = apply(&p, "same", "create", payload.clone()).await.unwrap();
    let replay = apply(&p, "same", "create", payload).await.unwrap();
    assert_eq!(first["outcome"], replay["outcome"]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn due_move_preserves_precision_and_every_unrelated_field() {
    let (_dir, p, pid) = db().await;
    let d=crate::workspace::tasks::create(&p,json!({"project_id":pid,"title":"Deadline","status":"todo","priority":"high","due_at":"2025-09-11T19:30:12.345Z","estimate_h":2,"notes_md":"Keep","subtasks":[],"tags":[],"alerts":[],"attachments":[]}),NOW).await.unwrap();
    let id = d["task"]["id"].as_str().unwrap();
    sqlx::query("INSERT INTO blocks(id,date,start_min,end_min,kind,task_id) VALUES('block','2025-09-11',540,600,'task',?)").bind(id).execute(&p).await.unwrap();
    let payload = json!({"taskId":id,"dueAt":"2025-09-12T19:30:12.345Z","expectedRevision":0});
    let r = apply(&p, "due", "move_due", payload.clone()).await.unwrap();
    let mut before = d["task"].clone();
    let mut after = r["changed_details"][0]["task"].clone();
    assert_eq!(after["due_at"], payload["dueAt"]);
    assert_eq!(after["revision"], 1);
    for k in ["due_at", "revision", "updated_at"] {
        before.as_object_mut().unwrap().remove(k);
        after.as_object_mut().unwrap().remove(k);
    }
    assert_eq!(before, after);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT date FROM blocks WHERE id='block'")
            .fetch_one(&p)
            .await
            .unwrap(),
        "2025-09-11"
    );
    assert_eq!(
        apply(&p, "due", "move_due", payload).await.unwrap()["changed_details"][0]["task"]
            ["revision"],
        1
    );
}

#[tokio::test]
async fn receipt_readback_failure_stays_uncertain_and_replays_without_duplicate_creation() {
    let (_dir, p, pid) = db().await;
    let payload = json!({"projectId":pid,"draft":draft()});
    let first = apply(&p, "lost-create-reply", "create", payload.clone())
        .await
        .unwrap();
    // Simulate a readback failure after the creation has already committed.
    sqlx::query("ALTER TABLE subtasks RENAME TO unavailable_subtasks")
        .execute(&p)
        .await
        .unwrap();
    let failure = apply(&p, "lost-create-reply", "create", payload.clone())
        .await
        .unwrap_err();
    assert_eq!(failure.code, "UnknownOutcome");
    sqlx::query("ALTER TABLE unavailable_subtasks RENAME TO subtasks")
        .execute(&p)
        .await
        .unwrap();
    let recovered = apply(&p, "lost-create-reply", "create", payload)
        .await
        .unwrap();
    assert_eq!(recovered["outcome"], first["outcome"]);
    assert_eq!(recovered["replayed"], true);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn moved_exception_queries_actual_time_and_hidden_frees_planner() {
    let (_dir, p, pid) = db().await;
    let c = apply(
        &p,
        "create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    let mut r = c["outcome"]["ref"].clone();
    r["occurrenceKey"] = json!("o:20");
    let mut d = draft();
    for k in ["repeat_rule", "repeat_until", "fold_policy"] {
        d.as_object_mut().unwrap().remove(k);
    }
    d["starts_at"] = json!("2025-09-14T03:45:00.000Z");
    d["start_local"] = json!("2025-09-13T23:45:00.000");
    d["duration_min"] = json!(90);
    d["show_in_day"] = json!(false);
    apply(
        &p,
        "edit",
        "edit_occurrence",
        json!({"ref":r,"expectedRevision":0,"draft":d}),
    )
    .await
    .unwrap();
    let mut tx = p.begin().await.unwrap();
    let visible = occurrences(
        &mut tx,
        "2025-09-14T04:00:00.000Z",
        "2025-09-15T04:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0]["ref"], r);
    assert!(occurrences(
        &mut tx,
        "2025-09-14T04:00:00.000Z",
        "2025-09-15T04:00:00.000Z",
        None,
        true
    )
    .await
    .unwrap()
    .is_empty());
}
#[tokio::test]
async fn one_off_delete_preserves_task_and_receipt_after_reopen() {
    let (dir, p, pid) = db().await;
    let t=crate::workspace::tasks::create(&p,json!({"project_id":pid,"title":"Linked","status":"todo","priority":"medium","due_at":null,"estimate_h":null,"notes_md":"","subtasks":[],"tags":[],"alerts":[],"attachments":[]}),NOW).await.unwrap();
    let mut d = draft();
    d["repeat_rule"] = json!("none");
    d["task_ids"] = json!([t["task"]["id"]]);
    let payload = json!({"projectId":"p","draft":d});
    let c = apply(&p, "create", "create", payload.clone())
        .await
        .unwrap();
    apply(
        &p,
        "remove",
        "remove",
        json!({"ref":c["outcome"]["ref"],"expectedRevision":0,"scope":"one_off"}),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
            .fetch_one(&p)
            .await
            .unwrap(),
        1
    );
    p.close().await;
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.path().join("agenda.db"))
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let replay = apply(&p, "create", "create", payload).await.unwrap();
    assert_eq!(replay["detail"], Value::Null);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn daily_dst_gap_skips_without_renumbering_and_fold_policy_is_stable() {
    let (_dir, p, pid) = db().await;
    let mut d = draft();
    d["start_local"] = json!("2025-03-08T02:30:00.000");
    d["starts_at"] = json!("2025-03-08T07:30:00.000Z");
    d["start_offset"] = json!("-05:00");
    d["repeat_rule"] = json!("daily");
    let c = apply(&p, "gap", "create", json!({"projectId":pid,"draft":d}))
        .await
        .unwrap();
    let mut tx = p.begin().await.unwrap();
    let m = recurrence::root(&mut tx, c["outcome"]["ref"]["meetingId"].as_str().unwrap())
        .await
        .unwrap();
    assert!(recurrence::resolve(&mut tx, &m, 1).await.unwrap().is_none());
    assert_eq!(
        recurrence::resolve(&mut tx, &m, 2).await.unwrap().unwrap()["starts_at"],
        "2025-03-10T06:30:00.000Z"
    );
    let (_, warnings) = recurrence::project(
        &mut tx,
        "2025-03-09T05:00:00.000Z",
        "2025-03-10T04:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert!(warnings.iter().any(|s| s.contains("2025-03-09")));
}
#[tokio::test]
async fn same_time_meetings_remain_distinct_and_future_week_uses_clock_today() {
    let (_dir, p, pid) = db().await;
    for id in ["a", "b"] {
        apply(&p, id, "create", json!({"projectId":pid,"draft":draft()}))
            .await
            .unwrap();
    }
    let mut tx = p.begin().await.unwrap();
    let ms = occurrences(
        &mut tx,
        "2025-09-12T00:00:00.000Z",
        "2025-09-13T00:00:00.000Z",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(ms.len(), 2);
    assert_ne!(ms[0]["id"], ms[1]["id"]);
    let q = AgendaQuery {
        date: "2027-01-01".into(),
        time_zone: "America/New_York".into(),
        project_id: Some(pid),
    };
    let week = snapshot(&mut tx, &q, NOW, true).await.unwrap();
    assert_eq!(week["today"], "2025-09-11");
    assert_eq!(week["week_start"], "2026-12-28");
    let today = snapshot(&mut tx, &q, NOW, false).await.unwrap();
    assert_eq!(today["query"]["date"], "2025-09-11");
}

#[tokio::test]
async fn converted_series_keeps_future_override_and_cannot_be_deleted_as_one_off() {
    let (_dir, p, pid) = db().await;
    let c = apply(
        &p,
        "create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    let r = c["outcome"]["ref"].clone();
    let mut future = r.clone();
    future["occurrenceKey"] = json!("o:3");
    apply(&p,"record","record",json!({"ref":future,"expectedRevision":0,"attendance":"missed","occurrence_notes_md":"Keep this history"})).await.unwrap();
    let mut d = draft();
    d["repeat_rule"] = json!("none");
    apply(
        &p,
        "end",
        "edit_following",
        json!({"ref":r,"expectedRevision":1,"draft":d}),
    )
    .await
    .unwrap();
    let mut tx = p.begin().await.unwrap();
    let detail = meeting_detail(&mut tx, &serde_json::from_value(r.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(detail["series"]["is_recurring"], true);
    let retained = meeting_detail(&mut tx, &serde_json::from_value(future).unwrap())
        .await
        .unwrap();
    assert_eq!(
        retained["occurrence"]["occurrence_notes_md"],
        "Keep this history"
    );
    drop(tx);
    assert!(apply(
        &p,
        "bad-delete",
        "remove",
        json!({"ref":r,"expectedRevision":2,"scope":"one_off"})
    )
    .await
    .is_err());
}
#[tokio::test]
async fn following_cancellation_cannot_erase_actual_past_and_stale_review_is_rejected() {
    let (_dir, p, pid) = db().await;
    let c = apply(
        &p,
        "create",
        "create",
        json!({"projectId":pid,"draft":draft()}),
    )
    .await
    .unwrap();
    let r = c["outcome"]["ref"].clone();
    let change =
        json!({"action":"remove","payload":{"ref":r,"expectedRevision":0,"scope":"following"}});
    let mut tx = p.begin().await.unwrap();
    let preview = preview_into(&mut tx, &change, NOW).await.unwrap();
    drop(tx);
    let input = AgendaInput {
        request_id: "late".into(),
        change: change.clone(),
        expected_fingerprint: Some(preview["fingerprint"].as_str().unwrap().into()),
    };
    assert!(apply_into(&p, input, "2025-09-12T15:00:00.000Z")
        .await
        .is_err());
    apply(&p,"record","record",json!({"ref":r,"expectedRevision":0,"attendance":"attended","occurrence_notes_md":"History"})).await.unwrap();
    let stale = AgendaInput {
        request_id: "stale".into(),
        change,
        expected_fingerprint: Some(preview["fingerprint"].as_str().unwrap().into()),
    };
    assert_eq!(
        apply_into(&p, stale, NOW).await.unwrap_err().code,
        "Conflict"
    );
}
#[tokio::test]
async fn cross_project_task_deletion_detaches_all_history_and_invalidates_review() {
    let (_dir, p, pid) = db().await;
    sqlx::query("INSERT INTO projects(id,name,color,sort_order) VALUES('q','Other','lime',1)")
        .execute(&p)
        .await
        .unwrap();
    let t=crate::workspace::tasks::create(&p,json!({"project_id":"q","title":"Foreign","status":"todo","priority":"medium","due_at":null,"estimate_h":null,"notes_md":"","subtasks":[],"tags":[],"alerts":[],"attachments":[]}),NOW).await.unwrap();
    let id = t["task"]["id"].as_str().unwrap();
    let mut d = draft();
    d["task_ids"] = json!([id]);
    let c = apply(&p, "create", "create", json!({"projectId":pid,"draft":d}))
        .await
        .unwrap();
    let r = c["outcome"]["ref"].clone();
    apply(&p,"record","record",json!({"ref":r,"expectedRevision":0,"attendance":"attended","occurrence_notes_md":"Keep notes"})).await.unwrap();
    let target = DeletionTarget {
        kind: "task".into(),
        id: id.into(),
    };
    let mut tx = p.begin().await.unwrap();
    let graph = crate::workspace::delete::graph(&mut tx, &target)
        .await
        .unwrap();
    assert_eq!(graph["agenda"].as_array().unwrap().len(), 1);
    drop(tx);
    let preview = crate::workspace::delete::preview(&p, &target)
        .await
        .unwrap();
    crate::workspace::delete::remove(&p, &target, preview["fingerprint"].as_str().unwrap(), NOW)
        .await
        .unwrap();
    let mut tx = p.begin().await.unwrap();
    let detail = meeting_detail(&mut tx, &serde_json::from_value(r).unwrap())
        .await
        .unwrap();
    assert!(detail["linked_tasks"].as_array().unwrap().is_empty());
    assert_eq!(detail["occurrence"]["occurrence_notes_md"], "Keep notes");
    assert!(detail["occurrence"]["task_ids"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(detail["occurrence"]["revision"], 2);
}
