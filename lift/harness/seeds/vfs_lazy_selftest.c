/* Self-test of the lazy files of the virtual volume (decision 0314): lift/runtime/fle_vfs.c (fle_vfs_put_lazy_file and the copy-on-write sites) and lift/harness/vfs_lazy.h (the FLVFSIX1 reader, reached through the seed list's "@lazy:" row).
 * Links the virtual file system of the runtime and nothing of the lifted code. Writes only into the directory that is its first argument (which must exist). The index it reads is made by this file (a small writer of the
 * grammar in the decision); the streams are plain files of known bytes.
 *
 *   cd lift && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -fsanitize=address,undefined -I runtime -I harness \
 *       runtime/fle_vfs.c runtime/fle_reg.c runtime/fle_env.c runtime/fle_imports.c runtime/fle_modules.c runtime/fle_string.c \
 *       harness/seeds/vfs_lazy_selftest.c -o $OUT/vfs_lazy_selftest && $OUT/vfs_lazy_selftest <existing empty dir>
 *
 * Prints "ok <name>" or "FAIL <name>" per check; the exit code is the number of failures (0 = all passed). The expected bytes come from the formula byte(p) = (p * 13 + 7) & 0xff of the position p in a stream, not from the code under test.
 * With three more arguments, <index.bin> <directory of the stream files> <directory of the original tree>, it also reads every file of an index made by another tool (tools/vfs-build.py) through the volume and compares it with the original. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "vfs_seed.h"

uint64_t R[16];

static int failures;

static void check(int cond, const char *name)
{
    printf("%s %s\n", cond ? "ok  " : "FAIL", name);
    if (!cond) failures++;
}

/* ---- a writer of the index grammar ---- */

typedef struct {
    uint8_t *b;
    size_t n, cap;
} wbuf;

static void wput(wbuf *w, const void *p, size_t n)
{
    if (w->n + n > w->cap) {
        w->cap = (w->n + n) * 2 + 64;
        w->b = realloc(w->b, w->cap);
    }
    memcpy(w->b + w->n, p, n);
    w->n += n;
}

static void w8(wbuf *w, unsigned v)
{
    uint8_t c = (uint8_t)v;
    wput(w, &c, 1);
}

static void wle(wbuf *w, uint64_t v, unsigned bytes)
{
    unsigned k;
    for (k = 0; k < bytes; k++) w8(w, (unsigned)(v >> (8 * k)) & 0xff);
}

static void wvar(wbuf *w, uint64_t v)
{
    while (v >= 0x80) { w8(w, (unsigned)(v & 0x7f) | 0x80); v >>= 7; }
    w8(w, (unsigned)v);
}

typedef struct {
    int kind;                 /* 0 file, 1 directory */
    const char *path;         /* UTF-8, '/', relative to the mount */
    int64_t dsec;             /* mtime - base, in seconds */
    unsigned attrs;
    unsigned stream;
    uint64_t size;
} ent;

#define BASE_FT UINT64_C(134344736000000000)
#define MOUNT "C:\\Program Files\\Image-Line\\FL Studio 21"
#define PAT(p) ((uint8_t)(((p) * 13u + 7u) & 0xffu))

static const ent T[] = {
    { 1, "Data", 5, 0, 0, 0 },
    { 1, "Data/Empty folder", 6, 0, 0, 0 },
    { 1, "Data/Patches", 7, 0, 0, 0 },
    { 0, "Data/Patches/a.txt", 100, 0x20, 0, 10 },
    { 0, "Data/Patches/zero.bin", 101, 0x20, 0, 0 },
    { 0, "Data/Patches/\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e.txt", -50, 0x21, 0, 5 },
    { 0, "Data/Patches/eager.txt", 1, 0x20, 0, 7 },
    { 0, "Data/b.bin", 3, 0x20, 0, 3000 },
    { 0, "Data/big.dat", 4000, 0x20, 1, 10000 },
};
#define NT (sizeof T / sizeof T[0])
#define LOGICAL0 3022u
#define LOGICAL1 10000u
#define CHUNK0 1024u
#define CHUNK1 4096u

