//! Tests for decision 0111 (PROVISIONAL, docs/decisions/0111-gdi-bulk.md): the DC attributes (current position, graphic modes, origins), the DC stack
//! (SaveDC, RestoreDC), solid brushes, pens, palette entries, icon and cursor objects, the cursor state, GetClipBox, the pixel operations on DIB sections
//! (FillRect, PatBlt, BitBlt(SRCCOPY)), and the 69 named stops, all of runtime/fle_gdi.c.
//!
//! Expected values are written here from the decision and the Learn pages it cites (MoveToEx, GetCurrentPositionEx, SetBkMode, SetROP2, SetMapMode,
//! SetStretchBltMode, GetStretchBltMode, SetWindowOrgEx, GetWindowOrgEx, SetViewportOrgEx, LPtoDP, SetBrushOrgEx, GetBrushOrgEx, SaveDC, RestoreDC, GdiFlush,
//! GetClipBox, UnrealizeObject, CreateSolidBrush, CreatePenIndirect, LOGPEN, GetPaletteEntries, DestroyIcon, DestroyCursor, GetIconInfo, SetCursor, GetCursor,
//! ShowCursor, PatBlt, FillRect, BitBlt, Display Device Context Defaults, Mapping Modes and Translations, Default Transformations), not from the C code. A
//! number the pages do not give (the first state number of SaveDC, the unused byte of a 32-bit pixel, the numbers of the constants) is marked "choice" or
//! "unofficial" and is the only thing such a test pins down. The host is Linux: nothing is compared with a real Windows. The tests were written in the first
//! stage WITHOUT being run (no compiler was allowed to build them); the C of every test program was checked with `clang -fsyntax-only`.
//!
//! The C sources of fle_gdi.c / fle_gdi.h come from runtime/; FLE_GDI_SRC_DIR (a directory with fle_gdi.c and fle_gdi.h) overrides it (the mutation runs).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_GDI_SRC_DIR") {
        return PathBuf::from(d);
    }
    runtime_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-gdi/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// fle_gdi.c (warnings are errors) and the files it links with: fle_gdc.c (the virtual screen DC), fle_imports.c (GetLastError), fle_env.c (fle_stop) and
/// fle_cursor.c (the standard cursors that SetCursor accepts; fle_gdi.c refers to it weakly).
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-gdi-bulk"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_gdi.c", true), ("fle_raster.c", true), ("fle_gdc.c", false), ("fle_imports.c", false), ("fle_env.c", false), ("fle_cursor.c", false)] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

pub struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

/// The C every test program starts with. K(f, a, b, ...) calls a runtime function like the guest does: the first four arguments in rcx, rdx, r8, r9, the fifth
/// and later on the stack where fle_arg looks for them. STOP(label, statements) runs the statements in a forked child and prints "label | signal | first line
/// of stderr" (signal 0 and an empty message: the statements returned); the parent's state is not changed by the child.
const BULK_TOP: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/wait.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_cursor.h"

uint64_t R[16];
static uint64_t stk[32];

