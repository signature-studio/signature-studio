/* (wasm, FLE_VMEM_ARENA, decision 0045: an object is a range of the arena of fle_sys.c instead of a memfd, and a
 * shared view is that range itself, so views of one object stay coherent; a copy-on-write view is a copy. The page
 * protections of views are recorded and not enforced. Everything below that mentions memfd or mmap is the native build.)
 * Memory protection and file mapping (docs/decisions/0024-apis-a3-a5-a13.md; the pages read are listed in
 * docs/investigations/apis-a3-a5-a13-run.md). Single guest thread. A case the Microsoft pages leave open, and every
 * case this runtime does not implement, stops the run through fle_stop (fle_env.h); where a page states that the
 * function fails without naming the error code, the code set here is a runtime choice and is marked "assumption".
 * A file mapping object is backed by an anonymous memfd (process memory, not a host file); a view is a MAP_SHARED
 * (or, for copy-on-write, MAP_PRIVATE) mapping of it. The host file system is never touched. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_vfs.h"
#include "fle_vmap.h"

static long fle_vmap_owner_tid;

static void fle_vmap_check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (fle_vmap_owner_tid == 0) fle_vmap_owner_tid = tid;
    if (tid != fle_vmap_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

static void fail(uint32_t err, uint64_t ret)
{
    fle_set_last_error(err);
    R[0] = ret;
}

/* Constants. Protection values are the ones of "Memory Protection Constants"; MEM_* and the MEMORY_BASIC_INFORMATION
 * members are on the VirtualQuery and MEMORY_BASIC_INFORMATION pages. The FILE_MAP_* numbers are not on the Learn pages
 * read (FILE_MAP_ALL_ACCESS is given there only by name): they come from windows-sys (docs.rs) and the mingw-w64 headers,
 * both unofficial. */
#define P_NOACCESS          0x01u
#define P_READONLY          0x02u
#define P_READWRITE         0x04u
#define P_WRITECOPY         0x08u
#define P_EXECUTE           0x10u
#define P_EXECUTE_READ      0x20u
#define P_EXECUTE_READWRITE 0x40u
#define SEC_COMMIT_         0x08000000u

#define MEM_COMMIT_  0x1000u
#define MEM_RESERVE_ 0x2000u
#define MEM_PRIVATE_ 0x20000u
#define MEM_MAPPED_  0x40000u

#define FMAP_COPY       0x00000001u
#define FMAP_WRITE      0x00000002u
#define FMAP_READ       0x00000004u
#define FMAP_ALL_ACCESS 0x000F001Fu

#define FLE_ERR_INVALID_PARAMETER_A 87u  /* assumption */
#define FLE_ERR_INVALID_ADDRESS_A   487u /* assumption (the code VirtualAlloc names for an address that does not fit) */
#define FLE_ERR_FILE_INVALID        1006u /* named on the CreateFileMapping page and in System Error Codes 1000-1299 */

static uint64_t down(uint64_t v, uint64_t a) { return v & ~(a - 1); }
static uint64_t up(uint64_t v, uint64_t a) { return (v + a - 1) & ~(a - 1); }

static int is_private_protection(uint32_t p)
{
    return p == P_NOACCESS || p == P_READONLY || p == P_READWRITE || p == P_EXECUTE || p == P_EXECUTE_READ ||
           p == P_EXECUTE_READWRITE;
}

/* ---- VirtualQuery ----
 * MSDN (VirtualQuery, MEMORY_BASIC_INFORMATION). Only memory this runtime tracks can be described: the VirtualAlloc
 * regions (fle_sys.c) and the views made here. Any other address (the lifted image, the host stack and heap, free
 * address space) was not described because the runtime did not track it and stopped the run; since decision 0123 a loaded image, a registered range and free memory answer (below). An address above the
 * highest address accessible to the process fails with ERROR_INVALID_PARAMETER (page). A buffer smaller than the
 * structure is not defined by the page and stops. The returned size is the structure size (page: the number of bytes
 * returned in the buffer). PartitionId is 0 (the page gives no meaning to it; assumption). */
typedef struct {
    uint64_t base;
    uint64_t len;       /* a multiple of FLE_PAGE_SIZE */
    uint32_t prot;      /* PAGE_READONLY, PAGE_READWRITE or PAGE_WRITECOPY */
    int live;
    uint64_t obj;       /* index into objs */
    int own;            /* arena: the view is a private copy that holds its own range (FILE_MAP_COPY) */
} vm_view;

typedef struct {
    int fd;             /* -1 once the last handle is closed */
    uint64_t buf;       /* arena: the object's range (0 once released); there is no fd */
    uint64_t size;      /* the object's maximum size in bytes (never 0) */
    uint32_t prot;      /* PAGE_READONLY or PAGE_READWRITE */
    uint32_t handles;
    uint32_t views;
} vm_obj;

typedef struct {
    uint64_t obj;
    int live;
} vm_handle;

static vm_view *views;
static uint64_t views_n, views_cap, views_live;
static vm_obj *objs;
static uint64_t objs_n, objs_cap, objs_live;
static vm_handle *hands;
static uint64_t hands_n, hands_cap, hands_live;
static int hook_registered;

uint64_t fle_vmap_handle_count(void) { return hands_live; }
uint64_t fle_vmap_view_count(void) { return views_live; }
uint64_t fle_vmap_object_count(void) { return objs_live; }

