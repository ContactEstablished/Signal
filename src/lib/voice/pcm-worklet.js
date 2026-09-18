// Adapted from Chorus: mono PCM frames; flush the tail before stop acknowledgement.
class Capture extends AudioWorkletProcessor {
  constructor() {
    super();
    this.samples = [];
    this.stopped = false;
    this.port.onmessage = ({ data }) => {
      if (data === 'stop') {
        this.stopped = true;
        if (this.samples.length)
          this.port.postMessage(new Float32Array(this.samples));
        this.samples = [];
        this.port.postMessage('stopped');
      }
    };
  }
  process(inputs) {
    if (this.stopped) return false;
    const channel = inputs[0]?.[0];
    if (channel)
      for (const sample of channel) {
        this.samples.push(sample);
        if (this.samples.length === 1024) {
          this.port.postMessage(new Float32Array(this.samples));
          this.samples = [];
        }
      }
    return true;
  }
}
registerProcessor('signal-pcm', Capture);
