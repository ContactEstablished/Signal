// @vitest-environment jsdom
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import VoiceInbox from '../../src/lib/views/VoiceInbox.svelte';
import { emailSummary, autoSummary } from '../../src/lib/voice/review';
import type { Draft, Candidate } from '../../src/lib/voice/api';
import type { Workspace } from '../../src/lib/state/app.svelte';
const api = vi.hoisted(() => ({ load: vi.fn(), save: vi.fn(), recovery: vi.fn(), setRecovery: vi.fn(), check: vi.fn(), cancel: vi.fn(), discard: vi.fn(), accept: vi.fn(), newDraft: vi.fn(), undo: vi.fn(), autoStatus: vi.fn(), onChecked: vi.fn() }));
const calendar = vi.hoisted(() => ({ status: vi.fn(), openOutlook: vi.fn() }));
const native = vi.hoisted(() => ({ openExternalUrl: vi.fn() }));
vi.mock('../../src/lib/voice/email-api', () => api);
vi.mock('../../src/lib/calendar/api', () => calendar);
vi.mock('../../src/lib/native/commands', async (original) => ({ ...(await original<object>()), ...native }));
let instance: ReturnType<typeof mount> | undefined;
let draft: Draft | null;
let checked: ((s: unknown) => void) | undefined;
const link = 'https://mail.google.com/mail/?authuser=me%40example.com#search/rfc822msgid%3Aa%40b.com';
const suggestion = (id: string, project_id: string | null): Candidate => ({ id, kind: 'task', selected: true, project_id, proposal_id: null, title: `Reply ${id}`, notes: '', subtasks: [], due_at: '2026-11-20T22:00:00.000Z', priority: 'high', estimate_h: null, source: 'Dana Lee · "Invoice" · Nov 18, 9:00 AM · me@example.com', warning: 'Due date inferred from "by Friday".', duplicate_ok: false, created_id: null, link_url: link });
const button = (text: string) => [...document.querySelectorAll('button')].find(b => b.textContent?.trim() === text)!;
/** Opens every collapsed suggestion. */
const expand = () => { document.querySelectorAll<HTMLButtonElement>('button.toggle[aria-expanded=false]').forEach(b => b.click()); flushSync(); };
const account = (email: string, mail: boolean, mail_tasks: boolean) => ({ kind: 'google', id: email, email, mail, mail_tasks, calendars: [], status: { last_sync: null, last_error: null, needs_sign_in: false } });
beforeEach(() => {
  vi.resetAllMocks();
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  HTMLDialogElement.prototype.close = function () { this.open = false; };
  draft = null;
  calendar.status.mockResolvedValue({ has_client: true, connecting: false, past_days: 7, future_days: 60, accounts: [account('me@example.com', true, true), account('work@example.com', true, false)] });
  api.load.mockImplementation(async () => structuredClone(draft));
  api.recovery.mockResolvedValue(null);
  api.cancel.mockResolvedValue(undefined);
  api.newDraft.mockImplementation((now: string, zone: string) => ({ id: 'email', source_kind: 'email', revision: 0, recorded_at: now, time_zone: zone, transcript: '', projects: [], candidates: [] }));
  api.save.mockImplementation(async (d: Draft) => { draft = { ...structuredClone(d), revision: d.revision + 1 }; return structuredClone(draft); });
  native.openExternalUrl.mockResolvedValue(undefined);
  api.autoStatus.mockResolvedValue(null);
  checked = undefined;
  api.onChecked.mockImplementation(async (fn: (s: unknown) => void) => { checked = fn; return () => {}; });
});
afterEach(async () => { if (instance) await unmount(instance); instance = undefined; document.body.innerHTML = ''; });
async function setup() {
  const workspace = { nowUtc: '2026-11-18T19:00:00.000Z', timeZone: 'America/New_York', projects: [{ id: 'p', name: 'Acme', color: 'cyan' }], voiceRecovery: false, imageRecovery: false, emailRecovery: false,
    acceptEmail: vi.fn(async (input: { candidate_ids: string[] }) => {
      for (const c of draft!.candidates) if (input.candidate_ids.includes(c.id)) { c.created_id = 'created-' + c.id; c.selected = false; }
      draft!.revision++;
      return { created: [], projects: [] };
    }) };
  const props = { workspace: workspace as unknown as Workspace, mode: 'email' as const, onOpenTask: vi.fn(), onOpenMeeting: vi.fn(), onSettings: vi.fn(), onGuard: vi.fn() };
  instance = mount(VoiceInbox, { target: document.body, props });
  await vi.waitFor(() => expect(button('Check email')).toBeTruthy());
  return { ...props, workspace };
}

