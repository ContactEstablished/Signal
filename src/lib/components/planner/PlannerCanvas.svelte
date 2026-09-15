<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import PlannerBlock from './PlannerBlock.svelte';
  import { packLanes } from '../../domain/lane-packing';
  import { localMinute, minuteLabel } from '../../domain/planner-rules';
  import { dateAt } from '../../domain/clock';
  import { errorMessage } from '../../domain/types';
  import type { PlannerItem } from '../../domain/planner';
  import type { TimerAction } from '../../domain/timers';
  import { SIDEBAR_BLOCK_MINUTES, type PlannerTaskDrag } from '../../domain/planner-drag';
  let {
    items,
    date,
    nowUtc,
    timeZone,
    pending = false,
    selection,
    onSelect,
    onCancelSelection,
    onMove,
    onDropTask,
    onOpenTask,
    onDone,
    onRemove,
    onTimer,
    onOpenMeeting,
    onJoin,
    onEditTime,
  }: {
    items: PlannerItem[];
    date: string;
    nowUtc: string;
    timeZone: string;
    pending?: boolean;
    selection: { startMin: number; endMin: number } | null;
    onSelect: (start: number, end: number, anchor: DOMRect) => void;
    onCancelSelection: () => void;
    onMove: (id: string, start: number, end: number) => Promise<void>;
    onDropTask: (id: string, start: number) => Promise<void>;
    onOpenTask: (id: string) => Promise<void>;
    onDone: (id: string) => Promise<void>;
    onRemove: (id: string) => Promise<void>;
    onTimer: (id: string, action: TimerAction) => Promise<void>;
    onOpenMeeting?: (
      ref: import('../../domain/agenda').MeetingRef,
    ) => Promise<void>;
    onJoin: (url: string) => Promise<void>;
    onEditTime: (id: string) => void;
  } = $props();
  type Gesture = {
    kind: 'select' | 'move' | 'resize';
    pointer: number;
    origin: number;
    start: number;
    end: number;
    id?: string;
    preview: { startMin: number; endMin: number };
  };
  let wrapper: HTMLDivElement;
  let gesture = $state<Gesture | null>(null),
    busy = $state(false),
    error = $state(''),
    scrollTop = $state(420),
    visibleHeight = $state(600);
  let frame = 0,
    pointerY = 0;
  let sidebarDrag = $state<PlannerTaskDrag | null>(null);
  let taskFrame = 0;
  const packed = $derived(packLanes(items));
  const today = $derived(dateAt(nowUtc, timeZone)),
    nowMin = $derived(localMinute(nowUtc, timeZone));
  const shade = $derived(date < today ? 1440 : date === today ? nowMin : 0);
  const preview = $derived(gesture?.preview ?? selection);
  const clamp = (n: number, min: number, max: number) =>
    Math.min(max, Math.max(min, n));
  const minute = (y: number) =>
    Math.round(
      (y - wrapper.getBoundingClientRect().top + wrapper.scrollTop) / 15,
    ) * 15;
  function dropStart(x: number, y: number) {
    if (!wrapper || pending || busy || selection || gesture) return null;
    const bounds = wrapper.getBoundingClientRect();
    if (x < bounds.left + 88 || x >= bounds.right - 12 || y < bounds.top || y >= bounds.bottom) return null;
    return clamp(minute(y), 0, 1440 - SIDEBAR_BLOCK_MINUTES);
  }
  const taskPreview = $derived.by(() => {
    scrollTop;
    if (!sidebarDrag) return null;
    const start = dropStart(sidebarDrag.x, sidebarDrag.y);
    return start === null ? null : { ...sidebarDrag, start, end: start + SIDEBAR_BLOCK_MINUTES };
  });
  export function previewTaskDrop(drag: PlannerTaskDrag | null) {
    sidebarDrag = drag;
    if (!drag) {
      cancelAnimationFrame(taskFrame);
      taskFrame = 0;
    } else if (!taskFrame) taskFrame = requestAnimationFrame(scrollTaskDrag);
  }
  function scrollTaskDrag() {
    taskFrame = 0;
    if (!sidebarDrag || pending || busy) return;
    const bounds = wrapper.getBoundingClientRect();
    if (sidebarDrag.x >= bounds.left + 88 && sidebarDrag.x < bounds.right &&
        sidebarDrag.y >= bounds.top && sidebarDrag.y < bounds.bottom) {
      const speed = sidebarDrag.y < bounds.top + 36 ? -10 : sidebarDrag.y > bounds.bottom - 36 ? 10 : 0;
      if (speed) { wrapper.scrollTop += speed; scrollTop = wrapper.scrollTop; }
    }
    taskFrame = requestAnimationFrame(scrollTaskDrag);
  }
  export async function dropTaskAt(id: string, x: number, y: number) {
    const start = dropStart(x, y);
    previewTaskDrop(null);
    if (start === null) return;
    busy = true;
    error = '';
    try { await onDropTask(id, start); }
    catch (e) { error = errorMessage(e); }
    finally { busy = false; }
  }
  function cancel() {
    const old = gesture;
    gesture = null;
    cancelAnimationFrame(frame);
    frame = 0;
    if (old && wrapper?.hasPointerCapture?.(old.pointer))
      wrapper.releasePointerCapture(old.pointer);
  }
  $effect(() => {
    date;
    untrack(() => {
      cancel();
      previewTaskDrop(null);
      if (wrapper) {
        wrapper.scrollTop = 420;
        scrollTop = 420;
      }
    });
  });
  onMount(() => {
    wrapper.scrollTop = 420;
    const observer = new ResizeObserver(
      () => (visibleHeight = wrapper.clientHeight),
    );
    observer.observe(wrapper);
    return () => {
      observer.disconnect();
      cancel();
      previewTaskDrop(null);
    };
  });
  function update(y: number) {
    if (!gesture) return;
    const m = minute(y);
    let start: number, end: number;
    if (gesture.kind === 'select') {
      start = clamp(Math.min(gesture.origin, m), 0, 1425);
      end = clamp(Math.max(gesture.origin, m), start + 15, 1440);
    } else if (gesture.kind === 'move') {
      const duration = gesture.end - gesture.start;
      start = clamp(gesture.start + m - gesture.origin, 0, 1440 - duration);
      end = start + duration;
    } else {
      start = gesture.start;
      end = clamp(m, start + 15, 1440);
    }
    gesture.preview = { startMin: start, endMin: end };
  }
  function autoscroll() {
    if (!gesture) return;
    const r = wrapper.getBoundingClientRect();
    const speed =
      pointerY < r.top + 36 ? -10 : pointerY > r.bottom - 36 ? 10 : 0;
    if (speed) {
      wrapper.scrollTop += speed;
      scrollTop = wrapper.scrollTop;
      update(pointerY);
    }
    frame = requestAnimationFrame(autoscroll);
  }
  function begin(e: PointerEvent, kind: Gesture['kind'], item?: PlannerItem) {
    if (e.button !== 0 || pending || busy || selection || gesture) return;
    e.preventDefault();
    e.stopPropagation();
    const origin = clamp(minute(e.clientY), 0, 1425);
    gesture = {
      kind,
      pointer: e.pointerId,
      origin,
      start: item?.start_min ?? origin,
      end: item?.end_min ?? origin + 15,
      id: item?.id,
      preview: {
        startMin: item?.start_min ?? origin,
        endMin: item?.end_min ?? origin + 15,
      },
    };
    pointerY = e.clientY;
    wrapper.setPointerCapture(e.pointerId);
    frame = requestAnimationFrame(autoscroll);
  }
  function down(e: PointerEvent) {
    if (
      (e.target as HTMLElement).closest('[data-planner-item],button') ||
      e.clientX < wrapper.getBoundingClientRect().left + 88
    )
      return;
    begin(e, 'select');
  }
  async function up(e: PointerEvent) {
    if (!gesture || e.pointerId !== gesture.pointer) return;
    update(e.clientY);
    const g = gesture;
    cancel();
    if (g.kind === 'select') {
      const r = wrapper.getBoundingClientRect();
      onSelect(
        g.preview.startMin,
        g.preview.endMin,
        new DOMRect(
          r.right - 20,
          r.top + g.preview.startMin - wrapper.scrollTop,
          0,
          g.preview.endMin - g.preview.startMin,
        ),
      );
    } else if (
      g.id &&
      (g.start !== g.preview.startMin || g.end !== g.preview.endMin)
    ) {
      busy = true;
      error = '';
      try {
        await onMove(g.id, g.preview.startMin, g.preview.endMin);
      } catch (e) {
        error = errorMessage(e);
      } finally {
        busy = false;
      }
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      cancel();
      onCancelSelection();
    }
  }}
