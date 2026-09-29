# AI Inbox — image intake

Approved for implementation September 28, 2026. Standalone development increment drawn from M8; does not start M5–M7 or ship a personal upgrade.

## Experience

- AI Inbox replaces the Voice Inbox header label, with Voice and Image input tabs. Voice retains its existing task-only workflow.
- Each input has one independently saved review. Image intake accepts one PNG/JPEG/WebP via clipboard paste, file picker, or native file drop. Native code validates actual format, decoding, the 10 MiB byte limit, and the 25-megapixel limit.
- Write instructions, then Analyze image. The existing configured AI connection receives the source image, prompt, reference datetime/timezone, and project names/IDs. Image sharing requires separate consent. A blank image-model setting inherits the configured model.
- Review the image alongside selected task and meeting suggestions. Enlarge the image; edit each suggestion; assign different projects; select all/some/none. Unknown projects become editable proposals requiring explicit approval.
- Tasks expose title, notes, subtasks, deadline, priority, and estimate. Meetings expose title, start/timezone/offset, duration, link, agenda, notes, recurrence weekdays/end, and repeated-time policy. Existing defaults are shown; absent start/duration and unsupported recurrence require correction before selection can be created.
- Create selected commits the selected tasks, meetings, and approved projects together. Tasks start in To Do. Meetings appear in Signal's calendar through ordinary occurrence projections, without duplicate planner blocks. Created rows open the existing editors.
- Reanalysis and source replacement require confirmation when uncreated suggestions would be replaced. Already-created references remain authoritative. Unaccepted suggestions persist until Finish/Discard.

## Storage and boundaries

- Migration 0008 preserves legacy voice drafts and receipts, adds source-specific draft uniqueness, and stores source images as bounded SQLite blobs keyed by opaque IDs with dimensions/MIME metadata. This keeps source adoption/replacement/discard transactional and requires no scratch-file cleanup. No source image becomes a business-record attachment.
- Draft edits use revision checks and lost-reply reconciliation. Image acceptance has its own durable recovery identity; the shared workspace queue blocks competing writes until an uncertain acceptance is resolved.
- The shared agenda creation helper runs inside the intake transaction and existing agenda transactions. Existing native validators remain authoritative. Duplicate acknowledgment is required for normalized task titles within a project or normalized meeting title/start within a project, including matching recurrence occurrences.
- Source images, instructions, and uncreated suggestions are removed on Finish/Discard; created records and acceptance receipts remain. Voice state and personal data are preserved.
- Provider requests retain native protected credentials, HTTPS validation, no authenticated redirects, bounded responses, timeout, and cancellation. Image text is source material, never authority to bypass review or execute actions. Failed analysis preserves the local review.
- One image per review. No external calendar synchronization, invitations, text/PDF import, source archive, or new voice meeting extraction. Ordinary offline workflows and manual draft editing remain available.

## Verification

See [AI Inbox verification](../../verification/ai-inbox.md). Synthetic source: `tests/fixtures/intake/mixed-intake.png`; it contains two tasks and two meetings across two projects, including one meeting without a confirmed time. It contains no user data.
