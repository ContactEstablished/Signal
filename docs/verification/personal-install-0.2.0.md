# Personal Windows upgrade — 0.2.0

Installed November 19, 2026 at the user's explicit request ("bump the version number and install the latest software so I can use it today"). This release adds the Telegram bot, ten project colors, Google and Outlook calendar sync, and email-to-task suggestions in the AI Inbox.

- Version bumped from 0.1.7 to 0.2.0 in package.json, the Tauri configuration, Cargo.toml, and the application entry in Cargo.lock.
- Personal identifier remains `com.contactestablished.signal`; development remains `dev.contactestablished.signal`.
- Before release: `pnpm check` 0 errors, `pnpm test` 212 passed, `cargo test` 153 passed (3 ignored).
- Built with `pnpm build:installer`. Release compilation and NSIS packaging succeeded.
- Before installation, confirmed no Signal process was running and copied the entire roaming data folder to `C:/Users/matth/Signal Backups/20261119-093629-before-0.2.0/com.contactestablished.signal`. All 6 files matched by SHA-256 and all 3 subdirectories, including attachments, were present. The sibling `manifest.json` records the hashes.
- Installer: `src-tauri/target/release/bundle/nsis/Signal_0.2.0_x64-setup.exe`.
- Installer SHA-256: `D3C0F22BC94A4DA20EEAA012447D78E049A4E3E4DEDAA49AA7B1AC5E1CADB7B7`.
- Ran the installer with `/S /UPDATE`; exit code 0. The installed executable and the Windows uninstall registration report 0.2.0.
- Installed executable: `C:/Users/matth/AppData/Local/Signal/signal.exe`, SHA-256 `CBCB1C2DB5C226D93C1C86CBDDF8367C3B4E36138726E4FDEB8F132FA8E1FF02`.
- All 6 personal data files were unchanged by the installer, before launch.
- Launched the installed application normally, without seed arguments. The process responds and opened on Your Day with the user's projects and tasks.
- On launch the database moved from migration 9 to 13 with no failed migrations. Read-only counts match the backup: 4 projects, 15 tasks.

No personal workspace tests, seeding, reset or database replacement occurred. Calendar and email accounts connected in development are not carried over; the user connects them in the installed copy under Settings > Calendar & email.
