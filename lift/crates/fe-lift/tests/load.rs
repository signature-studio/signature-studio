//! Tests of the library-loader answers and ImageNtHeader (docs/decisions/0029-apis-a6-a12.md): kernel32!LoadLibraryA,
//! LoadLibraryW, LoadLibraryExW, FreeLibrary, SetDllDirectoryW in runtime/fle_load.c and imagehlp!ImageNtHeader in
//! runtime/fle_imagehlp.c.
//!
//! 1. Generator: the six thunks call the runtime, and the response table has their rows (a synthetic PE that imports them).
//! 2. Runtime, return value / last error / output state: expected values are written here from the Microsoft pages
//!    named in decision 0029 and from decision 0015 (module handles 0x7FF800000000 + 0x10000 * N, N from its list),
//!    never read back from fle_load.c. A value the pages do not give is marked "assumption" or "inference".
//! 3. ImageNtHeader on the real FLEngine_x64.dll (mapped by harness/image.c at 0x400000; the expected e_lfanew is read
//!    from the file here) and on small synthetic images.
//! 4. Every input the pages leave open stops the run (SIGABRT, "fle:" message), among them the names of FLEngine's own
//!    plug-in DLLs (M2) and every Windows DLL that decision 0015 does not name.
//!
//! The host is Linux; nothing is compared with a real Windows. The C sources come from runtime/ when fle_load.c is
//! there, else from $FLE_A6_DIR, else from out-a6/work/ (development).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FLE_A6_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_load.c").exists() && runtime_dir().join("fle_imagehlp.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-a6/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-a6/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}
fn harness_dir() -> PathBuf {
    manifest().join("../../harness")
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

/// A PE with one import descriptor per (dll, name) (at most 6); entry 0x1000 calls the thunks at 0x1040 + 0x10 * k.
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

const LOADER_APIS: [&str; 5] = ["LoadLibraryA", "LoadLibraryW", "LoadLibraryExW", "FreeLibrary", "SetDllDirectoryW"];

#[test]
fn the_loader_thunks_and_image_nt_header_call_the_runtime() {
    let imps: Vec<(&str, &str)> = LOADER_APIS.iter().map(|n| ("KERNEL32.dll", *n)).collect();
    let (st, c) = gen(&imps);
    for name in LOADER_APIS {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
    }
    assert!(!st.unsupported.contains_key("Jmp: indirect through IAT slot (not lifted)"));

    let (st, c) = gen(&[("IMAGEHLP.DLL", "ImageNtHeader")]);
    assert_eq!(st.handled["jmp_imp:ImageNtHeader"], 1);
    assert!(c.contains("fle_imp_ImageNtHeader(); return;"));
}

#[test]
fn the_same_name_in_another_dll_is_not_the_same_api() {
    let (st, _) = gen(&[
        ("user32.dll", "FreeLibrary"),
        ("kernel32.dll", "ImageNtHeader"),
        ("imagehlp.dll", "LoadLibraryW"),
        ("kernel32.dll", "LoadLibrary"),
    ]);
    for n in ["jmp_imp:FreeLibrary", "jmp_imp:ImageNtHeader", "jmp_imp:LoadLibraryW", "jmp_imp:LoadLibrary"] {
        assert!(!st.handled.contains_key(n), "{n}");
    }
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 4);
}

#[test]
fn response_table_has_rows_for_the_six_apis() {
    let (_, c) = gen(&[("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW"), ("imagehlp.dll", "ImageNtHeader")]);
    assert!(c.contains("\"kernel32.dll\", \"LoadLibraryW\", UINT64_C("));
    assert!(c.contains("\"imagehlp.dll\", \"ImageNtHeader\", UINT64_C("));
    for n in LOADER_APIS {
        assert!(c.contains(&format!("\"kernel32.dll\", \"{n}\", UINT64_C(")), "{n}");
    }
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

fn clang_obj(src: &Path, obj: &Path, strict: bool) {
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

/// fle_load.c and fle_imagehlp.c (warnings are errors) plus the runtime files they link with, compiled once.
fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (f, strict) in [("fle_load", true), ("fle_imagehlp", true), ("fle_env", false), ("fle_imports", false), ("fle_modules", false), ("fle_string", false)] {
            let src = if f == "fle_load" || f == "fle_imagehlp" { src_dir().join(format!("{f}.c")) } else { runtime_dir().join(format!("{f}.c")) };
            let obj = dir.join(format!("{f}.o"));
            clang_obj(&src, &obj, strict);
            v.push(obj);
        }
        v
    })
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/mman.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_env.h"
#include "fle_load.h"
#include "fle_imagehlp.h"

uint64_t R[16];

