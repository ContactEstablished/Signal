import { invoke } from '@tauri-apps/api/core';
import type {
  TimerSnapshot,
  TaskTimeSnapshot,
  TimerWriteResult,
  StartInput,
  SessionInput,
  LogInput,
} from '../domain/timers';
export const initializeTimers = (): Promise<TimerSnapshot> =>
  invoke('initialize_timers');
export const getTimers = (): Promise<TimerSnapshot> => invoke('get_timers');
export const getTaskTime = (taskId: string): Promise<TaskTimeSnapshot> =>
  invoke('get_task_time', { taskId });
export const startTimer = (input: StartInput): Promise<TimerWriteResult> =>
  invoke('start_timer', { input });
export const pauseTimer = (
  input: SessionInput,
): Promise<TimerWriteResult> => invoke('pause_timer', { input });
export const resumeTimer = (
  input: SessionInput,
): Promise<TimerWriteResult> => invoke('resume_timer', { input });
export const stopTimer = (input: SessionInput): Promise<TimerWriteResult> =>
  invoke('stop_timer', { input });
export const logTime = (input: LogInput): Promise<TimerWriteResult> =>
  invoke('log_time', { input });
