import { invoke } from '@tauri-apps/api/core';
import type {
  AgendaInput,
  AgendaQuery,
  AgendaTask,
  MeetingChange,
  MeetingDetail,
  MeetingPreview,
  MeetingRef,
  MeetingWriteResult,
  TaskMeetingPage,
  TodaySnapshot,
  WeekSnapshot,
} from '../domain/agenda';
export const initializeAgenda = (timeZone: string): Promise<void> =>
  invoke('initialize_agenda', { timeZone });
export const getToday = (query: AgendaQuery): Promise<TodaySnapshot> =>
  invoke('get_today', { query });
export const getWeek = (query: AgendaQuery): Promise<WeekSnapshot> =>
  invoke('get_week', { query });
export const getMeetingDetail = (
  reference: MeetingRef,
): Promise<MeetingDetail> => invoke('get_meeting_detail', { reference });
export const getTaskMeetings = (
  taskId: string,
  query: AgendaQuery,
): Promise<TaskMeetingPage> => invoke('get_task_meetings', { taskId, query });
export const searchAgendaTasks = (input: {
  text: string;
  limit: number;
  cursor?: string;
}): Promise<{ items: AgendaTask[]; nextCursor: string | null }> =>
  invoke('search_agenda_tasks', { input });
export const previewMeetingChange = (
  change: MeetingChange,
): Promise<MeetingPreview> => invoke('preview_meeting_change', { change });
export const applyMeeting = (input: AgendaInput): Promise<MeetingWriteResult> =>
  invoke('apply_meeting', { input });
