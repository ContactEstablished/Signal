<svelte:options runes={true} />

<script lang="ts">
  import { Ellipsis, X } from 'lucide-svelte';
  import {
    errorMessage,
    type ProjectRecord,
    type DeletionPreview,
    type DeletionTarget,
    type CleanupResult,
  } from '../../domain/types';
  let {
    project,
    index,
    total,
    onEdit,
    onMove,
    onPreviewDeletion,
    onDelete,
  }: {
    project: ProjectRecord;
    index: number;
    total: number;
    onEdit: () => void;
    onMove: (direction: 'left' | 'right') => Promise<unknown>;
    onPreviewDeletion: (target: DeletionTarget) => Promise<DeletionPreview>;
    onDelete: (
      target: DeletionTarget,
      fingerprint: string,
    ) => Promise<CleanupResult>;
  } = $props();
  let open = $state(false),
    pending = $state(false),
    error = $state(''),
    preview = $state<DeletionPreview | null>(null);
  let dialog = $state<HTMLDialogElement>(null!);
  async function move(direction: 'left' | 'right') {
    pending = true;
    error = '';
    try {
      await onMove(direction);
      open = false;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      pending = false;
    }
  }
  async function ask() {
    pending = true;
    error = '';
    try {
      preview = await onPreviewDeletion({ kind: 'project', id: project.id });
      open = false;
      dialog.showModal();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      pending = false;
    }
  }
  async function remove() {
    if (!preview) return;
    pending = true;
    error = '';
    try {
      await onDelete({ kind: 'project', id: project.id }, preview.fingerprint);
      dialog.close();
    } catch (e) {
      error = errorMessage(e);
      preview = null;
    } finally {
      pending = false;
    }
  }
</script>

<div class="menu">
  <button
    class="outline"
    aria-label="Project menu"
    aria-expanded={open}
    onclick={() => (open = !open)}><Ellipsis /></button
  >{#if open}<div class="items">
      <button
        disabled={pending}
        onclick={() => {
          open = false;
          onEdit();
        }}>Edit project</button
      ><button disabled={pending || index === 0} onclick={() => move('left')}
        >Move left</button
      ><button
        disabled={pending || index === total - 1}
        onclick={() => move('right')}>Move right</button
      ><button class="danger" disabled={pending} onclick={ask}
        >Delete project…</button
      >
    </div>{/if}{#if error && !dialog?.open}<p class="error" role="alert">
      {error}
    </p>{/if}
</div>
<dialog
  class="m1-dialog"
  bind:this={dialog}
  aria-labelledby="delete-project-title"
  oncancel={(e) => {
    if (pending) e.preventDefault();
  }}
>
  <div class="dialog-head">
    <h2 id="delete-project-title">Delete {project.name}?</h2>
    <button
      aria-label="Cancel deletion"
      disabled={pending}
      onclick={() => dialog.close()}><X /></button
    >
  </div>
  <p>
    This permanently deletes the project and its related records. There is no
    undo. Original attachment source files are kept.
  </p>
  {#if preview}<ul>
      {#each Object.entries(preview.counts).filter(([, n]) => n > 0) as [kind, count]}<li
        >
          {count}
          {kind.replaceAll('_', ' ')}
        </li>{/each}
    </ul>{/if}{#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="dialog-footer">
    <button class="outline" disabled={pending} onclick={() => dialog.close()}
      >Cancel</button
    ><span class="spacer"></span>{#if preview}<button
        class="outline danger"
        disabled={pending}
        onclick={remove}>{pending ? 'Deleting…' : 'Delete permanently'}</button
      >{:else}<button class="outline" disabled={pending} onclick={ask}
        >Refresh affected records</button
      >{/if}
  </div>
</dialog>

<style>
  .menu {
    position: relative;
  }
  .items {
    position: absolute;
    right: 0;
    top: 100%;
    z-index: 4;
    width: var(--week-later-width);
    background: var(--surface);
    border: var(--line) solid var(--border-popover);
    border-radius: var(--radius-input);
    padding: var(--space-1);
    box-shadow: var(--shadow-popover);
  }
  .items button {
    display: block;
    width: 100%;
    text-align: left;
    padding: var(--space-2);
  }
  dialog {
    width: var(--modal-task-new);
  }
</style>
