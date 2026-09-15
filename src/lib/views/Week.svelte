<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import { ChevronLeft, ChevronRight, Plus } from 'lucide-svelte';
  import AgendaTaskCard from '../components/agenda/AgendaTaskCard.svelte';
  import MeetingPill from '../components/agenda/MeetingPill.svelte';
  import type {
    WeekSnapshot,
    AgendaTask,
    MeetingRef,
    DueDestination,
  } from '../domain/agenda';
  import { addDays } from '../domain/agenda-calendar';
  import { errorMessage } from '../domain/types';
  let {
    snapshot,
    timeZone,
    pending = false,
    loading = false,
    error = '',
    onRetry,
    onOpenTask,
    onOpenMeeting,
    onNewMeeting,
    onNavigateWeek,
    onMoveDue,
  }: {
    snapshot: WeekSnapshot | null;
    timeZone: string;
    pending?: boolean;
    loading?: boolean;
    error?: string;
    onRetry: () => unknown;
    onOpenTask: (id: string) => unknown;
    onOpenMeeting: (r: MeetingRef) => unknown;
    onNewMeeting: () => unknown;
    onNavigateWeek: (d: string) => unknown;
    onMoveDue: (
      id: string,
      d: DueDestination,
    ) => Promise<'committed' | 'cancelled'>;
  } = $props();
  let drag = $state<{
      task: AgendaTask;
      x: number;
      y: number;
      pointer: number;
      active: boolean;
      target: string | null;
    } | null>(null),
    busy = $state(false),
    moveError = $state('');
  let viewport: HTMLElement;
  let capture: HTMLElement | null = null;
  let frame = 0,
    pointerX = 0,
    pointerY = 0;
  let announcement = $state('');
  $effect(() => {
    const identity = snapshot?.query;
    untrack(() => {
      if (identity && !busy) endDrag();
    });
  });
  function endDrag() {
    cancelAnimationFrame(frame);
    frame = 0;
    const element = capture,
      pointer = drag?.pointer;
    capture = null;
    drag = null;
    if (
      element &&
      pointer !== undefined &&
      element.hasPointerCapture?.(pointer)
    )
      element.releasePointerCapture(pointer);
  }
  onDestroy(endDrag);
  function hitTarget() {
    if (drag)
      drag.target =
        document
          .elementFromPoint(pointerX, pointerY)
          ?.closest<HTMLElement>('[data-due-target]')?.dataset.dueTarget ??
        null;
  }
  function autoScroll() {
    if (!drag?.active || busy) {
      frame = 0;
      return;
    }
    const box = viewport.getBoundingClientRect();
    const edge = 36;
    const dy =
      pointerY < box.top + edge ? -10 : pointerY > box.bottom - edge ? 10 : 0;
    if (dy && pointerX >= box.left && pointerX <= box.right) {
      viewport.scrollTop += dy;
      hitTarget();
    }
    frame = requestAnimationFrame(autoScroll);
  }
  function begin(e: PointerEvent, task: AgendaTask) {
    if (pending || busy || e.button !== 0) return;
    e.preventDefault();
    capture = e.currentTarget as HTMLElement;
    capture.setPointerCapture?.(e.pointerId);
    drag = {
      task,
      x: e.clientX,
      y: e.clientY,
      pointer: e.pointerId,
      active: false,
      target: null,
    };
  }
  function move(e: PointerEvent) {
    if (!drag || drag.pointer !== e.pointerId) return;
    if (Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 5)
      drag.active = true;
    pointerX = e.clientX;
    pointerY = e.clientY;
    if (drag.active) {
      hitTarget();
      if (!frame) frame = requestAnimationFrame(autoScroll);
    }
  }
  const destination = (target: string): DueDestination =>
    target === 'none'
      ? { kind: 'none' }
      : target === 'later'
        ? { kind: 'picker' }
        : { kind: 'date', date: target };
  async function change(task: AgendaTask, d: DueDestination) {
    if (busy || pending) return;
    busy = true;
    moveError = '';
    try {
      const result = await onMoveDue(task.id, d);
      announcement =
        result === 'cancelled'
          ? `Deadline unchanged for ${task.title}.`
          : `Deadline updated for ${task.title}.`;
    } catch (e) {
      moveError = errorMessage(e);
    } finally {
      busy = false;
      endDrag();
      // Buttons are disabled while moving. Restore focus only after the
      // committed/source card is rendered and its controls are enabled.
      await tick();
      if (viewport.isConnected) {
        const card = [
          ...viewport.querySelectorAll<HTMLElement>('[data-task-id]'),
        ].find(
          (el) =>
            el.dataset.taskId === task.id && !el.classList.contains('preview'),
        );
        (
          card?.querySelector<HTMLButtonElement>('.move:not(:disabled)') ??
          viewport
        ).focus();
      }
    }
  }
  function release(e: PointerEvent) {
    if (!drag || drag.pointer !== e.pointerId) return;
    const d = drag;
    if (d.active && d.target) void change(d.task, destination(d.target));
    else endDrag();
  }
</script>

<svelte:window
  onpointermove={move}
  onpointerup={release}
  onlostpointercapture={() => {
    if (!busy) endDrag();
  }}
  onpointercancel={() => {
    if (!busy) endDrag();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && !busy) endDrag();
  }}
/>
<section
  bind:this={viewport}
  class="week-view"
  aria-label="Project week"
  tabindex="-1"
