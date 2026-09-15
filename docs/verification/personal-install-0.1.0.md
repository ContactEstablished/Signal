# Personal Windows installation — 0.1.0

Installed September 14, 2026 from the current working tree, including the in-progress M4 implementation. This does not change M4's manual acceptance status.

## Build and isolation

- Command: `. ./scripts/dev-env.ps1`, then `pnpm build:installer`.
- Installer: `src-tauri/target/release/bundle/nsis/Signal_0.1.0_x64-setup.exe`.
- Installer SHA-256: `F71932BB4D25E051FD0723AC3B5E4AB4649B0D4E8D9571E90AAA8ACC12A5B584`.
- Installed executable: `%LOCALAPPDATA%\Signal\signal.exe`.
- Start menu shortcut: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Signal.lnk`.
- Personal identifier: `com.contactestablished.signal`.
- Personal data: `%APPDATA%\com.contactestablished.signal`.
- Development identifier and data remain `dev.contactestablished.signal`.

## Verification

- `pnpm check`: zero errors and warnings.
- `pnpm test`: 108 tests passed across 33 files.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 40 tests passed.
- Release build and NSIS packaging succeeded. Vite reported its existing large-chunk advisory.
- Per-user silent installation exited successfully; executable, uninstall registration, and Start menu shortcut exist.
- Installed release launched with `--seed` exits with code 2, before opening any workspace.
- Ordinary installed process launched and exposed a responding window titled `Signal`.
- Personal application-data folder did not exist before launch. Native startup created the database and applied all five migrations successfully.
- SQLite integrity check returned `ok`; no fixture settings were present.
- The user began adding personal projects during verification. The live database was inspected read-only; it was not reset or edited. The first inspection showed zero tasks, meetings, blocks, time entries, attachments, tags, and timer sessions, with a newly added project. Therefore an all-zero database snapshot was not captured before the user began working.
- SHA-256 hashes of both existing development databases matched their pre-install values exactly.
- Visual UI verification was unavailable because the Computer Use native pipe could not connect. No visual acceptance is claimed.

The personal app was left running for the user. Future testing must use the development configuration. Back up the personal data folder with the app closed before an authorized upgrade; never clear it to recreate the initial empty state.
