<svelte:options runes={true} />

<script lang="ts">
  import TaskCard from './TaskCard.svelte';
  import {
    statusLabels,
    type TaskStatus,
    type BoardTask,
  } from '../../domain/types';
  let {
    status,
    tasks,
    total,
    filtered,
    nowUtc,
    timeZone,
    selectedTaskId,
    pending,
    onOpen,
    onMove,
    onDrag,
    preview = null,
    draggedTaskId = null,
  }: {
    status: TaskStatus;
    tasks: BoardTask[];
    total: number;
    filtered: boolean;
    nowUtc: string;
    timeZone: string;
    selectedTaskId: string | null;
    pending: boolean;
    onOpen: (id: string) => void;
    onMove: (id: string, status: TaskStatus) => void;
    onDrag: (e: PointerEvent, id: string) => void;
    preview?: {
      task: BoardTask;
      before: string | null;
      phase: 'drag' | 'drop';
    } | null;
    draggedTaskId?: string | null;
  } = $props();
  const cards = $derived.by(() => {
    const rows: { task: BoardTask; phase: 'drag' | 'drop' | null }[] =
      tasks.map((task) => ({ task, phase: null }));
    if (preview) {
      const before = rows.findIndex((row) => row.task.id === preview.before);
      rows.splice(before < 0 ? rows.length : before, 0, {
        task: { ...preview.task, status },
        phase: preview.phase,
      });
    }
    return rows;
  });
</script>

<section class="column" class:drop-target={!!preview} data-status={status}>
  <h2 class={status}>
    {statusLabels[status]}
    <span>{filtered ? `${tasks.length}/${total}` : total}</span>
  </h2>
  <div class="cards">
    {#each cards as row (row.phase ? `preview:${row.task.id}` : row.task.id)}
      <TaskCard
        task={row.task}
        {nowUtc}
        {timeZone}
        selected={!row.phase && selectedTaskId === row.task.id}
        {pending}
        preview={row.phase}
        dragSource={!row.phase && draggedTaskId === row.task.id}
        onOpen={() => onOpen(row.task.id)}
        onStatus={(value) => onMove(row.task.id, value)}
        {onDrag}
      />
    {/each}
    {#if !cards.length}<div class="empty">
        {filtered ? 'No matches' : 'No tasks'}
      </div>{/if}
  </div>
</section>

<style>
  .column {
    min-width: 0;
    min-height: var(--week-later-width);
  }
  h2 {
    font-size: var(--text-label);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    display: flex;
    gap: var(--space-2);
    margin: 0 0 var(--space-small);
    padding: 0 var(--space-1);
    color: var(--text-muted);
  }
  h2 span {
    color: var(--text-faint);
  }
  h2.in_progress {
    color: var(--accent);
  }
  h2.blocked {
    color: var(--warn);
  }
  h2.done {
    color: var(--lime);
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-small);
  }
  .empty {
    border: var(--line) dashed var(--border);
    padding: var(--space-6);
    text-align: center;
    color: var(--text-faint);
    border-radius: var(--radius-card);
  }
  .drop-target {
    background: var(--selection-fill);
    border-radius: var(--radius-card);
  }
</style>
