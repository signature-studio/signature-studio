//! Tests of decision 0240 (docs/0240-window-move-and-frame-rate.md, PROVISIONAL): the scene description of the desktop (runtime/fle_desk.c: fle_desk_scene_mode / fle_desk_scene) that the browser's WebGL2
//! presenter (app/src/engine/desk-gl.ts) draws from. The window table, the surfaces and the layered state are the STUBS of desk1_compose.rs (its C prefix is taken from that file); what is tested is what the description says.
//!
//! The browser is modelled by a MIRROR: one array of pixels per window, allocated when the description says `fresh`, and written ONLY with the rectangles of the description's updates. After every step the mirror of every window
//! must hold exactly the window's surface (nothing the browser needs was left out), and the windows must be listed in the order of the window table, bottom first, with the table's position and size.
//! Expected values are derived from the stubs' state, not read back from the C code.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("desk_scene");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const MAIN: &str = r##"
static uint64_t rs = 12345;
static uint32_t rnd(void) { rs = rs * 1664525u + 1013904223u; return rs >> 8; }
static uint8_t *tex[8];
static uint32_t tw[8], th[8];
static long long area_all;

/* decision 0244 (alpha1): the window table's frame margins (DwmExtendFrameIntoClientArea) and window region box (SetWindowRgn); fle_desk.c declares both weak */
static int gl_on[8], rg_on[8];
static int32_t gl_m[8][4], rg_b[8][4];
int fle_win_frame_margins(uint64_t h, int32_t m[4]) { unsigned l = (unsigned)(h - 0x1000); if (l >= 8 || !gl_on[l]) return 0; memcpy(m, gl_m[l], sizeof gl_m[l]); return 1; }
int fle_win_window_region_box(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b) { unsigned i = (unsigned)(h - 0x1000); if (i >= 8 || !rg_on[i]) return 0; *l = rg_b[i][0]; *t = rg_b[i][1]; *r = rg_b[i][2]; *b = rg_b[i][3]; return 1; }

/* the browser: apply a description to the mirror; returns the number of windows whose mirror differs from the surface afterwards */
static int apply(const fle_scene *sc, unsigned *nupd_out) {
    int bad = 0;
    for (uint32_t i = 0; i < sc->n_windows; i++) {
        const fle_scene_win *w = &sc->win[i];
        if (w->fresh) { free(tex[w->lid]); tex[w->lid] = calloc(1, (size_t)w->w * w->h * 4); tw[w->lid] = w->w; th[w->lid] = w->h; }
    }
    for (uint32_t k = 0; k < sc->n_updates; k++) {
        const fle_scene_upd *u = &sc->upd[k];
        win_t *p = &W[u->lid];
        CHECK(tex[u->lid] != NULL && u->x0 >= 0 && u->y0 >= 0 && u->x1 <= (int32_t)tw[u->lid] && u->y1 <= (int32_t)th[u->lid] && u->x0 < u->x1 && u->y0 < u->y1, "update %u of lid %u is inside its texture", k, u->lid);
        if (!tex[u->lid]) continue;
        for (int32_t y = u->y0; y < u->y1; y++) memcpy(tex[u->lid] + ((size_t)y * tw[u->lid] + u->x0) * 4, p->s.bits + (size_t)y * p->s.stride + (size_t)u->x0 * 4, (size_t)(u->x1 - u->x0) * 4);
        area_all += (long long)(u->x1 - u->x0) * (u->y1 - u->y0);
    }
    *nupd_out = sc->n_updates;
    for (uint32_t i = 0; i < sc->n_windows; i++) {
        const fle_scene_win *w = &sc->win[i];
        if (!tex[w->lid] || memcmp(tex[w->lid], W[w->lid].s.bits, (size_t)w->w * w->h * 4) != 0) bad++;
    }
    return bad;
}
/* the windows the table says are drawn, bottom first */
static unsigned expect_list(unsigned *lids) {
    unsigned n = 0;
    for (int i = (int)norder - 1; i >= 0; i--) if (W[order[i]].visible) lids[n++] = order[i];
    return n;
}
static int check_list(const fle_scene *sc, const char *when) {
    unsigned want[8], n = expect_list(want);
    int ok = sc->n_windows == n;
    for (unsigned i = 0; ok && i < n; i++) ok = sc->win[i].lid == want[i] && (int32_t)sc->win[i].x == W[want[i]].x && (int32_t)sc->win[i].y == W[want[i]].y && sc->win[i].w == W[want[i]].w && sc->win[i].h == W[want[i]].h;
    CHECK(ok, "%s: the windows, their order, positions and sizes are the table's (n_windows %u, want %u)", when, sc->n_windows, n);
    return ok;
}

