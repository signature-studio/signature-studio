/* Decision 0206 (dec1, opt-in, native only): a unit harness for the lifted sample-decoder DLLs (oggio_x64.dll, wavpackdll_x64.dll) of the installer.
 * Same link as the native fork-probe engine (FLEngine + the real modules + the runtime; harness/two/main_two.c maps the images the same way) but this main does not run FLEngine:
 * it maps FLEngine_x64.dll and the rebased modules, seeds the virtual volume (FLE_VFS_SEED), loads one module through the runtime's LoadLibraryW path (fle_realmod_load: DllMain
 * runs as the guest's LoadLibraryW would run it) and calls its exports through fle_call_guest, once per input file. The output lines are the ones of oggio_wine.c / wavpack_wine.c
 * (the REAL DLL under Wine), so the two outputs can be compared line by line.
 * Usage: main_dec1 <FLEngine dll> <module file>[:<module file>...] <oggio|wavpack> <list file: one guest path per line>
 * env: DEC1_DUMP=<prefix>  writes the decoded planes (oggio: float32 per channel; wavpack: int32 interleaved) to <prefix>.<n>.<ch>.raw */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/resource.h>

#include "boot.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "fle_vmap.h"
#include "vfs_seed.h"

void fle_teb_set_stack(uint64_t lo, uint64_t hi);

/* the three rows of the fake host object that the merged dispatch table of a FRUITY build names (harness/fruity-rows.txt); this harness never calls them */
void fruity_host_slot0(void) { }
void fruity_host_slot1(void) { }
void fruity_host_slot2(void) { }

#define ULL(x) ((unsigned long long)(x))

static uint64_t fnv(const void *p, size_t n, uint64_t h)
{
    const unsigned char *b = p;
    size_t i;
    for (i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; }
    return h;
}

static uint64_t entry_sp;

static uint64_t gcall(uint64_t va, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4)
{
    R[4] = entry_sp;
    return fle_call_guest(va, a1, a2, a3, a4);
}

static uint64_t export_va(uint64_t base, const char *want)
{
    uint64_t pe = base + fle_ld(base + 0x3c, 4), dd = pe + 24 + 112, dir = base + fle_ld(dd, 4);
    uint64_t nn = fle_ld(dir + 24, 4), afunc = fle_ld(dir + 28, 4), anames = fle_ld(dir + 32, 4), aord = fle_ld(dir + 36, 4), i;
    for (i = 0; i < nn; i++) {
        uint64_t nrva = fle_ld(base + anames + 4 * i, 4);
        if (strcmp((const char *)(uintptr_t)(base + nrva), want) == 0) return base + fle_ld(base + afunc + 4 * fle_ld(base + aord + 2 * i, 2), 4);
    }
    return 0;
}

static void guest_wide(uint16_t *dst, const char *s)
{
    while (*s) *dst++ = (uint16_t)(unsigned char)*s++;
    *dst = 0;
}

static void dump(const char *tag, unsigned n, int ch, const void *p, size_t bytes)
{
    const char *pre = getenv("DEC1_DUMP");
    char fn[600];
    FILE *f;
    if (!pre) return;
    snprintf(fn, sizeof fn, "%s.%s.%u.%d.raw", pre, tag, n, ch);
    f = fopen(fn, "wb");
    if (f) { fwrite(p, 1, bytes, f); fclose(f); }
}

