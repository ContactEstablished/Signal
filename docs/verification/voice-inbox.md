# Voice Inbox verification — September 18, 2026

Status: implementation present in development; native visual, microphone and live-provider acceptance remain open. The installed personal copy and its data were not changed. No personal installer was built or installed. Earlier uncommitted Board drag, mint palette and version changes were preserved.

## Implemented

- Header Voice Inbox route, local mono recording, level meter, elapsed time, five-minute limit, Stop & analyze and Cancel.
- Local Whisper transcription with bundled x64 CPU runtime and separate downloadable base.en/small.en models. The AudioWorklet ships as a separate local asset so it does not require a `data:` script exemption in the app CSP.
- Separate Voice & AI settings: microphone, model download/progress/cancellation, HTTPS provider base URL, model ID, protected key and explicit transmission consent. Changing the endpoint clears the consent checkbox.
- Saved transcript and recommendations grouped by project, source excerpts/warnings, editable task fields/subtasks, project proposal approval/color, select all/none/subset, explicit Create selected into To Do, and Open task.
- SQLite migration 0007; revision-checked draft saves; atomic batch creation reusing existing task/project validators; durable receipt replay; duplicate-title acknowledgment; pending acceptance recovery across restart.
- Ordinary offline operations and manual candidate creation remain available. No image/text-file/meeting intake or M5 work was added.

## Commands

Run in `C:\Projects\ContactEstablished\Signal`:

```powershell
. ./scripts/dev-env.ps1
pnpm dev:seed
```

This opens the existing September 2025 development workspace, under `dev.contactestablished.signal`. Relative dictated dates use the app's fixture date/time. For a real-clock, empty ordinary development workspace, close the seeded development instance and run `pnpm tauri dev`. Neither command accesses the personal application data.

```powershell
pnpm check
pnpm test --maxWorkers=4 --minWorkers=1
cargo test --manifest-path src-tauri/Cargo.toml --lib
pnpm tauri build
```

The last command uses the **default development identifier**, without the personal override. Its installer is a development artifact, not a personal update. Do not run `pnpm build:installer` or install an artifact unless the user explicitly requests a personal upgrade and its required backup is completed.

## Evidence

- Svelte/TypeScript check: pass, zero errors/warnings.
- Full frontend suite: **160 tests passed across 38 files** (`pnpm test --maxWorkers=4 --minWorkers=1`). The 9 Voice Inbox tests were rerun successfully after the final cancellation/close refinement.
- Rust suite: **60 tests passed**, zero failures (`cargo test --manifest-path src-tauri/Cargo.toml --lib`), including 11 native intake tests. `cargo fmt --check` also passes.
- Default Tauri development launch: built and started `src-tauri/target/debug/signal.exe --seed`; a native main-window handle and the September fixture title were observed. No startup error appeared in the captured launch output. This is launch evidence, not visual acceptance of the screen.
- Default Tauri release/NSIS build: succeeded. All 13 curated engine executable/DLL files plus provenance, MIT license and hash manifest were copied into `target/release/resources/whisper`. The final Vite output contains `assets/pcm-worklet-DSxcrb3f.js` (900 bytes), referenced as a local asset. Third-party license notices are bundled separately under `resources/licenses`. The existing main-chunk size advisory remains non-blocking.
- Engine smoke check: the bundled `whisper-cli.exe --help` confirms the CLI flags in use. A downloaded base.en model and the upstream public `samples/jfk.wav` produced a nonempty, correct transcript and valid JSON, exit 0. These are synthetic/public verification artifacts in ignored `.verification/voice/`; no user voice or project data was transmitted. Model SHA-256: `a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002`.
- Provider transport tests use a local HTTP fixture with a synthetic key (the public app settings still require HTTPS): configured request path/model/auth, valid response, rate limit, truncated output and redirect rejection pass. They are not a live-model acceptance test.
- Read-only inspection of the seeded development database confirms migration 7 succeeded and both intake tables exist.
- SQLite tests reopen a temporary database and replay the original acceptance without duplicating tasks. Selection defaults, approved new-project creation, late batch rollback, normalized duplicates, lost save replies and immutable created IDs are covered. A Windows DPAPI roundtrip/corruption test passes with synthetic data.

Native UI automation blocker: `sky.list_apps()` returned **“Computer Use native pipe is unavailable: failed to connect native pipe: The system cannot find the file specified. (os error 2)”**. Consequently native microphone permission, audio-device behavior, visual layout, and packaged end-to-end recording were not claimed as verified. No live provider credentials were supplied or taken from Chorus; no real AI call was made.

