//! Tests for the pixel model of runtime/fle_raster.c (decision 0126, PROVISIONAL): raster operations on device-dependent and device-independent surfaces, the 256 ternary codes, the 16 binary modes, stretching,
//! lines and the drawing surfaces of top-level windows.
//!
//! The expected values come from Microsoft Learn, not from the C code: the table of the 256 ternary raster operation codes ("Ternary Raster Operations", fetched into learn/Ternary-Raster-Operations.txt and
//! turned into ROP3_RPN below by tools/gen_rop3.py, which checks that the value of each reverse Polish expression with P = 0xF0, S = 0xCC, D = 0xAA is the 8-bit index and the high word of the code), the
//! binary raster operations (SetROP2), the BitBlt / StretchBlt / LineTo pages ("up to, but not including, the specified point"), and the DIB row order (a positive height is bottom-up). A number that no page gives
//! (the surface limits, the unused byte of a 32-bit pixel) is marked "choice" and is the only thing such a test pins down.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

mod etxt_common;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_RASTER_SRC_DIR") {
        return PathBuf::from(d);
    }
    runtime_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-raster/test");
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub const ROP3_RPN: [(&str, u32); 256] = [
    ("0", 0x00000042), ("DPSoon", 0x00010289), ("DPSona", 0x00020C89), ("PSon", 0x000300AA),
    ("SDPona", 0x00040C88), ("DPon", 0x000500A9), ("PDSxnon", 0x00060865), ("PDSaon", 0x000702C5),
    ("SDPnaa", 0x00080F08), ("PDSxon", 0x00090245), ("DPna", 0x000A0329), ("PSDnaon", 0x000B0B2A),
    ("SPna", 0x000C0324), ("PDSnaon", 0x000D0B25), ("PDSonon", 0x000E08A5), ("Pn", 0x000F0001),
    ("PDSona", 0x00100C85), ("DSon", 0x001100A6), ("SDPxnon", 0x00120868), ("SDPaon", 0x001302C8),
    ("DPSxnon", 0x00140869), ("DPSaon", 0x001502C9), ("PSDPSanaxx", 0x00165CCA), ("SSPxDSxaxn", 0x00171D54),
    ("SPxPDxa", 0x00180D59), ("SDPSanaxn", 0x00191CC8), ("PDSPaox", 0x001A06C5), ("SDPSxaxn", 0x001B0768),
    ("PSDPaox", 0x001C06CA), ("DSPDxaxn", 0x001D0766), ("PDSox", 0x001E01A5), ("PDSoan", 0x001F0385),
    ("DPSnaa", 0x00200F09), ("SDPxon", 0x00210248), ("DSna", 0x00220326), ("SPDnaon", 0x00230B24),
    ("SPxDSxa", 0x00240D55), ("PDSPanaxn", 0x00251CC5), ("SDPSaox", 0x002606C8), ("SDPSxnox", 0x00271868),
    ("DPSxa", 0x00280369), ("PSDPSaoxxn", 0x002916CA), ("DPSana", 0x002A0CC9), ("SSPxPDxaxn", 0x002B1D58),
    ("SPDSoax", 0x002C0784), ("PSDnox", 0x002D060A), ("PSDPxox", 0x002E064A), ("PSDnoan", 0x002F0E2A),
    ("PSna", 0x0030032A), ("SDPnaon", 0x00310B28), ("SDPSoox", 0x00320688), ("Sn", 0x00330008),
    ("SPDSaox", 0x003406C4), ("SPDSxnox", 0x00351864), ("SDPox", 0x003601A8), ("SDPoan", 0x00370388),
    ("PSDPoax", 0x0038078A), ("SPDnox", 0x00390604), ("SPDSxox", 0x003A0644), ("SPDnoan", 0x003B0E24),
    ("PSx", 0x003C004A), ("SPDSonox", 0x003D18A4), ("SPDSnaox", 0x003E1B24), ("PSan", 0x003F00EA),
    ("PSDnaa", 0x00400F0A), ("DPSxon", 0x00410249), ("SDxPDxa", 0x00420D5D), ("SPDSanaxn", 0x00431CC4),
    ("SDna", 0x00440328), ("DPSnaon", 0x00450B29), ("DSPDaox", 0x004606C6), ("PSDPxaxn", 0x0047076A),
    ("SDPxa", 0x00480368), ("PDSPDaoxxn", 0x004916C5), ("DPSDoax", 0x004A0789), ("PDSnox", 0x004B0605),
    ("SDPana", 0x004C0CC8), ("SSPxDSxoxn", 0x004D1954), ("PDSPxox", 0x004E0645), ("PDSnoan", 0x004F0E25),
    ("PDna", 0x00500325), ("DSPnaon", 0x00510B26), ("DPSDaox", 0x005206C9), ("SPDSxaxn", 0x00530764),
    ("DPSonon", 0x005408A9), ("Dn", 0x00550009), ("DPSox", 0x005601A9), ("DPSoan", 0x00570389),
    ("PDSPoax", 0x00580785), ("DPSnox", 0x00590609), ("DPx", 0x005A0049), ("DPSDonox", 0x005B18A9),
    ("DPSDxox", 0x005C0649), ("DPSnoan", 0x005D0E29), ("DPSDnaox", 0x005E1B29), ("DPan", 0x005F00E9),
    ("PDSxa", 0x00600365), ("DSPDSaoxxn", 0x006116C6), ("DSPDoax", 0x00620786), ("SDPnox", 0x00630608),
    ("SDPSoax", 0x00640788), ("DSPnox", 0x00650606), ("DSx", 0x00660046), ("SDPSonox", 0x006718A8),
    ("DSPDSonoxxn", 0x006858A6), ("PDSxxn", 0x00690145), ("DPSax", 0x006A01E9), ("PSDPSoaxxn", 0x006B178A),
    ("SDPax", 0x006C01E8), ("PDSPDoaxxn", 0x006D1785), ("SDPSnoax", 0x006E1E28), ("PDSxnan", 0x006F0C65),
    ("PDSana", 0x00700CC5), ("SSDxPDxaxn", 0x00711D5C), ("SDPSxox", 0x00720648), ("SDPnoan", 0x00730E28),
    ("DSPDxox", 0x00740646), ("DSPnoan", 0x00750E26), ("SDPSnaox", 0x00761B28), ("DSan", 0x007700E6),
    ("PDSax", 0x007801E5), ("DSPDSoaxxn", 0x00791786), ("DPSDnoax", 0x007A1E29), ("SDPxnan", 0x007B0C68),
    ("SPDSnoax", 0x007C1E24), ("DPSxnan", 0x007D0C69), ("SPxDSxo", 0x007E0955), ("DPSaan", 0x007F03C9),
    ("DPSaa", 0x008003E9), ("SPxDSxon", 0x00810975), ("DPSxna", 0x00820C49), ("SPDSnoaxn", 0x00831E04),
    ("SDPxna", 0x00840C48), ("PDSPnoaxn", 0x00851E05), ("DSPDSoaxx", 0x008617A6), ("PDSaxn", 0x008701C5),
    ("DSa", 0x008800C6), ("SDPSnaoxn", 0x00891B08), ("DSPnoa", 0x008A0E06), ("DSPDxoxn", 0x008B0666),
    ("SDPnoa", 0x008C0E08), ("SDPSxoxn", 0x008D0668), ("SSDxPDxax", 0x008E1D7C), ("PDSanan", 0x008F0CE5),
    ("PDSxna", 0x00900C45), ("SDPSnoaxn", 0x00911E08), ("DPSDPoaxx", 0x009217A9), ("SPDaxn", 0x009301C4),
    ("PSDPSoaxx", 0x009417AA), ("DPSaxn", 0x009501C9), ("DPSxx", 0x00960169), ("PSDPSonoxx", 0x0097588A),
    ("SDPSonoxn", 0x00981888), ("DSxn", 0x00990066), ("DPSnax", 0x009A0709), ("SDPSoaxn", 0x009B07A8),
    ("SPDnax", 0x009C0704), ("DSPDoaxn", 0x009D07A6), ("DSPDSaoxx", 0x009E16E6), ("PDSxan", 0x009F0345),
    ("DPa", 0x00A000C9), ("PDSPnaoxn", 0x00A11B05), ("DPSnoa", 0x00A20E09), ("DPSDxoxn", 0x00A30669),
    ("PDSPonoxn", 0x00A41885), ("PDxn", 0x00A50065), ("DSPnax", 0x00A60706), ("PDSPoaxn", 0x00A707A5),
    ("DPSoa", 0x00A803A9), ("DPSoxn", 0x00A90189), ("D", 0x00AA0029), ("DPSono", 0x00AB0889),
    ("SPDSxax", 0x00AC0744), ("DPSDaoxn", 0x00AD06E9), ("DSPnao", 0x00AE0B06), ("DPno", 0x00AF0229),
    ("PDSnoa", 0x00B00E05), ("PDSPxoxn", 0x00B10665), ("SSPxDSxox", 0x00B21974), ("SDPanan", 0x00B30CE8),
    ("PSDnax", 0x00B4070A), ("DPSDoaxn", 0x00B507A9), ("DPSDPaoxx", 0x00B616E9), ("SDPxan", 0x00B70348),
    ("PSDPxax", 0x00B8074A), ("DSPDaoxn", 0x00B906E6), ("DPSnao", 0x00BA0B09), ("DSno", 0x00BB0226),
    ("SPDSanax", 0x00BC1CE4), ("SDxPDxan", 0x00BD0D7D), ("DPSxo", 0x00BE0269), ("DPSano", 0x00BF08C9),
    ("PSa", 0x00C000CA), ("SPDSnaoxn", 0x00C11B04), ("SPDSonoxn", 0x00C21884), ("PSxn", 0x00C3006A),
    ("SPDnoa", 0x00C40E04), ("SPDSxoxn", 0x00C50664), ("SDPnax", 0x00C60708), ("PSDPoaxn", 0x00C707AA),
    ("SDPoa", 0x00C803A8), ("SPDoxn", 0x00C90184), ("DPSDxax", 0x00CA0749), ("SPDSaoxn", 0x00CB06E4),
    ("S", 0x00CC0020), ("SDPono", 0x00CD0888), ("SDPnao", 0x00CE0B08), ("SPno", 0x00CF0224),
    ("PSDnoa", 0x00D00E0A), ("PSDPxoxn", 0x00D1066A), ("PDSnax", 0x00D20705), ("SPDSoaxn", 0x00D307A4),
    ("SSPxPDxax", 0x00D41D78), ("DPSanan", 0x00D50CE9), ("PSDPSaoxx", 0x00D616EA), ("DPSxan", 0x00D70349),
    ("PDSPxax", 0x00D80745), ("SDPSaoxn", 0x00D906E8), ("DPSDanax", 0x00DA1CE9), ("SPxDSxan", 0x00DB0D75),
    ("SPDnao", 0x00DC0B04), ("SDno", 0x00DD0228), ("SDPxo", 0x00DE0268), ("SDPano", 0x00DF08C8),
    ("PDSoa", 0x00E003A5), ("PDSoxn", 0x00E10185), ("DSPDxax", 0x00E20746), ("PSDPaoxn", 0x00E306EA),
    ("SDPSxax", 0x00E40748), ("PDSPaoxn", 0x00E506E5), ("SDPSanax", 0x00E61CE8), ("SPxPDxan", 0x00E70D79),
    ("SSPxDSxax", 0x00E81D74), ("DSPDSanaxxn", 0x00E95CE6), ("DPSao", 0x00EA02E9), ("DPSxno", 0x00EB0849),
    ("SDPao", 0x00EC02E8), ("SDPxno", 0x00ED0848), ("DSo", 0x00EE0086), ("SDPnoo", 0x00EF0A08),
    ("P", 0x00F00021), ("PDSono", 0x00F10885), ("PDSnao", 0x00F20B05), ("PSno", 0x00F3022A),
    ("PSDnao", 0x00F40B0A), ("PDno", 0x00F50225), ("PDSxo", 0x00F60265), ("PDSano", 0x00F708C5),
    ("PDSao", 0x00F802E5), ("PDSxno", 0x00F90845), ("DPo", 0x00FA0089), ("DPSnoo", 0x00FB0A09),
    ("PSo", 0x00FC008A), ("PSDnoo", 0x00FD0A0A), ("DPSoo", 0x00FE02A9), ("1", 0x00FF0062),
];


