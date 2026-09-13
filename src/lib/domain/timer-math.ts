import type { TimerSession } from './timers';
export function timerElapsedMs(
  session: TimerSession,
  nowUtc: string,
): number {
  return (
    session.accumulated_ms +
    (session.state === 'running' && session.segment_started_at
      ? Math.max(
          0,
          Date.parse(nowUtc) - Date.parse(session.segment_started_at),
        )
      : 0)
  );
}
export function formatElapsedMs(ms: number): string {
  const s = Math.floor(Math.max(0, ms) / 1000);
  return [Math.floor(s / 3600), Math.floor((s % 3600) / 60), s % 60]
    .map((v) => String(v).padStart(2, '0'))
    .join(':');
}
