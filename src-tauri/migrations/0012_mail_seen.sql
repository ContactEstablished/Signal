-- Gmail messages Signal has already sent for task suggestions, so each email
-- is analyzed once. Only Gmail's opaque message IDs are kept, never content.
CREATE TABLE mail_seen (
 account_id TEXT NOT NULL,
 message_id TEXT NOT NULL,
 seen_at TEXT NOT NULL,
 PRIMARY KEY(account_id, message_id)
);
