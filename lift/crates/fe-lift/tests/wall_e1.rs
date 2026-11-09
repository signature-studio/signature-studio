//! Tests of decision 0112 (docs/decisions/0112-fle-env-claims.md): the machine-name wall of the census (units 830 and 880: kernel32!GetComputerNameExW) and its
//! neighbours: GetComputerNameExW / GetComputerNameExA / GetComputerNameW, advapi32!GetUserNameW, version!GetFileVersionInfoW and VerQueryValueW, and
//! kernel32!LocaleNameToLCID (a delay-loaded import of FLEngine).
//!
//! Expected values are written from decision 0112 and the Microsoft Learn pages named in it (GetComputerNameExW, COMPUTER_NAME_FORMAT, GetComputerNameW,
//! GetUserNameW, GetFileVersionInfoW, VerQueryValueW, LocaleNameToLCID, Computer Names, Naming conventions in Active Directory), not read back from the C code.
//! EVERY ANSWER IS PROVISIONAL: the host is Linux, nothing is compared with a real Windows, and no name, file, registry value or locale of the host is read. The
//! computer name and the user name are choices of this environment with no source (the decision lists them).
//!
//! 1. Generator: the seven new response-table rows are appended in order after the older rows, and the four FLEngine thunks that have a row are answered under
//!    their own DLL.
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE (the pseudo-VAs of GetProcAddress are called through
//!    fle_dispatch, as the guest does).
//! 3. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent).
//!
//! NOT RUN when this file was written (stage 1 of the work: no compiler, no cargo). Stage 2 compiles and runs it.

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
    let d = manifest().join("../../out-wallE1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/wall_s.rs and wall_r2.rs)
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

/// The seven rows of decision 0112 in the order of the decision (appended after the older rows, decision 0015). The last two are imported by no module at HEAD
/// and are kept last so that they can be dropped without moving the others.
const E1_ROWS: [(&str, &str); 7] = [
    ("kernel32.dll", "GetComputerNameExW"),
    ("kernel32.dll", "GetComputerNameW"),
    ("version.dll", "GetFileVersionInfoW"),
    ("version.dll", "VerQueryValueW"),
    ("kernel32.dll", "LocaleNameToLCID"),
    ("advapi32.dll", "GetUserNameW"),
    ("kernel32.dll", "GetComputerNameExA"),
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
fn the_seven_rows_are_appended_in_order_after_the_older_rows() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let pos = |dll: &str, name: &str| rows.iter().position(|(d, n, _)| (d.as_str(), n.as_str()) == (dll, name));
    // the block is found by its first row (rows of later decisions may follow it); it is contiguous and in the order of the decision
    let at = pos(E1_ROWS[0].0, E1_ROWS[0].1).expect("the first row of decision 0112");
    let block: Vec<(&str, &str)> = rows[at..at + E1_ROWS.len()].iter().map(|(d, n, _)| (d.as_str(), n.as_str())).collect();
    assert_eq!(block, E1_ROWS);
    // decision 0015: append only. HEAD 0c5000f has 331 rows, the last one gdi32!RealizePalette (decision 0108); the block comes after all of them, and after the
    // GetFileVersionInfoSizeW row of decision 0098 that the two version.dll rows belong with.
    assert!(at >= 331, "the block starts at row {at}");
    assert!(pos("gdi32.dll", "RealizePalette").unwrap() < at);
    assert!(pos("version.dll", "GetFileVersionInfoSizeW").unwrap() < at);
    assert!(pos("shfolder.dll", "SHGetFolderPathW").unwrap() < at);
    // one row per DLL and name
    for (dll, name) in E1_ROWS {
        assert_eq!(rows.iter().filter(|(d, n, _)| (d.as_str(), n.as_str()) == (dll, name)).count(), 1, "{dll}!{name}");
    }
    // the pseudo-VAs rise by 0x10 per row from 0x7FF900000010 (decision 0015)
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    // each row has a prototype and is named once in each of the two tables
    for (_, name) in E1_ROWS {
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name} }},")).count(), 2, "{name}: once in fle_dispatch_table, once in fle_resp_table");
    }
}

#[test]
fn the_four_imported_flengine_thunks_are_answered_under_their_own_dll() {
    // FLEngine imports kernel32!GetComputerNameExW (IAT 0x1971440, thunk 0x423430), kernel32!GetComputerNameW (0x1971790, 0x423420), version!GetFileVersionInfoW
    // (0x1970a00, 0x423df0) and version!VerQueryValueW (0x19709f8, 0x423e10), one thunk each. The DLL name is compared without case; another DLL's
    // GetComputerNameExW is another API.
    let (st, c) = gen(&[
        ("kernel32.dll", "GetComputerNameExW"),
        ("KERNEL32.dll", "GetComputerNameW"),
        ("version.dll", "GetFileVersionInfoW"),
        ("VERSION.DLL", "VerQueryValueW"),
        ("user32.dll", "GetComputerNameExW"),
    ]);
    for h in ["jmp_imp:GetComputerNameExW", "jmp_imp:GetComputerNameW", "jmp_imp:GetFileVersionInfoW", "jmp_imp:VerQueryValueW"] {
        assert_eq!(st.handled[h], 1, "{h}");
    }
    assert_eq!(c.matches("fle_imp_GetComputerNameExW(); return;").count(), 1, "the user32 thunk is not answered");
    let va = IB + 0x1040 + 0x10 * 4;
    assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "the thunk of user32!GetComputerNameExW stays unsupported");
    // the two rows that no module imports are handled when a module does import them, and not under another DLL
    let (st, _) = gen(&[("advapi32.dll", "GetUserNameW"), ("kernel32.dll", "GetComputerNameExA"), ("kernel32.dll", "GetUserNameW"), ("advapi32.dll", "GetComputerNameExA")]);
    assert_eq!(st.handled["jmp_imp:GetUserNameW"], 1);
    assert_eq!(st.handled["jmp_imp:GetComputerNameExA"], 1);
    // a delay-loaded import has no IAT thunk to answer: the row is reached through GetProcAddress (section 2)
    let (st, _) = gen(&[("kernel32.dll", "LocaleNameToLCID")]);
    assert_eq!(st.handled["jmp_imp:LocaleNameToLCID"], 1, "an IAT import of the same name would be answered as well");
}

