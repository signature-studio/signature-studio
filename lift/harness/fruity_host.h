/* The fake host object for CreateFruityInstance (docs/investigations/create-fruity-instance-harness.md; boundary record:
 * docs/investigations/host-engine-boundary.md, section 6). Harness only: no file of the runtime or of the generated C is changed.
 *
 * What FL64.exe passes to CreateFruityInstance (read from FL64.exe, 4c839f24...0c65, 0x140007520-0x140007557 and 0x1400075e5-0x140007649;
 * every number below is a READING of those instructions, nothing here was run):
 *   - a 0x38-byte object from operator new:  +0x00 vtable, +0x08 u32 HostVersion = 1, +0x0c u32 Flags = 0, +0x10 u64 = 0,
 *     +0x18 char* "FL Studio", +0x20 char* "", +0x28 char* "" (both point at the same empty string), +0x30 char* base directory;
 *   - a vtable of 3 slots:  slot 0 Dispatch(int id, int index, int value), which sign-extends edx, r8d, r9d to 64 bits and jumps
 *     through slot 2 of the object's own vtable; slot 1 returns at once (`ret 0`, rax untouched); slot 2 Dispatch(intptr id,
 *     intptr index, intptr value) which returns 1 when id == 3 and index <= 1 (UNSIGNED 64-bit compare: `cmp r8, 1; ja`), else 0;
 *     value is not read;
 *   - the base directory is the directory part of the engine path, WITH its trailing separator (std::wstring find_last_of(L"\\/") + 1
 *     characters, "" when there is none), converted to UTF-8 and copied by strcpy_s into a 0x8000-byte global buffer.
 * Values that are not FL64.exe's but this environment's (PM-visible in the investigation): the engine path is FLE_VIRTUAL_DLL_PATH
 * (fle_env.h, decision 0018), so the base directory is its directory; the VAs of the three stubs (FRUITY_STUB_VA below) are a choice.
 *
 * The three slots are C functions with the convention of the runtime answers (fle_imports.h): arguments in R[1] (this), R[2], R[8], R[9],
 * the result in R[0], the guest return address stays on the stack. The guest reaches them by `call [vtable+8*n]`, i.e. fle_dispatch(<the
 * VA in the slot>), so each VA must be a row of fle_dispatch_table. The generator is not changed: the rows are added to the MERGED table
 * by lift/harness/two/merge_tables_n.py --extra-rows lift/harness/fruity-rows.txt (a build with FLE_CALL_FRUITY=1 only).
 *
 * Single-header: the definitions of the stubs and of their log are compiled where FRUITY_HOST_IMPL is defined (lift/harness/fruity.c and
 * lift/harness/seeds/fruity_selftest.c define it); everywhere else this header gives the layout and the pure helpers. */
#ifndef FLE_HARNESS_FRUITY_HOST_H
#define FLE_HARNESS_FRUITY_HOST_H

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "fle_dispatch.h"
#include "fle_runtime.h"

/* The arguments FL64.exe passes in rcx and rdx (0x140006f2d, 0x140006f32). CreateFruityInstance never reads them. */
#define FRUITY_CALL_RCX UINT64_C(0xC0)
#define FRUITY_CALL_RDX UINT64_C(0xEDE95A)

/* The object (offsets as FL64.exe fills them). */
#define FRUITY_HOST_SIZE 0x38u
#define FRUITY_OFF_VTABLE 0x00u
#define FRUITY_OFF_VERSION 0x08u
#define FRUITY_OFF_FLAGS 0x0Cu
#define FRUITY_OFF_QWORD10 0x10u
#define FRUITY_OFF_NAME 0x18u
#define FRUITY_OFF_STR2 0x20u
#define FRUITY_OFF_STR3 0x28u
#define FRUITY_OFF_BASEDIR 0x30u
#define FRUITY_HOST_VERSION 1u
#define FRUITY_HOST_FLAGS 0u
#define FRUITY_PRODUCT_NAME "FL Studio"
#define FRUITY_VTABLE_SLOTS 3u
#define FRUITY_BASEDIR_CAP 0x8000u

