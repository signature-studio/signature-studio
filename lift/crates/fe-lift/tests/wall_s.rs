//! Tests of decision 0099 (docs/decisions/0099-wall-loader-batch2.md, census wall batch 2): the answers that keep four walls of the census from
//! stopping the run. Expected values are written from decision 0099, decision 0015 (module handle = 0x7FF800000000 + 0x10000 * N), the Learn
//! pages named in the decision (GetProcessDpiAwareness, GetSystemDefaultLocaleName, RegOpenKeyExW, GetProcAddress, LoadLibraryW, the delay-load
//! "Error handling and notification" page) and Wine's shell32.spec for the nine ordinals (UNOFFICIAL), not read back from the C code. The host
//! is Linux: nothing is compared with a real Windows.
//!
//! 1. Generator: the response table has the new rows, under the right DLL names, after every older row (decision 0015: append only).
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE (the pseudo-VAs of GetProcAddress are
//!    called through fle_dispatch, as the guest does): shcore.dll as module N = 35, shcore!GetProcessDpiAwareness,
//!    kernel32!GetSystemDefaultLocaleName, GetProcAddress by ordinal on shell32.dll, the nine shell32 stops.
//! 3. The registry walk of RegOpenKeyEx (empty key-path components), through the registry test helper.
//! 4. The delay-load descriptors of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent).

mod etxt_common;
mod fsx_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallS-test/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/msctf.rs)
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

/// The rows of decision 0099 in the order of the table (the order fixes the pseudo-VAs): (dll, name).
const NEW_ROWS: [(&str, &str); 11] = [
    ("kernel32.dll", "GetSystemDefaultLocaleName"),
    ("shcore.dll", "GetProcessDpiAwareness"),
    ("shell32.dll", "ILFindLastID"),
    ("shell32.dll", "ILRemoveLastID"),
    ("shell32.dll", "ILClone"),
    ("shell32.dll", "ILCloneFirst"),
    ("shell32.dll", "ILIsEqual"),
    ("shell32.dll", "ILCombine"),
    ("shell32.dll", "ILGetSize"),
    ("shell32.dll", "ILGetNext"),
    ("shell32.dll", "ILFree"),
];

/// The nine export ordinals the guest passes in unit 238 and the names they stand for (Wine's shell32.spec; decision 0099).
const ORDINALS: [(u32, &str); 9] = [
    (16, "ILFindLastID"),
    (17, "ILRemoveLastID"),
    (18, "ILClone"),
    (19, "ILCloneFirst"),
    (21, "ILIsEqual"),
    (25, "ILCombine"),
    (152, "ILGetSize"),
    (153, "ILGetNext"),
    (155, "ILFree"),
];

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

/// (dll, name, VA) of every row of fle_resp_table in the generated C, in table order.
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

#[test]
fn the_new_rows_follow_the_rows_before_them_in_the_response_table_in_order() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let n = NEW_ROWS.len();
    // The rows of later decisions are appended after these (decision 0015), so the block is found by its first row, not at the end of the table.
    let at = rows.iter().position(|(d, nm, _)| (d.as_str(), nm.as_str()) == NEW_ROWS[0]).expect("the first new row");
    let block: Vec<(&str, &str)> = rows[at..at + n].iter().map(|(d, nm, _)| (d.as_str(), nm.as_str())).collect();
    assert_eq!(block, NEW_ROWS);
    // decision 0015: the older rows do not move. The row before the new ones is the last row of decision 0098 (the first batch of wall responses,
    // appended after the deferred modules of decision 0096, which follow decision 0092), and VAs rise by 0x10 per row.
    assert_eq!((rows[at - 1].0.as_str(), rows[at - 1].1.as_str()), ("version.dll", "GetFileVersionInfoSizeW"));
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    for (_, name) in NEW_ROWS {
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name}");
    }
}