static int run_oggio(uint64_t base, FILE *list)
{
    uint64_t loadw = export_va(base, "OggioLoadFileW"), freei = export_va(base, "OggioFreeInfo");
    char line[1024];
    unsigned n = 0;
    static uint16_t wpath[1024];
    static unsigned char info[0x100] __attribute__((aligned(16)));
    if (!loadw || !freei) { printf("dec1: no OggioLoadFileW / OggioFreeInfo export\n"); return 2; }
    while (fgets(line, sizeof line, list)) {
        size_t l = strlen(line);
        uint64_t rc, hh = 14695981039346656037ULL;
        int rate, ch, c;
        uint64_t frames;
        uint64_t planes;
        while (l && (line[l - 1] == '\n' || line[l - 1] == '\r')) line[--l] = 0;
        if (!l) continue;
        memset(info, 0, sizeof info);
        guest_wide(wpath, line);
        rc = gcall(loadw, (uint64_t)(uintptr_t)wpath, (uint64_t)(uintptr_t)info, 0, 0) & 0xffffffffu;
        rate = *(int *)(info + 0); ch = *(int *)(info + 4); frames = *(uint64_t *)(info + 0x2c); planes = *(uint64_t *)(info + 0x0c);
        printf("file %s rc=%d rate=%d ch=%d frames=%llu", line, (int)rc, rate, ch, ULL(frames));
        if (rc == 0 && ch > 0 && ch <= 8 && planes) {
            for (c = 0; c < ch; c++) {
                const float *pl = (const float *)(uintptr_t)fle_ld(planes + 8u * (unsigned)c, 8);
                uint64_t hc = fnv(pl, (size_t)frames * 4, 14695981039346656037ULL);
                printf(" ch%d=%016llx", c, ULL(hc));
                hh = fnv(&hc, 8, hh);
                dump("f32", n, c, pl, (size_t)frames * 4);
            }
            printf(" all=%016llx", ULL(hh));
            { int k; for (k = 0; k < 2; k++) { uint64_t q = *(uint64_t *)(info + (k ? 0x1c : 0x14)); int j; printf(" s%s=\"", k ? "1c" : "14"); if (q) for (j = 0; j < 160; j++) { uint8_t c = (uint8_t)fle_ld(q + (unsigned)j, 1); if (!c) break; if (c >= 0x20 && c < 0x7f && c != 0x22 && c != 0x5c) putchar(c); else printf("\\x%02x", c); } printf("\""); } }
            gcall(freei, (uint64_t)(uintptr_t)info, 0, 0, 0);
        }
        printf("\n");
        fflush(stdout);
        n++;
    }
    return 0;
}

static int run_wavpack(uint64_t base, FILE *list)
{
    uint64_t open_ = export_va(base, "WavpackOpenFileInput"), nsamp = export_va(base, "WavpackGetNumSamples"), nch = export_va(base, "WavpackGetNumChannels");
    uint64_t rate = export_va(base, "WavpackGetSampleRate"), bps = export_va(base, "WavpackGetBitsPerSample"), mode = export_va(base, "WavpackGetMode");
    uint64_t unpack = export_va(base, "WavpackUnpackSamples"), close_ = export_va(base, "WavpackCloseFile"), nerr = export_va(base, "WavpackGetNumErrors");
    char line[1024];
    unsigned n = 0;
    static char err[256];
    static int32_t buf[4096 * 8] __attribute__((aligned(16)));
    if (!open_ || !unpack || !close_) { printf("dec1: wavpack exports missing\n"); return 2; }
    while (fgets(line, sizeof line, list)) {
        size_t l = strlen(line);
        uint64_t wpc, hh = 14695981039346656037ULL, total = 0, got;
        uint32_t ns, sr, m;
        int ch, bits;
        while (l && (line[l - 1] == '\n' || line[l - 1] == '\r')) line[--l] = 0;
        if (!l) continue;
        err[0] = 0;
        wpc = gcall(open_, (uint64_t)(uintptr_t)line, (uint64_t)(uintptr_t)err, 0x80 /* OPEN_FILE_UTF8 */, 0);
        printf("file %s", line);
        if (!wpc) { printf(" open=NULL error=\"%s\"\n", err); fflush(stdout); n++; continue; }
        ns = (uint32_t)gcall(nsamp, wpc, 0, 0, 0); ch = (int)gcall(nch, wpc, 0, 0, 0); sr = (uint32_t)gcall(rate, wpc, 0, 0, 0); bits = (int)gcall(bps, wpc, 0, 0, 0); m = (uint32_t)gcall(mode, wpc, 0, 0, 0);
        printf(" samples=%u ch=%d rate=%u bits=%d mode=%#x", ns, ch, sr, bits, m);
        if (ch > 0 && ch <= 8) {
            unsigned chunk = (unsigned)(sizeof buf / sizeof buf[0]) / (unsigned)ch;
            FILE *df = NULL;
            if (getenv("DEC1_DUMP")) { char fn[600]; snprintf(fn, sizeof fn, "%s.i32.%u.raw", getenv("DEC1_DUMP"), n); df = fopen(fn, "wb"); }
            for (;;) {
                got = gcall(unpack, wpc, (uint64_t)(uintptr_t)buf, chunk, 0) & 0xffffffffu;
                if (!got) break;
                hh = fnv(buf, (size_t)got * (unsigned)ch * 4, hh);
                if (df) fwrite(buf, 4, (size_t)got * (unsigned)ch, df);
                total += got;
            }
            if (df) fclose(df);
        }
        printf(" decoded=%llu errors=%u all=%016llx\n", ULL(total), nerr ? (unsigned)gcall(nerr, wpc, 0, 0, 0) : 0u, ULL(hh));
        gcall(close_, wpc, 0, 0, 0);
        fflush(stdout);
        n++;
    }
    return 0;
}