static uint64_t callv(void (*f)(void), uint64_t a, uint64_t b, uint64_t c)
{
    R[1] = a; R[2] = b; R[8] = c;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
#define SETLE(v) fle_set_last_error(v)
/* "<handle hex> <last error>" after a call that started with last error 5 */
#define SHOW(f, a, b, c) do { uint64_t r_; SETLE(5); r_ = callv(f, A(a), A(b), A(c)); printf("%llx %u\n", U(r_), (unsigned)LE()); } while (0)
#define LLW(s) SHOW(fle_imp_LoadLibraryW, u##s, 0, 0)
#define LLA(s) SHOW(fle_imp_LoadLibraryA, s, 0, 0)
#define LLEX(s, fl) SHOW(fle_imp_LoadLibraryExW, u##s, 0, fl)
"#;

fn run_main(name: &str, body: &str, extra_c: &[PathBuf], extra_args: &[&str]) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, format!("{PRELUDE}\nint main(int argc, char **argv)\n{{\n    (void)argc; (void)argv;\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir()).arg("-I").arg(harness_dir());
    for o in objs() {
        cmd.arg(o);
    }
    for f in extra_c {
        cmd.arg(f);
    }
    cmd.args(extra_args);
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run_main(name, body, &[], &[]);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message that contains `must_say`.
fn assert_aborts(prefix: &str, cases: &[(&str, &str, &str)]) {
    for (label, stmts, must_say) in cases {
        let body = format!("    {stmts}\n    printf(\"returned %llx\\n\", U(R[0]));");
        let o = run_main(&format!("{prefix}-{label}"), &body, &[], &[]);
        assert!(!o.ok, "{prefix}-{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{prefix}-{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{prefix}-{label}: no fle: message: {}", o.stderr);
        assert!(o.stderr.contains(must_say), "{prefix}-{label}: message lacks {must_say:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{prefix}-{label}: not SIGABRT");
    }
}

/// Handle of module number N of decision 0015 (1-based position in its list).
fn h(n: u64) -> String {
    format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * n)
}

// Module numbers from decision 0015, "N の割り当て": 1 advapi32 ... 9 kernel32 ... 19 user32 ... 25 ntdll.
const N_ADVAPI32: u64 = 1;
const N_CRT_STRING: u64 = 2;
const N_KERNEL32: u64 = 9;
const N_SHELL32: u64 = 16;
const N_SHFOLDER: u64 = 17;
const N_USER32: u64 = 19;
const N_WININET: u64 = 21;
const N_WINSPOOL: u64 = 23;
const N_NTDLL: u64 = 25;

// ---------------------------------------------------------------------------------------------
// 2. LoadLibraryW / LoadLibraryA / LoadLibraryExW
// ---------------------------------------------------------------------------------------------

#[test]
fn load_library_w_returns_the_virtual_handle_by_name() {
    // LoadLibraryW page: a name without a path is matched by base name against loaded modules; ".DLL" is appended when
    // the extension is omitted; the name is compared without case (decision 0015 name rules). Success leaves the last
    // error alone (the page does not say it is cleared): it stays 5.
    let rows: [(&str, u64); 14] = [
        ("kernel32.dll", N_KERNEL32),
        ("KERNEL32.DLL", N_KERNEL32),
        ("Kernel32.Dll", N_KERNEL32),
        ("kernel32", N_KERNEL32),
        ("KERNEL32", N_KERNEL32),
        ("User32.dll", N_USER32),
        ("advapi32", N_ADVAPI32),
        ("SHFolder.dll", N_SHFOLDER),
        ("Shell32", N_SHELL32),
        ("wininet.dll", N_WININET),
        ("NTDLL.DLL", N_NTDLL),
        ("winspool.drv", N_WINSPOOL),
        ("api-ms-win-crt-string-l1-1-0.dll", N_CRT_STRING),
        ("kernel32.dll", N_KERNEL32),
    ];
    let mut body = String::new();
    for (n, _) in rows {
        body += &format!("    LLW(\"{n}\");\n");
    }
    let got = lines("llw-names", &body);
    assert_eq!(got.len(), rows.len());
    for (i, (n, ix)) in rows.iter().enumerate() {
        assert_eq!(got[i], format!("{} 5", h(*ix)), "{n}");
    }
}

#[test]
fn load_library_of_an_empty_name_fails_with_null_and_126() {
    // Decision 0229 (ref1, PROVISIONAL; old: the empty name stopped by name, and decision 0178's exploration switch FLE_LOADLIB_MISSING answered it): an empty name names no module, so "the function cannot find the module,
    // the function fails" and "if the function fails, the return value is NULL" (the LoadLibraryW / LoadLibraryExW pages); the pages name no error code, ERROR_MOD_NOT_FOUND = 126 is the code this runtime gives
    // for a module that is not there (S-ref1-loadlib-empty-code).
    let got = lines("llw-empty", "    LLW(\"\");\n");
    assert_eq!(got, ["0 126"]);
}

#[test]
fn load_library_w_of_a_delay_only_dll_fails_with_126() {
    // Decision 0015: the DLLs that the PE only delay-loads are "not loaded" (GetModuleHandleW gives 126 for them); eight of the nine remained
    // after decision 0099 made shcore.dll virtual module N = 35 (tests/wall_s.rs), and seven remain since decision 0118 made rpcrt4.dll virtual module N = 36.
    // Page: failure returns NULL; the error is ERROR_MOD_NOT_FOUND = 126 (System Error Codes page; the LoadLibrary pages
    // name no code, the value is decision 0015's).
    // decision 0119: uxtheme.dll (N = 38) and wtsapi32.dll (N = 37) are virtual modules now and are no longer in this list of absent names (old: both here, 8 names; new: 6)
    // decision 0122: dwmapi.dll (N = 39) is a virtual module now and is no longer in this list (old: 6 names, dwmapi and DWMAPI.DLL first; new: 4)
    let names = ["imm32.dll", "msimg32", "propsys.dll", "windowscodecs"];
    let mut body = String::new();
    for n in names {
        body += &format!("    LLW(\"{n}\");\n");
    }
    let got = lines("llw-absent", &body);
    assert_eq!(got.len(), names.len());
    for (i, n) in names.iter().enumerate() {
        assert_eq!(got[i], "0 126", "{n}");
    }
}

#[test]
fn load_library_a_uses_the_same_names() {
    let got = lines(
        "lla-names",
        "    LLA(\"kernel32\");\n    LLA(\"USER32.DLL\");\n    LLA(\"Shell32.dll\");\n    LLA(\"ntdll\");\n    LLA(\"imm32.dll\");\n    LLA(\"IMM32\");\n", // decision 0122: dwmapi.dll is a module (old: LLA("dwmapi.dll") here, "0 126"); imm32 is still absent
    );
    assert_eq!(
        got,
        [format!("{} 5", h(N_KERNEL32)), format!("{} 5", h(N_USER32)), format!("{} 5", h(N_SHELL32)), format!("{} 5", h(N_NTDLL)), "0 126".into(), "0 126".into()]
    );
}

#[test]
fn load_library_ex_w_flags() {
    // LoadLibraryExW page: dwFlags 0 is identical to LoadLibrary; the data/image flags on an already loaded module give
    // its handle (and one more reference); the search flags only steer a file search (inference for a loaded module).
    let flags: [(u32, &str); 8] = [
        (0, "0"),
        (0x2, "LOAD_LIBRARY_AS_DATAFILE"),
        (0x20, "LOAD_LIBRARY_AS_IMAGE_RESOURCE"),
        (0x40, "LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE"),
        (0x22, "DATAFILE | IMAGE_RESOURCE"),
        (0x60, "DATAFILE_EXCLUSIVE | IMAGE_RESOURCE"),
        (0x800, "SEARCH_SYSTEM32"),
        (0x1000 | 0x200 | 0x400, "SEARCH_DEFAULT_DIRS | APPLICATION_DIR | USER_DIRS"),
    ];
    let mut body = String::new();
    for (f, _) in flags {
        body += &format!("    LLEX(\"user32.dll\", {f:#x}u);\n");
    }
    // decision 0119: UXTHEME is a virtual module now (N = 38, the handle h(38)); dwmapi.dll is still absent (old: both 126)
    // decision 0122: dwmapi.dll is a virtual module too (N = 39; old: still absent, 126); imm32.dll takes its place in the absent pair
    body += "    LLEX(\"imm32.dll\", 0u);\n    LLEX(\"msimg32.dll\", 0x800u);\n";
    let got = lines("llex-flags", &body);
    assert_eq!(got.len(), flags.len() + 2);
    for (i, (f, label)) in flags.iter().enumerate() {
        assert_eq!(got[i], format!("{} 5", h(N_USER32)), "{label} ({f:#x})");
    }
    assert_eq!(got[flags.len()], "0 126");
    assert_eq!(got[flags.len() + 1], "0 126");
}

#[test]
fn reference_counts_follow_load_and_free() {
    // FreeLibrary page: a module loaded at process start has count one; LoadLibrary and LoadLibraryEx (a loaded module,
    // also with the data flags) add one; FreeLibrary subtracts one and returns nonzero; the last error is untouched.
    let body = r#"
    uint64_t k = 8, u = 18;
    printf("init %u %u %u\n", fle_load_refcount(k), fle_load_refcount(u), fle_load_refcount(24));
    LLW("kernel32.dll"); LLW("KERNEL32"); LLA("kernel32");
    printf("k %u\n", fle_load_refcount(k));
    LLEX("kernel32.dll", 0x2u);
    printf("k %u\n", fle_load_refcount(k));
    SHOW(fle_imp_FreeLibrary, fle_module_handle(k), 0, 0);
    SHOW(fle_imp_FreeLibrary, fle_module_handle(k), 0, 0);
    printf("k %u u %u\n", fle_load_refcount(k), fle_load_refcount(u));
    SHOW(fle_imp_FreeLibrary, fle_module_handle(k), 0, 0);
    SHOW(fle_imp_FreeLibrary, fle_module_handle(k), 0, 0);
    printf("k %u\n", fle_load_refcount(k));
    printf("past %u\n", fle_load_refcount(41));
"#;
    let got = lines("refcount", body);
    let k = h(N_KERNEL32);
    assert_eq!(
        got,
        [
            "init 1 1 1".to_string(),
            format!("{k} 5"),
            format!("{k} 5"),
            format!("{k} 5"),
            "k 4".into(),
            format!("{k} 5"),
            "k 5".into(),
            "1 5".into(),
            "1 5".into(),
            "k 3 u 1".into(),
            "1 5".into(),
            "1 5".into(),
            "k 1".into(),
            "past 0".into(),
        ]
    );
}

#[test]
fn free_library_below_one_stops_and_other_handles_stop() {
    assert_aborts(
        "free",
        &[
            ("count-one", "SETLE(5); callv(fle_imp_FreeLibrary, fle_module_handle(8), 0, 0);", "reference count would reach zero"),
            ("after-balanced", "LLW(\"user32\"); callv(fle_imp_FreeLibrary, fle_module_handle(18), 0, 0); callv(fle_imp_FreeLibrary, fle_module_handle(18), 0, 0);", "reference count would reach zero"),
            ("null", "callv(fle_imp_FreeLibrary, 0, 0, 0);", "not a virtual module handle"),
            ("lifted-dll", "callv(fle_imp_FreeLibrary, 0x400000, 0, 0);", "lifted DLL itself"),
            ("garbage", "callv(fle_imp_FreeLibrary, 0x12345678, 0, 0);", "not a virtual module handle"),
            ("misaligned", "callv(fle_imp_FreeLibrary, fle_module_handle(8) + 1, 0, 0);", "not a virtual module handle"),
            ("past-table", "callv(fle_imp_FreeLibrary, fle_module_handle(fle_module_count()), 0, 0);", "not a virtual module handle"),
        ],
    );
}

#[test]
fn undefined_load_library_inputs_stop() {
    let long_w = "static uint16_t big[400]; { int i; for (i = 0; i < 399; i++) big[i] = 'a'; } callv(fle_imp_LoadLibraryW, A(big), 0, 0);";
    let long_a = "static char big[400]; memset(big, 'a', 399); callv(fle_imp_LoadLibraryA, A(big), 0, 0);";
    assert_aborts(
        "ll",
        &[
            ("w-null", "callv(fle_imp_LoadLibraryW, 0, 0, 0);", "lpLibFileName is NULL"),
            ("a-null", "callv(fle_imp_LoadLibraryA, 0, 0, 0);", "lpLibFileName is NULL"),
            ("ex-null", "callv(fle_imp_LoadLibraryExW, 0, 0, 0);", "lpLibFileName is NULL"),
            ("w-long", long_w, "longer than 256"),
            ("a-long", long_a, "longer than 256"),
            ("ex-hfile", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 1, 0);", "hFile is not NULL"),
            ("ex-dont-resolve", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x1);", "not modelled"),
            ("ex-altered-search", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x8);", "not modelled"),
            ("ex-dll-load-dir", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x100);", "not modelled"),
            ("ex-require-signed", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x80);", "not modelled"),
            ("ex-safe-current", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x2000);", "not modelled"),
            ("ex-unknown-bit", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x80000000u);", "not modelled"),
            ("ex-datafile-both", "callv(fle_imp_LoadLibraryExW, A(u\"kernel32\"), 0, 0x42);", "the page says not to"),
        ],
    );
}

#[test]
fn external_and_unnamed_modules_fail_with_null_and_126() {
    // Decision 0233 (load1, PROVISIONAL; old: each of these names stopped the run with "module ... is not a virtual module", and decision 0209's FLE_SOFT_STOP=1 / decision 0178's FLE_LOADLIB_MISSING answered NULL / 126
    // in an exploration): a bare module name that no module of the build claims and that is not a DLL of the Windows roster (the next test) is searched for in the folders of the standard search order, and the virtual volume
    // (a link of the loader alone has none) holds no file there, so "the function cannot find the module, the function fails" and "If the function fails, the return value is NULL" (LoadLibraryW / LoadLibraryExW pages,
    // Dynamic-link library search order page); the pages name no code: ERROR_MOD_NOT_FOUND (126) is the code of this file for a module that is not there (decision 0015; Wine 10.0 gives it for a DLL that is not found,
    // UNOFFICIAL: S-load1-code). FLEngine's own plug-in/tool DLLs (M2) and the product's DLLs are of this kind; "avrt" and "PowrProf.dll" were virtual modules since decision 0088.
    let names = [
        "dsp_ippv2.dll",
        "dsp_ippv2",
        "stemextract.dll",
        "declipper.dll",
        "beat_tracker.dll",
        "ILTools_x64.dll",
        "oggio_x64.dll",
        "Elastique_x64.dll",
        "wavpackdll_x64.dll",
        "DelZip64.dll",
        // old stops of undefined_load_library_inputs_stop that are plain "not there" names: a trailing point (a file without extension), a double extension, no extension (".dll" is appended: winspool.dll is not a
        // Windows DLL, winspool.drv is), a non-ASCII letter
        "kernel32.",
        "kernel32.dll.dll",
        "winspool",
        "kernel3\\u0131.dll",
    ];
    let mut body = String::new();
    for n in names {
        body += &format!("    LLW(\"{n}\");\n");
    }
    // the ANSI form: a byte above 0x7F is just a letter of a name that nothing has
    body += "    LLA(\"kernel3\\xe9.dll\");\n";
    let got = lines("ext-fail", &body);
    assert_eq!(got.len(), names.len() + 1);
    for (i, n) in names.iter().enumerate() {
        assert_eq!(got[i], "0 126", "{n}");
    }
    assert_eq!(got[names.len()], "0 126");
}

#[test]
fn windows_dlls_that_are_not_virtual_modules_and_api_set_names_stop_by_name() {
    // Decision 0233: the roster (runtime/fle_win_dll_roster.inc, UNOFFICIAL: the dlls/ directory names of Wine 10.0 and the ten names of decision 0015's investigation) lists DLLs that a real Windows ships. For one that this
    // environment holds no virtual module for, "not found" would be untrue and no handle is invented, so the run stops by name, as every one of these stopped before decision 0233 (the old test external_and_unnamed_modules_stop_by_name).
    // (Decision 0238: the API set names that stopped here -- api-ms-win-core-string-l1-1-0.dll, ext-ms-win-com-ole32-l1-1-5, api-ms-win-core-ums-l1-1-0.dll, API-MS-WIN-CORE-HEAP-L2-1-0.DLL -- are the loader's redirection to a host now:
    // the tests load2_* below. The test name keeps "api_set_names" for the one that remains: a contract whose host is no virtual module still stops, load2_a_contract_whose_host_is_not_a_virtual_module_stops_by_name.)
    let names = ["PSAPI.dll", "RICHED20.DLL", "URLMON.DLL", "iphlpapi.dll", "ws2_32.dll", "ucrtbase.dll", "msvcp140", "windows.media.dll"];
    let cases: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.replace(['.', '_'], "-"), format!("callv(fle_imp_LoadLibraryW, A(u\"{n}\"), 0, 0);"), "is not a virtual module and not a name this environment is defined to lack".to_string()))
        .collect();
    let refs: Vec<(&str, &str, &str)> = cases.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())).collect();
    assert_aborts("roster", &refs);
    // The stop message names the module (so the run's log says which module was reached).
    let o = run_main("roster-msg", "    callv(fle_imp_LoadLibraryW, A(u\"Ws2_32.DLL\"), 0, 0);", &[], &[]);
    assert!(o.stderr.contains("fle: LoadLibraryW: module \"ws2_32.dll\""), "{}", o.stderr);
    // Not API set names (no l<n>-<n>-<n> tail, a wrong prefix, a non-digit): not on the roster either, so the search of the volume ends without a module
    let not_sets = ["api-ms-win-core-string.dll", "xapi-ms-win-core-x-l1-1-0.dll", "api-ms-win-core-x-l1-1.dll", "api-ms-win-core-x-l1-1-a.dll", "api-ms-win-core_x-l1-1-0.dll", "ext-x-l1-1-0.dll.dll"];
    let mut body = String::new();
    for n in not_sets {
        body += &format!("    LLW(\"{n}\");\n");
    }
    let got = lines("roster-not-sets", &body);
    // "ext-x-l1-1-0.dll.dll": the name rules append nothing (it has a point) and it does not end with ".dll" once: not a set, not on the roster
    assert_eq!(got.len(), not_sets.len());
    for (i, n) in not_sets.iter().enumerate() {
        assert_eq!(got[i], "0 126", "{n}");
    }
}

