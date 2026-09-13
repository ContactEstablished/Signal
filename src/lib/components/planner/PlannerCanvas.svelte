<script lang="ts">
 import { onMount,untrack } from 'svelte';
 import PlannerBlock from './PlannerBlock.svelte';
 import { packLanes } from '../../domain/lane-packing';
 import { localMinute,minuteLabel } from '../../domain/planner-rules';
 import { dateAt } from '../../domain/clock';
 import { errorMessage } from '../../domain/types';
 import type { PlannerItem } from '../../domain/planner';
 import type { TimerAction } from '../../domain/timers';
 let {items,date,nowUtc,timeZone,pending=false,selection,onSelect,onCancelSelection,onMove,onDropTask,onOpenTask,onDone,onRemove,onTimer,onJoin,onEditTime}:{
 items:PlannerItem[];date:string;nowUtc:string;timeZone:string;pending?:boolean;selection:{startMin:number;endMin:number}|null;
 onSelect:(start:number,end:number,anchor:DOMRect)=>void;onCancelSelection:()=>void;onMove:(id:string,start:number,end:number)=>Promise<void>;onDropTask:(id:string,start:number)=>Promise<void>;
 onOpenTask:(id:string)=>Promise<void>;onDone:(id:string)=>Promise<void>;onRemove:(id:string)=>Promise<void>;onTimer:(id:string,action:TimerAction)=>Promise<void>;onJoin:(url:string)=>Promise<void>;onEditTime:(id:string)=>void;
 }=$props();
 type Gesture={kind:'select'|'move'|'resize';pointer:number;origin:number;start:number;end:number;id?:string;preview:{startMin:number;endMin:number}};
 let wrapper:HTMLDivElement;let gesture=$state<Gesture|null>(null),busy=$state(false),error=$state(''),scrollTop=$state(420),visibleHeight=$state(600);
 let frame=0,pointerY=0;
 const packed=$derived(packLanes(items));
 const today=$derived(dateAt(nowUtc,timeZone)), nowMin=$derived(localMinute(nowUtc,timeZone));
 const shade=$derived(date<today?1440:date===today?nowMin:0);
 const preview=$derived(gesture?.preview??selection);
 const clamp=(n:number,min:number,max:number)=>Math.min(max,Math.max(min,n));
 const minute=(y:number)=>Math.round((y-wrapper.getBoundingClientRect().top+wrapper.scrollTop)/15)*15;
 function cancel(){const old=gesture;gesture=null;cancelAnimationFrame(frame);frame=0;if(old&&wrapper?.hasPointerCapture?.(old.pointer))wrapper.releasePointerCapture(old.pointer);}
 $effect(()=>{date;untrack(()=>{cancel();if(wrapper){wrapper.scrollTop=420;scrollTop=420;}});});
 onMount(()=>{wrapper.scrollTop=420;const observer=new ResizeObserver(()=>visibleHeight=wrapper.clientHeight);observer.observe(wrapper);return()=>{observer.disconnect();cancel();};});
 function update(y:number){if(!gesture)return;const m=minute(y);let start:number,end:number;
  if(gesture.kind==='select'){start=clamp(Math.min(gesture.origin,m),0,1425);end=clamp(Math.max(gesture.origin,m),start+15,1440);}
  else if(gesture.kind==='move'){const duration=gesture.end-gesture.start;start=clamp(gesture.start+m-gesture.origin,0,1440-duration);end=start+duration;}
  else{start=gesture.start;end=clamp(m,start+15,1440);}
  gesture.preview={startMin:start,endMin:end};
 }
 function autoscroll(){if(!gesture)return;const r=wrapper.getBoundingClientRect();const speed=pointerY<r.top+36?-10:pointerY>r.bottom-36?10:0;if(speed){wrapper.scrollTop+=speed;scrollTop=wrapper.scrollTop;update(pointerY);}frame=requestAnimationFrame(autoscroll);}
 function begin(e:PointerEvent,kind:Gesture['kind'],item?:PlannerItem){if(e.button!==0||pending||busy||selection||gesture)return;e.preventDefault();e.stopPropagation();const origin=clamp(minute(e.clientY),0,1425);gesture={kind,pointer:e.pointerId,origin,start:item?.start_min??origin,end:item?.end_min??origin+15,id:item?.id,preview:{startMin:item?.start_min??origin,endMin:item?.end_min??origin+15}};pointerY=e.clientY;wrapper.setPointerCapture(e.pointerId);frame=requestAnimationFrame(autoscroll);}
 function down(e:PointerEvent){if((e.target as HTMLElement).closest('[data-planner-item],button')||e.clientX<wrapper.getBoundingClientRect().left+64)return;begin(e,'select');}
 async function up(e:PointerEvent){if(!gesture||e.pointerId!==gesture.pointer)return;update(e.clientY);const g=gesture;cancel();if(g.kind==='select'){const r=wrapper.getBoundingClientRect();onSelect(g.preview.startMin,g.preview.endMin,new DOMRect(r.right-20,r.top+g.preview.startMin-wrapper.scrollTop,0,g.preview.endMin-g.preview.startMin));}else if(g.id&&(g.start!==g.preview.startMin||g.end!==g.preview.endMin)){busy=true;error='';try{await onMove(g.id,g.preview.startMin,g.preview.endMin);}catch(e){error=errorMessage(e);}finally{busy=false;}}}
 async function drop(e:DragEvent){const id=e.dataTransfer?.getData('application/x-signal-task-id');if(!id||pending||busy)return;e.preventDefault();busy=true;error='';try{await onDropTask(id,clamp(minute(e.clientY),0,1425));}catch(e){error=errorMessage(e);}finally{busy=false;}}
