//! Tests of decision 0096 (docs/decisions/0096-deferred-modules.md, provisional): blend2d.dll and quickfontcache_x64.dll are deferred
//! modules, N = 33 and N = 34 of the virtual module table. LoadLibraryW/A/ExW succeeds with the handle 0x7FF800000000 + 0x10000 * N and
//! runs no DllMain; GetProcAddress hands out the pseudo-VA of a response-table row for each export name; calling that pseudo-VA stops
//! by name with the four register arguments; a real module (fle_realmod) of the same name wins.
//!
//! 1. Generator: the 11 rows (1 + 10) under the two dll names, appended after the rows that existed, and the exports match the PE
//!    export directories (the real DLLs are read when they are present: $BLEND2D_ORIG_PE and $QFC_PE, else lift/out-blend2d/Blend2D.dll
//!    and lift/out-qfc/quickfontcache_x64.dll; skipped when absent, failing when the variable is set and the file is missing).
//! 2. Runtime (a driver linked with every runtime source and the generated C of a synthetic PE that imports GetProcAddress, so the
//!    response table is the generator's own): LoadLibrary, the handles, the reference count, GetModuleHandleW/A, GetProcAddress for all
//!    11 exports and for names that are not exports (NULL and 127), the stop of every export, FreeLibrary, the names around the two.
//! 3. A second driver registers a real module named blend2d.dll (a fake image mapped at its base, DllMain = a generated function) and
//!    checks that it wins everywhere while quickfontcache_x64.dll stays deferred.
//!
//! Expected values are written here from decision 0096, decision 0015 (handle = 0x7FF800000000 + 0x10000 * N), decision 0082 (D11,
//! DllMain) and the export directories of the two DLLs (read with pefile and with fe_pe), not read back from the C code. The host is
//! Linux: nothing is compared with a real Windows.

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const BLEND2D: &str = "blend2d.dll";
const QFC: &str = "quickfontcache_x64.dll";
const BLEND2D_EXPORTS: [&str; 1] = ["AggCreateInstance"];
/// The ten exports of QuickFontCache_x64.dll in the order of the export directory's ordinals (1..10; pefile and fe_pe). dbkFCallWrapperAddr is
/// a variable in .bss, the other nine are code in .text.
const QFC_EXPORTS: [&str; 10] = [
    "dbkFCallWrapperAddr",
    "__dbk_fcall_wrapper",
    "InitSharedPython",
    "CreateVST3Instance",
    "LoadSharedElastiqueDll",
    "AcquireSharedAnimTimer",
    "AcquireSharedThreadBank",
    "FontCache_GetStats",
    "FontCache_CleanUp",
    "FontCache_AcquireFont",
];
/// The response table had 268 rows before decision 0096 (the last one SleepConditionVariableSRW, decision 0092); the 11 new rows follow.
const ROWS_BEFORE: usize = 268;
const N_BLEND2D: u64 = 33;
const N_QFC: u64 = 34;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-deferred/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}
fn handle(n: u64) -> u64 {
    0x7FF8_0000_0000 + 0x10000 * n
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE (ImageBase 0x140000000) that imports kernel32!GetProcAddress, so that the generator emits the response table. Its entry
// 0x140001000 is a DllMain that sets eax = 1 and returns when edx == 1 (DLL_PROCESS_ATTACH); the call of the thunk is behind the other
// branch and is never run. The third driver uses it as the DllMain of the real module blend2d.dll.
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

fn build_pe() -> Vec<u8> {
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
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(b"GetProcAddress\0");
    put(&mut b, 0x2400, &hn);
    w64(&mut b, data_at(0x2100), 0x2400);
    w64(&mut b, data_at(0x2200), 0x2400);

    let code: Vec<(u32, Vec<u8>)> = vec![
        (0x1000, vec![0xb8, 0x01, 0x00, 0x00, 0x00]),          // mov eax, 1
        (0x1005, vec![0x83, 0xfa, 0x01]),                      // cmp edx, 1
        (0x1008, vec![0x75, 0x01]),                            // jne 0x100b
        (0x100a, vec![0xc3]),                                  // ret
        (0x100b, [&[0xe8][..], &rel(0x1010, 0x1040)].concat()), // call 0x1040 (never run)
        (0x1010, vec![0xc3]),                                  // ret
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()), // jmp [GetProcAddress]
    ];
    for (rva, bytes) in code {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn gen_c() -> (fe_lift::Stats, String) {
    let mut out = Vec::new();
    let st = generate(&build_pe(), &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

/// The rows of `fle_resp_table` in the generated C, in order: (dll, name, pseudo-VA).
fn resp_rows(c: &str) -> Vec<(String, String, u64)> {
    let start = c.find("const fle_resp_entry fle_resp_table[] = {").unwrap();
    let end = c[start..].find("};").unwrap() + start;
    c[start..end]
        .lines()
        .filter(|l| l.trim_start().starts_with("{ \""))
        .map(|l| {
            let q: Vec<&str> = l.split('"').collect();
            let va = u64::from_str_radix(l.split("UINT64_C(0x").nth(1).unwrap().split(')').next().unwrap(), 16).unwrap();
            (q[1].to_string(), q[3].to_string(), va)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

#[test]
fn the_response_table_has_one_row_per_export_after_the_rows_that_existed() {
    let (st, c) = gen_c();
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    // Not pinned to the total (a later decision appends rows after these, as decision 0092 did after 0091's): the 268 rows that existed keep
    // their positions, and the 11 rows of decision 0096 are the ones right after them.
    assert!(rows.len() >= ROWS_BEFORE + 11, "{}", rows.len());
    for (i, (dll, name, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7ff9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i} {dll}!{name}: a pseudo-VA is base + 0x10 * (position), so appending moves none");
    }
    // The rows that existed keep their positions (decision 0015): the ones that other decisions pin, and the count before the new rows.
    let at = |n: &str| rows.iter().position(|(d, name, _)| d == "kernel32.dll" && name == n).unwrap_or_else(|| panic!("no row {n}"));
    assert_eq!(rows[at("GetModuleHandleW")].2, 0x7ff9_0000_00c0);
    assert_eq!(rows[at("GetProcAddress")].2, 0x7ff9_0000_00d0);
    assert_eq!(at("GetModuleHandleA"), 260);
    assert_eq!(at("SleepConditionVariableSRW"), ROWS_BEFORE - 1, "the last row of decision 0092 is the last row that existed");
    // The 11 rows come right after it: Blend2D's one, then QFC's ten in the order of the export directory's ordinals.
    let new = &rows[ROWS_BEFORE..ROWS_BEFORE + 11];
    assert_eq!((new[0].0.as_str(), new[0].1.as_str()), (BLEND2D, BLEND2D_EXPORTS[0]));
    for (i, n) in QFC_EXPORTS.iter().enumerate() {
        assert_eq!((new[1 + i].0.as_str(), new[1 + i].1.as_str()), (QFC, *n), "QFC row {i}");
    }
    assert_eq!(new[0].2, 0x7ff9_0000_0000 + 0x10 * (ROWS_BEFORE as u64 + 1));
    // No other row is under the two names, and each export appears once.
    assert_eq!(rows.iter().filter(|r| r.0 == BLEND2D).count(), 1);
    assert_eq!(rows.iter().filter(|r| r.0 == QFC).count(), 10);
    for n in BLEND2D_EXPORTS.iter().chain(QFC_EXPORTS.iter()) {
        assert_eq!(c.matches(&format!("\"{n}\"")).count(), 1, "{n} is in the table once");
    }
    assert!(c.contains("void fle_imp_deferred_blend2d_AggCreateInstance(void);"), "the deferred stops are declared");
    // Every new pseudo-VA is a dispatch entry; the dispatch count is the function entries plus one per row.
    for (_, n, va) in new {
        assert!(c.contains(&format!("    {{ UINT64_C({va:#x}), fle_imp_deferred_")), "{n} in fle_dispatch_table");
    }
    assert!(c.contains(&format!("const uint64_t fle_dispatch_count = {};", st.functions + st.resp_entries)));
    assert!(c.contains(&format!("const uint64_t fle_resp_count = {};", st.resp_entries)));
}

/// The export names in the table are the export directories of the real DLLs (skipped when a DLL is absent).
fn dll_path(var: &str, default: &str) -> Option<PathBuf> {
    let explicit = std::env::var_os(var);
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => manifest().join(default),
    };
    match std::fs::metadata(&path) {
        Ok(_) => Some(path),
        Err(e) if explicit.is_some() => panic!("{var}={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

fn exports_of(path: &Path) -> Vec<(String, u32, bool)> {
    let data = std::fs::read(path).unwrap();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let mut v: Vec<(String, u32, bool)> = pe.exports().unwrap().into_iter().map(|e| (e.name.expect("an export without a name"), e.ordinal, e.forwarder.is_some())).collect();
    v.sort_by_key(|e| e.1);
    v
}

#[test]
fn the_rows_are_the_export_directories_of_the_real_dlls() {
    let (_, c) = gen_c();
    let rows = resp_rows(&c);
    let under = |dll: &str| -> Vec<String> { rows.iter().filter(|r| r.0 == dll).map(|r| r.1.clone()).collect() };
    if let Some(p) = dll_path("BLEND2D_ORIG_PE", "../../out-blend2d/Blend2D.dll") {
        let e = exports_of(&p);
        assert!(e.iter().all(|x| !x.2), "no forwarder");
        assert_eq!(e.iter().map(|x| x.0.clone()).collect::<Vec<_>>(), under(BLEND2D));
        assert_eq!(e.len(), 1);
    }
    if let Some(p) = dll_path("QFC_PE", "../../out-qfc/quickfontcache_x64.dll") {
        let e = exports_of(&p);
        assert!(e.iter().all(|x| !x.2), "no forwarder");
        assert_eq!(e.iter().map(|x| x.0.clone()).collect::<Vec<_>>(), under(QFC), "all ten exports, in ordinal order");
        assert_eq!(e.iter().map(|x| x.1).collect::<Vec<_>>(), (1..=10).collect::<Vec<u32>>());
        assert_eq!(e.len(), 10);
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

const DRIVER: &str = r##"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <errno.h>
#include <sys/mman.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_load.h"
#include "fle_callback.h"
#include "fle_realmod.h"

static uint8_t stk[1 << 16] __attribute__((aligned(16)));
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))
#define DLLMAIN_VA 0x140001000ull

static unsigned dllmain_calls, other_calls;
uint64_t __real_fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9);
uint64_t __wrap_fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9)
{
    if (va == DLLMAIN_VA) dllmain_calls++; else other_calls++;
    return __real_fle_call_guest(va, rcx, rdx, r8, r9);
}

static void prep(uint64_t a, uint64_t b, uint64_t c)
{
    R[4] = (A(stk) + sizeof stk - 128) & ~UINT64_C(15);
    fle_st(R[4], 8, 0xdead0000u);
    R[1] = a; R[2] = b; R[8] = c; R[9] = 0;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static void out(void)
{
    uint64_t r = R[0];
    fle_imp_GetLastError();
    printf("%llx %llu\n", U(r), U(R[0]));
}
static void llw(const uint16_t *n) { prep(A(n), 0, 0); fle_imp_LoadLibraryW(); out(); }
static void lla(const char *n) { prep(A(n), 0, 0); fle_imp_LoadLibraryA(); out(); }
static void llex(const uint16_t *n, uint32_t flags) { prep(A(n), 0, flags); fle_imp_LoadLibraryExW(); out(); }
static void gmhw(const uint16_t *n) { prep(A(n), 0, 0); fle_imp_GetModuleHandleW(); out(); }
static void gmha(const char *n) { prep(A(n), 0, 0); fle_imp_GetModuleHandleA(); out(); }
static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n), 0); fle_imp_GetProcAddress();
    r = R[0];
    fle_imp_GetLastError();
    printf("%llx %llu\n", U(r), U(R[0]));
    return r;
}
static void free_lib(uint64_t m) { prep(m, 0, 0); fle_imp_FreeLibrary(); out(); }
#define LLW(s) llw((const uint16_t *)u##s)
#define LLEX(s, f) llex((const uint16_t *)u##s, f)
#define GMHW(s) gmhw((const uint16_t *)u##s)

/* every export: (N, name), Blend2D first, then QFC in the order of the export directory's ordinals */
static const struct { int n; const char *name; } EXPORTS[] = {
    { 33, "AggCreateInstance" },
    { 34, "dbkFCallWrapperAddr" }, { 34, "__dbk_fcall_wrapper" }, { 34, "InitSharedPython" }, { 34, "CreateVST3Instance" },
    { 34, "LoadSharedElastiqueDll" }, { 34, "AcquireSharedAnimTimer" }, { 34, "AcquireSharedThreadBank" }, { 34, "FontCache_GetStats" },
    { 34, "FontCache_CleanUp" }, { 34, "FontCache_AcquireFont" },
};
#define NEXPORTS (sizeof EXPORTS / sizeof EXPORTS[0])

static void mapped(uint64_t h)
{
    unsigned char v;
    int r = mincore((void *)(uintptr_t)h, 4096, &v);
    printf("mapped %llx %s\n", U(h), r == 0 ? "yes" : (errno == ENOMEM ? "no" : "error"));
}

#ifdef REAL_MODULE
/* A real module named blend2d.dll (fle_realmod.h): the fake image is mapped at its base; its entry point is the generated DllMain. */
#define RM_BASE 0x13F000000ull
const fle_realmod_entry fle_realmod_table[] = { { "blend2d.dll", RM_BASE } };
const uint64_t fle_realmod_count = 1;
static void w16(uint8_t *b, size_t o, uint16_t v) { memcpy(b + o, &v, 2); }
static void w32(uint8_t *b, size_t o, uint32_t v) { memcpy(b + o, &v, 4); }
static void map_fake_image(void)
{
    uint8_t *b = mmap((void *)RM_BASE, 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    size_t opt = 0x80 + 24;
    if (b != (uint8_t *)RM_BASE) { fprintf(stderr, "the fake image cannot be mapped at %#llx\n", U(RM_BASE)); exit(2); }
    w16(b, 0, 0x5a4d); w32(b, 0x3c, 0x80); w32(b, 0x80, 0x4550);
    w16(b, opt, 0x20b);
    w32(b, opt + 16, (uint32_t)(DLLMAIN_VA - RM_BASE));   /* AddressOfEntryPoint */
    w32(b, opt + 112, 0x400); w32(b, opt + 116, 0x100);    /* export directory */
    w32(b, 0x400 + 20, 1); w32(b, 0x400 + 24, 1);           /* one function, one name */
    w32(b, 0x400 + 28, 0x500); w32(b, 0x400 + 32, 0x510); w32(b, 0x400 + 36, 0x520);
    w32(b, 0x500, 0x2000);                                  /* AggCreateInstance at RVA 0x2000 (outside the directory) */
    w32(b, 0x510, 0x540); w16(b, 0x520, 0);
    memcpy(b + 0x540, "AggCreateInstance", 18);
}
#endif

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    unsigned i;
#ifdef REAL_MODULE
    map_fake_image();
#endif
    if (!strcmp(s, "table")) {
        printf("count %llu\n", U(fle_module_count()));
        printf("name32 %s\n", fle_module_name(32));
        printf("name33 %s\n", fle_module_name(33));
        printf("handle32 %llx\n", U(fle_module_handle(32)));
        printf("handle33 %llx\n", U(fle_module_handle(33)));
        printf("deferred %d %d %d %d\n", fle_module_is_deferred(31), fle_module_is_deferred(32), fle_module_is_deferred(33), fle_module_is_deferred(34));
        printf("past %p\n", (const void *)fle_module_name(41)); /* decision 0238: 41 modules (combase.dll after kernelbase.dll); 40 modules (decision 0122: 39; decision 0119: 38; old: 36) */
    } else if (!strcmp(s, "load")) {
        LLW("blend2d.dll"); LLW("Blend2D.dll"); LLW("BLEND2D"); LLW("blend2d.dll.");
        LLW("quickfontcache_x64.dll"); LLW("QuickFontCache_x64.dll"); LLW("QUICKFONTCACHE_X64"); LLW("quickfontcache_x64.dll.");
        lla("Blend2D.dll"); lla("blend2d"); lla("QuickFontCache_x64"); lla("quickfontcache_x64.dll");
        LLEX("blend2d.dll", 0); LLEX("Blend2D", 0x800); LLEX("blend2d.dll", 0x2); LLEX("blend2d", 0x20);
        LLEX("quickfontcache_x64.dll", 0); LLEX("QuickFontCache_x64", 0x800); LLEX("quickfontcache_x64.dll", 0x2); LLEX("quickfontcache_x64", 0x40);
        printf("calls %u %u depth %u\n", dllmain_calls, other_calls, fle_callback_depth());
        mapped(M(33)); mapped(M(34));
    } else if (!strcmp(s, "refs")) {
        printf("init %u %u\n", fle_load_refcount(32), fle_load_refcount(33));
        LLW("blend2d.dll"); LLW("BLEND2D"); lla("Blend2D.dll"); LLW("quickfontcache_x64.dll");
        printf("count %u %u\n", fle_load_refcount(32), fle_load_refcount(33));
        free_lib(M(33)); free_lib(M(33)); free_lib(M(34));
        printf("count %u %u\n", fle_load_refcount(32), fle_load_refcount(33));
        printf("past %u\n", fle_load_refcount(41)); /* decision 0238: 41 modules (combase.dll after kernelbase.dll); 40 modules (decision 0122: 39; decision 0119: 38; old: 36) */
    } else if (!strcmp(s, "gmh")) {
        const char *const a[] = { "blend2d.dll", "Blend2D.dll", "BLEND2D", "blend2d", "blend2d.dll.", "blend2d.", "blend2d.dll.dll", "blend2d2.dll",
            "quickfontcache_x64.dll", "QuickFontCache_x64.dll", "QUICKFONTCACHE_X64", "quickfontcache_x64", "quickfontcache_x64.dll.",
            "quickfontcache.dll", "quickfontcache_x86.dll", "quickfontcache_x64.dll.dll", "" };
        for (i = 0; i < sizeof a / sizeof a[0]; i++) gmha(a[i]);
        GMHW("blend2d.dll"); GMHW("Blend2D"); GMHW("quickfontcache_x64.dll"); GMHW("QuickFontCache_x64"); GMHW("blend2d2"); GMHW("quickfontcache");
        /* a load does not change what GetModuleHandle answers: the module is there from the start, like every virtual module */
        LLW("blend2d.dll"); gmha("blend2d.dll");
    } else if (!strcmp(s, "gpa-blend")) {
        gpa(M(33), "AggCreateInstance");
        gpa(M(33), "aggcreateinstance"); gpa(M(33), "AGGCREATEINSTANCE"); gpa(M(33), "AggCreateInstanc"); gpa(M(33), "AggCreateInstance2"); gpa(M(33), "");
        gpa(M(33), "GetLastError"); gpa(M(33), "FontCache_AcquireFont"); gpa(M(33), "DllMain");
        gpa(M(34), "AggCreateInstance"); gpa(M(9), "AggCreateInstance"); gpa(M(25), "AggCreateInstance");
    } else if (!strcmp(s, "gpa-qfc")) {
        for (i = 1; i < NEXPORTS; i++) gpa(M(34), EXPORTS[i].name);
        gpa(M(34), "FontCache_AcquireFonts"); gpa(M(34), "fontcache_acquirefont"); gpa(M(34), "FONTCACHE_ACQUIREFONT"); gpa(M(34), "FontCache_AcquireFon");
        gpa(M(34), "GetLastError"); gpa(M(34), "GetModuleHandleW"); gpa(M(34), "DllMain");
        gpa(M(33), "FontCache_AcquireFont"); gpa(M(33), "InitSharedPython"); gpa(M(9), "FontCache_AcquireFont"); gpa(M(9), "dbkFCallWrapperAddr");
    } else if (!strcmp(s, "gpa-others-unchanged")) {
        /* the rows of the other modules are still found under their own dll only */
        gpa(M(9), "GetLastError"); gpa(M(9), "GetProcAddress"); gpa(M(33), "GetProcAddress"); gpa(M(34), "GetProcAddress");
    } else if (!strncmp(s, "stop-", 5)) {
        unsigned k = (unsigned)atoi(s + 5);
        uint64_t va = gpa(M(EXPORTS[k].n), EXPORTS[k].name);
        fflush(stdout);
        R[0] = 0x77; R[1] = 0x1111; R[2] = 0x2222; R[8] = 0x3333; R[9] = 0x4444;
        fle_dispatch(va);
        printf("returned %llx\n", U(R[0]));
    } else if (!strcmp(s, "kept-pointer-stop")) {
        /* the guest's sequence for Blend2D: LoadLibraryW, GetProcAddress, keep the pointer, call it later */
        uint64_t h, va;
        prep(A(u"Blend2D.dll"), 0, 0); fle_imp_LoadLibraryW(); h = R[0];
        prep(h, A("AggCreateInstance"), 0); fle_imp_GetProcAddress(); va = R[0];
        printf("kept %d\n", va != 0); fflush(stdout);
        R[1] = 0xaa; R[2] = 0; R[8] = 0; R[9] = 0;
        fle_dispatch(va);
        printf("returned\n");
    } else if (!strcmp(s, "free-blend-count-one")) { free_lib(M(33));
    } else if (!strcmp(s, "free-qfc-count-one")) { free_lib(M(34));
    } else if (!strcmp(s, "free-blend-after-balanced")) { LLW("blend2d.dll"); free_lib(M(33)); free_lib(M(33));
    } else if (!strcmp(s, "free-past-the-table")) { free_lib(M(42)); /* decision 0238: past the 41 modules (M(41) with 40: kernelbase.dll only); decision 0238 earlier: past the 40 modules (M(40) was the first handle past 39 after decision 0122); decision 0122: past the 39 modules (decision 0119: M(39) past 38; old: M(37), the first handle past 36 modules) */
    } else if (!strcmp(s, "gpa-past-the-table")) { gpa(M(42), "AggCreateInstance"); /* decision 0238: past the 41 modules (M(41) with 40: kernelbase.dll only); decision 0238 earlier: past the 40 modules (M(40) after decision 0122); decision 0122: past the 39 modules (decision 0119: M(39); old: M(37)) */
    } else if (!strcmp(s, "gpa-misaligned")) { gpa(M(33) + 1, "AggCreateInstance");
    } else if (!strcmp(s, "gpa-ordinal")) { prep(M(33), 1, 0); fle_imp_GetProcAddress(); out();
    } else if (!strcmp(s, "near-1")) { LLW("blend2d2.dll");
    } else if (!strcmp(s, "near-2")) { LLW("blend2d.");
    } else if (!strcmp(s, "near-3")) { LLW("quickfontcache.dll");
    } else if (!strcmp(s, "near-4")) { LLW("quickfontcache_x86.dll");
    } else if (!strcmp(s, "near-5")) { LLW("quickfontcache_x64.dll.dll");
    } else if (!strcmp(s, "near-6")) { LLW("blend2d.dll.dll");
    } else if (!strcmp(s, "near-7")) { LLW("quickfontcache_x64.");
#ifdef REAL_MODULE
    } else if (!strcmp(s, "real-gmh-before")) {
        GMHW("blend2d.dll"); gmha("Blend2D"); GMHW("quickfontcache_x64.dll"); gmha("QuickFontCache_x64");
        printf("calls %u\n", dllmain_calls);
    } else if (!strcmp(s, "real-load")) {
        LLW("blend2d.dll");
        printf("calls %u\n", dllmain_calls);
        LLW("Blend2D"); lla("BLEND2D.DLL"); LLEX("blend2d.dll", 0x800);
        printf("calls %u\n", dllmain_calls);
        GMHW("blend2d.dll"); gmha("blend2d"); gmha("BLEND2D.");
    } else if (!strcmp(s, "real-gpa")) {
        uint64_t h;
        prep(A(u"blend2d.dll"), 0, 0); fle_imp_LoadLibraryW(); h = R[0];
        printf("handle %llx\n", U(h));
        gpa(h, "AggCreateInstance"); gpa(h, "aggcreateinstance"); gpa(h, "FontCache_AcquireFont"); gpa(h, "GetLastError");
        /* the deferred handle of the same name is not the module any more: not asked here (nothing hands it out), but QFC still is deferred */
        gpa(M(34), "FontCache_AcquireFont");
    } else if (!strcmp(s, "real-qfc")) {
        LLW("quickfontcache_x64.dll"); LLW("blend2d.dll"); LLW("QuickFontCache_x64");
        printf("calls %u\n", dllmain_calls);
        gpa(M(34), "FontCache_CleanUp");
    } else if (!strcmp(s, "real-refs")) {
        LLW("blend2d.dll"); LLW("blend2d.dll"); LLW("blend2d.dll");
        free_lib(RM_BASE); free_lib(RM_BASE);
        printf("two frees\n"); fflush(stdout);
        free_lib(RM_BASE);
    } else if (!strcmp(s, "real-free-before-load")) { free_lib(RM_BASE);
    } else if (!strcmp(s, "real-stop-deferred-qfc")) {
        uint64_t va;
        LLW("blend2d.dll");
        prep(M(34), A("FontCache_AcquireFont"), 0); fle_imp_GetProcAddress(); va = R[0];
        R[1] = 0x10; R[2] = 0x20; R[8] = 0x30; R[9] = 0x40;
        fle_dispatch(va);
#endif
    } else {
        printf("no scenario %s\n", s);
        return 3;
    }
    return 0;
}
"##;

fn runtime_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("rt"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(rt_dir())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("fle_") && n.ends_with(".c")
            })
            .collect();
        srcs.sort();
        let mut v = Vec::new();
        for s in srcs {
            let o = dir.join(s.file_name().unwrap().to_string_lossy().replace(".c", ".o"));
            let cc = Command::new("clang")
                .args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-c", "-I"])
                .arg(rt_dir())
                .arg(&s)
                .arg("-o")
                .arg(&o)
                .env("TMPDIR", work_dir().join("tmp"))
                .output()
                .expect("clang not found");
            assert!(cc.status.success(), "clang failed for {}: {}", s.display(), String::from_utf8_lossy(&cc.stderr));
            v.push(o);
        }
        v
    })
}

/// The driver over the generated C of the synthetic PE and every runtime object; `real` registers a real module named blend2d.dll.
fn build_driver(real: bool) -> PathBuf {
    let dir = work_dir();
    let tag = etxt_common::per_process(if real { "deferred-real" } else { "deferred-driver" });
    let (_, c) = gen_c();
    let gen_path = dir.join(format!("{tag}-gen.c"));
    let main_path = dir.join(format!("{tag}-main.c"));
    std::fs::write(&gen_path, &c).unwrap();
    std::fs::write(&main_path, DRIVER).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE"]);
    if real {
        cmd.arg("-DREAL_MODULE=1");
    }
    cmd.arg("-I").arg(rt_dir());
    for o in runtime_objs() {
        cmd.arg(o);
    }
    let cc = cmd
        .arg(&gen_path)
        .arg(&main_path)
        .arg("-lm")
        .arg("-Wl,--wrap=fle_call_guest")
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| build_driver(false))
}
fn real_driver() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| build_driver(true))
}

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run_with(exe: &Path, scenario: &str) -> Out {
    let r = Command::new(exe).arg(scenario).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}
