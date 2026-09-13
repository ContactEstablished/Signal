# Task 3-1 — Planner domain and native persistence

Status: **complete; M3 manually verified and approved September 13, 2026.** See [M3 verification](../../../verification/M3.md) for actual automated/native results, final user sign-off, and historical automation limitations. D1–D8 remain approved.

## Source of Truth

[Overview](Phase-3-Overview.md), [paired spec](../ImplementationSpecs/ImplementationSpec-3-1.md), [approved D1–D8](../Phase-3-Decisions.md), [PLAN](../../../../PLAN.md), [complete handoff](../../../../_design/design_handoff_signal/README.md), [accepted reference](../../../../_design/design_handoff_signal/Signal.standalone.html) #2a/#2b/#3e, [M2 evidence](../../../verification/M2.md).

## Initial Starting Point

M2 is accepted at `fbd1f58`. Migration 1 already has blocks with local date/minute fields, nullable task, done timestamp and carried source. Migration 3 has live timer sessions, nullable block links and durable timer request receipts. `timers::mutate` currently owns its transaction and Start does not accept a block. Workspace owns the frontend queue. No planner command, repository, lane helper, or planner component exists at kickoff. Existing Temporal frontend helpers and chrono native timestamps are available.

## Goal

Provide checked, transactional planner operations and pure scheduling/display helpers for the approved Your Day behavior, preserving M0–M2 data and accounting.

## Exact Scope

Create `src-tauri/migrations/0004_planner.sql`; `src-tauri/src/planner/{mod,models,tests}.rs`; `src/lib/domain/{planner,lane-packing,planner-rules}.ts`; `src/lib/native/planner.ts`; `tests/unit/{planner-rules,lane-packing,m3-migration}.test.ts`.

Modify `src-tauri/src/db.rs`, `lib.rs`, `timers/mod.rs`, `timers/models.rs`, `timers/tests.rs`, `workspace/delete.rs`, `workspace/tests.rs`; `src/lib/domain/timers.ts`; `src-tauri/Cargo.toml` and Cargo.lock only for native IANA timezone validation. No package.json/pnpm dependency change. Task 3-3 owns frontend timer state threading and shared application state.

## Non-Goals

No M3 view/components/global frontend state edits, fixture reset/new fixture scenarios, second database owner, independent hours totals, recurrence expansion, meeting editing, attendance, reports, notifications, full tray actions or M4 work. Preserve unrelated changes and edited ordinary/fixture databases.

## Dependencies

Accepted M2 and approved D1–D8 only. Hand exact DTO/helper signatures to Tasks 3-2/3-3 before their implementation.

## Step-by-step Work

1. Define query/draft/item/result contracts and source provenance.
2. Add migration 4 without rewriting earlier migrations or existing rows.
3. Implement valid calendar/timezone endpoints, display lanes, task eligibility, next-free placement, batch previews and scoped statistics.
4. Implement coherent read snapshots, revision/fingerprint validation, mutation receipts and repeat-safe source claims.
5. Extract the existing timer transaction body for atomic Stop+complete; support optional block Start while retaining existing task-timer association.
6. Implement block removal detachment and parent-deletion compatibility.
7. Verify failure rollback, replay, source claims, DST and migrations in real temporary SQLite databases; hand off to integration.

## Test Expectations

Meaningful tests cover independent overlap groups, deterministic ties, nested/chained overlaps, painted short intervals, 3/4+ lanes, scheduling across hidden-project occupancy/meetings, boundaries/no-space, carry duration/order/repeat/dismiss, copy provenance/repeat, due/unknown/exhausted estimates, local-day totals, DST gaps/repeated choices, stale revisions/previews, atomic Stop+complete, removal preserving entries/live sessions, and full rollback on late failure. Reopen a disk-backed database to verify durability. Never wipe user data for tests.

## Verification Commands

```powershell
pnpm check
pnpm test
. ./scripts/dev-env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Task 3-3 owns the additional native UI run with `pnpm dev:seed` and ordinary isolation checks.

## Acceptance Criteria

- Date/minute/kind/task invariants and explicit DST offset requirements hold at the native write boundary.
- Every runtime connection comes from the SQL-plugin pool; transactions prepare authoritative replies before commit.
- Old request replay cannot duplicate blocks, entries or source claims, or resurrect removed blocks.
- D2 automatic placement is checked against all projects; D1 painted lane layout never changes stored duration.
- Stop+complete is atomic and uses existing time-entry cache/revision behavior.
- Block removal preserves logged entries and live session identity; parent deletion remains valid.
- Seed data/markers and M2 receipt semantics are preserved.

## Review Checklist

- [x] Ownership and DTOs agree across all three pairs.
- [x] New schema is additive and original fields/data remain intact.
- [x] Calendar, scheduling, concurrency and replay behavior have meaningful tests.
- [x] Unknown and committed outcomes remain distinguishable.
- [x] Helpers are pure and M2 accounting is reused.
- [x] Native/task results are handed to Task 3-3; no premature completion commit.
