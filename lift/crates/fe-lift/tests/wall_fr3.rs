//! Tests of decision 0116 (docs/decisions/0116-fruity-walls.md, the walls of CreateFruityInstance): the registry seeding of the harness (FLE_REG_SEED,
//! harness/reg_seed.h), the two shell32 answers behind the Delphi helper f_63ab90 of FLEngine (SHGetSpecialFolderLocation and SHGetPathFromIDListW) and
//! GetCPInfo for CP_UTF8.
//!
//! Expected values are written from the Microsoft Learn pages named in the decision (SHGetSpecialFolderLocation, SHGetPathFromIDListW, SHGetFolderPathW,
//! CPINFO, GetCPInfo, CoTaskMemFree), from the format description of harness/reg_seed.h, and from decisions 0015, 0018, 0098, 0103, 0107; they are not read back
//! from the C code. EVERY ANSWER IS PROVISIONAL: the host is Linux, nothing is compared with a real Windows, and no folder, registry or user name of the host is
//! read.
//!
//! 1. Generator: the two new response-table rows are the last two, in the order they were reached, and both thunks of FLEngine are answered.
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE.
//! 3. The harness: the self-test of reg_seed.h (harness/seeds/reg_seed_selftest.c), the rules of a4-multi.sh for REG_SEED, and the guarantee that a build without
//!    FLE_FRUITY contains no registry seeding.
//! 4. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent).

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
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR3/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/wall_r2.rs)
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
fn the_two_shell32_rows_follow_the_0112_rows_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // decision 0015: append only. The rows of decision 0112 keep their places; the two rows of decision 0116 follow them, in the order the walls reached them (wall 2:
    // SHGetSpecialFolderLocation, then the SHGetPathFromIDListW call that follows it in the same Delphi helper). They were the last two rows when this test was written;
    // the rows of decision 0117 (tests/wall_fr4.rs) were appended after them.
    let at = |dll: &str, name: &str| rows.iter().position(|(d, nm, _)| (d.as_str(), nm.as_str()) == (dll, name)).unwrap_or_else(|| panic!("no row {dll}!{name}"));
    let k = at("kernel32.dll", "GetComputerNameExA");
    assert_eq!((rows[k + 1].0.as_str(), rows[k + 1].1.as_str()), ("shell32.dll", "SHGetSpecialFolderLocation"));
    assert_eq!((rows[k + 2].0.as_str(), rows[k + 2].1.as_str()), ("shell32.dll", "SHGetPathFromIDListW"));
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    // the position of GetComputerNameExW in the table (older than GetComputerNameExA by 6 rows) pins that nothing was inserted in between
    assert_eq!(at("kernel32.dll", "GetComputerNameExA"), at("kernel32.dll", "GetComputerNameExW") + 6);
    assert_eq!(rows.iter().filter(|(d, nm, _)| d == "shell32.dll" && (nm == "SHGetSpecialFolderLocation" || nm == "SHGetPathFromIDListW")).count(), 2);
    assert!(c.contains("void fle_imp_SHGetSpecialFolderLocation(void);"));
    assert!(c.contains("void fle_imp_SHGetPathFromIDListW(void);"));
}

#[test]
fn both_thunks_are_answered_under_shell32_and_no_other_dll() {
    // FLEngine imports shell32.dll!SHGetSpecialFolderLocation (IAT 0x1971f38, thunk 0x593370) and shell32.dll!SHGetPathFromIDListW (IAT 0x1971f50, thunk 0x593350).
    // The DLL name is compared without case; another DLL's function of the same name is another API.
    let (st, c) = gen(&[
        ("SHELL32.dll", "SHGetSpecialFolderLocation"),
        ("shell32.dll", "SHGetPathFromIDListW"),
        ("user32.dll", "SHGetSpecialFolderLocation"),
        ("shell32.dll", "SHGetPathFromIDListA"),
        ("kernel32.dll", "GetProcAddress"),
    ]);
    assert_eq!(st.handled["jmp_imp:SHGetSpecialFolderLocation"], 1);
    assert_eq!(st.handled["jmp_imp:SHGetPathFromIDListW"], 1);
    assert_eq!(c.matches("fle_imp_SHGetSpecialFolderLocation(); return;").count(), 1, "only the shell32 thunk is answered");
    assert_eq!(c.matches("fle_imp_SHGetPathFromIDListW(); return;").count(), 1);
    for k in [2u64, 3] {
        let va = IB + 0x1040 + 0x10 * k;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "thunk {k} (user32! / the ANSI form) stays unsupported");
    }
}

