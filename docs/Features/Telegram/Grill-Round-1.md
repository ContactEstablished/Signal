# Telegram Bot Integration: Design Questions, Round 1

Branch: `feature/telegram`

Status: **Answered.** ✅ **Selected answer** marks the design choice selected by Codex at the user's request. These are the working decisions for planning; they do not mean the feature is implemented or authorize changes to the personal installation. Added requirements and unresolved implementation checks are called out below.

| Question | Selected answer |
| --- | --- |
| Q1 - Where the bot runs | **(c)** Local long polling with Signal kept running in the tray. |
| Q2 - Who may talk to the bot | **Yes.** Pair one Telegram account; private chats only. |
| Q3 - Confirm before changes | **(a)** Confirm every task or meeting change before saving. |
| Q4 - Development vs. personal bot | **Yes.** Separate bots and credentials for each installation. |
| Q5 - Message types | **Text and voice notes in v1; photos later.** |
| Q6 - Proactive messages | **Yes to reply-only v1.** Reminders and agenda pushes come later. |
| Q7 - Lookups | **Yes.** `/today`, `/tomorrow`, simple day queries, and `/help`. |

A Telegram bot is not tied to a phone. You get a **bot token** from @BotFather, and Signal accepts messages only from **your Telegram account**. That account works on your phone, desktop and any other signed-in device.

---

## Q1 - Where the bot runs

Telegram has to deliver your messages somewhere.

- (a) Signal polls Telegram itself, with no server. Messages sent while Signal is closed wait at Telegram for up to 24 hours and are processed the next time Signal opens.
- (b) A small cloud relay that is always online, receives messages and queues them for Signal. It could acknowledge receipt while the PC is off, but current-data lookups and local changes would still wait for Signal unless data and processing were also hosted.
- ✅ **Selected (c): Option (a), plus Signal keeps running in the system tray when the window is closed. The bot answers while Signal is running and the PC is awake and online.**

