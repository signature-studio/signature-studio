//! KERNEL32 answers for dsp_ippv2_x64.dll (decision 0078, provisional): the runtime functions that the DLL's
//! DLL_PROCESS_ATTACH reaches, and the ones that stop by name.
//!
//! Expected values are written here from the Microsoft Learn pages named in the decision (and the project's decisions 0009, 0014,
//! 0016, 0018, 0047), not read back from the C code. The host is Linux: nothing is compared with a real Windows. (The decision
//! records a run of the same calls under Wine 10.0 as a cross-check only; Wine is not Windows and is not a source.)
//!
//! 1. Generator: every name of the decision has a row in the response table, `jmp [IAT]` and `call [IAT]` forms give the same runtime
//!    function; IsProcessorFeaturePresent and TlsFree have no row (existing tests assert they are absent).
//! 2. Runtime, driver linked with every runtime/fle_*.c: values, last errors, state.
//! 3. Runtime: the names that stop, each with "fle: <api>" on stderr and SIGABRT, and no value returned.
//! 4. The source reads nothing from the host.

mod etxt_common;

use fe_lift::generate;
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
    let d = manifest().join("../../out-k32dsp/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

/// Every runtime/fle_*.c, compiled once per process (the response rows name any `fle_imp_*`).
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

/// The generated unit of a PE with no code is not needed: the driver defines what the runtime expects from the generated C
/// (R, FL, X, the tables) in the weakest way that links.
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
#include "fle_sysinfo.h"
#include "fle_proc.h"
#include "fle_locale.h"
#include "fle_modules.h"
#include "fle_vol.h"
#include "fle_cpuid.h"

/* what the generated C defines and every runtime file may link against */
uint64_t R[16];
fle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };
fle_xmm X[16];
const fle_fn_entry fle_dispatch_table[] = { { 0, 0 } };
const uint64_t fle_dispatch_count = 0;
static uint64_t stk[64];
#define U(x) ((unsigned long long)(x))
#define ENTER() do { R[4] = (uint64_t)(uintptr_t)&stk[16]; stk[16] = 0xdead0000ull; } while (0)
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

fn stops(name: &str, api: &str, body: &str) {
    let o = run(name, &format!("{body}\n    printf(\"returned %llu\\n\", U(R[0]));"));
    assert!(!o.ok && !o.stdout.contains("returned"), "{name}: returned: {}", o.stdout);
    assert!(o.stderr.contains(&format!("fle: {api}")), "{name}: no 'fle: {api}': {}", o.stderr);
    assert_eq!(o.signal, Some(6), "{name}: not SIGABRT");
}

// ---- 1. Generator ---------------------------------------------------------------------------

/// Answered (decision 0078 section 1): the DLL_PROCESS_ATTACH line.
const ANSWERED: [&str; 21] = [   // decision 0238: old 17 (+ GetFileType, FlushFileBuffers, SetFilePointerEx, FindFirstFileExW, moved from STOPPED; their runtime tests are load2_* in tests/load.rs); decision 0126: old 16 (+ IsDebuggerPresent)
    "GetFileType",
    "FlushFileBuffers",
    "SetFilePointerEx",
    "FindFirstFileExW",
    "InitializeCriticalSectionAndSpinCount",
    "InitializeSListHead",
    "InterlockedFlushSList",
    "FlsAlloc",
    "FlsFree",
    "FlsGetValue",
    "FlsSetValue",
    "GetCommandLineA",
    "GetEnvironmentStringsW",
    "FreeEnvironmentStringsW",
    "GetOEMCP",
    "IsValidCodePage",
    // decision 0087: answered for CT_CTYPE1 and the range D (tests/locale.rs); it was in STOPPED (decision 0078 section 2).
    "GetStringTypeW",
    // decision 0102 W8: was in STOPPED (decision 0078 section 2); the start-up of a statically linked CRT reaches it.
    "EncodePointer",
    // decision 0102 W11: was in STOPPED; HeapAlloc now records the requested size of its blocks.
    "HeapSize",
    // decision 0113 wall 1: was in STOPPED; HeapReAlloc reallocates a block of the table of requested sizes.
    "HeapReAlloc",
    // decision 0126 wall 99 (S-no-debugger): was in STOPPED (decision 0078 section 2); returns 0, no debugger is attached to the virtual process (tests/wall_fr11.rs).
    "IsDebuggerPresent",
];

