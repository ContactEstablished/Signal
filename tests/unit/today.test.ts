// @vitest-environment jsdom
import { it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import Today from '../../src/lib/views/Today.svelte';
import { today } from './agenda-fixtures';
let component: ReturnType<typeof mount>;
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
it('opens the same hidden planner occurrence and plans the existing task without creating it', () => {
  const open = vi.fn(),
    plan = vi.fn();
  const snapshot = structuredClone(today);
  snapshot.meetings[0].show_in_day = false;
  component = mount(Today, {
    target: document.body,
    props: {
      snapshot,
      nowUtc: '2025-09-12T12:00:00Z',
      timeZone: 'UTC',
      onRetry: vi.fn(),
      onOpenTask: vi.fn(),
      onOpenMeeting: open,
      onPlanTask: plan,
      onJoin: vi.fn(),
      onNewMeeting: vi.fn(),
      onOpenYourDay: vi.fn(),
    },
  });
  flushSync();
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent?.trim() === 'Plan →')!
    .click();
  expect(plan).toHaveBeenCalledWith('t');
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent?.includes('Meeting ·'))!
    .click();
  expect(open).toHaveBeenCalledWith({ meetingId: 'm', occurrenceKey: 'o:0' });
  expect(document.querySelector('.hours')?.textContent).toContain('1h');
});
it('does not show Plan for a task already scheduled today', () => {
  const snapshot = structuredClone(today);
  snapshot.due_today[0].planned_ranges = [
    { date: '2025-09-12', start_min: 540, end_min: 600 },
  ];
  component = mount(Today, {
    target: document.body,
    props: {
      snapshot,
      nowUtc: '2025-09-12T12:00:00Z',
      timeZone: 'UTC',
      onRetry: vi.fn(),
      onOpenTask: vi.fn(),
      onOpenMeeting: vi.fn(),
      onPlanTask: vi.fn(),
      onJoin: vi.fn(),
      onNewMeeting: vi.fn(),
      onOpenYourDay: vi.fn(),
    },
  });
  flushSync();
  expect(
    [...document.querySelectorAll('button')].some(
      (b) => b.textContent?.trim() === 'Plan →',
    ),
  ).toBe(false);
});

it.each([
  { hours: [0.5, 0.25, 0], widths: ['100%', '50%', '0%'] },
  { hours: [0, 0, 0], widths: ['0%', '0%', '0%'] },
])(
  'scales completed-hours bars to the largest actual total: $hours',
  ({ hours, widths }) => {
    const snapshot = structuredClone(today);
    snapshot.hours_this_week = hours.map((hours, i) => ({
      ...today.hours_this_week[0],
      project_id: `p${i}`,
      hours,
    }));
    component = mount(Today, {
      target: document.body,
      props: {
        snapshot,
        nowUtc: '2025-09-12T12:00:00Z',
        timeZone: 'UTC',
        onRetry: vi.fn(),
        onOpenTask: vi.fn(),
        onOpenMeeting: vi.fn(),
        onPlanTask: vi.fn(),
        onJoin: vi.fn(),
        onNewMeeting: vi.fn(),
        onOpenYourDay: vi.fn(),
      },
    });
    flushSync();
    expect(
      [...document.querySelectorAll<HTMLElement>('.track span')].map(
        (el) => el.style.width,
      ),
    ).toEqual(widths);
  },
);
