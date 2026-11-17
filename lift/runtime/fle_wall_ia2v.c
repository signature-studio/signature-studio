/* kernel32!GetPrivateProfileStringW (wall ia2-W17 of decision 0171, met again by Help > FL Studio tutorials; decision 0191, PROVISIONAL). See fle_wall_ia2.h.
 * Separate from fle_wall_ia2.c: it reads a file of the virtual volume (CreateFileW, fle_vfs_file_info, CloseHandle), which the other file's tests do not link.
 *
 * Learn (winbase.h, "GetPrivateProfileStringW function"): "Retrieves a string from the specified section in an initialization file"; lpAppName NULL: "copies all section names in the file to the supplied buffer"; lpKeyName NULL: "all key
 * names in the section specified by the lpAppName parameter are copied"; lpDefault: "If the lpKeyName key cannot be found ... copies the default string"; NULL default = ""; "The function inserts a null character in the
 * lpReturnedString buffer to strip any trailing blanks" of the default; nSize in characters; lpFileName without a full path: "the system searches for the file in the Windows directory"; the return value is "the number of characters
 * copied to the buffer, not including the terminating null character"; with both names given and a buffer that is too small "the string is truncated and followed by a null character, and the return value is equal to nSize minus
 * one"; with a NULL name and a too small buffer "the last string is truncated and followed by two null characters ... equal to nSize minus two"; "In the event the initialization file specified by lpFileName is not found, or
 * contains invalid values, this function will set errorno with a value of '0x2' (File Not Found)"; "If the string associated with lpKeyName is enclosed in single or double quotation marks, the marks are discarded";
 * "not case-sensitive"; the file form is "[section]" lines and "key=string" lines. The registry's IniFileMapping (the page's lookup step 1) maps nothing in the virtual registry, so the file on the volume is read.
 * Choices without a source (S-ia2-ini-*): (1) a line is a section header when its first non-blank character is '[' and it has a ']' (the name is the text between, trimmed); a key line has an '=' (the name is the text before the first
 * '=' trimmed, the value the text after it trimmed of blanks and tabs); a line whose first non-blank character is ';' is a comment; the FIRST matching key of the FIRST matching section answers; (2) the file is ASCII text:
 * a byte of 0x80 or above, a UTF-16 byte-order mark, a file of more than 1 MiB, a NULL buffer or nSize below 3 stop by name (the code-page conversion and the page's silence about them are not guessed). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_wall_ia2.h"

static int lc(int c) { return c >= 'A' && c <= 'Z' ? c + 32 : c; }
static int is_blank(int c) { return c == ' ' || c == '\t'; }

/* does the UTF-16 string w (NUL-terminated) equal the ASCII bytes [s, s+n) ignoring case? */
static int name_eq(const uint16_t *w, const uint8_t *s, size_t n)
{
    size_t i;

    for (i = 0; i < n; i++) if (w[i] == 0 || lc(w[i]) != lc(s[i])) return 0;
    return w[n] == 0;
}

typedef struct { uint16_t *out; uint32_t cap; uint32_t len; int trunc; } outbuf;

/* append the ASCII bytes [s, s+n) and a NUL (a multi-string element) */
static void put_elem(outbuf *o, const uint8_t *s, size_t n)
{
    size_t i;

    for (i = 0; i < n; i++) {
        if (o->len + 3u > o->cap) { o->trunc = 1; return; }
        o->out[o->len++] = s[i];
    }
    if (o->len + 2u > o->cap) { o->trunc = 1; return; }
    o->out[o->len++] = 0;
}

