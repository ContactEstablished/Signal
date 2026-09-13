# M2 kickoff — Timers & time entries

Prepared September 12, 2026. Final status September 13: **M2 complete and manually accepted; all three pairs and refinements implemented. See [M2 verification](../../../verification/M2.md). M3 kickoff subsequently authorized; no M3 production code yet.**

## Source of truth and prerequisite

- [PLAN.md](../../../../PLAN.md) remains the sole roadmap. M2 follows M1 and stops before M3.
- [Complete handoff](../../../../_design/design_handoff_signal/README.md): task time card, concurrent timers, per-second ticks, header status, data model, and downstream block/tray interactions.
- [Accepted standalone HTML](../../../../_design/design_handoff_signal/Signal.standalone.html): #1b time card and #2b running pill/header treatments. Embedded accepted timer content was inspected as source during this kickoff; no new rendered comparison is claimed.
- [M1 overview](Phase-1-Overview.md), all four Task-1 documents and paired ImplementationSpec-1 documents, [approved M1 decisions](../Phase-1-Decisions.md), and [completed M1 verification](../../../verification/M1.md).
- [Approved M2 D1–D6](../Phase-2-Decisions.md): date-locked user approval, September 12, 2026.
- [Phase execution prompt](../Execution-2-Prompt.md): self-contained instructions to execute all three pairs, verify M2, and stop before M3.

The user completed M1 manual acceptance and authorized its commit/push. Live grounding found branch `main`, HEAD `b836397` (`feat(board): add projects tasks and task detail workflows`), and a clean working tree during initial grounding. At the approved drafting turn, the existing untracked Phase-2-Decisions.md and this overview were present; both are preserved and updated within the requested kickoff. The earlier M1 push succeeded. Preserve that commit, the supplied design files, and all subsequent user/concurrent changes. The original kickoff changed planning documents only; it does not stage, commit, push, run migrations, or implement timers.

## Goal and boundary

Deliver durable, concurrent task timers with Start/Pause/Resume/Stop, visibly advancing HH:MM:SS, atomic completed entries, manual Log, record-derived totals, a header running count, and a native tray tooltip. Restore active and paused sessions accurately according to the resolved clock policy. Preserve M1 task editing, attachment handling, drag previews, deletion confirmation, window geometry, and native close protection.

Planner block controls/association are M3; full tray timer actions are M6; palette results are M7. M2 must not implement scheduling, automatic task completion, notifications, attendance, reports, time-entry editing/deletion, or broader Settings workflows. The existing `time_entries.block_id` and ownership constraint must remain compatible with M3 without exposing a block picker now.

## Verified starting point