#[test]
fn a_thunk_of_a_new_row_is_handled_only_under_its_own_dll() {
    // The same API name under another DLL is another API (decision 0006 rules): shell32!ILClone has a row, user32!ILClone has not.
    let (st, c) = gen(&[("kernel32.dll", "GetProcAddress"), ("shell32.dll", "ILClone"), ("user32.dll", "ILClone"), ("kernel32.dll", "GetProcessDpiAwareness")]);
    assert_eq!(st.handled["jmp_imp:ILClone"], 1);
    assert!(!st.handled.contains_key("jmp_imp:GetProcessDpiAwareness"), "shcore's row is not kernel32's");
    assert!(c.contains("fle_imp_ILClone(); return;"));
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

const DRIVER: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_load.h"
#include "fle_locale.h"
#include "fle_proc.h"

static uint64_t stk[16];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define N_KERNEL32 9
#define N_SHELL32 16
#define N_SHCORE 35

static void prep(uint64_t a, uint64_t b)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[4] = A(stk);
    R[1] = a; R[2] = b;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static uint64_t le(void) { uint64_t r = R[0], e; fle_imp_GetLastError(); e = R[0]; R[0] = r; return e; }
static void llw(const uint16_t *n) { prep(A(n), 0); fle_imp_LoadLibraryW(); printf("%llx %llu\n", U(R[0]), U(le())); }
static void lla(const char *n) { prep(A(n), 0); fle_imp_LoadLibraryA(); printf("%llx %llu\n", U(R[0]), U(le())); }
static void gmh(const uint16_t *n) { prep(A(n), 0); fle_imp_GetModuleHandleW(); printf("%llx %llu\n", U(R[0]), U(le())); }
static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n)); fle_imp_GetProcAddress();
    r = R[0];
    printf("%s %llu\n", r ? "found" : "missing", U(le()));
    return r;
}
static uint64_t gpa_ord(uint64_t m, uint64_t ord)
{
    uint64_t r;
    prep(m, ord); fle_imp_GetProcAddress();
    r = R[0];
    printf("%s %llu\n", r ? "found" : "missing", U(le()));
    return r;
}
static uint64_t callva(uint64_t va, uint64_t a, uint64_t b)
{
    prep(a, b);
    fle_dispatch(va);
    return R[0];
}
#define LLW(s) llw((const uint16_t *)u##s)
#define GMH(s) gmh((const uint16_t *)u##s)

