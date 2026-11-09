/* X1 probe of Blend2D (see b2d_probe.h and docs/investigations/blend2d-jit-and-drawing-path.md). Harness only, native only.
 * Linked into the harness only by a build with B2D_PROBE=1 (lift/harness/two/build_two_native.sh), together with
 * -Wl,--wrap=fle_imp_VirtualAlloc (and the other memory APIs below): a call of the guest to one of them reaches __wrap_* here, which records
 * the arguments and then calls the runtime's own function (__real_*). Nothing is answered here: the runtime, the generated C and the
 * other harness files are not changed.
 *
 * What it does, in order:
 *   b2d_probe_precheck  (before the guest runs)  reads FLE_BLEND2D_PROBE / FLE_B2D_*;
 *   b2d_probe_run       (after DllMain returned) finds AggCreateInstance in the export table of the mapped Blend2D image, calls it with
 *                       fle_call_guest, calls attach(buffer, 64, 64, stride 256) through the object's vtable, then the drawing steps
 *                       of FLE_B2D_OPS, each time reading back the pixel buffer. Every line of output starts with "b2d: ". */
#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#ifdef __EMSCRIPTEN__
#error "b2d_probe.c is native only (signals, alarm, backtrace)"
#endif
#ifdef __GLIBC__
#include <execinfo.h>
#endif

#include "a4.h"
#include "boot.h"
#include "census/census.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_sys.h"
#include "b2d_probe.h"

#define ULL(x) ((unsigned long long)(x))
#define DEFAULT_MAX_CALLS UINT64_C(600000000)
#define DEFAULT_SECONDS 600u
#define W 64
#define H 64
#define STRIDE (W * 4)
#define EVLOG 512
#define MAX_REGIONS 64

/* the module as the guest names it, and the offsets of the Blend2D.dll that this probe reads (RVAs of the original file, image base 0x180000000) */
#define B2D_NAME "blend2d.dll"
#define RVA_AGGVT 0x1dbec0u        /* vtable of the Agg2D object (constructor 0x1800568a0 writes it) */
#define RVA_CTXINITAS 0x146ff0u    /* blContextInitAs(BLContext*, BLImage*, BLContextCreateInfo*) as Agg2D::attach calls it (0x180056ce9) */
#define RVA_TLS_INDEX 0x1f1560u    /* _tls_index of the TLS directory (AddressOfIndex) */
#define RVA_TLS_START 0x1e3770u    /* TLS template (StartAddressOfRawData .. EndAddressOfRawData, 32 bytes) */
#define TLS_TEMPLATE_BYTES 0x20u
#define RVA_JITRT_PROVIDER 0x1f2ff0u  /* the global JIT pipe provider the context takes when createInfo.flags bit 0 is clear */
#define RVA_FIXED_PROVIDER 0x1f30e0u  /* the provider it takes when bit 0 (DISABLE_JIT) is set */

/* Agg2D vtable slots: a READING of the disassembly of Blend2D.dll (docs/investigations/blend2d-jit-and-drawing-path.md section 3). */
enum {
    S_ATTACH = 1,       /* attach(buf, w, h, stride) */
    S_CLIPBOX = 3,
    S_CLEARALL = 8,     /* clearAll(r, g, b, a) */
    S_FILLCOLOR = 27,   /* fillColor(r, g, b, a) */
    S_NOFILL = 28,
    S_LINECOLOR = 30,   /* lineColor(r, g, b, a) */
    S_NOLINE = 31,
    S_LINEWIDTH = 42,   /* lineWidth(double in xmm1): the wrapper passes xmm1 on to the context's vtable +0x118 and clears Agg2D+0x59 (a READING; the constructor calls it with a constant in xmm1) */
    S_LINE = 61,        /* line(x1, y1, x2, y2) */
    S_RECT = 63,        /* rectangle(x1, y1, x2, y2) */
    S_ELLIPSE = 67,     /* ellipse(cx, cy, rx, ry) */
    S_RESETPATH = 82,
    S_MOVETO = 83,
    S_LINETO = 85,
    S_CLOSEPOLY = 102,
    S_DRAWPATH = 103    /* drawPath(flag): 0 fill, 1 stroke, 2 fill+stroke */
};

/* ---- configuration ---- */
static int cfg_on;
static int cfg_nojit;
static int cfg_tls;
static uint64_t cfg_max_calls = DEFAULT_MAX_CALLS;
static uint64_t cfg_seconds = DEFAULT_SECONDS;
static int cfg_trap;
static char ops_buf[256];
static const char *ops[32];
static int nops;

/* ---- state ---- */
static volatile int active;
static volatile sig_atomic_t in_report;
static uint64_t b2d_base, b2d_size;
static uint64_t call_va, calls_at_start, distinct_at_start;
static size_t depth_at_start;
static struct timespec t_start;
static uint8_t *pix;
static const char *cur_step = "(none)";
static uint64_t cur_obj;

int b2d_probe_requested(void) { return cfg_on; }

/* ---- the memory events of the guest ---- */
typedef struct {
    const char *api;
    uint64_t a[6], ret, caller;
    int exec;
    const char *step;
} mem_ev;
static mem_ev evlog[EVLOG];
static unsigned nev, nexec, nexec_by[4]; /* 0 VirtualAlloc, 1 CreateFileMappingW, 2 MapViewOfFile, 3 VirtualProtect */
static struct {
    uint64_t base, size;
    const char *how;
} xregs[MAX_REGIONS];
static unsigned nx;
static uint64_t last_cfm_size;

