/* Stage 1 of CreateFruityInstance (see fruity.h and docs/investigations/create-fruity-instance-harness.md). Harness only, native only.
 * Linked into the harness only by a build with FRUITY=1 (lift/harness/two/build_two_native.sh), together with
 * -Wl,--wrap=__cyg_profile_func_enter, which puts __wrap___cyg_profile_func_enter below in front of the trace hook of
 * harness/wasm/trace_native.c: that file, the runtime and the generated C are not changed.
 *
 * What it does, in order:
 *   fruity_precheck   (before the guest runs)   reads FLE_CALL_FRUITY / FLE_FRUITY_*, checks the three stub rows of the dispatch table;
 *   fruity_run        (after DllMain returned)  finds CreateFruityInstance in the PE export table of the mapped FLEngine image, builds the
 *                     fake host object, arms the limits and the stop report, calls fle_call_guest(va, 0xC0, 0xEDE95A, host, &instance),
 *                     prints "fruity: rax=... instance=...".
 * It never answers an API for the guest. A stop of the guest (abort, fault, limit) ends the process with a "fruity:" report; the report
 * of harness/wasm/trace_native.c (trace_report) is not used for it because that function prints once only (it already printed the
 * DllMain report) and would print the a4 lines a second time. Every line of this file's output starts with "fruity: ". */
#define _GNU_SOURCE
#include <errno.h>
#include "fle_test_env.h"
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#ifdef __EMSCRIPTEN__
/* Decision 0128 (PROVISIONAL): the wasm build of the FRUITY-on harness (build_nmod_wasm.sh FRUITY=1). What is native-only here (signals, alarm, process_vm_readv, backtrace) has a wasm form below under this
 * macro: a runtime abort reaches report() through trace_abort_hook (harness/wasm/trace_fruity.c), the wall limit is tested every 2^20 lifted entries in the entry hook, a guest address is read after a bounds
 * check against the linear memory. A native build does not see any of it (this file's native object is unchanged by the edit). */
#include <emscripten.h>
#else
#include <sys/uio.h>
#ifdef __GLIBC__
#include <execinfo.h>
#endif
#endif

#include "a4.h"
#include "boot.h"
#include "census/census.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_runtime.h"
#include "fle_thread.h" /* decision 0122: the thread table, the queues and the virtual clock in the stop report (weak: a link without fle_thread.c prints nothing) */
#include "fle_gdi.h"   /* decision 0129: the font selected into a DC and its LOGFONTW, for the record of the first text-drawing call */
#include "fle_sys.h"   /* decision 0129: the virtual clock (ms) for the stable-idle report */
void fle_win_process_ended(void) __attribute__((weak));
uint32_t fle_win_visible_toplevel_count(void) __attribute__((weak));   /* decision 0222: weak, like the other runtime parts this harness may be linked without */
#ifdef FLE_IDLE
#include "idle/idle.h" /* decision 0129: the stable-idle detector (only a build with IDLE=1 compiles it) */
#endif

#define FRUITY_HOST_IMPL
#include "fruity_host.h"
#include "fruity.h"

#define ULL(x) ((unsigned long long)(x))
#define MAX_IMAGES 16
#define RING 48
#define CACHE_SLOTS (1u << 18)
#define MAX_STOP_NAMES 16
#define DEFAULT_MAX_CALLS UINT64_C(20000000)
#define DEFAULT_SECONDS 60u

/* ---- configuration (fruity_precheck) ---- */
static int cfg_on;
static uint64_t cfg_max_calls = DEFAULT_MAX_CALLS;
static uint64_t cfg_seconds = DEFAULT_SECONDS;
static int cfg_trap;
static char stop_buf[512];
static const char *stop_names[MAX_STOP_NAMES];
static int n_stop;

/* ---- state of the call ---- */
static volatile int active;
static volatile sig_atomic_t in_report;
static struct {
    uint64_t base, size;
} imgs[MAX_IMAGES];
static int nimgs;
static fruity_host host;
static uint64_t instance_slot;
static uint64_t call_va, calls_at_start, distinct_at_start;
static size_t depth_at_start;
static struct timespec t_start;
#ifdef __EMSCRIPTEN__
static uint64_t wall_tick;
/* Decision 0128: a sign of life for the browser's page (Module.fleProgress is set by the Worker; absent under node): every 2^22 lifted entries since the call began. 32 calls in the measured run. */
EM_JS(void, fle_js_progress, (double entries), { if (typeof Module !== 'undefined' && typeof Module.fleProgress === 'function') Module.fleProgress(entries); });
static void on_wall_limit(void);
void fruity_surface_stop_event(void);
void fruity_surface_first_event_check(void);
#endif

int fruity_requested(void) { return cfg_on; }

void fruity_add_image(uint64_t base, uint64_t size) {
    if (nimgs < MAX_IMAGES) {
        imgs[nimgs].base = base;
        imgs[nimgs].size = size;
        nimgs++;
    }
}

static int in_images(uint64_t va, uint64_t n) {
    int i;
    for (i = 0; i < nimgs; i++) {
        if (va >= imgs[i].base && va - imgs[i].base <= imgs[i].size && n <= imgs[i].size - (va - imgs[i].base)) return 1;
    }
    return 0;
}

static void (*find_fn(uint64_t va))(void) {
    uint64_t lo = 0, hi = fle_dispatch_count;
    while (lo < hi) {
        uint64_t mid = lo + (hi - lo) / 2;
        if (fle_dispatch_table[mid].va < va) lo = mid + 1;
        else hi = mid;
    }
    if (lo < fle_dispatch_count && fle_dispatch_table[lo].va == va) return fle_dispatch_table[lo].fn;
    return NULL;
}

/* ---- reading guest memory without risking a fault (the report runs at a stop; a guest pointer can be anything) ---- */
#ifdef __EMSCRIPTEN__
static int peek(uint64_t addr, void *buf, size_t n) {
    uint64_t size = (uint64_t)__builtin_wasm_memory_size(0) * 65536u;
    if (addr == 0 || addr >= size || n > size - addr) return 0;
    memcpy(buf, (const void *)(uintptr_t)addr, n);
    return 1;
}
#else
static int peek(uint64_t addr, void *buf, size_t n) {
    struct iovec l, r;
    l.iov_base = buf;
    l.iov_len = n;
    r.iov_base = (void *)(uintptr_t)addr;
    r.iov_len = n;
    return process_vm_readv(getpid(), &l, 1, &r, 1, 0) == (ssize_t)n;
}
#endif

static size_t peek_some(uint64_t addr, uint8_t *buf, size_t n) {
    if (peek(addr, buf, n)) return n;
    if (n > 16 && peek(addr, buf, 16)) return 16;
    if (n > 8 && peek(addr, buf, 8)) return 8;
    return 0;
}

