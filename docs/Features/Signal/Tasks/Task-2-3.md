# Task 2-3 — Shared timer integration and milestone verification

Status: complete; M2 manually accepted September 13, 2026. D1–D6 remain approved. See [M2 verification](../../../verification/M2.md) for final results.

## Source of Truth

- [Overview](Phase-2-Overview.md), [approved decisions](../Phase-2-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-2-3.md).
- [Roadmap](../../../../PLAN.md), M2 and the phase completion gate.
- [Complete handoff](../../../../_design/design_handoff_signal/README.md) and [accepted reference](../../../../_design/design_handoff_signal/Signal.standalone.html), #1b time rail and #2b timer/header treatment.
- [Task 2-1](Task-2-1.md), [Task 2-2](Task-2-2.md), and their paired specifications.
- [Completed M1 evidence](../../../verification/M1.md). Historical M1 results are not M2 results.

## Initial Starting Point

M1 is implemented and accepted. `Workspace` owns one mutation queue, Board/detail state, revisions, and committed-result publication before refresh. `openFoundation()` loads the selected SQL database, installs baseline fixtures, calls `initialize_workspace`, and computes the Today badge. Fixture time is currently frozen independently in this function and `makeClock()`. `App.svelte` ticks once a minute, renders a hardcoded fixture date/time, and mounts task detail without timer bindings. No shared timer controller or header status component exists.

Task 2-1's native timer API and Task 2-2's timer UI are prerequisite contracts, not existing capabilities at kickoff.

## Goal

Connect persistent timers and manual time logging to the existing workspace, apply one advancing clock throughout the fixture, expose running status in the header, and verify the integrated native milestone. Preserve M1 and stop before M3.

## Exact Scope

Own existing:

- `src/App.svelte`
- `src/lib/state/app.svelte.ts`
- `src/lib/db/client.ts`
- `src/lib/domain/clock.ts`
- `src/lib/seed.ts` — explanatory comment only; preserve baseline records, IDs, and marker
- `src/styles/tokens.css`
- `src/styles/global.css`
- `tests/unit/clock.test.ts`
- `tests/unit/workspace.test.ts`
- `README.md`
- `PLAN.md`

Create:

- `src/lib/state/timers.svelte.ts`
- `src/lib/components/shell/TimerStatus.svelte`
- `tests/unit/timers-state.test.ts`
- `docs/verification/M2.md`

Own final integration, actual-result recording, and the completion commit. Task 2-1 owns native files, DTOs, command wrappers, and migrations. Task 2-2 owns timer detail/log components and presentation/domain helpers. No dependency or manifest change is planned.

As a final metadata-only integration exception, update completion status in `docs/Features/Signal/Tasks/Phase-2-Overview.md`, `Tasks/Task-2-1.md`, `Tasks/Task-2-2.md`, `Tasks/Task-2-3.md`, `ImplementationSpecs/ImplementationSpec-2-1.md`, `ImplementationSpecs/ImplementationSpec-2-2.md`, `ImplementationSpecs/ImplementationSpec-2-3.md`, `Phase-2-Decisions.md`, and `Execution-2-Prompt.md`. These paths share the `docs/Features/Signal/` prefix. Do not overwrite another owner's implementation contract or edits concurrently.

## Non-Goals

No planner, timer-to-block assignment, scheduling, task auto-completion, notifications, full tray timer menu, command-palette results, attendance, reports, or time-entry editing/deletion. Do not rewrite M0 fonts/window geometry or M1 editor/drag/attachment workflows. Do not reset fixture databases or recreate deleted fixture tasks. Preserve unrelated and concurrent work; never include it in the milestone commit implicitly.

## Dependencies

Tasks 2-1 and 2-2 must deliver their agreed typed APIs, tests, and UI callback contract. The integration task may draft state tests against those agreed interfaces while prerequisites are being implemented, but it must verify their actual signatures before wiring the application. It must not create a second mutation queue or write another task's owned files concurrently.

