use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInput {
    pub request_id: String,
    pub task_id: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionInput {
    pub request_id: String,
    pub task_id: String,
    pub session_id: String,
    pub expected_revision: i64,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogInput {
    pub request_id: String,
    pub task_id: String,
    pub started_at: String,
    pub duration_ms: i64,
}
