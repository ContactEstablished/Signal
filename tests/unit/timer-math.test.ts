import { it, expect } from 'vitest';
import {
  timerElapsedMs,
  formatElapsedMs,
} from '../../src/lib/domain/timer-math';
import type { TimerSession } from '../../src/lib/domain/timers';
const session: TimerSession = {
  id: 's',
  task_id: 'a',
  block_id: null,
  state: 'running',
  started_at: '2025-09-11T10:00:00.000Z',
  segment_started_at: '2025-09-11T11:00:00.000Z',
  accumulated_ms: 1500,
  ended_at: null,
  revision: 2,
  entry_id: null,
};
it('derives active segments, clamps clock rollback and excludes paused spans', () => {
  expect(timerElapsedMs(session, '2025-09-11T11:00:02.500Z')).toBe(4000);
  expect(timerElapsedMs(session, '2025-09-10T11:00:00Z')).toBe(1500);
  expect(
    timerElapsedMs(
      { ...session, state: 'paused', segment_started_at: null },
      '2030-01-01',
    ),
  ).toBe(1500);
});
it('floors only display seconds and never wraps hours at midnight', () => {
  expect(formatElapsedMs(1999)).toBe('00:00:01');
  expect(formatElapsedMs(360001999)).toBe('100:00:01');
  expect(formatElapsedMs(-10)).toBe('00:00:00');
});