static vm_view *view_containing(uint64_t a)
{
    uint64_t i;

    for (i = 0; i < views_n; i++)
        if (views[i].live && a >= views[i].base && a - views[i].base < views[i].len) return &views[i];
    return NULL;
}

#ifdef FLE_VMEM_ARENA
static int view_at_base(uint64_t a)
{
    uint64_t i;

    for (i = 0; i < views_n; i++)
        if (views[i].live && views[i].base == a) return 1;
    return 0;
}
#endif

static void put_u32(uint8_t *b, size_t o, uint32_t v) { memcpy(b + o, &v, 4); }
static void put_u64(uint8_t *b, size_t o, uint64_t v) { memcpy(b + o, &v, 8); }

/* MEMORY_BASIC_INFORMATION on x64 (offsets follow from the member types on the page and natural alignment):
 * BaseAddress 0, AllocationBase 8, AllocationProtect 16, PartitionId 20 (WORD), RegionSize 24, State 32, Protect 36,
 * Type 40, size 48. */
static void write_mbi(uint64_t out, uint64_t base, uint64_t alloc_base, uint32_t alloc_prot, uint64_t region, uint32_t state,
                      uint32_t prot, uint32_t type)
{
    uint8_t b[FLE_MBI_SIZE];

    memset(b, 0, sizeof b);
    put_u64(b, 0, base);
    put_u64(b, 8, alloc_base);
    put_u32(b, 16, alloc_prot);
    put_u64(b, 24, region);
    put_u32(b, 32, state);
    put_u32(b, 36, prot);
    put_u32(b, 40, type);
    memcpy((void *)fle_addr(out, (unsigned)sizeof b, 1), b, sizeof b);
}

/* ---- Decision 0123 (PROVISIONAL), wall 25: images and other registered ranges ---- */
#define VQ_MAX_IMAGES 16u
#define VQ_MAX_RANGES 16u
#define VQ_MAX_SECTIONS 24u
#define MEM_FREE_  0x10000u
#define MEM_IMAGE_ 0x1000000u
#define SCN_EXEC   0x20000000u
#define SCN_READ   0x40000000u
#define SCN_WRITE  0x80000000u

typedef struct {
    uint64_t va, size;         /* a run of pages of the image: [base + va, base + va + size), size a multiple of the page size */
    uint32_t protect;
} vq_run;

typedef struct {
    uint64_t base, size;
    unsigned nruns;
    vq_run runs[VQ_MAX_SECTIONS + 1];
} vq_image;

typedef struct {
    uint64_t base, size;
    uint32_t protect, type;
} vq_range;

static vq_image vq_images[VQ_MAX_IMAGES];
static unsigned vq_nimages;
static vq_range vq_ranges[VQ_MAX_RANGES];
static unsigned vq_nranges;

static uint32_t prot_of_characteristics(uint32_t ch)
{
    int x = (ch & SCN_EXEC) != 0, r = (ch & SCN_READ) != 0, w = (ch & SCN_WRITE) != 0;

    if (x) return w ? P_EXECUTE_READWRITE : (r ? P_EXECUTE_READ : P_EXECUTE);
    if (w) return P_READWRITE;
    if (r) return P_READONLY;
    return P_NOACCESS;
}

/* Reads the PE headers that the harness mapped at `base` (the image is in memory when this is called) and records the runs of pages by protection. */
void fle_vmap_register_image(uint64_t base)
{
    static const char api[] = "VirtualQuery";
    vq_image *im;
    uint32_t lfanew, soi, soh;
    uint16_t nsec, optsz, k;
    uint64_t sec;
    unsigned i;

    if (vq_nimages >= VQ_MAX_IMAGES) fle_stop(api, "more images than the runtime holds");
    if ((uint16_t)fle_ld(base, 2) != 0x5A4Du) fle_stop(api, "an image registered for VirtualQuery has no MZ signature at its base");
    lfanew = (uint32_t)fle_ld(base + 0x3C, 4);
    if ((uint32_t)fle_ld(base + lfanew, 4) != 0x00004550u) fle_stop(api, "an image registered for VirtualQuery has no PE signature");
    nsec = (uint16_t)fle_ld(base + lfanew + 6, 2);
    optsz = (uint16_t)fle_ld(base + lfanew + 20, 2);
    soi = (uint32_t)fle_ld(base + lfanew + 24 + 56, 4);
    soh = (uint32_t)fle_ld(base + lfanew + 24 + 60, 4);
    if (nsec > VQ_MAX_SECTIONS) fle_stop(api, "an image with more sections than the runtime holds");
    sec = base + lfanew + 24u + optsz;
    im = &vq_images[vq_nimages];
    memset(im, 0, sizeof *im);
    im->base = base;
    im->size = up(soi, FLE_PAGE_SIZE);
    /* the headers: PAGE_READONLY (Learn: the headers of an image are mapped read-only; the page does not say: a choice, S-vq-headers) */
    im->runs[0].va = 0;
    im->runs[0].size = up(soh, FLE_PAGE_SIZE);
    im->runs[0].protect = P_READONLY;
    im->nruns = 1;
    for (k = 0; k < nsec; k++) {
        uint32_t vsize = (uint32_t)fle_ld(sec + 40u * k + 8, 4), va = (uint32_t)fle_ld(sec + 40u * k + 12, 4), ch = (uint32_t)fle_ld(sec + 40u * k + 36, 4);
        uint64_t sz = up(vsize ? vsize : 1u, FLE_PAGE_SIZE);

        if ((va % FLE_PAGE_SIZE) != 0) fle_stop(api, "a section whose virtual address is not a multiple of the page size");
        im->runs[im->nruns].va = va;
        im->runs[im->nruns].size = sz;
        im->runs[im->nruns].protect = prot_of_characteristics(ch);
        im->nruns++;
    }
    /* sort by va (the section table is in order for every linker output seen; this makes it so) and merge neighbours of one protection */
    for (i = 1; i < im->nruns; i++) {
        unsigned j = i;

        while (j > 0 && im->runs[j - 1].va > im->runs[j].va) {
            vq_run t = im->runs[j - 1];

            im->runs[j - 1] = im->runs[j];
            im->runs[j] = t;
            j--;
        }
    }
    vq_nimages++;
}

