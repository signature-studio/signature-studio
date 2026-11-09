//! The runtime answers of decision 0102 (provisional) that the walls after Blend2D's DllMain(ATTACH) need: GetVersionExA (W5), HeapCreate and
//! HeapSetInformation (W6), GetStartupInfoA, GetModuleFileNameA, GetEnvironmentStrings and FreeEnvironmentStringsA (W7), EncodePointer and
//! DecodePointer (W8), SetHandleCount (W10), HeapSize (W11) and SPI_GETFONTSMOOTHINGCONTRAST (W13). GetModuleFileNameW and LoadStringW on
//! the handle of a real module (W1, W2) are in tests/realmod.rs; the `ret 0` (W9) and the inline jump table (W3) are in the generator tests
//! at the end of this file and in fe-decode/tests/inline_table.rs.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision, not read back from the C code. The host is Linux:
//! nothing is compared with a real Windows. The driver is linked with every runtime/fle_*.c, like tests/k32_dsp.rs.

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-wallb/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work().join(etxt_common::per_process("rt"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut srcs: Vec<PathBuf> = std::fs::read_dir(rt())
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
                .arg(rt())
                .arg(&s)
                .arg("-o")
                .arg(&o)
                .env("TMPDIR", work().join("tmp"))
                .output()
                .expect("clang not found");
            assert!(cc.status.success(), "clang failed for {}: {}", s.display(), String::from_utf8_lossy(&cc.stderr));
            v.push(o);
        }
        v
    })
}

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_osver.h"
#include "fle_vfs.h"
#include "fle_spi.h"

