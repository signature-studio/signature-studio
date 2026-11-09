/* X2 scan, second part (included by b2d_scan.c after b2d_scan_tests.h): the rest of the Agg2D entries. Argument shapes are READINGS of the
 * disassembly of each wrapper (docs/investigations/blend2d-nojit-slots.md section 3); a double goes in the xmm register of its position (xmm1..xmm3,
 * then the stack) and its raw word is also put in the integer register of the same position by cp(). */

TEST(geometry) { /* 62 triangle(6 doubles), 64 roundedRect(x1,y1,x2,y2,r), 65 roundedRect(x1,y1,x2,y2,rx,ry), 68 arc, 72 polygon(xy, n), 73 polyline(xy, n) */
    uint64_t a = P(dbuf);
    fill_noline(0, 0xff, 0xff, 0xff);
    c6(62, D(10), D(50), D(50), D(50), D(30), D(10));
    check_tri("SPEC: triangle(10,50, 50,50, 30,10) filled with the fill colour");
    clear_bg();
    memcpy(dbuf, (double[]){10, 50, 50, 50, 30, 10}, 6 * sizeof(double));
    c2(72, a, 3);
    check_tri("READING: slot 72 is polygon(const double *xy, int n)");
    clear_bg();
    c0(28);
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c2(73, a, 3);
    check_px("polyline-on-edge", 30, 50, 0xff, 0xff, 0xff, 0xff, "READING: slot 73 is polyline(const double *xy, int n), stroked with the line colour (an open path: the base of the triangle is its first segment)");
}
TEST(roundrect) {
    fill_noline(0xff, 0, 0, 0xff);
    c5(64, D(8), D(8), D(56), D(56), D(12));
    check_px("rr-centre", 32, 32, 0x00, 0x00, 0xff, 0xff, "SPEC");
    check_bg("rr-corner-cut", 8, 8, "SPEC: a rounded rectangle of radius 12 leaves the pixel at its exact corner uncovered");
    check_px("rr-edge-mid", 32, 9, 0x00, 0x00, 0xff, 0xff, "SPEC");
    {
        unsigned n = count_px(0x00, 0x00, 0xff, 0xff);
        double want = 48.0 * 48.0 - (4.0 - PI) * 12 * 12;
        check("rr-area", n > 0.97 * want && n < 1.03 * want, "SPEC", "fully covered pixels %u, area of the shape %.1f (within 3%%; edge pixels are partial)", n, want);
    }
}
TEST(roundrect2) { /* 65 with rx != ry, 66 with 8 doubles (a reading: x1,y1,x2,y2, rx_bottom,ry_bottom, rx_top,ry_top) */
    fill_noline(0xff, 0, 0, 0xff);
    c6(65, D(8), D(8), D(56), D(56), D(12), D(6));
    check_px("rr2-centre", 32, 32, 0x00, 0x00, 0xff, 0xff, "SPEC");
    check_bg("rr2-corner", 8, 8, "SPEC");
}
TEST(roundrect3) {
    fill_noline(0xff, 0, 0, 0xff);
    c8(66, D(8), D(8), D(56), D(56), D(12), D(12), D(4), D(4));
    check_px("rr3-centre", 32, 32, 0x00, 0x00, 0xff, 0xff, "SPEC");
}
TEST(arc) { /* 68: (cx, cy, rx, ry, start, sweep) */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c6(68, D(32), D(32), D(20), D(20), D(0), D(PI));
    value("changed-pixels", changed());
    check("arc-changed", changed() > 0, "READING", "slot 68 is arc(cx,cy,rx,ry,start,sweep) in radians; it draws something (%u pixels changed)", changed());
}
TEST(star) { /* 69: star(cx, cy, r1, r2, startAngle, numRays) */
    fill_noline(0xff, 0xff, 0xff, 0xff);
    c7(69, D(32), D(32), D(8), D(24), D(0), 5, 0);
    value("changed-pixels", changed());
}
TEST(ellipse2) { /* slot 67 with no fill and a line colour: Agg2D's convention is that the outline is drawn */
    colour(30, 0xff, 0xff, 0, 0xff);
    c1(42, D(2.0));
    c0(28);
    c4(67, D(32), D(32), D(20), D(12));
    value("changed-pixels", changed());
    check("ellipse-stroke-drawn", changed() > 0, "READING: the Agg2D convention (an ellipse is filled and/or stroked by the current attributes); not verified against this wrapper", "changed pixels %u", changed());
    { const uint8_t *e = px(52, 32); check("ellipse-stroke-right", e[1] > 0xe8 && e[2] > 0xe8 && e[0] < 0x18, "READING with a threshold chosen AFTER the first run (which expected exactly ff ff 00 and got 03 f4 f3): the outline is a 2-wide stroke centred on x=52 at the right end of the ellipse; the pixel there is expected mostly covered, not a derived value", "pixel(52,32) is %02x %02x %02x", e[0], e[1], e[2]); }
}
TEST(curve) { /* 70 quadricCurve(x1,y1, cx,cy, x2,y2), 71 cubicCurve(8 doubles), stroked */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c6(70, D(10), D(50), D(30), D(10), D(50), D(50));
    check_px("quad-mid", 30, 30, 0xff, 0xff, 0xff, 0xff, "SPEC: the point of a quadratic Bezier at t=0.5 is (P0+2*P1+P2)/4 = (30,30)");
    check("quad-start-covered", !is_px(10, 50, BG_B, BG_G, BG_R, 0xff), "SPEC: a stroke that starts at (10,50) touches that pixel (a butt cap is cut square, so the end pixel is partly covered: the first run expected a full pixel)", "pixel(10,50) is %02x %02x %02x", px(10, 50)[0], px(10, 50)[1], px(10, 50)[2]);
    clear_bg();
    c8(71, D(10), D(50), D(10), D(10), D(50), D(10), D(50), D(50));
    check_px("cubic-mid", 30, 20, 0xff, 0xff, 0xff, 0xff, "SPEC: the point of a cubic Bezier at t=0.5 is (P0+3*P1+3*P2+P3)/8 = (30,20)");
}
TEST(pathcurve) { /* 93 quadTo, 97 cubicTo on the current path, 83 moveTo, 103 drawPath(1) */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c0(82);
    c2(83, D(10), D(50));
    c4(93, D(30), D(10), D(50), D(50));
    c1(103, 1);
    check_px("pathquad-mid", 30, 30, 0xff, 0xff, 0xff, 0xff, "SPEC: moveTo(10,50) + quadTo(ctrl 30,10; end 50,50) passes through (30,30)");
    clear_bg();
    c0(82);
    c2(83, D(10), D(50));
    c6(97, D(10), D(10), D(50), D(10), D(50), D(50));
    c1(103, 1);
    check_px("pathcubic-mid", 30, 20, 0xff, 0xff, 0xff, 0xff, "SPEC: moveTo(10,50) + cubicTo(10,10; 50,10; 50,50) passes through (30,20)");
}
TEST(pathrel) { /* 84 moveRel, 86 lineRel, 88 horLineRel, 90 verLineRel, 87/89 horLineTo/verLineTo */
    fill_noline(0, 0xff, 0xff, 0xff);
    c0(82);
    c2(83, D(10), D(10));
    c1(88, D(40)); /* to (50,10) */
    c1(90, D(40)); /* to (50,50) */
    c2(86, D(-40), D(0)); /* to (10,50) */
    c0(102);
    c1(103, 0);
    check("pathrel-square", count_px(0xff, 0xff, 0x00, 0xff) == 40 * 40, "SPEC", "cyan pixels %u, the square (10,10)-(50,50) holds 1600", count_px(0xff, 0xff, 0x00, 0xff));
    clear_bg();
    c0(82);
    c2(84, D(10), D(10));
    c1(87, D(50));
    c1(89, D(50));
    c1(87, D(10));
    c0(102);
    c1(103, 0);
    check("pathabs-square", count_px(0xff, 0xff, 0x00, 0xff) == 40 * 40, "READING", "slot 84 moveRel from the empty path is moveTo(10,10), 87 horLineTo, 89 verLineTo: cyan pixels %u (1600 expected)", count_px(0xff, 0xff, 0x00, 0xff));
}
TEST(pathsmooth) { /* 94 quadRel, 95 quad smooth, 96 smooth rel, 98 cubicRel, 99 cubic smooth, 100 smooth rel */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c0(82);
    c2(83, D(5), D(40));
    c4(94, D(10), D(-20), D(20), D(0));
    c2(96, D(20), D(0));
    c6(98, D(0), D(-10), D(10), D(-10), D(10), D(0));
    c4(99, D(10), D(10), D(10), D(0));
    c4(100, D(5), D(5), D(5), D(0));
    c2(95, D(60), D(40));
    c1(103, 1);
    value("changed-pixels", changed());
}
TEST(pathellipse) { /* 91 arcTo, 92 arcRel, 101 addEllipse */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c0(82);
    c2(83, D(10), D(32));
    {
        uint64_t s[4] = {D(54), D(32), 0, 0};
        (void)s;
        c7(91, D(22), D(22), D(0), 0, 1, D(54), D(32));
    }
    c1(103, 1);
    value("changed-pixels-arcTo", changed());
    clear_bg();
    c0(82);
    c5(101, D(32), D(32), D(20), D(12), 0);
    fill_noline(0, 0xff, 0xff, 0xff);
    c1(103, 0);
    value("changed-pixels-addEllipse", changed());
}
TEST(pointinpath) { /* 126: pointInPath(int x, int y) on the current path */
    fill_noline(0, 0xff, 0xff, 0xff);
    tri_path();
    c0(102);
    value("inside-30-40", (long long)(c2(126, 30, 40) & 0xff));
    value("outside-5-5", (long long)(c2(126, 5, 5) & 0xff));
    check("pip-inside", (c2(126, 30, 40) & 0xff) == 1, "SPEC: (30,40) is inside the triangle (10,50)(50,50)(30,10); the wrapper returns a bool", "returned %u", (unsigned)(c2(126, 30, 40) & 0xff));
    check("pip-outside", (c2(126, 5, 5) & 0xff) == 0, "SPEC", "returned %u", (unsigned)(c2(126, 5, 5) & 0xff));
}
TEST(transform) { /* 52 reset, 58 translate, 56 scale, 55 rotate, 57 skew (a reading of the op codes), 9 worldToScreen(&x,&y) */
    double x, y;
    c0(52);
    c2(58, D(10), D(20));
    x = 1;
    y = 2;
    memcpy(dbuf, (double[]){1, 2}, 2 * sizeof(double));
    c2(9, P(&dbuf[0]), P(&dbuf[1]));
    x = dbuf[0];
    y = dbuf[1];
    dvalue("translate-x", x);
    dvalue("translate-y", y);
    check("translate", x == 11 && y == 22, "SPEC: translate(10,20) moves the point (1,2) to (11,22)", "got (%g,%g)", x, y);
    c0(52);
    c2(56, D(2), D(3));
    memcpy(dbuf, (double[]){5, 7}, 2 * sizeof(double));
    c2(9, P(&dbuf[0]), P(&dbuf[1]));
    check("scale", dbuf[0] == 10 && dbuf[1] == 21, "SPEC: scale(2,3) moves (5,7) to (10,21)", "got (%g,%g)", dbuf[0], dbuf[1]);
    c0(52);
    c1(55, D(PI / 2));
    memcpy(dbuf, (double[]){10, 0}, 2 * sizeof(double));
    c2(9, P(&dbuf[0]), P(&dbuf[1]));
    {
        double dx = dbuf[0] - 0, dy = dbuf[1] - 10;
        check("rotate", dx * dx + dy * dy < 1e-9, "SPEC: a rotation by pi/2 (radians, counter-clockwise in a y-up plane) moves (10,0) to (0,10)", "got (%.9g,%.9g)", dbuf[0], dbuf[1]);
    }
    c0(52);
    memcpy(dbuf, (double[]){3, 4}, 2 * sizeof(double));
    c2(9, P(&dbuf[0]), P(&dbuf[1]));
    check("reset", dbuf[0] == 3 && dbuf[1] == 4, "SPEC: after reset the point is unchanged", "got (%g,%g)", dbuf[0], dbuf[1]);
}
TEST(transdraw) { /* a transform changes where the next shape lands */
    c0(52);
    c2(58, D(20), D(10));
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 10, 10);
    check_px("translated-in", 25, 15, 0x00, 0x00, 0xff, 0xff, "SPEC: a 10x10 box at the origin after translate(20,10) covers x 20..30, y 10..20");
    check_bg("translated-origin", 5, 5, "SPEC");
    clear_bg();
    c0(52);
    c2(56, D(2), D(2));
    rect(5, 5, 15, 15);
    check("scaled-count", count_px(0x00, 0x00, 0xff, 0xff) == 20 * 20, "SPEC", "red pixels %u, scale(2,2) of a 10x10 box is 20x20 = 400", count_px(0x00, 0x00, 0xff, 0xff));
    check_px("scaled-in", 25, 25, 0x00, 0x00, 0xff, 0xff, "SPEC: x 10..30, y 10..30");
    clear_bg();
    c0(52);
    c1(55, D(PI / 2));
    c2(58, D(40), D(0));
    rect(0, 0, 10, 20);
    value("rot-changed", changed());
}
TEST(scalar) { /* 11 worldToScreen(scalar), 12 screenToWorld(scalar), 13 alignPoint(&x,&y), 10 screenToWorld(&x,&y) */
    c0(52);
    c1(11, D(7));
    dvalue("world-to-screen-scalar", ret_d());
    check("w2s-scalar", ret_d() > 6.999999 && ret_d() < 7.000001, "READING: slot 11 returns scalar*scale; with the identity matrix 7", "returned %.9g", ret_d());
    c1(12, D(7));
    dvalue("screen-to-world-scalar", ret_d());
    check("s2w-scalar", ret_d() > 6.999999 && ret_d() < 7.000001, "READING", "returned %.9g", ret_d());
    c0(52);
    c2(58, D(3), D(4));
    memcpy(dbuf, (double[]){13, 14}, 2 * sizeof(double));
    c2(10, P(&dbuf[0]), P(&dbuf[1]));
    check("s2w-point", dbuf[0] == 10 && dbuf[1] == 10, "SPEC: the inverse of translate(3,4) maps (13,14) to (10,10)", "got (%g,%g)", dbuf[0], dbuf[1]);
    memcpy(dbuf, (double[]){10.3, 20.7}, 2 * sizeof(double));
    c2(13, P(&dbuf[0]), P(&dbuf[1]));
    dvalue("align-x", dbuf[0]);
    dvalue("align-y", dbuf[1]);
}
TEST(linecaps) { /* 44/45 lineCap set/get, 46/47 lineJoin set/get */
    unsigned v;
    int ok = 1;
    for (v = 0; v < 3; v++) {
        uint32_t out = 0xdeadbeef;
        c1(44, v);
        c1(45, P(&out));
        value("cap-set-get", (long long)out);
        ok = ok && out == v;
    }
    check("cap-roundtrip", ok, "READING: slot 45 returns what slot 44 stored (0, 1, 2)", "all equal: %d", ok);
    ok = 1;
    for (v = 0; v < 5; v++) {
        uint32_t out = 0xdeadbeef;
        c1(46, v);
        c1(47, P(&out));
        value("join-set-get", (long long)out);
        ok = ok && out == v;
    }
    check("join-roundtrip", ok, "READING: slot 47 returns what slot 46 stored (0..4)", "all equal: %d", ok);
}
TEST(thickline) { /* line caps change the end of a thick line */
    unsigned cap;
    for (cap = 0; cap < 3; cap++) {
        clear_bg();
        colour(30, 0xff, 0xff, 0xff, 0xff);
        c1(44, cap);
        c1(42, D(10.0));
        c4(61, D(20), D(32), D(44), D(32));
        printf("scan: VALUE %s cap%u-px-left-of-start %u\n", cfg_full, cap, is_px(17, 32, 0xff, 0xff, 0xff, 0xff) ? 1u : 0u);
        value("cap-changed", changed());
    }
}
TEST(blendmode) { /* 15 blendMode(n) then a half-transparent blue rectangle over the background; FLE_B2D_SCAN=blendmode:N, N = 0..28 */
    uint64_t r;
    (void)r;
    clear_bg();
    c1(15, (unsigned)targ);
    fill_noline(0, 0, 0xff, 0x80);
    rect(8, 8, 56, 56);
    value("mode", targ);
    printf("scan: VALUE %s pixel-32-32 %02x %02x %02x %02x\n", cfg_full, px(32, 32)[0], px(32, 32)[1], px(32, 32)[2], px(32, 32)[3]);
    printf("scan: VALUE %s pixel-2-2 %02x %02x %02x %02x\n", cfg_full, px(2, 2)[0], px(2, 2)[1], px(2, 2)[2], px(2, 2)[3]);
}
TEST(misc) { /* 22 (calls the context vtable +0x100 with no argument), 23 (returns the double at impl+0x28), 25 (returns 0.0), 48/49 set/get a bool at impl+0x21 */
    uint32_t b = 0xdeadbeef;
    (void)b;
    c0(23);
    dvalue("slot23-initial", ret_d());
    c0(25);
    dvalue("slot25", ret_d());
    c1(48, 0);
    value("slot49-after-48-0", (long long)(c0(49) & 0xff));
    c1(48, 1);
    value("slot49-after-48-1", (long long)(c0(49) & 0xff));
    check("bool-roundtrip", (c0(49) & 0xff) == 1, "READING: slot 49 returns what slot 48 stored", "returned %u", (unsigned)(c0(49) & 0xff));
    c0(22);
    value("slot22-done", 1);
}
TEST(gradient) { /* 120 returns the colour ramp of the fill (256 entries of R,G,B,A int16, 15-bit); 121 linearGradient(x1,y1,x2,y2, 1 = fill); then a rectangle. This is how FLEngine uses it (f_6cb3d0). */
    uint64_t ramp;
    unsigned i;
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, (255 - t) * 32767 / 255); /* R */
        fle_st(ramp + 8 * i + 2, 2, 0);                       /* G */
        fle_st(ramp + 8 * i + 4, 2, t * 32767 / 255);         /* B */
        fle_st(ramp + 8 * i + 6, 2, 32767);                   /* A */
    }
    c5(121, D(0), D(0), D(64), D(0), 1);
    rect(0, 0, 64, 64);
    {
        const uint8_t *l = px(2, 32), *m = px(32, 32), *r = px(61, 32);
        printf("scan: VALUE %s left %02x %02x %02x %02x mid %02x %02x %02x %02x right %02x %02x %02x %02x (B G R A)\n", cfg_full, l[0], l[1], l[2], l[3], m[0], m[1], m[2], m[3], r[0], r[1], r[2], r[3]);
        check("grad-left-red", l[2] > 230 && l[0] < 25 && l[3] == 0xff, "SPEC: a linear gradient red -> blue over x 0..64 is almost red at x=2", "pixel (2,32) B G R A = %02x %02x %02x %02x", l[0], l[1], l[2], l[3]);
        check("grad-right-blue", r[0] > 230 && r[2] < 25 && r[3] == 0xff, "SPEC: almost blue at x=61", "pixel (61,32) B G R A = %02x %02x %02x %02x", r[0], r[1], r[2], r[3]);
        check("grad-mid", m[0] > 118 && m[0] < 138 && m[2] > 118 && m[2] < 138, "SPEC: the middle is about half red, half blue", "pixel (32,32) B G R = %02x %02x %02x", m[0], m[1], m[2]);
    }
    { unsigned x, c, md = 0; for (x = 0; x < 64; x++) for (c = 0; c < 3; c++) { unsigned a = px(x, 5)[c], b = px(x, 50)[c], d = a > b ? a - b : b - a; if (d > md) md = d; } value("grad-row5-vs-row50-largest-channel-difference", md); check("grad-rows-within-1", md <= 1, "SPEC: a horizontal gradient is the same on every row up to the 8-bit rounding (the first run expected 0; the measured difference is 1 and its cause is not determined)", "largest difference %u", md); }
}
TEST(gradvert) {
    uint64_t ramp;
    unsigned i;
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, 32767);
        fle_st(ramp + 8 * i + 2, 2, t * 32767 / 255);
        fle_st(ramp + 8 * i + 4, 2, 0);
        fle_st(ramp + 8 * i + 6, 2, 32767);
    }
    c5(121, D(0), D(8), D(0), D(56), 1);
    rect(0, 0, 64, 64);
    {
        const uint8_t *t = px(32, 4), *m = px(32, 32), *b = px(32, 60);
        printf("scan: VALUE %s top %02x %02x %02x %02x mid %02x %02x %02x %02x bottom %02x %02x %02x %02x (B G R A)\n", cfg_full, t[0], t[1], t[2], t[3], m[0], m[1], m[2], m[3], b[0], b[1], b[2], b[3]);
        check("gv-top", t[1] < 8 && t[2] > 247, "SPEC: a vertical gradient (255,0,0) -> (255,255,0) from y=8 to y=56 is the first colour above y=8 (pad)", "pixel (32,4) B G R = %02x %02x %02x", t[0], t[1], t[2]);
        check("gv-bottom", b[1] > 247 && b[2] > 247, "SPEC: the last colour below y=56", "pixel (32,60) B G R = %02x %02x %02x", b[0], b[1], b[2]);
        check("gv-mid", m[1] > 118 && m[1] < 138, "SPEC: green is about half in the middle", "pixel (32,32) G = %02x", m[1]);
    }
}
TEST(gradradial) { /* 122 radialGradient(x, y, r, flag) */
    uint64_t ramp;
    unsigned i;
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, 32767);
        fle_st(ramp + 8 * i + 2, 2, 32767);
        fle_st(ramp + 8 * i + 4, 2, 32767);
        fle_st(ramp + 8 * i + 6, 2, (255 - t) * 32767 / 255);
    }
    c4(122, D(32), D(32), D(30), 1);
    rect(0, 0, 64, 64);
    value("changed-pixels", changed());
    printf("scan: VALUE %s centre %02x %02x %02x %02x edge %02x %02x %02x %02x\n", cfg_full, px(32, 32)[0], px(32, 32)[1], px(32, 32)[2], px(32, 32)[3], px(2, 2)[0], px(2, 2)[1], px(2, 2)[2], px(2, 2)[3]);
}
TEST(gradrot) { /* 123: this, xmm1, xmm2, xmm3, then a double that is the argument of sin and cos, a double, an int flag (1 = fill) */
    uint64_t ramp;
    unsigned i;
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, (255 - t) * 32767 / 255);
        fle_st(ramp + 8 * i + 2, 2, t * 32767 / 255);
        fle_st(ramp + 8 * i + 4, 2, 0);
        fle_st(ramp + 8 * i + 6, 2, 32767);
    }
    c6(123, D(32), D(32), D(40), D(0.5), D(1.0), 1);
    rect(0, 0, 64, 64);
    value("changed-pixels", changed());
}
TEST(gradline) { /* a gradient on the stroke: 121 with the last argument not 1, then a thick line */
    uint64_t ramp;
    unsigned i;
    ramp = c1(120, 0);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, 32767);
        fle_st(ramp + 8 * i + 2, 2, t * 32767 / 255);
        fle_st(ramp + 8 * i + 4, 2, 0);
        fle_st(ramp + 8 * i + 6, 2, 32767);
    }
    c5(121, D(0), D(0), D(64), D(0), 0);
    c1(42, D(6.0));
    c4(61, D(4), D(32), D(60), D(32));
    value("changed-pixels", changed());
    printf("scan: VALUE %s left %02x %02x %02x %02x right %02x %02x %02x %02x\n", cfg_full, px(8, 32)[0], px(8, 32)[1], px(8, 32)[2], px(8, 32)[3], px(56, 32)[0], px(56, 32)[1], px(56, 32)[2], px(56, 32)[3]);
}
TEST(clipint) { /* 124: clipBox with ints (a reading) */
    clear_bg();
    c4(124, 16, 16, 48, 48);
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 64, 64);
    check("clipint-count", count_px(0x00, 0x00, 0xff, 0xff) == 32 * 32, "SPEC: a fill of the whole buffer is cut to the clip box", "red pixels %u, a 32x32 clip box holds 1024", count_px(0x00, 0x00, 0xff, 0xff));
}
TEST(clipreset) { /* a clip box and then a larger one */
    clear_bg();
    c4(3, D(16), D(16), D(48), D(48));
    c4(3, D(0), D(0), D(64), D(64));
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 64, 64);
    value("red-after-16-48-then-0-64", count_px(0x00, 0x00, 0xff, 0xff)); /* 1024 means the second clipBox intersected (the first run expected 4096, "replaces": an assumption) */
}
TEST(alphafill) { /* a translucent colour: source-over of (255,0,0,128) over 33 22 11 */
    fill_noline(0xff, 0, 0, 0x80);
    rect(8, 8, 56, 56);
    check_px("a128-red", 32, 32, 0x19, 0x11, 0x88, 0xff, "SPEC: source-over of (r=255,g=0,b=0,a=128) on B,G,R = 0x33,0x22,0x11: premultiplied source R=128; out = src + bg*(255-128)/255 = B 25, G 17, R 136 (the first run expected B=0x1a: a slip in the hand calculation, corrected from the arithmetic, not from the output)");
    {
        const uint8_t *p = px(32, 32);
        int dr = (int)p[2] - 0x88, dg = (int)p[1] - 0x11, db = (int)p[0] - 0x19;
        check("a128-red-within-1", dr >= -1 && dr <= 1 && dg >= -1 && dg <= 1 && db >= -1 && db <= 1, "SPEC", "B G R = %02x %02x %02x, expected 19 11 88 +-1", p[0], p[1], p[2]);
    }
}
TEST(reset) { /* 128: the defaults of the constructor again; then 8 clear and a fill */
    fill_noline(0, 0xff, 0, 0xff);
    c0(128);
    rect(8, 8, 40, 40);
    value("changed-pixels-after-128", changed());
}
TEST(slot129) { /* a reading: 129(a, b): stores a and b - a at +0x60/+0x68 and calls 42 */
    c2(129, D(2.0), D(6.0));
    value("slot129-done", 1);
    dvalue("slot43-after", (c0(43), ret_d()));
}
TEST(slots5960) {
    c0(59);
    c0(60);
    value("slot59-60-done", 1);
}
TEST(dtor) { /* 0: the destructor (delete this) */
    c0(0);
    value("dtor-done", 1);
}
TEST(graddump) { /* the row-difference of the horizontal gradient of `gradient`: prints rows 5 and 50 (x = 0..63) as B G R words; a tool of the investigation */
    uint64_t ramp;
    unsigned i, x, maxd = 0, ndiff = 0;
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, (255 - t) * 32767 / 255);
        fle_st(ramp + 8 * i + 2, 2, 0);
        fle_st(ramp + 8 * i + 4, 2, t * 32767 / 255);
        fle_st(ramp + 8 * i + 6, 2, 32767);
    }
    c5(121, D(0), D(0), D(64), D(0), 1);
    c0(31);
    rect(0, 0, 64, 64);
    for (x = 0; x < 64; x++) {
        const uint8_t *a = px(x, 5), *b = px(x, 50);
        unsigned c;
        for (c = 0; c < 3; c++) {
            unsigned d = a[c] > b[c] ? a[c] - b[c] : b[c] - a[c];
            if (d) ndiff++;
            if (d > maxd) maxd = d;
        }
    }
    value("channels-differing-between-row5-and-row50", ndiff);
    value("largest-channel-difference", maxd);
    for (x = 0; x < 64; x += 9) printf("scan: VALUE %s x=%u row5 %02x %02x %02x row50 %02x %02x %02x row20 %02x %02x %02x\n", cfg_full, x, px(x, 5)[0], px(x, 5)[1], px(x, 5)[2], px(x, 50)[0], px(x, 50)[1], px(x, 50)[2], px(x, 20)[0], px(x, 20)[1], px(x, 20)[2]);
}
TEST(clipdump) { /* clipBox twice: which rectangle is clipped afterwards (the count of red pixels for the sequence 16..48 then 0..64, and then 8..56 then 16..48) */
    clear_bg();
    c4(3, D(16), D(16), D(48), D(48));
    c4(3, D(8), D(8), D(56), D(56));
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 64, 64);
    value("red-after-16-48-then-8-56", count_px(0x00, 0x00, 0xff, 0xff));
    clear_bg();
    c4(3, D(0), D(0), D(64), D(64));
    c4(3, D(16), D(16), D(48), D(48));
    rect(0, 0, 64, 64);
    value("red-after-0-64-then-16-48", count_px(0x00, 0x00, 0xff, 0xff));
}
TEST(clearall6) { /* slot 6 tail-jumps to slot 8 (reading of the 4-instruction wrapper: it copies the 5th argument byte and jumps through vtable+0x40) */
    colour(6, 0x44, 0x55, 0x66, 0xff);
    check_px("slot6-clears", 20, 20, 0x66, 0x55, 0x44, 0xff, "READING: slot 6 is the same entry as slot 8 (clearAll)");
}
TEST(skew) { /* slot 57: a 2-double matrix operation (reading: the context's matrix op 10 of the table of op codes, applied after the current matrix) */
    c0(52);
    c2(57, D(0.5), D(0));
    memcpy(dbuf, (double[]){10, 20}, 2 * sizeof(double));
    c2(9, P(&dbuf[0]), P(&dbuf[1]));
    dvalue("skew-x-of-10-20", dbuf[0]);
    dvalue("skew-y-of-10-20", dbuf[1]);
    fill_noline(0xff, 0, 0, 0xff);
    c0(52);
    c2(57, D(0.5), D(0));
    rect(10, 10, 30, 30);
    value("changed-pixels", changed());
}
TEST(arcrel) { /* slot 92 arcRel: the shape of slot 91 (arcTo) with a relative end point */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    c0(82);
    c2(83, D(10), D(32));
    c7(92, D(22), D(22), D(0), 0, 1, D(44), D(0));
    c1(103, 1);
    value("changed-pixels", changed());
}

