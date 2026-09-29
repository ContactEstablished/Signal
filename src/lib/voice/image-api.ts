import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { emptyDraft, type Draft, type Acceptance, type Receipt } from './api';
export { cancel } from './api';
export const load = () => invoke<Draft | null>('load_image_draft');
export const save = (draft: Draft) => invoke<Draft>('save_image_draft', { draft });
export const setImage = (draft: Draft, bytes: number[] | null) =>
  invoke<Draft>('set_intake_image', { id: draft.id, revision: draft.revision, bytes });
export const image = (id: string) => invoke<string>('get_intake_image', { id });
export const setImagePath = (draft: Draft, path: string) => invoke<Draft>('set_intake_image_path', { id: draft.id, revision: draft.revision, path });
export const subscribeDrop = (receive: (paths: string[]) => void) => getCurrentWebview().onDragDropEvent(event => {
  if (event.payload.type === 'drop') receive(event.payload.paths);
});
export const discard = (draft: Draft) => invoke<void>('discard_voice_draft', { id: draft.id, revision: draft.revision });
export const suggest = (draft: Draft) => invoke<Draft>('suggest_image_items', { id: draft.id, revision: draft.revision });
export const accept = (input: Acceptance) => invoke<Receipt>('accept_voice_tasks', { input });
export const recovery = () => invoke<Acceptance | null>('image_recovery');
export const setRecovery = (input: Acceptance | null) => invoke<void>('set_image_recovery', { input });
export const newDraft = (now: string, zone: string): Draft => ({ ...emptyDraft(now, zone), source_kind: 'image', image_id: null });