/* Where everything sits in the one host block that holds it (the guest sees host addresses, as for every HeapAlloc block). */
#define FRUITY_BLOCK_VTABLE 0x40u
#define FRUITY_BLOCK_NAME 0x60u
#define FRUITY_BLOCK_EMPTY 0x70u
#define FRUITY_BLOCK_BASEDIR 0x80u
#define FRUITY_BLOCK_SIZE (FRUITY_BLOCK_BASEDIR + FRUITY_BASEDIR_CAP)

/* The pseudo-VAs of the three stubs: a region that no other runtime table uses (the response table is 0x7FF900000000 + 0x10 * (k + 1), the
 * handle regions are 0x7FF7... to 0x7FFA8...). A CHOICE of this harness, not a value from a source. lift/harness/fruity-rows.txt names the
 * same three VAs. */
#define FRUITY_STUB_VA_BASE UINT64_C(0x7FFB00000000)
#define FRUITY_STUB_VA(k) (FRUITY_STUB_VA_BASE + UINT64_C(0x10) * ((uint64_t)(k) + 1))

#define FRUITY_LOG_MAX 128u

typedef struct {
    uint64_t obj;      /* guest address of the 0x38-byte object */
    uint64_t vtable;   /* guest address of the 3-slot vtable */
    uint64_t name;     /* "FL Studio" */
    uint64_t empty;    /* "" : +0x20 and +0x28 point here */
    uint64_t basedir;  /* the 0x8000-byte buffer */
    size_t basedir_len;
    uint8_t *mem;      /* the host block behind all of them (never freed by the harness) */
} fruity_host;

/* One call of slot 1 or slot 2 that the engine made. slot 0 only forwards, so it is not logged: its work shows as a slot-2 record with
 * via_slot0 = 1 (id, index and value then hold the sign-extended 32-bit arguments). For slot 1 the arguments are edx and r8d as int32 and
 * value is 0. */
typedef struct {
    uint8_t slot;
    uint8_t via_slot0;
    int64_t id, index, value;
    uint64_t result;
} fruity_call;

/* What FL64.exe's slot 2 returns (0x140007540): 1 when id == 3 and the 64-bit index is 0 or 1 (unsigned), else 0. */
static inline uint64_t fruity_answer(uint64_t id, uint64_t index) {
    return (id == 3 && index <= 1) ? 1 : 0;
}

/* The length of the directory part of `path`, trailing separator included: std::wstring::find_last_of(L"\\/") + 1, 0 when there is none
 * (FL64.exe 0x140001330). */
static inline size_t fruity_dir_len(const char *path) {
    size_t n = strlen(path);
    while (n > 0) {
        if (path[n - 1] == '\\' || path[n - 1] == '/') return n;
        n--;
    }
    return 0;
}

static inline void fruity_put32(uint8_t *p, uint32_t v) { memcpy(p, &v, sizeof v); }
static inline void fruity_put64(uint8_t *p, uint64_t v) { memcpy(p, &v, sizeof v); }

/* Builds the object, the vtable and the strings in one calloc block. Returns 0, or -1 when the directory does not fit the 0x8000-byte
 * buffer or the block cannot be allocated. */
static inline int fruity_host_make(fruity_host *h, const char *engine_path) {
    size_t dl = fruity_dir_len(engine_path);
    uint8_t *m;
    unsigned k;
    if (dl + 1 > FRUITY_BASEDIR_CAP) return -1;
    m = (uint8_t *)calloc(1, FRUITY_BLOCK_SIZE);
    if (!m) return -1;
    h->mem = m;
    h->obj = (uint64_t)(uintptr_t)m;
    h->vtable = h->obj + FRUITY_BLOCK_VTABLE;
    h->name = h->obj + FRUITY_BLOCK_NAME;
    h->empty = h->obj + FRUITY_BLOCK_EMPTY;
    h->basedir = h->obj + FRUITY_BLOCK_BASEDIR;
    h->basedir_len = dl;
    memcpy(m + FRUITY_BLOCK_NAME, FRUITY_PRODUCT_NAME, sizeof FRUITY_PRODUCT_NAME);
    memcpy(m + FRUITY_BLOCK_BASEDIR, engine_path, dl);
    for (k = 0; k < FRUITY_VTABLE_SLOTS; k++) fruity_put64(m + FRUITY_BLOCK_VTABLE + 8u * k, FRUITY_STUB_VA(k));
    fruity_put64(m + FRUITY_OFF_VTABLE, h->vtable);
    fruity_put32(m + FRUITY_OFF_VERSION, FRUITY_HOST_VERSION);
    fruity_put32(m + FRUITY_OFF_FLAGS, FRUITY_HOST_FLAGS);
    fruity_put64(m + FRUITY_OFF_QWORD10, 0);
    fruity_put64(m + FRUITY_OFF_NAME, h->name);
    fruity_put64(m + FRUITY_OFF_STR2, h->empty);
    fruity_put64(m + FRUITY_OFF_STR3, h->empty);
    fruity_put64(m + FRUITY_OFF_BASEDIR, h->basedir);
    return 0;
}

