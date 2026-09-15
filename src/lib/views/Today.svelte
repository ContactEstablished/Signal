<script lang="ts">
  import { Plus, ArrowRight } from 'lucide-svelte';
  import WeekStrip from '../components/agenda/WeekStrip.svelte';
  import AgendaTaskCard from '../components/agenda/AgendaTaskCard.svelte';
  import MeetingPill from '../components/agenda/MeetingPill.svelte';
  import type { TodaySnapshot, MeetingRef } from '../domain/agenda';
  let {
    snapshot,
    nowUtc,
    timeZone,
    loading = false,
    pending = false,
    error = '',
    onRetry,
    onOpenTask,
    onOpenMeeting,
    onPlanTask,
    onJoin,
    onNewMeeting,
    onOpenYourDay,
  }: {
    snapshot: TodaySnapshot | null;
    nowUtc: string;
    timeZone: string;
    loading?: boolean;
    pending?: boolean;
    error?: string;
    onRetry: () => unknown;
    onOpenTask: (id: string) => unknown;
    onOpenMeeting: (r: MeetingRef) => unknown;
    onPlanTask: (id: string) => unknown;
    onJoin: (url: string) => unknown;
    onNewMeeting: () => unknown;
    onOpenYourDay: () => unknown;
  } = $props();
  const next = $derived(
    snapshot?.meetings.find(
      (m) =>
        Date.parse(m.starts_at) + m.duration_min * 60000 > Date.parse(nowUtc),
    )?.id,
  );
  const largestHours = $derived(
    Math.max(0, ...(snapshot?.hours_this_week.map((p) => p.hours) ?? [])),
  );
</script>

<main class="today-page">
  <header>
    <div>
      <h1>Today</h1>
      {#if snapshot}<p>
          {new Intl.DateTimeFormat('en-US', {
            dateStyle: 'full',
            timeZone: 'UTC',
          }).format(new Date(snapshot.today + 'T12:00Z'))} · {snapshot.counts
            .overdue} overdue · {snapshot.counts.due_today} due today · {snapshot
            .counts.meetings}
          {snapshot.counts.meetings === 1 ? 'meeting' : 'meetings'}
        </p>{/if}
    </div>
    <button class="outline" onclick={onOpenYourDay}
      >Plan in Your Day <ArrowRight /></button
    >
  </header>
  {#if error}<p class="error" role="alert">
      {error} <button class="outline" onclick={onRetry}>Retry</button>
    </p>{/if}{#if !snapshot}<p role="status">
      {loading
        ? 'Loading today…'
        : 'Today is unavailable. Retry to load your calendar.'}
    </p>{:else}<WeekStrip
      days={snapshot.days}
      today={snapshot.today}
      {onOpenTask}
      {onOpenMeeting}
    />
    <div class="columns">
      <div>
        {#each [{ key: 'overdue', title: 'Overdue', tasks: snapshot.overdue }, { key: 'due_today', title: 'Due today', tasks: snapshot.due_today }, { key: 'tomorrow', title: 'Tomorrow', tasks: snapshot.tomorrow }] as group}<section
            class:overdue={group.key === 'overdue'}
            class:tomorrow={group.key === 'tomorrow'}
          >
            <h2>{group.title} <span>{group.tasks.length}</span></h2>
            <div class="list">
              {#each group.tasks as task (task.id)}<AgendaTaskCard
                  {task}
                  displayDate={snapshot.today}
                  disabled={pending}
                  onOpen={onOpenTask}
                  onPlan={!task.planned_ranges.some(
                    (r) => r.date === snapshot!.today,
                  )
                    ? onPlanTask
                    : undefined}
                />{:else}<p>No {group.title.toLowerCase()} tasks.</p>{/each}
            </div>
          </section>{/each}
      </div>
      <aside>
        <section>
          <h2>
            Meetings <button
              class="outline"
              aria-label="New meeting"
              onclick={onNewMeeting}><Plus /></button
            >
          </h2>
          <div class="list">
            {#each snapshot.meetings as meeting (meeting.id)}<MeetingPill
                {meeting}
                {timeZone}
                displayDate={snapshot.today}
                next={meeting.id === next}
                past={Date.parse(meeting.starts_at) +
                  meeting.duration_min * 60000 <=
                  Date.parse(nowUtc)}
                onOpen={onOpenMeeting}
                {onJoin}
              />{:else}<p>No meetings today.</p>{/each}
          </div>
        </section>
        <section>
          <h2>Hours this week</h2>
          {#each snapshot.hours_this_week as p}<div
              class="hours"
              data-color={p.project_color}
            >
              <div>
                <span><span class="dot"></span> {p.project_name}</span><strong
                  >{Number(p.hours.toFixed(2))}h</strong
                >
              </div>
              <div class="track">
                <span
                  style:width={`${largestHours ? (p.hours / largestHours) * 100 : 0}%`}
                ></span>
              </div>
            </div>{/each}
          <p class="hint">Completed time entries · Monday–Sunday</p>
        </section>
        {#each snapshot.skipped_dates as warning}<p class="hint">
            {warning}
          </p>{/each}
      </aside>
    </div>{/if}
</main>

<style>
  .today-page {
    height: calc(100vh - var(--header-height));
    overscroll-behavior: contain;
    padding: var(--padding-page);
    overflow: auto;
    min-height: 0;
    flex: 1;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  header p {
    margin: 0;
  }
  header button {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);
    gap: var(--space-6);
  }
  section {
    margin-bottom: var(--space-6);
  }
  h2 {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    font-size: var(--text-label);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-muted);
  }
  h2 span {
    color: var(--text-faint);
  }
  h2 button {
    margin-left: auto;
    padding: var(--space-1);
  }
  .overdue h2 {
    color: var(--warn);
  }
  .overdue :global(article) {
    border-color: var(--warn-border);
  }
  .tomorrow {
    opacity: var(--opacity-tomorrow, 0.8);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .hours {
    margin-bottom: var(--space-4);
  }
  .hours > div {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .hours strong {
    font-family: var(--font-heading);
    font-size: var(--text-meta);
  }
  .track {
    margin-top: var(--space-2);
    height: var(--space-1);
    background: var(--surface-3);
    border-radius: var(--radius-chip);
    overflow: hidden;
  }
  .track span {
    background: var(--project-color);
  }
  .hint {
    font-size: var(--text-meta);
    color: var(--text-faint);
  }
</style>
