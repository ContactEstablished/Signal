<script lang="ts">
  import { minuteLabel } from '../../domain/time-display';
  import { Temporal } from '@js-temporal/polyfill';
  import { onMount, untrack } from 'svelte';
  import { X, ExternalLink, Check, Trash2 } from 'lucide-svelte';
  import LinkedTaskPicker from './LinkedTaskPicker.svelte';
  import MeetingChangePreview from './MeetingChangePreview.svelte';
  import {
    meetingStart,
    newMeetingDraft,
    timeChoices,
    meetingTimeZones,
    meetingWeekday,
    weeklyStartDate,
  } from '../../domain/agenda-calendar';
  import { renderMarkdown } from '../../domain/markdown';
  import type { ProjectRecord } from '../../domain/types';
  import type {
    AgendaTask,
    Attendance,
    MeetingChange,
    MeetingDetail,
    MeetingFields,
    MeetingPreview,
    MeetingWriteResult,
    SeriesDraft,
  } from '../../domain/agenda';
  let {
    projects,
    projectId,
    nowUtc,
    timeZone,
    detail = null,
    pending = false,
    recovery = false,
    error = '',
    onSave,
    onPreview,
    onSearch,
    onClose,
    onRetry,
    onOpenTask,
    onJoin,
    onSwitchTask,
  }: {
    projects: ProjectRecord[];
    projectId: string;
    nowUtc: string;
    timeZone: string;
    detail?: MeetingDetail | null;
    pending?: boolean;
    recovery?: boolean;
    error?: string;
    onSave: (c: MeetingChange, fp?: string) => Promise<MeetingWriteResult>;
    onPreview: (c: MeetingChange) => Promise<MeetingPreview>;
    onSearch: (q: {
      text: string;
      limit: number;
      cursor?: string;
    }) => Promise<{ items: AgendaTask[]; nextCursor: string | null }>;
    onClose: () => unknown;
    onRetry: () => unknown;
    onOpenTask: (id: string) => unknown;
    onJoin: (url: string) => unknown;
    onSwitchTask?: () => unknown;
  } = $props();
  let modal: HTMLDialogElement;
  let d = $state<SeriesDraft>(untrack(() => newMeetingDraft(nowUtc,
    meetingTimeZones.some((z) => z.value === timeZone) ? timeZone : meetingTimeZones[0].value))),
    owner = $state(untrack(() => projectId));
  let persisted = $state<MeetingDetail | null>(untrack(() => detail));
  let attendance = $state<Attendance>('unmarked'),
    notes = $state(''),
    scope = $state<'occurrence' | 'following'>('occurrence'),
    localError = $state(''),
    busy = $state(false),
    preview = $state<MeetingPreview | null>(null),
    change: MeetingChange | null = null;
  let baseline = '';
  let lastAttempt: MeetingChange | null = null;
  let discard = $state(false);
  let closeResolve: ((v: boolean) => void) | null = null;
  const locked = $derived(pending || recovery || busy);
  const occurrenceOnly = $derived(
    !!persisted?.series.is_recurring && scope === 'occurrence',
  );
  const date = $derived(d.start_local.split('T')[0]);
  const time = $derived(d.start_local.split('T')[1]);
  const hour = $derived(Number(time?.slice(0, 2)) % 12 || 12);
  const minute = $derived(time?.slice(3, 5) || '00');
  const period = $derived(Number(time?.slice(0, 2)) < 12 ? 'AM' : 'PM');
  const weekdayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const selectedWeekdays = $derived(d.repeat_weekdays ?? []);
  const firstDate = $derived.by(() => {
    if (d.repeat_rule !== 'weekly' || occurrenceOnly) return date;
    try { return weeklyStartDate(date, selectedWeekdays); } catch { return date; }
  });
  const choices = $derived.by(() => {
    try {
      return timeChoices(firstDate, time, d.time_zone);
    } catch {
      return [];
    }
  });
  const endLabel = $derived.by(() => {
    try {
      const c =
        choices.length === 1
          ? choices[0]
          : choices.find((c) => c.offset === d.start_offset);
      if (!c) return '';
      const end = Temporal.Instant.from(c.instant)
        .add({ minutes: d.duration_min })
        .toZonedDateTimeISO(d.time_zone);
      return `Ends ${end.toPlainDate()} at ${minuteLabel(end.hour * 60 + end.minute)} (${end.offset})`;
    } catch {
      return '';
    }
  });
  function previewLinks(node: HTMLElement) {
    const click = (e: MouseEvent) => {
      const a = (e.target as Element).closest<HTMLAnchorElement>('a[href]');
      if (a) {
        e.preventDefault();
        onJoin(a.href);
      }
    };
    node.addEventListener('click', click);
    return {
      destroy() {
        node.removeEventListener('click', click);
      },
    };
  }
  function value() {
    return JSON.stringify({ d, owner, attendance, notes });
  }
  function accept(result: MeetingDetail) {
    persisted = result;
    const o = result.occurrence;
    d = {
      title: o.title,
      starts_at: o.starts_at,
      start_local: o.start_local,
      start_offset: o.start_offset,
      time_zone: o.time_zone,
      duration_min: o.duration_min,
      link_url: o.link_url,
      agenda_md: o.agenda_md,
      notes_md: o.notes_md,
      reminder_min: o.reminder_min,
      show_in_day: o.show_in_day,
      task_ids: o.task_ids,
      repeat_rule: result.series.repeat_rule,
      repeat_weekdays: result.series.repeat_weekdays?.length
        ? [...result.series.repeat_weekdays]
        : [meetingWeekday(o.start_local.split('T')[0])],
      repeat_until: result.series.repeat_until,
      fold_policy: result.series.fold_policy,
    };
    attendance = o.attendance;
    notes = o.occurrence_notes_md;
    baseline = value();
  }
  onMount(() => {
    if (detail) accept(detail);
    else {
      if (choices.length === 2) d.start_offset = '';
      baseline = value();
    }
    modal.showModal();
    return () => closeResolve?.(false);
  });
  export async function requestClose(_reason?: unknown) {
    if (locked) return false;
    if (value() === baseline && !preview) return true;
    discard = true;
    return new Promise<boolean>((r) => (closeResolve = r));
  }
  function decide(v: boolean) {
    discard = false;
    closeResolve?.(v);
    closeResolve = null;
  }
  function wall(day: string, t: string) {
    d.start_local = `${day}T${t}`;
    d.start_offset = '';
    preview = null;
  }
  function setTime(h: number, m: string, meridiem: string) {
    const h24 = h % 12 + (meridiem === 'PM' ? 12 : 0);
    wall(date, `${String(h24).padStart(2, '0')}:${m}:00.000`);
  }
  function selectHour(h: number) {
    setTime(h, minute, h >= 7 && h <= 11 ? 'AM' : 'PM');
  }
  function toggleWeekday(day: number) {
    d.repeat_weekdays = selectedWeekdays.includes(day)
      ? selectedWeekdays.filter((v) => v !== day)
      : [...selectedWeekdays, day].sort((a, b) => a - b);
    d.start_offset = '';
    preview = null;
  }
  function fields(): SeriesDraft {
    if (d.repeat_rule === 'weekly' && !occurrenceOnly && !selectedWeekdays.length)
      throw new Error('Choose at least one repeat weekday.');
    const f = meetingStart(
      {
        date: firstDate,
        time,
        offset: choices.length === 1 ? choices[0].offset : d.start_offset,
      },
      d.time_zone,
    );
    if (!d.title.trim()) throw new Error('Enter a meeting title.');
    return {
      ...d,
      ...f,
      title: d.title.trim(),
      link_url: d.link_url?.trim() || null,
    };
  }
  function finish(c: MeetingChange, r: MeetingWriteResult) {
    preview = null;
    change = null;
    lastAttempt = null;
    localError = '';
    if (r.detail) {
      const previous = d;
      accept(r.detail);
      if (c.action === 'record') d = previous;
      if (c.action !== 'record') {
        busy = false;
        onClose();
      }
    } else {
      baseline = value();
      busy = false;
      onClose();
    }
  }
  export function acceptRetry(r: MeetingWriteResult) {
    if (lastAttempt) finish(lastAttempt, r);
  }
  async function commit(c: MeetingChange, fp?: string) {
    if (locked) return;
    busy = true;
    localError = '';
    lastAttempt = c;
    try {
      finish(c, await onSave(c, fp));
    } catch (e) {
      localError =
        typeof e === 'object' && e && 'message' in e
          ? String(e.message)
          : String(e);
      preview = null;
    } finally {
      busy = false;
    }
  }
  async function prepare(c: MeetingChange) {
    if (locked || preview || discard) return;
    localError = '';
    if (c.action === 'remove' || c.action === 'edit_following') {
      busy = true;
      try {
        preview = await onPreview(c);
        change = c;
      } catch (e) {
        localError =
          typeof e === 'object' && e && 'message' in e
            ? String(e.message)
            : String(e);
      } finally {
        busy = false;
      }
    } else await commit(c);
  }
  function save() {
    try {
      const draft = fields();
      if (!persisted) {
        void prepare({
          action: 'create',
          payload: { projectId: owner, draft },
        });
        return;
      }
      const ref = persisted.occurrence.ref,
        expectedRevision = persisted.occurrence.revision;
      const occurrence = { attendance, occurrence_notes_md: notes };
      if (
        scope === 'following' ||
        draft.repeat_rule !== persisted.series.repeat_rule
      ) {
        void prepare({
          action: 'edit_following',
          payload: { ref, expectedRevision, draft, occurrence },
        });
      } else {
        const { repeat_rule, repeat_weekdays, repeat_until, fold_policy, ...single } = draft;
        void prepare({
          action: 'edit_occurrence',
          payload: { ref, expectedRevision, draft: single as MeetingFields, occurrence },
        });
      }
    } catch (e) {
      localError = String(e);
    }
  }
  function record() {
    if (!persisted) return;
    void prepare({
      action: 'record',
      payload: {
        ref: persisted.occurrence.ref,
        expectedRevision: persisted.occurrence.revision,
        attendance,
        occurrence_notes_md: notes,
      },
    });
  }
  function remove() {
    if (!persisted) return;
    void prepare({
      action: 'remove',
      payload: {
        ref: persisted.occurrence.ref,
        expectedRevision: persisted.occurrence.revision,
        scope: !persisted.series.is_recurring ? 'one_off' : scope,
      },
    });
  }
