# AM/PM time display

Development-only change, September 14, 2026. The personal installation and its data were not changed.

## Behavior

- Calendar hours, current-time markers, block ranges, drag previews, task pickers, and planning previews use 12-hour AM/PM labels, without leading zeros on hours.
- Project meeting timelines, meeting end-time hints, recurrence change previews, and linked-meeting details use the same convention. Existing localized time-history labels explicitly request AM/PM.
- Deadline and time-log fields use a shared AM/PM text input, independent of the operating system's native time-picker format. It accepts shorthand such as `2pm` and `2:30`, formats on blur, and keeps invalid text available for correction. Existing fractional times are preserved when untouched.
- Planner endpoints still use minute offsets and canonical wall times internally. Midnight at the end of a day displays as `12:00 AM (next day)` and round-trips to minute 1440; noon remains `12:00 PM`.
- The calendar gutter is wider to accommodate AM/PM labels. Selection, drop hit areas, previews, and the current-time line follow that gutter.

## Verification

All 71 relevant tests pass across 11 suites, including the corrected AM/PM expectations and the final input/calendar reruns. Coverage includes display and input formatting, existing timestamp precision, invalid input, noon and midnight round-trips, timezone conversion, block editing, dragging and reopening a scheduled block, meeting saves, deadline recovery, and time-log draft preservation.

`pnpm check` passes with zero errors and warnings. `pnpm build` passes with the existing chunk-size advisory. Visual verification in the desktop application remains pending; automated tests use fixtures rather than personal data.