it('checks only the inboxes that suggest tasks and adds suggestions to the review', async () => {
  const p = await setup();
  await vi.waitFor(() => expect(document.body.textContent).toContain('From me@example.com'));
  expect(document.body.textContent).not.toContain('work@example.com');
  api.check.mockImplementation(async (d: Draft) => {
    draft = { ...structuredClone(d), revision: d.revision + 1, candidates: [...d.candidates, suggestion('a', 'p'), suggestion('b', null)] };
    return { draft: structuredClone(draft), checked: 4, suggested: 2, remaining: 3, inboxes: ['me@example.com'], problems: [] };
  });
  button('Check email').click();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(2));
  expect(document.querySelectorAll('article')[1].textContent).toContain('needs a project');
  expand();
  // The review was saved first, so the check runs against the stored revision.
  expect(api.save).toHaveBeenCalled();
  expect(api.check.mock.calls[0][0].source_kind).toBe('email');
  expect(document.body.textContent).toContain('Checked 4 new emails: 2 suggested tasks added below. 3 more new emails waiting');
  expect(document.body.textContent).toContain('Due date inferred from "by Friday".');
  // Email never invents projects: an unassigned suggestion blocks creation until one is chosen.
  expect(document.body.textContent).toContain('Which project is this task for?');
  expect(button('Create selected (2)').disabled).toBe(true);
  document.querySelectorAll('article')[1].querySelector<HTMLInputElement>('input[type=checkbox]')!.click(); flushSync();
  await vi.waitFor(() => expect(button('Create selected (1)').disabled).toBe(false));
  [...document.querySelectorAll('button')].find(b => b.textContent?.includes('Open email'))!.click();
  expect(native.openExternalUrl).toHaveBeenCalledWith(link);
  button('Create selected (1)').click();
  await vi.waitFor(() => expect(p.workspace.acceptEmail).toHaveBeenCalledOnce());
  expect(p.workspace.acceptEmail.mock.calls[0][0].candidate_ids).toEqual(['a']);
  expect(api.setRecovery).toHaveBeenCalledWith(expect.objectContaining({ candidate_ids: ['a'] }));
  await vi.waitFor(() => expect(api.setRecovery).toHaveBeenLastCalledWith(null));
  expect(p.workspace.emailRecovery).toBe(false);
  expect(p.workspace.voiceRecovery).toBe(false);
});

it('keeps existing suggestions and reports a failing inbox without hiding the rest', async () => {
  draft = { id: 'email', source_kind: 'email', revision: 3, recorded_at: '2026-11-18T14:00:00.000Z', time_zone: 'America/New_York', transcript: 'Dana is high priority', projects: [], candidates: [suggestion('old', 'p')] };
  await setup();
  expect((document.querySelector('[aria-label=Instructions]') as HTMLTextAreaElement).value).toBe('Dana is high priority');
  api.check.mockImplementation(async (d: Draft) => {
    draft = { ...structuredClone(d), revision: d.revision + 1, candidates: [...d.candidates, suggestion('new', 'p')] };
    return { draft: structuredClone(draft), checked: 1, suggested: 1, remaining: 0, inboxes: ['me@example.com'], problems: ['work@example.com: Connect this account again.'] };
  });
  button('Check email').click();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(2));
  expand();
  const titles = [...document.querySelectorAll<HTMLInputElement>('article input:not([type])')].map(i => i.value);
  expect(titles).toEqual(['Reply old', 'Reply new']);
  expect(document.querySelector('[role=alert]')?.textContent).toContain('work@example.com: Connect this account again.');
  // No confirm-and-replace step: checking email only adds.
  expect(document.querySelector('dialog')).toBeNull();
  api.check.mockRejectedValue(new Error('The Gmail API is not turned on'));
  button('Check email').click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('The Gmail API is not turned on'));
  expect(document.querySelectorAll('article')).toHaveLength(2);
});

