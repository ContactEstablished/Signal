# Implementation specification 2-3 — Shared timer integration and verification

Execution status: complete; M2 manually accepted September 13, 2026. [Final evidence](../../../verification/M2.md). The specification below remains the approved implementation contract.

Companion: [Task 2-3](../Tasks/Task-2-3.md). [D1–D6](../Phase-2-Decisions.md) were approved September 12, 2026. This is an executable implementation contract; no M2 execution result is asserted.

## Files and insertion points

Use the Task 2-3 ownership list.

- `db/client.ts`: extend `Foundation`; modify `openFoundation()` after `Database.load()` and baseline fixture loading, before `initialize_workspace` and badge reads.
- `domain/clock.ts`: replace the seeded frozen-clock branch in `makeClock()`. Preserve `dateAt()`, `dayBounds()`, fixture timezone, and calendar/DST behavior.
- `state/app.svelte.ts`: integrate `load`, `refresh`, `loadDetail`, `publish`, `mutate`, `delete`, and the existing single queue.
- `state/timers.svelte.ts`: own session/time-read state, read-generation guards, and unresolved timer intents; do not own another write queue.
- `App.svelte`: replace the 60000 ms repaint interval; bind task-detail time props; add header status; replace hardcoded fixture date/time and obsolete timer-preview copy.
- `components/shell/TimerStatus.svelte`: render read-only running status using existing font/icon/token conventions.
- `seed.ts`: update the opening comment to explain native M2 installation; do not change `seedTables`, `FIXTURE_VERSION`, IDs, or original entries.
- Shared CSS: add only required semantic values and responsive treatment. Preserve the complete original token inventory and all local font weights.

Task 2-1 owns native timer/clock/tray/deletion changes, domain timer DTOs, native wrappers, and migrations. Task 2-2 owns `TaskTimeCard`, task-detail changes, logging UI, and formatting/manual-log helpers. No dependency or manifest change is planned. Request corrections from the relevant owner.

## Prerequisite API contract

Use Task 2-1's exported types:

```ts
type TimerSession = {
  id: string;
  task_id: string;
  block_id: string | null;
  state: 'running' | 'paused' | 'stopped';
  started_at: string;
  segment_started_at: string | null;
  accumulated_ms: number;
  ended_at: string | null;
  revision: number;
  entry_id: string | null;
};

type ClockSnapshot = {
  now_utc: string;
  offset_ms: number;
};

type TimerSnapshot = ClockSnapshot & {
  sessions: TimerSession[]; // live running/paused sessions only
  warnings?: string[];
};

type TimerWriteResult = {
  snapshot: TimerSnapshot;
  detail: TaskDetail;
  entries: TimeEntry[];
  outcome: {
    session_id: string | null;
    entry_id: string | null;
  };
};
```

`TimeEntry` preserves the existing schema fields. Do not add time entries to the M1 `TaskDetail` shape; maintain a separate task-time read.

The native wrapper exports:

```text
initializeTimers() -> TimerSnapshot
getTimers() -> TimerSnapshot
getTaskTime(taskId) -> {snapshot, entries}
startTimer({requestId, taskId}) -> TimerWriteResult
pauseTimer({requestId, taskId, sessionId, expectedRevision}) -> TimerWriteResult
resumeTimer({requestId, taskId, sessionId, expectedRevision}) -> TimerWriteResult
stopTimer({requestId, taskId, sessionId, expectedRevision}) -> TimerWriteResult
logTime({requestId, taskId, startedAt, durationMs}) -> TimerWriteResult
```

Confirm these actual exports before integration. Native durable receipts make an exact request replay return fresh authoritative snapshot/detail/entries with its original outcome. The same ID with a changed normalized payload is Conflict. Do not infer successful completion merely from a later snapshot: explicit replay resolves an uncertain write.

## Foundation initialization and one clock

Extend `Foundation` with the timer snapshot returned during initialization. Use this order:

```text
runtime_config
Database.load(runtime.database)   // registered migrations applied
if runtime.seeded:
    loadSeed(runtime.database)    // existing debug/isolation checks remain
timerSnapshot = initializeTimers()
initialize_workspace             // attachment recovery, with clock now ready
read projects/tasks/counts
read tray preference
compute badge from Date.now() + timerSnapshot.offset_ms
return Foundation including timerSnapshot
```

