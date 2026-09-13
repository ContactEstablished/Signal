// @vitest-environment jsdom
import {it,expect,vi,beforeEach,afterEach} from 'vitest';
import {mount,unmount,flushSync} from 'svelte';
import BlockEditor from '../../src/lib/components/planner/BlockEditor.svelte';
import type {BlockDraft} from '../../src/lib/domain/planner';
let component:ReturnType<typeof mount>;
beforeEach(()=>{HTMLDialogElement.prototype.showModal=function(){this.open=true;};});
afterEach(async()=>{if(component)await unmount(component);document.body.innerHTML='';});
function setup(draft:BlockDraft={kind:'focus',task_id:null,start_min:540,end_min:600},date='2025-09-12'){
 const save=vi.fn().mockResolvedValue(undefined);
 component=mount(BlockEditor,{target:document.body,props:{draft,date,timeZone:'America/New_York',tasks:[],editing:true,pending:false,onSave:save,onClose:vi.fn(),onRetry:vi.fn()}});flushSync();return save;
}
function button(label:string){return document.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;}
function field(label:string){return document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;}
function change(label:string,value:string){const input=field(label);input.value=value;input.dispatchEvent(new Event('input',{bubbles:true}));input.dispatchEvent(new Event('change',{bubbles:true}));flushSync();}
function submit(){document.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));flushSync();}
it('nudges time and applies a duration on the chosen destination date',async()=>{
 const save=setup();button('Start 15 minutes earlier').click();flushSync();expect(field('Start time').value).toBe('08:45');
 const duration=[...document.querySelectorAll<HTMLButtonElement>('.duration button')].find(b=>b.textContent==='1.5h')!;duration.click();flushSync();expect(field('End time').value).toBe('10:15');
 change('Block date','2025-09-15');submit();await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:525,end_min:615}),'2025-09-15'));
});
it('keeps adjustments inside the day and rejects malformed wall times',async()=>{
 const save=setup({kind:'break',task_id:null,start_min:1425,end_min:1440});expect(button('End 15 minutes later').disabled).toBe(true);expect(button('Start 15 minutes later').disabled).toBe(true);expect([...document.querySelectorAll<HTMLButtonElement>('.duration button')].every(b=>b.disabled)).toBe(true);
 change('Start time','13:90');submit();await vi.waitFor(()=>expect(document.querySelector('[role="alert"]')).not.toBeNull());expect(save).not.toHaveBeenCalled();
});
it('clears old offset choices when moving to a DST fold and preserves the draft on failure',async()=>{
 const save=setup({kind:'focus',task_id:null,start_min:60,end_min:120,start_offset:'-04:00',end_offset:'-04:00'},'2026-10-31');
 change('Block date','2026-11-01');const offset=document.querySelector<HTMLSelectElement>('[aria-label="Start UTC offset"]')!;expect(offset.value).toBe('');submit();expect(save).not.toHaveBeenCalled();
 offset.value='-05:00';offset.dispatchEvent(new Event('change',{bubbles:true}));flushSync();save.mockRejectedValue(new Error('Write unavailable'));submit();await vi.waitFor(()=>expect(document.body.textContent).toContain('Write unavailable'));expect(field('Block date').value).toBe('2026-11-01');expect(offset.value).toBe('-05:00');
});
