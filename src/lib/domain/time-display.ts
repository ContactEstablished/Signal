import { parsePlannerTime } from './planner-time';

/** Display only: stored wall times and minute offsets remain unchanged. */
export function minuteLabel(minute: number): string {
  const hour = Math.floor(minute / 60) % 24;
  return `${hour % 12 || 12}:${String(minute % 60).padStart(2, '0')} ${hour < 12 ? 'AM' : 'PM'}${minute === 1440 ? ' (next day)' : ''}`;
}

export function wallTimeLabel(value: string): string {
  const match = /^(\d{2}):(\d{2})(:\d{2}(?:\.\d{1,3})?)?$/.exec(value);
  if (!match) return value;
  const hour = Number(match[1]);
  const precision = match[3] && Number(match[3].slice(1)) ? match[3] : '';
  return `${hour % 12 || 12}:${match[2]}${precision} ${hour < 12 ? 'AM' : 'PM'}`;
}

export function localDateTimeLabel(value: string): string {
  const [date, time] = value.split('T');
  return time ? `${date} ${wallTimeLabel(time)}` : value;
}

/** Convert typed clock time to the existing wall-time representation. */
export function parseWallTime(value: string): string | null {
  if (!value.trim()) return '';
  const match = /^(\d{1,2})(?::(\d{2}))?(:\d{2}(?:\.\d{1,3})?)?\s*(am|pm)?$/i.exec(value.trim());
  if (!match || (match[3] && Number(match[3].slice(1)) >= 60)) return null;
  const minute = parsePlannerTime(`${match[1]}:${match[2] ?? '00'}${match[4] ?? ''}`);
  if (!Number.isFinite(minute) || minute >= 1440) return null;
  return `${String(Math.floor(minute / 60)).padStart(2, '0')}:${String(minute % 60).padStart(2, '0')}${match[3] ?? ''}`;
}