## Setup and manual acceptance

1. **Model and microphone:** launch development → Voice Inbox → Voice & AI settings → Download base.en → choose microphone → Save settings. Confirm progress, success and a clear permission/device error if access is denied. Start recording, speak, confirm meter/time advance, stop. With AI unconfigured, the transcript should still appear and remain editable, followed by a settings/configuration error for suggestions.
2. **AI configuration:** Settings → Voice & AI → enter your provider's HTTPS **API URL**, model ID and API key → check consent → Save settings. Either a base URL or a full `/chat/completions` endpoint is accepted; Signal appends the suffix only if missing. Leave key blank on later saves to keep it, or use Remove saved key. This configuration is separate from Chorus. Do not paste a key into chat or verification notes.
3. **Known and new projects:** dictate “For Atlas Migration, write release notes and add a subtask to review them. For Launch Test, check the landing page.” Stop & analyze. Confirm transcript, task/project suggestions and supporting excerpts. Edit a title, notes and multiline subtasks; approve/color the proposed Launch Test project. Deselect one task → Create selected → Open task. Confirm To Do and the saved edits. The unselected suggestion should remain; an unneeded unapproved proposal should create no project.
4. **All/some/none and duplicates:** Select none creates nothing; select a subset and verify the count. An identical title in the same project should stop the batch until acknowledged or edited. Missing project/blank title/unapproved needed project must not leave partially created projects/tasks.
5. **Recovery and retention:** leave an unaccepted edited review, close/reopen development and return to Voice Inbox. Confirm transcript, edits and selections survived. Finish review/Discard should clear local review content and preserve already-created tasks. A failed AI request must preserve it. Cancel a recording/request and confirm no late recommendations replace the current screen.
6. **Timing and limits:** verify a short recording flushes the last spoken words; silence does not generate tasks; reaching five minutes automatically stops. Unplug/disable a test input and check the error/retry path. Verify relative deadlines against the displayed fixture date/timezone; ambiguous dates should require review.
7. **Native close and offline:** navigate away or close while recording/transcribing and verify the stop/leave confirmation. Once the model is installed, disconnect network and verify local transcription/manual editing/acceptance while AI reports unavailability. Ordinary task, meeting, planner and timer actions must remain usable.
8. **Packaged development gate:** before a personal release, verify microphone capture, model loading and provider flow in the packaged **development** app. No personal installation upgrade is authorized by these checks. Inspect layout at 1200×760 and 1600×960, keyboard focus, scrolling and error states.

## Storage and limits

Database: `signal-dev-september-2025.db` for seeded development, `signal.db` for ordinary development. Voice files are in `voice-dev` or `voice` under that build's app-data directory. Models persist there; DPAPI ciphertext is `credential.bin`; transient WAV/JSON goes under `temp/<operation-id>` and is removed after processing. Normal cancellation waits for the native worker to finish cleanup before allowing close; startup cleans scratch files left by an interrupted process. A failed Windows cleanup is reported rather than silently claiming deletion.

The transcript/review stays in SQLite until Finish review/Discard. Receipts retain only created record metadata and a request fingerprint for safe retries, without audio/transcript. SQLite row deletion is logical deletion, not a forensic secure erase. Provider-side retention depends on the configured provider. Five-minute audio, 80 KB transcript, 100 recommendations, 50 project proposals, bounded response size and 90-second provider/300-second transcription timeouts apply. Model downloads check expected byte size over HTTPS; this is not model signature verification. English only in this increment.

No personal release, installation, commit or push was performed as part of this implementation request.

## Model selection repair — September 18, 2026

The user reported Start recording did nothing after installing small.en. Read-only inspection of the development fixture showed a complete 487,614,201-byte small.en file, while `settings.voice.whisper_model` remained `base.en`. The download-completion handler reloaded the entire settings record, discarding unsaved model/provider form edits.

The settings screen now refreshes only installation/key-presence metadata after downloads. Download & select selects the newly installed model and explicitly prompts Save settings; re-downloading another installed model preserves the selection. Provider, microphone, consent and pending key edits remain in the form. The selected model is displayed, and the recording error identifies both the missing selected model and any installed alternative.

The existing development fixture's transcription-model preference was corrected from base.en to small.en with a conditional, single-field SQLite update. Provider settings/key were preserved; no credential was decrypted or printed, no provider request was sent, and personal application data was not accessed. The installed small.en model successfully transcribed the public JFK fixture through the bundled engine (exit 0). The running development app loads the preference on each Start recording attempt; no reinstall/restart is needed.

