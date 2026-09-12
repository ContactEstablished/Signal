<svelte:options runes={true} />

<script lang="ts">
  import { X } from 'lucide-svelte';
  import { projectColors, type Tag, type TagInput } from '../../domain/types';
  let {
    value,
    tags,
    onChange,
    onDraft = () => {},
    disabled = false,
  }: {
    value: TagInput[];
    tags: Tag[];
    onChange: (v: TagInput[]) => void;
    onDraft?: (pending: boolean) => void;
    disabled?: boolean;
  } = $props();
  let name = $state(''),
    color = $state('cyan');
  $effect(() => onDraft(!!name.trim()));
  export function flushPending() {
    add();
  }
  function title(tag: TagInput) {
    return 'id' in tag
      ? (tags.find((t) => t.id === tag.id)?.name ?? 'Tag')
      : tag.name;
  }
  function hue(tag: TagInput) {
    return 'id' in tag
      ? (tags.find((t) => t.id === tag.id)?.color ?? 'cyan')
      : tag.color;
  }
  function add() {
    if (!name.trim()) return;
    onChange([...value, { name: name.trim(), color }]);
    name = '';
  }
</script>

<section>
  <div class="eyebrow">Tags</div>
  <div class="actions selected-tags">
    {#each value as tag, i}<button
        type="button"
        class="chip"
        data-color={hue(tag)}
        {disabled}
        aria-label={`Remove tag ${title(tag)}`}
        onclick={() => onChange(value.filter((_, n) => n !== i))}
        >{title(tag)}<X /></button
      >{/each}
  </div>
  <div class="actions">
    <select
      aria-label="Add existing tag"
      {disabled}
      value=""
      onchange={(e) => {
        const id = e.currentTarget.value;
        if (id && !value.some((t) => 'id' in t && t.id === id))
          onChange([...value, { id }]);
        e.currentTarget.value = '';
      }}
      ><option value="">Choose a tag…</option>{#each tags as tag}<option
          value={tag.id}>{tag.name}</option
        >{/each}</select
    >
  </div>
  <div class="new-tag">
    <input
      aria-label="New tag name"
      placeholder="New tag…"
      bind:value={name}
      {disabled}
    /><select aria-label="New tag color" bind:value={color} {disabled}
      >{#each [...projectColors, 'orange'] as c}<option value={c}>{c}</option
        >{/each}</select
    ><button type="button" class="outline" {disabled} onclick={add}>Add</button>
  </div>
</section>

<style>
  section {
    margin-bottom: var(--space-5);
  }
  .selected-tags {
    margin: var(--space-2) 0;
  }
  .new-tag {
    display: flex;
    gap: var(--space-1);
    margin-top: var(--space-2);
    flex-wrap: wrap;
  }
  .new-tag input {
    width: 100%;
  }
  .new-tag select {
    flex: 1;
    min-width: 0;
  }
  .chip :global(svg) {
    width: var(--space-3);
    height: var(--space-3);
  }
</style>
