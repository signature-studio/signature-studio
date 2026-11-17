/* In-memory virtual registry (docs/decisions/0018-virtual-fs-reg-env.md; sources and the implemented / not
 * implemented list in docs/investigations/import-vfs-spec.md). The host registry is never read. The five
 * predefined roots exist and are empty, except that HKEY_LOCAL_MACHINE has one empty key, Software (decision
 * 0085, provisional). A missing key is ERROR_FILE_NOT_FOUND (2). Inputs the Microsoft pages
 * leave undefined, and features this runtime does not model (links, volatile semantics, security descriptors,
 * the WOW64 views, transactions, performance data), stop the run. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_reg.h"

#define KEY_NAME_MAX   255u      /* MSDN: Registry Element Size Limits (key name) */
#define VALUE_NAME_MAX 16383u    /* same page (value name) */
#define VALUE_DATA_MAX (1024u * 1024u)
#define CREATE_LEVELS_MAX 32u    /* MSDN: RegCreateKeyExW / Element Size Limits: 32 levels per call */
#define TREE_DEPTH_MAX 512u      /* same page: a tree can be 512 levels deep */

typedef struct {
    uint16_t *name;
    size_t nlen;
    uint32_t type;
    uint8_t *data;
    uint32_t size;
} reg_value;

typedef struct reg_key {
    uint16_t *name;
    size_t nlen;
    struct reg_key *parent;
    struct reg_key **kids;
    size_t nkids, kcap;
    reg_value *vals;
    size_t nvals, vcap;
    unsigned opens;
    unsigned depth;
    uint64_t wt;
} reg_key;

static const uint16_t empty_w[1] = { 0 };
static reg_key roots[5];                    /* CLASSES_ROOT, CURRENT_USER, LOCAL_MACHINE, USERS, CURRENT_CONFIG */
static const uint32_t root_low[5] = { 0x80000000u, 0x80000001u, 0x80000002u, 0x80000003u, 0x80000005u };
static uint64_t reg_clock = FLE_VFS_CLOCK_START;

static uint64_t rtick(void)
{
    reg_clock += UINT64_C(10000000);
    return reg_clock;
}

typedef struct {
    int used;
    reg_key *key;
    uint32_t access;
} kh_ent;

static kh_ent *kh;
static size_t kh_n, kh_cap;

static uint16_t *dup_w(const uint16_t *s, size_t n)
{
    uint16_t *c = malloc((n + 1) * sizeof *c);
    if (!c) fle_stop("registry", "out of memory");
    memcpy(c, s, n * sizeof *c);
    c[n] = 0;
    return c;
}

static reg_key *kid_find(const reg_key *k, const uint16_t *n, size_t nl)
{
    size_t i;
    for (i = 0; i < k->nkids; i++)
        if (fle_w_icmp(k->kids[i]->name, k->kids[i]->nlen, n, nl) == 0) return k->kids[i];
    return NULL;
}

static reg_key *kid_add(reg_key *par, const uint16_t *n, size_t nl)
{
    reg_key *k = calloc(1, sizeof *k);
    if (!k) fle_stop("registry", "out of memory");
    k->name = dup_w(n, nl);
    k->nlen = nl;
    k->parent = par;
    k->depth = par->depth + 1;
    k->wt = rtick();
    if (par->nkids == par->kcap) {
        size_t nc = par->kcap ? par->kcap * 2 : 8;
        reg_key **nk = realloc(par->kids, nc * sizeof *nk);
        if (!nk) fle_stop("registry", "out of memory");
        par->kids = nk;
        par->kcap = nc;
    }
    par->kids[par->nkids++] = k;
    par->wt = rtick();
    return k;
}

static reg_value *val_find(const reg_key *k, const uint16_t *n, size_t nl)
{
    size_t i;
    for (i = 0; i < k->nvals; i++)
        if (fle_w_icmp(k->vals[i].name, k->vals[i].nlen, n, nl) == 0) return &k->vals[i];
    return NULL;
}

/* Decision 0085 (provisional): HKEY_LOCAL_MACHINE starts with one empty key, Software (Learn: Registry Hives and
 * Structure of the Registry list it; nothing there says Windows 10 22H2). The roots are static, so the key is
 * added on the first call that resolves a handle (every entry point does, through predefined()). It is an
 * ordinary key (enumerate, query and delete treat it like any other). No handle is issued, and the virtual
 * clock and the root's write time are put back, so no later time stamp changes; the write time of Software is
 * the clock value at that moment, a choice, not a value from a page. Microsoft and below are not made. */
static int reg_booted;