/* ---- the PE export table of the mapped FLEngine image (read from guest memory; image.c is not touched) ---- */
static uint64_t export_va(uint64_t base, uint64_t size, const char *want, uint32_t *ordinal) {
    uint64_t pe, dd, dir, exp_rva, exp_size, afunc, anames, aord, i, nn, nf, wl = strlen(want);
    uint32_t ord_base;
    if (size < 0x1000 || fle_ld(base, 2) != 0x5A4D) return 0;
    pe = fle_ld(base + 0x3c, 4);
    if (pe + 0x108 > size) return 0;
    pe += base;
    if (fle_ld(pe, 4) != 0x00004550 || fle_ld(pe + 24, 2) != 0x20b) return 0;
    dd = pe + 24 + 112;
    exp_rva = fle_ld(dd, 4);
    exp_size = fle_ld(dd + 4, 4);
    if (!exp_rva || exp_rva + 40 > size) return 0;
    dir = base + exp_rva;
    ord_base = (uint32_t)fle_ld(dir + 16, 4);
    nf = fle_ld(dir + 20, 4);
    nn = fle_ld(dir + 24, 4);
    afunc = fle_ld(dir + 28, 4);
    anames = fle_ld(dir + 32, 4);
    aord = fle_ld(dir + 36, 4);
    if (nn > 0x10000 || afunc + 4 * nf > size || anames + 4 * nn > size || aord + 2 * nn > size) return 0;
    for (i = 0; i < nn; i++) {
        uint64_t nrva = fle_ld(base + anames + 4 * i, 4);
        uint64_t ord, frva;
        if (nrva + wl + 1 > size) continue;
        if (strncmp((const char *)(uintptr_t)(base + nrva), want, wl + 1) != 0) continue;
        ord = fle_ld(base + aord + 2 * i, 2);
        if (ord >= nf) return 0;
        frva = fle_ld(base + afunc + 4 * ord, 4);
        if (frva >= exp_rva && frva < exp_rva + exp_size) return 0; /* forwarder */
        if (ordinal) *ordinal = ord_base + (uint32_t)ord;
        return base + frva;
    }
    return 0;
}

/* ---- classification of lifted functions: is it a `jmp [IAT]` thunk, of which import (decided once per function) ---- */
typedef struct {
    void *fn;
    uint64_t va;
    const char *imp; /* "dll!name" of the import when the function is a lifted `jmp [rip+disp]` thunk of the image, else NULL */
    uint8_t stop;
} cinfo;

static cinfo cache[CACHE_SLOTS];
static cinfo spare;

static const char *thunk_import(uint64_t va) {
    const uint8_t *p;
    int32_t disp;
    if (!va || !in_images(va, 6)) return NULL;
    p = (const uint8_t *)(uintptr_t)va;
    if (p[0] != 0xFF || p[1] != 0x25) return NULL;
    memcpy(&disp, p + 2, sizeof disp);
    return image_import_name(va + 6 + (uint64_t)(int64_t)disp);
}

static int stop_match(const char *imp) {
    const char *bang = strchr(imp, '!');
    const char *api = bang ? bang + 1 : imp;
    int i;
    for (i = 0; i < n_stop; i++) {
        if (strcmp(stop_names[i], api) == 0 || strcmp(stop_names[i], imp) == 0) return 1;
    }
    return 0;
}

static void classify(void *fn, cinfo *c) {
    uint64_t i;
    c->fn = fn;
    c->va = 0;
    c->imp = NULL;
    c->stop = 0;
    for (i = 0; i < fle_dispatch_count; i++) {
        if ((void *)fle_dispatch_table[i].fn == fn) {
            c->va = fle_dispatch_table[i].va;
            break;
        }
    }
    c->imp = thunk_import(c->va);
    if (c->imp && n_stop) c->stop = (uint8_t)stop_match(c->imp);
}

static cinfo *cache_get(void *fn) {
    uint64_t k = (uint64_t)(uintptr_t)fn;
#ifdef __EMSCRIPTEN__
    size_t h = (size_t)((k * UINT64_C(0x9E3779B97F4A7C15)) >> 46); /* a function pointer is a small table index here: (k >> 4) would put 16 of them on one slot (harness/wasm/trace.c says the same of its hash) */
#else
    size_t h = (size_t)(((k >> 4) * UINT64_C(0x9E3779B97F4A7C15)) >> 46);
#endif
    size_t n;
    for (n = 0; n < CACHE_SLOTS; n++, h = (h + 1) & (CACHE_SLOTS - 1)) {
        if (cache[h].fn == fn) return &cache[h];
        if (!cache[h].fn) {
            classify(fn, &cache[h]);
            return &cache[h];
        }
    }
    classify(fn, &spare);
    return &spare;
}

/* ---- what the entry hook remembers: the last lifted entries, and the last entries into import thunks with their four register arguments ---- */
typedef struct {
    void *fn;
    uint64_t ret;
} erec;
typedef struct {
    const char *imp;
    uint64_t ret, a1, a2, a3, a4, seq;
} irec;
#define MAX_PEEK 16
static struct {
    uint64_t va;
    int reg;
    int has_off;       /* FLE_FRUITY_PEEK_AT=<va>:<reg>@<hex offset>: print the 64-bit value at [reg + offset] (and the byte at [reg + 0x10]) instead of a string */
    uint64_t off;
    unsigned shown;
} peek_at[MAX_PEEK];
static int n_peek_at;
static uint64_t peek_to;     /* FLE_FRUITY_PEEK_TO=<n>: ... and not after the n-th (0 = no end) */
static uint64_t peek_from;   /* FLE_FRUITY_PEEK_FROM=<n>: only entries at or after the n-th lifted entry since the call began are printed */
#ifdef FLE_G3_FORKPROBE
int fruity_import_log;   /* decision 0171, native probe build only: the g3 command `importlog on|off` prints every import thunk entered (name, return address, four register arguments) to stderr */
#endif
static erec ering[RING];
static irec iring[RING];
static uint64_t ering_n, iring_n;

static void report(const char *reason, int code);
static void finish(int code) __attribute__((noreturn));

static void stop_named(const cinfo *c) {
    char reason[300];
    snprintf(reason, sizeof reason,
             "named stop: the lifted import thunk f_%llx of %s was entered (FLE_FRUITY_STOP_AT); the runtime's answer for it has not run",
             ULL(c->va), c->imp ? c->imp : "?");
    report(reason, 6);
    finish(6);
}

void __real___cyg_profile_func_enter(void *fn, void *call_site);
static void read_utf16(uint64_t addr, char *out, size_t cap);
#ifdef FLE_IDLE
static int idle_tick_pending;
static void idle_stop(int st) __attribute__((noreturn));
#endif


/* clang's -finstrument-functions also instruments the static inline helpers of the runtime headers (fle_ld, fle_st, fle_dispatch, ...), so this
 * hook sees many functions that are not lifted code. A function is lifted code when it is a row of fle_dispatch_table (classify: va != 0), the
 * same test as is_lifted() of trace_native.c; only those are recorded here. The trace hook is called for every function either way. */
__attribute__((no_instrument_function)) void __wrap___cyg_profile_func_enter(void *fn, void *call_site) {
    cinfo *c = NULL;
    if (active) {
        cinfo *k = cache_get(fn);
        if (k->va) {
            uint64_t ret = fle_ld(R[4], 8);
            erec *e = &ering[ering_n % RING];
            c = k;
#ifdef __EMSCRIPTEN__
            if ((++wall_tick & 0xFFFFFu) == 0) {
                on_wall_limit();
                if ((wall_tick & 0x3FFFFFu) == 0) fle_js_progress((double)wall_tick);
            }
#endif
            e->fn = fn;
            e->ret = ret;
            ering_n++;
#ifdef FLE_IDLE
            idle_tick_pending = 1;
#endif
            if (n_peek_at) {
                int pk;

                for (pk = 0; pk < n_peek_at; pk++)
                    if (peek_at[pk].va == k->va && peek_at[pk].shown < 400 && ering_n >= peek_from && (peek_to == 0 || ering_n <= peek_to)) {
                        char str[200];
                        uint64_t v = R[peek_at[pk].reg];

                        peek_at[pk].shown++;
                        if (peek_at[pk].has_off) {
                            uint64_t q = 0, b10 = 0;
                            int ok1 = peek(v + peek_at[pk].off, &q, 8), ok2 = peek(v + 0x10, &b10, 1);

                            fprintf(stderr, "peek-at: f_%llx entry %llu reg R[%d]=%#llx [reg+%#llx]=%s%#llx [reg+0x10] byte=%s%#llx (return %#llx)\n", ULL(k->va), ULL(ering_n), peek_at[pk].reg, ULL(v), ULL(peek_at[pk].off),
                                    ok1 ? "" : "(unreadable)", ULL(q), ok2 ? "" : "(unreadable)", ULL(b10 & 0xff), ULL(ret));
                            continue;
                        }
                        read_utf16(v, str, sizeof str);
                        fprintf(stderr, "peek-at: f_%llx entry %llu reg R[%d]=%#llx UTF-16 \"%s\" (return %#llx)\n", ULL(k->va), ULL(ering_n), peek_at[pk].reg, ULL(v), str, ULL(ret));
                    }
            }
            if (c->imp) {
#ifdef FLE_G3_FORKPROBE
                if (fruity_import_log) fprintf(stderr, "implog: %s guest-return=%#llx args=%#llx %#llx %#llx %#llx\n", c->imp, ULL(ret), ULL(R[1]), ULL(R[2]), ULL(R[8]), ULL(R[9]));
#endif
                irec *r = &iring[iring_n % RING];
                r->imp = c->imp;
                r->ret = ret;
                r->a1 = R[1];
                r->a2 = R[2];
                r->a3 = R[8];
                r->a4 = R[9];
                r->seq = ering_n;
                iring_n++;
            }
        }
    }
    __real___cyg_profile_func_enter(fn, call_site);
    if (c && c->stop) stop_named(c);
#ifdef FLE_IDLE
    if (idle_tick_pending) {
        int st;

        idle_tick_pending = 0;
        st = idle_tick();
        if (st) idle_stop(st);
    }
#endif
}