static void note_xreg(uint64_t base, uint64_t size, const char *how) {
    if (nx < MAX_REGIONS) {
        xregs[nx].base = base;
        xregs[nx].size = size;
        xregs[nx].how = how;
        nx++;
    }
}

static void log_ev(const char *api, int kind, const uint64_t *a, int na, int exec, uint64_t caller) {
    mem_ev *e = &evlog[nev % EVLOG];
    int i;
    e->api = api;
    for (i = 0; i < 6; i++) e->a[i] = i < na ? a[i] : 0;
    e->ret = 0;
    e->caller = caller;
    e->exec = exec;
    e->step = cur_step;
    nev++;
    if (exec) {
        nexec++;
        nexec_by[kind]++;
    }
    printf("b2d: [%s] %s(%#llx, %#llx, %#llx, %#llx%s) %s caller=%#llx\n", cur_step, api, ULL(a[0]), ULL(a[1]), ULL(a[2]), ULL(a[3]),
           na > 4 ? ", ..." : "", exec ? "EXEC" : "non-exec", ULL(caller));
    fflush(stdout);
}

void __real_fle_imp_VirtualAlloc(void);
void __real_fle_imp_VirtualFree(void);
void __real_fle_imp_VirtualProtect(void);
void __real_fle_imp_CreateFileMappingW(void);
void __real_fle_imp_MapViewOfFile(void);
void __real_fle_imp_UnmapViewOfFile(void);

/* calls before the probe began (DllMain and what it loaded): counted only, to compare with the probe phase */
static unsigned pre_va, pre_va_exec, pre_vp, pre_vp_exec;

void __wrap_fle_imp_VirtualAlloc(void) {
    if (active) {
        uint64_t a[4] = {R[1], R[2], R[8], R[9]};
        int ex = (a[3] & 0xF0u) != 0; /* PAGE_EXECUTE 0x10, EXECUTE_READ 0x20, EXECUTE_READWRITE 0x40, EXECUTE_WRITECOPY 0x80 */
        log_ev("VirtualAlloc", 0, a, 4, ex, fle_ld(R[4], 8));
        __real_fle_imp_VirtualAlloc();
        printf("b2d:   -> %#llx\n", ULL(R[0]));
        if (ex && R[0]) note_xreg(R[0], a[1], "VirtualAlloc exec");
        fflush(stdout);
        return;
    }
    pre_va++;
    if ((R[9] & 0xF0u) != 0) pre_va_exec++;
    __real_fle_imp_VirtualAlloc();
}

unsigned b2d_pre_counts(unsigned *va_exec, unsigned *vp, unsigned *vp_exec) {
    *va_exec = pre_va_exec;
    *vp = pre_vp;
    *vp_exec = pre_vp_exec;
    return pre_va;
}

void __wrap_fle_imp_VirtualFree(void) {
    if (active) {
        uint64_t a[4] = {R[1], R[2], R[8], 0};
        printf("b2d: [%s] VirtualFree(%#llx, %#llx, %#llx)\n", cur_step, ULL(a[0]), ULL(a[1]), ULL(a[2]));
        fflush(stdout);
    }
    __real_fle_imp_VirtualFree();
}

void __wrap_fle_imp_VirtualProtect(void) {
    if (active) {
        uint64_t a[4] = {R[1], R[2], R[8], R[9]};
        log_ev("VirtualProtect", 3, a, 4, (a[2] & 0xF0u) != 0, fle_ld(R[4], 8));
    } else {
        pre_vp++;
        if ((R[8] & 0xF0u) != 0) pre_vp_exec++;
    }
    __real_fle_imp_VirtualProtect();
}

void __wrap_fle_imp_CreateFileMappingW(void) {
    if (active) {
        uint64_t a[6] = {R[1], R[2], R[8], R[9], fle_arg(5), fle_arg(6)};
        int ex = ((uint32_t)a[2] & 0xF0u) != 0; /* flProtect is the third argument */
        log_ev("CreateFileMappingW", 1, a, 6, ex, fle_ld(R[4], 8));
        last_cfm_size = (a[3] << 32) | (uint32_t)a[4];
    }
    __real_fle_imp_CreateFileMappingW();
    if (active) {
        printf("b2d:   -> %#llx\n", ULL(R[0]));
        fflush(stdout);
    }
}

void __wrap_fle_imp_MapViewOfFile(void) {
    if (active) {
        uint64_t a[5] = {R[1], R[2], R[8], R[9], fle_arg(5)};
        int ex = ((uint32_t)a[1] & 0x20u) != 0; /* FILE_MAP_EXECUTE */
        log_ev("MapViewOfFile", 2, a, 5, ex, fle_ld(R[4], 8));
        __real_fle_imp_MapViewOfFile();
        printf("b2d:   -> %#llx\n", ULL(R[0]));
        if (ex && R[0]) note_xreg(R[0], a[4] ? a[4] : last_cfm_size, "MapViewOfFile exec");
        fflush(stdout);
        return;
    }
    __real_fle_imp_MapViewOfFile();
}

void __wrap_fle_imp_UnmapViewOfFile(void) {
    if (active) {
        printf("b2d: [%s] UnmapViewOfFile(%#llx)\n", cur_step, ULL(R[1]));
        fflush(stdout);
    }
    __real_fle_imp_UnmapViewOfFile();
}

unsigned b2d_pre_counts(unsigned *va_exec, unsigned *vp, unsigned *vp_exec);

