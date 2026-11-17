/* Stage 3 of decision 0216 (opfs1, PROVISIONAL): the table of mounts and the Windows -> OPFS path translation. See fle_opfs_mount.h for the rules. Pure C (no OS, no browser). */
#include <string.h>
#include "fle_opfs_mount.h"

#define MAX_COMPS 256

typedef struct {
    uint16_t *units;       /* the component, UTF-16, not NUL-terminated, points into the working copy */
    size_t len;
} comp;

typedef struct {
    int used;
    char drive;            /* upper case */
    unsigned ncomp;
    uint16_t guest[FLE_OPFS_MAX_MOUNT_PREFIX + 1];   /* normalized guest prefix, kept for the reverse translation */
    size_t guest_len;
    comp comps[MAX_COMPS];
    char opfs[1024];
} mount;

static mount mounts[FLE_OPFS_MAX_MOUNTS];

/* simple case folding of UTF-16 units: ASCII, Latin-1, Greek, Cyrillic, fullwidth Latin (not the whole NTFS $UpCase table: S-opfs1-casefold) */
static uint16_t fold(uint16_t c)
{
    if (c >= 'A' && c <= 'Z') return (uint16_t)(c + 32);
    if (c >= 0xC0 && c <= 0xDE && c != 0xD7) return (uint16_t)(c + 32);
    if (c >= 0x391 && c <= 0x3A9) return (uint16_t)(c + 32);
    if (c >= 0x410 && c <= 0x42F) return (uint16_t)(c + 32);
    if (c >= 0x400 && c <= 0x40F) return (uint16_t)(c + 80);
    if (c >= 0xFF21 && c <= 0xFF3A) return (uint16_t)(c + 32);
    return c;
}

static int comp_eq(const comp *a, const comp *b)
{
    size_t i;
    if (a->len != b->len) return 0;
    for (i = 0; i < a->len; i++) if (fold(a->units[i]) != fold(b->units[i])) return 0;
    return 1;
}

static int is_device(const uint16_t *u, size_t len)
{
    size_t stem = 0, i;
    char s[8];
    static const char *const names[] = { "con", "prn", "aux", "nul" };
    while (stem < len && u[stem] != '.') stem++;
    while (stem > 0 && u[stem - 1] == ' ') stem--;
    if (stem < 3 || stem > 4) return 0;
    for (i = 0; i < stem; i++) { uint16_t c = fold(u[i]); if (c > 127) return 0; s[i] = (char)c; }
    s[stem] = 0;
    for (i = 0; i < 4; i++) if (stem == 3 && strcmp(s, names[i]) == 0) return 1;
    if (stem == 4 && s[3] >= '1' && s[3] <= '9' && (strncmp(s, "com", 3) == 0 || strncmp(s, "lpt", 3) == 0)) return 1;
    return 0;
}

/* the top-level OPFS folder the page keeps for itself (FLE_OPFS_RESERVED, "fl": /fl/state, /fl/license, the C: tree): a mount of the OPFS root does not show it to the guest */
static int is_reserved(const comp *c)
{
    static const uint16_t r[] = { 'f', 'l' };
    return c->len == 2 && fold(c->units[0]) == r[0] && fold(c->units[1]) == r[1];
}

#define NOT_ABSOLUTE 1   /* internal: not a drive-letter path */