> ✅ **Selected answer: (c).** Use local long polling without a hosted relay. Enable the existing tray setting during bot setup, with clear wording that closing the window keeps Signal running. Sleep, shutdown, loss of connectivity, or explicitly quitting Signal stops replies; tray mode does not provide launch-at-login.
>
> Telegram retains undelivered updates for **no longer than 24 hours**. On reconnect, process updates still available; do not promise lossless delivery. Persist received updates before acknowledging them and prevent duplicate changes on retries. See the [Telegram update delivery documentation](https://core.telegram.org/bots/api#getting-updates).
>
> The Signal database stays local, but bot messages pass through Telegram, and interpretation may send content to the configured AI provider. This is not an entirely local data path.
>
> *Code finding:* The current Rust database helper requires the database to be open. Start polling only after database initialization. Tray mode ("keep running when window closed", currently off by default) hides the window and keeps the process alive. See `src-tauri/src/db.rs` and `src-tauri/src/window.rs`.

---

## Q2 - Who may talk to the bot

Anyone can find a bot by its username. Proposed lock-down: a one-time pairing step.

1. You paste the bot token into Signal's Settings.
2. Signal shows a 6-digit code.
3. You send `/start 123456` to the bot.
4. Signal saves your Telegram user ID and silently ignores everyone else.
5. Settings has a way to unpair or re-pair.

> ✅ **Selected answer: Yes, with private-chat-only pairing.** A one-time code pairs one Telegram account, and every other sender is ignored. Check the paired numeric user ID and private chat ID for messages and confirmation-button callbacks; do not authorize by username.
>
> **Added requirements:** Generate the code randomly, expire it after five minutes, and invalidate it after five failed attempts across all senders. Pairing is available only after it is explicitly started in Signal Settings. Consume the code on success; unpairing invalidates pending confirmations. Keep the token out of logs and source control and store it securely within the correct installation's credential scope.

---

## Q3 - Confirm before changing anything?

Once Signal has understood a message, should it:

- ✅ **Selected (a): Show what it understood as a card in Telegram with buttons ✅ Confirm / ✏️ Change / ✖️ Cancel, and save only after you tap Confirm.**
- (b) Save right away and reply with a summary and an Undo button.
- (c) Hybrid: create tasks right away with Undo, but always confirm deletions, meeting changes and anything it is unsure about.

> ✅ **Selected answer: (a).** Require confirmation before creating, completing, cancelling, deleting, or editing tasks and meetings. This matches the existing intake review flow. Read-only lookups and help reply immediately without confirmation.
>
> **Added requirements:** Show the exact target, proposed change, resolved date/time and timezone, and recurrence scope when relevant. Resolve ambiguity before offering Confirm. Change produces a revised preview; Cancel discards the proposal. Repeated taps must not apply a change twice, and a target changed since preview must be reviewed again.

---

## Q4 - Development bot vs. personal bot

Telegram delivers a bot's messages to only one listener at a time. If the dev build and your personal Signal used the same bot, they would steal messages from each other. Use **two bots**: one for development/testing (e.g. `@MattSignalDevBot`) and one for your personal install. Each token lives in that app's own settings and data folder, which matches the dev/personal separation in AGENTS.md.

> ✅ **Selected answer: Yes, use two bots.** Use a dedicated dev bot during implementation and testing. Create and configure the personal bot only when the user explicitly asks to push this live. Bot creation is a future setup task, not an action performed by answering this document.
>
> Development uses `dev.contactestablished.signal` and `pnpm dev:seed`; the personal installation uses `com.contactestablished.signal`. Never copy the personal token, pairing, or data into development or automated tests. Run one polling worker per bot token.

---

## Q5 - What message types are in scope

Plain text is required. Your app already has local dictation and image prompts, so these could be added:

- **Voice notes:** Signal transcribes them locally and handles them like text.
- **Photos/screenshots:** for example, a photo of a meeting invite, sent through the existing image intake.

> ✅ **Selected answer: Text and voice notes in v1; photos/screenshots later.** Voice capture is useful on mobile and can reuse local transcription. Show the transcript with the proposed action so transcription mistakes can be corrected before confirmation.
>
> *Code finding and implementation requirement:* The current transcription function accepts PCM samples and writes a 16 kHz WAV file before invoking whisper.cpp (`src-tauri/src/intake/engine.rs`). Telegram voice notes therefore need a download/decode/resample step into that existing path. Verify real OGG/Opus input during development; support is not established by this decision. Voice notes remain part of the selected v1 scope, with decoding and validation required before calling that scope complete. Missing models, unsupported audio, or oversized recordings should produce a clear reply asking for text instead.

---

## Q6 - Does Signal message you first?

Should v1 only answer your messages, with no reminders, daily agenda pushes or "meeting in 10 minutes" alerts? Those could come in a later phase.

> ✅ **Selected answer: Yes, keep v1 reply-only.** Include replies, clarification questions, confirmation cards, results, and errors associated with your requests, including delayed responses after reconnecting. No scheduled reminders, daily agenda pushes, or unsolicited meeting alerts in v1.

---

## Q7 - Lookup commands

Besides add, cancel and complete, should the bot answer read-only questions in v1, such as `/today` (today's tasks and meetings) and "what's on Thursday?"? Lookups help you pick the right item to cancel or complete.

> ✅ **Selected answer: Yes.** v1 includes `/today`, `/tomorrow` and simple "what's on <day>" queries, plus `/help`. Use Signal's existing agenda rules for which tasks and meeting occurrences belong to a date, and query the database for results rather than letting the model invent them.
>
> **Added requirements:** Resolve relative dates against the original message timestamp in Signal's configured timezone, so an overnight delay does not silently change "today". Include the resolved date and timezone in the reply; clarify ambiguous dates. Return enough identifying detail to distinguish similarly named items, and state clearly when no items match.

---

## Coming in Round 2 (depends on the answers above)

- Which AI model interprets the messages. The existing intake uses a configurable OpenAI-compatible endpoint. Its voice prompt proposes tasks, and its image prompt proposes tasks and meetings; neither provides the full Telegram lookup/update command flow yet.
- What a task's completion note does. Tasks have no completion-note field today.
- How cancelling a recurring meeting should work: one occurrence, following occurrences, or "marked cancelled" (struck through).
- Follow-up question design: button choices plus free text.