it('points to settings when no inbox suggests tasks', async () => {
  calendar.status.mockResolvedValue({ has_client: true, connecting: false, past_days: 7, future_days: 60, accounts: [account('me@example.com', false, false)] });
  const p = await setup();
  await vi.waitFor(() => expect(document.body.textContent).toContain('No inbox is set up for suggestions yet.'));
  expect(button('Check email').disabled).toBe(true);
  button('Calendar & email settings').click();
  expect(p.onSettings).toHaveBeenCalledWith('Calendar & email');
  button('Voice & AI settings').click();
  expect(p.onSettings).toHaveBeenLastCalledWith();
});

it('summarizes a check in plain words', () => {
  expect(emailSummary({ checked: 0, suggested: 0, remaining: 0, problems: [] })).toBe('No new email since the last check.');
  expect(emailSummary({ checked: 0, suggested: 0, remaining: 0, problems: ['x'] })).toBe('');
  expect(emailSummary({ checked: 1, suggested: 0, remaining: 0, problems: [] })).toBe('Checked 1 new email: no tasks found.');
  expect(emailSummary({ checked: 15, suggested: 1, remaining: 1, problems: [] })).toBe('Checked 15 new emails: 1 suggested task added below. 1 more new email waiting; click Check email again.');
});

it('opens an Outlook suggestion in Outlook', async () => {
  const outlook = { ...suggestion('o', 'p'), link_url: null, mail_ref: 'acct-1:00FF' };
  draft = { id: 'email', source_kind: 'email', revision: 1, recorded_at: '2026-11-18T14:00:00.000Z', time_zone: 'America/New_York', transcript: '', projects: [], candidates: [outlook] };
  calendar.openOutlook.mockResolvedValue(undefined);
  await setup();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(1));
  expand();
  [...document.querySelectorAll('button')].find(b => b.textContent?.includes('Open email'))!.click();
  expect(calendar.openOutlook).toHaveBeenCalledWith('acct-1:00FF');
  expect(native.openExternalUrl).not.toHaveBeenCalled();
});

it('lists tasks Signal added by itself first, with Undo', async () => {
  const auto = { ...suggestion('auto', 'p'), selected: false, auto: true, created_id: 'task-9', title: 'Send invoice' };
  draft = { id: 'email', source_kind: 'email', revision: 2, recorded_at: '2026-11-18T14:00:00.000Z', time_zone: 'America/New_York', transcript: '', projects: [], candidates: [suggestion('r', 'p'), auto] };
  api.undo.mockImplementation(async (d: Draft, id: string) => {
    draft = { ...structuredClone(d), revision: d.revision + 1, candidates: d.candidates.filter(c => c.id !== id) };
    return structuredClone(draft);
  });
  const p = await setup();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(2));
  const groups = [...document.querySelectorAll('h3.group-title')].map(h => h.textContent?.trim());
  expect(groups).toEqual(['Added automatically', 'Acme']);
  const first = document.querySelectorAll('article')[0];
  expect(first.textContent).toContain('Added: Send invoice');
  expect(first.textContent).toContain('Dana Lee');
  [...first.querySelectorAll('button')].find(b => b.textContent?.includes('Open task'))!.click();
  expect(p.onOpenTask).toHaveBeenCalledWith('task-9');
  [...first.querySelectorAll('button')].find(b => b.textContent?.includes('Open email'))!.click();
  expect(native.openExternalUrl).toHaveBeenCalledWith(link);
  first.querySelector<HTMLButtonElement>('button[aria-label="Undo Send invoice"]')!.click();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(1));
  expect(api.undo.mock.calls[0][1]).toBe('auto');
  expect(document.body.textContent).toContain('Removed “Send invoice”.');
});

