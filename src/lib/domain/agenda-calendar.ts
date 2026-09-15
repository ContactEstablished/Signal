import { Temporal } from '@js-temporal/polyfill';
import type { DueDraft, SeriesDraft } from './agenda';
export const meetingTimeZones = [
  { label: 'Eastern', value: 'America/New_York' },
  { label: 'Central', value: 'America/Chicago' },
  { label: 'Mountain', value: 'America/Denver' },
  { label: 'Pacific', value: 'America/Los_Angeles' },
];
export const meetingWeekday = (date: string) => Temporal.PlainDate.from(date).dayOfWeek;
export function weeklyStartDate(date: string, weekdays: number[]): string {
  const start = Temporal.PlainDate.from(date);
  for (let offset = 0; offset < 7; offset++) {
    const candidate = start.add({ days: offset });
    if (weekdays.includes(candidate.dayOfWeek)) return candidate.toString();
  }
  throw new Error('Choose at least one repeat weekday.');
}
export const addDays = (date: string, n: number) =>
  Temporal.PlainDate.from(date).add({ days: n }).toString();
export const mondayOf = (date: string) => {
  const d = Temporal.PlainDate.from(date);
  return d.subtract({ days: d.dayOfWeek - 1 }).toString();
};
export function timeChoices(date: string, time: string, zone: string) {
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(date) ||
    !/^\d{2}:\d{2}(:\d{2}(\.\d{1,3})?)?$/.test(time)
  )
    throw new Error('Enter a complete date and time.');
  const wall = Temporal.PlainDateTime.from(`${date}T${time}`, {
    overflow: 'reject',
  });
  const values = (['earlier', 'later'] as const)
    .map((disambiguation) => wall.toZonedDateTime(zone, { disambiguation }))
    .filter((v) => v.toPlainDateTime().equals(wall));
  return values
    .filter((v, i) => !i || v.epochMilliseconds !== values[0].epochMilliseconds)
    .map((v) => ({
      offset: v.offset,
      instant: v.toInstant().toString({ smallestUnit: 'millisecond' }),
    }));
}
export function resolveTaskDate(d: DueDraft, zone: string): string | null {
  if (!d.date && !d.time) return null;
  const choices = timeChoices(d.date, d.time, zone);
  if (!choices.length)
    throw new Error(
      'This time does not exist because the clocks move forward. Choose another time.',
    );
  if (choices.length === 1) return choices[0].instant;
  const c = choices.find((c) => c.offset === d.offset);
  if (!c) throw new Error('This time occurs twice. Choose its UTC offset.');
  return c.instant;
}
export function dueFields(instant: string, zone: string): DueDraft {
  const z = Temporal.Instant.from(instant).toZonedDateTimeISO(zone);
  return {
    date: z.toPlainDate().toString(),
    time: z.toPlainTime().toString({ smallestUnit: 'millisecond' }),
    offset: z.offset,
  };
}
export function proposeTaskDate(
  task: { due_at: string | null },
  targetDate: string | null,
  zone: string,
):
  | { kind: 'ready'; due_at: string | null }
  | ({
      kind: 'confirm';
      choices: { offset: string; instant: string }[];
      reason: 'no-deadline' | 'gap' | 'fold';
    } & DueDraft) {
  if (targetDate === null) return { kind: 'ready', due_at: null };
  Temporal.PlainDate.from(targetDate);
  if (!task.due_at)
    return {
      kind: 'confirm',
      date: targetDate,
      time: '17:00:00.000',
      choices: [],
      reason: 'no-deadline',
    };
  const original = dueFields(task.due_at, zone);
  if (original.date === targetDate)
    return { kind: 'ready', due_at: task.due_at };
  const choices = timeChoices(targetDate, original.time, zone);
  if (choices.length === 1)
    return { kind: 'ready', due_at: choices[0].instant };
  return {
    kind: 'confirm',
    date: targetDate,
    time: original.time,
    choices,
    reason: choices.length ? 'fold' : 'gap',
  };
}
export function meetingStart(d: DueDraft, zone: string) {
  const at = resolveTaskDate(d, zone);
  if (!at) throw new Error('A meeting requires a start time.');
  const f = dueFields(at, zone);
  return {
    starts_at: at,
    start_local: `${f.date}T${f.time}`,
    start_offset: f.offset!,
    time_zone: zone,
  };
}
export function newMeetingDraft(nowUtc: string, zone: string): SeriesDraft {
  const z = Temporal.Instant.from(nowUtc).toZonedDateTimeISO(zone);
  const next = z
    .add({ hours: 1 })
    .with({ minute: 0, second: 0, millisecond: 0, microsecond: 0, nanosecond: 0 });
  return {
    ...meetingStart(
      {
        date: next.toPlainDate().toString(),
        time: next.toPlainTime().toString({ smallestUnit: 'millisecond' }),
        offset: next.offset,
      },
      zone,
    ),
    title: '',
    duration_min: 30,
    link_url: null,
    agenda_md: '',
    notes_md: '',
    reminder_min: 15,
    show_in_day: true,
    task_ids: [],
    repeat_rule: 'none',
    repeat_weekdays: [next.dayOfWeek <= 5 ? next.dayOfWeek : 1],
    repeat_until: null,
    fold_policy: 'earlier',
  };
}