size_t fle_ia2_ini_get(const uint8_t *data, size_t n, const uint16_t *app, const uint16_t *key, const uint16_t *def, uint16_t *out, uint32_t nsize)
{
    size_t pos = 0, i;
    int in_sec = 0;                 /* inside a section that matches lpAppName */
    int sec_done = 0;               /* the key listing of the first matching section is finished */
    const uint8_t *val = NULL;
    size_t vlen = 0;
    int found = 0;
    outbuf o = { out, nsize, 0, 0 };

    for (i = 0; i < n; i++) if (data[i] >= 0x80) fle_stop("GetPrivateProfileStringW", "the initialization file holds a byte of 0x80 or above: the code page conversion is not modelled");
    while (pos < n) {
        size_t ls = pos, le, a, b;
        while (pos < n && data[pos] != '\n') pos++;
        le = pos;
        if (pos < n) pos++;
        if (le > ls && data[le - 1] == '\r') le--;
        while (ls < le && is_blank(data[ls])) ls++;
        if (ls >= le || data[ls] == ';') continue;
        if (data[ls] == '[') {
            for (a = ls + 1; a < le && data[a] != ']'; a++) {}
            if (a < le) {
                const uint8_t *s = data + ls + 1;
                size_t sn = a - ls - 1;
                while (sn && is_blank(s[0])) { s++; sn--; }
                while (sn && is_blank(s[sn - 1])) sn--;
                if (app == NULL) put_elem(&o, s, sn);
                if (in_sec) sec_done = 1;
                in_sec = app != NULL && !sec_done && name_eq(app, s, sn);
                continue;
            }
        }
        if (!in_sec || app == NULL) continue;
        for (a = ls; a < le && data[a] != '='; a++) {}
        if (a >= le) continue;
        b = a;
        while (b > ls && is_blank(data[b - 1])) b--;
        if (key == NULL) {
            put_elem(&o, data + ls, b - ls);
            continue;
        }
        if (!found && name_eq(key, data + ls, b - ls)) {
            size_t vs = a + 1, ve = le;
            while (vs < ve && is_blank(data[vs])) vs++;
            while (ve > vs && is_blank(data[ve - 1])) ve--;
            if (ve - vs >= 2 && (data[vs] == '"' || data[vs] == '\'') && data[ve - 1] == data[vs]) { vs++; ve--; }
            val = data + vs;
            vlen = ve - vs;
            found = 1;
        }
    }
    if (app == NULL || key == NULL) {
        if (o.len == 0 && !o.trunc) { out[0] = 0; out[1] = 0; return 0; }   /* nothing listed: two NULs (the page is silent; an empty multi-string) */
        if (o.trunc) {
            /* the last string is truncated and followed by two NULs: nsize - 2 characters are the content */
            out[nsize - 2] = 0; out[nsize - 1] = 0;
            return nsize - 2u;
        }
        out[o.len] = 0;
        return o.len;
    }
    if (found) {
        size_t k;
        if (vlen > nsize - 1u) vlen = nsize - 1u;
        for (k = 0; k < vlen; k++) out[k] = val[k];
        out[vlen] = 0;
        return vlen;
    }
    {
        size_t dl = 0, k;
        if (def != NULL) { while (def[dl]) dl++; while (dl && (def[dl - 1] == ' ' || def[dl - 1] == '\t')) dl--; }
        if (dl > nsize - 1u) dl = nsize - 1u;
        for (k = 0; k < dl; k++) out[k] = def[k];
        out[dl] = 0;
        return dl;
    }
}

static uint64_t va_of(const void *p) { return (uint64_t)(uintptr_t)p; }

void fle_imp_GetPrivateProfileStringW(void)
{
    static const char api[] = "GetPrivateProfileStringW";
    uint64_t app_g = R[1], key_g = R[2], def_g = R[8], out_g = R[9], file_g;
    uint32_t nsize;
    uint16_t *app = NULL, *key = NULL, *def = NULL, *file = NULL, *buf;
    size_t n, fl = 0, k;
    const uint8_t *data = NULL;
    uint64_t size = 0, h, saved[16], st[8] = { 0 };
    unsigned cr = 0, cw = 0;
    int have = 0;
    size_t ret = 0;

    {
        uint64_t sp = R[4];
        nsize = (uint32_t)fle_ld(sp + 8u + 8u * 4u, 8);
        file_g = fle_ld(sp + 8u + 8u * 5u, 8);
    }
    if (out_g == 0 || nsize < 3u) fle_stop(api, "lpReturnedString is NULL or nSize is below 3: the page gives no behaviour");
    if (file_g == 0) fle_stop(api, "lpFileName is NULL: the page gives no behaviour");
    {   /* read the UTF-16 strings of the guest */
        #define RD(g, dst) do { if ((g) != 0) { size_t m = 0; while (fle_ld((g) + 2u * m, 2) != 0) { if (++m > 600u) fle_stop(api, "a string longer than 600 characters is not modelled"); } \
            (dst) = malloc((m + 1) * 2); for (k = 0; k <= m; k++) (dst)[k] = (uint16_t)fle_ld((g) + 2u * k, 2); } } while (0)
        RD(app_g, app); RD(key_g, key); RD(def_g, def);
        RD(file_g, file);
        #undef RD
    }
    /* the Windows directory for a name with no path (the page) */
    for (fl = 0; file[fl]; fl++) {}
    {
        int has_path = 0;
        for (k = 0; k < fl; k++) if (file[k] == '\\' || file[k] == '/' || file[k] == ':') has_path = 1;
        n = fl + (has_path ? 0u : 11u);
        buf = malloc((n + 1) * 2);
        if (!has_path) { static const char pre[] = "C:\\Windows\\"; for (k = 0; k < 11; k++) buf[k] = (uint16_t)pre[k]; memcpy(buf + 11, file, (fl + 1) * 2); }
        else memcpy(buf, file, (fl + 1) * 2);
    }
    memcpy(saved, R, sizeof saved);
    R[1] = va_of(buf); R[2] = 0x80000000u; R[8] = 3; R[9] = 0; st[5] = 3; st[6] = 0x80; st[7] = 0; R[4] = va_of(st); R[0] = 0;
    fle_imp_CreateFileW();
    h = R[0];
    memcpy(R, saved, sizeof saved);
    if (h != UINT64_MAX && h != 0) {
        if (!fle_vfs_file_info(api, h, &data, &size, &cr, &cw) || !(cr)) fle_stop(api, "the initialization file cannot be read");
        if (size > (1u << 20)) fle_stop(api, "the initialization file is larger than 1 MiB: not modelled");
        if (size >= 2 && ((data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF))) fle_stop(api, "the initialization file starts with a UTF-16 byte-order mark: not modelled");
        have = 1;
    }
    {
        uint16_t *tmp = calloc((size_t)nsize + 2u, 2);
        size_t r2 = fle_ia2_ini_get(have ? data : (const uint8_t *)"", have ? (size_t)size : 0u, app, key, def, tmp, nsize);
        size_t total = (app == NULL || key == NULL) ? r2 + 2u : r2 + 1u;
        if (total > nsize) total = nsize;
        for (k = 0; k < total; k++) fle_st(out_g + 2u * k, 2, tmp[k]);
        free(tmp);
        ret = r2;
    }
    if (have) {
        memcpy(saved, R, sizeof saved);
        R[1] = h;
        fle_imp_CloseHandle();
        memcpy(R, saved, sizeof saved);
    } else {
        fle_set_last_error(2);      /* the page: not found -> 0x2 */
    }
    R[0] = ret;
    free(app); free(key); free(def); free(file); free(buf);
}

