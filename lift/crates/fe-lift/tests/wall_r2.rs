//! Tests of decision 0107 (docs/decisions/0107-census-walls-batch3.md, census wall batch 3): the answers that keep three walls of the census
//! (units 276, 283, and 830 with 880) from stopping the run: GetSystemMetrics(SM_TABLETPC, SM_MAXIMUMTOUCHES, SM_CONVERTIBLESLATEMODE,
//! SM_SYSTEMDOCKED), CompareStringW (the first difference decides), and shfolder!SHGetFolderPathW with CSIDL_APPDATA.
//!
//! Expected values are written from decision 0107 (and decisions 0057, 0072, 0098, 0103, 0015), the Microsoft Learn pages named in it
//! (GetSystemMetrics, Handling Sorting in Your Applications, SHGetFolderPathW, the Windows-hardware page "Recommended settings for better
//! tablet experiences"), not read back from the C code. EVERY ANSWER IS PROVISIONAL: the host is Linux, nothing is compared with a real Windows,
//! and no display, file, locale, user name or folder of the host is read.
//!
//! 1. Generator: the new response-table row (shfolder.dll!SHGetFolderPathW) comes after the older rows, and both SHGetFolderPathW thunks of
//!    FLEngine (SHFolder.dll and shell32.dll) are answered.
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE (the pseudo-VAs of GetProcAddress are
//!    called through fle_dispatch, as the guest does).
//! 3. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent).

mod etxt_common;

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
    let d = manifest().join("../../out-wallR2/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/wall_s.rs)
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
fn the_shfolder_row_follows_the_shell32_row_of_the_same_function() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let at = |dll: &str, name: &str| rows.iter().position(|(d, n, _)| (d.as_str(), n.as_str()) == (dll, name)).unwrap_or_else(|| panic!("no row {dll}!{name}"));
    // decision 0015: append only. The row of decision 0103 (shell32) is older than the new row, which carries the DLL name in lower case, as every row.
    let (old, new) = (at("shell32.dll", "SHGetFolderPathW"), at("shfolder.dll", "SHGetFolderPathW"));
    assert!(old < new, "shell32 row {old}, shfolder row {new}");
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    // one row per DLL and name: no other row for shfolder.dll
    assert_eq!(rows.iter().filter(|(d, _, _)| d == "shfolder.dll").count(), 1);
    // the two rows are two pseudo-VAs of one runtime function (one prototype name for both in the dispatch table)
    assert!(c.contains("void fle_imp_SHGetFolderPathW(void);"));
    // each row names its function once in fle_dispatch_table and once in fle_resp_table: two rows, four lines
    let named = c.matches("fle_imp_SHGetFolderPathW },").count();
    assert_eq!(named, 4, "two pseudo-VAs of one function");
}

#[test]
fn both_sh_get_folder_path_w_thunks_of_flengine_are_answered_under_their_own_dll() {
    // FLEngine imports the function twice: from SHFolder.dll (IAT 0x1971820, thunk 0x4262e0) and from shell32.dll (IAT 0x1971f98, thunk 0x593450).
    // The DLL name is compared without case (the PE says "SHFolder.dll"); another DLL's SHGetFolderPathW is another API.
    let (st, c) = gen(&[("kernel32.dll", "GetProcAddress"), ("SHFolder.dll", "SHGetFolderPathW"), ("shell32.dll", "SHGetFolderPathW"), ("user32.dll", "SHGetFolderPathW")]);
    assert_eq!(st.handled["jmp_imp:SHFolder.SHGetFolderPathW"], 1);
    assert_eq!(st.handled["jmp_imp:SHGetFolderPathW"], 1);
    assert_eq!(c.matches("fle_imp_SHGetFolderPathW(); return;").count(), 2, "the user32 thunk is not answered");
    let va = IB + 0x1040 + 0x10 * 3;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "the thunk of user32!SHGetFolderPathW stays unsupported");
    // the ANSI export of shfolder.dll has no row
    let (st, _) = gen(&[("kernel32.dll", "GetProcAddress"), ("SHFolder.dll", "SHGetFolderPathA")]);
    assert!(!st.handled.keys().any(|k| k.contains("SHGetFolderPathA")), "{:?}", st.handled);
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
#include "fle_sysmetrics.h"
#include "fle_env.h"

static uint64_t stk[16];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define N_KERNEL32 9
#define N_SHELL32 16
#define N_SHFOLDER 17

/* Registers and stack as at the entry of an API: rcx, rdx, r8, r9, then the 5th and 6th arguments at [rsp+0x28], [rsp+0x30]. */
static void prep6(uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    stk[5] = a5;
    stk[6] = a6;
    R[4] = A(stk);
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static void prep(uint64_t a, uint64_t b) { prep6(a, b, 0, 0, 0, 0); }
static uint64_t le(void) { uint64_t r = R[0], e; fle_imp_GetLastError(); e = R[0]; R[0] = r; return e; }
static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n)); fle_imp_GetProcAddress();
    r = R[0];
    printf("%s %llu\n", r ? "found" : "missing", U(le()));
    return r;
}
static void callva6(uint64_t va, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6)
{
    prep6(a1, a2, a3, a4, a5, a6);
    fle_dispatch(va);
}

