# Mail and calendar integration — proposal

Drafted November 18, 2026. Matthew approved the build order the same day. All three steps are built in development: Google Calendar → meetings, Gmail → AI Inbox Email, and Outlook (classic, on this computer) for both. See the "as built" sections below and `docs/verification/google-calendar-sync.md`, `gmail-email-tasks.md` and `outlook-sync.md`.

## Goal

1. Meetings from Matthew's calendars appear in Signal without retyping.
2. Signal reads incoming email and, with the configured AI provider, suggests tasks (title, project, due date, priority, notes, link back to the email) for him to review and accept.

This is a deliberate exception to PLAN.md's offline core (no accounts or OAuth). Signal stays local-first: mail is read, never sent or changed; tokens are encrypted for the Windows user (DPAPI), not stored in SQLite; nothing is created without review.

## Accounts

| Account type | Calendar route | Mail route | Risk |
|---|---|---|---|
| Gmail / Google Workspace | Google Calendar API (read-only) | Gmail API (read-only) | Low. Needs a Google Cloud OAuth client he owns ("Desktop app"). Publish it to production unverified so refresh tokens do not expire weekly; Google shows an "unverified app" warning once per sign-in. |
| Work Outlook, classic Outlook desktop installed | Outlook's local automation interface (COM) | Same | Low technical risk: uses his already signed-in Outlook, no Microsoft app consent. Not available in "new Outlook". Company IT policy still applies. |
| Work Outlook, calendar publishing allowed | Published calendar (ICS) link | — | Low. Calendar only; many tenants disable publishing. |
| Work Outlook, web only | Outlook on the web inside a Signal window, read from the page | Same | High: breaks when Microsoft changes the page, slow, and most likely to conflict with company IT policy. Last resort. |
| Anything | — | Paste or drag an email into AI Inbox | None. Always works. |

Microsoft Graph is out: work tenants typically block user consent for third-party apps.

## Proposed experience

- Settings → Accounts: add Google account (browser sign-in), add Outlook (desktop) or calendar link. Per account: which calendars, which mailbox folders/labels, sync on/off.
- Calendar: read-only sync every few minutes plus on demand. Events become Signal meetings tagged with their source and external ID, mapped to projects by per-calendar default and optional keyword rules. Updates and cancellations follow the source; Signal edits to imported meetings are limited to project, notes and attendance.
- Email: new mail (default: inbox, last 7 days, skipping promotions/newsletters) is summarized by the AI provider into task suggestions in a new AI Inbox "Email" tab, reusing the existing intake review (edit, pick project, select, create). Each created task keeps a link to the original message. Sending email content to the AI provider needs explicit consent per account, like image sharing.
- Later: "what needs a reply today", daily digest in Your Day, Telegram nudges for urgent suggestions.

## Build order

1. Google Calendar → meetings (OAuth, token storage, sync, migration for external source/ID).
2. Gmail → AI Inbox Email tab.
3. Outlook by the route his setup allows.

## Calendar: as built (November 18, 2026)

Decisions taken while building step 1. They replace the proposal above where they differ.

- **Sign-in.** Desktop-app OAuth client owned by Matthew (Google Cloud project "Signal", External audience, Testing status for now). Browser sign-in with a loopback redirect on `127.0.0.1` and PKCE; scopes `openid email calendar.readonly`. Several Google accounts can be connected.
- **Secrets.** The client file is imported once through a file picker. The client secret and each account's refresh token are stored as DPAPI-protected files under the app-data `calendar/` folder (`calendar-dev/` for the seeded workspace), the same protection the Telegram token uses, instead of Windows Credential Manager. The client ID and the account/calendar choices are ordinary settings. Nothing secret is in SQLite or the repo.
- **Mapping.** Each calendar is either not brought in or brought into exactly one project. Keyword rules are deferred.
- **What syncs.** Each Google event instance in the window from 7 days back to 60 days ahead becomes an ordinary one-off Signal meeting, created and updated through the existing meeting commands, so Your Day, the week view and reminders need no special case. All-day events, declined invitations, out-of-office, focus-time, working-location and birthday events, and events longer than a day are skipped.
- **Ownership.** Google owns title, time, join link and agenda (description, HTML stripped, plus location). Signal keeps project, notes, attendance, reminders and linked tasks. A changed event updates the meeting and keeps those. A cancelled or deleted event removes an untouched meeting, or marks it canceled if it has notes, attendance or tasks. A meeting deleted in Signal is not brought back.
- **When.** Every 10 minutes while Signal runs, right after connecting or changing a calendar's project, and on Sync now. One sync at a time. Read-only: Signal never writes to Google.
- **Storage.** Migration 0011 adds `calendar_links` (account, calendar, event → meeting, start, fingerprint). Removing an account or unlinking a calendar keeps the meetings already brought in.

## Gmail: as built (November 18, 2026)

Decisions taken while building step 2. They replace the proposal above where they differ.