void fle_vmap_register_range(uint64_t base, uint64_t size, uint32_t protect, uint32_t type)
{
    if (vq_nranges >= VQ_MAX_RANGES) fle_stop("VirtualQuery", "more registered ranges than the runtime holds");
    vq_ranges[vq_nranges].base = down(base, FLE_PAGE_SIZE);
    vq_ranges[vq_nranges].size = up(base + size, FLE_PAGE_SIZE) - down(base, FLE_PAGE_SIZE);
    vq_ranges[vq_nranges].protect = protect;
    vq_ranges[vq_nranges].type = type;
    vq_nranges++;
}

/* The lowest address above `a` where something tracked begins (a VirtualAlloc region, a view, an image, a registered range), or FLE_MAX_APP_ADDR + 1. Used for the size of a free region. */
static uint64_t next_tracked_above(uint64_t a)
{
    uint64_t best = FLE_MAX_APP_ADDR + 1u, i;

    for (i = 0; i < vq_nimages; i++)
        if (vq_images[i].base > a && vq_images[i].base < best) best = vq_images[i].base;
    for (i = 0; i < vq_nranges; i++)
        if (vq_ranges[i].base > a && vq_ranges[i].base < best) best = vq_ranges[i].base;
    for (i = 0; i < views_n; i++)
        if (views[i].live && views[i].base > a && views[i].base < best) best = views[i].base;
    {
        uint64_t p = up(a + 1u, FLE_PAGE_SIZE);
        fle_sys_region_view r;
        uint64_t lim = best;

        /* VirtualAlloc regions: probe by the nearest region above (fle_sys_region_at answers only for an address inside one): scan the pages up to the nearest other boundary in 64 KiB steps */
        for (; p < lim; p += 0x10000u) {
            if (fle_sys_region_at(p, &r)) {
                if (r.base < best && r.base > a) best = r.base;
                else if (r.base <= a) { /* a is inside: not free */ }
                break;
            }
        }
    }
    return best;
}

static int vq_image_query(uint64_t pg, uint64_t out)
{
    unsigned i, k;

    for (i = 0; i < vq_nimages; i++) {
        vq_image *im = &vq_images[i];

        if (pg < im->base || pg - im->base >= im->size) continue;
        for (k = 0; k < im->nruns; k++) {
            uint64_t lo = im->base + im->runs[k].va, hi = lo + im->runs[k].size;

            if (pg >= lo && pg < hi) {
                uint64_t end = hi, kk = k + 1u;

                /* the run continues over the following runs of the same protection that touch it */
                while (kk < im->nruns && im->base + im->runs[kk].va == end && im->runs[kk].protect == im->runs[k].protect) {
                    end += im->runs[kk].size;
                    kk++;
                }
                if (end > im->base + im->size) end = im->base + im->size;
                write_mbi(out, pg, im->base, im->runs[0].protect, end - pg, MEM_COMMIT_, im->runs[k].protect, MEM_IMAGE_);
                return 1;
            }
        }
        /* a page of the image that no run covers (a gap between sections): the page is part of the allocation: committed, no access (a choice: S-vq-gap) */
        {
            uint64_t end = im->base + im->size, nxt = end;

            for (k = 0; k < im->nruns; k++)
                if (im->base + im->runs[k].va > pg && im->base + im->runs[k].va < nxt) nxt = im->base + im->runs[k].va;
            write_mbi(out, pg, im->base, im->runs[0].protect, nxt - pg, MEM_COMMIT_, P_NOACCESS, MEM_IMAGE_);
            return 1;
        }
    }
    return 0;
}

static int vq_range_query(uint64_t pg, uint64_t out)
{
    unsigned i;

    for (i = 0; i < vq_nranges; i++) {
        vq_range *r = &vq_ranges[i];

        if (pg >= r->base && pg - r->base < r->size) {
            write_mbi(out, pg, r->base, r->protect, r->base + r->size - pg, MEM_COMMIT_, r->protect, r->type);
            return 1;
        }
    }
    return 0;
}

