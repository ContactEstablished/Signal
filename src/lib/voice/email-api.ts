import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { emptyDraft, type Draft, type Acceptance, type Receipt } from './api';
export { cancel } from './api';

export interface CheckResult {
  draft: Draft;
  /** Emails sent to the AI provider in this check. */
  checked: number;
  /** New suggestions waiting for review. */
  suggested: number;
  /** Sure tasks Signal created without review. */
  added: number;
  /** New emails still waiting for a later check. */
  remaining: number;
  inboxes: string[];
  /** Per-inbox problems; other inboxes were still checked. */
  problems: string[];
}

/** The last automatic check (every 15 minutes while Signal is open). */
export interface AutoStatus {
  at: string;
  checked: number;
  suggested: number;
  added: number;
  remaining: number;
  error: string | null;
}

export const load = () => invoke<Draft | null>('load_email_draft');
export const save = (draft: Draft) => invoke<Draft>('save_email_draft', { draft });
export async function check(draft: Draft): Promise<CheckResult> {
  // Native work has a 150-second deadline. Also bound the desktop bridge wait.
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      invoke<CheckResult>('check_email', { id: draft.id, revision: draft.revision }),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => {
          void invoke('cancel_voice').catch(() => {});
          reject(new Error('Checking email timed out. Those emails will be checked again next time.'));
        }, 160_000);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
export const discard = (draft: Draft) => invoke<void>('discard_voice_draft', { id: draft.id, revision: draft.revision });
export const accept = (input: Acceptance) => invoke<Receipt>('accept_voice_tasks', { input });
export const recovery = () => invoke<Acceptance | null>('email_recovery');
export const setRecovery = (input: Acceptance | null) => invoke<void>('set_email_recovery', { input });
/** Deletes a task Signal added by itself and removes it from the review. */
export const undo = (draft: Draft, candidateId: string) =>
  invoke<Draft>('undo_email_task', { id: draft.id, revision: draft.revision, candidateId });
export const autoStatus = () => invoke<AutoStatus | null>('email_auto_status');
/** Runs after each automatic check; resolves with a function that stops listening. */
export const onChecked = (fn: (s: AutoStatus) => void) =>
  listen<AutoStatus>('signal://email-checked', (e) => fn(e.payload));
export const newDraft = (now: string, zone: string): Draft => ({ ...emptyDraft(now, zone), source_kind: 'email' });
