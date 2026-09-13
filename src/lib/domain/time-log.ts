import { dateAt } from './clock';
import { deadlineToUtc } from './deadlines';
export interface TimeLogDraft {
  date: string;
  startTime: string;
  offset: string;
  hours: number;
  minutes: number;
  seconds: number;
}
export const defaultTimeLog = (
  now: string,
  zone: string,
): TimeLogDraft => ({
  date: dateAt(now, zone),
  startTime: '',
  offset: '',
  hours: 0,
  minutes: 0,
  seconds: 0,
});
export function normalizeTimeLog(
  draft: TimeLogDraft,
  zone: string,
  now: string,
) {
  const { hours, minutes, seconds } = draft;
  if (
    ![hours, minutes, seconds].every(
      (v) => Number.isSafeInteger(v) && v >= 0,
    ) ||
    minutes > 59 ||
    seconds > 59
  )
    throw new Error('Use whole hours, and minutes/seconds from 0 to 59.');
  const durationMs = (hours * 3600 + minutes * 60 + seconds) * 1000;
  if (!Number.isSafeInteger(durationMs) || durationMs <= 0)
    throw new Error('Enter a positive duration.');
  const startedAt = deadlineToUtc(
    draft.date,
    draft.startTime,
    zone,
    draft.offset,
  );
  if (!startedAt) throw new Error('Enter a start date and time.');
  const end = Date.parse(startedAt) + durationMs;
  if (!Number.isFinite(end) || Math.abs(end) > 8.64e15)
    throw new Error('Duration is outside the supported date range.');
  const endedAt = new Date(end).toISOString();
  if (end > Date.parse(now))
    throw new Error('Logged time cannot end in the future.');
  return { startedAt, durationMs, endedAt };
}
