# Signal implementation plan

Status: M0 and M1 complete. On September 12, 2026, the user confirmed all M1 manual acceptance checks passed and authorized the milestone commit and push. See `docs/verification/M1.md` for automated results, native evidence, and user sign-off. M2 is complete and manually accepted on September 13, 2026, including the timer discovery and Board overflow refinements. See `docs/verification/M2.md`. M3 is complete and manually accepted September 13, 2026; its commit and push are authorized. See `docs/verification/M3.md` for actual automated/native evidence and final user sign-off. M4 kickoff is authorized; its unresolved product decisions are next. Assisted meeting/task intake was requested September 13, 2026 and is recorded below as proposed M8 work.

## Sources and scope

- Source of truth: `_design/design_handoff_signal/README.md`, read in full.
- Visual reference: `_design/design_handoff_signal/Signal.standalone.html`, opened and inspected in Microsoft Edge. Accepted screens: `#1b`, `#2a`–`#2e`, `#3a`–`#3e`. Rejected explorations do not inform implementation.
- The handoff lives under `_design/` in this repository, rather than the path originally quoted in the request.
- Precedence: explicit user instructions, then handoff README, then accepted HTML. Unspecified product behavior requires a question before implementation; proposals below are not settled requirements.
- M0 implementation was explicitly approved with the adjustments recorded in `docs/decisions.md`. M1 execution was subsequently authorized by the user after approval of D1–D8; the four task/spec pairs are complete, with manual acceptance confirmed by the user.
- Execute M0 through M7 in order, one milestone per review. Stop after each milestone for the user's review before starting the next.
- The user-requested assisted intake feature is tracked separately as proposed M8. It depends on existing task management and M4 meeting management; its placement can be revisited at kickoff. Adding it to the roadmap does not expand the active M3 scope.

## Fixed implementation choices

- Tauri 2 Rust backend; Svelte 5 runes, TypeScript, Vite; pnpm scripts and lockfile.
- SQLite through `tauri-plugin-sql`; checked-in, versioned SQL migrations. No alternate persistence library. Business data and settings stay in SQLite. `tauri-plugin-store` is exclusively for window state.
- `tauri-plugin-notification` for native notifications; Rust `TrayIcon` for the tray. Rust owns scheduling while the window is hidden.
- Plain CSS and custom properties, with all handoff tokens in `src/styles/tokens.css`; no Tailwind or component library. Include semantic variables for README screen-specific values as well as the Design tokens section, so component styles do not introduce raw design values.
- Bundle local WOFF2 Sora 600/700 and DM Sans 400/500, including license notices. No runtime font requests. Approved hour-label weight: Sora 600; do not add 500.
- `lucide-svelte` icons at 16px; preserve the meanings of circle, square, check, external-link, grip-vertical, play, and pause. Use the same icon family for other required controls.
- Markdown uses `marked` followed by sanitization. External links open through a narrow native boundary. Attachment operations copy files into application data, with database metadata pointing to the copied file.
- Pure TypeScript domain functions are independent of Svelte and native APIs. Vitest covers lane packing, carry-over, timer math, and summary generation. No E2E suite.
- Offline single-user operation: no accounts, OAuth, provider APIs, remote title fetching, telemetry, or remote assets. Email uses `mailto:` in M5; SMTP is later work outside M0–M7.
- M8's screenshot/text interpretation needs a separate processing decision: local models/OCR or an explicitly configured external service. No provider, credentials, cost, network dependency, or change to the offline baseline is approved by this roadmap addition. Ordinary task, meeting, and planner workflows must remain usable without the optional interpreter.

## Intended folder structure

Directories are introduced in their owning milestone, not populated with unused placeholder modules in M0.

