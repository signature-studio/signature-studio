/* Stage 3 test of decision 0216 (opfs1): the OPFS file library (fle_opfs_lib.js) used from C, compiled with emscripten (-sJSPI, shared memory like the engine's) and run in a dedicated Worker of Chrome or Firefox.
 * Prints 'T PASS|FAIL <name> <detail>' for checks, 'M <name> <value> <unit>' for measurements and 'DONE'. argv[1]: 'all' (default), 'persist-write' or 'persist-read' (two runs in two page loads). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_opfs.h"
#include "fle_opfs_state.h"

static int fails, checks;
#define CHECK(name, cond, ...) do { int ok_ = (cond); checks++; printf("T %s %s ", ok_ ? "PASS" : "FAIL", name); printf(__VA_ARGS__); printf("\n"); if (!ok_) fails++; } while (0)
static void M(const char *name, double v, const char *unit) { printf("M %s %.3f %s\n", name, v, unit); }
static uint8_t pat(double off) { return (uint8_t)(((uint64_t)off * 31u + 7u) & 255u); }
static void fill(uint8_t *b, int n, double at) { for (int i = 0; i < n; i++) b[i] = pat(at + i); }
static int verify(const uint8_t *b, int n, double at) { for (int i = 0; i < n; i++) if (b[i] != pat(at + i)) return 0; return 1; }
static int count_rows(const char *json) { int c = 0; for (const char *p = json; *p; p++) if (p[0] == '[' && p[1] == '"') c++; return c; }

static int write_pattern(const char *path, double total, int piece) {
    int fd = fle_opfs_open(path, FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
    if (fd < 0) return fd;
    uint8_t *b = malloc(piece);
    for (double off = 0; off < total; off += piece) { fill(b, piece, off); int w = fle_opfs_write(fd, b, piece, off); if (w != piece) { free(b); fle_opfs_close(fd); return -5; } }
    free(b);
    return fle_opfs_close(fd);
}
static int read_verify(const char *path, double total, int piece, double *bad) {
    int fd = fle_opfs_open(path, FLE_OPFS_READ);
    if (fd < 0) return fd;
    uint8_t *b = malloc(piece);
    *bad = 0;
    for (double off = 0; off < total; off += piece) { int r = fle_opfs_read(fd, b, piece, off); if (r != piece || !verify(b, piece, off)) *bad += 1; }
    free(b);
    fle_opfs_close(fd);
    return 0;
}

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "all";
    int r = fle_opfs_set_root("s3test");
    CHECK("set_root", r == 0, "%d", r);
    if (strcmp(mode, "persist-write") == 0) {
        fle_opfs_mkdir("p");
        int fd = fle_opfs_open("p/flushed.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
        uint8_t b[1000]; fill(b, 1000, 0);
        fle_opfs_write(fd, b, 1000, 0);
        CHECK("write-then-flush-without-close", fle_opfs_flush(fd) == 0, "fd %d", fd);   /* NOT closed: the page terminates the Worker */
        int fd2 = fle_opfs_open("p/buffered.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
        fle_opfs_write(fd2, b, 500, 0);   /* in the write-behind buffer, neither flushed nor closed */
        int fd3 = fle_opfs_open("p/closed.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
        fle_opfs_write(fd3, b, 700, 0);
        CHECK("write-then-close-without-fsync", fle_opfs_close(fd3) == 0, "ok");
        fle_opfs_state_put("p/state.bin", "OLD", 3);
        {   /* a put that is cut off after the .tmp was written and before the move: the old state must stay */
            int t = fle_opfs_open("p/state.bin.tmp", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
            fle_opfs_write(t, "NEW-HALF", 8, 0); fle_opfs_flush(t);   /* never closed or moved: the page terminates the Worker */
        }
        printf("DONE\n");
        return 0;
    }
    if (strcmp(mode, "persist-read") == 0) {
        double st[2]; int k = fle_opfs_stat("p/flushed.bin", st);
        CHECK("persist: flushed (not closed) file is there with its size", k == 1 && st[0] == 1000, "kind %d size %.0f", k, st[0]);
        int fd = fle_opfs_open("p/flushed.bin", FLE_OPFS_READ); uint8_t b[1000]; int n = fle_opfs_read(fd, b, 1000, 0); fle_opfs_close(fd);
        CHECK("persist: its bytes", n == 1000 && verify(b, 1000, 0), "%d", n);
        k = fle_opfs_stat("p/closed.bin", st);
        CHECK("persist: a file that was written and closed WITHOUT fsync survives the Worker's death and a page reload", k == 1 && st[0] == 700, "kind %d size %.0f", k, st[0]);
        { size_t zs = 0; int ze = 0; void *zg = fle_opfs_state_get("p/state.bin", &zs, &ze);
          CHECK("persist: the state of a put that was cut off after the .tmp was written is still the OLD one", zg != NULL && zs == 3 && memcmp(zg, "OLD", 3) == 0, "%zu", zs); free(zg); }
        k = fle_opfs_stat("p/buffered.bin", st);
        CHECK("persist: a write still in the write-behind buffer when the Worker is killed is LOST (size on disk)", k == 1 && st[0] == 0, "kind %d size %.0f", k, st[0]);
        printf("DONE\n");
        return 0;
    }

    /* 1. a 5 MiB file written and read in 4 KiB pieces, with the cache (256 KiB) */
    double t0 = fle_opfs_now();
    r = write_pattern("a/b/five.bin", 5.0 * 1048576, 4096);
    double tw = fle_opfs_now() - t0;
    CHECK("write 5 MiB in 4 KiB pieces", r == 0, "%d", r);
    double st[2]; int k = fle_opfs_stat("a/b/five.bin", st);
    CHECK("stat: a file of 5 MiB", k == 1 && st[0] == 5.0 * 1048576, "kind %d size %.0f mtime %.0f", k, st[0], st[1]);
    k = fle_opfs_stat("a/b", st); CHECK("stat: a directory", k == 2, "%d", k);
    k = fle_opfs_stat("a/nothing", st); CHECK("stat: a missing path is 0", k == 0, "%d", k);
    double bad; t0 = fle_opfs_now();
    r = read_verify("a/b/five.bin", 5.0 * 1048576, 4096, &bad);
    double tr = fle_opfs_now() - t0;
    CHECK("read it back in 4 KiB pieces: every byte right", r == 0 && bad == 0, "bad pieces %.0f", bad);
    M("cached 4K write", tw / 1280 * 1000, "us/call");
    M("cached 4K sequential read", tr / 1280 * 1000, "us/call");
    char *js = fle_opfs_stats(); printf("S cached %s\n", js); free(js);

    /* the same without the cache */
    fle_opfs_set_block(0);
    t0 = fle_opfs_now(); r = write_pattern("a/b/five-nocache.bin", 1048576.0, 4096); tw = fle_opfs_now() - t0;
    t0 = fle_opfs_now(); read_verify("a/b/five-nocache.bin", 1048576.0, 4096, &bad); tr = fle_opfs_now() - t0;
    M("uncached 4K write", tw / 256 * 1000, "us/call");
    M("uncached 4K sequential read", tr / 256 * 1000, "us/call");
    CHECK("uncached read right too", bad == 0, "%.0f", bad);
    fle_opfs_set_block(256 * 1024);

    /* 2. random 4 KiB reads: window 256 KiB (a hit costs a copy) vs none */
    {
        int fd = fle_opfs_open("a/b/five.bin", FLE_OPFS_READ); uint8_t b[4096]; uint32_t s = 12345; int n = 1500, okc = 0;
        t0 = fle_opfs_now();
        for (int i = 0; i < n; i++) { s = s * 1103515245u + 12345u; double at = (double)((s >> 4) % 1280) * 4096; if (fle_opfs_read(fd, b, 4096, at) == 4096 && verify(b, 4096, at)) okc++; }
        double t = fle_opfs_now() - t0;
        CHECK("random 4 KiB reads (cached) right", okc == n, "%d/%d", okc, n);
        M("cached random 4K read", t / n * 1000, "us/call");
        fle_opfs_close(fd);
        fle_opfs_set_block(0);
        fd = fle_opfs_open("a/b/five.bin", FLE_OPFS_READ); okc = 0; s = 12345;
        t0 = fle_opfs_now();
        for (int i = 0; i < n; i++) { s = s * 1103515245u + 12345u; double at = (double)((s >> 4) % 1280) * 4096; if (fle_opfs_read(fd, b, 4096, at) == 4096 && verify(b, 4096, at)) okc++; }
        t = fle_opfs_now() - t0;
        CHECK("random 4 KiB reads (uncached) right", okc == n, "%d/%d", okc, n);
        M("uncached random 4K read", t / n * 1000, "us/call");
        fle_opfs_close(fd);
        fle_opfs_set_block(256 * 1024);
    }

    /* 3. a 64 MiB file in 1 MiB blocks: no copy of the file is made anywhere (the guest memory is the only buffer) */
    {
        t0 = fle_opfs_now(); r = write_pattern("big/64.bin", 64.0 * 1048576, 1048576); tw = fle_opfs_now() - t0;
        CHECK("write 64 MiB in 1 MiB blocks", r == 0, "%d", r);
        t0 = fle_opfs_now(); r = read_verify("big/64.bin", 64.0 * 1048576, 1048576, &bad); tr = fle_opfs_now() - t0;
        CHECK("read 64 MiB in 1 MiB blocks: every byte right", r == 0 && bad == 0, "bad %.0f", bad);
        M("64 MiB write", 64.0 / (tw / 1000), "MiB/s");
        M("64 MiB read", 64.0 / (tr / 1000), "MiB/s");
        /* 64 KiB reads (below the cache block): what a sample loader with a small buffer does */
        t0 = fle_opfs_now(); r = read_verify("big/64.bin", 16.0 * 1048576, 65536, &bad); tr = fle_opfs_now() - t0;
        CHECK("read 16 MiB in 64 KiB pieces", r == 0 && bad == 0, "bad %.0f", bad);
        M("64 KiB reads", 16.0 / (tr / 1000), "MiB/s");
    }

    /* 4. one handle per path, shared */
    {
        int f1 = fle_opfs_open("share/x.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
        int f2 = fle_opfs_open("share/x.bin", FLE_OPFS_READ);
        CHECK("a second open of an open path works (one sync handle is shared)", f1 >= 3 && f2 >= 3 && f1 != f2, "%d %d", f1, f2);
        uint8_t b[100], c[100]; fill(b, 100, 0);
        fle_opfs_write(f1, b, 100, 0);
        CHECK("the size the guest sees includes buffered writes, through the other descriptor too", fle_opfs_size(f2) == 100, "%.0f", fle_opfs_size(f2));
        int n = fle_opfs_read(f2, c, 100, 0);
        CHECK("a read through the other descriptor sees the write", n == 100 && memcmp(b, c, 100) == 0, "%d", n);
        CHECK("removing an open file is EBUSY", fle_opfs_remove("share/x.bin") == -16, "%d", fle_opfs_remove("share/x.bin"));
        CHECK("renaming an open file is EBUSY", fle_opfs_rename("share/x.bin", "share/y.bin", 0) == -16, "%d", fle_opfs_rename("share/x.bin", "share/y.bin", 0));
        fle_opfs_close(f1);
        n = fle_opfs_read(f2, c, 100, 0);
        CHECK("after the first close the other descriptor still reads", n == 100, "%d", n);
        fle_opfs_close(f2);
        CHECK("rename after the last close", fle_opfs_rename("share/x.bin", "share/y.bin", 0) == 0, "ok");
        CHECK("the renamed file has its bytes", fle_opfs_stat("share/y.bin", st) == 1 && st[0] == 100, "%.0f", st[0]);
        CHECK("remove after the last close", fle_opfs_remove("share/y.bin") == 0, "ok");
        CHECK("removed", fle_opfs_stat("share/y.bin", st) == 0, "ok");
        CHECK("close of a descriptor twice is EBADF", fle_opfs_close(f2) == -9, "%d", fle_opfs_close(f2));
    }

    /* 5. errors */
    CHECK("open of a missing file without CREATE is ENOENT", fle_opfs_open("nothing/x", FLE_OPFS_READ) == -2, "%d", fle_opfs_open("nothing/x", FLE_OPFS_READ));
    CHECK("open EXCL of an existing file is EEXIST", fle_opfs_open("a/b/five.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_EXCL) == -17, "%d", fle_opfs_open("a/b/five.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_EXCL));
    CHECK("open of a directory as a file is EISDIR", fle_opfs_open("a/b", FLE_OPFS_READ) == -21, "%d", fle_opfs_open("a/b", FLE_OPFS_READ));
    CHECK("remove of a non-empty directory is ENOTEMPTY", fle_opfs_remove("a") == -39, "%d", fle_opfs_remove("a"));

    /* 6. truncate */
    {
        int fd = fle_opfs_open("a/b/five.bin", FLE_OPFS_READ | FLE_OPFS_WRITE);
        CHECK("truncate to 1000", fle_opfs_truncate(fd, 1000) == 0 && fle_opfs_size(fd) == 1000, "%.0f", fle_opfs_size(fd));
        uint8_t b[2000]; int n = fle_opfs_read(fd, b, 2000, 0);
        CHECK("a read past the end stops at the size", n == 1000 && verify(b, 1000, 0), "%d", n);
        n = fle_opfs_read(fd, b, 10, 5000);
        CHECK("a read beyond the end is 0", n == 0, "%d", n);
        fle_opfs_close(fd);
    }

    /* 7. directory listing, many files, UTF-8 names */
    {
        char name[64];
        t0 = fle_opfs_now();
        for (int i = 0; i < 300; i++) { snprintf(name, sizeof name, "many/f%04d.dat", i); int fd = fle_opfs_open(name, FLE_OPFS_WRITE | FLE_OPFS_CREATE); if (fd < 0) { CHECK("create 300 files", 0, "%d at %d", fd, i); break; } fle_opfs_write(fd, "x", 1, 0); fle_opfs_close(fd); }
        double tc = fle_opfs_now() - t0;
        M("create+write+close of a file", tc / 300, "ms/file");
        int err = 0; t0 = fle_opfs_now(); char *j = fle_opfs_list("many", &err); double tl = fle_opfs_now() - t0;
        CHECK("list: 300 entries", j != NULL && count_rows(j) == 300, "%d", j ? count_rows(j) : -err);
        M("list of 300 files (names, sizes, dates)", tl, "ms");
        free(j);
        t0 = fle_opfs_now(); j = fle_opfs_list_names("many", &err); double tn = fle_opfs_now() - t0;
        CHECK("list_names: 300 entries, names and kinds only", j != NULL && count_rows(j) == 300 && strstr(j, "\"f0007.dat\",1]") != NULL, "%d", j ? count_rows(j) : -err);
        M("list of 300 files, names only", tn, "ms");
        free(j);
        CHECK("list_names of a missing folder is NULL with ENOENT", fle_opfs_list_names("nope", &err) == NULL && err == -2, "%d", err);
        int fd = fle_opfs_open("日本語/サンプル #1.wav", FLE_OPFS_WRITE | FLE_OPFS_CREATE);
        CHECK("a UTF-8 path (Japanese folder and file) opens", fd >= 3, "%d", fd);
        fle_opfs_write(fd, "RIFF", 4, 0); fle_opfs_close(fd);
        j = fle_opfs_list("日本語", &err);
        CHECK("and is listed with its name", j != NULL && strstr(j, "サンプル #1.wav") != NULL, "%s", j ? j : "(null)");
        free(j);
        j = fle_opfs_list("missing-dir", &err);
        CHECK("list of a missing directory is NULL with ENOENT", j == NULL && err == -2, "%d", err);
    }

    /* 8. open+close cost */
    {
        t0 = fle_opfs_now();
        for (int i = 0; i < 100; i++) { int fd = fle_opfs_open("big/64.bin", FLE_OPFS_READ); fle_opfs_close(fd); }
        M("open + close of an existing file", (fle_opfs_now() - t0) / 100, "ms");
    }
    /* 8b. whole-file state (R4: registry subtree, licence files) */
    {
        size_t sz = 0; int er = 0; double sst[2];
        void *g = fle_opfs_state_get("state/registrations.bin", &sz, &er);
        CHECK("state: get of a missing file is ENOENT", g == NULL && er == -2, "%d", er);
        CHECK("state: put creates the file (and the folder)", fle_opfs_state_put("state/registrations.bin", "version-1", 9) == 0, "ok");
        g = fle_opfs_state_get("state/registrations.bin", &sz, &er);
        CHECK("state: get returns it", g != NULL && sz == 9 && memcmp(g, "version-1", 9) == 0 && ((char *)g)[9] == 0, "%zu", sz); free(g);
        CHECK("state: put again replaces it", fle_opfs_state_put("state/registrations.bin", "version-2-longer", 16) == 0, "ok");
        g = fle_opfs_state_get("state/registrations.bin", &sz, &er);
        CHECK("state: the new bytes", g != NULL && sz == 16 && memcmp(g, "version-2-longer", 16) == 0, "%zu", sz); free(g);
        CHECK("state: no .tmp is left behind", fle_opfs_stat("state/registrations.bin.tmp", sst) == 0, "ok");
        CHECK("state: an empty state is allowed", fle_opfs_state_put("state/empty.bin", "", 0) == 0 && ({ g = fle_opfs_state_get("state/empty.bin", &sz, &er); int ok = g != NULL && sz == 0; free(g); ok; }), "ok");
        int rd = fle_opfs_open("state/registrations.bin", FLE_OPFS_READ);
        CHECK("state: put over a file that is open for reading is EBUSY and leaves it intact", fle_opfs_state_put("state/registrations.bin", "xx", 2) == -16 && fle_opfs_size(rd) == 16, "ok");
        fle_opfs_close(rd);
        CHECK("state: no .tmp is left behind after the failure", fle_opfs_stat("state/registrations.bin.tmp", sst) == 0, "ok");
        {   /* a big state, 3 MiB */
            size_t big = 3u << 20; uint8_t *b = malloc(big); for (size_t i = 0; i < big; i++) b[i] = (uint8_t)(i * 7);
            double t0 = fle_opfs_now(); int pr = fle_opfs_state_put("state/big.bin", b, big); double tp = fle_opfs_now() - t0;
            t0 = fle_opfs_now(); g = fle_opfs_state_get("state/big.bin", &sz, &er); double tg = fle_opfs_now() - t0;
            CHECK("state: 3 MiB round trip", pr == 0 && g && sz == big && memcmp(g, b, big) == 0, "%d", pr);
            M("state put of 3 MiB (tmp, fsync, move)", tp, "ms"); M("state get of 3 MiB", tg, "ms");
            free(g); free(b);
        }
    }

    /* 9. Windows names ignore case, OPFS names do not: the library finds the entry that differs in case only */
    {
        int fd = fle_opfs_open("Case/Folder/Mixed.TXT", FLE_OPFS_WRITE | FLE_OPFS_CREATE);
        fle_opfs_write(fd, "abc", 3, 0); fle_opfs_close(fd);
        double cst[2];
        CHECK("case: stat with other case finds the file", fle_opfs_stat("case/FOLDER/mixed.txt", cst) == 1 && cst[0] == 3, "%.0f", cst[0]);
        CHECK("case: stat of the folder with other case", fle_opfs_stat("CASE/folder", cst) == 2, "ok");
        int f1 = fle_opfs_open("CASE/folder/MIXED.txt", FLE_OPFS_READ);
        char b3[3] = { 0 }; int n = f1 >= 3 ? fle_opfs_read(f1, b3, 3, 0) : -1;
        CHECK("case: open with other case reads the same bytes", f1 >= 3 && n == 3 && memcmp(b3, "abc", 3) == 0, "%d %d", f1, n);
        int f2 = fle_opfs_open("Case/Folder/Mixed.TXT", FLE_OPFS_READ);
        CHECK("case: an open file is found again under any spelling (one handle)", f2 >= 3 && fle_opfs_size(f2) == 3, "%d", f2);
        char *js2 = fle_opfs_stats(); CHECK("case: only one sync handle was opened for the two spellings", strstr(js2, "\"openFiles\":1") != NULL, "%s", js2); free(js2);
        fle_opfs_close(f1); fle_opfs_close(f2);
        int err = 0; char *j = fle_opfs_list("CASE/FOLDER", &err);
        CHECK("case: list under another spelling shows the OPFS spelling", j != NULL && strstr(j, "Mixed.TXT") != NULL, "%s", j ? j : "(null)");
        free(j);
        CHECK("case: create with another case opens the existing file, no second file", ({ int g = fle_opfs_open("case/folder/MIXED.TXT", FLE_OPFS_WRITE | FLE_OPFS_CREATE); if (g >= 3) fle_opfs_close(g); g >= 3; }), "ok");
        j = fle_opfs_list("Case/Folder", &err);
        CHECK("case: still one file in the folder", j != NULL && count_rows(j) == 1, "%d", j ? count_rows(j) : -err);
        free(j);
        CHECK("case: EXCL with other case is EEXIST", fle_opfs_open("case/folder/mixed.txt", FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_EXCL) == -17, "ok");
        CHECK("case: a folder created with another case is the existing folder", fle_opfs_mkdir("CASE/FOLDER") == 0 && ({ char *l = fle_opfs_list("Case", &err); int c = l && count_rows(l) == 1; free(l); c; }), "ok");
        CHECK("case: rename with only the case changed is allowed", fle_opfs_rename("Case/Folder/Mixed.TXT", "Case/Folder/mixed.txt", 0) == 0, "ok");
        j = fle_opfs_list("Case/Folder", &err);
        CHECK("case: and the name is now the new spelling", j != NULL && strstr(j, "\"mixed.txt\"") != NULL && count_rows(j) == 1, "%s", j ? j : "(null)");
        free(j);
        int g = fle_opfs_open("Case/Folder/Other.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE); fle_opfs_close(g);
        CHECK("rename onto an existing file is EEXIST", fle_opfs_rename("Case/Folder/other.BIN", "case/folder/MIXED.TXT", 0) == -17, "ok");
        CHECK("rename onto an existing file with the replace flag works", fle_opfs_rename("Case/Folder/other.BIN", "case/folder/MIXED.TXT", 1) == 0, "ok");
        j = fle_opfs_list("Case/Folder", &err);
        CHECK("case: one file left (the replaced one is gone)", j != NULL && count_rows(j) == 1, "%s", j ? j : "(null)");
        free(j);
        CHECK("remove with other case", fle_opfs_remove("CASE/FOLDER/MIXED.TXT") == 0 && fle_opfs_stat("case/folder/mixed.txt", cst) == 0, "ok");
        CHECK("remove of a missing name is ENOENT", fle_opfs_remove("case/folder/none") == -2, "ok");
        fle_opfs_set_casefold(0);
        CHECK("casefold off: another spelling is a different (missing) name", fle_opfs_stat("case/folder", cst) == 0 && fle_opfs_stat("Case/Folder", cst) == 2, "ok");
        fle_opfs_set_casefold(1);
        /* flush_all */
        int w = fle_opfs_open("Case/Folder/flushall.bin", FLE_OPFS_WRITE | FLE_OPFS_CREATE);
        fle_opfs_write(w, "zz", 2, 0);
        CHECK("flush_all reports one file with buffered writes", fle_opfs_flush_all() == 1, "ok");
        CHECK("and a second call finds nothing buffered", fle_opfs_flush_all() == 0, "ok");
        fle_opfs_close(w);
        /* close with and without fsync */
        {
            char nm[64]; double ta, tb;
            fle_opfs_mkdir("fs");
            fle_opfs_set_fsync_on_close(0);
            ta = fle_opfs_now();
            for (int i = 0; i < 50; i++) { snprintf(nm, sizeof nm, "fs/a%02d.bin", i); int q = fle_opfs_open(nm, FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC); fle_opfs_write(q, "0123456789", 10, 0); fle_opfs_close(q); }
            ta = (fle_opfs_now() - ta) / 50;
            fle_opfs_set_fsync_on_close(1);
            tb = fle_opfs_now();
            for (int i = 0; i < 50; i++) { snprintf(nm, sizeof nm, "fs/b%02d.bin", i); int q = fle_opfs_open(nm, FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC); fle_opfs_write(q, "0123456789", 10, 0); fle_opfs_close(q); }
            tb = (fle_opfs_now() - tb) / 50;
            fle_opfs_set_fsync_on_close(0);
            M("create + write 10 bytes + close, no fsync at close (default)", ta, "ms/file");
            M("create + write 10 bytes + close, fsync at close", tb, "ms/file");
            CHECK("both kinds of close wrote the bytes", fle_opfs_stat("fs/a49.bin", cst) == 1 && cst[0] == 10 && fle_opfs_stat("fs/b49.bin", cst) == 1 && cst[0] == 10, "ok");
            double tr0 = fle_opfs_now();
            for (int i = 0; i < 50; i++) { snprintf(nm, sizeof nm, "fs/a%02d.bin", i); int q = fle_opfs_open(nm, FLE_OPFS_READ); fle_opfs_close(q); }
            M("open + close of a file that was only read (no fsync, nothing written)", (fle_opfs_now() - tr0) / 50, "ms/file");
        }
        /* the cost of a miss in a big folder: the folder is listed to look for a name that differs in case only */
        double t1 = fle_opfs_now();
        int mm = fle_opfs_stat("many/F9999.dat", cst);
        double miss_on = fle_opfs_now() - t1;
        fle_opfs_set_casefold(0);
        t1 = fle_opfs_now();
        int mm2 = fle_opfs_stat("many/f9999.dat", cst);
        double miss_off = fle_opfs_now() - t1;
        fle_opfs_set_casefold(1);
        CHECK("a miss in a 300-file folder is a miss", mm == 0 && mm2 == 0, "%d %d", mm, mm2);
        M("stat of a missing name in a 300-file folder, casefold on (lists the folder)", miss_on, "ms");
        M("stat of a missing name in a 300-file folder, casefold off", miss_off, "ms");
    }
    js = fle_opfs_stats(); printf("S final %s\n", js); free(js);
    printf("R checks %d failed %d\n", checks, fails);
    printf("DONE\n");
    return fails ? 1 : 0;
}
