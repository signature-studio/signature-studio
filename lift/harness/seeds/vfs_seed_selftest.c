/* Self-test of lift/harness/vfs_seed.h (decision 0110). Links the virtual file system of the runtime and nothing of the lifted code, so it
 * builds and runs in seconds, apart from the 6-module build. Writes only into the directory that is its first argument (which must exist).
 *
 *   cd lift && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness \
 *       runtime/fle_vfs.c runtime/fle_reg.c runtime/fle_env.c runtime/fle_imports.c runtime/fle_modules.c runtime/fle_string.c \
 *       harness/seeds/vfs_seed_selftest.c -o $OUT/vfs_seed_selftest && $OUT/vfs_seed_selftest <existing empty dir>
 *
 * Prints "ok <name>" or "FAIL <name>" per check; the exit code is the number of failures (0 = all passed). Expected SHA-256 values are
 * standard test vectors (FIPS 180-4 examples) or were computed with Python's hashlib from the same byte formula, not by this code. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "vfs_seed.h"

uint64_t R[16];

static int failures;

static void check(int cond, const char *name)
{
    printf("%s %s\n", cond ? "ok  " : "FAIL", name);
    if (!cond) failures++;
}

static uint32_t attrs_of(const uint16_t *w)
{
    return (uint32_t)VS_CALL(fle_imp_GetFileAttributesW, VS_A(w));
}

static void write_file(const char *dir, const char *name, const uint8_t *data, size_t n)
{
    char path[1024];
    FILE *f;
    snprintf(path, sizeof path, "%s/%s", dir, name);
    f = fopen(path, "wb");
    if (!f || (n && fwrite(data, 1, n, f) != n)) {
        fprintf(stderr, "selftest: cannot write %s\n", path);
        exit(99);
    }
    fclose(f);
}

static void write_list(const char *dir, const char *name, const char *text)
{
    write_file(dir, name, (const uint8_t *)text, strlen(text));
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

/* the content of the virtual file at `w`, read through the guest-facing stubs: returns its size, or -1 */
static long read_back(const uint16_t *w, uint8_t *buf, size_t cap)
{
    uint64_t h;
    uint32_t n = 0, size;
    h = VS_CALL(fle_imp_CreateFileW, VS_A(w), 0x80000000u, 1, 0, 3, 0x80, 0);
    if (h == FLE_INVALID_HANDLE_VALUE) return -1;
    size = (uint32_t)VS_CALL(fle_imp_GetFileSize, h, 0);
    if (size > cap) return -1;
    if (size && (VS_CALL(fle_imp_ReadFile, h, VS_A(buf), size, VS_A(&n), 0) != 1 || n != size)) return -1;
    VS_CALL(fle_imp_CloseHandle, h);
    return (long)size;
}