```text
Signal/
  PLAN.md
  README.md                         # setup, scripts, seed scenarios, review instructions
  package.json
  pnpm-lock.yaml
  index.html
  vite.config.ts
  svelte.config.js
  tsconfig.json
  vitest.config.ts
  _design/design_handoff_signal/    # supplied references; retain as supplied
  docs/
    decisions.md                    # approved answers to handoff gaps
    verification/
      M0.md ... M7.md               # checks, screen comparisons, stubs, click-paths
  public/fonts/
    sora-600.woff2
    sora-700.woff2
    dm-sans-400.woff2
    dm-sans-500.woff2
    licenses/
  src/
    main.ts
    App.svelte
    styles/
      tokens.css
      fonts.css
      global.css
    lib/
      seed.ts                      # deterministic mock fixtures and loader
      domain/
        types.ts
        clock.ts
        dates.ts
        links.ts
        lane-packing.ts
        leftovers.ts
        timer-math.ts
        summary.ts
        digest.ts
      db/
        client.ts
        repositories/              # projects, tasks, meetings, blocks, time, summaries, settings
      state/
        app.svelte.ts
        projects.svelte.ts
        timers.svelte.ts
      native/
        commands.ts                # typed native command/event boundary
        attachments.ts
        clipboard.ts
        links.ts
        window.ts
      components/
        shell/                     # header, project tabs, project sub-bar
        ui/                        # shared modal, picker, fields, chips, toggles
        tasks/                     # cards, forms, subtasks, notes, time card
        meetings/                  # strip, pills, create/detail modals
        planner/                   # grid, blocks, picker, sidebar, leftovers
        summaries/
        settings/
      views/
        Board.svelte
        YourDay.svelte
        Today.svelte
        Week.svelte
        Settings.svelte
        FirstLaunch.svelte
  src-tauri/
    Cargo.toml
    Cargo.lock
    build.rs
    tauri.conf.json
    capabilities/
      default.json
    icons/
    migrations/
      0001_initial.sql             # every README table
      ...                          # approved additions, versioned explicitly
    src/
      main.rs
      lib.rs
      db.rs                        # SQL plugin setup/migration registration
      window.rs
      tray.rs
      timers.rs
      scheduler.rs
      notifications.rs
      attachments.rs
      backup.rs
  tests/
    unit/
      lane-packing.test.ts
      leftovers.test.ts
      timer-math.test.ts
      summary.test.ts
    fixtures/
      csv/
```

## Persistence and data integrity

M0 creates all 13 tables specified by the README: `projects`, `tasks`, `subtasks`, `tags`, `task_tags`, `attachments`, `meetings`, `meeting_tasks`, `blocks`, `time_entries`, `alerts`, `summaries`, and `settings`. Preserve every listed field. Add primary keys, foreign keys, enum checks, indexes, and transactional writes. Confirm deletion policy before selecting cascading business-data deletion behavior.

Treat `hours_worked` as derived from completed time entries, never an independently editable total. M0 retains it as a transactionally maintained cache: SQLite triggers update it inside the entry transaction, direct independent edits are rejected, and the `task_time_totals` view is the aggregate authority. Verified against `SUM(minutes)/60`.

Represent planner task/break/lunch/focus blocks using the README `blocks` kinds. Meetings appear as planner items projected from meeting records and recurrence occurrences, avoiding a second conflicting meeting schedule in `blocks`.

The schema does not specify durable running/paused timers, meeting attendance or visibility, a blocked-since date, or notification occurrence/delivery state. Resolve these before adding corresponding migrations; do not silently bury business entities in the window store. Proposal: additive tables/columns, retaining the original schema. Timestamp, timezone, recurrence, and deletion choices are approval items below.

## Seed and comparison strategy

- `src/lib/seed.ts` reproduces the four project names and colors: Atlas Migration/cyan, Website Refresh/lime, Home Renovation/magenta, CLI/violet.
- Inventory visible entities in `#1b`, `#2b`, and `#3d`, including IDs, statuses, due dates, tags, estimates, subtasks, notes, meetings, blocks, and time totals. Accepted cross-project records also come from `#2a`, `#2d`, and `#3b`/`#3c`. One coherent midday snapshot only; temporal comparison scenarios are deferred.
- Key task fixtures include ATL-501, Asana 1189, ATL-506, ATL-490, ClickUp 86c (on-call rotation), ATL-495, ATL-482, ATL-477, ATL-469, ATL-460, ATL-451; Website Refresh copy/accessibility tasks; Home Renovation tile/paint tasks; CLI Parse config flags. External identifiers are scoped with their provider/project rather than treated as global task keys.
- Meeting fixtures include Standup, Auth cutover sync, Data-store review, and the referenced Sprint planning occurrence. Time entries must produce the displayed totals instead of writing arbitrary `hours_worked` values.
- Dev-only `--seed` loading must be repeatable and must not modify the ordinary user database. Document the exact working pnpm/Tauri invocation once M0 validates argument forwarding.
- Approved fixture: September 11, 2025 at 13:42 America/New_York. Completed entries and task totals are coherent at that snapshot. The two illustrated active timers are deferred to M2, with no timer schema/state in M0. Production uses the real clock.
- `pnpm dev:seed` selects `signal-dev-september-2025.db`; ordinary `pnpm tauri dev` selects `signal.db`. Native Rust also rejects seeding outside a debug `--seed` process. Seed writes and their marker commit in one transaction; subsequent loads preserve existing records/settings.
- Mock counters and summary prose are inconsistent. The full reconciliation is in `docs/verification/M0.md`; no extra tasks were invented. Derived Today badge is 3.
- Actual attachment files are explicitly deferred to M1. Reference filenames are documented without fictitious database paths or sizes.