#[test]
fn the_functions_of_the_rows_are_declared_in_the_runtime_headers() {
    let env = std::fs::read_to_string(rt_dir().join("fle_env.h")).unwrap();
    let vfs = std::fs::read_to_string(rt_dir().join("fle_vfs.h")).unwrap();
    let loc = std::fs::read_to_string(rt_dir().join("fle_locale.h")).unwrap();
    for f in ["GetComputerNameExW", "GetComputerNameExA", "GetComputerNameW", "GetUserNameW"] {
        assert!(env.contains(&format!("void fle_imp_{f}(void);")), "{f}");
    }
    for f in ["GetFileVersionInfoW", "VerQueryValueW"] {
        assert!(vfs.contains(&format!("void fle_imp_{f}(void);")), "{f}");
    }
    assert!(loc.contains("void fle_imp_LocaleNameToLCID(void);"));
    // the definitions are in the existing files the decision names (no new runtime file: tests/modules.rs links every fle_*.c)
    for (file, f) in [
        ("fle_env.c", "GetComputerNameExW"),
        ("fle_env.c", "GetComputerNameExA"),
        ("fle_env.c", "GetComputerNameW"),
        ("fle_env.c", "GetUserNameW"),
        ("fle_vfs.c", "GetFileVersionInfoW"),
        ("fle_vfs.c", "VerQueryValueW"),
        ("fle_locale.c", "LocaleNameToLCID"),
    ] {
        let t = std::fs::read_to_string(rt_dir().join(file)).unwrap();
        assert!(t.contains(&format!("\nvoid fle_imp_{f}(void)\n{{")), "{file}: {f}");
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
#include "fle_vfs.h"

static uint64_t stk[16];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define N_ADVAPI32 1
#define N_KERNEL32 9
#define N_USER32 19
#define N_VERSION 20

/* Registers and stack as at the entry of an API: rcx, rdx, r8, r9, then the 5th and 6th arguments at [rsp+0x28], [rsp+0x30]. The last error starts at 5. */
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

/* "hex:61,e9" is the UTF-16 string with these units; anything else is ASCII. Always NUL-terminated. */
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

/* The buffers of the "nm" scenario are filled with 0xAA / 0xAAAA before the call, so that "first aaaa" means the call did not write and "next aaaa" means it
 * did not write past the NUL. */
static void show_buf(int w, const uint16_t *b16, const uint8_t *b8)
{
    int i = 0;
    unsigned fill = w ? 0xAAAAu : 0xAAu, first = w ? b16[0] : b8[0], next = 0;
    printf("first %x [", first);
    if (first != fill) {
        for (; i < 39 && (w ? b16[i] : b8[i]); i++) putchar(w ? b16[i] : b8[i]);
        if (i < 39) next = w ? b16[i + 1] : b8[i + 1];
    }
    printf("] next %x\n", next);
}

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    int i;
    if (!strcmp(s, "nm")) {                 /* api(exw|exa|w|un) fmt size|nullsize buf|null */
        const char *api = argv[2];
        int ex = api[0] == 'e', w = strcmp(api, "exa") != 0;
        uint32_t sz = (uint32_t)strtoull(argv[4], 0, 0);
        uint64_t fmt = strtoull(argv[3], 0, 0), buf = 0, ps = !strcmp(argv[4], "nullsize") ? 0 : A(&sz), r, e;
        uint16_t b16[40];
        uint8_t b8[40];
        memset(b16, 0xAA, sizeof b16);
        memset(b8, 0xAA, sizeof b8);
        if (!strcmp(argv[5], "buf")) buf = w ? A(b16) : A(b8);
        if (ex) prep6(fmt, buf, ps, 0, 0, 0); else prep6(buf, ps, 0, 0, 0, 0);
        if (!strcmp(api, "exw")) fle_imp_GetComputerNameExW();
        else if (!strcmp(api, "exa")) fle_imp_GetComputerNameExA();
        else if (!strcmp(api, "w")) fle_imp_GetComputerNameW();
        else fle_imp_GetUserNameW();
        r = R[0]; e = le();
        printf("ret %llu le %llu size %u ", U(r), U(e), sz);
        show_buf(w, b16, b8);
    } else if (!strcmp(s, "guest-cnex")) {  /* size: 0x2000 (25 sites of FLEngine) or 0x3ff (the site 0x1159fc9) */
        /* The shape of the 26 call sites: xor ecx,ecx (ComputerNameNetBIOS), a buffer that already holds the AppData path of SHGetFolderPathW (the same local
         * buffer is reused), the size variable set to the buffer size, no test of the result at 25 of them, then the guest reads the buffer up to its NUL. */
        static uint16_t big[0x2100];
        uint32_t sz = (uint32_t)strtoull(argv[2], 0, 0);
        const char *stale = "C:\\Users\\user\\AppData\\Roaming";
        size_t k;
        for (k = 0; k < 0x2100; k++) big[k] = 0xAAAA;
        for (k = 0; stale[k]; k++) big[k] = (uint8_t)stale[k];
        big[k] = 0;
        prep6(0, A(big), A(&sz), 0, 0, 0);
        fle_imp_GetComputerNameExW();
        printf("ret %llu size %u str [", U(R[0]), sz);
        for (k = 0; k < 0x2000 && big[k]; k++) putchar((int)big[k]);
        printf("] len %zu\n", k);
    } else if (!strcmp(s, "fvi")) {         /* path|null buf|null [mkdir=P] [mkfile=P] */
        uint8_t data[16];
        uint64_t hf;
        memset(data, 0xAA, sizeof data);
        for (i = 4; i < argc; i++) {
            if (!strncmp(argv[i], "mkdir=", 6)) {
                prep(A(wide(argv[i] + 6)), 0); fle_imp_CreateDirectoryW();
                if (R[0] != 1) { printf("mkdir failed\n"); return 2; }
            } else if (!strncmp(argv[i], "mkfile=", 7)) {
                prep6(A(wide(argv[i] + 7)), 0x40000000u, 0, 0, 2, 0x80);
                fle_imp_CreateFileW();
                hf = R[0];
                if (hf == FLE_INVALID_HANDLE_VALUE) { printf("mkfile failed\n"); return 2; }
                prep(hf, 0); fle_imp_CloseHandle();
            }
        }
        /* dwHandle is ignored by the page: a dirty value; dwLen 0x100 */
        prep6(!strcmp(argv[2], "null") ? 0 : A(wide(argv[2])), 0xDEADBEEFu, 0x100, !strcmp(argv[3], "buf") ? A(data) : 0, 0, 0);
        fle_imp_GetFileVersionInfoW();
        {
            uint64_t r = R[0], e = le();
            printf("ret %llu le %llu data %s\n", U(r), U(e), data[0] == 0xAA && data[15] == 0xAA ? "untouched" : "changed");
        }
    } else if (!strcmp(s, "vq")) {          /* block|null */
        uint16_t *sub = wide("\\");
        uint64_t out = 0;
        uint32_t len = 0;
        prep6(!strcmp(argv[2], "null") ? 0 : A(stk), A(sub), A(&out), A(&len), 0, 0);
        fle_imp_VerQueryValueW();
        printf("returned\n");
    } else if (!strcmp(s, "lnl")) {         /* name|null flags */
        uint64_t r, e;
        prep(!strcmp(argv[2], "null") ? 0 : A(wide(argv[2])), strtoull(argv[3], 0, 0));
        fle_imp_LocaleNameToLCID();
        r = R[0]; e = le();
        printf("ret %llx le %llu\n", U(r), U(e));
    } else if (!strcmp(s, "gpa-rows")) {
        gpa(M(N_KERNEL32), "GetComputerNameExW");
        gpa(M(N_KERNEL32), "GetComputerNameExA");
        gpa(M(N_KERNEL32), "GetComputerNameW");
        gpa(M(N_KERNEL32), "LocaleNameToLCID");
        gpa(M(N_ADVAPI32), "GetUserNameW");
        gpa(M(N_VERSION), "GetFileVersionInfoW");
        gpa(M(N_VERSION), "VerQueryValueW");
        gpa(M(N_USER32), "GetComputerNameExW");     /* another DLL: no row */
        gpa(M(N_KERNEL32), "GetUserNameW");         /* the row is under advapi32.dll */
        gpa(M(N_ADVAPI32), "GetComputerNameW");     /* the row is under kernel32.dll */
        gpa(M(N_KERNEL32), "getcomputernamew");     /* names are compared with exact case */
    } else if (!strcmp(s, "lnl-via-va")) {  /* what the delay-load helper of FLEngine ends in: GetProcAddress(kernel32, "LocaleNameToLCID"), then the call */
        uint64_t va = gpa(M(N_KERNEL32), "LocaleNameToLCID");
        callva6(va, A(wide("en-US")), 0, 0, 0, 0, 0);
        printf("ret %llx le %llu\n", U(R[0]), U(le()));
    } else if (!strcmp(s, "cnex-via-va")) { /* the pseudo-VA of the row, called through the dispatch table */
        uint16_t b[24];
        uint32_t sz = 24;
        uint64_t va = gpa(M(N_KERNEL32), "GetComputerNameExW");
        memset(b, 0, sizeof b);
        callva6(va, 0, A(b), A(&sz), 0, 0, 0);
        printf("ret %llu size %u [", U(R[0]), sz);
        for (i = 0; i < 24 && b[i]; i++) putchar((int)b[i]);
        printf("]\n");
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
        let tag = etxt_common::per_process("wall-e1-driver");
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

/// The run must abort through fle_stop (SIGABRT, "fle: <message>" on stderr) or through the locale file's own stop (same shape), with `must_say` in the message.
fn aborts(args: &[&str], must_say: &str) -> Out {
    let o = run(args);
    assert!(!o.ok, "{args:?}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{args:?}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle:"), "{args:?}: no fle: message: {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{args:?}: message lacks {must_say:?}: {}", o.stderr);
    assert!(!o.stdout.contains("returned"), "{args:?}: the call returned: {}", o.stdout);
    o
}

/// The text of `#define NAME "..."` in a runtime file: the first quoted string, as written in the C source (escapes not resolved).
fn def_str(file: &str, name: &str) -> String {
    let t = std::fs::read_to_string(rt_dir().join(file)).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let q1 = line.find('"').unwrap_or_else(|| panic!("{name}: no string"));
    let q2 = q1 + 1 + line[q1 + 1..].find('"').unwrap();
    line[q1 + 1..q2].to_string()
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

// ---- 2.1 The values of the environment (decision 0112: choices without a source) ----

const COMPUTER: &str = "DESKTOP-0000001";

#[test]
fn the_computer_name_is_a_netbios_name_and_the_user_name_is_the_one_of_the_profile_directory() {
    // Learn, Computer Names: "NetBIOS names consist of up to 15 bytes of OEM characters including letters, digits, hyphens, and periods ... by convention,
    // represented in uppercase"; Learn, Naming conventions in Active Directory: "Maximum name length: 15 characters", "Windows uses only uppercase names", "As the
    // NetBIOS computer name is the same as the DNS host name, the DNS host name restrictions also apply" (no period start, hyphens inside, not all digits).
    let name = def_str("fle_env.h", "FLE_VIRTUAL_COMPUTER_NAME");
    assert_eq!(name, COMPUTER);
    assert_eq!(def_const("fle_env.h", "FLE_COMPUTERNAME_NETBIOS_MAX"), 15);
    assert!(!name.is_empty() && name.len() <= 15, "{name}");
    assert!(name.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-'), "{name}");
    assert!(!name.starts_with('-') && !name.ends_with('-'));
    assert!(!name.bytes().all(|b| b.is_ascii_digit()));
    // the shape of the default name of a Windows 10 installation ("DESKTOP-" and seven characters: Microsoft Q&A, not a primary page) is 15 characters
    assert!(name.starts_with("DESKTOP-") && name.len() == 15);
    // the user name is the directory name inside FLE_VIRTUAL_USER_PROFILE_DIR (decision 0107): one place for the name, two constants that must agree
    let user = def_str("fle_env.h", "FLE_VIRTUAL_USER_NAME");
    let profile = def_str("fle_env.h", "FLE_VIRTUAL_USER_PROFILE_DIR").replace("\\\\", "\\");
    assert_eq!(user, "user");
    assert_eq!(profile, "C:\\Users\\user");
    assert!(profile.ends_with(&format!("\\{user}")));
    // the error numbers are those of the System Error Codes page: ERROR_BUFFER_OVERFLOW 111, ERROR_INSUFFICIENT_BUFFER 122, ERROR_MORE_DATA 234
    assert_eq!(def_const("fle_env.h", "FLE_W32_ERR_BUFFER_OVERFLOW"), 111);
    assert_eq!(def_const("fle_env.h", "FLE_W32_ERR_INSUFFICIENT_BUFFER"), 122);
    assert_eq!(def_const("fle_env.h", "FLE_W32_ERR_MORE_DATA"), 234);
}

// ---- 2.2 GetComputerNameExW / GetComputerNameExA ----

#[test]
fn get_computer_name_ex_w_writes_the_name_counts_without_the_nul_and_leaves_the_last_error() {
    // 0x2000 is the size that 25 of the 26 call sites of FLEngine pass; 16 is exactly the name and its NUL; 17 is more than enough.
    for size in ["0x2000", "17", "16"] {
        assert_eq!(lines(&["nm", "exw", "0", size, "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aaaa")], "size {size}");
    }
    // NameType is an enum (an int): the upper half of rcx does not count
    assert_eq!(lines(&["nm", "exw", "0xFFFFFFFF00000000", "0x2000", "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aaaa")]);
    // through the pseudo-VA of the row (GetProcAddress, then the dispatch table, as the guest calls a function it fetched)
    assert_eq!(lines(&["cnex-via-va"]), ["found 5".to_string(), format!("ret 1 size 15 [{COMPUTER}]")]);
}

#[test]
fn the_netbios_and_dns_host_name_formats_and_their_physical_forms_are_one_name() {
    // COMPUTER_NAME_FORMAT 0 NetBIOS, 1 DnsHostname, 4 PhysicalNetBIOS, 5 PhysicalDnsHostname. The Physical forms differ from the others only on a cluster node
    // (GetComputerNameExW page); the NetBIOS name is the OEM form of the DNS host name up to 15 characters (Computer Names page), and the name is upper case and
    // has 15 characters, so the four are one string.
    for fmt in ["0", "1", "4", "5"] {
        assert_eq!(lines(&["nm", "exw", fmt, "0x2000", "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aaaa")], "NameType {fmt}");
        assert_eq!(lines(&["nm", "exa", fmt, "0x2000", "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aa")], "ANSI NameType {fmt}");
    }
}

#[test]
fn a_buffer_that_is_too_small_fails_with_more_data_and_the_needed_size_and_is_not_written() {
    // Learn: "If the buffer is too small, the function fails and GetLastError returns ERROR_MORE_DATA. This parameter receives the size of the buffer required,
    // including the terminating null character." The name has 15 characters, so 16 TCHARs are needed.
    for size in ["15", "1", "0"] {
        assert_eq!(lines(&["nm", "exw", "0", size, "buf"]), ["ret 0 le 234 size 16 first aaaa [] next 0".to_string()], "size {size}");
        assert_eq!(lines(&["nm", "exa", "0", size, "buf"]), ["ret 0 le 234 size 16 first aa [] next 0".to_string()], "ANSI size {size}");
    }
}

#[test]
fn a_null_buffer_with_size_zero_asks_for_the_required_size() {
    // Learn: "set this parameter [lpBuffer] to NULL and use the required buffer size returned in the lpnSize parameter"; "If lpBuffer is NULL, this parameter must be zero."
    assert_eq!(lines(&["nm", "exw", "0", "0", "null"]), ["ret 0 le 234 size 16 first aaaa [] next 0".to_string()]);
    assert_eq!(lines(&["nm", "exa", "1", "0", "null"]), ["ret 0 le 234 size 16 first aa [] next 0".to_string()]);
    // a non-zero size with a NULL buffer breaks the page's rule: stop, small or large
    aborts(&["nm", "exw", "0", "5", "null"], "GetComputerNameExW: lpBuffer is NULL but *nSize is not zero");
    aborts(&["nm", "exw", "0", "0x2000", "null"], "GetComputerNameExW: lpBuffer is NULL but *nSize is not zero");
    aborts(&["nm", "exa", "0", "16", "null"], "GetComputerNameExA: lpBuffer is NULL but *nSize is not zero");
    // a NULL size pointer is not defined by the page
    aborts(&["nm", "exw", "0", "nullsize", "buf"], "GetComputerNameExW: the size pointer is NULL");
    aborts(&["nm", "exa", "0", "nullsize", "null"], "GetComputerNameExA: the size pointer is NULL");
}

#[test]
fn the_formats_without_a_derivable_answer_stop_by_name() {
    // 2 DnsDomain, 3 DnsFullyQualified, 6 PhysicalDnsDomain, 7 PhysicalDnsFullyQualified: the pages do not say what a computer that is not joined to a domain
    // returns. The check comes before the buffer is looked at (a large, valid buffer does not make them answerable).
    for fmt in ["2", "3", "6", "7"] {
        aborts(&["nm", "exw", fmt, "0x2000", "buf"], &format!("GetComputerNameExW: NameType {fmt} (a DNS domain or a fully qualified name)"));
        aborts(&["nm", "exa", fmt, "0x2000", "buf"], &format!("GetComputerNameExA: NameType {fmt} (a DNS domain or a fully qualified name)"));
    }
    // 8 ComputerNameMax is "Not used"; 9 and larger, and negative values (an int) are outside the enumeration
    aborts(&["nm", "exw", "8", "0x2000", "buf"], "GetComputerNameExW: NameType 8 (ComputerNameMax)");
    aborts(&["nm", "exw", "9", "0x2000", "buf"], "GetComputerNameExW: NameType 9 is outside the COMPUTER_NAME_FORMAT enumeration");
    aborts(&["nm", "exw", "0xFFFFFFFF", "0x2000", "buf"], "GetComputerNameExW: NameType 4294967295 is outside");
    // the upper half of rcx is not part of the int: 0x1_0000_0002 is NameType 2
    aborts(&["nm", "exw", "0x100000002", "0x2000", "buf"], "NameType 2 (a DNS domain");
}

#[test]
fn the_call_shape_of_the_26_flengine_sites_reads_the_name_and_not_the_stale_path() {
    // The 26 sites pass NameType 0 and a buffer that the SHGetFolderPathW call just filled with the AppData path (the same local buffer: f_10c7cd0, f_125da90).
    // 25 sites ignore the result and read the buffer up to its NUL: the NUL after the shorter name must cut the longer old text. The site 0x1159fc9 passes 0x3ff
    // and uses the returned count.
    for size in ["0x2000", "0x3ff"] {
        assert_eq!(lines(&["guest-cnex", size]), [format!("ret 1 size 15 str [{COMPUTER}] len 15")], "size {size}");
    }
}

// ---- 2.3 GetComputerNameW and GetUserNameW ----

#[test]
fn get_computer_name_w_answers_the_netbios_name_and_fails_with_buffer_overflow() {
    // The single FLEngine site passes 32 for a buffer of 16 TCHARs (MAX_COMPUTERNAME_LENGTH + 1); 16 is exactly the name and its NUL.
    for size in ["32", "16", "0x2000"] {
        assert_eq!(lines(&["nm", "w", "0", size, "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aaaa")], "size {size}");
    }
    // Learn (GetComputerNameW): "If the buffer is too small, the function fails and GetLastError returns ERROR_BUFFER_OVERFLOW ... specifies the size of the buffer
    // required, including the terminating null character." (111, not ERROR_MORE_DATA)
    for size in ["15", "1", "0"] {
        assert_eq!(lines(&["nm", "w", "0", size, "buf"]), ["ret 0 le 111 size 16 first aaaa [] next 0".to_string()], "size {size}");
    }
    // a zero size needs no buffer to fail; a NULL buffer with a size that would hold the name is not defined by the page
    assert_eq!(lines(&["nm", "w", "0", "0", "null"]), ["ret 0 le 111 size 16 first aaaa [] next 0".to_string()]);
    aborts(&["nm", "w", "0", "16", "null"], "GetComputerNameW: lpBuffer is NULL and the size would hold the name");
    aborts(&["nm", "w", "0", "nullsize", "buf"], "GetComputerNameW: the size pointer is NULL");
}

#[test]
fn get_user_name_w_answers_user_counting_the_nul_and_fails_with_insufficient_buffer() {
    // Learn (GetUserNameW): on output the size is "the number of TCHARs copied to the buffer, including the terminating null character" (5 for "user");
    // "If lpBuffer is too small, the function fails and GetLastError returns ERROR_INSUFFICIENT_BUFFER. This parameter receives the required buffer size,
    // including the terminating null character."
    for size in ["257", "5"] {
        assert_eq!(lines(&["nm", "un", "0", size, "buf"]), ["ret 1 le 5 size 5 first 75 [user] next aaaa".to_string()], "size {size}");
    }
    for size in ["4", "1", "0"] {
        assert_eq!(lines(&["nm", "un", "0", size, "buf"]), ["ret 0 le 122 size 5 first aaaa [] next 0".to_string()], "size {size}");
    }
    assert_eq!(lines(&["nm", "un", "0", "0", "null"]), ["ret 0 le 122 size 5 first aaaa [] next 0".to_string()]);
    aborts(&["nm", "un", "0", "5", "null"], "GetUserNameW: lpBuffer is NULL and the size would hold the name");
    aborts(&["nm", "un", "0", "nullsize", "buf"], "GetUserNameW: the size pointer is NULL");
}

#[test]
fn the_ansi_form_writes_bytes_and_not_wide_units() {
    // GetComputerNameExA: 8-bit characters, so the byte after the NUL is not written ("next aa") and the units of the W form are not touched.
    assert_eq!(lines(&["nm", "exa", "0", "16", "buf"]), [format!("ret 1 le 5 size 15 first 44 [{COMPUTER}] next aa")]);
    assert_eq!(lines(&["nm", "exa", "0", "15", "buf"]), ["ret 0 le 234 size 16 first aa [] next 0".to_string()]);
}

#[test]
fn the_rows_are_found_by_get_proc_address_under_their_own_dll_only() {
    assert_eq!(
        lines(&["gpa-rows"]),
        [
            "found 5", "found 5", "found 5", "found 5", "found 5", "found 5", "found 5", // the seven rows under kernel32.dll, advapi32.dll, version.dll
            "missing 127", "missing 127", "missing 127", "missing 127", // user32.dll, kernel32!GetUserNameW, advapi32!GetComputerNameW, wrong case
        ]
    );
}

// ---- 2.4 The version-resource calls ----

#[test]
fn get_file_version_info_w_fails_like_the_size_call_for_a_path_that_is_not_in_the_volume() {
    // Learn (GetFileVersionInfoW): "If the function fails, the return value is zero. To get extended error information, call GetLastError" (no code named);
    // dwHandle "is ignored". The failure is that of GetFileVersionInfoSizeW (decision 0098) for the same path: the file-system error of the path, 2 when the
    // parent exists and the file does not, 3 when a parent is missing. lpData is not touched; a NULL lpData is not looked at on the failure path.
    assert_eq!(lines(&["fvi", "C:\\d\\nofile.dll", "buf", "mkdir=C:\\d"]), ["ret 0 le 2 data untouched".to_string()]);
    assert_eq!(lines(&["fvi", "C:\\x\\y.dll", "buf", "mkdir=C:\\d"]), ["ret 0 le 3 data untouched".to_string()]);
    assert_eq!(lines(&["fvi", "C:\\d\\nofile.dll", "null", "mkdir=C:\\d"]), ["ret 0 le 2 data untouched".to_string()]);
    // decision 0129 (old -> new): the guest's own module path (GetModuleFileNameW of the lifted DLL: FLE_VIRTUAL_DLL_PATH) was "not a file of the virtual volume" here (old: "ret 0 le 3 data untouched").
    // The PM's decision (A, S-version-info-real-resource) makes it the RT_VERSION resource of the image mapped at 0x400000. This driver maps no image, so the call is not made here: tests/wall_fr12.rs
    // (the_version_info_of_the_engine_path_...) maps a PE and checks size, bytes, truncation and the queries.
}

// decision 0129 (old -> new): the test `the_version_calls_for_the_modules_own_path_end_at_the_size_call` is removed. Its premise (the size call fails for the module's own path, so GetFileVersionInfoW and VerQueryValueW
// are not reached: old answer "ret 0 handle 0 le 3") is reversed by the PM's decision (A): the size call answers the size of the RT_VERSION resource and the guest goes on to the two other calls. The same call with a
// mapped image (size 728, *lpdwHandle 0, the 26.1.7.5653 string) is tests/wall_fr12.rs `the_version_resource_of_the_real_engine_is_728_bytes_and_says_26_1_7_5653`.

#[test]
fn what_get_file_version_info_w_does_not_define_stops_by_name() {
    aborts(&["fvi", "null", "buf"], "GetFileVersionInfoW: lptstrFilename is NULL");
    // f_44ada0 asks for "comctl32.dll": a name with no directory part needs the LoadLibrary search sequence, which is not modelled (the size call stops first there)
    aborts(&["fvi", "comctl32.dll", "buf"], "GetFileVersionInfoW: a file name with no directory part");
    // a path that exists in the virtual file system (a directory or a file): no version resource of a virtual file is modelled
    aborts(&["fvi", "C:\\d\\f.dll", "buf", "mkdir=C:\\d", "mkfile=C:\\d\\f.dll"], "GetFileVersionInfoW: the path exists in the virtual file system");
    aborts(&["fvi", "C:\\d", "buf", "mkdir=C:\\d"], "GetFileVersionInfoW: the path exists in the virtual file system");
}

#[test]
fn ver_query_value_w_stops_for_a_null_argument_and_answers_zero_for_a_block_that_is_not_a_version_resource() {
    // decision 0129 (old -> new). Old: "No GetFileVersionInfoW succeeds here, and a block of the guest's own making is not parsed or called invalid": both a NULL pBlock and a block stopped with
    // "pBlock is not a version-information block of this environment". New: Learn VerQueryValueW "If the specified name does not exist or the specified resource is not valid, the return value is zero":
    // a block that does not start with a VS_VERSIONINFO node called "VS_VERSION_INFO" returns 0 (here: the zero-filled stack of the driver); a NULL pBlock, lpSubBlock, lplpBuffer or puLen still stops (the page
    // does not define it), with the new message. The queries on a real block are tests/wall_fr12.rs.
    aborts(&["vq", "null"], "VerQueryValueW: pBlock, lpSubBlock, lplpBuffer or puLen is NULL");
    assert_eq!(lines(&["vq", "block"]), ["returned".to_string()]);
}

// ---- 2.5 LocaleNameToLCID ----

#[test]
fn locale_name_to_lcid_answers_the_emulated_locale_and_the_predefined_names() {
    // Decision 0016: the emulated locale is en-US, LCID 0x0409, user default = system default. NULL is LOCALE_NAME_USER_DEFAULT, "!x-sys-default-locale" is
    // LOCALE_NAME_SYSTEM_DEFAULT, "" is LOCALE_NAME_INVARIANT (winnls.h; LOCALE_INVARIANT is 0x7F). The last error (5) is not touched.
    for name in ["en-US", "en-us", "EN-US", "null", "!x-sys-default-locale", "!X-SYS-DEFAULT-LOCALE"] {
        assert_eq!(lines(&["lnl", name, "0"]), ["ret 409 le 5".to_string()], "{name}");
    }
    assert_eq!(lines(&["lnl", "", "0"]), ["ret 7f le 5".to_string()]);
    // LOCALE_ALLOW_NEUTRAL_NAMES (0x08000000) is the one flag the page names; edx is a DWORD (the upper half of rdx does not count)
    assert_eq!(lines(&["lnl", "en-US", "0x08000000"]), ["ret 409 le 5".to_string()]);
    assert_eq!(lines(&["lnl", "en-US", "0xFFFFFFFF00000000"]), ["ret 409 le 5".to_string()]);
    // the delay-load helper of FLEngine ends in GetProcAddress(kernel32, "LocaleNameToLCID") and a call through the pseudo-VA
    assert_eq!(lines(&["lnl-via-va"]), ["found 5".to_string(), "ret 409 le 5".to_string()]);
}

#[test]
fn locale_name_to_lcid_stops_for_a_name_or_flag_the_environment_does_not_have() {
    // no table of locale names is given by the page: answering 0 would call a real name invalid, another value would be invented
    aborts(&["lnl", "de-DE", "0"], "LocaleNameToLCID: a locale name that this environment does not have");
    aborts(&["lnl", "en", "0"], "LocaleNameToLCID: a locale name that this environment does not have");
    aborts(&["lnl", "en-US ", "0"], "LocaleNameToLCID: a locale name that this environment does not have");
    aborts(&["lnl", "en-US", "1"], "LocaleNameToLCID: dwFlags is neither 0 nor LOCALE_ALLOW_NEUTRAL_NAMES");
    aborts(&["lnl", "en-US", "0x80000000"], "LocaleNameToLCID: dwFlags is neither 0 nor LOCALE_ALLOW_NEUTRAL_NAMES");
    // LOCALE_NAME_MAX_LENGTH (85) "includes space for a terminating null character": 84 characters is the longest name
    aborts(&["lnl", &"a".repeat(84), "0"], "LocaleNameToLCID: a locale name that this environment does not have");
    aborts(&["lnl", &"a".repeat(85), "0"], "LocaleNameToLCID: a name of LOCALE_NAME_MAX_LENGTH (85) characters or more");
    aborts(&["lnl", "hex:65,6e,2d,e9", "0"], "LocaleNameToLCID: a name with a character above 0x7F");
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

/// The target of the `call rel32` at `site`.
fn call_target(dll: &[u8], site: u64) -> u64 {
    let b = bytes_at(dll, site, 5);
    assert_eq!(b[0], 0xe8, "no call at {site:#x}");
    (site as i64 + 5 + i32::from_le_bytes(b[1..5].try_into().unwrap()) as i64) as u64
}

/// The slot of the `jmp [rip+disp32]` thunk at `thunk`.
fn thunk_slot(dll: &[u8], thunk: u64) -> u64 {
    let b = bytes_at(dll, thunk, 6);
    assert_eq!(&b[..2], &[0xff, 0x25], "no jmp [rip+d] at {thunk:#x}");
    (thunk as i64 + 6 + i32::from_le_bytes(b[2..6].try_into().unwrap()) as i64) as u64
}

#[test]
fn the_real_dll_has_26_computer_name_ex_w_sites_that_ask_for_the_netbios_name() {
    let Some(dll) = real_dll() else { return };
    // the thunk 0x423430 jumps through the IAT slot 0x1971440 (kernel32.dll!GetComputerNameExW by pefile)
    assert_eq!(thunk_slot(&dll, 0x423430), 0x1971440);
    let sites: [u64; 26] = [
        0xbf979a, 0xbfafb2, 0xbfd367, 0xbfe657, 0xf01198, 0xf03ff1, 0xf32e76, 0xf41233, 0xf52c41, 0xffc8b8, 0x103d347, 0x10b4bb1, 0x10c2270, 0x10c89d8, 0x10d214f, 0x1159fc9,
        0x117fafe, 0x11a612d, 0x11c0693, 0x11c12fc, 0x11d992c, 0x11e2b65, 0x11fdbbd, 0x121dc92, 0x125dcec, 0x128815d,
    ];
    let (mut wide, mut small) = (0, 0);
    for site in sites {
        assert_eq!(call_target(&dll, site), 0x423430, "{site:#x}");
        let b = bytes_at(&dll, site - 26, 26);
        let big = b[0..2] == [0xc7, 0x85] && b[6..10] == [0x00, 0x20, 0x00, 0x00] && b[10..12] == [0x33, 0xc9] && b[12..15] == [0x48, 0x8d, 0x95] && b[19..22] == [0x4c, 0x8d, 0x85] && b[2..6] == b[22..26];
        if big {
            // mov dword [rbp+size],0x2000; xor ecx,ecx (NameType 0); lea rdx,[rbp+buf]; lea r8,[rbp+size]; call
            wide += 1;
        } else {
            // the one other site: mov dword [rbp+0x2c],0x3ff; xor ecx,ecx; lea rdx,[rbp+0x30]; lea r8,[rbp+0x2c]; call; test eax,eax
            assert_eq!(site, 0x1159fc9);
            assert_eq!(bytes_at(&dll, site - 17, 17), [0xc7, 0x45, 0x2c, 0xff, 0x03, 0x00, 0x00, 0x33, 0xc9, 0x48, 0x8d, 0x55, 0x30, 0x4c, 0x8d, 0x45, 0x2c]);
            assert_eq!(bytes_at(&dll, site + 5, 2), [0x85, 0xc0]);
            small += 1;
        }
    }
    assert_eq!((wide, small), (25, 1));
    // the unit-880 site and the unit-830 site are two of the 25 (census: the stack of the wall)
    assert!(sites.contains(&0x10c89d8) && sites.contains(&0x125dcec));
    // the strings: the import name of GetComputerNameExW appears once; no module string asks for GetUserNameW or GetComputerNameExA (neither is imported)
    assert_eq!(count(&dll, b"GetComputerNameExW"), 1);
    assert_eq!(count(&dll, b"GetUserName"), 0);
    assert_eq!(count(&dll, b"GetComputerNameExA"), 0);
}

#[test]
fn the_real_dll_get_computer_name_w_site_has_a_16_character_buffer_and_the_size_32() {
    let Some(dll) = real_dll() else { return };
    // the thunk 0x423420 jumps through the IAT slot 0x1971790 (kernel32.dll!GetComputerNameW), one call site at 0x5df888:
    // mov dword [rbp+0x84],0x20; lea rcx,[rbp+0x64]; lea rdx,[rbp+0x84]; call; test eax,eax
    assert_eq!(thunk_slot(&dll, 0x423420), 0x1971790);
    assert_eq!(call_target(&dll, 0x5df888), 0x423420);
    assert_eq!(
        bytes_at(&dll, 0x5df873, 21),
        [0xc7, 0x85, 0x84, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x48, 0x8d, 0x4d, 0x64, 0x48, 0x8d, 0x95, 0x84, 0x00, 0x00, 0x00]
    );
    assert_eq!(bytes_at(&dll, 0x5df88d, 2), [0x85, 0xc0]);
    assert_eq!(count(&dll, b"GetComputerNameW\0"), 1);
}

#[test]
fn the_real_dll_version_sequences_call_the_size_first_and_the_module_path_comes_from_get_module_file_name_w() {
    let Some(dll) = real_dll() else { return };
    // thunks: GetFileVersionInfoSizeW 0x423e00 (slot 0x19709f0, answered by decision 0098), GetFileVersionInfoW 0x423df0 (0x1970a00), VerQueryValueW 0x423e10 (0x19709f8)
    assert_eq!(thunk_slot(&dll, 0x423e00), 0x19709f0);
    assert_eq!(thunk_slot(&dll, 0x423df0), 0x1970a00);
    assert_eq!(thunk_slot(&dll, 0x423e10), 0x19709f8);
    // the three pairs (size site, info site, first query site): f_44ada0, f_651ce0, f_a5fce0; each info call is guarded by `test ebx/eax` after the size call
    for (size, info, query) in [(0x44ade6u64, 0x44ae18u64, 0x44ae34u64), (0x651d2d, 0x651d55, 0x651d7a), (0xa5fd26, 0xa5fd53, 0xa5fd7b)] {
        assert_eq!(call_target(&dll, size), 0x423e00, "{size:#x}");
        assert_eq!(call_target(&dll, info), 0x423df0, "{info:#x}");
        assert_eq!(call_target(&dll, query), 0x423e10, "{query:#x}");
    }
    // f_651ce0 (unit 880): after the size call, `mov ebx,eax; test ebx,ebx; jbe` skips the allocation, GetFileVersionInfoW and VerQueryValueW when the size is 0
    assert_eq!(bytes_at(&dll, 0x651d32, 6), [0x89, 0xc3, 0x85, 0xdb, 0x0f, 0x86]);
    // f_652220 passes GetModuleFileNameW(hInstance, buf, 0x1ff) (thunk 0x4235a0, slot by thunk_slot) to f_6520b0, which calls f_651ce0
    assert_eq!(call_target(&dll, 0x652258), 0x4235a0);
    assert_eq!(bytes_at(&dll, 0x652252, 6), [0x41, 0xb8, 0xff, 0x01, 0x00, 0x00]);
    assert_eq!(call_target(&dll, 0x652284), 0x6520b0);
    assert_eq!(call_target(&dll, 0x6520f8), 0x651ce0);
    assert_eq!(call_target(&dll, 0x125d9e1), 0x652220);
    assert_eq!(call_target(&dll, 0x125db36), 0x125d9b0);
    // f_44ada0 is given the literal "comctl32.dll" (a name with no directory part) by f_837e60 and f_8793f0
    let wide = |s: &str| s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>();
    assert_eq!(bytes_at(&dll, 0x837fc4, 26), wide("comctl32.dll\0"));
    assert_eq!(bytes_at(&dll, 0x879b08, 26), wide("comctl32.dll\0"));
    assert_eq!(call_target(&dll, 0x837e84), 0x44ada0);
    assert_eq!(call_target(&dll, 0x879459), 0x44ada0);
    assert_eq!(count(&dll, b"VerQueryValueW"), 1);
    assert_eq!(count(&dll, b"GetFileVersionInfoW"), 1);
}

#[test]
fn the_real_dll_reaches_locale_name_to_lcid_through_a_delay_thunk_with_flags_zero() {
    let Some(dll) = real_dll() else { return };
    // f_446ad0: `xor edx,edx; call 0x4237d0` at 0x446b24 (dwFlags 0); the thunk is `jmp [rip+d]` through a delay-load slot; the name is a string of the DLL's
    // delay-import table, which the delay-load helper passes to GetProcAddress
    assert_eq!(bytes_at(&dll, 0x446b22, 2), [0x33, 0xd2]);
    assert_eq!(call_target(&dll, 0x446b24), 0x4237d0);
    assert_eq!(bytes_at(&dll, 0x4237d0, 2), [0xff, 0x25]);
    assert_eq!(count(&dll, b"LocaleNameToLCID"), 1);
}