#[test]
fn the_other_wintrust_and_crypt32_functions_are_still_not_answered() {
    // Decision 0116 stopped at WinVerifyTrust and kept it unanswered. Decision 0117 answers WinVerifyTrust and seven crypt32 functions (tests/wall_fr4.rs) from a table of
    // claims; the neighbours that nothing needs are still unanswered, with no row: a thunk of one of them stays `fle_unsupported`.
    let (st, c) = gen(&[("wintrust.dll", "WinVerifyTrustEx"), ("crypt32.dll", "CertOpenStore"), ("crypt32.dll", "CryptQueryObjectX"), ("kernel32.dll", "GetProcAddress")]);
    assert!(!st.handled.keys().any(|k| k.contains("WinVerifyTrustEx") || k.contains("CertOpenStore") || k.contains("CryptQueryObjectX")), "{:?}", st.handled);
    let rows = resp_rows(&c);
    assert!(!rows.iter().any(|(_, n, _)| n == "WinVerifyTrustEx" || n == "CertOpenStore"), "no row for them");
    for k in 0..3u64 {
        let va = IB + 0x1040 + 0x10 * k;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "thunk {k}");
    }
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
#include "fle_env.h"
#include "fle_ole.h"

static uint64_t stk[16];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define N_KERNEL32 9
#define N_SHELL32 16

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

static void show(const uint16_t *p, uint64_t buf_len)
{
    uint64_t i;
    for (i = 0; i < buf_len && p[i]; i++) putchar((int)p[i]);
}

static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n)); fle_imp_GetProcAddress();
    r = R[0];
    printf("%s %llu\n", r ? "found" : "missing", U(le()));
    return r;
}

