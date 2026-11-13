//! Tests of decision 0216 stage 3 (opfs1, PROVISIONAL): the pure-C Windows-path -> OPFS-path translation of lift/runtime/fle_opfs_mount.c (not wired into fle_vfs.c yet). No engine, no browser, no OS: the test is a C program
//! that calls fle_opfs_mount_add / fle_opfs_translate / fle_opfs_guest_path with UTF-16 paths and compares the results with the rules of fle_opfs_mount.h (the Windows path rules, from Microsoft Learn "Naming Files, Paths,
//! and Namespaces"; they are a model of Windows, not a measurement of it). FLE_OPFS1_RT_DIR (a copy of the two files) replaces their directory for mutation runs.

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
/// The directory of fle_opfs_mount.c / .h: harness/web/opfs1, deliberately NOT runtime/ (build_nmod_wasm.sh compiles every runtime/fle_*.c into the engine, and many tests link them all): the files move to runtime/ when fle_vfs.c uses them.
fn rt() -> PathBuf {
    match std::env::var_os("FLE_OPFS1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../../archive/harness-attic/web/opfs1"),
    }
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("opfs1map");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const TEST_C: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <uchar.h>
#include "fle_opfs_mount.h"

static int fails = 0, total = 0;
#define U(s) ((const uint16_t *)u##s)
static size_t len16(const uint16_t *s) { size_t n = 0; while (s[n]) n++; return n; }
static int eq16(const uint16_t *a, const uint16_t *b) { return len16(a) == len16(b) && memcmp(a, b, len16(a) * 2) == 0; }
static void check(const char *name, int cond) { total++; if (!cond) { fails++; printf("FAIL %s\n", name); } else printf("ok %s\n", name); }

/* translate; returns the result code, `out` = the OPFS path */
static char out[2048];
static int tr(const uint16_t *p) { out[0] = 0; return fle_opfs_translate(p, len16(p), out, sizeof out); }
static int trs(const char *name, const uint16_t *p, int rc, const char *want)
{
    int got = tr(p);
    int ok = got == rc && (rc != FLE_OPFS_MAPPED || strcmp(out, want) == 0);
    if (!ok) printf("   %s: rc=%d (want %d) out='%s' (want '%s')\n", name, got, rc, out, want ? want : "-");
    check(name, ok);
    return got;
}
static uint16_t big[40000];
static int trn(const char *name, size_t n, int rc)
{
    int got = fle_opfs_translate(big, n, out, sizeof out);
    if (got != rc) printf("   %s: rc=%d (want %d)\n", name, got, rc);
    check(name, got == rc);
    return got;
}
static size_t fill(const char *prefix, char c, size_t n)
{
    size_t i = 0, k;
    for (k = 0; prefix[k]; k++) big[i++] = (uint16_t)prefix[k];
    for (k = 0; k < n; k++) big[i++] = (uint16_t)c;
    big[i] = 0;
    return i;
}

