import type { CreateTaskInput } from './types';
export function newTaskDraft(projectId: string): CreateTaskInput {
  return {
    project_id: projectId,
    title: '',
    status: 'backlog',
    priority: 'medium',
    due_at: null,
    estimate_h: null,
    external_url: null,
    external_provider: null,
    external_id: null,
    blocked_reason: null,
    blocked_on: null,
    notes_md: '',
    subtasks: [],
    tags: [],
    alerts: [1440, 60],
    attachments: [],
  };
}
export function estimateValue(value: string): number | null {
  if (!value.trim()) return null;
  const n = Number(value);
  if (!Number.isFinite(n) || n < 0)
    throw new Error('Estimate must be a nonnegative number.');
  return n;
}
export function normalizeDraft(draft: CreateTaskInput): CreateTaskInput {
  const result = structuredClone(draft);
  result.title = result.title.trim();
  if (!result.title) throw new Error('Enter a task title.');
  if (
    result.estimate_h !== null &&
    (!Number.isFinite(result.estimate_h) || result.estimate_h < 0)
  )
    throw new Error('Estimate must be nonnegative.');
  if (!result.project_id) throw new Error('Choose a project.');
  result.alerts = [...new Set(result.alerts)].sort((a, b) => b - a);
  if (result.alerts.some((n) => !Number.isInteger(n) || n < 0))
    throw new Error('Use whole nonnegative alert minutes.');
  result.subtasks = result.subtasks.map((s) => ({
    ...s,
    title: s.title.trim(),
  }));
  if (result.subtasks.some((s) => !s.title))
    throw new Error('Subtasks need a title.');
  return result;
}
export function isDirty(current: unknown, baseline: unknown): boolean {
  return JSON.stringify(current) !== JSON.stringify(baseline);
}