`initializeTimers()` is idempotent because `openFoundation()` is also used for refresh. It must load authoritative current state without resetting sessions, offset, or markers. Native Task 2-1 owns atomic first-install fixture behavior, not frontend seed inserts.

Change the clock factory to an explicit offset-based interface, for example:

```ts
makeClock({
  offsetMs,
  timeZone,
  read = () => new Date()
})
```

Its `nowUtc()` returns `new Date(read().getTime() + offsetMs).toISOString()`. Ordinary mode uses offset zero and the resolved system timezone. Fixture mode uses the native offset and `America/New_York`. Use `snapshot.now_utc` as an authoritative timestamp/validation input; do not repeatedly derive a new offset from IPC delivery time. The native-provided `offset_ms` is the contract.

All Today/deadline/day-boundary/meeting inputs, live timer values, manual-log defaults/future checks, and native mutations must use the same simulated clock. On each tick preserve existing local-day-change refresh behavior. Focus refresh recomputes the system timezone in ordinary mode and uses the fixture timezone in seeded mode.

Retain exported fixture constants as documented anchor values where useful, but remove production branches that continually return the anchor. Deterministic tests inject `read`; frozen test clocks do not define runtime fixture behavior.

D6 installation remains native: persist `{real_anchor_utc, fixture_anchor_utc}` plus an M2 marker in one transaction; seed the two approved existing task sessions once. The initial elapsed values are ATL-477 2537000 ms and Parse config flags 2529000 ms, both `block_id=NULL`. Missing tasks or existing sessions are skipped with warnings; the marker prevents surprise later installation. Show initialization warnings without treating preserved user edits/deletions as startup corruption.

## Timer state and read ordering

`TimerState` uses Svelte runes for live sessions, per-task entries/read errors, and unresolved intents. It is owned by Workspace and does not import a second mutation queue. Keep request records keyed by task so navigation never changes a pending command's target.

Expose selected-task session and entry state to the view. Use generation guards for task-time reads. A late response for task A must not replace task B's entry list or current dialog. Invalidate pending timer reads before publishing a committed timer result so a response started before that mutation cannot overwrite its authoritative snapshot.

Reads may refresh all live sessions while loading entries for one task. Distinguish the global session read from task-specific entry loading. A task-time read failure must not erase known sessions or completed entries. A deletion invalidates affected entry reads and removes the deleted task's cached data; refresh the live session snapshot after the successful deletion.

Do not let an old foundation refresh replace a newer committed snapshot. Use the existing refresh generations plus a timer write/publication epoch, or an equivalent explicit ordering mechanism. A display tick only updates `nowUtc`; it never requests SQLite state every second.

## One queue, authoritative publication, and retries

Route timer and manual-log mutations through `Workspace.mutate` or a narrowly extended equivalent that still uses its existing `queue`. The queue also covers M1 field edits and deletions. Resolve a fresh action's live session ID/revision when its queued intent executes.

Create one opaque request ID for one deliberate write intent. Save its exact normalized payload before invoking native code. For a retry, use that retained ID, session ID, expected revision, and payload; do not recompute them from newer state.

```text
executeTimerIntent(taskId, action):
  if unresolved intent exists:
    allow only explicit retry of that exact intent
  else:
    enqueue an action intent
    at execution resolve authoritative session/revision
    allocate request ID and retain exact request
  invoke native request
  on authoritative success:
    invalidate older timer/detail/board reads
    publish snapshot
    publish detail through Workspace's existing task/revision path
    publish task entries
    if Log and this request ID has not been confirmed before:
      increment this task's logCompletionVersion once
    mark the request resolved and clear it
    attempt normal refresh
    if refresh fails: keep published state and show Saved; refresh failed
  on typed native AppError rejection:
    no commit occurred; clear recovery state and preserve draft/error
    Conflict rereads but never silently overwrites/reapplies
  on untyped transport/unknown outcome:
    retain exact request and payload
    set recoveryRequired for this task
    show explicit Retry for that same operation
    block replacement writes affecting this task until resolution
```

Task 2-1 guarantees a typed native `AppError` rejection means no commit and never throws such an error after commit. Do not classify uncertainty by message text. An untyped transport failure has unknown outcome even if a later read looks plausible.

Do not regenerate a request ID because an invoke rejected, a refresh failed, the route changed, or a component remounted. Do not allow modified Log fields to reuse an old request ID. Repeating `onLog` with the frozen submitted payload while unresolved reuses the original intent. Read retry and unresolved-write retry are distinct controller operations even if the UI's retry callback selects the appropriate one. Recovery must never disable its only Retry path.