/// Stop by name (decision 0078 section 2).
const STOPPED: [&str; 12] = [   // decision 0238: old 16 (GetFileType, FlushFileBuffers, SetFilePointerEx and FindFirstFileExW are answered for the virtual volume: tests/load.rs, load2_*); decision 0126: old 17 (IsDebuggerPresent moved to ANSWERED)
    "RtlCaptureContext",
    "RtlLookupFunctionEntry",
    "RtlVirtualUnwind",
    "RtlUnwindEx",
    "RtlPcToFileHeader",
    "UnhandledExceptionFilter",
    "SetUnhandledExceptionFilter",
    "SetStdHandle",
    "GetConsoleMode",
    "GetConsoleOutputCP",
    "WriteConsoleW",
    "GetModuleHandleExW",
];

#[test]
fn every_answered_and_stopped_name_has_one_row_and_a_declaration() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let mut headers = String::new();
    for f in std::fs::read_dir(rt()).unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_some_and(|e| e == "h") {
            headers += &std::fs::read_to_string(&p).unwrap();
        }
    }
    for n in ANSWERED.iter().chain(STOPPED.iter()) {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(headers.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in a runtime header");
    }
    // The two implemented names that are deliberately NOT rows (decision 0078 section 5).
    for n in ["IsProcessorFeaturePresent", "TlsFree"] {
        assert!(!table.contains(&format!("\"{n}\", \"fle_imp_")), "{n} has no row");
        assert!(headers.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared");
    }
}

/// With the real DLL (skipped when absent: lift/out-ext is not in the repository): every `call [IAT]` and `jmp [IAT]` of the 35
/// KERNEL32 names of decision 0078 that has a row is answered; the one name without a row (IsProcessorFeaturePresent, 5 sites)
/// stays unsupported, and TlsFree (1 jmp site) too. No other name is left.
#[test]
fn real_dll_leaves_only_the_two_unrowed_names() {
    let path = manifest().join("../../out-ext/dsp_ippv2_x64.dll");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skipped: {} not present", path.display());
        return;
    };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let seeds = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut std::io::sink()).unwrap();
    for n in ANSWERED.iter().chain(STOPPED.iter()) {
        let c = st.handled.get(format!("call_imp:{n}").as_str()).copied().unwrap_or(0);
        let j = st.handled.get(format!("jmp_imp:{n}").as_str()).copied().unwrap_or(0);
        assert!(c + j >= 1, "{n}: no call_imp or jmp_imp site");
    }
    assert_eq!(st.unsupported["Call: indirect through IAT slot (not lifted)"], 5, "IsProcessorFeaturePresent sites");
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 1, "TlsFree site");
}

// ---- 2. Runtime: answered -------------------------------------------------------------------

#[test]
fn initialize_critical_section_and_spin_count_succeeds_and_the_section_works() {
    // Learn: "always succeeds and returns a nonzero value". The object is then a section like any other.
    let o = run(
        "icsasc",
        r#"
    static uint8_t cs[64], cs2[64];
    R[1] = (uint64_t)(uintptr_t)cs; R[2] = 0xfa0; R[0] = 99; fle_imp_InitializeCriticalSectionAndSpinCount();
    printf("init %llu\n", U(R[0]));
    R[1] = (uint64_t)(uintptr_t)cs; fle_imp_EnterCriticalSection();
    R[1] = (uint64_t)(uintptr_t)cs; fle_imp_EnterCriticalSection();
    R[1] = (uint64_t)(uintptr_t)cs; fle_imp_LeaveCriticalSection();
    R[1] = (uint64_t)(uintptr_t)cs; fle_imp_LeaveCriticalSection();
    R[1] = (uint64_t)(uintptr_t)cs; fle_imp_DeleteCriticalSection();
    /* spin count 0 and a very large one are the same */
    R[1] = (uint64_t)(uintptr_t)cs2; R[2] = 0; R[0] = 99; fle_imp_InitializeCriticalSectionAndSpinCount();
    printf("init0 %llu\n", U(R[0]));
    R[1] = (uint64_t)(uintptr_t)cs2; fle_imp_DeleteCriticalSection();
    /* a deleted section can be initialized again (Learn: "must be deleted before it can be reinitialized") */
    R[1] = (uint64_t)(uintptr_t)cs2; R[2] = 0xffffffff; fle_imp_InitializeCriticalSectionAndSpinCount();
    printf("again %llu\n", U(R[0]));
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[16]);
"#,
    );
    assert_eq!(lines(&o), ["init 1", "init0 1", "again 1", "rsp_kept 1"]);
}

