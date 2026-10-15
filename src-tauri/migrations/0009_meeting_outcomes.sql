-- Keep marked cancellations on the calendar. The older cancelled flag remains
-- a removal tombstone; existing occurrence attendance, notes and links stay intact.
ALTER TABLE meeting_occurrences ADD COLUMN marked_cancelled INTEGER NOT NULL DEFAULT 0 CHECK(marked_cancelled IN (0,1));