A successful write followed by refresh failure resolves its write promise successfully. The dialog must not preserve a fake unsaved Log intent or resubmit it. Publish the authoritative `detail` first so its task revision and logged hours are available to the next queued task edit.

The controller's unknown-outcome state survives navigation attempts. Until resolved, block all other writes affecting the same task, including M1 field/child/attachment edits and task deletion. Project deletion checks every affected task for unresolved intents before writing. Apply this gate when the queued operation executes, not only when its button was clicked; an earlier queued write can become uncertain. Other tasks may continue. Explicit retry of the retained request bypasses only this recovery gate, not normal queue serialization.

Use a per-task transient `logCompletionVersion`, plus a set of confirmed Log request IDs. Increment once when a Log's committed result is first confirmed, including receipt replay through `onRetry`; a repeated replay must not increment again. This signals successful completion independently of which callback resolved it. A new form captures the current version and clears only if it actually submitted a Log whose result has now been confirmed. It must never clear an unrelated unsubmitted draft merely because history refreshed.

Deletion stays in the same queue. Task 2-1 provides transactional session removal and native tooltip update. After a successful delete, remove affected cached entry state, refresh sessions, and preserve existing project/task selection behavior. Do not attempt to stop/log a timer before confirmed deletion.

## Task UI binding and editor protection

Task 2-2 exports `TimerUiBindings` from the module script of `TaskTimeCard.svelte`; both it and `TaskDetailDialog` accept optional `time?: TimerUiBindings`.

```ts
type TimerUiBindings = {
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
  onLog: (input: {
    startedAt: string;
    durationMs: number;
  }) => Promise<void>;
};
```

Bind callbacks to the dialog's task ID. Workspace owns IDs/revisions; components never generate native command identity. Load task-time data when opening detail without discarding M1 fields. If timer reads fail, show retryable time-section errors while preserving the rest of the task editor.

Task 2-2 wraps timer/log operations in the existing dialog operations set and integrates dirty Log drafts with `requestClose`. App retains the existing native edit guard, pending-exit request IDs, and `workspace.settled()` behavior. A merely running or paused timer is persisted business state, not an unsaved editor draft, and must not prevent navigation/Close/Quit. An in-flight write receives pending protection; `recoveryRequired` explicitly prevents closure while an unknown write requires resolution. Freeze a submitted Log form during recovery, offer Retry, and clear it only through its confirmed completion version. Typed no-commit rejections leave the normal draft editable. Never infer these states from error strings.

Keep existing attachment routing, internal Board pointer drag, keyboard shortcuts, Ctrl+K gating, and editor focus restoration. Do not introduce a second top-level modal controller.

## Header and lifecycle

Mount `TimerStatus.svelte` in `.header-actions` before search. Derive the count from sessions with state `running`; paused sessions do not count. Render `1 timer running` or `N timers running` with the accepted lime treatment and a lucide icon. Zero running sessions needs no running pill. Do not create a clickable dead-end or implement a timer-management menu.

Use existing `--timer-fill` and design color variables. Add named CSS variables only where a required value is missing. Keep the native Windows titlebar, 52 px application header, project tabs, Today badge, search stub, and Settings entry usable at 1600×960 and 1200×760. Avoid a per-second screen-reader announcement of elapsed text.

Change App's clock interval to 1000 ms and clean it up on unmount/HMR. Elapsed display derives from persisted accumulated time plus the current running segment, clamped at zero per D2. Do not add milliseconds to stored state each tick. Focus/visibility restoration triggers a guarded timer/workspace refresh and immediate clock tick. Hidden WebView throttling cannot affect native accounting.

Native Task 2-1 updates a count-derived tooltip on startup, transitions, and deletion. Frontend does not push tray text every second. Preserve minimal Open/Quit menu and both close-to-tray modes.

Replace hardcoded September date/13:42 text with clock-derived fixture copy. Update milestone/timer-preview copy only for implemented M2 surfaces. Your Day remains the M3 preview, Today remains its later-phase preview, and notification/full-tray actions remain M6.

## Automated verification

Extend clock tests to verify an injected clock advances with the offset, restoration uses the same offset, ordinary offset is zero, and fixture timezone remains fixed. Retain 23/25-hour day-boundary tests. Do not keep the old production-frozen-fixture assertion.

