// A stand-in plugin thread for the protocol tests (no wasm): answers every PROCESS request with the input times 0.5 and leaves on QUIT. Runs as a node worker_threads Worker (node strips the types).
import { workerData } from 'node:worker_threads';
import { CMD, Slot, W, respond, waitRequest } from '../../src/plugins/protocol.ts';

const slot = Slot.attach((workerData as { sab: SharedArrayBuffer }).sab);
let last = 0;
for (;;) {
  const seq = waitRequest(slot, last);
  last = seq;
  if (Atomics.load(slot.ctrl, W.CMD) === CMD.QUIT) {
    respond(slot, seq, 0);
    break;
  }
  const n = Atomics.load(slot.ctrl, W.FRAMES);
  for (let i = 0; i < n; i++) {
    slot.outL[i] = slot.inL[i]! * 0.5;
    slot.outR[i] = slot.inR[i]! * 0.5;
  }
  respond(slot, seq, 7);
}