#[test]
fn initialize_critical_section_and_spin_count_on_a_live_section_stops() {
    // Learn: "Initializing a critical section that is already initialized results in undefined behavior".
    stops(
        "icsasc-twice",
        "InitializeCriticalSectionAndSpinCount",
        "static uint8_t cs[64]; R[1] = (uint64_t)(uintptr_t)cs; R[2] = 0; fle_imp_InitializeCriticalSectionAndSpinCount(); \
         R[1] = (uint64_t)(uintptr_t)cs; R[2] = 0; fle_imp_InitializeCriticalSectionAndSpinCount();",
    );
    stops(
        "icsasc-after-initializecriticalsection",
        "InitializeCriticalSectionAndSpinCount",
        "static uint8_t cs[64]; R[1] = (uint64_t)(uintptr_t)cs; fle_imp_InitializeCriticalSection(); \
         R[1] = (uint64_t)(uintptr_t)cs; R[2] = 0; fle_imp_InitializeCriticalSectionAndSpinCount();",
    );
    stops("icsasc-null", "InitializeCriticalSectionAndSpinCount", "R[1] = 0; R[2] = 0; fle_imp_InitializeCriticalSectionAndSpinCount();");
}

#[test]
fn slist_head_is_kept_outside_the_object_and_flushes_empty() {
    // Learn (InitializeSListHead): the structure "is for system use only"; (InterlockedFlushSList): "If the list is empty, the
    // return value is NULL". Nothing can push, so an initialized head is empty. The object's bytes are never touched.
    let o = run(
        "slist",
        r#"
    static uint8_t h[16] __attribute__((aligned(16)));
    memset(h, 0xA5, sizeof h);
    R[1] = (uint64_t)(uintptr_t)h; R[0] = 7; fle_imp_InitializeSListHead();
    printf("init_rax %llu\n", U(R[0]));
    { int same = 1; for (int i = 0; i < 16; i++) if (h[i] != 0xA5) same = 0; printf("bytes_untouched %d\n", same); }
    R[1] = (uint64_t)(uintptr_t)h; R[0] = 77; fle_imp_InterlockedFlushSList();
    printf("flush %llu\n", U(R[0]));
    R[1] = (uint64_t)(uintptr_t)h; fle_imp_InitializeSListHead();
    R[1] = (uint64_t)(uintptr_t)h; R[0] = 77; fle_imp_InterlockedFlushSList();
    printf("flush_after_reinit %llu\n", U(R[0]));
"#,
    );
    // A function that returns VOID leaves R[0] alone (fle_sys.h), so init_rax is the 7 set before the call.
    assert_eq!(lines(&o), ["init_rax 7", "bytes_untouched 1", "flush 0", "flush_after_reinit 0"]);
    stops(
        "slist-flush-uninit",
        "InterlockedFlushSList",
        "static uint8_t h[16] __attribute__((aligned(16))); R[1] = (uint64_t)(uintptr_t)h; fle_imp_InterlockedFlushSList();",
    );
    stops("slist-init-null", "InitializeSListHead", "R[1] = 0; fle_imp_InitializeSListHead();");
}

