<svelte:options runes={true} />

<script module lang="ts">
  import type { TimerSession, TimeEntry } from '../../domain/timers';
  export interface TimerUiBindings {
    session: TimerSession | null;
    entries: TimeEntry[];
    nowUtc: string;
    timeZone: string;
    loading: boolean;
    error: string;
    pending: boolean;
    recoveryRequired: boolean;
    logCompletionVersion: number;
    onRetry: () => Promise<void>;
    onStart: () => Promise<void>;
    onPause: () => Promise<void>;
    onResume: () => Promise<void>;
    onStop: () => Promise<void>;
    onLog: (input: {
      startedAt: string;
      durationMs: number;
    }) => Promise<void>;
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { Play, Pause, Square, Plus } from 'lucide-svelte';
  import { progress } from '../../domain/board';
  import { formatElapsedMs, timerElapsedMs } from '../../domain/timer-math';
  import { errorMessage } from '../../domain/types';
  import LogTimeForm from './LogTimeForm.svelte';
  import TimeEntryList from './TimeEntryList.svelte';
  let {
    hours,
    estimate,
    time,
    taskName = '',
    onPreview = () => {},
    onLogDirty = () => {},
    disabled = false,
  }: {
    hours: number;
    estimate: number | null;
    time?: TimerUiBindings;
    taskName?: string;
    onPreview?: (s: string) => void;
    onLogDirty?: (v: boolean) => void;
    disabled?: boolean;
  } = $props();
  let logOpen = $state(false),
    busy = $state(false),
    error = $state('');
  let form = $state<LogTimeForm>(),
    logButton = $state<HTMLButtonElement>();
  async function perform(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await action();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  function closeLog() {
    logOpen = false;
    onLogDirty(false);
    void tick().then(() => logButton?.isConnected && logButton.focus());
  }
  export async function saveLog() {
    return !logOpen || (!!form && (await form.submit()));
  }
</script>

<section class="time-card">
  <div class="eyebrow">Time · logged</div>
  <div class="hours">
    {hours.toLocaleString('en-US', { maximumFractionDigits: 2 })}<span
      >{estimate === null ? 'h' : ` / ${estimate}h`}</span
    >
  </div>
  <div class="progress">
    <span style:width={`${progress(hours, estimate)}%`}></span>
  </div>
  {#if estimate !== null && hours > estimate}<p class="warning">
      {(hours - estimate).toFixed(2)}h over estimate
    </p>{/if}
  {#if time}
    {#if time.session}<div
        class:running={time.session.state === 'running'}
        class="elapsed"
      >
        <span
          >{time.session.state === 'running' ? 'Running' : 'Paused'}</span
        ><strong
          >{formatElapsedMs(
            timerElapsedMs(time.session, time.nowUtc),
          )}</strong
        >
      </div>{/if}
    {#if time.loading}<p role="status">Loading time…</p>{/if}
    <div class="actions">
      {#if !time.session}<button
          class="start"
          disabled={disabled ||
            busy ||
            time.pending ||
            time.loading ||
            time.recoveryRequired ||
            !!time.error}
          onclick={() => perform(time!.onStart)}><Play />Start</button
        >
      {:else if time.session.state === 'running'}<button
          class="outline"
          disabled={disabled ||
            busy ||
            time.pending ||
            time.recoveryRequired}
          onclick={() => perform(time!.onPause)}><Pause />Pause</button
        >
      {:else}<button
          class="start"
          disabled={disabled ||
            busy ||
            time.pending ||
            time.recoveryRequired}
          onclick={() => perform(time!.onResume)}><Play />Resume</button
        >{/if}
      {#if time.session}<button
          class="outline"
          disabled={disabled ||
            busy ||
            time.pending ||
            time.recoveryRequired}
          onclick={() => perform(time!.onStop)}><Square />Stop</button
        >{/if}
      <button
        bind:this={logButton}
        class="outline"
        disabled={disabled ||
          busy ||
          time.pending ||
          time.recoveryRequired ||
          time.loading ||
          logOpen}
        onclick={() => (logOpen = true)}><Plus />Log time</button
      >
    </div>
    {#if time.error || error}<p class="error" role="alert">
        {time.error || error}
      </p>
      <button
        class="outline"
        disabled={busy || time.pending}
        onclick={() => perform(time!.onRetry)}>Retry time operation</button
      >{/if}
    {#if logOpen}<LogTimeForm
        bind:this={form}
        {time}
        {taskName}
        onDirty={onLogDirty}
        onClose={closeLog}
      />{/if}
    {#if !time.loading && (!time.error || time.entries.length > 0)}<TimeEntryList
        entries={time.entries}
        timeZone={time.timeZone}
      />{/if}
  {:else}<p>
      Timer and manual logging controls are previews. They don’t record time
      yet.
    </p>
    <div class="actions">
      <button
        class="start"
        onclick={() =>
          onPreview('Start timer · M2 preview. No timer was started.')}
        ><Play />Start preview</button
      ><button
        class="outline"
        onclick={() =>
          onPreview('Log time · M2 preview. No time entry was added.')}
        ><Plus />Log preview</button
      >
    </div>{/if}
</section>

<style>
  .time-card {
    padding: var(--space-3);
    background: var(--surface);
    border: var(--line) solid var(--border);
    border-radius: var(--radius-card);
    margin-bottom: var(--space-5);
  }
  .hours {
    font: var(--weight-semibold) var(--text-title) var(--font-heading);
    margin: var(--space-2) 0;
  }
  .hours span {
    font-size: var(--text-label);
    color: var(--text-muted);
  }
  .actions {
    margin-top: var(--space-3);
    flex-wrap: wrap;
  }
  .actions button {
    display: flex;
    gap: var(--space-1);
    align-items: center;
  }
  .start {
    padding: var(--space-2) var(--space-3);
    background: var(--lime);
    color: var(--on-lime);
    font-weight: var(--weight-semibold);
  }
  p {
    font-size: var(--text-meta);
    margin-top: var(--space-2);
  }
  .elapsed {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    margin-top: var(--space-3);
    border-radius: var(--radius-chip);
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  .elapsed.running {
    background: var(--timer-fill);
    color: var(--lime);
  }
  strong {
    font: var(--weight-semibold) var(--text-title) var(--font-heading);
    font-variant-numeric: tabular-nums;
  }
</style>
