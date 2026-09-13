# Implementation specification 3-1 — Planner contracts, calendar and transactions

Companion: [Task 3-1](../Tasks/Task-3-1.md). D1–D8 approved September 13, 2026. Baseline `fbd1f58`; planned APIs below are not baseline implementation claims.

## Files and API boundary

Use Task 3-1 ownership. Register migration 4 in `db::migrations` and `get_planner`/`apply_planner` in `lib::run`. Retain all existing commands, SQL-plugin pool, fixture clock initialization, native editor guard and attachment root isolation.

`domain/planner.ts` defines:

- `PlannerQuery {date:string,timeZone:string}`.
- `PlannerBlock`: original block fields plus `revision:number`, `time_zone:string|null`, `start_offset:string|null`, `end_offset:string|null`.
- `PlannerTask extends TaskRecord` with current `project_name` and `project_color`.
- `BlockDraft {kind,task_id,start_min,end_min,start_offset?,end_offset?,source_id?}`; query supplies date/timezone. Source IDs are opaque provenance, not labels.
- `PlannerSnapshot {date,time_zone,blocks,previous_blocks,copy_sources,tasks,meetings,entries,claims,dismissed,fingerprint,timers}`. Tasks are unfiltered; meetings are stored instances intersecting the day; entries start within that local date; timer snapshot is the existing M2 DTO. Source lists come from preceding date and most recent Monday strictly before selection.
- `PlannerItem {id,kind,start_min,end_min,title,project_name,project_color,task_id,block?,meeting?,session?}`; kind includes meeting. Nullable neutral fields are explicit.
- `PlannerInput {requestId,query,action,payload,expectedFingerprint?}` and `PlannerResult {snapshot,outcome:{block_ids:string[],task_id?:string,destination_date?:string},changed_details:TaskDetail[],changed_entries:Record<string,TimeEntry[]>}`.

`native/planner.ts` exports `getPlanner(query)` and `applyPlanner(input)` using `invoke`. CamelCase applies to envelope/input keys; schema-derived records retain snake_case.

Actions: create (`draft`), move (`id,expectedRevision` plus replacement range/offsets), done (`id,expectedRevision,done,stopSession?`), remove (`id,expectedRevision`, fingerprint required), dismiss (selected-date banner), batch (`mode:quick|carry|due|copy,blocks:BlockDraft[]`, fingerprint required). StopSession identifies the expected own-block session/revision; never stop a replacement session by task ID alone.

Approved editor refinement: create and move accept optional payload `destinationDate`, defaulting to `query.date`. The query remains the source/read context, including source block lookup and the returned snapshot. Validate the destination calendar date and endpoints in `query.timeZone`; move updates the same block's date/range/offsets and revision transactionally, preserving all other fields and associations. Return `outcome.destination_date` for these actions. The destination participates in canonical receipt matching; exact replay after the block leaves the source date succeeds without another move or revision increment. A late receipt/reply failure rolls back the move. No additional migration is needed for this refinement.

## Migration and provenance

Add block revision and nullable timezone/offset metadata, initialized safely for historical rows. Existing blocks remain floating local-time records; migration never changes their date/range or silently validates them in today's zone.

Create planner request receipts keyed by request ID with canonical payload and outcome JSON. They are opaque replay tombstones and outlive removal of their created blocks; receipt replay returns current state and the original outcome without recreating records. No task titles, notes, or file content enter receipts.

Create source claims keyed by operation provenance: `carry:<sourceId>` once globally; `copy:<sourceId>:<destinationDate>` per destination; due planning uses a per-task/date key. Store mode, source identity, task identity when applicable, destination date and nullable created block link. Block removal detaches the result link but preserves the claim so repetition cannot recreate a deliberately removed result. Parent task deletion can remove its source claims; mixed-batch request receipts remain replay tombstones. Neutral copy claims remain independent of project deletion. Handle all new foreign keys in existing deletion graph/remove order; no historical entity resurrection on exact replay.

Persist banner dismissal by destination date in SQLite settings, not the window store. Do not mark source blocks Done during carry.

## Calendar and scheduling authority

