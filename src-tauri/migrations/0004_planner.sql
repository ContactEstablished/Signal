-- Floating wall times are preserved; offsets record explicit endpoint choices.
ALTER TABLE blocks ADD COLUMN revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0);
ALTER TABLE blocks ADD COLUMN time_zone TEXT;
ALTER TABLE blocks ADD COLUMN start_offset TEXT;
ALTER TABLE blocks ADD COLUMN end_offset TEXT;
CREATE TABLE planner_requests (
 request_id TEXT PRIMARY KEY NOT NULL,
 payload_json TEXT NOT NULL,
 outcome_json TEXT NOT NULL
);
-- Claims survive removal of the result. Task deletion removes its claims explicitly.
CREATE TABLE planner_claims (
 id TEXT PRIMARY KEY NOT NULL,
 mode TEXT NOT NULL CHECK(mode IN ('carry','copy','due')),
 source_id TEXT NOT NULL,
 task_id TEXT REFERENCES tasks(id),
 destination_date TEXT NOT NULL,
 block_id TEXT REFERENCES blocks(id) ON DELETE SET NULL
);
CREATE INDEX planner_claims_task ON planner_claims(task_id);
