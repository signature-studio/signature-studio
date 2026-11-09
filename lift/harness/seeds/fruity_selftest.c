/* Self-test of lift/harness/fruity_host.h (the fake host object of CreateFruityInstance, docs/investigations/create-fruity-instance-harness.md).
 * Links nothing of the runtime or of the lifted code: it defines R and a three-row fle_dispatch_table of its own, builds the object, and calls the
 * stubs the way lifted code does (read the vtable pointer from the object, read the slot, fle_dispatch the VA in it). It writes nothing.
 *
 *   cd lift && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness harness/seeds/fruity_selftest.c -o $OUT/fruity_selftest \
 *       && $OUT/fruity_selftest harness/fruity-rows.txt
 *
 * The argument (optional) is lift/harness/fruity-rows.txt: when it is given the self-test also checks that the rows file and the header name the same
 * three VAs and functions. Prints "ok  <name>" or "FAIL <name>" per check; the exit code is the number of failures (0 = all passed). The expected
 * values are written out literally from the readings of FL64.exe (the header comment gives the addresses), not computed with the code under test.
 * STAGE 1: this file was only syntax-checked (clang -fsyntax-only), it was not compiled to a program or run. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FRUITY_HOST_IMPL
#include "fruity_host.h"

uint64_t R[16];

const fle_fn_entry fle_dispatch_table[] = {
    {FRUITY_STUB_VA_BASE + UINT64_C(0x10), fruity_host_slot0},
    {FRUITY_STUB_VA_BASE + UINT64_C(0x20), fruity_host_slot1},
    {FRUITY_STUB_VA_BASE + UINT64_C(0x30), fruity_host_slot2},
};
const uint64_t fle_dispatch_count = 3;

static int failures;

/* the registers that engine_call sets (rax to a marker, rcx, rdx, r8, r9); a stub may change rax, rdx, r8, r9 */
#define ARGS_MASK (1u << 0 | 1u << 1 | 1u << 2 | 1u << 8 | 1u << 9)

static void check(int cond, const char *name) {
    printf("%s %s\n", cond ? "ok  " : "FAIL", name);
    if (!cond) failures++;
}

static void set_pattern(void) {
    int i;
    for (i = 0; i < 16; i++) R[i] = UINT64_C(0x1111000000000000) + (uint64_t)i;
}

/* every register except the ones in `except` (a bit mask over R[]) still has the pattern */
static int pattern_intact_except(unsigned except) {
    int i;
    for (i = 0; i < 16; i++) {
        if ((except >> i) & 1u) continue;
        if (R[i] != UINT64_C(0x1111000000000000) + (uint64_t)i) return 0;
    }
    return 1;
}

/* `mov rax, [rcx]; call [rax + 8 * slot]` of the engine: the target VA comes out of the object's own vtable, the call goes through fle_dispatch. */
static void engine_call(const fruity_host *h, unsigned slot, uint64_t a2, uint64_t a8, uint64_t a9) {
    uint64_t vt = fle_ld(h->obj + FRUITY_OFF_VTABLE, 8);
    uint64_t target = fle_ld(vt + 8u * slot, 8);
    set_pattern();
    R[0] = UINT64_C(0xAAAAAAAAAAAAAAAA);
    R[1] = h->obj;
    R[2] = a2;
    R[8] = a8;
    R[9] = a9;
    fle_dispatch(target);
}

static int read_rows(const char *path) {
    FILE *f = fopen(path, "r");
    char line[256];
    int n = 0, ok = 1;
    if (!f) {
        printf("FAIL cannot open %s\n", path);
        return -1;
    }
    while (fgets(line, sizeof line, f)) {
        char *hash = strchr(line, '#');
        unsigned long long va;
        char name[128], want[64];
        if (hash) *hash = 0;
        if (sscanf(line, "%llx %127s", &va, name) != 2) continue;
        snprintf(want, sizeof want, "fruity_host_slot%d", n);
        if (n >= 3 || va != FRUITY_STUB_VA(n) || strcmp(name, want) != 0) ok = 0;
        n++;
    }
    fclose(f);
    return ok && n == 3;
}

