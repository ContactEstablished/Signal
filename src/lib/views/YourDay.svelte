<script lang="ts">
  import { tick, onDestroy } from 'svelte';
  import { ChevronLeft, ChevronRight, Plus } from 'lucide-svelte';
  import type { Workspace } from '../state/app.svelte';
  import type {
    BlockDraft,
    PlanPreview,
    PlannerSnapshot,
    BatchMode,
    BlockKind,
  } from '../domain/planner';
  import type { TimerAction } from '../domain/timers';
  import { SIDEBAR_BLOCK_MINUTES } from '../domain/planner-drag';
  import {
    addDays,
    automaticFloor,
    leftovers,
    normalizeBlock,
    plannerItems,
    previewPlan,
    taskDuration,
  } from '../domain/planner-rules';
  import { errorMessage, type CloseReason } from '../domain/types';
  import PlannerCanvas from '../components/planner/PlannerCanvas.svelte';
  import PlannerSidebar from '../components/planner/PlannerSidebar.svelte';
  import TaskPicker from '../components/planner/TaskPicker.svelte';
  import BlockEditor from '../components/planner/BlockEditor.svelte';
  import PlannerPreview from '../components/planner/PlannerPreview.svelte';
  let {
    workspace,
    onOpenTask,
    onNewTask,
    onOpenMeeting,
    onJoin,
    onSummary,
  }: {
    workspace: Workspace;
    onOpenTask: (id: string) => Promise<void>;
    onNewTask: () => Promise<void>;
    onOpenMeeting?: (
      ref: import('../domain/agenda').MeetingRef,
    ) => Promise<void>;
    onJoin: (url: string) => Promise<void>;
    onSummary: () => void;
  } = $props();
  const planner = $derived(workspace.planner),
    snapshot = $derived(
      planner.snapshot?.date === planner.date ? planner.snapshot : null,
    );
  const pending = $derived(
    planner.pending || Object.values(workspace.timers.pending).some(Boolean),
  );
  let quickBusy = $state(false);
  const locked = $derived(pending || planner.recovery || quickBusy);
  const items = $derived(
    snapshot
      ? plannerItems(
          {
            ...snapshot,
            timers: { ...snapshot.timers, sessions: workspace.timers.sessions },
          },
          planner.scope,
        )
      : [],
  );
  const left = $derived(snapshot ? leftovers(snapshot, planner.scope) : []);
  let selection = $state<{ startMin: number; endMin: number } | null>(null),
    anchor = $state<DOMRect | null>(null),
    editor = $state<{ id?: string; draft: BlockDraft } | null>(null);
  let preview = $state<{ plan: PlanPreview; snapshot: PlannerSnapshot } | null>(
      null,
    ),
    error = $state(''),
    offerTask = $state<string | null>(null);
  let confirm: HTMLDialogElement;
  let canvas = $state<PlannerCanvas>();
  let confirmation = $state<{
    title: string;
    body: string;
    accept: string;
  } | null>(null);
  let resolveConfirm: ((v: boolean) => void) | null = null;
  $effect(() => {
    planner.draftOpen = !!editor || !!preview || !!selection || !!confirmation;
  });
  onDestroy(() => {
    workspace.planner.draftOpen = false;
    resolveConfirm?.(false);
  });
  async function ask(title: string, body: string, accept: string) {
    if (confirmation) return false;
    confirmation = { title, body, accept };
    await tick();
    confirm.showModal();
    return new Promise<boolean>((r) => (resolveConfirm = r));
  }
  function answer(value: boolean) {
    confirm.close();
    confirmation = null;
    resolveConfirm?.(value);
    resolveConfirm = null;
  }
  export async function requestClose(_reason: CloseReason): Promise<boolean> {
    await workspace.settled();
    if (Object.values(workspace.timers.recovery).some(Boolean)) {
      error = 'Retry the pending task timer operation before leaving.';
      return false;
    }
    if (planner.recovery) {
      error = 'Retry the pending planner operation before leaving.';
      return false;
    }
    if (confirmation) {
      answer(false);
      return false;
    }
    if (editor || preview) {
      if (
        !(await ask(
          'Discard planner changes?',
          'Your unsaved block or planning preview will be discarded.',
          'Discard',
        ))
      )
        return false;
    }
    editor = null;
    preview = null;
    selection = null;
    anchor = null;
    return true;
  }
  async function closeDraft() {
    if (await requestClose('dialog')) error = '';
  }
  async function dateChange(date: string, follow = false) {
    if (!(await requestClose('navigation'))) return;
    planner.date = date;
    planner.followToday = follow;
    await planner.load(workspace.timeZone);
  }
  async function scopeChange(scope: string) {
    if (await requestClose('navigation')) planner.scope = scope;
  }
  export function planTask(id: string) {
    if (!snapshot) return;
    const task = snapshot.tasks.find((t) => t.id === id);
    if (!task) {
      error = 'Refresh Your Day to schedule this task.';
      return;
    }
    const start = Math.min(
      1425,
      automaticFloor(planner.date, workspace.nowUtc, workspace.timeZone) ?? 420,
    );
    const duration = taskDuration(task) ?? 30;
    editor = {
      draft: {
        kind: 'task',
        task_id: id,
        start_min: Math.min(start, 1440 - duration),
        end_min: Math.min(start + duration, 1440),
      },
    };
  }
  function addBlock() {
    editor = {
      draft: { kind: 'focus', task_id: null, start_min: 540, end_min: 600 },
    };
    error = '';
  }
  function editTime(id: string) {
    const block = snapshot?.blocks.find((b) => b.id === id);
    if (block)
      editor = {
        id,
        draft: {
          kind: block.kind,
          task_id: block.task_id,
          start_min: block.start_min,
          end_min: block.end_min,
          ...(block.time_zone === workspace.timeZone
            ? {
                start_offset: block.start_offset ?? undefined,
                end_offset: block.end_offset ?? undefined,
              }
            : {}),
        },
      };
  }
  async function save(draft: BlockDraft, destinationDate = planner.date) {
    if (!snapshot) return;
    const clean = normalizeBlock(draft, destinationDate, workspace.timeZone);
    if (editor?.id) {
      const b = snapshot.blocks.find((b) => b.id === editor!.id)!;
      await workspace.plannerAction('move', {
        id: b.id,
        expectedRevision: b.revision,
        destinationDate,
        start_min: clean.start_min,
        end_min: clean.end_min,
        start_offset: clean.start_offset,
        end_offset: clean.end_offset,
      });
    } else
      await workspace.plannerAction('create', {
        draft: clean,
        destinationDate,
      });
    editor = null;
    selection = null;
    anchor = null;
    error = '';
    await showDestination(destinationDate);
  }
  async function pick(choice: { kind: BlockKind; taskId?: string }) {
    if (!selection) return;
    const draft: BlockDraft = {
      kind: choice.kind,
      task_id: choice.taskId ?? null,
      start_min: selection.startMin,
      end_min: selection.endMin,
    };
    try {
      normalizeBlock(draft, planner.date, workspace.timeZone);
    } catch {
      editor = { draft };
      selection = null;
      anchor = null;
      return;
    }
    try {
      await save(draft);
    } catch (e) {
      error = errorMessage(e);
      throw e;
    }
  }
  async function move(id: string, start: number, end: number) {
    const b = snapshot?.blocks.find((b) => b.id === id);
    if (!b) return;
    const draft: BlockDraft = {
      kind: b.kind,
      task_id: b.task_id,
      start_min: start,
      end_min: end,
    };
    try {
      normalizeBlock(draft, planner.date, workspace.timeZone);
    } catch {
      editor = { id, draft };
      return;
    }
    editor = { id, draft };
    try {
      await save(draft);
    } catch (e) {
      error = errorMessage(e);
      throw e;
    }
  }
  async function dropTask(id: string, start: number) {
    if (locked || editor || preview || selection) return;
    const task = snapshot?.tasks.find(
      (t) => t.id === id && t.status !== 'done',
    );
    if (!task) throw new Error('This task is no longer available.');
    const duration = SIDEBAR_BLOCK_MINUTES;
    if (start + duration > 1440)
      throw new Error(
        'This task does not fit before midnight. Choose an earlier time.',
      );
    editor = {
      draft: {
        kind: 'task',
        task_id: id,
        start_min: start,
        end_min: start + duration,
      },
    };
    try {
      normalizeBlock(editor.draft, planner.date, workspace.timeZone);
    } catch {
      return;
    }
    await save(editor.draft);
  }
  async function plan(
    mode: BatchMode,
    ids?: string[],
    kind?: 'break' | 'lunch' | 'focus',
  ) {
    if (!snapshot || locked) return;
    const proposed = previewPlan(
      snapshot,
      mode,
      workspace.nowUtc,
      planner.scope,
      ids,
      kind,
    );
    error = '';
    if (mode !== 'quick') {
      preview = { plan: proposed, snapshot };
      return;
    }
    if (!proposed.blocks.length) {
      error = proposed.skipped[0]?.reason ?? 'No space left in this day.';
      return;
    }
    let blocks: BlockDraft[];
    try {
      blocks = proposed.blocks.map((b) =>
        normalizeBlock(b, planner.date, workspace.timeZone),
      );
    } catch {
      preview = { plan: proposed, snapshot };
      return;
    }
    quickBusy = true;
    try {
      await workspace.plannerAction(
        'batch',
        { mode, blocks },
        proposed.fingerprint,
      );
    } catch (e) {
      error = errorMessage(e);
    } finally {
      quickBusy = false;
    }
  }
  async function apply(blocks: BlockDraft[]) {
    if (!preview) return;
    await workspace.plannerAction(
      'batch',
      { mode: preview.plan.mode, blocks },
      preview.plan.fingerprint,
    );
    preview = null;
    error = '';
  }
  async function showDestination(date?: string) {
    if (date && date !== planner.date) {
      planner.date = date;
      planner.followToday = date === workspace.selectedDate;
      await planner.load(workspace.timeZone);
    }
  }
  async function retry() {
    try {
      const result = await workspace.retryPlanner();
      editor = null;
      preview = null;
      selection = null;
      anchor = null;
      error = '';
      await showDestination(result.outcome.destination_date);
    } catch (e) {
      error = errorMessage(e);
    }
  }
  async function done(id: string) {
    const block = snapshot?.blocks.find((b) => b.id === id);
    if (!block) return;
    const session = workspace.timers.sessions.find((s) => s.block_id === id);
    if (
      !block.done &&
      session &&
      !(await ask(
        'Stop timer and complete block?',
        'This saves the timer’s elapsed time and completes this block together. The task status stays unchanged.',
        'Stop and complete',
      ))
    )
      return;
    await workspace.plannerAction('done', {
      id,
      expectedRevision: block.revision,
      done: !block.done,
      ...(!block.done && session
        ? { stopSession: { id: session.id, revision: session.revision } }
        : {}),
    });
    if (
      !block.done &&
      block.task_id &&
      snapshot?.tasks.find((t) => t.id === block.task_id)?.status !== 'done'
    )
      offerTask = block.task_id;
  }
  async function remove(id: string) {
    const block = snapshot?.blocks.find((b) => b.id === id);
    if (!block || !snapshot) return;
    const fingerprint = snapshot.fingerprint;
    if (
      !(await ask(
        'Remove this block?',
        'The task and logged time are preserved. Any associated timer keeps running and will be detached from this block.',
        'Remove block',
      ))
    )
      return;
    await workspace.plannerAction(
      'remove',
      { id, expectedRevision: block.revision },
      fingerprint,
    );
  }
  async function timer(id: string, action: TimerAction) {
    const block = snapshot?.blocks.find((b) => b.id === id);
    if (!block?.task_id) return;
    const existing = workspace.timers.session(block.task_id);
    if (existing && existing.block_id !== id) {
      await onOpenTask(block.task_id);
      return;
    }
    await workspace.timeAction(block.task_id, action, undefined, false, id);
  }
  async function safe(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<main class="your-day">
  <div class="subbar">
    <h1>Your Day</h1>
    <div class="date-nav">
      <button
        aria-label="Previous day"
        onclick={() => dateChange(addDays(planner.date, -1))}
        ><ChevronLeft /></button
      ><input
        aria-label="Planner date"
        type="date"
        value={planner.date}
        onchange={(e) => {
          if (e.currentTarget.value) void dateChange(e.currentTarget.value);
        }}
      /><button
        aria-label="Next day"
        onclick={() => dateChange(addDays(planner.date, 1))}
        ><ChevronRight /></button
      ><button
        class="outline"
        onclick={() => dateChange(workspace.selectedDate, true)}>Today</button
      >
    </div>
    <select
      aria-label="Project scope"
      value={planner.scope}
      onchange={(e) => scopeChange(e.currentTarget.value)}
      ><option value="all">All projects</option
      >{#each workspace.projects as project}<option value={project.id}
          >{project.name}</option
        >{/each}</select
    ><span class="spacer"></span><button class="outline" onclick={onSummary}
      >Daily summary</button
    ><button class="outline" disabled={locked} onclick={addBlock}
      >Add block</button
    ><button
      class="primary"
      disabled={locked || !workspace.projects.length}
      onclick={onNewTask}><Plus />New task</button
    >
  </div>
  {#if error || planner.error}<div class="message error" role="alert">
      {planner.recovery
        ? planner.error
        : error || planner.error}{#if planner.recovery}<button
          class="outline"
          disabled={pending}
          onclick={retry}>Retry exact operation</button
        >{:else}<button
          class="outline"
          onclick={() => planner.load(workspace.timeZone)}
          >Refresh planner</button
        >{/if}
    </div>{/if}
  {#each Object.entries(workspace.timers.recovery).filter(([, required]) => required) as [taskId]}<div
      class="message error"
      role="alert"
    >
      Task timer result is unknown. <button
        class="outline"
        disabled={pending}
        onclick={() => safe(() => workspace.timeBindings(taskId).onRetry())}
        >Retry task timer</button
      >
    </div>{/each}
  {#if offerTask}<div class="message" role="status">
      Block completed. <button
        class="outline"
        onclick={() =>
          safe(async () => {
            await workspace.patch(offerTask!, { status: 'done' });
            offerTask = null;
          })}>Move task to Done</button
      ><button onclick={() => (offerTask = null)}>Keep task open</button>
    </div>{/if}
  {#if snapshot}
    {#if left.length && !snapshot.dismissed}<div class="leftover-banner">
        <strong
          >{left.length} unfinished {left.length === 1 ? 'block' : 'blocks'} from
          yesterday</strong
        ><span>Bring them into today’s plan.</span><button
          class="outline"
          disabled={locked ||
            automaticFloor(
              planner.date,
              workspace.nowUtc,
              workspace.timeZone,
            ) === null}
          onclick={() => plan('carry')}>Carry all</button
        ><button
          disabled={locked}
          onclick={() => safe(() => workspace.plannerAction('dismiss', {}))}
          >Dismiss</button
        >
      </div>{/if}
    <div class="layout">
      <PlannerCanvas
        bind:this={canvas}
        {items}
        date={planner.date}
        nowUtc={workspace.nowUtc}
        timeZone={workspace.timeZone}
        pending={locked}
        {selection}
        onSelect={(startMin, endMin, rect) => {
          selection = { startMin, endMin };
          anchor = rect;
          error = '';
        }}
        onCancelSelection={() => {
          if (!pending && !planner.recovery) {
            selection = null;
            anchor = null;
          }
        }}
        onMove={move}
        onDropTask={dropTask}
        {onOpenTask}
        onDone={done}
        onRemove={remove}
        onTimer={timer}
        {onJoin}
        {onOpenMeeting}
        onEditTime={editTime}
      /><PlannerSidebar
        {snapshot}
        scope={planner.scope}
        nowUtc={workspace.nowUtc}
        pending={locked}
        onPlan={plan}
        onAddTask={planTask}
        onTaskDrag={(drag) => canvas?.previewTaskDrop(drag)}
        onTaskDrop={(id, x, y) => canvas?.dropTaskAt(id, x, y)}
        {onOpenTask}
      />
    </div>
  {:else}<p>
      {planner.loading
        ? 'Loading your day…'
        : 'Your day is unavailable. Use Refresh planner to try again.'}
    </p>{/if}
</main>
{#if selection && anchor}<TaskPicker
    range={selection}
    {anchor}
    tasks={snapshot?.tasks.filter(
      (t) =>
        t.status !== 'done' &&
        (planner.scope === 'all' || t.project_id === planner.scope),
    ) ?? []}
    {pending}
    error={planner.recovery ? planner.error : error}
    recovery={planner.recovery}
    onRetry={retry}
    onPick={pick}
    onCancel={() => {
      if (!planner.recovery) {
        selection = null;
        anchor = null;
      }
    }}
  />{/if}
{#if editor}{#key editor}<BlockEditor
      draft={editor.draft}
      editing={!!editor.id}
      date={planner.date}
      timeZone={workspace.timeZone}
      tasks={snapshot?.tasks ?? []}
      {pending}
      recovery={planner.recovery}
      error={planner.error}
      onSave={save}
      onClose={() => void closeDraft()}
      onRetry={retry}
    />{/key}{/if}
{#if preview}<PlannerPreview
    preview={preview.plan}
    snapshot={preview.snapshot}
    {pending}
    recovery={planner.recovery}
    error={planner.error}
    onApply={apply}
    onClose={() => void closeDraft()}
    onRetry={retry}
  />{/if}
<dialog
  class="m1-dialog planner-confirm"
  bind:this={confirm}
  aria-label={confirmation?.title ?? 'Planner confirmation'}
  oncancel={(e) => {
    e.preventDefault();
    answer(false);
  }}
>
  {#if confirmation}<h2>{confirmation.title}</h2>
    <p>{confirmation.body}</p>
    <div class="actions">
      <button class="outline" onclick={() => answer(false)}>Cancel</button
      ><button class="primary" onclick={() => answer(true)}
        >{confirmation.accept}</button
      >
    </div>{/if}
</dialog>

<style>
  .your-day {
    height: calc(100vh - var(--header-height));
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 18px 20px 20px;
    gap: 16px;
  }
  .subbar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: none;
  }
  .subbar h1 {
    font-size: var(--text-page-small);
    margin: 0 12px 0 0;
  }
  .date-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .date-nav > button:not(.outline) {
    padding: 6px;
    display: flex;
  }
  .date-nav input {
    width: 142px;
    padding: 6px;
  }
  .subbar select {
    max-width: 190px;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--planner-sidebar);
    gap: var(--planner-gap);
    flex: 1;
    min-height: 0;
  }
  .leftover-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    border: 1px solid var(--warn-border);
    background: var(--warn-fill);
    padding: 10px 14px;
    border-radius: var(--radius-card);
  }
  .leftover-banner strong {
    color: var(--warn);
  }
  .leftover-banner span {
    flex: 1;
    color: var(--text-muted);
  }
  .message {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .planner-confirm {
    width: 480px;
  }
</style>
