<script lang="ts">
 import { onMount,untrack } from 'svelte';
 import { Minus,Plus } from 'lucide-svelte';
 import type { BlockDraft,PlannerTask } from '../../domain/planner';
 import { endpointChoices,minuteLabel,normalizeBlock } from '../../domain/planner-rules';
 import { errorMessage } from '../../domain/types';
 let {draft,editing=false,date,timeZone,tasks,pending,recovery=false,error='',onSave,onClose,onRetry}:{draft:BlockDraft;editing?:boolean;date:string;timeZone:string;tasks:PlannerTask[];pending:boolean;recovery?:boolean;error?:string;onSave:(draft:BlockDraft,date:string)=>Promise<void>;onClose:()=>void;onRetry:()=>Promise<void>}=$props();
 const initial=untrack(()=>draft);
 let selectedDate=$state(untrack(()=>date));
 let kind=$state(initial.kind),taskId=$state(initial.task_id??''),start=$state(minuteLabel(initial.start_min)),end=$state(minuteLabel(initial.end_min)),startOffset=$state(initial.start_offset??''),endOffset=$state(initial.end_offset??''),localError=$state(''),busy=$state(false);
 let dialog:HTMLDialogElement;
 function minutes(s:string){
  if(!/^\d{2}:\d{2}$/.test(s))return NaN;
  const hour=Number(s.slice(0,2)),minute=Number(s.slice(3));
  return hour<=24&&minute<60&&(hour<24||minute===0)?hour*60+minute:NaN;
 }
 function clearOffsets(){startOffset='';endOffset='';localError='';}
 function nudge(field:'start'|'end',delta:number){
  const a=minutes(start),b=minutes(end);
  if(!Number.isFinite(a)||!Number.isFinite(b))return;
  if(field==='start')start=minuteLabel(Math.max(0,Math.min(b-15,Math.round(a/15)*15+delta)));
  else end=minuteLabel(Math.min(1440,Math.max(a+15,Math.round(b/15)*15+delta)));
  clearOffsets();
 }
 function duration(value:number){end=minuteLabel(minutes(start)+value);clearOffsets();}
 const choices=(s:string)=>{try{return endpointChoices(selectedDate,minutes(s),timeZone);}catch{return [];}};
 const starts=$derived(choices(start)),ends=$derived(choices(end));
 onMount(()=>dialog.showModal());
 async function save(){if(pending||busy||recovery)return;busy=true;localError='';try{await onSave(normalizeBlock({kind,task_id:kind==='task'?taskId:null,start_min:minutes(start),end_min:minutes(end),...(startOffset?{start_offset:startOffset}:{}),...(endOffset?{end_offset:endOffset}:{})},selectedDate,timeZone),selectedDate);}catch(e){localError=errorMessage(e);}finally{busy=false;}}
