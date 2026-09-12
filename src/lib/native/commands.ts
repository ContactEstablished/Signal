import { invoke } from '@tauri-apps/api/core';
import type {
  Id,
  ProjectRecord,
  BoardSnapshot,
  TaskDetail,
  TaskPatch,
  CreateTaskInput,
  SubtaskInput,
  TagInput,
  TaskStatus,
  ProjectColor,
  DeletionTarget,
  DeletionPreview,
  StagedAttachment,
  CleanupResult,
  AttachmentRemoval,
} from '../domain/types';
export const listProjects = (): Promise<ProjectRecord[]> =>
  invoke('list_projects', {});
export const getBoard = (
  projectId: Id,
  dayStartUtc: string,
  dayEndUtc: string,
): Promise<BoardSnapshot> =>
  invoke('get_board', { projectId, dayStartUtc, dayEndUtc });
export const getTaskDetail = (taskId: Id): Promise<TaskDetail> =>
  invoke('get_task_detail', { taskId });
export const createProject = (input: {
  name: string;
  color: ProjectColor;
}): Promise<ProjectRecord> => invoke('create_project', { input });
export const updateProject = (
  id: Id,
  input: { name: string; color: ProjectColor },
): Promise<ProjectRecord> => invoke('update_project', { id, input });
export const moveProject = (
  id: Id,
  direction: 'left' | 'right',
): Promise<ProjectRecord[]> => invoke('move_project', { id, direction });
export const createTask = (input: CreateTaskInput): Promise<TaskDetail> =>
  invoke('create_task', { input });
export const updateTask = (
  id: Id,
  patch: TaskPatch,
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('update_task', { id, patch, expectedRevision });
export const moveTask = (
  id: Id,
  status: TaskStatus,
  beforeTaskId: Id | null,
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('move_task', { id, status, beforeTaskId, expectedRevision });
export const setSubtasks = (
  id: Id,
  inputs: SubtaskInput[],
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('set_subtasks', { id, inputs, expectedRevision });
export const setTaskTags = (
  id: Id,
  inputs: TagInput[],
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('set_task_tags', { id, inputs, expectedRevision });
export const setTaskAlerts = (
  id: Id,
  offsets: number[],
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('set_task_alerts', { id, offsets, expectedRevision });
export const previewDeletion = (
  target: DeletionTarget,
): Promise<DeletionPreview> => invoke('preview_deletion', { target });
export const deleteEntity = (
  target: DeletionTarget,
  fingerprint: string,
): Promise<CleanupResult & { deleted: true }> =>
  invoke('delete_entity', { target, fingerprint });
export const stageAttachments = (
  paths?: string[],
): Promise<StagedAttachment[]> =>
  invoke('stage_attachments', { paths: paths ?? null });
export const discardStagedAttachments = (
  tokens: string[],
): Promise<CleanupResult> => invoke('discard_staged_attachments', { tokens });
export const addAttachments = (
  taskId: Id,
  tokens: string[],
  expectedRevision: number,
): Promise<TaskDetail> =>
  invoke('add_attachments', { taskId, tokens, expectedRevision });
export const removeAttachment = (
  id: Id,
  expectedRevision: number,
): Promise<AttachmentRemoval> =>
  invoke('remove_attachment', { id, expectedRevision });
export const openAttachment = (id: Id): Promise<void> =>
  invoke('open_attachment', { id });
export const openTaskLink = (taskId: Id): Promise<void> =>
  invoke('open_task_link', { taskId });
export const openExternalUrl = (url: string): Promise<void> =>
  invoke('open_external_url', { url });
export const setEditGuard = (active: boolean): Promise<void> =>
  invoke('set_edit_guard', { active });
export const resolveExitRequest = (
  requestId: string,
  proceed: boolean,
): Promise<void> => invoke('resolve_exit_request', { requestId, proceed });
export const installFixtureAttachments = (
  fixtures: { id: string; filename: string; mime: string; bytes: number[] }[],
): Promise<void> => invoke('install_fixture_attachments', { fixtures });
