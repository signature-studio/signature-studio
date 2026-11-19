//! Tests of decision 0119 (docs/decisions/0119-window-model-and-fruity-walls.md): the window model of walls 9..17 of CreateFruityInstance (user32!SendMessageW, SetClassLongPtrW,
//! GetWindowLongPtrW, SetWindowLongPtrW (GWL_EXSTYLE, GWLP_USERDATA), SetWindowTextW, GetSystemMenu, DeleteMenu, DefWindowProcW (WM_SETICON, WM_SETTEXT), the TApplication form of
//! CreateWindowExW (style 0x84ca0000), the thunk of MakeObjectInstance as a window procedure, wtsapi32 and uxtheme as virtual modules N = 37 and 38) and PF 2 / PF 3 of
//! IsProcessorFeaturePresent.
//!
//! Expected values are written from the Microsoft Learn pages named in the decision (SendMessageW, SetClassLongPtrW, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowTextW,
//! WM_SETICON, WM_SETTEXT, GetSystemMenu, DeleteMenu, WM_SYSCOMMAND, Window Styles, WM_NCCALCSIZE, WTSRegisterSessionNotification, BufferedPaintInit, IsProcessorFeaturePresent,
//! System Error Codes) and the choices of the decision, not read back from the C code. EVERY ANSWER IS PROVISIONAL: nothing is compared with a real Windows. The values that no
//! page gives (the last error 1400 for a window that is not in the table, the handle regions, the six items of the window menu) are choices of the decision and are the only
//! thing such a test pins down.
//!
//! 1. Generator: the new rows are the last rows, in the order the walls were reached; no pseudo-VA moved.
//! 2. The driver (fle_win.c, fle_class.c, fle_callback.c, fle_imports.c, fle_env.c, plus a test-side fle_sys_region_at): behaviour.
//! 3. Source checks.
//! 4. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when absent).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_FR6_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR6/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator: a synthetic PE that imports the names, to see the rows of the response table
// ---------------------------------------------------------------------------------------------

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

