ALTER TABLE meetings ADD COLUMN time_zone TEXT;
ALTER TABLE meetings ADD COLUMN show_in_day INTEGER CHECK(show_in_day IN (0,1));
ALTER TABLE meetings ADD COLUMN active_version_id TEXT;
ALTER TABLE meetings ADD COLUMN revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0);
CREATE TABLE meeting_versions(id TEXT PRIMARY KEY, meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE, created_at TEXT NOT NULL);
CREATE TABLE meeting_segments(
 id TEXT PRIMARY KEY, version_id TEXT NOT NULL REFERENCES meeting_versions(id) ON DELETE CASCADE,
 from_ordinal INTEGER NOT NULL CHECK(from_ordinal>=0), to_ordinal INTEGER CHECK(to_ordinal>from_ordinal),
 anchor_local TEXT NOT NULL, anchor_utc TEXT NOT NULL, time_zone TEXT NOT NULL,
 repeat_rule TEXT NOT NULL CHECK(repeat_rule IN ('none','daily','weekly')), repeat_until TEXT,
 fold_policy TEXT NOT NULL CHECK(fold_policy IN ('earlier','later')),
 title TEXT NOT NULL, duration_min INTEGER NOT NULL CHECK(duration_min>0), link_url TEXT,
 agenda_md TEXT NOT NULL, notes_md TEXT NOT NULL, reminder_min INTEGER NOT NULL CHECK(reminder_min>=0),
 show_in_day INTEGER NOT NULL CHECK(show_in_day IN (0,1)), UNIQUE(version_id,from_ordinal)
);
CREATE TABLE meeting_segment_tasks(segment_id TEXT NOT NULL REFERENCES meeting_segments(id) ON DELETE CASCADE, task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE, PRIMARY KEY(segment_id,task_id));
CREATE INDEX meeting_segment_task_lookup ON meeting_segment_tasks(task_id);
CREATE TABLE meeting_occurrences(
 meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE, occurrence_key TEXT NOT NULL,
 ordinal INTEGER NOT NULL CHECK(ordinal>=0), revision INTEGER NOT NULL DEFAULT 0,
 cancelled INTEGER NOT NULL DEFAULT 0 CHECK(cancelled IN(0,1)), snapshot_json TEXT, actual_starts_at TEXT,
 attendance TEXT NOT NULL DEFAULT 'unmarked' CHECK(attendance IN('unmarked','attended','missed')),
 occurrence_notes_md TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL,
 PRIMARY KEY(meeting_id,occurrence_key), UNIQUE(meeting_id,ordinal)
);
CREATE INDEX meeting_occurrence_time ON meeting_occurrences(actual_starts_at);
CREATE TABLE meeting_occurrence_tasks(meeting_id TEXT NOT NULL, occurrence_key TEXT NOT NULL,
 task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
 FOREIGN KEY(meeting_id,occurrence_key) REFERENCES meeting_occurrences(meeting_id,occurrence_key) ON DELETE CASCADE,
 PRIMARY KEY(meeting_id,occurrence_key,task_id));
CREATE INDEX meeting_occurrence_task_lookup ON meeting_occurrence_tasks(task_id);
CREATE TABLE meeting_cutoffs(meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE, from_ordinal INTEGER NOT NULL CHECK(from_ordinal>=0), created_at TEXT NOT NULL, PRIMARY KEY(meeting_id,from_ordinal));
CREATE TABLE agenda_requests(request_id TEXT PRIMARY KEY, payload_hash TEXT NOT NULL, outcome_json TEXT NOT NULL);