</script>
<dialog class="m1-dialog editor" bind:this={dialog} aria-label="Schedule block" oncancel={e=>{e.preventDefault();onClose();}}>
 <h2>Schedule block</h2><p>{selectedDate} · {timeZone}</p>
 <form onsubmit={e=>{e.preventDefault();void save();}}>
 <fieldset disabled={pending||busy||recovery}>
 <label class="field">Block type<select bind:value={kind} disabled={editing}><option value="task">Task</option><option value="break">Break</option><option value="lunch">Lunch</option><option value="focus">Focus</option></select></label>
 {#if kind==='task'}<label class="field">Task<select bind:value={taskId} required disabled={editing}><option value="">Choose a task</option>{#each tasks as task}<option value={task.id}>{task.title} · {task.project_name}</option>{/each}</select></label>{/if}
 <div class="times">
  <div class="field"><label for="block-start">Start</label><div class="time-control"><button type="button" aria-label="Start 15 minutes earlier" title="15 minutes earlier" disabled={!Number.isFinite(minutes(start))||minutes(start)<=0} onclick={()=>nudge('start',-15)}><Minus aria-hidden="true"/></button><input id="block-start" aria-label="Start time" bind:value={start} placeholder="09:00" inputmode="numeric" pattern="[0-9]{2}:[0-9]{2}" required oninput={clearOffsets}/><button type="button" aria-label="Start 15 minutes later" title="15 minutes later" disabled={!Number.isFinite(minutes(start))||minutes(start)+15>=minutes(end)} onclick={()=>nudge('start',15)}><Plus aria-hidden="true"/></button></div></div>
  <div class="field"><label for="block-end">End</label><div class="time-control"><button type="button" aria-label="End 15 minutes earlier" title="15 minutes earlier" disabled={!Number.isFinite(minutes(end))||minutes(end)-15<=minutes(start)} onclick={()=>nudge('end',-15)}><Minus aria-hidden="true"/></button><input id="block-end" aria-label="End time" bind:value={end} placeholder="10:30" inputmode="numeric" pattern="[0-9]{2}:[0-9]{2}" required oninput={clearOffsets}/><button type="button" aria-label="End 15 minutes later" title="15 minutes later" disabled={!Number.isFinite(minutes(end))||minutes(end)>=1440} onclick={()=>nudge('end',15)}><Plus aria-hidden="true"/></button></div></div>
 </div>
 <label class="field">Date<input aria-label="Block date" type="date" bind:value={selectedDate} required onchange={clearOffsets}/></label>
 <div class="duration"><span>Duration</span>{#each [30,60,90,120] as value}<button class="outline" type="button" aria-pressed={minutes(end)-minutes(start)===value} disabled={!Number.isFinite(minutes(start))||minutes(start)+value>1440} onclick={()=>duration(value)}>{value<60?`${value}m`:`${value/60}h`}</button>{/each}</div>
 <p class="hint">Use ± to adjust by 15 minutes, or type a time. 24:00 ends the day.</p>
 {#if selectedDate!==date}<p class="hint">Saving {editing?'moves':'adds'} this block to {selectedDate} and opens that day.</p>{/if}
 {#if starts.length>1}<label class="field">Start occurs twice<select aria-label="Start UTC offset" bind:value={startOffset}><option value="">Choose UTC offset</option>{#each starts as c}<option value={c.offset}>{c.offset} · {c.instant}</option>{/each}</select></label>{/if}
 {#if ends.length>1}<label class="field">End occurs twice<select aria-label="End UTC offset" bind:value={endOffset}><option value="">Choose UTC offset</option>{#each ends as c}<option value={c.offset}>{c.offset} · {c.instant}</option>{/each}</select></label>{/if}
 </fieldset>
 {#if error||localError}<p class="error" role="alert">{error||localError}</p>{/if}
 <div class="dialog-foot">{#if recovery}<button type="button" class="primary" disabled={pending} onclick={()=>onRetry()}>Retry exact operation</button>{:else}<button class="primary" type="submit" disabled={pending||busy}>Save block</button>{/if}<button type="button" class="outline" disabled={pending||busy} onclick={onClose}>Cancel</button></div>
 </form>
</dialog>
<style>
 .editor{width:480px}.times{display:grid;grid-template-columns:1fr 1fr;gap:var(--space-3)}
 .time-control{display:flex;gap:var(--space-1);align-items:center}.time-control input{width:100%;min-width:0;padding:var(--space-2);text-align:center;font-variant-numeric:tabular-nums}
 .time-control button{display:grid;place-items:center;padding:var(--space-tight);border-color:var(--border-input);flex:none}
 .duration{display:flex;gap:var(--space-tight);align-items:center;margin-bottom:var(--space-3)}.duration>span{color:var(--text-muted);margin-right:var(--space-1)}.duration button{padding:var(--space-tight) var(--space-2)}.duration button[aria-pressed='true']{border-color:var(--accent);background:var(--selection-fill);color:var(--accent)}
 .hint{font-size:var(--text-meta)}fieldset{border:0;padding:0;margin:0}
</style>