static uint64_t cN(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    for (int i = 4; i < 13; i++) stk[i + 1] = a[i];    /* the fifth to the thirteenth argument: [rsp+0x28] ... (13 for StretchDIBits, decision 0113) */
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define K(f, ...) cN((void (*)(void))(f), (uint64_t[13]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
#define S(x) ((long long)(int32_t)(uint32_t)(x))
#define A(p) ((uint64_t)(uintptr_t)(p))
#define NEG(n) ((uint64_t)(int64_t)(n))
#define INREG(h) ((h) > FLE_GDI_HANDLE_BASE && (h) < FLE_GDI_HANDLE_BASE + FLE_GDI_REGION_SIZE)

static int pfd[2];
static pid_t spawn(void)
{
    pid_t p;
    fflush(stdout);
    fflush(stderr);
    if (pipe(pfd)) exit(2);
    p = fork();
    if (p == 0) { close(pfd[0]); dup2(pfd[1], 2); close(pfd[1]); }
    else close(pfd[1]);
    return p;
}
static void reap(const char *label, pid_t p)
{
    char buf[900], junk[256];
    ssize_t n = 0, k;
    int st = 0;
    char *nl;
    /* merge fix: read to EOF. One read closed the pipe while the child was still writing a long message in pieces: SIGPIPE (13) instead of SIGABRT (6). */
    while ((k = read(pfd[0], n < (ssize_t)sizeof buf - 1 ? buf + n : junk, n < (ssize_t)sizeof buf - 1 ? sizeof buf - 1 - (size_t)n : sizeof junk)) > 0)
        if (n < (ssize_t)sizeof buf - 1) n += k;
    buf[n] = 0;
    nl = strchr(buf, '\n');
    if (nl) *nl = 0;
    close(pfd[0]);
    waitpid(p, &st, 0);
    printf("%s | %d | %s\n", label, WIFSIGNALED(st) ? WTERMSIG(st) : 0, buf);
}
#define STOP(label, ...) do { pid_t p_ = spawn(); if (p_ == 0) { __VA_ARGS__; _exit(0); } reap(label, p_); } while (0)

static void (*thr_fn)(void);
static void *thr_main(void *a) { (void)a; K(thr_fn, 0, 0, 0, 0); return 0; }
static void on_second_thread(void (*f)(void)) { pthread_t t; thr_fn = f; pthread_create(&t, 0, thr_main, 0); pthread_join(t, 0); }

#define SCR() K(fle_imp_GetDC, 0)
#define MKDC() K(fle_imp_CreateCompatibleDC, 0)
#define DELDC(h) ((uint32_t)K(fle_imp_DeleteDC, (h)))
#define SELO(dc, h) K(fle_imp_SelectObject, (dc), (h))
#define MKBMP(w, h) K(fle_imp_CreateBitmap, (w), (h), 1, 1)
#define CCB(dc, w, h) K(fle_imp_CreateCompatibleBitmap, (dc), (w), (h))
#define DEL(h) ((uint32_t)K(fle_imp_DeleteObject, (h)))
#define KIND(h) fle_gdi_kind(h)
#define SELN(h) ((unsigned)fle_gdi_selected_count(h))
#define SOLID(c) K(fle_imp_CreateSolidBrush, (c))

static void attr(const char *label, uint64_t h)
{
    int32_t o[13] = { 0 };
    int k = fle_gdi_dc_attr(h, o);
    printf("%s %d |", label, k);
    for (int i = 0; i < 13; i++) printf(" %d", (int)o[i]);
    printf("\n");
}

static uint8_t lb[16];
static uint64_t mkbrush(uint32_t style, uint32_t color)
{
    uint64_t hatch = 0;
    memset(lb, 0, sizeof lb);
    memcpy(lb, &style, 4);
    memcpy(lb + 4, &color, 4);
    memcpy(lb + 8, &hatch, 8);
    return K(fle_imp_CreateBrushIndirect, A(lb));
}

static uint8_t lpn[16];
static uint64_t PEN(uint32_t style, int32_t width, uint32_t color)
{
    memset(lpn, 0, sizeof lpn);
    memcpy(lpn, &style, 4);
    memcpy(lpn + 4, &width, 4);
    memcpy(lpn + 12, &color, 4);
    return K(fle_imp_CreatePenIndirect, A(lpn));
}

/* a LOGPALETTE of 16 entries; byte i of the 64 entry bytes is i * 3 + 1 */
static uint8_t lpal[4 + 64];
static uint64_t MKPAL(void)
{
    memset(lpal, 0, sizeof lpal);
    lpal[0] = 0; lpal[1] = 3; lpal[2] = 16; lpal[3] = 0;
    for (unsigned i = 0; i < 64; i++) lpal[4 + i] = (uint8_t)(i * 3 + 1);
    return K(fle_imp_CreatePalette, A(lpal));
}

static uint8_t bmi[40 + 12 + 4 * 256 + 16];
static uint64_t bits_out;
static void p32(unsigned o, uint32_t v) { memcpy(bmi + o, &v, 4); }
static void mkbmi(int32_t w, int32_t h, unsigned bpp, uint32_t comp)
{
    uint16_t one = 1, b = (uint16_t)bpp;
    memset(bmi, 0, sizeof bmi);
    p32(0, 40); p32(4, (uint32_t)w); p32(8, (uint32_t)h); memcpy(bmi + 12, &one, 2); memcpy(bmi + 14, &b, 2); p32(16, comp);
}
static uint64_t MKDIB(uint64_t dc, int32_t w, int32_t h, unsigned bpp)
{
    mkbmi(w, h, bpp, 0);
    bits_out = 0;
    return K(fle_imp_CreateDIBSection, dc, A(bmi), 0, A(&bits_out), 0, 0);
}
static uint64_t MKDIBF(uint64_t dc, int32_t w, int32_t h, unsigned bpp, uint32_t m0, uint32_t m1, uint32_t m2)
{
    mkbmi(w, h, bpp, 3);
    p32(40, m0); p32(44, m1); p32(48, m2);
    bits_out = 0;
    return K(fle_imp_CreateDIBSection, dc, A(bmi), 0, A(&bits_out), 0, 0);
}
static uint8_t *BITS(uint64_t h)
{
    uint64_t b = 0;
    uint32_t o[6];
    return fle_gdi_dib_info(h, &b, o) ? (uint8_t *)(uintptr_t)b : NULL;
}
static void FILL(uint64_t h, int v)
{
    uint64_t b = 0;
    uint32_t o[6];
    if (fle_gdi_dib_info(h, &b, o)) memset((void *)(uintptr_t)b, v, (size_t)o[4] * o[1]);
}
/* the pixel bytes of every row in memory order (the padding of a row is not printed) */
static void dump(const char *label, uint64_t h)
{
    uint64_t b = 0;
    uint32_t o[6];
    if (!fle_gdi_dib_info(h, &b, o)) { printf("%s not a dib\n", label); return; }
    for (unsigned r = 0; r < o[1]; r++) {
        const uint8_t *p = (const uint8_t *)(uintptr_t)b + (size_t)r * o[4];
        printf("%s %u ", label, r);
        for (unsigned i = 0; i < o[0] * o[2] / 8; i++) printf("%02x", p[i]);
        printf("\n");
    }
}

static uint32_t rcv[4];
static uint64_t RCT(int32_t l, int32_t t, int32_t r, int32_t b)
{
    rcv[0] = (uint32_t)l; rcv[1] = (uint32_t)t; rcv[2] = (uint32_t)r; rcv[3] = (uint32_t)b;
    return A(rcv);
}
#define FILLRECT(dc, l, t, r, b, hbr) K(fle_imp_FillRect, (dc), RCT(l, t, r, b), (hbr))
#define PATBLT(dc, x, y, w, h, rop) K(fle_imp_PatBlt, (dc), (x), (y), (w), (h), (rop))
#define BITBLT(dc, x, y, w, h, sdc, sx, sy, rop) K(fle_imp_BitBlt, (dc), (x), (y), (w), (h), (sdc), (sx), (sy), (rop))

/* decision 0113: the source image of StretchDIBits (a header, 40 bytes + 3 masks, and 8 x 8 x 4 bytes of bits) and the call */
static uint8_t sbmi[40 + 12];
static uint8_t simg[512];
static void mksrc(int32_t w, int32_t h, unsigned bpp, uint32_t comp)
{
    uint16_t one = 1, b = (uint16_t)bpp;
    uint32_t v = 40;
    memset(sbmi, 0, sizeof sbmi);
    memcpy(sbmi, &v, 4); v = (uint32_t)w; memcpy(sbmi + 4, &v, 4); v = (uint32_t)h; memcpy(sbmi + 8, &v, 4);
    memcpy(sbmi + 12, &one, 2); memcpy(sbmi + 14, &b, 2); memcpy(sbmi + 16, &comp, 4);
    for (unsigned i = 0; i < sizeof simg; i++) simg[i] = (uint8_t)(i + 1);
}
#define SDIB(dc, xd, yd, wd, hd, xs, ys, ws, hs, usage, rop) K(fle_imp_StretchDIBits, (dc), (xd), (yd), (wd), (hd), (xs), (ys), (ws), (hs), A(simg), A(sbmi), (usage), (rop))

static uint8_t iio[32];
static uint64_t ICONI(uint32_t ficon, uint32_t hx, uint32_t hy, uint64_t mask, uint64_t color)
{
    uint8_t ii[32];
    memset(ii, 0, sizeof ii);
    memcpy(ii, &ficon, 4); memcpy(ii + 4, &hx, 4); memcpy(ii + 8, &hy, 4);
    memcpy(ii + 16, &mask, 8); memcpy(ii + 24, &color, 8);
    return K(fle_imp_CreateIconIndirect, A(ii));
}
"#;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{BULK_TOP}\n{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

/// The lines a program prints; the program must exit normally.
fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, "", body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements, word of the message) must stop the run with SIGABRT, a "fle: <api>:" message that contains `word`. `setup` runs once in the
/// parent first; every case runs in a forked child (STOP in BULK_TOP).
/// Merge fix (the first version ran `setup` in the parent): a forked child has its own thread id, and a runtime call in the child after the parent's `setup`
/// made a call stops it as "called from a second thread" (the single-thread rule of decision 0017). So `setup` runs in every child, before the statements.
fn stops(prefix: &str, setup: &str, api: &str, cases: &[(String, String, String)]) {
    let mut body = String::new();
    for (label, stmts, _) in cases {
        body.push_str(&format!("    STOP(\"{label}\", {setup}\n    {stmts});\n"));
    }
    let l = lines(prefix, &body);
    assert_eq!(l.len(), cases.len(), "{prefix}: {l:?}");
    for (line, (label, _, word)) in l.iter().zip(cases) {
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        assert_eq!(parts.len(), 3, "{label}: {line}");
        assert_eq!(parts[0], label, "{line}");
        assert_eq!(parts[1], "6", "{label}: not SIGABRT (the call returned or died otherwise): {line}");
        assert!(parts[2].starts_with(&format!("fle: {api}: ")), "{label}: not a 'fle: {api}:' message: {line}");
        assert!(parts[2].contains(word.as_str()), "{label}: message lacks {word:?}: {line}");
    }
}

fn case(label: &str, stmts: &str, word: &str) -> (String, String, String) {
    (label.to_string(), stmts.to_string(), word.to_string())
}

/// The four handles that every DC call refuses: a screen DC (sdc), NULL, a deleted memory DC (dcx), a bitmap (b). `call` has `@` where the hdc goes.
fn dc_refusals(call: &str) -> Vec<(String, String, String)> {
    vec![
        case("screen", &call.replace('@', "sdc"), "GetDC(NULL)"),
        case("null", &call.replace('@', "0"), "not a DC of this runtime"),
        case("deleted", &call.replace('@', "dcx"), "deleted by DeleteDC"),
        case("bitmap", &call.replace('@', "b"), "not a DC of this runtime"),
    ]
}

/// Decision 0121: the DCs that the attribute calls (the 14 functions of decision 0111 that took a memory DC only) still refuse: NULL, a deleted memory DC, a bitmap. A screen DC (GetDC(NULL)) is not
/// among them any more: it answers the attribute calls with the state of a new display DC (tests/wall_fr7.rs); the calls that stay memory-DC only (GetClipBox, the pixel operations) keep `dc_refusals`.
fn attr_dc_refusals(call: &str) -> Vec<(String, String, String)> {
    // decision 0125 (old: the list kept "null" and expected "not a DC of this runtime"): an attribute call with hdc NULL now fails (0, last error 6, S-hdc-null) and does not stop: tests/wall_fr10.rs
    // decision 0129 (old: the list kept "deleted" and expected "deleted by DeleteDC"): an attribute call with the handle of a memory DC that DeleteDC has deleted now fails like hdc NULL (0, last error 6, S-hdc-deleted)
    // and does not stop: tests/wall_fr12.rs (an_attribute_call_with_the_handle_of_a_deleted_memory_dc_...). The drawing calls (dc_refusals) still stop for it.
    dc_refusals(call).into_iter().filter(|(label, _, _)| label != "screen" && label != "null" && label != "deleted").collect()
}

const DC_SETUP: &str = "    uint64_t sdc = SCR(), dc = MKDC(), dcx = MKDC(), b = MKBMP(4, 4);\n    int32_t q[4] = { 0, 0, 0, 0 };\n    DELDC(dcx);";

/// The value of `#define NAME <number>` (decimal, 0x.. hex, with or without u/UINT64_C) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    let rest = rest.strip_prefix("UINT64_C(").unwrap_or(rest);
    if let Some(h) = rest.strip_prefix("0x") {
        let tok: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// C text without comments (strings are kept).
fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

/// The (dll, name) of every row of TABLE in imports_table.rs, in order.
fn table_rows() -> Vec<(String, String)> {
    let t = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let a = t.find("const TABLE").unwrap();
    let b = a + t[a..].find("\n];").unwrap();
    t[a..b]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("(\"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, _) = rest.split_once("\", \"")?;
            Some((dll.to_string(), name.to_string()))
        })
        .collect()
}

// ---- 1. Table, header, constants ----

/// The 27 rows that answer, in the order they are appended to imports_table.rs (after the 19 rows of decision 0108).
const ANSWERS_0111: [(&str, &str); 27] = [
    ("gdi32.dll", "MoveToEx"),
    ("gdi32.dll", "GetCurrentPositionEx"),
    ("gdi32.dll", "SetBkMode"),
    ("gdi32.dll", "SetROP2"),
    ("gdi32.dll", "SetMapMode"),
    ("gdi32.dll", "SetStretchBltMode"),
    ("gdi32.dll", "GetStretchBltMode"),
    ("gdi32.dll", "SetWindowOrgEx"),
    ("gdi32.dll", "GetWindowOrgEx"),
    ("gdi32.dll", "SetViewportOrgEx"),
    ("gdi32.dll", "LPtoDP"),
    ("gdi32.dll", "SetBrushOrgEx"),
    ("gdi32.dll", "GetBrushOrgEx"),
    ("gdi32.dll", "SaveDC"),
    ("gdi32.dll", "RestoreDC"),
    ("gdi32.dll", "GdiFlush"),
    ("gdi32.dll", "GetClipBox"),
    ("gdi32.dll", "UnrealizeObject"),
    ("gdi32.dll", "CreateSolidBrush"),
    ("gdi32.dll", "CreatePenIndirect"),
    ("gdi32.dll", "GetPaletteEntries"),
    ("user32.dll", "DestroyIcon"),
    ("user32.dll", "DestroyCursor"),
    ("user32.dll", "GetIconInfo"),
    ("user32.dll", "SetCursor"),
    ("user32.dll", "GetCursor"),
    ("user32.dll", "ShowCursor"),
];

/// The 69 rows that stop by name (class C), with the word their message must contain, in the order of the rows after the 27 answers.
const NAMED_STOPS_0111: [(&str, &str, &str); 69] = [
    ("gdi32.dll", "LineTo", "needs the rasterizer"),
    ("gdi32.dll", "Rectangle", "needs the rasterizer"),
    ("gdi32.dll", "RoundRect", "needs the rasterizer"),
    ("gdi32.dll", "Ellipse", "needs the rasterizer"),
    ("gdi32.dll", "Pie", "needs the rasterizer"),
    ("gdi32.dll", "Chord", "needs the rasterizer"),
    ("gdi32.dll", "Arc", "needs the rasterizer"),
    ("gdi32.dll", "ArcTo", "needs the rasterizer"),
    ("gdi32.dll", "AngleArc", "needs the rasterizer"),
    ("gdi32.dll", "Polyline", "needs the rasterizer"),
    ("gdi32.dll", "Polygon", "needs the rasterizer"),
    ("gdi32.dll", "PolyBezier", "needs the rasterizer"),
    ("gdi32.dll", "PolyBezierTo", "needs the rasterizer"),
    ("gdi32.dll", "ExtFloodFill", "needs the rasterizer"),
    ("gdi32.dll", "FrameRgn", "needs the rasterizer"),
    ("gdi32.dll", "GetPixel", "pixel transfer or read-back"),
    ("gdi32.dll", "SetPixel", "pixel transfer or read-back"),
    ("gdi32.dll", "SetPixelV", "pixel transfer or read-back"),
    ("gdi32.dll", "GetBitmapBits", "pixel transfer or read-back"),
    ("gdi32.dll", "SetDIBits", "pixel transfer or read-back"),
    ("gdi32.dll", "SetDIBitsToDevice", "pixel transfer or read-back"),
    ("gdi32.dll", "StretchDIBits", "pixel transfer"),   // decision 0113 wall 6: a same-format 1:1 SRCCOPY is answered; called with zeros it stops in pixel_stop ("only SRCCOPY")
    ("gdi32.dll", "StretchBlt", "pixel transfer"),   // g1-optional-stretchblt.diff: StretchBlt of equal sizes is BitBlt; the stop is the one of blit / pixel_stop
    ("gdi32.dll", "MaskBlt", "pixel transfer or read-back"),
    ("gdi32.dll", "CreateDIBitmap", "pixel transfer or read-back"),
    ("gdi32.dll", "ExtTextOutW", "glyph rasterization"),
    ("gdi32.dll", "GetTextMetricsW", "text metrics"),
    ("gdi32.dll", "GetTextExtentPoint32W", "text metrics"),
    ("gdi32.dll", "GetTextExtentPointW", "text metrics"),
    ("gdi32.dll", "SelectClipRgn", "clip regions"),
    ("gdi32.dll", "ExtSelectClipRgn", "clip regions"),
    ("gdi32.dll", "IntersectClipRect", "clip regions"),
    ("gdi32.dll", "ExcludeClipRect", "clip regions"),
    ("gdi32.dll", "GetClipRgn", "clip regions"),
    ("gdi32.dll", "RectVisible", "clip regions"),
    ("gdi32.dll", "GetRandomRgn", "clip regions"),
    ("gdi32.dll", "CreateRectRgn", "clip regions"),
    ("gdi32.dll", "CreateRectRgnIndirect", "clip regions"),
    ("gdi32.dll", "SetRectRgn", "clip regions"),
    ("gdi32.dll", "CombineRgn", "clip regions"),
    ("gdi32.dll", "GetRgnBox", "clip regions"),
    ("gdi32.dll", "OffsetRgn", "clip regions"),
    ("gdi32.dll", "EqualRgn", "clip regions"),
    ("gdi32.dll", "RectInRegion", "clip regions"),
    ("gdi32.dll", "GetRegionData", "clip regions"),
    ("gdi32.dll", "GetNearestPaletteIndex", "system palette"),
    ("gdi32.dll", "ResizePalette", "system palette"),
    ("gdi32.dll", "GetSystemPaletteEntries", "system palette"),
    ("gdi32.dll", "GetDCOrgEx", "final translation origin"),
    ("gdi32.dll", "GetEnhMetaFileHeader", "metafile"),
    ("gdi32.dll", "PlayEnhMetaFile", "metafile"),
    ("gdi32.dll", "GetEnhMetaFileBits", "metafile"),
    ("gdi32.dll", "GetEnhMetaFilePaletteEntries", "metafile"),
    ("gdi32.dll", "GetWinMetaFileBits", "metafile"),
    ("gdi32.dll", "GetEnhMetaFileDescriptionW", "metafile"),
    ("gdi32.dll", "DeleteEnhMetaFile", "metafile"),
    ("gdi32.dll", "SetEnhMetaFileBits", "metafile"),
    ("gdi32.dll", "CopyEnhMetaFileW", "metafile"),
    ("gdi32.dll", "SetWinMetaFileBits", "metafile"),
    ("user32.dll", "FrameRect", "needs the rasterizer"),
    ("user32.dll", "DrawEdge", "needs the rasterizer"),
    ("user32.dll", "DrawFocusRect", "needs the rasterizer"),
    ("user32.dll", "DrawFrameControl", "needs the rasterizer"),
    ("user32.dll", "DrawTextW", "glyph rasterization"),
    ("user32.dll", "DrawTextExW", "glyph rasterization"),
    ("user32.dll", "DrawIcon", "needs the rasterizer"),
    ("user32.dll", "DrawIconEx", "needs the rasterizer"),
    ("user32.dll", "CreateIcon", "pixel transfer or read-back"),
    ("user32.dll", "LoadBitmapW", "pixel transfer or read-back"),
];

#[test]
fn the_ninety_six_rows_are_appended_in_order_after_the_older_rows_and_declared() {
    let rows = table_rows();
    let h = std::fs::read_to_string(src_dir().join("fle_gdi.h")).unwrap();
    let first = rows.iter().position(|(_, n)| n == "MoveToEx").expect("MoveToEx row");
    // the table had 331 rows before decision 0111 (312 + the 19 rows of decision 0108): the new rows come after them (decision 0015: rows are only appended,
    // a pseudo-VA never moves)
    assert!(first >= 331, "the first row of decision 0111 is row {first}");
    let new: Vec<(String, String)> = ANSWERS_0111.iter().map(|(d, n)| (d.to_string(), n.to_string())).chain(NAMED_STOPS_0111.iter().map(|(d, n, _)| (d.to_string(), n.to_string()))).collect();
    assert_eq!(new.len(), 96);
    assert_eq!(&rows[first..first + 96], &new[..], "the 96 rows follow one another in the decided order");
    for (dll, n) in &new {
        assert_eq!(rows.iter().filter(|(d, m)| d == dll && m == n).count(), 1, "{n}: exactly one row");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_gdi.h");
    }
    // the last row of decision 0108 is directly before the first of decision 0111, and the older rows of this file keep their places
    assert_eq!(rows[first - 1].1, "RealizePalette");
    for old in ["CreatePalette", "DeleteObject", "GetStockObject", "LoadIconW", "CreateFontIndirectW", "GetObjectW", "CreateBitmap", "CreateCompatibleDC", "BitBlt", "PatBlt", "FillRect", "GetDIBits"] {
        let k = rows.iter().position(|(_, n)| n == old).unwrap_or_else(|| panic!("{old} row"));
        assert!(k < first, "{old} (row {k}) must stay before the first row of decision 0111 (row {first})");
    }
    // ("SendMessageW" was in the list below until decision 0119 (wall 9: runtime/fle_win.c answers it, tests/wall_fr6.rs); old: asserted to have no row)
    // the neighbours that decision 0111 does not answer and does not name: no row (not imported by ILTools, or window / message / menu / clipboard calls)
    // Decision 0123 (old: "ShowWindow", "BeginPaint", "InvalidateRect", "GetClientRect" and "IsWindow" were in this list and asserted to have no row at all): the window model of wall 24 answers them with rows appended at the end
    // of the table; what decision 0111 pins is that none of them is one of ITS rows, so they must lie after the 96 rows (and each has exactly one row)
    // decision 0126 (wall 55): old: "CreatePen" was in this list; it is a row now, appended at the end of the table (not one of the 96 rows of decision 0111: it lies after them, the loop below)
    for n in ["GetCurrentObject", "TextOutW", "GetBkMode", "GetROP2", "GetMapMode", "SetTextAlign", "SetPolyFillMode", "SetWindowExtEx", "SetViewportExtEx", "CreateFontW", "CreateHatchBrush", "ExtCreatePen", "ImageList_Create", "TrackPopupMenu"] {   // old: "MessageBeep" was the last name of this list; decision 0136 (runtime/fle_input.c) answers it with a row appended at the end of the table
        assert!(!rows.iter().any(|(_, m)| m == n), "{n} must not have a row in decision 0111");
    }
    for n in ["ShowWindow", "BeginPaint", "InvalidateRect", "GetClientRect", "IsWindow", "CreatePen"] {
        let at: Vec<usize> = rows.iter().enumerate().filter(|(_, (_, m))| m == n).map(|(k, _)| k).collect();
        assert!(at.iter().all(|k| *k >= first + 96), "{n} must not be one of the 96 rows of decision 0111: rows {at:?}");
        assert!(at.len() <= 1, "{n}: at most one row");
    }
}

#[test]
fn the_constants_of_decision_0111() {
    // the numbers are from the Windows SDK header copy of mingw-w64 (wingdi.h): UNOFFICIAL; the Learn pages give names only
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_KIND_PEN"), 12);
    assert_eq!((def_const("fle_gdi.h", "FLE_GDI_TRANSPARENT"), def_const("fle_gdi.h", "FLE_GDI_OPAQUE")), (1, 2));
    assert_eq!((def_const("fle_gdi.h", "FLE_GDI_R2_BLACK"), def_const("fle_gdi.h", "FLE_GDI_R2_COPYPEN"), def_const("fle_gdi.h", "FLE_GDI_R2_WHITE")), (1, 13, 16));
    assert_eq!((def_const("fle_gdi.h", "FLE_GDI_MM_TEXT"), def_const("fle_gdi.h", "FLE_GDI_MM_ANISOTROPIC")), (1, 8));
    assert_eq!(
        (def_const("fle_gdi.h", "FLE_GDI_BLACKONWHITE"), def_const("fle_gdi.h", "FLE_GDI_WHITEONBLACK"), def_const("fle_gdi.h", "FLE_GDI_COLORONCOLOR"), def_const("fle_gdi.h", "FLE_GDI_HALFTONE")),
        (1, 2, 3, 4)
    );
    assert_eq!((def_const("fle_gdi.h", "FLE_GDI_PS_SOLID"), def_const("fle_gdi.h", "FLE_GDI_PS_NULL"), def_const("fle_gdi.h", "FLE_GDI_PS_INSIDEFRAME")), (0, 5, 6));
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_SIMPLEREGION"), 2);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BRUSHORG_MAX"), 7);
    // LOGPEN = UINT + POINT (2 x LONG) + COLORREF; PALETTEENTRY = 4 BYTE
    assert_eq!(4 + 2 * 4 + 4, 16);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_LOGPEN_SIZE"), 16);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_PALETTEENTRY_SIZE"), 4);
    // raster operation codes (Learn BitBlt / PatBlt name them; the numbers are the SDK header's)
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_SRCCOPY"), 0x00CC0020);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_PATCOPY"), 0x00F00021);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_BLACKNESS"), 0x42);
    assert_eq!(def_const("fle_gdi.h", "FLE_GDI_WHITENESS"), 0x00FF0062);
}

// ---- 2. Attributes of a memory DC ----

