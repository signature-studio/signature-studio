/* Global memory, atoms, MulDiv and lstrlenW (docs/decisions/0024-apis-a3-a5-a13.md; the pages read are listed in
 * docs/investigations/apis-a3-a5-a13-run.md). Single guest thread. A case the Microsoft pages leave open stops the
 * run through fle_stop (fle_env.h); where a page says "the function fails" without naming the error code, the code
 * set here is a runtime choice and is marked "assumption". Nothing is read from the host. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_glob.h"

static long fle_glob_owner_tid;

static void fle_glob_check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (fle_glob_owner_tid == 0) fle_glob_owner_tid = tid;
    if (tid != fle_glob_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

#define FLE_ERR_NOT_ENOUGH_MEMORY 8u
#define FLE_ERR_NOT_LOCKED        158u

/* ---- Global memory ----
 * MSDN (GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock). GMEM_FIXED (0, the default): the return value is a pointer
 * and the handle equals the pointer. GMEM_MOVEABLE: the return value is a handle to the object, translated with
 * GlobalLock; the handle is a token (FLE_GLOBAL_HANDLE_BASE + 16 * (i + 1)) and the block is a host malloc block.
 * The page lists GMEM_MOVEABLE 0x2, GMEM_ZEROINIT 0x40, GHND 0x42, GPTR 0x40 and says the obsolete flags are
 * ignored without giving their numbers. The guest passes 0x2002 and 0x2042 (0x2000 is GMEM_DDESHARE/GMEM_SHARE in
 * the mingw-w64 winbase.h, an unofficial source); 0x2000 is therefore accepted and ignored, and any other bit stops. */
#define FLE_GMEM_MOVEABLE 0x0002u
#define FLE_GMEM_ZEROINIT 0x0040u
#define FLE_GMEM_IGNORED  0x2000u

typedef struct {
    uint8_t *mem;       /* NULL for a zero-byte (discarded) object */
    uint64_t size;
    uint32_t lock;
    int live;
} fle_movable;

static fle_movable *mv;
static uint64_t mv_n, mv_cap, mv_live;

/* Live fixed blocks: an open-addressing set of pointers (0 empty, 1 deleted). */
static uint64_t *fx_tab;
static uint64_t fx_cap, fx_used, fx_live;

static uint64_t fx_hash(uint64_t k) { return ((k >> 4) * UINT64_C(0x9E3779B97F4A7C15)) >> 20; }

static int fx_has(uint64_t k)
{
    uint64_t i, n;

    if (!fx_cap || k < 2) return 0;
    for (n = 0, i = fx_hash(k) & (fx_cap - 1); n < fx_cap; n++, i = (i + 1) & (fx_cap - 1)) {
        if (fx_tab[i] == 0) return 0;
        if (fx_tab[i] == k) return 1;
    }
    return 0;
}

static void fx_insert_raw(uint64_t k)
{
    uint64_t i = fx_hash(k) & (fx_cap - 1);

    while (fx_tab[i] > 1) i = (i + 1) & (fx_cap - 1);
    if (fx_tab[i] == 0) fx_used++;
    fx_tab[i] = k;
    fx_live++;
}

static void fx_insert(uint64_t k)
{
    if ((fx_used + 1) * 2 > fx_cap) {
        uint64_t *old = fx_tab, oc = fx_cap, i;
        uint64_t nc = fx_cap ? fx_cap * 2 : 256;

        fx_tab = calloc(nc, sizeof *fx_tab);
        if (!fx_tab) fle_stop("GlobalAlloc", "out of memory");
        fx_cap = nc;
        fx_used = fx_live = 0;
        for (i = 0; i < oc; i++)
            if (old[i] > 1) fx_insert_raw(old[i]);
        free(old);
    }
    fx_insert_raw(k);
}

static void fx_remove(uint64_t k)
{
    uint64_t i, n;

    for (n = 0, i = fx_hash(k) & (fx_cap - 1); n < fx_cap; n++, i = (i + 1) & (fx_cap - 1)) {
        if (fx_tab[i] == k) {
            fx_tab[i] = 1;
            fx_live--;
            return;
        }
    }
}

uint64_t fle_glob_movable_count(void) { return mv_live; }
uint64_t fle_glob_fixed_count(void) { return fx_live; }

