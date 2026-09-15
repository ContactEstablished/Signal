<script lang="ts">
  import { Circle, ExternalLink } from 'lucide-svelte';
  import type { MeetingOccurrence, MeetingRef } from '../../domain/agenda';
  let {
    meeting,
    timeZone,
    displayDate,
    next = false,
    past = false,
    onOpen,
    onJoin,
  }: {
    meeting: MeetingOccurrence;
    timeZone: string;
    displayDate?: string;
    next?: boolean;
    past?: boolean;
    onOpen: (ref: MeetingRef) => unknown;
    onJoin?: (url: string) => unknown;
  } = $props();
  const startDate = $derived(
    new Intl.DateTimeFormat('en-CA', {
      timeZone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).format(new Date(meeting.starts_at)),
  );
  const continues = $derived(!!displayDate && startDate < displayDate);
  const time = $derived(
    new Intl.DateTimeFormat('en-US', {
      timeZone,
      hour12: true,
      hour: 'numeric',
      minute: '2-digit',
    }).format(new Date(meeting.starts_at)),
  );
</script>

<div class="meeting" class:next class:past data-color={meeting.project_color}>
  <button
    class="open"
    title={`${meeting.title} · ${meeting.project_name} · ${time}`}
    onclick={() => onOpen(meeting.ref)}
    ><span class="label"
      ><Circle />{continues ? 'Continues from ' + startDate : 'Meeting'} · {time}</span
    ><strong>{meeting.title}</strong></button
  >{#if onJoin && meeting.link_url}<button
      class:primary={next}
      class="join"
      onclick={() => onJoin?.(meeting.link_url!)}>Join<ExternalLink /></button
    >{/if}
</div>

<style>
  .meeting {
    border: var(--line) dashed var(--project-meeting-border);
    background: var(--project-meeting-fill);
    border-radius: var(--radius-card);
    min-width: 0;
    padding: var(--space-2);
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .open {
    display: flex;
    flex-direction: column;
    text-align: left;
    gap: var(--space-1);
    padding: 0;
    min-width: 0;
    flex: 1;
  }
  .label {
    color: var(--project-color);
    font-size: var(--text-chip);
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-wrap: wrap;
  }
  .open strong {
    font-weight: var(--weight-medium);
    font-size: var(--text-label);
    overflow-wrap: anywhere;
  }
  .next {
    border-style: solid;
    border-color: var(--accent);
  }
  .past {
    opacity: var(--opacity-past-meeting);
  }
  .join {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--project-color);
    font-size: var(--text-label);
  }
  .join.primary {
    color: var(--on-accent);
  }
</style>