#[test]
fn the_roster_is_sorted_unique_lower_case_ascii_and_has_no_name_of_the_product() {
    let t = std::fs::read_to_string(runtime_dir().join("fle_win_dll_roster.inc")).unwrap();
    let names: Vec<&str> = t.lines().filter(|l| l.starts_with('"')).map(|l| l.trim_end_matches(',').trim_matches('"')).collect();
    assert!(names.len() > 500, "{} names", names.len());
    for w in names.windows(2) {
        assert!(w[0].as_bytes() < w[1].as_bytes(), "not sorted or not unique at {} / {}", w[0], w[1]);
    }
    for n in &names {
        assert!(n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'_'), "{n}");
        assert!(!n.starts_with("wine"), "{n}");
    }
    // the ten names of decision 0015's investigation are on it; none of the product's own DLLs is (the roster is a list of Windows' DLLs, not of what the product needs)
    for n in ["psapi.dll", "riched20.dll", "urlmon.dll", "ws2_32.dll", "mswsock.dll", "iphlpapi.dll", "wship6.dll", "fwpuclnt.dll", "idndl.dll", "normaliz.dll", "ucrtbase.dll", "dwmapi.dll"] {
        assert!(names.contains(&n), "{n} is not on the roster");
    }
    for n in ["oggio_x64.dll", "elastique_x64.dll", "ilvideo_x64.dll", "wavpackdll_x64.dll", "delzip64.dll", "dsp_ippv2_x64.dll", "blend2d.dll", "quickfontcache_x64.dll", "freetype_x64.dll", "iltools_x64.dll", "flengine_x64.dll"] {
        assert!(!names.contains(&n), "{n} is on the roster");
    }
}

// ---------------------------------------------------------------------------------------------
// Decision 0088: avrt.dll (N = 31) and powrprof.dll (N = 32) are virtual modules
// ---------------------------------------------------------------------------------------------

// Appended after the 4 API set names of decision 0081 (N = 27..30); decision 0015: append only.
const N_AVRT: u64 = 31;
const N_POWRPROF: u64 = 32;

#[test]
fn avrt_and_powrprof_load_by_name_and_get_module_handle_gives_the_same_handle() {
    // Decision 0088 and the name rules of decision 0015 (the LoadLibraryW page: ".DLL" is appended when the extension is
    // omitted, compared without case). The guest spells them "avrt" and "PowrProf.dll" (investigation 4.1, 4.2). Success
    // leaves the last error alone (5). GetModuleHandleW gives the same handle for the same name.
    let rows: [(&str, u64); 8] = [
        ("avrt", N_AVRT),
        ("avrt.dll", N_AVRT),
        ("AVRT", N_AVRT),
        ("Avrt.DLL", N_AVRT),
        ("PowrProf.dll", N_POWRPROF),
        ("powrprof", N_POWRPROF),
        ("POWRPROF.DLL", N_POWRPROF),
        ("PowrProf", N_POWRPROF),
    ];
    let mut body = String::new();
    for (n, _) in rows {
        body += &format!("    LLW(\"{n}\");\n");
    }
    body += "    LLA(\"avrt\");\n    LLA(\"POWRPROF.DLL\");\n    LLEX(\"avrt\", 0x800u);\n    LLEX(\"PowrProf.dll\", 0u);\n";
    body += "    { uint64_t g; g = callv(fle_imp_GetModuleHandleW, A(u\"avrt\"), 0, 0); printf(\"gmh %llx\\n\", U(g));\n";
    body += "      g = callv(fle_imp_GetModuleHandleW, A(u\"PowrProf.dll\"), 0, 0); printf(\"gmh %llx\\n\", U(g)); }\n";
    let got = lines("avrt-load", &body);
    assert_eq!(got.len(), rows.len() + 4 + 2);
    for (i, (n, ix)) in rows.iter().enumerate() {
        assert_eq!(got[i], format!("{} 5", h(*ix)), "{n}");
    }
    let (a, p) = (format!("{} 5", h(N_AVRT)), format!("{} 5", h(N_POWRPROF)));
    assert_eq!(&got[rows.len()..rows.len() + 4], [a, p.clone(), format!("{} 5", h(N_AVRT)), p]);
    assert_eq!(got[rows.len() + 4], format!("gmh {}", h(N_AVRT)));
    assert_eq!(got[rows.len() + 5], format!("gmh {}", h(N_POWRPROF)));
}