#[test]
fn fls_follows_the_learn_pages() {
    // FlsAlloc: "an FLS index initialized to zero" (the slot reads 0); FlsSetValue/GetValue round trip; FlsSetValue on an index
    // that was never allocated: ERROR_INVALID_PARAMETER 87 ("The index is not in range"); FlsFree nonzero.
    let o = run(
        "fls",
        r#"
    uint64_t a, b;
    R[1] = 0; fle_imp_FlsAlloc(); a = R[0];
    R[1] = 0; fle_imp_FlsAlloc(); b = R[0];
    printf("distinct %d not_out_of_indexes %d\n", a != b, a != 0xFFFFFFFFull && b != 0xFFFFFFFFull);
    R[1] = a; R[0] = 5; fle_imp_FlsGetValue(); printf("fresh %llu\n", U(R[0]));
    R[1] = a; R[2] = 0x1234; fle_imp_FlsSetValue(); printf("set %llu\n", U(R[0]));
    R[1] = a; fle_imp_FlsGetValue(); printf("get_a %llx\n", U(R[0]));
    R[1] = b; fle_imp_FlsGetValue(); printf("get_b %llx\n", U(R[0]));
    R[1] = b; R[2] = 0x5678; fle_imp_FlsSetValue();
    R[1] = a; fle_imp_FlsGetValue(); printf("get_a_again %llx\n", U(R[0]));
    R[1] = 100000; R[2] = 1; fle_set_last_error(5); fle_imp_FlsSetValue();
    { uint64_t r = R[0]; fle_imp_GetLastError(); printf("set_out_of_range %llu err %llu\n", U(r), U(R[0])); }
    R[1] = a; R[2] = 0; fle_imp_FlsSetValue();
    R[1] = a; fle_imp_FlsFree(); printf("free %llu\n", U(R[0]));
    R[1] = b; fle_imp_FlsGetValue(); printf("b_after_free_a %llx\n", U(R[0]));
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[16]);
"#,
    );
    assert_eq!(
        lines(&o),
        [
            "distinct 1 not_out_of_indexes 1",
            "fresh 0",
            "set 1",
            "get_a 1234",
            "get_b 0",
            "get_a_again 1234",
            "set_out_of_range 0 err 87",
            "free 1",
            "b_after_free_a 5678",
            "rsp_kept 1",
        ]
    );
}

#[test]
fn fls_cases_the_pages_leave_open_stop() {
    // FlsGetValue and FlsFree on an index that was not allocated or was freed: "the function fails" without a code.
    stops("fls-get-unallocated", "FlsGetValue", "R[1] = 3; fle_imp_FlsGetValue();");
    stops("fls-free-unallocated", "FlsFree", "R[1] = 3; fle_imp_FlsFree();");
    stops(
        "fls-free-twice",
        "FlsFree",
        "R[1] = 0; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; fle_imp_FlsFree(); R[1] = a; fle_imp_FlsFree(); }",
    );
    stops(
        "fls-get-freed",
        "FlsGetValue",
        "R[1] = 0; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; fle_imp_FlsFree(); R[1] = a; fle_imp_FlsGetValue(); }",
    );
    stops(
        "fls-set-freed",
        "FlsSetValue",
        "R[1] = 0; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; fle_imp_FlsFree(); R[1] = a; R[2] = 1; fle_imp_FlsSetValue(); }",
    );
    // FlsFree "causes the associated callback routine to be called ... if the corresponding FLS slot contains a non-NULL value":
    // a guest function is not called from here, so that combination stops; a NULL slot, or no callback, is freed.
    stops(
        "fls-free-with-callback-and-value",
        "FlsFree",
        "R[1] = 0x401000; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; R[2] = 9; fle_imp_FlsSetValue(); R[1] = a; fle_imp_FlsFree(); }",
    );
    let o = run(
        "fls-free-callback-null-slot",
        r#"
    R[1] = 0x401000; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; fle_imp_FlsFree(); printf("callback_null_slot %llu\n", U(R[0])); }
    R[1] = 0; fle_imp_FlsAlloc(); { uint64_t a = R[0]; R[1] = a; R[2] = 9; fle_imp_FlsSetValue(); R[1] = a; fle_imp_FlsFree(); printf("no_callback_value %llu\n", U(R[0])); }
"#,
    );
    assert_eq!(lines(&o), ["callback_null_slot 1", "no_callback_value 1"]);
}