/* SHGetSpecialFolderLocation(hwnd, csidl, &pidl) */
static uint64_t sfl(uint64_t hwnd, uint64_t csidl, int null_out, uint64_t *pidl)
{
    *pidl = 0;
    prep6(hwnd, csidl, null_out ? 0 : A(pidl), 0, 0, 0);
    fle_imp_SHGetSpecialFolderLocation();
    return R[0];
}

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    static uint16_t buf[300];
    uint64_t pidl = 0, hr;
    size_t i;
    for (i = 0; i < 300; i++) buf[i] = 0xAAAA;
    if (!strcmp(s, "lookup")) {
        uint64_t a = gpa(M(N_SHELL32), "SHGetSpecialFolderLocation"), b = gpa(M(N_SHELL32), "SHGetPathFromIDListW");
        gpa(M(N_SHELL32), "SHGetPathFromIDListA");           /* no row */
        gpa(M(N_SHELL32), "shgetspecialfolderlocation");     /* names are compared with exact case */
        gpa(M(N_KERNEL32), "SHGetSpecialFolderLocation");    /* the rows are under shell32.dll only */
        printf("distinct %d both-pseudo %d\n", a != b, a >= 0x7FF900000000ull && b >= 0x7FF900000000ull);
    } else if (!strcmp(s, "sf")) {          /* csidl : location, path, free, check */
        uint64_t live0 = fle_ole_task_live(), ok;
        hr = sfl(0, strtoull(argv[2], 0, 0), 0, &pidl);
        printf("hr %llx le %llu live+%llu\n", U(hr), U(le()), U(fle_ole_task_live() - live0));
        printf("block cb %llu csidl %llx marker %llx term %llu\n", U(fle_ld(pidl, 2)), U(fle_ld(pidl + 2, 4)), U(fle_ld(pidl + 6, 4)), U(fle_ld(pidl + 10, 2)));
        prep(pidl, A(buf));
        fle_imp_SHGetPathFromIDListW();
        ok = R[0];
        printf("ret %llu le %llu [", U(ok), U(le()));
        show(buf, 300);
        for (i = 0; i < 300 && buf[i]; i++) {}
        printf("] len %zu nul %x next %04x\n", i, buf[i], buf[i + 1]);
        prep(pidl, 0); fle_imp_CoTaskMemFree();
        printf("freed live+%llu\n", U(fle_ole_task_live() - live0));
    } else if (!strcmp(s, "sf-via-dispatch")) {     /* the guest reaches the functions through the pseudo-VAs of GetProcAddress */
        uint64_t va1 = gpa(M(N_SHELL32), "SHGetSpecialFolderLocation"), va2 = gpa(M(N_SHELL32), "SHGetPathFromIDListW");
        prep6(0, 0x1a, A(&pidl), 0, 0, 0);
        fle_dispatch(va1);
        printf("hr %llx\n", U(R[0]));
        prep(pidl, A(buf));
        fle_dispatch(va2);
        printf("ret %llu [", U(R[0]));
        show(buf, 300);
        printf("]\n");
    } else if (!strcmp(s, "sf-two-pidls")) {        /* two PIDLs are two blocks; freeing one keeps the other valid */
        uint64_t p1, p2;
        sfl(0, 0x1a, 0, &p1);
        sfl(0, 0x14, 0, &p2);
        printf("distinct %d\n", p1 != p2);
        prep(p1, 0); fle_imp_CoTaskMemFree();
        prep(p2, A(buf)); fle_imp_SHGetPathFromIDListW();
        printf("ret %llu [", U(R[0]));
        show(buf, 300);
        printf("]\n");
    } else if (!strcmp(s, "sf-stale")) {            /* a freed PIDL is no longer an ITEMIDLIST of this runtime */
        sfl(0, 0x1a, 0, &pidl);
        prep(pidl, 0); fle_imp_CoTaskMemFree();
        prep(pidl, A(buf)); fle_imp_SHGetPathFromIDListW();
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "sf-raw")) {              /* SHGetSpecialFolderLocation args: hwnd csidl null|out */
        hr = sfl(strtoull(argv[2], 0, 0), strtoull(argv[3], 0, 0), !strcmp(argv[4], "null"), &pidl);
        printf("returned %llx\n", U(hr));
    } else if (!strcmp(s, "path-raw")) {            /* SHGetPathFromIDListW args: pidl-kind buf-kind */
        static uint8_t forged[12] = { 10, 0, 0x1a, 0, 0, 0, 'P', 'I', 'D', 'L', 0, 0 };
        uint64_t p = !strcmp(argv[2], "null") ? 0 : !strcmp(argv[2], "stack") ? A(forged) : !strcmp(argv[2], "task12") ? 0 : 0;
        if (!strcmp(argv[2], "task12")) {
            prep(12, 0); fle_imp_CoTaskMemAlloc(); p = R[0];          /* a live task block of the right size, zero filled */
        } else if (!strcmp(argv[2], "task16")) {
            prep(16, 0); fle_imp_CoTaskMemAlloc(); p = R[0];
        } else if (!strcmp(argv[2], "good")) {
            sfl(0, 0x1a, 0, &p);
        }
        prep(p, !strcmp(argv[3], "null") ? 0 : A(buf));
        fle_imp_SHGetPathFromIDListW();
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "cpinfo")) {      /* code page, buffer|null */
        static uint8_t info[24];
        int nul = !strcmp(argv[3], "null");
        memset(info, 0xCC, sizeof info);
        prep(strtoull(argv[2], 0, 0), nul ? 0 : A(info));
        fle_imp_GetCPInfo();
        printf("ret %llu le %llu max %u def %02x %02x lead", U(R[0]), U(le()), (unsigned)(info[0] | info[1] << 8 | info[2] << 16 | (unsigned)info[3] << 24), info[4], info[5]);
        for (i = 6; i < 18; i++) printf(" %02x", info[i]);
        printf(" after %02x %02x\n", info[20], info[21]);
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
        let tag = etxt_common::per_process("wall-fr3-driver");
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
    assert!(!o.stdout.contains("returned"), "{args:?}: returned instead of stopping");
    o
}

const APPDATA: &str = "C:\\Users\\user\\AppData\\Roaming";
const FONTS: &str = "C:\\Windows\\Fonts";

#[test]
fn the_two_functions_are_found_through_get_proc_address_under_shell32_only() {
    let got = lines(&["lookup"]);
    assert_eq!(got, ["found 5", "found 5", "missing 127", "missing 127", "missing 127", "distinct 1 both-pseudo 1"]);
}

