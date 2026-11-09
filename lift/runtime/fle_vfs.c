/* In-memory virtual file system (docs/decisions/0018-virtual-fs-reg-env.md; sources and the list of what is
 * implemented in docs/investigations/import-vfs-spec.md). The host file system is never touched. One volume,
 * "C:", exists; it starts empty and only what the guest creates is there. Anything the Microsoft pages leave
 * undefined, and anything this runtime does not implement, stops the run (fle_stop) instead of succeeding. */
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_realmod.h"
#include "fle_vfs.h"

#define ATTR_READONLY  0x1u
#define ATTR_HIDDEN    0x2u
#define ATTR_SYSTEM    0x4u
#define ATTR_DIRECTORY 0x10u
#define ATTR_ARCHIVE   0x20u
#define ATTR_NORMAL    0x80u
#define ATTR_TEMPORARY 0x100u
#define ATTR_OFFLINE   0x1000u
#define ATTR_NOT_INDEXED 0x2000u
/* Attribute bits SetFileAttributesW lists as supported (MSDN: SetFileAttributesW), without NORMAL. */
#define ATTR_SETTABLE (ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_ARCHIVE | ATTR_TEMPORARY | ATTR_OFFLINE | ATTR_NOT_INDEXED)

#define GENERIC_READ_   0x80000000u
#define GENERIC_WRITE_  0x40000000u
#define GENERIC_EXEC_   0x20000000u
#define GENERIC_ALL_    0x10000000u
#define ACC_DELETE      0x00010000u
#define ACC_READ_CONTROL 0x00020000u
#define ACC_SYNCHRONIZE 0x00100000u
#define ACC_FILE_READ_DATA   0x1u
#define ACC_FILE_WRITE_DATA  0x2u
#define ACC_FILE_APPEND_DATA 0x4u
#define ACC_FILE_SPECIFIC    0x1FFu /* FILE_READ_DATA .. FILE_WRITE_ATTRIBUTES, bits 0..8 */
#define ACC_FILE_WRITE_ATTRIBUTES 0x100u

#define FLAG_BACKUP_SEMANTICS 0x02000000u
#define FLAG_DELETE_ON_CLOSE  0x04000000u
#define FLAG_SEQUENTIAL_SCAN  0x08000000u
#define FLAG_RANDOM_ACCESS    0x10000000u
#define FLAG_OPEN_REPARSE     0x00200000u
#define FLAG_WRITE_THROUGH    0x80000000u
/* Decision 0129 (wall 4): FILE_FLAG_OPEN_NO_RECALL 0x00100000 (Learn CreateFileW: "The file data is requested, but it should continue to be located in remote storage. It should not be transported back to local
 * storage. This flag is for use by remote storage systems."). The virtual volume has no remote storage, so there is nothing to recall: accepted with no effect (S-open-no-recall, a reading of "for use by remote
 * storage systems"). The guest passes it with FILE_ATTRIBUTE_NORMAL (0x00100080) when it opens a file for reading (f_108e340). */
#define FLAG_OPEN_NO_RECALL   0x00100000u

#define MAX_FILE_BYTES (UINT64_C(256) << 20)
#define MAX_PATH_UNITS 259u
#define MAX_COMPONENT  255u
#define MAXC 640u

/* ---- nodes ---- */

typedef struct fle_node {
    uint16_t *name;
    size_t nlen;
    struct fle_node *parent;
    int is_dir;
    uint32_t bits;               /* attribute bits without DIRECTORY and NORMAL */
    uint8_t *data;
    uint64_t size, cap;
    uint64_t ct, at, wt;
    struct fle_node **kids;
    size_t nkids, kcap;
    unsigned opens;
    int pending;                 /* deleted when the last handle closes */
} fle_node;

static fle_node root_node = { .is_dir = 1, .ct = FLE_VFS_CLOCK_START, .at = FLE_VFS_CLOCK_START, .wt = FLE_VFS_CLOCK_START };
static fle_node *cwd_node = &root_node;
static uint64_t clock_now = FLE_VFS_CLOCK_START;

static uint64_t tick(void)
{
    clock_now += UINT64_C(10000000);
    return clock_now;
}

static void fail(uint32_t err, uint64_t ret)
{
    fle_set_last_error(err);
    R[0] = ret;
}

static uint32_t node_attrs(const fle_node *n)
{
    uint32_t a = n->bits | (n->is_dir ? ATTR_DIRECTORY : 0u);
    return a ? a : ATTR_NORMAL;
}

static fle_node *find_kid(const fle_node *d, const uint16_t *c, size_t cl)
{
    size_t i;
    for (i = 0; i < d->nkids; i++)
        if (fle_w_icmp(d->kids[i]->name, d->kids[i]->nlen, c, cl) == 0) return d->kids[i];
    return NULL;
}

static void attach(fle_node *par, fle_node *n)
{
    if (par->nkids == par->kcap) {
        size_t nc = par->kcap ? par->kcap * 2 : 8;
        fle_node **nk = realloc(par->kids, nc * sizeof *nk);
        if (!nk) fle_stop("vfs", "out of memory");
        par->kids = nk;
        par->kcap = nc;
    }
    par->kids[par->nkids++] = n;
    n->parent = par;
}

static void detach(fle_node *n)
{
    fle_node *p = n->parent;
    size_t i;
    for (i = 0; i < p->nkids; i++) {
        if (p->kids[i] == n) {
            memmove(&p->kids[i], &p->kids[i + 1], (p->nkids - i - 1) * sizeof p->kids[0]);
            p->nkids--;
            break;
        }
    }
    n->parent = NULL;
}

static void set_name(fle_node *n, const uint16_t *name, size_t nl)
{
    uint16_t *c = malloc((nl + 1) * sizeof *c);
    if (!c) fle_stop("vfs", "out of memory");
    memcpy(c, name, nl * sizeof *c);
    c[nl] = 0;
    free(n->name);
    n->name = c;
    n->nlen = nl;
}

static fle_node *node_new(fle_node *par, const uint16_t *name, size_t nl, int dir, uint32_t bits)
{
    fle_node *n = calloc(1, sizeof *n);
    if (!n) fle_stop("vfs", "out of memory");
    set_name(n, name, nl);
    n->is_dir = dir;
    n->bits = bits;
    n->ct = n->at = n->wt = tick();
    attach(par, n);
    return n;
}

static void node_free(fle_node *n)
{
    detach(n);
    free(n->data);
    free(n->name);
    free(n->kids);
    free(n);
}

static void node_reserve(const char *api, fle_node *n, uint64_t need)
{
    if (need > MAX_FILE_BYTES) fle_stop(api, "file larger than the 256 MiB virtual limit");
    if (need > n->cap) {
        uint64_t nc = n->cap * 2 > need ? n->cap * 2 : need;
        uint8_t *nd;
        if (nc < 4096) nc = 4096;
        if (nc > MAX_FILE_BYTES) nc = MAX_FILE_BYTES;
        nd = realloc(n->data, nc);
        if (!nd) fle_stop(api, "out of memory");
        memset(nd + n->cap, 0, nc - n->cap);
        n->data = nd;
        n->cap = nc;
    }
}

static void node_truncate(fle_node *n)
{
    free(n->data);
    n->data = NULL;
    n->size = n->cap = 0;
}

static int is_ancestor_or_self(const fle_node *a, const fle_node *b)
{
    for (; b; b = b->parent)
        if (b == a) return 1;
    return 0;
}

static int tree_has_open(const fle_node *n)
{
    size_t i;
    if (n->opens) return 1;
    for (i = 0; i < n->nkids; i++)
        if (tree_has_open(n->kids[i])) return 1;
    return 0;
}

/* "C:\a\b" for a node ("C:\" for the root). malloc'd. */
static uint16_t *node_path(const fle_node *n, size_t *len)
{
    const fle_node *chain[MAXC];
    size_t depth = 0, i, total = 2, o = 0;
    uint16_t *s;
    for (; n->parent; n = n->parent) {
        if (depth >= MAXC) fle_stop("vfs", "directory tree too deep");
        chain[depth++] = n;
    }
    for (i = 0; i < depth; i++) total += 1 + chain[i]->nlen;
    if (depth == 0) total = 3;
    s = malloc((total + 1) * sizeof *s);
    if (!s) fle_stop("vfs", "out of memory");
    s[o++] = 'C';
    s[o++] = ':';
    if (depth == 0) s[o++] = '\\';
    for (i = depth; i-- > 0;) {
        s[o++] = '\\';
        memcpy(s + o, chain[i]->name, chain[i]->nlen * sizeof *s);
        o += chain[i]->nlen;
    }
    s[o] = 0;
    *len = o;
    return s;
}

/* ---- paths ---- */

typedef struct {
    const uint16_t *p[MAXC];
    size_t l[MAXC];
    size_t n;
    int trail;       /* the text ends with a separator */
    int wild;        /* the last component has * or ? */
    uint16_t *own;
} vpath;

static int is_sep(uint16_t c) { return c == '\\' || c == '/'; }

static void vp_push(const char *api, vpath *v, const uint16_t *p, size_t l)
{
    if (v->n >= MAXC) fle_stop(api, "path has too many components");
    v->p[v->n] = p;
    v->l[v->n] = l;
    v->n++;
}

static void vp_free(vpath *v)
{
    free(v->own);
    v->own = NULL;
}

static int is_device(const uint16_t *c, size_t cl)
{
    char b[5];
    size_t n = 0, i;
    while (n < cl && c[n] != '.') n++;
    while (n > 0 && c[n - 1] == ' ') n--;
    if (n != 3 && n != 4) return 0;
    for (i = 0; i < n; i++) {
        uint16_t f = fle_w_fold(c[i]);
        if (i == 3 && (c[i] == 0xB9 || c[i] == 0xB2 || c[i] == 0xB3)) {
            b[i] = '1';
        } else if (f > 0x7f) {
            return 0;
        } else {
            b[i] = (char)(f >= 'a' && f <= 'z' ? f - 32 : f);
        }
    }
    b[n] = 0;
    if (n == 3) return !strcmp(b, "CON") || !strcmp(b, "PRN") || !strcmp(b, "AUX") || !strcmp(b, "NUL");
    return (!strncmp(b, "COM", 3) || !strncmp(b, "LPT", 3)) && b[3] >= '1' && b[3] <= '9';
}

/* Naming rules from MSDN (Naming Files, Paths, and Namespaces). A name that breaks them is not given an error
 * code by the pages, so the run stops. */
static void check_component(const char *api, vpath *v, const uint16_t *c, size_t cl, int wild_ok)
{
    size_t k;
    if (cl > MAX_COMPONENT) fle_stop(api, "path component longer than 255 characters");
    for (k = 0; k < cl; k++) {
        uint16_t ch = c[k];
        if (ch < 32 || ch == '<' || ch == '>' || ch == '"' || ch == '|' || ch == ':')
            fle_stop(api, "reserved character in a path component (streams are not implemented)");
        if (ch == '*' || ch == '?') {
            if (!wild_ok) fle_stop(api, "wildcard in a path that does not allow one");
            v->wild = 1;
        }
    }
    if (c[cl - 1] == ' ' || c[cl - 1] == '.') fle_stop(api, "path component ends with a space or a period");
    if (is_device(c, cl)) fle_stop(api, "reserved device name in a path (devices are not implemented)");
}

static void parse_path(const char *api, uint64_t guest, int wild_last_ok, vpath *v)
{
    size_t len, i;
    uint16_t *s;

    v->n = 0;
    v->trail = 0;
    v->wild = 0;
    v->own = NULL;
    s = fle_w_read_alloc(api, guest, MAX_PATH_UNITS, &len);
    v->own = s;
    if (len == 0) fle_stop(api, "empty path is not defined");
    if (len >= 4 && is_sep(s[0]) && is_sep(s[1]) && (s[2] == '?' || s[2] == '.') && is_sep(s[3]))
        fle_stop(api, "the \\\\?\\ and \\\\.\\ prefixes are not implemented");
    i = 0;
    if (len >= 2 && s[1] == ':') {
        if (fle_w_fold(s[0]) != 'c') fle_stop(api, "only drive C: exists");
        i = 2;
        if (i >= len || !is_sep(s[i])) fle_stop(api, "drive-relative path (C:name) is not implemented");
    } else if (is_sep(s[0])) {
        if (len >= 2 && is_sep(s[1])) fle_stop(api, "UNC path is not implemented");
    } else {
        const fle_node *chain[MAXC];
        size_t depth = 0, k;
        const fle_node *n;
        for (n = cwd_node; n->parent; n = n->parent) chain[depth++] = n;
        for (k = depth; k-- > 0;) vp_push(api, v, chain[k]->name, chain[k]->nlen);
    }
    v->trail = is_sep(s[len - 1]);
    while (i < len) {
        size_t j = i, cl;
        while (j < len && !is_sep(s[j])) j++;
        cl = j - i;
        if (cl > 0) {
            const uint16_t *c = s + i;
            if (cl == 1 && c[0] == '.') {
                /* current directory */
            } else if (cl == 2 && c[0] == '.' && c[1] == '.') {
                if (v->n > 0) v->n--;
            } else {
                check_component(api, v, c, cl, wild_last_ok && j == len);
                vp_push(api, v, c, cl);
            }
        }
        i = j + 1;
    }
}

/* 0 found; 2 the last component is missing (its directory exists); 3 an earlier component is missing or is a
 * file (ERROR_PATH_NOT_FOUND). *parent is the directory holding the last component (NULL for the root). */
static uint32_t lookup(const char *api, const vpath *v, fle_node **node, fle_node **parent)
{
    fle_node *cur = &root_node;
    size_t i;
    *parent = NULL;
    *node = NULL;
    if (v->n == 0) {
        *node = &root_node;
        return 0;
    }
    for (i = 0; i < v->n; i++) {
        fle_node *k;
        if (!cur->is_dir) return 3;
        k = find_kid(cur, v->p[i], v->l[i]);
        if (i + 1 == v->n) {
            *parent = cur;
            *node = k;
            if (k && v->trail && !k->is_dir) fle_stop(api, "a trailing separator on a file name is not defined");
            return k ? 0u : 2u;
        }
        if (!k) return 3;
        cur = k;
    }
    return 3;
}

/* ---- file handles ---- */

typedef struct {
    int used;
    fle_node *node;
    uint64_t pos;
    unsigned rd, wr, app, del, wattr;
    unsigned share;
    int delete_on_close;
} fh_ent;

static fh_ent *fh;
static size_t fh_n, fh_cap;

static int in_region(uint64_t h, uint64_t base)
{
    return h > base && h < base + FLE_HANDLE_REGION_SIZE;
}

static uint64_t hnorm(uint64_t h)
{
    uint64_t c, off, idx;
    if (h == 0 || (h >> 32) != 0) return h;
    c = (FLE_VFS_FILE_BASE & ~UINT64_C(0xFFFFFFFF)) | h;
    if (!in_region(c, FLE_VFS_FILE_BASE)) return h;
    off = c - FLE_VFS_FILE_BASE;
    if (off % 4) return h;
    idx = off / 4 - 1;
    return idx < fh_n && fh[idx].used ? c : h;
}

/* Decision 0178 (PROVISIONAL, S-dlg1-handle32): "64-bit versions of Windows use 32-bit handles for interoperability. When sharing a handle between 32-bit and 64-bit applications, only the lower 32 bits are
 * significant, so it is safe to truncate the handle ... or sign-extend the handle" (Learn, Interprocess communication; file handles are among the named objects of that page). The engine's zip reader keeps
 * the file handle of CreateFileW in a 32-bit field and passes the zero-extended low half to ReadFile (decision 0178, the .zip route of File -> Open...). The file handles of this volume are 0x7FFA0000xxxx: a
 * value below 2^32 that, with the high half of that region, is a LIVE file handle IS that handle. Every other value (a stranger below 2^32, a closed handle, a find handle) is as before. */
static uint64_t hnorm(uint64_t h);

/* -1 invalid (NULL, INVALID_HANDLE_VALUE), 0 not ours, 1 file region, 2 find region */
static int h_kind(uint64_t h)
{
    h = hnorm(h);
    if (h == 0 || h == FLE_INVALID_HANDLE_VALUE) return -1;
    if (in_region(h, FLE_VFS_FILE_BASE)) return 1;
    if (in_region(h, FLE_VFS_FIND_BASE)) return 2;
    return 0;
}

static fh_ent *fh_get(uint64_t h)
{
    uint64_t off, idx;
    h = hnorm(h);
    if (!in_region(h, FLE_VFS_FILE_BASE)) return NULL;
    off = h - FLE_VFS_FILE_BASE;
    if (off % 4) return NULL;
    idx = off / 4 - 1;
    if (idx >= fh_n || !fh[idx].used) return NULL;
    return &fh[idx];
}

