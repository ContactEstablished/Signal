import { it, expect, vi, beforeEach } from 'vitest';
import { Workspace } from '../../src/lib/state/app.svelte';
import { newTaskDraft } from '../../src/lib/domain/task-draft';
import { week } from './agenda-fixtures';
import type {
  TaskDetail,
  ProjectRecord,
  TaskRecord,
} from '../../src/lib/domain/types';
const api = vi.hoisted(() => ({
  applyMeeting: vi.fn(),
  getWeek: vi.fn(),
  setEditGuard: vi.fn(),
  createTask: vi.fn(),
  moveProject: vi.fn(),
  updateTask: vi.fn(),
  listProjects: vi.fn(),
  getBoard: vi.fn(),
  getTaskDetail: vi.fn(),
  open: vi.fn(),
  stopTimer: vi.fn(),
  deleteEntity: vi.fn(),
  getTaskTime: vi.fn(),
}));
vi.mock('../../src/lib/native/commands', () => api);
vi.mock('../../src/lib/native/timers', () => api);
vi.mock('../../src/lib/native/agenda', () => api);
vi.mock('../../src/lib/db/client', () => ({ openFoundation: api.open }));
const project: ProjectRecord = {
  id: 'p',
  name: 'Test',
  color: 'cyan',
  sort_order: 0,
  manager_name: null,
  manager_email: null,
  summary_send_at: null,
  summary_tone: 'brief',
};
const task = {
  ...newTaskDraft('p'),
  id: 'task',
  created_at: '2025-09-11T00:00:00.000Z',
  updated_at: '2025-09-11T00:00:00.000Z',
  done_at: null,
  blocked_since: null,
  sort_order: 0,
  revision: 0,
  hours_worked: 0,
} as TaskRecord;
function reply(revision: number, title = 'Saved'): TaskDetail {
  return {
    task: { ...task, revision, title },
    project,
    subtasks: [],
    tags: [],
    alerts: [],
    attachments: [],
    meetings: [],
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  api.open.mockRejectedValue(new Error('Read unavailable'));
});
it('refreshes the browsed Week after task deadline edits and calendar moves', async () => {
  const w = new Workspace();
  w.projects = [project];
  w.active = 'p';
  w.view = 'week';
  w.timeZone = 'America/New_York';
  w.agenda.followWeek = false;
  w.agenda.weekDate = '2025-09-12';
  w.agenda.week = week;
  api.open.mockResolvedValue({});
  api.listProjects.mockResolvedValue([project]);
  api.getBoard.mockResolvedValue({
    project,
    tasks: [],
    meetings: [],
    tags: [],
  });
  api.getTaskDetail.mockResolvedValue(reply(0));
  await w.loadDetail('task');

  const later = {
    ...week.days[4].tasks[0],
    id: 'task',
    due_at: '2026-09-18T21:00:00.000Z',
  };
  const outside = {
    ...week,
    days: week.days.map((d) => ({ ...d, tasks: [] })),
    later: [later],
  };
  api.updateTask.mockResolvedValue({
    ...reply(1),
    task: { ...reply(1).task, due_at: later.due_at },
  });
  api.getWeek.mockResolvedValue(outside);
  await w.patch('task', { due_at: later.due_at });
  expect(w.agenda.week?.later[0].due_at).toBe(later.due_at);
  expect(w.agenda.week?.days.flatMap((d) => d.tasks)).toEqual([]);

  const moved = { ...later, due_at: '2025-09-12T21:00:12.345Z' };
  api.applyMeeting.mockResolvedValue({
    detail: null,
    changed_details: [
      { ...reply(2), task: { ...reply(2).task, due_at: moved.due_at } },
    ],
  });
  api.getWeek.mockResolvedValue({
    ...outside,
    later: [],
    days: outside.days.map((d) => ({
      ...d,
      tasks: d.date === '2025-09-12' ? [moved] : [],
    })),
  });
  await w.agendaAction({
    action: 'move_due',
    payload: { taskId: 'task', dueAt: moved.due_at, expectedRevision: 1 },
  });
  expect(w.agenda.week?.later).toEqual([]);
  expect(w.agenda.week?.days[4].tasks[0].due_at).toBe(moved.due_at);
  expect(api.getWeek).toHaveBeenLastCalledWith({
    date: '2025-09-12',
    timeZone: 'America/New_York',
    projectId: 'p',
  });
  expect(w.notice).toBe('');
});
it('publishes a committed creation despite refresh failure and does not retry it', async () => {
  const w = new Workspace();
  w.projects = [project];
  w.active = 'p';
  w.board = { project, tasks: [], meetings: [], tags: [] };
  api.createTask.mockResolvedValue(reply(0));
  expect(await w.createTask(newTaskDraft('p'))).toEqual(reply(0));
  expect(w.board.tasks[0].title).toBe('Saved');
  expect(w.notice).toContain('Saved; refresh failed');
  expect(api.createTask).toHaveBeenCalledTimes(1);
});
it('serializes edits using authoritative revisions even while refresh fails', async () => {
  const w = new Workspace();
  w.editor = { kind: 'detail', taskId: 'task' };
  api.getTaskDetail.mockResolvedValue(reply(0));
  await w.loadDetail('task');
  api.updateTask.mockImplementation(async (_id, _patch, revision) =>
    reply(revision + 1),
  );
  await Promise.all([
    w.patch('task', { title: 'One' }),
    w.patch('task', { title: 'Two' }),
  ]);
  expect(api.updateTask.mock.calls.map((c) => c[2])).toEqual([0, 1]);
  expect(w.detail?.task.revision).toBe(2);
});
it('propagates rejected writes without publishing or calling refresh', async () => {
  const w = new Workspace();
  api.createTask.mockRejectedValue(new Error('Write rejected'));
  await expect(w.createTask(newTaskDraft('p'))).rejects.toThrow(
    'Write rejected',
  );
  expect(api.open).not.toHaveBeenCalled();
  expect(w.detail).toBeNull();
});

