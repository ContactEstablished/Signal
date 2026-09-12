<svelte:options runes={true} />

<script lang="ts">
  import { Plus, X, ExternalLink } from 'lucide-svelte';
  import type { Attachment, StagedAttachment } from '../../domain/types';
  let {
    files,
    staged,
    pending = false,
    onAdd,
    onOpen,
    onRemove,
    onDiscard,
    onRetry,
  }: {
    files: Attachment[];
    staged: StagedAttachment[];
    pending?: boolean;
    onAdd: () => void;
    onOpen: (id: string) => void;
    onRemove: (id: string) => void;
    onDiscard: (token: string) => void;
    onRetry?: () => void;
  } = $props();
  const size = (n: number) =>
    n >= 1024 * 1024
      ? `${(n / 1024 / 1024).toFixed(1)} MB`
      : n >= 1024
        ? `${(n / 1024).toFixed(1)} KB`
        : `${n} B`;
</script>

<section>
  <div class="eyebrow">Attachments</div>
  <div class="files">
    {#each files as file}<div class="attachment">
        <button
          type="button"
          disabled={pending}
          onclick={() => onOpen(file.id)}
          title={`${file.mime} · ${size(file.size)}`}
          ><span class="kind"
            >{file.filename.split('.').pop()?.toUpperCase()}</span
          >{file.filename}<ExternalLink /></button
        ><button
          type="button"
          aria-label={`Remove ${file.filename}`}
          disabled={pending}
          onclick={() => onRemove(file.id)}><X /></button
        >
      </div>{/each}{#each staged as file}<div class="attachment staged">
        <span>{file.filename} · {size(file.size)} · Ready</span><button
          type="button"
          aria-label={`Discard staged ${file.filename}`}
          disabled={pending}
          onclick={() => onDiscard(file.token)}><X /></button
        >
      </div>{/each}<button
      type="button"
      class="outline add"
      disabled={pending}
      onclick={onAdd}><Plus />{pending ? 'Adding…' : 'Add'}</button
    >{#if staged.length && onRetry}<button
        type="button"
        class="outline"
        disabled={pending}
        onclick={onRetry}>Retry adding files</button
      >{/if}
  </div>
  <small>Local copies · drop files anywhere in this dialog</small>
</section>

<style>
  section {
    margin-bottom: var(--space-4);
  }
  .files {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .attachment {
    display: flex;
    align-items: center;
    border: var(--line) solid var(--border-input);
    background: var(--surface-2);
    border-radius: var(--radius-input);
    min-width: 0;
    max-width: 100%;
    font-size: var(--text-meta);
  }
  .attachment button {
    display: flex;
    align-items: center;
    gap: var(--space-tight);
    padding: var(--space-tight);
    overflow-wrap: anywhere;
    text-align: left;
  }
  .kind {
    color: var(--accent);
    font: var(--weight-semibold) var(--text-chip) var(--font-heading);
  }
  .add {
    border-style: dashed;
    display: flex;
    gap: var(--space-tight);
    align-items: center;
  }
  .staged {
    color: var(--warn);
    padding-left: var(--space-2);
  }
  small {
    color: var(--text-faint);
    font-size: var(--text-chip);
  }
</style>
