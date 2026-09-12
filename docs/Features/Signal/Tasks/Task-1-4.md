# Task 1-4 — Shell integration and milestone verification

Status: complete (September 12, 2026). Implementation and automated checks are complete; the user confirmed all manual acceptance checks and authorized the milestone commit and push. See [M1 execution evidence and user sign-off](../../../verification/M1.md).

## Source of Truth

- [Overview](Phase-1-Overview.md), [approved decisions](../Phase-1-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-1-4.md).
- [Roadmap](../../../../PLAN.md), M1 and required completion gate; [M0 verification](../../../verification/M0.md).
- [Handoff](../../../../_design/design_handoff_signal/README.md) and [accepted reference](../../../../_design/design_handoff_signal/Signal.standalone.html), #1b/#3a.
- Paired specifications [1-1](../ImplementationSpecs/ImplementationSpec-1-1.md), [1-2](../ImplementationSpecs/ImplementationSpec-1-2.md), [1-3](../ImplementationSpecs/ImplementationSpec-1-3.md).

## Initial Starting Point

M0 App.svelte owns shell state and previews. It has no shared workspace controller, Board mounting, task-dialog routing, or file-drop subscription. Its successful native/font/seed/tray checks are recorded in M0.md, not new M1 evidence. Tasks 1-1–1-3 are prerequisite deliverables, not implemented capabilities at kickoff.

## Goal

Connect the three M1 work groups into the native shell and prove the complete project/Board/task/attachment workflows while preserving M0 and stopping before M2.

## Exact Scope

Own existing `src/App.svelte`, `src/styles/{tokens,global}.css`, `src/lib/seed.ts`, `src-tauri/tauri.conf.json`, and `README.md`.

Create `src/lib/state/app.svelte.ts`, `src/lib/domain/{clock,mutations}.ts`, `src/lib/native/attachments.ts`, `src/lib/seed-attachments.ts`, `src/lib/fixtures/{gateway-arch.pdf,latency-p95.png,README.md}`, `tests/unit/{clock,mutations}.test.ts`, and `docs/verification/M1.md`.

Own final shared-file integration and the milestone commit. Other tasks retain their file ownership; request corrections from the relevant owner rather than writing concurrently into their files. Update PLAN.md's M1 status only during a separately authorized M1 execution/completion, not during this kickoff.

## Non-Goals

No M2 timers/manual Log, M3 scheduling, M4 Week/meeting editing, summaries, notifications, imports, or palette results. No rewrite of M0 fonts/window/tray code, no fixture counter padding, and no deletion of ordinary user data for testing. Preserve unrelated/concurrent changes and existing untracked kickoff work; do not sweep them into a feature commit without authorization.

## Dependencies

Tasks 1-1–1-3 must expose their agreed types, callbacks, and tested behavior. M0 prerequisites/launch commands remain applicable. Task 1-1 owns dependency changes and native command registration. Task 1-4 is the sole integrator for App.svelte/shared state/global styles/fixture hookup.

## Step-by-step Work

1. Check prerequisite signatures, source changes, unit evidence, and remaining defects.
2. Add a shared fixture-aware clock, serialized mutation controller, and query-generation guards.
3. Replace only M1 previews in App.svelte; mount Board/project/task dialogs and route callbacks.
4. Connect OS drop events to the active task editor, preserving internal Board drag.
5. Create valid synthetic attachment fixtures and idempotent debug-only installation on existing seeded databases.
6. Integrate focus/keyboard/close behavior, shared form styles, load errors, and badge/project refresh.
7. Run required commands and native fresh/upgrade/restart/visual/failure checks; fix M1 defects with their owners.
8. Record actual results, residual stubs, exact commands and click-paths; make the authorized conventional M1 commit and stop before M2.

## Test Expectations

Test queue serialization, stale response suppression, committed-write/readback-failure distinction, fixed/real clock boundaries, and create completion idempotence through the real helpers. Do not replace persistence/native checks with mocked callbacks. Verify actual native attachment open/drop and destructive graph behavior only on disposable records/files.

## Verification Commands

From repository root:

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

Quit before switching to `pnpm tauri dev`. A local Vite page alone never satisfies native verification. Finish with `git diff --check` and inspect `git status --short` before selecting intended commit files.

## Acceptance Criteria

- M1 entry points replace their previews; every later-feature action remains explicit and honest.
- Task/project changes refresh the Board, open details, header projects, and Today badge without stale reads or duplicate writes.
- Keyboard/focus/native file drop work alongside Board pointer drag and M0 Ctrl+K/tray behavior.
- A single active-editor controller guards project/task drafts, uses mode-specific close choices, and keeps project sub-bar navigation available on previews.
- Existing fixtures upgrade repeatably, synthetic files are labeled, and ordinary data/files stay separate.
- Required commands and native/visual/restart checks have actual recorded outcomes; no unverified pass is asserted.
- M1.md inventories results/stubs/commands/click-paths and discrepancies, and the milestone is conventionally committed only when complete.
- Stop before M2; partial verification means a partial milestone with a precise blocker.

## Review Checklist

- [x] All four task ownership sets and callbacks are integrated without duplicate stores.
- [x] Required UI, persistence, deletion, file, error and accessibility paths were exercised.
- [x] M0 native dimensions/fonts/tray/seed isolation remain intact.
- [x] No future workflow is advertised as implemented.
- [x] Git staging contains only authorized M1 files.
- [x] Handoff states actual command/test results and the review boundary.
