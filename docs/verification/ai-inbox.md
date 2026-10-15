# AI Inbox image intake — development verification

Date: September 28, 2026. Implementation is in development and ready for user review. The installed personal application was not rebuilt, restarted, migrated, or modified.

## Implemented

AI Inbox with independent Voice/Image reviews; one pasted, selected, or dropped PNG/JPEG/WebP; native image validation; image preview/enlargement; prompt-driven mixed task/meeting suggestions; individual editing/project assignment; approved project proposals; missing-time/recurrence correction; atomic selected creation; duplicate acknowledgment; durable acceptance recovery; source cleanup on draft removal; existing record-editor links.

Migration 0008 preserves the original voice draft, receipts, provider settings, and pending voice acceptance. Images are transactional SQLite blobs in the selected workspace database. They are not included in installers or attached to created records.

## Checks run

| Check | Result |
|---|---|
| `pnpm check` | Passed: zero errors, zero warnings |
| `pnpm test` | Passed: 41 files, 173 tests |
| Final targeted image/voice regression | Passed: 16 tests, including the additional created-meeting editor-routing check (174 frontend tests total) |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` after loading `scripts/dev-env.ps1` | Passed: 72 tests; two opt-in provider diagnostics ignored by default |
| Final native intake regression after source-ID hardening | Passed: 23 tests; two opt-in diagnostics ignored |
| Explicit `live_image_provider_probe` | Passed: four mixed candidates from the synthetic image, including an unresolved meeting time; no business records created |
| `pnpm build` | Passed; Vite reports its bundle-size advisory (~702 kB main JS before gzip) |
| `cargo fmt` / diff whitespace validation | Applied / passed |
| `pnpm dev:seed` | Native development window launched successfully under the development identifier |

The initial direct Cargo invocation selected an incomplete newer Visual Studio installation. Loading the repository's existing `scripts/dev-env.ps1` selected the installed working C++ toolchain; compilation and the native suite then passed.

New native tests cover supported formats, corrupt/oversized sources, source replacement and stale revisions, independent drafts including cross-source ID collision rejection, source cleanup, migration of legacy voice state, image persistence after database reopen, atomic mixed creation and rollback, duplicate meeting detection, receipt replay after draft removal, missing/ambiguous times, project proposals, and image request construction.

New UI/domain tests cover restored source preview, selected invalid-meeting blocking, selected-task-only acceptance, analysis failure and cancellation with late results, clipboard input, independent image recovery, and explicit repeated-time offset/weekday choices. The existing voice suite remains green.

## Native observations

Using the computer-use skill, inspected the seeded development window, AI Inbox entry, both input tabs, Windows file selection, loaded source preview, prompt editing/autosave, and the editable meeting candidate form. The synthetic image loaded through the Windows picker and displayed alongside the review. An incomplete meeting disabled Create selected and showed the missing-start explanation. The inbox container keeps the header and input tabs outside the scrolling review.

The native file picker required keyboard Enter after setting the filename because its WebView2-owned Open button failed the automation tool's pointer-target check. Keyboard confirmation succeeded. No application defect or permission bypass was needed.

The inspected window capture was 1373×916. Exact 1200×760 and 1600×960 visual acceptance, native clipboard/drop gestures, and the complete UI path from configured image-sharing consent through creation and reopening both record editors remain manual acceptance checks. Provider extraction, mixed persistence, and editor routing are covered independently by the checks above; those do not constitute a claim that every manual gesture was exercised.

## Review path

1. Run `. ./scripts/dev-env.ps1`, then `pnpm dev:seed`.
2. Open **AI Inbox → Image**. Choose `tests/fixtures/intake/mixed-intake.png`, or paste/drop a comparable synthetic image.
3. In **Voice & AI settings**, retain the configured provider; set an image-capable model if needed, enable image-sharing consent, and save.
4. Enter: “Extract all meetings and tasks. Keep uncertain meeting times unset for review.” Analyze.
5. Check project assignment and editable task/meeting fields. Resolve the missing Design sync time or deselect that meeting. Create selected; open created items in their ordinary editors and verify calendar views.
6. Leave an unaccepted suggestion, switch input tabs, and restart. Confirm the independent image review and source persist. Finish/Discard removes draft material while preserving created records.

Live provider diagnostic, synthetic data only:

```powershell
. ./scripts/dev-env.ps1
$env:SIGNAL_IMAGE_PROVIDER_PROBE = 'dev-fixture'
cargo test --manifest-path src-tauri/Cargo.toml live_image_provider_probe -- --ignored --nocapture
```

This explicit diagnostic reads only the development fixture's configured AI connection and protected key. It sends only the checked-in synthetic image and fictional project context, prints only the result count, and neither alters configuration nor writes business records. Ordinary image analysis still requires its separate in-app consent.

## Clipboard paste and automatic interpretation — October 2, 2026

Added a visible **Paste image** action and native clipboard-image reading through the [Tauri Clipboard plugin](https://v2.tauri.app/plugin/clipboard/). The typed Rust command reads only an image on request, converts RGBA pixels to PNG on a blocking worker, and enforces the existing 25-megapixel and 10-MiB limits. No clipboard polling, writing, text reading, or general clipboard permissions were added to the webview.

Ctrl+V outside text fields uses native image reading; browser paste events still accept image files or file items inside text fields. Ordinary text paste remains native to the text field. Imports are blocked during another operation, an open modal/editor, or acceptance recovery. Empty/unreadable clipboard errors preserve the existing preview, suggestions, and unsaved instructions. Existing suggestions still require confirmation before replacement; uncertain source writes reload the saved review before another operation.

Successful image imports now start analysis automatically, including file selection and drops. An image can be analyzed with no typed instructions. The default extraction prompt distinguishes meetings from tasks, extracts visible meeting details (including duration from explicit start/end times), and leaves unclear projects unset rather than inventing an assignment. The review asks which project the item belongs to and blocks creation until one is chosen. Missing or ambiguous start/duration/recurrence still requires correction. Existing image-sharing consent, cancellation, and explicit **Create selected** remain in effect.

Validation:

- `pnpm check`: zero errors and warnings.
- Image/voice component regression: 25 tests passed. New coverage includes empty-inbox native paste through automatic meeting analysis and project selection, clipboard file items, text-paste preservation, duplicate gesture suppression, clipboard errors preserving edits, declined replacement, acceptance recovery, image limits, and uncertain source-write reconciliation.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 74 passed; two live-provider diagnostics ignored. New tests verify clipboard color/alpha preservation, malformed/oversized pixel rejection, optional instructions in the provider request, and a project-unassigned meeting becoming a real meeting with its extracted time, duration, link, and agenda intact after project selection. No duplicate task is created.
- `pnpm build`: passed with the existing chunk-size advisory (~707 kB main JavaScript before gzip).
- `cargo fmt -- --check` and `git diff --check`: passed.
- `pnpm dev:seed`: development binary compiled and opened a responding **Signal · September 2025 fixture** window using the default development identifier.

The real Windows clipboard gesture and a new live model response were not exercised in this pass; component tests mock clipboard/provider replies, and native tests use synthetic pixels and an in-memory database. For manual review, copy a screenshot of an invitation, open the development window's Image tab, paste, choose a project if asked, and review the prefilled meeting before creating it. Also try ordinary text paste into optional instructions and an empty clipboard.

The installed personal 0.1.3 process retained its PID/start time and executable SHA-256. No personal installer was built or run, and no personal application data was used in tests or changed by this work. The earlier version-bump edits remain in the working tree.

## Authorized personal upgrade to 0.1.4 — October 2, 2026

At the user's subsequent request, bumped the package, Cargo package/lock entry, and Tauri version to 0.1.4. Built with `pnpm build:installer` using the unchanged `com.contactestablished.signal` identifier. Both Signal copies were closed before installation; the entire personal roaming data directory, including attachment directories, was copied outside application data and verified by file counts and SHA-256 hashes.

The installer completed successfully in update mode. Windows registration and the installed executable both report 0.1.4. All 18 bundled resources match the source files; the installed executable matches the build except for Tauri's expected NSIS bundle-type marker. Personal data files remained byte-for-byte identical to the backup before first launch. The installed copy then opened a responding Signal window. The new clipboard and AI interpretation flow is now available for user testing; this upgrade does not claim completion of the remaining real clipboard and live-provider acceptance checks.

## Image response limits — October 5, 2026

The reported error was `Image analysis failed. Verify that your configured model supports image input. AI response was truncated. Try a shorter transcript. Your review is saved.` This corresponds to a successful provider response with `finish_reason: "length"`, not a missing-settings or image-support rejection. The shared transport capped output at 8,000 tokens, and the image error wrapper added image-capability advice to every failure.

The [OpenRouter model catalog](https://openrouter.ai/api/v1/models), checked October 5, lists `deepseek/deepseek-v4.1-flash` with image input, optional reasoning, and reasoning enabled at high effort by default. [OpenRouter's reasoning documentation](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens) explains that reasoning generally shares `max_tokens` with visible output. Reasoning exhausting that budget is a likely explanation; the original response's token-usage breakdown is unavailable, so this is not a verified observation of its reasoning usage.

Development changes disable optional reasoning for this exact OpenRouter model alongside the already supported Flash alias, increase the image output ceiling to 16,000 tokens, and recognize response-limit errors separately. Image failures now explain that the output limit was reached, preserve the image and instructions, and suggest retrying or cropping to fewer items. The unconditional image-capability warning is removed, and shared transport messages refer to the review rather than a transcript. Partial output still produces no new recommendations; there is no automatic retry. A higher output ceiling permits larger responses and can increase usage when needed.

Validation:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked intake::` after loading `scripts/dev-env.ps1`: 26 passed, two live-provider diagnostics ignored. Regressions cover reasoning configuration for both input types, unchanged requests for other providers/models, and reasoning-only or partial truncation in streamed and regular JSON responses.
- `pnpm test -- tests/unit/image-inbox.test.ts tests/unit/voice-inbox.test.ts`: 26 passed. The new UI regression verifies that edited instructions, the source image, and prior recommendations survive the output-limit error without automatic retry or record creation.
- `pnpm check`: zero errors and warnings; `cargo fmt -- --check` and `git diff --check`: passed.

