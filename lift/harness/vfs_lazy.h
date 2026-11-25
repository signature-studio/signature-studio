/* Lazy install tree on the virtual volume (decision 0314, PROVISIONAL). Header-only; included by vfs_seed.h (it uses vs_fail, vs_wide, vs_utf8_units and the vs_entry rows of the seed list) and nothing else.
 *
 * A row of the seed list whose host field is "@lazy:<host index file>" (the guest field is the mount directory with a trailing backslash, the SHA-256 field is that of the index file) makes the harness read the
 * index (format FLVFSIX1 v1: docs/decisions/0314-lazy-install-tree.md, the section 'Format') and put EVERY file and directory of it on the volume as a lazy file: name, size, attributes and times now, the bytes
 * only when the guest reads them. The bytes live in the streams the index names; a stream is the host file "<directory of the index file>/<stream name>" (the page serves it from the chunks), and the one read
 * function the runtime is given (fle_vfs_lazy_read_fn) is open + pread on it. The index is read after the eager rows of the list were put, so a file that has bytes in the list wins (fle_vfs_put_lazy_file answers 7
 * for a path that exists, and the file is counted as skipped).
 *
 * Nothing here is reached by the guest. A malformed index (a bound is checked at every read) or a limit stops the harness by name before the guest runs. The limits: 16 streams, 65,536 entries, a chunk of 1..8 MiB,
 * a path of 259 UTF-16 units. The chunk hashes in the index are not read here (the page checks them when it fetches a chunk). */
#ifndef FLE_HARNESS_VFS_LAZY_H
#define FLE_HARNESS_VFS_LAZY_H

#include <fcntl.h>
#include <unistd.h>

#define VL_MAX_STREAMS 16u   /* streams one index may have */
#define VL_REG_MAX 1024u     /* streams the volume holds (fle_vfs.c LAZY_MAX_STREAMS): the index streams and one per file row of the seed list (#1345; 472 rows before the 109 of Sytrus's Data, #1414) */
#define VL_MAX_ENTRIES 65536u
#define VL_MAX_CHUNK (UINT64_C(8) << 20)
#define VL_MAX_INDEX_BYTES (UINT64_C(16) << 20)
#define VL_NAME_MAX 64u
#define VL_ATTR_ARCHIVE 0x20u
#define VL_ATTR_READONLY 0x01u
#define VL_ATTR_HIDDEN 0x02u
#define VL_ATTR_SYSTEM 0x04u

typedef struct {
    char *path;     /* the host file (malloc'd) */
    int fd;
    char sha[65];   /* #1345: the SHA-256 the seed list gives for a seed file; '' for an index stream (the index file is checked at start) */
    int checked;    /* #1345: set once the bytes of a seed file have been checked against sha (at its first read) */
} vl_stream;

static vl_stream vl_streams[VL_REG_MAX];
static unsigned vl_n_streams;

/* a bounded reader over the index bytes: any read past the end sets `bad` and returns 0 */
typedef struct {
    const uint8_t *p;
    size_t n, i;
    int bad;
} vl_cur;

static uint64_t vl_uint(vl_cur *c, unsigned bytes)
{
    uint64_t v = 0;
    unsigned k;
    if (c->bad || bytes > c->n - c->i) { c->bad = 1; return 0; }
    for (k = 0; k < bytes; k++) v |= (uint64_t)c->p[c->i + k] << (8 * k);
    c->i += bytes;
    return v;
}

static uint64_t vl_varint(vl_cur *c)
{
    uint64_t v = 0;
    unsigned shift = 0;
    for (;;) {
        uint8_t b;
        if (c->bad || c->i >= c->n || shift > 63) { c->bad = 1; return 0; }
        b = c->p[c->i++];
        v |= (uint64_t)(b & 0x7F) << shift;
        if (!(b & 0x80)) return v;
        shift += 7;
    }
}