>
  <span class="sr-only" role="status">{announcement}</span>
  <header>
    {#if snapshot}<div class="dates">
        <button
          class="outline"
          aria-label="Previous week"
          disabled={busy}
          onclick={() => onNavigateWeek(addDays(snapshot!.week_start, -7))}
          ><ChevronLeft /></button
        >
        <h2>{snapshot.week_start} – {addDays(snapshot.week_start, 6)}</h2>
        <button
          class="outline"
          aria-label="Next week"
          disabled={busy}
          onclick={() => onNavigateWeek(addDays(snapshot!.week_start, 7))}
          ><ChevronRight /></button
        >
      </div>{/if}<button class="outline" onclick={onNewMeeting}
      ><Plus />New meeting</button
    >
  </header>
  {#if error || moveError}<p class="error" role="alert">
      {error || moveError}
      <button class="outline" onclick={onRetry}>Retry</button>
    </p>{/if}
  {#if !snapshot}<p>
      {loading ? 'Loading week…' : 'Week is unavailable.'}
    </p>{:else}{#if snapshot.overdue_before_week.length}<details>
        <summary
          >{snapshot.overdue_before_week.length} earlier overdue tasks</summary
        >
        <div class="older">
          {#each snapshot.overdue_before_week as task}<AgendaTaskCard
              {task}
              deadlineZone={timeZone}
              compact
              disabled={busy || pending}
              onOpen={onOpenTask}
              onMove={() => change(task, { kind: 'picker' })}
            />{/each}
        </div>
      </details>{/if}
    <div class="grid">
      {#each snapshot.days as day}<section
          class="day"
          class:today={day.date === snapshot.today}
          data-due-target={day.date}
        >
          <h3>
            {new Intl.DateTimeFormat('en-US', {
              weekday: 'short',
              day: 'numeric',
              timeZone: 'UTC',
            }).format(new Date(day.date + 'T12:00Z'))}
          </h3>
          {#if day.date === snapshot.today}<span class="urgency">today</span
            >{:else if snapshot.overdue_counts[day.date]}<span class="urgency"
              >{snapshot.overdue_counts[day.date]} overdue</span
            >{/if}
          <div class="cards">
            {#each day.meetings as meeting (meeting.id)}<MeetingPill
                {meeting}
                displayDate={day.date}
                {timeZone}
                onOpen={onOpenMeeting}
              />{/each}{#each day.tasks as task (task.id)}<AgendaTaskCard
                {task}
                compact
                disabled={busy || pending}
                faded={drag?.active && drag.task.id === task.id}
                onOpen={onOpenTask}
                onMove={() => change(task, { kind: 'picker' })}
                onDrag={begin}
              />{/each}{#if drag?.active && drag.target === day.date}<AgendaTaskCard
                task={drag.task}
                compact
                preview
                onOpen={() => {}}
              />{/if}{#if !day.tasks.length && !day.meetings.length}<div
                class="empty"
              >
                Nothing due
              </div>{/if}
          </div>
        </section>{/each}
      <aside>
        {#each [{ title: 'Next week & later', target: 'later', tasks: snapshot.later }, { title: 'No date', target: 'none', tasks: snapshot.no_date }] as group}<section
            data-due-target={group.target}
          >
            <h3>{group.title}</h3>
            <div class="cards">
              {#each group.tasks as task (task.id)}<AgendaTaskCard
                  {task}
                  deadlineZone={timeZone}
                  compact
                  disabled={busy || pending}
                  faded={drag?.active && drag.task.id === task.id}
                  onOpen={onOpenTask}
                  onMove={() => change(task, { kind: 'picker' })}
                  onDrag={begin}
                />{/each}{#if drag?.active && drag.target === group.target}<AgendaTaskCard
                  task={drag.task}
                  compact
                  preview
                  onOpen={() => {}}
                />{/if}<button class="empty" disabled={!drag}
                >Drop here {group.target === 'none'
                  ? 'to clear deadline'
                  : 'to choose a date'}</button
              >
            </div>
          </section>{/each}
      </aside>
    </div>
    {#each snapshot.skipped_dates as warning}<p class="muted">
        {warning}
      </p>{/each}{/if}
</section>

<style>
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .week-view {
    min-height: 0;
    min-width: 0;
    flex: 1;
    overflow: auto;
  }
  header,
  .dates {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  header {
    justify-content: space-between;
    margin-bottom: var(--space-5);
  }
  header button {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  h2 {
    font-size: var(--text-label);
    margin: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr)) var(--week-later-width);
    gap: var(--week-gap);
  }
  .day {
    min-width: 0;
    padding: var(--space-2) var(--space-1);
    border-radius: var(--radius-card);
    min-height: var(--agenda-week-min-height);
  }
  h3 {
    font-size: var(--text-meta);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-muted);
    margin: 0 0 var(--space-2);
    overflow-wrap: anywhere;
  }
  .today {
    background: var(--today-wash);
  }
  .today h3 {
    color: var(--text);
  }
  .urgency {
    display: block;
    color: var(--warn);
    font-size: var(--text-chip);
    margin-bottom: var(--space-2);
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .empty {
    color: var(--text-faint);
    border: var(--line) dashed var(--border-dashed);
    border-radius: var(--radius-card);
    padding: var(--space-3);
    font-size: var(--text-meta);
    overflow-wrap: anywhere;
  }
  aside {
    min-width: 0;
  }
  aside section {
    margin-bottom: var(--space-5);
    padding: var(--space-2) 0;
  }
  .older {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-2);
    margin: var(--space-3) 0;
  }
  details {
    margin-bottom: var(--space-3);
  }
  summary {
    color: var(--warn);
    cursor: pointer;
  }
</style>
