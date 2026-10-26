import { invoke } from '@tauri-apps/api/core';
export interface PairingStatus {
  active: boolean;
  seconds_left: number;
  failures: number;
  locked: boolean;
}
export interface WorkerStatus {
  running: boolean;
  connected: boolean;
  last_error: string | null;
  last_poll: string | null;
}
export interface TelegramStatus {
  has_token: boolean;
  bot_username: string;
  enabled: boolean;
  consent: boolean;
  paired: boolean;
  tray: boolean;
  pairing: PairingStatus;
  worker: WorkerStatus;
}
export interface PairingStart {
  code: string;
  minutes: number;
  bot_username: string;
}
export const status = () => invoke<TelegramStatus>('telegram_status');
export const saveToken = (token: string) =>
  invoke<{ bot_username: string }>('telegram_save_token', { token });
export const removeBot = () => invoke<void>('telegram_remove_bot');
export const setConsent = (consent: boolean) =>
  invoke<void>('telegram_set_consent', { consent });
export const setEnabled = (enabled: boolean) =>
  invoke<void>('telegram_set_enabled', { enabled });
export const startPairing = () =>
  invoke<PairingStart>('telegram_start_pairing');
export const cancelPairing = () => invoke<void>('telegram_cancel_pairing');
export const unpair = () => invoke<void>('telegram_unpair');
export const runtime = (timeZone: string) =>
  invoke<void>('telegram_runtime', { timeZone });