const DRIVER: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_raster.h"

#define CHECK(c) do { if (!(c)) { printf("FAIL line %d: %s\n", __LINE__, #c); exit(1); } } while (0)

static fle_rsurf mk(uint32_t w, uint32_t h, uint32_t bpp, int bu)
{
    fle_rsurf s;
    memset(&s, 0, sizeof s);
    s.stride = ((w * bpp + 31) & ~31u) / 8;
    s.bits = calloc(1, (size_t)s.stride * h);
    s.width = w; s.height = h; s.bpp = bpp; s.bottom_up = bu;
    return s;
}

static void named(void)
{
    uint32_t idx;
    CHECK(fle_r_rop3_valid(0x00CC0020, &idx) && idx == 0xCC);                 /* SRCCOPY */
    CHECK(fle_r_rop3_valid(0x00F00021, &idx) && idx == 0xF0);                 /* PATCOPY */
    CHECK(fle_r_rop3_valid(0x00000042, &idx) && idx == 0);                    /* BLACKNESS */
    CHECK(fle_r_rop3_valid(0x00FF0062, &idx) && idx == 0xFF);                 /* WHITENESS */
    CHECK(!fle_r_rop3_valid(0x00CC0021, &idx));                               /* right index, wrong low word */
    CHECK(!fle_r_rop3_valid(0x40CC0020, &idx));                               /* CAPTUREBLT: the caller strips the flag bits */
    CHECK(fle_r_rop3_code(0x66) == 0x00660046u);                              /* SRCINVERT */
    CHECK(fle_r_rop3(0xCC, 0x123456, 0xABCDEF, 0x0F0F0F) == 0xABCDEF);        /* SRCCOPY */
    CHECK(fle_r_rop3(0xF0, 0x123456, 0xABCDEF, 0x0F0F0F) == 0x123456);        /* PATCOPY */
    CHECK(fle_r_rop3(0x66, 0x123456, 0xABCDEF, 0x0F0F0F) == (0xABCDEFu ^ 0x0F0F0Fu));       /* SRCINVERT = DSx */
    CHECK(fle_r_rop3(0x88, 0, 0xABCDEF, 0x0F0F0F) == (0xABCDEFu & 0x0F0F0Fu));              /* SRCAND */
    CHECK(fle_r_rop3(0xEE, 0, 0xABCDEF, 0x0F0F0F) == (0xABCDEFu | 0x0F0F0Fu));              /* SRCPAINT */
    CHECK(fle_r_rop3(0x55, 0, 0xABCDEF, 0x0F0F0F) == (~0x0F0F0Fu & 0xFFFFFF));              /* DSTINVERT */
    CHECK(fle_r_rop3(0x5A, 0x123456, 0, 0x0F0F0F) == (0x123456u ^ 0x0F0F0Fu));              /* PATINVERT */
    CHECK(fle_r_rop3(0x33, 0, 0xABCDEF, 0) == (~0xABCDEFu & 0xFFFFFF));                     /* NOTSRCCOPY */
    CHECK(fle_r_rop3(0x11, 0, 0xABCDEF, 0x0F0F0F) == (~(0xABCDEFu | 0x0F0F0Fu) & 0xFFFFFF));/* NOTSRCERASE */
    CHECK(fle_r_rop3(0x44, 0, 0xABCDEF, 0x0F0F0F) == (0xABCDEFu & ~0x0F0F0Fu & 0xFFFFFF));  /* SRCERASE = SDna */
    CHECK(fle_r_rop3(0xBB, 0, 0xABCDEF, 0x0F0F0F) == ((~0xABCDEFu | 0x0F0F0Fu) & 0xFFFFFF));/* MERGEPAINT = DSno */
    CHECK(fle_r_rop3(0xC0, 0x123456, 0xABCDEF, 0) == (0x123456u & 0xABCDEFu));              /* MERGECOPY = PSa */
    CHECK(fle_r_rop3(0xFB, 0x123456, 0xABCDEF, 0x0F0F0F) == ((0x123456u | ~0xABCDEFu | 0x0F0F0Fu) & 0xFFFFFF)); /* PATPAINT = DPSnoo */
    CHECK(fle_r_idx_uses_source(0xCC) && !fle_r_idx_uses_source(0xF0) && !fle_r_idx_uses_source(0x55) && !fle_r_idx_uses_source(0));
    CHECK(!fle_r_idx_uses_pattern(0xCC) && fle_r_idx_uses_pattern(0xF0) && fle_r_idx_uses_pattern(0x5A) && fle_r_idx_uses_pattern(0xC0));
    CHECK(!fle_r_idx_uses_dest(0xCC) && !fle_r_idx_uses_dest(0xF0) && fle_r_idx_uses_dest(0x55) && fle_r_idx_uses_dest(0x66) && !fle_r_idx_uses_dest(0xFF));
}

