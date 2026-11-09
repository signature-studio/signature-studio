/* Seeding of the virtual volume with product files of the installer (decision 0110, PROVISIONAL).
 * Header-only helper of lift/harness/two/main_two.c and lift/harness/census/main_two.c; it is not part of the runtime and the runtime
 * gets no new code: the files are written through the guest-facing C stubs of lift/runtime/fle_vfs.c (CreateDirectoryW, CreateFileW,
 * WriteFile, SetFilePointer, ReadFile, CloseHandle, GetFileAttributesW), called before the entry, exactly as lifted code would call them.
 *
 * Off by default. FLE_VFS_SEED unset or empty: vfs_seed_from_env() returns 0 at once, prints nothing and touches nothing, so the run is
 * the one of a harness without this file (the volume is empty, decision 0018).
 *
 * FLE_VFS_SEED=<list file>. One entry per line, three TAB-separated fields:
 *     <guest path> TAB <host file> TAB <SHA-256 of the host file, 64 hex digits>
 * Empty lines and lines that start with '#' are ignored.
 *   guest path  C:\dir\...\file: drive C: only, backslashes only, ASCII, no '.' or '..' component, at most 259 characters. The directories
 *               above the file are created when they are missing. This is the only thing the guest can see of the host: the files of
 *               the list, at these paths, and the directories on the way.
 *   host file   an absolute path, or a path relative to $FLE_VFS_SEED_ROOT (an absolute directory; a relative path without it, or with a
 *               ".." component, is refused). The repository holds no absolute host path.
 *   SHA-256     checked before anything is written: every entry is read and hashed first, and one mismatch (or an unreadable file, or a
 *               file over 64 MiB, or a duplicate guest path) stops the harness by name, with nothing written to the volume.
 * Decision 0129 (PROVISIONAL, an extension of this mechanism): a line whose guest path ends with a backslash and whose two other fields are both the single character '-' makes the directory (and the
 * directories above it) without any file in it: the installed product has directories that hold files this list does not give, and the guest looks at the directory itself (GetFileAttributesW,
 * FindFirstFileW). The limit of entries is VS_MAX_ENTRIES (2,048 since decision 0129; it was 256).
 * Success prints "vfs seed:" lines on stdout. Any failure prints "vfs-seed: ..." on stderr and vfs_seed_from_env() returns -1 (the
 * caller exits 2 before the guest runs).
 *
 * What the seeding changes for the guest, besides the files and directories themselves (decision 0110, section "Side effects"): the
 * virtual clock of fle_vfs.c has advanced by one second per change, and the file handles issued afterwards start above the ones used here
 * (handle values are never reused). R[], and the last error, are put back as they were (R[] is saved and restored; the last error is set to 0). */
#ifndef FLE_HARNESS_VFS_SEED_H
#define FLE_HARNESS_VFS_SEED_H

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "fle_imports.h"
#include "fle_runtime.h"
#include "fle_vfs.h"

#define VFS_SEED_ENV "FLE_VFS_SEED"
#define VFS_SEED_ROOT_ENV "FLE_VFS_SEED_ROOT"

#define VS_MAX_ENTRIES 2048   /* decision 0129: was 256 */
#define VS_MAX_FILE_BYTES (UINT64_C(64) << 20)
#define VS_GUEST_MAX 259        /* UTF-16 code units of a guest path (decision 0178: the list is UTF-8, so a path of up to 259 units can take 3 bytes per unit) */
#define VS_GUEST_BYTES (VS_GUEST_MAX * 3 + 1)
#define VS_HOST_MAX 4096
#define VS_LINE_MAX 8192
#define VS_ATTR_DIRECTORY 0x10u
#define VS_GENERIC_READ_WRITE 0xC0000000u
#define VS_CREATE_NEW 1u
#define VS_ATTR_NORMAL 0x80u
#define VS_CALL_RETURN_ADDRESS UINT64_C(0xdead5eed)

typedef struct {
    char guest[VS_GUEST_BYTES];
    char host[VS_HOST_MAX];
    char sha[65];
    unsigned line;
    uint8_t *data;
    size_t size;
    int is_dir;      /* decision 0129: a directory line (guest path ends with a backslash; host and SHA-256 are '-') */
} vs_entry;

