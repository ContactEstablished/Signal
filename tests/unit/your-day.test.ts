// @vitest-environment jsdom
import { it,expect,vi,beforeEach,afterEach } from 'vitest';
import { mount,unmount,flushSync } from 'svelte';
import YourDay from '../../src/lib/views/YourDay.svelte';
import { Workspace } from '../../src/lib/state/app.svelte';
import type { PlannerSnapshot } from '../../src/lib/domain/planner';
let component:ReturnType<typeof mount>;
vi.mock('../../src/lib/native/planner',()=>({getPlanner:vi.fn(),applyPlanner:vi.fn()}));
beforeEach(()=>{vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});HTMLDialogElement.prototype.showModal=function(){this.open=true;};HTMLDialogElement.prototype.close=function(){this.open=false;};});
afterEach(async()=>{if(component)await unmount(component);document.body.innerHTML='';vi.unstubAllGlobals();});
function setup(){const w=new Workspace();w.nowUtc='2025-09-12T12:00:00Z';w.timeZone='UTC';w.planner.initialize('2025-09-12','UTC');w.planner.snapshot={date:'2025-09-12',time_zone:'UTC',blocks:[],previous_blocks:[],copy_sources:[],tasks:[{id:'created',title:'Newly created task',status:'todo',estimate_h:1,hours_worked:0,project_id:'p',project_name:'Project',project_color:'cyan',external_url:null,external_provider:null,external_id:null,priority:'medium',due_at:null,blocked_reason:null,blocked_on:null,notes_md:'',created_at:'2025-09-12T00:00:00.000Z',updated_at:'2025-09-12T00:00:00.000Z',done_at:null,sort_order:0,blocked_since:null,revision:0}],meetings:[],entries:[],claims:[],dismissed:false,fingerprint:'f',timers:{sessions:[],offset_ms:0,now_utc:w.nowUtc}} satisfies PlannerSnapshot;component=mount(YourDay,{target:document.body,props:{workspace:w,onOpenTask:vi.fn(),onNewTask:vi.fn(),onJoin:vi.fn(),onSummary:vi.fn()}});flushSync();return w;}
it('create-and-plan uses the exact created task without recreating it when scheduling fails',async()=>{
 const w=setup();const create=vi.spyOn(w,'createTask');vi.spyOn(w,'plannerAction').mockRejectedValue({code:'Validation',message:'No space'});(component as {planTask:(id:string)=>void}).planTask('created');flushSync();expect(document.querySelector<HTMLSelectElement>('select[required]')?.value).toBe('created');document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));await vi.waitFor(()=>expect(document.body.textContent).toContain('No space'));expect(create).not.toHaveBeenCalled();expect(w.planner.snapshot?.tasks[0].id).toBe('created');expect(w.planner.draftOpen).toBe(true);
});
it('refuses native close with unresolved timer or planner outcomes',async()=>{
 const w=setup();w.planner.recovery=true;expect(await (component as {requestClose:(r:string)=>Promise<boolean>}).requestClose('native-close')).toBe(false);w.planner.recovery=false;w.timers.recovery.created=true;expect(await (component as {requestClose:(r:string)=>Promise<boolean>}).requestClose('native-quit')).toBe(false);
});

it('quick-adds once at the next free slot without a preview, and reports no space without writing',async()=>{
 const w=setup();w.planner.snapshot!.blocks=[{id:'occupied',date:'2025-09-12',start_min:720,end_min:765,kind:'focus',task_id:null,done:0,done_at:null,carried_from_block_id:null,revision:0,time_zone:'UTC',start_offset:'+00:00',end_offset:'+00:00'}];flushSync();
 let resolve!:()=>void;
 const action=vi.spyOn(w,'plannerAction').mockImplementation(()=>new Promise(r=>{resolve=()=>r({snapshot:w.planner.snapshot!,outcome:{block_ids:['quick']},changed_details:[],changed_entries:{}});}));
 const button=document.querySelector<HTMLButtonElement>('.quick button')!;button.click();button.click();flushSync();
 expect(action).toHaveBeenCalledTimes(1);expect(action).toHaveBeenCalledWith('batch',{mode:'quick',blocks:[expect.objectContaining({kind:'break',start_min:765,end_min:795})]},'f');
 expect(document.querySelector('dialog[open]')).toBeNull();expect(button.disabled).toBe(true);
 resolve();await vi.waitFor(()=>expect(button.disabled).toBe(false));
 w.nowUtc='2025-09-12T23:50:00Z';flushSync();button.click();flushSync();
 expect(document.body.textContent).toContain('No space left in this day');expect(action).toHaveBeenCalledTimes(1);
});