static void binary(void)
{
    for (uint32_t m = 1; m <= 16; m++) CHECK(fle_r_rop2_valid(m));
    CHECK(!fle_r_rop2_valid(0) && !fle_r_rop2_valid(17));
    CHECK(fle_r_rop2(13, 0x123456, 0xABCDEF) == 0x123456);                    /* R2_COPYPEN */
    CHECK(fle_r_rop2(7, 0x123456, 0xABCDEF) == (0x123456u ^ 0xABCDEFu));      /* R2_XORPEN */
    CHECK(fle_r_rop2(6, 0x123456, 0xABCDEF) == (~0xABCDEFu & 0xFFFFFF));      /* R2_NOT */
    CHECK(fle_r_rop2(16, 0, 0) == 0xFFFFFF);                                  /* R2_WHITE */
    CHECK(fle_r_rop2(1, 0xFFFFFF, 0xFFFFFF) == 0);                            /* R2_BLACK */
    CHECK(fle_r_rop2(11, 1, 0xABCDEF) == 0xABCDEF);                           /* R2_NOP */
}

static void surface(void)
{
    fle_rsurf d = mk(8, 6, 32, 0), e = mk(8, 6, 24, 1);
    uint64_t n;

    CHECK(fle_r_direct(&d) && fle_r_direct(&e));
    fle_r_rop_rect(&d, NULL, (fle_rrect){1, 1, 4, 3}, 0xF0, 0x0000FF);        /* PATCOPY red */
    CHECK(fle_r_get(&d, 1, 1) == 0x0000FF && fle_r_get(&d, 3, 2) == 0x0000FF && fle_r_get(&d, 4, 2) == 0 && fle_r_get(&d, 3, 3) == 0);
    CHECK(fle_r_row(&d, 1)[4] == 0x00 && fle_r_row(&d, 1)[6] == 0xFF && fle_r_row(&d, 1)[7] == 0);   /* BGRX: x = 1 is bytes 4..7 = B G R X; the unused byte is written 0 (choice) */
    fle_r_put(&e, 0, 0, 0x112233);
    CHECK(e.bits[(size_t)(6 - 1) * e.stride + 0] == 0x11 && e.bits[(size_t)(6 - 1) * e.stride + 2] == 0x33);   /* bottom-up: image row 0 is the LAST memory row; bytes B G R */
    n = fle_r_rop_rect(&d, &(fle_rrect){2, 0, 3, 2}, (fle_rrect){0, 0, 8, 6}, 0xF0, 0x00FF00);
    CHECK(n == 2 && fle_r_get(&d, 2, 0) == 0x00FF00 && fle_r_get(&d, 2, 1) == 0x00FF00 && fle_r_get(&d, 3, 1) == 0x0000FF);   /* the clip keeps one column */
    n = fle_r_rop_rect(&d, NULL, (fle_rrect){-5, -5, 100, 100}, 0xFF, 0);
    CHECK(n == 48 && fle_r_get(&d, 7, 5) == 0xFFFFFF);                        /* a rectangle larger than the surface is cut to it */
    n = fle_r_rop_rect(&d, NULL, (fle_rrect){3, 3, 3, 9}, 0xF0, 1);
    CHECK(n == 0);                                                            /* an empty rectangle */
}

