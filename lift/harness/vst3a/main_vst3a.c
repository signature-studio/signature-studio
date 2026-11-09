/* vst3a copy of main_vst.c (decision 0165, PROVISIONAL): Serum 2 (an instrument, separate controller class, MIDI note events, TLS callback, per-function counters).
 * Minimal VST3 host for a lifted plugin (decision 0156, PROVISIONAL). Maps the plugin's PE at its ImageBase, runs the PE entry (DllMain) and then the VST3
 * module/component/processor calls of the public SDK documentation (the call order of the "Audio Processor Call Sequence" workflow diagram) through the lifted machine
 * code, with the plugin's vtables called as `call [vtable+8*slot]` (fle_dispatch). Native build (build_vst_native.sh); the wasm build uses the same source.
 * usage: vst-native FILE [--blocks N] [--frames F] [--scale S] [--silence] [--param-first V] [--out FILE] [--dump-in FILE] [--rate HZ] [--no-process]
 * Output lines "vst:" are logs; "RESULT" lines are for tools/. Nothing here is a measurement of a real Windows host. */
#define _GNU_SOURCE
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#ifndef __EMSCRIPTEN__
#include <execinfo.h>
#include <signal.h>
#include <sys/time.h>
#include <dlfcn.h>
#endif
#if defined(__x86_64__)
#include <xmmintrin.h>
#endif
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#include <sys/resource.h>
#endif

#include "boot.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"
#include "fle_vmap.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "vst_host.h"
#include "vst3a.h"
#include "fle_countf.h"
#include "fle_mxcsr.h" /* the guest MXCSR (fle_mxcsr): printed at the stages, to tell whether the plugin sets FTZ/DAZ */
#include "vfs_seed.h" /* decision 0110/0129: FLE_VFS_SEED, directories and files written to the virtual volume before the guest runs (off when unset) */

void fle_teb_set_stack(uint64_t lo, uint64_t hi);
extern uint64_t vst_module_base, vst_module_size;
extern uint64_t fle_tls_values[];
void fle_imp_TlsAlloc(void);
void fle_imp_LoadLibraryW(void);
#ifndef VST_RM_NAME
#error "VST_RM_NAME"
#endif
/* The instruction counters of a COUNT build (fle_count.h): all instructions, scalar SSE floating-point ones, other SSE ones. Zero in a build without -DFLE_COUNT. */
uint64_t fle_icount, fle_icount_s, fle_icount_p, fle_icount_x, fle_icount_v;
#ifdef FLE_MX_STATS
void fle_mx_stats_dump(void);
#endif

#define ULL(x) ((unsigned long long)(x))

static double cpu_now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &t);
    return (double)t.tv_sec + 1e-9 * (double)t.tv_nsec;
}
static double wall_now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + 1e-9 * (double)t.tv_nsec;
}
static uint64_t icount(void) { return fle_icount; }

/* export by name from the mapped image's export directory */
static uint64_t export_va(uint64_t base, const char *name)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)base;
    uint32_t lfa, erva, nn, af, an, ao;
    memcpy(&lfa, m + 0x3c, 4);
    memcpy(&erva, m + lfa + 24 + 112, 4);
    if (!erva) return 0;
    memcpy(&nn, m + erva + 24, 4); memcpy(&af, m + erva + 28, 4); memcpy(&an, m + erva + 32, 4); memcpy(&ao, m + erva + 36, 4);
    for (uint32_t i = 0; i < nn; i++) {
        uint32_t nr, fr;
        uint16_t ord;
        memcpy(&nr, m + an + 4 * i, 4);
        memcpy(&ord, m + ao + 2 * i, 2);
        if (strcmp((const char *)m + nr, name) == 0) { memcpy(&fr, m + af + 4 * ord, 4); return base + fr; }
    }
    return 0;
}

/* Implicit TLS of the module (S-vst-tls): the loader's work for a PE with a TLS directory. Index = a slot that the runtime's own TlsAlloc hands out; the slot of the first
 * thread (fle_tls_values, which GS:[0x58] answers) holds a block made of the template and the zero fill; the index is written to AddressOfIndex. Threads that CreateThread
 * makes later start with zeroed slots (fle_imports.c) and would not find the block: the plugin's own threads are not modelled. TLS callbacks are not called (OTT has none). */
uint64_t tls_cb[8];
int ntls_cb;
uint64_t vst3a_tls_idx, vst3a_tls_start, vst3a_tls_len;
static int tls_setup(uint64_t base, uint64_t size)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)base;
    uint32_t lfa, rva, sz, zero;
    uint64_t start, end, idxva, cbva, block, idx;
    size_t n;
    memcpy(&lfa, m + 0x3c, 4);
    memcpy(&rva, m + lfa + 24 + 112 + 9 * 8, 4);
    memcpy(&sz, m + lfa + 24 + 112 + 9 * 8 + 4, 4);
    if (!rva || sz < 0x28) return 0;
    memcpy(&start, m + rva, 8); memcpy(&end, m + rva + 8, 8); memcpy(&idxva, m + rva + 16, 8); memcpy(&cbva, m + rva + 24, 8); memcpy(&zero, m + rva + 32, 4);
    if (end < start || start < base || end > base + size) { fprintf(stderr, "vst: bad TLS directory\n"); exit(2); }
    n = (size_t)(end - start) + zero;
    block = (uint64_t)(uintptr_t)aligned_alloc(64, (n + 63) & ~(size_t)63);
    memset((void *)(uintptr_t)block, 0, (n + 63) & ~(size_t)63);
    memcpy((void *)(uintptr_t)block, (void *)(uintptr_t)start, (size_t)(end - start));
    fle_imp_TlsAlloc();
    idx = (uint32_t)R[0];
    fle_tls_values[idx] = block;
    fle_st(idxva, 4, idx);
    vst3a_tls_idx = idx; vst3a_tls_start = start; vst3a_tls_len = n;
    if (cbva) { for (int k = 0; k < 8 && fle_ld(cbva + 8u * k, 8); k++) { tls_cb[ntls_cb++] = fle_ld(cbva + 8u * k, 8); } }
    printf("vst: TLS callbacks: %d\n", ntls_cb);
    printf("vst: tls raw %llu B + zero %u -> index %llu\n", ULL(end - start), zero, ULL(idx));
    return 1;
}

/* S-vst3a-iatptr: image_load fills every IAT slot with the slot's own VA (harness/image.c), which the lifted `call [IAT]` / `jmp [IAT]` never read (they call the row's function directly).
 * Serum 2's code also loads an IAT slot into a register and calls it later (`mov rdi,[GetModuleHandleW slot]; ...; call rdi`): fle_dispatch(slot VA) then finds no entry. As a loader
 * would write the address of the function, this writes into each slot of an import that has a row the pseudo-VA of that row (the VA GetProcAddress also hands out), which the
 * dispatch table maps to the row's function. Imports without a row (ordinals) keep the slot VA. */
static int iat_fixup(uint64_t base, uint64_t size)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)base;
    uint32_t lfa, imp_rva;
    int nfix = 0, nmiss = 0;
    memcpy(&lfa, m + 0x3c, 4);
    memcpy(&imp_rva, m + lfa + 24 + 112 + 8, 4);
    if (!imp_rva) return 0;
    for (uint32_t off = imp_rva;; off += 20) {
        uint32_t oft, namerva, ft;
        char dll[64];
        memcpy(&oft, m + off, 4); memcpy(&namerva, m + off + 12, 4); memcpy(&ft, m + off + 16, 4);
        if (!oft && !namerva && !ft) break;
        { size_t i; for (i = 0; i < 63 && m[namerva + i]; i++) dll[i] = (char)((m[namerva + i] >= 'A' && m[namerva + i] <= 'Z') ? m[namerva + i] + 32 : m[namerva + i]); dll[i] = 0; }
        for (uint32_t k = 0;; k++) {
            uint64_t th; const char *nm; uint64_t j;
            memcpy(&th, m + (oft ? oft : ft) + 8u * k, 8);
            if (!th) break;
            if (th >> 63) {
                unsigned o;
                for (o = 0; o < vst3a_ord_count; o++)
                    if (!strcmp(vst3a_ord_table[o].dll, dll) && vst3a_ord_table[o].ordinal == (int)(th & 0xffff)) break;
                if (o < vst3a_ord_count) { fle_st(base + ft + 8u * k, 8, vst3a_ord_table[o].va); nfix++; } else nmiss++;
                continue;
            }
            nm = (const char *)m + (th & 0x7fffffffu) + 2;
            for (j = 0; j < fle_resp_count; j++)
                if (!strcmp(fle_resp_table[j].dll, dll) && !strcmp(fle_resp_table[j].name, nm)) break;
            if (j == fle_resp_count) { nmiss++; printf("vst: IAT: no row for %s!%s\n", dll, nm); continue; }
            fle_st(base + ft + 8u * k, 8, fle_resp_table[j].va);
            nfix++;
        }
    }
    (void)size;
    printf("vst: IAT slots set to the row's pseudo-VA: %d (without a row: %d)\n", nfix, nmiss);
    return nfix;
}

