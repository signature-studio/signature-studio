//! Tests of decision 0118 (docs/decisions/0118-cpuid-baseline-and-fruity-walls.md): wall 7 (rpcrt4.dll as virtual module N = 36 and rpcrt4!UuidCreate), wall 8
//! (user32!LoadIconW(hInstance, name) from the RT_GROUP_ICON of the image, and user32!GetIconInfo of that icon), GetModuleFileNameW/A(NULL) = the host exe FL64.exe, and the
//! bytes of the real FLEngine_x64.dll that those decisions rest on.
//!
//! Expected values are written from the Microsoft Learn pages named in the decision (UuidCreate, GUID, LoadIconW, LookupIconIdFromDirectoryEx, GetIconInfo, ICONINFO, NEWHEADER,
//! RESDIR, ICONRESDIR, GetModuleFileNameW), from RFC 4122 section 4.4, and from decisions 0015, 0018, 0054, 0099, 0104, 0108; not read back from the C code. The UUID values are
//! also computed by an independent implementation of SplitMix64 written in this file (`expected_uuid`) and, once, by a Python script (run record). EVERY ANSWER IS
//! PROVISIONAL: nothing is compared with a real Windows.
//!
//! 1. Generator: the new row (rpcrt4.dll!UuidCreate) is the last row; the thunk is answered under rpcrt4.dll only.
//! 2. UuidCreate: the form of the UUID (version 4, variant bits), the fixed sequence, the stops.
//! 3. LoadIconW and GetIconInfo on the real FLEngine_x64.dll (mapped by harness/image.c at 0x400000) and on synthetic images (the choice rules).
//! 4. GetModuleFileNameW/A(NULL): the exe path; 0x400000 stays the DLL path; GetModuleHandleW(NULL) stays 0x400000.
//! 5. The bytes of the real DLL (skipped when absent).

mod etxt_common;
mod rows_0132;
mod rows_0139;
mod rows_0140;
mod rows_0144;
mod rows_0147;
mod rows_0267;

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
    let d = manifest().join("../../out-wallFR5/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

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

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

#[test]
fn the_uuid_create_row_is_the_last_row_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let n = rows.len();
    // decision 0015: append only. The last row of decision 0117 is GetUserPreferredUILanguages; the one row of decision 0118 follows it. Decision 0119 appended 8 rows
    // after it (tests/wall_fr6.rs; old: the UuidCreate row was the last one, `rows[n - 1]`), decision 0122 another 15 (tests/wall_fr8.rs), decision 0123 another 73 (tests/wall_fr9.rs), decision 0125 another 2 (tests/wall_fr10.rs): LATER_ROWS = 98 (decision 0123: 96; decision 0122: 23; decision 0119 only: 8).
    // decision 0126 appended 33 more (tests/wall_fr11.rs; old value 8 + 15 + 73 + 2).
    const LATER_ROWS: usize = 8 + 15 + 73 + 2 + 33 + 4 + rows_0132::ROWS_0132.len() + 10 + rows_0139::ROWS_0139.len() + rows_0140::ROWS_0140.len() + rows_0144::ROWS_0144.len() + rows_0147::ROWS_0147.len() + rows_0267::ROWS_0267.len(); // decision 0267: + the row of tests/rows_0267/mod.rs after that of decision 0147; decision 0147: + the row of tests/rows_0147/mod.rs after those of decision 0144; decision 0136: + the 10 rows of tests/wall_fr14.rs, appended after those of decision 0132; decision 0139: + tests/rows_0139/mod.rs after those
    // decision 0129 (old: ... + 33): + 4 rows (tests/wall_fr12.rs); decision 0132: + tests/rows_0132/mod.rs
    assert_eq!((rows[n - LATER_ROWS - 2].0.as_str(), rows[n - LATER_ROWS - 2].1.as_str()), ("kernel32.dll", "GetUserPreferredUILanguages"));
    assert_eq!((rows[n - LATER_ROWS - 1].0.as_str(), rows[n - LATER_ROWS - 1].1.as_str()), ("rpcrt4.dll", "UuidCreate"));
    for (k, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (k as u64 + 1), "row {k}");
    }
    assert_eq!(rows.iter().filter(|(d, _, _)| d == "rpcrt4.dll").count(), 1);
    assert!(c.contains("void fle_imp_UuidCreate(void);"));
}

