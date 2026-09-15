<script lang="ts">
  import { localDateTimeLabel } from '../../domain/time-display';
  import type { MeetingPreview } from '../../domain/agenda';
  let {
    preview,
    pending,
    onApply,
    onCancel,
  }: {
    preview: MeetingPreview;
    pending: boolean;
    onApply: () => unknown;
    onCancel: () => unknown;
  } = $props();
</script>

<section class="preview" aria-label="Review meeting change">
  <h3>Review meeting change</h3>
  <p>
    {Object.entries(preview.affected_counts)
      .map(([k, n]) => `${n} ${k.replaceAll('_', ' ')}`)
      .join(' · ')}
  </p>
  {#if preview.preserved_overrides.length}<strong
      >These later individual edits will remain:</strong
    >
    <ul>
      {#each preview.preserved_overrides as m}<li>
          {m.title} · {localDateTimeLabel(m.start_local)} · {m.time_zone}
        </li>{/each}
    </ul>{/if}{#each [...preview.warnings, ...preview.skipped_dates] as warning}<p
    >
      {warning}
    </p>{/each}
  <p>Linked tasks, task blocks and logged time are preserved.</p>
  <div class="agenda-actions">
    <button class="primary" type="button" disabled={pending} onclick={onApply}
      >Apply change</button
    ><button class="outline" type="button" disabled={pending} onclick={onCancel}
      >Keep editing</button
    >
  </div>
</section>

<style>
  .preview {
    border: var(--line) solid var(--warn-border);
    border-radius: var(--radius-card);
    padding: var(--space-4);
    background: var(--warn-fill);
    margin-top: var(--space-4);
  }
  h3 {
    font-size: var(--text-card);
  }
  ul {
    padding-left: var(--space-5);
    max-height: var(--agenda-picker-height);
    overflow: auto;
  }
</style>
