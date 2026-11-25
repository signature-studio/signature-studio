/* Two-module harness main (stage 2/3 of docs/investigations/two-module-design.md section 7; decisions 0080 and 0082).
 * The same source builds the native harness and the wasm harness (like lift/harness/main.c and lift/harness/wasm/main.c, which
 * are not changed). It maps FLEngine_x64.dll at its ImageBase and the rebased real module (a file made by `fe-pe rebase`, at the
 * base that the link names in fle_realmod_table) at that base, then calls the DllMain-equivalent entry of FLEngine as the
 * single-module harness does. Loading the real module is done by the guest's own LoadLibraryW (fle_realmod.c).
 * Issue #1341: a real module's file is not read at start. It is matched to its table entry by name here, and its image is read and mapped when the guest first asks for the module (fle_realmod_map_image).
 * Usage: main_two <FLEngine dll> <real module file>[:<real module file>...] [entry_va_hex] */
#define _GNU_SOURCE
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#include <sys/resource.h>
#else
#include "fle_host.h"
#endif

#include "a4.h"
#include "boot.h"
#include "fle_dispatch.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "fle_vmap.h" /* decision 0123: fle_vmap_register_image / fle_vmap_register_range (what VirtualQuery describes) */
#include "vfs_seed.h"
#ifdef FLE_FRUITY
#include "fruity.h" /* stage 1 of CreateFruityInstance (docs/investigations/create-fruity-instance-harness.md): only a build with FRUITY=1 compiles it */
#include "reg_seed.h" /* decision 0116: FLE_REG_SEED, registry values written before the guest runs (the same build only) */
#include "authenticode_claims.h" /* decision 0117: FLE_AUTH_CLAIMS, the table of Authenticode claims that WinVerifyTrust answers from (the same build only) */
#endif
#ifdef FLE_B2D_PROBE
#include "b2d_probe.h" /* X1 (docs/investigations/blend2d-jit-and-drawing-path.md): only a build with B2D_PROBE=1 compiles it */
#endif
#ifdef FLE_B2D_SCAN
#include "b2d_scan.h" /* X2 (docs/investigations/blend2d-nojit-slots.md): only a build with B2D_SCAN=1 compiles it */
#endif

void fle_teb_set_stack(uint64_t lo, uint64_t hi); /* runtime/fle_vex.h (weak definition in every generated unit), decision 0076 */

static void (*find_fn(uint64_t va))(void) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) {
        if (fle_dispatch_table[i].va == va) return fle_dispatch_table[i].fn;
    }
    return NULL;
}

/* Decision 0183 (wrp1, PROVISIONAL): a hook that the link supplies (harness/two/wrp1_vst3.c, linked into every engine build by Issue #1326): called after a real module is mapped, with its base and size. */
void wrp1_after_image_load(uint64_t base, uint64_t size) __attribute__((weak));
/* fruity.c (FLE_FRUITY builds, fruity.h) registers an image for the FRUITY harness; a weak reference keeps the other builds linking */
void fruity_add_image(uint64_t base, uint64_t size) __attribute__((weak));
/* Issue #1342: the page instantiates the separate wasm module of a real module (fe-wasmgen hybrid --separate; app/src/engine/xmod.ts) and returns 1; 0 when the module has none, and the page's
 * import is missing in a link that has no separate module. Weak: a build without the import keeps building (the native harness). */
int fle_xmod_load(const char *dll_path) __attribute__((weak));

/* N real modules: argv[2] is one file or several separated by ':'. One file: the output is the one of the one-module main_two.c.
 * MAIN_REAL_MAX only sizes these arrays. It is larger than the RM_MAX (16) of fle_realmod.c on purpose: more files than the runtime
 * holds are stopped by the runtime (fle_realmod.c table_check, by name), not by a second limit here. */
enum { MAIN_REAL_MAX = 32 };
static char *rpath[MAIN_REAL_MAX];
static int rm_file_of[MAIN_REAL_MAX];   /* #1341: the file (index into rpath) of each table entry, -1 when none */
static struct timespec t_start;