/* ---- the PE export table of the mapped image ---- */
static uint64_t export_va(uint64_t base, uint64_t size, const char *want) {
    uint64_t pe, dd, dir, exp_rva, exp_size, afunc, anames, aord, i, nn, nf, wl = strlen(want);
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
    nf = fle_ld(dir + 20, 4);
    nn = fle_ld(dir + 24, 4);
    afunc = fle_ld(dir + 28, 4);
    anames = fle_ld(dir + 32, 4);
    aord = fle_ld(dir + 36, 4);
    if (nn > 0x10000 || afunc + 4 * nf > size || anames + 4 * nn > size || aord + 2 * nn > size) return 0;
    for (i = 0; i < nn; i++) {
        uint64_t nrva = fle_ld(base + anames + 4 * i, 4), ord, frva;
        if (nrva + wl + 1 > size) continue;
        if (strncmp((const char *)(uintptr_t)(base + nrva), want, wl + 1) != 0) continue;
        ord = fle_ld(base + aord + 2 * i, 2);
        if (ord >= nf) return 0;
        frva = fle_ld(base + afunc + 4 * ord, 4);
        if (frva >= exp_rva && frva < exp_rva + exp_size) return 0;
        return base + frva;
    }
    return 0;
}

static int have_fn(uint64_t va) {
    uint64_t lo = 0, hi = fle_dispatch_count;
    while (lo < hi) {
        uint64_t mid = lo + (hi - lo) / 2;
        if (fle_dispatch_table[mid].va < va) lo = mid + 1;
        else hi = mid;
    }
    return lo < fle_dispatch_count && fle_dispatch_table[lo].va == va;
}

/* ---- the stop report ---- */
static void report(const char *reason, int code);
static void finish(int code) __attribute__((noreturn));

static const char *kind_of(const char *m) {
    if (strstr(m, "not a known function entry")) return "fle_dispatch_miss";
    if (strstr(m, "unsupported instruction")) return "fle_unsupported";
    if (strstr(m, "undefined flag")) return "fle_rd (undefined flag read)";
    if (strstr(m, "shift count")) return "fle_bad_count";
    if (strncmp(m, "fle: ", 5) == 0) return "fle_stop (a runtime message that names an API and the reason)";
    return "(not a runtime message)";
}

static void classify_target(uint64_t va) {
    fle_sys_region_view rv;
    unsigned i;
    int hit = 0;
    printf("b2d: classification of the dispatch target %#llx:\n", ULL(va));
    if (va >= b2d_base && va < b2d_base + b2d_size) printf("b2d:   inside the Blend2D image (RVA %#llx) but not a lifted function entry\n", ULL(va - b2d_base));
    if (fle_sys_region_at(va, &rv)) {
        uint64_t pg = (va - rv.base) / FLE_PAGE_SIZE;
        printf("b2d:   inside the VirtualAlloc region [%#llx, %#llx), alloc_protect=%#x, page protect=%#x (%s)\n", ULL(rv.base),
               ULL(rv.base + rv.size), rv.alloc_protect, rv.pages[pg], (rv.pages[pg] & 0xF0u) ? "EXECUTABLE" : "not executable");
        hit = 1;
    }
    for (i = 0; i < nx; i++) {
        if (va >= xregs[i].base && va < xregs[i].base + (xregs[i].size ? xregs[i].size : FLE_PAGE_SIZE)) {
            printf("b2d:   inside the executable region #%u [%#llx, +%#llx) made by %s\n", i, ULL(xregs[i].base), ULL(xregs[i].size), xregs[i].how);
            hit = 1;
        }
    }
    if (!hit) printf("b2d:   not inside any VirtualAlloc region or executable view that this probe recorded\n");
    for (i = 0; i < nx; i++) {
        uint64_t lo = xregs[i].base, sz = xregs[i].size ? xregs[i].size : FLE_PAGE_SIZE, nz_end = 0, nz = 0, k;
        if (va < lo || va >= lo + sz) continue;
        for (k = 0; k < sz; k++) {
            uint8_t b = *(const uint8_t *)(uintptr_t)(lo + k);
            if (b) {
                nz++;
                nz_end = k + 1;
            }
        }
        printf("b2d:   region #%u: %llu non-zero bytes of %llu, last non-zero byte at offset %#llx (a fresh VirtualAlloc region is all zero)\n", i, ULL(nz), ULL(sz),
               ULL(nz_end ? nz_end - 1 : 0));
        printf("b2d:   the %u bytes at the dispatch target (x86-64 machine code that the guest generated, as a reading):", 48u);
        for (k = 0; k < 48 && va + k < lo + sz; k++) printf(" %02x", *(const uint8_t *)(uintptr_t)(va + k));
        printf("\n");
        printf("b2d:   the first 16 bytes of the region:");
        for (k = 0; k < 16; k++) printf(" %02x", *(const uint8_t *)(uintptr_t)(lo + k));
        printf("\n");
    }
}

static void print_shadow(void) {
    size_t d = trace_depth(), k, shown = 0;
    printf("b2d: shadow call stack (lifted functions entered and not left): depth %zu; the lowest %zu frames are left over from before this call\n", d,
           depth_at_start);
    for (k = 0; k < d && shown < 40; k++) {
        uint64_t va, ret;
        if (d - k <= depth_at_start) break;
        if (!trace_frame(k, &va, &ret)) break;
        printf("b2d:   #%zu %#llx (Blend2D RVA %s%#llx) guest-return=%#llx\n", k, ULL(va), va >= b2d_base && va < b2d_base + b2d_size ? "" : "outside ",
               ULL(va >= b2d_base ? va - b2d_base : va), ULL(ret));
        shown++;
    }
}