static const uint8_t *vl_take(vl_cur *c, uint64_t n)
{
    const uint8_t *r;
    if (c->bad || n > c->n - c->i) { c->bad = 1; return NULL; }
    r = c->p + c->i;
    c->i += (size_t)n;
    return r;
}

/* #1345: the bytes of a seed file are checked against its SHA-256 at its first read (the whole file is read and hashed once), so the start-up does not hash the list. A mismatch is reported by name and the read fails. */
static int vl_check(vl_stream *s)
{
    struct stat st;
    uint8_t *buf;
    char got[65];
    size_t done = 0;
    if (fstat(s->fd, &st) != 0 || !S_ISREG(st.st_mode) || (uint64_t)st.st_size > VS_MAX_FILE_BYTES) {
        fprintf(stderr, "vfs seed: %s: cannot be read whole for its SHA-256 check\n", s->path);
        return -1;
    }
    buf = malloc(st.st_size ? (size_t)st.st_size : 1);
    if (!buf) {
        fprintf(stderr, "vfs seed: %s: out of memory for its SHA-256 check\n", s->path);
        return -1;
    }
    while (done < (size_t)st.st_size) {
        ssize_t k = pread(s->fd, buf + done, (size_t)st.st_size - done, (off_t)done);
        if (k <= 0) {
            free(buf);
            fprintf(stderr, "vfs seed: %s: cannot be read whole for its SHA-256 check\n", s->path);
            return -1;
        }
        done += (size_t)k;
    }
    vs_sha256_hex(buf, done, got);
    free(buf);
    if (strcmp(got, s->sha) != 0) {
        fprintf(stderr, "vfs seed: %s: SHA-256 mismatch at the first read: expected %s, got %s\n", s->path, s->sha, got);
        return -1;
    }
    return 0;
}

/* The read function of the runtime: the bytes [off, off + n) of the stream, by pread on the host file (opened on the first read). */
static int vl_read(void *ctx, unsigned stream, uint64_t off, void *dst, size_t n)
{
    uint8_t *d = dst;
    vl_stream *s;
    (void)ctx;
    if (stream >= vl_n_streams) return 0;
    s = &vl_streams[stream];
    if (s->fd < 0) {
        s->fd = open(s->path, O_RDONLY);
        if (s->fd < 0) return 0;
    }
    if (s->sha[0] && !s->checked) {
        if (vl_check(s) != 0) return 0;
        s->checked = 1;
    }
    while (n) {
        ssize_t k = pread(s->fd, d, n, (off_t)off);
        if (k <= 0) return 0;
        d += k;
        off += (uint64_t)k;
        n -= (size_t)k;
    }
    return 1;
}

static int vl_ieq_n(const char *a, const char *b, size_t n)
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