static const char *const il_names[9] = { "ILFindLastID", "ILRemoveLastID", "ILClone", "ILCloneFirst", "ILIsEqual", "ILCombine", "ILGetSize", "ILGetNext", "ILFree" };
static const unsigned il_ords[9] = { 16, 17, 18, 19, 21, 25, 152, 153, 155 };

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    if (!strcmp(s, "shcore-module")) {
        lla("Shcore.dll");                       /* what the delay-load helper calls (LoadLibraryA) */
        LLW("shcore.dll"); LLW("SHCORE"); LLW("ShCore.DLL");
        GMH("shcore.dll"); GMH("Shcore");
        printf("refs %u\n", fle_load_refcount(34));
        printf("count %llu name34 %s\n", U(fle_module_count()), fle_module_name(34));
    } else if (!strcmp(s, "shcore-lookup")) {
        uint64_t m = M(N_SHCORE);
        gpa(m, "GetProcessDpiAwareness");
        gpa(m, "GetDpiForMonitor");               /* decision 0144 (wall 4) answers it: a row under shcore.dll (runtime/fle_appfile.c) */
        gpa(m, "GetScaleFactorForMonitor");       /* no row: not answered, so NULL and 127 */
        gpa(m, "getprocessdpiawareness");         /* names are compared with exact case */
        gpa(M(N_KERNEL32), "GetProcessDpiAwareness");   /* the row is under shcore.dll only */
        gpa(M(N_SHELL32), "GetProcessDpiAwareness");
        gpa(m, "SetInputScope");
    } else if (!strcmp(s, "dpi")) {
        uint64_t va = gpa(M(N_SHCORE), "GetProcessDpiAwareness"), v;
        v = 0xAAAAAAAAAAAAAAAAull; callva(va, 0xFFFFFFFFFFFFFFFFull, A(&v));
        printf("cur %llx v=%llx le=%llu\n", U(R[0]), U(v), U(le()));
        v = 0xAAAAAAAAAAAAAAAAull; callva(va, 0, A(&v));
        printf("null %llx v=%llx\n", U(R[0]), U(v));
        v = 0xAAAAAAAAAAAAAAAAull; callva(va, 0x1234, A(&v));
        printf("other %llx v=%llx\n", U(R[0]), U(v));
        v = 0xAAAAAAAAAAAAAAAAull; callva(va, 0xFFFFFFFFFFFFFFFEull, A(&v));
        printf("thread-pseudo %llx v=%llx\n", U(R[0]), U(v));
        callva(va, 0xFFFFFFFFFFFFFFFFull, 0);
        printf("null-ptr %llx\n", U(R[0]));
        callva(va, 0, 0);
        printf("both-null %llx\n", U(R[0]));
    } else if (!strcmp(s, "locale-name")) {
        uint64_t va = gpa(M(N_KERNEL32), "GetSystemDefaultLocaleName");
        static uint16_t b[100];
        static const char *const names[] = { "en-US" };
        unsigned i;
        for (i = 0; i < 100; i++) b[i] = 0xEEEE;
        callva(va, A(b), 85);
        printf("n85 %llu le=%llu [%c%c%c%c%c] nul=%x next=%x\n", U(R[0]), U(le()), b[0], b[1], b[2], b[3], b[4], b[5], b[6]);
        for (i = 0; i < 100; i++) b[i] = 0xEEEE;
        callva(va, A(b), 6);
        printf("n6 %llu nul=%x next=%x\n", U(R[0]), b[5], b[6]);
        for (i = 0; i < 100; i++) b[i] = 0xEEEE;
        callva(va, A(b), 5);
        printf("n5 %llu le=%llu untouched=%d\n", U(R[0]), U(le()), b[0] == 0xEEEE && b[4] == 0xEEEE);
        callva(va, A(b), 0);
        printf("n0 %llu le=%llu untouched=%d\n", U(R[0]), U(le()), b[0] == 0xEEEE);
        callva(va, 0, 85);
        printf("null %llu le=%llu\n", U(R[0]), U(le()));
        callva(va, 0, 0);
        printf("null0 %llu le=%llu\n", U(R[0]), U(le()));
        for (i = 0; i < 100; i++) b[i] = 0xEEEE;
        callva(va, A(b), 0xFFFFFFFF00000055ull);     /* only the low dword is the int argument */
        printf("hi %llu [%c%c%c%c%c]\n", U(R[0]), b[0], b[1], b[2], b[3], b[4]);
        /* the same name as LOCALE_SNAME of GetLocaleInfoW for the emulated locale (one source) */
        for (i = 0; i < 100; i++) b[i] = 0xEEEE;
        prep(0x409, 0x5C); R[8] = A(b); R[9] = 85; fle_imp_GetLocaleInfoW();
        printf("snme %llu [%c%c%c%c%c]\n", U(R[0]), b[0], b[1], b[2], b[3], b[4]);
        (void)names;
    } else if (!strcmp(s, "locale-name-negative")) {
        uint64_t va = gpa(M(N_KERNEL32), "GetSystemDefaultLocaleName");
        static uint16_t b[10];
        callva(va, A(b), 0xFFFFFFFFull);
    } else if (!strcmp(s, "ordinals")) {
        unsigned i;
        for (i = 0; i < 9; i++) {
            uint64_t by_ord, by_name;
            printf("ord %u\n", il_ords[i]);
            by_ord = gpa_ord(M(N_SHELL32), il_ords[i]);
            by_name = gpa(M(N_SHELL32), il_names[i]);
            printf("same %d pseudo %d\n", by_ord == by_name, by_ord >= 0x7FF900000000ull);
        }
    } else if (!strcmp(s, "ordinal-unlisted")) { gpa_ord(M(N_SHELL32), 20);
    } else if (!strcmp(s, "ordinal-unlisted-hi")) { gpa_ord(M(N_SHELL32), 0xFFFF);
    } else if (!strcmp(s, "ordinal-null")) { gpa_ord(M(N_SHELL32), 0);
    } else if (!strcmp(s, "ordinal-kernel32")) { gpa_ord(M(N_KERNEL32), 18);
    } else if (!strcmp(s, "ordinal-shcore")) { gpa_ord(M(N_SHCORE), 16);
    } else if (!strcmp(s, "ordinal-exe")) { gpa_ord(0x400000, 18);
    } else if (!strcmp(s, "ordinal-garbage-handle")) { gpa_ord(0x12345678, 18);
    } else if (!strncmp(s, "il-stop-", 8)) {
        unsigned i = (unsigned)atoi(s + 8);
        uint64_t va = gpa_ord(M(N_SHELL32), il_ords[i]);
        printf("calling\n"); fflush(stdout);
        callva(va, 0, 0);
        printf("returned\n");
    } else {
        printf("no scenario %s\n", s);
        return 3;
    }
    return 0;
}
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let dir = work_dir();
        let tag = etxt_common::per_process("wall-s-driver");
        let (_, c) = gen(&GUEST_IMPORTS);
        let gen_c = dir.join(format!("{tag}-gen.c"));
        let main_c = dir.join(format!("{tag}-main.c"));
        std::fs::write(&gen_c, &c).unwrap();
        std::fs::write(&main_c, DRIVER).unwrap();
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
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        exe
    })
}

