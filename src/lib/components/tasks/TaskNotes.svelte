<svelte:options runes={true} />

<script lang="ts">
  import { renderMarkdown } from '../../domain/markdown';
  let {
    value,
    persisted = '',
    creation = false,
    editing = $bindable(false),
    disabled = false,
    onInput,
    onSave,
    onCancel,
    onOpenLink,
  }: {
    value: string;
    persisted?: string;
    creation?: boolean;
    editing?: boolean;
    disabled?: boolean;
    onInput: (v: string) => void;
    onSave: () => void;
    onCancel: () => void;
    onOpenLink: (url: string) => void;
  } = $props();
  let preview = $state(false);
  function intercept(node: HTMLElement) {
    const listener = (e: Event) => link(e as MouseEvent);
    node.addEventListener('click', listener);
    return { destroy: () => node.removeEventListener('click', listener) };
  }
  function link(e: MouseEvent) {
    const anchor = (e.target as HTMLElement).closest('a');
    if (anchor) {
      e.preventDefault();
      onOpenLink(anchor.href);
    }
  }
</script>

<section>
  <div class="heading">
    <span class="eyebrow">Notes</span>{#if !creation && !editing}<button
        type="button"
        class="outline"
        {disabled}
        onclick={() => (editing = true)}>Edit</button
      >{/if}
  </div>
  {#if creation || editing}<textarea
      aria-label="Task notes in Markdown"
      rows="6"
      placeholder="Notes in Markdown…"
      {value}
      {disabled}
      oninput={(e) => onInput(e.currentTarget.value)}
      onkeydown={(e) => {
        if (!creation && e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          onCancel();
          editing = false;
        }
      }}
    ></textarea>
    <div class="actions">
      <button type="button" class="outline" onclick={() => (preview = !preview)}
        >{preview ? 'Hide preview' : 'Preview Markdown'}</button
      >{#if !creation}<span class="spacer"></span><button
          type="button"
          class="outline"
          {disabled}
          onclick={() => {
            onCancel();
            editing = false;
          }}>Cancel</button
        ><button type="button" class="primary" {disabled} onclick={onSave}
          >Save notes</button
        >{/if}
    </div>{/if}{#if (!creation && !editing) || preview}<div
      class="markdown"
      use:intercept
      role="region"
      aria-label="Rendered task notes"
    >
      {@html renderMarkdown(creation || editing ? value : persisted)}
    </div>
    {#if !persisted && !editing && !creation}<p>No notes yet.</p>{/if}{/if}
</section>

<style>
  section {
    margin-bottom: var(--space-5);
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }
  textarea {
    width: 100%;
  }
  .actions {
    margin-top: var(--space-2);
  }
  .markdown {
    color: var(--text-2);
    font-size: var(--text-base);
    line-height: var(--leading-notes);
    overflow-wrap: anywhere;
  }
  .markdown :global(p) {
    color: var(--text-2);
  }
  .markdown :global(a) {
    color: var(--accent);
  }
  .markdown :global(pre) {
    white-space: pre-wrap;
    background: var(--surface-2);
    padding: var(--space-3);
    border-radius: var(--radius-input);
  }
</style>
