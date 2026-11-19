/* Self-test of lift/harness/reg_seed.h (decision 0116). Links the virtual registry of the runtime and nothing of the lifted code, so it builds and
 * runs in seconds. Writes only into the directory that is its first argument (which must exist). The registry is a process-wide singleton, so the
 * checks run in one order: what an earlier check wrote is part of the state of the later ones (the comments say where it matters).
 *
 *   cd lift && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness \
 *       runtime/fle_vfs.c runtime/fle_reg.c runtime/fle_env.c runtime/fle_imports.c runtime/fle_modules.c runtime/fle_string.c \
 *       harness/seeds/reg_seed_selftest.c -o $OUT/reg_seed_selftest && $OUT/reg_seed_selftest <existing empty dir>
 *
 * Prints "ok <name>" or "FAIL <name>" per check; the exit code is the number of failures (0 = all passed). Expected values are written from the format
 * description in reg_seed.h and from the Microsoft pages cited in runtime/fle_reg.c (REG_SZ data are UTF-16 with the terminating NUL), not read back from
 * the code under test. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "reg_seed.h"

uint64_t R[16];

static int failures;
static uint64_t stk_t[32];

static void check(int cond, const char *name)
{
    printf("%s %s\n", cond ? "ok  " : "FAIL", name);
    if (!cond) failures++;
}

static void write_list(const char *dir, const char *name, const char *text)
{
    char path[1024];
    FILE *f;
    snprintf(path, sizeof path, "%s/%s", dir, name);
    f = fopen(path, "wb");
    if (!f || fwrite(text, 1, strlen(text), f) != strlen(text)) {
        fprintf(stderr, "selftest: cannot write %s\n", path);
        exit(99);
    }
    fclose(f);
}

static void path_of(char *out, size_t cap, const char *dir, const char *name)
{
    snprintf(out, cap, "%s/%s", dir, name);
}

static void set_pattern(void)
{
    int i;
    for (i = 0; i < 16; i++) R[i] = 0x1111000000000000ull + (uint64_t)i;
}

static int pattern_intact(void)
{
    int i;
    for (i = 0; i < 16; i++)
        if (R[i] != 0x1111000000000000ull + (uint64_t)i) return 0;
    return 1;
}

static uint64_t call_t(void (*fn)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6)
{
    memset(stk_t, 0, sizeof stk_t);
    stk_t[0] = 0xdead5eedull;
    stk_t[5] = a5;
    stk_t[6] = a6;
    R[1] = a1;
    R[2] = a2;
    R[8] = a3;
    R[9] = a4;
    R[4] = (uint64_t)(uintptr_t)stk_t;
    R[0] = 0xBAD;
    fn();
    return R[0];
}

static void wide(uint16_t *dst, const char *s)
{
    size_t i = 0;
    for (; s[i]; i++) dst[i] = (uint16_t)(unsigned char)s[i];
    dst[i] = 0;
}

/* Reads the value back the way the guest does: RegOpenKeyExW(root, key, 0, KEY_READ), RegQueryValueExW, RegCloseKey. Returns the status of the
 * query (2 when the key or value is missing); *type and *size are set on success; data receives up to cap bytes. */
static uint32_t read_back(uint64_t root, const char *key, const char *name, uint32_t *type, uint32_t *size, uint8_t *data, uint32_t cap)
{
    uint16_t k[512], n[256];
    uint64_t h = 0;
    uint32_t cb = cap, st;
    wide(k, key);
    wide(n, name);
    st = (uint32_t)call_t(fle_imp_RegOpenKeyExW, root, (uint64_t)(uintptr_t)k, 0, 0x20019, (uint64_t)(uintptr_t)&h, 0);
    if (st != 0) return st;
    st = (uint32_t)call_t(fle_imp_RegQueryValueExW, h, (uint64_t)(uintptr_t)n, 0, (uint64_t)(uintptr_t)type, (uint64_t)(uintptr_t)data,
                          (uint64_t)(uintptr_t)&cb);
    *size = cb;
    call_t(fle_imp_RegCloseKey, h, 0, 0, 0, 0, 0);
    return st;
}

