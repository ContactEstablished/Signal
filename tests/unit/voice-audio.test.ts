import { it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
it('the worklet emits ordered mono frames, flushes a short tail, then stops processing', () => {
  const frames: unknown[] = [];
  let Processor: any;
  class Base {
    port = {
      postMessage: (v: unknown) => frames.push(v),
      onmessage: null as any,
    };
  }
  runInNewContext(readFileSync('src/lib/voice/pcm-worklet.js', 'utf8'), {
    AudioWorkletProcessor: Base,
    Float32Array,
    registerProcessor: (_name: string, ctor: any) => (Processor = ctor),
  });
  const processor = new Processor();
  for (let n = 0; n < 9; n++)
    processor.process([[new Float32Array(128).fill(n / 10)]]);
  expect(frames).toHaveLength(1);
  expect(frames[0] as Float32Array).toHaveLength(1024);
  expect((frames[0] as Float32Array)[0]).toBe(0);
  expect((frames[0] as Float32Array)[1023]).toBeCloseTo(0.7);
  processor.port.onmessage({ data: 'stop' });
  expect(frames).toHaveLength(3);
  expect(frames[1] as Float32Array).toHaveLength(128);
  expect((frames[1] as Float32Array)[0]).toBeCloseTo(0.8);
  expect(frames[2]).toBe('stopped');
  expect(processor.process([[new Float32Array(128)]])).toBe(false);
});
