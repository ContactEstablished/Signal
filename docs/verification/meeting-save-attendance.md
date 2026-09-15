# Save meeting attendance and notes

Development-only fix, September 14, 2026. The installed personal copy and its data were not changed.

## Behavior

- Save meeting now saves meeting details, attendance, and occurrence notes together, then closes the editor after success.
- A failed save keeps the editor open and preserves the draft for retry. Schedule and occurrence changes commit in one database transaction, with the existing request identity protecting retries from duplicate changes.
- For a recurring meeting, attendance and occurrence notes apply only to the selected occurrence, including when updating this and following meetings.
- Save attendance & notes remains available as a separate shortcut. It stays in the editor and preserves any unsaved meeting detail changes.

## Verification

- 25 frontend tests passed across meeting-editor and agenda-state suites, covering combined saves, attendance-only changes through the main button, successful close, failure preservation, receipt replay, and the separate attendance shortcut.
- 22 native agenda tests passed, including transaction rollback on an induced attendance-write failure, exact-request replay, database reopen persistence, recurring occurrence isolation, and invalid attendance payload rejection.
- `pnpm check` passed with zero errors and warnings.
- `pnpm build` passed with the existing chunk-size advisory.
- `git diff --check` passed.

Visual verification remains pending. Automated tests used fixtures and temporary databases, not personal application data.

Until the development fix is released, use Save attendance & notes in the installed copy to persist attendance before closing the meeting.
