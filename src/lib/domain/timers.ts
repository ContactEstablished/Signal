import type { TaskDetail } from './types';
export interface TimerSession {
  id: string;
  task_id: string;
  task_title?: string;
  project_id?: string;
  project_name?: string;
  block_id: string | null;
  state: 'running' | 'paused' | 'stopped';
  started_at: string;
  segment_started_at: string | null;
  accumulated_ms: number;
  ended_at: string | null;
  revision: number;
  entry_id: string | null;
}
export interface TimeEntry {
  id: string;
  task_id: string;
  block_id: string | null;
  started_at: string;
  ended_at: string;
  minutes: number;
}
export interface ClockSnapshot {
  now_utc: string;
  offset_ms: number;
}
export interface TimerSnapshot extends ClockSnapshot {
  sessions: TimerSession[];
  warnings?: string[];
}
export interface TaskTimeSnapshot {
  snapshot: TimerSnapshot;
  entries: TimeEntry[];
}
export interface TimerWriteResult extends TaskTimeSnapshot {
  detail: TaskDetail;
  outcome: { session_id: string | null; entry_id: string | null };
}
export interface StartInput {
  requestId: string;
  taskId: string;
}
export interface SessionInput extends StartInput {
  sessionId: string;
  expectedRevision: number;
}
export interface LogInput extends StartInput {
  startedAt: string;
  durationMs: number;
}
export type TimerAction = 'start' | 'pause' | 'resume' | 'stop' | 'log';
