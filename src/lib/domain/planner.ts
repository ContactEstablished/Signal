import type { Meeting, ProjectColor, TaskDetail, TaskRecord } from './types';
import type { TimeEntry, TimerSession, TimerSnapshot } from './timers';
export type BlockKind = 'task' | 'break' | 'lunch' | 'focus';
export type BatchMode = 'quick' | 'carry' | 'due' | 'copy';
export interface PlannerQuery { date: string; timeZone: string }
export interface PlannerBlock {
 id: string; date: string; start_min: number; end_min: number; kind: BlockKind;
 task_id: string | null; done: number; done_at: string | null;
 carried_from_block_id: string | null; revision: number;
 time_zone: string | null; start_offset: string | null; end_offset: string | null;
}
export interface PlannerTask extends TaskRecord { project_name: string; project_color: ProjectColor }
export interface PlannerMeeting extends Meeting { project_name: string; project_color: ProjectColor }
export interface BlockDraft {
 kind: BlockKind; task_id: string | null; start_min: number; end_min: number;
 start_offset?: string; end_offset?: string; source_id?: string;
}
export interface PlannerClaim { id: string; mode: Exclude<BatchMode, 'quick'>; source_id: string; task_id: string | null; destination_date: string; block_id: string | null }
export interface PlannerSnapshot {
 date: string; time_zone: string; blocks: PlannerBlock[]; previous_blocks: PlannerBlock[];
 copy_sources: PlannerBlock[]; tasks: PlannerTask[]; meetings: PlannerMeeting[];
 entries: TimeEntry[]; claims: PlannerClaim[]; dismissed: boolean; fingerprint: string; timers: TimerSnapshot;
}
export interface PlannerItem {
 id: string; kind: BlockKind | 'meeting'; start_min: number; end_min: number;
 title: string; project_name: string | null; project_color: ProjectColor | null; task_id: string | null;
 block?: PlannerBlock; meeting?: PlannerMeeting; session?: TimerSession; task?: PlannerTask;
}
export interface PlannerInput {
 requestId: string; query: PlannerQuery; action: 'create' | 'move' | 'done' | 'remove' | 'dismiss' | 'batch';
 payload: Record<string, unknown>; expectedFingerprint?: string;
}
export interface PlannerResult {
 snapshot: PlannerSnapshot; outcome: { block_ids: string[]; task_id?: string; destination_date?: string };
 changed_details: TaskDetail[]; changed_entries: Record<string, TimeEntry[]>;
}
export interface PlanPreview {
 mode: BatchMode; blocks: BlockDraft[]; skipped: { id: string; reason: string }[];
 conflicts: string[]; fingerprint: string;
}