/* ---- SHA-256 (FIPS 180-4), one shot. The harness links no crypto library. ---- */

static const uint32_t vs_k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
};

static uint32_t vs_ror(uint32_t x, unsigned r)
{
    return (x >> r) | (x << (32u - r));
}

static void vs_block(uint32_t h[8], const uint8_t *p)
{
    uint32_t w[64], a, b, c, d, e, f, g, hh;
    unsigned i;
    for (i = 0; i < 16; i++)
        w[i] = ((uint32_t)p[4 * i] << 24) | ((uint32_t)p[4 * i + 1] << 16) | ((uint32_t)p[4 * i + 2] << 8) | (uint32_t)p[4 * i + 3];
    for (i = 16; i < 64; i++) {
        uint32_t s0 = vs_ror(w[i - 15], 7) ^ vs_ror(w[i - 15], 18) ^ (w[i - 15] >> 3);
        uint32_t s1 = vs_ror(w[i - 2], 17) ^ vs_ror(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    a = h[0]; b = h[1]; c = h[2]; d = h[3]; e = h[4]; f = h[5]; g = h[6]; hh = h[7];
    for (i = 0; i < 64; i++) {
        uint32_t big1 = vs_ror(e, 6) ^ vs_ror(e, 11) ^ vs_ror(e, 25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t t1 = hh + big1 + ch + vs_k[i] + w[i];
        uint32_t big0 = vs_ror(a, 2) ^ vs_ror(a, 13) ^ vs_ror(a, 22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = big0 + maj;
        hh = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    h[0] += a; h[1] += b; h[2] += c; h[3] += d; h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
}

/* hex receives 64 lower-case hex digits and a NUL. */
static void vs_sha256_hex(const uint8_t *data, size_t n, char hex[65])
{
    uint32_t h[8] = { 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19 };
    uint8_t tail[128];
    size_t full = n / 64, rem = n % 64, tl, i;
    uint64_t bits = (uint64_t)n * 8u;
    for (i = 0; i < full; i++) vs_block(h, data + 64 * i);
    memset(tail, 0, sizeof tail);
    if (rem) memcpy(tail, data + 64 * full, rem);
    tail[rem] = 0x80;
    tl = rem < 56 ? 64 : 128;
    for (i = 0; i < 8; i++) tail[tl - 1 - i] = (uint8_t)(bits >> (8 * i));
    vs_block(h, tail);
    if (tl == 128) vs_block(h, tail + 64);
    for (i = 0; i < 8; i++) snprintf(hex + 8 * i, 9, "%08x", (unsigned)h[i]);
}

/* ---- errors ---- */

static int vs_fail(const char *list, unsigned line, const char *fmt, ...) __attribute__((format(printf, 3, 4)));
static int vs_fail(const char *list, unsigned line, const char *fmt, ...)
{
    va_list ap;
    if (line) fprintf(stderr, "vfs-seed: %s:%u: ", list, line);
    else fprintf(stderr, "vfs-seed: %s: ", list);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    return -1;
}

/* ---- the list ---- */

/* Decision 0178 (PROVISIONAL): a guest path in the list is UTF-8 (a pure ASCII path is exactly what it was). vs_utf8_units() checks the bytes[0..n) and counts the UTF-16 code units they make: 0 on success with
 * *units set, -1 for an invalid sequence (a stray continuation byte, a truncated or overlong sequence, a surrogate code point U+D800..U+DFFF, a code point above U+10FFFF). The 1 to 3 byte forms are one unit, the
 * 4 byte form two. */
static int vs_utf8_units(const char *s, size_t n, size_t *units)
{
    size_t i = 0, u = 0;
    while (i < n) {
        unsigned char c = (unsigned char)s[i];
        uint32_t cp;
        unsigned extra, k;
        if (c < 0x80) { cp = c; extra = 0; }
        else if (c >= 0xC2 && c <= 0xDF) { cp = c & 0x1F; extra = 1; }
        else if (c >= 0xE0 && c <= 0xEF) { cp = c & 0x0F; extra = 2; }
        else if (c >= 0xF0 && c <= 0xF4) { cp = c & 0x07; extra = 3; }
        else return -1;
        for (k = 1; k <= extra; k++) {
            if (i + k >= n || ((unsigned char)s[i + k] & 0xC0) != 0x80) return -1;
            cp = (cp << 6) | ((unsigned char)s[i + k] & 0x3F);
        }
        if ((extra == 2 && cp < 0x800) || (extra == 3 && cp < 0x10000) || cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF)) return -1;
        u += cp >= 0x10000 ? 2 : 1;
        i += extra + 1;
    }
    *units = u;
    return 0;
}

static int vs_ieq(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
    }
    return *a == *b;
}

/* The guest path rules of the list (stricter than fle_vfs.c, which stops by name on anything outside the Microsoft naming rules).
 * Returns NULL when the path is acceptable, else the reason. */
static const char *vs_guest_problem(const char *g)
{
    size_t n = strlen(g), i, comp = 3, units;
    if (vs_utf8_units(g, n, &units) != 0) return "is not valid UTF-8";   /* decision 0178; ASCII is valid UTF-8 and has units == n */
    if (units > VS_GUEST_MAX) return "longer than 259 characters";
    if (n < 4 || (g[0] != 'C' && g[0] != 'c') || g[1] != ':' || g[2] != '\\') return "does not start with C:\\";
    for (i = 3; i <= n; i++) {
        if (g[i] == '\\' || g[i] == 0) {
            size_t cl = i - comp, cu;
            const char *c = g + comp;
            if (cl == 0) return "has an empty component (or ends with a backslash)";
            (void)vs_utf8_units(c, cl, &cu);   /* valid: the whole path is */
            if (cu > 255) return "has a component longer than 255 characters";
            if ((cl == 1 && c[0] == '.') || (cl == 2 && c[0] == '.' && c[1] == '.')) return "has a '.' or '..' component";
            if (c[cl - 1] == ' ' || c[cl - 1] == '.') return "has a component that ends with a space or a period";
            comp = i + 1;
        } else {
            unsigned char ch = (unsigned char)g[i];
            /* bytes 0x80..0xFF belong to a multi-byte sequence that vs_utf8_units() accepted (decision 0178); the ASCII rule is as before */
            if (ch < 0x20 || ch == 0x7f || ch == '<' || ch == '>' || ch == ':' || ch == '"' || ch == '/' || ch == '|' || ch == '?' || ch == '*')
                return "has a character outside printable ASCII or one of < > : \" / | ? *";
        }
    }
    return NULL;
}

static int vs_resolve_host(const char *list, unsigned line, const char *in, char *out, size_t cap)
{
    int n;
    if (in[0] == 0) return vs_fail(list, line, "empty host file name");
    if (in[0] == '/') {
        n = snprintf(out, cap, "%s", in);
    } else {
        const char *root = getenv(VFS_SEED_ROOT_ENV);
        const char *p;
        size_t rl;
        if (!root || !root[0]) return vs_fail(list, line, "the host file '%s' is relative and %s is not set", in, VFS_SEED_ROOT_ENV);
        if (root[0] != '/') return vs_fail(list, line, "%s must be an absolute directory: %s", VFS_SEED_ROOT_ENV, root);
        for (p = in; *p;) {
            const char *e = strchr(p, '/');
            size_t cl = e ? (size_t)(e - p) : strlen(p);
            if (cl == 2 && p[0] == '.' && p[1] == '.') return vs_fail(list, line, "the relative host file '%s' has a '..' component", in);
            p += cl;
            if (*p == '/') p++;
        }
        rl = strlen(root);
        while (rl > 1 && root[rl - 1] == '/') rl--;
        n = snprintf(out, cap, "%.*s/%s", (int)rl, root, in);
    }
    if (n < 0 || (size_t)n >= cap) return vs_fail(list, line, "the host file name is too long");
    return 0;
}

static int vs_read_host(const char *list, unsigned line, vs_entry *e)
{
    struct stat st;
    FILE *f;
    size_t got;
    if (stat(e->host, &st) != 0) return vs_fail(list, line, "cannot stat the host file %s", e->host);
    if (!S_ISREG(st.st_mode)) return vs_fail(list, line, "the host file %s is not a regular file", e->host);
    if ((uint64_t)st.st_size > VS_MAX_FILE_BYTES) return vs_fail(list, line, "the host file %s is larger than 64 MiB", e->host);
    e->size = (size_t)st.st_size;
    e->data = malloc(e->size ? e->size : 1);
    if (!e->data) return vs_fail(list, line, "out of memory for %s", e->host);
    f = fopen(e->host, "rb");
    if (!f) return vs_fail(list, line, "cannot open the host file %s", e->host);
    got = e->size ? fread(e->data, 1, e->size, f) : 0;
    if (got != e->size || fgetc(f) != EOF) {
        fclose(f);
        return vs_fail(list, line, "the host file %s changed size or could not be read whole", e->host);
    }
    fclose(f);
    return 0;
}

static int vs_parse(const char *list, vs_entry *ent, size_t *count)
{
    FILE *f = fopen(list, "r");
    char buf[VS_LINE_MAX];
    unsigned line = 0;
    size_t n = 0;
    if (!f) return vs_fail(list, 0, "cannot open the list");
    while (fgets(buf, sizeof buf, f)) {
        char *t1, *t2, *p;
        size_t len = strlen(buf), i;
        line++;
        if (len == 0 || buf[len - 1] != '\n') {
            if (!feof(f)) {
                fclose(f);
                return vs_fail(list, line, "line longer than %d characters", VS_LINE_MAX - 1);
            }
        }
        while (len > 0 && (buf[len - 1] == '\n' || buf[len - 1] == '\r')) buf[--len] = 0;
        if (len == 0 || buf[0] == '#') continue;
        if (n == VS_MAX_ENTRIES) {
            fclose(f);
            return vs_fail(list, line, "more than %d entries", VS_MAX_ENTRIES);
        }
        t1 = strchr(buf, '\t');
        t2 = t1 ? strchr(t1 + 1, '\t') : NULL;
        if (!t1 || !t2 || strchr(t2 + 1, '\t')) {
            fclose(f);
            return vs_fail(list, line, "expected three TAB-separated fields (guest path, host file, SHA-256)");
        }
        *t1 = 0;
        *t2 = 0;
        ent[n].is_dir = 0;
        if (strcmp(t1 + 1, "-") == 0 && strcmp(t2 + 1, "-") == 0) {
            size_t gl = strlen(buf);
            if (gl < 4 || buf[gl - 1] != '\\') {
                fclose(f);
                return vs_fail(list, line, "a directory line ('-' and '-') needs a guest path that ends with a backslash");
            }
            buf[gl - 1] = 0;
            ent[n].is_dir = 1;
        }
        {
            size_t gu;
            if (vs_utf8_units(buf, strlen(buf), &gu) != 0) gu = strlen(buf);   /* decision 0178: units, not bytes (the same for ASCII); an invalid sequence is reported by vs_guest_problem below */
            if (gu > VS_GUEST_MAX || strlen(buf) > VS_GUEST_BYTES - 1) {
                fclose(f);
                return vs_fail(list, line, "guest path longer than 259 characters");
            }
        }
        {
            const char *why = vs_guest_problem(buf);
            if (why) {
                fclose(f);
                return vs_fail(list, line, "guest path '%s' %s", buf, why);
            }
        }
        if (ent[n].is_dir) {
            memset(ent[n].sha, '0', 64);
            ent[n].sha[64] = 0;
            strcpy(ent[n].guest, buf);
            ent[n].line = line;
            ent[n].host[0] = 0;
            for (i = 0; i < n; i++) {
                if (vs_ieq(ent[i].guest, ent[n].guest)) {
                    fclose(f);
                    return vs_fail(list, line, "the guest path %s is listed twice (line %u)", ent[n].guest, ent[i].line);
                }
            }
            n++;
            continue;
        }
        if (strlen(t2 + 1) != 64) {
            fclose(f);
            return vs_fail(list, line, "the SHA-256 field is not 64 hex digits");
        }
        for (p = t2 + 1, i = 0; i < 64; i++, p++) {
            char c = *p;
            if (c >= 'A' && c <= 'F') c = (char)(c + 32);
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) {
                fclose(f);
                return vs_fail(list, line, "the SHA-256 field is not 64 hex digits");
            }
            ent[n].sha[i] = c;
        }
        ent[n].sha[64] = 0;
        strcpy(ent[n].guest, buf);
        ent[n].line = line;
        if (vs_resolve_host(list, line, t1 + 1, ent[n].host, sizeof ent[n].host) != 0) {
            fclose(f);
            return -1;
        }
        for (i = 0; i < n; i++) {
            if (vs_ieq(ent[i].guest, ent[n].guest)) {
                fclose(f);
                return vs_fail(list, line, "the guest path %s is listed twice (line %u)", ent[n].guest, ent[i].line);
            }
        }
        n++;
    }
    fclose(f);
    if (n == 0) return vs_fail(list, 0, "the list has no entry");
    *count = n;
    return 0;
}

/* ---- the guest-facing stubs, called as the lifted code calls them: arguments in rcx, rdx, r8, r9 (R[1], R[2], R[8], R[9]) and
 * arguments 5.. at [rsp + 0x28 + 8 * (n - 5)] (fle_env.h fle_arg), the return value in R[0]. ---- */

static uint64_t vs_callv(void (*fn)(void), const uint64_t *a)
{
    static uint64_t stk[32];
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = VS_CALL_RETURN_ADDRESS;
    R[1] = a[0];
    R[2] = a[1];
    R[8] = a[2];
    R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0;
    fn();
    return R[0];
}
#define VS_CALL(fn, ...) vs_callv(fn, (const uint64_t[12]){ __VA_ARGS__ })
#define VS_A(x) ((uint64_t)(uintptr_t)(x))

/* Decision 0178: the first n bytes of a validated UTF-8 string as UTF-16 (for ASCII exactly the old byte-per-unit copy). */
static void vs_wide(uint16_t *dst, const char *s, size_t n)
{
    size_t i = 0, k = 0;
    while (i < n) {
        unsigned char c = (unsigned char)s[i];
        uint32_t cp;
        unsigned extra, j;
        if (c < 0x80) { cp = c; extra = 0; }
        else if (c < 0xE0) { cp = c & 0x1F; extra = 1; }
        else if (c < 0xF0) { cp = c & 0x0F; extra = 2; }
        else { cp = c & 0x07; extra = 3; }
        for (j = 1; j <= extra; j++) cp = (cp << 6) | ((unsigned char)s[i + j] & 0x3F);
        if (cp >= 0x10000) { cp -= 0x10000; dst[k++] = (uint16_t)(0xD800 + (cp >> 10)); dst[k++] = (uint16_t)(0xDC00 + (cp & 0x3FF)); }
        else dst[k++] = (uint16_t)cp;
        i += extra + 1;
    }
    dst[k] = 0;
}

/* Creates the directories above the file (C:\a, C:\a\b, ...) that are missing; a path component that exists as a file stops. */
static int vs_ensure_dirs(const char *list, const vs_entry *e, size_t *ndirs)
{
    uint16_t w[VS_GUEST_MAX + 1];
    size_t i, n = strlen(e->guest);
    for (i = 3; i <= n; i++) {
        uint32_t attrs;
        if (i == n ? !e->is_dir : e->guest[i] != '\\') continue;   /* decision 0129: the whole path of a directory entry is a directory too */
        vs_wide(w, e->guest, i);
        attrs = (uint32_t)VS_CALL(fle_imp_GetFileAttributesW, VS_A(w));
        if (attrs == FLE_INVALID_FILE_ATTRIBUTES) {
            if (VS_CALL(fle_imp_CreateDirectoryW, VS_A(w), 0) != 1)
                return vs_fail(list, e->line, "CreateDirectoryW failed for %.*s", (int)i, e->guest);
            (*ndirs)++;
        } else if (!(attrs & VS_ATTR_DIRECTORY)) {
            return vs_fail(list, e->line, "%.*s exists in the virtual volume and is not a directory", (int)i, e->guest);
        }
    }
    return 0;
}

/* Writes one file, reads it back through the same handle and compares it with the host bytes, closes the handle. */
static int vs_put(const char *list, const vs_entry *e, size_t *ndirs)
{
    uint16_t w[VS_GUEST_MAX + 1];
    uint32_t cnt = 0;
    uint64_t h;
    if (vs_ensure_dirs(list, e, ndirs) != 0) return -1;
    vs_wide(w, e->guest, strlen(e->guest));
    h = VS_CALL(fle_imp_CreateFileW, VS_A(w), VS_GENERIC_READ_WRITE, 0, 0, VS_CREATE_NEW, VS_ATTR_NORMAL, 0);
    if (h == FLE_INVALID_HANDLE_VALUE) return vs_fail(list, e->line, "CreateFileW(CREATE_NEW) failed for %s", e->guest);
    if (e->size) {
        uint8_t *back = malloc(e->size);
        int same;
        if (!back) return vs_fail(list, e->line, "out of memory for the read-back of %s", e->guest);
        if (VS_CALL(fle_imp_WriteFile, h, VS_A(e->data), (uint64_t)e->size, VS_A(&cnt), 0) != 1 || cnt != e->size) {
            free(back);
            return vs_fail(list, e->line, "WriteFile did not write all %zu bytes of %s", e->size, e->guest);
        }
        if ((uint32_t)VS_CALL(fle_imp_SetFilePointer, h, 0, 0, 0) != 0) {
            free(back);
            return vs_fail(list, e->line, "SetFilePointer(FILE_BEGIN) failed for %s", e->guest);
        }
        cnt = 0;
        same = VS_CALL(fle_imp_ReadFile, h, VS_A(back), (uint64_t)e->size, VS_A(&cnt), 0) == 1 && cnt == e->size && memcmp(back, e->data, e->size) == 0;
        free(back);
        if (!same) return vs_fail(list, e->line, "the virtual file %s does not read back as the host bytes", e->guest);
    }
    if (VS_CALL(fle_imp_CloseHandle, h) != 1) return vs_fail(list, e->line, "CloseHandle failed for %s", e->guest);
    return 0;
}

static int vs_run(const char *list)
{
    vs_entry *ent = calloc(VS_MAX_ENTRIES, sizeof *ent);
    size_t n = 0, i, ndirs = 0;
    int rc = -1;
    if (!ent) return vs_fail(list, 0, "out of memory");
    if (vs_parse(list, ent, &n) != 0) goto done;
    for (i = 0; i < n; i++) {
        char got[65];
        if (ent[i].is_dir) continue;
        if (vs_read_host(list, ent[i].line, &ent[i]) != 0) goto done;
        vs_sha256_hex(ent[i].data, ent[i].size, got);
        if (strcmp(got, ent[i].sha) != 0) {
            vs_fail(list, ent[i].line, "SHA-256 mismatch for %s (host file %s): expected %s, got %s; nothing was written", ent[i].guest,
                    ent[i].host, ent[i].sha, got);
            goto done;
        }
    }
    printf("vfs seed: list %s, %zu files\n", list, n);
    for (i = 0; i < n; i++) {
        if (ent[i].is_dir) {
            if (vs_ensure_dirs(list, &ent[i], &ndirs) != 0) goto done;
            printf("vfs seed: %s\\ (directory)\n", ent[i].guest);
            continue;
        }
        if (vs_put(list, &ent[i], &ndirs) != 0) goto done;
        printf("vfs seed: %s <- %s (%zu bytes, sha256 %s)\n", ent[i].guest, ent[i].host, ent[i].size, ent[i].sha);
    }
    printf("vfs seed: done, %zu files, %zu directories created\n", n, ndirs);
    rc = 0;
done:
    for (i = 0; i < VS_MAX_ENTRIES; i++) free(ent[i].data);
    free(ent);
    return rc;
}

/* 0: not asked for, or done. -1: failed (a message is on stderr). */
static int vfs_seed_from_env(void)
{
    const char *list = getenv(VFS_SEED_ENV);
    uint64_t saved[16];
    int rc;
    if (!list || !list[0]) return 0;
    memcpy(saved, R, sizeof saved);
    rc = vs_run(list);
    memcpy(R, saved, sizeof saved);
    if (rc == 0) fle_set_last_error(0);
    return rc;
}

#endif /* FLE_HARNESS_VFS_SEED_H */
