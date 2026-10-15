use super::{data::*, engine, images};
use crate::workspace::projects;
use serde_json::json;
use sqlx::SqlitePool;
const NOW: &str = "2026-09-28T16:00:00.000Z";
async fn database() -> SqlitePool {
    let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
    for m in crate::db::migrations() {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    p
}
fn draft(kind: &str) -> Draft {
    serde_json::from_value(json!({"id":kind,"source_kind":kind,"revision":0,"recorded_at":NOW,"time_zone":"America/New_York","transcript":"Make meetings and tasks for Atlas","projects":[],"candidates":[]})).unwrap()
}
fn task(id: &str, p: &str) -> Candidate {
    serde_json::from_value(json!({"id":id,"kind":"task","selected":true,"project_id":p,"proposal_id":null,"title":id,"notes":"","subtasks":[],"due_at":null,"priority":"medium","estimate_h":null,"source":"Top row","warning":"","duplicate_ok":false,"created_id":null})).unwrap()
}
fn meeting(id: &str, p: &str) -> Candidate {
    let mut c = task(id, p);
    c.kind = "meeting".into();
    c.meeting = Some(
        images::meeting_suggestion(
            &json!({"start_local":"2026-09-29T10:00:00","duration_min":30}),
            id,
            "America/New_York",
        )
        .unwrap(),
    );
    c
}
fn request(d: &Draft) -> Accept {
    Accept {
        request_id: format!("accept-{}", d.revision),
        draft_id: d.id.clone(),
        revision: d.revision,
        candidate_ids: d
            .candidates
            .iter()
            .filter(|c| c.created_id.is_none() && c.selected)
            .map(|c| c.id.clone())
            .collect(),
    }
}
fn png() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(30, 20)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .unwrap();
    cursor.into_inner()
}
#[test]
fn clipboard_pixels_preserve_color_alpha_and_image_limits() {
    let rgba = [255, 0, 0, 255, 0, 100, 200, 128];
    let bytes = images::clipboard_png(&rgba, 2, 1).unwrap();
    assert_eq!(
        images::validate_image(&bytes).unwrap(),
        ("image/png".into(), 2, 1)
    );
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().to_rgba8().as_raw(),
        &rgba
    );
    assert!(images::clipboard_png(&rgba, 0, 1).is_err());
    assert!(images::clipboard_png(&rgba, 5001, 5000).is_err());
    assert!(images::clipboard_png(&rgba[..7], 2, 1).is_err());
}