typedef struct {
    unsigned extra_streams;   /* streams with no files added after the two */
    uint64_t chunk0;          /* chunk size of stream 0 (CHUNK0 when 0) */
    const char *mount;
    const char *name0;
    uint32_t flags;
    int64_t logical0_delta;
    int trailing;
    int count_delta;          /* added to n_files in the header */
    int stream_of_last;       /* stream number written for the last entry (-1: the real one) */
    int nchunks_delta;        /* added to the chunk count of stream 0 */
} iopt;

static void build_index(wbuf *w, const iopt *o)
{
    const char *mount = o->mount ? o->mount : MOUNT;
    uint64_t c0 = o->chunk0 ? o->chunk0 : CHUNK0;
    uint64_t l0 = (uint64_t)((int64_t)LOGICAL0 + o->logical0_delta);
    unsigned ns = 2 + o->extra_streams, s;
    size_t i, nd = 0, nf = 0, common;
    const char *prev = "";
    memset(w, 0, sizeof *w);
    wput(w, "FLVFSIX1", 8);
    wle(w, 1, 4);
    wle(w, o->flags, 4);
    wle(w, 262144, 4);
    wle(w, BASE_FT, 8);
    for (i = 0; i < NT; i++) { if (T[i].kind) nd++; else nf++; }
    wvar(w, ns);
    wvar(w, nd);
    wvar(w, (uint64_t)((int64_t)nf + o->count_delta));
    wvar(w, strlen(mount));
    wput(w, mount, strlen(mount));
    for (s = 0; s < ns; s++) {
        char name[32];
        uint64_t logical = s == 0 ? l0 : s == 1 ? LOGICAL1 : 0, chunk = s == 0 ? c0 : s == 1 ? CHUNK1 : 4096, k, nch = (logical + chunk - 1) / chunk;
        if (s == 0 && o->name0) snprintf(name, sizeof name, "%s", o->name0);
        else snprintf(name, sizeof name, s == 0 ? "small-x" : s == 1 ? "large-y" : "extra-%u", s);
        wvar(w, strlen(name));
        wput(w, name, strlen(name));
        w8(w, s == 1 ? 1 : 0);
        wvar(w, chunk);
        wvar(w, s == 0 ? (uint64_t)((int64_t)nch + o->nchunks_delta) : nch);
        wvar(w, logical);
        for (k = 0; k < (s == 0 ? (uint64_t)((int64_t)nch + o->nchunks_delta) : nch); k++) {
            uint8_t h[32];
            memset(h, (int)(s * 16 + k), sizeof h);
            wput(w, h, sizeof h);
        }
    }
    for (i = 0; i < NT; i++) {
        const char *p = T[i].path;
        size_t pl = strlen(p), sl;
        int64_t d = T[i].dsec;
        unsigned stream = T[i].stream;
        for (common = 0; prev[common] && common < pl && prev[common] == p[common]; common++) {}
        sl = pl - common;
        w8(w, (unsigned)T[i].kind);
        wvar(w, common);
        wvar(w, sl);
        wput(w, p + common, sl);
        wvar(w, d >= 0 ? (uint64_t)d << 1 : (((uint64_t)(-d) << 1) - 1));
        w8(w, T[i].attrs);
        if (!T[i].kind) {
            if (i == NT - 1 && o->stream_of_last >= 0) stream = (unsigned)o->stream_of_last;
            w8(w, stream);
            wvar(w, T[i].size);
            if (o->flags & 1u) { uint8_t z[32] = { 0 }; wput(w, z, 32); }
        }
        prev = p;
    }
    if (o->trailing) w8(w, 0);
}

/* ---- the guest-side calls ---- */

static uint64_t open_ro(const uint16_t *w)
{
    return VS_CALL(fle_imp_CreateFileW, VS_A(w), 0x80000000u, 1, 0, 3, 0x80, 0);
}

static uint64_t open_rw(const uint16_t *w)
{
    return VS_CALL(fle_imp_CreateFileW, VS_A(w), 0xC0000000u, 0, 0, 3, 0x80, 0);
}

static uint32_t attrs_of(const uint16_t *w)
{
    return (uint32_t)VS_CALL(fle_imp_GetFileAttributesW, VS_A(w));
}

