// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import VoiceInbox from '../../src/lib/views/VoiceInbox.svelte';
import type { Draft, Candidate } from '../../src/lib/voice/api';
import type { Workspace } from '../../src/lib/state/app.svelte';
const api = vi.hoisted(() => ({
  load: vi.fn(),
  save: vi.fn(),
  recovery: vi.fn(),
  setRecovery: vi.fn(),
  suggest: vi.fn(),
  cancel: vi.fn(),
  discard: vi.fn(),
  settings: vi.fn(),
  emptyDraft: vi.fn(),
}));
const capture = vi.hoisted(() => ({ record: vi.fn() }));
vi.mock('../../src/lib/voice/api', () => api);
vi.mock('../../src/lib/voice/capture', () => capture);
let component: ReturnType<typeof mount>;
let draft: Draft;
const candidate = (id: string): Candidate => ({
  id,
  selected: true,
  project_id: 'p',
  proposal_id: null,
  title: id,
  notes: '',
  subtasks: [],
  due_at: null,
  priority: 'medium',
  estimate_h: null,
  source: 'write release notes',
  warning: '',
  duplicate_ok: false,
  created_id: null,
});
const button = (text: string) =>
  [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text,
  )!;
const edit = (selector: string, value: string) => {
  const el = document.querySelector<HTMLTextAreaElement>(selector)!;
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
};
beforeEach(() => {
  vi.resetAllMocks();
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  draft = {
    id: 'd',
    revision: 1,
    recorded_at: '2026-09-18T16:00:00Z',
    time_zone: 'America/New_York',
    transcript: 'For Atlas, write release notes',
    projects: [],
    candidates: [candidate('First'), candidate('Second')],
  };
  api.load.mockImplementation(async () => structuredClone(draft));
  api.recovery.mockResolvedValue(null);
  api.save.mockImplementation(async (d) => {
    draft = { ...structuredClone(d), revision: d.revision + 1 };
    return structuredClone(draft);
  });
  api.setRecovery.mockResolvedValue(undefined);
  api.cancel.mockResolvedValue(undefined);
});
afterEach(async () => {
  if (component) await unmount(component);
  vi.useRealTimers();
  document.body.innerHTML = '';
});
async function setup() {
  const workspace = {
    nowUtc: '2026-09-18T16:00:00Z',
    timeZone: 'America/New_York',
    projects: [{ id: 'p', name: 'Atlas' }],
    voiceRecovery: false,
    acceptVoice: vi.fn().mockImplementation(async (input) => {
      for (const c of draft.candidates)
        if (input.candidate_ids.includes(c.id)) {
          c.created_id = 'created-' + c.id;
          c.selected = false;
        }
      draft.revision++;
      return {
        created: input.candidate_ids.map((id: string) => ({
          id: 'created-' + id,
          project_id: 'p',
          title: id,
        })),
        projects: [],
      };
    }),
  };
  const p = {
    workspace: workspace as unknown as Workspace,
    onOpenTask: vi.fn(),
    onSettings: vi.fn(),
    onGuard: vi.fn(),
  };
  component = mount(VoiceInbox, { target: document.body, props: p });
  await vi.waitFor(() => expect(button('Select none')).toBeTruthy());
  return { ...p, workspace };
}
it('shows elapsed suggestion time, preserves a timed-out transcript and lets the user retry', async () => {
  draft.candidates = [];
  let fail!: (reason: Error) => void;
  api.suggest.mockImplementationOnce(() => new Promise((_, reject) => { fail = reject; }));
  await setup();
  vi.useFakeTimers();
  button('Suggest tasks').click();
  await vi.advanceTimersByTimeAsync(0);
  flushSync();
  expect(api.suggest).toHaveBeenCalledOnce();
  await vi.advanceTimersByTimeAsync(12_000);
  flushSync();
  expect(document.body.textContent).toContain('12s elapsed');
  expect(button('Cancel')).toBeTruthy();
  fail(new Error('Task suggestions timed out. Your transcript is saved.'));
  await vi.advanceTimersByTimeAsync(0);
  flushSync();
  expect(document.querySelector('[role=alert]')?.textContent).toContain('timed out');
  expect((document.querySelector('[aria-label=Transcript]') as HTMLTextAreaElement).value).toBe(draft.transcript);
  expect(button('Suggest tasks').disabled).toBe(false);
  api.suggest.mockResolvedValueOnce({ ...draft, candidates: [candidate('Retry result')] });
  button('Suggest tasks').click();
  await vi.advanceTimersByTimeAsync(0);
  flushSync();
  expect(api.suggest).toHaveBeenCalledTimes(2);
  expect((document.querySelector('article input[type=text], article input:not([type])') as HTMLInputElement).value).toBe('Retry result');
  expect(document.body.textContent).not.toContain('12s elapsed');
});
it('creates only selected edited suggestions and keeps unselected tasks in the saved review', async () => {
  const p = await setup();
  const selections = document.querySelectorAll<HTMLInputElement>(
    'article input[type=checkbox]',
  );
  selections[2].click();
  flushSync();
  edit('article input[type=text], article input:not([type])', 'Edited title');
  await vi.waitFor(() =>
    expect(button('Create selected (1)').disabled).toBe(false),
  );
  button('Create selected (1)').click();
  await vi.waitFor(() =>
    expect(p.workspace.acceptVoice).toHaveBeenCalledOnce(),
  );
  expect(p.workspace.acceptVoice).toHaveBeenCalledWith(
    expect.objectContaining({ candidate_ids: ['First'], revision: 2 }),
  );
  await vi.waitFor(() => expect(button('Open task')).toBeTruthy());
  button('Open task').click();
  expect(p.onOpenTask).toHaveBeenCalledWith('created-First');
  expect(draft.candidates[0].title).toBe('Edited title');
  expect(draft.candidates[1].created_id).toBeNull();
  expect(api.setRecovery).toHaveBeenLastCalledWith(null);
});
it('a failed AI request preserves transcript and manual recommendations', async () => {
  await setup();
  api.suggest.mockRejectedValue(new Error('Provider unavailable'));
  button('Suggest tasks').click();
  await vi.waitFor(() => expect(button('Continue')).toBeTruthy());
  button('Continue').click();
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Provider unavailable'),
  );
  expect(
    (document.querySelector('[aria-label=Transcript]') as HTMLTextAreaElement)
      .value,
  ).toBe(draft.transcript);
  expect(document.querySelectorAll('article')).toHaveLength(2);
  expect(api.discard).not.toHaveBeenCalled();
});
it('retries the durable original acceptance identity after restart', async () => {
  const pending = {
    request_id: 'original',
    draft_id: 'd',
    revision: 1,
    candidate_ids: ['First'],
  };
  api.recovery.mockResolvedValue(pending);
  const p = await setup();
  expect(p.workspace.voiceRecovery).toBe(true);
  expect(
    button('Select none').disabled ||
      button('Select none').closest('fieldset')!.disabled,
  ).toBe(true);
  button('Retry task creation').click();
  await vi.waitFor(() =>
    expect(p.workspace.acceptVoice).toHaveBeenCalledWith(pending),
  );
  await vi.waitFor(() =>
    expect(api.setRecovery).toHaveBeenLastCalledWith(null),
  );
  expect(p.workspace.voiceRecovery).toBe(false);
});
it('unknown acceptance outcome stays locked and retries the same request', async () => {
  const p = await setup();
  vi.mocked(p.workspace.acceptVoice).mockRejectedValueOnce({
    code: 'UnknownOutcome',
    message: 'Reply lost',
  });
  button('Create selected (2)').click();
  await vi.waitFor(() => expect(button('Retry task creation')).toBeTruthy());
  expect(p.workspace.voiceRecovery).toBe(true);
  const input = vi.mocked(p.workspace.acceptVoice).mock.calls[0][0];
  button('Retry task creation').click();
  await vi.waitFor(() =>
    expect(p.workspace.acceptVoice).toHaveBeenCalledTimes(2),
  );
  expect(vi.mocked(p.workspace.acceptVoice).mock.calls[1][0]).toEqual(input);
});
it('recovers a lost draft-save reply before submitting newer edits', async () => {
  await setup();
  let old: Draft | null = null;
  api.save.mockImplementationOnce(async (d) => {
    old = structuredClone(d);
    throw new Error('Reply lost');
  });
  edit('[aria-label=Transcript]', 'First edit');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Could not save review'),
  );
  edit('[aria-label=Transcript]', 'Newer edit');
  button('Retry saving').click();
  await vi.waitFor(() => expect(api.save).toHaveBeenCalledTimes(3));
  expect(api.save.mock.calls[1][0]).toEqual(old);
  expect(api.save.mock.calls[2][0].transcript).toBe('Newer edit');
  expect(draft.transcript).toBe('Newer edit');
});

