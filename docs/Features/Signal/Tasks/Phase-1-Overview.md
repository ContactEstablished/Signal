# M1 kickoff — Projects & Board

Date: September 12, 2026. Status: **M1 complete; all four task/spec pairs implemented and manual acceptance confirmed by the user. Commit and push authorized.** See [M1 verification](../../../verification/M1.md).

The grounding and kickoff validation below are historical planning evidence; the linked verification report records the completed implementation.

## Source of truth

- [Root roadmap](../../../../PLAN.md), especially M1, persistence constraints, and the milestone completion gate. It remains the sole roadmap; this kickoff does not replace it.
- [Design handoff](../../../../_design/design_handoff_signal/README.md), read in full during grounding. Accepted #1b defines the Board and task details; #3a defines task creation.
- [Accepted standalone reference](../../../../_design/design_handoff_signal/Signal.standalone.html). Its embedded HTML for #1b and #3a was inspected during this kickoff. M0's earlier rendered inspection is recorded separately; no new rendered comparison is claimed here.
- [Approved M0 decisions](../../../decisions.md) and [completed M0 verification](../../../verification/M0.md).
- [Approved M1 decisions](../Phase-1-Decisions.md). The user explicitly approved D1–D8 without amendments on September 12, 2026.

## Goal and boundary

Prepare implementation-ready work for project management, a five-column Board, task creation/details, and local attachments. The eventual implementation must match the accepted references while deriving counts and hours from records. It must remain usable offline, persist changes across restart, and preserve M0's native shell, local fonts, SQL-plugin storage, isolated fixture, window store, and tray behavior.

M2 timer state, Start/Log behavior, M3 planner scheduling, M4 Week/meeting editing, M5 summaries, M6 notifications, and M7 imports/palette results remain outside M1. Board/Week/Notes navigation is included; Week and Notes bodies remain clearly identified previews. Create-and-plan may create a task and navigate to the existing Your Day preview, but must not claim a block was scheduled.

## Verified starting point

Grounded on branch `main`, HEAD `83fc039` (`Separate design files`). The tree was clean before the initial grounding turn. At the approved drafting turn, `git status --short` showed the existing untracked `docs/Features/` overview/decision brief from that grounding; these are preserved and updated within the requested kickoff. Preserve the user's design-file commit and every later user/concurrent change; do not revert, stage, or commit them as part of this kickoff.

| Existing source | Verified fact | Consequence for M1 |
| --- | --- | --- |
| `src/App.svelte` | Runes own `data`, `active`, `selectedProject`, `load`, `stub`, `keyboard`, and `toggleTray`; project destinations and Add project are previews. | Integration must replace only M1 previews and preserve Settings/tray/palette behavior. |
| `src/lib/db/client.ts` | `openFoundation()` selects the native database, loads the fixture conditionally, queries project/counter records, and loads the tray preference. `Project` currently exposes only id/name/color. | Full Board/task read models and typed mutation boundaries do not exist yet. |
| `src-tauri/src/db.rs` | `migrations()` registers version 1. Private `pool()` clones the SQL plugin's existing `SqlitePool`. Narrow commands handle runtime configuration, tray preference, and debug seeding. | Reuse this connection owner; introduce typed business operations rather than enabling arbitrary frontend writes. |
| `src-tauri/src/lib.rs` | `run()` registers the existing commands and SQL/store plugins, installs the tray, and restores the native window. | New commands/plugins need explicit registration without replacing M0 setup. |
| `src-tauri/capabilities/default.json` | Frontend permissions are core default, SQL load, and SQL select. | File and link operations need a deliberately scoped native boundary. |
| `src-tauri/migrations/0001_initial.sql` | All 13 handoff tables exist with foreign keys, constraints, indexes, a derived time view, and transactional hours-cache triggers. No cascading product deletion is defined. | Preserve migration 1; approved additions require a new migration and both fresh/upgrade verification. |
| Same migration | Projects and subtasks have `sort_order`; tasks have neither ordering nor blocked-since fields. | D3/D4 materially affect the migration; these are planned additions, not existing capabilities. |
| `src/lib/seed.ts` | `FIXTURE_NOW`, `FIXTURE_ZONE`, `FIXTURE_DATABASE`, `FIXTURE_VERSION`, `seedTables`, and `loadSeed()` define the isolated September 11, 2025, 13:42 New York fixture. | Keep the existing repeatable marker behavior. M1 attachment fixtures must work on an already seeded database without reseeding business records. |
| `src/lib/domain/dates.ts` | `localDate()` and `todayCount()` use local calendar boundaries; no editable deadline conversion API exists. | D6 needs explicit UI normalization and timezone tests. |
| `src/styles/tokens.css`, `fonts.css`, `global.css` | Local font faces and handoff dimensions/colors/state tokens exist; global controls currently cover buttons and inputs. | Reuse tokens and extend form/focus treatment for M1 controls; never add Sora 500. |
| `tests/unit/foundation.test.ts` | Six tests cover the actual initial SQL migration, fixture totals, transactional cache behavior, constraints, independent databases, and local dates. | Retain this baseline; add meaningful mutation/file/domain tests, not tests that merely count new components. |
| `package.json`, `src-tauri/Cargo.toml` | Existing pnpm scripts and Rust dependencies support M0. Markdown parsing/sanitization and native file/dialog/link integration are not installed as application features. | Dependency and lockfile ownership must be assigned explicitly. |