## Milestones

### M0 — Scaffold

M0 is complete. `pnpm check`, six meaningful foundation tests, production frontend build, Rust checks/native launch, seed loading/reloading, ordinary-data isolation, SQLite integrity, local fonts, initial/minimum dimensions, geometry persistence/maximized restart, OFF close/relaunch, and tray hiding have passed. On September 12, 2026, the user manually confirmed both tray Open and Quit work, completing the final check that native automation could not perform. Implementation is committed as `1886045`. See `docs/verification/M0.md` for evidence and review paths. This records the M0 review boundary. M1 was subsequently authorized; its completed verification and user acceptance are in `docs/verification/M1.md`.

- Validate local pnpm/Node, Rust, Windows build prerequisites, and WebView2; initialize the fixed stack and scripts (`check`, `test`, `tauri`).
- Build the token inventory: exact colors and tints, border variants, type scales/weights, spacing, geometry, radii, shadows, focus ring, opacity, and 120ms transitions. Bundle fonts and use dark mode only.
- Register the SQL plugin and complete initial schema migration; add repository boundaries and startup loading/error handling using approved copy.
- Add deterministic debug-only seed loading and the single approved September 2025 midday clock. Extra scenarios deferred.
- Build the shell: 52px header, Signal wordmark, Today badge, Your Day tab, project tabs with 8px dots and active underline, add-project affordance, 220px search/palette stub, and Settings entry.
- Configure 1600×960 initial window and 1200×760 minimum. Use a README-permitted titlebar approach, with window geometry persisted only in the store plugin.
- Wire the close-to-tray setting and hide/restore path, default OFF. Provide the minimum Open/Quit tray behavior needed to recover a hidden M0 window; full menu and notifications remain M6.
- Stubs: destination view bodies, project creation, palette results, all later feature actions, and notification delivery. Identify their scope clearly in verification notes.
- Verify: launch seeded app → inspect header/tabs → Settings → toggle Keep running in the system tray → close → restore through tray → turn toggle off → close and relaunch. Check minimum sizing and font loading offline.
- Intended commit: `feat(scaffold): initialize Signal desktop shell and local data foundation`.

### M1 — Projects & Board (`#1b`, `#3a`)

Execution status: complete. All four task/spec pairs are implemented, automated checks pass, and the user confirmed all manual acceptance checks on September 12, 2026. The milestone commit and push are authorized. Evidence and remaining later-phase stubs are recorded in `docs/verification/M1.md`. M2 execution was subsequently authorized; see its status below.

- Implement project create/read/update/delete and sort using the approved edit/delete interaction; offer cyan/lime/magenta/violet by default, excluding orange.
- Build Board/Week/Notes sub-bar, specified filter/group affordances, 5 equal status columns (14px gaps), database counts, and cards with exact chip/due/progress/blocked/done/selected treatments.
- Drag cards between columns with persisted status and `done_at` updates. Use a translucent destination card preview that becomes solid on drop, with a faded source and failure restoration (user review refinement, September 12, 2026). Resolve card ordering and blocked-reason editing before implementation.
- Render the 09–18 meetings strip, positioned pills, project-colored label, and live now tick. Meeting editing arrives in M4.
- Implement the 600px new-task modal, all specified fields, alert chips, tags, subtasks, markdown/drop attachments, submit shortcuts, and external-link paste detection. URL alone fills provider/ID; a title is only filled from user-supplied pasted title text.
- Implement the 820px task detail modal with `1fr 280px` rails, breadcrumb/link, editable fields, subtask progress, sanitized markdown, attachment copying, tags, priority, due/alerts, linked meeting, and time card.
- Stage Start/Log controls for M2; create-and-plan navigation reaches the Your Day placeholder until M3. Do not claim either workflow complete in M1.
- Verify: Atlas Migration → Board → ATL-482 → inspect/edit details → attach a local file → reopen → Esc → New task → paste supported links/title → create → drag through statuses → edit/reorder a project → restart and verify persistence.
- Intended commit: `feat(board): add projects tasks and task detail workflows`.