/* ---- other buffer sizes and re-attach (FLEngine attaches again on every draw: f_6cb020) ---- */
static uint8_t *bigbuf;
static unsigned bw, bh, bstride;
static const uint8_t *bpx(unsigned x, unsigned y) { return bigbuf + (size_t)y * bstride + (size_t)x * 4; }
static int is_big(unsigned x, unsigned y, unsigned b, unsigned g, unsigned r) { const uint8_t *p = bpx(x, y); return p[0] == b && p[1] == g && p[2] == r && p[3] == 0xff; }
static unsigned big_count(unsigned b, unsigned g, unsigned r) {
    unsigned n = 0, x, y;
    for (y = 0; y < bh; y++)
        for (x = 0; x < bw; x++) n += bpx(x, y)[0] == b && bpx(x, y)[1] == g && bpx(x, y)[2] == r && bpx(x, y)[3] == 0xff;
    return n;
}
static void attach_big(unsigned w, unsigned h, unsigned stride) {
    unsigned x, y;
    uint64_t s[1] = {stride};
    bw = w;
    bh = h;
    bstride = stride;
    bigbuf = aligned_alloc(64, ((size_t)stride * h + 63) & ~(size_t)63); /* C11: the size must be a multiple of the alignment (emscripten returns NULL otherwise; glibc does not) */
    if (!bigbuf) {
        printf("scan: RESULT %s status=no-memory\n", cfg_full);
        exit(2);
    }
    for (y = 0; y < h; y++)
        for (x = 0; x < w; x++) {
            uint8_t *p = bigbuf + (size_t)y * stride + (size_t)x * 4;
            p[0] = BG_B, p[1] = BG_G, p[2] = BG_R, p[3] = 0xff;
        }
    printf("scan: CALL 1\n");
    gcall(slotfn(1), obj, P(bigbuf), w, h, 1, s);
}
static void big_scene(unsigned w, unsigned h) { /* a rectangle, an ellipse, a triangle, a rounded rectangle, a thick line and a gradient, in a w x h buffer */
    double fw = w, fh = h;
    uint64_t ramp;
    unsigned i;
    fill_noline(0xff, 0, 0, 0xff);
    rect(fw * 0.05, fh * 0.05, fw * 0.45, fh * 0.45);
    fill_noline(0xff, 0xff, 0, 0xff);
    c4(67, D(fw * 0.75), D(fh * 0.25), D(fw * 0.2), D(fh * 0.15));
    fill_noline(0, 0xff, 0xff, 0xff);
    c0(82);
    c2(83, D(fw * 0.05), D(fh * 0.95));
    c2(85, D(fw * 0.45), D(fh * 0.95));
    c2(85, D(fw * 0.25), D(fh * 0.55));
    c0(102);
    c1(103, 0);
    fill_noline(0xff, 0, 0xff, 0xff);
    c5(64, D(fw * 0.55), D(fh * 0.55), D(fw * 0.95), D(fh * 0.95), D(fh * 0.1));
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(3.0));
    c4(61, D(0), D(fh * 0.5), D(fw), D(fh * 0.5));
    ramp = c1(120, 1);
    for (i = 0; i < 256; i++) {
        uint64_t t = i;
        fle_st(ramp + 8 * i + 0, 2, (255 - t) * 32767 / 255);
        fle_st(ramp + 8 * i + 2, 2, t * 32767 / 255);
        fle_st(ramp + 8 * i + 4, 2, 0);
        fle_st(ramp + 8 * i + 6, 2, 32767);
    }
    c5(121, D(fw * 0.5), D(0), D(fw * 0.55), D(0), 1);
    c0(31);
    rect(fw * 0.5, fh * 0.02, fw * 0.55, fh * 0.04);
}
static void big_report(const char *tag) {
    printf("scan: VALUE %s %s-hash %#llx\n", cfg_full, tag, ULL(fnv(bigbuf, (size_t)bstride * bh)));
    printf("scan: VALUE %s %s-non-background-pixels %u\n", cfg_full, tag, bw * bh - big_count(BG_B, BG_G, BG_R));
    printf("scan: VALUE %s %s-red-pixels %u\n", cfg_full, tag, big_count(0x00, 0x00, 0xff));
    printf("scan: VALUE %s %s-cyan-pixels %u\n", cfg_full, tag, big_count(0xff, 0xff, 0x00));
}
TEST(big640) {
    attach_big(640, 360, 640 * 4);
    big_scene(640, 360);
    big_report("640x360");
    check("big-rect-area", big_count(0x00, 0x00, 0xff) > 0.97 * (0.4 * 640) * (0.4 * 360), "SPEC: the rectangle (0.05..0.45 of the buffer) is fully red: its area in whole pixels is within 3%% of w*h*0.16", "red pixels %u of about %.0f", big_count(0x00, 0x00, 0xff), 0.16 * 640 * 360);
    check("big-rect-inside", is_big(100, 100, 0x00, 0x00, 0xff), "SPEC", "pixel(100,100) is %02x %02x %02x", bpx(100, 100)[0], bpx(100, 100)[1], bpx(100, 100)[2]);
    check("big-corner-bg", is_big(639, 0, BG_B, BG_G, BG_R), "SPEC", "pixel(639,0) is %02x %02x %02x", bpx(639, 0)[0], bpx(639, 0)[1], bpx(639, 0)[2]);
}
TEST(bigodd) { /* a width that is no multiple of 4 or 8, and a stride with padding */
    attach_big(333, 77, 333 * 4 + 12);
    big_scene(333, 77);
    big_report("333x77");
    check("bigodd-rect-inside", is_big(40, 15, 0x00, 0x00, 0xff), "SPEC", "pixel(40,15) is %02x %02x %02x", bpx(40, 15)[0], bpx(40, 15)[1], bpx(40, 15)[2]);
}
TEST(bigtall) { /* more rows than columns */
    attach_big(48, 900, 48 * 4);
    big_scene(48, 900);
    big_report("48x900");
}
TEST(tiny) { /* 3x3 and 1x1 buffers */
    attach_big(3, 3, 12);
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 3, 3);
    big_report("3x3");
    check("tiny-3x3-all-red", big_count(0x00, 0x00, 0xff) == 9, "SPEC: a rectangle that covers the whole 3x3 buffer", "red pixels %u of 9", big_count(0x00, 0x00, 0xff));
    attach_big(1, 1, 4);
    rect(-5, -5, 5, 5);
    big_report("1x1");
    check("tiny-1x1-red", big_count(0x00, 0x00, 0xff) == 1, "SPEC", "red pixels %u of 1", big_count(0x00, 0x00, 0xff));
}
TEST(reattach) { /* attach again and again, with other buffers: the context of the old one is released */
    unsigned k;
    for (k = 0; k < 6; k++) {
        unsigned w = 40 + 20 * k, h = 30 + 10 * k;
        attach_big(w, h, w * 4);
        fill_noline(0xff, 0, 0, 0xff);
        rect(2, 2, w - 2, h - 2);
        printf("scan: VALUE %s attach%u-%ux%u red %u expect %u\n", cfg_full, k, w, h, big_count(0x00, 0x00, 0xff), (w - 4) * (h - 4));
        check("reattach-red", big_count(0x00, 0x00, 0xff) == (w - 4) * (h - 4), "SPEC: a rectangle with integer corners 2 pixels inside the buffer covers (w-4)*(h-4) pixels", "attach %u (%ux%u): red pixels %u", k, w, h, big_count(0x00, 0x00, 0xff));
    }
    check("reattach-first-buffer-unchanged", changed() == 0, "SPEC: drawing after the second attach does not touch the buffer of the first", "changed pixels in the first buffer %u", changed());
}