static int vl_name_ok(const uint8_t *s, size_t n)
{
    size_t i;
    if (n == 0 || n > VL_NAME_MAX) return 0;
    for (i = 0; i < n; i++) {
        uint8_t c = s[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-' || c == '_' || c == '.')) return 0;
    }
    return !(n == 1 && s[0] == '.') && !(n == 2 && s[0] == '.' && s[1] == '.');
}

/* Reads the index `ix` (n bytes; `index_host` is its host file name, `mount` the guest directory of the list row without the trailing backslash) and, unless `dry`, puts its streams, directories and files on the volume.
 * A dry run checks everything the index says (bounds, limits, names, sizes against the streams) and touches nothing: vl_load runs it first, so a malformed index leaves the volume as it was (a file the volume itself
 * refuses is the one failure left for the second run). Returns 0 or -1 (a message on stderr). *nfiles / *ndirs / *nskipped / *nbytes count the lazy files and directories put, the files that an eager file already held,
 * and the bytes of the lazy files. */
static int vl_parse(int dry, const char *list, unsigned line, const char *index_host, const char *mount, const uint8_t *ix, size_t n, uint64_t *nfiles, uint64_t *ndirs, uint64_t *nskipped, uint64_t *nbytes)
{
    vl_cur c = { ix, n, 0, 0 };
    uint32_t version, flags;
    uint64_t base_ft, ns, nd, nf, mount_len, total, e, seen_d = 0, seen_f = 0;
    uint64_t logical[VL_MAX_STREAMS], cursor[VL_MAX_STREAMS];
    const uint8_t *m;
    char dir[VS_HOST_MAX];
    static char prev[VS_GUEST_BYTES];
    size_t prev_len = 0, dl, mnt_len = strlen(mount), i;
    unsigned s, base;

    if (n < 8 || memcmp(ix, "FLVFSIX1", 8) != 0) return vs_fail(list, line, "the lazy index %s does not start with FLVFSIX1", index_host);
    c.i = 8;
    version = (uint32_t)vl_uint(&c, 4);
    flags = (uint32_t)vl_uint(&c, 4);
    (void)vl_uint(&c, 4);   /* small_max: informative */
    base_ft = vl_uint(&c, 8);
    ns = vl_varint(&c);
    nd = vl_varint(&c);
    nf = vl_varint(&c);
    mount_len = vl_varint(&c);
    m = vl_take(&c, mount_len);
    if (c.bad) return vs_fail(list, line, "the lazy index %s is cut short in its header", index_host);
    if (version != 1) return vs_fail(list, line, "the lazy index %s has version %u (this reader knows 1)", index_host, (unsigned)version);
    if (flags & ~UINT32_C(1)) return vs_fail(list, line, "the lazy index %s has unknown flags 0x%x", index_host, (unsigned)flags);
    if (ns == 0 || ns > VL_MAX_STREAMS) return vs_fail(list, line, "the lazy index %s has %llu streams (1..%u are supported)", index_host, (unsigned long long)ns, VL_MAX_STREAMS);
    if (nd > VL_MAX_ENTRIES || nf > VL_MAX_ENTRIES || nd + nf > VL_MAX_ENTRIES) return vs_fail(list, line, "the lazy index %s has %llu directories and %llu files (at most %u entries)", index_host, (unsigned long long)nd, (unsigned long long)nf, VL_MAX_ENTRIES);
    if (mount_len != mnt_len || !vl_ieq_n((const char *)m, mount, mnt_len)) return vs_fail(list, line, "the mount directory of the lazy index %s is not the one of the list row (%s)", index_host, mount);

    dl = strlen(index_host);
    while (dl > 0 && index_host[dl - 1] != '/') dl--;
    if (dl >= sizeof dir) return vs_fail(list, line, "the host file name of the lazy index is too long");
    memcpy(dir, index_host, dl);
    dir[dl] = 0;

    if (!dry && vl_n_streams + ns > VL_REG_MAX) return vs_fail(list, line, "the volume holds %u lazy streams and the lazy index %s adds %llu", vl_n_streams, index_host, (unsigned long long)ns);
    base = vl_n_streams;
    for (s = 0; s < ns; s++) {
        uint64_t name_len = vl_varint(&c), chunk, nchunks;
        const uint8_t *name = vl_take(&c, name_len);
        unsigned coding = (unsigned)vl_uint(&c, 1);
        int id;
        chunk = vl_varint(&c);
        nchunks = vl_varint(&c);
        logical[s] = vl_varint(&c);
        if (c.bad) return vs_fail(list, line, "the lazy index %s is cut short in stream %u", index_host, s);
        if (nchunks > (c.n - c.i) / 32u) c.bad = 1;
        else (void)vl_take(&c, nchunks * 32u);   /* the SHA-256 of each chunk: the page checks them, this reader does not need them */
        if (c.bad) return vs_fail(list, line, "the lazy index %s is cut short in the chunk hashes of stream %u", index_host, s);
        if (!vl_name_ok(name, (size_t)name_len)) return vs_fail(list, line, "stream %u of the lazy index %s has a name outside [A-Za-z0-9._-] or longer than %u", s, index_host, VL_NAME_MAX);
        if (coding > 1) return vs_fail(list, line, "stream %u of the lazy index %s has coding %u (0 identity, 1 brotli)", s, index_host, coding);
        if (chunk == 0 || chunk > VL_MAX_CHUNK) return vs_fail(list, line, "stream %u of the lazy index %s has chunk size %llu (1..%llu)", s, index_host, (unsigned long long)chunk, (unsigned long long)VL_MAX_CHUNK);
        if (nchunks != (logical[s] + chunk - 1) / chunk) return vs_fail(list, line, "stream %u of the lazy index %s has %llu chunks for %llu bytes of %llu per chunk", s, index_host, (unsigned long long)nchunks, (unsigned long long)logical[s], (unsigned long long)chunk);
        if (dl + name_len >= VS_HOST_MAX) return vs_fail(list, line, "the host file name of stream %u is too long", s);
        if (!dry) {
            vl_stream *vs = &vl_streams[base + s];
            vs->path = malloc(dl + name_len + 1);
            if (!vs->path) return vs_fail(list, line, "out of memory for stream %u of the lazy index", s);
            memcpy(vs->path, dir, dl);
            memcpy(vs->path + dl, name, (size_t)name_len);
            vs->path[dl + name_len] = 0;
            vs->fd = -1;
            vs->sha[0] = 0;
            vs->checked = 0;
            id = fle_vfs_lazy_add_stream(logical[s]);
            if (id != (int)(base + s)) return vs_fail(list, line, "the volume cannot take stream %u of the lazy index (%d)", s, id);
            vl_n_streams++;
        }
        cursor[s] = 0;
    }
    if (!dry) fle_vfs_lazy_set_reader(vl_read, NULL);

    {
        uint16_t mw[VS_GUEST_MAX + 1];
        int rc;
        vs_wide(mw, mount, mnt_len);
        rc = dry ? 0 : fle_vfs_put_dir(mw, 0, base_ft);
        if (rc != 0) return vs_fail(list, line, "the mount directory %s cannot be made on the volume (code %d)", mount, rc);
    }

    total = nd + nf;
    for (e = 0; e < total; e++) {
        static char full[VS_GUEST_BYTES + 8];
        uint16_t w[VS_GUEST_MAX + 1];
        unsigned kind = (unsigned)vl_uint(&c, 1);
        uint64_t common = vl_varint(&c), suf = vl_varint(&c), zz, size = 0, off = 0;
        const uint8_t *sp = NULL;
        unsigned attrs, stream = 0;
        uint64_t ft;
        size_t units, k, flen;
        int64_t delta;
        int rc;

        if (!c.bad && (common > prev_len || suf >= sizeof prev - common)) c.bad = 1;
        if (!c.bad) sp = vl_take(&c, suf);
        zz = vl_varint(&c);
        attrs = (unsigned)vl_uint(&c, 1);
        if (kind == 0) {
            stream = (unsigned)vl_uint(&c, 1);
            size = vl_varint(&c);
            if (flags & 1u) (void)vl_take(&c, 32);
        }
        if (c.bad) return vs_fail(list, line, "the lazy index %s is cut short or malformed at entry %llu", index_host, (unsigned long long)e);
        if (kind > 1) return vs_fail(list, line, "entry %llu of the lazy index %s has kind %u", (unsigned long long)e, index_host, kind);
        memcpy(prev + common, sp, (size_t)suf);
        prev_len = (size_t)(common + suf);
        prev[prev_len] = 0;
        if (prev_len == 0) return vs_fail(list, line, "entry %llu of the lazy index %s has an empty path", (unsigned long long)e, index_host);
        if (vs_utf8_units(prev, prev_len, &units) != 0) return vs_fail(list, line, "entry %llu of the lazy index %s is not valid UTF-8", (unsigned long long)e, index_host);
        if (mnt_len + 1 + prev_len >= sizeof full) return vs_fail(list, line, "entry %llu of the lazy index %s has a path that is too long", (unsigned long long)e, index_host);
        memcpy(full, mount, mnt_len);
        full[mnt_len] = '\\';
        for (k = 0; k < prev_len; k++) {
            char ch = prev[k];
            if (ch == '\\' || (unsigned char)ch < 0x20) return vs_fail(list, line, "entry %llu of the lazy index %s has a backslash or a control character in its path", (unsigned long long)e, index_host);
            full[mnt_len + 1 + k] = ch == '/' ? '\\' : ch;
        }
        flen = mnt_len + 1 + prev_len;
        full[flen] = 0;
        if (vs_utf8_units(full, flen, &units) != 0 || units > VS_GUEST_MAX) return vs_fail(list, line, "entry %llu of the lazy index %s is longer than 259 UTF-16 units (%s)", (unsigned long long)e, index_host, full);
        vs_wide(w, full, flen);
        delta = (int64_t)(zz >> 1) ^ -(int64_t)(zz & 1);
        if (delta > INT64_C(1) << 40 || delta < -(INT64_C(1) << 40)) return vs_fail(list, line, "entry %llu of the lazy index %s has a time %lld s from the base", (unsigned long long)e, index_host, (long long)delta);
        ft = base_ft + (uint64_t)(delta * INT64_C(10000000));
        if (kind == 1) {
            rc = dry ? 0 : fle_vfs_put_dir(w, attrs & (VL_ATTR_READONLY | VL_ATTR_HIDDEN | VL_ATTR_SYSTEM | VL_ATTR_ARCHIVE), ft);
            if (rc != 0) return vs_fail(list, line, "lazy directory %s: the volume answered %d", full, rc);
            (*ndirs)++;
            seen_d++;
            continue;
        }
        if (stream >= ns) return vs_fail(list, line, "entry %llu (%s) of the lazy index %s names stream %u of %llu", (unsigned long long)e, full, index_host, stream, (unsigned long long)ns);
        seen_f++;
        off = cursor[stream];
        if (size > logical[stream] - off) return vs_fail(list, line, "file %s of the lazy index %s runs past the end of its stream %u", full, index_host, stream);
        cursor[stream] = off + size;
        rc = dry ? 0 : fle_vfs_put_lazy_file(w, base + stream, off, size, attrs, ft);
        if (rc == 7) { (*nskipped)++; continue; }
        if (rc != 0) return vs_fail(list, line, "lazy file %s: the volume answered %d", full, rc);
        (*nfiles)++;
        *nbytes += size;
    }
    if (seen_d != nd || seen_f != nf) return vs_fail(list, line, "the lazy index %s says %llu directories and %llu files but holds %llu and %llu", index_host, (unsigned long long)nd, (unsigned long long)nf, (unsigned long long)seen_d, (unsigned long long)seen_f);
    if (c.i != c.n) return vs_fail(list, line, "the lazy index %s has %zu bytes after its last entry", index_host, c.n - c.i);
    for (i = 0; i < ns; i++)
        if (cursor[i] != logical[i]) return vs_fail(list, line, "the files of stream %zu of the lazy index %s add up to %llu bytes, the stream has %llu", i, index_host, (unsigned long long)cursor[i], (unsigned long long)logical[i]);
    return 0;
}

/* the check and then the load */
static int vl_load(const char *list, unsigned line, const char *index_host, const char *mount, const uint8_t *ix, size_t n, uint64_t *nfiles, uint64_t *ndirs, uint64_t *nskipped, uint64_t *nbytes)
{
    uint64_t f = 0, d = 0, k = 0, b = 0;
    if (vl_parse(1, list, line, index_host, mount, ix, n, &f, &d, &k, &b) != 0) return -1;
    return vl_parse(0, list, line, index_host, mount, ix, n, nfiles, ndirs, nskipped, nbytes);
}

#endif /* FLE_HARNESS_VFS_LAZY_H */
