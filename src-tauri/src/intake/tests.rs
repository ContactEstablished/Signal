use super::{data::*, engine, settings};
use crate::workspace::{models::*, projects};
use serde_json::json;
use sqlx::SqlitePool;
const NOW: &str = "2026-09-18T16:00:00.000Z";
async fn database() -> SqlitePool {
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
fn draft() -> Draft {
    serde_json::from_value(json!({"id":"draft","revision":0,"recorded_at":NOW,"time_zone":"America/New_York","transcript":"For Atlas, write the release notes. For Launch, check the website.","projects":[],"candidates":[]})).unwrap()
}
fn candidate(id: &str, project: &str) -> Candidate {
    serde_json::from_value(json!({"id":id,"selected":true,"project_id":project,"proposal_id":null,"title":id,"notes":"Review before launch","subtasks":["Draft","", "Review"],"due_at":null,"priority":"medium","estimate_h":null,"source":"write the release notes","warning":"","duplicate_ok":false,"created_id":null})).unwrap()
}
fn request(d: &Draft, ids: &[&str]) -> Accept {
    Accept {
        request_id: uid(),
        draft_id: d.id.clone(),
        revision: d.revision,
        candidate_ids: ids.iter().map(|s| s.to_string()).collect(),
    }
}
async fn project(pool: &SqlitePool) -> String {
    projects::save(
        pool,
        None,
        ProjectInput {
            name: "Atlas".into(),
            color: "cyan".into(),
        },
    )
    .await
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[tokio::test]
async fn draft_roundtrip_conflict_and_lost_reply_retry() {
    let pool = database().await;
    let input = draft();
    let saved = save(&pool, input.clone()).await.unwrap();
    assert_eq!(
        json!(saved),
        json!(save(&pool, input.clone()).await.unwrap())
    );
    assert_eq!(json!(load(&pool).await.unwrap().unwrap()), json!(saved));
    let mut stale = input;
    stale.transcript = "different".into();
    assert!(save(&pool, stale).await.is_err());
    let mut forged = saved.clone();
    forged.projects.push(Proposal {
        id: "new".into(),
        name: "Fake".into(),
        color: "mint".into(),
        approved: true,
        created_id: Some("forged".into()),
    });
    assert!(save(&pool, forged).await.is_err());
    assert!(discard(&pool, &saved.id, 0).await.is_err());
    discard(&pool, &saved.id, saved.revision).await.unwrap();
    assert!(load(&pool).await.unwrap().is_none());
}
#[tokio::test]
async fn selected_tasks_are_atomic_todo_and_receipts_survive_draft_removal() {
    let pool = database().await;
    let p = project(&pool).await;
    let mut d = draft();
    d.candidates = vec![
        candidate("Write notes", &p),
        candidate("Keep for later", &p),
    ];
    let d = save(&pool, d).await.unwrap();
    let input = request(&d, &["Write notes"]);
    let receipt = accept(&pool, input.clone(), NOW).await.unwrap();
    assert_eq!(receipt["created"].as_array().unwrap().len(), 1);
    let row: (String, String, Option<String>, Option<f64>) =
        sqlx::query_as("SELECT status,priority,due_at,estimate_h FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row, ("todo".into(), "medium".into(), None, None));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM subtasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    let saved = load(&pool).await.unwrap().unwrap();
    assert!(saved.candidates[0].created_id.is_some());
    assert!(saved.candidates[1].created_id.is_none());
    discard(&pool, &saved.id, saved.revision).await.unwrap();
    assert_eq!(accept(&pool, input.clone(), NOW).await.unwrap(), receipt);
    let mut different = input;
    different.candidate_ids = vec!["Keep for later".into()];
    assert!(accept(&pool, different, NOW).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn proposed_projects_require_approval_and_failed_task_rolls_back_whole_batch() {
    let pool = database().await;
    let mut d = draft();
    d.projects.push(Proposal {
        id: "proposal".into(),
        name: "Launch".into(),
        color: "mint".into(),
        approved: false,
        created_id: None,
    });
    let mut c = candidate("Check website", "");
    c.project_id = None;
    c.proposal_id = Some("proposal".into());
    d.candidates = vec![c.clone()];
    let mut d = save(&pool, d).await.unwrap();
    assert!(accept(&pool, request(&d, &["Check website"]), NOW)
        .await
        .is_err());
    d.projects[0].approved = true;
    c.id = "Bad task".into();
    c.title = "".into();
    d.candidates.push(c);
    let mut d = save(&pool, d).await.unwrap();
    assert!(
        accept(&pool, request(&d, &["Check website", "Bad task"]), NOW)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    d.candidates[1].title = "Second task".into();
    let d = save(&pool, d).await.unwrap();
    let result = accept(&pool, request(&d, &["Check website", "Bad task"]), NOW)
        .await
        .unwrap();
    assert_eq!(result["projects"].as_array().unwrap().len(), 1);
    assert_eq!(result["created"].as_array().unwrap().len(), 2);
}
#[tokio::test]
async fn duplicate_titles_in_one_batch_require_explicit_acknowledgment() {
    let pool = database().await;
    let p = project(&pool).await;
    let mut d = draft();
    let a = candidate("release notes", &p);
    let mut b = candidate("b", &p);
    b.title = " RELEASE  notes ".into();
    d.candidates = vec![a, b];
    let mut d = save(&pool, d).await.unwrap();
    assert!(accept(&pool, request(&d, &["release notes", "b"]), NOW)
        .await
        .is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    d.candidates[1].duplicate_ok = true;
    let d = save(&pool, d).await.unwrap();
    assert!(accept(&pool, request(&d, &["release notes", "b"]), NOW)
        .await
        .is_ok());
}
#[test]
fn ai_output_is_grounded_and_unknown_projects_are_unapproved() {
    let content = json!({"tasks":[{"project_id":"invented","project_name":"Launch","title":"Check website","source":"check the website","subtasks":[]}]});
    let parsed = engine::parse_suggestions(draft(), &[], &content.to_string()).unwrap();
    assert!(!parsed.projects[0].approved);
    assert_eq!(
        parsed.candidates[0].proposal_id.as_deref(),
        Some(parsed.projects[0].id.as_str())
    );
    assert!(parsed.candidates[0].project_id.is_none());
    assert!(parsed.candidates[0].due_at.is_none());
    assert_eq!(parsed.candidates[0].priority, "medium");
    let mut bad = content;
    bad["tasks"][0]["source"] = json!("words not in source");
    assert!(engine::parse_suggestions(draft(), &[], &bad.to_string()).is_err());
    assert!(engine::parse_suggestions(draft(), &[], "{truncated").is_err());
}
#[test]
fn ambiguous_project_names_remain_unassigned() {
    let content = json!({"tasks":[{"project_id":null,"project_name":"Atlas","title":"Write notes","source":"write the release notes","subtasks":[]}]});
    let parsed = engine::parse_suggestions(
        draft(),
        &[
            json!({"id":"a","name":"Atlas"}),
            json!({"id":"b","name":"Atlas"}),
        ],
        &content.to_string(),
    )
    .unwrap();
    assert!(parsed.candidates[0].project_id.is_none());
    assert!(parsed.candidates[0].proposal_id.is_none());
    assert!(parsed.candidates[0].warning.contains("Multiple"));
}
#[test]
fn numbered_sources_resolve_to_original_text_and_reject_invented_references() {
    let d = draft();
    let body = engine::suggestion_body(&d, &[], "test-model");
    let user: serde_json::Value =
        serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(
        user["source_excerpts"][1],
        json!({"id":1,"text":"For Launch, check the website."})
    );
    let mut content = json!({"tasks":[{"title":"Check website","project_name":"Launch","source_id":1,"source":"invented paraphrase","subtasks":[]}]});
    let result = engine::parse_suggestions(d.clone(), &[], &content.to_string()).unwrap();
    assert_eq!(
        result.candidates[0].source,
        "For Launch, check the website."
    );
    for invalid in [json!(-1), json!(99), json!("1"), json!(null), json!(0.5)] {
        content["tasks"][0]["source_id"] = invalid;
        assert!(engine::parse_suggestions(d.clone(), &[], &content.to_string()).is_err());
    }
}

#[test]
fn extraction_disables_optional_high_reasoning_only_for_verified_provider_model_pair() {
    for (url, model, fast) in [
        (
            "https://openrouter.ai/api/v1",
            "~deepseek/deepseek-flash-latest",
            true,
        ),
        (
            "https://openrouter.ai/api/v1/chat/completions",
            "~deepseek/deepseek-flash-latest",
            true,
        ),
        (
            "https://other.example/v1",
            "~deepseek/deepseek-flash-latest",
            false,
        ),
        (
            "https://openrouter.ai/api/v1",
            "mandatory-reasoning-model",
            false,
        ),
    ] {
        let mut body = engine::suggestion_body(&draft(), &[], model);
        engine::configure_extraction(&mut body, url);
        assert_eq!(
            body.get("reasoning").cloned(),
            if fast {
                Some(json!({"enabled":false}))
            } else {
                None
            }
        );
        assert_eq!(body["model"], model);
    }
}
#[test]
fn pcm_wav_and_provider_endpoint_boundaries() {
    let bytes = engine::wav(&[i16::MIN, 0, i16::MAX]);
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(bytes.len(), 50);
    assert_eq!(&bytes[24..28], &16000u32.to_le_bytes());
    assert_eq!(&bytes[40..44], &6u32.to_le_bytes());
    assert_eq!(&bytes[44..46], &i16::MIN.to_le_bytes());
    assert!(settings::endpoint("https://api.example.test/v1").is_ok());
    for bad in [
        "http://api.example.test",
        "https://secret@api.example.test",
        "https://api.example.test?key=secret",
        "https://api.example.test/#fragment",
    ] {
        assert!(settings::endpoint(bad).is_err());
    }
}

#[tokio::test]
async fn persisted_review_and_receipt_reopen_without_duplicate_writes() {
    let dir = tempfile::tempdir().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("voice.db"))
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options.clone()).await.unwrap();
    for migration in crate::db::migrations() {
        sqlx::raw_sql(migration.sql).execute(&pool).await.unwrap();
    }
    let p = project(&pool).await;
    let mut d = draft();
    d.candidates.push(candidate("Persist me", &p));
    let d = save(&pool, d).await.unwrap();
    let input = request(&d, &["Persist me"]);
    let receipt = accept(&pool, input.clone(), NOW).await.unwrap();
    pool.close().await;
    let pool = SqlitePool::connect_with(options).await.unwrap();
    let restored = load(&pool).await.unwrap().unwrap();
    assert_eq!(restored.transcript, d.transcript);
    assert!(restored.candidates[0].created_id.is_some());
    assert_eq!(accept(&pool, input, NOW).await.unwrap(), receipt);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    pool.close().await;
}
#[test]
fn audio_frames_reject_lost_order_and_duration_overflow_without_advancing() {
    let mut capture = engine::Capture {
        id: "test".into(),
        next: 0,
        samples: vec![],
    };
    assert!(capture.push(1, vec![1; 1024]).is_err());
    assert!(capture.push(0, vec![]).is_err());
    assert!(capture.push(0, vec![1; 4097]).is_err());
    assert_eq!(capture.next, 0);
    capture.push(0, vec![1; 1024]).unwrap();
    assert!(capture.push(0, vec![1; 1024]).is_err());
    capture.samples.resize(engine::MAX_SAMPLES, 0);
    assert!(capture.push(1, vec![1]).is_err());
    assert_eq!(capture.samples.len(), engine::MAX_SAMPLES);
}
#[cfg(windows)]
#[test]
fn credentials_are_windows_protected_and_corruption_is_rejected() {
    let secret = b"synthetic-test-key-never-a-real-credential";
    let encrypted = settings::protect(secret, false).unwrap();
    assert_ne!(encrypted, secret);
    assert_eq!(settings::protect(&encrypted, true).unwrap(), secret);
    assert!(settings::protect(b"not-a-protected-blob", true).is_err());
}

fn provider_fixture(
    status: &str,
    body: String,
    extra_headers: &str,
) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let content_type = if extra_headers.contains("Content-Type:") {
        ""
    } else {
        "Content-Type: application/json\r\n"
    };
    let response=format!("HTTP/1.1 {status}\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}",body.len());
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = socket.read(&mut buf).unwrap();
            request.extend_from_slice(&buf[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                let len = headers
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length: ")
                            .and_then(|v| v.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + len {
                    break;
                }
            }
        }
        socket.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}/v1"), handle)
}
#[tokio::test]
async fn provider_transport_uses_configured_endpoint_and_rejects_failures_without_echoing_secrets()
{
    let body = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"tasks\":[]}"}}]})
        .to_string();
    let (url, server) = provider_fixture("200 OK", body, "");
    let result = engine::provider_content(
        engine::client(3).unwrap(),
        &url,
        "synthetic-key",
        &json!({"model":"chosen-model","messages":[]}),
    )
    .await
    .unwrap();
    assert_eq!(result, "{\"tasks\":[]}");
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /v1/chat/completions"));
    assert!(request.contains("Bearer synthetic-key"));
    assert!(request.contains("chosen-model"));
    for (status, body, headers) in [
        (
            "429 Too Many Requests",
            "synthetic-private-provider-body".into(),
            "",
        ),
        (
            "302 Found",
            "redirect".into(),
            "Location: http://127.0.0.1:1/never-follow\r\n",
        ),
        (
            "200 OK",
            json!({"choices":[{"finish_reason":"length","message":{"content":"{}"}}]}).to_string(),
            "",
        ),
    ] {
        let (url, server) = provider_fixture(status, body, headers);
        let error = engine::provider_content(
            engine::client(3).unwrap(),
            &url,
            "synthetic-key",
            &json!({}),
        )
        .await
        .err()
        .unwrap();
        server.join().unwrap();
        assert!(!error.message.contains("synthetic-private"));
        assert!(!error.message.contains("synthetic-key"));
        assert_ne!(error.code, "Network");
    }
}

