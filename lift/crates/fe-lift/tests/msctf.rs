//! Tests of decision 0055 (docs/decisions/0055-msctf-loadlibrary.md): msctf.dll is a virtual module (N = 26), so
//! kernel32!LoadLibraryW("Msctf.dll") and GetModuleHandleW give its handle, and msctf!SetInputScope is the one export that
//! the guest resolves with GetProcAddress. Calling SetInputScope stops the run by name: no HRESULT is made up.
//!
//! 1. Generator: the response table has the msctf.dll row, and only under that DLL name.
//! 2. Runtime, through a driver linked with every runtime source and the generated C of a synthetic PE (so the pseudo-VA of
//!    GetProcAddress is called through fle_dispatch, as the guest does): handles, last error, GetProcAddress, FreeLibrary,
//!    the stop of SetInputScope, and the names that stay unanswered.
//! 3. The bytes of the real FLEngine_x64.dll that the decision rests on (skipped when the DLL is absent).
//!
//! Expected values are written here from decision 0055, decision 0015 (handle = 0x7FF800000000 + 0x10000 * N) and the Learn
//! pages named in the decision (LoadLibraryW, GetProcAddress, SetInputScope, SetErrorMode, System Error Codes), not read
//! back from the C code. The host is Linux: nothing is compared with a real Windows.
//!
//! FLE_MSCTF_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs of the run record.

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
    match std::env::var_os("FLE_MSCTF_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-msc/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

// ---------------------------------------------------------------------------------------------
// A synthetic PE that imports (dll, name) pairs, one thunk each (the same shape as tests/load.rs)
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

const GUEST_IMPORTS: [(&str, &str); 3] = [("kernel32.dll", "GetProcAddress"), ("kernel32.dll", "LoadLibraryW"), ("msctf.dll", "SetInputScope")];

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

#[test]
fn the_response_table_has_the_msctf_row_under_msctf_dll_only() {
    let (st, c) = gen(&GUEST_IMPORTS);
    assert_eq!(st.handled["jmp_imp:SetInputScope"], 1);
    assert!(c.contains("fle_imp_SetInputScope(); return;"));
    assert!(c.contains("\"msctf.dll\", \"SetInputScope\", UINT64_C("));
    // The same API name under another DLL is another API (it has no row, and its thunk stays unsupported).
    let (st, c) = gen(&[("kernel32.dll", "GetProcAddress"), ("user32.dll", "SetInputScope"), ("msctf.dll", "SetInputScopes")]);
    assert!(!st.handled.contains_key("jmp_imp:SetInputScope"));
    assert!(!c.contains("\"user32.dll\", \"SetInputScope\""));
    assert!(!c.contains("\"msctf.dll\", \"SetInputScopes\""));
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 2);
}

// ---------------------------------------------------------------------------------------------
// 2. Runtime
// ---------------------------------------------------------------------------------------------

/// The handle of module number N (decision 0015: 1-based position, 0x7FF800000000 + 0x10000 * N). msctf.dll is the 26th.
fn h(n: u64) -> String {
    format!("{:x}", 0x7FF8_0000_0000u64 + 0x10000 * n)
}
const N_USER32: u64 = 19;
const N_KERNEL32: u64 = 9;
const N_NTDLL: u64 = 25;
const N_MSCTF: u64 = 26;

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

static uint64_t stk[16];
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))

static void prep(uint64_t a, uint64_t b)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[4] = A(stk);
    R[1] = a; R[2] = b;
    R[0] = 0xBADBADBADull;
    fle_set_last_error(5);
}
static void out(void)
{
    uint64_t r = R[0];
    fle_imp_GetLastError();
    printf("%llx %llu\n", U(r), U(R[0]));
}
static void llw(const uint16_t *n) { prep(A(n), 0); fle_imp_LoadLibraryW(); out(); }
static void gmh(const uint16_t *n) { prep(A(n), 0); fle_imp_GetModuleHandleW(); out(); }
static uint64_t gpa(uint64_t m, const char *n)
{
    uint64_t r;
    prep(m, A(n)); fle_imp_GetProcAddress();
    r = R[0];
    fle_imp_GetLastError();
    printf("%s %llu\n", r ? "found" : "missing", U(R[0]));
    return r;
}
static void free_lib(uint64_t m) { prep(m, 0); fle_imp_FreeLibrary(); out(); }
#define LLW(s) llw((const uint16_t *)u##s)
#define GMH(s) gmh((const uint16_t *)u##s)
#define M(n) (0x7FF800000000ull + 0x10000ull * (n))

