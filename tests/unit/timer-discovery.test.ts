// @vitest-environment jsdom
import { it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import TimerStatus from '../../src/lib/components/shell/TimerStatus.svelte';
import type { TimerSession } from '../../src/lib/domain/timers';
let component: ReturnType<typeof mount>;
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const session = (
  id: string,
  state: TimerSession['state'] = 'running',
): TimerSession => ({
  id,
  task_id: `task-${id}`,
  task_title: `Task ${id}`,
  project_id: `project-${id}`,
  project_name: `Project ${id}`,
  block_id: null,
  state,
  started_at: '2025-09-11T12:00:00Z',
  segment_started_at: state === 'running' ? '2025-09-11T12:00:00Z' : null,
  accumulated_ms: 0,
  ended_at: null,
  revision: 0,
  entry_id: null,
});
function setup() {
  const state = new SvelteMap([
    [
      'sessions',
      [session('A'), session('B'), session('C'), session('D', 'paused')],
    ],
  ]);
  const onOpen = vi.fn();
  component = mount(TimerStatus, {
    target: document.body,
    props: {
      get sessions() {
        return state.get('sessions')!;
      },
      nowUtc: '2025-09-11T12:01:05Z',
      onOpen,
    },
  });
  flushSync();
  const trigger =
    document.querySelector<HTMLButtonElement>('.timer-status')!;
  return { state, onOpen, trigger };
}
it('lists all running tasks across projects and opens the selected task, excluding paused sessions', () => {
  const { trigger, onOpen } = setup();
  expect(trigger.textContent?.replace(/\s+/g, ' ')).toContain('3 timers running');
  trigger.click();
  flushSync();
  const rows = [
    ...document.querySelectorAll<HTMLButtonElement>('.timer-task'),
  ];
  expect(rows).toHaveLength(3);
  expect(rows[1].textContent).toContain('Task B');
  expect(rows[1].textContent).toContain('Project B');
  expect(rows[1].textContent).toContain('00:01:05');
  rows[1].click();
  flushSync();
  expect(onOpen).toHaveBeenCalledExactlyOnceWith('task-B');
  expect(document.querySelector('.timer-list')).toBeNull();
});
it('supports keyboard dismissal, outside click, and live removal of stopped or paused timers', async () => {
  const { trigger, state } = setup();
  trigger.focus();
  trigger.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }),
  );
  await vi.waitFor(() =>
    expect(document.activeElement?.classList.contains('timer-task')).toBe(
      true,
    ),
  );
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  flushSync();
  expect(document.activeElement).toBe(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  trigger.click();
  flushSync();
  document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }));
  flushSync();
  expect(document.querySelector('.timer-list')).toBeNull();
  trigger.click();
  flushSync();
  state.set('sessions', [session('A')]);
  flushSync();
  expect(trigger.textContent?.replace(/\s+/g, ' ')).toContain('1 timer running');
  expect(document.querySelectorAll('.timer-task')).toHaveLength(1);
  state.set('sessions', []);
  flushSync();
  expect(document.querySelector('.timer-picker')).toBeNull();
});
