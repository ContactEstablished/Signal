import { it, expect } from 'vitest';
import {
  deadlineChoices,
  deadlineToUtc,
  deadlineFields,
} from '../../src/lib/domain/deadlines';
const zone = 'America/New_York';
it('requires explicit fall-back offsets and rejects spring gaps or invalid dates', () => {
  expect(deadlineToUtc('2025-09-11', '17:00', zone)).toBe(
    '2025-09-11T21:00:00.000Z',
  );
  expect(() => deadlineChoices('2025-03-09', '02:30', zone)).toThrow(
    /does not exist/,
  );
  expect(
    deadlineChoices('2025-11-02', '01:30', zone).map((c) => c.offset),
  ).toEqual(['-04:00', '-05:00']);
  expect(() => deadlineToUtc('2025-11-02', '01:30', zone)).toThrow(/twice/);
  expect(deadlineToUtc('2025-11-02', '01:30', zone, '-05:00')).toBe(
    '2025-11-02T06:30:00.000Z',
  );
  expect(deadlineFields('2025-11-02T06:30:00.000Z', zone).offset).toBe(
    '-05:00',
  );
  expect(() => deadlineChoices('2025-02-29', '17:00', zone)).toThrow();
});
