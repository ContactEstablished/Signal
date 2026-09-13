# Implementation specification 2-1 — Durable timers and native accounting

Execution status: complete; M2 manually accepted September 13, 2026. [Final evidence](../../../verification/M2.md). The specification below remains the approved implementation contract.

Companion: [Task 2-1](../Tasks/Task-2-1.md). [D1–D6](../Phase-2-Decisions.md) approved September 12, 2026. These APIs are now implemented for M2; they were absent from the M1 baseline.

## Ownership and insertion points

Use only Task 2-1's enumerated files. Register migration 3 in `db::migrations()` after versions 1/2. Register new clock state and `timers` commands in `lib::run`; retain existing plugins, capability restrictions, root lock, editor exit guard, window geometry, and release seed rejection. No additional crate is expected: SQLx, chrono, UUID, serde, tokio, and Tauri are installed.

`timers/mod.rs` contains Tauri adapters and core operations accepting the existing pool plus an injected clock value. `timers/models.rs` owns validated native DTOs. `clock.rs` owns ready-state/offset sampling and anchor parsing. Route runtime clocks in workspace/mod.rs and attachments/mod.rs through it. `workspace/models.rs::now(bool)` may remain a test-only fixed-time helper; it must not supply seeded production mutations after initialization.

Do not change M1 TaskDetail or existing task command payloads. New time reads and write envelopes provide a separate entry list. Task 2-3 integrates frontend clocks/state; Task 2-2 consumes the new timer types.

## Migration 3 and invariants

Keep migrations 1/2 byte-for-byte unchanged. Add two timer-owned tables, retaining all original time-entry fields:

```text
timer_sessions(
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  block_id TEXT NULL REFERENCES blocks(id),
  state TEXT NOT NULL CHECK running|paused|stopped,
  started_at TEXT NOT NULL,
  segment_started_at TEXT NULL,
  accumulated_ms INTEGER NOT NULL CHECK >= 0,
  ended_at TEXT NULL,
  revision INTEGER NOT NULL CHECK >= 0,
  entry_id TEXT UNIQUE NULL REFERENCES time_entries(id)
)
timer_requests(
  request_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  kind TEXT NOT NULL CHECK start|pause|resume|stop|log,
  payload_json TEXT NOT NULL,
  session_id TEXT NULL REFERENCES timer_sessions(id),
  entry_id TEXT NULL REFERENCES time_entries(id)
)
```

Use NOT NULL on primary keys and consistent UTC strings. A partial UNIQUE index on `timer_sessions(task_id) WHERE state IN ('running','paused')` enforces one live session. Add task/session/entry lookup indexes for receipts and history as appropriate. Check state-specific nullability: running has a segment start and no end/entry; paused has no segment start/end/entry; stopped has no segment start and has end/entry. Require end >= original start when present. Validate nonnegative safe integer milliseconds and revisions in native DTOs as well as database constraints.

Keep stopped session rows as durable session identity; receipts retain action identity. They are internal accounting records, not additional active timers. Do not add timer state to settings or the window store. The fixture clock anchor/marker are settings, not timer entities. No attendance/notification schema.

M2 commands create task-based sessions with `block_id=NULL`; native code must reject a supplied block field rather than silently ignore it. Preserve the nullable relationship for M3. If a nonnull block exists in a future consumer, enforce the original same-task invariant at storage level with analogous session block triggers. A Stop uses the retained session block ID and existing entry triggers as the final defense.

