/** Accepted #1b, #2a–e and #3b–d, reconciled at 2025-09-11 13:42 America/New_York.
 * No active timer state or attachment files. See docs/verification/M0.md for conflicts.
 * Opaque stable fixture keys; external provider IDs are attributes, not primary keys.
 */
export const FIXTURE_NOW = '2025-09-11T17:42:00.000Z';
export const FIXTURE_ZONE = 'America/New_York';
export const FIXTURE_DATABASE = 'sqlite:signal-dev-september-2025.db';
export const FIXTURE_VERSION = 'september-2025-midday-v1';
type Value = string | number | null;
export interface SeedTable { table: string; rows: Record<string, Value>[] }
const id = (n: number) => `f870c681-27a4-4d65-87be-${n.toString(16).padStart(12, '0')}`;
const project = { atlas: id(1), website: id(2), home: id(3), cli: id(4) };
// All fixture times below explicitly use September's New York UTC offset.
const instant = (date: string, time = '17:00') => new Date(`${date}T${time}:00-04:00`).toISOString();
const tasks = [
  [101, 'atlas', 'Deprecate legacy /v1 endpoints', 'jira', 'ATL-501', 'backlog', '2025-10-02', null],
  [102, 'atlas', 'Write postmortem template', 'asana', '1189', 'backlog', null, null],
  [103, 'atlas', 'Evaluate feature-flag service', 'jira', 'ATL-506', 'backlog', '2025-10-09', null],
  [104, 'atlas', 'Write rollback runbook', 'jira', 'ATL-490', 'todo', '2025-09-12', 6],
  [105, 'atlas', 'Update on-call rotation doc', 'clickup', '86c', 'todo', '2025-09-19', null],
  [106, 'atlas', 'Load-test new gateway', 'jira', 'ATL-495', 'todo', '2025-09-23', null],
  [107, 'atlas', 'Migrate auth service to new gateway', 'jira', 'ATL-482', 'in_progress', '2025-09-11', 12],
  [108, 'atlas', 'Dual-write to new data store', 'jira', 'ATL-477', 'in_progress', '2025-09-16', 16],
  [109, 'atlas', 'Provision prod certificates', 'jira', 'ATL-469', 'blocked', '2025-09-09', null],
  [110, 'atlas', 'Set up staging gateway', 'jira', 'ATL-460', 'done', '2025-09-08', null],
  [111, 'atlas', 'Draft migration plan', 'jira', 'ATL-451', 'done', '2025-09-08', null],
  [112, 'website', 'Hero section copy', null, null, 'in_progress', null, null],
  [113, 'website', 'Nav accessibility pass', null, null, 'todo', '2025-09-15', 2],
  [114, 'home', 'Order tile samples', null, null, 'todo', '2025-09-11', 1],
  [115, 'home', 'Paint samples on wall', null, null, 'todo', '2025-09-13', null],
  [116, 'cli', 'Parse config flags', 'clickup', 'CU-86c', 'in_progress', '2025-09-19', 3],
] as const;
const taskRows = tasks.map(([key, p, title, provider, externalId, status, due, estimate]) => ({
  id: id(key), project_id: project[p], title, external_provider: provider, external_id: externalId,
  // Only Jira URLs are fully specified by accepted references. Never fabricate provider links.
  external_url: provider === 'jira' ? `https://northwind.atlassian.net/browse/${externalId}` : null,
  status, priority: key === 107 ? 'high' : 'medium', due_at: due ? instant(due) : null,
  estimate_h: estimate, blocked_reason: key === 109 ? 'Waiting on security team approval' : null,
  blocked_on: key === 109 ? 'Marcus T.' : null,
  notes_md: key === 107 ? 'Latency p95 on the new gateway is 38ms vs 51ms legacy. Cutover blocked on ATL-469 certs — if not approved by Fri, ship with staging certs behind the flag.' : '',
  created_at: instant('2025-08-28', '09:00'), updated_at: FIXTURE_NOW,
  done_at: status === 'done' ? instant('2025-09-08') : null,
}));
const block = (key: number, date: string, start: number, end: number, task: number | null, done = false, kind = 'task') => ({
  id: id(key), date, start_min: start, end_min: end, kind, task_id: task === null ? null : id(task),
  done: Number(done), done_at: done ? instant(date, `${Math.floor(end / 60).toString().padStart(2, '0')}:${(end % 60).toString().padStart(2, '0')}`) : null,
  carried_from_block_id: null,
});
const blocks = [
  block(301, '2025-09-10', 840, 960, 104),
  block(302, '2025-09-10', 960, 1020, 112),
  block(303, '2025-09-10', 1020, 1080, 114),
  block(304, '2025-09-11', 540, 630, 107, true),
  block(305, '2025-09-11', 540, 630, 106, true),
  block(306, '2025-09-11', 630, 660, null, false, 'break'),
  block(307, '2025-09-11', 660, 720, 104, true),
  block(308, '2025-09-11', 660, 720, 112),
  block(309, '2025-09-11', 720, 780, null, false, 'lunch'),
  block(310, '2025-09-11', 780, 900, 108),
  block(311, '2025-09-11', 780, 870, 116),
  block(312, '2025-09-11', 900, 990, 114),
  // "Chase Security on prod certs" is a block for ATL-469, not an extra task.
  block(313, '2025-09-11', 990, 1020, 109),
];
const entry = (key: number, task: number, date: string, start: string, minutes: number, blockKey: number | null = null) => {
  const started = instant(date, start);
  return { id: id(key), task_id: id(task), block_id: blockKey === null ? null : id(blockKey), started_at: started,
    ended_at: new Date(Date.parse(started) + minutes * 60_000).toISOString(), minutes };
};
export const seedTables: SeedTable[] = [
  { table: 'projects', rows: [
    { id: project.atlas, name: 'Atlas Migration', color: 'cyan', manager_name: 'Dana Kim', manager_email: 'dana@northwind.dev', summary_send_at: '17:00', summary_tone: 'brief', sort_order: 0 },
    { id: project.website, name: 'Website Refresh', color: 'lime', manager_name: 'Priya Raman', manager_email: 'priya@studio.co', summary_send_at: '16:30', summary_tone: 'detailed', sort_order: 1 },
    { id: project.home, name: 'Home Renovation', color: 'magenta', manager_name: null, manager_email: null, summary_send_at: null, summary_tone: 'brief', sort_order: 2 },
    { id: project.cli, name: 'CLI', color: 'violet', manager_name: null, manager_email: null, summary_send_at: '17:00', summary_tone: 'brief', sort_order: 3 },
  ] },
  { table: 'tasks', rows: taskRows },
  { table: 'subtasks', rows: ['Inventory current auth routes', 'Shadow traffic to new gateway', 'Compare token validation latency', 'Flip 10% of prod traffic', 'Remove legacy middleware'].map((title, i) => ({ id: id(201 + i), task_id: id(107), title, done: Number(i < 3), sort_order: i })) },
  { table: 'tags', rows: ['api', 'research', 'ops', 'data', 'security'].map((name, i) => ({ id: id(221 + i), name, color: ['cyan', 'lime', 'magenta', 'lime', 'orange'][i] })) },
  { table: 'task_tags', rows: [[101,221], [103,222], [104,223], [106,221], [107,221], [107,225], [108,224]].map(([t, tag]) => ({task_id: id(t), tag_id: id(tag)})) },
  { table: 'meetings', rows: [
    { id: id(251), project_id: project.website, title: 'Standup', starts_at: instant('2025-09-11', '09:30'), duration_min: 30, link_url: null, agenda_md: '', repeat_rule: 'none', reminder_min: 15 },
    { id: id(252), project_id: project.atlas, title: 'Auth cutover sync', starts_at: instant('2025-09-11', '14:00'), duration_min: 45, link_url: null, agenda_md: '1. Confirm 10% traffic results (p95, error rate)\n2. Cert status — is Security signing off Friday?\n3. Go / no-go for Tue Sep 16 cutover', repeat_rule: 'none', reminder_min: 15 },
    { id: id(253), project_id: project.atlas, title: 'Data-store review', starts_at: instant('2025-09-12', '09:30'), duration_min: 30, link_url: null, agenda_md: '', repeat_rule: 'none', reminder_min: 15 },
    { id: id(254), project_id: project.atlas, title: 'Sprint planning', starts_at: instant('2025-09-10', '10:00'), duration_min: 60, link_url: null, agenda_md: '', repeat_rule: 'none', reminder_min: 15 },
  ] },
  { table: 'meeting_tasks', rows: [{ meeting_id: id(252), task_id: id(107) }, { meeting_id: id(252), task_id: id(109) }] },
  { table: 'blocks', rows: blocks },
  { table: 'time_entries', rows: [
    // Historical accounting intervals are fixture assumptions, not asserted reference history.
    entry(401, 107, '2025-09-09', '09:00', 390),
    entry(402, 108, '2025-09-08', '08:00', 540),
    entry(403, 104, '2025-09-10', '14:00', 60, 301),
    entry(404, 107, '2025-09-11', '09:00', 90, 304),
    entry(405, 106, '2025-09-11', '09:00', 90, 305),
    entry(406, 104, '2025-09-11', '11:00', 60, 307),
    entry(407, 112, '2025-09-11', '11:00', 60, 308),
  ] },
  { table: 'alerts', rows: [1440,60].map((offset_min, i) => ({ id: id(451 + i), task_id: id(107), offset_min })) },
];

/** Loader is unavailable in production and the Rust boundary independently enforces --seed. */
export async function loadSeed(database: string): Promise<void> {
  if (!import.meta.env.DEV || database !== FIXTURE_DATABASE) throw new Error('Seed loading requires the isolated development database.');
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('seed_database', { version: FIXTURE_VERSION, tables: seedTables });
}