it('shows empty-day planning actions and completed blocks according to date and project scope',()=>{
 const w=setup();expect(document.querySelector('.empty-hint')).not.toBeNull();expect(document.querySelector('section[aria-label="Empty day planning"]')).not.toBeNull();
 w.planner.snapshot!.blocks=[{id:'complete',date:'2025-09-12',start_min:540,end_min:600,kind:'task',task_id:'created',done:1,done_at:'2025-09-12T10:00:00Z',carried_from_block_id:null,revision:1,time_zone:'UTC',start_offset:'+00:00',end_offset:'+00:00'}];flushSync();
 expect(document.querySelector('.empty-hint')).toBeNull();expect(document.querySelector('section[aria-label="Completed blocks"]')?.textContent).toContain('Newly created task');expect(document.querySelector('section[aria-label="Completed blocks"]')?.textContent).toContain('Done today');
 w.planner.scope='other';flushSync();expect(document.querySelector('section[aria-label="Completed blocks"]')).toBeNull();expect(document.querySelector('.empty-hint')).not.toBeNull();
 w.planner.scope='all';w.nowUtc='2025-09-13T01:00:00Z';flushSync();expect(document.querySelector('section[aria-label="Completed blocks"]')?.textContent).toContain('Done on this day');
 w.planner.snapshot!.blocks[0].done=0;flushSync();expect(document.querySelector('section[aria-label="Completed blocks"]')).toBeNull();
});

it('schedules a Due soon task through its plus button without changing its due date',async()=>{
 const w=setup();const due='2025-09-13T12:00:00Z';w.planner.snapshot!.tasks[0].due_at=due;flushSync();
 const action=vi.spyOn(w,'plannerAction').mockResolvedValue({snapshot:w.planner.snapshot!,outcome:{block_ids:['new']},changed_details:[],changed_entries:{}});
 document.querySelector<HTMLButtonElement>('section[aria-label="Due soon"] button[title="Choose a time"]')!.click();flushSync();
 expect(document.querySelector<HTMLSelectElement>('select[required]')?.value).toBe('created');
 document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));
 await vi.waitFor(()=>expect(action).toHaveBeenCalledWith('create',expect.objectContaining({draft:expect.objectContaining({task_id:'created'})})));
 expect(w.planner.snapshot!.tasks[0].due_at).toBe(due);
});

it('drags a Due soon task onto the grid with its remaining duration and blocks drag during recovery',async()=>{
 const w=setup();w.planner.snapshot!.tasks[0].due_at='2025-09-13T12:00:00Z';w.planner.snapshot!.tasks[0].estimate_h=3;w.planner.snapshot!.tasks[0].hours_worked=1.5;flushSync();
 const action=vi.spyOn(w,'plannerAction').mockResolvedValue({snapshot:w.planner.snapshot!,outcome:{block_ids:['new']},changed_details:[],changed_entries:{}});
 const row=document.querySelector<HTMLElement>('.due-row')!;const data=new Map<string,string>();
 const transfer={setData:(type:string,value:string)=>data.set(type,value),getData:(type:string)=>data.get(type),effectAllowed:''};
 const start=new Event('dragstart',{bubbles:true,cancelable:true});Object.defineProperty(start,'dataTransfer',{value:transfer});row.dispatchEvent(start);
 expect(row.draggable).toBe(true);expect(data.get('application/x-signal-task-id')).toBe('created');
 const grid=document.querySelector<HTMLElement>('.canvas-scroll')!;grid.getBoundingClientRect=()=>new DOMRect(0,100,800,600);grid.scrollTop=420;
 const drop=new MouseEvent('drop',{bubbles:true,cancelable:true,clientY:220});Object.defineProperty(drop,'dataTransfer',{value:transfer});grid.dispatchEvent(drop);
 await vi.waitFor(()=>expect(action).toHaveBeenCalledWith('create',expect.objectContaining({draft:expect.objectContaining({task_id:'created',start_min:540,end_min:630})})));
 w.planner.recovery=true;flushSync();data.clear();const blocked=new Event('dragstart',{bubbles:true,cancelable:true});Object.defineProperty(blocked,'dataTransfer',{value:transfer});row.dispatchEvent(blocked);
 expect(row.draggable).toBe(false);expect(blocked.defaultPrevented).toBe(true);expect(data.size).toBe(0);
 expect(document.querySelector<HTMLButtonElement>('.due-row button[title="Choose a time"]')!.disabled).toBe(true);
});

it('moves the same block to the chosen date and follows it after save',async()=>{
 const w=setup();w.planner.snapshot!.blocks=[{id:'block',date:'2025-09-12',start_min:540,end_min:600,kind:'task',task_id:'created',done:0,done_at:null,carried_from_block_id:null,revision:3,time_zone:'UTC',start_offset:'+00:00',end_offset:'+00:00'}];flushSync();
 const action=vi.spyOn(w,'plannerAction').mockResolvedValue({snapshot:w.planner.snapshot!,outcome:{block_ids:['block'],destination_date:'2025-09-15'},changed_details:[],changed_entries:{}});
 const load=vi.spyOn(w.planner,'load').mockResolvedValue(undefined);
 document.querySelector<HTMLButtonElement>('.block-actions [aria-label="Edit time"]')!.click();flushSync();
 const date=document.querySelector<HTMLInputElement>('input[aria-label="Block date"]')!;date.value='2025-09-15';date.dispatchEvent(new Event('input',{bubbles:true}));date.dispatchEvent(new Event('change',{bubbles:true}));flushSync();
 document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));
 await vi.waitFor(()=>expect(w.planner.date).toBe('2025-09-15'));
 expect(action).toHaveBeenCalledWith('move',expect.objectContaining({id:'block',expectedRevision:3,destinationDate:'2025-09-15',start_min:540,end_min:600}));expect(w.planner.followToday).toBe(false);expect(load).toHaveBeenCalledWith('UTC');
});
