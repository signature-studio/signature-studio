/* mt1 (decision 0173), S1: main_vst.c with an EXTERNAL INPUT mode for a plugin Worker. Derived from lift/harness/vst/main_vst.c (which is NOT changed); the init, the host objects and the process call are the
 * same code. Differences (search for `ext`):
 *   --ext             the process loop takes each input block from the embedder (import env.mt1_ext_wait, which blocks until a block arrives and copies it into the two input buffers; it returns 0 to quit)
 *                     and hands each output block back (import env.mt1_ext_done). --blocks N is then only the capacity of the statistics arrays. out_hash is computed over the outputs exactly as before, so a
 *                     producer that sends the input stream the original harness synthesises must get the original out_hash (8104f50e938ccf36 for the default 40 blocks, decision 0156/0164).
 *   --preroll N       before the first real block, N synthetic blocks go through process() (warm-up of lazily compiled code and of first-touched memory); --preroll-reset 1|2 then cycles
 *                     setProcessing(false/true) (1) or setActive(false/true) + setProcessing (2) so that the plugin's state is (ideally) the one of a fresh instance; whether the first real block is bit-identical
 *                     to a run without pre-roll is a MEASUREMENT (RESULT out_hash), not an assumption.
 * Only built for emscripten (the imports are wasm imports of module "env"); the browser host that implements them is lift/harness/web/mt1/vst-host.js. */
