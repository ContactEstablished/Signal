# Voice Inbox — approved standalone increment

Approved for implementation September 18, 2026. This records the reviewed plan and its implementation boundaries. M5–M7 and the remaining screenshot/text/meeting intake in M8 are not started by this increment.

## Approved experience

1. Open Voice Inbox from the header microphone button. Start a recording, see input level and elapsed time, then Stop & analyze or Cancel. One recording at a time; five-minute maximum.
2. Transcribe locally using the Chorus whisper.cpp approach. Default English model is `base.en`; `small.en` is an optional download. Capture mono 16 kHz PCM and flush the final partial frame before transcription. Silent input produces no recommendations.
3. Submit the transcript, existing project names/IDs and recording datetime/timezone to the separately configured AI provider. Signal has its own endpoint, model and credential settings. Never load Chorus credentials.
4. Review the transcript alongside task recommendations grouped by project. Edit title, project, notes, subtasks, deadline, priority and estimate. Display supporting source excerpts and uncertainty. Optional facts must be supported by the dictation; defaults are medium priority, no deadline, no estimate. Date-only deadlines use the existing 17:00 convention with a visible assumption; ambiguous dates remain unset for review.
5. Propose unknown projects with editable name/color and explicit approval. Create a proposed project only when accepted tasks need it. Ambiguous existing project names require selection.
6. Select all, some or none. Selected tasks are created in **To Do** regardless of dictated status, through existing validation. All selected tasks and required new projects commit together. The review never writes business records automatically.
7. Open created tasks in the existing task editor. Unaccepted candidates remain editable. Finish review or Discard clears the transcript/draft, while preserving created tasks.

## Storage, privacy and failure behavior

- One review draft persists in SQLite across restart, with revision checks. Migration 0007 adds `intake_drafts` and `intake_receipts`; the existing settings table stores voice configuration and pending acceptance identity.
- Ordinary and seeded development use separate databases and separate voice/model/credential directories. Personal data uses the stable personal identifier only in an explicitly authorized personal build.
- Windows DPAPI protects the API key for the current user. The key is not returned to the webview after saving and is not stored in SQLite. No audio, transcript, request body, key or provider response body is logged by the feature.
- A microphone recording stays in memory until local transcription. Native scratch WAV/JSON files are removed after processing/cancel/failure; startup clears interrupted scratch files. There is no audio archive or attachment creation. Draft text remains local until the user clears it.
- Sending to AI requires explicit provider configuration and consent. Authenticated requests use HTTPS and do not follow redirects. Provider usage may incur charges. Cancel prevents late responses from overwriting the review; the provider may still process or bill a cancelled request.
- Processing errors preserve the transcript and review. Retryable draft saves reconcile a lost reply before sending newer edits. Acceptance persists the original request identity before writing and reuses it after restart. Receipts prevent duplicate creation even after the review has been cleared.
- Normalized duplicate titles in the same project require acknowledgment. Native validation failure rolls back the entire selected batch. A created ID cannot be forged or erased by editing the saved review.
- Existing workspace mutation serialization and refresh paths publish created projects/tasks. Pending uncertain acceptance blocks conflicting UI mutations until Retry resolves it.
- The app remains usable offline. Downloaded models support local transcription; manually edited candidates can be accepted without an AI service. Network suggestions report an actionable error and retain local work.

## Implementation map

| Responsibility | Files |
|---|---|
| Header route, settings destination, close guard | `src/App.svelte` |
| Recording, transcript and candidate review | `src/lib/views/VoiceInbox.svelte` |
| Microphone/model/provider configuration | `src/lib/components/voice/VoiceSettings.svelte` |
| Capture/worklet/native API types | `src/lib/voice/` |
| Shared workspace mutation/refresh | `src/lib/state/app.svelte.ts` |
| Native capture, local engine, provider, DPAPI, drafts/receipts | `src-tauri/src/intake/` |
| Atomic reuse of existing creation validators | `src-tauri/src/workspace/tasks.rs`, `projects.rs` |
| Database migration | `src-tauri/migrations/0007_voice_intake.sql` |
| Curated Whisper runtime/license/hashes | `src-tauri/resources/whisper/` |

## Excluded

No background listening, global dictation shortcut, audio archive, cloud transcription, speaker separation, automatic acceptance, meeting creation, screenshots, text-file attachments/import, external calendar synchronization, or automatic task status selection. The personal installation is not rebuilt, installed, restarted or migrated by this development work.

## Acceptance

Automated tests cover capture framing/bounds, provider response failures, draft persistence, selection/editing, atomic creation, duplicate acknowledgment, project approval, retry identity, restart recovery and protected credentials. Native microphone/permission behavior, a configured live AI provider, packaged end-to-end flow, and visual acceptance require the manual checks in [the verification record](../../verification/voice-inbox.md). Build success is not a substitute for those checks.
