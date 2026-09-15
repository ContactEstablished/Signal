# Meeting time and weekday improvements

Implemented September 14, 2026, in development only. The personal installation is awaiting the user's explicit request to go live.

## Behavior

- Meeting time uses separate hour, minute, and AM/PM selectors. New meetings default to the next whole hour, with minute `00`. Minute choices are `00` through `55` in five-minute steps.
- Selecting hours 7–11 sets AM; selecting 12 or 1–6 sets PM. An explicit AM/PM change is retained when minutes change.
- Timezone choices are Eastern (`America/New_York`), Central (`America/Chicago`), Mountain (`America/Denver`), and Pacific (`America/Los_Angeles`), including their daylight-saving rules.
- Duration shortcuts are 15m, 25m, 30m, 45m, 55m, and 1h.
- Weekly recurrence has Monday–Friday toggle buttons, with at least one day required. If the start date is not selected, the form identifies the first selected day on or after it and uses that date for the first meeting.
- Recurrence changes on existing series require the existing “This and following” scope and review. A single-occurrence edit cannot change the weekday pattern.

## Compatibility

Migration `0006_meeting_weekdays.sql` adds a nullable weekday array to meeting segments. Existing segments keep their original weekly anchor and occurrence identities. Do not modify or reapply migration 0005.

Saved times are not silently rounded when editing unrelated fields. An existing non-five-minute value is retained as a saved option, and its seconds/milliseconds are preserved until the user edits the time. Existing timezones outside the four choices remain available as a saved option. Existing weekend recurrence is retained and shown as a selected weekend button when applicable.

## Verification

- `pnpm check`: zero errors and warnings.
- `pnpm test`: 119 tests passed across 33 files.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 46 tests passed.
- `pnpm build`: passed, with the existing Vite chunk-size advisory.
- Added checks cover hour defaults and manual override, minute choices, duration shortcuts, timezone conversion, weekday validation and submission, following-edit scope, persistence after reopening, Today/Week projection, bounded queries years after the anchor, daylight-saving transitions, recurrence end, cancellation, historical meeting preservation, notes, and migration of an existing weekly schedule.
- Personal executable SHA-256 remained `24CC0EFE71EFBED51AFF3D710D1F43860E463025D638BF0E8D6D2275A0F217FB`. The installed process retained its original start time. No personal database was opened or migrated, and no personal installer was built or run for this change.
- Visual inspection remains pending: the browser tool reported “No browser is available.” The temporary preview server was stopped and its preview files removed. Automated component tests passed; native visual acceptance is not claimed.

## Manual review when requested

Use the development fixture workspace before an authorized personal update. Create Monday–Thursday, Monday/Wednesday, and Monday/Tuesday meetings and inspect Today, Week, and Your Day. Exercise the time selectors, timezone list, and duration shortcuts. Reopen a meeting to check the saved weekdays; edit one occurrence and then future occurrences to review their scopes.

Only build/install the personal update after the user asks to push it live, with the existing backup and isolation rules in `AGENTS.md`.