The database has 16 product/internal tables after migration 3 (M1's 14 plus two). Keep the original M1-only test's 14-table assertion; add a separate M2 migration test for fresh and populated upgrade paths.

## DTOs and commands

`src/lib/domain/timers.ts` mirrors native fields and exports:

```ts
interface TimerSession {
  id: string; task_id: string; block_id: string | null;
  state: 'running' | 'paused' | 'stopped';
  started_at: string; segment_started_at: string | null;
  accumulated_ms: number; ended_at: string | null;
  revision: number; entry_id: string | null;
}
interface TimeEntry {
  id: string; task_id: string; block_id: string | null;
  started_at: string; ended_at: string; minutes: number;
}
interface ClockSnapshot { now_utc: string; offset_ms: number }
interface TimerSnapshot extends ClockSnapshot {
  sessions: TimerSession[]; // running and paused only, sorted by task_id/id
  warnings?: string[];
}
interface TaskTimeSnapshot { snapshot: TimerSnapshot; entries: TimeEntry[] }
interface TimerWriteResult extends TaskTimeSnapshot {
  detail: TaskDetail;
  outcome: { session_id: string | null; entry_id: string | null };
}
```

Import TaskDetail from existing `domain/types.ts`. Export corresponding input types and wrappers in `src/lib/native/timers.ts`:

| Wrapper / native snake_case command | Input | Result |
| --- | --- | --- |
| initializeTimers / initialize_timers | none | TimerSnapshot |
| getTimers / get_timers | none | TimerSnapshot |
| getTaskTime / get_task_time | taskId | TaskTimeSnapshot |
| startTimer / start_timer | `{requestId,taskId}` | TimerWriteResult |
| pauseTimer / pause_timer | `{requestId,taskId,sessionId,expectedRevision}` | TimerWriteResult |
| resumeTimer / resume_timer | same | TimerWriteResult |
| stopTimer / stop_timer | same | TimerWriteResult |
| logTime / log_time | `{requestId,taskId,startedAt,durationMs}` | TimerWriteResult |

Mutation wrappers invoke native with `{input}`; native serde request DTOs use camelCase input names and reject unknown fields. Outputs retain snake_case record names. Read get_task_time receives taskId in the normal Tauri camelCase wrapper argument. Validate IDs as opaque nonblank text, never external provider keys. Entry reads use `started_at DESC,id DESC`, within one read transaction with the snapshot. Missing task is NotFound, not an empty successful task-time read.

Reuse serializable AppError codes Validation/NotFound/Conflict/Database. Every native rejection means no mutation committed; post-commit tooltip errors become warning strings on a successful reply and/or native logs, never rejection. Frontend untyped transport errors can have an unknown outcome and must replay the same request, as specified in 2-3.

## Atomic transitions and replay

All mutations begin `BEGIN IMMEDIATE`. Normalize input once, and serialize a canonical typed payload containing kind, task ID, and all intent fields except request ID. Do not hash unstable JSON map order. Reject a request ID already used with any different normalized payload. A receipt lookup precedes revision/state validation, since a successfully executed request necessarily changed that state.

```text
begin immediate
verify task still exists
if request receipt exists:
    require exact canonical payload match
    prepare fresh current snapshot/detail/entries and original outcome IDs
    commit read transaction; return success
validate current session identity/revision and normalized intent
apply transition, or intentional idempotent state no-op
append completed entry if required; existing cache triggers run here
insert request receipt with outcome session/entry identity
prepare authoritative snapshot + task detail + entries inside transaction
commit
refresh tray status best-effort from current committed DB
return prepared success (optional warning)
```

Receipt lookup cannot create a deleted task: require its existence, and deletion removes owned receipts. Replays return current authoritative data rather than an obsolete serialized task detail that could roll back UI state. Outcome identifies the original result even if a newer session now exists.

Start: if a live session exists, return it unchanged (including paused state); write a receipt for this intent. Otherwise generate a UUID session, running at native now, accumulated 0, revision 0. Do not automatically change task status, including Done/Blocked tasks. An old Start with a previously committed receipt returns its old outcome and current live snapshot; it must never create another timer after its original session stopped.

Pause: validate task/session and expected revision; if running, add `max(0, now - segment_started_at)` milliseconds to accumulated, clear segment start, set paused, increment revision once. Already paused with current revision is a no-op. Resume: paused becomes running with segment start now and revision+1; already running with current revision is a no-op. Stopped sessions reject Pause/Resume. Receipt replay bypasses these fresh-command checks only for the same normalized request.

Stop: after checking session ownership, an already stopped session returns its existing entry without inserting another, even under a new request ID (a state no-op, with a new receipt). Otherwise require expected revision, finalize active milliseconds, and insert one entry with a new UUID, original session start, `ended_at=max(now,started_at)`, `minutes=active_ms/60000.0`, and retained block ID. Update session stopped/end/entry/revision in the same transaction. Zero active time is valid. A stop targeting an old session must never operate on a replacement live session for that task.

Only a new completed entry changes task accounting revision/updated_at: invoke the existing task bump operation within the same transaction after entry insertion. Pause/Resume/Start do not alter task fields/revision, but do alter session state/fingerprint where applicable. Stopped-entry replay/no-op does not bump task revision. Preserve accumulated completed segments across clock corrections; checked integer addition must reject overflow before committing.

## Manual Log

Normalize startedAt through the existing UTC instant parser. durationMs must be a positive safe integer; reject invalid/nonfinite/fractional milliseconds or overflow. Compute endedAt in native code; reject end later than the shared native now. UI validates local timezone ambiguity before supplying the instant; native validates the resulting instant/duration again. No arbitrary current-time override or database argument is accepted.

Insert one original-schema entry with NULL block ID, active minutes=durationMs/60000.0, and receipt in one transaction. Start may be on an earlier date; overnight and overlapping entries are legal, even on the same task. Logging does not mutate a live session. A repeated request returns the original entry, and a changed payload under that request ID is Conflict. New intentional identical logs with distinct request IDs remain allowed by D4; do not deduplicate by time range.

## Runtime clock and fixture installation

Manage one native clock with initialization readiness and a signed offset in milliseconds. Ordinary offset=0; fixture offset=`fixture_anchor_ms-real_anchor_ms`. Every runtime sample is UTC wall time plus that offset. Never increment persisted elapsed per tick. UTC/DST timezone display changes do not alter elapsed; actual wall-clock corrections do according to D2. Never silently reset the clock on restart or refresh. Fail visibly on corrupt/incomplete anchors rather than replacing them.

`initialize_timers` is invoked after Database.load and original seed loading, before initialize_workspace/attachment recovery. It obtains the native-selected pool, acquires an initialization gate, and runs the following idempotent transaction for a debug seeded process:

1. Verify the existing M0 fixture marker. An ordinary process must not install or consume fixture settings.
2. If `fixture_timers_m2` is absent, require no preexisting orphan clock anchor, sample real UTC once, and insert `fixture_clock_m2` JSON with `real_anchor_utc` and `fixture_anchor_utc='2025-09-11T17:42:00.000Z'`.
3. Use the existing opaque fixture task IDs for numeric keys 108 and 116 from seed.ts (UUID suffixes hex `6c` and `74`). Native constants are fixed, not supplied by an untrusted frontend. Add running sessions with `started_at=segment_started_at` at fixture anchor minus 2,537,000 ms and 2,529,000 ms, accumulated 0, NULL block, revision 0. Preserve any existing live session by skipping that task.
4. Missing task or occupied session is a deterministic skip, not task recreation. Persist skipped IDs/reasons in the M2 marker JSON and include readable warnings. Insert no extra tasks, blocks, historical entries, or hours totals.
5. Write the marker with installation/skips and commit anchor/sessions/marker together. On a marker hit, validate/read the existing anchor; do not reinstall stopped/deleted sessions or reinterpret skipped tasks on later launches.

After commit, set the process clock ready and return the snapshot/offset/warnings. Ordinary initialization sets ready offset zero without adding seed settings. Concurrent initialize calls reuse the same persisted anchor. Startup/readiness failures must prevent enabling mutation controls. The snapshot is loaded independently of any open task dialog.

Workspace and attachment adapters subsequently sample this same clock for every task/file mutation and cleanup timestamp. Change their helper signatures/call sites deliberately rather than retaining one frozen caller. Core operations still accept injected `now` so tests remain deterministic. M0 baseline seed records and marker stay unchanged. Task 2-3 updates foundation badge and all frontend clock consumers to the returned offset before showing interactive UI.

## Deletion and native tooltip

Extend workspace deletion graph to include owned timer_sessions (including stopped history for cleanup) and timer_requests. Include persisted session IDs/state/revision/timestamps/receipt relationships in its fingerprint; never add computed elapsed or sampled now. Public preview count `timer_sessions` counts running/paused sessions only; stopped records may be counted separately as `timer_history` if shown, and technical request rows must not be presented as user history.

Confirmed deletion removes receipts first, sessions second, then existing time entries while tasks still exist for the cache triggers; retain the remainder of the existing deletion/attachment queue logic. Graph reads and removal stay transactional. A newly committed timer transition changes the fingerprint; a repaint does not. A concurrent Stop/delete serializes through SQLite: either Stop commits then deletion requires renewed confirmation, or deletion commits and Stop is NotFound. Preserve sessions from unrelated tasks/projects. Do not log immediately before deleting.

Keep tray ID `signal-tray` and menu Open/Quit. Add one native refresh helper querying committed live state: no sessions → `Signal`; otherwise `Signal · N timer(s) running · P timer(s) paused`, omitting zero categories and using correct singular/plural. Count-derived tooltip needs no per-second task and remains correct while hidden. Initialize, successful transitions/replays, and successful task/project deletion request refresh. Serialize tooltip refresh reads/sets with a native mutex or equivalent so a stale read cannot overwrite a newer count. Tray failure does not roll back a completed accounting mutation; refresh on later operation/read/focus initialization retries it. Do not introduce extra menu actions or notifications.

## Tests, failure injection, and handoff

Native tests call production core operations with temporary SQLite files/pools and injected UTC values. Test unique-live-session enforcement, concurrent different tasks, simultaneous same-task Start, receipt-payload mismatch, Pause/Resume stale revisions, zero/subsecond/fractional Stop, Pause exclusion, stopped-session duplicate Stop with new/same IDs, old Start replay after replacement, and unknown-response simulation by discarding a committed reply then replaying its request.

Test transaction failures before entry insert, after insert before session/receipt completion, and during authoritative reply construction; assert rollback of entry/cache/session/receipt/revision. Test manual future/overflow/zero validation, legal overlaps/overnight, and exact receipt replay. Pause must survive reopen without accumulating shutdown; running reopen includes the UTC gap. Forward/backward wall values exercise D2 clamping and nondecreasing accumulated completed segments, not a false monotonic-clock promise.

Extend native deletion tests with all three migrations and live/stopped sessions, stale preview after state change, no conflict from clock-only advancement, discard without logging, unrelated survival, and Stop/delete races. Existing attachment tests keep their file behavior and use compatible migration setup; fixed `now(true)` can remain test-only. M0/M1 frontend migration-only tests retain their original scope.

Fixture tests execute actual installer logic against fresh and upgraded temporary databases: anchors/session starts, original row/total preservation, once-only reload, removed/missing/occupied task handling, marker/anchor/session rollback, persisted elapsed across restart, malformed marker/anchor rejection, and ordinary/debug/release guards. Do not wipe application data for tests. Record new test counts/results when implemented, then hand APIs and evidence to Tasks 2-2/2-3. Task 2-3 owns native UI comparison, actual sleep/tray/close checks, M2.md, and final conventional commit.
