# Implementation specification 1-4 — Shell integration and verification

Companion: [Task 1-4](../Tasks/Task-1-4.md). [Approved decisions](../Phase-1-Decisions.md) dated September 12, 2026. Execution is complete, including user-confirmed manual acceptance on September 12, 2026. This remains the implementation contract; results and sign-off are recorded in [M1 verification](../../../verification/M1.md).

## Files and insertion points

Use the exact Task 1-4 ownership list. Replace M1 branches in existing App.svelte: the Add project `stub()` call, selected-project placeholder body, and task-related routing. Preserve Today/Your Day, Settings and its `toggleTray()` path, native diagnostics, shell tokens, and the Ctrl+K preview. Keep src/main.ts, fonts.css, and migration 1 unchanged. Task 1-1 exclusively owns the narrow editor-exit additions to window.rs/tray.rs; this integrator must not write those files concurrently.

`state/app.svelte.ts` owns shared route/project/task/query state in runes. `domain/mutations.ts` owns the pure queued-operation helper; `domain/clock.ts` owns the testable clock interface. `native/attachments.ts` owns the single OS drop subscription/cleanup. It wraps existing `@tauri-apps/api/webview` `getCurrentWebview().onDragDropEvent`, whose declaration was verified in installed `webview.d.ts`; API callbacks provide native paths for drop events. It imports application commands from Task 1-1, not an unrestricted filesystem plugin.

Global CSS extends existing form/focus rules for textarea/select/dialog where needed; token edits add named values only when M1 needs a missing design value. Do not restyle unrelated M0 surfaces. In tauri.conf.json, retain native dimensions/decorations/tray configuration and native file-drop support. If local fixture asset loading uses fetch, allow only `'self'` in connect-src alongside existing IPC endpoints; never add remote wildcards or loosen script policy.

## Shared state and route contract

Represent the active route as Today, Your Day, Settings, or `{projectId,view:'board'|'week'|'notes'}`; project subviews Week/Notes render named previews. Keep one active task dialog: none, new draft, or detail(taskId). Store BoardSnapshot and TaskDetail separately as authoritative reads, plus loading/error/query-generation values. Do not fork per-component business stores.

Mount Task 1-2's ProjectSubBar once in the common project-route wrapper above all three subview bodies. Route Filter to the mounted Board's `openFilters()`; show Filter/Group only on Board. Header Add project and ProjectMenu Edit use one `onRequestProjectEditor` controller path. The active-editor union is none, project create/edit, task new, or task detail; never open two editors or let one editor clear another's native guard.

`clock.ts` exposes `nowUtc():string`, `timeZone:string`, and local-day boundary conversion through Temporal. Seed uses the existing FIXTURE_NOW/FIXTURE_ZONE; ordinary uses the real Date clock and resolved system zone. Update the ordinary now tick on a bounded minute cadence, refresh on window focus and local-day changes, and clear subscriptions on destruction. Fixture time must never advance with elapsed runtime. Boundaries are local midnight through next local midnight, converted to UTC, including 23/25-hour DST days.

On initial startup call existing openFoundation(), retain native runtime selection, then fetch selected project/details through Task 1-1. `loadBoard()` passes the clock-derived day interval. Increment a query generation for every selection/reload; discard responses if their generation or entity no longer matches current state. Handle deleted selections as NotFound and choose a valid destination, not an endless error retry.

## Mutation serialization and error semantics

Use one controller queue for M1 business mutations. This single-user milestone does not need concurrent autosave requests. Queue operation intents rather than captured stale full DTOs; at execution read the newest authoritative revision for that task, call the corresponding native operation, then apply its authoritative returned state. A subsequent operation on that task must use the new revision.

```text
enqueue(intent):
  await previous operation settlement
  resolve current entity/revision
  await native mutation(intent, expectedRevision)
  publish committed reply
  attempt refresh projects + badge + selected Board/detail
  if refresh fails: retain committed reply; show "Saved; refresh failed" + Retry refresh
```

The queue must continue after rejection. Errors before commit propagate to the originating editor with its dirty buffer intact. Conflict requires a reread and explicit user reapply/discard; never automatically overwrite newer state. All refreshes use generation guards. Do not retry a committed creation/deletion merely because refresh failed. Mutations returning only a project/deletion result must still refresh affected task readers and badge.

For `removeAttachment`, publish the returned `detail` and pass its `cleanupPending` warning to the editor without treating the operation as rejected. `discardStagedAttachments` may finish logically with cleanup queued; confirmed draft closure can proceed with that warning once the durable discard succeeds. A database rejection of discard retains tokens and the editor for retry. Failed multi-file staging does not add any descriptors from that batch to the draft.

Project create selects the new project Board. Project removal selects the next surviving neighbor in prior order, then previous, then Your Day. Task removal closes its detail and focuses New task or the nearest surviving card. Only successful task creation may navigate create-and-plan to Your Day, with text confirming creation and stating the planner is a preview. No block/timer/meeting record is written by navigation.

## Dialogs, keyboard, and drop routing

Task 1-2/1-3 own their native HTML dialogs; the parent coordinates which is open and invokes each dialog's dirty/pending close flow before navigation. Preserve trigger references for focus restoration, checking that elements remain connected after refresh/deletion. An in-dialog confirmation restores focus within its parent editor on cancel.

Keep a reference to the active editor's exported `requestClose(reason):Promise<boolean>`. All dialog, navigation, native-close, and native-quit paths await this method before unmounting or changing route; false leaves state and guard intact. The controller deduplicates closure and completion so successful Create during a pending close cannot navigate or discard tokens twice. Creation offers Discard/Keep editing on dirty close; detail uses Save/Discard/Keep editing. A pending deletion confirmation is canceled by closure, never approved by it.