static void blit(void)
{
    fle_rsurf g = mk(8, 1, 32, 0), h = mk(4, 1, 32, 0), t24 = mk(4, 1, 24, 0), h2 = mk(4, 1, 32, 0), h3 = mk(4, 1, 32, 0);
    uint32_t want[8] = {0x10, 0x11, 0x10, 0x11, 0x12, 0x13, 0x14, 0x17};
    uint32_t want2[8] = {0x12, 0x13, 0x14, 0x15, 0x16, 0x15, 0x16, 0x17};
    uint32_t src[8] = {0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17};

    for (uint32_t x = 0; x < 8; x++) fle_r_put(&g, x, 0, 0x10 + x);
    fle_r_blit(&g, NULL, 2, 0, 5, 1, &g, 0, 0, 0xCC, 0);                      /* the same surface, the destination overlaps the source to the right */
    for (uint32_t x = 0; x < 8; x++) CHECK(fle_r_get(&g, x, 0) == want[x]);
    for (uint32_t x = 0; x < 8; x++) fle_r_put(&g, x, 0, 0x10 + x);
    fle_r_blit(&g, NULL, 0, 0, 5, 1, &g, 2, 0, 0xCC, 0);                      /* ... and to the left */
    for (uint32_t x = 0; x < 8; x++) CHECK(fle_r_get(&g, x, 0) == want2[x]);
    for (uint32_t x = 0; x < 8; x++) fle_r_put(&g, x, 0, 0x10 + x);
    fle_r_blit(&g, NULL, 2, 0, 5, 1, &g, 0, 0, 0x66, 0);                      /* D ^ S with an overlap: the source is read before it is overwritten */
    for (uint32_t x = 2; x < 7; x++) CHECK(fle_r_get(&g, x, 0) == (src[x] ^ src[x - 2]));
    memset(h.bits, 0xEE, h.stride);
    for (uint32_t x = 0; x < 4; x++) fle_r_put(&t24, x, 0, 0x010203 * (x + 1));
    fle_r_blit(&h, NULL, 0, 0, 4, 1, &t24, 0, 0, 0xCC, 0);                    /* 24 -> 32 bits: converted, the unused byte is 0 (choice) */
    for (uint32_t x = 0; x < 4; x++) { CHECK(fle_r_get(&h, x, 0) == 0x010203u * (x + 1)); CHECK(h.bits[x * 4 + 3] == 0); }
    memset(h3.bits, 0x5A, h3.stride);
    h2.keep_x = h3.keep_x = 1;
    fle_r_blit(&h2, NULL, 0, 0, 4, 1, &h3, 0, 0, 0xCC, 0);                    /* SRCCOPY between two DIB sections of the same format copies the unused byte (choice) */
    CHECK(h2.bits[3] == 0x5A);
    h2.keep_x = 0;
    fle_r_blit(&h2, NULL, 0, 0, 4, 1, &h3, 0, 0, 0xCC, 0);                    /* a window surface or a DDB has none */
    CHECK(h2.bits[3] == 0);
}

