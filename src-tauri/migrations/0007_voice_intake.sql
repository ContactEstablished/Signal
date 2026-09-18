CREATE TABLE intake_drafts (
 id TEXT PRIMARY KEY NOT NULL,
 revision INTEGER NOT NULL DEFAULT 0,
 payload TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX intake_single_draft ON intake_drafts ((1));
CREATE TABLE intake_receipts (
 request_id TEXT PRIMARY KEY NOT NULL,
 fingerprint TEXT NOT NULL,
 result TEXT NOT NULL,
 created_at TEXT NOT NULL
);
