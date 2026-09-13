<svelte:options runes={true} />

<script lang="ts">
  import type { TimeEntry } from '../../domain/timers';
  import { formatElapsedMs } from '../../domain/timer-math';
  let { entries, timeZone }: { entries: TimeEntry[]; timeZone: string } =
    $props();
  const date = (value: string) =>
    new Intl.DateTimeFormat('en-US', {
      timeZone,
      dateStyle: 'medium',
      timeStyle: 'medium',
    }).format(new Date(value));
</script>

<details class="history">
  <summary>Logged entries · {entries.length}</summary>
  {#each entries as entry (entry.id)}<div class="entry">
      <strong
        >{formatElapsedMs(Math.round(entry.minutes * 60000))} active</strong
      ><span>{date(entry.started_at)} → {date(entry.ended_at)}</span>
    </div>{:else}<p>No completed time entries.</p>{/each}
</details>

<style>
  .history {
    margin-top: var(--space-4);
    font-size: var(--text-meta);
  }
  summary {
    cursor: pointer;
    color: var(--text-muted);
  }
  .entry {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) 0;
    border-bottom: var(--line) solid var(--border);
  }
  strong {
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-semibold);
  }
  span {
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
</style>
