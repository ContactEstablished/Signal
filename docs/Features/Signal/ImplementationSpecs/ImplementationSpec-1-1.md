# Implementation specification 1-1 — Persistence and native operations

Status: complete; manual acceptance confirmed by the user on September 12, 2026. See [M1 verification](../../../verification/M1.md).

Companion: [Task 1-1](../Tasks/Task-1-1.md). [D1–D8](../Phase-1-Decisions.md) were approved September 12, 2026. All new APIs below are planned contracts, not existing M0 functions.

## Files and insertion points

Use Task 1-1's exact ownership list. In `db.rs`, make the existing `pool()` accessible as `pub(crate)` and extend `migrations()` with version 2; do not open another production connection. In `lib.rs`, register `workspace`, `attachments`, and `links`, their commands, and Rust dialog/opener plugins on the existing builder. Preserve `RuntimeState` selection, store/tray setup, native window restoration, and release seed rejection.

`workspace/mod.rs` provides command adapters; `models.rs` DTOs/errors/validation; `projects.rs`, `tasks.rs`, and `delete.rs` contain operations accepting the plugin pool and clock rather than requiring a WebView in their core logic. Tests call those same operations using a temporary SQLx pool. `attachments/mod.rs` owns all managed-file paths and lifecycle. `links.rs` owns http/https opening and rejects credentials/other protocols.

Keep `capabilities/default.json` unchanged: frontend SQL stays load/select only. Dialog/opener operations are exposed through narrow application commands using the Rust plugins, not broad frontend file/opener permissions. Native external-link helpers validate again even when the frontend already parsed the URL.

## Dependencies and compatibility

Task 1-1 installs and locks `marked`, `dompurify`, and `@js-temporal/polyfill`; add a supported DOM test environment as a dev dependency for sanitizer tests. Add Tauri 2 Rust dialog/opener plugins, UUID generation, UTC parsing/formatting, and a small OS file-lock dependency as needed by the implementations below. Resolve compatible versions against the installed toolchain at execution, commit lockfiles, and do not migrate the fixed stack or add a second persistence library.

