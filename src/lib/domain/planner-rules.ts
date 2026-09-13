import { Temporal } from '@js-temporal/polyfill';
import { dateAt, dayBounds } from './clock';
import { deadlineChoices } from './deadlines';
import type { BlockDraft, PlannerBlock, PlannerSnapshot, PlannerTask, PlannerItem, PlannerMeeting, PlanPreview, BatchMode } from './planner';
export const addDays = (date: string, days: number) => Temporal.PlainDate.from(date).add({ days }).toString();
export function previousMonday(date: string) {
 const d = Temporal.PlainDate.from(date);
 return d.subtract({ days: d.dayOfWeek === 1 ? 7 : d.dayOfWeek-1 }).toString();
}
export function minuteLabel(min: number) { return `${String(Math.floor(min/60)).padStart(2,'0')}:${String(min%60).padStart(2,'0')}`; }
export function endpointChoices(date: string, minute: number, zone: string) {
 if (!Number.isInteger(minute) || minute < 0 || minute > 1440) throw new Error('Time must be within this day.');
 return deadlineChoices(minute === 1440 ? addDays(date,1) : date, minuteLabel(minute === 1440 ? 0 : minute), zone);
}
export function normalizeBlock(draft: BlockDraft, date: string, zone: string): BlockDraft {
 if (![draft.start_min,draft.end_min].every(n=>Number.isInteger(n) && n%15===0) || draft.start_min<0 || draft.end_min>1440 || draft.end_min<=draft.start_min) throw new Error('Choose a range within one day in 15-minute steps.');
 if ((draft.kind==='task') !== Boolean(draft.task_id) || !['task','break','lunch','focus'].includes(draft.kind)) throw new Error('Choose a task or neutral block.');
 const resolve = (minute: number, offset?: string) => {
  const choices = endpointChoices(date,minute,zone);
  const selected = choices.find(c => c.offset===offset);
  if (choices.length>1 && !selected) throw new Error('This time occurs twice. Choose its UTC offset.');
  if (offset && !selected) throw new Error('This UTC offset does not match the selected time.');
  return selected ?? choices[0];
 };
 const start=resolve(draft.start_min,draft.start_offset), end=resolve(draft.end_min,draft.end_offset);
 if (end.instant<=start.instant) throw new Error('End must follow start in the selected offsets.');
 return {...draft,start_offset:start.offset,end_offset:end.offset};
}
export function localMinute(now: string, zone: string) {
 const z=Temporal.Instant.from(now).toZonedDateTimeISO(zone);
 return z.hour*60+z.minute+z.second/60+z.millisecond/60000;
}
export function automaticFloor(date: string, now: string, zone: string) {
 const today=dateAt(now,zone);
 return date<today ? null : date>today ? 420 : Math.ceil(localMinute(now,zone)/15)*15;
}
export interface Range { start_min: number; end_min: number }
export function nextFree(occupied: readonly Range[], duration: number, floor: number): Range | null {
 if (duration<=0 || !Number.isFinite(duration)) return null;
 let start=Math.ceil(floor/15)*15;
 for (const r of [...occupied].sort((a,b)=>a.start_min-b.start_min || a.end_min-b.end_min)) {
  if (r.end_min<=start) continue;
  if (start+duration<=r.start_min) break;
  start=Math.ceil(r.end_min/15)*15;
 }
 return start+duration<=1440 ? {start_min:start,end_min:start+duration} : null;
}
export const overlaps = (a: Range,b: Range) => a.start_min<b.end_min && b.start_min<a.end_min;
export function taskDuration(task: PlannerTask, automatic=false): number | null {
 if (task.estimate_h===null) return 30;
 const remaining=(task.estimate_h-task.hours_worked)*60;
 return remaining>0 ? Math.min(120,Math.ceil(remaining/15)*15) : automatic ? null : 15;
}
export function projectMeeting(meeting: PlannerMeeting, date: string, zone: string): PlannerItem | null {
 const bounds=dayBounds(date,zone), start=Date.parse(meeting.starts_at), end=start+meeting.duration_min*60000;
 if (end<=Date.parse(bounds.start) || start>=Date.parse(bounds.end)) return null;
 // A fixed wall-time grid collapses repeated DST hours. Cover every occupied
 // wall minute so automatic placement cannot overlap the other side of a fold.
 const clippedStart=Math.max(start,Date.parse(bounds.start)),clippedEnd=Math.min(end,Date.parse(bounds.end));
 let startMin=1440,endMin=0;
 const include=(ms:number)=>{const minute=Math.floor(localMinute(new Date(ms).toISOString(),zone));startMin=Math.min(startMin,minute);endMin=Math.max(endMin,minute+1);};
 for(let ms=clippedStart;ms<clippedEnd;ms+=60000)include(ms);
 include(clippedEnd-1);
 return {id:meeting.id,kind:'meeting',start_min:startMin,end_min:endMin,title:meeting.title,project_name:meeting.project_name,project_color:meeting.project_color,task_id:null,meeting};
}
export function plannerItems(snapshot: PlannerSnapshot, scope='all'): PlannerItem[] {
 const tasks=new Map(snapshot.tasks.map(t=>[t.id,t]));
 const blocks: PlannerItem[]=snapshot.blocks.filter(b=>!b.task_id || scope==='all' || tasks.get(b.task_id)?.project_id===scope).map(b=> {
  const task=b.task_id ? tasks.get(b.task_id) : undefined;
  return {id:b.id,kind:b.kind,start_min:b.start_min,end_min:b.end_min,title:task?.title ?? b.kind[0].toUpperCase()+b.kind.slice(1),project_name:task?.project_name ?? null,project_color:task?.project_color ?? null,task_id:b.task_id,task,block:b,session:snapshot.timers.sessions.find(s=>s.task_id===b.task_id)};
 });
 return [...blocks,...snapshot.meetings.filter(m=>scope==='all'||m.project_id===scope).map(m=>projectMeeting(m,snapshot.date,snapshot.time_zone)).filter((m):m is PlannerItem=>m!==null)];
}
const compare=(a:string,b:string)=>a<b?-1:a>b?1:0;
export function dueSoon(snapshot: PlannerSnapshot, scope='all') {
 const end=dayBounds(addDays(snapshot.date,7),snapshot.time_zone).start;
 const priority={high:0,medium:1,low:2};
 return snapshot.tasks.filter(t=>t.status!=='done' && t.due_at && t.due_at<end && (scope==='all'||t.project_id===scope)).sort((a,b)=>compare(a.due_at??'',b.due_at??'') || priority[a.priority]-priority[b.priority] || compare(a.title,b.title) || compare(a.id,b.id));
}
export function unscheduled(snapshot: PlannerSnapshot, scope='all') {
 return snapshot.tasks.filter(t=>t.status!=='done' && (scope==='all'||t.project_id===scope) && !snapshot.blocks.some(b=>b.task_id===t.id));
}
export function leftovers(snapshot: PlannerSnapshot, scope='all') {
 return snapshot.previous_blocks.filter(b=>b.kind==='task' && !b.done && !snapshot.claims.some(c=>c.id===`carry:${b.id}`) && snapshot.tasks.some(t=>t.id===b.task_id && t.status!=='done' && (scope==='all'||t.project_id===scope))).sort((a,b)=>a.start_min-b.start_min || compare(a.id,b.id));
}
export function plannerTotals(snapshot: PlannerSnapshot, scope='all') {
 const items=plannerItems(snapshot,scope), blocks=items.filter(i=>i.block);
 const bounds=dayBounds(snapshot.date,snapshot.time_zone);
 const plannedMinutes=(item:PlannerItem)=>item.meeting?(Math.min(Date.parse(item.meeting.starts_at)+item.meeting.duration_min*60000,Date.parse(bounds.end))-Math.max(Date.parse(item.meeting.starts_at),Date.parse(bounds.start)))/60000:item.end_min-item.start_min;
 return { planned:items.reduce((n,i)=>n+plannedMinutes(i),0), logged:snapshot.entries.filter(e=>scope==='all'||snapshot.tasks.some(t=>t.id===e.task_id&&t.project_id===scope)).reduce((n,e)=>n+e.minutes,0), done:blocks.filter(i=>i.block?.done).length, total:blocks.length };
}
export function previewPlan(snapshot: PlannerSnapshot, mode: BatchMode, now: string, scope='all', selectedIds?: string[], quickKind: 'break'|'lunch'|'focus'='break'): PlanPreview {
 const result: PlanPreview={mode,blocks:[],skipped:[],conflicts:[],fingerprint:snapshot.fingerprint};
 const floor=automaticFloor(snapshot.date,now,snapshot.time_zone);
 const occupied: Range[]=plannerItems(snapshot);
 const add=(id: string,draft: BlockDraft, duration: number, fixed=false) => {
  if (floor===null) { result.skipped.push({id,reason:'Automatic planning is unavailable on past dates.'}); return; }
  const range=fixed ? draft : nextFree(occupied,duration,floor);
  if (!range) { result.skipped.push({id,reason:'No space left in this day.'}); return; }
  if (fixed && occupied.some(r=>overlaps(r,range))) result.conflicts.push(id);
  result.blocks.push({...draft,start_min:range.start_min,end_min:range.end_min}); occupied.push(range);
 };
 const from=(b: PlannerBlock): BlockDraft=>({kind:b.kind,task_id:b.task_id,start_min:b.start_min,end_min:b.end_min,source_id:b.id});
 if (mode==='quick') add(quickKind,{kind:quickKind,task_id:null,start_min:0,end_min:0},quickKind==='break'?30:60);
 if (mode==='carry') for (const b of leftovers(snapshot,scope).filter(b=>!selectedIds||selectedIds.includes(b.id))) add(b.id,from(b),b.end_min-b.start_min);
 if (mode==='copy') for (const b of snapshot.copy_sources.filter(b=>!b.task_id||scope==='all'||snapshot.tasks.some(t=>t.id===b.task_id&&t.project_id===scope))) {
  if (snapshot.claims.some(c=>c.id===`copy:${b.id}:${snapshot.date}`)) result.skipped.push({id:b.id,reason:'Already copied to this date.'});
  else add(b.id,from(b),b.end_min-b.start_min,true);
 }
 if (mode==='due') for (const t of dueSoon(snapshot,scope)) {
  const duration=taskDuration(t,true);
  if (snapshot.blocks.some(b=>b.task_id===t.id)||snapshot.claims.some(c=>c.id===`due:${t.id}:${snapshot.date}`)) result.skipped.push({id:t.id,reason:'Already planned for this date.'});
  else if (duration===null) result.skipped.push({id:t.id,reason:'Estimate is exhausted.'});
  else add(t.id,{kind:'task',task_id:t.id,start_min:0,end_min:0,source_id:t.id},duration);
 }
 return result;
}