/* ---- kernel32!WritePrivateProfileStringW (wall ia2-W30: the Help "Internet links" write after they read) ----
 * BOOL f(LPCWSTR lpAppName rcx, LPCWSTR lpKeyName rdx, LPCWSTR lpString r8, LPCWSTR lpFileName r9). Learn (winbase.h, "WritePrivateProfileStringW function"): "Copies a string into the specified section of an initialization file";
 * lpAppName "the name of the section to which the string will be copied. If the section does not exist, it is created"; lpKeyName "the name of the key to be associated with a string. If the key does not exist in the specified
 * section, it is created. If this parameter is NULL, the entire section, including all entries within the section, is deleted"; lpString "A null-terminated string to be written to the file. If this parameter is NULL, the key
 * pointed to by the lpKeyName parameter is deleted"; lpFileName: a name without a full path is the Windows directory's; "If the file was created using Unicode characters, the function writes Unicode characters to the file.
 * Otherwise, the function writes ANSI characters"; a nonzero return value means success. The file is created when it does not exist (the page: "If the file does not exist, the function creates it", read from the Remarks).
 * Choices without a source (S-ia2-ini-write): the text is ASCII with CR LF line ends; a key that exists is rewritten as `key=string` in the first section of that name (the key's spelling of THIS call); a new key is appended after
 * the last line of its section; a new section is appended at the end of the file after one empty line; the other lines are kept byte for byte. A non-ASCII unit, an existing UTF-16 file or a NULL lpAppName stops by name. */
typedef struct { size_t s, e, t; int kind; size_t ns, ne; } ini_line;   /* [s, e) the text, [e, t) its line end; kind 1 = [section] header (name [ns, ne)), 2 = key line (name [ns, ne)), 0 = other */