#ifdef FLE_G3_FORKPROBE
/* decision 0222 (diagnostic): the return value of every import thunk while `importlog on` is set (the g3 line is printed on the exit of the thunk) */
extern void __real___cyg_profile_func_exit(void *fn, void *call_site);
__attribute__((no_instrument_function)) void __wrap___cyg_profile_func_exit(void *fn, void *call_site) {
    if (fruity_import_log && active) {
        cinfo *k = cache_get(fn);

        if (k->va && k->imp) fprintf(stderr, "implog-ret: %s = %#llx\n", k->imp, ULL(R[0]));
    }
    __real___cyg_profile_func_exit(fn, call_site);
}
#endif

/* ---- the stop report ---- */
static const char *func_name(uint64_t va, char *buf, size_t n) {
    const char *imp = thunk_import(va);
    const char *ex = image_export_name(va);
    if (imp) snprintf(buf, n, "f_%llx (import thunk of %s)", ULL(va), imp);
    else if (ex) snprintf(buf, n, "%s", ex);
    else snprintf(buf, n, "f_%llx", ULL(va));
    return buf;
}

static const char *kind_of(const char *m) {
    if (strstr(m, "not a known function entry")) return "fle_dispatch_miss";
    if (strstr(m, "unsupported instruction")) return "fle_unsupported";
    if (strstr(m, "undefined flag")) return "fle_rd (undefined flag read)";
    if (strstr(m, "shift count")) return "fle_bad_count";
    if (strncmp(m, "fle: ", 5) == 0) return "fle_stop (a runtime message that names an API and the reason)";
    return "(not a runtime message)";
}

static void read_utf16(uint64_t addr, char *out, size_t cap) {
    size_t n = 0;
    uint8_t w[2];
    while (n + 1 < cap && n < 259) {
        if (!peek(addr + 2 * n, w, 2)) {
            if (n == 0) snprintf(out, cap, "(unreadable)");
            else out[n] = 0;
            return;
        }
        if (w[0] == 0 && w[1] == 0) break;
        out[n++] = (w[1] == 0 && w[0] >= 0x20 && w[0] < 0x7f) ? (char)w[0] : '?';
    }
    out[n] = 0;
}

static void describe_value(const char *label, uint64_t v) {
    uint8_t b[64];
    size_t got = 0, i, m;
    char s[80];
    printf("fruity:   %s=%#llx", label, ULL(v));
    if (v >= 0x10000) got = peek_some(v, b, sizeof b);
    if (got) {
        printf("  bytes[%zu]=", got > 32 ? (size_t)32 : got);
        for (i = 0; i < got && i < 32; i++) printf("%02x", b[i]);
        m = 0;
        for (i = 0; i + 1 < got && m + 1 < sizeof s; i += 2) {
            if (b[i + 1] != 0 || b[i] < 0x20 || b[i] > 0x7e) break;
            s[m++] = (char)b[i];
        }
        s[m] = 0;
        if (m >= 3) printf("  as UTF-16LE: \"%s\"%s", s, i + 1 >= got ? "..." : "");
        m = 0;
        for (i = 0; i < got && m + 1 < sizeof s; i++) {
            if (b[i] < 0x20 || b[i] > 0x7e) break;
            s[m++] = (char)b[i];
        }
        s[m] = 0;
        if (m >= 4) printf("  as ANSI: \"%s\"%s", s, i >= got ? "..." : "");
    }
    printf("\n");
}

/* WinVerifyTrust(HWND, GUID *pgActionID, WINTRUST_DATA *pWVTData). The offsets are the x64 layout of the Microsoft Learn structures
 * WINTRUST_DATA and WINTRUST_FILE_INFO, worked out from the member types (a READING, not checked here); the raw bytes are printed too. */
static void decode_wvt(void) {
    uint8_t g[16], d[0x58], fi[0x20];
    static const uint8_t generic_v2[16] = {0x6b, 0xc5, 0xaa, 0x00, 0x44, 0xcd, 0xd0, 0x11, 0x8c, 0xc2, 0x00, 0xc0, 0x4f, 0xc2, 0x95, 0xee};
    uint64_t pfile;
    size_t i;
    printf("fruity: WinVerifyTrust arguments: hWnd=%#llx pgActionID=%#llx pWVTData=%#llx\n", ULL(R[1]), ULL(R[2]), ULL(R[8]));
    if (peek(R[2], g, sizeof g)) {
        uint32_t d1;
        uint16_t d2, d3;
        memcpy(&d1, g, 4);
        memcpy(&d2, g + 4, 2);
        memcpy(&d3, g + 6, 2);
        printf("fruity:   action GUID {%08x-%04x-%04x-%02X%02X-%02X%02X%02X%02X%02X%02X}%s\n", d1, d2, d3, g[8], g[9], g[10], g[11], g[12], g[13],
               g[14], g[15], memcmp(g, generic_v2, 16) == 0 ? " = WINTRUST_ACTION_GENERIC_VERIFY_V2" : " (not GENERIC_VERIFY_V2)");
    } else {
        printf("fruity:   pgActionID is not readable\n");
    }
    if (!peek(R[8], d, sizeof d)) {
        printf("fruity:   pWVTData is not readable\n");
        return;
    }
    printf("fruity:   WINTRUST_DATA bytes[0x58]=");
    for (i = 0; i < sizeof d; i++) printf("%02x", d[i]);
    printf("\n");
    {
        uint32_t cb, ui, rev, un, st, pf;
        memcpy(&cb, d + 0, 4);
        memcpy(&ui, d + 24, 4);
        memcpy(&rev, d + 28, 4);
        memcpy(&un, d + 32, 4);
        memcpy(&pfile, d + 40, 8);
        memcpy(&st, d + 48, 4);
        memcpy(&pf, d + 72, 4);
        printf("fruity:   cbStruct=%u dwUIChoice=%u fdwRevocationChecks=%u dwUnionChoice=%u pFile=%#llx dwStateAction=%u dwProvFlags=%#x (offsets 0,24,28,32,40,48,72)\n",
               cb, ui, rev, un, ULL(pfile), st, pf);
    }
    if (pfile && peek(pfile, fi, sizeof fi)) {
        uint64_t path, hfile, known;
        uint32_t cb;
        char ws[272];
        memcpy(&cb, fi, 4);
        memcpy(&path, fi + 8, 8);
        memcpy(&hfile, fi + 16, 8);
        memcpy(&known, fi + 24, 8);
        read_utf16(path, ws, sizeof ws);
        printf("fruity:   WINTRUST_FILE_INFO cbStruct=%u pcwszFilePath=%#llx hFile=%#llx pgKnownSubject=%#llx path=\"%s\" (offsets 0,8,16,24)\n", cb,
               ULL(path), ULL(hfile), ULL(known), ws);
    }
}

