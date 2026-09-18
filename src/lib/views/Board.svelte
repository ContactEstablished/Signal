<svelte:options runes={true} />

<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import TaskDragGhost from '../components/TaskDragGhost.svelte';
  import BoardColumn from '../components/board/BoardColumn.svelte';
  import BoardFilters from '../components/board/BoardFilters.svelte';
  import MeetingsStrip from '../components/meetings/MeetingsStrip.svelte';
  import {
    emptyFilters,
    filterTasks,
    groupTasks,
    hasFilters,
  } from '../domain/board';
  import {
    statuses,
    errorMessage,
    statusLabels,
    type BoardTask,
    type BoardSnapshot,
    type TaskStatus,
  } from '../domain/types';
  let {
    snapshot,
    loading = false,
    readError = '',
    runningTaskIds = new Set<string>(),
    nowUtc,
    timeZone,
    selectedDate,
    selectedTaskId = null,
    onRetry,
    onOpenTask,
    onMoveTask,
    onPreview,
    onOpenMeeting,
  }: {
    snapshot: BoardSnapshot | null;
    loading?: boolean;
    readError?: string;
    runningTaskIds?: ReadonlySet<string>;
    nowUtc: string;
    timeZone: string;
    selectedDate: string;
    selectedTaskId?: string | null;
    onRetry: () => void;
    onOpenTask: (id: string) => void;
    onMoveTask: (
      id: string,
      status: TaskStatus,
      before: string | null,
    ) => Promise<unknown>;
    onOpenMeeting?: (ref: import('../domain/agenda').MeetingRef) => unknown;
    onPreview: (s: string) => void;
  } = $props();
  let filters = $state(emptyFilters()),
    filtersOpen = $state(false),
    pending = $state(false),
    error = $state('');
  let container = $state<HTMLDivElement>(null!);
  let groups = $derived(
    groupTasks(filterTasks(snapshot?.tasks ?? [], filters, nowUtc, timeZone)),
  );
  let all = $derived(groupTasks(snapshot?.tasks ?? []));
  let drag = $state<{
    id: string;
    pointerId: number;
    startX: number;
    startY: number;
    x: number;
    y: number;
    active: boolean;
    status: TaskStatus | null;
    before: string | null;
  } | null>(null);
  let frame = 0;
  let suppressClick = false;
  let capture: { element: HTMLElement; pointerId: number } | null = null;
  let dropping = $state<{
    task: BoardTask;
    status: TaskStatus;
    before: string | null;
  } | null>(null);
  let draggedTask = $derived(
    drag?.active
      ? (snapshot?.tasks.find((task) => task.id === drag?.id) ?? null)
      : null,
  );
  let preview = $derived(
    dropping
      ? { ...dropping, phase: 'drop' as const }
      : draggedTask && drag?.status
        ? {
            task: draggedTask,
            status: drag.status,
            before: drag.before,
            phase: 'drag' as const,
          }
        : null,
  );
  let announcement = $derived(
    dropping
      ? `Moving ${dropping.task.title} to ${statusLabels[dropping.status]}…`
      : draggedTask && drag?.status
        ? `${draggedTask.title} will move to ${statusLabels[drag.status]}. Release to drop; Escape to cancel.`
        : '',
  );
  export function openFilters() {
    filtersOpen = true;
  }
  async function move(id: string, status: TaskStatus, before: string | null) {
    if (pending) return;
    pending = true;
    error = '';
    try {
      await onMoveTask(id, status, before);
    } catch (e) {
      error = errorMessage(e);
    } finally {
      pending = false;
      dropping = null;
    }
  }
  function cancel() {
    drag = null;
    cancelAnimationFrame(frame);
    const previous = capture;
    capture = null;
    if (previous?.element.hasPointerCapture(previous.pointerId))
      previous.element.releasePointerCapture(previous.pointerId);
  }
  function target(x: number, y: number) {
    if (!drag) return;
    const column = document
      .elementFromPoint(x, y)
      ?.closest<HTMLElement>('[data-status]');
    if (!column || !container.contains(column)) {
      drag.status = null;
      return;
    }
    const status = column.dataset.status as TaskStatus;
    // The placeholder moves siblings. Hold its current slot while the pointer
    // remains inside it, avoiding an oscillating before/after target.
    const ghost = column.querySelector<HTMLElement>('[data-drop-preview]');
    const ghostBounds = ghost?.getBoundingClientRect();
    if (
      drag.status === status &&
      ghostBounds &&
      y >= ghostBounds.top &&
      y <= ghostBounds.bottom
    )
      return;
    drag.status = status;
    const cards = [
      ...column.querySelectorAll<HTMLElement>('[data-task-id]'),
    ].filter((c) => c.dataset.taskId !== drag?.id);
    drag.before =
      cards.find(
        (c) =>
          y <
          c.getBoundingClientRect().top + c.getBoundingClientRect().height / 2,
      )?.dataset.taskId ?? null;
  }
  function scroll() {
    if (!drag?.active) return;
    const bounds = container.getBoundingClientRect();
    if (drag.x >= bounds.left && drag.x <= bounds.right && drag.y >= bounds.top && drag.y <= bounds.bottom) {
      if (drag.y > bounds.bottom - 40) container.scrollTop += 8;
      else if (drag.y < bounds.top + 40) container.scrollTop -= 8;
    }
    target(drag.x, drag.y);
    frame = requestAnimationFrame(scroll);
  }
  function start(e: PointerEvent, id: string) {
    if (pending || loading || readError || drag || e.button !== 0) return;
    suppressClick = false;
    const element = e.currentTarget as HTMLElement;
    element.setPointerCapture(e.pointerId);
    capture = { element, pointerId: e.pointerId };
    drag = {
      id,
      pointerId: e.pointerId,
      startX: e.clientX,
      startY: e.clientY,
      x: e.clientX,
      y: e.clientY,
      active: false,
      status: null,
      before: null,
    };
  }
  function pointerMove(e: PointerEvent) {
    if (!drag || drag.pointerId !== e.pointerId) return;
    drag.x = e.clientX;
    drag.y = e.clientY;
    if (
      !drag.active &&
      Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > 5
    ) {
      drag.active = true;
      suppressClick = true;
      frame = requestAnimationFrame(scroll);
    }
    if (drag.active) {
      e.preventDefault();
      target(e.clientX, e.clientY);
    }
  }
  function pointerUp(event: PointerEvent) {
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (drag.active) target(event.clientX, event.clientY);
    const current = drag;
    const task = draggedTask;
    cancel();
    if (current.active && current.status && task) {
      dropping = { task, status: current.status, before: current.before };
      void move(current.id, current.status, current.before).then(async () => {
        await tick();
        if (!container?.isConnected) return;
        const card = [
          ...container.querySelectorAll<HTMLElement>('[data-task-id]'),
        ].find((card) => card.dataset.taskId === current.id);
        card
          ?.querySelector<HTMLButtonElement>('.card-open')
          ?.focus({ preventScroll: true });
      });
    }
  }
  // selectedDate is supplied by the ticking workspace clock. Compare the
  // actual context value so a same-day clock tick cannot cancel a gesture.
  const dragContext = $derived(
    JSON.stringify([snapshot?.project.id, selectedDate, timeZone, filters]),
  );
  $effect(() => {
    dragContext;
    untrack(cancel);
  });
  $effect(() => {
    if (loading || readError || (drag && !snapshot?.tasks.some(task => task.id === drag?.id))) cancel();
  });
  onDestroy(cancel);
