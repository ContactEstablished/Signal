import { it, expect, vi, beforeEach } from 'vitest';
import { AgendaState } from '../../src/lib/state/agenda.svelte';
import type {
  MeetingChange,
  MeetingWriteResult,
  TodaySnapshot,
} from '../../src/lib/domain/agenda';
const api = vi.hoisted(() => ({
  getToday: vi.fn(),
  getWeek: vi.fn(),
  getMeetingDetail: vi.fn(),
  getTaskMeetings: vi.fn(),
  applyMeeting: vi.fn(),
}));
vi.mock('../../src/lib/native/agenda', () => api);
const change: MeetingChange = {
  action: 'move_due',
  payload: {
    taskId: 't',
    dueAt: '2025-09-12T13:00:12.345Z',
    expectedRevision: 3,
  },
};
const reply: MeetingWriteResult = {
  detail: null,
  outcome: { ref: null, removed: false, task_id: 't' },
  changed_details: [],
  affected_project_ids: [],
  affected_task_ids: ['t'],
  revision: null,
  replayed: true,
};
beforeEach(() => vi.resetAllMocks());
it('retains exact uncertain writes and blocks replacement payloads until receipt replay', async () => {
  const state = new AgendaState();
  state.reserve(change);
  api.applyMeeting.mockRejectedValueOnce({
    code: 'UnknownOutcome',
    message: 'Commit uncertain',
  });
  await expect(state.execute()).rejects.toMatchObject({
    code: 'UnknownOutcome',
  });
  const original = api.applyMeeting.mock.calls[0][0];
  expect(state.recovery).toBe(true);
  expect(() => state.reserve(change)).toThrow('Retry');
  state.weekDate = '2027-01-01';
  api.applyMeeting.mockResolvedValue(reply);
  await state.execute();
  expect(api.applyMeeting.mock.calls[1][0]).toEqual(original);
  expect(state.recovery).toBe(false);
});
it('rejects late reads after a new date query or a confirmed cancellation', async () => {
  const s = new AgendaState();
  let resolve!: (v: TodaySnapshot) => void;
  api.getToday.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
  const old = s.loadToday({ date: '2025-09-11', timeZone: 'UTC' });
  const newer = { today: '2025-09-12' } as TodaySnapshot;
  api.getToday.mockResolvedValue(newer);
  await s.loadToday({ date: '2025-09-12', timeZone: 'UTC' });
  resolve({ today: '2025-09-11' } as TodaySnapshot);
  await old;
  expect(s.today?.today).toBe('2025-09-12');
  api.getToday.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
  const stale = s.loadToday({ date: '2025-09-12', timeZone: 'UTC' });
  s.publish({
    ...reply,
    outcome: { ref: { meetingId: 'm', occurrenceKey: 'o:0' }, removed: true },
  });
  resolve(newer);
  await stale;
  expect(s.today).toBeNull();
});
it('known rollback unlocks a corrected draft with a new identity', async () => {
  const s = new AgendaState();
  s.reserve(change);
  api.applyMeeting.mockRejectedValueOnce({
    code: 'Conflict',
    message: 'Changed',
  });
  await expect(s.execute()).rejects.toMatchObject({ code: 'Conflict' });
  const id = api.applyMeeting.mock.calls[0][0].requestId;
  s.reserve(change);
  api.applyMeeting.mockResolvedValue(reply);
  await s.execute();
  expect(api.applyMeeting.mock.calls[1][0].requestId).not.toBe(id);
});
it('keeps an uncertain original request when receipt recovery hits a database failure', async () => {
  const s = new AgendaState();
  s.reserve(change);
  api.applyMeeting.mockRejectedValueOnce({ code: 'UnknownOutcome' });
  await expect(s.execute()).rejects.toBeDefined();
  const original = api.applyMeeting.mock.calls[0][0];
  api.applyMeeting.mockRejectedValueOnce({
    code: 'Database',
    message: 'Receipt read unavailable',
  });
  await expect(s.execute()).rejects.toMatchObject({ code: 'Database' });
  expect(s.recovery).toBe(true);
  expect(() => s.reserve(change)).toThrow('Retry');
  api.applyMeeting.mockResolvedValue(reply);
  await s.execute();
  expect(api.applyMeeting.mock.calls[2][0]).toEqual(original);
  expect(s.recovery).toBe(false);
});

it('read refresh cannot erase the exact retry explanation', async () => {
  const s = new AgendaState();
  s.reserve(change);
  api.applyMeeting.mockRejectedValueOnce({ code: 'UnknownOutcome' });
  await expect(s.execute()).rejects.toBeDefined();
  api.getToday.mockResolvedValue({ today: '2025-09-12' });
  await s.loadToday({ date: '2025-09-12', timeZone: 'UTC' });
  expect(s.error).toContain('Retry this exact action');
  expect(s.guarded).toBe(true);
});
it('an older response cannot clear a newer loading indicator', async () => {
  const s = new AgendaState();
  let first!: (v: unknown) => void, second!: (v: unknown) => void;
  api.getToday
    .mockImplementationOnce(() => new Promise((r) => (first = r)))
    .mockImplementationOnce(() => new Promise((r) => (second = r)));
  const a = s.loadToday({ date: '2025-09-11', timeZone: 'UTC' });
  const b = s.loadToday({ date: '2025-09-12', timeZone: 'UTC' });
  first({ today: '2025-09-11' });
  await a;
  expect(s.loading).toBe(true);
  second({ today: '2025-09-12' });
  await b;
  expect(s.loading).toBe(false);
});