static void report(const char *reason, int code) {
    static const char *const rn[16] = {"rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"};
    char msg[4096];
    const char *m = trace_full_message();
    struct timespec t1;
    size_t i;
    unsigned k, n;
    if (in_report) _exit(5);
    in_report = 1;
    alarm(0);
    snprintf(msg, sizeof msg, "%s", m ? m : "");
    for (i = strlen(msg); i > 0 && (msg[i - 1] == '\n' || msg[i - 1] == '\r'); i--) msg[i - 1] = 0;
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("b2d: == stop: %s\n", reason);
    printf("b2d: step in progress: %s\n", cur_step);
    printf("b2d: runtime message: %s\n", msg[0] ? msg : "(none)");
    printf("b2d: abort kind: %s\n", kind_of(msg));
    {
        const char *p = strstr(msg, "not a known function entry") ? strstr(msg, "0x") : NULL;
        if (p) classify_target(strtoull(p, NULL, 16));
    }
    printf("b2d: elapsed %.2f s; lifted entries since the probe began: %llu; new distinct functions: %llu\n",
           (double)(t1.tv_sec - t_start.tv_sec) + (double)(t1.tv_nsec - t_start.tv_nsec) / 1e9, ULL(trace_call_count() - calls_at_start),
           ULL(trace_distinct_count() - distinct_at_start));
    print_shadow();
    printf("b2d: guest registers at the stop (R[], eager):\n");
    for (i = 0; i < 16; i += 4)
        printf("b2d:   %s=%#llx %s=%#llx %s=%#llx %s=%#llx\n", rn[i], ULL(R[i]), rn[i + 1], ULL(R[i + 1]), rn[i + 2], ULL(R[i + 2]), rn[i + 3], ULL(R[i + 3]));
    n = nev < 24 ? nev : 24;
    printf("b2d: last %u memory events of the guest (newest first); %u executable of %u:\n", n, nexec, nev);
    for (k = 0; k < n; k++) {
        const mem_ev *e = &evlog[(nev - 1 - k) % EVLOG];
        printf("b2d:   [%s] %s(%#llx, %#llx, %#llx, %#llx) %s caller=%#llx\n", e->step, e->api, ULL(e->a[0]), ULL(e->a[1]), ULL(e->a[2]), ULL(e->a[3]),
               e->exec ? "EXEC" : "non-exec", ULL(e->caller));
    }
    if (pix) {
        unsigned changed = 0;
        for (i = 0; i < (size_t)W * H * 4; i++) changed += pix[i] != 0;
        printf("b2d: pixel buffer at %p: %u of %d bytes are non-zero; first pixel bytes %02x %02x %02x %02x\n", (void *)pix, changed, W * H * 4, pix[0],
               pix[1], pix[2], pix[3]);
    }
    fflush(stdout);
#ifdef __GLIBC__
    {
        void *bt[64];
        int nb = backtrace(bt, 64);
        printf("b2d: C stack of the harness at the stop (backtrace(); the lines below are written by backtrace_symbols_fd and have no \"b2d:\" prefix):\n");
        fflush(stdout);
        backtrace_symbols_fd(bt, nb, 1);
    }
#endif
    printf("b2d: exit code %d (130+: n/a; 134 SIGABRT, 139 SIGSEGV, 135 SIGBUS, 132 SIGILL, 136 SIGFPE, 4 wall time, 3 entry limit)\n", code);
    fflush(stdout);
}

static void finish(int code) {
    fflush(stdout);
    if (cfg_trap) {
        fprintf(stderr, "b2d: FLE_B2D_TRAP=1: raising SIGTRAP; under gdb `bt` shows the C stack\n");
        raise(SIGTRAP);
    }
    _exit(code);
}

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
        snprintf(reason, sizeof reason, "limit: wall time %llu s after the probe began (FLE_B2D_SECONDS)", ULL(cfg_seconds));
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

static void on_calls_limit(void) {
    char reason[200];
    snprintf(reason, sizeof reason, "limit: %llu lifted function entries after the probe began (FLE_B2D_MAX_CALLS)", ULL(cfg_max_calls));
    report(reason, 3);
    finish(3);
}

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

/* ---- configuration ---- */
static int parse_num(const char *name, uint64_t lo, uint64_t hi, uint64_t *out) {
    const char *s = getenv(name);
    char *end;
    unsigned long long v;
    if (!s || !*s) return 0;
    errno = 0;
    v = strtoull(s, &end, 10);
    if (errno != 0 || end == s || *end != 0 || v < lo || v > hi) {
        fprintf(stderr, "b2d: %s=\"%s\" is not a decimal number from %llu to %llu\n", name, s, ULL(lo), ULL(hi));
        return -1;
    }
    *out = v;
    return 1;
}

static int op_known(const char *s) {
    static const char *const known[] = {"clear", "rect", "blend", "line", "path", "ellipse"};
    size_t i;
    for (i = 0; i < sizeof known / sizeof known[0]; i++)
        if (strcmp(s, known[i]) == 0) return 1;
    return 0;
}