/>
<div class="canvas-shell">
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="fade-label">
    {minuteLabel(0)}–{minuteLabel(Math.floor(scrollTop / 60) * 60)} · {items.some(
      (i) => i.start_min < scrollTop,
    )
      ? 'Earlier blocks above'
      : 'Nothing planned earlier'}
  </div>
  <div
    class="canvas-scroll"
    bind:this={wrapper}
    role="application"
    aria-label="Daily schedule. Use Add block or Edit time for keyboard scheduling."
    onscroll={() => (scrollTop = wrapper.scrollTop)}
    onpointerdown={down}
    onpointermove={(e) => {
      if (gesture?.pointer === e.pointerId) {
        pointerY = e.clientY;
        update(pointerY);
      }
    }}
    onpointerup={up}
    onpointercancel={cancel}
    onlostpointercapture={cancel}
  >
    <div class="grid">
      {#if !items.length}<div
          class="empty-hint"
          style:top={`${Math.min(1320, scrollTop + 60)}px`}
        >
          Drag across any hours to plan a block<span
            >Or use Add block to choose a task and time.</span
          >
        </div>{/if}
      <div class="past" style:height={`${shade}px`}></div>
      {#each Array.from({ length: 25 }, (_, i) => i) as hour}<div
          class="hour"
          style:top={`${hour * 60}px`}
        >
          <span>{minuteLabel(hour * 60)}</span>
        </div>{/each}
      {#each packed as p (p.item.id)}
        <div
          data-planner-item={p.item.id}
          class="position"
          class:source={gesture?.id === p.item.id}
          style:top={`${p.top}px`}
          style:height={`${p.height}px`}
          style:left={`calc(88px + (100% - 88px + 4px) * ${p.lane / p.lanes})`}
          style:width={`calc((100% - 88px + 4px) / ${p.lanes} - 4px)`}
        >
          <PlannerBlock
            item={p.item}
            compact={p.lanes >= 4 || p.height < 40}
            dense={p.height < 80}
            {nowUtc}
            {timeZone}
            pending={pending || busy}
            {onOpenTask}
            {onDone}
            {onRemove}
            {onTimer}
            {onJoin}
            {onOpenMeeting}
            {onEditTime}
            onGesture={(e, kind) => begin(e, kind, p.item)}
          />
        </div>
      {/each}
      {#if taskPreview}<div class="task-drop-preview" data-task-drop-preview
          data-color={taskPreview.project_color}
          style:top={`${taskPreview.start}px`} style:height={`${SIDEBAR_BLOCK_MINUTES}px`}>
          <strong>{taskPreview.title}</strong>
          <span>{minuteLabel(taskPreview.start)}–{minuteLabel(taskPreview.end)} · 1h · Release to schedule</span>
        </div>{/if}
      {#if preview}<div
          class="selection"
          style:top={`${preview.startMin}px`}
          style:height={`${preview.endMin - preview.startMin}px`}
        >
          {minuteLabel(preview.startMin)}–{minuteLabel(preview.endMin)} · {gesture?.kind ===
            'select' || selection
            ? 'Select a task or personal time'
            : 'Release to save'}
        </div>{/if}
      {#if date === today}<div class="now" style:top={`${nowMin}px`}>
          <span></span><time>{minuteLabel(Math.floor(nowMin))}</time>
        </div>{/if}
    </div>
  </div>
  <div class="fade-label">
    {items.some((i) => i.end_min > scrollTop + visibleHeight)
      ? 'Later blocks below'
      : 'Nothing planned later'} · {minuteLabel(1440)}
  </div>
</div>

<style>
  .task-drop-preview {
    position: absolute;
    left: 88px;
    right: 0;
    z-index: 3;
    pointer-events: none;
    border: 2px dashed var(--project-color, var(--accent));
    border-radius: var(--radius-input);
    background: var(--selection-fill);
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .task-drop-preview strong {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .task-drop-preview span { font-size: var(--text-meta); color: var(--text-muted); }
  .canvas-shell {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-section);
    border-radius: var(--radius-panel);
    background: var(--bg-raised);
    overflow: hidden;
  }
  .canvas-scroll {
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
    flex: 1;
    scrollbar-width: thin;
    touch-action: none;
    overscroll-behavior: contain;
  }
  .grid {
    height: 1441px;
    position: relative;
    margin-right: 12px;
  }
  .hour {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border);
    pointer-events: none;
  }
  .hour span {
    white-space: nowrap;
    position: absolute;
    top: -9px;
    width: 78px;
    text-align: right;
    color: var(--text-faint);
    background: var(--bg-raised);
    font: var(--weight-hour) var(--text-meta) var(--font-heading);
  }
  .hour:after {
    content: '';
    position: absolute;
    top: 29px;
    left: 88px;
    right: 0;
    border-top: 1px dotted var(--border);
  }
  .past {
    position: absolute;
    left: 88px;
    right: 0;
    background: var(--past-shade);
    pointer-events: none;
  }
  .position {
    position: absolute;
    min-width: 0;
    z-index: 1;
  }
  .source {
    opacity: var(--opacity-drag-source);
  }
  .selection {
    position: absolute;
    left: 88px;
    right: 0;
    z-index: 3;
    border: 1.5px dashed var(--selection-border);
    background: var(--selection-fill);
    color: var(--accent);
    padding: 3px 10px;
    pointer-events: none;
  }
  .now {
    position: absolute;
    left: 88px;
    right: 0;
    height: 1px;
    background: var(--warn);
    z-index: 4;
    pointer-events: none;
  }
  .now span {
    position: absolute;
    width: var(--now-dot);
    height: var(--now-dot);
    left: -3px;
    top: -3px;
    border-radius: 50%;
    background: var(--warn);
  }
  .now time {
    white-space: nowrap;
    position: absolute;
    right: calc(100% + 8px);
    top: -8px;
    color: var(--warn);
    background: var(--bg-raised);
    font: var(--weight-hour) var(--text-meta) var(--font-heading);
  }
  .empty-hint {
    position: absolute;
    left: 104px;
    right: 20px;
    border: 1.5px dashed var(--selection-border);
    border-radius: var(--radius-input);
    padding: 18px;
    color: var(--accent);
    background: var(--selection-fill);
    pointer-events: none;
  }
  .empty-hint span {
    display: block;
    color: var(--text-muted);
    font-size: var(--text-meta);
    margin-top: 6px;
  }
  .fade-label {
    color: var(--text-faint);
    font-size: var(--text-meta);
    text-align: center;
    padding: 5px;
    pointer-events: none;
  }
  .error {
    padding: 6px;
    margin: 0;
  }
</style>
