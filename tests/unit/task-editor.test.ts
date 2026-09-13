// @vitest-environment jsdom
import { beforeAll, afterEach, it, expect, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import NewTaskDialog from '../../src/lib/components/tasks/NewTaskDialog.svelte';
import TaskDetailDialog from '../../src/lib/components/tasks/TaskDetailDialog.svelte';
import type { TimerUiBindings } from '../../src/lib/components/tasks/TaskTimeCard.svelte';
import TaskFields from '../../src/lib/components/tasks/TaskFields.svelte';
import { newTaskDraft } from '../../src/lib/domain/task-draft';
import type { TaskDetail, ProjectRecord } from '../../src/lib/domain/types';
let component: ReturnType<typeof mount>;
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLElement.prototype.scrollIntoView = function () {};
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const project: ProjectRecord = {
  id: 'p',
  name: 'Test',
  color: 'cyan',
  manager_name: null,
  manager_email: null,
  summary_send_at: null,
  summary_tone: 'brief',
  sort_order: 0,
};
function form(onCreate = vi.fn().mockResolvedValue({} as TaskDetail)) {
  const onCreated = vi.fn();
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(NewTaskDialog, {
    target,
    props: {
      projects: [project],
      defaultProjectId: 'p',
      tags: [],
      timeZone: 'America/New_York',
      stagedAttachments: [],
      onCreate,
      onCreated,
      onStage: async () => [],
      onDiscardStaged: async () => ({ cleanupPending: false }),
      onOpenExternalUrl: async () => {},
      onClose: () => {},
    },
  });
  flushSync();
  return {
    onCreate,
    onCreated,
    editor: component as ReturnType<typeof NewTaskDialog>,
  };
}
function input(selector: string, text: string) {
  const el = document.querySelector<HTMLInputElement>(selector)!;
  el.value = text;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  return el;
}
function create() {
  document.querySelector('dialog')!.dispatchEvent(
    new KeyboardEvent('keydown', {
      key: 'Enter',
      ctrlKey: true,
      bubbles: true,
    }),
  );
}
it('creates a pasted Markdown link once, closes cleanly, and ignores repeated completion shortcuts', async () => {
  const { onCreate, onCreated, editor } = form();
  input(
    '[aria-label="Task link"]',
    '[Gateway](https://example.atlassian.net/browse/ATL-42)',
  );
  create();
  await vi.waitFor(() => expect(onCreated).toHaveBeenCalledTimes(1));
  expect(onCreate.mock.calls[0][0]).toMatchObject({
    title: 'Gateway',
    external_provider: 'jira',
    external_id: 'ATL-42',
  });
  expect(await editor.requestClose('native-close')).toBe(true);
  create();
  await Promise.resolve();
  expect(onCreate).toHaveBeenCalledTimes(1);
});
it('retains a rejected creation and includes pending subtask/tag text in explicit retry', async () => {
  const onCreate = vi
    .fn()
    .mockRejectedValueOnce(new Error('Disk write rejected'))
    .mockResolvedValue({});
  const { editor } = form(onCreate);
  input('.task-title', 'Retained task');
  input('[aria-label="New subtask"]', 'First step');
  input('[aria-label="New tag name"]', 'Review');
  create();
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Disk write rejected'),
  );
  expect(
    (document.querySelector('.task-title') as HTMLInputElement).value,
  ).toBe('Retained task');
  expect(onCreate.mock.calls[0][0]).toMatchObject({
    subtasks: [{ title: 'First step', done: false }],
    tags: [{ name: 'Review', color: 'cyan' }],
  });
  create();
  await vi.waitFor(() => expect(onCreate).toHaveBeenCalledTimes(2));
  expect(await editor.requestClose('dialog')).toBe(true);
});
it('protects unadded child text on close and returns to editing on Escape', async () => {
  const { editor } = form();
  input('[aria-label="New subtask"]', 'Do not lose this');
  const close = editor.requestClose('native-quit');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Discard this task draft?'),
  );
  document
    .querySelector('dialog')!
    .dispatchEvent(new Event('cancel', { cancelable: true }));
  expect(await close).toBe(false);
  expect(
    (
      document.querySelector(
        '[aria-label="New subtask"]',
      ) as HTMLInputElement
    ).value,
  ).toBe('Do not lose this');
});
it('keeps estimate validation independent from changing the due date', () => {
  const invalid = vi.fn(),
    value = newTaskDraft('p');
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(TaskFields, {
    target,
    props: {
      value,
      alerts: [],
      timeZone: 'America/New_York',
      onInput: () => {},
      onCommit: () => {},
      onAlerts: () => {},
      onInvalid: invalid,
    },
  });
  flushSync();
  input('[inputmode="decimal"]', '-2');
  expect(invalid).toHaveBeenLastCalledWith(true);
  const date = input('[type="date"]', '2025-09-11');
  date.dispatchEvent(new Event('change'));
  flushSync();
  expect(invalid).toHaveBeenLastCalledWith(true);
  input('[inputmode="decimal"]', '0');
  expect(invalid).toHaveBeenLastCalledWith(false);
});

