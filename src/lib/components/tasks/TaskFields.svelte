<svelte:options runes={true} />

<script lang="ts">
  import TimeInput from '../TimeInput.svelte';
  import {
    statuses,
    statusLabels,
    type TaskPatch,
    type TaskStatus,
    type Priority,
  } from '../../domain/types';
  import {
    deadlineFields,
    deadlineChoices,
    deadlineToUtc,
    type DeadlineChoice,
  } from '../../domain/deadlines';
  import { estimateValue } from '../../domain/task-draft';
  let {
    value,
    alerts,
    timeZone,
    onInput,
    onCommit,
    onAlerts,
    onInvalid,
    onCancel = () => {},
    disabled = false,
    rail = false,
  }: {
    value: TaskPatch;
    alerts: number[];
    timeZone: string;
    onInput: (patch: TaskPatch) => void;
    onCommit: (keys: (keyof TaskPatch)[]) => void;
    onAlerts: (values: number[]) => void;
    onInvalid: (invalid: boolean) => void;
    onCancel?: (key: keyof TaskPatch) => void;
    disabled?: boolean;
    rail?: boolean;
  } = $props();
  let date = $state(''),
    time = $state(''),
    offset = $state(''),
    estimate = $state(''),
    error = $state(''),
    estimateError = $state(''),
    dueError = $state(''),
    choices = $state<DeadlineChoice[]>([]),
    custom = $state('');
  let lastDue: string | null | undefined = undefined,
    lastEstimate: number | null | undefined = undefined;
  $effect(() => {
    if (value.due_at !== lastDue) {
      lastDue = value.due_at;
      const fields = deadlineFields(value.due_at ?? null, timeZone);
      date = fields.date;
      time = fields.time;
      offset = fields.offset;
    }
  });
  $effect(() => {
    if (value.estimate_h !== lastEstimate) {
      lastEstimate = value.estimate_h;
      estimate = value.estimate_h?.toString() ?? '';
    }
  });
  function set(patch: TaskPatch, commit = false) {
    onInput(patch);
    if (commit) onCommit(Object.keys(patch) as (keyof TaskPatch)[]);
  }
  $effect(() => onInvalid(!!estimateError || !!dueError || !!custom.trim()));
  function due() {
    try {
      choices = date && time ? deadlineChoices(date, time, timeZone) : [];
      const due_at = deadlineToUtc(date, time, timeZone, offset || undefined);
      dueError = '';
      set({ due_at }, true);
    } catch (e) {
      dueError = String((e as Error).message);
    }
  }
  function estimateCommit(commit = true) {
    try {
      const estimate_h = estimateValue(estimate);
      estimateError = '';
      set({ estimate_h }, commit);
    } catch (e) {
      estimateError = (e as Error).message;
    }
  }
  function textKey(e: KeyboardEvent, key: keyof TaskPatch) {
    if (e.key === 'Enter') {
      e.preventDefault();
      (e.currentTarget as HTMLInputElement).blur();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onCancel(key);
    }
  }
  function toggleAlert(n: number) {
    onAlerts(
      alerts.includes(n)
        ? alerts.filter((v) => v !== n)
        : [...alerts, n].sort((a, b) => b - a),
    );
  }
  function addAlert() {
    const n = Number(custom);
    if (!custom.trim() || !Number.isInteger(n) || n < 0) {
      error = 'Use whole nonnegative minutes.';
      return;
    }
    onAlerts([...new Set([...alerts, n])].sort((a, b) => b - a));
    custom = '';
    error = '';
  }
  export function resetInvalid() {
    const fields = deadlineFields(value.due_at ?? null, timeZone);
    date = fields.date;
    time = fields.time;
    offset = fields.offset;
    estimate = value.estimate_h?.toString() ?? '';
    error = '';
    estimateError = '';
    dueError = '';
    custom = '';
  }
</script>

