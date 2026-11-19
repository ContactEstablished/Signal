import { meetingStart } from '../domain/agenda-calendar';
import type { Candidate } from './api';
export function meetingIssue(c: Candidate): string {
  if (c.kind !== 'meeting') return '';
  const m = c.meeting;
  if (!m) return 'Supply meeting details.';
  try {
    const fields = meetingStart({date:m.start_local.slice(0,10), time:m.start_local.slice(11), offset:m.start_offset}, m.time_zone);
    if (fields.starts_at !== m.starts_at) return 'Confirm the meeting start time.';
    if (!Number.isInteger(m.duration_min) || m.duration_min < 1 || m.duration_min > 1440) return 'Enter a duration from 1 to 1440 minutes.';
    if (!['none','daily','weekly'].includes(m.repeat_rule)) return 'Choose a supported recurrence.';
    if (m.repeat_rule === 'weekly' && !m.repeat_weekdays?.length) return 'Choose repeat weekdays.';
    if (m.repeat_until && m.repeat_until < m.start_local.slice(0,10)) return 'Repeat end must be on or after the start date.';
    return '';
  } catch (e) { return e instanceof Error ? e.message : 'Confirm the meeting date, time, and timezone.'; }
}
/** One-line result of an email check, shown above the review. */
const n = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;
function found(r: { suggested: number; added?: number }): string {
  const parts = [
    r.added ? `${n(r.added, 'task')} added automatically` : '',
    r.suggested ? `${n(r.suggested, 'suggested task')} added below` : '',
  ].filter(Boolean);
  return parts.length ? parts.join(', ') + '.' : 'no tasks found.';
}
export function emailSummary(r: { checked: number; suggested: number; added?: number; remaining: number; problems: string[] }): string {
  if (!r.checked) return r.problems.length ? '' : 'No new email since the last check.';
  const more = r.remaining ? ` ${n(r.remaining, 'more new email')} waiting; click Check email again.` : '';
  return `Checked ${n(r.checked, 'new email')}: ${found(r)}${more}`;
}
/** One line about the last automatic check. */
export function autoSummary(s: { at: string; checked: number; suggested: number; added: number; remaining: number }, timeZone?: string): string {
  const at = new Date(s.at).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', timeZone });
  if (!s.checked) return `Checked automatically at ${at}. No new email.`;
  const more = s.remaining ? ` ${n(s.remaining, 'more new email')} will be checked in a few minutes.` : '';
  return `Checked automatically at ${at}: ${n(s.checked, 'new email')}, ${found(s)}${more}`;
}
