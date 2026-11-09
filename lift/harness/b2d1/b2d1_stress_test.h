/* Decision 0160: the random drawing script of swap1 (lift/out-swap1/oracle/gen.h + stress_oracle.c: rectangles, ellipses, polygon fills, line strokes, open polyline
 * strokes, polygon stroke+fill; 1280x720; seed 777) driven through the Agg2D slots of the lifted wrapper. Included by b2d_scan.c as the test `stress`.
 * FLE_B2D1_STRESS="N mask W H" (default "20000 0x3f 1280 720"). Prints the FNV-1a of the buffer after every 1000 ops and at the end (swap1: 20000 ops, mask 0x3f
 * give e4b385310d0fca08 for the real DLL, upstream JIT, DISABLE_JIT and no-JIT builds). Uses its own buffer and attaches again (so it also destroys the first context). */
#include <time.h>
static double st_cpu(void) { struct timespec t; clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &t); return (double)t.tv_sec + 1e-9 * (double)t.tv_nsec; }
static uint32_t st_S = 777;
static uint32_t st_rnd(void) { st_S = st_S * 1664525u + 1013904223u; return st_S >> 8; }
static double st_rf(double a, double b) { return a + (b - a) * ((st_rnd() & 0xFFFF) / 65535.0); }
typedef struct { int kind; unsigned r, g, b, a, r2, g2, b2, a2; double lw; double v[12]; int n; } st_op;
static st_op st_next(int Wd, int Hd, int mask) {
    st_op o; int kinds[8], nk = 0, k, i;
    for (k = 0; k < 8; k++) if (mask & (1 << k)) kinds[nk++] = k;
    o.kind = kinds[st_rnd() % nk]; o.r = st_rnd() & 255; o.g = st_rnd() & 255; o.b = st_rnd() & 255; o.a = (st_rnd() % 4 == 0) ? 255 : (st_rnd() & 255);
    o.r2 = st_rnd() & 255; o.g2 = st_rnd() & 255; o.b2 = st_rnd() & 255; o.a2 = (st_rnd() & 255) | 64; o.lw = st_rf(0.5, 6); o.n = 0;
    switch (o.kind) {
    case 0: o.v[0] = st_rf(0, Wd - 150); o.v[1] = st_rf(0, Hd - 150); o.v[2] = o.v[0] + st_rf(2, 150); o.v[3] = o.v[1] + st_rf(2, 150); break;
    case 1: o.v[0] = st_rf(60, Wd - 60); o.v[1] = st_rf(60, Hd - 60); o.v[2] = st_rf(2, 60); o.v[3] = st_rf(2, 60); break;
    case 2: case 4: case 5: {
        double x, y;
        o.n = 3 + st_rnd() % 4; x = st_rf(60, Wd - 60); y = st_rf(60, Hd - 60);
        for (i = 0; i < o.n; i++) { o.v[2 * i] = x + st_rf(-50, 50); o.v[2 * i + 1] = y + st_rf(-50, 50); }
        break; }
    case 3: o.v[0] = st_rf(0, Wd); o.v[1] = st_rf(0, Hd); o.v[2] = st_rf(0, Wd); o.v[3] = st_rf(0, Hd); break;
    default: break;
    }
    return o;
}
TEST(stress) {
    int N = 20000, mask = 0x3f, Wd = 1280, Hd = 720, i, k;
    double t0;
    const char *e = getenv("FLE_B2D1_STRESS");
    uint8_t *sb;
    if (e) sscanf(e, "%d %i %d %d", &N, &mask, &Wd, &Hd);
    sb = aligned_alloc(64, ((size_t)Wd * Hd * 4 + 63) & ~(size_t)63);
    if (!sb) return;
    memset(sb, 0, (size_t)Wd * Hd * 4);
    {
        uint64_t s[1] = {(uint64_t)Wd * 4};
        cp(1, P(sb), (uint64_t)Wd, (uint64_t)Hd, 1, s);
    }
    colour(8, 0x20, 0x28, 0x30, 0xff);
    t0 = st_cpu();
    for (i = 0; i < N; i++) {
        st_op p = st_next(Wd, Hd, mask);
        colour(27, p.r, p.g, p.b, p.a); colour(30, p.r2, p.g2, p.b2, p.a2); c1(42, D(p.lw));
        switch (p.kind) {
        case 0: c0(31); c4(63, D(p.v[0]), D(p.v[1]), D(p.v[2]), D(p.v[3])); break;
        case 1: c0(31); c4(67, D(p.v[0]), D(p.v[1]), D(p.v[2]), D(p.v[3])); break;
        case 2: case 4: case 5:
            if (p.kind == 2) c0(31);
            if (p.kind == 4) c0(28);
            c0(82); c2(83, D(p.v[0]), D(p.v[1]));
            for (k = 1; k < p.n; k++) c2(85, D(p.v[2 * k]), D(p.v[2 * k + 1]));
            if (p.kind != 4) c0(102);
            c1(103, p.kind == 2 ? 0 : p.kind == 4 ? 1 : 2);
            break;
        case 3: c0(28); c4(61, D(p.v[0]), D(p.v[1]), D(p.v[2]), D(p.v[3])); break;
        }
        if (i % 1000 == 999) printf("scan: VALUE %s op%d %#llx\n", cfg_full, i + 1, ULL(fnv(sb, (size_t)Wd * Hd * 4)));
    }
    printf("scan: TIMING %s stress process-cpu-seconds %.3f for %d ops (the process also ran the module start-up before the test)\n", cfg_full, st_cpu() - t0, N);
    printf("scan: VALUE %s final N=%d mask=%#x %#llx\n", cfg_full, N, mask, ULL(fnv(sb, (size_t)Wd * Hd * 4)));
}

