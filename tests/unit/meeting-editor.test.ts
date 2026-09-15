// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import NewMeetingDialog from '../../src/lib/components/meetings/NewMeetingDialog.svelte';
import { project, meeting } from './agenda-fixtures';
import type {
  MeetingChange,
  MeetingWriteResult,
} from '../../src/lib/domain/agenda';
let component: ReturnType<typeof mount>;
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const props = () => ({
  projects: [project],
  projectId: 'p',
  nowUtc: '2025-09-12T12:00Z',
  timeZone: 'UTC',
  onSave: vi.fn(),
  onPreview: vi.fn(),
  onSearch: vi.fn().mockResolvedValue({ items: [], nextCursor: null }),
  onClose: vi.fn(),
  onRetry: vi.fn(),
  onOpenTask: vi.fn(),
  onJoin: vi.fn(),
});
it('saves a detail edit with a clean field-only payload and resolved wall/UTC input', async () => {
  const p = props();
  p.onSave.mockResolvedValue({ detail: meeting });
  component = mount(NewMeetingDialog, {
    target: document.body,
    props: { ...p, detail: meeting },
  });
  flushSync();
  document
    .querySelector('form')!
    .dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(p.onSave).toHaveBeenCalledOnce());
  const c = p.onSave.mock.calls[0][0] as MeetingChange;
  expect(c.action).toBe('edit_occurrence');
  if (c.action === 'edit_occurrence') {
    expect(c.payload.expectedRevision).toBe(4);
    expect(c.payload.draft).not.toHaveProperty('revision');
    expect(c.payload.draft).not.toHaveProperty('repeat_rule');
    expect(c.payload.draft).not.toHaveProperty('repeat_weekdays');
    expect(c.payload.draft.start_local).toBe('2025-09-12T12:15:00.000');
  }
});

function select(selector: string, value: string) {
  const el = document.querySelector<HTMLSelectElement>(selector)!;
  el.value = value;
  el.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}
function labeledSelect(label: string) {
  return [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.trim().startsWith(label))!.querySelector('select')!;
}
function changeSelect(el: HTMLSelectElement, value: string) {
  el.value = value;
  el.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}
