// @vitest-environment jsdom
import { it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { reactiveTime } from '../fixtures/timer-bindings.svelte';
import LogTimeForm from '../../src/lib/components/tasks/LogTimeForm.svelte';
import type { TimerUiBindings } from '../../src/lib/components/tasks/TaskTimeCard.svelte';
let component: ReturnType<typeof mount>;
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
function input(label: string, value: string) {
  const el = document.querySelector<HTMLInputElement>(
    `[aria-label="${label}"]`,
  )!;
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}
it('retains a rejected Log draft and submits exactly once while awaiting its result', async () => {
  let reject!: (e: unknown) => void;
  const onLog = vi.fn(() => new Promise<void>((_, r) => (reject = r))),
    onClose = vi.fn();
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
    onLog,
    onRetry: async () => {},
  };
  component = mount(LogTimeForm, {
    target: document.body,
    props: { time, taskName: 'Test task', onClose, onDirty: vi.fn() },
  });
  flushSync();
  input('Log start time', '10:00');
  input('Log seconds', '30');
  const form = document.querySelector('form')!;
  form.dispatchEvent(
    new Event('submit', { bubbles: true, cancelable: true }),
  );
  form.dispatchEvent(
    new Event('submit', { bubbles: true, cancelable: true }),
  );
  expect(onLog).toHaveBeenCalledTimes(1);
  expect(onLog.mock.calls[0]).toEqual([
    { startedAt: '2025-09-11T10:00:00.000Z', durationMs: 30000 },
  ]);
  reject({ code: 'Database', message: 'Disk full' });
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Disk full'),
  );
  expect(onClose).not.toHaveBeenCalled();
  expect(
    document.querySelector<HTMLInputElement>('[aria-label="Log seconds"]')
      ?.value,
  ).toBe('30');
});

it('freezes an uncertain submitted Log and closes it once when Retry confirms the receipt', async () => {
  const onClose = vi.fn();
  const time = reactiveTime();
  time.onLog = async () => {
    time.recoveryRequired = true;
    throw new Error('IPC response lost');
  };
  component = mount(LogTimeForm, {
    target: document.body,
    props: { time, taskName: 'Test', onClose, onDirty: vi.fn() },
  });
  flushSync();
  input('Log start time', '10:00');
  input('Log seconds', '30');
  const editor = component as ReturnType<typeof LogTimeForm>;
  expect(await editor.submit()).toBe(false);
  flushSync();
  expect(document.querySelector('fieldset')?.disabled).toBe(true);
  expect(onClose).not.toHaveBeenCalled();
  time.recoveryRequired = false;
  time.logCompletionVersion++;
  flushSync();
  expect(onClose).toHaveBeenCalledTimes(1);
  time.logCompletionVersion++;
  flushSync();
  expect(onClose).toHaveBeenCalledTimes(1);
});
it('does not clear an unrelated unsent Log draft when completed history changes', () => {
  const onClose = vi.fn();
  const time = reactiveTime({ logCompletionVersion: 1 });
  component = mount(LogTimeForm, {
    target: document.body,
    props: { time, taskName: 'Test', onClose, onDirty: vi.fn() },
  });
  flushSync();
  input('Log start time', '10:00');
  time.logCompletionVersion++;
  flushSync();
  expect(onClose).not.toHaveBeenCalled();
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Log start time"]',
    )?.value,
  ).toBe('10:00');
});
