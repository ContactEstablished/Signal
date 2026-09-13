import type { TimerUiBindings } from '../../src/lib/components/tasks/TaskTimeCard.svelte';
export function reactiveTime(overrides: Partial<TimerUiBindings> = {}) {
  const time = $state<TimerUiBindings>({
    session: null,
    entries: [],
    nowUtc: '2025-09-11T12:00:00Z',
    timeZone: 'UTC',
    loading: false,
    error: '',
    pending: false,
    recoveryRequired: false,
    logCompletionVersion: 0,
    onStart: async () => {},
    onPause: async () => {},
    onResume: async () => {},
    onStop: async () => {},
    onLog: async () => {},
    onRetry: async () => {},
    ...overrides,
  });
  return time;
}
