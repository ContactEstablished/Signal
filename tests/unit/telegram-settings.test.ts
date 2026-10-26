// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import TelegramSettings from '../../src/lib/components/telegram/TelegramSettings.svelte';
import type { TelegramStatus } from '../../src/lib/telegram/api';
const api = vi.hoisted(() => ({
  status: vi.fn(),
  saveToken: vi.fn(),
  removeBot: vi.fn(),
  setConsent: vi.fn(),
  setEnabled: vi.fn(),
  startPairing: vi.fn(),
  cancelPairing: vi.fn(),
  unpair: vi.fn(),
  runtime: vi.fn(),
}));
vi.mock('../../src/lib/telegram/api', () => api);
let component: ReturnType<typeof mount>;
let state: TelegramStatus;
const enableTray = vi.fn();
const fresh = (): TelegramStatus => ({
  has_token: false,
  bot_username: '',
  enabled: false,
  consent: false,
  paired: false,
  tray: false,
  pairing: { active: false, seconds_left: 0, failures: 0, locked: false },
  worker: { running: false, connected: false, last_error: null, last_poll: null },
});
beforeEach(() => {
  vi.resetAllMocks();
  state = fresh();
  api.status.mockImplementation(async () => structuredClone(state));
  api.saveToken.mockImplementation(async () => {
    state.has_token = true;
    state.bot_username = 'signal_dev_bot';
    return { bot_username: 'signal_dev_bot' };
  });
  api.setConsent.mockImplementation(async (v: boolean) => (state.consent = v));
  api.setEnabled.mockImplementation(async (v: boolean) => (state.enabled = v));
  enableTray.mockImplementation(async () => (state.tray = true));
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
const button = (text: string) =>
  [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text,
  ) as HTMLButtonElement | undefined;
const checkbox = (label: string) =>
  [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.includes(label))!
    .querySelector('input') as HTMLInputElement;
async function setup() {
  component = mount(TelegramSettings, {
    target: document.body,
    props: { onEnableTray: enableTray },
  });
  await vi.waitFor(() => expect(document.body.textContent).toContain('Your bot'));
}
it('saves a trimmed bot token and then shows the connected bot without keeping the token', async () => {
  await setup();
  const input = document.querySelector<HTMLInputElement>('input[type="password"]')!;
  input.value = '  123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw  ';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  button('Save token')!.click();
  await vi.waitFor(() =>
    expect(api.saveToken).toHaveBeenCalledWith('123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw'),
  );
  await vi.waitFor(() => expect(document.body.textContent).toContain('@signal_dev_bot'));
  expect(document.querySelector('input[type="password"]')).toBeNull();
  expect(document.body.textContent).not.toContain('AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw');
});

it('keeps listening unavailable until a bot is saved and the AI disclosure is accepted', async () => {
  await setup();
  expect(checkbox('Listen for my Telegram messages').disabled).toBe(true);
  state.has_token = true;
  state.bot_username = 'b';
  await vi.waitFor(() => expect(button('Remove bot')).toBeTruthy(), { timeout: 4000 });
  expect(checkbox('Listen for my Telegram messages').disabled).toBe(true);
  checkbox('I agree to send').click();
  await vi.waitFor(() => expect(api.setConsent).toHaveBeenCalledWith(true));
  await vi.waitFor(() => expect(checkbox('Listen for my Telegram messages').disabled).toBe(false));
});

it('turning listening on first keeps Signal running in the tray', async () => {
  state = { ...fresh(), has_token: true, bot_username: 'b', consent: true };
  await setup();
  checkbox('Listen for my Telegram messages').click();
  await vi.waitFor(() => expect(api.setEnabled).toHaveBeenCalledWith(true));
  expect(enableTray).toHaveBeenCalledBefore(api.setEnabled);
  await vi.waitFor(() => expect(document.body.textContent).toContain('Signal is now listening'));
});

it('shows the pairing message, the wrong-attempt count, and hides it once paired', async () => {
  state = { ...fresh(), has_token: true, bot_username: 'signal_bot', consent: true, enabled: true, tray: true };
  api.startPairing.mockImplementation(async () => {
    state.pairing = { active: true, seconds_left: 290, failures: 0, locked: false };
    return { code: '123456', minutes: 5, bot_username: 'signal_bot' };
  });
  await setup();
  await vi.waitFor(() => expect(button('Pair my Telegram account')).toBeTruthy());
  button('Pair my Telegram account')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('/start 123456'));
  expect(document.body.textContent).toContain('@signal_bot');
  expect(document.body.textContent).toContain('within 5 minutes');

  state.pairing.failures = 2;
  await vi.waitFor(() => expect(document.body.textContent).toContain('2 wrong attempts'), { timeout: 4000 });

  state.paired = true;
  state.pairing = { active: false, seconds_left: 0, failures: 0, locked: false };
  await vi.waitFor(() => expect(document.body.textContent).toContain('Everyone else is ignored'), { timeout: 4000 });
  expect(document.body.textContent).not.toContain('/start 123456');
});

it('reports a failure from the app and warns when the tray option is off', async () => {
  state = { ...fresh(), has_token: true, bot_username: 'b', consent: true, enabled: true, tray: false };
  api.removeBot.mockRejectedValue({ message: 'Something failed.' });
  await setup();
  expect(document.body.textContent).toContain('closing the window will quit Signal');
  button('Keep Signal running in the tray')!.click();
  await vi.waitFor(() => expect(enableTray).toHaveBeenCalled());
  await vi.waitFor(() => expect(document.body.textContent).not.toContain('closing the window will quit Signal'));
  button('Remove bot')!.click();
  await vi.waitFor(() => expect(document.body.textContent).toContain('Something failed.'));
});
