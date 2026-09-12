import { Temporal } from '@js-temporal/polyfill';
import type { BoardTask, Priority, Meeting, TaskStatus } from './types';
import { statuses } from './types';
import { dateAt, dayBounds } from './clock';
export type DueFilter = 'all' | 'overdue' | 'today' | 'next7days' | 'noDate';
export interface BoardFilterState {
  text: string;
  priorities: Priority[];
  tagIds: string[];
  due: DueFilter;
}
export const emptyFilters = (): BoardFilterState => ({
  text: '',
  priorities: [],
  tagIds: [],
  due: 'all',
});
export function hasFilters(f: BoardFilterState) {
  return !!(
    f.text.trim() ||
    f.priorities.length ||
    f.tagIds.length ||
    f.due !== 'all'
  );
}
export function filterTasks(
  tasks: BoardTask[],
  filters: BoardFilterState,
  now: string,
  zone: string,
) {
  const today = dateAt(now, zone),
    limit = Temporal.PlainDate.from(today).add({ days: 7 }).toString(),
    search = filters.text.trim().toLocaleLowerCase();
  return tasks.filter((t) => {
    if (
      search &&
      !`${t.title} ${t.external_id ?? ''}`.toLocaleLowerCase().includes(search)
    )
      return false;
    if (filters.priorities.length && !filters.priorities.includes(t.priority))
      return false;
    if (
      filters.tagIds.length &&
      !t.tags.some((tag) => filters.tagIds.includes(tag.id))
    )
      return false;
    const due = t.due_at ? dateAt(t.due_at, zone) : null;
    switch (filters.due) {
      case 'overdue':
        return due !== null && due < today && t.status !== 'done';
      case 'today':
        return due === today;
      case 'next7days':
        return due !== null && due > today && due <= limit;
      case 'noDate':
        return due === null;
      default:
        return true;
    }
  });
}
export function groupTasks(tasks: BoardTask[]) {
  return Object.fromEntries(
    statuses.map((status) => [
      status,
      tasks
        .filter((t) => t.status === status)
        .sort(
          (a, b) => a.sort_order - b.sort_order || a.id.localeCompare(b.id),
        ),
    ]),
  ) as Record<TaskStatus, BoardTask[]>;
}
export function urgent(
  task: Pick<BoardTask, 'due_at' | 'status'>,
  now: string,
) {
  return (
    task.status !== 'done' &&
    task.due_at !== null &&
    Date.parse(task.due_at) <= Date.parse(now) + 86400000
  );
}
export function progress(hours: number, estimate: number | null) {
  return estimate && estimate > 0
    ? Math.min(100, Math.max(0, (hours / estimate) * 100))
    : 0;
}
export function dueLabel(value: string | null, zone: string) {
  return value
    ? new Intl.DateTimeFormat('en-US', {
        timeZone: zone,
        month: 'short',
        day: 'numeric',
      }).format(new Date(value))
    : '';
}
export function dropIntent(status: TaskStatus, beforeTaskId: string | null) {
  return { status, beforeTaskId };
}
export function meetingLayout(meetings: Meeting[], date: string, zone: string) {
  const bounds = dayBounds(date, zone),
    dayStart = Date.parse(bounds.start),
    dayEnd = Date.parse(bounds.end);
  const start = Temporal.PlainDate.from(date)
      .toPlainDateTime('09:00')
      .toZonedDateTime(zone).epochMilliseconds,
    end = Temporal.PlainDate.from(date)
      .toPlainDateTime('18:00')
      .toZonedDateTime(zone).epochMilliseconds;
  const lanes: number[] = [];
  return [...meetings]
    .sort(
      (a, b) =>
        a.starts_at.localeCompare(b.starts_at) || a.id.localeCompare(b.id),
    )
    .filter(
      (m) =>
        Date.parse(m.starts_at) < dayEnd &&
        Date.parse(m.starts_at) + m.duration_min * 60000 > dayStart,
    )
    .map((meeting) => {
      const a = Math.max(start, Date.parse(meeting.starts_at)),
        b = Math.min(
          end,
          Date.parse(meeting.starts_at) + meeting.duration_min * 60000,
        ),
        outside = b <= a;
      let lane = 0;
      if (!outside) {
        lane = lanes.findIndex((e) => e <= a);
        if (lane < 0) lane = lanes.length;
        lanes[lane] = b;
      }
      return {
        meeting,
        outside,
        lane,
        left: Math.max(0, ((a - start) / (end - start)) * 100),
        width: outside ? 0 : ((b - a) / (end - start)) * 100,
      };
    });
}
