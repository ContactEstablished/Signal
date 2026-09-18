import { invoke } from '@tauri-apps/api/core';
import workletUrl from './pcm-worklet.js?url&no-inline';
export interface Recording {
  stop: () => Promise<string>;
  cancel: () => Promise<void>;
}
export async function record(
  deviceId: string,
  level: (value: number, seconds: number) => void,
  autoStop: () => void,
  failed: (error: unknown) => void,
): Promise<Recording> {
  let stream: MediaStream | undefined,
    context: AudioContext | undefined,
    node: AudioWorkletNode | undefined;
  let id: string | undefined,
    ended = false,
    sequence = 0,
    total = 0,
    queued = 0,
    failure: unknown;
  let chain = Promise.resolve();
  let acknowledge: (() => void) | undefined;
  const cleanup = async () => {
    stream?.getTracks().forEach((t) => t.stop());
    node?.disconnect();
    if (context && context.state !== 'closed') await context.close();
  };
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      audio: {
        channelCount: 1,
        ...(deviceId ? { deviceId: { exact: deviceId } } : {}),
      },
    });
    context = new AudioContext({ sampleRate: 16000 });
    if (context.sampleRate !== 16000)
      throw new Error(
        'This microphone cannot record at 16 kHz. Choose another input device.',
      );
    await context.audioWorklet.addModule(workletUrl);
    id = await invoke<string>('start_voice_capture');
    node = new AudioWorkletNode(context, 'signal-pcm');
    node.port.onmessage = ({ data }) => {
      if (data === 'stopped') {
        acknowledge?.();
        return;
      }
      if (failure) return;
      const floats = data as Float32Array;
      const samples = Array.from(
        floats.subarray(0, Math.max(0, 16000 * 300 - total)),
        (v) => Math.round(Math.max(-1, Math.min(1, v)) * 32767),
      );
      if (!samples.length) return;
      total += samples.length;
      queued += samples.length;
      level(
        Math.sqrt(samples.reduce((n, s) => n + s * s, 0) / samples.length) /
          32768,
        total / 16000,
      );
      if (queued > 16000 * 120) {
        failure = new Error(
          'Audio delivery stalled. Record again after the app responds.',
        );
        failed(failure);
        return;
      }
      const n = sequence++;
      chain = chain
        .then(() =>
          invoke<void>('voice_frame', { captureId: id, sequence: n, samples }),
        )
        .then(() => {
          queued -= samples.length;
        })
        .catch((e) => {
          if (!failure) {
            failure = e;
            failed(e);
          }
        });
      if (total === 16000 * 300) autoStop();
    };
    context.createMediaStreamSource(stream).connect(node);
    // A silent output keeps the worklet processing without microphone feedback.
    const mute = context.createGain();
    mute.gain.value = 0;
    node.connect(mute).connect(context.destination);
    stream.getAudioTracks()[0].onended = () => {
      if (!ended) failed(new Error('Microphone disconnected. Record again.'));
    };
    await context.resume();
    return {
      stop: async () => {
        if (ended) throw new Error('Recording already stopped.');
        ended = true;
        try {
          await new Promise<void>((resolve, reject) => {
            const timer = setTimeout(
              () => reject(new Error('Microphone did not finish recording.')),
              2000,
            );
            acknowledge = () => {
              clearTimeout(timer);
              resolve();
            };
            node!.port.postMessage('stop');
          });
          await cleanup();
          await chain;
          if (failure) throw failure;
          return await invoke<string>('stop_voice_capture', { captureId: id });
        } catch (e) {
          await cleanup();
          await invoke('cancel_voice');
          throw e;
        }
      },
      cancel: async () => {
        ended = true;
        await cleanup();
        await chain;
        await invoke('cancel_voice');
      },
    };
  } catch (e) {
    ended = true;
    await cleanup();
    if (id) await invoke('cancel_voice');
    throw e;
  }
}
