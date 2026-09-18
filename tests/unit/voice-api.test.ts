import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { suggest, emptyDraft } from '../../src/lib/voice/api';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());

it('recovers from an unresponsive desktop bridge without changing the draft', async () => {
  const draft = emptyDraft('2026-09-18T16:00:00Z', 'America/New_York');
  draft.transcript = 'Write release notes';
  let finish!: (value: typeof draft) => void;
  invoke.mockImplementation((command) => command === 'suggest_voice_tasks'
    ? new Promise((resolve) => { finish = resolve; })
    : Promise.resolve());
  const result = suggest(draft);
  const assertion = expect(result).rejects.toThrow('Task suggestions timed out. Your transcript is saved.');
  await vi.advanceTimersByTimeAsync(100_000);
  await assertion;
  expect(invoke).toHaveBeenCalledWith('cancel_voice');
  finish({ ...draft, transcript: 'Late reply' });
  await Promise.resolve();
  expect(draft.transcript).toBe('Write release notes');
  expect(vi.getTimerCount()).toBe(0);
});

it.each([true, false])('clears its watchdog after a native success/failure (%s)', async (success) => {
  const draft = emptyDraft('2026-09-18T16:00:00Z', 'America/New_York');
  if (success) invoke.mockResolvedValue(draft);
  else invoke.mockRejectedValue(new Error('Provider unavailable'));
  if (success) expect(await suggest(draft)).toEqual(draft);
  else await expect(suggest(draft)).rejects.toThrow('Provider unavailable');
  expect(vi.getTimerCount()).toBe(0);
  await vi.advanceTimersByTimeAsync(100_000);
  expect(invoke).toHaveBeenCalledTimes(1);
});