#[test]
fn appdata_pidl_is_an_s_ok_block_that_converts_to_the_roaming_app_data_path_and_is_freed_by_co_task_mem_free() {
    // Learn: SHGetSpecialFolderLocation "If this function succeeds, it returns S_OK" and the caller frees the IDList with CoTaskMemFree; SHGetPathFromIDListW
    // "Returns TRUE if successful" with the file system path in a buffer of at least MAX_PATH characters. The path is the one SHGetFolderPathW gives for the same csidl.
    // Neither function touches the last error (the driver sets it to 5 before each call). The PIDL is a 12-byte task block: SHITEMID cb 10, abID = csidl + "PIDL", cb 0.
    let want = format!("ret 1 le 5 [{APPDATA}] len {} nul 0 next aaaa", APPDATA.len());
    assert_eq!(APPDATA.len(), 29);
    assert_eq!(
        lines(&["sf", "0x1a"]),
        ["hr 0 le 5 live+1", "block cb 10 csidl 1a marker 4c444950 term 0", &want, "freed live+0"]
    );
    let want = format!("ret 1 le 5 [{FONTS}] len {} nul 0 next aaaa", FONTS.len());
    assert_eq!(lines(&["sf", "0x14"]), ["hr 0 le 5 live+1", "block cb 10 csidl 14 marker 4c444950 term 0", &want, "freed live+0"]);
    // decision 0132 (wall 17): CSIDL_SYSTEM 0x25, Learn (CSIDL): "The Windows System folder. A typical path is C:\Windows\System32."; the same block shape, no trailing backslash
    let want = "ret 1 le 5 [C:\\Windows\\System32] len 19 nul 0 next aaaa";
    assert_eq!(lines(&["sf", "0x25"]), ["hr 0 le 5 live+1", "block cb 10 csidl 25 marker 4c444950 term 0", want, "freed live+0"]);
}

#[test]
fn the_functions_work_through_the_pseudo_vas_as_the_guest_calls_them() {
    let got = lines(&["sf-via-dispatch"]);
    assert_eq!(got, ["found 5", "found 5", "hr 0", &format!("ret 1 [{APPDATA}]")]);
}

#[test]
fn two_pidls_are_two_blocks_and_freeing_one_leaves_the_other_valid() {
    let got = lines(&["sf-two-pidls"]);
    assert_eq!(got, ["distinct 1", &format!("ret 1 [{FONTS}]")]);
}

#[test]
fn the_arguments_the_pages_leave_open_and_the_other_csidl_values_stop_by_name() {
    let name = "SHGetSpecialFolderLocation";
    // csidl: only the two of SHGetFolderPathW (CSIDL_FONTS 0x14, CSIDL_APPDATA 0x1a); CSIDL_PERSONAL 0x5, the flag bits and CSIDL_DESKTOP 0 are not answered
    aborts(&["sf-raw", "0", "0x5", "out"], &format!("{name}: csidl 0x5 is not answered (only CSIDL_FONTS 0x14, CSIDL_APPDATA 0x1a and CSIDL_SYSTEM 0x25 are)"));
    aborts(&["sf-raw", "0", "0", "out"], &format!("{name}: csidl 0 is not answered"));
    aborts(&["sf-raw", "0", "0x801a", "out"], &format!("{name}: csidl 0x801a is not answered"));
    aborts(&["sf-raw", "0", "0x24", "out"], &format!("{name}: csidl 0x24 is not answered"));
    // hwnd is "Reserved" on the page: a value other than NULL is not modelled; a NULL ppidl is not defined
    aborts(&["sf-raw", "0x1234", "0x1a", "out"], &format!("{name}: hwnd is not NULL"));
    aborts(&["sf-raw", "0", "0x1a", "null"], &format!("{name}: ppidl is NULL"));
}

#[test]
fn only_a_live_pidl_of_this_runtime_converts_and_everything_else_stops_by_name() {
    let name = "SHGetPathFromIDListW";
    let not_mine = format!("{name}: pidl is not an ITEMIDLIST that SHGetSpecialFolderLocation of this runtime issued");
    aborts(&["sf-stale"], &not_mine);                     // freed with CoTaskMemFree
    aborts(&["path-raw", "stack", "buf"], &not_mine);     // a pointer that is not a task block, even with the right bytes
    aborts(&["path-raw", "task12", "buf"], &not_mine);    // a live task block of 12 bytes with the wrong content
    aborts(&["path-raw", "task16", "buf"], &not_mine);    // a live task block of another size
    aborts(&["path-raw", "null", "buf"], &format!("{name}: pidl is NULL"));
    aborts(&["path-raw", "good", "null"], &format!("{name}: pszPath is NULL"));
}