/* ---- drawPath with every flag of its jump table (0..5), and a deterministic random mix (FLE_B2D_SCAN=fuzz:<seed>) ---- */
TEST(drawflags) {
    unsigned f;
    for (f = 0; f < 8; f++) {
        clear_bg();
        colour(27, 0, 0xff, 0xff, 0xff);
        colour(30, 0xff, 0, 0, 0xff);
        c1(42, D(3.0));
        tri_path();
        c0(102);
        c1(103, f);
        printf("scan: VALUE %s flag%u-hash %#llx changed %u\n", cfg_full, f, ULL(fnv(pix, NBYTES)), changed());
    }
}

static uint64_t rs;
static uint32_t rnd(void) {
    rs = rs * UINT64_C(6364136223846793005) + UINT64_C(1442695040888963407);
    return (uint32_t)(rs >> 33);
}
static double rc(int lo, int hi) { return lo + (double)(rnd() % (unsigned)((hi - lo) * 8 + 1)) / 8.0; } /* multiples of 1/8 in [lo, hi]: exact in binary */
static unsigned rb(void) { return rnd() & 0xff; }
static const char *const op_name[] = {"line", "tri", "rect", "rrect", "rrect2", "rrect3", "ellipse", "arc", "star", "quad", "cubic", "polygon", "polyline", "path", "pathrel", "patharc", "pathsmooth", "xform", "clip", "grad", "mode", "reset"};
TEST(fuzz) {
    unsigned it, k;
    uint64_t ramp;
    rs = UINT64_C(0x9e3779b97f4a7c15) * (uint64_t)(targ < 0 ? 0 : targ + 1);
    for (it = 0; it < 150; it++) {
        unsigned op = rnd() % 22, m;
        printf("scan: ITER %u %s\n", it, op_name[op]);
        fflush(stdout);
        if (rnd() % 3) colour(27, rb(), rb(), rb(), rnd() % 4 ? 0xff : rb()); else c0(28);
        if (rnd() % 2) colour(30, rb(), rb(), rb(), rnd() % 4 ? 0xff : rb()); else c0(31);
        {
            static const double lw[] = {0.0, 0.5, 1.0, 2.5, 7.0, 20.0};
            c1(42, D(lw[rnd() % 6]));
        }
        c1(44, rnd() % 3);
        c1(46, rnd() % 5);
        if (rnd() % 5 == 0) c1(15, rnd() % 29);
        switch (op) {
        case 0: c4(61, D(rc(-20, 84)), D(rc(-20, 84)), D(rc(-20, 84)), D(rc(-20, 84))); break;
        case 1: c6(62, D(rc(-10, 74)), D(rc(-10, 74)), D(rc(-10, 74)), D(rc(-10, 74)), D(rc(-10, 74)), D(rc(-10, 74))); break;
        case 2: c4(63, D(rc(-10, 70)), D(rc(-10, 70)), D(rc(0, 80)), D(rc(0, 80))); break;
        case 3: c5(64, D(rc(0, 30)), D(rc(0, 30)), D(rc(30, 70)), D(rc(30, 70)), D(rc(0, 20))); break;
        case 4: c6(65, D(rc(0, 30)), D(rc(0, 30)), D(rc(30, 70)), D(rc(30, 70)), D(rc(0, 20)), D(rc(0, 20))); break;
        case 5: c8(66, D(rc(0, 30)), D(rc(0, 30)), D(rc(30, 70)), D(rc(30, 70)), D(rc(0, 20)), D(rc(0, 20)), D(rc(0, 20)), D(rc(0, 20))); break;
        case 6: c4(67, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 40)), D(rc(0, 40))); break;
        case 7: c6(68, D(rc(0, 64)), D(rc(0, 64)), D(rc(1, 30)), D(rc(1, 30)), D(rc(0, 6)), D(rc(-6, 6))); break;
        case 8: c7(69, D(rc(10, 54)), D(rc(10, 54)), D(rc(1, 10)), D(rc(5, 30)), D(rc(0, 6)), 3 + rnd() % 8, 0); break;
        case 9: c6(70, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64))); break;
        case 10: c8(71, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64))); break;
        case 11:
        case 12: {
            unsigned n = 3 + rnd() % 6;
            for (k = 0; k < 2 * n; k++) dbuf[k] = rc(-10, 74);
            c2(op == 11 ? 72 : 73, P(dbuf), n);
            break;
        }
        case 13: {
            c0(82);
            c2(83, D(rc(0, 64)), D(rc(0, 64)));
            for (k = 0; k < 2 + rnd() % 4; k++) switch (rnd() % 5) {
            case 0: c2(85, D(rc(-5, 70)), D(rc(-5, 70))); break;
            case 1: c1(87, D(rc(-5, 70))); break;
            case 2: c1(89, D(rc(-5, 70))); break;
            case 3: c4(93, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64))); break;
            default: c6(97, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64))); break;
            }
            if (rnd() % 2) c0(102);
            c1(103, rnd() % 6);
            break;
        }
        case 14: {
            c0(82);
            c2(83, D(rc(0, 64)), D(rc(0, 64)));
            for (k = 0; k < 2 + rnd() % 4; k++) switch (rnd() % 5) {
            case 0: c2(86, D(rc(-20, 20)), D(rc(-20, 20))); break;
            case 1: c1(88, D(rc(-20, 20))); break;
            case 2: c1(90, D(rc(-20, 20))); break;
            case 3: c4(94, D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20))); break;
            default: c6(98, D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20))); break;
            }
            c1(103, rnd() % 6);
            break;
        }
        case 15: {
            c0(82);
            c2(83, D(rc(0, 64)), D(rc(0, 64)));
            c7(rnd() % 2 ? 91 : 92, D(rc(1, 30)), D(rc(1, 30)), D(rc(0, 6)), rnd() % 2, rnd() % 2, D(rc(0, 64)), D(rc(0, 64)));
            c1(103, rnd() % 6);
            break;
        }
        case 16: {
            c0(82);
            c2(83, D(rc(0, 64)), D(rc(0, 64)));
            c4(93, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)));
            c2(95, D(rc(0, 64)), D(rc(0, 64)));
            c4(99, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)));
            c4(100, D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)), D(rc(-20, 20)));
            c2(96, D(rc(-20, 20)), D(rc(-20, 20)));
            c1(103, rnd() % 6);
            break;
        }
        case 17:
            m = rnd() % 5;
            if (m == 0) c0(52);
            else if (m == 1) c2(58, D(rc(-20, 20)), D(rc(-20, 20)));
            else if (m == 2) c2(56, D(rc(1, 3)), D(rc(1, 3)));
            else if (m == 3) c1(55, D(rc(-3, 3)));
            else c2(57, D(rc(-1, 1)), D(rc(-1, 1)));
            break;
        case 18: c4(rnd() % 2 ? 3 : 124, rnd() % 2 ? D(rc(0, 30)) : (uint64_t)(rnd() % 30), D(rc(0, 30)), D(rc(34, 64)), D(rc(34, 64))); break;
        case 19:
            ramp = c1(120, rnd() % 2);
            for (k = 0; k < 256; k++) {
                fle_st(ramp + 8 * k + 0, 2, (rnd() & 0x7fff));
                fle_st(ramp + 8 * k + 2, 2, (rnd() & 0x7fff));
                fle_st(ramp + 8 * k + 4, 2, (rnd() & 0x7fff));
                fle_st(ramp + 8 * k + 6, 2, 0x4000 + (rnd() & 0x3fff));
            }
            m = rnd() % 3;
            if (m == 0) c5(121, D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), D(rc(0, 64)), rnd() % 2);
            else if (m == 1) c4(122, D(rc(0, 64)), D(rc(0, 64)), D(rc(1, 40)), rnd() % 2);
            else c6(123, D(rc(0, 64)), D(rc(0, 64)), D(rc(1, 40)), D(rc(0, 3)), D(rc(1, 2)), rnd() % 2);
            rect(0, 0, 64, 64);
            break;
        case 20: c1(15, rnd() % 29); break;
        default: c0(52); c4(3, D(0), D(0), D(64), D(64)); break;
        }
        if (it % 10 == 9) printf("scan: VALUE %s iter%u-hash %#llx\n", cfg_full, it, ULL(fnv(pix, NBYTES)));
    }
}
TEST(tiny1) { attach_big(1, 1, 4); fill_noline(0xff, 0, 0, 0xff); rect(-5, -5, 5, 5); big_report("1x1"); }
TEST(tiny3) { attach_big(3, 3, 12); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 3, 3); big_report("3x3"); }
TEST(tiny3b) { attach_big(3, 3, 12); big_report("3x3-attach-only"); }
TEST(tiny1b) { attach_big(1, 1, 4); big_report("1x1-attach-only"); }
TEST(tiny2) { attach_big(2, 2, 8); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 2, 2); big_report("2x2"); }
TEST(tiny4) { attach_big(4, 4, 16); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 4, 4); big_report("4x4"); }
TEST(tiny5) { attach_big(5, 5, 20); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 5, 5); big_report("5x5"); }
TEST(tiny7) { attach_big(7, 7, 28); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 7, 7); big_report("7x7"); }
TEST(tiny9) { attach_big(9, 9, 36); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 9, 9); big_report("9x9"); }
TEST(tiny1x16) { attach_big(1, 16, 4); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 1, 16); big_report("1x16"); }
TEST(tiny16x1) { attach_big(16, 1, 64); fill_noline(0xff, 0, 0, 0xff); rect(0, 0, 16, 1); big_report("16x1"); }