/* normalizes `path` into `work` (a copy) and comps[]; returns 0, NOT_ABSOLUTE or a negative error (*ncomp then holds the components before the fault: a fault is reported only for a path under a mount); *drive = upper-case letter; *long_form = \\?\ prefix; *norm_len = length of the normalized path (C:\a\b) */
static int normalize(const uint16_t *path, size_t len, uint16_t *work, comp *comps, unsigned *ncomp, char *drive, int *long_form, size_t *norm_len, int *device)
{
    size_t i = 0, w = 0, start;
    unsigned n = 0;
    int over = 0;
    *long_form = 0; *device = 0;
    if (len >= 4 && path[0] == '\\' && path[1] == '\\' && path[2] == '?' && path[3] == '\\') { *long_form = 1; i = 4; }
    *ncomp = 0;
    if (len - i < 3 || !((path[i] >= 'A' && path[i] <= 'Z') || (path[i] >= 'a' && path[i] <= 'z')) || path[i + 1] != ':' || (path[i + 2] != '\\' && path[i + 2] != '/')) return NOT_ABSOLUTE;
    *drive = (char)(path[i] >= 'a' ? path[i] - 32 : path[i]);
    if (len > FLE_OPFS_MAX_PATH_LONG + 4u) { over = 1; len = FLE_OPFS_MAX_PATH_LONG + 4u; }   /* the rest is not read, but the path is still known to be under a mount or not */
    i += 2;
    while (i < len) {
        size_t cl;
        while (i < len && (path[i] == '\\' || path[i] == '/')) i++;
        if (i >= len) break;
        start = w;
        while (i < len && path[i] != '\\' && path[i] != '/') {
            uint16_t c = path[i];
            if (c < 0x20 || c == '<' || c == '>' || c == ':' || c == '"' || c == '|' || c == '?' || c == '*') { *ncomp = n; return FLE_OPFS_E_INVALID; }
            if (c >= 0xD800 && c <= 0xDBFF) { if (i + 1 >= len || path[i + 1] < 0xDC00 || path[i + 1] > 0xDFFF) { *ncomp = n; return FLE_OPFS_E_INVALID; } work[w++] = c; i++; c = path[i]; }
            else if (c >= 0xDC00 && c <= 0xDFFF) { *ncomp = n; return FLE_OPFS_E_INVALID; }
            work[w++] = c; i++;
        }
        cl = w - start;
        if (*long_form && ((cl == 1 && work[start] == '.') || (cl == 2 && work[start] == '.' && work[start + 1] == '.'))) { *ncomp = n; return FLE_OPFS_E_INVALID; }   /* literal in \\?\ form, and no OPFS name */
        if (!*long_form) {
            if (cl == 1 && work[start] == '.') { w = start; continue; }
            if (cl == 2 && work[start] == '.' && work[start + 1] == '.') { w = start; if (n > 0) { n--; w = (size_t)(comps[n].units - work); } continue; }
            while (cl > 0 && (work[start + cl - 1] == '.' || work[start + cl - 1] == ' ')) cl--;   /* Win32 strips trailing dots and spaces */
            if (cl == 0) { *ncomp = n; return FLE_OPFS_E_INVALID; }
            w = start + cl;
        }
        if (cl > FLE_OPFS_MAX_COMPONENT) { *ncomp = n; return FLE_OPFS_E_TOO_LONG; }
        if (!*long_form && is_device(work + start, cl)) *device = 1;
        if (n >= MAX_COMPS) { *ncomp = n; return FLE_OPFS_E_TOO_LONG; }
        comps[n].units = work + start; comps[n].len = cl; n++;
    }
    *ncomp = n;
    if (over) return FLE_OPFS_E_TOO_LONG;
    *norm_len = 2;
    for (i = 0; i < n; i++) *norm_len += 1 + comps[i].len;
    if (n == 0) *norm_len = 3;
    if (!*long_form && *norm_len > FLE_OPFS_MAX_PATH) return FLE_OPFS_E_TOO_LONG;
    return 0;
}

void fle_opfs_mount_clear(void) { memset(mounts, 0, sizeof mounts); }
int fle_opfs_mount_count(void) { int i, n = 0; for (i = 0; i < FLE_OPFS_MAX_MOUNTS; i++) if (mounts[i].used) n++; return n; }

