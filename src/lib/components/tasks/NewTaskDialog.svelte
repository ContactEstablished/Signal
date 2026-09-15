<svelte:options runes={true} />

<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { X } from 'lucide-svelte';
  import TaskFields from './TaskFields.svelte';
  import SubtaskList from './SubtaskList.svelte';
  import TaskNotes from './TaskNotes.svelte';
  import TaskTags from './TaskTags.svelte';
  import AttachmentList from './AttachmentList.svelte';
  import {
    newTaskDraft,
    normalizeDraft,
    isDirty,
  } from '../../domain/task-draft';
  import { applyPastedLink } from '../../domain/links';
  import {
    errorMessage,
    type ProjectRecord,
    type Tag,
    type CreateTaskInput,
    type TaskDetail,
    type StagedAttachment,
    type CleanupResult,
    type CloseReason,
  } from '../../domain/types';
  let {
    projects,
    defaultProjectId,
    tags,
    timeZone,
    stagedAttachments,
    onCreate,
    onCreated,
    onStage,
    onDiscardStaged,
    onOpenExternalUrl,
    onClose,
    onMeeting,
  }: {
    projects: ProjectRecord[];
    defaultProjectId: string;
    tags: Tag[];
    timeZone: string;
    stagedAttachments: StagedAttachment[];
    onCreate: (input: CreateTaskInput, plan: boolean) => Promise<TaskDetail>;
    onCreated: (plan: boolean) => void;
    onStage: (paths?: string[]) => Promise<StagedAttachment[]>;
    onDiscardStaged: (tokens: string[]) => Promise<CleanupResult>;
    onOpenExternalUrl: (url: string) => Promise<void>;
    onClose: () => void;
    onMeeting?: () => unknown;
  } = $props();
  let draft = $state(newTaskDraft('')),
    baseline = newTaskDraft(''),
    link = $state(''),
    error = $state(''),
    notice = $state(''),
    pending = $state(false),
    completed = $state(false),
    invalid = $state(false),
    confirm = $state(false),
    dropping = $state(false);
  let subtaskList: SubtaskList;
  let taskTags: TaskTags;
  let subtaskDraft = $state(false),
    tagDraft = $state(false);
  let dialog: HTMLDialogElement;
  let operation: Promise<unknown> | null = null,
    closePromise: Promise<boolean> | null = null,
    resolveClose: ((v: boolean) => void) | null = null;
  onMount(() => {
    draft = newTaskDraft(defaultProjectId);
    baseline = newTaskDraft(defaultProjectId);
    dialog.showModal();
  });
  async function run(action: () => Promise<unknown>) {
    if (operation) return operation;
    pending = true;
    error = '';
    operation = action()
      .catch((e) => {
        error = errorMessage(e);
      })
      .finally(() => {
        operation = null;
        pending = false;
      });
    return operation;
  }
  function parseLink() {
    try {
      const parsed = applyPastedLink(link, draft.title);
      draft = { ...draft, ...parsed };
      error = '';
      notice = parsed.external_provider
        ? `${parsed.external_provider} · ${parsed.external_id}`
        : parsed.external_url
          ? 'Web link · Local task'
          : '';
    } catch (e) {
      error = errorMessage(e);
    }
  }
  async function create(plan = false) {
    if (pending || completed || confirm) return;
    if (invalid) {
      error = 'Finish correcting the date or estimate before creating.';
      return;
    }
    subtaskList.flushPending();
    taskTags.flushPending();
    await run(async () => {
      const parsed = applyPastedLink(link, draft.title);
      const input = normalizeDraft({
        ...$state.snapshot(draft),
        ...parsed,
        attachments: stagedAttachments.map((f) => f.token),
      });
      await onCreate(input, plan);
      completed = true;
      baseline = $state.snapshot(draft);
      onCreated(plan);
    });
  }
  export function dropTarget() {
    return !confirm ? dialog : null;
  }
  export function setDropFeedback(active: boolean) {
    dropping = active;
  }
  export async function stage(paths?: string[]) {
    await run(() => onStage(paths));
  }
  export async function requestClose(_reason: CloseReason): Promise<boolean> {
    if (operation) await operation;
    if (completed) return true;
    if (closePromise) return closePromise;
    if (
      !isDirty(draft, baseline) &&
      !link.trim() &&
      !stagedAttachments.length &&
      !invalid &&
      !subtaskDraft &&
      !tagDraft
    )
      return true;
    confirm = true;
    void tick().then(() => {
      const panel = dialog.querySelector<HTMLElement>('.confirm-panel');
      panel?.scrollIntoView({ block: 'nearest' });
      panel?.querySelector<HTMLButtonElement>('button:last-child')?.focus();
    });
    closePromise = new Promise((r) => (resolveClose = r));
    return closePromise;
  }
  async function decide(discard: boolean) {
    if (discard) {
      try {
        const result = await onDiscardStaged(
          stagedAttachments.map((f) => f.token),
        );
        if (result.cleanupPending)
          notice = 'Discarded. File cleanup will retry.';
      } catch (e) {
        error = errorMessage(e);
        discard = false;
      }
    }
    confirm = false;
    resolveClose?.(discard);
    closePromise = null;
    resolveClose = null;
  }
  function keyboard(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      void create(e.shiftKey);
    }
  }
  function backdrop(node: HTMLDialogElement) {
    const close = (event: MouseEvent) => {
      const b = node.getBoundingClientRect();
      if (
        event.target === node &&
        (event.clientX < b.left ||
          event.clientX > b.right ||
          event.clientY < b.top ||
          event.clientY > b.bottom)
      )
        onClose();
    };
    node.addEventListener('click', close);
    return { destroy: () => node.removeEventListener('click', close) };
  }
