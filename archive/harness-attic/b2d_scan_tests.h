/* X2 scan: the tests of the entries that X1 measured and of the ones FLEngine calls most (included by b2d_scan.c; see b2d_scan.h).
 * Slot numbers are indexes into the Agg2D vtable of Blend2D.dll; their arguments are READINGS of the disassembly
 * (docs/investigations/blend2d-nojit-slots.md section 3). Colour calls take (r, g, b, a) bytes. A SPEC check is a rule that does not depend on what
 * the wrapper does; a READING check is what the wrapper seems to do. */

TEST(clear) {
    colour(8, 0x11, 0x22, 0x33, 0xff);
    check_px("clear-origin", 0, 0, 0x33, 0x22, 0x11, 0xff, "SPEC: opaque clearAll(r=0x11,g=0x22,b=0x33) stores B,G,R,A = 33 22 11 ff");
    check_px("clear-last", 63, 63, 0x33, 0x22, 0x11, 0xff, "SPEC");
}
TEST(rect) {
    fill_noline(0xff, 0, 0, 0xff);
    rect(8, 8, 40, 40);
    check_px("rect-inside", 20, 20, 0x00, 0x00, 0xff, 0xff, "SPEC: opaque red fill");
    check_bg("rect-outside", 50, 50, "SPEC");
    check("rect-count", changed() == 1024, "SPEC", "changed pixels %u; a 32x32 box with integer corners covers 1024 pixels", changed());
}
TEST(blend) {
    fill_noline(0xff, 0, 0, 0xff);
    rect(8, 8, 40, 40);
    fill_noline(0, 0, 0xff, 0x80);
    rect(24, 24, 56, 56);
    check_px("blend-over-red", 30, 30, 0x80, 0x00, 0x7f, 0xff, "SPEC: source-over of (r=0,g=0,b=255,a=128) on opaque red (rounded as X1 measured)");
    check_px("blend-over-bg", 50, 50, 0x99, 0x11, 0x08, 0xff, "SPEC: the same colour over 33 22 11 ff");
}
TEST(line) {
    colour(30, 0, 0xff, 0, 0xff);
    c1(42, D(3.0));
    c4(61, D(2), D(60), D(60), D(2));
    check_px("line-on", 31, 31, 0x00, 0xff, 0x00, 0xff, "SPEC: the middle of a 3-wide opaque green diagonal");
    check_bg("line-off", 5, 5, "SPEC");
}
TEST(path) {
    fill_noline(0, 0xff, 0xff, 0xff);
    tri_path();
    c0(102);
    c1(103, 0);
    check_tri("SPEC: the fill colour (r=0,g=255,b=255)");
}
TEST(ellipse) {
    fill_noline(0xff, 0xff, 0, 0xff);
    c4(67, D(32), D(32), D(20), D(12));
    check_px("ellipse-centre", 32, 32, 0x00, 0xff, 0xff, 0xff, "SPEC");
    check_bg("ellipse-outside", 2, 2, "SPEC");
    {
        double a = area(0, BG_B, 0x00); /* coverage seen in the blue channel: 0x33 -> 0 */
        check("ellipse-area", a > 0.97 * PI * 20 * 12 && a < 1.03 * PI * 20 * 12, "SPEC", "covered area %.1f, pi*20*12 = %.1f (within 3%%)", a, PI * 20 * 12);
    }
}

