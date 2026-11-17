// The stand-in consumer of the audio ring for the time the AudioWorklet does not run (decision 0211).
// With the audio output on by default the engine always has an external consumer: its DirectSound play cursor is the worklet's read position RD (decision 0139). A browser keeps an AudioContext suspended until the page has had
// a gesture (and for ever when it blocks audio, or has no output device); then RD stands still, the guest's play cursor stands still, and the song does not advance (measured, decision 0211: transport started, song position
// frozen, "the virtual clock fell behind" warnings). This moves RD with the page's clock instead, at the ring's rate, while the engine says the device is PLAYING and the worklet is not processing; nothing is heard then
// (the engine's frames are overwritten in the ring) but the transport runs as it did before the audio output was on by default. As soon as the worklet processes quanta again it owns RD and this does nothing.
// It only writes RD (the worklet's own word) and reads the worklet's quantum counter and the device state; it never touches the data.
import { W } from './ring-layout.ts';

const STATE_PLAYING = 2;
/** how long the worklet's quantum counter must stand still before this sink moves RD (S-aud3-grace): the audio thread can hiccup for tens of ms under load without being gone, and two writers moving RD would skip audio */
export const GRACE_MS = 100;

export class SoftSink {
  /** frames RD was moved by this sink, and how many ticks moved it (for the beacon: 0 when the worklet did all the work) */
  frames = 0;
  ticksMoved = 0;
  private lastQ = -1;
  private baseAt = 0;
  private carry = 0;
  private engaged = false;
  private readonly header: Int32Array;
  private readonly rate: () => number;

  constructor(header: Int32Array, rate: () => number) { this.header = header; this.rate = rate; }

  /** one tick at page time `now` (ms); returns the frames RD was moved by */
  tick(now: number): number {
    const h = this.header;
    const q = Atomics.load(h, W.Q_TOTAL);
    if (q !== this.lastQ) { this.lastQ = q; this.baseAt = now; this.carry = 0; this.engaged = false; return 0; }          // the worklet processes quanta (or this is the first tick)
    if (Atomics.load(h, W.STATE) !== STATE_PLAYING) { this.baseAt = now; this.carry = 0; this.engaged = false; return 0; }
    if (!this.engaged && now - this.baseAt < GRACE_MS) return 0;
    this.engaged = true;
    const dt = now - this.baseAt;
    this.baseAt = now;
    if (!(dt > 0)) return 0;
    const exact = (dt * this.rate()) / 1000 + this.carry;
    const n = Math.floor(exact);
    this.carry = exact - n;
    if (n > 0) { Atomics.add(h, W.RD, n); this.frames += n; this.ticksMoved++; }
    return n;
  }
}
