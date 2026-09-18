<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Mic, Square, X, Check, Plus, ArrowRight } from 'lucide-svelte';
  import * as api from '../voice/api';
  import { record, type Recording } from '../voice/capture';
  import {
    errorMessage,
    projectColors,
    type CloseReason,
  } from '../domain/types';
  import type { Workspace } from '../state/app.svelte';
  import { dueFields, resolveTaskDate } from '../domain/agenda-calendar';
  let {
    workspace,
    onOpenTask,
    onSettings,
    onGuard,
  }: {
    workspace: Workspace;
    onOpenTask: (id: string) => unknown;
    onSettings: () => unknown;
    onGuard: (active: boolean) => void;
  } = $props();
  let draft = $state<api.Draft | null>(null),
    ready = $state(false),
    phase = $state<
      'idle' | 'recording' | 'transcribing' | 'suggesting' | 'accepting'
    >('idle'),
    error = $state(''),
    notice = $state(''),
    level = $state(0),
    seconds = $state(0),
    suggestionSeconds = $state(0),
    unsaved = $state(false),
    saving = $state(false),
    pending = $state<api.Acceptance | null>(null),
    created = $state<api.Receipt | null>(null);
  let recording: Recording | null = null,
    disposed = false,
    generation = 0,
    edits = 0,
    saveChain = Promise.resolve(),
    timer: ReturnType<typeof setTimeout> | undefined;
  let pendingSave: api.Draft | null = null;
  let dateErrors = $state<string[]>([]);
  let confirmation = $state(''),
    answer: ((v: boolean) => void) | null = null;
  const busy = $derived(phase !== 'idle');
  $effect(() => {
    if (phase !== 'suggesting') return;
    suggestionSeconds = 0;
    const started = Date.now();
    const interval = setInterval(() => {
      suggestionSeconds = Math.floor((Date.now() - started) / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });
  const grouped = $derived.by(() => {
    const groups = new Map<string, api.Candidate[]>();
    for (const c of draft?.candidates ?? []) {
      const name = c.created_id
        ? 'Created tasks'
        : (workspace.projects.find((p) => p.id === c.project_id)?.name ??
          draft?.projects.find((p) => p.id === c.proposal_id)?.name ??
          'Choose a project');
      const list = groups.get(name) ?? [];
      list.push(c);
      groups.set(name, list);
    }
    return [...groups.entries()];
  });
  const selected = $derived(
    draft?.candidates.filter((c) => c.selected && !c.created_id) ?? [],
  );
  $effect(() =>
    onGuard(busy || unsaved || saving || !!pending || !!dateErrors.length),
  );
  function ask(message: string) {
    confirmation = message;
    return new Promise<boolean>((r) => (answer = r));
  }
  function decide(v: boolean) {
    confirmation = '';
    answer?.(v);
    answer = null;
  }
  function modal(node: HTMLDialogElement) {
    node.showModal();
    return {
      destroy() {
        node.close();
      },
    };
  }
  onMount(() => {
    void (async () => {
      try {
        draft = await api.load();
        pending = await api.recovery();
        workspace.voiceRecovery = !!pending;
        ready = true;
      } catch (e) {
        error = errorMessage(e);
      }
    })();
  });
  onDestroy(() => {
    disposed = true;
    generation++;
    clearTimeout(timer);
    if (recording) void recording.cancel();
    if (busy) void api.cancel();
    decide(false);
    onGuard(false);
  });
  function changed() {
    unsaved = true;
    edits++;
    clearTimeout(timer);
    timer = setTimeout(() => void persist(), 500);
  }
  async function persist() {
    clearTimeout(timer);
    saveChain = saveChain.then(async () => {
      if (!draft || !unsaved || pending) return;
      saving = true;
      const version = edits;
      try {
        if (pendingSave) {
          const recovered = await api.save(pendingSave);
          if (draft.id === recovered.id) draft.revision = recovered.revision;
          pendingSave = null;
        }
        pendingSave = $state.snapshot(draft);
        const saved = await api.save(pendingSave);
        pendingSave = null;
        if (draft?.id === saved.id) draft.revision = saved.revision;
        if (version === edits) unsaved = false;
      } catch (e) {
        if (
          e &&
          typeof e === 'object' &&
          'code' in e &&
          e.code === 'Validation'
        ) {
          // Validation ran before any write; retry the user's corrected fields.
          pendingSave = null;
        }
        error = `Could not save review: ${errorMessage(e)}`;
      } finally {
        saving = false;
      }
    });
    await saveChain;
  }
  export async function requestClose(_reason: CloseReason): Promise<boolean> {
    if (dateErrors.length) {
      error = 'Correct the invalid deadline before leaving.';
      return false;
    }
    if (pending) {
      error = 'Resolve the pending task creation using Retry before leaving.';
      return false;
    }
    if (busy) {
      if (
        !(await ask(
          'Stop this voice operation and leave? Your saved review will remain.',
        ))
      )
        return false;
      if (!(await cancel())) return false;
    }
    await persist();
    return !unsaved;
  }
  async function cancel() {
    generation++;
    const live = recording;
    recording = null;
    try {
      await live?.cancel();
      await api.cancel();
      phase = 'idle';
      level = 0;
      return true;
    } catch (e) {
      error = errorMessage(e);
      return false;
    }
  }
  async function begin() {
    if (busy || pending || !ready) return;
    if (draft) {
      error =
        'Finish or discard the current review before starting another recording.';
      return;
    }
    error = '';
    notice = '';
    phase = 'recording';
    seconds = 0;
    const epoch = ++generation;
    try {
      const config = await api.settings();
      if (disposed || epoch !== generation) return;
      if (
        !config.models.find((m) => m.id === config.settings.whisper_model)
          ?.installed
      ) {
        const installed = config.models.filter((m) => m.installed).map((m) => m.id);
        throw new Error(
          `Selected transcription model ${config.settings.whisper_model} is not installed. ` +
          (installed.length
            ? `${installed.join(', ')} is installed. Select it in Settings → Voice & AI and click Save settings.`
            : 'Download and select a model in Settings → Voice & AI, then click Save settings.'),
        );
      }
      draft = api.emptyDraft(workspace.nowUtc, workspace.timeZone);
      changed();
      await persist();
      if (unsaved) throw new Error('Save the draft before recording.');
      if (disposed || epoch !== generation) return;
      const live = await record(
        config.settings.microphone,
        (n, s) => {
          level = n;
          seconds = s;
        },
        () => void stop(),
        (e) => {
          error = errorMessage(e);
          void cancel();
        },
      );
      if (epoch !== generation) {
        await live.cancel();
        return;
      }
      recording = live;
    } catch (e) {
      if (!disposed && epoch === generation) {
        error = errorMessage(e);
        phase = 'idle';
      }
    }
  }
  async function stop() {
    if (!recording || phase !== 'recording') return;
    const live = recording;
    recording = null;
    phase = 'transcribing';
    const epoch = generation;
    try {
      const text = await live.stop();
      if (disposed || epoch !== generation) return;
      draft!.transcript = text;
      changed();
      await persist();
      if (disposed || epoch !== generation) return;
      phase = 'idle';
      level = 0;
      if (text.trim()) await analyze(false);
      else
        notice =
          'No speech was detected. Discard this empty draft to record again, or enter a transcript.';
    } catch (e) {
      if (epoch === generation) {
        error = errorMessage(e);
        phase = 'idle';
      }
    }
  }
  async function analyze(confirm = true) {
    if (!draft || busy || pending) return;
    if (
      confirm &&
      draft.candidates.some((c) => !c.created_id) &&
      !(await ask(
        'Regenerate recommendations? Unaccepted task edits will be replaced.',
      ))
    )
      return;
    const epoch = ++generation;
    await persist();
    if (unsaved || disposed || epoch !== generation) return;
    phase = 'suggesting';
    error = '';
    try {
      const result = await api.suggest($state.snapshot(draft));
      if (disposed || epoch !== generation) return;
      draft = result;
      dateErrors = [];
      changed();
      await persist();
      notice = result.candidates.length
        ? 'Review every project and task before creating.'
        : 'No tasks found. Edit the transcript or add a task manually.';
    } catch (e) {
      if (epoch === generation) error = errorMessage(e);
    } finally {
      if (epoch === generation) phase = 'idle';
    }
  }
  async function accept() {
    if (busy || !draft || dateErrors.length) return;
    error = '';
    const epoch = ++generation;
    await persist();
    if (unsaved || disposed || epoch !== generation) return;
    if (!pending) {
      if (!selected.length) return;
      pending = {
        request_id: crypto.randomUUID(),
        draft_id: draft.id,
        revision: draft.revision,
        candidate_ids: selected.map((c) => c.id),
      };
    }
    phase = 'accepting';
    workspace.voiceRecovery = true;
    try {
      await api.setRecovery($state.snapshot(pending));
      created = await workspace.acceptVoice($state.snapshot(pending));
      await api.setRecovery(null);
      pending = null;
      workspace.voiceRecovery = false;
      draft = await api.load();
      notice =
        'Selected tasks created in To Do. Unselected tasks remain in this review.';
    } catch (e) {
      error = errorMessage(e);
      if (
        e &&
        typeof e === 'object' &&
        'code' in e &&
        ['Validation', 'Conflict', 'NotFound', 'Database'].includes(
          String(e.code),
        )
      ) {
        try {
          await api.setRecovery(null);
          pending = null;
          workspace.voiceRecovery = false;
        } catch {
          error += ' Could not clear recovery; retry before leaving.';
        }
      }
    } finally {
      phase = 'idle';
    }
  }
  async function discard() {
    if (
      !draft ||
      busy ||
      pending ||
      !(await ask(
        'Discard this transcript and unaccepted suggestions? Created tasks will remain.',
      ))
    )
      return;
    await persist();
    if (unsaved) return;
    try {
      await api.discard($state.snapshot(draft));
      draft = null;
      created = null;
      dateErrors = [];
      notice = 'Review cleared. Created tasks remain in their projects.';
    } catch (e) {
      error = errorMessage(e);
    }
  }
  function assign(c: api.Candidate, v: string) {
    c.project_id = v.startsWith('project:') ? v.slice(8) : null;
    c.proposal_id = v.startsWith('new:') ? v.slice(4) : null;
    c.duplicate_ok = false;
    changed();
  }
  function addTask() {
    if (!draft) return;
    draft.candidates.push({
      id: crypto.randomUUID(),
      selected: true,
      project_id: null,
      proposal_id: null,
      title: '',
      notes: '',
      subtasks: [],
      due_at: null,
      priority: 'medium',
      estimate_h: null,
      source: '',
      warning: '',
      duplicate_ok: false,
      created_id: null,
    });
    changed();
  }
  function deadline(c: api.Candidate, value: string) {
    try {
      c.due_at = value
        ? resolveTaskDate(
            { date: value.slice(0, 10), time: value.slice(11) },
            draft!.time_zone,
          )
        : null;
      dateErrors = dateErrors.filter((id) => id !== c.id);
      changed();
    } catch (e) {
      dateErrors = [...new Set([...dateErrors, c.id])];
      error = errorMessage(e);
    }
  }
</script>

<main class="voice-page">
  <div class="heading">
    <div>
      <h1>Voice Inbox</h1>
      <p>
        Dictate your projects and tasks. Review first; create only what you
        choose.
      </p>
    </div>
    <button class="outline" onclick={onSettings}>Voice & AI settings</button>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if !ready}<p>Loading saved review…</p>{:else}
    <section class="capture">
      {#if phase === 'recording'}<button class="primary" onclick={stop}
          ><Square />Stop & analyze</button
        ><strong
          >{Math.floor(seconds / 60)}:{String(
            Math.floor(seconds % 60),
          ).padStart(2, '0')} / 5:00</strong
        ><meter aria-label="Microphone level" min="0" max="0.3" value={level}
        ></meter>
      {:else}<button
          class="primary"
          disabled={busy || !!pending || !!draft}
          onclick={begin}><Mic />Start recording</button
        >{/if}
      {#if busy}<span role="status"
          >{phase === 'recording'
            ? 'Listening…'
            : phase === 'transcribing'
              ? 'Transcribing locally…'
              : phase === 'suggesting'
                ? `Suggesting tasks… ${suggestionSeconds}s elapsed. This can take up to 90 seconds.`
                : 'Creating selected tasks…'}</span
        >{#if phase !== 'accepting'}<button class="outline" onclick={cancel}
            ><X />Cancel</button
          >{/if}{/if}
      {#if !draft && !busy}<button
          class="outline"
          onclick={() => {
            draft = api.emptyDraft(workspace.nowUtc, workspace.timeZone);
            changed();
          }}>Enter transcript manually</button
        >{/if}
      <small
        >Audio stays on this computer. Only the transcript and project names go
        to your configured AI provider.</small
      >
    </section>
    {#if pending}<section class="recovery">
        <strong>Task creation needs confirmation.</strong>
        <p>
          Retry checks the original request; it will not create the same tasks
          twice.
        </p>
        <button class="primary" disabled={busy} onclick={accept}
          >Retry task creation</button
        >
      </section>{/if}
    {#if draft}
      <div class="review">
        <section class="source">
          <h2>Transcript</h2>
          <textarea
            aria-label="Transcript"
            rows="16"
            bind:value={draft.transcript}
            oninput={changed}
            disabled={busy || !!pending}></textarea>
          <p>
            {draft.time_zone} · recorded {new Date(
              draft.recorded_at,
            ).toLocaleString()}
          </p>
          <button
            class="outline"
            disabled={busy || !!pending || !draft.transcript.trim()}
            onclick={() => analyze()}>{phase === 'suggesting' ? 'Suggesting tasks…' : 'Suggest tasks'}</button
          >
          <p aria-live="polite">
            {saving
              ? 'Saving review…'
              : unsaved
                ? 'Unsaved changes'
                : 'Review saved locally'}
          </p>
          {#if unsaved}<button class="outline" onclick={persist}
              >Retry saving</button
            >{/if}
        </section>
        <section class="suggestions">
          <h2>Task recommendations</h2>
          <fieldset disabled={busy || !!pending}>
            {#each draft.projects as p (p.id)}<section
                class="proposed"
                data-color={p.color}
              >
                <h3>
                  {p.created_id ? 'Created project' : 'Proposed new project'}
                </h3>
                <label
                  >Name<input
                    bind:value={p.name}
                    oninput={changed}
                    disabled={!!p.created_id}
                  /></label
                ><label
                  >Color<select
                    bind:value={p.color}
                    onchange={changed}
                    disabled={!!p.created_id}
                    >{#each projectColors as color}<option>{color}</option
                      >{/each}</select
                  ></label
                >{#if !p.created_id}<label class="inline"
                    ><input
                      type="checkbox"
                      bind:checked={p.approved}
                      onchange={changed}
                    />Approve creating this project when accepting its tasks</label
                  >{/if}
              </section>{/each}
            <div class="actions">
              <button
                class="outline"
                onclick={() => {
                  draft!.candidates.forEach((c) => {
                    if (!c.created_id) c.selected = true;
                  });
                  changed();
                }}>Select all</button
              ><button
                class="outline"
                onclick={() => {
                  draft!.candidates.forEach((c) => (c.selected = false));
                  changed();
                }}>Select none</button
              ><button class="outline" onclick={addTask}
                ><Plus />Add task</button
              ><button
                class="outline"
                onclick={() => {
                  draft!.projects.push({
                    id: crypto.randomUUID(),
                    name: '',
                    color: 'cyan',
                    approved: false,
                    created_id: null,
                  });
                  changed();
                }}>New project proposal</button
              >
            </div>
            {#each grouped as [name, candidates] (name)}<h3 class="group-title">
                {name || 'Unnamed project'}
              </h3>
              {#each candidates as c (c.id)}
                <article class:created={!!c.created_id}>
                  {#if c.created_id}<strong><Check />Created: {c.title}</strong
                    ><button
                      class="outline"
                      onclick={() => onOpenTask(c.created_id!)}
                      >Open task <ArrowRight /></button
                    >
                  {:else}
                    <label class="inline"
                      ><input
                        type="checkbox"
                        bind:checked={c.selected}
                        onchange={changed}
                      /><strong>Include task · To Do</strong></label
                    >
                    <label
                      >Project<select
                        value={c.project_id
                          ? 'project:' + c.project_id
                          : c.proposal_id
                            ? 'new:' + c.proposal_id
                            : ''}
                        onchange={(e) => assign(c, e.currentTarget.value)}
                        ><option value="">Choose a project…</option
                        >{#each workspace.projects as p}<option
                            value={'project:' + p.id}
                            >{p.name} · {p.color}</option
                          >{/each}{#each draft.projects as p}<option
                            value={'new:' + p.id}
                            >{p.name || 'Unnamed project'}
                            {p.created_id ? '' : '(new)'}</option
                          >{/each}</select
                      ></label
                    >
                    <label
                      >Title<input
                        bind:value={c.title}
                        oninput={() => {
                          c.duplicate_ok = false;
                          changed();
                        }}
                        maxlength="500"
                      /></label
                    >
                    <label
                      >Notes<textarea
                        bind:value={c.notes}
                        oninput={changed}
                        rows="3"></textarea></label
                    >
                    <label
                      >Subtasks · one per line<textarea
                        value={c.subtasks.join('\n')}
                        oninput={(e) => {
                          c.subtasks = e.currentTarget.value.split('\n');
                          changed();
                        }}
                        rows="2"></textarea></label
                    >
                    <div class="task-fields">
                      <label
                        >Due · {draft.time_zone}<input
                          type="datetime-local"
                          step="0.001"
                          value={c.due_at
                            ? `${dueFields(c.due_at, draft.time_zone).date}T${dueFields(c.due_at, draft.time_zone).time}`
                            : ''}
                          onchange={(e) => deadline(c, e.currentTarget.value)}
                        /></label
                      ><label
                        >Priority<select
                          bind:value={c.priority}
                          onchange={changed}
                          ><option>low</option><option>medium</option><option
                            >high</option
                          ></select
                        ></label
                      ><label
                        >Estimate · hours<input
                          type="number"
                          min="0"
                          step="0.25"
                          value={c.estimate_h ?? ''}
                          oninput={(e) => {
                            c.estimate_h =
                              e.currentTarget.value === ''
                                ? null
                                : Number(e.currentTarget.value);
                            changed();
                          }}
                        /></label
                      >
                    </div>
                    {#if c.source}<blockquote>
                        {c.source}
                      </blockquote>{/if}{#if c.warning}<p class="warning">
                        {c.warning}
                      </p>{/if}
                    <label class="inline"
                      ><input
                        type="checkbox"
                        bind:checked={c.duplicate_ok}
                        onchange={changed}
                      />Allow an identical title in this project if intentional</label
                    >
                  {/if}
                </article>
              {/each}{/each}
            {#if !draft.candidates.length}<p>
                Recommendations will appear here. You can also add and edit
                tasks manually.
              </p>{/if}
            <div class="actions footer">
              <button
                class="primary"
                disabled={!selected.length ||
                  unsaved ||
                  saving ||
                  !!dateErrors.length}
                onclick={accept}
                ><Check />Create selected ({selected.length})</button
              ><button class="outline" onclick={discard}
                >{draft.candidates.length &&
                draft.candidates.every((c) => c.created_id)
                  ? 'Finish review'
                  : 'Discard draft'}</button
              >
            </div>
          </fieldset>
        </section>
      </div>
    {/if}
    {#if created}<p role="status">
        Created {created.created.length} tasks in To Do.
      </p>{/if}
  {/if}
  {#if confirmation}<dialog
      class="confirm"
      use:modal
      oncancel={() => decide(false)}
      aria-label="Confirm voice action"
    >
      <p>{confirmation}</p>
      <div class="actions">
        <button class="primary" onclick={() => decide(true)}>Continue</button
        ><button class="outline" onclick={() => decide(false)}
          >Keep editing</button
        >
      </div>
    </dialog>{/if}
</main>

<style>
  .voice-page {
    padding: 24px;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  .heading {
    display: flex;
    justify-content: space-between;
    gap: 24px;
    align-items: center;
  }
  h1 {
    font-family: var(--font-heading);
    font-size: var(--text-title);
  }
  h2 {
    font-size: var(--text-page-small);
  }
  h3 {
    font-size: var(--text-card);
  }
  p,
  small {
    color: var(--text-muted);
    line-height: 1.5;
  }
  .capture {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 16px;
    background: var(--surface);
    padding: 20px;
    border-radius: var(--radius-panel);
    margin: 20px 0;
  }
  .capture small {
    width: 100%;
  }
  .review {
    display: grid;
    grid-template-columns: minmax(260px, 1fr) minmax(480px, 2fr);
    gap: 24px;
  }
  .source textarea {
    width: 100%;
  }
  fieldset {
    border: 0;
    padding: 0;
    min-width: 0;
  }
  label {
    display: grid;
    gap: 6px;
    margin: 12px 0;
  }
  input,
  textarea,
  select {
    width: 100%;
  }
  textarea {
    resize: vertical;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .inline input {
    width: auto;
  }
  .task-fields {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr;
    gap: 12px;
  }
  article,
  .proposed,
  .recovery {
    padding: 18px;
    margin: 16px 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .proposed {
    border-color: var(--project-color);
  }
  .created {
    opacity: 0.75;
  }
  .warning {
    color: var(--warn);
  }
  .error {
    color: var(--danger);
  }
  blockquote {
    border-left: 2px solid var(--border);
    padding-left: 12px;
    color: var(--text-muted);
    margin: 16px 0;
  }
  .footer {
    position: sticky;
    bottom: 0;
    background: var(--bg);
    padding: 16px 0;
  }
  .confirm::backdrop {
    background: var(--scrim);
  }
  .confirm {
    color: var(--text);
    max-width: 480px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: 24px;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
