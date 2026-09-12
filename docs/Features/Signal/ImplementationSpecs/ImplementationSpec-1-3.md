# Implementation specification 1-3 — Task creation and detail editors

Status: complete; manual acceptance confirmed by the user on September 12, 2026. See [M1 verification](../../../verification/M1.md).

Companion: [Task 1-3](../Tasks/Task-1-3.md). [D1–D8](../Phase-1-Decisions.md) approved September 12, 2026. New command/type contracts come from [specification 1-1](ImplementationSpec-1-1.md); they are not implemented M0 APIs.

## Files and integration points

Create only the eight task components, four domain helpers, and four test files enumerated in Task 1-3. Import shared records/inputs from planned `domain/types.ts`. Consume existing `domain/dates.ts` without modifying it. Task 1-1 owns dependencies including marked, DOMPurify, Temporal, and a DOM test environment.

NewTaskDialog and TaskDetailDialog own native `<dialog>` presentation, focus trapping, local confirmation state, and close-event interception; do not import a shared Modal/ConfirmDialog. Task 1-4 owns App.svelte mounting, single active-editor routing, shared revision-aware mutation queues, native drop subscription, and cross-view refresh. Emit close requests/completion callbacks rather than installing competing global shortcut handlers.

## Planned controller contract

NewTaskDialog receives projects/defaultProjectId, nowUtc/timeZone, staged attachment descriptors, and callbacks `onCreate(input,plan)`, `onStage(paths?)`, `onDiscardStaged(tokens)`, `onClose()`. TaskDetailDialog receives authoritative TaskDetail plus nowUtc/timeZone and callbacks `onPatch(id,patch)`, `onSubtasks(id,inputs)`, `onTags(id,inputs)`, `onAlerts(id,offsets)`, `onAttach(id,tokens)`, `onRemoveAttachment(attachmentId)`, `onOpenAttachment(id)`, `onOpenTaskLink(id)`, `onOpenExternalUrl(url)`, `onPreviewDeletion(target)`, `onDeleteEntity(target,fingerprint)`, `onClose()`, and `onPreview(message)`.

The controller adds the current task's expectedRevision when executing update/child/attachment mutations, following specification 1-1; successful replies are TaskDetail with updated revision. Conflicts trigger a reread while retaining the dirty buffer and offering reapply/discard. Never reapply automatically against changed state. Deletion uses a fresh fingerprint, not a revision alone. A readback error after commit is distinct from mutation rejection, preventing duplicate submissions on retry.

Both task dialogs also receive `stagedAttachments:StagedAttachment[]`, `onStage(paths?):Promise<StagedAttachment[]>`, `onDiscardStaged(tokens):Promise<{cleanupPending:boolean}>`, `onOpenExternalUrl(url)`, and `onPreview(message)`. These callbacks support detail Add/drop retry as well as creation and Markdown preview links. Attachment removal is the explicit exception to the bare-detail reply: `onRemoveAttachment` returns `{detail,cleanupPending}`. Publish that detail and show pending cleanup as a warning; do not retry committed metadata deletion.

Both task dialogs export `requestClose(reason:'dialog'|'navigation'|'native-close'|'native-quit'):Promise<boolean>`. Resolve true only after all in-flight saves/staging settle and the draft is clean, successfully submitted, or explicitly discarded. Resolve false for cancellation or save/discard failure. Deduplicate simultaneous close requests; leave the dialog mounted while deciding. The parent alone unmounts/routes/clears the native guard after true. Dialog Cancel/Escape/backdrop requests enter this same controller flow through `onClose()`.

For new tasks, retain D5's Create/Cancel semantics: changed-draft closure offers Discard/Keep editing; never implicitly create a task to satisfy a generic Save-on-close action. Details may offer Save/Discard/Keep editing for uncommitted fields. Save completes only the pending edits, not a new task creation or deletion. Native exit uses the same mode-specific choices.

CreateTaskInput contains editable scalars, ordered subtasks, tags, alerts, and staging tokens. Native code generates UUID/timestamps/order and derives hours. Existing task project assignment remains breadcrumb/read-only in M1; the creation project picker selects the target. TaskDetail carries all linked meetings; the rail shows actual summaries with an M4 preview action.

## Draft normalization and saving

`task-draft.ts` exports creation defaults, validation, submission normalization, and dirty comparison against the initial normalized draft. Default Backlog/Medium, blank notes/collections, null due/estimate, and visible day/hour alert choices selected (`1440,60`). Stored offsets can remain without a due instant; no reminder delivery is promised. Trim title and reject blank; blank estimate is NULL, not zero; reject negative/nonfinite values. Native validation repeats these constraints.

Creation keeps one draft until success or confirmed discard. Disable duplicate submission while pending. After successful creation, clear consumed staging tokens and emit completion/navigation exactly once. On rejection retain all draft values and ready tokens. Native Confirm-discard is an in-dialog state: Escape/backdrop/Cancel requests it for changed fields or staged files; confirmation releases staging tokens before closing, and cleanup failures show a recoverable warning.

Detail text fields commit on blur/Enter, selectors on selection, and notes through explicit Edit→Save/Cancel. Suppress blur after Enter to avoid sending the same mutation twice. Keep a baseline and attempted buffer per active field. Escape within an active field explicitly cancels only that field edit and consumes the event; it does not also close the dialog. Dialog-level Escape enters the normal dirty/failed close flow. Close while dirty/failed asks Save/Discard/Keep editing; wait for in-flight writes before final closure. Reloads never overwrite dirty buffers. Notes cancel restores persisted notes without affecting already saved fields.

