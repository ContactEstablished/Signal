import { expect, it } from 'vitest';
import { minuteLabel, wallTimeLabel, localDateTimeLabel, parseWallTime } from '../../src/lib/domain/time-display';
import { parsePlannerTime } from '../../src/lib/domain/planner-time';
import { endpointChoices } from '../../src/lib/domain/planner-rules';

it('displays midnight, noon, afternoon and next-day midnight without changing planner values', () => {
  for (const [minute, label] of [[0, '12:00 AM'], [60, '1:00 AM'], [720, '12:00 PM'], [780, '1:00 PM'], [870, '2:30 PM'], [1440, '12:00 AM (next day)']] as const) {
    expect(minuteLabel(minute)).toBe(label);
    expect(parsePlannerTime(label)).toBe(minute);
  }
  expect(endpointChoices('2026-09-14', 1440, 'America/New_York')[0].instant).toBe('2026-09-15T04:00:00.000Z');
  expect(endpointChoices('2026-09-14', 840, 'America/New_York')[0].instant).toBe('2026-09-14T18:00:00.000Z');
});

it('keeps precise existing wall times while formatting their hour', () => {
  expect(wallTimeLabel('17:00:00.000')).toBe('5:00 PM');
  expect(localDateTimeLabel('2026-09-14T15:30:12.345')).toBe('2026-09-14 3:30:12.345 PM');
  expect(parseWallTime('3:30:12.345 PM')).toBe('15:30:12.345');
  expect(parseWallTime('12:00 AM')).toBe('00:00');
  expect(parseWallTime('12:00 PM')).toBe('12:00');
  expect(parseWallTime('2:30')).toBe('14:30');
  expect(parseWallTime('2:90 PM')).toBeNull();
  expect(parseWallTime('24:00')).toBeNull();
});