#[test]
fn get_cp_info_answers_utf8_with_a_max_char_size_of_4_a_question_mark_and_no_lead_bytes() {
    // Learn (CPINFO): LeadByte "is not even provided for many multi-byte encodings, for example, UTF-8 and GB18030" (all 0 here); DefaultChar "usually the '?'
    // character"; MaxCharSize "a value larger than 2 for other character set types" - 4 is the choice (RFC 3629: at most 4 bytes per scalar value). Exactly the
    // 20 bytes of sizeof(CPINFO) are written, 18 of data and 2 of padding (the driver's buffer holds 0xCC after them). The last error is not touched (the driver sets 5).
    let got = lines(&["cpinfo", "65001", "buf"]);
    assert_eq!(got, ["ret 1 le 5 max 4 def 3f 00 lead 00 00 00 00 00 00 00 00 00 00 00 00 after cc cc"]);
    // CP_UTF8 given through its other spellings is the same number; a NULL pointer fails with ERROR_INVALID_PARAMETER (87), as for the old cases
    assert_eq!(lines(&["cpinfo", "0xfde9", "buf"]), got);
    let got = lines(&["cpinfo", "65001", "null"]);
    assert_eq!(got[0].split(' ').take(4).collect::<Vec<_>>(), ["ret", "0", "le", "87"]);
}

#[test]
fn get_cp_info_keeps_its_old_answers_and_stops_for_the_other_code_pages() {
    // the emulated ANSI and OEM code pages (1252 and 437) are single-byte as before
    assert_eq!(lines(&["cpinfo", "1252", "buf"]), ["ret 1 le 5 max 1 def 3f 00 lead 00 00 00 00 00 00 00 00 00 00 00 00 after cc cc"]);
    assert_eq!(lines(&["cpinfo", "437", "buf"]), ["ret 1 le 5 max 1 def 3f 00 lead 00 00 00 00 00 00 00 00 00 00 00 00 after cc cc"]);
    // anything else (Shift-JIS 932, UTF-7 65000, GB18030 54936, Latin-2 1250) still stops by name
    for cp in ["932", "65000", "54936", "1250"] {
        aborts(&["cpinfo", cp, "buf"], "GetCPInfo: code page other than the emulated ANSI and OEM code pages");
    }
}

#[test]
fn the_csidl_constants_and_virtual_paths_of_the_new_functions_are_those_of_sh_get_folder_path_w() {
    let ole = std::fs::read_to_string(rt_dir().join("fle_ole.c")).unwrap();
    let env = std::fs::read_to_string(rt_dir().join("fle_env.c")).unwrap();
    // fle_ole.c repeats the two csidl numbers and the two sub-paths (fle_env.c is linked without it by other tests): the text must agree
    assert!(env.contains("#define FLE_CSIDL_FONTS 0x0014") && ole.contains("#define FLE_SPECIAL_CSIDL_FONTS   0x0014u"));
    assert!(env.contains("#define FLE_CSIDL_APPDATA 0x001a") && ole.contains("#define FLE_SPECIAL_CSIDL_APPDATA 0x001au"));
    assert!(env.contains("#define FLE_FONTS_SUBDIR \"\\\\Fonts\"") && ole.contains("FLE_VIRTUAL_WINDOWS_DIR \"\\\\Fonts\""));
    assert!(env.contains("#define FLE_APPDATA_SUBDIR \"\\\\AppData\\\\Roaming\"") && ole.contains("FLE_VIRTUAL_USER_PROFILE_DIR \"\\\\AppData\\\\Roaming\""));
    // and SHGetFolderPathW itself still answers exactly the same paths (the existing tests of decision 0107 pin the text; this pins the pair)
    assert!(env.contains("FLE_VIRTUAL_WINDOWS_DIR FLE_FONTS_SUBDIR") && env.contains("FLE_VIRTUAL_USER_PROFILE_DIR FLE_APPDATA_SUBDIR"));
}

// ---------------------------------------------------------------------------------------------
// 3. The harness
// ---------------------------------------------------------------------------------------------

