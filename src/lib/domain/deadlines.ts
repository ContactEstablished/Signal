import { Temporal } from '@js-temporal/polyfill';
export interface DeadlineChoice {
  offset: string;
  instant: string;
}
export function deadlineChoices(
  date: string,
  time: string,
  timeZone: string,
): DeadlineChoice[] {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || !/^\d{2}:\d{2}$/.test(time))
    throw new Error('Enter a complete date and time.');
  const wall = Temporal.PlainDateTime.from(`${date}T${time}`, {
    overflow: 'reject',
  });
  const fields = {
    year: wall.year,
    month: wall.month,
    day: wall.day,
    hour: wall.hour,
    minute: wall.minute,
    timeZone,
  };
  try {
    const z = Temporal.ZonedDateTime.from(fields, {
      disambiguation: 'reject',
      overflow: 'reject',
    });
    return [
      {
        offset: z.offset,
        instant: z.toInstant().toString({ smallestUnit: 'millisecond' }),
      },
    ];
  } catch {
    const candidates = (['earlier', 'later'] as const)
      .map((disambiguation) =>
        Temporal.ZonedDateTime.from(fields, {
          disambiguation,
          overflow: 'reject',
        }),
      )
      .filter((z) => z.toPlainDateTime().equals(wall));
    if (!candidates.length)
      throw new Error(
        'This local time does not exist because the clocks move forward. Choose another time.',
      );
    return candidates.map((z) => ({
      offset: z.offset,
      instant: z.toInstant().toString({ smallestUnit: 'millisecond' }),
    }));
  }
}
export function deadlineToUtc(
  date: string,
  time: string,
  zone: string,
  offset?: string,
): string | null {
  if (!date && !time) return null;
  const choices = deadlineChoices(date, time, zone);
  if (choices.length === 1) return choices[0].instant;
  const choice = choices.find((c) => c.offset === offset);
  if (!choice)
    throw new Error('This time occurs twice. Choose its UTC offset.');
  return choice.instant;
}
export function deadlineFields(instant: string | null, timeZone: string) {
  if (!instant) return { date: '', time: '', offset: '' };
  const z = Temporal.Instant.from(instant).toZonedDateTimeISO(timeZone);
  return {
    date: z.toPlainDate().toString(),
    time: z.toPlainTime().toString({ smallestUnit: 'minute' }),
    offset: z.offset,
  };
}
