// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import VoiceSettings from '../../src/lib/components/voice/VoiceSettings.svelte';
import type { SettingsReply } from '../../src/lib/voice/api';
const api = vi.hoisted(() => ({
  settings: vi.fn(),
  download: vi.fn(),
  saveSettings: vi.fn(),
  cancel: vi.fn(),
}));
vi.mock('../../src/lib/voice/api', () => api);
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
let component: ReturnType<typeof mount>;
let stored: SettingsReply;
beforeEach(() => {
  vi.resetAllMocks();
  stored = {
    settings: {
      endpoint: '',
      model: '',
      whisper_model: 'base.en',
      microphone: '',
      consent: false,
    },
    has_key: false,
    models: [
      { id: 'base.en', bytes: 147964211, installed: false },
      { id: 'small.en', bytes: 487614201, installed: false },
    ],
  };
  api.settings.mockImplementation(async () => structuredClone(stored));
  api.download.mockImplementation(async (id: string) => {
    stored.models.find((m) => m.id === id)!.installed = true;
  });
  api.saveSettings.mockImplementation(async (settings, key) => {
    stored.settings = structuredClone(settings);
    stored.has_key = !!key;
  });
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
function button(text: string) {
  return [...document.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text,
  )!;
}
function field<T extends HTMLInputElement | HTMLSelectElement>(label: string) {
  return [...document.querySelectorAll('label')]
    .find((l) => l.textContent?.trim().startsWith(label))!
    .querySelector<T>('input, select')!;
}
function change(el: HTMLInputElement | HTMLSelectElement, value: string) {
  el.value = value;
  el.dispatchEvent(
    new Event(el.tagName === 'SELECT' ? 'change' : 'input', { bubbles: true }),
  );
  flushSync();
}
async function setup() {
  component = mount(VoiceSettings, { target: document.body });
  await vi.waitFor(() => expect(button('Save settings')).toBeTruthy());
}
function download(id: string) {
  const row = [...document.querySelectorAll('.model')].find((r) =>
    r.textContent?.includes(id),
  )!;
  row.querySelector('button')!.click();
}
it('downloading small.en preserves pending provider fields and saves the selected installed model', async () => {
  await setup();
  change(field('Local transcription model'), 'small.en');
  change(field('HTTPS API URL'), 'https://provider.example/v1');
  change(field('Model ID'), 'review-model');
  change(field('API key'), 'synthetic-test-key');
  field<HTMLInputElement>('I agree').click();
  flushSync();
  download('small.en');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain(
      'small.en installed and selected',
    ),
  );
  expect(field('Local transcription model').value).toBe('small.en');
  expect(field('HTTPS API URL').value).toBe('https://provider.example/v1');
  expect(field('Model ID').value).toBe('review-model');
  expect(field('API key').value).toBe('synthetic-test-key');
  expect(field<HTMLInputElement>('I agree').checked).toBe(true);
  button('Save settings').click();
  await vi.waitFor(() =>
    expect(api.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        whisper_model: 'small.en',
        model: 'review-model',
        consent: true,
      }),
      'synthetic-test-key',
    ),
  );
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Voice settings saved.'),
  );
  expect(stored.settings.whisper_model).toBe('small.en');
});
it('Download & select selects a newly installed model without silently saving other form changes', async () => {
  await setup();
  download('small.en');
  await vi.waitFor(() =>
    expect(field('Local transcription model').value).toBe('small.en'),
  );
  expect(api.saveSettings).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('Click Save settings to use it.');
});
it('re-downloading another installed model preserves the selected model', async () => {
  stored.models.forEach((m) => (m.installed = true));
  stored.settings.whisper_model = 'small.en';
  await setup();
  download('base.en');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('base.en re-downloaded'),
  );
  expect(field('Local transcription model').value).toBe('small.en');
});
it('failed download preserves form edits and does not mark the model installed', async () => {
  await setup();
  change(field('Local transcription model'), 'small.en');
  change(field('Model ID'), 'pending-model');
  api.download.mockRejectedValue(new Error('Download interrupted'));
  download('small.en');
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('Download interrupted'),
  );
  expect(field('Local transcription model').value).toBe('small.en');
  expect(field('Model ID').value).toBe('pending-model');
  expect(api.settings).toHaveBeenCalledTimes(1);
  expect(stored.models[1].installed).toBe(false);
});
