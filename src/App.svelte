<svelte:options runes={true} />
<script lang="ts">
  import { onMount } from 'svelte';
  import { Search, Plus, Settings, X, ArrowRight, Database, Bell, Circle } from 'lucide-svelte';
  import { openFoundation, setTrayPreference, type Foundation } from './lib/db/client';
  let data = $state<Foundation | null>(null);
  let loading = $state(true);
  let error = $state('');
  let active = $state('day');
  let settingsPage = $state('Notifications');
  let saving = $state(false);
  let preferenceError = $state('');
  let modal: HTMLDialogElement;
  let modalTitle = $state('');
  let modalText = $state('');
  let palette = $state(false);
  let query = $state('');
  const settingPages = ['General', 'Notifications', 'Managers & summaries', 'Integrations', 'Appearance', 'Data & backup'];
  const selectedProject = $derived(data?.projects.find(project => project.id === active));
  const heading = $derived(selectedProject?.name ?? (active === 'today' ? 'Today' : 'Your Day'));
  async function load() {
    loading = true; error = '';
    try { data = await openFoundation(); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }
  onMount(() => {
    void load();
    if (import.meta.env.DEV) void document.fonts.ready.then(() => {
      console.info('Signal shell diagnostics', JSON.stringify({
        viewport: [innerWidth, innerHeight],
        headerHeight: document.querySelector('header')?.getBoundingClientRect().height,
        fonts: [...document.fonts].map(face => ({ family: face.family, weight: face.weight, status: face.status })),
        fontRequests: performance.getEntriesByType('resource').filter(r => r.name.includes('/fonts/')).map(r => r.name),
      }));
    });
  });
  function stub(title: string, text: string, isPalette = false) {
    modalTitle = title; modalText = text; palette = isPalette; query = ''; modal.showModal();
  }
  function keyboard(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      if (!modal.open) stub('Search or jump to…', 'Command palette preview. Search results and actions arrive in M7.', true);
    }
  }
  async function toggleTray() {
    if (!data || saving) return;
    saving = true; preferenceError = '';
    try { await setTrayPreference(!data.tray); data.tray = !data.tray; }
    catch (e) { preferenceError = `Could not save the tray preference. ${String(e)}`; }
    finally { saving = false; }
  }
</script>

