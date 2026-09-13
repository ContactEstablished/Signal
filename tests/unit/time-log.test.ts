import { it, expect } from 'vitest';
import {
  normalizeTimeLog,
  defaultTimeLog,
} from '../../src/lib/domain/time-log';
const now = '2025-11-04T12:00:00.000Z',
  zone = 'America/New_York';
it('validates positive time and future ends, with overnight durations', () => {
  const d = {
    ...defaultTimeLog(now, zone),
    date: '2025-11-01',
    startTime: '23:45',
    hours: 1,
    minutes: 0,
    seconds: 1,
  };
  const r = normalizeTimeLog(d, zone, now);
  expect(r.durationMs).toBe(3601000);
  expect(r.endedAt).toBe('2025-11-02T04:45:01.000Z');
  for (const hours of [-1, NaN, Infinity, 1.5, Number.MAX_SAFE_INTEGER])
    expect(() => normalizeTimeLog({ ...d, hours }, zone, now)).toThrow();
  expect(() =>
    normalizeTimeLog({ ...d, hours: 0, seconds: 0 }, zone, now),
  ).toThrow();
  expect(() =>
    normalizeTimeLog({ ...d, date: '2025-11-05' }, zone, now),
  ).toThrow('future');
});
it('rejects DST gaps and requires explicit repeated-time offset', () => {
  const d = {
    ...defaultTimeLog(now, zone),
    hours: 1,
    date: '2025-03-09',
    startTime: '02:30',
  };
  expect(() => normalizeTimeLog(d, zone, now)).toThrow('does not exist');
  d.date = '2025-11-02';
  d.startTime = '01:30';
  expect(() => normalizeTimeLog(d, zone, now)).toThrow('twice');
  expect(
    normalizeTimeLog({ ...d, offset: '-04:00' }, zone, now).startedAt,
  ).toBe('2025-11-02T05:30:00.000Z');
  expect(
    normalizeTimeLog({ ...d, offset: '-05:00' }, zone, now).startedAt,
  ).toBe('2025-11-02T06:30:00.000Z');
});
