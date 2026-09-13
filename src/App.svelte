<svelte:options runes={true} />

<script lang="ts">
  import TimerStatus from './lib/components/shell/TimerStatus.svelte';
  import { onMount, tick } from 'svelte';
  import YourDay from './lib/views/YourDay.svelte';
  import type { TaskDetail } from './lib/domain/types';
  import {
    Search,
    Plus,
    Settings,
    X,
    ArrowRight,
    Database,
    Bell,
    Circle,
  } from 'lucide-svelte';
  import { listen } from '@tauri-apps/api/event';
  import { setTrayPreference } from './lib/db/client';
  import { Workspace } from './lib/state/app.svelte';
  import * as commands from './lib/native/commands';
  import { subscribeAttachments } from './lib/native/attachments';
  import {
    errorMessage,
    type CloseReason,
    type ProjectRecord,
    type ProjectColor,
  } from './lib/domain/types';
  import Board from './lib/views/Board.svelte';
  import ProjectSubBar from './lib/components/board/ProjectSubBar.svelte';
  import ProjectDialog from './lib/components/projects/ProjectDialog.svelte';
  import NewTaskDialog from './lib/components/tasks/NewTaskDialog.svelte';
  import TaskDetailDialog from './lib/components/tasks/TaskDetailDialog.svelte';
  const workspace = new Workspace();
  const data = $derived(workspace.foundation),
    active = $derived(workspace.active),
    loading = $derived(workspace.loading),
    error = $derived(workspace.error);
  const selectedProject = $derived(
    workspace.projects.find((p) => p.id === active),
  );
  const heading = $derived(active === 'today' ? 'Today' : 'Your Day');
  let day = $state<YourDay>(null!);
  let createdTask: TaskDetail | null = null;
  let board = $state<Board>(null!);
  $effect(() => {
    const guarded = !!workspace.editor || workspace.planner.guarded || Object.values(workspace.timers.pending).some(Boolean) || Object.values(workspace.timers.recovery).some(Boolean);
    void commands.setEditGuard(guarded).catch(e => workspace.notice = errorMessage(e));
  });
  let projectDialog = $state<ProjectDialog>(null!);
  let newDialog = $state<NewTaskDialog>(null!);
  let detailDialog = $state<TaskDetailDialog>(null!);
  let settingsPage = $state('Notifications'),
    saving = $state(false),
    preferenceError = $state('');
  let modal: HTMLDialogElement;
  let modalTitle = $state(''),
    modalText = $state(''),
    palette = $state(false),
    query = $state('');
  const settingPages = [
    'General',
    'Notifications',
    'Managers & summaries',
    'Integrations',
    'Appearance',
    'Data & backup',
  ];
  let pendingExitId: string | undefined;
  let closing: Promise<boolean> | null = null,
    trigger: HTMLElement | null = null,
    afterProjectSave: string | null = null;
  function currentEditor() {
    return workspace.editor?.kind === 'project'
      ? projectDialog
      : workspace.editor?.kind === 'new'
        ? newDialog
        : workspace.editor?.kind === 'detail'
          ? detailDialog
          : null;
  }
  function closeEditor(
    reason: CloseReason = 'dialog',
    requestId?: string,
  ): Promise<boolean> {
    if (requestId) pendingExitId = requestId;
    if (closing) return closing;
    closing = (async () => {
      try {
        const editor = currentEditor();
        let proceed = editor ? await editor.requestClose(reason) : true;
        if (proceed && day) proceed = await day.requestClose(reason);
        if (workspace.planner.recovery || Object.values(workspace.timers.recovery).some(Boolean)) proceed = false;
        await workspace.settled();
        if (pendingExitId)
          await commands.resolveExitRequest(pendingExitId, proceed);
        else if (proceed && workspace.editor)
          await commands.setEditGuard(workspace.planner.guarded);
        if (proceed) {
          workspace.editor = null;
          workspace.detail = null;
          const focus = trigger;
          requestAnimationFrame(() => {
            if (focus?.isConnected) focus.focus();
            else
              document
                .querySelector<HTMLButtonElement>('[data-new-task]')
                ?.focus();
          });
        }
        return proceed;
      } catch (e) {
        workspace.notice = errorMessage(e);
        if (pendingExitId)
          await commands
            .resolveExitRequest(pendingExitId, false)
            .catch(() => {});
        return false;
      } finally {
        closing = null;
        pendingExitId = undefined;
      }
    })();
    return closing;
  }
  async function navigate(
    destination: string,
    view: 'board' | 'week' | 'notes' = 'board',
  ) {
    if (await closeEditor('navigation'))
      await workspace.select(destination, view);
  }
  async function openProject(project: ProjectRecord | null) {
    if (!(await closeEditor('navigation'))) return;
    try {
      trigger = document.activeElement as HTMLElement;
      await commands.setEditGuard(true);
      workspace.editor = { kind: 'project', project };
    } catch (e) {
      workspace.notice = errorMessage(e);
    }
  }
  async function openNew() {
    const project = selectedProject ?? workspace.projects.find(p => p.id === workspace.planner.scope) ?? workspace.projects[0];
    if (!project) return;
    if (!(await closeEditor('navigation'))) return;
    try {
      trigger = document.activeElement as HTMLElement;
      await commands.setEditGuard(true);
      workspace.staged = [];
      workspace.editor = { kind: 'new', projectId: project.id };
    } catch (e) {
      workspace.notice = errorMessage(e);
    }
  }
  async function openTask(taskId: string, revealProject = false) {
    if (!(await closeEditor('navigation'))) return;
    try {
      trigger = document.activeElement as HTMLElement;
      const detail = await workspace.loadDetail(taskId);
      if (revealProject)
        await workspace.select(detail.task.project_id, 'board');
      await commands.setEditGuard(true);
      workspace.staged = [];
      workspace.editor = { kind: 'detail', taskId };
    } catch (e) {
      workspace.notice = errorMessage(e);
    }
  }
  async function projectSave(input: { name: string; color: ProjectColor }) {
    if (workspace.editor?.kind !== 'project') return;
    const project = workspace.editor.project;
    const result = project
      ? await workspace.updateProject(project.id, input)
      : await workspace.createProject(input);
    afterProjectSave = result.id;
  }
  async function projectClose() {
    if (await closeEditor()) {
      if (afterProjectSave) {
        const id = afterProjectSave;
        afterProjectSave = null;
        await workspace.select(id);
      }
    }
  }
  async function created(plan: boolean) {
    if (await closeEditor()) {
      if (plan) {
        await workspace.select('day');
        await tick();
        if (createdTask) day?.planTask(createdTask.task.id);
      }
    }
  }
  async function load() {
    await workspace.load();
  }
  onMount(() => {
    void load();
    let disposed = false;
    const cleanup: (() => void)[] = [];
    void listen<{ requestId: string; intent: 'close' | 'quit' }>(
      'signal://exit-request',
      (event) =>
        void closeEditor(
          event.payload.intent === 'quit' ? 'native-quit' : 'native-close',
          event.payload.requestId,
        ),
    ).then((unlisten) => (disposed ? unlisten() : cleanup.push(unlisten)));
    void subscribeAttachments(
      () =>
        workspace.editor?.kind === 'new'
          ? newDialog
          : workspace.editor?.kind === 'detail'
            ? detailDialog
            : null,
      (e) => (workspace.notice = errorMessage(e)),
    )
      .then((unlisten) => (disposed ? unlisten() : cleanup.push(unlisten)))
      .catch((e) => (workspace.notice = errorMessage(e)));
    const tick = setInterval(() => workspace.tick(), 1000);
    const focus = () => {
      workspace.tick();
      if (workspace.foundation)
        void workspace
          .refresh()
          .catch((e) => (workspace.notice = errorMessage(e)));
    };
    const visibility = () => {
      if (document.visibilityState === 'visible') focus();
    };
    window.addEventListener('focus', focus);
    document.addEventListener('visibilitychange', visibility);
    if (import.meta.env.DEV)
      void document.fonts.ready.then(() =>
        console.info(
          'Signal shell diagnostics',
          JSON.stringify({
            viewport: [innerWidth, innerHeight],
            headerHeight: document
              .querySelector('header')
              ?.getBoundingClientRect().height,
            fonts: [...document.fonts].map((f) => ({
              family: f.family,
              weight: f.weight,
              status: f.status,
            })),
            fontRequests: performance
              .getEntriesByType('resource')
              .filter((r) => r.name.includes('/fonts/'))
              .map((r) => r.name),
          }),
        ),
      );
    return () => {
      disposed = true;
      cleanup.forEach((fn) => fn());
      clearInterval(tick);
      window.removeEventListener('focus', focus);
      document.removeEventListener('visibilitychange', visibility);
    };
  });
  function stub(title: string, text: string, isPalette = false) {
    if (workspace.editor) return;
    modalTitle = title;
    modalText = text;
    palette = isPalette;
    query = '';
    modal.showModal();
  }
  function keyboard(event: KeyboardEvent) {
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === 'k'
    ) {
      event.preventDefault();
      if (!modal.open && !workspace.editor)
        stub(
          'Search or jump to…',
          'Command palette preview. Search results and actions arrive in M7.',
          true,
        );
    } else if (
      event.key === 'n' &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey &&
      !workspace.editor &&
      !document.querySelector('dialog[open]') &&
      selectedProject &&
      workspace.view === 'board' &&
      !(event.target as HTMLElement).closest(
        'input,textarea,select,[contenteditable]',
      )
    ) {
      event.preventDefault();
      void openNew();
    }
  }
  async function toggleTray() {
    if (!data || saving) return;
    saving = true;
    preferenceError = '';
    try {
      await setTrayPreference(!data.tray);
      data.tray = !data.tray;
    } catch (e) {
      preferenceError = errorMessage(e);
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={keyboard} />
<header class="app-header">
  <span class="wordmark">Signal</span>
  <nav class="tabs" aria-label="Workspace">
    <button
      class="tab"
      class:active={active === 'today'}
      aria-current={active === 'today' ? 'page' : undefined}
      onclick={() => navigate('today')}
      >Today <span
        class="badge"
        aria-label={`${data?.badge ?? 0} overdue or due today`}
        >{data?.badge ?? 0}</span
      ></button
    >
    <button
      class="tab"
      class:active={active === 'day'}
      aria-current={active === 'day' ? 'page' : undefined}
      onclick={() => navigate('day')}>Your Day</button
    >
    <span class="divider"></span>
    {#each workspace.projects as project (project.id)}
      <button
        class="tab project-tab"
        class:active={active === project.id}
        data-color={project.color}
        aria-current={active === project.id ? 'page' : undefined}
        onclick={() => navigate(project.id)}
        ><span class="dot"></span>{project.name}</button
      >
    {/each}
    <button
      class="add-project"
      aria-label="Add project"
      title="Add project"
      onclick={() => openProject(null)}><Plus /></button
    >
  </nav>
  <div class="header-actions">
    <TimerStatus
      sessions={workspace.timers.sessions}
      nowUtc={workspace.nowUtc}
      onOpen={(id) => void openTask(id, true)}
    />
    <button
      class="search"
      aria-label="Open command palette"
      onclick={() =>
        stub(
          'Search or jump to…',
          'Command palette preview. Search results and actions arrive in M7.',
          true,
        )}
      ><Search /><span>Search or jump to…</span><kbd>Ctrl K</kbd></button
    >
    <button
      class="settings-button"
      class:selected={active === 'settings'}
      aria-current={active === 'settings' ? 'page' : undefined}
      onclick={() => navigate('settings')}><Settings />Settings</button
    >
  </div>
</header>

{#if workspace.notice}<div class="workspace-notice" role="status">
    {workspace.notice}<button
      class="outline"
      onclick={() =>
        void workspace
          .refresh()
          .catch((e) => (workspace.notice = errorMessage(e)))}
      >Refresh</button
    ><button
      aria-label="Dismiss message"
      onclick={() => (workspace.notice = '')}><X /></button
    >
  </div>{/if}
{#if loading}
  <main class="page">
    <section class="placeholder" role="status">
      <Database />
      <h1>Opening your workspace…</h1>
      <p>Loading local data.</p>
    </section>
  </main>
{:else if error}
  <main class="page">
    <section class="placeholder" role="alert">
      <Database />
      <h1>Signal couldn’t open local data.</h1>
      <p>
        Your workspace could not be loaded. Retry after resolving the error
        below.
      </p>
      <pre>{error}</pre>
      <button class="outline" onclick={load}>Retry</button>
    </section>
  </main>
{:else if active === 'settings'}
  <main class="settings-layout">
    <aside class="settings-nav">
      <div class="eyebrow">Settings</div>
      <nav aria-label="Settings sections">
        {#each settingPages as page}<button
            class:chosen={settingsPage === page}
            aria-current={settingsPage === page ? 'page' : undefined}
            onclick={() => (settingsPage = page)}>{page}</button
          >{/each}
      </nav>
    </aside>
    <section class="settings-content">
      <h1>{settingsPage}</h1>
      {#if settingsPage === 'Notifications'}
        <div class="setting-row">
          <div>
            <h3 id="tray-label">Keep running in the system tray</h3>
            <p id="tray-help">
              Closing the window hides Signal instead of quitting.
            </p>
          </div>
          <button
            class="toggle"
            class:on={data?.tray}
            role="switch"
            aria-checked={data?.tray ?? false}
            aria-labelledby="tray-label"
            aria-describedby="tray-help"
            disabled={saving}
            onclick={toggleTray}><span></span></button
          >
        </div>
        {#if preferenceError}<p role="alert" class="error">
            {preferenceError}
          </p>{/if}
        <p class="tray-tip">
          Use the Signal tray icon → Open to return, or Quit to exit.
        </p>
        <div class="stub-note">
          <Bell />
          <div>
            <h2>Notification settings preview</h2>
            <p>
              Reminders, toast notifications, sounds, and timer actions
              arrive in M6.
            </p>
          </div>
        </div>
      {:else}
        <div class="stub-note">
          <Settings />
          <div>
            <h2>{settingsPage} preview</h2>
            <p>This section is a placeholder for a later milestone.</p>
          </div>
        </div>
      {/if}
    </section>
  </main>
{:else if active === 'day'}
  <YourDay bind:this={day} {workspace} onOpenTask={openTask} onNewTask={openNew} onJoin={commands.openExternalUrl} onSummary={() => stub('Daily summary', 'Summary generation arrives in M5.')} />
{:else if selectedProject}
  <main class="page project-page">
    <ProjectSubBar
      project={selectedProject}
      projects={workspace.projects}
      view={workspace.view}
      onView={(view) => void navigate(active, view)}
      onFilter={() => board?.openFilters()}
      onNewTask={() => void openNew()}
      onEdit={() => void openProject(selectedProject)}
      onMove={(direction) =>
        workspace.moveProject(selectedProject.id, direction)}
      onPreviewDeletion={commands.previewDeletion}
      onDelete={(target, fingerprint) =>
        workspace.delete(target, fingerprint)}
    />
    {#if workspace.view === 'board'}{#key active}<Board
          bind:this={board}
          snapshot={workspace.board}
          runningTaskIds={new Set(
            workspace.timers.sessions
              .filter((s) => s.state === 'running')
              .map((s) => s.task_id),
          )}
          loading={workspace.boardLoading}
          readError={workspace.boardError}
          nowUtc={workspace.nowUtc}
          timeZone={workspace.timeZone}
          selectedDate={workspace.selectedDate}
          selectedTaskId={workspace.editor?.kind === 'detail'
            ? workspace.editor.taskId
            : null}
          onRetry={() => void workspace.loadBoard()}
          onOpenTask={(id) => void openTask(id)}
          onMoveTask={(id, status, before) =>
            workspace.moveTask(id, status, before)}
          onPreview={(message) => (workspace.notice = message)}
        />{/key}
    {:else}<section class="placeholder">
        <h2>{workspace.view === 'week' ? 'Week' : 'Notes'} preview</h2>
        <p>
          {workspace.view === 'week'
            ? 'Week planning and meeting editing arrive in M4.'
            : 'Project Notes is reserved for a later milestone.'}
        </p>
        <button class="outline" onclick={() => navigate(active, 'board')}
          >Return to Board</button
        >
      </section>{/if}
  </main>
{:else}
  <main class="page">
    <section class="destination-heading">
      <div>
        <h1>{heading}</h1>
        <p>
          {data?.runtime.seeded
            ? new Intl.DateTimeFormat('en-US', {
                timeZone: workspace.timeZone,
                dateStyle: 'full',
                timeStyle: 'medium',
              }).format(new Date(workspace.nowUtc)) + ' · Live fixture'
            : 'Your local workspace'}
        </p>
      </div>
      <span class="milestone">M2 · Timers & time entries</span>
    </section>
    <section class="placeholder">
      <div class="placeholder-symbol" data-color="cyan"><Circle /></div>
      <h2>
        {selectedProject
          ? 'Project views are coming next.'
          : active === 'today'
            ? 'Your daily overview starts here.'
            : 'A place to plan your day.'}
      </h2>
      <p>
        {selectedProject
          ? 'Board, Week, and Notes are placeholders. Project and task workflows begin in M1.'
          : active === 'today'
            ? 'The Today digest arrives in M4.'
            : 'The day planner arrives in M3. Open a task to start a timer or log time.'}
      </p>
      <button class="outline" onclick={() => navigate('settings')}
        >Open Settings <ArrowRight /></button
      >
    </section>
    {#if data?.runtime.seeded}<footer class="fixture-status">
        <Database /><span
          >Isolated fixture · {data.projects.length} projects · {data.counts
            .tasks} tasks · {data.counts.meetings} meetings · {data.counts
            .blocks} blocks · {data.counts.time_entries} time entries</span
        >
      </footer>{/if}
  </main>
{/if}

{#if workspace.editor?.kind === 'project'}
  <ProjectDialog
    bind:this={projectDialog}
    project={workspace.editor.project}
    onSave={projectSave}
    onClose={() => void projectClose()}
  />
{:else if workspace.editor?.kind === 'new'}
  <NewTaskDialog
    bind:this={newDialog}
    projects={workspace.projects}
    defaultProjectId={workspace.editor.projectId}
    tags={workspace.board?.tags ?? []}
    timeZone={workspace.timeZone}
    stagedAttachments={workspace.staged}
    onCreate={async (input) => { createdTask = await workspace.createTask(input); return createdTask; }}
    onCreated={(plan) => void created(plan)}
    onStage={(paths) => workspace.stage(paths)}
    onDiscardStaged={(tokens) => workspace.discard(tokens)}
    onOpenExternalUrl={commands.openExternalUrl}
    onClose={() => void closeEditor()}
  />
{:else if workspace.editor?.kind === 'detail' && workspace.detail}
  <TaskDetailDialog
    bind:this={detailDialog}
    detail={workspace.detail}
    time={workspace.timeBindings(workspace.detail.task.id)}
    tags={workspace.board?.tags ?? workspace.detail.tags}
    timeZone={workspace.timeZone}
    stagedAttachments={workspace.staged}
    onPatch={(id, patch) => workspace.patch(id, patch)}
    onSubtasks={(id, inputs) => workspace.setSubtasks(id, inputs)}
    onTags={(id, inputs) => workspace.setTags(id, inputs)}
    onAlerts={(id, offsets) => workspace.setAlerts(id, offsets)}
    onStage={(paths) => workspace.stage(paths)}
    onDiscardStaged={(tokens) => workspace.discard(tokens)}
    onAttach={(id, tokens) => workspace.attach(id, tokens)}
    onRemoveAttachment={(id) => workspace.removeAttachment(id)}
    onOpenAttachment={commands.openAttachment}
    onOpenTaskLink={commands.openTaskLink}
    onOpenExternalUrl={commands.openExternalUrl}
    onPreviewDeletion={commands.previewDeletion}
    onDeleteEntity={(target, fingerprint) =>
      workspace.delete(target, fingerprint)}
    onDeleted={() => {
      void commands
        .setEditGuard(false)
        .then(() => {
          workspace.editor = null;
          workspace.detail = null;
        })
        .catch((e) => (workspace.notice = errorMessage(e)));
    }}
    onClose={() => void closeEditor()}
  />
{/if}

<dialog
  bind:this={modal}
  aria-labelledby="stub-title"
  onclose={() => (query = '')}
>
  <div class="dialog-heading">
    <h2 id="stub-title">{modalTitle}</h2>
    <button aria-label="Close preview" onclick={() => modal.close()}
      ><X /></button
    >
  </div>
  {#if palette}<label class="palette-input"
      ><Search /><input
        aria-label="Search preview"
        bind:value={query}
        placeholder="Search tasks, projects, or commands…"
      /></label
    >{/if}
  <p>{modalText}</p>
  <div class="dialog-footer">
    <span class="eyebrow">Preview only</span><button
      class="outline"
      onclick={() => modal.close()}>Close</button
    >
  </div>
</dialog>

<style>
  .workspace-notice {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-6);
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: var(--text-label);
  }
  .project-page {
    min-height: 0;
    padding-top: var(--board-top);
  }

  .app-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    gap: var(--header-gap);
    padding: 0 var(--header-padding);
    background: var(--bg-raised);
    border-bottom: var(--line) solid var(--border-section);
    white-space: nowrap;
  }
  .wordmark {
    font: var(--weight-bold) var(--text-wordmark) var(--font-heading);
    letter-spacing: var(--tracking-title);
  }
  .tabs {
    display: flex;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: thin;
    height: 100%;
    align-items: center;
    gap: var(--tab-gap);
    min-width: 0;
  }
  .tab {
    flex: none;
    height: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--tab-padding);
    position: relative;
    border: 0;
    border-radius: 0;
    font-weight: var(--weight-medium);
    color: var(--text-muted);
  }
  .tab.active {
    color: var(--text);
  }
  .tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: var(--underline);
    background: var(--text);
  }
  .project-tab.active::after {
    background: var(--project-color);
  }
  .badge {
    font: var(--weight-semibold) var(--text-chip) var(--font-heading);
    padding: var(--line) var(--space-tight);
    background: var(--warn);
    color: var(--on-warn);
    border-radius: var(--radius-input);
  }
  .divider {
    width: var(--line);
    height: var(--space-5);
    background: var(--border-input);
    margin: 0 var(--space-2);
    flex: none;
  }
  .add-project {
    display: grid;
    place-items: center;
    padding: var(--space-2);
    color: var(--text-faint);
  }
  .header-actions {
    flex: none;
    margin-left: auto;
    display: flex;
    gap: var(--space-4);
    align-items: center;
  }
  .search {
    width: var(--search-width);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-tight) var(--space-3);
    background: var(--surface-2);
    border-color: var(--border-section);
    border-radius: var(--radius-input);
    color: var(--text-muted);
    font-size: var(--text-label);
  }
  .search:hover {
    background: var(--surface-3);
  }
  .search kbd {
    margin-left: auto;
    font: var(--text-chip) var(--font-heading);
  }
  .settings-button {
    display: flex;
    gap: var(--space-tight);
    align-items: center;
    color: var(--text-muted);
    padding: var(--space-tight) 0;
  }
  .settings-button.selected {
    color: var(--text);
  }
  .page {
    padding: var(--padding-page);
    min-height: calc(100vh - var(--header-height));
    display: flex;
    flex-direction: column;
  }
  .destination-heading {
    display: flex;
    justify-content: space-between;
    align-items: start;
  }
  .milestone {
    border: var(--line) solid var(--border-section);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-chip);
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  .placeholder {
    margin: auto;
    padding: var(--padding-modal);
    text-align: center;
    max-width: var(--modal-task-new);
  }
  .placeholder p {
    line-height: var(--leading-notes);
  }
  .placeholder-symbol {
    display: inline-flex;
    padding: var(--space-3);
    background: var(--project-task-fill);
    color: var(--project-color);
    border-radius: var(--radius-panel);
    margin-bottom: var(--space-5);
  }
  .placeholder .outline {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .fixture-status {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-faint);
    font-size: var(--text-meta);
    padding-top: var(--space-6);
  }
  .settings-layout {
    display: grid;
    grid-template-columns: var(--settings-nav) 1fr;
    min-height: calc(100vh - var(--header-height));
  }
  .settings-nav {
    background: var(--bg-raised);
    border-right: var(--line) solid var(--border-section);
    padding: var(--space-6) var(--space-3);
  }
  .settings-nav .eyebrow {
    margin: 0 var(--space-3) var(--space-4);
  }
  .settings-nav nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .settings-nav button {
    text-align: left;
    padding: var(--space-small) var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-label);
  }
  .settings-nav .chosen {
    background: var(--surface-3);
    color: var(--text);
  }
  .settings-content {
    padding: var(--space-8);
    max-width: var(--modal-task-detail);
  }
  .settings-content h1 {
    font-size: var(--text-page-small);
    margin-bottom: var(--space-6);
  }
  .setting-row {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    padding-bottom: var(--space-5);
    border-bottom: var(--line) solid var(--border-section);
  }
  .setting-row h3 {
    font: var(--weight-medium) var(--text-base) var(--font-body);
    margin: 0 0 var(--space-1);
  }
  .setting-row p {
    font-size: var(--text-label);
    margin: 0;
  }
  .toggle {
    flex: none;
    margin-left: auto;
    width: var(--toggle-width);
    height: var(--toggle-height);
    border: 0;
    border-radius: var(--radius-toggle);
    background: var(--surface-3);
    padding: var(--toggle-inset);
  }
  .toggle span {
    display: block;
    width: var(--toggle-knob);
    height: var(--toggle-knob);
    border-radius: var(--radius-round);
    background: var(--text-faint);
    transition: transform var(--transition);
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle.on span {
    background: var(--on-accent);
    transform: translateX(
      calc(
        var(--toggle-width) - var(--toggle-knob) - 2 * var(--toggle-inset)
      )
    );
  }
  .tray-tip {
    font-size: var(--text-label);
    margin-top: var(--space-4);
  }
  .stub-note {
    display: flex;
    align-items: start;
    gap: var(--space-3);
    padding: var(--space-5);
    margin-top: var(--space-8);
    border: var(--line) dashed var(--border-dashed);
    border-radius: var(--radius-panel);
    color: var(--text-muted);
  }
  .stub-note h2 {
    font-size: var(--text-base);
  }
  .stub-note p {
    margin: 0;
    font-size: var(--text-label);
  }
  .error {
    color: var(--danger);
    margin-top: var(--space-3);
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--text-muted);
    font-size: var(--text-label);
  }
  dialog {
    background: var(--surface);
    color: var(--text);
    width: var(--modal-task-new);
    padding: var(--padding-modal);
    border: var(--line) solid var(--border-popover);
    border-radius: var(--radius-modal);
    box-shadow: var(--shadow-modal);
  }
  dialog[open] {
    animation: enter var(--transition);
  }
  dialog::backdrop {
    background: var(--scrim);
  }
  .dialog-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-6);
    margin-bottom: var(--space-5);
  }
  .dialog-heading h2 {
    margin: 0;
  }
  .dialog-heading button {
    display: flex;
    padding: var(--space-1);
  }
  .palette-input {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .palette-input input {
    flex: 1;
    min-width: 0;
  }
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: var(--space-6);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: scale(var(--enter-scale));
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    dialog[open] {
      animation: none;
    }
  }
</style>