#[test]
fn a_new_memory_dc_starts_with_the_defaults_of_the_display_device_context_page() {
    // Learn "Display Device Context Defaults": background mode OPAQUE (2), drawing mode R2_COPYPEN (13), mapping mode MM_TEXT (1), stretch mode BLACKONWHITE
    // (1), current pen position (0,0), brush origin (0,0), viewport origin (0,0), window origin (0,0); no saved state
    let l = lines("bulk-defaults", r#"
    uint64_t dc = MKDC(), dc2 = MKDC();
    attr("dc", dc);
    attr("dc2", dc2);
    attr("not a dc", MKBMP(2, 2));
    attr("null", 0);
"#);
    assert_eq!(l, ["dc 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0", "dc2 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0", "not a dc 0 | 0 0 0 0 0 0 0 0 0 0 0 0 0", "null 0 | 0 0 0 0 0 0 0 0 0 0 0 0 0"]);
}

#[test]
fn move_to_and_the_current_position_follow_the_pages() {
    // MoveToEx: "updates the current position to the specified point and optionally returns the previous position"; lppt NULL: "the previous position is not returned"
    let l = lines("bulk-moveto", r#"
    uint64_t dc = MKDC(), dc2 = MKDC();
    int32_t p[2] = { 77, 88 };
    uint64_t r1 = K(fle_imp_MoveToEx, dc, 10, 20, A(p));
    printf("move %llu prev %d %d\n", U(r1), (int)p[0], (int)p[1]);
    p[0] = 77; p[1] = 88;
    uint64_t r2 = K(fle_imp_MoveToEx, dc, NEG(-5), 7, 0);
    printf("move2 %llu untouched %d %d\n", U(r2), (int)p[0], (int)p[1]);
    uint64_t r3 = K(fle_imp_GetCurrentPositionEx, dc, A(p));
    printf("get %llu pos %d %d\n", U(r3), (int)p[0], (int)p[1]);
    uint64_t r4 = K(fle_imp_MoveToEx, dc, 0xAAAAAAAA00000003ull, 0xBBBBBBBB00000004ull, A(p));
    printf("lowdword %llu prev %d %d\n", U(r4), (int)p[0], (int)p[1]);
    attr("dc", dc);
    attr("dc2", dc2);
"#);
    assert_eq!(
        l,
        [
            "move 1 prev 0 0",
            "move2 1 untouched 77 88",
            "get 1 pos -5 7",
            "lowdword 1 prev -5 7",
            "dc 1 | 2 13 1 1 3 4 0 0 0 0 0 0 0",
            "dc2 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
        ]
    );
}

#[test]
fn the_graphic_modes_return_the_previous_mode_and_start_at_the_defaults() {
    let l = lines("bulk-modes", r#"
    uint64_t dc = MKDC(), dc2 = MKDC();
    uint64_t a1 = K(fle_imp_SetBkMode, dc, 1), a2 = K(fle_imp_SetBkMode, dc, 2), a3 = K(fle_imp_SetBkMode, dc, 0xFFFFFFFF00000001ull);
    printf("bk %llu %llu %llu\n", U(a1), U(a2), U(a3));
    uint64_t b1 = K(fle_imp_SetROP2, dc, 7), b2 = K(fle_imp_SetROP2, dc, 16), b3 = K(fle_imp_SetROP2, dc, 1);
    printf("rop2 %llu %llu %llu\n", U(b1), U(b2), U(b3));
    uint64_t c1 = K(fle_imp_SetMapMode, dc, 8), c2 = K(fle_imp_SetMapMode, dc, 3), c3 = K(fle_imp_SetMapMode, dc, 1);
    printf("map %llu %llu %llu\n", U(c1), U(c2), U(c3));
    uint64_t d1 = K(fle_imp_SetStretchBltMode, dc, 4), d2 = K(fle_imp_GetStretchBltMode, dc), d3 = K(fle_imp_SetStretchBltMode, dc, 3), d4 = K(fle_imp_GetStretchBltMode, dc);
    printf("stretch %llu %llu %llu %llu\n", U(d1), U(d2), U(d3), U(d4));
    attr("dc", dc);
    attr("dc2", dc2);
    printf("flush %llu\n", U(K(fle_imp_GdiFlush, 0)));
"#);
    assert_eq!(
        l,
        [
            "bk 2 1 2",
            "rop2 13 7 16",
            "map 1 8 3",
            "stretch 1 4 4 3",
            "dc 1 | 1 1 1 3 0 0 0 0 0 0 0 0 0",
            "dc2 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "flush 1",
        ]
    );
}

#[test]
fn a_mode_the_pages_do_not_list_or_a_dc_that_is_not_modelled_stops_by_name() {
    let mut v = vec![case("zero", "K(fle_imp_SetBkMode, dc, 0)", "neither OPAQUE"), case("three", "K(fle_imp_SetBkMode, dc, 3)", "neither OPAQUE")];
    v.extend(attr_dc_refusals("K(fle_imp_SetBkMode, @, 1)"));
    stops("bulk-mode-stops-bk", DC_SETUP, "SetBkMode", &v);
    let mut v = vec![case("zero", "K(fle_imp_SetROP2, dc, 0)", "not one of the 16 mix modes"), case("seventeen", "K(fle_imp_SetROP2, dc, 17)", "not one of the 16 mix modes")];
    v.extend(attr_dc_refusals("K(fle_imp_SetROP2, @, 1)"));
    stops("bulk-mode-stops-rop2", DC_SETUP, "SetROP2", &v);
    let mut v = vec![case("zero", "K(fle_imp_SetMapMode, dc, 0)", "not one of the 8 mapping modes"), case("nine", "K(fle_imp_SetMapMode, dc, 9)", "not one of the 8 mapping modes")];
    v.extend(attr_dc_refusals("K(fle_imp_SetMapMode, @, 1)"));
    stops("bulk-mode-stops-map", DC_SETUP, "SetMapMode", &v);
    let mut v = vec![case("zero", "K(fle_imp_SetStretchBltMode, dc, 0)", "not one of the 4 modes"), case("five", "K(fle_imp_SetStretchBltMode, dc, 5)", "not one of the 4 modes")];
    v.extend(attr_dc_refusals("K(fle_imp_SetStretchBltMode, @, 1)"));
    stops("bulk-mode-stops-stretch", DC_SETUP, "SetStretchBltMode", &v);
    stops("bulk-mode-stops-get", DC_SETUP, "GetStretchBltMode", &attr_dc_refusals("K(fle_imp_GetStretchBltMode, @)"));
}

#[test]
fn the_window_and_viewport_origins_and_lptodp_follow_the_page_formula() {
    // Learn "Mapping Modes and Translations": Dx = ((Lx - WOx) * VEx / WEx) + VOx; MM_TEXT has the extents 1 and 1 ("Default Transformations"), so Dx = Lx - WOx + VOx
    let l = lines("bulk-origins", r#"
    uint64_t dc = MKDC(), dc2 = MKDC();
    int32_t p[2] = { 9, 9 };
    uint64_t r1 = K(fle_imp_SetWindowOrgEx, dc, 5, 6, A(p));
    printf("wo %llu prev %d %d\n", U(r1), (int)p[0], (int)p[1]);
    uint64_t r2 = K(fle_imp_SetViewportOrgEx, dc, 100, 200, A(p));
    printf("vo %llu prev %d %d\n", U(r2), (int)p[0], (int)p[1]);
    uint64_t r3 = K(fle_imp_SetViewportOrgEx, dc, 300, 400, A(p));
    printf("vo2 %llu prev %d %d\n", U(r3), (int)p[0], (int)p[1]);
    uint64_t r4 = K(fle_imp_SetViewportOrgEx, dc, 100, 200, 0);
    uint64_t r5 = K(fle_imp_GetWindowOrgEx, dc, A(p));
    printf("null %llu get %llu %d %d\n", U(r4), U(r5), (int)p[0], (int)p[1]);
    int32_t pts[4] = { 10, 10, 0, 0 };
    uint64_t r6 = K(fle_imp_LPtoDP, dc, A(pts), 2);
    printf("lp %llu %d %d %d %d\n", U(r6), (int)pts[0], (int)pts[1], (int)pts[2], (int)pts[3]);
    attr("dc", dc);
    int32_t q[2] = { -134217728, 134217727 };
    uint64_t r7 = K(fle_imp_LPtoDP, dc2, A(q), 1);
    printf("edge %llu %d %d\n", U(r7), (int)q[0], (int)q[1]);
    /* the origins are independent of the mapping mode (Mapping Modes and Translations: "Changing a mapping mode does not affect the currently set origins") */
    K(fle_imp_SetMapMode, dc2, 2);
    K(fle_imp_SetWindowOrgEx, dc2, 1, 2, 0);
    attr("dc2", dc2);
"#);
    assert_eq!(
        l,
        [
            "wo 1 prev 0 0",
            "vo 1 prev 0 0",
            "vo2 1 prev 100 200",
            "null 1 get 1 5 6",
            "lp 1 105 204 95 194",
            "dc 1 | 2 13 1 1 0 0 5 6 100 200 0 0 0",
            "edge 1 -134217728 134217727",
            "dc2 1 | 2 13 2 1 0 0 1 2 0 0 0 0 0",
        ]
    );
}

#[test]
fn lptodp_and_the_origin_calls_stop_where_the_pages_leave_the_result_open() {
    let mut v = vec![
        case("over-27-bits-positive", "q[0] = 134217728; K(fle_imp_LPtoDP, dc, A(q), 1)", "exceeds 27 bits"),
        case("over-27-bits-negative", "q[1] = -134217729; K(fle_imp_LPtoDP, dc, A(q), 1)", "exceeds 27 bits"),
        case("count-zero", "K(fle_imp_LPtoDP, dc, A(q), 0)", "c is not positive"),
        case("count-negative", "K(fle_imp_LPtoDP, dc, A(q), NEG(-1))", "c is not positive"),
        case("null-points", "K(fle_imp_LPtoDP, dc, 0, 1)", "lppt is NULL"),
        case("other-mapping-mode", "K(fle_imp_SetMapMode, dc, 2); K(fle_imp_LPtoDP, dc, A(q), 1)", "not MM_TEXT"),
    ];
    v.extend(attr_dc_refusals("K(fle_imp_LPtoDP, @, A(q), 1)"));
    stops("bulk-lptodp-stops", DC_SETUP, "LPtoDP", &v);
    stops("bulk-wo-stops", DC_SETUP, "SetWindowOrgEx", &attr_dc_refusals("K(fle_imp_SetWindowOrgEx, @, 1, 2, 0)"));
    stops("bulk-vo-stops", DC_SETUP, "SetViewportOrgEx", &attr_dc_refusals("K(fle_imp_SetViewportOrgEx, @, 1, 2, 0)"));
    let mut g = vec![case("null-point", "K(fle_imp_GetWindowOrgEx, dc, 0)", "lppoint is NULL")];
    g.extend(attr_dc_refusals("K(fle_imp_GetWindowOrgEx, @, A(q))"));
    stops("bulk-getwo-stops", DC_SETUP, "GetWindowOrgEx", &g);
    let mut m = vec![case("null-point", "K(fle_imp_GetCurrentPositionEx, dc, 0)", "lppt is NULL")];
    m.extend(attr_dc_refusals("K(fle_imp_GetCurrentPositionEx, @, A(q))"));
    stops("bulk-getpos-stops", DC_SETUP, "GetCurrentPositionEx", &m);
    stops("bulk-moveto-stops", DC_SETUP, "MoveToEx", &attr_dc_refusals("K(fle_imp_MoveToEx, @, 1, 2, 0)"));
}

#[test]
fn the_brush_origin_is_kept_for_values_zero_to_seven() {
    // SetBrushOrgEx: lppt "receives the previous brush origin"; GetBrushOrgEx: "values between 0 and 7"; "The default brush origin coordinates are (0,0)"
    let l = lines("bulk-brushorg", r#"
    uint64_t dc = MKDC();
    int32_t p[2] = { 9, 9 };
    uint64_t r1 = K(fle_imp_SetBrushOrgEx, dc, 3, 4, A(p));
    printf("set %llu prev %d %d\n", U(r1), (int)p[0], (int)p[1]);
    uint64_t r2 = K(fle_imp_GetBrushOrgEx, dc, A(p));
    printf("get %llu %d %d\n", U(r2), (int)p[0], (int)p[1]);
    uint64_t r3 = K(fle_imp_SetBrushOrgEx, dc, 7, 0, 0);
    uint64_t r4 = K(fle_imp_GetBrushOrgEx, dc, A(p));
    printf("null-lppt %llu get %llu %d %d\n", U(r3), U(r4), (int)p[0], (int)p[1]);
    attr("dc", dc);
"#);
    assert_eq!(l, ["set 1 prev 0 0", "get 1 3 4", "null-lppt 1 get 1 7 0", "dc 1 | 2 13 1 1 0 0 0 0 0 0 7 0 0"]);
    let mut v = vec![
        case("x-eight", "K(fle_imp_SetBrushOrgEx, dc, 8, 0, 0)", "outside 0 to 7"),
        case("y-eight", "K(fle_imp_SetBrushOrgEx, dc, 0, 8, 0)", "outside 0 to 7"),
        case("x-negative", "K(fle_imp_SetBrushOrgEx, dc, NEG(-1), 0, 0)", "outside 0 to 7"),
        case("y-negative", "K(fle_imp_SetBrushOrgEx, dc, 0, NEG(-1), 0)", "outside 0 to 7"),
    ];
    v.extend(attr_dc_refusals("K(fle_imp_SetBrushOrgEx, @, 1, 1, 0)"));
    stops("bulk-brushorg-stops", DC_SETUP, "SetBrushOrgEx", &v);
    let mut g = vec![case("null-point", "K(fle_imp_GetBrushOrgEx, dc, 0)", "lppt is NULL")];
    g.extend(attr_dc_refusals("K(fle_imp_GetBrushOrgEx, @, A(q))"));
    stops("bulk-getbrushorg-stops", DC_SETUP, "GetBrushOrgEx", &g);
}

// ---- 3. SaveDC, RestoreDC ----

#[test]
fn save_dc_pushes_a_state_and_restore_dc_pops_it_with_the_ones_above() {
    // SaveDC: "the return value identifies the saved state" (the first is 1: a choice); RestoreDC: positive = a specific instance, negative = relative ("-1
    // restores the most recently saved state"), the restored state "and any subsequent states" are popped
    let l = lines("bulk-savedc", r#"
    uint64_t dc = MKDC(), dc2 = MKDC(), dc3 = MKDC(), b1 = MKBMP(4, 4), b2 = MKBMP(8, 8);
    SELO(dc, b1);
    uint64_t s1 = K(fle_imp_SaveDC, dc);
    SELO(dc, b2);
    K(fle_imp_SetBkMode, dc, 1);
    K(fle_imp_SetROP2, dc, 7);
    K(fle_imp_SetViewportOrgEx, dc, 9, 9, 0);
    uint64_t s2 = K(fle_imp_SaveDC, dc);
    K(fle_imp_SetMapMode, dc, 3);
    printf("saved %llu %llu\n", U(s1), U(s2));
    attr("top", dc);
    printf("sel before %u %u\n", SELN(b1), SELN(b2));
    uint64_t r1 = K(fle_imp_RestoreDC, dc, NEG(-1));
    printf("restore -1: %llu\n", U(r1));
    attr("after-1", dc);
    uint64_t r2 = K(fle_imp_RestoreDC, dc, 1);
    printf("restore 1: %llu\n", U(r2));
    attr("after1", dc);
    uint64_t cur = SELO(dc, b1);
    printf("sel after %u %u current is b1 %d\n", SELN(b1), SELN(b2), cur == b1);
    uint64_t n1 = K(fle_imp_RestoreDC, dc, 1), n2 = K(fle_imp_RestoreDC, dc, NEG(-1)), n3 = K(fle_imp_RestoreDC, dc, 5);
    printf("no state: %llu %llu %llu\n", U(n1), U(n2), U(n3));
    uint64_t o1 = K(fle_imp_SaveDC, dc2), o2 = K(fle_imp_RestoreDC, dc2, 2);
    printf("own stack: %llu %llu\n", U(o1), U(o2));
    uint64_t t1 = K(fle_imp_SaveDC, dc3), t2 = K(fle_imp_SaveDC, dc3), t3 = K(fle_imp_SaveDC, dc3);
    uint64_t q1 = K(fle_imp_RestoreDC, dc3, 2);
    int32_t o[13];
    fle_gdi_dc_attr(dc3, o);
    uint64_t q2 = K(fle_imp_RestoreDC, dc3, 2);
    printf("triple %llu %llu %llu restore2 %llu depth %d again %llu\n", U(t1), U(t2), U(t3), U(q1), (int)o[12], U(q2));
"#);
    assert_eq!(
        l,
        [
            "saved 1 2",
            "top 1 | 1 7 3 1 0 0 0 0 9 9 0 0 2",
            "sel before 0 1",
            "restore -1: 1",
            "after-1 1 | 1 7 1 1 0 0 0 0 9 9 0 0 1",
            "restore 1: 1",
            "after1 1 | 2 13 1 1 0 0 0 0 0 0 0 0 0",
            "sel after 1 0 current is b1 1",
            "no state: 0 0 0",
            "own stack: 1 0",
            "triple 1 2 3 restore2 1 depth 1 again 0",
        ]
    );
}

#[test]
fn restore_dc_stops_where_the_pages_leave_the_state_open() {
    let setup = "    uint64_t sdc = SCR(), dc = MKDC(), dcx = MKDC(), b = MKBMP(4, 4), dcb = MKDC(), b2 = MKBMP(2, 2), br1 = SOLID(1), br2 = SOLID(2);\n    DELDC(dcx);\n    K(fle_imp_SaveDC, dc);";
    let mut v = vec![
        case("text-colour", "K(fle_imp_SetTextColor, dc, 5); K(fle_imp_RestoreDC, dc, NEG(-1))", "text colour or the background colour"),
        case("background-colour", "K(fle_imp_SetBkColor, dc, 5); K(fle_imp_RestoreDC, dc, NEG(-1))", "text colour or the background colour"),
        case("position", "K(fle_imp_MoveToEx, dc, 1, 1, 0); K(fle_imp_RestoreDC, dc, NEG(-1))", "current position differs"),
        case("brush-origin", "K(fle_imp_SetBrushOrgEx, dc, 1, 1, 0); K(fle_imp_RestoreDC, dc, NEG(-1))", "brush origin differs"),
        case("zero", "K(fle_imp_RestoreDC, dc, 0)", "nSavedDC is 0"),
        case(
            "deleted-object",
            "SELO(dcb, br1); K(fle_imp_SaveDC, dcb); SELO(dcb, br2); DEL(br1); K(fle_imp_RestoreDC, dcb, NEG(-1))",
            "was deleted since",
        ),
        case(
            "bitmap-held-elsewhere",
            "SELO(dcb, b2); K(fle_imp_SaveDC, dcb); SELO(dcb, b); SELO(dc, b2); K(fle_imp_RestoreDC, dcb, NEG(-1))",
            "another DC",
        ),
    ];
    v.extend(attr_dc_refusals("K(fle_imp_RestoreDC, @, 1)"));
    stops("bulk-restore-stops", setup, "RestoreDC", &v);
    stops("bulk-save-stops", setup, "SaveDC", &attr_dc_refusals("K(fle_imp_SaveDC, @)"));
}

// ---- 4. GetClipBox ----

#[test]
fn get_clip_box_gives_the_bitmap_of_the_memory_dc_in_logical_units() {
    // GetClipBox: the rectangle of the visible area in logical units, the return value SIMPLEREGION (2) for a single rectangle. A memory DC has no clip region
    // here and no window, so the visible area is the selected bitmap (a reading of the memory DC page)
    let l = lines("bulk-clipbox", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    int32_t r[4] = { 9, 9, 9, 9 };
    uint64_t a = K(fle_imp_GetClipBox, dc, A(r));
    printf("placeholder %llu %d %d %d %d\n", U(a), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
    uint64_t b8 = MKBMP(8, 6);
    SELO(dc, b8);
    a = K(fle_imp_GetClipBox, dc, A(r));
    printf("bitmap %llu %d %d %d %d\n", U(a), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
    K(fle_imp_SetWindowOrgEx, dc, 3, 4, 0);
    K(fle_imp_SetViewportOrgEx, dc, 1, 1, 0);
    a = K(fle_imp_GetClipBox, dc, A(r));
    printf("origins %llu %d %d %d %d\n", U(a), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
    K(fle_imp_SetWindowOrgEx, dc, 0, 0, 0);
    K(fle_imp_SetViewportOrgEx, dc, 0, 0, 0);
    uint64_t d = MKDIB(sdc, 5, 3, 24);
    SELO(dc, d);
    a = K(fle_imp_GetClipBox, dc, A(r));
    printf("dib %llu %d %d %d %d\n", U(a), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
    uint64_t c = CCB(sdc, 20, 10);
    SELO(dc, c);
    a = K(fle_imp_GetClipBox, dc, A(r));
    printf("colour %llu %d %d %d %d\n", U(a), (int)r[0], (int)r[1], (int)r[2], (int)r[3]);
"#);
    assert_eq!(l, ["placeholder 2 0 0 1 1", "bitmap 2 0 0 8 6", "origins 2 2 3 10 9", "dib 2 0 0 5 3", "colour 2 0 0 20 10"]);
    let mut v = vec![
        case("other-mapping-mode", "K(fle_imp_SetMapMode, dc, 7); K(fle_imp_GetClipBox, dc, A(q))", "not MM_TEXT"),
        case("null-rect", "K(fle_imp_GetClipBox, dc, 0)", "lprect is NULL"),
    ];
    v.extend(dc_refusals("K(fle_imp_GetClipBox, @, A(q))"));
    stops("bulk-clipbox-stops", DC_SETUP, "GetClipBox", &v);
}

// ---- 5. UnrealizeObject, CreateSolidBrush, CreatePenIndirect ----

#[test]
fn unrealize_object_does_nothing_for_a_brush_and_a_palette_and_stops_for_the_rest() {
    // UnrealizeObject: "If hgdiobj is a brush, UnrealizeObject does nothing, and the function returns TRUE"; palettes: nonzero; stock objects: "should not be used"
    let l = lines("bulk-unrealize", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t br = SOLID(0x123456), pal = MKPAL(), ht = K(fle_imp_CreateHalftonePalette, sdc), bi = mkbrush(0, 7);
    uint64_t a = K(fle_imp_UnrealizeObject, br), b = K(fle_imp_UnrealizeObject, pal), c = K(fle_imp_UnrealizeObject, ht), d = K(fle_imp_UnrealizeObject, bi);
    printf("unrealize %llu %llu %llu %llu\n", U(a), U(b), U(c), U(d));
    printf("kinds still %d %d %d\n", KIND(br), KIND(pal), KIND(ht));
"#);
    assert_eq!(l, ["unrealize 1 1 1 1", "kinds still 9 1 1"]);
    let setup = "    uint64_t sdc = SCR(), dc = MKDC(), b = MKBMP(4, 4), br = SOLID(1), pen = PEN(0, 0, 0), sb = K(fle_imp_GetStockObject, 5), gone = SOLID(2);\n    DEL(gone);";
    let v = vec![
        case("stock-brush", "K(fle_imp_UnrealizeObject, sb)", "brush or a palette made by the guest"),
        case("default-brush", "K(fle_imp_UnrealizeObject, fle_gdi_default_object(FLE_GDI_DEFAULT_BRUSH))", "brush or a palette made by the guest"),
        case("default-palette", "K(fle_imp_UnrealizeObject, fle_gdi_default_object(FLE_GDI_DEFAULT_PALETTE))", "brush or a palette made by the guest"),
        case("pen", "K(fle_imp_UnrealizeObject, pen)", "brush or a palette made by the guest"),
        case("bitmap", "K(fle_imp_UnrealizeObject, b)", "brush or a palette made by the guest"),
        case("dc", "K(fle_imp_UnrealizeObject, dc)", "brush or a palette made by the guest"),
        case("deleted", "K(fle_imp_UnrealizeObject, gone)", "not a live object"),
        case("null", "K(fle_imp_UnrealizeObject, 0)", "not a live object"),
    ];
    stops("bulk-unrealize-stops", setup, "UnrealizeObject", &v);
}

#[test]
fn a_solid_brush_gets_a_gdi_handle_and_is_selected_and_deleted_like_the_one_of_create_brush_indirect() {
    let l = lines("bulk-solidbrush", r#"
    uint64_t dc = MKDC();
    uint64_t h1 = K(fle_imp_CreateSolidBrush, 0xFFFFFFFF00112233ull), h2 = K(fle_imp_CreateSolidBrush, 0);
    uint32_t bi[2] = { 7, 7 };
    int k = fle_gdi_brush_info(h1, bi);
    printf("kinds %d %d region %d %d distinct %d\n", KIND(h1), KIND(h2), INREG(h1), INREG(h2), h1 != h2);
    printf("info %d style %u colour %08x live %u\n", k, bi[0], bi[1], (unsigned)fle_gdi_brush_live_count());
    uint64_t prev = SELO(dc, h1);
    printf("previous is the default brush %d\n", prev == fle_gdi_default_object(FLE_GDI_DEFAULT_BRUSH));
    uint32_t d1 = DEL(h1);
    uint64_t prev2 = SELO(dc, h2);
    uint32_t d2 = DEL(h1), d3 = DEL(h1);
    printf("delete selected %u; second select returns the first %d; delete %u again %u live %u\n", d1, prev2 == h1, d2, d3, (unsigned)fle_gdi_brush_live_count());
"#);
    assert_eq!(
        l,
        [
            "kinds 9 9 region 1 1 distinct 1",
            "info 1 style 0 colour 00112233 live 2",
            "previous is the default brush 1",
            "delete selected 0; second select returns the first 1; delete 1 again 0 live 1",
        ]
    );
    let v = vec![
        // decision 0126 (wall 56): old: ("palette-rgb", CreateSolidBrush(0x02FFFFFF)) stopped ("nonzero high byte"); the high-order byte 2 is PALETTERGB, an ordinary RGB on this display (S-palettergb): it answers now. The word of
        // the message is "high-order byte" (old: "nonzero high byte"); PALETTEINDEX (0x01) and every other high-order byte still stop.
        case("high-byte", "K(fle_imp_CreateSolidBrush, 0x01000000)", "high-order byte"),
    ];
    stops("bulk-solidbrush-stops", "", "CreateSolidBrush", &v);
}

#[test]
fn a_pen_gets_a_gdi_handle_and_is_selected_and_deleted() {
    // CreatePenIndirect: "creates a logical cosmetic pen that has the style, width, and color"; LOGPEN: lopnStyle PS_SOLID .. PS_INSIDEFRAME (0 to 6)
    let l = lines("bulk-pen", r#"
    uint64_t dc = MKDC();
    uint64_t h[7];
    for (unsigned s = 0; s < 7; s++) h[s] = PEN(s, 1, 0x000000FF);
    int ok = 1;
    for (unsigned s = 0; s < 7; s++) ok = ok && KIND(h[s]) == FLE_GDI_KIND_PEN && INREG(h[s]);
    printf("seven %d live %u\n", ok, (unsigned)fle_gdi_pen_live_count());
    uint64_t p = PEN(5, 3, 0x00ABCDEF);
    uint32_t pi[3] = { 9, 9, 9 };
    int k = fle_gdi_pen_info(p, pi);
    printf("info %d %u %u %08x\n", k, pi[0], pi[1], pi[2]);
    uint64_t dflt = SELO(dc, p);
    printf("previous is the default pen %d\n", dflt == fle_gdi_default_object(FLE_GDI_DEFAULT_PEN));
    uint32_t d1 = DEL(p);
    uint64_t sp = K(fle_imp_GetStockObject, 7);
    uint64_t prev = SELO(dc, sp);
    uint32_t d2 = DEL(p), d3 = DEL(p);
    printf("delete selected %u; the stock pen replaced it %d; delete %u again %u live %u\n", d1, prev == p, d2, d3, (unsigned)fle_gdi_pen_live_count());
"#);
    assert_eq!(
        l,
        [
            "seven 1 live 7",
            "info 1 5 3 00abcdef",
            "previous is the default pen 1",
            "delete selected 0; the stock pen replaced it 1; delete 1 again 0 live 7",
        ]
    );
    let setup = "    uint64_t pen = PEN(0, 0, 0);\n    uint8_t buf[32];";
    let v = vec![
        case("style-7", "PEN(7, 0, 0)", "only PS_SOLID to PS_INSIDEFRAME"),
        case("style-8", "PEN(8, 0, 0)", "only PS_SOLID to PS_INSIDEFRAME"),
        case("style-huge", "PEN(0xFFFFFFFF, 0, 0)", "only PS_SOLID to PS_INSIDEFRAME"),
        case("width-negative", "PEN(0, -1, 0)", "lopnWidth.x is negative"),
        // decision 0126: the word of the message is "high-order byte" (old: "nonzero high byte")
        case("palette-colour", "PEN(0, 0, 0x01000000)", "high-order byte"),
        case("null", "K(fle_imp_CreatePenIndirect, 0)", "plpen is NULL"),
        case("get-object-of-a-pen", "K(fle_imp_GetObjectW, pen, 16, A(buf))", "kind this table does not describe"),
    ];
    stops("bulk-pen-stops", setup, "CreatePenIndirect", &v[..6]);
    stops("bulk-pen-getobject", setup, "GetObjectW", &v[6..]);
}

// ---- 6. GetPaletteEntries ----

#[test]
fn palette_entries_are_copied_from_a_created_palette_with_the_clamp_of_the_page() {
    // GetPaletteEntries: "the number of entries retrieved"; "If the nEntries parameter specifies more entries than exist in the palette, the remaining
    // members of the PALETTEENTRY structure are not altered"
    let l = lines("bulk-paletteentries", r#"
    uint64_t pal = MKPAL();
    uint8_t buf[80];
    int ok;
    memset(buf, 0xEE, sizeof buf);
    uint64_t a = K(fle_imp_GetPaletteEntries, pal, 0, 16, A(buf));
    ok = 1; for (unsigned i = 0; i < 64; i++) ok = ok && buf[i] == (uint8_t)(i * 3 + 1);
    int tail = 1; for (unsigned i = 64; i < 80; i++) tail = tail && buf[i] == 0xEE;
    printf("full %llu ok %d untouched %d\n", U(a), ok, tail);
    memset(buf, 0xEE, sizeof buf);
    uint64_t b = K(fle_imp_GetPaletteEntries, pal, 14, 8, A(buf));
    ok = 1; for (unsigned i = 0; i < 8; i++) ok = ok && buf[i] == (uint8_t)((56 + i) * 3 + 1);
    tail = 1; for (unsigned i = 8; i < 80; i++) tail = tail && buf[i] == 0xEE;
    printf("part %llu ok %d untouched %d\n", U(b), ok, tail);
    memset(buf, 0xEE, sizeof buf);
    uint64_t c = K(fle_imp_GetPaletteEntries, pal, 16, 4, A(buf)), d = K(fle_imp_GetPaletteEntries, pal, 3, 0, A(buf)), e = K(fle_imp_GetPaletteEntries, pal, 99, 4, A(buf));
    tail = 1; for (unsigned i = 0; i < 80; i++) tail = tail && buf[i] == 0xEE;
    printf("none %llu %llu %llu untouched %d\n", U(c), U(d), U(e), tail);
    uint64_t f = K(fle_imp_GetPaletteEntries, pal, 0xFFFFFFFF00000005ull, 0xFFFFFFFF00000002ull, A(buf));
    ok = 1; for (unsigned i = 0; i < 8; i++) ok = ok && buf[i] == (uint8_t)((20 + i) * 3 + 1);
    printf("lowdword %llu ok %d\n", U(f), ok);
"#);
    assert_eq!(l, ["full 16 ok 1 untouched 1", "part 2 ok 1 untouched 1", "none 0 0 0 untouched 1", "lowdword 2 ok 1"]);
    let setup = "    uint64_t sdc = SCR(), dc = MKDC(), pal = MKPAL(), ht = K(fle_imp_CreateHalftonePalette, sdc), b = MKBMP(4, 4), br = SOLID(1), gone = MKPAL();\n    uint8_t buf[80];\n    DEL(gone);";
    let v = vec![
        case("null-palette", "K(fle_imp_GetPaletteEntries, 0, 0, 4, A(buf))", "hpal is NULL"),
        case("null-entries", "K(fle_imp_GetPaletteEntries, pal, 0, 4, 0)", "pPalEntries is NULL"),
        case("halftone", "K(fle_imp_GetPaletteEntries, ht, 0, 4, A(buf))", "halftone palette"),
        case("default-palette", "K(fle_imp_GetPaletteEntries, fle_gdi_default_object(FLE_GDI_DEFAULT_PALETTE), 0, 4, A(buf))", "not a live palette made by CreatePalette"),
        case("bitmap", "K(fle_imp_GetPaletteEntries, b, 0, 4, A(buf))", "not a live palette made by CreatePalette"),
        case("brush", "K(fle_imp_GetPaletteEntries, br, 0, 4, A(buf))", "not a live palette made by CreatePalette"),
        case("deleted", "K(fle_imp_GetPaletteEntries, gone, 0, 4, A(buf))", "not a live palette made by CreatePalette"),
    ];
    stops("bulk-paletteentries-stops", setup, "GetPaletteEntries", &v);
}

// ---- 7. Icons and cursors ----

#[test]
fn get_icon_info_makes_two_new_bitmaps_for_a_cursor_and_destroy_cursor_deletes_it() {
    let l = lines("bulk-iconinfo", r#"
    uint64_t sdc = SCR(), mask = MKBMP(32, 32), color = CCB(sdc, 32, 32), mmask = MKBMP(16, 32);
    uint64_t cur = ICONI(0, 3, 4, mask, color), mono = ICONI(0, 1, 2, mmask, 0);
    uint64_t r = K(fle_imp_GetIconInfo, cur, A(iio));
    uint32_t fi, hx, hy;
    uint64_t hm, hc;
    uint32_t b[4];
    memcpy(&fi, iio, 4); memcpy(&hx, iio + 4, 4); memcpy(&hy, iio + 8, 4); memcpy(&hm, iio + 16, 8); memcpy(&hc, iio + 24, 8);
    printf("info %llu fIcon %u hot %u %u\n", U(r), fi, hx, hy);
    fle_gdi_bitmap_info(hm, b);
    printf("mask kind %d fmt %d size %u x %u planes %u bits %u new %d\n", KIND(hm), fle_gdi_bitmap_format(hm), b[0], b[1], b[2], b[3], hm != mask);
    fle_gdi_bitmap_info(hc, b);
    printf("color kind %d fmt %d size %u x %u planes %u bits %u new %d\n", KIND(hc), fle_gdi_bitmap_format(hc), b[0], b[1], b[2], b[3], hc != color);
    uint64_t r2 = K(fle_imp_GetIconInfo, cur, A(iio));
    uint64_t hm2, hc2;
    memcpy(&hm2, iio + 16, 8); memcpy(&hc2, iio + 24, 8);
    printf("second call %llu: new bitmaps %d %d; deletable %u %u %u %u\n", U(r2), hm2 != hm && hm2 != mask, hc2 != hc && hc2 != color, DEL(hm), DEL(hc), DEL(hm2), DEL(hc2));
    uint64_t r3 = K(fle_imp_GetIconInfo, mono, A(iio));
    memcpy(&fi, iio, 4); memcpy(&hx, iio + 4, 4); memcpy(&hy, iio + 8, 4); memcpy(&hm, iio + 16, 8); memcpy(&hc, iio + 24, 8);
    fle_gdi_bitmap_info(hm, b);
    printf("mono %llu fIcon %u hot %u %u mask %u x %u fmt %d colour %llu\n", U(r3), fi, hx, hy, b[0], b[1], fle_gdi_bitmap_format(hm), U(hc));
    uint32_t d1 = (uint32_t)K(fle_imp_DestroyCursor, cur), k1 = KIND(cur);
    uint32_t d2 = (uint32_t)K(fle_imp_DestroyCursor, cur), d3 = (uint32_t)K(fle_imp_DestroyIcon, cur), d4 = (uint32_t)K(fle_imp_DestroyIcon, 0), d5 = (uint32_t)K(fle_imp_DestroyCursor, 0);
    printf("destroy %u kind %d; again %u %u; null %u %u\n", d1, k1, d2, d3, d4, d5);
    uint64_t ic = ICONI(1, 0, 0, MKBMP(8, 8), CCB(sdc, 8, 8));
    uint32_t e1 = (uint32_t)K(fle_imp_DestroyIcon, ic), e2 = (uint32_t)K(fle_imp_DestroyIcon, mono);
    printf("destroy icon %u cursor by DestroyIcon %u kinds %d %d\n", e1, e2, KIND(ic), KIND(mono));
    uint64_t cur2 = ICONI(0, 0, 0, MKBMP(8, 8), 0);
    printf("handles are never reused %d\n", cur2 != cur && cur2 != mono && cur2 != ic);
"#);
    assert_eq!(
        l,
        [
            "info 1 fIcon 0 hot 3 4",
            "mask kind 7 fmt 0 size 32 x 32 planes 1 bits 1 new 1",
            "color kind 7 fmt 1 size 32 x 32 planes 1 bits 32 new 1",
            "second call 1: new bitmaps 1 1; deletable 1 1 1 1",
            "mono 1 fIcon 0 hot 1 2 mask 16 x 32 fmt 0 colour 0",
            "destroy 1 kind 0; again 0 0; null 0 0",
            "destroy icon 1 cursor by DestroyIcon 1 kinds 0 0",
            "handles are never reused 1",
        ]
    );
}

/// Decision 0113 wall 8 (provisional): the Learn pages say not to use DestroyCursor on a shared cursor and give no result; the call returns 1 and the cursor stays valid
/// (the return value is a choice without a source). The handle of IDC_SIZEALL (32646) is FLE_CURSOR_HANDLE_BASE + 4 * (32646 - 32512 + 1) = 0x7FCA021C, the one the guest passed.
#[test]
fn destroy_cursor_of_a_shared_standard_cursor_returns_1_and_keeps_it() {
    let l = lines("c1-destroycursor-shared", r#"
    uint64_t arrow = K(fle_imp_LoadCursorW, 0, 32512), prev;
    printf("arrow %d\n", arrow == 0x7FCA0004);
    printf("destroy %llu\n", U(K(fle_imp_DestroyCursor, arrow)));
    printf("destroy again %llu\n", U(K(fle_imp_DestroyCursor, arrow)));
    printf("still the same handle %d\n", K(fle_imp_LoadCursorW, 0, 32512) == arrow);
    prev = K(fle_imp_SetCursor, arrow);
    printf("setcursor prev %llu now %d\n", U(prev), K(fle_imp_GetCursor) == arrow);
    printf("destroy the current one %llu\n", U(K(fle_imp_DestroyCursor, arrow)));
    printf("sizeall %llu\n", U(K(fle_imp_DestroyCursor, 0x7FCA021C)));
"#);
    assert_eq!(l, ["arrow 1", "destroy 1", "destroy again 1", "still the same handle 1", "setcursor prev 0 now 1", "destroy the current one 1", "sizeall 1"]);
    // DestroyIcon is not extended to a cursor of the table, and a cursor of CreateIconIndirect is still destroyed as before
    let setup = "    uint64_t arrow = K(fle_imp_LoadCursorW, 0, 32512);";
    stops("c1-destroyicon-shared-cursor", setup, "DestroyIcon", &[case("destroyicon-of-a-shared-cursor", "K(fle_imp_DestroyIcon, arrow)", "not issued by this runtime's GDI table")]);
}

#[test]
fn icon_and_cursor_calls_stop_where_the_pages_give_no_result() {
    let setup = "    uint64_t sdc = SCR(), ico = K(fle_imp_LoadIconW, 0, 32512), arrow = K(fle_imp_LoadCursorW, 0, 32512), b = MKBMP(4, 4);\n    uint64_t cur = ICONI(0, 0, 0, MKBMP(8, 8), 0), icn = ICONI(1, 0, 0, MKBMP(8, 8), 0), used = ICONI(0, 0, 0, MKBMP(8, 8), 0);\n    uint8_t buf[64];\n    K(fle_imp_SetCursor, used);";
    let d = vec![
        case("shared-icon", "K(fle_imp_DestroyIcon, ico)", "not an icon or cursor made by CreateIconIndirect"),
        case("shared-cursor-from-another-table", "K(fle_imp_DestroyIcon, arrow)", "not issued by this runtime's GDI table"),
        case("bitmap", "K(fle_imp_DestroyIcon, b)", "not an icon or cursor made by CreateIconIndirect"),
        case("current-cursor", "K(fle_imp_DestroyIcon, used)", "current cursor"),
        case("never-issued", "K(fle_imp_DestroyIcon, 0x1234)", "not issued by this runtime's GDI table"),
    ];
    stops("bulk-destroyicon-stops", setup, "DestroyIcon", &d);
    let c = vec![
        case("icon-given-to-destroycursor", "K(fle_imp_DestroyCursor, icn)", "fIcon TRUE"),
        // decision 0113 wall 8: DestroyCursor of a shared standard cursor returns 1 (destroy_cursor_of_a_shared_standard_cursor_returns_1_and_keeps_it); a number that is no standard cursor still stops
        case("never-issued-cursor", "K(fle_imp_DestroyCursor, 0x7FCA0001)", "not issued by this runtime's GDI table"),
        case("shared-icon", "K(fle_imp_DestroyCursor, ico)", "not an icon or cursor made by CreateIconIndirect"),
        case("current-cursor", "K(fle_imp_DestroyCursor, used)", "current cursor"),
    ];
    stops("bulk-destroycursor-stops", setup, "DestroyCursor", &c);
    let g = vec![
        case("icon", "K(fle_imp_GetIconInfo, icn, A(buf))", "fIcon TRUE"),
        case("shared-icon", "K(fle_imp_GetIconInfo, ico, A(buf))", "not an icon or cursor made by CreateIconIndirect"),
        case("shared-cursor", "K(fle_imp_GetIconInfo, arrow, A(buf))", "not an icon or cursor made by CreateIconIndirect"),
        case("null-handle", "K(fle_imp_GetIconInfo, 0, A(buf))", "NULL"),
        case("null-struct", "K(fle_imp_GetIconInfo, cur, 0)", "piconinfo is NULL"),
        case("destroyed", "K(fle_imp_DestroyCursor, cur); K(fle_imp_GetIconInfo, cur, A(buf))", "not an icon or cursor made by CreateIconIndirect"),
    ];
    stops("bulk-geticoninfo-stops", setup, "GetIconInfo", &g);
}

#[test]
fn the_cursor_state_follows_set_cursor_get_cursor_and_show_cursor() {
    // SetCursor: "The return value is the handle to the previous cursor, if there was one. If there was no previous cursor, the return value is NULL"; GetCursor:
    // "If there is no cursor, the return value is NULL"; ShowCursor: "the return value specifies the new display counter", "If a mouse is installed, the initial display count is 0"
    let l = lines("bulk-cursor", r#"
    uint64_t sdc = SCR(), arrow = K(fle_imp_LoadCursorW, 0, 32512);
    uint64_t cc = ICONI(0, 0, 0, MKBMP(8, 8), CCB(sdc, 8, 8));
    printf("start %llu\n", U(K(fle_imp_GetCursor, 0)));
    uint64_t p1 = K(fle_imp_SetCursor, arrow), g1 = K(fle_imp_GetCursor, 0), p2 = K(fle_imp_SetCursor, cc), p3 = K(fle_imp_SetCursor, cc), p4 = K(fle_imp_SetCursor, 0), g4 = K(fle_imp_GetCursor, 0);
    printf("set %d %d %d %d get %d %d\n", p1 == 0, p2 == arrow, p3 == cc, p4 == cc, g1 == arrow, g4 == 0);
    uint64_t w = fle_cursor_image_handle_of(FLE_CURSOR_IDC_IBEAM);
    uint64_t p5 = K(fle_imp_SetCursor, w);
    printf("image cursor accepted %d\n", p5 == 0);
    int a = (int)S(K(fle_imp_ShowCursor, 1)), b = (int)S(K(fle_imp_ShowCursor, 1)), c = (int)S(K(fle_imp_ShowCursor, 0)), d = (int)S(K(fle_imp_ShowCursor, 0));
    int e = (int)S(K(fle_imp_ShowCursor, 0x100000000ull)), f = (int)S(K(fle_imp_ShowCursor, 1));
    printf("show %d %d %d %d %d %d count %d\n", a, b, c, d, e, f, (int)fle_gdi_cursor_display_count());
    printf("current %d\n", fle_gdi_cursor_current() == w);
"#);
    assert_eq!(l, ["start 0", "set 1 1 1 1 get 1 1", "image cursor accepted 1", "show 1 2 1 0 -1 0 count 0", "current 1"]);
    let setup = "    uint64_t sdc = SCR(), ico = K(fle_imp_LoadIconW, 0, 32512), b = MKBMP(4, 4);\n    uint64_t gone = ICONI(0, 0, 0, MKBMP(8, 8), 0);\n    K(fle_imp_DestroyCursor, gone);";
    let v = vec![
        case("icon-of-loadiconw", "K(fle_imp_SetCursor, ico)", "not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect"),
        case("bitmap", "K(fle_imp_SetCursor, b)", "not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect"),
        case("never-issued", "K(fle_imp_SetCursor, 0x1234)", "not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect"),
        case("destroyed", "K(fle_imp_SetCursor, gone)", "not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect"),
    ];
    stops("bulk-setcursor-stops", setup, "SetCursor", &v);
}

// ---- 8. Pixel operations on DIB sections ----

#[test]
fn fill_rect_paints_the_half_open_rectangle_with_a_solid_brush_into_a_32_bit_dib() {
    // FillRect: "includes the left and top borders, but excludes the right and bottom borders". The pixel is blue, green, red and an unused byte; the fills write
    // the unused byte as 0 (a CHOICE WITHOUT A SOURCE). COLORREF 0x00112233 is red 0x33, green 0x22, blue 0x11.
    let l = lines("bulk-fillrect32", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t d = MKDIB(sdc, 4, -3, 32), br = SOLID(0x00112233);
    FILL(d, 0xFF);
    SELO(dc, d);
    uint64_t r = FILLRECT(dc, 1, 1, 3, 2, br);
    printf("ret %llu\n", U(r));
    dump("row", d);
"#);
    assert_eq!(
        l,
        [
            "ret 1",
            "row 0 ffffffffffffffffffffffffffffffff",
            "row 1 ffffffff1122330011223300ffffffff",
            "row 2 ffffffffffffffffffffffffffffffff",
        ]
    );
}

#[test]
fn a_bottom_up_dib_has_its_first_memory_row_at_the_bottom_of_the_image() {
    // BITMAPINFOHEADER: a positive biHeight is a bottom-up DIB with the origin at the lower left corner. Device row 0 of a 3-row bottom-up DIB is memory row 2.
    let l = lines("bulk-bottomup", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t d = MKDIB(sdc, 4, 3, 24), br = SOLID(0x00112233);
    FILL(d, 0xFF);
    SELO(dc, d);
    uint64_t r = FILLRECT(dc, 0, 0, 2, 1, br);
    printf("ret %llu\n", U(r));
    dump("row", d);
"#);
    assert_eq!(l, ["ret 1", "row 0 ffffffffffffffffffffffff", "row 1 ffffffffffffffffffffffff", "row 2 112233112233ffffffffffff"]);
}

#[test]
fn pat_blt_paints_with_the_brush_or_black_or_white() {
    // PatBlt: PATCOPY copies the pattern (a solid brush: its colour), BLACKNESS the colour at index 0 of the physical palette (black), WHITENESS index 1 (white).
    // COLORREF 0x000C0B0A is red 0x0A, green 0x0B, blue 0x0C.
    let l = lines("bulk-patblt", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t d = MKDIB(sdc, 4, -4, 32), br = SOLID(0x000C0B0A);
    FILL(d, 0x55);
    SELO(dc, d);
    uint64_t w = PATBLT(dc, 1, 1, 2, 2, 0xFF0062);
    uint64_t k = PATBLT(dc, 0, 0, 1, 1, 0x42);
    SELO(dc, br);
    uint64_t p = PATBLT(dc, 3, 3, 1, 1, 0xF00021);
    printf("ret %llu %llu %llu\n", U(w), U(k), U(p));
    dump("row", d);
"#);
    assert_eq!(
        l,
        [
            "ret 1 1 1",
            "row 0 00000000555555555555555555555555",
            "row 1 55555555ffffff00ffffff0055555555",
            "row 2 55555555ffffff00ffffff0055555555",
            "row 3 5555555555555555555555550c0b0a00",
        ]
    );
}

#[test]
fn a_rectangle_is_clipped_to_the_bitmap_and_an_empty_one_paints_nothing() {
    let l = lines("bulk-clip", r#"
    uint64_t sdc = SCR(), dc = MKDC(), br = SOLID(0x00112233);
    uint64_t a = MKDIB(sdc, 4, -4, 32), b = MKDIB(sdc, 4, -4, 32), c = MKDIB(sdc, 4, -4, 32), d = MKDIB(sdc, 4, -4, 32);
    SELO(dc, a);
    uint64_t r1 = FILLRECT(dc, NEG(-2), NEG(-2), 2, 2, br);
    SELO(dc, b);
    uint64_t r2 = FILLRECT(dc, 3, 3, 10, 10, br);
    SELO(dc, c);
    uint64_t r3 = FILLRECT(dc, 10, 10, 12, 12, br);
    SELO(dc, d);
    uint64_t r4 = FILLRECT(dc, 3, 3, 1, 1, br);
    printf("ret %llu %llu %llu %llu\n", U(r1), U(r2), U(r3), U(r4));
    dump("a", a); dump("b", b); dump("c", c); dump("d", d);
"#);
    let z = "00000000000000000000000000000000";
    let two = "11223300112233000000000000000000";
    let last = "00000000000000000000000011223300";
    assert_eq!(l[0], "ret 1 1 1 1");
    assert_eq!(&l[1..5], [format!("a 0 {two}"), format!("a 1 {two}"), format!("a 2 {z}"), format!("a 3 {z}")]);
    assert_eq!(&l[5..9], [format!("b 0 {z}"), format!("b 1 {z}"), format!("b 2 {z}"), format!("b 3 {last}")]);
    assert_eq!(&l[9..13], [format!("c 0 {z}"), format!("c 1 {z}"), format!("c 2 {z}"), format!("c 3 {z}")]);
    assert_eq!(&l[13..17], [format!("d 0 {z}"), format!("d 1 {z}"), format!("d 2 {z}"), format!("d 3 {z}")]);
    assert_eq!(l.len(), 17);
}

#[test]
fn the_origins_of_the_dc_move_the_rectangle() {
    // Dx = Lx - WOx + VOx (Mapping Modes and Translations, MM_TEXT)
    let l = lines("bulk-origin-fill", r#"
    uint64_t sdc = SCR(), dc = MKDC(), br = SOLID(0x00112233);
    uint64_t d = MKDIB(sdc, 4, -4, 32);
    SELO(dc, d);
    K(fle_imp_SetWindowOrgEx, dc, 2, 0, 0);
    uint64_t r1 = FILLRECT(dc, 2, 0, 3, 1, br);
    K(fle_imp_SetViewportOrgEx, dc, 1, 1, 0);
    uint64_t r2 = FILLRECT(dc, 3, 1, 4, 2, br);
    uint64_t r3 = PATBLT(dc, 5, 4, 1, 1, 0x42);
    printf("ret %llu %llu %llu\n", U(r1), U(r2), U(r3));
    dump("row", d);
"#);
    let z = "00000000000000000000000000000000";
    assert_eq!(
        l,
        [
            "ret 1 1 1".to_string(),
            "row 0 11223300000000000000000000000000".to_string(),
            format!("row 1 {z}"),
            "row 2 00000000000000001122330000000000".to_string(),
            format!("row 3 {z}"),
        ]
    );
}

#[test]
fn fill_rect_with_the_window_text_system_colour_plus_one_and_a_bitfields_dib() {
    // FillRect: hbr "may be ... a color value ... one of the standard system colors (the value 1 must be added)": COLOR_WINDOWTEXT (8) + 1 = 9 is the one
    // system colour with a value here (decision 0108 D11: black, a choice). A 32-bit BI_BITFIELDS DIB with the 8-8-8 masks has the same pixel layout.
    let l = lines("bulk-fill-sys", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    uint64_t d = MKDIBF(sdc, 2, -1, 32, 0x00FF0000, 0x0000FF00, 0x000000FF);
    FILL(d, 0x77);
    SELO(dc, d);
    uint64_t r = FILLRECT(dc, 0, 0, 1, 1, 9);
    printf("ret %llu\n", U(r));
    dump("row", d);
"#);
    assert_eq!(l, ["ret 1", "row 0 0000000077777777"]);
}

#[test]
fn bit_blt_copies_the_pixel_bytes_between_dib_sections_of_the_same_format() {
    // BitBlt SRCCOPY: "Copies the source rectangle directly to the destination rectangle"; the unused byte of a 32-bit pixel is copied too (a choice);
    // "BitBlt only does clipping on the destination DC". The source pattern is byte k = k + 1 over the 64 bytes of a 4 x 4 x 32-bit top-down DIB.
    let l = lines("bulk-bitblt", r#"
    uint64_t sdc = SCR(), a = MKDC(), b = MKDC();
    uint64_t sa = MKDIB(sdc, 4, -4, 32), d1 = MKDIB(sdc, 4, -4, 32), d2 = MKDIB(sdc, 4, -4, 32), d3 = MKDIB(sdc, 4, -4, 32);
    uint8_t *p = BITS(sa);
    for (unsigned i = 0; i < 64; i++) p[i] = (uint8_t)(i + 1);
    SELO(a, sa);
    SELO(b, d1);
    uint64_t r1 = BITBLT(b, 1, 1, 2, 2, a, 0, 0, 0xCC0020);
    SELO(b, d2);
    uint64_t r2 = BITBLT(b, 3, 3, 2, 2, a, 0, 0, 0xCC0020);
    SELO(b, d3);
    uint64_t r3 = BITBLT(b, NEG(-1), NEG(-1), 2, 2, a, 0, 0, 0xCC0020);
    printf("ret %llu %llu %llu\n", U(r1), U(r2), U(r3));
    dump("d1", d1); dump("d2", d2); dump("d3", d3);
"#);
    let z = "00000000000000000000000000000000";
    assert_eq!(l[0], "ret 1 1 1");
    assert_eq!(
        &l[1..5],
        [format!("d1 0 {z}"), "d1 1 00000000010203040506070800000000".to_string(), "d1 2 00000000111213141516171800000000".to_string(), format!("d1 3 {z}")]
    );
    assert_eq!(&l[5..9], [format!("d2 0 {z}"), format!("d2 1 {z}"), format!("d2 2 {z}"), "d2 3 00000000000000000000000001020304".to_string()]);
    assert_eq!(&l[9..13], ["d3 0 15161718000000000000000000000000".to_string(), format!("d3 1 {z}"), format!("d3 2 {z}"), format!("d3 3 {z}")]);
    assert_eq!(l.len(), 13);
}

#[test]
fn bit_blt_inside_one_bitmap_moves_like_memmove_in_both_directions() {
    let l = lines("bulk-bitblt-overlap", r#"
    uint64_t sdc = SCR(), dc = MKDC(), dc2 = MKDC(), dc3 = MKDC();
    uint64_t h = MKDIB(sdc, 4, -2, 32), v = MKDIB(sdc, 2, -4, 32), u = MKDIB(sdc, 2, -4, 32);
    uint8_t *p = BITS(h);
    for (unsigned i = 0; i < 4; i++) memset(p + 4 * i, (int)(i + 1), 4);
    for (unsigned r = 0; r < 4; r++) { memset(BITS(v) + 8 * r, (int)((r + 1) * 0x11), 8); memset(BITS(u) + 8 * r, (int)((r + 1) * 0x11), 8); }
    SELO(dc, h); SELO(dc2, v); SELO(dc3, u);
    uint64_t r1 = BITBLT(dc, 1, 0, 3, 1, dc, 0, 0, 0xCC0020);
    uint64_t r2 = BITBLT(dc2, 0, 1, 2, 3, dc2, 0, 0, 0xCC0020);
    uint64_t r3 = BITBLT(dc3, 0, 0, 2, 3, dc3, 0, 1, 0xCC0020);
    printf("ret %llu %llu %llu\n", U(r1), U(r2), U(r3));
    dump("h", h); dump("down", v); dump("up", u);
"#);
    assert_eq!(
        l,
        [
            "ret 1 1 1",
            "h 0 01010101010101010202020203030303",
            "h 1 00000000000000000000000000000000",
            "down 0 1111111111111111",
            "down 1 1111111111111111",
            "down 2 2222222222222222",
            "down 3 3333333333333333",
            "up 0 2222222222222222",
            "up 1 3333333333333333",
            "up 2 4444444444444444",
            "up 3 4444444444444444",
        ]
    );
}

#[test]
fn bit_blt_copies_device_rows_between_a_top_down_and_a_bottom_up_dib_and_8_bit_dibs() {
    let l = lines("bulk-bitblt-orient", r#"
    uint64_t sdc = SCR(), a = MKDC(), b = MKDC(), c = MKDC(), d = MKDC();
    uint64_t src = MKDIB(sdc, 2, -2, 24), dst = MKDIB(sdc, 2, 2, 24);
    const uint8_t px[12] = { 1,2,3, 4,5,6, 7,8,9, 10,11,12 };
    memcpy(BITS(src), px, 6);
    memcpy(BITS(src) + 8, px + 6, 6);
    SELO(a, src); SELO(b, dst);
    uint64_t r1 = BITBLT(b, 0, 0, 2, 2, a, 0, 0, 0xCC0020);
    dump("dst", dst);
    /* decision 0126 (wall 55): old: an 8-bit DIB section was the destination of a same-format copy (4 x 1, `d8 0 01020304`) and a 0-wide copy returned 1. An 8-bit destination needs a colour-to-index mapping that no page
     * gives ("the colour-to-pixel mapping of the other forms (palette index, 5-5-5 / 5-6-5) is not on the pages"): it stops now, in tests below (fills_and_blits_stop_where_the_pages_leave_the_bytes_open). The same
     * orientation check is made with 32-bit sections: the source is bottom-up or top-down, the device rows are the same. */
    uint64_t s32 = MKDIB(sdc, 4, -1, 32), d32 = MKDIB(sdc, 4, -1, 32);
    for (unsigned i = 0; i < 16; i++) BITS(s32)[i] = (uint8_t)(i + 1);
    SELO(c, s32); SELO(d, d32);
    uint64_t r2 = BITBLT(d, 0, 0, 4, 1, c, 0, 0, 0xCC0020);
    uint64_t r3 = BITBLT(d, 0, 0, 0, 5, c, 0, 0, 0xCC0020);
    printf("ret %llu %llu %llu\n", U(r1), U(r2), U(r3));
    dump("d32", d32);
"#);
    assert_eq!(l, ["dst 0 0708090a0b0c", "dst 1 010203040506", "ret 1 1 1", "d32 0 0102030405060708090a0b0c0d0e0f10"]);
}

#[test]
fn fills_and_blits_stop_where_the_pages_leave_the_bytes_open() {
    let setup = r#"
    uint64_t sdc = SCR(), dc = MKDC(), dc2 = MKDC(), dcx = MKDC(), b = MKBMP(4, 4);
    uint64_t d32 = MKDIB(sdc, 4, -4, 32), d32b = MKDIB(sdc, 4, -4, 32), d24 = MKDIB(sdc, 4, -4, 24), d16 = MKDIB(sdc, 4, -4, 16), d8 = MKDIB(sdc, 4, -4, 8), d4 = MKDIB(sdc, 4, -4, 4), d4b = MKDIB(sdc, 4, -4, 4);
    uint64_t f555 = MKDIBF(sdc, 4, -4, 16, 0x7C00, 0x03E0, 0x001F), f565 = MKDIBF(sdc, 4, -4, 16, 0xF800, 0x07E0, 0x001F);
    mkbmi(4, -4, 8, 0); p32(40, 0x11223344);
    uint64_t other8 = K(fle_imp_CreateDIBSection, sdc, A(bmi), 0, A(&bits_out), 0, 0);
    uint64_t solid = SOLID(0x00112233), hollow = mkbrush(1, 0), nullb = K(fle_imp_GetStockObject, 5), dsc = SCR();
    DELDC(dcx);
    SELO(dc, d32);
    SELO(dc2, d32b);
"#;
    // PatBlt and FillRect on a destination that the pages do not fix
    let mut p = vec![
        case("16-bit", "SELO(dc, d16); PATBLT(dc, 0, 0, 1, 1, 0x42)", "not a 24- or 32-bit RGB form"),
        case("16-bit-bitfields", "SELO(dc, f565); PATBLT(dc, 0, 0, 1, 1, 0x42)", "not a 24- or 32-bit RGB form"),
        case("8-bit", "SELO(dc, d8); PATBLT(dc, 0, 0, 1, 1, 0x42)", "not a 24- or 32-bit RGB form"),
        case("4-bit", "SELO(dc, d4); PATBLT(dc, 0, 0, 1, 1, 0x42)", "not a 24- or 32-bit RGB form"),
        case("other-mapping-mode", "K(fle_imp_SetMapMode, dc, 2); PATBLT(dc, 0, 0, 1, 1, 0x42)", "not MM_TEXT"),
        case("screen-dc", "PATBLT(dsc, 0, 0, 1, 1, 0x42)", "no window table is linked")   /* decision 0126: old word "GetDC(NULL)": the screen DC draws on the surface of the window table; this unit link has none */,
        case("null-dc", "PATBLT(0, 0, 0, 1, 1, 0x42)", "not a DC of this runtime"),
        case("deleted-dc", "PATBLT(dcx, 0, 0, 1, 1, 0x42)", "deleted by DeleteDC"),
    ];
    // decision 0126 (wall 55): old: ("arguments-in-the-message", PATBLT(dc, 1, 2, 30, 40, 0x5A0049)) stopped with the rop and the rectangle in the message; PATINVERT paints now. The message form is pinned by the one stop that is left (a rop that is not one of the 256 codes):
    p.push(case("arguments-in-the-message", "PATBLT(dc, 1, 2, 30, 40, 0xFF0063)", "rop 0xff0063, 30 x 40 at (1, 2)"));
    stops("bulk-patblt-stops", setup, "PatBlt", &p);
    let f = vec![
        case("16-bit", "SELO(dc, d16); FILLRECT(dc, 0, 0, 1, 1, solid)", "not a 24- or 32-bit RGB form"),
        case("8-bit", "SELO(dc, d8); FILLRECT(dc, 0, 0, 1, 1, solid)", "not a 24- or 32-bit RGB form"),
        case("null-rect", "K(fle_imp_FillRect, dc, 0, solid)", "lprc is NULL"),
        case("not-a-brush", "FILLRECT(dc, 0, 0, 1, 1, b)", "the handle is not a brush"),
        case("null-brush", "FILLRECT(dc, 0, 0, 1, 1, 0)", "not a live brush"),
        case("other-mapping-mode", "K(fle_imp_SetMapMode, dc, 2); FILLRECT(dc, 0, 0, 1, 1, solid)", "not MM_TEXT"),
        case("screen-dc", "FILLRECT(dsc, 0, 0, 1, 1, solid)", "no window table is linked")   /* decision 0126: old word "GetDC(NULL)" */,
        case("deleted-dc", "FILLRECT(dcx, 0, 0, 1, 1, solid)", "deleted by DeleteDC"),
        case("arguments-in-the-message", "FILLRECT(dc, 0, 0, 1, 1, 0x1234)", "hbr 0x1234"),   // (a brush handle that was never issued: the one stop of FillRect that has the argument in its message)
    ];
    stops("bulk-fillrect-stops", setup, "FillRect", &f);
    let bb = vec![
        case("source-is-a-screen-dc", "BITBLT(dc, 0, 0, 1, 1, dsc, 0, 0, 0xCC0020)", "no window table is linked")   /* decision 0126: old word "hdcSrc is a DC of GetDC(NULL)" */,
        case("destination-is-a-screen-dc", "BITBLT(dsc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "no window table is linked")   /* decision 0126: old word "hdc is a DC of GetDC(NULL)" */,
        case("source-deleted", "BITBLT(dc, 0, 0, 1, 1, dcx, 0, 0, 0xCC0020)", "hdcSrc was deleted by DeleteDC"),
        case("destination-null", "BITBLT(0, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "hdc is not a DC of this runtime"),
        case("masks-differ", "SELO(dc, f555); SELO(dc2, f565); BITBLT(dc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "the pixel format is not a 24- or 32-bit RGB form"),
        case("colour-tables-differ", "SELO(dc, d8); SELO(dc2, other8); BITBLT(dc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "the pixel format is not a 24- or 32-bit RGB form"),
        case("fewer-than-8-bits", "SELO(dc, d4); SELO(dc2, d4b); BITBLT(dc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "the pixel format is not a 24- or 32-bit RGB form"),
        case("other-mapping-mode-destination", "K(fle_imp_SetMapMode, dc, 2); BITBLT(dc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "hdc is in mapping mode 2"),
        case("other-mapping-mode-source", "K(fle_imp_SetMapMode, dc2, 2); BITBLT(dc, 0, 0, 1, 1, dc2, 0, 0, 0xCC0020)", "hdcSrc is in mapping mode 2"),
    ];
    stops("bulk-bitblt-stops", setup, "BitBlt", &bb);
}

// ---- 9. Memory DC answer of GetDeviceCaps (the optional hook) ----

#[test]
fn the_memory_dc_answer_of_get_device_caps_reports_the_bits_and_planes_of_the_selected_bitmap() {
    // READING: a memory DC reports the colour format of the bitmap selected into it. The row of GetDeviceCaps is NOT wired to this function by the main diff
    // (g1-optional-getdevicecaps.diff does it): this test calls the function directly.
    let l = lines("bulk-memcaps", r#"
    uint64_t sdc = SCR(), dc = MKDC();
    int32_t v = -1;
    int k = fle_gdi_memdc_caps(dc, 12, &v);
    printf("placeholder bits %d %d", k, (int)v);
    k = fle_gdi_memdc_caps(dc, 14, &v);
    printf(" planes %d %d\n", k, (int)v);
    SELO(dc, MKDIB(sdc, 2, 2, 24));
    k = fle_gdi_memdc_caps(dc, 12, &v); printf("dib24 %d %d", k, (int)v);
    SELO(dc, MKDIBF(sdc, 2, 2, 16, 0xF800, 0x07E0, 0x001F));
    k = fle_gdi_memdc_caps(dc, 12, &v); printf(" dib16 %d %d", k, (int)v);
    SELO(dc, CCB(sdc, 4, 4));
    k = fle_gdi_memdc_caps(dc, 12, &v); printf(" colour %d %d", k, (int)v);
    k = fle_gdi_memdc_caps(dc, 14, &v); printf(" planes %d %d\n", k, (int)v);
    v = -7;
    k = fle_gdi_memdc_caps(sdc, 12, &v);
    int k2 = fle_gdi_memdc_caps(0, 12, &v);
    printf("not a memory dc: %d %d value %d\n", k, k2, (int)v);
"#);
    assert_eq!(l, ["placeholder bits 1 1 planes 1 1", "dib24 1 24 dib16 1 16 colour 1 32 planes 1 1", "not a memory dc: 0 0 value -7"]);
    let setup = "    uint64_t dc = MKDC();\n    int32_t v;";
    let v = vec![
        case("horzres", "fle_gdi_memdc_caps(dc, 8, &v)", "index 8 of a memory DC"),
        case("logpixelsx", "fle_gdi_memdc_caps(dc, 88, &v)", "index 88 of a memory DC"),
    ];
    stops("bulk-memcaps-stops", setup, "GetDeviceCaps", &v);
}

// ---- 9b. StretchBlt of equal sizes (g1-optional-stretchblt.diff) ----

#[test]
fn stretch_blt_of_equal_sizes_copies_like_bit_blt_and_other_sizes_stop() {
    let l = lines("bulk-stretchblt", r#"
    uint64_t sdc = SCR(), a = MKDC(), b = MKDC();
    uint64_t sa = MKDIB(sdc, 4, -4, 32), d1 = MKDIB(sdc, 4, -4, 32);
    uint8_t *p = BITS(sa);
    for (unsigned i = 0; i < 64; i++) p[i] = (uint8_t)(i + 1);
    SELO(a, sa);
    SELO(b, d1);
    uint64_t r = K(fle_imp_StretchBlt, b, 1, 1, 2, 2, a, 0, 0, 2, 2, 0xCC0020);
    printf("ret %llu\n", U(r));
    dump("d1", d1);
"#);
    let z = "00000000000000000000000000000000";
    assert_eq!(
        l,
        [
            "ret 1".to_string(),
            format!("d1 0 {z}"),
            "d1 1 00000000010203040506070800000000".to_string(),
            "d1 2 00000000111213141516171800000000".to_string(),
            format!("d1 3 {z}"),
        ]
    );
    let setup = "    uint64_t sdc = SCR(), a = MKDC(), b = MKDC(), sa = MKDIB(sdc, 4, -4, 32), d1 = MKDIB(sdc, 4, -4, 32);\n    SELO(a, sa);\n    SELO(b, d1);";
    let v = vec![
        // decision 0126 (wall 55): old: ("stretch", 2x2 to 4x4), ("compress", 2x2 to 1x1) and ("rop", 0x8800C6 SRCAND) stopped ("sizes differ", "only SRCCOPY"); StretchBlt answers every ternary code and every size now
        // (S-stretch-groups, S-stretch-halftone; tests/raster.rs pins the pixels). ("mirror-x" and "both-negative" still stop, with another word: their SOURCE rectangle of width -2 at x 0 reaches outside the 2x2 source)
        case("mirror-x", "K(fle_imp_StretchBlt, b, 0, 0, 2, 2, a, 0, 0, NEG(-2), 2, 0xCC0020)", "source rectangle is not inside the source"),
        case("both-negative", "K(fle_imp_StretchBlt, b, 0, 0, NEG(-2), 2, a, 0, 0, NEG(-2), 2, 0xCC0020)", "source rectangle is not inside the source"),
        case("arguments-in-the-message", "K(fle_imp_StretchBlt, b, 1, 2, 3, 4, a, 5, 6, 7, 8, 0xCC0020)", "3 x 4 to (1, 2) from 7 x 8 at (5, 6)"),
    ];
    stops("bulk-stretchblt-stops", setup, "StretchBlt", &v);
}

// ---- 9c. StretchDIBits (decision 0113 wall 6) ----

/// Learn (StretchDIBits): copies the colour data of a rectangle of a DIB to the destination rectangle, "the number of scan lines copied" is returned; (BITMAPINFO) a bottom-up
/// DIB has its origin at the lower-left corner. The answered form is a same-format 1:1 SRCCOPY into a DIB section. Rows are written as hexadecimal bytes in memory order.
#[test]
fn stretch_dibits_copies_a_same_format_one_to_one_rectangle_into_a_dib_section() {
    let l = lines("c1-stretchdibits", r#"
    uint64_t sdc = SCR(), b = MKDC();
    /* source: 4 x 4, 32 bits BI_RGB, top-down; pixel (r, c) is the bytes 16r + 4c + 1 .. +4 */
    mksrc(4, -4, 32, 0);
    uint64_t d1 = MKDIB(sdc, 4, -4, 32);
    SELO(b, d1);
    uint64_t r = SDIB(b, 1, 1, 2, 2, 1, 1, 2, 2, 0, 0xCC0020);
    printf("ret %llu\n", U(r));
    dump("top-down", d1);
    /* the same call into a bottom-up destination: the device rows are the memory rows in reverse order */
    uint64_t d2 = MKDIB(sdc, 4, 4, 32);
    SELO(b, d2);
    r = SDIB(b, 0, 0, 4, 1, 0, 3, 4, 1, 0, 0xCC0020);
    printf("ret %llu\n", U(r));
    dump("bottom-up-dest", d2);
"#);
    let z = "00000000000000000000000000000000";
    assert_eq!(
        l,
        [
            "ret 2".to_string(),
            format!("top-down 0 {z}"),
            "top-down 1 0000000015161718191a1b1c00000000".to_string(),
            "top-down 2 0000000025262728292a2b2c00000000".to_string(),
            format!("top-down 3 {z}"),
            "ret 1".to_string(),
            // destination row 0 of a bottom-up bitmap is its LAST memory row; source row 3 = bytes 49..64
            format!("bottom-up-dest 0 {z}"),
            format!("bottom-up-dest 1 {z}"),
            format!("bottom-up-dest 2 {z}"),
            "bottom-up-dest 3 3132333435363738393a3b3c3d3e3f40".to_string(),
        ]
    );
}

#[test]
fn stretch_dibits_reads_a_bottom_up_source_from_its_top_row_and_honours_the_viewport_origin() {
    let l = lines("c1-stretchdibits-bu", r#"
    uint64_t sdc = SCR(), b = MKDC();
    mksrc(2, 2, 32, 0);                         /* bottom-up 2 x 2: memory row 0 (bytes 1..8) is the BOTTOM row */
    uint64_t d1 = MKDIB(sdc, 3, -2, 32);
    SELO(b, d1);
    K(fle_imp_SetViewportOrgEx, b, 1, 0, 0);    /* device x = logical x + 1 */
    uint64_t r = SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020);
    printf("ret %llu\n", U(r));
    dump("d", d1);
"#);
    assert_eq!(
        l,
        [
            "ret 2".to_string(),
            // device row 0 is the top row of the source image = memory row 1 (bytes 9..16), shifted by one pixel
            "d 0 00000000090a0b0c0d0e0f10".to_string(),
            "d 1 000000000102030405060708".to_string(),
        ]
    );
    // 24 bits: the row padding of the destination is not written, the 9 pixel bytes of each row are
    let l = lines("c1-stretchdibits-24", r#"
    uint64_t sdc = SCR(), b = MKDC();
    mksrc(3, -2, 24, 0);
    uint64_t d1 = MKDIB(sdc, 3, -2, 24);
    SELO(b, d1);
    uint64_t r = SDIB(b, 0, 0, 3, 2, 0, 0, 3, 2, 0, 0xCC0020);
    printf("ret %llu\n", U(r));
    dump("d", d1);
    printf("pad %02x%02x%02x %02x%02x%02x\n", BITS(d1)[9], BITS(d1)[10], BITS(d1)[11], BITS(d1)[21], BITS(d1)[22], BITS(d1)[23]);
"#);
    // the source stride is 12 (9 + 3 padding): row 1 starts at byte 12 of the source, so its first byte is 13
    assert_eq!(l, ["ret 2", "d 0 010203040506070809", "d 1 0d0e0f101112131415", "pad 000000 000000"]);
    // BI_BITFIELDS 32 bits with the 8-8-8 masks, source and destination alike
    let l = lines("c1-stretchdibits-bf", r#"
    uint64_t sdc = SCR(), b = MKDC();
    mksrc(2, -1, 32, 3);
    { uint32_t m[3] = { 0xFF0000, 0xFF00, 0xFF }; memcpy(sbmi + 40, m, 12); }
    uint64_t d1 = MKDIBF(sdc, 2, -1, 32, 0xFF0000, 0xFF00, 0xFF);
    SELO(b, d1);
    printf("ret %llu\n", U(SDIB(b, 0, 0, 2, 1, 0, 0, 2, 1, 0, 0xCC0020)));
    dump("d", d1);
"#);
    assert_eq!(l, ["ret 1", "d 0 0102030405060708"]);
}

#[test]
fn stretch_dibits_forms_the_pages_do_not_fix_stop_by_name() {
    let setup = r#"    uint64_t sdc = SCR(), b = MKDC(), b2 = MKDC(), dead = MKDC(), d32 = MKDIB(sdc, 4, -4, 32), d24 = MKDIB(sdc, 4, -4, 24), d16 = MKDIB(sdc, 4, -4, 16), dd = MKBMP(4, 4);
    DELDC(dead);
    mksrc(4, -4, 32, 0);
    SELO(b, d32);
    SELO(b2, d24);"#;
    let ok = "0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020";
        // decision 0126 (walls 55, 94): old: these cases stopped and now answer (StretchDIBits copies into a device-dependent bitmap, stretches, mirrors, converts between formats and writes where the destination is partly outside:
    // S-source-outside, S-stretch-groups; tests/wall_fr11.rs and tests/raster.rs pin the pixels): rop-srcand (SRCAND is one of the 256 codes), destination-placeholder, destination-ddb, depth-differs, compression-differs,
    // stretch, compress, mirror-x, both-negative, zero-size, source-outside-right/-bottom/-left, bottom-up-partial, bottom-up-fewer-rows, destination-outside-right/-bottom/-left. The rest stop, with the words of the new messages
    // (old words: "only SRCCOPY" -> "rop is not one of the 256 ternary raster-operation codes", "hdc is a DC of GetDC(NULL)" -> "no window table is linked", "not a 24-bit or 32-bit RGB DIB" -> "not a 24- or 32-bit RGB form").
let cases = vec![
        case("rop-zero", "SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0)", "rop is not one of the 256 ternary raster-operation codes"),
        case("usage-pal-colors", "SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 1, 0xCC0020)", "iUsage is not DIB_RGB_COLORS"),
        case("usage-2", "SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 2, 0xCC0020)", "iUsage is not DIB_RGB_COLORS"),
        case("destination-screen-dc", "SDIB(sdc, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "no window table is linked"),
        case("destination-null", "SDIB(0, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "hdc is not a DC of this runtime"),
        case("destination-deleted", "SDIB(dead, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "hdc was deleted by DeleteDC"),
        case("destination-other-mapping-mode", "K(fle_imp_SetMapMode, b, 2); SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "hdc is in mapping mode 2"),
        case("lpbits-null", "K(fle_imp_StretchDIBits, b, 0, 0, 2, 2, 0, 0, 2, 2, 0, A(sbmi), 0, 0xCC0020)", "lpbmi or lpBits is NULL"),
        case("lpbmi-null", "K(fle_imp_StretchDIBits, b, 0, 0, 2, 2, 0, 0, 2, 2, A(simg), 0, 0, 0xCC0020)", "lpbmi or lpBits is NULL"),
        case("header-is-not-40", "{ uint32_t v = 12; memcpy(sbmi, &v, 4); } SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "biSize is not 40"),
        case("planes-2", "{ uint16_t v = 2; memcpy(sbmi + 12, &v, 2); } SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "biPlanes is not 1"),
        case("height-zero", "mksrc(4, 0, 32, 0); SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "biHeight is 0"),
        case("source-16-bits", "mksrc(4, -4, 16, 0); SELO(b, d16); SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "not a 24- or 32-bit RGB form"),
        case("destination-16-bits", "SELO(b, d16); SDIB(b, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0xCC0020)", "not a 24- or 32-bit RGB form"),
        case("arguments-in-the-message", "SDIB(b, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0x8800C6)", "3 x 4 to (1, 2) from 7 x 8 at (5, 6)"),
    ];
    let _ = ok;
    stops("c1-stretchdibits-stops", setup, "StretchDIBits", &cases);
    let l = lines("c1-stretchdibits-thread", r#"
    uint64_t b = MKDC();
    STOP("second-thread", on_second_thread(fle_imp_StretchDIBits));
"#);
    assert!(l[0].starts_with("second-thread | 6 | fle: StretchDIBits: ") && l[0].contains("second thread"), "{l:?}");
}

// ---- 10. The named stops ----

/// Decision 0123 (wall 46-49): the eight region calls of the list above that the window model answers now (fle_gdi.c: rectangles in Y-X band form, tests/wall_fr9.rs pins their results). Their rows stay where decision 0111 put
/// them (decision 0015), only the function behind the row changed from a stop to an answer.
const ANSWERED_BY_0123: [&str; 8] = ["CreateRectRgn", "CreateRectRgnIndirect", "SetRectRgn", "CombineRgn", "GetRgnBox", "OffsetRgn", "EqualRgn", "RectInRegion"];

/// Decision 0126 (walls 55 to 94): the sixteen calls of the list above that the pixel model answers now (runtime/fle_gdi.c, fle_raster.c; tests/wall_fr11.rs and tests/raster.rs pin their results). Their rows stay where
/// decision 0111 put them (decision 0015), only the function behind the row changed from a named stop to an answer. Found by comparing the GDI_STOP lines of the clean tree (59) and of this tree (43): exactly these 16.
/// The other two of the 61 that are answered by 0126 without having been a GDI_STOP line (StretchBlt and StretchDIBits answer other sizes now: S-stretch-groups) are not in the list, as they were answers before.
/// decision 0284 (B): the one row of NAMED_STOPS_0111 that decision 0191 turned into a function of its own, opt-in with the layer-4 marker row (wall ia2-W27); without the marker (a build that does not link the production import
/// table) it still stopped by name. The marker is gone and the function is in every build, so the call answers.
const ANSWERED_BY_0284: [&str; 1] = ["RectVisible"];

const ANSWERED_BY_0126: [&str; 16] = [
    "LineTo", "Rectangle", "GetPixel", "SetPixel", "SetPixelV", "GetBitmapBits", "SetDIBits", "SetDIBitsToDevice", "MaskBlt", "SelectClipRgn", "ExtSelectClipRgn", "IntersectClipRect", "ExcludeClipRect", "GetClipRgn",
    "GetRegionData", "FrameRect",
];

#[test]
fn the_sixty_one_calls_that_need_the_rasterizer_the_fonts_or_the_system_still_stop_by_name_and_eight_region_calls_answer_since_decision_0123() {
    // old: the sixty-nine calls ... stop by name (all 69 rows of NAMED_STOPS_0111 stopped when called with zeros)
    // decision 0126 (old: the 61 that the eight region calls left; 45 now: the sixteen of ANSWERED_BY_0126 answer)
    let still: Vec<&(&str, &str, &str)> = NAMED_STOPS_0111.iter().filter(|(_, n, _)| !ANSWERED_BY_0123.contains(n) && !ANSWERED_BY_0126.contains(n) && !ANSWERED_BY_0284.contains(n)).collect();
    assert_eq!(still.len(), 44);
    let mut body = String::new();
    for (_, n, _) in still.iter() {
        body.push_str(&format!("    STOP(\"{n}\", K(fle_imp_{n}, 0, 0, 0, 0));\n"));
    }
    let l = lines("bulk-named-stops", &body);
    assert_eq!(l.len(), 44);
    for (line, (_, n, word)) in l.iter().zip(still.iter().map(|t| **t)) {
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        assert_eq!(parts.len(), 3, "{n}: {line}");
        assert_eq!(parts[0], n);
        assert_eq!(parts[1], "6", "{n}: not SIGABRT: {line}");
        assert!(parts[2].starts_with(&format!("fle: {n}: ")), "{n}: {line}");
        assert!(parts[2].contains(word), "{n}: message lacks {word:?}: {line}");
        // decision 0126 (old: every message ended with "(decision 0111)"): the stops that decision 0126 rewrote (StretchDIBits and others) end with "(decision 0126)"
        assert!(parts[2].ends_with("(decision 0111)") || parts[2].ends_with("(decision 0126)"), "{n}: {line}");
    }
}

#[test]
fn a_second_thread_stops_each_of_the_answering_calls() {
    let mut body = String::new();
    for (_, n) in ANSWERS_0111 {
        body.push_str(&format!("    STOP(\"{n}\", MKDC(); on_second_thread(fle_imp_{n}));\n"));
    }
    let l = lines("bulk-second-thread", &body);
    assert_eq!(l.len(), 27);
    for (line, (_, n)) in l.iter().zip(ANSWERS_0111) {
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        assert_eq!(parts.len(), 3, "{n}: {line}");
        assert_eq!(parts[0], n);
        assert_eq!(parts[1], "6", "{n}: the second thread was not stopped: {line}");
        assert!(parts[2].starts_with(&format!("fle: {n}: ")) && parts[2].contains("second thread"), "{n}: {line}");
    }
    // the older calls that now do pixel work keep their thread rule
    let mut old = String::new();
    for n in ["BitBlt", "PatBlt", "FillRect"] {
        old.push_str(&format!("    STOP(\"{n}\", MKDC(); on_second_thread(fle_imp_{n}));\n"));
    }
    let l = lines("bulk-second-thread-pixels", &old);
    for (line, n) in l.iter().zip(["BitBlt", "PatBlt", "FillRect"]) {
        assert!(line.contains(&format!("| 6 | fle: {n}: ")) && line.contains("second thread"), "{line}");
    }
}

// ---- 11. Generator: the thunks call the runtime ----

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// A PE that imports `dll`!`api` only (slot 0x2200); entry 0x1000 calls the thunk at 0x1040 (`jmp [0x2200]`).
fn build_one(dll: &str, api: &str) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    // one descriptor: the DLL (OFT 0x2100, Name 0x2300, FirstThunk 0x2200); the second descriptor stays zero and ends the table
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }
    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_thunks_of_the_new_rows_call_the_runtime_and_the_names_are_part_of_the_match() {
    let va = IB + 0x1040;
    // every one of the 96 rows, with its own DLL name: the thunk calls the function and nothing is left unsupported
    for (dll, n) in ANSWERS_0111.iter().map(|(d, n)| (*d, *n)).chain(NAMED_STOPS_0111.iter().map(|(d, n, _)| (*d, *n))) {
        let img = build_one(dll, n);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert_eq!(st.handled[format!("jmp_imp:{n}").as_str()], 1, "{dll}!{n}");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{dll}!{n}");
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{dll}!{n} still unsupported");
        assert_eq!(st.unsupported_total, 0, "{dll}!{n}: {:?}", st.unsupported);
    }
    // another DLL, the A / W spelling and the neighbours are not these APIs
    for (dll, api) in [("other.dll", "MoveToEx"), ("gdi32.dll", "MoveToExA"), ("user32.dll", "MoveToEx"), ("gdi32.dll", "DestroyCursor"), ("user32.dll", "SaveDC"), ("gdi32.dll", "DrawTextW"), ("user32.dll", "LineTo"), ("gdi32.dll", "GetBkMode")] {   // decision 0126: old: ("gdi32.dll", "CreatePen") was here too; it is a row now (the wrong-dll rows stay)
        let img = build_one(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!c.contains(&format!("fle_imp_{api}")), "{dll}!{api}");
        assert_eq!(st.handled.keys().filter(|k| k.starts_with("jmp_imp:")).count(), 0, "{dll}!{api}");
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 12. Source checks ----

#[test]
fn the_source_of_decision_0111_pins_where_the_pixel_bytes_are_written() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_gdi.c")).unwrap());
    // the block of a DIB section is still allocated in one place and the runtime writes into it in exactly three: dib_fill (PatBlt, FillRect), the memmove of BitBlt
    // and, since decision 0113 wall 6, the memcpy of StretchDIBits (a same-format 1:1 SRCCOPY from the guest's image into a DIB section)
    // decision 0126 (old: the runtime wrote into a DIB section in exactly three places: `memmove(` of BitBlt, `memcpy(dib_row(` of StretchDIBits and `dib_fill(` of PatBlt and FillRect, with `calloc(1, (size_t)bytes)` once and
    // `if (n == 4) p[3] = 0;` for the unused byte): the pixels are written by fle_raster.c now, through fle_r_* functions and surfaces, so gdi.c has none of the three (0, 0, 0) and `calloc(1, (size_t)bytes)` appears twice
    // (CreateDIBSection and the block of a device-dependent bitmap, S-ddb-lazy). The rule that remains here: no indexing or storing through `bits` in fle_gdi.c.
    assert_eq!(code.matches("calloc(1, (size_t)bytes)").count(), 2);
    assert_eq!(code.matches("memmove(").count(), 0);
    assert_eq!(code.matches("memcpy(dib_row(").count(), 0);
    assert_eq!(code.matches("dib_fill(").count(), 0);
    assert!(!code.contains("->bits[") && !code.contains("*(uint8_t *)"));
    // decision 0126 (old: "GetDIBits and RealizePalette still touch no pixel": both stopped through fle_stop): RealizePalette still does; GetDIBits answers now (it writes the lines of a DDB into the guest's buffer through fle_st
    // and stops through pixel_stop on the forms the page leaves open)
    {
        let a = code.find("void fle_imp_RealizePalette(void)").expect("RealizePalette");
        let end = a + code[a..].find("\n}\n").unwrap();
        let body = &code[a..end];
        assert!(body.contains("fle_stop(api") && !body.contains("fle_st(") && !body.contains("memcpy") && !body.contains("memset") && !body.contains("->bits"), "RealizePalette");
    }
    // the 69 named stops are one macro line each and touch nothing
    // Decision 0123 (old: 68): the eight region calls of ANSWERED_BY_0123 are functions of their own now: the definition of the macro and 59 uses
    // decision 0126 (old: 60 = the definition and 59 uses): the sixteen calls of ANSWERED_BY_0126 are functions of their own now: the definition of the macro and 43 uses (found by comparing the clean tree and this tree)
    // decision 0129 (old: 44): the six text APIs (ExtTextOutW, GetTextMetricsW, GetTextExtentPoint32W, GetTextExtentPointW, DrawTextW, DrawTextExW) are `GDI_TEXT_STOP` now (they call the weak fle_text_hook, then stop by name
    // as before; tests/wall_fr12.rs): 44 - 6 = 38 = the definition of the macro and 37 uses
    // decision 0191 (old: 38): RectVisible is a function of its own now (opt-in with the layer-4 marker; the stop of decision 0111 remains without it): the definition of the macro and 36 uses
    assert_eq!(code.matches("GDI_STOP(").count(), 37, "the definition of the macro and 36 uses (g1-optional-stretchblt.diff: StretchBlt is a function of its own; decision 0113: so is StretchDIBits; decision 0123: eight region calls; decision 0126: sixteen more; decision 0191: RectVisible)");
    // the cursor table is reached through a weak reference only
    assert!(code.contains("__attribute__((weak))") && code.contains("fle_cursor_image_idc_of"));
    // nothing here reads the host: the older source check of tests/gdi.rs covers the forbidden words; this one pins the new includes
    assert!(!code.contains("#include <unistd.h>") && !code.contains("#include <sys/"));
}
