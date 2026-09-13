import { readFileSync } from 'node:fs';
import initSqlJs from 'sql.js';
import { it, expect } from 'vitest';
import { seedTables } from '../../src/lib/seed';
it('adds durable timer tables without changing existing rows and enforces one live timer', async () => {
  const SQL = await initSqlJs();
  const db = new SQL.Database();
  try {
    db.run('PRAGMA foreign_keys=ON');
    for (const name of ['0001_initial.sql', '0002_board_fields.sql'])
      db.run(readFileSync('src-tauri/migrations/' + name, 'utf8'));
    for (const table of seedTables)
      for (const row of table.rows) {
        const keys = Object.keys(row);
        db.run(
          `INSERT INTO ${table.table} (${keys.join(',')}) VALUES (${keys.map(() => '?').join(',')})`,
          Object.values(row),
        );
      }
    const before = db.exec('SELECT * FROM tasks ORDER BY id');
    const entries = db.exec('SELECT * FROM time_entries ORDER BY id');
    db.run(readFileSync('src-tauri/migrations/0003_timers.sql', 'utf8'));
    expect(db.exec('SELECT * FROM tasks ORDER BY id')).toEqual(before);
    expect(db.exec('SELECT * FROM time_entries ORDER BY id')).toEqual(
      entries,
    );
    expect(db.exec('PRAGMA foreign_key_check')).toEqual([]);
    expect(
      db.exec("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")[0]
        .values[0][0],
    ).toBe(16);
    const id = String(before[0].values[0][0]);
    db.run(
      "INSERT INTO timer_sessions(id,task_id,state,started_at,segment_started_at) VALUES('one',?,'running','2025-09-11T13:00:00Z','2025-09-11T13:00:00Z')",
      [id],
    );
    expect(() =>
      db.run(
        "INSERT INTO timer_sessions(id,task_id,state,started_at) VALUES('two',?,'paused','2025-09-11T13:00:00Z')",
        [id],
      ),
    ).toThrow();
    expect(() =>
      db.run("UPDATE timer_sessions SET state='stopped' WHERE id='one'"),
    ).toThrow();
  } finally {
    db.close();
  }
});