static int is_str(const uint8_t *d, uint32_t size, const char *s)
{
    size_t n = strlen(s), i;
    if (size != 2 * (n + 1)) return 0;
    for (i = 0; i <= n; i++)
        if (d[2 * i] != (uint8_t)s[i] || d[2 * i + 1] != 0) return 0;
    return 1;
}

static int run_list(const char *list)
{
    setenv(REG_SEED_ENV, list, 1);
    return reg_seed_from_env();
}

int main(int argc, char **argv)
{
    const char *dir = argc > 1 ? argv[1] : NULL;
    char p1[1100], p2[1100], p3[1100], joined[2300];
    uint32_t type, size, st;
    uint8_t data[512];

    if (!dir) {
        fprintf(stderr, "usage: reg_seed_selftest <existing empty directory>\n");
        return 99;
    }

    /* off by default: unset and empty do nothing, even with a garbage environment */
    unsetenv(REG_SEED_ENV);
    set_pattern();
    check(reg_seed_from_env() == 0, "unset: returns 0");
    check(pattern_intact(), "unset: R[] untouched");
    setenv(REG_SEED_ENV, "", 1);
    check(reg_seed_from_env() == 0, "empty value: returns 0");
    unsetenv(REG_SEED_ENV);
    st = read_back(0xFFFFFFFF80000001ull, "Software", "x", &type, &size, data, sizeof data);
    check(st == 2, "unset/empty: HKCU\\Software is still missing (the registry is untouched)");

    /* a good list: three types, the default value, both roots, a key shared by two values, case-insensitive root, spaces in names */
    write_list(dir, "good.tsv",
               "# a comment line\n"
               "\n"
               "HKCU\\Software\\Image-Line\\Shared\\Paths\tShared data\tREG_SZ\tC:\\Users\\user\\Documents\\Image-Line\\\n"
               "hkcu\\SOFTWARE\\Image-Line\\Shared\\Paths\tFL Studio engine\tREG_EXPAND_SZ\t%ProgramFiles%\\Image-Line\\FLEngine_x64.dll\r\n"
               "HKLM\\Software\\Test\\Sub\tCount\tREG_DWORD\t0x10\n"
               "HKLM\\Software\\Test\\Sub\tDec\tREG_DWORD\t4294967295\n"
               "HKLM\\Software\\Test\\Sub\tBlob\tREG_BINARY\t00ff7f\n"
               "HKLM\\Software\\Test\\Sub\tEmptyBlob\tREG_BINARY\t\n"
               "HKLM\\Software\\Test\\Sub\t\tREG_SZ\tdefault text\n"
               "HKLM\\Software\\Test\\Sub\tEmptyText\tREG_SZ\t\n");
    path_of(p1, sizeof p1, dir, "good.tsv");
    set_pattern();
    fle_set_last_error(77);
    check(run_list(p1) == 0, "good list: returns 0");
    check(pattern_intact(), "good list: R[] put back");
    st = read_back(0xFFFFFFFF80000001ull, "Software\\Image-Line\\Shared\\Paths", "Shared data", &type, &size, data, sizeof data);
    check(st == 0 && type == 1 && is_str(data, size, "C:\\Users\\user\\Documents\\Image-Line\\"), "REG_SZ value reads back (UTF-16 with the NUL)");
    st = read_back(0xFFFFFFFF80000001ull, "software\\image-line\\shared\\paths", "shared DATA", &type, &size, data, sizeof data);
    check(st == 0, "key and value names are found without regard to case");
    st = read_back(0xFFFFFFFF80000001ull, "Software\\Image-Line\\Shared\\Paths", "FL Studio engine", &type, &size, data, sizeof data);
    check(st == 0 && type == 2 && is_str(data, size, "%ProgramFiles%\\Image-Line\\FLEngine_x64.dll"), "REG_EXPAND_SZ value reads back (lower-case root, CRLF line)");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "Count", &type, &size, data, sizeof data);
    check(st == 0 && type == 4 && size == 4 && data[0] == 0x10 && data[1] == 0 && data[2] == 0 && data[3] == 0, "REG_DWORD 0x10 reads back little endian");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "Dec", &type, &size, data, sizeof data);
    check(st == 0 && type == 4 && size == 4 && data[0] == 0xff && data[1] == 0xff && data[2] == 0xff && data[3] == 0xff, "REG_DWORD 4294967295 reads back");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "Blob", &type, &size, data, sizeof data);
    check(st == 0 && type == 3 && size == 3 && data[0] == 0x00 && data[1] == 0xff && data[2] == 0x7f, "REG_BINARY reads back");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "EmptyBlob", &type, &size, data, sizeof data);
    check(st == 0 && type == 3 && size == 0, "empty REG_BINARY is a zero-length value");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "", &type, &size, data, sizeof data);
    check(st == 0 && type == 1 && is_str(data, size, "default text"), "an empty value name is the unnamed (default) value");
    st = read_back(0xFFFFFFFF80000002ull, "Software\\Test\\Sub", "EmptyText", &type, &size, data, sizeof data);
    check(st == 0 && type == 1 && size == 2 && data[0] == 0 && data[1] == 0, "empty REG_SZ text is the NUL only");
    st = read_back(0xFFFFFFFF80000001ull, "Software\\Test", "Count", &type, &size, data, sizeof data);
    check(st == 2, "HKCU did not get the HKLM keys");
    fle_imp_GetLastError();
    check((uint32_t)R[0] == 0, "the last error is 0 after a good list (it was 77 before)");

    /* two lists in one variable, separated by ':' */
    write_list(dir, "a.tsv", "HKCU\\Software\\Two\tA\tREG_DWORD\t1\n");
    write_list(dir, "b.tsv", "HKCU\\Software\\Two\tB\tREG_DWORD\t2\n");
    path_of(p1, sizeof p1, dir, "a.tsv");
    path_of(p2, sizeof p2, dir, "b.tsv");
    snprintf(joined, sizeof joined, "%s:%s", p1, p2);
    check(run_list(joined) == 0, "two lists joined by ':' : returns 0");
    check(read_back(0xFFFFFFFF80000001ull, "Software\\Two", "A", &type, &size, data, sizeof data) == 0 &&
              read_back(0xFFFFFFFF80000001ull, "Software\\Two", "B", &type, &size, data, sizeof data) == 0,
          "both lists were written");

    /* every refusal writes nothing: each list has a good line first and a bad line after it, and the good line must not appear */
    {
        static const struct {
            const char *name, *bad;
        } bad[] = {
            { "three fields", "HKCU\\Software\\Neg\tA\tREG_SZ\n" },
            { "five fields", "HKCU\\Software\\Neg\tA\tREG_SZ\tx\ty\n" },
            { "root HKCR", "HKCR\\Software\\Neg\tA\tREG_SZ\tx\n" },
            { "root only", "HKCU\\\tA\tREG_SZ\tx\n" },
            { "no root", "Software\\Neg\tA\tREG_SZ\tx\n" },
            { "empty component", "HKCU\\Software\\\\Neg\tA\tREG_SZ\tx\n" },
            { "trailing backslash", "HKCU\\Software\\Neg\\\tA\tREG_SZ\tx\n" },
            { "component over 255", NULL },
            { "more than 32 components", NULL },
            { "type REG_MULTI_SZ", "HKCU\\Software\\Neg\tA\tREG_MULTI_SZ\tx\n" },
            { "type REG_QWORD", "HKCU\\Software\\Neg\tA\tREG_QWORD\t1\n" },
            { "type lower case", "HKCU\\Software\\Neg\tA\treg_sz\tx\n" },
            { "dword not a number", "HKCU\\Software\\Neg\tA\tREG_DWORD\t12ab\n" },
            { "dword too large", "HKCU\\Software\\Neg\tA\tREG_DWORD\t4294967296\n" },
            { "dword hex too large", "HKCU\\Software\\Neg\tA\tREG_DWORD\t0x100000000\n" },
            { "dword empty", "HKCU\\Software\\Neg\tA\tREG_DWORD\t\n" },
            { "dword 0x only", "HKCU\\Software\\Neg\tA\tREG_DWORD\t0x\n" },
            { "binary odd digits", "HKCU\\Software\\Neg\tA\tREG_BINARY\t0\n" },
            { "binary not hex", "HKCU\\Software\\Neg\tA\tREG_BINARY\t0g\n" },
            { "non-ASCII data", "HKCU\\Software\\Neg\tA\tREG_SZ\tcaf\xc3\xa9\n" },
            { "control character in the name", "HKCU\\Software\\Neg\tA\x01\tREG_SZ\tx\n" },
            { "same value twice", "HKCU\\Software\\Neg\tA\tREG_SZ\tx\nhkcu\\software\\NEG\ta\tREG_SZ\ty\n" },
        };
        size_t i;
        char text[4096], longc[300];
        memset(longc, 'k', 256);
        longc[256] = 0;
        for (i = 0; i < sizeof bad / sizeof bad[0]; i++) {
            char nm[200];
            if (strcmp(bad[i].name, "component over 255") == 0) snprintf(text, sizeof text, "HKCU\\Software\\Ok\tA\tREG_DWORD\t1\nHKCU\\%s\tA\tREG_SZ\tx\n", longc);
            else if (strcmp(bad[i].name, "more than 32 components") == 0) {
                size_t k;
                char key[200] = "HKCU\\a";
                for (k = 0; k < 32; k++) strcat(key, "\\b");
                snprintf(text, sizeof text, "HKCU\\Software\\Ok\tA\tREG_DWORD\t1\n%s\tA\tREG_SZ\tx\n", key);
            } else snprintf(text, sizeof text, "HKCU\\Software\\Ok\tA\tREG_DWORD\t1\n%s", bad[i].bad);
            write_list(dir, "bad.tsv", text);
            path_of(p1, sizeof p1, dir, "bad.tsv");
            set_pattern();
            snprintf(nm, sizeof nm, "refused: %s", bad[i].name);
            check(run_list(p1) == -1, nm);
            snprintf(nm, sizeof nm, "refused: %s - nothing written", bad[i].name);
            check(read_back(0xFFFFFFFF80000001ull, "Software\\Ok", "A", &type, &size, data, sizeof data) == 2 &&
                      read_back(0xFFFFFFFF80000001ull, "Software\\Neg", "A", &type, &size, data, sizeof data) == 2,
                  nm);
        }
    }
    path_of(p1, sizeof p1, dir, "does-not-exist.tsv");
    check(run_list(p1) == -1, "a missing list file is refused");
    write_list(dir, "empty.tsv", "# only a comment\n\n");
    path_of(p1, sizeof p1, dir, "empty.tsv");
    check(run_list(p1) == -1, "a list without entries is refused");
    check(run_list("") == 0, "(an empty value is 'not asked for')");
    path_of(p1, sizeof p1, dir, "a.tsv");
    snprintf(joined, sizeof joined, "%s:", p1);
    check(run_list(joined) == -1, "an empty file name in the list of lists is refused");
    {
        char many[1200];
        int k;
        many[0] = 0;
        for (k = 0; k < 9; k++) {
            strcat(many, k ? ":" : "");
            strcat(many, p1);
        }
        check(run_list(many) == -1, "more than 8 list files are refused");
    }
    /* a value already in the registry is not an error: a second run of a good list sets the same values again */
    path_of(p3, sizeof p3, dir, "good.tsv");
    check(run_list(p3) == 0, "a second run of the same list succeeds (RegCreateKeyExW opens, RegSetValueExW overwrites)");

    printf("%d failure(s)\n", failures);
    return failures;
}