#[test]
fn get_command_line_a_is_the_ascii_text_of_the_wide_one() {
    let o = run(
        "cmdlineA",
        r#"
    const char *a; const uint16_t *w; size_t i, n;
    fle_imp_GetCommandLineA(); a = (const char *)(uintptr_t)R[0];
    fle_imp_GetCommandLineW(); w = (const uint16_t *)(uintptr_t)R[0];
    n = strlen(a);
    printf("same_length %d\n", n == fle_w_len(w));
    { int same = 1; for (i = 0; i <= n; i++) if ((uint16_t)(uint8_t)a[i] != w[i]) same = 0; printf("same_text %d\n", same); }
    printf("text %s\n", a);
    fle_imp_GetCommandLineA(); printf("stable %d\n", (const char *)(uintptr_t)R[0] == a);
"#,
    );
    assert_eq!(
        lines(&o),
        [
            "same_length 1",
            "same_text 1",
            // decision 0018: the quoted virtual path of the lifted DLL, no arguments
            "text \"C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll\"",
            "stable 1",
        ]
    );
}

#[test]
fn environment_strings_block_has_the_page_format() {
    // Learn: "Var1=Value1\0 ... VarN=ValueN\0\0". The virtual environment starts empty (decision 0018): the empty block is the
    // single terminating NUL. After SetEnvironmentVariableW the block holds NAME=value\0 and the final NUL.
    let o = run(
        "envblock",
        r#"
    uint16_t *b; size_t n = 0;
    fle_imp_GetEnvironmentStringsW(); b = (uint16_t *)(uintptr_t)R[0];
    printf("nonnull %d first_is_nul %d\n", b != NULL, b[0] == 0);
    R[1] = (uint64_t)(uintptr_t)b; R[0] = 7; fle_imp_FreeEnvironmentStringsW(); printf("free %llu\n", U(R[0]));
    { uint16_t nm[] = { 'F','L','E','_','A',0 }, v1[] = { 'x',0 }, nm2[] = { 'B',0 }, v2[] = { 'y','z',0 };
      R[1] = (uint64_t)(uintptr_t)nm; R[2] = (uint64_t)(uintptr_t)v1; fle_imp_SetEnvironmentVariableW();
      R[1] = (uint64_t)(uintptr_t)nm2; R[2] = (uint64_t)(uintptr_t)v2; fle_imp_SetEnvironmentVariableW(); }
    fle_imp_GetEnvironmentStringsW(); b = (uint16_t *)(uintptr_t)R[0];
    { const uint16_t *p = b; int k = 0; char got[3][16];
      while (*p) { size_t l = fle_w_len(p), j; for (j = 0; j < l; j++) got[k][j] = (char)p[j]; got[k][l] = 0; k++; p += l + 1; }
      printf("entries %d\n", k); { int i; for (i = 0; i < k; i++) printf("entry %s\n", got[i]); }
      printf("terminated %d\n", p[0] == 0); n = (size_t)(p - b) + 1; printf("units %zu\n", n); }
    R[1] = (uint64_t)(uintptr_t)b; fle_imp_FreeEnvironmentStringsW(); printf("free2 %llu\n", U(R[0]));
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[16]);
"#,
    );
    assert_eq!(
        lines(&o),
        [
            "nonnull 1 first_is_nul 1",
            "free 1",
            "entries 2",
            "entry FLE_A=x",
            "entry B=yz",
            "terminated 1",
            // "FLE_A=x\0" 8 + "B=yz\0" 5 + the final NUL 1
            "units 14",
            "free2 1",
            "rsp_kept 1",
        ]
    );
    // FreeEnvironmentStringsW takes "a pointer ... obtained by calling the GetEnvironmentStringsW function": anything else stops.
    stops("envfree-foreign", "FreeEnvironmentStringsW", "static uint16_t x[2]; R[1] = (uint64_t)(uintptr_t)x; fle_imp_FreeEnvironmentStringsW();");
    stops(
        "envfree-twice",
        "FreeEnvironmentStringsW",
        "fle_imp_GetEnvironmentStringsW(); { uint64_t b = R[0]; R[1] = b; fle_imp_FreeEnvironmentStringsW(); R[1] = b; fle_imp_FreeEnvironmentStringsW(); }",
    );
}

