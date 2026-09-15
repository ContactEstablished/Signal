// @vitest-environment jsdom
import { it,expect,vi,beforeEach,afterEach } from 'vitest';
import { mount,unmount,flushSync } from 'svelte';
import PlannerCanvas from '../../src/lib/components/planner/PlannerCanvas.svelte';
import TaskPicker from '../../src/lib/components/planner/TaskPicker.svelte';
import type { PlannerItem,PlannerTask } from '../../src/lib/domain/planner';
let component:ReturnType<typeof mount>;
beforeEach(()=>{
 vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});
 HTMLElement.prototype.setPointerCapture=vi.fn();HTMLElement.prototype.releasePointerCapture=vi.fn();HTMLElement.prototype.hasPointerCapture=()=>false;HTMLElement.prototype.scrollIntoView=vi.fn();
 HTMLDialogElement.prototype.showModal=function(){this.open=true;};HTMLDialogElement.prototype.close=function(){this.open=false;};
});
afterEach(async()=>{if(component)await unmount(component);document.body.innerHTML='';vi.unstubAllGlobals();});
function pointer(el:Element,type:string,y:number){const e=new MouseEvent(type,{bubbles:true,button:0,clientX:200,clientY:y});Object.defineProperty(e,'pointerId',{value:1});el.dispatchEvent(e);flushSync();}
function setup(items:PlannerItem[]=[],move=vi.fn().mockResolvedValue(undefined)){
 const props={items,date:'2025-09-12',timeZone:'UTC',nowUtc:'2025-09-12T12:00:00Z',selection:null,onSelect:vi.fn(),onCancelSelection:vi.fn(),onMove:move,onDropTask:vi.fn(),onOpenTask:vi.fn(),onDone:vi.fn(),onRemove:vi.fn(),onTimer:vi.fn(),onJoin:vi.fn(),onEditTime:vi.fn()};
 component=mount(PlannerCanvas,{target:document.body,props});flushSync();const wrapper=document.querySelector('.canvas-scroll') as HTMLDivElement;wrapper.getBoundingClientRect=()=>new DOMRect(0,100,800,600);wrapper.scrollTop=420;return{props,wrapper,grid:document.querySelector('.grid')!};
}
it('selects a scrolled reverse range and cancels captured gestures without writing',()=>{
 const {props,wrapper,grid}=setup();pointer(grid,'pointerdown',310);pointer(wrapper,'pointerup',220);expect(props.onSelect).toHaveBeenCalledWith(540,630,expect.any(DOMRect));
 pointer(grid,'pointerdown',220);pointer(wrapper,'pointercancel',310);pointer(wrapper,'pointerup',310);expect(props.onSelect).toHaveBeenCalledTimes(1);expect(props.onMove).not.toHaveBeenCalled();
});