static void reg_boot(void)
{
    static const uint16_t software[8] = { 'S', 'o', 'f', 't', 'w', 'a', 'r', 'e' };
    uint64_t saved_clock = reg_clock, saved_root_wt = roots[2].wt;
    reg_key *k;
    reg_booted = 1;
    k = kid_add(&roots[2], software, 8);
    reg_clock = saved_clock;
    k->wt = saved_clock;
    roots[2].wt = saved_root_wt;
}

/* A predefined handle: the low 32 bits name the root and the high 32 bits are all 0 or all 1. FLEngine passes
 * the zero-extended form (mov ecx,0x80000001 at 0x419a12); the sign-extended form is what winnt.h defines.
 * Accepting both is an inference about advapi32, not something the pages read state. */
static reg_key *predefined(uint64_t h)
{
    uint32_t hi = (uint32_t)(h >> 32), lo = (uint32_t)h;
    size_t i;
    if (!reg_booted) reg_boot();
    if (hi != 0 && hi != 0xFFFFFFFFu) return NULL;
    for (i = 0; i < 5; i++)
        if (lo == root_low[i]) return &roots[i];
    if (lo == 0x80000004u || lo == 0x80000050u || lo == 0x80000060u)
        fle_stop("registry", "HKEY_PERFORMANCE_DATA / _TEXT / _NLSTEXT are not implemented");
    return NULL;
}

/* Resolves an hKey argument. Returns the key (access = all rights for a predefined key) or NULL when the handle
 * is not valid (the caller returns ERROR_INVALID_HANDLE; the page names no code - inference). A value in the key
 * handle region that was not issued stops. */
static reg_key *key_arg(const char *api, uint64_t h, uint32_t *access)
{
    reg_key *p = predefined(h);
    uint64_t off, idx;
    if (p) {
        *access = 0xFFFFFFFFu;
        return p;
    }
    if (h == 0) return NULL;
    if (h <= FLE_REG_KEY_BASE || h >= FLE_REG_KEY_BASE + FLE_HANDLE_REGION_SIZE)
        fle_stop(api, "hKey is neither a predefined key nor a handle issued by the virtual registry");
    off = h - FLE_REG_KEY_BASE;
    if (off % 4) return NULL;
    idx = off / 4 - 1;
    if (idx >= kh_n || !kh[idx].used) return NULL;
    *access = kh[idx].access;
    return kh[idx].key;
}

static uint64_t kh_alloc(reg_key *k, uint32_t access)
{
    if (kh_n == kh_cap) {
        size_t nc = kh_cap ? kh_cap * 2 : 32;
        kh_ent *nk = realloc(kh, nc * sizeof *nk);
        if (!nk) fle_stop("registry", "out of memory");
        kh = nk;
        kh_cap = nc;
    }
    if (kh_n + 2 >= FLE_HANDLE_REGION_SIZE / 4) fle_stop("registry", "key handle region exhausted");
    kh[kh_n].used = 1;
    kh[kh_n].key = k;
    kh[kh_n].access = access;
    k->opens++;
    kh_n++;
    return FLE_REG_KEY_BASE + 4u * (uint64_t)kh_n;
}

/* Decision 0126 (PROVISIONAL), wall 81 (S-wow64-views): KEY_WOW64_64KEY 0x0100 and KEY_WOW64_32KEY 0x0200 (Learn, Accessing an Alternate Registry View and Registry Key Security and Access Rights: "Indicates that an
 * application on 64-bit Windows should operate on the 32-bit [64-bit] registry view"; "These flags have no effect on shared registry keys"; "Either KEY_WOW64_32KEY or KEY_WOW64_64KEY can be specified. If both flags are
 * specified, the function fails with ERROR_INVALID_PARAMETER"). The numbers are the SDK's (the Learn pages name the flags without values: UNOFFICIAL). The virtual registry is ONE view (decision 0085: a 64-bit process, no
 * redirected keys): the guest passes KEY_READ | KEY_WOW64_32KEY (0x20219) to read HKLM\Software keys; the flag selects a view this registry does not have, and no key of the seed is redirected, so one flag is accepted and
 * changes nothing (a CHOICE: the 32-bit view of a key the seed holds is the same key). Both flags together fail, as the page says, with ERROR_INVALID_PARAMETER (87). Generic rights and MAXIMUM_ALLOWED still stop. */
#define FLE_KEY_WOW64_64KEY 0x00000100u
#define FLE_KEY_WOW64_32KEY 0x00000200u

static uint32_t check_sam(const char *api, uint32_t sam)
{
    if (sam & ~(0x000F003Fu | FLE_KEY_WOW64_64KEY | FLE_KEY_WOW64_32KEY))
        fle_stop(api, "samDesired has bits that are not implemented (generic, MAXIMUM_ALLOWED)");
    return sam;
}

