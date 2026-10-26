-- Telegram bot intake. Updates are stored before Telegram is told to forget them,
-- every reply is written to the outbox in the same transaction as its effects,
-- and applied proposals are recorded so a retry can never change data twice.
CREATE TABLE telegram_updates (
 update_id INTEGER PRIMARY KEY NOT NULL,
 received_at TEXT NOT NULL,
 payload TEXT NOT NULL,
 processed INTEGER NOT NULL DEFAULT 0 CHECK(processed IN (0,1))
);
CREATE INDEX telegram_updates_pending ON telegram_updates(update_id) WHERE processed=0;
CREATE TABLE telegram_flow (
 chat_id INTEGER PRIMARY KEY NOT NULL,
 state_json TEXT NOT NULL
);
CREATE TABLE telegram_outbox (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 chat_id INTEGER NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('send','edit','answer')),
 message_id INTEGER,
 callback_id TEXT,
 body TEXT NOT NULL,
 markup_json TEXT,
 flow_token TEXT
);
CREATE TABLE telegram_applied (
 proposal_id TEXT PRIMARY KEY NOT NULL,
 applied_at TEXT NOT NULL,
 summary TEXT NOT NULL
);
