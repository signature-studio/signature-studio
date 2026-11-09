/* Decision 0195 (opt-in): the scan test `b3ov` (FLE_B2D_SCAN=b3ov) runs the scenes of b3scene.h through the lifted Agg2D wrapper, i.e. with the adapter and the upstream Blend2D of the build under test.
 * Parameters from the environment: B3_OP B3_STYLE B3_GEOM B3_RAMPNOISE B3_OPAQUE B3_SEED B3_N B3_IDX0 (defaults 16 1 1 0 0 1 100 0). Prints `scan: VALUE b3ov b3 <idx> <hash>` per scene, the same hash as
 * lift/harness/b2d1/b3_wine.c prints for the real DLL. Included by b2d_scan.c (after the helpers c0..c8, cp, fle_st) and used only through the table entry T(b3ov, ...). */
#include "b2d1/b3scene.h"
static unsigned b3e(const char *n, unsigned d) { const char *v = getenv(n); return v && *v ? (unsigned)strtoul(v, NULL, 0) : d; }
static void b3s_call(B3Env *e, unsigned slot, int n, const uint64_t *w) {
    (void)e;
    cp(slot, n > 0 ? w[0] : 0, n > 1 ? w[1] : 0, n > 2 ? w[2] : 0, n > 3 ? n - 3 : 0, w + 3);
}
static uint64_t b3s_ret(B3Env *e, unsigned slot, int n, const uint64_t *w) {
    (void)e;
    return cp(slot, n > 0 ? w[0] : 0, n > 1 ? w[1] : 0, n > 2 ? w[2] : 0, n > 3 ? n - 3 : 0, w + 3);
}
static void b3s_poke(B3Env *e, uint64_t ptr, unsigned off, uint16_t v) { (void)e; fle_st(ptr + off, 2, v); }
static void b3s_mark(B3Env *e, const char *tag) { if (!strcmp(tag, "post") && getenv("B3_DUMP")) { FILE *f = fopen(getenv("B3_DUMP"), "ab"); fwrite(e->pix, 1, NBYTES, f); fclose(f); } if (strcmp(tag, "post")) printf("scan: VALUE %s mark %s %016llx\n", cfg_full, tag, ULL(fnv(e->pix, NBYTES))); }
TEST(b3ov) {
    B3Env e = {b3s_call, b3s_ret, b3s_poke, pix, 0, getenv("B3_DEBUG") ? b3s_mark : NULL};
    unsigned op = b3e("B3_OP", 16), style = b3e("B3_STYLE", 1), geom = b3e("B3_GEOM", 1), rn = b3e("B3_RAMPNOISE", 0), opq = b3e("B3_OPAQUE", 0);
    uint64_t seed = b3e("B3_SEED", 1);
    long n = (long)b3e("B3_N", 100), i0 = (long)b3e("B3_IDX0", 0), i;
    for (i = i0; i < i0 + n; i++) {
        b3_scene(&e, op, style, geom, rn, opq, seed, i);
        printf("scan: VALUE %s b3 %ld %016llx\n", cfg_full, i, ULL(fnv(pix, NBYTES)));
    }
}
