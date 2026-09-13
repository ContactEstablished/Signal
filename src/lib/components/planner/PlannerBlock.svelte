<script lang="ts">
 import { Coffee,Utensils,Focus,Video,GripVertical,Play,Pause,Square,Check,ChevronDown,FileText,Clock,RotateCcw,Trash2,Timer,X } from 'lucide-svelte';
 import type { PlannerItem } from '../../domain/planner';
 import type { TimerAction } from '../../domain/timers';
 import { minuteLabel,endpointChoices } from '../../domain/planner-rules';
 import { formatElapsedMs,timerElapsedMs } from '../../domain/timer-math';
 import { errorMessage } from '../../domain/types';
 let {item,compact=false,dense=false,nowUtc,timeZone,pending=false,onOpenTask,onDone,onRemove,onTimer,onJoin,onEditTime,onGesture}: {
  item:PlannerItem;compact?:boolean;dense?:boolean;nowUtc:string;timeZone:string;pending?:boolean;
  onOpenTask:(id:string)=>Promise<void>;onDone:(id:string)=>Promise<void>;onRemove:(id:string)=>Promise<void>;
  onTimer:(id:string,action:TimerAction)=>Promise<void>;onJoin:(url:string)=>Promise<void>;onEditTime:(id:string)=>void;
  onGesture:(event:PointerEvent,kind:'move'|'resize')=>void;
 }=$props();
 let details:HTMLDialogElement;let busy=$state(false),error=$state('');
 const own=$derived(!!item.block&&item.session?.block_id===item.block.id);
 const running=$derived(own&&item.session?.state==='running');
 const elapsed=$derived(item.session ? formatElapsedMs(timerElapsedMs(item.session,nowUtc)) : '');
 const over=$derived.by(()=>{
  if(!running||!item.block)return false;
  try {const choices=endpointChoices(item.block.date,item.end_min,timeZone);const end=choices.find(c=>c.offset===item.block?.end_offset)??(choices.length===1?choices[0]:null);return !!end&&Date.parse(nowUtc)>Date.parse(end.instant);}catch{return false;}
 });
 async function act(fn:()=>Promise<void>) {if(busy||pending)return;busy=true;error='';try{await fn();details?.close();}catch(e){error=errorMessage(e);}finally{busy=false;}}