static long read_all(const uint16_t *w, uint8_t *buf, size_t cap)
{
    uint64_t h = open_ro(w);
    uint32_t n = 0, size;
    if (h == FLE_INVALID_HANDLE_VALUE) return -1;
    size = (uint32_t)VS_CALL(fle_imp_GetFileSize, h, 0);
    if (size > cap) return -1;
    if (size && (VS_CALL(fle_imp_ReadFile, h, VS_A(buf), size, VS_A(&n), 0) != 1 || n != size)) { VS_CALL(fle_imp_CloseHandle, h); return -1; }
    VS_CALL(fle_imp_CloseHandle, h);
    return (long)size;
}

static int is_pattern(const uint8_t *b, size_t n, uint64_t pos)
{
    size_t i;
    for (i = 0; i < n; i++)
        if (b[i] != PAT(pos + i)) return 0;
    return 1;
}

static uint64_t fetched_calls, fetched_bytes;
static int fail_reads;

static int counting_reader(void *ctx, unsigned stream, uint64_t off, void *dst, size_t n)
{
    fetched_calls++;
    fetched_bytes += n;
    if (fail_reads) return 0;
    return vl_read(ctx, stream, off, dst, n);
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

static void write_stream(const char *dir, const char *name, uint64_t n)
{
    uint8_t *b = malloc(n);
    uint64_t i;
    for (i = 0; i < n; i++) b[i] = PAT(i);
    write_file(dir, name, b, n);
    free(b);
}

static int quiet_parse(const wbuf *w, const char *mount)
{
    uint64_t f = 0, d = 0, k = 0, b = 0;
    int rc;
    FILE *keep = stderr;
    stderr = fopen("/dev/null", "w");
    rc = vl_parse(1, "t.tsv", 1, "vfs/index.bin", mount ? mount : MOUNT, w->b, w->n, &f, &d, &k, &b);
    fclose(stderr);
    stderr = keep;
    return rc;
}

int main(int argc, char **argv)
{
    const char *dir = argc > 1 ? argv[1] : NULL;
    char hex[65], list[1024], text[8192], vdir[1100];
    wbuf ix;
    iopt o;
    uint8_t back[20000];
    uint64_t h, lazy_bytes, lz;
    uint32_t cnt;
    size_t i, k;
    long n;
    uint16_t attr_data[18];

    if (!dir) {
        fprintf(stderr, "usage: %s <existing directory to write test files into> [<index.bin> <stream directory> <original tree>]\n", argv[0]);
        return 64;
    }
    snprintf(vdir, sizeof vdir, "%s/vfs", dir);
    mkdir(vdir, 0755);

    /* the files: streams, an eager file for the seed to win with, the list */
    write_stream(vdir, "small-x", LOGICAL0);
    write_stream(vdir, "large-y", LOGICAL1);
    memset(&o, 0, sizeof o);
    o.stream_of_last = -1;
    build_index(&ix, &o);
    write_file(vdir, "index.bin", ix.b, ix.n);
    vs_sha256_hex(ix.b, ix.n, hex);
    write_file(dir, "eager.bin", (const uint8_t *)"EAGER!!", 7);
    snprintf(text, sizeof text,
             "%s\\Data\\Patches\\eager.txt\teager.bin\t%s\n"
             "%s\\\t@lazy:vfs/index.bin\t%s\n",
             MOUNT, "0000000000000000000000000000000000000000000000000000000000000000", MOUNT, hex);
    {   /* the SHA-256 of the eager file */
        vs_sha256_hex((const uint8_t *)"EAGER!!", 7, hex);
        char *z = strstr(text, "0000000000000000000000000000000000000000000000000000000000000000");
        memcpy(z, hex, 64);
        vs_sha256_hex(ix.b, ix.n, hex);
    }
    write_file(dir, "lazy.tsv", (const uint8_t *)text, strlen(text));
    snprintf(list, sizeof list, "%s/lazy.tsv", dir);

    /* ---- the dry check: good, then every refusal ---- */
    check(quiet_parse(&ix, NULL) == 0, "the index is accepted");
    check(fle_vfs_lazy_files(NULL) == 0, "the check touches nothing: no lazy file yet");
    {
        static const struct { const char *name; iopt o; const char *mount; } bad[] = {
            { "17 streams", { .extra_streams = 15, .stream_of_last = -1 }, NULL },
            { "chunk size 9 MiB", { .chunk0 = (UINT64_C(9) << 20), .stream_of_last = -1 }, NULL },
            { "the chunk count is one too many for the stream", { .nchunks_delta = 1, .stream_of_last = -1 }, NULL },
            { "mount differs", { .stream_of_last = -1 }, "C:\\Other" },
            { "stream name with a slash", { .name0 = "a/b", .stream_of_last = -1 }, NULL },
            { "stream name '..'", { .name0 = "..", .stream_of_last = -1 }, NULL },
            { "unknown flag", { .flags = 2, .stream_of_last = -1 }, NULL },
            { "files do not add up to the stream", { .logical0_delta = 1, .stream_of_last = -1 }, NULL },
            { "a trailing byte", { .trailing = 1, .stream_of_last = -1 }, NULL },
            { "the file count is wrong", { .count_delta = 1, .stream_of_last = -1 }, NULL },
            { "a file names a stream that is not there", { .stream_of_last = 7 }, NULL },
        };
        for (i = 0; i < sizeof bad / sizeof bad[0]; i++) {
            wbuf b;
            char nm[160];
            build_index(&b, &bad[i].o);
            snprintf(nm, sizeof nm, "refused: %s", bad[i].name);
            check(quiet_parse(&b, bad[i].mount) != 0, nm);
            free(b.b);
        }
    }
    {   /* a flag bit0 index (a SHA-256 after each file) is read too */
        wbuf b;
        iopt f = { .flags = 1, .stream_of_last = -1 };
        build_index(&b, &f);
        check(quiet_parse(&b, NULL) == 0, "an index with the per-file SHA-256 flag is accepted");
        free(b.b);
    }
    {   /* every cut of the index is refused, and no byte flip crashes the reader (this binary is built with the sanitizers) */
        int all_cut = 1, flips_ok = 1;
        for (k = 0; k < ix.n; k++) {
            wbuf cut = { ix.b, k, k };
            if (quiet_parse(&cut, NULL) == 0) all_cut = 0;
        }
        check(all_cut, "every proper prefix of the index is refused");
        for (k = 0; k < ix.n; k++) {
            wbuf c = { malloc(ix.n), ix.n, ix.n };
            memcpy(c.b, ix.b, ix.n);
            c.b[k] ^= 0xff;
            (void)quiet_parse(&c, NULL);
            c.b[k] = (uint8_t)(ix.b[k] ^ 0x80);
            (void)quiet_parse(&c, NULL);
            free(c.b);
        }
        check(flips_ok, "no single-byte change of the index makes the reader fail by crashing");
    }
    check(attrs_of(u"C:\\Program Files") == FLE_INVALID_FILE_ATTRIBUTES, "after the refusals the volume is still empty");

    /* ---- load through the seed list: the eager row first, then the index ---- */
    setenv(VFS_SEED_ENV, list, 1);
    setenv(VFS_SEED_ROOT_ENV, dir, 1);
    {
        uint64_t save[16];
        for (i = 0; i < 16; i++) R[i] = 0x1111000000000000ull + i;
        memcpy(save, R, sizeof save);
        check(vfs_seed_from_env() == 0, "the list with the @lazy row loads");
        check(memcmp(save, R, sizeof save) == 0, "R[] put back");
    }
    fle_vfs_lazy_set_reader(counting_reader, NULL);
    lz = fle_vfs_lazy_files(&lazy_bytes);
    check(lz == 4 && lazy_bytes == 10u + 5u + 3000u + 10000u, "4 lazy files (a.txt, the Japanese name, b.bin, big.dat; zero.bin is an ordinary empty file and eager.txt came from the list): count and bytes");
    check(fetched_calls == 0, "loading fetched nothing from the streams");
    check((attrs_of(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Empty folder") & 0x10u) != 0, "the empty directory exists");
    check((attrs_of(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\a.txt") & 0xFFu) == 0x20u, "a.txt: ARCHIVE");
    check((attrs_of(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\日本語.txt") & 0xFFu) == 0x21u, "the Japanese name: ARCHIVE | READONLY");
    check(attrs_of(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\A.TXT") == 0x20u, "the name is found without case");

    /* times and sizes through GetFileAttributesExW */
    memset(attr_data, 0, sizeof attr_data);
    check(VS_CALL(fle_imp_GetFileAttributesExW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat"), 0, VS_A(attr_data)) == 1, "GetFileAttributesExW(big.dat)");
    {
        uint64_t wt, ct;
        uint32_t lo, hi;
        memcpy(&ct, (char *)attr_data + 4, 8);
        memcpy(&wt, (char *)attr_data + 20, 8);
        memcpy(&hi, (char *)attr_data + 28, 4);
        memcpy(&lo, (char *)attr_data + 32, 4);
        check(wt == BASE_FT + UINT64_C(4000) * 10000000u && ct == wt, "big.dat: creation = last write = base + 4000 s");
        check(hi == 0 && lo == 10000u, "big.dat: size 10000");
    }

    /* reads: the whole file, then in pieces, at an odd offset, at the end */
    fetched_calls = fetched_bytes = 0;
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat", back, sizeof back);
    check(n == 10000 && is_pattern(back, 10000, 0), "big.dat reads back (stream 1 offset 0)");
    check(fetched_bytes == 10000 && fle_vfs_lazy_files(NULL) == 4, "reading asked the host for exactly the bytes read, and the file stayed lazy");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\b.bin", back, sizeof back);
    check(n == 3000 && is_pattern(back, 3000, 10u + 0u + 5u + 7u), "b.bin reads back (stream 0, after a.txt, zero.bin, the Japanese name and eager.txt)");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\日本語.txt", back, sizeof back);
    check(n == 5 && is_pattern(back, 5, 10), "the Japanese-named file reads back (offset 10)");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\zero.bin", back, sizeof back);
    check(n == 0, "zero.bin has size 0");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\eager.txt", back, sizeof back);
    check(n == 7 && memcmp(back, "EAGER!!", 7) == 0, "eager.txt: the bytes of the list won over the index");
    h = open_ro(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat");
    {
        uint32_t got = 0;
        int ok = 1;
        uint64_t pos = 0;
        long lo = 1234;
        VS_CALL(fle_imp_SetFilePointer, h, (uint64_t)lo, 0, 0);
        fetched_calls = fetched_bytes = 0;
        while (pos + 1234 < 10000) {
            uint32_t want = 777;
            if (pos + 1234 + want > 10000) want = (uint32_t)(10000 - pos - 1234);
            if (VS_CALL(fle_imp_ReadFile, h, VS_A(back), want, VS_A(&got), 0) != 1 || got != want || !is_pattern(back, want, pos + 1234)) { ok = 0; break; }
            pos += want;
        }
        check(ok && pos + 1234 == 10000, "big.dat read in 777-byte pieces from offset 1234");
        VS_CALL(fle_imp_ReadFile, h, VS_A(back), 10, VS_A(&got), 0);
        check(R[0] == 1 && got == 0, "a read at the end answers 1 with 0 bytes (decision 0206)");
    }
    VS_CALL(fle_imp_CloseHandle, h);

    /* copy: lazy to lazy, nothing fetched */
    fetched_calls = fetched_bytes = 0;
    check(VS_CALL(fle_imp_CopyFileW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat"), VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\copy.dat"), 1) == 1, "CopyFileW of a lazy file");
    check(fetched_calls == 0 && fle_vfs_lazy_files(NULL) == 5, "the copy is lazy and fetched nothing");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\copy.dat", back, sizeof back);
    check(n == 10000 && is_pattern(back, 10000, 0), "the copy reads as the original");

    /* write: copy-on-write of the whole file, the stream file stays as it is */
    h = open_rw(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\copy.dat");
    VS_CALL(fle_imp_SetFilePointer, h, 2, 0, 0);
    fetched_calls = fetched_bytes = 0;
    cnt = 0;
    check(VS_CALL(fle_imp_WriteFile, h, VS_A("XYZ"), 3, VS_A(&cnt), 0) == 1 && cnt == 3, "WriteFile into a lazy file");
    check(fetched_bytes == 10000 && fle_vfs_lazy_files(NULL) == 4, "the write read the whole file once and it is ordinary now");
    VS_CALL(fle_imp_CloseHandle, h);
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\copy.dat", back, sizeof back);
    check(n == 10000 && memcmp(back + 2, "XYZ", 3) == 0 && is_pattern(back, 2, 0) && is_pattern(back + 5, 9995, 5), "the written file: 3 bytes changed, the rest as it was");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat", back, sizeof back);
    check(n == 10000 && is_pattern(back, 10000, 0), "the original is unchanged by the write to the copy");
    {
        FILE *f;
        uint8_t again[10000];
        char p[1200];
        snprintf(p, sizeof p, "%s/large-y", vdir);
        f = fopen(p, "rb");
        check(f && fread(again, 1, 10000, f) == 10000 && is_pattern(again, 10000, 0), "the stream file on the host is unchanged");
        if (f) fclose(f);
    }

    /* SetEndOfFile (fle_vfs_file_set_end), CREATE_ALWAYS (truncate), delete, peek, move */
    h = open_rw(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\b.bin");
    VS_CALL(fle_imp_SetFilePointer, h, 100, 0, 0);
    check(fle_vfs_file_set_end("t", h) == 1, "SetEndOfFile on a lazy file");
    VS_CALL(fle_imp_CloseHandle, h);
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\b.bin", back, sizeof back);
    check(n == 100 && is_pattern(back, 100, 22), "b.bin cut to 100 bytes keeps its first bytes");
    h = VS_CALL(fle_imp_CreateFileW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\a.txt"), 0x40000000u, 0, 0, 2, 0x80, 0);
    check(h != FLE_INVALID_HANDLE_VALUE, "CREATE_ALWAYS on a lazy file");
    VS_CALL(fle_imp_CloseHandle, h);
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\a.txt", back, sizeof back);
    check(n == 0, "a.txt is empty after CREATE_ALWAYS");
    check(fle_vfs_lazy_files(NULL) == 2, "lazy files left: big.dat and the Japanese name");
    check(VS_CALL(fle_imp_MoveFileW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\big.dat"), VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\moved.dat")) == 1, "MoveFileW of a lazy file");
    n = read_all(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\moved.dat", back, sizeof back);
    check(n == 10000 && is_pattern(back, 10000, 0), "the moved file reads as before");
    {
        const uint8_t *data = NULL;
        uint64_t sz = 0;
        check(fle_vfs_peek_file(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\moved.dat", &data, &sz) == 1 && sz == 10000 && data && is_pattern(data, 10000, 0), "fle_vfs_peek_file on a lazy file gives the bytes");
        check(fle_vfs_lazy_files(NULL) == 1, "the peek made the file ordinary");
    }
    check(VS_CALL(fle_imp_DeleteFileW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\日本語.txt")) == 0, "DeleteFileW of a READONLY lazy file fails (as for any read-only file)");
    check(VS_CALL(fle_imp_SetFileAttributesW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\日本語.txt"), 0x80) == 1, "SetFileAttributesW(NORMAL) on a lazy file");
    check(VS_CALL(fle_imp_DeleteFileW, VS_A(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Data\\Patches\\日本語.txt")) == 1, "DeleteFileW of a lazy file");
    check(fle_vfs_lazy_files(NULL) == 0, "no lazy file is left");

    /* the time of a file the guest makes does not depend on the lazy entries */
    h = VS_CALL(fle_imp_CreateFileW, VS_A(u"C:\\new.txt"), 0x40000000u, 0, 0, 1, 0x80, 0);
    VS_CALL(fle_imp_CloseHandle, h);
    memset(attr_data, 0, sizeof attr_data);
    VS_CALL(fle_imp_GetFileAttributesExW, VS_A(u"C:\\new.txt"), 0, VS_A(attr_data));
    {
        uint64_t wt;
        memcpy(&wt, (char *)attr_data + 20, 8);
        check(wt > FLE_VFS_CLOCK_START && wt < FLE_VFS_CLOCK_START + UINT64_C(100) * 10000000u, "a new file's time is within 100 s of the virtual clock's start (the 9 lazy entries did not move it)");
    }

    /* an index made by another tool, if given */
    if (argc > 4) {
        /* the cases above are the self-contained part; the comparison with an original tree is run by tools/vfs-conformance.sh */
        printf("note: %s %s %s are for tools/vfs-conformance.sh\n", argv[2], argv[3], argv[4]);
    }

    free(ix.b);
    printf("%d failure(s)\n", failures);
    return failures;
}