/* Splits a subkey path at backslashes. Empty components (leading, trailing or doubled backslashes) and
 * components over 255 characters are not defined by the pages and stop the run. With allow_empty (decision 0099: RegOpenKeyEx only) an
 * empty component is recorded with length 0 instead of stopping here, so that the caller walks the components in order and stops only if
 * the walk reaches it (a missing key before it ends the walk first: ERROR_FILE_NOT_FOUND). */
typedef struct {
    const uint16_t *p[CREATE_LEVELS_MAX + 1];
    size_t l[CREATE_LEVELS_MAX + 1];
    size_t n;
} kpath;

static void split_key(const char *api, const uint16_t *s, size_t len, kpath *kp, size_t max_levels, int allow_empty)
{
    size_t i = 0;
    kp->n = 0;
    if (len == 0) return;
    for (;;) {
        size_t j = i;
        while (j < len && s[j] != '\\') j++;
        if (j == i && !allow_empty) fle_stop(api, "an empty component (leading, trailing or doubled backslash) in a key path");
        if (j - i > KEY_NAME_MAX) fle_stop(api, "key name component longer than 255 characters");
        if (kp->n >= max_levels) fle_stop(api, "key path has more levels than the page allows per call");
        kp->p[kp->n] = s + i;
        kp->l[kp->n] = j - i;
        kp->n++;
        if (j >= len) break;
        i = j + 1;
    }
}

static uint32_t ret(uint32_t code)
{
    R[0] = code;
    return code;
}

/* ---- open ---- */

static void open_common(const char *api, uint64_t hkey, const uint16_t *sub, size_t sublen, uint32_t options,
                        uint32_t sam, uint64_t pout)
{
    uint32_t acc;
    reg_key *base, *cur;
    kpath kp;
    size_t i;
    int is_pre;

    if (options != 0) fle_stop(api, "ulOptions is not 0 (REG_OPTION_OPEN_LINK is not implemented)");
    if (pout == 0) fle_stop(api, "phkResult is NULL");
    check_sam(api, sam);
    if ((sam & (FLE_KEY_WOW64_64KEY | FLE_KEY_WOW64_32KEY)) == (FLE_KEY_WOW64_64KEY | FLE_KEY_WOW64_32KEY)) {
        ret(87u);               /* the page: "If both flags are specified, the function fails with ERROR_INVALID_PARAMETER" */
        return;
    }
    base = key_arg(api, hkey, &acc);
    if (!base) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    is_pre = predefined(hkey) != NULL;
    split_key(api, sub, sublen, &kp, 512, 1);
    if (kp.n == 0 && is_pre) {
        /* Page: a NULL or empty subkey with a predefined key returns the same handle. */
        fle_st(pout, 8, hkey);
        ret(0);
        return;
    }
    cur = base;
    for (i = 0; i < kp.n; i++) {
        /* Decision 0099 (provisional): the walk is in path order; an empty component is reached only when every key before it exists. What
         * Windows does there is on no page (Learn does not say; a Microsoft blog says a doubled backslash is not legal), so it stops. */
        if (kp.l[i] == 0)
            fle_stop(api, "an empty component (leading, trailing or doubled backslash) in a key path, reached after the keys before it exist");
        cur = kid_find(cur, kp.p[i], kp.l[i]);
        if (!cur) {
            ret(FLE_W32_ERR_FILE_NOT_FOUND);
            return;
        }
    }
    fle_st(pout, 8, kh_alloc(cur, sam));
    ret(0);
}

/* advapi32!RegOpenKeyExW (MSDN: RegOpenKeyExW). hKey rcx, lpSubKey rdx (optional), ulOptions r8d, samDesired
 * r9d, phkResult [5]. Key names are not case sensitive (page). A NULL or empty subkey with a predefined key
 * returns the same handle (page); with another key it returns a new handle to that key (page: "a new handle").
 * A missing key returns ERROR_FILE_NOT_FOUND (2) (page example). An invalid hKey returns 6 (inference). */
void fle_imp_RegOpenKeyExW(void)
{
    static const char api[] = "RegOpenKeyExW";
    uint16_t *s = NULL;
    size_t len = 0;
    if (R[2] != 0) s = fle_w_read_alloc(api, R[2], KEY_NAME_MAX * 512u, &len);
    open_common(api, R[1], s, len, (uint32_t)R[8], (uint32_t)R[9], fle_arg(5));
    free(s);
}

/* advapi32!RegOpenKeyExA: same, with an ANSI subkey name (the page is the RegOpenKeyExA page). Only ASCII is
 * accepted: the ANSI code page is not modelled, a byte above 0x7F stops. */