TEST(stubs) { /* the 52 slots that point at a 1-instruction stub: 49 `ret` (the function at RVA 0xc530) and 3 `xor eax,eax; ret` (0xb820; slots 14, 16, 18). Each is called once; the buffer must not change. */
    static const unsigned ret_slots[] = {2, 4, 5, 7, 17, 19, 20, 21, 24, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 50, 51, 53, 54, 74, 75, 76, 77, 78, 79, 80, 81,
                                         104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 125, 127};
    static const unsigned xor_slots[] = {14, 16, 18};
    unsigned k;
    for (k = 0; k < sizeof ret_slots / sizeof ret_slots[0]; k++) c0(ret_slots[k]);
    for (k = 0; k < sizeof xor_slots / sizeof xor_slots[0]; k++) {
        uint64_t r = c0(xor_slots[k]);
        printf("scan: VALUE %s slot%u-returned %u\n", cfg_full, xor_slots[k], (unsigned)(uint32_t)r);
        check("xor-stub-returns-0", (uint32_t)r == 0, "READING: the stub is `xor eax,eax; ret`", "slot %u returned %u", xor_slots[k], (unsigned)(uint32_t)r);
    }
    check("stubs-leave-buffer", changed() == 0, "READING: a stub does nothing", "changed pixels %u", changed());
    value("slots-called", (long long)(sizeof ret_slots / sizeof ret_slots[0] + sizeof xor_slots / sizeof xor_slots[0]));
}