No Board, task editor, project editor, repository directory, component directory, or view directory exists yet. Paths for those capabilities in PLAN.md are intended structure, not verified implementation.

## Task/spec pairs and ownership

All pairs below are executable planning contracts based on the approved decisions. Their new paths/symbols remain planned deliverables, not existing implementation. Each task's Exact Scope is its exclusive write set; specification 1-1 is the canonical shared DTO/command contract.

| Task and paired specification | Cohesive responsibility | Dependency | Ownership boundary |
| --- | --- | --- | --- |
| [Task 1-1](Task-1-1.md) / [Spec 1-1](../ImplementationSpecs/ImplementationSpec-1-1.md) | Persistence, migrations, project/task mutations, ordering/deletion, attachment lifecycle, native exit guard, typed contracts. | M0 complete; D1–D8 approved. | Native modules/registration and narrow window/tray exit interception, migration 2, typed APIs, manifests/lockfiles, persistence/file tests, db/client.ts. |
| [Task 1-2](Task-1-2.md) / [Spec 1-2](../ImplementationSpecs/ImplementationSpec-1-2.md) | Project controls, Board/cards/filtering/drag, project-scoped meetings strip. | 1-1 contracts available. | Board/project/strip components, Board view, board.ts and its tests. No App.svelte writes. |
| [Task 1-3](Task-1-3.md) / [Spec 1-3](../ImplementationSpecs/ImplementationSpec-1-3.md) | New-task/details, fields/subtasks/tags/alerts/notes/links/attachment presentation and errors. | 1-1 contracts available. | Task components and task-draft/links/markdown/deadlines helpers/tests. No Board or App.svelte writes. |
| [Task 1-4](Task-1-4.md) / [Spec 1-4](../ImplementationSpecs/ImplementationSpec-1-4.md) | Shell/controller/clock/drop integration, synthetic fixtures, final runtime/visual verification and handoff. | 1-1, 1-2, 1-3. | App.svelte, app state, clock/mutations helpers/tests, native drop adapter, token/global CSS, seed hookup/assets, tauri.conf.json, README and M1 verification. |

Tasks 1-2 and 1-3 can proceed independently after Task 1-1 publishes its types/dependencies/native contract; Task 1-4 integrates them. Task 1-4 owns shared-file integration, while fixes in another owner's files return to that owner. No shared Modal module is required: project/task components own their native HTML dialogs. Runtime checks for component tasks require mounting by Task 1-4; successful component compilation is not a runtime verification claim. During this kickoff, independent Board and task-editor drafting was delegated; the primary agent performed grounding and final review.

Technical consequences are specified explicitly: task revisions reject stale saves; a durable attachment-operation table supports cleanup; an editor exit guard protects drafts on native Close and tray Quit; an OS lock protects each selected database's managed-file lifecycle. These support D1/D5/D8 and do not add timer, attendance, or notification state. Migration 1 remains unchanged.

## M1 scope coverage

Reviewed against the M1 roadmap and complete handoff on September 12, 2026. The existing four pairs are retained and revised; this review does not create a second competing task set. The original roughly 12-hour target applied to M0 only; this kickoff neither transfers that estimate to M1 nor adds work to fill a time target.

| M1 requirement | Primary task/spec | Integration and boundary |
| --- | --- | --- |
| Project CRUD, colors, manual project sort | 1-1 persistence; 1-2 controls | 1-4 connects header Add and project selection; preserve manager fields. |
| Five-column Board, card states/counts, filters and manual drag order | 1-2 | 1-1 persists order/status; 1-4 mounts and refreshes badge. |
| Board/Week/Notes sub-bar | 1-2 | 1-4 keeps navigation above every project subview; Week/Notes bodies remain previews. |
| Project meetings strip and now tick | 1-2 | 1-4 supplies fixture/real clock; no meeting creation, editing, or recurrence expansion. |
| New-task fields, paste parsing, alerts, shortcuts | 1-3 | 1-1 atomically creates records; 1-4 owns routing and drop subscription. Alert delivery remains M6. |
| Task details, notes, children, links, blocked fields and deadlines | 1-3 | 1-1 validates/persists; 1-4 serializes saves and protects close/navigation. |
| Local attachment add/open/remove and synthetic fixture assets | 1-1 lifecycle; 1-3 UI | 1-4 owns assets, native drop routing, and repeatable fixture hookup. |
| Time display and create-and-plan affordance | 1-3 | Hours remain derived and read-only; Start/Log remain M2 previews and planning remains M3. |
| Migration, offline/native/restart/visual gate and handoff | 1-4 integrates evidence from all pairs | Record actual results, conventional M1 commit during execution, stop before M2. |

