// Decision C-2: the handshake between the engine Worker (which composes the picture in the engine's shared memory) and the render Worker (which draws what changed). One small shared block of Int32 words:
// GEN is the generation of the picture the engine published last; ACK is the generation the render Worker has finished reading. The engine composes only while ACK = GEN, so the picture (which only
// fle_desk_compose writes) does not change while the render Worker reads it. Only the engine writes GEN and only the render Worker writes ACK.

const GEN = 0, ACK = 1, MODE = 2;
export const DESK_CTL_BYTES = 64;

/** the render Worker's presenter, once it exists: none yet, the picture with putImageData, or the scene with WebGL2 */
export const PRESENTER_NONE = 0, PRESENTER_IMAGE = 1, PRESENTER_SCENE = 2;

export const newDeskControl = (): SharedArrayBuffer => new SharedArrayBuffer(DESK_CTL_BYTES);

export class DeskGate {
  private readonly ctl: Int32Array;
  constructor(sab: SharedArrayBuffer) { this.ctl = new Int32Array(sab); }
  /** the render Worker has not finished reading the last published picture */
  get busy(): boolean { return Atomics.load(this.ctl, ACK) !== Atomics.load(this.ctl, GEN); }
  /** the generation of the last picture published */
  get published(): number { return Atomics.load(this.ctl, GEN); }
  publish(gen: number): void { Atomics.store(this.ctl, GEN, gen); }
  ack(gen: number): void { Atomics.store(this.ctl, ACK, gen); }
  /** the render Worker's presenter (PRESENTER_*): the engine composes for it only once one exists (Issue #1520) */
  get presenter(): number { return Atomics.load(this.ctl, MODE); }
  setPresenter(mode: number): void { Atomics.store(this.ctl, MODE, mode); }
}