int main(void)
{
    uint16_t g[600];
    int rc, i;
    const char *IL = "fl/C/Users/user/Documents/Image-Line";

    fle_opfs_mount_clear();
    check("count0", fle_opfs_mount_count() == 0);
    trs("empty-table", U("C:\\Users\\user\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    check("add-IL", fle_opfs_mount_add(U("C:\\Users\\user\\Documents\\Image-Line"), IL) == 0);
    check("add-IL-trailing-backslash-is-same", fle_opfs_mount_add(U("c:/users/USER/documents/image-line/"), IL) == 0);   /* a second slot with the same prefix: harmless */
    fle_opfs_mount_clear();
    check("add-IL", fle_opfs_mount_add(U("C:\\Users\\user\\Documents\\Image-Line"), IL) == 0);
    check("add-Data", fle_opfs_mount_add(U("C:\\Users\\user\\Documents\\Image-Line\\Data"), "fl/data") == 0);
    check("add-D-root", fle_opfs_mount_add(U("D:\\"), "") == 0);
    check("count3", fle_opfs_mount_count() == 3);

    /* basic mapping, the mount's own folder, the case of the rest is kept */
    trs("map-file", U("C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\x.fst"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/FL Studio/x.fst");
    trs("map-root-of-mount", U("C:\\Users\\user\\Documents\\Image-Line"), FLE_OPFS_MAPPED, IL);
    trs("map-root-of-mount-trailing", U("C:\\Users\\user\\Documents\\Image-Line\\"), FLE_OPFS_MAPPED, IL);
    trs("map-case-insensitive-prefix", U("c:\\USERS\\User\\documents\\IMAGE-line\\MixedCase.Fst"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/MixedCase.Fst");
    trs("map-forward-slashes", U("C:/Users/user/Documents/Image-Line/a/b"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a/b");
    trs("map-repeated-separators", U("C:\\\\Users\\\\user\\/\\Documents\\Image-Line\\\\a\\\\\\b"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a/b");
    trs("map-trailing-sep-kept-out", U("C:\\Users\\user\\Documents\\Image-Line\\a\\"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a");

    /* not mounted: the volume handles it */
    trs("not-other-folder", U("C:\\Users\\user\\Documents\\Other\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-whole-component", U("C:\\Users\\user\\Documents\\Image-Line2\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-parent-of-mount", U("C:\\Users\\user\\Documents"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-other-drive", U("E:\\Users\\user\\Documents\\Image-Line\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-relative", U("Image-Line\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-drive-relative", U("C:Users\\user"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-unc", U("\\\\server\\share\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-device-namespace", U("\\\\.\\C:\\a"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("not-empty", U(""), FLE_OPFS_NOT_MOUNTED, NULL);

    /* longest prefix wins */
    trs("longest-prefix", U("C:\\Users\\user\\Documents\\Image-Line\\Data\\p\\q.wav"), FLE_OPFS_MAPPED, "fl/data/p/q.wav");
    trs("longest-prefix-not-for-sibling", U("C:\\Users\\user\\Documents\\Image-Line\\Database\\q.wav"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/Database/q.wav");

    /* . and .. are resolved before the lookup */
    trs("dot", U("C:\\Users\\user\\Documents\\Image-Line\\.\\a\\.\\b"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a/b");
    trs("dotdot-inside", U("C:\\Users\\user\\Documents\\Image-Line\\a\\..\\b"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/b");
    trs("dotdot-leaves-mount", U("C:\\Users\\user\\Documents\\Image-Line\\..\\Other\\x"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("dotdot-leaves-and-returns", U("C:\\Users\\user\\Documents\\Image-Line\\..\\..\\Documents\\Image-Line\\x"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/x");
    trs("dotdot-above-drive-root", U("C:\\..\\..\\Users\\user\\Documents\\Image-Line\\x"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/x");
    trs("dotdot-into-other-mount", U("C:\\Users\\user\\Documents\\Image-Line\\Data\\..\\x"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/x");
    trs("dotdot-to-drive-root", U("D:\\a\\..\\.."), FLE_OPFS_MAPPED, "");

    /* trailing dots and spaces are stripped (Win32), a name of only dots is invalid */
    trs("trailing-dot", U("C:\\Users\\user\\Documents\\Image-Line\\a.b."), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a.b");
    trs("trailing-space", U("C:\\Users\\user\\Documents\\Image-Line\\a.b "), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a.b");
    trs("trailing-dot-space-mixed", U("C:\\Users\\user\\Documents\\Image-Line\\a . .\\b"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a/b");
    trs("only-dots", U("C:\\Users\\user\\Documents\\Image-Line\\...\\b"), FLE_OPFS_E_INVALID, NULL);
    trs("only-spaces", U("C:\\Users\\user\\Documents\\Image-Line\\  \\b"), FLE_OPFS_E_INVALID, NULL);
    trs("leading-dot-kept", U("C:\\Users\\user\\Documents\\Image-Line\\.hidden"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/.hidden");
    trs("inner-space-kept", U("C:\\Users\\user\\Documents\\Image-Line\\a b  c"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a b  c");

    /* invalid characters */
    {
        const char *bad = "<>\"|?*";
        char nm[64];
        for (i = 0; bad[i]; i++) {
            uint16_t p[80]; size_t k = 0; const char *pre = "C:\\Users\\user\\Documents\\Image-Line\\a";
            while (*pre) p[k++] = (uint16_t)*pre++;
            p[k++] = (uint16_t)bad[i]; p[k++] = 'b'; p[k] = 0;
            sprintf(nm, "invalid-char-%c", bad[i]);
            check(nm, fle_opfs_translate(p, k, out, sizeof out) == FLE_OPFS_E_INVALID);
        }
    }
    trs("invalid-colon-in-name", U("C:\\Users\\user\\Documents\\Image-Line\\a:stream"), FLE_OPFS_E_INVALID, NULL);
    trs("invalid-control", U("C:\\Users\\user\\Documents\\Image-Line\\a\x01" "b"), FLE_OPFS_E_INVALID, NULL);
    trs("invalid-outside-mount-is-the-volumes", U("C:\\Users\\other\\a<b"), FLE_OPFS_NOT_MOUNTED, NULL);
    trs("invalid-under-root-mount", U("D:\\a\\b|c"), FLE_OPFS_E_INVALID, NULL);

    /* device names */
    trs("device-CON", U("C:\\Users\\user\\Documents\\Image-Line\\CON"), FLE_OPFS_E_DEVICE, NULL);
    trs("device-nul-ext", U("C:\\Users\\user\\Documents\\Image-Line\\a\\nul.txt"), FLE_OPFS_E_DEVICE, NULL);
    trs("device-Com1", U("C:\\Users\\user\\Documents\\Image-Line\\Com1"), FLE_OPFS_E_DEVICE, NULL);
    trs("device-LPT9-space", U("C:\\Users\\user\\Documents\\Image-Line\\LPT9 .x"), FLE_OPFS_E_DEVICE, NULL);
    trs("device-AUX-in-folder", U("C:\\Users\\user\\Documents\\Image-Line\\aux\\file"), FLE_OPFS_E_DEVICE, NULL);
    trs("not-device-COM0", U("C:\\Users\\user\\Documents\\Image-Line\\COM0"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/COM0");
    trs("not-device-com10", U("C:\\Users\\user\\Documents\\Image-Line\\com10"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/com10");
    trs("not-device-CONSOLE", U("C:\\Users\\user\\Documents\\Image-Line\\CONSOLE"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/CONSOLE");
    trs("not-device-prefix", U("C:\\Users\\user\\Documents\\Image-Line\\xnul"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/xnul");
    trs("device-outside-mount-is-the-volumes", U("C:\\Users\\user\\Documents\\Other\\CON"), FLE_OPFS_NOT_MOUNTED, NULL);

    /* lengths */
    {
        size_t n = fill("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\", 'a', 255);
        trn("component-255-ok-long-form", n, FLE_OPFS_MAPPED);
        n = fill("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\", 'a', 256);
        trn("component-256-too-long-long-form", n, FLE_OPFS_E_TOO_LONG);
        n = fill("C:\\Users\\user\\Documents\\Image-Line\\", 'a', 223);
        trn("component-223-in-259-ok", n, FLE_OPFS_MAPPED);
        n = fill("C:\\Users\\user\\Documents\\Image-Line\\", 'a', 255);
        trn("component-255-in-short-path-too-long", n, FLE_OPFS_E_TOO_LONG);
        /* 259 = MAX_PATH - 1 units in all */
        n = fill("C:\\Users\\user\\Documents\\Image-Line\\", 'a', 259 - strlen("C:\\Users\\user\\Documents\\Image-Line\\"));
        check("len-259-built", n == 259);
        trn("path-259-ok", n, FLE_OPFS_MAPPED);
        n = fill("C:\\Users\\user\\Documents\\Image-Line\\", 'a', 260 - strlen("C:\\Users\\user\\Documents\\Image-Line\\"));
        trn("path-260-too-long", n, FLE_OPFS_E_TOO_LONG);
        /* the \\?\ prefix lifts it: 260 + 4 */
        {
            uint16_t tmp[400]; size_t k; for (k = 0; k <= n; k++) tmp[k] = big[k];
            big[0] = '\\'; big[1] = '\\'; big[2] = '?'; big[3] = '\\';
            for (k = 0; k <= n; k++) big[4 + k] = tmp[k];
            trn("long-prefix-260-ok", n + 4, FLE_OPFS_MAPPED);
        }
        /* 32767 units after the prefix: a component limit of 255 still holds, so build it of folders */
        {
            size_t o = 0, k; const char *pre = "\\\\?\\C:\\Users\\user\\Documents\\Image-Line";
            for (k = 0; pre[k]; k++) big[o++] = (uint16_t)pre[k];
            while (o + 202 < 32767 + 4) { size_t j; big[o++] = '\\'; for (j = 0; j < 200; j++) big[o++] = 'd'; }
            big[o] = 0;
            check("long-path-near-32767-built", o > 32000 && o <= 32767 + 4);
            trn("long-path-near-32767-ok-but-no-space-for-out", o, FLE_OPFS_E_SPACE);   /* `out` is 2048 bytes: the error is the buffer's, not the path's */
            {
                static char bigout[70000];
                rc = fle_opfs_translate(big, o, bigout, sizeof bigout);
                check("long-path-near-32767-mapped-with-big-buffer", rc == FLE_OPFS_MAPPED && strlen(bigout) > 32000);
            }
            while (o < 32767 + 4 + 10) { size_t j; big[o++] = '\\'; for (j = 0; j < 100; j++) big[o++] = 'd'; }   /* folders, so that no single component is the fault */
            big[o] = 0;
            trn("long-path-over-32767", o, FLE_OPFS_E_TOO_LONG);
            check("long-path-over-32767-outside-mount-is-the-volumes", ({ big[4] = 'E'; fle_opfs_translate(big, o, out, sizeof out); }) == FLE_OPFS_NOT_MOUNTED);
        }
    }
    trs("long-form-keeps-trailing-dot", U("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\a."), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a.");
    trs("long-form-dot-refused", U("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\.\\a"), FLE_OPFS_E_INVALID, NULL);
    trs("long-form-dotdot-refused", U("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\..\\a"), FLE_OPFS_E_INVALID, NULL);
    trs("long-form-device-is-a-file", U("\\\\?\\C:\\Users\\user\\Documents\\Image-Line\\CON"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/CON");
    trs("long-form-case-insensitive", U("\\\\?\\c:\\users\\USER\\Documents\\image-line\\a"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/a");

    /* UTF-8 of the components: 2, 3 and 4 bytes; a lone surrogate is invalid */
    trs("utf8-2byte", U("C:\\Users\\user\\Documents\\Image-Line\\caf\u00e9"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/caf\xC3\xA9");
    trs("utf8-3byte", U("C:\\Users\\user\\Documents\\Image-Line\\\u3042\u3044.flp"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/\xE3\x81\x82\xE3\x81\x84.flp");
    trs("utf8-4byte-surrogate-pair", U("C:\\Users\\user\\Documents\\Image-Line\\\U00020BB7"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/\xF0\xA0\xAE\xB7");
    {
        uint16_t lone_hi[] = { 'C', ':', '\\', 'U', 's', 'e', 'r', 's', '\\', 'u', 's', 'e', 'r', '\\', 'D', 'o', 'c', 'u', 'm', 'e', 'n', 't', 's', '\\', 'I', 'm', 'a', 'g', 'e', '-', 'L', 'i', 'n', 'e', '\\', 'a', 0xD800, 'b' };
        uint16_t lone_lo[] = { 'C', ':', '\\', 'U', 's', 'e', 'r', 's', '\\', 'u', 's', 'e', 'r', '\\', 'D', 'o', 'c', 'u', 'm', 'e', 'n', 't', 's', '\\', 'I', 'm', 'a', 'g', 'e', '-', 'L', 'i', 'n', 'e', '\\', 'a', 0xDC00, 'b' };
        check("lone-high-surrogate-invalid", fle_opfs_translate(lone_hi, sizeof lone_hi / 2, out, sizeof out) == FLE_OPFS_E_INVALID);
        check("lone-low-surrogate-invalid", fle_opfs_translate(lone_lo, sizeof lone_lo / 2, out, sizeof out) == FLE_OPFS_E_INVALID);
    }
    trs("case-fold-greek-cyrillic", U("C:\\Users\\user\\Documents\\Image-Line\\\u03a9\u0416"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/\xCE\xA9\xD0\x96");

    /* the output buffer */
    {
        const uint16_t *p = U("C:\\Users\\user\\Documents\\Image-Line\\a.flp");
        size_t want = strlen("fl/C/Users/user/Documents/Image-Line/a.flp");
        char small[128];
        check("buffer-exact", fle_opfs_translate(p, len16(p), small, want + 1) == FLE_OPFS_MAPPED && strlen(small) == want);
        check("buffer-one-short", fle_opfs_translate(p, len16(p), small, want) == FLE_OPFS_E_SPACE);
        check("buffer-zero", fle_opfs_translate(p, len16(p), small, 0) == FLE_OPFS_E_SPACE);
    }

    /* a mount of the OPFS root: the page's own folder "fl" is not there for the guest */
    trs("D-root", U("D:\\"), FLE_OPFS_MAPPED, "");
    trs("D-file", U("D:\\Songs\\a.flp"), FLE_OPFS_MAPPED, "Songs/a.flp");
    trs("D-fl-hidden", U("D:\\fl\\state"), FLE_OPFS_E_HIDDEN, NULL);
    trs("D-FL-hidden", U("d:\\FL"), FLE_OPFS_E_HIDDEN, NULL);
    trs("D-fl-via-dotdot-hidden", U("D:\\Songs\\..\\fl\\license"), FLE_OPFS_E_HIDDEN, NULL);
    trs("D-flx-visible", U("D:\\flx\\a"), FLE_OPFS_MAPPED, "flx/a");
    trs("D-sub-fl-visible", U("D:\\Songs\\fl"), FLE_OPFS_MAPPED, "Songs/fl");
    trs("D-long-form-fl-hidden", U("\\\\?\\D:\\fl\\x"), FLE_OPFS_E_HIDDEN, NULL);
    trs("C-mount-opfs-prefix-fl-not-hidden", U("C:\\Users\\user\\Documents\\Image-Line\\fl"), FLE_OPFS_MAPPED, "fl/C/Users/user/Documents/Image-Line/fl");

    /* the reverse */
    rc = fle_opfs_guest_path(0, "fl/C/Users/user/Documents/Image-Line/FL Studio/x.fst", g, 600);
    check("rev-IL", rc > 0 && eq16(g, U("C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\x.fst")) && (size_t)rc == len16(g));
    rc = fle_opfs_guest_path(0, IL, g, 600);
    check("rev-IL-root", rc > 0 && eq16(g, U("C:\\Users\\user\\Documents\\Image-Line")));
    check("rev-IL-sibling-prefix-refused", fle_opfs_guest_path(0, "fl/C/Users/user/Documents/Image-Line2/a", g, 600) == FLE_OPFS_E_INVALID);
    check("rev-IL-other-refused", fle_opfs_guest_path(0, "fl/other", g, 600) == FLE_OPFS_E_INVALID);
    rc = fle_opfs_guest_path(1, "fl/data/p/q.wav", g, 600);
    check("rev-Data", rc > 0 && eq16(g, U("C:\\Users\\user\\Documents\\Image-Line\\Data\\p\\q.wav")));
    rc = fle_opfs_guest_path(2, "Songs/\xE3\x81\x82.flp", g, 600);
    check("rev-D-utf8", rc > 0 && eq16(g, U("D:\\Songs\\\u3042.flp")));
    rc = fle_opfs_guest_path(2, "\xF0\xA0\xAE\xB7", g, 600);
    check("rev-D-surrogates", rc == 5 && eq16(g, U("D:\\\U00020BB7")));
    rc = fle_opfs_guest_path(2, "", g, 600);
    check("rev-D-root", rc > 0 && eq16(g, U("D:\\")));
    check("rev-D-fl-hidden", fle_opfs_guest_path(2, "fl/state", g, 600) == FLE_OPFS_E_HIDDEN);
    check("rev-D-fl-hidden-bare", fle_opfs_guest_path(2, "fl", g, 600) == FLE_OPFS_E_HIDDEN);
    check("rev-D-flx-visible", fle_opfs_guest_path(2, "flx/a", g, 600) > 0);
    check("rev-bad-index", fle_opfs_guest_path(3, "a", g, 600) == FLE_OPFS_E_INVALID && fle_opfs_guest_path(-1, "a", g, 600) == FLE_OPFS_E_INVALID);
    check("rev-bad-utf8", fle_opfs_guest_path(2, "a\xC3", g, 600) == FLE_OPFS_E_INVALID);
    check("rev-space", fle_opfs_guest_path(2, "Songs/a.flp", g, 6) == FLE_OPFS_E_SPACE);
    /* round trip */
    {
        const char *paths[] = { "Songs/a.flp", "x", "\xE3\x81\x82/\xF0\xA0\xAE\xB7/z.wav", "a b/c.d/e" };
        for (i = 0; i < 4; i++) {
            char back[256];
            rc = fle_opfs_guest_path(2, paths[i], g, 600);
            check("roundtrip-guest-path", rc > 0);
            check("roundtrip-translate", fle_opfs_translate(g, len16(g), back, sizeof back) == FLE_OPFS_MAPPED && strcmp(back, paths[i]) == 0);
        }
    }

    /* mount_add errors */
    check("add-relative-refused", fle_opfs_mount_add(U("Users\\a"), "x") == FLE_OPFS_E_INVALID);
    check("add-device-refused", fle_opfs_mount_add(U("C:\\aux"), "x") == FLE_OPFS_E_INVALID);
    check("add-bad-char-refused", fle_opfs_mount_add(U("C:\\a|b"), "x") == FLE_OPFS_E_INVALID);
    {
        size_t n = fill("C:\\", 'a', 0), k;
        for (k = 0; k < 6; k++) { size_t j; big[n++] = '\\'; for (j = 0; j < 100; j++) big[n++] = 'x'; }
        big[n] = 0;
        check("add-too-long-refused", fle_opfs_mount_add(big, "x") == FLE_OPFS_E_INVALID || fle_opfs_mount_add(big, "x") == FLE_OPFS_E_TOO_LONG);
    }
    fle_opfs_mount_clear();
    for (i = 0; i < FLE_OPFS_MAX_MOUNTS; i++) { uint16_t p[8] = { 'C', ':', '\\', (uint16_t)('a' + i), 0 }; if (fle_opfs_mount_add(p, "x") != 0) break; }
    check("table-full-at-16", i == FLE_OPFS_MAX_MOUNTS && fle_opfs_mount_add(U("C:\\zz"), "x") == -5);
    fle_opfs_mount_clear();
    check("clear-count0", fle_opfs_mount_count() == 0);
    trs("clear-not-mounted", U("C:\\a\\b"), FLE_OPFS_NOT_MOUNTED, NULL);

    printf("TOTAL %d FAILS %d\n", total, fails);
    return fails ? 1 : 0;
}
"#;

#[test]
fn opfs1_mount_translate() {
    let dir = work();
    let c = dir.join("map_test.c");
    std::fs::write(&c, TEST_C).unwrap();
    let exe = dir.join("map_test");
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE"])
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_opfs_mount.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "compile failed:\n{}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().unwrap();
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let fails: Vec<&str> = out.lines().filter(|l| l.starts_with("FAIL") || l.starts_with("   ")).collect();
    assert!(run.status.success() && fails.is_empty(), "failed checks:\n{}\n--- full output ---\n{}", fails.join("\n"), out);
    let total: usize = out.lines().find(|l| l.starts_with("TOTAL")).and_then(|l| l.split_whitespace().nth(1)).and_then(|n| n.parse().ok()).expect("no TOTAL line");
    assert!(total >= 120, "expected at least 120 checks, ran {total}");
    eprintln!("opfs1 map: {total} checks");
}
