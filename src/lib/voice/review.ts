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