#[tokio::test]
async fn full_completion_urls_are_not_appended_twice() {
    for suffix in ["", "/", "/chat/completions", "/chat/completions/"] {
        let body =
            json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"tasks\":[]}"}}]})
                .to_string();
        let (base, server) = provider_fixture("200 OK", body, "");
        let content = engine::provider_content(
            engine::client(3).unwrap(),
            &format!("{base}{suffix}"),
            "synthetic-key",
            &json!({"model":"test","messages":[]}),
        )
        .await
        .unwrap();
        assert_eq!(content, "{\"tasks\":[]}");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
        assert!(!request.contains("/chat/completions/chat/completions"));
    }
    assert_eq!(
        engine::completion_url(" https://openrouter.ai/api/v1/chat/completions/ "),
        "https://openrouter.ai/api/v1/chat/completions"
    );
    assert_eq!(
        engine::completion_url("https://provider.example/custom/prefix/v1"),
        "https://provider.example/custom/prefix/v1/chat/completions"
    );
}
#[test]
fn provider_failures_explain_the_relevant_fix_and_preserve_transcript_guidance() {
    for (status, hint) in [
        (401, "API key"),
        (403, "permissions"),
        (404, "API URL or model"),
        (429, "quota"),
        (503, "temporarily unavailable"),
    ] {
        let error = engine::provider_error(status);
        assert!(error.message.contains(hint));
        assert!(error.message.contains(&format!("HTTP {status}")));
        assert!(error.message.contains("Your transcript is saved"));
    }
}

