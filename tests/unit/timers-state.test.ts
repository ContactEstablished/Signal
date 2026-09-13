import { it, expect, vi, beforeEach } from 'vitest';
import { TimerState } from '../../src/lib/state/timers.svelte';
import type {
  TimerWriteResult,
  TimerSession,
} from '../../src/lib/domain/timers';
const api = vi.hoisted(() => ({
  startTimer: vi.fn(),
  pauseTimer: vi.fn(),
  resumeTimer: vi.fn(),
  stopTimer: vi.fn(),
  logTime: vi.fn(),
  getTaskTime: vi.fn(),
}));
vi.mock('../../src/lib/native/timers', () => api);
beforeEach(() => vi.resetAllMocks());
const session: TimerSession = {
  id: 's',
  task_id: 'a',
  block_id: null,
  state: 'running',
  started_at: '2025-09-11T00:00:00.000Z',
  segment_started_at: '2025-09-11T00:00:00.000Z',
  accumulated_ms: 0,
  ended_at: null,
  entry_id: null,
  revision: 0,
};
const reply = () =>
  ({
    snapshot: {
      now_utc: '2025-09-11T01:00:00.000Z',
      offset_ms: 0,
      sessions: [],
    },
    entries: [],
    detail: { task: { id: 'a', hours_worked: 1, revision: 1 } },
    outcome: { session_id: 's', entry_id: 'e' },
  }) as unknown as TimerWriteResult;
it('retries uncertain Stop with original identity and blocks replacement writes', async () => {
  const state = new TimerState();
  state.sessions = [session];
  api.stopTimer
    .mockRejectedValueOnce(new Error('transport disconnected'))
    .mockResolvedValue(reply());
  await expect(state.execute('a', 'stop')).rejects.toThrow();
  const first = api.stopTimer.mock.calls[0][0];
  expect(state.recovery.a).toBe(true);
  expect(() => state.assertWritable('a')).toThrow();
  await expect(state.execute('a', 'start')).rejects.toThrow();
  await state.execute('a', 'stop', undefined, true);
  expect(api.stopTimer.mock.calls[1][0]).toEqual(first);
  expect(state.recovery.a).toBe(false);
  expect(state.sessions).toEqual([]);
});
it('acknowledges Log once on exact-payload recovery and unlocks known failures', async () => {
  const state = new TimerState(),
    input = { startedAt: '2025-09-11T00:00:00.000Z', durationMs: 1000 };
  api.logTime
    .mockRejectedValueOnce(new Error('lost reply'))
    .mockResolvedValue(reply());
  await expect(state.execute('a', 'log', input)).rejects.toThrow();
  await state.execute('a', 'log', input);
  expect(api.logTime.mock.calls[0][0].requestId).toBe(
    api.logTime.mock.calls[1][0].requestId,
  );
  expect(state.completions.a).toBe(1);
  api.logTime.mockRejectedValueOnce({
    code: 'Validation',
    message: 'Future',
  });
  await expect(state.execute('a', 'log', input)).rejects.toMatchObject({
    code: 'Validation',
  });
  expect(state.recovery.a).toBe(false);
  expect(() => state.assertWritable('a')).not.toThrow();
});
it('does not publish a stale read after a committed transition', async () => {
  const state = new TimerState();
  let resolve!: (v: unknown) => void;
  api.getTaskTime.mockReturnValue(new Promise((r) => (resolve = r)));
  const read = state.load('a');
  state.publish({ ...reply().snapshot, sessions: [session] });
  resolve({ snapshot: { ...reply().snapshot, sessions: [] }, entries: [] });
  await read;
  expect(state.sessions).toEqual([session]);
  expect(state.loading.a).toBe(false);
});
it('counts only running sessions across tasks and preserves known data after read failure', async () => {
  const state = new TimerState();
  state.sessions = [
    session,
    {
      ...session,
      id: 'p',
      task_id: 'b',
      state: 'paused',
      segment_started_at: null,
    },
  ];
  expect(state.running).toBe(1);
  api.getTaskTime.mockRejectedValue(new Error('read failed'));
  await state.load('a');
  expect(state.session('a')).toEqual(session);
  expect(state.errors.a).toBe('read failed');
});
