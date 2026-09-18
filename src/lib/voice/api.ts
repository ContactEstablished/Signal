import { invoke } from '@tauri-apps/api/core';
import type { ProjectColor, Priority } from '../domain/types';
export interface Proposal {
  id: string;
  name: string;
  color: ProjectColor;
  approved: boolean;
  created_id: string | null;
}
export interface Candidate {
  id: string;
  selected: boolean;
  project_id: string | null;
  proposal_id: string | null;
  title: string;
  notes: string;
  subtasks: string[];
  due_at: string | null;
  priority: Priority;
  estimate_h: number | null;
  source: string;
  warning: string;
  duplicate_ok: boolean;
  created_id: string | null;
}
export interface Draft {
  id: string;
  revision: number;
  recorded_at: string;
  time_zone: string;
  transcript: string;
  projects: Proposal[];
  candidates: Candidate[];
}
export interface Acceptance {
  request_id: string;
  draft_id: string;
  revision: number;
  candidate_ids: string[];
}
export interface Receipt {
  created: { id: string; project_id: string; title: string }[];
  projects: { id: string; name: string }[];
}
export interface VoiceSettings {
  endpoint: string;
  model: string;
  whisper_model: string;
  microphone: string;
  consent: boolean;
}
export interface SettingsReply {
  settings: VoiceSettings;
  has_key: boolean;
  models: { id: string; bytes: number; installed: boolean }[];
}
export const settings = () => invoke<SettingsReply>('voice_settings');
export const saveSettings = (settings: VoiceSettings, key: string | null) =>
  invoke<void>('save_voice_settings', { settings, key });
export const download = (model: string) =>
  invoke<void>('download_voice_model', { model });
export const cancel = () => invoke<void>('cancel_voice');
export const load = () => invoke<Draft | null>('load_voice_draft');
export const save = (draft: Draft) =>
  invoke<Draft>('save_voice_draft', { draft });
export const discard = (draft: Draft) =>
  invoke<void>('discard_voice_draft', {
    id: draft.id,
    revision: draft.revision,
  });
export async function suggest(draft: Draft): Promise<Draft> {
  // Native work has a 90-second deadline. Also bound the desktop bridge wait.
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      invoke<Draft>('suggest_voice_tasks', {
        id: draft.id,
        revision: draft.revision,
      }),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => {
          void cancel().catch(() => {});
          reject(new Error('Task suggestions timed out. Your transcript is saved. Retry Suggest tasks, or choose another model in Voice & AI settings.'));
        }, 100_000);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
export const accept = (input: Acceptance) =>
  invoke<Receipt>('accept_voice_tasks', { input });
export const recovery = () => invoke<Acceptance | null>('voice_recovery');
export const setRecovery = (input: Acceptance | null) =>
  invoke<void>('set_voice_recovery', { input });
export function emptyDraft(now: string, zone: string): Draft {
  return {
    id: crypto.randomUUID(),
    revision: 0,
    recorded_at: now,
    time_zone: zone,
    transcript: '',
    projects: [],
    candidates: [],
  };
}
