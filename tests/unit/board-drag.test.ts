// @vitest-environment jsdom
import { afterEach, beforeEach, it, expect, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import Board from '../../src/lib/views/Board.svelte';
import { newTaskDraft } from '../../src/lib/domain/task-draft';
import type {
  BoardSnapshot,
  BoardTask,
  TaskStatus,
} from '../../src/lib/domain/types';
let component: ReturnType<typeof mount> | undefined;
let hit: ReturnType<typeof vi.fn>;
const released = vi.fn();
beforeEach(() => {
  vi.stubGlobal(
    'requestAnimationFrame',
    vi.fn(() => 1),
  );
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  hit = vi.fn();
  Object.defineProperty(document, 'elementFromPoint', {
    value: hit,
    configurable: true,
  });
  HTMLElement.prototype.setPointerCapture = vi.fn();
  HTMLElement.prototype.hasPointerCapture = () => true;
  HTMLElement.prototype.releasePointerCapture = released;
  released.mockClear();
});
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
});
function rect(top: number, height = 80) {
  return {
    top,
    bottom: top + height,
    left: 200,
    right: 400,
    x: 200,
    y: top,
    width: 200,
    height,
    toJSON() {},
  } as DOMRect;
}
function pointer(
  target: EventTarget,
  type: string,
  x: number,
  y: number,
  id = 7,
) {
  const event = new MouseEvent(type, {
    bubbles: true,
    button: 0,
    clientX: x,
    clientY: y,
  });
  Object.defineProperty(event, 'pointerId', { value: id });
  target.dispatchEvent(event);
  flushSync();
}
function setup() {
  const task = (id: string, status: TaskStatus): BoardTask => ({
    ...newTaskDraft('p'),
    id,
    title: id,
    status,
    sort_order: 0,
    revision: 0,
    hours_worked: 2,
    estimate_h: 4,
    created_at: '2025-09-11T00:00:00.000Z',
    updated_at: '2025-09-11T00:00:00.000Z',
    done_at: null,
    blocked_since: null,
    tags: [],
    subtask_done: 0,
    subtask_total: 0,
  });
  const initial: BoardSnapshot = {
    project: {
      id: 'p',
      name: 'Review',
      color: 'cyan',
      sort_order: 0,
      manager_name: null,
      manager_email: null,
      summary_send_at: null,
      summary_tone: 'brief',
    },
    tasks: [task('A', 'backlog'), task('B', 'todo')],
    tags: [],
    meetings: [],
  };
  const state = new SvelteMap([['snapshot', initial]]);
  let resolve!: () => void, reject!: (e: Error) => void;
  const result = new Promise<void>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  const move = vi.fn(
    async (id: string, status: TaskStatus, before: string | null) => {
      await result;
      state.set('snapshot', {
        ...initial,
        tasks: initial.tasks.map((task) =>
          task.id === id
            ? { ...task, status, sort_order: before ? 0 : 1 }
            : task,
        ),
      });
    },
  );
  const target = document.createElement('div');
  document.body.append(target);
  component = mount(Board, {
    target,
    props: {
      get snapshot() {
        return state.get('snapshot')!;
      },
      nowUtc: '2025-09-11T17:42:00.000Z',
      timeZone: 'America/New_York',
      selectedDate: '2025-09-11',
      onRetry: () => {},
      onOpenTask: () => {},
      onMoveTask: move,
      onPreview: () => {},
    },
  });
  flushSync();
  const source = document.querySelector<HTMLElement>('[data-task-id="A"]')!;
  const grip = source.querySelector<HTMLButtonElement>(
    '[aria-label="Drag A"]',
  )!;
  const lane = document.querySelector<HTMLElement>('[data-status="todo"]')!;
  const b = lane.querySelector<HTMLElement>('[data-task-id="B"]')!;
  b.getBoundingClientRect = () => rect(300);
  hit.mockReturnValue(b);
  return { source, grip, lane, b, move, resolve, reject };
}
it('shows the actual card as an inert preview, holds its slot, and becomes solid on drop before save completes', async () => {
  const { source, grip, lane, move, resolve } = setup();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  const preview = lane.querySelector<HTMLElement>('[data-drop-preview]')!;
  expect(preview.textContent).toContain('A');
  expect(preview.textContent).toContain('2/4h');
  expect(preview.classList.contains('drag-preview')).toBe(true);
  expect(preview.inert).toBe(true);
  expect(preview.querySelector('[aria-label="Drag A"]')).toBeNull();
  expect(source.classList.contains('drag-source')).toBe(true);
  expect(lane.querySelector('.insertion')).toBeNull();
  preview.getBoundingClientRect = () => rect(280, 100);
  // Without the placeholder hit guard this position crosses B's midpoint and flips the anchor.
  pointer(window, 'pointermove', 250, 360);
  pointer(window, 'pointerup', 250, 360);
  expect(move).toHaveBeenCalledExactlyOnceWith('A', 'todo', 'B');
  expect(lane.querySelector('.drop-preview')).not.toBeNull();
  expect(document.querySelector('[data-task-id="A"]')).toBeNull();
  resolve();
  await vi.waitFor(() => {
    expect(lane.querySelector('[data-task-id="A"]')).not.toBeNull();
    expect(lane.querySelector('[data-drop-preview]')).toBeNull();
  });
  expect(released).toHaveBeenCalledWith(7);
});
it('restores the original card and error on rejected drop without a duplicate save', async () => {
  const { grip, move, reject } = setup();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  pointer(window, 'pointerup', 250, 290);
  pointer(window, 'pointerup', 250, 290);
  reject(new Error('Move rejected'));
  await vi.waitFor(() =>
    expect(document.querySelector('[role="alert"]')?.textContent).toContain(
      'Move rejected',
    ),
  );
  expect(
    document.querySelector('[data-status="backlog"] [data-task-id="A"]'),
  ).not.toBeNull();
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
  expect(move).toHaveBeenCalledTimes(1);
});
it('cancels on Escape, pointer cancel, or release outside the Board without writing', () => {
  const { grip, move } = setup();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  window.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }),
  );
  flushSync();
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
  expect(document.querySelector('.drag-source')).toBeNull();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  pointer(window, 'pointercancel', 250, 290);
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  hit.mockReturnValue(document.body);
  pointer(window, 'pointerup', 900, 100);
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
  expect(move).not.toHaveBeenCalled();
  expect(released).toHaveBeenCalledTimes(3);
});
it('ignores a different pointer and treats a grip click as no move', () => {
  const { grip, move } = setup();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290, 8);
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
  pointer(window, 'pointerup', 20, 320);
  expect(move).not.toHaveBeenCalled();
});

it('allows navigation during a pending move without focusing a destroyed Board', async () => {
  const { grip, move, resolve } = setup();
  pointer(grip, 'pointerdown', 20, 320);
  pointer(window, 'pointermove', 250, 290);
  pointer(window, 'pointerup', 250, 290);
  await unmount(component!);
  component = undefined;
  resolve();
  await move.mock.results[0].value;
  await Promise.resolve();
  expect(document.querySelector('[data-drop-preview]')).toBeNull();
});
