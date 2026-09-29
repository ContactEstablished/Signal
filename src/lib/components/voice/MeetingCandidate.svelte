<script lang="ts">
  import type { SeriesDraft } from '../../domain/agenda';
  import { meetingStart, timeChoices } from '../../domain/agenda-calendar';
  import { meetingIssue } from '../../voice/review';
  let { meeting = $bindable(), onChange }: { meeting: SeriesDraft; onChange: () => void } = $props();
  const choices = $derived.by(() => {
    try { return timeChoices(meeting.start_local.slice(0,10), meeting.start_local.slice(11), meeting.time_zone); }
    catch { return []; }
  });
  const issue = $derived(meetingIssue({kind:'meeting', meeting} as import('../../voice/api').Candidate));
  function resolve(resetOffset = false) {
    if (resetOffset) meeting.start_offset = '';
    try {
      Object.assign(meeting, meetingStart({ date:meeting.start_local.slice(0,10), time:meeting.start_local.slice(11), offset:meeting.start_offset }, meeting.time_zone));
    } catch { meeting.starts_at = ''; }
    onChange();
  }
  function weekday(day: number, enabled: boolean) {
    meeting.repeat_weekdays = enabled ? [...(meeting.repeat_weekdays ?? []), day].sort() : (meeting.repeat_weekdays ?? []).filter(d => d !== day);
    onChange();
  }
</script>
<div class="fields">
  <label>Start<input type="datetime-local" step="1" bind:value={meeting.start_local} onchange={() => resolve(true)} /></label>
  <label>Timezone<input bind:value={meeting.time_zone} onchange={() => resolve(true)} /></label>
  <label>Duration · minutes<input type="number" min="1" max="1440" bind:value={meeting.duration_min} oninput={onChange} /></label>
</div>
{#if choices.length > 1}<label>Repeated time · UTC offset<select bind:value={meeting.start_offset} onchange={() => resolve()}><option value="">Choose offset…</option>{#each choices as c}<option value={c.offset}>{c.offset}</option>{/each}</select></label>{/if}
<label>Meeting link<input type="url" value={meeting.link_url ?? ''} oninput={e => {meeting.link_url = e.currentTarget.value || null; onChange();}} /></label>
<label>Agenda<textarea rows="3" bind:value={meeting.agenda_md} oninput={onChange}></textarea></label>
<label>Notes<textarea rows="2" bind:value={meeting.notes_md} oninput={onChange}></textarea></label>
<div class="fields">
  <label>Repeat<select bind:value={meeting.repeat_rule} onchange={onChange}>{#if !['none','daily','weekly'].includes(meeting.repeat_rule)}<option value={meeting.repeat_rule}>Needs correction</option>{/if}<option value="none">Does not repeat</option><option value="daily">Daily</option><option value="weekly">Weekly</option></select></label>
  {#if meeting.repeat_rule !== 'none'}<label>Repeat through<input type="date" value={meeting.repeat_until ?? ''} onchange={e => {meeting.repeat_until = e.currentTarget.value || null; onChange();}} /></label><label>When clocks repeat<select bind:value={meeting.fold_policy} onchange={onChange}><option value="earlier">Earlier offset</option><option value="later">Later offset</option></select></label>{/if}
</div>
{#if meeting.repeat_rule === 'weekly'}<div class="weekdays">{#each ['Mon','Tue','Wed','Thu','Fri','Sat','Sun'] as day, index}<label><input type="checkbox" checked={meeting.repeat_weekdays?.includes(index + 1)} onchange={e => weekday(index + 1,e.currentTarget.checked)} />{day}</label>{/each}</div>{/if}
<p class="defaults">Defaults: 15-minute reminder preference; visible in Your Day. Reminder delivery follows Signal’s existing support.</p>
{#if issue}<p class="warning" role="status">{issue} This meeting cannot be created until corrected.</p>{/if}
<style>
  label { display: grid; gap: 6px; margin: 12px 0; }
  input, select, textarea { width: 100%; }
  .fields { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 12px; }
  .weekdays { display: flex; gap: 10px; flex-wrap: wrap; }
  .weekdays label { display: flex; align-items: center; }
  .weekdays input { width: auto; }
  .warning { color: var(--warn); }
  .defaults { color: var(--text-muted); font-size: var(--text-label); }
</style>
