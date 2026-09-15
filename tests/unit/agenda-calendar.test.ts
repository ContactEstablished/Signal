import { it, expect } from 'vitest';
import {
  proposeTaskDate,
  resolveTaskDate,
  meetingStart,
  mondayOf,
  newMeetingDraft,
  weeklyStartDate,
} from '../../src/lib/domain/agenda-calendar';
const zone = 'America/New_York';
it('moves only the calendar date while preserving seconds/milliseconds through DST', () => {
  const result = proposeTaskDate(
    { due_at: '2025-10-31T19:30:12.345Z' },
    '2025-11-07',
    zone,
  );
  expect(result).toEqual({ kind: 'ready', due_at: '2025-11-07T20:30:12.345Z' });
  expect(
    proposeTaskDate({ due_at: '2025-11-02T06:30:12.345Z' }, '2025-11-02', zone),
  ).toEqual({ kind: 'ready', due_at: '2025-11-02T06:30:12.345Z' });
});
it('requires explicit review for gaps, folds and missing deadlines', () => {
  expect(
    proposeTaskDate({ due_at: '2025-03-08T07:30:12.345Z' }, '2025-03-09', zone),
  ).toMatchObject({ kind: 'confirm', reason: 'gap', time: '02:30:12.345' });
  expect(
    proposeTaskDate({ due_at: '2025-11-01T05:30:12.345Z' }, '2025-11-02', zone),
  ).toMatchObject({ kind: 'confirm', reason: 'fold' });
  expect(() =>
    resolveTaskDate({ date: '2025-11-02', time: '01:30:12.345' }, zone),
  ).toThrow('twice');
  expect(
    resolveTaskDate(
      { date: '2025-11-02', time: '01:30:12.345', offset: '-05:00' },
      zone,
    ),
  ).toBe('2025-11-02T06:30:12.345Z');
  expect(proposeTaskDate({ due_at: null }, '2025-09-12', zone)).toMatchObject({
    kind: 'confirm',
    time: '17:00:00.000',
    reason: 'no-deadline',
  });
});
it('clears only an explicitly empty deadline and rejects incomplete or invalid dates', () => {
  expect(resolveTaskDate({ date: '', time: '' }, zone)).toBeNull();
  expect(() =>
    resolveTaskDate({ date: '2025-02-29', time: '17:00' }, zone),
  ).toThrow();
  expect(() =>
    resolveTaskDate({ date: '2025-09-12', time: '' }, zone),
  ).toThrow();
  expect(proposeTaskDate({ due_at: '2025-09-12T13:00Z' }, null, zone)).toEqual({
    kind: 'ready',
    due_at: null,
  });
});
it('defaults new meetings to minute 00 and rolls the next hour across midnight', () => {
  expect(mondayOf('2026-01-01')).toBe('2025-12-29');
  const d = newMeetingDraft('2025-09-13T03:59:17.123Z', zone);
  expect(d.start_local).toBe('2025-09-13T00:00:00.000');
  expect(d.duration_min).toBe(30);
  expect(d.show_in_day).toBe(true);
  expect(newMeetingDraft('2025-09-12T13:07:17.123Z', zone).start_local)
    .toBe('2025-09-12T10:00:00.000');
  expect(
    meetingStart({ date: '2025-09-12', time: '09:00:12.345' }, zone).starts_at,
  ).toBe('2025-09-12T13:00:12.345Z');
});
it('starts a weekly schedule on the first selected day on or after its start date', () => {
  expect(weeklyStartDate('2026-09-14', [1, 3])).toBe('2026-09-14');
  expect(weeklyStartDate('2026-09-15', [1, 3])).toBe('2026-09-16');
  expect(weeklyStartDate('2026-09-18', [1, 2, 3, 4])).toBe('2026-09-21');
  expect(() => weeklyStartDate('2026-09-14', [])).toThrow('at least one');
});