</script>
<svelte:window onkeydown={e=>{if(e.key==='Escape'){cancel();onCancelSelection();}}}/>
<div class="canvas-shell">
 {#if error}<p class="error" role="alert">{error}</p>{/if}
 <div class="fade-label">00:00–{minuteLabel(Math.floor(scrollTop/60)*60)} · {items.some(i=>i.start_min<scrollTop)?'Earlier blocks above':'Nothing planned earlier'}</div>
 <div class="canvas-scroll" bind:this={wrapper} role="application" aria-label="Daily schedule. Use Add block or Edit time for keyboard scheduling." onscroll={()=>scrollTop=wrapper.scrollTop} onpointerdown={down} onpointermove={e=>{if(gesture?.pointer===e.pointerId){pointerY=e.clientY;update(pointerY);}}} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel} ondragover={e=>{if(e.dataTransfer?.types.includes('application/x-signal-task-id'))e.preventDefault();}} ondrop={drop}>
  <div class="grid">
  {#if !items.length}<div class="empty-hint" style:top={`${Math.min(1320,scrollTop+60)}px`}>Drag across any hours to plan a block<span>Or use Add block to choose a task and time.</span></div>{/if}
   <div class="past" style:height={`${shade}px`}></div>
   {#each Array.from({length:25},(_,i)=>i) as hour}<div class="hour" style:top={`${hour*60}px`}><span>{String(hour).padStart(2,'0')}:00</span></div>{/each}
   {#each packed as p (p.item.id)}
    <div data-planner-item={p.item.id} class="position" class:source={gesture?.id===p.item.id} style:top={`${p.top}px`} style:height={`${p.height}px`} style:left={`calc(64px + (100% - 64px + 4px) * ${p.lane/p.lanes})`} style:width={`calc((100% - 64px + 4px) / ${p.lanes} - 4px)`}>
     <PlannerBlock item={p.item} compact={p.lanes>=4||p.height<40} dense={p.height<80} {nowUtc} {timeZone} pending={pending||busy} {onOpenTask} {onDone} {onRemove} {onTimer} {onJoin} {onEditTime} onGesture={(e,kind)=>begin(e,kind,p.item)}/>
    </div>
   {/each}
   {#if preview}<div class="selection" style:top={`${preview.startMin}px`} style:height={`${preview.endMin-preview.startMin}px`}>{minuteLabel(preview.startMin)}–{minuteLabel(preview.endMin)} · {gesture?.kind==='select'||selection?'Select a task or personal time':'Release to save'}</div>{/if}
   {#if date===today}<div class="now" style:top={`${nowMin}px`}><span></span><time>{minuteLabel(Math.floor(nowMin))}</time></div>{/if}
  </div>
 </div>
 <div class="fade-label">{items.some(i=>i.end_min>scrollTop+visibleHeight)?'Later blocks below':'Nothing planned later'} · 24:00</div>
</div>
<style>
 .canvas-shell{min-height:0;display:flex;flex-direction:column;border:1px solid var(--border-section);border-radius:var(--radius-panel);background:var(--bg-raised);overflow:hidden}.canvas-scroll{overflow-y:auto;overflow-x:hidden;min-height:0;flex:1;scrollbar-width:thin;touch-action:none;overscroll-behavior:contain}.grid{height:1441px;position:relative;margin-right:12px}.hour{position:absolute;left:0;right:0;border-top:1px solid var(--border);pointer-events:none}.hour span{position:absolute;top:-9px;width:54px;text-align:right;color:var(--text-faint);background:var(--bg-raised);font:var(--weight-hour) var(--text-meta) var(--font-heading)}.hour:after{content:'';position:absolute;top:29px;left:64px;right:0;border-top:1px dotted var(--border)}.past{position:absolute;left:64px;right:0;background:var(--past-shade);pointer-events:none}.position{position:absolute;min-width:0;z-index:1}.source{opacity:var(--opacity-drag-source)}.selection{position:absolute;left:64px;right:0;z-index:3;border:1.5px dashed var(--selection-border);background:var(--selection-fill);color:var(--accent);padding:3px 10px;pointer-events:none}.now{position:absolute;left:64px;right:0;height:1px;background:var(--warn);z-index:4;pointer-events:none}.now span{position:absolute;width:var(--now-dot);height:var(--now-dot);left:-3px;top:-3px;border-radius:50%;background:var(--warn)}.now time{position:absolute;right:calc(100% + 8px);top:-8px;color:var(--warn);background:var(--bg-raised);font:var(--weight-hour) var(--text-meta) var(--font-heading)}.empty-hint{position:absolute;left:80px;right:20px;border:1.5px dashed var(--selection-border);border-radius:var(--radius-input);padding:18px;color:var(--accent);background:var(--selection-fill);pointer-events:none}.empty-hint span{display:block;color:var(--text-muted);font-size:var(--text-meta);margin-top:6px}.fade-label{color:var(--text-faint);font-size:var(--text-meta);text-align:center;padding:5px;pointer-events:none}.error{padding:6px;margin:0}
</style>
