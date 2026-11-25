/* The plugin-worker entry of one lifted VST3 module (lift/harness/web/mt1/plugin-worker/build.sh; app/src/plugins/plugin-worker.ts). The start-up is the VST3 host
 * sequence of main_vst3a.c, in the same order: LoadLibraryW of the module (DllMain runs inside), InitDll, GetPluginFactory, the classes, createInstance(IComponent),
 * IComponent::initialize, IAudioProcessor, the controller (its own class or the component itself), the connection, the buses and their arrangements,
 * setupProcessing, setActive(true), setProcessing(true). Then the block loop: mt1_ext_wait returns 1 with the input block in the two buffers, one process() call runs
 * on it (the ProcessData of main_vst3a.c: bus 0 is the two buffers, the parameter and event lists are the host's), and mt1_ext_done takes the output block; 0 ends
 * the loop (the Worker asked to quit). The first mt1_ext_wait is READY for the host. The teardown after the loop mirrors main_vst3a.c. */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "boot.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_runtime.h"
#include "fle_vmap.h"
#include "vfs_seed.h"
#include "vst3a.h"
#include "vst_host.h"

void fle_teb_set_stack(uint64_t lo, uint64_t hi);
/* ott-host.ts calls _emscripten_stack_alloc for the argument strings: the reference keeps the function in the link (and in the exports) */
void *emscripten_stack_alloc(unsigned long n);
void *(*volatile vst_keep_stack_alloc)(unsigned long) = emscripten_stack_alloc;
extern uint64_t vst_module_base, vst_module_size;
extern uint64_t fle_tls_values[];
void fle_imp_TlsAlloc(void);
void fle_imp_LoadLibraryW(void);

/* the host side of plugin-worker.ts: the block hooks of ott-host.ts (env.mt1_ext_wait, env.mt1_ext_done) */
int mt1_ext_wait(uint32_t inL, uint32_t inR, uint32_t frames, uint32_t block);
void mt1_ext_done(uint32_t outL, uint32_t outR, uint32_t frames, uint32_t block);

typedef struct {
    int32_t mode, ssize, maxs; int32_t pad; double rate;
} proc_setup;
_Static_assert(sizeof(proc_setup) == 24, "ProcessSetup");

/* guest structures (main_vst3a.c): every pointer is a 64-bit guest address (also in the wasm32 build) */
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

#define ULL(x) ((unsigned long long)(x))

static uint64_t fnv(uint64_t h, const void *p, size_t n)
{
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= UINT64_C(0x100000001b3); }
    return h;
}

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

/* implicit TLS of the module (the loader's work for a PE with a TLS directory; see main_vst3a.c). The TLS callbacks of the directory are kept; main calls them
 * with DLL_PROCESS_ATTACH before DllMain. */
static uint64_t tls_cb[8];
static int ntls_cb;
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
    if (cbva) for (int k = 0; k < 8 && fle_ld(cbva + 8u * k, 8); k++) tls_cb[ntls_cb++] = fle_ld(cbva + 8u * k, 8);
    return 1;
}

/* IAT slots of an import with a row get the pseudo-VA of the row (what GetProcAddress hands out), so that a `mov reg,[slot]; call reg` reaches the row's function. */
static int iat_fixup(uint64_t base)
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
                /* an ordinal import: the response row of "#<n>" when the lift has one (an answer of lift/harness/vst/ordinals-vst.txt), else the stop by number */
                char on[16];
                snprintf(on, sizeof on, "#%u", (unsigned)(th & 0xffff));
                for (j = 0; j < fle_resp_count; j++)
                    if (!strcmp(fle_resp_table[j].dll, dll) && !strcmp(fle_resp_table[j].name, on)) break;
                if (j < fle_resp_count) { fle_st(base + ft + 8u * k, 8, fle_resp_table[j].va); nfix++; continue; }
                for (unsigned o = 0; o < vst3a_ord_count; o++)
                    if (!strcmp(vst3a_ord_table[o].dll, dll) && vst3a_ord_table[o].ordinal == (int)(th & 0xffff)) { fle_st(base + ft + 8u * k, 8, vst3a_ord_table[o].va); nfix++; break; }
                continue;
            }
            nm = (const char *)m + (th & 0x7fffffffu) + 2;
            for (j = 0; j < fle_resp_count; j++)
                if (!strcmp(fle_resp_table[j].dll, dll) && !strcmp(fle_resp_table[j].name, nm)) break;
            if (j == fle_resp_count) { nmiss++; continue; }
            fle_st(base + ft + 8u * k, 8, fle_resp_table[j].va);
            nfix++;
        }
    }
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