int main(int argc, char **argv)
{
    const char *s = argc > 1 ? argv[1] : "";
    if (!strcmp(s, "load")) {
        LLW("msctf.dll"); LLW("Msctf.dll"); LLW("MSCTF.DLL"); LLW("msctf"); LLW("MsCtF");
        GMH("Msctf.dll"); GMH("msctf");
    } else if (!strcmp(s, "load-after-others")) {
        LLW("kernel32.dll"); LLW("msctf.dll"); LLW("ntdll"); LLW("user32");
    } else if (!strcmp(s, "gpa")) {
        uint64_t m = M(26);
        uint64_t va = gpa(m, "SetInputScope");
        printf("va %s\n", va >= 0x7FF900000000ull ? "pseudo" : "bad");
        gpa(m, "setinputscope"); gpa(m, "SETINPUTSCOPE"); gpa(m, "SetInputScopes"); gpa(m, "SetInputScop");
        gpa(m, "SetInputScopeX"); gpa(m, "GetLastError"); gpa(m, "TF_CreateThreadMgr"); gpa(m, "");
        gpa(M(19), "SetInputScope"); gpa(M(9), "SetInputScope"); gpa(M(25), "SetInputScope");
    } else if (!strcmp(s, "dispatch")) {
        uint64_t va = gpa(M(26), "SetInputScope");
        printf("before\n"); fflush(stdout);
        prep(0x1234, 0); R[1] = 0x1234; R[2] = 0;
        fle_dispatch(va);
        printf("returned %llx\n", U(R[0]));
    } else if (!strcmp(s, "free")) {
        free_lib(M(26));          /* the process loaded it at start: count 1, so this must stop */
    } else if (!strcmp(s, "free-balanced")) {
        LLW("Msctf.dll"); free_lib(M(26)); printf("count %u\n", fle_load_refcount(25));
        LLW("msctf"); LLW("msctf.dll"); printf("count %u\n", fle_load_refcount(25));
        free_lib(M(26)); free_lib(M(26)); printf("count %u\n", fle_load_refcount(25));
    } else if (!strcmp(s, "table")) {
        printf("count %llu\n", U(fle_module_count()));
        printf("name25 %s\n", fle_module_name(25));
        printf("name24 %s\n", fle_module_name(24));
        printf("handle25 %llx\n", U(fle_module_handle(25)));
        printf("past %p\n", (const void *)fle_module_name(41)); /* decision 0238: 41 modules (kernelbase.dll and combase.dll appended); 40 modules (kernelbase.dll appended); decision 0122: 39 modules (dwmapi.dll appended); decision 0119: 38; old: 36, the end after rpcrt4.dll */
    } else if (!strcmp(s, "lack")) {
        /* the seven names that decision 0029 defines as lacking (nine until decision 0099 made shcore.dll a module, eight until decision 0118 made rpcrt4.dll one) keep failing with 126 */
        LLW("imm32.dll"); LLW("msimg32"); LLW("propsys");
        LLW("windowscodecs"); /* decision 0122: dwmapi.dll is a module now (N = 39; old: first here, 5 names; new: 4). Decision 0119: uxtheme.dll (N = 38) and wtsapi32.dll (N = 37) are modules (old: both here, 7 names) */
    } else if (!strcmp(s, "avrt-powrprof")) {
        /* decision 0088: N = 31 and 32. The table linked here is the generator's real response table: it has no row under either dll. */
        static const char *const av[] = { "AvSetMmMaxThreadCharacteristicsW", "AvSetMmThreadCharacteristicsW", "AvSetMmThreadPriority",
            "AvQuerySystemResponsiveness", "AvRevertMmThreadCharacteristics", "AvRtCreateThreadOrderingGroup", "AvRtCreateThreadOrderingGroupEx",
            "AvRtDeleteThreadOrderingGroup", "AvRtJoinThreadOrderingGroup", "AvRtLeaveThreadOrderingGroup", "AvRtWaitOnThreadOrderingGroup" };
        static const char *const pw[] = { "PowerGetActiveScheme", "PowerSetActiveScheme" };
        unsigned i;
        LLW("avrt"); LLW("PowrProf.dll"); GMH("avrt"); GMH("powrprof");
        for (i = 0; i < sizeof av / sizeof av[0]; i++) gpa(M(31), av[i]);
        for (i = 0; i < sizeof pw / sizeof pw[0]; i++) gpa(M(32), pw[i]);
        gpa(M(31), "GetLastError"); gpa(M(32), "GetLastError"); gpa(M(31), "SetInputScope"); gpa(M(32), "SetInputScope");
        gpa(M(31), "PowerGetActiveScheme"); gpa(M(32), "AvSetMmThreadCharacteristicsW");
        LLW("dwmapi"); LLW("uxtheme"); /* decision 0122: dwmapi is virtual module N = 39 (old: 126) */
    } else if (!strcmp(s, "unknown-1")) { LLW("msctf2.dll");
    } else if (!strcmp(s, "unknown-2")) { LLW("msctf.dll.dll");
    } else if (!strcmp(s, "unknown-3")) { LLW("mscti.dll");
    } else if (!strcmp(s, "unknown-4")) { LLW("msctfime.ime");
    } else if (!strcmp(s, "unknown-5")) { LLW("msctfp.dll");
    } else if (!strcmp(s, "unknown-6")) { LLW("msctf.");
    } else if (!strcmp(s, "handle-37")) { gpa(M(42), "SetInputScope"); /* decision 0238: a handle past the 41 modules (M(41) with 40: kernelbase.dll only); decision 0238 earlier: a handle past the 40 modules (M(40) with 39 after decision 0122); decision 0122: a handle past the 39 modules (decision 0119: M(39) with 38; old: M(37) with 36) */
    } else if (!strcmp(s, "handle-25-plus-1")) { gpa(M(25) + 1, "SetInputScope");
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
        let tag = etxt_common::per_process("msctf-driver");
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

#[test]
fn msctf_is_the_26th_virtual_module() {
    // Decision 0055: appended after ntdll.dll (decision 0015: append only, never reorder).
    let got = lines("table");
    assert_eq!(got[0], "count 41"); // decision 0238: kernelbase.dll and combase.dll appended (40 with kernelbase.dll only) (decision 0122: 39, dwmapi.dll appended) (decision 0119: 38, wtsapi32.dll and uxtheme.dll appended; old: 36)
    assert_eq!(got[1], "name25 msctf.dll");
    assert_eq!(got[2], "name24 ntdll.dll");
    assert_eq!(got[3], format!("handle25 {}", h(N_MSCTF)));
    assert!(got[4].starts_with("past ") && (got[4].contains("nil") || got[4].ends_with(" 0x0") || got[4].ends_with(" 0")), "{}", got[4]);
}

#[test]
fn load_library_w_gives_the_msctf_handle_and_leaves_the_last_error_alone() {
    // LoadLibraryW page: success returns the module handle; the extension may be omitted (".DLL" is appended) and the name is
    // compared without case (decision 0015 rules). The page does not say the last error is cleared: it stays 5.
    let got = lines("load");
    let m = h(N_MSCTF);
    let want: Vec<String> = (0..7).map(|_| format!("{m} 5")).collect();
    assert_eq!(got, want);
}

#[test]
fn the_other_handles_are_unchanged_by_the_new_module() {
    let got = lines("load-after-others");
    assert_eq!(got, [format!("{} 5", h(N_KERNEL32)), format!("{} 5", h(N_MSCTF)), format!("{} 5", h(N_NTDLL)), format!("{} 5", h(N_USER32))]);
}

#[test]
fn get_proc_address_finds_set_input_scope_in_msctf_only() {
    // GetProcAddress page: the name is compared with exact spelling and case; failure is NULL. Decision 0015: a row that the
    // table does not have is NULL and 127 (ERROR_PROC_NOT_FOUND, System Error Codes page).
    let got = lines("gpa");
    let mut want: Vec<&str> = vec!["found 5", "va pseudo"];
    want.extend(["missing 127"; 8]); // setinputscope, SETINPUTSCOPE, SetInputScopes, SetInputScop, SetInputScopeX, GetLastError, TF_CreateThreadMgr, ""
    want.extend(["missing 127"; 3]); // the same name in user32, kernel32 and ntdll
    assert_eq!(got, want);
}

#[test]
fn calling_set_input_scope_stops_by_name_through_fle_dispatch() {
    // Learn SetInputScope: S_OK on success, E_INVALIDARG for a window of another thread. Both depend on a window; this runtime
    // has no window table, so the run stops and says which call it was. It must not return.
    let o = aborts("dispatch", "fle: SetInputScope:");
    assert!(o.stdout.contains("found 5") && o.stdout.contains("before"), "{}", o.stdout);
    assert!(!o.stdout.contains("returned"), "{}", o.stdout);
    assert!(o.stderr.contains("no HRESULT is invented"), "{}", o.stderr);
}

#[test]
fn free_library_follows_the_reference_count_of_the_new_module() {
    // FreeLibrary page: a module loaded at process start has count one; LoadLibrary adds one, FreeLibrary subtracts one and
    // returns nonzero; the last error is untouched. The virtual process loads msctf.dll at start (decision 0055).
    let got = lines("free-balanced");
    let m = h(N_MSCTF);
    assert_eq!(
        got,
        [
            format!("{m} 5"), // LoadLibraryW: count 2
            "1 5".into(),     // FreeLibrary: count 1
            "count 1".into(),
            format!("{m} 5"),
            format!("{m} 5"),
            "count 3".into(),
            "1 5".into(),
            "1 5".into(),
            "count 1".into(),
        ]
    );
    aborts("free", "reference count would reach zero");
}

#[test]
fn the_four_lacking_names_still_fail_with_126() {
    // Decision 0122: dwmapi is virtual module N = 39 (old: the first of five lacking names; the test was `the_five_lacking_names_still_fail_with_126`).
    // Decision 0029: dwmapi, imm32, msimg32, propsys and windowscodecs are the names this environment lacks now (shcore was the ninth until decision 0099 made it
    // virtual module N = 35, rpcrt4 the eighth until decision 0118 made it N = 36, wtsapi32 the seventh until decision 0119 made it N = 37 and uxtheme the sixth
    // until decision 0119 made it N = 38).
    // msctf.dll is not one of them (decision 0055), and nothing else joins them.
    let got = lines("lack");
    assert_eq!(got, ["0 126"; 4]);
}

#[test]
fn avrt_and_powrprof_are_modules_with_no_row_in_the_real_response_table() {
    // Decision 0088: N = 31 (avrt.dll) and N = 32 (powrprof.dll) load and have handles; the response table of the generator has no row under
    // either dll, so every GetProcAddress is NULL and ERROR_PROC_NOT_FOUND (127), also for a name that another dll's row has. dwmapi and
    // uxtheme (decisions 0015, 0029) failed with 126 in the same run until decisions 0119 and 0122 made them modules.
    let got = lines("avrt-powrprof");
    let (a, p) = (h(31), h(32));
    let mut want: Vec<String> = vec![format!("{a} 5"), format!("{p} 5"), format!("{a} 5"), format!("{p} 5")];
    want.extend((0..11 + 2 + 6).map(|_| "missing 127".to_string())); // 11 avrt names, 2 PowrProf names, 6 other lookups
    // decision 0122: dwmapi.dll is the module N = 39 (old: "0 126"); uxtheme.dll is the module N = 38 since decision 0119 (old: "0 126" too)
    want.extend([format!("{} 5", h(39)), format!("{} 5", h(38))]);
    assert_eq!(got, want);
}

#[test]
fn names_near_msctf_are_not_answered() {
    for (i, n) in ["msctf2.dll", "msctf.dll.dll", "mscti.dll", "msctfime.ime", "msctfp.dll", "msctf."].iter().enumerate() {
        // decision 0233 (old: a stop "module \"<name>\" is not a virtual module ..."): not msctf.dll and not a DLL of Windows' roster, so the search of the volume finds nothing: NULL and 126 (the pages: "If the function cannot find the module, the function fails")
        let o = run(&format!("unknown-{}", i + 1));
        if *n == "msctfp.dll" {
            // msctfp.dll is a DLL of a real Windows (on the roster of decision 0233, from Wine 10.0): the stop by name stays for it
            assert!(!o.ok && o.stderr.contains("module \"msctfp.dll\" is not a virtual module"), "{n}: {}", o.stderr);
            continue;
        }
        assert!(o.ok, "{n}: {}", o.stderr);
        assert_eq!(o.stdout.trim_end(), "0 126", "{n}");
    }
}

#[test]
fn a_handle_past_the_table_or_misaligned_is_not_a_module() {
    aborts("handle-37", "not a virtual module handle");
    aborts("handle-25-plus-1", "not a virtual module handle");
}

// ---------------------------------------------------------------------------------------------
// 3. The bytes of the real DLL that decision 0055 rests on
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

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect()
}

#[test]
fn the_guest_asks_for_msctf_dll_and_set_input_scope_in_the_forms_the_decision_describes() {
    let Some(dll) = real_dll() else { return };
    // f_5de9e0: lea rcx,[rip+0x23] -> 0x5dea18 = L"Msctf.dll"; mov edx,0x8000 (SEM_NOOPENFILEERRORBOX); call f_44db40.
    assert_eq!(bytes_at(&dll, 0x5de9ee, 7), [0x48, 0x8d, 0x0d, 0x23, 0x00, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x5dea18, 20), utf16("Msctf.dll"));
    assert_eq!(bytes_at(&dll, 0x5de9f5, 5), [0xba, 0x00, 0x80, 0x00, 0x00]);
    assert_eq!(bytes_at(&dll, 0x5de9fa, 1), [0xe8]);
    // f_44db40: SetErrorMode(edx) ; LoadLibraryW(f_414850(rcx)) ; SetErrorMode(old) ; return the LoadLibraryW result.
    assert_eq!(bytes_at(&dll, 0x44db53, 4), [0x8b, 0x4d, 0x58, 0xe8]); // mov ecx,[rbp+0x58]; call SetErrorMode thunk
    assert_eq!(bytes_at(&dll, 0x44db75, 4), [0x8b, 0x4d, 0x3c, 0xe8]); // mov ecx,[rbp+0x3c]; call SetErrorMode thunk
    assert_eq!(bytes_at(&dll, 0x44db7d, 4), [0x48, 0x8b, 0x45, 0x30]); // mov rax,[rbp+0x30] = the LoadLibraryW result
    // f_5dea70: cmp qword [0x13d9ef8],0 ; jbe -> the "return 0" stub 0x5dea60 ; otherwise the "return 1" stub 0x5dea50.
    assert_eq!(bytes_at(&dll, 0x5dea83, 7), [0x48, 0x83, 0x3d, 0x6d, 0xb4, 0xdf, 0x00]);
    assert_eq!(bytes_at(&dll, 0x5dea8b, 2), [0x76, 0x10]);
    assert_eq!(bytes_at(&dll, 0x5dea50, 3), [0xb0, 0x01, 0xc3]);
    assert_eq!(bytes_at(&dll, 0x5dea60, 3), [0x33, 0xc0, 0xc3]);
    // f_5dea30: FreeLibrary(handle) only when the handle is above zero.
    assert_eq!(bytes_at(&dll, 0x5dea3c, 2), [0x76, 0x0c]);
    // f_5e0f30: GetProcAddress by the wide name L"SetInputScope" at 0x5e0f94 through the helper 0x4235f0.
    assert_eq!(bytes_at(&dll, 0x5e0f94, 28), utf16("SetInputScope"));
    assert_eq!(bytes_at(&dll, 0x5e0f63, 1), [0xe8]);
    // f_5e0f30 calls the pointer when it is not NULL, else returns E_FAIL (0x80004005).
    assert_eq!(bytes_at(&dll, 0x5e0f86, 5), [0xb8, 0x05, 0x40, 0x00, 0x80]);
}