void fle_imp_RegOpenKeyExA(void)
{
    static const char api[] = "RegOpenKeyExA";
    uint16_t *s = NULL;
    size_t len = 0;
    if (R[2] != 0) {
        size_t n = 0, i;
        while (fle_ld(R[2] + n, 1) != 0) {
            if (++n > KEY_NAME_MAX * 512u) fle_stop(api, "subkey name longer than the supported limit");
        }
        s = malloc((n + 1) * sizeof *s);
        if (!s) fle_stop(api, "out of memory");
        for (i = 0; i < n; i++) {
            uint8_t c = (uint8_t)fle_ld(R[2] + i, 1);
            if (c > 0x7f) fle_stop(api, "non-ASCII byte in an ANSI key name (code page not modelled)");
            s[i] = c;
        }
        s[n] = 0;
        len = n;
    }
    open_common(api, R[1], s, len, (uint32_t)R[8], (uint32_t)R[9], fle_arg(5));
    free(s);
}

/* advapi32!RegCloseKey (MSDN: RegCloseKey). Closing a predefined key succeeds and changes nothing (the page
 * says only keys that are not predefined need closing; success for a predefined key is an inference). A closed
 * or invalid handle returns 6 (inference). */
void fle_imp_RegCloseKey(void)
{
    static const char api[] = "RegCloseKey";
    uint64_t h = R[1], off;
    if (predefined(h)) {
        ret(0);
        return;
    }
    if (h == 0) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (h <= FLE_REG_KEY_BASE || h >= FLE_REG_KEY_BASE + FLE_HANDLE_REGION_SIZE)
        fle_stop(api, "hKey is neither a predefined key nor a handle issued by the virtual registry");
    off = h - FLE_REG_KEY_BASE;
    if (off % 4 || off / 4 - 1 >= kh_n || !kh[off / 4 - 1].used) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    kh[off / 4 - 1].used = 0;
    kh[off / 4 - 1].key->opens--;
    ret(0);
}

/* ---- create ---- */

/* advapi32!RegCreateKeyExW (MSDN: RegCreateKeyExW). hKey rcx, lpSubKey rdx, Reserved r8d (0), lpClass r9 (ignored,
 * the page allows that), dwOptions [5], samDesired [6], lpSecurityAttributes [7] (non-NULL stops), phkResult
 * [8], lpdwDisposition [9]. Creates every missing key of the path (up to 32 levels, page) and opens the final
 * one; the disposition is REG_CREATED_NEW_KEY (1) when the final key was created, else REG_OPENED_EXISTING_KEY
 * (2). An empty subkey gives a new handle to hKey (page). NULL subkey stops (page: cannot be NULL). dwOptions
 * 0 and REG_OPTION_VOLATILE (1) are accepted (volatile is the same in memory); backup/restore and link stop.
 * The page says a key cannot be created as a direct child of HKEY_USERS or HKEY_LOCAL_MACHINE but names no
 * code, so that case stops (HKEY_LOCAL_MACHINE\Software exists from the start, decision 0085, so a path that
 * begins with it never meets this rule). The access to hKey is not checked against samDesired (page: the check is against
 * the security descriptor, which is not modelled). */
