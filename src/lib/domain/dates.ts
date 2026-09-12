/** Date boundaries use the local calendar, not UTC's date substring. */
export function localDate(instant: Date, timeZone?: string): string {
  const parts = new Intl.DateTimeFormat('en-US', { timeZone, year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(instant);
  const value = (type: string) => parts.find(p => p.type === type)!.value;
  return `${value('year')}-${value('month')}-${value('day')}`;
}
export function todayCount(tasks: { due_at: string | null; status: string }[], now: Date, timeZone?: string): number {
  const today = localDate(now, timeZone);
  return tasks.filter(task => task.status !== 'done' && task.due_at !== null && localDate(new Date(task.due_at), timeZone) <= today).length;
}
