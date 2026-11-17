// The input state machine of the page (decision 0141): DOM pointer / keyboard / wheel facts in, RawInput events out.
//  - InputTracker keeps what the page must know to stay in step with the engine: which mouse buttons and which keys it has told the engine are down, so that a lost mouse-up, a blur or a pointercancel
//    ends with a button-up / key-up for each of them and the engine never keeps a key or button "stuck";
//  - buttons are taken from the pointer event's `buttons` MASK, not from `button`: per MDN "pointerdown ... for mouse, fired when the device transitions from no buttons pressed to at least one button pressed",
//    so a second button pressed while the first is held arrives as a pointermove whose `buttons` changed;
//  - clientToScreen() maps a CSS position on the desktop view to the engine's integer screen position;
//  - WheelAccumulator turns the DOM's deltaY into the engine's wheel delta (WHEEL_DELTA 120 per notch, positive = away from the user).
import { BTN, IN, SCREEN, type RawInput } from './events.ts';
import { keyInfo, type KeyInfo } from './keymap.ts';

export interface ViewRect { left: number; top: number; width: number; height: number }

/** the engine's screen position (integers, clamped to the virtual screen) under a CSS client position of the desktop view; `inside` says whether the position was on the view at all */
export function clientToScreen(clientX: number, clientY: number, rect: ViewRect, screen: { width: number; height: number } = SCREEN): { x: number; y: number; inside: boolean } {
  if (!(rect.width > 0) || !(rect.height > 0)) return { x: 0, y: 0, inside: false };
  const fx = (clientX - rect.left) / rect.width, fy = (clientY - rect.top) / rect.height;
  const inside = fx >= 0 && fx < 1 && fy >= 0 && fy < 1;
  const clamp = (f: number, max: number): number => Math.max(0, Math.min(max - 1, Math.floor(f * max)));
  return { x: clamp(fx, screen.width), y: clamp(fy, screen.height), inside };
}

/** DOM `buttons` mask bit -> engine button (BTN_*) */
const MASK_BUTTONS: readonly [number, number][] = [[1, BTN.L], [2, BTN.R], [4, BTN.M]];

export interface PointerFact { x: number; y: number; buttons: number }

export class InputTracker {
  private mask = 0;
  private lastX = -1;
  private lastY = -1;
  private readonly held = new Map<string, KeyInfo>();
  private readonly emit: (e: RawInput) => void;
  constructor(emit: (e: RawInput) => void) { this.emit = emit; }

  get buttonsDown(): number { return this.mask; }
  get keysDown(): string[] { return [...this.held.keys()]; }

  private move(x: number, y: number): void {
    if (x === this.lastX && y === this.lastY) return;
    this.lastX = x;
    this.lastY = y;
    this.emit({ k: 'mouse', kind: IN.MOVE, x, y, data: 0 });
  }

  /** a pointermove / pointerdown / pointerup: the position first (when it changed), then the buttons that changed, ups before downs */
  pointer(p: PointerFact): void {
    this.move(p.x, p.y);
    const now = p.buttons & 7;
    const changed = now ^ this.mask;
    if (changed === 0) return;
    for (const [bit, btn] of MASK_BUTTONS) if ((changed & bit) && !(now & bit)) this.emit({ k: 'mouse', kind: IN.BUP, x: p.x, y: p.y, data: btn });
    for (const [bit, btn] of MASK_BUTTONS) if ((changed & bit) && (now & bit)) this.emit({ k: 'mouse', kind: IN.BDOWN, x: p.x, y: p.y, data: btn });
    this.mask = now;
  }

  /** pointercancel: the pointer will send nothing more, so every button the engine believes down is released where the pointer last was */
  pointerCancel(): void { this.pointer({ x: Math.max(0, this.lastX), y: Math.max(0, this.lastY), buttons: 0 }); }

  wheel(x: number, y: number, delta: number): void {
    if (delta === 0) return;
    this.move(x, y);
    this.emit({ k: 'mouse', kind: IN.WHEEL, x, y, data: delta });
  }

  /** a keydown for a key code the policy forwards (auto-repeats too: the engine derives "previous key state" from its own state) */
  keyDown(code: string): boolean {
    const k = keyInfo(code);
    if (!k) return false;
    this.held.set(code, k);
    this.emit({ k: 'key', kind: IN.KDOWN, vk: k.vk, scan: k.scan, ext: k.ext ? 1 : 0 });
    return true;
  }

  keyUp(code: string): boolean {
    const k = keyInfo(code);
    if (!k) return false;
    this.held.delete(code);
    this.emit({ k: 'key', kind: IN.KUP, vk: k.vk, scan: k.scan, ext: k.ext ? 1 : 0 });
    return true;
  }

  /** the desktop lost the focus (blur, hidden tab, window deactivated): release every key (last pressed first) and every button */
  releaseAll(): void {
    for (const [, k] of [...this.held.entries()].reverse()) this.emit({ k: 'key', kind: IN.KUP, vk: k.vk, scan: k.scan, ext: k.ext ? 1 : 0 });
    this.held.clear();
    if (this.mask !== 0) this.pointerCancel();
  }
}

/** DOM wheel -> engine wheel. deltaMode 0 (pixels): 100 px = one notch of WHEEL_DELTA = 120 (Chrome's usual detent), remainders add up so that a trackpad's small steps still turn into notches; 1 (lines): 120 per line; 2 (pages): 120 * 3 per page. Positive deltaY scrolls DOWN in the DOM, the engine's positive delta is "forward" (up), so the sign flips. */
export class WheelAccumulator {
  private rest = 0;
  private lastAt = -Infinity;
  private readonly resetMs: number;
  constructor(resetMs = 300) { this.resetMs = resetMs; }
  add(deltaY: number, deltaMode: number, at: number): number {
    if (at - this.lastAt > this.resetMs) this.rest = 0;
    this.lastAt = at;
    const notches = deltaMode === 1 ? deltaY : deltaMode === 2 ? deltaY * 3 : deltaY / 100;
    this.rest += -notches * 120;
    const whole = Math.trunc(this.rest / 120) * 120 || 0;   // (|| 0: no negative zero)
    this.rest -= whole;
    return whole;
  }
}