#[test]
fn avrt_and_powrprof_have_nothing_to_find_and_dwmapi_and_uxtheme_stay_absent() {
    // Decision 0088: GetProcAddress on the new handles is NULL and 127 for the guest's names (no response-table row; here no
    // table is linked at all, tests/modules.rs checks the same with a table that has rows). Decision 0015/0029: dwmapi and
    // uxtheme are still not loaded (NULL, 126), in the same body as avrt and PowrProf. (shcore.dll was the second absent name here until
    // decision 0099 made it virtual module N = 35; tests/wall_s.rs checks it. Decision 0119 made uxtheme.dll virtual module N = 38: LoadLibraryW("UxTheme.dll")
    // answers its handle and leaves the last error (5) now; old: "0 126". dwmapi stays absent.)
    let body = r#"
    uint64_t a, p, f;
    SETLE(5); LLW("imm32.dll"); /* decision 0122: dwmapi is a virtual module (N = 39: "h(39) 5"); imm32 stays absent and takes its place here (old: LLW("dwmapi") gave "0 126") */
    a = callv(fle_imp_LoadLibraryW, A(u"avrt"), 0, 0);
    SETLE(5); LLW("UxTheme.dll");
    p = callv(fle_imp_LoadLibraryW, A(u"PowrProf.dll"), 0, 0);
    SETLE(5); f = callv(fle_imp_GetProcAddress, a, A("AvSetMmThreadCharacteristicsW"), 0);
    printf("gpa-avrt %llx %u\n", U(f), (unsigned)LE());
    SETLE(5); f = callv(fle_imp_GetProcAddress, p, A("PowerGetActiveScheme"), 0);
    printf("gpa-powrprof %llx %u\n", U(f), (unsigned)LE());
    SETLE(5); f = callv(fle_imp_GetProcAddress, a, A("PowerGetActiveScheme"), 0);
    printf("gpa-swapped %llx %u\n", U(f), (unsigned)LE());
    printf("same %d %d\n", a == fle_module_handle(30), p == fle_module_handle(31));
"#;
    let got = lines("avrt-gpa", body);
    assert_eq!(got, ["0 126".to_string(), format!("{} 5", h(38)), "gpa-avrt 0 127".into(), "gpa-powrprof 0 127".into(), "gpa-swapped 0 127".into(), "same 1 1".into()]);
}

#[test]
fn avrt_and_powrprof_follow_the_free_library_rule_of_the_other_virtual_modules() {
    // FreeLibrary page and decision 0082 D11 (unchanged): a module loaded at process start has count one; LoadLibrary adds one;
    // FreeLibrary subtracts one and returns nonzero (last error untouched); a count of one stops by name.
    let body = r#"
    printf("init %u %u\n", fle_load_refcount(30), fle_load_refcount(31));
    LLW("avrt"); LLW("PowrProf.dll"); LLW("powrprof");
    printf("count %u %u\n", fle_load_refcount(30), fle_load_refcount(31));
    SHOW(fle_imp_FreeLibrary, fle_module_handle(30), 0, 0);
    SHOW(fle_imp_FreeLibrary, fle_module_handle(31), 0, 0);
    SHOW(fle_imp_FreeLibrary, fle_module_handle(31), 0, 0);
    printf("count %u %u\n", fle_load_refcount(30), fle_load_refcount(31));
    printf("past %u\n", fle_load_refcount(41)); /* decision 0238: 41 modules (combase.dll after kernelbase.dll); 40 modules (old: 39 after decision 0122, 38 with 38 modules) */
"#;
    let got = lines("avrt-refs", body);
    let (a, p) = (h(N_AVRT), h(N_POWRPROF));
    assert_eq!(
        got,
        [
            "init 1 1".to_string(),
            format!("{a} 5"),
            format!("{p} 5"),
            format!("{p} 5"),
            "count 2 3".into(),
            "1 5".into(),
            "1 5".into(),
            "1 5".into(),
            "count 1 1".into(),
            "past 0".into(),
        ]
    );
    assert_aborts(
        "avrt-free",
        &[
            ("avrt-count-one", "SETLE(5); callv(fle_imp_FreeLibrary, fle_module_handle(30), 0, 0);", "reference count would reach zero"),
            ("powrprof-count-one", "SETLE(5); callv(fle_imp_FreeLibrary, fle_module_handle(31), 0, 0);", "reference count would reach zero"),
            (
                "avrt-after-balanced",
                "LLW(\"avrt\"); callv(fle_imp_FreeLibrary, fle_module_handle(30), 0, 0); callv(fle_imp_FreeLibrary, fle_module_handle(30), 0, 0);",
                "reference count would reach zero",
            ),
            ("past-the-new-end", "callv(fle_imp_FreeLibrary, fle_module_handle(41), 0, 0);", "not a virtual module handle"), // decision 0238: 41 modules (combase.dll after kernelbase.dll); 40 modules (old: 39 after decision 0122, 38 before)
        ],
    );
}

#[test]
fn the_other_ten_windows_dlls_still_stop_by_name_after_decision_0088() {
    // Decision 0088 leaves these ten names of the investigation alone: no handle and no failure is invented for them. Decision 0233: they are on the roster of Windows' DLLs (runtime/fle_win_dll_roster.inc), so they stop by name
    // still (the test is unchanged).
    let names = ["psapi", "riched20", "urlmon", "ws2_32", "mswsock", "iphlpapi", "wship6", "fwpuclnt", "idndl", "normaliz"];
    let cases: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.to_string(), format!("callv(fle_imp_LoadLibraryW, A(u\"{n}\"), 0, 0);"), format!("module \"{n}.dll\" is not a virtual module")))
        .collect();
    let refs: Vec<(&str, &str, &str)> = cases.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())).collect();
    assert_aborts("ten", &refs);
}

// ---------------------------------------------------------------------------------------------
// SetDllDirectoryW
// ---------------------------------------------------------------------------------------------

#[test]
fn set_dll_directory_stores_replaces_and_restores() {
    // SetDllDirectoryW page: NULL restores the default order; "" removes the current directory from the default order;
    // another string is added; each call replaces the previous one. Return nonzero; the page lists no failure.
    let body = r#"
    const uint16_t *d;
    d = fle_load_dll_directory(); printf("start %s\n", d ? "set" : "null");
    SHOW(fle_imp_SetDllDirectoryW, u"C:\\Plugins", 0, 0);
    d = fle_load_dll_directory(); printf("dir %s %u %04x\n", d ? "set" : "null", d ? (unsigned)fle_w_len(d) : 0u, d ? d[0] : 0);
    SHOW(fle_imp_SetDllDirectoryW, u"D:\\Other\\x", 0, 0);
    d = fle_load_dll_directory(); printf("dir %u %04x %04x %04x\n", (unsigned)fle_w_len(d), d[0], d[2], d[3]);
    SHOW(fle_imp_SetDllDirectoryW, u"", 0, 0);
    d = fle_load_dll_directory(); printf("empty %s %u\n", d ? "set" : "null", d ? (unsigned)fle_w_len(d) : 99u);
    SHOW(fle_imp_SetDllDirectoryW, 0, 0, 0);
    d = fle_load_dll_directory(); printf("null %s\n", d ? "set" : "null");
    SHOW(fle_imp_SetDllDirectoryW, 0, 0, 0);
"#;
    let got = lines("setdlldir", body);
    assert_eq!(
        got,
        [
            "start null",
            "1 5",
            "dir set 10 0043",
            "1 5",
            "dir 10 0044 005c 004f",
            "1 5",
            "empty set 0",
            "1 5",
            "null null",
            "1 5",
        ]
    );
}

#[test]
fn set_dll_directory_does_not_change_what_a_load_finds() {
    // The directory is only stored: the virtual file system holds no module file, so loads answer as before.
    let got = lines(
        "setdlldir-load",
        "    SHOW(fle_imp_SetDllDirectoryW, u\"C:\\\\Plugins\", 0, 0);\n    LLW(\"kernel32\");\n    LLW(\"imm32\");\n", // decision 0122: imm32 (old: dwmapi, which is a virtual module now)
    );
    assert_eq!(got, ["1 5".to_string(), format!("{} 5", h(N_KERNEL32)), "0 126".into()]);
}

#[test]
fn set_dll_directory_too_long_stops() {
    assert_aborts(
        "setdlldir-long",
        &[("long", "static uint16_t big[40000]; { int i; for (i = 0; i < 39999; i++) big[i] = 'a'; } callv(fle_imp_SetDllDirectoryW, A(big), 0, 0);", "longer than the supported limit")],
    );
}

// ---------------------------------------------------------------------------------------------
// 3. ImageNtHeader
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
fn image_nt_header_on_the_real_dll() {
    let Some(path) = real_dll_path() else { return };
    // Expected values come from the file, parsed here: e_lfanew at 0x3c (PE Format page), Machine at +4 of the NT headers
    // (0x8664), Magic at +24 (0x20b), ImageBase at +48.
    let bytes = std::fs::read(&path).unwrap();
    let lfanew = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as u64;
    let image_base = u64::from_le_bytes(bytes[lfanew as usize + 48..lfanew as usize + 56].try_into().unwrap());
    assert_eq!(image_base, 0x400000, "the fixture's ImageBase");
    let nt = 0x400000 + lfanew;
    let dir = work_dir();
    let name = "inth-real";
    let body = r#"
    harness_image img;
    char err[256];
    if (image_load(argv[1], &img, err, sizeof err) != 0) { fprintf(stderr, "load: %s\n", err); return 3; }
    SETLE(5);
    R[1] = 0x400000; R[0] = 0xBAD; fle_imp_ImageNtHeader();
    printf("%llx %u\n", U(R[0]), (unsigned)LE());
    printf("%08x %04x %04x\n", (unsigned)fle_ld(R[0] ? 0x400000 + fle_ld(0x40003c, 4) : 0, 4), (unsigned)fle_ld(0x400000 + fle_ld(0x40003c, 4) + 4, 2), (unsigned)fle_ld(0x400000 + fle_ld(0x40003c, 4) + 24, 2));
"#;
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    std::fs::write(&c, format!("{PRELUDE}\n#include \"boot.h\"\nint main(int argc, char **argv)\n{{\n    (void)argc;\n{body}\n    return 0;\n}}\n")).unwrap();
    let exe = dir.join(&uname);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir()).arg("-I").arg(harness_dir());
    for o in objs() {
        cmd.arg(o);
    }
    cmd.arg(harness_dir().join("image.c"));
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).arg(&path).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let out = String::from_utf8_lossy(&r.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got[0], format!("{nt:x} 5"), "result is Base + e_lfanew, last error untouched");
    assert_eq!(got[1], "00004550 8664 020b", "the pointed-to structure: Signature, Machine, Magic");
}

