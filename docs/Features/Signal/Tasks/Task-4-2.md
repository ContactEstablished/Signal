# Task 4-2 — Today digest and project Week presentation

Status: **production implementation delivered; required native/manual acceptance pending.** D1–D8 approved September 13, 2026. See [M4 verification](../../../verification/M4.md) for actual evidence and remaining gates. Frontend/component implementation and regression tests pass.

## Source of Truth

[Overview](Phase-4-Overview.md), [approved decisions](../Phase-4-Decisions.md), [paired specification](../ImplementationSpecs/ImplementationSpec-4-2.md), [PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), accepted Today #2d and Week #3d in [HTML](../../../../_design/design_handoff_signal/Signal.standalone.html). Explicit decisions override ambiguous mock content. Dated subtasks remain display-only reference content.

## Initial Starting Point

M3 baseline is `main` at `0ee20b4`, accepted and pushed. `src/App.svelte` renders Today and project Week placeholders. Existing Workspace owns the shared clock, revisioned task mutations and queue; `YourDay.planTask` opens planning for an existing task. Existing deadline helpers only expose minute precision and must not be used to round Week moves. Existing Board provides the accepted translucent destination card/faded source pattern. Local fonts, Lucide, tokens and jsdom/Svelte component testing exist.

Task 4-1 agenda snapshots/calendar helpers are planned prerequisites, not baseline APIs. The only initial M4 change was the decision brief; preserve it and any concurrent owner changes.

## Goal

Render the Today digest and project Week from authoritative snapshots, with discoverable task/meeting navigation, calendar-correct totals and safe due-date movement by pointer and keyboard.

## Exact Scope

Create only:

- `src/lib/views/Today.svelte` and `Week.svelte`.
- `src/lib/components/agenda/AgendaTaskCard.svelte`, `MeetingPill.svelte`, `WeekStrip.svelte`, `DueDateDialog.svelte`.
- `tests/unit/today.test.ts`, `week.test.ts`, `due-date-dialog.test.ts`.

Use Svelte 5 runes, TypeScript, local fonts, Lucide and existing CSS custom properties. Export local presentation types from component module scripts when necessary; Task 4-1 owns shared DTOs.

## Non-Goals

No native calls, persistence, new clocks, queues, App/Workspace/state/shared-token/fixture/dependency edits, occurrence expansion or meeting mutation/editor implementation. No task status/timer/block mutation as a side effect of moving a deadline. No general activity feed, subtask deadlines, summaries, notification delivery or M8 intake. Preserve unrelated changes; do not stage or commit another owner's files.

## Dependencies

Task 4-1 supplies authoritative Today/Week snapshots, occurrence identity and calendar/deadline helpers with full precision. Task 4-3 mounts views and DueDateDialog, owns navigation, mutation/recovery guards, draft close coordination and all native interactions. Views accept awaited callbacks and never establish an independent mutation queue.

## Step-by-step Work

1. Reconcile snapshots and callback signatures with Tasks 4-1/4-3.
2. Build shared task rows/cards, meeting pills and the seven-day strip.
3. Implement Today sections, counts, meeting actions and completed-hours bars from supplied data.
4. Implement Week days, Later/No date, off-week overdue access and internal vertical scrolling without page horizontal overflow.
5. Implement translucent pointer movement and its keyboard equivalent; mount the presentation-only due editor through Task 4-3 callbacks.
6. Test date-sensitive presentation, cancellation/failure and accessibility, then hand off for native integration.

## Test Expectations

Cover Monday–Sunday and timezone boundaries; Done exclusion in digest buckets versus struck-through weekly items; hidden planner meetings still present; completed-only hours; plan/open/join callbacks; same-ID multi-day occurrence rendering; week navigation; precise date-only movement; No date/Later/no-deadline editor behavior; pointer cancel and duplicate release; callback failure; gap/fold prompts; editor focus and retained rejected values. Domain calendar arithmetic belongs to Task 4-1; component assertions establish UI behavior, not database durability.

## Verification Commands

From repository root:

```powershell
pnpm check
pnpm test -- tests/unit/today.test.ts tests/unit/week.test.ts tests/unit/due-date-dialog.test.ts
pnpm test
pnpm build
```

After Task 4-3 integration:

```powershell
. ./scripts/dev-env.ps1
pnpm dev:seed
```

Runtime checks remain pending until observed.

## Acceptance Criteria

- Today displays the current workspace date, Monday–Sunday strip, unfinished Overdue/Due today/Tomorrow, meetings and completed hours by project.
- Earlier times today remain Due today; live timers do not inflate completed hours.
- Week shows seven local date columns, meetings, compact tasks, Later and No date; Done remains visible with strike-through.
- Hidden-in-planner meetings remain visible; cancellation and recurrence data come from the shared occurrence authority.
- Moving a task changes only due_at, preserving local time including seconds/milliseconds; ambiguous/nonexistent/no-deadline cases enter an explicit editor.
- Dragging shows a translucent destination card, restores on cancellation/rejection and becomes authoritative after commit. Keyboard users can make the same move.
- No page horizontal overflow at 1200×760 or 1600×960; truncated content remains accessible.
- Pending/loading/error/unknown states do not masquerade as empty or successfully saved data.

## Review Checklist

- [ ] Owned files and DTO/callback contracts agree across all three pairs.
- [ ] Calendar buckets and hours use shared clock/zone and supplied authoritative records.
- [ ] Due precision survives date changes and no unrelated task fields change.
- [ ] Picker/cancel/failure/recovery and native close participation have defined owners.
- [ ] Pointer and keyboard workflows expose the same destinations.
- [ ] Reference comparison and minimum-window tests have actual evidence before completion.
- [ ] Task 4-3 owns final phase verification, docs and milestone commit.