Official references checked during kickoff: [Tauri dialog](https://v2.tauri.app/plugin/dialog/) supports Rust file selection; [Tauri opener](https://v2.tauri.app/plugin/opener/) provides opening integration. [Marked](https://marked.js.org/) requires a separate sanitization step. Task 1-3 prescribes the sanitizer policy; [Temporal ZonedDateTime](https://tc39.es/proposal-temporal/docs/zoneddatetime.html) supplies explicit timezone ambiguity handling. No packages are installed during kickoff.

## Migration 2

Create `0002_board_fields.sql` and register it after version 1. Preserve every original table/field/constraint/trigger. Add to `tasks`:

```sql
sort_order INTEGER NOT NULL DEFAULT 0
blocked_since TEXT NULL
revision INTEGER NOT NULL DEFAULT 0
```

Use additive ALTER statements with nonnegative checks for order/revision. Backfill dense order per `(project_id,status)` using `created_at,id` as deterministic tie-breakers; this retains the current fixture's visible card order. Leave historical `blocked_since` NULL: migration must not invent when an existing task became blocked. Add an index on `(project_id,status,sort_order,id)`.

Fresh startup runs migrations before `seed_database`, so migration backfill alone does not order newly inserted fixture tasks. In the existing successful first-seed transaction in `db.rs`, normalize only the just-inserted fixture tasks with the same `(created_at,id)` rule before committing the M0 marker. Do not normalize on marker-hit reloads, reset revisions, or rewrite original seed fields. Retain the version-1-only fixture shape so M0 tests remain valid. Verify both sequences: M0 seed then migration 2, and migrations 1+2 then first seed; each must produce identical dense initial orders and preserve later manual moves on reload.

Add a technical attachment lifecycle table, `attachment_file_ops(token TEXT PRIMARY KEY, relative_path TEXT UNIQUE NOT NULL, filename TEXT NOT NULL, size INTEGER NOT NULL, mime TEXT NOT NULL, state TEXT NOT NULL, created_at TEXT NOT NULL, last_error TEXT)`, with state constrained to `staging|ready|delete_pending`, nonnegative size, and a state index. This supports D1/D8 file recovery, not timer/attendance/notification state. Version 2 therefore retains the 13 handoff tables and adds one internal file-operation table. M0's version-1-only test still expects exactly 13 tables; the new upgrade test checks both versions separately.

## Shared DTOs and commands

`domain/types.ts` mirrors snake_case database fields and exports `Id=string`, `TaskStatus`, `Priority`, and `ProjectColor` (four approved project colors). `ProjectRecord` includes all project fields. `TaskRecord` includes all task fields plus the three migration-2 fields; `hours_worked` is output-only. `BoardTask` adds tags and `subtask_done/subtask_total`. `BoardSnapshot={project,tasks,meetings,tags}`. `TaskDetail={task,project,subtasks,tags,attachments,alerts,meetings}`. A `StagedAttachment` exposes token, filename, size, and mime, never an arbitrary final destination.

`CreateTaskInput` accepts project_id, title, status, priority, due_at, estimate_h, external_url/provider/id, blocked_reason/on, notes_md, ordered subtasks, tags, alert offsets, and staged attachment tokens. Omitted child collections default to empty except the UI's explicit default alerts. `TaskPatch` is a whitelist of editable scalar fields, excluding project_id, IDs, hours, timestamps, order, revision, and blocked_since. Include status and external metadata as atomic groups. `SubtaskInput={id?:Id,title,done}` uses array order. `TagInput={id:Id}|{name:string,color:string}` refers to existing tags or creates/reuses a normalized tag; never silently edit a global tag from one task.

Publish these wrappers in `native/commands.ts`; Rust command names are their snake_case equivalents:

| API | Result/behavior |
| --- | --- |
| `listProjects()` | Ordered `ProjectRecord[]`. |
| `getBoard(projectId,dayStartUtc,dayEndUtc)` | Full project tasks, tags, and meetings intersecting the half-open day interval; no recurrence expansion. |
| `getTaskDetail(taskId)` | Authoritative detail or NotFound. |
| `createProject({name,color})`, `updateProject(id,{name,color})` | ProjectRecord; preserve existing manager/summary settings. |
| `moveProject(id,'left'|'right')` | Ordered ProjectRecord[]; boundary move is a no-op. |
| `createTask(input)` | TaskDetail; native generates opaque UUID and timestamps. |
| `updateTask(id,patch,expectedRevision)` | TaskDetail after atomic scalar update. |
| `moveTask(id,status,beforeTaskId,expectedRevision)` | TaskDetail after full-column order/status update; null target appends. |
| `setSubtasks(id,inputs,expectedRevision)` | TaskDetail; validate ownership of retained subtask IDs, replace/reorder atomically. |
| `setTaskTags(id,inputs,expectedRevision)` | TaskDetail; trim names, case-insensitive reuse, deduplicate relations; retain unrelated tags. |
| `setTaskAlerts(id,offsets,expectedRevision)` | TaskDetail; unique nonnegative integer offsets. |
| `previewDeletion({kind:'task'|'project',id})` | Affected counts and opaque fingerprint of the exact current graph. |
| `deleteEntity(target,fingerprint)` | `{deleted:true,cleanupPending:boolean}` or changed-preview Conflict; no mutation on mismatch. |
| `stageAttachments(paths?)` | StagedAttachment[]; omitted paths opens native picker, cancellation returns []. Paths supplied only from native drop events. |
| `discardStagedAttachments(tokens)` | `{cleanupPending:boolean}`; mark owned staging operations delete_pending and run cleanup. |
| `addAttachments(taskId,tokens,expectedRevision)` | TaskDetail; adopt all ready tokens atomically. |
| `removeAttachment(id,expectedRevision)` | `{detail:TaskDetail,cleanupPending:boolean}`; remove metadata and enqueue copy deletion atomically. |
| `openAttachment(id)`, `openTaskLink(taskId)`, `openExternalUrl(url)` | Void or typed failure; resolve authoritative paths/URLs in Rust. |
| `installFixtureAttachments(fixtures)` | Debug seed-only idempotent addition described below. |
| `setEditGuard(active)` | Acknowledge native registration of an open M1 editor before input is enabled. |
| `resolveExitRequest(requestId,proceed)` | Resolve a native Close/Quit request after the editor's save/discard flow; reject stale IDs. |

Reject unknown JSON fields where appropriate. Use a serializable `AppError={code,message,field?:string}` with Validation, NotFound, Conflict, Database, and File codes. File cleanup pending after a committed deletion is a committed success with a visible warning, never a false rollback claim. Do not return success before the transaction commits. `db/client.ts` retains `openFoundation()`/`setTrayPreference()` exports; expand project output compatibly, without routing business writes through SQL execute.

## Transactions, clocks, and ordering

Production instants use native UTC; fixture mutations use the same fixed instant as `FIXTURE_NOW`. Validate/normalize supplied instants to canonical UTC ISO with milliseconds. Read-only UI clock selection belongs to Task 1-4; it must agree with native fixture time. Do not accept a frontend database name or arbitrary timestamp to select a mutation target.

Each task write begins a transaction, checks `expectedRevision`, performs all related changes, increments revision once for a real change, updates `updated_at`, and returns committed detail. A mismatch must leave all data untouched. Full task child-list writes cannot replace another task's IDs. Task 1-4 also serializes UI writes, supplying the latest revision when each queued intent executes; revision checks remain authoritative against stale readers.

Construct the authoritative reply using reads inside the write transaction, then commit, then return that prepared reply. A read failure before commit rolls back; do not perform a fallible detail read after commit and report it as a rejected mutation. Optional post-commit filesystem cleanup instead contributes `cleanupPending` to its success envelope. Pure read commands assemble multi-query Board/detail results within one read transaction to avoid mixing revisions across child collections.

Status transitions: entering Done sets `done_at` to now; leaving Done clears it. Entering Blocked sets `blocked_since`; leaving clears it. Re-selecting the current status preserves transition timestamps, including NULL unknown history. Keep reason/person values when leaving Blocked so later re-entry can edit them; only display them as blockers while blocked. Native code never accepts an independent hours total.

For a move, load complete source/destination column ID order, remove the moving ID, insert before a validated same-project destination ID or append, and write dense zero-based order for affected columns in one transaction. Increment revisions for tasks whose persisted order changes so stale readers cannot overwrite them. Preserve all hidden relative ordering. Self-target/no positional change is a no-op. Status changes through detail controls append to the destination. New tasks append. No timer/block completion follows task status changes.

Project ordering similarly swaps neighbors and normalizes dense order in one transaction. Project name must be trimmed/nonblank; color is one of the four defaults. Estimates accept NULL or finite nonnegative numbers, including zero. External metadata is either recognized provider fields with a safe URL, a generic safe URL with null provider/ID, or absent; preserve existing fixture external IDs that have no URL when unrelated fields are edited.

## Deletion graph and confirmation

Preview returns counts for tasks, subtasks, relations, attachments, meetings, blocks, time entries, alerts, and summaries actually affected. Include stable ordered record IDs, revisions/values, and related file metadata in the fingerprint; counts alone do not detect a changed graph. Recompute under the writer transaction after confirmation and reject differences before deleting anything.

For task deletion, collect its blocks and files; remove its time entries first so cache triggers still see the task. Set surviving blocks' `carried_from_block_id` to NULL when they reference a removed block. Remove task meeting-links, alerts, tags-links, subtasks, attachment metadata, and task blocks, then the task. Retain meetings, global tags, unrelated records, and neutral blocks. If a surviving time entry references a removed block, detach its block_id before removal while preserving that entry.

For project deletion, union those task graphs with project meetings and summaries. Remove links for deleted meetings even when their tasks survive in another project; preserve those other tasks. Remove meetings/summaries before the project. Normalize surviving project order. Enqueue every owned attachment copy in the same SQL transaction as metadata removal. Use explicit operations with foreign keys ON, not blanket cascade/disabled constraints. Any database failure rolls the entire graph and queue back.

## Attachment lifecycle and recovery

Select managed roots beneath application data by native RuntimeState: `attachments/` for ordinary and `attachments-dev/` for fixture. Use generated opaque relative filenames; retain user-facing original filenames only as metadata. Acquire an OS exclusive lock for the selected database's managed-file root for the process lifetime before recovery; fail clearly if another process already owns that same database/root. Separate ordinary/fixture roots may have separate locks. Tests use injected temporary roots.

Staging allocates a token and durable `staging` row before copying into the root. Accept regular local files; reject directories, traversal, and source/destination aliasing. Stream copies without loading entire user files into JS memory. Determine actual size after successful copy and transition the row to ready; failed copies become delete_pending with a readable error. Never return a token for an incomplete copy. No product file-size cap is introduced by this plan.

A multi-file staging request publishes descriptors only when every copy succeeds. If any copy fails, queue all operations created by that request for cleanup and return a File error identifying the failed file and any pending cleanup; never strand successful unreturned tokens as ready. Previously staged files from other requests remain available. Cancellation of the picker returns an empty list without changing the draft. Serialize staging/adoption/discard/cleanup within the process so cleanup cannot race an in-flight copy; the OS root lock separately excludes another process.

Creation/addition adopts ready tokens by inserting attachment rows pointing at those owned files and deleting their operation rows in the same business transaction. A failed task write leaves ready tokens available for retry; confirmed draft discard converts them to delete_pending. On restart, staging/ready operations belong to abandoned nonpersistent drafts and become delete_pending. Cleanup processes only queued paths inside the correct root and not referenced by committed attachments, records failures, and retries on next startup/attachment operation. Thus a crash between copy and DB adoption leaves a tracked orphan, never a fabricated attachment row.

Removal commits metadata deletion plus delete_pending first, then attempts filesystem removal. Missing owned files count as already cleaned. Access-denied failures remain queued and reported. Never delete source paths. Canonicalize existing targets and validate resolved parents for not-yet-existing files; reject symlink/reparse escapes. Opening uses attachment ID → database metadata → confined existing regular file → OS opener; absence is a File error. An attachment copy shares no path with another attachment.

`installFixtureAttachments` accepts exactly the two approved synthetic assets as bytes, filenames, MIME, and stable fixture IDs supplied by Task 1-4. Native guards require debug build, seed RuntimeState, correct fixture marker, and the existing ATL-482 fixture task. Use normal staging/adoption operations and an additional M1 fixture marker committed with both attachments. Repeat calls with the marker preserve user removals/edits; do not change the existing M0 seed version or repopulate other records. Enforce an internal bound suitable for these two small known fixtures, separate from user-file handling. No release file installation path is available.

## Native editor exit guard

`exit_guard.rs` manages an active-editor flag and at most one pending `{requestId,intent:'close'|'quit'}`. Register it in lib.rs. Task 1-4 awaits setEditGuard(true) before exposing an editor, and sets false only after its clean/confirmed close and pending writes settle. Registration failure blocks editing with a visible error rather than claiming protection.

In window.rs, intercept CloseRequested before the existing save/hide handling when this guard is active: prevent close and emit `signal://exit-request` with ID/intent to the main window. In tray.rs, intercept the Quit branch similarly before app.exit. Repeated requests reuse the outstanding request; make the main window visible/focused for the decision. No active editor follows the unchanged M0 path.

resolveExitRequest(false) clears the request but keeps the editor guard. After successful save or confirmed discard, resolveExitRequest(true) clears guard/request and performs the original intent: window.close() to reuse M0 save/hide behavior, or save geometry then app.exit(0) for Quit. Avoid recursion by clearing state before reissuing the native action. Never rely on a late asynchronous frontend listener to stop a process that Rust has already exited. Test stale IDs, repeated close, cancel, save failure, and both tray modes. Forced OS termination remains outside in-memory draft recovery.

## Verification and handoff

`m1-migration.test.ts` applies migration 1, loads existing seed rows, then applies 2; compare every old field, counts, sums, FK integrity, deterministic task order, NULL blocked history, and zero initial revisions. Also verify fresh 1→2, and SQL rejection of invalid new values.

Native tests must additionally call the actual first-seed transaction after migration 2 and prove dense ordering, marker-hit preservation of a manual move, and rollback of both seed records and marker on failure. Keep this assertion out of the unchanged M0 test helper, which intentionally executes only migration 1. Test transaction reply preparation failures, multi-file partial-copy compensation, and committed attachment removal with cleanup pending.

Rust workspace tests cover the real project/task functions, revision conflicts, same-status preservation, fractional time-cache invariants, filtered move intents/full order, and deletion rollback/stale previews across project links and carried-block references. Attachment tests use temporary real files and injected failures: partial copy, abandoned stage, adoption rollback, missing/locked file, cleanup retry, duplicate/invalid token, traversal/reparse escape, and ordinary/fixture separation. Link tests reject unsafe protocols/credentials before opening (use an injected opener so tests do not launch applications).

Run the companion commands; record actual counts/results. After integration, test real picker/drop/open/removal, kill/restart recovery only on disposable fixture records, and an M0-to-M1 fixture upgrade without reseeding tasks. Hand contracts and evidence to Task 1-4; it owns final runtime results and the conventional M1 commit. No tests or migrations have been executed as part of writing this specification.
