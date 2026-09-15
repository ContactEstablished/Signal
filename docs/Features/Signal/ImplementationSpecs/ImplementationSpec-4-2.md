# Implementation specification 4-2 — Today and Week presentation

Execution update, September 13, 2026: this contract is implemented. Required native/manual gates remain pending; see [M4 verification](../../../verification/M4.md). `MeetingDetail.series` additionally returns `is_recurring` to protect former-series history after recurrence ends. The specification below retains its original planning language where it describes the M3 starting point.

Status: implemented and audited; required native/manual acceptance remains pending. See [M4 verification](../../../verification/M4.md). Companion [Task 4-2](../Tasks/Task-4-2.md); [D1–D8](../Phase-4-Decisions.md) approved September 13, 2026. [Spec 4-1](ImplementationSpec-4-1.md) owns authoritative snapshots/calendar helpers; [Spec 4-3](ImplementationSpec-4-3.md) owns integration, meeting editors and persistence coordination.

## Exact files and insertion points

Create only Task 4-2's nine listed files. Today/Week are new views; Task 4-3 replaces App's existing placeholders with them. AgendaTaskCard owns task row/card presentation, MeetingPill owns occurrence display/navigation, WeekStrip owns seven day panels, and DueDateDialog owns the controlled deadline form. Shared domain DTOs/helpers are created by Task 4-1. No component imports Workspace, native APIs or an independently sampled clock. Any necessary semantic token additions belong to Task 4-3.

Existing Board card pointer UX is a reference for translucent destination/faded source behavior, not permission to change Board ownership or reuse status mutation callbacks. Existing minute-only deadlineFields must not initialize a Week move.

## Planned props, callbacks and data flow

Import AgendaTask, AgendaDay, TodaySnapshot, WeekSnapshot and MeetingRef/MeetingOccurrence from domain/agenda.ts. Task 4-1 defines the canonical fields:

- AgendaTask extends BoardTask with project_name/project_color and `planned_ranges:{date,start_min,end_min}[]` for its query calendar window.
- AgendaDay has date, tasks and meetings. TodaySnapshot has query, native-clock today, week_start, seven days, overdue/due_today/tomorrow task arrays, today's meetings, hours_this_week project rows, counts and fingerprint.
- WeekSnapshot has query, native-clock today, project, week_start, seven days, later/no_date tasks, overdue_before_week and overdue_counts, fingerprint. Use snapshot.today for overdue/current-day styling, independently of the pinned week date.
- MeetingOccurrence has stable ref, occurrence revision, actual starts_at/duration, owning project metadata, show_in_day and attendance. Series labels do not authorize frontend recurrence expansion.

Views receive snapshot or null, nowUtc, timeZone, loading, pending, recoveryRequired and error. All mutations/navigation use callbacks:

```ts
type DueDestination =
  | { kind: 'date'; date: string }
  | { kind: 'none' }
  | { kind: 'picker' };
onOpenTask(id: string): Promise<void>;
onOpenMeeting(ref: MeetingRef): Promise<void>;
onJoin(url: string): Promise<void>;
onPlanTask(id: string): Promise<void>;
onMoveDue(id: string, destination: DueDestination):
  Promise<'committed' | 'cancelled'>;
onNavigateWeek(date: string): Promise<void>;
onNewTask(): Promise<void>;
onNewMeeting(): Promise<void>;
onOpenYourDay(): Promise<void>;
onRetry(): Promise<void>;
```

Supply only callbacks a view uses. Retry delegates to the integration owner, which distinguishes read failures from unknown mutations. A successful callback means confirmed commit or explicit cancellation, never merely that an editor opened. Task 4-3 uses the existing Workspace queue and the planned M4 applyMeeting action `move_due {taskId,dueAt,expectedRevision}` through AgendaState; it owns durable request identity and authoritative publication. This receipt-backed deadline command is a proposed M4 contract, not a baseline API. Do not assume baseline Workspace.patch already has durable transport recovery.

