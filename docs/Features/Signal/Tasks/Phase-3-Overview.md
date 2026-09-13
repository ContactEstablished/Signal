# M3 kickoff — Your Day

D1–D8 approved as written September 13, 2026. Baseline: accepted M2 `fbd1f58`, branch `main`. At kickoff only the pre-existing untracked Phase-3-Decisions.md was present; it is preserved and date-locked. This kickoff adds planning documents only; production execution follows validation.

Execution update: **M3 complete and manually accepted September 13, 2026.** The user authorized commit/push and M4 kickoff. See [verification](../../../verification/M3.md); baseline descriptions below are historical kickoff facts.

## Source of truth

[PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), [accepted HTML](../../../../_design/design_handoff_signal/Signal.standalone.html) #2a/#2b/#3e, [approved decisions](../Phase-3-Decisions.md), [M2 verification](../../../verification/M2.md). User authorization includes kickoff and beginning M3; do not ask again for these decisions.

## Goal and boundary

Build the local Your Day planner: a 24-hour scroll grid, date/project scope, selection/picker, move/resize, overlapping lanes, task/neutral blocks, existing-instance meetings, associated timers, completion, sidebar, carry-over and repeat-safe planning previews. Retain SQL-plugin persistence, one Workspace queue, live fixture offset, local fonts/dark tokens/Lucide and native close protection. No M4 recurrence/editing/attendance, M5 summaries, M6 reminders/full tray or M7 palette/import work.

## Verified baseline

M2 has 52 frontend/18 Rust passing tests, user acceptance, native window and fixture isolation; those are historical prerequisite results, not M3 tests. blocks exist in migration 1; nullable timer/entry block_id and durable timers in migration 3. App renders a Your Day placeholder. Workspace owns queue/refresh/editor and TimerState supplies live sessions. No planner implementation exists at kickoff. Exact new APIs in specs are contracts until implemented.

## Executable pairs

| Pair | Responsibility | Dependencies |
| --- | --- | --- |
| [Task 3-1](Task-3-1.md) / [Spec 3-1](../ImplementationSpecs/ImplementationSpec-3-1.md) | Domain/calendar/lane rules, migration/native snapshots and durable operations, timer and deletion compatibility | M2 + D1–D8 |
| [Task 3-2](Task-3-2.md) / [Spec 3-2](../ImplementationSpecs/ImplementationSpec-3-2.md) | Canvas, blocks, task picker and pointer/keyboard presentation | Task 3-1 contracts |
| [Task 3-3](Task-3-3.md) / [Spec 3-3](../ImplementationSpecs/ImplementationSpec-3-3.md) | Sidebar, date/scope, workflows, one-queue integration, close guards, native gate and completion commit | Task 3-1/2 |

Task 3-1 owns shared DTOs and native modules. Task 3-2 owns callback-only components. Task 3-3 owns existing App/Workspace/TimerState, YourDay and shared tokens. Metadata updates to these documents are a Task 3-3 integration exception. No overlapping production ownership.

## Main risks and gate

Painted 26px cards must not overwrite logical 15-minute durations. Hidden-project occupancy must constrain automatic scheduling. Local endpoints need gap/fold validation on both sides. Stop+complete requires one native transaction, and unknown results must retain identity without disabling Retry. Carry/copy provenance survives result removal to prevent accidental recreation. Read-only stored meetings must not imply recurrence support.

Run `pnpm check`, `pnpm test`, `pnpm build`; source `scripts/dev-env.ps1`, then cargo test/check/fmt using src-tauri/Cargo.toml. Launch `pnpm dev:seed`, then ordinary `pnpm tauri dev` after quitting. Compare native 1200×760/1600×960, persisted edits/restart/isolated seed, overlap/selection/resize, task timers, no-space/carry/repeat, DST, unknown/draft close behavior, offline and prior Board/attachment/tray regressions. Record actual outcomes in docs/verification/M3.md. Never substitute a Vite-only page for native verification.

Execute [Execution-3-Prompt](../Execution-3-Prompt.md) in dependency order. Final milestone commit after the gate: `feat(planner): add daily scheduling overlap lanes and carry-over`. No push without authorization. Stop before M4.