/* Where the stop is: the VA that "unsupported instruction at <VA>" names, else the innermost lifted function of the shadow stack (*from_msg = 0).
 * Returns the import name when that VA is a lifted `jmp [IAT]` thunk. */
static const char *stop_import(const char *msg, uint64_t *va_out, int *from_msg) {
    static const char key[] = "unsupported instruction at ";
    const char *p = strstr(msg, key);
    uint64_t va = p ? strtoull(p + sizeof key - 1, NULL, 16) : 0;
    *from_msg = va != 0;
    if (!va && trace_depth() > depth_at_start) {
        uint64_t f, r;
        if (trace_frame(0, &f, &r)) va = f;
    }
    *va_out = va;
    return va ? thunk_import(va) : NULL;
}

static void print_shadow(void) {
    size_t depth = trace_depth(), k, shown = 0;
    printf("fruity: shadow call stack (lifted functions entered and not left): depth %zu; the lowest %zu are frames left over from before this call "
           "(frames that the guest's own longjmp discarded are never popped)\n",
           depth, depth_at_start);
    for (k = 0; k < depth && shown < 40; k++) {
        uint64_t va, ret;
        char nb[160];
        if (depth - k <= depth_at_start) break;
        if (!trace_frame(k, &va, &ret)) break;
        printf("fruity:   #%zu %#llx %s guest-return=%#llx\n", k, ULL(va), func_name(va, nb, sizeof nb), ULL(ret));
        shown++;
    }
}

#ifdef FLE_IDLE
/* Decision 0129: (b') the guest was stable for N virtual seconds (st = 1), or the virtual-clock limit was reached first (st = 2). The report is the one of every stop plus the idle block. */
static void idle_stop(int st) {
    char reason[300];
    if (st == 1)
        snprintf(reason, sizeof reason, "stable idle (decision 0129, stop condition (b')): no new function entry, no new API name, and unchanged window table and drawing surfaces for N virtual seconds");
    else
        snprintf(reason, sizeof reason, "limit: the virtual clock moved %llu s after CreateFruityInstance began without a stable idle (FLE_FRUITY_VIRTUAL_SECONDS)", ULL(idle_virtual_limit_ms() / 1000));
    report(reason, st == 1 ? 10 : 11);
    idle_describe(stdout);
    fflush(stdout);
    finish(st == 1 ? 10 : 11);
}
#endif

/* Decision 0129, stop condition (d'): the first call of a text-drawing API (ExtTextOutW, DrawTextW, DrawTextExW, GetTextExtentPoint32W, GetTextExtentPointW, GetTextMetricsW). fle_gdi.c calls this (weak there) just
 * before it stops by name, with R[] as the guest set it. It prints the caller (the innermost lifted function and the guest return address), the arguments of the call as the x64 convention gives them (the string
 * and its length, the rectangle, the flags), the font selected into the DC and its LOGFONTW, and the last 30 lifted entries; the stop that follows is the named stop of the runtime. It never answers anything. */
static void text_string(uint64_t p, uint64_t n, char *out, size_t cap) {
    size_t k = 0, m = n > 120 ? 120 : (size_t)n;
    uint8_t w[2];
    if (!p) { snprintf(out, cap, "(NULL)"); return; }
    for (; k < m && k + 1 < cap; k++) {
        if (!peek(p + 2 * k, w, 2)) { if (k == 0) snprintf(out, cap, "(unreadable)"); else out[k] = 0; return; }
        out[k] = (w[1] == 0 && w[0] >= 0x20 && w[0] < 0x7f) ? (char)w[0] : '?';
    }
    out[k] = 0;
}

static void text_rect(const char *label, uint64_t p) {
    int32_t q[4];
    if (!p) { printf("fruity:   %s = NULL\n", label); return; }
    if (!peek(p, q, sizeof q)) { printf("fruity:   %s = %#llx (unreadable)\n", label, ULL(p)); return; }
    printf("fruity:   %s = %#llx {left %d, top %d, right %d, bottom %d}\n", label, ULL(p), q[0], q[1], q[2], q[3]);
}

void fle_text_hook(const char *api, uint64_t hdc) {
    char nb[160], str[160];
    uint64_t sva = 0, f, r, k, n;
    uint64_t a5 = fle_ld(R[4] + 0x28, 8), a6 = fle_ld(R[4] + 0x30, 8), a7 = fle_ld(R[4] + 0x38, 8), a8 = fle_ld(R[4] + 0x40, 8);
    uint64_t hfont = fle_gdi_dc_font(hdc);
    uint8_t lf[FLE_GDI_LOGFONTW_SIZE];
    static int done;
    if (done || !active) return;
    done = 1;
    printf("fruity: ==== TEXT DRAWING REACHED (decision 0129, stop condition (d')): the first call of %s; entries since the call began %llu; virtual clock %llu ms\n", api,
           ULL(trace_call_count() - calls_at_start), ULL(fle_sys_clock_ticks() / FLE_TICKS_PER_MS));
    if (trace_depth() > depth_at_start && trace_frame(0, &sva, &f)) {
        printf("fruity:   caller: innermost lifted function %s, guest return address of this call %#llx (stack top [rsp] = %#llx)\n", func_name(sva, nb, sizeof nb), ULL(fle_ld(R[4], 8)), ULL(R[4]));
    }
    for (k = 1; k < 8 && trace_frame(k, &sva, &r); k++) printf("fruity:   caller chain #%llu: %s (guest return %#llx)\n", ULL(k), func_name(sva, nb, sizeof nb), ULL(r));
    printf("fruity:   hdc = %#llx\n", ULL(hdc));
    if (strcmp(api, "ExtTextOutW") == 0) {
        /* BOOL ExtTextOutW(HDC hdc, int x, int y, UINT options, const RECT *lprect, LPCWSTR lpString, UINT c, const INT *lpDx) */
        text_string(a6, (uint32_t)a7, str, sizeof str);
        printf("fruity:   x = %d, y = %d, options = %#x, c = %u, lpDx = %#llx\n", (int32_t)R[2], (int32_t)R[8], (uint32_t)R[9], (uint32_t)a7, ULL(a8));
        text_rect("lprect", a5);
        printf("fruity:   string (%#llx) = \"%s\"\n", ULL(a6), str);
    } else if (strcmp(api, "DrawTextW") == 0 || strcmp(api, "DrawTextExW") == 0) {
        /* int DrawTextW(HDC hdc, LPCWSTR lpchText, int cchText, LPRECT lprc, UINT format); DrawTextExW(..., LPDRAWTEXTPARAMS lpdtp) */
        text_string(R[2], (uint32_t)R[8], str, sizeof str);
        printf("fruity:   cchText = %d, format = %#x%s\n", (int32_t)R[8], (uint32_t)a5, strcmp(api, "DrawTextExW") == 0 ? "" : "");
        text_rect("lprc", R[9]);
        if (strcmp(api, "DrawTextExW") == 0) printf("fruity:   lpdtp = %#llx\n", ULL(a6));
        printf("fruity:   string (%#llx) = \"%s\"\n", ULL(R[2]), str);
    } else if (strcmp(api, "GetTextExtentPoint32W") == 0 || strcmp(api, "GetTextExtentPointW") == 0) {
        /* BOOL GetTextExtentPoint32W(HDC hdc, LPCWSTR lpString, int c, LPSIZE psizl) */
        text_string(R[2], (uint32_t)R[8], str, sizeof str);
        printf("fruity:   c = %d, psizl = %#llx\n", (int32_t)R[8], ULL(R[9]));
        printf("fruity:   string (%#llx) = \"%s\"\n", ULL(R[2]), str);
    } else {
        printf("fruity:   lptm = %#llx (GetTextMetricsW has no string)\n", ULL(R[2]));
    }
    if (hfont && fle_gdi_font_logfont(hfont, lf)) {
        int32_t h32, w32, esc, ori, wt;
        char face[40];
        size_t i;
        memcpy(&h32, lf, 4); memcpy(&w32, lf + 4, 4); memcpy(&esc, lf + 8, 4); memcpy(&ori, lf + 12, 4); memcpy(&wt, lf + 16, 4);
        for (i = 0; i < 31; i++) { uint16_t u = (uint16_t)(lf[28 + 2 * i] | (lf[29 + 2 * i] << 8)); if (!u) break; face[i] = (u >= 0x20 && u < 0x7f) ? (char)u : '?'; }
        face[i] = 0;
        printf("fruity:   font selected into hdc: handle %#llx LOGFONTW lfHeight %d lfWidth %d lfEscapement %d lfOrientation %d lfWeight %d lfItalic %u lfUnderline %u lfStrikeOut %u lfCharSet %u lfOutPrecision %u lfClipPrecision %u lfQuality %u lfPitchAndFamily %#x lfFaceName \"%s\"\n",
               ULL(hfont), h32, w32, esc, ori, wt, lf[20], lf[21], lf[22], lf[23], lf[24], lf[25], lf[26], lf[27], face);
        printf("fruity:   LOGFONTW bytes[92] =");
        for (i = 0; i < sizeof lf; i++) printf("%s%02x", i % 28 == 0 ? "\nfruity:     " : "", lf[i]);
        printf("\n");
    } else {
        printf("fruity:   font selected into hdc: handle %#llx (a stock or default font of the virtual DC table: no LOGFONTW is recorded for it; decision 0066 gives a LOGFONT only to the fonts CreateFontIndirectW made)\n", ULL(hfont));
    }
    n = ering_n < 30 ? ering_n : 30;
    printf("fruity:   the last %llu lifted function entries before this call, newest first (seq = entries since the call began):\n", ULL(n));
    for (k = 0; k < n; k++) {
        const erec *e = &ering[(ering_n - 1 - k) % RING];
        cinfo *c = cache_get(e->fn);
        printf("fruity:     #%llu %s guest-return=%#llx seq=%llu\n", ULL(k), func_name(c->va, nb, sizeof nb), ULL(e->ret), ULL(ering_n - k));
    }
    printf("fruity: ==== the stop that follows is the named stop of the runtime (the text API is not answered)\n");
    fflush(stdout);
}