/// Maps `len` bytes at 0x400000, fills them with `fill` (C statements on `p`), and calls ImageNtHeader(base).
fn synth(setup: &str, base: &str) -> String {
    format!(
        "    {{\n        uint8_t *p = mmap((void *)0x400000, 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);\n\
         if (p != (uint8_t *)0x400000) {{ fprintf(stderr, \"map failed\\n\"); return 3; }}\n        {setup}\n    }}\n    SETLE(5); R[1] = {base}; R[0] = 0xBAD; fle_imp_ImageNtHeader();\n"
    )
}

#[test]
fn image_nt_header_on_synthetic_images() {
    // Page: the result points at the structure that starts at the "PE\0\0" signature, whose file offset is the 4-byte
    // value at 0x3c (PE Format). Three offsets: the usual 0x80, the smallest the runtime accepts (0x40, the size of the
    // DOS header) and the largest (0xFFF, an unaligned one: the structure is read byte-wise).
    for (name, lfanew) in [("usual", 0x80u32), ("smallest", 0x40), ("largest", 0xFFF)] {
        let setup = format!("p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = {lfanew}; memcpy(p + {lfanew}, \"PE\\0\\0\", 4);");
        let body = format!("{}    printf(\"%llx %u\\n\", U(R[0]), (unsigned)LE());\n", synth(&setup, "0x400000"));
        let got = lines(&format!("inth-{name}"), &body);
        assert_eq!(got, [format!("{:x} 5", 0x400000u64 + lfanew as u64)], "{name}");
    }
}

