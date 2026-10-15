<script lang="ts">
  import { Check, CircleX } from 'lucide-svelte';
  import type { Attendance } from '../../domain/agenda';
  let { title, attendance = 'unmarked', disabled = false, alwaysVisible = false, onMark }: {
    title: string;
    attendance?: Attendance;
    disabled?: boolean;
    alwaysVisible?: boolean;
    onMark: (value: Attendance) => unknown;
  } = $props();
</script>

<div class="meeting-quick-actions" class:visible={alwaysVisible}>
  <button
    type="button"
    class:attended={attendance === 'attended'}
    aria-label={`${attendance === 'attended' ? 'Clear attended mark for' : 'Mark attended:'} ${title}`}
    aria-pressed={attendance === 'attended'}
    title={attendance === 'attended' ? 'Undo attended' : 'Mark attended'}
    {disabled}
    onclick={() => onMark(attendance === 'attended' ? 'unmarked' : 'attended')}
  ><Check aria-hidden="true" />{#if alwaysVisible}Attended{/if}</button>
  <button
    type="button"
    class:canceled={attendance === 'canceled'}
    aria-label={`${attendance === 'canceled' ? 'Restore canceled meeting:' : 'Mark canceled:'} ${title}`}
    aria-pressed={attendance === 'canceled'}
    title={attendance === 'canceled' ? 'Undo canceled' : 'Mark canceled'}
    {disabled}
    onclick={() => onMark(attendance === 'canceled' ? 'unmarked' : 'canceled')}
  ><CircleX aria-hidden="true" />{#if alwaysVisible}Canceled{/if}</button>
</div>

<style>
  .meeting-quick-actions {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
    opacity: 0;
    pointer-events: none;
  }
  :global(.meeting-status-host:hover) .meeting-quick-actions,
  :global(.meeting-status-host:focus-within) .meeting-quick-actions,
  .meeting-quick-actions:focus-within,
  .visible {
    opacity: 1;
    pointer-events: auto;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 22px;
    height: 22px;
    padding: 2px 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
    color: var(--text-muted);
    font-size: var(--text-chip);
  }
  button:hover { color: var(--text); border-color: var(--text-muted); }
  button.attended { color: var(--lime); border-color: var(--lime); }
  button.canceled { color: var(--text); border-color: var(--text-muted); }
  button :global(svg) { width: 14px; height: 14px; }
  .visible button { width: auto; height: auto; padding: 5px 8px; }
  @media (hover: none) { .meeting-quick-actions { opacity: 1; pointer-events: auto; } }
</style>
