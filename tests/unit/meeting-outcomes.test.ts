// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import PlannerBlock from '../../src/lib/components/planner/PlannerBlock.svelte';
import MeetingPill from '../../src/lib/components/agenda/MeetingPill.svelte';
import { meetingRecordChange } from '../../src/lib/domain/agenda';
import type { Attendance } from '../../src/lib/domain/agenda';
import type { PlannerItem } from '../../src/lib/domain/planner';
import { meeting } from './agenda-fixtures';

let component: ReturnType<typeof mount> | undefined;
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  HTMLDialogElement.prototype.close = function () { this.open = false; };
});
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.innerHTML = '';
});
const button = (label: string) => document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`)!;
function setup(attendance: Attendance = 'unmarked', pending = false, onRecordMeeting = vi.fn().mockResolvedValue(undefined)) {
  const occurrence = { ...meeting.occurrence, attendance, occurrence_notes_md: 'Keep these decisions', link_url: 'https://example.com/meeting' };
  const item: PlannerItem = { id: occurrence.id, kind: 'meeting', title: occurrence.title, start_min: 735, end_min: 765, project_name: occurrence.project_name, project_color: occurrence.project_color, task_id: null, meeting: occurrence };
  const props = { item, compact: true, nowUtc: '2025-09-12T12:00:00Z', timeZone: 'UTC', pending, onRecordMeeting, onOpenMeeting: vi.fn(), onOpenTask: vi.fn(), onDone: vi.fn(), onRemove: vi.fn(), onTimer: vi.fn(), onJoin: vi.fn(), onEditTime: vi.fn(), onGesture: vi.fn() };
  component = mount(PlannerBlock, { target: document.body, props });
  flushSync();
  return { props, occurrence };
}

it('marks the compact meeting occurrence directly without opening its details or moving it', async () => {
  const { props, occurrence } = setup();
  const attended = button('Mark attended: Review');
  attended.focus();
  expect(attended.closest('.meeting-status-host')).toBeTruthy();
  attended.click();
  await vi.waitFor(() => expect(props.onRecordMeeting).toHaveBeenCalledWith(occurrence, 'attended'));
  expect((document.querySelector('dialog') as HTMLDialogElement).open).toBe(false);
  expect(props.onOpenMeeting).not.toHaveBeenCalled();
  expect(props.onGesture).not.toHaveBeenCalled();
  expect(props.onDone).not.toHaveBeenCalled();
});

it('keeps canceled meetings visible and editable, hides Join, and allows restoration', async () => {
  const { props, occurrence } = setup('canceled');
  expect(document.querySelector('.block')?.classList.contains('canceled')).toBe(true);
  expect(document.querySelector('.title')?.textContent).toBe('Review');
  expect(document.querySelector('.meeting-outcome')?.textContent).toBe('Canceled');
  expect(button('Restore canceled meeting: Review').getAttribute('aria-pressed')).toBe('true');
  expect([...document.querySelectorAll('button')].some(b => b.textContent?.trim() === 'Join')).toBe(false);
  button('Restore canceled meeting: Review').click();
  await vi.waitFor(() => expect(props.onRecordMeeting).toHaveBeenCalledWith(occurrence, 'unmarked'));
  button('Details for Review').click();
  expect((document.querySelector('dialog') as HTMLDialogElement).open).toBe(true);
});

it('shows the attended mark and supports clearing it', async () => {
  const { props, occurrence } = setup('attended');
  expect(document.querySelector('.meeting-outcome')?.textContent).toBe('Attended');
  expect(button('Clear attended mark for Review').getAttribute('aria-pressed')).toBe('true');
  button('Clear attended mark for Review').click();
  await vi.waitFor(() => expect(props.onRecordMeeting).toHaveBeenCalledWith(occurrence, 'unmarked'));
});

it('prevents duplicate or locked writes and preserves the source on a rejected mark', async () => {
  let reject!: (reason: Error) => void;
  const save = vi.fn(() => new Promise<void>((_, fail) => { reject = fail; }));
  const { occurrence } = setup('unmarked', false, save);
  button('Mark canceled: Review').click();
  button('Mark attended: Review').click();
  expect(save).toHaveBeenCalledOnce();
  reject(new Error('This meeting changed. Refresh and try again.'));
  await vi.waitFor(() => expect(document.querySelector('[role=alert]')?.textContent).toContain('This meeting changed'));
  expect(document.querySelector('.block')?.classList.contains('canceled')).toBe(false);
  expect(occurrence.occurrence_notes_md).toBe('Keep these decisions');
  expect(button('Mark canceled: Review').disabled).toBe(false);
  await unmount(component!); component = undefined; document.body.innerHTML = '';
  setup('unmarked', true, save);
  expect(button('Mark canceled: Review').disabled).toBe(true);
  button('Mark canceled: Review').click();
  expect(save).toHaveBeenCalledOnce();
});

it('records only the chosen occurrence with its existing notes and concurrency revision', () => {
  const occurrence = { ...meeting.occurrence, occurrence_notes_md: 'Keep these decisions' };
  expect(meetingRecordChange(occurrence, 'canceled')).toEqual({ action: 'record', payload: { ref: occurrence.ref, expectedRevision: 4, attendance: 'canceled', occurrence_notes_md: 'Keep these decisions' } });
  expect(() => meetingRecordChange({ ...occurrence, revision: undefined }, 'attended')).toThrow('Refresh');
  expect(() => meetingRecordChange({ ...occurrence, occurrence_notes_md: undefined }, 'attended')).toThrow('Refresh');
});

it('retains the canceled status and quick actions in the agenda meeting card', async () => {
  const onRecordMeeting = vi.fn().mockResolvedValue(undefined);
  const occurrence = { ...meeting.occurrence, attendance: 'canceled' as const, link_url: 'https://example.com/meeting' };
  component = mount(MeetingPill, { target: document.body, props: { meeting: occurrence, timeZone: 'UTC', onOpen: vi.fn(), onJoin: vi.fn(), onRecordMeeting } });
  flushSync();
  expect(document.querySelector('.meeting')?.classList.contains('canceled')).toBe(true);
  expect(document.querySelector('.join')).toBeNull();
  button('Mark attended: Review').click();
  await vi.waitFor(() => expect(onRecordMeeting).toHaveBeenCalledWith(occurrence, 'attended'));
});