</script>

<dialog
  use:backdrop
  class="m1-dialog"
  class:drop-target={dropping}
  bind:this={dialog}
  aria-label="New task"
  onkeydown={keyboard}
  oncancel={(e) => {
    e.preventDefault();
    if (confirm) void decide(false);
    else onClose();
  }}
>
  <div class="dialog-head">
    <select
      aria-label="Project for new task"
      bind:value={draft.project_id}
      disabled={pending || confirm}
      >{#each projects as p}<option value={p.id}>{p.name}</option
        >{/each}</select
    >
    <div class="actions">
      <span class="chip">Task</span><button
        type="button"
        class="outline"
        onclick={() => onMeeting?.()}>Meeting</button
      ><button aria-label="Close new task" onclick={onClose}><X /></button>
    </div>
  </div>
  <fieldset disabled={pending || confirm}>
    <label class="field"
      ><span>Link · Jira, Asana, ClickUp, or a web URL</span><input
        aria-label="Task link"
        placeholder="Paste a link or [title](URL)…"
        bind:value={link}
        onblur={parseLink}
        onpaste={() => setTimeout(parseLink, 0)}
      /></label
    ><label class="field"
      ><span>Title</span><input
        class="task-title"
        placeholder="What needs to get done?"
        bind:value={draft.title}
      /></label
    ><TaskFields
      value={draft}
      alerts={draft.alerts}
      {timeZone}
      onInput={(patch) => (draft = { ...draft, ...patch })}
      onCommit={() => {}}
      onAlerts={(alerts) => (draft = { ...draft, alerts })}
      onInvalid={(v) => (invalid = v)}
      disabled={pending || confirm}
    /><TaskTags
      bind:this={taskTags}
      onDraft={(v) => (tagDraft = v)}
      value={draft.tags}
      {tags}
      onChange={(tags) => (draft = { ...draft, tags })}
      disabled={pending || confirm}
    /><SubtaskList
      bind:this={subtaskList}
      onDraft={(v) => (subtaskDraft = v)}
      value={draft.subtasks}
      onInput={(subtasks) => (draft = { ...draft, subtasks })}
      onCommit={() => {}}
      disabled={pending || confirm}
    /><TaskNotes
      value={draft.notes_md}
      creation
      onInput={(notes_md) => (draft = { ...draft, notes_md })}
      onSave={() => {}}
      onCancel={() => {}}
      onOpenLink={(url) => void run(() => onOpenExternalUrl(url))}
      disabled={pending || confirm}
    /><AttachmentList
      files={[]}
      staged={stagedAttachments}
      {pending}
      onAdd={() => void stage()}
      onOpen={() => {}}
      onRemove={() => {}}
      onDiscard={(token) => void run(() => onDiscardStaged([token]))}
    />
  </fieldset>
  {#if notice}<p class="preview-note" role="status">
      {notice}
    </p>{/if}{#if error}<p class="error" role="alert">
      {error}
    </p>{/if}{#if confirm}<section class="confirm-panel">
      <h3>Discard this task draft?</h3>
      <p>Your unsaved fields and staged attachment copies will be discarded.</p>
      <div class="actions">
        <button class="outline danger" onclick={() => decide(true)}
          >Discard draft</button
        ><button class="outline" onclick={() => decide(false)}
          >Keep editing</button
        >
      </div>
    </section>{/if}
  <div class="dialog-footer">
    <span class="hint">Ctrl ↵ create · Ctrl Shift ↵ create & plan</span><span
      class="spacer"
    ></span><button class="outline" disabled={pending} onclick={onClose}
      >Cancel</button
    ><button
      class="primary"
      disabled={pending || confirm}
      onclick={() => create()}>{pending ? 'Creating…' : 'Create task'}</button
    >
  </div>
</dialog>

<style>
  dialog {
    width: var(--modal-task-new);
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  .task-title {
    font: var(--weight-semibold) var(--text-input-title) var(--font-heading);
    letter-spacing: var(--tracking-title);
  }
  .hint {
    font-size: var(--text-chip);
    color: var(--text-faint);
  }
</style>
