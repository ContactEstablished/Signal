# Personal Windows upgrade — 0.1.2

Installed September 16, 2026 at the user's explicit request to build and install v0.1.2.

## Release

- Version updated in package.json, tauri.conf.json, Cargo.toml and the application entry in Cargo.lock.
- Includes the Board task drag feedback committed in `d2c6863`: translucent pointer-following card, tinted insertion preview and destination status. This does not add dragging/reordering the project tabs themselves.
- Personal identifier remains `com.contactestablished.signal`; development remains `dev.contactestablished.signal`.
- Built with `. ./scripts/dev-env.ps1` then `pnpm build:installer`. Release compilation and NSIS packaging passed. Vite retains its existing non-blocking main-chunk warning (656.25 kB / 170.10 kB gzip).
- Installer: `C:/Users/matth/Downloads/Signal_0.1.2_x64-setup.exe`.
- Installer SHA-256: `AC78254A0DAF0D1EB144FB1D01DF76CA8E3635B59176070BD1C929C8F8EC0A41`.
- Installed executable: `C:/Users/matth/AppData/Local/Signal/signal.exe`.
- Installed executable SHA-256: `1A551A6A3DCC65342960EF7F85440A90A40898A46021F1ECA17749D56E3383A1`.

## Backup and preservation

- Personal Signal was already closed; verified no process at its installed executable path before backup and installation.
- Full backup: `C:/Users/matth/Documents/Signal Backups/before-0.1.2-20260916-164835`.
- `application-data` contains the entire roaming folder, including attachment directories and settings. The prior 0.1.1 executable is also retained. Compared copied file SHA-256 hashes and directory sets.
- Read-only SQLite integrity verification created an empty WAL and its shared-memory sidecar. The initial pre-install guard detected these additions and stopped before running the installer. Verified every original file remained unchanged and the WAL was empty, then added the sidecars to the backup and refreshed its manifest. No user record was altered.
- All six applied migrations matched source SHA-384 checksums; this release introduces no new migrations.
- Installer ran with `/S /UPDATE`, exit code 0. Installed executable and Windows uninstall registry report 0.1.2.
- All five personal data files were byte-for-byte unchanged immediately after installation and before launching the update. The installed executable matches the build except for the expected three-byte Tauri bundle marker (`UNK` → `NSS`).
- Launched the installed executable normally with no seed arguments. The process responds and has a window titled Signal.
- After launch, read-only SQLite comparison found every row unchanged across all 26 tables. Integrity and foreign-key checks passed; all six migration records remain successful. Verification manifests are beside the backup.
- No personal fixture loading, reset, replacement or test run occurred. No database or attachment files were bundled.

## Verification limits and handoff

The prior development checkpoint passed 147 frontend tests, 49 Rust tests, type checking and build. Only version metadata changed for this release, so those suites were not rerun; the actual 0.1.2 release build and installer were verified above.

The updated personal application is left open. Visual confirmation of dragging a card within a project's Board remains for the user; process responsiveness is not a claim of native visual acceptance. This upgrade does not mark the outstanding M4 manual gates passed. Version changes and this release record are included in the September 18 development checkpoint.

For recovery, close Signal and preserve its current data separately before using this backup; restoring the backup would omit work created after the upgrade.