/* "hex:61,e9" is the UTF-16 string with these units; anything else is ASCII. Always NUL-terminated (explicit lengths may stop before it). */
static uint16_t *wide(const char *s)
{
    size_t n, i = 0;
    uint16_t *w;
    if (!strncmp(s, "hex:", 4)) {
        const char *p = s + 4;
        w = calloc(strlen(p) + 2, 2);
        while (*p) {
            char *e;
            w[i++] = (uint16_t)strtoul(p, &e, 16);
            p = *e == ',' ? e + 1 : e;
        }
        return w;
    }
    n = strlen(s);
    w = calloc(n + 2, 2);
    for (i = 0; i < n; i++) w[i] = (uint8_t)s[i];
    return w;
}

static void show(const uint16_t *p, uint64_t buf_len)
{
    uint64_t i;
    for (i = 0; i < buf_len && p[i]; i++) putchar((int)p[i]);
}

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    if (!strcmp(s, "sm")) {                 /* index */
        prep(strtoull(argv[2], 0, 0), 0);
        fle_imp_GetSystemMetrics();
        printf("ret %d le %llu\n", (int)R[0], U(le()));
    } else if (!strcmp(s, "sm-guest-f_80e360")) {
        /* The calls of the guest's f_80e360 in its order: 94, 86, 0x2003, 0x2004, then (f_80e3f0) 95. Prints the five answers. */
        static const uint32_t order[5] = { 94, 86, 0x2003, 0x2004, 95 };
        int i;
        for (i = 0; i < 5; i++) {
            prep(order[i], 0);
            fle_imp_GetSystemMetrics();
            printf("%x %d\n", order[i], (int)R[0]);
        }
    } else if (!strcmp(s, "cmp")) {         /* lcid flags s1 n1 s2 n2 */
        uint16_t *a = wide(argv[4]), *b = wide(argv[6]);
        prep6(strtoull(argv[2], 0, 0), strtoull(argv[3], 0, 0), A(a), (uint64_t)(int64_t)strtoll(argv[5], 0, 0), A(b), (uint64_t)(int64_t)strtoll(argv[7], 0, 0));
        fle_imp_CompareStringW();
        printf("ret %llu le %llu\n", U(R[0]), U(le()));
    } else if (!strcmp(s, "shf-lookup")) {
        uint64_t a = gpa(M(N_SHFOLDER), "SHGetFolderPathW"), b = gpa(M(N_SHELL32), "SHGetFolderPathW");
        gpa(M(N_SHFOLDER), "SHGetFolderPathA");        /* no row */
        gpa(M(N_SHFOLDER), "shgetfolderpathw");        /* names are compared with exact case */
        gpa(M(N_KERNEL32), "SHGetFolderPathW");        /* the rows are under shell32.dll and shfolder.dll only */
        printf("distinct %d both-pseudo %d\n", a != b, a >= 0x7FF900000000ull && b >= 0x7FF900000000ull);
    } else if (!strcmp(s, "shf")) {         /* via(shfolder|shell32) csidl token flags buf|null */
        static uint16_t p[300];
        uint64_t m = !strcmp(argv[2], "shfolder") ? M(N_SHFOLDER) : M(N_SHELL32), va;
        size_t i;
        for (i = 0; i < 300; i++) p[i] = 0xAAAA;
        prep(m, A("SHGetFolderPathW")); fle_imp_GetProcAddress(); va = R[0];
        callva6(va, 0, strtoull(argv[3], 0, 0), strtoull(argv[4], 0, 0), strtoull(argv[5], 0, 0), !strcmp(argv[6], "buf") ? A(p) : 0, 0);
        printf("hr %llx le %llu [", U(R[0]), U(le()));
        show(p, 300);
        for (i = 0; i < 300 && p[i]; i++) {}
        printf("] len %zu nul %x next %04x\n", i, p[i], p[i + 1]);
    } else if (!strcmp(s, "guest-appdata")) {
        /* What f_125da90 (unit 880) and f_10c7cd0 (unit 830) do: LoadLibraryW(L"SHFolder.dll"), GetProcAddress(h, "SHGetFolderPathW"), the call
         * (0, CSIDL_APPDATA, NULL, 0, buf) without a NULL test, then FreeLibrary(h). */
        static uint16_t p[300];
        uint16_t *dll = wide("SHFolder.dll");
        uint64_t h, va;
        size_t i;
        p[0] = 0;
        prep(A(dll), 0); fle_imp_LoadLibraryW(); h = R[0];
        printf("handle %llx le %llu\n", U(h), U(le()));
        prep(h, A("SHGetFolderPathW")); fle_imp_GetProcAddress(); va = R[0];
        printf("proc %s le %llu\n", va ? "found" : "NULL", U(le()));
        callva6(va, 0, 0x1a, 0, 0, A(p), 0);
        printf("hr %llx path [", U(R[0]));
        show(p, 300);
        for (i = 0; i < 300 && p[i]; i++) {}
        printf("] len %zu\n", i);
        prep(h, 0); fle_imp_FreeLibrary();
        printf("free %llu\n", U(R[0]));
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
        let tag = etxt_common::per_process("wall-r2-driver");
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

fn run(args: &[&str]) -> Out {
    let r = Command::new(driver()).args(args).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(args: &[&str]) -> Vec<String> {
    let o = run(args);
    assert!(o.ok, "{args:?}: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn aborts(args: &[&str], must_say: &str) -> Out {
    let o = run(args);
    assert!(!o.ok, "{args:?}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{args:?}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle:"), "{args:?}: no fle: message: {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{args:?}: message lacks {must_say:?}: {}", o.stderr);
    o
}

fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(rt_dir().join(file)).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

// ---- 2.1 GetSystemMetrics: the four indexes of f_80e360 after SM_DIGITIZER (unit 276) ----

/// (index, Learn name, value of decision 0107). 86 and 95 follow from the page text and the environment (Windows 10 is neither of the two systems
/// the page lists for SM_TABLETPC; there is no digitizer for SM_MAXIMUMTOUCHES); 0x2003 and 0x2004 are choices (the page gives no value for a
/// desktop PC; the OEM page recommends ConvertibleSlateMode 1 for a desktop).
const SM_ANSWERS: [(u64, &str, i32); 4] = [(86, "SM_TABLETPC", 0), (95, "SM_MAXIMUMTOUCHES", 0), (0x2003, "SM_CONVERTIBLESLATEMODE", 1), (0x2004, "SM_SYSTEMDOCKED", 0)];

#[test]
fn the_four_indexes_after_sm_digitizer_are_answered_and_the_last_error_is_left_alone() {
    for (idx, name, v) in SM_ANSWERS {
        assert_eq!(lines(&["sm", &idx.to_string()]), [format!("ret {v} le 5")], "{name}");
        // the index is an int: the upper half of rcx does not count
        assert_eq!(lines(&["sm", &format!("{:#x}", 0xFFFF_FFFF_0000_0000u64 | idx)]), [format!("ret {v} le 5")], "{name} with a dirty upper half");
    }
}

#[test]
fn the_index_numbers_and_values_are_the_learn_numbers_and_the_decided_values() {
    // The numbers are those of the Learn table of GetSystemMetrics: SM_TABLETPC 86, SM_MAXIMUMTOUCHES 95, SM_CONVERTIBLESLATEMODE 0x2003,
    // SM_SYSTEMDOCKED 0x2004 (and SM_DIGITIZER 94, answered by decision 0098).
    for (idx, name, v) in SM_ANSWERS {
        let short = name.strip_prefix("SM_").unwrap();
        assert_eq!(def_const("fle_sysmetrics.h", &format!("FLE_SM_IDX_{short}")), idx, "{name}");
        assert_eq!(def_const("fle_sysmetrics.h", &format!("FLE_SM_{short}")), v as u64, "{name}");
    }
    assert_eq!(def_const("fle_sysmetrics.h", "FLE_SM_IDX_DIGITIZER"), 94);
}

#[test]
fn the_guests_decision_after_these_answers_is_not_tablet_and_does_not_depend_on_the_slate_value() {
    // f_80e360 (read from the DLL's bytes, see the test on the real DLL): [ec8] = SM_DIGITIZER & 1, [ec9] = (SM_DIGITIZER & 0xc) != 0, [ecb] = SM_TABLETPC != 0,
    // [eca] = SM_CONVERTIBLESLATEMODE == 0, [ecc] = SM_SYSTEMDOCKED != 0 (never read again). f_80e3f0 then sets the mode: 2 when [ec8] and [eca], else
    // 1 when [ec9] or [ecb], else 0; and asks SM_MAXIMUMTOUCHES, which matters only for mode 2.
    let got = lines(&["sm-guest-f_80e360"]);
    assert_eq!(got, ["5e 0", "56 0", "2003 1", "2004 0", "5f 0"]);
    let v: Vec<i32> = got.iter().map(|l| l.split(' ').nth(1).unwrap().parse().unwrap()).collect();
    let mode = |digitizer: i32, tablet: i32, slate_metric: i32| {
        let (touch, pen, tablet_pc, slate) = (digitizer & 1 != 0, digitizer & 0xc != 0, tablet != 0, slate_metric == 0);
        if touch && slate {
            2
        } else if pen || tablet_pc {
            1
        } else {
            0
        }
    };
    assert_eq!(mode(v[0], v[1], v[2]), 0, "the mode with the decided answers");
    // the choice 1 against 0 for SM_CONVERTIBLESLATEMODE does not change the guest's mode while SM_DIGITIZER is 0
    assert_eq!(mode(v[0], v[1], 0), 0);
    assert_eq!(mode(v[0], v[1], 1), 0);
    // it would, with an integrated touch digitizer (NID_INTEGRATED_TOUCH): that is why decision 0107 lists the value as a point for the PM
    assert_eq!(mode(1, v[1], 0), 2);
    assert_eq!(mode(1, v[1], 1), 0);
}

#[test]
fn the_neighbouring_indexes_are_still_not_answered() {
    // 87 SM_MEDIACENTER, 88 SM_STARTER, 93, 96 and 0x2002 are not asked by the guest and stop by number; 0x2001 as before.
    for idx in ["87", "88", "93", "96", "0x2002", "0x2001"] {
        let o = run(&["sm", idx]);
        assert_eq!(o.signal, Some(6), "{idx}: {}", o.stderr);
        assert!(o.stderr.contains("is not answered"), "{idx}: {}", o.stderr);
    }
    // and the seventeen indexes of decision 0057 that stay undecided keep their message (tests/sysmetrics.rs lists them all)
    aborts(&["sm", "2"], "index 2 (SM_CXVSCROLL) is used by the guest but no value is decided (decision 0057)");
    aborts(&["sm", "69"], "index 69 (SM_CYDRAG) is used by the guest but no value is decided (decision 0057)");
}

// ---- 2.2 CompareStringW: the first difference decides (unit 283) ----

fn cmp(lcid: &str, flags: &str, a: &str, b: &str) -> String {
    // -1: NUL-terminated, as the guest's Delphi CompareText passes the lengths it has; explicit lengths are used in the second form
    let n = |s: &str| if let Some(h) = s.strip_prefix("hex:") { if h.is_empty() { 0 } else { h.split(',').count() } } else { s.len() };
    let (la, lb) = (n(a).to_string(), n(b).to_string());
    let x = lines(&["cmp", lcid, flags, a, "-1", b, "-1"]);
    let y = lines(&["cmp", lcid, flags, a, &la, b, &lb]);
    assert_eq!(x, y, "NUL-terminated and explicit lengths differ for {a:?} {b:?}");
    assert_eq!(x.len(), 1);
    x[0].clone()
}

#[test]
fn the_pair_of_unit_283_is_decided_at_its_first_unit() {
    // Delphi's CompareText of f_4399e0 calls CompareStringW(LOCALE_USER_DEFAULT 0x400, NORM_IGNORECASE, ...) and subtracts 2. In the callback of
    // EnumFontFamiliesExW (decision 0098) it compares the last entry of the list, "Default", with the new face name "Segoe UI". Learn ("Handling
    // Sorting in Your Applications"): the sorting functions "evaluate strings character by character"; D and S are letters that differ at position 0,
    // and 'd' sorts before 's'. The space of "Segoe UI" comes after the first difference. Before decision 0107 this stopped on the space.
    for lcid in ["0x400", "0x409", "0x800"] {
        for flags in ["1", "0"] {
            assert_eq!(cmp(lcid, flags, "Default", "Segoe UI"), "ret 1 le 5", "{lcid} {flags}");
            assert_eq!(cmp(lcid, flags, "Segoe UI", "Default"), "ret 3 le 5", "{lcid} {flags}");
        }
    }
}

#[test]
fn what_follows_the_first_difference_is_not_looked_at() {
    // The first differing unit decides; the tails may hold spaces, symbols, digits, and non-ASCII units.
    let rows = [
        ("abc de", "abd e", 1),
        ("abd e", "abc de", 3),
        ("a1 x", "a2", 1),
        ("Zeta y", "alpha x", 3),
        ("alpha x", "Zeta y", 1),
        ("ab_c", "ac", 1),
        ("hex:61,62,e9", "hex:61,63", 1),
        ("hex:61,63", "hex:61,62,2022,20", 3),
        ("x y z", "y", 1),
        ("9 lives", "1", 3),
    ];
    for flags in ["0", "1"] {
        for (a, b, want) in rows {
            assert_eq!(cmp("0x409", flags, a, b), format!("ret {want} le 5"), "{a:?} {b:?} flags {flags}");
        }
    }
}

#[test]
fn the_old_decided_cases_keep_their_answers() {
    // equal strings (also after folding with NORM_IGNORECASE), and the all-alphanumeric order of tests/locale.rs
    assert_eq!(cmp("0x409", "0", "abc", "abc"), "ret 2 le 5");
    assert_eq!(cmp("0x409", "1", "ABC", "abc"), "ret 2 le 5");
    assert_eq!(cmp("0x409", "0", "abc", "abd"), "ret 1 le 5");
    assert_eq!(cmp("0x409", "0", "a9", "a10"), "ret 3 le 5");
    assert_eq!(cmp("0x409", "0", "Abc", "abd"), "ret 1 le 5");
    // a primary difference outranks a case difference before it (decision 0087-era reading, unchanged)
    assert_eq!(cmp("0x409", "0", "Ab", "aC"), "ret 1 le 5");
    // decision 0072: a symbol against a letter or digit at position 0
    assert_eq!(cmp("0x409", "0", "_x64.dll", "ppv2.dll"), "ret 1 le 5");
    assert_eq!(cmp("0x409", "0", "ppv2.dll", "_x64.dll"), "ret 3 le 5");
}

#[test]
fn what_is_not_decided_still_stops_by_name() {
    // Each of these stops with a "fle: CompareStringW: " message. The reason (where it matters) is in the message.
    let stop = |lcid: &str, flags: &str, a: &str, b: &str, why: &str| {
        aborts(&["cmp", lcid, flags, a, "-1", b, "-1"], &format!("CompareStringW: {why}"));
    };
    // a space (or any non-alphanumeric unit) BEFORE the first difference: the page does not say how it weighs (hyphen and apostrophe: the page says they are special)
    // decision 0126 (walls 62, 68; S-cmp-equal-prefix, S-cmp-equal-hyphen): old: "a b" against "a c" and "a-b" against "a-c" stopped here ("character other than an ASCII letter or digit before the first difference");
    // a unit that is equal on both sides at the same position does not decide the order, so these two pairs give CSTR_LESS_THAN now (tests/locale.rs compare_string_one_a_prefix_of_the_other_is_the_shorter_first)
    // the equal unit is a control character (0x01), which is outside the printable symbols the rule accepts: still a stop, with the new message of decision 0126
    // decision 0284 (B): old: stop("0x409", "0", "hex:61,1,62", "hex:61,1,63", "character other than an ASCII letter, digit or printable symbol before the first difference"). The Sorting Weight Table gives
    // U+0001 a weight (Script Member 6, Alphabetic Weight 3) and it is in every build now: the equal unit does not decide, "b" (14/9) against "c" (14/10) does.
    assert_eq!(lines(&["cmp", "0x409", "0", "hex:61,1,62", "-1", "hex:61,1,63", "-1"]), ["ret 1 le 5".to_string()]);
    assert_eq!(lines(&["cmp", "0x409", "0", "hex:61,1,63", "-1", "hex:61,1,62", "-1"]), ["ret 3 le 5".to_string()]);
    // a symbol AT the first difference after position 0 (decision 0072 limits the symbol rule to position 0)
    // decision 0129 (walls 7, 8, 13; S-cmp-symbol-any-position): old: "a_b" against "ab" and "Ab_" against "aBc" stopped here ("character other than an ASCII letter or digit in unequal strings"). A printable
    // symbol (the underscore) against a letter at the first difference is the smaller wherever it is (Learn "Handling Sorting in Your Applications"), so both give CSTR_LESS_THAN (1); the same pairs are pinned in tests/locale.rs
    assert_eq!(lines(&["cmp", "0x409", "0", "a_b", "-1", "ab", "-1"]), ["ret 1 le 5".to_string()]);
    assert_eq!(lines(&["cmp", "0x409", "0", "Ab_", "-1", "aBc", "-1"]), ["ret 1 le 5".to_string()]);
    // decision 0126 (wall 72, S-cmp-prefix): old: "a b" against "a" stopped ("prefix or case-only difference"); the shorter string is the smaller now
    // (tests/locale.rs); a case-only difference of equal length stays a stop (the "ABC" against "abc" row below)
    // a non-ASCII unit at the first difference
    // decision 0284 (B): old: stop("0x409", "0", "hex:e9", "a", ...) and stop("0x409", "0", "a b", "hex:61,e9", ...) ("character other than an ASCII letter or digit in unequal strings"). The Sorting Weight Table is in
    // every build now: U+00E9 is 14/33 ("e Acute"), "a" is 14/2, so U+00E9 is the greater (3); "a b" against "a" U+00E9: the space (7/2) against U+00E9 (14/33), the space is the smaller (1).
    assert_eq!(lines(&["cmp", "0x409", "0", "hex:e9", "-1", "a", "-1"]), ["ret 3 le 5".to_string()]);
    assert_eq!(lines(&["cmp", "0x409", "0", "a b", "-1", "hex:61,e9", "-1"]), ["ret 1 le 5".to_string()]);
    // a letter against a digit, a prefix, a case-only difference without NORM_IGNORECASE, empty against non-empty
    // decision 0171 (wall ia1-W11, S-ia1-cmp-digit-letter): old: stop("0x409", "0", "a", "1 b", "letter against digit"); a digit against a letter is ordered digit first now
    assert_eq!(lines(&["cmp", "0x409", "0", "a", "-1", "1 b", "-1"]), ["ret 3 le 5".to_string()]);
    assert_eq!(lines(&["cmp", "0x409", "0", "1 b", "-1", "a", "-1"]), ["ret 1 le 5".to_string()]);
    // old: "abc" against "abc d" and "" against "a b" stopped as prefixes; decision 0126 (S-cmp-prefix) answers them

    // decision 0126 (wall 72): old word "prefix or case-only difference" (the stop covered the prefix too); the prefix is answered, the stop that is left says "case-only difference"
    stop("0x409", "0", "ABC", "abc", "case-only difference");

    // other flags (SORT_STRINGSORT, NORM_IGNORENONSPACE) and other locales
    // decision 0129 (wall 5; S-cmp-digits-flag): old word "dwCmpFlags other than 0 and NORM_IGNORECASE"; SORT_DIGITSASNUMBERS (0x8) is accepted now, so the message names it too
    stop("0x409", "0x1000", "Default", "Segoe UI", "dwCmpFlags other than 0, NORM_IGNORECASE and SORT_DIGITSASNUMBERS");
    stop("0x409", "2", "Default", "Segoe UI", "dwCmpFlags other than 0, NORM_IGNORECASE and SORT_DIGITSASNUMBERS");
    stop("0x407", "1", "Default", "Segoe UI", "locale other than the emulated one");
    stop("0x7f", "1", "Default", "Segoe UI", "locale other than the emulated one");
}

// ---- 2.3 shfolder!SHGetFolderPathW and CSIDL_APPDATA (units 830, 880) ----

const APPDATA: &str = "C:\\Users\\user\\AppData\\Roaming";
const FONTS: &str = "C:\\Windows\\Fonts";

#[test]
fn shfolder_has_the_shget_folder_path_w_row_and_no_other() {
    // module N = 17 is shfolder.dll (the 17th DLL of the PE import table, decision 0015), N = 16 is shell32.dll
    let got = lines(&["shf-lookup"]);
    assert_eq!(got, ["found 5", "found 5", "missing 127", "missing 127", "missing 127", "distinct 1 both-pseudo 1"]);
}

#[test]
fn csidl_appdata_is_the_virtual_roaming_app_data_folder_through_either_dll() {
    // Learn: S_OK (0); "The returned path does not include a trailing backslash"; the buffer is written with its NUL. The default of
    // FOLDERID_RoamingAppData (which CSIDL_APPDATA maps to since Windows Vista) is %USERPROFILE%\AppData\Roaming = C:\Users\<user name>\AppData\Roaming; the
    // user name "user" is a choice of decision 0107. SHGFP_TYPE_CURRENT (0) and SHGFP_TYPE_DEFAULT (1) give the same path: nothing is redirected.
    let want = |p: &str| format!("hr 0 le 5 [{p}] len {} nul 0 next aaaa", p.len());
    assert_eq!(APPDATA.len(), 29);
    for via in ["shfolder", "shell32"] {
        assert_eq!(lines(&["shf", via, "0x1a", "0", "0", "buf"]), [want(APPDATA)], "{via}");
        assert_eq!(lines(&["shf", via, "0x1a", "0", "1", "buf"]), [want(APPDATA)], "{via} DEFAULT");
        // CSIDL_FONTS (decision 0103) is answered by the second DLL as well
        assert_eq!(lines(&["shf", via, "0x14", "0", "0", "buf"]), [want(FONTS)], "{via} fonts");
    }
    // the path is built from the virtual user profile directory (one place for the user name) and the Windows Vista and later subdirectory
    let h = std::fs::read_to_string(rt_dir().join("fle_env.h")).unwrap();
    assert!(h.contains("#define FLE_VIRTUAL_USER_PROFILE_DIR \"C:\\\\Users\\\\user\""), "fle_env.h");
    let c = std::fs::read_to_string(rt_dir().join("fle_env.c")).unwrap();
    assert!(c.contains("#define FLE_APPDATA_SUBDIR \"\\\\AppData\\\\Roaming\""), "fle_env.c");
    assert!(c.contains("#define FLE_CSIDL_APPDATA 0x001a"), "fle_env.c");
}

#[test]
fn the_guests_machine_id_sequence_gets_a_non_null_function_and_the_appdata_path() {
    // f_125da90 (unit 880) and f_10c7cd0 (unit 830): LoadLibraryW(L"SHFolder.dll"), GetProcAddress(h, "SHGetFolderPathW") without a NULL test,
    // the call (0, 0x1a, NULL, 0, buf), then FreeLibrary(h). The handle is module N = 17: 0x7FF800000000 + 0x10000 * 17 = 0x7ff800110000 (the value
    // that gdb showed at the wall, tests of decision 0099). The reference count was 1 (loaded at start), so the FreeLibrary succeeds.
    let got = lines(&["guest-appdata"]);
    assert_eq!(
        got,
        [
            "handle 7ff800110000 le 5".to_string(),
            "proc found le 5".to_string(),
            "hr 0 path [C:\\Users\\user\\AppData\\Roaming] len 29".to_string(),
            "free 1".to_string(),
        ]
    );
}

#[test]
fn the_other_csidl_values_flags_tokens_and_a_null_buffer_stop_by_name() {
    for via in ["shfolder", "shell32"] {
        let name = "SHGetFolderPathW";
        // any other CSIDL, and the flag bits CSIDL_FLAG_CREATE (0x8000) and CSIDL_FLAG_DONT_VERIFY (0x4000), which change what the call does
        // decision 0126 (walls 59, 60, 80): old: 0x24, 0x5 and 0x1c were here ("csidl 0x24 is not answered (only CSIDL_FONTS 0x14 and CSIDL_APPDATA 0x1a are)"); 0x05 PERSONAL, 0x1c LOCAL_APPDATA, 0x23 COMMON_APPDATA,
        // 0x24 WINDOWS, 0x26 PROGRAM_FILES, 0x28 PROFILE, 0x2a, 0x2b, 0x2c are answered now (tests/wall_fr11.rs); 0x25 (CSIDL_SYSTEM) and 0x2d have no answer yet and stop, with the new message
        aborts(&["shf", via, "0x25", "0", "0", "buf"], &format!("{name}: csidl 0x25 is not answered"));
        aborts(&["shf", via, "0x2d", "0", "0", "buf"], &format!("{name}: csidl 0x2d is not answered"));
        aborts(&["shf", via, "0x801a", "0", "0", "buf"], &format!("{name}: csidl 0x801a is not answered"));
        aborts(&["shf", via, "0x401a", "0", "0", "buf"], &format!("{name}: csidl 0x401a is not answered"));
        aborts(&["shf", via, "0x1a", "5", "0", "buf"], &format!("{name}: hToken is not NULL"));
        aborts(&["shf", via, "0x1a", "0", "2", "buf"], &format!("{name}: dwFlags is neither"));
        aborts(&["shf", via, "0x1a", "0", "0", "null"], &format!("{name}: pszPath is NULL"));
    }
}

// ---------------------------------------------------------------------------------------------
// 3. The real DLL
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}

/// The file bytes at virtual address `va` (the section table is read here, not through fe-pe).
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let le32 = |o: usize| u32::from_le_bytes(dll[o..o + 4].try_into().unwrap());
    let le16 = |o: usize| u16::from_le_bytes(dll[o..o + 2].try_into().unwrap());
    let lf = le32(0x3c) as usize;
    let nsec = le16(lf + 6) as usize;
    let osz = le16(lf + 20) as usize;
    let rva = (va - 0x400000) as u32;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(o + 8), le32(o + 12), le32(o + 16), le32(o + 20));
        if rva >= sva && rva < sva + vs.max(rs) {
            let off = rp as usize + (rva - sva) as usize;
            return dll[off..off + n].to_vec();
        }
    }
    panic!("va {va:#x} is in no section");
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn the_real_dll_has_the_two_thunks_and_25_call_setups_with_csidl_1a() {
    let Some(dll) = real_dll() else { return };
    // the thunk 0x4262e0 jumps through the IAT slot 0x1971820 (SHFolder.dll!SHGetFolderPathW by pefile / lift/out-imports/imports.txt), the thunk
    // 0x593450 through 0x1971f98 (shell32.dll!SHGetFolderPathW, decision 0103): jmp [rip+disp32], disp = slot - (thunk + 6)
    assert_eq!(bytes_at(&dll, 0x4262e0, 6), [0xff, 0x25, 0x3a, 0xb5, 0x54, 0x01]);
    assert_eq!(0x4262e0u64 + 6 + 0x0154_b53a, 0x1971820);
    assert_eq!(bytes_at(&dll, 0x593450, 6), [0xff, 0x25, 0x42, 0xeb, 0x3d, 0x01]);
    assert_eq!(0x593450u64 + 6 + 0x013d_eb42, 0x1971f98);
    // 24 copies of the UTF-16 strings "SHFolder.dll" and "SHGetFolderPathW" (each in a function that fetches the function with LoadLibraryW +
    // GetProcAddress), and 25 call setups `xor ecx,ecx; mov edx,0x1a; xor r8,r8; xor r9,r9` (CSIDL_APPDATA, hToken NULL, dwFlags 0) in the file.
    let wide = |s: &str| s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>();
    assert_eq!(count(&dll, &wide("SHFolder.dll")), 24);
    assert_eq!(count(&dll, &wide("SHGetFolderPathW")), 24);
    assert_eq!(count(&dll, &[0x33, 0xc9, 0xba, 0x1a, 0x00, 0x00, 0x00, 0x4d, 0x33, 0xc0, 0x4d, 0x33, 0xc9]), 25);
    // unit 880's function f_125da90: LoadLibraryW(L"SHFolder.dll") at 0x125dc0f/0x125dc16 and the call through the GetProcAddress result at
    // 0x125dc55 (`call [rbp+0x198]`); unit 830's f_10c7cd0 has the same at 0x10c8941 (`call [rbp+0x860]`)
    assert_eq!(bytes_at(&dll, 0x125dc55, 6), [0xff, 0x95, 0x98, 0x01, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x10c8941, 6), [0xff, 0x95, 0x60, 0x08, 0x00, 0x00]);
}