#[tokio::test]
async fn streamed_completion_uses_final_answer_and_handles_fragmented_unicode() {
    let answer = "{\"tasks\":[],\"note\":\"café\"}";
    let event = json!({"choices":[{"delta":{"content":answer,"reasoning":"discard this"},"finish_reason":null}]}).to_string();
    let wire = format!(": keepalive\r\n\r\ndata: {event}\r\n\r\ndata: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n");
    let mut stream = engine::CompletionStream::default();
    for byte in wire.as_bytes() {
        stream.push(&[*byte]).unwrap();
    }
    assert_eq!(stream.finish().unwrap(), answer);
    let (url, server) = provider_fixture("200 OK", wire, "Content-Type: text/event-stream\r\n");
    let content = engine::provider_content(
        engine::client(3).unwrap(),
        &url,
        "synthetic-key",
        &json!({"model":"test"}),
    )
    .await
    .unwrap();
    assert_eq!(content, answer);
    let request = server.join().unwrap();
    assert!(request.contains("\"stream\":true"));
}

#[test]
fn interrupted_or_failed_streams_never_become_recommendations() {
    let mut stream = engine::CompletionStream::default();
    stream
        .push(b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n")
        .unwrap();
    assert!(stream.finish().is_err());
    for bad in [
        "data: {\"error\":{\"message\":\"private provider detail\"}}\n\n",
        "data: {\"choices\":[{\"finish_reason\":\"length\"}]}\n\n",
        "data: {invalid}\n\n",
    ] {
        let error = engine::CompletionStream::default()
            .push(bad.as_bytes())
            .unwrap_err();
        assert!(!error.message.contains("private provider detail"));
    }
}

#[tokio::test]
async fn suggestion_deadline_drops_stalled_work_and_allows_retry() {
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };
    struct Cleanup(Arc<AtomicBool>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let cleaned = Arc::new(AtomicBool::new(false));
    let guard = Cleanup(cleaned.clone());
    let error = engine::bounded_suggestions(
        async move {
            let _guard = guard;
            std::future::pending::<Result<()>>().await
        },
        Duration::from_millis(20),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "Timeout");
    assert!(error.message.contains("Your transcript is saved"));
    assert!(cleaned.load(Ordering::SeqCst));
    assert_eq!(
        engine::bounded_suggestions(async { Ok(7) }, Duration::from_secs(1))
            .await
            .unwrap(),
        7
    );
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "Opt-in live provider diagnostic: development fixture only; synthetic by default, saved-review requires separate opt-in; may incur provider usage"]
async fn live_dev_fixture_provider_probe() {
    // Never load personal configuration. Saved-review mode is a separate opt-in
    // for reproducing a user-authorized failure; it never writes any records.
    assert_eq!(
        std::env::var("SIGNAL_VOICE_PROVIDER_PROBE").as_deref(),
        Ok("dev-fixture")
    );
    let root = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap())
        .join("dev.contactestablished.signal");
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(root.join("signal-dev-september-2025.db"))
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let raw: String = sqlx::query_scalar("SELECT value FROM settings WHERE key='voice'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let config: settings::Settings = serde_json::from_str(&raw).unwrap();
    assert!(config.consent);
    let endpoint = settings::endpoint(&config.endpoint).unwrap();
    assert_eq!(endpoint.host_str(), Some("openrouter.ai"));
    let encrypted = std::fs::read(root.join("voice-dev/credential.bin")).unwrap();
    let key = String::from_utf8(settings::protect(&encrypted, true).unwrap()).unwrap();
    let saved_review =
        std::env::var("SIGNAL_VOICE_PROVIDER_INPUT").as_deref() == Ok("saved-review");
    let input = if saved_review {
        load(&pool).await.unwrap().unwrap()
    } else {
        draft()
    };
    let projects = if saved_review {
        projects::list(&pool).await.unwrap()
    } else {
        vec![
            json!({"id":"demo-atlas","name":"Atlas"}),
            json!({"id":"demo-launch","name":"Launch"}),
        ]
    };
    let body = engine::suggestion_body(&input, &projects, &config.model);
    let started = std::time::Instant::now();
    println!("Native provider probe: request ready, saved-review mode={saved_review}");
    let content = engine::bounded_suggestions(
        engine::provider_content(engine::client(90).unwrap(), &config.endpoint, &key, &body),
        std::time::Duration::from_secs(90),
    )
    .await
    .unwrap();
    let result = engine::parse_suggestions(input, &projects, &content).unwrap();
    assert!(!result.candidates.is_empty());
    // Counts and elapsed time only: no credentials, provider payloads or user text.
    println!(
        "Native provider probe: {} candidates in {:.1}s",
        result.candidates.len(),
        started.elapsed().as_secs_f64()
    );
    pool.close().await;
}