fn compile_and_run_reg_seed_selftest() -> (bool, String) {
    let dir = work_dir();
    let tag = etxt_common::uniq("reg-seed-selftest");
    let exe = dir.join(&tag);
    let rt = rt_dir();
    let h = harness_dir();
    let mut cmd = Command::new("clang");
    cmd.args(["-std=gnu11", "-O0", "-Wall", "-Wextra", "-Werror", "-fPIE", "-I"]).arg(&rt).arg("-I").arg(&h);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c"] {
        cmd.arg(rt.join(f));
    }
    let cc = cmd.arg(h.join("seeds/reg_seed_selftest.c")).arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "reg_seed_selftest does not compile warning-free: {}", String::from_utf8_lossy(&cc.stderr));
    let work = dir.join(format!("{tag}-dir"));
    std::fs::create_dir_all(&work).unwrap();
    let r = Command::new(&exe).arg(&work).output().expect("run failed");
    (r.status.success(), String::from_utf8_lossy(&r.stdout).to_string())
}

#[test]
fn the_registry_seed_self_test_passes_with_every_check_and_no_failure() {
    // 69 checks at the time of writing; the self-test itself exits with the number of failures. Link set = the one of the tests of the virtual registry plus
    // nothing of the lifted code (the harness header calls only the guest-facing stubs of fle_reg.c).
    let (ok, out) = compile_and_run_reg_seed_selftest();
    assert!(ok, "the self-test failed:\n{out}");
    assert!(!out.contains("FAIL"), "{out}");
    assert!(out.lines().filter(|l| l.starts_with("ok")).count() >= 69, "fewer checks than expected:\n{out}");
    assert!(out.trim_end().ends_with("0 failure(s)"), "{out}");
}

fn script_out(extra_env: &[(&str, &str)], seed: &str) -> (i32, String) {
    // a4-multi.sh refuses a bad REG_SEED before it builds anything: give it a readable module file (never lifted here) and a throw-away OUT
    let dir = work_dir();
    let out = dir.join(etxt_common::uniq("a4m-out"));
    let module = format!("m.dll:{}:0x2000000", manifest().join("Cargo.toml").display());
    let mut cmd = Command::new("bash");
    cmd.arg(harness_dir().join("a4-multi.sh")).arg(&module).env("OUT", &out).env("FLE", manifest().join("Cargo.toml")).env("FEBIN", "/nonexistent").env("TMPDIR", dir.join("tmp")).env_remove("FLE_CALL_FRUITY");
    if !seed.is_empty() {
        cmd.env("REG_SEED", seed);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let r = cmd.output().expect("bash failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn a4_multi_refuses_reg_seed_without_the_fruity_build_or_with_a_bad_file() {
    let seed = harness_dir().join("seeds/reg-seed-app.tsv");
    let seed = seed.canonicalize().unwrap();
    let s = seed.to_str().unwrap();
    // only the FRUITY build contains the seeding: any other build is refused (exit 2) with the reason
    let (code, err) = script_out(&[], s);
    assert_eq!(code, 2);
    assert!(err.contains("REG_SEED is set without FLE_CALL_FRUITY=1"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "0")], s);
    assert_eq!(code, 2);
    assert!(err.contains("REG_SEED is set without FLE_CALL_FRUITY=1"), "{err}");
    // with the FRUITY build: a relative path, an unreadable file, a quote or a space are refused
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], "relative/reg.tsv");
    assert_eq!(code, 2);
    assert!(err.contains("every file must be an absolute path"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], "/nonexistent/reg.tsv");
    assert_eq!(code, 2);
    assert!(err.contains("cannot read REG_SEED file"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], &format!("{s}:/nonexistent/second.tsv"));
    assert_eq!(code, 2);
    assert!(err.contains("cannot read REG_SEED file /nonexistent/second.tsv"), "{err}");
    let (code, err) = script_out(&[("FLE_CALL_FRUITY", "1")], &format!("{s} x"));
    assert_eq!(code, 2);
    assert!(err.contains("REG_SEED holds a quote, a space"), "{err}");
}

#[test]
fn a_caller_fle_reg_seed_does_not_seed_a_run_that_did_not_ask_for_it() {
    // the script unsets FLE_REG_SEED unless REG_SEED asks for it, as it does for FLE_VFS_SEED (decision 0110)
    let t = std::fs::read_to_string(harness_dir().join("a4-multi.sh")).unwrap();
    assert!(t.contains("unset FLE_REG_SEED\nREG_SEED=${REG_SEED:-}"), "a4-multi.sh must unset the caller's FLE_REG_SEED before it reads REG_SEED");
    assert!(t.contains("export FLE_REG_SEED=\"$REG_SEED\""));
}

