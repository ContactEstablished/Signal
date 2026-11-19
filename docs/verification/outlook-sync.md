# Outlook (classic) → meetings and AI Inbox email

Built November 18, 2026 in development. Design: `docs/Features/Signal/Mail-Calendar-Integration-Plan.md`, "Outlook: as built".

## Automated checks

- `pnpm check`: 0 errors, 0 warnings.
- `pnpm test`: 45 files, 208 tests passed.
  - `tests/unit/calendar-settings.test.ts` adds:
    - finding Outlook accounts without a Google client file;
    - the email switch showing without a reconnect;
    - mapping an Outlook calendar to a project;
    - finding again when every account is already listed.
  - `tests/unit/email-inbox.test.ts` adds: Open email on an Outlook suggestion opens it in Outlook, not the browser.
- Rust tests: 148 passed, 3 ignored. `src-tauri/src/outlook/tests.rs` covers:
  - reading a Teams meeting: join link, and agenda without the join block;
  - separate IDs for occurrences of a series;
  - skip and cancel rules matching Google;
  - preferring a link in the location;
  - reading an email and dropping the quoted reply;
  - checking message references;
  - reading the bridge's answer and its errors;
  - the encoded script fitting on a Windows command line.

## Bridge probes on this machine

Classic Outlook with the trupanion.com, affirma.com and contactestablished.com accounts. Read-only probes printed counts and lengths only, never content.

- All three stores were read without an Outlook security prompt, in about 1 to 1.3 seconds per request.
- Affirma: 21 events in the sync window, 20 of them occurrences of recurring series.
- Trupanion and Contact Established: 0 calendar items in classic Outlook at the time of the probe.

## In the dev app

Run: `pnpm tauri dev` with the dev identifier (`dev.contactestablished.signal`), unseeded.

1. Settings → Calendar & email shows "3 · Outlook on this computer". Find Outlook accounts added `matthew.wilson@trupanion.com`, `mwilson@affirma.com` and `matthew@contactestablished.com`, each with one Calendar and the "Suggest tasks from this inbox" switch, off.
2. Mapping the Affirma calendar to Personal brought in 21 meetings, from November 12, 2026 to January 14, 2027, 16 with a Teams join link. Its status showed "Last synced". Sync now afterwards said "Synced. Nothing changed."
3. Your Day for November 18 showed "2027 Open Enrollment Information Meeting" from Affirma next to the Google meetings. Its meeting dialog says "From Outlook. Title, time, link and agenda follow Outlook…", with the Teams join link and the agenda.

The personal installation and its data (`com.contactestablished.signal`) were not touched.

## Pending

- Check email against real Outlook mail, and Open email on an Outlook suggestion. Turning on an Outlook inbox's switch sends that work mailbox's new email to the AI provider, so Matthew turns it on himself.

## Automatic checking

Outlook inboxes use the same automatic checks, sure-task rules and switch as Gmail; the bridge now also returns each message's To addresses so Signal can tell direct mail from CC. Rust tests cover the To parsing. Verified in dev with Gmail mail only so far; see `gmail-email-tasks.md`.

## Known limits

- Classic Outlook only. The new Outlook app cannot be read.
- Outlook is started in the background when Signal needs it.
- Tasks created from Outlook email have no web link; the email is opened from the suggestion while it is in the review.
- A moved occurrence of a recurring meeting is treated as removed and created again, so notes on the old occurrence stay on a canceled copy.