int main(int argc, char **argv)
{
    harness_image img, rimg;
    char err[256];
    char *rcopy, *tok;
    uint8_t *stk;
    int nreal = 0, k, idx;
    uint64_t base, lib;
    FILE *list;
    const char *mode, *modname;
    struct rlimit rl = {HARNESS_MAX_AS_BYTES, HARNESS_MAX_AS_BYTES};

    if (argc < 5) { fprintf(stderr, "usage: %s <FLEngine dll> <module file>[:...] <oggio|wavpack> <list>\n", argv[0]); return 64; }
    mode = argv[3];
    modname = !strcmp(mode, "oggio") ? "oggio_x64.dll" : "wavpackdll_x64.dll";
    if (setrlimit(RLIMIT_AS, &rl) != 0) { perror("setrlimit"); return 2; }
    trace_init();
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "image: %s\n", err); return 2; }
    fle_vmap_register_image(img.base);
    rcopy = strdup(argv[2]);
    for (tok = strtok(rcopy, ":"); tok; tok = strtok(NULL, ":")) {
        if (image_load(tok, &rimg, err, sizeof err) != 0) { fprintf(stderr, "real module image: %s (%s)\n", err, tok); return 2; }
        fle_vmap_register_image(rimg.base);
        nreal++;
    }
    if ((int)fle_realmod_count != nreal) { fprintf(stderr, "fle_realmod_table has %llu entries and %d module files were mapped\n", ULL(fle_realmod_count), nreal); return 2; }
    if (vfs_seed_from_env() != 0) return 2;
    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) { perror("mmap stack"); return 2; }
    entry_sp = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    R[4] = entry_sp;
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES);
    fle_vmap_register_range((uint64_t)(uintptr_t)stk, HARNESS_STACK_BYTES, 0x04u, 0x20000u);
    idx = fle_realmod_find(modname);
    if (idx < 0) { printf("dec1: no real module \"%s\"\n", modname); return 2; }
    R[4] -= 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    lib = fle_realmod_load("LoadLibraryW", idx);
    R[4] += 8;
    base = fle_realmod_base_of(idx);
    printf("dec1: %s loaded: handle=%#llx base=%#llx\n", modname, ULL(lib), ULL(base));
    fflush(stdout);
    list = fopen(argv[4], "r");
    if (!list) { perror(argv[4]); return 2; }
    k = !strcmp(mode, "oggio") ? run_oggio(base, list) : run_wavpack(base, list);
    trace_report("dec1 done");
    return k;
}
