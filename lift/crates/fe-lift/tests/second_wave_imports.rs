//! Tests for the imports of the second wave (Issue #1538; the table of every import of the built-in plugin DLLs is
//! pm-work/findings/second-wave-imports.md). Each import this change answers has its row in src/imports_table.rs, its runtime
//! function in runtime/, and a test here that calls it through the import thunk of a generated unit (the synthetic PE below), the
//! unit being linked with every runtime/fle_*.c. The values come from the Microsoft Learn pages named in each case, written here,
//! not read back from the C code. The host is Linux: nothing is compared with a real Windows, and no window, file or network is used.
//!
//! An import that no row answers stops by name: `fle: import <dll>!<name> is not modeled` (src/lib.rs, unmodelled_import).

mod etxt_common;

use fe_lift::generate;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;
const IB: u64 = 0x1_4000_0000;
const SLOT: u32 = 0x2200;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    manifest().join("../../runtime")
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-second-wave/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

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

/// A PE32+ image with one import descriptor (`dll`, one thunk `api` at SLOT) and the entry 0x1000: `call [SLOT]; ret`.
fn build(dll: &str, api: &str) -> Vec<u8> {
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
    w32(&mut b, d + 16, SLOT);
    put(&mut b, 0x2300, format!("{dll}\0").as_bytes());
    put(&mut b, 0x2400, &[[0u8, 0u8].as_slice(), api.as_bytes(), &[0u8]].concat());
    w64(&mut b, data_at(0x2100), 0x2400);
    w64(&mut b, data_at(SLOT), 0x2400);

    let t = TEXT_RAW;
    b[t] = 0xff;
    b[t + 1] = 0x15;
    b[t + 2..t + 6].copy_from_slice(&((SLOT as i64 - 0x1006) as i32).to_le_bytes());
    b[t + 6] = 0xc3;
    b
}

const MAIN_HEAD: &str = r#"
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
extern void f_140001000(void);
void fle_imp_InitializeCriticalSection(void);
void fle_imp_LeaveCriticalSection(void);
void fle_imp_DeleteCriticalSection(void);
void fle_imp_CreateMutexW(void);
void fle_imp_GetLastError(void);
static uint64_t stk[64];
"#;

/// The main program of one case: `setup` runs before the entry (the case sets the R registers in it), `check` after the entry returns.
fn main_src(setup: &str, check: &str) -> String {
    format!(
        "{MAIN_HEAD}\nint main(void)\n{{\n    R[4] = (uint64_t)(uintptr_t)&stk[32];\n    stk[32] = 0xdead0000ull;\n    R[0] = 0xBAD;\n{setup}\n    f_140001000();\n{check}\n    return 0;\n}}\n"
    )
}

