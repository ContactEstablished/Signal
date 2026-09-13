// @vitest-environment jsdom
import { it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import TaskTimeCard, {
  type TimerUiBindings,
} from '../../src/lib/components/tasks/TaskTimeCard.svelte';
let component: ReturnType<typeof mount>;
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const binding = (): TimerUiBindings => ({
  session: null,
  entries: [],
  nowUtc: '2025-09-11T12:00:00Z',
  timeZone: 'UTC',
  loading: false,
  error: '',
  pending: false,
  recoveryRequired: false,
  logCompletionVersion: 0,
  onStart: vi.fn(),
  onPause: vi.fn(),
  onResume: vi.fn(),
  onStop: vi.fn(),
  onLog: vi.fn(),
  onRetry: vi.fn(),
});
const button = (text: string) =>
  [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text,
  )!;
it('suppresses repeated Start while pending and displays failure without changing totals', async () => {
  const time = binding();
  let reject!: (e: Error) => void;
  time.onStart = vi.fn(() => new Promise<void>((_, r) => (reject = r)));
  component = mount(TaskTimeCard, {
    target: document.body,
    props: { hours: 2, estimate: 4, time },
  });
  flushSync();
  button('Start').click();
  button('Start').click();
  expect(time.onStart).toHaveBeenCalledTimes(1);
  reject(new Error('Cannot save'));
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Cannot save'),
  );
  expect(document.querySelector('.hours')?.textContent).toContain('2');
});
it('shows paused elapsed and Resume/Stop, excluding the paused span', () => {
  const time = binding();
  time.session = {
    id: 's',
    task_id: 'a',
    block_id: null,
    state: 'paused',
    started_at: '2025-09-10T10:00:00Z',
    segment_started_at: null,
    accumulated_ms: 65000,
    ended_at: null,
    revision: 1,
    entry_id: null,
  };
  component = mount(TaskTimeCard, {
    target: document.body,
    props: { hours: 2, estimate: 4, time },
  });
  flushSync();
  expect(document.body.textContent).toContain('00:01:05');
  expect(button('Resume')).toBeTruthy();
  expect(button('Stop')).toBeTruthy();
  expect(document.body.textContent).not.toContain('Start preview');
});
