// mt1: does a JSPI-suspending process proxy let several guest threads of ONE Worker keep several plugin Workers busy at once? (decision 0173 §5.1, B-sync + thread bank)
// The "guest" is a tiny wasm module (WAT in the repo history of this file: guest.wat) whose only import is proc(k). Three ways to call the plugins, K plugin Workers each spinning workUs per block:
//   seq    : ONE guest thread calls plugin 0..K-1 one after the other; proc blocks the Worker with Atomics.wait (a plain import)         -> expected ~ K * work
//   jspi   : K guest threads (WebAssembly.promising), each loops over its own plugin; proc is WebAssembly.Suspending and waits with Atomics.waitAsync  -> expected ~ work
// wasm is the bytes of guest.wat (wat2wasm).
const WASM_B64 = 'AGFzbQEAAAABDAJgAX8Bf2ACf38BfwIMAQNlbnYEcHJvYwAAAwMCAQEHDwIFZ3Vlc3QAAQNzZXEAAgpnAiUBAn8CQANAIAIgAU8NASADIAAQAGohAyACQQFqIQIMAAsLIAMLPwEDfwJAA0AgAiABTw0BQQAhAwJAA0AgAyAATw0BIAQgAxAAaiEEIANBAWohAwwACwsgAkEBaiECDAALCyAECw==';
const FRAMES = 4096, BLK = FRAMES * 2 * 4, OFF_IN = 64, OFF_OUT = 64 + BLK;
function b64(s) { const b = atob(s); const u = new Uint8Array(b.length); for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i); return u; }
self.onmessage = async (e) => {
  const { sabs, mode, blocks, workUs } = e.data;
  const K = sabs.length;
  const ctrls = sabs.map((s) => new Int32Array(s, 0, 16)), ins = sabs.map((s) => new Float32Array(s, OFF_IN, BLK / 4)), outs = sabs.map((s) => new Float32Array(s, OFF_OUT, BLK / 4));
  const src = new Float32Array(BLK / 4).fill(0.25), dst = new Float32Array(BLK / 4);
  for (const c of ctrls) Atomics.store(c, 2, workUs);
  const seqs = new Int32Array(K);
  const send = (k) => { const r = ++seqs[k]; ins[k].set(src); Atomics.store(ctrls[k], 0, r); Atomics.notify(ctrls[k], 0); return r; };
  const blocking = (k) => { const r = send(k); const c = ctrls[k]; for (;;) { const v = Atomics.load(c, 1); if (v === r) break; Atomics.wait(c, 1, v); } dst.set(outs[k]); return 1; };
  const suspending = async (k) => { const r = send(k); const c = ctrls[k]; for (;;) { const v = Atomics.load(c, 1); if (v === r) break; const w = Atomics.waitAsync(c, 1, v); if (w.async) await w.value; } dst.set(outs[k]); return 1; };
  const mod = await WebAssembly.compile(b64(WASM_B64));
  const t0 = performance.now();
  let steps = 0;
  if (mode === 'seq') {
    const inst = await WebAssembly.instantiate(mod, { env: { proc: blocking } });
    steps = inst.exports.seq(K, blocks);
  } else {
    const inst = await WebAssembly.instantiate(mod, { env: { proc: new WebAssembly.Suspending(suspending) } });
    const guest = WebAssembly.promising(inst.exports.guest);
    const rs = await Promise.all(Array.from({ length: K }, (_, k) => guest(k, blocks)));   // K concurrent JSPI stacks
    steps = rs.reduce((a, b) => a + b, 0);
  }
  const ms = performance.now() - t0;
  for (const c of ctrls) { Atomics.store(c, 0, -1); Atomics.notify(c, 0); }
  postMessage({ ms, perBlockMs: ms / blocks, steps, expectedSteps: K * blocks });
};