fn run(scenario: &str) -> Out {
    let r = Command::new(driver()).arg(scenario).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(scenario: &str) -> Vec<String> {
    let o = run(scenario);
    assert!(o.ok, "{scenario}: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn aborts(scenario: &str, must_say: &str) -> Out {
    let o = run(scenario);
    assert!(!o.ok, "{scenario}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{scenario}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle:"), "{scenario}: no fle: message: {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{scenario}: message lacks {must_say:?}: {}", o.stderr);
    o
}

/// The handle of module number N (decision 0015: 1-based position, 0x7FF800000000 + 0x10000 * N).
fn h(n: u64) -> String {
    format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * n)
}
const N_SHCORE: u64 = 35;

#[test]
fn shcore_is_the_35th_module_and_loads_by_every_spelling() {
    // Decision 0099 (after decisions 0015, 0088): shcore.dll is appended after the deferred modules of decision 0096 (N = 35). LoadLibraryA (the delay-load helper's call), LoadLibraryW
    // and GetModuleHandleW give its handle; names are compared without case and ".dll" is appended (the pages); success leaves the last error (5).
    let got = lines("shcore-module");
    let s = h(N_SHCORE);
    assert_eq!(
        got,
        [
            format!("{s} 5"),
            format!("{s} 5"),
            format!("{s} 5"),
            format!("{s} 5"),
            format!("{s} 5"),
            format!("{s} 5"),
            // the process loaded it at start: count 1 (FreeLibrary page), plus the four LoadLibrary calls
            "refs 5".to_string(),
            "count 41 name34 shcore.dll".to_string(), // decision 0238: kernelbase.dll appended (decision 0122: 39, dwmapi.dll appended) (decision 0119: 38, wtsapi32.dll and uxtheme.dll appended; old: count 36)
        ]
    );
}

#[test]
fn shcore_has_only_the_two_dpi_rows() {
    let got = lines("shcore-lookup");
    // found, found (GetDpiForMonitor: decision 0144), then: GetScaleFactorForMonitor (no row), wrong case, kernel32's handle, shell32's handle, an unrelated name: each NULL and 127.
    assert_eq!(got, ["found 5", "found 5", "missing 127", "missing 127", "missing 127", "missing 127", "missing 127"]);
}

#[test]
fn get_process_dpi_awareness_answers_the_current_process_as_dpi_unaware() {
    // Learn (GetProcessDpiAwareness): hprocess NULL = the current process; S_OK (0) and E_INVALIDARG (0x80070057) for a handle or pointer that is
    // not valid. Decision 0099: the current process (NULL or the GetCurrentProcess pseudo handle -1) is PROCESS_DPI_UNAWARE (0); another handle
    // (this environment cannot make one) and a NULL pointer are E_INVALIDARG. Only 4 bytes are written (the enum is an int); the last error is not touched.
    let got = lines("dpi");
    assert_eq!(
        got,
        [
            "found 5",
            "cur 0 v=aaaaaaaa00000000 le=5",
            "null 0 v=aaaaaaaa00000000",
            "other 80070057 v=aaaaaaaaaaaaaaaa",
            "thread-pseudo 80070057 v=aaaaaaaaaaaaaaaa",
            "null-ptr 80070057",
            "both-null 80070057",
        ]
    );
}

#[test]
fn get_system_default_locale_name_gives_en_us_and_fails_on_a_small_buffer() {
    // Learn (GetSystemDefaultLocaleName): success returns the length including the NUL; failure 0 with ERROR_INSUFFICIENT_BUFFER (122) when the
    // buffer is too small or NULL. Decision 0016: the system default locale is en-US, whose name has 5 characters, 6 with the NUL.
    let got = lines("locale-name");
    assert_eq!(
        got,
        [
            "found 5",
            "n85 6 le=5 [en-US] nul=0 next=eeee",
            "n6 6 nul=0 next=eeee",
            "n5 0 le=122 untouched=1",
            "n0 0 le=122 untouched=1",
            "null 0 le=122",
            "null0 0 le=122",
            "hi 6 [en-US]",
            "snme 6 [en-US]",
        ]
    );
}