Task rows accept display mode (digest/compact), task, shared clock/zone, pending, optional drag preview/source state and callbacks. Optional running timer indicator is supplied from integration, never queried locally. Meeting pills accept occurrence, display date, mode and open/join callbacks. Stable render keys include occurrence ref and display date where the same overnight occurrence appears in two days; navigation passes the ref, never a generated row index or start timestamp.

## Today and seven-day strip

Render long date/counts and Plan in Your Day action; Monday–Sunday strip from the snapshot; left Overdue/Due today/Tomorrow, right Meetings/Hours this week. Done tasks are excluded only from the three digest buckets; weekly strip Done tasks are struck through. Earlier times today remain Due today. Counts come from snapshot records, never reference mock labels.

Rows show project dot/title and project/status/completed hours/planned time metadata. Plan is available for unfinished tasks whose planned_ranges contain no block on snapshot.today and calls onPlanTask with the existing ID. It does not create another task. Strip task/meeting pills are buttons opening their detail. Empty bucket copy is distinct from loading, missing snapshot or query error.

Meetings use the supplied actual occurrence starts and elapsed duration, including overnight labeling. Highlight the earliest not-ended occurrence by actual UTC start with stable identity tie-break; concurrent items still remain visible. A stored safe link exposes Join through onJoin; no link means no fake join action. Past styling follows actual end. Show in Your Day false does not hide meetings from this view. Joining does not mark attendance.

Hours bars use completed-entry totals grouped by local start date within the snapshot's Monday–Sunday range. Running elapsed must never be added. Show zero values clearly, use project colors and scale bars against the largest displayed total (all zero means zero-width bars); label the actual hours so width never suggests a fixed quota.

## Week layout and destinations

Render project Board/Week/Notes navigation through integration or its existing sub-bar; this task must not create a second route store. Show week navigation/title/stats and New task/New meeting entry points. The grid uses `repeat(7,minmax(0,1fr)) var(--week-later-width)` with 10px gaps and the existing 200px final column. At 1200px constrain card min-width to zero, wrap controls and truncate labels while preserving full accessible names/detail opening. Use bounded internal vertical scrolling; no page-level horizontal overflow or hidden offscreen controls.

Each day shows local date, today wash and unfinished overdue count only when the date is before today. Render compact tasks and dashed meeting pills with project color. Done tasks remain struck through. Show actual empty-day placeholders. Later means due dates after the displayed Sunday, No date means null; neither may silently swallow tasks due before the displayed Monday. Expose earlier overdue tasks in a labeled, expandable list with Open task and Move to date, preserving real dates.

Meetings can span multiple days and retain one stable identity; show continuation/time metadata without inventing extra occurrences/counts. No meeting dragging is implemented by this view. No dated subtask fields are introduced.

## Due-date pointer and keyboard workflow

Use primary-pointer drag handles on tasks with pointer capture, a small movement threshold before activating drag, and target hit testing by observed element geometry. Capture the source ID/date/revision context, but let the parent revalidate current authority. Do not mutate task arrays optimistically.

During drag show a semitransparent destination AgendaTaskCard and faded source using existing drag opacity tokens; the preview is inert/aria-hidden and has no duplicate active controls. Do not use a bright insertion line. Date columns, Later and No date have labeled target states. Source-to-identical-date drops are no-ops; release outside recognized targets cancels.

On release call onMoveDue once, retaining a pending visual until callback settlement. For date-preserving moves, render full opacity only when authoritative snapshot publication confirms the new due date. For destinations requiring an editor, parent owns the dialog while the Promise remains pending. Cancel restores source without a success treatment. Rejection restores source and exposes an actionable error; unknown state keeps mutation controls disabled until parent reconciliation. Never replace an unknown request with another payload.

