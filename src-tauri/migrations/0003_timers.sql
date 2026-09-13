-- Durable sessions and request receipts; completed entries retain the M0 accounting boundary.
CREATE TABLE timer_sessions (
 id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id),
 block_id TEXT REFERENCES blocks(id), state TEXT NOT NULL CHECK(state IN ('running','paused','stopped')),
 started_at TEXT NOT NULL, segment_started_at TEXT,
 accumulated_ms INTEGER NOT NULL DEFAULT 0 CHECK(accumulated_ms BETWEEN 0 AND 9007199254740991),
 ended_at TEXT, revision INTEGER NOT NULL DEFAULT 0 CHECK(revision BETWEEN 0 AND 9007199254740991),
 entry_id TEXT UNIQUE REFERENCES time_entries(id),
 CHECK(ended_at IS NULL OR ended_at >= started_at),
 CHECK((state='running' AND segment_started_at IS NOT NULL AND ended_at IS NULL AND entry_id IS NULL)
 OR (state='paused' AND segment_started_at IS NULL AND ended_at IS NULL AND entry_id IS NULL)
 OR (state='stopped' AND segment_started_at IS NULL AND ended_at IS NOT NULL AND entry_id IS NOT NULL))
);
CREATE UNIQUE INDEX timer_live_task ON timer_sessions(task_id) WHERE state IN ('running','paused');
CREATE INDEX timer_task ON timer_sessions(task_id,id);
CREATE TABLE timer_requests (
 request_id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id),
 kind TEXT NOT NULL CHECK(kind IN ('start','pause','resume','stop','log')),
 payload_json TEXT NOT NULL, session_id TEXT REFERENCES timer_sessions(id), entry_id TEXT REFERENCES time_entries(id)
);
CREATE INDEX timer_requests_task ON timer_requests(task_id);
CREATE INDEX timer_requests_session ON timer_requests(session_id);
CREATE INDEX timer_requests_entry ON timer_requests(entry_id);
CREATE TRIGGER timer_block_insert BEFORE INSERT ON timer_sessions
WHEN NEW.block_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM blocks WHERE id=NEW.block_id AND task_id=NEW.task_id)
BEGIN SELECT RAISE(ABORT,'timer and block must belong to the same task'); END;
CREATE TRIGGER timer_block_update BEFORE UPDATE OF block_id,task_id ON timer_sessions
WHEN NEW.block_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM blocks WHERE id=NEW.block_id AND task_id=NEW.task_id)
BEGIN SELECT RAISE(ABORT,'timer and block must belong to the same task'); END;
