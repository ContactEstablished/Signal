# Telegram Bot Integration: Design Questions, Round 2

Branch: `feature/telegram`

Round 1 (Q1-Q7) is settled in [Grill-Round-1.md](Grill-Round-1.md). These decisions build on those answers.

Status: **Answered.** ✅ **Selected answer** marks the design choice selected by Codex at the user's request. These are working decisions for planning, not claims that the feature is implemented or authorization to change the personal installation. Changes to the original recommendations and added requirements are identified below.

| Question | Selected answer |
| --- | --- |
| Q8 - AI model | **Reuse the AI Inbox provider and model**, with consent that explicitly covers Telegram input. |
| Q9 - Completion note | **Yes.** Append a dated section to existing task notes. |
| Q10 - Meeting cancellation | **Amended.** Preserve a cancelled meeting by default; make removal and ending a series explicit choices. |
| Q11 - Finding items | **Yes.** Up to five distinguishable matches per page, with more results or refinement. |
| Q12 - Missing details | **Yes to all four defaults**, shown in the confirmation preview. |
| Q13 - Follow-ups | **Amended.** One active question; accept relevant replies without silently consuming unrelated requests. |
| Q14 - Testing | **Yes.** Isolated fake-service tests, then a manual test with the dev bot and seeded development app. |

---

## Q8 - Which AI model interprets messages

Telegram interpretation would use the AI endpoint, model and credential already configured in the AI Inbox. Telegram would have no separate provider/model settings. Provider requests require consent covering the input being sent.

> ✅ **Selected answer: Reuse the AI Inbox provider and model, with explicit Telegram consent coverage.** Reuse the shared consent mechanism, but update its disclosure to cover Telegram text, locally generated voice transcripts, and the project/item context needed for interpretation. During Telegram setup, require acceptance of that expanded scope; do not silently treat an older consent flag as acceptance. Revoking consent stops subsequent AI requests.
>
> **Added requirements:** Handle `/help`, `/cancel`, `/today`, `/tomorrow`, and button selections deterministically where possible. These local operations need no AI call. The model proposes structured intents; application code resolves IDs, checks permissions, queries actual data, and applies only confirmed changes. Send only necessary context to the provider. Missing configuration or provider failure produces a clear reply without applying changes.
>
> *Code finding:* `src-tauri/src/intake/settings.rs` stores the configurable OpenAI-compatible connection and protects its key for the Windows user in installation-specific storage. The current consent text in `VoiceSettings.svelte` covers transcripts and project names for task suggestions. The voice prompt proposes tasks; the image prompt proposes tasks and meetings. Telegram completion, cancellation, and natural-language lookup intents need their own validated interpretation path.

---

## Q9 - Completion note

Tasks have no completion-note field. When you say "mark X done: waiting on vendor, shipped Friday", should Signal append a dated "Completion note" section to the task's existing notes, with no new database column?

> ✅ **Selected answer: Yes.** Append a dated **Completion note** section to the existing `notes_md` only when the user supplies a note. Preserve the existing notes and the user's wording. Show the appended text in the confirmation preview. No dedicated completion-note column or migration is needed for this choice.
>
> **Added requirements:** Date the section using the confirmed completion time in Signal's configured timezone, including the offset. Save the note and completion in one transaction through the existing task mutation rules, including `done_at` and revision checks. Retries must not append the note twice. If the task changed after preview, show a fresh preview; if it is already done, report that without appending another completion note. The example's "waiting on vendor" remains note text, not an instruction to change status to blocked.

---

## Q10 - Cancelling meetings

The original proposal treated cancellation as removal: permanently delete a one-off, hide a recurring occurrence, or cut off that occurrence and all following occurrences.

> ✅ **Selected answer (amended): Use "marked cancelled" for ordinary cancellation, and make removal explicit.** "Cancel this meeting" should retain its calendar entry and notes, visibly marked cancelled. Apply this to one-off meetings and individual recurring occurrences using the existing outcome-recording path.
>
> For a recurring meeting without an explicit scope, offer **Cancel this occurrence (keep visible)** / **End series from this occurrence (remove this and following)**. Selecting either leads to the usual confirmation card. Ending the series uses the existing cutoff; it does not generate an unlimited set of marked-cancelled occurrences. From the first occurrence it ends the whole series only if existing past-meeting protections allow it.
>
> Explicit requests to **delete/remove** use the existing removal choices: permanently delete a one-off, hide this recurring occurrence, or remove this and following. The preview must name the exact date, scope, and effect, including permanent deletion where applicable. Keep cancellation notes and existing occurrence notes intact; preserve existing revision checks and protections against removing past meetings. If the requested scope is unclear, ask before previewing.
>
> *Code finding:* `src-tauri/src/agenda/mod.rs` implements hard deletion, occurrence hiding, and following cutoffs separately from `record` with `attendance: "canceled"`. The latter sets `marked_cancelled` and retains the occurrence. This selection deliberately uses that existing distinction instead of equating cancellation with deletion.

