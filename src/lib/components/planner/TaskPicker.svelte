<script lang="ts">
 import { onMount } from 'svelte';
 import { Search,Coffee,Utensils,Focus } from 'lucide-svelte';
 import type { BlockKind,PlannerTask } from '../../domain/planner';
 import { minuteLabel } from '../../domain/planner-rules';
 let {range,anchor,tasks,pending,error,recovery=false,onRetry,onPick,onCancel}:{range:{startMin:number;endMin:number};anchor:DOMRect;tasks:PlannerTask[];pending:boolean;error:string;recovery?:boolean;onRetry?:()=>Promise<void>;onPick:(choice:{kind:BlockKind;taskId?:string})=>Promise<void>;onCancel:()=>void}=$props();
 let dialog:HTMLDialogElement,search:HTMLInputElement;let query=$state(''),index=$state(0),left=$state(0),top=$state(0),busy=$state(false);
 const filtered=$derived(tasks.filter(t=>`${t.title} ${t.external_id??t.id} ${t.project_name}`.toLowerCase().includes(query.toLowerCase())));
 function position(){const r=dialog.getBoundingClientRect();left=Math.max(8,Math.min(anchor.right+8,innerWidth-r.width-8));top=Math.max(8,Math.min(anchor.top,innerHeight-r.height-8));}
 onMount(()=>{const before=document.activeElement as HTMLElement;dialog.showModal();position();search.focus();window.addEventListener('resize',position);return()=>{window.removeEventListener('resize',position);if(before?.isConnected)before.focus();};});
 async function pick(choice:{kind:BlockKind;taskId?:string}){if(pending||busy||recovery)return;busy=true;try{await onPick(choice);}catch{/* Parent retains the range and error. */}finally{busy=false;}}
 function keys(e:KeyboardEvent){if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();index=Math.max(0,Math.min(filtered.length-1,index+(e.key==='ArrowDown'?1:-1)));dialog.querySelector(`[data-index="${index}"]`)?.scrollIntoView({block:'nearest'});}else if(e.key==='Enter'&&filtered[index]){e.preventDefault();void pick({kind:'task',taskId:filtered[index].id});}}
</script>
<dialog class="picker" bind:this={dialog} style:left={`${left}px`} style:top={`${top}px`} aria-label="Choose a block" oncancel={e=>{e.preventDefault();if(!pending&&!busy)onCancel();}}>
 <h2>{minuteLabel(range.startMin)}–{minuteLabel(range.endMin)}</h2>
 <label class="search"><Search/><input bind:this={search} aria-label="Search tasks" placeholder="Search tasks or projects…" bind:value={query} oninput={()=>index=0} onkeydown={keys}/></label>
 <div class="tasks">{#each filtered as task,i (task.id)}<button data-index={i} class:active={i===index} disabled={pending||busy||recovery} onclick={()=>pick({kind:'task',taskId:task.id})}><strong>{task.title}</strong><span data-color={task.project_color}><i class="dot"></i>{task.project_name} · {task.external_id??'Local task'}</span></button>{/each}{#if !filtered.length}<p>No matching tasks.</p>{/if}</div>
 <div class="neutral"><button disabled={pending||busy||recovery} onclick={()=>pick({kind:'break'})}><Coffee/>Break</button><button disabled={pending||busy||recovery} onclick={()=>pick({kind:'lunch'})}><Utensils/>Lunch</button><button disabled={pending||busy||recovery} onclick={()=>pick({kind:'focus'})}><Focus/>Focus</button></div>
 {#if error}<p class="error" role="alert">{error}</p>{/if}{#if recovery}<button class="primary" disabled={pending||busy} onclick={()=>onRetry?.()}>Retry exact operation</button>{:else}<button class="outline" disabled={pending||busy} onclick={onCancel}>Cancel</button>{/if}
</dialog>
<style>
 .picker{position:fixed;margin:0;width:var(--picker-width);max-height:calc(100vh - 16px);padding:16px;border:1px solid var(--border-popover);border-radius:var(--radius-panel);box-shadow:var(--shadow-popover);background:var(--surface);color:var(--text)}.picker::backdrop{background:transparent}.picker h2{font-size:var(--text-base);margin-bottom:12px}.search{display:flex;gap:6px;align-items:center}.search input{min-width:0;width:100%}.tasks{max-height:260px;overflow:auto;margin:12px 0}.tasks button{display:block;width:100%;text-align:left;padding:8px}.tasks strong{display:block;font-weight:500}.tasks span{font-size:var(--text-meta);color:var(--text-muted);display:flex;align-items:center;gap:5px}.tasks .active{background:var(--surface-3)}.neutral{display:flex;gap:6px;border-top:1px solid var(--border);padding:12px 0}.neutral button{display:flex;align-items:center;gap:5px;padding:5px}
</style>