### M2 — Timers & time entries

Execution status: **complete and manually accepted September 13, 2026.** All three task/spec pairs and follow-up timer discovery/Board overflow corrections are implemented, with 52 frontend and 18 Rust tests passing. Native and user verification, including resolved PNG default-viewer behavior, are recorded in `docs/verification/M2.md`. Completion commit: `feat(timers): persist concurrent timers and logged time`. No push authorized. M3 kickoff is now authorized.

- Implement a persisted one-timer-per-task state machine supporting concurrent tasks and the approved pause/resume semantics.
- Derive elapsed time from persisted timestamps; repaint each second without accumulating tick drift. Restore accurately after restart and while hidden.
- Display running elapsed time as `HH:MM:SS` (for example `00:02:17`), with visibly advancing seconds. Explicit user preference recorded September 12, 2026; implemented in M2.
- Stop atomically appends a completed time entry and clears/finalizes timer state exactly once; manual Log writes entries through the same time accounting boundary.
- Update derived hours, progress displays, header singular/plural timer chip, and tray tooltip. Associate entries with a block when started from a block in M3.
- Unit tests: simultaneous tasks, duplicate start/stop, elapsed time after restart, paused intervals, fractional minutes, and approved clock/sleep behavior.
- Verify: task A → Start → task B → Start → check header/tray → pause/resume → quit/relaunch → Stop both → verify individual entries and totals → Log time manually.
- Stubs: planner timer controls and full tray timer actions remain M3/M6.
- Intended commit: `feat(timers): persist concurrent timers and logged time`.

### M3 — Your Day (`#2a`, `#2b`, `#3e` right)

Execution status: **complete and manually accepted September 13, 2026.** D1–D8 and all three task/spec pairs are implemented, including the accepted editor, Due soon, completed/empty-day and Quick add refinements. Latest full checks pass 79 frontend and 27 Rust tests; final affected frontend checks, type/build/Rust checks and standalone Tauri debug build pass. The user verified and approved M3 and authorized commit/push. See `docs/verification/M3.md` for evidence and historical automation limitations. M4 kickoff is now authorized.

- Build the date/scope sub-bar, 24h scrolling planner at 60px/hour, 64px gutter, initial 07:00 scroll, past shade, orange now line, fades, and 320px sidebar with 20px gap.
- Implement drag selection with 15-minute snap, selection preview, 360px task picker, search, keyboard navigation, cancellation/removal, and task/break/lunch/focus creation.
- Implement move/resize, done toggles, linked task detail, active timer pause/stop, run-over indicator, and the specified offer to move a task to Done.
- User review refinement (September 13, 2026): provide ±15-minute Start/End controls, 30/60/90/120-minute duration shortcuts, and a date picker beneath the time fields. Saving an explicit date change moves the same block and opens its destination day, preserving task due dates and timer/history associations.
- Project task/meeting visuals from their own colors; neutral break/lunch; finalize unspecified focus treatment before styling it.
- Implement pure greedy lane packing for concurrent planner items, equal lane widths and 4px gaps, with content collapsed at 4+ lanes. Resolve the README's three-lane wording gap before implementation.
- Implement Quick add durations 30/60/60, due-soon drag-in sized to remaining estimate capped at 2h, unscheduled rows, leftovers/done lists, and approved footer statistics.
- Implement leftovers banner Carry all/Pick/Dismiss and individual Carry, preserving source links and the approved carry duration, placement, and duplicate handling.
- Implement the empty-day hint and approved Plan from due dates/Copy last Monday behavior. First-launch onboarding remains M7.
- Unit tests: adjacent/nested/chained overlaps, deterministic ties, independent overlap groups, 4+ lanes, carry scheduling around meetings, multiple carry items, day bounds, repeat carry, and no-space behavior.
- Verify: Your Day morning scenario → select 09:00–10:30 → choose task → move/resize → overlap multiple items → run timer → complete block → scope to Atlas → carry selected leftovers → open empty day → exercise planning actions.
- Stubs: Daily summary opens its placeholder until M5; meeting editing remains M4.
- Intended commit: `feat(planner): add daily scheduling overlap lanes and carry-over`.