int b2d_probe_precheck(void) {
    static const char *const others[] = {"FLE_B2D_MODE", "FLE_B2D_OPS", "FLE_B2D_MAX_CALLS", "FLE_B2D_SECONDS", "FLE_B2D_TRAP", "FLE_B2D_TLS"};
    const char *e = getenv("FLE_BLEND2D_PROBE");
    const char *s;
    size_t i;
    if (e && *e && strcmp(e, "0") != 0 && strcmp(e, "1") != 0) {
        fprintf(stderr, "b2d: FLE_BLEND2D_PROBE=\"%s\": only unset, empty, 0 or 1 are defined\n", e);
        return -1;
    }
    cfg_on = (e && strcmp(e, "1") == 0);
    if (!cfg_on) {
        for (i = 0; i < sizeof others / sizeof others[0]; i++) {
            s = getenv(others[i]);
            if (s && *s) {
                fprintf(stderr, "b2d: %s is set but FLE_BLEND2D_PROBE=1 is not: refused (nothing runs)\n", others[i]);
                return -1;
            }
        }
        return 0;
    }
    if (parse_num("FLE_B2D_MAX_CALLS", 1, UINT64_C(1) << 40, &cfg_max_calls) < 0) return -1;
    if (parse_num("FLE_B2D_SECONDS", 1, 3600, &cfg_seconds) < 0) return -1;
    s = getenv("FLE_B2D_TRAP");
    if (s && *s) {
        if (strcmp(s, "0") != 0 && strcmp(s, "1") != 0) {
            fprintf(stderr, "b2d: FLE_B2D_TRAP=\"%s\": only 0 or 1 are defined\n", s);
            return -1;
        }
        cfg_trap = (strcmp(s, "1") == 0);
    }
    s = getenv("FLE_B2D_TLS");
    if (s && *s) {
        if (strcmp(s, "0") != 0 && strcmp(s, "1") != 0) {
            fprintf(stderr, "b2d: FLE_B2D_TLS=\"%s\": only 0 or 1 are defined\n", s);
            return -1;
        }
        cfg_tls = (strcmp(s, "1") == 0);
    }
    s = getenv("FLE_B2D_MODE");
    if (s && *s) {
        if (strcmp(s, "agg") == 0) cfg_nojit = 0;
        else if (strcmp(s, "nojit") == 0) cfg_nojit = 1;
        else {
            fprintf(stderr, "b2d: FLE_B2D_MODE=\"%s\": only agg or nojit are defined\n", s);
            return -1;
        }
    }
    s = getenv("FLE_B2D_OPS");
    if (!s || !*s) s = "clear,rect,blend,line,path,ellipse";
    if (strcmp(s, "none") != 0) {
        char *tok, *save = NULL;
        if (strlen(s) >= sizeof ops_buf) {
            fprintf(stderr, "b2d: FLE_B2D_OPS is longer than %zu characters\n", sizeof ops_buf - 1);
            return -1;
        }
        snprintf(ops_buf, sizeof ops_buf, "%s", s);
        for (tok = strtok_r(ops_buf, ",", &save); tok; tok = strtok_r(NULL, ",", &save)) {
            if (!op_known(tok)) {
                fprintf(stderr, "b2d: FLE_B2D_OPS names \"%s\": the steps are clear, rect, blend, line, path, ellipse (or the single word none)\n", tok);
                return -1;
            }
            if (nops == 32) {
                fprintf(stderr, "b2d: FLE_B2D_OPS names more than 32 steps\n");
                return -1;
            }
            ops[nops++] = tok;
        }
    }
    return 0;
}

/* ---- calling the guest ---- */
static uint64_t gcall(uint64_t va, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, int nstk, const uint64_t *stk) {
    /* arguments 5.. of the Microsoft x64 call: fle_call_guest puts the callee's rsp at ((R[4] - 32) & ~15) - 8, so argument n >= 5 is at
     * call_sp + 32 + 8 * (n - 5), call_sp = (R[4] - 32) & ~15; the words are the harness stack above R[4] (free: the harness leaves 64 bytes) */
    uint64_t call_sp = (R[4] - FLE_CB_SHADOW_BYTES) & ~UINT64_C(15);
    int i;
    for (i = 0; i < nstk; i++) fle_st(call_sp + FLE_CB_SHADOW_BYTES + 8u * (unsigned)i, 8, stk[i]);
    return fle_call_guest(va, a1, a2, a3, a4);
}

static uint64_t d2u(double d) {
    uint64_t u;
    memcpy(&u, &d, sizeof u);
    return u;
}

static void setx(unsigned n, double d) {
    X[n].u64[0] = d2u(d);
    X[n].u64[1] = 0;
}

static uint64_t slotfn(uint64_t obj, unsigned slot) { return fle_ld(fle_ld(obj, 8) + 8u * slot, 8); }

/* Agg2D calls: this = obj. Colour arguments are bytes in edx, r8d, r9d and the 5th argument. */
static void agg_color4(uint64_t obj, unsigned slot, unsigned r, unsigned g, unsigned b, unsigned a) {
    uint64_t stk[1] = {a};
    gcall(slotfn(obj, slot), obj, r, g, b, 1, stk);
}

static void agg_nop(uint64_t obj, unsigned slot) { gcall(slotfn(obj, slot), obj, 0, 0, 0, 0, NULL); }

static void agg_d4(uint64_t obj, unsigned slot, double x1, double y1, double x2, double y2) {
    uint64_t stk[1] = {d2u(y2)};
    setx(1, x1);
    setx(2, y1);
    setx(3, x2);
    gcall(slotfn(obj, slot), obj, 0, 0, 0, 1, stk);
}

static void agg_d2(uint64_t obj, unsigned slot, double x, double y) {
    setx(1, x);
    setx(2, y);
    gcall(slotfn(obj, slot), obj, 0, 0, 0, 0, NULL);
}

