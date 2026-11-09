// JS side of the JSPI thread switch (docs/decisions/0124-wasm-thread-switching.md, PROVISIONAL). An emscripten JS library: link with
//   -sJSPI -sSUPPORT_LONGJMP=wasm --js-library lift/harness/web/threads/fle_thread_lib.js     (and compile lift/runtime/fle_thread*.c with -DFLE_THREAD_WASM)
// The same file serves node and the browser (it is part of the emscripten glue, not a separate script). It contains NO scheduling policy: runtime/fle_thread.c decides which
// thread runs next (decision 0122) and passes the index; this file only moves the processor:
//   * every guest thread other than the first is one WebAssembly.promising call of the export fle_wasm_thread_entry (its own JSPI stack); the first thread is `main`;
//   * fle_wasm_thread_switch(from, to, spNow, spNew) is an ASYNC import (WebAssembly.Suspending): it records `from`'s __stack_pointer, returns a Promise that is resolved when some
//     thread switches back to `from`, and QUEUES (microtask) the hand-over to `to`: the hand-over never runs inside the import call, so the caller's stack is suspended first;
//   * the hand-over to a thread that has not started sets __stack_pointer to the top of that thread's shadow stack region and starts promising(entry)(to); to a suspended thread it
//     sets __stack_pointer to the value saved at that thread's last switch away and resolves that thread's Promise;
//   * a finished thread calls fle_wasm_thread_exit(to, spNew): a JS throw that unwinds its JSPI stack (every wasm frame, also inside a guest setjmp landing) and rejects its promise,
//     whose handler then does the hand-over to `to`. Nothing else is stored per finished thread.
// A trap, RangeError (JSPI stack) or other exception in a thread, and an exit() inside a non-first thread, are reported with the thread index (Module.fleThreadFailed / console.error).
addToLibrary({
  $FleThreads__deps: ['$stackRestore'],
  $FleThreads: {
    sp: [],            // __stack_pointer saved at the last switch away from thread i
    resume: [],        // the resolver of the Promise a suspended thread i waits on
    started: [],
    entry: null,       // WebAssembly.promising(fle_wasm_thread_entry)
    switches: 0,
    starts: 0,
    ends: 0,
    failed: null,
    handover(to, spNew) {
      const S = FleThreads;
      if (S.failed) return;
      if (!S.started[to]) {
        S.started[to] = true;
        S.starts++;
        stackRestore(spNew);
        S.entry(to).then(
          () => S.fail(to, new Error('fle_wasm_thread_entry returned (a thread body never returns)')),
          (e) => S.ended(to, e));
      } else {
        const r = S.resume[to];
        S.resume[to] = null;
        if (!r) { S.fail(to, new Error('hand-over to thread ' + to + ', which is not suspended in a switch')); return; }
        stackRestore(S.sp[to]);   // the shadow stack of `to`, set right before its Promise is resolved (the toy of the design memo does the same)
        r();
      }
    },
    ended(idx, e) {
      const S = FleThreads;
      if (e && e.fleThreadExit) { S.ends++; S.handover(e.to, e.sp); return; }
      S.fail(idx, e);
    },
    fail(idx, e) {
      const S = FleThreads;
      // exit() inside a thread: emscripten's quit_ has run (Module.quit set the exit code); nothing more to report
      if (e && (e.name === 'ExitStatus')) { S.failed = S.failed || { idx, exit: e.status }; return; }
      if (!S.failed) S.failed = { idx, error: e };
      const msg = 'fle thread ' + idx + ' failed: ' + (e && e.stack ? e.stack.split('\n').slice(0, 3).join(' | ') : String(e));
      if (typeof Module.fleThreadFailed === 'function') Module.fleThreadFailed(idx, e, msg);
      else { err(msg); if (typeof process !== 'undefined' && process.exitCode === undefined) process.exitCode = 99; }
    },
  },
  fle_wasm_thread_init__deps: ['$FleThreads'],
  fle_wasm_thread_init__sig: 'v',
  fle_wasm_thread_init: () => {
    const S = FleThreads;
    if (S.entry) return;
    S.entry = WebAssembly.promising(wasmExports['fle_wasm_thread_entry']);
    S.started[0] = true;
    Module['fleThreadStats'] = () => ({ switches: S.switches, starts: S.starts, ends: S.ends, failed: S.failed && { idx: S.failed.idx, exit: S.failed.exit, error: S.failed.error && String(S.failed.error) } });
  },
  fle_wasm_thread_switch__deps: ['$FleThreads', '$stackRestore'],
  fle_wasm_thread_switch__sig: 'viiii',
  fle_wasm_thread_switch__async: true,
  fle_wasm_thread_switch: (from, to, spNow, spNew) => {
    const S = FleThreads;
    S.sp[from] = spNow;
    S.switches++;
    return new Promise((resolve) => {
      S.resume[from] = resolve;
      queueMicrotask(() => S.handover(to, spNew));
    });
  },
  fle_wasm_thread_slot_new__deps: ['$FleThreads'],
  fle_wasm_thread_slot_new__sig: 'vi',
  fle_wasm_thread_slot_new: (idx) => {
    // decision 0196: the place idx of the thread table is given to a NEW thread (the one that ended there is gone: its Promise was rejected and its hand-over done before any C code could create a thread):
    // the first hand-over to idx starts it. Without this, `started[idx]` stays true and the hand-over fails with "not suspended in a switch".
    const S = FleThreads;
    S.started[idx] = false;
    S.resume[idx] = null;
  },
  fle_wasm_thread_exit__deps: ['$FleThreads'],
  fle_wasm_thread_exit__sig: 'vii',
  fle_wasm_thread_exit: (to, spNew) => {
    throw { fleThreadExit: true, to, sp: spNew };
  },
});
