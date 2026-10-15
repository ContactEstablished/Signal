# Personal Windows upgrade — 0.1.6

Installed October 6, 2026 at the user's explicit request to build and run a new installer for the meeting outcome controls.

## Release

- Version updated in package.json, tauri.conf.json, Cargo.toml and the application entry in Cargo.lock.
- Includes Attended/Canceled actions on calendar meeting bars, retained gray/struck-through cancellations, undo, occurrence isolation, and released planning time. See [meeting outcomes](meeting-outcomes.md).
- Personal identifier remains `com.contactestablished.signal`; default development remains `dev.contactestablished.signal`.
- Built with `. ./scripts/dev-env.ps1` and `pnpm build:installer`. Release compilation and NSIS packaging passed. The existing Vite chunk-size advisory remains non-blocking.
- Installer: `C:/Projects/ContactEstablished/Signal/src-tauri/target/release/bundle/nsis/Signal_0.1.6_x64-setup.exe`.
- Installer SHA-256: `D2B6B6DF15BA839E623DE4871A11CFDD2AF9FC2B10FF000381E5C58297D98B01`.
- Installed executable: `C:/Users/matth/AppData/Local/Signal/signal.exe`.
- Installed executable SHA-256: `40EDBE1895774DE51EC34E7AF4DDA8F3A145C0CC62B900CC663672C89443BA4D`.

## Backup and preservation

- Closed the installed 0.1.5 copy gracefully and confirmed no personal Signal process remained before backup and installation.
- Full backup: `C:/Users/matth/Signal Backups/20261006-122430-before-0.1.6/com.contactestablished.signal`.
- Copied the entire roaming folder, including attachments and voice files, outside application data. SHA-256 verification matched all 7 files and all 3 subdirectories. The sibling `manifest.json` records paths, hashes, versions and the installer hash.
- Installer ran with `/S /UPDATE`, exit code 0. The executable and Windows uninstall registration report 0.1.6.
- All 7 personal files remained byte-for-byte unchanged immediately after installation and before launch. All 18 installed resource files matched source hashes.
- The installed executable matches the release build except for the expected three-byte Tauri bundle marker (`UNK` → `NSS`).
- Launched the installed executable normally, without seed arguments. It responds and has a window titled Signal.
- After launch, read-only SQLite verification confirmed migration 9 installed the default-zero retained-cancellation column. All migration versions 1–9 are successful and match source SHA-384 checksums. Previous migration history is preserved.
- All existing rows and columns across 28 user tables were unchanged. SQLite integrity passed and there were no foreign-key violations. Verification results are in the sibling `post-upgrade-database-verification.json`.
- After launch, the 4 attachment/voice files still matched their backup hashes. The active workspace lock was excluded from this second check because the running application holds it exclusively; it was included in the full pre-launch verification.
- No personal fixture loading, reset, replacement or test run occurred. No local databases or attachments were bundled.

## Verification limits

Before release, 51 focused frontend tests and 77 native tests passed (2 opt-in live-provider probes ignored). Type checking and the frontend build passed. Only release metadata changed afterward; the actual personal release build and installer were then verified.

The installed application is left open for the user to try the controls. Native visual confirmation remains for the user; process responsiveness and database checks do not constitute visual acceptance.

For recovery, close Signal and preserve its current data separately before using the backup. Restoring it would omit work created after the upgrade.
