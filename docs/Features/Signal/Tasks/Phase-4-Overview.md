# M4 kickoff — Today, Week and meeting management

Status: **implemented; milestone partial pending native/manual acceptance.** D1–D8 approved as proposed by the user and date-locked September 13, 2026. Execution implemented all three task/spec pairs. The unattended hardening audit is complete for review (147 frontend tests and 49 Rust tests pass). See [M4 evidence and remaining gates](../../../verification/M4.md). The user authorized committing and pushing the current development work on September 15, 2026; manual acceptance remains pending and M5 has not started.

## Source of truth and baseline

[PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), accepted #2d/#3d/#3b in [standalone HTML](../../../../_design/design_handoff_signal/Signal.standalone.html), [approved decisions](../Phase-4-Decisions.md), [M3 overview](Phase-3-Overview.md), and [M3 verification](../../../verification/M3.md).

M3 is manually accepted at `0ee20b4` on `main`, pushed to `origin/main`. At kickoff the only pre-existing change was the untracked Phase-4-Decisions.md; its approved content is preserved and date-locked. Historical M3 checks are prerequisite evidence, not M4 passes. No M4 code or database changes have run.

## Goal and boundary

Deliver a cross-project Today digest, project Week due-date planning, and one-off/recurring meeting creation, editing, notes, linked tasks, attendance, cancellation and planner visibility. All meeting consumers must share one bounded native occurrence authority.

Retain Tauri 2, Svelte 5 runes, TypeScript, Vite, pnpm, the SQL-plugin pool, plain CSS tokens, bundled local fonts, Lucide, one Workspace clock and mutation queue. Business records stay in SQLite; the store plugin remains window-state-only. Preserve M0–M3 tasks, time accounting, planner blocks, attachments, seed edits and ordinary-data isolation.

Do not implement M5 summaries, M6 reminder delivery or full tray scheduling, M7 general palette/polish/import/backup, or M8 screenshot/text intake. Save meeting reminder preferences without claiming reminders fire. Do not add subtask deadlines or a general task activity feed.

## Executable pairs and ownership

| Pair | Work | Dependency |
| --- | --- | --- |
| [Task 4-1](Task-4-1.md) / [Spec 4-1](../ImplementationSpecs/ImplementationSpec-4-1.md) | Additive meeting schema, stable occurrence projection, calendar queries, durable writes, native read/deletion compatibility and domain tests | Accepted M3 and approved D1–D8 |
| [Task 4-2](Task-4-2.md) / [Spec 4-2](../ImplementationSpecs/ImplementationSpec-4-2.md) | Callback-driven Today/Week, cards/pills/strip, due-date editor, drag and keyboard presentation | Task 4-1 DTO/helper contract |
| [Task 4-3](Task-4-3.md) / [Spec 4-3](../ImplementationSpecs/ImplementationSpec-4-3.md) | Meeting editors, shared state/queue/close protection, all view integration, native verification and milestone handoff | Tasks 4-1 and 4-2 |

Task 4-1 owns shared domain/native files and existing native consumers. Task 4-2 owns only its new presentation files. Task 4-3 owns App/Workspace, bootstrap, existing UI consumers, shared styles and final metadata. The task/spec file lists are authoritative; any prerequisite correction remains with its owner, or transfers explicitly to the integrator after handoff. Never silently edit another active owner's files.

## Integration contracts and risks

- Migration 5 is additive. SQL migration cannot discover the workspace timezone: idempotent native initialization must backfill legacy meeting metadata after seed loading and before agenda-dependent reads, preserving exact UTC starts. Ordinary and fixture databases initialize separately.
- Stable occurrence references are independent of their displayed start date. Bounded projection merges generated slots with retained overrides by actual occurrence time. Edit This and following preserves the past and individual future overrides; cancellation cutoffs/exclusions suppress regeneration. No future planner-block copies or unlimited materialization.
- Board, Today, Week, task-linked meeting pages and Your Day use the same projection and cancellation authority. Show in Your Day filters only planner display, occupancy and totals.
- Today follows the shared clock, including the advancing fixture offset. Week selection is independently pinned. Monday–Sunday boundaries and completed-entry local start dates govern calendar buckets and weekly logged hours.
- Due moves preserve time through milliseconds, including explicit DST resolution. A durable agenda request performs only the revisioned deadline change; planner blocks, task status, entries and timers remain unchanged.
- Meeting/due writes use the existing Workspace queue with retained immutable request identities. Publish confirmed changes before refresh; an unknown outcome keeps Retry available and prevents conflicting writes or unsafe native exit. A failed read after a confirmed save must not invite duplicate creation.
- M5 can consume explicitly attended occurrences by actual start date. M6 can consume stable occurrence identities and saved reminder preferences; neither milestone is implemented here.

Exact proposed types, schema and algorithms in the paired specs are implementation contracts, not claims about the existing M3 code.

## Execution and verification gate

Invoke [Execution-4-Prompt](../Execution-4-Prompt.md) to begin implementation in dependency order. The current request authorizes kickoff; it does not itself execute these tasks. D1–D8 need no further approval.

Required implementation checks: `pnpm check`, `pnpm test`, `pnpm build`; source `scripts/dev-env.ps1`, then native cargo test/check/fmt with `src-tauri/Cargo.toml`. Launch `pnpm dev:seed`, quit, then `pnpm tauri dev`. Use disposable test records and temporary databases; never reset the user's edited fixture to match a screenshot.

Native acceptance must cover Today → Plan → Week → move deadline → task detail; meeting creation/linking/Join; one-off/daily/weekly and DST; occurrence/series edits and cancellations; notes/attendance; hidden planner occupancy; cross-project links/deletion; retry, restart, seed isolation, both window sizes, local fonts/offline behavior and prior timer/planner/attachment/tray regressions. Record actual evidence and precise pending checks in docs/verification/M4.md.

Task 4-3 owns the final milestone commit after required gates: `feat(agenda): add Today Week and meeting management`. No push without authorization. Stop for user review before M5.

## Kickoff validation

The three pairs, source links, ownership boundaries, existing integration points and repository script names were reviewed. This is documentation validation only; no M4 unit, build, migration or native-runtime pass is asserted. Remaining complexity is recurrence/history and cross-view recovery, covered by required tests rather than an unresolved product decision.