int fle_opfs_mount_add(const uint16_t *guest_prefix, const char *opfs_prefix)
{
    static uint16_t work[FLE_OPFS_MAX_PATH_LONG + 8];
    comp comps[MAX_COMPS];
    unsigned n = 0, k;
    char drive = 0;
    int long_form, device, rc, slot = -1, i;
    size_t norm = 0, len = 0, p, o = 0;
    mount *m;

    while (guest_prefix[len]) len++;
    rc = normalize(guest_prefix, len, work, comps, &n, &drive, &long_form, &norm, &device);
    if (rc == NOT_ABSOLUTE) return FLE_OPFS_E_INVALID;
    if (rc < 0) return rc;
    if (device || norm > FLE_OPFS_MAX_MOUNT_PREFIX || strlen(opfs_prefix) >= sizeof mounts[0].opfs) return FLE_OPFS_E_INVALID;
    for (i = 0; i < FLE_OPFS_MAX_MOUNTS; i++) if (!mounts[i].used) { slot = i; break; }
    if (slot < 0) return -5;
    m = &mounts[slot];
    memset(m, 0, sizeof *m);
    m->used = 1; m->drive = drive; m->ncomp = n;
    m->guest[o++] = (uint16_t)drive; m->guest[o++] = ':';
    for (k = 0; k < n; k++) {
        m->guest[o++] = '\\';
        for (p = 0; p < comps[k].len; p++) m->guest[o++] = comps[k].units[p];
    }
    if (n == 0) m->guest[o++] = '\\';
    m->guest[o] = 0; m->guest_len = o;
    /* the components point into the mount's own copy */
    {
        size_t q = n == 0 ? 3 : 2;
        for (k = 0; k < n; k++) { q++; m->comps[k].units = m->guest + q; m->comps[k].len = comps[k].len; q += comps[k].len; }
    }
    strcpy(m->opfs, opfs_prefix);
    return 0;
}

static size_t utf8_of(const uint16_t *u, size_t len, char *out, size_t cap)
{
    size_t i, o = 0;
    for (i = 0; i < len; i++) {
        uint32_t cp = u[i];
        if (cp >= 0xD800 && cp < 0xDC00) { cp = 0x10000 + ((cp - 0xD800) << 10) + (u[i + 1] - 0xDC00); i++; }
        if (cp < 0x80) { if (o + 1 > cap) return (size_t)-1; out[o++] = (char)cp; }
        else if (cp < 0x800) { if (o + 2 > cap) return (size_t)-1; out[o++] = (char)(0xC0 | (cp >> 6)); out[o++] = (char)(0x80 | (cp & 0x3F)); }
        else if (cp < 0x10000) { if (o + 3 > cap) return (size_t)-1; out[o++] = (char)(0xE0 | (cp >> 12)); out[o++] = (char)(0x80 | ((cp >> 6) & 0x3F)); out[o++] = (char)(0x80 | (cp & 0x3F)); }
        else { if (o + 4 > cap) return (size_t)-1; out[o++] = (char)(0xF0 | (cp >> 18)); out[o++] = (char)(0x80 | ((cp >> 12) & 0x3F)); out[o++] = (char)(0x80 | ((cp >> 6) & 0x3F)); out[o++] = (char)(0x80 | (cp & 0x3F)); }
    }
    return o;
}

int fle_opfs_translate(const uint16_t *guest_path, size_t len, char *out, size_t cap)
{
    static uint16_t work[FLE_OPFS_MAX_PATH_LONG + 8];
    comp comps[MAX_COMPS];
    unsigned n = 0, k;
    char drive = 0;
    int long_form, device, rc, i, best = -1;
    unsigned best_n = 0;
    size_t norm, o = 0;

    if (cap == 0) return FLE_OPFS_E_SPACE;
    rc = normalize(guest_path, len, work, comps, &n, &drive, &long_form, &norm, &device);
    if (rc == NOT_ABSOLUTE) return FLE_OPFS_NOT_MOUNTED;
    for (i = 0; i < FLE_OPFS_MAX_MOUNTS; i++) {
        mount *m = &mounts[i];
        if (!m->used || m->drive != drive || m->ncomp > n) continue;
        for (k = 0; k < m->ncomp; k++) if (!comp_eq(&m->comps[k], &comps[k])) break;
        if (k == m->ncomp && (best < 0 || m->ncomp > best_n)) { best = i; best_n = m->ncomp; }
    }
    if (best < 0) return FLE_OPFS_NOT_MOUNTED;
    if (rc < 0) return rc;
    if (device) return FLE_OPFS_E_DEVICE;
    if (mounts[best].opfs[0] == 0 && n > best_n && is_reserved(&comps[best_n])) return FLE_OPFS_E_HIDDEN;
    {
        size_t pl = strlen(mounts[best].opfs);
        if (pl + 1 > cap) return FLE_OPFS_E_SPACE;
        memcpy(out, mounts[best].opfs, pl);
        o = pl;
    }
    for (k = best_n; k < n; k++) {
        size_t w;
        if (o > 0) { if (o + 1 >= cap) return FLE_OPFS_E_SPACE; out[o++] = '/'; }
        w = utf8_of(comps[k].units, comps[k].len, out + o, cap - o - 1);
        if (w == (size_t)-1) return FLE_OPFS_E_SPACE;
        o += w;
    }
    out[o] = 0;
    return FLE_OPFS_MAPPED;
}

