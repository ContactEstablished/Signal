-- All 13 handoff tables. No cascade or product deletion behavior is defined.
-- Instants use canonical UTC ISO strings; planner date/minutes remain local.
CREATE TABLE projects (
  id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL,
  manager_name TEXT, manager_email TEXT, summary_send_at TEXT,
  summary_tone TEXT NOT NULL DEFAULT 'brief' CHECK(summary_tone IN ('brief','detailed')),
  sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tasks (
  id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), title TEXT NOT NULL,
  external_url TEXT, external_provider TEXT, external_id TEXT,
  status TEXT NOT NULL DEFAULT 'backlog' CHECK(status IN ('backlog','todo','in_progress','blocked','done')),
  priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('low','medium','high')),
  due_at TEXT, estimate_h REAL CHECK(estimate_h IS NULL OR estimate_h >= 0),
  hours_worked REAL NOT NULL DEFAULT 0 CHECK(hours_worked >= 0),
  blocked_reason TEXT, blocked_on TEXT, notes_md TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, done_at TEXT
);
CREATE TABLE subtasks (
  id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), title TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0 CHECK(done IN (0,1)), sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tags (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL);
CREATE TABLE task_tags (task_id TEXT NOT NULL REFERENCES tasks(id), tag_id TEXT NOT NULL REFERENCES tags(id), PRIMARY KEY(task_id,tag_id));
CREATE TABLE attachments (
  id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), filename TEXT NOT NULL,
  path TEXT NOT NULL, size INTEGER NOT NULL CHECK(size >= 0), mime TEXT NOT NULL
);
CREATE TABLE meetings (
  id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), title TEXT NOT NULL,
  starts_at TEXT NOT NULL, duration_min INTEGER NOT NULL CHECK(duration_min > 0), link_url TEXT,
  agenda_md TEXT NOT NULL DEFAULT '', notes_md TEXT NOT NULL DEFAULT '',
  repeat_rule TEXT NOT NULL DEFAULT 'none' CHECK(repeat_rule IN ('none','daily','weekly')),
  reminder_min INTEGER NOT NULL DEFAULT 15 CHECK(reminder_min >= 0)
);
CREATE TABLE meeting_tasks (meeting_id TEXT NOT NULL REFERENCES meetings(id), task_id TEXT NOT NULL REFERENCES tasks(id), PRIMARY KEY(meeting_id,task_id));
CREATE TABLE blocks (
  id TEXT PRIMARY KEY NOT NULL, date TEXT NOT NULL CHECK(date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  start_min INTEGER NOT NULL CHECK(start_min >= 0 AND start_min < 1440),
  end_min INTEGER NOT NULL CHECK(end_min > start_min AND end_min <= 1440),
  kind TEXT NOT NULL CHECK(kind IN ('task','break','lunch','focus')),
  task_id TEXT REFERENCES tasks(id), done INTEGER NOT NULL DEFAULT 0 CHECK(done IN (0,1)),
  done_at TEXT, carried_from_block_id TEXT REFERENCES blocks(id),
  CHECK((kind = 'task' AND task_id IS NOT NULL) OR (kind != 'task' AND task_id IS NULL))
);
CREATE TABLE time_entries (
  id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), block_id TEXT REFERENCES blocks(id),
  started_at TEXT NOT NULL, ended_at TEXT NOT NULL CHECK(ended_at >= started_at),
  minutes REAL NOT NULL CHECK(minutes >= 0)
);
CREATE TABLE alerts (id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), offset_min INTEGER NOT NULL CHECK(offset_min >= 0), UNIQUE(task_id,offset_min));
CREATE TABLE summaries (
  id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), date TEXT NOT NULL,
  body_mrkdwn TEXT NOT NULL, sent_at TEXT, sent_via TEXT
);
CREATE TABLE settings (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
INSERT INTO settings(key,value) VALUES ('close_to_tray','false');

CREATE INDEX tasks_project_status ON tasks(project_id,status);
CREATE INDEX tasks_due ON tasks(due_at) WHERE status != 'done';
CREATE INDEX subtasks_task ON subtasks(task_id,sort_order);
CREATE INDEX task_tags_tag ON task_tags(tag_id);
CREATE INDEX attachments_task ON attachments(task_id);
CREATE INDEX meetings_project_start ON meetings(project_id,starts_at);
CREATE INDEX meeting_tasks_task ON meeting_tasks(task_id);
CREATE INDEX blocks_date ON blocks(date,start_min);
CREATE INDEX blocks_task ON blocks(task_id);
CREATE INDEX time_entries_task ON time_entries(task_id,started_at);
CREATE INDEX time_entries_block ON time_entries(block_id);
CREATE INDEX summaries_project_date ON summaries(project_id,date);

-- The aggregate is authoritative. Triggers execute in the writer's transaction,
-- including when a later milestone edits/reassigns an entry.
CREATE VIEW task_time_totals AS
SELECT tasks.id AS task_id, COALESCE(SUM(time_entries.minutes),0)/60.0 AS hours_worked
FROM tasks LEFT JOIN time_entries ON time_entries.task_id = tasks.id GROUP BY tasks.id;
CREATE TRIGGER time_entries_insert AFTER INSERT ON time_entries BEGIN
  UPDATE tasks SET hours_worked = (SELECT hours_worked FROM task_time_totals WHERE task_id = NEW.task_id) WHERE id = NEW.task_id;
END;
CREATE TRIGGER time_entries_update AFTER UPDATE ON time_entries BEGIN
  UPDATE tasks SET hours_worked = (SELECT hours_worked FROM task_time_totals WHERE task_id = tasks.id) WHERE id IN (OLD.task_id,NEW.task_id);
END;
CREATE TRIGGER time_entries_delete AFTER DELETE ON time_entries BEGIN
  UPDATE tasks SET hours_worked = (SELECT hours_worked FROM task_time_totals WHERE task_id = OLD.task_id) WHERE id = OLD.task_id;
END;
CREATE TRIGGER tasks_cache_insert BEFORE INSERT ON tasks WHEN NEW.hours_worked != 0 BEGIN
  SELECT RAISE(ABORT,'hours_worked is derived from time_entries');
END;
CREATE TRIGGER tasks_cache_update BEFORE UPDATE OF hours_worked ON tasks
WHEN ABS(NEW.hours_worked - (SELECT hours_worked FROM task_time_totals WHERE task_id = NEW.id)) > 0.000000001 BEGIN
  SELECT RAISE(ABORT,'hours_worked is derived from time_entries');
END;
CREATE TRIGGER entry_block_insert BEFORE INSERT ON time_entries
WHEN NEW.block_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM blocks WHERE id = NEW.block_id AND task_id = NEW.task_id) BEGIN
  SELECT RAISE(ABORT,'entry and block must belong to the same task');
END;
CREATE TRIGGER entry_block_update BEFORE UPDATE OF block_id,task_id ON time_entries
WHEN NEW.block_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM blocks WHERE id = NEW.block_id AND task_id = NEW.task_id) BEGIN
  SELECT RAISE(ABORT,'entry and block must belong to the same task');
END;
