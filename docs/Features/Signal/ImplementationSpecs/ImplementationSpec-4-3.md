# Implementation specification 4-3 — Meeting UI, agenda integration and verification

Execution update, September 13, 2026: this contract is implemented. Required native/manual gates remain pending; see [M4 verification](../../../verification/M4.md). `MeetingDetail.series` additionally returns `is_recurring` to protect former-series history after recurrence ends. The specification below retains its original planning language where it describes the M3 starting point.

Companion: [Task 4-3](../Tasks/Task-4-3.md). [D1–D8](../Phase-4-Decisions.md) approved September 13, 2026. Task 4-1 native/domain APIs and Task 4-2 presentation are prerequisites, not verified implementations at kickoff. Execution begins only when the phase execution prompt is invoked or implementation is explicitly authorized.

## Files and insertion points

Use Task 4-3 ownership. Do not modify another task's owned files concurrently.

- `db/client.ts::openFoundation`: after Database.load and optional loadSeed, initialize agenda metadata with the actual workspace timezone before business reads. Preserve existing initializeTimers/initialize_workspace ordering requirements; do not introduce another database pool or seed marker reset.
- `state/agenda.svelte.ts`: selected week, bounded Today/Week/meeting/task-linked reads, preview state, immutable write intent, pending/recovery/error state and query invalidation.
- `state/app.svelte.ts`: extend Editor, load/select/refresh/loadDetail/publish/mutate/delete and revision handling; own agenda actions and exact retry through the existing queue.
- `App.svelte`: mount Today/Week, meeting editors and DueDateDialog; extend currentEditor/closeEditor/navigate/openNew/created plus native exit handling.
- `MeetingsStrip`, `Board`, `YourDay`, `PlannerCanvas`, `PlannerBlock`: replace meeting placeholders with onOpenMeeting stable-ref callbacks. Preserve task pointer gestures, short-block lane packing and Join actions.
- `NewTaskDialog`: expose guarded Task/Meeting switching. `TaskDetailDialog`: render dedicated bounded occurrence reads and related-meeting navigation, never raw stale detail.meetings fallback.
- Meeting components own presentation/drafts only; AgendaState owns native boundary calls and Workspace owns writes.

## Prerequisite API and identity

Consume canonical Task 4-1 exports in `domain/agenda.ts` and `native/agenda.ts`; reconcile exact names before implementation.

```ts
type MeetingRef = { meetingId: string; occurrenceKey: string };
type AgendaQuery = { date: string; timeZone: string; projectId?: string };
```

MeetingOccurrence preserves compatible Meeting display fields and adds ref, revision, saved timezone, planner visibility, attendance, occurrence notes and owning project metadata. Stable refs must survive rescheduling and future-series changes; do not derive identity from the edited UTC start, title, array index or current cadence.

Use initializeAgenda, getToday, getWeek, getMeetingDetail, getTaskMeetings, searchAgendaTasks, previewMeetingChange and applyMeeting. `getTaskMeetings(taskId, AgendaQuery)` returns `TaskMeetingPage {task_id,week_start,range_start_utc,range_end_utc,occurrences,previous_date,next_date}` for the requested date's Monday–Sunday week. Render the range plus Previous/Next week controls in task detail; no unlimited expansion or raw fallback. Generic `TaskDetail.meetings` remains an empty deprecated compatibility field. `searchAgendaTasks({text,limit,cursor?})` uses 1–50 limit and returns `{items:AgendaTask[],nextCursor:string|null}`. Keep selected IDs across pages.

AgendaInput includes the durable `move_due` action with task ID, normalized dueAt or null, and expected task revision. This is required because the existing Workspace.patch path does not retain unknown-outcome task intents. Do not silently substitute bare patch plus blind resubmission.

