# Approved M0 decisions

The user's M0 approval supersedes the original proposed plan and conflicting mock details.

- M0 only, bounded to the foundation scope; stop for review before M1. The roughly 12-hour target is a scope estimate, not a minimum runtime or a claim of time spent.
- Tauri 2, Svelte 5 runes, TypeScript, Vite, pnpm. Plain CSS properties, dark mode, lucide-svelte. No Board, planner, or timer behavior in M0.
- Native Windows titlebar above the 52px header. Initial **content** size 1600×960; minimum 1200×760. Dimensions in the request written as `1600960` and `1200760` mean the handoff's width × height.
- Local Sora 600/700 and DM Sans 400/500. Hour labels use Sora 600. No Sora 500.
- All 13 handoff tables and every listed field are retained. Opaque text IDs; canonical UTC ISO timestamps for instants; local `YYYY-MM-DD` and integer minutes for planner blocks. Fractional time-entry minutes supported.
- `hours_worked` is a cache maintained by SQLite triggers inside the writing transaction. `task_time_totals` derives the authoritative aggregate. Direct independent edits to the cache are rejected.
- SQL plugin owns database connections and migrations. Rust uses **the plugin's sqlx pool**, not another connection/persistence library, to perform atomic seed writes and narrowly typed settings updates. Frontend SQL capability allows only load/select. The test-only sql.js dependency runs the checked-in migration in SQLite WASM; it is not application storage.
- Business settings, including close-to-tray, stay in SQLite. The store plugin is used only for window position, size, and maximized state. No window-state plugin or localStorage.
- Close-to-tray defaults OFF. A minimal native Open/Quit menu exists before the window can be hidden. Notifications and full tray timer actions remain M6.
- One isolated debug database behind the native `--seed` argument, using September 11, 2025 at 13:42 America/New_York. Other scenarios are deferred. Ordinary startup never runs the fixture loader.
- Counters derive from actual records. No extra tasks are invented to reproduce mock counts. Fixture conflicts and necessary historical interval assumptions are recorded in `verification/M0.md`.
- Attachment files are deferred to M1. The two reference filenames are inventoried, without fictitious paths or sizes in the database.
- Timer, attendance, blocked-since, recurrence-occurrence, and notification-delivery schema additions wait for their respective milestones. No deletion UI/API or cascading business deletion is implemented.
- Destinations, project creation, and the palette explicitly identify themselves as previews. The palette opens with Ctrl+K on Windows (also accepts Meta+K), closes with Escape, and does not return invented results.

Later decisions in PLAN.md remain unresolved and do not authorize M1.