#[test]
fn oem_code_page_and_valid_code_pages_follow_the_emulated_locale() {
    // GetOEMCP: "the current OEM code page identifier for the operating system" = the emulated locale's OEM code page
    // (decision 0016: 437). IsValidCodePage: nonzero for the installed ones; only the two of the environment (1252, 437)
    // are decided, any other identifier stops (the page lists none).
    let o = run(
        "cp",
        r#"
    R[0] = 99; fle_imp_GetOEMCP(); printf("oemcp %llu\n", U(R[0]));
    fle_imp_GetACP(); printf("acp %llu\n", U(R[0]));
    R[1] = 1252; fle_imp_IsValidCodePage(); printf("valid_1252 %llu\n", U(R[0]));
    R[1] = 437; fle_imp_IsValidCodePage(); printf("valid_437 %llu\n", U(R[0]));
"#,
    );
    assert_eq!(lines(&o), ["oemcp 437", "acp 1252", "valid_1252 1", "valid_437 1"]);
    for (label, cp) in [("65001", 65001u32), ("932", 932), ("0", 0), ("99999", 99999), ("0xffffffff", 0xffffffff)] {
        stops(&format!("validcp-{label}"), "IsValidCodePage", &format!("R[1] = {cp}u; fle_imp_IsValidCodePage();"));
    }
}

#[test]
fn is_processor_feature_present_agrees_with_the_cpuid_answer() {
    // CPUID (decisions 0009, 0047, 0118): leaf 1 EDX has FPU (0), CX8 (8), CMOV (15), MMX (23), FXSR (24), SSE (25), SSE2 (26); ECX 0; no leaf 7;
    // no extended leaves. PF numbers from the Learn page. Every feature the CPUID answer does not report is 0; every feature it reports that has a PF number is 1
    // (decision 0119, PM's decision 2: PF_COMPARE_EXCHANGE_DOUBLE (2) = CX8 and PF_MMX_INSTRUCTIONS_AVAILABLE (3) = MMX were 0 until then; decision 0118 had left them
    // and listed them as an open point); PAE is 1 ("All x64 processors always return a nonzero value"); FASTFAIL is 1 (the __fastfail page: native support from Windows 8;
    // the emulated OS is Windows 10).
    let table: [(u32, u32); 19] = [
        (6, 1),  // PF_XMMI_INSTRUCTIONS_AVAILABLE (SSE)
        (10, 1), // PF_XMMI64_INSTRUCTIONS_AVAILABLE (SSE2)
        (9, 1),  // PF_PAE_ENABLED
        (23, 1), // PF_FASTFAIL_AVAILABLE
        (2, 1),  // PF_COMPARE_EXCHANGE_DOUBLE (EDX bit 8 = CX8 is 1 since decision 0118; this answer follows it since decision 0119)
        (3, 1),  // PF_MMX_INSTRUCTIONS_AVAILABLE (EDX bit 23 = MMX is 1 since decision 0118; this answer follows it since decision 0119)
        (7, 0),  // PF_3DNOW_INSTRUCTIONS_AVAILABLE
        (8, 0),  // PF_RDTSC_INSTRUCTION_AVAILABLE (EDX bit 4 is 0: TSC is not in the baseline)
        (13, 0), // PF_SSE3_INSTRUCTIONS_AVAILABLE
        (14, 0), // PF_COMPARE_EXCHANGE128
        (17, 0), // PF_XSAVE_ENABLED
        (22, 0), // PF_RDWRFSGSBASE_AVAILABLE
        (36, 0), // PF_SSSE3_INSTRUCTIONS_AVAILABLE
        (37, 0), // PF_SSE4_1_INSTRUCTIONS_AVAILABLE
        (38, 0), // PF_SSE4_2_INSTRUCTIONS_AVAILABLE
        (39, 0), // PF_AVX_INSTRUCTIONS_AVAILABLE
        (40, 0), // PF_AVX2_INSTRUCTIONS_AVAILABLE
        (41, 0), // PF_AVX512F_INSTRUCTIONS_AVAILABLE
        (60, 0), // PF_BMI2_INSTRUCTIONS_AVAILABLE
    ];
    let mut body = String::new();
    for (f, _) in &table {
        body += &format!("    R[1] = {f}; R[0] = 99; fle_imp_IsProcessorFeaturePresent(); printf(\"pf%u %llu\\n\", {f}u, U(R[0]));\n");
    }
    // The answers are compared with the CPUID the runtime itself gives.
    body += r#"
    R[0] = 1; R[1] = 0; fle_cpuid();
    printf("cpuid1_edx_sse %llu sse2 %llu mmx %llu tsc %llu cx8 %llu ecx %llu\n", U((R[2] >> 25) & 1), U((R[2] >> 26) & 1), U((R[2] >> 23) & 1), U((R[2] >> 4) & 1), U((R[2] >> 8) & 1), U(R[1]));
    printf("cpuid1_edx_fpu %llu cmov %llu fxsr %llu\n", U((R[2] >> 0) & 1), U((R[2] >> 15) & 1), U((R[2] >> 24) & 1));
    R[0] = 7; R[1] = 0; fle_cpuid(); printf("leaf7_is_leaf1_data %d\n", R[2] == 0x07808101ull);
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[16]);
"#;
    let o = run("pf", &format!("{body}"));
    let mut want: Vec<String> = table.iter().map(|(f, v)| format!("pf{f} {v}")).collect();
    want.push("cpuid1_edx_sse 1 sse2 1 mmx 1 tsc 0 cx8 1 ecx 0".to_string());
    want.push("cpuid1_edx_fpu 1 cmov 1 fxsr 1".to_string());
    want.push("leaf7_is_leaf1_data 1".to_string());
    want.push("rsp_kept 1".to_string());
    assert_eq!(lines(&o), want);
    // A number that no choice of the environment decides: stops by name, with the number.
    for f in [0u32, 1, 12, 15, 20, 21, 24, 29, 42, 61, 89, 1000] {
        let o = run(&format!("pf-stop-{f}"), &format!("R[1] = {f}; fle_imp_IsProcessorFeaturePresent(); printf(\"returned %llu\\n\", U(R[0]));"));
        assert!(!o.ok && !o.stdout.contains("returned"), "pf {f}: returned");
        assert!(o.stderr.contains("fle: IsProcessorFeaturePresent") && o.stderr.contains(&format!("feature {f}:")), "pf {f}: {}", o.stderr);
        assert_eq!(o.signal, Some(6));
    }
}