static void stretch(void)
{
    fle_rsurf a2 = mk(2, 1, 32, 0), b4 = mk(4, 1, 32, 0), q4 = mk(4, 1, 32, 0), q2 = mk(2, 1, 32, 0);
    uint32_t m;

    fle_r_put(&a2, 0, 0, 0xAA); fle_r_put(&a2, 1, 0, 0xBB);
    fle_r_stretch(&b4, NULL, 0, 0, 4, 1, &a2, 0, 0, 2, 1, 0xCC, 0, 3);
    CHECK(fle_r_get(&b4, 0, 0) == 0xAA && fle_r_get(&b4, 1, 0) == 0xAA && fle_r_get(&b4, 2, 0) == 0xBB && fle_r_get(&b4, 3, 0) == 0xBB);
    fle_r_stretch(&b4, NULL, 0, 0, 4, 1, &a2, 2, 0, -2, 1, 0xCC, 0, 3);       /* StretchBlt: the sign of the source width and destination width: opposite signs mirror */
    CHECK(fle_r_get(&b4, 0, 0) == 0xBB && fle_r_get(&b4, 1, 0) == 0xBB && fle_r_get(&b4, 2, 0) == 0xAA && fle_r_get(&b4, 3, 0) == 0xAA);
    fle_r_stretch(&b4, NULL, 4, 0, -4, 1, &a2, 0, 0, 2, 1, 0xCC, 0, 3);
    CHECK(fle_r_get(&b4, 0, 0) == 0xBB && fle_r_get(&b4, 3, 0) == 0xAA);
    fle_r_stretch(&b4, NULL, 4, 0, -4, 1, &a2, 2, 0, -2, 1, 0xCC, 0, 3);      /* both negative: not mirrored */
    CHECK(fle_r_get(&b4, 0, 0) == 0xAA && fle_r_get(&b4, 3, 0) == 0xBB);
    fle_r_put(&q4, 0, 0, 0xF0F0F0); fle_r_put(&q4, 1, 0, 0x0FFF0F); fle_r_put(&q4, 2, 0, 0x102030); fle_r_put(&q4, 3, 0, 0x304050);
    fle_r_stretch(&q2, NULL, 0, 0, 2, 1, &q4, 0, 0, 4, 1, 0xCC, 0, 1);        /* BLACKONWHITE: the pixels that are merged are ANDed */
    CHECK(fle_r_get(&q2, 0, 0) == (0xF0F0F0u & 0x0FFF0Fu) && fle_r_get(&q2, 1, 0) == (0x102030u & 0x304050u));
    fle_r_stretch(&q2, NULL, 0, 0, 2, 1, &q4, 0, 0, 4, 1, 0xCC, 0, 2);        /* WHITEONBLACK: ORed */
    CHECK(fle_r_get(&q2, 0, 0) == (0xF0F0F0u | 0x0FFF0Fu) && fle_r_get(&q2, 1, 0) == (0x102030u | 0x304050u));
    fle_r_stretch(&q2, NULL, 0, 0, 2, 1, &q4, 0, 0, 4, 1, 0xCC, 0, 3);        /* COLORONCOLOR: the pixels that are not used are deleted (the one under the centre stays) */
    CHECK(fle_r_get(&q2, 0, 0) == 0x0FFF0Fu && fle_r_get(&q2, 1, 0) == 0x304050u);
    fle_r_stretch(&q2, NULL, 0, 0, 2, 1, &q4, 0, 0, 4, 1, 0xCC, 0, 4);        /* HALFTONE: the mean of the source pixels (choice for the exact rule) */
    m = fle_r_get(&q2, 0, 0);
    CHECK((m & 0xFF) == (0xF0 + 0x0F) / 2 && ((m >> 8) & 0xFF) == (0xF0 + 0xFF) / 2 && ((m >> 16) & 0xFF) == (0xF0 + 0x0F) / 2);
}

