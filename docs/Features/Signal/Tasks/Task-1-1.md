# Task 1-1 — Persistence and native operations

Status: complete (September 12, 2026). Implementation and automated checks are complete; the user confirmed all manual acceptance checks and authorized the milestone commit and push. See [M1 execution evidence and user sign-off](../../../verification/M1.md).

## Source of Truth

- [Phase overview](Phase-1-Overview.md) and [approved decisions](../Phase-1-Decisions.md).
- [Implementation specification](../ImplementationSpecs/ImplementationSpec-1-1.md).
- [Roadmap](../../../../PLAN.md), M1 and persistence constraints.
- [Handoff schema](../../../../_design/design_handoff_signal/README.md), State & data model.
- Existing `src-tauri/migrations/0001_initial.sql`, `src-tauri/src/db.rs`, and `src/lib/db/client.ts`.

## Initial Starting Point

M0 has 13 tables, a transactional hours cache, SQL-plugin-owned connections, read-only frontend SQL permissions, debug seed isolation, and narrow settings commands. Project/task mutations, task ordering, blocked-since, attachment operations, and full task read models do not exist. `pool()` is private; `migrations()` registers only version 1.

## Goal

Provide one tested, typed persistence boundary for every M1 mutation and file operation, retaining M0 data and connection ownership.

## Exact Scope

Own these existing files: `src-tauri/src/{db,lib,window,tray}.rs`, `src-tauri/{Cargo.toml,Cargo.lock}`, `package.json`, `pnpm-lock.yaml`, and `src/lib/db/client.ts`. Changes to window/tray are limited to the M1 editor exit guard; preserve their M0 geometry and default-OFF hide/recovery behavior.

Create:

- `src-tauri/migrations/0002_board_fields.sql`.
- `src-tauri/src/workspace/{mod,models,projects,tasks,delete,tests}.rs`.
- `src-tauri/src/attachments/{mod,tests}.rs`, `src-tauri/src/links.rs`, and `src-tauri/src/exit_guard.rs`.
- `src/lib/domain/types.ts`, `src/lib/native/commands.ts`.
- `tests/unit/m1-migration.test.ts`.

These are exact ownership sets, not instructions to create unused scaffolding. Rust submodules must contain their named business behavior. Task 1-1 owns all dependency changes needed by Tasks 1-2/1-3/1-4.

## Non-Goals

Do not edit migration 1, App.svelte, styles, Board/task components, seed records, fixture assets, M0 tests, or roadmap/handoff documents. Do not implement timers, planner scheduling, meeting CRUD/recurrence, notifications, imports, or backup. Preserve all unrelated/concurrent work, including the untracked kickoff documents present at kickoff. Do not stage or commit other owners' files.

## Dependencies

M0 is complete and D1–D8 are approved. No M1 task dependency. Task 1-4 owns the actual synthetic assets and frontend fixture hookup; the native fixture operation must compile without those assets being present.

## Step-by-step Work

1. Preserve migration 1; implement the additive migration and fresh/upgrade tests, including first-seed ordering when migration 2 precedes fixture insertion.
2. Publish the shared DTOs and command signatures from the paired specification before dependent UI work.
3. Implement validated read/project/task operations using the existing plugin pool and real transactions.
4. Implement status transitions, full-column ordering, revision conflicts, and preview-checked deletion.
5. Implement attachment staging, commit, copy removal/recovery, and scoped external opening.
6. Register modules/commands/plugins while preserving native setup, tray/window behavior, and database selection.
7. Add meaningful native tests against temporary SQLite/files and an upgrade test against the existing migration/seed shape.
8. Hand Task 1-4 actual command/test results and the published contracts. Native UI checks remain an integrated requirement.

## Test Expectations

Prove real mutation transactions, rollback, stale-revision rejection, ordering including hidden cards, status timestamps, deletion graph preservation, fixture isolation, file staging/recovery, and path confinement. Exercise domain helpers through the same functions used by commands, not a second test-only implementation. Existing six M0 tests must remain valid.

## Verification Commands

From repository root after implementing this task:

```powershell
pnpm check
pnpm test
. ./scripts/dev-env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

After Task 1-4 integration, launch `pnpm dev:seed` and separately `pnpm tauri dev`; verify actual native command, attachment, and restart behavior. Never run deletion verification against ordinary user records.

## Acceptance Criteria

- Fresh and upgraded databases retain all original fields/records and hours-cache invariants.
- Runtime database selection remains native-owned; no frontend SQL write capability is added.
- All declared commands validate IDs, enums, nullability, times, relationships, and revision/fingerprint expectations.
- Ordering/status/child-record writes are atomic; successful replies contain authoritative state.
- Confirmed deletion removes exactly its approved graph and queues only owned attachment copies for cleanup.
- A failed copy/save/delete or process interruption has a defined recoverable outcome.
- Attachment removal distinguishes committed metadata changes from pending file cleanup; failed multi-file staging publishes no partial batch.
- Seed attachment installation is debug/fixture-only and repeatable on existing M0 fixtures.
- Rust and migration tests have recorded outcomes; no UI persistence claim is made from mocks.

## Review Checklist

- [x] Migration 1 and M0 records are unchanged.
- [x] Native pool, transaction, and timestamp ownership are consistent.
- [x] Revision conflicts and changed deletion previews reject safely.
- [x] Source files, unrelated data, and ordinary/fixture storage remain separate.
- [x] Every command has typed success/error behavior and a consumer or explicit Task 1-4 hookup.
- [x] Dependency/lockfile changes are limited to M1 needs.
- [x] Task 1-4 receives actual evidence and owns the milestone commit.
