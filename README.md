# Signal

Offline desktop workspace built with Tauri 2, Svelte 5 runes, TypeScript, Vite, and SQLite. **M0 foundation only; Board, planner, and timers remain explicit previews.**

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

See [M0 verification and review paths](docs/verification/M0.md), [approved decisions](docs/decisions.md), and [milestone plan](PLAN.md). M1 starts only after review.
