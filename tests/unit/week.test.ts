// @vitest-environment jsdom
import { it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import Week from '../../src/lib/views/Week.svelte';
import { week } from './agenda-fixtures';
let component: ReturnType<typeof mount>;
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
it('shows the full local deadline in Later so a different year is visible', () => {
  component = mount(Week, {
    target: document.body,
    props: {
      snapshot: {
        ...week,
        later: [
          {
            ...week.days[4].tasks[0],
            id: 'later',
            due_at: '2026-09-19T01:00:00.000Z',
          },
        ],
      },
      timeZone: 'America/New_York',
      onRetry: vi.fn(),
      onOpenTask: vi.fn(),
      onOpenMeeting: vi.fn(),
      onNewMeeting: vi.fn(),
      onNavigateWeek: vi.fn(),
      onMoveDue: vi.fn(),
    },
  });
  flushSync();
  const deadline = document.querySelector('[data-due-target="later"] time');
  expect(deadline?.textContent).toBe('Due Sep 18, 2026');
  expect(deadline?.getAttribute('datetime')).toBe('2026-09-19T01:00:00.000Z');
});
it('routes keyboard moves through the same explicit date editor and cancels without changing the source', async () => {
  const move = vi.fn().mockResolvedValue('cancelled');
  component = mount(Week, {
    target: document.body,
    props: {
      snapshot: week,
      timeZone: 'UTC',
      onRetry: vi.fn(),
      onOpenTask: vi.fn(),
      onOpenMeeting: vi.fn(),
      onNewMeeting: vi.fn(),
      onNavigateWeek: vi.fn(),
      onMoveDue: move,
    },
  });
  flushSync();
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent?.includes('Move to date'))!
    .click();
  await vi.waitFor(() =>
    expect(move).toHaveBeenCalledWith('t', { kind: 'picker' }),
  );
  expect(
    document.querySelector('[data-due-target="2025-09-12"]')?.textContent,
  ).toContain('Keep the deadline');
  expect(document.querySelectorAll('[data-due-target]').length).toBe(9);
  expect(week.days[4].tasks[0].due_at).toBe('2025-09-12T15:30:12.345Z');
});

it('shows a translucent target, cancels on Escape, and commits only the selected drop', async () => {
  vi.stubGlobal(
    'requestAnimationFrame',
    vi.fn(() => 1),
  );
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  const hit = vi.fn();
  Object.defineProperty(document, 'elementFromPoint', {
    value: hit,
    configurable: true,
  });
  HTMLElement.prototype.setPointerCapture = vi.fn();
  HTMLElement.prototype.hasPointerCapture = () => false;
  const move = vi.fn().mockResolvedValue('committed');
  component = mount(Week, {
    target: document.body,
    props: {
      snapshot: week,
      timeZone: 'UTC',
      onRetry: vi.fn(),
      onOpenTask: vi.fn(),
      onOpenMeeting: vi.fn(),
      onNewMeeting: vi.fn(),
      onNavigateWeek: vi.fn(),
      onMoveDue: move,
    },
  });
  flushSync();
  const grip = document.querySelector('.grip')!;
  const target = document.querySelector('[data-due-target="2025-09-13"]')!;
  hit.mockReturnValue(target);
  const pointer = (target: EventTarget, type: string, x: number, y: number) => {
    const e = new MouseEvent(type, {
      bubbles: true,
      button: 0,
      clientX: x,
      clientY: y,
    });
    Object.defineProperty(e, 'pointerId', { value: 7 });
    target.dispatchEvent(e);
    flushSync();
  };
  pointer(grip, 'pointerdown', 100, 200);
  pointer(window, 'pointermove', 300, 200);
  expect(target.querySelector('.preview')).not.toBeNull();
  expect(document.querySelector('.faded')).not.toBeNull();
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  flushSync();
  expect(target.querySelector('.preview')).toBeNull();
  expect(move).not.toHaveBeenCalled();
  pointer(grip, 'pointerdown', 100, 200);
  pointer(window, 'pointermove', 300, 200);
  pointer(window, 'pointerup', 300, 200);
  await vi.waitFor(() =>
    expect(move).toHaveBeenCalledWith('t', {
      kind: 'date',
      date: '2025-09-13',
    }),
  );
  await vi.waitFor(() => expect(target.querySelector('.preview')).toBeNull());
  await vi.waitFor(() =>
    expect(document.activeElement?.classList.contains('move')).toBe(true),
  );
  vi.unstubAllGlobals();
});