Use existing Temporal frontend helpers for local dates and offset choice. Add `chrono-tz` 0.10.x for native IANA validation; [official API](https://docs.rs/chrono-tz/0.10.4/chrono_tz/) supplies chrono TimeZone implementations. Native code must validate real dates, known zones, integer minute bounds, endpoint existence, and the chosen offset when local time is ambiguous. End 1440 means midnight of the next calendar date. Persist selected offsets and the zone in which they were resolved. Reject unknown fields/kinds and mismatched task/kind. New/edited intervals snap to 15 minutes; untouched historical intervals remain readable.

Expose pure helpers in `planner-rules.ts`: date addition/previous Monday, endpoint normalization/choices, meeting projection clipped to the local day, duration selection, next-free interval, eligibility, carry/due/copy preview and footer totals. Use UTC instants to identify meetings intersecting a date, convert endpoints into local visual minutes; no recurrence expansion in M3.

For a meeting crossing a DST fold, the fixed grid must cover the complete envelope of occupied wall minutes on both sides of the repeated hour. Native occupancy uses the same conservative envelope. Planned meeting totals use actual UTC elapsed duration clipped to the selected local day, independently of the visual envelope.

Automatic placement uses actual intervals from every project plus meetings, half-open overlap semantics, and the next 15-minute boundary at/after current local time for today or 07:00 for future days. Past automatic actions reject. Scan merged occupied intervals; place only if the whole range fits before 1440. Account for intervals reserved earlier in the same batch. Validate again under the native write transaction against current clock and data. A preview becoming stale requires a new preview, never an invisible relocation on Apply.

D5 remaining durations: positive remaining hours rounded up to 15 minutes, max 120; unknown 30; exhausted15 for explicit drag, skipped by automatic due planning. Due soon is due before midnight at selected date+7, including overdue; unfinished only. Sort due instant, priority high/medium/low, title and ID deterministically. Unscheduled excludes any task already blocked that date. Scope filters display/statistics and automatic candidates only, never occupied time. Planned is sum of block plus meeting durations (parallel counts separately); Logged is completed entries starting that local date; Blocks is done/all non-meeting blocks. Neutral blocks stay visible in every scope.

Carry candidates are unfinished task blocks from preceding day whose task is not Done and whose carry claim is absent, ordered source start/ID. Preserve original planned duration and source link, not remaining estimate. Copy candidates come from the most recent Monday strictly before selection, all task/neutral kinds, original wall time, done reset, no meetings; preserve overlaps and separate copy claims. Repeated batch requests and repeated source selection cannot duplicate results. Batch previews show conflicts/skips/no-space and explicit offset choices where needed; reject invalid/ambiguous endpoints until resolved.

## Lane packing

`lane-packing.ts` exports `packLanes(items)` returning `{item,lane,lanes,top,height}`. Sort start then actual end then stable ID. Painted interval is `[start,min(1440,max(end,start+26)))`; logical minutes remain untouched. Greedily assign the lowest free lane; track overlap-connected groups and apply each group's maximum concurrent lane count to all its members. Separate touching painted intervals are independent; deterministic ties; no mutation of inputs. Canvas computes equal widths with 4px gaps. D1 keeps full content at 3 and compact at 4+, with short-item access handled by UI.

## Native transaction/replay algorithm

`planner::snapshot(conn,query,now,offset)` reads a coherent snapshot using one connection. Fingerprint canonical selected/source/task/meeting/entry/claim/dismissal records and live persisted session fields, excluding current clock and elapsed repaint values. Running time alone does not invalidate a preview, but persisted timer transitions do.

`apply_into(pool,input,now,offset)` uses BEGIN IMMEDIATE:

1. Validate/canonicalize envelope. Look up exact request ID before revision/fingerprint checks. Changed payload is Conflict. Matching receipt returns fresh snapshot/original outcome, with no business write.
2. Check required expected revision and fingerprint. Validate target task/block/source existence and current eligible state. For batch, recompute eligibility, required duration/provenance, all-project occupancy and time floor; reject stale proposals rather than silently changing them.
3. Apply the operation, claims and durable receipt in the same transaction. Prepare snapshot and changed task details before commit. Any failure—including receipt insertion or reply reads—rolls back all changes.
4. After commit, tooltip refresh is best effort and cannot turn success into apparent rollback.

Extract `timers::mutate_in_transaction` from M2's existing transaction body. The public mutate wrapper still begins/commits once and preserves old normalized receipt payloads. Planner done with Stop calls this body on the same connection, then marks block done and records planner receipt atomically. Use a derived stable internal Stop request ID. Existing completed-entry cache triggers and task revision bump remain authoritative.

Extend StartInput with optional blockId omitted from serialized payload when absent, preserving M2 receipts. Validate a provided block belongs to the task; only a new session receives it. Existing running/paused sessions retain their original association. Block removal updates linked session/entry block_id to null, detaches carried children/result links, then removes the block; it neither logs nor stops time. Completion offers task Done separately in the UI, not inside an implicit timer action.

## Verification and handoff

Use registered migrations in temporary disk-backed SQLite tests; never open a user database for destructive tests. Exercise stale snapshot/revision, repeat payload mismatch, replay after removal, different request IDs against claimed sources, rollback after cache entry insertion, Stop+complete replacement-session protection, block removal preserving live elapsed and entries, parent deletion ordering, ordinary/seed migration preservation, DST gap/fold and explicit offsets, no-space/hidden-scope occupancy and rollover. Reopen the pool for durability. Pure frontend tests cover painted lanes, date boundaries/placement/candidates and totals.

Task 3-3 consumes this contract and owns frontend TimerState threading, full native UI verification, evidence and final milestone commit. No M3 completion claim until native/runtime gates pass.
