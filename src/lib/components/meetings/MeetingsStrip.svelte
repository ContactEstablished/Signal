<svelte:options runes={true} />

<script lang="ts">
  import type { Meeting, ProjectRecord } from '../../domain/types';
  import { meetingLayout } from '../../domain/board';
  import { dateAt } from '../../domain/clock';
  let {
    meetings,
    project,
    selectedDate,
    nowUtc,
    timeZone,
    onPreview,
  }: {
    meetings: Meeting[];
    project: ProjectRecord;
    selectedDate: string;
    nowUtc: string;
    timeZone: string;
    onPreview: (message: string) => void;
  } = $props();
  let layout = $derived(meetingLayout(meetings, selectedDate, timeZone));
  let visible = $derived(layout.filter((r) => !r.outside));
  let rows = $derived(Math.max(1, ...visible.map((r) => r.lane + 1)));
  let localTime = $derived(
    new Intl.DateTimeFormat('en-GB', {
      timeZone,
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    }).format(new Date(nowUtc)),
  );
  let minute = $derived(
    Number(localTime.slice(0, 2)) * 60 + Number(localTime.slice(3)),
  );
  let showNow = $derived(
    dateAt(nowUtc, timeZone) === selectedDate &&
      minute >= 540 &&
      minute <= 1080,
  );
  function time(value: string) {
    return new Intl.DateTimeFormat('en-US', {
      timeZone,
      hour: 'numeric',
      minute: '2-digit',
    }).format(new Date(value));
  }
</script>

<section class="strip" data-color={project.color} aria-label="Project meetings">
  <div class="caption">
    <strong>Meetings</strong><span
      >{new Intl.DateTimeFormat('en-US', {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        timeZone: 'UTC',
      }).format(new Date(selectedDate + 'T12:00:00Z'))} · {layout.length}
      {layout.length === 1 ? 'meeting' : 'meetings'}</span
    >
  </div>
  <div
    class="timeline"
    style:height={`calc(var(--space-modal) + ${rows} * var(--space-8))`}
  >
    <div class="hours">
      {#each Array.from({ length: 10 }, (_, i) => 9 + i) as hour}<span
          style:left={`${((hour - 9) / 9) * 100}%`}
          >{hour.toString().padStart(2, '0')}</span
        >{/each}
    </div>
    {#each visible as item}<button
        class="meeting"
        style:left={`${item.left}%`}
        style:width={`${item.width}%`}
        style:top={`calc(var(--space-1) + ${item.lane} * var(--space-8))`}
        title={`${item.meeting.title} · ${time(item.meeting.starts_at)}`}
        onclick={() =>
          onPreview(`${item.meeting.title} · Meeting details arrive in M4.`)}
        >{item.meeting.title}</button
      >{/each}{#if showNow}<div
        class="now"
        style:left={`${((minute - 540) / 540) * 100}%`}
      >
        <span>{localTime}</span>
      </div>{/if}
  </div>
  {#each layout.filter((r) => r.outside) as item}<button
      class="outside"
      onclick={() =>
        onPreview(`${item.meeting.title} · Meeting details arrive in M4.`)}
      >{time(item.meeting.starts_at)} · {item.meeting.title} · Outside 09–18</button
    >{/each}
</section>

<style>
  .strip {
    background: var(--bg-raised);
    border: var(--line) solid var(--border-section);
    border-radius: var(--radius-panel);
    padding: var(--meetings-caption-padding);
    margin-bottom: var(--board-top);
    display: grid;
    grid-template-columns: var(--meetings-label-width) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: center;
  }
  .caption {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    align-items: flex-start;
  }
  .caption strong {
    font: var(--weight-semibold) var(--text-meta) var(--font-heading);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--project-color);
  }
  .caption span {
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
  .timeline {
    position: relative;
    margin-right: var(--space-small);
  }
  .hours {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: var(--space-4);
    border-top: var(--line) solid var(--border-section);
  }
  .hours span {
    position: absolute;
    font: var(--weight-hour) var(--text-meta) var(--font-heading);
    color: var(--text-faint);
    transform: translateX(-50%);
  }
  .meeting {
    position: absolute;
    height: var(--space-7);
    border: var(--line) solid var(--project-meeting-border);
    background: var(--project-meeting-fill);
    color: var(--project-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-meta);
    text-align: left;
  }
  .now {
    position: absolute;
    top: 0;
    bottom: var(--space-5);
    border-left: var(--line) solid var(--warn);
    pointer-events: none;
  }
  .now span {
    position: absolute;
    top: calc(-1 * var(--space-3));
    left: 0;
    transform: translateX(-50%);
    color: var(--warn);
    font: var(--weight-semibold) var(--text-meta) var(--font-heading);
  }
  .outside {
    grid-column: 2;
    font-size: var(--text-meta);
    color: var(--project-color);
    display: block;
  }
</style>
