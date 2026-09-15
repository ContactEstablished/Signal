// @vitest-environment jsdom
import { it,expect,vi,beforeEach,afterEach } from 'vitest';
import { mount,unmount,flushSync } from 'svelte';
import YourDay from '../../src/lib/views/YourDay.svelte';
import { Workspace } from '../../src/lib/state/app.svelte';
import type { PlannerSnapshot } from '../../src/lib/domain/planner';
let component:ReturnType<typeof mount>;
vi.mock('../../src/lib/native/planner',()=>({getPlanner:vi.fn(),applyPlanner:vi.fn()}));
beforeEach(()=>{HTMLElement.prototype.setPointerCapture=vi.fn();HTMLElement.prototype.releasePointerCapture=vi.fn();HTMLElement.prototype.hasPointerCapture=()=>false;vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});HTMLDialogElement.prototype.showModal=function(){this.open=true;};HTMLDialogElement.prototype.close=function(){this.open=false;};});
afterEach(async()=>{if(component)await unmount(component);document.body.innerHTML='';vi.unstubAllGlobals();});
function setup(onOpenTask=vi.fn()){const w=new Workspace();w.nowUtc='2025-09-12T12:00:00Z';w.timeZone='UTC';w.planner.initialize('2025-09-12','UTC');w.planner.snapshot={date:'2025-09-12',time_zone:'UTC',blocks:[],previous_blocks:[],copy_sources:[],tasks:[{id:'created',title:'Newly created task',status:'todo',estimate_h:1,hours_worked:0,project_id:'p',project_name:'Project',project_color:'cyan',external_url:null,external_provider:null,external_id:null,priority:'medium',due_at:null,blocked_reason:null,blocked_on:null,notes_md:'',created_at:'2025-09-12T00:00:00.000Z',updated_at:'2025-09-12T00:00:00.000Z',done_at:null,sort_order:0,blocked_since:null,revision:0}],meetings:[],entries:[],claims:[],dismissed:false,fingerprint:'f',timers:{sessions:[],offset_ms:0,now_utc:w.nowUtc}} satisfies PlannerSnapshot;component=mount(YourDay,{target:document.body,props:{workspace:w,onOpenTask,onNewTask:vi.fn(),onJoin:vi.fn(),onSummary:vi.fn()}});flushSync();return w;}
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

