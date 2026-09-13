# M2 execution prompt

Execution status: M2 complete and manually accepted September 13, 2026. All three task/spec pairs and the requested refinements are implemented; [M2 verification](../../verification/M2.md) records final evidence. The original authorized execution prompt is retained below. D1–D6 remain approved. M3 kickoff is subsequently authorized.

---

Implement **Signals M2 — Timers & time entries** in `C:\Projects\ContactEstablished\Signal`. Execute all three tasks with their paired implementation specifications, including production code, meaningful tests, integration, and native verification. Do not stop after creating more planning documents. Stop before M3.

Read:

- [PLAN.md](../../../PLAN.md), including fixed stack, M2, persistence, and milestone completion gate.
- [Complete design handoff](../../../_design/design_handoff_signal/README.md) and accepted #1b/#2b portions of [Signal.standalone.html](../../../_design/design_handoff_signal/Signal.standalone.html). Use #2b timer/header styling without implementing its planner.
- [M2 overview](Tasks/Phase-2-Overview.md) and [approved D1–D6](Phase-2-Decisions.md).
- [Task 2-1](Tasks/Task-2-1.md) with [Spec 2-1](ImplementationSpecs/ImplementationSpec-2-1.md).
- [Task 2-2](Tasks/Task-2-2.md) with [Spec 2-2](ImplementationSpecs/ImplementationSpec-2-2.md).
- [Task 2-3](Tasks/Task-2-3.md) with [Spec 2-3](ImplementationSpecs/ImplementationSpec-2-3.md).
- [M1 verification](../../verification/M1.md) for accepted behavior and regression paths.

M1 is complete, manually accepted, and committed/pushed as `b836397`. M2 D1–D6 were explicitly approved without amendments on September 12, 2026. Inspect current branch/status and applicable repository instructions before editing; do not reset the tree to that historical commit. Preserve the existing M2 planning documents, design files, user data, and all unrelated/concurrent changes.

The phase delivers:

1. One persisted live session per task, concurrent tasks, Start/Pause/Resume/Stop, and HH:MM:SS updated visibly each second from timestamps.
2. UTC wall-clock accounting: running includes sleep/shutdown; paused excludes them; negative segments clamp to zero. Preserve millisecond precision/fractional minutes. System clock corrections affect elapsed per D2; no correction-review feature.
3. Atomic, retry-safe Stop and manual Log writes through the existing SQL-plugin pool and completed-entry hours-cache triggers. No double logging after an uncertain IPC result or failed refresh.
4. Manual Log date/start/duration, explicit timezone/DST handling, calculated end/future validation, overlaps allowed, read-only entry history, and existing dirty/native-close protection.
5. Running-count header and native running/paused tooltip; timer-aware task/project deletion confirmation discards unlogged sessions with a clear warning.
6. A shared live September fixture clock anchored once at the approved snapshot, with the two accepted task timers installed once and no modification of ordinary data or original seed records.

Implement in dependency order: native contracts/schema/clock/accounting (2-1), callback-driven task UI/domain helpers (2-2), then shared integration and full verification (2-3). Honor each task's file ownership. All runtime writes use the existing Workspace queue on the frontend and transactions on the backend. A successful write publishes current authoritative state before readback; an uncertain write retries the same retained request ID/payload. Do not substitute a second database connection owner, store-plugin timer persistence, or elapsed counters incremented by ticks.

Retain native Windows titlebar, dimensions, local fonts, dark tokens, Lucide, window geometry, minimal tray Open/Quit, and M1 attachment/drag/editor behavior. Do not add planner scheduling or block timer controls, notifications, meeting attendance, reports, entry editing/deletion, full tray actions, palette results, or M3 implementation. Ordinary release startup must never seed fixtures. Fixture tests must not wipe ordinary or edited development databases.

Run from repository root and record actual outcomes:

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

After quitting the current instance, launch ordinary mode with `pnpm tauri dev`. The seed script is exactly `tauri dev -- -- --seed`. A Vite page or passing unit tests alone does not satisfy native verification.

Verify the complete Task 2-3 matrix: two task timers, advancing seconds, pause/resume, hidden restoration, running/paused Quit/relaunch recovery, exactly-once Stop/Log with correct entries/totals, dirty/unknown-outcome close protection, stale deletion previews, advancing/repeatable seed installation and ordinary isolation. Compare timer/header treatment at 1600×960 and 1200×760, confirm local font/offline operation, and check M1 drag/attachments/window/tray regressions. Inject clock corrections in tests; do not change the user's system clock for testing.

Create `docs/verification/M2.md` with implemented scope, commands, click-paths, fixture reconciliation, test results, native/visual/manual evidence, limitations, and remaining later-phase stubs. Update README, PLAN, and M2 planning statuses to reflect actual progress. Previous M1 sign-off is not M2 acceptance. If a required native check cannot be performed, finish independent work, document the precise limitation, leave that check pending, and report the milestone partial rather than inventing a pass.

Once all required implementation and verification gates are satisfied, inspect/stage only intended M2 work (including these M2 planning documents), and commit:

```text
feat(timers): persist concurrent timers and logged time
```

Report the commit hash and results. Do not push without M2 push authorization. Stop for review before M3. Do not ask for D1–D6 approval again or treat a routine implementation choice as a new product decision.
