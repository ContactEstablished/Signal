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