Timer-state/workspace tests must cover:

- Two concurrent task sessions and paused exclusion from running count.
- Reads arriving after selection change or a committed mutation.
- Stop/Log publication of task totals, revisions, entries, and sessions before a failed refresh.
- A queued M1 edit using the revision published by an earlier accounting result.
- Unknown response followed by exact same-ID/payload retry; changed replacement intent and M1 edits/deletions are blocked, including project deletion affecting an unresolved task.
- Typed no-commit rejection restores ordinary draft editing; recovery has an enabled Retry path.
- Log completion version advances once per confirmed request, including replay through Retry; no unrelated draft is cleared.
- Duplicate clicks and remount/navigation unable to create duplicate writes.
- Deletion clearing affected session/entry views without altering unrelated sessions.
- Read errors preserving authoritative known state and exposing recovery.

Use deferred promises and the actual controller/helpers to test ordering; do not merely verify that mocked methods were called. Run the full Task 2-3 command list for the final integrated state and rerun relevant checks after fixes.

## Native and visual verification

Record actual outcomes in `docs/verification/M2.md`.

| Area | Required evidence |
| --- | --- |
| Upgrade and seed | Existing M1 fixture upgrades additively; first M2 installation anchors the clock and adds approved sessions once; repeated launch preserves edits, stopped/deleted sessions, and attachment markers. Fresh installation is proved with temporary native databases rather than wiping user data. |
| Ordinary isolation | Launch ordinary mode separately; verify no fixture clock/session installation and no changes to ordinary records during seeded checks. |
| Concurrent flow | On disposable tasks A/B, Start both, observe advancing HH:MM:SS and header/tray count, pause A, leave B running, hide/restore, Quit/relaunch, resume A, then Stop both and inspect distinct entries/totals. |
| Accounting | Immediate Stop, sub-minute duration, paused Stop, repeated actions/retries, long duration, and manual overlapping/overnight entries behave according to D1–D4. Test gaps/ambiguous local times and future-end rejection. |
| Recovery/failure | Lost-response replay creates no duplicate entry, failed readback preserves committed totals, stale revisions require reapply, and task deletion/Stop races have native transactional tests plus user-visible recovery. |
| Clock policy | Injected tests verify forward/backward clock behavior and running/paused restoration. Verify actual native sleep/restart where possible; never change the user's system clock merely to test. Distinguish simulated evidence from actual sleep testing. |
| Deletion | Disposable task/project previews warn about lost unlogged time; persisted timer transitions invalidate preview, display ticking alone does not, and unrelated sessions survive. |
| M1/M0 regression | Dirty task/manual-log native Close and tray Quit protection; tray OFF/ON Open/Quit; Board drag preview, task fields, attachments, and existing window geometry remain functional. |
| Visual/offline | Compare #1b/#2b timer treatments at both supported sizes, verify local Sora/DM Sans loading and no new remote assets, and check operation offline while retaining the local dev server. |

Record exact task names/IDs, commands, click-paths, and observed before/after entries where useful. Destructive checks use disposable fixture records. Do not delete ordinary records, reset app-data directories, or invent additional fixture tasks to match mock counters.

If automation cannot exercise a required native interaction, state the precise limitation and retain the manual check as pending until a real user result arrives. Historical M1 sign-off does not accept new M2 behavior automatically.

## Handoff and commit ownership

M2.md records implementation boundary, approved clock/accounting semantics, the evolving fixture anchor, seed reconciliation, commands, click-paths, actual test counts, native/visual outcomes, failures, and remaining later-phase stubs. README explains ordinary/seed launch and the advancing fixture; PLAN changes to complete only after the completion gate.

As the sole final status integrator, update the overview, all three task documents, all three paired specifications, decisions status, and `docs/Features/Signal/Execution-2-Prompt.md` consistently. Exact paths are listed in Task 2-3. This is a metadata-only exception to document ownership after prerequisite implementation handoff, never permission to overwrite concurrently edited contracts.

The integrator owns the final conventional commit:

```text
feat(timers): persist concurrent timers and logged time
```

Inspect status/diff and stage only intended M2 files. Do not implicitly include unrelated work or push unless authorized for this phase. Kickoff creates planning documents only; this task's later execution owns implementation verification and the completion commit. If a required check is blocked, finish independent work, document the exact blocker, and report M2 partial. Stop for review before M3.