`applyMeeting({requestId,change,expectedFingerprint?})` returns `{detail:MeetingDetail|null,outcome:{ref:MeetingRef|null,removed:boolean,task_id?:string},changed_details:TaskDetail[],affected_project_ids:string[],affected_task_ids:string[],revision:number|null,replayed:boolean}`. MeetingDetail contains `{occurrence,series:{repeat_rule,repeat_until,fold_policy,revision},linked_tasks:AgendaTask[]}`. Publish changed task details/revisions and explicit cancellation/removal outcomes before readback. Invalidate affected snapshots; after series mutation mark affected queries stale/unavailable if replacement content cannot load. A replay may return current detail null after subsequent deletion while preserving original outcome; never resurrect that original entity. A query alone never confirms an unknown write.

MeetingFields contains title, starts_at, start_local, start_offset, time_zone, duration_min, link_url, agenda_md, notes_md, reminder_min, show_in_day and task_ids. SeriesDraft adds repeat_rule, repeat_until and fold_policy. Preserve canonical local start and explicit offset alongside the UTC instant; native cross-validates all three against the saved IANA zone. Canonical MeetingChange actions are `create {projectId,draft:SeriesDraft}`, `edit_occurrence {ref,expectedRevision,draft:MeetingFields}`, `edit_following {ref,expectedRevision,draft:SeriesDraft}`, `record {ref,expectedRevision,attendance,occurrence_notes_md}`, `remove {ref,expectedRevision,scope:'one_off'|'occurrence'|'following'}`, and `move_due {taskId,dueAt,expectedRevision}`. Owning project is selected on create; this phase does not relocate existing series between projects. `previewMeetingChange(change)` is read-only; reviewed fingerprint is required for edit_following/remove.

## AgendaState and shared Workspace queue

AgendaState owns a single retained immutable intent with requestId, action, canonical payload, reviewed fingerprint/revisions and original query/ref. It does not own a second mutation queue.

Each read records query/ref identity, read ticket and mutation epoch. Accept success and error only when all remain current. Preserve known data during ordinary read failures with a visible stale/error state; suppress known canceled rows immediately after authoritative write outcomes. Old requests cannot overwrite a newer selected meeting or repopulate cancelled occurrences.

Reserve an intent before enqueue to block duplicate clicks. Inside Workspace queue, recheck agenda/planner/timer recovery constraints, call native apply, invalidate read epochs, publish committed outcome/details, then attempt refresh. A fallback failure reports Saved; refresh failed. It never returns the write to an unsaved state or resubmits it.

Typed native no-commit Validation/NotFound/Conflict/Database results unlock editing while retaining the draft and error. UnknownOutcome, commit uncertainty and unknown transport outcomes retain the exact intent; expose Retry and freeze replacement payloads. Exact Retry uses the original query/ref/payload/fingerprint even after route/date changes. Native receipt replay supplies the authoritative original outcome and current state. Do not infer these categories from text messages.

A conservative agenda recovery guard blocks all other business writes until resolution. Compose it with existing planner recovery and affected timer guards, including task/project deletion and due-date moves. Retrying agenda must bypass only its own guard, not unrelated pending recovery. Reads and safe navigation to recovery remain usable.

The existing `mutate(operation, plannerRetry)` bypass must become an explicit operation/recovery kind or equivalent checked structure. Do not add a blanket boolean that skips every guard. All successful task, timer, planner or meeting writes invalidate relevant agenda queries; all successful agenda changes invalidate Board/planner/detail-related queries. Timer Stop/Log affects Hours this week; task changes affect due/status buckets.

The Today route follows Workspace.selectedDate clock-today; the new AgendaState.today snapshot is query-keyed to that date. Week uses its own selected calendar date normalized to Monday by Task 4-1 helpers. Date changes invalidate old requests. Midnight moves Today and a following-current-week view, not a deliberately pinned historical week. A getToday reply can normalize query.date across a midnight race; refresh the shared clock and accept/reload only the matching effective-day ticket. Week uses its separate snapshot.today for styling while query.date remains pinned. Focus/restoration uses existing refresh/tick rather than another interval or clock.

