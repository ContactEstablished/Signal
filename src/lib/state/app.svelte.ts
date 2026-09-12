import { openFoundation, type Foundation } from '../db/client';
import * as commands from '../native/commands';
import { makeClock, dateAt, dayBounds } from '../domain/clock';
import { mutationQueue, latestQuery } from '../domain/mutations';
import {
  errorMessage,
  type TaskDetail,
  type BoardSnapshot,
  type ProjectRecord,
  type ProjectColor,
  type TaskPatch,
  type TaskStatus,
  type SubtaskInput,
  type TagInput,
  type CreateTaskInput,
  type DeletionTarget,
  type StagedAttachment,
} from '../domain/types';
export type Editor =
  | { kind: 'new'; projectId: string }
  | { kind: 'detail'; taskId: string }
  | { kind: 'project'; project: ProjectRecord | null }
  | null;
export class Workspace {
  foundation = $state<Foundation | null>(null);
  projects = $state<ProjectRecord[]>([]);
  active = $state('day');
  view = $state<'board' | 'week' | 'notes'>('board');
  board = $state<BoardSnapshot | null>(null);
  detail = $state<TaskDetail | null>(null);
  editor = $state<Editor>(null);
  staged = $state<StagedAttachment[]>([]);
  loading = $state(true);
  boardLoading = $state(false);
  error = $state('');
  boardError = $state('');
  notice = $state('');
  nowUtc = $state(new Date().toISOString());
  timeZone = $state(Intl.DateTimeFormat().resolvedOptions().timeZone);
  private queue = mutationQueue();
  private boardQuery = latestQuery();
  private detailQuery = latestQuery();
  private refreshQuery = latestQuery();
  private revisions = new Map<string, number>();
  private clock = makeClock(false);
  async load() {
    this.loading = true;
    this.error = '';
    try {
      this.foundation = await openFoundation();
      this.clock = makeClock(this.foundation.runtime.seeded);
      this.tick();
      if (this.foundation.runtime.seeded && import.meta.env.DEV) {
        const fixtures = await import('../seed-attachments');
        await fixtures.installSeedAttachments();
      }
      this.projects = await commands.listProjects();
    } catch (e) {
      this.error = errorMessage(e);
    } finally {
      this.loading = false;
    }
  }
  tick() {
    const previous = dateAt(this.nowUtc, this.timeZone);
    this.nowUtc = this.clock.nowUtc();
    this.timeZone = this.clock.timeZone;
    if (this.foundation && previous !== dateAt(this.nowUtc, this.timeZone))
      void this.refresh().catch((e) => (this.notice = errorMessage(e)));
  }
  get selectedDate() {
    return dateAt(this.nowUtc, this.timeZone);
  }
  async select(active: string, view: 'board' | 'week' | 'notes' = 'board') {
    this.active = active;
    this.view = view;
    this.boardQuery.invalidate();
    this.board = null;
    this.boardError = '';
    if (this.projects.some((p) => p.id === active)) await this.loadBoard();
  }
  async loadBoard() {
    const id = this.active;
    if (!this.projects.some((p) => p.id === id)) return;
    const ticket = this.boardQuery.next();
    this.boardLoading = true;
    this.boardError = '';
    const bounds = dayBounds(this.selectedDate, this.timeZone);
    try {
      const reply = await commands.getBoard(id, bounds.start, bounds.end);
      if (this.boardQuery.current(ticket) && this.active === id) {
        this.board = reply;
        for (const task of reply.tasks)
          this.revisions.set(task.id, task.revision);
      }
    } catch (e) {
      if (this.boardQuery.current(ticket)) this.boardError = errorMessage(e);
    } finally {
      if (this.boardQuery.current(ticket)) this.boardLoading = false;
    }
  }
  async loadDetail(id: string) {
    const ticket = this.detailQuery.next();
    const reply = await commands.getTaskDetail(id);
    if (this.detailQuery.current(ticket)) {
      this.detail = reply;
      this.revisions.set(id, reply.task.revision);
    }
    return reply;
  }
  async refresh() {
    const ticket = this.refreshQuery.next();
    const foundation = await openFoundation();
    const projects = await commands.listProjects();
    if (!this.refreshQuery.current(ticket)) return;
    this.foundation = foundation;
    this.projects = projects;
    await this.loadBoard();
    if (
      this.refreshQuery.current(ticket) &&
      this.editor?.kind === 'detail' &&
      this.detail
    )
      await this.loadDetail(this.editor.taskId);
  }
  private publish(reply: unknown) {
    if (!reply || typeof reply !== 'object') return;
    if (Array.isArray(reply)) { this.projects = reply as ProjectRecord[]; return; }
    if ('task' in reply) {
      const detail = reply as TaskDetail;
      this.revisions.set(detail.task.id, detail.task.revision);
      if (
        this.editor?.kind === 'detail' &&
        this.editor.taskId === detail.task.id
      )
        this.detail = detail;
      if (this.board?.project.id === detail.task.project_id) {
        const task = {
          ...detail.task,
          tags: detail.tags,
          subtask_done: detail.subtasks.filter((s) => s.done).length,
          subtask_total: detail.subtasks.length,
        };
        this.board = {
          ...this.board,
          tasks: [...this.board.tasks.filter((t) => t.id !== task.id), task],
        };
      }
    } else if ('id' in reply && 'color' in reply) {
      const project = reply as ProjectRecord;
      this.projects = [
        ...this.projects.filter((p) => p.id !== project.id),
        project,
      ].sort((a, b) => a.sort_order - b.sort_order || a.id.localeCompare(b.id));
    }
  }

