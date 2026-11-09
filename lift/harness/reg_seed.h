/* Seeding of the virtual registry (decision 0116, PROVISIONAL). Header-only helper of lift/harness/two/main_two.c, compiled only into a
 * build with FLE_FRUITY (build_two_native.sh FRUITY=1), so that the harness objects and the native binary of a build without it are the
 * ones of the harness before this file existed. It is not part of the runtime and the runtime gets no new code: the values are written
 * through the guest-facing C stubs of lift/runtime/fle_reg.c (RegCreateKeyExW, RegSetValueExW, RegCloseKey), called before the entry,
 * exactly as lifted code would call them (the same way as vfs_seed.h, decision 0110, does for files).
 *
 * Off by default. FLE_REG_SEED unset or empty: reg_seed_from_env() returns 0 at once, prints nothing and touches nothing, so the run is the
 * one of a harness without this file (the registry is empty, decisions 0018 and 0085).
 *
 * FLE_REG_SEED=<list file>[:<list file>...]. One entry per line, four TAB-separated fields:
 *     <key> TAB <value name> TAB <type> TAB <data>
 * Empty lines and lines that start with '#' are ignored.
 *   key         HKCU\<subkey> or HKLM\<subkey> (case-insensitive root; no other root), backslashes only, ASCII, no empty component, no
 *               component over 255 characters, at most 32 components (the limit of one RegCreateKeyExW call). The keys on the way are created.
 *   value name  ASCII; empty = the unnamed (default) value of the key.
 *   type        REG_SZ, REG_EXPAND_SZ, REG_DWORD or REG_BINARY. (REG_MULTI_SZ, REG_QWORD and the others are refused here.)
 *   data        REG_SZ / REG_EXPAND_SZ: the text, printable ASCII (the stored value is that text plus the terminating NUL, UTF-16);
 *               REG_DWORD: decimal or 0x-prefixed hexadecimal, at most 0xFFFFFFFF; REG_BINARY: an even number of hexadecimal digits (may be empty).
 * The whole list (every file) is read and checked first. One bad line, an unreadable file, or the same key + value name listed twice (names
 * and keys compare without regard to case, as the registry does) stops the harness by name with nothing written. Success prints "reg seed:"
 * lines on stdout; any failure prints "reg-seed: ..." on stderr and reg_seed_from_env() returns -1 (the caller exits 2 before the guest runs).
 *
 * What the seeding changes for the guest, besides the keys and values themselves (decision 0116): the virtual clock of fle_reg.c has advanced
 * (one tick per key created and per value written) so the write times of the keys it touched are those ticks, and the key handles issued
 * afterwards start above the ones used here (handle values are never reused). R[], and the last error, are put back as they were (R[] is saved
 * and restored; the last error is set to 0). */
#ifndef FLE_HARNESS_REG_SEED_H
#define FLE_HARNESS_REG_SEED_H

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_reg.h"
#include "fle_runtime.h"

#define REG_SEED_ENV "FLE_REG_SEED"

#define RS_MAX_ENTRIES 256
#define RS_KEY_MAX 1024
#define RS_NAME_MAX 256
#define RS_DATA_MAX 1024
#define RS_LINE_MAX 4096
#define RS_FILES_MAX 8
#define RS_CALL_RETURN_ADDRESS UINT64_C(0xdead5eed)
#define RS_KEY_WRITE 0x20006u /* KEY_SET_VALUE | KEY_CREATE_SUB_KEY | STANDARD_RIGHTS_WRITE: inside the sam bits fle_reg.c accepts */

typedef struct {
    int root; /* 1 = HKCU, 2 = HKLM */
    char key[RS_KEY_MAX];
    char name[RS_NAME_MAX];
    uint32_t type;
    uint8_t data[2 * RS_DATA_MAX + 2];
    uint32_t size;
    char shown[RS_DATA_MAX + 16];
    unsigned line;
    const char *list;
} rs_entry;

static int rs_fail(const char *list, unsigned line, const char *fmt, ...) __attribute__((format(printf, 3, 4)));
static int rs_fail(const char *list, unsigned line, const char *fmt, ...)
{
    va_list ap;
    if (line) fprintf(stderr, "reg-seed: %s:%u: ", list, line);
    else fprintf(stderr, "reg-seed: %s: ", list);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    return -1;
}

