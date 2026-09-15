# Signal development and personal installation

The user uses an installed copy of Signal for real work. Keep it separate from development.

Changes stay in development until the user explicitly asks to push them live. Do not rebuild, reinstall, restart, or migrate the personal installation as part of an ordinary fix.

- Run development using the default `src-tauri/tauri.conf.json` (`dev.contactestablished.signal`). Use `pnpm dev:seed` for the existing sample workspace.
- Build personal installers with `pnpm build:installer`. Its override uses the stable application identifier `com.contactestablished.signal`.
- Never use the personal configuration for development or automated tests. Never seed, reset, replace, or edit the user's personal application data as part of testing.
- On Windows, personal data is under `%APPDATA%\com.contactestablished.signal`; development data remains under `%APPDATA%\dev.contactestablished.signal`.
- Preserve the personal identifier across releases so upgrades retain the user's data. Do not bundle local databases or attachments into installers.
- Before an authorized upgrade of the personal installation, close that copy and back up its entire roaming application-data folder, including attachments. Keep the backup outside the application-data folder.