static void u16_to_ascii(uint64_t p, char *out, size_t cap)
{
    size_t i;
    for (i = 0; i + 1 < cap; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2 * i, 2);
        if (!c) break;
        out[i] = c < 0x80 ? (char)c : '?';
    }
    out[i] = 0;
}

static void hexid(uint64_t p, char *out)
{
    for (int i = 0; i < 16; i++) snprintf(out + 2 * i, 3, "%02X", (unsigned)fle_ld(p + i, 1));
}

typedef struct { uint32_t id; char title[64]; double def; int32_t steps; int32_t flags; } pinfo;

typedef struct {
    int32_t mode, ssize, maxs; int32_t pad; double rate;
} proc_setup;
_Static_assert(sizeof(proc_setup) == 24, "ProcessSetup");
/* guest structures: every pointer is a 64-bit guest address (also in the wasm32 build, where a host pointer has 32 bits) */
typedef struct { int32_t nch; int32_t pad; uint64_t silence; uint64_t buf; } abb;
_Static_assert(sizeof(abb) == 24, "AudioBusBuffers");
typedef struct {
    int32_t mode, ssize, nsamples, nin, nout; int32_t pad;
    uint64_t in, out, inpc, outpc, inev, outev, ctx;
} proc_data;
_Static_assert(sizeof(proc_data) == 80, "ProcessData");
typedef struct {
    uint32_t state; uint32_t pad0; double rate; int64_t proj_samples; int64_t sys_time; int64_t cont_samples; double proj_music, bar_music, cyc_start, cyc_end, tempo;
    int32_t ts_num, ts_den; uint8_t chord[4]; int32_t smpte_off; uint32_t fps, fps_flags; int32_t to_clock; int32_t pad1;
} proc_ctx;
_Static_assert(sizeof(proc_ctx) == 112, "ProcessContext");

static uint64_t fnv(uint64_t h, const void *p, size_t n)
{
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= UINT64_C(0x100000001b3); }
    return h;
}

/* Per-function instruction counts of a COUNT build (count_fn.py): fle_fn_* are indexed by the function number of fnlist.tsv. fprof_snap() remembers the counters;
 * fprof_dump(what, file) writes the difference since the last snapshot (or since the start) as rows "fid calls n s p x v" sorted by n and prints the summary. */
#ifdef FLE_COUNT
#include "fle_countf_ext.h"
static uint64_t *snap[6];
static void fprof_snap(void)
{
    uint64_t *cur[6] = { fle_fn_n, fle_fn_s, fle_fn_p, fle_fn_x, fle_fn_v, fle_fn_calls };
    for (int k = 0; k < 6; k++) { if (!snap[k]) snap[k] = calloc(fle_fn_nf, 8); memcpy(snap[k], cur[k], (size_t)fle_fn_nf * 8); }
}
typedef struct { uint32_t fid; uint64_t n, s, p, x, v, calls; } frow;
static int frow_cmp(const void *a, const void *b) { uint64_t x = ((const frow *)a)->n, y = ((const frow *)b)->n; return x < y ? 1 : x > y ? -1 : 0; }
static void fprof_dump(const char *what, const char *file)
{
    uint64_t *cur[6] = { fle_fn_n, fle_fn_s, fle_fn_p, fle_fn_x, fle_fn_v, fle_fn_calls };
    frow *rows = malloc((size_t)fle_fn_nf * sizeof *rows);
    size_t nr = 0, nfn_called = 0;
    uint64_t tot = 0, tx = 0, tv = 0, ts = 0, tp = 0, tc = 0;
    for (uint32_t i = 0; i < fle_fn_nf; i++) {
        uint64_t d[6];
        for (int k = 0; k < 6; k++) d[k] = cur[k][i] - (snap[k] ? snap[k][i] : 0);
        if (!d[0] && !d[5]) continue;
        rows[nr++] = (frow){ i, d[0], d[1], d[2], d[3], d[4], d[5] };
        tot += d[0]; ts += d[1]; tp += d[2]; tx += d[3]; tv += d[4]; tc += d[5];
        nfn_called++;
    }
    qsort(rows, nr, sizeof *rows, frow_cmp);
    printf("RESULT fprof_%s functions=%zu instructions=%llu scalar_sse=%llu packed_sse=%llu x87=%llu avx=%llu calls=%llu\n", what, nfn_called, ULL(tot), ULL(ts), ULL(tp), ULL(tx), ULL(tv), ULL(tc));
    {
        uint64_t cum = 0; size_t k90 = 0, k99 = 0, k999 = 0, k9999 = 0;
        for (size_t i = 0; i < nr; i++) {
            cum += rows[i].n;
            if (!k90 && cum * 10 >= tot * 9) k90 = i + 1;
            if (!k99 && cum * 100 >= tot * 99) k99 = i + 1;
            if (!k999 && cum * 1000 >= tot * 999) k999 = i + 1;
            if (!k9999 && cum * 10000 >= tot * 9999) k9999 = i + 1;
        }
        printf("RESULT fprof_%s_cover n90=%zu n99=%zu n999=%zu n9999=%zu\n", what, k90, k99, k999, k9999);
    }
    if (file) {
        FILE *f = fopen(file, "w");
        if (f) { fprintf(f, "fid\tcalls\tn\tscalar_sse\tpacked_sse\tx87\tavx\n"); for (size_t i = 0; i < nr; i++) fprintf(f, "%u\t%llu\t%llu\t%llu\t%llu\t%llu\t%llu\n", rows[i].fid, ULL(rows[i].calls), ULL(rows[i].n), ULL(rows[i].s), ULL(rows[i].p), ULL(rows[i].x), ULL(rows[i].v)); fclose(f); }
    }
    free(rows);
    fprof_snap();
}
#else
static void fprof_snap(void) {}
static void fprof_dump(const char *what, const char *file) { (void)what; (void)file; }
#endif

/* S-vst3a-registry: a REG_SZ value in the runtime's in-memory registry, created through the guest-facing stubs (as vfs_seed.h calls the VFS stubs): RegCreateKeyExW(root, subkey, 0, NULL, 0,
 * KEY_ALL_ACCESS-less KEY_SET_VALUE|KEY_CREATE_SUB_KEY, NULL, &h, NULL), RegSetValueExW(h, name, 0, REG_SZ, data, bytes), RegCloseKey. R[] and the stack pointer are put back. */
