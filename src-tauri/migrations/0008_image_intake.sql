DROP INDEX intake_single_draft;
ALTER TABLE intake_drafts ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'voice' CHECK(source_kind IN ('voice','image'));
CREATE UNIQUE INDEX intake_source_draft ON intake_drafts(source_kind);
CREATE TABLE intake_images (
 id TEXT PRIMARY KEY NOT NULL,
 draft_id TEXT NOT NULL UNIQUE REFERENCES intake_drafts(id) ON DELETE CASCADE,
 mime TEXT NOT NULL,
 width INTEGER NOT NULL,
 height INTEGER NOT NULL,
 bytes BLOB NOT NULL
);