/* The live movable object of a handle, or NULL when the value is not a movable handle that is live. */
static fle_movable *mv_of(uint64_t h)
{
    uint64_t off, i;

    if (h <= FLE_GLOBAL_HANDLE_BASE || h >= FLE_GLOBAL_HANDLE_BASE + FLE_GLOBAL_REGION_SIZE) return NULL;
    off = h - FLE_GLOBAL_HANDLE_BASE;
    if (off % FLE_GLOBAL_HANDLE_STEP) return NULL;
    i = off / FLE_GLOBAL_HANDLE_STEP - 1;
    if (i >= mv_n || !mv[i].live) return NULL;
    return &mv[i];
}

/* kernel32!GlobalAlloc: HGLOBAL GlobalAlloc(UINT uFlags rcx (ecx), SIZE_T dwBytes rdx). The page: zero flags is
 * GMEM_FIXED; GMEM_MOVEABLE with dwBytes 0 returns a handle to a discarded object; failure returns NULL and the code
 * is on GetLastError (the page names none; ERROR_NOT_ENOUGH_MEMORY 8 is an assumption, the only failure here is an
 * allocation that the host refuses). GMEM_FIXED with dwBytes 0 is not defined by the page: it is allocated as one
 * byte, the same choice as LocalAlloc. "Aligned on an 8-byte boundary": malloc gives 16. */
void fle_imp_GlobalAlloc(void)
{
    uint32_t flags = (uint32_t)R[1];
    uint64_t bytes = R[2];
    int movable = (flags & FLE_GMEM_MOVEABLE) != 0, zero = (flags & FLE_GMEM_ZEROINIT) != 0;
    void *p;

    fle_glob_check_thread("GlobalAlloc");
    if (flags & ~(FLE_GMEM_MOVEABLE | FLE_GMEM_ZEROINIT | FLE_GMEM_IGNORED))
        fle_stop("GlobalAlloc", "uFlags has a bit that the page does not give a number for");
    if (movable) {
        fle_movable *o;

        if (mv_n + 2 >= FLE_GLOBAL_REGION_SIZE / FLE_GLOBAL_HANDLE_STEP) fle_stop("GlobalAlloc", "movable handle region exhausted");
        p = NULL;
        if (bytes != 0) {
            p = !fle_host_size_fits(bytes) ? NULL : zero ? calloc(1, (size_t)bytes) : malloc((size_t)bytes);
            if (!p) {
                fle_set_last_error(FLE_ERR_NOT_ENOUGH_MEMORY);
                R[0] = 0;
                return;
            }
        }
        if (mv_n == mv_cap) {
            uint64_t nc = mv_cap ? mv_cap * 2 : 64;
            fle_movable *nm = realloc(mv, nc * sizeof *nm);

            if (!nm) fle_stop("GlobalAlloc", "out of memory");
            mv = nm;
            mv_cap = nc;
        }
        o = &mv[mv_n++];
        o->mem = p;
        o->size = bytes;
        o->lock = 0;
        o->live = 1;
        mv_live++;
        R[0] = FLE_GLOBAL_HANDLE_BASE + FLE_GLOBAL_HANDLE_STEP * mv_n;
        return;
    }
    if (bytes == 0) bytes = 1;
    p = !fle_host_size_fits(bytes) ? NULL : zero ? calloc(1, (size_t)bytes) : malloc((size_t)bytes);
    if (!p) {
        fle_set_last_error(FLE_ERR_NOT_ENOUGH_MEMORY);
        R[0] = 0;
        return;
    }
    fx_insert((uint64_t)(uintptr_t)p);
    R[0] = (uint64_t)(uintptr_t)p;
}

/* kernel32!GlobalFree: HGLOBAL GlobalFree(HGLOBAL hMem rcx). Success returns NULL. A locked object is freed (page).
 * The parameter is annotated _Frees_ptr_opt_ on the page, so NULL is accepted; the page does not say what NULL
 * returns, and NULL (success) is returned without changing the last error (assumption). A value that is neither a
 * live fixed block nor a live movable handle (a handle already freed, a LocalAlloc block, a stray pointer) is not
 * defined by the page and stops the run. */
void fle_imp_GlobalFree(void)
{
    uint64_t h = R[1];
    fle_movable *o;

    fle_glob_check_thread("GlobalFree");
    if (h == 0) {
        R[0] = 0;
        return;
    }
    o = mv_of(h);
    if (o) {
        free(o->mem);
        o->mem = NULL;
        o->live = 0;
        mv_live--;
        R[0] = 0;
        return;
    }
    if (!fx_has(h)) fle_stop("GlobalFree", "handle is not a live GlobalAlloc block (freed, from another allocator, or stray)");
    fx_remove(h);
    free((void *)(uintptr_t)h);
    R[0] = 0;
}

