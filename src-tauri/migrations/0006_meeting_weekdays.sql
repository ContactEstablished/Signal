-- NULL preserves legacy weekly schedules and their occurrence identities.
-- Explicit weekdays use ISO numbering (Monday=1, Sunday=7).
ALTER TABLE meeting_segments ADD COLUMN repeat_weekdays TEXT
  CHECK(repeat_weekdays IS NULL OR (json_valid(repeat_weekdays) AND json_type(repeat_weekdays)='array'));