void fle_imp_VirtualQuery(void)
{
    static const char api[] = "VirtualQuery";
    uint64_t addr = R[1], out = R[2], len = R[8];
    uint64_t pg;
    fle_sys_region_view r;
    vm_view *v;

    fle_vmap_check_thread(api);
    if (addr > FLE_MAX_APP_ADDR) {
        fail(FLE_ERR_INVALID_PARAMETER_A /* named on the page */, 0);
        return;
    }
    if (len < FLE_MBI_SIZE) fle_stop(api, "dwLength is smaller than MEMORY_BASIC_INFORMATION (the page gives no behavior)");
    if (out == 0) fle_stop(api, "lpBuffer is NULL");
    pg = down(addr, FLE_PAGE_SIZE);
    if (fle_sys_region_at(pg, &r)) {
        uint64_t i = (pg - r.base) / FLE_PAGE_SIZE, n = r.size / FLE_PAGE_SIZE, j = i;
        uint8_t st = r.pages[i];

        while (j < n && r.pages[j] == st) j++;
        write_mbi(out, pg, r.base, r.alloc_protect, (j - i) * FLE_PAGE_SIZE, st ? MEM_COMMIT_ : MEM_RESERVE_, st, MEM_PRIVATE_);
        R[0] = FLE_MBI_SIZE;
        return;
    }
    v = view_containing(pg);
    if (v) {
        write_mbi(out, pg, v->base, v->prot, v->base + v->len - pg, MEM_COMMIT_, v->prot, MEM_MAPPED_);
        R[0] = FLE_MBI_SIZE;
        return;
    }
    /* Decision 0123 (PROVISIONAL), wall 25: a loaded image (main module and real modules) answers from its section table, a registered range (the first thread's stack) from its protection, and any other address is FREE
     * memory as far as this runtime knows (the page: MEM_FREE; AllocationBase, AllocationProtect, Protect and Type are undefined and are 0 here). */
    if (vq_image_query(pg, out) || vq_range_query(pg, out)) {
        R[0] = FLE_MBI_SIZE;
        return;
    }
    write_mbi(out, pg, 0, 0, next_tracked_above(pg) - pg, MEM_FREE_, 0, 0);
    R[0] = FLE_MBI_SIZE;
}

/* ---- Inline hook points (decision 0105) ----
 * A module whose generated unit has hook points registers its image (base, SizeOfImage) and a table of the function entries whose first
 * 5 bytes the guest may overwrite with `jmp rel32` (fe-lift --hook-points). Single guest thread, as everything in this file. */
#define IH_SIZE      5u  /* jmp rel32 */
#define IH_MAX_MODS  8u
#define IH_MAX_PAGES 64u

typedef struct {
    uint64_t base;
    uint64_t size;
    const fle_hook_point *pts;
    uint64_t n;
} ih_mod;

typedef struct {
    uint64_t page;
    uint32_t prot;
} ih_page;

static ih_mod ih_mods[IH_MAX_MODS];
static unsigned ih_nmods;
/* The protection that VirtualProtect last set on a page of a registered image (not enforced; a page not listed has the protection of its
 * section, fle_hook_point.prot). Protection belongs to whole pages (Memory Protection Constants), so two hook points of one page share it. */
static ih_page ih_pages[IH_MAX_PAGES];
static unsigned ih_npages;

void fle_hook_register(uint64_t base, uint64_t size, const fle_hook_point *pts, uint64_t n)
{
    if (ih_nmods >= IH_MAX_MODS) fle_stop("inline hook", "more modules with hook points than the runtime holds");
    ih_mods[ih_nmods].base = base;
    ih_mods[ih_nmods].size = size;
    ih_mods[ih_nmods].pts = pts;
    ih_mods[ih_nmods].n = n;
    ih_nmods++;
}

static void ih_hex5(char *out, uint64_t v)
{
    unsigned k;

    for (k = 0; k < IH_SIZE; k++) snprintf(out + 3u * k, 4, k + 1u < IH_SIZE ? "%02x " : "%02x", (unsigned)((v >> (8u * k)) & 0xffu));
}

uint64_t fle_hook_target(uint64_t va, uint64_t orig5)
{
    uint64_t cur = fle_ld(va, IH_SIZE), t;
    char have[16], want[16], why[200];

    if (cur == orig5) return 0;
    ih_hex5(have, cur);
    ih_hex5(want, orig5);
    if ((cur & 0xffu) != 0xe9u) {
        snprintf(why, sizeof why, "hook point %#llx: the first 5 bytes are now %s; the original %s and 'e9 rel32' are the only ones modelled (decision 0105)",
                 (unsigned long long)va, have, want);
        fle_stop("inline hook", why);
    }
    t = va + IH_SIZE + (uint64_t)(int64_t)(int32_t)(uint32_t)(cur >> 8);
    if (t == 0) {
        snprintf(why, sizeof why, "hook point %#llx: the jmp rel32 that the guest wrote (%s) goes to address 0", (unsigned long long)va, have);
        fle_stop("inline hook", why);
    }
    return t;
}

void fle_site_store32(uint64_t va, uint64_t addr, uint32_t expect, uint32_t nw)
{
    uint32_t cur = (uint32_t)fle_ld(addr, 4);
    char why[200];

    if (cur == nw) return;
    if (cur != expect) {
        snprintf(why, sizeof why, "site %#llx: the word at %#llx is %#x; the override expects %#x (the guest's value) or %#x (already overridden) (decision 0120)",
                 (unsigned long long)va, (unsigned long long)addr, (unsigned)cur, (unsigned)expect, (unsigned)nw);
        fle_stop("site override", why);
    }
    fle_st(addr, 4, nw);
}

static const ih_mod *ih_image_at(uint64_t a)
{
    unsigned i;

    for (i = 0; i < ih_nmods; i++)
        if (a >= ih_mods[i].base && a - ih_mods[i].base < ih_mods[i].size) return &ih_mods[i];
    return NULL;
}

