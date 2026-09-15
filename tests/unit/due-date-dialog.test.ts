// @vitest-environment jsdom
import { it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import DueDateDialog from '../../src/lib/components/agenda/DueDateDialog.svelte';
let component: ReturnType<typeof mount>;
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
});
afterEach(async () => {
  if (component) await unmount(component);
  document.body.innerHTML = '';
});
it('submits unchanged fractional time once and retains a failed draft', async () => {
  let reject!: (e: unknown) => void;
  const save = vi.fn(() => new Promise<void>((_, r) => (reject = r)));
  component = mount(DueDateDialog, {
    target: document.body,
    props: {
      draft: { date: '2025-09-12', time: '15:30:12.345' },
      taskTitle: 'Task',
      reason: 'move',
      timeZone: 'UTC',
      onChange: vi.fn(),
      onSave: save,
      onCancel: vi.fn(),
      onRetry: vi.fn(),
    },
  });
  flushSync();
  const form = document.querySelector('form')!;
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  expect(save).toHaveBeenCalledTimes(1);
  expect(save).toHaveBeenCalledWith({
    date: '2025-09-12',
    time: '15:30:12.345',
  });
  reject(new Error('No write'));
  await vi.waitFor(() =>
    expect(document.body.textContent).toContain('No write'),
  );
  expect(
    document.querySelector<HTMLInputElement>('input[aria-label="Deadline time"]')!.value,
  ).toBe('3:30:12.345 PM');
});
it('keeps exact Retry enabled and prevents closure during unknown recovery', async () => {
  const retry = vi.fn();
  component = mount(DueDateDialog, {
    target: document.body,
    props: {
      draft: { date: '2025-11-02', time: '01:30:00.000' },
      taskTitle: 'Task',
      reason: 'fold',
      timeZone: 'America/New_York',
      recoveryRequired: true,
      onChange: vi.fn(),
      onSave: vi.fn(),
      onCancel: vi.fn(),
      onRetry: retry,
    },
  });
  flushSync();
  expect(
    await (
      component as { requestClose: () => Promise<boolean> }
    ).requestClose(),
  ).toBe(false);
  const button = [...document.querySelectorAll('button')].find((b) =>
    b.textContent?.includes('Retry'),
  )!;
  expect(button.disabled).toBe(false);
  button.click();
  expect(retry).toHaveBeenCalledOnce();
});
