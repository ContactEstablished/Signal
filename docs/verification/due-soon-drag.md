# Due Soon calendar dragging

Development-only change, September 14, 2026. The installed personal copy was not rebuilt, installed, restarted, or migrated.

## Behavior

- Drag a task from Due Soon or Unscheduled by its row, grip, or title. A short click still opens the task; the plus button still opens the time editor.
- After a small movement threshold, a translucent task rectangle follows the pointer. Over the calendar, a one-hour preview snaps to the quarter-hour grid and shows the destination time. Dragging near the calendar's top or bottom scrolls it.
- Releasing over the calendar saves one editable 60-minute task block, independent of the task's estimate or logged time. Near midnight, the full-hour range is limited to 23:00–24:00.
- Due Soon hides tasks that have a persisted block on the displayed date. This applies to existing blocks too. Removing the block or moving it to another day makes the task eligible again. Other dates have their own scheduling state.
- A cancelled or failed drop leaves the task in the sidebar. Escape, pointer cancellation, lost capture, window blur, date/scope changes, and pending recovery cancel the drag. A drag does not trigger the title's open action.
- Task due dates, estimates, and recorded time are not changed by scheduling.

## Implementation and verification

Sidebar dragging uses pointer capture, as the Board already does. The installed Tauri configuration schema states that HTML5 frontend drag/drop on Windows requires disabling native drag/drop. Native drop handling is used for file attachments, so it remains enabled; task dragging no longer depends on HTML5 drag events.

Component and integration coverage checks the floating and calendar previews, saved duration, removal only after persistence, failed saves, cancellation, recovery, task clicks and keyboard activation, restored sidebar eligibility, quarter-hour snapping, midnight boundaries, and editing the resulting block. Existing planner movement, resize, and block editor tests are included.

Results: 30 tests passed across four relevant suites; `pnpm check` passed with zero errors and warnings; `pnpm build` passed with the existing chunk-size advisory. The personal executable hash remained unchanged.

Visual verification is pending because the computer/browser tool returned no available apps or browsers. No personal data was used for testing.
