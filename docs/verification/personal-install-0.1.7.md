# Personal Windows upgrade — 0.1.7

Installed October 14, 2026 at the user's explicit request, after implementing clipboard image attachments in task dialogs.

- Personal identifier remains `com.contactestablished.signal`; development remains `dev.contactestablished.signal`.
- Built with Visual Studio 2022 Professional's developer environment and `pnpm build:installer`. Release compilation and NSIS packaging succeeded.
- Before installation, confirmed the personal application was closed and copied its entire roaming data folder to `C:/Users/matth/Signal Backups/20261014-131602-before-0.1.7/com.contactestablished.signal`. Verified SHA-256 hashes for all 6 files and the presence of all 3 subdirectories, including attachments. The sibling `manifest.json` records hashes and installation results.
- Installer: `src-tauri/target/release/bundle/nsis/Signal_0.1.7_x64-setup.exe`.
- Installer SHA-256: `9BFEB09E4B2553F8173DD1E1224B9612D96B62F847B8B77E50E5E07C30495869`.
- Ran the installer with `/S /UPDATE`; exit code 0. The installed executable and Windows uninstall registration report 0.1.7.
- All 6 personal data files remained byte-for-byte unchanged immediately after installation and before launch.
- Installed executable: `C:/Users/matth/AppData/Local/Signal/signal.exe`.
- Installed executable SHA-256: `EDB82FBD4BAAC817E008F2A51B62B50F59EDDFD58634E7B3784B99D73815AF76`.
- Compared installed and release executable bytes: only the expected three-byte Tauri bundle marker differs (`UNK` to `NSS`).
- Launched the installed application normally, without seed arguments, and confirmed its process responds.

Before release, 19 focused frontend tests and 4 native attachment tests passed. Svelte checks reported no errors or warnings, and the frontend build succeeded with its existing chunk-size advisory.

No personal workspace tests, seeding, reset, or database replacement occurred. Clipboard behavior was verified with automated editor tests and native image storage tests; visual confirmation in the installed application remains for the user.