#[test]
fn a_negative_locale_name_size_stops() {
    aborts("locale-name-negative", "GetSystemDefaultLocaleName: a negative cchLocaleName is not defined by the page");
}

#[test]
fn the_nine_shell32_ordinals_resolve_to_the_same_pseudo_va_as_the_names() {
    let got = lines("ordinals");
    let mut want: Vec<String> = Vec::new();
    for (ord, _) in ORDINALS {
        want.push(format!("ord {ord}"));
        want.push("found 5".into()); // by ordinal: the last error is left alone
        want.push("found 5".into()); // by name
        want.push("same 1 pseudo 1".into());
    }
    assert_eq!(got, want);
}

#[test]
fn an_ordinal_that_is_not_in_the_table_stops_with_the_module_and_the_ordinal() {
    // 20 is ILGlobalClone in Wine's list: not one of the nine, so nothing is said about it. Neither are another module's ordinals.
    let o = aborts("ordinal-unlisted", "GetProcAddress by ordinal (lpProcName 0x14) is not supported: shell32.dll has no export-ordinal row for it");
    assert!(!o.stdout.contains("found"), "{}", o.stdout);
    aborts("ordinal-unlisted-hi", "GetProcAddress by ordinal (lpProcName 0xffff) is not supported: shell32.dll has no export-ordinal row");
    aborts("ordinal-kernel32", "GetProcAddress by ordinal (lpProcName 0x12) is not supported: kernel32.dll has no export-ordinal row");
    aborts("ordinal-shcore", "GetProcAddress by ordinal (lpProcName 0x10) is not supported: shcore.dll has no export-ordinal row");
}

#[test]
fn the_other_ordinal_stops_keep_their_old_message() {
    // Ordinal 0 (NULL), the lifted DLL's own handle and a value that is not a module handle: the message of before decision 0099, unchanged
    // (tests/realmod.rs asserts the same text for a real module).
    for s in ["ordinal-null", "ordinal-exe", "ordinal-garbage-handle"] {
        let o = run(s);
        assert_eq!(o.signal, Some(6), "{s}: {}", o.stderr);
        let line = o.stderr.lines().find(|l| l.starts_with("fle: GetProcAddress by ordinal")).unwrap_or_else(|| panic!("{s}: {}", o.stderr));
        assert!(line.ends_with("is not supported"), "{s}: {line}");
    }
}

