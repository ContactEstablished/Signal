import * as api from '../native/agenda';
import { errorMessage } from '../domain/types';
import type {
  AgendaInput,
  AgendaQuery,
  MeetingChange,
  MeetingDetail,
  MeetingRef,
  MeetingWriteResult,
  TaskMeetingPage,
  TodaySnapshot,
  WeekSnapshot,
} from '../domain/agenda';
export class AgendaState {
  today = $state<TodaySnapshot | null>(null);
  week = $state<WeekSnapshot | null>(null);
  detail = $state<MeetingDetail | null>(null);
  linked = $state<TaskMeetingPage | null>(null);
  linkedDate = $state('');
  linkedTaskId = $state('');
  weekDate = $state('');
  followWeek = $state(true);
  loading = $state(false);
  pending = $state(false);
  recovery = $state(false);
  draftOpen = $state(false);
  error = $state('');
  detailError = $state('');
  linkedError = $state('');
  private epoch = 0;
  private viewTicket = 0;
  private reads: Record<string, number> = {};
  private intent: AgendaInput | null = null;
  get guarded() {
    return this.pending || this.recovery || this.draftOpen;
  }
  invalidate() {
    this.epoch++;
    this.viewTicket++;
    this.loading = false;
  }
  invalidateViews() {
    this.invalidate();
    this.today = null;
    this.week = null;
    this.linked = null;
  }
  assertWritable() {
    if (this.recovery)
      throw new Error(
        'Resolve the pending agenda action with Retry before making another change.',
      );
  }
  private async read<T>(
    key: string,
    operation: () => Promise<T>,
    accept: (r: T) => void,
    error: (s: string) => void,
  ) {
    const ticket = (this.reads[key] ?? 0) + 1;
    this.reads[key] = ticket;
    const epoch = this.epoch;
    try {
      const r = await operation();
      if (this.reads[key] === ticket && this.epoch === epoch) {
        accept(r);
        error('');
      }
    } catch (e) {
      if (this.reads[key] === ticket && this.epoch === epoch)
        error(errorMessage(e));
    }
  }
  async loadToday(q: AgendaQuery) {
    const ticket = ++this.viewTicket;
    this.loading = true;
    await this.read(
      'today',
      () => api.getToday(q),
      (r) => (this.today = r),
      (s) => {
        if (!this.recovery) this.error = s;
      },
    );
    if (ticket === this.viewTicket) this.loading = false;
  }
  async loadWeek(q: AgendaQuery) {
    if (
      this.week &&
      (this.week.query.date !== q.date ||
        this.week.query.timeZone !== q.timeZone ||
        this.week.query.projectId !== q.projectId)
    )
      this.week = null;
    const ticket = ++this.viewTicket;
    this.loading = true;
    await this.read(
      'week',
      () => api.getWeek(q),
      (r) => (this.week = r),
      (s) => {
        if (!this.recovery) this.error = s;
      },
    );
    if (ticket === this.viewTicket) this.loading = false;
  }
  async loadDetail(ref: MeetingRef) {
    this.detail = null;
    this.detailError = '';
    await this.read(
      'detail',
      () => api.getMeetingDetail(ref),
      (r) => (this.detail = r),
      (s) => (this.detailError = s),
    );
  }
  async loadLinked(id: string, q: AgendaQuery) {
    this.linkedDate = q.date;
    this.linkedTaskId = id;
    this.linked = null;
    await this.read(
      'linked',
      () => api.getTaskMeetings(id, q),
      (r) => (this.linked = r),
      (s) => (this.linkedError = s),
    );
  }
  reserve(change: MeetingChange, expectedFingerprint?: string) {
    this.assertWritable();
    if (this.pending) throw new Error('An agenda action is already pending.');
    this.intent = JSON.parse(
      JSON.stringify({
        requestId: crypto.randomUUID(),
        change,
        expectedFingerprint,
      }),
    );
    this.pending = true;
    this.error = '';
  }
  cancelReserved(e: unknown) {
    if (!this.recovery) {
      this.pending = false;
      this.intent = null;
      this.error = errorMessage(e);
    }
  }
  publish(r: MeetingWriteResult) {
    this.invalidateViews();
    this.detail = r.detail;
  }
  async execute(): Promise<MeetingWriteResult> {
    if (!this.intent) throw new Error('No agenda operation to retry.');
    this.pending = true;
    try {
      const r = await api.applyMeeting(this.intent);
      this.publish(r);
      this.intent = null;
      this.recovery = false;
      this.error = '';
      return r;
    } catch (e) {
      const code =
        e && typeof e === 'object' && 'code' in e ? String(e.code) : '';
      // A failed receipt lookup cannot prove the original uncertain write
      // rolled back. Keep its identity until native replay can resolve it.
      this.recovery =
        (this.recovery && code === 'Database') ||
        !['Validation', 'Conflict', 'NotFound', 'Database'].includes(code);
      if (!this.recovery) this.intent = null;
      this.error = this.recovery
        ? 'The result is unknown. Retry this exact action safely.'
        : errorMessage(e);
      throw e;
    } finally {
      this.pending = false;
    }
  }
}
