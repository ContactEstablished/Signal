<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';
  import {
    emptyFilters,
    type BoardFilterState,
    type DueFilter,
  } from '../../domain/board';
  import type { Tag, Priority } from '../../domain/types';
  let {
    filters = $bindable(),
    tags,
    onClose,
  }: { filters: BoardFilterState; tags: Tag[]; onClose: () => void } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => dialog.showModal());
  function priority(value: Priority) {
    filters = {
      ...filters,
      priorities: filters.priorities.includes(value)
        ? filters.priorities.filter((p) => p !== value)
        : [...filters.priorities, value],
    };
  }
  function tag(id: string) {
    filters = {
      ...filters,
      tagIds: filters.tagIds.includes(id)
        ? filters.tagIds.filter((t) => t !== id)
        : [...filters.tagIds, id],
    };
  }
</script>

<dialog
  class="m1-dialog"
  bind:this={dialog}
  aria-labelledby="filter-title"
  oncancel={(e) => {
    e.preventDefault();
    onClose();
  }}
>
  <div class="dialog-head">
    <h2 id="filter-title">Filter tasks</h2>
    <button aria-label="Close filters" onclick={onClose}><X /></button>
  </div>
  <label class="field"
    ><span>Title or external ID</span><input
      value={filters.text}
      oninput={(e) => (filters = { ...filters, text: e.currentTarget.value })}
    /></label
  >
  <div class="eyebrow">Priority</div>
  <div class="actions">
    {#each ['low', 'medium', 'high'] as p}<button
        class="outline"
        class:chosen={filters.priorities.includes(p as Priority)}
        aria-pressed={filters.priorities.includes(p as Priority)}
        onclick={() => priority(p as Priority)}>{p}</button
      >{/each}
  </div>
  <label class="field"
    ><span>Due</span><select
      value={filters.due}
      onchange={(e) =>
        (filters = { ...filters, due: e.currentTarget.value as DueFilter })}
      ><option value="all">Any date</option><option value="overdue"
        >Overdue</option
      ><option value="today">Today</option><option value="next7days"
        >Tomorrow through the next 7 days</option
      ><option value="noDate">No date</option></select
    ></label
  >
  <div class="eyebrow">Tags · match any selected</div>
  <div class="actions">
    {#each tags as t}<button
        class="chip"
        data-color={t.color}
        aria-pressed={filters.tagIds.includes(t.id)}
        class:chosen={filters.tagIds.includes(t.id)}
        onclick={() => tag(t.id)}>{t.name}</button
      >{/each}
  </div>
  <div class="dialog-footer">
    <button class="outline" onclick={() => (filters = emptyFilters())}
      >Clear filters</button
    ><span class="spacer"></span><button class="primary" onclick={onClose}
      >Done</button
    >
  </div>
</dialog>

<style>
  dialog {
    width: var(--picker-width);
  }
  .chosen {
    outline: var(--line) solid var(--accent);
  }
  .actions {
    margin: var(--space-2) 0 var(--space-4);
  }
</style>