  private mutate<T>(operation: () => Promise<T>): Promise<T> {
    return this.queue.enqueue(async () => {
      let result: T;
      try {
        result = await operation();
      } catch (e) {
        if (
          e &&
          typeof e === 'object' &&
          'code' in e &&
          e.code === 'Conflict'
        ) {
          try {
            await this.refresh();
          } catch {
            /* The initiating editor retains the attempted write and original conflict. */
          }
        }
        throw e;
      }
      this.publish(result);
      try {
        await this.refresh();
      } catch (e) {
        this.notice = `Saved; refresh failed. ${errorMessage(e)}`;
      }
      return result;
    });
  }
  private revision(id: string) {
    const revision = this.revisions.get(id);
    if (revision === undefined)
      throw new Error('Reload this task before editing.');
    return revision;
  }
  createProject(input: { name: string; color: ProjectColor }) {
    return this.mutate(() => commands.createProject(input));
  }
  updateProject(id: string, input: { name: string; color: ProjectColor }) {
    return this.mutate(() => commands.updateProject(id, input));
  }
  moveProject(id: string, direction: 'left' | 'right') {
    return this.mutate(() => commands.moveProject(id, direction));
  }
  createTask(input: CreateTaskInput) {
    return this.mutate(async () => {
      const result = await commands.createTask(input);
      this.staged = this.staged.filter(
        (f) => !input.attachments.includes(f.token),
      );
      return result;
    });
  }
  patch(id: string, patch: TaskPatch) {
    return this.mutate(() => commands.updateTask(id, patch, this.revision(id)));
  }
  moveTask(id: string, status: TaskStatus, before: string | null) {
    return this.mutate(() =>
      commands.moveTask(id, status, before, this.revision(id)),
    );
  }
  setSubtasks(id: string, inputs: SubtaskInput[]) {
    return this.mutate(() =>
      commands.setSubtasks(id, inputs, this.revision(id)),
    );
  }
  setTags(id: string, inputs: TagInput[]) {
    return this.mutate(() =>
      commands.setTaskTags(id, inputs, this.revision(id)),
    );
  }
  setAlerts(id: string, offsets: number[]) {
    return this.mutate(() =>
      commands.setTaskAlerts(id, offsets, this.revision(id)),
    );
  }
  async stage(paths?: string[]) {
    const editor = this.editor;
    const files = await commands.stageAttachments(paths);
    if (this.editor !== editor) {
      await commands.discardStagedAttachments(files.map((f) => f.token));
      return [];
    }
    this.staged = [...this.staged, ...files];
    return files;
  }
  async discard(tokens: string[]) {
    const result = await commands.discardStagedAttachments(tokens);
    this.staged = this.staged.filter((f) => !tokens.includes(f.token));
    if (result.cleanupPending)
      this.notice = 'Attachment cleanup is queued and will retry.';
    return result;
  }
  attach(id: string, tokens: string[]) {
    return this.mutate(async () => {
      const reply = await commands.addAttachments(
        id,
        tokens,
        this.revision(id),
      );
      this.staged = this.staged.filter((f) => !tokens.includes(f.token));
      return reply;
    });
  }
  removeAttachment(id: string) {
    return this.mutate(async () => {
      if (this.editor?.kind !== 'detail')
        throw new Error('Open the task before removing an attachment.');
      const reply = await commands.removeAttachment(
        id,
        this.revision(this.editor.taskId),
      );
      this.publish(reply.detail);
      return reply;
    });
  }
  delete(target: DeletionTarget, fingerprint: string) {
    return this.mutate(async () => {
      const order = this.projects.map((p) => p.id),
        at = order.indexOf(target.id);
      const result = await commands.deleteEntity(target, fingerprint);
      if (target.kind === 'project')
        this.projects = this.projects.filter((p) => p.id !== target.id);
      if (target.kind === 'task' && this.board)
        this.board = {
          ...this.board,
          tasks: this.board.tasks.filter((t) => t.id !== target.id),
        };
      if (target.kind === 'project' && this.active === target.id) {
        this.active = order[at + 1] ?? order[at - 1] ?? 'day';
        this.board = null;
      }
      if (
        target.kind === 'task' &&
        this.editor?.kind === 'detail' &&
        this.editor.taskId === target.id
      ) {
        this.detailQuery.invalidate();
        this.detail = null;
      }
      if (result.cleanupPending)
        this.notice = 'Deleted. Attachment cleanup remains queued.';
      return result;
    });
  }
  settled() {
    return this.queue.settled();
  }
}
