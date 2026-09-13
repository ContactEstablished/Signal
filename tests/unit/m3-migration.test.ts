import { readFileSync } from 'node:fs';
import initSqlJs from 'sql.js';
import { it,expect } from 'vitest';
import { seedTables } from '../../src/lib/seed';
it('preserves fixture wall times, cache and marker and detaches removed claim results',async()=>{
 const SQL=await initSqlJs(),db=new SQL.Database();
 try {
  db.run('PRAGMA foreign_keys=ON');
  for(const file of ['0001_initial.sql','0002_board_fields.sql','0003_timers.sql']) db.run(readFileSync('src-tauri/migrations/'+file,'utf8'));
  for(const table of seedTables) for(const row of table.rows) {const keys=Object.keys(row);db.run(`INSERT INTO ${table.table} (${keys.join(',')}) VALUES (${keys.map(()=>'?').join(',')})`,Object.values(row));}
  db.run("INSERT INTO settings VALUES('fixture_version','september-2025-midday-v1')");
  const fields='id,date,start_min,end_min,kind,task_id,done,done_at,carried_from_block_id';
  const before=db.exec(`SELECT ${fields} FROM blocks ORDER BY id`),tasks=db.exec('SELECT * FROM tasks ORDER BY id');
  db.run(readFileSync('src-tauri/migrations/0004_planner.sql','utf8'));
  expect(db.exec(`SELECT ${fields} FROM blocks ORDER BY id`)).toEqual(before);expect(db.exec('SELECT * FROM tasks ORDER BY id')).toEqual(tasks);
  db.run("INSERT INTO blocks(id,date,start_min,end_min,kind) VALUES('neutral','2025-09-12',420,480,'focus')");
  db.run("INSERT INTO planner_claims VALUES('copy:source:2025-09-12','copy','source',NULL,'2025-09-12','neutral')");
  db.run("DELETE FROM blocks WHERE id='neutral'");
  expect(db.exec('SELECT block_id FROM planner_claims')[0].values).toEqual([[null]]);
  expect(db.exec('PRAGMA foreign_key_check')).toEqual([]);
  expect(db.exec("SELECT value FROM settings WHERE key='fixture_version'")[0].values[0][0]).toBe('september-2025-midday-v1');
 }finally{db.close();}
});