int main(int argc, char **argv)
{
    const char *dir = argc > 1 ? argv[1] : NULL;
    char hex[65], list[1024], text[4096];
    uint8_t *big, back[100000];
    size_t i;
    long n;
    uint32_t a;
    uint64_t h;

    if (!dir) {
        fprintf(stderr, "usage: %s <existing directory to write test files into>\n", argv[0]);
        return 64;
    }

    /* SHA-256 */
    vs_sha256_hex((const uint8_t *)"abc", 3, hex);
    check(strcmp(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad") == 0, "sha256 abc");
    vs_sha256_hex((const uint8_t *)"", 0, hex);
    check(strcmp(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855") == 0, "sha256 empty");
    vs_sha256_hex((const uint8_t *)"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", 56, hex);
    check(strcmp(hex, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1") == 0, "sha256 56 bytes (two padding blocks)");
    big = malloc(1000000);
    memset(big, 'a', 1000000);
    vs_sha256_hex(big, 1000000, hex);
    check(strcmp(hex, "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0") == 0, "sha256 one million 'a'");
    free(big);
    big = malloc(100000);
    for (i = 0; i < 100000; i++) big[i] = (uint8_t)((i * 7 + 3) & 0xff);
    vs_sha256_hex(big, 100000, hex);
    check(strcmp(hex, "d96bab6a55ee326ba206dd4a85a6e95e14360d7fabbf448f03e689c24382b7d0") == 0, "sha256 pattern 100000 bytes");

    /* off by default: unset, then empty */
    unsetenv(VFS_SEED_ENV);
    set_pattern();
    check(vfs_seed_from_env() == 0, "unset: returns 0");
    check(pattern_intact(), "unset: R[] untouched");
    check(attrs_of(u"C:\\Program Files") == FLE_INVALID_FILE_ATTRIBUTES, "unset: the volume is still empty");
    setenv(VFS_SEED_ENV, "", 1);
    check(vfs_seed_from_env() == 0, "empty value: returns 0");
    unsetenv(VFS_SEED_ENV);

    /* a good list: a file, an empty file and a 100000-byte file; one SHA-256 in upper case; relative host names */
    write_file(dir, "abc.bin", (const uint8_t *)"abc", 3);
    write_file(dir, "empty.bin", (const uint8_t *)"", 0);
    write_file(dir, "big.bin", big, 100000);
    snprintf(text, sizeof text,
             "# a comment\n\nC:\\Program Files\\Image-Line\\X\\a\\abc.bin\tabc.bin\tBA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD\r\n"
             "C:\\Program Files\\Image-Line\\X\\a\\b\\empty.bin\tempty.bin\te3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n"
             "c:\\Program Files\\Image-Line\\X\\a\\BIG.bin\t./big.bin\td96bab6a55ee326ba206dd4a85a6e95e14360d7fabbf448f03e689c24382b7d0");
    write_list(dir, "ok.tsv", text);
    snprintf(list, sizeof list, "%s/ok.tsv", dir);
    setenv(VFS_SEED_ENV, list, 1);
    setenv(VFS_SEED_ROOT_ENV, dir, 1);
    set_pattern();
    check(vfs_seed_from_env() == 0, "good list: returns 0");
    check(pattern_intact(), "good list: R[] put back");
    /* the 3 seeded files used the file handle numbers 1..3 (never reused), so the first handle the guest gets is the 4th */
    h = VS_CALL(fle_imp_CreateFileW, VS_A(u"C:\\Program Files\\Image-Line\\X\\a\\abc.bin"), 0x80000000u, 1, 0, 3, 0x80, 0);
    check(h == FLE_VFS_FILE_BASE + 4u * 4u, "good list: the first guest handle is the 4th (the seeding used 3 handle numbers)");
    VS_CALL(fle_imp_CloseHandle, h);
    check((attrs_of(u"C:\\Program Files\\Image-Line\\X\\a\\b") & 0x10u) != 0, "good list: a directory made on the way exists");
    a = attrs_of(u"C:\\Program Files\\Image-Line\\X\\a\\abc.bin");
    check(a != FLE_INVALID_FILE_ATTRIBUTES && !(a & 0x10u), "good list: the file exists and is not a directory");
    n = read_back(u"C:\\Program Files\\Image-Line\\X\\a\\abc.bin", back, sizeof back);
    check(n == 3 && memcmp(back, "abc", 3) == 0, "good list: abc.bin reads back");
    n = read_back(u"C:\\Program Files\\Image-Line\\X\\a\\b\\empty.bin", back, sizeof back);
    check(n == 0, "good list: empty.bin has size 0");
    n = read_back(u"C:\\Program Files\\Image-Line\\X\\a\\big.bin", back, sizeof back);
    check(n == 100000 && memcmp(back, big, 100000) == 0, "good list: big.bin reads back (the guest path is case-insensitive)");
    fle_imp_GetLastError();
    check(R[0] == 0, "good list: the last error is 0");
    check(attrs_of(u"C:\\Windows") == FLE_INVALID_FILE_ATTRIBUTES, "good list: nothing outside the list appeared");
    /* the same list again: CREATE_NEW refuses to overwrite */
    set_pattern();
    check(vfs_seed_from_env() == -1, "second run of the same list: refused (CREATE_NEW)");
    check(pattern_intact(), "second run: R[] put back");

    /* refusals leave the volume untouched: the first entry is good, the second is not */
    write_file(dir, "one.bin", (const uint8_t *)"abc", 3);
    {
        static const struct { const char *name, *second; } bad[] = {
            { "sha mismatch", "C:\\neg\\two.bin\tabc.bin\t0000000000000000000000000000000000000000000000000000000000000000\n" },
            { "sha not hex", "C:\\neg\\two.bin\tabc.bin\tzzzz\n" },
            { "missing host file", "C:\\neg\\two.bin\tno-such-file.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n" },
            { "relative host with ..", "C:\\neg\\two.bin\t../abc.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n" },
            { "duplicate guest path", "c:\\NEG\\ONE.bin\tabc.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n" },
            { "guest path with a dot component", "C:\\neg\\.\\two.bin\tabc.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n" },
            { "guest path on another drive", "D:\\neg\\two.bin\tabc.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n" },
            { "two fields only", "C:\\neg\\two.bin\tabc.bin\n" },
            { "four fields", "C:\\neg\\two.bin\tabc.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\tx\n" },
        };
        for (i = 0; i < sizeof bad / sizeof bad[0]; i++) {
            char nm[128];
            snprintf(text, sizeof text, "C:\\neg\\one.bin\tone.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n%s", bad[i].second);
            write_list(dir, "bad.tsv", text);
            snprintf(list, sizeof list, "%s/bad.tsv", dir);
            setenv(VFS_SEED_ENV, list, 1);
            snprintf(nm, sizeof nm, "refused (%s): returns -1", bad[i].name);
            check(vfs_seed_from_env() == -1, nm);
            snprintf(nm, sizeof nm, "refused (%s): nothing written (C:\\neg does not exist)", bad[i].name);
            check(attrs_of(u"C:\\neg") == FLE_INVALID_FILE_ATTRIBUTES, nm);
        }
    }
    /* a relative host name without FLE_VFS_SEED_ROOT, and a missing list */
    unsetenv(VFS_SEED_ROOT_ENV);
    snprintf(list, sizeof list, "%s/bad.tsv", dir);
    write_list(dir, "bad.tsv", "C:\\neg\\one.bin\tone.bin\tba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n");
    check(vfs_seed_from_env() == -1 && attrs_of(u"C:\\neg") == FLE_INVALID_FILE_ATTRIBUTES, "relative host name without FLE_VFS_SEED_ROOT: refused");
    setenv(VFS_SEED_ENV, "/nonexistent-dir/no-list.tsv", 1);
    check(vfs_seed_from_env() == -1, "missing list file: refused");
    unsetenv(VFS_SEED_ENV);
    free(big);

    printf("%d failure(s)\n", failures);
    return failures;
}
