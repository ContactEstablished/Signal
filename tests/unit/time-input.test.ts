// @vitest-environment jsdom
import { expect, it, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import TimeInput from '../../src/lib/components/TimeInput.svelte';
let component: ReturnType<typeof mount>;
afterEach(async () => { if (component) await unmount(component); document.body.innerHTML = ''; });

function setup(value: string) {
  const oninput = vi.fn();
  component = mount(TimeInput, { target: document.body, props: { value, label: 'Time', oninput } });
  flushSync();
  return { field: document.querySelector('input')!, oninput };
}

it('shows AM/PM without changing a saved fractional time on mount or blur', () => {
  const { field, oninput } = setup('15:30:12.345');
  expect(field.value).toBe('3:30:12.345 PM');
  field.dispatchEvent(new FocusEvent('blur')); flushSync();
  expect(field.value).toBe('3:30:12.345 PM');
  expect(oninput).not.toHaveBeenCalled();
});

it('accepts shorthand, supplies canonical time, and normalizes the visible text on blur', () => {
  const { field, oninput } = setup('09:00');
  field.value = '2pm'; field.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  expect(oninput).toHaveBeenLastCalledWith('14:00');
  field.dispatchEvent(new FocusEvent('blur')); flushSync();
  expect(field.value).toBe('2:00 PM');
  expect(field.checkValidity()).toBe(true);
});

it('keeps invalid text visible and prevents browser submission until corrected', () => {
  const { field, oninput } = setup('09:00');
  field.value = '2:90 PM'; field.dispatchEvent(new Event('input', { bubbles: true }));
  field.dispatchEvent(new FocusEvent('blur')); flushSync();
  expect(field.value).toBe('2:90 PM');
  expect(field.checkValidity()).toBe(false);
  expect(oninput).toHaveBeenLastCalledWith('2:90 PM');
  field.value = '2:30 PM'; field.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  expect(field.checkValidity()).toBe(true);
  expect(oninput).toHaveBeenLastCalledWith('14:30');
});