function pointer(el:Element|Window,type:string,x:number,y:number){
 const e=new MouseEvent(type,{bubbles:true,cancelable:true,button:0,clientX:x,clientY:y});Object.defineProperty(e,'pointerId',{value:7});el.dispatchEvent(e);flushSync();return e;
}
function dueDragSetup(){
 const w=setup();w.planner.snapshot!.tasks[0].due_at='2025-09-13T12:00:00Z';w.planner.snapshot!.tasks[0].estimate_h=4;flushSync();
 const grid=document.querySelector<HTMLElement>('.canvas-scroll')!;grid.getBoundingClientRect=()=>new DOMRect(0,100,800,600);grid.scrollTop=420;
 const row=document.querySelector<HTMLElement>('.due-row')!;
 return {w,grid,row};
}
it('previews and saves a one-hour Due soon drag, then removes it only after successful persistence',async()=>{
 const {w,row}=dueDragSetup();const due=w.planner.snapshot!.tasks[0].due_at;
 let resolve!:()=>void;
 const action=vi.spyOn(w,'plannerAction').mockImplementation((_action,payload)=>new Promise(r=>{resolve=()=>{
  const d=(payload as {draft:{start_min:number;end_min:number;task_id:string}}).draft;
  w.planner.snapshot!.blocks=[{...d,id:'new',date:w.planner.date,kind:'task',done:0,done_at:null,carried_from_block_id:null,revision:0,time_zone:'UTC',start_offset:'+00:00',end_offset:'+00:00'}];
  r({snapshot:w.planner.snapshot!,outcome:{block_ids:['new']},changed_details:[],changed_entries:{}});
 };}));
 pointer(row,'pointerdown',900,200);pointer(window,'pointermove',250,220);
 expect(row.draggable).toBe(false);expect(document.querySelector('[data-task-drag-ghost]')?.textContent).toContain('1 hour');
 expect(document.querySelector('[data-task-drop-preview]')?.textContent).toContain('9:00 AM–10:00 AM');
 expect(document.querySelector('[data-task-drop-preview]')?.getAttribute('style')).toContain('height: 60px');
 pointer(window,'pointerup',250,220);pointer(window,'pointerup',250,220);
 expect(action).toHaveBeenCalledTimes(1);expect(action).toHaveBeenCalledWith('create',expect.objectContaining({draft:expect.objectContaining({task_id:'created',start_min:540,end_min:600})}));
 expect(document.querySelector('.due-row')).not.toBeNull();expect(document.querySelector('[data-task-drag-ghost]')).toBeNull();expect(document.querySelector('[data-task-drop-preview]')).toBeNull();
 resolve();await vi.waitFor(()=>expect(document.querySelector('.due-row')).toBeNull());
 expect(document.querySelector('[data-planner-item="new"]')?.textContent).toContain('Newly created task');
 expect(w.planner.snapshot!.tasks[0].due_at).toBe(due);expect(w.planner.snapshot!.tasks[0].estimate_h).toBe(4);
 document.querySelector<HTMLButtonElement>('.block-actions [aria-label="Edit time"]')!.click();flushSync();
 expect(document.querySelector<HTMLInputElement>('[aria-label="Start time"]')?.value).toBe('9:00 AM');
 expect(document.querySelector<HTMLInputElement>('[aria-label="End time"]')?.value).toBe('10:00 AM');
});
it('keeps a task in Due soon after a failed save',async()=>{
 const {w,row}=dueDragSetup();vi.spyOn(w,'plannerAction').mockRejectedValue(new Error('Save unavailable'));
 pointer(row,'pointerdown',900,200);pointer(window,'pointermove',250,220);pointer(window,'pointerup',250,220);
 await vi.waitFor(()=>expect(document.body.textContent).toContain('Save unavailable'));
 expect(document.querySelector('.due-row')).not.toBeNull();expect(w.planner.snapshot!.blocks).toHaveLength(0);
 expect(document.querySelector('[data-task-drop-preview]')).toBeNull();
});
it('cancels sidebar drags outside the calendar, with Escape, and during recovery',()=>{
 const {w,row}=dueDragSetup();const action=vi.spyOn(w,'plannerAction');
 pointer(row,'pointerdown',900,200);pointer(window,'pointermove',950,230);pointer(window,'pointerup',950,230);
 expect(action).not.toHaveBeenCalled();expect(document.querySelector('[data-task-drag-ghost]')).toBeNull();
 pointer(row,'pointerdown',900,200);pointer(window,'pointermove',250,220);
 window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));flushSync();pointer(window,'pointerup',250,220);
 expect(action).not.toHaveBeenCalled();expect(document.querySelector('[data-task-drop-preview]')).toBeNull();
 pointer(row,'pointerdown',900,200);pointer(window,'pointermove',250,220);w.planner.recovery=true;flushSync();pointer(window,'pointerup',250,220);
 expect(action).not.toHaveBeenCalled();expect(document.querySelector('[data-task-drag-ghost]')).toBeNull();
 expect(document.querySelector<HTMLButtonElement>('.due-row button[title="Choose a time"]')!.disabled).toBe(true);
});
it('returns scheduled tasks to Due soon when their block is removed',()=>{
 const {w}=dueDragSetup();
 w.planner.snapshot!.blocks=[{id:'existing',date:w.planner.date,start_min:540,end_min:600,kind:'task',task_id:'created',done:0,done_at:null,carried_from_block_id:null,revision:0,time_zone:'UTC',start_offset:'+00:00',end_offset:'+00:00'}];flushSync();
 expect(document.querySelector('.due-row')).toBeNull();
 expect(document.querySelector('section[aria-label="Due soon"]')?.textContent).toContain('No unscheduled tasks due soon');
 w.planner.snapshot!.blocks=[];flushSync();expect(document.querySelector('.due-row')).not.toBeNull();
});

it('keeps task clicks and keyboard activation available without opening a task after dragging',()=>{
 const open=vi.fn();const w=setup(open);w.planner.snapshot!.tasks[0].due_at='2025-09-13T12:00:00Z';flushSync();
 const title=document.querySelector<HTMLButtonElement>('.due-row button.due')!;
 pointer(title,'pointerdown',900,200);pointer(window,'pointerup',900,200);
 title.dispatchEvent(new MouseEvent('click',{bubbles:true,detail:1}));expect(open).toHaveBeenCalledTimes(1);
 pointer(title,'pointerdown',900,200);pointer(window,'pointermove',950,250);pointer(window,'pointerup',950,250);
 title.dispatchEvent(new MouseEvent('click',{bubbles:true,detail:1}));expect(open).toHaveBeenCalledTimes(1);
 title.click();expect(open).toHaveBeenCalledTimes(2);
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