/* kernel32!GlobalLock: LPVOID GlobalLock(HGLOBAL hMem rcx). Fixed: the pointer equals the handle, the lock count
 * stays zero. Movable: the count goes up by one and the pointer to the first byte is returned; a zero-byte (discarded)
 * object returns NULL and its count stays zero (page). The page names no error code for that NULL, so the last error
 * is left alone. An unknown handle stops the run. */
void fle_imp_GlobalLock(void)
{
    uint64_t h = R[1];
    fle_movable *o;

    fle_glob_check_thread("GlobalLock");
    o = mv_of(h);
    if (o) {
        if (o->mem == NULL) {
            R[0] = 0;
            return;
        }
        if (o->lock == UINT32_MAX) fle_stop("GlobalLock", "lock count overflow");
        o->lock++;
        R[0] = (uint64_t)(uintptr_t)o->mem;
        return;
    }
    if (!fx_has(h)) fle_stop("GlobalLock", "handle is not a live GlobalAlloc object");
    R[0] = h;
}

/* kernel32!GlobalUnlock: BOOL GlobalUnlock(HGLOBAL hMem rcx). Fixed: TRUE. Movable: the count goes down by one;
 * still locked gives nonzero; unlocked now gives 0 with the last error NO_ERROR (0); already unlocked gives 0 with
 * ERROR_NOT_LOCKED (158, named on the page and in System Error Codes). A discarded object has a zero count, so it
 * answers like an unlocked one. An unknown handle stops the run. */
void fle_imp_GlobalUnlock(void)
{
    uint64_t h = R[1];
    fle_movable *o;

    fle_glob_check_thread("GlobalUnlock");
    o = mv_of(h);
    if (o) {
        if (o->lock == 0) {
            fle_set_last_error(FLE_ERR_NOT_LOCKED);
            R[0] = 0;
            return;
        }
        o->lock--;
        if (o->lock == 0) {
            fle_set_last_error(0);
            R[0] = 0;
            return;
        }
        R[0] = 1;
        return;
    }
    if (!fx_has(h)) fle_stop("GlobalUnlock", "handle is not a live GlobalAlloc object");
    R[0] = 1;
}

/* ---- Atoms ----
 * MSDN (AddAtomW, DeleteAtom, FindAtomW, GlobalAddAtomW, GlobalFindAtomW, About Atom Tables). Two tables, local and
 * global, empty at start and independent of each other. String atoms are 0xC000..0xFFFF, integer atoms 0x0001..0xBFFF.
 * The value given to a new string atom is a choice (the pages say only "unique"): the first free value from
 * 0xC000 up, continuing after the last one given and wrapping inside 0xC000..0xFFFF. Case is not significant in a
 * search and the case of the first string added is kept; the folding is ASCII only and a name with a unit above
 * 0x7F stops the run (the pages do not say how other letters fold). The name limit is FLE_ATOM_MAX_UNITS; an empty
 * name and a name above the limit are not defined by the pages and stop the run. */
#define FLE_ATOM_SLOTS (FLE_ATOM_STRING_LAST - FLE_ATOM_STRING_FIRST + 1u)
#define FLE_ATOM_INT_LAST 0xBFFFu
#define FLE_ERR_INVALID_PARAMETER_A 87u /* assumption */
#define FLE_ERR_FILE_NOT_FOUND_A    2u  /* assumption */
#define FLE_ERR_INVALID_HANDLE_A    6u  /* assumption */

typedef struct {
    uint16_t *name;
    size_t len;
    uint32_t refs;
} fle_atom;

typedef struct {
    fle_atom *slot;     /* FLE_ATOM_SLOTS entries, allocated at first use */
    uint32_t next;
    uint32_t live;
} fle_atom_table;

static fle_atom_table atoms[2]; /* 0 local, 1 global */

uint64_t fle_glob_atom_count(int global_table) { return atoms[global_table ? 1 : 0].live; }

static fle_atom_table *atom_tab(int g)
{
    fle_atom_table *t = &atoms[g];

    if (!t->slot) {
        t->slot = calloc(FLE_ATOM_SLOTS, sizeof *t->slot);
        if (!t->slot) fle_stop("atom", "out of memory");
    }
    return t;
}

