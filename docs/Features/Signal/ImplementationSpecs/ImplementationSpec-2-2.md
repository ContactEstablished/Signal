# Implementation specification 2-2 — Task timer controls and manual time logging

Execution status: complete; M2 manually accepted September 13, 2026. [Final evidence](../../../verification/M2.md). The specification below remains the approved implementation contract.

Status: ready after Task 2-1; not implemented or verified.

Companion: [Task 2-2](../Tasks/Task-2-2.md). [D1–D6](../Phase-2-Decisions.md) approved September 12, 2026. Native contracts come from [specification 2-1](ImplementationSpec-2-1.md); [specification 2-3](ImplementationSpec-2-3.md) owns controller integration and final verification.

## Files and insertion points

Use only Task 2-2's listed files. Replace TaskTimeCard's preview-only script/template with an optional real-binding branch; retain existing hours/progress styling. Extend TaskDetailDialog's props, operations tracking, dirty(), requestClose(), closeChoice(), time-card mount, and deletion panel. Extend ProjectMenu's existing deletion copy beside affected counts.

Create LogTimeForm and TimeEntryList beneath the time section. Consume existing deadlines.ts and domain/types.ts without modifying them. Do not add timer fields to TaskDetail. Task 2-1 creates domain/timers.ts; Task 2-3 owns application state and all native invocations.

## Planned component/controller API

Export the following type from TaskTimeCard.svelte's module script, importing TimerSession and TimeEntry from domain/timers.ts:

```ts
export interface TimerUiBindings {
  session: TimerSession | null;
  entries: TimeEntry[];
  nowUtc: string;
  timeZone: string;
  loading: boolean;
  error: string;
  pending: boolean;
  recoveryRequired: boolean;
  logCompletionVersion: number;
  onRetry: () => Promise<void>;
  onStart: () => Promise<void>;
  onPause: () => Promise<void>;
  onResume: () => Promise<void>;
  onStop: () => Promise<void>;
  onLog: (input: { startedAt: string; durationMs: number }) => Promise<void>;
}
```

TaskDetailDialog and TaskTimeCard accept `time?: TimerUiBindings`. Preserve existing props and preview callback compatibility until Task 2-3 connects the bindings. Without bindings, retain explicit preview labeling; never expose apparently working controls backed by silent no-ops. The existing onPreview callback may become optional if the real-binding branch does not need it.

Task 2-3 binds the current task/session/revision and owns request IDs. UI callbacks carry normalized manual values only. `nowUtc` is the shared ordinary/fixture clock, not an independently sampled Date.now(). TaskDetailDialog wraps awaited time callbacks in its tracked operations; failures remain visible and must still reject to a submitting form rather than being swallowed by the existing run() helper.

`logCompletionVersion` is a per-task transient controller counter advanced once per committed manual-log request ID. Duplicate delivery/retry never increments twice. Bindings publish authoritative entries, session, detail totals, and version; reads do not falsely acknowledge a submission.

## Elapsed arithmetic and presentation

Create `timer-math.ts` with `timerElapsedMs(session, nowUtc)` and `formatElapsedMs(milliseconds)`. Running elapsed is accumulated_ms plus `max(0, now - segment_started_at)`; paused/stopped elapsed is accumulated_ms. Native contracts guarantee valid instants and nonnegative accumulated milliseconds. Never count a paused interval or subtract a backward-clock segment from accumulated work.

Formatting floors milliseconds to seconds, renders at least two hour digits, and never wraps after 24 hours:

```ts
seconds = Math.floor(Math.max(0, milliseconds) / 1000);
hours = Math.floor(seconds / 3600);
minutes = Math.floor((seconds % 3600) / 60);
remainder = seconds % 60;
```

Do not round stored duration or add per-component ticking. Task 2-3 repaints the supplied shared clock every second. Running elapsed is separately labeled; completed hours/progress continue using `detail.task.hours_worked` and the existing progress helper until successful Stop/Log publication. Use Sora 600, existing style tokens, and `font-variant-numeric: tabular-nums`; do not add a font weight or token inventory.

## Timer controls and read-only history

No active session: Start and Log time. Running: elapsed, Running, Pause, Stop, and Log time. Paused: frozen elapsed, Paused, Resume, Stop, and Log time. Done/Blocked/task status changes do not disable or implicitly mutate the session. Stop is allowed while paused and for zero active duration.

Disable duplicate actions while the relevant operation is pending. Loading/error states are explicit; retry failure cannot masquerade as an empty history. Keep errors near the controls with a Retry action. Recovery-required state disables new transitions until reconciliation. Never replace a failed transition with optimistic session state or silently restart a session.

TimeEntryList receives entries plus timezone. Show completed start/end date/time and active duration derived from `Math.round(minutes * 60_000)` (recover integer milliseconds before formatting to avoid binary floating-point display artifacts); use deterministic newest-first ordering with ID as a tie-breaker. Distinguish active duration from full session span when they differ. Show an empty state only after successful loading. No edit/delete controls, invented notes, or planner navigation.

## Manual-log normalization and layout

Create `time-log.ts` exporting a draft shape, a default-draft helper accepting nowUtc/timeZone, and `normalizeTimeLog(draft, timeZone, nowUtc)` returning `{startedAt, durationMs, endedAt}`. The callback submits only startedAt/durationMs; native code repeats validation and calculates the authoritative end.