static void line(void)
{
    fle_rsurf l = mk(10, 10, 32, 0);
    uint64_t n;

    n = fle_r_line(&l, NULL, 1, 1, 6, 1, 0xFF, 13);                           /* LineTo: "up to, but not including, the specified point"; Lines: the starting pixel is always included */
    CHECK(n == 5);
    for (uint32_t x = 1; x < 6; x++) CHECK(fle_r_get(&l, x, 1) == 0xFF);
    CHECK(fle_r_get(&l, 6, 1) == 0 && fle_r_get(&l, 0, 1) == 0);
    n = fle_r_line(&l, NULL, 2, 5, 2, 2, 0xFF00, 13);
    CHECK(n == 3 && fle_r_get(&l, 2, 5) == 0xFF00 && fle_r_get(&l, 2, 3) == 0xFF00 && fle_r_get(&l, 2, 2) == 0);
    n = fle_r_line(&l, NULL, 5, 5, 5, 5, 1, 13);
    CHECK(n == 0);                                                            /* a zero-length line draws nothing */
    n = fle_r_line(&l, NULL, 0, 8, 6, 9, 0x123456, 13);
    CHECK(n == 6);
    n = fle_r_line(&l, NULL, 0, 0, 4, 4, 0x77, 13);
    CHECK(n == 4 && fle_r_get(&l, 0, 0) == 0x77 && fle_r_get(&l, 3, 3) == 0x77 && fle_r_get(&l, 4, 4) == 0);
}

static void window(void)
{
    fle_rsurf *w = fle_raster_window_surface(7, 16, 8), *w2;

    CHECK(w && w->width == 16 && fle_raster_surface_count() == 1 && fle_raster_nonzero_pixels(w) == 0);   /* a new surface is all 0 (choice: S-surface-init) */
    CHECK(fle_raster_window_surface(7, 0, 8) == NULL && fle_raster_window_surface(7, 20000, 8) == NULL);   /* 0 and above 16384 are refused (choice) */
    fle_r_rop_rect(w, NULL, (fle_rrect){0, 0, 4, 2}, 0xF0, 0x0000FF);
    w2 = fle_raster_window_surface(7, 20, 10);                                /* a resize keeps the overlap (choice: S-surface-resize) */
    CHECK(fle_r_get(w2, 3, 1) == 0x0000FF && fle_r_get(w2, 4, 1) == 0 && w2->width == 20);
    fle_raster_describe(stdout, 7);
}