### M4 — Today digest & Week (`#2d`, `#3d`, `#3b`)

- Build Today title/date/counts, seven-day strip, Overdue/Due today/Tomorrow, meetings with next Join action, per-project Hours this week, and Plan links into Your Day.
- Build project Week navigation, seven due-date buckets, 200px later/no-date column, compact task cards, dashed meeting pills/placeholders, overdue counts, and today wash.
- Drag to change due date using the approved treatment of time-of-day; synchronize Board, Today, Week, and Your Day queries.
- Implement new meeting (460px) and meeting detail (760px, `1fr 260px`) with agenda, notes, linked tasks, time/duration, external link, reminder, and Show in Your Day.
- Implement none/daily/weekly recurrence with approved occurrence-edit/delete, timezone, attendance, and visibility semantics; project recurring meetings without duplicate blocks.
- Verify: Today → Plan an unplanned due task → Week → drag a card to Friday → task modal shows changed date → create/edit meeting → link ATL-482 → set weekly repeat → check Today/Week/Your Day occurrence → Join opens its external URL.
- Stubs: native meeting reminders remain M6.
- Intended commit: `feat(agenda): add Today Week and meeting management`.

### M5 — Daily summary (`#2c`, `#3c`)

- Implement Managers & summaries settings: one manager per project, send-at, Brief/Detailed, automatic-draft preference, default-mail setting, and exact helper copy. SMTP is deferred.
- Implement pure per-project summary generation using the approved historical/attendance model: Yesterday = completed yesterday blocks + tasks becoming Done yesterday + attended meetings; Today = planned blocks in order + meetings; Blockers = blocked tasks/reason/person/since-date.
- Emit Slack mrkdwn with project/date, Yesterday/Today/Blockers, bullets, and linked external IDs; Brief omits hours, Detailed appends hours and subtask progress.
- Implement 640px summary modal, manager-qualified tabs, no-manager note, To row, source toggles, editable body, Regenerate, Copy for Slack, and `mailto:` handoff.
- Persist drafts and sent summaries according to the approved sent-state semantics; opening a mail client cannot itself prove email delivery.
- Unit tests on seed data: project isolation, date boundaries, source switches, deterministic ordering, overlapping source deduplication policy, no items, missing manager, blockers, Slack escaping/links, Brief vs Detailed.
- Verify: Settings → Managers & summaries → configure manager/tone → Your Day → Daily summary → toggle sources → edit → copy and inspect clipboard → regenerate → Send email → verify saved history using the agreed sent action.
- Stubs: automatic timed drafting/reminders are connected to the Rust scheduler in M6; SMTP remains later work.
- Intended commit: `feat(summaries): generate and review project daily updates`.

### M6 — Tray & notifications (`#2e`)

- Complete Notifications settings: tray, toasts, block nudge, sound, default offsets, meeting reminder, 09:00 overdue nudge, weekday 17:15 unsent-summary reminder, and 22:00–08:00 quiet hours.
- Complete tray Open, running timer start/stop actions, Today's summary, and Quit. Retain explicit opt-in for close-to-tray unless the user approves a different M6 default.
- Run scheduling in Rust while hidden; reload relevant database changes, expand recurrence, and persist notification delivery/snooze state after approval of missed-event behavior.
- Fire task offset, meeting, block-start, overdue, and summary notifications; connect manager send-at to automatic draft creation without automatic sending.
- Route native toast activation to the correct window/item. Use in-app toasts for unsupported OS actions when visible: Snooze/Mark done, Start timer/Push 15m, and the specified reminder actions.
- Verify with a controlled dev clock and safe local fixtures: enable tray → close → fire each reminder → click toast → correct item focused → snooze/push → respect quiet hours → restart without duplicate delivery → tray Quit exits.
- Record actual Windows notification capability limits and native rendering differences; do not promise a custom pixel-identical OS toast.
- Intended commit: `feat(notifications): add tray scheduling and native reminders`.

### M7 — Polish