## Step-by-step Work

1. Inspect prerequisite implementations and their results; resolve API discrepancies with their owners.
2. Wire timer initialization after SQL loading and baseline seed loading, before workspace initialization and attachment recovery.
3. Replace frozen frontend clock reads with the native-provided clock offset; retain fixture timezone and injected deterministic test clocks.
4. Add timer state with guarded reads, authoritative result publication, and retained request identity for unresolved writes.
5. Route all timer/log mutations through the existing Workspace queue; integrate returned task totals/revisions with Board and detail. Prevent other writes affecting a task whose prior request outcome is unknown.
6. Bind the Task 2-2 UI, add the header running count, update live fixture labels, and repaint elapsed values every second.
7. Preserve editor/native-close protection, project/task deletion behavior, and M0 tray recovery; refresh timer reads on focus and restoration.
8. Run automated, native persistence, failure, seed isolation, and visual checks. Fix M2 defects through their file owners.
9. Record exact outcomes, launch commands, click-paths, remaining stubs, and limitations. Complete the authorized conventional milestone commit only when the M2 gate passes; stop before M3.

## Test Expectations

Exercise integrated queue ordering across task edits and timer writes, authoritative publication before refresh failure, stable request identity across uncertain retries, stale read suppression, task selection changes, deletion invalidation, advancing fixture clock behavior, and per-second timestamp-derived rendering. Tests must distinguish paused time from running time and completed totals. Verify explicit recovery state and exactly one Log completion notification per committed request identity, including receipt replay.

Use real native temporary-database tests and actual Tauri launches to verify persistence, receipt replay, migration/seed repeatability, and tray behavior. Mocked frontend callbacks alone do not satisfy those requirements. Do not add tests that merely restate constants or claim M1's prior counts as new results.

## Verification Commands

Run from repository root:

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

Quit the running instance before ordinary launch with `pnpm tauri dev`. The seed script expands to `tauri dev -- -- --seed`. A Vite-only page cannot verify native persistence or tray behavior.

Finish with `git diff --check` and `git status --short`; inspect the exact staged set before committing.

## Acceptance Criteria

- Start/Pause/Resume/Stop and manual Log work from task detail, with visibly advancing HH:MM:SS and no duplicate completed entries.
- Different tasks can run concurrently; paused sessions remain visible but do not count as running.
- One Workspace queue serializes M1 and M2 writes; committed replies update sessions, entries, task totals, and revisions before any refresh.
- A failed readback cannot cause a second write. An uncertain response retains the original request ID and payload for explicit retry and blocks other affected-task writes until resolved.
- Every frontend fixture clock consumer agrees with the native offset; fixture time advances across restarts without reinstalling stopped/deleted sessions.
- Header count and native tooltip agree after transitions and deletion; Open/Quit and native editor protection still work in both tray modes.
- Required automated, native, restart, visual, and isolation outcomes are recorded honestly.
- M2 handoff is complete and the conventional feature commit contains only intended changes. Blocked required verification is reported as partial.
- M3 remains unimplemented.

## Review Checklist

- [x] All three ownership sets and typed contracts are integrated.
- [x] No duplicate queue, independent fixture clock, or mutable elapsed counter remains.
- [x] Unknown-outcome retries retain identity; committed readback failures do not resubmit.
- [x] Recovery never disables its only Retry path; confirmed Log completion clears only the submitted form.
- [x] Completed totals remain separate from active elapsed time.
- [x] Seed installation preserves original rows, markers, edits, deletions, and ordinary isolation.
- [x] Native restart, hide/restore, tooltip, and close-guard checks have actual evidence.
- [x] M1 drag, task editing, attachments, fonts, and supported window sizes remain usable.
- [x] Handoff and statuses distinguish implemented, verified, pending, and later-phase work.
- [x] The milestone commit is made only after completion; stop for review before M3.