- **Access.** The Google sign-in now also asks for `gmail.readonly`, so the same accounts serve calendar and email. Accounts connected before this change must be connected again; Google shows a separate checkbox for email. Read-only: Signal never sends, moves, labels, deletes or marks mail. The Gmail API must be enabled in the Google Cloud project.
- **Consent.** Per account, Settings → Calendar & email has "Suggest tasks from this inbox", off by default. Turning it on is the consent to send that inbox's new email to the AI provider configured in Voice & AI. It cannot be turned on until Google has granted email access.
- **When.** On demand at first; replaced by automatic checking below.
- **What is read.** `in:inbox newer_than:7d`, skipping the Promotions, Social, Forums and Updates categories. At most 15 emails per check, taking turns between inboxes; the rest wait for the next check and the count is shown. Bodies are plain text (HTML converted), quoted replies removed, at most 4,000 characters. Attachments are not read.
- **Each email once.** Migration 0012 adds `mail_seen` (account, Gmail message ID, time); only the opaque ID is kept, never content. An email is marked only after its suggestions are saved, so a failed or cancelled check repeats it next time. Removing an account clears its rows.
- **Suggestions.** Tasks only. The AI proposes title, notes, subtasks, priority, an inferred due date with the reason in the warning, and an existing project when the email clearly belongs to one. Email never proposes new projects; an item without a project must be given one before it can be created. Email content is treated as untrusted data in the prompt.
- **Review.** One saved Email review. Each check adds to it and never replaces existing suggestions or edits. Each suggestion shows sender, subject, time and inbox, and Open email. Created tasks keep the Gmail link in the task's link field. Clearing the review does not bring those emails back.
- **Storage.** Migration 0013 rebuilds `intake_drafts` and `intake_images` so a review's source can be `email`, copying image reviews first so none is lost to the cascade.

## Outlook: as built (November 18, 2026)

Decisions taken while building step 3. They replace the proposal above where they differ.

- **Route.** Classic Outlook on this computer, through its COM automation interface. Matthew added the three work accounts (affirma.com, trupanion.com, contactestablished.com) to classic Outlook. The new Outlook (`olk.exe`) has no interface other programs can read, so it is not supported. No Microsoft sign-in, app registration or tenant consent is involved; Signal uses the profile Outlook already has.
- **Bridge.** A read-only PowerShell 5.1 script (`src-tauri/src/outlook/bridge.ps1`) compiled into Signal and run hidden with `-EncodedCommand`, one request per run, answering with one JSON line. Requests: list accounts, read calendar events in a window, list recent inbox message IDs, read messages by ID, and open one message in Outlook. It never sends, moves, marks or changes anything. Outlook is started in the background when needed. No Outlook security prompt appeared on this machine when reading senders and bodies.
- **Accounts.** Settings → Calendar & email → "Outlook on this computer" → Find Outlook accounts adds each account with a delivery store; finding again refreshes the store and adds new ones. Each account has one calendar (its default Calendar folder), mapped to a project or not brought in, and its own "Suggest tasks from this inbox" switch, off by default. Outlook accounts are kept in the same calendar settings as Google accounts with `kind: outlook`.
- **Calendar.** Same window, skip rules and ownership as Google, and the same reconcile code: recurring series are expanded by Outlook, each occurrence keyed by its global appointment ID plus start time. All-day, declined and out-of-office items and items over a day are skipped; cancelled meetings are treated as cancelled. The join link is a link in the location, else the first Teams, Zoom, Meet, Webex, GoTo, Chime or Whereby link in the body; the Teams join block is stripped from the agenda. Meetings say "From Outlook" in the meeting dialog.
- **Email.** Same Check email flow as Gmail: inbox messages received in the last 7 days, newest first, at most 200 IDs listed per inbox, 15 emails per check shared with Gmail inboxes. Each email is remembered by its Outlook entry ID in `mail_seen`. Suggestions carry a `mail_ref` (account and entry ID) instead of a web link, and Open email shows the message in Outlook.

## Automatic checking and sure tasks (November 18, 2026)

Approved by Matthew, who removed a stated deadline from the criteria for a sure task. He expects to fine-tune it with use.

- **When.** While Signal is open, it checks the ticked inboxes (Gmail and Outlook) every 15 minutes, first one minute after start, and again two minutes after an inbox is switched on. Check email still runs a check now. A round is skipped when a check is already running, the AI is busy with another job, the time zone is not known yet, or the Email review was edited in the last two minutes.
- **Sure.** A task is sure only when all three hold: a real person wrote to Matthew directly (his address in To, not only CC; not automated mail), they ask for something specific, and it clearly belongs to one of his projects. A deadline is not required; the due date is inferred as before. The AI says whether each task is sure and, if not, why in a few words; Signal checks the person and To parts itself from the email headers.
- **Switch.** Per inbox, "Add sure tasks automatically", off by default and available only while "Suggest tasks from this inbox" is on. Only inboxes with it on can add tasks without review.
- **Added automatically.** Sure tasks are created at once, tagged "From email" (blue), with the email link or Outlook reference. They appear at the top of AI Inbox → Email under "Added automatically" with Open task, Open email and Undo. A sure task that cannot be created (for example a possible duplicate) stays in the review with "Not added automatically: …".
- **Review.** Everything else goes to the review as before, with a one-line "Not added automatically: …" reason.
- **Undo.** Deletes the task and removes it from the review. Refused once time has been tracked on the task.
- **Status.** The Email tab shows the last automatic check ("Checked automatically at 9:25 PM: …") and reloads the review after a background check once nothing on screen is mid-edit.
- **Later.** A Telegram message for high-priority items.
- **Review layout (November 19, 2026).** At Matthew's request, each email suggestion is a collapsed bar: include checkbox, title, sender · priority · due date (and "needs a project" when none is set), and a Delete button. Clicking the title opens the fields. Delete removes the suggestion from the saved review. It does not come back because each email is read only once (`mail_seen`); a later email on the same matter can still produce a new suggestion. Voice and image reviews keep the open layout.

## Open questions

- Does each work account's IT policy allow automated reading of mail on his own machine?
