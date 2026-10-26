use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What the user asked for, as proposed by the AI (or built locally). Every field is a
/// suggestion until the application resolves it against stored data and the user confirms.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    AddTask(TaskDraft),
    AddMeeting(MeetingDraft),
    CompleteTask(CompleteDraft),
    CancelMeeting(CancelDraft),
    Lookup(LookupDraft),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TaskDraft {
    pub title: String,
    pub project_name: Option<String>,
    pub project_id: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub notes: String,
    pub priority: Option<String>,
    pub estimate_h: Option<f64>,
    /// Set when the user was given the 17:00 default so the card can say so.
    pub time_defaulted: bool,
    /// "earlier" or "later" once the user has resolved a repeated local time.
    pub fold: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MeetingDraft {
    pub title: String,
    pub project_name: Option<String>,
    pub project_id: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub duration_min: Option<i64>,
    pub link_url: Option<String>,
    pub notes: String,
    /// "earlier" or "later" once the user has resolved a repeated local time.
    pub fold: Option<String>,
    pub duration_defaulted: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CompleteDraft {
    pub query: String,
    pub note: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CancelDraft {
    pub query: String,
    /// A date the user named, used to bound which occurrences are searched.
    pub date: Option<String>,
    /// "cancel" keeps the meeting visible; "remove" is an explicit delete request.
    pub mode: String,
    /// "occurrence" or "following" for a recurring meeting.
    pub scope: Option<String>,
    pub meeting_id: Option<String>,
    pub occurrence_key: Option<String>,
    /// Monday-aligned start of the searched week; moves with the paging buttons.
    pub window_start: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LookupDraft {
    pub date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Opt {
    pub label: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Question {
    /// What the answer fills in: project, task, meeting, scope, time, date, fold, mode.
    pub field: String,
    pub prompt: String,
    pub options: Vec<Opt>,
    pub page: usize,
    pub text_ok: bool,
}

/// The exact change that will be written if the user taps Confirm.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    CreateTask {
        input: Value,
    },
    CompleteTask {
        task_id: String,
        revision: i64,
        notes_md: String,
        note: Option<String>,
    },
    CreateMeeting {
        project_id: String,
        draft: Value,
    },
    CancelMeeting {
        meeting_id: String,
        occurrence_key: String,
        revision: i64,
        occurrence_notes: String,
    },
    RemoveMeeting {
        meeting_id: String,
        occurrence_key: String,
        revision: i64,
        scope: String,
        fingerprint: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub lines: Vec<String>,
    /// What to tell the user once this has been saved.
    pub done_text: String,
    pub op: Op,
    /// The action this proposal came from, so Change can revise it.
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Question(Question),
    Confirm(Proposal),
    /// Waiting for the correction text after the user tapped Change.
    Revise(Proposal),
    /// A new request arrived while another was open.
    Conflict { incoming: Incoming },
}

/// A request received while another was waiting for an answer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Incoming {
    pub text: String,
    pub message_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Flow {
    pub token: String,
    pub step: i64,
    pub message_time: String,
    pub expires_at: String,
    pub text: String,
    pub current: Option<Action>,
    pub queue: Vec<Action>,
    pub stage: Option<Stage>,
    /// Stage to return to when the user chooses to continue after a conflict.
    pub parked: Option<Box<Stage>>,
    pub done: Vec<String>,
    pub message_id: Option<i64>,
}

pub fn clip(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}