static int rs_ieq(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
    }
    return *a == *b;
}

static int rs_ieq_n(const char *a, const char *b, size_t n)
{
    size_t i;
    for (i = 0; i < n; i++) {
        char x = a[i], y = b[i];
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
    }
    return 1;
}

static int rs_hexval(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/* NULL when the text is printable ASCII (0x20..0x7e), else the reason. */
static const char *rs_ascii_problem(const char *s)
{
    for (; *s; s++)
        if ((unsigned char)*s < 0x20 || (unsigned char)*s > 0x7e) return "has a character outside printable ASCII";
    return NULL;
}

/* key = "HKCU\a\b" or "HKLM\a\b". Returns NULL when acceptable (and sets *root, *sub), else the reason. */
static const char *rs_key_problem(const char *k, int *root, const char **sub)
{
    size_t n = strlen(k), i, comp, levels = 0;
    const char *why = rs_ascii_problem(k);
    if (why) return why;
    if (n > 5 && k[4] == '\\' && rs_ieq_n(k, "HKCU", 4)) *root = 1;
    else if (n > 5 && k[4] == '\\' && rs_ieq_n(k, "HKLM", 4)) *root = 2;
    else return "does not start with HKCU\\ or HKLM\\ followed by a subkey";
    *sub = k + 5;
    comp = 5;
    for (i = 5; i <= n; i++) {
        if (k[i] == '\\' || k[i] == 0) {
            size_t cl = i - comp;
            if (cl == 0) return "has an empty component (or ends with a backslash)";
            if (cl > 255) return "has a component longer than 255 characters";
            levels++;
            comp = i + 1;
        }
    }
    if (levels > 32) return "has more than 32 components";
    return NULL;
}

static int rs_parse_data(const char *list, unsigned line, rs_entry *e, const char *ty, char *data)
{
    size_t n = strlen(data), i;
    if (strcmp(ty, "REG_SZ") == 0 || strcmp(ty, "REG_EXPAND_SZ") == 0) {
        const char *why = rs_ascii_problem(data);
        if (why) return rs_fail(list, line, "the data %s", why);
        if (n > RS_DATA_MAX) return rs_fail(list, line, "the text is longer than %d characters", RS_DATA_MAX);
        e->type = strcmp(ty, "REG_SZ") == 0 ? FLE_REG_SZ : FLE_REG_EXPAND_SZ;
        for (i = 0; i < n; i++) {
            e->data[2 * i] = (uint8_t)data[i];
            e->data[2 * i + 1] = 0;
        }
        e->data[2 * n] = 0;
        e->data[2 * n + 1] = 0;
        e->size = (uint32_t)(2 * n + 2);
        snprintf(e->shown, sizeof e->shown, "\"%s\"", data);
    } else if (strcmp(ty, "REG_DWORD") == 0) {
        uint64_t v = 0;
        int hex = n > 2 && data[0] == '0' && (data[1] == 'x' || data[1] == 'X');
        size_t from = hex ? 2 : 0;
        if (n == from) return rs_fail(list, line, "REG_DWORD data is empty");
        for (i = from; i < n; i++) {
            int d = hex ? rs_hexval(data[i]) : (data[i] >= '0' && data[i] <= '9' ? data[i] - '0' : -1);
            if (d < 0) return rs_fail(list, line, "REG_DWORD data '%s' is not a decimal or 0x-hexadecimal number", data);
            v = v * (hex ? 16u : 10u) + (uint64_t)d;
            if (v > 0xFFFFFFFFu) return rs_fail(list, line, "REG_DWORD data '%s' is larger than 0xFFFFFFFF", data);
        }
        e->type = FLE_REG_DWORD;
        for (i = 0; i < 4; i++) e->data[i] = (uint8_t)(v >> (8 * i));
        e->size = 4;
        snprintf(e->shown, sizeof e->shown, "%u (0x%x)", (unsigned)v, (unsigned)v);
    } else if (strcmp(ty, "REG_BINARY") == 0) {
        if (n % 2) return rs_fail(list, line, "REG_BINARY data has an odd number of hexadecimal digits");
        if (n / 2 > RS_DATA_MAX) return rs_fail(list, line, "REG_BINARY data is longer than %d bytes", RS_DATA_MAX);
        for (i = 0; i < n; i += 2) {
            int a = rs_hexval(data[i]), b = rs_hexval(data[i + 1]);
            if (a < 0 || b < 0) return rs_fail(list, line, "REG_BINARY data is not hexadecimal");
            e->data[i / 2] = (uint8_t)(a * 16 + b);
        }
        e->type = FLE_REG_BINARY;
        e->size = (uint32_t)(n / 2);
        snprintf(e->shown, sizeof e->shown, "%zu bytes", n / 2);
    } else {
        return rs_fail(list, line, "the type '%s' is not one of REG_SZ, REG_EXPAND_SZ, REG_DWORD, REG_BINARY", ty);
    }
    return 0;
}

static int rs_parse(const char *list, rs_entry *ent, size_t *count)
{
    FILE *f = fopen(list, "r");
    char buf[RS_LINE_MAX];
    unsigned line = 0;
    size_t n = *count, i;
    if (!f) return rs_fail(list, 0, "cannot open the list");
    while (fgets(buf, sizeof buf, f)) {
        char *t1, *t2, *t3;
        size_t len = strlen(buf);
        const char *why, *sub = NULL;
        int root = 0;
        line++;
        if (len == 0 || buf[len - 1] != '\n') {
            if (!feof(f)) {
                fclose(f);
                return rs_fail(list, line, "line longer than %d characters", RS_LINE_MAX - 1);
            }
        }
        while (len > 0 && (buf[len - 1] == '\n' || buf[len - 1] == '\r')) buf[--len] = 0;
        if (len == 0 || buf[0] == '#') continue;
        if (n == RS_MAX_ENTRIES) {
            fclose(f);
            return rs_fail(list, line, "more than %d entries", RS_MAX_ENTRIES);
        }
        t1 = strchr(buf, '\t');
        t2 = t1 ? strchr(t1 + 1, '\t') : NULL;
        t3 = t2 ? strchr(t2 + 1, '\t') : NULL;
        if (!t1 || !t2 || !t3 || strchr(t3 + 1, '\t')) {
            fclose(f);
            return rs_fail(list, line, "expected four TAB-separated fields (key, value name, type, data)");
        }
        *t1 = *t2 = *t3 = 0;
        why = rs_key_problem(buf, &root, &sub);
        if (why) {
            fclose(f);
            return rs_fail(list, line, "key '%s' %s", buf, why);
        }
        if (strlen(buf) >= RS_KEY_MAX) {
            fclose(f);
            return rs_fail(list, line, "key longer than %d characters", RS_KEY_MAX - 1);
        }
        why = rs_ascii_problem(t1 + 1);
        if (why || strlen(t1 + 1) >= RS_NAME_MAX) {
            fclose(f);
            return rs_fail(list, line, "the value name %s", why ? why : "is too long");
        }
        memset(&ent[n], 0, sizeof ent[n]);
        ent[n].root = root;
        snprintf(ent[n].key, sizeof ent[n].key, "%s", sub);
        snprintf(ent[n].name, sizeof ent[n].name, "%s", t1 + 1);
        ent[n].line = line;
        ent[n].list = list;
        if (rs_parse_data(list, line, &ent[n], t2 + 1, t3 + 1) != 0) {
            fclose(f);
            return -1;
        }
        for (i = 0; i < n; i++) {
            if (ent[i].root == ent[n].root && rs_ieq(ent[i].key, ent[n].key) && rs_ieq(ent[i].name, ent[n].name)) {
                fclose(f);
                return rs_fail(list, line, "'%s' [%s] is listed twice (%s:%u)", buf, t1 + 1, ent[i].list, ent[i].line);
            }
        }
        n++;
    }
    fclose(f);
    *count = n;
    return 0;
}

/* ---- the guest-facing stubs, called as the lifted code calls them (arguments in rcx, rdx, r8, r9 = R[1], R[2], R[8], R[9], the 5th and
 * later at [rsp + 0x28 + 8 * (n - 5)], the return value in R[0]) ---- */

static uint64_t rs_callv(void (*fn)(void), const uint64_t *a)
{
    static uint64_t stk[32];
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = RS_CALL_RETURN_ADDRESS;
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
#define RS_CALL(fn, ...) rs_callv(fn, (const uint64_t[12]){ __VA_ARGS__ })
#define RS_A(x) ((uint64_t)(uintptr_t)(x))

static int rs_put(const rs_entry *e)
{
    uint16_t sub[RS_KEY_MAX], nm[RS_NAME_MAX];
    uint64_t h = 0;
    size_t i, kl = strlen(e->key), nl = strlen(e->name);
    uint32_t st;
    for (i = 0; i <= kl; i++) sub[i] = (uint16_t)(unsigned char)e->key[i];
    for (i = 0; i <= nl; i++) nm[i] = (uint16_t)(unsigned char)e->name[i];
    st = (uint32_t)RS_CALL(fle_imp_RegCreateKeyExW, e->root == 1 ? FLE_HKEY_CURRENT_USER : FLE_HKEY_LOCAL_MACHINE, RS_A(sub), 0, 0, 0, RS_KEY_WRITE, 0,
                           RS_A(&h), 0);
    if (st != 0) return rs_fail(e->list, e->line, "RegCreateKeyExW failed (%u) for %s", st, e->key);
    st = (uint32_t)RS_CALL(fle_imp_RegSetValueExW, h, RS_A(nm), 0, e->type, RS_A(e->data), e->size);
    if (st != 0) return rs_fail(e->list, e->line, "RegSetValueExW failed (%u) for %s [%s]", st, e->key, e->name);
    st = (uint32_t)RS_CALL(fle_imp_RegCloseKey, h);
    if (st != 0) return rs_fail(e->list, e->line, "RegCloseKey failed (%u) for %s", st, e->key);
    return 0;
}

static const char *rs_type_name(uint32_t t)
{
    return t == FLE_REG_SZ ? "REG_SZ" : t == FLE_REG_EXPAND_SZ ? "REG_EXPAND_SZ" : t == FLE_REG_DWORD ? "REG_DWORD" : "REG_BINARY";
}

static int rs_run(char *lists)
{
    rs_entry *ent = calloc(RS_MAX_ENTRIES, sizeof *ent);
    const char *files[RS_FILES_MAX];
    size_t nfiles = 0, n = 0, i;
    char *p = lists;
    int rc = -1;
    if (!ent) return rs_fail(lists, 0, "out of memory");
    for (;;) {
        char *c = strchr(p, ':');
        if (c) *c = 0;
        if (p[0] == 0) {
            rs_fail(lists, 0, "an empty file name in the list of lists");
            goto done;
        }
        if (nfiles == RS_FILES_MAX) {
            rs_fail(lists, 0, "more than %d list files", RS_FILES_MAX);
            goto done;
        }
        files[nfiles++] = p;
        if (!c) break;
        p = c + 1;
    }
    for (i = 0; i < nfiles; i++)
        if (rs_parse(files[i], ent, &n) != 0) goto done;
    if (n == 0) {
        rs_fail(files[0], 0, "the list has no entry");
        goto done;
    }
    printf("reg seed: %zu list file(s), %zu values\n", nfiles, n);
    for (i = 0; i < n; i++) {
        if (rs_put(&ent[i]) != 0) goto done;
        printf("reg seed: %s\\%s [%s] %s %s\n", ent[i].root == 1 ? "HKCU" : "HKLM", ent[i].key, ent[i].name, rs_type_name(ent[i].type), ent[i].shown);
    }
    printf("reg seed: done, %zu values\n", n);
    rc = 0;
done:
    free(ent);
    return rc;
}

/* 0: not asked for, or done. -1: failed (a message is on stderr). */
static int reg_seed_from_env(void)
{
    const char *env = getenv(REG_SEED_ENV);
    uint64_t saved[16];
    char *copy;
    int rc;
    if (!env || !env[0]) return 0;
    copy = strdup(env);
    if (!copy) return rs_fail(env, 0, "out of memory");
    memcpy(saved, R, sizeof saved);
    rc = rs_run(copy);
    memcpy(R, saved, sizeof saved);
    if (rc == 0) fle_set_last_error(0);
    free(copy);
    return rc;
}

#endif /* FLE_HARNESS_REG_SEED_H */
