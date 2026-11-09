/* clangcache: a transparent object cache in front of clang for the test suite (decision 0149).
 *
 * The tests run `Command::new("clang")` with `-O0 ... <runtime>/fle_*.c <generated>.c -o exe`, i.e. they recompile every runtime source
 * on every call. This program is installed as `clang` in a directory placed first in PATH by harness/test.sh. It
 *   - splits a compile+link command into one `clang -c` per .c input and keeps each object in a content-addressed cache
 *     (key: clang identity, the compile flags, the source bytes, and the bytes of every header the preprocessor read, taken from -MD);
 *   - links the cached objects with the real clang in the original argument order;
 *   - answers `clang -c x.c -o x.o` from the same cache.
 * Everything it does not fully understand (an unknown flag, -E, -x, -S, a compile error, an input that is not .c/.o/.a, a source over
 * 24 MiB) runs the real clang unchanged. A failed compile is never cached and is repeated by the unchanged original command, so the
 * diagnostics and exit status are the real ones.
 * FLE_TEST_NOCACHE=1 turns the cache off (the real clang runs at once).
 *
 * Env: FLE_CC_CACHE_DIR (cache root, required for caching), FLE_CC_REAL (real clang), FLE_CC_STATS (append one TSV line per decision).
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <limits.h>
#include <time.h>
#include <stdarg.h>
#include <libgen.h>
#include <sys/stat.h>
#include <sys/file.h>
#include <sys/wait.h>
#include <sys/types.h>

/* ---------------------------------------------------------------- SHA-256 */
typedef struct { uint32_t h[8]; uint64_t len; uint8_t buf[64]; size_t n; } Sha;
static const uint32_t K256[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74,
    0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d,
    0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e,
    0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};
