<script lang="ts">
  import { wallTimeLabel, parseWallTime } from '../domain/time-display';
  let { value = '', label, disabled = false, oninput, onchange, element = $bindable() }: {
    value?: string;
    label: string;
    disabled?: boolean;
    oninput: (value: string) => void;
    onchange?: () => void;
    element?: HTMLInputElement;
  } = $props();
  let text = $state('');
  let lastValue: string | undefined;
  $effect(() => {
    if (value !== lastValue) {
      lastValue = value;
      const formatted = wallTimeLabel(value);
      text = formatted;
      element?.setCustomValidity(parseWallTime(formatted) === null ? 'Enter a time like 2:30 PM.' : '');
    }
  });
  function input(event: Event) {
    const field = event.currentTarget as HTMLInputElement;
    text = field.value;
    const parsed = parseWallTime(text);
    field.setCustomValidity(parsed === null ? 'Enter a time like 2:30 PM.' : '');
    lastValue = parsed ?? text;
    oninput(lastValue);
  }
  function blur() {
    const parsed = parseWallTime(text);
    if (parsed !== null) text = wallTimeLabel(parsed);
  }
</script>

<input bind:this={element} type="text" aria-label={label} {disabled}
  value={text} oninput={input} onchange={() => onchange?.()} onblur={blur}
  placeholder="2:30 PM" autocomplete="off" spellcheck={false} />

<style>
  input { width: 100%; min-width: 0; }
</style>