int main(void) {
    const fle_scene *sc;
    unsigned nu;

    fle_desk_scene_mode(1);
    /* 1. an empty desktop: header, no window, whole */
    sc = fle_desk_scene();
    CHECK(sc && sc->magic == FLE_SCENE_MAGIC && sc->version == FLE_SCENE_VERSION && sc->width == 1920 && sc->height == 1080 && sc->n_windows == 0 && sc->n_updates == 0, "empty header");
    CHECK((sc->flags & FLE_SCENE_FLAG_WHOLE) && sc->background == (5u | (7u << 8) | (10u << 16)), "first call is whole; background colour");
    /* 2. one window: described, its whole surface is the update, fresh */
    mkwin(1, 10, 20, 100, 50, 0x11, 0x22, 0x33);
    order[0] = 1; norder = 1;
    sc = fle_desk_scene();
    CHECK(sc->n_windows == 1 && sc->win[0].lid == 1 && sc->win[0].x == 10 && sc->win[0].y == 20 && sc->win[0].w == 100 && sc->win[0].h == 50 && sc->win[0].stride == 400 && sc->win[0].fresh, "a new window is described and fresh");
    CHECK(sc->n_updates == 1 && sc->upd[0].lid == 1 && sc->upd[0].x0 == 0 && sc->upd[0].y0 == 0 && sc->upd[0].x1 == 100 && sc->upd[0].y1 == 50 && (sc->flags & FLE_SCENE_FLAG_CHANGED), "its whole surface is the update");
    CHECK(apply(sc, &nu) == 0, "mirror equals surface");
    /* 3. nothing changed: nothing to do */
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0 && !(sc->flags & (FLE_SCENE_FLAG_CHANGED | FLE_SCENE_FLAG_WHOLE)) && !sc->win[0].fresh, "unchanged: no update, not changed");
    /* 4. the same pixels written again are not reported; one changed pixel inside a big written rectangle is reported alone */
    fle_desk_wrote(1, 0, 0, 100, 50);
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0, "the same pixels again: %u updates", sc->n_updates);
    setpx(1, 20, 10, 1, 2, 3, 0);
    fle_desk_wrote(1, 0, 0, 100, 50);
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 1 && sc->upd[0].x0 == 20 && sc->upd[0].y0 == 10 && sc->upd[0].x1 == 21 && sc->upd[0].y1 == 11, "one pixel: %u updates", sc->n_updates);
    CHECK(apply(sc, &nu) == 0, "mirror equals surface after the pixel");
    /* 5. a window moves: the description changes, no pixel is sent */
    W[1].x = 300; W[1].y = 200;
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED) && sc->win[0].x == 300 && sc->win[0].y == 200 && !sc->win[0].fresh, "a move sends no pixel");
    /* 6. restack, hide, show */
    mkwin(2, 50, 40, 100, 50, 0x44, 0x55, 0x66);
    order[0] = 2; order[1] = 1; norder = 2;
    sc = fle_desk_scene();
    CHECK(sc->n_windows == 2 && sc->win[0].lid == 1 && sc->win[1].lid == 2 && sc->win[1].fresh && !sc->win[0].fresh, "two windows, bottom first, only the new one fresh");
    CHECK(apply(sc, &nu) == 0 && nu == 1, "only window 2's surface sent (%u updates)", nu);
    order[0] = 1; order[1] = 2;
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED) && sc->win[0].lid == 2 && sc->win[1].lid == 1, "restacking sends no pixel");
    W[2].visible = 0;
    sc = fle_desk_scene();
    CHECK(sc->n_windows == 1 && sc->win[0].lid == 1 && sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED), "hidden: gone from the description");
    W[2].visible = 1;
    sc = fle_desk_scene();
    CHECK(sc->n_windows == 2 && sc->win[0].lid == 2 && sc->win[0].fresh, "shown again: described again, fresh (the browser may have dropped it)");
    apply(sc, &nu);
    /* 7. a window with another size: fresh with the whole surface */
    free(W[1].s.bits); W[1].w = 120; W[1].h = 70; W[1].s.width = 120; W[1].s.height = 70; W[1].s.stride = 480; W[1].s.bits = calloc(1, 120 * 70 * 4); fillrect(1, 0, 0, 120, 70, 9, 9, 9);
    sc = fle_desk_scene();
    CHECK(sc->win[1].lid == 1 && sc->win[1].w == 120 && sc->win[1].fresh && sc->n_updates == 1 && sc->upd[0].x1 == 120 && sc->upd[0].y1 == 70, "resized: fresh, whole");
    CHECK(apply(sc, &nu) == 0, "mirror equals surface after the resize");
    /* 8. fle_desk_resync: everything is sent again */
    fle_desk_resync();
    sc = fle_desk_scene();
    CHECK((sc->flags & FLE_SCENE_FLAG_WHOLE) && sc->n_updates == sc->n_windows && sc->n_windows == 2, "resync: every window whole (%u updates)", sc->n_updates);
    CHECK(apply(sc, &nu) == 0, "mirror equals surface after the resync");

    /* 8b. glass and window region (decision 0244): described in the window's own coordinates, a change sends no pixel and is a change of the description */
    sc = fle_desk_scene();
    CHECK(!(sc->win[0].layer & FLE_SCENE_GLASS) && sc->win[0].cl == 0 && sc->win[0].ct == 0 && sc->win[0].cr == (int32_t)sc->win[0].w && sc->win[0].cb == (int32_t)sc->win[0].h, "no glass; the region box is the whole window");
    gl_on[2] = 1; gl_m[2][0] = -1; gl_m[2][1] = -1; gl_m[2][2] = -1; gl_m[2][3] = -1;
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED) && (sc->win[0].layer & FLE_SCENE_GLASS) && !(sc->win[0].layer & FLE_SCENE_LAYERED) && sc->win[0].gx0 == 0 && sc->win[0].gx1 == 0 && sc->win[0].gy0 == 0 && sc->win[0].gy1 == 0, "a sheet of glass (margins -1): the inner box is empty; no pixel");
    gl_m[2][0] = 10; gl_m[2][1] = 20; gl_m[2][2] = 5; gl_m[2][3] = 6;
    sc = fle_desk_scene();
    CHECK(sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED) && (sc->win[0].layer & FLE_SCENE_GLASS) && sc->win[0].gx0 == 10 && sc->win[0].gy0 == 5 && sc->win[0].gx1 == (int32_t)sc->win[0].w - 20 && sc->win[0].gy1 == (int32_t)sc->win[0].h - 6, "margins: the inner box (%d,%d)-(%d,%d)", sc->win[0].gx0, sc->win[0].gy0, sc->win[0].gx1, sc->win[0].gy1);
    gl_m[2][0] = 900; gl_m[2][1] = 900; gl_m[2][2] = 900; gl_m[2][3] = 900;
    sc = fle_desk_scene();
    CHECK(sc->win[0].gx0 == (int32_t)sc->win[0].w && sc->win[0].gy0 == (int32_t)sc->win[0].h && sc->win[0].gx1 == sc->win[0].gx0 && sc->win[0].gy1 == sc->win[0].gy0, "margins larger than the window: the box is clamped, never inverted");
    gl_on[2] = 0;
    sc = fle_desk_scene();
    CHECK(!(sc->win[0].layer & FLE_SCENE_GLASS) && (sc->flags & FLE_SCENE_FLAG_CHANGED), "the glass is gone");
    rg_on[1] = 1; rg_b[1][0] = 10; rg_b[1][1] = 20; rg_b[1][2] = 100; rg_b[1][3] = 60;
    sc = fle_desk_scene();
    CHECK(sc->win[1].lid == 1 && sc->n_updates == 0 && (sc->flags & FLE_SCENE_FLAG_CHANGED) && sc->win[1].cl == 10 && sc->win[1].ct == 20 && sc->win[1].cr == 100 && sc->win[1].cb == 60, "region box: (%d,%d)-(%d,%d), no pixel", sc->win[1].cl, sc->win[1].ct, sc->win[1].cr, sc->win[1].cb);
    rg_b[1][0] = -50; rg_b[1][1] = -50; rg_b[1][2] = 5000; rg_b[1][3] = 5000;
    sc = fle_desk_scene();
    CHECK(sc->win[1].cl == 0 && sc->win[1].ct == 0 && sc->win[1].cr == (int32_t)sc->win[1].w && sc->win[1].cb == (int32_t)sc->win[1].h, "region box outside the window is clamped to [0,w]x[0,h]");
    rg_b[1][0] = 90; rg_b[1][1] = 40; rg_b[1][2] = 30; rg_b[1][3] = 10;
    sc = fle_desk_scene();
    CHECK(sc->win[1].cl <= sc->win[1].cr && sc->win[1].ct <= sc->win[1].cb, "an inverted region box is never described inverted");
    rg_on[1] = 0;
    sc = fle_desk_scene();
    CHECK(sc->win[1].cl == 0 && sc->win[1].cr == (int32_t)sc->win[1].w, "no region: the whole window again");
    apply(sc, &nu);

    /* 9. random steps: writes of the same pixels, new pixels, a pixel inside a big rectangle, moves, hide/show, restacking; after each the mirror equals every surface */
    mkwin(3, -5, -6, 1930, 1092, 1, 2, 3); mkwin(4, 700, 300, 300, 200, 7, 8, 9); mkwin(5, 0, 0, 100, 30, 1, 1, 1);
    order[0] = 5; order[1] = 4; order[2] = 3; order[3] = 1; order[4] = 2; norder = 5;
    for (unsigned l = 1; l <= 5; l++) W[l].visible = 1;
    sc = fle_desk_scene(); apply(sc, &nu);
    long long bad_total = 0;
    for (int step = 0; step < 2000; step++) {
        unsigned lid = 1 + rnd() % 5; win_t *w = &W[lid];
        uint32_t x0 = rnd() % w->w, y0 = rnd() % w->h, x1 = x0 + 1 + rnd() % 200, y1 = y0 + 1 + rnd() % 120;
        if (x1 > w->w) x1 = w->w;
        if (y1 > w->h) y1 = w->h;
        switch (rnd() % 8) {
        case 0: case 1: fle_desk_wrote(lid, x0, y0, x1, y1); break;
        case 2: case 3: fillrect(lid, x0, y0, x1, y1, (uint8_t)rnd(), (uint8_t)rnd(), (uint8_t)rnd()); fle_desk_wrote(lid, x0, y0, x1, y1); break;
        case 4: setpx(lid, x0, y0, (uint8_t)rnd(), 7, 7, 0); fle_desk_wrote(lid, 0, 0, w->w, w->h); break;
        case 5: w->x += (int32_t)(rnd() % 41) - 20; w->y += (int32_t)(rnd() % 41) - 20; break;
        case 6: w->visible = !w->visible; break;
        case 7: { unsigned a = rnd() % norder, b = rnd() % norder, t = order[a]; order[a] = order[b]; order[b] = t; break; }
        }
        sc = fle_desk_scene();
        if (!check_list(sc, "random step")) { printf("step %d\n", step); break; }
        bad_total += apply(sc, &nu);
    }
    CHECK(bad_total == 0, "the mirror differed from a surface %lld times in 2000 random steps", bad_total);
    printf("checks %d failed %d area %lld\n", checks, failed, area_all);
    return failed != 0;
}
"##;

#[test]
fn the_scene_description_loses_nothing_and_a_move_or_restack_sends_no_pixel() {
    let dir = work_dir();
    let full = std::fs::read_to_string(manifest().join("tests/desk1_compose.rs")).expect("desk1_compose.rs");
    let prog = full.split("const PROG: &str = r##\"").nth(1).expect("PROG").split("\"##;").next().unwrap();
    let prefix = prog.split("int main(void)").next().unwrap();
    let name = etxt_common::uniq("desk-scene");
    let src = dir.join(format!("{name}.c"));
    let exe = dir.join(format!("{name}.out"));
    std::fs::write(&src, format!("{prefix}{MAIN}")).unwrap();
    let cc = Command::new("clang")
        .args(["-O1", "-std=gnu11", "-w"])
        .arg("-I")
        .arg(rt_dir())
        .arg(&src)
        .arg(rt_dir().join("fle_desk.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", &dir)
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&exe);
    assert!(run.status.success(), "the scene test failed:\n{out}");
    assert!(out.contains("failed 0"), "{out}");
    eprintln!("desk scene: {}", out.lines().last().unwrap_or(""));
}
