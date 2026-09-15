<script lang="ts">
  import type { AgendaTask } from '../../domain/agenda';
  import { onMount } from 'svelte';
  let {
    selected,
    labels = [],
    disabled = false,
    onChange,
    onSearch,
  }: {
    selected: string[];
    labels?: AgendaTask[];
    disabled?: boolean;
    onChange: (ids: string[]) => void;
    onSearch: (input: {
      text: string;
      limit: number;
      cursor?: string;
    }) => Promise<{ items: AgendaTask[]; nextCursor: string | null }>;
  } = $props();
  let query = $state(''),
    items = $state<AgendaTask[]>([]),
    cursor = $state<string | null>(null),
    error = $state(''),
    busy = $state(false);
  let serial = 0;
  let known = $state<Record<string, AgendaTask>>({});
  async function search(more = false) {
    const s = ++serial;
    busy = true;
    try {
      const r = await onSearch({
        text: query,
        limit: 20,
        ...(more && cursor ? { cursor } : {}),
      });
      if (s !== serial) return;
      items = more ? [...items, ...r.items] : r.items;
      cursor = r.nextCursor;
      known = {
        ...known,
        ...Object.fromEntries(r.items.map((t) => [t.id, t])),
      };
      error = '';
    } catch (e) {
      if (s === serial) error = String(e);
    } finally {
      if (s === serial) busy = false;
    }
  }
  onMount(() => {
    void search();
  });
</script>

<div class="linked-picker">
  <label
    >Linked tasks<input
      type="search"
      placeholder="Search task, ID or project"
      bind:value={query}
      {disabled}
      oninput={() => void search()}
    /></label
  >{#if selected.length}<div class="selected">
      {#each selected as id}<button
          type="button"
          class="outline"
          {disabled}
          onclick={() => onChange(selected.filter((t) => t !== id))}
          >{labels.find((t) => t.id === id)?.title ??
            known[id]?.title ??
            'Linked task'} ×</button
        >{/each}
    </div>{/if}
  <div class="options">
    {#each items as t}<label class="choice"
        ><input
          type="checkbox"
          checked={selected.includes(t.id)}
          {disabled}
          onchange={() =>
            onChange(
              selected.includes(t.id)
                ? selected.filter((id) => id !== t.id)
                : [...selected, t.id],
            )}
        /><span
          >{t.title}<small
            >{t.project_name} · {t.status.replaceAll('_', ' ')}</small
          ></span
        ></label
      >{/each}
  </div>
  {#if cursor}<button
      type="button"
      class="outline"
      disabled={busy || disabled}
      onclick={() => search(true)}>More tasks</button
    >{/if}{#if busy}<span class="muted">Searching…</span>{/if}{#if error}<p
      class="error"
      role="alert"
    >
      {error}<button type="button" onclick={() => search()}>Retry</button>
    </p>{/if}
</div>

<style>
  .options {
    max-height: var(--agenda-picker-height);
    overflow: auto;
    margin-top: var(--space-2);
  }
  .choice {
    flex-direction: row;
    margin-bottom: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    font-weight: var(--weight-normal);
  }
  .choice input {
    width: auto;
  }
  .choice span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    color: var(--text-muted);
  }
  .selected {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
    margin-top: var(--space-2);
  }
  .selected button {
    font-size: var(--text-meta);
  }
  .linked-picker > label {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
</style>
