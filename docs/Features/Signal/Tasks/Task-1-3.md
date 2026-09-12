# Task 1-3 — Task creation and detail editors

Status: complete (September 12, 2026). Implementation and automated checks are complete; the user confirmed all manual acceptance checks and authorized the milestone commit and push. See [M1 execution evidence and user sign-off](../../../verification/M1.md).

## Source of Truth

- [Phase overview](Phase-1-Overview.md), [approved decisions](../Phase-1-Decisions.md), and [paired specification](../ImplementationSpecs/ImplementationSpec-1-3.md).
- [Roadmap](../../../../PLAN.md), M1; [handoff](../../../../_design/design_handoff_signal/README.md), task detail/new task; [accepted reference](../../../../_design/design_handoff_signal/Signal.standalone.html), #1b/#3a.

## Initial Starting Point

M0 has local-date helpers, foundation SQL reads, schema, and six foundation tests. Task dialogs, draft helpers, Markdown sanitization, editable deadline conversion, and attachment workflows do not exist. Task 1-1's native contracts are planned dependencies; Task 1-4 will mount and verify the components in the native shell.

## Goal

Provide the complete M1 task creation/detail components with typed persistence callbacks, visible failures, safe local content handling, and explicit later-feature stubs.

## Exact Scope

Create only:

- `src/lib/components/tasks/{NewTaskDialog,TaskDetailDialog,TaskFields,SubtaskList,TaskNotes,AttachmentList,TaskTimeCard,TaskTags}.svelte`.
- `src/lib/domain/{task-draft,links,markdown,deadlines}.ts`.
- `tests/unit/{task-draft,links,markdown,deadlines}.test.ts`.

Include the 600px creation dialog, 820px detail dialog with 280px rail, all specified fields, subtasks, alerts, tags, notes, attachment presentation, task deletion entry, and supported paste behavior. Use owned native HTML dialogs and token-based local styles.

## Non-Goals

No App.svelte, Board/project UI, manifest, migration, native command, shared state/global CSS, or fixture-file edits. No timer/log records, planner blocks, meeting editing, notifications, remote provider fetching, or imports. Preserve unrelated/concurrent work and the existing kickoff documents; do not stage another task owner's files.

## Dependencies

Task 1-1 supplies DTOs, native operations, dependency installation, revisions, deletion fingerprints, and file lifecycle. Task 1-4 supplies serialized mutation callbacks, shared clock/zone, OS drop routing, refresh, and final mounting. These components must compile independently after Task 1-1; do not import a future shared Modal component.

## Step-by-step Work

1. Implement draft validation, link parsing, deadlines, and Markdown helpers against shared DTOs.
2. Build field/subtask/tag/notes/attachment/time components with labels and saving/errors.
3. Build NewTaskDialog: Create/Cancel, discard protection, staging, and create/create-and-plan shortcuts.
4. Build TaskDetailDialog: committed-field saves, explicit notes Save/Cancel, derived time, linked meeting preview, and confirmed deletion.
5. Test the real helpers and provide callback contracts to Task 1-4.
6. After integration, exercise native dialogs, failures, attachment operations, restart, and reference comparisons; report actual results.

## Test Expectations

Cover title-preserving normalization, supported/unsafe links, malicious Markdown, remote-image exclusion, empty/invalid estimates, dirty drafts, timezone gaps/ambiguities, and untouched timestamp preservation. Runtime checks cover rejection, duplicate submission, pending/dirty close, file failures, and confirmation focus. Mocked callbacks never prove native persistence.

## Verification Commands

From repository root:

```powershell
pnpm check
pnpm test
pnpm build
```

After integration:

```powershell
. ./scripts/dev-env.ps1
pnpm dev:seed
```

Keep runtime checks explicitly deferred until the dialogs are mounted. Do not claim them from compilation alone.

## Acceptance Criteria

- All task fields submit atomically once; create-and-plan navigates only after successful creation and never claims scheduling.
- Committed-field/notes saves follow D5; failed or stale writes preserve the attempted edit and visible error.
- Markdown cannot execute active content or load remote images; links use the native boundary.
- Date selection exposes editable 17:00 local time, validates gaps/ambiguity, and preserves untouched existing UTC instants.
- Dirty/pending/failed edits cannot silently disappear through Escape, backdrop, or close controls.
- Attachment UI uses real metadata/staging tokens, with no invented paths or source-file deletion.
- Both dialogs expose staging/retry/discard and one asynchronous close contract; committed cleanup warnings do not masquerade as failed saves.
- Time/subtask progress/tags/alerts/meetings derive from records; local tasks display Local task.
- Start/Log, meeting editing, and planning retain their stubs; dialogs match #1b/#3a and fit the minimum window.

## Review Checklist

- [x] Contracts and ownership match Tasks 1-1/1-4.
- [x] Revision conflicts, repeat submissions, close/discard and save failures are covered.
- [x] No broad frontend filesystem/SQL write capability or remote lookup was added.
- [x] Keyboard, focus and file-drop behavior remain coherent with the shell.
- [x] Unit and native evidence are distinguished; Task 1-4 owns final handoff/commit.
