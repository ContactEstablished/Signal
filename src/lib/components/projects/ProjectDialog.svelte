<svelte:options runes={true} />

<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { X } from 'lucide-svelte';
  import {
    projectColors,
    errorMessage,
    type ProjectRecord,
    type ProjectColor,
    type CloseReason,
  } from '../../domain/types';
  let {
    project = null,
    onSave,
    onClose,
  }: {
    project?: ProjectRecord | null;
    onSave: (input: { name: string; color: ProjectColor }) => Promise<unknown>;
    onClose: () => void;
  } = $props();
  let name = $state(''),
    color = $state<ProjectColor>('cyan'),
    error = $state(''),
    pending = $state(false),
    confirm = $state(false);
  let initial = { name: '', color: 'cyan' as ProjectColor };
  let dialog: HTMLDialogElement;
  let resolveClose: ((v: boolean) => void) | null = null;
  let closePromise: Promise<boolean> | null = null;
  let saving: Promise<void> | null = null;
  onMount(() => {
    initial = { name: project?.name ?? '', color: project?.color ?? 'cyan' };
    name = initial.name;
    color = initial.color;
    dialog.showModal();
  });
  function save() {
    if (saving) return saving;
    saving = (async () => {
      pending = true;
      error = '';
      try {
        if (!name.trim()) throw new Error('Enter a project name.');
        await onSave({ name: name.trim(), color });
        initial = { name, color };
        onClose();
      } catch (e) {
        error = errorMessage(e);
      } finally {
        pending = false;
        saving = null;
      }
    })();
    return saving;
  }
  export async function requestClose(_reason: CloseReason): Promise<boolean> {
    if (saving) await saving;
    if (name === initial.name && color === initial.color) return true;
    if (closePromise) return closePromise;
    confirm = true;
    void tick().then(() =>
      dialog
        .querySelector<HTMLButtonElement>('.confirm-panel button:last-child')
        ?.focus(),
    );
    closePromise = new Promise<boolean>((r) => (resolveClose = r));
    return closePromise;
  }
  function decide(value: boolean) {
    confirm = false;
    resolveClose?.(value);
    resolveClose = null;
    closePromise = null;
  }
</script>

<dialog
  class="m1-dialog"
  bind:this={dialog}
  aria-labelledby="project-title"
  oncancel={(e) => {
    e.preventDefault();
    if (confirm) decide(false);
    else onClose();
  }}
>
  <div class="dialog-head">
    <h2 id="project-title">{project ? 'Edit project' : 'New project'}</h2>
    <button aria-label="Close project editor" onclick={onClose}><X /></button>
  </div>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <fieldset disabled={pending || confirm}>
      <label class="field"
        ><span>Project name</span><input bind:value={name} required /></label
      >
      <div class="eyebrow">Color</div>
      <div class="actions colors">
        {#each projectColors as value}<button
            type="button"
            data-color={value}
            aria-label={value}
            aria-pressed={color === value}
            class:chosen={color === value}
            onclick={() => (color = value)}
            ><span class="dot"></span>{value}</button
          >{/each}
      </div>
    </fieldset>
    {#if error}<p role="alert" class="error">
        {error}
      </p>{/if}{#if confirm}<section class="confirm-panel">
        <h3>Discard project changes?</h3>
        <p>Your unsaved changes will be lost.</p>
        <div class="actions">
          <button
            type="button"
            class="danger outline"
            onclick={() => decide(true)}>Discard changes</button
          ><button type="button" class="outline" onclick={() => decide(false)}
            >Keep editing</button
          >
        </div>
      </section>{/if}
    <div class="dialog-footer">
      <button type="button" class="outline" disabled={pending} onclick={onClose}
        >Cancel</button
      ><span class="spacer"></span><button
        class="primary"
        disabled={pending || confirm}
        >{pending
          ? 'Saving…'
          : project
            ? 'Save changes'
            : 'Create project'}</button
      >
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: var(--modal-meeting-new);
  }
  fieldset {
    padding: 0;
    border: 0;
    margin: 0;
  }
  .colors {
    margin-top: var(--space-2);
  }
  .colors button {
    padding: var(--space-2);
    display: flex;
    gap: var(--space-tight);
    align-items: center;
  }
  .chosen {
    border-color: var(--project-color);
  }
</style>
