import { it, expect, vi, beforeEach } from 'vitest';
import { Workspace } from '../../src/lib/state/app.svelte';
import { newTaskDraft } from '../../src/lib/domain/task-draft';
import type {
  TaskDetail,
  ProjectRecord,
  TaskRecord,
} from '../../src/lib/domain/types';
const api = vi.hoisted(() => ({
  createTask: vi.fn(),
  moveProject: vi.fn(),
  updateTask: vi.fn(),
  listProjects: vi.fn(),
  getBoard: vi.fn(),
  getTaskDetail: vi.fn(),
  open: vi.fn(),
}));
vi.mock('../../src/lib/native/commands', () => api);
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

it('retains authoritative project order when the following refresh fails',async()=>{
 const w=new Workspace();const second={...project,id:'second',name:'Second',sort_order:1};w.projects=[project,second];
 api.moveProject.mockResolvedValue([{...second,sort_order:0},{...project,sort_order:1}]);
 await w.moveProject('second','left');expect(w.projects.map(p=>p.id)).toEqual(['second','p']);expect(w.notice).toContain('Saved; refresh failed');
});
