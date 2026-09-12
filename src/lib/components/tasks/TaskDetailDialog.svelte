<svelte:options runes={true} />

<script lang="ts">
  import { onMount, untrack, tick } from 'svelte';
  import { X, ExternalLink } from 'lucide-svelte';
  import TaskFields from './TaskFields.svelte';
  import SubtaskList from './SubtaskList.svelte';
  import TaskNotes from './TaskNotes.svelte';
  import TaskTags from './TaskTags.svelte';
  import TaskTimeCard from './TaskTimeCard.svelte';
  import AttachmentList from './AttachmentList.svelte';
  import { applyPastedLink } from '../../domain/links';
  import {
    errorMessage,
    type TaskDetail,
    type TaskPatch,
    type SubtaskInput,
    type TagInput,
    type Tag,
    type StagedAttachment,
    type CleanupResult,
    type AttachmentRemoval,
    type DeletionTarget,
    type DeletionPreview,
    type CloseReason,
  } from '../../domain/types';
  let {
    detail,
    tags,
    timeZone,
    stagedAttachments,
    onPatch,
    onSubtasks,
    onTags,
    onAlerts,
    onStage,
    onDiscardStaged,
    onAttach,
    onRemoveAttachment,
    onOpenAttachment,
    onOpenTaskLink,
    onOpenExternalUrl,
    onPreviewDeletion,
    onDeleteEntity,
    onDeleted,
    onClose,
  }: {
    detail: TaskDetail;
    tags: Tag[];
    timeZone: string;
    stagedAttachments: StagedAttachment[];
    onPatch: (id: string, patch: TaskPatch) => Promise<TaskDetail>;
    onSubtasks: (id: string, inputs: SubtaskInput[]) => Promise<TaskDetail>;
    onTags: (id: string, inputs: TagInput[]) => Promise<TaskDetail>;
    onAlerts: (id: string, offsets: number[]) => Promise<TaskDetail>;
    onStage: (paths?: string[]) => Promise<StagedAttachment[]>;
    onDiscardStaged: (tokens: string[]) => Promise<CleanupResult>;
    onAttach: (id: string, tokens: string[]) => Promise<TaskDetail>;
    onRemoveAttachment: (id: string) => Promise<AttachmentRemoval>;
    onOpenAttachment: (id: string) => Promise<void>;
    onOpenTaskLink: (id: string) => Promise<void>;
    onOpenExternalUrl: (url: string) => Promise<void>;
    onPreviewDeletion: (target: DeletionTarget) => Promise<DeletionPreview>;
    onDeleteEntity: (
      target: DeletionTarget,
      fingerprint: string,
    ) => Promise<CleanupResult>;
    onDeleted: () => void;
    onClose: () => void;
  } = $props();
  const keys = [
    'title',
    'status',
    'priority',
    'due_at',
    'estimate_h',
    'external_url',
    'external_provider',
    'external_id',
    'blocked_reason',
    'blocked_on',
    'notes_md',
  ] as const;
  const patchOf = (d: TaskDetail): TaskPatch =>
    Object.fromEntries(keys.map((k) => [k, d.task[k]]));
  const childrenOf = (d: TaskDetail): SubtaskInput[] =>
    d.subtasks.map((s) => ({ id: s.id, title: s.title, done: !!s.done }));
  let buffer = $state<TaskPatch>({}),
    baseline: TaskPatch = {},
    subtasks = $state<SubtaskInput[]>([]),
    subBaseline: SubtaskInput[] = [],
    tagValues = $state<TagInput[]>([]),
    tagBaseline: TagInput[] = [],
    alerts = $state<number[]>([]),
    alertBaseline: number[] = [];
  let error = $state(''),
    notice = $state(''),
    pending = $state(0),
    invalid = $state(false),
    confirm = $state(false),
    deletion = $state<DeletionPreview | null>(null),
    deleteMode = $state(false),
    notesEditing = $state(false),
    dropping = $state(false),
    link = $state('');
  let subtaskList: SubtaskList;
  let taskTags: TaskTags;
  let subtaskDraft = $state(false),
    tagDraft = $state(false),
    deleted = $state(false);
  let dialog: HTMLDialogElement;
  let fields: TaskFields;
  let closePromise: Promise<boolean> | null = null,
    resolveClose: ((v: boolean) => void) | null = null;
  const operations = new Set<Promise<unknown>>();
  let initialized = false;
  const same = (a: unknown, b: unknown) =>
    JSON.stringify(a) === JSON.stringify(b);
  function sync(next: TaskDetail) {
    const patch = patchOf(next);
    for (const k of keys) {
      if (!initialized || same(buffer[k], baseline[k]))
        buffer = { ...buffer, [k]: patch[k] };
    }
    baseline = patch;
    if (!initialized || same(subtasks, subBaseline))
      subtasks = childrenOf(next);
    subBaseline = childrenOf(next);
    const nextTags = next.tags.map((t) => ({ id: t.id }));
    if (!initialized || same(tagValues, tagBaseline)) tagValues = nextTags;
    tagBaseline = nextTags;
    const nextAlerts = next.alerts.map((a) => a.offset_min);
    if (!initialized || same(alerts, alertBaseline)) alerts = nextAlerts;
    alertBaseline = nextAlerts;
    if (!initialized || link === (detail.task.external_url ?? ''))
      link = next.task.external_url ?? '';
    initialized = true;
  }
  $effect(() => {
    const next = detail;
    untrack(() => sync(next));
  });
  onMount(() => dialog.showModal());
  function run<T>(action: () => Promise<T>): Promise<T | undefined> {
    pending++;
    error = '';
    const result = action()
      .catch((e) => {
        error = errorMessage(e);
        return undefined;
      })
      .finally(() => {
        pending--;
        operations.delete(result);
      });
    operations.add(result);
    return result;
  }
  async function commit(selected: (keyof TaskPatch)[]) {
    const patch = Object.fromEntries(
      selected
        .filter((k) => !same(buffer[k], baseline[k]))
        .map((k) => [k, buffer[k]]),
    ) as TaskPatch;
    if (!Object.keys(patch).length) return;
    const result = await run(() => onPatch(detail.task.id, patch));
    if (result) {
      for (const k of selected) {
        if (same(buffer[k], patch[k]))
          buffer = { ...buffer, [k]: result.task[k] };
        baseline = { ...baseline, [k]: result.task[k] };
      }
    }
  }
  async function saveSubtasks() {
    if (same(subtasks, subBaseline)) return;
    const submitted = $state.snapshot(subtasks);
    const result = await run(() => onSubtasks(detail.task.id, submitted));
    if (result) {
      subBaseline = childrenOf(result);
      if (same(subtasks, submitted)) subtasks = subBaseline;
    }
  }
  async function saveTags(values: TagInput[]) {
    tagValues = values;
    const submitted = $state.snapshot(values);
    const result = await run(() => onTags(detail.task.id, submitted));
    if (result) {
      tagBaseline = result.tags.map((t) => ({ id: t.id }));
      if (same(tagValues, submitted)) tagValues = tagBaseline;
    }
  }
  async function saveAlerts(values: number[]) {
    alerts = values;
    const result = await run(() => onAlerts(detail.task.id, [...values]));
    if (result) {
      alertBaseline = result.alerts.map((a) => a.offset_min);
      if (same(alerts, values)) alerts = alertBaseline;
    }
  }
  function dirty() {
    return (
      subtaskDraft ||
      tagDraft ||
      invalid ||
      !same(buffer, baseline) ||
      !same(subtasks, subBaseline) ||
      !same(tagValues, tagBaseline) ||
      !same(alerts, alertBaseline) ||
      stagedAttachments.length > 0 ||
      link !== (detail.task.external_url ?? '')
    );
  }
  async function linkCommit() {
    if (link === (detail.task.external_url ?? '')) return;
    try {
      const parsed = applyPastedLink(link, String(buffer.title ?? ''));
      buffer = { ...buffer, ...parsed };
      await commit([
        'external_url',
        'external_provider',
        'external_id',
        'title',
      ]);
    } catch (e) {
      error = errorMessage(e);
    }
  }
  export function dropTarget() {
    return !confirm && !deleteMode ? dialog : null;
  }
  export function setDropFeedback(active: boolean) {
    dropping = active;
  }
  export async function stage(paths?: string[]) {
    await run(async () => {
      const files = await onStage(paths);
      if (files.length)
        await onAttach(
          detail.task.id,
          files.map((f) => f.token),
        );
    });
  }
  async function retryFiles() {
    await run(() =>
      onAttach(
        detail.task.id,
        stagedAttachments.map((f) => f.token),
      ),
    );
  }
  async function discardTokens(tokens: string[]) {
    const result = await run(() => onDiscardStaged(tokens));
    if (result?.cleanupPending)
      notice = 'Removed from this draft. File cleanup will retry.';
  }
  async function removeFile(id: string) {
    const result = await run(() => onRemoveAttachment(id));
    if (result?.cleanupPending)
      notice =
        'Attachment removed. File cleanup will retry when it becomes available.';
  }
  export async function requestClose(_reason: CloseReason): Promise<boolean> {
    await Promise.allSettled([...operations]);
    if (deleted) return true;
    deleteMode = false;
    deletion = null;
    if (closePromise) return closePromise;
    if (!dirty()) return true;
    confirm = true;
    void tick().then(() => {
      const panel = dialog.querySelector<HTMLElement>('.confirm-panel');
      panel?.scrollIntoView({ block: 'nearest' });
      panel?.querySelector<HTMLButtonElement>('button:last-child')?.focus();
    });
    closePromise = new Promise((r) => (resolveClose = r));
    return closePromise;
  }
  function finishClose(value: boolean) {
    confirm = false;
    resolveClose?.(value);
    resolveClose = null;
    closePromise = null;
  }
  async function closeChoice(choice: 'save' | 'discard' | 'keep') {
    if (choice === 'keep') {
      finishClose(false);
      return;
    }
    error = '';
    if (choice === 'discard') {
      const result = await run(() =>
        onDiscardStaged(stagedAttachments.map((f) => f.token)),
      );
      if (result) {
        if (result.cleanupPending)
          notice = 'Draft discarded. File cleanup remains queued.';
        finishClose(true);
      } else finishClose(false);
      return;
    }
    if (invalid) {
      error = 'Correct the date or estimate before saving.';
      finishClose(false);
      return;
    }
    subtaskList.flushPending();
    taskTags.flushPending();
    await Promise.allSettled([...operations]);
    if (error) {
      finishClose(false);
      return;
    }
    await linkCommit();
    if (error) {
      finishClose(false);
      return;
    }
    await commit([...keys]);
    if (error) {
      finishClose(false);
      return;
    }
    await saveSubtasks();
    if (error) {
      finishClose(false);
      return;
    }
    if (!same(tagValues, tagBaseline)) await saveTags(tagValues);
    if (error) {
      finishClose(false);
      return;
    }
    if (!same(alerts, alertBaseline)) await saveAlerts(alerts);
    if (error) {
      finishClose(false);
      return;
    }
    if (stagedAttachments.length) await retryFiles();
    finishClose(!dirty() && !error);
  }
  async function askDelete() {
    deleteMode = true;
    deletion = null;
    const preview = await run(() =>
      onPreviewDeletion({ kind: 'task', id: detail.task.id }),
    );
    if (preview) deletion = preview;
  }
  async function removeTask() {
    if (!deletion) return;
    const result = await run(() =>
      onDeleteEntity(
        { kind: 'task', id: detail.task.id },
        deletion!.fingerprint,
      ),
    );
    if (result) {
      deleted = true;
      try {
        await onDiscardStaged(stagedAttachments.map((f) => f.token));
      } catch (e) {
        notice =
          'Task deleted; staged files will be recovered on restart. ' +
          errorMessage(e);
      }
      onDeleted();
    } else deletion = null;
  }
  function fieldEscape(e: KeyboardEvent, key: keyof TaskPatch) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      buffer = { ...buffer, [key]: baseline[key] };
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
  class="m1-dialog detail"
  class:drop-target={dropping}
  bind:this={dialog}
  aria-label="Task details"
  oncancel={(e) => {
    e.preventDefault();
    if (pending) return;
    if (confirm) finishClose(false);
    else if (deleteMode) {
      deleteMode = false;
      deletion = null;
    } else onClose();
  }}