</script>
{#snippet icon()}{#if item.kind==='break'}<Coffee />{:else if item.kind==='lunch'}<Utensils />{:else if item.kind==='focus'}<Focus />{:else if item.kind==='meeting'}<Video />{/if}{/snippet}
{#snippet actions()}
 <div class="actions block-actions">
 {#if item.meeting}
  {#if item.meeting.link_url}<button class="outline" disabled={busy||pending} onclick={()=>act(()=>onJoin(item.meeting!.link_url!))}><Video aria-hidden="true"/>Join</button>{/if}
  <span class="muted">Meeting editing arrives in M4.</span>
 {:else if item.block}
  {#if item.task_id}<button class="outline" aria-label="Open task" onclick={()=>act(()=>onOpenTask(item.task_id!))}><FileText aria-hidden="true"/>Open</button>{/if}
  <button class="outline" aria-label="Edit time" disabled={busy||pending} onclick={()=>{details?.close();onEditTime(item.id);}}><Clock aria-hidden="true"/>Edit</button>
  <button class="outline" aria-label={item.block.done?'Reopen block':'Complete block'} disabled={busy||pending} onclick={()=>act(()=>onDone(item.id))}>{#if item.block.done}<RotateCcw aria-hidden="true"/>Reopen{:else}<Check aria-hidden="true"/>Complete{/if}</button>
  {#if item.task_id}
   {#if own&&item.session}
    <button class="outline" disabled={busy||pending} onclick={()=>act(()=>onTimer(item.id,item.session!.state==='paused'?'resume':'pause'))}>{#if item.session.state==='paused'}<Play aria-hidden="true"/>Resume{:else}<Pause aria-hidden="true"/>Pause{/if}</button>
    <button class="outline" disabled={busy||pending} onclick={()=>act(()=>onTimer(item.id,'stop'))}><Square aria-hidden="true"/>Stop</button>
   {:else if item.session}<button class="outline" aria-label="View existing task timer" title="View existing task timer" onclick={()=>act(()=>onOpenTask(item.task_id!))}><Timer aria-hidden="true"/>Timer</button>
   {:else}<button class="outline" aria-label="Start timer" disabled={busy||pending||!!item.block.done} onclick={()=>act(()=>onTimer(item.id,'start'))}><Play aria-hidden="true"/>Start</button>{/if}
  {/if}
  <button class="outline danger" aria-label="Remove block" disabled={busy||pending} onclick={()=>act(()=>onRemove(item.id))}><Trash2 aria-hidden="true"/>Remove</button>
 {/if}
 </div>
{/snippet}
<div class="block" class:neutral={!item.project_color} class:meeting={item.kind==='meeting'} class:done={!!item.block?.done} class:running class:compact data-color={item.project_color??undefined}>
 <div class="top">
  {#if item.block}<button class="grip" aria-label={`Move ${item.title}`} title="Drag to move; use Edit time for keyboard" disabled={pending} onpointerdown={e=>onGesture(e,'move')}><GripVertical /></button>{/if}
  {#if item.block&&!compact}<button class="check" aria-label={item.block.done?'Reopen block':'Complete block'} disabled={busy||pending} onclick={()=>act(()=>onDone(item.id))}>{#if item.block.done}<Check />{:else}<span class="box"></span>{/if}</button>{/if}
  {@render icon()}
  {#if compact}
   {#if own&&item.session}<Timer aria-label={item.session.state==='running'?'Timer running':'Timer paused'}/>{/if}
   <button class="title" onclick={()=>details.showModal()} aria-label={`Details for ${item.title}`}>{item.title}</button>
  {:else}<span class="kind">{item.kind}</span>{#if dense&&own&&item.session}<Timer aria-label={item.session.state==='running'?'Timer running':'Timer paused'}/>{/if}<span class="range">{minuteLabel(item.start_min)}–{minuteLabel(item.end_min)}</span>{/if}
  <button class="disclose" aria-label={`Show actions for ${item.title}`} onclick={()=>details.showModal()}><ChevronDown /></button>
 </div>
 {#if !compact}
  <button class="title full-title" onclick={()=>details.showModal()} aria-label={`Details for ${item.title}`}>{item.title}</button>
  {#if !dense}{#if own&&item.session}<div class="timer" class:over><span>{item.session.state==='paused'?'Paused':'Running'} · {elapsed}{over?' · Ran over':''}</span><button aria-label={item.session.state==='paused'?'Resume timer':'Pause timer'} disabled={busy||pending} onclick={()=>act(()=>onTimer(item.id,item.session!.state==='paused'?'resume':'pause'))}>{#if item.session.state==='paused'}<Play />{:else}<Pause />{/if}</button><button aria-label="Stop timer" disabled={busy||pending} onclick={()=>act(()=>onTimer(item.id,'stop'))}><Square /></button></div>
  {:else}<div class="meta">{#if item.task}{item.task.external_id??'Local task'} · {item.task.hours_worked.toFixed(1)}h logged{item.task.estimate_h===null?'':` / ${item.task.estimate_h}h`}{:else}{item.project_name??'Personal time'}{/if}{#if item.block?.carried_from_block_id} · Carried over{/if}</div>{/if}{/if}
  {#if !dense&&item.meeting?.link_url}<button class="meeting-join" disabled={busy||pending} onclick={()=>act(()=>onJoin(item.meeting!.link_url!))}>Join <Video aria-hidden="true"/></button>{/if}
 {/if}
 {#if item.block}<button class="resize" aria-label={`Resize ${item.title}`} title="Drag to resize; use Edit time for keyboard" disabled={pending} onpointerdown={e=>onGesture(e,'resize')}></button>{/if}
</div>
{#if error}<p class="error" role="alert">{error}</p>{/if}
<dialog class="m1-dialog block-detail" bind:this={details} aria-label={`${item.title} details`}>
 <div class="block-detail-heading"><h2>{item.title}</h2><button class="close-details" aria-label="Close block details" title="Close" onclick={()=>details.close()}><X aria-hidden="true"/></button></div>
 <p>{minuteLabel(item.start_min)}–{minuteLabel(item.end_min)} · {item.project_name??'Personal time'}</p>
 {#if item.session}<p class="timer">{own?'This block':'Task timer elsewhere'} · {item.session.state} · {elapsed}{over?' · Over planned time':''}</p>{/if}
 {@render actions()}
 {#if error}<p class="error" role="alert">{error}</p>{/if}
</dialog>
<style>
 .block{height:100%;position:relative;border:1px solid var(--project-task-border);border-left:3px solid var(--project-color);border-radius:var(--radius-input);background:var(--project-task-fill);padding:var(--block-padding);overflow:hidden;}
 .neutral{border:1px dashed var(--border-dashed);background:var(--surface);color:var(--text-muted)}.meeting{border-style:dashed;background:var(--project-meeting-fill)}.done{opacity:var(--opacity-done-block)}.done .title{text-decoration:line-through}.running{border-color:var(--project-color);box-shadow:var(--project-ring);background:var(--project-active-fill)}
 .compact{padding:2px 4px}.top{display:flex;gap:4px;align-items:center;min-width:0}.title{min-width:0;flex:1;text-align:left;font-weight:500;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;padding:0}.kind{font:600 var(--text-chip) var(--font-heading);letter-spacing:.04em;text-transform:uppercase;color:var(--project-color);flex:1}.neutral .kind{color:var(--text-muted)}.full-title{display:block;width:100%;margin-top:4px}.range{font-size:var(--text-meta);color:var(--text-muted);white-space:nowrap}.grip,.check,.disclose{display:grid;place-items:center;padding:0;flex:none}.grip{touch-action:none;cursor:grab;color:var(--text-muted)}.box{width:14px;height:14px;border:1px solid var(--border-dashed);border-radius:3px}.meeting-join{display:flex;align-items:center;gap:4px;padding:0;color:var(--project-color);font-size:var(--text-meta)}.meta{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;font-size:var(--text-meta);color:var(--text-muted);margin-top:3px}.timer{display:flex;align-items:center;gap:4px;color:var(--lime);font-size:var(--text-meta)}.timer span{flex:1}.timer button{padding:0}.over{color:var(--warn)}.resize{position:absolute;bottom:0;height:7px;left:15%;width:70%;padding:0;cursor:ns-resize;touch-action:none}.resize:hover{background:var(--project-border)}.error{position:relative;z-index:3;background:var(--surface)}
 .block-detail {
  width: max-content;
  min-width: min(var(--picker-width), calc(100vw - var(--space-8)));
  max-width: min(var(--modal-task-new), calc(100vw - var(--space-8)));
 }
 .block-detail-heading {
  display: flex;
  align-items: start;
  gap: var(--space-4);
  margin-bottom: var(--space-2);
 }
 .block-detail-heading h2 { flex: 1; min-width: 0; margin: 0; overflow-wrap: anywhere; }
 .close-details { display: grid; place-items: center; padding: var(--space-1); flex: none; color: var(--text-muted); }
 .block-actions { gap: var(--space-tight); }
 .block-actions button {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-tight) var(--space-2);
  font: var(--weight-medium) var(--text-label) var(--font-body);
  white-space: nowrap;
 }
</style>
