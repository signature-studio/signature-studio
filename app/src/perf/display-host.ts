// The browser's side of the display meter (decision 0192): requestAnimationFrame, visibilitychange and PerformanceObserver handed to DisplayMonitor (display.ts), which never touches a browser global itself.
import type { DisplayHost, PerfEntryLike } from './display.ts';

export function browserDisplayHost(): DisplayHost {
  return {
    now: () => performance.timeOrigin + performance.now(),
    timeOrigin: () => performance.timeOrigin,
    requestFrame: (cb) => { requestAnimationFrame(cb); },
    setTimer: (cb, ms) => setTimeout(cb, ms),
    clearTimer: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
    isVisible: () => document.visibilityState !== 'hidden',
    onVisibility: (cb) => { document.addEventListener('visibilitychange', () => cb(document.visibilityState !== 'hidden')); },
    observe: (type, options, cb) => {
      try {
        if (typeof PerformanceObserver === 'undefined' || !(PerformanceObserver.supportedEntryTypes ?? []).includes(type)) return false;
        new PerformanceObserver((list) => cb(list.getEntries() as unknown as PerfEntryLike[])).observe({ type, ...options } as PerformanceObserverInit);
        return true;
      } catch { return false; }
    },
  };
}
