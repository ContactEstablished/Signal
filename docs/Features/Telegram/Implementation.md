# Telegram bot: how it works

Status: implemented in development on `feature/telegram`. The personal installation has not been rebuilt, restarted, migrated or modified. The decisions this follows are in [Grill-Round-1.md](Grill-Round-1.md) and [Grill-Round-2.md](Grill-Round-2.md).

## What you can do

Message your private bot from Telegram on any device signed in to your account:

| You say | What Signal does |
| --- | --- |
| "Add a task to send the invoice to Acme by Friday" | Shows a card (title, project, deadline) and creates the task when you tap Confirm. |
| "Schedule a design review Tuesday at 2pm for 45 minutes" | Shows a card and creates the meeting when you tap Confirm. |
| "Vendor invoice is done: waiting on vendor, shipped Friday" | Marks the task done and adds a dated "Completion note" section to its notes. |
| "The Thursday standup is cancelled" | Marks the meeting cancelled; it stays on your calendar, struck through. |
| "Delete the vendor call" | Permanently removes it, after a confirmation card that says so. |
| `/today`, `/tomorrow`, "what's on Friday?" | Lists tasks due, overdue tasks (today only) and meetings. No confirmation needed. |
| A voice note | Transcribed on your computer, shown as "Heard: …", then handled like text. |
| `/help`, `/cancel` | Help, or discard whatever Signal is currently asking you. |

When something is missing or unclear, Signal asks with buttons (projects, time, which of several matches, which part of a recurring meeting) and also accepts a typed answer. **Change** on a card lets you type a correction ("make it 4pm").

## Behavior details

- **Confirmation first.** Nothing is saved until you tap Confirm. Cards show the exact item, date, time, time zone, defaults that were filled in, and for meetings the effect on a series.
- **Defaults.** A deadline date without a time becomes 5:00 PM in your time zone; a meeting without a length is 30 minutes; a task without a project asks which one (Signal cannot create projects from Telegram). A meeting without a date or time always asks.
- **Dates.** "Today", "Friday" and similar are worked out from when you **sent** the message, not when Signal handled it. Repeated or nonexistent local times (daylight saving changes) are put to you as a question.
- **Finding items.** Matching is against stored tasks and meetings (never invented by the AI), five per page with More, Search again and Earlier/Later for meeting dates. Already-done tasks are reported, not completed again. Meetings are searched in a bounded window (the next 7 days, or the one day you named).
- **Cancelling.** "Cancelled" keeps the meeting visible with its notes. For a repeating meeting you choose **Cancel this occurrence (keep visible)** or **End series from this occurrence**. Explicit delete or remove requests use the existing removal choices; past meetings keep their existing protections.
- **One thing at a time.** A new message while a question is open is never swallowed as an answer: you are asked to **Continue current request** or **Discard it and start this request**. Each question or card expires 30 minutes after it is shown, and expiry is reported on your next message. Expired or replaced buttons cannot change anything.
- **Reply-only.** Signal never messages you first.

## Setup (Settings → Telegram)

1. In Telegram, message **@BotFather**, send `/newbot` and copy the token. Use **one bot for development and a different one for the personal installation.**
2. Paste the token and Save. Signal checks it with Telegram and stores it protected for your Windows user, in this installation's own folder (`telegram-dev` for the seeded development copy, `telegram` otherwise).
3. Accept the AI disclosure. Telegram text and voice transcripts go to the AI provider configured in **Voice & AI** (audio stays local). The AI settings there must be filled in first.
4. Turn on **Listen for my Telegram messages**. This also turns on "Keep running in the system tray", because the bot only answers while Signal is running.
5. **Pair my Telegram account**, then send `/start 123456` (the code shown) to your bot within 5 minutes. Only that private chat is served afterwards; everyone else is silently ignored. A wrong code is silently ignored; five wrong attempts cancel the code.

## How it is built

All code is under `src-tauri/src/telegram/`; the Settings screen is `src/lib/components/telegram/TelegramSettings.svelte`.

| Part | File |
| --- | --- |
| Telegram HTTP client (long polling, send/edit, file download) | `api.rs` |
| Message → structured requests with the AI provider; strict validation of its output | `ai.rs` |
| Conversation: commands, questions, cards, buttons, applying confirmed changes | `flow.rs` |
| Receiving, authorization, ordering, exactly-once handling, reply outbox | `engine.rs` |
| Looking up projects, tasks, meetings and a day's agenda | `search.rs` |
| Dates, times and daylight-saving checks | `dates.rs` |
| Pairing code and saved settings | `config.rs` |
| Background worker, token storage, Settings commands | `runtime.rs` |
| Voice note decoding (Ogg/Opus → 16 kHz mono) | `voice.rs` |

Reliability rules:

- Every update is **stored before** Telegram is told to forget it (the next `getUpdates` offset comes from the stored updates), so a crash or restart replays it instead of losing it. Telegram keeps undelivered messages for at most 24 hours.
- Handling an update, its effects on your data, the conversation state, the queued replies and "this update is done" all commit in **one database transaction**. Duplicate deliveries and repeated button taps cannot change data twice; a proposal that was applied is remembered.
- Replies are written to an **outbox** and sent afterwards. If Telegram or the network is down they stay queued and are sent later without repeating the change.
- Changes use the existing task and agenda rules (revision checks, request receipts, past-meeting protections). If a task or meeting changed after the card was shown, nothing is applied and a fresh card is shown.
- The bot token is never logged and is removed from every error message. It is encrypted with Windows DPAPI like the AI key. The paired account is checked by numeric user ID and private chat ID, never by username.
- Database changes: migration `0010_telegram.sql` adds `telegram_updates`, `telegram_flow`, `telegram_outbox` and `telegram_applied`. No existing table changed. The task update and agenda apply paths gained `update_in` / `apply_in` variants that run inside a caller-owned transaction; their behavior for the existing screens is unchanged.
- New Rust dependencies: `ogg` and `opus` (libopus is compiled into the app; building needs CMake, which is already installed on this machine).

## Not in v1

Photos and screenshots, proactive messages (reminders, daily agenda, meeting alerts), creating projects from Telegram, editing existing tasks or meetings other than completing and cancelling, groups, and more than one paired account.
