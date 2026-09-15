import { expect, it } from 'vitest';
import { parsePlannerTime } from '../../src/lib/domain/planner-time';
import { minuteLabel } from '../../src/lib/domain/planner-rules';

it('understands AM/PM, business-hour defaults, and explicit 24-hour times', () => {
  const cases: [string, number][] = [
    ['2pm', 840], ['2 PM', 840], [' 2:30 pM ', 870], ['2:30', 870],
    ['2', 840], ['6:45', 1125], ['7', 420], ['11:30', 690],
    ['12', 720], ['12am', 0], ['12:30 AM', 30], ['12pm', 720],
    ['2:30am', 150], ['02:30', 150], ['00:30', 30], ['14:30', 870],
    ['23:45', 1425], ['24:00', 1440],
  ];
  for (const [text, minutes] of cases) expect(parsePlannerTime(text), text).toBe(minutes);
});

it('preserves every existing quarter-hour value through display and parse', () => {
  for (let minute = 0; minute <= 1440; minute += 15)
    expect(parsePlannerTime(minuteLabel(minute))).toBe(minute);
});

it('rejects incomplete input, invalid clocks, and conflicting suffixes', () => {
  for (const text of ['', ' ', '2:', '2:3', '2:60', '13:90', '25:00', '24:15',
    '0pm', '13pm', '24am', '-2', '2.5', '2:30:00', '2pm tomorrow', 'pm', '2am pm'])
    expect(parsePlannerTime(text), text).toBeNaN();
});