static void print_host_log(void) {
    uint64_t total = fruity_host_log_total(), i;
    printf("fruity: calls the engine made into the fake host object: slot0=%llu slot1=%llu slot2=%llu\n", ULL(fruity_host_calls(0)),
           ULL(fruity_host_calls(1)), ULL(fruity_host_calls(2)));
    for (i = 0; i < total && i < FRUITY_LOG_MAX; i++) {
        const fruity_call *c = fruity_host_log_at(i);
        if (!c) break;
        if (c->slot == 1) {
            printf("fruity:   #%llu slot1 edx=%lld r8d=%lld -> (no result)\n", ULL(i), (long long)c->id, (long long)c->index);
        } else {
            printf("fruity:   #%llu slot2%s id=%lld index=%lld value=%lld -> %llu\n", ULL(i), c->via_slot0 ? " (via slot 0)" : "", (long long)c->id,
                   (long long)c->index, (long long)c->value, ULL(c->result));
        }
    }
    if (total > FRUITY_LOG_MAX) printf("fruity:   ... %llu more calls not logged\n", ULL(total - FRUITY_LOG_MAX));
}

static void report(const char *reason, int code) {
    static const char *const rn[16] = {"rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"};
    char msg[4096], nb[160];
    const char *m = trace_full_message();
    const char *imp;
    uint64_t sva = 0, k, n;
    struct timespec t1;
    size_t i;
    int from_msg = 0;
    if (in_report) _exit(5);
    in_report = 1;
#ifdef __EMSCRIPTEN__
    fruity_surface_stop_event(); /* decision 0128: the drawing surfaces at the stop, taken before the report reads anything */
#else
    alarm(0);
#endif
    snprintf(msg, sizeof msg, "%s", m ? m : "");
    for (i = strlen(msg); i > 0 && (msg[i - 1] == '\n' || msg[i - 1] == '\r'); i--) msg[i - 1] = 0;
    {
        /* decision 0209: "fle: unsupported instruction at 0x..." names no API; when that address is a lifted `jmp [IAT]` thunk, the import it stands for (read from the image bytes, as the line below says) is added to the message, so
         * that the message alone (the page's beacon carries it) tells the API. Nothing else of the report changes. */
        uint64_t va0 = 0;
        int from0 = 0;
        const char *im0 = strstr(msg, "unsupported instruction at ") ? stop_import(msg, &va0, &from0) : NULL;

        if (im0 && from0 && !strstr(msg, "[import ") && strlen(msg) + strlen(im0) + 12 < sizeof msg) snprintf(msg + strlen(msg), sizeof msg - strlen(msg), " [import %s]", im0);
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("fruity: == stop: %s\n", reason);
    printf("fruity: runtime message: %s\n", msg[0] ? msg : "(none)");
    printf("fruity: abort kind: %s\n", kind_of(msg));
    {
        const char *p = strstr(msg, "0x");
        if (p && strstr(msg, "not a known function entry")) {
            uint64_t va = strtoull(p, NULL, 16);
            const char *im = image_import_name(va);
            printf("fruity: dispatch target: %#llx%s%s\n", ULL(va), im ? " import " : "", im ? im : "");
        }
    }
    imp = stop_import(msg, &sva, &from_msg);
    if (imp) {
        printf("fruity: the stop is in the lifted thunk f_%llx (%s) = `jmp [IAT slot]` of import %s (read from the image bytes there)\n", ULL(sva),
               from_msg ? "named by the message" : "innermost lifted function", imp);
    } else if (sva) {
        printf("fruity: %s %#llx is not a `jmp [IAT slot]` thunk of the mapped images (or its bytes are not in an image)\n",
               from_msg ? "the instruction that the message names," : "the innermost lifted function", ULL(sva));
    }
    printf("fruity: CreateFruityInstance at %#llx; elapsed %.2f s; lifted entries since the call began: %llu; new distinct functions: %llu\n",
           ULL(call_va), (double)(t1.tv_sec - t_start.tv_sec) + (double)(t1.tv_nsec - t_start.tv_nsec) / 1e9, ULL(trace_call_count() - calls_at_start),
           ULL(trace_distinct_count() - distinct_at_start));
    print_shadow();
#ifdef __EMSCRIPTEN__
    {
        extern size_t trace_depth_max(void);
        /* decision 0128: what only a wasm run has. "wf1:" lines are not "fruity:" lines (the native report has no such line) */
        printf("wf1: shadow stack depth, the largest of the whole run (lifted frames, not wasm frames): %zu; linear memory now %llu bytes; entries since program start %llu\n", trace_depth_max(),
               ULL((uint64_t)__builtin_wasm_memory_size(0) * 65536u), ULL(trace_call_count()));
    }
#endif
    printf("fruity: guest registers at the stop (R[], eager):\n");
    for (i = 0; i < 16; i += 4) {
        printf("fruity:   %s=%#llx %s=%#llx %s=%#llx %s=%#llx\n", rn[i], ULL(R[i]), rn[i + 1], ULL(R[i + 1]), rn[i + 2], ULL(R[i + 2]), rn[i + 3],
               ULL(R[i + 3]));
    }
    {
        uint64_t q[8];
        if (peek(R[4], q, sizeof q)) {
            printf("fruity: guest stack at rsp (return address, 4 shadow slots, then stack arguments):");
            for (i = 0; i < 8; i++) printf(" %#llx", ULL(q[i]));
            printf("\n");
        }
    }
    printf("fruity: argument registers (rcx rdx r8 r9) and what they point at (best effort, process_vm_readv):\n");
    describe_value("rcx", R[1]);
    describe_value("rdx", R[2]);
    describe_value("r8 ", R[8]);
    describe_value("r9 ", R[9]);
    {
        /* decision 0129 (information only): the stack arguments 5 to 8 of a stop at an import thunk, at [rsp + 0x28 + 8 * (n - 5)] when the thunk starts, and what they point at */
        char lb[24];
        unsigned an;

        for (an = 5; an <= 8; an++) {
            uint64_t v = 0;

            if (!peek(R[4] + 0x28u + 8u * (an - 5u), &v, 8)) break;
            snprintf(lb, sizeof lb, "arg%u", an);
            describe_value(lb, v);
        }
    }
    if (imp && strstr(imp, "!WinVerifyTrust")) decode_wvt();
    n = iring_n < RING ? iring_n : RING;
    printf("fruity: last %llu entries into import thunks, newest first (a `jmp [IAT]` thunk entered from lifted code; calls through pseudo-VAs or "
           "`call [IAT]` do not show here; args = rcx rdx r8 r9 at the entry):\n",
           ULL(n));
    for (k = 0; k < n; k++) {
        const irec *r = &iring[(iring_n - 1 - k) % RING];
        printf("fruity:   #%llu %s guest-return=%#llx args=%#llx %#llx %#llx %#llx seq=%llu\n", ULL(k), r->imp, ULL(r->ret), ULL(r->a1), ULL(r->a2),
               ULL(r->a3), ULL(r->a4), ULL(r->seq));
    }
    n = ering_n < RING ? ering_n : RING;
    printf("fruity: last %llu lifted function entries, newest first (seq = entries since the call began):\n", ULL(n));
    for (k = 0; k < n; k++) {
        const erec *e = &ering[(ering_n - 1 - k) % RING];
        cinfo *c = cache_get(e->fn);
        printf("fruity:   #%llu %s guest-return=%#llx seq=%llu\n", ULL(k), func_name(c->va, nb, sizeof nb), ULL(e->ret), ULL(ering_n - k));
    }
    print_host_log();
    if (fle_thread_report) {
        /* decision 0122: the guest threads, their queues, the virtual clock and the windows at the stop. The report is printed from the stop handler: it reads the tables only. */
        printf("fruity: guest threads at the stop (decision 0122):\n");
        fle_thread_report(stdout);
    }
    printf("fruity: the host object is at %#llx (vtable %#llx, base directory \"%s\"); instance slot at %#llx holds %#llx\n", ULL(host.obj),
           ULL(host.vtable), host.mem ? (const char *)(host.mem + FRUITY_BLOCK_BASEDIR) : "", ULL((uint64_t)(uintptr_t)&instance_slot),
           ULL(instance_slot));
    fflush(stdout);
#ifdef __GLIBC__
    {
        void *bt[64];
        int nb2 = backtrace(bt, 64);
        printf("fruity: C stack of the harness at the stop (backtrace(); lines below are written by backtrace_symbols_fd, so they have no "
               "\"fruity:\" prefix; PIE offsets: use `addr2line -f -e <two-native> <offset>`, or run under gdb with FLE_FRUITY_TRAP=1 and use bt):\n");
        fflush(stdout);
        backtrace_symbols_fd(bt, nb2, 1);
    }
#endif
    printf("fruity: exit code %d (the codes of harness/wasm/trace_native.c: 134 SIGABRT, 139 SIGSEGV, 135 SIGBUS, 132 SIGILL, 136 SIGFPE, 4 wall time, 3 "
           "entry limit; 6 = named stop of this file)\n",
           code);
    fflush(stdout);
}


static void finish(int code) {
    fflush(stdout);
    if (cfg_trap) {
        fprintf(stderr, "fruity: FLE_FRUITY_TRAP=1: raising SIGTRAP; under gdb `bt` shows the C stack\n");
        raise(SIGTRAP);
    }
    _exit(code);
}

#ifdef __EMSCRIPTEN__
static void on_abort_wasm(void) {
    report("SIGABRT (runtime abort, see runtime message)", 134);
    finish(134);
}

static void on_wall_limit(void) {
    struct timespec t1;
    char reason[200];
    clock_gettime(CLOCK_MONOTONIC, &t1);
    if ((uint64_t)(t1.tv_sec - t_start.tv_sec) < cfg_seconds) return;
    snprintf(reason, sizeof reason, "limit: wall time %llu s after CreateFruityInstance began (FLE_FRUITY_SECONDS)", ULL(cfg_seconds));
    report(reason, 4);
    finish(4);
}
#else
static void on_signal(int sig, siginfo_t *si, void *uc) {
    char reason[200];
    int code;
    (void)uc;
    switch (sig) {
    case SIGABRT:
        snprintf(reason, sizeof reason, "SIGABRT (runtime abort, see runtime message)");
        code = 134;
        break;
    case SIGSEGV:
        snprintf(reason, sizeof reason, "SIGSEGV fault address %p", si->si_addr);
        code = 139;
        break;
    case SIGBUS:
        snprintf(reason, sizeof reason, "SIGBUS fault address %p", si->si_addr);
        code = 135;
        break;
    case SIGILL:
        snprintf(reason, sizeof reason, "SIGILL");
        code = 132;
        break;
    case SIGFPE:
        snprintf(reason, sizeof reason, "SIGFPE");
        code = 136;
        break;
    case SIGALRM:
        snprintf(reason, sizeof reason, "limit: wall time %llu s after CreateFruityInstance began (FLE_FRUITY_SECONDS)", ULL(cfg_seconds));
        code = 4;
        break;
    default:
        snprintf(reason, sizeof reason, "signal %d", sig);
        code = 128 + sig;
        break;
    }
    report(reason, code);
    finish(code);
}
#endif

static void on_calls_limit(void) {
    char reason[200];
    snprintf(reason, sizeof reason, "limit: %llu lifted function entries after CreateFruityInstance began (FLE_FRUITY_MAX_CALLS)", ULL(cfg_max_calls));
    report(reason, 3);
    finish(3);
}

#ifdef __EMSCRIPTEN__
static void install_handlers(void) {
    extern void (*trace_abort_hook)(void);
    trace_abort_hook = on_abort_wasm;
}
#else
static void install_handlers(void) {
    struct sigaction sa;
    const int sigs[] = {SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGALRM};
    size_t i;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_signal;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&sa.sa_mask);
    for (i = 0; i < sizeof sigs / sizeof sigs[0]; i++) sigaction(sigs[i], &sa, NULL);
}
#endif

