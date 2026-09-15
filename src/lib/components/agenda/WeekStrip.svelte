<script lang="ts">
  import type { AgendaDay, MeetingRef } from '../../domain/agenda';
  let {
    days,
    today,
    onOpenTask,
    onOpenMeeting,
  }: {
    days: AgendaDay[];
    today: string;
    onOpenTask: (id: string) => unknown;
    onOpenMeeting: (r: MeetingRef) => unknown;
  } = $props();
</script>

<div class="strip">
  {#each days as day}<section class:today={day.date === today}>
      <h3>
        {new Intl.DateTimeFormat('en-US', {
          weekday: 'short',
          day: 'numeric',
          timeZone: 'UTC',
        }).format(new Date(day.date + 'T12:00Z'))}
      </h3>
      {#each day.tasks as t}<button
          class:done={t.status === 'done'}
          data-color={t.project_color}
          title={t.title}
          onclick={() => onOpenTask(t.id)}>{t.title}</button
        >{/each}{#each day.meetings as m}<button
          data-color={m.project_color}
          title={`${m.title} · ${m.project_name}`}
          onclick={() => onOpenMeeting(m.ref)}>○ {m.title}</button
        >{/each}{#if !day.tasks.length && !day.meetings.length}<span
          class="muted">Nothing due</span
        >{/if}
    </section>{/each}
</div>

<style>
  .strip {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: var(--week-gap);
    margin: var(--space-6) 0;
  }
  .strip section {
    border: var(--line) solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    padding: var(--space-3);
    min-width: 0;
  }
  .strip .today {
    background: var(--surface-2);
    border-color: var(--border-today);
  }
  h3 {
    font-size: var(--text-meta);
    color: var(--text-muted);
    margin: 0 0 var(--space-2);
  }
  button {
    display: block;
    width: 100%;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--project-meeting-fill);
    color: var(--project-color);
    border-radius: var(--radius-chip);
    padding: var(--space-1);
    margin-bottom: var(--space-1);
    font-size: var(--text-meta);
  }
  .done {
    text-decoration: line-through;
    opacity: var(--opacity-done-card);
  }
  .muted {
    font-size: var(--text-meta);
  }
</style>
