// The DOM side of the input (decision 0141): listeners on the desktop element that turn pointer, wheel and key events into RawInput through the InputTracker (src/input/tracker.ts) and hand them to `onRaw`.
// What the listeners do, with the MDN pages they rest on (read 2026-11-01):
//  - pointer events: PointerEvent (pointerdown "for mouse ... fired when the device transitions from no buttons pressed to at least one button pressed", pointermove, pointerup, pointercancel);
//  - pointer capture: Element.setPointerCapture(pointerId) - "Subsequent events for the pointer will be targeted at the capture element until capture is released (via releasePointerCapture() or the pointerup event)",
//    so a drag that leaves the view still ends with a pointerup here;
//  - keys: KeyboardEvent.code, the physical key (src/input/keymap.ts); the browser's own action is suppressed only while the desktop has the focus (keyPolicy);
//  - the desktop is a focusable element (tabindex 0): a pointerdown on it focuses it; losing the focus (blur), a hidden tab or a pointercancel releases every key and button the engine was told about;
//  - decision 0260: the key listeners are on the desktop only, so when the browser moves the focus to another control of the page on its own (observed in headless Chrome, about once in 30 clicks: the "Monitor" button, no script call, no pointer press, no Tab) every key, an Escape that closes a menu included, is dropped until the next click: the focus goes back to the desktop (also when it fell to the page body) unless the user moved it (a pointer press on that control or outside the desktop, or a Tab, in the last second) or the control is a text field.
import type { RawInput } from './events.ts';
import type { InputMeta } from '../ui/latency.ts';
import { keyPolicy } from './keymap.ts';
import { clientToScreen, InputTracker, WheelAccumulator, type ViewRect } from './tracker.ts';

export interface DesktopInputOptions {
  /** false while the engine cannot take input (starting, stopped): the listeners then do nothing and the browser keeps every key */
  enabled(): boolean;
  /** Tab goes to the engine (default: Tab leaves the desktop) */
  sendTab?(): boolean;
  /** `meta` (decision 0202): when the browser made the DOM event (event.timeStamp) and when its listener started, on the clock performance.timeOrigin + performance.now(); absent for a cause that is not an input event (blur, hidden tab) */
  onRaw(r: RawInput, from: string, meta?: InputMeta): void;
  /** the rectangle of the desktop view in client coordinates */
  rect(): ViewRect;
  /** decision 0212: the size of the guest's screen now (default: 1920 x 1080, input/events.ts SCREEN) */
  screen?: () => { width: number; height: number };
}

type AnyListener = (e: never) => void;

export interface DesktopInput { tracker: InputTracker; detach(): void }

