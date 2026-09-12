import { it, expect } from 'vitest';
import {
  filterTasks,
  emptyFilters,
  groupTasks,
  progress,
  urgent,
  meetingLayout,
} from '../../src/lib/domain/board';
import type { BoardTask, Meeting } from '../../src/lib/domain/types';
const now = '2025-09-11T17:42:00.000Z',
  zone = 'America/New_York';
const task = (id: string, extra: Partial<BoardTask> = {}): BoardTask =>
  ({
    id,
    title: id,
    external_id: null,
    status: 'todo',
    priority: 'medium',
    due_at: null,
    tags: [],
    sort_order: 0,
    ...extra,
  }) as BoardTask;
it('combines OR within dimensions and AND across them without hiding undated semantics', () => {
  const items = [
    task('a', {
      priority: 'high',
      due_at: '2025-09-12T02:00:00.000Z',
      tags: [{ id: 'x', name: 'x', color: 'cyan' }],
    }),
    task('b', {
      priority: 'low',
      status: 'done',
      due_at: '2025-09-10T21:00:00.000Z',
    }),
    task('c'),
  ];
  expect(
    filterTasks(
      items,
      {
        ...emptyFilters(),
        priorities: ['high', 'medium'],
        tagIds: ['x', 'y'],
        due: 'today',
      },
      now,
      zone,
    ).map((t) => t.id),
  ).toEqual(['a']);
  expect(
    filterTasks(items, { ...emptyFilters(), due: 'overdue' }, now, zone),
  ).toEqual([]);
  expect(
    filterTasks(items, { ...emptyFilters(), due: 'noDate' }, now, zone).map(
      (t) => t.id,
    ),
  ).toEqual(['c']);
  expect(urgent(items[2], now)).toBe(false);
  expect(groupTasks([task('z'), task('a')]).todo.map((t) => t.id)).toEqual([
    'a',
    'z',
  ]);
  expect(progress(8, 0)).toBe(0);
  expect(progress(13, 12)).toBe(100);
});
it('clips overnight meetings, separates overlaps and retains outside-hours records', () => {
  const m = (id: string, starts_at: string, duration_min: number) =>
    ({ id, starts_at, duration_min }) as Meeting;
  const layout = meetingLayout(
    [
      m('overnight', '2025-09-11T03:00:00Z', 660),
      m('overlap', '2025-09-11T13:30:00Z', 60),
      m('late', '2025-09-11T23:00:00Z', 60),
    ],
    '2025-09-11',
    zone,
  );
  expect(layout.map((r) => r.lane)).toEqual([0, 1, 0]);
  expect(layout[0].left).toBe(0);
  expect(layout[2].outside).toBe(true);
});