</script>

<svelte:window
  onpointermove={pointerMove}
  onpointerup={pointerUp}
  onpointercancel={(event) => {
    if (drag?.pointerId === event.pointerId) cancel();
  }}
  onlostpointercapture={(event) => {
    if (drag?.pointerId === event.pointerId) cancel();
  }}
  onblur={cancel}
  onkeydown={(e) => {
    if (e.key === 'Escape' && drag) {
      e.preventDefault();
      cancel();
    }
  }}
/>
{#if draggedTask && drag}<TaskDragGhost title={draggedTask.title}
    hint={drag.status ? `Release to move to ${statusLabels[drag.status]}` : 'Drop into a board column'}
    x={drag.x} y={drag.y} color={snapshot?.project.color} />{/if}
{#if readError}<div role="alert" class="error">
    {readError} <button class="outline" onclick={onRetry}>Retry</button>
  </div>{/if}
{#if error}<p role="alert" class="error">{error}</p>{/if}
{#if snapshot}<MeetingsStrip
    {onOpenMeeting}
    meetings={snapshot.meetings}
    project={snapshot.project}
    {selectedDate}
    {nowUtc}
    {timeZone}
    {onPreview}
  />
  <div
    class="board-scroll"
    class:dragging={!!draggedTask}
    bind:this={container}
    aria-busy={loading || pending}
  >
    <div class="columns">
      {#each statuses as status}<BoardColumn
          {status}
          tasks={dropping
            ? groups[status].filter((task) => task.id !== dropping?.task.id)
            : groups[status]}
          total={all[status].length}
          filtered={hasFilters(filters)}
          {runningTaskIds}
          {nowUtc}
          {timeZone}
          {selectedTaskId}
          {pending}
          onOpen={(id, event) => {
            if (!suppressClick || event.detail === 0) onOpenTask(id);
          }}
          onMove={(id, value) => {
            if (snapshot?.tasks.find((t) => t.id === id)?.status !== value)
              void move(id, value, null);
          }}
          onDrag={start}
          draggedTaskId={draggedTask?.id ?? null}
          preview={preview?.status === status ? preview : null}
        />{/each}
    </div>
  </div>
  {#if filtersOpen}<BoardFilters
      bind:filters
      tags={snapshot.tags}
      onClose={() => {
        filtersOpen = false;
        document
          .querySelector<HTMLButtonElement>('[data-filter-trigger]')
          ?.focus();
      }}
    />{/if}
{:else if loading}<p role="status">Loading Board…</p>{:else if !readError}<p>
    Select or create a project to open its Board.
  </p>{/if}
<p class="drag-announcement" role="status" aria-live="polite">
  {announcement}
</p>
{#if pending}<p class="saving" role="status">Saving task order…</p>{/if}

<style>
  .board-scroll {
    overflow: auto;
    max-height: calc(100vh - var(--header-height) - var(--week-later-width));
    padding-bottom: var(--space-6);
  }
  .dragging,
  .dragging :global(*) {
    cursor: grabbing !important;
    user-select: none;
  }
  .drag-announcement {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .columns {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--board-gap);
  }
  .saving {
    font-size: var(--text-meta);
    color: var(--accent);
  }
</style>
