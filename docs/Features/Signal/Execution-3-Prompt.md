# M3 execution prompt

Implement Signals M3 — Your Day in `C:\Projects\ContactEstablished\Signal`. User approved D1–D8 as written September 13, 2026 and authorized starting implementation. Execute production code, meaningful tests and native integration; do not stop at planning documents. Stop before M4.

Read [PLAN](../../../PLAN.md), [complete handoff](../../../_design/design_handoff_signal/README.md), accepted #2a/#2b/#3e in [HTML](../../../_design/design_handoff_signal/Signal.standalone.html), [decisions](Phase-3-Decisions.md), [overview](Tasks/Phase-3-Overview.md), [M2 verification](../../verification/M2.md), and each pair:

1. [Task 3-1](Tasks/Task-3-1.md) with [Spec 3-1](ImplementationSpecs/ImplementationSpec-3-1.md).
2. [Task 3-2](Tasks/Task-3-2.md) with [Spec 3-2](ImplementationSpecs/ImplementationSpec-3-2.md).
3. [Task 3-3](Tasks/Task-3-3.md) with [Spec 3-3](ImplementationSpecs/ImplementationSpec-3-3.md).

Baseline M2 is `fbd1f58`; inspect current branch/status and preserve all user/concurrent changes. Do not reset the tree, rewrite earlier migrations/seed rows, or treat an existing accepted timer as disposable. Use temporary databases for destructive tests.

Follow the fixed Tauri2/Svelte5 runes/TypeScript/Vite/pnpm stack, plain CSS tokens/local fonts/Lucide, SQL-plugin pool, UTC instants and local block date/minutes. Extend M2 timer association without replacing its accounting/receipt logic. Retain one Workspace mutation queue, publish committed replies before readback, retry unknown writes with the same request payload/identity, and protect native Close/Quit and other writes while recovery is unresolved.

Deliver the 24h planner and scoped sidebar,15-minute pointer/keyboard selection/move/resize, painted-extent lane packing, task/break/lunch/focus picker, local-time validation, read-only meetings, timer/run-over/completion controls, block removal preserving time, leftovers and repeat-safe bulk previews per D1–D8. Decisions are approved; routine implementation choices do not require another approval. Recurrence expansion/editing/attendance remain M4, summaries M5, notifications/full tray M6.

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

After quitting the current instance, verify ordinary startup with `pnpm tauri dev`. Record actual native persistence/restart, seed isolation, layout sizes, keyboard/drag, timer accounting, preview/no-space/DST/carry/retry and prior milestone regression outcomes in docs/verification/M3.md. Update README, PLAN and M3 statuses honestly. If a required check cannot run, finish independent work, identify the exact limitation and report partial; never invent a pass.

Once all gates pass, inspect/stage only intended M3 files and commit `feat(planner): add daily scheduling overlap lanes and carry-over`. Report hash/results. No push without authorization. Stop for review before M4.