fn lines_with(exe: &Path, scenario: &str) -> Vec<String> {
    let o = run_with(exe, scenario);
    assert!(o.ok, "{scenario}: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}
fn lines(scenario: &str) -> Vec<String> {
    lines_with(driver(), scenario)
}
fn aborts_with(exe: &Path, scenario: &str, must_say: &str) -> Out {
    let o = run_with(exe, scenario);
    assert!(!o.ok, "{scenario}: expected an abort, got stdout {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{scenario}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.starts_with("fle:"), "{scenario}: no fle: message: {}", o.stderr);
    assert!(o.stderr.contains(must_say), "{scenario}: message lacks {must_say:?}: {}", o.stderr);
    assert!(!o.stdout.contains("returned"), "{scenario}: returned");
    o
}
fn aborts(scenario: &str, must_say: &str) -> Out {
    aborts_with(driver(), scenario, must_say)
}

fn ok5(n: u64) -> String {
    format!("{:x} 5", handle(n))
}
const MISS_126: &str = "0 126";
const MISS_127: &str = "0 127";

#[test]
fn the_table_has_the_two_deferred_modules_n33_and_n34_and_shcore_after_them() {
    // Decision 0099 appended shcore.dll (N = 35, index 34) after the two deferred modules; decision 0118 appended rpcrt4.dll (N = 36, index 35); decision 0119 appended
    // wtsapi32.dll (N = 37) and uxtheme.dll (N = 38): the count is 38 (old: 36).
    let got = lines("table");
    assert_eq!(got[0], "count 41"); // decision 0238: kernelbase.dll and combase.dll appended (40 with kernelbase.dll only) (decision 0122: dwmapi.dll, 39; old: 38)
    assert_eq!(got[1], "name32 blend2d.dll");
    assert_eq!(got[2], "name33 quickfontcache_x64.dll");
    assert_eq!(got[3], format!("handle32 {:x}", handle(N_BLEND2D)));
    assert_eq!(got[4], format!("handle33 {:x}", handle(N_QFC)));
    assert_eq!(got[3], "handle32 7ff800210000");
    assert_eq!(got[4], "handle33 7ff800220000");
    assert_eq!(got[5], "deferred 0 1 1 0", "only the two entries N = 33, 34 are deferred (avrt.dll, powrprof.dll before them and shcore.dll after them are not)");
    assert!(got[6].starts_with("past ") && (got[6].contains("nil") || got[6].ends_with(" 0x0") || got[6].ends_with(" 0")), "{}", got[6]);
}

#[test]
fn load_library_succeeds_with_the_handle_of_the_rule_and_runs_no_dllmain() {
    let got = lines("load");
    // LoadLibraryW (4 spellings each), LoadLibraryA (4), LoadLibraryExW (8): the handle, last error untouched (5, set before the call)
    let (b, q) = (ok5(N_BLEND2D), ok5(N_QFC));
    let want: Vec<String> = [&b, &b, &b, &b, &q, &q, &q, &q, &b, &b, &q, &q, &b, &b, &b, &b, &q, &q, &q, &q].iter().map(|s| s.to_string()).collect();
    assert_eq!(&got[..20], &want[..]);
    // No DllMain, no other call into the guest, and nothing is mapped at either handle (no image).
    assert_eq!(got[20], "calls 0 0 depth 0");
    assert_eq!(got[21], format!("mapped {:x} no", handle(N_BLEND2D)));
    assert_eq!(got[22], format!("mapped {:x} no", handle(N_QFC)));
    assert_eq!(got.len(), 23);
}

#[test]
fn the_reference_count_is_the_one_of_a_virtual_module() {
    // Decision 0082 and the FreeLibrary page, as for avrt.dll and powrprof.dll (decision 0088): count one from the start, LoadLibrary adds one,
    // FreeLibrary subtracts one and returns nonzero (last error untouched). A count of one stops by name (D11).
    let got = lines("refs");
    assert_eq!(
        got,
        [
            "init 1 1".to_string(),
            ok5(N_BLEND2D),
            ok5(N_BLEND2D),
            ok5(N_BLEND2D),
            ok5(N_QFC),
            "count 4 2".into(),
            "1 5".into(),
            "1 5".into(),
            "1 5".into(),
            "count 2 1".into(),
            "past 0".into(),
        ]
    );
    aborts("free-blend-count-one", "the reference count would reach zero");
    aborts("free-qfc-count-one", "the reference count would reach zero");
    aborts("free-blend-after-balanced", "the reference count would reach zero");
    aborts("free-past-the-table", "not a virtual module handle");
}

#[test]
fn get_module_handle_answers_like_for_any_virtual_module() {
    let got = lines("gmh");
    let (b, q) = (ok5(N_BLEND2D), ok5(N_QFC));
    // GetModuleHandleA: blend2d.dll x5 spellings (the 4th "blend2d" has no extension, the 5th a trailing point), then the near names
    let mut want: Vec<String> = vec![b.clone(), b.clone(), b.clone(), b.clone(), b.clone()];
    want.extend(["blend2d.", "blend2d.dll.dll", "blend2d2.dll"].iter().map(|_| MISS_126.to_string()));
    want.extend([&q, &q, &q, &q, &q].iter().map(|s| s.to_string()));
    // quickfontcache.dll (the 32-bit DLL), quickfontcache_x86.dll, quickfontcache_x64.dll.dll and "" are not modules
    want.extend(["", "", "", ""].iter().map(|_| MISS_126.to_string()));
    // GetModuleHandleW
    want.extend([&b, &b, &q, &q].iter().map(|s| s.to_string()));
    want.extend(["", ""].iter().map(|_| MISS_126.to_string()));
    // after LoadLibraryW the answer is the same (the module is there from the start)
    want.push(b.clone());
    want.push(b.clone());
    assert_eq!(got, want);
    // "blend2d." : the trailing point removes the extension rule, so the name is "blend2d", which is not "blend2d.dll"
    assert_eq!(got[5], MISS_126);
}

#[test]
fn get_proc_address_gives_a_pseudo_va_for_each_export_name_of_blend2d() {
    let (_, c) = gen_c();
    let rows = resp_rows(&c);
    let va = |dll: &str, name: &str| rows.iter().find(|r| r.0 == dll && r.1 == name).unwrap_or_else(|| panic!("no row {dll}!{name}")).2;
    let got = lines("gpa-blend");
    assert_eq!(got[0], format!("{:x} 5", va(BLEND2D, "AggCreateInstance")), "the pseudo-VA of the row; success leaves the last error alone");
    assert!(va(BLEND2D, "AggCreateInstance") >= 0x7FF9_0000_0000);
    // exact spelling and case (the page: the name is compared as written)
    for (i, n) in ["aggcreateinstance", "AGGCREATEINSTANCE", "AggCreateInstanc", "AggCreateInstance2", "", "GetLastError", "FontCache_AcquireFont", "DllMain"].iter().enumerate() {
        assert_eq!(got[1 + i], MISS_127, "{n:?}");
    }
    // the name of one module's export under the other module's handle, kernel32's and ntdll's: not found
    for i in 9..12 {
        assert_eq!(got[i], MISS_127, "line {i}");
    }
    assert_eq!(got.len(), 12);
}

#[test]
fn get_proc_address_gives_a_pseudo_va_for_each_of_the_ten_exports_of_qfc() {
    let (_, c) = gen_c();
    let rows = resp_rows(&c);
    let got = lines("gpa-qfc");
    let mut seen = std::collections::BTreeSet::new();
    // the nine exports after the first in the driver's list (the driver starts at index 1 of its table: the Blend2D export is index 0)
    for (i, n) in QFC_EXPORTS.iter().enumerate().take(10) {
        let want = rows.iter().find(|r| r.0 == QFC && r.1 == *n).unwrap().2;
        assert!(want >= 0x7FF9_0000_0000 && seen.insert(want), "{n}: the pseudo-VA is in the response region and unique");
        assert_eq!(got[i], format!("{want:x} 5"), "{n}");
    }
    assert_eq!(seen.len(), 10);
    // names that are not exports: NULL and ERROR_PROC_NOT_FOUND (near names, case, other modules' names, Blend2D's names under QFC's handle)
    for i in 10..(10 + 7 + 4) {
        assert_eq!(got[i], MISS_127, "line {i}: {}", got[i]);
    }
    assert_eq!(got.len(), 10 + 7 + 4);
    // the existing rows are still found under their own dll only
    let g = lines("gpa-others-unchanged");
    let k32 = |n: &str| rows.iter().find(|r| r.0 == "kernel32.dll" && r.1 == n).unwrap().2;
    assert_eq!(g, [format!("{:x} 5", k32("GetLastError")), format!("{:x} 5", k32("GetProcAddress")), MISS_127.to_string(), MISS_127.to_string()]);
}

/// Every one of the 11 pseudo-VAs, called through fle_dispatch as the guest's `call rax` does, stops by name with the four register arguments.
#[test]
fn calling_any_export_stops_by_name_with_the_register_arguments() {
    let all: Vec<(&str, &str)> = BLEND2D_EXPORTS.iter().map(|n| (BLEND2D, *n)).chain(QFC_EXPORTS.iter().map(|n| (QFC, *n))).collect();
    assert_eq!(all.len(), 11);
    for (k, (module, export)) in all.iter().enumerate() {
        let o = run_with(driver(), &format!("stop-{k}"));
        assert!(!o.ok, "{module}!{export}: expected a stop");
        assert_eq!(o.signal, Some(6), "{module}!{export}: {}", o.stderr);
        // the stdout has the GetProcAddress line only (found, last error 5), never "returned"
        assert!(!o.stdout.contains("returned"), "{module}!{export}");
        assert_eq!(
            o.stderr,
            format!(
                "fle: {module}!{export}: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later) RCX=0x1111 RDX=0x2222 R8=0x3333 R9=0x4444\n"
            ),
            "{module}!{export}"
        );
    }
    // the guest's own sequence for Blend2D: LoadLibraryW, GetProcAddress, keep the pointer, call it later
    let o = aborts("kept-pointer-stop", "fle: blend2d.dll!AggCreateInstance: the module is installed but not lifted yet");
    assert!(o.stderr.contains("RCX=0xaa RDX=0x0 R8=0x0 R9=0x0"), "{}", o.stderr);
    assert!(o.stdout.starts_with("kept 1"));
}

#[test]
fn what_decision_0096_does_not_change_still_stops_or_fails_as_before() {
    // Names near the two (the 32-bit QuickFontCache.dll, a trailing point, a doubled extension) are not modules: the old named stop.
    // (scenario, the name as fle_load.c normalizes it): near-2 is "blend2d." and near-7 is "quickfontcache_x64." (the point is removed, nothing is appended)
    let near = [
        ("near-1", "blend2d2.dll"),
        ("near-2", "blend2d"),
        ("near-3", "quickfontcache.dll"),
        ("near-4", "quickfontcache_x86.dll"),
        ("near-5", "quickfontcache_x64.dll.dll"),
        ("near-6", "blend2d.dll.dll"),
        ("near-7", "quickfontcache_x64"),
    ];
    // Decision 0233 (old: each stopped with "fle: LoadLibraryW: module \"<name>\" is not a virtual module and not a name this environment is defined to lack"): these are names of the product's DLLs that no module of the build claims
    // and that are not on the roster of Windows' DLLs, so the search of the (empty) volume ends without a module: the page's failure, NULL and ERROR_MOD_NOT_FOUND (126). No handle is invented.
    for (scenario, shown) in near {
        let o = run_with(driver(), scenario);
        assert!(o.ok, "{scenario} ({shown}): {}", o.stderr);
        assert_eq!(o.stdout.trim_end(), "0 126", "{scenario} ({shown})");
    }
    // GetProcAddress by ordinal and a handle that is not a module handle (past the table, misaligned) stop as for every virtual module
    aborts("gpa-ordinal", "GetProcAddress by ordinal");
    aborts("gpa-past-the-table", "not a virtual module handle");
    aborts("gpa-misaligned", "not a virtual module handle");
}

// ---------------------------------------------------------------------------------------------
// 3. A real module of the same name wins (decision 0096 item 5)
// ---------------------------------------------------------------------------------------------

const RM_BASE: u64 = 0x1_3F00_0000;

#[test]
fn a_real_module_named_blend2d_wins_over_the_deferred_one() {
    let d = real_driver();
    // before the load: a real module is not found (decision 0082), not the deferred handle; QFC is still the deferred module
    let got = lines_with(d, "real-gmh-before");
    assert_eq!(got, [MISS_126.to_string(), MISS_126.to_string(), ok5(N_QFC), ok5(N_QFC), "calls 0".to_string()]);
    // the load runs DllMain once (decision 0082: before LoadLibraryW returns), returns the base, and the counting follows the real module
    let got = lines_with(d, "real-load");
    assert_eq!(got[0], format!("{RM_BASE:x} 5"));
    assert_eq!(got[1], "calls 1");
    assert_eq!(&got[2..5], [format!("{RM_BASE:x} 5"), format!("{RM_BASE:x} 5"), format!("{RM_BASE:x} 5")]);
    assert_eq!(got[5], "calls 1", "DllMain is not entered again");
    // after the load GetModuleHandleW/A answer the base, with the case and extension rules
    assert_eq!(&got[6..9], [format!("{RM_BASE:x} 5"), format!("{RM_BASE:x} 5"), MISS_126.to_string()], "the third name is 'BLEND2D.' = 'blend2d', which is not a module");
    assert_ne!(RM_BASE, handle(N_BLEND2D));
    assert_eq!(got.len(), 9);
}

#[test]
fn with_a_real_blend2d_get_proc_address_reads_its_export_directory_not_the_deferred_rows() {
    let got = lines_with(real_driver(), "real-gpa");
    assert_eq!(got[0], format!("handle {RM_BASE:x}"));
    assert_eq!(got[1], format!("{:x} 5", RM_BASE + 0x2000), "base + the export RVA of the image, not a response-table pseudo-VA");
    assert!(RM_BASE + 0x2000 < 0x7FF9_0000_0000);
    assert_eq!(got[2], MISS_127, "exact case");
    assert_eq!(got[3], MISS_127, "an export of the other deferred module is not an export of this one");
    assert_eq!(got[4], MISS_127);
    // QFC is still the deferred module: its export has a pseudo-VA under the deferred handle
    let (_, c) = gen_c();
    let want = resp_rows(&c).iter().find(|r| r.0 == QFC && r.1 == "FontCache_AcquireFont").unwrap().2;
    assert_eq!(got[5], format!("{want:x} 5"));
}

#[test]
fn a_real_blend2d_next_to_a_deferred_qfc_loads_each_its_own_way() {
    let got = lines_with(real_driver(), "real-qfc");
    // QFC: deferred handle, no DllMain; Blend2D: the base and one DllMain; QFC again: the deferred handle
    assert_eq!(got[0], ok5(N_QFC));
    assert_eq!(got[1], format!("{RM_BASE:x} 5"));
    assert_eq!(got[2], ok5(N_QFC));
    assert_eq!(got[3], "calls 1", "only the real module has a DllMain");
    let (_, c) = gen_c();
    let want = resp_rows(&c).iter().find(|r| r.0 == QFC && r.1 == "FontCache_CleanUp").unwrap().2;
    assert_eq!(got[4], format!("{want:x} 5"));
    assert_eq!(got.len(), 5);
    // calling QFC's export still stops as deferred, with the arguments; the real module's load has happened before in the same run
    let o = aborts_with(
        real_driver(),
        "real-stop-deferred-qfc",
        "fle: quickfontcache_x64.dll!FontCache_AcquireFont: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later) RCX=0x10 RDX=0x20 R8=0x30 R9=0x40",
    );
    assert_eq!(o.stderr.lines().count(), 1);
}

#[test]
fn a_real_blend2d_follows_the_real_module_rules_of_free_library() {
    let d = real_driver();
    // three loads (count 3), three frees succeed: the last FreeLibrary keeps the module resident (decision 0206, always since decision 0243; old: the third stopped with the real-module message of decision 0082 D11)
    let o = run_with(d, "real-refs");
    assert!(o.ok, "real-refs: did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    assert!(o.stdout.contains("two frees"), "{}", o.stdout);
    assert_eq!(o.stdout.lines().filter(|l| *l == "1 5").count(), 3);
    // FreeLibrary of the base before the load: the real-module stop
    aborts_with(d, "real-free-before-load", "FreeLibrary of a real module that is not loaded");
}

#[test]
fn the_driver_without_a_real_module_is_the_deferred_build() {
    // the same scenarios without the table: GetModuleHandle finds the deferred module from the start (N = 33) and the load gives N = 33
    let g = lines("gmh");
    assert_eq!(g[0], ok5(N_BLEND2D));
    let l = lines("load");
    assert_eq!(l[0], ok5(N_BLEND2D));
}