it('labels the calendar and current-time marker in AM/PM', () => {
 setup();
 expect([...document.querySelectorAll('.hour span')].map(e=>e.textContent)).toContain('1:00 PM');
 expect(document.querySelector('.now time')?.textContent).toBe('12:00 PM');
});
it('preserves move duration, submits once and restores the source after rejection',async()=>{
 let reject!:(e:Error)=>void;const move=vi.fn(()=>new Promise<void>((_,r)=>reject=r));
 const block={id:'b',kind:'focus',start_min:540,end_min:600,title:'Focus',project_name:null,project_color:null,task_id:null,block:{id:'b',date:'2025-09-12',done:0}} as PlannerItem;
 const {wrapper}=setup([block],move);pointer(document.querySelector('.grip')!,'pointerdown',220);pointer(wrapper,'pointerup',280);pointer(wrapper,'pointerup',280);expect(move).toHaveBeenCalledTimes(1);expect(move).toHaveBeenCalledWith('b',600,660);reject(new Error('Save failed'));await vi.waitFor(()=>expect(document.body.textContent).toContain('Save failed'));expect(document.querySelector('.position')?.getAttribute('style')).toContain('540px');expect(document.querySelector('.source')).toBeNull();
});
it('limits resizing to at least fifteen minutes and keeps meetings read-only',async()=>{
 const block={id:'b',kind:'focus',start_min:540,end_min:600,title:'Focus',project_name:null,project_color:null,task_id:null,block:{id:'b',date:'2025-09-12',done:0}} as PlannerItem;
 const meeting={id:'m',kind:'meeting',start_min:600,end_min:630,title:'Meeting',project_name:'P',project_color:'cyan',task_id:null,meeting:{id:'m',link_url:null}} as PlannerItem;
 const {wrapper,props}=setup([block,meeting]);pointer(document.querySelector('.resize')!,'pointerdown',280);pointer(wrapper,'pointerup',100);expect(props.onMove).toHaveBeenCalledWith('b',540,555);expect(document.querySelector('[data-planner-item="m"] .resize')).toBeNull();expect(document.querySelector('[data-planner-item="m"] .check')).toBeNull();
});
it('filters picker tasks, supports keyboard choice, and keeps a failed selection open',async()=>{
 let rejectSave!:(error:Error)=>void;
 const pick=vi.fn(()=>new Promise<void>((_,reject)=>rejectSave=reject)),cancel=vi.fn();
 component=mount(TaskPicker,{target:document.body,props:{range:{startMin:540,endMin:630},anchor:new DOMRect(700,300,0,90),tasks:[{id:'a',title:'Alpha',project_name:'One'},{id:'b',title:'Beta',project_name:'Two'}] as PlannerTask[],pending:false,error:'',onPick:pick,onCancel:cancel}});
 flushSync();
 const input=document.querySelector('input')!;
 input.value='Two';input.dispatchEvent(new Event('input',{bubbles:true}));flushSync();
 input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));flushSync();
 expect(pick).toHaveBeenCalledWith({kind:'task',taskId:'b'});
 const dialog=document.querySelector('dialog') as HTMLDialogElement;
 const cancelButton=[...dialog.querySelectorAll('button')].find(b=>b.textContent?.trim()==='Cancel')!;
 expect(cancelButton.disabled).toBe(true);
 dialog.dispatchEvent(new Event('cancel',{cancelable:true}));
 expect(cancel).not.toHaveBeenCalled();
 rejectSave(new Error('offline'));
 // Wait for the user-visible end of the pending save, not just its invocation.
 await vi.waitFor(()=>expect(cancelButton.disabled).toBe(false));
 expect(dialog.open).toBe(true);expect(input.value).toBe('Two');
 dialog.dispatchEvent(new Event('cancel',{cancelable:true}));
 expect(cancel).toHaveBeenCalledOnce();
});

it('previews the same snapped one-hour range that is saved, including near midnight',async()=>{
 const {wrapper,props}=setup();
 const canvas=component as {previewTaskDrop:(d:{id:string;title:string;project_color:'cyan';x:number;y:number}|null)=>void;dropTaskAt:(id:string,x:number,y:number)=>Promise<void>};
 canvas.previewTaskDrop({id:'task',title:'Due soon task',project_color:'cyan',x:250,y:228});flushSync();
 expect(document.querySelector('[data-task-drop-preview]')?.textContent).toContain('9:15 AM–10:15 AM');
 await canvas.dropTaskAt('task',250,228);flushSync();expect(props.onDropTask).toHaveBeenCalledWith('task',555);
 expect(document.querySelector('[data-task-drop-preview]')).toBeNull();
 wrapper.scrollTop=1000;
 canvas.previewTaskDrop({id:'task',title:'Due soon task',project_color:'cyan',x:250,y:680});flushSync();
 expect(document.querySelector('[data-task-drop-preview]')?.textContent).toContain('11:00 PM–12:00 AM (next day)');
 await canvas.dropTaskAt('task',250,680);expect(props.onDropTask).toHaveBeenLastCalledWith('task',1380);
 await canvas.dropTaskAt('task',20,200);expect(props.onDropTask).toHaveBeenCalledTimes(2);
});
