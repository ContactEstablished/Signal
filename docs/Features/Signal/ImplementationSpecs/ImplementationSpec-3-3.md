# Implementation specification 3-3 — Your Day integration and verification

Companion [Task 3-3](../Tasks/Task-3-3.md). D1–D8 approved September 13, 2026. APIs in [Spec 3-1](ImplementationSpec-3-1.md)/[Spec 3-2](ImplementationSpec-3-2.md) are prerequisite contracts; no execution result asserted.

## Files and data flow

Use task ownership. New PlannerState owns view/read/recovery state, not a write queue. Workspace modifies load/select/refresh/publish/mutate/delete/timeAction/revision guards. TimerState threads optional blockId through retained Start inputs. App mounts YourDay at active day and composes close/navigation/new-task paths. YourDay orchestrates components without native calls. Preserve shell/fonts/tokens and Board overflow fix.

Consume getPlanner(query),applyPlanner(input), shared DTO/domain rules. All-project snapshot has blocks/previous_blocks/copy_sources/tasks/meetings/entries/claims/dismissed/fingerprint/timers. Planner reply adds changed_details plus **changed_entries keyed by task ID with complete histories**, not only selected-date entries, so Stop+complete can publish all M2 accounting before any failing refresh.

## PlannerState and queue

Keep planner date separate from Workspace.selectedDate (clock-today for Board/badge). Initialize current date; date arrows/picker pin, Today follows clock-today. Midnight updates a following view, not a pinned past/future date. Scope all/project filters display/stats locally; deleted scoped project falls back to All; neutral blocks remain visible.

Read captures query identity,ticket,mutation epoch and accepts only if still current. Queue every write on existing Workspace.mutate/enqueue boundary; retain immutable input before invoke. Confirmed reply invalidates stale reads and publishes planner/timer/accounting state before fallback refresh; refresh failure says Saved, never resubmits. Accounting publishes globally, dated content only to its matching current query.

Keep one unresolved planner intent with requestId/payload/query. Typed native no-commit errors unlock mutation while preserving draft/error. Unknown transport outcomes retain intent and require exact Retry even after date/scope changes. Reads cannot clear recovery. Broad planner recovery blocks conflicting planner/task/project/timer writes and deletion; Retry stays enabled. Check existing M2 unknown guards for affected tasks too. Do not infer success from a snapshot or error text.

Stop+complete publishes changed details/revisions through existing Workspace.publish, complete changed task histories into TimerState, timer sessions and block before refresh. Independent Move task to Done uses Workspace.patch with the newly published revision. Existing M2 unknown intent/payload must remain unaffected.

## Date/sidebar/planning workflows

Sub-bar has date navigation/Today/scope, summary placeholder and New task. Layout `minmax(0,1fr) 320px`,20px gap. Use fixed planner token inventory and local Sora 600 hour labels; don't announce seconds live.

Sidebar applies approved domain helpers for Quick add 30/60/60, Unscheduled, Due soon, leftovers/done and totals. Scope changes candidates/display, never native all-project occupancy. Past automatic actions disabled with explanation; manual editing still validates normally. Selected-date previous-day leftovers retain original lengths/source, Dismiss only banner, Pick explicit selection, Carry all deterministic order.

Quick add submits a single valid next-free proposal immediately through Workspace, blocking duplicate clicks until settlement; no-space is a non-writing message, and unresolved DST endpoints open explicit review. Done today uses selected-date completed blocks and their planned lengths, filtered by scope, with neutral blocks retained; do not infer meeting attendance. Empty scoped days expose the grid hint and due/copy preview actions with actual due/overdue candidate counts.

PlannerPreview renders proposals/skips/no-space/conflicts/provenance/selected date and explicit endpoint offset choices. Due/Copy always require Apply. Pass reviewed fingerprint; stale rejection needs refresh/review, no silent relocation. Copy uses previous Monday strictly before selected date and own provenance, never carry links or meeting copies.

## Block editing and lifecycle

BlockEditor is the keyboard alternative for creation, movement, resizing, and offset selection. Reject gaps. Repeated endpoints require an explicit offset; 24:00 means the next midnight. Persist choices. A timezone change preserves wall minutes and requires fresh validation on edit. Automatic previews can require per-row offset choices before Apply.

Approved editor refinement: keep typed Start/End fields and add bounded ±15-minute controls, 30/60/90/120-minute duration shortcuts, and a date picker immediately beneath the times. Initialize a local selected date and use it for endpoint validation; date/time changes clear offset selections. `onSave(draft, date)` passes the selected destination to YourDay. Keep the original planner query for create/move and include `destinationDate` in the payload. After confirmed success, clear the editor and load the destination day; exact Retry uses `outcome.destination_date` to do the same. A read failure after commit must not repeat the write. Preserve draft/date on save failure. Moving a block never changes its task due date or existing timer/history associations.

Start supplies optional blockId; an existing task timer retains its association. Only the associated running block gets the active ring and run-over state past its resolved end. Completing its live session asks Stop and complete or Cancel; the native operation is atomic. Sessions associated elsewhere are untouched. Reopening a block does not restart a timer or reverse history. Removal warns and detaches the block while preserving the task, logged time, and live session identity. Moving the task to Done is a separate optional offer.

Capture the created TaskDetail in App's existing onCreate wrapper. onCreated(plan) selects Your Day and prepares scheduling for that exact task. Creation and scheduling are separate transactions; cancel, no-space, or failure leaves the task available and cannot reissue creation. New task from Your Day uses the project scope or first project with the existing picker.

## Native editor protection and focus

Compose planner dirty, pending, and recovery state with App's closeEditor and native edit guard. One editor cannot clear the guard while another has work. Pending exits await the queue; an unknown outcome refuses closure until Retry. Unsaved block edits offer Discard or Keep editing. Persisted running or paused timers alone do not block close. Preserve attachments, shortcuts, header discovery, and focus restoration. Canceling a provisional gesture does not delete a record. Block removal requires its explicit confirmation.

## Tests, runtime gate, and handoff

Test deferred reads, exact retry after a date change, committed publication before a failed refresh, queued revision updates, native guard composition, scope versus occupancy, eligibility and totals, stale or DST previews, and preservation of a newly created task.

Compare the native window with #2a/#2b/#3e at 1200×760 and 1600×960. Verify 07:00 initial scroll and midnight reachability; a 09:00–10:30 selection and keyboard picker cancellation; task and neutral creation; movement, resizing, and autoscroll; short painted overlaps and three/four-plus lanes; Quick add with hidden scope and no space; Carry, Pick, Dismiss, and repeated requests; Due and Copy previews; associated, concurrent, and paused timers; Stop and complete, cancellation, and optional task completion; removal preserving history; restart; DST gaps and folds without changing the OS clock; dirty/native close and tray behavior; M2 header, Board, and attachments; ordinary/seed isolation; local fonts and offline startup.

Record actual commands, results, dates, record IDs, click paths, screenshots, and limitations in docs/verification/M3.md. Preserve the edited fixture; its advancing date can differ from the static mock. Update README, PLAN, and M3 metadata consistently. Commit only after the gate: `feat(planner): add daily scheduling overlap lanes and carry-over`. Do not push without authorization. If required verification is blocked, finish independent work and report partial. Stop before M4.