fn preprocess(defines: &[&str]) -> String {
    let dir = work_dir();
    let mut cmd = Command::new("clang");
    cmd.args(["-E", "-P", "-std=gnu11", "-I"]).arg(rt_dir()).arg("-I").arg(harness_dir());
    for d in defines {
        cmd.arg(d);
    }
    let r = cmd.arg(harness_dir().join("two/main_two.c")).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(r.status.success(), "clang -E failed: {}", String::from_utf8_lossy(&r.stderr));
    String::from_utf8_lossy(&r.stdout).to_string()
}

#[test]
fn a_build_without_fle_fruity_contains_no_registry_seeding_and_the_fruity_build_calls_it_once() {
    // the native binary of the default build must stay what it was: the harness main is compiled without -DFLE_FRUITY there
    let off = preprocess(&[]);
    assert!(!off.contains("reg_seed") && !off.contains("rs_run") && !off.contains("FLE_REG_SEED"), "the default build must not contain the registry seeding");
    assert!(off.contains("vfs_seed_from_env"), "sanity: the preprocessed text is main_two.c");
    let on = preprocess(&["-DFLE_FRUITY=1"]);
    assert_eq!(on.matches("reg_seed_from_env() != 0").count(), 1, "one call, checked");
    assert!(on.contains("FLE_REG_SEED"));
    // the call comes after the file seeding and before the fruity precheck, i.e. before the guest runs
    let (v, r, f) = (on.find("vfs_seed_from_env() != 0").unwrap(), on.find("reg_seed_from_env() != 0").unwrap(), on.find("fruity_precheck() != 0").unwrap());
    assert!(v < r && r < f, "order: vfs seed {v}, reg seed {r}, fruity precheck {f}");
}