#[test]
fn image_nt_header_stops_on_bad_headers_and_other_bases() {
    let good = "p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0x80; memcpy(p + 0x80, \"PE\\0\\0\", 4);";
    let case = |setup: &str, base: &str| synth(setup, base);
    let cases: Vec<(&str, String, &str)> = vec![
        ("no-mz", case("p[0] = 'X'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0x80; memcpy(p + 0x80, \"PE\\0\\0\", 4);", "0x400000"), "no MZ signature"),
        ("lfanew-too-small", case("p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0x3f; memcpy(p + 0x3f, \"PE\\0\\0\", 4);", "0x400000"), "out of range"),
        ("lfanew-zero", case("p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0;", "0x400000"), "out of range"),
        ("lfanew-too-large", case("p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0x1000; memcpy(p + 0x1000, \"PE\\0\\0\", 4);", "0x400000"), "out of range"),
        ("no-pe-signature", case("p[0] = 'M'; p[1] = 'Z'; *(uint32_t *)(p + 0x3c) = 0x80; memcpy(p + 0x80, \"PX\\0\\0\", 4);", "0x400000"), "no PE signature"),
        ("base-null", case(good, "0"), "Base is not the lifted image"),
        ("base-unaligned", case(good, "0x400001"), "Base is not the lifted image"),
        ("base-image-base-of-the-file", case(good, "0x140000000ull"), "Base is not the lifted image"),
        ("base-module-handle", case(good, "fle_module_handle(8)"), "Base is not the lifted image"),
        ("base-heap-pointer", case(good, "A(malloc(64))"), "Base is not the lifted image"),
    ];
    for (label, setup, must_say) in &cases {
        let body = format!("{setup}    printf(\"returned %llx\\n\", U(R[0]));\n");
        let o = run_main(&format!("inth-abort-{label}"), &body, &[], &[]);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle: ImageNtHeader:"), "{label}: {}", o.stderr);
        assert!(o.stderr.contains(must_say), "{label}: message lacks {must_say:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

// ---------------------------------------------------------------------------------------------
// Through the generated response table and fle_dispatch
// ---------------------------------------------------------------------------------------------

fn all_runtime_sources() -> Vec<PathBuf> {
    let rt = runtime_dir();
    let mut v: Vec<PathBuf> = std::fs::read_dir(&rt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("fle_") && n.ends_with(".c")
        })
        .collect();
    for f in ["fle_load.c", "fle_imagehlp.c"] {
        if !rt.join(f).exists() {
            v.push(src_dir().join(f));
        }
    }
    v.sort();
    v
}

#[test]
fn get_proc_address_then_dispatch_reaches_the_loader_answers() {
    let (_, c) = gen(&[("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW"), ("imagehlp.dll", "ImageNtHeader")]);
    let dir = work_dir();
    let tag = etxt_common::uniq("gen-loader");
    let gen_c = dir.join(format!("{tag}.c"));
    std::fs::write(&gen_c, &c).unwrap();
    let main = r#"#include <stdio.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"

int main(void)
{
    static const uint16_t dw[] = { 'i','m','m','3','2', 0 }; /* decision 0122: dwmapi is a virtual module now; imm32 is still absent (old: "dwmapi") */
    static const char ll[] = "LoadLibraryW";
    static const char ih[] = "ImageNtHeader";
    uint64_t va;
    fle_set_last_error(5);
    R[1] = fle_module_handle(8); R[2] = (uint64_t)(uintptr_t)ll; fle_imp_GetProcAddress();
    va = R[0];
    if (va == 0) { printf("FAIL no LoadLibraryW\n"); return 1; }
    R[1] = (uint64_t)(uintptr_t)dw; R[0] = 0; fle_dispatch(va);
    printf("ll %llu\n", (unsigned long long)R[0]);
    fle_imp_GetLastError(); printf("le %llu\n", (unsigned long long)R[0]);
    R[1] = fle_module_handle(7); R[2] = (uint64_t)(uintptr_t)ih; fle_imp_GetProcAddress();
    printf("ih %s\n", R[0] ? "found" : "missing");
    return 0;
}
"#;
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, main).unwrap();
    let exe = dir.join(&tag);
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for p in all_runtime_sources() {
        cmd.arg(p);
    }
    let cc = cmd.arg(&gen_c).arg(&mainc).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().unwrap();
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    // LoadLibraryW("imm32") reached through GetProcAddress and fle_dispatch: NULL and 126 (decision 0122: it was "dwmapi", which is a virtual module now). imagehlp's ImageNtHeader is
    // reachable through the imagehlp.dll handle (module 8 = index 7).
    assert_eq!(out.lines().collect::<Vec<_>>(), ["ll 0", "le 126", "ih found"]);
}


// ---------------------------------------------------------------------------------------------
// Decision 0233 (load1, PROVISIONAL): a module name with a path, and the search of the virtual volume
// ---------------------------------------------------------------------------------------------
//
// Expected values are written from the Microsoft Learn pages (LoadLibraryW: "If the string specifies a full path, the function searches only that path for the module", "If the string specifies a relative path or a module name
// without a path, the function uses a standard search strategy", "If the function cannot find the module, the function fails", "If the function fails, the return value is NULL"; LoadLibraryExW: "If lpFileName specifies a
// relative path, the entire relative path is appended to every token in the DLL search path list"; Dynamic-link library search order: the folders), never read back from fle_load.c. The pages name no error code: 126
// (ERROR_MOD_NOT_FOUND, decision 0015) is also what Wine 10.0 returns for a file or folder that does not exist (UNOFFICIAL: ntdll/loader.c open_dll_file, ntdll/error.h). These runs link fle_vfs.c, so the volume exists
// (it starts empty: fle_vfs.h).

fn vfs_c() -> Vec<PathBuf> {
    vec![runtime_dir().join("fle_vfs.c")]
}

/// The runs below put files on the volume first (fle_vfs_put_file: the browser bridge's call, decision 0193).
const PUT: &str = r#"
static void put_file(const uint16_t *dir, const uint16_t *file)
{
    uint8_t *d = (uint8_t *)malloc(4);
    memcpy(d, "MZ\0\0", 4);
    if (fle_vfs_make_dirs(dir) != 0 || fle_vfs_put_file(file, d, 4) != 0) { printf("put failed\n"); exit(3); }
}
"#;

/// run_main with fle_vfs.c linked and fle_vfs.h / the put_file helper declared before main (the body is the text of main).
fn run_main_with_prelude(name: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, format!("{PRELUDE}\n#include \"fle_vfs.h\"\n{PUT}\nint main(int argc, char **argv)\n{{\n    (void)argc; (void)argv;\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir()).arg("-I").arg(harness_dir());
    for o in objs() {
        cmd.arg(o);
    }
    for f in vfs_c() {
        cmd.arg(f);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

#[test]
fn load1_a_path_that_the_volume_lacks_fails_with_null_and_126() {
    // the reference project's two names (the engine asks for them while it reads an MP3 and a Vorbis-in-WAV sample), the same with the other spellings of the A/Ex forms, and other forms of a path
    let body = r#"
    LLW("C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\oggio_x64.dll");
    LLW("ffmpeg\\x64\\ilvideo_x64.dll");
    LLW("ffmpeg/x64/ilvideo_x64.dll");
    LLW(".\\x.dll");
    LLW("..\\x.dll");
    LLW("\\Program Files\\x.dll");
    LLW("D:\\plugins\\x.dll");
    LLA("ffmpeg\\x64\\ilvideo_x64.dll");
    LLA("C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\oggio_x64.dll");
    LLEX("C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\oggio_x64.dll", 0);
    LLEX("ffmpeg\\x64\\ilvideo_x64.dll", 0x2);
    LLEX("ffmpeg\\x64\\ilvideo_x64.dll", 0x20 | 0x40);
    LLEX("ffmpeg\\x64\\ilvideo_x64.dll", 0x1000);
    LLW("C:\\Program Files\\Common Files\\VST3\\Serum2.vst3\\Contents\\x86_64-win\\Serum2.vst3");
    LLW("oggio_x64.dll");
    LLW("dir/kernel32.dll");
    LLW("C:\\Program Files\\Some Plugin\\ws2_32.dll");
    LLW("C:\\Windows\\System32\\a_dll_that_no_roster_has.dll");
    LLA("..\\psapi.dll");
"#;
    let o = run_main_with_prelude("l1-missing", body);
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let got: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(got.len(), 19);
    for (i, g) in got.iter().enumerate() {
        assert_eq!(*g, "0 126", "call {i}");
    }
}

#[test]
fn load1_a_file_of_the_volume_that_is_not_a_module_of_the_build_stops_by_name() {
    // boundary (b): the volume holds a file at the place the search looks, native code that was not lifted does not run, so no handle is invented and no failure is claimed (the file is there): the stop names the path
    let put = r#"
    put_file(u"C:\\Program Files\\Image-Line\\FL Studio 21\\Shared", u"C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\oggio_x64.dll");
    put_file(u"C:\\Program Files\\Image-Line\\FL Studio 21\\ffmpeg\\x64", u"C:\\Program Files\\Image-Line\\FL Studio 21\\ffmpeg\\x64\\ilvideo_x64.dll");
    put_file(u"C:\\Windows\\System32", u"C:\\Windows\\System32\\sysmod.dll");
    put_file(u"C:\\Windows", u"C:\\Windows\\winmod.dll");
    put_file(u"C:\\Program Files\\Image-Line\\FL Studio 21", u"C:\\Program Files\\Image-Line\\FL Studio 21\\appmod.dll");
    put_file(u"C:\\Users\\user\\Downloads", u"C:\\Users\\user\\Downloads\\x.dll");
    put_file(u"C:\\dlldir", u"C:\\dlldir\\userdirmod.dll");
"#;
    let cases: [(&str, &str, &str); 11] = [
        ("full", "LoadLibraryW, A(u\"C:\\\\Program Files\\\\Image-Line\\\\FL Studio 21\\\\Shared\\\\oggio_x64.dll\")", "holds a file at \"C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\oggio_x64.dll\""),
        ("full-slashes", "LoadLibraryW, A(u\"C:/Program Files/Image-Line/FL Studio 21/Shared/oggio_x64.dll\")", "holds a file at"),
        ("full-a", "LoadLibraryA, A(\"C:\\\\Program Files\\\\Image-Line\\\\FL Studio 21\\\\Shared\\\\oggio_x64.dll\")", "holds a file at"),
        ("relative-appdir", "LoadLibraryW, A(u\"ffmpeg\\\\x64\\\\ilvideo_x64.dll\")", "holds a file at \"C:\\Program Files\\Image-Line\\FL Studio 21\\ffmpeg\\x64\\ilvideo_x64.dll\""),
        ("bare-appdir", "LoadLibraryW, A(u\"appmod.dll\")", "holds a file at \"C:\\Program Files\\Image-Line\\FL Studio 21\\appmod.dll\""),
        ("bare-appdir-noext", "LoadLibraryW, A(u\"appmod\")", "holds a file at"),
        ("bare-system", "LoadLibraryW, A(u\"sysmod.dll\")", "holds a file at \"C:\\Windows\\System32\\sysmod.dll\""),
        ("bare-windows", "LoadLibraryW, A(u\"winmod.dll\")", "holds a file at \"C:\\Windows\\winmod.dll\""),
        ("datafile", "LoadLibraryExW, A(u\"appmod.dll\")", "holds a file at"),
        ("relative-cwd", "LoadLibraryW, A(u\"x.dll\")", "holds a file at"),
        ("setdlldirectory", "LoadLibraryW, A(u\"userdirmod.dll\")", "holds a file at \"C:\\dlldir\\userdirmod.dll\""),
    ];
    for (label, call, must_say) in cases {
        let mut body = put.to_string();
        if label == "relative-cwd" {
            body += "    callv(fle_imp_SetCurrentDirectoryW, A(u\"C:\\\\Users\\\\user\\\\Downloads\"), 0, 0);\n";
        }
        if label == "setdlldirectory" {
            body += "    callv(fle_imp_SetDllDirectoryW, A(u\"C:\\\\dlldir\"), 0, 0);\n";
        }
        let (func, arg) = call.split_once(", ").unwrap();
        let flags = if label == "datafile" { "0x2" } else { "0" };
        body += &format!("    callv(fle_imp_{func}, {arg}, 0, {flags});\n    printf(\"returned %llx\\n\", U(R[0]));\n");
        let o = run_main_with_prelude(&format!("l1-file-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(must_say), "{label}: message lacks {must_say:?}: {}", o.stderr);
        assert!(o.stderr.contains("not a module of this build"), "{label}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

#[test]
fn load1_a_path_whose_name_is_a_virtual_module_and_forms_the_volume_does_not_model_stop_by_name() {
    let cases: [(&str, &str, &str); 10] = [
        ("virtual-full", "callv(fle_imp_LoadLibraryW, A(u\"c:\\\\windows\\\\kernel32.dll\"), 0, 0);", "a path to"),
        ("virtual-slash", "callv(fle_imp_LoadLibraryW, A(u\"C:/Windows/System32/kernel32.dll\"), 0, 0);", "\"kernel32.dll\""),
        ("roster-system32", "callv(fle_imp_LoadLibraryW, A(u\"C:\\\\Windows\\\\System32\\\\ws2_32.dll\"), 0, 0);", "a path to \"ws2_32.dll\", a DLL that a Windows installation ships"),
        ("roster-syswow64", "callv(fle_imp_LoadLibraryW, A(u\"c:\\\\WINDOWS\\\\syswow64\\\\psapi.dll\"), 0, 0);", "a path to \"psapi.dll\""),
        ("roster-windows", "callv(fle_imp_LoadLibraryExW, A(u\"C:\\\\Windows\\\\urlmon\"), 0, 0x2);", "a path to \"urlmon.dll\""),
        ("virtual-noext", "callv(fle_imp_LoadLibraryW, A(u\"C:\\\\Windows\\\\System32\\\\user32\"), 0, 0);", "\"user32.dll\""),
        ("drive-relative", "callv(fle_imp_LoadLibraryW, A(u\"c:kernel32.dll\"), 0, 0);", "drive-relative"),
        ("unc", "callv(fle_imp_LoadLibraryW, A(u\"\\\\\\\\srv\\\\share\\\\x.dll\"), 0, 0);", "UNC or device"),
        ("colon-inside", "callv(fle_imp_LoadLibraryW, A(u\"C:\\\\a\\\\b:c.dll\"), 0, 0);", "':'"),
        ("a-high-byte", "callv(fle_imp_LoadLibraryA, A(\"dir\\\\kernel3\\xe9.dll\"), 0, 0);", "above 0x7F"),
    ];
    for (label, stmt, must_say) in cases {
        let o = run_main_with_prelude(&format!("l1-stop-{label}"), &format!("    {stmt}\n    printf(\"returned %llx\\n\", U(R[0]));"));
        assert!(!o.ok, "{label}: expected an abort, stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains(must_say), "{label}: message lacks {must_say:?}: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

#[test]
fn load1_the_environment_variables_of_the_exploration_are_gone_from_the_loader() {
    // decision 0233 removes the getenv calls of fle_load.c: FLE_LOADLIB_MISSING (decision 0178, 0206), FLE_LOADLIB_TRACE (decision 0191) and the soft stop of decision 0209 at the unknown-module site. A set variable changes nothing.
    let t = std::fs::read_to_string(src_dir().join("fle_load.c")).unwrap();
    assert!(!t.contains("getenv"), "fle_load.c still reads the environment");
    assert!(!t.contains("FLE_SOFT_OR_STOP"), "fle_load.c still has a soft-stop site");
    let o = run_main_with_prelude("l1-env", "    setenv(\"FLE_LOADLIB_MISSING\", \"1\", 1); setenv(\"FLE_SOFT_STOP\", \"1\", 1);\n    LLW(\"C:\\\\x\\\\y.dll\"); LLW(\"nothere.dll\");\n");
    assert!(o.ok, "{}", o.stderr);
    assert_eq!(o.stdout.lines().collect::<Vec<_>>(), ["0 126", "0 126"]);
}

#[test]
fn load1_get_proc_address_of_a_null_module_is_a_lookup_in_the_process_image() {
    // Decision 0233 (load1, PROVISIONAL; old: the run stopped with "GetProcAddress with hModule 0 that is not a virtual module handle is not supported"). The GetProcAddress page has no sentence for a NULL hModule ("A handle to the DLL
    // module ... returned by LoadLibrary / LoadLibraryEx / LoadPackagedLibrary / GetModuleHandle"; "If the function fails, the return value is NULL"). Wine 10.0 reads NULL as the process's own image (UNOFFICIAL: kernelbase/loader.c
    // get_proc_address line 72) and a name that the image does not export fails with STATUS_PROCEDURE_NOT_FOUND (ntdll/loader.c LdrGetProcedureAddress), which ntdll/error.h line 487 maps to ERROR_PROC_NOT_FOUND = 127. The image of this
    // environment is the lifted FLEngine_x64.dll (GetModuleHandleW(NULL), decision 0015), whose export directory holds three names (CreateFruityInstance, __dbk_fcall_wrapper, dbkFCallWrapperAddr; read from the PE file by this
    // test's author, tables/flengine-exports.txt of the decision). The same lookup is made for the handle 0x400000. The engine's own exports and ordinals still stop (the runtime does not hand out the engine's own addresses).
    let body = r#"
    SETLE(5); callv(fle_imp_GetProcAddress, 0, A("ElAllocator_SetFunctions"), 0); printf("%llx %u\n", U(R[0]), (unsigned)LE());
    SETLE(5); callv(fle_imp_GetProcAddress, 0x400000, A("ElAllocator_SetFunctions"), 0); printf("%llx %u\n", U(R[0]), (unsigned)LE());
    SETLE(5); callv(fle_imp_GetProcAddress, 0, A("CreateFruityInstanc"), 0); printf("%llx %u\n", U(R[0]), (unsigned)LE());
    SETLE(5); callv(fle_imp_GetProcAddress, 0, A("a_name_that_is_longer_than_thirty_one_characters_for_sure"), 0); printf("%llx %u\n", U(R[0]), (unsigned)LE());
    LLW("Elastique_x64.dll");
"#;
    let got = lines("l1-gpa-null", body);
    assert_eq!(got, ["0 127", "0 127", "0 127", "0 127", "0 126"]);
    assert_aborts(
        "l1-gpa",
        &[
            ("garbage-handle", "callv(fle_imp_GetProcAddress, 0x1234, A(\"x\"), 0);", "not a virtual module handle"),
            ("own-export", "callv(fle_imp_GetProcAddress, 0, A(\"CreateFruityInstance\"), 0);", "one of its own exports or by ordinal"),
            ("own-export-exe", "callv(fle_imp_GetProcAddress, 0x400000, A(\"dbkFCallWrapperAddr\"), 0);", "one of its own exports or by ordinal"),
            ("null-ordinal", "callv(fle_imp_GetProcAddress, 0, 7, 0);", "by ordinal"),
        ],
    );
}


// ---------------------------------------------------------------------------------------------
// Decision 0238 (load2, PROVISIONAL): API set contract names
// ---------------------------------------------------------------------------------------------
//
// Expected values are written from the Microsoft Learn pages "Windows API sets" and "API set loader operation" (saved in docs/investigations/load2/tables/contracts/): "The loader recognizes a name that begins with api- or ext- and routes it to the
// API set runtime ... the .dll suffix isn't part of the contract name being resolved"; "The operation succeeds when the schema on the current device maps that contract to a usable host"; "If the contract isn't mapped on the current device, a direct LoadLibrary
// fails". The pages give no table: the contract -> host rows are Wine 10.0's apisetschema.spec (UNOFFICIAL) and the sub-version rule (l1-1-0 and l1-1-7 are one entry) is Wine 10.0's ntdll/loader.c get_apiset_entry (UNOFFICIAL). Hosts: advapi32 N = 1,
// kernel32 N = 9, ole32 N = 13, user32 N = 19, kernelbase N = 40 and combase N = 41 (decision 0238). 126 is the code of this file for a module that is not there (decision 0015).

const N_OLE32: u64 = 13;
const N_KERNELBASE: u64 = 40;
const N_COMBASE: u64 = 41;

#[test]
fn load2_a_contract_name_gives_the_handle_of_its_host_and_one_more_reference_on_it() {
    let rows: [(&str, u64); 13] = [
        ("api-ms-win-core-string-l1-1-0.dll", N_KERNELBASE),
        ("API-MS-WIN-CORE-STRING-L1-1-0.DLL", N_KERNELBASE),
        ("api-ms-win-core-string-l1-1-0", N_KERNELBASE),       // the .dll is not part of the contract name
        ("api-ms-win-core-string-l1-1-7.dll", N_KERNELBASE),   // the sub-version (the last number) is not part of the lookup
        ("api-ms-win-core-string-l1-1-99", N_KERNELBASE),
        ("api-ms-win-core-string-l2-1-0.dll", N_KERNELBASE),   // l2-1 is another entry of the schema, with the same host
        ("API-MS-WIN-CORE-HEAP-L2-1-0.DLL", N_KERNELBASE),
        ("ext-ms-win-kernel32-package-current-l1-1-0.dll", N_KERNELBASE),
        ("ext-ms-win-com-ole32-l1-1-5", N_OLE32),
        ("api-ms-win-core-ums-l1-1-0.dll", N_KERNEL32),
        ("api-ms-win-security-systemfunctions-l1-1-0.dll", N_ADVAPI32),
        ("ext-ms-win-ntuser-dialogbox-l1-1-0.dll", N_USER32),
        ("api-ms-win-rtcore-ntuser-window-l1-1-0.dll", N_USER32),
    ];
    let mut body = String::from("    printf(\"init %u\\n\", fle_load_refcount(39));\n");
    for (n, _) in rows {
        body += &format!("    LLW(\"{n}\");\n");
    }
    body += "    LLA(\"api-ms-win-core-string-l1-1-0.dll\");\n    LLEX(\"api-ms-win-core-string-l1-1-0.dll\", 0x800u);\n    LLEX(\"api-ms-win-core-string-l1-1-0.dll\", 0x2u);\n";
    body += "    printf(\"count %u\\n\", fle_load_refcount(39));\n";
    body += "    SHOW(fle_imp_FreeLibrary, fle_module_handle(39), 0, 0);\n    printf(\"count %u\\n\", fle_load_refcount(39));\n";
    let got = lines("load2-host", &body);
    assert_eq!(got[0], "init 1");
    for (i, (n, ix)) in rows.iter().enumerate() {
        assert_eq!(got[1 + i], format!("{} 5", h(*ix)), "{n}");
    }
    let k = 1 + rows.len();
    for j in 0..3 {
        assert_eq!(got[k + j], format!("{} 5", h(N_KERNELBASE)), "A / Ex form {j}");
    }
    // kernelbase had the process-start count 1; 8 of the 13 names and the 3 A / Ex loads above are on it: 1 + 8 + 3
    assert_eq!(got[k + 3], "count 12");
    assert_eq!(got[k + 4], "1 5");
    assert_eq!(got[k + 5], "count 11");
}

#[test]
fn load2_the_minor_version_is_part_of_the_lookup_and_the_virtual_contract_modules_of_the_table_keep_their_handles() {
    // l1-2 of core-string is not in the schema (only l1-1 and l2-1): the lookup key is the name without its last component, so the minor version matters. A name the schema lacks is searched like any other bare name (nothing on the volume: NULL, 126).
    // The five contract-named virtual modules of the table (N = 2 and 27..30, decisions 0015 and 0081) are found by name before the schema: their own handles, as before.
    let body = r#"
    LLW("api-ms-win-core-string-l1-2-0.dll");
    LLW("api-ms-win-core-string-l1-0-0.dll");
    LLW("api-ms-win-core-string.dll");
    LLW("api-ms-win-core-xyzzy-l1-1-0.dll");
    LLW("ext-ms-win-xyzzy-l1-1-0");
    LLW("api-.dll");
    LLW("api-win-core-samplefeature.dll");
    LLW("api-ms-win-crt-string-l1-1-0.dll");
    LLW("api-ms-win-core-synch-l1-2-0.dll");
    LLW("api-ms-win-core-fibers-l1-1-1.dll");
    LLW("api-ms-win-core-localization-l1-2-1.dll");
    LLW("api-ms-win-appmodel-runtime-l1-1-2.dll");
    LLW("api-ms-win-core-synch-l1-2-1.dll");
"#;
    let got = lines("load2-minor", body);
    for (i, g) in got[..7].iter().enumerate() {
        assert_eq!(g, "0 126", "name {i}");
    }
    for (i, n) in [N_CRT_STRING, 27, 28, 29, 30].iter().enumerate() {
        assert_eq!(got[7 + i], format!("{} 5", h(*n)), "virtual contract module {i}");
    }
    // a name of the same contract that the table does not list is the schema's: kernelbase
    assert_eq!(got[12], format!("{} 5", h(N_KERNELBASE)));
}

#[test]
fn load2_a_contract_without_a_host_fails_and_a_contract_whose_host_is_not_a_virtual_module_stops_by_name() {
    // the schema has api-ms-win-deprecated-apis-legacy-l1-1 and maps it to no host: "not mapped", so the load fails (NULL, 126)
    let got = lines("load2-nohost", "    LLW(\"api-ms-win-deprecated-apis-legacy-l1-1-0.dll\");\n    LLW(\"api-ms-win-deprecated-apis-advapi-l1-1-0\");\n    LLW(\"API-MS-WIN-DEPRECATED-APIS-LEGACY-L1-2-0.DLL\");\n");
    assert_eq!(got, ["0 126", "0 126", "0 126"]);
    // hosts that have no virtual module: ucrtbase (api-ms-win-crt-*), wintypes (core-winrt-robuffer), sechost: no handle and no failure is invented
    assert_aborts(
        "load2-nohostmodule",
        &[
            ("crt-runtime", "callv(fle_imp_LoadLibraryW, A(u\"api-ms-win-crt-runtime-l1-1-0.dll\"), 0, 0);", "mapped to the host \"ucrtbase.dll\""),
            ("robuffer", "callv(fle_imp_LoadLibraryExW, A(u\"api-ms-win-core-winrt-robuffer-l1-1-0.dll\"), 0, 0x800);", "mapped to the host \"wintypes.dll\""),
            ("crt-a", "callv(fle_imp_LoadLibraryA, A(\"API-MS-WIN-CRT-STDIO-L1-1-0\"), 0, 0);", "mapped to the host \"ucrtbase.dll\""),
        ],
    );
    let o = run_main("load2-msg", "    callv(fle_imp_LoadLibraryW, A(u\"api-ms-win-core-winrt-robuffer-l1-1-0.dll\"), 0, 0);", &[], &[]);
    assert!(o.stderr.contains("fle: LoadLibraryW: the API set contract \"api-ms-win-core-winrt-robuffer-l1-1-0.dll\""), "{}", o.stderr);
}

#[test]
fn load2_a_contract_that_the_schema_lacks_is_searched_as_a_file_like_any_other_bare_name() {
    // Wine 10.0 (UNOFFICIAL) find_apiset_dll gives STATUS_APISET_NOT_PRESENT for a name that the schema lacks and the file search goes on; the volume holds nothing (fail) or a file (a module this build cannot run: stop by name)
    let put = r#"
    put_file(u"C:\\Windows\\System32", u"C:\\Windows\\System32\\api-ms-win-core-xyzzy-l1-1-0.dll");
"#;
    let o = run_main_with_prelude("load2-other-file", &format!("{put}    LLW(\"api-ms-win-core-other-l1-1-0.dll\");\n"));
    assert!(o.ok && o.stdout == "0 126\n", "a name that is not on the volume fails: {} {}", o.stdout, o.stderr);
    let body = format!("{put}    callv(fle_imp_LoadLibraryW, A(u\"api-ms-win-core-xyzzy-l1-1-0.dll\"), 0, 0);\n    printf(\"returned %llx\\n\", U(R[0]));\n");
    let o = run_main_with_prelude("load2-xyzzy-file", &body);
    assert!(!o.ok && !o.stdout.contains("returned"), "{}", o.stdout);
    assert!(o.stderr.contains("holds a file at \"C:\\Windows\\System32\\api-ms-win-core-xyzzy-l1-1-0.dll\"") && o.stderr.contains("not a module of this build"), "{}", o.stderr);
    assert_eq!(o.signal, Some(6));
}

#[test]
fn load2_get_module_handle_of_a_contract_gives_the_host_when_the_host_is_a_loaded_module() {
    // GetModuleHandle pages name no case for a contract; Wine 10.0 (UNOFFICIAL) LdrGetDllHandleEx finds the loaded module of the host's path. A host that this environment has not loaded, a contract without a host and an unknown contract: NULL, 126.
    let names: [(&str, &str, u64); 10] = [
        ("W", "api-ms-win-core-string-l1-1-0.dll", N_KERNELBASE),
        ("W", "API-MS-WIN-CORE-STRING-L1-1-0", N_KERNELBASE),
        ("A", "api-ms-win-core-string-l1-1-5.dll", N_KERNELBASE),
        ("W", "ext-ms-win-com-ole32-l1-1-5.dll", N_OLE32),
        ("W", "api-ms-win-crt-runtime-l1-1-0.dll", 0),
        ("W", "api-ms-win-core-winrt-l1-1-0.dll", N_COMBASE),
        ("W", "api-ms-win-core-winrt-robuffer-l1-1-0.dll", 0),
        ("W", "api-ms-win-deprecated-apis-legacy-l1-1-0.dll", 0),
        ("W", "api-ms-win-core-xyzzy-l1-1-0.dll", 0),
        ("W", "api-ms-win-core-synch-l1-2-0.dll", 27),   // the contract-named virtual module of the table: its own handle
    ];
    let mut body = String::new();
    for (f, n, _) in names {
        let (func, arg) = if f == "W" { ("GetModuleHandleW", format!("A(u\"{n}\")")) } else { ("GetModuleHandleA", format!("A(\"{n}\")")) };
        body += &format!("    SHOW(fle_imp_{func}, {arg}, 0, 0);\n");
    }
    let got = lines("load2-gmh", &body);
    for (i, (_, n, ix)) in names.iter().enumerate() {
        if *ix == 0 {
            assert_eq!(got[i], "0 126", "{n}");
        } else {
            assert_eq!(got[i], format!("{} 5", h(*ix)), "{n}");
        }
    }
}

#[test]
fn load2_get_proc_address_on_a_host_handle_asks_the_response_table_under_the_host_name() {
    // the handle of a contract is the host's: kernelbase.dll has no response-table row (none is linked here), so every name is missing (NULL, 127), as for the other virtual modules without rows
    let body = r#"
    uint64_t k = callv(fle_imp_LoadLibraryW, A(u"api-ms-win-core-string-l1-1-0.dll"), 0, 0), f;
    SETLE(5); f = callv(fle_imp_GetProcAddress, k, A("CompareStringEx"), 0);
    printf("gpa %llx %u\n", U(f), (unsigned)LE());
    printf("same %d\n", k == fle_module_handle(39));
"#;
    let got = lines("load2-gpa", body);
    assert_eq!(got, ["gpa 0 127", "same 1"]);
}

#[test]
fn load2_the_schema_is_sorted_unique_lower_case_and_has_the_hosts_of_the_contracts_the_crt_asks_for() {
    let t = std::fs::read_to_string(runtime_dir().join("fle_apiset_schema.inc")).unwrap();
    let rows: Vec<(&str, &str)> = t
        .lines()
        .filter(|l| l.starts_with("{ \""))
        .map(|l| {
            let p: Vec<&str> = l.split('"').collect();
            (p[1], p[3])
        })
        .collect();
    assert_eq!(rows.len(), 511, "Wine 10.0 apisetschema.spec has 511 rows");
    for w in rows.windows(2) {
        assert!(w[0].0 < w[1].0, "not strictly ascending: {} / {}", w[0].0, w[1].0);
    }
    for (k, _) in &rows {
        assert!(k.starts_with("api-") || k.starts_with("ext-"), "{k}");
        assert!(k.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'), "{k}");
        assert!(!k.ends_with('-'), "{k}");
    }
    assert_eq!(rows.iter().filter(|(_, h)| h.is_empty()).count(), 3, "three contracts are in the schema without a host");
    // the contracts that the static UCRT asks for with LoadLibraryExW (winapi_thunks.cpp of the CRT), by the lookup key, and their hosts in the schema
    let want: [(&str, &str); 6] = [
        ("api-ms-win-core-string-l1-1", "kernelbase.dll"),
        ("api-ms-win-core-datetime-l1-1", "kernelbase.dll"),
        ("api-ms-win-core-file-l1-2", "kernelbase.dll"),
        ("api-ms-win-core-processthreads-l1-1", "kernel32.dll"),
        ("api-ms-win-rtcore-ntuser-window-l1-1", "user32.dll"),
        ("api-ms-win-core-winrt-l1-1", "combase.dll"),
    ];
    for (k, host) in want {
        assert_eq!(rows.iter().find(|r| r.0 == k).map(|r| r.1), Some(host), "{k}");
    }
}

#[test]
fn load2_the_resolver_answers_the_four_cases_by_the_lookup_key() {
    // fle_apiset_resolve(name, &host): the name is the lower-case spelling with its extension. 0 not a contract (no api- / ext- prefix, or four characters or fewer), 1 routed to the API set runtime but the schema lacks it,
    // 2 in the schema and mapped to no host, 3 mapped to the host in *host.
    let body = r#"
    static const char *const names[] = {
        "", "api", "api-", "ext-", "api-x", "xapi-ms-win-core-string-l1-1-0.dll", "ap-ms-win-core-string-l1-1-0.dll",
        "api-ms-win-core-string-l1-1-0.dll", "api-ms-win-core-string-l1-1-0", "api-ms-win-core-string-l1-1-12.dll", "api-ms-win-core-string-l1-1.dll", "api-ms-win-core-string-l1-2-0.dll", "api-ms-win-core-string-l1.dll",
        "api-ms-win-deprecated-apis-legacy-l1-1-0.dll", "ext-ms-win-com-ole32-l1-1-5.dll", "ext-ms-win-ntuser-dialogbox-l1-1-0.dll", "api-ms-win-core-string-l1-1-0.dll.dll", "api-ms-win-core-string-l1-1-0.x-y",
    };
    unsigned i;
    for (i = 0; i < sizeof names / sizeof names[0]; i++) {
        const char *host = (const char *)1;
        int r = fle_apiset_resolve(names[i], &host);
        printf("%d %s\n", r, r == 3 ? host : (host == NULL ? "(null)" : "(unset)"));
    }
"#;
    let got = lines("load2-resolver", body);
    let want = [
        "0 (null)", "0 (null)", "0 (null)", "0 (null)", "1 (null)", "0 (null)", "0 (null)",
        "3 kernelbase.dll", "3 kernelbase.dll", "3 kernelbase.dll", "1 (null)", "1 (null)", "1 (null)",
        "2 (null)", "3 ole32.dll", "3 user32.dll", "3 kernelbase.dll", "3 kernelbase.dll",
    ];
    assert_eq!(got, want);
}
