/* Decision 0195 (opt-in test tool, never linked into the engine): one random draw per scene, through the public Blend2D API, for one TUPLE
 * (composition operator x target format x geometry x fill|stroke x style). Linked once against a Blend2D build and run with the same arguments against another;
 * the per-scene FNV-1a hashes of the final pixel buffer are written to a file (8 bytes per scene) and the two files are compared with cmp.
 *   libraries: upstream a7f9476 WITH the JIT (the stand-in for the real DLL, checked against the DLL under Wine by the Agg2D fuzz of the same decision),
 *              upstream without JIT + the templates of lift/harness/b2d1/b2d3-compop.patch (the replacement), upstream without JIT unpatched.
 * usage: b2d3_diff OP=<name|n> FMT=prgb32|xrgb32 GEOM=<g> KIND=fill|stroke STYLE=<s> N=<scenes> SEED=<n> [GA=0|1] [GQ=0|1|2|r] [CLIP=0|1] [XF=0|1] [OUT=<file>] [DUMP=<scene> DUMPFILE=<f>] [NOJIT=1]
 *   GEOM : rectd recti rrect ellipse circle triangle polygon path(lines) pathc(curves) pathm(mixed) line box (fill) , STYLE: rgba32 (…Rgba32 call) solid(rgba) solid32 solid64 lin rad con
 * The generator is a pure function of (SEED, scene index): the same arguments give the same scenes in every build.  Prints one RESULT line. */
#include <blend2d.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <vector>

static uint64_t fnv(const uint8_t *p, size_t n) { uint64_t h = 0xcbf29ce484222325ULL; for (size_t i = 0; i < n; i++) h = (h ^ p[i]) * 0x100000001b3ULL; return h; }
struct Rng {
    uint64_t s;
    uint64_t next() { s += 0x9e3779b97f4a7c15ULL; uint64_t z = s; z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL; z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL; return z ^ (z >> 31); }
    uint32_t u(uint32_t n) { return (uint32_t)(next() % n); }
    double f(double a, double b) { return a + (b - a) * ((next() >> 11) * (1.0 / 9007199254740992.0)); }
    bool p(double pr) { return f(0, 1) < pr; }
};

static const char *const op_names[] = {"SRC_OVER", "SRC_COPY", "SRC_IN", "SRC_OUT", "SRC_ATOP", "DST_OVER", "DST_COPY", "DST_IN", "DST_OUT", "DST_ATOP", "XOR", "CLEAR", "PLUS", "MINUS", "MODULATE", "MULTIPLY", "SCREEN", "OVERLAY", "DARKEN", "LIGHTEN", "COLOR_DODGE", "COLOR_BURN", "LINEAR_BURN", "LINEAR_LIGHT", "PIN_LIGHT", "HARD_LIGHT", "SOFT_LIGHT", "DIFFERENCE", "EXCLUSION"};
static const char *arg(int argc, char **argv, const char *k, const char *def) { size_t n = strlen(k); for (int i = 1; i < argc; i++) if (!strncmp(argv[i], k, n) && argv[i][n] == '=') return argv[i] + n + 1; return def; }