function detailForm(
  onPatch: ReturnType<typeof vi.fn>,
  time?: TimerUiBindings,
) {
  const d: TaskDetail = {
    task: {
      ...newTaskDraft('p'),
      id: 'task',
      title: 'Original',
      hours_worked: 0,
      revision: 0,
      sort_order: 0,
      blocked_since: null,
      done_at: null,
      created_at: '2025-09-11T00:00:00.000Z',
      updated_at: '2025-09-11T00:00:00.000Z',
    },
    project,
    subtasks: [],
    tags: [],
    alerts: [],
    attachments: [],
    meetings: [],
  };
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(TaskDetailDialog, {
    target,
    props: {
      detail: d,
      time,
      tags: [],
      timeZone: 'America/New_York',
      stagedAttachments: [],
      onPatch,
      onSubtasks: async () => d,
      onTags: async () => d,
      onAlerts: async () => d,
      onStage: async () => [],
      onDiscardStaged: async () => ({ cleanupPending: false }),
      onAttach: async () => d,
      onRemoveAttachment: async () => ({
        detail: d,
        cleanupPending: false,
      }),
      onOpenAttachment: async () => {},
      onOpenTaskLink: async () => {},
      onOpenExternalUrl: async () => {},
      onPreviewDeletion: async () => ({
        counts: {},
        fingerprint: 'preview',
      }),
      onDeleteEntity: async () => ({ cleanupPending: false }),
      onDeleted: () => {},
      onClose: () => {},
    },
  });
  flushSync();
  return { d, editor: component as ReturnType<typeof TaskDetailDialog> };
}
it('waits for a pending field write on native close and commits Enter/blur only once', async () => {
  let resolve!: (value: TaskDetail) => void;
  const onPatch = vi.fn(
    () => new Promise<TaskDetail>((r) => (resolve = r)),
  );
  const { d, editor } = detailForm(onPatch);
  const title = input('.title', 'Changed');
  title.focus();
  title.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }),
  );
  expect(onPatch).toHaveBeenCalledTimes(1);
  let closed = false;
  const close = editor.requestClose('native-close').then((result) => {
    closed = result;
    return result;
  });
  await Promise.resolve();
  expect(closed).toBe(false);
  resolve({ ...d, task: { ...d.task, title: 'Changed', revision: 1 } });
  expect(await close).toBe(true);
  expect(onPatch).toHaveBeenCalledTimes(1);
});
it('retains failed detail edits and cancellation consumes Escape without closing the editor', async () => {
  const onPatch = vi.fn().mockRejectedValue(new Error('Revision conflict'));
  const { editor } = detailForm(onPatch);
  const title = input('.title', 'Attempted');
  title.dispatchEvent(new FocusEvent('blur'));
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Revision conflict'),
  );
  const close = editor.requestClose('native-quit');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Unsaved task changes'),
  );
  document
    .querySelector('dialog')!
    .dispatchEvent(new Event('cancel', { cancelable: true }));
  expect(await close).toBe(false);
  expect(title.value).toBe('Attempted');
  title.dispatchEvent(
    new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
      cancelable: true,
    }),
  );
  flushSync();
  expect(title.value).toBe('Original');
});

it('protects an unsent Log draft before closing', async () => {
  const time: TimerUiBindings = {
    session: null,
    entries: [],
    nowUtc: '2025-09-11T12:00:00Z',
    timeZone: 'UTC',
    loading: false,
    error: '',
    pending: false,
    recoveryRequired: false,
    logCompletionVersion: 0,
    onStart: async () => {},
    onPause: async () => {},
    onResume: async () => {},
    onStop: async () => {},
    onLog: async () => {},
    onRetry: async () => {},
  };
  const { editor } = detailForm(vi.fn(), time);
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent?.trim() === 'Log time')!
    .click();
  flushSync();
  input('[aria-label="Log start time"]', '10:00');
  const closing = editor.requestClose('native-quit');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Unsaved task changes'),
  );
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent?.trim() === 'Keep editing')!
    .click();
  expect(await closing).toBe(false);
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Log start time"]',
    )?.value,
  ).toBe('10:00');
});

it('blocks native Close and Quit while a time result is unknown and leaves Retry available', async () => {
  const retry = vi.fn().mockResolvedValue(undefined);
  const time: TimerUiBindings = {
    session: null,
    entries: [],
    nowUtc: '2025-09-11T12:00:00Z',
    timeZone: 'UTC',
    loading: false,
    error: 'The result is unknown.',
    pending: false,
    recoveryRequired: true,
    logCompletionVersion: 0,
    onStart: async () => {},
    onPause: async () => {},
    onResume: async () => {},
    onStop: async () => {},
    onLog: async () => {},
    onRetry: retry,
  };
  const { editor } = detailForm(vi.fn(), time);
  expect(await editor.requestClose('native-close')).toBe(false);
  expect(await editor.requestClose('native-quit')).toBe(false);
  flushSync();
  const button = [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === 'Retry time operation',
  )!;
  expect(button.disabled).toBe(false);
  button.click();
  await vi.waitFor(() => expect(retry).toHaveBeenCalledTimes(1));
});
