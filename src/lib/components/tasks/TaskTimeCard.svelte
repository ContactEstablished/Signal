<svelte:options runes={true} />

<script lang="ts">
  import { Play, Plus } from 'lucide-svelte';
  import { progress } from '../../domain/board';
  let {
    hours,
    estimate,
    onPreview,
  }: {
    hours: number;
    estimate: number | null;
    onPreview: (s: string) => void;
  } = $props();
</script>

<section>
  <div class="eyebrow">Time</div>
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
  <p class="preview-help">
    Timer and manual logging controls are previews. They don’t record time yet.
  </p>
  <div class="actions">
    <button
      type="button"
      class="start"
      onclick={() =>
        onPreview('Start timer · M2 preview. No timer was started.')}
      ><Play />Start preview</button
    ><button
      type="button"
      class="outline"
      onclick={() =>
        onPreview('Log time · M2 preview. No time entry was added.')}
      ><Plus />Log preview</button
    >
  </div>
</section>

<style>
  section {
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
</style>