<svelte:window onkeydown={keyboard} />
<header class="app-header">
  <span class="wordmark">Signal</span>
  <nav class="tabs" aria-label="Workspace">
    <button class="tab" class:active={active === 'today'} aria-current={active === 'today' ? 'page' : undefined} onclick={() => active = 'today'}>Today <span class="badge" aria-label={`${data?.badge ?? 0} overdue or due today`}>{data?.badge ?? 0}</span></button>
    <button class="tab" class:active={active === 'day'} aria-current={active === 'day' ? 'page' : undefined} onclick={() => active = 'day'}>Your Day</button>
    <span class="divider"></span>
    {#each data?.projects ?? [] as project (project.id)}
      <button class="tab project-tab" class:active={active === project.id} data-color={project.color} aria-current={active === project.id ? 'page' : undefined} onclick={() => active = project.id}><span class="dot"></span>{project.name}</button>
    {/each}
    <button class="add-project" aria-label="Add project" title="Add project · M1 preview" onclick={() => stub('Add project', 'Project creation arrives in M1. This is a preview of the entry point.')}><Plus /></button>
  </nav>
  <div class="header-actions">
    <button class="search" aria-label="Open command palette" onclick={() => stub('Search or jump to…', 'Command palette preview. Search results and actions arrive in M7.', true)}><Search /><span>Search or jump to…</span><kbd>Ctrl K</kbd></button>
    <button class="settings-button" class:selected={active === 'settings'} aria-current={active === 'settings' ? 'page' : undefined} onclick={() => active = 'settings'}><Settings />Settings</button>
  </div>
</header>

{#if loading}
  <main class="page"><section class="placeholder" role="status"><Database /><h1>Opening your workspace…</h1><p>Loading local data.</p></section></main>
{:else if error}
  <main class="page"><section class="placeholder" role="alert"><Database /><h1>Signal couldn’t open local data.</h1><p>Your workspace could not be loaded. Retry after resolving the error below.</p><pre>{error}</pre><button class="outline" onclick={load}>Retry</button></section></main>
{:else if active === 'settings'}
  <main class="settings-layout">
    <aside class="settings-nav"><div class="eyebrow">Settings</div><nav aria-label="Settings sections">{#each settingPages as page}<button class:chosen={settingsPage === page} aria-current={settingsPage === page ? 'page' : undefined} onclick={() => settingsPage = page}>{page}</button>{/each}</nav></aside>
    <section class="settings-content">
      <h1>{settingsPage}</h1>
      {#if settingsPage === 'Notifications'}
        <div class="setting-row"><div><h3 id="tray-label">Keep running in the system tray</h3><p id="tray-help">Closing the window hides Signal instead of quitting.</p></div><button class="toggle" class:on={data?.tray} role="switch" aria-checked={data?.tray ?? false} aria-labelledby="tray-label" aria-describedby="tray-help" disabled={saving} onclick={toggleTray}><span></span></button></div>
        {#if preferenceError}<p role="alert" class="error">{preferenceError}</p>{/if}
        <p class="tray-tip">Use the Signal tray icon → Open to return, or Quit to exit.</p>
        <div class="stub-note"><Bell /><div><h2>Notification settings preview</h2><p>Reminders, toast notifications, sounds, and timer actions arrive in M6.</p></div></div>
      {:else}
        <div class="stub-note"><Settings /><div><h2>{settingsPage} preview</h2><p>This section is a placeholder for a later milestone.</p></div></div>
      {/if}
    </section>
  </main>
{:else}
  <main class="page">
    <section class="destination-heading"><div><h1>{heading}</h1><p>{data?.runtime.seeded ? 'Thursday, September 11, 2025 · 13:42 · Fixture preview' : 'Your local workspace'}</p></div><span class="milestone">M0 · Foundation</span></section>
    <section class="placeholder">
      <div class="placeholder-symbol" data-color={selectedProject?.color ?? 'cyan'}><Circle /></div>
      <h2>{selectedProject ? 'Project views are coming next.' : active === 'today' ? 'Your daily overview starts here.' : 'A place to plan your day.'}</h2>
      <p>{selectedProject ? 'Board, Week, and Notes are placeholders. Project and task workflows begin in M1.' : active === 'today' ? 'The Today digest arrives in M4.' : 'The day planner arrives in M3. Timer workflows arrive in M2.'}</p>
      <button class="outline" onclick={() => active = 'settings'}>Open Settings <ArrowRight /></button>
    </section>
    {#if data?.runtime.seeded}<footer class="fixture-status"><Database /><span>Isolated fixture · {data.projects.length} projects · {data.counts.tasks} tasks · {data.counts.meetings} meetings · {data.counts.blocks} blocks · {data.counts.time_entries} time entries</span></footer>{/if}
  </main>
{/if}

<dialog bind:this={modal} aria-labelledby="stub-title" onclose={() => query = ''}>
  <div class="dialog-heading"><h2 id="stub-title">{modalTitle}</h2><button aria-label="Close preview" onclick={() => modal.close()}><X /></button></div>
  {#if palette}<label class="palette-input"><Search /><input aria-label="Search preview" bind:value={query} placeholder="Search tasks, projects, or commands…" /></label>{/if}
  <p>{modalText}</p>
  <div class="dialog-footer"><span class="eyebrow">Preview only</span><button class="outline" onclick={() => modal.close()}>Close</button></div>
</dialog>

<style>
  .app-header { height: var(--header-height); display: flex; align-items: center; gap: var(--header-gap); padding: 0 var(--header-padding); background: var(--bg-raised); border-bottom: var(--line) solid var(--border-section); white-space: nowrap; }
  .wordmark { font: var(--weight-bold) var(--text-wordmark) var(--font-heading); letter-spacing: var(--tracking-title); }
  .tabs { display: flex; height: 100%; align-items: center; gap: var(--tab-gap); min-width: 0; }
  .tab { height: 100%; display: flex; align-items: center; gap: var(--space-2); padding: 0 var(--tab-padding); position: relative; border: 0; border-radius: 0; font-weight: var(--weight-medium); color: var(--text-muted); }
  .tab.active { color: var(--text); }
  .tab.active::after { content: ''; position: absolute; bottom: 0; left: 0; right: 0; height: var(--underline); background: var(--text); }
  .project-tab.active::after { background: var(--project-color); }
  .badge { font: var(--weight-semibold) var(--text-chip) var(--font-heading); padding: var(--line) var(--space-tight); background: var(--warn); color: var(--on-warn); border-radius: var(--radius-input); }
  .divider { width: var(--line); height: var(--space-5); background: var(--border-input); margin: 0 var(--space-2); flex: none; }
  .add-project { display: grid; place-items: center; padding: var(--space-2); color: var(--text-faint); }
  .header-actions { margin-left: auto; display: flex; gap: var(--space-4); align-items: center; }
  .search { width: var(--search-width); display: flex; align-items: center; gap: var(--space-2); padding: var(--space-tight) var(--space-3); background: var(--surface-2); border-color: var(--border-section); border-radius: var(--radius-input); color: var(--text-muted); font-size: var(--text-label); }
  .search:hover { background: var(--surface-3); }
  .search kbd { margin-left: auto; font: var(--text-chip) var(--font-heading); }
  .settings-button { display: flex; gap: var(--space-tight); align-items: center; color: var(--text-muted); padding: var(--space-tight) 0; }
  .settings-button.selected { color: var(--text); }
  .page { padding: var(--padding-page); min-height: calc(100vh - var(--header-height)); display: flex; flex-direction: column; }
  .destination-heading { display: flex; justify-content: space-between; align-items: start; }
  .milestone { border: var(--line) solid var(--border-section); padding: var(--space-1) var(--space-2); border-radius: var(--radius-chip); color: var(--text-muted); font-size: var(--text-meta); }
  .placeholder { margin: auto; padding: var(--padding-modal); text-align: center; max-width: var(--modal-task-new); }
  .placeholder p { line-height: var(--leading-notes); }
  .placeholder-symbol { display: inline-flex; padding: var(--space-3); background: var(--project-task-fill); color: var(--project-color); border-radius: var(--radius-panel); margin-bottom: var(--space-5); }
  .placeholder .outline { display: inline-flex; align-items: center; gap: var(--space-2); }
  .fixture-status { display: flex; justify-content: center; align-items: center; gap: var(--space-2); color: var(--text-faint); font-size: var(--text-meta); padding-top: var(--space-6); }
  .settings-layout { display: grid; grid-template-columns: var(--settings-nav) 1fr; min-height: calc(100vh - var(--header-height)); }
  .settings-nav { background: var(--bg-raised); border-right: var(--line) solid var(--border-section); padding: var(--space-6) var(--space-3); }
  .settings-nav .eyebrow { margin: 0 var(--space-3) var(--space-4); }
  .settings-nav nav { display: flex; flex-direction: column; gap: var(--space-1); }
  .settings-nav button { text-align: left; padding: var(--space-small) var(--space-3); color: var(--text-muted); font-size: var(--text-label); }
  .settings-nav .chosen { background: var(--surface-3); color: var(--text); }
  .settings-content { padding: var(--space-8); max-width: var(--modal-task-detail); }
  .settings-content h1 { font-size: var(--text-page-small); margin-bottom: var(--space-6); }
  .setting-row { display: flex; align-items: center; gap: var(--space-8); padding-bottom: var(--space-5); border-bottom: var(--line) solid var(--border-section); }
  .setting-row h3 { font: var(--weight-medium) var(--text-base) var(--font-body); margin: 0 0 var(--space-1); }
  .setting-row p { font-size: var(--text-label); margin: 0; }
  .toggle { flex: none; margin-left: auto; width: var(--toggle-width); height: var(--toggle-height); border: 0; border-radius: var(--radius-toggle); background: var(--surface-3); padding: var(--toggle-inset); }
  .toggle span { display: block; width: var(--toggle-knob); height: var(--toggle-knob); border-radius: var(--radius-round); background: var(--text-faint); transition: transform var(--transition); }
  .toggle.on { background: var(--accent); }
  .toggle.on span { background: var(--on-accent); transform: translateX(calc(var(--toggle-width) - var(--toggle-knob) - 2 * var(--toggle-inset))); }
  .tray-tip { font-size: var(--text-label); margin-top: var(--space-4); }
  .stub-note { display: flex; align-items: start; gap: var(--space-3); padding: var(--space-5); margin-top: var(--space-8); border: var(--line) dashed var(--border-dashed); border-radius: var(--radius-panel); color: var(--text-muted); }
  .stub-note h2 { font-size: var(--text-base); }
  .stub-note p { margin: 0; font-size: var(--text-label); }
  .error { color: var(--danger); margin-top: var(--space-3); }
  pre { white-space: pre-wrap; overflow-wrap: anywhere; color: var(--text-muted); font-size: var(--text-label); }
  dialog { background: var(--surface); color: var(--text); width: var(--modal-task-new); padding: var(--padding-modal); border: var(--line) solid var(--border-popover); border-radius: var(--radius-modal); box-shadow: var(--shadow-modal); }
  dialog[open] { animation: enter var(--transition); }
  dialog::backdrop { background: var(--scrim); }
  .dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: var(--space-6); margin-bottom: var(--space-5); }
  .dialog-heading h2 { margin: 0; }
  .dialog-heading button { display: flex; padding: var(--space-1); }
  .palette-input { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-4); }
  .palette-input input { flex: 1; min-width: 0; }
  .dialog-footer { display: flex; align-items: center; justify-content: space-between; margin-top: var(--space-6); }
  @keyframes enter { from { opacity: 0; transform: scale(var(--enter-scale)); } to { opacity: 1; transform: scale(1); } }
  @media (prefers-reduced-motion: reduce) { dialog[open] { animation: none; } }
</style>
