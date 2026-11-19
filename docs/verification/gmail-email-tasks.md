# Gmail → AI Inbox email tasks

Built November 18, 2026 in development. Design: `docs/Features/Signal/Mail-Calendar-Integration-Plan.md`, "Gmail: as built".

## Automated checks

- `pnpm check`: 0 errors, 0 warnings.
- `pnpm test`: 45 files, 206 tests passed.
  - `tests/unit/email-inbox.test.ts` covers:
    - checking only inboxes that suggest tasks;
    - saving before a check and adding to the review;
    - the check summary;
    - blocking items without a project;
    - Open email;
    - creating through the email recovery path;
    - keeping existing suggestions when an inbox fails or a check errors;
    - the settings link when no inbox is set up.
  - `tests/unit/calendar-settings.test.ts` covers the reconnect prompt and the per-inbox toggle.
- Rust tests: 140 passed, 3 ignored.
  - `src-tauri/src/intake/email_tests.rs` covers:
    - Gmail message parsing: headers, plain text, HTML and snippet fallback, attachments skipped;
    - trimming quoted replies;
    - Gmail links;
    - taking turns between inboxes;
    - the prompt;
    - tying suggestions to their email and dropping unknown emails and projects;
    - adding to a review without losing created items;
    - email reviews being tasks only with web links;
    - created tasks keeping the link, priority and due date;
    - migration 0013 keeping saved voice and image reviews.
  - The sign-in test now requires `gmail.readonly` and rejects any scope that could change mail.

## In the dev app

Run: `pnpm tauri dev` with the dev identifier (`dev.contactestablished.signal`), unseeded.

1. Migrations 0012 and 0013 applied to the dev database. Schema checks:
   - `intake_drafts` allows `email`;
   - `intake_images` references the rebuilt table;
   - the foreign-key check is clean;
   - `mail_seen` exists.
   A copy of the dev database is in `C:\Users\matth\.signal\dev-backups\`.
2. AI Inbox shows Voice, Image and Email tabs. Email intake said "No inbox is set up for suggestions yet." with Check email disabled. Calendar & email settings opened the renamed settings page.
3. Settings → Calendar & email listed both connected Google accounts with "Connect … again to allow email", because they were connected before email access existed.

Real mail, November 18, 2026 (dev copy):
- Matthew enabled the Gmail API, reconnected both accounts with email allowed and turned on suggestions for both;
- Check email ran against real mail and produced suggestions;
- he created one task from a suggestion. The dev database holds it ("Finish EventNation email function after Postmark approval") with `external_url` set to the Gmail link for the source message, and Matthew confirmed it works.

The personal installation and its data (`com.contactestablished.signal`) were not touched.

## Automatic checking and sure tasks (November 18, 2026)

Design: plan doc, "Automatic checking and sure tasks".

Automated checks:
- `pnpm check`: 0 errors, 0 warnings.
- `pnpm test`: 211 passed. New: the "Added automatically" group with Undo, reloading after a background check, and the check summaries; the per-inbox "Add sure tasks automatically" switch.
- Rust tests: 153 passed, 3 ignored. New in `email_tests.rs`:
  - the prompt asks whether each task is sure without needing a deadline;
  - sure needs a person writing to you directly about a project;
  - only inboxes with the switch on mark sure tasks;
  - sure tasks are added and tagged, a duplicate stays for review, Undo removes the task and is refused once time is tracked;
  - a review edited moments ago is left alone.

In the dev app (dev identifier, real mail):
1. The dev workspace had no project that matched the mail, so an EventNation project was created in dev, the dev Email review was cleared, and the 10 `mail_seen` rows for one recent matt@mattwilson.dev batch were removed so those emails were read again. The switch is on for matt@mattwilson.dev in dev.
2. The background check ran on its own at 9:25 PM (10 emails, 4 suggestions; before the project existed).
3. Check email: "Checked 10 new emails: 2 tasks added automatically, 4 suggested tasks added below." Both tasks (from Faith Keiser, To Matthew, EventNation) appeared under "Added automatically" with Open task, Open email, Undo and the source line, and carry the blue "From email" tag in the database. The four review items each give a reason: status update with no specific ask; unclear project; vague ask.
4. Undo on "Reply to Faith on rebuilding SVG mockups as HTML" showed "Removed “Reply to Faith on rebuilding SVG mockups as HTML”." and the task is gone from the database.

The personal installation and its data were not touched.

## Collapsed suggestions with Delete (November 19, 2026)

- `pnpm check`: 0 errors. `pnpm test`: 212 passed; new test covers the collapsed bar (title, sender, priority, due date), including without opening, opening the fields, and Delete removing the item from the saved review. Existing email tests now open rows before reading fields.
- Dev app: the four review items showed as bars, three marked "needs a project". Opening "Verify Azure access…" showed its fields. Delete on "Reply to Sãntos Júnior's follow-up" showed "Deleted …", and the stored review (revision 8) no longer holds it.

## Known limits

- Automatic checks run only while Signal is open.
- 15 emails per check; a busy week takes several clicks.
- Outlook mail is covered in `docs/verification/outlook-sync.md`.