it('shows the results of an automatic check without disturbing edits', async () => {
  draft = { id: 'email', source_kind: 'email', revision: 1, recorded_at: '2026-11-18T14:00:00.000Z', time_zone: 'America/New_York', transcript: '', projects: [], candidates: [suggestion('a', 'p')] };
  api.autoStatus.mockResolvedValue({ at: '2026-11-18T19:00:00.000Z', checked: 0, suggested: 0, added: 0, remaining: 0, error: null });
  await setup();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Checked automatically at 2:00 PM. No new email.'));
  expect(document.body.textContent).toContain('Signal checks every 15 minutes while it is open');
  // The background check stored a newer review; the page reloads it.
  draft = { ...draft!, revision: 3, candidates: [...draft!.candidates, { ...suggestion('b', 'p'), auto: true, created_id: 'task-b' }] };
  checked!({ at: '2026-11-18T19:15:00.000Z', checked: 3, suggested: 0, added: 1, remaining: 0, error: null });
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(2));
  expect(document.body.textContent).toContain('Checked automatically at 2:15 PM: 3 new emails, 1 task added automatically.');
});

it('summarizes automatic additions', () => {
  expect(emailSummary({ checked: 4, suggested: 2, added: 1, remaining: 0, problems: [] })).toBe('Checked 4 new emails: 1 task added automatically, 2 suggested tasks added below.');
  expect(autoSummary({ at: '2026-11-18T19:00:00.000Z', checked: 20, suggested: 1, added: 2, remaining: 5 }, 'America/New_York')).toBe('Checked automatically at 2:00 PM: 20 new emails, 2 tasks added automatically, 1 suggested task added below. 5 more new emails will be checked in a few minutes.');
});

it('shows each suggestion as a collapsed bar with its title and a Delete button', async () => {
  draft = { id: 'email', source_kind: 'email', revision: 1, recorded_at: '2026-11-18T14:00:00.000Z', time_zone: 'America/New_York', transcript: '', projects: [], candidates: [suggestion('a', 'p'), suggestion('b', 'p')] };
  await setup();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(2));
  const first = document.querySelectorAll('article')[0];
  // Collapsed: title, sender, priority and due date on the bar; no fields yet.
  expect(first.querySelector('.toggle')!.textContent).toContain('Reply a');
  expect(first.querySelector('.toggle')!.textContent).toContain('Dana Lee · high priority · due Nov 20');
  expect(first.querySelector('input:not([type])')).toBeNull();
  // The include checkbox works without opening the row.
  first.querySelector<HTMLInputElement>('input[aria-label="Include Reply a"]')!.click(); flushSync();
  await vi.waitFor(() => expect(button('Create selected (1)')).toBeTruthy());
  first.querySelector<HTMLButtonElement>('.toggle')!.click(); flushSync();
  expect(first.querySelector('.toggle')!.getAttribute('aria-expanded')).toBe('true');
  expect(first.querySelector<HTMLInputElement>('input:not([type])')!.value).toBe('Reply a');
  // Delete drops it from the saved review.
  first.querySelector<HTMLButtonElement>('button[aria-label="Delete Reply a"]')!.click();
  await vi.waitFor(() => expect(document.querySelectorAll('article')).toHaveLength(1));
  expect(document.body.textContent).toContain('Deleted “Reply a”.');
  await vi.waitFor(() => expect(draft!.candidates.map(c => c.id)).toEqual(['b']));
});
