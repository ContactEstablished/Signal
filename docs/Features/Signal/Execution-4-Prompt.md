# M4 execution prompt

Execution status (September 13, 2026): all three task/spec pairs implemented. Automated checks and seeded/ordinary native startup pass. The milestone remains partial pending the required native/manual gates in [M4 verification](../../verification/M4.md). No completion commit or push; no M5 work.

Implement Signal M4 — Today, Week and meeting management in `C:\Projects\ContactEstablished\Signal`. D1–D8 were approved as proposed September 13, 2026. When the user invokes this execution prompt, read and execute all three task/spec pairs through production implementation, meaningful tests and native verification. Do not stop after creating or reviewing documents. Stop before M5.

Read [PLAN](../../../PLAN.md), the [complete handoff](../../../_design/design_handoff_signal/README.md), accepted #2d/#3d/#3b in [HTML](../../../_design/design_handoff_signal/Signal.standalone.html), [approved decisions](Phase-4-Decisions.md), [overview](Tasks/Phase-4-Overview.md), [M3 evidence](../../verification/M3.md), and each pair in dependency order:

1. [Task 4-1](Tasks/Task-4-1.md) with [Spec 4-1](ImplementationSpecs/ImplementationSpec-4-1.md).
2. [Task 4-2](Tasks/Task-4-2.md) with [Spec 4-2](ImplementationSpecs/ImplementationSpec-4-2.md).
3. [Task 4-3](Tasks/Task-4-3.md) with [Spec 4-3](ImplementationSpecs/ImplementationSpec-4-3.md).

The accepted M3 baseline is `0ee20b4`, pushed on `main`. Inspect current branch/status and preserve user/concurrent changes. Do not reset databases or the worktree, rewrite migrations 1–4, reinstall fixtures over edited records, or discard running timers. Destructive verification uses temporary databases or explicitly disposable test records.

Keep the fixed stack and offline baseline: Tauri 2, Svelte 5 runes, TypeScript, Vite, pnpm, SQL-plugin SQLite, plain CSS custom properties, local Sora 600/700 and DM Sans 400/500, Lucide and dark mode. Retain one shared clock/Workspace mutation queue. Use UTC instants for meetings/time entries and existing local date/minute planner fields. No provider APIs or new persistence owner.

Deliver the Today digest and seven-day strip, project Week with due-date movement and keyboard alternatives, and complete meeting creation/detail workflows. Implement stable bounded recurring occurrence projection, explicit DST choices, retained exceptions, durable cancellation, per-occurrence attendance/notes, cross-project task links and Show in Your Day. Preserve deadline seconds/milliseconds, task status, timers and planner blocks when moving due dates.

Initialize legacy meeting metadata atomically after seeding without shifting stored instants. Route all meeting read consumers through the same occurrence authority. Use durable receipts for meeting and deadline writes; retain exact unknown requests for Retry, publish confirmed outcomes before refresh, and compose native Close/Quit protection with existing timer/planner/editor guards. Routine implementation choices consistent with approved D1–D8 do not require another approval.

M4 saves reminder preferences but does not deliver notifications. Summaries remain M5, native scheduling/full tray M6, general palette/import/backup/polish M7, and assisted screenshot/text intake M8. Do not add subtask deadlines, calendar synchronization, inferred attendance or a general task activity feed.

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

After quitting the seeded instance, run `pnpm tauri dev` for ordinary startup/isolation. A working Vite server alone is insufficient. Compare the native app with the accepted references at 1600×960 and 1200×760; verify local fonts, persistence/restart, offline behavior, keyboard/drag, Today→Plan, recurrence/DST/exceptions, attendance/notes/links, cancellation non-resurrection, unknown retry/exit protection and prior-milestone regressions.

Record exact commands, outcomes, fixture dates/records, click-paths, visual differences, remaining stubs and pending user checks in `docs/verification/M4.md`. Update README, PLAN and M4 execution statuses accurately. If a prerequisite or required verification is blocked, complete independent work, identify the precise blocker and report partial; never claim unobserved checks passed.

After required gates pass, inspect and stage only intended M4 files and commit `feat(agenda): add Today Week and meeting management`. Report the hash and actual verification results. Do not push without authorization. Stop for the user's review before M5.