void fle_imp_RegCreateKeyExW(void)
{
    static const char api[] = "RegCreateKeyExW";
    uint64_t hkey = R[1], pout = fle_arg(8), pdisp = fle_arg(9);
    uint32_t options = (uint32_t)fle_arg(5), sam = (uint32_t)fle_arg(6), acc;
    uint16_t *s;
    size_t len, i;
    reg_key *cur, *base;
    kpath kp;
    uint32_t disp = 2;

    if (R[2] == 0) fle_stop(api, "lpSubKey is NULL (the page says it cannot be)");
    if ((uint32_t)R[8] != 0) fle_stop(api, "Reserved is not 0");
    if (options > 1) fle_stop(api, "dwOptions is not REG_OPTION_NON_VOLATILE or REG_OPTION_VOLATILE");
    if (fle_arg(7) != 0) fle_stop(api, "lpSecurityAttributes is not NULL (not implemented)");
    if (pout == 0) fle_stop(api, "phkResult is NULL");
    check_sam(api, sam);
    if ((sam & (FLE_KEY_WOW64_64KEY | FLE_KEY_WOW64_32KEY)) == (FLE_KEY_WOW64_64KEY | FLE_KEY_WOW64_32KEY)) {
        ret(87u);
        return;
    }
    s = fle_w_read_alloc(api, R[2], KEY_NAME_MAX * CREATE_LEVELS_MAX, &len);
    base = key_arg(api, hkey, &acc);
    if (!base) {
        free(s);
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    /* Decision 0126 (PROVISIONAL), wall 73 (S-regcreate-trailing-backslash): the guest creates "Software\Image-Line\Shared\" (a path that ends with ONE backslash: its Delphi code builds the key as a base path plus a "\" and the
     * name that follows, and the name is empty here). The pages of RegCreateKeyExW say "creates all missing keys in the specified path" and "If lpSubKey is a pointer to an empty string, phkResult receives a new handle to
     * the key specified by hKey" and say nothing of a backslash at the end of the path. A CHOICE WITHOUT A SOURCE: one trailing backslash is dropped (the key is the one the path names without it); a leading or a doubled
     * backslash, and a path that is only a backslash, still stop. */
    if (len >= 2 && s[len - 1] == '\\' && s[len - 2] != '\\') len--;
    split_key(api, s, len, &kp, CREATE_LEVELS_MAX, 0);
    cur = base;
    for (i = 0; i < kp.n; i++) {
        reg_key *k = kid_find(cur, kp.p[i], kp.l[i]);
        if (!k) {
            if (cur == &roots[2] || cur == &roots[3])
                fle_stop(api, "creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page)");
            if (cur->depth + 1 > TREE_DEPTH_MAX) fle_stop(api, "key tree deeper than 512 levels");
            k = kid_add(cur, kp.p[i], kp.l[i]);
            disp = 1;
        }
        cur = k;
    }
    free(s);
    if (pdisp) fle_st(pdisp, 4, disp);
    fle_st(pout, 8, kh_alloc(cur, sam));
    ret(0);
}

/* ---- values ---- */

static int type_ok(uint32_t t)
{
    return t == FLE_REG_NONE || t == FLE_REG_SZ || t == FLE_REG_EXPAND_SZ || t == FLE_REG_BINARY ||
           t == FLE_REG_DWORD || t == FLE_REG_MULTI_SZ || t == FLE_REG_QWORD;
}

/* advapi32!RegSetValueExW (MSDN: RegSetValueExW, Registry Value Types). hKey rcx, lpValueName rdx (NULL or ""
 * is the unnamed value), Reserved r8d (0), dwType r9d, lpData [5], cbData [6]. The key must have been opened
 * with KEY_SET_VALUE (page); otherwise 5 (inference for the code). Types accepted: NONE, SZ, EXPAND_SZ,
 * BINARY, DWORD, MULTI_SZ, QWORD; DWORD_BE and LINK stop. SZ/EXPAND_SZ data must be whole UTF-16 units ending
 * in a NUL and MULTI_SZ must end in two NULs, with cbData counting them (page); otherwise it stops. DWORD
 * needs cbData 4 and QWORD 8 (the page does not say what other sizes do: stop). lpData NULL needs cbData 0.
 * Values are limited to 1 MiB (the page's standard-format limit) and names to 16,383 characters. */
void fle_imp_RegSetValueExW(void)
{
    static const char api[] = "RegSetValueExW";
    uint32_t type = (uint32_t)R[9], cb = (uint32_t)fle_arg(6), acc;
    uint64_t pd = fle_arg(5);
    reg_key *k;
    uint16_t *name = NULL;
    size_t nl = 0;
    reg_value *v;
    uint8_t *copy = NULL;

    if ((uint32_t)R[8] != 0) fle_stop(api, "Reserved is not 0");
    if (!type_ok(type)) fle_stop(api, "value type is not implemented (REG_DWORD_BIG_ENDIAN, REG_LINK, others)");
    if (cb > VALUE_DATA_MAX) fle_stop(api, "value larger than 1 MiB");
    if (pd == 0 && cb != 0) fle_stop(api, "lpData is NULL but cbData is not 0");
    if (type == FLE_REG_SZ || type == FLE_REG_EXPAND_SZ) {
        if (cb < 2 || cb % 2 || fle_ld(pd + cb - 2, 2) != 0) fle_stop(api, "string data is not NUL-terminated within cbData");
    } else if (type == FLE_REG_MULTI_SZ) {
        if (cb < 4 || cb % 2 || fle_ld(pd + cb - 2, 2) != 0 || fle_ld(pd + cb - 4, 2) != 0)
            fle_stop(api, "REG_MULTI_SZ data does not end in two NULs within cbData");
    } else if (type == FLE_REG_DWORD && cb != 4) {
        fle_stop(api, "REG_DWORD with cbData != 4");
    } else if (type == FLE_REG_QWORD && cb != 8) {
        fle_stop(api, "REG_QWORD with cbData != 8");
    }
    k = key_arg(api, R[1], &acc);
    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_SET_VALUE)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (R[2] != 0) name = fle_w_read_alloc(api, R[2], VALUE_NAME_MAX, &nl);
    else name = dup_w(NULL, 0);
    if (cb) {
        copy = malloc(cb);
        if (!copy) fle_stop(api, "out of memory");
        memcpy(copy, (const void *)fle_addr(pd, (unsigned)cb, 0), cb);
    }
    v = val_find(k, name, nl);
    if (v) {
        free(v->data);
        free(name);
    } else {
        if (k->nvals == k->vcap) {
            size_t nc = k->vcap ? k->vcap * 2 : 8;
            reg_value *nv = realloc(k->vals, nc * sizeof *nv);
            if (!nv) fle_stop(api, "out of memory");
            k->vals = nv;
            k->vcap = nc;
        }
        v = &k->vals[k->nvals++];
        v->name = name;
        v->nlen = nl;
    }
    v->type = type;
    v->data = copy;
    v->size = cb;
    k->wt = rtick();
    ret(0);
}