static const fle_hook_point *ih_point_at(const ih_mod *m, uint64_t a)
{
    uint64_t i;

    for (i = 0; i < m->n; i++)
        if (m->pts[i].va == a) return &m->pts[i];
    return NULL;
}

/* VirtualProtect on an address of a registered image. The runtime does not track the memory of an image, and the lifted code does not
 * read its own instructions, so the only change of protection that is answered is the one that prepares an annotated hook: exactly a hook
 * point, at most 5 bytes. Everything else in the image stops by name. The old protection is the one of the first page; the new one is
 * recorded for every page of the range and enforced nowhere (decision 0045: the wasm port does not enforce page protection either). */
static void ih_protect(const char *api, const ih_mod *m, uint64_t addr, uint64_t size, uint32_t np, uint64_t pold)
{
    const fle_hook_point *p = ih_point_at(m, addr);
    uint64_t from = down(addr, FLE_PAGE_SIZE), to = up(addr + size, FLE_PAGE_SIZE), pg;
    uint32_t old = 0;
    unsigned i;
    char why[260];

    if (p == NULL) {
        snprintf(why, sizeof why,
                 "inline hook at an address that is not an annotated hook point (decision 0105): address %#llx, size %llu, new protection %#x, image base %#llx",
                 (unsigned long long)addr, (unsigned long long)size, (unsigned)np, (unsigned long long)m->base);
        fle_stop(api, why);
    }
    if (size > IH_SIZE) {
        snprintf(why, sizeof why,
                 "range of %llu bytes at the hook point %#llx; a hook is the 5 bytes of jmp rel32 (decision 0105), new protection %#x",
                 (unsigned long long)size, (unsigned long long)addr, (unsigned)np);
        fle_stop(api, why);
    }
    if (pold == 0) {
        fail(FLE_ERR_INVALID_PARAMETER_A, 0);
        return;
    }
    old = p->prot;
    for (i = 0; i < ih_npages; i++)
        if (ih_pages[i].page == from) old = ih_pages[i].prot;
    fle_st(pold, 4, old);
    for (pg = from; pg < to; pg += FLE_PAGE_SIZE) {
        for (i = 0; i < ih_npages; i++)
            if (ih_pages[i].page == pg) break;
        if (i == ih_npages) {
            if (ih_npages >= IH_MAX_PAGES) fle_stop(api, "more pages of inline hook points than the runtime records");
            ih_pages[ih_npages++].page = pg;
        }
        ih_pages[i].prot = np;
    }
    R[0] = 1;
}

/* ---- VirtualProtect ----
 * MSDN (VirtualProtect). Only committed pages of one VirtualAlloc region are changed. lpflOldProtect NULL fails (page).
 * "If the state of any page in the specified region is not committed, the function fails and returns without
 * modifying the access protection of any pages", and the pages "cannot span adjacent reserved regions": both fail
 * with 0 (the page names no code; ERROR_INVALID_ADDRESS 487 is an assumption). The previous protection of the first
 * page is stored. flNewProtect is one of the six values VirtualAlloc accepts (fle_sys.c); the other constants (guard,
 * no-cache, write-combine, write-copy, CFG flags, enclave) are not modelled and stop the run, as does dwSize 0 (the
 * page does not define it). A view made by MapViewOfFile, and any memory the runtime does not track (the lifted
 * image, the host heap and stack), is not implemented and stops the run; the one exception is the 5-byte hook points
 * of a registered image (ih_protect above, decision 0105). */
void fle_imp_VirtualProtect(void)
{
    static const char api[] = "VirtualProtect";
    uint64_t addr = R[1], size = R[2], pold = R[9];
    uint32_t np = (uint32_t)R[8];
    uint64_t from, to, i;
    fle_sys_region_view r;
    const ih_mod *m;

    fle_vmap_check_thread(api);
    if (!is_private_protection(np)) fle_stop(api, "flNewProtect is not one of the six protections modelled");
    if (size == 0) fle_stop(api, "dwSize 0 is not defined by the page");
    if (addr + size < addr || addr > FLE_MAX_APP_ADDR) fle_stop(api, "range overflows the address space (not defined by the page)");
    if ((m = ih_image_at(addr)) != NULL) {
        ih_protect(api, m, addr, size, np, pold);
        return;
    }
    if (pold == 0) {
        fail(FLE_ERR_INVALID_PARAMETER_A, 0);
        return;
    }
    from = down(addr, FLE_PAGE_SIZE);
    to = up(addr + size, FLE_PAGE_SIZE);
    if (!fle_sys_region_at(from, &r)) {
        if (view_containing(from)) fle_stop(api, "changing the protection of a file view is not implemented");
        fle_stop(api, "address is not in a VirtualAlloc region (the runtime does not track other memory)");
    }
    if (to > r.base + r.size) {
        fail(FLE_ERR_INVALID_ADDRESS_A, 0);
        return;
    }
    for (i = (from - r.base) / FLE_PAGE_SIZE; i < (to - r.base) / FLE_PAGE_SIZE; i++) {
        if (r.pages[i] == 0) {
            fail(FLE_ERR_INVALID_ADDRESS_A, 0);
            return;
        }
    }
    fle_st(pold, 4, r.pages[(from - r.base) / FLE_PAGE_SIZE]);
    fle_sys_set_protection(from, to, np);
    R[0] = 1;
}

/* ---- File mapping objects and views ---- */

