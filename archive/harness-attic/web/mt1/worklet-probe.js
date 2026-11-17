class P extends AudioWorkletProcessor {
  constructor() {
    super(); this.n = 0;
    const r = {};
    try {
      r.SharedArrayBuffer = typeof SharedArrayBuffer; r.Atomics_wait = typeof Atomics.wait; r.waitAsync = typeof Atomics.waitAsync;
      r.WebAssembly = typeof WebAssembly;
      r.JSPI = typeof WebAssembly.Suspending + '/' + typeof WebAssembly.promising;
      try { new WebAssembly.Memory({ initial: 1, maximum: 2, shared: true }); r.Memory_shared = 'ok'; } catch (e) { r.Memory_shared = 'ERR ' + e.message; }
      r.Worker = typeof Worker; r.crossOriginIsolated = globalThis.crossOriginIsolated; r.sampleRate = sampleRate; r.fetch = typeof fetch; r.performance = typeof performance; r.currentFrame = typeof currentFrame;
      r.navigator = typeof navigator; r.TextDecoder = typeof TextDecoder; r.setTimeout = typeof setTimeout; r.postMessage_port = typeof this.port.postMessage;
      try { const ia = new Int32Array(new SharedArrayBuffer(8)); r.Atomics_wait_in_worklet = Atomics.wait(ia, 0, 0, 1); } catch (e) { r.Atomics_wait_in_worklet = 'ERR ' + e.message; }
    } catch (e) { r.err = String(e); }
    this.port.postMessage(r);
    this.port.onmessage = (e) => { if (e.data && e.data.sab) { this.ia = new Int32Array(e.data.sab); } };
  }
  process() {
    this.n++;
    if (this.n === 200) this.port.postMessage({ quanta: this.n, currentTime });
    return true;
  }
}
registerProcessor('p', P);