Subtask lists add/edit/check/remove/reorder while retaining IDs, sending full ordered lists atomically. Tags submit existing IDs or new normalized name/color input; no global tag rename occurs here. Alerts submit unique offsets, including day/hour choices and an Add control for a nonnegative custom minute offset. Status and blocked reason/person use native transition rules; unknown historical blocked-since displays as unknown, not a fabricated date.

## Links and sanitized Markdown

`links.ts` parses URL-only or a complete Markdown link `[title](URL)` using URL parsing and http/https-only validation; reject credentials and malformed URLs. Recognize anchored Jira `/browse/KEY-number`, Asana `app.asana.com/0/project/task` and `/1/.../task/...`, and ClickUp `app.clickup.com/t/taskId` or workspace-prefixed `/workspace/t/taskId`. Asana/ClickUp hostnames must match exactly; self-hosted Jira permits a valid hostname with the explicit browse path, without asserting a network-verified provider identity.

Return recognized metadata, a generic safe-link result, or validation error. Unsupported safe URLs can remain without provider/ID. URL-only never fetches or fabricates a title. Markdown title prefills only an empty title. Unrelated field edits must preserve legacy fixture external IDs without URLs. Providerless display is Local task. Clicking an external task link uses `openTaskLink(taskId)`; rendered note links use `openExternalUrl(url)`, validated again natively.

`markdown.ts` owns one parse-then-sanitize function used by notes display/preview: marked output → DOMPurify allowlist → render. Allow paragraphs/headings/lists/blockquote/emphasis/code/pre/links and appropriate harmless attributes. Strip scripts/styles/frames/forms, handlers, images (local or remote), and unsafe URL schemes. Do not modify sanitized HTML later in a way that bypasses that policy. Intercept allowed anchor activation and send it to the native opener; never navigate the WebView. No untrusted source goes directly into `{@html}`. Test the actual DOMPurify configuration in the installed test DOM, not only string replacement.

## Deadline conversion

`deadlines.ts` uses the Task 1-1 Temporal dependency and explicit IANA timezone. Newly selecting a date shows editable 17:00; clearing produces NULL. Preserve untouched `due_at` byte-for-byte. Reject partial/invalid wall dates/times. Convert a valid confirmed local date/time to canonical UTC with milliseconds, never use the host timezone implicitly for the New York fixture.

Use `Temporal.ZonedDateTime.from(fields,{disambiguation:'reject'})`. If it rejects, compare earlier/later candidates: if neither preserves the requested wall fields, show a nonexistent-time error; if both preserve them with different offsets, show the two offset choices and require selection. Do not silently move spring-forward input or pick one fall-back occurrence. Retain an existing instant's offset choice when editing an otherwise unchanged ambiguous deadline. Ordinary clock/zone comes from Task 1-4; fixture zone is America/New_York.

## Layout, interaction, and attachments

New dialog matches #3a: 600px, project picker and Task/Meeting segment, link first, 20px Sora title, status/priority/estimate, due/alerts, tags, subtasks, notes/drop area, footer hints/actions. Meeting selection shows an M4 preview while retaining the task draft. Handle Ctrl/Meta+Enter and +Shift only inside this dialog; the Shift action creates then requests the Your Day preview. Never intercept ordinary multiline typing globally.

Detail matches #1b: 820px, `1fr 280px`, 22px Sora title, breadcrumb/external chip, lime subtask progress, notes/attachments, right rail status/priority/due/time/tags/meetings, created date/close hint. Constrain height and scroll content so close/footer stay reachable at 1200×760. Reuse tokens, local fonts, Lucide, 120ms transitions, reduced-motion handling, and visible focus/error states.

TaskTimeCard is read-only derived hours/estimate, including overrun, NULL/zero estimates without division errors. Start/Log visibly report M2 preview. AttachmentList shows actual filename/size/MIME and pending/errors. The native picker returns staging descriptors, cancellation is neutral, and native drop paths arrive only through Task 1-4's active-editor routing. Never derive a filesystem path from browser File.name. Staged files join task creation, existing-task Add adopts tokens after staging, and successful Remove deletes only the managed copy. Missing/open/copy/remove errors remain readable.

Show a full-dialog drop affordance: files may be dropped anywhere within the active task dialog, including outside Notes. A failed adoption retains ready tokens with Retry/Remove controls; unadopted tokens and in-flight staging count as pending draft work. Confirmed discard releases them exactly once. Durable discard with `cleanupPending` permits closure with a warning; a database failure retains tokens for retry. A failed multi-file staging batch adds no descriptors to the draft.

Task deletion opens an in-dialog permanent-deletion confirmation showing fresh affected counts. A changed fingerprint requires a new preview and confirmation. After commit the parent closes the deleted task and refreshes Board/badge; pending file cleanup is a warning, not a failed deletion. Failure retains the dialog/error. Focus returns to the invoking card if it survives, otherwise to a useful Board control.

## Verification and handoff

Tests use actual helpers: supported/hostile/link-lookalike cases, retained titles, unsafe Markdown/schemes/remote images, blank/zero/negative estimates, dirty comparison, repeated submit suppression, leap/invalid dates, UTC preservation, spring gap and fall ambiguity. Add controller/runtime coverage for revision conflicts, blur/Enter duplicate saves, queue failures and pending close in Task 1-4.

Run the companion commands. After integration verify ATL-482 edits/reopen, child fields, sanitized notes, real file add/drop/open/remove, dirty/failed close, task deletion on disposable records, keyboard creation and honest create-and-plan navigation. Restart verifies persistence; mocks do not. Compare #1b/#3a at both supported sizes. Hand actual evidence and remaining checks to Task 1-4, which owns final verification and milestone commit. No M2 behavior is implemented here.