static int in_map_region(uint64_t h)
{
    return h > FLE_VMAP_HANDLE_BASE && h < FLE_VMAP_HANDLE_BASE + FLE_VMAP_REGION_SIZE;
}

/* The slot index of an issued handle value, or -1 when the value was never issued. */
static int64_t hand_index(uint64_t h)
{
    uint64_t off, i;

    if (!in_map_region(h)) return -1;
    off = h - FLE_VMAP_HANDLE_BASE;
    if (off % 4u) return -1;
    i = off / 4u - 1u;
    return i < hands_n ? (int64_t)i : -1;
}

int fle_vmap_is_handle(uint64_t h) { return hand_index(h) >= 0; }

static void obj_release_if_unused(uint64_t o)
{
    vm_obj *x = &objs[o];

#ifdef FLE_VMEM_ARENA
    /* views alias the range, so it lives until the last view is unmapped too */
    if (x->handles == 0 && x->views == 0 && x->buf) {
        fle_sys_host_release(x->buf);
        x->buf = 0;
    }
#else
    if (x->handles == 0 && x->fd >= 0) {
        close(x->fd);
        x->fd = -1;
    }
#endif
    if (x->handles == 0 && x->views == 0) objs_live--;
}

static uint64_t hand_new(uint64_t obj)
{
    if (hands_n == hands_cap) {
        uint64_t nc = hands_cap ? hands_cap * 2 : 16;
        vm_handle *n = realloc(hands, nc * sizeof *n);

        if (!n) fle_stop("CreateFileMappingW", "out of memory");
        hands = n;
        hands_cap = nc;
    }
    if (hands_n + 2 >= FLE_VMAP_REGION_SIZE / 4u) fle_stop("CreateFileMappingW", "file mapping handle region exhausted");
    hands[hands_n].obj = obj;
    hands[hands_n].live = 1;
    hands_n++;
    hands_live++;
    objs[obj].handles++;
    return FLE_VMAP_HANDLE_BASE + 4u * hands_n;
}

int fle_vmap_dup_handle(uint64_t h, uint64_t *out)
{
    int64_t i = hand_index(h);

    if (i < 0 || !hands[i].live) return 0;
    *out = hand_new(hands[i].obj);
    return 1;
}

int fle_vmap_close_handle(uint64_t h)
{
    int64_t i = hand_index(h);
    uint64_t o;

    if (i < 0 || !hands[i].live) return 0;
    o = hands[i].obj;
    hands[i].live = 0;
    hands_live--;
    objs[o].handles--;
    obj_release_if_unused(o);
    return 1;
}

/* CloseHandle for the handles issued here (called by fle_imp_CloseHandle through fle_vfs_add_close_hook). A mapped
 * view stays valid after the handle is closed (page: the object does not close until every view is unmapped). */
static int vmap_close_hook(uint64_t h)
{
    if (!in_map_region(h)) return 0;
    if (hand_index(h) < 0) fle_stop("CloseHandle", "handle value was not issued by this runtime");
    if (!fle_vmap_close_handle(h)) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return 1;
    }
    R[0] = 1;
    return 1;
}

static void hook_once(void)
{
    if (hook_registered) return;
    fle_vfs_add_close_hook(vmap_close_hook);
    hook_registered = 1;
}

/* kernel32!CreateFileMappingW: HANDLE f(hFile rcx, lpFileMappingAttributes rdx, flProtect r8d, dwMaximumSizeHigh r9d,
 * dwMaximumSizeLow [5], lpName [6]). MSDN (CreateFileMappingW):
 *  - implemented: flProtect PAGE_READONLY (0x2) or PAGE_READWRITE (0x4), alone or with SEC_COMMIT (the page says it is
 *    assumed when no attribute is given and has no effect on file-backed objects); unnamed (lpName NULL); default
 *    security (lpFileMappingAttributes NULL).
 *  - hFile INVALID_HANDLE_VALUE: an object "backed by the system paging file" of the given size, whose pages start as
 *    0 (page). The size must be given (page); size 0 is a failure the page does not give a code for and stops.
 *  - hFile a handle of the virtual file system: the object has the file's content as it is now (a snapshot; the page
 *    says a mapped file and ReadFile/WriteFile "are not necessarily coherent", and nothing is ever written back to the
 *    virtual file, so a PAGE_READWRITE object over a file stops the run). The file must be open for reading (page:
 *    PAGE_READONLY needs GENERIC_READ; a handle without it is not defined and stops). Maximum size 0 means the file's
 *    size; a file of length 0 fails with ERROR_FILE_INVALID (1006, page). A size above the file's length is not
 *    defined for a read-only object and stops. A size below the length is kept as the object's size.
 *  - NULL, a closed or a find handle: 0 with ERROR_INVALID_HANDLE (6; the page names no code; assumption).
 *  - not implemented, stop: named objects, security attributes, the other PAGE_* values, SEC_IMAGE, SEC_RESERVE,
 *    SEC_LARGE_PAGES and the other attributes.
 * A new object leaves the last error alone; the page's ERROR_ALREADY_EXISTS case needs a name. */
