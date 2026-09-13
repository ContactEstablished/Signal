import { it,expect } from 'vitest';
import { automaticFloor,nextFree,normalizeBlock,previousMonday,previewPlan,plannerTotals,dueSoon,taskDuration,projectMeeting } from '../../src/lib/domain/planner-rules';
import type { PlannerSnapshot,PlannerTask,PlannerBlock } from '../../src/lib/domain/planner';
const task=(id:string,extra:Partial<PlannerTask>={}):PlannerTask=>({id,project_id:'p',project_name:'P',project_color:'cyan',title:id,status:'todo',priority:'medium',estimate_h:null,hours_worked:0,due_at:'2025-09-11T12:00:00.000Z',...extra} as PlannerTask);
const block=(id:string,start_min:number,end_min:number,extra:Partial<PlannerBlock>={}):PlannerBlock=>({id,date:'2025-09-12',start_min,end_min,kind:'task',task_id:'a',done:0,...extra} as PlannerBlock);
const snapshot=():PlannerSnapshot=>({date:'2025-09-12',time_zone:'America/New_York',blocks:[],previous_blocks:[],copy_sources:[],tasks:[task('a')],meetings:[],entries:[],claims:[],dismissed:false,fingerprint:'f',timers:{sessions:[],now_utc:'2025-09-11T17:42:00Z',offset_ms:0}});
it('reserves both sides of a meeting crossing a DST fold while totals retain actual duration',()=>{
 const s=snapshot();s.date='2026-11-01';s.meetings=[{id:'fold',project_id:'p',project_name:'P',project_color:'cyan',title:'Fold meeting',starts_at:'2026-11-01T05:45:00Z',duration_min:30,link_url:null,agenda_md:'',notes_md:'',repeat_rule:'none',reminder_min:15}];
 expect(projectMeeting(s.meetings[0],s.date,s.time_zone)).toMatchObject({start_min:60,end_min:120});
 expect(plannerTotals(s).planned).toBe(30);
 expect(previewPlan(s,'quick','2026-11-01T05:00:00Z').blocks[0]).toMatchObject({start_min:120,end_min:150});
});
it('uses real calendar boundaries and today/future automatic floors',()=>{
 expect(previousMonday('2025-09-15')).toBe('2025-09-08');
 expect(previousMonday('2026-01-01')).toBe('2025-12-29');
 expect(automaticFloor('2025-09-11','2025-09-11T17:42:00Z','America/New_York')).toBe(825);
 expect(automaticFloor('2025-09-12','2025-09-11T17:42:00Z','America/New_York')).toBe(420);
 expect(automaticFloor('2025-09-10','2025-09-11T17:42:00Z','America/New_York')).toBeNull();
 expect(nextFree([{start_min:420,end_min:460},{start_min:450,end_min:500}],30,420)).toEqual({start_min:510,end_min:540});
 expect(nextFree([],30,1425)).toBeNull();
});
it('requires real DST endpoints and explicit fold offsets without shifting wall times',()=>{
 const draft={kind:'focus' as const,task_id:null,start_min:120,end_min:180};
 expect(()=>normalizeBlock(draft,'2026-03-08','America/New_York')).toThrow('does not exist');
 const fold={...draft,start_min:60,end_min:120};
 expect(()=>normalizeBlock(fold,'2026-11-01','America/New_York')).toThrow('twice');
 expect(normalizeBlock({...fold,start_offset:'-05:00'},'2026-11-01','America/New_York')).toMatchObject({start_min:60,start_offset:'-05:00',end_offset:'-05:00'});
 expect(()=>normalizeBlock({...fold,start_offset:'+01:00'},'2026-11-01','America/New_York')).toThrow();
 expect(normalizeBlock({...draft,start_min:1425,end_min:1440},'2026-03-08','America/New_York').end_offset).toBe('-04:00');
});
it('automatic scoped proposals avoid hidden projects and stored meetings',()=>{
 const s=snapshot();s.tasks.push(task('hidden',{project_id:'other'}));s.blocks=[block('busy',420,480,{task_id:'hidden'})];
 s.meetings=[{id:'m',project_id:'other',project_name:'Other',project_color:'lime',title:'Meeting',starts_at:'2025-09-12T12:00:00Z',duration_min:30} as never];
 expect(previewPlan(s,'due','2025-09-11T17:42:00Z','p').blocks[0]).toMatchObject({start_min:510,end_min:540,task_id:'a'});
 expect(plannerTotals(s,'p').planned).toBe(0);
 expect(plannerTotals(s).planned).toBe(90);
});
it('preserves carry lengths, source order and global claims; copy uses independent provenance',()=>{
 const s=snapshot();s.previous_blocks=[block('later',600,720),block('first',540,585)];
 s.copy_sources=[block('monday',500,560,{done:1})];
 expect(previewPlan(s,'carry','2025-09-11T17:42:00Z').blocks.map(b=>[b.source_id,b.end_min-b.start_min])).toEqual([['first',45],['later',120]]);
 s.claims=[{id:'carry:first',mode:'carry',source_id:'first',task_id:'a',destination_date:s.date,block_id:null}];
 expect(previewPlan(s,'carry','2025-09-11T17:42:00Z').blocks.map(b=>b.source_id)).toEqual(['later']);
 expect(previewPlan(s,'copy','2025-09-11T17:42:00Z').blocks[0]).toMatchObject({source_id:'monday',start_min:500,end_min:560});
});
it('handles due cutoff, unknown/exhausted estimates, totals and repeated due claims',()=>{
 const s=snapshot();s.tasks=[task('overdue',{estimate_h:10,hours_worked:1}),task('unknown'),task('exhausted',{estimate_h:1,hours_worked:2}),task('outside',{due_at:'2025-09-19T04:00:00.000Z'}),task('done',{status:'done'})];
 expect(taskDuration(s.tasks[0])).toBe(120);expect(taskDuration(s.tasks[2])).toBe(15);
 expect(dueSoon(s).map(t=>t.id)).toEqual(['exhausted','overdue','unknown']);
 const plan=previewPlan(s,'due','2025-09-11T17:42:00Z');expect(plan.blocks).toHaveLength(2);expect(plan.skipped[0].reason).toContain('exhausted');
 s.blocks=[block('one',540,600,{done:1}),block('two',540,600,{kind:'focus',task_id:null})];s.entries=[{id:'e',task_id:'overdue',minutes:12.5} as never];
 expect(plannerTotals(s)).toEqual({planned:120,logged:12.5,done:1,total:2});
 s.claims=[{id:`due:unknown:${s.date}`,mode:'due',source_id:'unknown',task_id:'unknown',destination_date:s.date,block_id:null}];
 expect(previewPlan(s,'due','2025-09-11T17:42:00Z').blocks.map(b=>b.task_id)).toEqual(['overdue']);
});
