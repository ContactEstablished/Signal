# Personal Windows upgrade — 0.1.1

Installed September 14, 2026 following the user's request for the latest local version with all existing content preserved.

## Release

- Version bumped from 0.1.0 to 0.1.1 in package.json, the Tauri configuration, Cargo.toml, and the application entry in Cargo.lock.
- Includes meeting time controls, selected weekly weekdays, combined meeting/attendance saves, friendly block time entry, one-hour Due Soon dragging, and AM/PM displays throughout.
- Personal identifier remains `com.contactestablished.signal`; development remains `dev.contactestablished.signal`.
- Build: `. ./scripts/dev-env.ps1`, then `pnpm build:installer`.
- Delivered installer: `C:\Users\matth\Downloads\Signal_0.1.1_x64-setup.exe`.
- Installer SHA-256: `49BC567B4846818D39FA288118E2A3157C3DB3F7299E731A5E9471FBC78AD713`.
- Installed executable: `C:\Users\matth\AppData\Local\Signal\signal.exe`, product version 0.1.1.
- Installed executable SHA-256: `3F5001607E70D854C63A575741552741734B9C6DC8301F50E3274B305877B72F`.

## Backup and preservation

- Closed the old application normally through its guarded close path; confirmed its process exited before copying data.
- Full backup: `C:\Users\matth\Documents\Signal Backups\before-0.1.1-20260914-200949`.
- Backup contains the entire personal roaming folder, including attachments directory and settings, plus the prior executable. Every copied file was checked against its source with SHA-256. Verification manifests are stored beside the backup.
- Before upgrading, SQLite integrity and foreign-key checks passed, and checksums of all five previously applied migrations matched the source exactly.
- Ran the installer with `/S /UPDATE`; exit code 0. Registry and executable both report 0.1.1.
- All personal data files were byte-for-byte unchanged immediately after installation, before first launch.
- The installed executable differs from the standalone build only in Tauri's three-byte bundle marker (`UNK` to `NSS`), confirmed by an exact byte comparison after accounting for that expected marker.
- Launched normally, without seed arguments. Migration 6 adds the nullable weekly-day column. All six migrations report success.
- Read-only verification after launch compared every original column and row across 25 tables against the backup: all matched exactly. Database integrity and foreign-key checks passed again.
- Preserved counts include 3 projects, 3 tasks, 7 meetings, 3 schedule blocks, and 2 occurrence records.
- The new personal process was left running with a responding window titled Signal. No visual UI acceptance is claimed.

## Checks

- Frontend: 142 tests passed across 36 suites.
- Native: 49 tests passed, including the legacy weekday migration preservation test.
- `pnpm check`: zero errors and warnings.
- Release build and NSIS packaging passed; the existing Vite chunk-size advisory remains.

For any future recovery, first close Signal and preserve its current data separately. The backup reflects the workspace immediately before this upgrade; restoring it would omit work entered afterward.