/* What an atom function got in its LPCWSTR: an integer atom (ATOM_INT, value in *v; IS_INTRESOURCE: every bit above
 * the low 16 is zero), a "#dddd" decimal string (ATOM_DEC, value in *v, saturated at 0x10000), or a name. */
enum { ATOM_INT, ATOM_DEC, ATOM_NAME };

static int atom_arg(const char *api, uint64_t p, uint32_t *v, uint16_t **name, size_t *len)
{
    uint16_t *s;
    size_t n, i;

    if ((p >> 16) == 0) {
        *v = (uint32_t)p;
        return ATOM_INT;
    }
    s = fle_w_read_alloc(api, p, FLE_ATOM_MAX_UNITS, &n);
    if (n == 0) fle_stop(api, "empty atom name is not defined");
    for (i = 0; i < n; i++)
        if (s[i] > 0x7F) fle_stop(api, "atom name with a non-ASCII character (case folding of other letters is not defined)");
    if (s[0] == '#' && n >= 2) {
        uint32_t d = 0;

        for (i = 1; i < n && s[i] >= '0' && s[i] <= '9'; i++) {
            d = d * 10u + (uint32_t)(s[i] - '0');
            if (d > 0x10000u) d = 0x10000u;
        }
        if (i == n) {
            free(s);
            *v = d;
            return ATOM_DEC;
        }
    }
    *name = s;
    *len = n;
    return ATOM_NAME;
}

static long atom_find(const fle_atom_table *t, const uint16_t *name, size_t len)
{
    uint32_t i;

    for (i = 0; i < FLE_ATOM_SLOTS; i++)
        if (t->slot[i].name && fle_w_icmp(t->slot[i].name, t->slot[i].len, name, len) == 0) return (long)i;
    return -1;
}

static void atom_fail(uint32_t err)
{
    fle_set_last_error(err);
    R[0] = 0;
}

/* Add: an integer atom needs 0x0001..0xBFFF (page); "#dddd" gives the number, and 0 or 0xC000 and above fail (page).
 * A name already present returns its atom and adds a reference; a new name gets a new atom with one reference. The
 * failures return 0; the code is not named by the page (87, assumption). */
static void atom_add(const char *api, int g)
{
    uint32_t v = 0;
    uint16_t *name = NULL;
    size_t len = 0;
    fle_atom_table *t;
    long i;
    uint32_t k;

    fle_glob_check_thread(api);
    switch (atom_arg(api, R[1], &v, &name, &len)) {
    case ATOM_INT:
        if ((v & 0xFFFFu) < 1u || (v & 0xFFFFu) > FLE_ATOM_INT_LAST) {
            atom_fail(FLE_ERR_INVALID_PARAMETER_A);
            return;
        }
        R[0] = v & 0xFFFFu;
        return;
    case ATOM_DEC:
        if (v < 1u || v > FLE_ATOM_INT_LAST) {
            atom_fail(FLE_ERR_INVALID_PARAMETER_A);
            return;
        }
        R[0] = v;
        return;
    default:
        break;
    }
    t = atom_tab(g);
    i = atom_find(t, name, len);
    if (i >= 0) {
        if (t->slot[i].refs == UINT32_MAX) fle_stop(api, "atom reference count overflow");
        t->slot[i].refs++;
        free(name);
        R[0] = FLE_ATOM_STRING_FIRST + (uint32_t)i;
        return;
    }
    if (t->live >= FLE_ATOM_SLOTS) fle_stop(api, "atom table full (not defined by the pages)");
    for (k = 0; t->slot[(t->next + k) % FLE_ATOM_SLOTS].name; k++) {}
    i = (long)((t->next + k) % FLE_ATOM_SLOTS);
    t->slot[i].name = name;
    t->slot[i].len = len;
    t->slot[i].refs = 1;
    t->live++;
    t->next = ((uint32_t)i + 1u) % FLE_ATOM_SLOTS;
    R[0] = FLE_ATOM_STRING_FIRST + (uint32_t)i;
}

/* Find: the search ignores case (page). An integer atom (MAKEINTATOM, low word 0x0001..0xBFFF) is returned as it is
 * and a word outside that range fails (page); "#dddd" in range gives the number (the AddAtom page's rule; the Find
 * pages do not say what an out-of-range "#dddd" does, so that stops the run). A name that is not in the table fails
 * with 0 (page); the code is not named (2, assumption). */
