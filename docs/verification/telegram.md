# Telegram bot: development verification

Date: October 25, 2026. Implemented on `feature/telegram` in development. The installed personal application was **not** rebuilt, restarted, migrated or modified, and no personal data or credentials were read or used. Design: [Implementation.md](../Features/Telegram/Implementation.md).

## Automated checks

| Check | Result |
| --- | --- |
| `cargo test --lib` (after `. ./scripts/dev-env.ps1`) | Passed: 114 tests (was 78). Three opt-in tests are ignored: two provider diagnostics and the real-recording check below. |
| Telegram tests only | 36 passed, 1 ignored (manual real recording) |
| `cargo fmt -- --check` on the new code | Passed |
| `pnpm check` | Passed: 0 errors, 0 warnings |
| `pnpm test` | Passed: 43 files, 198 tests (5 new for the Settings screen) |
| `pnpm build` | Passed (Vite's existing bundle-size advisory only) |
| Launch of `pnpm dev:seed` | The development window opened under the development identifier; migration 10 (`telegram_intake`) applied to the seeded development database. The app was then closed. |

### What the Telegram tests cover

The conversation is tested end to end against a **fake Telegram** and a **scripted AI**, with a real temporary SQLite database and the real task and agenda code. No real bot token or AI key is used by any automated test.

- Pairing: one-time code, 5-minute expiry, five-failure lock, right code from a group chat or wrong user ignored, silence when pairing was never started, nobody else served afterwards.
- Add a task: project question, card with the 5:00 PM default shown, nothing saved before Confirm, saved exactly once, stale or repeated button taps change nothing, the same Confirm delivered twice applies once.
- Add a meeting: missing time asked, typed "2:30pm" understood, 30-minute default marked; repeated and nonexistent local times (daylight saving) put to the user as a question.
- Complete a task: completion note appended exactly once, existing notes preserved, "waiting on vendor" stays note text (the task is not set to blocked), asking again reports it is already done; several matches offered as buttons; a task edited after the card was shown is not completed and a fresh card follows.
- Cancel a meeting: marked cancelled and kept (notes intact); cancelling again is reported; explicit delete permanently removes after its own confirmation; a recurring meeting asks for scope and "End series" uses the cutoff.
- Lookups: `/today` and `/tomorrow` use stored data with no AI call; a message handled the next morning still resolves "tomorrow" from when it was sent.
- Conversation rules: unrelated text while a question is open asks Continue/Discard and does not spend an AI call; `/cancel` clears everything; expired cards cannot be used and expiry is reported; each new question or card gets a fresh 30 minutes; Change applies a typed correction.
- Reliability: an unreachable Telegram leaves replies queued and sends them later without repeating the change; an update stored before a restart is handled once even when Telegram delivers it again; AI failure changes nothing.
- Voice: a voice note is transcribed and "Heard: …" is shown before the card; an unreadable one asks for text.
- Telegram HTTP client against a local fake server: request shapes, error classes (bad token, another listener, rate limit), the token never appearing in any error, download size limits before and during streaming.
- Settings screen: save token, consent and listening gates, tray prompt, pairing code display and completion, error display.

### Real voice-note check

A spoken sentence was generated with Windows text-to-speech, encoded to Telegram's format (Ogg/Opus, 48 kHz mono) with ffmpeg, decoded by the new decoder, and transcribed by the bundled whisper.cpp with the development `small.en` model:

> "add a task to send the invoice to acme by friday at 3 p.m., and mark the vendor review as done."

Re-run with `SIGNAL_TEST_OGG=<file.ogg> SIGNAL_TEST_MODEL=<ggml model> cargo test --lib real_opus -- --ignored --nocapture`.

A defect found and fixed while testing: the first version of the 48→16 kHz low-pass filter had a wrong centre tap and let high tones through. The filter now has a direct test.

## Manual test with your development bot

Everything below uses the **development** copy only (`pnpm dev:seed`, identifier `dev.contactestablished.signal`). Do not use your personal installation or its bot for this.

Prerequisites: the dev copy's **Voice & AI** settings have an AI provider URL, model ID and API key saved (the dev copy keeps its own), and ideally the `base.en` or `small.en` model installed for voice. The seeded workspace's clock is fixed at September 11, 2025, so "today" in Telegram means that day inside the dev app.

1. In Telegram, message **@BotFather**: `/newbot`, choose a name and a username ending in `bot`, copy the token.
2. Start the dev app (`pnpm dev:seed`) and open **Settings → Telegram**.
3. Paste the token → **Save token**. Expected: "Connected to @yourbot".
4. Tick the AI disclosure, then **Listen for my Telegram messages**. Expected: "Connected to Telegram." and the tray option turns on.
5. **Pair my Telegram account**. Expected: a message like `/start 123456`. Send exactly that to your bot in Telegram. Expected: "✅ Paired" with the help text, and Settings shows paired. Try a second Telegram account or a group: it must get no reply.
6. Try each, confirming or cancelling with the buttons:
   - "add a task to send the invoice to Acme by Friday" → pick a project → card shows 5:00 PM default → Confirm → the task appears on the board (the app refreshes by itself).
   - "schedule a design review next Tuesday" → asked for a time → type "2pm" → card with 30 min (default) → Confirm.
   - "mark the \<one of your seeded tasks\> done, finished the first pass" → card → Confirm → open the task: notes end with a dated "Completion note".
   - "the \<seeded meeting\> is cancelled" → Confirm → it stays on the calendar, struck through.
   - "delete the design review" → card says it permanently deletes → Confirm.
   - `/today`, `/tomorrow`, "what's on Friday?"
   - Start a request, then send an unrelated message while it is waiting → Continue/Discard buttons. Then `/cancel`.
   - Tap **Change** on a card and type "make it 4pm".
   - Send a voice note: "add a task to water the plants in \<project\>" → "Heard: …" then a card.
7. Close the Signal window (do not Quit). Expected: Signal stays in the tray and still answers in Telegram. Use the tray icon → Quit, send a message, start the dev app again within 24 hours: the message is handled after reconnecting.
8. **Unpair** and **Remove bot** in Settings when finished.