void fle_imp_RegCreateKeyExW(void);
void fle_imp_RegSetValueExW(void);
void fle_imp_RegCloseKey(void);
static uint64_t seed_call(void (*fn)(void), const uint64_t *a)
{
    static uint64_t stk[32];
    memset(stk, 0, sizeof stk);
    stk[0] = HARNESS_SENTINEL_RET;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (int i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0;
    fn();
    return R[0];
}
static void w_of(uint16_t *d, const char *s) { size_t i; for (i = 0; s[i]; i++) d[i] = (unsigned char)s[i]; d[i] = 0; }
static int seed_registry_sz(uint64_t root, const char *subkey, const char *name, const char *value)
{
    static uint16_t wsub[300], wname[100], wval[600];
    static uint64_t hk;
    uint64_t save4 = R[4], r;
    w_of(wsub, subkey); w_of(wname, name); w_of(wval, value);
    r = seed_call(fle_imp_RegCreateKeyExW, (const uint64_t[12]){ root, (uint64_t)(uintptr_t)wsub, 0, 0, 0, 0x6, 0, (uint64_t)(uintptr_t)&hk, 0 });
    if (r != 0) { fprintf(stderr, "vst: RegCreateKeyExW -> %llu\n", ULL(r)); R[4] = save4; return -1; }
    r = seed_call(fle_imp_RegSetValueExW, (const uint64_t[12]){ hk, (uint64_t)(uintptr_t)wname, 0, 1, (uint64_t)(uintptr_t)wval, 2 * (strlen(value) + 1) });
    if (r != 0) { fprintf(stderr, "vst: RegSetValueExW -> %llu\n", ULL(r)); R[4] = save4; return -1; }
    seed_call(fle_imp_RegCloseKey, (const uint64_t[12]){ hk });
    R[4] = save4;
    printf("vst: registry seeded: HKLM\\%s\\%s = \"%s\"\n", subkey, name, value);
    return 0;
}

#ifndef __EMSCRIPTEN__
/* VST3A_SAMPLE=<file>: a sampling profile of the process phase (ITIMER_PROF every 1 ms of CPU time; the handler stores the top 3 return addresses, relative to the load address of this
 * executable). The file gets one line per sample "off0 off1 off2" in hex; tools/resolve_samples.py maps them to the lifted functions (f_<VA> symbols) with nm. Diagnostic only. */
#define SAMP_MAX 400000
static uint32_t samp_off[SAMP_MAX][3];
static volatile unsigned samp_n;
static uintptr_t samp_base;
static void samp_handler(int sig)
{
    void *fr[5];
    int n, k;
    unsigned i = samp_n;
    (void)sig;
    if (i >= SAMP_MAX) return;
    n = backtrace(fr, 5);
    for (k = 0; k < 3; k++) samp_off[i][k] = (k + 2 < n && (uintptr_t)fr[k + 2] >= samp_base) ? (uint32_t)((uintptr_t)fr[k + 2] - samp_base) : 0;
    samp_n = i + 1;
}
static void samp_start(void)
{
    Dl_info di;
    struct itimerval it = { { 0, 1000 }, { 0, 1000 } };
    void *fr[2];
    if (!dladdr((void *)samp_start, &di)) return;
    samp_base = (uintptr_t)di.dli_fbase;
    backtrace(fr, 2); /* load libgcc now, not in the handler */
    signal(SIGPROF, samp_handler);
    setitimer(ITIMER_PROF, &it, NULL);
}
static void samp_stop(const char *file)
{
    struct itimerval it = { { 0, 0 }, { 0, 0 } };
    FILE *f;
    setitimer(ITIMER_PROF, &it, NULL);
    f = fopen(file, "w");
    if (!f) return;
    for (unsigned i = 0; i < samp_n; i++) fprintf(f, "%x %x %x\n", samp_off[i][0], samp_off[i][1], samp_off[i][2]);
    fclose(f);
    printf("vst: %u profile samples written to %s\n", samp_n, file);
}
#endif

/* ---- --multi LIST: several instances of the plugin in one process, as the reference project has (6 Serum 2). LIST = state indexes (files state/sN_c3.bin and sN_c2.bin; -1 = no state), e.g. 0,1,3,4,-1,5.
 * Each instance: createInstance(IComponent), initialize, IAudioProcessor, a separate controller (createInstance + initialize), IConnectionPoint connect, setState (processor state; the controller states
 * only when the primary run did them: not here), the buses, setupProcessing, setActive, setProcessing. Then the same note is sent to every instance and one round = process of each instance in turn
 * (the CPU time of the round is what a single audio thread needs for all of them). ---- */
typedef struct { uint64_t comp, proc, ctrl; float *ob[2]; uint64_t op[2]; abb obs; proc_data pd; double cpu_med; int state; } mi_inst;
static int mi_run(uint64_t fac, const char *cid, vst_host *host, proc_setup *ps, const char *list, const char *state_dir, int frames, double rate, int blocks, int note_on, const int *notes, int nnotes, int yield_ms)
{
    mi_inst in[16];
    int n = 0;
    char *tmp = strdup(list), *t;
    double c_all0 = cpu_now(), c_after_init;
    uint64_t i_all0 = icount();
    proc_ctx *pc = calloc(1, sizeof *pc);
    for (t = strtok(tmp, ","); t && n < 16; t = strtok(NULL, ",")) {
        mi_inst *x = &in[n];
        uint64_t pobj = (uint64_t)(uintptr_t)calloc(1, 16), r, tuid = (uint64_t)(uintptr_t)calloc(1, 16);
        double c0 = cpu_now();
        uint64_t i0 = icount();
        memset(x, 0, sizeof *x);
        x->state = atoi(t);
        fle_st(pobj, 8, 0);
        r = vcall(fac, 6, (uint64_t)(uintptr_t)cid, (uint64_t)(uintptr_t)IID_IComponent, pobj, 0);
        x->comp = fle_ld(pobj, 8);
        if ((r & 0xffffffffu) || !x->comp) { printf("vst: multi: createInstance failed\n"); return 3; }
        vcall(x->comp, 3, host->host_app, 0, 0, 0);
        fle_st(pobj, 8, 0);
        vcall(x->comp, 0, (uint64_t)(uintptr_t)IID_IAudioProcessor, pobj, 0, 0);
        x->proc = fle_ld(pobj, 8);
        r = vcall(x->comp, 5, tuid, 0, 0, 0);
        if ((r & 0xffffffffu) == 0) {
            fle_st(pobj, 8, 0);
            vcall(fac, 6, tuid, (uint64_t)(uintptr_t)IID_IEditController, pobj, 0);
            x->ctrl = fle_ld(pobj, 8);
            if (x->ctrl) vcall(x->ctrl, 3, host->host_app, 0, 0, 0);
        }
        if (x->ctrl) {
            vst_iid cpid; uint64_t po = (uint64_t)(uintptr_t)calloc(1, 16), cpc, cpe;
            vst_iid_make(cpid, 0x70A4156F, 0x6E6E4026, 0x989148BF, 0xAA60D8D1);
            vcall(x->comp, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpc = fle_ld(po, 8);
            fle_st(po, 8, 0);
            vcall(x->ctrl, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpe = fle_ld(po, 8);
            if (cpc && cpe) { vcall(cpc, 3, cpe, 0, 0, 0); vcall(cpe, 3, cpc, 0, 0, 0); }
            vcall(x->ctrl, 16, host->comp_handler, 0, 0, 0);
        }
        if (x->state >= 0) {
            char fn[512];
            FILE *f;
            uint8_t *b;
            long ns;
            snprintf(fn, sizeof fn, "%s/s%d_c3.bin", state_dir ? state_dir : "state", x->state);
            f = fopen(fn, "rb");
            if (!f) { fprintf(stderr, "vst: cannot open %s\n", fn); return 2; }
            fseek(f, 0, SEEK_END); ns = ftell(f); fseek(f, 0, SEEK_SET);
            b = malloc((size_t)ns);
            if (fread(b, 1, (size_t)ns, f) != (size_t)ns) return 2;
            fclose(f);
            r = vcall(x->comp, 12, vst3a_stream_new(b, (size_t)ns), 0, 0, 0);
            free(b);
            if (r & 0xffffffffu) printf("vst: multi: setState(%d) -> %#llx\n", x->state, ULL(r & 0xffffffffu));
            if (yield_ms > 0) { void fle_imp_Sleep(void); uint64_t sv4 = R[4]; for (int y = 0; y < yield_ms; y += 10) { R[1] = 10; fle_imp_Sleep(); } R[4] = sv4; }
        }
        {
            uint64_t arr_out = (uint64_t)(uintptr_t)calloc(1, 8);
            fle_st(arr_out, 8, 3);
            vcall(x->comp, 10, 0, 1, 0, 1);   /* activateBus(audio, out, 0, true) */
            vcall(x->comp, 10, 1, 0, 0, 1);   /* activateBus(event, in, 0, true) */
            vcall(x->comp, 10, 1, 1, 0, 1);   /* activateBus(event, out, 0, true) */
            vcall(x->proc, 3, 0, 0, arr_out, 1);
        }
        vcall(x->proc, 7, (uint64_t)(uintptr_t)ps, 0, 0, 0);
        vcall(x->comp, 11, 1, 0, 0, 0);
        vcall(x->proc, 8, 1, 0, 0, 0);
        for (int c = 0; c < 2; c++) { x->ob[c] = calloc((size_t)frames, 4); x->op[c] = (uint64_t)(uintptr_t)x->ob[c]; }
        x->obs.nch = 2; x->obs.silence = 0; x->obs.buf = (uint64_t)(uintptr_t)x->op;
        x->pd.nout = 1; x->pd.out = (uint64_t)(uintptr_t)&x->obs; x->pd.inpc = host->param_changes; x->pd.outpc = host->out_changes; x->pd.inev = host->event_list; x->pd.outev = host->event_list; x->pd.ctx = (uint64_t)(uintptr_t)pc;
        printf("vst: multi: instance %d (state %d) ready  cpu %.1f ms  icount %llu\n", n, x->state, 1e3 * (cpu_now() - c0), ULL(icount() - i0));
        n++;
    }
    free(tmp);
    c_after_init = cpu_now();
    printf("RESULT multi_init instances=%d cpu_ms=%.1f icount=%llu\n", n, 1e3 * (c_after_init - c_all0), ULL(icount() - i_all0));
    pc->rate = rate; pc->tempo = 200.0; pc->ts_num = 4; pc->ts_den = 4; pc->state = (1u << 1) | (1u << 10) | (1u << 13) | (1u << 9);
    {
        double *round = calloc((size_t)blocks, sizeof(double)), *per = calloc((size_t)blocks * (size_t)n, sizeof(double));
        uint64_t n0 = 0, ic_round = 0, ic0;
        double maxabs[16] = {0};
        for (int b = 0; b < blocks; b++) {
            vst_ev_clear();
            if (note_on && b == 0) for (int q = 0; q < nnotes; q++) vst_ev_note(1, notes[q], 100.0f / 127.0f, 0, q + 1);
            pc->proj_samples = (int64_t)n0;
            pc->proj_music = (double)n0 / rate * pc->tempo / 60.0;
            ic0 = icount();
            {
                double t0 = cpu_now();
                for (int k = 0; k < n; k++) {
                    double a = cpu_now();
                    mi_inst *x = &in[k];
                    uint64_t r;
                    x->pd.nsamples = frames;
                    r = vcall(x->proc, 9, (uint64_t)(uintptr_t)&x->pd, 0, 0, 0);
                    per[(size_t)b * (size_t)n + (size_t)k] = cpu_now() - a;
                    if (r & 0xffffffffu) { printf("vst: multi: process returned %#llx (instance %d, block %d)\n", ULL(r & 0xffffffffu), k, b); return 3; }
                    for (int c = 0; c < 2; c++) for (int i = 0; i < frames; i++) { double av = fabs((double)x->ob[c][i]); if (av > maxabs[k]) maxabs[k] = av; }
                }
                round[b] = cpu_now() - t0;
            }
            ic_round += icount() - ic0;
            n0 += (uint64_t)frames;
        }
        {
            double *s = malloc((size_t)blocks * sizeof(double));
            int m = 0;
            for (int b = 1; b < blocks; b++) s[m++] = round[b];
            for (int i = 1; i < m; i++) { double v = s[i]; int j = i - 1; while (j >= 0 && s[j] > v) { s[j + 1] = s[j]; j--; } s[j + 1] = v; }
            printf("RESULT multi_round instances=%d blocks=%d cpu_ms_block0=%.3f cpu_ms_median=%.3f cpu_ms_min=%.3f cpu_ms_max=%.3f block_ms_real=%.4f icount_per_round=%.0f\n", n, blocks, 1e3 * round[0], 1e3 * s[m / 2], 1e3 * s[0], 1e3 * s[m - 1], 1e3 * frames / rate, (double)ic_round / blocks);
            for (int k = 0; k < n; k++) {
                for (int b = 1; b < blocks; b++) s[b - 1] = per[(size_t)b * (size_t)n + (size_t)k];
                for (int i = 1; i < m; i++) { double v = s[i]; int j = i - 1; while (j >= 0 && s[j] > v) { s[j + 1] = s[j]; j--; } s[j + 1] = v; }
                printf("RESULT multi_instance k=%d state=%d cpu_ms_median=%.3f maxabs=%.6f\n", k, in[k].state, 1e3 * s[m / 2], maxabs[k]);
            }
            free(s);
        }
    }
    for (int k = 0; k < n; k++) {
        vcall(in[k].proc, 8, 0, 0, 0, 0);
        vcall(in[k].comp, 11, 0, 0, 0, 0);
        vcall(in[k].comp, 4, 0, 0, 0, 0);
        if (in[k].ctrl) { vcall(in[k].ctrl, 4, 0, 0, 0, 0); vcall(in[k].ctrl, 2, 0, 0, 0, 0); }
        vcall(in[k].proc, 2, 0, 0, 0, 0);
        vcall(in[k].comp, 2, 0, 0, 0, 0);
    }
    return 0;
}

int main(int argc, char **argv)
{
    const char *path = NULL, *outfile = NULL, *infile = NULL;
    int blocks = 40, frames = 4096, do_silence = 0, do_process = 1, note_on = 0, note_off_block = -1, no_connect = 0;
    const char *want_class = NULL, *state_dir = NULL, *presets_dir = NULL, *multi_list = NULL;
    int yield_ms = 0;
    int state_proc_only = 0;
    int note_list[16] = {60}, nnotes = 1;
    int state_idx = -1;
    double scale = 1.0, rate = 44100.0, pfirst = -1.0;
    harness_image img;
    char err[256];
    vst_host host;
    uint8_t *stk;
    double t_start, t_dll, t_init, t_active;
    uint64_t ic_start, ic_dll, ic_init, ic_active;
    uint64_t fac, comp = 0, proc = 0, ctrl = 0;
    int ncls, audio_cls = -1;
    char cid[16];
    int ctrl_same = 0;

    setvbuf(stdout, NULL, _IOLBF, 0);
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--blocks") && i + 1 < argc) blocks = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--frames") && i + 1 < argc) frames = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--scale") && i + 1 < argc) scale = atof(argv[++i]);
        else if (!strcmp(argv[i], "--rate") && i + 1 < argc) rate = atof(argv[++i]);
        else if (!strcmp(argv[i], "--silence")) do_silence = 1;
        else if (!strcmp(argv[i], "--no-process")) do_process = 0;
        else if (!strcmp(argv[i], "--note")) note_on = 1;
        else if (!strcmp(argv[i], "--notes") && i + 1 < argc) {   /* e.g. 60,64,67,72: all notes on at block 0 (note-off at --note-off-block) */
            char *t = strtok(argv[++i], ","); nnotes = 0; note_on = 1;
            while (t && nnotes < 16) { note_list[nnotes++] = atoi(t); t = strtok(NULL, ","); }
        }
        else if (!strcmp(argv[i], "--note-off-block") && i + 1 < argc) note_off_block = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--class") && i + 1 < argc) want_class = argv[++i];
        else if (!strcmp(argv[i], "--no-connect")) no_connect = 1;
        else if (!strcmp(argv[i], "--yield-ms") && i + 1 < argc) yield_ms = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--multi") && i + 1 < argc) multi_list = argv[++i];
        else if (!strcmp(argv[i], "--state-proc-only")) state_proc_only = 1;
        else if (!strcmp(argv[i], "--presets-dir") && i + 1 < argc) presets_dir = argv[++i];
        else if (!strcmp(argv[i], "--state") && i + 1 < argc) state_idx = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--state-dir") && i + 1 < argc) state_dir = argv[++i];
        else if (!strcmp(argv[i], "--param-first") && i + 1 < argc) pfirst = atof(argv[++i]);
        else if (!strcmp(argv[i], "--out") && i + 1 < argc) outfile = argv[++i];
        else if (!strcmp(argv[i], "--dump-in") && i + 1 < argc) infile = argv[++i];
        else if (argv[i][0] != '-' && !path) path = argv[i];
        else { fprintf(stderr, "usage: %s FILE [--blocks N] [--frames F] [--scale S] [--silence] [--param-first V] [--out FILE] [--dump-in FILE] [--rate HZ] [--no-process]\n", argv[0]); return 64; }
    }
    if (!path) { fprintf(stderr, "usage: %s FILE ...\n", argv[0]); return 64; }
#ifndef __EMSCRIPTEN__
    {
        struct rlimit rl = {HARNESS_MAX_AS_BYTES * 2, HARNESS_MAX_AS_BYTES * 2};
        if (!getenv("VST_NO_RLIMIT")) setrlimit(RLIMIT_AS, &rl); /* the guard of the other harnesses; an address-sanitizer build needs the address space */
    }
#endif
    if (image_load(path, &img, err, sizeof err) != 0) { fprintf(stderr, "image: %s\n", err); return 2; }
    fle_vmap_register_image(img.base);
    iat_fixup(img.base, img.size);
    vst_module_base = img.base; vst_module_size = img.size;
    printf("vst: image base=%#llx size=%#llx entry=%#llx imports=%zu exports=%zu\n", ULL(img.base), ULL(img.size), ULL(img.entry), img.nimports, img.nexports);
#ifndef __EMSCRIPTEN__
    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) { perror("mmap stack"); return 2; }
