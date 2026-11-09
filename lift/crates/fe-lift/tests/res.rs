//! Tests of the resource APIs (docs/decisions/0025-resource-apis.md): kernel32!FindResourceW, LoadResource,
//! LockResource, SizeofResource and user32!LoadStringW in runtime/fle_res.c.
//!
//! 1. Generator: the five thunks call the runtime (a synthetic PE that imports them).
//! 2. Runtime on the real FLEngine_x64.dll, mapped by harness/image.c at its ImageBase 0x400000. Expected values
//!    are written from the Microsoft pages named in decision 0025, and from fe-pe's resource parser and an
//!    independent Python parser (lift/out-a2/work/dump_rsrc.py), never read back from fle_res.c.
//!    The DLL path is $FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll. If FLENGINE_PE is set and the
//!    file is missing the tests fail; if only the default path is missing they print SKIPPED and pass (same as
//!    fe-pe's tests/real_dll.rs).
//! 3. Runtime on small synthetic images (no resource directory, several languages, malformed data), mapped the same way.
//! 4. Every input that the pages leave open stops the run (SIGABRT, "fle:" message).
//!
//! The host is Linux; nothing is compared with a real Windows. $FLE_RES_DIR (a test knob) names the directory
//! that holds fle_res.c/.h instead of runtime/ (used before the files were placed in runtime/).

mod etxt_common;

use fe_lift::generate;
use fe_pe::{Pe, ResourceId, RT_STRING};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}

fn res_dir() -> PathBuf {
    std::env::var_os("FLE_RES_DIR").map(PathBuf::from).unwrap_or_else(runtime_dir)
}

fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-a2/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
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

