/* Driver of the real-module regression test (tests/realmod.rs; the stage 1 driver of docs/investigations/stage1-two-module.md).
 * Maps the synthetic host (0x400000) and the synthetic second module (0x2000000) with harness/image.c (unchanged), maps the shared
 * log page at 0x3000000, then calls one scenario export of the host through fle_call_guest and prints the log.
 *   driver HOST.dll MOD2.dll SCENARIO [MODE]
 * The second module is NOT registered here: it exists only when the link brings a fle_realmod_table (realmod_table.c). Without one
 * the runtime has no real module. The deepest fle_callback_depth() at a call of the second module's entry point is measured with
 * -Wl,--wrap=fle_call_guest. */
#define _GNU_SOURCE
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#include "boot.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"

void fle_teb_set_stack(uint64_t lo, uint64_t hi);
uint64_t __real_fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9);

#define LOGPAGE 0x3000000ull
struct logent { uint32_t tag; uint32_t pad; uint64_t val; };
struct shlog { uint32_t n; uint32_t mode; struct logent e[1000]; };

static uint64_t mod2_entry;
static unsigned max_depth_at_entry;

uint64_t __wrap_fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9)
{
    if (va == mod2_entry && fle_callback_depth() > max_depth_at_entry) max_depth_at_entry = fle_callback_depth();
    return __real_fle_call_guest(va, rcx, rdx, r8, r9);
}

static void dump_log(void)
{
    volatile struct shlog *L = (volatile struct shlog *)LOGPAGE;
    uint32_t i;
    for (i = 0; i < L->n; i++) {
        char t[5];
        uint32_t g = L->e[i].tag;
        t[0] = (char)(g >> 24); t[1] = (char)(g >> 16); t[2] = (char)(g >> 8); t[3] = (char)g; t[4] = 0;
        printf("log %2u %s %#llx\n", i, t, (unsigned long long)L->e[i].val);
    }
    fflush(stdout);
}

static void on_abort(int sig)
{
    (void)sig;
    printf("SIGABRT (runtime stop or dispatch miss)\n");
    dump_log();
    _exit(134);
}

static uint64_t find_export(uint64_t base, const char *name)
{
    uint64_t lfanew = fle_ld(base + 0x3c, 4);
    uint64_t opt = base + lfanew + 24;
    uint64_t e = base + fle_ld(opt + 112, 4);
    uint32_t nn = (uint32_t)fle_ld(e + 24, 4);
    uint64_t afunc = base + fle_ld(e + 28, 4), anames = base + fle_ld(e + 32, 4), aord = base + fle_ld(e + 36, 4);
    for (uint32_t i = 0; i < nn; i++) {
        const char *s = (const char *)(base + fle_ld(anames + 4ull * i, 4));
        if (strcmp(s, name) == 0) return base + fle_ld(afunc + 4 * fle_ld(aord + 2ull * i, 2), 4);
    }
    return 0;
}

int main(int argc, char **argv)
{
    harness_image himg, mimg;
    char err[256];
    const char *scen;
    uint32_t mode = 0;
    uint64_t fn, rv;
    uint8_t *stk;
    void *lp;

    if (argc < 4) { fprintf(stderr, "usage: driver HOST.dll MOD2.dll SCENARIO [MODE]\n"); return 64; }
    scen = argv[3];
    if (argc > 4) mode = (uint32_t)strtoul(argv[4], 0, 0);

    if (image_load(argv[1], &himg, err, sizeof err) != 0) { fprintf(stderr, "host image: %s\n", err); return 2; }
    if (image_load(argv[2], &mimg, err, sizeof err) != 0) { fprintf(stderr, "mod2 image: %s\n", err); return 2; }
    mod2_entry = mimg.entry;
    printf("host base=%#llx size=%#llx entry=%#llx; mod2 base=%#llx size=%#llx entry=%#llx\n", (unsigned long long)himg.base,
           (unsigned long long)himg.size, (unsigned long long)himg.entry, (unsigned long long)mimg.base,
           (unsigned long long)mimg.size, (unsigned long long)mimg.entry);
    lp = mmap((void *)LOGPAGE, 0x10000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (lp != (void *)LOGPAGE) { fprintf(stderr, "log page mmap failed\n"); return 2; }
    ((struct shlog *)LOGPAGE)->mode = mode;
    signal(SIGABRT, on_abort);

    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) { perror("stack"); return 2; }
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES);

    fn = find_export(himg.base, scen);
    if (!fn) { fprintf(stderr, "no export %s in the host\n", scen); return 2; }
    printf("scenario %s at %#llx mode %u dispatch_count %llu\n", scen, (unsigned long long)fn, mode, (unsigned long long)fle_dispatch_count);
    fflush(stdout);
    {
        uint64_t sp0 = R[4];
        rv = __real_fle_call_guest(fn, 0, 0, 0, 0);
        printf("scenario returned eax=%#llx rsp_kept=%d depth=%u\n", (unsigned long long)(uint32_t)rv, R[4] == sp0, fle_callback_depth());
    }
    printf("max callback depth at a DllMain call: %u\n", max_depth_at_entry);
    dump_log();
    return 0;
}
