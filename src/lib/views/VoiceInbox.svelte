<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { Mic, Square, X, Check, Plus, ArrowRight, ClipboardPaste, Mail, ExternalLink, ChevronRight, Trash2 } from 'lucide-svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import * as voiceApi from '../voice/api';
  import * as imageApi from '../voice/image-api';
  import * as emailApi from '../voice/email-api';
  import * as calendarApi from '../calendar/api';
  import { openExternalUrl } from '../native/commands';
  import MeetingCandidate from '../components/voice/MeetingCandidate.svelte';
  import type { MeetingRef } from '../domain/agenda';
  import { record, type Recording } from '../voice/capture';
  import {
    errorMessage,
    projectColors,
    type CloseReason,
  } from '../domain/types';
  import type { Workspace } from '../state/app.svelte';
  import { meetingIssue, emailSummary, autoSummary } from '../voice/review';
  import { dueFields, resolveTaskDate } from '../domain/agenda-calendar';
  let {
    workspace,
    mode = 'voice',
    onOpenMeeting = () => {},
    onOpenTask,
    onSettings,
    onGuard,
  }: {
    workspace: Workspace;
    mode?: 'voice' | 'image' | 'email';
    onOpenMeeting?: (ref: MeetingRef) => unknown;
    onOpenTask: (id: string) => unknown;
    /** Opens a settings page; Voice & AI when no page is given. */
    onSettings: (page?: string) => unknown;
    onGuard: (active: boolean) => void;
  } = $props();
  const isImage = untrack(() => mode === 'image');
  const isEmail = untrack(() => mode === 'email');
  const api = isImage ? { ...voiceApi, ...imageApi } : isEmail ? { ...voiceApi, ...emailApi } : voiceApi;
  // Image and email suggestions never invent projects; each item needs one chosen.
  const needsProject = isImage || isEmail;
  /** Inboxes that suggest tasks; null until loaded. */
  let inboxes = $state<string[] | null>(null);
  function flagRecovery(on: boolean) {
    if (isImage) workspace.imageRecovery = on;
    else if (isEmail) workspace.emailRecovery = on;
    else workspace.voiceRecovery = on;
  }
  let imageUrl = $state(''), zoom = $state(false);
  let fileInput = $state<HTMLInputElement>();
  let draft = $state<voiceApi.Draft | null>(null),
    ready = $state(false),
    phase = $state<
      'idle' | 'uploading' | 'recording' | 'transcribing' | 'suggesting' | 'accepting' | 'undoing'
    >('idle'),
    error = $state(''),
    notice = $state(''),
    level = $state(0),
    seconds = $state(0),
    suggestionSeconds = $state(0),
    unsaved = $state(false),
    saving = $state(false),
    pending = $state<voiceApi.Acceptance | null>(null),
    created = $state<voiceApi.Receipt | null>(null);
  let recording: Recording | null = null,
    disposed = false,
    generation = 0,
    edits = 0,
    saveChain = Promise.resolve(),
    timer: ReturnType<typeof setTimeout> | undefined;
  let unlistenDrop: (() => void) | undefined;
  let unlistenChecked: (() => void) | undefined;
  /** The last automatic email check, and whether the review on screen is older than it. */
  let lastAuto = $state<emailApi.AutoStatus | null>(null),
    behind = $state(false);
  let pendingSave: voiceApi.Draft | null = null;
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
  const AUTO = 'Added automatically';
  const grouped = $derived.by(() => {
    const groups = new Map<string, voiceApi.Candidate[]>();
    for (const c of draft?.candidates ?? []) {
      const name = c.created_id
        ? c.auto ? AUTO : 'Created items'
        : (workspace.projects.find((p) => p.id === c.project_id)?.name ??
          draft?.projects.find((p) => p.id === c.proposal_id)?.name ??
          'Choose a project');
      const list = groups.get(name) ?? [];
      list.push(c);
      groups.set(name, list);
    }
    // Tasks Signal added by itself come first, where Undo is easy to find.
    return [...groups.entries()].sort(([a], [b]) => Number(b === AUTO) - Number(a === AUTO));
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
    if (isImage) void imageApi.subscribeDrop(paths => {
      if (workspace.editor || busy || pending || !ready) return;
      if (paths.length !== 1) { error = 'Choose one image per review.'; return; }
      void replaceImage(paths[0]);
    }).then(fn => disposed ? fn() : unlistenDrop = fn).catch(e => error = errorMessage(e));
    if (isEmail) void emailApi.onChecked(s => {
      lastAuto = s;
      behind = true;
    }).then(fn => disposed ? fn() : unlistenChecked = fn).catch(() => {});
    void (async () => {
      try {
        draft = await api.load();
        pending = await api.recovery();
        flagRecovery(!!pending);
        if (isEmail) {
          inboxes = (await calendarApi.status()).accounts.filter(a => a.mail && a.mail_tasks).map(a => a.email);
          lastAuto = await emailApi.autoStatus();
        }
        if (isImage && draft?.image_id) imageUrl = await imageApi.image(draft.image_id);
        ready = true;
      } catch (e) {
        error = errorMessage(e);
      }
    })();
  });
  onDestroy(() => {
    disposed = true;
    unlistenDrop?.();
    unlistenChecked?.();
    generation++;
    clearTimeout(timer);
    if (recording) void recording.cancel();
    if (busy) void api.cancel();
    decide(false);
    onGuard(false);
  });
  // After an automatic check, show its results once nothing on screen is mid-edit.
  $effect(() => {
    if (!behind || !ready || busy || unsaved || saving || pending) return;
    behind = false;
    const epoch = generation, edited = edits;
    void emailApi.load().then(d => {
      if (disposed || epoch !== generation || edited !== edits || unsaved) { behind = true; return; }
      draft = d;
    }).catch(e => (error = errorMessage(e)));
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
      error = 'Correct the invalid date before leaving.';
      return false;
    }
    if (pending) {
      error = 'Resolve the pending task creation using Retry before leaving.';
      return false;
    }
    if (saving || phase === 'uploading') return false;
    if (busy) {
      if (
        !(await ask(
          'Stop this operation and leave? Your saved review will remain.',
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
        'Regenerate recommendations? Uncreated suggestion edits will be replaced.',
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
        ? 'Review every project and suggestion before creating.'
        : 'No suggestions found. Edit your instructions or add an item manually.';
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
    flagRecovery(true);
    try {
      await api.setRecovery($state.snapshot(pending));
      const input = $state.snapshot(pending);
      created = await (isImage ? workspace.acceptImage(input) : isEmail ? workspace.acceptEmail(input) : workspace.acceptVoice(input));
      await api.setRecovery(null);
      pending = null;
      flagRecovery(false);
      draft = await api.load();
      notice =
        'Selected items created. Unselected suggestions remain in this review.';
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
          flagRecovery(false);
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
      !(await ask(isEmail
        ? 'Clear this review? Uncreated suggestions are removed and those emails will not be suggested again. Created tasks remain.'
        : 'Discard this source and unaccepted suggestions? Created items will remain.',
      ))
    )
      return;
    await persist();
    if (unsaved) return;
    try {
      await api.discard($state.snapshot(draft));
      draft = null;
      imageUrl = '';
      created = null;
      dateErrors = [];
      notice = 'Review cleared. Created items remain in their projects.';
    } catch (e) {
      error = errorMessage(e);
    }
  }
  async function checkEmail() {
    if (!isEmail || busy || pending || !ready || dateErrors.length) return;
    error = '';
    notice = '';
    const epoch = ++generation;
    // New suggestions are added to this review; nothing already here is replaced.
    if (!draft) { draft = emailApi.newDraft(workspace.nowUtc, workspace.timeZone); changed(); }
    await persist();
    if (unsaved || disposed || epoch !== generation || !draft) return;
    phase = 'suggesting';
    try {
      const result = await emailApi.check($state.snapshot(draft));
      if (disposed || epoch !== generation) return;
      draft = result.draft;
      inboxes = result.inboxes;
      notice = emailSummary(result);
      if (result.problems.length) error = result.problems.join(' ');
    } catch (e) {
      if (epoch === generation) error = errorMessage(e);
    } finally {
      if (epoch === generation) phase = 'idle';
    }
  }
  /** Deletes a task Signal added by itself; refused once time is tracked on it. */
  async function undo(c: voiceApi.Candidate) {
    if (!isEmail || !draft || busy || pending) return;
    error = '';
    notice = '';
    const epoch = ++generation;
    await persist();
    if (unsaved || disposed || epoch !== generation || !draft) return;
    phase = 'undoing';
    try {
      const next = await emailApi.undo($state.snapshot(draft), c.id);
      if (disposed || epoch !== generation) return;
      draft = next;
      notice = `Removed “${c.title}”.`;
    } catch (e) {
      if (epoch === generation) error = errorMessage(e);
    } finally {
      if (epoch === generation) phase = 'idle';
    }
  }
  function openEmail(c: voiceApi.Candidate) {
    const opened = c.mail_ref ? calendarApi.openOutlook(c.mail_ref) : openExternalUrl(c.link_url!);
    void opened.catch(e => (error = errorMessage(e)));
  }
  function assign(c: voiceApi.Candidate, v: string) {
    c.project_id = v.startsWith('project:') ? v.slice(8) : null;
    c.proposal_id = v.startsWith('new:') ? v.slice(4) : null;
    c.duplicate_ok = false;
    changed();
  }
  /** Email suggestions that are expanded; the rest show only their title bar. */
  const open = new SvelteSet<string>();
  function toggle(id: string) {
    if (open.has(id)) open.delete(id);
    else open.add(id);
  }
  /** Who sent it, the priority and the due date, for the collapsed bar. */
  function barInfo(c: voiceApi.Candidate): string {
    const from = c.source?.split(' · ')[0] ?? '';
    const due = c.due_at
      ? 'due ' + new Date(c.due_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', timeZone: draft?.time_zone })
      : '';
    return [from, c.priority + ' priority', due].filter(Boolean).join(' · ');
  }
  /** Drops a suggestion you don't want; its email has been read and is not suggested again. */
  function remove(c: voiceApi.Candidate) {
    if (!draft || busy || pending || c.created_id) return;
    const i = draft.candidates.findIndex(x => x.id === c.id);
    if (i < 0) return;
    draft.candidates.splice(i, 1);
    open.delete(c.id);
    error = '';
    notice = `Deleted “${c.title || 'Untitled task'}”.`;
    changed();
  }
  function addTask() {
    if (!draft) return;
    const id = crypto.randomUUID();
    if (isEmail) open.add(id);
    draft.candidates.push({
      id,
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
  function deadline(c: voiceApi.Candidate, value: string) {
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

  function canImportImage() {
    return isImage && ready && !disposed && !busy && !pending && !workspace.editor && !confirmation && !zoom
      && !document.querySelector('dialog[open]');
  }
  function isTextInput(target: EventTarget | null) {
    return target instanceof Element && !!target.closest('input, textarea, [contenteditable]:not([contenteditable="false"]), [role="textbox"]');
  }
  function pasteShortcut(e: KeyboardEvent) {
    if (e.defaultPrevented || e.repeat || e.altKey || e.shiftKey || !(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'v'
      || !canImportImage() || isTextInput(e.target)) return;
    // A non-editable WebView target may not emit a paste event for native bitmaps.
    // Prevent the browser's paste so this gesture imports the image exactly once.
    e.preventDefault();
    void replaceImage(imageApi.clipboardImage);
  }
  function pasteImage(e: ClipboardEvent) {
    if (e.defaultPrevented || !canImportImage()) return;
    const clipboard = e.clipboardData;
    if (!clipboard) return;
    const files = Array.from(clipboard.files ?? []);
    if (!files.length) {
      for (const item of Array.from(clipboard.items ?? [])) {
        if (item.kind !== 'file') continue;
        const file = item.getAsFile();
        if (file) files.push(file);
      }
    }
    if (files.length) {
      e.preventDefault();
      void upload(files);
    } else if (!Array.from(clipboard.types ?? []).some(type => type.startsWith('text/'))) {
      e.preventDefault();
      void replaceImage(imageApi.clipboardImage);
    }
  }
  async function upload(files: FileList | File[] | null) {
    if (!files?.length || !canImportImage()) return;
    if (files.length !== 1) { error = 'Choose one image per review.'; return; }
    const file = files[0];
    if (file.size > 10 * 1024 * 1024) { error = 'Choose an image up to 10 MiB.'; return; }
    await replaceImage(async () => Array.from(new Uint8Array(await file.arrayBuffer())));
  }
  async function replaceImage(source: number[] | string | null | (() => Promise<number[]>)) {
    if (!canImportImage()) return;
    phase = 'uploading'; error = '';
    let replacing = false;
    let analyzeSource = false;
    try {
      const bytes = typeof source === 'function' ? await source() : source;
      if (disposed) return;
      if (draft?.candidates.some(c => !c.created_id) && !(await ask('Replace the source and clear uncreated suggestions? Created items remain.'))) return;
      if (disposed) return;
      if (!draft) { draft = imageApi.newDraft(workspace.nowUtc, workspace.timeZone); changed(); }
      await persist();
      if (unsaved || !draft || disposed) return;
      replacing = true;
      draft = typeof bytes === 'string' ? await imageApi.setImagePath($state.snapshot(draft), bytes) : await imageApi.setImage($state.snapshot(draft), bytes);
      imageUrl = draft.image_id ? await imageApi.image(draft.image_id) : '';
      dateErrors = [];
      analyzeSource = bytes !== null;
    } catch (e) {
      error = errorMessage(e);
      // A dropped bridge reply may follow a successful source replacement.
      // Reconcile before allowing another revisioned operation.
      if (replacing) {
        try { draft = await imageApi.load(); imageUrl = draft?.image_id ? await imageApi.image(draft.image_id) : ''; }
        catch { ready = false; error += ' Reopen AI Inbox to recover the saved review.'; }
      }
    } finally { phase = 'idle'; }
    if (analyzeSource && !disposed) await analyze(false);
  }
  function addMeeting() {
    addTask();
    const c = draft!.candidates[draft!.candidates.length - 1];
    c.kind = 'meeting';
    c.meeting = { title:'', starts_at:'', start_local:'', start_offset:'', time_zone:draft!.time_zone, duration_min:0, link_url:null, agenda_md:'', notes_md:'', reminder_min:15, show_in_day:true, task_ids:[], repeat_rule:'none', repeat_weekdays:[], repeat_until:null, fold_policy:'earlier' };
    changed();
  }

</script>

{#snippet fields(c: voiceApi.Candidate)}
                    {#if needsProject && !c.project_id && !c.proposal_id}<p class="warning">Which project is this {c.kind === 'meeting' ? 'meeting' : 'task'} for?</p>{/if}
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
                          >{/each}{#each draft!.projects as p}<option
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
                    {#if c.kind === 'meeting' && c.meeting}
                      <MeetingCandidate bind:meeting={c.meeting} onChange={() => {c.duplicate_ok = false; changed();}} />
                    {:else}
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
                        >Due · {draft!.time_zone}<input
                          type="datetime-local"
                          step="0.001"
                          value={c.due_at
                            ? `${dueFields(c.due_at, draft!.time_zone).date}T${dueFields(c.due_at, draft!.time_zone).time}`
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
                    {/if}
                    {#if c.source}<blockquote>
                        {c.source}
                      </blockquote>{/if}{#if c.link_url || c.mail_ref}<button class="outline" onclick={() => openEmail(c)}>Open email <ExternalLink /></button>{/if}{#if c.warning}<p class="warning">
                        {c.warning}
                      </p>{/if}
                    <label class="inline"
                      ><input
                        type="checkbox"
                        bind:checked={c.duplicate_ok}
                        onchange={changed}
                      />Allow a duplicate {c.kind === 'meeting' ? 'title and start time' : 'title'} in this project if intentional</label
                    >
{/snippet}


<svelte:window onkeydown={pasteShortcut} onpaste={pasteImage} ondragover={e => { if (isImage && e.dataTransfer?.types.includes('Files')) e.preventDefault(); }} ondrop={e => { if (isImage && e.dataTransfer?.files.length) { e.preventDefault(); void upload(e.dataTransfer.files); } }} />
<main class="voice-page">
  <div class="heading">
    <div>
      <h1>{isImage ? 'Image intake' : isEmail ? 'Email intake' : 'Voice intake'}</h1>
      <p>
        {isImage ? 'Paste an image to identify meetings and tasks automatically. Choose a project if needed, then review and create.'
          : isEmail ? 'Signal reads your new email and suggests tasks with priorities and due dates. Sure tasks from inboxes set to add automatically are added for you; review the rest.'
          : 'Dictate your projects and tasks. Review first; create only what you choose.'}
      </p>
    </div>
    <button class="outline" onclick={() => onSettings()}>Voice & AI settings</button>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if !ready}<p>Loading saved review…</p>{:else}
    {#if isImage}
      <section class="capture">
        <input class="file-input" bind:this={fileInput} type="file" accept="image/png,image/jpeg,image/webp" aria-label="Choose image" onchange={e => { void upload(e.currentTarget.files); e.currentTarget.value = ''; }} />
        <button class="primary" disabled={busy || !!pending} onclick={() => replaceImage(imageApi.clipboardImage)}><ClipboardPaste />Paste image</button>
        <button class="outline" disabled={busy || !!pending} onclick={() => fileInput?.click()}>{draft?.image_id ? 'Replace image' : 'Choose image'}</button>
        {#if draft?.image_id}<button class="outline" disabled={busy || !!pending} onclick={() => replaceImage(null)}>Remove image</button>{/if}
        <span>Copy an image or screenshot, then paste with Ctrl+V or Paste image. You can also choose or drop a file. PNG, JPEG, WebP · up to 10 MiB / 25 megapixels.</span>
        {#if busy}<span role="status">{phase === 'suggesting' ? `Analyzing image… ${suggestionSeconds}s elapsed (up to 90 seconds).` : phase === 'uploading' ? 'Saving image…' : 'Creating selected items…'}</span>
          {#if phase === 'suggesting'}<button class="outline" onclick={cancel}>Cancel analysis</button>{/if}
        {/if}
      </section>
    {:else if isEmail}
      <section class="capture">
        <button class="primary" disabled={busy || !!pending || !inboxes?.length || !!dateErrors.length} onclick={checkEmail}><Mail />Check email</button>
        {#if busy}<span role="status">{phase === 'suggesting' ? `Reading new email… ${suggestionSeconds}s elapsed (up to 2 minutes).` : phase === 'undoing' ? 'Removing task…' : 'Creating selected tasks…'}</span>
          {#if phase === 'suggesting'}<button class="outline" onclick={cancel}><X />Cancel</button>{/if}
        {:else if inboxes && !inboxes.length}<span>No inbox is set up for suggestions yet.</span><button class="outline" onclick={() => onSettings('Calendar & email')}>Calendar & email settings</button>
        {:else if inboxes}<span>From {inboxes.join(', ')}</span>{/if}
        {#if lastAuto}<span class="auto-status">{autoSummary(lastAuto, workspace.timeZone)}{#if lastAuto.error} <span class="warning">{lastAuto.error}</span>{/if}</span>{/if}
        <small>Signal checks every 15 minutes while it is open, or now with Check email. It only reads your email; it never sends, moves or marks anything. Each check sends up to 15 new emails from the last 7 days to your configured AI provider, and each email is checked once. Promotions, social, forum and update emails are skipped.</small>
      </section>
    {:else}
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
    {/if}
    {#if pending}<section class="recovery">
        <strong>Item creation needs confirmation.</strong>
        <p>
          Retry checks the original request; it will not create the same items
          twice.
        </p>
        <button class="primary" disabled={busy} onclick={accept}
          >{isImage ? 'Retry item creation' : 'Retry task creation'}</button
        >
      </section>{/if}
    {#if draft}
      <div class="review">
        <section class="source">
          <h2>{isImage ? 'Image and instructions' : isEmail ? 'Instructions' : 'Transcript'}</h2>
          {#if isImage && imageUrl}<button class="image-preview" aria-label="Enlarge source image" onclick={() => zoom = true}><img src={imageUrl} alt="Source for task and meeting suggestions" /></button>{/if}
          {#if isEmail}<p>Optional guidance for the next Check email, such as which senders matter most.</p>{/if}
          {#if isImage}<p>Instructions are optional. Meetings, times, and other visible details are filled in automatically. Add guidance here if needed, then analyze again.</p>{/if}
          <textarea
            aria-label={isImage || isEmail ? 'Instructions' : 'Transcript'}
            placeholder={isImage ? 'Optional: assign these meetings to Atlas, or focus on a specific part of the image…' : isEmail ? 'Optional: anything from Dana is high priority; ignore automated reports…' : undefined}
            rows={isImage || isEmail ? 5 : 16}
            bind:value={draft.transcript}
            oninput={changed}
            disabled={busy || !!pending}></textarea>
          <p>
            {draft.time_zone} · {isEmail ? 'started' : 'recorded'} {new Date(
              draft.recorded_at,
            ).toLocaleString()}
          </p>
          {#if !isEmail}<button
            class="outline"
            disabled={busy || !!pending || (isImage ? !draft.image_id : !draft.transcript.trim())}
            onclick={() => analyze()}>{phase === 'suggesting' ? 'Analyzing…' : isImage ? 'Analyze image' : 'Suggest tasks'}</button
          >{/if}
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
          <h2>{isImage ? 'Suggested tasks and meetings' : isEmail ? 'Suggested tasks from email' : 'Task recommendations'}</h2>
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
                    />Approve creating this project when accepting its items</label
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
              >{#if isImage}<button class="outline" onclick={addMeeting}>Add meeting</button>{/if}<button
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
                  {#if c.created_id}<strong><Check />{c.auto ? 'Added' : 'Created'}: {c.title}</strong
                    ><button
                      class="outline"
                      onclick={() => c.kind === 'meeting' ? onOpenMeeting({meetingId:c.created_id!, occurrenceKey:'o:0'}) : onOpenTask(c.created_id!)}
                      >Open {c.kind === 'meeting' ? 'meeting' : 'task'} <ArrowRight /></button
                    >{#if c.auto}{#if c.link_url || c.mail_ref}<button class="outline" onclick={() => openEmail(c)}>Open email <ExternalLink /></button>{/if}<button
                        class="outline"
                        onclick={() => undo(c)}
                        aria-label={`Undo ${c.title}`}><X />Undo</button
                      >{#if c.source}<blockquote>{c.source}</blockquote>{/if}{/if}
                  {:else if isEmail}
                    <div class="bar">
                      <input
                        type="checkbox"
                        bind:checked={c.selected}
                        onchange={changed}
                        aria-label={`Include ${c.title || 'untitled task'}`}
                      /><button
                        class="toggle"
                        aria-expanded={open.has(c.id)}
                        aria-controls={`fields-${c.id}`}
                        onclick={() => toggle(c.id)}
                        ><span class="chevron" class:open={open.has(c.id)}><ChevronRight /></span
                        ><span class="bar-text"
                          ><strong>{c.title || 'Untitled task'}</strong
                          ><small>{barInfo(c)}{#if !c.project_id && !c.proposal_id}{' · '}<span class="warning">needs a project</span>{/if}</small
                        ></span
                      ></button
                      ><button
                        class="outline"
                        disabled={busy || !!pending}
                        onclick={() => remove(c)}
                        aria-label={`Delete ${c.title || 'untitled task'}`}><Trash2 />Delete</button
                      >
                    </div>
                    {#if open.has(c.id)}<div class="fields" id={`fields-${c.id}`}>{@render fields(c)}</div>{/if}
                  {:else}
                    <label class="inline"
                      ><input
                        type="checkbox"
                        bind:checked={c.selected}
                        onchange={changed}
                      /><strong>Include {c.kind === 'meeting' ? 'meeting' : 'task · To Do'}</strong></label
                    >
                    {@render fields(c)}
                  {/if}
                </article>
              {/each}{/each}
            {#if !draft.candidates.length}<p>
                Recommendations will appear here. You can also add and edit
                {isImage ? 'tasks and meetings' : 'tasks'} manually.
              </p>{/if}
            <div class="actions footer">
              <button
                class="primary"
                disabled={!selected.length ||
                  unsaved ||
                  saving ||
                  !!dateErrors.length || selected.some(c => !!meetingIssue(c) || (needsProject && !c.project_id && !c.proposal_id))}
                onclick={accept}
                ><Check />Create selected ({selected.length})</button
              ><button class="outline" onclick={discard}
                >{draft.candidates.length &&
                draft.candidates.every((c) => c.created_id)
                  ? 'Finish review'
                  : isEmail ? 'Clear review' : 'Discard draft'}</button
              >
            </div>
          </fieldset>
        </section>
      </div>
    {/if}
    {#if created}<p role="status">
        Created {created.created.length} items. Tasks are in To Do; meetings appear in Signal’s calendar.
      </p>{/if}
  {/if}
  {#if zoom && imageUrl}<dialog class="zoom" use:modal oncancel={() => zoom = false} aria-label="Source image"><button class="outline" onclick={() => zoom = false}>Close image</button><img src={imageUrl} alt="Source at full size" /></dialog>{/if}
  {#if confirmation}<dialog
      class="confirm"
      use:modal
      oncancel={() => decide(false)}
      aria-label="Confirm intake action"
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
  .file-input { display: none; }
  .image-preview { display: block; width: 100%; padding: 0; overflow: hidden; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .image-preview img { width: 100%; max-height: 420px; object-fit: contain; }
  .zoom { max-width: 95vw; max-height: 90vh; overflow: auto; background: var(--surface); color: var(--text); }
  .zoom img { display: block; max-width: none; }
  .zoom button { position: sticky; top: 0; }

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
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .bar input {
    width: auto;
  }
  .toggle {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    padding: 0;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .bar-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .bar-text strong,
  .bar-text small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar-text small {
    color: var(--text-muted);
  }
  .chevron {
    display: inline-flex;
    transition: transform 0.15s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .fields {
    margin-top: 16px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
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
