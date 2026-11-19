-- AI Inbox gains an 'email' review. SQLite cannot change a CHECK constraint, so
-- intake_drafts is rebuilt. intake_images is rebuilt with it, child first, so no
-- image review is lost to ON DELETE CASCADE while the parent table is replaced.
CREATE TABLE intake_drafts_v2 (
 id TEXT PRIMARY KEY NOT NULL,
 revision INTEGER NOT NULL DEFAULT 0,
 payload TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 source_kind TEXT NOT NULL DEFAULT 'voice' CHECK(source_kind IN ('voice','image','email'))
);
INSERT INTO intake_drafts_v2(id,revision,payload,updated_at,source_kind)
 SELECT id,revision,payload,updated_at,source_kind FROM intake_drafts;
CREATE TABLE intake_images_v2 (
 id TEXT PRIMARY KEY NOT NULL,
 draft_id TEXT NOT NULL UNIQUE REFERENCES intake_drafts_v2(id) ON DELETE CASCADE,
 mime TEXT NOT NULL,
 width INTEGER NOT NULL,
 height INTEGER NOT NULL,
 bytes BLOB NOT NULL
);
INSERT INTO intake_images_v2(id,draft_id,mime,width,height,bytes)
 SELECT id,draft_id,mime,width,height,bytes FROM intake_images;
DROP TABLE intake_images;
DROP TABLE intake_drafts;
ALTER TABLE intake_drafts_v2 RENAME TO intake_drafts;
ALTER TABLE intake_images_v2 RENAME TO intake_images;
CREATE UNIQUE INDEX intake_source_draft ON intake_drafts(source_kind);