// ---- 3. Runtime: stop by name ---------------------------------------------------------------

#[test]
fn the_names_that_stop_say_their_name_and_never_return() {
    for n in STOPPED {
        // R[0] is set to a marker first: nothing may be returned (the run ends before the printf).
        stops(&format!("stop-{n}"), n, &format!("R[0] = 0x600D; R[1] = 0; R[2] = 0; R[8] = 0; fle_imp_{n}();"));
    }
}

// ---- 3b. Generated code: thunk and `call [IAT]`, through the generator ------------------------

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
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// A PE importing kernel32.dll!{names}; slot i is at 0x2200 + 8 * i. The code: `call [slot i]` for each name in turn, then `ret`.
fn pe_calling(names: &[&str]) -> Vec<u8> {
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
    b[data_at(0x2300)..data_at(0x2300) + 13].copy_from_slice(b"kernel32.dll\0");
    let mut hn = 0x2400u32;
    for (i, n) in names.iter().enumerate() {
        let at = data_at(hn);
        b[at + 2..at + 2 + n.len()].copy_from_slice(n.as_bytes());
        w64(&mut b, data_at(0x2100 + 8 * i as u32), hn as u64);
        w64(&mut b, data_at(0x2200 + 8 * i as u32), hn as u64);
        hn += 0x30;
    }
    let mut at = 0x1000u32;
    for i in 0..names.len() as u32 {
        let o = TEXT_RAW + (at - TEXT_RVA) as usize;
        b[o] = 0xff;
        b[o + 1] = 0x15;
        b[o + 2..o + 6].copy_from_slice(&rel(at + 6, 0x2200 + 8 * i));
        at += 6;
    }
    b[TEXT_RAW + (at - TEXT_RVA) as usize] = 0xc3;
    b
}