size_t fle_ia2_ini_put(const uint8_t *in, size_t n, const uint16_t *app, const uint16_t *key, const uint16_t *str, uint8_t *out, size_t cap)
{
    static const char api[] = "WritePrivateProfileStringW";
    ini_line *L = malloc((n + 2) * sizeof *L);
    size_t nl = 0, pos = 0, i, o = 0, hs = (size_t)-1, he, kl = 0, sl = 0, al = 0, kidx = (size_t)-1, ins = (size_t)-1;

    for (i = 0; app[i]; i++) { if (app[i] >= 0x80) fle_stop(api, "a non-ASCII unit in lpAppName: the code page conversion is not modelled"); al++; }
    if (key) for (i = 0; key[i]; i++) { if (key[i] >= 0x80) fle_stop(api, "a non-ASCII unit in lpKeyName: not modelled"); kl++; }
    if (str) for (i = 0; str[i]; i++) { if (str[i] >= 0x80) fle_stop(api, "a non-ASCII unit in lpString: not modelled"); sl++; }
    for (i = 0; i < n; i++) if (in[i] >= 0x80) fle_stop(api, "the initialization file holds a byte of 0x80 or above: not modelled");
    while (pos < n) {
        ini_line *l = &L[nl++];
        size_t b, a;
        l->s = pos;
        while (pos < n && in[pos] != '\n') pos++;
        l->e = pos;
        if (pos < n) pos++;
        l->t = pos;
        if (l->e > l->s && in[l->e - 1] == '\r') l->e--;
        l->kind = 0; l->ns = l->ne = 0;
        b = l->s;
        while (b < l->e && is_blank(in[b])) b++;
        if (b < l->e && in[b] == '[') {
            for (a = b + 1; a < l->e && in[a] != ']'; a++) {}
            if (a < l->e) {
                l->ns = b + 1; l->ne = a;
                while (l->ns < l->ne && is_blank(in[l->ns])) l->ns++;
                while (l->ne > l->ns && is_blank(in[l->ne - 1])) l->ne--;
                l->kind = 1;
            }
        } else if (b < l->e && in[b] != ';') {
            for (a = b; a < l->e && in[a] != '='; a++) {}
            if (a < l->e) { l->ns = b; l->ne = a; while (l->ne > l->ns && is_blank(in[l->ne - 1])) l->ne--; l->kind = 2; }
        }
    }
    for (i = 0; i < nl; i++) if (L[i].kind == 1 && name_eq(app, in + L[i].ns, L[i].ne - L[i].ns)) { hs = i; break; }
    he = nl;
    if (hs != (size_t)-1) {
        for (i = hs + 1; i < nl; i++) if (L[i].kind == 1) { he = i; break; }
        if (key != NULL) {
            ins = hs;
            for (i = hs + 1; i < he; i++) {
                if (L[i].kind == 2 && kidx == (size_t)-1 && name_eq(key, in + L[i].ns, L[i].ne - L[i].ns)) kidx = i;
                if (L[i].e > L[i].s) ins = i;           /* the last non-empty line of the section */
            }
        }
    }
    #define EMIT(c) do { if (o + 1 > cap) fle_stop(api, "the file would exceed the buffer of this model"); out[o++] = (uint8_t)(c); } while (0)
    #define EMIT_KV() do { for (i = 0; i < kl; i++) EMIT(key[i]); EMIT('='); for (i = 0; i < sl; i++) EMIT(str[i]); EMIT('\r'); EMIT('\n'); } while (0)
    {
        size_t li;
        for (li = 0; li < nl; li++) {
            if (hs != (size_t)-1 && key == NULL && li >= hs && li < he) continue;          /* lpKeyName NULL: the whole section goes */
            if (li == kidx) {
                if (str != NULL) EMIT_KV();                                                  /* replaced (lpString NULL: deleted) */
            } else {
                for (i = L[li].s; i < L[li].e; i++) EMIT(in[i]);
                if (L[li].t > L[li].e) { EMIT('\r'); EMIT('\n'); }
                else if (li + 1 < nl || (key != NULL && str != NULL && kidx == (size_t)-1 && li == ins)) { EMIT('\r'); EMIT('\n'); }   /* the last line had no line end: give it one before something follows */
            }
            if (li == ins && kidx == (size_t)-1 && key != NULL && str != NULL) EMIT_KV();     /* a new key after the last non-empty line of its section */
        }
        if (hs == (size_t)-1 && key != NULL && str != NULL) {                               /* a new section at the end, after one empty line */
            if (nl > 0) { EMIT('\r'); EMIT('\n'); }
            EMIT('['); for (i = 0; i < al; i++) EMIT(app[i]); EMIT(']'); EMIT('\r'); EMIT('\n');
            EMIT_KV();
        }
    }
    #undef EMIT_KV
    #undef EMIT
    free(L);
    return o;
}

