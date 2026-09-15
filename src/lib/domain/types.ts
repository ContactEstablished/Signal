export type Id = string;
export const statuses = [
  'backlog',
  'todo',
  'in_progress',
  'blocked',
  'done',
] as const;
export type TaskStatus = (typeof statuses)[number];
export type Priority = 'low' | 'medium' | 'high';
export type ProjectColor = 'cyan' | 'lime' | 'magenta' | 'violet';
export const projectColors: ProjectColor[] = [
  'cyan',
  'lime',
  'magenta',
  'violet',
];
export const statusLabels: Record<TaskStatus, string> = {
  backlog: 'Backlog',
  todo: 'To Do',
  in_progress: 'In Progress',
  blocked: 'Blocked',
  done: 'Done',
};
export interface ProjectRecord {
  id: Id;
  name: string;
  color: ProjectColor;
  manager_name: string | null;
  manager_email: string | null;
  summary_send_at: string | null;
  summary_tone: 'brief' | 'detailed';
  sort_order: number;
}
export interface TaskRecord {
  id: Id;
  project_id: Id;
  title: string;
  external_url: string | null;
  external_provider: string | null;
  external_id: string | null;
  status: TaskStatus;
  priority: Priority;
  due_at: string | null;
  estimate_h: number | null;
  hours_worked: number;
  blocked_reason: string | null;
  blocked_on: string | null;
  notes_md: string;
  created_at: string;
  updated_at: string;
  done_at: string | null;
  sort_order: number;
  blocked_since: string | null;
  revision: number;
}
export interface Tag {
  id: Id;
  name: string;
  color: string;
}
export interface Subtask {
  id: Id;
  task_id: Id;
  title: string;
  done: number;
  sort_order: number;
}
export interface Attachment {
  id: Id;
  task_id: Id;
  filename: string;
  path: string;
  size: number;
  mime: string;
}
export interface Alert {
  id: Id;
  task_id: Id;
  offset_min: number;
}
export interface Meeting {
  ref?: import('./agenda').MeetingRef;
  id: Id;
  project_id: Id;
  title: string;
  starts_at: string;
  duration_min: number;
  link_url: string | null;
  agenda_md: string;
  notes_md: string;
  repeat_rule: string;
  reminder_min: number;
}
export interface BoardTask extends TaskRecord {
  tags: Tag[];
  subtask_done: number;
  subtask_total: number;
}
export interface BoardSnapshot {
  project: ProjectRecord;
  tasks: BoardTask[];
  meetings: Meeting[];
  tags: Tag[];
}
export interface TaskDetail {
  task: TaskRecord;
  project: ProjectRecord;
  subtasks: Subtask[];
  tags: Tag[];
  attachments: Attachment[];
  alerts: Alert[];
  /** @deprecated Empty compatibility field; use bounded getTaskMeetings. */
  meetings: Meeting[];
}
export interface StagedAttachment {
  token: string;
  filename: string;
  size: number;
  mime: string;
}
export interface SubtaskInput {
  id?: Id;
  title: string;
  done: boolean;
}
export type TagInput = { id: Id } | { name: string; color: string };
export type TaskPatch = Partial<
  Pick<
    TaskRecord,
    | 'title'
    | 'status'
    | 'priority'
    | 'due_at'
    | 'estimate_h'
    | 'external_url'
    | 'external_provider'
    | 'external_id'
    | 'blocked_reason'
    | 'blocked_on'
    | 'notes_md'
  >
>;
export interface CreateTaskInput {
  project_id: Id;
  title: string;
  status: TaskStatus;
  priority: Priority;
  due_at: string | null;
  estimate_h: number | null;
  external_url: string | null;
  external_provider: string | null;
  external_id: string | null;
  blocked_reason: string | null;
  blocked_on: string | null;
  notes_md: string;
  subtasks: SubtaskInput[];
  tags: TagInput[];
  alerts: number[];
  attachments: string[];
}
export interface AppError {
  code: 'Validation' | 'NotFound' | 'Conflict' | 'Database' | 'File';
  message: string;
  field?: string;
}
export type DeletionTarget = { kind: 'task' | 'project'; id: Id };
export interface DeletionPreview {
  counts: Record<string, number>;
  fingerprint: string;
}
export interface CleanupResult {
  cleanupPending: boolean;
}
export interface AttachmentRemoval extends CleanupResult {
  detail: TaskDetail;
}
export type CloseReason =
  | 'dialog'
  | 'navigation'
  | 'native-close'
  | 'native-quit';
export function errorMessage(error: unknown): string {
  return typeof error === 'object' && error && 'message' in error
    ? String(error.message)
    : String(error);
}
