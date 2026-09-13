<svelte:options runes={true} />

<script lang="ts">
  import { Play, ChevronDown, ArrowRight } from 'lucide-svelte';
  import { tick } from 'svelte';
  import type { TimerSession } from '../../domain/timers';
  import { timerElapsedMs, formatElapsedMs } from '../../domain/timer-math';
  let {
    sessions,
    nowUtc,
    onOpen,
  }: {
    sessions: TimerSession[];
    nowUtc: string;
    onOpen: (taskId: string) => void;
  } = $props();
  const running = $derived(sessions.filter((s) => s.state === 'running'));
  const id = $props.id();
  let open = $state(false);
  let root = $state<HTMLDivElement>(null!);
  let trigger = $state<HTMLButtonElement>(null!);
  let panel = $state<HTMLDivElement>(null!);
  $effect(() => {
    if (!running.length) open = false;
  });
  function dismiss(restoreFocus = false) {
    open = false;
    if (restoreFocus) trigger?.focus();
  }
  async function triggerKey(event: KeyboardEvent) {
    if (event.key !== 'ArrowDown') return;
    event.preventDefault();
    open = true;
    await tick();
    panel?.querySelector('button')?.focus();
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (open && !root?.contains(e.target as Node)) dismiss();
  }}
  onfocusin={(e) => {
    if (open && !root?.contains(e.target as Node)) dismiss();
  }}
  onkeydown={(e) => {
    if (open && e.key === 'Escape') {
      e.preventDefault();
      dismiss(true);
    }
  }}
/>
{#if running.length}
  <div class="timer-picker" bind:this={root}>
    <button
      class="timer-status"
      bind:this={trigger}
      aria-expanded={open}
      aria-controls={id}
      onclick={() => (open = !open)}
      onkeydown={triggerKey}
    >
      <Play />{running.length}
      {running.length === 1 ? 'timer' : 'timers'} running<ChevronDown />
    </button>
    {#if open}
      <div
        class="timer-list"
        {id}
        role="region"
        aria-label="Running timers"
        bind:this={panel}
      >
        <p class="list-heading">Running timers</p>
        <p class="hint">Open a task to pause or stop its timer.</p>
        <ul>
          {#each running as session (session.id)}
            <li>
              <button
                class="timer-task"
                onclick={() => {
                  onOpen(session.task_id);
                  dismiss();
                }}
              >
                <Play class="running-icon" />
                <span class="task-label"
                  ><strong>{session.task_title ?? 'Task'}</strong><span
                    class="project"
                    >{session.project_name ?? 'Project'}</span
                  ></span
                >
                <span class="elapsed"
                  >{formatElapsedMs(timerElapsedMs(session, nowUtc))}</span
                >
                <ArrowRight />
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
{/if}

<style>
  .timer-picker {
    position: relative;
  }
  .timer-status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-tight) var(--space-3);
    border-radius: var(--radius-button);
    background: var(--timer-fill);
    color: var(--lime);
    font: var(--weight-semibold) var(--text-label) var(--font-heading);
    white-space: nowrap;
  }
  .timer-status:hover,
  .timer-status[aria-expanded='true'] {
    border-color: var(--lime);
  }
  .timer-list {
    position: absolute;
    z-index: 40;
    top: calc(100% + var(--space-2));
    right: 0;
    width: 400px;
    max-width: calc(100vw - var(--space-8));
    max-height: min(60vh, 480px);
    overflow-y: auto;
    background: var(--bg-raised);
    border: var(--line) solid var(--border-popover);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-popover);
    padding: var(--space-2);
  }
  .list-heading {
    margin: var(--space-2);
    font: var(--weight-semibold) var(--text-label) var(--font-heading);
  }
  .hint {
    margin: var(--space-2);
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .timer-task {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-3) var(--space-2);
    text-align: left;
    border-radius: var(--radius-button);
  }
  .timer-task:hover {
    background: var(--surface-2);
  }
  .timer-task :global(.running-icon),
  .elapsed {
    color: var(--lime);
  }
  .task-label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1;
    min-width: 0;
  }
  strong {
    font-size: var(--text-label);
    font-weight: var(--weight-medium);
    overflow-wrap: anywhere;
  }
  .project {
    color: var(--text-muted);
    font-size: var(--text-meta);
    overflow-wrap: anywhere;
  }
  .elapsed {
    font-size: var(--text-meta);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
