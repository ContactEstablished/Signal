# Task 3-3 — Your Day integration and milestone verification

Status: **complete; M3 manually verified and approved September 13, 2026.** See [M3 verification](../../../verification/M3.md) for actual automated/native results, final user sign-off, and historical automation limitations. D1–D8 remain approved.

## Source of Truth

[Overview](Phase-3-Overview.md), [decisions](../Phase-3-Decisions.md), [paired spec](../ImplementationSpecs/ImplementationSpec-3-3.md), [PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), [accepted HTML](../../../../_design/design_handoff_signal/Signal.standalone.html) #2a/#2b/#3e, [Task 3-1](Task-3-1.md), [Task 3-2](Task-3-2.md), and [M2 evidence](../../../verification/M2.md).

## Initial Starting Point

Baseline main `fbd1f58`; preserve the existing M3 decision brief. M2 has concurrent timers, manual Log, discovery, native close protection and one Workspace queue. Workspace.selectedDate means clock-today. App Your Day is a placeholder; NewTaskDialog onCreate returns TaskDetail then onCreated(plan) only shows an M3 notice. Prerequisite M3 APIs/components are planned until handed off.

## Goal

Deliver the integrated daily planner/sidebar/workflows, preserve M0–M2 and verify M3 without beginning M4.

## Exact Scope

Create `src/lib/views/YourDay.svelte`, `src/lib/state/planner.svelte.ts`, `src/lib/components/planner/PlannerSidebar.svelte`, `PlannerPreview.svelte`, `BlockEditor.svelte`; `tests/unit/planner-state.test.ts`, `your-day.test.ts`; `docs/verification/M3.md`.

Own existing `src/App.svelte`, `src/lib/state/app.svelte.ts`, `src/lib/state/timers.svelte.ts` for optional block Start threading, `src/styles/tokens.css`, `global.css`, `tests/unit/workspace.test.ts`, README.md, PLAN.md. Own final metadata updates in all M3 planning docs as an integration exception. Task 3-1 owns native/domain/wrappers; Task 3-2 owns canvas/picker. No seed/package changes.

## Non-Goals

No recurrence/editing/attendance, Today/Week, summaries, reminders/fulltray, onboarding/import/backup, fixture reset or M2 semantics rewrite. Preserve unrelated/concurrent changes.

## Dependencies

Tasks3-1/3-2 agreed types/helpers/components and meaningful tests. Verify actual exports before wiring. Retain exactly one Workspace mutation queue.

## Step-by-step Work

1. Reconcile prerequisite snapshot/write/accounting and component contracts.
2. Implement PlannerState selected date/scope, guarded reads, pending/error/recovery and retained exact retries.
3. Wire all planner/timer/edit/delete invalidation through Workspace.
4. Replace placeholder with date/scope,canvas,sidebar,totals,leftovers,empty/previews.
5. Wire Quick add, drag-in, carry/Pick/Dismiss, due planning and Monday copy.
6. Add offset-aware keyboard editor, remove/done confirmations and separate optional task-Done offer.
7. Reuse task editor/new-task create-and-plan without duplicating creation.
8. Compose planner draft/pending/unknown native close protection.
9. Verify native/persistence/visual/regression gate, document results and commit only after completion.

## Test Expectations

Deferred read races, exact retry after date change, publication before failed refresh, accounting revisions, guard composition, date/scope/eligibility/totals, stale previews/DST, and create-success/plan-failure distinction. Real native tests and UI runs are required beyond mocked callbacks.

## Verification Commands

```powershell
pnpm check
pnpm test
pnpm build
. ./scripts/dev-env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
pnpm dev:seed
```

Quit before ordinary `pnpm tauri dev`. Finish git diff --check/status; stage intended files only.

## Acceptance Criteria

- Date/scope/sidebar/totals follow D2/D4/D5 without hidden-project collisions.
- Selection/picker/move/resize/edit/carry/Quick add/bulk previews persist and repeat safely.
- Linked timers preserve identity/accounting; Stop+complete atomic, task Done optional.
- Unknown writes retain exact request and Retry; close cannot lose recovery.
- Committed publication precedes refresh; failures never duplicate saved work.
- Native/visual/offline/isolation/regression results recorded honestly.
- Completion commit follows gate; blocked verification is partial; M4 stays unimplemented.

## Review Checklist

- [x] Contracts/ownership match actual implementation.
- [x] One queue/clock and unfiltered occupancy preserved.
- [x] Retry/stale/DST/preview/accounting behavior tested.
- [x] Native guard composes with existing task editor.
- [x] Final user acceptance covers the native gate; agent evidence and exact-size/offline automation limitations are recorded separately.
- [x] Handoff separates verified/pending/later-phase work.
- [x] Commit follows gate and no M4 work begins.
