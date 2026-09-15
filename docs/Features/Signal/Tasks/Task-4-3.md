# Task 4-3 — Meeting editors, agenda integration and milestone verification

Status: **production implementation delivered; required native/manual acceptance pending.** D1–D8 approved September 13, 2026. See [M4 verification](../../../verification/M4.md) for actual evidence and remaining gates. Integration and startup checks pass; milestone completion/commit remains deferred.

## Source of Truth

[Overview](Phase-4-Overview.md), [approved decisions](../Phase-4-Decisions.md), [paired specification](../ImplementationSpecs/ImplementationSpec-4-3.md), [roadmap](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), and accepted #2d/#3d/#3b in [standalone reference](../../../../_design/design_handoff_signal/Signal.standalone.html). Consume [Task 4-1](Task-4-1.md), [Task 4-2](Task-4-2.md), and paired specifications. [M3 verification](../../../verification/M3.md) is historical evidence, not an M4 pass.

## Initial Starting Point

M3 is accepted and committed at `0ee20b4` on `main`, pushed to origin/main. Protect the pre-existing Phase-4-Decisions.md brief. Today and project Week remain App placeholders. MeetingsStrip, PlannerBlock and task detail show stored meetings with M4 detail notices. Workspace owns the shared mutation queue, clock, task revisions, planner/timer state and recovery guards. NewTaskDialog has an unimplemented Meeting switch. There is no AgendaState or meeting editor. Prerequisite occurrence APIs and Today/Week components remain planned until their task handoffs.

## Goal

Integrate coherent Today/Week/Board/Your Day occurrences, meeting creation/editing/attendance/notes/linking, reliable due-date moves, shared recovery and native close protection. Verify M4 and stop before M5.

## Exact Scope

Create `src/lib/state/agenda.svelte.ts`; `src/lib/components/meetings/NewMeetingDialog.svelte`, `MeetingDetailDialog.svelte`, `MeetingChangePreview.svelte`, `LinkedTaskPicker.svelte`; `tests/unit/agenda-state.test.ts`, `meeting-editor.test.ts`; and `docs/verification/M4.md`.

Own existing `src/App.svelte`, `src/lib/state/app.svelte.ts`, `src/lib/db/client.ts`, `src/lib/views/YourDay.svelte`, `Board.svelte`, `src/lib/components/planner/PlannerCanvas.svelte`, `PlannerBlock.svelte`, `src/lib/components/tasks/NewTaskDialog.svelte`, `TaskDetailDialog.svelte`, `src/lib/components/meetings/MeetingsStrip.svelte`, `src/lib/state/planner.svelte.ts` only for required guard composition, `src/styles/global.css`, `tokens.css`, and affected `tests/unit/workspace.test.ts`, `foundation.test.ts`, `task-editor.test.ts`, `board.test.ts`, `planner-canvas.test.ts`, `your-day.test.ts`. Own README.md, PLAN.md and final status metadata across all M4 documents.

Task 4-1 owns domain/native APIs, schema, occurrence authority and native read/deletion integration. Task 4-2 owns Today/Week and agenda presentation including DueDateDialog. It delivers callback-driven components; this task owns App mounting and mutation orchestration. No dependency/seed-reset change is planned.

## Non-Goals

No M5 summaries, M6 notification delivery, full tray actions, M7 global palette/onboarding or M8 intake. No general activity feed, arbitrary recurrence rules, provider synchronization, subtask deadlines, task creation from Plan links, or timer-accounting changes. Preserve unrelated/concurrent work and edited fixture data; never include it implicitly in a commit.

## Dependencies

Tasks 4-1 and 4-2 must deliver the agreed occurrence, snapshot, durable mutation, due-date and presentation contracts with meaningful tests. Verify actual exports at handoff. Do not add another write queue or independently expand recurrence in components.

## Step-by-step Work

1. Reconcile prerequisite types, authoritative write results, bounded linked-meeting queries and component callbacks.
2. Initialize legacy meeting metadata after baseline seed loading and before business queries; preserve legacy instants and one-time timezone assignment.
3. Implement guarded AgendaState reads, immutable intents, previews, request receipts, recovery and publication before refresh.
4. Integrate Today/Week routing, date navigation, Plan links and durable due-date movement through Workspace.
5. Build new/detail meeting editors, timezone/DST validation, linked-task picker, recurrence change previews, attendance and separate occurrence notes.
6. Replace Board/planner/task-detail meeting stubs with stable-ref navigation; preserve scoped visibility and safe Join links.
7. Compose editor/task/planner/timer/agenda close guards, queued writes, retry paths and focus restoration.
8. Run automated/native/visual/isolation/regression checks, fix defects through file owners and record actual results.
9. Update handoff/status metadata; make the final conventional commit only after the completion gate. Stop before M5.

## Test Expectations

Use deferred reads and actual controllers to exercise changing date/project/ref, write publication before failing refresh, exact uncertain retries, preview conflict/review, deletion invalidation, due-date precision, duplicate clicks, dirty native Close/Quit and composed recovery guards. Verify canceled date editing is not reported committed. Native tests establish recurrence, transaction and migration behavior; actual Tauri runs establish user interaction, persistence and visual acceptance.

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

Quit the seeded instance before ordinary `pnpm tauri dev`. Finish `git diff --check` and `git status --short`; inspect the intended staged set before committing. Kickoff drafting itself does not run feature verification or implement production code.

## Acceptance Criteria

- Today/Week, Board, Your Day and task-linked meeting detail share stable, cancellation-aware occurrences.
- Week moves preserve due precision and unrelated task/timer/block data; absent/DST-invalid deadlines receive explicit review; cancelled edits do not save.
- Plan opens scheduling for the existing task on clock-today with an appropriate scope.
- Meeting defaults, recurrence/end/fold policy, scope previews, visibility, links, attendance and notes match D3–D7.
- Future exceptions and past occurrence history survive series changes; cancellation does not resurrect after refresh/restart.
- One Workspace queue serializes mutations; exact retry and native Close/Quit guards preserve unresolved work.
- Committed publication/invalidation happens before fallback refresh; failed reads never silently show cancelled meetings or repeat writes.
- Native/visual/offline/isolation/regression results and remaining stubs are recorded honestly.
- Completion commit follows the gate; blocked required checks leave M4 partial. M5 remains unimplemented.

## Review Checklist

- [ ] Every meeting consumer uses stable refs and bounded occurrence reads.
- [ ] One queue/clock remains; initialization preserves legacy instants and edits.
- [ ] Due-date movement uses durable receipts and complete deadline precision.
- [ ] Preview/retry/stale-read/guard behavior is verified.
- [ ] Meeting/task navigation protects dirty forms and source attachments.
- [ ] Attendance is explicit; visibility only filters planner display/occupancy/totals.
- [ ] Accepted screens, supported sizes, local assets and offline behavior are checked.
- [ ] Ordinary data and edited fixture records remain separate and preserved.
- [ ] Results distinguish passed, failed, pending and later-phase work.
- [ ] Intended completion commit only; no unauthorized push or M5 implementation.