uint64_t R[16];
fle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };
fle_xmm X[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
static uint64_t stk[64];
#define U(x) ((unsigned long long)(x))
#define ENTER() do { R[4] = (uint64_t)(uintptr_t)&stk[16]; stk[16] = 0xdead0000ull; } while (0)
#define P(x) ((uint64_t)(uintptr_t)(x))
static uint64_t call1(void (*f)(void), uint64_t a) { R[1] = a; f(); return R[0]; }
static uint64_t call3(void (*f)(void), uint64_t a, uint64_t b, uint64_t c) { R[1] = a; R[2] = b; R[8] = c; f(); return R[0]; }
static uint64_t call4(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn run(name: &str, body: &str) -> Out {
    let dir = work();
    let tag = etxt_common::uniq(name);
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, format!("{PRE}\nint main(void)\n{{\n    ENTER();\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-I"]).arg(rt());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines(o: &Out) -> Vec<String> {
    assert!(o.ok, "signal {:?}: {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// The body must stop the run: SIGABRT, `fle: <api>` on stderr, nothing returned.
fn stops(name: &str, api: &str, body: &str) -> String {
    let o = run(name, &format!("{body}\n    printf(\"returned %llu\\n\", U(R[0]));"));
    assert!(!o.ok && !o.stdout.contains("returned"), "{name}: returned: {}", o.stdout);
    assert!(o.stderr.contains(&format!("fle: {api}")), "{name}: no 'fle: {api}': {}", o.stderr);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT");
    o.stderr
}

// ---- W5: GetVersionExA (decision 0102) ------------------------------------------------------------------------------------------

/// Learn, GetVersionExA: the same text as GetVersionExW with OSVERSIONINFOA / OSVERSIONINFOEXA. The structures are 20 + 128 = 148 (0x94) and
/// 156 (0x9c) bytes (DWORD x5, CHAR[128], then WORD, WORD, WORD, BYTE, BYTE). The emulated OS is 10.0.19045, platform 2, workstation (decision 0014).
#[test]
fn get_version_ex_a_fills_the_ansi_structures_like_the_w_form() {
    let o = run(
        "gvexa",
        r#"
    uint8_t b[160]; uint32_t v; uint16_t w; uint32_t sz;
    memset(b, 0xAB, sizeof b); sz = 148; memcpy(b, &sz, 4);
    printf("a ret %llu\n", U(call1(fle_imp_GetVersionExA, P(b))));
    memcpy(&v, b + 4, 4); printf("a major %u\n", v); memcpy(&v, b + 8, 4); printf("a minor %u\n", v);
    memcpy(&v, b + 12, 4); printf("a build %u\n", v); memcpy(&v, b + 16, 4); printf("a plat %u\n", v);
    printf("a csd0 %u csd127 %u tail %x\n", b[20], b[147], b[148]);
    memset(b, 0xAB, sizeof b); sz = 156; memcpy(b, &sz, 4);
    printf("x ret %llu\n", U(call1(fle_imp_GetVersionExA, P(b))));
    memcpy(&w, b + 148, 2); printf("x spmaj %u ", w); memcpy(&w, b + 150, 2); printf("spmin %u ", w);
    memcpy(&w, b + 152, 2); printf("suite %u ", w); printf("prod %u res %x\n", b[154], b[155]);
    memset(b, 0, sizeof b); sz = 276;   /* the W size is not an A size */
    memcpy(b, &sz, 4); R[0] = 7;
    printf("bad ret %llu le %u major %u\n", U(call1(fle_imp_GetVersionExA, P(b))), LE(), *(uint32_t *)(b + 4));
"#,
    );
    assert_eq!(
        lines(&o),
        [
            "a ret 1", "a major 10", "a minor 0", "a build 19045", "a plat 2", "a csd0 0 csd127 0 tail ab", // 148 bytes written, the byte after is not
            "x ret 1", "x spmaj 0 spmin 0 suite 0 prod 1 res ab", // wReserved is not written
            "bad ret 0 le 87 major 0",
        ]
    );
}

#[test]
fn get_version_ex_a_with_a_null_pointer_stops() {
    stops("gvexa-null", "GetVersionExA", "call1(fle_imp_GetVersionExA, 0);");
}

// ---- W6: HeapCreate, HeapSetInformation (decision 0102) --------------------------------------------------------------------------

/// Learn, HeapCreate: a new private heap handle (not the process heap's); HeapAlloc/HeapFree work on it. Learn, HeapSetInformation: the page's own
/// example sequence, HeapSetInformation(NULL, HeapEnableTerminationOnCorruption, NULL, 0) and HeapSetInformation(hHeap, HeapCompatibilityInformation,
/// &2, 4) on a heap made with HeapCreate(0, 0, 0), succeed (nonzero).
#[test]
fn heap_create_gives_private_heaps_and_the_pages_set_information_example_succeeds() {
    let o = run(
        "heapcreate",
        r#"
    uint32_t two = 2, zero = 0;
    uint64_t ph = call1(fle_imp_GetProcessHeap, 0), h1, h2, p, q;
    h1 = call3(fle_imp_HeapCreate, 0, 0x1000, 0);
    h2 = call3(fle_imp_HeapCreate, 0, 0, 0);
    printf("distinct %d %d %d\n", h1 != 0 && h2 != 0, h1 != h2, h1 != ph && h2 != ph);
    printf("term %llu\n", U(call4(fle_imp_HeapSetInformation, 0, 1, 0, 0)));
    printf("lfh %llu le %u\n", U(call4(fle_imp_HeapSetInformation, h1, 0, P(&two), 4)), LE());
    p = call3(fle_imp_HeapAlloc, h1, 8, 100);
    printf("alloc %d zero %d\n", p != 0, ((uint8_t *)p)[99] == 0 && ((uint8_t *)p)[0] == 0);
    printf("free %llu\n", U(call3(fle_imp_HeapFree, h1, 0, p)));
    q = call3(fle_imp_HeapAlloc, h2, 0, 1);
    printf("alloc2 %d\n", q != 0);
    printf("free2 %llu\n", U(call3(fle_imp_HeapFree, h2, 0, q)));
    /* HEAP_NO_SERIALIZE heaps cannot get the LFH (the page) */
    h1 = call3(fle_imp_HeapCreate, 1, 0, 0);
    printf("noser %llu le %u\n", U(call4(fle_imp_HeapSetInformation, h1, 0, P(&two), 4)), LE());
    printf("process lfh %llu\n", U(call4(fle_imp_HeapSetInformation, ph, 0, P(&two), 4)));
"#,
    );
    assert_eq!(
        lines(&o),
        ["distinct 1 1 1", "term 1", "lfh 1 le 0", "alloc 1 zero 1", "free 1", "alloc2 1", "free2 1", "noser 0 le 87", "process lfh 1"]
    );
}

#[test]
fn heap_create_and_set_information_stop_on_what_the_page_leaves_open_or_the_runtime_does_not_model() {
    let cases: &[(&str, &str, &str)] = &[
        ("exec", "HeapCreate", "call3(fle_imp_HeapCreate, 0x40000, 0, 0);"),
        ("fixed", "HeapCreate", "call3(fle_imp_HeapCreate, 0, 0x1000, 0x100000);"),
        ("otherbit", "HeapCreate", "call3(fle_imp_HeapCreate, 0x2, 0, 0);"),
        ("class3", "HeapSetInformation", "call4(fle_imp_HeapSetInformation, 0, 3, 0, 0);"),
        ("class1-info", "HeapSetInformation", "uint32_t z = 0; call4(fle_imp_HeapSetInformation, 0, 1, P(&z), 4);"),
        ("class0-null-heap", "HeapSetInformation", "uint32_t t = 2; call4(fle_imp_HeapSetInformation, 0, 0, P(&t), 4);"),
        ("class0-value1", "HeapSetInformation", "uint32_t t = 1; uint64_t h = call3(fle_imp_HeapCreate, 0, 0, 0); call4(fle_imp_HeapSetInformation, h, 0, P(&t), 4);"),
        ("class0-len", "HeapSetInformation", "uint32_t t = 2; uint64_t h = call3(fle_imp_HeapCreate, 0, 0, 0); call4(fle_imp_HeapSetInformation, h, 0, P(&t), 8);"),
        ("unknown-handle", "HeapSetInformation", "call4(fle_imp_HeapSetInformation, 0x12340000, 1, 0, 0);"),
        ("alloc-on-unknown", "HeapAlloc", "call3(fle_imp_HeapAlloc, 0x7FF700020000ull, 0, 8);"),
    ];
    for (label, api, body) in cases {
        stops(&format!("heapstop-{label}"), api, body);
    }
}

// ---- W11: HeapSize (decision 0102) -----------------------------------------------------------------------------------------------

/// Learn, HeapSize: "the requested size of the allocated memory block"; HeapAlloc(…, 0) was requested with 0 bytes.
#[test]
fn heap_size_returns_the_requested_size_of_heap_alloc_blocks() {
    let o = run(
        "heapsize",
        r#"
    uint64_t ph = call1(fle_imp_GetProcessHeap, 0), h = call3(fle_imp_HeapCreate, 0, 0, 0), a, b, c, d;
    a = call3(fle_imp_HeapAlloc, ph, 0, 100);
    b = call3(fle_imp_HeapAlloc, ph, 8, 1);
    c = call3(fle_imp_HeapAlloc, ph, 0, 0);
    d = call3(fle_imp_HeapAlloc, h, 1, 4096);
    printf("%llu %llu %llu %llu\n", U(call3(fle_imp_HeapSize, ph, 0, a)), U(call3(fle_imp_HeapSize, ph, 0, b)), U(call3(fle_imp_HeapSize, ph, 1, c)), U(call3(fle_imp_HeapSize, h, 0, d)));
    call3(fle_imp_HeapFree, ph, 0, a);
    a = call3(fle_imp_HeapAlloc, ph, 0, 7);   /* the freed address may be reused: the size is the new request */
    printf("again %llu\n", U(call3(fle_imp_HeapSize, ph, 0, a)));
"#,
    );
    assert_eq!(lines(&o), ["100 1 0 4096", "again 7"]);
}

#[test]
fn heap_size_stops_on_what_the_page_does_not_define() {
    let cases: &[(&str, &str)] = &[
        ("null", "call3(fle_imp_HeapSize, call1(fle_imp_GetProcessHeap, 0), 0, 0);"),
        ("flags", "uint64_t p = call3(fle_imp_HeapAlloc, call1(fle_imp_GetProcessHeap, 0), 0, 8); call3(fle_imp_HeapSize, call1(fle_imp_GetProcessHeap, 0), 4, p);"),
        ("not-a-heap-block", "uint8_t x[8]; call3(fle_imp_HeapSize, call1(fle_imp_GetProcessHeap, 0), 0, P(x));"),
        ("freed", "uint64_t ph = call1(fle_imp_GetProcessHeap, 0), p = call3(fle_imp_HeapAlloc, ph, 0, 8); call3(fle_imp_HeapFree, ph, 0, p); call3(fle_imp_HeapSize, ph, 0, p);"),
        ("other-heap", "uint64_t ph = call1(fle_imp_GetProcessHeap, 0), h = call3(fle_imp_HeapCreate, 0, 0, 0), p = call3(fle_imp_HeapAlloc, ph, 0, 8); call3(fle_imp_HeapSize, h, 0, p);"),
        ("bad-handle", "call3(fle_imp_HeapSize, 0x1000, 0, 0x2000);"),
    ];
    for (label, body) in cases {
        stops(&format!("heapsize-{label}"), "HeapSize", body);
    }
}

// ---- W7: the ANSI forms (decision 0102) ------------------------------------------------------------------------------------------

/// GetStartupInfoA fills 0x68 bytes like the W form (decision 0083): every member zero, cb = 0x68, whatever cb held.
#[test]
fn get_startup_info_a_zeroes_the_structure_and_sets_cb() {
    let o = run(
        "gsia",
        r#"
    uint8_t b[0x70]; uint32_t cb; int i, nz = 0;
    memset(b, 0xAB, sizeof b);
    call1(fle_imp_GetStartupInfoA, P(b));
    memcpy(&cb, b, 4);
    for (i = 4; i < 0x68; i++) nz += b[i] != 0;
    printf("cb %x nonzero %d tail %x\n", cb, nz, b[0x68]);
"#,
    );
    assert_eq!(lines(&o), ["cb 68 nonzero 0 tail ab"]);
    stops("gsia-null", "GetStartupInfoA", "call1(fle_imp_GetStartupInfoA, 0);");
}

/// The virtual environment starts empty, so the block is the single terminating NUL; after SetEnvironmentVariableW("AB", "cd") it is
/// "AB=cd\0\0" (Learn: "Var1=Value1\0 ... VarN=ValueN\0\0"). Free takes only a pointer the Get returned.
#[test]
fn get_environment_strings_is_the_ansi_block_and_free_takes_only_its_own_blocks() {
    let o = run(
        "envA",
        r#"
    static const uint16_t nm[] = { 'A', 'B', 0 }, vl[] = { 'c', 'd', 0 };
    char *e = (char *)call1(fle_imp_GetEnvironmentStrings, 0);
    printf("empty %d %d\n", e[0], e[1] == 0 || 1);
    printf("free %llu\n", U(call1(fle_imp_FreeEnvironmentStringsA, P(e))));
    R[1] = P(nm); R[2] = P(vl); fle_imp_SetEnvironmentVariableW();
    e = (char *)call1(fle_imp_GetEnvironmentStrings, 0);
    printf("block %s|%d %d\n", e, e[5], e[6]);
    printf("free %llu\n", U(call1(fle_imp_FreeEnvironmentStringsA, P(e))));
"#,
    );
    assert_eq!(lines(&o), ["empty 0 1", "free 1", "block AB=cd|0 0", "free 1"]);
    stops("envA-free-unknown", "FreeEnvironmentStringsA", "uint8_t x[4]; call1(fle_imp_FreeEnvironmentStringsA, P(x));");
    stops(
        "envA-free-twice",
        "FreeEnvironmentStringsA",
        "char *e = (char *)call1(fle_imp_GetEnvironmentStrings, 0); call1(fle_imp_FreeEnvironmentStringsA, P(e)); call1(fle_imp_FreeEnvironmentStringsA, P(e));",
    );
}

/// GetModuleFileNameA is the ASCII text of GetModuleFileNameW's answer: NULL is the host exe FL64.exe (49 characters; decision 0118, provisional; it was the
/// DLL's path until decision 0117), 0x400000 is the DLL's path of decision 0018 (57 characters); the buffer rules are the W form's: a path that does not
/// fit is truncated with a NUL, nSize is returned and the error is 122; nSize 0 returns 0.
#[test]
fn get_module_file_name_a_is_the_virtual_exe_path_for_null_and_the_dll_path_for_0x400000_with_the_buffer_rules_of_the_w_form() {
    let dll = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
    let exe = "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe";
    assert_eq!(dll.len(), 57);
    assert_eq!(exe.len(), 49);
    let o = run(
        "gmfna",
        r#"
    char b[80]; uint64_t n;
    memset(b, 'z', sizeof b);
    n = call3(fle_imp_GetModuleFileNameA, 0, P(b), 80); printf("%llu %s\n", U(n), b);
    memset(b, 'z', sizeof b);
    n = call3(fle_imp_GetModuleFileNameA, 0x400000, P(b), 58); printf("%llu %s\n", U(n), b);
    memset(b, 'z', sizeof b);
    fle_set_last_error(0);
    n = call3(fle_imp_GetModuleFileNameA, 0, P(b), 10); printf("trunc %llu [%s] %u\n", U(n), b, LE());
    fle_set_last_error(7);
    n = call3(fle_imp_GetModuleFileNameA, 0, P(b), 0); printf("zero %llu %u\n", U(n), LE());
"#,
    );
    assert_eq!(
        lines(&o),
        [format!("49 {exe}"), format!("57 {dll}"), format!("trunc 10 [{}] 122", &exe[..9]), "zero 0 7".to_string()]
    );
    stops("gmfna-virtual", "GetModuleFileNameA", "call3(fle_imp_GetModuleFileNameA, 0x7FF800090000ull, 0x1000, 10);");
    stops("gmfna-null-buffer", "GetModuleFileNameA", "call3(fle_imp_GetModuleFileNameA, 0, 0, 10);");
}

// ---- W8: EncodePointer, DecodePointer (decision 0102) ----------------------------------------------------------------------------

/// Learn: DecodePointer "decodes a pointer that was previously encoded with EncodePointer". The values are the runtime's own; what the pages
/// promise is the round trip, so that is what is asserted (and that the transform is a function and does not collapse values).
#[test]
fn encode_and_decode_pointer_round_trip() {
    let o = run(
        "encdec",
        r#"
    uint64_t v[] = { 0, 1, 0x400000, 0x7FFF00001000ull, 0xFFFFFFFFFFFFFFFFull, 0x8000000000000000ull, 0x123456789abcdefull };
    int i, j, rt = 1, det = 1, inj = 1;
    for (i = 0; i < 7; i++) {
        uint64_t e = call1(fle_imp_EncodePointer, v[i]), e2 = call1(fle_imp_EncodePointer, v[i]);
        rt &= call1(fle_imp_DecodePointer, e) == v[i];
        det &= e == e2;
        for (j = 0; j < i; j++) inj &= e != call1(fle_imp_EncodePointer, v[j]);
    }
    printf("roundtrip %d deterministic %d injective %d\n", rt, det, inj);
    printf("decode-of-plain %d\n", call1(fle_imp_DecodePointer, 0x400000) != 0x400000);
"#,
    );
    assert_eq!(lines(&o), ["roundtrip 1 deterministic 1 injective 1", "decode-of-plain 1"]);
}

// ---- W10: SetHandleCount (decision 0102) -----------------------------------------------------------------------------------------

#[test]
fn set_handle_count_returns_the_requested_count() {
    let o = run(
        "shc",
        r#"
    printf("%llu %llu %llu\n", U(call1(fle_imp_SetHandleCount, 0)), U(call1(fle_imp_SetHandleCount, 20)), U(call1(fle_imp_SetHandleCount, 0xdeadbeef00000000ull | 512)));
"#,
    );
    assert_eq!(lines(&o), ["0 20 512"], "only the low DWORD (UINT) of the argument counts");
}

// ---- W13: SPI_GETFONTSMOOTHINGCONTRAST (decision 0102) ---------------------------------------------------------------------------

/// Learn, SPI_GETFONTSMOOTHINGCONTRAST (0x200C): "pvParam must point to a UINT ... Valid contrast values are from 1000 to 2200. The default value is 1400."
#[test]
fn spi_get_font_smoothing_contrast_is_the_pages_default() {
    let o = run(
        "spi200c",
        r#"
    uint32_t c = 0xABABABAB;
    printf("%llu %u\n", U(call4(fle_imp_SystemParametersInfoW, 0x200C, 0, P(&c), 0)), c);
"#,
    );
    assert_eq!(lines(&o), ["1 1400"]);
    for (label, body) in [
        ("null", "call4(fle_imp_SystemParametersInfoW, 0x200C, 0, 0, 0);"),
        ("uiparam", "uint32_t c; call4(fle_imp_SystemParametersInfoW, 0x200C, 1, P(&c), 0);"),
        ("winini", "uint32_t c; call4(fle_imp_SystemParametersInfoW, 0x200C, 0, P(&c), 1);"),
    ] {
        stops(&format!("spi200c-{label}"), "SystemParametersInfoW", body);
    }
    // SPI_GETFONTSMOOTHINGTYPE (0x200A) was a stop here (decision 0102: the page gives no default, a setting of the machine); decision 0103
    // answers it (FE_FONTSMOOTHINGSTANDARD, tests/wall_q.rs), so the merged tree checks a neighbour that is still not answered: the SET
    // action SPI_SETFONTSMOOTHINGTYPE (0x200B)
    let e = stops("spi200b", "SystemParametersInfoW", "uint32_t c; call4(fle_imp_SystemParametersInfoW, 0x200B, 0, P(&c), 0);");
    assert!(e.contains("0x200b"), "{e}");
}

// ---- W9: `ret 0` (decision 0102), generator --------------------------------------------------------------------------------------

/// A function that is only `ret 0` (c2 00 00: pops no argument bytes) is the same as `ret`. `ret 8` (a callee that pops arguments) was unsupported
/// in decision 0102 because the caller of this convention pops only the return address; decision 0103 D7 lifts it (the callee adds the imm16
/// bytes to RSP, the caller's lifted call adds the 8 of the return address), and the merged tree has that: `ret 0` keeps the handler `ret`, a
/// nonzero count has the handler `ret_imm16` (tests/wall_q_lift.rs checks more values).
#[test]
fn ret_zero_is_ret_and_ret_with_a_count_adds_it_to_rsp() {
    fn lift(code: &[u8]) -> (std::collections::BTreeMap<String, usize>, std::collections::BTreeMap<String, usize>, String) {
        // a PE with one function at RVA 0x1000, found through the entry point
        let mut b = vec![0u8; 0x1000];
        b[0..2].copy_from_slice(b"MZ");
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        let lf = 0x80usize;
        b[lf..lf + 4].copy_from_slice(b"PE\0\0");
        let f = lf + 4;
        b[f..f + 2].copy_from_slice(&0x8664u16.to_le_bytes());
        b[f + 2..f + 4].copy_from_slice(&1u16.to_le_bytes());
        b[f + 16..f + 18].copy_from_slice(&240u16.to_le_bytes());
        b[f + 18..f + 20].copy_from_slice(&0x2022u16.to_le_bytes());
        let o = lf + 24;
        b[o..o + 2].copy_from_slice(&0x20bu16.to_le_bytes());
        b[o + 16..o + 20].copy_from_slice(&0x1000u32.to_le_bytes());
        b[o + 24..o + 32].copy_from_slice(&0x1_4000_0000u64.to_le_bytes());
        b[o + 32..o + 36].copy_from_slice(&0x1000u32.to_le_bytes());
        b[o + 36..o + 40].copy_from_slice(&0x200u32.to_le_bytes());
        b[o + 56..o + 60].copy_from_slice(&0x2000u32.to_le_bytes());
        b[o + 60..o + 64].copy_from_slice(&0x200u32.to_le_bytes());
        b[o + 68..o + 70].copy_from_slice(&3u16.to_le_bytes());
        b[o + 108..o + 112].copy_from_slice(&16u32.to_le_bytes());
        let s = o + 240;
        b[s..s + 5].copy_from_slice(b".text");
        b[s + 8..s + 12].copy_from_slice(&0x100u32.to_le_bytes());
        b[s + 12..s + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        b[s + 16..s + 20].copy_from_slice(&0x200u32.to_le_bytes());
        b[s + 20..s + 24].copy_from_slice(&0x200u32.to_le_bytes());
        b[s + 36..s + 40].copy_from_slice(&0x6000_0020u32.to_le_bytes());
        b.resize(0x400, 0);
        b[0x200..0x200 + code.len()].copy_from_slice(code);
        let mut out = Vec::new();
        let st = fe_lift::generate(&b, &mut out).unwrap();
        (st.handled.iter().map(|(k, v)| (k.to_string(), *v)).collect(), st.unsupported, String::from_utf8_lossy(&out).to_string())
    }
    let (handled, unsupported, c) = lift(&[0xc2, 0x00, 0x00]);
    assert_eq!(handled.get("ret"), Some(&1), "{handled:?}");
    assert!(unsupported.is_empty(), "{unsupported:?}");
    assert!(c.contains("return;"), "{c}");
    let (handled, unsupported, c) = lift(&[0xc2, 0x08, 0x00]);
    assert_eq!(handled.get("ret"), None, "{handled:?}");
    assert_eq!(handled.get("ret_imm16"), Some(&1), "{handled:?}");
    assert!(unsupported.is_empty(), "{unsupported:?}");
    assert!(c.contains("R[4] += 8; return;"), "{c}");
}
