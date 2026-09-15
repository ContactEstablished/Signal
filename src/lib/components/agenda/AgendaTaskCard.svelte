<script lang="ts">
  import { minuteLabel } from '../../domain/time-display';
  import { GripVertical, CalendarDays, Play } from 'lucide-svelte';
  import type { AgendaTask } from '../../domain/agenda';
  let {
    task,
    displayDate,
    deadlineZone,
    compact = false,
    disabled = false,
    preview = false,
    faded = false,
    running = false,
    onOpen,
    onPlan,
    onMove,
    onDrag,
  }: {
    task: AgendaTask;
    displayDate?: string;
    deadlineZone?: string;
    compact?: boolean;
    disabled?: boolean;
    preview?: boolean;
    faded?: boolean;
    running?: boolean;
    onOpen: (id: string) => unknown;
    onPlan?: (id: string) => unknown;
    onMove?: (id: string) => unknown;
    onDrag?: (e: PointerEvent, task: AgendaTask) => void;
  } = $props();
</script>

<article
  data-task-id={task.id}
  class:compact
  class:preview
  class:faded
  class:done={task.status === 'done'}
  data-color={task.project_color}
  aria-hidden={preview || undefined}
>
  <div class="title-row">
    {#if onDrag}<button
        class="grip"
        aria-label={`Drag ${task.title} to a date`}
        disabled={disabled || preview}
        onpointerdown={(e) => onDrag?.(e, task)}><GripVertical /></button
      >{/if}<button
      class="title"
      title={task.title}
      disabled={preview}
      onclick={() => onOpen(task.id)}
      >{#if running}<Play />{/if}{task.title}</button
    >
  </div>
  <div class="meta">
    <span class="dot"></span><span
      >{compact ? task.external_id || 'Local task' : task.project_name}</span
    >{#if !compact}<span
        >· {task.status.replaceAll('_', ' ')} · {Number(
          task.hours_worked.toFixed(2),
        )}h</span
      >{/if}
  </div>
  {#if deadlineZone && task.due_at}<div class="meta">
      <time datetime={task.due_at}
        >Due {new Intl.DateTimeFormat('en-US', {
          month: 'short',
          day: 'numeric',
          year: 'numeric',
          timeZone: deadlineZone,
        }).format(new Date(task.due_at))}</time
      >
    </div>{/if}
  {#if !compact && task.planned_ranges.some((r) => !displayDate || r.date === displayDate)}<div
      class="meta"
    >
      Planned {task.planned_ranges
        .filter((r) => !displayDate || r.date === displayDate)
        .map(
          (r) =>
            `${r.date} ${minuteLabel(r.start_min)}`,
        )
        .join(' · ')}
    </div>{/if}
  {#if onPlan || onMove}<div class="actions">
      {#if onPlan}<button
          class="plan"
          disabled={disabled || preview}
          onclick={() => onPlan?.(task.id)}>Plan →</button
        >{/if}{#if onMove}<button
          class="move"
          disabled={disabled || preview}
          onclick={() => onMove?.(task.id)}
          ><CalendarDays /><span>Move to date</span></button
        >{/if}
    </div>{/if}
</article>

<style>
  article {
    min-width: 0;
    border: var(--line) solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    padding: var(--padding-card);
  }
  .title-row {
    display: flex;
    gap: var(--space-1);
    align-items: flex-start;
  }
  .title {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    padding: 0;
    text-align: left;
    font-size: var(--text-card);
    font-weight: var(--weight-medium);
    line-height: var(--leading-card);
    overflow-wrap: anywhere;
  }
  .grip {
    padding: 0;
    color: var(--text-faint);
    touch-action: none;
    cursor: grab;
  }
  .meta {
    display: flex;
    gap: var(--space-1);
    align-items: center;
    color: var(--text-muted);
    font-size: var(--text-meta);
    margin-top: var(--space-2);
    flex-wrap: wrap;
  }
  .meta span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
    flex-wrap: wrap;
  }
  .plan,
  .move {
    color: var(--accent);
    font-size: var(--text-meta);
    padding: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .done {
    opacity: var(--opacity-done-card);
  }
  .done .title {
    text-decoration: line-through;
  }
  .preview {
    opacity: var(--drag-preview-opacity, 0.55);
    border-color: var(--project-color);
    pointer-events: none;
  }
  .faded {
    opacity: var(--drag-source-opacity, 0.3);
  }
  .compact {
    padding: var(--space-2);
  }
  .compact .meta {
    font-size: var(--text-chip);
  }
</style>
