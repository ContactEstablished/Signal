import { initializeAgenda } from '../native/agenda';
import { initializeTimers } from '../native/timers';
import type { TimerSnapshot } from '../domain/timers';
import Database from '@tauri-apps/plugin-sql';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { todayCount } from '../domain/dates';
export interface Project {
  id: string;
  name: string;
  color: string;
}
export interface Runtime {
  database: string;
  seeded: boolean;
}
export interface Foundation {
  runtime: Runtime;
  timerSnapshot: TimerSnapshot;
  projects: Project[];
  badge: number;
  tray: boolean;
  counts: {
    tasks: number;
    meetings: number;
    blocks: number;
    time_entries: number;
  };
}
export async function openFoundation(): Promise<Foundation> {
  if (!isTauri())
    throw new Error(
      'Open Signal with pnpm tauri dev to use the local database.',
    );
  const runtime = await invoke<Runtime>('runtime_config');
  const db = await Database.load(runtime.database);
  let now = new Date();
  let zone: string | undefined;
  if (runtime.seeded) {
    const seed = await import('../seed');
    await seed.loadSeed(runtime.database);

    zone = seed.FIXTURE_ZONE;
  }
  const timerSnapshot = await initializeTimers();
  now = new Date(Date.now() + timerSnapshot.offset_ms);
  await initializeAgenda(
    zone ?? Intl.DateTimeFormat().resolvedOptions().timeZone,
  );
  await invoke('initialize_workspace');
  const projects = await db.select<Project[]>(
    'SELECT id,name,color FROM projects ORDER BY sort_order,id',
  );
  const tasks = await db.select<{ due_at: string | null; status: string }[]>(
    'SELECT due_at,status FROM tasks',
  );
  const [counts] = await db.select<Foundation['counts'][]>(
    `SELECT (SELECT COUNT(*) FROM tasks) AS tasks, (SELECT COUNT(*) FROM meetings) AS meetings, (SELECT COUNT(*) FROM blocks) AS blocks, (SELECT COUNT(*) FROM time_entries) AS time_entries`,
  );
  const tray = await invoke<boolean>('load_tray_preference');
  return {
    runtime,
    timerSnapshot,
    projects,
    badge: todayCount(tasks, now, zone),
    tray,
    counts,
  };
}
export async function setTrayPreference(enabled: boolean): Promise<void> {
  await invoke('set_tray_preference', { enabled });
}