/* Shared data-out rule of RegQueryValueExW and RegEnumValueW (MSDN: both pages). pdata may be NULL; pcb may be
 * NULL only when pdata is NULL (otherwise the page's rule is broken and the run stops). With pdata NULL and
 * pcb non-NULL the size is stored and the call succeeds. A buffer that is too small gives ERROR_MORE_DATA (234)
 * and the required size in *pcb. Returns the status. */
static uint32_t data_out(const char *api, const reg_value *v, uint64_t ptype, uint64_t pdata, uint64_t pcb)
{
    if (pcb == 0 && pdata != 0) fle_stop(api, "lpcbData is NULL but lpData is not (the page allows NULL only when lpData is NULL)");
    if (pdata != 0) {
        uint32_t cap = (uint32_t)fle_ld(pcb, 4);
        if (cap < v->size) {
            if (ptype) fle_st(ptype, 4, v->type);
            fle_st(pcb, 4, v->size);
            return FLE_W32_ERR_MORE_DATA;
        }
        if (v->size) memcpy((void *)fle_addr(pdata, (unsigned)v->size, 1), v->data, v->size);
    }
    if (ptype) fle_st(ptype, 4, v->type);
    if (pcb) fle_st(pcb, 4, v->size);
    return 0;
}

/* advapi32!RegQueryValueExW (MSDN: RegQueryValueExW). hKey rcx (needs KEY_QUERY_VALUE: else 5, inference),
 * lpValueName rdx (NULL or "" is the unnamed value), lpReserved r8 (must be NULL: stops), lpType r9, lpData [5],
 * lpcbData [6]. A missing value returns ERROR_FILE_NOT_FOUND (2) (page). Data is returned as stored: strings
 * are NOT given extra terminators (page). The type is also written with ERROR_MORE_DATA (the page is silent:
 * inference). */
void fle_imp_RegQueryValueExW(void)
{
    static const char api[] = "RegQueryValueExW";
    uint32_t acc;
    reg_key *k;
    uint16_t *name = NULL;
    size_t nl = 0;
    reg_value *v;

    if (R[8] != 0) fle_stop(api, "lpReserved is not NULL");
    k = key_arg(api, R[1], &acc);
    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_QUERY_VALUE)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (R[2] != 0) name = fle_w_read_alloc(api, R[2], VALUE_NAME_MAX, &nl);
    v = val_find(k, name ? name : empty_w, nl);
    free(name);
    if (!v) {
        ret(FLE_W32_ERR_FILE_NOT_FOUND);
        return;
    }
    ret(data_out(api, v, R[9], fle_arg(5), fle_arg(6)));
}

/* advapi32!RegDeleteValueW (MSDN: RegDeleteValueW). hKey rcx (KEY_SET_VALUE, else 5), lpValueName rdx (NULL or
 * "" is the unnamed value). A missing value returns 2 (the page names no code; inference). */
void fle_imp_RegDeleteValueW(void)
{
    static const char api[] = "RegDeleteValueW";
    uint32_t acc;
    reg_key *k = key_arg(api, R[1], &acc);
    uint16_t *name = NULL;
    size_t nl = 0;
    reg_value *v;

    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_SET_VALUE)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (R[2] != 0) name = fle_w_read_alloc(api, R[2], VALUE_NAME_MAX, &nl);
    v = val_find(k, name ? name : empty_w, nl);
    free(name);
    if (!v) {
        ret(FLE_W32_ERR_FILE_NOT_FOUND);
        return;
    }
    free(v->name);
    free(v->data);
    memmove(v, v + 1, (size_t)(k->vals + k->nvals - (v + 1)) * sizeof *v);
    k->nvals--;
    k->wt = rtick();
    ret(0);
}

/* ---- enumeration and info ---- */

static void put_filetime(uint64_t p, uint64_t t)
{
    if (p) fle_st(p, 8, t);
}