## Today, Week and deadline integration

Mount Task 4-2 Today when active is today and Week for project view week. Retain Board/Week/Notes sub-bar and existing project selection; Notes remains a later-phase stub. Register task revisions from authoritative agenda snapshots before edits.

Shared callbacks:

```ts
onOpenTask(taskId: string): Promise<void>;
onOpenMeeting(ref: MeetingRef): Promise<void>;
onPlanTask(taskId: string): Promise<void>;
onJoin(url: string): Promise<void>;
onNavigateWeek(date: string): Promise<void>;
onMoveDue(taskId: string, destination:
  | {kind: 'date'; date: string}
  | {kind: 'none'}
  | {kind: 'picker'}): Promise<'committed' | 'cancelled'>;
```

Date moves use Task 4-1 pure deadline helpers. Existing local due time retains seconds and fractional precision. No date explicitly clears; Later opens a date picker. Missing due time shows editable 17:00. Gaps/repeated times open DueDateDialog with explicit correction/offset selection. Preserve the source timestamp and precision in draft state until an explicit user edit changes them. No drag failure changes status, timer, logged entries or planner blocks.

App mounts Task 4-2 DueDateDialog and awaits its committed/cancelled result. Its Save routes to durable AgendaInput.move_due. Unknown outcome keeps the editor/drag unresolved and Retry available; Escape/cancel cannot pretend the move committed. Direct unambiguous moves use the same receipt path. Week makes a drop opaque only on confirmed committed result; cancellation restores the source.

DueDateDialog controlled props are taskTitle, draft `{date,time,offset?}`, reason `move|no-deadline|gap|fold|later`, timeZone, pending, recoveryRequired, error; callbacks onChange(draft), onSave(draft):Promise<void>, onCancel():Promise<void>, onRetry():Promise<void>. App owns draft/native guard; expose millisecond input precision and keep the untouched source precision. Confirmed Save or receipt replay clears the editor; unknown recovery freezes replacement fields but leaves Retry enabled.

Plan from Today first completes close/navigation guards, sets planner.followToday=true, planner.date=Workspace.selectedDate and appropriate all/project scope, loads Your Day, awaits mount and calls `YourDay.planTask(existingTaskId)`. Verify the task exists in fresh planner data. No new task or due-date edit occurs. Change the existing generic Task created refresh error to context-appropriate wording for this path.

## New meeting editor and switching

Use 460px new-meeting and 760px detail tokens with the accepted 260px detail rail; both scroll within supported window sizes. Task/Meeting switches use existing requestClose/discard behavior. Never discard a task draft or staged attachment copies merely because Meeting was selected; discard uses the existing cleanup path only after confirmation. Reverse switching preserves the same guard rules.

NewMeetingDialog shows all D3 defaults: owning project, title, visible IANA timezone, next 15-minute local boundary, elapsed 30-minute duration, no recurrence, reminder 15 minutes, Show in Your Day ON. Duration chips 30/45/60 plus integer custom 1–1440; show end date/time for overnight meetings. Boundary rollover uses calendar helpers and the shared clock.

Support HTTP(S) join link, agenda and shared Markdown notes; no network title fetch. Use existing renderMarkdown sanitization and native commands.openExternalUrl for links. No direct innerHTML from user Markdown, arbitrary file URL or renderer navigation. Preserve valid notes text on validation error.

Recurrence controls: none/daily/weekly; Never/On date inclusive in saved timezone; visible Earlier/Later series fold choice default Earlier; show skipped spring-gap explanation. One-off and individually edited ambiguous starts require explicit offset choice; gaps are errors. Changing workspace timezone never reinterprets saved series timezone.

LinkedTaskPicker searches any existing task, including Done/cross-project, through the bounded native lookup. Show project name and status; track selected task IDs independently of search-page results. Do not drop a linked task because it is not on the current search page. Meeting color always belongs to its owning project.

