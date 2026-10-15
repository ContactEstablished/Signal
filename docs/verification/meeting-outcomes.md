# Meeting outcomes from the calendar

Implemented October 6, 2026, then installed as personal version 0.1.6 at the user's explicit request. Tests use synthetic fixtures and temporary databases. See the [installation and preservation record](personal-install-0.1.6.md).

## Behavior

- Hover or focus a meeting in Your Day to reveal the checkmark (Attended) and circled X (Canceled). Actions are also visible in the block details menu and on touch devices.
- Attended shows a green status label. Canceled keeps the meeting at its original time, with a gray, struck-through title and a Canceled label. Join is hidden for canceled meetings.
- Click the selected action again to clear the mark. Either action can replace the other. Existing notes, task links, and schedule details are preserved.
- Marks apply only to the selected recurring occurrence. Today and Week also expose the actions, and calendar summaries preserve the canceled styling. Today does not highlight a canceled or attended meeting as the next meeting.
- Canceled meetings remain on the timeline but do not reserve time for automatic planning or contribute to planned minutes. Restoring them reserves their time again.
- Saves use existing revision checks, transaction boundaries, and exact-request recovery. Buttons lock while saving; failures show an error without changing the displayed outcome.

## Storage compatibility

Migration 9 adds a default-zero `marked_cancelled` column to `meeting_occurrences`. It preserves attendance history, notes, snapshots, task links, and the existing removal tombstone. The API represents retained cancellations as attendance `canceled`; the old removal actions retain their existing behavior.

## Verification

- 51 frontend tests passed across meeting-outcomes, planner-rules, planner-canvas, today, week, meeting-editor, and agenda-state. Coverage includes compact bars, undo, preserved notes, concurrency metadata, disabled/duplicate writes, rejected saves, and retained cancellations releasing planning time.
- Full native suite: 77 passed, 2 opt-in live-provider tests ignored. New tests verify additive migration compatibility, database reopen persistence, recurrence isolation, schedule/notes preservation, exact-request replay, stale-write rejection, combined meeting edits, and planner visibility/occupancy.
- `pnpm check`: zero errors or warnings.
- `pnpm build`: passed with the existing bundle-size advisory.
- `git diff --check`: passed.

Visual/native click-path verification is pending: the computer-use connector reported no available browsers. The temporary synthetic preview files and development server were removed after that check; no personal data was used.

## Manual acceptance

Run development with the default configuration and `pnpm dev:seed`. In Your Day, hover a meeting and mark it attended, clear the mark, then mark it canceled. Confirm that the canceled bar stays at the same time, turns gray, and has a struck-through title. Relaunch development and confirm persistence. Restore it, verify notes in the editor, and check that other occurrences of a recurring meeting retain their own marks. Use Tab/Enter and the details menu to verify keyboard access.
