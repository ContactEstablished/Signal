<svelte:options runes={true} />

<script lang="ts">
  import { Plus, X, ArrowUp, ArrowDown } from 'lucide-svelte';
  import type { SubtaskInput } from '../../domain/types';
  let {
    value,
    onInput,
    onCommit,
    onDraft = () => {},
    disabled = false,
  }: {
    value: SubtaskInput[];
    onInput: (v: SubtaskInput[]) => void;
    onCommit: () => void;
    onDraft?: (pending: boolean) => void;
    disabled?: boolean;
  } = $props();
  let title = $state('');
  $effect(() => onDraft(!!title.trim()));
  export function flushPending() {
    add();
  }
  let done = $derived(value.filter((v) => v.done).length);
  function change(index: number, patch: Partial<SubtaskInput>, commit = false) {
    onInput(value.map((v, i) => (i === index ? { ...v, ...patch } : v)));
    if (commit) onCommit();
  }
  function add() {
    if (!title.trim()) return;
    onInput([...value, { title: title.trim(), done: false }]);
    title = '';
    onCommit();
  }
  function move(index: number, offset: number) {
    const next = [...value];
    [next[index], next[index + offset]] = [next[index + offset], next[index]];
    onInput(next);
    onCommit();
  }
</script>

<section>
  <div class="heading">
    <span class="eyebrow">Subtasks</span><span>{done}/{value.length}</span>
  </div>
  <div class="progress">
    <span style:width={`${value.length ? (done / value.length) * 100 : 0}%`}
    ></span>
  </div>
  {#each value as item, i}<div class="row">
      <input
        type="checkbox"
        aria-label={`Complete ${item.title}`}
        checked={item.done}
        {disabled}
        onchange={(e) => change(i, { done: e.currentTarget.checked }, true)}
      /><input
        class:done={item.done}
        aria-label={`Subtask ${i + 1} title`}
        value={item.title}
        {disabled}
        oninput={(e) => change(i, { title: e.currentTarget.value })}
        onblur={onCommit}
      /><button
        type="button"
        aria-label={`Move subtask ${i + 1} up`}
        disabled={disabled || i === 0}
        onclick={() => move(i, -1)}><ArrowUp /></button
      ><button
        type="button"
        aria-label={`Move subtask ${i + 1} down`}
        disabled={disabled || i === value.length - 1}
        onclick={() => move(i, 1)}><ArrowDown /></button
      ><button
        type="button"
        aria-label={`Remove ${item.title}`}
        {disabled}
        onclick={() => {
          onInput(value.filter((_, n) => n !== i));
          onCommit();
        }}><X /></button
      >
    </div>{/each}
  <div class="row">
    <input
      aria-label="New subtask"
      placeholder="Add a subtask…"
      bind:value={title}
      {disabled}
      onkeydown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          add();
        }
      }}
    /><button
      type="button"
      class="outline"
      aria-label="Add subtask"
      {disabled}
      onclick={add}><Plus /></button
    >
  </div>
</section>

<style>
  section {
    margin-bottom: var(--space-5);
  }
  .heading {
    display: flex;
    justify-content: space-between;
    margin-bottom: var(--space-2);
    font-size: var(--text-meta);
    color: var(--lime);
  }
  .progress > span {
    background: var(--lime);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-2);
  }
  .row input:not([type='checkbox']) {
    flex: 1;
    min-width: 0;
    padding: var(--space-tight);
    font-size: var(--text-label);
  }
  .row input[type='checkbox'] {
    accent-color: var(--lime);
    width: var(--subtask-checkbox);
    height: var(--subtask-checkbox);
  }
  .row button {
    padding: var(--line);
    display: flex;
  }
  .done {
    text-decoration: line-through;
    color: var(--text-faint);
  }
</style>