export function attachDesktopInput(el: HTMLElement, opts: DesktopInputOptions): DesktopInput {
  let from = 'init';
  let meta: InputMeta | undefined;
  const tracker = new InputTracker((r) => opts.onRaw(r, from, meta));
  const wheel = new WheelAccumulator();
  const at = (e: { clientX: number; clientY: number }): { x: number; y: number } => clientToScreen(e.clientX, e.clientY, opts.rect(), opts.screen?.());
  const cur = (name: string): void => { from = name; };

  const onPointer = (name: string) => (e: PointerEvent): void => {
    if (!opts.enabled()) return;
    if (e.pointerType !== 'mouse' && !e.isPrimary) return;
    cur(name);
    const p = at(e);
    if (name === 'pointercancel') { tracker.pointerCancel(); return; }
    // touch and pen report a contact as `buttons` 1 (Pointer Events), so they act as the left button
    tracker.pointer({ x: p.x, y: p.y, buttons: e.buttons });
  };
  const onDown = (e: PointerEvent): void => {
    if (!opts.enabled()) return;
    e.preventDefault();
    try { el.setPointerCapture(e.pointerId); } catch { /* a synthetic pointer has no capture */ }
    el.focus({ preventScroll: true });
    onPointer('pointerdown')(e);
  };
  const onUp = (e: PointerEvent): void => {
    if (!opts.enabled()) return;
    e.preventDefault();
    onPointer('pointerup')(e);
  };
  const onWheel = (e: WheelEvent): void => {
    if (!opts.enabled()) return;
    e.preventDefault();
    cur('wheel');
    const p = at(e);
    const d = wheel.add(e.deltaY, e.deltaMode, performance.now());
    if (d !== 0) tracker.wheel(p.x, p.y, d);
  };
  const onContext = (e: Event): void => { if (opts.enabled()) e.preventDefault(); };
  const onKey = (down: boolean) => (e: KeyboardEvent): void => {
    if (!opts.enabled()) return;
    const d = keyPolicy(e, { sendTab: opts.sendTab?.() ?? false });
    if (d.prevent) e.preventDefault();
    if (!d.forward) return;
    cur(down ? 'keydown' : 'keyup');
    if (down) tracker.keyDown(e.code); else tracker.keyUp(e.code);
  };
  const release = (): void => { cur('blur'); tracker.releaseAll(); };
  let userMovedAt = -Infinity;
  let pressed: Node | null = null;
  const notePress = (e: Event): void => { pressed = e.target as Node | null; userMovedAt = performance.now(); };
  const noteTab = (e: KeyboardEvent): void => { if (e.code === 'Tab') { pressed = null; userMovedAt = performance.now(); } };
  const isTextEntry = (a: Element | null): boolean => { const t = (a as HTMLElement | null)?.tagName; return t === 'INPUT' || t === 'TEXTAREA' || t === 'SELECT' || (a as HTMLElement | null)?.isContentEditable === true; };
  const onDesktopFocusOut = (e: FocusEvent): void => {
    const to = e.relatedTarget as Element | null;
    if (to && isTextEntry(to)) return;
    const recent = performance.now() - userMovedAt < 1000;
    const byUser = recent && (pressed === null || (to ? pressed === to || to.contains(pressed) || (pressed.contains?.(to) ?? false) : !el.contains(pressed)));
    if (byUser) return;
    setTimeout(() => { const a = document.activeElement; if (opts.enabled() && a !== el && (to ? a === to : a === null || a === document.body)) el.focus({ preventScroll: true }); }, 0);
  };
  const onVisibility = (): void => { if (document.visibilityState === 'hidden') release(); };
  const stop = (e: Event): void => { if (opts.enabled()) e.preventDefault(); };

  // the DOM events that carry a user's input: their own timeStamp and the instant their listener starts go with every RawInput they cause
  const stamped = new Set(['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'wheel', 'keydown', 'keyup']);
  const withMeta = (type: string, fn: AnyListener): AnyListener => (stamped.has(type)
    ? ((e: Event): void => {
      meta = { tEvent: performance.timeOrigin + e.timeStamp, tListener: performance.timeOrigin + performance.now() };
      try { (fn as (e: Event) => void)(e); } finally { meta = undefined; }
    }) as unknown as AnyListener
    : fn);
  const el_: [string, AnyListener, AddEventListenerOptions?][] = [
    ['pointerdown', onDown as unknown as AnyListener], ['pointermove', onPointer('pointermove') as unknown as AnyListener], ['pointerup', onUp as unknown as AnyListener],
    ['pointercancel', onPointer('pointercancel') as unknown as AnyListener], ['wheel', onWheel as unknown as AnyListener, { passive: false }], ['contextmenu', onContext as unknown as AnyListener],
    ['auxclick', stop as unknown as AnyListener], ['dragstart', stop as unknown as AnyListener], ['selectstart', stop as unknown as AnyListener],
    ['keydown', onKey(true) as unknown as AnyListener], ['keyup', onKey(false) as unknown as AnyListener], ['blur', release as unknown as AnyListener],
  ];
  const added = el_.map(([t, f, o]): [string, AnyListener, AddEventListenerOptions | undefined] => [t, withMeta(t, f), o]);
  for (const [t, f, o] of added) el.addEventListener(t, f as EventListener, o);
  window.addEventListener('blur', release);
  el.addEventListener('focusout', onDesktopFocusOut);
  document.addEventListener('pointerdown', notePress, true);
  document.addEventListener('keydown', noteTab, true);
  document.addEventListener('visibilitychange', onVisibility);
  return {
    tracker,
    detach() {
      for (const [t, f, o] of added) el.removeEventListener(t, f as EventListener, o);
      window.removeEventListener('blur', release);
      el.removeEventListener('focusout', onDesktopFocusOut);
      document.removeEventListener('pointerdown', notePress, true);
      document.removeEventListener('keydown', noteTab, true);
      document.removeEventListener('visibilitychange', onVisibility);
    },
  };
}