struct Mode { const char *name; uint32_t flags, cpu; };
/* host: whatever asmjit reports for this machine; sse2/sse41/avx2: the JIT restricted to that level (BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES; the lifted guest sees SSE2 only, decision 0118); nojit: the fixed (reference) pipeline */
static Mode parse_mode(const char *n) {
    const uint32_t S2 = BL_RUNTIME_CPU_FEATURE_X86_SSE2, S3 = S2 | BL_RUNTIME_CPU_FEATURE_X86_SSE3, SS = S3 | BL_RUNTIME_CPU_FEATURE_X86_SSSE3, S41 = SS | BL_RUNTIME_CPU_FEATURE_X86_SSE4_1, S42 = S41 | BL_RUNTIME_CPU_FEATURE_X86_SSE4_2,
                   AV = S42 | BL_RUNTIME_CPU_FEATURE_X86_AVX, A2 = AV | BL_RUNTIME_CPU_FEATURE_X86_AVX2;
    if (!strcmp(n, "host")) return Mode{"host", 0, 0};
    if (!strcmp(n, "nojit")) return Mode{"nojit", BL_CONTEXT_CREATE_FLAG_DISABLE_JIT, 0};
    if (!strcmp(n, "sse2")) return Mode{"sse2", BL_CONTEXT_CREATE_FLAG_ISOLATED_JIT_RUNTIME | BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES, S2};
    if (!strcmp(n, "ssse3")) return Mode{"ssse3", BL_CONTEXT_CREATE_FLAG_ISOLATED_JIT_RUNTIME | BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES, SS};
    if (!strcmp(n, "sse41")) return Mode{"sse41", BL_CONTEXT_CREATE_FLAG_ISOLATED_JIT_RUNTIME | BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES, S41};
    if (!strcmp(n, "avx")) return Mode{"avx", BL_CONTEXT_CREATE_FLAG_ISOLATED_JIT_RUNTIME | BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES, AV};
    if (!strcmp(n, "avx2")) return Mode{"avx2", BL_CONTEXT_CREATE_FLAG_ISOLATED_JIT_RUNTIME | BL_CONTEXT_CREATE_FLAG_OVERRIDE_CPU_FEATURES, A2};
    fprintf(stderr, "unknown mode %s\n", n); exit(2);
}
struct Params { int op, fmt; const char *geom, *kind, *style; int ga, gq, clip, xf; };

static uint32_t rand_color(Rng &r, int allowAlpha = 1) {
    uint32_t a = !allowAlpha ? 255 : r.p(0.3) ? 255 : r.p(0.05) ? 0 : r.u(256);
    return (a << 24) | (r.u(256) << 16) | (r.u(256) << 8) | r.u(256);   /* the context premultiplies */
}
static void fill_dst(Rng &r, uint8_t *buf, int w, int h, int fmt) {
    int mode = (int)r.u(4);
    for (int i = 0; i < w * h; i++) {
        uint32_t a, rr, g, b;
        if (fmt == 1) { a = 255; rr = r.u(256); g = r.u(256); b = r.u(256); }
        else {
            a = mode == 0 ? 255 : mode == 1 ? r.u(256) : mode == 2 ? (r.p(0.5) ? 0 : r.u(256)) : r.u(256);
            if (mode == 3 && r.p(0.3)) a = 255;
            rr = r.u(a + 1); g = r.u(a + 1); b = r.u(a + 1);
            if (a == 0) rr = g = b = 0;
        }
        uint32_t px = (a << 24) | (rr << 16) | (g << 8) | b;
        memcpy(buf + 4 * i, &px, 4);
    }
}
static BLPoint rp(Rng &r, int w, int h) { return BLPoint(r.f(-6, w + 6), r.f(-6, h + 6)); }
static BLMatrix2D rand_matrix(Rng &r, int w, int h) {
    BLMatrix2D m = BLMatrix2D::makeIdentity();
    m.rotate(r.f(-3.2, 3.2), w * 0.5, h * 0.5); m.scale(r.f(0.5, 1.8), r.f(0.5, 1.8)); m.translate(r.f(-8, 8), r.f(-8, 8));
    return m;
}
static void build_gradient(Rng &r, BLGradient &g, int kind, int w, int h) {
    BLExtendMode ext = (BLExtendMode)(r.u(3) == 0 ? BL_EXTEND_MODE_PAD : r.u(2) ? BL_EXTEND_MODE_REPEAT : BL_EXTEND_MODE_REFLECT);
    if (kind == 0) { BLLinearGradientValues v(r.f(-4, w + 4), r.f(-4, h + 4), r.f(-4, w + 4), r.f(-4, h + 4)); g.create(v, ext); }
    else if (kind == 1) { double cx = r.f(0, w), cy = r.f(0, h); BLRadialGradientValues v(cx, cy, cx + r.f(-6, 6), cy + r.f(-6, 6), r.f(2, w * 0.8 + 4), 0.0); g.create(v, ext); }
    else { BLConicGradientValues v(r.f(0, w), r.f(0, h), r.f(0, 6.28), 1.0); g.create(v, ext); }
    int n = 2 + (int)r.u(4); bool opaque = r.p(0.3);
    std::vector<double> off; off.push_back(r.p(0.7) ? 0.0 : r.f(0, 0.3)); for (int i = 1; i < n - 1; i++) off.push_back(r.f(0, 1)); off.push_back(r.p(0.7) ? 1.0 : r.f(0.7, 1));
    for (size_t i = 1; i < off.size(); i++) for (size_t j = i; j > 0 && off[j] < off[j - 1]; j--) { double t = off[j]; off[j] = off[j - 1]; off[j - 1] = t; }
    for (int i = 0; i < n; i++) g.addStop(off[i], BLRgba32(rand_color(r, !opaque)));
    if (r.p(0.3)) g.setTransform(rand_matrix(r, w, h));
}
static void build_path(Rng &r, BLPath &p, int w, int h, int mode) {   /* 0 lines, 1 curves, 2 mixed */
    p.moveTo(rp(r, w, h)); int n = 2 + (int)r.u(6);
    for (int i = 0; i < n; i++) {
        int k = mode == 0 ? 0 : mode == 1 ? 1 + (int)r.u(2) : (int)r.u(3);
        if (k == 0) p.lineTo(rp(r, w, h)); else if (k == 1) p.quadTo(rp(r, w, h), rp(r, w, h)); else p.cubicTo(rp(r, w, h), rp(r, w, h), rp(r, w, h));
    }
    if (r.p(0.7)) p.close();
}