function button(text: string) {
  return [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === text)!;
}
function title(value: string) {
  const el = [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.trim() === 'Title')!.querySelector('input')!;
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}
function submit() {
  document.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  flushSync();
}
it('offers five-minute choices, defaults each chosen hour, and retains an explicit AM/PM override', async () => {
  const p = props();
  p.onSave.mockResolvedValue({ detail: meeting });
  component = mount(NewMeetingDialog, { target: document.body, props: p });
  flushSync();
  expect(document.querySelector('input[type="time"]')).toBeNull();
  const minutes = document.querySelector<HTMLSelectElement>('[aria-label="Meeting minute"]')!;
  expect(minutes.value).toBe('00');
  expect([...minutes.options].map((o) => o.value)).toEqual(['00', '05', '10', '15', '20', '25', '30', '35', '40', '45', '50', '55']);
  for (const h of [7, 8, 9, 10, 11, 12, 1, 2, 3, 4, 5, 6]) {
    select('[aria-label="Meeting hour"]', String(h));
    expect(document.querySelector<HTMLSelectElement>('[aria-label="Meeting AM/PM"]')!.value)
      .toBe(h >= 7 && h <= 11 ? 'AM' : 'PM');
  }
  select('[aria-label="Meeting hour"]', '9');
  select('[aria-label="Meeting AM/PM"]', 'PM');
  select('[aria-label="Meeting minute"]', '25');
  expect(document.querySelector<HTMLSelectElement>('[aria-label="Meeting AM/PM"]')!.value).toBe('PM');
  expect([...labeledSelect('Timezone').options].map((o) => o.text)).toEqual(['Eastern', 'Central', 'Mountain', 'Pacific']);
  changeSelect(labeledSelect('Timezone'), 'America/Chicago');
  title('Evening meeting');
  submit();
  await vi.waitFor(() => expect(p.onSave).toHaveBeenCalledOnce());
  expect(p.onSave.mock.calls[0][0].payload.draft).toMatchObject({
    start_local: '2025-09-12T21:25:00.000', starts_at: '2025-09-13T02:25:00.000Z', time_zone: 'America/Chicago',
  });
});
it.each([15, 25, 30, 45, 55, 60])('sets the duration to %i minutes using its quick link', (minutes) => {
  component = mount(NewMeetingDialog, { target: document.body, props: props() });
  flushSync();
  button(minutes === 60 ? '1h' : `${minutes}m`).click();
  flushSync();
  const duration = [...document.querySelectorAll('label')].find((l) => l.textContent?.startsWith('Duration'))!.querySelector('input')!;
  expect(duration.value).toBe(String(minutes));
});
it('saves selected weekdays and advances a Friday start to Monday when Friday is excluded', async () => {
  const p = props();
  p.onSave.mockResolvedValue({ detail: meeting });
  component = mount(NewMeetingDialog, { target: document.body, props: p });
  flushSync();
  title('Monday through Thursday');
  changeSelect(labeledSelect('Repeats'), 'weekly');
  expect(document.querySelectorAll('[aria-label="Repeat on weekdays"] button')).toHaveLength(5);
  button('Fri').click(); flushSync();
  submit();
  expect(p.onSave).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('Choose at least one repeat weekday.');
  for (const day of ['Mon', 'Tue', 'Wed', 'Thu']) { button(day).click(); flushSync(); }
  expect(document.body.textContent).toContain('First meeting: 2025-09-15');
  submit();
  await vi.waitFor(() => expect(p.onSave).toHaveBeenCalledOnce());
  expect(p.onSave.mock.calls[0][0].payload.draft).toMatchObject({ repeat_rule: 'weekly', repeat_weekdays: [1, 2, 3, 4], start_local: '2025-09-15T09:00:00.000' });
});
it('keeps weekday changes within the following scope and includes them in the review', async () => {
  const p = props();
  p.onPreview.mockResolvedValue({ fingerprint: 'f', affected_counts: {}, preserved_overrides: [], skipped_dates: [], warnings: [], diagnostic_range: { start: '', end: '' } });
  component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: {
    ...meeting, series: { ...meeting.series, is_recurring: true, repeat_rule: 'weekly', repeat_weekdays: [1, 3] },
  } } });
  flushSync();
  expect(button('Mon').disabled).toBe(true);
  changeSelect(labeledSelect('Edit scope'), 'following');
  expect(button('Mon').disabled).toBe(false);
  button('Tue').click(); flushSync();
  submit();
  await vi.waitFor(() => expect(p.onPreview).toHaveBeenCalledOnce());
  expect(p.onPreview.mock.calls[0][0]).toMatchObject({ action: 'edit_following', payload: { draft: { repeat_weekdays: [1, 2, 3] } } });
  expect(p.onSave).not.toHaveBeenCalled();
});
it('preserves saved timezone and precise legacy time when editing only the title', async () => {
  const p = props();
  p.onSave.mockResolvedValue({ detail: meeting });
  component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: {
    ...meeting, occurrence: { ...meeting.occurrence, starts_at: '2025-09-12T12:17:12.345Z', start_local: '2025-09-12T12:17:12.345' },
  } } });
  flushSync();
  expect(document.querySelector<HTMLSelectElement>('[aria-label="Meeting minute"]')!.value).toBe('17');
  expect(labeledSelect('Timezone').value).toBe('UTC');
  title('Rename only'); submit();
  await vi.waitFor(() => expect(p.onSave).toHaveBeenCalledOnce());
  expect(p.onSave.mock.calls[0][0].payload.draft.starts_at).toBe('2025-09-12T12:17:12.345Z');
});
it('requests explicit discard for dirty drafts and refuses Close while recovering', async () => {
  component = mount(NewMeetingDialog, {
    target: document.body,
    props: props(),
  });
  flushSync();
  const title = [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.trim() === 'Title')!
    .querySelector('input')!;
  title.value = 'Unsaved';
  title.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  const close = (
    component as { requestClose: () => Promise<boolean> }
  ).requestClose();
  flushSync();
  expect(document.body.textContent).toContain(
    'Discard unsaved meeting changes?',
  );
  [...document.querySelectorAll('button')]
    .find((b) => b.textContent === 'Keep editing')!
    .click();
  expect(await close).toBe(false);
  expect(title.value).toBe('Unsaved');
});

it.each(['save', 'receipt replay'])(
  'saves meeting fields, attendance and notes together and closes after %s',
  async (mode) => {
    const p = props();
    const saved = {
      detail: { ...meeting, occurrence: { ...meeting.occurrence, title: 'Updated meeting', revision: 5,
        attendance: 'attended', occurrence_notes_md: 'Saved decision' } },
    } as MeetingWriteResult;
    p.onSave.mockResolvedValue(saved);
    if (mode === 'receipt replay') p.onSave.mockRejectedValueOnce({ code: 'UnknownOutcome', message: 'Lost reply' });
    component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: meeting } });
    flushSync();
    title('Updated meeting');
    const notes = [...document.querySelectorAll('label')].find(l => l.textContent?.trim() === 'Occurrence notes')!.querySelector('textarea')!;
    notes.value = 'Saved decision'; notes.dispatchEvent(new Event('input', { bubbles: true }));
    changeSelect(labeledSelect('Attendance'), 'attended');
    submit();
    await vi.waitFor(() => expect(p.onSave).toHaveBeenCalledOnce());
    expect(p.onSave.mock.calls[0][0]).toMatchObject({ action: 'edit_occurrence', payload: {
      expectedRevision: 4, draft: { title: 'Updated meeting' },
      occurrence: { attendance: 'attended', occurrence_notes_md: 'Saved decision' },
    } });
    if (mode === 'receipt replay') {
      await vi.waitFor(() => { flushSync(); expect(document.querySelector('fieldset')!.disabled).toBe(false); });
      expect(p.onClose).not.toHaveBeenCalled();
      expect(notes.value).toBe('Saved decision');
      (component as { acceptRetry: (r: MeetingWriteResult) => void }).acceptRetry(saved);
      flushSync();
    }
    await vi.waitFor(() => expect(p.onClose).toHaveBeenCalledOnce());
    expect(await (component as { requestClose: () => Promise<boolean> }).requestClose()).toBe(true);
    expect(document.body.textContent).not.toContain('Discard unsaved meeting changes?');
  },
);

