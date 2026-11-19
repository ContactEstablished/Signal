import { invoke } from '@tauri-apps/api/core';
export interface Calendar {
  id: string;
  name: string;
  color: string;
  primary: boolean;
  project_id: string | null;
}
export interface AccountStatus {
  last_sync: string | null;
  last_error: string | null;
  needs_sign_in: boolean;
}
export interface Account {
  /** Google (signed in from Signal) or classic Outlook on this computer. */
  kind: 'google' | 'outlook';
  id: string;
  email: string;
  calendars: Calendar[];
  /** Google allowed Signal to read this inbox. */
  mail: boolean;
  /** Signal suggests tasks from this inbox. */
  mail_tasks: boolean;
  /** Sure tasks from this inbox are added without review. */
  auto_add: boolean;
  status: AccountStatus;
}
export interface CalendarStatus {
  has_client: boolean;
  connecting: boolean;
  accounts: Account[];
  past_days: number;
  future_days: number;
}
export interface SyncSummary {
  created: number;
  updated: number;
  removed: number;
  canceled: number;
  errors: string[];
}
export const status = () => invoke<CalendarStatus>('calendar_status');
/** Opens a file picker; false when the person cancels it. */
export const importClient = () =>
  invoke<boolean>('calendar_import_client', { path: null });
/** Opens the browser for Google sign-in and resolves with the account email. */
export const connect = () => invoke<string>('calendar_connect');
export const cancelConnect = () => invoke<void>('calendar_cancel_connect');
export const setProject = (
  accountId: string,
  calendarId: string,
  projectId: string | null,
) =>
  invoke<void>('calendar_set_project', { accountId, calendarId, projectId });
export const removeAccount = (accountId: string) =>
  invoke<void>('calendar_remove_account', { accountId });
export const syncNow = () => invoke<SyncSummary>('calendar_sync_now');
export const runtime = (timeZone: string) =>
  invoke<void>('calendar_runtime', { timeZone });
/** Adds the accounts set up in classic Outlook on this computer. */
export const addOutlook = () =>
  invoke<{ added: string[]; found: number }>('outlook_add_accounts');
/** Shows an Outlook suggestion's email in Outlook (classic). */
export const openOutlook = (mailRef: string) =>
  invoke<void>('outlook_open', { mailRef });
/** Turns task suggestions from this account's inbox on or off. */
export const setMail = (accountId: string, on: boolean) =>
  invoke<void>('calendar_set_mail', { accountId, on });
/** Adds sure tasks from this inbox without review. */
export const setAutoAdd = (accountId: string, on: boolean) =>
  invoke<void>('calendar_set_auto_add', { accountId, on });
