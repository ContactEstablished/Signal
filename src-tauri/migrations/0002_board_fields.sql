ALTER TABLE tasks ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0 CHECK(sort_order >= 0);
ALTER TABLE tasks ADD COLUMN blocked_since TEXT;
ALTER TABLE tasks ADD COLUMN revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0);
UPDATE tasks SET sort_order = (SELECT COUNT(*) FROM tasks AS preceding
  WHERE preceding.project_id=tasks.project_id AND preceding.status=tasks.status
  AND (preceding.created_at < tasks.created_at OR (preceding.created_at=tasks.created_at AND preceding.id < tasks.id)));
CREATE INDEX tasks_board_order ON tasks(project_id,status,sort_order,id);
CREATE TABLE attachment_file_ops (
 token TEXT PRIMARY KEY NOT NULL, relative_path TEXT UNIQUE NOT NULL,
 filename TEXT NOT NULL, size INTEGER NOT NULL CHECK(size >= 0), mime TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('staging','ready','delete_pending')),
 created_at TEXT NOT NULL, last_error TEXT
);
CREATE INDEX attachment_ops_state ON attachment_file_ops(state);
