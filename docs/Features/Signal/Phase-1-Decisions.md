# Approved M1 decisions

Approved and date-locked September 12, 2026. Status: **D1–D8 approved without amendments.**

These decisions resolve M1 gaps in [PLAN.md](../../../PLAN.md). They were explicitly approved for this kickoff, rather than inherited from M0 approval. The [kickoff overview](Tasks/Phase-1-Overview.md) records verified implementation facts and the task split.

## Approved decisions

| ID | Approved behavior | Material consequence |
| --- | --- | --- |
| D1 | Permanent task/project deletion after a confirmation showing affected records. A project deletion removes its tasks, meetings, summaries, and task-owned blocks/time entries/attachments. Detach links to surviving records. No archive or undo in M1. | Requires a precise transactional deletion graph plus recoverable cleanup of owned files. Source attachment files are never deleted. Retain global tags and unrelated projects, records, neutral blocks, and settings. |
| D2 | A project menu provides Edit, Move left/right, and Delete. Create/edit offers cyan, lime, magenta, violet. | Gives project management a defined entry point without adding another screen or introducing orange into the default palette. |
| D3 | Persist manual task ordering in each column; cross-column drops update status and position. Filter by text, priority, tag, and due state. Group remains fixed to Status. Week/Notes remain labeled stubs. | Adds task ordering to the schema. Final specs must normalize full-column positions and explain filtered drops and count semantics. |
| D4 | Edit optional blocked reason/person in task details. Record blocked-since automatically on entering Blocked; clear it on leaving. | Adds a UTC blocked-since field. Existing blocked records with unknown history must not get an invented historical date. |
| D5 | New tasks use Create/Cancel and prompt before discarding a changed draft. Task details save each field on commit/blur; notes use Edit → Save/Cancel. Failed saves keep the editor and error visible. | Requires separate draft/persisted state, serialized writes, and clear failure behavior; closing must not silently lose an unsaved edit. |
| D6 | Opaque internal IDs remain authoritative. Tasks without a provider display “Local task.” Choosing a due date shows an editable time initially set to 17:00 local, then stores the confirmed instant in UTC. | No fabricated provider ID and no hidden date-only conversion policy. Existing timestamps remain unchanged. The 17:00 value is a visible editable default, not an undisclosed interpretation of all date-only data. |
| D7 | Accept a provider URL alone or a Markdown link `[title](URL)`. URL-only fills provider/ID; never fetch a remote title or overwrite a title already entered. | Local parsing only. Invalid/unrecognized URLs need honest feedback; arbitrary page fetching and provider integrations remain outside scope. |
| D8 | Add/open/remove local attachment copies. Approve clearly labeled synthetic `gateway-arch.pdf` and `latency-p95.png` fixture files for M1. Start/Log, meeting editing, and create-and-plan's destination retain later-milestone stubs. | Fixture files must be valid, explicitly synthetic, and isolated from ordinary attachments. M1 must not pretend timer, meeting editing, or scheduling workflows exist. |

## Resolution record

On September 12, 2026, the user stated: “Yes I approve D1-D8, please run /phase-kickoff to create the task/spec pairs.” All eight decisions above are therefore approved without amendments. The current authorization is to finish the planning documents; it does not instruct this kickoff to change production code.

## Decisions deferred to their owning milestones

Pause/stop, sleep/shutdown timer accounting, manual Log fields, planner lane/carry/focus rules, recurring meetings, attendance, summaries, notification delivery, imports, and backups are not M1 decisions. Do not enlarge this kickoff to resolve or implement them.