No live provider request was made, and these checks used synthetic inputs. The installed personal application was not rebuilt, reinstalled, restarted, migrated, or modified. The fix remains in development pending an explicit request to push it live.

## Authorized personal upgrade to 0.1.5 — October 5, 2026

At the user's explicit request to create and run the fixed installer, bumped the package, Cargo package/lock entry, and Tauri version to 0.1.5. `pnpm build:installer` completed successfully with the personal override and unchanged `com.contactestablished.signal` identifier. The generated NSIS script includes only the application executable and the 18 checked-in transcription/license resources; no local database or attachments are bundled.

Signal was already closed before backup. Copied the entire roaming application-data folder to `C:\Users\matth\Signal Backups\20261005-091031-before-0.1.5\com.contactestablished.signal`, including attachment and credential directories. Verified all seven files by size and SHA-256 and all three subdirectories by count. The adjacent `manifest.json` records the file hashes. Reverified the original against that manifest immediately before installation.

Ran `src-tauri/target/release/bundle/nsis/Signal_0.1.5_x64-setup.exe` with `/S /UPDATE`; it exited with code 0. Before relaunch, all personal data files remained byte-for-byte identical to the backup. Windows registration and the installed executable both report 0.1.5. All 18 installed resources match the source files. The installed executable differs from the release build only in the three bytes of Tauri's expected `__TAURI_BUNDLE_TYPE_VAR_UNK` to `__TAURI_BUNDLE_TYPE_VAR_NSS` marker.

Launched the installed executable at `C:\Users\matth\AppData\Local\Signal\signal.exe` and verified the responding `Signal` window (PID 8416). The output-limit and optional-reasoning fix is now installed for user testing. No image analysis or provider request was triggered during installation verification; the saved image review remains available for the user to retry.

Installer SHA-256: `71A1DE56FD6DCCE4649AA8A455C1F3002A5DF46E18B4B0583D102186925151E0`.