/* ---- configuration ---- */
static int parse_num(const char *name, uint64_t lo, uint64_t hi, uint64_t *out) {
    const char *s = getenv(name);
    char *end;
    unsigned long long v;
    if (!s || !*s) return 0;
    errno = 0;
    v = strtoull(s, &end, 10);
    if (errno != 0 || end == s || *end != 0 || v < lo || v > hi) {
        fprintf(stderr, "fruity: %s=\"%s\" is not a decimal number from %llu to %llu\n", name, s, ULL(lo), ULL(hi));
        return -1;
    }
    *out = v;
    return 1;
}

int fruity_precheck(void) {
    static const char *const others[] = {"FLE_FRUITY_MAX_CALLS", "FLE_FRUITY_SECONDS", "FLE_FRUITY_STOP_AT", "FLE_FRUITY_TRAP", "FLE_FRUITY_STABLE_SECONDS", "FLE_FRUITY_VIRTUAL_SECONDS", "FLE_FRUITY_PEEK_AT", "FLE_FRUITY_PEEK_FROM", "FLE_FRUITY_PEEK_TO"};
    static void (*const stubs[3])(void) = {fruity_host_slot0, fruity_host_slot1, fruity_host_slot2};
    const char *e = FLE_TEST_GETENV("FLE_CALL_FRUITY");
    const char *s;
    unsigned k;
    size_t i;
    if (e && *e && strcmp(e, "0") != 0 && strcmp(e, "1") != 0) {
        fprintf(stderr, "fruity: FLE_CALL_FRUITY=\"%s\": only unset, empty, 0 or 1 are defined\n", e);
        return -1;
    }
#ifdef __EMSCRIPTEN__
    cfg_on = 1;   /* decision 0243: the wasm build is the engine run (the manifest passed FLE_CALL_FRUITY=1) */
#else
    cfg_on = (e && strcmp(e, "1") == 0);
#endif
    if (!cfg_on) {
        for (i = 0; i < sizeof others / sizeof others[0]; i++) {
            s = getenv(others[i]);
            if (s && *s) {
                fprintf(stderr, "fruity: %s is set but FLE_CALL_FRUITY=1 is not: refused (nothing runs)\n", others[i]);
                return -1;
            }
        }
        return 0;
    }
#ifdef FLE_IDLE
    if (idle_configure() < 0) return -1; /* decision 0129: FLE_FRUITY_STABLE_SECONDS / FLE_FRUITY_VIRTUAL_SECONDS (unset = nothing) */
#else
    {
        const char *i1 = FLE_TEST_GETENV("FLE_FRUITY_STABLE_SECONDS"), *i2 = FLE_TEST_GETENV("FLE_FRUITY_VIRTUAL_SECONDS");
        if ((i1 && *i1) || (i2 && *i2)) {
            fprintf(stderr, "fruity: FLE_FRUITY_STABLE_SECONDS / FLE_FRUITY_VIRTUAL_SECONDS need a build with IDLE=1 (build_two_native.sh): refused (nothing runs)\n");
            return -1;
        }
    }
#endif
    if (parse_num("FLE_FRUITY_MAX_CALLS", 1, UINT64_C(1) << 40, &cfg_max_calls) < 0) return -1;
    if (parse_num("FLE_FRUITY_SECONDS", 1, 36000, &cfg_seconds) < 0) return -1;
    s = FLE_TEST_GETENV("FLE_FRUITY_TRAP");
    if (s && *s) {
        if (strcmp(s, "0") != 0 && strcmp(s, "1") != 0) {
            fprintf(stderr, "fruity: FLE_FRUITY_TRAP=\"%s\": only 0 or 1 are defined\n", s);
            return -1;
        }
        cfg_trap = (strcmp(s, "1") == 0);
    }
    s = FLE_TEST_GETENV("FLE_FRUITY_STOP_AT");
    if (s && *s) {
        char *tok, *save = NULL;
        if (strlen(s) >= sizeof stop_buf) {
            fprintf(stderr, "fruity: FLE_FRUITY_STOP_AT is longer than %zu characters\n", sizeof stop_buf - 1);
            return -1;
        }
        snprintf(stop_buf, sizeof stop_buf, "%s", s);
        for (tok = strtok_r(stop_buf, ",", &save); tok; tok = strtok_r(NULL, ",", &save)) {
            if (n_stop == MAX_STOP_NAMES) {
                fprintf(stderr, "fruity: FLE_FRUITY_STOP_AT names more than %d APIs\n", MAX_STOP_NAMES);
                return -1;
            }
            stop_names[n_stop++] = tok;
        }
        if (n_stop == 0) {
            fprintf(stderr, "fruity: FLE_FRUITY_STOP_AT=\"%s\" names no API\n", s);
            return -1;
        }
    }
    s = FLE_TEST_GETENV("FLE_FRUITY_PEEK_FROM");
    if (s && *s) peek_from = strtoull(s, NULL, 10);
    s = FLE_TEST_GETENV("FLE_FRUITY_PEEK_TO");
    if (s && *s) peek_to = strtoull(s, NULL, 10);
    s = FLE_TEST_GETENV("FLE_FRUITY_PEEK_AT");
    if (s && *s) {
        /* decision 0129 (information only): FLE_FRUITY_PEEK_AT=<hex VA>[:<reg>][,...]: at the first 40 entries of the lifted function f_<VA> print the UTF-16 string that the register points at (reg = rcx, rdx, r8 or r9; default rdx) */
        char pb[256], *tok, *save = NULL;

        snprintf(pb, sizeof pb, "%s", s);
        for (tok = strtok_r(pb, ",", &save); tok; tok = strtok_r(NULL, ",", &save)) {
            char *colon = strchr(tok, ':');
            char *at = NULL;
            int reg = 2;

            if (n_peek_at == MAX_PEEK) {
                fprintf(stderr, "fruity: FLE_FRUITY_PEEK_AT names more than %d functions\n", MAX_PEEK);
                return -1;
            }
            if (colon) {
                *colon++ = 0;
                at = strchr(colon, '@');
                if (at) *at++ = 0;
                reg = strcmp(colon, "rcx") == 0 ? 1 : strcmp(colon, "rdx") == 0 ? 2 : strcmp(colon, "r8") == 0 ? 8 : strcmp(colon, "r9") == 0 ? 9 : -1;
                if (reg < 0) {
                    fprintf(stderr, "fruity: FLE_FRUITY_PEEK_AT register must be rcx, rdx, r8 or r9\n");
                    return -1;
                }
            }
            peek_at[n_peek_at].va = strtoull(tok, NULL, 16);
            peek_at[n_peek_at].reg = reg;
            peek_at[n_peek_at].has_off = at != NULL;
            peek_at[n_peek_at].off = at ? strtoull(at, NULL, 16) : 0;
            n_peek_at++;
        }
    }
    for (k = 0; k < 3; k++) {
        if (find_fn(FRUITY_STUB_VA(k)) != stubs[k]) {
            fprintf(stderr,
                    "fruity: the dispatch table has no row %#llx -> fruity_host_slot%u: this binary was not linked from a common.c merged with "
                    "lift/harness/fruity-rows.txt (build it with FLE_CALL_FRUITY=1 through lift/harness/a4-multi.sh, or FRUITY_ROWS=... for gen_two.sh and "
                    "FRUITY=1 for build_two_native.sh); nothing runs\n",
                    ULL(FRUITY_STUB_VA(k)), k);
            return -1;
        }
    }
    return 0;
}

