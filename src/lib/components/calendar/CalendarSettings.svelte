<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as api from '../../calendar/api';
  import { errorMessage, type ProjectRecord } from '../../domain/types';
  let { projects = [] }: { projects?: ProjectRecord[] } = $props();
  let data = $state<api.CalendarStatus | null>(null),
    error = $state(''),
    notice = $state(''),
    busy = $state(false),
    waiting = $state(false);
  let timer: ReturnType<typeof setInterval> | undefined;
  async function load() {
    data = await api.status();
  }
  onMount(() => {
    void load().catch((e) => (error = errorMessage(e)));
    timer = setInterval(() => void load().catch(() => {}), 5000);
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
  const importClient = () =>
    run(async () => {
      if (await api.importClient()) notice = 'Google client file loaded.';
    });
  async function connect() {
    if (busy) return;
    busy = true;
    waiting = true;
    error = '';
    notice = '';
    try {
      const email = await api.connect();
      notice = `Connected ${email}. Choose which calendars to bring in below.`;
      await load();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
      waiting = false;
    }
  }
  const choose = (a: api.Account, c: api.Calendar, projectId: string) => {
    const name = a.kind === 'outlook' ? a.email : c.name;
    return run(
      () => api.setProject(a.id, c.id, projectId || null),
      projectId
        ? `${name} meetings will appear in ${projects.find((p) => p.id === projectId)?.name ?? 'that project'}.`
        : `Stopped following ${name}. Meetings already brought in stay in Signal.`,
    );
  };
  const syncNow = () =>
    run(async () => {
      const r = await api.syncNow();
      const parts = [
        r.created && `${r.created} added`,
        r.updated && `${r.updated} updated`,
        r.removed && `${r.removed} removed`,
        r.canceled && `${r.canceled} marked canceled`,
      ].filter(Boolean);
      notice = parts.length
        ? `Synced: ${parts.join(', ')}.`
        : 'Synced. Nothing changed.';
      if (r.errors.length) error = r.errors.join(' ');
    });
  const setMail = (a: api.Account, on: boolean) =>
    run(
      () => api.setMail(a.id, on),
      on
        ? `Email from ${a.email} will be suggested as tasks. Signal checks every 15 minutes while it is open; see AI Inbox → Email.`
        : `Stopped suggesting tasks from ${a.email}.`,
    );
  const setAutoAdd = (a: api.Account, on: boolean) =>
    run(
      () => api.setAutoAdd(a.id, on),
      on
        ? `Sure tasks from ${a.email} will be added by themselves, tagged From email. Everything else still waits for review.`
        : `Tasks from ${a.email} will wait for your review.`,
    );
  const findOutlook = () =>
    run(async () => {
      const r = await api.addOutlook();
      notice = r.added.length
        ? `Added ${r.added.join(', ')}. Choose a project for each calendar below.`
        : `Outlook has ${r.found} account${r.found === 1 ? '' : 's'}; all are already here.`;
    });
  const when = (iso: string | null) =>
    iso
      ? new Date(iso).toLocaleString(undefined, {
          weekday: 'short',
          hour: 'numeric',
          minute: '2-digit',
        })
      : 'not yet';
  const google = $derived(data?.accounts.filter((a) => a.kind !== 'outlook') ?? []);
  const outlook = $derived(data?.accounts.filter((a) => a.kind === 'outlook') ?? []);
  const following = $derived(
    data?.accounts.some((a) => a.calendars.some((c) => c.project_id)) ?? false,
  );
</script>

{#snippet account(a: api.Account)}
  <article class="account">
    <header>
      <strong>{a.email}</strong>
      <span class="muted">Last synced {when(a.status.last_sync)}</span>
    </header>
    {#if a.status.last_error}<p role="alert" class="error">
        {a.status.last_error}
      </p>{/if}
    {#if a.status.needs_sign_in}<button
        class="primary"
        disabled={busy}
        onclick={connect}>Connect {a.email} again</button
      >{/if}
    <ul>
      {#each a.calendars as c (c.id)}
        <li>
          <span class="swatch" style:background={c.color}></span>
          <span class="name"
            >{c.name}{#if c.primary}<span class="muted"> · main</span
              >{/if}</span
          >
          <select
            aria-label={`Project for ${a.kind === 'outlook' ? `${a.email} calendar` : c.name}`}
            value={c.project_id ?? ''}
            disabled={busy}
            onchange={(e) => choose(a, c, e.currentTarget.value)}
            ><option value="">Don't bring in</option
            >{#each projects as p (p.id)}<option value={p.id}
                >{p.name}</option
              >{/each}</select
          >
        </li>
      {/each}
    </ul>
    {#if a.mail}
      <label class="mail">
        <input
          type="checkbox"
          checked={a.mail_tasks}
          disabled={busy}
          onchange={(e) => setMail(a, e.currentTarget.checked)}
        />
        <span
          >Suggest tasks from this inbox<span class="muted">
            · sends new email to your AI provider (Settings → Voice & AI)</span
          ></span
        >
      </label>
      <label class="mail auto">
        <input
          type="checkbox"
          checked={a.auto_add}
          disabled={busy || !a.mail_tasks}
          onchange={(e) => setAutoAdd(a, e.currentTarget.checked)}
        />
        <span
          >Add sure tasks automatically<span class="muted">
            · a person wrote to you directly, asks for something specific and
            it matches a project; Undo is in AI Inbox → Email</span
          ></span
        >
      </label>
    {:else}
      <p>
        To suggest tasks from this inbox, connect it again and allow Signal
        to read email on Google's screen.
      </p>
      <button class="outline" disabled={busy} onclick={connect}
        >Connect {a.email} again to allow email</button
      >
    {/if}
    <button
      class="outline danger"
      disabled={busy}
      onclick={() =>
        run(
          () => api.removeAccount(a.id),
          `Removed ${a.email}. Meetings already brought in stay in Signal.`,
        )}>Remove account</button
    >
  </article>
{/snippet}

<section class="calendars">
  <p>
    Bring meetings from your Google and Outlook calendars into Signal. Signal only reads
    your calendars, never changes them, and checks every 10 minutes while it
    is running. Google or Outlook decides each meeting's title, time, link and
    description; your project, notes, attendance and linked tasks stay yours.
  </p>
  <p>
    Signal can also suggest tasks from new email. It only reads your email and
    never sends, moves, labels or deletes anything. Turn it on per account
    below; suggestions appear in AI Inbox → Email for you to review. With
    “Add sure tasks automatically” on, tasks a person asks of you directly about
    one of your projects are added without review, and you can undo them there.
  </p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if data}
    <h2>1 · Google client file</h2>
    {#if data.has_client}
      <p>
        Loaded. It is stored for this Windows user and this copy of Signal only.
      </p>
      <button class="outline" disabled={busy} onclick={importClient}
        >Replace client file</button
      >
    {:else}
      <p>
        Choose the <code>client_secret_….json</code> file you downloaded from
        Google Cloud Console (Desktop app client).
      </p>
      <button class="primary" disabled={busy} onclick={importClient}
        >Choose client file</button
      >
    {/if}

    <h2>2 · Google accounts</h2>
    {#each google as a (a.id)}{@render account(a)}{/each}
    {#if waiting}
      <p role="status">
        Finish signing in in your browser. Google may say Signal is unverified;
        choose <strong>Advanced</strong> → <strong>Go to Signal</strong>, then
        allow calendar access. To get task suggestions from email, also tick
        the box to read your email.
      </p>
      <button class="outline" onclick={() => void api.cancelConnect()}
        >Cancel sign-in</button
      >
    {:else}
      <button
        class={google.length ? 'outline' : 'primary'}
        disabled={busy || !data.has_client}
        onclick={connect}>Connect a Google account</button
      >
    {/if}

    <h2>3 · Outlook on this computer</h2>
    <p>
      Signal reads the accounts set up in classic Outlook on this computer,
      starting Outlook in the background when it needs to. The new Outlook
      app can't be read. Open email shows the message in Outlook.
    </p>
    {#each outlook as a (a.id)}{@render account(a)}{/each}
    <button
      class={outlook.length ? 'outline' : 'primary'}
      disabled={busy}
      onclick={findOutlook}>Find Outlook accounts</button
    >

    <h2>4 · Sync</h2>
    <p>
      Signal brings in meetings from the last {data.past_days} days and the next
      {data.future_days} days. All-day events, declined invitations and out-of-office
      blocks are skipped. If a meeting is cancelled in Google or Outlook, Signal removes it,
      or marks it canceled if you added notes, attendance or tasks.
    </p>
    <button
      class="outline"
      disabled={busy || !following}
      onclick={syncNow}>Sync now</button
    >
  {:else}<p role="status">Loading calendar settings…</p>{/if}
</section>

<style>
  .calendars {
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
  button {
    justify-self: start;
  }
  .account {
    display: grid;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    border-radius: 8px;
  }
  .account header {
    display: flex;
    gap: 12px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .muted {
    color: var(--text-muted);
    font-size: 0.9em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: grid;
    grid-template-columns: 12px 1fr minmax(160px, 220px);
    gap: 10px;
    align-items: center;
  }
  .mail {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .mail.auto {
    padding-left: 24px;
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