- Complete command palette: jump to project/task, new task, start timer, daily summary; complete keyboard shortcuts (`n`, platform command-K/Enter/Shift-Enter, Esc, picker arrows/Enter) without intercepting typing in inputs.
- Complete focus management, restoration, disabled/hover/error treatments, 120ms fade/scale transitions, empty states, and first project/import flow (`#3e`).
- Implement Jira/Asana/ClickUp CSV import locally, with approved field mappings, status conversions, duplicate handling, preview/error copy, and atomic commit. Request representative exports before finalizing parser behavior.
- Implement Data & backup SQLite export/import with a consistent database snapshot, schema/integrity validation, and the approved replacement/recovery behavior. Resolve attachment backup expectations explicitly.
- Complete approved behavior for previously unspecified Notes/Filter/Group and remaining settings destinations, or agree their scope before claiming polish complete.
- Run final visual comparisons for every accepted screen at matched viewport sizes; check 1200×760 minimum and 1600×960 reference size, all specified states, offline operation, and clean console.
- Verify: first launch → create project/import CSV → palette jump/new task/start timer/summary → export database → change local data → import approved backup → verify restored entities and agreed attachment behavior.
- Intended commit: `feat(polish): complete keyboard onboarding import and backup flows`.

### M8 — Assisted meeting and task intake (proposed placement)

Feature requested September 13, 2026. Provide a dedicated screen that turns user-supplied screenshots, text files, or pasted notes into editable meeting/task candidates for a selected project. The user can supply instructions such as “Convert these into meeting entries for the Atlas Migration project” or “Build tasks based on these comments for the Website Refresh project.” This is roadmap scope, not an approved implementation specification or an addition to M3.

- **Input:** drop or choose a screenshot or text file, or paste text into a form. Include a project selector, a meeting/task choice, and a free-text instruction field. Resolve an absent or ambiguous project before saving; do not silently create a project from a name in the source.
- **Interpretation:** extract the best supported meeting/task candidates from the supplied content and instructions. For meetings, suggest title, date/time, duration, link, and agenda where available. For tasks, suggest title, notes, subtasks, due date, priority, and estimate where supported. Leave missing optional details blank or apply clearly identified existing defaults; do not invent links, deadlines, estimates, or meeting times as if they appeared in the source.
- **Review:** show the source beside editable candidate rows, with uncertain readings, assumptions, and missing fields identified. Let the user correct fields, change the target project, exclude candidates, and add details before creating selected records. Unclear dates, timezone, AM/PM, or recurrence must remain visible for resolution. A meeting with no valid start/duration stays a candidate until those required fields are supplied; optional details can be completed later in the normal meeting/task editor.
- **Creation:** create the reviewed selection through the existing native validation and transaction boundaries. Use repeat-safe request identities, report possible duplicates against existing records, and show exactly what was created. Canceling a preview creates no business records; retrying an interrupted creation must not duplicate them. Importing meetings must not also create duplicate planner blocks.
- **Follow-up:** link each created record to its existing task or meeting editor so the user can fill in more details. Refresh Board, Today, Week, and Your Day using the normal project/date rules.
- **Failure and recovery:** preserve editable candidates when interpretation or validation fails, allow partial source recognition to be reviewed, and report unreadable content or unsupported files clearly. Define draft/source retention and restart recovery at kickoff.
- **Boundary:** reuse M1 task creation and M4 meeting creation/recurrence validation. Keep M7's structured CSV importer distinct. This feature does not synchronize an external calendar, invite attendees, fetch linked documents, or execute instructions found inside imported content.
- **Verification:** screenshot plus project instruction → review meeting candidates → correct an ambiguous time → create selected meetings → open one to add details; repeat with a text file and pasted comments to create tasks. Test unreadable/partial input, missing dates, project isolation, duplicate/retry handling, cancellation, processing failure, persistence across restart, and ordinary offline use without the interpreter.
- Intended commit, after implementation and verification: `feat(intake): create reviewed meetings and tasks from screenshots and text`.

## Required milestone completion gate