it('stopping recording persists the local transcript before AI and ignores a cancelled response', async () => {
  api.load.mockResolvedValue(null);
  api.emptyDraft.mockReturnValue({
    ...draft,
    candidates: [],
    revision: 0,
    transcript: '',
  });
  api.settings.mockResolvedValue({
    settings: { whisper_model: 'base.en', microphone: '' },
    models: [{ id: 'base.en', installed: true }],
  });
  const live = {
    stop: vi.fn().mockResolvedValue('For Atlas, write release notes'),
    cancel: vi.fn().mockResolvedValue(undefined),
  };
  capture.record.mockResolvedValue(live);
  let finish!: (value: Draft) => void;
  api.suggest.mockImplementation(
    () => new Promise<Draft>((resolve) => (finish = resolve)),
  );
  const workspace = {
    nowUtc: draft.recorded_at,
    timeZone: draft.time_zone,
    projects: [],
    voiceRecovery: false,
    acceptVoice: vi.fn(),
  } as unknown as Workspace;
  component = mount(VoiceInbox, {
    target: document.body,
    props: {
      workspace,
      onOpenTask: vi.fn(),
      onSettings: vi.fn(),
      onGuard: vi.fn(),
    },
  });
  await vi.waitFor(() => expect(button('Start recording')).toBeTruthy());
  button('Start recording').click();
  await vi.waitFor(() => expect(capture.record).toHaveBeenCalledOnce());
  button('Stop & analyze').click();
  await vi.waitFor(() => expect(api.suggest).toHaveBeenCalledOnce());
  expect(draft.transcript).toBe('For Atlas, write release notes');
  expect(draft.revision).toBe(2);
  button('Cancel').click();
  await vi.waitFor(() => expect(api.cancel).toHaveBeenCalled());
  finish({ ...draft, candidates: [candidate('Late answer')] });
  await Promise.resolve();
  flushSync();
  expect(document.body.textContent).not.toContain('Late answer');
  expect(document.querySelectorAll('article')).toHaveLength(0);
});