/* ---- the call ---- */
int fruity_run(uint64_t entry_sp) {
    uint64_t rax;
    uint32_t ord = 0;
    int32_t eax;
    void (*fn)(void);
    char nb[160];
    if (!cfg_on) return 0;
    printf("fruity: DllMain returned (the report above is its report). Stage 1: calling CreateFruityInstance as FL64.exe does "
           "(docs/investigations/host-engine-boundary.md 5.1 #10)\n");
    if (nimgs < 1) {
        printf("fruity: no image was registered (fruity_add_image); nothing to call\n");
        return 2;
    }
    call_va = export_va(imgs[0].base, imgs[0].size, "CreateFruityInstance", &ord);
    if (!call_va) {
        printf("fruity: the PE export table of the image at %#llx has no (non-forwarded) CreateFruityInstance\n", ULL(imgs[0].base));
        return 2;
    }
    fn = find_fn(call_va);
    if (!fn) {
        printf("fruity: CreateFruityInstance %#llx is not a row of fle_dispatch_table (not a lifted function entry)\n", ULL(call_va));
        return 2;
    }
    printf("fruity: CreateFruityInstance: export ordinal %u, va %#llx (PE export table of the image at %#llx)\n", ord, ULL(call_va), ULL(imgs[0].base));
    if (fruity_host_make(&host, FLE_VIRTUAL_DLL_PATH) != 0) {
        printf("fruity: cannot build the host object (engine path \"%s\")\n", FLE_VIRTUAL_DLL_PATH);
        return 2;
    }
    {
        uint64_t vt = fle_ld(host.obj + FRUITY_OFF_VTABLE, 8);
        printf("fruity: host object at %#llx (0x%x bytes): vtable=%#llx [slot0=%#llx slot1=%#llx slot2=%#llx] version=%llu flags=%#llx +0x10=%#llx\n",
               ULL(host.obj), FRUITY_HOST_SIZE, ULL(vt), ULL(fle_ld(vt, 8)), ULL(fle_ld(vt + 8, 8)), ULL(fle_ld(vt + 16, 8)),
               ULL(fle_ld(host.obj + FRUITY_OFF_VERSION, 4)), ULL(fle_ld(host.obj + FRUITY_OFF_FLAGS, 4)), ULL(fle_ld(host.obj + FRUITY_OFF_QWORD10, 8)));
        printf("fruity:   +0x18 name=%#llx \"%s\"  +0x20=%#llx \"%s\"  +0x28=%#llx \"%s\"  +0x30 base directory=%#llx \"%s\" (buffer 0x%x bytes)\n",
               ULL(fle_ld(host.obj + FRUITY_OFF_NAME, 8)), (const char *)(uintptr_t)fle_ld(host.obj + FRUITY_OFF_NAME, 8),
               ULL(fle_ld(host.obj + FRUITY_OFF_STR2, 8)), (const char *)(uintptr_t)fle_ld(host.obj + FRUITY_OFF_STR2, 8),
               ULL(fle_ld(host.obj + FRUITY_OFF_STR3, 8)), (const char *)(uintptr_t)fle_ld(host.obj + FRUITY_OFF_STR3, 8),
               ULL(fle_ld(host.obj + FRUITY_OFF_BASEDIR, 8)), (const char *)(uintptr_t)fle_ld(host.obj + FRUITY_OFF_BASEDIR, 8), FRUITY_BASEDIR_CAP);
    }
    if (R[4] != entry_sp) {
        printf("fruity: note: R[4] after DllMain is %#llx, not the %#llx it had before the entry call; the call uses the latter\n", ULL(R[4]),
               ULL(entry_sp));
        R[4] = entry_sp;
    }
    if (fle_callback_depth() != 0) printf("fruity: note: fle_call_guest nesting depth is %u before the call (frames of an earlier callback were discarded)\n", fle_callback_depth());
    instance_slot = 0; /* FL64.exe's `instance` variable is zeroed before the call (0x14000764d) */
    calls_at_start = trace_call_count();
    distinct_at_start = trace_distinct_count();
    depth_at_start = trace_depth();
    printf("fruity: before the call: functions executed (distinct) %llu, entries %llu, shadow depth %zu (frames left over from DllMain's longjmps)\n",
           ULL(distinct_at_start), ULL(calls_at_start), depth_at_start);
    printf("fruity: limits: %llu lifted entries, %llu s wall%s%s%s\n", ULL(cfg_max_calls), ULL(cfg_seconds), n_stop ? ", named stop at: " : "",
           n_stop ? stop_names[0] : "", n_stop > 1 ? ", ..." : "");
    printf("fruity: calling %s(rcx=%#llx, rdx=%#llx, r8=host %#llx, r9=&instance %#llx) with rsp=%#llx\n", func_name(call_va, nb, sizeof nb),
           ULL(FRUITY_CALL_RCX), ULL(FRUITY_CALL_RDX), ULL(host.obj), ULL((uint64_t)(uintptr_t)&instance_slot), ULL(R[4]));
    fflush(stdout);

    clock_gettime(CLOCK_MONOTONIC, &t_start);
    trace_message_reset(); /* the report's "runtime message" must be one of this phase (trace_native.c keeps the last message of the whole run) */
    if (fle_thread_set_idle_stop) {
        /* decision 0122, stop condition (b): the first thread waits for input with an empty queue and nothing else can run: the scheduler stops by name. FLE_FRUITY_NO_IDLE_STOP=1 switches it off. */
#ifdef __EMSCRIPTEN__
        fle_thread_set_idle_stop(0);   /* decision 0243: the wasm build never stops at the first idle (the manifest passed FLE_FRUITY_NO_IDLE_STOP=1) */
#else
        const char *ni = getenv("FLE_FRUITY_NO_IDLE_STOP");

        fle_thread_set_idle_stop(!(ni && ni[0] == '1'));
#endif
        if (FLE_TEST_GETENV("FLE_FRUITY_THREAD_TRACE") && fle_thread_set_trace) fle_thread_set_trace(1);
    }
    install_handlers();
    trace_limit_hook = on_calls_limit;
    trace_set_call_limit(calls_at_start + cfg_max_calls);
#ifndef __EMSCRIPTEN__
    alarm((unsigned)cfg_seconds);
#endif
#ifdef FLE_IDLE
    if (idle_enabled()) idle_begin(fle_sys_clock_ticks() / FLE_TICKS_PER_MS, trace_distinct_count());
#endif
    active = 1;
    rax = fle_call_guest(call_va, FRUITY_CALL_RCX, FRUITY_CALL_RDX, host.obj, (uint64_t)(uintptr_t)&instance_slot);
    active = 0;
#ifndef __EMSCRIPTEN__
    alarm(0);
#endif
    trace_limit_hook = NULL;

    eax = (int32_t)(uint32_t)rax;
    printf("fruity: rax=%#llx instance=%#llx\n", ULL(rax), ULL(instance_slot));
    printf("fruity: CreateFruityInstance RETURNED (eax=%d) after %llu lifted entries\n", eax, ULL(trace_call_count() - calls_at_start));
    if ((eax & ~2) == 0) {
        printf("fruity: FL64.exe would now call instance->vtable[0](instance) (0x140007789-0x1400077b9; a reading). NOT done in stage 1: recorded only\n");
    } else if (eax == -2) {
        printf("fruity: FL64.exe would show MessageBoxA \"Verification error\" for -2 (0x140007791-0x1400077aa; a reading). Not done\n");
    } else {
        printf("fruity: FL64.exe does nothing for this return value (0x140007789-0x140007794; a reading)\n");
    }
    if (instance_slot) {
        uint64_t vt = 0, s0 = 0;
        if (peek(instance_slot, &vt, 8)) {
            printf("fruity: instance->vtable=%#llx", ULL(vt));
            if (peek(vt, &s0, 8)) printf(" slot0=%#llx (%s)", ULL(s0), func_name(s0, nb, sizeof nb));
            printf("\n");
        } else {
            printf("fruity: instance %#llx is not readable\n", ULL(instance_slot));
        }
    }
    if (fle_win_process_ended) {   /* decision 0222: FL64.exe ends here, and the system destroys the windows the application left (fle_win.c) */
        uint32_t before = fle_win_visible_toplevel_count ? fle_win_visible_toplevel_count() : 0;

        fle_win_process_ended();
        printf("fruity: the process ended: visible top-level windows %u before, %u after the system destroyed the windows the application left\n", before, fle_win_visible_toplevel_count ? fle_win_visible_toplevel_count() : 0);
    }
    print_host_log();
    fflush(stdout);
    return 0;
}