#[test]
fn the_uuid_create_thunk_is_answered_under_rpcrt4_and_no_other_dll() {
    let (st, c) = gen(&[("RPCRT4.dll", "UuidCreate"), ("user32.dll", "UuidCreate"), ("rpcrt4.dll", "UuidCreateSequential"), ("kernel32.dll", "GetProcAddress")]);
    assert_eq!(st.handled["jmp_imp:UuidCreate"], 1);
    assert_eq!(c.matches("fle_imp_UuidCreate(); return;").count(), 1, "only the rpcrt4 thunk is answered");
    for k in [1u64, 2] {
        let va = IB + 0x1040 + 0x10 * k;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "thunk {k} (another DLL / another function) stays unsupported");
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The driver (every runtime source, the generated C of a synthetic PE, harness/image.c)
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
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_proc.h"
#include "fle_gdi.h"
#include "fle_res.h"
#include "boot.h"

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
static void hex16(const uint8_t *p) { int i; for (i = 0; i < 16; i++) printf("%02x", p[i]); }

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
    harness_image img;
    char err[256];
    unsigned i;

    if (argc > 2 && strcmp(s, "uuid") != 0 && strcmp(s, "gmfn") != 0) {
        if (image_load(argv[2], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    }
    if (!strcmp(s, "uuid")) {          /* n: the first n UUIDs, one per line, then the last error */
        static uint8_t u[16];
        int n = atoi(argv[2]), k;
        for (k = 0; k < n; k++) {
            memset(u, 0xCC, sizeof u);
            prep(A(u), 0, 0, 0);
            fle_imp_UuidCreate();
            printf("%llu ", U(R[0])); hex16(u); printf(" le %llu\n", U(le()));
        }
    } else if (!strcmp(s, "uuid-null")) {
        prep(0, 0, 0, 0);
        fle_imp_UuidCreate();
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "rpcrt4-module")) {
        uint16_t *n1 = wide("rpcrt4.dll"), *n2 = wide("RPCRT4"), *n3 = wide("RpcRt4.DLL");
        prep(A(n1), 0, 0, 0); fle_imp_LoadLibraryW(); printf("%llx %llu\n", U(R[0]), U(le()));
        prep(A(n2), 0, 0, 0); fle_imp_GetModuleHandleW(); printf("%llx %llu\n", U(R[0]), U(le()));
        prep(A(n3), 0, 0, 0); fle_imp_LoadLibraryW(); printf("%llx %llu\n", U(R[0]), U(le()));
        printf("count %llu name35 %s handle35 %llx\n", U(fle_module_count()), fle_module_name(35), U(fle_module_handle(35)));
        printf("refs %u\n", fle_load_refcount(35));
        /* the lookup of the function: a pseudo-VA under rpcrt4 only */
        {
            static const char nm[] = "UuidCreate";
            prep(M(36), A(nm), 0, 0); fle_imp_GetProcAddress(); printf("gpa-rpcrt4 %s %llu\n", R[0] ? "found" : "missing", U(le()));
            prep(M(9), A(nm), 0, 0); fle_imp_GetProcAddress(); printf("gpa-kernel32 %s %llu\n", R[0] ? "found" : "missing", U(le()));
        }
    } else if (!strcmp(s, "gmfn")) {   /* handle: the path as GetModuleFileNameW gives it (wide) and as GetModuleFileNameA gives it */
        static uint16_t w[300]; static char a[300];
        uint64_t h = strtoull(argv[2], 0, 0);
        prep(h, A(w), 300, 0); fle_imp_GetModuleFileNameW();
        printf("W %llu ", U(R[0])); for (i = 0; w[i]; i++) putchar((char)w[i]); printf("\n");
        prep(h, A(a), 300, 0); fle_imp_GetModuleFileNameA();
        printf("A %llu %s\n", U(R[0]), a);
        prep(0, 0, 0, 0); fle_imp_GetModuleHandleW(); printf("GetModuleHandleW(NULL) %llx\n", U(R[0]));
    } else if (!strcmp(s, "icon")) {   /* argv[3] = the name (MAINICON | an id); prints handle, kind, then GetIconInfo */
        uint64_t name, h, h2;
        static uint8_t ii[32];
        uint16_t *w;
        if (!strcmp(argv[3], "MAINICON")) { w = wide("MAINICON"); name = A(w); }
        else name = strtoull(argv[3], 0, 0);
        prep(0x400000, name, 0, 0); fle_imp_LoadIconW(); h = R[0];
        prep(0x400000, name, 0, 0); fle_imp_LoadIconW(); h2 = R[0];
        printf("handle %llx same %d kind %d le %llu\n", U(h), h == h2, fle_gdi_kind(h), U(le()));
        printf("live %llu bitmaps %llu\n", U(fle_gdi_live_count()), U(fle_gdi_bitmap_live_count()));
        memset(ii, 0xCC, sizeof ii);
        prep(h, A(ii), 0, 0); fle_imp_GetIconInfo();
        printf("GetIconInfo %llu fIcon %u hot %u %u hbmMask %llx hbmColor %llx\n", U(R[0]), *(uint32_t *)(ii + 0), *(uint32_t *)(ii + 4), *(uint32_t *)(ii + 8),
               U(*(uint64_t *)(ii + 16)), U(*(uint64_t *)(ii + 24)));
        printf("bitmaps %llu kinds %d %d\n", U(fle_gdi_bitmap_live_count()), fle_gdi_kind(*(uint64_t *)(ii + 16)), fle_gdi_kind(*(uint64_t *)(ii + 24)));
        {
            static uint8_t bm[32];
            prep(*(uint64_t *)(ii + 16), 32, A(bm), 0); fle_imp_GetObjectW();
            printf("mask GetObjectW %llu w %u h %u planes %u bits %u\n", U(R[0]), *(uint32_t *)(bm + 4), *(uint32_t *)(bm + 8), *(uint16_t *)(bm + 16), *(uint16_t *)(bm + 18));
            prep(*(uint64_t *)(ii + 24), 32, A(bm), 0); fle_imp_GetObjectW();
            printf("color GetObjectW %llu w %u h %u planes %u bits %u\n", U(R[0]), *(uint32_t *)(bm + 4), *(uint32_t *)(bm + 8), *(uint16_t *)(bm + 16), *(uint16_t *)(bm + 18));
        }
        prep(h, A(ii), 0, 0); fle_imp_GetIconInfo();
        printf("again mask differs %d\n", *(uint64_t *)(ii + 16) != 0);
    } else if (!strcmp(s, "pick")) {   /* argv[3..6] = cx cy depth name(MAINICON | id): the RT_GROUP_ICON entry that fle_icon_group_pick chooses */
        uint64_t out[5], name;
        uint16_t *w;
        int ok;
        if (!strcmp(argv[6], "MAINICON")) { w = wide("MAINICON"); name = A(w); } else name = strtoull(argv[6], 0, 0);
        ok = fle_icon_group_pick("pick", 0x400000, name, (uint32_t)atoi(argv[3]), (uint32_t)atoi(argv[4]), (uint32_t)atoi(argv[5]), out);
        if (!ok) printf("none\n");
        else printf("w %llu h %llu depth %llu rt_icon %llu\n", U(out[1]), U(out[2]), U(out[3]), U(out[4]));
    } else if (!strcmp(s, "icon-stop")) {   /* argv[3] = hinstance, argv[4] = name (MAINICON | number) */
        uint64_t hi = strtoull(argv[3], 0, 0), name;
        uint16_t *w;
        if (!strcmp(argv[4], "MAINICON")) { w = wide("MAINICON"); name = A(w); }
        else if (!strcmp(argv[4], "OTHER")) { w = wide("OTHERICON"); name = A(w); }
        else name = strtoull(argv[4], 0, 0);
        prep(hi, name, 0, 0); fle_imp_LoadIconW();
        printf("returned %llu\n", U(R[0]));
    } else if (!strcmp(s, "iconinfo-stop")) {   /* argv[3] = hicon (after a LoadIconW of the application icon unless "x") */
        static uint8_t ii[32];
        uint64_t h = strtoull(argv[3], 0, 0);
        if (h == 1) { prep(0, 32512, 0, 0); fle_imp_LoadIconW(); h = R[0]; }
        prep(h, argv[4][0] == 'n' ? 0 : A(ii), 0, 0); fle_imp_GetIconInfo();
        printf("returned %llu\n", U(R[0]));
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
        let tag = etxt_common::per_process("wall-fr5-driver");
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
        cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt).arg("-I").arg(harness_dir());
        for p in &srcs {
            cmd.arg(p);
        }
        let cc = cmd.arg(harness_dir().join("image.c")).arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
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

// ---------------------------------------------------------------------------------------------
// 2. UuidCreate
// ---------------------------------------------------------------------------------------------

/// The independent implementation: SplitMix64 (Steele, Lea, Flood 2014) from the seed of the decision, two outputs per UUID, little-endian bytes, then the version and variant bits
/// of RFC 4122 section 4.4. Written here from the RFC and the paper's constants, not read back from fle_proc.c.
fn expected_uuid(k: usize) -> String {
    let mut st: u64 = 0x5555_5555_5555_5555;
    let mut next = || {
        st = st.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = st;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut b = [0u8; 16];
    for _ in 0..=k {
        let (lo, hi) = (next(), next());
        b[..8].copy_from_slice(&lo.to_le_bytes());
        b[8..].copy_from_slice(&hi.to_le_bytes());
    }
    b[7] = (b[7] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn uuid_create_writes_version_4_uuids_of_a_fixed_sequence_and_returns_rpc_s_ok() {
    let got = lines(&["uuid", "6"]);
    assert_eq!(got.len(), 6);
    let mut seen = std::collections::BTreeSet::new();
    for (k, l) in got.iter().enumerate() {
        // "<rc> <32 hex> le <last error>": RPC_S_OK = 0 and the last error is left as the caller set it (5)
        let f: Vec<&str> = l.split(' ').collect();
        assert_eq!((f[0], f[2], f[3]), ("0", "le", "5"), "{l}");
        assert_eq!(f[1], expected_uuid(k), "UUID #{k}");
        let b = f[1];
        // RFC 4122 4.4: byte 7 (the high byte of Data3 in memory) has the version 4 in its high nibble; byte 8 has the variant bits 10 in its top two
        assert_eq!(&b[14..15], "4", "version nibble of #{k}");
        assert!(matches!(&b[16..17], "8" | "9" | "a" | "b"), "variant of #{k}: {}", &b[16..17]);
        assert!(seen.insert(b.to_string()), "the same UUID twice");
    }
    // the same sequence in every run (no host randomness, clock or address)
    assert_eq!(lines(&["uuid", "6"]), got);
    // frozen first values (the same two computed by a Python script of the run record, which also gave 1c588f8c-dce4-4d23-bfd8-2e006150e3ef for the first one in the GUID layout)
    assert_eq!(expected_uuid(0), "8c8f581ce4dc234dbfd82e006150e3ef");
    assert_eq!(expected_uuid(1), "3f377f9c5238374eaa208dd13deccb21");
    // in the GUID layout of guiddef.h (Data1 and Data2 and Data3 little endian) the first one reads 1c588f8c-dce4-4d23-bfd8-2e006150e3ef
    let u = expected_uuid(0);
    let data1 = u32::from_le_bytes(hex(&u[0..8]).try_into().unwrap());
    let data2 = u16::from_le_bytes(hex(&u[8..12]).try_into().unwrap());
    let data3 = u16::from_le_bytes(hex(&u[12..16]).try_into().unwrap());
    assert_eq!(format!("{data1:08x}-{data2:04x}-{data3:04x}-{}-{}", &u[16..20], &u[20..32]), "1c588f8c-dce4-4d23-bfd8-2e006150e3ef");
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn uuid_create_with_a_null_pointer_stops_by_name() {
    aborts(&["uuid-null"], "UuidCreate: Uuid is NULL");
}

#[test]
fn rpcrt4_is_virtual_module_36_and_loads_by_every_spelling_and_only_it_has_uuid_create() {
    let got = lines(&["rpcrt4-module"]);
    let h = format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * 36);
    assert_eq!(
        got,
        [
            format!("{h} 5"),
            format!("{h} 5"),
            format!("{h} 5"),
            format!("count 41 name35 rpcrt4.dll handle35 {h}"), // decision 0238 appended kernelbase.dll: 40 modules (39 after) decision 0122 appended dwmapi.dll: 39 modules (decision 0119 appended wtsapi32.dll and uxtheme.dll: 38; old: count 36)
            "refs 3".to_string(), // started with 1 (the process loaded it), plus two LoadLibraryW calls
            "gpa-rpcrt4 found 5".to_string(),
            "gpa-kernel32 missing 127".to_string(),
        ]
    );
}

/// Decision 0118 took rpcrt4.dll out of the list of names this environment is defined to lack (decisions 0015, 0029): LoadLibrary looks in the module table first, so a name left in
/// both lists would still load; the list itself is checked here, so that the two cannot drift apart. Seven names remained after decision 0118; decision 0119 took wtsapi32.dll
/// and uxtheme.dll out as well (old: seven names; new: five), decision 0122 dwmapi.dll (four).
#[test]
fn rpcrt4_is_no_longer_in_the_list_of_lacking_names_and_the_four_others_are() {
    let t = std::fs::read_to_string(rt_dir().join("fle_load.c")).unwrap();
    let a = t.find("static const char *const fle_load_absent[] = {").expect("the list");
    let list = &t[a..a + t[a..].find("};").unwrap()];
    let names: Vec<&str> = list.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, s)| s).collect();
    // decision 0126 (wall 86, S-webview2-absent): "webview2loader.dll" is added (old: four names, windowscodecs.dll last); the guest's f_be3520 takes the NULL of LoadLibraryW("WebView2Loader.dll") and returns 0
    assert_eq!(names, ["imm32.dll", "msimg32.dll", "propsys.dll", "windowscodecs.dll", "webview2loader.dll"]); // decision 0122: dwmapi.dll is a module (old: five names, dwmapi.dll first)
    assert!(!names.contains(&"rpcrt4.dll"));
}

// ---------------------------------------------------------------------------------------------
// 3. LoadIconW (hInstance non-NULL) and GetIconInfo on the real DLL
// ---------------------------------------------------------------------------------------------

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

#[test]
fn mainicon_of_the_dll_is_the_32_by_32_32_bit_entry_with_one_shared_handle_and_get_icon_info_answers_it() {
    let Some(dll) = real_dll_path() else { return };
    let dll = dll.to_string_lossy().to_string();
    let got = lines(&["icon", &dll, "MAINICON"]);
    // The group of FLEngine's MAINICON has 13 entries (read by pefile): 32x32 at 4, 8 and 32 bits, and larger and smaller ones. Display depth 32 (fle_gdc.h), SM_CXICON = 32
    // (Learn "About Icons"): the 32x32 entry of 32 bits (RT_ICON id 10) is chosen. One handle for the same group of the same image (the LoadIcon page: "retrieves a handle to the
    // existing resource"): handle in the table's region (0x7FBA0000 + 4 * (slot + 1)), kind ICON (5). The table is fresh: slots 0..32 are the stock slots, the first created slot is 33.
    assert!(got[0].starts_with("handle 7fba0"), "{}", got[0]);
    assert!(got[0].contains(" same 1 kind 5 le 5"), "{}", got[0]);
    // live objects: the icon (1); no bitmap yet
    assert_eq!(got[1], "live 1 bitmaps 0");
    // GetIconInfo: fIcon 1, hot spot (16, 16) = the centre of the 32 x 32 icon (a reading of "always in the center"), two NEW bitmaps (a colour icon: a mask and a colour bitmap)
    let f: Vec<&str> = got[2].split(' ').collect();
    assert_eq!(&f[..8], ["GetIconInfo", "1", "fIcon", "1", "hot", "16", "16", "hbmMask"]);
    assert_ne!(f[8], "0");
    assert_eq!(f[9], "hbmColor");
    assert_ne!(f[10], "0");
    assert_ne!(f[8], f[10]);
    // the two bitmaps are live, of the BITMAP kind (7), and counted
    assert_eq!(got[3], "bitmaps 2 kinds 7 7");
    // GetObjectW on them: the mask 32 x 32, 1 plane, 1 bit; the colour bitmap 32 x 32, 1 plane, 32 bits (the virtual display's format, fle_gdc.h; the ICONINFO page: "the hbmMask
    // and hbmColor bitmaps are the same size, each of which is the size of the icon")
    assert_eq!(got[4], "mask GetObjectW 32 w 32 h 32 planes 1 bits 1");
    assert_eq!(got[5], "color GetObjectW 32 w 32 h 32 planes 1 bits 32");
    assert_eq!(got[6], "again mask differs 1");
}

/// (cx, cy, display depth) -> the entry of the group of MAINICON that LookupIconIdFromDirectoryEx's rules (Learn) choose: the largest entry that does not exceed the size; of those
/// the entry whose depth is the display's; else the greatest depth below it; else (every one exceeds it) the lowest. The entries (read by pefile and by the test below):
/// 16x16: 4 bit (RT_ICON 2), 8 bit (5), 32 bit (13); 20x20: 32 bit (12); 24x24: 32 bit (11); 32x32: 4 bit (1), 8 bit (4), 32 bit (10); 40x40: 32 bit (9);
/// 48x48: 8 bit (3), 32 bit (8); 64x64: 32 bit (7); 256x256: 32 bit (6).
#[test]
fn the_entry_is_chosen_by_the_rules_of_lookup_icon_id_from_directory_ex() {
    let Some(dll) = real_dll_path() else { return };
    let dll = dll.to_string_lossy().to_string();
    // Decision 0121 (the triage of the failure that decision 0119 reported): the 256 x 256 entry of this group (RT_ICON 6, ICONDIR bytes 262,184) is stored PNG-compressed (the first 8 bytes of
    // the resource are 89 50 4e 47 0d 0a 1a 0a, 48,775 bytes; read with pefile), and fle_icon_group_pick stops on a PNG-compressed component by name (fle_res.c: "a PNG-compressed component
    // (the Vista form) is not read and stops"). The earlier expectation "w 256 h 256 depth 32 rt_icon 6" for the two requests of 256 and 1000 could never be met with the real DLL (it was met only
    // when this test was skipped, FLENGINE_PE unset and no binaries/ in the tree). Old: those two cases expected the 256 x 256 entry; new: they expect the named stop, and a request that the
    // 64 x 64 entry (the largest that is not PNG) satisfies is checked at 64 (RT_ICON 7). The other twelve cases are unchanged.
    let cases: [(&str, &str, &str, &str); 13] = [
        ("32", "32", "32", "w 32 h 32 depth 32 rt_icon 10"), // exact size, exact depth
        ("32", "32", "8", "w 32 h 32 depth 8 rt_icon 4"),    // exact size, exact depth (the 8 bit entry)
        ("32", "32", "4", "w 32 h 32 depth 4 rt_icon 1"),
        ("32", "32", "16", "w 32 h 32 depth 8 rt_icon 4"),   // no 16 bit entry: the greatest depth that does not exceed the display's
        ("32", "32", "2", "w 32 h 32 depth 4 rt_icon 1"),    // every depth exceeds the display's: the lowest
        ("32", "32", "64", "w 32 h 32 depth 32 rt_icon 10"), // the display is deeper than every entry: the greatest
        ("50", "50", "32", "w 48 h 48 depth 32 rt_icon 8"),  // the largest that does not exceed 50 x 50
        ("47", "47", "32", "w 40 h 40 depth 32 rt_icon 9"),
        ("20", "20", "32", "w 20 h 20 depth 32 rt_icon 12"),
        ("24", "24", "8", "w 24 h 24 depth 32 rt_icon 11"),  // only a 32 bit 24x24 entry: it exceeds the display's depth, and it is the lowest there is
        ("100", "40", "32", "w 40 h 40 depth 32 rt_icon 9"), // a non square request: width <= 100 and height <= 40
        ("64", "64", "32", "w 64 h 64 depth 32 rt_icon 7"),  // the largest entry that is not PNG-compressed
        ("16", "16", "32", "w 16 h 16 depth 32 rt_icon 13"),
    ];
    for (cx, cy, d, want) in cases {
        assert_eq!(lines(&["pick", &dll, cx, cy, d, "MAINICON"]), [want], "request {cx}x{cy} at {d} bits");
    }
    // the 256 x 256 entry would be chosen for a request of 256 or more (ICONRESDIR: 0 means 256): it is PNG-compressed, so the run stops by name instead of reading it
    for (cx, cy) in [("256", "256"), ("1000", "1000")] {
        aborts(&["pick", &dll, cx, cy, "32", "MAINICON"], "the chosen RT_ICON is PNG-compressed: not read");
    }
    // every entry exceeds the request: the page gives no rule, so it stops; no group of that name: returns 0 to the caller
    aborts(&["pick", &dll, "15", "15", "32", "MAINICON"], "every entry of the RT_GROUP_ICON exceeds the requested size");
    assert_eq!(lines(&["pick", &dll, "32", "32", "32", "7"]), ["none"]);
}

#[test]
fn the_icon_choice_rules_stop_instead_of_guessing() {
    let Some(dll) = real_dll_path() else { return };
    let dll = dll.to_string_lossy().to_string();
    // a name the module has no RT_GROUP_ICON for: the page names no error code for a failed load, so it stops (no NULL and no last error is made up)
    aborts(&["icon-stop", &dll, "0x400000", "OTHER"], "the module has no RT_GROUP_ICON of that name");
    aborts(&["icon-stop", &dll, "0x400000", "7"], "the module has no RT_GROUP_ICON of that name");
    // a module handle that is not an image of this runtime (no resources) still stops by name, from the resource reader
    aborts(&["icon-stop", &dll, "0x12345678", "MAINICON"], "is not a module handle of this runtime");
    // the application icon (hInstance NULL) is unchanged
    let o = run(&["icon-stop", &dll, "0", "32512"]);
    assert!(o.ok, "{}", o.stderr);
    assert!(o.stdout.starts_with("returned ") && o.stdout.trim() != "returned 0", "{}", o.stdout);
}

#[test]
fn get_icon_info_still_stops_for_the_application_icon_null_and_a_null_struct() {
    let Some(dll) = real_dll_path() else { return };
    let dll = dll.to_string_lossy().to_string();
    // LoadIconW(NULL, IDI_APPLICATION) is a shared icon without a module group: not answered (decision 0111 stopped here; decision 0118 answers only the icon of a module group)
    aborts(&["iconinfo-stop", &dll, "1", "x"], "not an icon or cursor made by CreateIconIndirect");
    aborts(&["iconinfo-stop", &dll, "0", "x"], "not an icon or cursor made by CreateIconIndirect");
}

// ---------------------------------------------------------------------------------------------
// 4. GetModuleFileNameW/A(NULL)
// ---------------------------------------------------------------------------------------------

#[test]
fn get_module_file_name_of_null_is_the_host_exe_and_of_0x400000_the_dll_and_get_module_handle_of_null_is_unchanged() {
    let exe = "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe";
    let dll = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
    let got = lines(&["gmfn", "0"]);
    assert_eq!(got, [format!("W 49 {exe}"), format!("A 49 {exe}"), "GetModuleHandleW(NULL) 400000".to_string()]);
    let got = lines(&["gmfn", "0x400000"]);
    assert_eq!(got, [format!("W 57 {dll}"), format!("A 57 {dll}"), "GetModuleHandleW(NULL) 400000".to_string()]);
    // the exe path is the same string as the FL64.exe row of the claims table and of the registry seed (decision 0117)
    let tsv = std::fs::read_to_string(harness_dir().join("seeds/authenticode-claims.tsv")).unwrap();
    assert!(tsv.lines().any(|l| l.starts_with("FL64.exe\t")), "the claims table has the FL64.exe row");
    let reg = std::fs::read_to_string(harness_dir().join("seeds/reg-seed-nsis.tsv")).unwrap();
    assert!(reg.contains(exe), "the registry seed names the same exe path");
    // the path of the exe has the directory of the DLL path (decision 0018) and the file name FL64.exe
    assert_eq!(&exe[..exe.rfind('\\').unwrap()], &dll[..dll.rfind('\\').unwrap()]);
}

#[test]
fn the_exe_path_is_defined_once_and_the_dll_path_keeps_its_definition() {
    let env = std::fs::read_to_string(rt_dir().join("fle_env.h")).unwrap();
    assert!(env.contains("#define FLE_VIRTUAL_EXE_PATH    \"C:\\\\Program Files\\\\Image-Line\\\\FL Studio 21\\\\FL64.exe\""));
    assert!(env.contains("#define FLE_VIRTUAL_DLL_PATH    \"C:\\\\Program Files\\\\Image-Line\\\\FL Studio 21\\\\FLEngine_x64.dll\""));
    let vfs = std::fs::read_to_string(rt_dir().join("fle_vfs.c")).unwrap();
    assert!(vfs.contains("if (h == 0) return FLE_VIRTUAL_EXE_PATH;"));
    assert!(vfs.contains("if (h == FLE_EXE_HANDLE) return FLE_VIRTUAL_DLL_PATH;"));
}

// ---------------------------------------------------------------------------------------------
// 5. The bytes of the real FLEngine_x64.dll that the decision rests on
// ---------------------------------------------------------------------------------------------

fn real_dll() -> Option<Vec<u8>> {
    std::fs::read(manifest().join("../../../binaries/win64/FLEngine_x64.dll")).ok()
}

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn rd16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}

fn va_to_off(dll: &[u8], va: u64) -> Option<usize> {
    let lf = rd32(dll, 0x3c) as usize;
    let nsec = rd16(dll, lf + 6) as usize;
    let optsz = rd16(dll, lf + 20) as usize;
    let rva = va.checked_sub(0x40_0000)? as u32;
    for k in 0..nsec {
        let s = lf + 24 + optsz + 40 * k;
        let vsz = rd32(dll, s + 8);
        let vrva = rd32(dll, s + 12);
        let raw = rd32(dll, s + 20);
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

fn cstr_at(dll: &[u8], rva: u32) -> String {
    let off = va_to_off(dll, 0x40_0000 + rva as u64).unwrap();
    let end = off + dll[off..].iter().position(|&b| b == 0).unwrap();
    String::from_utf8_lossy(&dll[off..end]).to_string()
}

fn import_of_slot(dll: &[u8], slot_va: u64) -> Option<(String, String)> {
    let lf = rd32(dll, 0x3c) as usize;
    let dir = rd32(dll, lf + 24 + 112 + 8) as u64;
    let mut d = va_to_off(dll, 0x40_0000 + dir)?;
    loop {
        let (oft, name, ft) = (rd32(dll, d), rd32(dll, d + 12), rd32(dll, d + 16));
        if name == 0 {
            return None;
        }
        let thunks = if oft != 0 { oft } else { ft };
        let mut i = 0u64;
        loop {
            let to = va_to_off(dll, 0x40_0000 + thunks as u64 + 8 * i)?;
            let ent = u64::from_le_bytes(dll[to..to + 8].try_into().unwrap());
            if ent == 0 {
                break;
            }
            if 0x40_0000 + ft as u64 + 8 * i == slot_va && ent >> 63 == 0 {
                return Some((cstr_at(dll, name).to_ascii_lowercase(), cstr_at(dll, ent as u32 + 2)));
            }
            i += 1;
        }
        d += 20;
    }
}

fn thunk_import(dll: &[u8], va: u64) -> (String, String) {
    let b = bytes_at(dll, va, 6);
    assert_eq!(&b[..2], &[0xff, 0x25], "thunk {va:#x} is not jmp [rip+disp]");
    let disp = i32::from_le_bytes(b[2..6].try_into().unwrap()) as i64;
    import_of_slot(dll, (va as i64 + 6 + disp) as u64).unwrap_or_else(|| panic!("no import at the slot of thunk {va:#x}"))
}

#[test]
fn the_cpu_check_the_uuid_the_icon_and_the_window_calls_of_create_fruity_instance_are_where_the_decision_says() {
    let Some(dll) = real_dll() else { return };
    // CPU feature word: f_639a20 sets 0x10 from EDX bit 15 (test rax,0x8000; or [..],0x10) and, when EDX bit 24 (FXSR) is set (test rax,0x1000000), 0x40 and then calls the small function
    // 0x6399d0 (mov al,1; ret), whose result sets 0x20000. These two bytes sequences are the "+2" of decision 0118: the only new function entries a4 gets from the FXSR bit.
    assert_eq!(bytes_at(&dll, 0x639b3d, 6), [0x48, 0xa9, 0x00, 0x00, 0x00, 0x01]);
    assert_eq!(bytes_at(&dll, 0x639b4c, 5), [0xe8, 0x7f, 0xfe, 0xff, 0xff]);
    assert_eq!(bytes_at(&dll, 0x6399d0, 3), [0xb0, 0x01, 0xc3]);
    // CMOV: test rax,0x8000 at 0x639b2a (the flag the engine's own check needs)
    assert_eq!(bytes_at(&dll, 0x639b2a, 6), [0x48, 0xa9, 0x00, 0x80, 0x00, 0x00]);
    // The delay-loaded UuidCreate: the delay-load name table has "rpcrt4.dll" (0x197700a) and "UuidCreate" (0x1977018)
    let rp: Vec<u8> = b"rpcrt4.dll\0".to_vec();
    assert_eq!(bytes_at(&dll, 0x197700a, rp.len()), rp);
    let uc: Vec<u8> = b"UuidCreate\0".to_vec();
    assert_eq!(bytes_at(&dll, 0x1977018, uc.len()), uc);
    // the icon: CreateFruityInstance calls LoadIconW through the thunk 0x4250f0 with the module handle [0x15ec298] and L"MAINICON" (0x13767f8)
    assert_eq!(thunk_import(&dll, 0x4250f0), ("user32.dll".to_string(), "LoadIconW".to_string()));
    assert_eq!(thunk_import(&dll, 0x424d30), ("user32.dll".to_string(), "GetIconInfo".to_string()));
    let name: Vec<u8> = "MAINICON\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(bytes_at(&dll, 0x13767f8, name.len()), name);
    assert_eq!(bytes_at(&dll, 0x13764d8, 7), [0x48, 0x8b, 0x0d, 0xb9, 0x5d, 0x27, 0x00]);
    // the next walls (not answered): SendMessageW (thunk 0x425390) with WM_SETICON (mov edx,0x80 at 0x86b6a4) and SetClassLongPtrW (thunk 0x4259a0) with GCLP_HICON (-14,
    // mov edx,0xfffffff2 at 0x86b6c9), then CreateWindowExW (thunk 0x425780) with dwStyle 0x84ca0000 (mov r9d,0x84ca0000 at 0x867f1c), class "TApplication" (0x8675f0)
    assert_eq!(thunk_import(&dll, 0x425390), ("user32.dll".to_string(), "SendMessageW".to_string()));
    assert_eq!(thunk_import(&dll, 0x4259a0), ("user32.dll".to_string(), "SetClassLongPtrW".to_string()));
    assert_eq!(thunk_import(&dll, 0x425780), ("user32.dll".to_string(), "CreateWindowExW".to_string()));
    assert_eq!(bytes_at(&dll, 0x86b6a4, 5), [0xba, 0x80, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x86b6c9, 5), [0xba, 0xf2, 0xff, 0xff, 0xff]);
    assert_eq!(bytes_at(&dll, 0x867f1c, 6), [0x41, 0xb9, 0x00, 0x00, 0xca, 0x84]);
    let cls: Vec<u8> = "TApplication\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(bytes_at(&dll, 0x8675f0, cls.len()), cls);
}

#[test]
fn the_group_of_mainicon_has_13_entries_and_the_32_by_32_32_bit_one_names_rt_icon_10() {
    let Some(dll) = real_dll() else { return };
    // The RT_GROUP_ICON named MAINICON: NEWHEADER (0, 1, 13) then 13 RESDIR of 14 bytes. Found by walking the resource directory here (not through the runtime): type 14, the
    // name entry "MAINICON".
    let lf = rd32(&dll, 0x3c) as usize;
    let rsrc_rva = rd32(&dll, lf + 24 + 112 + 2 * 8) as u64;
    let base = va_to_off(&dll, 0x40_0000 + rsrc_rva).unwrap();
    let dir = |off: usize| -> (usize, usize) { (rd16(&dll, base + off + 12) as usize, rd16(&dll, base + off + 14) as usize) };
    let (nn, ni) = dir(0);
    let mut grp = None;
    for i in 0..nn + ni {
        let (id, to) = (rd32(&dll, base + 16 + 8 * i), rd32(&dll, base + 20 + 8 * i));
        if id & 0x8000_0000 == 0 && id == 14 {
            grp = Some((to & 0x7fff_ffff) as usize);
        }
    }
    let t = grp.expect("RT_GROUP_ICON level");
    let (n2, i2) = dir(t);
    assert_eq!((n2, i2), (1, 0), "one name entry (MAINICON), no id entry");
    let (nameoff, lang) = (rd32(&dll, base + t + 16), rd32(&dll, base + t + 20));
    let nm = base + (nameoff & 0x7fff_ffff) as usize;
    let len = rd16(&dll, nm) as usize;
    let s: String = (0..len).map(|k| char::from_u32(rd16(&dll, nm + 2 + 2 * k) as u32).unwrap()).collect();
    assert_eq!(s, "MAINICON");
    let l = (lang & 0x7fff_ffff) as usize;
    let leaf = rd32(&dll, base + l + 20) as usize;
    let (drva, dsz) = (rd32(&dll, base + leaf), rd32(&dll, base + leaf + 4));
    assert_eq!(dsz, 6 + 14 * 13);
    let o = va_to_off(&dll, 0x40_0000 + drva as u64).unwrap();
    assert_eq!((rd16(&dll, o), rd16(&dll, o + 2), rd16(&dll, o + 4)), (0, 1, 13));
    let mut ent = Vec::new();
    for k in 0..13 {
        let e = o + 6 + 14 * k;
        let (w, h) = (dll[e] as u32, dll[e + 1] as u32);
        ent.push((if w == 0 { 256 } else { w }, if h == 0 { 256 } else { h }, rd16(&dll, e + 4), rd16(&dll, e + 6), rd32(&dll, e + 8), rd16(&dll, e + 12)));
    }
    // (width, height, planes, bit count, bytes in res, RT_ICON id)
    assert_eq!(ent[0], (32, 32, 1, 4, 744, 1));
    assert_eq!(ent[3], (32, 32, 1, 8, 2216, 4));
    assert_eq!(ent[9], (32, 32, 1, 32, 4264, 10));
    assert_eq!(ent.iter().filter(|e| e.0 == 32 && e.1 == 32).count(), 3, "three 32x32 entries (4, 8 and 32 bits): the depth decides");
    assert!(ent.iter().all(|e| e.2 == 1), "every entry has one plane");
}
