<script lang="ts">
  import NewMeetingDialog from './NewMeetingDialog.svelte';
  import type { ComponentProps } from 'svelte';
  import type { MeetingWriteResult } from '../../domain/agenda';
  let props: ComponentProps<typeof NewMeetingDialog> = $props();
  let editor: NewMeetingDialog;
  export async function requestClose(_reason?: unknown) {
    return editor ? editor.requestClose() : true;
  }
  export function acceptRetry(result: MeetingWriteResult) {
    editor?.acceptRetry(result);
  }
</script>

<NewMeetingDialog bind:this={editor} {...props} />