/* Minimal VST3 host for a lifted plugin (decision 0156, PROVISIONAL). Maps the plugin's PE at its ImageBase, runs the PE entry (DllMain) and then the VST3
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
#include "fle_imports.h"
#include "vst_host.h"
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
uint64_t fle_icount, fle_icount_s, fle_icount_p;
#ifdef FLE_MX_STATS
void fle_mx_stats_dump(void);
#endif

#define ULL(x) ((unsigned long long)(x))

#ifdef __EMSCRIPTEN__
__attribute__((import_module("env"), import_name("mt1_ext_wait"))) extern int mt1_ext_wait(float *in_l, float *in_r, int frames, int block);
__attribute__((import_module("env"), import_name("mt1_ext_done"))) extern void mt1_ext_done(const float *out_l, const float *out_r, int frames, int block);
#else
static int mt1_ext_wait(float *in_l, float *in_r, int frames, int block) { (void)in_l; (void)in_r; (void)frames; (void)block; return 0; }
static void mt1_ext_done(const float *out_l, const float *out_r, int frames, int block) { (void)out_l; (void)out_r; (void)frames; (void)block; }
#endif

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
    if (cbva && fle_ld(cbva, 8)) { fprintf(stderr, "vst: the module has TLS callbacks: not called by this harness\n"); exit(2); }
    printf("vst: tls raw %llu B + zero %u -> index %llu\n", ULL(end - start), zero, ULL(idx));
    return 1;
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

int main(int argc, char **argv)
{
    const char *path = NULL, *outfile = NULL, *infile = NULL;
    int blocks = 40, frames = 4096, do_silence = 0, do_process = 1, ext = 0, preroll = 0, preroll_reset = 0, nb_done;
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
        else if (!strcmp(argv[i], "--ext")) ext = 1;
        else if (!strcmp(argv[i], "--preroll") && i + 1 < argc) preroll = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--preroll-reset") && i + 1 < argc) preroll_reset = atoi(argv[++i]);
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

    t_start = cpu_now(); ic_start = icount();
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
            if (!strcmp(cat, "Audio Module Class") && audio_cls < 0) { audio_cls = i; for (int k = 0; k < 16; k++) cid[k] = (char)fle_ld(info + k, 1); }
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
    /* ---- buses and arrangements ---- */
    {
        int nin = (int)vcall(comp, 7, 0, 0, 0, 0), nout = (int)vcall(comp, 7, 0, 1, 0, 0);
        uint64_t bi = (uint64_t)(uintptr_t)calloc(1, 300);
        uint64_t arr_in = (uint64_t)(uintptr_t)calloc(8, 8), arr_out = (uint64_t)(uintptr_t)calloc(8, 8), r;
        printf("vst: getBusCount(audio,in)=%d (audio,out)=%d  events in=%d out=%d\n", nin, nout, (int)vcall(comp, 7, 1, 0, 0, 0), (int)vcall(comp, 7, 1, 1, 0, 0));
        for (int d = 0; d < 2; d++) {
            int n = d ? nout : nin;
            for (int i = 0; i < n; i++) {
                char nm[64];
                memset((void *)(uintptr_t)bi, 0, 300);
                r = vcall(comp, 8, 0, (uint64_t)d, (uint64_t)i, bi);
                u16_to_ascii(bi + 12, nm, sizeof nm);
                printf("vst: getBusInfo(audio,%s,%d) -> %#llx channels=%d name=\"%s\" busType=%d flags=%#x\n", d ? "out" : "in", i, ULL(r & 0xffffffffu), (int)fle_ld(bi + 8, 4), nm, (int)fle_ld(bi + 268, 4), (unsigned)fle_ld(bi + 272, 4));
                r = vcall(comp, 10, 0, (uint64_t)d, (uint64_t)i, 1);
                printf("vst: activateBus(audio,%s,%d,true) -> %#llx\n", d ? "out" : "in", i, ULL(r & 0xffffffffu));
            }
        }
        for (int i = 0; i < nin; i++) fle_st(arr_in + 8u * i, 8, 3);
        for (int i = 0; i < nout; i++) fle_st(arr_out + 8u * i, 8, 3);
        r = vcall(proc, 3, arr_in, (uint64_t)nin, arr_out, (uint64_t)nout);
        printf("vst: setBusArrangements(stereo x %d in, stereo x %d out) -> %#llx\n", nin, nout, ULL(r & 0xffffffffu));
        r = vcall(proc, 5, 0, 0, 0, 0);
        printf("vst: canProcessSampleSize(kSample32) -> %#llx\n", ULL(r & 0xffffffffu));
        printf("vst: getLatencySamples() = %llu  getTailSamples() = %llu\n", ULL(vcall(proc, 6, 0, 0, 0, 0) & 0xffffffffu), ULL(vcall(proc, 10, 0, 0, 0, 0) & 0xffffffffu));
        if (nin != 1 || nout != 1) printf("vst: note: this harness feeds bus 0 only; other buses get silence\n");
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
    if (do_process && getenv("VST_DIAG_HOST_FTZ")) {
#if defined(__x86_64__)
        /* DIAGNOSTIC ONLY (decision 0156, worst-block analysis): FTZ|DAZ on the HOST's MXCSR. The guest's MXCSR (fle_mxcsr) is not touched; the output is NOT the plugin's
         * (denormal results differ), so the hash must not be compared. It tells whether the time of a denormal-heavy block is spent in the host's denormal handling. */
        _mm_setcsr(_mm_getcsr() | 0x8040u);
        printf("vst: DIAGNOSTIC host MXCSR FTZ|DAZ set; the output below is not the plugin's\n");
#endif
    }
    if (do_process) {
        float *inb[2], *outb[2];
        uint64_t inp[2], outp[2]; /* float** as the guest sees it */
        abb ib, ob;
        proc_data *pd = calloc(1, sizeof *pd);
        proc_ctx *pc = calloc(1, sizeof *pc);
        uint32_t rng = 0x12345678u;
        uint64_t n0 = 0, h = UINT64_C(0xcbf29ce484222325), ic_blocks = 0, ics_blocks = 0, icp_blocks = 0;
        double cpu_sum = 0, wall_sum = 0, cpu_min = 1e9, cpu_max = 0, maxabs = 0, sumsq = 0;
        long nonfinite = 0, nsamp = 0;
        FILE *fo = outfile ? fopen(outfile, "wb") : NULL, *fi = infile ? fopen(infile, "wb") : NULL;
        double *cpu_blk = calloc((size_t)blocks, sizeof(double));
        uint64_t *ic_blk = calloc((size_t)blocks, sizeof(uint64_t));
        for (int c = 0; c < 2; c++) { inb[c] = calloc((size_t)frames, 4); outb[c] = calloc((size_t)frames, 4); }
        inp[0] = (uint64_t)(uintptr_t)inb[0]; inp[1] = (uint64_t)(uintptr_t)inb[1]; outp[0] = (uint64_t)(uintptr_t)outb[0]; outp[1] = (uint64_t)(uintptr_t)outb[1];
        ib.nch = 2; ib.silence = 0; ib.buf = (uint64_t)(uintptr_t)inp; ob.nch = 2; ob.silence = 0; ob.buf = (uint64_t)(uintptr_t)outp;
        pd->mode = 0; pd->ssize = 0; pd->nin = 1; pd->nout = 1; pd->in = (uint64_t)(uintptr_t)&ib; pd->out = (uint64_t)(uintptr_t)&ob;
        pd->inpc = host.param_changes; pd->outpc = host.out_changes; pd->inev = host.event_list; pd->outev = host.event_list; pd->ctx = (uint64_t)(uintptr_t)pc;
        pc->rate = rate; pc->tempo = 200.0; pc->ts_num = 4; pc->ts_den = 4;
        pc->state = (1u << 1) | (1u << 10) | (1u << 13) | (1u << 9);
        nb_done = blocks;
        if (ext && preroll > 0) {
            uint32_t rngp = 0x9e3779b9u;
            double first_ms = 0, last_ms = 0, nzp;
            for (int pb = 0; pb < preroll; pb++) {
                double a, d;
                uint64_t rr;
                for (int i = 0; i < frames; i++) {
                    double t = (double)((uint64_t)pb * (uint64_t)frames + (uint64_t)i) / rate;
                    rngp ^= rngp << 13; rngp ^= rngp >> 17; rngp ^= rngp << 5;
                    nzp = ((double)(int32_t)rngp) / 2147483648.0;
                    inb[0][i] = (float)(0.5 * sin(6.283185307179586 * 440.0 * t) + 0.05 * nzp);
                    rngp ^= rngp << 13; rngp ^= rngp >> 17; rngp ^= rngp << 5;
                    nzp = ((double)(int32_t)rngp) / 2147483648.0;
                    inb[1][i] = (float)(0.4 * sin(6.283185307179586 * 660.0 * t) + 0.05 * nzp);
                    outb[0][i] = outb[1][i] = 0.0f;
                }
                pd->nsamples = frames;
                pc->proj_samples = (int64_t)((uint64_t)pb * (uint64_t)frames);
                pc->proj_music = (double)pc->proj_samples / rate * pc->tempo / 60.0;
                a = wall_now();
                rr = vcall(proc, 9, (uint64_t)(uintptr_t)pd, 0, 0, 0);
                d = 1e3 * (wall_now() - a);
                if (pb == 0) first_ms = d;
                last_ms = d;
                if ((rr & 0xffffffffu) != 0) { printf("vst: preroll process returned %#llx at block %d\n", ULL(rr & 0xffffffffu), pb); return 3; }
            }
            printf("RESULT preroll blocks=%d first_ms=%.3f last_ms=%.3f reset=%d\n", preroll, first_ms, last_ms, preroll_reset);
            if (preroll_reset >= 1) {
                vcall(proc, 8, 0, 0, 0, 0);
                if (preroll_reset >= 2) { vcall(comp, 11, 0, 0, 0, 0); vcall(comp, 11, 1, 0, 0, 0); }
                vcall(proc, 8, 1, 0, 0, 0);
            }
            vst_pc_clear();
            if (pfirst >= 0 && ctrl && pfirst_idx >= 0) vst_pc_add(pi[pfirst_idx].id, 0, pfirst);
            pc->proj_samples = 0; pc->proj_music = 0;
        }
        for (int b = 0; b < blocks; b++) {
            double t0, t1, w0, w1;
            uint64_t i0, i1, r, s0, s1, q0, q1;
            if (ext && !mt1_ext_wait(inb[0], inb[1], frames, b)) { nb_done = b; break; }
            for (int i = 0; !ext && i < frames; i++) {
                double t = (double)(n0 + (uint64_t)i) / rate;
                rng ^= rng << 13; rng ^= rng >> 17; rng ^= rng << 5;
                double nz = ((double)(int32_t)rng) / 2147483648.0;
                inb[0][i] = do_silence ? 0.0f : (float)(scale * (0.5 * sin(6.283185307179586 * 440.0 * t) + 0.05 * nz));
                rng ^= rng << 13; rng ^= rng >> 17; rng ^= rng << 5;
                nz = ((double)(int32_t)rng) / 2147483648.0;
                inb[1][i] = do_silence ? 0.0f : (float)(scale * (0.4 * sin(6.283185307179586 * 660.0 * t) + 0.05 * nz));
                outb[0][i] = outb[1][i] = 0.0f;
            }
            if (ext) { memset(outb[0], 0, (size_t)frames * 4); memset(outb[1], 0, (size_t)frames * 4); }
            if (fi) { fwrite(inb[0], 4, (size_t)frames, fi); fwrite(inb[1], 4, (size_t)frames, fi); }
            pd->nsamples = frames;
            pc->proj_samples = (int64_t)n0;
            pc->proj_music = (double)n0 / rate * pc->tempo / 60.0;
            t0 = cpu_now(); w0 = wall_now(); i0 = icount(); s0 = fle_icount_s; q0 = fle_icount_p;
            r = vcall(proc, 9, (uint64_t)(uintptr_t)pd, 0, 0, 0);
            t1 = cpu_now(); w1 = wall_now(); i1 = icount(); s1 = fle_icount_s; q1 = fle_icount_p;
            ics_blocks += s1 - s0; icp_blocks += q1 - q0;
            if ((r & 0xffffffffu) != 0) { printf("vst: process returned %#llx at block %d\n", ULL(r & 0xffffffffu), b); return 3; }
            if (b == 0) vst_pc_clear();
            if (ext) mt1_ext_done(outb[0], outb[1], frames, b);
            cpu_blk[b] = t1 - t0; ic_blk[b] = i1 - i0;
            cpu_sum += t1 - t0; wall_sum += w1 - w0; ic_blocks += i1 - i0;
            if (t1 - t0 < cpu_min) cpu_min = t1 - t0;
            if (t1 - t0 > cpu_max) cpu_max = t1 - t0;
            for (int c = 0; c < 2; c++) {
                for (int i = 0; i < frames; i++) {
                    float v = outb[c][i];
                    if (!isfinite(v)) nonfinite++;
                    else { double av = fabs((double)v); if (av > maxabs) maxabs = av; sumsq += (double)v * (double)v; }
                    nsamp++;
                }
                h = fnv(h, outb[c], (size_t)frames * 4);
                if (fo) fwrite(outb[c], 4, (size_t)frames, fo);
            }
            n0 += (uint64_t)frames;
            if (b < 3 || b == blocks - 1 || getenv("VST_BLOCKS")) printf("vst: block %d  cpu %.3f ms  wall %.3f ms  icount %llu  guest MXCSR %#x\n", b, 1e3 * (t1 - t0), 1e3 * (w1 - w0), ULL(i1 - i0), (unsigned)fle_mxcsr);
        }
        blocks = nb_done;
        if (fo) fclose(fo);
        if (fi) fclose(fi);
        /* median over blocks 1.. (block 0 pays first-touch costs; reported separately) */
        {
            double *s = malloc((size_t)blocks * sizeof(double));
            int m = 0;
            for (int b = 1; b < blocks; b++) s[m++] = cpu_blk[b];
            for (int i = 1; i < m; i++) { double v = s[i]; int j = i - 1; while (j >= 0 && s[j] > v) { s[j + 1] = s[j]; j--; } s[j + 1] = v; }
            printf("RESULT blocks=%d frames=%d rate=%.0f cpu_ms_block0=%.3f cpu_ms_median=%.3f cpu_ms_min=%.3f cpu_ms_max=%.3f cpu_ms_mean=%.3f wall_ms_mean=%.3f icount_per_block_mean=%.0f icount_scalar_sse_per_block=%.0f icount_other_sse_per_block=%.0f block_ms_real=%.4f\n",
                   blocks, frames, rate, 1e3 * cpu_blk[0], m ? 1e3 * s[m / 2] : 0.0, 1e3 * cpu_min, 1e3 * cpu_max, 1e3 * cpu_sum / blocks, 1e3 * wall_sum / blocks, (double)ic_blocks / blocks, (double)ics_blocks / blocks, (double)icp_blocks / blocks, 1e3 * frames / rate);
            free(s);
        }
        printf("RESULT out_hash=%016llx nonfinite=%ld samples=%ld maxabs=%.6f rms=%.6f scale=%g silence=%d param_first=%g\n", ULL(h), nonfinite, nsamp, maxabs, sqrt(sumsq / (double)(nsamp ? nsamp : 1)), scale, do_silence, pfirst);
    }
    vst_host_report();
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
