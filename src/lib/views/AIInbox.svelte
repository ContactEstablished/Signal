<script lang="ts">
  import { untrack } from 'svelte';
  import VoiceInbox from './VoiceInbox.svelte';
  import type { Workspace } from '../state/app.svelte';
  import type { CloseReason } from '../domain/types';
  import type { MeetingRef } from '../domain/agenda';
  let { workspace, onOpenTask, onOpenMeeting, onSettings, onGuard }: {
    workspace: Workspace; onOpenTask:(id:string)=>unknown; onOpenMeeting:(ref:MeetingRef)=>unknown;
    onSettings:()=>unknown; onGuard:(active:boolean)=>void;
  } = $props();
  let tab = $state<'voice'|'image'>(untrack(() => workspace.imageRecovery ? 'image' : 'voice'));
  let child = $state<VoiceInbox | null>(null);
  let switching = $state(false);
  export async function requestClose(reason: CloseReason) { return (await child?.requestClose(reason)) ?? true; }
  async function select(next: 'voice'|'image') {
    if (next === tab || switching) return;
    switching = true;
    try { if (await requestClose('navigation')) tab = next; }
    finally { switching = false; }
  }
</script>
<section class="ai-inbox" aria-label="AI Inbox">
<nav class="inbox-tabs" aria-label="AI Inbox inputs"><strong>AI Inbox</strong><button class:active={tab === 'voice'} aria-pressed={tab === 'voice'} disabled={switching} onclick={() => select('voice')}>Voice</button><button class:active={tab === 'image'} aria-pressed={tab === 'image'} disabled={switching} onclick={() => select('image')}>Image</button></nav>
{#key tab}<VoiceInbox bind:this={child} mode={tab} {workspace} {onOpenTask} {onOpenMeeting} {onSettings} {onGuard} />{/key}
</section>
<style>
  .ai-inbox { height: calc(100vh - var(--header-height)); display: flex; flex-direction: column; overflow: hidden; }
  .inbox-tabs { flex: none; display: flex; align-items: center; gap: 12px; padding: 12px 24px; border-bottom: 1px solid var(--border); }
  strong { margin-right: 12px; }
  button { padding: 8px 18px; border: 1px solid var(--border); border-radius: var(--radius-button); color: var(--text-muted); background: var(--surface); }
  button.active { color: var(--text); border-color: var(--accent); }
</style>