#[tokio::test]
async fn image_without_instructions_keeps_meeting_details_until_project_is_chosen() {
    let p = database().await;
    let project = projects::save(
        &p,
        None,
        crate::workspace::models::ProjectInput {
            name: "Atlas".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap();
    let project_id = project["id"].as_str().unwrap().to_owned();
    let mut d = draft("image");
    d.transcript.clear();
    let body = images::suggestion_body(
        &d,
        &[project.clone()],
        "configured-model",
        "data:image/png;base64,abc",
    );
    let context: serde_json::Value =
        serde_json::from_str(body["messages"][1]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(context["prompt"]
        .as_str()
        .unwrap()
        .contains("Identify meetings and tasks"));
    assert_eq!(context["projects"][0]["id"], project_id);
    let response = json!({"tasks":[{
        "kind":"meeting", "project_id":null, "project_name":"", "title":"Design review",
        "notes":"", "subtasks":[], "source":"Invitation: September 29, 10:00–10:45 AM",
        "meeting":{"start_local":"2026-09-29T10:00:00", "duration_min":45, "link_url":"https://example.com/meeting", "agenda_md":"Review the designs", "repeat_rule":"none"}
    }]}).to_string();
    let d = engine::parse_suggestions(d, &[project], &response).unwrap();
    assert!(d.projects.is_empty());
    assert!(d.candidates[0].project_id.is_none());
    let mut d = save(&p, d).await.unwrap();
    assert!(accept(&p, request(&d), NOW).await.is_err());
    d.candidates[0].project_id = Some(project_id);
    let d = save(&p, d).await.unwrap();
    let receipt = accept(&p, request(&d), NOW).await.unwrap();
    assert_eq!(receipt["created"][0]["kind"], "meeting");
    let stored: (String, i64, String, String) =
        sqlx::query_as("SELECT starts_at,duration_min,link_url,agenda_md FROM meetings WHERE id=?")
            .bind(receipt["created"][0]["id"].as_str().unwrap())
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(
        stored,
        (
            "2026-09-29T14:00:00.000Z".into(),
            45,
            "https://example.com/meeting".into(),
            "Review the designs".into()
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}

#[test]
fn image_formats_bounds_and_corruption() {
    for format in [
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::WebP,
    ] {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(30, 20)
            .write_to(&mut bytes, format)
            .unwrap();
        assert_eq!(images::validate_image(bytes.get_ref()).unwrap().1, 30);
    }
    assert!(images::validate_image(b"not an image").is_err());
    assert!(images::validate_image(&vec![0; images::MAX_BYTES + 1]).is_err());
    assert!(images::validate_image(&png()[..40]).is_err());
    let mut big = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_luma8(5001, 5000)
        .write_to(&mut big, image::ImageFormat::Png)
        .unwrap();
    assert!(images::validate_image(big.get_ref()).is_err());
}
#[tokio::test]
async fn source_isolation_replacement_cleanup_and_stale_revision() {
    let p = database().await;
    let voice = save(&p, draft("voice")).await.unwrap();
    let mut collision = draft("image");
    collision.id = voice.id.clone();
    assert!(save(&p, collision).await.is_err());
    let d = save(&p, draft("image")).await.unwrap();
    let mut d = images::set_image_into(&p, &d.id, d.revision, Some(png()))
        .await
        .unwrap();
    let original = d.image_id.clone();
    d.candidates.push(task("pending", "p"));
    d = save(&p, d).await.unwrap();
    assert!(images::set_image_into(&p, &d.id, d.revision - 1, None)
        .await
        .is_err());
    let d = images::set_image_into(&p, &d.id, d.revision, Some(png()))
        .await
        .unwrap();
    assert_ne!(d.image_id, original);
    assert!(d.candidates.is_empty());
    assert_eq!(load(&p).await.unwrap().unwrap().id, voice.id);
    assert_eq!(
        load_source(&p, "image").await.unwrap().unwrap().image_id,
        d.image_id
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM intake_images")
            .fetch_one(&p)
            .await
            .unwrap(),
        1
    );
    discard(&p, &d.id, d.revision).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM intake_images")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    assert!(load(&p).await.unwrap().is_some());
}

#[tokio::test]
async fn migration_preserves_legacy_voice_and_image_bytes_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("test.db"))
        .create_if_missing(true);
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let migrations = crate::db::migrations();
    for m in migrations.iter().filter(|m| m.version < 8) {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    let mut legacy = json!(draft("voice"));
    legacy.as_object_mut().unwrap().remove("source_kind");
    legacy.as_object_mut().unwrap().remove("image_id");
    sqlx::query("INSERT INTO intake_drafts VALUES('voice',0,?,?)")
        .bind(legacy.to_string())
        .bind(NOW)
        .execute(&p)
        .await
        .unwrap();
    sqlx::query("INSERT INTO settings(key,value) VALUES('voice_pending','legacy recovery')")
        .execute(&p)
        .await
        .unwrap();
    for m in migrations.iter().filter(|m| m.version >= 8) {
        sqlx::raw_sql(m.sql).execute(&p).await.unwrap();
    }
    assert_eq!(load(&p).await.unwrap().unwrap().source_kind, "voice");
    let d = save(&p, draft("image")).await.unwrap();
    let d = images::set_image_into(&p, &d.id, d.revision, Some(png()))
        .await
        .unwrap();
    p.close().await;
    let p = SqlitePool::connect_with(options).await.unwrap();
    assert_eq!(
        load_source(&p, "image").await.unwrap().unwrap().image_id,
        d.image_id
    );
    assert_eq!(
        sqlx::query_scalar::<_, Vec<u8>>("SELECT bytes FROM intake_images")
            .fetch_one(&p)
            .await
            .unwrap(),
        png()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key='voice_pending'")
            .fetch_one(&p)
            .await
            .unwrap(),
        "legacy recovery"
    );
    p.close().await;
}
#[tokio::test]
async fn mixed_creation_is_atomic_replay_safe_and_detects_meeting_duplicates() {
    let p = database().await;
    let project = projects::save(
        &p,
        None,
        crate::workspace::models::ProjectInput {
            name: "Atlas".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap();
    let id = project["id"].as_str().unwrap();
    let mut d = draft("image");
    d.candidates = vec![task("Write notes", id), meeting("Review", id)];
    d.candidates[1].meeting.as_mut().unwrap()["duration_min"] = json!(0);
    let d = save(&p, d).await.unwrap();
    assert!(accept(&p, request(&d), NOW).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    let mut d = load_source(&p, "image").await.unwrap().unwrap();
    d.candidates[1].meeting.as_mut().unwrap()["duration_min"] = json!(30);
    let d = save(&p, d).await.unwrap();
    let req = request(&d);
    let receipt = accept(&p, req.clone(), NOW).await.unwrap();
    assert_eq!(receipt["created"].as_array().unwrap().len(), 2);
    assert_eq!(accept(&p, req.clone(), NOW).await.unwrap(), receipt);
    let mut d = load_source(&p, "image").await.unwrap().unwrap();
    let mut duplicate = meeting("second", id);
    duplicate.title = " Review ".into();
    d.candidates.push(duplicate);
    let d = save(&p, d).await.unwrap();
    assert!(accept(&p, request(&d), NOW)
        .await
        .unwrap_err()
        .message
        .contains("duplicate meeting"));
    let mut d = d;
    d.candidates.last_mut().unwrap().duplicate_ok = true;
    let d = save(&p, d).await.unwrap();
    accept(&p, request(&d), NOW).await.unwrap();
    let d = load_source(&p, "image").await.unwrap().unwrap();
    discard(&p, &d.id, d.revision).await.unwrap();
    assert_eq!(accept(&p, req, NOW).await.unwrap(), receipt);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meetings")
            .fetch_one(&p)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM blocks")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}
#[test]
fn interpretation_keeps_missing_and_ambiguous_meeting_fields_for_review() {
    let m = images::meeting_suggestion(
        &json!({"start_local":"2026-11-01T01:30:00"}),
        "Review",
        "America/New_York",
    )
    .unwrap();
    assert_eq!(m["starts_at"], "");
    assert_eq!(m["duration_min"], 0);
    let m = images::meeting_suggestion(
        &json!({"start_local":"2026-11-01T01:30:00","start_offset":"-05:00","duration_min":30}),
        "Review",
        "America/New_York",
    )
    .unwrap();
    assert_eq!(m["starts_at"], "2026-11-01T06:30:00.000Z");
    let input = json!({"tasks":[{"kind":"meeting","title":"Review","project_name":"New launch","source":"Calendar row","subtasks":[],"meeting":{"duration_min":null}}]});
    let d = engine::parse_suggestions(draft("image"), &[], &input.to_string()).unwrap();
    assert_eq!(d.candidates[0].kind, "meeting");
    assert!(!d.projects[0].approved);
    assert_eq!(d.candidates[0].meeting.as_ref().unwrap()["starts_at"], "");
    let body = images::suggestion_body(&d, &[], "configured-model", "data:image/png;base64,abc");
    assert_eq!(body["messages"][1]["content"][1]["type"], "image_url");
    assert!(body.to_string().contains("Make meetings and tasks"));
    assert!(engine::parse_suggestions(d, &[], "{broken}").is_err());
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "Opt-in configured development provider test; sends only the synthetic fixture and may incur usage"]
async fn live_image_provider_probe() {
    use super::settings;
    use base64::Engine;
    assert_eq!(
        std::env::var("SIGNAL_IMAGE_PROVIDER_PROBE").as_deref(),
        Ok("dev-fixture")
    );
    let root = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap())
        .join("dev.contactestablished.signal");
    let p = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(root.join("signal-dev-september-2025.db"))
                .read_only(true),
        )
        .await
        .unwrap();
    let raw: String = sqlx::query_scalar("SELECT value FROM settings WHERE key='voice'")
        .fetch_one(&p)
        .await
        .unwrap();
    let config: settings::Settings = serde_json::from_str(&raw).unwrap();
    assert!(
        config.consent,
        "The development AI connection must already be configured with consent."
    );
    settings::endpoint(&config.endpoint).unwrap();
    let protected = std::fs::read(root.join("voice-dev/credential.bin")).unwrap();
    let key = String::from_utf8(settings::protect(&protected, true).unwrap()).unwrap();
    let model = if config.image_model.is_empty() {
        &config.model
    } else {
        &config.image_model
    };
    let source = include_bytes!("../../../tests/fixtures/intake/mixed-intake.png");
    let image = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(source)
    );
    let mut d = draft("image");
    d.transcript =
        "Extract all meetings and tasks. Keep uncertain meeting times unset for review.".into();
    let projects = vec![
        json!({"id":"atlas","name":"Atlas Migration"}),
        json!({"id":"website","name":"Website Refresh"}),
    ];
    let body = images::suggestion_body(&d, &projects, model, &image);
    let content = engine::bounded_suggestions(
        engine::provider_content(engine::client(90).unwrap(), &config.endpoint, &key, &body),
        std::time::Duration::from_secs(90),
    )
    .await
    .unwrap();
    let result = engine::parse_suggestions(d, &projects, &content).unwrap();
    assert!(result.candidates.iter().any(|c| c.kind == "meeting"));
    assert!(result.candidates.iter().any(|c| c.kind == "task"));
    assert!(result
        .candidates
        .iter()
        .any(|c| c.kind == "meeting" && c.meeting.as_ref().is_some_and(|m| m["starts_at"] == "")));
    println!(
        "Synthetic image interpreted into {} reviewed candidates; no records created.",
        result.candidates.len()
    );
    p.close().await;
}