static double ms_since_start(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)(t.tv_sec - t_start.tv_sec) * 1000.0 + (double)(t.tv_nsec - t_start.tv_nsec) / 1e6;
}

/* #1341: the name fle_realmod_table gives a real module, from its file name "<stem>.<base hex>.dll" (fe-lift writes a rebased file so, decision 0080 D1): the stem in lower case and ".dll",
 * so "fruity_balance_x64.6800000.dll" is "fruity_balance_x64.dll". Returns 0, or -1 when the file name has no base suffix. */
static int rm_table_name(const char *path, char *out, size_t n) {
    const char *base = strrchr(path, '/'), *ext, *p;
    size_t len, i, stem;
    base = base ? base + 1 : path;
    len = strlen(base);
    if (len < 5 || strcmp(base + len - 4, ".dll") != 0) return -1;
    ext = base + len - 4;
    for (p = ext; p > base && p[-1] != '.'; p--) {}
    if (p == base || p - 1 == base) return -1;
    for (i = 0; p + i < ext; i++)
        if (!isxdigit((unsigned char)p[i])) return -1;
    if (i == 0) return -1;
    stem = (size_t)(p - 1 - base);
    if (stem + 5 > n) return -1;
    for (i = 0; i < stem; i++) out[i] = (char)tolower((unsigned char)base[i]);
    memcpy(out + stem, ".dll", 5);
    return 0;
}

/* #1341: fle_realmod.c calls this when the guest first asks for real module idx (LoadLibrary*), before the module's headers are read. The file is read now: the whole PE image is mapped
 * at its base, and the read is logged on stderr (image-read: <file name> requested=1 <ms since the start of main>). A file that cannot be mapped, or whose ImageBase is not the table's base,
 * stops the run by name; no empty image is made. requested is 1 because the only caller is the guest's request. */