void fle_imp_CreateFileMappingW(void)
{
    static const char api[] = "CreateFileMappingW";
    uint64_t hfile = R[1], attrs = R[2], name = fle_arg(6);
    uint32_t flp = (uint32_t)R[8];
    uint64_t size = ((uint64_t)(uint32_t)R[9] << 32) | (uint32_t)fle_arg(5);
    uint32_t prot = flp & 0xFFu, sec = flp & ~0xFFu;
    const uint8_t *data = NULL;
    uint64_t fsize = 0, osize, o;
    unsigned rd = 0, wr = 0;
    int fd = -1;
    uint64_t abuf = 0;

    fle_vmap_check_thread(api);
    if (attrs != 0) fle_stop(api, "lpFileMappingAttributes is not NULL (security descriptors are not implemented)");
    if (name != 0) fle_stop(api, "a named file mapping object is not implemented");
    if (prot != P_READONLY && prot != P_READWRITE) fle_stop(api, "flProtect is not PAGE_READONLY or PAGE_READWRITE (not implemented)");
    if (sec != 0 && sec != SEC_COMMIT_) fle_stop(api, "flProtect has a SEC_* attribute that is not implemented");
    if (hfile == FLE_INVALID_HANDLE_VALUE) {
        if (size == 0) fle_stop(api, "an anonymous object needs a size; size 0 is a failure the page gives no code for");
        osize = size;
    } else {
        if (!fle_vfs_file_info(api, hfile, &data, &fsize, &rd, &wr)) {
            fail(FLE_W32_ERR_INVALID_HANDLE, 0);
            return;
        }
        if (!rd) fle_stop(api, "the file handle has no read access (the page requires GENERIC_READ; failure code not given)");
        if (prot == P_READWRITE) fle_stop(api, "PAGE_READWRITE over a virtual file is not implemented (no write-back to the file)");
        if (size == 0 && fsize == 0) {
            fail(FLE_ERR_FILE_INVALID, 0);
            return;
        }
        if (size > fsize) fle_stop(api, "a maximum size above the file's length is not defined for a read-only object");
        osize = size ? size : fsize;
    }
    if (osize > FLE_VMAP_MAX_BYTES) fle_stop(api, "object larger than the runtime's limit of 1 GiB");

#ifdef FLE_VMEM_ARENA
    abuf = fle_sys_host_reserve(0, up(osize, FLE_PAGE_SIZE));
    if (!abuf) {
        fail(8 /* ERROR_NOT_ENOUGH_MEMORY, assumption */, 0);
        return;
    }
    memset((void *)(uintptr_t)abuf, 0, (size_t)up(osize, FLE_PAGE_SIZE));
    if (data && osize) memcpy((void *)(uintptr_t)abuf, data, (size_t)osize);
#else
    fd = memfd_create("fle-map", MFD_CLOEXEC);
    if (fd < 0) fle_stop(api, "memfd_create failed");
    if (ftruncate(fd, (off_t)up(osize, FLE_PAGE_SIZE)) != 0) fle_stop(api, "ftruncate failed");
    if (data && osize) {
        uint64_t done = 0;

        while (done < osize) {
            ssize_t w = pwrite(fd, data + done, (size_t)(osize - done), (off_t)done);

            if (w <= 0) fle_stop(api, "pwrite failed");
            done += (uint64_t)w;
        }
    }
#endif
    if (objs_n == objs_cap) {
        uint64_t nc = objs_cap ? objs_cap * 2 : 16;
        vm_obj *n = realloc(objs, nc * sizeof *n);

        if (!n) fle_stop(api, "out of memory");
        objs = n;
        objs_cap = nc;
    }
    o = objs_n++;
    objs[o].fd = fd;
    objs[o].buf = abuf;
    objs[o].size = osize;
    objs[o].prot = prot;
    objs[o].handles = 0;
    objs[o].views = 0;
    objs_live++;
    hook_once();
    R[0] = hand_new(o);
}

/* kernel32!MapViewOfFile: LPVOID f(hFileMappingObject rcx, dwDesiredAccess edx, dwFileOffsetHigh r8d,
 * dwFileOffsetLow r9d, dwNumberOfBytesToMap [5]). MSDN (MapViewOfFile):
 *  - dwDesiredAccess: FILE_MAP_READ (needs PAGE_READONLY or PAGE_READWRITE), FILE_MAP_WRITE and FILE_MAP_ALL_ACCESS
 *    (same thing; needs PAGE_READWRITE; with FILE_MAP_READ added the same), FILE_MAP_COPY (copy-on-write; needs either
 *    protection, and the page says writes become private and are never written back). Any other value (FILE_MAP_EXECUTE,
 *    large pages, CFG) stops. A request the object's protection does not allow is a failure the page gives no code for
 *    and stops.
 *  - the offset must be a multiple of the allocation granularity (65536) and inside the mapping; the length must lie
 *    within the object's size, 0 meaning "to the end"; a request that breaks these stops (the page does not give a code).
 *  - the view is a new range of address space aligned to the allocation granularity, of the length rounded up to the
 *    page size, whose bytes are the object's. Views of one object are coherent (page). The return value is its base.
 *  - a copy-on-write view reports PAGE_WRITECOPY in VirtualQuery (the page lists it as a possible access); that choice is
 *    an inference.
 * The last error is left alone on success. */
