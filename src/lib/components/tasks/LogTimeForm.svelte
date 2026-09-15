<svelte:options runes={true} />

<script lang="ts">
  import TimeInput from '../TimeInput.svelte';
  import { onMount, onDestroy, untrack } from 'svelte';
  import { defaultTimeLog, normalizeTimeLog } from '../../domain/time-log';
  import { deadlineChoices } from '../../domain/deadlines';
  import type { TimerUiBindings } from './TaskTimeCard.svelte';
  import { errorMessage } from '../../domain/types';
  let {
    time,
    taskName,
    onDirty,
    onClose,
  }: {
    time: TimerUiBindings;
    taskName: string;
    onDirty: (v: boolean) => void;
    onClose: () => void;
  } = $props();
  const initial = untrack(() => defaultTimeLog(time.nowUtc, time.timeZone));
  let draft = $state({ ...initial }),
    error = $state(''),
    saving = $state(false),
    submitted = $state(false),
    submissionVersion = 0,
    alive = true;
  let start = $state<HTMLInputElement>();
  const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(initial));
  const choices = $derived.by(() => {
    try {
      return deadlineChoices(draft.date, draft.startTime, time.timeZone);
    } catch {
      return [];
    }
  });
  const preview = $derived.by(() => {
    try {
      return normalizeTimeLog(draft, time.timeZone, time.nowUtc).endedAt;
    } catch {
      return null;
    }
  });
  $effect(() => onDirty(dirty || submitted));
  $effect(() => {
    if (submitted && time.logCompletionVersion > submissionVersion) {
      submitted = false;
      onDirty(false);
      onClose();
    }
  });
  onMount(() => start?.focus());
  onDestroy(() => {
    alive = false;
    onDirty(false);
  });
  export async function submit(): Promise<boolean> {
    if (saving) return false;
    try {
      const input = normalizeTimeLog(draft, time.timeZone, time.nowUtc);
      saving = true;
      error = '';
      submissionVersion = time.logCompletionVersion;
      submitted = true;
      await time.onLog({
        startedAt: input.startedAt,
        durationMs: input.durationMs,
      });
      if (alive) {
        submitted = false;
        onDirty(false);
        onClose();
      }
      return true;
    } catch (e) {
      error = errorMessage(e);
      if (!time.recoveryRequired) submitted = false;
      return false;
    } finally {
      saving = false;
    }
  }
</script>

<form
  class="log-form"
  onsubmit={(e) => {
    e.preventDefault();
    void submit();
  }}
>
  <h3>Log time</h3>
  <p>{taskName} · {time.timeZone}</p>
  <fieldset disabled={saving || time.pending || time.recoveryRequired}>
    <label class="field"
      ><span>Date</span><input
        aria-label="Log date"
        type="date"
        bind:value={draft.date}
        onchange={() => (draft.offset = '')}
      /></label
    >
    <label class="field"
      ><span>Start time</span><TimeInput
        bind:element={start}
        label="Log start time"
        value={draft.startTime}
        oninput={(v) => { draft.startTime = v; draft.offset = ''; }}
        onchange={() => (draft.offset = '')}
      /></label
    >
    {#if choices.length > 1}<label class="field"
        ><span>This time occurs twice. Choose UTC offset</span><select
          aria-label="Log UTC offset"
          bind:value={draft.offset}
          ><option value="">Choose offset</option
          >{#each choices as choice}<option value={choice.offset}
              >{choice.offset}</option
            >{/each}</select
        ></label
      >{/if}
    <div class="duration">
      {#each ['hours', 'minutes', 'seconds'] as key}<label class="field"
          ><span>{key}</span><input
            aria-label={`Log ${key}`}
            type="number"
            min="0"
            max={key === 'hours' ? undefined : 59}
            step="1"
            value={draft[key as 'hours' | 'minutes' | 'seconds']}
            oninput={(e) =>
              (draft = {
                ...draft,
                [key]:
                  e.currentTarget.value === ''
                    ? NaN
                    : Number(e.currentTarget.value),
              })}
          /></label
        >{/each}
    </div>
  </fieldset>
  {#if preview}<p>
      Ends {new Intl.DateTimeFormat('en-US', {
        timeZone: time.timeZone,
        dateStyle: 'medium',
        hour12: true,
        timeStyle: 'medium',
      }).format(new Date(preview))}
    </p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if time.recoveryRequired}<p class="warning">
      The saved result is unknown. Use Retry above before changing or
      discarding this log.
    </p>{/if}
  <div class="actions">
    <button
      type="submit"
      class="primary"
      disabled={saving || time.pending || time.recoveryRequired}
      >Save time</button
    ><button
      type="button"
      class="outline"
      disabled={saving || time.pending || time.recoveryRequired}
      onclick={() => {
        onDirty(false);
        onClose();
      }}>Cancel</button
    >
  </div>
</form>

<style>
  .log-form {
    margin-top: var(--space-4);
    padding-top: var(--space-3);
    border-top: var(--line) solid var(--border);
  }
  h3 {
    font: var(--weight-semibold) var(--text-base) var(--font-heading);
  }
  p {
    font-size: var(--text-meta);
    overflow-wrap: anywhere;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .duration {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-1);
  }
  input {
    min-width: 0;
    width: 100%;
  }
  .actions {
    flex-wrap: wrap;
  }
</style>