#define ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
static void sha_block(Sha *s, const uint8_t *p) {
    uint32_t w[64], a, b, c, d, e, f, g, h;
    for (int i = 0; i < 16; i++) w[i] = (uint32_t)p[4 * i] << 24 | p[4 * i + 1] << 16 | p[4 * i + 2] << 8 | p[4 * i + 3];
    for (int i = 16; i < 64; i++) {
        uint32_t s0 = ROR(w[i - 15], 7) ^ ROR(w[i - 15], 18) ^ (w[i - 15] >> 3), s1 = ROR(w[i - 2], 17) ^ ROR(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
    for (int i = 0; i < 64; i++) {
        uint32_t S1 = ROR(e, 6) ^ ROR(e, 11) ^ ROR(e, 25), ch = (e & f) ^ (~e & g), t1 = h + S1 + ch + K256[i] + w[i];
        uint32_t S0 = ROR(a, 2) ^ ROR(a, 13) ^ ROR(a, 22), mj = (a & b) ^ (a & c) ^ (b & c), t2 = S0 + mj;
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}
static void sha_init(Sha *s) {
    static const uint32_t i0[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    memcpy(s->h, i0, sizeof i0); s->len = 0; s->n = 0;
}
static void sha_upd(Sha *s, const void *d, size_t n) {
    const uint8_t *p = d;
    s->len += n;
    if (s->n) {
        size_t k = 64 - s->n; if (k > n) k = n;
        memcpy(s->buf + s->n, p, k); s->n += k; p += k; n -= k;
        if (s->n == 64) { sha_block(s, s->buf); s->n = 0; }
    }
    while (n >= 64) { sha_block(s, p); p += 64; n -= 64; }
    if (n) { memcpy(s->buf, p, n); s->n = n; }
}
static void sha_hex(Sha *s, char out[65]) {
    uint64_t bits = s->len * 8;
    uint8_t pad = 0x80;
    sha_upd(s, &pad, 1);
    pad = 0;
    while (s->n != 56) sha_upd(s, &pad, 1);
    uint8_t lb[8];
    for (int i = 0; i < 8; i++) lb[i] = bits >> (56 - 8 * i);
    sha_upd(s, lb, 8);
    for (int i = 0; i < 8; i++) sprintf(out + 8 * i, "%08x", s->h[i]);
}
static void sha_str(Sha *s, const char *x) { sha_upd(s, x, strlen(x)); sha_upd(s, "\0", 1); }

/* ---------------------------------------------------------------- helpers */
static char *xstrdup(const char *s) { char *r = strdup(s); if (!r) abort(); return r; }
static char *xfmt(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
static char *xfmt(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    char *r = NULL;
    if (vasprintf(&r, fmt, ap) < 0) abort();
    va_end(ap);
    return r;
}
static double now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec + t.tv_nsec / 1e9; }

static char *g_real, *g_cache, *g_stats;
static char g_clangid[65];

static void stat_line(const char *kind, const char *what, double secs) {
    if (!g_stats) return;
    char *l = xfmt("%s\t%.3f\t%s\n", kind, secs, what);
    int fd = open(g_stats, O_WRONLY | O_APPEND | O_CREAT, 0644);
    if (fd >= 0) { if (write(fd, l, strlen(l)) < 0) {} close(fd); }
}

static int read_file(const char *p, char **buf, size_t *len) {
    int fd = open(p, O_RDONLY);
    if (fd < 0) return -1;
    struct stat st;
    if (fstat(fd, &st) < 0 || !S_ISREG(st.st_mode)) { close(fd); return -1; }
    size_t cap = st.st_size + 1, n = 0;
    char *b = malloc(cap);
    if (!b) { close(fd); return -1; }
    for (;;) {
        if (n + 1 >= cap) { cap *= 2; b = realloc(b, cap); if (!b) { close(fd); return -1; } }
        ssize_t r = read(fd, b + n, cap - n - 1);
        if (r < 0) { if (errno == EINTR) continue; free(b); close(fd); return -1; }
        if (r == 0) break;
        n += r;
    }
    close(fd);
    b[n] = 0;
    *buf = b; *len = n;
    return 0;
}

typedef struct { char *path; char hex[65]; int sens; } Memo;
static Memo *g_memo; static int g_nmemo;

/* hash of a file, and whether its text mentions something that makes the object depend on the path/time */
static int file_hash(const char *path, char hex[65], int *sens) {
    for (int i = 0; i < g_nmemo; i++)
        if (!strcmp(g_memo[i].path, path)) { memcpy(hex, g_memo[i].hex, 65); if (sens) *sens = g_memo[i].sens; return 0; }
    char *b; size_t n;
    if (read_file(path, &b, &n) < 0) return -1;
    Sha s; sha_init(&s); sha_upd(&s, b, n); sha_hex(&s, hex);
    int se = 0;
    static const char *pat[] = {"__FILE__", "__BASE_FILE__", "__builtin_FILE", "__FILE_NAME__", "__DATE__", "__TIME__", "__TIMESTAMP__", "__COUNTER__", NULL};
    for (int i = 0; pat[i] && !se; i++) if (memmem(b, n, pat[i], strlen(pat[i]))) se = 1;
    free(b);
    g_memo = realloc(g_memo, (g_nmemo + 1) * sizeof(Memo));
    g_memo[g_nmemo].path = xstrdup(path); memcpy(g_memo[g_nmemo].hex, hex, 65); g_memo[g_nmemo].sens = se; g_nmemo++;
    if (sens) *sens = se;
    return 0;
}

static void mkdir_p(const char *d) {
    char *t = xstrdup(d);
    for (char *p = t + 1; *p; p++)
        if (*p == '/') { *p = 0; mkdir(t, 0777); *p = '/'; }
    mkdir(t, 0777);
    free(t);
}
static char *shard(const char *kind, const char *hex) { return xfmt("%s/%s/%.2s/%s", g_cache, kind, hex, hex); }
static int exists(const char *p) { return access(p, F_OK) == 0; }

static int copy_file(const char *from, const char *to, mode_t mode) {
    int in = open(from, O_RDONLY);
    if (in < 0) return -1;
    unlink(to);
    int out = open(to, O_WRONLY | O_CREAT | O_TRUNC, mode);
    if (out < 0) { close(in); return -1; }
    char buf[1 << 16];
    ssize_t r;
    int rc = 0;
    while ((r = read(in, buf, sizeof buf)) > 0) {
        char *p = buf;
        while (r > 0) { ssize_t w = write(out, p, r); if (w < 0) { if (errno == EINTR) continue; rc = -1; break; } p += w; r -= w; }
        if (rc) break;
    }
    if (r < 0) rc = -1;
    close(in); close(out);
    return rc;
}

/* store bytes of a file under its own hash in the content store; returns the hash in hex */
static int store_blob(const char *from, char hex[65]) {
    char *b; size_t n;
    if (read_file(from, &b, &n) < 0) return -1;
    Sha s; sha_init(&s); sha_upd(&s, b, n); sha_hex(&s, hex);
    free(b);
    char *dst = shard("b", hex);
    if (!exists(dst)) {
        char *d2 = xstrdup(dst); mkdir_p(dirname(d2));
        char *tmp = xfmt("%s.tmp%d", dst, getpid());
        if (copy_file(from, tmp, 0644) < 0 || rename(tmp, dst) < 0) { unlink(tmp); return -1; }
    }
    return 0;
}

static int run(char **argv, const char *errfile, int quiet_out) {
    pid_t pid = fork();
    if (pid == 0) {
        if (errfile) { int fd = open(errfile, O_WRONLY | O_CREAT | O_TRUNC, 0644); if (fd >= 0) { dup2(fd, 2); close(fd); } }
        if (quiet_out) { int fd = open("/dev/null", O_WRONLY); if (fd >= 0) { dup2(fd, 1); close(fd); } }
        execv(argv[0], argv);
        _exit(127);
    }
    int st;
    while (waitpid(pid, &st, 0) < 0 && errno == EINTR) {}
    return WIFEXITED(st) ? WEXITSTATUS(st) : 128 + WTERMSIG(st);
}

static void passthrough(char **argv, int argc, const char *why) {
    stat_line("pass", why, 0);
    char **v = calloc(argc + 1, sizeof(char *));
    v[0] = g_real;
    for (int i = 1; i < argc; i++) v[i] = argv[i];
    execv(g_real, v);
    perror("clangcache: exec real clang");
    _exit(127);
}

/* ---------------------------------------------------------------- the cache */
typedef struct { char *flag; int takes_arg; } Opt;

/* Does this compile flag need to be in the key and passed to `clang -c`? Returns 0 = not understood, 1 = compile+link, 2 = link only (not passed to -c), 3 = compile only. */
static int classify(const char *a, int *consumes) {
    *consumes = 0;
    if (!strcmp(a, "-I") || !strcmp(a, "-isystem") || !strcmp(a, "-include")) { *consumes = 1; return 3; }
    if (!strncmp(a, "-I", 2) || !strncmp(a, "-D", 2) || !strncmp(a, "-U", 2) || !strncmp(a, "-isystem", 8)) return 3;
    if (!strncmp(a, "-O", 2) && (a[2] == 0 || (a[3] == 0 && strchr("0123sgz", a[2])))) return 3;
    if (!strncmp(a, "-std=", 5) || !strcmp(a, "-w") || !strcmp(a, "-g") || !strcmp(a, "-Werror") || !strcmp(a, "-Wall") || !strcmp(a, "-Wextra") || !strncmp(a, "-Wno-", 5)) return 3;
    if (!strncmp(a, "-Wl,", 4) || !strcmp(a, "-lm") || !strcmp(a, "-ldl") || !strcmp(a, "-lrt") || !strcmp(a, "-lpthread") || 0) return 2;
    if (!strcmp(a, "-pthread") || !strcmp(a, "-no-pie") || !strcmp(a, "-pie") || !strcmp(a, "-static") || !strcmp(a, "-m64") || !strcmp(a, "-fPIE") || !strcmp(a, "-fPIC") || !strcmp(a, "-fno-pie") || !strcmp(a, "-fno-pic")) return 1;
    static const char *f[] = {"-fno-builtin", "-ffp-contract=off", "-ffp-contract=on", "-ffreestanding", "-fno-stack-protector", "-fno-omit-frame-pointer", "-fno-ident",
                              "-fasynchronous-unwind-tables", "-finstrument-functions", "-fno-strict-aliasing", "-fwrapv", "-fno-common", "-fvisibility=hidden", "-mno-red-zone",
                              "-msse", "-msse2", "-mno-avx", "-fno-math-errno", "-fno-unwind-tables", "-fexceptions", "-fno-asynchronous-unwind-tables", NULL};
    for (int i = 0; f[i]; i++) if (!strcmp(a, f[i])) return 1;
    return 0;
}

/* canonical spelling of a path argument for the key */
static char *canon(const char *p) {
    char rp[PATH_MAX];
    if (realpath(p, rp)) return xstrdup(rp);
    return xstrdup(p);
}

typedef struct { char *src; char **cflags; int ncf; char *obj; } Unit;

/* parse the dep file written by -MD -MF: "target: a b \<nl> c" */
static int parse_deps(const char *dfile, char ***out, int *nout) {
    char *b; size_t n;
    if (read_file(dfile, &b, &n) < 0) return -1;
    char **v = NULL; int nv = 0;
    char *p = strchr(b, ':');
    if (!p) { free(b); return -1; }
    p++;
    while (*p) {
        while (*p == ' ' || *p == '\n' || *p == '\t' || (*p == '\\' && (p[1] == '\n' || p[1] == '\r'))) p += (*p == '\\') ? 2 : 1;
        if (!*p) break;
        char tok[PATH_MAX]; int t = 0;
        while (*p && *p != ' ' && *p != '\n' && *p != '\t') {
            if (*p == '\\' && p[1] == ' ') { tok[t++] = ' '; p += 2; continue; }
            if (*p == '\\' && p[1] == '\\') { tok[t++] = '\\'; p += 2; continue; }
            if (*p == '$' && p[1] == '$') { tok[t++] = '$'; p += 2; continue; }
            if (t < PATH_MAX - 1) tok[t++] = *p;
            p++;
        }
        tok[t] = 0;
        v = realloc(v, (nv + 1) * sizeof(char *));
        v[nv++] = xstrdup(tok);
    }
    free(b);
    *out = v; *nout = nv;
    return 0;
}

static int sys_path(const char *p) { return !strncmp(p, "/usr/", 5) || !strncmp(p, "/opt/", 5) || !strncmp(p, "/lib/", 5); }

/* Manifest of one key k1: text, one entry after the other:
 *   E <sens 0|1> <object blob> <stderr blob or -> <compile seconds> <source as given> <cwd>
 *   H <sha256 of a file the preprocessor read> <mtime_ns> <size> <path>      (zero or more)
 * An entry is valid when every H file still has its hash (system files: the same mtime and size are enough) and, for sens=1 (the object
 * depends on the path of the source, e.g. __FILE__, assert, -g, or warnings were printed), the source spelling and cwd are the same. */
static int manifest_lookup(const char *k1, const char *src, const char *cwd, char *objhex, char *errhex, double *cost) {
    char *mp = shard("m", k1);
    char *b; size_t n;
    if (read_file(mp, &b, &n) < 0) return 0;
    int found = 0, ok = 0;
    char sens = '0', o[80] = "", e[80] = "", sp[PATH_MAX] = "", cw[PATH_MAX] = ""; double cs = 0;
    int have = 0;
    char *save = NULL;
    for (char *line = strtok_r(b, "\n", &save); line; line = strtok_r(NULL, "\n", &save)) {
        if (line[0] == 'E') {
            if (have && ok) { found = 1; break; }
            have = 0; ok = 0;
            char sc[8];
            if (sscanf(line, "E %7s %79s %79s %lf %4095s %4095s", sc, o, e, &cs, sp, cw) == 6) {
                sens = sc[0]; have = 1;
                ok = !(sens == '1' && (strcmp(sp, src) || strcmp(cw, cwd)));
            }
        } else if (line[0] == 'H' && have && ok) {
            char hx[80]; long long mt, sz; int off = 0;
            if (sscanf(line, "H %79s %lld %lld %n", hx, &mt, &sz, &off) < 3 || !off) { ok = 0; continue; }
            const char *pth = line + off;
            struct stat st;
            if (sys_path(pth) && stat(pth, &st) == 0 && (long long)st.st_size == sz && (long long)st.st_mtim.tv_sec * 1000000000LL + st.st_mtim.tv_nsec == mt) continue;
            char cur[65];
            if (file_hash(pth, cur, NULL) < 0 || strcmp(cur, hx)) ok = 0;
        }
    }
    if (have && ok) found = 1;
    if (found) {
        snprintf(objhex, 65, "%.64s", o); snprintf(errhex, 65, "%.64s", e); *cost = cs;
        char *op = shard("b", objhex);
        if (!exists(op)) found = 0;
        if (found && errhex[0] != '-' && !exists(shard("b", errhex))) found = 0;
    }
    free(b);
    return found;
}

/* Sources that a test writes itself (anything under a directory named out-*) get a new name in every run (name-pid-n.c): an object stored for them would never be
 * used again. They are compiled for the link and the object is removed afterwards. Returns 0 and the temporary object path in *objpath, or 1 when clang fails
 * (the caller then runs the original command, which prints the real diagnostics). */
static int compile_direct(const char *src, char **cflags, int ncf, char **objpath) {
    char *tobj = xfmt("%s/t/%d-%ld-d.o", g_cache, getpid(), (long)(now() * 1000));
    char *terr = xfmt("%s.err", tobj);
    { char *tdir = xfmt("%s/t", g_cache); mkdir_p(tdir); }
    char **v = calloc(ncf + 8, sizeof(char *));
    int nv = 0;
    v[nv++] = g_real; v[nv++] = "-Qunused-arguments";
    for (int i = 0; i < ncf; i++) v[nv++] = cflags[i];
    v[nv++] = "-c"; v[nv++] = (char *)src; v[nv++] = "-o"; v[nv++] = tobj;
    double c0 = now();
    int rc = run(v, terr, 0);
    if (rc != 0) { unlink(tobj); unlink(terr); return 1; }
    char *eb; size_t en;
    if (read_file(terr, &eb, &en) == 0 && en) { if (write(2, eb, en) < 0) {} }
    unlink(terr);
    stat_line("direct", src, now() - c0);
    *objpath = tobj;
    return 0;
}

static int is_generated(const char *src) {
    /* FLE_CC_ROOT is lift/ of the tree under test (runner.sh sets it): what a test writes lives in lift/out-*; without the variable nothing counts as generated */
    const char *root = getenv("FLE_CC_ROOT");
    if (!root || !*root) return 0;
    char rp[PATH_MAX], rr[PATH_MAX];
    const char *p = realpath(src, rp) ? rp : src;
    if (!realpath(root, rr)) return 0;
    size_t n = strlen(rr);
    if (strncmp(p, rr, n) != 0 || p[n] != '/') return 1;   /* outside the tree: written by somebody else, do not keep it */
    return strncmp(p + n + 1, "out-", 4) == 0;
}

/* Compile one source through the cache. On success returns 0 and *objpath names a file (in the cache) holding the object. Other: 1 = failed (caller reruns the original command), 2 = not cacheable (same). */
static int compile_cached(const char *src, char **cflags, int ncf, char **objpath, int *hit) {
    double t0 = now();
    char srchex[65]; int ssens = 0;
    if (file_hash(src, srchex, &ssens) < 0) return 2;
    struct stat sst;
    if (stat(src, &sst) < 0 || sst.st_size > (24 << 20)) return 2;
    Sha s; sha_init(&s);
    sha_str(&s, "clangcache-v1"); sha_str(&s, g_clangid); sha_str(&s, srchex);
    { const char *bn = strrchr(src, '/'); sha_str(&s, bn ? bn + 1 : src); } /* the ELF FILE symbol holds the base name */
    int sens = ssens;
    for (int i = 0; i < ncf; i++) {
        const char *f = cflags[i];
        if (!strcmp(f, "-g") || !strcmp(f, "-finstrument-functions")) sens = 1;
        if ((!strcmp(f, "-I") || !strcmp(f, "-isystem") || !strcmp(f, "-include")) && i + 1 < ncf) { sha_str(&s, f); char *c = canon(cflags[++i]); sha_str(&s, c); continue; }
        if (!strncmp(f, "-I", 2) && f[2]) { char *c = canon(f + 2); sha_str(&s, "-I"); sha_str(&s, c); continue; }
        sha_str(&s, f);
    }
    char k1[65]; sha_hex(&s, k1);
    char cwd[PATH_MAX]; if (!getcwd(cwd, sizeof cwd)) return 2;
    if (strchr(src, ' ') || strchr(cwd, ' ')) return 2;
    char objhex[65], errhex[65]; double cost = 0;
    if (manifest_lookup(k1, src, cwd, objhex, errhex, &cost)) {
        *objpath = shard("b", objhex); *hit = 1;
        utimensat(AT_FDCWD, *objpath, NULL, 0);
        if (errhex[0] != '-') { char *ep = shard("b", errhex); char *eb; size_t en; if (read_file(ep, &eb, &en) == 0) { if (write(2, eb, en) < 0) {} } }
        stat_line("hit", src, cost);
        return 0;
    }
    /* miss: serialize the compilers of one key */
    char *lp = xfmt("%s/l/%.2s/%s", g_cache, k1, k1);
    { char *d2 = xstrdup(lp); mkdir_p(dirname(d2)); }
    int lfd = open(lp, O_RDWR | O_CREAT, 0644);
    if (lfd >= 0) flock(lfd, LOCK_EX);
    if (manifest_lookup(k1, src, cwd, objhex, errhex, &cost)) {
        *objpath = shard("b", objhex); *hit = 1;
        if (errhex[0] != '-') { char *ep = shard("b", errhex); char *eb; size_t en; if (read_file(ep, &eb, &en) == 0) { if (write(2, eb, en) < 0) {} } }
        stat_line("hit", src, cost);
        if (lfd >= 0) close(lfd);
        return 0;
    }
    char *tdir = xfmt("%s/t", g_cache); mkdir_p(tdir);
    char *tobj = xfmt("%s/t/%d-%ld.o", g_cache, getpid(), (long)(t0 * 1000));
    char *tdep = xfmt("%s/t/%d-%ld.d", g_cache, getpid(), (long)(t0 * 1000));
    char *terr = xfmt("%s/t/%d-%ld.err", g_cache, getpid(), (long)(t0 * 1000));
    char **v = calloc(ncf + 10, sizeof(char *));
    int nv = 0;
    v[nv++] = g_real; v[nv++] = "-Qunused-arguments";
    for (int i = 0; i < ncf; i++) v[nv++] = cflags[i];
    v[nv++] = "-c"; v[nv++] = (char *)src; v[nv++] = "-o"; v[nv++] = tobj; v[nv++] = "-MD"; v[nv++] = "-MF"; v[nv++] = tdep;
    double c0 = now();
    int rc = run(v, terr, 0);
    double cdur = now() - c0;
    int ret = 1;
    char **deps = NULL; int ndeps = 0;
    if (rc == 0 && parse_deps(tdep, &deps, &ndeps) == 0) {
        char *eb = NULL; size_t en = 0;
        read_file(terr, &eb, &en);
        if (en) { if (write(2, eb, en) < 0) {} sens = 1; }
        char errh[65] = "-";
        if (en && store_blob(terr, errh) < 0) { ret = 2; goto done; }
        char oh[65];
        if (store_blob(tobj, oh) < 0) { ret = 2; goto done; }
        char *hl = xfmt("%s", "");
        int ok = 1;
        char *mp = shard("m", k1);
        for (int i = 0; i < ndeps && ok; i++) {
            char hx[65]; int dsens = 0;
            if (strchr(deps[i], ' ') || strchr(deps[i], '\n')) { ok = 0; break; }
            if (file_hash(deps[i], hx, &dsens) < 0) { ok = 0; break; }
            if (dsens) sens = 1;
            struct stat st; long long mt = 0, sz = 0;
            if (stat(deps[i], &st) == 0) { mt = (long long)st.st_mtim.tv_sec * 1000000000LL + st.st_mtim.tv_nsec; sz = st.st_size; }
            hl = xfmt("%sH %s %lld %lld %s\n", hl, hx, mt, sz, deps[i]);
        }
        if (ok) {
            char *ent = xfmt("E %d %s %s %.3f %s %s\n%s", sens, oh, errh, cdur, src, cwd, hl);
            char *old = NULL; size_t oldn = 0;
            if (read_file(mp, &old, &oldn) < 0) { old = xstrdup(""); }
            char *d2 = xstrdup(mp); mkdir_p(dirname(d2));
            char *tm = xfmt("%s.tmp%d", mp, getpid());
            FILE *f = fopen(tm, "w");
            if (f) { fputs(old, f); fputs(ent, f); fclose(f); if (rename(tm, mp) < 0) unlink(tm); }
            *objpath = shard("b", oh); *hit = 0; ret = 0;
            stat_line("miss", src, cdur);
        } else ret = 2;
    }
done:
    unlink(tobj); unlink(tdep); unlink(terr);
    if (lfd >= 0) close(lfd);
    return ret;
}

int main(int argc, char **argv) {
    char self[PATH_MAX];
    ssize_t sl = readlink("/proc/self/exe", self, sizeof self - 1);
    if (sl < 0) return 127;
    self[sl] = 0;
    char selfdir[PATH_MAX];
    strcpy(selfdir, self);
    dirname(selfdir);
    g_real = getenv("FLE_CC_REAL");
    if (!g_real || !*g_real) {
        const char *path = getenv("PATH");
        char *pc = xstrdup(path ? path : "/usr/bin:/bin");
        char real[PATH_MAX] = "";
        for (char *sv = NULL, *d = strtok_r(pc, ":", &sv); d; d = strtok_r(NULL, ":", &sv)) {
            char rd[PATH_MAX];
            if (!realpath(d, rd) || !strcmp(rd, selfdir)) continue;
            snprintf(real, sizeof real, "%s/clang", d);
            if (access(real, X_OK) == 0) break;
            real[0] = 0;
        }
        if (!real[0]) { fprintf(stderr, "clangcache: no real clang in PATH\n"); return 127; }
        g_real = xstrdup(real);
    }
    g_cache = getenv("FLE_CC_CACHE_DIR");
    g_stats = getenv("FLE_CC_STATS");
    const char *noc = getenv("FLE_TEST_NOCACHE");
    if (!g_cache || !*g_cache || (noc && *noc && strcmp(noc, "0"))) passthrough(argv, argc, "nocache");
    /* identity of the compiler */
    {
        char rp[PATH_MAX]; struct stat st;
        if (!realpath(g_real, rp) || stat(rp, &st) < 0) passthrough(argv, argc, "noid");
        Sha s; sha_init(&s); sha_str(&s, rp);
        char t[128]; snprintf(t, sizeof t, "%lld:%lld", (long long)st.st_size, (long long)st.st_mtim.tv_sec * 1000000000LL + st.st_mtim.tv_nsec); sha_str(&s, t);
        /* the driver is a thin wrapper around cc1 in /usr/lib/llvm-N: also its resource directory is a dependency through -MD */
        sha_hex(&s, g_clangid);
    }
    /* classify the command line */
    char **cflags = calloc(argc + 1, sizeof(char *)); int ncf = 0;   /* flags for `clang -c` */
    char **plan = calloc(argc + 1, sizeof(char *)); int nplan = 0;   /* link command, in order; .c replaced later */
    int *isc = calloc(argc + 1, sizeof(int));
    char *out = NULL; int dashc = 0, nc = 0;
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        int cons;
        if (!strcmp(a, "-c")) { dashc = 1; continue; }
        if (!strcmp(a, "-o")) { if (i + 1 >= argc) passthrough(argv, argc, "bad-o"); out = argv[++i]; continue; }
        if (a[0] == '-') {
            int k = classify(a, &cons);
            if (!k) passthrough(argv, argc, a);
            if (cons && i + 1 >= argc) passthrough(argv, argc, "bad-arg");
            if (k == 1 || k == 3) { cflags[ncf++] = argv[i]; if (cons) cflags[ncf++] = argv[i + 1]; }
            if (k == 1 || k == 2 || k == 3) { plan[nplan++] = argv[i]; if (cons) plan[nplan++] = argv[i + 1]; }
            if (cons) i++;
            continue;
        }
        size_t n = strlen(a);
        if (n > 2 && !strcmp(a + n - 2, ".c")) { isc[nplan] = 1; plan[nplan++] = argv[i]; nc++; continue; }
        if (n > 2 && !strcmp(a + n - 2, ".o") && !dashc) { plan[nplan++] = argv[i]; continue; }
        passthrough(argv, argc, "input");
    }
    if (!nc || !out) passthrough(argv, argc, nc ? "no-o" : "no-c-input");
    if (dashc && nc != 1) passthrough(argv, argc, "multi-c");
    mkdir_p(g_cache);
    if (dashc)
        for (int i = 0; i < nplan; i++)
            if (isc[i] && is_generated(plan[i])) passthrough(argv, argc, "generated-source");
    /* compile */
    char **objs = calloc(nplan + 1, sizeof(char *));
    char **tmpobjs = calloc(nplan + 1, sizeof(char *)); int ntmp = 0;
    for (int i = 0; i < nplan; i++) {
        if (!isc[i]) continue;
        int hit = 0;
        int r;
        if (!dashc && is_generated(plan[i])) {
            r = compile_direct(plan[i], cflags, ncf, &objs[i]);
            if (r == 0) tmpobjs[ntmp++] = objs[i];
        } else {
            r = compile_cached(plan[i], cflags, ncf, &objs[i], &hit);
        }
        if (r) { for (int k = 0; k < ntmp; k++) unlink(tmpobjs[k]); passthrough(argv, argc, r == 1 ? "compile-failed" : "uncacheable"); }
    }
    if (dashc) {
        for (int i = 0; i < nplan; i++) if (isc[i]) { if (copy_file(objs[i], out, 0644) < 0) passthrough(argv, argc, "copy-failed"); }
        return 0;
    }
    char **v = calloc(nplan + 8, sizeof(char *)); int nv = 0;
    v[nv++] = g_real;
    v[nv++] = "-Qunused-arguments";
    for (int i = 0; i < nplan; i++) v[nv++] = isc[i] ? objs[i] : plan[i];
    v[nv++] = "-o"; v[nv++] = out;
    double l0 = now();
    int rc = run(v, NULL, 0);
    stat_line("link", out, now() - l0);
    for (int k = 0; k < ntmp; k++) unlink(tmpobjs[k]);
    return rc;
}
