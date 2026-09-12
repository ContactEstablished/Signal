export function mutationQueue() {
  let tail: Promise<unknown> = Promise.resolve();
  return {
    enqueue<T>(operation: () => Promise<T>): Promise<T> {
      const result = tail.then(operation);
      tail = result.catch(() => undefined);
      return result;
    },
    settled: () => tail.then(() => undefined),
  };
}
export function latestQuery() {
  let generation = 0;
  return {
    next: () => ++generation,
    current: (ticket: number) => ticket === generation,
    invalidate: () => {
      generation++;
    },
  };
}
