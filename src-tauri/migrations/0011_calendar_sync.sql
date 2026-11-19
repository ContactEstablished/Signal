-- Meetings imported from an external calendar (Google Calendar first).
-- One row per event instance. The meeting itself is an ordinary Signal meeting.
-- meeting_id becomes NULL when the meeting is deleted in Signal, so a later sync
-- does not bring it back; the sync deletes the row itself when the event goes away.
-- fingerprint is a hash of the synced fields, or 'canceled' while the event is
-- cancelled but the meeting is kept because it has notes, attendance or tasks.
CREATE TABLE calendar_links (
 account_id TEXT NOT NULL,
 calendar_id TEXT NOT NULL,
 event_id TEXT NOT NULL,
 meeting_id TEXT UNIQUE REFERENCES meetings(id) ON DELETE SET NULL,
 starts_at TEXT NOT NULL,
 fingerprint TEXT NOT NULL,
 PRIMARY KEY(account_id, calendar_id, event_id)
);
CREATE INDEX calendar_links_window ON calendar_links(account_id, calendar_id, starts_at);
