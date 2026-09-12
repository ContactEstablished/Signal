# Implementation specification 1-2 — Projects and Board

Status: complete; manual acceptance confirmed by the user on September 12, 2026. See [M1 verification](../../../verification/M1.md).

Companion: [Task 1-2](../Tasks/Task-1-2.md). [D1–D8](../Phase-1-Decisions.md) approved September 12, 2026. New APIs are defined by [specification 1-1](ImplementationSpec-1-1.md), not existing M0 code.

## Files, contracts, and ownership

Create only Task 1-2's enumerated files, exporting components/helpers at file root. Consume existing `src/lib/domain/dates.ts` without editing it. Import planned DTOs from `domain/types.ts`; never duplicate database record definitions or issue SQL from components.

Task 1-4 mounts Board in existing App.svelte and supplies `snapshot:BoardSnapshot|null`, `loading`, `readError`, `nowUtc`, `timeZone`, `selectedDate`, `selectedTaskId`, ordered projects, and callbacks. UI callbacks mirror the Task 1-1 payloads:

```text
onRetry(), onOpenTask(id), onNewTask(projectId)
onSelectProjectView('board'|'week'|'notes'), onPreview(message)
onRequestProjectEditor({mode:'create'|'edit',projectId?})
onCreateProject({name,color}), onUpdateProject(id,{name,color})
onMoveProject(id,'left'|'right')
onMoveTask(id,status,beforeTaskId)  // controller supplies latest expectedRevision
onPreviewDeletion(target), onDeleteEntity(target,fingerprint)
```

Mutation callbacks return the native result or reject with AppError. The integrator adds revision, serialization, and cross-view refresh; it must not convert a successful commit followed by failed readback into a retryable write failure. The underlying native wrapper is `moveTask(id,status,beforeTaskId,expectedRevision)`.

`getBoard(projectId,dayStartUtc,dayEndUtc)` supplies complete project tasks, tags, and day-intersecting meetings. BoardTask includes `revision`, `sort_order`, `blocked_since`, `tags`, `subtask_done`, and `subtask_total`. Fetching and UTC day boundaries belong to Task 1-4. Business records are immutable component inputs; only filter/drag/dialog drafts are local state.

## Component responsibilities

- Task 1-4 mounts ProjectSubBar once above the selected project subview, including Week/Notes previews, so navigation back to Board stays available.
- Board composes strip/columns and owns transient filters/drag state. It exports `openFilters()` for the parent to route the sub-bar Filter action to the mounted Board.
- ProjectSubBar renders Board/Week/Notes, Filter, fixed Group · Status, New task, and the project-menu entry. Filter/Group appear only on Board; navigation, New task, and the project menu remain available on previews.
- BoardColumn renders label/count and card/drop sequence.
- TaskCard renders title, provider ID or Local task, tags, due label, optional hours/estimate, blocked reason, selection, and done treatment. Enter/Space opens; nested action controls cannot accidentally open or drag.
- BoardFilters supplies labeled controls, Clear, Escape and focus restoration.
- ProjectDialog uses native HTML dialog for create/edit draft and confirmation presentation. ProjectMenu owns menu intent/state and fetches deletion preview through callbacks. Do not depend on a separate shared modal module.
- MeetingsStrip renders the chosen local day's count and 09–18 geometry; clicking a meeting reports an M4 preview.
- `board.ts` contains pure filter/group/presentation/layout/drop functions with explicit inputs, no Svelte/database/system-clock/native imports.

Use explicit runes mode in application components without forcing it globally on lucide dependencies.

## Filters, order, and dates

Define `DueFilter='all'|'overdue'|'today'|'next7days'|'noDate'` and `BoardFilterState={text,priorities:Priority[],tagIds:Id[],due:DueFilter}`. Trim and case-fold search against title/external ID. Priorities OR together, tags OR together, dimensions AND together; empty selections impose no restriction.

Use the supplied timezone's calendar date: overdue is before today and excludes Done; Today is equal; Next 7 days is tomorrow through today+7 inclusive; No date is NULL. Today/Next 7 days can include Done. Explain the future-day range in filter help. Use calendar arithmetic (the Task 1-1 Temporal dependency is available), not seven 24-hour durations across DST. Card urgency is separate: an unfinished card is orange when calendar-overdue or due no later than now+24h.

Urgency requires a non-null due instant; undated cards never receive urgency treatment.

Group fixed statuses backlog/todo/in_progress/blocked/done, sorting by sort_order then ID. Counts show total when unfiltered and visible/total whenever a filter is active. Filters are view state; switching projects resets them in M1. The global Today badge remains an unfiltered record-derived count.

## Drag and mutation behavior