int main(int argc, char **argv) {
    static const char path[] = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
    static const char dir[] = "C:\\Program Files\\Image-Line\\FL Studio 21\\";
    fruity_host h;
    int id, ix, bad;
    uint64_t vt;

    check(FRUITY_HOST_SIZE == 0x38u && FRUITY_OFF_VTABLE == 0u && FRUITY_OFF_VERSION == 8u && FRUITY_OFF_FLAGS == 0xCu &&
              FRUITY_OFF_QWORD10 == 0x10u && FRUITY_OFF_NAME == 0x18u && FRUITY_OFF_STR2 == 0x20u && FRUITY_OFF_STR3 == 0x28u &&
              FRUITY_OFF_BASEDIR == 0x30u,
          "object layout constants are FL64.exe's (0x38 bytes; 0, 8, 0xc, 0x10, 0x18, 0x20, 0x28, 0x30)");
    check(FRUITY_CALL_RCX == 0xC0 && FRUITY_CALL_RDX == 0xEDE95A, "the call arguments are 0xC0 and 0xEDE95A");
    check(FRUITY_STUB_VA(0) == UINT64_C(0x7FFB00000010) && FRUITY_STUB_VA(1) == UINT64_C(0x7FFB00000020) &&
              FRUITY_STUB_VA(2) == UINT64_C(0x7FFB00000030),
          "stub VAs are 0x7FFB00000010, 0x7FFB00000020, 0x7FFB00000030");
    check(FRUITY_VTABLE_SLOTS == 3u && FRUITY_BASEDIR_CAP == 0x8000u, "3 vtable slots, 0x8000-byte base directory buffer");
    check(FRUITY_BLOCK_VTABLE >= FRUITY_HOST_SIZE && FRUITY_BLOCK_NAME >= FRUITY_BLOCK_VTABLE + 8u * FRUITY_VTABLE_SLOTS &&
              FRUITY_BLOCK_EMPTY >= FRUITY_BLOCK_NAME + (unsigned)sizeof FRUITY_PRODUCT_NAME && FRUITY_BLOCK_BASEDIR >= FRUITY_BLOCK_EMPTY + 1u,
          "the pieces of the host block do not overlap");

    check(fruity_dir_len("C:\\a\\b.dll") == 5, "dir_len: C:\\a\\b.dll -> 5 (trailing separator included)");
    check(fruity_dir_len("C:/a/b.dll") == 5, "dir_len: forward slashes count");
    check(fruity_dir_len("C:\\x/y\\z.dll") == 7, "dir_len: the LAST of both separators");
    check(fruity_dir_len("b.dll") == 0, "dir_len: no separator -> 0 (empty string)");
    check(fruity_dir_len("") == 0, "dir_len: empty path -> 0");
    check(fruity_dir_len("C:\\") == 3, "dir_len: C:\\ -> 3");

    check(fruity_host_make(&h, path) == 0, "fruity_host_make succeeds");
    check(h.mem != NULL && h.obj == (uint64_t)(uintptr_t)h.mem, "the object is at the start of the block");
    check(h.basedir_len == strlen(dir), "base directory length");
    check(fle_ld(h.obj + FRUITY_OFF_VTABLE, 8) == h.vtable, "+0x00 is the vtable");
    check(fle_ld(h.obj + FRUITY_OFF_VERSION, 4) == 1, "+0x08 HostVersion = 1");
    check(fle_ld(h.obj + FRUITY_OFF_FLAGS, 4) == 0, "+0x0c Flags = 0");
    check(fle_ld(h.obj + FRUITY_OFF_QWORD10, 8) == 0, "+0x10 = 0");
    check(fle_ld(h.obj + FRUITY_OFF_NAME, 8) == h.name && strcmp((const char *)(uintptr_t)h.name, "FL Studio") == 0, "+0x18 -> \"FL Studio\"");
    check(fle_ld(h.obj + FRUITY_OFF_STR2, 8) == h.empty && fle_ld(h.obj + FRUITY_OFF_STR3, 8) == h.empty &&
              ((const char *)(uintptr_t)h.empty)[0] == 0,
          "+0x20 and +0x28 -> the same empty string");
    check(fle_ld(h.obj + FRUITY_OFF_BASEDIR, 8) == h.basedir && strcmp((const char *)(uintptr_t)h.basedir, dir) == 0,
          "+0x30 -> the directory of the engine path, with its trailing backslash");
    check(((const char *)(uintptr_t)h.basedir)[strlen(dir)] == 0 && ((const char *)(uintptr_t)h.basedir)[FRUITY_BASEDIR_CAP - 1] == 0,
          "the base directory buffer is NUL-terminated and zero-filled to 0x8000 bytes");
    vt = fle_ld(h.obj + FRUITY_OFF_VTABLE, 8);
    check(fle_ld(vt, 8) == UINT64_C(0x7FFB00000010) && fle_ld(vt + 8, 8) == UINT64_C(0x7FFB00000020) && fle_ld(vt + 16, 8) == UINT64_C(0x7FFB00000030),
          "the vtable holds the three stub VAs");
    check(FRUITY_OFF_BASEDIR + 8u == FRUITY_HOST_SIZE && fle_ld(h.obj + 0x34, 4) == (h.basedir >> 32),
          "the pointer at +0x30 ends exactly at the object size 0x38 (no padding: +0x34 is the high half of the base directory pointer)");

    /* fruity_answer: only (id 3, index 0 or 1) is 1, over a literal grid */
    bad = 0;
    for (id = -3; id <= 6; id++) {
        for (ix = -3; ix <= 3; ix++) {
            uint64_t want = (id == 3 && (ix == 0 || ix == 1)) ? 1 : 0;
            if (fruity_answer((uint64_t)(int64_t)id, (uint64_t)(int64_t)ix) != want) bad++;
        }
    }
    check(bad == 0, "fruity_answer is 1 exactly for id 3 with index 0 or 1 (negative ids and indexes included)");
    check(fruity_answer(3, UINT64_C(0xFFFFFFFF)) == 0 && fruity_answer(UINT64_C(0x100000003), 0) == 0, "fruity_answer: the compare is on 64 bits, unsigned");

    /* through slot 2 */
    fruity_host_log_reset();
    engine_call(&h, 2, 3, 1, 99);
    check(R[0] == 1 && pattern_intact_except(ARGS_MASK), "slot 2 (3, 1, 99) -> 1, only rax changes of the registers it was given");
    engine_call(&h, 2, 3, 2, 0);
    check(R[0] == 0, "slot 2 (3, 2, 0) -> 0");
    engine_call(&h, 2, 2, 0, 0);
    check(R[0] == 0, "slot 2 (2, 0, 0) -> 0");
    engine_call(&h, 2, UINT64_C(0x100000003), 0, 0);
    check(R[0] == 0, "slot 2 does not truncate the id to 32 bits");
    engine_call(&h, 2, 3, UINT64_C(0xFFFFFFFF), 0);
    check(R[0] == 0, "slot 2 does not sign-extend the index (4294967295 is not <= 1)");
    check(fruity_host_calls(2) == 5 && fruity_host_calls(0) == 0 && fruity_host_calls(1) == 0, "counters after 5 slot-2 calls");

    /* through slot 0: sign extension of edx, r8d, r9d, then slot 2 through the object's own vtable */
    fruity_host_log_reset();
    engine_call(&h, 0, 3, 1, 0);
    check(R[0] == 1, "slot 0 (3, 1, 0) -> 1 (forwarded to slot 2)");
    engine_call(&h, 0, UINT64_C(0xFFFFFFFF00000003), UINT64_C(0xAAAAAAAA00000001), 0);
    check(R[0] == 1, "slot 0 uses only the low 32 bits of rdx and r8");
    engine_call(&h, 0, 3, UINT64_C(0xFFFFFFFF), 0);
    check(R[0] == 0, "slot 0 (3, r8d = -1) -> 0: index -1 is sign-extended and is not <= 1");
    engine_call(&h, 0, UINT64_C(0x80000003), 0, 0);
    check(R[0] == 0, "slot 0: edx = 0x80000003 is a negative int, not 3");
    check(R[2] == UINT64_C(0xFFFFFFFF80000003) && R[8] == 0 && R[9] == 0, "slot 0 left the sign-extended arguments in rdx, r8, r9 (as the movsxd of FL64.exe)");
    engine_call(&h, 0, 4, 0, 5);
    check(R[0] == 0 && pattern_intact_except(ARGS_MASK), "slot 0 (4, 0, 5) -> 0; other registers untouched");
    check(fruity_host_calls(0) == 5 && fruity_host_calls(2) == 5, "5 slot-0 calls were forwarded to 5 slot-2 calls");
    {
        const fruity_call *c = fruity_host_log_at(0);
        check(c && c->slot == 2 && c->via_slot0 == 1 && c->id == 3 && c->index == 1 && c->value == 0 && c->result == 1, "log #0: slot 2 via slot 0, (3, 1, 0) -> 1");
        c = fruity_host_log_at(2);
        check(c && c->slot == 2 && c->via_slot0 == 1 && c->id == 3 && c->index == -1 && c->result == 0, "log #2: index logged as the sign-extended -1");
        check(fruity_host_log_at(5) == NULL && fruity_host_log_total() == 5, "log has 5 entries");
    }

    /* slot 1: nothing changes, not even rax */
    fruity_host_log_reset();
    engine_call(&h, 1, 7, 8, 9);
    check(R[0] == UINT64_C(0xAAAAAAAAAAAAAAAA) && R[2] == 7 && R[8] == 8 && R[9] == 9 && pattern_intact_except(ARGS_MASK),
          "slot 1 changes no register (rax keeps its value)");
    check(fruity_host_calls(1) == 1 && fruity_host_calls(0) == 0 && fruity_host_calls(2) == 0, "slot 1 counted once, nothing forwarded");
    {
        const fruity_call *c = fruity_host_log_at(0);
        check(c && c->slot == 1 && c->id == 7 && c->index == 8 && c->via_slot0 == 0, "log #0: slot 1, edx 7, r8d 8");
    }

    /* the log keeps the first FRUITY_LOG_MAX calls and counts the rest */
    fruity_host_log_reset();
    for (id = 0; id < (int)FRUITY_LOG_MAX + 10; id++) engine_call(&h, 2, 3, 0, (uint64_t)id);
    check(fruity_host_log_total() == FRUITY_LOG_MAX + 10 && fruity_host_log_at(FRUITY_LOG_MAX - 1) != NULL && fruity_host_log_at(FRUITY_LOG_MAX) == NULL,
          "log: FRUITY_LOG_MAX entries kept, the total counts all");

    if (argc > 1) {
        int r = read_rows(argv[1]);
        if (r >= 0) check(r == 1, "fruity-rows.txt names the same VAs and functions as the header (3 rows)");
    }
    printf("%d failure(s)\n", failures);
    return failures;
}
