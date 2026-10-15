<script lang="ts">
  import { Circle, ExternalLink } from 'lucide-svelte';
  import type { Attendance, MeetingOccurrence, MeetingRef } from '../../domain/agenda';
  import MeetingQuickActions from '../meetings/MeetingQuickActions.svelte';
  import { errorMessage } from '../../domain/types';
  let {
    meeting,
    timeZone,
    displayDate,
    next = false,
    past = false,
    onOpen,
    onJoin,
    pending = false,
    onRecordMeeting,
  }: {
    meeting: MeetingOccurrence;
    timeZone: string;
    displayDate?: string;
    next?: boolean;
    past?: boolean;
    onOpen: (ref: MeetingRef) => unknown;
    onJoin?: (url: string) => unknown;
    pending?: boolean;
    onRecordMeeting?: (meeting: MeetingOccurrence, attendance: Attendance) => Promise<void>;
  } = $props();
  let busy = $state(false), error = $state('');
  async function mark(attendance: Attendance) {
    if (busy || pending || !onRecordMeeting) return;
    busy = true; error = '';
    try { await onRecordMeeting(meeting, attendance); }
    catch (e) { error = errorMessage(e); }
    finally { busy = false; }
  }
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

<div class="meeting meeting-status-host" class:next class:past class:canceled={meeting.attendance === 'canceled'} data-color={meeting.project_color} data-attendance={meeting.attendance}>
  <button
    class="open"
    title={`${meeting.title} · ${meeting.project_name} · ${time}`}
    onclick={() => onOpen(meeting.ref)}
    ><span class="label"
      ><Circle />{continues ? 'Continues from ' + startDate : 'Meeting'} · {time}</span
    ><strong>{meeting.title}</strong>{#if meeting.attendance === 'attended' || meeting.attendance === 'canceled'}<span class="outcome" class:attended={meeting.attendance === 'attended'}>{meeting.attendance === 'attended' ? 'Attended' : 'Canceled'}</span>{/if}</button
  >{#if onRecordMeeting}<MeetingQuickActions title={meeting.title} attendance={meeting.attendance} disabled={busy || pending} onMark={mark} />{/if}
  {#if onJoin && meeting.link_url && meeting.attendance !== 'canceled'}<button
      class:primary={next}
      class="join"
      onclick={() => onJoin?.(meeting.link_url!)}>Join<ExternalLink /></button
    >{/if}
</div>
{#if error}<p class="error" role="alert">{error}</p>{/if}

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
  .canceled { background: var(--surface); border-color: var(--border-dashed); }
  .canceled .open strong { color: var(--text-muted); text-decoration: line-through; }
  .canceled .label { color: var(--text-muted); }
  .outcome { color: var(--text-muted); font-size: var(--text-chip); }
  .outcome.attended { color: var(--lime); }
  .error { color: var(--danger); }
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