Escape, pointercancel, unexpected lost capture, unmount and project/week changes before submission cancel the gesture without writing. Clean up capture/listeners/animation frames. After submission navigation follows parent close guards rather than canceling a possibly committed write. If vertical autoscroll is needed, confine it to the view's scroll container and recompute targets from current geometry; never move the page sideways.

Every task has a keyboard Move to date action using the same callback with picker destination. The editor exposes explicit date, Later selection and Clear deadline action through integration. Meetings and card child controls never begin task dragging. Restore focus to the moved task in its destination, or its source/initiating control after cancel; if it moves outside the current week, focus the relevant Later/No date control and announce the resulting date.

## Precision, DST and DueDateDialog

Task 4-1 exports proposeTaskDate(task,targetDate,zone) and resolveTaskDate(draft,zone). Proposals preserve the current local HH:mm:ss.SSS by converting the original instant directly; a normal destination becomes a canonical UTC due_at. A null destination explicitly clears the deadline. The editor Clear deadline action submits an empty date/time pair through resolveTaskDate; both empty resolves to null, while a partially empty pair is a validation error. Missing deadlines require a visible 17:00 default; spring-gap or repeated destination times require the editor. Repeated times must not inherit a silent earlier/later choice from the old date.

DueDateDialog is mounted by Task 4-3 and receives:

```ts
draft: { date: string; time: string; offset?: string };
taskTitle: string;
reason: 'move' | 'no-deadline' | 'gap' | 'fold' | 'later';
timeZone: string;
pending: boolean;
recoveryRequired: boolean;
error: string;
onChange(draft): void;
onSave(draft): Promise<void>;
onCancel(): Promise<void>;
onRetry(): Promise<void>;
```

The parent owns the controlled draft and native close/navigation guard. Show date, time including seconds/milliseconds when present, workspace IANA zone, offset choices when repeated, Save and Cancel. A time input with `step="0.001"` or equivalent explicit fields must retain untouched precision; changing date must not truncate seconds. A user changing only hour/minute does not accidentally clear retained seconds. Use Task 4-1 helpers for displayed validation and parent revalidation before save. Invalid dates/times remain visible; gaps explain the nonexistent local time and folds require a chosen offset. Clear stale offset choices after date/time/zone changes.

Use a local in-flight guard to prevent duplicate Save before pending props update. Known rejection retains editable draft/error. Unknown outcome freezes fields and disables discard/cancel with exact Retry available. Do not clear the form or claim success merely because a read refreshed. A confirmed commit/replayed receipt is the parent signal to close it. Escape/backdrop call the parent cancel path, never close the native dialog directly around unsaved changes. Focus the date/time field requiring attention, then restore the initiating move control.

## Tests and native verification

Create meaningful jsdom component tests with Svelte mount/unmount/flushSync and explicit dialog/pointer/geometry stubs. Today tests cover date/clock/zone boundaries, Done filtering versus weekly strike-through, counts/zero hours, hidden planner occurrence visibility, overnight stable refs, navigation/Plan/Join and error states. Week tests cover seven days/later/no-date/earlier-overdue access, same date no-op, pointer cancellation/double release, translucent preview and rejection restoration, editor cancellation, pending/unknown disablement and keyboard destinations. Due dialog tests cover retained HH:mm:ss.SSS, visible 17:00 default, gap/fold choices, stale offset clearing, rejected-save draft retention, duplicate submission, controlled cancel and focus.

Do not duplicate domain arithmetic tests or claim mocked callbacks verify durable receipts. After Task 4-3 integration, compare #2d/#3d at 1600×960 and 1200×760. Use a disposable task with nonzero seconds/milliseconds, move it by pointer and keyboard, verify unchanged status/timer/blocks/hours, cancel a Later edit, clear/reapply a deadline, and test DST gap/fold editor choices. Check Today Plan opens the existing task in Your Day, meeting ref opens the same occurrence across views, hidden planner meetings remain in Today/Week, and offline operation uses local fonts/data. Root/integrator records actual results in M4 verification and owns the final conventional commit; no test/build result substitutes for native observation.
