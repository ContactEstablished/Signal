/** Parse typed block times to minutes since midnight; NaN means incomplete/invalid. */
export function parsePlannerTime(value: string): number {
  if (/^12:00\s*am\s*\(next day\)$/i.test(value.trim())) return 1440;
  const match = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/i.exec(value.trim());
  if (!match) return NaN;
  const [, hourText, minuteText, suffix] = match;
  let hour = Number(hourText);
  const minute = Number(minuteText ?? 0);
  if (minute > 59) return NaN;
  if (suffix) {
    if (hour < 1 || hour > 12) return NaN;
    hour = hour % 12 + (suffix.toLowerCase() === 'pm' ? 12 : 0);
  } else {
    if (hour > 24 || (hour === 24 && minute !== 0)) return NaN;
    // Leading-zero times are explicit 24-hour entries (e.g. 02:30).
    const explicit24Hour = hourText.length === 2 && hourText.startsWith('0');
    if (!explicit24Hour && hour >= 1 && hour <= 6) hour += 12;
  }
  return hour * 60 + minute;
}
