# Task 1-2 — Projects and Board

Status: complete (September 12, 2026). Implementation and automated checks are complete; the user confirmed all manual acceptance checks and authorized the milestone commit and push. See [M1 execution evidence and user sign-off](../../../verification/M1.md).

## Source of Truth

- [M1 overview](Phase-1-Overview.md), [approved decisions](../Phase-1-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-1-2.md).
- [Roadmap](../../../../PLAN.md), M1.
- [Handoff](../../../../_design/design_handoff_signal/README.md), Project → Board, and [accepted reference](../../../../_design/design_handoff_signal/Signal.standalone.html), #1b.

## Initial Starting Point

M0 supplies project tabs, fonts, tokens, SQLite records, and destination previews. `src/lib/domain/dates.ts` supplies local-calendar classification. No Board or project editor exists. Task 1-1's read models/mutations are planned prerequisite contracts, not M0 APIs.

## Goal

Deliver project-management presentation and a five-column Board with persistent status/order changes, filters, record-derived cards/counts, and a project-scoped meetings strip.

## Exact Scope

Exclusively create `src/lib/views/Board.svelte`; `src/lib/components/board/{ProjectSubBar,BoardColumn,TaskCard,BoardFilters}.svelte`; `src/lib/components/projects/{ProjectMenu,ProjectDialog}.svelte`; `src/lib/components/meetings/MeetingsStrip.svelte`; `src/lib/domain/board.ts`; and `tests/unit/board.test.ts`.

Include project create/edit/reorder/delete presentation; Board/Week/Notes controls; task selection/creation callbacks; filters; drag ordering/status changes; pending, empty, error, and selection states. Component-specific CSS stays in owned files and uses existing tokens.

## Non-Goals

No task forms, Markdown, attachments, timers, planner, meeting editing/recurrence, Week or Notes bodies. Do not change schema, native commands, dependencies, seed data, App.svelte, shared state, or global styles. Preserve all unrelated/concurrent changes and pre-existing kickoff documents; do not stage another owner's files.

## Dependencies

Task 1-1 supplies types and native command contracts. Task 1-4 supplies fetching, clock/date props, shared mutation serialization/refresh, routing, and final mounting. Task 1-3 supplies task dialogs. Tasks 1-2 and 1-3 can work independently after the 1-1 contract is published; no import of a Task 1-4 component is required.

## Step-by-step Work

1. Verify Task 1-1 types/signatures instead of inventing another API.
2. Implement pure filtering, grouping, due presentation, meeting geometry, and drop-intent helpers.
3. Build sub-bar, project menu/native HTML dialog, and keyboard-accessible filters.
4. Render five columns/cards and the meetings strip using accepted visual treatments.
5. Wire callbacks and pending/errors; keep committed state on mutation failure.
6. Implement filtered drag insertion and an accessible status-change alternative.
7. Add meaningful helper tests, then hand contracts to the integrator.
8. Perform mounted native checks after Task 1-4 integration and record actual results.

## Test Expectations

Cover combined filters, local date/DST boundaries, deterministic ordering, filtered-drop intents, meeting clipping, zero estimates, and overdue/done presentation. Runtime checks cover project changes, focus/keyboard, filtered drag, failure retention, restart, and both supported sizes. Retain M0 tests; no component-existence tests.

## Verification Commands

From repository root:

```powershell
pnpm check
pnpm test
pnpm build
```

After Task 1-4 mounts the components:

```powershell
. ./scripts/dev-env.ps1
pnpm dev:seed
```

Use `pnpm tauri dev` for ordinary-data checks after quitting the fixture instance. Record unmounted/native checks as deferred, never passed by compilation alone.

## Acceptance Criteria

- Project create/edit, Move left/right, and affected-record deletion confirmation work with four approved colors.
- Accepted five-column/card anatomy and selected/blocked/done states render; fixture Atlas counts remain 3/3/2/1/2.
- Filters combine correctly, filtered counts show visible/total, and clearing restores full order.
- Drag persists status/order while retaining hidden relative order and handling stale reads/errors.
- Meetings are selected-project/day records with fixture-aware now display.
- Task controls and future project subviews route through callbacks with honest stubs.
- The common project sub-bar remains usable on Week/Notes previews; project editors use the integrator's native guard and close protocol.
- Both required sizes remain usable with local fonts and token styling.

## Review Checklist

- [x] File ownership and shared contracts match Tasks 1-1/1-4.
- [x] D1–D4 and deletion-preview conflicts are respected.
- [x] No independent mutable business store or clock exists here.
- [x] Empty/filter-empty/loading/error/pending states are distinct.
- [x] Keyboard operation, drag cancellation, and focus return work.
- [x] Verification is recorded honestly; final commit belongs to Task 1-4.