---

## Q11 - Finding the right item

When your words match several tasks or meetings, such as "cancel the standup", Signal shows up to 5 matches as buttons, plus a "type a different name" option. It completes tasks only if they are still open. Meeting-title lookup needs to be added alongside the existing agenda task search.

> ✅ **Selected answer: Yes, with pagination and identifying details.** Show up to five matches per page, with **More results** when needed and a way to refine the search. Include project and status/due date for tasks, and project, occurrence date/time, and timezone for meetings. A single matching series does not by itself identify which occurrence to change.
>
> **Added requirements:** Search stored data, not model-generated matches. For recurring meetings, resolve candidates within a bounded date range; if the request lacks enough date context, ask rather than expanding an unbounded series. Zero matches prompts refinement. Even one match still requires the Q3 confirmation before mutation. Bind selection buttons to stable IDs/occurrence references and revalidate them on use. Already-completed tasks may be reported as done, but must not be completed again.

---

## Q12 - Defaults for missing details

When you do not give a detail, Signal fills it in like this:

- **Task with no project:** ask with buttons listing your projects.
- **Task with a date but no time:** due 17:00 local, as the app already does.
- **Meeting with no duration:** 30 minutes.
- **Meeting with no time or date:** always ask.

> ✅ **Selected answer: Yes to all four, with visible defaults.** Ask for a missing task project using paginated project buttons and typed selection. Default a date-only task deadline to **17:00 in Signal's configured timezone**, and a meeting without a duration to **30 minutes**. Always ask for a missing meeting date or start time.
>
> **Added requirements:** Mark inferred values in the confirmation preview and allow Change before saving. A task with no date has no deadline. Explicit duration or end time overrides the duration default. Use the original message timestamp for relative dates, as selected in Q7; clarify ambiguous dates, timezones, and daylight-saving times rather than guessing. If no suitable project exists, ask the user to create one in Signal and retry; project creation through Telegram is outside this v1 decision.

---

## Q13 - Follow-up questions

Each question shows buttons for likely answers and accepts a typed reply. Only one question is active at a time. The original proposal treated every new message as its answer; that behavior is amended below. A question expires after 30 minutes, and `/cancel` clears it.

> ✅ **Selected answer (amended): One active question, with contextual replies.** Accept a button, a direct reply to the question, or text that clearly answers it. If a message appears to start another request or is ambiguous, offer **Continue current request** / **Discard it and start this request**. Do not silently replace the pending request or treat unrelated text as a field value.
>
> **Added requirements:** `/cancel` always discards the pending flow without changes, and `/help` remains available. Read-only `/today` and `/tomorrow` may reply without replacing the pending question. Expire each question and final confirmation 30 minutes after it is issued; persist its identity, state, and expiry across restarts. Expired, replaced, or already-consumed buttons cannot apply changes. Check expiry when processing input, and report expiry on the next interaction rather than sending an unsolicited timer alert. All changes still require the explicit Confirm button from Q3.

---

## Q14 - Testing

Implementation testing will use a fake Telegram server, followed by a final manual test with the dedicated dev bot using `pnpm dev:seed`. Live AI interpretation requires the development app's own AI credential and consent in Settings.

> ✅ **Selected answer: Yes.** Use isolated automated tests with fake Telegram and AI responses, synthetic data, and temporary test storage. Then validate the complete flow manually with the real dev bot and seeded development workspace. Automated tests must not depend on live bot tokens or AI credentials.
>
> **Required coverage:** Pairing expiry and attempt limits; unauthorized users/chats/callbacks; missing or revoked AI consent; ambiguous matches and defaults; completion notes saved exactly once; retained cancellation versus removal; recurrence scope and past-meeting protection; stale revisions; follow-up expiry and replacement; repeated updates/taps; restart and retry around both commit and reply delivery; delayed-message dates; and real OGG/Opus decoding into the local transcription path.
>
> The manual check covers text, voice, lookups, clarification, Confirm/Change/Cancel, tray operation, and reconnecting after downtime. Use only `dev.contactestablished.signal` with the dev bot. Never read, seed, reset, migrate, or test against personal application data or credentials. These are future implementation checks; answering this document does not claim that they have passed.