/* ---- the entries FLEngine calls most (slots 26..31 colours, 42 lineWidth(?), 82..103 path, 120..123 ellipse/arc, 3 clipBox) ---- */
TEST(fillcolor) { /* slot 26 sets the fill colour from a packed word (reading), 27 from r,g,b,a, 28 noFill, 29/30 the line colour, 31 noLine */
    colour(27, 1, 2, 3, 4);
    value("fill-word-27", (long long)fle_ld(obj + 0x118, 4));
    check("fill-27-argb", fle_ld(obj + 0x118, 4) == 0x04010203, "READING: slot 27 stores A<<24|R<<16|G<<8|B (r=1,g=2,b=3,a=4)", "fill word %#x", (unsigned)fle_ld(obj + 0x118, 4));
    c1(26, 0x00aabbcc);
    value("fill-word-26", (long long)fle_ld(obj + 0x118, 4));
    c0(28);
    value("fill-word-28", (long long)fle_ld(obj + 0x118, 4));
    colour(30, 5, 6, 7, 8);
    value("line-word-30", (long long)fle_ld(obj + 0x11c, 4));
    c1(29, 0x00112233);
    value("line-word-29", (long long)fle_ld(obj + 0x11c, 4));
    c0(31);
    value("line-word-31", (long long)fle_ld(obj + 0x11c, 4));
    check("fill-after-nofill-is-0", fle_ld(obj + 0x118, 4) == 0, "READING: slot 28 stores 0 at Agg2D+0x118", "fill word %#x", (unsigned)fle_ld(obj + 0x118, 4));
}
TEST(clipbox) { /* slot 3: clipBox(x1,y1,x2,y2) in doubles (reading: x1,y1,x2,y2 -> a box x, y, w, h passed to the context's vtable +0x168) */
    clear_bg();
    c4(3, D(16), D(16), D(48), D(48));
    fill_noline(0xff, 0, 0, 0xff);
    rect(0, 0, 64, 64);
    check_px("clip-inside", 32, 32, 0x00, 0x00, 0xff, 0xff, "SPEC: a fill of the whole buffer is cut to the clip box: red inside");
    check_bg("clip-outside-left", 8, 32, "SPEC");
    check_bg("clip-outside-bottom", 32, 56, "SPEC");
    check("clip-count", count_px(0x00, 0x00, 0xff, 0xff) == 32 * 32, "SPEC", "red pixels %u, a 32x32 clip box holds 1024", count_px(0x00, 0x00, 0xff, 0xff));
}
TEST(nofill) {
    fill_noline(0xff, 0, 0, 0xff);
    c0(28);
    rect(8, 8, 40, 40);
    check("nofill-rect-unchanged", changed() == 0, "SPEC", "changed pixels %u: a rectangle with no fill and no line draws nothing", changed());
}
TEST(rectline) { /* the outline of a rectangle: line colour + noFill */
    c0(28);
    colour(30, 0, 0xff, 0, 0xff);
    c1(42, D(2.0));
    rect(10, 10, 50, 50);
    check_px("outline-top-edge", 30, 10, 0x00, 0xff, 0x00, 0xff, "SPEC: the middle of a 2-wide stroke centred on y=10 is fully covered at y=10");
    check_bg("outline-inside", 30, 30, "SPEC");
    check_bg("outline-outside", 2, 2, "SPEC");
}
TEST(linewidth) { /* slot 42, 43: set / get the line width (reading: set stores into the context's state through its vtable +0x118) */
    c1(42, D(5.0));
    c0(43);
    dvalue("width-after-set-5", ret_d());
    c1(42, D(1.0));
    c0(43);
    dvalue("width-after-set-1", ret_d());
    check("width-get", ret_d() == 1.0, "READING", "slot 43 returned %g after slot 42 stored 1.0", ret_d());
}
TEST(pathfill) { /* the path entries 82 resetPath, 83 moveTo, 85 lineTo, 102 closePolygon, 103 drawPath(0): cyan triangle */
    fill_noline(0, 0xff, 0xff, 0xff);
    tri_path();
    c0(102);
    c1(103, 0);
    check_tri("SPEC");
    {
        double a = area(2, BG_R, 0); /* the red channel falls from 0x11 to 0: coverage */
        double want = 0.5 * 40 * 40;
        check("tri-area", a > 0.97 * want && a < 1.03 * want, "SPEC", "covered area %.1f, the triangle has %.1f (within 3%%)", a, want);
    }
}
TEST(pathstroke) { /* drawPath(1): stroke only */
    colour(30, 0xff, 0xff, 0xff, 0xff);
    c1(42, D(2.0));
    c0(28);
    tri_path();
    c0(102);
    c1(103, 1);
    check_px("stroke-on-edge", 30, 50, 0xff, 0xff, 0xff, 0xff, "SPEC: the base of the triangle at y=50 is on the stroke");
    check_bg("stroke-inside", 30, 40, "SPEC");
}
TEST(pathboth) { /* drawPath(2): fill and stroke */
    colour(27, 0, 0xff, 0xff, 0xff);
    colour(30, 0xff, 0, 0, 0xff);
    c1(42, D(2.0));
    tri_path();
    c0(102);
    c1(103, 2);
    check_px("both-inside", 30, 40, 0xff, 0xff, 0x00, 0xff, "SPEC: the fill colour inside");
    check_px("both-edge", 30, 50, 0x00, 0x00, 0xff, 0xff, "SPEC: the line colour on the edge");
}
TEST(curves) { /* quad 84? 86.. 100: lineRel/moveRel/arcs/curves of the path (readings: 84 moveRel, 86 lineRel, 87 horLineTo, 88 horLineRel, 89 verLineTo, 90 verLineRel) */
    fill_noline(0, 0xff, 0xff, 0xff);
    c0(82);
    c2(83, D(10), D(10));
    c1(87, D(50)); /* horizontal line to x = 50 */
    c1(89, D(50)); /* vertical line to y = 50 */
    c1(87, D(10)); /* horizontal line back to x = 10 */
    c0(102);
    c1(103, 0);
    check_px("hv-inside", 30, 30, 0xff, 0xff, 0x00, 0xff, "SPEC: the square (10,10)-(50,50) is filled");
    check("hv-count", count_px(0xff, 0xff, 0x00, 0xff) == 40 * 40, "SPEC", "cyan pixels %u, a 40x40 square holds 1600", count_px(0xff, 0xff, 0x00, 0xff));
}