## Meeting detail, preview and occurrence state

MeetingDetailDialog renders the selected stable occurrence, effective fields, linked tasks and series versus occurrence notes distinctly. Read failure shows Retry and does not fabricate an empty linked list or load a raw meeting row in its place.

Recurring edit/removal asks This occurrence or This and following. MeetingChangePreview calls native preview with normalized action and displays effective date/time, retained future overrides, affected scope, skipped gaps and deletion/cancellation consequences. Changing recurrence or shared defaults uses edit_following even when converting a one-off into a series; a plain individual edit uses edit_occurrence. For a not-yet-started selected override the reviewed draft applies to that selected occurrence, preserving separate notes/attendance; subsequent overridden occurrences are retained. If the selected actual occurrence is past, preview explains its historical payload is protected. Following removal that would touch actual past is rejected with guidance to cancel one occurrence or choose a later safe boundary. Preview does not write. Apply retains its fingerprint; stale rejection preserves input and requires refreshed preview/review, never an automatic changed-scope retry.

One-off Delete displays affected-record confirmation. Recurring removal writes exclusion/cutoff. Explain that linked tasks, task planner blocks and time entries remain. Project deletion keeps its approved stronger semantics; after success clear affected detail/picker snapshots and route safely away from removed project/meeting refs.

Attendance is an explicit Unmarked/Attended/Missed control for the occurrence only. Joining, ending, hiding or opening a meeting does not change it. Occurrence notes save separately from shared series notes/agenda and retain identity through rescheduling. Per-occurrence linked-task overrides are explicit; series changes preserve established future overrides as approved.

Show in Your Day modifies only planner display, automatic occupancy and planned totals. Today/Week/Board/task-linked views continue to show hidden occurrences. Every surface honors cancellations. Do not create planner meeting blocks. Notes are available by opening linked meeting detail; no general task activity feed.

## Editor lifecycle and native protection

Extend Workspace.Editor and App.currentEditor for new/detail meeting and due-date flows. Editors expose requestClose(reason):Promise<boolean>, await their in-flight action and protect dirty input. Keep one top-level modal routing authority. Navigating linked task→meeting or meeting→task first runs the current editor's guard and then opens the target; do not stack unsaved modal controllers.

App's native edit guard includes agenda drafts/pending/recovery in addition to existing editor/planner/timer state. After `workspace.settled()`, recheck all recovery flags before resolving native exit; a write may become unknown while awaiting the queue. No editor may clear a guard belonging to another operation. Unknown outcome refuses Close/Quit with enabled exact Retry. Persisted running/paused timers alone remain safe to close.

Retain source trigger focus and restore it when connected; fallback to the relevant view action. Guard Ctrl+K/new shortcuts while editors are active. Leave attachment drop routing specific to existing task editors. Do not add meeting attachments or make native file drops silently replace meeting notes.

## Automated verification

Use actual state controllers and deferred promises for:

- Date/project/ref changes while prior Today/Week/detail/task-linked reads resolve; late errors cannot replace the current state.
- Committed meeting and move_due replies published before refresh failure, including task revisions, cancelled-ref suppression and counts marked stale.
- Unknown create/edit/delete/attendance/move_due responses followed by exact request/payload replay, duplicate clicks and enabled Retry.
- Changed replacement intent blocked; typed no-commit errors retain editable drafts; stale preview requires review.
- Queued timer/task/planner/meeting operations respecting all recovery guards and project deletion invalidating cross-project links.
- Deadline seconds/fractions/DST/no-date defaults, cancel returning cancelled, Today Plan reusing the existing task on clock-today.
- Dirty Task↔Meeting switching, native Close/tray Quit, link navigation and staged-attachment preservation.
- Separate occurrence notes and attendance, owning-project colors and hidden-from-planner-only behavior.