int main(int argc, char **argv)
{
    static const struct { const char *name; void (*f)(void); } sec[] = {
        { "named", named }, { "binary", binary }, { "surface", surface }, { "blit", blit }, { "stretch", stretch }, { "line", line }, { "window", window },
    };

    if (argc < 2) return 2;
    for (unsigned i = 0; i < sizeof sec / sizeof sec[0]; i++)
        if (strcmp(argv[1], sec[i].name) == 0) {
            sec[i].f();
            printf("ok %s\n", argv[1]);
            return 0;
        }
    return 2;
}
"##;

/// The value of a reverse Polish expression of the page: operands P, S, D (8-bit patterns 0xF0, 0xCC, 0xAA) or the constants 0 and 1; operators a (and), o (or), x (xor), n (not).
fn rpn_value(e: &str, p: u32, s: u32, d: u32, width_mask: u32) -> u32 {
    if e == "0" {
        return 0;
    }
    if e == "1" {
        return width_mask;
    }
    let mut st: Vec<u32> = Vec::new();
    for c in e.chars() {
        match c {
            'P' => st.push(p),
            'S' => st.push(s),
            'D' => st.push(d),
            'n' => {
                let a = st.pop().unwrap();
                st.push(!a & width_mask);
            }
            'a' | 'o' | 'x' => {
                let b = st.pop().unwrap();
                let a = st.pop().unwrap();
                st.push(match c {
                    'a' => a & b,
                    'o' => a | b,
                    _ => a ^ b,
                });
            }
            other => panic!("unknown token {other} in {e}"),
        }
    }
    assert_eq!(st.len(), 1, "{e}");
    st[0] & width_mask
}

fn clang(args: &[&str]) {
    let o = Command::new("clang").args(args).env("TMPDIR", work_dir()).output().expect("clang not found");
    assert!(o.status.success(), "clang {:?}: {}", args, String::from_utf8_lossy(&o.stderr));
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("drv"));
        std::fs::create_dir_all(&dir).unwrap();
        let obj = dir.join("fle_raster.o");
        let c = dir.join("driver.c");
        std::fs::write(&c, DRIVER).unwrap();
        // the unit under test is compiled with warnings as errors
        clang(&["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c", "-I", src_dir().to_str().unwrap(), "-I", runtime_dir().to_str().unwrap(), src_dir().join("fle_raster.c").to_str().unwrap(), "-o", obj.to_str().unwrap()]);
        let exe = dir.join("driver");
        clang(&["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I", src_dir().to_str().unwrap(), "-I", runtime_dir().to_str().unwrap(), c.to_str().unwrap(), obj.to_str().unwrap(), "-o", exe.to_str().unwrap()]);
        exe
    })
}

fn run(section: &str) -> String {
    let o = Command::new(driver()).arg(section).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success() && out.contains(&format!("ok {section}")), "section {section}: status {:?}\nstdout: {out}\nstderr: {}", o.status, String::from_utf8_lossy(&o.stderr));
    out
}

fn bitwise_pattern(v: u32) -> u32 {
    // the 8-bit index of an operand pattern spread to 24 colour bits (one byte per channel)
    v * 0x010101
}

#[test]
fn the_256_codes_of_the_page_are_the_codes_the_model_knows() {
    // every row: high word = index (the page: "the 16-bit operation index"), low word = the page's code; the model's table answers the same code for the same index and accepts only that code
    let mut seen = std::collections::BTreeSet::new();
    let mut c = String::from("#include <stdio.h>\n#include \"fle_raster.h\"\nint main(void){ uint32_t idx; \n");
    for (k, (_e, code)) in ROP3_RPN.iter().enumerate() {
        assert_eq!((code >> 16) as usize, k, "row {k}: the high word is the index");
        assert!(seen.insert(*code));
        c += &format!("if (fle_r_rop3_code({k}) != 0x{code:08X}u || !fle_r_rop3_valid(0x{code:08X}u, &idx) || idx != {k}u) {{ printf(\"BAD {k}\\n\"); return 1; }}\n");
        // a code with the right index and another low word is not valid
        let wrong = code ^ 1;
        c += &format!("if (fle_r_rop3_valid(0x{wrong:08X}u, &idx)) {{ printf(\"BAD-wrong {k}\\n\"); return 1; }}\n");
    }
    c += "printf(\"ok codes\\n\"); return 0; }\n";
    let dir = work_dir().join(etxt_common::per_process("codes"));
    std::fs::create_dir_all(&dir).unwrap();
    let cf = dir.join("codes.c");
    std::fs::write(&cf, c).unwrap();
    // link with the object of the unit (built by driver())
    let _ = driver();
    let obj = work_dir().join(etxt_common::per_process("drv")).join("fle_raster.o");
    let exe = dir.join("codes");
    clang(&["-O0", "-std=gnu11", "-fPIE", "-I", src_dir().to_str().unwrap(), "-I", runtime_dir().to_str().unwrap(), cf.to_str().unwrap(), obj.to_str().unwrap(), "-o", exe.to_str().unwrap()]);
    let o = Command::new(&exe).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("ok codes"), "{out}");
    assert_eq!(seen.len(), 256);
}

