import { it, expect } from 'vitest';
import {
  makeClock,
  dayBounds,
  fixtureInstant,
} from '../../src/lib/domain/clock';
it('advances an injected offset clock and uses calendar midnights across DST', () => {
  let real = Date.parse('2026-01-01T00:00:00Z');
  const offset = Date.parse(fixtureInstant) - real;
  const clock = makeClock({
    offsetMs: offset,
    timeZone: 'America/New_York',
    read: () => new Date(real),
  });
  expect(clock.nowUtc()).toBe(fixtureInstant);
  real += 2500;
  expect(clock.nowUtc()).toBe('2025-09-11T17:42:02.500Z');
  for (const [date, hours] of [
    ['2025-03-09', 23],
    ['2025-11-02', 25],
  ] as const) {
    const bounds = dayBounds(date, 'America/New_York');
    expect(
      (Date.parse(bounds.end) - Date.parse(bounds.start)) / 3600000,
    ).toBe(hours);
  }
  expect(
    makeClock({
      read: () => new Date('2030-01-01'),
      timeZone: 'UTC',
    }).nowUtc(),
  ).toBe('2030-01-01T00:00:00.000Z');
});
