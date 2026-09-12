import { it, expect } from 'vitest';
import {
  makeClock,
  dayBounds,
  fixtureInstant,
} from '../../src/lib/domain/clock';
it('freezes fixtures and uses calendar midnights across DST', () => {
  expect(makeClock(true, () => new Date('2030-01-01')).nowUtc()).toBe(
    fixtureInstant,
  );
  for (const [date, hours] of [
    ['2025-03-09', 23],
    ['2025-11-02', 25],
  ] as const) {
    const bounds = dayBounds(date, 'America/New_York');
    expect((Date.parse(bounds.end) - Date.parse(bounds.start)) / 3600000).toBe(
      hours,
    );
  }
  expect(makeClock(false, () => new Date('2030-01-01'), 'UTC').nowUtc()).toBe(
    '2030-01-01T00:00:00.000Z',
  );
});