/* advapi32!RegEnumKeyExW (MSDN: RegEnumKeyExW). hKey rcx (KEY_ENUMERATE_SUB_KEYS, else 5), dwIndex edx, lpName
 * r8, lpcchName r9, lpReserved [5] (NULL), lpClass [6], lpcchClass [7], lpftLastWriteTime [8]. Keys come in
 * creation order (the page says the order is arbitrary). An index past the last key returns ERROR_NO_MORE_ITEMS
 * (259). A name that does not fit (size includes the NUL) returns ERROR_MORE_DATA (234) and copies nothing; the
 * page does not say what *lpcchName holds then and it is left alone. Keys have no class: if lpClass is given,
 * it receives "" and *lpcchClass 0. */
void fle_imp_RegEnumKeyExW(void)
{
    static const char api[] = "RegEnumKeyExW";
    uint32_t acc, idx = (uint32_t)R[2];
    uint64_t pname = R[8], pcch = R[9], pclass = fle_arg(6), pcclass = fle_arg(7), pft = fle_arg(8);
    reg_key *k, *c;

    if (fle_arg(5) != 0) fle_stop(api, "lpReserved is not NULL");
    if (pname == 0 || pcch == 0) fle_stop(api, "lpName or lpcchName is NULL");
    if (pclass != 0 && pcclass == 0) fle_stop(api, "lpClass without lpcchClass");
    k = key_arg(api, R[1], &acc);
    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_ENUMERATE_SUB_KEYS)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (idx >= k->nkids) {
        ret(FLE_W32_ERR_NO_MORE_ITEMS);
        return;
    }
    c = k->kids[idx];
    if ((uint32_t)fle_ld(pcch, 4) < c->nlen + 1) {
        ret(FLE_W32_ERR_MORE_DATA);
        return;
    }
    fle_w_write(pname, c->name, c->nlen + 1);
    fle_st(pcch, 4, (uint32_t)c->nlen);
    if (pclass != 0) {
        if ((uint32_t)fle_ld(pcclass, 4) < 1) {
            ret(FLE_W32_ERR_MORE_DATA);
            return;
        }
        fle_st(pclass, 2, 0);
    }
    if (pcclass != 0) fle_st(pcclass, 4, 0);
    put_filetime(pft, c->wt);
    ret(0);
}

/* advapi32!RegEnumValueW (MSDN: RegEnumValueW). hKey rcx (KEY_QUERY_VALUE, else 5), dwIndex edx, lpValueName
 * r8, lpcchValueName r9, lpReserved [5] (NULL), lpType [6], lpData [7], lpcbData [8]. Values come in creation
 * order. An index past the last value returns 259. A value name that does not fit (size includes the NUL)
 * returns 234 (the page does not say what *lpcchValueName holds: left alone); data that does not fit returns
 * 234 with the required size in *lpcbData (page). On success *lpcchValueName is the length without the NUL. */
void fle_imp_RegEnumValueW(void)
{
    static const char api[] = "RegEnumValueW";
    uint32_t acc, idx = (uint32_t)R[2];
    uint64_t pname = R[8], pcch = R[9], ptype = fle_arg(6), pdata = fle_arg(7), pcb = fle_arg(8);
    reg_key *k;
    reg_value *v;
    uint32_t st;

    if (fle_arg(5) != 0) fle_stop(api, "lpReserved is not NULL");
    if (pname == 0 || pcch == 0) fle_stop(api, "lpValueName or lpcchValueName is NULL");
    k = key_arg(api, R[1], &acc);
    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_QUERY_VALUE)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (idx >= k->nvals) {
        ret(FLE_W32_ERR_NO_MORE_ITEMS);
        return;
    }
    v = &k->vals[idx];
    if ((uint32_t)fle_ld(pcch, 4) < v->nlen + 1) {
        ret(FLE_W32_ERR_MORE_DATA);
        return;
    }
    st = data_out(api, v, ptype, pdata, pcb);
    if (st) {
        ret(st);
        return;
    }
    fle_w_write(pname, v->name, v->nlen + 1);
    fle_st(pcch, 4, (uint32_t)v->nlen);
    ret(0);
}

/* advapi32!RegQueryInfoKeyW (MSDN: RegQueryInfoKeyW). hKey rcx (KEY_QUERY_VALUE, else 5), lpClass rdx, lpcchClass
 * r8, lpReserved r9 (NULL), lpcSubKeys [5], lpcbMaxSubKeyLen [6], lpcbMaxClassLen [7], lpcValues [8],
 * lpcbMaxValueNameLen [9], lpcbMaxValueLen [10], lpcbSecurityDescriptor [11], lpftLastWriteTime [12]. Every
 * requested count is computed from the tree; names are in characters without the NUL, value data in bytes.
 * Keys have no class (""; class counts 0). lpClass given without lpcchClass returns ERROR_INVALID_PARAMETER
 * (87, page). lpcbSecurityDescriptor non-NULL stops: security descriptors are not modelled. */