/* The stubs. They are rows of the merged dispatch table (fruity-rows.txt), so they are not static. */
void fruity_host_slot0(void);
void fruity_host_slot1(void);
void fruity_host_slot2(void);
/* Counters and the log (the first FRUITY_LOG_MAX calls of slots 1 and 2). */
uint64_t fruity_host_calls(unsigned slot);
uint64_t fruity_host_log_total(void);
const fruity_call *fruity_host_log_at(uint64_t i);
void fruity_host_log_reset(void);

#ifdef FRUITY_HOST_IMPL

static fruity_call fruity_log_buf[FRUITY_LOG_MAX];
static uint64_t fruity_log_n;
static uint64_t fruity_slot_calls[3];
static int fruity_via_slot0;

static void fruity_log_add(unsigned slot, int via0, int64_t id, int64_t index, int64_t value, uint64_t result) {
    if (fruity_log_n < FRUITY_LOG_MAX) {
        fruity_call *c = &fruity_log_buf[fruity_log_n];
        c->slot = (uint8_t)slot;
        c->via_slot0 = (uint8_t)via0;
        c->id = id;
        c->index = index;
        c->value = value;
        c->result = result;
    }
    fruity_log_n++;
}

/* Slot 0, FL64.exe 0x140007520: mov rax,[rcx]; movsxd r9,r9d; movsxd r8,r8d; movsxd rdx,edx; jmp [rax+0x10]. The jump goes through the
 * vtable of the object the engine passed as `this`, as in FL64.exe, and does not touch the stack. */
void fruity_host_slot0(void) {
    uint64_t vt;
    fruity_slot_calls[0]++;
    R[2] = (uint64_t)(int64_t)(int32_t)R[2];
    R[8] = (uint64_t)(int64_t)(int32_t)R[8];
    R[9] = (uint64_t)(int64_t)(int32_t)R[9];
    vt = fle_ld(R[1], 8);
    fruity_via_slot0 = 1;
    fle_dispatch(fle_ld(vt + 8u * 2u, 8));
    fruity_via_slot0 = 0;
}

/* Slot 1, FL64.exe 0x140007530: ret 0. Nothing is changed, not even rax. */
void fruity_host_slot1(void) {
    fruity_slot_calls[1]++;
    fruity_log_add(1, 0, (int64_t)(int32_t)R[2], (int64_t)(int32_t)R[8], 0, 0);
}

/* Slot 2, FL64.exe 0x140007540. */
void fruity_host_slot2(void) {
    uint64_t id = R[2], index = R[8], value = R[9];
    uint64_t res = fruity_answer(id, index);
    fruity_slot_calls[2]++;
    fruity_log_add(2, fruity_via_slot0, (int64_t)id, (int64_t)index, (int64_t)value, res);
    R[0] = res;
}

uint64_t fruity_host_calls(unsigned slot) { return slot < 3 ? fruity_slot_calls[slot] : 0; }
uint64_t fruity_host_log_total(void) { return fruity_log_n; }
const fruity_call *fruity_host_log_at(uint64_t i) { return (i < fruity_log_n && i < FRUITY_LOG_MAX) ? &fruity_log_buf[i] : NULL; }
void fruity_host_log_reset(void) {
    memset(fruity_log_buf, 0, sizeof fruity_log_buf);
    memset(fruity_slot_calls, 0, sizeof fruity_slot_calls);
    fruity_log_n = 0;
    fruity_via_slot0 = 0;
}

#endif /* FRUITY_HOST_IMPL */

#endif /* FLE_HARNESS_FRUITY_HOST_H */
