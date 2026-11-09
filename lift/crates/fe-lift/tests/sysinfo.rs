//! Tests for the system-information answers in runtime/fle_sysinfo.c (docs/decisions/0034-sysinfo.md; the pages read
//! and what they do not say: docs/investigations/sysinfo-run.md): GetNativeSystemInfo,
//! GetLogicalProcessorInformation, IsWow64Process, GlobalMemoryStatus, GetProcessAffinityMask,
//! SetProcessAffinityMask.
//!
//! Expected values are written here from the rules on the Microsoft pages (GetNativeSystemInfo, SYSTEM_INFO,
//! GetLogicalProcessorInformation, SYSTEM_LOGICAL_PROCESSOR_INFORMATION, CACHE_DESCRIPTOR, IsWow64Process,
//! GlobalMemoryStatus, MEMORYSTATUS, GetProcessAffinityMask, SetProcessAffinityMask), not read back from the C code.
//! A value that no page fixes is a setting of the headless virtual environment (not a real machine's value); the
//! tests that pin such a value say "setting" next to it. Struct layouts are mirrored in the test programs with natural
//! alignment. The host is Linux: nothing is compared with a real Windows.
//!
//! The C sources come from runtime/ when fle_sysinfo.c is there, otherwise from out-sysinfo/work/ (development).
//! The test that links every runtime source (the GetProcAddress path) uses runtime/ only.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_sysinfo.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-sysinfo/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-sysinfo/test");
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

