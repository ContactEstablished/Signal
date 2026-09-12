<svelte:options runes={true} />

<script lang="ts">
  import { GripVertical, ChevronDown } from 'lucide-svelte';
  import {
    statuses,
    statusLabels,
    type BoardTask,
    type TaskStatus,
  } from '../../domain/types';
  import { dueLabel, progress, urgent } from '../../domain/board';
  let {
    task,
    nowUtc,
    timeZone,
    selected = false,
    pending = false,
    preview = null,
    dragSource = false,
    onOpen,
    onStatus,
    onDrag,
  }: {
    task: BoardTask;
    nowUtc: string;
    timeZone: string;
    selected?: boolean;
    pending?: boolean;
    preview?: 'drag' | 'drop' | null;
    dragSource?: boolean;
    onOpen: () => void;
    onStatus: (status: TaskStatus) => void;
    onDrag: (event: PointerEvent, id: string) => void;
  } = $props();
</script>

<article
  class:selected
  class:drag-source={dragSource}
  class:drag-preview={preview === 'drag'}
  class:drop-preview={preview === 'drop'}
  inert={!!preview}
  aria-hidden={preview ? 'true' : undefined}
  data-drop-preview={preview ? '' : undefined}
  class:blocked={task.status === 'blocked'}
  class:done={task.status === 'done'}
  data-task-id={preview ? undefined : task.id}
>
  <button
    class="card-open"
    onclick={onOpen}
    aria-label={`Open ${task.title}`}
    disabled={pending || !!preview}
    ><span class="title">{task.title}</span>
    <span class="meta"
      ><span class="chips"
        ><span class="id"
          >{task.external_provider ? task.external_id : 'Local task'}</span
        >{#each task.tags as tag}<span class="chip" data-color={tag.color}
            >{tag.name}</span
          >{/each}</span
      ><span class:urgent={urgent(task, nowUtc)} class="due"
        >{dueLabel(task.due_at, timeZone)}</span
      ></span
    >
    {#if task.estimate_h !== null || task.hours_worked > 0}<span class="time"
        ><span class="progress"
          ><span
            style:width={`${progress(task.hours_worked, task.estimate_h)}%`}
          ></span></span
        ><span
          >{task.hours_worked.toLocaleString('en-US', {
            maximumFractionDigits: 2,
          })}{task.estimate_h !== null ? `/${task.estimate_h}` : ''}h</span
        ></span
      >{/if}
    {#if task.status === 'blocked' && task.blocked_reason}<span class="reason"
        >{task.blocked_reason}</span
      >{/if}
  </button>
  {#if !preview}<div class="card-actions">
      <button
        aria-label={`Drag ${task.title}`}
        title="Drag to reorder"
        onpointerdown={(event) => onDrag(event, task.id)}
        disabled={pending || !!preview}><GripVertical /></button
      ><label
        ><ChevronDown /><select
          aria-label={`Status of ${task.title}`}
          value={task.status}
          disabled={pending || !!preview}
          onchange={(e) => onStatus(e.currentTarget.value as TaskStatus)}
          >{#each statuses as status}<option value={status}
              >{statusLabels[status]}</option
            >{/each}</select
        ></label
      >
    </div>{/if}
</article>

<style>
  article {
    position: relative;
    background: var(--surface);
    border: var(--line) solid var(--border);
    border-radius: var(--radius-card);
    box-shadow: none;
    transition:
      opacity var(--transition),
      border-color var(--transition);
  }
  article:hover {
    background: var(--surface-2);
  }
  article.selected {
    border-color: var(--accent);
  }
  article.blocked {
    border-color: var(--blocked-border);
  }
  article.blocked.selected {
    border-color: var(--accent);
  }
  article.done {
    opacity: var(--opacity-done-card);
  }
  article.drag-source {
    opacity: var(--opacity-drag-source);
    pointer-events: none;
  }
  article.drag-preview {
    opacity: var(--opacity-drag-preview);
    border-color: var(--accent);
    pointer-events: none;
  }
  article.drop-preview {
    border-color: var(--accent);
    pointer-events: none;
  }
  article.drop-preview:not(.done) {
    opacity: 1;
  }
  article.drag-preview .card-open,
  article.drop-preview .card-open {
    color: var(--text);
  }
  .card-open {
    width: 100%;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--padding-card);
    border: 0;
  }
  .title {
    font-weight: var(--weight-medium);
    font-size: var(--text-card);
    line-height: var(--leading-card);
    overflow-wrap: anywhere;
  }
  .done .title {
    text-decoration: line-through;
  }
  .meta {
    display: flex;
    width: 100%;
    gap: var(--space-tight);
    align-items: center;
    justify-content: space-between;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    min-width: 0;
  }
  .id {
    background: var(--surface-3);
    color: var(--text-muted);
    font: var(--weight-semibold) var(--text-chip) var(--font-heading);
    padding: calc(var(--space-1) - var(--line))
      calc(var(--space-2) - var(--line));
    border-radius: var(--radius-chip);
  }
  .due {
    font-size: var(--text-meta);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .urgent {
    color: var(--warn);
  }
  .time {
    display: flex;
    width: 100%;
    align-items: center;
    gap: var(--space-2);
    font: var(--weight-semibold) var(--text-meta) var(--font-heading);
    color: var(--text-muted);
  }
  .progress {
    flex: 1;
  }
  .reason {
    color: var(--blocked-text);
    font-size: var(--text-label);
  }
  .card-actions {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    display: flex;
    align-items: center;
    color: var(--text-muted);
    background: var(--surface-2);
    border-radius: var(--radius-chip);
    opacity: 0;
  }
  article:hover .card-actions,
  article:focus-within .card-actions {
    opacity: 1;
  }
  .card-actions button {
    padding: 0;
    display: flex;
    touch-action: none;
  }
  .card-actions label {
    position: relative;
    display: flex;
  }
  .card-actions select {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
  }
  .card-actions label:focus-within {
    outline: var(--focus-border);
  }
</style>