>
  <div class="left">
    <div class="breadcrumb" data-color={detail.project.color}>
      <span class="dot"></span>{detail.project.name}<span>/</span><span
        class="chip"
        >{detail.task.external_provider
          ? detail.task.external_id
          : 'Local task'}</span
      >{#if detail.task.external_url}<button
          class="chip"
          onclick={() => void run(() => onOpenTaskLink(detail.task.id))}
          >Open in {detail.task.external_provider ?? 'browser'}<ExternalLink
          /></button
        >{/if}<span class="spacer"></span><button
        aria-label="Close task details"
        onclick={onClose}><X /></button
      >
    </div>
    <label class="field"
      ><span class="sr-label">Task title</span><input
        class="title"
        value={buffer.title ?? ''}
        disabled={!!pending || confirm || deleteMode}
        oninput={(e) => (buffer = { ...buffer, title: e.currentTarget.value })}
        onblur={() => void commit(['title'])}
        onkeydown={(e) => {
          fieldEscape(e, 'title');
          if (e.key === 'Enter') {
            e.preventDefault();
            e.currentTarget.blur();
          }
        }}
      /></label
    >
    <label class="field"
      ><span>External link</span><input
        aria-label="Edit task external link"
        bind:value={link}
        disabled={!!pending || confirm || deleteMode}
        onblur={linkCommit}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            e.currentTarget.blur();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            link = detail.task.external_url ?? '';
          }
        }}
      /></label
    >
    <SubtaskList
      bind:this={subtaskList}
      onDraft={(v) => (subtaskDraft = v)}
      value={subtasks}
      onInput={(v) => (subtasks = v)}
      onCommit={() => void saveSubtasks()}
      disabled={!!pending || confirm || deleteMode}
    />
    <TaskNotes
      value={buffer.notes_md ?? ''}
      persisted={detail.task.notes_md}
      bind:editing={notesEditing}
      disabled={!!pending || confirm || deleteMode}
      onInput={(notes_md) => (buffer = { ...buffer, notes_md })}
      onSave={() =>
        void commit(['notes_md']).then(() => {
          if (!error) notesEditing = false;
        })}
      onCancel={() => (buffer = { ...buffer, notes_md: baseline.notes_md })}
      onOpenLink={(url) => void run(() => onOpenExternalUrl(url))}
    />
    <AttachmentList
      files={detail.attachments}
      staged={stagedAttachments}
      pending={!!pending || confirm || deleteMode}
      onAdd={() => void stage()}
      onOpen={(id) => void run(() => onOpenAttachment(id))}
      onRemove={(id) => void removeFile(id)}
      onDiscard={(token) => void discardTokens([token])}
      onRetry={() => void retryFiles()}
    />
    {#if error}<div class="error" role="alert">
        {error}
        <div class="actions">
          <button class="outline" onclick={() => void closeChoice('save')}
            >Retry pending edits</button
          ><button
            class="outline"
            onclick={() => {
              buffer = { ...baseline };
              subtasks = [...subBaseline];
              tagValues = [...tagBaseline];
              alerts = [...alertBaseline];
              link = detail.task.external_url ?? '';
              fields.resetInvalid();
              error = '';
            }}>Discard pending field edits</button
          >
        </div>
      </div>{/if}{#if notice}<p class="preview-note" role="status">
        {notice}
      </p>{/if}
    {#if confirm}<section class="confirm-panel">
        <h3>Unsaved task changes</h3>
        <p>Save your pending edits or discard them before closing.</p>
        <div class="actions">
          <button
            class="primary"
            disabled={!!pending}
            onclick={() => closeChoice('save')}>Save changes</button
          ><button
            class="outline danger"
            disabled={!!pending}
            onclick={() => closeChoice('discard')}>Discard changes</button
          ><button
            class="outline"
            disabled={!!pending}
            onclick={() => closeChoice('keep')}>Keep editing</button
          >
        </div>
      </section>{/if}
    {#if deleteMode}<section class="confirm-panel">
        <h3>Delete this task permanently?</h3>
        <p>There is no undo. Original attachment files are kept.</p>
        {#if deletion}<ul>
            {#each Object.entries(deletion.counts).filter(([, n]) => n > 0) as [name, n]}<li
              >
                {n}
                {name.replaceAll('_', ' ')}
              </li>{/each}
          </ul>{/if}
        <div class="actions">
          <button
            class="outline"
            disabled={!!pending}
            onclick={() => {
              deleteMode = false;
              deletion = null;
            }}>Cancel</button
          >{#if deletion}<button
              class="outline danger"
              disabled={!!pending}
              onclick={removeTask}>Delete permanently</button
            >{:else}<button
              class="outline"
              disabled={!!pending}
              onclick={askDelete}>Refresh affected records</button
            >{/if}
        </div>
      </section>{/if}
    <footer>
      <span
        >Created {new Intl.DateTimeFormat('en-US', {
          timeZone,
          dateStyle: 'medium',
        }).format(new Date(detail.task.created_at))}</span
      ><span class="spacer"></span><button
        class="danger"
        disabled={!!pending || confirm}
        onclick={askDelete}>Delete task</button
      ><span>Esc to close</span>
    </footer>
  </div>
  <aside>
    <TaskFields
      bind:this={fields}
      value={buffer}
      {alerts}
      {timeZone}
      rail
      onInput={(patch) => (buffer = { ...buffer, ...patch })}
      onCommit={(keys) => void commit(keys)}
      onAlerts={(v) => void saveAlerts(v)}
      onInvalid={(v) => (invalid = v)}
      onCancel={(key) => (buffer = { ...buffer, [key]: baseline[key] })}
      disabled={!!pending || confirm || deleteMode}
    />{#if detail.task.status === 'blocked'}<p class="since">
        Blocked since {detail.task.blocked_since
          ? new Intl.DateTimeFormat('en-US', {
              timeZone,
              dateStyle: 'medium',
            }).format(new Date(detail.task.blocked_since))
          : 'unknown'}
      </p>{/if}<TaskTimeCard
      hours={detail.task.hours_worked}
      estimate={detail.task.estimate_h}
      onPreview={(s) => (notice = s)}
    /><TaskTags
      bind:this={taskTags}
      onDraft={(v) => (tagDraft = v)}
      value={tagValues}
      {tags}
      onChange={(v) => void saveTags(v)}
      disabled={!!pending || confirm || deleteMode}
    />
    <div class="eyebrow">Linked meetings</div>
    {#each detail.meetings as meeting}<button
        class="linked-meeting"
        data-color={detail.project.color}
        onclick={() =>
          (notice = `${meeting.title} · Meeting details arrive in M4.`)}
        >{meeting.title}<span
          >{new Intl.DateTimeFormat('en-US', {
            timeZone,
            month: 'short',
            day: 'numeric',
            hour: 'numeric',
            minute: '2-digit',
          }).format(new Date(meeting.starts_at))}</span
        ></button
      >{:else}<p class="since">No linked meetings</p>{/each}
  </aside>
</dialog>

<style>
  .detail {
    width: var(--modal-task-detail);
    padding: 0;
  }
  .detail[open] {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--task-rail);
  }
  .left {
    padding: var(--padding-modal);
    min-width: 0;
  }
  aside {
    padding: var(--space-6);
    background: var(--rail-bg);
    border-left: var(--line) solid var(--border-section);
  }
  .breadcrumb {
    display: flex;
    gap: var(--space-tight);
    align-items: center;
    flex-wrap: wrap;
    font-size: var(--text-meta);
    color: var(--text-muted);
    margin-bottom: var(--space-5);
  }
  .title {
    font: var(--weight-semibold) var(--text-title) var(--font-heading);
    letter-spacing: var(--tracking-title);
    padding: var(--space-tight) 0;
    border-color: transparent;
    background: transparent;
    width: 100%;
  }
  .sr-label {
    position: absolute;
    clip-path: inset(50%);
    width: 1px;
    height: 1px;
    overflow: hidden;
  }
  footer {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    flex-wrap: wrap;
    color: var(--text-faint);
    font-size: var(--text-chip);
    border-top: var(--line) solid var(--border-section);
    padding-top: var(--space-5);
    margin-top: var(--space-5);
  }
  .since {
    font-size: var(--text-meta);
    margin-top: var(--space-2);
  }
  .linked-meeting {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    width: 100%;
    text-align: left;
    color: var(--project-color);
    background: var(--project-meeting-fill);
    border-color: var(--project-border);
    padding: var(--space-2);
    margin-top: var(--space-2);
  }
  .linked-meeting span {
    font-size: var(--text-meta);
  }
</style>