#[test]
fn the_two_seed_files_say_where_their_values_come_from() {
    // reg-seed-app.tsv is NOT from the installer (the installer never writes "Shared data"): its header says so, and it holds exactly one value
    let t = std::fs::read_to_string(harness_dir().join("seeds/reg-seed-app.tsv")).unwrap();
    assert!(t.lines().next().unwrap().starts_with("# NOT FROM THE INSTALLER"), "{t}");
    let data: Vec<&str> = t.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    assert_eq!(data, ["HKCU\\Software\\Image-Line\\Shared\\Paths\tShared data\tREG_SZ\tC:\\Users\\user\\Documents\\Image-Line\\"]);
    // reg-seed-nsis.tsv is the installer's: the NSIS script (read with tools/nsis_script_dump.py) writes "FL Studio", "FL Studio engine" under HKLM and HKCU
    // Software\Image-Line\Shared\Paths and "Install path" under HKCU; it does not write "Shared data", so the file has no such line
    let t = std::fs::read_to_string(harness_dir().join("seeds/reg-seed-nsis.tsv")).unwrap();
    assert!(t.lines().next().unwrap().starts_with("# FROM THE INSTALLER"), "{t}");
    let data: Vec<Vec<&str>> = t.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split('\t').collect()).collect();
    assert_eq!(data.len(), 5);
    for f in &data {
        assert_eq!(f.len(), 4);
        assert_eq!(f[2], "REG_SZ");
        assert!(f[0].ends_with("\\Software\\Image-Line\\Shared\\Paths") || f[0].ends_with("Software\\Image-Line\\Shared\\Paths"), "{f:?}");
        assert!(f[3].starts_with("C:\\Program Files\\Image-Line\\FL Studio 21"), "{f:?}");
        assert!(!f[1].eq_ignore_ascii_case("Shared data"), "the installer never writes it");
    }
    let names: Vec<(&str, &str)> = data.iter().map(|f| (&f[0][..4], f[1])).collect();
    assert_eq!(names, [("HKLM", "FL Studio"), ("HKLM", "FL Studio engine"), ("HKCU", "FL Studio"), ("HKCU", "FL Studio engine"), ("HKCU", "Install path")]);
    // the key is the one FLEngine reads: UTF-16 "Software\Image-Line\Shared\Paths" and "Shared data" (reading of f_652e60), when the DLL is available
    if let Some(dll) = real_dll() {
        for needle in ["Software\\Image-Line\\Shared\\Paths", "Shared data"] {
            let w: Vec<u8> = needle.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            assert!(count(&dll, &w) >= 1, "FLEngine has no UTF-16 {needle:?}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 4. The real DLL
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    std::fs::read(p).ok()
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

fn va_to_off(dll: &[u8], va: u64) -> Option<usize> {
    // PE section walk (ImageBase 0x400000)
    let lf = u32::from_le_bytes(dll[0x3c..0x40].try_into().ok()?) as usize;
    let nsec = u16::from_le_bytes(dll[lf + 6..lf + 8].try_into().ok()?) as usize;
    let optsz = u16::from_le_bytes(dll[lf + 20..lf + 22].try_into().ok()?) as usize;
    let rva = va.checked_sub(0x40_0000)? as u32;
    for k in 0..nsec {
        let s = lf + 24 + optsz + 40 * k;
        let vsz = u32::from_le_bytes(dll[s + 8..s + 12].try_into().ok()?);
        let vrva = u32::from_le_bytes(dll[s + 12..s + 16].try_into().ok()?);
        let raw = u32::from_le_bytes(dll[s + 20..s + 24].try_into().ok()?);
        if rva >= vrva && rva < vrva + vsz {
            return Some((rva - vrva + raw) as usize);
        }
    }
    None
}

fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let o = va_to_off(dll, va).unwrap_or_else(|| panic!("va {va:#x} not in a section"));
    dll[o..o + n].to_vec()
}

#[test]
fn the_real_dll_reads_shared_data_from_hkcu_paths_and_calls_the_shell_pair_in_one_helper() {
    let Some(dll) = real_dll() else { return };
    // f_652e60 loads the UTF-16 strings of the key and the value name (0x652eb2: lea rdx,[rip+0xcf] -> 0x652f88; 0x652ed3: lea r9,[rip+0xfe] -> 0x652fd8)
    let key: Vec<u8> = "Software\\Image-Line\\Shared\\Paths\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let val: Vec<u8> = "Shared data\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(bytes_at(&dll, 0x652f88, key.len()), key);
    assert_eq!(bytes_at(&dll, 0x652fd8, val.len()), val);
    assert_eq!(bytes_at(&dll, 0x652eb2, 7), [0x48, 0x8d, 0x15, 0xcf, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x652ed3, 7), [0x4c, 0x8d, 0x0d, 0xfe, 0x00, 0x00, 0x00]);
    // f_63ab90: xor ecx,ecx; mov edx,esi; call 0x593370 (SHGetSpecialFolderLocation); test eax,0x80000000; jne; ... call 0x593350 (SHGetPathFromIDListW)
    assert_eq!(bytes_at(&dll, 0x63abaa, 2), [0x33, 0xc9]);
    assert_eq!(bytes_at(&dll, 0x63abac, 2), [0x89, 0xf2]);
    assert_eq!(bytes_at(&dll, 0x63abb2, 1), [0xe8]);
    assert_eq!(bytes_at(&dll, 0x63abb7, 5), [0xa9, 0x00, 0x00, 0x00, 0x80]);
    // the two thunks are `jmp [rip+disp]` (ff 25) through the IAT slots 0x1971f38 and 0x1971f50
    assert_eq!(bytes_at(&dll, 0x593370, 2), [0xff, 0x25]);
    assert_eq!(bytes_at(&dll, 0x593350, 2), [0xff, 0x25]);
}

#[test]
fn the_real_dll_asks_for_utf8_code_page_information_and_verifies_its_own_signature_with_a_zero_test() {
    let Some(dll) = real_dll() else { return };
    // f_452960 reaches GetCPInfo (thunk 0x423400) with the code page 0xfde9 (65001) that the stop report showed in rcx
    assert_eq!(bytes_at(&dll, 0x423400, 2), [0xff, 0x25]);
    // 0x1375bce: call WinVerifyTrust (thunk 0x4256d0); 0x1375bd3: test eax,eax; sete al; the second call (state action 2) at 0x1375bf8; 0x1375bfd: cmp byte [rbp+0x3b7],0
    assert_eq!(bytes_at(&dll, 0x1375bd3, 4), [0x85, 0xc0, 0x0f, 0x94]);
    assert_eq!(bytes_at(&dll, 0x4256d0, 2), [0xff, 0x25]);
    // a non-zero result leads to 0x1375ed4 and from there to the return value -2 (mov dword [rbp+0x5dc], 0xfffffffe at 0x1375eeb)
    assert_eq!(bytes_at(&dll, 0x1375eeb, 10), [0xc7, 0x85, 0xdc, 0x05, 0x00, 0x00, 0xfe, 0xff, 0xff, 0xff]);
}

#[allow(dead_code)]
fn _unused(_: &[u8]) {
    let _ = work_dir;
}