void fle_realmod_map_image(const char *api, int idx) {
    const char *path = rpath[rm_file_of[idx]];
    const char *name = strrchr(path, '/');
    harness_image rimg;
    char err[256], why[384];
    name = name ? name + 1 : path;
    fprintf(stderr, "image-read: %s requested=1 %.1f\n", name, ms_since_start());
    if (image_load(path, &rimg, err, sizeof err) != 0) {
        snprintf(why, sizeof why, "real module \"%s\": the file %s could not be mapped: %s", fle_realmod_table[idx].name, path, err);
        fle_stop(api, why);
    }
    if (rimg.base != fle_realmod_table[idx].base) {
        snprintf(why, sizeof why, "real module \"%s\": the file %s is mapped at %#llx, the table gives %#llx", fle_realmod_table[idx].name, path,
                 (unsigned long long)rimg.base, (unsigned long long)fle_realmod_table[idx].base);
        fle_stop(api, why);
    }
    printf("real module image: base=%#llx size=%#llx entry=%#llx imports(cumulative)=%zu exports(cumulative)=%zu\n",
           (unsigned long long)rimg.base, (unsigned long long)rimg.size,
           (unsigned long long)rimg.entry, rimg.nimports, rimg.nexports);
    printf("real module table: name=%s base=%#llx\n", fle_realmod_table[idx].name, (unsigned long long)fle_realmod_table[idx].base);
    if (fle_xmod_load && fle_xmod_load(path)) { /* #1342: the code of this module is a separate wasm module, instantiated by the page at this request; one line, as image-read above */
        char wname[256];
        snprintf(wname, sizeof wname, "%.*s.wasm", (int)(strlen(name) - 4), name);
        fprintf(stderr, "image-read: %s requested=1 %.1f\n", wname, ms_since_start());
    }
    if (fruity_add_image) fruity_add_image(rimg.base, rimg.size);
    fle_vmap_register_image(rimg.base); /* decision 0123 */
    if (wrp1_after_image_load) wrp1_after_image_load(rimg.base, rimg.size); /* decision 0183 (harness/two/wrp1_vst3.c; for a module that is not at WRP1_VST3_BASE it does nothing) */
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "FLEngine_x64.dll";
    const char *real = argc > 2 ? argv[2] : NULL;
    harness_image img;
    char err[256];
    uint64_t entry;
    void (*fn)(void);
    uint8_t *stk;
    char *rcopy = NULL;
    int nreal = 0;
#ifdef FLE_FRUITY
    uint64_t fruity_sp = 0;
#endif
#ifdef FLE_B2D_PROBE
    uint64_t b2d_sp = 0;
#endif
#ifdef FLE_B2D_SCAN
    uint64_t scan_sp = 0;
#endif

    if (!real) {
        fprintf(stderr, "usage: %s <FLEngine dll> <real module file>[:<real module file>...] [entry_va_hex]\n", argv[0]);
        return 64;
    }
    rcopy = strdup(real);
    if (!rcopy) {
        perror("strdup");
        return 2;
    }
    for (char *tok = strtok(rcopy, ":"); tok; tok = strtok(NULL, ":")) {
        if (nreal == MAIN_REAL_MAX) {
            fprintf(stderr, "more than %d real module files in %s\n", MAIN_REAL_MAX, real);
            return 64;
        }
        rpath[nreal++] = tok;
    }
    if (nreal == 0) {
        fprintf(stderr, "no real module file in \"%s\"\n", real);
        return 64;
    }
#ifndef __EMSCRIPTEN__
    {
        struct rlimit rl = {HARNESS_MAX_AS_BYTES, HARNESS_MAX_AS_BYTES};
        if (setrlimit(RLIMIT_AS, &rl) != 0) {
            perror("setrlimit");
            return 2;
        }
    }
#endif
    clock_gettime(CLOCK_MONOTONIC, &t_start);
    trace_init();
    if (image_load(path, &img, err, sizeof err) != 0) {
        fprintf(stderr, "image: %s\n", err);
        return 2;
    }
    a4_set_image(img.base, img.size); /* the a4 survey reads FLEngine's InitContext only */
    fle_vmap_register_image(img.base); /* decision 0123: VirtualQuery describes the loaded image from its section table */
#ifdef FLE_FRUITY
    fruity_add_image(img.base, img.size);
#endif
    printf("image: base=%#llx size=%#llx entry=%#llx imports=%zu exports=%zu\n",
           (unsigned long long)img.base, (unsigned long long)img.size,
           (unsigned long long)img.entry, img.nimports, img.nexports);

    /* #1341: no real module file is read here. Each file is matched to its table entry by name (rm_table_name); the image is mapped when the guest asks for it (fle_realmod_map_image). */
    if (fle_realmod_count > MAIN_REAL_MAX) {
        fprintf(stderr, "fle_realmod_table has %llu entries (at most %d are matched)\n", (unsigned long long)fle_realmod_count, MAIN_REAL_MAX);
        return 2;
    }
    for (uint64_t i = 0; i < fle_realmod_count; i++) rm_file_of[i] = -1;
    for (int k = 0; k < nreal; k++) {
        char want[64];
        int idx = -1;
        if (rm_table_name(rpath[k], want, sizeof want) != 0) {
            fprintf(stderr, "the real module file %s has no base suffix (<name>.<base hex>.dll)\n", rpath[k]);
            return 2;
        }
        for (uint64_t i = 0; i < fle_realmod_count; i++)
            if (strcmp(fle_realmod_table[i].name, want) == 0) idx = (int)i;
        if (idx < 0) {
            fprintf(stderr, "the file %s (name %s) names no entry of fle_realmod_table\n", rpath[k], want);
            return 2;
        }
        if (rm_file_of[idx] >= 0) {
            fprintf(stderr, "the files %s and %s name the same real module %s\n", rpath[rm_file_of[idx]], rpath[k], want);
            return 2;
        }
        rm_file_of[idx] = k;
    }
    /* every table entry must have a file (fle_realmod.c would otherwise stop at the first LoadLibrary of it) */
    if (fle_realmod_count != (uint64_t)nreal) {
        fprintf(stderr, "fle_realmod_table has %llu entries and %d real module files were given\n",
                (unsigned long long)fle_realmod_count, nreal);
        return 2;
    }

    entry = argc > 3 ? strtoull(argv[3], NULL, 0) : img.entry;
    fn = find_fn(entry);
    if (!fn) {
        printf("entry %#llx is not in fle_dispatch_table (not a lifted function entry)\n",
               (unsigned long long)entry);
        return 2;
    }

    /* FLE_VFS_SEED=<list> (decision 0110, provisional): product files of the installer written to the virtual volume before the guest runs.
     * Unset: vfs_seed_from_env returns 0 and does nothing. R[] is put back as it was. */
    if (vfs_seed_from_env() != 0) return 2;
#ifdef FLE_FRUITY
    /* FLE_REG_SEED=<list>[:<list>...] (decision 0116, provisional): values written to the virtual registry before the guest runs. Compiled only into
     * a FRUITY build. Unset: reg_seed_from_env returns 0 and does nothing. R[] is put back as it was. */
    if (reg_seed_from_env() != 0) return 2;
    /* Issue #1383: the HKCU values the page saved are replayed after the seed (a saved value replaces the seed's); the page's library only (fle_reg.c) */
    fle_reg_restore_saved();
    /* FLE_AUTH_CLAIMS=<tsv> (decision 0117, provisional): the claims of tools/authenticode_check.py registered with the runtime under their guest paths. Compiled only
     * into a FRUITY build. Unset: auth_claims_from_env returns 0 and registers nothing (the runtime's table stays empty and WinVerifyTrust stops by name). */
    if (auth_claims_from_env(path) != 0) return 2;
    if (fruity_precheck() != 0) return 2; /* FLE_CALL_FRUITY / FLE_FRUITY_*: read and checked before the guest runs; off = does nothing */
#endif
#ifdef FLE_B2D_PROBE
    if (b2d_probe_precheck() != 0) return 2; /* FLE_BLEND2D_PROBE / FLE_B2D_*: read and checked before the guest runs; off = does nothing */
#endif
#ifdef FLE_B2D_SCAN
    {
        int sc = b2d_scan_precheck(); /* FLE_B2D_SCAN=<test>: read and checked before the guest runs; `list` prints the tests and ends; unset = does nothing */
        if (sc < 0) return 2;
        if (sc > 0) return 0;
    }
#endif

#ifndef __EMSCRIPTEN__
    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) {
        perror("mmap stack");
        return 2;
    }
