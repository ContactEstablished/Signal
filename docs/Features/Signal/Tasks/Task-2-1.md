# Task 2-1 — Durable timers and native time accounting

Status: complete; M2 manually accepted September 13, 2026. D1–D6 remain approved. See [M2 verification](../../../verification/M2.md) for final results.

## Source of Truth

- [Phase overview](Phase-2-Overview.md), [approved decisions](../Phase-2-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-2-1.md).
- [Roadmap](../../../../PLAN.md), M2 and persistence/completion constraints; [handoff](../../../../_design/design_handoff_signal/README.md), concurrent timers and original table fields.
- [Completed M1 verification](../../../verification/M1.md); migrations 1/2, native workspace commands, file lifecycle, and editor exit guard are existing production prerequisites.

## Initial Starting Point

The SQL plugin owns database connections. `db::migrations()` registers versions 1 and 2; `db::pool()` exposes the native-selected pool. Existing time-entry triggers maintain `hours_worked`. Workspace and attachment command adapters use the frozen fixture helper in `workspace/models.rs`. Tray setup exposes Open/Quit and a fixed tooltip. No timer state, receipt ledger, advancing clock service, or timer command exists.

## Goal

Provide one transactional, typed native timer/accounting boundary that supports concurrent tasks, paused sessions, accurate declared clock behavior, replay-safe completed entries, M1 deletion compatibility, and isolated live fixture initialization.

## Exact Scope

Create:

- `src-tauri/migrations/0003_timers.sql`
- `src-tauri/src/clock.rs`
- `src-tauri/src/timers/mod.rs`
- `src-tauri/src/timers/models.rs`
- `src-tauri/src/timers/tests.rs`
- `src/lib/domain/timers.ts`
- `src/lib/native/timers.ts`
- `tests/unit/m2-migration.test.ts`

Modify:

- `src-tauri/src/db.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/tray.rs`
- `src-tauri/src/workspace/mod.rs`
- `src-tauri/src/workspace/models.rs`
- `src-tauri/src/workspace/delete.rs`
- `src-tauri/src/workspace/tests.rs`
- `src-tauri/src/attachments/mod.rs`
- `src-tauri/src/attachments/tests.rs`

The attachment changes are limited to the shared runtime clock and compatible tests. Keep the fixed stack and installed dependencies; no manifest, broad permission, window-store, or original migration changes are required.

## Non-Goals

No Svelte UI/controller changes, frontend clock implementation, planner controls, entry editing/deletion UI, notifications, full tray actions, palette results, or automatic task status changes. Do not reset seed data or add ordinary fixture rows. Preserve unrelated/concurrent changes and existing kickoff documents; final milestone staging/commit belongs to Task 2-3.

## Dependencies

None beyond accepted M1 and approved M2 D1–D6. Task 2-3 invokes native initialization after SQL load and original seed load, before native attachment recovery; command implementations must compile and be independently testable before that hookup exists.

## Step-by-step Work

1. Implement the additive schema, native DTOs, validation, and typed frontend wrappers prescribed in the paired spec.
2. Add a shared runtime clock, including atomic fixture anchor/session installation and repeatable skip reporting.
3. Implement serialized transactional Start/Pause/Resume/Stop and manual Log, with durable request receipts and authoritative replies.
4. Add timer/entry reads and update native tooltip from committed live session counts.
5. Extend deletion graph/fingerprints and cleanup ordering for sessions/receipts without changing unrelated M1 behavior.
6. Route native workspace/attachment mutation clocks through the shared clock; preserve pure injected clocks in tests.
7. Run native, migration, and compatibility checks; hand exact APIs/results to Tasks 2-2/2-3.

## Test Expectations

Use real SQLite transactions and all three migrations in temporary native databases. Cover concurrent tasks, duplicate requests, stale revisions, stopped-session replay after a new Start, request-payload conflicts, rollback before reply/commit, active/paused restart arithmetic, fractional/zero durations, clock corrections, deletion races, and seed marker/anchor rollback and preservation. Retain M0/M1 migration-only tests and add M2 coverage separately.

## Verification Commands

From repository root:

```powershell
pnpm check
pnpm test -- tests/unit/m2-migration.test.ts
. ./scripts/dev-env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

After integration, `pnpm dev:seed` and separately `pnpm tauri dev` exercise the native workflow described in Task 2-3. Native test results do not establish rendered UI or actual OS sleep behavior.

## Acceptance Criteria

- At most one running/paused session per task is enforced by SQLite, independently of UI disabling.
- Stop appends one entry and finalizes its session atomically; manual Log and request replay cannot double-account.
- Session identity and revision prevent a stale action from affecting a replacement session.
- Pause excludes its interval; D2 UTC wall-clock behavior includes running shutdown/sleep time and prevents negative durations.
- Original entry fields, aggregate/cache triggers, task revisions, and block/task constraints remain intact.
- M2 seed anchor/sessions/marker commit together, preserve edits/deletions, and never install in ordinary/release mode.
- Deletion previews include live session loss; real transitions invalidate confirmation, repaint alone does not.
- Native tooltip remains correct while hidden without depending on a frontend timer tick.
- Errors before commit reject; optional presentation failures after commit never imply rollback.

## Review Checklist

- [x] Every production connection comes from the SQL-plugin pool.
- [x] Original migrations, rows, and hours-cache authority are preserved.
- [x] Replay checks precede state/revision validation without accepting changed payloads.
- [x] Old requests cannot resurrect deleted entities or stop replacement sessions.
- [x] All native mutation clock consumers share the initialized runtime offset.
- [x] Fixture skip/marker, task/project deletion, and rollback tests use real operations.
- [x] Actual test evidence and bindings are handed to Task 2-3; no premature completion commit.
