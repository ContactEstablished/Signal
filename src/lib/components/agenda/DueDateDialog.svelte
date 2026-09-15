<script lang="ts">
  import TimeInput from '../TimeInput.svelte';
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';
  import { timeChoices } from '../../domain/agenda-calendar';
  import type { DueDraft } from '../../domain/agenda';
  let {
    draft,
    taskTitle,
    timeZone,
    pending = false,
    recoveryRequired = false,
    error = '',
    reason,
    onChange,
    onSave,
    onCancel,
    onRetry,
  }: {
    draft: DueDraft;
    taskTitle: string;
    timeZone: string;
    pending?: boolean;
    recoveryRequired?: boolean;
    error?: string;
    reason: string;
    onChange: (d: DueDraft) => void;
    onSave: (d: DueDraft) => Promise<void>;
    onCancel: () => unknown;
    onRetry: () => unknown;
  } = $props();
  let dialog: HTMLDialogElement;
  let localError = $state(''),
    busy = $state(false),
    discard = $state(false);
  let baseline = '';
  let closeResolve: ((v: boolean) => void) | null = null;
  const choices = $derived.by(() => {
    try {
      return timeChoices(draft.date, draft.time, timeZone);
    } catch {
      return [];
    }
  });
  onMount(() => {
    baseline = JSON.stringify(draft);
    dialog.showModal();
    return () => closeResolve?.(false);
  });
  async function save(clear = false) {
    if (busy || pending || recoveryRequired) return;
    busy = true;
    try {
      await onSave(clear ? { date: '', time: '' } : draft);
    } catch (e) {
      localError = String(e);
    } finally {
      busy = false;
    }
  }
  export async function requestClose(_reason?: unknown) {
    if (pending || busy || recoveryRequired) return false;
    if (JSON.stringify(draft) === baseline) return true;
    discard = true;
    return new Promise<boolean>((resolve) => (closeResolve = resolve));
  }
  function decide(v: boolean) {
    discard = false;
    closeResolve?.(v);
    closeResolve = null;
  }
</script>

<dialog
  bind:this={dialog}
  class="agenda-dialog"
  oncancel={(e) => {
    e.preventDefault();
    if (!pending && !recoveryRequired && !busy) onCancel();
  }}
>
  <button
    class="dialog-x"
    aria-label="Close deadline editor"
    disabled={pending || recoveryRequired || busy}
    onclick={onCancel}><X /></button
  >
  <h2>Move deadline</h2>
  {#if discard}<section>
      <h3>Discard unsaved deadline changes?</h3>
      <div class="agenda-actions">
        <button class="outline" onclick={() => decide(false)}
          >Keep editing</button
        ><button class="outline danger" onclick={() => decide(true)}
          >Discard changes</button
        >
      </div>
    </section>{/if}
  <p>{taskTitle}</p>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <fieldset disabled={pending || busy || recoveryRequired || discard}>
      <label
        >Date<input
          type="date"
          value={draft.date}
          oninput={(e) =>
            onChange({
              ...draft,
              date: e.currentTarget.value,
              offset: undefined,
            })}
        /></label
      ><label
        >Time<TimeInput
          label="Deadline time"
          value={draft.time}
          oninput={(e) =>
            onChange({
              ...draft,
              time: e,
              offset: undefined,
            })}
        /></label
      >
      <p>{timeZone}</p>
      {#if reason === 'no-deadline'}<p>
          New deadline defaults to 5:00 PM. Adjust it before saving.
        </p>{/if}{#if choices.length === 2}<label
          >UTC offset<select
            value={draft.offset ?? ''}
            onchange={(e) =>
              onChange({ ...draft, offset: e.currentTarget.value })}
            ><option value="">Choose the occurrence of this time</option
            >{#each choices as c}<option value={c.offset}>{c.offset}</option
              >{/each}</select
          ></label
        >{/if}
      <div class="agenda-actions">
        <button class="primary" type="submit">Save deadline</button><button
          class="outline"
          type="button"
          onclick={() => save(true)}>Clear deadline</button
        ><button class="outline" type="button" onclick={onCancel}>Cancel</button
        >
      </div>
    </fieldset>
  </form>
  {#if error || localError}<p class="error" role="alert">
      {error || localError}
    </p>{/if}{#if recoveryRequired}<button class="primary" onclick={onRetry}
      >Retry exact save</button
    >{/if}
</dialog>

<style>
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
</style>
