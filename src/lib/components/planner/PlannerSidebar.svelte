<script lang="ts">
 import TaskDragGhost from '../TaskDragGhost.svelte';
 import { onDestroy,untrack } from 'svelte';
 import { Coffee,Utensils,Focus,Plus,Check,GripVertical } from 'lucide-svelte';
 import { automaticFloor,dueSoon,unscheduled,leftovers,plannerTotals,plannerItems } from '../../domain/planner-rules';
 import { dateAt } from '../../domain/clock';
 import type { PlannerSnapshot,BatchMode,PlannerTask } from '../../domain/planner';
 import type { PlannerTaskDrag } from '../../domain/planner-drag';
 let {snapshot,scope,nowUtc,pending,onPlan,onAddTask,onOpenTask,onTaskDrag,onTaskDrop}:{snapshot:PlannerSnapshot;scope:string;nowUtc:string;pending:boolean;onPlan:(mode:BatchMode,ids?:string[],kind?:'break'|'lunch'|'focus')=>void;onAddTask:(id:string)=>void;onOpenTask:(id:string)=>Promise<void>;onTaskDrag:(drag:PlannerTaskDrag|null)=>void;onTaskDrop:(id:string,x:number,y:number)=>Promise<void>|undefined}=$props();
 const scheduled=$derived(new Set(snapshot.blocks.filter(b=>b.date===snapshot.date).map(b=>b.task_id)));
 const candidates=$derived(unscheduled(snapshot,scope)),due=$derived(dueSoon(snapshot,scope).filter(t=>!scheduled.has(t.id))),left=$derived(leftovers(snapshot,scope)),totals=$derived(plannerTotals(snapshot,scope));
 const completed=$derived(plannerItems(snapshot,scope).filter(item=>item.block?.done).sort((a,b)=>a.start_min-b.start_min||a.id.localeCompare(b.id)));
 const empty=$derived(plannerItems(snapshot,scope).length===0);
 const past=$derived(automaticFloor(snapshot.date,nowUtc,snapshot.time_zone)===null);
 let picked=$state<string[]>([]);
 let drag=$state<(PlannerTaskDrag & { pointerId:number; startX:number; startY:number; active:boolean })|null>(null);
 let capture:{element:Element;pointerId:number}|null=null;
 let suppressClick=false;
 function cancelDrag(){
  drag=null;
  onTaskDrag(null);
  const old=capture;capture=null;
  if(old?.element.hasPointerCapture?.(old.pointerId))old.element.releasePointerCapture(old.pointerId);
 }
 function beginDrag(e:PointerEvent,task:PlannerTask){
  suppressClick=false;
  if(pending||drag||e.button!==0||(e.target as Element).closest('[data-schedule-action]'))return;
  const element=e.target as Element;
  element.setPointerCapture(e.pointerId);
  capture={element,pointerId:e.pointerId};
  drag={id:task.id,title:task.title,project_color:task.project_color,pointerId:e.pointerId,startX:e.clientX,startY:e.clientY,x:e.clientX,y:e.clientY,active:false};
 }
 function moveDrag(e:PointerEvent){
  if(!drag||drag.pointerId!==e.pointerId)return;
  if(pending){cancelDrag();return;}
  drag.x=e.clientX;drag.y=e.clientY;
  if(!drag.active&&Math.hypot(drag.x-drag.startX,drag.y-drag.startY)>5){drag.active=true;suppressClick=true;}
  if(drag.active){e.preventDefault();onTaskDrag({id:drag.id,title:drag.title,project_color:drag.project_color,x:drag.x,y:drag.y});}
 }
 function endDrag(e:PointerEvent){
  if(!drag||drag.pointerId!==e.pointerId)return;
  const ended=drag;cancelDrag();
  if(ended.active&&!pending){e.preventDefault();void onTaskDrop(ended.id,e.clientX,e.clientY);}
 }
 $effect(()=>{snapshot.date;scope;untrack(cancelDrag);});
 $effect(()=>{if(pending)cancelDrag();});
 onDestroy(cancelDrag);