| Live file / symbol | Verified fact | M2 implication |
| --- | --- | --- |
| `src-tauri/migrations/0001_initial.sql` | `time_entries` has opaque IDs, task/block references, start/end instants, REAL minutes, ordering/nonnegative checks, and block/task consistency triggers. `task_time_totals` and entry triggers maintain cached task hours. | Preserve this migration and every field. Use its accounting boundary; paused spans mean active minutes need not equal the full wall interval. |
| `src-tauri/migrations/0002_board_fields.sql` | M1 adds task order, blocked-since, revision, and internal `attachment_file_ops`. | New timer persistence requires a versioned additive migration; do not modify migrations 1 or 2 or reinterpret the existing 14-table M1 test. |
| `src-tauri/src/db.rs::migrations`, `pool`, `RuntimeState` | Only migration versions 1 and 2 are registered. `pool` clones the native-selected SQL-plugin pool. Runtime owns seed/ordinary selection. | Reuse the connection owner and runtime selection; no frontend-selected database and no store-plugin business state. |
| `src-tauri/src/lib.rs::run` | Registers M1 commands and services; no timer module/service is registered. SQL load is initiated through `openFoundation`. | Timer initialization must occur only once storage is ready, then load persisted state even without a task dialog. |
| `src-tauri/src/tray.rs::setup` | Tray `signal-tray` has Open/Quit, fixed `Signal` tooltip, and M1 Quit interception. | Add a state-derived native tooltip while retaining the existing menu and guard. |
| `src-tauri/src/workspace/delete.rs::graph`, `preview`, `remove` | Explicit graph fingerprint and transactional deletion cover tasks/children/blocks/entries/files; no timers exist. | New owned records must participate in preview, stale confirmation detection, and atomic deletion under D5. |
| `src/lib/components/tasks/TaskTimeCard.svelte` | Receives `hours`, `estimate`, and `onPreview`; Start/Log are clearly labeled stubs. | Replace these stubs with real controls and explicit pending/error states; preserve derived completed totals. |
| `src/lib/components/tasks/TaskDetailDialog.svelte` | Hosts TaskTimeCard and exports `requestClose`; task fields/notes/deletion already coordinate saves and draft protection. | Compose timer and manual-log actions with that close contract, without creating a second unguarded editor. |
| `src/lib/state/app.svelte.ts::Workspace` | Owns authoritative Board/detail state, one mutation queue, revisions, committed-reply publication, refresh-failure notices, and selection guards. | Integrate accounting replies without replaying a completed Stop/Log on a failed refresh. Avoid competing business stores. |
| `src/lib/native/commands.ts`, `src/lib/domain/types.ts` | No timer API/DTO or completed-entry list exists. TaskDetail has task, project, subtasks, tags, attachments, alerts, meetings. | Introduce explicit typed timer/time-entry contracts; retain compatibility of existing M1 consumers. |
| `src/lib/domain/clock.ts::makeClock`, `src-tauri/src/workspace/models.rs::now`, `src/lib/db/client.ts::openFoundation` | Seed time is frozen independently in frontend clock, native mutation clock, and foundation badge reads. `App.svelte` calls `workspace.tick()` every 60 seconds. | A one-second repaint alone cannot fix seeded elapsed time. D6 requires coordinated clock changes, rather than mixing real 2026 timer instants with frozen 2025 UI dates. |
| `src/lib/seed.ts` | Four baseline projects, 16 tasks, seven completed time entries, and no active sessions. Stable tasks include ATL-477 and Parse config flags. | Preserve the existing marker and all user edits; install M2 additions separately and atomically. |
| `src/styles/tokens.css` | Existing `--timer-fill` uses lime tint; local Sora 600/700 and DM Sans 400/500 remain bundled. | Reuse accepted style values and font weights, adding semantic variables only if required. |

## Three executable task/spec pairs

Read each task together with its deeper implementation specification. Task 2-1 defines native contracts, Task 2-2 consumes them for the UI, and Task 2-3 integrates all shared state and owns the milestone completion gate.

| Pair | Responsibility | Dependencies and ownership |
| --- | --- | --- |
| [Task 2-1](Task-2-1.md) / [Spec 2-1](../ImplementationSpecs/ImplementationSpec-2-1.md) | Native timer state machine, additive schema, exact-once time accounting, read models, deletion compatibility, native clock and tray status. | None beyond accepted M1 and approved D1–D6. Own new Rust timer/clock modules and migration; native registration, database, tray, workspace deletion/model changes; timer DTOs/command wrappers; native and migration tests. |
| [Task 2-2](Task-2-2.md) / [Spec 2-2](../ImplementationSpecs/ImplementationSpec-2-2.md) | Task timer controls, HH:MM:SS math/presentation, manual Log draft and read-only entry list. | Task 2-1's resolved contract. Own TaskTimeCard/TaskDetailDialog, new timer/log components and pure timer/log helpers, associated component/domain tests. |
| [Task 2-3](Task-2-3.md) / [Spec 2-3](../ImplementationSpecs/ImplementationSpec-2-3.md) | Shared controller/header integration, synchronized fixture clock and repeatable timer seeds, native verification and handoff. | Tasks 2-1 and 2-2. Own App.svelte, workspace/timer state, frontend foundation/clock/seed hookup, shell status component, shared tokens if needed, integration tests, README/PLAN/M2 verification and final milestone commit. |