Task 4-1 supplies real database recurrence/receipt/migration/range/deletion tests. Task 4-2 supplies component drag/keyboard/layout behavior. Do not substitute component mocks for native transactional evidence or reuse historical counts as current passes.

## Native verification, documentation and commit

Run Task 4-3 commands and compare #2d/#3d/#3b at 1600×960 and 1200×760. Use disposable fixture meetings/tasks; preserve existing edited fixtures and ordinary records. Record exact selected dates, IDs/refs, observations and failures.

| Native check | Required observable result |
| --- | --- |
| Ordinary upgrade | Legacy UTC starts and existing edited meeting fields remain unchanged; initial saved timezone is recorded once. |
| Repeated seed launch | Existing recurring metadata expands without duplicate seed rows or recreating canceled occurrences. |
| Today buckets | Earlier time today stays Due today; past date is Overdue; Done is absent from unfinished buckets. |
| Week movement | Source deadline seconds/fractions survive direct movement; explicit editor resolves absent/gap/fold deadlines. |
| Cancel movement | Escape or Cancel restores the source and makes no write; unknown transport remains recoverable. |
| Plan | Existing task opens in today's planner editor; task count and due_at do not change. |
| Cross-view occurrence | Today/Week/Board/Your Day open the same stable ref for the same occurrence. |
| Overnight meeting | Both intersected dates render appropriate portions while detail uses one stable occurrence. |
| Planner visibility | Hidden meeting disappears only from planner occupancy/display/totals. |
| Occurrence edit | Reschedule preserves attendance, occurrence notes and explicit linked-task overrides. |
| Following edit | Preview lists retained overrides; unchanged past instances keep original history. |
| Cancellation | This/Following cancellation survives refresh and restart without recurrence resurrection. |
| Joined meeting | Safe external Join opens; attendance remains unchanged until explicitly recorded. |
| Removal | Disposable one-off removal preserves linked tasks, planner task blocks and time entries. |
| Dirty protection | Task/Meeting switches, navigation, native Close and tray Quit retain or explicitly discard drafts. |
| Timer regression | Concurrent timers, block association and atomic Stop-and-complete still account correctly. |
| Native recovery | Open/Quit and both tray preference modes remain usable; uncertain writes retain Retry. |
| Local assets | Font loading remains local; offline checks keep the development loopback server running. |

Required click paths: Today→Plan→Your Day existing task; project Week→drag due task→Friday→task detail verifies preserved time; keyboard Move/No date/Later and cancellation; New task→Meeting switch with dirty draft; create meeting→link ATL-482 and a cross-project task→weekly recurrence→Today/Week/Board/Your Day identical occurrence; Join safe URL; hide planner only; This occurrence edit→attendance/notes→This and following preview retains override; cancel occurrence/following→restart confirms no resurrection; one-off removal preserves linked task/timer/block; task detail→meeting notes.

Exercise week/midnight/overnight and DST gap/fold scenarios with fixture inputs or injected clocks; never change the user's system clock. Verify ordinary and seed startup/migration repeatability, restart persistence, tray OFF/ON Open/Quit and dirty guards, M2 timer/header/manual Log, M3 planner/carry/Stop-and-complete, Board drag/attachments, local fonts and offline use while retaining the dev server. Required unexercisable checks remain pending with precise automation limitations until user confirmation.

Write docs/verification/M4.md with commands, actual counts, startup/visual evidence, migration/legacy timezone behavior, recurrence identity/limits, click paths, failed attempts and remaining M5/M6/M7/M8 stubs. Update README/PLAN and M4 metadata consistently, without overwriting another owner's contract.

Final commit after the gate: `feat(agenda): add Today Week and meeting management`. Inspect/stage only intended M4 work. No push without phase authorization. If required verification is blocked, finish independent work, report partial and defer completion commit. Stop for review before M5. Kickoff itself writes planning material only; execution is a separate authorized step.