#[test]
fn each_shell32_pidl_function_is_found_and_stops_by_name_when_called() {
    for (i, (_, name)) in ORDINALS.iter().enumerate() {
        // decision 0126 (wall 87, S-il-path-form): old: all nine PIDL functions stopped by name; ILFree (ordinal 155) frees an ITEMIDLIST of fle_ole.c now (the Learn page: "pidl can be NULL"; the same as CoTaskMemFree), so called
        // with NULL it returns (the scenario prints "returned"); the other eight still stop. tests/wall_fr11.rs pins ILFree of a live list and of a stray pointer.
        if *name == "ILFree" {
            let o = run(&format!("il-stop-{i}"));
            assert!(o.ok && o.stdout.contains("calling") && o.stdout.contains("returned"), "{name}: {} {}", o.stdout, o.stderr);
            continue;
        }
        let o = aborts(&format!("il-stop-{i}"), &format!("fle: shell32!{name}: not answered: a shell PIDL (ITEMIDLIST) function"));
        assert!(o.stdout.contains("calling"), "{name}: {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{name}: {}", o.stdout);
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Registry: the key path is walked in order; an empty component stops only when it is reached
// ---------------------------------------------------------------------------------------------

use fsx_common::{assert_aborts, lines as reg_lines};

const HKCU: &str = "0xFFFFFFFF80000001ull";

#[test]
fn a_trailing_backslash_after_a_missing_key_is_file_not_found_like_the_guest_call() {
    // unit 830: RegOpenKeyExW(HKCU, L"Software\\Image-Line\\Registrations\\", 0, KEY_ALL_ACCESS) with Software\Image-Line created and
    // Registrations missing. Learn: a missing key is ERROR_FILE_NOT_FOUND (2). Decision 0099: the walk goes component by component and ends at the
    // first missing key, before the empty last component is reached. The output handle is not written.
    let l = reg_lines(
        "reg-wall-s-guest",
        &format!(
            r#"
    uint64_t k = 7, k2 = 8;
    printf("create %llu\n", U(CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("Software\\Image-Line\\Shared"), 0, 0, 0, 0xF003F, 0, A(&k2), 0)));
    printf("guest %llu k=%llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Image-Line\\Registrations\\"), 0, 0xF003F, A(&k))), U(k));
    printf("guestA %llu k=%llu\n", U(CALL(fle_imp_RegOpenKeyExA, {HKCU}, A("Software\\Image-Line\\Registrations\\"), 0, 0xF003F, A(&k))), U(k));
    printf("missing-first %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Nope\\"), 0, 0x20019, A(&k))));
    printf("missing-doubled %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Nope\\\\Deeper"), 0, 0x20019, A(&k))));
    printf("missing-middle-trailing %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Nope\\"), 0, 0x20019, A(&k))));
    printf("missing-then-leading-empty-pair %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Image-Line\\Nope\\\\"), 0, 0x20019, A(&k))));
    printf("plain-missing %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Image-Line\\Registrations"), 0, 0xF003F, A(&k))));
    printf("existing %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Image-Line\\Shared"), 0, 0xF003F, A(&k))));
    printf("empty %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(""), 0, 0xF003F, A(&k))));
"#
        ),
    );
    assert_eq!(
        l,
        [
            "create 0",
            "guest 2 k=7",
            "guestA 2 k=7",
            "missing-first 2",
            "missing-doubled 2",
            "missing-middle-trailing 2",
            "missing-then-leading-empty-pair 2",
            "plain-missing 2",
            "existing 0",
            "empty 0",
        ]
    );
}

#[test]
fn an_empty_component_that_the_walk_reaches_still_stops_by_name() {
    // Leading, doubled and trailing backslashes stop when every key before the empty component exists (and at once for a leading one: the
    // base key is the key before it). The page does not say what Windows does there (decision 0099).
    let setup = format!(
        r#"
    uint64_t k = 0;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("A\\B"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
"#
    );
    let cases: Vec<(&str, String)> = vec![
        ("leading-missing-key", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"\\\\Nope\"), 0, 0x20019, A(&k));")),
        ("leading-existing-key", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"\\\\A\"), 0, 0x20019, A(&k));")),
        ("only-a-backslash", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"\\\\\"), 0, 0x20019, A(&k));")),
        ("trailing-existing", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"A\\\\\"), 0, 0x20019, A(&k));")),
        ("trailing-existing-two-levels", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"A\\\\B\\\\\"), 0, 0x20019, A(&k));")),
        ("doubled-existing", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"A\\\\\\\\B\"), 0, 0x20019, A(&k));")),
        ("trailing-existing-A", format!("CALL(fle_imp_RegOpenKeyExA, {HKCU}, A(\"A\\\\B\\\\\"), 0, 0x20019, A(&k));")),
        // RegDeleteKeyW is not changed: an empty component stops when the path is split. decision 0126 (wall 73, S-regcreate-trailing-backslash): old: ("create-trailing-missing", RegCreateKeyExW(HKCU, "Nope\\")) stopped too;
        // a path that ends with ONE backslash is created without it now (tests/wall_fr11.rs)
        ("delete-trailing-missing", format!("CALL(fle_imp_RegDeleteKeyW, {HKCU}, W(\"Nope\\\\\"));")),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (*a, b.as_str())).collect();
    assert_aborts("reg-wall-s-empty", &setup, &cases);
}

// ---------------------------------------------------------------------------------------------
// 4. The delay-load descriptors of the real DLL
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// File offset of an RVA (the section table is read here, not through fe-pe).
fn rva_off(d: &[u8], rva: u32) -> usize {
    let pe = le32(d, 0x3c) as usize;
    let nsec = u16::from_le_bytes(d[pe + 6..pe + 8].try_into().unwrap()) as usize;
    let opt = u16::from_le_bytes(d[pe + 20..pe + 22].try_into().unwrap()) as usize;
    let st = pe + 24 + opt;
    for i in 0..nsec {
        let s = st + 40 * i;
        let (vsz, va, rsz, rptr) = (le32(d, s + 8), le32(d, s + 12), le32(d, s + 16), le32(d, s + 20));
        if rva >= va && rva < va + vsz.max(rsz) {
            return (rptr + rva - va) as usize;
        }
    }
    panic!("rva {rva:#x} is in no section");
}