Draft fields: date, startTime, selected UTC offset, hours, minutes, seconds. Default local date is today; start is blank; duration has zero/default numeric fields and remains invalid until positive. The untouched initialized form is not dirty merely because it has defaults.

Use existing deadlineChoices/deadlineToUtc for `YYYY-MM-DD` and `HH:mm`. Reject missing/invalid dates/times and nonexistent spring-forward times. Ambiguous fall-back input must display the returned offsets and require selection. Clear an obsolete offset choice when date/time changes. Never infer the host timezone when the fixture supplies America/New_York.

Hours are a nonnegative integer; minutes/seconds are integers 0–59. Reject nonnumeric, fractional, negative, nonfinite, zero-total, or unsafe-integer milliseconds. Compute end by adding duration milliseconds to the resolved UTC instant; reject end later than supplied nowUtc, and catch unsupported date ranges. Overnight spans and overlaps are allowed; changing duration does not silently shift start. Manual logging does not pause or stop a running session.

LogTimeForm is a nested section in the existing detail dialog, not a second native/global modal. Display task name, timezone, labeled fields, calculated end including date/seconds, offset choice when necessary, and Save time/Cancel. Keep the panel usable inside the existing 280px rail and scrollable detail at 1200×760. Set initial focus on the start input; restore focus to Log time after save/cancel. Do not announce elapsed seconds through an assertive live region.

## Submission and recovery behavior

Keep one normalized submitted payload and a local in-flight guard. Await `onLog(payload)` and clear the form only on confirmed success, including the controller's completion-version acknowledgment. Capture logCompletionVersion at submission; a subsequent increment for this task acknowledges the retained submitted log even if recovery came through shared onRetry. A version advance clears only an actually submitted form. It must never clear a new unsent form opened after an earlier request, and callbacks from a completed form must not close a newer form instance.

The controller distinguishes definitive native AppError rejection from uncertain IPC/transport outcome without inspecting message text. Known rejection retains editable values and error. The same payload retries with the retained request ID; a changed payload receives a new request only after the previous outcome is known. UI code must not generate request IDs. The controller removes unresolved requests only when their outcome is known.

When recoveryRequired is true, freeze submitted fields, prohibit new saves/transitions, retain the exact normalized payload, and offer retry/reconciliation. Retry may invoke onLog with that frozen payload or shared onRetry; both use the retained controller request. Do not call Cancel/Discard an undo of an uncertain committed entry. Completion-version acknowledgment must clear once without resubmission even when authoritative refresh arrives before the original callback settles.

## Existing editor and close coordination

Add form dirty/invalid/submitted status to TaskDetailDialog's existing close decision, keeping it separate from field baselines. Expose LogTimeForm methods or equivalent typed callbacks for dirty inspection, explicit submission, and discard; do not create another global editor registration.

requestClose waits existing and time operations. Running/paused persistence alone is not an unsaved draft and never prevents close. An unresolved write blocks closure until reconciled; explain why and offer Retry/Keep editing. The existing Save changes action must explicitly indicate that it also saves a pending time log, validate it, and await acknowledgment before closing. Discard changes may discard an unsent or definitively rejected log draft along with the existing draft choices; it cannot cancel an unknown submission.

Preserve notes text, subtask/tag draft text, attachments, invalid fields, and deduplicated native Close/Quit behavior. A timer state/detail refresh must not overwrite dirty task fields or reset an open log form. Escape/backdrop enter the existing close path rather than bypassing it. A log error must not activate an unrelated “Retry pending edits” action that silently omits the log.

## Timer-aware deletion

When `deletion.counts.timer_sessions > 0` or `preview.counts.timer_sessions > 0`, show plain copy: running and paused sessions will be discarded and their unlogged time will be lost. These counts describe active sessions, not stopped history. Preserve existing affected-record counts and permanent-deletion language.

Use the existing preview fingerprint flow. Task 2-1 invalidates the fingerprint after Start/Pause/Resume/Stop, but ordinary elapsed repaint does not. On conflict, require a new preview and confirmation; never retry deletion automatically. UI does not auto-stop/log before deletion.

## Verification and handoff

Add meaningful tests in the owned files: running/paused arithmetic, backward segment clamps, fractional seconds, 100-hour formatting; positive manual normalization, zero/fractional/unsafe rejection, overnight/future boundaries, DST gap and both explicit overlap offsets; timer state buttons, duplicate suppression, errors, completed totals; exact-payload recovery, completion-version acknowledgment, and form retention. Extend task-editor tests for dirty notes plus a log draft, pending native close, failed save, unresolved outcome, and successful retry without duplicate logging. Include a new unsent form surviving an earlier submission's delayed completion.

Run the Task 2-2 verification commands, then hand bindings and results to Task 2-3. Runtime checks after integration: Start/Pause/Resume/Stop from detail, a second concurrent task, advancing seconds, unchanged totals until Stop, completed history with paused span, valid overnight Log, future rejection, and close while a log/notes draft exists. Compare the rail to #1b, verify keyboard/focus and minimum-window layout, and exercise both deletion warnings and stale previews.

Mocked callbacks do not establish database durability, restart recovery, exact-once native accounting, or tray behavior. Task 2-3 owns those checks, phase evidence, and the conventional milestone commit after completion. Do not claim native checks or commit this task's work on behalf of the final integrator.
