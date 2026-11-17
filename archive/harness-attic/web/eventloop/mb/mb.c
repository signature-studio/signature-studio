// M-B prototype: a C "message loop" whose GetMessage is a JS Promise. One source, three builds (build.sh):
//   -DMODE_SYNC   : host_get_message is an ordinary (synchronous) import that returns at once  -> size/speed baseline, no suspension
//   -sASYNCIFY    : host_get_message is an EM_ASYNC_JS import, the stack is unwound/rewound by the Binaryen Asyncify pass
//   -sJSPI        : the same import, suspended with WebAssembly.Suspending / promising (no code transformation)
// deep_*: a call chain of `depth` frames between the exported entry and the import (the engine's GetMessage is called from deep inside
// WndProc -> DispatchMessage -> ... frames). PAD_BYTES of memory locals per frame; WIDE=1 adds 24 scalar locals that are live across the call.
#include <emscripten.h>
#include <stdint.h>
#include <string.h>

typedef struct { int32_t type, a, b, pad; double t; } MSG;
enum { MSG_QUIT = 9, MSG_PING = 1 };

#ifndef PAD_BYTES
#define PAD_BYTES 256
#endif

#ifdef MODE_SYNC
EM_JS(int, host_get_message, (MSG *m), { return Module.syncGet(m); });
EM_JS(int, host_yield, (void), { return 1; });
EM_JS(int, host_yield_task, (void), { return 1; });
#else
EM_ASYNC_JS(int, host_get_message, (MSG *m), {
  return await Module.asyncGet(m);
});
EM_ASYNC_JS(int, host_yield, (void), {
  await Promise.resolve();
  return 1;
});
EM_ASYNC_JS(int, host_yield_task, (void), {
  await new Promise((r) => { const c = Module.mc; c.port2.onmessage = () => r(); c.port1.postMessage(0); });
  return 1;
});
#endif
EM_JS(void, host_reply, (int seq), { postMessage(seq); });
EM_JS(int, host_nop, (void), { return 0; });

static volatile int sink;

__attribute__((noinline)) static int deep_get(int depth, MSG *m) {
  volatile char pad[PAD_BYTES];
  pad[depth % PAD_BYTES] = (char)depth;
  if (depth == 0) return host_get_message(m);
  int r = deep_get(depth - 1, m);
  return r + pad[depth % PAD_BYTES];
}
// the same chain without MSG argument for the yield benchmarks
__attribute__((noinline)) static int deep_yield(int depth, int kind) {
  volatile char pad[PAD_BYTES];
  pad[depth % PAD_BYTES] = (char)depth;
  if (depth == 0) return kind == 0 ? host_nop() : kind == 1 ? host_yield() : host_yield_task();
  int r = deep_yield(depth - 1, kind);
  return r + pad[depth % PAD_BYTES];
}
// WIDE variant: 24 scalar locals live across the call (what Asyncify has to save per frame)
__attribute__((noinline)) static int64_t deep_yield_wide(int depth, int kind, int64_t s) {
  int64_t a0 = s + 1, a1 = s * 3, a2 = s ^ 5, a3 = s + 7, a4 = s * 11, a5 = s ^ 13, a6 = s + 17, a7 = s * 19;
  int64_t a8 = s + 23, a9 = s * 29, a10 = s ^ 31, a11 = s + 37, a12 = s * 41, a13 = s ^ 43, a14 = s + 47, a15 = s * 53;
  int64_t a16 = s + 59, a17 = s * 61, a18 = s ^ 67, a19 = s + 71, a20 = s * 73, a21 = s ^ 79, a22 = s + 83, a23 = s * 89;
  if (depth == 0) return (kind == 0 ? host_nop() : kind == 1 ? host_yield() : host_yield_task()) + s;
  int64_t r = deep_yield_wide(depth - 1, kind, s + 1);
  sink = (int)(a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7);
  return r + a8 + a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19 + a20 + a21 + a22 + a23;
}

// GetMessage loop: take a message `depth` frames down, answer PING with host_reply(seq), until QUIT
EMSCRIPTEN_KEEPALIVE int loop_echo(int depth) {
  MSG m;
  int n = 0;
  for (;;) {
    if (!deep_get(depth, &m)) break;
    if (m.type == MSG_QUIT) break;
    if (m.type == MSG_PING) { host_reply(m.a); n++; }
  }
  return n;
}

// n suspensions (kind 1: microtask, kind 2: macrotask) or n plain calls (kind 0) through a chain of `depth` frames
EMSCRIPTEN_KEEPALIVE int bench_yield(int depth, int kind, int n, int wide) {
  int64_t acc = 0;
  for (int i = 0; i < n; i++) acc += wide ? deep_yield_wide(depth, kind, i) : deep_yield(depth, kind);
  sink = (int)acc;
  return (int)(acc & 0x7fffffff);
}

// how deep can the native (V8) call stack go: plain call, JSPI promising export, Asyncify-instrumented
__attribute__((noinline, optnone)) static int rec_depth_(int n) {
  volatile int x = n;
  if (n == 0) return 0;
  return rec_depth_(n - 1) + (x & 1);
}
EMSCRIPTEN_KEEPALIVE int rec_depth(int n) { return rec_depth_(n); }

int main(void) { return 0; }
