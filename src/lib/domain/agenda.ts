import type { BoardTask, Meeting, ProjectRecord, TaskDetail } from './types';
export interface MeetingRef {
  meetingId: string;
  occurrenceKey: string;
}
export type Attendance = 'unmarked' | 'attended' | 'missed';
export interface OccurrenceRecord {
  attendance: Attendance;
  occurrence_notes_md: string;
}
export interface AgendaQuery {
  date: string;
  timeZone: string;
  projectId?: string;
}
export interface MeetingFields {
  title: string;
  starts_at: string;
  start_local: string;
  start_offset: string;
  time_zone: string;
  duration_min: number;
  link_url: string | null;
  agenda_md: string;
  notes_md: string;
  reminder_min: number;
  show_in_day: boolean;
  task_ids: string[];
}
export interface SeriesDraft extends MeetingFields {
  repeat_rule: 'none' | 'daily' | 'weekly';
  /** ISO weekdays; omitted for legacy schedules anchored to a single weekday. */
  repeat_weekdays?: number[];
  repeat_until: string | null;
  fold_policy: 'earlier' | 'later';
}
export interface MeetingOccurrence extends Meeting, MeetingFields {
  ref: MeetingRef;
  revision: number;
  attendance: Attendance;
  occurrence_notes_md: string;
  project_name: string;
  project_color: ProjectRecord['color'];
}
export interface AgendaTask extends BoardTask {
  project_name: string;
  project_color: ProjectRecord['color'];
  planned_ranges: { date: string; start_min: number; end_min: number }[];
}
export interface AgendaDay {
  date: string;
  tasks: AgendaTask[];
  meetings: MeetingOccurrence[];
}
export interface TodaySnapshot {
  query: AgendaQuery;
  today: string;
  week_start: string;
  days: AgendaDay[];
  overdue: AgendaTask[];
  due_today: AgendaTask[];
  tomorrow: AgendaTask[];
  meetings: MeetingOccurrence[];
  hours_this_week: {
    project_id: string;
    project_name: string;
    project_color: ProjectRecord['color'];
    hours: number;
  }[];
  counts: {
    overdue: number;
    due_today: number;
    tomorrow: number;
    meetings: number;
  };
  fingerprint: string;
  skipped_dates: string[];
}
export interface WeekSnapshot {
  query: AgendaQuery;
  today: string;
  project: ProjectRecord;
  week_start: string;
  days: AgendaDay[];
  later: AgendaTask[];
  no_date: AgendaTask[];
  overdue_before_week: AgendaTask[];
  overdue_counts: Record<string, number>;
  fingerprint: string;
  skipped_dates: string[];
}
export interface MeetingDetail {
  occurrence: MeetingOccurrence;
  series: Pick<SeriesDraft, 'repeat_rule' | 'repeat_weekdays' | 'repeat_until' | 'fold_policy'> & {
    revision: number;
    is_recurring: boolean;
  };
  linked_tasks: AgendaTask[];
}
export interface TaskMeetingPage {
  task_id: string;
  week_start: string;
  range_start_utc: string;
  range_end_utc: string;
  occurrences: MeetingOccurrence[];
  previous_date: string;
  next_date: string;
}
export type MeetingChange =
  | { action: 'create'; payload: { projectId: string; draft: SeriesDraft } }
  | {
      action: 'edit_occurrence';
      payload: {
        ref: MeetingRef;
        expectedRevision: number;
        draft: MeetingFields;
        occurrence?: OccurrenceRecord;
      };
    }
  | {
      action: 'edit_following';
      payload: {
        ref: MeetingRef;
        expectedRevision: number;
        draft: SeriesDraft;
        occurrence?: OccurrenceRecord;
      };
    }
  | {
      action: 'record';
      payload: {
        ref: MeetingRef;
        expectedRevision: number;
        attendance: Attendance;
        occurrence_notes_md: string;
      };
    }
  | {
      action: 'remove';
      payload: {
        ref: MeetingRef;
        expectedRevision: number;
        scope: 'one_off' | 'occurrence' | 'following';
      };
    }
  | {
      action: 'move_due';
      payload: {
        taskId: string;
        dueAt: string | null;
        expectedRevision: number;
      };
    };
export interface AgendaInput {
  requestId: string;
  change: MeetingChange;
  expectedFingerprint?: string;
}
export interface MeetingWriteResult {
  detail: MeetingDetail | null;
  outcome: { ref: MeetingRef | null; removed: boolean; task_id?: string };
  changed_details: TaskDetail[];
  affected_project_ids: string[];
  affected_task_ids: string[];
  revision: number | null;
  replayed: boolean;
}
export interface MeetingPreview {
  fingerprint: string;
  affected_counts: Record<string, number>;
  preserved_overrides: MeetingOccurrence[];
  skipped_dates: string[];
  diagnostic_range: { start: string; end: string };
  warnings: string[];
}
export type DueDestination =
  | { kind: 'date'; date: string }
  | { kind: 'none' }
  | { kind: 'picker' };
export interface DueDraft {
  date: string;
  time: string;
  offset?: string;
}
export const refKey = (r: MeetingRef) => JSON.stringify(r);