fn cstr(d: &[u8], off: usize) -> String {
    let e = d[off..].iter().position(|&b| b == 0).unwrap();
    String::from_utf8_lossy(&d[off..off + e]).to_string()
}

/// (dll, function, IAT slot VA) of every delay import of the DLL, in table order.
fn delay_imports(d: &[u8]) -> Vec<(String, String, u64)> {
    let pe = le32(d, 0x3c) as usize;
    let image_base = le64(d, pe + 24 + 24);
    let dir = le32(d, pe + 24 + 112 + 13 * 8);
    let mut o = rva_off(d, dir);
    let mut out = Vec::new();
    loop {
        let (rva_dll, rva_iat, rva_int) = (le32(d, o + 4), le32(d, o + 12), le32(d, o + 16));
        if rva_dll == 0 {
            break;
        }
        let dll = cstr(d, rva_off(d, rva_dll));
        let mut k = 0usize;
        loop {
            let ent = le64(d, rva_off(d, rva_int) + 8 * k);
            if ent == 0 {
                break;
            }
            assert_eq!(ent >> 63, 0, "an import by ordinal");
            out.push((dll.clone(), cstr(d, rva_off(d, (ent & 0x7fff_ffff) as u32 + 2)), image_base + rva_iat as u64 + 8 * k as u64));
            k += 1;
        }
        o += 32;
    }
    out
}

#[test]
fn the_two_failing_delay_imports_are_what_the_decision_says() {
    // Census units 245 and 265 (gdb at the delay-load helper f_41d9a0): descriptor Shcore.dll with IAT slot 0x1976490 and descriptor kernel32.dll
    // with IAT slot 0x1976398. The names are read from the delay import directory of the file.
    let Some(d) = real_dll() else { return };
    let imps = delay_imports(&d);
    assert_eq!(imps.len(), 57);
    assert!(imps.iter().any(|(dll, f, slot)| dll == "Shcore.dll" && f == "GetProcessDpiAwareness" && *slot == 0x1976490));
    assert!(imps.iter().any(|(dll, f, slot)| dll == "kernel32.dll" && f == "GetSystemDefaultLocaleName" && *slot == 0x1976398));
    // exactly two Shcore functions, and the other eight absent names of decision 0029 are the delay-loaded DLLs that still have no module
    let shcore: Vec<&str> = imps.iter().filter(|(dll, _, _)| dll == "Shcore.dll").map(|(_, f, _)| f.as_str()).collect();
    assert_eq!(shcore, ["GetDpiForMonitor", "GetProcessDpiAwareness"]);
}

#[test]
fn the_guest_asks_shell32_for_exactly_the_nine_ordinals_of_the_table() {
    // unit 238: f_712080 = LoadLibraryW(L"shell32.dll") then GetProcAddress(h, n) with `mov edx, imm32` before each call 0x4235f0. The nine
    // immediates are read from the code bytes at 0x7120b1... (VA 0x712080 + the pattern ba <imm32> e8).
    let Some(d) = real_dll() else { return };
    let start = rva_off(&d, 0x712080 - 0x400000);
    let code = &d[start..start + 0x100];
    let mut ords = Vec::new();
    let mut i = 0;
    while i + 10 <= code.len() {
        // mov edx, imm32 (ba imm32) followed by call rel32 (e8) in the straight-line prologue of f_712080
        if code[i] == 0xba && code[i + 5] == 0xe8 {
            ords.push(le32(code, i + 1));
            i += 10;
        } else {
            i += 1;
        }
    }
    assert_eq!(ords, ORDINALS.iter().map(|(o, _)| *o).collect::<Vec<_>>());
    // the string it loads is "shell32.dll" in UTF-16 at 0x712184
    let s = rva_off(&d, 0x712184 - 0x400000);
    let w: Vec<u16> = (0..11).map(|k| u16::from_le_bytes([d[s + 2 * k], d[s + 2 * k + 1]])).collect();
    assert_eq!(String::from_utf16(&w).unwrap(), "shell32.dll");
}
