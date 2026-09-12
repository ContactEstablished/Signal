import { it, expect } from 'vitest';
import { mutationQueue, latestQuery } from '../../src/lib/domain/mutations';
it('serializes intents through rejection and reads the latest revision at execution', async () => {
  const q = mutationQueue(),
    events: string[] = [];
  let revision = 0;
  const a = q.enqueue(async () => {
    events.push('a');
    await Promise.resolve();
    revision++;
  });
  const b = q.enqueue(async () => {
    expect(revision).toBe(1);
    events.push('b');
    throw new Error('rejected');
  });
  const c = q.enqueue(async () => {
    events.push('c');
    revision++;
    return revision;
  });
  await a;
  await expect(b).rejects.toThrow('rejected');
  expect(await c).toBe(2);
  expect(events).toEqual(['a', 'b', 'c']);
});
it('invalidates late responses after selection changes', () => {
  const q = latestQuery();
  const old = q.next();
  const current = q.next();
  expect(q.current(old)).toBe(false);
  expect(q.current(current)).toBe(true);
  q.invalidate();
  expect(q.current(current)).toBe(false);
});