it('saves an attendance-only change with the main button', async () => {
  const p = props();
  p.onSave.mockResolvedValue({ detail: { ...meeting, occurrence: { ...meeting.occurrence, attendance: 'attended', revision: 5 } } });
  component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: meeting } });flushSync();
  changeSelect(labeledSelect('Attendance'), 'attended');
  document.querySelector('form')!.requestSubmit();
  await vi.waitFor(() => expect(p.onClose).toHaveBeenCalledOnce());
  expect(p.onSave).toHaveBeenCalledOnce();
  expect(p.onSave.mock.calls[0][0].payload.occurrence).toEqual({ attendance: 'attended', occurrence_notes_md: '' });
  expect(await (component as { requestClose: () => Promise<boolean> }).requestClose()).toBe(true);
});
it('keeps all edits and displays an error when the combined save fails', async () => {
  const p = props();p.onSave.mockRejectedValue({ code: 'Database', message: 'Write unavailable' });
  component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: meeting } });flushSync();
  title('Keep this edit');changeSelect(labeledSelect('Attendance'), 'attended');submit();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Write unavailable'));
  expect(p.onClose).not.toHaveBeenCalled();expect(labeledSelect('Attendance').value).toBe('attended');
  expect([...document.querySelectorAll('label')].find(l => l.textContent?.trim() === 'Title')!.querySelector('input')!.value).toBe('Keep this edit');
});
it('the attendance-only shortcut keeps unrelated meeting edits dirty and stays open', async () => {
  const p = props();p.onSave.mockResolvedValue({ detail: { ...meeting, occurrence: { ...meeting.occurrence, attendance: 'attended', revision: 5 } } });
  component = mount(NewMeetingDialog, { target: document.body, props: { ...p, detail: meeting } });flushSync();
  title('Unsaved schedule edit');changeSelect(labeledSelect('Attendance'), 'attended');button('Save attendance & notes').click();
  await vi.waitFor(() => { flushSync(); expect(document.querySelector('fieldset')!.disabled).toBe(false); });
  expect(p.onSave.mock.calls[0][0].action).toBe('record');expect(p.onClose).not.toHaveBeenCalled();
  const closing=(component as { requestClose: () => Promise<boolean> }).requestClose();flushSync();
  expect(document.body.textContent).toContain('Discard unsaved meeting changes?');button('Keep editing').click();expect(await closing).toBe(false);
});

it('submits a rapid repeated Save only once while the first request is unresolved', async () => {
  const p = props();
  let resolve!: (r: unknown) => void;
  p.onSave.mockImplementation(() => new Promise((r) => (resolve = r)));
  component = mount(NewMeetingDialog, {
    target: document.body,
    props: { ...p, detail: meeting },
  });
  flushSync();
  const form = document.querySelector('form')!;
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  expect(p.onSave).toHaveBeenCalledOnce();
  expect(
    await (
      component as { requestClose: () => Promise<boolean> }
    ).requestClose(),
  ).toBe(false);
  resolve({ detail: meeting });
  await vi.waitFor(() => {
    flushSync();
    expect(document.querySelector('fieldset')!.disabled).toBe(false);
  });
});

it('allows recurrence end and fold settings while converting a one-off into a series', () => {
  component = mount(NewMeetingDialog, {
    target: document.body,
    props: { ...props(), detail: meeting },
  });
  flushSync();
  const repeat = [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.startsWith('Repeats'))!
    .querySelector('select')!;
  repeat.value = 'weekly';
  repeat.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
  const end = [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.startsWith('Ends ('))!
    .querySelector('input')!;
  const fold = [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.startsWith('Repeated-hour'))!
    .querySelector('select')!;
  expect(end.disabled).toBe(false);
  expect(fold.disabled).toBe(false);
});