static void agg_d1(uint64_t obj, unsigned slot, double x) {
    setx(1, x);
    gcall(slotfn(obj, slot), obj, 0, 0, 0, 0, NULL);
}

/* ---- the pixel buffer ---- */
static uint64_t fnv(const uint8_t *p, size_t n) {
    uint64_t h = UINT64_C(0xcbf29ce484222325);
    size_t i;
    for (i = 0; i < n; i++) h = (h ^ p[i]) * UINT64_C(0x100000001b3);
    return h;
}

static void px(const char *label, unsigned x, unsigned y) {
    const uint8_t *p = pix + (size_t)y * STRIDE + (size_t)x * 4;
    printf("b2d:   pixel(%u,%u) %-9s bytes %02x %02x %02x %02x (B G R A order if PRGB32)\n", x, y, label, p[0], p[1], p[2], p[3]);
}

static unsigned count_changed(const uint8_t *before) {
    unsigned n = 0, i;
    for (i = 0; i < (unsigned)(W * H); i++)
        if (memcmp(pix + 4u * i, before + 4u * i, 4) != 0) n++;
    return n;
}

typedef struct {
    uint64_t sum;
    uint8_t copy[W * H * 4];
} snap;
static snap snap_before;

static void step_begin(const char *name) {
    cur_step = name;
    memcpy(snap_before.copy, pix, sizeof snap_before.copy);
    snap_before.sum = fnv(pix, (size_t)W * H * 4);
    printf("b2d: step %s: begin (buffer fnv1a %#llx)\n", name, ULL(snap_before.sum));
    fflush(stdout);
}

static unsigned ev_at_step_begin;
static uint64_t calls_at_step_begin;

static void step_end(const char *name) {
    uint64_t sum = fnv(pix, (size_t)W * H * 4);
    unsigned ch = count_changed(snap_before.copy);
    printf("b2d: step %s: returned; buffer fnv1a %#llx (%s); %u of %d pixels changed; lifted entries %llu; memory events %u (executable %u so far)\n", name,
           ULL(sum), sum == snap_before.sum ? "UNCHANGED" : "changed", ch, W * H, ULL(trace_call_count() - calls_at_step_begin), nev - ev_at_step_begin, nexec);
    fflush(stdout);
}

#define STEP_BEGIN(n)                           \
    do {                                        \
        ev_at_step_begin = nev;                 \
        calls_at_step_begin = trace_call_count(); \
        step_begin(n);                          \
    } while (0)

static void run_op(const char *op, uint64_t obj) {
    if (strcmp(op, "clear") == 0) {
        STEP_BEGIN("clear");
        agg_color4(obj, S_CLEARALL, 0x11, 0x22, 0x33, 0xff);
        step_end("clear");
        px("origin", 0, 0);
        px("centre", 32, 32);
        px("last", 63, 63);
    } else if (strcmp(op, "rect") == 0) {
        STEP_BEGIN("rect");
        agg_color4(obj, S_FILLCOLOR, 0xff, 0, 0, 0xff);
        agg_nop(obj, S_NOLINE);
        agg_d4(obj, S_RECT, 8, 8, 40, 40);
        step_end("rect");
        px("inside", 20, 20);
        px("outside", 50, 50);
        px("corner", 8, 8);
    } else if (strcmp(op, "blend") == 0) {
        STEP_BEGIN("blend");
        agg_color4(obj, S_FILLCOLOR, 0, 0, 0xff, 0x80);
        agg_nop(obj, S_NOLINE);
        agg_d4(obj, S_RECT, 24, 24, 56, 56);
        step_end("blend");
        px("overlap", 30, 30);
        px("only-new", 50, 50);
    } else if (strcmp(op, "line") == 0) {
        STEP_BEGIN("line");
        agg_color4(obj, S_LINECOLOR, 0, 0xff, 0, 0xff);
        agg_d1(obj, S_LINEWIDTH, 3.0);
        agg_d4(obj, S_LINE, 2, 60, 60, 2);
        step_end("line");
        px("on-line", 31, 31);
        px("off-line", 5, 5);
    } else if (strcmp(op, "path") == 0) {
        STEP_BEGIN("path");
        agg_color4(obj, S_FILLCOLOR, 0, 0xff, 0xff, 0xff);
        agg_nop(obj, S_NOLINE);
        agg_nop(obj, S_RESETPATH);
        agg_d2(obj, S_MOVETO, 10, 50);
        agg_d2(obj, S_LINETO, 50, 50);
        agg_d2(obj, S_LINETO, 30, 10);
        agg_nop(obj, S_CLOSEPOLY);
        gcall(slotfn(obj, S_DRAWPATH), obj, 0, 0, 0, 0, NULL); /* flag 0 = fill only */
        step_end("path");
        px("inside", 30, 40);
        px("outside", 5, 5);
    } else if (strcmp(op, "ellipse") == 0) {
        STEP_BEGIN("ellipse");
        agg_color4(obj, S_FILLCOLOR, 0xff, 0xff, 0, 0xff);
        agg_nop(obj, S_NOLINE);
        agg_d4(obj, S_ELLIPSE, 32, 32, 20, 12);
        step_end("ellipse");
        px("centre", 32, 32);
        px("outside", 2, 2);
    }
}