1. Run `pnpm check` and `pnpm test`; report actual outcomes. Before domain tests arrive, clearly report that no domain tests exist, instead of adding meaningless tests to make a green count.
2. Launch `pnpm tauri dev`, verify the native window appears, and inspect startup/runtime errors. A successful Vite server alone does not satisfy this gate.
3. Open the seeded application and the accepted HTML at the same content size; compare the milestone's implemented screen geometry, color, font, spacing, copy, and states. Compare internal mock app frames, not the outer design-review canvas.
4. Verify persisted changes across restart, applicable offline behavior, and no console errors. M0 validates migrations on a fresh database and a second launch.
5. Record in `docs/verification/Mn.md`: implemented features, remaining stubs, exact seed command/scenario, exact click-path, command results, visual differences, and token-derived states. Hover is one surface step lighter; focus uses `0 0 0 3px rgba(87,211,234,.15)` and the specified accent border.
6. Commit only the milestone's intended files with its conventional-commit message; do not sweep unrelated user files into commits. Report the commit hash.
7. Stop for the user's review. Fix milestone issues before advancing. Do not describe a milestone as complete if required checks could not run.

## Decisions requiring answers

These questions identify gaps rather than granting permission to invent behavior. Resolve M0 items with plan approval; ask later items before their affected work begins.

### M0 decisions — resolved by user approval

1. One isolated debug `--seed` database, September 2025 midday snapshot only. Document inconsistencies and fixture assumptions; do not invent extra tasks. Attachment files deferred to M1.
2. Preserve all 13 tables and fields, opaque text IDs, UTC instants, local planner date/minutes, transactional hours cache. Additional timer/attendance/notification schema waits for its milestone; deletion behavior is not implemented.
3. Sora 600 hour labels, no Sora 500, local licensed WOFF2 assets.
4. Explicit destination/action stubs, native Windows titlebar above the 52px header, 1600×960 initial and 1200×760 minimum, store plugin for geometry only, default-OFF close-to-tray with Open/Quit recovery menu.

### Before M1–M3

- Define project/task deletion behavior and project edit/sort access; within-column task ordering; Filter options; whether Group is fixed to Status; where blocked reason/person/since are edited.
- Define pasted title format, task save/cancel behavior, local-only task IDs, date-only task deadlines, and the manual Log dialog fields/copy. Approve fixture attachment files if real attachments remain unavailable.
- Define pause vs stop, whether time continues through system sleep/app shutdown, handling clock corrections, and minute rounding. Proposed elapsed accounting includes shutdown time for running timers and excludes paused intervals.
- Resolve carry duration: README says identical-length blocks although chips say remaining work. Define three-lane content (README says below three/full and four+/title only), focus-block visuals, done-block/timer interaction, no-estimate drag duration, next-free-slot end-of-day behavior, scope of occupied time, and Planned/Logged/Blocks counting with parallel work and meetings.
- Define Plan from due dates and Copy last Monday scheduling/duplicates, and whether a dismissed/carried leftover can reappear the next day.

### Before M4–M7

- Define recurring meeting timezone/DST behavior, occurrence vs series edits, attendance UI, meeting notes appearing in linked-task activity, and Week drag preservation of due time. The Week mock shows a dated subtask although the subtask schema has no date; confirm whether that is display-only reference content or a required data-model addition.
- Define summary deduplication, weekend meaning of Yesterday, source-toggle mapping for meetings, since-date history, treatment of non-task blocks, and sent status after Copy/`mailto:`. The README says tabs only for manager projects, so the mock's self-summary CLI tab does not override it. Confirm whether separate project Notes/self summaries are required.
- Define missed-notification handling after sleep/quiet hours, snooze persistence, and whether close-to-tray stays opt-in in M6. Confirm how the UI records sent summaries so reminders can stop reliably.
- Supply/approve CSV formats and mappings, duplicate policy, import preview/error copy, backup replacement behavior, and whether attachment files must travel with the SQLite export.

### Before M8 — assisted intake

- Confirm milestone placement and provide representative screenshots/text for extraction and review acceptance. There is no accepted handoff screen for this new feature; design its screen and review flow at kickoff.
- Choose local OCR/model processing versus an optional external model service, including installation or configuration, credentials if applicable, expected cost, source-data handling, and behavior when processing is unavailable. Preserve the offline core; do not select or integrate a provider silently.
- Define supported image/text formats, size limits, multi-file handling, relative-date reference date/timezone, recurrence suggestions, candidate defaults, duplicate matching, and whether candidate/source drafts persist across restart or become record attachments.

## Approval boundary

M0 was approved with the decisions above. Stop after its verification and conventional commit for the user's review. Later unresolved decisions are raised at their milestone; M1 required separate approval, which the user subsequently supplied together with D1–D8 and explicit execution of all four task/spec pairs.