#[test]
fn the_real_dll_f_80e360_stores_the_answers_as_the_decision_reads_them() {
    let Some(dll) = real_dll() else { return };
    // 0x80e3a4: mov ecx,0x56 (the byte before is the previous call); the store of SM_TABLETPC is `test eax,eax; setne al; mov [rip+d],al`
    assert_eq!(bytes_at(&dll, 0x80e3a4, 12), [0xe8, 0x37, 0x6b, 0xc1, 0xff, 0x85, 0xc0, 0x0f, 0x95, 0xc0, 0x88, 0x05]);
    // SM_CONVERTIBLESLATEMODE: `test eax,eax; sete al; mov [rip+d],al` (the flag is "== 0"), then `mov ecx,0x2004`
    assert_eq!(bytes_at(&dll, 0x80e3b9, 17), [0xe8, 0x22, 0x6b, 0xc1, 0xff, 0x85, 0xc0, 0x0f, 0x94, 0xc0, 0x88, 0x05, 0x01, 0x8b, 0xde, 0x00, 0xb9]);
    assert_eq!(bytes_at(&dll, 0x80e3ce, 12), [0xe8, 0x0d, 0x6b, 0xc1, 0xff, 0x85, 0xc0, 0x0f, 0x95, 0xc0, 0x88, 0x05]);
    // f_80e3f0 combines them: movzx rax,[ec8]; test [eca],al  (touch and slate -> mode 2); else movzx rax,[ec9]; or al,[ecb] (pen or tablet PC -> mode 1)
    assert_eq!(bytes_at(&dll, 0x80e4b3, 14), [0x48, 0x0f, 0xb6, 0x05, 0x0d, 0x8a, 0xde, 0x00, 0x84, 0x05, 0x09, 0x8a, 0xde, 0x00]);
    assert_eq!(bytes_at(&dll, 0x80e4cf, 14), [0x48, 0x0f, 0xb6, 0x05, 0xf2, 0x89, 0xde, 0x00, 0x0a, 0x05, 0xee, 0x89, 0xde, 0x00]);
    // the call that asks SM_MAXIMUMTOUCHES (0x5f) and stores it in [ec4]
    assert_eq!(bytes_at(&dll, 0x80e501, 5), [0xb9, 0x5f, 0x00, 0x00, 0x00]);
}

#[test]
fn the_real_dll_compare_text_passes_user_default_locale_and_norm_ignorecase() {
    let Some(dll) = real_dll() else { return };
    // f_4399e0 (Delphi CompareText for a UnicodeString): ecx = 0x400 (LOCALE_USER_DEFAULT), edx = 1 (NORM_IGNORECASE), r8 = string 1, r9d = its length,
    // [rsp+0x20] = string 2, [rsp+0x28] = its length, call 0x4231c0 (CompareStringW), then `sub eax,2`
    assert_eq!(bytes_at(&dll, 0x43_9a1f, 10), [0xb9, 0x00, 0x04, 0x00, 0x00, 0xba, 0x01, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x43_9a38, 8), [0xe8, 0x83, 0x97, 0xfe, 0xff, 0x83, 0xe8, 0x02]);
}