/* ---- EXPERIMENT for gdb (never called by the probe itself): run machine code that the guest generated in an executable region, on the host CPU, with the
 * Microsoft x64 convention and the guest's argument registers. Used from gdb at a dispatch miss: `call b2d_native_call(<target>)`, then store the result in R[0]
 * and pop the two frames of fle_dispatch_miss / fle_dispatch. It shows what the generated code computes; it is NOT a way to run the product in a browser. ---- */
typedef uint64_t (__attribute__((ms_abi)) *b2d_ms_fn)(uint64_t, uint64_t, uint64_t, uint64_t);
uint64_t b2d_native_call(uint64_t va);
uint64_t b2d_native_call(uint64_t va) {
    b2d_ms_fn f = (b2d_ms_fn)(uintptr_t)va;
    return f(R[1], R[2], R[8], R[9]);
}

/* ---- the call ---- */
int b2d_probe_run(uint64_t entry_sp) {
    int idx;
    uint64_t agg_va, obj, vt, vtb, stride_slot[1];
    int rc = 0, i;
    if (!cfg_on) return 0;
    printf("b2d: DllMain returned (the report above is its report). X1 probe: AggCreateInstance, attach(%dx%d, stride %d), then the steps; mode %s; tls scaffold %s\n", W, H,
           STRIDE, cfg_nojit ? "nojit (experiment: the context is replaced by one made with BL_CONTEXT_CREATE_FLAG_DISABLE_JIT)" : "agg (the guest's own attach)",
           cfg_tls ? "ON (experiment: _tls_index and a TLS block are set up the way the Windows loader does)" : "off");
    idx = fle_realmod_find(B2D_NAME);
    if (idx < 0) {
        printf("b2d: no real module \"%s\" in fle_realmod_table\n", B2D_NAME);
        return 2;
    }
    b2d_base = fle_realmod_base_of(idx);
    b2d_size = fle_ld(fle_ld(b2d_base + 0x3c, 4) + b2d_base + 24 + 56, 4);
    agg_va = export_va(b2d_base, b2d_size, "AggCreateInstance");
    if (!agg_va || !have_fn(agg_va)) {
        printf("b2d: the Blend2D image at %#llx has no (non-forwarded, lifted) AggCreateInstance\n", ULL(b2d_base));
        return 2;
    }
    call_va = agg_va;
    printf("b2d: Blend2D base %#llx size %#llx; AggCreateInstance %#llx (RVA %#llx)\n", ULL(b2d_base), ULL(b2d_size), ULL(agg_va), ULL(agg_va - b2d_base));
    {
        unsigned ve, vp, vpe, va = b2d_pre_counts(&ve, &vp, &vpe);
        printf("b2d: before the probe (DllMain and what it loaded): VirtualAlloc %u calls, %u with an executable protection; VirtualProtect %u calls, %u executable\n", va, ve,
               vp, vpe);
    }
    printf("b2d: JIT provider (RVA %#x) slots +8 %#llx +0x10 %#llx +0x18 %#llx (non-zero = initialised by the module's static initialiser during DllMain); fixed provider (RVA %#x) +0x10 %#llx\n",
           RVA_JITRT_PROVIDER, ULL(fle_ld(b2d_base + RVA_JITRT_PROVIDER + 8, 8)), ULL(fle_ld(b2d_base + RVA_JITRT_PROVIDER + 0x10, 8)),
           ULL(fle_ld(b2d_base + RVA_JITRT_PROVIDER + 0x18, 8)), RVA_FIXED_PROVIDER, ULL(fle_ld(b2d_base + RVA_FIXED_PROVIDER + 0x10, 8)));
    if (R[4] != entry_sp) {
        printf("b2d: note: R[4] after DllMain is %#llx, not the %#llx it had before the entry call; the probe uses the latter\n", ULL(R[4]), ULL(entry_sp));
        R[4] = entry_sp;
    }
    pix = aligned_alloc(64, (size_t)W * H * 4);
    if (!pix) {
        printf("b2d: cannot allocate the pixel buffer\n");
        return 2;
    }
    memset(pix, 0, (size_t)W * H * 4);

    if (cfg_tls) {
        /* EXPERIMENT (not in the runtime): what the Windows loader does for a module with a TLS directory. An index that TlsAlloc has not handed out
         * (the table of fle_imports.c has 1088 slots) gets a block initialised from the template; _tls_index is set to it. */
        extern uint64_t fle_tls_values[];
        const unsigned tidx = 1000;
        uint8_t *blk = calloc(1, 0x100);
        if (!blk || fle_tls_values[tidx] != 0) {
            printf("b2d: TLS scaffold: slot %u is not free or no memory\n", tidx);
            return 2;
        }
        memcpy(blk, (const void *)(uintptr_t)(b2d_base + RVA_TLS_START), TLS_TEMPLATE_BYTES);
        fle_tls_values[tidx] = (uint64_t)(uintptr_t)blk;
        fle_st(b2d_base + RVA_TLS_INDEX, 4, tidx);
        printf("b2d: TLS scaffold: _tls_index (RVA %#x) := %u, fle_tls_values[%u] := %p (template %u bytes copied)\n", RVA_TLS_INDEX, tidx, tidx, (void *)blk,
               TLS_TEMPLATE_BYTES);
    }

    depth_at_start = trace_depth();
    calls_at_start = trace_call_count();
    distinct_at_start = trace_distinct_count();
    printf("b2d: before the calls: functions executed (distinct) %llu, entries %llu, shadow depth %zu\n", ULL(distinct_at_start), ULL(calls_at_start), depth_at_start);
    printf("b2d: limits: %llu lifted entries, %llu s wall\n", ULL(cfg_max_calls), ULL(cfg_seconds));
    fflush(stdout);

    clock_gettime(CLOCK_MONOTONIC, &t_start);
    trace_message_reset();
    install_handlers();
    trace_limit_hook = on_calls_limit;
    trace_set_call_limit(calls_at_start + cfg_max_calls);
    alarm((unsigned)cfg_seconds);
    active = 1;

    cur_step = "AggCreateInstance";
    obj = gcall(agg_va, 0, 0, 0, 0, 0, NULL);
    cur_obj = obj;
    vt = obj ? fle_ld(obj, 8) : 0;
    vtb = b2d_base + RVA_AGGVT;
    printf("b2d: AggCreateInstance returned %#llx; object vtable %#llx (%s the Agg2D vtable at RVA %#x); %llu lifted entries so far\n", ULL(obj), ULL(vt),
           vt == vtb ? "is" : "IS NOT", RVA_AGGVT, ULL(trace_call_count() - calls_at_start));
    if (!obj || vt != vtb) {
        active = 0;
        return 3;
    }
    fflush(stdout);

    cur_step = "attach";
    stride_slot[0] = STRIDE;
    gcall(slotfn(obj, S_ATTACH), obj, (uint64_t)(uintptr_t)pix, W, H, 1, stride_slot);
    printf("b2d: attach(%p, %d, %d, stride %d) returned; Agg2D+0x18 (BLContext impl) = %#llx, Agg2D+8 (BLImage impl) = %#llx; %llu lifted entries so far; memory events %u (executable %u)\n",
           (void *)pix, W, H, STRIDE, ULL(fle_ld(obj + 0x18, 8)), ULL(fle_ld(obj + 8, 8)), ULL(trace_call_count() - calls_at_start), nev, nexec);
    {
        uint64_t img = fle_ld(obj + 8, 8);
        printf("b2d: BLImage impl: data %#llx (the buffer is %p), stride %lld, width %llu, height %llu, format/flags/depth dword %#llx\n", ULL(fle_ld(img, 8)),
               (void *)pix, (long long)(int64_t)fle_ld(img + 8, 8), ULL(fle_ld(img + 0x10, 4)), ULL(fle_ld(img + 0x14, 4)), ULL(fle_ld(img + 0x18, 4)));
        printf("b2d: BLContext impl+0x608 (pipe provider the context took) = %#llx (%s)\n", ULL(fle_ld(fle_ld(obj + 0x18, 8) + 0x608, 8)),
               fle_ld(fle_ld(obj + 0x18, 8) + 0x608, 8) == b2d_base + RVA_JITRT_PROVIDER ? "the JIT provider" : fle_ld(fle_ld(obj + 0x18, 8) + 0x608, 8) == b2d_base + RVA_FIXED_PROVIDER ? "the fixed (non-JIT) provider" : "unknown");
    }
    fflush(stdout);

    if (cfg_nojit) {
        /* EXPERIMENT: repeat what attach does after it has made the image, with createInfo.flags = 1 (BL_CONTEXT_CREATE_FLAG_DISABLE_JIT). The old
         * context is leaked on purpose (no release call: not needed for an observation). */
        uint32_t ci[8] = {1, 0, 0, 0, 0, 0, 0, 0};
        uint64_t tmpctx[2] = {0, 0}, impl, vt2, res;
        cur_step = "nojit-context";
        res = gcall(b2d_base + RVA_CTXINITAS, (uint64_t)(uintptr_t)tmpctx, obj + 8, (uint64_t)(uintptr_t)ci, 0, 0, NULL);
        printf("b2d: nojit: blContextInitAs(createInfo.flags=1) returned %#llx, new BLContext = {%#llx, %#llx}\n", ULL(res), ULL(tmpctx[0]), ULL(tmpctx[1]));
        if ((uint32_t)res != 0 || !tmpctx[0]) {
            active = 0;
            return 3;
        }
        fle_st(obj + 0x18, 8, tmpctx[0]);
        fle_st(obj + 0x20, 8, tmpctx[1]);
        impl = fle_ld(obj + 0x18, 8);
        vt2 = fle_ld(impl, 8);
        gcall(fle_ld(vt2 + 0x110, 8), impl, 1, 0, 0, 0, NULL);
        gcall(fle_ld(vt2 + 0x98, 8), impl, 1, 2, 0, 0, NULL);
        printf("b2d: nojit: context replaced; BLContext impl+0x608 = %#llx (%s); memory events %u (executable %u)\n", ULL(fle_ld(impl + 0x608, 8)),
               fle_ld(impl + 0x608, 8) == b2d_base + RVA_FIXED_PROVIDER ? "the fixed (non-JIT) provider" : "NOT the fixed provider", nev, nexec);
        fflush(stdout);
    }

    for (i = 0; i < nops; i++) run_op(ops[i], obj);

    active = 0;
    alarm(0);
    trace_limit_hook = NULL;
    printf("b2d: SUMMARY: steps %d, memory events %u, executable %u (VirtualAlloc %u, CreateFileMappingW %u, MapViewOfFile %u, VirtualProtect %u); lifted entries %llu\n",
           nops, nev, nexec, nexec_by[0], nexec_by[1], nexec_by[2], nexec_by[3], ULL(trace_call_count() - calls_at_start));
    printf("b2d: pixel(0,0) after all steps: %02x %02x %02x %02x\n", pix[0], pix[1], pix[2], pix[3]);
    fflush(stdout);
    return rc;
}
