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
