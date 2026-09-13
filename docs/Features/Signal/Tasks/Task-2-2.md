# Task 2-2 — Task timer controls and manual time logging

Status: complete; M2 manually accepted September 13, 2026. D1–D6 remain approved. See [M2 verification](../../../verification/M2.md) for final results.

## Source of Truth

- [Phase overview](Phase-2-Overview.md), [D1–D6](../Phase-2-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-2-2.md).
- [Roadmap](../../../../PLAN.md), M2; [handoff](../../../../_design/design_handoff_signal/README.md); [accepted HTML](../../../../_design/design_handoff_signal/Signal.standalone.html), task detail #1b and active timers #2b.
- D1–D6 were approved by the user on September 12, 2026. Task 2-1's timer contracts are planned prerequisites, not existing M1 APIs.

## Initial Starting Point

TaskTimeCard displays completed hours/progress and explicit Start/Log previews. TaskDetailDialog owns the native dialog, draft baselines, tracked operations, requestClose, and task-deletion confirmation. ProjectMenu owns project-deletion confirmation. Existing deadlines.ts rejects nonexistent wall times and requires an offset choice for ambiguous times. TaskDetail contains no timer sessions or time-entry list; retain that separation.

## Goal

Replace the task time previews with real callback-driven timer controls, visible HH:MM:SS elapsed time, explicit manual logging, and completed-entry history while preserving M1 editor and close behavior.

## Exact Scope

Modify only:

- `src/lib/components/tasks/{TaskTimeCard,TaskDetailDialog}.svelte`.
- `src/lib/components/projects/ProjectMenu.svelte`.
- `tests/unit/task-editor.test.ts`.

Create only:

- `src/lib/components/tasks/{LogTimeForm,TimeEntryList}.svelte`.
- `src/lib/domain/{timer-math,time-log}.ts`.
- `tests/unit/{timer-math,time-log,timer-controls,time-log-form}.test.ts`.

Use existing plain CSS tokens, local fonts, Lucide, and Svelte 5 runes. Export the component binding contract described in the paired spec for Task 2-3.

## Non-Goals

No native commands, SQL/migrations, timer store/controller, App.svelte, shared DTO, tokens, dependency, fixture, or clock-source edits. No planner controls, entry editing/deletion, timer notifications, reports, palette commands, or additional tray actions. Preserve unrelated/concurrent changes; do not stage or commit another owner's work.

## Dependencies

Task 2-1 supplies TimerSession/TimeEntry types, persisted transitions, retry-safe accounting, and timer-aware deletion previews. Task 2-3 supplies the authoritative clock, task-specific bindings, request IDs, serialized mutations, uncertain-outcome recovery, state publication, and native mounting. Components compile with optional bindings before integration and clearly retain previews until working callbacks are supplied.

## Step-by-step Work

1. Implement pure elapsed formatting and manual-log normalization using explicit clock/zone arguments and existing deadline conversion.
2. Replace TaskTimeCard's preview state with Start or Pause/Resume/Stop, completed totals, separate active elapsed time, Log time, and entry history when bindings are available.
3. Add the nested manual-log panel with retained drafts, explicit Save time/Cancel, DST choices, calculated end, and safe duplicate/retry behavior.
4. Extend TaskDetailDialog's tracked operations and close handling to include log drafts and unresolved writes without losing notes, children, or attachments.
5. Show explicit unlogged-time-loss warnings in task/project deletion confirmations when active session counts are nonzero.
6. Test real helpers/components, hand bindings to Task 2-3, and verify the mounted native workflow after integration.

## Test Expectations

Cover floor-only formatting, hours beyond 24, pause exclusion, negative clock segments, fractional minutes, invalid/zero manual durations, future ends, overnight/DST conversion, repeated clicks, visible rejection, frozen uncertain submissions, completion-version acknowledgment, and close regression with notes plus a log draft. Mock callbacks establish component behavior, not native persistence.

## Verification Commands

From repository root:

```powershell
pnpm check
pnpm test -- tests/unit/timer-math.test.ts tests/unit/time-log.test.ts tests/unit/timer-controls.test.ts tests/unit/time-log-form.test.ts tests/unit/task-editor.test.ts
pnpm test
pnpm build
```

After Task 2-3 mounts the bindings:

```powershell
. ./scripts/dev-env.ps1
pnpm dev:seed
```

Exercise runtime checks in the paired spec; record them as pending until actually observed.

## Acceptance Criteria

- Real controls appear only with working bindings; elapsed seconds advance from the injected authoritative clock.
- Running, paused, stopped, loading, pending, and error states are distinguishable; task status does not gate or change timer state.
- Completed totals remain unchanged until Stop/Log succeeds; list durations use recorded minutes rather than session span.
- Manual inputs validate explicit timezone/DST rules, preserve attempted values on failure, and cannot duplicate a committed entry during retry.
- Pending/uncertain writes and dirty log drafts participate in the existing close flow; field/notes/attachment safeguards remain intact.
- Task/project deletion warns that running/paused sessions lose unlogged time; stale previews require renewed confirmation.
- The task rail and log panel fit the minimum native window, with keyboard access, visible focus, and no second modal/editor coordinator.

## Review Checklist

- [x] Owned paths and callback contracts agree with Tasks 2-1/2-3.
- [x] No UI interval or optimistic mutation replaces authoritative persistence.
- [x] Known rejection and uncertain transport outcomes have different recovery behavior.
- [x] Completed entry duration, elapsed display, and local wall-time conversion preserve D1–D4.
- [x] Native close/deletion regressions and reference comparison have explicit evidence.
- [x] Task 2-3 owns final phase verification, handoff, and milestone commit.