</script>

<dialog
  bind:this={modal}
  class="agenda-dialog"
  class:detail-mode={!!detail}
  oncancel={(e) => {
    e.preventDefault();
    onClose();
  }}
>
  <button
    class="dialog-x"
    aria-label="Close meeting"
    disabled={locked}
    onclick={onClose}><X /></button
  >
  <div class="meeting-heading">
    <span class="eyebrow"
      >{persisted ? persisted.occurrence.project_name : 'New meeting'}</span
    >{#if !persisted && onSwitchTask}<button
        class="outline"
        disabled={locked}
        onclick={onSwitchTask}>Task</button
      >{/if}
  </div>
  <h2>{persisted ? persisted.occurrence.title : 'Schedule a meeting'}</h2>
  {#if discard}<section class="discard">
      <h3>Discard unsaved meeting changes?</h3>
      <div class="agenda-actions">
        <button class="outline" onclick={() => decide(false)}
          >Keep editing</button
        ><button class="outline danger" onclick={() => decide(true)}
          >Discard changes</button
        >
      </div>
    </section>{/if}
  <form
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    <fieldset disabled={locked || !!preview || discard}>
      <div class="meeting-columns">
        <div class="meeting-main">
          {#if !persisted}<label
              >Project<select bind:value={owner}
                >{#each projects as p}<option value={p.id}>{p.name}</option
                  >{/each}</select
              ></label
            >{/if}<label>Title<input bind:value={d.title} required /></label
          ><label
            >Agenda<textarea
              rows="3"
              bind:value={d.agenda_md}
              placeholder="One item per line"
            ></textarea></label
          ><label
            >Shared meeting notes<textarea
              rows="3"
              bind:value={d.notes_md}
              placeholder="Markdown supported"
            ></textarea></label
          >{#if d.notes_md}<div class="markdown-preview" use:previewLinks>
              {@html renderMarkdown(d.notes_md)}
            </div>{/if}<LinkedTaskPicker
            selected={d.task_ids}
            labels={persisted?.linked_tasks ?? []}
            disabled={locked}
            onChange={(ids) => (d.task_ids = ids)}
            {onSearch}
          />{#if persisted}<section class="occurrence">
              <h3>This occurrence</h3>
              <label
                >Attendance<select bind:value={attendance}
                  ><option value="unmarked">Unmarked</option><option
                    value="attended">Attended</option
                  ><option value="missed">Missed</option></select
                ></label
              ><label
                >Occurrence notes<textarea rows="3" bind:value={notes}
                ></textarea></label
              ><button class="outline" type="button" onclick={record}
                ><Check />Save attendance & notes</button
              >{#if notes}<div class="markdown-preview" use:previewLinks>
                  {@html renderMarkdown(notes)}
                </div>{/if}{#each persisted.linked_tasks as t}<button
                  class="task-link"
                  type="button"
                  onclick={() => onOpenTask(t.id)}
                  >{t.title} · {t.project_name} ↗</button
                >{/each}
            </section>{/if}
        </div>
        <aside class="meeting-rail">
          <label
            >When<input
              type="date"
              value={date}
              required
              oninput={(e) => wall(e.currentTarget.value, time)}
            /></label>
          <div class="meeting-time" role="group" aria-label="Meeting start time">
            <label>Hour<select aria-label="Meeting hour" value={hour}
              onchange={(e) => selectHour(Number(e.currentTarget.value))}>
              {#each Array.from({ length: 12 }, (_, i) => i + 1) as h}
                <option value={h}>{h}</option>
              {/each}
            </select></label>
            <label>Minute<select aria-label="Meeting minute" value={minute}
              onchange={(e) => setTime(hour, e.currentTarget.value, period)}>
              {#if Number(minute) % 5 !== 0}<option value={minute}>{minute} (saved)</option>{/if}
              {#each Array.from({ length: 12 }, (_, i) => String(i * 5).padStart(2, '0')) as m}
                <option value={m}>{m}</option>
              {/each}
            </select></label>
            <label>AM/PM<select aria-label="Meeting AM/PM" value={period}
              onchange={(e) => setTime(hour, minute, e.currentTarget.value)}>
              <option value="AM">AM</option><option value="PM">PM</option>
            </select></label>
          </div>
          <label
            >Timezone<select
              bind:value={d.time_zone}
              onchange={() => {
                d.start_offset = '';
                preview = null;
              }}
            >
              {#if !meetingTimeZones.some((z) => z.value === d.time_zone)}
                <option value={d.time_zone}>{d.time_zone} (saved)</option>
              {/if}
              {#each meetingTimeZones as z}<option value={z.value}>{z.label}</option>{/each}
            </select></label
          >{#if choices.length === 2}<label
              >This time occurs twice<select bind:value={d.start_offset}
                ><option value="">Choose UTC offset</option
                >{#each choices as c}<option value={c.offset}>{c.offset}</option
                  >{/each}</select
              ></label
            >{:else if !choices.length}<p class="error">
              Choose a valid local time and timezone.
            </p>{/if}<label
            >Duration (elapsed minutes)<input
              type="number"
              min="1"
              max="1440"
              step="1"
              bind:value={d.duration_min}
              required
            /></label
          >
          <div class="agenda-actions">
            {#each [15, 25, 30, 45, 55, 60] as minutes}<button
                class="outline"
                type="button"
                onclick={() => (d.duration_min = minutes)}
                >{minutes === 60 ? '1h' : `${minutes}m`}</button
              >{/each}
          </div>
          {#if endLabel}<p class="hint">{endLabel}</p>{/if}<label
            >Join link<input
              type="url"
              bind:value={d.link_url}
              placeholder="https://…"
            /></label
          >{#if persisted?.occurrence.link_url}<button
              class="outline"
              type="button"
              onclick={() => onJoin(persisted!.occurrence.link_url!)}
              ><ExternalLink />Join</button
            >{/if}{#if persisted && persisted.series.is_recurring}<label
              >Edit scope<select bind:value={scope}
                ><option value="occurrence">This occurrence</option><option
                  value="following">This and following</option
                ></select
              ></label
            >{/if}<label
            >Repeats<select bind:value={d.repeat_rule} disabled={occurrenceOnly}
              ><option value="none">Does not repeat</option><option
                value="daily">Daily</option
              ><option value="weekly">Weekly</option></select
            ></label
          >{#if d.repeat_rule === 'weekly'}
            <div class="repeat-weekdays" role="group" aria-label="Repeat on weekdays">
              {#each weekdayLabels as label, i}
                {#if i < 5 || selectedWeekdays.includes(i + 1)}
                  <button type="button" class="outline" class:chosen={selectedWeekdays.includes(i + 1)}
                    aria-pressed={selectedWeekdays.includes(i + 1)} disabled={occurrenceOnly}
                    onclick={() => toggleWeekday(i + 1)}>{label}</button>
                {/if}
              {/each}
            </div>
            {#if !selectedWeekdays.length}<p class="error">Choose at least one repeat weekday.</p>
            {:else if firstDate !== date}<p class="hint">First meeting: {firstDate}. Repeats on the selected days.</p>{/if}
          {/if}{#if d.repeat_rule !== 'none'}<label
              >Ends (blank means Never)<input
                type="date"
                value={d.repeat_until ?? ''}
                disabled={occurrenceOnly}
                oninput={(e) =>
                  (d.repeat_until = e.currentTarget.value || null)}
              /></label
            ><label
              >Repeated-hour preference<select
                bind:value={d.fold_policy}
                disabled={occurrenceOnly}
                ><option value="earlier">Earlier offset</option><option
                  value="later">Later offset</option
                ></select
              ></label
            >
            <p class="hint">
              Keeps its wall time in {d.time_zone}. Dates with a nonexistent
              time are skipped.
            </p>{/if}<label
            >Reminder (minutes before)<input
              type="number"
              min="0"
              step="1"
              bind:value={d.reminder_min}
            /></label
          >
          <p class="hint">Saved preference. Reminder delivery arrives in M6.</p>
          <label class="check"
            ><input type="checkbox" bind:checked={d.show_in_day} />Show in Your
            Day</label
          >
        </aside>
      </div>
      <div class="agenda-actions">
        <button class="primary" type="submit"
          >{persisted ? 'Save meeting' : 'Create meeting'}</button
        ><button class="outline" type="button" onclick={onClose}>Cancel</button
        >{#if persisted}<button
            class="outline danger"
            type="button"
            onclick={remove}
            ><Trash2 />{!persisted.series.is_recurring
              ? 'Delete'
              : 'Cancel occurrence / following'}</button
          >{/if}
      </div>
    </fieldset>
  </form>
  {#if preview}<MeetingChangePreview
      {preview}
      pending={locked}
      onApply={() => {
        if (change) void commit(change, preview!.fingerprint);
      }}
      onCancel={() => (preview = null)}
    />{/if}{#if error || localError}<p class="error" role="alert">
      {error || localError}
    </p>{/if}{#if recovery}<button
      class="primary"
      disabled={pending}
      onclick={onRetry}>Retry exact save</button
    >{/if}
</dialog>

<style>
  .meeting-time {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }
  .repeat-weekdays {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .repeat-weekdays button {
    flex: 1;
    padding: var(--space-2);
  }
  .repeat-weekdays .chosen {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--bg-raised);
  }
  .detail-mode {
    width: var(--modal-meeting-detail);
  }
  .meeting-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-right: var(--space-6);
    margin-bottom: var(--space-3);
  }
  .meeting-columns {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .detail-mode .meeting-columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--meeting-rail);
    gap: var(--space-5);
  }
  .meeting-main,
  .meeting-rail {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }
  .detail-mode .meeting-rail {
    background: var(--rail-bg);
    padding: var(--space-3);
    border-radius: var(--radius-card);
  }
  .occurrence {
    border-top: var(--line) solid var(--border);
    padding-top: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .check {
    flex-direction: row;
    align-items: center;
  }
  .check input {
    width: auto;
  }
  .hint {
    font-size: var(--text-meta);
    margin: 0;
  }
  .task-link {
    text-align: left;
    color: var(--accent);
    font-size: var(--text-label);
    padding: 0;
  }
  .markdown-preview {
    color: var(--text-2);
    font-size: var(--text-base);
    overflow-wrap: anywhere;
  }
  .discard {
    background: var(--warn-fill);
    padding: var(--space-4);
    border: var(--line) solid var(--warn-border);
    border-radius: var(--radius-card);
    margin-bottom: var(--space-4);
  }
</style>
