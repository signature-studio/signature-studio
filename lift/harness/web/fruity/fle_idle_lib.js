// JS side of the browser build's idle (decision 0136, PROVISIONAL; REPLACES the content of this file of decision 0128 (the import now takes the virtual clock and the next deadline and returns a double); harness/fruity_idle.c). An emscripten JS library: link with -sJSPI --js-library <this file>
// (build_nmod_wasm.sh FRUITY=1 does). fle_idle_wait(nowTicks, nextTicks) is an ASYNC import (WebAssembly.Suspending): it tells the host (Module.fleIdle(idleNo, nowTicks, nextTicks) - the Worker posts it to the page),
// then returns a Promise that the host resolves through Module.fleIdleDeliver(code) (< 0: end the session; 0: input events were queued / the clock was moved; > 0: move the virtual clock to that many ticks). While the
// Promise is pending the JSPI stack of the engine's running thread is suspended and the Worker's event loop runs (messages from the page, timers): that is the return of control to the browser's event loop.
// The library contains no policy.
addToLibrary({
  $FleIdle: { resolve: null, count: 0 },
  fle_idle_wait__deps: ['$FleIdle'],
  fle_idle_wait__sig: 'ddd',
  fle_idle_wait__async: true,
  // decision 0280 (PROVISIONAL): the page's input ring, read by the scheduler (harness/fruity_idle.c fle_input_ring_poll). Plain synchronous imports; the Worker sets Module.fleInputPoll(ptr, max) -> count and Module.fleInputDone(ptr, n).
  fle_js_input_poll__sig: 'iii',
  fle_js_input_poll: (buf, max) => { const f = Module['fleInputPoll']; return f ? f(buf, max) : 0; },
  fle_js_input_done__sig: 'vii',
  fle_js_input_done: (buf, n) => { const f = Module['fleInputDone']; if (f) f(buf, n); },
  fle_idle_wait: (nowTicks, nextTicks) => {
    const S = FleIdle;
    S.count++;
    Module['fleIdleDeliver'] = (code) => { const r = S.resolve; S.resolve = null; if (r) { r(code); return true; } return false; };
    Module['fleIdleWaiting'] = () => S.resolve !== null;
    return new Promise((resolve) => {
      S.resolve = resolve;
      if (typeof Module['fleIdle'] === 'function') Module['fleIdle'](S.count, nowTicks, nextTicks);
    });
  },
});