#else
    stk = aligned_alloc(4096, HARNESS_STACK_BYTES);
#endif
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES);
    fle_vmap_register_range((uint64_t)(uintptr_t)stk, HARNESS_STACK_BYTES, 0x04u, 0x20000u);
    tls_setup(img.base, img.size);
    if (vfs_seed_from_env() != 0) return 2;
    vst_host_init(&host);

    if (presets_dir) { if (seed_registry_sz(UINT64_C(0xFFFFFFFF80000002), "Software\\XferRecords\\Serum2InstallerPrefs", "PresetsDir", presets_dir) != 0) return 2; }
    t_start = cpu_now(); ic_start = icount();
    /* ---- the module's TLS callbacks with DLL_PROCESS_ATTACH, before DllMain (S-vst3a-tlscb: the loader does it; here it runs before LoadLibraryW, i.e. before the module is registered) ---- */
    for (int k = 0; k < ntls_cb; k++) {
        const uint64_t a[6] = { img.base, 1, 0, 0, 0, 0 };
        uint64_t c0 = icount();
        gcall(tls_cb[k], a, NULL);
        printf("vst: TLS callback %d at %#llx (DLL_PROCESS_ATTACH) returned, icount %llu\n", k, ULL(tls_cb[k]), ULL(icount() - c0));
    }
    /* ---- LoadLibraryW of the plugin: fle_realmod_load runs DllMain(hinst = base, DLL_PROCESS_ATTACH) (decision 0082) ---- */
    {
        uint16_t wname[128];
        const char *nm = VST_RM_NAME;
        uint64_t r;
        size_t i;
        for (i = 0; nm[i]; i++) wname[i] = (unsigned char)nm[i];
        wname[i] = 0;
        R[4] -= 8;
        fle_st(R[4], 8, HARNESS_SENTINEL_RET);
        R[1] = (uint64_t)(uintptr_t)wname;
        fle_imp_LoadLibraryW();
        R[4] += 8;
        r = R[0];
        t_dll = cpu_now(); ic_dll = icount();
        printf("vst: LoadLibraryW(\"%s\") = %#llx (DllMain ran inside)  cpu %.3f ms  icount %llu\n", nm, ULL(r), 1e3 * (t_dll - t_start), ULL(ic_dll - ic_start));
        if (r != img.base) return 3;
    }
    /* ---- InitDll, GetPluginFactory ---- */
    {
        uint64_t va = export_va(img.base, "InitDll"), r;
        const uint64_t z[6] = {0};
        printf("vst: exports InitDll=%#llx GetPluginFactory=%#llx ExitDll=%#llx\n", ULL(va), ULL(export_va(img.base, "GetPluginFactory")), ULL(export_va(img.base, "ExitDll")));
        if (va) { r = gcall(va, z, NULL); printf("vst: InitDll() returned %llu\n", ULL(r & 0xff)); if (!(r & 0xff)) return 3; }
        va = export_va(img.base, "GetPluginFactory");
        fac = gcall(va, z, NULL);
        printf("vst: GetPluginFactory() = %#llx\n", ULL(fac));
        if (!fac) return 3;
    }
    /* ---- the factory ---- */
    {
        uint64_t info = (uint64_t)(uintptr_t)calloc(1, 512), pobj = (uint64_t)(uintptr_t)calloc(1, 16), r;
        char s[512];
        r = vcall(fac, 3, info, 0, 0, 0);
        u16_to_ascii(0, s, 1);
        { char v[65], u[257], e[129]; for (int i = 0; i < 64; i++) v[i] = (char)fle_ld(info + i, 1); v[64] = 0; for (int i = 0; i < 256; i++) u[i] = (char)fle_ld(info + 64 + i, 1); u[256] = 0; for (int i = 0; i < 128; i++) e[i] = (char)fle_ld(info + 320 + i, 1); e[128] = 0;
          printf("vst: getFactoryInfo -> %#llx vendor=\"%s\" url=\"%s\" email=\"%s\" flags=%#x\n", ULL(r), v, u, e, (unsigned)fle_ld(info + 448, 4)); }
        ncls = (int)vcall(fac, 4, 0, 0, 0, 0);
        printf("vst: countClasses() = %d\n", ncls);
        for (int i = 0; i < ncls; i++) {
            char h[40], cat[33], nm[65];
            memset((void *)(uintptr_t)info, 0, 512);
            r = vcall(fac, 5, (uint64_t)i, info, 0, 0);
            hexid(info, h);
            for (int k = 0; k < 32; k++) cat[k] = (char)fle_ld(info + 20 + k, 1); cat[32] = 0;
            for (int k = 0; k < 64; k++) nm[k] = (char)fle_ld(info + 52 + k, 1); nm[64] = 0;
            printf("vst: class %d: getClassInfo -> %#llx cid=%s cardinality=%#x category=\"%s\" name=\"%s\"\n", i, ULL(r), h, (unsigned)fle_ld(info + 16, 4), cat, nm);
            if (!strcmp(cat, "Audio Module Class") && audio_cls < 0 && (!want_class || strstr(nm, want_class))) { audio_cls = i; for (int k = 0; k < 16; k++) cid[k] = (char)fle_ld(info + k, 1); }
        }
        /* IPluginFactory2: getClassInfo2 for the vendor/version/subcategories */
        {
            uint64_t f2 = 0;
            r = vcall(fac, 0, (uint64_t)(uintptr_t)IID_IPluginFactory2, pobj, 0, 0);
            f2 = fle_ld(pobj, 8);
            printf("vst: queryInterface(IPluginFactory2) -> %#llx\n", ULL(r));
            if ((r & 0xffffffffu) == 0 && f2) {
                for (int i = 0; i < ncls; i++) {
                    char sub[129], ven[65], ver[65], sdk[65];
                    memset((void *)(uintptr_t)info, 0, 512);
                    r = vcall(f2, 7, (uint64_t)i, info, 0, 0);
                    for (int k = 0; k < 128; k++) sub[k] = (char)fle_ld(info + 120 + k, 1); sub[128] = 0;
                    for (int k = 0; k < 64; k++) ven[k] = (char)fle_ld(info + 248 + k, 1); ven[64] = 0;
                    for (int k = 0; k < 64; k++) ver[k] = (char)fle_ld(info + 312 + k, 1); ver[64] = 0;
                    for (int k = 0; k < 64; k++) sdk[k] = (char)fle_ld(info + 376 + k, 1); sdk[64] = 0;
                    printf("vst: class %d: getClassInfo2 -> %#llx classFlags=%#x sub=\"%s\" vendor=\"%s\" version=\"%s\" sdk=\"%s\"\n", i, ULL(r), (unsigned)fle_ld(info + 116, 4), sub, ven, ver, sdk);
                }
            }
        }
        if (audio_cls < 0) { printf("vst: no Audio Module Class\n"); return 3; }
        /* createInstance(cid, IComponent) */
        fle_st(pobj, 8, 0);
        r = vcall(fac, 6, (uint64_t)(uintptr_t)cid, (uint64_t)(uintptr_t)IID_IComponent, pobj, 0);
        comp = fle_ld(pobj, 8);
        printf("vst: createInstance(IComponent) -> %#llx component=%#llx\n", ULL(r), ULL(comp));
        if ((r & 0xffffffffu) != 0 || !comp) return 3;
    }
    /* ---- IComponent::initialize(hostContext) ---- */
    {
        uint64_t r = vcall(comp, 3, host.host_app, 0, 0, 0);
        printf("vst: IComponent::initialize(IHostApplication) -> %#llx\n", ULL(r & 0xffffffffu));
        if ((r & 0xffffffffu) != 0) return 3;
    }
    {
        uint64_t pobj = (uint64_t)(uintptr_t)calloc(1, 16), r;
        r = vcall(comp, 0, (uint64_t)(uintptr_t)IID_IAudioProcessor, pobj, 0, 0);
        proc = fle_ld(pobj, 8);
        printf("vst: queryInterface(IAudioProcessor) -> %#llx processor=%#llx\n", ULL(r & 0xffffffffu), ULL(proc));
        if ((r & 0xffffffffu) != 0 || !proc) return 3;
        fle_st(pobj, 8, 0);
        r = vcall(comp, 0, (uint64_t)(uintptr_t)IID_IEditController, pobj, 0, 0);
        ctrl = fle_ld(pobj, 8);
        printf("vst: queryInterface(IEditController) on the component -> %#llx controller=%#llx\n", ULL(r & 0xffffffffu), ULL(ctrl));
        if ((r & 0xffffffffu) == 0 && ctrl) ctrl_same = 1;
        else {
            uint64_t tuid = (uint64_t)(uintptr_t)calloc(1, 16);
            r = vcall(comp, 5, tuid, 0, 0, 0);
            printf("vst: getControllerClassId -> %#llx\n", ULL(r & 0xffffffffu));
            if ((r & 0xffffffffu) == 0) {
                fle_st(pobj, 8, 0);
                r = vcall(fac, 6, tuid, (uint64_t)(uintptr_t)IID_IEditController, pobj, 0);
                ctrl = fle_ld(pobj, 8);
                printf("vst: createInstance(IEditController) -> %#llx controller=%#llx\n", ULL(r & 0xffffffffu), ULL(ctrl));
                if ((r & 0xffffffffu) == 0 && ctrl) {
                    r = vcall(ctrl, 3, host.host_app, 0, 0, 0);
                    printf("vst: IEditController::initialize -> %#llx\n", ULL(r & 0xffffffffu));
                } else ctrl = 0;
            }
        }
    }
    if (ctrl && !ctrl_same && !no_connect) {
        /* S-vst3a-connect: IConnectionPoint (vst/ivstmessage.h, IID 70A4156F-6E6E-4026-9891-48BFAA60D8D1) of component and controller connected to each other, as a host does when the two are separate classes */
        vst_iid cpid; uint64_t po = (uint64_t)(uintptr_t)calloc(1, 16), cpc, cpe, r;
        vst_iid_make(cpid, 0x70A4156F, 0x6E6E4026, 0x989148BF, 0xAA60D8D1);
        r = vcall(comp, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpc = fle_ld(po, 8);
        printf("vst: queryInterface(IConnectionPoint) on the component -> %#llx %#llx\n", ULL(r & 0xffffffffu), ULL(cpc));
        fle_st(po, 8, 0);
        r = vcall(ctrl, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpe = fle_ld(po, 8);
        printf("vst: queryInterface(IConnectionPoint) on the controller -> %#llx %#llx\n", ULL(r & 0xffffffffu), ULL(cpe));
        if (cpc && cpe) {
            r = vcall(cpc, 3, cpe, 0, 0, 0); printf("vst: component->connect(controller) -> %#llx\n", ULL(r & 0xffffffffu));
            r = vcall(cpe, 3, cpc, 0, 0, 0); printf("vst: controller->connect(component) -> %#llx\n", ULL(r & 0xffffffffu));
        }
    }
    t_init = cpu_now(); ic_init = icount();
    printf("vst: factory..initialize cpu %.3f ms  icount %llu\n", 1e3 * (t_init - t_dll), ULL(ic_init - ic_dll));
    /* ---- controller: handler, parameters ---- */
    int nparam = 0, pfirst_idx = -1;
    pinfo *pi = NULL;
    if (ctrl) {
        uint64_t r = vcall(ctrl, 16, host.comp_handler, 0, 0, 0);
        uint64_t info = (uint64_t)(uintptr_t)calloc(1, 800);
        printf("vst: IEditController::setComponentHandler -> %#llx\n", ULL(r & 0xffffffffu));
        nparam = (int)vcall(ctrl, 8, 0, 0, 0, 0);
        printf("vst: getParameterCount() = %d\n", nparam);
        pi = calloc((size_t)(nparam > 0 ? nparam : 1), sizeof *pi);
        for (int i = 0; i < nparam; i++) {
            double d4[4] = {0, 0, 0, 0};
            memset((void *)(uintptr_t)info, 0, 800);
            r = vcall(ctrl, 9, (uint64_t)i, info, 0, 0);
            pi[i].id = (uint32_t)fle_ld(info, 4);
            u16_to_ascii(info + 4, pi[i].title, sizeof pi[i].title);
            pi[i].steps = (int32_t)fle_ld(info + 772, 4);
            memcpy(&pi[i].def, (void *)(uintptr_t)(info + 776), 8);
            pi[i].flags = (int32_t)fle_ld(info + 788, 4);
            (void)d4;
            vcall(ctrl, 14, pi[i].id, 0, 0, 0);
            printf("vst: param %d: id=%u title=\"%s\" steps=%d default=%.6f flags=%#x value=%.6f\n", i, pi[i].id, pi[i].title, pi[i].steps, pi[i].def, (unsigned)pi[i].flags, gret_d());
            if (pfirst_idx < 0 && !(pi[i].flags & ((1 << 1) | (1 << 4) | (1 << 16)))) pfirst_idx = i;
        }
    }
    /* ---- a saved state (S-vst3a-state): the processor state to IComponent::setState (slot 12) and IEditController::setComponentState (slot 5), the controller state to
     * IEditController::setState (slot 6), each in an IBStream over the bytes of a file. FL Studio's own sequence is not known; this is the order of the SDK's hosting documentation. ---- */
    if (state_idx >= 0) {
        char fn[512];
        uint8_t *bufs[2] = {0, 0};
        long ns[2] = {0, 0};
        uint64_t r, so;
        for (int k = 0; k < 2; k++) {
            FILE *f;
            snprintf(fn, sizeof fn, "%s/s%d_c%d.bin", state_dir ? state_dir : "state", state_idx, k ? 2 : 3);
            f = fopen(fn, "rb");
            if (!f) { fprintf(stderr, "vst: cannot open %s\n", fn); return 2; }
            fseek(f, 0, SEEK_END); ns[k] = ftell(f); fseek(f, 0, SEEK_SET);
            bufs[k] = malloc((size_t)ns[k]);
            if (fread(bufs[k], 1, (size_t)ns[k], f) != (size_t)ns[k]) return 2;
            fclose(f);
        }
        {
            double c0 = cpu_now(); uint64_t i0 = icount();
            so = vst3a_stream_new(bufs[0], (size_t)ns[0]);
            r = vcall(comp, 12, so, 0, 0, 0);
            printf("vst: IComponent::setState(%ld bytes, state %d) -> %#llx  cpu %.3f ms icount %llu\n", ns[0], state_idx, ULL(r & 0xffffffffu), 1e3 * (cpu_now() - c0), ULL(icount() - i0));
            if (ctrl && !ctrl_same && !state_proc_only) {
                c0 = cpu_now(); i0 = icount();
                so = vst3a_stream_new(bufs[0], (size_t)ns[0]);
                r = vcall(ctrl, 5, so, 0, 0, 0);
                printf("vst: IEditController::setComponentState -> %#llx  cpu %.3f ms icount %llu\n", ULL(r & 0xffffffffu), 1e3 * (cpu_now() - c0), ULL(icount() - i0));
                c0 = cpu_now(); i0 = icount();
                so = vst3a_stream_new(bufs[1], (size_t)ns[1]);
                r = vcall(ctrl, 6, so, 0, 0, 0);
                printf("vst: IEditController::setState(%ld bytes) -> %#llx  cpu %.3f ms icount %llu\n", ns[1], ULL(r & 0xffffffffu), 1e3 * (cpu_now() - c0), ULL(icount() - i0));
            }
        }
        vst3a_stream_report();
        if (yield_ms > 0) {   /* S-vst3a-cv: the plugin's worker thread (multisample loader) runs while the host thread sleeps in the virtual clock */
            void fle_imp_Sleep(void);
            uint64_t sv4 = R[4];
            for (int y = 0; y < yield_ms; y += 10) { R[1] = 10; fle_imp_Sleep(); }
            R[4] = sv4;
            printf("vst: yielded %d ms of virtual time to the plugin's threads\n", yield_ms);
        }
        if (ctrl && !ctrl_same && !state_proc_only && pi) {
            int nd = 0;
            for (int i = 0; i < nparam; i++) {
                double v;
                vcall(ctrl, 14, pi[i].id, 0, 0, 0);
                v = gret_d();
                if (fabs(v - pi[i].def) > 1e-9) {
                    nd++;
                    if (nd <= (getenv("VST3A_PARAM_LIST") ? atoi(getenv("VST3A_PARAM_LIST")) : 0)) printf("vst: param-after-state id=%u \"%s\" default %.6f value %.6f\n", pi[i].id, pi[i].title, pi[i].def, v);
                }
            }
            printf("RESULT params_changed_by_state=%d of %d\n", nd, nparam);
        }
    }
    /* ---- buses and arrangements ---- */
    int nb_in = 0, nb_out = 0, ev_in = 0, ev_out = 0, ch_in[8] = {0}, ch_out[8] = {0};
    {
        int nin = (int)vcall(comp, 7, 0, 0, 0, 0), nout = (int)vcall(comp, 7, 0, 1, 0, 0);
        uint64_t bi = (uint64_t)(uintptr_t)calloc(1, 300);
        uint64_t arr_in = (uint64_t)(uintptr_t)calloc(8, 8), arr_out = (uint64_t)(uintptr_t)calloc(8, 8), r;
        ev_in = (int)vcall(comp, 7, 1, 0, 0, 0); ev_out = (int)vcall(comp, 7, 1, 1, 0, 0);
        printf("vst: getBusCount(audio,in)=%d (audio,out)=%d  events in=%d out=%d\n", nin, nout, ev_in, ev_out);
        if (nin > 8 || nout > 8) vst3a_stop("buses", "more than 8 audio buses");
        nb_in = nin; nb_out = nout;
        for (int d = 0; d < 2; d++) {
            int n = d ? nout : nin;
            for (int i = 0; i < n; i++) {
                char nm[64];
                memset((void *)(uintptr_t)bi, 0, 300);
                r = vcall(comp, 8, 0, (uint64_t)d, (uint64_t)i, bi);
                u16_to_ascii(bi + 12, nm, sizeof nm);
                (d ? ch_out : ch_in)[i] = (int)fle_ld(bi + 8, 4);
                printf("vst: getBusInfo(audio,%s,%d) -> %#llx channels=%d name=\"%s\" busType=%d flags=%#x\n", d ? "out" : "in", i, ULL(r & 0xffffffffu), (int)fle_ld(bi + 8, 4), nm, (int)fle_ld(bi + 268, 4), (unsigned)fle_ld(bi + 272, 4));
                r = vcall(comp, 10, 0, (uint64_t)d, (uint64_t)i, 1);
                printf("vst: activateBus(audio,%s,%d,true) -> %#llx\n", d ? "out" : "in", i, ULL(r & 0xffffffffu));
            }
        }
        for (int d = 0; d < 2; d++) {
            int n = d ? ev_out : ev_in;
            for (int i = 0; i < n; i++) {
                char nm[64];
                memset((void *)(uintptr_t)bi, 0, 300);
                r = vcall(comp, 8, 1, (uint64_t)d, (uint64_t)i, bi);
                u16_to_ascii(bi + 12, nm, sizeof nm);
                printf("vst: getBusInfo(event,%s,%d) -> %#llx channels=%d name=\"%s\" busType=%d flags=%#x\n", d ? "out" : "in", i, ULL(r & 0xffffffffu), (int)fle_ld(bi + 8, 4), nm, (int)fle_ld(bi + 268, 4), (unsigned)fle_ld(bi + 272, 4));
                r = vcall(comp, 10, 1, (uint64_t)d, (uint64_t)i, 1);
                printf("vst: activateBus(event,%s,%d,true) -> %#llx\n", d ? "out" : "in", i, ULL(r & 0xffffffffu));
            }
        }
        for (int i = 0; i < nin; i++) fle_st(arr_in + 8u * i, 8, ch_in[i] == 1 ? 1 : 3);
        for (int i = 0; i < nout; i++) fle_st(arr_out + 8u * i, 8, ch_out[i] == 1 ? 1 : 3);
        r = vcall(proc, 3, arr_in, (uint64_t)nin, arr_out, (uint64_t)nout);
        printf("vst: setBusArrangements(%d in, %d out; mono if the bus has 1 channel, else stereo) -> %#llx\n", nin, nout, ULL(r & 0xffffffffu));
        r = vcall(proc, 5, 0, 0, 0, 0);
        printf("vst: canProcessSampleSize(kSample32) -> %#llx\n", ULL(r & 0xffffffffu));
        printf("vst: getLatencySamples() = %llu  getTailSamples() = %llu\n", ULL(vcall(proc, 6, 0, 0, 0, 0) & 0xffffffffu), ULL(vcall(proc, 10, 0, 0, 0, 0) & 0xffffffffu));
    }
    /* ---- setupProcessing, setActive, setProcessing ---- */
    proc_setup *ps = calloc(1, sizeof *ps);
    ps->mode = 0; ps->ssize = 0; ps->maxs = frames; ps->rate = rate;
    {
        uint64_t r = vcall(proc, 7, (uint64_t)(uintptr_t)ps, 0, 0, 0);
        printf("vst: setupProcessing(realtime, float32, max %d, %.0f Hz) -> %#llx\n", frames, rate, ULL(r & 0xffffffffu));
        r = vcall(comp, 11, 1, 0, 0, 0);
        t_active = cpu_now(); ic_active = icount();
        printf("vst: IComponent::setActive(true) -> %#llx\n", ULL(r & 0xffffffffu));
        r = vcall(proc, 8, 1, 0, 0, 0);
        printf("vst: setProcessing(true) -> %#llx   guest MXCSR now %#x\n", ULL(r & 0xffffffffu), (unsigned)fle_mxcsr);
        printf("vst: DllMain..setActive cpu %.3f ms  icount %llu\n", 1e3 * (t_active - t_start), ULL(ic_active - ic_start));
        printf("RESULT init_cpu_ms=%.3f init_icount=%llu dllmain_cpu_ms=%.3f dllmain_icount=%llu init_icount_scalar_sse=%llu init_icount_other_sse=%llu\n", 1e3 * (t_active - t_start), ULL(ic_active - ic_start), 1e3 * (t_dll - t_start), ULL(ic_dll - ic_start), ULL(fle_icount_s), ULL(fle_icount_p));
    }
    /* ---- the parameter, if asked ---- */
    if (pfirst >= 0 && ctrl && pfirst_idx >= 0) {
        double d[4] = {0, 0, pfirst, 0};
        uint64_t r = vcall_d(ctrl, 15, pi[pfirst_idx].id, 0, 0, 0, d);
        vcall(ctrl, 14, pi[pfirst_idx].id, 0, 0, 0);
        printf("vst: setParamNormalized(id %u \"%s\", %.4f) -> %#llx, getParamNormalized = %.6f\n", pi[pfirst_idx].id, pi[pfirst_idx].title, pfirst, ULL(r & 0xffffffffu), gret_d());
        vst_pc_add(pi[pfirst_idx].id, 0, pfirst);
    }
    /* ---- process ---- */
    if (do_process && getenv("VST3A_DIAG_GUEST_FTZ")) {
        /* DIAGNOSTIC ONLY (decision 0165): the GUEST's MXCSR gets FTZ (bit 15) and DAZ (bit 6) before the first process call, as if the host had set them on the audio thread. The output is NOT the
         * plugin's with the real MXCSR (denormals are flushed); it tells how much of the time is spent in the exact handling of denormal operands. */
        fle_mxcsr |= 0x8040u;
        printf("vst: DIAGNOSTIC guest MXCSR now %#x (FTZ|DAZ set by the harness); the output below is not the plugin's with its own MXCSR\n", (unsigned)fle_mxcsr);
    }
    if (do_process && getenv("VST_DIAG_HOST_FTZ")) {
#if defined(__x86_64__)
        /* DIAGNOSTIC ONLY (decision 0156, worst-block analysis): FTZ|DAZ on the HOST's MXCSR. The guest's MXCSR (fle_mxcsr) is not touched; the output is NOT the plugin's
         * (denormal results differ), so the hash must not be compared. It tells whether the time of a denormal-heavy block is spent in the host's denormal handling. */
        _mm_setcsr(_mm_getcsr() | 0x8040u);
        printf("vst: DIAGNOSTIC host MXCSR FTZ|DAZ set; the output below is not the plugin's\n");
#endif
    }
    if (do_process) {
        static float *ibuf[8][2], *obuf[8][2];
        static uint64_t ip[8][2], op[8][2]; /* float** as the guest sees it, one pair per bus */
        static abb ibs[8], obs[8];
        proc_data *pd = calloc(1, sizeof *pd);
        proc_ctx *pc = calloc(1, sizeof *pc);
        uint32_t rng = 0x12345678u;
        uint64_t n0 = 0, h = UINT64_C(0xcbf29ce484222325), ic_blocks = 0, ics_blocks = 0, icp_blocks = 0;
        double cpu_sum = 0, wall_sum = 0, cpu_min = 1e9, cpu_max = 0, maxabs = 0, sumsq = 0;
        long nonfinite = 0, nsamp = 0, nonzero = 0;
        double bus_maxabs[8] = {0};
        FILE *fo = outfile ? fopen(outfile, "wb") : NULL, *fi = infile ? fopen(infile, "wb") : NULL;
        double *cpu_blk = calloc((size_t)blocks, sizeof(double));
        uint64_t *ic_blk = calloc((size_t)blocks, sizeof(uint64_t));
        const char *ev_prof = getenv("VST3A_PROFILE_BLOCK");
        int prof_block = ev_prof ? atoi(ev_prof) : 1;
        for (int i = 0; i < nb_in; i++) {
            for (int c = 0; c < 2; c++) { ibuf[i][c] = calloc((size_t)frames, 4); ip[i][c] = (uint64_t)(uintptr_t)ibuf[i][c]; }
            ibs[i].nch = ch_in[i] > 2 ? 2 : ch_in[i]; ibs[i].silence = 0; ibs[i].buf = (uint64_t)(uintptr_t)ip[i];
        }
        for (int i = 0; i < nb_out; i++) {
            for (int c = 0; c < 2; c++) { obuf[i][c] = calloc((size_t)frames, 4); op[i][c] = (uint64_t)(uintptr_t)obuf[i][c]; }
            obs[i].nch = ch_out[i] > 2 ? 2 : ch_out[i]; obs[i].silence = 0; obs[i].buf = (uint64_t)(uintptr_t)op[i];
        }
        pd->mode = 0; pd->ssize = 0; pd->nin = nb_in; pd->nout = nb_out; pd->in = nb_in ? (uint64_t)(uintptr_t)ibs : 0; pd->out = nb_out ? (uint64_t)(uintptr_t)obs : 0;
        pd->inpc = host.param_changes; pd->outpc = host.out_changes; pd->inev = host.event_list; pd->outev = host.event_list; pd->ctx = (uint64_t)(uintptr_t)pc;
        pc->rate = rate; pc->tempo = 200.0; pc->ts_num = 4; pc->ts_den = 4;
        pc->state = (1u << 1) | (1u << 10) | (1u << 13) | (1u << 9);
        fprof_dump("init", getenv("VST3A_PROFILE_INIT"));
#ifndef __EMSCRIPTEN__
        if (getenv("VST3A_SAMPLE")) samp_start();
#endif
        for (int b = 0; b < blocks; b++) {
            double t0, t1, w0, w1;
            uint64_t i0, i1, r, s0, s1, q0, q1;
            for (int i = 0; i < frames; i++) {
                double t = (double)(n0 + (uint64_t)i) / rate;
                rng ^= rng << 13; rng ^= rng >> 17; rng ^= rng << 5;
                double nz = ((double)(int32_t)rng) / 2147483648.0;
                float l = do_silence ? 0.0f : (float)(scale * (0.5 * sin(6.283185307179586 * 440.0 * t) + 0.05 * nz));
                rng ^= rng << 13; rng ^= rng >> 17; rng ^= rng << 5;
                nz = ((double)(int32_t)rng) / 2147483648.0;
                float rr = do_silence ? 0.0f : (float)(scale * (0.4 * sin(6.283185307179586 * 660.0 * t) + 0.05 * nz));
                for (int k = 0; k < nb_in; k++) { ibuf[k][0][i] = l; ibuf[k][1][i] = rr; }
                for (int k = 0; k < nb_out; k++) obuf[k][0][i] = obuf[k][1][i] = 0.0f;
            }
            if (fi && nb_in) { fwrite(ibuf[0][0], 4, (size_t)frames, fi); fwrite(ibuf[0][1], 4, (size_t)frames, fi); }
            vst_ev_clear();
            if (note_on && b == 0) for (int q = 0; q < nnotes; q++) vst_ev_note(1, note_list[q], 100.0f / 127.0f, 0, q + 1);
            if (note_on && note_off_block >= 0 && b == note_off_block) for (int q = 0; q < nnotes; q++) vst_ev_note(0, note_list[q], 0.0f, 0, q + 1);
            pd->nsamples = frames;
            pc->proj_samples = (int64_t)n0;
            pc->proj_music = (double)n0 / rate * pc->tempo / 60.0;
            if (b == prof_block) fprof_snap();
            t0 = cpu_now(); w0 = wall_now(); i0 = icount(); s0 = fle_icount_s; q0 = fle_icount_p;
            r = vcall(proc, 9, (uint64_t)(uintptr_t)pd, 0, 0, 0);
            t1 = cpu_now(); w1 = wall_now(); i1 = icount(); s1 = fle_icount_s; q1 = fle_icount_p;
            if (b == prof_block) fprof_dump("process", getenv("VST3A_PROFILE_PROCESS"));
            ics_blocks += s1 - s0; icp_blocks += q1 - q0;
            if ((r & 0xffffffffu) != 0) { printf("vst: process returned %#llx at block %d\n", ULL(r & 0xffffffffu), b); return 3; }
            if (b == 0) vst_pc_clear();
            cpu_blk[b] = t1 - t0; ic_blk[b] = i1 - i0;
            cpu_sum += t1 - t0; wall_sum += w1 - w0; ic_blocks += i1 - i0;
            if (t1 - t0 < cpu_min) cpu_min = t1 - t0;
            if (t1 - t0 > cpu_max) cpu_max = t1 - t0;
            for (int k = 0; k < nb_out; k++) {
                for (int c = 0; c < (int)obs[k].nch; c++) {
                    for (int i = 0; i < frames; i++) {
                        float v = obuf[k][c][i];
                        if (!isfinite(v)) nonfinite++;
                        else { double av = fabs((double)v); if (av > maxabs) maxabs = av; if (av > bus_maxabs[k]) bus_maxabs[k] = av; if (v != 0.0f) nonzero++; sumsq += (double)v * (double)v; }
                        nsamp++;
                    }
                    h = fnv(h, obuf[k][c], (size_t)frames * 4);
                    if (fo && k == 0) fwrite(obuf[k][c], 4, (size_t)frames, fo);
                }
            }
            n0 += (uint64_t)frames;
            if (b < 3 || b == blocks - 1 || getenv("VST_BLOCKS")) printf("vst: block %d  cpu %.3f ms  wall %.3f ms  icount %llu  events_out_added %llu  guest MXCSR %#x\n", b, 1e3 * (t1 - t0), 1e3 * (w1 - w0), ULL(i1 - i0), vst_ev_added(), (unsigned)fle_mxcsr);
        }
#ifndef __EMSCRIPTEN__
        if (getenv("VST3A_SAMPLE")) samp_stop(getenv("VST3A_SAMPLE"));
#endif
        if (fo) fclose(fo);
        if (fi) fclose(fi);
        /* median over blocks 1.. (block 0 pays first-touch costs; reported separately) */
        {
            double *sv = malloc((size_t)blocks * sizeof(double));
            int m = 0;
            for (int b = 1; b < blocks; b++) sv[m++] = cpu_blk[b];
            for (int i = 1; i < m; i++) { double v = sv[i]; int j = i - 1; while (j >= 0 && sv[j] > v) { sv[j + 1] = sv[j]; j--; } sv[j + 1] = v; }
            printf("RESULT blocks=%d frames=%d rate=%.0f cpu_ms_block0=%.3f cpu_ms_median=%.3f cpu_ms_min=%.3f cpu_ms_max=%.3f cpu_ms_mean=%.3f wall_ms_mean=%.3f icount_per_block_mean=%.0f icount_scalar_sse_per_block=%.0f icount_other_sse_per_block=%.0f block_ms_real=%.4f\n",
                   blocks, frames, rate, 1e3 * cpu_blk[0], m ? 1e3 * sv[m / 2] : 0.0, 1e3 * cpu_min, 1e3 * cpu_max, 1e3 * cpu_sum / blocks, 1e3 * wall_sum / blocks, (double)ic_blocks / blocks, (double)ics_blocks / blocks, (double)icp_blocks / blocks, 1e3 * frames / rate);
            if (blocks >= 4) printf("RESULT icount_block1=%llu icount_block2=%llu icount_block3=%llu icount_last=%llu\n", ULL(ic_blk[1]), ULL(ic_blk[2]), ULL(ic_blk[3]), ULL(ic_blk[blocks - 1]));
            free(sv);
        }
        printf("RESULT out_hash=%016llx nonfinite=%ld samples=%ld nonzero=%ld maxabs=%.6f rms=%.6f scale=%g silence=%d param_first=%g note=%d note_off_block=%d\n", ULL(h), nonfinite, nsamp, nonzero, maxabs, sqrt(sumsq / (double)(nsamp ? nsamp : 1)), scale, do_silence, pfirst, note_on, note_off_block);
        for (int k = 0; k < nb_out; k++) printf("RESULT out_bus%d maxabs=%.6f\n", k, bus_maxabs[k]);
    }
    vst_host_report();
    vst3a_msg_report();
    vst3a_cv_report();
    if (multi_list) { int rc = mi_run(fac, cid, &host, ps, multi_list, state_dir, frames, rate, blocks, note_on, note_list, nnotes, yield_ms); if (rc) return rc; }
#ifdef FLE_MX_STATS
    fle_mx_stats_dump();
#endif
    /* ---- teardown ---- */
    {
        uint64_t r;
        vcall(proc, 8, 0, 0, 0, 0);
        vcall(comp, 11, 0, 0, 0, 0);
        r = vcall(comp, 4, 0, 0, 0, 0);
        printf("vst: setProcessing(false), setActive(false), IComponent::terminate -> %#llx\n", ULL(r & 0xffffffffu));
        if (ctrl && !ctrl_same) { vcall(ctrl, 4, 0, 0, 0, 0); vcall(ctrl, 2, 0, 0, 0, 0); }
        vcall(proc, 2, 0, 0, 0, 0);
        r = vcall(comp, 2, 0, 0, 0, 0);
        printf("vst: release(component) -> %llu\n", ULL(r & 0xffffffffu));
        /* a second instance of the same class (the reference project has two OTT): createInstance + initialize + setupProcessing + setActive(true), then its teardown */
        if (do_process) {
            uint64_t pobj = (uint64_t)(uintptr_t)calloc(1, 16), c2, p2;
            double t2a = cpu_now(), t2b;
            uint64_t i2a = icount();
            fle_st(pobj, 8, 0);
            r = vcall(fac, 6, (uint64_t)(uintptr_t)cid, (uint64_t)(uintptr_t)IID_IComponent, pobj, 0);
            c2 = fle_ld(pobj, 8);
            if ((r & 0xffffffffu) == 0 && c2) {
                vcall(c2, 3, host.host_app, 0, 0, 0);
                fle_st(pobj, 8, 0);
                vcall(c2, 0, (uint64_t)(uintptr_t)IID_IAudioProcessor, pobj, 0, 0);
                p2 = fle_ld(pobj, 8);
                vcall(p2, 7, (uint64_t)(uintptr_t)ps, 0, 0, 0);
                vcall(c2, 11, 1, 0, 0, 0);
                t2b = cpu_now();
                printf("RESULT second_instance_init_cpu_ms=%.3f second_instance_init_icount=%llu\n", 1e3 * (t2b - t2a), ULL(icount() - i2a));
                vcall(c2, 11, 0, 0, 0, 0);
                vcall(c2, 4, 0, 0, 0, 0);
                vcall(p2, 2, 0, 0, 0, 0);
                vcall(c2, 2, 0, 0, 0, 0);
            }
        }
        vcall(fac, 2, 0, 0, 0, 0);
        {
            uint64_t va = export_va(img.base, "ExitDll");
            const uint64_t z[6] = {0};
            if (va) { gcall(va, z, NULL); printf("vst: ExitDll() returned\n"); }
        }
    }
    printf("vst: done\n");
    return 0;
}