#else
    stk = aligned_alloc(4096, HARNESS_STACK_BYTES); /* page-aligned lower bound: StackLimit is that bound (decision 0076) */
    if (!stk) {
        perror("aligned_alloc stack");
        return 2;
    }
#endif
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES); /* decision 0076 */
    fle_vmap_register_range((uint64_t)(uintptr_t)stk, HARNESS_STACK_BYTES, 0x04u /* PAGE_READWRITE */, 0x20000u /* MEM_PRIVATE */); /* decision 0123: VirtualQuery inside the first thread's stack */
#ifdef FLE_FRUITY
    fruity_sp = R[4];
#endif
#ifdef FLE_B2D_PROBE
    b2d_sp = R[4];
#endif
#ifdef FLE_B2D_SCAN
    scan_sp = R[4];
#endif
    R[1] = img.base;
    R[2] = 1;
    R[8] = 0;
    printf("calling entry %s rcx=%#llx rdx=1 r8=0 rsp=%#llx\n",
           image_export_name(entry) ? image_export_name(entry) : "(no export name)",
           (unsigned long long)R[1], (unsigned long long)R[4]);
    fflush(stdout);

    R[4] -= 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    fn();
    R[4] += 8;

    trace_report("returned from entry");
    printf("rax=%#llx\n", (unsigned long long)R[0]);
#ifdef FLE_FRUITY
    if (fruity_requested()) return fruity_run(fruity_sp); /* FLE_CALL_FRUITY=1: after the DllMain report above, which is printed as before */
#endif
#ifdef FLE_B2D_PROBE
    if (b2d_probe_requested()) return b2d_probe_run(b2d_sp); /* FLE_BLEND2D_PROBE=1: after the DllMain report above, which is printed as before */
#endif
#ifdef FLE_B2D_SCAN
    if (b2d_scan_requested()) return b2d_scan_run(scan_sp); /* FLE_B2D_SCAN=<test>: after the DllMain report above, which is printed as before */
#endif
    return 0;
}