Drop before a visible card sends that ID as `beforeTaskId`; end-of-column sends NULL and means end of the full unfiltered column. Backend ordering preserves hidden relative order. Self-target/no positional change is a no-op. Provide a status menu or equivalent keyboard-accessible action that appends to the destination; unchanged status does nothing.

User-approved drag refinement (September 12, 2026): show a translucent full-card preview in the destination slot and fade the source; replace the bright insertion line. On release, show the destination card at normal opacity while saving, then replace it with the authoritative card. Keep the source records unchanged until commit and restore their presentation on failure. Preview cards are inert, and their occupied slot must not cause hover-target oscillation. Await the serialized callback with pending feedback; disable repeat/conflicting gestures. Never compute/write status timestamps or hours in the UI. On Conflict, reread through the parent and ask the user to retry their drag; do not silently reapply an obsolete drop. On failure, retain committed order and show an error. Cancel on Escape, project navigation, or destruction.

On Windows WebView2, keep OS file drop support for Task 1-3; use pointer-based internal card dragging with pointer capture rather than depending on HTML5 drag/drop that may compete with native file events. Release capture on cancel/destruction, distinguish a click from a drag with a small movement threshold, and autoscroll the board container near its edges. The accessible status action remains available regardless of pointer support.

## Projects and confirmation

Require trimmed nonblank names and four colors. Editing only changes name/color, preserving summary configuration. Disable left/right at list boundaries and during a pending project mutation. On create success the parent selects the new project; on delete it selects the next surviving neighbor or Your Day if none remain.

Header Add project and menu Edit both call the parent's `onRequestProjectEditor` routing flow. Task 1-4 awaits native edit-guard registration before mounting ProjectDialog with create/edit mode and its initial record. ProjectDialog exports `requestClose(reason):Promise<boolean>` using the same reasons and settlement contract as Task 1-3. Pending saves settle first; rejected saves preserve the draft. Cancel/Escape/navigation/native exit with changed values requires confirmed discard or successful submission. An application close never confirms a pending entity deletion.

Delete requests a fresh native preview, presents affected counts and permanent-deletion wording, and sends that fingerprint only after confirmation. Cancel does not mutate. A changed graph requires a refreshed preview and renewed confirmation; never silently approve larger counts. A committed deletion with pending file cleanup closes the deleted entity's view and shows the cleanup warning, rather than retrying deletion. Failed database deletion retains the confirmation/error.

## Meetings and visual invariants

Meeting intervals intersect `[dayStartUtc,dayEndUtc)`; no recurrence expansion. Count all intersecting records, including those outside the visible strip. Render the local-time intersection clipped to 09–18 and provide a small outside-hours list for records with no visible intersection. Overlapping pills use deterministic stacked rows within the strip so every pill remains selectable. Handle overnight meetings and exact endpoints without negative widths. Show the now tick only for the clock's local date when inside 09–18.

Fixture Atlas has one September 11 meeting; do not add Website Standup to force the mock count. Preserve fixture 13:42 and derived auth 8h/runbook 2h.

The strip uses bg-raised with its MEETINGS label and pills in the current project's color; include local date/count and an orange now tick with time label. Column labels are uppercase Sora 600 12px with faint counts; In Progress is cyan, Blocked orange, Done lime. Card titles are DM Sans 500 13.5px/1.35, due labels align right beside ID/tag chips, and progress tracks are 4px. Reuse existing blocked-border/blocked-text tokens, surface-3 ID chips, and each tag's color tint. New task remains cyan. Read token values from tokens.css rather than duplicating literals in component CSS.

Use five equal `minmax(0,1fr)` columns with `--board-gap`14px, token padding/radii, 16px Lucide, Sora600 labels, and DM Sans titles. Cards have no shadow. Match orange blocked/due treatment, cyan selected border/progress, lime Done label, and done opacity/strike-through. NULL/zero estimates never divide by zero; actual hours remain visible and visual progress caps at 100%. Long titles/chips cannot force the window beyond1200×760; card contents wrap or truncate accessibly.

Distinguish no project, empty project, filtered no matches, initial loading, load failure, and retained-content refresh failure. Views must not fabricate successful persistence.

## Tests, runtime verification, and handoff

`tests/unit/board.test.ts` covers AND/OR filters, calendar/DST boundaries, done classification, stable order, filtered end/before-target intents, clipped/overlapping/overnight meetings, and progress edge cases. Call the actual helpers. Run Task 1-2's commands.

After Task 1-4 mounting, exercise create/edit/reorder/delete on disposable projects, cancellation and stale preview, task open, within/across-column filtered drag, pointer autoscroll/cancel, status keyboard alternative, errors, and restart. Compare #1b at 1600×960 and 1200×760. Test native OS attachment drag after internal card drag to catch event interference.

Provide actual unit results, component callbacks, and any deferred runtime checks to Task 1-4. It owns final verification documentation and conventional milestone commit; no M2 work belongs here.
