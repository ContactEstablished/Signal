<script lang="ts">
  let { title, hint, x, y, color }: {
    title: string; hint: string; x: number; y: number; color?: string;
  } = $props();
</script>

<div class="task-drag-ghost" data-task-drag-ghost data-color={color}
  style:left={`${x + 14}px`} style:top={`${y + 14}px`} aria-hidden="true">
  <strong>{title}</strong><span>{hint}</span>
</div>

<style>
  .task-drag-ghost{position:fixed;z-index:1000;pointer-events:none;width:260px;height:60px;display:flex;flex-direction:column;gap:4px;padding:8px 12px;border:1px solid var(--project-color,var(--accent));border-radius:var(--radius-input);background:var(--surface);opacity:.75;box-shadow:0 8px 24px #0005}
  strong{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;font-size:var(--text-label)}
  span{font-size:var(--text-meta);color:var(--text-muted)}
</style>
