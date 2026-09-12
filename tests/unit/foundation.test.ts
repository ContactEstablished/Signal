import { readFileSync } from 'node:fs';
import initSqlJs, { type Database } from 'sql.js';
import { beforeAll, afterEach, describe, expect, it } from 'vitest';
import { seedTables, FIXTURE_NOW, FIXTURE_ZONE } from '../../src/lib/seed';
import { todayCount, localDate } from '../../src/lib/domain/dates';
let SQL: Awaited<ReturnType<typeof initSqlJs>>;
let databases: Database[] = [];
beforeAll(async () => { SQL = await initSqlJs(); });
afterEach(() => { databases.forEach(db => db.close()); databases = []; });
function database() {
  const db = new SQL.Database(); databases.push(db);
  db.run('PRAGMA foreign_keys=ON');
  db.run(readFileSync('src-tauri/migrations/0001_initial.sql', 'utf8'));
  return db;
}
function seed(db: Database) {
  db.run('BEGIN');
  try {
    for (const table of seedTables) for (const row of table.rows) {
      const keys = Object.keys(row);
      db.run(`INSERT INTO ${table.table} (${keys.join(',')}) VALUES (${keys.map(() => '?').join(',')})`, Object.values(row));
    }
    db.run('COMMIT');
  } catch (e) { db.run('ROLLBACK'); throw e; }
}
function scalar(db: Database, sql: string) { return db.exec(sql)[0].values[0][0]; }
describe('M0 persistence integrity (real SQLite via test-only WASM)', () => {
  it('migrates all 13 tables, loads the fixture and derives coherent totals', () => {
    const db = database(); seed(db);
    expect(scalar(db, "SELECT COUNT(*) FROM sqlite_master WHERE type='table'")).toBe(13);
    expect(db.exec('PRAGMA foreign_key_check')).toEqual([]);
    expect(scalar(db, 'SELECT COUNT(*) FROM tasks')).toBe(16);
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-482'")).toBe(8);
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-490'")).toBe(2);
    expect(scalar(db, 'SELECT COUNT(*) FROM tasks JOIN task_time_totals ON tasks.id=task_id WHERE tasks.hours_worked != task_time_totals.hours_worked')).toBe(0);
    expect(scalar(db, "SELECT value FROM settings WHERE key='close_to_tray'")).toBe('false');
  });
  it('maintains cache on fractional inserts and reassignment, and rolls back together', () => {
    const db = database(); seed(db);
    const source = String(scalar(db, "SELECT id FROM tasks WHERE external_id='ATL-482'"));
    const target = String(scalar(db, "SELECT id FROM tasks WHERE external_id='ATL-477'"));
    db.run('BEGIN');
    db.run("INSERT INTO time_entries VALUES ('test-entry',?,NULL,'2025-09-11T16:00:00.000Z','2025-09-11T16:01:30.000Z',1.5)", [source]);
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-482'")).toBeCloseTo(8.025);
    db.run("UPDATE time_entries SET task_id=? WHERE id='test-entry'", [target]);
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-482'")).toBe(8);
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-477'")).toBeCloseTo(9.025);
    db.run('ROLLBACK');
    expect(scalar(db, "SELECT hours_worked FROM tasks WHERE external_id='ATL-477'")).toBe(9);
    expect(() => db.run('UPDATE tasks SET hours_worked=100')).toThrow(/derived/);
  });
  it('rejects invalid references, inverted blocks and cross-task time entries', () => {
    const db = database(); seed(db);
    expect(() => db.run("INSERT INTO subtasks VALUES ('invalid','missing','bad',0,0)")).toThrow(/FOREIGN KEY/);
    expect(() => db.run("UPDATE blocks SET end_min=start_min")).toThrow(/CHECK/);
    expect(() => db.run("UPDATE tasks SET status='unknown'")).toThrow(/CHECK/);
    expect(() => db.run("UPDATE time_entries SET block_id=(SELECT id FROM blocks WHERE kind='task' AND task_id != time_entries.task_id LIMIT 1)")).toThrow(/same task/);
  });
  it('keeps an independent ordinary database empty when the fixture loads', () => {
    const ordinary = database(); const fixture = database(); seed(fixture);
    expect(scalar(ordinary, 'SELECT COUNT(*) FROM projects')).toBe(0);
    expect(scalar(fixture, 'SELECT COUNT(*) FROM projects')).toBe(4);
  });
});
describe('local calendar badge', () => {
  it('counts overdue plus due today without counting completed or undated tasks', () => {
    const tasks = seedTables.find(t => t.table === 'tasks')!.rows.map(r => ({status: String(r.status), due_at: r.due_at as string | null}));
    expect(todayCount(tasks, new Date(FIXTURE_NOW), FIXTURE_ZONE)).toBe(3);
  });
  it('uses local day boundaries across UTC midnight and DST changes', () => {
    expect(localDate(new Date('2025-09-12T02:00:00Z'), FIXTURE_ZONE)).toBe('2025-09-11');
    expect(todayCount([{status:'todo',due_at:'2025-09-12T03:59:00Z'}, {status:'todo',due_at:'2025-09-12T04:00:00Z'}],new Date(FIXTURE_NOW),FIXTURE_ZONE)).toBe(1);
    expect(localDate(new Date('2025-11-02T05:30:00Z'), FIXTURE_ZONE)).toBe('2025-11-02');
    expect(localDate(new Date('2025-11-02T06:30:00Z'), FIXTURE_ZONE)).toBe('2025-11-02');
  });
});