static uint64_t ncall(uint64_t obj, unsigned slot, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5)
{
    return vcall(obj, slot, a2, a3, a4, a5) & 0xffffffffu;
}

int main(int argc, char **argv)
{
    const char *path = NULL;
    int frames = 4096;
    double rate = 44100.0;
    harness_image img;
    char err[256];
    vst_host host;
    uint8_t *stk;
    uint64_t fac, comp = 0, proc = 0, ctrl = 0;
    int ncls, audio_cls = -1;
    char cid[16];
    int ctrl_same = 0;
    int nin = 0, nout = 0, ch_in[8] = {0}, ch_out[8] = {0};
    int ev_in = 0;
    int failed = 0;

    setvbuf(stdout, NULL, _IOLBF, 0);
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--frames") && i + 1 < argc) frames = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--rate") && i + 1 < argc) rate = atof(argv[++i]);
        else if ((!strcmp(argv[i], "--blocks") || !strcmp(argv[i], "--preroll") || !strcmp(argv[i], "--preroll-reset")) && i + 1 < argc) i++;
        else if (!strcmp(argv[i], "--ext")) continue;
        else if (argv[i][0] != '-' && !path) path = argv[i];
        else { fprintf(stderr, "usage: %s FILE [--frames N] [--rate HZ]\n", argv[0]); return 64; }
    }
    if (!path) { fprintf(stderr, "usage: %s FILE\n", argv[0]); return 64; }

    if (image_load(path, &img, err, sizeof err) != 0) { fprintf(stderr, "image: %s\n", err); return 2; }
    fle_vmap_register_image(img.base);
    iat_fixup(img.base);
    vst_module_base = img.base; vst_module_size = img.size;
    printf("vst: image base=%#llx size=%#llx entry=%#llx imports=%zu exports=%zu\n", ULL(img.base), ULL(img.size), ULL(img.entry), img.nimports, img.nexports);
    stk = aligned_alloc(4096, HARNESS_STACK_BYTES);
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES);
    fle_vmap_register_range((uint64_t)(uintptr_t)stk, HARNESS_STACK_BYTES, 0x04u, 0x20000u);
    tls_setup(img.base, img.size);
    if (vfs_seed_from_env() != 0) return 2;
    vst_host_init(&host);

    /* ---- the module's TLS callbacks with DLL_PROCESS_ATTACH, before DllMain (main_vst3a.c) ---- */
    for (int k = 0; k < ntls_cb; k++) {
        const uint64_t a[6] = { img.base, 1, 0, 0, 0, 0 };
        gcall(tls_cb[k], a, NULL);
        printf("vst: TLS callback %d at %#llx (DLL_PROCESS_ATTACH) returned\n", k, ULL(tls_cb[k]));
    }
    /* ---- LoadLibraryW of the module: DllMain runs inside (fle_realmod_load) ---- */
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
        printf("vst: LoadLibraryW(\"%s\") = %#llx (DllMain ran inside)\n", nm, ULL(r));
        if (r != img.base) return 3;
    }
    /* ---- InitDll, GetPluginFactory ---- */
    {
        uint64_t va = export_va(img.base, "InitDll");
        const uint64_t z[6] = {0};
        if (va) { uint64_t r = gcall(va, z, NULL); printf("vst: InitDll() returned %llu\n", ULL(r & 0xff)); if (!(r & 0xff)) return 3; }
        va = export_va(img.base, "GetPluginFactory");
        fac = gcall(va, z, NULL);
        printf("vst: GetPluginFactory() = %#llx\n", ULL(fac));
        if (!fac) return 3;
    }
    /* ---- the factory: the classes, and the Audio Module Class ---- */
    {
        uint64_t info = (uint64_t)(uintptr_t)calloc(1, 512), pobj = (uint64_t)(uintptr_t)calloc(1, 16), r;
        vcall(fac, 3, info, 0, 0, 0);
        ncls = (int)vcall(fac, 4, 0, 0, 0, 0);
        printf("vst: countClasses() = %d\n", ncls);
        for (int i = 0; i < ncls; i++) {
            char h[40], cat[33], nm[65];
            memset((void *)(uintptr_t)info, 0, 512);
            r = vcall(fac, 5, (uint64_t)i, info, 0, 0);
            hexid(info, h);
            for (int k = 0; k < 32; k++) cat[k] = (char)fle_ld(info + 20 + k, 1); cat[32] = 0;
            for (int k = 0; k < 64; k++) nm[k] = (char)fle_ld(info + 52 + k, 1); nm[64] = 0;
            printf("vst: class %d: getClassInfo -> %#llx cid=%s category=\"%s\" name=\"%s\"\n", i, ULL(r & 0xffffffffu), h, cat, nm);
            if (!strcmp(cat, "Audio Module Class") && audio_cls < 0) { audio_cls = i; for (int k = 0; k < 16; k++) cid[k] = (char)fle_ld(info + k, 1); }
        }
        if (audio_cls < 0) { printf("vst: no Audio Module Class\n"); return 3; }
        fle_st(pobj, 8, 0);
        r = vcall(fac, 6, (uint64_t)(uintptr_t)cid, (uint64_t)(uintptr_t)IID_IComponent, pobj, 0);
        comp = fle_ld(pobj, 8);
        printf("vst: createInstance(IComponent) -> %#llx component=%#llx\n", ULL(r & 0xffffffffu), ULL(comp));
        if ((r & 0xffffffffu) != 0 || !comp) return 3;
    }
    /* ---- IComponent::initialize(hostContext) ---- */
    {
        uint64_t r = ncall(comp, 3, host.host_app, 0, 0, 0);
        printf("vst: IComponent::initialize(IHostApplication) -> %#llx\n", ULL(r));
        if (r != 0) return 3;
    }
    {
        uint64_t pobj = (uint64_t)(uintptr_t)calloc(1, 16), r;
        r = ncall(comp, 0, (uint64_t)(uintptr_t)IID_IAudioProcessor, pobj, 0, 0);
        proc = fle_ld(pobj, 8);
        printf("vst: queryInterface(IAudioProcessor) -> %#llx processor=%#llx\n", ULL(r), ULL(proc));
        if (r != 0 || !proc) return 3;
        fle_st(pobj, 8, 0);
        r = ncall(comp, 0, (uint64_t)(uintptr_t)IID_IEditController, pobj, 0, 0);
        ctrl = fle_ld(pobj, 8);
        printf("vst: queryInterface(IEditController) on the component -> %#llx controller=%#llx\n", ULL(r), ULL(ctrl));
        if (r == 0 && ctrl) ctrl_same = 1;
        else {
            uint64_t tuid = (uint64_t)(uintptr_t)calloc(1, 16);
            r = ncall(comp, 5, tuid, 0, 0, 0);
            printf("vst: getControllerClassId -> %#llx\n", ULL(r));
            if (r == 0) {
                fle_st(pobj, 8, 0);
                r = ncall(fac, 6, tuid, (uint64_t)(uintptr_t)IID_IEditController, pobj, 0);
                ctrl = fle_ld(pobj, 8);
                printf("vst: createInstance(IEditController) -> %#llx controller=%#llx\n", ULL(r), ULL(ctrl));
                if (r == 0 && ctrl) {
                    r = ncall(ctrl, 3, host.host_app, 0, 0, 0);
                    printf("vst: IEditController::initialize -> %#llx\n", ULL(r));
                } else ctrl = 0;
            }
        }
    }
    if (ctrl && !ctrl_same) {
        /* IConnectionPoint of the component and of the controller, connected to each other (IID 70A4156F-6E6E-4026-9891-48BFAA60D8D1) */
        vst_iid cpid; uint64_t po = (uint64_t)(uintptr_t)calloc(1, 16), cpc, cpe, r;
        vst_iid_make(cpid, 0x70A4156F, 0x6E6E4026, 0x989148BF, 0xAA60D8D1);
        r = ncall(comp, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpc = fle_ld(po, 8);
        fle_st(po, 8, 0);
        r = ncall(ctrl, 0, (uint64_t)(uintptr_t)cpid, po, 0, 0); cpe = fle_ld(po, 8);
        if (cpc && cpe) {
            r = ncall(cpc, 3, cpe, 0, 0, 0); printf("vst: component->connect(controller) -> %#llx\n", ULL(r));
            r = ncall(cpe, 3, cpc, 0, 0, 0); printf("vst: controller->connect(component) -> %#llx\n", ULL(r));
        }
    }
    if (ctrl) {
        uint64_t r = ncall(ctrl, 16, host.comp_handler, 0, 0, 0);
        printf("vst: IEditController::setComponentHandler -> %#llx\n", ULL(r));
    }
    /* ---- buses and arrangements ---- */
    {
        uint64_t bi = (uint64_t)(uintptr_t)calloc(1, 300);
        uint64_t arr_in = (uint64_t)(uintptr_t)calloc(8, 8), arr_out = (uint64_t)(uintptr_t)calloc(8, 8), r;
        int ev_out;
        nin = (int)vcall(comp, 7, 0, 0, 0, 0); nout = (int)vcall(comp, 7, 0, 1, 0, 0);
        ev_in = (int)vcall(comp, 7, 1, 0, 0, 0); ev_out = (int)vcall(comp, 7, 1, 1, 0, 0);
        printf("vst: getBusCount(audio,in)=%d (audio,out)=%d  events in=%d out=%d\n", nin, nout, ev_in, ev_out);
        if (nin > 8 || nout > 8) { fprintf(stderr, "vst: more than 8 audio buses\n"); return 3; }
        for (int d = 0; d < 2; d++) {
            int n = d ? nout : nin;
            for (int i = 0; i < n; i++) {
                memset((void *)(uintptr_t)bi, 0, 300);
                r = vcall(comp, 8, 0, (uint64_t)d, (uint64_t)i, bi);
                (d ? ch_out : ch_in)[i] = (int)fle_ld(bi + 8, 4);
                printf("vst: getBusInfo(audio,%s,%d) -> %#llx channels=%d\n", d ? "out" : "in", i, ULL(r & 0xffffffffu), (int)fle_ld(bi + 8, 4));
                r = vcall(comp, 10, 0, (uint64_t)d, (uint64_t)i, 1);
                printf("vst: activateBus(audio,%s,%d,true) -> %#llx\n", d ? "out" : "in", i, ULL(r & 0xffffffffu));
            }
        }
        for (int d = 0; d < 2; d++) {
            int n = d ? ev_out : ev_in;
            for (int i = 0; i < n; i++) {
                r = vcall(comp, 10, 1, (uint64_t)d, (uint64_t)i, 1);
                printf("vst: activateBus(event,%s,%d,true) -> %#llx\n", d ? "out" : "in", i, ULL(r & 0xffffffffu));
            }
        }
        for (int i = 0; i < nin; i++) fle_st(arr_in + 8u * i, 8, ch_in[i] == 1 ? 1 : 3);
        for (int i = 0; i < nout; i++) fle_st(arr_out + 8u * i, 8, ch_out[i] == 1 ? 1 : 3);
        r = vcall(proc, 3, arr_in, (uint64_t)nin, arr_out, (uint64_t)nout);
        printf("vst: setBusArrangements(%d in, %d out) -> %#llx\n", nin, nout, ULL(r & 0xffffffffu));
        r = vcall(proc, 5, 0, 0, 0, 0);
        printf("vst: canProcessSampleSize(kSample32) -> %#llx\n", ULL(r & 0xffffffffu));
    }
    /* ---- setupProcessing, setActive, setProcessing: READY ---- */
    {
        proc_setup *ps = calloc(1, sizeof *ps);
        uint64_t r;
        ps->mode = 0; ps->ssize = 0; ps->maxs = frames; ps->rate = rate;
        r = vcall(proc, 7, (uint64_t)(uintptr_t)ps, 0, 0, 0);
        printf("vst: setupProcessing(realtime, float32, max %d, %.0f Hz) -> %#llx\n", frames, rate, ULL(r & 0xffffffffu));
        r = vcall(comp, 11, 1, 0, 0, 0);
        printf("vst: IComponent::setActive(true) -> %#llx\n", ULL(r & 0xffffffffu));
        r = vcall(proc, 8, 1, 0, 0, 0);
        printf("vst: setProcessing(true) -> %#llx\n", ULL(r & 0xffffffffu));
    }
    /* ---- the block loop: one process() call per block, between mt1_ext_wait (the input block is in inL/inR) and mt1_ext_done (the output block is outL/outR) ---- */
    {
        uint32_t inL = (uint32_t)(uintptr_t)calloc((size_t)frames, 4), inR = (uint32_t)(uintptr_t)calloc((size_t)frames, 4);
        uint32_t outL = (uint32_t)(uintptr_t)calloc((size_t)frames, 4), outR = (uint32_t)(uintptr_t)calloc((size_t)frames, 4);
        static abb ibs[8], obs[8];
        static uint64_t ip[8][2], op[8][2]; /* float** as the guest sees it, one pair per bus; bus 0 is the block buffers, the other buses are silence */
        proc_data *pd = calloc(1, sizeof *pd);
        proc_ctx *pc = calloc(1, sizeof *pc);
        uint64_t h = UINT64_C(0xcbf29ce484222325), n0 = 0;
        long nonfinite = 0, nsamp = 0, nonzero = 0;
        double maxabs = 0, sumsq = 0;
        uint32_t b = 0;
        for (int k = 0; k < nin; k++) {
            ip[k][0] = k ? (uint64_t)(uintptr_t)calloc((size_t)frames, 4) : inL;
            ip[k][1] = k ? (uint64_t)(uintptr_t)calloc((size_t)frames, 4) : inR;
            ibs[k].nch = ch_in[k] > 2 ? 2 : ch_in[k]; ibs[k].silence = 0; ibs[k].buf = (uint64_t)(uintptr_t)ip[k];
        }
        for (int k = 0; k < nout; k++) {
            op[k][0] = k ? (uint64_t)(uintptr_t)calloc((size_t)frames, 4) : outL;
            op[k][1] = k ? (uint64_t)(uintptr_t)calloc((size_t)frames, 4) : outR;
            obs[k].nch = ch_out[k] > 2 ? 2 : ch_out[k]; obs[k].silence = 0; obs[k].buf = (uint64_t)(uintptr_t)op[k];
        }
        pd->mode = 0; pd->ssize = 0; pd->nin = nin; pd->nout = nout;
        pd->in = nin ? (uint64_t)(uintptr_t)ibs : 0; pd->out = nout ? (uint64_t)(uintptr_t)obs : 0;
        pd->inpc = host.param_changes; pd->outpc = host.out_changes; pd->inev = host.event_list; pd->outev = host.event_list; pd->ctx = (uint64_t)(uintptr_t)pc;
        pc->rate = rate; pc->tempo = 200.0; pc->ts_num = 4; pc->ts_den = 4;
        pc->state = (1u << 1) | (1u << 10) | (1u << 13) | (1u << 9);
        while (mt1_ext_wait(inL, inR, (uint32_t)frames, b)) {
            uint64_t r;
            pd->nsamples = frames;
            pc->proj_samples = (int64_t)n0;
            pc->proj_music = (double)n0 / rate * pc->tempo / 60.0;
            vst_ev_clear();
            /* an instrument (it has an event input bus) gets one note-on in the first block: middle C at 100/127 (main_vst3a.c --note) */
            if (b == 0 && ev_in > 0) vst_ev_note(1, 60, 100.0f / 127.0f, 0, 1);
            r = vcall(proc, 9, (uint64_t)(uintptr_t)pd, 0, 0, 0);
            if (b == 0) vst_pc_clear();
            if ((r & 0xffffffffu) != 0) {
                printf("vst: process returned %#llx at block %u\n", ULL(r & 0xffffffffu), b);
                failed = 1;
            }
            for (int c = 0; nout && c < obs[0].nch; c++) {
                const float *o = (const float *)(uintptr_t)(c ? outR : outL);
                for (int i = 0; i < frames; i++) {
                    float v = o[i];
                    if (!isfinite(v)) nonfinite++;
                    else { double av = fabs((double)v); if (av > maxabs) maxabs = av; if (v != 0.0f) nonzero++; sumsq += (double)v * (double)v; }
                    nsamp++;
                }
                h = fnv(h, o, (size_t)frames * 4);
            }
            mt1_ext_done(outL, outR, (uint32_t)frames, b);
            n0 += (uint64_t)frames;
            b++;
            if (failed) break;
        }
        printf("RESULT out_hash=%016llx nonfinite=%ld samples=%ld nonzero=%ld maxabs=%.6f rms=%.6f blocks=%u\n", ULL(h), nonfinite, nsamp, nonzero, maxabs, sqrt(sumsq / (double)(nsamp ? nsamp : 1)), b);
        printf("vst: block loop ended after %u blocks\n", b);
    }
    /* ---- teardown (main_vst3a.c) ---- */
    {
        uint64_t r;
        ncall(proc, 8, 0, 0, 0, 0);
        ncall(comp, 11, 0, 0, 0, 0);
        r = ncall(comp, 4, 0, 0, 0, 0);
        printf("vst: setProcessing(false), setActive(false), IComponent::terminate -> %#llx\n", ULL(r));
        if (ctrl && !ctrl_same) { ncall(ctrl, 4, 0, 0, 0, 0); ncall(ctrl, 2, 0, 0, 0, 0); }
        ncall(proc, 2, 0, 0, 0, 0);
        r = ncall(comp, 2, 0, 0, 0, 0);
        printf("vst: release(component) -> %llu\n", ULL(r));
        ncall(fac, 2, 0, 0, 0, 0);
        {
            uint64_t va = export_va(img.base, "ExitDll");
            const uint64_t z[6] = {0};
            if (va) { gcall(va, z, NULL); printf("vst: ExitDll() returned\n"); }
        }
    }
    printf("vst: done\n");
    return failed ? 3 : 0;
}
