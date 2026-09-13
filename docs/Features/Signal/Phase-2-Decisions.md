# Approved M2 decisions — Timers & time entries

Approved and date-locked September 12, 2026. Status: **D1–D6 approved without amendments.** The user explicitly approved the recommendations below. M1's D1–D8 approvals remain intact; their deferred timer decisions are resolved here.

Sources: [roadmap](../../../PLAN.md), [complete handoff](../../../_design/design_handoff_signal/README.md), [accepted HTML](../../../_design/design_handoff_signal/Signal.standalone.html), [M1 decisions](Phase-1-Decisions.md), and [M2 grounding](Tasks/Phase-2-Overview.md).

Execution update: D1–D6 have been implemented. M2 is complete and manually accepted September 13, 2026; see [verification](../../verification/M2.md). Approval of these decisions is unchanged.

## Already settled

- One active session per task, with concurrent sessions on different tasks.
- Display elapsed time as **HH:MM:SS**, visibly advancing each second. This is the user's September 12 preference, not a new approval item.
- Rust owns persisted timer transitions and accounting through the existing SQL-plugin pool. Store plugin remains window geometry only.
- Stop creates one completed entry atomically. Completed entries alone determine `hours_worked`; the existing aggregate and transactional cache remain authoritative.
- M2 supplies task timer controls, manual Log, header status, and tray tooltip. Planner interactions, full tray actions, notifications, and the command palette remain later milestones.

## Approved decisions

| ID | Approved behavior | Consequence and boundary |
| --- | --- | --- |
| D1 — Session lifecycle | **Start → Pause → Resume → Stop.** Pause retains the same session and excludes paused time; it does not log an entry. Stop works from running or paused, logs accumulated active time once, and ends the session. Closing a dialog, changing routes, or changing task status does not stop a timer or change task status automatically. | A paused session still occupies the task's one-session slot. Repeated Start must return the existing session without resetting it; repeated Stop must not add another entry. A stopped session can be followed by a new one. Entry start/end describe the full session span; `minutes` is active duration and may be shorter because of pauses. |
| D2 — Sleep, shutdown, and clock changes | Running time **continues during sleep, hidden-window time, and app shutdown**; paused time does not. Use UTC wall-clock differences, with negative segment durations clamped to zero. System clock corrections can therefore change the measured duration; timezone/DST changes alone do not. | This is an explicit wall-clock accounting policy, not a promise to recover true elapsed time after a manually changed system clock. Preserve accumulated completed segments; never write negative minutes or an end before the entry start. No clock-correction review workflow in M2. Tests must cover forward/backward clock changes and accurately document this limitation. |
| D3 — Precision and totals | Keep millisecond precision internally and write fractional `minutes` without rounding to whole minutes. Floor only the displayed seconds; hours may retain the existing two-decimal display. Keep running time visibly separate from completed logged totals until Stop. | Sub-minute sessions count; Pause does not change Board totals. An immediate zero-duration Stop is allowed and ends the session once. Do not add a minimum billable interval, hourly cap, or rounding preference. Durations longer than 24 hours do not wrap. |
| D4 — Manual Log and entry visibility | Task detail → **Log time** opens a form with local date, local start time, and positive duration in hours/minutes/seconds. Default date is today; start and duration require entry. Show timezone, calculated end date/time, task, and **Save time / Cancel**. Reject future end times; allow overnight spans and overlapping entries, including overlaps on the same task. Include a compact read-only list of completed entries in the time section. | Reuse the existing timezone/DST validation behavior: reject nonexistent times and require an offset choice for ambiguous ones. Show active duration separately from session span where pauses exist. Save is explicit and retry-safe; errors retain the draft, and dirty/native-close behavior follows M1. No entry editing/deletion, notes field, reporting page, overlap merging, or planner block picker. Manual entries have `block_id = NULL`; logging does not affect an active timer. |
| D5 — Deletion with a session | Extend the existing permanent-deletion preview to include running and paused sessions. Confirmed deletion discards those sessions together with the task/project's existing owned history; it does not auto-log time immediately before deleting it. State clearly that unlogged time will be lost. | Include persisted timer identity/state/revision in the fingerprint and delete within the same transaction. A Start/Pause/Resume/Stop after preview requires a fresh preview; the display ticking alone does not invalidate confirmation. Preserve all unrelated sessions. This extends the approved M1 deletion graph to the new owned records. |
| D6 — Interactive September fixture | Evolve `pnpm dev:seed` from a frozen clock into a **live September fixture clock**. On its first M2 installation, anchor simulated time at September 11, 2025, 13:42 America/New_York; advance by real elapsed wall time, including between app launches. Persist the anchor so relaunch does not reset time. Add the two illustrated active sessions once: **ATL-477 00:42:17** and **Parse config flags 00:42:09**. | This explicitly changes the M0/M1 frozen-clock decision for M2. All fixture clock consumers must agree, including native task timestamps, deadline/Today computations, meetings, and timers. Preserve original records, completed totals, attachment markers, and user edits; use a separate atomic M2 marker and do not recreate stopped/deleted sessions. Keep ordinary data isolated. No reset UI or additional fixture scenarios; exact frozen comparisons use injected test clocks. |

## Reference reconciliation

Accepted #2b shows `2 timers running` and two active tasks: Dual-write to new data store (ATL-477), `00:42:17`; Parse config flags, `00:42:09`. These are not ATL-482 or the rollback runbook. The existing seed already contains those tasks and their 13:00 blocks, but no active timer state.

At the approved 13:42:00 fixture instant, reproducing those elapsed values implies starts at 12:59:43 and 12:59:51. Treat these sub-minute offsets from the displayed 13:00 block labels as a documented fixture reconciliation. Keep M2 seeded sessions task-based with no block association; actual start-from-block behavior belongs to M3. Preserve ATL-477's existing 9 completed hours, auth's 8 hours, and runbook's 2 hours; active time is not already included in those totals.

If an existing edited fixture is missing either illustrated task, installation must preserve that deletion rather than recreate the task or overwrite user state. The executable spec must define a deterministic skip/report path and ensure the marker prevents later surprise installation.

## Resolution record

On September 12, 2026, the user stated: “I approve D1-D6. Let's start on M2. Phase prompt, tasks and specs. Whatever we need. Let's do it.” All six decisions are approved without amendments. The approved kickoff produces three executable task/spec pairs and a self-contained execution prompt. No additional product approval is needed for these six behaviors. Planning documents are not evidence that their implementation or native verification has occurred.

## September 13 acceptance follow-up

The user approved adding running icons to task cards and a clickable header dropdown listing all running tasks, with navigation to each task's existing editor. This extends the M2 discovery UI and permits changes to `TaskCard.svelte`, `BoardColumn.svelte`, and `Board.svelte` alongside Task 2-3 integration. Task 2-1's live-session snapshot additionally supplies read-only task/project labels. D1–D6 accounting and scope exclusions remain unchanged; no M3 or full tray timer actions are introduced.

Implemented and native-verified; see [M2 verification](../../verification/M2.md). The user subsequently resolved PNG viewing through the default-app setting and accepted M2 after the Board overflow correction. See final verification. No push authorized or performed.
