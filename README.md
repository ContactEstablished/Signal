# Signal

Offline desktop workspace built with Tauri 2, Svelte 5 runes, TypeScript, Vite, and SQLite. **M1 is complete and manually accepted by the user.** Projects, Board, task editors, and local attachments now have production code. Planner and timers remain explicit previews.

Use Node 22.14+, pnpm 10.33.2, Rust stable MSVC, Visual Studio's Desktop development with C++ workload, Windows SDK, and WebView2.

```powershell
pnpm install --frozen-lockfile
. ./scripts/dev-env.ps1
pnpm tauri dev
```

The helper selects an installed Visual Studio C++ toolchain in the current shell; it is useful on machines where Rust auto-detects an incomplete newer installation. A correctly configured Developer PowerShell can run the pnpm commands directly.

```powershell
pnpm dev:seed  # isolated September 11, 2025, 13:42 fixture
pnpm check
pnpm test
pnpm build
```

Close the running instance before switching between ordinary and seeded development. `pnpm dev` alone starts Vite and cannot open native local data. The seed is repeatable and never writes to the ordinary database. Runtime fonts are bundled locally.

Settings → Notifications → Keep running in the system tray defaults OFF. When enabled, native Close hides Signal; the Signal tray icon → Open restores it, and Quit exits.

See [M1 results, acceptance, and click-paths](docs/verification/M1.md), [M0 verification](docs/verification/M0.md), [approved M1 decisions](docs/Features/Signal/Phase-1-Decisions.md), and [milestone plan](PLAN.md). M2 has not begun.

Select a project → Board → New task, or press `n` outside a text field. `Ctrl+Enter` creates; `Ctrl+Shift+Enter` creates and opens the Your Day preview. Open a card to edit fields, subtasks, Markdown notes, tags, alerts, and attachments. The project menu provides edit, reorder, and confirmed permanent deletion.

Seeded attachment copies live under `attachments-dev`; ordinary copies use `attachments`, both under Signal's application-data directory. The two bundled fixture files are explicitly synthetic. Deleting an attachment removes the managed copy and preserves its source. Project/task deletion is permanent and requires an affected-record preview; use disposable fixture records for testing.
