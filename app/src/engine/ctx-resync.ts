// A 2D canvas can lose its context (the GPU process crashed or was reset, Chrome's `contextlost` / `contextrestored` events; decision 0201). The canvas is then drawn on no longer, and when the context comes back it is
// EMPTY: what the page showed is gone, and nothing but the engine's own next write would bring it back. The engine's idle redraw writes only a 1-pixel strip of the main window, so even then the rest
// stayed blank. So both events ask for a RESYNC: the Worker forgets what each sink was given (Presenter.forgetSeen), so that the next look sends every surface whole; with display method D the compositor reports the whole picture again
// (fle_desk_resync, decision 0223).

/** the part of HTMLCanvasElement / OffscreenCanvas that is used */
export interface ContextLossTarget { addEventListener(type: 'contextlost' | 'contextrestored', listener: (ev: Event) => void): void }

export interface ContextLossHooks {
  /** the context was lost; the canvas is drawn on no longer until `restored` */
  lost(): void;
  /** the context is back, with an empty canvas */
  restored(): void;
}

/**
 * The HTML rule (as with WebGL): the page cancels `contextlost` to allow the browser to restore the context, so the listener always does. Nothing else is done to the canvas here.
 * Measured in Chrome 156 (headless, this machine, decision 0201): `Browser.crashGpuProcess` fires `contextlost` on the page's canvases and on an OffscreenCanvas in a Worker; `contextrestored` did not follow
 * (about 19 s watched, with the event cancelled). Whether Chrome restores only a cancelled event was not tested.
 */
export function watchContextLoss(target: ContextLossTarget, hooks: ContextLossHooks): void {
  target.addEventListener('contextlost', (ev) => { ev.preventDefault(); hooks.lost(); });
  target.addEventListener('contextrestored', () => hooks.restored());
}