/* The file handle for an API, or NULL when the guest passed an invalid or closed handle (the caller then
 * fails with ERROR_INVALID_HANDLE). A handle that this runtime did not hand out stops the run. */
static fh_ent *file_arg(const char *api, uint64_t h)
{
    int k = h_kind(h);
    if (k == 0) fle_stop(api, "handle was not issued by the virtual file system");
    if (k != 1) return NULL;
    return fh_get(h);
}

static uint64_t fh_alloc(fle_node *n, unsigned rd, unsigned wr, unsigned app, unsigned del, unsigned wattr,
                         unsigned share, int doc)
{
    fh_ent *e;
    if (fh_n == fh_cap) {
        size_t nc = fh_cap ? fh_cap * 2 : 32;
        fh_ent *nf = realloc(fh, nc * sizeof *nf);
        if (!nf) fle_stop("CreateFileW", "out of memory");
        fh = nf;
        fh_cap = nc;
    }
    if (fh_n + 2 >= FLE_HANDLE_REGION_SIZE / 4) fle_stop("CreateFileW", "file handle region exhausted");
    e = &fh[fh_n];
    memset(e, 0, sizeof *e);
    e->used = 1;
    e->node = n;
    e->rd = rd;
    e->wr = wr;
    e->app = app;
    e->del = del;
    e->wattr = wattr;
    e->share = share;
    e->delete_on_close = doc;
    n->opens++;
    if (doc) n->pending = 1;
    fh_n++;
    return FLE_VFS_FILE_BASE + 4u * (uint64_t)fh_n;
}

static void fh_close(fh_ent *e)
{
    fle_node *n = e->node;
    e->used = 0;
    n->opens--;
    if (n->opens == 0 && n->pending) node_free(n);
}

/* The sharing rules of CreateFile (MSDN: CreateFileW, dwDesiredAccess and dwShareMode): a new open conflicts
 * with an existing handle when it asks for an access the existing handle's share mode does not allow, or when
 * an existing handle has an access the new share mode does not allow. */
static int share_conflict(const fle_node *n, unsigned rd, unsigned wr, unsigned del, unsigned share)
{
    size_t i;
    for (i = 0; i < fh_n; i++) {
        const fh_ent *e = &fh[i];
        if (!e->used || e->node != n) continue;
        if ((rd && !(e->share & 1u)) || (wr && !(e->share & 2u)) || (del && !(e->share & 4u))) return 1;
        if ((e->rd && !(share & 1u)) || ((e->wr || e->app) && !(share & 2u)) || (e->del && !(share & 4u))) return 1;
    }
    return 0;
}

/* kernel32!CreateFileW (MSDN: CreateFileW). Arguments: lpFileName rcx, dwDesiredAccess edx, dwShareMode r8d,
 * lpSecurityAttributes r9, dwCreationDisposition [5], dwFlagsAndAttributes [6], hTemplateFile [7]. Implemented:
 * the five dispositions, the sharing rules, FILE_FLAG_DELETE_ON_CLOSE, FILE_FLAG_BACKUP_SEMANTICS (accepted;
 * directory handles are not implemented), the caching hints (accepted, no effect). Stops: non-NULL security
 * attributes or template, access bits outside the file rights and DELETE/READ_CONTROL/SYNCHRONIZE, other flags
 * (overlapped, no-buffering, ...), an invalid disposition, opening a directory, a trailing separator. Failures:
 * INVALID_HANDLE_VALUE with 2 (missing file), 3 (missing directory), 80 (CREATE_NEW exists), 5 (delete pending
 * - MSDN DeleteFile; a directory without BACKUP_SEMANTICS and a read-only or hidden/system file - the page's
 * "is writable" and CREATE_ALWAYS text, error code for read-only is an inference), 32 (sharing violation).
 * Last error: 183 when CREATE_ALWAYS/OPEN_ALWAYS open an existing file, 0 when they create one; other
 * successes leave it alone. */
