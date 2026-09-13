import * as api from '../native/timers';
import { errorMessage } from '../domain/types';
import type {
  TimerSnapshot,
  TimeEntry,
  TimerAction,
  StartInput,
  SessionInput,
  LogInput,
  TimerWriteResult,
} from '../domain/timers';
type Intent = {
  action: TimerAction;
  input: StartInput | SessionInput | LogInput;
};
export class TimerState {
  sessions = $state<TimerSnapshot['sessions']>([]);
  entries = $state<Record<string, TimeEntry[]>>({});
  errors = $state<Record<string, string>>({});
  loading = $state<Record<string, boolean>>({});
  pending = $state<Record<string, boolean>>({});
  recovery = $state<Record<string, boolean>>({});
  completions = $state<Record<string, number>>({});
  private intents = new Map<string, Intent>();
  private confirmed = new Set<string>();
  private reads = new Map<string, number>();
  private epoch = 0;
  private readSerial = 0;
  get running() {
    return this.sessions.filter((s) => s.state === 'running').length;
  }
  session(id: string) {
    return this.sessions.find((s) => s.task_id === id) ?? null;
  }
  version() {
    return this.epoch;
  }
  publish(snapshot: TimerSnapshot) {
    this.epoch++;
    this.sessions = snapshot.sessions;
  }
  accept(snapshot: TimerSnapshot, epoch: number) {
    if (epoch === this.epoch) this.publish(snapshot);
  }
  async load(id: string) {
    const serial = ++this.readSerial;
    this.reads.set(id, serial);
    const epoch = this.epoch;
    this.loading[id] = true;
    try {
      const r = await api.getTaskTime(id);
      if (this.reads.get(id) !== serial || epoch !== this.epoch) return;
      this.sessions = r.snapshot.sessions;
      this.entries[id] = r.entries;
      if (!this.recovery[id]) this.errors[id] = '';
    } catch (e) {
      if (this.reads.get(id) === serial) this.errors[id] = errorMessage(e);
    } finally {
      if (this.reads.get(id) === serial) this.loading[id] = false;
    }
  }
  assertWritable(id: string) {
    if (this.recovery[id])
      throw new Error(
        'Resolve the pending time operation with Retry before editing this task.',
      );
  }
  async execute(
    id: string,
    action: TimerAction,
    log?: { startedAt: string; durationMs: number },
    retry = false,
    blockId?: string,
  ): Promise<TimerWriteResult> {
    let intent = this.intents.get(id);
    if (this.recovery[id] && !retry) {
      if (
        !(
          action === 'log' &&
          intent?.action === 'log' &&
          JSON.stringify(log) ===
            JSON.stringify({
              startedAt: (intent.input as LogInput).startedAt,
              durationMs: (intent.input as LogInput).durationMs,
            })
        )
      )
        this.assertWritable(id);
    }
    if (!intent) {
      const input: StartInput = {
        requestId: crypto.randomUUID(),
        taskId: id,
        ...(action === 'start' && blockId ? { blockId } : {}),
      };
      if (action === 'log') {
        if (!log) throw new Error('Enter time to log.');
        intent = { action, input: { ...input, ...log } };
      } else if (action === 'start') intent = { action, input };
      else {
        const session = this.session(id);
        if (!session) throw new Error('Reload this task’s timer.');
        intent = {
          action,
          input: {
            ...input,
            sessionId: session.id,
            expectedRevision: session.revision,
          },
        };
      }
      this.intents.set(id, intent);
    }
    this.pending[id] = true;
    this.errors[id] = '';
    try {
      const r = await (intent.action === 'start'
        ? api.startTimer(intent.input)
        : intent.action === 'log'
          ? api.logTime(intent.input as LogInput)
          : intent.action === 'pause'
            ? api.pauseTimer(intent.input as SessionInput)
            : intent.action === 'resume'
              ? api.resumeTimer(intent.input as SessionInput)
              : api.stopTimer(intent.input as SessionInput));
      this.publish(r.snapshot);
      this.entries[id] = r.entries;
      if (
        intent.action === 'log' &&
        !this.confirmed.has(intent.input.requestId)
      ) {
        this.confirmed.add(intent.input.requestId);
        this.completions[id] = (this.completions[id] ?? 0) + 1;
      }
      this.intents.delete(id);
      this.recovery[id] = false;
      return r;
    } catch (e) {
      const known =
        !!e &&
        typeof e === 'object' &&
        'code' in e &&
        ['Validation', 'NotFound', 'Conflict', 'Database'].includes(
          String(e.code),
        );
      this.recovery[id] = !known;
      if (known) this.intents.delete(id);
      this.errors[id] = known
        ? errorMessage(e)
        : 'The result is unknown. Retry to check this exact operation safely.';
      throw e;
    } finally {
      this.pending[id] = false;
    }
  }
  retryAction(id: string) {
    return this.intents.get(id)?.action;
  }
  clear(id: string) {
    this.reads.delete(id);
    delete this.entries[id];
    delete this.errors[id];
    this.sessions = this.sessions.filter((s) => s.task_id !== id);
    this.epoch++;
  }
}
