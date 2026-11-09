/* JSPI measurement hooks (docs/investigations/jspi-real-wasm.md). A measurement aid: not part of the engine, the runtime or the lifted
 * code. build_nmod_wasm.sh compiles and links it only with JSPI=1.
 *
 * It sits in front of (linker --wrap) the runtime's implementations of the Win32 calls that can block on a real Windows and are called
 * by the InitUnits run (EnterCriticalSection, WaitForSingleObject, AcquireSRWLockExclusive), of three that do I/O (LoadLibraryW,
 * ReadFile, CreateFileW), and of the function-entry hook of trace.c (-finstrument-functions). At a call the configuration selects it
 * (globalThis.JSPI_CFG, set by the driver) either records the state (guest return address, guest rsp, live setjmp landings, depth of
 * fle_call_guest) or also suspends the wasm in a JS import made by EM_ASYNC_JS (a WebAssembly.Suspending function under -sJSPI) for
 * one microtask / one setTimeout(0) / one MessageChannel round trip, and then calls the real function. Neither the guest registers
 * nor the guest memory nor the real function's arguments are touched; nothing is printed, so the stdout of the engine is unchanged.
 *
 * Configuration keys (numbers): mode (0 record only, 1 microtask, 2 setTimeout(0), 3 MessageChannel round trip, 4 a synchronous JS
 * import call as a control), probe (1: count the stack frames the engine reports at the point), every_<k>, from_<k>, to_<k>
 * (kind k is selected at its ordinals from..to, every `every`-th; every_<k> = 0 or absent: not selected), exit_after (exit(0) after
 * that many completed suspensions). Kinds: 0 EnterCriticalSection, 1 WaitForSingleObject, 2 AcquireSRWLockExclusive, 3 lifted
 * function entry, 4 LoadLibraryW, 5 ReadFile, 6 CreateFileW.
 * Results: globalThis.JSPI_REC = {n, f: Float64Array(n * 12)}, globalThis.JSPI_COUNTS (calls per kind at exit). */
#include <emscripten.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "a4.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"

enum { K_ENTERCS, K_WAITSO, K_ACQSRW, K_ENTRY, K_LOADLIB, K_READFILE, K_CREATEFILE, K_N };

EM_JS(double, jspi_cfg_num, (const char *key, double dflt), {
    const k = UTF8ToString(key);
    const c = globalThis.JSPI_CFG;
    return (c && Object.prototype.hasOwnProperty.call(c, k)) ? Number(c[k]) : dflt;
});

EM_JS(void, jspi_rec, (int kind, double n, double ret, double sp, int nland, int cbdepth, double dt_total, double dt_js, int mode), {
    let r = globalThis.JSPI_REC;
    if (!r) r = globalThis.JSPI_REC = { n: 0, f: new Float64Array(12 * 4096) };
    if ((r.n + 1) * 12 > r.f.length) { const g = new Float64Array(r.f.length * 2); g.set(r.f); r.f = g; }
    const p = globalThis.JSPI_PROBE; globalThis.JSPI_PROBE = null;
    const o = r.n * 12;
    r.f[o] = kind; r.f[o + 1] = n; r.f[o + 2] = ret; r.f[o + 3] = sp; r.f[o + 4] = nland; r.f[o + 5] = cbdepth;
    r.f[o + 6] = dt_total; r.f[o + 7] = dt_js; r.f[o + 8] = p ? p[0] : -1; r.f[o + 9] = p ? p[1] : -1; r.f[o + 10] = mode; r.f[o + 11] = r.n;
    r.n++;
});

EM_JS(void, jspi_counts, (int kind, double n), {
    const c = globalThis.JSPI_COUNTS || (globalThis.JSPI_COUNTS = []);
    c[kind] = n;
});

/* the frames of the engine's stack at this point (wasm frames and JS frames), taken without suspending */
EM_JS(void, jspi_probe, (void), {
    const old = Error.stackTraceLimit; Error.stackTraceLimit = 1000000;
    const lines = new Error().stack.split('\n').slice(1);
    Error.stackTraceLimit = old;
    let js = 0;
    for (const l of lines) if (l.indexOf('wasm-function') < 0) js++;
    globalThis.JSPI_PROBE = [lines.length, js];
    if (lines.length > (globalThis.JSPI_DEEPEST_N || 0)) { globalThis.JSPI_DEEPEST_N = lines.length; globalThis.JSPI_DEEPEST = lines.join('\n'); }
});

EM_JS(double, jspi_nop, (int mode), {
    return 0;
});