The scope review closed concrete specification gaps: ordering on first seed after migration 2; transaction reply preparation before commit; batch attachment compensation and explicit cleanup warnings; staging/retry callbacks for both task dialogs; one mode-specific editor close protocol; and persistent sub-bar navigation on preview destinations. Unit/native acceptance checks were extended accordingly. These are refinements of D1–D8, not newly approved feature work.

## Required execution evidence

Commands verified as existing/configured in the repository, run from its root:

```powershell
pnpm check
pnpm test
pnpm build
. ./scripts/dev-env.ps1
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
pnpm dev:seed
```

Use `pnpm tauri dev` for ordinary data. Quit the current instance before switching launch modes. `pnpm dev` alone is not native verification. M0 already verified the seed alias; its explicit expansion is `pnpm tauri dev -- -- -- --seed` on the recorded pnpm version.

The eventual M1 gate must include:

1. Fresh migration and upgrade of an existing M0 fixture, without changing its task/time records or ordinary data.
2. Project create/edit/reorder, then Board → ATL-482 → editable details → local attachment → reopen.
3. New task → supported pasted link/title → creation → status/ordering changes → restart and persistence checks.
4. Deletion and attachment failure/recovery cases according to the resolved policy, exercised only on purpose-created verification records.
5. Native startup and runtime console checks, keyboard/focus behavior, and offline/local-resource behavior.
6. Matched 1600×960 and minimum 1200×760 visual review of #1b/#3a, including blocked, done, selected, empty, loading, and error states.
7. Actual results and remaining stubs in `docs/verification/M1.md`; conventional milestone commit; stop before M2.

No implementation commands, tests, migrations, or native launches were run during this planning-only kickoff. Prior M0 passes remain attributed to M0's verification record.

## Reference reconciliation and risks

- The Board mock labels Backlog 4 and Done 3 while showing three and two cards. Retain actual Atlas counts 3/3/2/1/2; do not invent hidden tasks.
- The accepted Board mock shows two meetings, including Standup. The M0 fixture assigns Standup to Website Refresh; Atlas has Auth cutover sync on September 11. Project Board queries must stay project-scoped, so Atlas's strip has one fixture meeting that day.
- The accepted Board shows an earlier 6.5h auth total and now 12:06. M0's approved snapshot is 13:42 with auth 8h and runbook 2h. Preserve that coherent fixture; compare visual treatment without overwriting recorded hours or clock to force matching text.
- D8 approves synthetic `gateway-arch.pdf` and `latency-p95.png` for implementation. They must visibly identify themselves as synthetic; filenames must not imply original attachments were recovered.
- Database commits and filesystem changes cannot share one SQLite transaction. Specification 1-1 defines staging, compensation, cleanup retry, missing-file errors, and ordinary/fixture file-root isolation; implementation must prove these failure paths.
- Deleting task-owned blocks can affect surviving `carried_from_block_id` links. Deletion must enumerate this graph, preserve unrelated records, and avoid accidentally relying on disabled foreign keys.
- Reordering while filtered can corrupt hidden positions unless the API operates on the full column order. Specifications 1-1/1-2 define insertion before a visible target or at the end of the full column.
- Inline saves must serialize or reject stale writes; failed edits must remain visible. Local due-time input must handle invalid/nonexistent times rather than silently shifting them.

## Kickoff validation and execution prerequisite

Final kickoff validation on September 12, 2026 passed: four exact task/spec pairs; all eleven required sections in every task; all local document links resolve; no overlapping ownership across the 66 paths in the four Exact Scope sections; no stale pending-decision markers or trailing whitespace. Existing insertion files and installed native drop declarations were rechecked. `git status --short` shows only `docs/Features/`; no tracked production file changed. These checks validate planning documents, not M1 implementation. No stage, commit, migration, package installation, or application test was performed.

The planning-only restriction applied to the original kickoff. The user subsequently authorized all four implementation pairs, confirmed all manual acceptance checks, and authorized committing and pushing M1. No unresolved M1 product decision remains. M2 has not begun.
