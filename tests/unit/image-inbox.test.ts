// @vitest-environment jsdom
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import VoiceInbox from '../../src/lib/views/VoiceInbox.svelte';
import { meetingIssue } from '../../src/lib/voice/review';
import { meetingStart } from '../../src/lib/domain/agenda-calendar';
import type { Draft, Candidate } from '../../src/lib/voice/api';
import type { Workspace } from '../../src/lib/state/app.svelte';
const api = vi.hoisted(() => ({load:vi.fn(),save:vi.fn(),recovery:vi.fn(),setRecovery:vi.fn(),suggest:vi.fn(),cancel:vi.fn(),discard:vi.fn(),image:vi.fn(),setImage:vi.fn(),setImagePath:vi.fn(),subscribeDrop:vi.fn(),newDraft:vi.fn()}));
vi.mock('../../src/lib/voice/image-api',()=>api);
let instance: ReturnType<typeof mount> | undefined;
let draft: Draft;
const task = (): Candidate => ({id:'task',kind:'task',selected:true,project_id:'p',proposal_id:null,title:'Write notes',notes:'',subtasks:[],due_at:null,priority:'medium',estimate_h:null,source:'Bottom row',warning:'',duplicate_ok:false,created_id:null});
const meeting = (): Candidate => ({...task(),id:'meeting',kind:'meeting',title:'Review',meeting:{title:'Review',starts_at:'',start_local:'',start_offset:'',time_zone:'America/New_York',duration_min:0,link_url:null,agenda_md:'',notes_md:'',reminder_min:15,show_in_day:true,task_ids:[],repeat_rule:'none',repeat_weekdays:[],repeat_until:null,fold_policy:'earlier'}});
const button = (text:string) => [...document.querySelectorAll('button')].find(b=>b.textContent?.trim()===text)!;
beforeEach(()=>{
  vi.resetAllMocks();
  HTMLDialogElement.prototype.showModal=function(){this.open=true;};
  HTMLDialogElement.prototype.close=function(){this.open=false;};
  draft={id:'image',source_kind:'image',image_id:'source',revision:1,recorded_at:'2026-09-28T16:00:00.000Z',time_zone:'America/New_York',transcript:'Make meetings and tasks',projects:[],candidates:[task(),meeting()]};
  api.load.mockImplementation(async()=>structuredClone(draft));
  api.save.mockImplementation(async(d)=>{draft={...structuredClone(d),revision:d.revision+1};return structuredClone(draft);});
  api.image.mockResolvedValue('data:image/png;base64,abc');
  api.recovery.mockResolvedValue(null);
  api.subscribeDrop.mockResolvedValue(()=>{});
  api.cancel.mockResolvedValue(undefined);
});
afterEach(async()=>{ if(instance) await unmount(instance); instance=undefined; document.body.innerHTML=''; vi.useRealTimers(); });
async function setup(){
  const workspace={nowUtc:draft.recorded_at,timeZone:draft.time_zone,projects:[{id:'p',name:'Atlas',color:'cyan'}],imageRecovery:false,voiceRecovery:false,acceptImage:vi.fn(async(input)=>{
    for(const c of draft.candidates) if(input.candidate_ids.includes(c.id)){c.created_id='created-'+c.id;c.selected=false;}
    draft.revision++;
    return {created:[],projects:[]};
  })};
  const props={workspace:workspace as unknown as Workspace,mode:'image' as const,onOpenTask:vi.fn(),onOpenMeeting:vi.fn(),onSettings:vi.fn(),onGuard:vi.fn()};
  instance=mount(VoiceInbox,{target:document.body,props});
  await vi.waitFor(()=>expect(button('Analyze image')).toBeTruthy());
  return {...props,workspace};
}
it('restores the image and blocks incomplete selected meetings while allowing selected tasks',async()=>{
  const p=await setup();
  expect(document.querySelector('img')?.getAttribute('src')).toContain('data:image/png');
  expect(button('Create selected (2)').disabled).toBe(true);
  document.querySelectorAll<HTMLInputElement>('article')[1]?.querySelector<HTMLInputElement>('input[type=checkbox]')?.click(); flushSync();
  await vi.waitFor(()=>expect(button('Create selected (1)').disabled).toBe(false));
  button('Create selected (1)').click();
  await vi.waitFor(()=>expect(p.workspace.acceptImage).toHaveBeenCalledOnce());
  expect(p.workspace.acceptImage.mock.calls[0][0].candidate_ids).toEqual(['task']);
  expect(draft.candidates[1].created_id).toBeNull();
  expect(p.workspace.voiceRecovery).toBe(false);
});
it('preserves an edited prompt and suggestions on failed analysis and ignores cancelled replies',async()=>{
  await setup();
  let finish!: (d:Draft)=>void;
  api.suggest.mockImplementation(()=>new Promise< Draft >(resolve=>finish=resolve));
  button('Analyze image').click(); await vi.waitFor(()=>expect(button('Continue')).toBeTruthy()); button('Continue').click();
  await vi.waitFor(()=>expect(api.suggest).toHaveBeenCalledOnce());
  button('Cancel analysis').click(); await vi.waitFor(()=>expect(api.cancel).toHaveBeenCalledOnce());
  finish({...draft,candidates:[]});
  await new Promise(resolve=>setTimeout(resolve,20));flushSync();
  expect(document.querySelectorAll('article')).toHaveLength(2);
  expect((document.querySelector('[aria-label=Instructions]') as HTMLTextAreaElement).value).toBe(draft.transcript);
  api.suggest.mockRejectedValue(new Error('Unsupported image model'));
  button('Analyze image').click(); await vi.waitFor(()=>expect(button('Continue')).toBeTruthy()); button('Continue').click();
  await vi.waitFor(()=>expect(document.body.textContent).toContain('Unsupported image model'));
  expect(document.querySelectorAll('article')).toHaveLength(2);
});
it('restores the exact image acceptance identity without touching voice recovery',async()=>{
  const pending={request_id:'original',draft_id:'image',revision:1,candidate_ids:['task']};api.recovery.mockResolvedValue(pending);
  const p=await setup();
  button('Retry item creation').click();
  await vi.waitFor(()=>expect(p.workspace.acceptImage).toHaveBeenCalledWith(pending));
  await vi.waitFor(()=>expect(api.setRecovery).toHaveBeenLastCalledWith(null));
  expect(p.workspace.imageRecovery).toBe(false);
});
it('accepts exactly one pasted image and reconciles source replacement before review',async()=>{
  draft.candidates=[];
  await setup();
  api.setImage.mockImplementation(async(d)=>({...d,revision:d.revision+1,image_id:'replacement'}));
  const event=new Event('paste',{cancelable:true});
  Object.defineProperty(event,'clipboardData',{value:{files:[{size:3,arrayBuffer:async()=>new Uint8Array([1,2,3]).buffer}]}});
  window.dispatchEvent(event);
  await vi.waitFor(()=>expect(api.setImage).toHaveBeenCalledWith(expect.objectContaining({id:'image'}),[1,2,3]));
  expect(event.defaultPrevented).toBe(true);
  await vi.waitFor(()=>expect(api.image).toHaveBeenLastCalledWith('replacement'));
});
it('requires a deliberate DST offset and valid recurrence before meeting acceptance',()=>{
  const c=meeting(); const m=c.meeting!;
  m.start_local='2026-11-01T01:30:00';m.duration_min=30;
  expect(meetingIssue(c)).toContain('occurs twice');
  Object.assign(m,meetingStart({date:'2026-11-01',time:'01:30:00',offset:'-05:00'},m.time_zone));
  expect(meetingIssue(c)).toBe('');
  m.repeat_rule='weekly'; expect(meetingIssue(c)).toContain('weekdays');
  m.repeat_weekdays=[7];expect(meetingIssue(c)).toBe('');
});
it('opens a created meeting using its occurrence reference',async()=>{
  draft.candidates[1].created_id='created-meeting';draft.candidates[1].selected=false;
  const p=await setup();
  button('Open meeting').click();
  expect(p.onOpenMeeting).toHaveBeenCalledWith({meetingId:'created-meeting',occurrenceKey:'o:0'});
  expect(p.onOpenTask).not.toHaveBeenCalled();
});
