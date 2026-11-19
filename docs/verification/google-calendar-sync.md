# Google Calendar sync

Built November 18, 2026 in development. Design: `docs/Features/Signal/Mail-Calendar-Integration-Plan.md`, "Calendar: as built".

## Automated checks

- `pnpm check`: 0 errors, 0 warnings.
- `pnpm test`: 44 files, 201 tests passed. `tests/unit/calendar-settings.test.ts` covers the client-file step, connecting, choosing a project, Sync now, and the reconnect prompt.
- Rust tests: 129 passed, 3 ignored. `src-tauri/src/calendar/tests.rs` covers creating meetings and repeat syncs that change nothing; a Google change updating the meeting while keeping notes; deleted or cancelled events removing untouched meetings; a meeting with notes marked canceled and then restored; a meeting deleted in Signal not coming back; declined events; unlinking keeping meetings; client-file parsing; PKCE against the RFC 7636 example; redirect state checks; ID-token email; event parsing; and calendar list merging.

## In the dev app

Run: `pnpm tauri dev` with the dev identifier (`dev.contactestablished.signal`), unseeded, with a real clock. The process was `src-tauri\target\debug\signal.exe`.

1. Settings → Calendars showed the three steps. Choose client file opened the Windows picker. The client JSON (kept outside the repo) loaded with "Google client file loaded." `calendar\client.bin` appeared in the dev app-data folder and does not contain the secret in plain text.
2. Connect a Google account opened Google sign-in in the default browser. Matthew connected two accounts (a mattwilson.dev account and a taxapp.com work account). Both listed their calendars, including shared team calendars and US holidays.
3. Two test projects, TaxApp and Personal, were created in the empty dev workspace. Linking the work calendar to TaxApp showed "… meetings will appear in TaxApp." Linking the mattwilson.dev calendar to Personal worked the same way. The background sync started on each change. Sync now then reported "Synced. Nothing changed."
4. The dev database held 45 TaxApp meetings, from November 12, 2026 to January 15, 2027, with 45 calendar links. The Personal calendar had no timed events in that period.
5. Your Day for November 18 showed the two meetings Google had for that day at the right local times: 11:30 AM and 1:00 PM Eastern.
6. Opening Dailystandup showed the TaxApp project, the note "From Google Calendar. Title, time, link and agenda follow Google…", the Google description as the agenda, the Google Meet join link, Eastern time, 30 minutes, and a 10-minute reminder.

The personal installation and its data (`com.contactestablished.signal`) were not touched; it has no calendar folder. This feature is pending a future authorized installation.

## Known limits

- One project per calendar. A work calendar that mixes projects puts all of its meetings into one project until keyword or AI routing exists. A meeting's project can be changed by hand.
- The meeting dialog still lets you edit title and time on imported meetings. The next sync replaces them with Google's values, as the note in the dialog says.
- The Google Cloud app is in Testing, so Google may expire sign-ins after 7 days. Signal then shows "Connect … again". Publishing the app removes that limit.