void fle_imp_CreateFileW(void)
{
    static const char api[] = "CreateFileW";
    uint32_t access = (uint32_t)R[2], share = (uint32_t)R[8];
    uint32_t disp = (uint32_t)fle_arg(5), fa = (uint32_t)fle_arg(6);
    unsigned rd, wr, app, del, wattr, doc;
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    int created = 0;
    uint64_t h;
    const uint32_t allowed_flags = FLAG_BACKUP_SEMANTICS | FLAG_DELETE_ON_CLOSE | FLAG_SEQUENTIAL_SCAN |
                                   FLAG_RANDOM_ACCESS | FLAG_OPEN_REPARSE | FLAG_WRITE_THROUGH | FLAG_OPEN_NO_RECALL;
    const uint32_t allowed_attrs = ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_ARCHIVE | ATTR_NORMAL | ATTR_TEMPORARY;

    /* Decision 0206 (dec1, PROVISIONAL): a SECURITY_ATTRIBUTES whose lpSecurityDescriptor is NULL is accepted (Learn SECURITY_ATTRIBUTES: NULL = 'the default security descriptor of the calling process'; bInheritHandle only matters
     * to a child process and there is none). It is what the MSVC C++ library passes (_wfsopen); every other non-NULL value still stops (a pointer below 64 KiB or not 8-byte aligned is not dereferenced). */
    if (R[9] != 0 && !(R[9] >= 0x10000 && (R[9] & 7) == 0 && fle_ld(R[9], 4) == 24 && fle_ld(R[9] + 8, 8) == 0)) fle_stop(api, "lpSecurityAttributes is not NULL and not a SECURITY_ATTRIBUTES with a NULL descriptor (not implemented)");
    if (fle_arg(7) != 0) fle_stop(api, "hTemplateFile is not NULL (not implemented)");
    if (share & ~7u) fle_stop(api, "dwShareMode has bits outside FILE_SHARE_READ/WRITE/DELETE");
    if (disp < 1 || disp > 5) fle_stop(api, "dwCreationDisposition is not one of the five documented values");
    if (access & ~(GENERIC_READ_ | GENERIC_WRITE_ | GENERIC_ALL_ | ACC_DELETE | ACC_READ_CONTROL | ACC_SYNCHRONIZE |
                   ACC_FILE_SPECIFIC))
        fle_stop(api, "dwDesiredAccess has bits that are not implemented (GENERIC_EXECUTE, MAXIMUM_ALLOWED, ...)");
    if (fa & ~(allowed_flags | allowed_attrs))
        fle_stop(api, "dwFlagsAndAttributes has flags or attributes that are not implemented");

    rd = (access & (GENERIC_READ_ | GENERIC_ALL_ | ACC_FILE_READ_DATA)) != 0;
    wr = (access & (GENERIC_WRITE_ | GENERIC_ALL_ | ACC_FILE_WRITE_DATA)) != 0;
    app = (access & ACC_FILE_APPEND_DATA) != 0 && !wr;
    del = (access & (ACC_DELETE | GENERIC_ALL_)) != 0;
    wattr = (access & (GENERIC_WRITE_ | GENERIC_ALL_ | ACC_FILE_WRITE_ATTRIBUTES)) != 0;
    doc = (fa & FLAG_DELETE_ON_CLOSE) != 0;
    if (doc) del = 1;

    parse_path(api, R[1], 0, &v);
    if (v.trail) fle_stop(api, "a trailing separator in a CreateFile name is not defined");
    st = lookup(api, &v, &n, &par);
    if (st == 3) {
        vp_free(&v);
        fail(3, FLE_INVALID_HANDLE_VALUE);
        return;
    }
    if (st == 2) {
        if (disp == 3 || disp == 5) {
            vp_free(&v);
            fail(2, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (disp == 5 && !wr) fle_stop(api, "TRUNCATE_EXISTING without GENERIC_WRITE");
        n = node_new(par, v.p[v.n - 1], v.l[v.n - 1], 0, fa & (ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_ARCHIVE | ATTR_TEMPORARY));
        created = 1;
    } else {
        if (n->pending) {
            vp_free(&v);
            fail(5, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (disp == 1) {
            vp_free(&v);
            fail(80, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (n->is_dir) {
            if (!(fa & FLAG_BACKUP_SEMANTICS)) {
                vp_free(&v);
                fail(5, FLE_INVALID_HANDLE_VALUE);
                return;
            }
            fle_stop(api, "opening a directory handle is not implemented");
        }
        if (disp == 5 && !wr) fle_stop(api, "TRUNCATE_EXISTING without GENERIC_WRITE");
        if ((wr || app || del || disp == 2 || disp == 5) && (n->bits & ATTR_READONLY)) {
            vp_free(&v);
            fail(5, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (disp == 2 && (fa & ATTR_NORMAL) && (n->bits & (ATTR_HIDDEN | ATTR_SYSTEM))) {
            vp_free(&v);
            fail(5, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (share_conflict(n, rd, wr || app, del, share)) {
            vp_free(&v);
            fail(32, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (disp == 2 || disp == 5) {
            node_truncate(n);
            n->wt = n->at = tick();
        }
    }
    vp_free(&v);
    h = fh_alloc(n, rd, wr, app, del, wattr, share, (int)doc);
    if (created) {
        if (disp == 2 || disp == 4) fle_set_last_error(0);
    } else if (disp == 2 || disp == 4) {
        fle_set_last_error(183);
    }
    R[0] = h;
}

/* kernel32!ReadFile (MSDN: ReadFile). hFile rcx, lpBuffer rdx, nNumberOfBytesToRead r8d, lpNumberOfBytesRead
 * r9, lpOverlapped [5]. Synchronous only: a non-NULL lpOverlapped, a NULL lpNumberOfBytesRead and a zero-byte
 * read stop the run (the page leaves them open or they are asynchronous). The byte count is set to zero first
 * (page). A read at or beyond the end fails with ERROR_HANDLE_EOF (38), one that crosses the end succeeds with
 * the bytes before it (both from the page). A handle without read access fails with 5 (the page requires read
 * access; the code is an inference); an invalid or closed handle fails with 6 (inference). */
void fle_imp_ReadFile(void)
{
    static const char api[] = "ReadFile";
    uint64_t buf = R[2], pr = R[9];
    uint32_t n = (uint32_t)R[8];
    fh_ent *e;
    uint64_t avail, k;

    if (fle_arg(5) != 0) fle_stop(api, "lpOverlapped is not NULL (asynchronous I/O is not implemented)");
    if (pr == 0) fle_stop(api, "lpNumberOfBytesRead is NULL");
    fle_st(pr, 4, 0);
    e = file_arg(api, R[1]);
    if (!e) {
        fail(6, 0);
        return;
    }
    if (!e->rd) {
        fail(5, 0);
        return;
    }
    /* Decision 0126 (PROVISIONAL), wall 95, S-readfile-zero (a READING, not a statement of the page): the page does not define nNumberOfBytesToRead = 0. It gives two rules for "a read operation": "begins at or beyond the end of the
     * file ... fails with the error ERROR_HANDLE_EOF" and "begins before the end of the file ... succeeds, and the number of bytes read is the number of bytes that were read before the end of file was reached". They are applied
     * to a zero-byte read as they stand: at or beyond the end it fails with 38, before the end it succeeds with 0 bytes read; the buffer is not touched, so a NULL lpBuffer is not examined for n = 0. A real Windows may answer a zero-byte
     * read at the end of the file with success; withdrawn when a measurement says so. */
    if (n != 0 && buf == 0) fle_stop(api, "lpBuffer is NULL");
    if (e->pos >= e->node->size) {
        /* Decision 0206 (dec1, PROVISIONAL; always on since decision 0243). Microsoft Learn, 'Testing for the End of a File'
         * (https://learn.microsoft.com/en-us/windows/win32/FileIO/testing-for-the-end-of-a-file): 'When a synchronous read operation gets to the end of a file, ReadFile returns TRUE and sets the variable pointed to by
         * the lpNumberOfBytesRead parameter to zero.' The ReadFile page's Remarks say 'If a read operation on a file begins at or beyond the end of the file, then the read operation fails with the error ERROR_HANDLE_EOF'
         * (the reading above); the two pages disagree for synchronous handles. The real oggio_x64.dll under Wine 10.0 reads FLAC files to the end (the MSVC CRT's _read turns a FALSE return into an error and libFLAC stops
         * before the last blocks). The byte count is already zero; the last error is left alone. A zero-byte read (n == 0) keeps S-readfile-zero. */
        if (n != 0) {
            R[0] = 1;
            return;
        }
        fail(38, 0);
        return;
    }
    if (n == 0) {
        R[0] = 1;
        return;
    }
    avail = e->node->size - e->pos;
    k = n < avail ? n : avail;
    memcpy((void *)fle_addr(buf, (unsigned)k, 1), e->node->data + e->pos, (size_t)k);
    e->pos += k;
    fle_st(pr, 4, k);
    R[0] = 1;
}

/* kernel32!WriteFile (MSDN: WriteFile). Same argument layout as ReadFile. The byte count is zeroed first. A
 * handle without write access fails with 5 (inference). A zero-byte write succeeds and changes nothing (page:
 * does not truncate or extend). A write past the end zero-fills the gap (MSDN: SetFilePointer). A handle with
 * only FILE_APPEND_DATA writes at the end (MSDN: File Access Rights Constants). Files are limited to 256 MiB
 * (a limit of this runtime; larger stops). */
void fle_imp_WriteFile(void)
{
    static const char api[] = "WriteFile";
    uint64_t buf = R[2], pw = R[9];
    uint32_t n = (uint32_t)R[8];
    fh_ent *e;
    uint64_t at;

    if (fle_arg(5) != 0) fle_stop(api, "lpOverlapped is not NULL (asynchronous I/O is not implemented)");
    if (pw == 0) fle_stop(api, "lpNumberOfBytesWritten is NULL");
    fle_st(pw, 4, 0);
    e = file_arg(api, R[1]);
    if (!e) {
        fail(6, 0);
        return;
    }
    if (!e->wr && !e->app) {
        fail(5, 0);
        return;
    }
    if (n == 0) {
        R[0] = 1;
        return;
    }
    if (buf == 0) fle_stop(api, "lpBuffer is NULL");
    at = e->app ? e->node->size : e->pos;
    if (at + n < at) fle_stop(api, "file position overflow");
    node_reserve(api, e->node, at + n);
    memcpy(e->node->data + at, (const void *)fle_addr(buf, (unsigned)n, 0), n);
    if (at + n > e->node->size) e->node->size = at + n;
    e->pos = at + n;
    e->node->wt = e->node->at = tick();
    fle_st(pw, 4, n);
    R[0] = 1;
}

/* kernel32!CloseHandle (MSDN: CloseHandle). A file handle is closed; the last close of a delete-pending file
 * removes it. NULL, INVALID_HANDLE_VALUE, a closed handle and a find handle fail with 0 and ERROR_INVALID_HANDLE
 * (6; the page names no code - inference). Any other value stops: only file handles exist in this runtime. */
static int (*close_hooks[8])(uint64_t h);   /* decision 0136: 4 -> 8 (fle_token.c is the fifth) */
static unsigned n_close_hooks;

void fle_vfs_add_close_hook(int (*hook)(uint64_t h))
{
    unsigned i;
    for (i = 0; i < n_close_hooks; i++)
        if (close_hooks[i] == hook) return;
    if (n_close_hooks == sizeof close_hooks / sizeof close_hooks[0]) fle_stop("CloseHandle", "too many handle hooks");
    close_hooks[n_close_hooks++] = hook;
}

int fle_vfs_file_info(const char *api, uint64_t h, const uint8_t **data, uint64_t *size, unsigned *can_read, unsigned *can_write)
{
    fh_ent *e = file_arg(api, h);
    if (!e) return 0;
    *data = e->node->data;
    *size = e->node->size;
    *can_read = e->rd;
    *can_write = e->wr;
    return 1;
}

/* Decision 0178 (PROVISIONAL): what GetFileInformationByHandle and SetEndOfFile (fle_wall_dlg1.c) need of an open file. Additive: no existing function changes. */
int fle_vfs_file_stat(const char *api, uint64_t h, const void **node, uint32_t *attrs, uint64_t *ct, uint64_t *at, uint64_t *wt, uint64_t *size)
{
    fh_ent *e = file_arg(api, h);
    if (!e) return 0;
    *node = e->node;
    *attrs = node_attrs(e->node);
    *ct = e->node->ct;
    *at = e->node->at;
    *wt = e->node->wt;
    *size = e->node->size;
    return 1;
}

/* Sets the size of the file to the file pointer (zero-filled when it grows). Returns 0 for an invalid handle, -1 when the handle was not opened for writing, 1 on success. */
int fle_vfs_file_set_end(const char *api, uint64_t h)
{
    fh_ent *e = file_arg(api, h);
    if (!e) return 0;
    if (!e->wr && !e->app) return -1;
    if (e->pos > e->node->size) {
        node_reserve(api, e->node, e->pos);   /* the new area is zero (node_reserve) */
    } else if (e->node->data != NULL && e->pos < e->node->size) {
        memset(e->node->data + e->pos, 0, e->node->size - e->pos);   /* a later growth must read zeros, not the cut bytes */
    }
    e->node->size = e->pos;
    e->node->wt = e->node->at = tick();
    return 1;
}

void fle_imp_CloseHandle(void)
{
    static const char api[] = "CloseHandle";
    unsigned i;
    fh_ent *e;
    for (i = 0; i < n_close_hooks; i++)
        if (close_hooks[i](R[1])) return;
    e = file_arg(api, R[1]);
    if (!e) {
        fail(6, 0);
        return;
    }
    fh_close(e);
    R[0] = 1;
}

/* kernel32!SetFilePointer (MSDN: SetFilePointer). hFile rcx, lDistanceToMove edx (signed), lpDistanceToMoveHigh
 * r8, dwMoveMethod r9d. The distance is 64 bits when the high pointer is non-NULL, else the signed 32-bit low
 * part. A negative result fails with ERROR_NEGATIVE_SEEK (131), the pointer stays, the return value is
 * INVALID_SET_FILE_POINTER. A result that needs more than 32 bits without a high pointer fails the same way
 * (the page gives no code; 87 is an inference). A result of exactly 0xFFFFFFFF stops (the page says it is a valid
 * low part but ambiguous with failure). Success with a high pointer writes the high part and sets the last
 * error to 0 (the page's example reads it to tell success). Positions past the end are allowed. A method outside
 * 0..2 stops. */
void fle_imp_SetFilePointer(void)
{
    static const char api[] = "SetFilePointer";
    uint64_t ph = R[8];
    uint32_t method = (uint32_t)R[9];
    int64_t dist, base, np;
    fh_ent *e;

    e = file_arg(api, R[1]);
    if (!e) {
        fail(6, FLE_INVALID_SET_FILE_POINTER);
        return;
    }
    if (method > 2) fle_stop(api, "dwMoveMethod is not FILE_BEGIN, FILE_CURRENT or FILE_END");
    if (ph != 0) dist = (int64_t)(((uint64_t)(uint32_t)fle_ld(ph, 4) << 32) | (uint32_t)R[2]);
    else dist = (int32_t)(uint32_t)R[2];
    base = method == 0 ? 0 : method == 1 ? (int64_t)e->pos : (int64_t)e->node->size;
    np = base + dist;
    if (np < 0) {
        fail(131, FLE_INVALID_SET_FILE_POINTER);
        return;
    }
    if (ph == 0 && (uint64_t)np > 0xFFFFFFFFull) {
        fail(87, FLE_INVALID_SET_FILE_POINTER);
        return;
    }
    if (ph == 0 && (uint64_t)np == 0xFFFFFFFFull) fle_stop(api, "new position 0xFFFFFFFF is ambiguous with failure");
    e->pos = (uint64_t)np;
    if (ph != 0) {
        fle_st(ph, 4, (uint64_t)np >> 32);
        fle_set_last_error(0);
    }
    R[0] = (uint32_t)(uint64_t)np;
}

/* Decision 0238 (load2, PROVISIONAL): kernel32!SetFilePointerEx, BOOL f(HANDLE hFile rcx, LARGE_INTEGER liDistanceToMove rdx, PLARGE_INTEGER lpNewFilePointer r8, DWORD dwMoveMethod r9d).
 * It stopped by name (fle_vol.c, decision 0078); decision 0206 (dec1) answered it for the sample decoders only through an opt-in harness wrap, and the CRT of oggio_x64.dll / Elastique_x64.dll (_lseeki64, fseek)
 * reaches it in the engine's own build. Learn (SetFilePointerEx): "Moves the file pointer of the specified file"; "A positive value moves the pointer forward in the file and a negative value moves the file pointer backward";
 * FILE_BEGIN "the liDistanceToMove parameter is interpreted as an unsigned value"; FILE_CURRENT, FILE_END; lpNewFilePointer "If this parameter is NULL, the new file pointer is not returned"; "If the function succeeds, the return value is
 * nonzero. If the function fails, the return value is zero. To get extended error information, call GetLastError". As SetFilePointer above (the same file pointer): a position before the beginning fails with ERROR_NEGATIVE_SEEK (131; the
 * SetFilePointer page names it), the pointer stays, an invalid or closed handle fails with 6 (inference, as SetFilePointer); positions past the end are allowed. FILE_BEGIN with a negative 64-bit value (unsigned: above 2^63) and a method
 * outside 0..2 are not defined by the page and stop. Success leaves the last error alone. */
void fle_imp_SetFilePointerEx(void)
{
    static const char api[] = "SetFilePointerEx";
    uint64_t pnew = R[8];
    uint32_t method = (uint32_t)R[9];
    int64_t dist = (int64_t)R[2], base, np;
    fh_ent *e = file_arg(api, R[1]);

    if (!e) {
        fail(6, 0);
        return;
    }
    if (method > 2) fle_stop(api, "dwMoveMethod is not FILE_BEGIN, FILE_CURRENT or FILE_END");
    if (method == 0 && dist < 0) fle_stop(api, "FILE_BEGIN with liDistanceToMove above 2^63 (the page reads it as unsigned and names no limit)");
    base = method == 0 ? 0 : method == 1 ? (int64_t)e->pos : (int64_t)e->node->size;
    np = base + dist;
    if (np < 0) {
        fail(131, 0);
        return;
    }
    e->pos = (uint64_t)np;
    if (pnew != 0) fle_st(pnew, 8, (uint64_t)np);
    R[0] = 1;
}

/* Decision 0238 (load2, PROVISIONAL): kernel32!FlushFileBuffers, BOOL f(HANDLE hFile rcx) (stopped by name before; the opt-in harness wrap of decision 0206 answered nonzero for any open file; the CRT's fflush / _commit reach it).
 * Learn (FlushFileBuffers): "Flushes the buffers of a specified file and causes all buffered data to be written to a file"; "The file handle must have the GENERIC_WRITE access right"; "If the function succeeds, the return value is nonzero.
 * If the function fails, the return value is zero. To get extended error information, call GetLastError". The volume keeps the bytes of a file in memory, so there is nothing to write out. A handle opened without write access fails
 * (the page names no code: ERROR_ACCESS_DENIED, 5, "Access is denied", System Error Codes; Wine 10.0 gives it, UNOFFICIAL); NULL, INVALID_HANDLE_VALUE, a closed handle and a find handle fail with ERROR_INVALID_HANDLE (6). */
void fle_imp_FlushFileBuffers(void)
{
    static const char api[] = "FlushFileBuffers";
    fh_ent *e = file_arg(api, R[1]);

    if (!e) {
        fail(6, 0);
        return;
    }
    if (!e->wr && !e->app) {
        fail(5, 0);
        return;
    }
    R[0] = 1;
}

/* kernel32!GetFileSize (MSDN: GetFileSize). hFile rcx, lpFileSizeHigh rdx (optional). Returns the low part and
 * writes the high part when asked. Success sets the last error to 0 (page: GetLastError returns NO_ERROR
 * upon success). An invalid or closed handle returns INVALID_FILE_SIZE with 6 (inference). */
void fle_imp_GetFileSize(void)
{
    static const char api[] = "GetFileSize";
    fh_ent *e = file_arg(api, R[1]);
    uint64_t ph = R[2];
    if (!e) {
        fail(6, FLE_INVALID_FILE_SIZE);
        return;
    }
    if (ph != 0) fle_st(ph, 4, e->node->size >> 32);
    fle_set_last_error(0);
    R[0] = (uint32_t)e->node->size;
}

/* kernel32!SetFileTime (MSDN: SetFileTime). hFile rcx, lpCreationTime rdx, lpLastAccessTime r8, lpLastWriteTime
 * r9. A NULL pointer or a FILETIME of 0/0 leaves that time alone (page). A FILETIME of 0xFFFFFFFF/0xFFFFFFFF
 * (suppress later updates) is not implemented and stops. A handle without write-attributes access fails with 5
 * (the page requires FILE_WRITE_ATTRIBUTES; the code is an inference). */
void fle_imp_SetFileTime(void)
{
    static const char api[] = "SetFileTime";
    fh_ent *e = file_arg(api, R[1]);
    uint64_t pt[3] = { R[2], R[8], R[9] };
    uint64_t val[3];
    int i;
    if (!e) {
        fail(6, 0);
        return;
    }
    for (i = 0; i < 3; i++) {
        val[i] = pt[i] ? fle_ld(pt[i], 8) : 0;
        if (val[i] == UINT64_C(0xFFFFFFFFFFFFFFFF)) fle_stop(api, "FILETIME 0xFFFFFFFF/0xFFFFFFFF is not implemented");
    }
    if (!e->wattr) {
        fail(5, 0);
        return;
    }
    if (val[0]) e->node->ct = val[0];
    if (val[1]) e->node->at = val[1];
    if (val[2]) e->node->wt = val[2];
    R[0] = 1;
}

/* ---- directory search ---- */

typedef struct {
    uint32_t attrs;
    uint64_t ct, at, wt, size;
    uint16_t name[MAX_COMPONENT + 1];
    size_t nlen;
} find_item;

typedef struct {
    int used;
    find_item *items;
    size_t n, idx;
} find_ent;

static find_ent *fd;
static size_t fd_n, fd_cap;

static void item_from(find_item *it, const fle_node *n, const uint16_t *name, size_t nl)
{
    it->attrs = node_attrs(n);
    it->ct = n->ct;
    it->at = n->at;
    it->wt = n->wt;
    it->size = n->is_dir ? 0 : n->size;
    memcpy(it->name, name, nl * sizeof *name);
    it->name[nl] = 0;
    it->nlen = nl;
}

static void fill_find_data(uint64_t dst, const find_item *it)
{
    uint8_t *p = (uint8_t *)fle_addr(dst, FLE_FIND_DATA_SIZE, 1);
    memset(p, 0, FLE_FIND_DATA_SIZE);
    fle_st(dst + 0, 4, it->attrs);
    fle_st(dst + 4, 8, it->ct);
    fle_st(dst + 12, 8, it->at);
    fle_st(dst + 20, 8, it->wt);
    fle_st(dst + 28, 4, it->size >> 32);
    fle_st(dst + 32, 4, (uint32_t)it->size);
    memcpy(p + FLE_FIND_DATA_NAME_OFF, it->name, (it->nlen + 1) * sizeof(uint16_t));
}

static int glob(const uint16_t *p, size_t pl, const uint16_t *s, size_t sl)
{
    while (pl) {
        if (*p == '*') {
            size_t k;
            while (pl && *p == '*') {
                p++;
                pl--;
            }
            if (!pl) return 1;
            for (k = 0; k <= sl; k++)
                if (glob(p, pl, s + k, sl - k)) return 1;
            return 0;
        }
        if (!sl) return 0;
        if (*p != '?' && fle_w_fold(*p) != fle_w_fold(*s)) return 0;
        p++;
        pl--;
        s++;
        sl--;
    }
    return sl == 0;
}

static void item_push(find_item **items, size_t *n, size_t *cap, const fle_node *node, const uint16_t *name, size_t nl)
{
    if (*n == *cap) {
        size_t nc = *cap ? *cap * 2 : 16;
        find_item *ni = realloc(*items, nc * sizeof **items);
        if (!ni) fle_stop("FindFirstFileW", "out of memory");
        *items = ni;
        *cap = nc;
    }
    item_from(&(*items)[(*n)++], node, name, nl);
}

/* kernel32!FindFirstFileW (MSDN: FindFirstFileW, WIN32_FIND_DATAW). lpFileName rcx, lpFindFileData rdx.
 * A name without wildcards returns the entry for that file or directory itself. A name whose last component
 * has * or ? lists the matching entries of that directory (order: "." and ".." first, then creation order;
 * the page says the order is not defined). "." and ".." for a non-root directory is NOT on the pages read
 * (it is known Windows behaviour; unverified here). "*.*" is treated as "*" (also not on the pages; inference).
 * Failures return INVALID_HANDLE_VALUE: no match 2 (page), missing directory 3 (inference), a trailing
 * backslash 123 (the page says it always fails; the code is an inference). Short names are empty (the page says
 * 8.3 names may not exist). Wildcards outside the last component, an empty or NULL name stop. */
void fle_imp_FindFirstFileW(void)
{
    static const char api[] = "FindFirstFileW";
    uint64_t pdata = R[2];
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    find_item *items = NULL;
    size_t cnt = 0, cap = 0, i;
    find_ent *fe;

    if (pdata == 0) fle_stop(api, "lpFindFileData is NULL");
    parse_path(api, R[1], 1, &v);
    if (v.trail || v.n == 0) {
        if (v.n == 0 && !v.trail) fle_stop(api, "a name that resolves to no component is not defined");
        vp_free(&v);
        fail(123, FLE_INVALID_HANDLE_VALUE);
        return;
    }
    if (v.wild) {
        vpath d = v;
        const uint16_t *pat = v.p[v.n - 1];
        size_t pl = v.l[v.n - 1];
        static const uint16_t star_dot_star[3] = { '*', '.', '*' };
        static const uint16_t star[1] = { '*' };
        d.n--;
        d.trail = 0;
        st = lookup(api, &d, &n, &par);
        if (st != 0 || !n->is_dir) {
            vp_free(&v);
            fail(3, FLE_INVALID_HANDLE_VALUE);
            return;
        }
        if (pl == 3 && !memcmp(pat, star_dot_star, sizeof star_dot_star)) {
            pat = star;
            pl = 1;
        }
        if (n != &root_node) {
            static const uint16_t dot[1] = { '.' }, dotdot[2] = { '.', '.' };
            if (glob(pat, pl, dot, 1)) item_push(&items, &cnt, &cap, n, dot, 1);
            if (glob(pat, pl, dotdot, 2)) item_push(&items, &cnt, &cap, n->parent, dotdot, 2);
        }
        for (i = 0; i < n->nkids; i++)
            if (glob(pat, pl, n->kids[i]->name, n->kids[i]->nlen))
                item_push(&items, &cnt, &cap, n->kids[i], n->kids[i]->name, n->kids[i]->nlen);
    } else {
        st = lookup(api, &v, &n, &par);
        if (st == 0) item_push(&items, &cnt, &cap, n, n->name, n->nlen);
    }
    vp_free(&v);
    if (cnt == 0) {
        free(items);
        fail(st == 3 ? 3u : 2u, FLE_INVALID_HANDLE_VALUE);
        return;
    }
    if (fd_n == fd_cap) {
        size_t nc = fd_cap ? fd_cap * 2 : 16;
        find_ent *nf = realloc(fd, nc * sizeof *nf);
        if (!nf) fle_stop(api, "out of memory");
        fd = nf;
        fd_cap = nc;
    }
    if (fd_n + 2 >= FLE_HANDLE_REGION_SIZE / 4) fle_stop(api, "find handle region exhausted");
    fe = &fd[fd_n++];
    fe->used = 1;
    fe->items = items;
    fe->n = cnt;
    fe->idx = 1;
    fill_find_data(pdata, &items[0]);
    R[0] = FLE_VFS_FIND_BASE + 4u * (uint64_t)fd_n;
}

static find_ent *find_get(uint64_t h)
{
    uint64_t off, idx;
    h = hnorm(h);
    if (!in_region(h, FLE_VFS_FIND_BASE)) return NULL;
    off = h - FLE_VFS_FIND_BASE;
    if (off % 4) return NULL;
    idx = off / 4 - 1;
    if (idx >= fd_n || !fd[idx].used) return NULL;
    return &fd[idx];
}


/* Decision 0238 (load2, PROVISIONAL): kernel32!FindFirstFileExW, HANDLE f(LPCWSTR lpFileName rcx, FINDEX_INFO_LEVELS fInfoLevelId edx, LPVOID lpFindFileData r8, FINDEX_SEARCH_OPS fSearchOp r9d, LPVOID lpSearchFilter [5th],
 * DWORD dwAdditionalFlags [6th]). It stopped by name (fle_vol.c, decision 0078; the statically linked CRT expands a wildcard of a command-line argument with it, and DelZip64's extraction code reaches it; decision 0206 answered the
 * plain cases through an opt-in harness wrap). Learn (FindFirstFileExW): fInfoLevelId FindExInfoStandard (0) "return a WIN32_FIND_DATA structure", FindExInfoBasic (1) the same "without the cAlternateFileName (short name)"; fSearchOp
 * FindExSearchNameMatch (0) "The search for a file that matches a specified file name", FindExSearchLimitToDirectories (1) "this is an advisory flag. If the file system supports directory filtering, the function searches for a file that
 * matches the specified name and is also a directory. If the file system does not support directory filtering, this value is silently ignored" (so every match may be returned); lpSearchFilter "At this time, none of the supported
 * fSearchOp values require extended search information. Therefore, this pointer must be NULL"; dwAdditionalFlags FIND_FIRST_EX_LARGE_FETCH (2) "Uses a larger buffer for directory queries, which can increase performance of the find
 * operation" (a hint). The plain cases are FindFirstFileW (short names are empty there, which both info levels allow). FindExSearchLimitToDevices (2), a filter, FIND_FIRST_EX_CASE_SENSITIVE (1), FIND_FIRST_EX_ON_DISK_ENTRIES_ONLY (4)
 * and a level above 1 change what the search means and stop by name. */
void fle_imp_FindFirstFileExW(void)
{
    static const char api[] = "FindFirstFileExW";
    uint32_t level = (uint32_t)R[2], op = (uint32_t)R[9], flags = (uint32_t)fle_arg(6);

    if (level > 1) fle_stop(api, "fInfoLevelId is not FindExInfoStandard or FindExInfoBasic");
    if (op > 1) fle_stop(api, "fSearchOp is not FindExSearchNameMatch or FindExSearchLimitToDirectories");
    if (fle_arg(5) != 0) fle_stop(api, "lpSearchFilter is not NULL (the page: it must be NULL)");
    if (flags & ~2u) fle_stop(api, "dwAdditionalFlags has a bit other than FIND_FIRST_EX_LARGE_FETCH (not modelled)");
    R[2] = R[8];   /* FindFirstFileW(lpFileName rcx, lpFindFileData rdx) */
    fle_imp_FindFirstFileW();
}

/* kernel32!FindNextFileW (MSDN: FindNextFileW). hFindFile rcx, lpFindFileData rdx. After the last entry it
 * fails with ERROR_NO_MORE_FILES (18, page). A closed, invalid or file handle fails with 6 (inference). */
void fle_imp_FindNextFileW(void)
{
    static const char api[] = "FindNextFileW";
    int k = h_kind(R[1]);
    find_ent *fe;
    if (k == 0) fle_stop(api, "handle was not issued by the virtual file system");
    if (R[2] == 0) fle_stop(api, "lpFindFileData is NULL");
    fe = k == 2 ? find_get(R[1]) : NULL;
    if (!fe) {
        fail(6, 0);
        return;
    }
    if (fe->idx >= fe->n) {
        fail(18, 0);
        return;
    }
    fill_find_data(R[2], &fe->items[fe->idx++]);
    R[0] = 1;
}

/* kernel32!FindClose (MSDN: FindClose). A closed, invalid or file handle fails with 0 and 6 (inference). */
void fle_imp_FindClose(void)
{
    static const char api[] = "FindClose";
    int k = h_kind(R[1]);
    find_ent *fe;
    if (k == 0) fle_stop(api, "handle was not issued by the virtual file system");
    fe = k == 2 ? find_get(R[1]) : NULL;
    if (!fe) {
        fail(6, 0);
        return;
    }
    free(fe->items);
    fe->items = NULL;
    fe->used = 0;
    R[0] = 1;
}

/* ---- attributes ---- */

/* Decision 0126 (wall 87): 1 when the path (a guest wide string) names a file or directory of the virtual file system, 0 when it does not. For shell32!ILCreateFromPathW (fle_ole.c, weak reference). */
int fle_vfs_path_exists(const char *api, uint64_t guest_path)
{
    vpath v;
    fle_node *n, *par;
    uint32_t st;

    parse_path(api, guest_path, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    return st == 0;
}

/* kernel32!GetFileAttributesW (MSDN: GetFileAttributesW). lpFileName rcx. Returns the attributes (the root
 * directory has FILE_ATTRIBUTE_DIRECTORY only: the page does not say what a real root has). Failure returns
 * INVALID_FILE_ATTRIBUTES with 2 or 3 (inference for the choice between them).
 * The empty name "" (decision 0123 item 1, provisional): INVALID_FILE_ATTRIBUTES with last error 3 (ERROR_PATH_NOT_FOUND). A CHOICE WITHOUT A
 * SOURCE: neither GetFileAttributesW nor Naming Files, Paths, and Namespaces says what an empty name gives (the PM remembers 3; UNVERIFIED).
 * Withdrawn when a measurement on a real Windows gives another answer. Only this API answers it: the other path APIs keep stopping on "". */
void fle_imp_GetFileAttributesW(void)
{
    static const char api[] = "GetFileAttributesW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    if (R[1] != 0 && fle_ld(R[1], 2) == 0) {
        fail(3, FLE_INVALID_FILE_ATTRIBUTES);
        return;
    }
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, FLE_INVALID_FILE_ATTRIBUTES);
        return;
    }
    R[0] = node_attrs(n);
}

/* Decision 0183 (wrp1, PROVISIONAL): see fle_vfs.h. */
int fle_vfs_guest_is_file(const char *api, uint64_t guest)
{
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    parse_path(api, guest, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    return st == 0 && n && !n->is_dir;
}

/* kernel32!GetFileAttributesExW (MSDN: GetFileAttributesExW, WIN32_FILE_ATTRIBUTE_DATA). lpFileName rcx,
 * fInfoLevelId edx (only GetFileExInfoStandard = 0), lpFileInformation r8. Failure returns 0 with 2 or 3. */
void fle_imp_GetFileAttributesExW(void)
{
    static const char api[] = "GetFileAttributesExW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    uint64_t out = R[8];
    if ((uint32_t)R[2] != 0) fle_stop(api, "fInfoLevelId is not GetFileExInfoStandard");
    if (out == 0) fle_stop(api, "lpFileInformation is NULL");
    /* Decision 0126 (PROVISIONAL), wall 70 (S-empty-attrs-ex, the choice of GetFileAttributesW of decision 0123 item 1 applied to the Ex form): the empty name "" fails with 0 and last error 3 (ERROR_PATH_NOT_FOUND); the
     * page of GetFileAttributesExW does not say what an empty name gives. The structure is left as it is. */
    if (R[1] != 0 && fle_ld(R[1], 2) == 0) {
        fail(3, 0);
        return;
    }
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    fle_st(out + 0, 4, node_attrs(n));
    fle_st(out + 4, 8, n->ct);
    fle_st(out + 12, 8, n->at);
    fle_st(out + 20, 8, n->wt);
    fle_st(out + 28, 4, n->is_dir ? 0 : n->size >> 32);
    fle_st(out + 32, 4, n->is_dir ? 0 : (uint32_t)n->size);
    R[0] = 1;
}

/* kernel32!SetFileAttributesW (MSDN: SetFileAttributesW). lpFileName rcx, dwFileAttributes edx. Only the values
 * the page lists as supported are accepted (anything else, including FILE_ATTRIBUTE_DIRECTORY, stops). NORMAL
 * alone clears the settable bits. READONLY is not honored on directories (page). The root cannot be changed
 * (stops). Missing path: 0 with 2 or 3. */
void fle_imp_SetFileAttributesW(void)
{
    static const char api[] = "SetFileAttributesW";
    uint32_t a = (uint32_t)R[2];
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    if (a & ~(ATTR_SETTABLE | ATTR_NORMAL)) fle_stop(api, "dwFileAttributes has values the page does not list as supported");
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    if (n == &root_node) fle_stop(api, "changing the attributes of the volume root is not implemented");
    n->bits = a & ATTR_SETTABLE;
    if (n->is_dir) n->bits &= ~ATTR_READONLY;
    R[0] = 1;
}

/* ---- directories, delete, copy, move ---- */

/* kernel32!CreateDirectoryW (MSDN: CreateDirectoryW). lpPathName rcx, lpSecurityAttributes rdx (non-NULL
 * stops: there is no security). An existing directory (or file) fails with ERROR_ALREADY_EXISTS (183); a
 * missing intermediate directory with ERROR_PATH_NOT_FOUND (3); both from the page. */
void fle_imp_CreateDirectoryW(void)
{
    static const char api[] = "CreateDirectoryW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    if (R[2] != 0) fle_stop(api, "lpSecurityAttributes is not NULL (not implemented)");
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    if (st == 0) {
        vp_free(&v);
        fail(183, 0);
        return;
    }
    if (st == 3) {
        vp_free(&v);
        fail(3, 0);
        return;
    }
    node_new(par, v.p[v.n - 1], v.l[v.n - 1], 1, 0);
    vp_free(&v);
    R[0] = 1;
}

/* kernel32!RemoveDirectoryW (MSDN: RemoveDirectoryW). lpPathName rcx. Only an empty directory is removed
 * (page). Not empty: 145 (ERROR_DIR_NOT_EMPTY, an inference: the page names no code); a file: 267
 * (ERROR_DIRECTORY, inference); the root: 5; the current directory: 32 (the SetCurrentDirectory page says the
 * current directory is locked; the code is an inference). Missing: 2 or 3. */
void fle_imp_RemoveDirectoryW(void)
{
    static const char api[] = "RemoveDirectoryW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    if (n == &root_node) {
        fail(5, 0);
        return;
    }
    if (!n->is_dir) {
        fail(267, 0);
        return;
    }
    if (n->nkids) {
        fail(145, 0);
        return;
    }
    if (n == cwd_node) {
        fail(32, 0);
        return;
    }
    node_free(n);
    R[0] = 1;
}

/* kernel32!DeleteFileW (MSDN: DeleteFileW). lpFileName rcx. Missing: 2 (3 for a missing directory); read-only:
 * 5 (both page). A file with another handle that did not allow FILE_SHARE_DELETE: 32 (page). A file whose
 * handles all allow it is marked and removed when the last handle closes (page); later opens fail with 5
 * (page). A directory: 5 (inference; the page sends the reader to RemoveDirectory). */
void fle_imp_DeleteFileW(void)
{
    static const char api[] = "DeleteFileW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    if (n->is_dir || n->pending || (n->bits & ATTR_READONLY)) {
        fail(5, 0);
        return;
    }
    if (share_conflict(n, 0, 0, 1, 7)) {
        fail(32, 0);
        return;
    }
    if (n->opens) n->pending = 1;
    else node_free(n);
    R[0] = 1;
}

/* kernel32!CopyFileW (MSDN: CopyFileW). lpExistingFileName rcx, lpNewFileName rdx, bFailIfExists r8d. Missing
 * source: 2 (page; 3 for a missing directory). The destination existing with bFailIfExists: 0 with 80
 * (inference; the page gives no code). Destination hidden or read-only: 5 (page). Attributes are copied (page).
 * Directories (source or destination) stop. Sharing: the source is opened for read allowing read and write,
 * the destination for write allowing nothing; a conflict returns 32 (the share modes are an inference). The
 * copy keeps the source's last-write time (not on the page). */
void fle_imp_CopyFileW(void)
{
    static const char api[] = "CopyFileW";
    vpath s, d;
    fle_node *sn, *sp, *dn, *dp;
    uint32_t sst, dst;
    uint32_t fail_if = (uint32_t)R[8];

    parse_path(api, R[1], 0, &s);
    parse_path(api, R[2], 0, &d);
    sst = lookup(api, &s, &sn, &sp);
    if (sst) {
        vp_free(&s);
        vp_free(&d);
        fail(sst, 0);
        return;
    }
    if (sn->is_dir) fle_stop(api, "copying a directory is not implemented");
    if (d.trail || d.n == 0) fle_stop(api, "destination names no file");
    dst = lookup(api, &d, &dn, &dp);
    if (dst == 3) {
        vp_free(&s);
        vp_free(&d);
        fail(3, 0);
        return;
    }
    if (dst == 0) {
        if (dn->is_dir) fle_stop(api, "the destination is a directory");
        if (fail_if) {
            vp_free(&s);
            vp_free(&d);
            fail(80, 0);
            return;
        }
        if (dn == sn || dn->pending) {
            vp_free(&s);
            vp_free(&d);
            fail(dn == sn ? 32u : 5u, 0);
            return;
        }
        if (dn->bits & (ATTR_HIDDEN | ATTR_READONLY)) {
            vp_free(&s);
            vp_free(&d);
            fail(5, 0);
            return;
        }
        if (share_conflict(dn, 0, 1, 0, 0)) {
            vp_free(&s);
            vp_free(&d);
            fail(32, 0);
            return;
        }
    }
    if (share_conflict(sn, 1, 0, 0, 3)) {
        vp_free(&s);
        vp_free(&d);
        fail(32, 0);
        return;
    }
    if (dst == 2) dn = node_new(dp, d.p[d.n - 1], d.l[d.n - 1], 0, sn->bits);
    node_truncate(dn);
    if (sn->size) {
        node_reserve(api, dn, sn->size);
        memcpy(dn->data, sn->data, (size_t)sn->size);
    }
    dn->size = sn->size;
    dn->bits = sn->bits;
    dn->wt = sn->wt;
    dn->at = tick();
    vp_free(&s);
    vp_free(&d);
    R[0] = 1;
}

/* kernel32!MoveFileW (MSDN: MoveFileW). lpExistingFileName rcx, lpNewFileName rdx. Files and directories (with
 * their children) move inside the one volume. The new name must not exist (page): 0 with 183 (the code is an
 * inference). Missing source: 2 or 3; missing destination directory: 3. A file or tree with open handles, or the
 * current directory or one of its parents: 32 (the CreateFile page says delete access covers rename; the code is
 * an inference). Moving a directory into itself and moving the root stop. */
void fle_imp_MoveFileW(void)
{
    static const char api[] = "MoveFileW";
    vpath s, d;
    fle_node *sn, *sp, *dn, *dp;
    uint32_t sst, dst;

    parse_path(api, R[1], 0, &s);
    parse_path(api, R[2], 0, &d);
    sst = lookup(api, &s, &sn, &sp);
    if (sst) {
        vp_free(&s);
        vp_free(&d);
        fail(sst, 0);
        return;
    }
    if (sn == &root_node) fle_stop(api, "moving the volume root");
    if (d.n == 0) {
        vp_free(&s);
        vp_free(&d);
        fail(183, 0);
        return;
    }
    if (d.trail && !sn->is_dir) fle_stop(api, "a trailing separator on a file's new name is not defined");
    dst = lookup(api, &d, &dn, &dp);
    if (dst == 3) {
        vp_free(&s);
        vp_free(&d);
        fail(3, 0);
        return;
    }
    if (dst == 0) {
        vp_free(&s);
        vp_free(&d);
        fail(183, 0);
        return;
    }
    if (is_ancestor_or_self(sn, dp)) fle_stop(api, "moving a directory into itself");
    if (sn->pending) {
        vp_free(&s);
        vp_free(&d);
        fail(5, 0);
        return;
    }
    if (tree_has_open(sn) || is_ancestor_or_self(sn, cwd_node)) {
        vp_free(&s);
        vp_free(&d);
        fail(32, 0);
        return;
    }
    detach(sn);
    set_name(sn, d.p[d.n - 1], d.l[d.n - 1]);
    attach(dp, sn);
    vp_free(&s);
    vp_free(&d);
    R[0] = 1;
}

/* ---- paths of the process ---- */

/* Writes `len` units of s plus a NUL to the guest, or reports the required size. Returns the value for R[0]. */
static uint32_t fit(const char *api, const uint16_t *s, size_t len, uint32_t nbuf, uint64_t buf)
{
    if (nbuf > len) {
        if (buf == 0) fle_stop(api, "lpBuffer is NULL but nSize leaves room for the string");
        fle_w_write(buf, s, len + 1);
        return (uint32_t)len;
    }
    return (uint32_t)(len + 1);
}

/* kernel32!GetFullPathNameW (MSDN: GetFullPathNameW). lpFileName rcx, nBufferLength edx, lpBuffer r8,
 * lpFilePart r9. Joins the current directory, resolves "." and "..", keeps a trailing backslash, and does not
 * check that the path exists (page). The return value is the length without the NUL, or the required size
 * including it when the buffer is too small (page). lpFilePart gets the address of the last component, or NULL
 * when the text ends with a backslash or is the root (page: NULL for a directory; the check here is by syntax
 * only). Only drive C: exists; UNC, "\\?\", drive-relative and device names stop. */
void fle_imp_GetFullPathNameW(void)
{
    static const char api[] = "GetFullPathNameW";
    vpath v;
    uint32_t nbuf = (uint32_t)R[2];
    uint64_t buf = R[8], ppart = R[9];
    size_t total = 2, o = 0, i, last_off = 0;
    uint16_t *s;
    uint32_t r;

    /* Decision 0126 (PROVISIONAL), wall 69 (S-fullpath-empty): GetFullPathNameW with an EMPTY lpFileName (the guest's Delphi ExpandFileName of an empty file name, 0x43c9c4: the call is made with a buffer of 0x104 units and
     * the result is the length of the string it gets). The page names no rule for an empty name: "If the function fails for any other reason, the return value is zero. To get extended error information, call
     * GetLastError". A READING (no page): an empty string names no file, so the function fails: the return value 0 and ERROR_INVALID_NAME (123, System Error Codes: "The filename, directory name, or volume label syntax is
     * incorrect") as the last error, the buffer untouched, lpFilePart untouched. The guest takes the 0 as an empty expanded name (its code at 0x43c9c6 compares the result with 0x104 and copies that many units). */
    if (R[1] != 0 && fle_ld(R[1], 2) == 0) {
        fle_set_last_error(123u);
        R[0] = 0;
        return;
    }
    parse_path(api, R[1], 0, &v);
    if (v.n == 0) total = 3;
    for (i = 0; i < v.n; i++) total += 1 + v.l[i];
    if (v.trail && v.n > 0) total++;
    s = malloc((total + 1) * sizeof *s);
    if (!s) fle_stop(api, "out of memory");
    s[o++] = 'C';
    s[o++] = ':';
    if (v.n == 0) s[o++] = '\\';
    for (i = 0; i < v.n; i++) {
        s[o++] = '\\';
        last_off = o;
        memcpy(s + o, v.p[i], v.l[i] * sizeof *s);
        o += v.l[i];
    }
    if (v.trail && v.n > 0) s[o++] = '\\';
    s[o] = 0;
    r = fit(api, s, o, nbuf, buf);
    if (r == (uint32_t)o && ppart != 0) {
        uint64_t fp = (v.trail || v.n == 0) ? 0 : buf + 2u * last_off;
        fle_st(ppart, 8, fp);
    }
    free(s);
    vp_free(&v);
    R[0] = r;
}

/* kernel32!GetCurrentDirectoryW (MSDN: GetCurrentDirectory). nBufferLength ecx, lpBuffer rdx. Returns the
 * length without the NUL, or the required size including it when the buffer is too small (page); NULL with
 * length 0 asks for the size (page). The initial directory is the volume root "C:\" (a decision of 0018). The
 * string has no trailing backslash except for the root (convention, not stated on the page). */
void fle_imp_GetCurrentDirectoryW(void)
{
    static const char api[] = "GetCurrentDirectoryW";
    size_t len;
    uint16_t *s = node_path(cwd_node, &len);
    R[0] = fit(api, s, len, (uint32_t)R[1], R[2]);
    free(s);
}

/* kernel32!SetCurrentDirectoryW (MSDN: SetCurrentDirectory). lpPathName rcx; relative or full. The directory
 * must exist: 2 or 3 when it does not, 267 (ERROR_DIRECTORY) when it is a file (the codes are inferences). */
void fle_imp_SetCurrentDirectoryW(void)
{
    static const char api[] = "SetCurrentDirectoryW";
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    parse_path(api, R[1], 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    if (!n->is_dir) {
        fail(267, 0);
        return;
    }
    cwd_node = n;
    R[0] = 1;
}

/* kernel32!GetTempFileNameW: UINT f(LPCWSTR lpPathName rcx, LPCWSTR lpPrefixString rdx, UINT uUnique r8d, LPWSTR lpTempFileName r9). Decision 0132 (PROVISIONAL), wall 22 (the news-download thread of the guest asks for
 * ("C:\Windows\", "ILNewsDownload", 0x755f60ca)). Learn (GetTempFileNameW): "Creates a name for a temporary file. If a unique file name is generated, an empty file is created and the handle to it is released; otherwise, only a
 * file name is generated"; "The string cannot be longer than MAX_PATH-14 characters or GetTempFileName will fail. If this parameter is NULL, the function fails"; "The function uses up to the first three characters of this
 * string as the prefix of the file name"; "the function does not attempt to verify the uniqueness of the file name when uUnique is nonzero"; "If the function succeeds, the return value specifies the unique numeric value used
 * in the temporary file name. If the uUnique parameter is nonzero, the return value specifies that same number"; failure is zero with ERROR_BUFFER_OVERFLOW "if the length of the string pointed to by the lpPathName
 * parameter is more than MAX_PATH-14 characters"; the form is "<path>\<pre><uuuu>.TMP", "<uuuu>: Hexadecimal value of uUnique", "Only the lower 16 bits of the uUnique parameter are used"; "If uUnique is not zero, you must create
 * the file yourself". Answered: uUnique nonzero: the name is written (the file is not created, the page's words) and the result is uUnique as it was passed. The choices without a source: S-gettempfilename-name: the backslash
 * between <path> and <pre> is not added when the path already ends with one (the page's form shows one backslash; GetTempPathW's result always ends with one, so the guest's path does), the four digits are the lower 16 bits as
 * %04X (upper case, zero-padded: the page says "hexadecimal" and "uuuu"), the extension is the page's ".TMP" (an operating system writes ".tmp", which a case-insensitive volume does not tell apart). uUnique zero (a file is
 * created and named after the system time), a NULL pointer (the page names no error code for a NULL path; a NULL prefix or buffer is not described), an empty path or a character of the prefix above 0x7E stop by name. */
void fle_imp_GetTempFileNameW(void)
{
    static const char api[] = "GetTempFileNameW";
    uint64_t pp = R[1], pf = R[2], out = R[9];
    uint32_t uniq = (uint32_t)R[8];
    uint16_t *path, *pre, name[300];
    size_t plen, flen, n, i;
    char hex[8];

    if (pp == 0) fle_stop(api, "lpPathName is NULL: the page says the function fails and names no error code");
    if (pf == 0) fle_stop(api, "lpPrefixString is NULL: the page does not describe it");
    if (out == 0) fle_stop(api, "lpTempFileName is NULL: the page does not describe it");
    if (uniq == 0) fle_stop(api, "uUnique is 0: the function would create a file named after the system time (the page's words); only a nonzero uUnique, which only forms a name, is answered");
    path = fle_w_read_alloc(api, pp, 4096, &plen);
    if (plen > 260u - 14u) {   /* MAX_PATH - 14 */
        free(path);
        fail(111, 0);          /* ERROR_BUFFER_OVERFLOW (the page) */
        return;
    }
    if (plen == 0) fle_stop(api, "lpPathName is empty: the page does not describe it");
    pre = fle_w_read_alloc(api, pf, 4096, &flen);
    if (flen > 3) flen = 3;
    for (i = 0; i < flen; i++)
        if (pre[i] > 0x7E || pre[i] < 0x20) fle_stop(api, "a character of the prefix is not printable ASCII: the OEM character set is not modelled");
    n = 0;
    memcpy(name, path, plen * sizeof *name);
    n = plen;
    if (name[n - 1] != '\\') name[n++] = '\\';
    for (i = 0; i < flen; i++) name[n++] = pre[i];
    snprintf(hex, sizeof hex, "%04X", (unsigned)(uniq & 0xFFFFu));
    for (i = 0; hex[i]; i++) name[n++] = (uint16_t)hex[i];
    name[n++] = '.'; name[n++] = 'T'; name[n++] = 'M'; name[n++] = 'P';
    name[n] = 0;
    fle_w_write(out, name, n + 1);
    free(path);
    free(pre);
    R[0] = uniq;
}

/* kernel32!GetTempPathW (MSDN: GetTempPathW). nBufferLength ecx, lpBuffer rdx. The first non-empty of the
 * virtual environment's TMP, TEMP, USERPROFILE (the host environment is not read), else the virtual Windows
 * directory; the result ends with a backslash (page) and is not checked for existence (page). A result over
 * 261 characters (the page's maximum) stops. Return: length without NUL, or the size needed including it. */
void fle_imp_GetTempPathW(void)
{
    static const char api[] = "GetTempPathW";
    static const uint16_t n_tmp[] = { 'T', 'M', 'P', 0 };
    static const uint16_t n_temp[] = { 'T', 'E', 'M', 'P', 0 };
    static const uint16_t n_up[] = { 'U', 'S', 'E', 'R', 'P', 'R', 'O', 'F', 'I', 'L', 'E', 0 };
    const uint16_t *names[3] = { n_tmp, n_temp, n_up };
    const uint16_t *val = NULL;
    uint16_t *win = NULL, *s;
    size_t len, i;

    for (i = 0; i < 3 && !val; i++) {
        const uint16_t *e = fle_env_get(names[i]);
        if (e && e[0]) val = e;
    }
    if (!val) {
        win = fle_w_from_ascii(FLE_VIRTUAL_WINDOWS_DIR);
        val = win;
    }
    len = fle_w_len(val);
    s = malloc((len + 2) * sizeof *s);
    if (!s) fle_stop(api, "out of memory");
    memcpy(s, val, len * sizeof *s);
    if (len == 0 || s[len - 1] != '\\') s[len++] = '\\';
    s[len] = 0;
    if (len > 261) fle_stop(api, "temporary path longer than the page's maximum of 261 characters");
    R[0] = fit(api, s, len, (uint32_t)R[1], R[2]);
    free(s);
    free(win);
}

/* Decision 0104 (provisional): the directory of a real module's path. "Shared" is the directory the installer puts the bundled DLLs in
 * (docs/investigations/bundled-dll-inventory.md section 4); the part before it is the install directory of FLE_VIRTUAL_DLL_PATH
 * (the PM's value of decision 0018, itself not a Windows rule). */
#define FLE_VIRTUAL_SHARED_DIR "C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\"

/* The virtual path of a module for GetModuleFileNameW/A: NULL is the executable file of the process, FLE_VIRTUAL_EXE_PATH (decision 0118,
 * provisional: it was FLE_VIRTUAL_DLL_PATH from decision 0018 to 0117); the lifted DLL's handle (0x400000, decision 0015) is FLE_VIRTUAL_DLL_PATH
 * (decision 0018, the PM's value, not a Windows rule; GetModuleHandleW(NULL) still answers 0x400000, so NULL and 0x400000 are no longer the same module
 * here, which is decision 0118's open point). The handle (= base) of a real module (fle_realmod.h) is
 * FLE_VIRTUAL_SHARED_DIR + the file name that the module's own export directory records (decision 0104, provisional; this also settles the
 * value that decision 0082 D6 left open and that decision 0102 W1 first set to the directory of FLE_VIRTUAL_DLL_PATH + the table name).
 * Any other handle stops (the other virtual modules have no path). */
static const char *module_path(const char *api, uint64_t h, char *buf, size_t cap)
{
    char nm[128];
    int rm;

    if (h == 0) return FLE_VIRTUAL_EXE_PATH;
    if (h == FLE_EXE_HANDLE) return FLE_VIRTUAL_DLL_PATH;
    rm = FLE_REALMOD_PRESENT() ? fle_realmod_find_handle(h) : -1;
    if (rm < 0 || fle_realmod_file_name(api, rm, nm, sizeof nm) <= 0)
        fle_stop(api, "only the lifted DLL has a path (hModule is not NULL or 0x400000)");
    snprintf(buf, cap, "%s%s", FLE_VIRTUAL_SHARED_DIR, nm);
    return buf;
}

/* kernel32!GetModuleFileNameW (MSDN: GetModuleFileNameW). hModule rcx, lpFilename rdx, nSize r8d. The path is module_path().
 * A path shorter than nSize is copied and its length returned. nSize 0 returns 0 and leaves the last error.
 * A path that does not fit is truncated to nSize characters including the NUL, nSize is returned and the last
 * error is ERROR_INSUFFICIENT_BUFFER (122) (page, Windows Vista and later text). */
void fle_imp_GetModuleFileNameW(void)
{
    static const char api[] = "GetModuleFileNameW";
    char full[512];
    uint64_t h = R[1], buf = R[2];
    uint32_t nsize = (uint32_t)R[8];
    uint16_t *p = fle_w_from_ascii(module_path(api, h, full, sizeof full));
    size_t len = fle_w_len(p);

    if (nsize == 0) {
        free(p);
        R[0] = 0;
        return;
    }
    if (buf == 0) fle_stop(api, "lpFilename is NULL");
    if (len < nsize) {
        fle_w_write(buf, p, len + 1);
        free(p);
        R[0] = (uint32_t)len;
        return;
    }
    fle_w_write(buf, p, nsize - 1u);
    fle_st(buf + 2u * (nsize - 1u), 2, 0);
    free(p);
    fle_set_last_error(FLE_W32_ERR_INSUFFICIENT_BUFFER);
    R[0] = nsize;
}

/* kernel32!GetModuleFileNameA (MSDN: GetModuleFileNameA, decision 0102 W7): the same path and rules as GetModuleFileNameW, in the 8-bit
 * process code page; the path is ASCII, which that code page keeps as it is (the assumption of GetCommandLineA, decision 0078). */
void fle_imp_GetModuleFileNameA(void)
{
    static const char api[] = "GetModuleFileNameA";
    char full[512];
    uint64_t h = R[1], buf = R[2], i;
    uint32_t nsize = (uint32_t)R[8];
    const char *p = module_path(api, h, full, sizeof full);
    size_t len = strlen(p);

    if (nsize == 0) {
        R[0] = 0;
        return;
    }
    if (buf == 0) fle_stop(api, "lpFilename is NULL");
    if (len < nsize) {
        for (i = 0; i <= len; i++) fle_st(buf + i, 1, (unsigned char)p[i]);
        R[0] = (uint32_t)len;
        return;
    }
    for (i = 0; i + 1 < nsize; i++) fle_st(buf + i, 1, (unsigned char)p[i]);
    fle_st(buf + nsize - 1u, 1, 0);
    fle_set_last_error(FLE_W32_ERR_INSUFFICIENT_BUFFER);
    R[0] = nsize;
}

/* kernel32!GetWindowsDirectoryW and GetSystemDirectoryW (MSDN: GetWindowsDirectoryW, GetSystemDirectoryW).
 * lpBuffer rcx, uSize edx. The values are FLE_VIRTUAL_WINDOWS_DIR and FLE_VIRTUAL_SYSTEM_DIR (decisions of
 * 0018, not Windows rules); no trailing backslash (page). Return: length without NUL; when uSize is too small
 * the size needed including the NUL (the Windows page is ambiguous on whether the NUL counts; the System page
 * says it does). The directories are not created in the virtual volume. */
static void dir_answer(const char *api, const char *value)
{
    uint16_t *s = fle_w_from_ascii(value);
    R[0] = fit(api, s, fle_w_len(s), (uint32_t)R[2], R[1]);
    free(s);
}

void fle_imp_GetWindowsDirectoryW(void)
{
    dir_answer("GetWindowsDirectoryW", FLE_VIRTUAL_WINDOWS_DIR);
}

void fle_imp_GetSystemDirectoryW(void)
{
    dir_answer("GetSystemDirectoryW", FLE_VIRTUAL_SYSTEM_DIR);
}

/* version!GetFileVersionInfoSizeW (decision 0098, PROVISIONAL; decision 0129 adds the product's own module): DWORD f(LPCWSTR lptstrFilename rcx, LPDWORD lpdwHandle rdx). Learn
 * (GetFileVersionInfoSizeW): lptstrFilename is "The name of the file of interest. The function uses the search sequence specified by the
 * LoadLibrary function."; lpdwHandle is "A pointer to a variable that the function sets to zero" (optional); the return value is the size in
 * bytes of the file's version information, or zero on failure ("To get extended error information, call GetLastError").
 * Decision 0129 (S-version-info-real-resource, PM's decision): the path FLE_VIRTUAL_DLL_PATH (the guest's own engine, GetModuleFileNameW; compared with ASCII case folding) names the lifted DLL, whose PE
 * holds a real RT_VERSION resource (26.1.7.5653): the answer is the size of that resource in bytes (fle_res_version_block), *lpdwHandle = 0. The guest needs it: with 0 (the answer before decision 0129) it did
 * not know its own version ("0.0") and showed the 'created using a more recent version' warning for every project of an older version (decision 0129, wall-log). The block is the bytes of the resource, not
 * a made-up value.
 * Any other path: as before (decision 0098). This environment's file system holds only what the guest created (decision 0018), so a path that does not name a file there has no version
 * information: the answer is the failure the file system gives for that path (the same 2 or 3 as GetFileAttributesW), with *lpdwHandle set to zero. The guest's callers treat a size of 0 as "no version
 * information" (f_651ce0, f_44ada0). The page does not say if lpdwHandle is written when the call fails: it is written in every case (inference from "sets to zero").
 * Stops: a NULL name, a name with no directory part (the LoadLibrary search sequence is not modelled), and a path that exists in the virtual
 * file system (no version resource of a virtual file is modelled). */
/* Decision 0129: the RT_VERSION of the lifted DLL is read by fle_res.c. A weak reference, as fle_cursor.c has for fle_cursor_load_named (decision 0125): a build that does not link fle_res.c (most test drivers of
 * this file) stops by name for the engine path instead of failing to link. */
extern int fle_res_version_block(const char *api, uint64_t *va, uint32_t *size) __attribute__((weak));

static int is_engine_path(const uint16_t *s, size_t len)
{
    static const char want[] = FLE_VIRTUAL_DLL_PATH;
    size_t i;

    if (len != sizeof want - 1) return 0;
    for (i = 0; i < len; i++)
        if (fle_w_fold(s[i]) != fle_w_fold((uint16_t)(unsigned char)want[i])) return 0;
    return 1;
}

void fle_imp_GetFileVersionInfoSizeW(void)
{
    static const char api[] = "GetFileVersionInfoSizeW";
    uint64_t name = R[1], handle = R[2];
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    uint16_t *s;
    size_t len, i;
    int dir_part = 0, eng;

    if (name == 0) fle_stop(api, "lptstrFilename is NULL: the page does not define it (decision 0098)");
    if (handle != 0) fle_st(handle, 4, 0);
    s = fle_w_read_alloc(api, name, MAX_PATH_UNITS, &len);
    for (i = 0; i < len; i++)
        if (is_sep(s[i]) || s[i] == ':') dir_part = 1;
    eng = is_engine_path(s, len);
    free(s);
    if (!dir_part) fle_stop(api, "a file name with no directory part: the LoadLibrary search sequence is not modelled (decision 0098)");
    if (eng) {
        uint64_t va;
        uint32_t sz;

        if (!fle_res_version_block) fle_stop(api, "the path is the engine's own and no resource reader is linked (fle_res.c): decision 0129");
        if (fle_res_version_block(api, &va, &sz)) {
            R[0] = sz;
            return;
        }
        fail(FLE_W32_ERR_FILE_NOT_FOUND, 0);   /* a PE without a version resource: the page names no code; the file-system error that a missing file gives */
        return;
    }
    parse_path(api, name, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    fle_stop(api, "the path exists in the virtual file system: no version resource of a virtual file is modelled (decision 0098)");
}

/* version!GetFileVersionInfoW (decision 0112, PROVISIONAL; decision 0129 adds the product's own module): BOOL f(LPCWSTR lptstrFilename rcx, DWORD dwHandle edx, DWORD dwLen r8d, LPVOID lpData r9). Learn
 * (GetFileVersionInfoW): lptstrFilename is "The name of the file. If a full path is not specified, the function uses the search sequence specified by the
 * LoadLibrary function."; dwHandle "is ignored"; dwLen is the size of the lpData buffer ("Call the GetFileVersionInfoSize function first ... The dwLen member should be equal to or greater than
 * that value. If the buffer pointed to by lpData is not large enough, the function truncates the file's version information to the size of the buffer."); lpData "Pointer to a buffer that receives the
 * file-version information"; "If the function fails, the return value is zero. To get extended error information, call GetLastError" (the page names no code).
 * Decision 0129 (S-version-info-real-resource): for FLE_VIRTUAL_DLL_PATH the buffer receives the bytes of the RT_VERSION resource of the lifted DLL (fle_res_version_block): the first min(dwLen, size)
 * bytes ("truncates"), return 1. The form of the block is the form of the resource (a VS_VERSIONINFO): Learn does not say what GetFileVersionInfo puts in the buffer (the page says only that VerQueryValue reads it);
 * giving the resource's bytes is a CHOICE WITHOUT A SOURCE (S-version-info-block-form), made so that no value is invented; VerQueryValueW below parses exactly this form. A NULL lpData or dwLen 0 for that path
 * stops (the page does not say). Any other path: as before (the answer of GetFileVersionInfoSizeW (decision 0098) for the same path: a path that does not name a file of the virtual file system has no
 * version information, so the call fails with the file-system error of that path (the same 2 or 3 as GetFileAttributesW), lpData is not touched, dwHandle and dwLen are not looked at). Stops: a NULL name, a
 * name with no directory part (the LoadLibrary search sequence is not modelled), and a path that exists in the virtual file system (no version resource of a virtual file is modelled). */
void fle_imp_GetFileVersionInfoW(void)
{
    static const char api[] = "GetFileVersionInfoW";
    uint64_t name = R[1], data = R[9];
    uint32_t len32 = (uint32_t)R[8];
    vpath v;
    fle_node *n, *par;
    uint32_t st;
    uint16_t *s;
    size_t len, i;
    int dir_part = 0, eng;

    if (name == 0) fle_stop(api, "lptstrFilename is NULL: the page does not define it (decision 0112)");
    s = fle_w_read_alloc(api, name, MAX_PATH_UNITS, &len);
    for (i = 0; i < len; i++)
        if (is_sep(s[i]) || s[i] == ':') dir_part = 1;
    eng = is_engine_path(s, len);
    free(s);
    if (!dir_part) fle_stop(api, "a file name with no directory part: the LoadLibrary search sequence is not modelled (decisions 0098, 0112)");
    if (eng) {
        uint64_t va;
        uint32_t sz, k, cnt;

        if (!fle_res_version_block) fle_stop(api, "the path is the engine's own and no resource reader is linked (fle_res.c): decision 0129");
        if (!fle_res_version_block(api, &va, &sz)) {
            fail(FLE_W32_ERR_FILE_NOT_FOUND, 0);
            return;
        }
        if (data == 0 || len32 == 0) fle_stop(api, "lpData is NULL or dwLen is 0: the page does not say what the call does (decision 0129)");
        cnt = len32 < sz ? len32 : sz;
        for (k = 0; k < cnt; k++) fle_st(data + k, 1, (uint8_t)fle_ld(va + k, 1));
        R[0] = 1;
        return;
    }
    parse_path(api, name, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st) {
        fail(st, 0);
        return;
    }
    fle_stop(api, "the path exists in the virtual file system: no version resource of a virtual file is modelled (decisions 0098, 0112)");
}

/* version!VerQueryValueW (decision 0112, PROVISIONAL; decision 0129 answers it for the block of GetFileVersionInfoW): BOOL f(LPCVOID pBlock rcx, LPCWSTR lpSubBlock rdx, LPVOID *lplpBuffer r8, PUINT puLen r9). Learn
 * (VerQueryValueW): pBlock is "The version-information resource returned by the GetFileVersionInfo function"; lpSubBlock "must consist of names separated by backslashes (\) and it must have one of the
 * following forms": "\" = "The root block. The function retrieves a pointer to the VS_FIXEDFILEINFO structure"; "\VarFileInfo\Translation" = "The translation array in a Var variable information structure - the
 * Value member of this structure. The function retrieves a pointer to this array of language and code page identifiers"; "\StringFileInfo\lang-codepage\string-name" = "A value in a language-specific
 * StringTable structure. The lang-codepage name is a concatenation of a language and code page identifier pair found as a DWORD in the translation array for the resource. Here the lang-codepage name must be
 * specified as a hexadecimal string". *lplpBuffer = "the address of a pointer to the requested version information in the buffer pointed to by pBlock"; *puLen = "the length in characters of the string
 * stored at lplpBuffer" for a version information value, "the size in bytes of the array" for the translation array, "the size in bytes of the structure" for the root block. Return value: "If the
 * specified version-information structure exists, and version information is available, the return value is nonzero ... If the specified name does not exist or the specified resource is not valid, the return
 * value is zero."
 * The block is the one GetFileVersionInfoW wrote (the bytes of a VS_VERSIONINFO: Learn "VS_VERSIONINFO structure", "String structure", "StringTable structure", "Var structure": wLength, wValueLength, wType,
 * szKey, padding to 4 bytes, Value, padding, Children). It is parsed from the guest memory at pBlock: its first node must be the key "VS_VERSION_INFO" with a root wLength that is at least the size of a node
 * and a multiple of nothing in particular; a block that is not (a guest-made buffer or a block that is not valid) gives 0, with the page's "the specified resource is not valid". The three forms are answered as the
 * page says; a name that does not exist gives 0 (puLen and the pointer are set to 0 first: the page does not say; a choice, S-verquery-miss). A sub-block with any other shape (another number of
 * components, or a first component other than StringFileInfo/VarFileInfo) gives 0 as a name that does not exist. The lang-codepage and the string name are compared with ASCII case folding (the hex digits
 * of the guest's own "%.4x%.4x" are lower case; the PE's key is upper case: the page does not say how the comparison is made: S-verquery-fold). The pointer and puLen are in the guest's buffer (the page: "freed
 * when the associated pBlock memory is freed"). */
#define VQ_MAX_NODE 0x4000u

typedef struct {
    uint64_t at;          /* guest address of the node */
    uint32_t wlen, wvlen, wtype;
    uint64_t key;         /* guest address of szKey */
    size_t klen;          /* units, without the NUL */
    uint64_t value;       /* guest address of the Value (4-byte aligned after the key) */
    uint64_t children;    /* guest address of the first child (4-byte aligned after the value), or == end when none */
    uint64_t end;         /* at + wlen */
} vq_node;

static uint64_t vq_align4(uint64_t a) { return (a + 3u) & ~UINT64_C(3); }

/* Parses the node at `at` that must end at or before `limit`. 0 = not a valid node. */
static int vq_node_at(uint64_t at, uint64_t limit, vq_node *nd)
{
    uint64_t p;
    size_t k = 0;

    if (at + 6u > limit) return 0;
    nd->at = at;
    nd->wlen = (uint32_t)fle_ld(at, 2);
    nd->wvlen = (uint32_t)fle_ld(at + 2, 2);
    nd->wtype = (uint32_t)fle_ld(at + 4, 2);
    if (nd->wlen < 8u || nd->wlen > VQ_MAX_NODE || at + nd->wlen > limit) return 0;
    nd->end = at + nd->wlen;
    nd->key = at + 6u;
    for (p = nd->key; p + 2u <= nd->end; p += 2u, k++)
        if (fle_ld(p, 2) == 0) break;
    if (p + 2u > nd->end) return 0;
    nd->klen = k;
    nd->value = vq_align4(p + 2u);
    if (nd->value > nd->end) return 0;
    {
        /* wValueLength is a number of BYTES for a binary value (wType 0) and a number of CHARACTERS for a text one (wType 1) (Learn "String structure": "wValueLength: The size of the Value member, in words") */
        uint64_t vb = nd->wtype == 1u ? (uint64_t)nd->wvlen * 2u : (uint64_t)nd->wvlen;

        if (nd->value + vb > nd->end) return 0;
        nd->children = vq_align4(nd->value + vb);
        if (nd->children > nd->end) nd->children = nd->end;
    }
    return 1;
}

static int vq_key_is(const vq_node *nd, const uint16_t *want, size_t wl)
{
    size_t i;

    if (nd->klen != wl) return 0;
    for (i = 0; i < wl; i++)
        if (fle_w_fold((uint16_t)fle_ld(nd->key + 2u * i, 2)) != fle_w_fold(want[i])) return 0;
    return 1;
}

/* the child of `par` whose key equals want (wl units); 0 when none */
static int vq_child(const vq_node *par, const uint16_t *want, size_t wl, vq_node *out)
{
    uint64_t at = par->children;

    while (at + 6u <= par->end) {
        vq_node c;

        if (!vq_node_at(at, par->end, &c)) return 0;
        if (vq_key_is(&c, want, wl)) {
            *out = c;
            return 1;
        }
        at = vq_align4(c.end);
    }
    return 0;
}

void fle_imp_VerQueryValueW(void)
{
    static const char api[] = "VerQueryValueW";
    static const uint16_t root_key[] = { 'V','S','_','V','E','R','S','I','O','N','_','I','N','F','O' };
    static const uint16_t k_sfi[] = { 'S','t','r','i','n','g','F','i','l','e','I','n','f','o' };
    static const uint16_t k_vfi[] = { 'V','a','r','F','i','l','e','I','n','f','o' };
    static const uint16_t k_trn[] = { 'T','r','a','n','s','l','a','t','i','o','n' };
    uint64_t block = R[1], sub = R[2], pbuf = R[8], plen = R[9];
    uint16_t *s;
    size_t len, i, nc = 0, cs[8], ce[8];
    vq_node root, nd, t;
    int ok = 0;
    uint64_t rdata = 0;
    uint32_t rlen = 0;

    if (block == 0 || sub == 0 || pbuf == 0 || plen == 0) fle_stop(api, "pBlock, lpSubBlock, lplpBuffer or puLen is NULL: the page does not define it (decision 0129)");
    fle_st(pbuf, 8, 0);
    fle_st(plen, 4, 0);
    if (!vq_node_at(block, block + VQ_MAX_NODE, &root) || !vq_key_is(&root, root_key, sizeof root_key / sizeof root_key[0])) {
        R[0] = 0;   /* "the specified resource is not valid" */
        return;
    }
    s = fle_w_read_alloc(api, sub, 260u, &len);
    /* split on backslashes: the sub-block starts with one */
    if (len == 0 || s[0] != '\\') {
        free(s);
        R[0] = 0;
        return;
    }
    if (len == 1) {
        /* the root block: the VS_FIXEDFILEINFO that is the Value of the root node (wValueLength bytes) */
        free(s);
        if (root.wvlen == 0) {
            R[0] = 0;
            return;
        }
        fle_st(pbuf, 8, root.value);
        fle_st(plen, 4, root.wvlen);
        R[0] = 1;
        return;
    }
    for (i = 1; i < len && nc < 8; ) {
        size_t j = i;

        while (j < len && s[j] != '\\') j++;
        cs[nc] = i;
        ce[nc] = j;
        nc++;
        i = j + 1;
    }
    if (nc == 2 && ce[0] - cs[0] == sizeof k_vfi / sizeof k_vfi[0]) {
        /* \VarFileInfo\Translation */
        if (vq_child(&root, k_vfi, sizeof k_vfi / sizeof k_vfi[0], &nd) && ce[1] - cs[1] == sizeof k_trn / sizeof k_trn[0]) {
            int same = 1;

            for (i = 0; i < ce[1] - cs[1]; i++)
                if (fle_w_fold(s[cs[1] + i]) != fle_w_fold(k_trn[i])) same = 0;
            if (same && vq_child(&nd, k_trn, sizeof k_trn / sizeof k_trn[0], &t) && t.wvlen != 0) {
                rdata = t.value;
                rlen = t.wvlen;          /* the size in bytes of the array (wType 0: wValueLength is bytes) */
                ok = 1;
            }
        }
    } else if (nc == 3 && ce[0] - cs[0] == sizeof k_sfi / sizeof k_sfi[0]) {
        /* \StringFileInfo\lang-codepage\string-name */
        int same = 1;

        for (i = 0; i < ce[0] - cs[0]; i++)
            if (fle_w_fold(s[cs[0] + i]) != fle_w_fold(k_sfi[i])) same = 0;
        if (same && vq_child(&root, k_sfi, sizeof k_sfi / sizeof k_sfi[0], &nd)) {
            vq_node tab, val;

            if (vq_child(&nd, s + cs[1], ce[1] - cs[1], &tab) && vq_child(&tab, s + cs[2], ce[2] - cs[2], &val) && val.wvlen != 0) {
                rdata = val.value;
                rlen = val.wvlen;        /* the length in characters of the string (wType 1: wValueLength is characters; it includes the terminating NUL in a PE: the page says "the length in characters of the string") */
                ok = 1;
            }
        }
    }
    free(s);
    if (!ok) {
        R[0] = 0;
        return;
    }
    fle_st(pbuf, 8, rdata);
    fle_st(plen, 4, rlen);
    R[0] = 1;
}

/* ---- wintrust!WinVerifyTrust and the crypt32 calls behind the engine's signer-name check (decision 0117, PROVISIONAL) ----
 * What the guest does (read from FLEngine's CreateFruityInstance, 0x1375bce..0x1375ec8; measured in the walls of decision 0117): it calls WinVerifyTrust
 * with WINTRUST_ACTION_GENERIC_VERIFY_V2 and a WINTRUST_DATA for the file path that GetModuleFileNameW gave (StateAction VERIFY, then CLOSE), expects 0, then
 * CryptQueryObject(FILE, path, PKCS7_SIGNED_EMBED, BINARY) -> CryptMsgGetParam(CMSG_SIGNER_INFO_PARAM) -> CertFindCertificateInStore(CERT_FIND_SUBJECT_CERT)
 * -> CertGetNameStringW(CERT_NAME_SIMPLE_DISPLAY_TYPE) and compares the name with L"Image Line NV".
 *
 * WHAT THESE ANSWERS ARE. The file is not in the virtual file system, and this runtime verifies no signature. The harness registers CLAIMS (fle_trust_claim_add): for
 * a guest path, the result of tools/authenticode_check.py on the real file (the Authenticode digest of the file equals the signed digest, and the SignerInfo
 * signature verifies with the embedded signer certificate), and what that tool read from the certificate (the signer's CN, the issuer Name, the serial number). A
 * call answers 0 / TRUE only for a path that has an ESTABLISHED claim. The certificate chain to a root, revocation and the time are NOT verified: a real Windows
 * may answer differently (for example the signer certificate's validity starts after the virtual clock of this environment). An empty table (every build
 * whose harness registers nothing) stops every call by name, as the unanswered import did before. Nothing is read from the host here.
 *
 * Learn (WinVerifyTrust): "If the trust provider verifies that the subject is trusted for the specified action, the return value is zero. No other value
 * besides zero should be considered a successful return."; (WINTRUST_DATA) StateAction VERIFY "The hWVTStateData member will receive a handle to the state
 * data. This handle must be freed by specifying the WTD_STATEACTION_CLOSE action in a subsequent call."; CLOSE "Free the hWVTStateData member previously
 * allocated with the WTD_STATEACTION_VERIFY action."; (WINTRUST_FILE_INFO) pcwszFilePath "Full path and file name of the file to be verified. This parameter
 * cannot be NULL."; hFile and pgKnownSubject are optional. The request must not ask for more than the claim holds: revocation checking, a policy callback,
 * signature settings, a catalog or a URL reference stop by name. The value returned for CLOSE, and the contents of hWVTStateData after CLOSE, are not on the
 * pages: 0, and left as the caller has it (choices without a source).
 * Learn (CryptQueryObject): 11 arguments; "This parameter is a pointer to a null-terminated Unicode string that contains the path and name of the file to
 * query" (FILE); dwFlags "reserved for future use and must be set to zero"; phCertStore and phMsg receive handles that are freed with CertCloseStore and
 * CryptMsgClose; they and the three type outputs are optional. The numbers are those of wincrypt.h (the Learn page names the constants and gives no number;
 * a local mingw-w64 wincrypt.h was read): CERT_QUERY_OBJECT_FILE 1, CERT_QUERY_CONTENT_PKCS7_SIGNED_EMBED 10 (flag 1<<10), CERT_QUERY_FORMAT_BINARY 1 (flag 1<<1),
 * X509_ASN_ENCODING 1 | PKCS_7_ASN_ENCODING 0x10000. Only that one content/format combination is answered.
 * Learn (CryptMsgGetParam): CMSG_SIGNER_INFO_PARAM "pointer to a BYTE array to receive a CMSG_SIGNER_INFO structure. Returns information on a message
 * signer. This includes the issuer and serial number of the signer's certificate and authenticated and unauthenticated attributes"; pvData "can be NULL to
 * set the size"; pcbData is the size on entry and the number of bytes stored on return; ERROR_MORE_DATA "The specified buffer is not large enough". Only
 * dwVersion (1, CMSG_SIGNER_INFO_V1), Issuer and SerialNumber are filled (the claim holds nothing else); the algorithm identifiers, the encrypted hash and the
 * two attribute lists are zero. The guest reads only the 0x20 bytes from offset 8 (the Issuer and SerialNumber blobs). Learn (CERT_INFO): the serial number blob is
 * little-endian ("The least significant byte is the zero byte of the pbData member ... Leading 0x00 or 0xFF bytes are removed"); the claim's serial is
 * big-endian and is reversed here.
 * Learn (CertFindCertificateInStore): CERT_FIND_SUBJECT_CERT "Data type of pvFindPara: CERT_INFO structure. Searches for a certificate with both an issuer and
 * a serial number that match the issuer and serial number in the CERT_INFO structure"; no match: NULL, GetLastError CRYPT_E_NOT_FOUND (0x80092004 in
 * winerror.h); a non-NULL result "must be freed by CertFreeCertificateContext". The CERT_CONTEXT returned has dwCertEncodingType, hCertStore and nothing
 * else (the claim holds no certificate): pbCertEncoded NULL, cbCertEncoded 0, pCertInfo NULL.
 * Learn (CertGetNameStringW): CERT_NAME_SIMPLE_DISPLAY_TYPE (4) "Iterates through ... the first occurrence of: szOID_COMMON_NAME, ..."; the return value is the
 * number of characters including the terminating zero, and "If pszNameString is NULL or cchNameString is zero, returns the required size of the destination
 * string (including the terminating NULL character)". A buffer that is too small is not defined: stops. Learn (CertFreeCertificateContext): "always returns
 * nonzero". (CertCloseStore): flags 0 "close the store with memory remaining allocated for contexts that have not been freed" (TRUE). (CryptMsgClose): nonzero.
 * Tokens are never reused; a freed or foreign handle stops by name. */
#define TRUST_MAX_CLAIMS 8u
#define TRUST_MAX_OBJS   64u

#define TRUST_KIND_STATE 1
#define TRUST_KIND_STORE 2
#define TRUST_KIND_MSG   3
#define TRUST_KIND_CERT  4

#define WTD_STATEACTION_IGNORE 0u
#define WTD_STATEACTION_VERIFY 1u
#define WTD_STATEACTION_CLOSE  2u
#define WINTRUST_DATA_SIZE_X64 0x58u
#define WINTRUST_FILE_INFO_SIZE_X64 0x20u
#define CMSG_SIGNER_INFO_SIZE_X64 136u
#define CERT_CONTEXT_SIZE_X64 40u
#define ENC_X509_PKCS7 0x10001u
#define CERT_FIND_SUBJECT_CERT_ 0xB0000u
#define CERT_NAME_SIMPLE_DISPLAY_TYPE_ 4u
#define CMSG_SIGNER_INFO_PARAM_ 6u
#define ERR_CRYPT_E_NOT_FOUND 0x80092004u

typedef struct {
    char *path, *row, *why, *cn;
    uint8_t *issuer, *serial;     /* serial: unsigned big-endian, no leading zero byte except for the value 0 */
    size_t issuer_len, serial_len;
    int established;
} trust_claim;

typedef struct {
    int kind;
    int live;
    int claim;
    uint64_t token;
    uint64_t store;               /* a certificate context: the store it was found in */
} trust_obj;

static trust_claim trust_claims[TRUST_MAX_CLAIMS];
static size_t trust_nclaims;
static trust_obj trust_objs[TRUST_MAX_OBJS];
static size_t trust_nobjs;
static uint64_t trust_next_token;

static void trust_stop(const char *api, const char *fmt, ...) __attribute__((noreturn, format(printf, 2, 3)));
static void trust_stop(const char *api, const char *fmt, ...)
{
    static char msg[640];
    va_list ap;

    va_start(ap, fmt);
    vsnprintf(msg, sizeof msg, fmt, ap);
    va_end(ap);
    fle_stop(api, msg);
}

static uint8_t *trust_dup(const void *p, size_t n)
{
    uint8_t *q = malloc(n ? n : 1);

    if (!q) fle_stop("authenticode claims", "out of memory");
    if (n) memcpy(q, p, n);
    return q;
}

static int trust_ieq_ascii(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
    }
    return *a == *b;
}

int fle_trust_claim_add(const fle_trust_claim *c)
{
    size_t i, n;
    trust_claim *t;

    if (!c || !c->guest_path || !c->row || !c->signer_cn || !c->why) return -1;
    n = strlen(c->guest_path);
    if (n < 1 || n > 259) return -1;
    for (i = 0; i < n; i++)
        if ((unsigned char)c->guest_path[i] < 0x20 || (unsigned char)c->guest_path[i] > 0x7E) return -1;
    n = strlen(c->signer_cn);
    if (n > 200) return -1;
    for (i = 0; i < n; i++)
        if ((unsigned char)c->signer_cn[i] < 0x20 || (unsigned char)c->signer_cn[i] > 0x7E) return -1;
    if (c->established && (n == 0 || !c->issuer_der || c->issuer_len == 0 || !c->serial_be || c->serial_len == 0)) return -1;
    if (c->issuer_len > 1024 || c->serial_len > 64) return -1;
    if (trust_nclaims >= TRUST_MAX_CLAIMS) return -1;
    for (i = 0; i < trust_nclaims; i++)
        if (trust_ieq_ascii(trust_claims[i].path, c->guest_path)) return -1;
    t = &trust_claims[trust_nclaims];
    t->path = (char *)trust_dup(c->guest_path, strlen(c->guest_path) + 1);
    t->row = (char *)trust_dup(c->row, strlen(c->row) + 1);
    t->why = (char *)trust_dup(c->why, strlen(c->why) + 1);
    t->cn = (char *)trust_dup(c->signer_cn, strlen(c->signer_cn) + 1);
    t->issuer = trust_dup(c->issuer_der, c->issuer_len);
    t->issuer_len = c->issuer_len;
    t->serial = trust_dup(c->serial_be, c->serial_len);
    t->serial_len = c->serial_len;
    t->established = c->established ? 1 : 0;
    trust_nclaims++;
    return 0;
}

size_t fle_trust_claim_count(void) { return trust_nclaims; }

unsigned fle_trust_states_live(void)
{
    unsigned n = 0;
    size_t i;

    for (i = 0; i < trust_nobjs; i++)
        if (trust_objs[i].kind == TRUST_KIND_STATE && trust_objs[i].live) n++;
    return n;
}

unsigned fle_trust_objects_live(void)
{
    unsigned n = 0;
    size_t i;

    for (i = 0; i < trust_nobjs; i++)
        if (trust_objs[i].kind != TRUST_KIND_STATE && trust_objs[i].live) n++;
    return n;
}

static trust_obj *trust_new(const char *api, int kind, int claim, uint64_t token)
{
    trust_obj *o;

    if (trust_nobjs >= TRUST_MAX_OBJS) fle_stop(api, "too many WinVerifyTrust states and crypt32 objects in one run (the table holds 64; tokens are never reused)");
    o = &trust_objs[trust_nobjs++];
    o->kind = kind;
    o->live = 1;
    o->claim = claim;
    o->token = token;
    o->store = 0;
    return o;
}

static uint64_t trust_new_token(void)
{
    trust_next_token++;
    return FLE_TRUST_HANDLE_BASE + 4u * trust_next_token;
}

/* The live object of this kind with this token; a handle that was closed, never issued or of another kind stops. */
static trust_obj *trust_get(const char *api, int kind, uint64_t token, const char *what)
{
    size_t i;

    for (i = 0; i < trust_nobjs; i++) {
        if (trust_objs[i].token != token || trust_objs[i].kind != kind) continue;
        if (!trust_objs[i].live) trust_stop(api, "%s %#llx was already closed or freed (the pages do not define a second close; decision 0117)", what, (unsigned long long)token);
        return &trust_objs[i];
    }
    trust_stop(api, "%s %#llx was not issued by this runtime (decision 0117)", what, (unsigned long long)token);
}

/* The claim for the guest path at `guest` (UTF-16, NUL-terminated). A path that is not in the table, and a claim that is not established, stop by name. */
static int trust_claim_for(const char *api, uint64_t guest)
{
    size_t len, i, k;
    uint16_t *w = fle_w_read_alloc(api, guest, 1024, &len);
    char *a;

    a = malloc(len + 1);
    if (!a) fle_stop(api, "out of memory");
    for (i = 0; i < len; i++) a[i] = w[i] < 0x80 ? (char)w[i] : '?';
    a[len] = 0;
    free(w);
    for (k = 0; k < trust_nclaims; k++) {
        if (!trust_ieq_ascii(trust_claims[k].path, a)) continue;
        if (!trust_claims[k].established)
            trust_stop(api, "the table of authenticode claims has the row '%s' for %s, but the row is not established: %s (decision 0117; no signature is claimed for this file)",
                       trust_claims[k].row, trust_claims[k].path, trust_claims[k].why);
        free(a);
        return (int)k;
    }
    if (trust_nclaims == 0)
        trust_stop(api, "no file is vouched for: the table of authenticode claims is empty (a build without FLE_FRUITY or without FLE_AUTH_CLAIMS; decision 0117), so the signature of '%s' is not answered", a);
    trust_stop(api, "the path '%s' is not in the table of authenticode claims (%zu claim(s); decision 0117): nothing is known about this file's signature", a, trust_nclaims);
}

static int trust_guid_is_generic_verify_v2(uint64_t g)
{
    static const uint8_t d4[8] = { 0x8C, 0xC2, 0x00, 0xC0, 0x4F, 0xC2, 0x95, 0xEE };
    unsigned i;

    if (fle_ld(g, 4) != 0x00AAC56Bu || fle_ld(g + 4, 2) != 0xCD44u || fle_ld(g + 6, 2) != 0x11D0u) return 0;
    for (i = 0; i < 8; i++)
        if (fle_ld(g + 8 + i, 1) != d4[i]) return 0;
    return 1;
}

void fle_imp_WinVerifyTrust(void)
{
    static const char api[] = "WinVerifyTrust";
    uint64_t action = R[2], data = R[8], file, state_h;
    uint32_t sa;
    int claim;

    if (trust_nclaims == 0)
        fle_stop(api, "no file is vouched for: the table of authenticode claims is empty (a build without FLE_FRUITY or without FLE_AUTH_CLAIMS; decision 0117)");
    if (action == 0) fle_stop(api, "pgActionID is NULL: the page does not define it (decision 0117)");
    if (!trust_guid_is_generic_verify_v2(action)) fle_stop(api, "pgActionID is not WINTRUST_ACTION_GENERIC_VERIFY_V2 {00AAC56B-CD44-11D0-8CC2-00C04FC295EE}: no other action is modelled (decision 0117)");
    if (data == 0) fle_stop(api, "pWVTData is NULL: the page does not define it (decision 0117)");
    if (fle_ld(data, 4) != WINTRUST_DATA_SIZE_X64) trust_stop(api, "WINTRUST_DATA.cbStruct is %#llx, not 0x58 (the 64-bit structure; decision 0117)", (unsigned long long)fle_ld(data, 4));
    if (fle_ld(data + 8, 8) != 0) fle_stop(api, "pPolicyCallbackData is not NULL: a policy-specific request is not modelled (decision 0117)");
    if (fle_ld(data + 16, 8) != 0) fle_stop(api, "pSIPClientData is not NULL: not modelled (decision 0117)");
    if (fle_ld(data + 24, 4) < 1 || fle_ld(data + 24, 4) > 4) fle_stop(api, "dwUIChoice is not one of the values 1..4 of the page (decision 0117)");
    if (fle_ld(data + 28, 4) != 0) fle_stop(api, "fdwRevocationChecks asks for revocation checking: the claims table has no revocation result (decision 0117)");
    if (fle_ld(data + 32, 4) != 1) fle_stop(api, "dwUnionChoice is not WTD_CHOICE_FILE: only a file is modelled (decision 0117)");
    if (fle_ld(data + 64, 8) != 0) fle_stop(api, "pwszURLReference is not NULL: the page says it is reserved (decision 0117)");
    if (fle_ld(data + 72, 4) != 0 && fle_ld(data + 72, 4) != 0x10u) fle_stop(api, "dwProvFlags is neither 0 nor WTD_REVOCATION_CHECK_NONE: other flags change what is verified and are not modelled (decision 0117)");
    if (fle_ld(data + 80, 8) != 0) fle_stop(api, "pSignatureSettings is not NULL: not modelled (decision 0117)");
    sa = (uint32_t)fle_ld(data + 48, 4);
    state_h = fle_ld(data + 56, 8);
    if (sa == WTD_STATEACTION_CLOSE) {
        trust_obj *o = trust_get(api, TRUST_KIND_STATE, state_h, "the WinVerifyTrust state handle");

        o->live = 0;
        R[0] = 0;
        return;
    }
    if (sa != WTD_STATEACTION_VERIFY && sa != WTD_STATEACTION_IGNORE) fle_stop(api, "dwStateAction is not WTD_STATEACTION_IGNORE, VERIFY or CLOSE: the catalog actions are not modelled (decision 0117)");
    if (sa == WTD_STATEACTION_VERIFY && state_h != 0) fle_stop(api, "dwStateAction is VERIFY and hWVTStateData is not NULL: a state is already attached (the page does not define it; decision 0117)");
    file = fle_ld(data + 40, 8);
    if (file == 0) fle_stop(api, "pFile is NULL for WTD_CHOICE_FILE: the page does not define it (decision 0117)");
    if (fle_ld(file, 4) != WINTRUST_FILE_INFO_SIZE_X64) fle_stop(api, "WINTRUST_FILE_INFO.cbStruct is not 0x20 (the 64-bit structure; decision 0117)");
    if (fle_ld(file + 8, 8) == 0) fle_stop(api, "pcwszFilePath is NULL: the page says it cannot be (decision 0117)");
    if (fle_ld(file + 16, 8) != 0) fle_stop(api, "WINTRUST_FILE_INFO.hFile is not NULL: a file handle is not modelled (decision 0117)");
    if (fle_ld(file + 24, 8) != 0) fle_stop(api, "WINTRUST_FILE_INFO.pgKnownSubject is not NULL: not modelled (decision 0117)");
    claim = trust_claim_for(api, fle_ld(file + 8, 8));
    if (sa == WTD_STATEACTION_VERIFY) {
        uint64_t tok = trust_new_token();

        trust_new(api, TRUST_KIND_STATE, claim, tok);
        fle_st(data + 56, 8, tok);
    }
    R[0] = 0;
}

void fle_imp_CryptQueryObject(void)
{
    static const char api[] = "CryptQueryObject";
    uint64_t pencoding = fle_arg(6), pcontent = fle_arg(7), pformat = fle_arg(8), pstore = fle_arg(9), pmsg = fle_arg(10), pctx = fle_arg(11);
    int claim;
    uint64_t st = 0, ms = 0;

    if ((uint32_t)R[1] != 1u) fle_stop(api, "dwObjectType is not CERT_QUERY_OBJECT_FILE: a blob is not modelled (decision 0117)");
    if (R[2] == 0) fle_stop(api, "pvObject is NULL for CERT_QUERY_OBJECT_FILE: the page does not define it (decision 0117)");
    if ((uint32_t)R[8] != (1u << 10)) fle_stop(api, "dwExpectedContentTypeFlags is not CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED only: no other content type is modelled (decision 0117)");
    if ((uint32_t)R[9] != (1u << 1)) fle_stop(api, "dwExpectedFormatTypeFlags is not CERT_QUERY_FORMAT_FLAG_BINARY only: no other format is modelled (decision 0117)");
    if ((uint32_t)fle_arg(5) != 0) fle_stop(api, "dwFlags is not zero: the page says it is reserved (decision 0117)");
    if (pctx != 0) fle_stop(api, "ppvContext is not NULL: the context for an embedded PKCS #7 message is not modelled (decision 0117)");
    claim = trust_claim_for(api, R[2]);
    if (pstore) {
        trust_obj *o = trust_new(api, TRUST_KIND_STORE, claim, trust_new_token());
        st = o->token;
    }
    if (pmsg) {
        trust_obj *o = trust_new(api, TRUST_KIND_MSG, claim, trust_new_token());
        ms = o->token;
    }
    if (pencoding) fle_st(pencoding, 4, ENC_X509_PKCS7);
    if (pcontent) fle_st(pcontent, 4, 10u);   /* CERT_QUERY_CONTENT_PKCS7_SIGNED_EMBED */
    if (pformat) fle_st(pformat, 4, 1u);      /* CERT_QUERY_FORMAT_BINARY */
    if (pstore) fle_st(pstore, 8, st);
    if (pmsg) fle_st(pmsg, 8, ms);
    R[0] = 1;
}

static size_t trust_pad8(size_t n) { return (n + 7u) & ~(size_t)7u; }

static void trust_put_bytes(uint64_t dst, const uint8_t *src, size_t n)
{
    size_t i;

    for (i = 0; i < n; i++) fle_st(dst + i, 1, src[i]);
}

/* The serial number as CERT_INFO holds it: little-endian, no most-significant 0x00 bytes (a value of 0 keeps one byte). */
static size_t trust_serial_le(const trust_claim *c, uint8_t *out)
{
    size_t n = c->serial_len, i;

    for (i = 0; i < n; i++) out[i] = c->serial[n - 1 - i];
    while (n > 1 && out[n - 1] == 0) n--;
    return n;
}

void fle_imp_CryptMsgGetParam(void)
{
    static const char api[] = "CryptMsgGetParam";
    uint64_t pdata = R[9], pcb = fle_arg(5);
    trust_obj *m = trust_get(api, TRUST_KIND_MSG, R[1], "the message handle");
    const trust_claim *c = &trust_claims[m->claim];
    uint8_t ser[64];
    size_t sl, need, off_i, off_s;

    if ((uint32_t)R[2] != CMSG_SIGNER_INFO_PARAM_) fle_stop(api, "dwParamType is not CMSG_SIGNER_INFO_PARAM: no other parameter is modelled (decision 0117)");
    if ((uint32_t)R[8] != 0) fle_stop(api, "dwIndex is not 0: the claim names one signer (the page lists CRYPT_E_INVALID_INDEX, whose number it does not give; decision 0117)");
    if (pcb == 0) fle_stop(api, "pcbData is NULL: the page does not define it (decision 0117)");
    sl = trust_serial_le(c, ser);
    off_i = CMSG_SIGNER_INFO_SIZE_X64;
    off_s = off_i + trust_pad8(c->issuer_len);
    need = off_s + trust_pad8(sl);
    if (pdata == 0) {
        fle_st(pcb, 4, need);
        R[0] = 1;
        return;
    }
    if (fle_ld(pcb, 4) < need) {
        fle_st(pcb, 4, need);
        fle_set_last_error(FLE_W32_ERR_MORE_DATA);
        R[0] = 0;
        return;
    }
    {
        size_t i;

        for (i = 0; i < need; i++) fle_st(pdata + i, 1, 0);
    }
    fle_st(pdata, 4, 1u);                         /* dwVersion = CMSG_SIGNER_INFO_V1 */
    fle_st(pdata + 8, 4, c->issuer_len);          /* Issuer: CERT_NAME_BLOB */
    fle_st(pdata + 16, 8, pdata + off_i);
    fle_st(pdata + 24, 4, sl);                    /* SerialNumber: CRYPT_INTEGER_BLOB */
    fle_st(pdata + 32, 8, pdata + off_s);
    trust_put_bytes(pdata + off_i, c->issuer, c->issuer_len);
    trust_put_bytes(pdata + off_s, ser, sl);
    fle_st(pcb, 4, need);
    R[0] = 1;
}

static int trust_blob_matches_serial(uint64_t blob, const trust_claim *c)
{
    uint8_t mine[64], theirs[64];
    size_t n = trust_serial_le(c, mine), tn = fle_ld(blob, 4), i;
    uint64_t p = fle_ld(blob + 8, 8);

    if (tn > sizeof theirs || (tn != 0 && p == 0)) return 0;
    for (i = 0; i < tn; i++) theirs[i] = (uint8_t)fle_ld(p + i, 1);
    while (tn > 0 && theirs[tn - 1] == 0) tn--;
    while (n > 0 && mine[n - 1] == 0) n--;
    return n == tn && memcmp(mine, theirs, n) == 0;
}

static int trust_blob_matches_issuer(uint64_t blob, const trust_claim *c)
{
    size_t tn = fle_ld(blob, 4), i;
    uint64_t p = fle_ld(blob + 8, 8);

    if (tn != c->issuer_len || p == 0) return 0;
    for (i = 0; i < tn; i++)
        if ((uint8_t)fle_ld(p + i, 1) != c->issuer[i]) return 0;
    return 1;
}

void fle_imp_CertFindCertificateInStore(void)
{
    static const char api[] = "CertFindCertificateInStore";
    uint64_t para = fle_arg(5), prev = fle_arg(6), mem;
    trust_obj *s = trust_get(api, TRUST_KIND_STORE, R[1], "the certificate store handle");
    const trust_claim *c = &trust_claims[s->claim];
    trust_obj *o;

    if ((uint32_t)R[2] != ENC_X509_PKCS7) fle_stop(api, "dwCertEncodingType is not X509_ASN_ENCODING | PKCS_7_ASN_ENCODING (decision 0117)");
    if ((uint32_t)R[8] != 0) fle_stop(api, "dwFindFlags is not zero: not modelled (decision 0117)");
    if ((uint32_t)R[9] != CERT_FIND_SUBJECT_CERT_) fle_stop(api, "dwFindType is not CERT_FIND_SUBJECT_CERT: no other search is modelled (decision 0117)");
    if (para == 0) fle_stop(api, "pvFindPara is NULL for CERT_FIND_SUBJECT_CERT: the page does not define it (decision 0117)");
    if (prev != 0) fle_stop(api, "pPrevCertContext is not NULL: continuing a search is not modelled (decision 0117)");
    /* CERT_INFO (x64): SerialNumber at +8 (cbData, pbData at +16), Issuer at +0x30 (cbData, pbData at +0x38) */
    if (!trust_blob_matches_issuer(para + 0x30, c) || !trust_blob_matches_serial(para + 8, c)) {
        fle_set_last_error(ERR_CRYPT_E_NOT_FOUND);
        R[0] = 0;
        return;
    }
    mem = (uint64_t)(uintptr_t)calloc(1, CERT_CONTEXT_SIZE_X64);
    if (!mem) fle_stop(api, "out of memory");
    fle_st(mem, 4, ENC_X509_PKCS7);               /* dwCertEncodingType; pbCertEncoded, cbCertEncoded, pCertInfo stay 0 (the claim holds no certificate) */
    fle_st(mem + 32, 8, s->token);                /* hCertStore */
    o = trust_new(api, TRUST_KIND_CERT, s->claim, mem);
    o->store = s->token;
    R[0] = mem;
}

void fle_imp_CertGetNameStringW(void)
{
    static const char api[] = "CertGetNameStringW";
    uint64_t psz = fle_arg(5);
    uint32_t cch = (uint32_t)fle_arg(6);
    trust_obj *x = trust_get(api, TRUST_KIND_CERT, R[1], "the certificate context");
    const trust_claim *c = &trust_claims[x->claim];
    size_t len = strlen(c->cn), i;

    if ((uint32_t)R[2] != CERT_NAME_SIMPLE_DISPLAY_TYPE_) fle_stop(api, "dwType is not CERT_NAME_SIMPLE_DISPLAY_TYPE: no other name type is modelled (decision 0117)");
    if ((uint32_t)R[8] != 0) fle_stop(api, "dwFlags is not zero: CERT_NAME_ISSUER_FLAG and the others are not modelled (decision 0117)");
    if (R[9] != 0) fle_stop(api, "pvTypePara is not NULL: the page says it is not used for CERT_NAME_SIMPLE_DISPLAY_TYPE (decision 0117)");
    if (len == 0) fle_stop(api, "the claim has no common name (the first of CN, OU, O, E is not modelled; decision 0117)");
    if (psz == 0 || cch == 0) {
        R[0] = len + 1;
        return;
    }
    if (cch < len + 1) fle_stop(api, "cchNameString is smaller than the name: the page does not define a truncated result (decision 0117)");
    for (i = 0; i < len; i++) fle_st(psz + 2 * i, 2, (unsigned char)c->cn[i]);
    fle_st(psz + 2 * len, 2, 0);
    R[0] = len + 1;
}

void fle_imp_CertFreeCertificateContext(void)
{
    static const char api[] = "CertFreeCertificateContext";
    trust_obj *x;

    if (R[1] == 0) {
        R[0] = 1;
        return;
    }
    x = trust_get(api, TRUST_KIND_CERT, R[1], "the certificate context");
    x->live = 0;
    R[0] = 1;
}

void fle_imp_CertCloseStore(void)
{
    static const char api[] = "CertCloseStore";
    trust_obj *s = trust_get(api, TRUST_KIND_STORE, R[1], "the certificate store handle");

    if ((uint32_t)R[2] != 0) fle_stop(api, "dwFlags is not zero: CERT_CLOSE_STORE_CHECK_FLAG and FORCE_FLAG are not modelled (decision 0117)");
    s->live = 0;
    R[0] = 1;
}

void fle_imp_CryptMsgClose(void)
{
    static const char api[] = "CryptMsgClose";
    trust_obj *m = trust_get(api, TRUST_KIND_MSG, R[1], "the message handle");

    m->live = 0;
    R[0] = 1;
}

/* Decision 0188 (exp1, PROVISIONAL), for the G3 script command `vfsdump` (harness/fruity_g3.c): the bytes of a file on the virtual volume (a file the guest wrote), by its guest path. Returns 1 with *data / *size
 * (valid until the next write to the file; data is NULL for size 0), 0 when the path is not a file. Reads nothing into the guest and writes nothing. */
int fle_vfs_peek_file(const uint16_t *wpath, const uint8_t **data, uint64_t *size)
{
    static const char api[] = "fle_vfs_peek_file";
    vpath v;
    fle_node *n, *par;
    uint32_t st;

    parse_path(api, (uint64_t)(uintptr_t)wpath, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st || n->is_dir) return 0;
    *data = n->data;
    *size = n->size;
    return 1;
}

/* Decision 0193 (fpick1, PROVISIONAL), for the browser file dialog bridge (fle_filedlg.c): files that the USER chose in the page are put on the virtual volume from the host side, and a file
 * that the engine saved is looked at. Nothing here is reached by the guest's own calls, and nothing changes for a build that does not call it. */

/* The naming rules of check_component, answered instead of stopped (the name comes from a person's file system, not from the guest). */
static int put_name_ok(const uint16_t *c, size_t cl)
{
    size_t k;
    if (cl == 0 || cl > MAX_COMPONENT) return 0;
    for (k = 0; k < cl; k++) {
        uint16_t ch = c[k];
        if (ch < 32 || ch == '<' || ch == '>' || ch == '"' || ch == '|' || ch == ':' || ch == '*' || ch == '?' || ch == '/' || ch == '\\') return 0;
    }
    if (c[cl - 1] == ' ' || c[cl - 1] == '.') return 0;
    if ((cl == 1 && c[0] == '.') || (cl == 2 && c[0] == '.' && c[1] == '.')) return 0;
    return !is_device(c, cl);
}

/* Walks (and with `make` creates) the directories of "C:\a\b\...". `want_leaf` = the last component is a file name that the caller handles: *leaf / *leaf_len say where it starts and the node returned is its
 * directory. Returns the directory node, or NULL: *err = 1 invalid path, 5 a component is a file. */
static fle_node *put_walk(const uint16_t *w, int want_leaf, const uint16_t **leaf, size_t *leaf_len, int *err)
{
    size_t len = 0, i = 3;
    fle_node *cur = &root_node;

    *err = 0;
    while (w[len]) len++;
    if (len < 3 || len > MAX_PATH_UNITS || fle_w_fold(w[0]) != 'c' || w[1] != ':' || w[2] != '\\') { *err = 1; return NULL; }
    while (i < len) {
        size_t j = i, cl;
        fle_node *k;
        while (j < len && w[j] != '\\') j++;
        cl = j - i;
        if (!put_name_ok(w + i, cl)) { *err = 1; return NULL; }
        if (want_leaf && j == len) { *leaf = w + i; *leaf_len = cl; return cur; }
        k = find_kid(cur, w + i, cl);
        if (k == NULL) k = node_new(cur, w + i, cl, 1, 0);
        else if (!k->is_dir) { *err = 5; return NULL; }
        cur = k;
        i = j + 1;
    }
    if (want_leaf) { *err = 1; return NULL; }   /* "C:\" or a trailing separator: no file name */
    return cur;
}

/* Creates the directory "C:\a\b" and the ones above it. 0 done (or it existed), 1 invalid path, 5 a component is a file. */
int fle_vfs_make_dirs(const uint16_t *wdir)
{
    int err;
    (void)put_walk(wdir, 0, NULL, NULL, &err);
    return err;
}

/* Puts a file on the volume: the path "C:\dir\name" (missing directories are created), the content `data` (size bytes, from malloc: THE VOLUME TAKES IT OVER, also when the call fails; NULL with size 0 is an empty file).
 * An existing file with no open handle is replaced. 0 done; 1 invalid path; 2 the target is a directory; 3 the target is open; 4 larger than the volume's 256 MiB limit; 5 a directory component is a file. */
int fle_vfs_put_file(const uint16_t *wpath, uint8_t *data, uint64_t size)
{
    const uint16_t *leaf = NULL;
    size_t leaf_len = 0;
    int err;
    fle_node *dir, *n;

    if (size > MAX_FILE_BYTES) { free(data); return 4; }
    dir = put_walk(wpath, 1, &leaf, &leaf_len, &err);
    if (dir == NULL) { free(data); return err; }
    n = find_kid(dir, leaf, leaf_len);
    if (n != NULL && n->is_dir) { free(data); return 2; }
    if (n != NULL && (n->opens || n->pending)) { free(data); return 3; }
    if (n == NULL) n = node_new(dir, leaf, leaf_len, 0, 0);
    else { free(n->data); n->wt = n->at = tick(); }
    n->data = size ? data : NULL;
    if (!size) free(data);
    n->size = n->cap = size;
    return 0;
}

/* The state of a file of the volume, by its UTF-16 guest path: 1 and the size, the number of open handles, the last-write time (FILETIME ticks) and the bytes (valid until the next write; NULL for size 0); 0 when
 * the path is not a file or does not exist. The path comes from the caller's own record (a path the engine was handed), so a malformed one stops by name as everywhere else in this file. */
int fle_vfs_file_state(const uint16_t *wpath, uint64_t *size, unsigned *opens, uint64_t *wt, const uint8_t **data)
{
    static const char api[] = "fle_vfs_file_state";
    vpath v;
    fle_node *n, *par;
    uint32_t st;

    parse_path(api, (uint64_t)(uintptr_t)wpath, 0, &v);
    st = lookup(api, &v, &n, &par);
    vp_free(&v);
    if (st || n->is_dir) return 0;
    *size = n->size;
    *opens = n->opens;
    *wt = n->wt;
    *data = n->data;
    return 1;
}
