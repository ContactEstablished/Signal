# Implementation specification 3-2 — Planner canvas and interactions

Companion [Task 3-2](../Tasks/Task-3-2.md); [D1–D8](../Phase-3-Decisions.md) approved September 13, 2026. Consume [Spec 3-1](ImplementationSpec-3-1.md) DTOs/helpers and [Spec 3-3](ImplementationSpec-3-3.md) callbacks. No baseline planner implementation is claimed.

## Files and presentation contract

Create only Task 3-2's four files. PlannerCanvas owns transient gesture geometry; PlannerBlock owns rendering/disclosure; TaskPicker owns search/focus. No native/global Workspace imports.

Use `PlannerItem` snake_case fields from domain/planner.ts: id,kind,start_min,end_min,title,project_name,project_color,task_id,block?,meeting?,session?. A task's supplied session may belong elsewhere; association is `session.block_id===block.id`. `packLanes(items)` returns `{item,lane,lanes,top,height}`.

Canvas props: items,date,nowUtc,timeZone,pending,selection `{startMin,endMin}|null`; callbacks `onSelect(startMin,endMin,anchor:DOMRect)`, `onCancelSelection()`, awaited `onMove(blockId,startMin,endMin)`, `onDropTask(taskId,startMin)`, `onOpenTask(taskId)`, `onDone(blockId)`, `onRemove(blockId)`, `onTimer(blockId,action)`, `onJoin(url)`; `onEditTime(blockId)` opens Task 3-3 BlockEditor. onMove serves resize too. Compact details may be local PlannerBlock disclosure, preserving the same action callbacks.

Picker props: range,anchor,tasks,pending,error; awaited `onPick({kind,taskId?})`, `onCancel`. Parent retains provisional selection until cancellation or confirmed persistence. Local search never creates a record. Task 3-3 resolves DST endpoint choices; never silently select an ambiguous offset.

## Grid, clock and lanes

A bounded scrolling wrapper contains 1440px logical content and 64px gutter. Set scrollTop 420 after first mount or deliberate selected-date change, not on timer ticks/items refresh/scope changes. Show past-date full shade/future no shade/today shade before local now. Now line only for the selected current date. Use parent nowUtc/timeZone, no interval.

Fade labels describe offscreen ranges; say nothing planned only when no item intersects that range. Midnight remains reachable and marker/fade layers are pointer-transparent.

For each packed group, equal widths minus 4px gaps; left=gutter+lane*(width+gap). Retain logical range for labels/persistence. Project-tinted task blocks have checkbox/prefix/time/title/meta; own-block running adds active ring, lime elapsed, Pause/Stop; own-block paused shows frozen elapsed/Resume/Stop. Other-associated sessions expose task navigation without reassignment. Run-over uses resolved block endpoint and never stops time.

Neutral Break/Lunch/Focus are dashed with distinct icons. Meetings are project-colored dashed, stored instance times/link and safe Join callback; no move/resize/done/remove/timer controls. At 3 lanes retain full content when height permits. At 4+ or short 26px height use compact title and a focusable disclosure exposing full details/actions. Do not depend on hover or let controls overlap adjacent blocks.

## Gesture state and failure behavior

Use a discriminated gesture: select(origin/current),move(original range/pointer origin/preview),resize(original/preview), each with pointerId. Primary pointer only. Start selection on free canvas, not gutter/item/child controls; movement uses a drag handle. Capture pointer and clean up on all exit paths.

Convert minute=`clientY-scrollWrapperRect.top+scrollTop`; snap nearest15, clamp starts0..1425/ends15..1440. Reverse selection sorts endpoints with minimum15. Move preserves duration and clamps whole interval; resize keeps start and minimum15. Historical non-snapped records stay readable; editing follows snapped rules. Use translucent destination/faded source and cyan dashed selection. Submit exactly once on release; local pending blocks duplicates until settlement. Render final state from authoritative props. Rejection restores original position and exposes error. Escape/pointercancel/unexpected lost capture/date change/unmount cancel without mutation; stop animation frames/release capture. Internal edge autoscroll uses requestAnimationFrame and recalculates preview from current pointer/scroll; do not scroll the page.

Only recognize internal task drag MIME `application/x-signal-task-id`, carrying an opaque ID. Parent revalidates eligibility/duration/endpoints. Drop provides snapped start; D5 duration comes from domain/integration. Supply visible keyboard Add block and Edit time routes through parent BlockEditor; pointer dragging is not the only scheduling mechanism.

## Picker and accessibility

360px picker anchors to selection's right, flips/clamps inside viewport and stays visible after scroll/resize. Focus search on open. Filter supplied task options by title/ID/project. Arrow keys move active option; Enter picks; Escape cancels. Tab reaches neutral choices/Cancel. Use buttons or complete combobox/listbox semantics. Pending disables duplicate selection; failed save preserves query/selection/error. Restore initiating focus. Announce errors/actions, not elapsed seconds.

## Verification and handoff

Use jsdom rectangle/scroll/capture stubs to verify conversion, reverse/day-edge ranges, move duration, resize minimum, duplicate completion/cancel/lost capture, rejection, internal drop and picker search/keys. Verify 3/4 lanes and short-block accessible controls, meeting restrictions. These establish frontend behavior only.

Native gate with integration: compare #2a/#2b/#3e at 1200×760/1600×960;07:00 initial scroll, midnight reachability, short adjacent blocks,3/4+ lanes, scrolled drag/resize/autoscroll, no horizontal overflow, picker clipping, keyboard-only creation/edit, own/elsewhere/paused timers and run-over. DST attempts must enter parent validation without silent writes. Task 3-3 records actual evidence and owns final commit; no passing native claim from unit tests alone.
