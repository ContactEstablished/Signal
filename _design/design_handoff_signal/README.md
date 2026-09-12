# Handoff: Signal — personal Kanban + day planner

## Overview
Signal is a single-user desktop app for tracking work across several projects. Each project holds **tasks** and **meetings**. Tasks link out to Jira / Asana / ClickUp, carry a due date, an estimate and hours worked, tags, priority, subtasks, notes and attachments. The **Your Day** view is a vertical day planner: meetings appear automatically, the user drags time blocks and assigns a task to each (several blocks may run in parallel — the user runs multiple AI agents), checks blocks off, and runs timers that auto-log hours to the task. A **Daily summary** button drafts a per-project "Yesterday / Today / Blockers" message in Slack formatting for that project's manager. The app lives in the system tray and fires native toast notifications when the window is closed.

Target stack (user's choice): **Tauri 2 + Svelte 5 + TypeScript**, **SQLite** local store (via `tauri-plugin-sql` or `rusqlite`), OS notifications via `tauri-plugin-notification`, tray via Tauri's `TrayIcon`.

## About the design files
`Task App Explorations.dc.html` (and `Signal.standalone.html`, a self-contained copy) are **design references built in HTML** — static hi-fi mockups showing intended look and behavior. They are not production code. Recreate them as Svelte components using the tokens and specs below. The file is a review canvas with three "turns"; **only the Signal direction is the target design**:

- Turn 3 (top): `#3a` new-task modal · `#3b` meeting modal (detail + blank) · `#3c` Settings → Managers & summaries · `#3d` project Week tab · `#3e` empty states
- Turn 2: `#2a` Your Day (morning, popover) · `#2b` Your Day (mid-day, timers, lanes) · `#2c` Daily summary panel · `#2d` Today digest · `#2e` Settings → Notifications + tray toasts
- Turn 1: `#1b` project Board with task modal (target). `#1a`, `#1c`, `#1d` are rejected explorations — **ignore** their palettes/layouts, but `#1d`'s content (settings copy, toast copy) was ported into `#2e`.

Open the file in a browser; ids are anchors (`…html#2b`).

## Fidelity
**High-fidelity.** Colors, type, spacing and copy are final. Recreate 1:1. Icons are deliberately text glyphs (`○ ▪ ◌ ✓ ↗ ⋮⋮`) — swap for a consistent 16px icon set (Lucide is a good fit) keeping the same meaning.

---

## Design tokens

### Color (dark theme only)
| Token | Hex | Use |
|---|---|---|
| `bg` | `#0f121a` | App background |
| `bg-raised` | `#12161f` | Header bar, planner canvas, settings sidebar, modal body |
| `surface` | `#171b26` | Cards, list rows |
| `surface-2` | `#1a1f2b` | Inputs, inner chips, search field |
| `surface-3` | `#1f2432` / `#242a3a` | Selected segment, ID chips, progress track |
| `border` | `rgba(255,255,255,.07–.08)` | Card/section borders; `.10–.12` for inputs & popovers |
| `text` | `#e2e6ee` | Primary text (never pure white) |
| `text-2` | `#c3c9d6` | Body copy in notes |
| `text-muted` | `#8d95a8` | Secondary labels |
| `text-faint` | `#4c5568` | Hints, hour labels, disabled |
| `accent` | `#57d3ea` (cyan) | Primary action, focus, selection, **Atlas Migration** project |
| `lime` | `#c1ea55` | Done / timer running / **Website Refresh** |
| `magenta` | `#f279c9` | **Home Renovation** |
| `violet` | `#b48cff` | **CLI** |
| `warn` | `#ff9f52` (orange) | Due today / overdue / leftovers / "now" line — **reserved, never a project color** |
| `danger` | `#ff7a7a` | Destructive (Delete) |
| Primary button text on cyan | `#04202a` | |
| Button text on lime | `#172000` | |
| Badge text on orange | `#2a1400` | |

Project colors are user-picked from the palette `cyan, lime, magenta, violet, orange(#ff9f52 allowed only if user insists — default excludes it)`. Tints: `rgba(color, .08)` meeting fill, `.10–.14` task fill / selected row, `.15` tag chip, `.35–.5` borders.

### Typography
- **Sora** (Google Fonts) 600/700 — app name, headings, section labels (uppercase, `letter-spacing:.04–.06em`, 10–12px), numbers, IDs, buttons.
- **DM Sans** 400/500 — everything else. Base 13px. Card titles 13.5px/500, line-height 1.35. Modal titles 22px/600 Sora, `letter-spacing:-.02em`. Page titles 17–22px Sora 600.
- Minimum text size 10px (chip labels), 11–11.5px for metadata.

### Spacing & shape
- Base grid 4px. Card padding `12px 14px`; modal padding `26px 28px`; page padding `24px`.
- Radius: chips 5px · inputs/buttons 7–8px · cards 10px · planner/panels 12px · modal 16px · toggle pill 10px.
- Shadows: modal `0 30px 80px rgba(0,0,0,.6)`; popover `0 24px 60px rgba(0,0,0,.6)`; toast `0 12px 32px rgba(0,0,0,.55)`. Cards have no shadow.
- Focus/active ring: `0 0 0 3px rgba(87,211,234,.15)` + 1.5px solid accent border.

---

## Screens

### 1. App shell (all screens)
- Window 1600×960 in mocks; min 1200×760. Header **52px**, `bg-raised`, bottom border.
- Left → right: traffic lights (Tauri: native titlebar or `decorations:false` + drag region), wordmark "Signal" (Sora 700 15px), tabs.
- Tabs (DM Sans 500, muted): `Today` (orange count badge = overdue + due today), `Your Day`, 1px divider, one tab per project with an 8px color dot. Active tab: text `#e2e6ee` + 2px bottom border (project color for a project tab, `#e2e6ee` for Today/Your Day). `+` adds a project.
- Right: status chip when timers run (lime tint, "2 timers running"), search field 220px (`⌘K` opens command palette), `Settings`.

### 2. Project → Board (`#1b`)
- Sub-bar: segmented `Board | Week | Notes`; `Filter`, `Group · Status` outline chips; primary `+ New task` (cyan).
- **Meetings strip** (card, `bg-raised`): label "MEETINGS" (magenta in mock → use **project color**), date + count; a 09–18h timeline with meeting pills positioned by time, orange "now" tick with time label.
- **Board**: 5 equal columns, gap 14px: Backlog, To Do, In Progress (label cyan), Blocked (label orange), Done (label lime). Column label Sora 600 12px uppercase + faint count.
- **Task card**: title → row of chips [`ID` chip (`surface-3`, muted), tag chips (tint of tag color)] with due date right-aligned (orange when due ≤ 24h or overdue) → optional progress row (4px track `surface-3`, fill cyan, label `6.5/12h`). Blocked cards: orange border `.4`, reason line in `#ffc496`. Done: `opacity:.55`, line-through. Card being viewed: 1px solid cyan border.
- Drag card between columns = status change. Click = open task modal.

### 3. Task modal (`#1b`)
- 820px wide, grid `1fr 280px`, radius 16, scrim `rgba(6,8,14,.62)`. Esc closes.
- Left: breadcrumb (project dot, name, `/`, ID, "Open in Jira ↗" chip) → title (22px Sora) → SUBTASKS (progress bar lime, circular checkboxes 15px, done items faint + line-through) → NOTES (13px/1.6, markdown) → ATTACHMENTS (chips with type prefix colored, `+ Add` dashed).
- Right rail (`#10141c`): Status, Priority, Due (orange if today; "Alerts 1 day, 1 hour before" hint), Time card (big hours, progress, `▶ Start` lime + `+ Log` buttons), Tags, Linked meeting (project tint). Footer: created date · "esc to close".
- Timer: one active timer per task; multiple tasks may run simultaneously. Stopping a timer appends a `time_entries` row and updates `hours_worked`.

### 4. Project → Week (`#3d`)
- Sub-bar: segmented with Week active; `‹ Sep 8 – 14 ›`; stats line; `+ New task`.
- Grid `repeat(7,1fr) 200px`, gap 10. Day headers Sora 600 11px faint; today header `#e2e6ee` + orange "today", column gets `rgba(255,255,255,.03)` wash. Overdue count in orange on past days.
- Meetings render as dashed-border pills with `○ MEETING` prefix + time. Tasks as board cards (compact). Empty days show a dashed placeholder. Last column: "NEXT WEEK & LATER" then "NO DATE" (dashed card border). Drag card to a day = set due date.

### 5. Today digest (`#2d`)
- Title "Today" + long date + counts; `Plan in Your Day →` outline button.
- **Week strip**: 7 cards (`surface`, radius 10), today card `#1a1f2b` with `.18` border; items are project-tinted pills, meetings prefixed `○`, done items line-through.
- Two columns: left OVERDUE (orange label, card border orange `.35`) / DUE TODAY / TOMORROW (opacity .8); right MEETINGS (past `.55`, next one cyan border + `Join ↗` primary) / HOURS THIS WEEK (per-project bars, project color).
- Row anatomy: project dot · title · meta (project · status · hours · planned time) · right-aligned Sora 600 11px time/badge. "Plan" link (cyan) on items with no block.

### 6. Your Day (`#2a`, `#2b`, empty `#3e`)
- Sub-bar: `‹ Thursday, Sep 11 ›` + "Today" link; scope segmented `All projects | ● Atlas only` (one project); right: `Daily summary` (cyan outline) + `+ New task`.
- **Leftovers banner** (only when yesterday has unfinished blocks, dismissible): orange tint `.1` / border `.35`, "YESTERDAY" label, count, one chip per block (project dot · title · remaining), actions `Carry all to today` (orange fill), `Pick`, `Dismiss`. Carrying creates identical-length blocks starting at the next free slot after now.
- Layout: `1fr 320px`, gap 20.
- **Planner**: `bg-raised` card, radius 12, vertical scroll over 24h; **60px per hour**; default scroll puts 07:00 at top; time gutter 64px; hour labels Sora 500 11px faint (current hour muted). Fade at top/bottom with "↑ 00:00 – 07:00 · nothing planned". Past time gets `rgba(0,0,0,.18)` shade. **Now line**: 1px orange, 7px dot, time label in gutter.
- **Blocks** (absolutely positioned, `top=(minutesFrom00:00)*1px`, `height=duration*1px`, min height 26px):
  - Task block: fill `rgba(project,.10)`, 1px border `rgba(project,.35–.45)`, radius 8, padding 8 10. Row 1: 14px checkbox (radius 4, project color) · `▪ TASK` prefix (Sora 600 10px project color) · time range right. Row 2: title 500. Row 3: meta (ID · hours logged → total). Active block (timer running): 1.5px solid project border, fill `.14`, ring `0 0 0 3px rgba(project,.15)`, lime timer pill `● 00:42:17` + `pause · stop`. Done block: checkbox filled with `✓`, title line-through muted, `opacity:.75`.
  - Meeting block: fill `.08`, **dashed** 1px border `.5–.6`, `○ MEETING` prefix, title, `time · duration · link`; when < 30 min away show `Join in 18m ↗` (project color, 600).
  - Break / Lunch: neutral dashed `rgba(255,255,255,.14)`, `◌ BREAK` / `◌ LUNCH` prefix, muted text.
  - Overlap → **side-by-side lanes**: split available width equally among concurrent blocks (4px gap), assign lanes greedily by start time. Any number of lanes; below 3 lanes keep full card content, at 4+ collapse to title only.
  - Selection in progress: dashed 1.5px cyan border, fill `.08`, label `09:00 – 10:30 · 1h 30m`, "Choose a task for this block…".
  - Drag to move, drag bottom edge to resize (15 min snapping). Free area shows `drag here to plan 17:00 – 18:00` on hover.
- **Pick-a-task popover** (`#2a`): 360px, `surface`, border `.12`, radius 12, anchored to the right of the new block. Title "What do you want to work on?", search input, DUE SOON list (project dot, title, meta line, due badge; keyboard ↑↓ ↵), then OR: `Break | Lunch | Focus, no task`. Footer hints. Esc cancels and removes the block.
- **Sidebar**: QUICK ADD (Break / Lunch / Focus — click adds 30/60/60 min at the next free slot) · UNSCHEDULED · DUE SOON (draggable rows with `⋮⋮` grip, project dot, title, remaining hours + due; drag onto the planner creates a block sized to remaining hours, capped at 2h) · YESTERDAY'S LEFTOVERS (`Carry` chip) or DONE TODAY (mid-day) · footer stats `Planned · Logged · Blocks n / m`.
- **Empty day** (`#3e`): dashed cyan hint block "Drag across any hours to plan a block"; "Nothing planned yet · 3 tasks are due this week" with `Plan from due dates` and `Copy last Monday` actions.

### 7. Daily summary panel (`#2c`)
- Opens from the header button as a 640px modal (or right sheet). Title + date, tabs per project **that has a manager**, note for projects without one.
- "To" row with avatar initials + name + email; SOURCES chips (toggleable, cyan tint): Yesterday's blocks · Done column · Today's plan · Blocked.
- Editable body (`bg`, border `.10`, radius 10, 13.5px/1.65) rendered from Slack mrkdwn: `*Project — date*`, sections **Yesterday** (lime), **Today** (cyan), **Blockers** (orange), `•` bullets, IDs as links (cyan). Label "EDITABLE · SLACK FORMAT".
- Footer: `Regenerate` (faint text), `Send email` (outline), `Copy for Slack` (primary). Copy writes mrkdwn to clipboard; Send opens `mailto:` (default) or SMTP if configured.
- Generation rules (per project): Yesterday = blocks checked done yesterday + tasks whose status became Done yesterday + meetings attended yesterday; Today = today's planned blocks (in time order) + today's meetings; Blockers = tasks in Blocked with their reason and `blocked_on` person + since-date. Brief tone: one line per item, no hours. Detailed: append `(x/yh)` and subtask progress.

### 8. Settings (`#2e`, `#3c`)
- 180px left nav (`bg-raised`): General · Notifications · Managers & summaries · Integrations · Appearance · Data & backup. Active item `surface-3`.
- **Notifications**: toggles (36×20 pill, on = cyan with `#04202a` knob right, off = `surface-3` with faint knob left): Keep running in system tray · Toast notifications · Nudge when a block starts · Play a sound. Default alerts chips (1 day, 1 hour selected). Rows: Meetings remind 15 min before with Join · Overdue nudge 09:00 · Summary reminder weekdays 17:15 if not sent · Quiet hours 22:00–08:00.
- **Managers & summaries**: table `150px 1fr 110px 120px` — Project · Manager (avatar, name · email) · Send at (time) · Tone (`Brief | Detailed` segmented). Row for projects with no manager is dashed with `Set up`. Below: toggle "Draft automatically at send time" (never sends unreviewed), "Send email via Default mail app · SMTP", tone explainer.

### 9. Tray toasts (`#2e`)
Native OS notifications; the mock shows the intended content. 380px card: header line (project dot · "Signal · context" · right-aligned status in accent color) → title Sora 600 14px → meta → action buttons.
1. Task due in 1h: `Open task` (primary) · `Snooze 30m` · `Mark done`
2. Meeting in 15 min: `Join ↗` (primary) · `Open agenda`
3. Block starting: `▶ Start timer` (lime) · `Push 15m`
4. 17:15 summary reminder: `Review & send` · `Later`
5. Morning overdue summary (from `#1d`): `Open Today` · `Reschedule`
Clicking a toast focuses/reopens the window on the relevant item. Respect quiet hours.

### 10. New task / new meeting (`#3a`, `#3b`)
- 600px (task) / 460px (meeting) modal; project picker chip + `Task | Meeting` segmented top-right.
- Task: LINK field first — pasting a Jira/Asana/ClickUp URL detects the provider, shows `✓ Jira · ATL-512` (lime) and fills the title; title field 20px Sora; Status · Priority · Estimate; Due · Alerts chips; Tags; Subtasks; Notes (markdown, drop files anywhere). Footer hints `⌘↵ create · ⌘⇧↵ create & plan in Your Day`.
- Meeting: title, When, Duration segmented (30m/45m/1h), LINK, Agenda (one item per line), Linked tasks search. Detail view (760px, `1fr 260px`): agenda list, notes, LINKED TASKS with status; rail: When, Duration, Link, Repeats, Reminder, "Show in Your Day" checkbox, `Delete` (danger) footer.

### 11. First launch (`#3e`)
Centered 420px column: four color dots, "Start with a project.", explainer, name input with color picker, `Create project` + `Import from Jira / Asana / ClickUp` (outline).

---

## Interactions & behavior summary
- Command palette `⌘K`: jump to project/task, "new task", "start timer on…", "daily summary".
- Keyboard: `n` new task on board; `⌘↵` submit; `esc` closes modals/popovers; `↑↓ ↵` in pickers.
- Transitions: modal/popover fade+scale 120ms ease-out; block drag snaps to 15 min; toasts are native.
- Timer ticks each second; when a block's end passes with the timer still running, mark `ran over` (orange meta) but keep running.
- Checking a block done: if the task's remaining estimate ≤ 0 or user confirms, offer "Move task to Done".
- Window close → hide to tray when the setting is on; tray menu: Open, Start/stop timers, Today's summary, Quit.

## State & data model (SQLite)
```sql
projects(id, name, color, manager_name, manager_email, summary_send_at, summary_tone, sort_order)
tasks(id, project_id, title, external_url, external_provider, external_id, status /*backlog|todo|in_progress|blocked|done*/,
      priority /*low|medium|high*/, due_at, estimate_h, hours_worked, blocked_reason, blocked_on, notes_md, created_at, updated_at, done_at)
subtasks(id, task_id, title, done, sort_order)
tags(id, name, color); task_tags(task_id, tag_id)
attachments(id, task_id, filename, path, size, mime)
meetings(id, project_id, title, starts_at, duration_min, link_url, agenda_md, notes_md, repeat_rule, reminder_min)
meeting_tasks(meeting_id, task_id)
blocks(id, date, start_min, end_min, kind /*task|break|lunch|focus*/, task_id, done, done_at, carried_from_block_id)
time_entries(id, task_id, block_id, started_at, ended_at, minutes)
alerts(id, task_id, offset_min)            -- default 1440, 60
summaries(id, project_id, date, body_mrkdwn, sent_at, sent_via)
settings(key, value)
```
Derived: `hours_worked = SUM(time_entries.minutes)/60`; Today badge = count(due ≤ today AND status ≠ done).

## Assets
Fonts: Sora and DM Sans from Google Fonts (bundle the woff2 files locally for offline use). No images. Icons: text glyphs in mocks — replace with Lucide (`circle`, `square`, `check`, `external-link`, `grip-vertical`, `play`, `pause`).

## Files
- `Task App Explorations.dc.html` — source design canvas (all turns; target = turns 2–3 and `#1b`).
- `Signal.standalone.html` — same canvas bundled into a single offline file.
- `../` project root also contains `support.js` (runtime for the source file; not needed for the standalone copy).