/* attach/draw/destroy cost: N times attach (a new context on a 64x64 buffer; the previous one is destroyed by the wrapper), one clear, one filled rectangle and a stroked path. TIMING lines are not compared. */
TEST(attachloop) {
    int N = 2000, i;
    double t0;
    const char *e = getenv("FLE_B2D1_ATTACHLOOP");
    uint8_t *sb;
    if (e) N = atoi(e);
    sb = aligned_alloc(64, NBYTES);
    if (!sb) return;
    memset(sb, 0, NBYTES);
    t0 = st_cpu();
    for (i = 0; i < N; i++) {
        uint64_t s[1] = {STRIDE};
        cp(1, P(sb), W, H, 1, s);
        colour(8, 0x20, 0x28, 0x30, 0xff);
        colour(27, 200, 100, 50, 255); c0(31); c4(63, D(5), D(5), D(40), D(30));
        colour(30, 10, 200, 20, 255); c1(42, D(2.5)); c0(28); c0(82); c2(83, D(3), D(60)); c2(85, D(30), D(10)); c2(85, D(60), D(50)); c1(103, 1);
    }
    printf("scan: TIMING %s attachloop process-cpu-seconds %.3f for %d attach+clear+rect+path\n", cfg_full, st_cpu() - t0, N);
    printf("scan: VALUE %s final %#llx\n", cfg_full, ULL(fnv(sb, NBYTES)));
}

/* Decision 0160: the rotation of the context (Agg2D slot 55 -> applyTransformOp(ROTATE), which needs sin/cos): the userTransform after rotating by a series of angles, as hex
 * words, read from the state the wrapper sees. The lifted DLL computes sin/cos with its own C runtime (MSVC), the swapped build with the host's libm: the values show whether the
 * two agree bit for bit. VALUE lines are compared between builds. */
TEST(rotprobe) {
    static const double ang[] = {0.1, 0.25, 0.5, 0.7853981633974483, 1.0, 1.0471975511965976, 1.5707963267948966, 2.0, 2.5, 3.0, 3.141592653589793, 4.0, 5.5, 6.0, -0.3, -1.7, 12.345, 100.0};
    unsigned i, k;
    for (i = 0; i < sizeof ang / sizeof *ang; i++) {
        uint64_t impl, st;
        c0(52);
        c1(55, D(ang[i]));
        impl = fle_ld(obj + 0x18, 8); st = fle_ld(impl + 8, 8);
        printf("scan: VALUE %s rot%u", cfg_full, i);
        for (k = 0; k < 6; k++) printf(" %#llx", ULL(fle_ld(st + 0xc0 + 8 * k, 8)));
        printf("\n");
    }
}

/* Decision 0160: which function of the DLL's C runtime is sin, cos, ...: calls candidates (RVAs in FLE_B2D1_LIBM, default a list) as functions of one double (xmm0 in, xmm0 out)
 * for a few inputs and prints the result words; the caller matches them with the host's libm. Not compared between builds (the candidates are lifted in both). */
TEST(libmprobe) {
    static const double xs[][2] = {{0.7853981633974483, 0.7853981633974483}, {1.0, 1.0}, {0.5, 2.0}, {7.5, 2.0}, {27.0, 1.0}, {-0.25, 3.0}};
    unsigned k;
    unsigned long cand = 0;
    const char *e = getenv("FLE_B2D1_LIBM");
    uint64_t base = fle_realmod_base_of(fle_realmod_find(B2D_NAME));
    if (e) cand = strtoul(e, NULL, 0);
    if (!cand) return;
    printf("scan: LIBM %#lx", cand);
    for (k = 0; k < sizeof xs / sizeof *xs; k++) {
        X[0].u64[0] = D(xs[k][0]); X[0].u64[1] = 0; X[1].u64[0] = D(xs[k][1]); X[1].u64[1] = 0;
        gcall(base + cand, 0, 0, 0, 0, 0, NULL);
        printf(" %#llx", ULL(X[0].u64[0]));
    }
    printf("\n");
    fflush(stdout);
}
