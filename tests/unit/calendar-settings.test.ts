// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount } from 'svelte';
import CalendarSettings from '../../src/lib/components/calendar/CalendarSettings.svelte';
import type { CalendarStatus } from '../../src/lib/calendar/api';
import type { ProjectRecord } from '../../src/lib/domain/types';
const api = vi.hoisted(() => ({
  status: vi.fn(),
  importClient: vi.fn(),
  connect: vi.fn(),
  cancelConnect: vi.fn(),
  setProject: vi.fn(),
  removeAccount: vi.fn(),
  syncNow: vi.fn(),
  runtime: vi.fn(),
  setMail: vi.fn(),
  setAutoAdd: vi.fn(),
  addOutlook: vi.fn(),
}));
vi.mock('../../src/lib/calendar/api', () => api);
let component: ReturnType<typeof mount>;
let state: CalendarStatus;
const projects = [
  { id: 'p1', name: 'TaxApp', color: 'cyan' },
  { id: 'p2', name: 'EventNation', color: 'lime' },
] as ProjectRecord[];
const account = () => ({
  kind: 'google' as 'google' | 'outlook',
  id: 'a1',
  email: 'me@example.com',
  mail: false,
  mail_tasks: false,
  auto_add: false,
  calendars: [
    { id: 'me@example.com', name: 'me@example.com', color: '#9fc6e7', primary: true, project_id: null },
  ],
  status: { last_sync: null, last_error: null, needs_sign_in: false },
});
beforeEach(() => {
  vi.resetAllMocks();
  state = { has_client: false, connecting: false, accounts: [], past_days: 7, future_days: 60 };
  api.status.mockImplementation(async () => structuredClone(state));
  api.importClient.mockImplementation(async () => {
    state.has_client = true;
    return true;
  });
  api.connect.mockImplementation(async () => {
    state.accounts = [account()];
    return 'me@example.com';
  });
  api.setProject.mockImplementation(async (_a: string, _c: string, p: string | null) => {
    state.accounts[0].calendars[0].project_id = p;
  });
  api.setMail.mockImplementation(async (_a: string, on: boolean) => {
    state.accounts[0].mail_tasks = on;
    state.accounts[0].auto_add &&= on;
  });
  api.setAutoAdd.mockImplementation(async (_a: string, on: boolean) => {
    state.accounts[0].auto_add = on;
  });
  api.syncNow.mockResolvedValue({ created: 3, updated: 1, removed: 0, canceled: 0, errors: [] });
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const button = (text: string) =>
  [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as
    | HTMLButtonElement
    | undefined;
async function setup() {
  component = mount(CalendarSettings, { target: document.body, props: { projects } });
  await vi.waitFor(() => expect(document.body.textContent).toContain('Google client file'));
}

it('needs the client file before an account can connect', async () => {
  await setup();
  expect(button('Connect a Google account')!.disabled).toBe(true);
  button('Choose client file')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Google client file loaded.'));
  expect(button('Connect a Google account')!.disabled).toBe(false);
});

it('connects an account, maps a calendar to a project and syncs', async () => {
  state.has_client = true;
  await setup();
  button('Connect a Google account')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Connected me@example.com.'));
  expect(button('Sync now')!.disabled).toBe(true);
  const select = document.querySelector<HTMLSelectElement>('select[aria-label="Project for me@example.com"]')!;
  select.value = 'p2';
  select.dispatchEvent(new Event('change', { bubbles: true }));
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('me@example.com meetings will appear in EventNation.'),
  );
  expect(api.setProject).toHaveBeenCalledWith('a1', 'me@example.com', 'p2');
  await vi.waitFor(() => expect(button('Sync now')!.disabled).toBe(false));
  button('Sync now')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Synced: 3 added, 1 updated.'));
});

it('offers to reconnect an account Google signed out', async () => {
  state.has_client = true;
  state.accounts = [account()];
  state.accounts[0].status = {
    last_sync: null,
    last_error: 'Google no longer accepts Signal\'s sign-in for this account. Connect the account again.',
    needs_sign_in: true,
  };
  await setup();
  expect(document.querySelector('[role="alert"]')!.textContent).toContain('Connect the account again');
  expect(button('Connect me@example.com again')).toBeDefined();
});

it('asks to reconnect before email suggestions, then turns them on per inbox', async () => {
  state.has_client = true;
  state.accounts = [account()];
  component = mount(CalendarSettings, { target: document.body, props: { projects } });
  await vi.waitFor(() => expect(document.body.textContent).toContain('Connect me@example.com again to allow email'));
  expect(document.body.querySelector('label.mail')).toBeNull();
  // Reconnecting grants read-only email access.
  api.connect.mockImplementationOnce(async () => {
    state.accounts[0].mail = true;
    return 'me@example.com';
  });
  const reconnect = [...document.querySelectorAll('button')].find((b) => b.textContent?.includes('again to allow email'))!;
  reconnect.click();
  const box = await vi.waitFor(() => {
    const el = document.body.querySelector<HTMLInputElement>('label.mail input');
    expect(el).not.toBeNull();
    return el!;
  });
  expect(box.checked).toBe(false);
  const auto = () => document.body.querySelector<HTMLInputElement>('label.mail.auto input')!;
  // Adding without review needs suggestions on first.
  expect(auto().disabled).toBe(true);
  expect(document.body.textContent).toContain('sends new email to your AI provider');
  box.click();
  await vi.waitFor(() => expect(api.setMail).toHaveBeenCalledWith('a1', true));
  await vi.waitFor(() => expect(document.body.textContent).toContain('Email from me@example.com will be suggested as tasks.'));
  expect(document.body.querySelector<HTMLInputElement>('label.mail input')!.checked).toBe(true);
  expect(document.body.textContent).toContain('Signal checks every 15 minutes');
  await vi.waitFor(() => expect(auto().disabled).toBe(false));
  expect(auto().checked).toBe(false);
  auto().click();
  await vi.waitFor(() => expect(api.setAutoAdd).toHaveBeenCalledWith('a1', true));
  await vi.waitFor(() => expect(document.body.textContent).toContain('Sure tasks from me@example.com will be added by themselves'));
  expect(auto().checked).toBe(true);
});

it('finds the Outlook accounts on this computer and maps their calendar', async () => {
  api.addOutlook.mockImplementation(async () => {
    state.accounts.push({
      ...account(),
      kind: 'outlook',
      id: 'o1',
      email: 'me@work.com',
      mail: true,
      calendars: [{ id: 'outlook', name: 'Calendar', color: '#0078d4', primary: true, project_id: null }],
    });
    return { added: ['me@work.com'], found: 1 };
  });
  api.setProject.mockImplementation(async (_a: string, _c: string, p: string | null) => {
    state.accounts[0].calendars[0].project_id = p;
  });
  await setup();
  // Outlook needs no Google client file.
  expect(button('Find Outlook accounts')!.disabled).toBe(false);
  button('Find Outlook accounts')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Added me@work.com.'));
  // Outlook accounts can already read mail, so the switch shows without reconnecting.
  expect(document.body.querySelector('label.mail')).not.toBeNull();
  expect(document.body.textContent).not.toContain('again to allow email');
  const select = document.querySelector<HTMLSelectElement>('select[aria-label="Project for me@work.com calendar"]')!;
  select.value = 'p1';
  select.dispatchEvent(new Event('change', { bubbles: true }));
  await vi.waitFor(() => expect(api.setProject).toHaveBeenCalledWith('o1', 'outlook', 'p1'));
  await vi.waitFor(() => expect(document.body.textContent).toContain('me@work.com meetings will appear in TaxApp.'));
  await vi.waitFor(() => expect(button('Sync now')!.disabled).toBe(false));
  api.addOutlook.mockResolvedValue({ added: [], found: 1 });
  button('Find Outlook accounts')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Outlook has 1 account; all are already here.'));
});