Route `n` only when Board is active, no modal is open, and focus is outside input/textarea/select/contenteditable. Dialog submit shortcuts stay inside Task 1-3. Ctrl+K continues to open the M0 palette preview only when it cannot bypass a dirty modal. Escape closes the active layer through its existing discard/pending-save rules; it never starts a second close while a write is unresolved.

Subscribe once to native drag/drop. `enter/over` only show an attachment target for the active task editor; `drop` sends native paths to its staging callback; `leave` clears feedback. If no task editor is active, do not import files or create a task implicitly. Catch staged results arriving after the dialog closes and discard those tokens. Unsubscribe on destroy/HMR. Preserve native file support while Task 1-2 uses pointer dragging for internal cards; verify both in one native session.

Gate drop routing to the active task dialog's full bounds and suspend it during discard/delete confirmation. Match native event coordinates to the dialog's CSS coordinate space using the current window scale factor; verify at Windows scaling above 100%. Drop outside the dialog does not import behind its scrim. Pass staged descriptors and discard callbacks to both creation and detail; the controller retains ready tokens until adoption or durable discard.

Before opening an M1 task/project editor, await setEditGuard(true) from Task 1-1; do not expose editable input before its acknowledgement. Subscribe to `signal://exit-request`. Its native guard prevents Close or tray Quit from acting until the editor's mode-specific close flow resolves. Clean editors proceed immediately; dirty/failed editors use the choices above, and pending writes settle first. Resolve the exact request ID with proceed=true only after clean save or confirmed discard; cancel/error resolves false and retains the editor. Native code then performs the original close-to-tray or Quit action. Ordinary dialog closure also clears the native guard once all edits settle. Do not use an asynchronous frontend close listener as the only protection. Test native Close and tray Quit with clean, dirty, saving, and failed editors in both tray modes. Forced OS termination does not preserve an in-memory draft and is not claimed to do so.

## Synthetic fixture attachment hookup

Create valid small `src/lib/fixtures/gateway-arch.pdf` and `latency-p95.png`, both visibly labeled “Synthetic Signal fixture.” PDF content can be a simple illustrative gateway diagram/text; PNG an illustrative latency comparison. Do not imply these are recovered originals. The adjacent README records generation method, synthetic status, and rights/origin. Use standard document/plot tools and inspect both files during implementation.

`seed-attachments.ts` imports these asset URLs, loads their local bytes, and calls installFixtureAttachments with stable opaque IDs, actual bytes/MIME, and the exact approved filenames. Dynamic-import it only under import.meta.env.DEV after the existing M0 loadSeed completes; use the existing fixture context checks, never an ordinary startup path. The native command independently enforces isolation. Implement the API without embedding missing assets in Task 1-1 so prerequisite native compilation remains independent.

Use a new M1 attachment marker, leaving FIXTURE_VERSION and all existing M0 entity IDs/records intact. Repeated launch must not reinstall attachments that a user removed after the marker was set. Test both an existing marked M0 fixture and a fresh temporary seeded database. Asset files may be bundled for development preview, but installation remains debug/seed-only; no business file is fetched remotely.

## Verification matrix

Run Task 1-4 commands once for the final integrated state; rerun relevant checks after any subsequent fixes. Do not use the original six tests to claim coverage of new mutations. Record test counts and failure details truthfully in docs/verification/M1.md.

| Requirement | Concrete native evidence |
| --- | --- |
| Upgrade/repeatability | Existing M0 fixture → migration 2 → two attachments once; fresh migrations → first seed also gives dense task order. Repeat launch preserves manual order, edited/deleted fixture attachment state, and original task/time totals. |
| Ordinary isolation | Separate ordinary launch has no fixture project/attachment installation; record database/file-root evidence before/after fixture work. |
| Project lifecycle | Create disposable project, edit color/name, move left/right, cancel deletion, confirm preview, verify related removal and surviving records. |
| Board | Five statuses/counts, selection/blocked/done/empty/filter states; drag before visible card and filtered end, keyboard status alternative, then restart. |
| Tasks | Create with URL-only/Markdown title, edit every field family, preserve untouched due instant, dirty-close/rejection/conflict paths, subtasks/tags/alerts, status timestamps and derived hours. |
| Files/links | Real picker and OS drop → managed copy → restart/open → remove while source survives; missing/locked copy error; cleanup retry; unsafe links rejected and safe link opens externally. |
| Keyboard/close | n outside text fields only, Ctrl+Enter/+Shift, Escape/focus restoration, palette gating, native Close with dirty draft and tray OFF/ON, Open/Quit regression. |
| Visual/offline | Compare internal #1b/#3a frames at 1600×960 and 1200×760; confirm local fonts and no remote notes/assets; test with network unavailable while local dev server stays up. |

Use disposable fixture records/files for destructive and failure checks; never delete ordinary user records or wipe application-data directories to simulate a fresh database. Rust tests use temporary databases; UI upgrade uses the existing fixture. If the UI cannot access a native control, report the exact limitation and obtain a manual result instead of marking it passed.

## Handoff and commit

M1.md must include implemented boundary, still-stubbed controls, exact commands, fixture reconciliation, click-paths, actual command/native/visual outcomes, and failures or platform limits. Update README launch/review instructions and PLAN M1 status only during authorized M1 execution. Final feature commit is `feat(board): add projects tasks and task detail workflows`; inspect/stage only the intended M1 changes and report its hash. Do not include unrelated pre-existing kickoff/user files implicitly.

Kickoff itself neither commits nor implements. If a required M1 check is blocked later, finish independent work, document the precise blocker, and report partial. Stop for review before M2.