#[test]
fn call_through_the_iat_reaches_the_new_runtime_functions_and_the_stop_aborts() {
    let names = ["InitializeCriticalSectionAndSpinCount", "FlsAlloc", "GetOEMCP"];
    let img = pe_calling(&names);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    for n in names {
        assert_eq!(st.handled[format!("call_imp:{n}").as_str()], 1, "{n}");
    }
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    assert!(c.contains("void fle_imp_FlsAlloc(void); R[4] -= 8;") || c.contains("{ void fle_imp_FlsAlloc(void);"), "{c}");
    let dir = work();
    let tag = etxt_common::uniq("gen-call");
    let gen = dir.join(format!("{tag}.c"));
    let main = dir.join(format!("{tag}-main.c"));
    let exe = dir.join(&tag);
    std::fs::write(&gen, &c).unwrap();
    std::fs::write(
        &main,
        r#"
#include <stdio.h>
#include <string.h>
#include "fle_runtime.h"
extern void f_140001000(void);
static uint64_t stk[64];
static uint8_t cs[64];
int main(void)
{
    stk[32] = 0xdead0000ull; R[4] = (uint64_t)(uintptr_t)&stk[32];
    /* rcx = the section, edx = the spin count; the three calls run in order; rax after the last is the OEM code page */
    R[1] = (uint64_t)(uintptr_t)cs; R[2] = 4000; R[0] = 0;
    f_140001000();
    printf("rax %llu\n", (unsigned long long)R[0]);
    printf("rsp_kept %d\n", R[4] == (uint64_t)(uintptr_t)&stk[32]);
    return 0;
}
"#,
    )
    .unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-I"]).arg(rt());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&gen).arg(&main).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().unwrap();
    let so = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(r.status.success(), "{so}\n{}", String::from_utf8_lossy(&r.stderr));
    // GetOEMCP is the last call: 437 (decision 0016). The first two ran without stopping (the section was initialized, FlsAlloc gave an index).
    assert_eq!(so.lines().collect::<Vec<_>>(), ["rax 437", "rsp_kept 1"]);

    // A stopping name through the same form: `call [RtlCaptureContext]` ends the run by name.
    let img = pe_calling(&["RtlCaptureContext"]);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    assert_eq!(st.handled["call_imp:RtlCaptureContext"], 1);
    let c = String::from_utf8(out).unwrap();
    let tag = etxt_common::uniq("gen-stop");
    let gen = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&gen, &c).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-I"]).arg(rt());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&gen).arg(&main).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().unwrap();
    assert_eq!(r.status.signal(), Some(6));
    assert!(String::from_utf8_lossy(&r.stderr).contains("fle: RtlCaptureContext:"), "{}", String::from_utf8_lossy(&r.stderr));
    assert!(!String::from_utf8_lossy(&r.stdout).contains("rax"));
}

// ---- 4. Source ------------------------------------------------------------------------------

fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

#[test]
fn the_new_code_reads_nothing_from_the_host() {
    // The additions of decision 0078 in each file, cut from the first marker to the end of the function group.
    let cases: [(&str, &str); 5] = [
        ("fle_sys.c", "fle_cs_init"),
        ("fle_imports.c", "Fiber local storage"),
        ("fle_env.c", "kernel32!GetCommandLineA"),
        ("fle_locale.c", "kernel32!GetOEMCP"),
        ("fle_sysinfo.c", "kernel32!IsProcessorFeaturePresent"),
    ];
    for (f, marker) in cases {
        let raw = std::fs::read_to_string(rt().join(f)).unwrap();
        let at = raw.find(marker).unwrap_or_else(|| panic!("{f}: {marker}"));
        let code = strip_comments(&raw[at..(at + 6000).min(raw.len())]);
        for bad in ["getenv(", "environ[", "extern char **environ", "getpid", "sysconf", "get_nprocs", "/proc", "/sys/", "cpuid", "clock_gettime", "fopen", "open("] {
            assert!(!code.contains(bad), "{f}: {bad}");
        }
    }
}
