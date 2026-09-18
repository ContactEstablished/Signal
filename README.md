# Signal

Offline desktop workspace built with Tauri 2, Svelte 5 runes, TypeScript, Vite, and SQLite. **M1–M3 are complete and manually accepted.** Projects, Board, task editors, attachments, concurrent persisted timers, and manual time logging have production code. Your Day includes scheduling, overlap lanes, block timers, carry-over, quick time/date editing and planning shortcuts. See [M3 verification and final acceptance](docs/verification/M3.md).

## Personal Windows installation

```powershell
. ./scripts/dev-env.ps1
pnpm build:installer
```

Run the installer from `src-tauri/target/release/bundle/nsis/`. It installs Signal for the current Windows user. The installed release starts empty, uses the real clock, and rejects `--seed`. Launch it from the Signal Start menu shortcut; Node, Rust, and the development server are not needed to run it.

The personal build uses `com.contactestablished.signal`, with its SQLite database, attachments, and preferences in `%APPDATA%\com.contactestablished.signal`. Default development keeps `dev.contactestablished.signal` and all existing test data. `pnpm dev:seed` continues to open the existing September fixture workspace. Both copies can run independently.

For future personal updates, keep the personal identifier unchanged. Close the installed app and copy its entire `%APPDATA%\com.contactestablished.signal` folder somewhere safe before running a new installer. Upgrades apply database migrations in place; they must never reset the database or copy fixture data into it. See [AGENTS.md](AGENTS.md) for the development isolation rules.

## Development

Meeting time controls and selectable weekly weekdays have development-only improvements pending an explicit live-update request. See [behavior, compatibility, and verification](docs/verification/meeting-time-weekdays.md). The personal installation remains on its existing build.

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

See [M1 results, acceptance, and click-paths](docs/verification/M1.md), [M0 verification](docs/verification/M0.md), [approved M1 decisions](docs/Features/Signal/Phase-1-Decisions.md), and [milestone plan](PLAN.md). See [M2 results and final acceptance](docs/verification/M2.md). M3 is complete. M4 D1–D8 are approved; the [M4 kickoff and three task/spec pairs](docs/Features/Signal/Tasks/Phase-4-Overview.md) and [execution prompt](docs/Features/Signal/Execution-4-Prompt.md) are ready. All three M4 task/spec pairs are implemented: Today, Week and meeting management. The milestone remains partial pending the required native/manual acceptance in [M4 verification](docs/verification/M4.md). The user authorized committing and pushing the current development work on September 15, 2026; this does not update the personal installation. Automated checks pass (147 frontend tests and 49 Rust tests).

Select a project → Board → New task, or press `n` outside a text field. `Ctrl+Enter` creates; `Ctrl+Shift+Enter` creates and opens Your Day to schedule the task. Open a card to edit fields, subtasks, Markdown notes, tags, alerts, and attachments. The project menu provides edit, reorder, and confirmed permanent deletion.

Seeded attachment copies live under `attachments-dev`; ordinary copies use `attachments`, both under Signal's application-data directory. The two bundled fixture files are explicitly synthetic. Deleting an attachment removes the managed copy and preserves its source. Project/task deletion is permanent and requires an affected-record preview; use disposable fixture records for testing.

Running task cards show a lime timer icon. Click the header running count to see task names, projects, and elapsed times; select a row to open its project and task detail.

Open a task → Time · logged → Start. Pause excludes the paused interval; Resume continues the same session. Different tasks can run together. Stop saves one completed entry; active elapsed time stays separate from logged hours. Closing the app keeps running/paused sessions in SQLite. Running elapsed includes time while the app is closed and follows the system wall clock.

Log time asks for local date, start, and duration in hours/minutes/seconds. It shows the timezone and calculated end, rejects future ends, and allows overlaps/overnight entries. Expand Logged entries for read-only history. If a time operation reports an unknown result, use Retry time operation before editing or closing that task; Retry reuses the original request identity to prevent duplicate entries.

The debug seed command is exactly `tauri dev -- -- --seed`. M2 installs its own fixture anchor and two accepted timers once, after M0 data loading. The simulated September clock advances across relaunches. Repeated seeding preserves edited/deleted records and completed/stopped sessions. Ordinary startup uses the real clock and never installs fixtures. See M2 verification for fixture reconciliation and the current acceptance boundary.

## Voice Inbox (development)

Open the header **Voice Inbox** button. In **Settings → Voice & AI**, download `base.en` (148 MB) or the optional `small.en` (488 MB), select your microphone and save. Configure Signal’s own Chat Completions-compatible HTTPS API URL (either the base URL or full `/chat/completions` endpoint), model ID and API key; consent is required before transmitting transcript/project names. Signal does not read Chorus settings or keys. Provider charges may apply; no cloud transcription is used.

Start recording → Stop & analyze → review transcript and project-grouped suggestions → approve any proposed projects → edit/select tasks → Create selected. Accepted tasks enter **To Do**. Open task uses the existing editor. Finish review or Discard draft clears the saved transcript/review; created tasks remain. Offline, local transcription and manual task review still work; suggestions need the configured service. One draft persists across restarts. Relative dates in seeded development use the fixture clock shown by the app.

This feature is implemented in development. Recording/transcription were confirmed by the user and native provider analysis succeeded; final review/acceptance checks remain pending. Launch with `. ./scripts/dev-env.ps1` then `pnpm dev:seed`; no personal reinstall is needed for development. See [setup, privacy, verification and click-paths](docs/verification/voice-inbox.md).
