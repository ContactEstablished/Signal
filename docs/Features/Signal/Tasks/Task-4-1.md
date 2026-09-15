# Task 4-1 — Agenda, occurrences and native persistence

Status: **production implementation delivered; required native/manual acceptance pending.** D1–D8 approved September 13, 2026. See [M4 verification](../../../verification/M4.md) for actual evidence and remaining gates. Native/calendar implementation and regression tests pass.

## Source of Truth

[Overview](Phase-4-Overview.md), [paired specification](../ImplementationSpecs/ImplementationSpec-4-1.md), [approved decisions](../Phase-4-Decisions.md), [PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), accepted #2d/#3d/#3b in [standalone reference](../../../../_design/design_handoff_signal/Signal.standalone.html), and [M3 verification](../../../verification/M3.md). Proposed APIs below are not baseline capabilities.

## Initial Starting Point

Migration 1 stores raw meetings and meeting-task links; migrations 2–4 support task revisions, timers and planner provenance. There is no recurrence timezone, occurrence/attendance authority, native meeting editor or Today/Week repository. `workspace::tasks::board`, `workspace::models::detail`, and `planner::snapshot` read raw meetings. The existing SQL-plugin pool, transaction/error helpers, chrono/chrono-tz, Temporal, task revision handling and durable planner receipts are available. `deadlineFields` rounds to minute precision and must not be used to preserve existing seconds during Week moves.

## Goal

Implement one bounded occurrence authority and durable meeting/deadline mutations shared by Today, Week, Board, Your Day and task-linked details, preserving M0–M3 records and approved recurrence history.

## Exact Scope

Create `src-tauri/migrations/0005_agenda.sql`; `src-tauri/src/agenda/{mod,models,recurrence,tests}.rs`; `src/lib/domain/{agenda,agenda-calendar}.ts`; `src/lib/native/agenda.ts`; and `tests/unit/{agenda-calendar,m4-migration}.test.ts`.

Modify only registration in `src-tauri/src/{db,lib}.rs`; native consumer/deletion compatibility in `src-tauri/src/workspace/{tasks,models,delete,tests}.rs` and `src-tauri/src/planner/{mod,tests}.rs`; and minimal compatible DTO changes in `src/lib/domain/{types,planner}.ts`. Existing dependencies suffice. Task 4-3 owns bootstrap, shared state and UI consumers.

## Non-Goals

No Today/Week/meeting components, App/Workspace edits, seed reset/new fixture content, UI global styles, new database owner, notification delivery, summaries, imports or M5+ behavior. Never create planner blocks for meetings or infer attendance. Preserve unrelated and concurrent work; no commit/push during kickoff.

## Dependencies

Accepted M3 and approved M4 D1–D8. Publish the exact DTO/action/calendar contracts to Tasks 4-2/4-3 before implementation. Runtime initialization after existing seed loading is Task 4-3's integration responsibility.

## Step-by-step Work

1. Define occurrence identity, immutable version/segment schema, DTOs, calendar helpers and action/result contracts.
2. Register migration 5; implement atomic idempotent runtime-zone initialization preserving every legacy instant and seed marker.
3. Implement seekable range projection, retained exceptions, attendance snapshots, cancellation/cutoff, timezone/DST rules and bounded detail queries.
4. Implement Today/Week/task-linked reads and all-project task search. Replace native Board/planner raw reads with the same authority.
5. Implement change previews, transactional writes, durable retry receipts and the due-only task move action.
6. Extend deletion graphs and compatibility DTOs without breaking timer/history accounting.
7. Test fresh/upgrade/reopen, history preservation, conflicts/replay and failure rollback; hand the contracts and evidence to integration.

## Test Expectations

Meaningful tests cover daily/weekly/end rules, gaps/folds, UTC precision, overnight intersection, series edits retaining past and individually overridden futures, exceptions moved into/out of requested windows, notes/attendance preservation, cancellation non-resurrection, replay after deletion, stale revisions/fingerprints, linked-task overrides, planner-hidden occupancy, task due-date seconds/milliseconds, local week/hour buckets and migration isolation. Use temporary databases/files only; no wiping application data.

## Verification Commands

From repository root during implementation:

```powershell
pnpm check
pnpm test
. ./scripts/dev-env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Task 4-3 owns `pnpm build`, `pnpm dev:seed`, ordinary startup and final native visual/regression verification. No tests or migrations were executed during this planning task.

## Acceptance Criteria

- Original schema fields/instants, edited seed records and all M2/M3 accounting survive upgrade and repeated initialization unchanged.
- Every meeting consumer uses stable occurrence references and the same cancellation/range semantics; raw `TaskDetail.meetings` is never a runtime display fallback.
- Daily/weekly projection preserves series wall time with bounded seeking, explicit fold choice, gap skipping and overnight intersection.
- Series changes preserve historical actual occurrences and edited future exceptions; permanent exclusions/cutoff suppress them on every subsequent query.
- Meeting and due-date writes are transactional, revision checked and replay safe, including unknown-response recovery and post-delete replay.
- Week due moves alter only due_at and ordinary task revision/update metadata; time precision, timers, status and planner blocks remain intact.
- Task/project deletion removes or detaches all new meeting relationships while retaining unrelated records and durable replay tombstones.

## Review Checklist

- [ ] Exact ownership and DTO names agree with Tasks 4-2/4-3.
- [ ] Runtime timezone backfill occurs after seeding and is idempotent.
- [ ] Range queries seek rather than iterate from series origin; exceptions use actual timestamps.
- [ ] Historical snapshots, end-date edits, recurrence changes and cutoff are covered separately.
- [ ] Receipt replay precedes existence/revision checks and cannot recreate removed records.
- [ ] Native helpers are used by tests and all consumers; integration/runtime evidence remains distinct.
- [ ] Handoff is complete; Task 4-3 owns the milestone commit and stop-before-M5 gate.
