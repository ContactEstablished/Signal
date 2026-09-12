import { readFileSync } from 'node:fs';
import initSqlJs from 'sql.js';
import { it, expect } from 'vitest';
import { seedTables } from '../../src/lib/seed';
it('upgrades populated M0 data without changing original fields, cache, or foreign keys', async () => {
  const SQL = await initSqlJs(),
    db = new SQL.Database();
  try {
    db.run('PRAGMA foreign_keys=ON');
    db.run(readFileSync('src-tauri/migrations/0001_initial.sql', 'utf8'));
    for (const table of seedTables)
      for (const row of table.rows) {
        const keys = Object.keys(row);
        db.run(
          `INSERT INTO ${table.table} (${keys.join(',')}) VALUES (${keys.map(() => '?').join(',')})`,
          Object.values(row),
        );
      }
    const columns = db
      .exec('PRAGMA table_info(tasks)')[0]
      .values.map((r) => r[1])
      .join(',');
    const before = db.exec(`SELECT ${columns} FROM tasks ORDER BY id`);
    db.run(readFileSync('src-tauri/migrations/0002_board_fields.sql', 'utf8'));
    expect(db.exec(`SELECT ${columns} FROM tasks ORDER BY id`)).toEqual(before);
    expect(db.exec('PRAGMA foreign_key_check')).toEqual([]);
    expect(
      db.exec("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")[0]
        .values[0][0],
    ).toBe(14);
    expect(
      db.exec(
        'SELECT COUNT(*) FROM tasks WHERE blocked_since IS NOT NULL OR revision!=0',
      )[0].values[0][0],
    ).toBe(0);
    expect(
      db.exec(
        'SELECT project_id,status FROM tasks GROUP BY project_id,status HAVING MIN(sort_order)!=0 OR MAX(sort_order)!=COUNT(*)-1',
      ),
    ).toEqual([]);
    expect(() => db.run('UPDATE tasks SET revision=-1')).toThrow();
    expect(() => db.run('UPDATE tasks SET sort_order=-1')).toThrow();
  } finally {
    db.close();
  }
});
