import { Temporal } from '@js-temporal/polyfill';
export const fixtureInstant = '2025-09-11T17:42:00.000Z';
export const fixtureZone = 'America/New_York';
export function makeClock({
  offsetMs = 0,
  timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone,
  read = () => new Date(),
}: { offsetMs?: number; timeZone?: string; read?: () => Date } = {}) {
  return {
    timeZone,
    nowUtc: () => new Date(read().getTime() + offsetMs).toISOString(),
  };
}
export function dateAt(instant: string, zone: string) {
  return Temporal.Instant.from(instant)
    .toZonedDateTimeISO(zone)
    .toPlainDate()
    .toString();
}
export function dayBounds(date: string, zone: string) {
  const start = Temporal.PlainDate.from(date).toZonedDateTime(zone);
  return {
    start: start.toInstant().toString({ smallestUnit: 'millisecond' }),
    end: start
      .add({ days: 1 })
      .toInstant()
      .toString({ smallestUnit: 'millisecond' }),
  };
}