void fle_imp_MapViewOfFile(void)
{
    static const char api[] = "MapViewOfFile";
    uint64_t h = R[1], off = ((uint64_t)(uint32_t)R[8] << 32) | (uint32_t)R[9], bytes = fle_arg(5);
    uint32_t acc = (uint32_t)R[2];
    int64_t hi = hand_index(h);
    vm_obj *x;
    uint32_t vprot;
    int hprot, copy = 0;
    uint64_t span, a, len;
    void *p;
    vm_view *v;

    fle_vmap_check_thread(api);
    if (hi < 0) {
        if (h == 0 || h == FLE_INVALID_HANDLE_VALUE) {
            fail(FLE_W32_ERR_INVALID_HANDLE, 0); /* assumption: the page names no code */
            return;
        }
        fle_stop(api, "handle was not issued as a file mapping object");
    }
    if (!hands[hi].live) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0); /* assumption */
        return;
    }
    x = &objs[hands[hi].obj];
    if (acc == FMAP_READ) {
        vprot = P_READONLY;
        hprot = PROT_READ;
    } else if (acc == FMAP_WRITE || acc == (FMAP_WRITE | FMAP_READ) || acc == FMAP_ALL_ACCESS) {
        if (x->prot != P_READWRITE) fle_stop(api, "write access to an object that was not created PAGE_READWRITE (failure code not given)");
        vprot = P_READWRITE;
        hprot = PROT_READ | PROT_WRITE;
    } else if (acc == FMAP_COPY) {
        vprot = P_WRITECOPY;
        hprot = PROT_READ | PROT_WRITE;
        copy = 1;
    } else {
        fle_stop(api, "dwDesiredAccess is not FILE_MAP_READ, FILE_MAP_WRITE, FILE_MAP_ALL_ACCESS or FILE_MAP_COPY");
    }
    if (off % FLE_ALLOC_GRANULARITY) fle_stop(api, "file offset is not a multiple of the allocation granularity (failure code not given)");
    if (off >= x->size) fle_stop(api, "file offset is outside the mapping (failure code not given)");
    if (bytes == 0) bytes = x->size - off;
    if (bytes > x->size - off) fle_stop(api, "dwNumberOfBytesToMap goes beyond the object's size (failure code not given)");
    len = up(bytes, FLE_PAGE_SIZE);

#ifdef FLE_VMEM_ARENA
    (void)hprot;
    (void)span;
    (void)p;
    if (copy) {
        a = fle_sys_host_reserve(0, len);
        if (!a) {
            fail(8 /* ERROR_NOT_ENOUGH_MEMORY, assumption */, 0);
            return;
        }
        memcpy((void *)(uintptr_t)a, (const void *)(uintptr_t)(x->buf + off), (size_t)len);
    } else {
        /* the view is the object's own memory, so every view of the object sees every write (page: views are
         * coherent). Two live views at the same offset would have the same base address: not representable. */
        a = x->buf + off;
        if (view_at_base(a)) fle_stop(api, "a second live view of one object at the same offset (a view cannot be aliased at another address in wasm)");
    }
#else
    span = len + FLE_ALLOC_GRANULARITY - FLE_PAGE_SIZE;
    p = mmap(NULL, span, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (p == MAP_FAILED) {
        fail(8 /* ERROR_NOT_ENOUGH_MEMORY, assumption */, 0);
        return;
    }
    a = up((uint64_t)(uintptr_t)p, FLE_ALLOC_GRANULARITY);
    if (a > (uint64_t)(uintptr_t)p) munmap(p, a - (uint64_t)(uintptr_t)p);
    if ((uint64_t)(uintptr_t)p + span > a + len) munmap((void *)(uintptr_t)(a + len), (uint64_t)(uintptr_t)p + span - (a + len));
    if (mmap((void *)(uintptr_t)a, len, hprot, (copy ? MAP_PRIVATE : MAP_SHARED) | MAP_FIXED, x->fd, (off_t)off) == MAP_FAILED) {
        munmap((void *)(uintptr_t)a, len);
        fail(8 /* ERROR_NOT_ENOUGH_MEMORY, assumption */, 0);
        return;
    }
#endif
    if (views_n == views_cap) {
        uint64_t nc = views_cap ? views_cap * 2 : 16;
        vm_view *n = realloc(views, nc * sizeof *n);

        if (!n) fle_stop(api, "out of memory");
        views = n;
        views_cap = nc;
    }
    v = &views[views_n++];
    v->base = a;
    v->len = len;
    v->prot = vprot;
    v->live = 1;
    v->obj = hands[hi].obj;
    v->own = copy;
    x->views++;
    views_live++;
    R[0] = a;
}

/* kernel32!UnmapViewOfFile: BOOL f(lpBaseAddress rcx). MSDN: the address must be identical to the one MapViewOfFile
 * returned; the range becomes available for other allocations. A value that is not the base of a live view fails with
 * 0 (the page names no code; ERROR_INVALID_ADDRESS 487 is an assumption). Nothing is written back anywhere. */
void fle_imp_UnmapViewOfFile(void)
{
    uint64_t a = R[1], i;

    fle_vmap_check_thread("UnmapViewOfFile");
    for (i = 0; i < views_n; i++) {
        if (views[i].live && views[i].base == a) {
            uint64_t o = views[i].obj;

#ifdef FLE_VMEM_ARENA
            if (views[i].own) fle_sys_host_release(views[i].base);
#else
            if (munmap((void *)(uintptr_t)views[i].base, views[i].len) != 0) fle_stop("UnmapViewOfFile", "munmap failed");
#endif
            views[i].live = 0;
            views_live--;
            objs[o].views--;
            obj_release_if_unused(o);
            R[0] = 1;
            return;
        }
    }
    fail(FLE_ERR_INVALID_ADDRESS_A, 0);
}
