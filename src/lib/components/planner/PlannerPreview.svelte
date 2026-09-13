<script lang="ts">
 import { onMount,untrack } from 'svelte';
 import type { PlannerSnapshot,PlanPreview,BlockDraft } from '../../domain/planner';
 import { minuteLabel,normalizeBlock,endpointChoices } from '../../domain/planner-rules';
 import { errorMessage } from '../../domain/types';
 let {preview,snapshot,pending,recovery=false,error='',onApply,onClose,onRetry}:{preview:PlanPreview;snapshot:PlannerSnapshot;pending:boolean;recovery?:boolean;error?:string;onApply:(blocks:BlockDraft[])=>Promise<void>;onClose:()=>void;onRetry:()=>Promise<void>}=$props();
 let blocks=$state(untrack(()=>preview.blocks.map(b=>({...b})))),localError=$state(''),busy=$state(false);let dialog:HTMLDialogElement;
 const title=(b:BlockDraft)=>snapshot.tasks.find(t=>t.id===b.task_id)?.title??b.kind;
 function choices(min:number){try{return endpointChoices(snapshot.date,min,snapshot.time_zone);}catch{return [];}}
 onMount(()=>dialog.showModal());
 async function apply(){if(pending||busy||recovery)return;busy=true;localError='';try{await onApply(blocks.map(b=>normalizeBlock(b,snapshot.date,snapshot.time_zone)));}catch(e){localError=errorMessage(e);}finally{busy=false;}}
</script>
<dialog class="m1-dialog preview" bind:this={dialog} aria-label="Review plan" oncancel={e=>{e.preventDefault();onClose();}}>
 <h2>Review {preview.mode==='copy'?'Monday copy':preview.mode==='due'?'due-date plan':preview.mode==='carry'?'carry-over':'Quick add'}</h2>
 <p>{snapshot.date} · {snapshot.time_zone} · {blocks.length} proposed blocks</p>
 <div class="rows">{#each blocks as b,i}<section><strong>{title(b)}</strong><span>{minuteLabel(b.start_min)}–{minuteLabel(b.end_min)}</span>
 {#if preview.conflicts.includes(b.source_id??'')}<p class="warning">Overlaps existing or proposed time. This overlap will be preserved.</p>{/if}
 {#each ['start','end'] as endpoint}{@const min=endpoint==='start'?b.start_min:b.end_min}{@const options=choices(min)}
  {#if !options.length}<p class="error">{endpoint} is not a valid local time. Cancel and choose another date or edit manually.</p>{:else if options.length>1}<label>{endpoint} occurs twice<select aria-label={`${endpoint} offset for ${title(b)}`} disabled={pending||recovery} value={(endpoint==='start'?b.start_offset:b.end_offset)??''} onchange={e=>{if(endpoint==='start')blocks[i].start_offset=e.currentTarget.value;else blocks[i].end_offset=e.currentTarget.value;}}><option value="">Choose UTC offset</option>{#each options as c}<option value={c.offset}>{c.offset}</option>{/each}</select></label>{/if}
 {/each}</section>{/each}
 {#each preview.skipped as skip}<p>{snapshot.tasks.find(t=>t.id===skip.id)?.title??skip.id}: {skip.reason}</p>{/each}</div>
 {#if error||localError}<p class="error" role="alert">{error||localError}</p>{/if}
 <div class="dialog-foot">{#if recovery}<button class="primary" disabled={pending} onclick={()=>onRetry()}>Retry exact operation</button>{:else}<button class="primary" disabled={pending||busy||!blocks.length} onclick={apply}>Apply plan</button>{/if}<button class="outline" disabled={pending||busy} onclick={onClose}>Cancel</button></div>
</dialog>
<style>.preview{width:600px}.rows{max-height:55vh;overflow:auto}.rows section{border-bottom:1px solid var(--border);padding:10px 0}.rows span{float:right;color:var(--text-muted)}.rows label{display:flex;gap:8px;align-items:center;margin-top:8px}.rows p{font-size:var(--text-label);margin:8px 0}</style>