void fle_imp_WritePrivateProfileStringW(void)
{
    static const char api[] = "WritePrivateProfileStringW";
    uint64_t app_g = R[1], key_g = R[2], str_g = R[8], file_g = R[9], h, saved[16], st[8] = { 0 };
    uint16_t *app = NULL, *key = NULL, *str = NULL, *file = NULL, *buf;
    uint8_t *old = NULL, *out;
    size_t n, fl = 0, k, oldn = 0, outn;
    const uint8_t *data = NULL;
    uint64_t size = 0;
    unsigned cr = 0, cw = 0;
    int ok = 1;

    /* The page: "If all parameters are NULL, the function flushes the cache" and the setup recipe calls it with the first three NULL and the file name; "if it flushes the cached version of the most recently accessed initialization file,
     * the return value is zero". This runtime keeps no cache (every call reads the file), so a flush has nothing to do: 0, nothing written, the last error left alone (the page gives none for the flush: S-ia2-ini-flush-lasterror). */
    if (app_g == 0 && key_g == 0 && str_g == 0) { R[0] = 0; return; }
    if (app_g == 0) fle_stop(api, "lpAppName is NULL but lpKeyName or lpString is not: the page defines a NULL lpAppName only for the flush of the cache (all of them NULL)");   /* decision 0209: soft: zero ("If the function fails, the return value is zero") and ERROR_INVALID_PARAMETER; the page is silent about this input, the failure value is its general one (S-crash1-9) */
    if (file_g == 0) fle_stop(api, "lpFileName is NULL for a write: the page gives no behaviour");   /* decision 0209: soft, as above */
    #define RD(g, dst) do { if ((g) != 0) { size_t m = 0; while (fle_ld((g) + 2u * m, 2) != 0) { if (++m > 600u) fle_stop(api, "a string longer than 600 characters is not modelled"); } \
        (dst) = malloc((m + 1) * 2); for (k = 0; k <= m; k++) (dst)[k] = (uint16_t)fle_ld((g) + 2u * k, 2); } } while (0)
    RD(app_g, app); RD(key_g, key); RD(str_g, str); RD(file_g, file);
    #undef RD
    for (fl = 0; file[fl]; fl++) {}
    {
        int has_path = 0;
        for (k = 0; k < fl; k++) if (file[k] == '\\' || file[k] == '/' || file[k] == ':') has_path = 1;
        n = fl + (has_path ? 0u : 11u);
        buf = malloc((n + 1) * 2);
        if (!has_path) { static const char pre[] = "C:\\Windows\\"; for (k = 0; k < 11; k++) buf[k] = (uint16_t)pre[k]; memcpy(buf + 11, file, (fl + 1) * 2); }
        else memcpy(buf, file, (fl + 1) * 2);
    }
    /* read the file, when it exists */
    memcpy(saved, R, sizeof saved);
    R[1] = va_of(buf); R[2] = 0x80000000u; R[8] = 3; R[9] = 0; st[5] = 3; st[6] = 0x80; st[7] = 0; R[4] = va_of(st); R[0] = 0;
    fle_imp_CreateFileW();
    h = R[0];
    memcpy(R, saved, sizeof saved);
    if (h != UINT64_MAX && h != 0) {
        if (!fle_vfs_file_info(api, h, &data, &size, &cr, &cw) || !cr) fle_stop(api, "the initialization file cannot be read");
        if (size > (1u << 18)) fle_stop(api, "the initialization file is larger than 256 KiB: not modelled");
        if (size >= 2 && ((data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF))) fle_stop(api, "the initialization file starts with a UTF-16 byte-order mark: not modelled");
        oldn = (size_t)size;
        old = malloc(oldn + 1);
        memcpy(old, data, oldn);
        memcpy(saved, R, sizeof saved);
        R[1] = h;
        fle_imp_CloseHandle();
        memcpy(R, saved, sizeof saved);
    }
    out = malloc(oldn + 1200 + 4 * 600);
    outn = fle_ia2_ini_put(old ? old : (const uint8_t *)"", oldn, app, key, str, out, oldn + 1200 + 4 * 600 - 1);
    /* write it back (CREATE_ALWAYS) */
    memcpy(saved, R, sizeof saved);
    R[1] = va_of(buf); R[2] = 0x40000000u; R[8] = 0; R[9] = 0; st[5] = 2; st[6] = 0x80; st[7] = 0; R[4] = va_of(st); R[0] = 0;
    fle_imp_CreateFileW();
    h = R[0];
    memcpy(R, saved, sizeof saved);
    if (h == UINT64_MAX || h == 0) {
        ok = 0;            /* the error code was set by CreateFileW */
    } else {
        uint32_t wr = 0;
        memcpy(saved, R, sizeof saved);
        R[1] = h; R[2] = va_of(out); R[8] = outn; R[9] = va_of(&wr); st[5] = 0; R[4] = va_of(st); R[0] = 0;
        fle_imp_WriteFile();
        if (R[0] == 0 || wr != outn) ok = 0;
        R[1] = h;
        fle_imp_CloseHandle();
        memcpy(R, saved, sizeof saved);
    }
    free(app); free(key); free(str); free(file); free(buf); free(old); free(out);
    R[0] = ok ? 1u : 0u;
}