/// fle_sysinfo.c (warnings are errors) plus the files it links with.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-sysinfo"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in
            [("fle_sysinfo.c", true), ("fle_proc.c", false), ("fle_sys.c", false), ("fle_imports.c", false), ("fle_env.c", false)]
        {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

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
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_sysinfo.h"

uint64_t R[16];

static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    R[0] = 0x99;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[4]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
#define PSEUDO UINT64_C(0xFFFFFFFFFFFFFFFF)

/* SYSTEM_INFO (48 bytes on x64) */
typedef struct {
    union { uint32_t dwOemId; struct { uint16_t wProcessorArchitecture; uint16_t wReserved; } s; } u;
    uint32_t dwPageSize;
    void *lpMinimumApplicationAddress;
    void *lpMaximumApplicationAddress;
    uint64_t dwActiveProcessorMask;
    uint32_t dwNumberOfProcessors;
    uint32_t dwProcessorType;
    uint32_t dwAllocationGranularity;
    uint16_t wProcessorLevel;
    uint16_t wProcessorRevision;
} SI;

/* MEMORYSTATUS (56 bytes on x64) */
typedef struct {
    uint32_t dwLength;
    uint32_t dwMemoryLoad;
    uint64_t dwTotalPhys;
    uint64_t dwAvailPhys;
    uint64_t dwTotalPageFile;
    uint64_t dwAvailPageFile;
    uint64_t dwTotalVirtual;
    uint64_t dwAvailVirtual;
} MS;

/* SYSTEM_LOGICAL_PROCESSOR_INFORMATION (32 bytes on x64) */
typedef struct {
    uint64_t ProcessorMask;
    uint32_t Relationship;
    union {
        struct { uint8_t Flags; } ProcessorCore;
        struct { uint32_t NodeNumber; } NumaNode;
        struct { uint8_t Level; uint8_t Associativity; uint16_t LineSize; uint32_t Size; uint32_t Type; } Cache;
        uint64_t Reserved[2];
    } u;
} SLPI;
"#;

fn run_src(name: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message and no output after it.
fn assert_aborts(prefix: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{stmts}\n    printf(\"returned\\n\");");
        let o = run_src(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

const ROWS: [&str; 6] = [
    "GetNativeSystemInfo",
    "GetLogicalProcessorInformation",
    "IsWow64Process",
    "GlobalMemoryStatus",
    "GetProcessAffinityMask",
    "SetProcessAffinityMask",
];

// ---- 1. Table, header, generator ----

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_sysinfo.h")).unwrap();
    for n in ROWS {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void);")), "{n} is declared in fle_sysinfo.h");
    }
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

/// A PE with kernel32.dll imports `names` (at most 16). Entry 0x1000 calls the thunks at 0x1100 + 0x10 * k, each `jmp [IAT slot k]`.
fn build(names: &[&str]) -> Vec<u8> {
    assert!(names.len() <= 16);
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

    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    for (k, n) in names.iter().enumerate() {
        let name_rva = 0x2400 + 0x20 * k as u32;
        let mut hn = vec![0u8, 0u8];
        hn.extend_from_slice(n.as_bytes());
        hn.push(0);
        put(&mut b, name_rva, &hn);
        w64(&mut b, data_at(0x2100 + 8 * k as u32), name_rva as u64);
        w64(&mut b, data_at(0x2200 + 8 * k as u32), name_rva as u64);
    }

    let t = TEXT_RAW;
    let mut code: Vec<(u32, Vec<u8>)> = Vec::new();
    for k in 0..names.len() {
        let at = 0x1000 + 5 * k as u32;
        let th = 0x1100 + 0x10 * k as u32;
        code.push((at, [&[0xe8][..], &rel(at + 5, th)].concat()));
        code.push((th, [&[0xff, 0x25][..], &rel(th + 6, 0x2200 + 8 * k as u32)].concat()));
    }
    code.push((0x1000 + 5 * names.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen(names: &[&str]) -> (fe_lift::Stats, String) {
    let img = build(names);
    let mut out = Vec::new();
    let st = fe_lift::generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

#[test]
fn thunks_call_the_runtime_and_rows_are_in_the_response_table() {
    let mut names: Vec<&str> = ROWS.to_vec();
    names.push("GetProcAddress");
    let (st, c) = gen(&names);
    for n in ROWS {
        assert_eq!(st.handled[format!("jmp_imp:{n}").as_str()], 1, "{n}");
        assert!(c.contains(&format!("fle_imp_{n}(); return;")), "{n}: the thunk calls the runtime");
        assert!(c.contains(&format!("{{ \"kernel32.dll\", \"{n}\", ")), "{n} in fle_resp_table");
    }
    // The row count is whatever the table holds (other work adds rows): only a lower bound that covers these rows.
    assert!(st.resp_entries >= ROWS.len() + 1, "{}", st.resp_entries);
}

/// With the real DLL and the all-pdata seeds: the IAT thunks of the normal import table are handled thunks,
/// and every row is in the response table (skipped when the DLL is absent). The delay-loaded names
/// (GetNativeSystemInfo, GetLogicalProcessorInformation, IsWow64Process) are reached through GetProcAddress by the
/// delay-load helper, so they have no jmp_imp thunk.
#[test]
fn real_dll_handles_the_normal_import_rows_and_has_all_rows_in_the_table() {
    let p = manifest().join("../../../binaries/win64/FLEngine_x64.dll");
    let Ok(data) = std::fs::read(p) else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let seeds = fe_decode::pdata_seeds(&pe).unwrap();
    assert!(seeds.len() > 50_000, "{}", seeds.len());
    // The C is large (about 200 MB): count it without keeping it, and find the rows with a counting writer.
    struct Scan {
        tail: Vec<u8>,
        found: Vec<bool>,
    }
    impl std::io::Write for Scan {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.tail.extend_from_slice(buf);
            for (i, n) in ROWS.iter().enumerate() {
                let pat = format!("{{ \"kernel32.dll\", \"{n}\", ");
                if !self.found[i] && self.tail.windows(pat.len()).any(|w| w == pat.as_bytes()) {
                    self.found[i] = true;
                }
            }
            let keep = self.tail.len().saturating_sub(128);
            self.tail.drain(..keep);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut w = Scan { tail: Vec::new(), found: vec![false; ROWS.len()] };
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut w).unwrap();
    for n in ["GlobalMemoryStatus", "GetProcessAffinityMask", "SetProcessAffinityMask"] {
        assert!(st.handled.get(format!("jmp_imp:{n}").as_str()).copied().unwrap_or(0) >= 1, "{n} is a handled thunk");
    }
    for (i, n) in ROWS.iter().enumerate() {
        assert!(w.found[i], "{n} in fle_resp_table");
    }
}

// ---- 2. GetNativeSystemInfo ----

#[test]
fn native_system_info_equals_system_info_and_has_the_fields_the_page_fixes() {
    let l = lines(
        "sysinfo-native",
        r#"
    SI a, b;
    memset(&a, 0xEE, sizeof a);
    memset(&b, 0xDD, sizeof b);
    CALL(fle_imp_GetSystemInfo, A(&a), 0, 0, 0);
    uint64_t r0 = CALL(fle_imp_GetNativeSystemInfo, A(&b), 0, 0, 0);
    printf("size %zu\n", sizeof b);
    printf("same_bytes_as_getsysteminfo %d\n", memcmp(&a, &b, sizeof a) == 0);
    printf("r0_kept %llu\n", U(r0));
    printf("arch %u\n", b.u.s.wProcessorArchitecture);
    printf("page %u\n", b.dwPageSize);
    printf("gran %u\n", b.dwAllocationGranularity);
    printf("type %u\n", b.dwProcessorType);
    printf("min_lt_max %d\n", (uint64_t)b.lpMinimumApplicationAddress < (uint64_t)b.lpMaximumApplicationAddress);
    printf("mask_matches_count %d\n", __builtin_popcountll(b.dwActiveProcessorMask) == (int)b.dwNumberOfProcessors);
    printf("nproc_nonzero %d\n", b.dwNumberOfProcessors != 0);
"#,
    );
    assert_eq!(
        l,
        [
            "size 48",
            // VOID: R[0] keeps the caller's 0x99 = 153
            "same_bytes_as_getsysteminfo 1",
            "r0_kept 153",
            // SYSTEM_INFO page: PROCESSOR_ARCHITECTURE_AMD64 = 9; PROCESSOR_AMD_X8664 = 8664; the page ties the page size to
            // VirtualAlloc's page, 4096 here (the host page size, which fle_sys.c checks)
            "arch 9",
            "page 4096",
            // setting: the page gives no number for the allocation granularity; 65536 is the value of the environment
            "gran 65536",
            "type 8664",
            "min_lt_max 1",
            // SYSTEM_INFO page: "dwActiveProcessorMask: bit 0 is processor 0"; the count is the number of set bits here
            "mask_matches_count 1",
            "nproc_nonzero 1",
        ]
    );
}

#[test]
fn native_system_info_environment_settings() {
    // setting (decision 0034, not a real machine's values): one logical processor, mask 1.
    let l = lines(
        "sysinfo-settings",
        r#"
    SI b;
    CALL(fle_imp_GetNativeSystemInfo, A(&b), 0, 0, 0);
    printf("nproc %u\n", b.dwNumberOfProcessors);
    printf("mask %llx\n", U(b.dwActiveProcessorMask));
    printf("sysinfo_mask_macro %llx\n", U(FLE_SYSINFO_PROC_MASK));
"#,
    );
    assert_eq!(l, ["nproc 1", "mask 1", "sysinfo_mask_macro 1"]);
}

// ---- 3. GetLogicalProcessorInformation ----

#[test]
fn logical_processor_information_buffer_protocol() {
    let l = lines(
        "glpi-protocol",
        r#"
    static SLPI buf[16];
    uint32_t len = 0;
    uint64_t ok;
    uint32_t need;

    /* size query: Buffer NULL, length 0 -> FALSE, ERROR_INSUFFICIENT_BUFFER, required length */
    fle_set_last_error(5);
    ok = CALL(fle_imp_GetLogicalProcessorInformation, 0, A(&len), 0, 0);
    printf("query ok %llu le %u\n", U(ok), LE());
    need = len;
    printf("need_multiple_of_struct %d need_nonzero %d\n", need % sizeof(SLPI) == 0, need != 0);
    printf("need %u\n", need);

    /* too small, with a Buffer */
    len = need - 1;
    fle_set_last_error(5);
    ok = CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    printf("small ok %llu le %u len_is_need %d\n", U(ok), LE(), len == need);

    len = 1;
    ok = CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    printf("tiny ok %llu len_is_need %d\n", U(ok), len == need);

    /* exactly enough: TRUE, length = bytes returned; the last error is not touched by a success */
    len = need;
    fle_set_last_error(5);
    ok = CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    printf("exact ok %llu le %u len_is_need %d\n", U(ok), LE(), len == need);

    /* more than enough: TRUE and the length is set to the bytes returned, not left at the input value */
    len = sizeof buf;
    ok = CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    printf("large ok %llu len_is_need %d\n", U(ok), len == need);
"#,
    );
    assert_eq!(
        l,
        [
            // GetLogicalProcessorInformation page: too small -> FALSE, ERROR_INSUFFICIENT_BUFFER (122), ReturnLength = required length
            "query ok 0 le 122",
            "need_multiple_of_struct 1 need_nonzero 1",
            // setting: 7 records of 32 bytes (fle_sysinfo.h)
            "need 224",
            "small ok 0 le 122 len_is_need 1",
            "tiny ok 0 len_is_need 1",
            // success: TRUE, ReturnLength = bytes returned; the last error stays 5
            "exact ok 1 le 5 len_is_need 1",
            "large ok 1 len_is_need 1",
        ]
    );
}

#[test]
fn logical_processor_information_records_follow_the_pages() {
    let l = lines(
        "glpi-records",
        r#"
    static SLPI buf[16];
    uint32_t len = 0;
    uint64_t sysmask, procmask;
    SI si;
    unsigned i, n, core = 0, numa = 0, cache = 0, pkg = 0, bad = 0, logical = 0, l1 = 0, l2 = 0, l3 = 0, badcache = 0;

    CALL(fle_imp_GetNativeSystemInfo, A(&si), 0, 0, 0);
    CALL(fle_imp_GetProcessAffinityMask, PSEUDO, A(&procmask), A(&sysmask), 0);
    CALL(fle_imp_GetLogicalProcessorInformation, 0, A(&len), 0, 0);
    len = sizeof buf;
    CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    n = len / sizeof(SLPI);
    for (i = 0; i < n; i++) {
        SLPI *r = &buf[i];
        /* page: "At least one bit will be set"; a mask names only processors of the system */
        if (r->ProcessorMask == 0 || (r->ProcessorMask & ~sysmask) != 0) bad++;
        switch (r->Relationship) {
        case 0:
            core++;
            logical += __builtin_popcountll(r->ProcessorMask);
            if (r->u.ProcessorCore.Flags > 1) bad++;
            break;
        case 1:
            numa++;
            if (r->u.NumaNode.NodeNumber != 0) bad++;
            break;
        case 2:
            cache++;
            if (r->u.Cache.Level == 1) l1++; else if (r->u.Cache.Level == 2) l2++; else if (r->u.Cache.Level == 3) l3++; else badcache++;
            if (r->u.Cache.Type > 3 || r->u.Cache.Size == 0 || r->u.Cache.LineSize == 0) badcache++;
            break;
        case 3:
            pkg++;
            break;
        default:
            bad++;
        }
    }
    printf("struct_size %zu\n", sizeof(SLPI));
    printf("records %u\n", n);
    printf("bad %u badcache %u\n", bad, badcache);
    /* the MSDN example's own count of logical processors equals SYSTEM_INFO's dwNumberOfProcessors */
    printf("logical_equals_si %d\n", logical == si.dwNumberOfProcessors);
    printf("system_mask_equals_si_mask %d\n", sysmask == si.dwActiveProcessorMask);
    /* page: a non-NUMA multiprocessor system reports one NUMA node, whose number is 0 */
    printf("numa %u\n", numa);
    printf("core %u pkg %u\n", core, pkg);
    printf("caches %u l1 %u l2 %u l3 %u\n", cache, l1, l2, l3);
"#,
    );
    assert_eq!(
        l,
        [
            // 8 (ULONG_PTR) + 4 (enum) + 4 (padding) + 16 (union)
            "struct_size 32",
            // setting: core, NUMA node, L1 data, L1 instruction, L2, L3, package
            "records 7",
            "bad 0 badcache 0",
            "logical_equals_si 1",
            "system_mask_equals_si_mask 1",
            "numa 1",
            "core 1 pkg 1",
            "caches 4 l1 2 l2 1 l3 1",
        ]
    );
}

#[test]
fn logical_processor_information_cache_values_are_the_settings() {
    // setting (decision 0034): the cache sizes are not on any page; they are the environment's values.
    let l = lines(
        "glpi-cache",
        r#"
    static SLPI buf[16];
    uint32_t len = sizeof buf;
    unsigned i, n;
    CALL(fle_imp_GetLogicalProcessorInformation, A(buf), A(&len), 0, 0);
    n = len / sizeof(SLPI);
    for (i = 0; i < n; i++)
        if (buf[i].Relationship == 2)
            printf("L%u type %u size %u assoc %u line %u\n", buf[i].u.Cache.Level, buf[i].u.Cache.Type, buf[i].u.Cache.Size,
                   buf[i].u.Cache.Associativity, buf[i].u.Cache.LineSize);
"#,
    );
    // Type: CacheUnified 0, CacheInstruction 1, CacheData 2 (PROCESSOR_CACHE_TYPE page)
    assert_eq!(
        l,
        [
            "L1 type 2 size 32768 assoc 8 line 64",
            "L1 type 1 size 32768 assoc 8 line 64",
            "L2 type 0 size 262144 assoc 8 line 64",
            "L3 type 0 size 8388608 assoc 16 line 64",
        ]
    );
}

// ---- 4. IsWow64Process ----

#[test]
fn is_wow64_process_says_false_for_the_current_process() {
    let l = lines(
        "wow64",
        r#"
    uint64_t out = 0xEEEEEEEEEEEEEEEEull, ok;
    fle_imp_GetCurrentProcess();
    ok = CALL(fle_imp_IsWow64Process, R[0], A(&out), 0, 0);
    /* BOOL is 4 bytes: the upper half of the 8 bytes is not touched */
    printf("ok %llu value %u upper %x\n", U(ok), (uint32_t)out, (uint32_t)(out >> 32));
    printf("pseudo_handle %llx\n", U(PSEUDO));
"#,
    );
    // IsWow64Process page: a 64-bit application under 64-bit Windows -> FALSE; success is nonzero
    assert_eq!(l, ["ok 1 value 0 upper eeeeeeee", "pseudo_handle ffffffffffffffff"]);
}

// ---- 5. GlobalMemoryStatus ----

#[test]
fn global_memory_status_fields_are_consistent_with_the_page() {
    let l = lines(
        "memstatus",
        r#"
    MS m;
    SI si;
    uint64_t r0;
    memset(&m, 0xEE, sizeof m);
    CALL(fle_imp_GetNativeSystemInfo, A(&si), 0, 0, 0);
    r0 = CALL(fle_imp_GlobalMemoryStatus, A(&m), 0, 0, 0);
    printf("size %zu\n", sizeof m);
    printf("r0_kept %llu\n", U(r0));
    /* MEMORYSTATUS page: "You do not need to set this member ... the function sets it" */
    printf("length_is_size %d\n", m.dwLength == sizeof m);
    printf("load_0_100 %d\n", m.dwMemoryLoad <= 100);
    printf("avail_le_total_phys %d\n", m.dwAvailPhys <= m.dwTotalPhys);
    printf("avail_le_total_pagefile %d\n", m.dwAvailPageFile <= m.dwTotalPageFile);
    printf("avail_le_total_virtual %d\n", m.dwAvailVirtual <= m.dwTotalVirtual);
    printf("total_phys_nonzero %d\n", m.dwTotalPhys != 0);
    /* "dwTotalVirtual: the size of the user-mode portion of the virtual address space" = the SYSTEM_INFO range */
    printf("virtual_covers_si_range %d\n",
           m.dwTotalVirtual >= (uint64_t)si.lpMaximumApplicationAddress - (uint64_t)si.lpMinimumApplicationAddress);
    /* "approximate percentage of physical memory that is in use" */
    printf("load_matches_phys %d\n", m.dwMemoryLoad == (uint32_t)(100 * (m.dwTotalPhys - m.dwAvailPhys) / m.dwTotalPhys));
    /* a second call may differ ("volatile"); here nothing is tracked, so it is equal */
    {
        MS m2;
        CALL(fle_imp_GlobalMemoryStatus, A(&m2), 0, 0, 0);
        printf("second_call_equal %d\n", memcmp(&m, &m2, sizeof m) == 0);
    }
"#,
    );
    assert_eq!(
        l,
        [
            "size 56",
            "r0_kept 153",
            "length_is_size 1",
            "load_0_100 1",
            "avail_le_total_phys 1",
            "avail_le_total_pagefile 1",
            "avail_le_total_virtual 1",
            "total_phys_nonzero 1",
            "virtual_covers_si_range 1",
            "load_matches_phys 1",
            "second_call_equal 1",
        ]
    );
}

#[test]
fn global_memory_status_values_are_the_settings() {
    // setting (decision 0034): 4 GiB physical, 2 GiB available, 8 GiB / 6 GiB commit, load 50, whole user range virtual.
    let l = lines(
        "memstatus-settings",
        r#"
    MS m;
    SI si;
    CALL(fle_imp_GetNativeSystemInfo, A(&si), 0, 0, 0);
    CALL(fle_imp_GlobalMemoryStatus, A(&m), 0, 0, 0);
    printf("load %u\n", m.dwMemoryLoad);
    printf("phys %llu %llu\n", U(m.dwTotalPhys), U(m.dwAvailPhys));
    printf("pagefile %llu %llu\n", U(m.dwTotalPageFile), U(m.dwAvailPageFile));
    printf("virtual_equals_range %d avail_equals_total %d\n",
           m.dwTotalVirtual == (uint64_t)si.lpMaximumApplicationAddress - (uint64_t)si.lpMinimumApplicationAddress + 1,
           m.dwAvailVirtual == m.dwTotalVirtual);
"#,
    );
    assert_eq!(
        l,
        [
            "load 50",
            "phys 4294967296 2147483648",
            "pagefile 8589934592 6442450944",
            "virtual_equals_range 1 avail_equals_total 1",
        ]
    );
}

// ---- 6. Process affinity ----

#[test]
fn process_affinity_mask_rules() {
    let l = lines(
        "affinity",
        r#"
    uint64_t p = 0xEE, s = 0xEE, ok;
    SI si;
    CALL(fle_imp_GetNativeSystemInfo, A(&si), 0, 0, 0);
    ok = CALL(fle_imp_GetProcessAffinityMask, PSEUDO, A(&p), A(&s), 0);
    printf("get ok %llu process %llx system %llx\n", U(ok), U(p), U(s));
    /* page: the process mask is a subset of the system mask; the system mask is the configured processors */
    printf("subset %d system_is_si_mask %d\n", (p & ~s) == 0, s == si.dwActiveProcessorMask);

    /* Set to a subset (here the same mask): nonzero */
    fle_set_last_error(5);
    ok = CALL(fle_imp_SetProcessAffinityMask, PSEUDO, s, 0, 0);
    printf("set_same ok %llu le %u\n", U(ok), LE());
    CALL(fle_imp_GetProcessAffinityMask, PSEUDO, A(&p), A(&s), 0);
    printf("after_set %llx\n", U(p));

    /* page: a processor that is not configured -> zero and ERROR_INVALID_PARAMETER; the mask stays */
    ok = CALL(fle_imp_SetProcessAffinityMask, PSEUDO, s | (s << 1), 0, 0);
    printf("set_outside ok %llu le %u\n", U(ok), LE());
    ok = CALL(fle_imp_SetProcessAffinityMask, PSEUDO, UINT64_C(1) << 63, 0, 0);
    printf("set_high_bit ok %llu le %u\n", U(ok), LE());
    CALL(fle_imp_GetProcessAffinityMask, PSEUDO, A(&p), A(&s), 0);
    printf("unchanged %llx\n", U(p));
"#,
    );
    assert_eq!(
        l,
        [
            // setting: one processor, so the masks are 1
            "get ok 1 process 1 system 1",
            "subset 1 system_is_si_mask 1",
            "set_same ok 1 le 5",
            "after_set 1",
            "set_outside ok 0 le 87",
            "set_high_bit ok 0 le 87",
            "unchanged 1",
        ]
    );
}

// ---- 7. Undefined cases stop the run ----

#[test]
fn undefined_cases_abort() {
    assert_aborts(
        "abort-si",
        &[
            ("native-null", "CALL(fle_imp_GetNativeSystemInfo, 0, 0, 0, 0);"),
            ("glpi-null-length", "static SLPI b[16]; CALL(fle_imp_GetLogicalProcessorInformation, A(b), 0, 0, 0);"),
            (
                "glpi-null-buffer-enough-length",
                "uint32_t len = 4096; CALL(fle_imp_GetLogicalProcessorInformation, 0, A(&len), 0, 0);",
            ),
            ("wow64-other-handle", "uint32_t o; CALL(fle_imp_IsWow64Process, 0, A(&o), 0, 0);"),
            ("wow64-thread-handle", "uint32_t o; CALL(fle_imp_IsWow64Process, UINT64_C(0xFFFFFFFFFFFFFFFE), A(&o), 0, 0);"),
            ("wow64-null-out", "CALL(fle_imp_IsWow64Process, PSEUDO, 0, 0, 0);"),
            ("memstatus-null", "CALL(fle_imp_GlobalMemoryStatus, 0, 0, 0, 0);"),
            ("getaff-other-handle", "uint64_t a, b; CALL(fle_imp_GetProcessAffinityMask, 0, A(&a), A(&b), 0);"),
            ("getaff-null-process-out", "uint64_t b; CALL(fle_imp_GetProcessAffinityMask, PSEUDO, 0, A(&b), 0);"),
            ("getaff-null-system-out", "uint64_t a; CALL(fle_imp_GetProcessAffinityMask, PSEUDO, A(&a), 0, 0);"),
            ("setaff-other-handle", "CALL(fle_imp_SetProcessAffinityMask, 0, 1, 0, 0);"),
            ("setaff-empty-mask", "CALL(fle_imp_SetProcessAffinityMask, PSEUDO, 0, 0, 0);"),
        ],
    );
}

#[test]
fn a_second_thread_stops_the_run() {
    let dir = work_dir();
    let tag = etxt_common::uniq("sysinfo-thread2");
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    let src = format!(
        r#"{PRELUDE}
static void *other(void *a) {{ MS m; (void)a; CALL(fle_imp_GlobalMemoryStatus, A(&m), 0, 0, 0); return NULL; }}
int main(void)
{{
    uint32_t out;
    pthread_t t;
    CALL(fle_imp_IsWow64Process, PSEUDO, A(&out), 0, 0);
    pthread_create(&t, NULL, other, NULL);
    pthread_join(t, NULL);
    printf("returned\n");
    return 0;
}}
"#
    );
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r: Output = Command::new(&exe).output().expect("run failed");
    assert_eq!(r.status.signal(), Some(6));
    assert!(String::from_utf8_lossy(&r.stderr).contains("fle: GlobalMemoryStatus: called from a second thread"));
    assert!(!String::from_utf8_lossy(&r.stdout).contains("returned"));
}

// ---- 8. GetProcAddress finds the rows ----

fn runtime_sources(rt: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(rt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c")
        })
        .collect();
    v.sort();
    v
}

/// Links the generated unit (with GetProcAddress and the six rows) with every runtime source and asks the real
/// GetProcAddress for each name. Needs fle_sysinfo.c in runtime/ (the table names fle_imp_* of every row).
#[test]
fn get_proc_address_finds_the_rows_and_keeps_not_found_for_the_rest() {
    if !runtime_dir().join("fle_sysinfo.c").exists() {
        return;
    }
    let mut names: Vec<&str> = ROWS.to_vec();
    names.push("GetProcAddress");
    let (_, c) = gen(&names);
    let dir = work_dir();
    let rt = runtime_dir();
    let tag = etxt_common::uniq("gen-sysinfo-tables");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &c).unwrap();
    let main = r#"#include <stdio.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"

static uint64_t gpa(uint64_t mod, const char *name)
{
    uint64_t a;
    fle_set_last_error(5);
    R[1] = mod; R[2] = (uint64_t)(uintptr_t)name; R[0] = 99;
    fle_imp_GetProcAddress();
    a = R[0];
    fle_imp_GetLastError();
    printf("%s %s %llu\n", name, a != 0 ? "found" : "null", (unsigned long long)R[0]);
    return a;
}

int main(void)
{
    uint64_t k = fle_module_handle(8), u = fle_module_handle(18), va;
    unsigned char si[48];
    unsigned long long len;
    /* the delay-load helper of the DLL asks for this name first (0x45413e, pdata-seeds-run.md) */
    va = gpa(k, "GetNativeSystemInfo");
    if (va == 0) return 1;
    memset(si, 0xEE, sizeof si);
    R[1] = (uint64_t)(uintptr_t)si; R[0] = 99;
    fle_dispatch(va);
    printf("native_arch %u page %u\n", si[0] | (si[1] << 8), si[4] | (si[5] << 8));
    va = gpa(k, "GetLogicalProcessorInformation");
    if (va == 0) return 1;
    { unsigned char buf[512]; unsigned l = 0; uint64_t ok;
      R[1] = 0; R[2] = (uint64_t)(uintptr_t)&l; fle_dispatch(va); ok = R[0];
      printf("glpi_query %llu %u\n", (unsigned long long)ok, l);
      len = sizeof buf; (void)len;
      l = sizeof buf; R[1] = (uint64_t)(uintptr_t)buf; R[2] = (uint64_t)(uintptr_t)&l; fle_dispatch(va);
      printf("glpi %llu %u\n", (unsigned long long)R[0], l); }
    gpa(k, "IsWow64Process");
    gpa(k, "GlobalMemoryStatus");
    gpa(k, "GetProcessAffinityMask");
    gpa(k, "SetProcessAffinityMask");
    /* exact spelling, module, and names that are not answered */
    gpa(k, "getnativesysteminfo");
    gpa(u, "GetNativeSystemInfo");
    gpa(k, "GetNumaHighestNodeNumber");
    gpa(k, "IsProcessorFeaturePresent");
    gpa(k, "GlobalMemoryStatusEx");
    gpa(k, "GetLogicalProcessorInformationEx");
    return 0;
}
"#;
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
    for p in runtime_sources(&rt) {
        cmd.arg(p);
    }
    let cc = cmd.arg(&src).arg(&mainc).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            // found: a success keeps the last error (5); GetProcAddress page: NULL on failure, 127 is the project's code (decision 0015)
            "GetNativeSystemInfo found 5",
            "native_arch 9 page 4096",
            "GetLogicalProcessorInformation found 5",
            "glpi_query 0 224",
            "glpi 1 224",
            "IsWow64Process found 5",
            "GlobalMemoryStatus found 5",
            "GetProcessAffinityMask found 5",
            "SetProcessAffinityMask found 5",
            "getnativesysteminfo null 127",
            "GetNativeSystemInfo null 127",
            "GetNumaHighestNodeNumber null 127",
            "IsProcessorFeaturePresent null 127",
            "GlobalMemoryStatusEx null 127",
            "GetLogicalProcessorInformationEx null 127",
        ]
    );
}

// ---- 9. The source reads nothing from the host ----

fn strip_comments(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            o.push(b[i] as char);
            i += 1;
        }
    }
    o
}

#[test]
fn source_uses_no_host_processor_memory_time_or_file_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_sysinfo.c")).unwrap());
    for bad in [
        "sysconf", "get_nprocs", "sched_getaffinity", "sysinfo(", "/proc", "/sys/", "getenv", "environ", "getpid", "clock_gettime",
        "gettimeofday", "<time.h>", "fopen", "open(", "mmap", "cpuid", "exit(", "_exit",
    ] {
        assert!(!code.contains(bad), "fle_sysinfo.c must not use {bad}");
    }
}