it('retains authoritative project order when the following refresh fails', async () => {
  const w = new Workspace();
  const second = {
    ...project,
    id: 'second',
    name: 'Second',
    sort_order: 1,
  };
  w.projects = [project, second];
  api.moveProject.mockResolvedValue([
    { ...second, sort_order: 0 },
    { ...project, sort_order: 1 },
  ]);
  await w.moveProject('second', 'left');
  expect(w.projects.map((p) => p.id)).toEqual(['second', 'p']);
  expect(w.notice).toContain('Saved; refresh failed');
});

it('publishes Stop accounting before failed refresh and supplies its revision to a queued edit', async () => {
  const w = new Workspace();
  w.editor = { kind: 'detail', taskId: 'task' };
  w.timers.sessions = [
    {
      id: 's',
      task_id: 'task',
      block_id: null,
      state: 'running',
      started_at: '2025-09-11T00:00:00Z',
      segment_started_at: '2025-09-11T00:00:00Z',
      accumulated_ms: 0,
      ended_at: null,
      revision: 0,
      entry_id: null,
    },
  ];
  api.stopTimer.mockResolvedValue({
    snapshot: {
      sessions: [],
      now_utc: '2025-09-11T01:00:00Z',
      offset_ms: 0,
    },
    detail: { ...reply(1), task: { ...reply(1).task, hours_worked: 1 } },
    entries: [
      {
        id: 'entry',
        task_id: 'task',
        block_id: null,
        started_at: '2025-09-11T00:00:00Z',
        ended_at: '2025-09-11T01:00:00Z',
        minutes: 60,
      },
    ],
    outcome: { session_id: 's', entry_id: 'entry' },
  });
  api.updateTask.mockResolvedValue(reply(2));
  await Promise.all([
    w.timeAction('task', 'stop'),
    w.patch('task', { title: 'After Stop' }),
  ]);
  expect(api.stopTimer).toHaveBeenCalledTimes(1);
  expect(api.updateTask.mock.calls[0][2]).toBe(1);
  expect(w.timers.entries.task[0].minutes).toBe(60);
  expect(w.timers.running).toBe(0);
  expect(w.notice).toContain('Saved; refresh failed');
});

