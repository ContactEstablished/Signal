import { it, expect } from 'vitest';
import {
  newTaskDraft,
  normalizeDraft,
  estimateValue,
  isDirty,
} from '../../src/lib/domain/task-draft';
it('validates creation without conflating blank estimates and zero', () => {
  const initial = newTaskDraft('p');
  expect(estimateValue('')).toBeNull();
  expect(estimateValue('0')).toBe(0);
  expect(() => estimateValue('-1')).toThrow();
  expect(() => estimateValue('Infinity')).toThrow();
  expect(() => normalizeDraft(initial)).toThrow();
  const draft = { ...initial, title: ' Task ', alerts: [60, 60, 1440] };
  expect(normalizeDraft(draft).alerts).toEqual([1440, 60]);
  expect(normalizeDraft(draft).title).toBe('Task');
  expect(isDirty(draft, initial)).toBe(true);
  expect(isDirty(initial, newTaskDraft('p'))).toBe(false);
});