fn build_imports(imps: &[(&str, &str)]) -> Vec<u8> {
    assert!(imps.len() <= 6);
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
    w32(&mut b, o + 124, 20 * (imps.len() as u32 + 1));
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
    let d = data_at(DATA_RVA);
    for (k, (dll, name)) in imps.iter().enumerate() {
        let k32 = k as u32;
        let (oft, ft, dll_rva, hn_rva) = (0x2100 + 0x10 * k32, 0x2200 + 0x10 * k32, 0x2300 + 0x20 * k32, 0x2500 + 0x40 * k32);
        w32(&mut b, d + 20 * k, oft);
        w32(&mut b, d + 20 * k + 12, dll_rva);
        w32(&mut b, d + 20 * k + 16, ft);
        put(&mut b, dll_rva, format!("{dll}\0").as_bytes());
        put(&mut b, hn_rva, &[&[0u8, 0u8][..], name.as_bytes(), &[0u8]].concat());
        w64(&mut b, data_at(oft), hn_rva as u64);
        w64(&mut b, data_at(ft), hn_rva as u64);
    }
    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..imps.len() as u32 {
        let at = 0x1000 + 5 * k;
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k)].concat()));
        code.push((0x1040 + 0x10 * k, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k, 0x2200 + 0x10 * k)].concat()));
    }
    code.push((0x1000 + 5 * imps.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen(imps: &[(&str, &str)]) -> (fe_lift::Stats, String) {
    let img = build_imports(imps);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

const GUEST_IMPORTS: [(&str, &str); 2] = [("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW")];

fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").expect("no response table");
    let end = start + c[start..].find("};").unwrap();
    c[start..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("{ \"")?;
            let (dll, rest) = l.split_once("\", \"")?;
            let (name, rest) = rest.split_once("\", UINT64_C(0x")?;
            let (va, _) = rest.split_once(')')?;
            Some((dll.to_string(), name.to_string(), u64::from_str_radix(va, 16).unwrap()))
        })
        .collect()
}

/// The eight rows of decision 0119 in the order the walls were reached (decision 0015: append only).
const NEW_ROWS: [(&str, &str); 8] = [
    ("user32.dll", "SendMessageW"),
    ("user32.dll", "SetClassLongPtrW"),
    ("wtsapi32.dll", "WTSRegisterSessionNotification"),
    ("uxtheme.dll", "BufferedPaintInit"),
    ("user32.dll", "GetSystemMenu"),
    ("user32.dll", "DeleteMenu"),
    ("user32.dll", "GetWindowLongPtrW"),
    ("user32.dll", "SetWindowTextW"),
];

#[test]
fn the_eight_new_rows_are_a_block_of_eight_in_the_order_they_were_reached_and_no_pseudo_va_moved() {
    // Decision 0122 appends 15 rows after these eight (old name: `..._are_the_last_eight_...`, which asserted the eight at the END of the table); the block is now found by its
    // first row, as tests/wall_s.rs does for decision 0099's rows. The intent is the same: decision 0015, append only, no pseudo-VA moves.
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let at = rows.iter().position(|(d, nm, _)| (d.as_str(), nm.as_str()) == NEW_ROWS[0]).expect("the first new row");
    // decision 0015: append only. The row of decision 0118 (rpcrt4.dll!UuidCreate) is the one right before the eight of this decision.
    assert_eq!((rows[at - 1].0.as_str(), rows[at - 1].1.as_str()), ("rpcrt4.dll", "UuidCreate"));
    for (k, (dll, name)) in NEW_ROWS.iter().enumerate() {
        assert_eq!((rows[at + k].0.as_str(), rows[at + k].1.as_str()), (*dll, *name), "row {}", at + k);
    }
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    for (_, name) in NEW_ROWS {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
    // IsProcessorFeaturePresent still has no row (decision 0078, tests/sysinfo.rs: GetProcAddress does not find it): PF 2 / PF 3 changed the function, not the table
    assert!(rows.iter().all(|(_, n2, _)| n2 != "IsProcessorFeaturePresent"));
}

#[test]
fn each_new_thunk_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in NEW_ROWS {
        let other = if dll == "user32.dll" { "kernel32.dll" } else { "user32.dll" };
        let (st, c) = gen(&[(dll, name), (other, name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
        for k in [1u64, 2] {
            let va = IB + 0x1040 + 0x10 * k;
            assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{name}: thunk {k} stays unsupported");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver: fle_win.c + fle_class.c (+ fle_callback.c, fle_imports.c, fle_env.c) and a main() with stub window procedures
// ---------------------------------------------------------------------------------------------

fn clang_obj(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd.arg("-I").arg(src_dir()).arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-fr6"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [("fle_win.c", true), ("fle_class.c", true), ("fle_callback.c", false), ("fle_imports.c", false), ("fle_env.c", false)] {
            let base = if ours { src_dir() } else { rt_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_obj(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

const PRELUDE: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stddef.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_win.h"

uint64_t R[16];
static uint64_t stk[0x2000] __attribute__((aligned(4096)));          /* 64 KiB */
#define E0 ((uint64_t)(uintptr_t)stk + 0x8000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define BAD UINT64_C(0xBADBADBAD)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))

static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }

static uint64_t S(const char *s)
{
    static uint16_t b[8][400];
    static unsigned k;
    uint16_t *p = b[k++ % 8];
    size_t i, n = strlen(s);
    for (i = 0; i < n; i++) p[i] = (uint8_t)s[i];
    p[n] = 0;
    return A(p);
}

/* ---- a VirtualAlloc region as the guest's MakeObjectInstance makes it: the tests fill the block; fle_sys_region_at is the test's own ---- */
static uint8_t blk[0x1000] __attribute__((aligned(4096)));
static int blk_on;
static uint8_t blk_pages[1] = { 0x40 };
int fle_sys_region_at(uint64_t addr, fle_sys_region_view *out)
{
    if (!blk_on || addr < A(blk) || addr >= A(blk) + sizeof blk) return 0;
    out->base = A(blk); out->size = sizeof blk; out->alloc_protect = 0x40; out->pages = blk_pages;
    return 1;
}
/* the block of Delphi's MakeObjectInstance (decision 0071 investigation 3.3): [blk+8..16) the template `pop r11; jmp [rip+0]`, [blk+0x10] the guest function, slots of 0x15 bytes at blk+0x18 */
#define SLOT0 (A(blk) + 0x18)
static void make_block(uint64_t target_fn)
{
    static const uint8_t tpl[8] = { 0x41, 0x5b, 0xff, 0x25, 0, 0, 0, 0 };
    int32_t d;
    memset(blk, 0, sizeof blk);
    memcpy(blk + 8, tpl, 8);
    memcpy(blk + 0x10, &target_fn, 8);
    blk[0x18] = 0xe8;
    d = (int32_t)((int64_t)(A(blk) + 8) - (int64_t)(SLOT0 + 5));
    memcpy(blk + 0x19, &d, 4);
    blk_on = 1;
    blk_pages[0] = 0x40;
}

/* ---- the guest: stub window procedures in a replacement dispatch table ---- */
#define VA_REC    0x2000      /* records every message; answers 1 to WM_NCCREATE, 0 to the others, unless `fail` says otherwise */
#define VA_COMMON 0x52a570    /* the common stub of MakeObjectInstance: r11 = slot + 5, gets (hwnd, msg, wParam, lParam) */
#define VA_DEF    0x424920    /* the guest's DefWindowProcW thunk */
typedef struct { int tag; uint64_t h, msg, wp, lp, r11; int32_t rect[4]; } ent;
static ent lg[256];
static unsigned ln;
static uint64_t send_result = 0;   /* what VA_REC answers to a message other than the three of CreateWindowExW */
static uint64_t nccreate_result = 1;
static void cb_rec(void)
{
    ent *e = &lg[ln++];
    e->tag = 1; e->h = R[1]; e->msg = R[2]; e->wp = R[8]; e->lp = R[9]; e->r11 = R[11];
    if (e->msg == 0x83 && e->lp) memcpy(e->rect, (void *)(uintptr_t)e->lp, 16);
    R[0] = e->msg == 0x81 ? nccreate_result : (e->msg == 0x83 || e->msg == 0x01) ? 0 : send_result;
}
static void cb_common(void)
{
    ent *e = &lg[ln++];
    e->tag = 2; e->h = R[1]; e->msg = R[2]; e->wp = R[8]; e->lp = R[9]; e->r11 = R[11];
    R[0] = 0x5151;
}
static void cb_def(void) { fle_imp_DefWindowProcW(); }
const fle_fn_entry fle_dispatch_table[] = { { VA_REC, cb_rec }, { VA_DEF, cb_def }, { VA_COMMON, cb_common } };
const uint64_t fle_dispatch_count = 3;

static uint64_t callv(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d)
{
    uint64_t keep4 = R[4], r;
    memset(stk, 0, 0x100);
    stk[0] = RET0;
    R[1] = a; R[2] = b; R[8] = c; R[9] = d;
    R[4] = A(stk);
    R[0] = BAD;
    f();
    r = R[0];
    R[4] = keep4;
    return r;
}
#define CALL(f, ...) callv((void (*)(void))(f), __VA_ARGS__)

static uint8_t wcb[72];
static uint64_t REGP(const char *name, uint64_t proc)
{
    uint32_t v;
    uint64_t hi = 0x400000, nm = S(name);
    memset(wcb, 0, sizeof wcb);
    v = 3; memcpy(wcb + 0, &v, 4); memcpy(wcb + 8, &proc, 8);
    memcpy(wcb + 24, &hi, 8); memcpy(wcb + 64, &nm, 8);
    return CALL(fle_imp_RegisterClassW, A(wcb), 0, 0, 0);
}

static uint64_t cw_raw(uint64_t ex, uint64_t cls, uint64_t name, uint64_t style, uint64_t x, uint64_t y, uint64_t w, uint64_t h,
                       uint64_t parent, uint64_t menu, uint64_t hinst, uint64_t param)
{
    uint64_t save4 = R[4], call_sp = (R[4] - 0x80) & ~UINT64_C(15), in_sp = call_sp - 8, r;
    fle_st(call_sp + 0x20, 8, x); fle_st(call_sp + 0x28, 8, y); fle_st(call_sp + 0x30, 8, w); fle_st(call_sp + 0x38, 8, h);
    fle_st(call_sp + 0x40, 8, parent); fle_st(call_sp + 0x48, 8, menu); fle_st(call_sp + 0x50, 8, hinst); fle_st(call_sp + 0x58, 8, param);
    fle_st(in_sp, 8, RET0);
    R[1] = ex; R[2] = cls; R[8] = name; R[9] = style; R[4] = in_sp; R[0] = BAD;
    fle_imp_CreateWindowExW();
    r = R[0];
    if (R[4] != in_sp) printf("R4-NOT-RESTORED\n");
    R[4] = save4;
    return r;
}
static uint16_t empty[1] = { 0 };
#define CW_TOOL(cls) cw_raw(0x80, S(cls), A(empty), 0x80000000u, 0, 0, 0, 0, 0, 0, 0x400000, 0)
/* the call of the guest at 0x867f7f: TApplication (ex 0x80, style 0x84ca0000, x = SM_CXSCREEN / 2, y = SM_CYSCREEN / 2, size 0 x 0, hInstance 0x400000) */
#define CW_APP(x, y, title) cw_raw(0x80, S("TApplication"), S(title), 0x84ca0000u, x, y, 0, 0, 0, 0, 0x400000, 0)

static void pw(uint64_t h)
{
    fle_win_view v;
    if (!fle_win_get(h, &v)) { printf("win %llx none\n", U(h)); return; }
    printf("win %llx style=%x ex=%x rect=%d,%d,%d,%d live=%d text_len=%u big=%llx small=%llx user=%llx wts=%d/%u\n", U(h), (unsigned)v.style, (unsigned)v.exstyle,
           (int)v.x, (int)v.y, (int)v.cx, (int)v.cy, v.live, (unsigned)v.text_len, U(v.icon_big), U(v.icon_small), U(v.userdata), v.wts_registered, (unsigned)v.wts_flags);
}
"##;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n    R[4] = E0;\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(rt_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, "", body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn want(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle: <api>: " message that contains `word`, and no output after it. `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, api: &str, word: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run_src(&format!("{prefix}-{label}"), "", &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(&format!("fle: {api}: ")), "{label}: no 'fle: {api}' message: {}", o.stderr);
        assert!(o.stderr.contains(word), "{label}: message lacks {word:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

/// The guest's own window: class TApplication with the class procedure DefWindowProcW's thunk (WNDCLASSW.lpfnWndProc = 0x424920, measured), created as f_867e10 does.
const APP: &str = "uint64_t h; REGP(\"TApplication\", VA_DEF); h = CW_APP(960, 540, \"FL Studio\"); ln = 0;";

// ---- wall 9: SendMessageW / SetClassLongPtrW (PM's decision 1: a CHOICE WITHOUT A SOURCE: 0 and last error 1400) ----

#[test]
fn send_message_and_set_class_long_ptr_answer_zero_and_1400_for_a_window_that_is_not_in_the_table() {
    let l = lines(
        "fr6-null-hwnd",
        r#"
    uint64_t r, h;
    REGP("TApplication", VA_DEF); h = CW_APP(960, 540, "FL Studio"); ln = 0;
    /* the guest's two calls of wall 9: SendMessageW(NULL, WM_SETICON, ICON_BIG, hIcon), SetClassLongPtrW(NULL, GCLP_HICON, hIcon) */
    SETLE(0x1111);
    r = CALL(fle_imp_SendMessageW, 0, 0x80, 1, 0x7fba0624); printf("send-null %llu le %u\n", U(r), LE());
    SETLE(0x2222);
    r = CALL(fle_imp_SetClassLongPtrW, 0, 0xfffffff2u, 0x7fba0624, 0); printf("setclass-null %llu le %u\n", U(r), LE());
    {
        static const uint64_t bad[] = { 0x7F990000, 0x7F990002, 0x7F99000C, 0x7F98FFFC, 0x17F990004ull, 0xFFFFFFFF7F990004ull, 0x7FA90004, 0x7FBA0004 };
        unsigned i;
        for (i = 0; i < sizeof bad / sizeof bad[0]; i++) {
            SETLE(0x3333);
            r = CALL(fle_imp_SendMessageW, bad[i], 0x80, 1, 5);
            printf("send %llx -> %llu le %u\n", U(bad[i]), U(r), LE());
            SETLE(0x4444);
            r = CALL(fle_imp_SetClassLongPtrW, bad[i], 0xfffffff2u, 5, 0);
            printf("setclass %llx -> %llu le %u\n", U(bad[i]), U(r), LE());
        }
    }
"#,
    );
    let mut w: Vec<String> = vec!["send-null 0 le 1400".into(), "setclass-null 0 le 1400".into()];
    for b in ["7f990000", "7f990002", "7f99000c", "7f98fffc", "17f990004", "ffffffff7f990004", "7fa90004", "7fba0004"] {
        w.push(format!("send {b} -> 0 le 1400"));
        w.push(format!("setclass {b} -> 0 le 1400"));
    }
    assert_eq!(l, w);
}

#[test]
fn a_window_whose_creation_failed_is_not_a_live_window_for_send_message() {
    // Learn WM_NCCREATE: FALSE makes CreateWindowEx return a NULL handle; the window stays in the table as not live (decision 0071)
    let l = lines(
        "fr6-dead",
        r#"
    uint64_t r, h;
    REGP("TPUtilWindow", VA_REC);
    nccreate_result = 0;
    h = CW_TOOL("TPUtilWindow");
    printf("h %llu count %llu msgs %u\n", U(h), U(fle_win_count()), ln);
    SETLE(7);
    r = CALL(fle_imp_SendMessageW, 0x7F990004, 0x80, 1, 5);
    printf("send %llu le %u msgs %u\n", U(r), LE(), ln);
"#,
    );
    assert_eq!(l, want(&["h 0 count 1 msgs 1", "send 0 le 1400 msgs 1"]));
}

#[test]
fn send_message_to_a_window_of_the_table_calls_its_window_procedure_and_returns_the_whole_result() {
    let l = lines(
        "fr6-send",
        r#"
    uint64_t r, h;
    REGP("TPUtilWindow", VA_REC);
    h = CW_TOOL("TPUtilWindow");
    ln = 0;
    SETLE(0x6666);
    send_result = UINT64_C(0x123456789abcdef0);
    r = CALL(fle_imp_SendMessageW, h, 0xDEADBEEF00000400ull, 0x1111222233334444ull, 0x5555666677778888ull);
    printf("r %llx le %x msgs %u\n", U(r), LE(), ln);
    printf("m h %llx msg %llx wp %llx lp %llx\n", U(lg[0].h), U(lg[0].msg), U(lg[0].wp), U(lg[0].lp));
    printf("depth %u r4 %d\n", fle_callback_depth(), R[4] == E0);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "r 123456789abcdef0 le 6666 msgs 1",
            // Msg is a UINT: the upper half of rdx is not part of it
            "m h 7f990004 msg 400 wp 1111222233334444 lp 5555666677778888",
            "depth 0 r4 1",
        ])
    );
}

#[test]
fn send_message_to_hwnd_broadcast_stops_by_name() {
    // HWND_BROADCAST (0xffff) is defined by the SendMessage page (all top-level windows): not modelled
    assert_aborts("fr6-send-bc", "", "SendMessageW", "HWND_BROADCAST", &[("broadcast", "CALL(fle_imp_SendMessageW, 0xFFFF, 0x80, 1, 5);")]);
}

#[test]
fn the_window_procedure_of_a_live_window_that_is_not_an_entry_stops_by_name_in_send_message() {
    let o = run_src(
        "fr6-send-nonentry",
        "",
        "    uint64_t h; REGP(\"TPUtilWindow\", VA_REC); h = CW_TOOL(\"TPUtilWindow\"); CALL(fle_imp_SetWindowLongPtrW, h, 0xFFFFFFFCull, 0x12345, 0);\n    CALL(fle_imp_SendMessageW, h, 0x80, 1, 5);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned") && o.signal == Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("fle: indirect call to 0x12345 is not a known function entry"), "{}", o.stderr);
}

// ---- wall 10: the TApplication form of CreateWindowExW (style 0x84ca0000) ----

#[test]
fn create_window_answers_the_tapplication_form_and_sends_the_three_messages_with_the_proposed_rectangle() {
    let l = lines(
        "fr6-app",
        r#"
    uint64_t h;
    REGP("TApplication", VA_REC);
    SETLE(0x1234);
    h = CW_APP(960, 540, "FL Studio");
    printf("h %llx count %llu msgs %u le %x\n", U(h), U(fle_win_count()), ln, LE());
    pw(h);
    printf("m0 msg %llx wp %llx; m1 msg %llx wp %llx; m2 msg %llx wp %llx\n", U(lg[0].msg), U(lg[0].wp), U(lg[1].msg), U(lg[1].wp), U(lg[2].msg), U(lg[2].wp));
    /* Learn WM_NCCALCSIZE: on entry the RECT holds the proposed window rectangle: (x, y, x + cx, y + cy) */
    printf("rect %d %d %d %d\n", lg[1].rect[0], lg[1].rect[1], lg[1].rect[2], lg[1].rect[3]);
"#,
    );
    assert_eq!(
        l,
        want(&[
            "h 7f990004 count 1 msgs 5 le 1234",   // decision 0219: a WS_POPUP window also gets WM_SIZE and WM_MOVE after WM_CREATE (was msgs 3); the first three are still NCCREATE, NCCALCSIZE, CREATE below
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=9 big=0 small=0 user=0 wts=0/0",
            "m0 msg 81 wp 0; m1 msg 83 wp 0; m2 msg 1 wp 0",
            "rect 960 540 960 540",
        ])
    );
}

#[test]
fn the_tool_window_form_of_decision_0071_still_has_the_rectangle_0_0_0_0() {
    let l = lines(
        "fr6-tool",
        r#"
    uint64_t h;
    REGP("TPUtilWindow", VA_REC);
    h = CW_TOOL("TPUtilWindow");
    printf("h %llx rect %d %d %d %d\n", U(h), lg[1].rect[0], lg[1].rect[1], lg[1].rect[2], lg[1].rect[3]);
    pw(h);
"#,
    );
    assert_eq!(l, want(&["h 7f990004 rect 0 0 0 0", "win 7f990004 style=80000000 ex=80 rect=0,0,0,0 live=1 text_len=0 big=0 small=0 user=0 wts=0/0"]));
}

/// Decision 0123 (old: `the_tapplication_form_stops_for_cw_usedefault_a_size_a_parent_and_a_long_title`: CW_USEDEFAULT, a size, a parent, lpParam, another extended style, WS_VISIBLE and WS_THICKFRAME all stopped the
/// TApplication form): the form is not special any more, the parameters are kept as the Learn page of CreateWindowExW gives them. What still stops: a non-NULL hMenu for a window that is not a child, an hInstance that is not
/// the lifted DLL, a title longer than 256 units.
#[test]
fn the_tapplication_form_keeps_cw_usedefault_a_size_a_parent_and_a_long_title_as_the_page_gives_them_since_decision_0123() {
    let setup = "REGP(\"TApplication\", VA_REC);";
    let l = lines(
        "fr6-app-answered",
        &format!(
            r#"
    {setup}
    fle_win_view v;
    uint64_t h;
    SETLE(0);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 0x80000000u, 0, 100, 100, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("x-default %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 5, 0x80000000u, 100, 100, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("y-default %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 5, 6, 0x80000000u, 100, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("width-default %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 5, 6, 100, 0x80000000u, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("height-default %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 5, 6, 100, 200, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("size %d %d %d %d\n", v.x, v.y, v.cx, v.cy);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 0, 0, 0, 0, 0x7F990004ull, 0, 0x400000, 0);
    printf("parent live %d\n", h != 0);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 0, 0, 0, 0, 0x7F99FFF0ull, 0, 0x400000, 0);
    printf("parent not in table %llu le %u\n", U(h), LE());
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ca0000u, 0, 0, 0, 0, 0, 0, 0x400000, 1);
    printf("param live %d\n", h != 0);
    h = cw_raw(0x88, S("TApplication"), S("FL Studio"), 0x84ca0000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("ex %x\n", (unsigned)v.exstyle);
    h = cw_raw(0x80, S("TApplication"), S("FL Studio"), 0x84ce0000u, 0, 0, 0, 0, 0, 0, 0x400000, 0);
    fle_win_get(h, &v); printf("thickframe style %x live %d\n", (unsigned)v.style, v.live);
"#
        ),
    );
    assert_eq!(
        l[..8],
        [
            "x-default 0 0 100 100",
            // Learn: the system ignores y only when x is CW_USEDEFAULT and nHeight only when nWidth is (for a pop-up window both become 0). The page gives no meaning to a y or an nHeight of CW_USEDEFAULT alone: it is kept as given (-2147483648 = 0x80000000), a choice listed with S-default
            "y-default 5 -2147483648 100 100",
            "width-default 5 6 0 0",
            "height-default 5 6 100 -2147483648",
            "size 5 6 100 200",
            "parent live 1",
            "parent not in table 0 le 1400",
            "param live 1",
        ]
    );
    assert_eq!(l[8..], ["ex 88", "thickframe style 84ce0000 live 1"]);
    let one = |label: &str, stmt: &str, word: &str| assert_aborts("fr6-app-stop", setup, "CreateWindowExW", word, &[(label, stmt)]);
    one("menu", "cw_raw(0x80, S(\"TApplication\"), S(\"FL Studio\"), 0x84ca0000u, 960, 540, 0, 0, 0, 1, 0x400000, 0);", "hMenu is not NULL");
    one("hinst", "cw_raw(0x80, S(\"TApplication\"), S(\"FL Studio\"), 0x84ca0000u, 960, 540, 0, 0, 0, 0, 0, 0);", "hInstance is not the lifted DLL");
    // a title of 257 units: longer than 256 units stops (a limit of this runtime, not of the page)
    assert_aborts(
        "fr6-app-title",
        &format!("{setup} static uint16_t big[300]; {{ unsigned i; for (i = 0; i < 299; i++) big[i] = 'x'; big[257] = 0; }}"),
        "CreateWindowExW",
        "longer than 256 units",
        &[("long", "cw_raw(0x80, S(\"TApplication\"), A(big), 0x84ca0000u, 960, 540, 0, 0, 0, 0, 0x400000, 0);")],
    );
}

// ---- wall 14: DefWindowProcW(WM_SETICON) ; wall 17: WM_SETTEXT ; SetWindowTextW ----

#[test]
fn wm_seticon_through_the_class_procedure_returns_the_previous_icon_per_wparam() {
    // Learn WM_SETICON: wParam ICON_BIG 1 / ICON_SMALL 0; lParam the new handle (NULL removes); "The return value is a handle to the previous large or small icon ... NULL if the
    // window previously had no icon of the type indicated by wParam"; DefWindowProc does the same
    let l = lines(
        "fr6-seticon",
        &format!(
            r#"
    {APP}
    uint64_t a, b, c, d, e;
    SETLE(0x7777);
    a = CALL(fle_imp_SendMessageW, h, 0x80, 1, 0x7fba0624);            /* what f_86b680 does */
    b = CALL(fle_imp_SendMessageW, h, 0x80, 1, 0x7fba0628);
    c = CALL(fle_imp_SendMessageW, h, 0x80, 0, 0x7fba062c);            /* the small icon is separate */
    d = CALL(fle_imp_SendMessageW, h, 0x80, 1, 0);                     /* NULL removes the big icon: the previous one comes back */
    e = CALL(fle_imp_SendMessageW, h, 0x80, 1, 0x7fba0630);
    printf("%llx %llx %llx %llx %llx le %x\n", U(a), U(b), U(c), U(d), U(e), LE());
    pw(h);
"#
        ),
    );
    assert_eq!(
        l,
        want(&[
            "0 7fba0624 0 7fba0628 0 le 7777",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=9 big=7fba0630 small=7fba062c user=0 wts=0/0",
        ])
    );
}

#[test]
fn wm_seticon_with_another_wparam_stops_by_name() {
    assert_aborts("fr6-seticon-stop", APP, "DefWindowProcW", "WM_SETICON with wParam", &[("two", "CALL(fle_imp_SendMessageW, h, 0x80, 2, 5);"), ("big-with-high-bits", "CALL(fle_imp_SendMessageW, h, 0x80, 0x100000001ull, 5);")]);
}

#[test]
fn set_window_text_sets_the_text_through_wm_settext_and_null_is_the_empty_string() {
    let l = lines(
        "fr6-settext",
        &format!(
            r#"
    {APP}
    uint64_t r;
    SETLE(0x8888);
    r = CALL(fle_imp_SetWindowTextW, h, S("Hello"), 0, 0);
    printf("r %llu le %x\n", U(r), LE()); pw(h);
    r = CALL(fle_imp_SetWindowTextW, h, 0, 0, 0);
    printf("r %llu le %x\n", U(r), LE()); pw(h);
    r = CALL(fle_imp_SendMessageW, h, 0x0C, 0, S("via send"));
    printf("send r %llu\n", U(r)); pw(h);
"#
        ),
    );
    assert_eq!(
        l,
        want(&[
            "r 1 le 8888",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=5 big=0 small=0 user=0 wts=0/0",
            "r 1 le 8888",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=0 big=0 small=0 user=0 wts=0/0",
            "send r 1",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=8 big=0 small=0 user=0 wts=0/0",
        ])
    );
}

#[test]
fn set_window_text_for_a_window_that_is_not_in_the_table_is_zero_and_1400_and_a_long_text_stops() {
    let l = lines(
        "fr6-settext-bad",
        r#"
    uint64_t r;
    SETLE(1);
    r = CALL(fle_imp_SetWindowTextW, 0, S("x"), 0, 0);
    printf("null %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_SetWindowTextW, 0x7F990004, S("x"), 0, 0);
    printf("never-issued %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(l, want(&["null 0 le 1400", "never-issued 0 le 1400"]));
    assert_aborts(
        "fr6-settext-long",
        &format!("{APP} static uint16_t big[300]; {{ unsigned i; for (i = 0; i < 299; i++) big[i] = 'x'; big[257] = 0; }}"),
        "DefWindowProcW",
        "longer than 256 units",
        &[("long", "CALL(fle_imp_SetWindowTextW, h, A(big), 0, 0);")],
    );
}

// ---- wall 15: SetClassLongPtrW(GCLP_HICON) ----

#[test]
fn set_class_long_ptr_replaces_the_icon_of_the_class_and_returns_the_previous_one() {
    // Learn SetClassLongPtrW: GCLP_HICON -14 "Replaces a handle to the icon associated with the class"; "the return value is the previous value of the specified offset"
    let l = lines(
        "fr6-setclass",
        &format!(
            r#"
    {APP}
    uint64_t a, b, c;
    uint8_t out[72];
    SETLE(0x9999);
    a = CALL(fle_imp_SetClassLongPtrW, h, 0xfffffff2u, 0x7fba0624, 0);
    b = CALL(fle_imp_SetClassLongPtrW, h, 0xfffffff2u, 0x7fba0628, 0);
    printf("%llx %llx le %x\n", U(a), U(b), LE());
    /* GetClassInfoW reports the new hIcon (WNDCLASSW.hIcon at offset 32) */
    memset(out, 0, sizeof out);
    c = CALL(fle_imp_GetClassInfoW, 0x400000, S("TApplication"), A(out), 0);
    printf("gci %llu hIcon %llx\n", U(c), U(*(uint64_t *)(out + 32)));
"#
        ),
    );
    assert_eq!(l, want(&["0 7fba0624 le 9999", "gci 1 hIcon 7fba0628"]));
}

#[test]
fn set_class_long_ptr_stops_for_every_other_index_of_a_live_window() {
    let cases: Vec<(String, String)> = [("hcursor", -12i64), ("hbrbackground", -10), ("wndproc", -24), ("style", -26), ("cbclsextra", -20), ("hiconsm", -34), ("hmodule", -16), ("menuname", -8), ("extra-0", 0), ("minus-1", -1)]
        .iter()
        .map(|(l, i)| (l.to_string(), format!("CALL(fle_imp_SetClassLongPtrW, h, (uint64_t)(uint32_t)(int32_t){i}, 5, 0);")))
        .collect();
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts("fr6-setclass-stop", APP, "SetClassLongPtrW", "is not GCLP_HICON", &refs);
}

// ---- wall 17: GetWindowLongPtrW / SetWindowLongPtrW (GWL_EXSTYLE, GWLP_USERDATA) ----

#[test]
fn get_window_long_ptr_gives_the_stored_values_and_set_window_long_ptr_returns_the_previous_ones() {
    let l = lines(
        "fr6-gwl",
        &format!(
            r#"
    {APP}
    uint64_t r;
    SETLE(0xAAAA);
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xffffffecu, 0, 0); printf("exstyle %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffff0u, 0, 0); printf("style %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffffau, 0, 0); printf("hinstance %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffff8u, 0, 0); printf("hwndparent %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xffffffebu, 0, 0); printf("userdata %llx\n", U(r));   /* GWLP_USERDATA -21 */
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffffcu, 0, 0); printf("wndproc %llx\n", U(r));
    /* the guest's f_86ccd0: ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE); SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | 0x80) */
    r = CALL(fle_imp_SetWindowLongPtrW, h, 0xffffffecu, 0x80 | 0x40000, 0); printf("set-exstyle prev %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xffffffecu, 0, 0); printf("exstyle %llx\n", U(r));
    r = CALL(fle_imp_SetWindowLongPtrW, h, 0xffffffebu, 0x123456789ull, 0); printf("set-userdata prev %llx\n", U(r));
    r = CALL(fle_imp_GetWindowLongPtrW, h, 0xffffffebu, 0, 0); printf("userdata %llx\n", U(r));
    printf("le %x\n", LE());
"#
        ),
    );
    assert_eq!(
        l,
        want(&[
            "exstyle 80",
            // GWL_STYLE is a LONG: the 32-bit style 0x84ca0000 sign-extended to LONG_PTR
            "style ffffffff84ca0000",
            "hinstance 400000",
            "hwndparent 0",
            "userdata 0",
            // the class procedure of the guest's TApplication window (the stub here keeps the class's lpfnWndProc: VA_DEF)
            "wndproc 424920",
            "set-exstyle prev 80",
            "exstyle 40080",
            "set-userdata prev 0",
            "userdata 123456789",
            "le aaaa",
        ])
    );
}

#[test]
fn get_window_long_ptr_for_a_window_that_is_not_in_the_table_is_zero_and_1400_and_other_indexes_stop() {
    let l = lines(
        "fr6-gwl-bad",
        r#"
    uint64_t r;
    SETLE(1);
    r = CALL(fle_imp_GetWindowLongPtrW, 0, 0xffffffecu, 0, 0); printf("null %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GetWindowLongPtrW, 0x7F990004, 0xfffffff0u, 0, 0); printf("never %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(l, want(&["null 0 le 1400", "never 0 le 1400"]));
    // decision 0123 (old: ("id", -12) was in this list: GWLP_ID is answered now, the id of a top-level window is 0): the other indexes still stop
    let l = lines("fr6-gwl-id", &format!("{APP}\n    printf(\"id %llu\\n\", U(CALL(fle_imp_GetWindowLongPtrW, h, 0xfffffff4u, 0, 0)));"));
    assert_eq!(l, want(&["id 0"]));
    let cases: Vec<(String, String)> = [("extra-0", 0), ("extra-8", 8), ("dlg-user", 16), ("minus-1", -1)]
        .iter()
        .map(|(l, i)| (l.to_string(), format!("CALL(fle_imp_GetWindowLongPtrW, h, (uint64_t)(uint32_t)(int32_t){i}, 0, 0);")))
        .collect();
    let refs: Vec<(&str, &str)> = cases.iter().map(|(l, s)| (l.as_str(), s.as_str())).collect();
    assert_aborts("fr6-gwl-stop", APP, "GetWindowLongPtrW", "is not one of GWL_EXSTYLE", &refs);
    assert_aborts("fr6-gwl-stop-extra", APP, "GetWindowLongPtrW", "extra memory and DWLP_* are not modelled", &refs[..1]);
}

// ---- wall 16: GetSystemMenu / DeleteMenu ----

#[test]
fn get_system_menu_gives_a_stable_handle_per_window_and_the_guests_three_deletions_succeed() {
    let l = lines(
        "fr6-menu",
        &format!(
            r#"
    {APP}
    uint64_t m, m2, r, h2;
    SETLE(0xBBBB);
    printf("before %d\n", fle_win_menu_items(h));
    m = CALL(fle_imp_GetSystemMenu, h, 0, 0, 0);
    m2 = CALL(fle_imp_GetSystemMenu, h, 0, 0, 0);
    printf("m %llx same %d items %x\n", U(m), m == m2, fle_win_menu_items(h));
    /* the guest's f_867e10: DeleteMenu(SC_MAXIMIZE 0xF030), (SC_SIZE 0xF000), (SC_MOVE 0xF010), all MF_BYCOMMAND (0) */
    r = CALL(fle_imp_DeleteMenu, m, 0xF030, 0, 0); printf("del max %llu items %x\n", U(r), fle_win_menu_items(h));
    r = CALL(fle_imp_DeleteMenu, m, 0xF000, 0, 0); printf("del size %llu items %x\n", U(r), fle_win_menu_items(h));
    r = CALL(fle_imp_DeleteMenu, m, 0xF010, 0, 0); printf("del move %llu items %x\n", U(r), fle_win_menu_items(h));
    printf("le %x\n", LE());
    /* bRevert TRUE: NULL, the six items are back */
    r = CALL(fle_imp_GetSystemMenu, h, 1, 0, 0); printf("revert %llu items %x\n", U(r), fle_win_menu_items(h));
    /* a second window has its own menu handle */
    h2 = CW_APP(10, 20, "second");
    r = CALL(fle_imp_GetSystemMenu, h2, 0, 0, 0); printf("second %llx differs %d\n", U(r), r != m);
"#
        ),
    );
    assert_eq!(
        l,
        want(&[
            "before -1",
            "m 7f980004 same 1 items 3f",
            "del max 1 items 37",
            "del size 1 items 36",
            "del move 1 items 34",
            "le bbbb",
            "revert 0 items 3f",
            "second 7f980008 differs 1",
        ])
    );
}

#[test]
fn get_system_menu_and_delete_menu_stop_for_every_form_the_pages_leave_open() {
    // GetSystemMenu: a window that is not in the table, or without WS_SYSMENU; DeleteMenu: another handle, MF_BYPOSITION, an item that is not (or no longer) there
    assert_aborts(
        "fr6-menu-stop",
        APP,
        "GetSystemMenu",
        "is not a window of this table",
        &[("null", "CALL(fle_imp_GetSystemMenu, 0, 0, 0, 0);"), ("never", "CALL(fle_imp_GetSystemMenu, 0x7F990008, 0, 0, 0);")],
    );
    assert_aborts(
        "fr6-menu-nosysmenu",
        "REGP(\"TPUtilWindow\", VA_REC); uint64_t h = CW_TOOL(\"TPUtilWindow\");",
        "GetSystemMenu",
        "no WS_SYSMENU",
        &[("tool-window", "CALL(fle_imp_GetSystemMenu, h, 0, 0, 0);")],
    );
    let setup = format!("{APP} uint64_t m = CALL(fle_imp_GetSystemMenu, h, 0, 0, 0);");
    assert_aborts(
        "fr6-delmenu-handle",
        &setup,
        "DeleteMenu",
        "is not the window menu",
        &[("null", "CALL(fle_imp_DeleteMenu, 0, 0xF030, 0, 0);"), ("window-handle", "CALL(fle_imp_DeleteMenu, h, 0xF030, 0, 0);"), ("misaligned", "CALL(fle_imp_DeleteMenu, m + 2, 0xF030, 0, 0);"), ("never", "CALL(fle_imp_DeleteMenu, m + 4, 0xF030, 0, 0);")],
    );
    assert_aborts("fr6-delmenu-flags", &setup, "DeleteMenu", "is not MF_BYCOMMAND", &[("by-position", "CALL(fle_imp_DeleteMenu, m, 0, 0x400, 0);"), ("flags-1", "CALL(fle_imp_DeleteMenu, m, 0xF030, 1, 0);")]);
    assert_aborts(
        "fr6-delmenu-item",
        &setup,
        "DeleteMenu",
        "is not in the window menu",
        &[
            ("unknown", "CALL(fle_imp_DeleteMenu, m, 0xF999, 0, 0);"),
            ("zero", "CALL(fle_imp_DeleteMenu, m, 0, 0, 0);"),
            ("twice", "CALL(fle_imp_DeleteMenu, m, 0xF030, 0, 0); CALL(fle_imp_DeleteMenu, m, 0xF030, 0, 0);"),
        ],
    );
}

// ---- wall 11: WTSRegisterSessionNotification ; wall 12: BufferedPaintInit ----

#[test]
fn wts_register_session_notification_records_the_first_flags_and_returns_true() {
    let l = lines(
        "fr6-wts",
        &format!(
            r#"
    {APP}
    uint64_t r;
    SETLE(0xCCCC);
    r = CALL(fle_imp_WTSRegisterSessionNotification, h, 0, 0, 0); printf("first %llu\n", U(r)); pw(h);
    /* Learn: "If the window handle passed in this function is already registered, the value of the dwFlags parameter is ignored" */
    r = CALL(fle_imp_WTSRegisterSessionNotification, h, 1, 0, 0); printf("second %llu\n", U(r)); pw(h);
    printf("le %x msgs %u\n", LE(), ln);
"#
        ),
    );
    assert_eq!(
        l,
        want(&[
            "first 1",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=9 big=0 small=0 user=0 wts=1/0",
            "second 1",
            "win 7f990004 style=84ca0000 ex=80 rect=960,540,0,0 live=1 text_len=9 big=0 small=0 user=0 wts=1/0",
            "le cccc msgs 0",
        ])
    );
    assert_aborts(
        "fr6-wts-stop",
        APP,
        "WTSRegisterSessionNotification",
        "",
        &[("null", "CALL(fle_imp_WTSRegisterSessionNotification, 0, 0, 0, 0);"), ("flags-2", "CALL(fle_imp_WTSRegisterSessionNotification, h, 2, 0, 0);"), ("never", "CALL(fle_imp_WTSRegisterSessionNotification, 0x7F990008, 0, 0, 0);")],
    );
}

#[test]
fn buffered_paint_init_is_s_ok_and_counts_the_calls() {
    let l = lines(
        "fr6-bpi",
        r#"
    uint64_t r;
    SETLE(0xDDDD);
    printf("count %u\n", fle_win_buffered_paint_init_count());
    r = CALL(fle_imp_BufferedPaintInit, 0, 0, 0, 0); printf("r %llu\n", U(r));
    r = CALL(fle_imp_BufferedPaintInit, 1, 2, 3, 4); printf("r %llu\n", U(r));
    printf("count %u le %x\n", fle_win_buffered_paint_init_count(), LE());
"#,
    );
    assert_eq!(l, want(&["count 0", "r 0", "r 0", "count 2 le dddd"]));
}

// ---- wall 13: a window procedure that is the thunk of the guest's MakeObjectInstance (a VirtualAlloc region the guest wrote itself) ----

#[test]
fn a_window_procedure_in_a_virtualalloc_region_with_the_makeobjectinstance_shape_is_called_with_r11_set() {
    let l = lines(
        "fr6-thunk",
        r#"
    uint64_t r, h, prev;
    REGP("TPUtilWindow", VA_REC);
    h = CW_TOOL("TPUtilWindow");
    make_block(VA_COMMON);
    ln = 0;
    /* what f_867e10 does: SetWindowLongPtrW(hwnd, GWLP_WNDPROC, slot): the value is kept as a number; the previous procedure comes back */
    prev = CALL(fle_imp_SetWindowLongPtrW, h, 0xFFFFFFFCull, SLOT0, 0);
    printf("prev %llx msgs %u\n", U(prev), ln);
    /* SendMessageW reaches the common stub of the guest through the template: r11 = slot + 5 (the return address that `call template` pushed and `pop r11` took) */
    r = CALL(fle_imp_SendMessageW, h, 0x80, 1, 0x7fba0624);
    printf("r %llx msgs %u tag %d h %llx msg %llx wp %llx lp %llx r11-ok %d\n", U(r), ln, lg[0].tag, U(lg[0].h), U(lg[0].msg), U(lg[0].wp), U(lg[0].lp), lg[0].r11 == SLOT0 + 5);
    printf("depth %u r4 %d\n", fle_callback_depth(), R[4] == E0);
    /* SetWindowTextW goes the same way (WM_SETTEXT 0xC) and its result is the procedure's result */
    r = CALL(fle_imp_SetWindowTextW, h, 0x1234, 0, 0);
    printf("settext r %llx msg %llx lp %llx\n", U(r), U(lg[1].msg), U(lg[1].lp));
"#,
    );
    assert_eq!(
        l,
        want(&[
            "prev 2000 msgs 0",
            "r 5151 msgs 1 tag 2 h 7f990004 msg 80 wp 1 lp 7fba0624 r11-ok 1",
            "depth 0 r4 1",
            "settext r 5151 msg c lp 1234",
        ])
    );
}

#[test]
fn a_class_window_procedure_that_is_the_thunk_gets_the_three_creation_messages_through_it_too() {
    // the three messages of CreateWindowExW go through the same call: the stub answers 0x5151 to every message. Decision 0123 (old: the creation stopped after WM_NCCREATE with "WM_NCCREATE returned 0x5151", msgs 1): a
    // nonzero WM_NCCREATE result continues like TRUE (S-wmcreate), so the second message goes through the thunk too, and the creation stops after it: the page says WM_NCCALCSIZE (wParam FALSE) should return zero
    let o = run_src(
        "fr6-thunk-class",
        "#include <signal.h>\n#include <unistd.h>\nstatic void hnd(int s) { (void)s; printf(\"msgs %u tag %d msg %llx r11-ok %d\\n\", ln, lg[0].tag, U(lg[0].msg), lg[0].r11 == SLOT0 + 5); fflush(stdout); _exit(7); }",
        "    signal(SIGABRT, hnd);\n    make_block(VA_COMMON);\n    REGP(\"TPUtilWindow\", SLOT0);\n    CW_TOOL(\"TPUtilWindow\");\n    printf(\"returned\\n\");",
    );
    assert_eq!(o.stdout, "msgs 2 tag 2 msg 81 r11-ok 1\n", "{}", o.stderr);
    assert!(o.stderr.contains("fle: CreateWindowExW: WM_NCCALCSIZE (wParam FALSE) returned 0x5151"), "{}", o.stderr);
}

#[test]
fn a_thunk_of_any_other_shape_stops_by_name_and_never_runs() {
    let send = "uint64_t h; REGP(\"TPUtilWindow\", VA_REC); h = CW_TOOL(\"TPUtilWindow\"); make_block(VA_COMMON);";
    let tail = "CALL(fle_imp_SetWindowLongPtrW, h, 0xFFFFFFFCull, SLOT0, 0); ln = 0; CALL(fle_imp_SendMessageW, h, 0x80, 1, 5);";
    let cases: Vec<(String, String, &str)> = vec![
        ("not-e8".into(), format!("{send} blk[0x18] = 0xe9; {tail}"), "does not start with E8"),
        ("template-differs".into(), format!("{send} blk[8 + 1] = 0x5a; {tail}"), "are not `pop r11; jmp [rip+0]`"),
        ("template-jump-differs".into(), format!("{send} blk[8 + 4] = 1; {tail}"), "are not `pop r11; jmp [rip+0]`"),
        ("page-not-executable".into(), format!("{send} blk_pages[0] = 0x04; {tail}"), "not marked for running code"),
        ("page-noaccess".into(), format!("{send} blk_pages[0] = 0x01; {tail}"), "not marked for running code"),
        ("call-leaves-region".into(), format!("{send} {{ int32_t d = 0x7fffff00; memcpy(blk + 0x19, &d, 4); }} {tail}"), "leaves its VirtualAlloc region"),
        ("call-before-region".into(), format!("{send} {{ int32_t d = -0x2000; memcpy(blk + 0x19, &d, 4); }} {tail}"), "leaves its VirtualAlloc region"),
        ("slot-at-the-end".into(), format!("{send} CALL(fle_imp_SetWindowLongPtrW, h, 0xFFFFFFFCull, A(blk) + 0xff8, 0); ln = 0; CALL(fle_imp_SendMessageW, h, 0x80, 1, 5);"), "end of a VirtualAlloc region"),
    ];
    for (label, body, word) in &cases {
        let o = run_src(&format!("fr6-thunk-{label}"), "", &format!("    {body}\n    printf(\"returned\\n\");"));
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains("fle: SendMessageW: ") && o.stderr.contains(word), "{label}: {}", o.stderr);
    }
    // the pointer after the template is not a function entry: fle_call_guest stops by name (the shape is right, the target is unknown)
    let o = run_src(
        "fr6-thunk-target",
        "",
        &format!("    {send} {{ uint64_t bad = 0x12345; memcpy(blk + 0x10, &bad, 8); }} {tail}\n    printf(\"returned\\n\");"),
    );
    assert!(!o.ok && !o.stdout.contains("returned") && o.signal == Some(6), "{}", o.stderr);
    assert!(o.stderr.contains("fle: indirect call to 0x12345 is not a known function entry"), "{}", o.stderr);
}

#[test]
fn an_address_outside_every_virtualalloc_region_goes_to_fle_call_guest_unchanged() {
    // the block is on but the procedure VA_REC (0x2000) is not in it: the procedure is an entry of the image, called as before (decision 0071); r11 is not touched
    let l = lines(
        "fr6-thunk-outside",
        r#"
    uint64_t r, h;
    REGP("TPUtilWindow", VA_REC);
    h = CW_TOOL("TPUtilWindow");
    make_block(VA_COMMON);
    ln = 0;
    R[11] = 0xABCD;
    send_result = 0x77;
    r = CALL(fle_imp_SendMessageW, h, 0x400, 0, 0);
    printf("r %llx tag %d r11 %llx\n", U(r), lg[0].tag, U(lg[0].r11));
"#,
    );
    assert_eq!(l, want(&["r 77 tag 1 r11 abcd"]));
}

// ---------------------------------------------------------------------------------------------
// 2b. The virtual modules wtsapi32.dll (N = 37) and uxtheme.dll (N = 38): the whole runtime and a synthetic PE (the driver of tests/wall_fr5.rs)
// ---------------------------------------------------------------------------------------------

const MOD_DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_load.h"
#include "fle_env.h"
#include "fle_win.h"

static uint64_t stk[32];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))

static void prep(uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[4] = A(stk);
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static uint64_t le(void) { uint64_t r = R[0], e; fle_imp_GetLastError(); e = R[0]; R[0] = r; return e; }
static uint16_t *wide(const char *s)
{
    size_t n = strlen(s), i;
    uint16_t *w = calloc(n + 1, 2);
    for (i = 0; i < n; i++) w[i] = (uint8_t)s[i];
    return w;
}

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    if (!strcmp(s, "modules")) {
        static const char *const names[] = { "wtsapi32.dll", "WTSAPI32", "WtsApi32.DLL", "uxtheme.dll", "UXTHEME", "UxTheme.DLL" };
        unsigned i;
        for (i = 0; i < 6; i++) { prep(A(wide(names[i])), 0, 0, 0); fle_imp_LoadLibraryW(); printf("%s %llx %llu\n", names[i], U(R[0]), U(le())); }
        prep(A(wide("wtsapi32")), 0, 0, 0); fle_imp_GetModuleHandleW(); printf("gmh-wts %llx %llu\n", U(R[0]), U(le()));
        prep(A(wide("uxtheme.dll")), 0, 0, 0); fle_imp_GetModuleHandleW(); printf("gmh-ux %llx %llu\n", U(R[0]), U(le()));
        printf("count %llu name36 %s name37 %s handle36 %llx handle37 %llx\n", U(fle_module_count()), fle_module_name(36), fle_module_name(37), U(fle_module_handle(36)), U(fle_module_handle(37)));
        printf("refs %u %u\n", fle_load_refcount(36), fle_load_refcount(37));
        {
            static const char a[] = "WTSRegisterSessionNotification", b[] = "BufferedPaintInit", c[] = "BeginBufferedPaint", d[] = "GetDpiForMonitor", e2[] = "EndBufferedPaint";
            prep(M(37), A(a), 0, 0); fle_imp_GetProcAddress(); printf("wts!WTSRegisterSessionNotification %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(38), A(b), 0, 0); fle_imp_GetProcAddress(); printf("uxtheme!BufferedPaintInit %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(38), A(a), 0, 0); fle_imp_GetProcAddress(); printf("uxtheme!WTSRegisterSessionNotification %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(37), A(b), 0, 0); fle_imp_GetProcAddress(); printf("wts!BufferedPaintInit %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(38), A(c), 0, 0); fle_imp_GetProcAddress(); printf("uxtheme!BeginBufferedPaint %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(38), A(e2), 0, 0); fle_imp_GetProcAddress(); printf("uxtheme!EndBufferedPaint %s %llu\n", R[0] ? "found" : "missing", U(le()));
            (void)d;
        }
    } else if (!strcmp(s, "absent")) {
        static const char *const names[] = { "imm32.dll", "msimg32", "propsys.dll", "windowscodecs" };
        unsigned i;
        for (i = 0; i < 4; i++) { prep(A(wide(names[i])), 0, 0, 0); fle_imp_LoadLibraryW(); printf("%s %llx %llu\n", names[i], U(R[0]), U(le())); }
    } else {
        printf("no scenario %s\n", s);
        return 3;
    }
    return 0;
}
"#;

fn mod_driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("wall-fr6-moddriver");
        // a PE that imports GetProcAddress and LoadLibraryW, so that the generated C carries the response table
        let img = build_imports(&GUEST_IMPORTS);
        let mut out = Vec::new();
        generate(&img, &mut out).unwrap();
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &out).unwrap();
        std::fs::write(&main_c, MOD_DRIVER).unwrap();
        let rt = rt_dir();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(&rt)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let exe = dir.join(&tag);
        let mut cmd = Command::new("clang");
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt).arg("-I").arg(manifest().join("../../harness"));
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(manifest().join("../../harness/image.c")).arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn mod_lines(args: &[&str]) -> Vec<String> {
    let r = Command::new(mod_driver()).args(args).output().expect("run failed");
    assert!(r.status.success(), "{args:?}: {} {}", String::from_utf8_lossy(&r.stderr), String::from_utf8_lossy(&r.stdout));
    String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect()
}

#[test]
fn wtsapi32_and_uxtheme_load_by_every_spelling_and_each_has_only_its_own_row() {
    let got = mod_lines(&["modules"]);
    let (hw, hu) = (format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * 37), format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * 38));
    assert_eq!(
        got,
        [
            format!("wtsapi32.dll {hw} 5"),
            format!("WTSAPI32 {hw} 5"),
            format!("WtsApi32.DLL {hw} 5"),
            format!("uxtheme.dll {hu} 5"),
            format!("UXTHEME {hu} 5"),
            format!("UxTheme.DLL {hu} 5"),
            format!("gmh-wts {hw} 5"),
            format!("gmh-ux {hu} 5"),
            format!("count 41 name36 wtsapi32.dll name37 uxtheme.dll handle36 {hw} handle37 {hu}"), // decision 0238: 40 modules (kernelbase.dll appended); decision 0122: 39 modules (dwmapi.dll appended; old: 38)
            // the process loaded each at start (count 1, FreeLibrary page) plus three LoadLibraryW calls
            "refs 4 4".to_string(),
            "wts!WTSRegisterSessionNotification found 5".to_string(),
            "uxtheme!BufferedPaintInit found 5".to_string(),
            // a row of another module's dll is not found under this one; the other names of uxtheme have no row (decision 0119)
            "uxtheme!WTSRegisterSessionNotification missing 127".to_string(),
            "wts!BufferedPaintInit missing 127".to_string(),
            "uxtheme!BeginBufferedPaint missing 127".to_string(),
            "uxtheme!EndBufferedPaint missing 127".to_string(),
        ]
    );
}

#[test]
fn the_four_names_that_remain_absent_still_fail_with_126() {
    // Decision 0122: dwmapi.dll is virtual module N = 39 (old name `the_five_names_that_remain_absent_still_fail_with_126`: dwmapi, DWMAPI.DLL, imm32.dll, msimg32, propsys.dll, windowscodecs).
    let got = mod_lines(&["absent"]);
    assert_eq!(got.len(), 4);
    for (l, n) in got.iter().zip(["imm32.dll", "msimg32", "propsys.dll", "windowscodecs"]) {
        assert_eq!(l, &format!("{n} 0 126"));
    }
    let t = std::fs::read_to_string(rt_dir().join("fle_load.c")).unwrap();
    let a = t.find("static const char *const fle_load_absent[] = {").expect("the list");
    let list = &t[a..a + t[a..].find("};").unwrap()];
    let names: Vec<&str> = list.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, s)| s).collect();
    // decision 0126 (wall 86, S-webview2-absent): old: the four names above; "webview2loader.dll" is the fifth (it fails with 126 too)
    assert_eq!(names, ["imm32.dll", "msimg32.dll", "propsys.dll", "windowscodecs.dll", "webview2loader.dll"]);
}

// ---- a second thread stops every new API ----

#[test]
fn a_second_thread_stops_every_new_api_of_the_window_table() {
    for (label, api, call) in [
        ("send", "SendMessageW", "CALL(fle_imp_SendMessageW, 0, 0x80, 1, 5)"),
        ("setclass", "SetClassLongPtrW", "CALL(fle_imp_SetClassLongPtrW, 0, 0xfffffff2u, 5, 0)"),
        ("getlong", "GetWindowLongPtrW", "CALL(fle_imp_GetWindowLongPtrW, 0, 0xffffffecu, 0, 0)"),
        ("settext", "SetWindowTextW", "CALL(fle_imp_SetWindowTextW, 0, 0, 0, 0)"),
        ("sysmenu", "GetSystemMenu", "CALL(fle_imp_GetSystemMenu, H1, 0, 0, 0)"),
        ("delmenu", "DeleteMenu", "CALL(fle_imp_DeleteMenu, 0, 0xF030, 0, 0)"),
        ("wts", "WTSRegisterSessionNotification", "CALL(fle_imp_WTSRegisterSessionNotification, H1, 0, 0, 0)"),
        ("bpi", "BufferedPaintInit", "CALL(fle_imp_BufferedPaintInit, 0, 0, 0, 0)"),
    ] {
        let o = run_src(
            &format!("fr6-thread-{label}"),
            &format!("#define H1 UINT64_C(0x7F990004)\nstatic void *th(void *x) {{ (void)x; {call}; printf(\"returned-from-thread\\n\"); return 0; }}"),
            "    pthread_t t;\n    REGP(\"TPUtilWindow\", VA_REC);\n    CW_TOOL(\"TPUtilWindow\");\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
        );
        assert!(!o.ok && !o.stdout.contains("returned"), "{label}: {}", o.stdout);
        assert_eq!(o.signal, Some(6), "{label}");
        assert!(o.stderr.contains(&format!("fle: {api}: ")) && o.stderr.contains("second thread"), "{label}: {}", o.stderr);
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Source checks
// ---------------------------------------------------------------------------------------------

fn strip_comments(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

#[test]
fn the_last_error_1400_is_set_in_one_place_and_the_window_table_reads_no_host_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_win.c")).unwrap());
    // PM's decision 1 (a choice without a source): exactly one place sets the last error, with the constant 1400 (ERROR_INVALID_WINDOW_HANDLE); every API of the table that gives 0 for a window that is not in it calls invalid_window()
    // Decision 0122 adds the second place, PostMessageW's full queue (ERROR_NOT_ENOUGH_QUOTA, Learn PostMessage); old: exactly one call.
    // Decision 0123 (old: exactly 2 calls): the window model sets the last error where a page gives a failure: 32 calls, each with a named constant or one of the System Error Codes numbers that have no constant (1406, 8)
    // Decision 0126 (old: 32): two more places, both with a named constant, found by comparing the clean tree and this tree function by function: fle_imp_LockWindowUpdate (a window that is not live: 1400, S-err) and
    // fle_imp_GetCursorInfo (a cbSize other than 24: ERROR_INVALID_PARAMETER 87, S-cursorinfo-size): 34.
    // Decision 0129 (old: 34): one more place, found by comparing the clean tree and this tree function by function: fle_imp_GetLastActivePopup (an invalid window: FLE_ERROR_INVALID_WINDOW_HANDLE_W, an argument that is already in the set below): 35.
    // Decision 0147 (old: 35): fle_imp_PostThreadMessageW sets the last error in two places, both with a named constant (ERROR_INVALID_THREAD_ID 1444 for an unknown thread or a thread without a queue, ERROR_NOT_ENOUGH_QUOTA 1816 for a full queue): 37.
    // Decision 0267 (old: 37): fle_imp_SetWindowPlacement sets ERROR_INVALID_PARAMETER (FLE_ERROR_INVALID_PARAMETER_W, a named constant already in the set) for a length other than the 44 bytes of WINDOWPLACEMENT (a window that is not live goes through win_arg, as in the other APIs): 38.
    assert_eq!(code.matches("fle_set_last_error(").count(), 38);
    assert!(code.contains("fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE)"));
    assert!(code.contains("fle_set_last_error(FLE_ERROR_NOT_ENOUGH_QUOTA_W)"));
    let h = std::fs::read_to_string(src_dir().join("fle_win.h")).unwrap();
    assert!(h.contains("#define FLE_ERROR_INVALID_WINDOW_HANDLE 1400u"));
    // decision 0122: PostMessageW and GetMessageW (an hWnd that is not a window of the thread) call it too: 7 (old: 5)
    assert_eq!(code.matches("invalid_window();").count(), 7, "SendMessageW, SetClassLongPtrW, GetWindowLongPtrW, SetWindowTextW, SetWindowLongPtrW, PostMessageW and GetMessageW");
    for bad in [
        "getpid", "getenv", "environ", "fopen", "open(", "/proc", "/etc", "/home", "/dev", "/tmp", "clock_gettime", "gettimeofday", "time(", "nanosleep", "usleep", "sleep(", "select(", "<time.h>", "<sys/time.h>",
        "pthread", "<sys/", "<unistd.h>", "<signal.h>", "fork(", "syscall", "futex", "__asm__", "abort(", "X11", "DISPLAY", "wayland", "ioctl", "dlopen", "LoadLibrary", "setjmp", "longjmp", "(void (*)",
    ] {
        assert!(!code.contains(bad), "fle_win.c must not use {bad}");
    }
    // Decision 0122 puts the message queue API (PostMessageW, GetMessageW, PeekMessageW, WaitMessage, PostQuitMessage, TranslateMessage, DispatchMessageW) into fle_win.c; they are
    // tested by tests/wall_fr8.rs. MsgWaitForMultipleObjects stays in fle_thread.c (it waits for objects too). Old: none of the names was allowed in fle_win.c.
    assert!(!code.contains("MsgWaitForMultipleObjects"), "MsgWaitForMultipleObjects belongs to fle_thread.c");
    // fle_sys_region_at is a weak reference: a link without fle_sys.c sees no region and hands every procedure to fle_call_guest
    assert!(code.contains("fle_sys_region_at(uint64_t addr, fle_sys_region_view *out) __attribute__((weak))"));
}

#[test]
fn fle_class_set_hicon_changes_one_value_of_one_class_and_uses_no_host_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_class.c")).unwrap());
    assert!(code.contains("void fle_class_set_hicon(const char *api, uint16_t atom, uint64_t value, uint64_t *prev)"));
    let h = std::fs::read_to_string(src_dir().join("fle_class.h")).unwrap();
    assert!(h.contains("#define FLE_CLASS_WC_OFF_HICON 32"));
}

// ---------------------------------------------------------------------------------------------
// 4. The bytes of the real FLEngine_x64.dll that decision 0119 rests on (skipped when the DLL is absent)
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}
fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn sections(dll: &[u8]) -> Vec<(u32, u32, u32, u32)> {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    (0..nsec).map(|i| lf + 24 + osz + 40 * i).map(|o| (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20))).collect()
}
fn hex_at(dll: &[u8], va: u64, n: usize) -> String {
    let rva = (va - 0x400000) as u32;
    for (vs, sva, rs, rp) in sections(dll) {
        if rva >= sva && rva < sva + vs.max(rs) {
            let o = rp as usize + (rva - sva) as usize;
            return dll[o..o + n].iter().map(|x| format!("{x:02x}")).collect();
        }
    }
    panic!("va {va:#x} is in no section");
}
fn call_sites(dll: &[u8], thunk: u64) -> Vec<u64> {
    let (vs, sva, _, rp) = sections(dll)[0];
    let text = &dll[rp as usize..rp as usize + vs as usize];
    let mut found = Vec::new();
    for i in 0..text.len().saturating_sub(5) {
        if text[i] == 0xe8 {
            let rel = i32::from_le_bytes(text[i + 1..i + 5].try_into().unwrap()) as i64;
            let va = 0x400000 + sva as u64 + i as u64;
            if (va as i64 + 5 + rel) as u64 == thunk {
                found.push(va);
            }
        }
    }
    found
}

#[test]
fn the_guests_calls_of_walls_9_to_17_have_the_arguments_the_decision_lists() {
    let Some(dll) = real_dll() else { return };
    let h = |va: u64, n: usize| hex_at(&dll, va, n);
    // wall 9 (f_86b680 at 0x86b6a4..0x86b6b2): mov edx,0x80 (WM_SETICON); mov r8d,1 (ICON_BIG); call SendMessageW (thunk 0x425390); then mov edx,0xfffffff2 (GCLP_HICON = -14); call SetClassLongPtrW (thunk 0x4259a0)
    assert_eq!(h(0x86b6a4, 5), "ba80000000");
    assert_eq!(h(0x86b6a9, 6), "41b801000000");
    assert_eq!(h(0x86b6c9, 5), "baf2ffffff");
    assert!(call_sites(&dll, 0x425390).contains(&0x86b6b2) && call_sites(&dll, 0x4259a0).contains(&0x86b6d1));
    // wall 10 (f_867e10): mov ecx,0x80; mov r9d,0x84ca0000; x, y = half of GetSystemMetrics(0), (1); call CreateWindowExW (thunk 0x425780) at 0x867f7f
    assert_eq!(h(0x867f0d, 5), "b980000000");
    assert_eq!(h(0x867f1c, 6), "41b90000ca84");
    // x = GetSystemMetrics(SM_CXSCREEN) / 2 as the guest computes it: mov r10d,esi; shr r10d,0x1f; add r10d,esi (0x867f22)
    assert_eq!(h(0x867f22, 10), "448bd641c1ea1f4403d6");
    assert!(call_sites(&dll, 0x425780).contains(&0x867f7f));
    // wall 11 (0x867fbf): call the delay-load stub 0x425750 (a `jmp [slot]`) with edx = 0 (xor edx,edx at 0x867fbd); the stub's slot is the delay-load import of wtsapi32!WTSRegisterSessionNotification
    assert_eq!(h(0x867fbd, 2), "33d2");
    assert_eq!(h(0x425750, 2), "ff25");
    assert!(call_sites(&dll, 0x425750).contains(&0x867fbf));
    // wall 12 (0x867fe8): call the delay-load stub 0x5dc640 (uxtheme!BufferedPaintInit); the only call site of it
    assert_eq!(call_sites(&dll, 0x5dc640), vec![0x867fe8]);
    assert_eq!(h(0x5dc640, 2), "ff25");
    // the two DLL names of the delay-load descriptors
    assert_eq!(h(0x1976d6a, 13), "77747361706933322e646c6c00");
    assert_eq!(h(0x1977166, 12), "75787468656d652e646c6c00");
    // wall 13: the common stub of MakeObjectInstance and the template that the guest copies into its block
    assert_eq!(h(0x13be804, 8), "415bff2500000000");
    assert_eq!(h(0x52a570, 8), "554883ec40488bec");
    assert_eq!(h(0x52a593, 3), "41ff13");
    assert_eq!(h(0x52a58f, 4), "498b4b08");
    // wall 15..16: GetSystemMenu(hwnd, 0) at 0x868077 and the three DeleteMenu ids SC_MAXIMIZE 0xf030, SC_SIZE 0xf000, SC_MOVE 0xf010 (all with uFlags 0: xor r8,r8)
    assert!(call_sites(&dll, 0x424ed0).contains(&0x868077));
    assert_eq!(h(0x868082, 5), "ba30f00000");
    assert_eq!(h(0x868092, 5), "ba00f00000");
    assert_eq!(h(0x8680ae, 5), "ba10f00000");
    assert_eq!(h(0x868087, 3), "4d33c0"); // xor r8,r8: uFlags = MF_BYCOMMAND (0)
    assert_eq!(h(0x8680b3, 3), "4d33c0");
    // wall 17 (f_86ccd0): GetWindowLongPtrW(hwnd, -20); SetWindowLongPtrW(hwnd, -20, ex | 0x80); SetWindowTextW(hwnd, NULL)
    assert_eq!(h(0x86ce10, 5), "baecffffff");
    assert_eq!(h(0x86ce1a, 6), "480d80000000"); // or rax,0x80: WS_EX_TOOLWINDOW
    assert!(call_sites(&dll, 0x425970).contains(&0x86ce15) && call_sites(&dll, 0x425530).contains(&0x86ce39));
    // wall 14: TApplication.WndProc (f_868b40) forwards the message to DefWindowProcW (thunk 0x424920) at 0x868b79
    assert!(call_sites(&dll, 0x424920).contains(&0x868b79));
}

#[test]
fn set_window_long_ptr_for_a_window_that_is_not_in_the_table_is_zero_and_1400() {
    // PM's decision 1 covers SetWindowLongPtrW as well (a CHOICE WITHOUT A SOURCE): the guest's own calls always come with a window of the table, so this is not reached by CreateFruityInstance
    let l = lines(
        "fr6-swl-bad",
        r#"
    uint64_t r;
    SETLE(1);
    r = CALL(fle_imp_SetWindowLongPtrW, 0, 0xfffffffcu, 0x1234, 0); printf("null %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_SetWindowLongPtrW, 0x7F990004, 0xffffffecu, 0x80, 0); printf("never %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(l, want(&["null 0 le 1400", "never 0 le 1400"]));
}