struct Run {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

/// Every runtime/fle_*.c compiled once per process (the rows can name any `fle_imp_*`), with the two support stubs of call_iat.rs.
fn runtime_objs() -> &'static Vec<PathBuf> {
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
        srcs.push(manifest().join("tests/support/clip_host_stub.c"));
        srcs.push(manifest().join("tests/support/mutex_real_stub.c"));
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

/// One case: `dll!api` is the entry's only call; the unit is generated, linked with the runtime, and run.
fn run(name: &str, dll: &str, api: &str, setup: &str, check: &str) -> Run {
    let mut out = Vec::new();
    generate(&build(dll, api), &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    let dir = work();
    let tag = etxt_common::uniq(name);
    let gen_c = dir.join(format!("{tag}.c"));
    let main_c = dir.join(format!("{tag}-main.c"));
    let exe = dir.join(&tag);
    std::fs::write(&gen_c, c).unwrap();
    std::fs::write(&main_c, main_src(setup, check)).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-no-pie", "-I"]).arg(rt());
    for o in runtime_objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg(&gen_c).arg(&main_c).arg("-lm").arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    Run {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn lines(r: &Run) -> Vec<String> {
    assert!(r.ok, "did not exit normally (signal {:?}): {}\n{}", r.signal, r.stderr, r.stdout);
    r.stdout.lines().map(|s| s.to_string()).collect()
}

/// The rows of this change: (DLL, API, runtime function). Each is in the table exactly once, and in no extra-imports file of the plugin build.
const ROWS: &[(&str, &str, &str)] = &[
    ("kernel32.dll", "TryEnterCriticalSection", "fle_imp_TryEnterCriticalSection"),
    ("kernel32.dll", "GetUserDefaultLangID", "fle_imp_GetUserDefaultLangID"),
    ("kernel32.dll", "lstrcpyA", "fle_imp_lstrcpyA"),
    ("kernel32.dll", "lstrcatW", "fle_imp_lstrcatW"),
    ("kernel32.dll", "lstrcpynW", "fle_imp_lstrcpynW"),
    ("kernel32.dll", "CreateMutexW", "fle_imp_CreateMutexW"),
    ("kernel32.dll", "ReleaseMutex", "fle_imp_ReleaseMutex"),
    ("kernel32.dll", "OpenFileMappingW", "fle_imp_OpenFileMappingW"),
    ("user32.dll", "GetClassLongPtrW", "fle_imp_GetClassLongPtrW"),
    ("winmm.dll", "joyGetNumDevs", "fle_imp_joyGetNumDevs"),
    ("ole32.dll", "StringFromGUID2", "fle_imp_StringFromGUID2"),
    ("vcruntime140.dll", "memset", "fle_imp_memset"),
    ("api-ms-win-crt-string-l1-1-0.dll", "memset", "fle_imp_memset"),
];

#[test]
fn every_row_is_in_the_table_once_and_names_a_runtime_function() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let extra = std::fs::read_to_string(manifest().join("../../harness/seeds/ship6-imports.txt")).unwrap();
    for (dll, api, f) in ROWS {
        let row = format!("(\"{dll}\", \"{api}\", \"{f}\", \"jmp_imp:{api}\")");
        assert_eq!(table.matches(&row).count(), 1, "{dll}!{api}: exactly one row in imports_table.rs");
        let in_extra = extra.lines().any(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            w.len() >= 2 && w[0].eq_ignore_ascii_case(dll) && w[1] == *api
        });
        assert!(!in_extra, "{dll}!{api} must not be a row of the plugin build's extra imports (set_extra refuses a duplicate)");
        let defined = std::fs::read_dir(rt()).unwrap().any(|e| {
            let p = e.unwrap().path();
            p.extension().is_some_and(|x| x == "c") && std::fs::read_to_string(&p).unwrap().contains(&format!("void {f}(void)"))
        });
        assert!(defined, "{f} is defined in runtime/*.c");
    }
}

#[test]
fn try_enter_critical_section_enters_a_free_section_and_counts_the_owner_again() {
    // Learn TryEnterCriticalSection: nonzero when the section is entered or already owned by this thread; the count goes up. The entry is
    // called twice (the count reaches 2), then the section is left twice and deleted, which needs the count back at zero.
    let setup = "    static uint8_t cs[64];\n    R[1] = (uint64_t)(uintptr_t)cs;\n    fle_imp_InitializeCriticalSection();\n    R[1] = (uint64_t)(uintptr_t)cs;";
    let check = "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);\n    R[1] = (uint64_t)(uintptr_t)cs;\n    f_140001000();\n    printf(\"ret_again %llu\\n\", (unsigned long long)R[0]);\n    R[1] = (uint64_t)(uintptr_t)cs;\n    fle_imp_LeaveCriticalSection();\n    R[1] = (uint64_t)(uintptr_t)cs;\n    fle_imp_LeaveCriticalSection();\n    R[1] = (uint64_t)(uintptr_t)cs;\n    fle_imp_DeleteCriticalSection();\n    printf(\"deleted\\n\");";
    let l = lines(&run("try-enter-free", "kernel32.dll", "TryEnterCriticalSection", setup, check));
    assert_eq!(l, vec!["ret 1", "ret_again 1", "deleted"]);
}

#[test]
fn try_enter_critical_section_of_a_section_not_initialized_stops() {
    let setup = "    static uint8_t cs[64];\n    R[1] = (uint64_t)(uintptr_t)cs;";
    let r = run("try-enter-uninit", "kernel32.dll", "TryEnterCriticalSection", setup, "    printf(\"returned\\n\");");
    assert!(!r.ok, "{}", r.stdout);
    assert_eq!(r.signal, Some(6));
    assert!(r.stderr.contains("fle: TryEnterCriticalSection on a section"), "{}", r.stderr);
    assert!(!r.stdout.contains("returned"));
}

#[test]
fn get_user_default_lang_id_is_the_low_word_of_the_default_lcid() {
    // Learn: the LANGID of the user default locale. The emulated locale is en-US (LCID 0x0409, decision 0044), so the LANGID is 0x0409.
    let r = run("langid", "kernel32.dll", "GetUserDefaultLangID", "", "    printf(\"ret %llx\\n\", (unsigned long long)R[0]);");
    assert_eq!(lines(&r), vec!["ret 409"]);
}

#[test]
fn lstrcpya_copies_the_string_with_its_nul_and_returns_the_buffer() {
    let setup = "    static char dst[16];\n    static const char src[] = \"abc\";\n    static char *keep;\n    keep = dst;\n    memset(dst, 'x', sizeof dst);\n    R[1] = (uint64_t)(uintptr_t)dst;\n    R[2] = (uint64_t)(uintptr_t)src;";
    let check = "    printf(\"dst %s tail_x %d ret_is_dst %d\\n\", keep, keep[4] == 'x', R[0] == (uint64_t)(uintptr_t)keep);";
    let l = lines(&run("lstrcpya", "kernel32.dll", "lstrcpyA", setup, check));
    // the NUL after "abc" is copied; the byte after it is not touched
    assert_eq!(l, vec!["dst abc tail_x 1 ret_is_dst 1"]);
}

#[test]
fn lstrcatw_appends_after_the_end_of_the_buffer_and_returns_it() {
    let setup = "    static uint16_t dst[16] = { 'a', 'b', 0 };\n    static uint16_t src[] = { 'c', 'd', 0 };\n    static uint16_t *keep;\n    keep = dst;\n    R[1] = (uint64_t)(uintptr_t)dst;\n    R[2] = (uint64_t)(uintptr_t)src;";
    let check = "    printf(\"ret_is_dst %d\\n\", R[0] == (uint64_t)(uintptr_t)keep);\n    printf(\"units %d %d %d %d %d\\n\", keep[0], keep[1], keep[2], keep[3], keep[4]);";
    let l = lines(&run("lstrcatw", "kernel32.dll", "lstrcatW", setup, check));
    assert_eq!(l, vec!["ret_is_dst 1".to_string(), format!("units {} {} {} {} {}", 'a' as u32, 'b' as u32, 'c' as u32, 'd' as u32, 0)]);
}

#[test]
fn lstrcpynw_copies_at_most_count_minus_one_units_and_always_ends_with_a_nul() {
    // Learn lstrcpyn: at most iMaxLength - 1 characters, and the buffer is terminated by NUL.
    let setup = "    static uint16_t dst[8];\n    static uint16_t src[] = { 'a', 'b', 'c', 'd', 'e', 'f', 0 };\n    static uint16_t *keep;\n    keep = dst;\n    memset(dst, 0x55, sizeof dst);\n    R[1] = (uint64_t)(uintptr_t)dst;\n    R[2] = (uint64_t)(uintptr_t)src;\n    R[8] = 4;";
    let check = "    printf(\"units %d %d %d %d ret_is_dst %d\\n\", keep[0], keep[1], keep[2], keep[3], R[0] == (uint64_t)(uintptr_t)keep);";
    let l = lines(&run("lstrcpynw", "kernel32.dll", "lstrcpynW", setup, check));
    assert_eq!(l, vec![format!("units {} {} {} 0 ret_is_dst 1", 'a' as u32, 'b' as u32, 'c' as u32)]);
}

#[test]
fn create_mutex_w_makes_an_unnamed_handle() {
    // Learn CreateMutexW: a handle to the new mutex (nonzero); no name, no security attributes.
    let setup = "    R[1] = 0;\n    R[8] = 0;";
    let check = "    printf(\"handle_nonzero %d\\n\", R[0] != 0);";
    assert_eq!(lines(&run("create-mutex", "kernel32.dll", "CreateMutexW", setup, check)), vec!["handle_nonzero 1"]);
}

#[test]
fn release_mutex_answers_one_for_the_handle_of_a_mutex() {
    // Learn ReleaseMutex: nonzero on success. The handle is made by a direct call of CreateMutexW (the runtime function, not the thunk).
    let setup = "    R[1] = 0;\n    R[8] = 0;\n    fle_imp_CreateMutexW();\n    R[1] = R[0];";
    let check = "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);";
    assert_eq!(lines(&run("release-mutex", "kernel32.dll", "ReleaseMutex", setup, check)), vec!["ret 1"]);
}

#[test]
fn open_file_mapping_w_of_a_name_that_does_not_exist_fails_with_file_not_found() {
    // Learn OpenFileMapping: NULL on failure, and the last error ERROR_FILE_NOT_FOUND (2) when no object of that name exists.
    let setup = "    static uint16_t name[] = { 'n', 'o', 0 };\n    R[1] = 4;\n    R[2] = 0;\n    R[8] = (uint64_t)(uintptr_t)name;";
    let check = "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);\n    fle_imp_GetLastError();\n    printf(\"last_error %llu\\n\", (unsigned long long)R[0]);";
    assert_eq!(lines(&run("open-mapping", "kernel32.dll", "OpenFileMappingW", setup, check)), vec!["ret 0", "last_error 2"]);
}

#[test]
fn get_class_long_ptr_w_of_an_invalid_window_answers_zero_with_1400() {
    // Learn GetClassLongPtrW: zero on failure. The window rule is the one of SetClassLongPtrW (a choice without a source: 1400).
    let setup = "    R[1] = 0x1234;\n    R[2] = (uint64_t)(int64_t)(-14);";
    let check = "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);\n    fle_imp_GetLastError();\n    printf(\"last_error %llu\\n\", (unsigned long long)R[0]);";
    assert_eq!(lines(&run("getclass-invalid", "user32.dll", "GetClassLongPtrW", setup, check)), vec!["ret 0", "last_error 1400"]);
}

#[test]
fn joy_get_num_devs_answers_zero_devices() {
    // Learn joyGetNumDevs: the number of joystick devices. This machine has none.
    let r = run("joy", "winmm.dll", "joyGetNumDevs", "", "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);");
    assert_eq!(lines(&r), vec!["ret 0"]);
}

#[test]
fn string_from_guid2_writes_the_braced_upper_case_form_and_counts_39() {
    // Learn StringFromGUID2: "{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}" (38 characters) and the NUL: 39 units returned.
    let setup = "    static uint8_t g[16] = { 0x78, 0x56, 0x34, 0x12, 0xbc, 0x9a, 0xf0, 0xde, 1, 2, 3, 4, 5, 6, 7, 8 };\n    static uint16_t out[64];\n    static uint16_t *keep;\n    keep = out;\n    R[1] = (uint64_t)(uintptr_t)g;\n    R[2] = (uint64_t)(uintptr_t)out;\n    R[8] = 64;";
    let check = "    char s[40];\n    for (int i = 0; i < 39; i++) s[i] = (char)keep[i];\n    s[39] = 0;\n    printf(\"ret %llu\\n\", (unsigned long long)R[0]);\n    printf(\"text %s\\n\", s);";
    let l = lines(&run("guid2", "ole32.dll", "StringFromGUID2", setup, check));
    assert_eq!(l, vec!["ret 39".to_string(), "text {12345678-9ABC-DEF0-0102-030405060708}".to_string()]);
}

#[test]
fn string_from_guid2_into_a_buffer_of_38_units_answers_zero() {
    let setup = "    static uint8_t g[16] = { 0 };\n    static uint16_t out[64];\n    R[1] = (uint64_t)(uintptr_t)g;\n    R[2] = (uint64_t)(uintptr_t)out;\n    R[8] = 38;";
    let l = lines(&run("guid2-small", "ole32.dll", "StringFromGUID2", setup, "    printf(\"ret %llu\\n\", (unsigned long long)R[0]);"));
    assert_eq!(l, vec!["ret 0"]);
}

#[test]
fn memset_of_the_two_crt_dlls_fills_n_bytes_and_returns_the_pointer() {
    // The CRT memset: (dst rcx, value rdx, count r8); it returns dst.
    for dll in ["vcruntime140.dll", "api-ms-win-crt-string-l1-1-0.dll"] {
        let setup = "    static uint8_t buf[8];\n    static uint8_t *keep;\n    keep = buf;\n    memset(buf, 0, sizeof buf);\n    R[1] = (uint64_t)(uintptr_t)buf;\n    R[2] = 0x41;\n    R[8] = 4;";
        let check = "    printf(\"ret_is_dst %d bytes %d %d %d %d %d\\n\", R[0] == (uint64_t)(uintptr_t)keep, keep[0], keep[1], keep[2], keep[3], keep[4]);";
        let l = lines(&run(&format!("memset-{}", dll.len()), dll, "memset", setup, check));
        assert_eq!(l, vec!["ret_is_dst 1 bytes 65 65 65 65 0".to_string()], "{dll}");
    }
}

#[test]
fn an_import_that_no_row_answers_stops_by_name() {
    // kernel32!GetComputerNameA is in no row: its call stops with the name, not with "unsupported instruction".
    let r = run("unnamed", "kernel32.dll", "GetComputerNameA", "", "    printf(\"returned\\n\");");
    assert!(!r.ok, "{}", r.stdout);
    assert_eq!(r.signal, Some(6));
    assert!(r.stderr.contains("fle: import kernel32.dll!GetComputerNameA is not modeled"), "{}", r.stderr);
    assert!(!r.stderr.contains("unsupported instruction"), "{}", r.stderr);
    assert!(!r.stdout.contains("returned"));
}