<fieldset {disabled} class:rail>
  <div class="scalar-grid">
    <label class="field"
      ><span>Status</span><select
        value={value.status}
        onchange={(e) =>
          set({ status: e.currentTarget.value as TaskStatus }, true)}
        >{#each statuses as status}<option value={status}
            >{statusLabels[status]}</option
          >{/each}</select
      ></label
    ><label class="field"
      ><span>Priority</span><select
        value={value.priority}
        onchange={(e) =>
          set({ priority: e.currentTarget.value as Priority }, true)}
        >{#each ['low', 'medium', 'high'] as priority}<option value={priority}
            >{priority[0].toUpperCase() + priority.slice(1)}</option
          >{/each}</select
      ></label
    ><label class="field"
      ><span>Estimate · hours</span><input
        type="text"
        inputmode="decimal"
        placeholder="—"
        bind:value={estimate}
        oninput={() => estimateCommit(false)}
        onblur={() => estimateCommit()}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            e.currentTarget.blur();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            onCancel('estimate_h');
            estimate = value.estimate_h?.toString() ?? '';
            estimateError = '';
          }
        }}
      /></label
    >
  </div>
  <div class="due-grid">
    <label class="field"
      ><span>Due date</span><input
        type="date"
        bind:value={date}
        onchange={() => {
          if (date && !time) time = '17:00';
          if (!date) time = '';
          due();
        }}
      /></label
    ><label class="field"
      ><span>Time · {timeZone}</span><TimeInput
        label="Deadline time"
        value={time}
        oninput={(v) => { time = v; }}
        disabled={!date || disabled}
        onchange={due}
      /></label
    >
  </div>
  {#if choices.length > 1}<label class="field"
      ><span>This time occurs twice · choose offset</span><select
        bind:value={offset}
        onchange={due}
        ><option value="">Choose UTC offset</option
        >{#each choices as choice}<option value={choice.offset}
            >UTC{choice.offset}</option
          >{/each}</select
      ></label
    >{/if}
  <div class="alerts">
    <span class="eyebrow">Alerts · before due</span>
    <div class="actions">
      {#each [...new Set( [1440, 60, ...alerts], )].sort((a, b) => b - a) as n}<button
          type="button"
          class="chip"
          class:chosen={alerts.includes(n)}
          aria-pressed={alerts.includes(n)}
          onclick={() => toggleAlert(n)}
          >{n === 1440 ? '1 day' : n === 60 ? '1 hour' : `${n} min`}</button
        >{/each}
    </div>
    <div class="actions">
      <input
        class="alert-minutes"
        aria-label="Custom alert minutes before due"
        placeholder="Minutes"
        inputmode="numeric"
        bind:value={custom}
      /><button type="button" class="outline" onclick={addAlert}>Add</button>
    </div>
    <small>Saved offsets; notifications arrive in M6.</small>
  </div>
  {#if value.status === 'blocked'}<label class="field"
      ><span>Blocked reason</span><input
        value={value.blocked_reason ?? ''}
        oninput={(e) => set({ blocked_reason: e.currentTarget.value || null })}
        onblur={() => onCommit(['blocked_reason'])}
        onkeydown={(e) => textKey(e, 'blocked_reason')}
      /></label
    ><label class="field"
      ><span>Waiting on</span><input
        value={value.blocked_on ?? ''}
        oninput={(e) => set({ blocked_on: e.currentTarget.value || null })}
        onblur={() => onCommit(['blocked_on'])}
        onkeydown={(e) => textKey(e, 'blocked_on')}
      /></label
    >{/if}{#if error || estimateError || dueError}<p class="error" role="alert">
      {error || estimateError || dueError}
    </p>{/if}
</fieldset>

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .scalar-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-3);
  }
  .due-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  .rail .scalar-grid,
  .rail .due-grid {
    grid-template-columns: 1fr;
    gap: 0;
  }
  .alerts {
    margin-bottom: var(--space-4);
  }
  .alerts .actions {
    margin: var(--space-2) 0;
  }
  .chosen {
    background: var(--selection-fill);
    color: var(--accent);
    border-color: var(--accent);
  }
  .alert-minutes {
    width: var(--space-8);
    min-width: 0;
    flex: 1;
  }
  small {
    font-size: var(--text-chip);
    color: var(--text-faint);
  }
</style>