Each implementation path has one owner in the task Exact Scope lists. In particular, Task 2-3 owns frontend integration and shared CSS; Task 2-1 owns native registration and commands. Task 2-2 supplies callbacks rather than editing the controller independently. The specifications agree on TimerSession/TimeEntry DTOs, request identity and replay, current authoritative replies, TimerUiBindings, and publication before refresh. No production API is claimed implemented by those contracts.

## Risks addressed by the implementation specifications

- Stop can commit before an IPC reply is lost. Durable session/request identity must prevent a retried Stop or manual Log from creating a second entry or affecting a newly started session.
- A per-second renderer must derive elapsed from a clock and persisted anchors, not increment an in-memory counter. Hidden WebView throttling must not control accounting or native tooltip correctness.
- Paused session span differs from active duration. Reads, entry presentation, aggregates, and future summary consumers must preserve this distinction.
- A stale deletion preview must detect changed persisted timer state, without becoming invalid every display tick. Concurrent deletion and Stop must have transactional, deterministic outcomes.
- Existing M1 draft saves and time writes can race around revisions/refresh. Preserve edits and committed results, and keep failed drafts retryable without retrying a successful time write.
- D6 changes the fixture clock contract. Native and frontend clocks, badge day boundaries, and fixture tests must agree. Existing ordinary data and edited seed records must remain intact.
- Local manual Log dates must handle midnight, DST gaps and repeated times, fractional durations, future times, and declared overlap behavior consistently.
- A running count must exclude paused sessions, use singular/plural copy, and remain usable at both supported window sizes. The specifications keep paused state visible in task details and the native tooltip while excluding it from the running header count.

## Planned verification commands and native gate

The scripts and paths below exist and were inspected. They are commands for implementation verification; **none was run as a new M2 test during this kickoff**. M1's prior 31 frontend / 12 Rust passing tests remain historical M1 evidence.

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

The seed script is exactly `tauri dev -- -- --seed` in package.json. Ordinary launch remains `pnpm tauri dev`; quit the existing instance before switching. A Vite-only page cannot prove native timers or persistence.

Final native path: task A → Start → observe advancing seconds → task B → Start → header/tray → Pause/Resume → hide/restore → Quit/relaunch → verify running and paused restoration → Stop each → inspect distinct entries and totals → Log time → reopen/restart. Include same-task repeat actions, failure/retry, task/project deletion, clock/sleep cases, ordinary/fixture separation, and M1 editor-close/drag/attachment regressions. Use disposable records for destructive testing.

Compare the accepted #1b time rail and #2b timer/header treatment at 1600×960 and 1200×760; do not build the #2b planner in M2. Record local font loading, offline behavior, actual visual differences, failed checks, and user-reported checks separately. The planned milestone commit remains `feat(timers): persist concurrent timers and logged time`, owned by Task 2-3 only after the M2 completion gate. Stop for review before M3.

## Current kickoff outcome

The user approved D1–D6 without amendments. This kickoff contains the updated decision record, this overview, three tasks with three paired implementation specifications, and the requested execution prompt. The native/API and UI/controller contracts were reviewed together; each task uses the established eleven-section format. Final kickoff validation on September 12, 2026 passed: three exact task/spec pairs; all eleven required sections in each task; all local links, code fences, and whitespace in nine planning documents; 44 distinct implementation paths with no overlapping ownership (24 existing, 20 planned); matching UI/controller binding fields; and inspected launch/check commands. At kickoff, Git status contained only the nine M2 planning documents, including the two preserved from initial grounding. That planning-only turn changed no tracked production file. Subsequent authorized execution and its actual results are recorded in [M2 verification](../../../verification/M2.md).

The execution order was Task 2-1, then Task 2-2 and Task 2-3. Read each paired specification before editing; no further decision approval is needed for D1–D6. The milestone is complete only after its implemented behavior, automated checks, native/manual gate, and completion commit have actual evidence.