/// A PE with one import descriptor per (dll, name) (at most 5); entry 0x1000 calls the thunks at 0x1040 + 0x10 * k.
fn build_imports(imps: &[(&str, &str)]) -> Vec<u8> {
    assert!(imps.len() <= 5);
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

#[test]
fn the_five_thunks_call_the_runtime() {
    let imps = [
        ("kernel32.dll", "FindResourceW"),
        ("kernel32.dll", "LoadResource"),
        ("kernel32.dll", "LockResource"),
        ("kernel32.dll", "SizeofResource"),
        ("USER32.dll", "LoadStringW"),
    ];
    let img = build_imports(&imps);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for (_, name) in imps {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    assert!(!st.unsupported.contains_key("Jmp: indirect through IAT slot (not lifted)"));
    // Same name in another DLL is not the same API.
    let img = build_imports(&[("kernel32.dll", "LoadStringW"), ("user32.dll", "FindResourceW")]);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    assert!(!st.handled.contains_key("jmp_imp:LoadStringW"));
    assert!(!st.handled.contains_key("jmp_imp:FindResourceW"));
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 2);
}

// ---------------------------------------------------------------------------------------------
// C driver
// ---------------------------------------------------------------------------------------------

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_env.h"
#include "fle_res.h"
#include "boot.h"

uint64_t R[16];
static uint64_t stk[32];

static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv(f, (uint64_t[4]){ __VA_ARGS__ })
#define A(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
#define W(s) A(u##s)
#define U(x) ((unsigned long long)(x))
#define HI FLE_EXE_HANDLE
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static uint64_t FNV(uint64_t va, uint32_t n)
{
    uint64_t h = 0xcbf29ce484222325ull;
    uint32_t i;
    for (i = 0; i < n; i++) { h ^= *(const uint8_t *)(uintptr_t)(va + i); h *= 0x100000001b3ull; }
    return h;
}
#define FIND(name, type) CALL(fle_imp_FindResourceW, HI, name, type)
#define LOADSTR(uid, buf, cch) CALL(fle_imp_LoadStringW, HI, uid, buf, cch)
"#;

fn run_with(name: &str, image: &PathBuf, body: &str) -> Out {
    let rt = runtime_dir();
    let rd = res_dir();
    let harness = manifest().join("../../harness");
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!(
        "{PRELUDE}\nint main(int argc, char **argv)\n{{\n    harness_image img;\n    char err[256];\n    (void)argc;\n    \
         if (image_load(argv[1], &img, err, sizeof err) != 0) {{ fprintf(stderr, \"load: %s\\n\", err); return 3; }}\n{body}\n    return 0;\n}}\n"
    );
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rd).arg("-I").arg(&rt).arg("-I").arg(&harness);
    for f in ["fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c"] {
        cmd.arg(rt.join(f));
    }
    cmd.arg(rd.join("fle_res.c")).arg(harness.join("image.c"));
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).arg(image).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    use std::os::unix::process::ExitStatusExt;
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines_on(name: &str, image: &PathBuf, body: &str) -> Vec<String> {
    let o = run_with(name, image, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, and the message must contain `must_say`.
fn assert_aborts(prefix: &str, image: &PathBuf, setup: &str, cases: &[(&str, &str, &str)]) {
    for (label, stmts, must_say) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run_with(&format!("{prefix}-{label}"), image, &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert!(o.stderr.contains(must_say), "{label}: message lacks {must_say:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

fn real_dll_path() -> Option<PathBuf> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => manifest().join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

macro_rules! real {
    ($path:ident) => {
        let Some($path) = real_dll_path() else { return };
    };
}

fn c_wide(s: &str) -> String {
    let mut v: Vec<String> = s.encode_utf16().map(|u| format!("{u}")).collect();
    v.push("0".into());
    format!("A((const uint16_t[]){{ {} }})", v.join(", "))
}

fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

const E_DATA: u32 = 1812;
const E_TYPE: u32 = 1813;
const E_NAME: u32 = 1814;

// ---------------------------------------------------------------------------------------------
// 2. Real DLL: LoadStringW
// ---------------------------------------------------------------------------------------------

/// Block number of string ID X is X / 16 + 1 and the index is X % 16 (Resource Types page). The strings below were
/// read by the independent Python parser: ID 64896 is block 4057 index 0.
#[test]
fn load_string_copies_a_known_string_and_terminates_it() {
    real!(dll);
    let l = lines_on(
        "res-ls-basic",
        &dll,
        r#"
    static uint16_t buf[64];
    int i;
    uint64_t r;
    for (i = 0; i < 64; i++) buf[i] = 0xAAAA;
    SETLE(77);
    r = LOADSTR(64896, A(buf), 64);
    printf("ret %llu le %u\n", U(r), LE());
    for (i = 0; i < 22; i++) printf("%04x ", buf[i]);
    printf("\n%04x %04x\n", buf[62], buf[63]);
"#,
    );
    // "Set Size Exceeded." is 18 characters; the success leaves the last error alone (page: silent; decision 0025).
    assert_eq!(l[0], "ret 18 le 77");
    let want: Vec<String> = "Set Size Exceeded.".encode_utf16().map(|c| format!("{c:04x}")).collect();
    let got: Vec<&str> = l[1].split_whitespace().collect();
    assert_eq!(&got[..18], &want.iter().map(|s| s.as_str()).collect::<Vec<_>>()[..]);
    assert_eq!(got[18], "0000", "NUL appended");
    assert_eq!(got[19], "aaaa", "nothing written past the NUL");
    assert_eq!(l[2], "aaaa aaaa");
}

#[test]
fn load_string_truncates_and_still_terminates() {
    real!(dll);
    let l = lines_on(
        "res-ls-trunc",
        &dll,
        r#"
    static uint16_t buf[32];
    int i;
    uint64_t r;
    int cch[] = { 10, 1, 18, 19, 2 };
    int k;
    for (k = 0; k < 5; k++) {
        for (i = 0; i < 32; i++) buf[i] = 0xAAAA;
        r = LOADSTR(64896, A(buf), cch[k]);
        printf("%d %llu", cch[k], U(r));
        for (i = 0; i < 21; i++) printf(" %04x", buf[i]);
        printf("\n");
    }
"#,
    );
    let text: Vec<u16> = "Set Size Exceeded.".encode_utf16().collect();
    // (cchBufferMax, expected return): the page says the string is truncated and NUL-terminated, the return is the
    // number of characters copied without the NUL. cchBufferMax 1 copies nothing and writes only the NUL.
    for (row, (cch, ret)) in [(10usize, 9usize), (1, 0), (18, 17), (19, 18), (2, 1)].into_iter().enumerate() {
        let f: Vec<&str> = l[row].split_whitespace().collect();
        assert_eq!(f[0], cch.to_string());
        assert_eq!(f[1], ret.to_string(), "cch {cch}");
        for i in 0..21 {
            let v = u16::from_str_radix(f[2 + i], 16).unwrap();
            let want = if i < ret { text[i] } else if i == ret { 0 } else { 0xAAAA };
            assert_eq!(v, want, "cch {cch} unit {i}");
        }
    }
}

#[test]
fn load_string_with_zero_size_returns_a_pointer_into_the_image() {
    real!(dll);
    let l = lines_on(
        "res-ls-ptr",
        &dll,
        r#"
    static uint64_t slot[2];
    uint64_t r, p;
    int i;
    slot[0] = 0x1111; slot[1] = 0x2222;
    SETLE(9);
    r = LOADSTR(64896, A(slot), 0);
    p = slot[0];
    printf("ret %llu le %u\n", U(r), LE());
    printf("inimage %d tail %llx\n", p > 0x400000 && p < 0x400000 + 0x18c0000, U(slot[1]));
    for (i = 0; i < 18; i++) printf("%04x ", *(const uint16_t *)(uintptr_t)(p + 2 * i));
    printf("\n");
    /* The pointer is to the counted string: the unit before it is the length. */
    printf("len %u\n", *(const uint16_t *)(uintptr_t)(p - 2));
"#,
    );
    assert_eq!(l[0], "ret 18 le 9");
    assert_eq!(l[1], "inimage 1 tail 2222", "an 8-byte pointer is stored and nothing else is written");
    let want: Vec<String> = "Set Size Exceeded.".encode_utf16().map(|c| format!("{c:04x}")).collect();
    assert_eq!(l[2].trim(), want.join(" "));
    assert_eq!(l[3], "len 18");
}

#[test]
fn load_string_missing_strings_return_zero_and_leave_the_buffer() {
    real!(dll);
    let l = lines_on(
        "res-ls-missing",
        &dll,
        r#"
    static uint16_t buf[8];
    static uint64_t slot[1];
    uint64_t ids[] = { 64897, 5, 0, 64895, 64898 };
    int k, i;
    for (k = 0; k < 5; k++) {
        for (i = 0; i < 8; i++) buf[i] = 0xAAAA;
        SETLE(0);
        uint64_t r = LOADSTR(ids[k], A(buf), 8);
        printf("%llu le %u b0 %04x\n", U(r), LE(), buf[0]);
    }
    slot[0] = 0x1111;
    SETLE(0);
    uint64_t r = LOADSTR(64897, A(slot), 0);
    printf("ptr-mode %llu le %u slot %llx\n", U(r), LE(), U(slot[0]));
"#,
    );
    // 64897 and 64898 are unused IDs inside block 4057 (zero length; the Python parser lists 15 such IDs in this
    // block); 5, 0 and 64895 are in blocks that do not exist (block 4057 is the first of the 40 string blocks).
    // Codes are decision 0025 (the page names none).
    for row in 0..5 {
        assert_eq!(l[row], format!("0 le {E_NAME} b0 aaaa"), "row {row}");
    }
    assert_eq!(l[5], format!("ptr-mode 0 le {E_NAME} slot 1111"));
}

/// The 625 strings of the DLL, found through fe-pe's parser, must all come back identical, and no other ID may
/// answer. Two implementations (Rust parser, C runtime) and the Python count (625) agree.
#[test]
fn load_string_matches_fe_pe_for_every_id() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    let pe = Pe::parse(&data).unwrap();
    let strs = pe.strings().unwrap();
    assert_eq!(strs.len(), 625);
    let want: BTreeMap<u32, (usize, u64)> = strs
        .iter()
        .map(|s| {
            let mut sum: u64 = 0;
            for &u in &s.text {
                sum = sum.wrapping_mul(31).wrapping_add(u as u64);
            }
            (s.id, (s.text.len(), sum))
        })
        .collect();
    let l = lines_on(
        "res-ls-all",
        &dll,
        r#"
    static uint16_t buf[1024];
    uint32_t uid, zeros = 0, bad = 0, i;
    for (uid = 0; uid < 0x10000; uid++) {
        uint64_t r, sum = 0;
        for (i = 0; i < 1024; i++) buf[i] = 0xAAAA;
        SETLE(0);
        r = LOADSTR(uid, A(buf), 1024);
        if (r == 0) {
            zeros++;
            if (LE() != 1814 || buf[0] != 0xAAAA) bad++;
            continue;
        }
        for (i = 0; i < r; i++) sum = sum * 31 + buf[i];
        if (buf[r] != 0 || buf[r + 1] != 0xAAAA) bad++;
        printf("%u %llu %llu\n", uid, U(r), U(sum));
    }
    printf("zeros %u bad %u\n", zeros, bad);
"#,
    );
    let mut got = BTreeMap::new();
    for line in &l[..l.len() - 1] {
        let f: Vec<u64> = line.split(' ').map(|x| x.parse().unwrap()).collect();
        got.insert(f[0] as u32, (f[1] as usize, f[2]));
    }
    assert_eq!(got, want);
    assert_eq!(l[l.len() - 1], format!("zeros {} bad 0", 0x10000 - want.len()));
}

#[test]
fn load_string_longest_string_and_embedded_text() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    let pe = Pe::parse(&data).unwrap();
    let longest = pe.strings().unwrap().into_iter().max_by_key(|s| s.text.len()).unwrap();
    assert_eq!((longest.id, longest.text.len()), (65114, 230)); // Python parser
    let l = lines_on(
        "res-ls-long",
        &dll,
        &format!(
            r#"
    static uint16_t buf[300];
    uint64_t r;
    r = LOADSTR({id}, A(buf), 300);
    printf("%llu %u\n", U(r), buf[229]);
    r = LOADSTR({id}, A(buf), 230);
    printf("%llu %u %u\n", U(r), buf[228], buf[229]);
"#,
            id = longest.id
        ),
    );
    assert_eq!(l[0], format!("230 {}", longest.text[229]));
    assert_eq!(l[1], format!("229 {} 0", longest.text[228]));
}

// ---------------------------------------------------------------------------------------------
// 2b. Real DLL: FindResourceW, LoadResource, LockResource, SizeofResource
// ---------------------------------------------------------------------------------------------

#[test]
fn find_load_lock_size_for_known_resources() {
    real!(dll);
    // Values from the Python parser: (type, name, size, first 8 data bytes).
    // RT_VERSION (16) name 1: 728 bytes; RT_MANIFEST (24) name 1: 1495 bytes, "<?xml ve"; RT_GROUP_ICON (14) name
    // "MAINICON": 188 bytes; RT_ICON (3) name 2: 296 bytes; RT_DIALOG (5) name "MYDLGO": 82 bytes.
    let l = lines_on(
        "res-known",
        &dll,
        &format!(
            r#"
    struct {{ uint64_t name, type; }} k[] = {{
        {{ 1, 16 }}, {{ 1, 24 }}, {{ {mainicon}, 14 }}, {{ 2, 3 }}, {{ {mydlgo}, 5 }},
    }};
    int i, j;
    for (i = 0; i < 5; i++) {{
        uint64_t hr, hg, p, sz;
        SETLE(77);
        hr = FIND(k[i].name, k[i].type);
        hg = CALL(fle_imp_LoadResource, HI, hr);
        p = CALL(fle_imp_LockResource, hg);
        sz = CALL(fle_imp_SizeofResource, HI, hr);
        printf("%d le %u same %d size %llu", i, LE(), hg == p, U(sz));
        for (j = 0; j < 8; j++) printf(" %02x", *(const uint8_t *)(uintptr_t)(p + j));
        printf("\n");
    }}
"#,
            mainicon = c_wide("MAINICON"),
            mydlgo = c_wide("MYDLGO"),
        ),
    );
    assert_eq!(l[0], "0 le 77 same 1 size 728 d8 02 34 00 00 00 56 00");
    assert_eq!(l[1], "1 le 77 same 1 size 1495 3c 3f 78 6d 6c 20 76 65");
    assert_eq!(l[2], "2 le 77 same 1 size 188 00 00 01 00 0d 00 20 20");
    assert!(l[3].starts_with("3 le 77 same 1 size 296 28 00 00 00 10 00 00 00"), "{}", l[3]);
    assert!(l[4].starts_with("4 le 77 same 1 size 82 44 04 00 54 00 00 00 00"), "{}", l[4]);
}

#[test]
fn name_and_type_rules_of_find_resource() {
    real!(dll);
    let l = lines_on(
        "res-rules",
        &dll,
        &format!(
            r##"
    uint64_t r;
    /* hModule NULL selects the module of the process (the lifted DLL). */
    r = CALL(fle_imp_FindResourceW, 0, 1, 16); printf("null-module %d\n", r != 0);
    /* "#" and decimal digits is an integer ID, for the name and for the type (page). */
    r = FIND({n1}, 16);      printf("hash-name %d\n", r != 0 && r == FIND(1, 16));
    r = FIND(1, {t16});      printf("hash-type %d\n", r != 0 && r == FIND(1, 16));
    r = FIND({n1}, {t16});   printf("hash-both %d\n", r != 0 && r == FIND(1, 16));
    r = FIND({n001}, 16);    printf("hash-leading-zeros %d\n", r != 0 && r == FIND(1, 16));
    /* A string name is compared without regard to ASCII case (decision 0025: the page does not say). */
    r = FIND({mainicon}, 14); printf("exact %d\n", r != 0);
    r = FIND({mainicon_l}, 14); printf("lower %d\n", r != 0 && r == FIND({mainicon}, 14));
    r = FIND({mainicon_m}, 14); printf("mixed %d\n", r != 0 && r == FIND({mainicon}, 14));
    /* Not found: the type level or the name level decides the code. */
    SETLE(0); r = FIND({mainico}, 14);  printf("short %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND({mainicon_x}, 14); printf("long %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(999, 16);        printf("noid %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(1, 99);          printf("notype %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(1, {tnone});     printf("notype-name %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND({mainicon}, 16); printf("name-under-wrong-type %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(0, 16);          printf("id0 %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(1, 0);           printf("type0 %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND({empty}, 16);    printf("empty-name %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(1, {empty});     printf("empty-type %llu le %u\n", U(r), LE());
    /* An ID above 0xFFFF cannot be written as MAKEINTRESOURCE; "#65535" is the largest "#" form. */
    SETLE(0); r = FIND({n65535}, 16);   printf("hash-max %llu le %u\n", U(r), LE());
"##,
            n1 = c_wide("#1"),
            n001 = c_wide("#001"),
            t16 = c_wide("#16"),
            mainicon = c_wide("MAINICON"),
            mainicon_l = c_wide("mainicon"),
            mainicon_m = c_wide("MainIcon"),
            mainico = c_wide("MAINICO"),
            mainicon_x = c_wide("MAINICONX"),
            tnone = c_wide("NOSUCHTYPE"),
            empty = c_wide(""),
            n65535 = c_wide("#65535"),
        ),
    );
    let want = [
        "null-module 1".to_string(),
        "hash-name 1".into(),
        "hash-type 1".into(),
        "hash-both 1".into(),
        "hash-leading-zeros 1".into(),
        "exact 1".into(),
        "lower 1".into(),
        "mixed 1".into(),
        format!("short 0 le {E_NAME}"),
        format!("long 0 le {E_NAME}"),
        format!("noid 0 le {E_NAME}"),
        format!("notype 0 le {E_TYPE}"),
        format!("notype-name 0 le {E_TYPE}"),
        format!("name-under-wrong-type 0 le {E_NAME}"),
        format!("id0 0 le {E_NAME}"),
        format!("type0 0 le {E_TYPE}"),
        format!("empty-name 0 le {E_NAME}"),
        format!("empty-type 0 le {E_TYPE}"),
        format!("hash-max 0 le {E_NAME}"),
    ];
    assert_eq!(l, want);
}

/// Every leaf of the tree, found through fe-pe, must be reachable with the Find/Load/Lock/Sizeof chain and give the
/// same size, the same data address (image base + data RVA) and the same bytes (FNV-1a).
#[test]
fn every_leaf_is_found_and_its_bytes_match_fe_pe() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    let pe = Pe::parse(&data).unwrap();
    let leaves = pe.resources().unwrap();
    assert_eq!(leaves.len(), 327);
    // (type, folded name) must be unique, else FindResourceW's first match would not be the leaf.
    let mut seen = std::collections::BTreeSet::new();
    for l in &leaves {
        let fold = |id: &ResourceId| match id {
            ResourceId::Id(n) => format!("#{n}"),
            ResourceId::Name(s) => s.to_ascii_lowercase(),
        };
        assert!(seen.insert((fold(&l.type_id), fold(&l.name))), "duplicate after case folding: {l:?}");
    }
    let key = |id: &ResourceId| match id {
        ResourceId::Id(n) => format!("{n}"),
        ResourceId::Name(s) => c_wide(s),
    };
    let mut body = String::from("    uint64_t hr, hg, p, sz;\n");
    let mut expect = Vec::new();
    for (i, l) in leaves.iter().enumerate() {
        body += &format!(
            "    hr = FIND({n}, {t}); hg = CALL(fle_imp_LoadResource, HI, hr); p = CALL(fle_imp_LockResource, hg); \
             sz = CALL(fle_imp_SizeofResource, HI, hr);\n    printf(\"{i} %llu %llx %d %llx\\n\", U(sz), U(p - 0x400000), hg == p && hr != 0, U(FNV(p, (uint32_t)sz)));\n",
            n = key(&l.name),
            t = key(&l.type_id)
        );
        expect.push(format!("{i} {} {:x} 1 {:x}", l.size, l.data_rva, fnv(pe.resource_data(l).unwrap())));
    }
    let got = lines_on("res-all-leaves", &dll, &body);
    assert_eq!(got.len(), expect.len());
    for (g, e) in got.iter().zip(&expect) {
        assert_eq!(g, e);
    }
}

#[test]
fn the_string_block_resource_is_the_block_of_sixteen() {
    real!(dll);
    // FindResourceW(RT_STRING = 6, block 4057) -> the block that LoadStringW(64896) reads from: 68 bytes (measured
    // by the Python parser), 16 length words and the 18 + 0... units; data begins with the length 18.
    let l = lines_on(
        "res-strblock",
        &dll,
        r#"
    uint64_t hr = FIND(4057, 6), hg = CALL(fle_imp_LoadResource, HI, hr), p = CALL(fle_imp_LockResource, hg);
    printf("%llu %u %u\n", U(CALL(fle_imp_SizeofResource, HI, hr)), *(const uint16_t *)(uintptr_t)p, *(const uint16_t *)(uintptr_t)(p + 2 + 36));
    SETLE(0);
    printf("%llu le %u\n", U(FIND(4056, 6)), LE());
"#,
    );
    // Block 4057: string 0 has 18 units (2 + 36 bytes); the word after it is string 1's length, 0 (ID 64897 unused).
    assert_eq!(l[0], "68 18 0");
    assert_eq!(l[1], format!("0 le {E_NAME}"));
}

#[test]
fn module_handles_and_the_other_dll_handles() {
    real!(dll);
    // GetModuleHandleW(NULL) and the image base are the same handle (decision 0015), accepted by every API.
    let l = lines_on(
        "res-handles",
        &dll,
        r#"
    uint64_t me, hr, hg;
    static uint16_t buf[16];
    R[1] = 0; fle_imp_GetModuleHandleW(); me = R[0];
    hr = CALL(fle_imp_FindResourceW, me, 1, 16);
    hg = CALL(fle_imp_LoadResource, me, hr);
    printf("%llx %d %d %llu %llu\n", U(me), hr != 0, hg != 0, U(CALL(fle_imp_SizeofResource, me, hr)), U(CALL(fle_imp_LoadStringW, me, 64896, A(buf), 16)));
"#,
    );
    assert_eq!(l[0], "400000 1 1 728 15");
}

#[test]
fn inputs_the_pages_leave_open_stop_the_run() {
    real!(dll);
    let setup = "    uint64_t hr = FIND(1, 16), hg = CALL(fle_imp_LoadResource, HI, hr);\n    static uint16_t buf[16];\n    (void)hg;\n";
    let long_name = format!("A((const uint16_t[]){{ {} 0 }})", "65, ".repeat(300));
    assert_aborts(
        "res-abort",
        &dll,
        setup,
        &[
            // other modules have no resources here
            ("find-user32", "CALL(fle_imp_FindResourceW, fle_module_handle(18), 1, 16);", "user32.dll"),
            ("find-kernel32", "CALL(fle_imp_FindResourceW, fle_module_handle(8), 1, 16);", "kernel32.dll"),
            ("find-garbage-module", "CALL(fle_imp_FindResourceW, 0x1234, 1, 16);", "not a module handle"),
            ("load-user32", "CALL(fle_imp_LoadResource, fle_module_handle(18), hr);", "user32.dll"),
            ("size-user32", "CALL(fle_imp_SizeofResource, fle_module_handle(18), hr);", "user32.dll"),
            ("loadstr-user32", "CALL(fle_imp_LoadStringW, fle_module_handle(18), 64896, A(buf), 16);", "user32.dll"),
            ("loadstr-garbage-module", "CALL(fle_imp_LoadStringW, 0x1234, 64896, A(buf), 16);", "not a module handle"),
            // LoadStringW with hInstance NULL: "optional" on the page but not defined
            ("loadstr-null-module", "CALL(fle_imp_LoadStringW, 0, 64896, A(buf), 16);", "hInstance is NULL"),
            // handles that were not returned by the previous call
            ("load-null-hrsrc", "CALL(fle_imp_LoadResource, HI, 0);", "hResInfo is NULL"),
            ("load-garbage-hrsrc", "CALL(fle_imp_LoadResource, HI, 0x1234);", "not a resource handle"),
            ("load-hrsrc-plus-1", "CALL(fle_imp_LoadResource, HI, hr + 1);", "not a resource handle"),
            ("load-with-hglobal", "CALL(fle_imp_LoadResource, HI, hg);", "not a resource handle"),
            ("size-null-hrsrc", "CALL(fle_imp_SizeofResource, HI, 0);", "hResInfo is NULL"),
            ("size-garbage-hrsrc", "CALL(fle_imp_SizeofResource, HI, 0x1234);", "not a resource handle"),
            ("lock-null", "CALL(fle_imp_LockResource, 0);", "hResData is NULL"),
            ("lock-hrsrc", "CALL(fle_imp_LockResource, hr);", "not a LoadResource result"),
            ("lock-garbage", "CALL(fle_imp_LockResource, 0x1234);", "not a LoadResource result"),
            ("lock-inside-data", "CALL(fle_imp_LockResource, hg + 1);", "not a LoadResource result"),
            // names that are not defined
            ("find-hash-only", &format!("FIND({}, 16);", c_wide("#")), "without digits"),
            ("find-hash-letters", &format!("FIND({}, 16);", c_wide("#abc")), "not a decimal number"),
            ("find-hash-minus", &format!("FIND({}, 16);", c_wide("#-1")), "not a decimal number"),
            ("find-hash-too-big", &format!("FIND({}, 16);", c_wide("#65536")), "above 65535"),
            ("find-hash-huge", &format!("FIND({}, 16);", c_wide("#99999999999999999999")), "above 65535"),
            ("find-type-hash-letters", &format!("FIND(1, {});", c_wide("#x")), "not a decimal number"),
            ("find-long-name", &format!("FIND({long_name}, 16);"), "longer than the supported limit"),
            // LoadStringW arguments
            ("loadstr-negative-cch", "LOADSTR(64896, A(buf), 0xFFFFFFFFu);", "negative"),
            ("loadstr-id-above-16-bits", "LOADSTR(0x10000, A(buf), 16);", "above 0xFFFF"),
            ("loadstr-null-buffer", "LOADSTR(64896, 0, 16);", "lpBuffer is NULL"),
            ("loadstr-null-buffer-ptrmode", "LOADSTR(64896, 0, 0);", "lpBuffer is NULL"),
        ],
    );
}

// ---------------------------------------------------------------------------------------------
// 3. Synthetic images
// ---------------------------------------------------------------------------------------------

const S_RSRC_RVA: u32 = 0x1000;
const S_RSRC_RAW: usize = 0x400;
const S_RSRC_LEN: usize = 0x800;
const HIBIT: u32 = 0x8000_0000;

fn s_table(r: &mut [u8], off: usize, n_name: u16, n_id: u16, entries: &[(u32, u32)]) {
    w16(r, off + 12, n_name);
    w16(r, off + 14, n_id);
    for (i, (a, b)) in entries.iter().enumerate() {
        w32(r, off + 16 + 8 * i, *a);
        w32(r, off + 20 + 8 * i, *b);
    }
}

fn s_leaf(r: &mut [u8], off: usize, data_off: u32, size: u32) {
    w32(r, off, S_RSRC_RVA + data_off);
    w32(r, off + 4, size);
}

fn s_block(strs: &[(usize, &[u16])]) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..16 {
        match strs.iter().find(|(k, _)| *k == i) {
            Some((_, u)) => {
                out.extend_from_slice(&(u.len() as u16).to_le_bytes());
                for c in *u {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
            None => out.extend_from_slice(&0u16.to_le_bytes()),
        }
    }
    out
}

/// A PE32+ at ImageBase 0x400000 (size 0x3000) with a .rsrc section at RVA 0x1000; `with_dir` = false leaves data
/// directory 2 empty. `fill` writes the tree into the section bytes.
fn build_rsrc(with_dir: bool, fill: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut b = vec![0u8; 0x1000];
    b.resize(S_RSRC_RAW + S_RSRC_LEN + 0x400, 0);
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w64(&mut b, o + 24, 0x40_0000);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    if with_dir {
        w32(&mut b, o + 128, S_RSRC_RVA);
        w32(&mut b, o + 132, S_RSRC_LEN as u32);
    }
    let s0 = o + 240;
    b[s0..s0 + 5].copy_from_slice(b".rsrc");
    w32(&mut b, s0 + 8, S_RSRC_LEN as u32);
    w32(&mut b, s0 + 12, S_RSRC_RVA);
    w32(&mut b, s0 + 16, S_RSRC_LEN as u32);
    w32(&mut b, s0 + 20, S_RSRC_RAW as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);
    fill(&mut b[S_RSRC_RAW..S_RSRC_RAW + S_RSRC_LEN]);
    b
}

fn write_image(name: &str, bytes: &[u8]) -> PathBuf {
    let p = work_dir().join(format!("{}.dll", etxt_common::per_process(name)));
    std::fs::write(&p, bytes).unwrap();
    p
}

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// RT_STRING (6) block 3 (IDs 32..47) in language 0x409: ID 32 "Hi", ID 33 "a\0b" (embedded NUL), ID 47 "end";
/// block 1 (IDs 0..15) has only ID 5 and has trailing padding bytes.
fn good_tree(r: &mut [u8]) {
    s_table(r, 0x000, 0, 1, &[(6, HIBIT | 0x100)]);
    s_table(r, 0x100, 0, 2, &[(1, HIBIT | 0x120), (3, HIBIT | 0x140)]);
    s_table(r, 0x120, 0, 1, &[(0x409, 0x160)]);
    s_table(r, 0x140, 0, 1, &[(0x409, 0x170)]);
    let b1 = s_block(&[(5, &u16s("five"))]);
    let b3 = s_block(&[(0, &u16s("Hi")), (1, &[b'a' as u16, 0, b'b' as u16]), (15, &u16s("end"))]);
    s_leaf(r, 0x160, 0x300, b1.len() as u32 + 4); // 4 bytes of padding after the 16th string
    s_leaf(r, 0x170, 0x400, b3.len() as u32);
    r[0x300..0x300 + b1.len()].copy_from_slice(&b1);
    r[0x400..0x400 + b3.len()].copy_from_slice(&b3);
}

#[test]
fn synthetic_image_strings_blocks_padding_and_embedded_nul() {
    let p = write_image("good", &build_rsrc(true, good_tree));
    let l = lines_on(
        "res-syn-good",
        &p,
        r#"
    static uint16_t buf[16];
    static uint64_t slot[1];
    uint64_t r;
    int i;
    r = LOADSTR(5, A(buf), 16);   printf("five %llu %04x %04x\n", U(r), buf[0], buf[4]);
    r = LOADSTR(32, A(buf), 16);  printf("hi %llu %04x %04x\n", U(r), buf[1], buf[2]);
    /* embedded NUL: the count is the length word, not strlen */
    for (i = 0; i < 16; i++) buf[i] = 0xAAAA;
    r = LOADSTR(33, A(buf), 16);  printf("nul %llu %04x %04x %04x %04x\n", U(r), buf[0], buf[1], buf[2], buf[3]);
    r = LOADSTR(33, A(slot), 0);  printf("nulptr %llu\n", U(r));
    r = LOADSTR(47, A(buf), 16);  printf("end %llu %04x\n", U(r), buf[2]);
    SETLE(0); r = LOADSTR(34, A(buf), 16); printf("unused %llu le %u\n", U(r), LE());
    SETLE(0); r = LOADSTR(16, A(buf), 16); printf("noblock %llu le %u\n", U(r), LE());
    /* type 6 exists, so a missing block is a missing name; the block of type 7 is a missing type */
    SETLE(0); r = FIND(2, 6); printf("find2 %llu le %u\n", U(r), LE());
    SETLE(0); r = FIND(1, 7); printf("find7 %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(
        l,
        vec![
            "five 4 0066 0000".to_string(), // 'f' then NUL at index 4
            "hi 2 0069 0000".into(),
            "nul 3 0061 0000 0062 0000".into(),
            "nulptr 3".into(),
            "end 3 0064".into(),
            format!("unused 0 le {E_NAME}"),
            format!("noblock 0 le {E_NAME}"),
            format!("find2 0 le {E_NAME}"),
            format!("find7 0 le {E_TYPE}"),
        ]
    );
}

#[test]
fn synthetic_image_without_a_resource_directory() {
    let p = write_image("nodir", &build_rsrc(false, |_| {}));
    let l = lines_on(
        "res-syn-nodir",
        &p,
        r#"
    static uint16_t buf[4];
    SETLE(0); printf("find %llu le %u\n", U(FIND(1, 16)), LE());
    SETLE(0); printf("str %llu le %u\n", U(LOADSTR(64896, A(buf), 4)), LE());
"#,
    );
    assert_eq!(l, vec![format!("find 0 le {E_DATA}"), format!("str 0 le {E_DATA}")]);
}

#[test]
fn synthetic_image_with_an_empty_resource_directory_has_no_type() {
    let p = write_image("emptydir", &build_rsrc(true, |_| {}));
    let l = lines_on(
        "res-syn-emptydir",
        &p,
        r#"
    SETLE(0); printf("find %llu le %u\n", U(FIND(1, 16)), LE());
"#,
    );
    assert_eq!(l, vec![format!("find 0 le {E_TYPE}")]);
}

#[test]
fn synthetic_name_without_a_language_leaf_is_lang_not_found() {
    let p = write_image(
        "nolang",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(7, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 0, &[]);
        }),
    );
    let l = lines_on("res-syn-nolang", &p, "    SETLE(0); printf(\"%llu le %u\\n\", U(FIND(7, 10)), LE());");
    assert_eq!(l, vec!["0 le 1815".to_string()]);
}

#[test]
fn synthetic_malformed_images_stop_the_run() {
    // several language leaves under one (type, name)
    let multi = write_image(
        "multilang",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(7, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 2, &[(0x409, 0x200), (0x407, 0x210)]);
            s_leaf(r, 0x200, 0x300, 4);
            s_leaf(r, 0x210, 0x310, 4);
        }),
    );
    assert_aborts("res-syn-multi", &multi, "", &[("find", "FIND(7, 10);", "language")]);

    // data outside the image
    let outside = write_image(
        "outside",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(7, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 1, &[(0, 0x200)]);
            w32(r, 0x200, 0x2ff0); // data RVA 0x2ff0 + size 0x20 passes SizeOfImage 0x3000
            w32(r, 0x204, 0x20);
        }),
    );
    assert_aborts(
        "res-syn-outside",
        &outside,
        "    uint64_t hr = FIND(7, 10);\n",
        &[("load", "CALL(fle_imp_LoadResource, HI, hr);", "outside the image"), ("size", "CALL(fle_imp_SizeofResource, HI, hr);", "outside the image")],
    );

    // a data entry where a subdirectory is needed, and the reverse
    let bad_level = write_image(
        "badlevel",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, 0x200)]);
        }),
    );
    assert_aborts("res-syn-level", &bad_level, "", &[("type", "FIND(7, 10);", "data entry at the type level")]);
    let bad_lang = write_image(
        "badlang",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(7, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 1, &[(0, HIBIT | 0x140)]);
        }),
    );
    assert_aborts("res-syn-lang", &bad_lang, "", &[("lang", "FIND(7, 10);", "subdirectory at the language level")]);

    // an entry count that runs past the directory
    let long_table = write_image(
        "longtable",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(10, HIBIT | 0x100)]);
            w16(r, 0x100 + 14, 0xffff);
        }),
    );
    assert_aborts("res-syn-count", &long_table, "", &[("find", "FIND(7, 10);", "outside the directory")]);

    // a string block that is too short, and a length that runs past the block
    let short_block = write_image(
        "shortblock",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(6, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(1, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 1, &[(0, 0x200)]);
            s_leaf(r, 0x200, 0x300, 20); // 10 length words only
        }),
    );
    assert_aborts("res-syn-short", &short_block, "    static uint16_t buf[4];\n", &[("str", "LOADSTR(15, A(buf), 4);", "shorter than its 16 strings")]);
    let long_len = write_image(
        "longlen",
        &build_rsrc(true, |r| {
            s_table(r, 0x000, 0, 1, &[(6, HIBIT | 0x100)]);
            s_table(r, 0x100, 0, 1, &[(1, HIBIT | 0x120)]);
            s_table(r, 0x120, 0, 1, &[(0, 0x200)]);
            s_leaf(r, 0x200, 0x300, 32);
            r[0x300] = 0x40; // string 0 claims 64 units in a 32-byte block
        }),
    );
    assert_aborts("res-syn-longlen", &long_len, "    static uint16_t buf[4];\n", &[("str", "LOADSTR(5, A(buf), 4);", "runs past the block")]);
}

// ---------------------------------------------------------------------------------------------
// 4. fe-pe agrees about the real resource section sizes used above
// ---------------------------------------------------------------------------------------------

#[test]
fn real_dll_facts_used_by_these_tests() {
    real!(dll);
    let data = std::fs::read(&dll).unwrap();
    let pe = Pe::parse(&data).unwrap();
    assert_eq!(pe.optional.image_base, 0x400000);
    assert_eq!(pe.optional.size_of_image, 0x18c0000);
    let leaves = pe.resources().unwrap();
    let get = |t: u32, n: ResourceId| leaves.iter().find(|l| l.type_id == ResourceId::Id(t) && l.name == n).unwrap();
    assert_eq!(get(16, ResourceId::Id(1)).size, 728);
    assert_eq!(get(24, ResourceId::Id(1)).size, 1495);
    assert_eq!(get(14, ResourceId::Name("MAINICON".into())).size, 188);
    assert_eq!(get(3, ResourceId::Id(2)).size, 296);
    assert_eq!(get(5, ResourceId::Name("MYDLGO".into())).size, 82);
    assert_eq!(get(RT_STRING, ResourceId::Id(4057)).size, 68);
}