</script>
<svelte:window onpointermove={moveDrag} onpointerup={endDrag}
 onpointercancel={e=>{if(drag?.pointerId===e.pointerId)cancelDrag();}}
 onblur={cancelDrag} onkeydown={e=>{if(e.key==='Escape'&&drag){e.preventDefault();cancelDrag();}}}/>
{#if drag?.active}<TaskDragGhost title={drag.title} hint="1 hour · Drop on your calendar"
 x={drag.x} y={drag.y} color={drag.project_color}/>{/if}
<aside aria-label="Planner sidebar">
 {#if empty}<section class="empty-day" aria-label="Empty day planning"><h2>Nothing planned yet</h2><p>{due.length} {due.length===1?'task is':'tasks are'} due this week or overdue.</p><div class="actions"><button class="outline" disabled={pending||past||!due.length} onclick={()=>onPlan('due')}>Plan from due dates</button><button class="outline" disabled={pending||past||!snapshot.copy_sources.length} onclick={()=>onPlan('copy')}>Copy last Monday</button></div></section>{/if}
 <section><h2>Quick add</h2><div class="quick"><button disabled={pending||past} onclick={()=>onPlan('quick',undefined,'break')}><Coffee/>Break <span>30m</span></button><button disabled={pending||past} onclick={()=>onPlan('quick',undefined,'lunch')}><Utensils/>Lunch <span>1h</span></button><button disabled={pending||past} onclick={()=>onPlan('quick',undefined,'focus')}><Focus/>Focus <span>1h</span></button></div>{#if past}<p class="hint">Automatic planning is unavailable on past dates. Use Add block to edit manually.</p>{/if}</section>
 {#if left.length}<section><h2>Left over from yesterday <span>{left.length}</span></h2>{#each left as block}{@const task=snapshot.tasks.find(t=>t.id===block.task_id)!}<label class="left"><input type="checkbox" value={block.id} bind:group={picked} disabled={pending}/><span>{task.title}<small>{block.end_min-block.start_min} min planned</small></span></label>{/each}<div class="actions"><button class="outline" disabled={pending||past} onclick={()=>onPlan('carry')}>Carry all</button><button class="outline" disabled={pending||past||!picked.length} onclick={()=>onPlan('carry',picked)}>Pick selected</button></div></section>{/if}
 <section><h2>Unscheduled <span>{candidates.length}</span></h2><p class="hint">Drag to schedule 1 hour, or use + to choose a time.</p>{#each candidates as task}<div class="task" draggable={false} onpointerdown={e=>beginDrag(e,task)} onlostpointercapture={e=>{if(drag?.pointerId===e.pointerId)cancelDrag();}} class:drag-source={drag?.active&&drag.id===task.id} role="group" aria-label={`Schedule ${task.title}`} data-color={task.project_color}><button class="task-title" onclick={e=>{if(!suppressClick||e.detail===0)void onOpenTask(task.id);}}>{task.title}<small><i class="dot"></i>{task.project_name}</small></button><button data-schedule-action aria-label={`Schedule ${task.title}`} disabled={pending} onclick={()=>onAddTask(task.id)}><Plus/></button></div>{/each}{#if !candidates.length}<p class="hint">All unfinished tasks have a block on this date.</p>{/if}</section>
 <section aria-label="Due soon"><h2>Due soon <span>{due.length}</span></h2><p class="hint">Drag to schedule 1 hour, or use + to choose a time.</p>{#each due as task}<div class="due-row" data-color={task.project_color} draggable={false} onpointerdown={e=>beginDrag(e,task)} onlostpointercapture={e=>{if(drag?.pointerId===e.pointerId)cancelDrag();}} class:drag-source={drag?.active&&drag.id===task.id} role="group" aria-label={`Schedule ${task.title}`}><GripVertical class="due-grip" aria-hidden="true"/><i class="dot"></i><button class="due" onclick={e=>{if(!suppressClick||e.detail===0)void onOpenTask(task.id);}}><span>{task.title}<small class="remaining">{task.estimate_h===null?'No estimate':`${Math.max(0,task.estimate_h-task.hours_worked).toFixed(1)}h remaining`}</small></span><small>{task.due_at?new Intl.DateTimeFormat('en-US',{timeZone:snapshot.time_zone,month:'short',day:'numeric'}).format(new Date(task.due_at)):''}</small></button><button data-schedule-action class="schedule" aria-label={`Schedule ${task.title}`} title="Choose a time" disabled={pending} onclick={()=>onAddTask(task.id)}><Plus aria-hidden="true"/></button></div>{/each}{#if !due.length}<p class="hint">No unscheduled tasks due soon for this day.</p>{/if}{#if !empty}<button class="outline" disabled={pending||past||!due.length} onclick={()=>onPlan('due')}>Plan from due dates</button>{/if}</section>
 {#if !empty}<section><button class="outline" disabled={pending||past||!snapshot.copy_sources.length} onclick={()=>onPlan('copy')}>Copy last Monday</button><p class="hint">Review task and personal blocks before applying.</p></section>{/if}
 {#if completed.length}<section aria-label="Completed blocks"><h2>Done {snapshot.date===dateAt(nowUtc,snapshot.time_zone)?'today':'on this day'} <span>{completed.length}</span></h2><div class="completed-list">{#each completed as item}<div class="completed-row" data-color={item.project_color??undefined}><Check aria-hidden="true"/>{#if item.task_id}<button onclick={()=>onOpenTask(item.task_id!)}>{item.title}</button>{:else}<span>{item.title}</span>{/if}<small>{(item.end_min-item.start_min)/60}h</small></div>{/each}</div></section>{/if}
 <footer><div><strong>{(totals.planned/60).toFixed(1)}h</strong><span>Planned</span></div><div><strong>{(totals.logged/60).toFixed(1)}h</strong><span>Logged</span></div><div><strong>{totals.done}/{totals.total}</strong><span>Blocks</span></div></footer>
</aside>
<style>
 .task,.due-row{touch-action:none;user-select:none}
 .drag-source{opacity:var(--opacity-drag-source)}
aside{min-height:0;overflow:auto;scrollbar-width:thin;display:flex;flex-direction:column;gap:16px}section{padding:0 0 16px;border-bottom:1px solid var(--border)}h2{font-size:var(--text-label);display:flex;justify-content:space-between;color:var(--text-muted)}h2>span{color:var(--text-faint)}.quick{display:flex;flex-direction:column;gap:6px}.quick button{display:flex;align-items:center;gap:8px;padding:8px 10px;background:var(--surface);border:1px dashed var(--border-dashed);text-align:left}.quick span{margin-left:auto;color:var(--text-faint)}.hint{font-size:var(--text-meta);margin:8px 0;color:var(--text-faint)}.task{display:flex;align-items:center;border:1px solid var(--border);border-radius:var(--radius-input);padding:8px;margin-top:6px;background:var(--surface);cursor:grab}.task-title{min-width:0;flex:1;text-align:left;padding:0;font-weight:500}.task small{display:flex;align-items:center;gap:5px;color:var(--text-muted);font-weight:400;margin-top:3px}.task button:last-child{padding:4px}.due-row{display:flex;align-items:center;gap:6px;cursor:grab}.due-row .schedule{padding:4px;flex:none}.due{display:flex;min-width:0;flex:1;gap:10px;text-align:left;padding:8px 0}.due span{flex:1;min-width:0}.due .remaining{display:block;color:var(--text-muted);font-size:var(--text-meta);margin-top:3px}.due-row :global(.due-grip){color:var(--text-faint);width:12px;flex:none}.due-row .dot{flex:none;width:8px;height:8px}.due small{color:var(--warn);white-space:nowrap}.left{display:flex;align-items:flex-start;gap:8px;margin:10px 0}.left small{display:block;color:var(--text-muted)}footer{display:flex;justify-content:space-between;margin-top:auto;padding:12px 0}footer div{display:flex;flex-direction:column}footer strong{font-family:var(--font-heading);font-weight:600}footer span{font-size:var(--text-meta);color:var(--text-faint)}.empty-day{border:1px dashed var(--selection-border);border-radius:var(--radius-panel);padding:14px;background:var(--selection-fill)}.empty-day p{color:var(--text-muted);font-size:var(--text-meta)}.completed-list{border:1px solid var(--border);border-radius:var(--radius-card);background:var(--surface);padding:12px 14px}.completed-row{display:flex;gap:8px;align-items:center;color:var(--text-muted);margin:8px 0}.completed-row :global(svg){color:var(--project-color,var(--lime));flex:none}.completed-row button,.completed-row>span{flex:1;min-width:0;text-align:left;text-decoration:line-through;padding:0;color:var(--text-muted)}.completed-row small{white-space:nowrap}</style>