/* ---- extreme values: NaN, infinities, huge and tiny numbers and odd widths through the drawing entries. FLE_B2D_SCAN=extreme:<V*100+O> runs value V (0..11) in operation O (0..10);
 * extreme alone runs all of them in one process. A native build arms alarm(20) before the calls (the process ends with "limit: wall time"), a wasm build has no alarm. ---- */
static const uint64_t ext_bits[] = {
    0x7ff8000000000000ull, /* 0 NaN */
    0x7ff0000000000000ull, /* 1 +inf */
    0xfff0000000000000ull, /* 2 -inf */
    0x7fefffffffffffffull, /* 3 DBL_MAX */
    0x0000000000000001ull, /* 4 smallest denormal */
    0x8000000000000000ull, /* 5 -0.0 */
    0x4415af1d78b58c40ull, /* 6 1e20 */
    0xc415af1d78b58c40ull, /* 7 -1e20 */
    0x41e0000000000000ull, /* 8 2^31 */
    0x41f0000000000000ull, /* 9 2^32 */
    0x4330000000000000ull, /* 10 2^52 */
    0x3810000000000000ull, /* 11 a tiny normal */
};
#define EXT_NV 12
#define EXT_NO 11
static const char *const ext_opname[EXT_NO] = {"rect-x1", "rect-y1", "line-x1", "ellipse-rx", "triangle", "roundrect-r", "path-lineTo-x", "path-lineTo-y", "quad-curve", "rotate-rect", "scale-rect"};
static void ext_one(unsigned vi, unsigned op) {
    uint64_t v = ext_bits[vi];
    static const double wd[3] = {0.0, 1.0, 1e9};
    unsigned k;
    for (k = 0; k < 3; k++) {
        printf("scan: ITER value %u op %u (%s) width %u\n", vi, op, ext_opname[op], k);
        fflush(stdout);
        colour(27, 0x20 + 20 * vi, 0xff - 10 * vi, 0x40, 0xff);
        colour(30, 0xff, 0x80, 0x10 * k, 0xc0);
        c1(42, D(wd[k]));
        switch (op) {
        case 0: c4(63, v, D(10), D(40), D(40)); break;
        case 1: c4(63, D(5), v, D(60), D(50)); break;
        case 2: c4(61, v, D(5), D(50), D(60)); break;
        case 3: c4(67, D(32), D(32), v, D(10)); break;
        case 4: c6(62, D(5), D(5), v, D(30), D(50), D(60)); break;
        case 5: c5(64, D(8), D(8), D(56), D(56), v); break;
        case 6: c0(82); c2(83, D(10), D(10)); c2(85, v, D(30)); c0(102); c1(103, k); break;
        case 7: c0(82); c2(83, D(10), D(10)); c2(85, D(50), v); c0(102); c1(103, k); break;
        case 8: c6(70, D(10), D(50), v, D(10), D(50), D(50)); break;
        case 9: c0(52); c1(55, v); c4(63, D(10), D(10), D(40), D(40)); c0(52); break;
        default: c0(52); c2(56, v, D(1)); c4(63, D(10), D(10), D(40), D(40)); c0(52); break;
        }
    }
}
TEST(extreme) {
    unsigned vi, op;
#ifndef __EMSCRIPTEN__
    alarm(20);
#endif
    if (targ >= 0) {
        vi = (unsigned)targ / 100;
        op = (unsigned)targ % 100;
        if (vi >= EXT_NV || op >= EXT_NO) {
            printf("scan: RESULT %s status=bad-argument\n", cfg_full);
            exit(2);
        }
        ext_one(vi, op);
    } else {
        for (vi = 0; vi < EXT_NV; vi++)
            for (op = 0; op < EXT_NO; op++) ext_one(vi, op);
    }
    clear_bg();
    fill_noline(0xff, 0, 0, 0xff);
    rect(8, 8, 40, 40);
    check("after-extremes-a-plain-rect-is-right", changed() == 1024 && is_px(20, 20, 0x00, 0x00, 0xff, 0xff), "SPEC: after the extreme calls (the transform was reset by slot 52) a clear and a 32x32 rectangle still give 1024 red pixels", "changed %u", changed());
}
