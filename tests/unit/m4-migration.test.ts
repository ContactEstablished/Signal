import { it, expect } from 'vitest';
import initSqlJs from 'sql.js';
import { readFileSync } from 'node:fs';
it('upgrades legacy rows unchanged and detaches occurrence links without deleting foreign tasks', async () => {
  const SQL = await initSqlJs();
  const db = new SQL.Database();
  db.run('PRAGMA foreign_keys=ON');
  for (const file of [
    '0001_initial.sql',
    '0002_board_fields.sql',
    '0003_timers.sql',
    '0004_planner.sql',
  ])
    db.run(readFileSync(`src-tauri/migrations/${file}`, 'utf8'));
  db.run(
    "INSERT INTO projects(id,name,color,sort_order) VALUES('p','Legacy','cyan',0); INSERT INTO tasks(id,project_id,title,created_at,updated_at) VALUES('t','p','Keep','2025-09-11T00:00:00.000Z','2025-09-11T00:00:00.000Z'); INSERT INTO meetings(id,project_id,title,starts_at,duration_min) VALUES('m','p','Old','2025-11-02T06:30:12.345Z',30); INSERT INTO meeting_tasks VALUES('m','t');",
  );
  db.run(readFileSync('src-tauri/migrations/0005_agenda.sql', 'utf8'));
  expect(
    db.exec('SELECT starts_at,time_zone,active_version_id FROM meetings')[0]
      .values,
  ).toEqual([['2025-11-02T06:30:12.345Z', null, null]]);
  db.run(
    "INSERT INTO meeting_occurrences(meeting_id,occurrence_key,ordinal,updated_at) VALUES('m','o:0',0,'2025-09-11T00:00:00.000Z'); INSERT INTO meeting_occurrence_tasks VALUES('m','o:0','t'); DELETE FROM meeting_tasks WHERE meeting_id='m'; DELETE FROM meetings WHERE id='m';",
  );
  expect(db.exec('SELECT id FROM tasks')[0].values).toEqual([['t']]);
  expect(db.exec('SELECT * FROM meeting_occurrence_tasks')).toEqual([]);
  expect(db.exec('PRAGMA foreign_key_check')).toEqual([]);
  db.close();
});
