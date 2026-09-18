<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import * as api from '../../voice/api';
  import { errorMessage } from '../../domain/types';
  let data = $state<api.SettingsReply | null>(null),
    key = $state(''),
    error = $state(''),
    notice = $state(''),
    busy = $state(false),
    progress = $state(0);
  let devices = $state<MediaDeviceInfo[]>([]),
    unlisten: (() => void) | undefined;
  let disposed = false,
    downloading = $state(false);
  async function load() {
    data = await api.settings();
  }
  onMount(() => {
    void load().catch((e) => (error = errorMessage(e)));
    void listen<{ received: number; total: number }>(
      'signal://voice-download',
      (e) => (progress = e.payload.received / e.payload.total),
    ).then((fn) => (disposed ? fn() : (unlisten = fn)));
  });
  onDestroy(() => {
    disposed = true;
    unlisten?.();
    if (busy) void api.cancel();
  });
  async function microphones() {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      stream.getTracks().forEach((t) => t.stop());
      devices = (await navigator.mediaDevices.enumerateDevices()).filter(
        (d) => d.kind === 'audioinput',
      );
    } catch {
      error =
        'Microphone access was denied or no input is available. Check Windows microphone permissions.';
    }
  }
  async function save() {
    if (!data || busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      await api.saveSettings(
        $state.snapshot(data.settings),
        key.trim() ? key : null,
      );
      key = '';
      await load();
      notice = 'Voice settings saved.';
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function download(id: string) {
    if (busy || !data) return;
    const selectAfterDownload = !data.models.find((m) => m.id === id)?.installed;
    busy = true;
    downloading = true;
    progress = 0;
    error = '';
    try {
      await api.download(id);
      const refreshed = await api.settings();
      // Refresh file status only: the form may contain an unsaved model,
      // microphone, provider, consent choice or key.
      data.models = refreshed.models;
      data.has_key = refreshed.has_key;
      if (selectAfterDownload) data.settings.whisper_model = id;
      notice = selectAfterDownload
        ? `${id} installed and selected. Click Save settings to use it.`
        : `${id} re-downloaded. Your model selection is unchanged.`;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
      downloading = false;
    }
  }
</script>

<section class="voice-settings">
  <p>
    Transcription happens on this computer. Task suggestions use the AI provider
    you configure here. Chorus settings and credentials are separate.
  </p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if data}
    <fieldset disabled={busy}>
      <label
        >Microphone<select bind:value={data.settings.microphone}
          ><option value="">System default</option
          >{#if data.settings.microphone && !devices.some((d) => d.deviceId === data!.settings.microphone)}<option
              value={data.settings.microphone}>Saved microphone</option
            >{/if}{#each devices as d}<option value={d.deviceId}
              >{d.label || 'Microphone'}</option
            >{/each}</select
        ></label
      >
      <button class="outline" onclick={microphones}
        >Choose from available microphones</button
      >
      <label
        >Local transcription model<select
          bind:value={data.settings.whisper_model}
          ><option value="base.en">Base English · smaller and faster</option
          ><option value="small.en">Small English · higher accuracy</option
          ></select
        ></label
      >
      {#each data.models as m}<div class="model">
          <span
            >{m.id} · {Math.round(m.bytes / 1e6)} MB · {m.installed
              ? 'Installed'
              : 'Not installed'}</span
          ><button class="outline" onclick={() => download(m.id)}
            >{m.installed ? 'Re-download' : 'Download & select'}</button
          >
        </div>{/each}
      <p>Selected for recording: <strong>{data.settings.whisper_model}</strong>.
        {#if !data.models.find((m) => m.id === data!.settings.whisper_model)?.installed}
          Download this model or select an installed model above.
        {/if}
        Click Save settings after changing the selection.</p>
      <h2>AI provider</h2>
      <label
        >HTTPS API URL<input
          type="url"
          bind:value={data.settings.endpoint}
          oninput={() => {
            data!.settings.consent = false;
          }}
          placeholder="https://your-provider.example/v1"
        /></label
      >
      <p>Enter the provider's base URL or its full <code>/chat/completions</code>
        URL. Signal adds the endpoint only when it is missing.</p>
      <label
        >Model ID<input
          bind:value={data.settings.model}
          placeholder="Enter your provider’s model ID"
        /></label
      >
      <label
        >API key<input
          type="password"
          bind:value={key}
          autocomplete="off"
          placeholder={data.has_key
            ? 'Saved securely · leave blank to keep'
            : 'Enter API key'}
        /></label
      >
      {#if data.has_key}<button
          class="outline"
          onclick={async () => {
            try {
              await api.saveSettings($state.snapshot(data!.settings), '');
              await load();
              notice = 'API key removed.';
            } catch (e) {
              error = errorMessage(e);
            }
          }}>Remove saved key</button
        >{/if}
      <label class="consent"
        ><input type="checkbox" bind:checked={data.settings.consent} />I agree
        to send my transcript and project names to this provider for task
        suggestions. Provider usage charges may apply. Audio stays local.</label
      >
      <button class="primary" onclick={save}>Save settings</button>
    </fieldset>
    {#if busy}<p role="status">
        Working… {progress ? `${Math.round(progress * 100)}% downloaded` : ''}
      </p>
      {#if downloading}<button class="outline" onclick={() => api.cancel()}
          >Cancel download</button
        >{/if}{/if}
  {:else}<p role="status">Loading voice settings…</p>{/if}
</section>

<style>
  .voice-settings {
    max-width: 680px;
  }
  p {
    color: var(--text-muted);
    line-height: 1.5;
  }
  fieldset {
    border: 0;
    padding: 0;
    display: grid;
    gap: 16px;
  }
  label {
    display: grid;
    gap: 8px;
  }
  input,
  select {
    width: 100%;
  }
  .consent {
    display: flex;
    align-items: flex-start;
    line-height: 1.5;
  }
  .consent input {
    width: auto;
    margin-top: 4px;
  }
  .model {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  h2 {
    font-size: var(--text-page-small);
    margin-top: 16px;
  }
  button {
    justify-self: start;
  }
</style>
