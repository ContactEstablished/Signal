<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as api from '../../telegram/api';
  import { errorMessage } from '../../domain/types';
  let {
    onEnableTray,
  }: {
    onEnableTray: () => Promise<void>;
  } = $props();
  let data = $state<api.TelegramStatus | null>(null),
    token = $state(''),
    error = $state(''),
    notice = $state(''),
    busy = $state(false),
    pairing = $state<api.PairingStart | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;
  async function load() {
    const next = await api.status();
    // A finished or expired pairing hides its code.
    if (pairing && (next.paired || !next.pairing.active)) pairing = null;
    data = next;
  }
  onMount(() => {
    void load().catch((e) => (error = errorMessage(e)));
    timer = setInterval(() => void load().catch(() => {}), 2000);
  });
  onDestroy(() => clearInterval(timer));
  async function run(work: () => Promise<unknown>, done = '') {
    if (busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      await work();
      await load();
      if (done) notice = done;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const saveToken = () =>
    run(async () => {
      const bot = await api.saveToken(token.trim());
      token = '';
      notice = `Connected to @${bot.bot_username}.`;
    });
  const setEnabled = (enabled: boolean) =>
    run(
      async () => {
        if (enabled && data && !data.tray) await onEnableTray();
        await api.setEnabled(enabled);
      },
      enabled
        ? 'Signal is now listening for your Telegram messages.'
        : 'Signal stopped listening.',
    );
  const startPairing = () =>
    run(async () => {
      pairing = await api.startPairing();
    });
  const minutes = $derived(
    data ? Math.max(1, Math.ceil(data.pairing.seconds_left / 60)) : 1,
  );
</script>

<section class="telegram">
  <p>
    Message your own private Telegram bot to add tasks and meetings, finish a
    task with a note, or cancel a meeting. Signal always shows what it
    understood and waits for you to tap <strong>Confirm</strong> before saving
    anything. Only your paired Telegram account can use the bot.
  </p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if data}
    <h2>1 · Your bot</h2>
    {#if data.has_token}
      <p>
        Connected to <strong>@{data.bot_username}</strong>. The token is stored
        securely for this Windows user and this copy of Signal only.
      </p>
      <button
        class="outline"
        disabled={busy}
        onclick={() => run(api.removeBot, 'Bot removed.')}>Remove bot</button
      >
    {:else}
      <p>
        In Telegram, message <strong>@BotFather</strong>, send
        <code>/newbot</code>, and follow the steps. Paste the token it gives you
        here. Use a different bot for development and for your personal copy.
      </p>
      <label
        >Bot token<input
          type="password"
          bind:value={token}
          autocomplete="off"
          placeholder="123456789:AA…"
          disabled={busy}
        /></label
      >
      <button
        class="primary"
        disabled={busy || !token.trim()}
        onclick={saveToken}>Save token</button
      >
    {/if}

    <h2>2 · AI use</h2>
    <label class="consent"
      ><input
        type="checkbox"
        checked={data.consent}
        disabled={busy}
        onchange={(e) => run(() => api.setConsent(e.currentTarget.checked))}
      />I agree to send the text of my Telegram messages, the transcripts of my
      voice notes, and my project names to the AI provider configured in Voice
      &amp; AI so it can work out what to add or change. Voice recordings stay on
      this computer. Provider usage charges may apply. Messages also pass
      through Telegram.</label
    >

    <h2>3 · Listening</h2>
    <label class="consent"
      ><input
        type="checkbox"
        checked={data.enabled}
        disabled={busy || !data.has_token || !data.consent}
        onchange={(e) => setEnabled(e.currentTarget.checked)}
      />Listen for my Telegram messages</label
    >
    <p>
      Signal answers only while it is running and this computer is awake and
      online. Turning this on also keeps Signal running in the system tray when
      you close the window; use the tray icon → Quit to exit completely.
      Messages sent while Signal is closed wait at Telegram for up to 24 hours.
    </p>
    {#if data.enabled && !data.tray}
      <p role="alert" class="error">
        The tray option is off, so closing the window will quit Signal and stop
        the bot.
      </p>
      <button
        class="outline"
        disabled={busy}
        onclick={() => run(onEnableTray)}
        >Keep Signal running in the tray</button
      >
    {/if}
    {#if data.enabled}
      <p role="status" class="state">
        {#if data.worker.connected}Connected to Telegram.{:else if data.worker.last_error}Not
          connected: {data.worker.last_error}{:else}Connecting…{/if}
      </p>
    {/if}

    <h2>4 · Pair your account</h2>
    {#if data.paired}
      <p>Your Telegram account is paired. Everyone else is ignored.</p>
      <button
        class="outline"
        disabled={busy}
        onclick={() =>
          run(api.unpair, 'Unpaired. Pending requests were discarded.')}
        >Unpair</button
      >
    {:else if pairing && data.pairing.active}
      <p>
        Open <strong>@{pairing.bot_username}</strong> in Telegram and send this
        message within {minutes} minute{minutes === 1 ? '' : 's'}:
      </p>
      <p class="code" aria-label="Pairing message">/start {pairing.code}</p>
      {#if data.pairing.failures}<p role="alert" class="error">
          {data.pairing.failures} wrong attempt{data.pairing.failures === 1
            ? ''
            : 's'}. The code stops working after 5.
        </p>{/if}
      <button
        class="outline"
        disabled={busy}
        onclick={() => run(api.cancelPairing)}>Cancel pairing</button
      >
    {:else}
      {#if data.pairing.locked}<p role="alert" class="error">
          Too many wrong attempts. Start pairing again for a new code.
        </p>{/if}
      <button
        class="primary"
        disabled={busy || !data.enabled}
        onclick={startPairing}>Pair my Telegram account</button
      >
      {#if !data.enabled}<p>Turn on listening first.</p>{/if}
    {/if}
  {:else}<p role="status">Loading Telegram settings…</p>{/if}
</section>

<style>
  .telegram {
    max-width: 680px;
    display: grid;
    gap: 12px;
  }
  p {
    color: var(--text-muted);
    line-height: 1.5;
    margin: 0;
  }
  h2 {
    font-size: var(--text-page-small);
    margin: 16px 0 0;
  }
  label {
    display: grid;
    gap: 8px;
  }
  input:not([type='checkbox']) {
    width: 100%;
  }
  .consent {
    display: flex;
    align-items: flex-start;
    line-height: 1.5;
    gap: 8px;
  }
  .consent input {
    width: auto;
    margin-top: 4px;
  }
  .code {
    font-family: var(--font-mono, monospace);
    font-size: 1.4em;
    letter-spacing: 0.05em;
  }
  button {
    justify-self: start;
  }
</style>