#[test]
fn every_ternary_operation_computes_the_boolean_function_of_the_page_on_the_24_colour_bits() {
    // P = pattern (brush), S = source, D = destination; 24 colour bits: for each of the 256 functions and a set of operands the model's result equals the page's RPN expression evaluated bit by bit
    let operands: [(u32, u32, u32); 6] = [(0x000000, 0x000000, 0x000000), (0xFFFFFF, 0xFFFFFF, 0xFFFFFF), (0x123456, 0xABCDEF, 0x0F0F0F), (0xF0F0F0, 0xCCCCCC, 0xAAAAAA), (0x00FF00, 0xFF00FF, 0x0000FF), (0x7F8081, 0x010203, 0xFEFDFC)];
    let mut c = String::from("#include <stdio.h>\n#include \"fle_raster.h\"\nint main(void){\n");
    for (k, (e, _)) in ROP3_RPN.iter().enumerate() {
        for (p, s, d) in operands {
            let want = rpn_value(e, p, s, d, 0xFFFFFF);
            c += &format!("if (fle_r_rop3({k}, 0x{p:06X}u, 0x{s:06X}u, 0x{d:06X}u) != 0x{want:06X}u) {{ printf(\"BAD {k} %06x\\n\", fle_r_rop3({k}, 0x{p:06X}u, 0x{s:06X}u, 0x{d:06X}u)); return 1; }}\n");
        }
        // the index is the function on the predefined values P = 0xF0, S = 0xCC, D = 0xAA (the page)
        assert_eq!(rpn_value(e, 0xF0, 0xCC, 0xAA, 0xFF) as usize, k, "{e}");
        // which operands the function depends on: derived from the index (a function uses S when swapping S changes it for some P, D)
        let f = |p: u32, s: u32, d: u32| rpn_value(e, p, s, d, 1);
        let dep_s = (0..4).any(|m| f(m & 1, 0, m >> 1) != f(m & 1, 1, m >> 1));
        let dep_p = (0..4).any(|m| f(0, m & 1, m >> 1) != f(1, m & 1, m >> 1));
        let dep_d = (0..4).any(|m| f(m & 1, m >> 1, 0) != f(m & 1, m >> 1, 1));
        c += &format!(
            "if (fle_r_idx_uses_source({k}) != {} || fle_r_idx_uses_pattern({k}) != {} || fle_r_idx_uses_dest({k}) != {}) {{ printf(\"BAD-dep {k}\\n\"); return 1; }}\n",
            dep_s as u32, dep_p as u32, dep_d as u32
        );
    }
    c += "printf(\"ok boolean\\n\"); return 0; }\n";
    let dir = work_dir().join(etxt_common::per_process("bool"));
    std::fs::create_dir_all(&dir).unwrap();
    let cf = dir.join("bool.c");
    std::fs::write(&cf, c).unwrap();
    let _ = driver();
    let obj = work_dir().join(etxt_common::per_process("drv")).join("fle_raster.o");
    let exe = dir.join("bool");
    clang(&["-O0", "-std=gnu11", "-fPIE", "-I", src_dir().to_str().unwrap(), "-I", runtime_dir().to_str().unwrap(), cf.to_str().unwrap(), obj.to_str().unwrap(), "-o", exe.to_str().unwrap()]);
    let o = Command::new(&exe).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success() && out.contains("ok boolean"), "{out}");
    let _ = bitwise_pattern(1);
}

#[test]
fn the_named_codes_and_the_flag_bits() {
    run("named");
}

#[test]
fn the_sixteen_binary_modes() {
    run("binary");
}

#[test]
fn surfaces_32_bit_top_down_and_24_bit_bottom_up_with_clipping() {
    run("surface");
}

#[test]
fn blit_overlap_in_both_directions_and_the_unused_byte() {
    run("blit");
}

#[test]
fn stretch_enlarge_mirror_and_the_four_shrink_modes() {
    run("stretch");
}

#[test]
fn lines_include_the_start_and_exclude_the_end() {
    run("line");
}

#[test]
fn window_surfaces_keep_their_overlap_on_resize_and_have_limits() {
    let out = run("window");
    assert!(out.contains("surface of top-level window lid 7: 20 x 10"), "{out}");
}

#[test]
fn the_source_of_the_pixel_model_reads_no_host_state() {
    let s = std::fs::read_to_string(src_dir().join("fle_raster.c")).unwrap();
    let code: String = s.lines().filter(|l| !l.trim_start().starts_with("/*") && !l.trim_start().starts_with("*")).collect::<Vec<_>>().join("\n");
    for bad in ["getenv", "fopen", "/proc", "/dev", "/tmp", "clock_gettime", "gettimeofday", "time(", "X11", "wayland", "dlopen", "pthread", "<unistd.h>", "<sys/", "syscall", "system("] {
        assert!(!code.contains(bad), "fle_raster.c must not use {bad}");
    }
    assert!(Path::new(&src_dir().join("fle_raster.h")).exists());
}