EM_ASYNC_JS(double, jspi_suspend, (int mode), {
    const t0 = performance.now();
    if (mode === 1) {
        await Promise.resolve();
    } else if (mode === 2) {
        await new Promise((r) => setTimeout(r, 0));
    } else if (mode === 3) {
        let mc = globalThis.JSPI_MC;
        if (!mc) {
            const ch = new MessageChannel();
            const q = [];
            ch.port1.onmessage = () => { if (ch.port1.unref) ch.port1.unref(); q.shift()(); };
            mc = globalThis.JSPI_MC = { p1: ch.port1, p2: ch.port2, q };
        }
        await new Promise((r) => { mc.q.push(r); if (mc.p1.ref) mc.p1.ref(); mc.p2.postMessage(0); });
    }
    return performance.now() - t0;
});

static struct {
    int ready;
    int mode, probe;
    uint32_t exit_after;
    uint64_t every[K_N], from[K_N], to[K_N];
    uint64_t calls[K_N];
    uint64_t done;
} g;

static void report_counts(void) {
    for (int k = 0; k < K_N; k++) jspi_counts(k, (double)g.calls[k]);
}

static void init_cfg(void) {
    static const char *const ek[K_N] = {"every_0", "every_1", "every_2", "every_3", "every_4", "every_5", "every_6"};
    static const char *const fk[K_N] = {"from_0", "from_1", "from_2", "from_3", "from_4", "from_5", "from_6"};
    static const char *const tk[K_N] = {"to_0", "to_1", "to_2", "to_3", "to_4", "to_5", "to_6"};
    g.ready = 1;
    g.mode = (int)jspi_cfg_num("mode", 0);
    g.probe = (int)jspi_cfg_num("probe", 0);
    g.exit_after = (uint32_t)jspi_cfg_num("exit_after", 0);
    for (int k = 0; k < K_N; k++) {
        g.every[k] = (uint64_t)jspi_cfg_num(ek[k], 0);
        g.from[k] = (uint64_t)jspi_cfg_num(fk[k], 1);
        g.to[k] = (uint64_t)jspi_cfg_num(tk[k], 1e18);
    }
    atexit(report_counts);
}

__attribute__((no_instrument_function))
static void point(int kind) {
    uint64_t n;
    int nland = 0, mode;
    uint64_t ret, sp;
    double t0, t1, js = 0;
    if (!g.ready) init_cfg();
    n = ++g.calls[kind];
    if (!g.every[kind] || n < g.from[kind] || n > g.to[kind] || (n - g.from[kind]) % g.every[kind]) return;
    mode = g.mode;
    sp = R[4];
    ret = fle_ld(sp, 8);
    for (const fle_landing *l = fle_landing_top; l; l = l->prev) nland++;
    if (mode == 0) {
        if (g.probe) jspi_probe();
        jspi_rec(kind, (double)n, (double)ret, (double)sp, nland, (int)fle_callback_depth(), 0.0, 0.0, mode);
        return;
    }
    if (g.probe) jspi_probe();
    t0 = emscripten_get_now();
    if (mode == 4) js = jspi_nop(0);
    else js = jspi_suspend(mode);
    t1 = emscripten_get_now();
    jspi_rec(kind, (double)n, (double)ret, (double)sp, nland, (int)fle_callback_depth(), t1 - t0, js, mode);
    if (g.exit_after && ++g.done >= g.exit_after) exit(0);
}

#define WRAP(name, kind)                                                                                                            \
    void __real_##name(void);                                                                                                       \
    void __wrap_##name(void) {                                                                                                      \
        point(kind);                                                                                                                \
        __real_##name();                                                                                                            \
    }

WRAP(fle_imp_EnterCriticalSection, K_ENTERCS)
WRAP(fle_imp_WaitForSingleObject, K_WAITSO)
WRAP(fle_imp_AcquireSRWLockExclusive, K_ACQSRW)
WRAP(fle_imp_LoadLibraryW, K_LOADLIB)
WRAP(fle_imp_ReadFile, K_READFILE)
WRAP(fle_imp_CreateFileW, K_CREATEFILE)

/* The function-entry hook (-finstrument-functions) of trace.c, wrapped. A lifted function entry is the one that trace.c counted. */
void __real___cyg_profile_func_enter(void *fn, void *call_site);

__attribute__((no_instrument_function))
void __wrap___cyg_profile_func_enter(void *fn, void *call_site) {
    static uint64_t last;
    uint64_t c;
    __real___cyg_profile_func_enter(fn, call_site);
    c = trace_call_count();
    if (c == last) return;
    last = c;
    if (!g.ready) init_cfg();
    if (!g.every[K_ENTRY]) {
        g.calls[K_ENTRY] = c;
        return;
    }
    g.calls[K_ENTRY] = c - 1;
    point(K_ENTRY);
}
