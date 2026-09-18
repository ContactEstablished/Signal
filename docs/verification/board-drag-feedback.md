# Board dragging feedback

Development-only change following the 0.1.1 personal release. The installed application, installer, and personal data were not changed.

## Behavior

- Dragging a board task shows the same translucent, pointer-following card used for Due Soon scheduling. Both views now share that visual component.
- The original card fades, and the destination column shows a dashed, tinted card preview in the intended insertion slot. The floating card names the destination status.
- Drag from the title/card body or its existing grip. A short click still opens the task, keyboard activation still works, and the status menu remains available.
- Release moves the task using the existing persistence path. A rejected save restores the original card and reports the error; repeated pointer-up events cannot submit twice.
- Escape, lost pointer capture, pointer cancellation, window blur, project/date/filter changes, and leaving the board clear the relevant drag feedback without writing. Leaving the board removes the destination preview; release outside cancels the move.
- Existing insertion-slot stabilization and keyboard focus restoration remain in place. Board scrolling is limited to dragging within the board's bounds.

## Verification

All 30 relevant tests passed across the Board, board-drag, planner-canvas, and Your Day suites, including the final board-drag rerun. Coverage exercises pointer-following feedback, body and grip dragging, click suppression, keyboard activation, empty In Progress/Blocked/Done columns, insertion order, save rejection, cancellation, project changes, and the existing Due Soon scheduling workflow.

`pnpm check` passed with zero errors and warnings. `pnpm build` passed with the existing chunk-size advisory. Desktop visual verification remains pending.


## September 16: drag cancelled by the clock

The user reported that v0.1.2 started dragging a project Board task and then abruptly stopped. The new regression reproduced cancellation on the first one-second clock tick, without a pointer cancellation or failed database write.

Cause: Board's cancellation effect directly read `selectedDate`, which is supplied by the live workspace clock getter. Each clock update reran that effect even when the local date stayed the same. Earlier Board tests used a constant clock and missed this dependency.

Fix: derive a stable value from the project ID, local calendar date, timezone and filters, then cancel only when that value changes. Clock ticks on the same day leave the drag intact; actual navigation, filter changes, midnight, lost capture, Escape and window blur still cancel safely. No pointer-capture workaround or visual rollback was needed.

Verification:

- Observed the new same-day clock regression fail before the fix (11 original tests passed; the new case failed).
- After the fix, all **16 tests** across `board-drag.test.ts` and `board.test.ts` pass. New cases cover same-day ticks, local midnight, and slow body/grip drags through three successive clock updates followed by exactly one saved drop.
- `pnpm check`: zero errors and warnings.
- `pnpm build`: pass, retaining the existing non-blocking chunk advisory.
- Development launched with `. ./scripts/dev-env.ps1` and `pnpm dev:seed`; the fixture app is left running for review.
- Native visual verification remains unperformed: Computer Use returned “native pipe is unavailable” on discovery, retry and after reset. The browser control also reported no available browser. Component regression evidence is not a native drag sign-off.
- Installed personal v0.1.2 process and executable were left unchanged. No personal data was read, seeded, edited or migrated for this fix. No installer was rebuilt or installed.

Before the next personal release, use **Signal · September 2025 fixture → a project → Board**. Hold a task by its title for at least three seconds, move it across lanes, pause and drop; confirm it persists in the destination. Repeat with the grip and Escape cancellation. Installation of the corrected build requires the user's explicit request under AGENTS.md.