Validation: `pnpm check` passes with zero errors/warnings. Four settings regression tests cover preserving unsaved provider/key/consent fields, selecting a fresh download, preserving another selection on re-download, and retaining edits on download failure. The targeted settings/Voice Inbox suite passes all 13 tests with no unhandled errors; `pnpm build` also passes with the existing non-blocking chunk-size advisory. Actual microphone recording remains for the user's retry.

## Suggest tasks HTTP 404 repair — September 18, 2026

The user confirmed native recording and transcription worked, then reported no recommendations after Suggest tasks. The saved development provider URL already ended in `/api/v1/chat/completions`; the client appended `/chat/completions` again. This produced the wrong request path. OpenRouter documents the full endpoint as `https://openrouter.ai/api/v1/chat/completions` ([official reference](https://openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request)). Its public model catalog also listed the configured model alias; the model selection was preserved.

The native client now accepts either a base URL or full Chat Completions endpoint, including trailing slashes. It preserves custom provider path prefixes, and still requires HTTPS without embedded credentials/query/fragment for configured providers. The settings label/help describe both accepted forms. HTTP failures distinguish URL/model, authentication, permissions, quota and server errors without exposing provider response bodies. Voice Inbox errors use the danger color and the request button reads “Suggesting tasks…” while waiting.

The saved review was present (revision 2, 335 transcript characters, no candidates) before the fix. No transcript, provider settings, key or model ID was changed, and no authenticated external request was made by the repair. Native development reload restores the saved review; retry via Voice Inbox → Suggest tasks without recording again. The personal installation is unchanged.

Regression coverage includes real local HTTP requests for base/full endpoints with and without trailing slashes, custom-prefix preservation, and status-specific error guidance. `pnpm check` passes with zero errors/warnings; all 62 Rust tests and the 13 targeted settings/Voice Inbox tests pass. `pnpm build` passes with the existing chunk-size advisory. The development process restarted with a freshly compiled binary and the September fixture title. Live recommendations require the user's retry with the configured provider.

## Suggestion wait recovery — September 18, 2026

The user corrected an incorrect API key, then reported an apparently stalled Suggest tasks request. The saved development credential authenticated successfully against OpenRouter's read-only key endpoint (HTTP 200). No key, credential bytes, raw provider response, or user transcript was printed or written to diagnostics. A synthetic completion using the saved endpoint/model and the production prompt returned one task in 4.5 seconds through .NET HttpClient. A second opt-in probe through the production Rust reqwest client, request builder, and suggestion parser returned two synthetic candidates in 5.0 seconds. These calls used only synthetic text/project names and did not create records. They establish that the current provider configuration works; they do **not** reproduce or establish the cause of the original UI stall.

The whole native suggestion command now has a 90-second deadline, including draft/settings loading. Timeout drops its operation future and releases its job guard. Network timeouts receive an actionable Timeout error. A separate 100-second frontend deadline covers a missing desktop-bridge response and requests cancellation without waiting indefinitely for that reply. Late results are ignored; task creation is not subject to this retry timeout. While waiting, the screen shows elapsed seconds, the expected maximum wait and Cancel. HTTP 401 wording now describes authentication failure and checking the provider/key pairing rather than concluding that the key itself is wrong.

The native dev watcher rebuilt and restarted the September fixture window. The saved review remained unchanged (revision 2, 335 transcript characters, zero candidates, original update timestamp). The personal Signal process retained its original process ID/start time; no personal data, settings, installation, or release artifact was changed.

Validation: `pnpm check` passes with zero errors/warnings. All 14 automated native intake tests pass; the live diagnostic is ignored by default and passed separately when explicitly invoked. The deadline regression verifies stalled work is dropped and subsequent work succeeds. `pnpm build` passes with the existing non-blocking chunk-size advisory. The frontend deadline tests cover an unresolved IPC call, late response, transcript preservation and timer cleanup after success/failure. A component regression covers elapsed time, error display and successful retry.

Final targeted frontend run: **17 tests passed across 3 files** (`pnpm test -- tests/unit/voice-api.test.ts tests/unit/voice-inbox.test.ts tests/unit/voice-settings.test.ts`). The first version of the new component test incorrectly looked for an editable input value in `textContent`; correcting the assertion to inspect the input value made the test pass without another production-code change.

The opt-in native probe reads only the **development fixture** provider settings/key, sends synthetic text and may incur provider usage. Run only when diagnosing this configured provider:

```powershell
. ./scripts/dev-env.ps1
$env:SIGNAL_VOICE_PROVIDER_PROBE='dev-fixture'
cargo test --manifest-path src-tauri/Cargo.toml live_dev_fixture_provider_probe -- --ignored --nocapture
Remove-Item Env:SIGNAL_VOICE_PROVIDER_PROBE
```

User retry: in the running development app, open Voice Inbox and click Suggest tasks on the saved transcript. Watch the elapsed-seconds status; expect recommendations or a clear error within about 90 seconds (100 seconds if the desktop bridge fails to reply). Cancel preserves the saved transcript. No new recording is needed. Native UI automation remains unavailable because its native pipe cannot be reached, so the user's original in-app request and visual retry remain unverified.

## Saved-transcript timeout repair — September 18, 2026

The previous timeout change exposed the failure but did not solve it. This follow-up reproduced the failure using the actual saved development transcript and project list inside the running Tauri application. Temporary metadata-only tracing showed successful draft/settings/key loading, HTTP 200, then no completed non-streaming response before the 90-second deadline. The trace contained stages, times and byte counts, never credentials, transcript text, raw response bodies or reasoning text. Native UI automation still cannot connect to its native pipe.

OpenRouter's public model metadata identifies the configured `~deepseek/deepseek-flash-latest` alias as optional reasoning enabled by default at high effort. A non-thinking request completed once in about nine seconds but failed source validation; a subsequent non-streaming attempt again remained on keep-alives for over a minute before a development rebuild interrupted it. Disabling reasoning alone was therefore not treated as a verified fix. A streamed request with numbered source excerpts returned four valid recommendations from the saved transcript in 2.4 seconds. Provider latency varies; that is measured evidence, not a promised response time.

Final implementation:

- Request streamed Chat Completions and consume SSE incrementally, stopping at its completion marker. Handle fragmented UTF-8, keep-alives, truncation, incomplete streams and provider errors without displaying partial recommendations or private error bodies. Preserve the existing response-size limit and cancellation/deadline behavior. Providers that return a complete JSON response still work.
- Disable optional high reasoning only for the verified OpenRouter/DeepSeek alias pair; preserve the configured model/key and all other providers/models. This uses OpenRouter's [documented reasoning control](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens).
- Supply numbered excerpts and request `source_id`. Signal resolves the citation to the exact original transcript text. Unknown, negative, fractional and nonnumeric IDs are rejected. Strict legacy quote validation remains for providers returning the prior format.

The full running-desktop command then succeeded in approximately 40 seconds and its normal revision-checked draft save stored **six unaccepted recommendations**. A read-only database check after the final rebuild confirmed revision 3, unchanged 335-character transcript, six candidates, zero created IDs, every source excerpt present in the transcript, and every assigned project existing. No task-creation command was invoked. The temporary startup probe and all tracing code were removed; no automatic analysis runs at startup in the delivered code. The saved recommendations are ready under **Voice Inbox** without another recording or provider call.

Final verification: `pnpm check` passes with zero errors/warnings; the targeted frontend suite passes **17 tests**; the final native intake suite passes **18 tests** with one explicitly ignored live-provider diagnostic. New tests cover SSE transport, byte-fragmented Unicode, rejected interrupted/error streams, authoritative source-ID resolution and the narrow provider/model reasoning override. The Tauri dev watcher rebuilt and opened the final September fixture window. Personal Signal retained its original PID/start time and installation; no personal data or installer was changed. Visual acceptance of the recommendations remains with the user.

For an explicitly authorized reproduction using a saved **development** review, the ignored probe additionally accepts `$env:SIGNAL_VOICE_PROVIDER_INPUT='saved-review'`. It sends that review and development project names to the already configured provider after checking saved consent, but does not save or accept results. Without this separate opt-in, the probe uses synthetic text/projects. Clear both diagnostic environment variables after use. No keys belong in command arguments, logs, documentation or chat.

## Commit verification — September 18, 2026

At the user's commit/push request, the full frontend suite passed **168 tests across 40 files**, the full Rust library suite passed **67 tests** (one live-provider diagnostic intentionally ignored), `pnpm check` reported zero errors/warnings, `cargo fmt --check` passed, and `pnpm build` succeeded with the existing main-chunk advisory. All 13 vendored Whisper runtime hashes match the checked-in manifest. The reviewed changes contain no credential files, local databases, recordings, model downloads, or temporary diagnostics. This checkpoint also includes the pending Board clock-tick drag repair, Mint project color, and v0.1.2 version/release documentation. It does not install or update the personal application or close remaining manual acceptance gates.