it('clears all deleted project timers before a failed refresh while retaining unrelated sessions', async () => {
  const w = new Workspace();
  const session = {
    id: 's',
    task_id: 'task',
    block_id: null,
    state: 'running' as const,
    started_at: '2025-09-11T00:00:00Z',
    segment_started_at: '2025-09-11T00:00:00Z',
    accumulated_ms: 0,
    ended_at: null,
    revision: 0,
    entry_id: null,
  };
  w.timers.sessions = [session, { ...session, id: 'other', task_id: 'other' }];
  w.timers.entries.task = [];
  api.getBoard.mockResolvedValue({
    project,
    tasks: [task],
    meetings: [],
    tags: [],
  });
  api.deleteEntity.mockResolvedValue({ cleanupPending: false });
  await w.delete({ kind: 'project', id: 'p' }, 'fingerprint');
  expect(w.timers.sessions.map((s) => s.task_id)).toEqual(['other']);
  expect(w.timers.entries.task).toBeUndefined();
  expect(w.notice).toContain('Saved; refresh failed');
});

it('publishes confirmed deadline changes before refresh failure and serializes the next task revision', async () => {
  const w = new Workspace();
  w.editor = { kind: 'detail', taskId: 'task' };
  w.detail = reply(0);
  api.applyMeeting.mockResolvedValue({
    detail: null,
    outcome: { ref: null, removed: false, task_id: 'task' },
    changed_details: [reply(1)],
    affected_project_ids: ['p'],
    affected_task_ids: ['task'],
    revision: null,
    replayed: false,
  });
  api.updateTask.mockResolvedValue(reply(2));
  await w.agendaAction({
    action: 'move_due',
    payload: {
      taskId: 'task',
      dueAt: '2025-09-12T17:00:12.345Z',
      expectedRevision: 0,
    },
  });
  expect(w.detail?.task.revision).toBe(1);
  expect(w.notice).toContain('Saved; refresh failed');
  await w.patch('task', { title: 'After move' });
  expect(api.updateTask.mock.calls[0][2]).toBe(1);
  expect(api.applyMeeting).toHaveBeenCalledOnce();
});
it('agenda exact retry bypasses its own recovery but never planner recovery', async () => {
  const w = new Workspace();
  api.applyMeeting.mockRejectedValueOnce({
    code: 'UnknownOutcome',
    message: 'Lost reply',
  });
  await expect(
    w.agendaAction({
      action: 'move_due',
      payload: { taskId: 'task', dueAt: null, expectedRevision: 0 },
    }),
  ).rejects.toMatchObject({ code: 'UnknownOutcome' });
  await expect(w.createTask(newTaskDraft('p'))).rejects.toThrow('Retry');
  expect(api.createTask).not.toHaveBeenCalled();
  w.planner.recovery = true;
  await expect(w.retryAgenda()).rejects.toThrow();
  expect(api.applyMeeting).toHaveBeenCalledOnce();
  w.planner.recovery = false;
  api.applyMeeting.mockResolvedValue({
    detail: null,
    outcome: { ref: null, removed: false, task_id: 'task' },
    changed_details: [],
    affected_project_ids: [],
    affected_task_ids: [],
    revision: null,
    replayed: true,
  });
  await w.retryAgenda();
  expect(api.applyMeeting.mock.calls[1][0]).toEqual(
    api.applyMeeting.mock.calls[0][0],
  );
  expect(w.agenda.recovery).toBe(false);
});