int main(int argc, char **argv) {
    blRuntimeInit();
    Params P; const char *s;
    s = arg(argc, argv, "OP", "17"); P.op = -1; for (int i = 0; i < 29; i++) if (!strcmp(s, op_names[i])) P.op = i; if (P.op < 0) P.op = atoi(s);
    s = arg(argc, argv, "FMT", "prgb32"); P.fmt = !strcmp(s, "xrgb32") ? 1 : 0;
    P.geom = arg(argc, argv, "GEOM", "pathm"); P.kind = arg(argc, argv, "KIND", "fill"); P.style = arg(argc, argv, "STYLE", "solid");
    P.ga = atoi(arg(argc, argv, "GA", "1")); s = arg(argc, argv, "GQ", "2"); P.gq = s[0] == 'r' ? -1 : atoi(s); P.clip = atoi(arg(argc, argv, "CLIP", "0")); P.xf = atoi(arg(argc, argv, "XF", "0"));
    long N = atol(arg(argc, argv, "N", "1000")); uint64_t seed = strtoull(arg(argc, argv, "SEED", "1"), 0, 0);
    const char *out = arg(argc, argv, "OUT", 0); long dump = atol(arg(argc, argv, "DUMP", "-1")); const char *dumpfile = arg(argc, argv, "DUMPFILE", "dump.bin"); int nojit = atoi(arg(argc, argv, "NOJIT", "0"));
    std::vector<Mode> modes; { char tmp[256]; strncpy(tmp, arg(argc, argv, "MODES", nojit ? "nojit" : "host"), 255); tmp[255] = 0; for (char *t = strtok(tmp, ","); t; t = strtok(NULL, ",")) modes.push_back(parse_mode(t)); }
    const size_t NM = modes.size();
    const bool stroke = !strcmp(P.kind, "stroke");
    const char *out2 = arg(argc, argv, "OUT2", 0);
    std::vector<FILE *> fo(NM, (FILE *)NULL), fp(NM, (FILE *)NULL);
    for (size_t m = 0; m < NM; m++) { char nm[512]; if (out) { snprintf(nm, sizeof nm, NM > 1 ? "%s.%s" : "%s", out, modes[m].name); fo[m] = fopen(nm, "wb"); } if (out2) { snprintf(nm, sizeof nm, NM > 1 ? "%s.%s" : "%s", out2, modes[m].name); fp[m] = fopen(nm, "wb"); } }
    const int probe = out2 != NULL;
    struct Stat { long scenes_diff = 0, pix_diff = 0, max_pix = 0, max_ch = 0, h1 = 0, h2 = 0, h3_4 = 0, h5 = 0, first = -1, big_scenes = 0; };
    std::vector<Stat> stt(NM);
    static uint8_t bufm[8][80 * 80 * 4];
    uint64_t chain = 0xcbf29ce484222325ULL; long changed = 0, notimpl = 0, other_rc = 0;
    static uint8_t buf[80 * 80 * 4], init[80 * 80 * 4], buf2[80 * 80 * 4];
    for (long sc = 0; sc < N; sc++) {
        Rng r{seed * 0x100000001b3ULL + (uint64_t)sc * 0x9e3779b97f4a7c15ULL + 12345};
        int w = 8 + (int)r.u(65), h = 8 + (int)r.u(65);
        fill_dst(r, buf, w, h, P.fmt); memcpy(init, buf, (size_t)w * h * 4);
        int gq = P.gq >= 0 ? P.gq : (int)r.u(3);
        BLFillRule frule = r.p(0.5) ? BL_FILL_RULE_EVEN_ODD : BL_FILL_RULE_NON_ZERO;
        bool hasga = P.ga && r.p(0.5); double gav = hasga ? r.f(0.05, 1.0) : 1.0;
        bool hasxf = P.xf && r.p(0.5); BLMatrix2D xfm = hasxf ? rand_matrix(r, w, h) : BLMatrix2D::makeIdentity();
        bool hasclip = P.clip && r.p(0.5); BLRect cliprc = hasclip ? BLRect(r.f(0, w * 0.5), r.f(0, h * 0.5), r.f(4, w), r.f(4, h)) : BLRect(0, 0, 0, 0);
        double sw = 0, ml = 0; BLStrokeCap cap = BL_STROKE_CAP_BUTT; BLStrokeJoin jn = BL_STROKE_JOIN_MITER_CLIP;
        if (stroke) { sw = r.p(0.2) ? r.f(0.1, 1) : r.f(1, 9); cap = (BLStrokeCap)r.u(6); jn = (BLStrokeJoin)r.u(5); ml = r.f(1, 6); }
        const char *st = P.style; bool direct = !strcmp(st, "rgba32");
        uint32_t c32 = rand_color(r);
        BLGradient grad; BLRgba rgbaF = BLRgba(BLRgba32(c32)); BLRgba64 rgba64 = BLRgba64(BLRgba32(c32)); if (!strcmp(st, "solid64") && r.p(0.5)) rgba64.value ^= (uint64_t)r.u(256) << 8;
        if (!strcmp(st, "lin") || !strcmp(st, "rad") || !strcmp(st, "con")) build_gradient(r, grad, !strcmp(st, "lin") ? 0 : !strcmp(st, "rad") ? 1 : 2, w, h);
        /* geometry */
        const char *g = P.geom; BLGeometryType gt; BLPath path; union { BLBoxI bi; BLBox bd; BLRectI ri; BLRect rd; BLRoundRect rr; BLEllipse el; BLCircle ci; BLTriangle tr; BLLine ln; } u; memset(&u, 0, sizeof u);
        std::vector<BLPoint> pts; BLArrayView<BLPoint> view; const void *gd = &u;
        bool al = r.p(0.3);   /* pixel-aligned */
        auto cx = [&](double lim) { double v = r.f(-4, lim + 4); return al ? floor(v) : v; };
        if (!strcmp(g, "rectd")) { gt = BL_GEOMETRY_TYPE_RECTD; double x = cx(w), y = cx(h); u.rd = BLRect(x, y, al ? floor(r.f(1, w)) : r.f(0.5, w), al ? floor(r.f(1, h)) : r.f(0.5, h)); }
        else if (!strcmp(g, "recti")) { gt = BL_GEOMETRY_TYPE_RECTI; u.ri = BLRectI((int)r.f(-4, w), (int)r.f(-4, h), 1 + (int)r.u(w), 1 + (int)r.u(h)); }
        else if (!strcmp(g, "box")) { gt = BL_GEOMETRY_TYPE_BOXD; double x0 = cx(w), y0 = cx(h); u.bd = BLBox(x0, y0, x0 + (al ? floor(r.f(1, w)) : r.f(0.5, w)), y0 + (al ? floor(r.f(1, h)) : r.f(0.5, h))); }
        else if (!strcmp(g, "rrect")) { gt = BL_GEOMETRY_TYPE_ROUND_RECT; u.rr = BLRoundRect(r.f(-2, w * 0.5), r.f(-2, h * 0.5), r.f(3, w), r.f(3, h), r.f(0, 12), r.f(0, 12)); }
        else if (!strcmp(g, "ellipse")) { gt = BL_GEOMETRY_TYPE_ELLIPSE; u.el = BLEllipse(r.f(0, w), r.f(0, h), r.f(1, w * 0.7), r.f(1, h * 0.7)); }
        else if (!strcmp(g, "circle")) { gt = BL_GEOMETRY_TYPE_CIRCLE; u.ci = BLCircle(r.f(0, w), r.f(0, h), r.f(1, w * 0.7)); }
        else if (!strcmp(g, "triangle")) { gt = BL_GEOMETRY_TYPE_TRIANGLE; BLPoint a = rp(r, w, h), b = rp(r, w, h), c = rp(r, w, h); u.tr = BLTriangle(a.x, a.y, b.x, b.y, c.x, c.y); }
        else if (!strcmp(g, "line")) { gt = BL_GEOMETRY_TYPE_LINE; BLPoint a = rp(r, w, h), b = rp(r, w, h); u.ln = BLLine(a.x, a.y, b.x, b.y); }
        else if (!strcmp(g, "polygon")) { gt = BL_GEOMETRY_TYPE_POLYGOND; int n = 3 + (int)r.u(6); for (int i = 0; i < n; i++) pts.push_back(rp(r, w, h)); view.reset(pts.data(), pts.size()); gd = &view; }
        else if (!strcmp(g, "polyline")) { gt = BL_GEOMETRY_TYPE_POLYLINED; int n = 2 + (int)r.u(7); for (int i = 0; i < n; i++) pts.push_back(rp(r, w, h)); view.reset(pts.data(), pts.size()); gd = &view; }
        else { gt = BL_GEOMETRY_TYPE_PATH; build_path(r, path, w, h, !strcmp(g, "path") ? 0 : !strcmp(g, "pathc") ? 1 : 2); gd = &path; }
        /* one run: the draw of the tuple on `pix` (w x h) under composition operator `op` */
        auto run = [&](uint8_t *pix, int op, const Mode &md) -> BLResult {
            BLImage img; img.createFromData(w, h, P.fmt ? BL_FORMAT_XRGB32 : BL_FORMAT_PRGB32, pix, w * 4);
            BLContextCreateInfo ci; memset(&ci, 0, sizeof ci); ci.flags = md.flags; ci.cpuFeatures = md.cpu;
            BLContext ctx; ctx.begin(img, ci);
            ctx.setGradientQuality((BLGradientQuality)(gq == 0 ? BL_GRADIENT_QUALITY_NEAREST : gq == 1 ? BL_GRADIENT_QUALITY_SMOOTH : BL_GRADIENT_QUALITY_DITHER));
            ctx.setFillRule(frule);
            if (hasga) ctx.setGlobalAlpha(gav);
            if (hasxf) ctx.setTransform(xfm);
            if (hasclip) ctx.clipToRect(cliprc);
            if (stroke) { ctx.setStrokeWidth(sw); ctx.setStrokeCaps(cap); ctx.setStrokeJoin(jn); ctx.setStrokeMiterLimit(ml); }
            ctx.setCompOp((BLCompOp)op);
            if (!direct) {
                if (!strcmp(st, "solid")) { if (stroke) ctx.setStrokeStyle(rgbaF); else ctx.setFillStyle(rgbaF); }
                else if (!strcmp(st, "solid32")) { if (stroke) ctx.setStrokeStyle(BLRgba32(c32)); else ctx.setFillStyle(BLRgba32(c32)); }
                else if (!strcmp(st, "solid64")) { if (stroke) ctx.setStrokeStyle(rgba64); else ctx.setFillStyle(rgba64); }
                else { if (stroke) ctx.setStrokeStyle(grad); else ctx.setFillStyle(grad); }
            }
            BLResult rc;
            if (direct) rc = stroke ? blContextStrokeGeometryRgba32(&ctx, gt, gd, c32) : blContextFillGeometryRgba32(&ctx, gt, gd, c32);
            else rc = stroke ? blContextStrokeGeometry(&ctx, gt, gd) : blContextFillGeometry(&ctx, gt, gd);
            ctx.end();
            return rc;
        };
        BLResult rc = BL_SUCCESS;
        for (size_t m = 0; m < NM; m++) {
            uint8_t *bm = m == 0 ? buf : bufm[m]; if (m) memcpy(bm, init, (size_t)w * h * 4);
            BLResult rm = run(bm, P.op, modes[m]); if (m == 0) rc = rm;
            if (probe) {   /* the fetch probe: the same draw as SRC_COPY on a zero canvas (supported by every library), to tell a difference of the colour source from one of the operator */
                memset(buf2, 0, (size_t)w * h * 4); if (P.fmt) for (int i = 0; i < w * h; i++) buf2[4 * i + 3] = 255;
                run(buf2, BL_COMP_OP_SRC_COPY, modes[m]);
                uint64_t ph = fnv(buf2, (size_t)w * h * 4); if (fp[m]) fwrite(&ph, 8, 1, fp[m]);
            }
            uint64_t hm = fnv(bm, (size_t)w * h * 4); if (fo[m]) fwrite(&hm, 8, 1, fo[m]);
            if (m) {   /* statistics against mode 0 */
                long pd = 0, mc = 0;
                for (int i = 0; i < w * h; i++) { int dd = 0; for (int c = 0; c < 4; c++) { int a = bm[4 * i + c] - buf[4 * i + c]; if (a < 0) a = -a; if (a > dd) dd = a; } if (dd) { pd++; if (dd > mc) mc = dd; if (dd == 1) stt[m].h1++; else if (dd == 2) stt[m].h2++; else if (dd <= 4) stt[m].h3_4++; else stt[m].h5++; } }
                if (pd) { if (stt[m].first < 0) stt[m].first = sc; stt[m].scenes_diff++; stt[m].pix_diff += pd; if (pd > stt[m].max_pix) stt[m].max_pix = pd; if (mc > stt[m].max_ch) stt[m].max_ch = mc; if (mc > 1) stt[m].big_scenes++; }
            }
        }
        if (rc == BL_ERROR_NOT_IMPLEMENTED) notimpl++; else if (rc != BL_SUCCESS) other_rc++;
        uint64_t hsh = fnv(buf, (size_t)w * h * 4);
        if (memcmp(buf, init, (size_t)w * h * 4)) changed++;
        chain = (chain ^ hsh) * 0x100000001b3ULL;
        if (sc == dump) { FILE *fd = fopen(dumpfile, "wb"); int hdr[2] = {w, h}; fwrite(hdr, 4, 2, fd); fwrite(init, 1, (size_t)w * h * 4, fd); fwrite(buf, 1, (size_t)w * h * 4, fd); fclose(fd); }
    }
    for (size_t m = 0; m < NM; m++) { if (fo[m]) fclose(fo[m]); if (fp[m]) fclose(fp[m]); }
    printf("RESULT op=%s fmt=%s geom=%s kind=%s style=%s ga=%d gq=%d clip=%d xf=%d N=%ld seed=%llu changed=%ld notimpl=%ld other_rc=%ld chain=%016llx\n", op_names[P.op < 29 ? P.op : 0], P.fmt ? "xrgb32" : "prgb32", P.geom, P.kind, P.style, P.ga, P.gq, P.clip, P.xf, N,
           (unsigned long long)seed, changed, notimpl, other_rc, (unsigned long long)chain);
    for (size_t m = 1; m < NM; m++)
        printf("STAT mode=%s vs %s tuple=%s/%s/%s/%s/%s N=%ld scenes_diff=%ld (%.4f%%) scenes_diff_gt1=%ld pix_diff_total=%ld max_pix_per_scene=%ld max_channel_diff=%ld pix_by_maxdiff: 1=%ld 2=%ld 3-4=%ld 5+=%ld first_scene=%ld\n", modes[m].name, modes[0].name, op_names[P.op < 29 ? P.op : 0], P.fmt ? "xrgb32" : "prgb32", P.geom, P.kind, P.style, N,
               stt[m].scenes_diff, 100.0 * stt[m].scenes_diff / (N ? N : 1), stt[m].big_scenes, stt[m].pix_diff, stt[m].max_pix, stt[m].max_ch, stt[m].h1, stt[m].h2, stt[m].h3_4, stt[m].h5, stt[m].first);
    return 0;
}
