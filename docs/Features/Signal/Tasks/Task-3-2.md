# Task 3-2 — Planner canvas, blocks and task picker

Status: **complete; M3 manually verified and approved September 13, 2026.** See [M3 verification](../../../verification/M3.md) for actual automated/native results, final user sign-off, and historical automation limitations. D1–D8 remain approved.

## Source of Truth

[Overview](Phase-3-Overview.md), [decisions](../Phase-3-Decisions.md), [paired spec](../ImplementationSpecs/ImplementationSpec-3-2.md), [PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), accepted #2a/#2b/#3e in [HTML](../../../../_design/design_handoff_signal/Signal.standalone.html).

## Initial Starting Point

M2 baseline `main` at `fbd1f58` has no planner components; Your Day is an App placeholder. Tokens, local fonts, Lucide and injected timer arithmetic exist. Task 3-1's domain contracts are planned prerequisites. Existing component tests use jsdom/Svelte mount/unmount/flushSync.

## Goal

Deliver callback-driven planner presentation and accessible pointer/keyboard interactions, leaving persistence and coordination to Task 3-3.

## Exact Scope

Create `src/lib/components/planner/PlannerCanvas.svelte`, `PlannerBlock.svelte`, `TaskPicker.svelte`; `tests/unit/planner-canvas.test.ts`. Additional presentation types may be exported from component module scripts; shared DTOs belong to Task 3-1.

## Non-Goals

No SQL/native calls, global state, App/YourDay/sidebar/shared-token/fixture/dependency edits, recurrence/editing/attendance, summaries or notifications. No independent timer loop or task automation. Preserve unrelated changes and another owner's files.

## Dependencies

Task 3-1 records/display items/lane helpers; Task 3-3 supplies selection, clock, pending/error state, keyboard BlockEditor and awaited callbacks. No second write queue.

## Step-by-step Work

1. Reconcile typed props with Tasks3-1/3-3.
2. Build internally scrolling24h grid/gutter/hours/fades/past/now.
3. Render packed task/neutral/meeting blocks and accessible compact actions.
4. Add selection/move/resize/task drag-in, autoscroll and cancellation.
5. Implement anchored search picker with task/Break/Lunch/Focus choices and keyboard access.
6. Test callbacks/geometry/error recovery, then hand off for native integration.

## Test Expectations

Cover scrolled coordinate conversion,15-minute snap, day edges/reverse selection, retained move duration, resize minimum, cancellation/lost capture, one submit, rejection restoration, compact accessibility, meeting read-only controls, picker search/arrows/Enter/Escape. Domain packing correctness belongs to Task 3-1.

## Verification Commands

```powershell
pnpm check
pnpm test -- tests/unit/planner-canvas.test.ts
pnpm test
pnpm build
```

After Task 3-3 integration: source `scripts/dev-env.ps1`, run `pnpm dev:seed`; actual native checks remain pending until observed.

## Acceptance Criteria

- 60px/hour,64px gutter,24h internal scrolling and07:00 initial position.
- Manual selection/move/resize snaps15 minutes, stays inside the date and permits intentional overlaps.
- Painted26px lanes never change stored durations;3 lanes full,4+ compact with accessible details.
- Picker cancellation writes nothing; successful changes render from authoritative props.
- Running/paused/done/neutral/meeting states are distinct; no meeting mutations.
- Gestures recover after cancel or callback rejection; keyboard creation/edit/actions available.
- No page-level horizontal overflow at both supported window sizes.

## Review Checklist

- [x] Props and ownership agree across pairs.
- [x] No persistence or independent clock in components.
- [x] Painted versus logical geometry stays separate.
- [x] Pointer/keyboard/cancel/failure paths tested.
- [x] Compact details and meeting restrictions accessible.
- [x] Task 3-3 owns native evidence and milestone commit.