it('keeps a trailing newline while editing multiple subtasks', async () => {
  await setup();
  const area =
    document.querySelectorAll<HTMLTextAreaElement>('article textarea')[1];
  area.value = 'Draft\n';
  area.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  expect(area.value).toBe('Draft\n');
  area.value += 'Review';
  area.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  await vi.waitFor(() => expect(api.save).toHaveBeenCalled());
  expect(draft.candidates[0].subtasks).toEqual(['Draft', 'Review']);
});

it('does not send a transcript when the view closes during its pending save', async () => {
  await setup();
  let finish!: (value: Draft) => void;
  api.save.mockImplementation(
    () => new Promise<Draft>((resolve) => (finish = resolve)),
  );
  edit('[aria-label=Transcript]', 'Updated dictation');
  button('Suggest tasks').click();
  await vi.waitFor(() => expect(button('Continue')).toBeTruthy());
  button('Continue').click();
  await vi.waitFor(() => expect(api.save).toHaveBeenCalled());
  await unmount(component);
  component = null!;
  finish({ ...draft, revision: 2 });
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(api.suggest).not.toHaveBeenCalled();
});
it('a corrected invalid draft is saved instead of replaying rejected fields', async () => {
  await setup();
  api.save.mockRejectedValueOnce({
    code: 'Validation',
    message: 'Transcript too large',
  });
  edit('[aria-label=Transcript]', 'Invalid transcript');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Transcript too large'),
  );
  edit('[aria-label=Transcript]', 'Corrected transcript');
  button('Retry saving').click();
  await vi.waitFor(() => expect(api.save).toHaveBeenCalledTimes(2));
  expect(api.save.mock.calls[1][0].transcript).toBe('Corrected transcript');
});
