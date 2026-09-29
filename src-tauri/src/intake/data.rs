use crate::workspace::{models::*, projects, tasks};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub id: String,
    pub name: String,
    pub color: String,
    pub approved: bool,
    pub created_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    #[serde(default = "task_kind")]
    pub kind: String,
    #[serde(default)]
    pub meeting: Option<Value>,
    pub id: String,
    pub selected: bool,
    pub project_id: Option<String>,
    pub proposal_id: Option<String>,
    pub title: String,
    pub notes: String,
    pub subtasks: Vec<String>,
    pub due_at: Option<String>,
    pub priority: String,
    pub estimate_h: Option<f64>,
    pub source: String,
    pub warning: String,
    pub duplicate_ok: bool,
    pub created_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    #[serde(default = "voice_kind")]
    pub source_kind: String,
    #[serde(default)]
    pub image_id: Option<String>,
    pub id: String,
    pub revision: i64,
    pub recorded_at: String,
    pub time_zone: String,
    pub transcript: String,
    pub projects: Vec<Proposal>,
    pub candidates: Vec<Candidate>,
}
fn task_kind() -> String {
    "task".into()
}
fn voice_kind() -> String {
    "voice".into()
}
impl Draft {
    pub fn validate(&self) -> Result<()> {
        nonblank(&self.id)?;
        choice(&self.source_kind, &["voice", "image"])?;
        if self.revision < 0 {
            return Err(AppError::conflict());
        }
        instant(&self.recorded_at)?;
        self.time_zone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| AppError::validation("Choose a valid timezone."))?;
        if self.transcript.len() > 80_000 || self.candidates.len() > 100 || self.projects.len() > 50
        {
            return Err(AppError::validation(
                "This draft is too large. Use a shorter dictation.",
            ));
        }
        let mut ids = HashSet::new();
        for p in &self.projects {
            nonblank(&p.id)?;
            if !ids.insert(&p.id) || p.name.len() > 200 {
                return Err(AppError::validation("Invalid proposed project."));
            }
            choice(&p.color, COLORS)?;
        }
        let mut ids = HashSet::new();
        for c in &self.candidates {
            nonblank(&c.id)?;
            choice(&c.kind, &["task", "meeting"])?;
            if self.source_kind == "voice" && c.kind != "task" {
                return Err(AppError::validation("Voice supports tasks only."));
            }
            if c.meeting
                .as_ref()
                .is_some_and(|m| m.to_string().len() > 40_000)
            {
                return Err(AppError::validation("Meeting suggestion is too large."));
            }
            if !ids.insert(&c.id)
                || c.title.len() > 500
                || c.notes.len() > 20_000
                || c.source.len() > 10_000
                || c.warning.len() > 2_000
                || c.subtasks.len() > 50
                || c.subtasks.iter().any(|s| s.len() > 500)
            {
                return Err(AppError::validation("Invalid task suggestion."));
            }
        }
        Ok(())
    }
}
pub async fn load(pool: &SqlitePool) -> Result<Option<Draft>> {
    load_source(pool, "voice").await
}
pub async fn load_source(pool: &SqlitePool, source: &str) -> Result<Option<Draft>> {
    let payload: Option<String> =
        sqlx::query_scalar("SELECT payload FROM intake_drafts WHERE source_kind=? LIMIT 1")
            .bind(source)
            .fetch_optional(pool)
            .await?;
    payload
        .map(|s| {
            serde_json::from_str(&s)
                .map_err(|_| AppError::validation("The voice draft could not be read."))
        })
        .transpose()
}
pub async fn save(pool: &SqlitePool, mut draft: Draft) -> Result<Draft> {
    draft.validate()?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let old: Option<String> =
        sqlx::query_scalar("SELECT payload FROM intake_drafts WHERE source_kind=? LIMIT 1")
            .bind(&draft.source_kind)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(old) = old {
        let old: Draft = serde_json::from_str(&old).map_err(|_| AppError::conflict())?;
        if old.id == draft.id && old.revision == draft.revision + 1 {
            let mut retried = draft.clone();
            retried.revision = old.revision;
            if json!(retried) == json!(old) {
                return Ok(old);
            }
        }
        if old.id != draft.id || old.revision != draft.revision {
            return Err(AppError::conflict());
        }
        if old.source_kind != draft.source_kind || old.image_id != draft.image_id {
            return Err(AppError::conflict());
        }
        // Accepted IDs are authoritative; editing a draft cannot erase receipts.
        for c in &old.candidates {
            if c.created_id.is_some()
                && !draft
                    .candidates
                    .iter()
                    .any(|n| n.id == c.id && json!(n) == json!(c))
            {
                return Err(AppError::conflict());
            }
        }
        for p in &old.projects {
            if p.created_id.is_some()
                && !draft
                    .projects
                    .iter()
                    .any(|n| n.id == p.id && n.created_id == p.created_id)
            {
                return Err(AppError::conflict());
            }
        }
        for c in &draft.candidates {
            if c.created_id.is_some()
                && !old
                    .candidates
                    .iter()
                    .any(|n| n.id == c.id && json!(n) == json!(c))
            {
                return Err(AppError::conflict());
            }
        }
        for p in &draft.projects {
            if p.created_id.is_some()
                && !old
                    .projects
                    .iter()
                    .any(|n| n.id == p.id && n.created_id == p.created_id)
            {
                return Err(AppError::conflict());
            }
        }
    } else if draft.revision != 0
        || draft.candidates.iter().any(|c| c.created_id.is_some())
        || draft.projects.iter().any(|p| p.created_id.is_some())
    {
        return Err(AppError::conflict());
    }
    // A new source draft must never overwrite a different source's ID.
    let owner: Option<String> =
        sqlx::query_scalar("SELECT source_kind FROM intake_drafts WHERE id=?")
            .bind(&draft.id)
            .fetch_optional(&mut *tx)
            .await?;
    if owner.is_some_and(|kind| kind != draft.source_kind) {
        return Err(AppError::conflict());
    }
    if draft.source_kind == "image" && draft.image_id.is_some() {
        let found: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM intake_images WHERE id=? AND draft_id=?")
                .bind(&draft.image_id)
                .bind(&draft.id)
                .fetch_one(&mut *tx)
                .await?;
        if found != 1 {
            return Err(AppError::conflict());
        }
    }
    draft.revision += 1;
    sqlx::query("INSERT INTO intake_drafts(id,revision,payload,updated_at,source_kind) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,payload=excluded.payload,updated_at=excluded.updated_at")
        .bind(&draft.id).bind(draft.revision).bind(json!(draft).to_string()).bind(chrono::Utc::now().to_rfc3339()).bind(&draft.source_kind).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(draft)
}
pub async fn discard(pool: &SqlitePool, id: &str, revision: i64) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let n = sqlx::query("DELETE FROM intake_drafts WHERE id=? AND revision=?")
        .bind(id)
        .bind(revision)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if n != 1 {
        return Err(AppError::conflict());
    }
    sqlx::query("DELETE FROM intake_images WHERE draft_id=?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accept {
    pub request_id: String,
    pub draft_id: String,
    pub revision: i64,
    pub candidate_ids: Vec<String>,
}
pub async fn accept(pool: &SqlitePool, request: Accept, now: &str) -> Result<Value> {
    nonblank(&request.request_id)?;
    let fingerprint = format!("{:x}", Sha256::digest(json!(request).to_string()));
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let receipt: Option<(String, String)> =
        sqlx::query_as("SELECT fingerprint,result FROM intake_receipts WHERE request_id=?")
            .bind(&request.request_id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some((original, result)) = receipt {
        if original != fingerprint {
            return Err(AppError::conflict());
        }
        return serde_json::from_str(&result).map_err(|_| {
            AppError::new(
                "UnknownOutcome",
                "The saved receipt could not be read. Keep this request and retry.",
            )
        });
    }
    let raw: String =
        sqlx::query_scalar("SELECT payload FROM intake_drafts WHERE id=? AND revision=?")
            .bind(&request.draft_id)
            .bind(request.revision)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(AppError::conflict)?;
    let mut draft: Draft = serde_json::from_str(&raw).map_err(|_| AppError::conflict())?;
    draft.validate()?;
    let ids: HashSet<_> = request.candidate_ids.iter().collect();
    if ids.is_empty()
        || ids.len() != request.candidate_ids.len()
        || ids.iter().any(|id| {
            !draft
                .candidates
                .iter()
                .any(|c| &c.id == *id && c.created_id.is_none())
        })
    {
        return Err(AppError::validation("Choose uncreated suggestions."));
    }
    let mut mapped = HashMap::new();
    let mut created_projects = Vec::new();
    for p in &mut draft.projects {
        if !draft
            .candidates
            .iter()
            .any(|c| ids.contains(&c.id) && c.proposal_id.as_ref() == Some(&p.id))
        {
            continue;
        }
        if let Some(id) = &p.created_id {
            mapped.insert(p.id.clone(), id.clone());
            continue;
        }
        if !p.approved {
            return Err(AppError::validation(
                "Approve the proposed project before creating its tasks.",
            ));
        }
        let name = nonblank(&p.name)?;
        let existing = projects::list_in(&mut tx).await?;
        if existing
            .iter()
            .any(|v| v["name"].as_str().unwrap_or("").trim().to_lowercase() == name.to_lowercase())
        {
            return Err(AppError::validation(
                "A project with this name already exists. Assign its tasks to that project.",
            ));
        }
        let result = projects::save_in(
            &mut tx,
            None,
            ProjectInput {
                name,
                color: p.color.clone(),
            },
        )
        .await?;
        let id = result["id"].as_str().unwrap().to_owned();
        p.created_id = Some(id.clone());
        mapped.insert(p.id.clone(), id);
        created_projects.push(result);
    }
    let mut created = Vec::new();
    for c in &mut draft.candidates {
        if !ids.contains(&c.id) {
            continue;
        }
        if c.project_id.is_some() == c.proposal_id.is_some() {
            return Err(AppError::validation(
                "Choose one project for every selected task.",
            ));
        }
        let project_id = c
            .project_id
            .clone()
            .or_else(|| c.proposal_id.as_ref().and_then(|p| mapped.get(p).cloned()))
            .ok_or_else(|| AppError::validation("Choose a project."))?;
        let normalize = |s: &str| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        if c.kind == "meeting" {
            let mut meeting = c
                .meeting
                .clone()
                .ok_or_else(|| AppError::validation("Supply meeting details."))?;
            meeting["title"] = json!(c.title);
            let meeting = crate::agenda::models::normalize(&meeting, true)?;
            let at = text(&meeting, "starts_at")?;
            let start = crate::agenda::models::parse(at)?;
            let end = crate::agenda::models::utc(start + chrono::Duration::seconds(1));
            let existing =
                crate::agenda::occurrences(&mut tx, at, &end, Some(&project_id), false).await?;
            if !c.duplicate_ok
                && existing.iter().any(|v| {
                    v["project_id"] == project_id
                        && v["starts_at"] == meeting["starts_at"]
                        && normalize(v["title"].as_str().unwrap_or("")) == normalize(&c.title)
                })
            {
                return Err(AppError::validation(format!(
                    "Possible duplicate meeting: {}. Allow duplicates if intentional.",
                    c.title
                )));
            }
            let reference = crate::agenda::create_in(&mut tx, &project_id, &meeting, now).await?;
            c.created_id = reference["meetingId"].as_str().map(str::to_owned);
            created.push(json!({"id":c.created_id,"project_id":project_id,"title":c.title,"kind":"meeting","ref":reference}));
        } else {
            let existing: Vec<String> =
                sqlx::query_scalar("SELECT title FROM tasks WHERE project_id=?")
                    .bind(&project_id)
                    .fetch_all(&mut *tx)
                    .await?;
            if !c.duplicate_ok && existing.iter().any(|t| normalize(t) == normalize(&c.title)) {
                return Err(AppError::validation(format!(
                    "Possible duplicate: {}. Allow duplicates if intentional.",
                    c.title
                )));
            }
            let result = tasks::create_in(&mut tx, json!({"project_id":project_id,"title":c.title,"status":"todo","priority":c.priority,"notes_md":c.notes,"due_at":c.due_at,"estimate_h":c.estimate_h,"external_url":null,"external_provider":null,"external_id":null,"blocked_reason":null,"blocked_on":null,"subtasks":c.subtasks.iter().filter(|s| !s.trim().is_empty()).map(|s| json!({"title":s,"done":false})).collect::<Vec<_>>(),"tags":[],"alerts":[1440,60],"attachments":[]}), now).await?;
            c.created_id = result["task"]["id"].as_str().map(str::to_owned);
            created.push(
                json!({"id":c.created_id,"project_id":project_id,"title":c.title,"kind":"task"}),
            );
        }
        c.selected = false;
    }
    draft.revision += 1;
    sqlx::query("UPDATE intake_drafts SET payload=?,revision=?,updated_at=? WHERE id=?")
        .bind(json!(draft).to_string())
        .bind(draft.revision)
        .bind(now)
        .bind(&draft.id)
        .execute(&mut *tx)
        .await?;
    let receipt = json!({"created":created,"projects":created_projects.iter().map(|p| json!({"id":p["id"],"name":p["name"]})).collect::<Vec<_>>()});
    sqlx::query("INSERT INTO intake_receipts VALUES(?,?,?,?)")
        .bind(&request.request_id)
        .bind(fingerprint)
        .bind(receipt.to_string())
        .bind(now)
        .execute(&mut *tx)
        .await?;
    tx.commit().await.map_err(|_| {
        AppError::new(
            "UnknownOutcome",
            "Task creation outcome is uncertain. Retry the same request.",
        )
    })?;
    Ok(receipt)
}
