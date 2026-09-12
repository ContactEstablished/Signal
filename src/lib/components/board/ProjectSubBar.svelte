<svelte:options runes={true} />

<script lang="ts">
  import { Filter, Plus } from 'lucide-svelte';
  import ProjectMenu from '../projects/ProjectMenu.svelte';
  import type {
    ProjectRecord,
    DeletionTarget,
    DeletionPreview,
    CleanupResult,
  } from '../../domain/types';
  let {
    project,
    projects,
    view,
    onView,
    onFilter,
    onNewTask,
    onEdit,
    onMove,
    onPreviewDeletion,
    onDelete,
  }: {
    project: ProjectRecord;
    projects: ProjectRecord[];
    view: 'board' | 'week' | 'notes';
    onView: (v: 'board' | 'week' | 'notes') => void;
    onFilter: () => void;
    onNewTask: () => void;
    onEdit: () => void;
    onMove: (d: 'left' | 'right') => Promise<unknown>;
    onPreviewDeletion: (t: DeletionTarget) => Promise<DeletionPreview>;
    onDelete: (t: DeletionTarget, f: string) => Promise<CleanupResult>;
  } = $props();
</script>

<div class="subbar">
  <nav aria-label="Project views">
    {#each ['board', 'week', 'notes'] as v}<button
        class:active={view === v}
        aria-current={view === v ? 'page' : undefined}
        onclick={() => onView(v as typeof view)}
        >{v[0].toUpperCase() + v.slice(1)}</button
      >{/each}
  </nav>
  {#if view === 'board'}<button
      class="outline"
      data-filter-trigger
      onclick={onFilter}><Filter />Filter</button
    ><span class="outline fixed-group">Group · Status</span>{/if}<span
    class="spacer"
  ></span><ProjectMenu
    {project}
    index={projects.findIndex((p) => p.id === project.id)}
    total={projects.length}
    {onEdit}
    {onMove}
    {onPreviewDeletion}
    {onDelete}
  /><button class="primary" onclick={onNewTask}><Plus />New task</button>
</div>

<style>
  .subbar {
    display: flex;
    align-items: center;
    gap: var(--space-small);
    margin-bottom: var(--space-4);
  }
  nav {
    display: flex;
    padding: var(--space-1);
    border: var(--line) solid var(--border-section);
    background: var(--bg-raised);
    border-radius: var(--radius-card);
  }
  nav button {
    padding: var(--space-tight) var(--space-4);
    color: var(--text-muted);
  }
  nav button.active {
    background: var(--surface-3);
    color: var(--text);
  }
  .outline {
    display: flex;
    align-items: center;
    gap: var(--space-tight);
  }
  .fixed-group {
    color: var(--text-muted);
    border: var(--line) solid var(--border-input);
    border-radius: var(--radius-button);
  }
</style>
