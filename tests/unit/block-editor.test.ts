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
 const save=setup();button('Start 15 minutes earlier').click();flushSync();expect(field('Start time').value).toBe('8:45 AM');
 const duration=[...document.querySelectorAll<HTMLButtonElement>('.duration button')].find(b=>b.textContent==='1.5h')!;duration.click();flushSync();expect(field('End time').value).toBe('10:15 AM');
 change('Block date','2025-09-15');submit();await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:525,end_min:615}),'2025-09-15'));
});
it('keeps adjustments inside the day and rejects malformed wall times',async()=>{
 const save=setup({kind:'break',task_id:null,start_min:1425,end_min:1440});expect(button('End 15 minutes later').disabled).toBe(true);expect(button('Start 15 minutes later').disabled).toBe(true);expect([...document.querySelectorAll<HTMLButtonElement>('.duration button')].every(b=>b.disabled)).toBe(true);
 change('Start time','13:90');submit();await vi.waitFor(()=>expect(document.querySelector('[role="alert"]')).not.toBeNull());expect(save).not.toHaveBeenCalled();
});

it('saves an unchanged end-of-day midnight and preserves it after nudging', async () => {
 const save=setup({kind:'focus',task_id:null,start_min:1380,end_min:1440});
 expect(field('End time').value).toBe('12:00 AM (next day)');
 button('End 15 minutes earlier').click();flushSync();
 expect(field('End time').value).toBe('11:45 PM');
 button('End 15 minutes later').click();flushSync();submit();
 await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:1380,end_min:1440}),'2025-09-12'));
});
it('clears old offset choices when moving to a DST fold and preserves the draft on failure',async()=>{
 const save=setup({kind:'focus',task_id:null,start_min:60,end_min:120,start_offset:'-04:00',end_offset:'-04:00'},'2026-10-31');
 change('Block date','2026-11-01');const offset=document.querySelector<HTMLSelectElement>('[aria-label="Start UTC offset"]')!;expect(offset.value).toBe('');submit();expect(save).not.toHaveBeenCalled();
 offset.value='-05:00';offset.dispatchEvent(new Event('change',{bubbles:true}));flushSync();save.mockRejectedValue(new Error('Write unavailable'));submit();await vi.waitFor(()=>expect(document.body.textContent).toContain('Write unavailable'));expect(field('Block date').value).toBe('2026-11-01');expect(offset.value).toBe('-05:00');
});

it('allows browser submission with 2pm and an unsuffixed 2:30 end',async()=>{
 const save=setup();change('Start time','2pm');change('End time','2:30');
 const form=document.querySelector('form')!;
 expect(field('Start time').hasAttribute('pattern')).toBe(false);
 expect(form.checkValidity()).toBe(true);
 form.requestSubmit();flushSync();
 await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:840,end_min:870}),'2025-09-12'));
 expect(field('Start time').value).toBe('2:00 PM');expect(field('End time').value).toBe('2:30 PM');
});

it('normalizes an afternoon start on blur and accepts mixed 12/24-hour input',async()=>{
 const save=setup();change('Start time','2:30');
 field('Start time').dispatchEvent(new FocusEvent('blur'));flushSync();
 expect(field('Start time').value).toBe('2:30 PM');
 change('End time','15:30');submit();
 await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:870,end_min:930}),'2025-09-12'));
});

it('keeps explicit early-morning times and supports duration and nudges after shorthand',async()=>{
 const save=setup();change('Start time','2am');change('End time','3am');
 field('Start time').dispatchEvent(new FocusEvent('blur'));flushSync();
 expect(field('Start time').value).toBe('2:00 AM');
 button('Start 15 minutes later').click();flushSync();expect(field('Start time').value).toBe('2:15 AM');
 const duration=[...document.querySelectorAll<HTMLButtonElement>('.duration button')].find(b=>b.textContent==='1.5h')!;
 duration.click();flushSync();expect(field('End time').value).toBe('3:45 AM');submit();
 await vi.waitFor(()=>expect(save).toHaveBeenCalledWith(expect.objectContaining({start_min:135,end_min:225}),'2025-09-12'));
});

it('provides a useful error without losing malformed input',async()=>{
 const save=setup();change('Start time','2:90');change('End time','3pm');
 field('Start time').dispatchEvent(new FocusEvent('blur'));flushSync();submit();
 await vi.waitFor(()=>expect(document.body.textContent).toContain('Enter a start time like 2pm, 2:30, or 14:30.'));
 expect(field('Start time').value).toBe('2:90');expect(save).not.toHaveBeenCalled();
});

it('still rejects reversed ranges and times off the planner quarter-hour grid',async()=>{
 const save=setup();change('Start time','3pm');change('End time','2pm');submit();
 await vi.waitFor(()=>expect(document.body.textContent).toContain('Choose a range within one day in 15-minute steps.'));
 expect(save).not.toHaveBeenCalled();change('Start time','2:10');change('End time','3pm');submit();
 await vi.waitFor(()=>expect(document.querySelector('[role="alert"]')).not.toBeNull());expect(save).not.toHaveBeenCalled();
});