void fle_imp_RegQueryInfoKeyW(void)
{
    static const char api[] = "RegQueryInfoKeyW";
    uint32_t acc, maxsub = 0, maxval_name = 0, maxval = 0;
    uint64_t pclass = R[2], pcclass = R[8];
    reg_key *k;
    size_t i;

    if (R[9] != 0) fle_stop(api, "lpReserved is not NULL");
    if (fle_arg(11) != 0) fle_stop(api, "lpcbSecurityDescriptor is not NULL (security descriptors are not modelled)");
    k = key_arg(api, R[1], &acc);
    if (!k) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    if (!(acc & FLE_KEY_QUERY_VALUE)) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (pclass != 0 && pcclass == 0) {
        ret(FLE_W32_ERR_INVALID_PARAMETER);
        return;
    }
    if (pclass != 0) {
        if ((uint32_t)fle_ld(pcclass, 4) < 1) {
            ret(FLE_W32_ERR_MORE_DATA);
            return;
        }
        fle_st(pclass, 2, 0);
    }
    if (pcclass != 0) fle_st(pcclass, 4, 0);
    for (i = 0; i < k->nkids; i++)
        if (k->kids[i]->nlen > maxsub) maxsub = (uint32_t)k->kids[i]->nlen;
    for (i = 0; i < k->nvals; i++) {
        if (k->vals[i].nlen > maxval_name) maxval_name = (uint32_t)k->vals[i].nlen;
        if (k->vals[i].size > maxval) maxval = k->vals[i].size;
    }
    if (fle_arg(5)) fle_st(fle_arg(5), 4, (uint32_t)k->nkids);
    if (fle_arg(6)) fle_st(fle_arg(6), 4, maxsub);
    if (fle_arg(7)) fle_st(fle_arg(7), 4, 0);
    if (fle_arg(8)) fle_st(fle_arg(8), 4, (uint32_t)k->nvals);
    if (fle_arg(9)) fle_st(fle_arg(9), 4, maxval_name);
    if (fle_arg(10)) fle_st(fle_arg(10), 4, maxval);
    put_filetime(fle_arg(12), k->wt);
    ret(0);
}

/* advapi32!RegDeleteKeyW (MSDN: RegDeleteKeyW). hKey rcx (access rights do not matter, page), lpSubKey rdx
 * (NULL stops, page: cannot be NULL; an empty string stops, the page does not define it). The key must exist
 * (2: inference) and have no subkeys: the page says it cannot, without a code, so ERROR_ACCESS_DENIED (5) is
 * an inference. The page says a deleted key goes away only when its last handle closes: deleting a key that
 * has an open handle is not implemented and stops. The path may have several components. */
void fle_imp_RegDeleteKeyW(void)
{
    static const char api[] = "RegDeleteKeyW";
    uint32_t acc;
    reg_key *base, *cur;
    uint16_t *s;
    size_t len, i;
    kpath kp;

    if (R[2] == 0) fle_stop(api, "lpSubKey is NULL (the page says it cannot be)");
    s = fle_w_read_alloc(api, R[2], KEY_NAME_MAX * 512u, &len);
    if (len == 0) fle_stop(api, "empty lpSubKey is not defined");
    base = key_arg(api, R[1], &acc);
    if (!base) {
        free(s);
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    split_key(api, s, len, &kp, 512, 0);
    cur = base;
    for (i = 0; i < kp.n && cur; i++) cur = kid_find(cur, kp.p[i], kp.l[i]);
    free(s);
    if (!cur) {
        ret(FLE_W32_ERR_FILE_NOT_FOUND);
        return;
    }
    if (cur->nkids) {
        ret(FLE_W32_ERR_ACCESS_DENIED);
        return;
    }
    if (cur->opens) fle_stop(api, "deleting a key that still has open handles is not implemented");
    {
        reg_key *p = cur->parent;
        size_t j;
        for (j = 0; j < p->nkids; j++) {
            if (p->kids[j] == cur) {
                memmove(&p->kids[j], &p->kids[j + 1], (p->nkids - j - 1) * sizeof p->kids[0]);
                p->nkids--;
                break;
            }
        }
        p->wt = rtick();
        for (j = 0; j < cur->nvals; j++) {
            free(cur->vals[j].name);
            free(cur->vals[j].data);
        }
        free(cur->vals);
        free(cur->kids);
        free(cur->name);
        free(cur);
    }
    ret(0);
}

/* advapi32!RegFlushKey (MSDN: RegFlushKey). Nothing is written anywhere: a valid handle returns 0. An invalid
 * or closed handle returns 6 (inference). */
void fle_imp_RegFlushKey(void)
{
    static const char api[] = "RegFlushKey";
    uint32_t acc;
    if (!key_arg(api, R[1], &acc)) {
        ret(FLE_W32_ERR_INVALID_HANDLE);
        return;
    }
    ret(0);
}
