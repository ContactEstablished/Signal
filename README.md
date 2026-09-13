# Signal

Offline desktop workspace built with Tauri 2, Svelte 5 runes, TypeScript, Vite, and SQLite. **M1–M3 are complete and manually accepted.** Projects, Board, task editors, attachments, concurrent persisted timers, and manual time logging have production code. Your Day includes scheduling, overlap lanes, block timers, carry-over, quick time/date editing and planning shortcuts. See [M3 verification and final acceptance](docs/verification/M3.md).

Use Node 22.14+, pnpm 10.33.2, Rust stable MSVC, Visual Studio's Desktop development with C++ workload, Windows SDK, and WebView2.

```powershell
pnpm install --frozen-lockfile
. ./scripts/dev-env.ps1
pnpm tauri dev
```

The helper selects an installed Visual Studio C++ toolchain in the current shell; it is useful on machines where Rust auto-detects an incomplete newer installation. A correctly configured Developer PowerShell can run the pnpm commands directly.

```powershell
pnpm dev:seed  # isolated fixture clock, anchored once at Sep 11, 2025, 13:42 ET
pnpm check
pnpm test
pnpm build
```

Close the running instance before switching between ordinary and seeded development. `pnpm dev` alone starts Vite and cannot open native local data. The seed is repeatable and never writes to the ordinary database. Runtime fonts are bundled locally.

Settings → Notifications → Keep running in the system tray defaults OFF. When enabled, native Close hides Signal; the Signal tray icon → Open restores it, and Quit exits.

See [M1 results, acceptance, and click-paths](docs/verification/M1.md), [M0 verification](docs/verification/M0.md), [approved M1 decisions](docs/Features/Signal/Phase-1-Decisions.md), and [milestone plan](PLAN.md). See [M2 results and final acceptance](docs/verification/M2.md). M3 is complete; M4 Today, Week and meeting management kickoff is authorized.

Select a project → Board → New task, or press `n` outside a text field. `Ctrl+Enter` creates; `Ctrl+Shift+Enter` creates and opens Your Day to schedule the task. Open a card to edit fields, subtasks, Markdown notes, tags, alerts, and attachments. The project menu provides edit, reorder, and confirmed permanent deletion.

Seeded attachment copies live under `attachments-dev`; ordinary copies use `attachments`, both under Signal's application-data directory. The two bundled fixture files are explicitly synthetic. Deleting an attachment removes the managed copy and preserves its source. Project/task deletion is permanent and requires an affected-record preview; use disposable fixture records for testing.

Running task cards show a lime timer icon. Click the header running count to see task names, projects, and elapsed times; select a row to open its project and task detail.

Open a task → Time · logged → Start. Pause excludes the paused interval; Resume continues the same session. Different tasks can run together. Stop saves one completed entry; active elapsed time stays separate from logged hours. Closing the app keeps running/paused sessions in SQLite. Running elapsed includes time while the app is closed and follows the system wall clock.

Log time asks for local date, start, and duration in hours/minutes/seconds. It shows the timezone and calculated end, rejects future ends, and allows overlaps/overnight entries. Expand Logged entries for read-only history. If a time operation reports an unknown result, use Retry time operation before editing or closing that task; Retry reuses the original request identity to prevent duplicate entries.

The debug seed command is exactly `tauri dev -- -- --seed`. M2 installs its own fixture anchor and two accepted timers once, after M0 data loading. The simulated September clock advances across relaunches. Repeated seeding preserves edited/deleted records and completed/stopped sessions. Ordinary startup uses the real clock and never installs fixtures. See M2 verification for fixture reconciliation and the current acceptance boundary.