int fle_opfs_guest_path(int index, const char *opfs_path, uint16_t *out, size_t cap)
{
    mount *m = NULL;
    size_t o = 0, pl, i;
    int j, seen = 0;
    for (j = 0; j < FLE_OPFS_MAX_MOUNTS; j++) if (mounts[j].used) { if (seen++ == index) { m = &mounts[j]; break; } }
    if (!m) return FLE_OPFS_E_INVALID;
    pl = strlen(m->opfs);
    if (strncmp(opfs_path, m->opfs, pl) != 0 || (pl > 0 && opfs_path[pl] != 0 && opfs_path[pl] != '/')) return FLE_OPFS_E_INVALID;
    opfs_path += pl;
    if (pl > 0 && *opfs_path == '/') opfs_path++;
    if (pl == 0 && (opfs_path[0] == 'f' || opfs_path[0] == 'F') && (opfs_path[1] == 'l' || opfs_path[1] == 'L') && (opfs_path[2] == 0 || opfs_path[2] == '/')) return FLE_OPFS_E_HIDDEN;
    if (m->guest_len + 1 > cap) return FLE_OPFS_E_SPACE;
    memcpy(out, m->guest, m->guest_len * sizeof(uint16_t));
    o = m->guest_len;
    if (*opfs_path && m->guest[o - 1] != '\\') out[o++] = '\\';
    for (i = 0; opfs_path[i];) {
        uint32_t cp; unsigned char c = (unsigned char)opfs_path[i];
        if (c < 0x80) { cp = c; i += 1; }
        else if ((c & 0xE0) == 0xC0 && opfs_path[i + 1]) { cp = ((uint32_t)(c & 0x1F) << 6) | (opfs_path[i + 1] & 0x3F); i += 2; }
        else if ((c & 0xF0) == 0xE0 && opfs_path[i + 1] && opfs_path[i + 2]) { cp = ((uint32_t)(c & 0x0F) << 12) | ((uint32_t)(opfs_path[i + 1] & 0x3F) << 6) | (opfs_path[i + 2] & 0x3F); i += 3; }
        else if ((c & 0xF8) == 0xF0 && opfs_path[i + 1] && opfs_path[i + 2] && opfs_path[i + 3]) { cp = ((uint32_t)(c & 0x07) << 18) | ((uint32_t)(opfs_path[i + 1] & 0x3F) << 12) | ((uint32_t)(opfs_path[i + 2] & 0x3F) << 6) | (opfs_path[i + 3] & 0x3F); i += 4; }
        else return FLE_OPFS_E_INVALID;
        if (cp == '/') cp = '\\';
        if (cp >= 0x10000) { if (o + 3 > cap) return FLE_OPFS_E_SPACE; cp -= 0x10000; out[o++] = (uint16_t)(0xD800 + (cp >> 10)); out[o++] = (uint16_t)(0xDC00 + (cp & 0x3FF)); }
        else { if (o + 2 > cap) return FLE_OPFS_E_SPACE; out[o++] = (uint16_t)cp; }
    }
    out[o] = 0;
    return (int)o;
}