static void atom_findx(const char *api, int g)
{
    uint32_t v = 0;
    uint16_t *name = NULL;
    size_t len = 0;
    long i;

    fle_glob_check_thread(api);
    switch (atom_arg(api, R[1], &v, &name, &len)) {
    case ATOM_INT:
        if ((v & 0xFFFFu) < 1u || (v & 0xFFFFu) > FLE_ATOM_INT_LAST) {
            atom_fail(FLE_ERR_INVALID_PARAMETER_A);
            return;
        }
        R[0] = v & 0xFFFFu;
        return;
    case ATOM_DEC:
        if (v < 1u || v > FLE_ATOM_INT_LAST) fle_stop(api, "\"#dddd\" outside 1..0xBFFF is not defined for a search");
        R[0] = v;
        return;
    default:
        break;
    }
    i = atom_find(atom_tab(g), name, len);
    free(name);
    if (i < 0) {
        atom_fail(FLE_ERR_FILE_NOT_FOUND_A);
        return;
    }
    R[0] = FLE_ATOM_STRING_FIRST + (uint32_t)i;
}

void fle_imp_AddAtomW(void) { atom_add("AddAtomW", 0); }
void fle_imp_GlobalAddAtomW(void) { atom_add("GlobalAddAtomW", 1); }
void fle_imp_FindAtomW(void) { atom_findx("FindAtomW", 0); }
void fle_imp_GlobalFindAtomW(void) { atom_findx("GlobalFindAtomW", 1); }

/* kernel32!DeleteAtom: ATOM DeleteAtom(ATOM nAtom cx). An integer atom (0x0001..0xBFFF): no effect, returns 0 (page).
 * A string atom in the local table: the count goes down by one and the name goes away at zero; success returns 0.
 * An atom that is not in the table fails and returns nAtom (page); the code is not named (6, assumption). */
void fle_imp_DeleteAtom(void)
{
    uint16_t a = (uint16_t)R[1];
    fle_atom_table *t;
    fle_atom *e;

    fle_glob_check_thread("DeleteAtom");
    if (a >= 1u && a <= FLE_ATOM_INT_LAST) {
        R[0] = 0;
        return;
    }
    t = atom_tab(0);
    e = a >= FLE_ATOM_STRING_FIRST ? &t->slot[a - FLE_ATOM_STRING_FIRST] : NULL;
    if (!e || !e->name) {
        fle_set_last_error(FLE_ERR_INVALID_HANDLE_A);
        R[0] = a;
        return;
    }
    if (--e->refs == 0) {
        free(e->name);
        e->name = NULL;
        e->len = 0;
        t->live--;
    }
    R[0] = 0;
}

/* ---- MulDiv, lstrlenW ----
 * MSDN MulDiv: (nNumber * nNumerator) / nDenominator with a 64-bit intermediate, rounded to the nearest integer; a
 * positive half integer rounds up and a negative one rounds down (so halves go away from zero); an overflow of the
 * 32-bit result or nDenominator 0 returns -1. */
void fle_imp_MulDiv(void)
{
    int64_t a = (int32_t)(uint32_t)R[1], b = (int32_t)(uint32_t)R[2], d = (int32_t)(uint32_t)R[8];
    int64_t p = a * b;
    uint64_t up, ud, q;
    int neg;

    if (d == 0) {
        R[0] = 0xFFFFFFFFu;
        return;
    }
    neg = (p < 0) != (d < 0) && p != 0;
    up = p < 0 ? (uint64_t)(-p) : (uint64_t)p;
    ud = d < 0 ? (uint64_t)(-d) : (uint64_t)d;
    q = (2u * up + ud) / (2u * ud);
    if (neg ? q > UINT64_C(0x80000000) : q > UINT64_C(0x7FFFFFFF)) {
        R[0] = 0xFFFFFFFFu;
        return;
    }
    R[0] = (uint32_t)(neg ? (int32_t)(-(int64_t)q) : (int32_t)q);
}

/* kernel32!lstrlenW: int lstrlenW(LPCWSTR rcx). The number of UTF-16 units before the NUL; NULL returns 0 (page). */
void fle_imp_lstrlenW(void)
{
    uint64_t p = R[1];
    uint64_t n = 0;

    if (p != 0)
        while (fle_ld(p + 2u * n, 2) != 0) n++;
    R[0] = (uint32_t)n;
}
