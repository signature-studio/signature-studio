//! Tests for the virtual input locale of runtime/fle_kbd.c: user32!GetKeyboardLayout(0)
//! (docs/decisions/0063-virtual-keyboard-layout.md; sources: docs/investigations/getkeyboardlayout-caller.md).
//!
//! Expected values are written here from the decision (Microsoft Learn: GetKeyboardLayout; decision 0016: LCID 0x0409; the PM's
//! provisional choice B1 for the high word), not read back from the C code. The low word is a LANGID (Learn: official) and is the
//! low 16 bits of the emulated LCID (decision 0016). The high word 0x0409 is a PROVISIONAL choice: Learn says only "a device handle
//! to the physical layout of the keyboard" and gives no value; 0x0409 is a real-Windows convention that only unofficial text
//! states. The host is Linux: nothing is compared with a real Windows; no keyboard, layout, input method, registry, clock, thread
//! or process is read.
//!
//! The C sources of fle_kbd.c / fle_kbd.h come from runtime/ when fle_kbd.c is there, otherwise from out-kbd/work/ (development).
//! FLE_KBD_SRC_DIR (a directory with fle_kbd.c and fle_kbd.h) overrides both: the mutation runs of the run record use it. Files
//! that already existed always come from runtime/.

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
    if let Ok(d) = std::env::var("FLE_KBD_SRC_DIR") {
        return PathBuf::from(d);
    }
    if runtime_dir().join("fle_kbd.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-kbd/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-kbd/test");
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

/// The objects: fle_kbd.c (warnings are errors), and the existing files it links with: fle_locale.c (fle_emulated_locale and the
/// locale answers the tests compare with), fle_imports.c (GetLastError, fle_set_last_error), fle_env.c (fle_stop).
struct Objs {
    kbd: PathBuf,
    locale: PathBuf,
    imports: PathBuf,
    env: PathBuf,
}

fn objs() -> &'static Objs {
    static O: OnceLock<Objs> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-kbd"));
        std::fs::create_dir_all(&dir).unwrap();
        let one = |name: &str, ours: bool| {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            o
        };
        Objs { kbd: one("fle_kbd.c", true), locale: one("fle_locale.c", false), imports: one("fle_imports.c", false), env: one("fle_env.c", false) }
    })
}

pub struct Out {
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
#include "fle_env.h"
#include "fle_locale.h"
#include "fle_kbd.h"

uint64_t R[16];
static uint64_t stk[32];

/* a[0..3] go to rcx, rdx, r8, r9; R[0] starts as a marker so that "not written" shows. */
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
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define U(x) ((unsigned long long)(x))
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
#define KBL(id) CALL(fle_imp_GetKeyboardLayout, (id), 0, 0, 0)
"#;

fn compile_main_with(name: &str, top: &str, body: &str, objs: &[&Path], extra_c: Option<&str>) -> PathBuf {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(int argc, char **argv)\n{{\n    (void)argc; (void)argv;\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs {
        cmd.arg(o);
    }
    if let Some(x) = extra_c {
        let xc = dir.join(format!("{uname}-x.c"));
        std::fs::write(&xc, x).unwrap();
        cmd.arg(xc);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn compile_main(name: &str, top: &str, body: &str) -> PathBuf {
    let o = objs();
    compile_main_with(name, top, body, &[&o.kbd, &o.locale, &o.imports, &o.env], None)
}

fn exec(exe: &Path, args: &[&str]) -> Out {
    let r = Command::new(exe).args(args).env("TMPDIR", work_dir().join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    let exe = compile_main(name, "", body);
    exec(&exe, &[])
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// One program for every call: argv[1] = the whole rcx (u64). Output "ret <hex>" (the whole R[0]).
fn cli() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        compile_main(
            "kbd-cli",
            "",
            r#"
    uint64_t rcx = strtoull(argv[1], 0, 0);
    uint64_t r = KBL(rcx);
    printf("ret %llx\n", U(r));
"#,
        )
    })
}

/// What GetKeyboardLayout(rcx) did: Ok(the whole 64-bit return value), or Err(stderr) when the run stopped (SIGABRT,
/// "fle: GetKeyboardLayout", nothing printed before the stop).
fn call(rcx: u64) -> Result<u64, String> {
    let o = exec(cli(), &[&format!("{rcx:#x}")]);
    if o.ok {
        let t: Vec<&str> = o.stdout.trim().split(' ').collect();
        assert_eq!(t[0], "ret", "odd output {}", o.stdout);
        return Ok(u64::from_str_radix(t[1], 16).unwrap());
    }
    assert_eq!(o.signal, Some(6), "rcx {rcx:#x}: not SIGABRT: {}", o.stderr);
    assert!(o.stderr.contains("fle: GetKeyboardLayout"), "rcx {rcx:#x}: no 'fle: GetKeyboardLayout' message: {}", o.stderr);
    assert!(o.stdout.is_empty(), "rcx {rcx:#x}: output before the stop: {}", o.stdout);
    Err(o.stderr)
}

/// The value of `#define NAME <number>` (decimal, or UINT64_C(0x..), or 0x..) in a runtime header.
fn def_const(file: &str, name: &str) -> u64 {
    let t = std::fs::read_to_string(src_dir().join(file)).or_else(|_| std::fs::read_to_string(runtime_dir().join(file))).unwrap();
    let line = t.lines().find(|l| l.starts_with(&format!("#define {name} "))).unwrap_or_else(|| panic!("{name} not in {file}"));
    let rest = line[format!("#define {name} ").len()..].trim_start();
    if let Some(r) = rest.strip_prefix("UINT64_C(") {
        let tok = r.split(')').next().unwrap();
        return u64::from_str_radix(tok.trim_start_matches("0x"), 16).unwrap();
    }
    if let Some(r) = rest.strip_prefix("0x") {
        let tok: String = r.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        return u64::from_str_radix(&tok, 16).unwrap();
    }
    rest.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// C text without comments (strings are kept).
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
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

/// Decision 0016: the emulated LCID is en-US 0x0409 (a setting of this lift environment). Learn, GetKeyboardLayout: the low word
/// of the HKL is a Language Identifier (official). The high word 0x0409 is the PM's provisional choice B1 (decision 0063).
const LCID: u64 = 0x0409;
const HIGH_WORD: u64 = 0x0409;
const HKL: u64 = (HIGH_WORD << 16) | LCID;
/// The virtual GetCurrentThreadId value (decision 0056), written here from the decision.
const TID: u64 = 0x1001;

// ---- 1. Table, header, constants ----

#[test]
fn the_row_is_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_kbd.h")).unwrap();
    let row = "(\"user32.dll\", \"GetKeyboardLayout\", \"fle_imp_GetKeyboardLayout\", \"jmp_imp:GetKeyboardLayout\")";
    assert_eq!(table.matches(row).count(), 1, "exactly one row in imports_table.rs");
    assert!(h.contains("void fle_imp_GetKeyboardLayout(void)"));
    // The neighbours that must agree with this HKL are not answered by this decision. EnumDisplayMonitors was in this list until
    // decision 0065 (runtime/fle_monitor.c, tests/monitor.rs) answered it: it was the next stop of the harness
    // (docs/investigations/next-stop-enumdisplaymonitors.md) and has nothing to do with the keyboard layout, so the claim
    // "not answered" no longer holds here.
    for n in [
        // old: "GetKeyState" was in this list; decision 0126 (wall 61, S-no-keys) answered it (and GetAsyncKeyState): runtime/fle_win.c, tests/wall_fr11.rs
        // old: "GetKeyboardState" was in this list; decision 0136 (the input round trip) answers it with a row appended at the end of the table: runtime/fle_input.c, tests/wall_fr14.rs
        "GetKeyboardLayoutList", "GetKeyboardLayoutNameW", "GetKeyboardLayoutNameA", "LoadKeyboardLayoutW", "ActivateKeyboardLayout",
        "GetKeyboardLayoutA",
    ] {
        assert!(!table.contains(&format!("\"{n}\"")), "{n} must not be answered by this decision");
    }
}

#[test]
fn the_constants_are_the_decided_ones() {
    // provisional choice B1 (decision 0063): the high word is 0x0409, a fixed constant
    assert_eq!(def_const("fle_kbd.h", "FLE_KBD_HKL_HIGH_WORD"), HIGH_WORD);
}

// ---- 2. The answer ----

#[test]
fn the_current_thread_gets_the_virtual_hkl() {
    // idThread 0 = "0 for the current thread" (Learn): 0x04090409, a 64-bit handle value whose upper 32 bits are 0
    let r = call(0).unwrap();
    assert_eq!(r, 0x0409_0409);
    assert_eq!(r, HKL);
    assert_eq!(r >> 32, 0, "upper 32 bits");
    assert_eq!(r & 0xFFFF, LCID, "low word = LANGID of the emulated locale");
    assert_eq!(r >> 16, HIGH_WORD, "high word");
}

#[test]
fn only_the_low_dword_of_rcx_is_the_argument() {
    // idThread is a DWORD: the guest may leave anything in the upper half of rcx (`xor ecx, ecx` clears all of it, but a caller
    // that loads ecx only leaves the upper half undefined)
    assert_eq!(call(0xFFFF_FFFF_0000_0000), Ok(HKL));
    assert_eq!(call(0xAAAA_AAAA_0000_0000), Ok(HKL));
    // and the low dword decides: a high bit with a low dword that is not 0 is another thread
    assert!(call(0x0000_0001_0000_0001).is_err());
    assert!(call(0xFFFF_FFFF_0000_0001).is_err());
    assert!(call(0x0000_0000_8000_0000).is_err());
}

#[test]
fn the_low_word_is_the_emulated_langid_and_follows_the_locale_object() {
    // Decision 0016: the same LCID answers GetThreadLocale (the whole LCID) and GetUserDefaultUILanguage (its low 16 bits);
    // the HKL's low word is that LANGID (Learn: "The low word contains a Language Identifier").
    let l = lines(
        "kbd-langid",
        r#"
    uint64_t h = KBL(0);
    uint64_t tl = CALL(fle_imp_GetThreadLocale, 0, 0, 0, 0);
    uint64_t ui = CALL(fle_imp_GetUserDefaultUILanguage, 0, 0, 0, 0);
    printf("%llx %llx %llx %llx\n", U(h), U(tl), U(ui), U(fle_kbd_hkl()));
    printf("%d %d\n", (int)((h & 0xFFFF) == (tl & 0xFFFF)), (int)((h & 0xFFFF) == ui));
    printf("%llx\n", U(fle_emulated_locale.lcid));
"#,
    );
    assert_eq!(l, ["4090409 409 409 4090409", "1 1", "409"]);
}

#[test]
fn the_low_word_is_the_lcid_and_the_high_word_is_not_derived_from_it() {
    // A different emulated locale (a German LCID with a sort id in its upper half: 0x00020407; the sort id's bit lies in a bit that
    // is 0 in 0x0409<<16, so an unmasked OR would change the result): the low word follows the LCID's low 16 bits (the sort id
    // is masked off), the high word stays 0x0409 (decision 0063: a fixed constant). fle_locale.c is replaced by a one-object
    // definition; nothing else changes.
    let o = objs();
    let exe = compile_main_with(
        "kbd-variant",
        "",
        r#"
    printf("%llx %llx\n", U(KBL(0)), U(fle_kbd_hkl()));
"#,
        &[&o.kbd, &o.imports, &o.env],
        Some("#include <stdint.h>\n#include \"fle_locale.h\"\nconst fle_locale_t fle_emulated_locale = { .lcid = 0x00020407, .acp = 1252, .oemcp = 437 };\n"),
    );
    let r = exec(&exe, &[]);
    assert!(r.ok, "{}", r.stderr);
    assert_eq!(r.stdout.trim(), "4090407 4090407");
}

#[test]
fn it_is_stateless_leaves_the_last_error_alone_and_keeps_the_return_address() {
    // Learn names no failure for this call; this call succeeds, so the last error is not touched
    let l = lines(
        "kbd-state",
        r#"
    uint64_t r1, r2;
    SETLE(0x1234);
    r1 = KBL(0);
    r2 = KBL(0);
    printf("same %d %d\n", (int)(r1 == r2), (int)(r1 == 0x4090409));
    printf("last error %x\n", LE());
    printf("return address %llx\n", U(stk[0]));
    SETLE(0);
    KBL(0);
    printf("last error %x\n", LE());
"#,
    );
    assert_eq!(l, ["same 1 1", "last error 1234", "return address dead0000", "last error 0"]);
}

#[test]
fn the_other_registers_do_not_matter_and_the_callee_saved_ones_are_kept() {
    // only rcx is an argument: rdx, r8, r9 hold anything. The registers a Win64 callee must preserve (rbx 3, rbp 5, rsi 6, rdi 7,
    // r12 12 .. r15 15 in R[]) are not changed, and rsp (R[4]) is not moved.
    let l = lines(
        "kbd-regs",
        r#"
    uint64_t keep[16];
    int i, same = 1;
    for (i = 0; i < 16; i++) R[i] = 0x1111000000000000ull + (uint64_t)i;
    memset(stk, 0, sizeof stk);
    R[1] = 0; R[2] = 0xFFFFFFFFFFFFFFFFull; R[8] = 0x1234; R[9] = 0x5678;
    R[4] = (uint64_t)(uintptr_t)stk;
    memcpy(keep, R, sizeof keep);
    fle_imp_GetKeyboardLayout();
    for (i = 0; i < 16; i++) {
        int volatile_reg = (i == 0 || i == 1 || i == 2 || i == 8 || i == 9 || i == 10 || i == 11);
        if (!volatile_reg && R[i] != keep[i]) same = 0;
    }
    printf("%llx %d\n", U(R[0]), same);
"#,
    );
    assert_eq!(l, ["4090409 1"]);
}

// ---- 3. Stops ----

#[test]
fn another_thread_id_stops_with_a_named_reason() {
    for id in [1u64, 4, 0x1000, 0x1002, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
        let e = call(id).expect_err(&format!("idThread {id:#x} must stop"));
        assert!(e.contains(&format!("idThread {id} is not 0")), "{id:#x}: {e}");
        assert!(e.contains("no thread table") && e.contains("decision 0063"), "{id:#x}: {e}");
        assert!(!e.contains("virtual GetCurrentThreadId"), "{id:#x}: only the virtual TID gets that reason: {e}");
    }
}

#[test]
fn the_virtual_get_current_thread_id_value_stops_with_its_own_reason() {
    // GetCurrentThreadId answers 0x1001 (decision 0056); that value is not a key into a thread table either
    let e = call(TID).expect_err("the virtual tid must stop");
    assert!(e.contains("virtual GetCurrentThreadId") && e.contains("decisions 0056, 0063"), "{e}");
    assert!(!e.contains("is not 0"), "{e}");
    // and the stub's TID is the one of the header of fle_imports.h
    assert_eq!(def_const("fle_imports.h", "FLE_EMULATED_TID"), TID);
    // a high half with the TID in the low half is the same value (DWORD)
    assert!(call(0xFFFF_FFFF_0000_1001).expect_err("tid").contains("virtual GetCurrentThreadId"));
}

// ---- 4. Source checks ----

#[test]
fn source_uses_no_host_keyboard_layout_input_method_registry_clock_thread_process_or_file() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_kbd.c")).unwrap());
    for bad in [
        "getpid", "getppid", "gettid", "getenv", "extern char **environ", "fopen", "open(", "stat(", "/proc", "/etc", "/home", "/dev", "/tmp", "exit(", "_exit",
        "clock_gettime", "gettimeofday", "time(", "clock(", "nanosleep", "usleep", "sleep(", "select(", "poll(", "<time.h>", "<sys/time.h>", "pthread",
        "<sys/", "<unistd.h>", "<signal.h>", "fork(", "exec", "syscall", "sched_", "futex", "asm", "__asm__", "uintptr_t", "abort(", "X11", "DISPLAY", "XKB",
        "setxkbmap", "localectl", "setlocale", "nl_langinfo", "LANG", "LC_", "wayland", "ibus", "fcitx", "xinput", "libinput", "ioctl", "emscripten", "SDL",
        "fle_host", "static int", "static uint64_t", "static uint32_t", "fle_set_last_error", "fle_reg", "fle_vfs",
    ] {
        assert!(!code.contains(bad), "fle_kbd.c must not use {bad}");
    }
    // `long` is not portable (32 bits on wasm32 and on Windows, 64 on Linux): fixed-width integers only
    let b = code.as_bytes();
    let mut i = 0;
    while let Some(p) = code[i..].find("long") {
        let at = i + p;
        let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let after = at + 4 >= b.len() || !(b[at + 4].is_ascii_alphanumeric() || b[at + 4] == b'_');
        assert!(!(before && after), "fle_kbd.c uses `long` near byte {at}");
        i = at + 4;
    }
    // the value comes from the locale object and the named constant; fle_stop is the stop; R[0] is written once
    assert!(code.contains("fle_emulated_locale.lcid") && code.contains("FLE_KBD_HKL_HIGH_WORD") && code.contains("fle_stop("));
    assert_eq!(code.matches("R[0] =").count(), 1, "R[0] is written once");
    assert_eq!(code.matches("fle_stop(").count(), 2, "two stops: the virtual tid (fixed reason) and any other id (formatted reason)");
}

#[test]
fn header_is_plain_c() {
    let h = strip_comments(&std::fs::read_to_string(src_dir().join("fle_kbd.h")).unwrap());
    for bad in ["asm", "__asm__", "<unistd.h>", "<sys/", "<pthread.h>", "<time.h>", "long "] {
        assert!(!h.contains(bad), "fle_kbd.h must not contain {bad}");
    }
}

// ---- 5. Generator: the thunk calls the runtime ----

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

/// A PE that imports `dll`!`api` (slot 0x2200). Entry 0x1000: `xor ecx,ecx; call 0x1040; ret`; 0x1040 is the thunk.
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
    w32(&mut b, d + 16, 0x2200);
    let mut nm = dll.as_bytes().to_vec();
    nm.push(0);
    put(&mut b, 0x2300, &nm);
    let mut hn = vec![0u8, 0u8];
    hn.extend_from_slice(api.as_bytes());
    hn.push(0);
    put(&mut b, 0x2400, &hn);
    for (rva, v) in [(0x2100, 0x2400u64), (0x2200, 0x2400)] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, vec![0x33, 0xc9]),
        (0x1002, [&[0xe8][..], &rel(0x1007, 0x1040)].concat()),
        (0x1007, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn the_thunk_calls_the_runtime() {
    let img = build("user32.dll", "GetKeyboardLayout");
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.handled["jmp_imp:GetKeyboardLayout"], 1);
    assert!(c.contains("fle_imp_GetKeyboardLayout(); return;"));
    let va = IB + 0x1040;
    assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{va:#x} still unsupported");
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
}

#[test]
fn the_dll_name_and_the_api_name_are_part_of_the_match() {
    // the same API name from another DLL, and the neighbours of the same DLL, are not this API: they stay unsupported
    for (dll, api) in [
        ("other.dll", "GetKeyboardLayout"), ("user32.dll", "GetKeyboardLayoutList"), ("user32.dll", "GetKeyboardLayoutNameW"), ("user32.dll", "GetKeyboardLayoutA"),
        ("user32.dll", "LoadKeyboardLayoutW"),
    ] {
        let img = build(dll, api);
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();
        assert!(!st.handled.contains_key("jmp_imp:GetKeyboardLayout"), "{dll}!{api}");
        assert!(!c.contains("fle_imp_GetKeyboardLayout"), "{dll}!{api}");
        let va = IB + 0x1040;
        assert!(c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported(UINT64_C({va:#x})); }}")), "{dll}!{api}");
    }
}

// ---- 6. The bytes of the real DLL that the investigation rests on ----

fn real_dll() -> Option<Vec<u8>> {
    let p = std::env::var("FLENGINE_DLL").map(PathBuf::from).unwrap_or_else(|_| manifest().join("../../../binaries/win64/FLEngine_x64.dll"));
    std::fs::read(p).ok()
}

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}

/// The file offset of `rva` (the section table is read here, not through fe-pe).
fn rva_off(dll: &[u8], rva: u32) -> usize {
    let lf = le32(dll, 0x3c) as usize;
    let nsec = le16(dll, lf + 6) as usize;
    let osz = le16(dll, lf + 20) as usize;
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        let (vs, sva, rs, rp) = (le32(dll, o + 8), le32(dll, o + 12), le32(dll, o + 16), le32(dll, o + 20));
        if rva >= sva && rva < sva + vs.max(rs) {
            return rp as usize + (rva - sva) as usize;
        }
    }
    panic!("rva {rva:#x} is in no section");
}

/// The file bytes at virtual address `va` (ImageBase 0x400000).
fn bytes_at(dll: &[u8], va: u64, n: usize) -> Vec<u8> {
    let off = rva_off(dll, (va - 0x400000) as u32);
    dll[off..off + n].to_vec()
}

/// Every import of the DLL as (dll name, function name, IAT slot VA). Ordinal imports have the name "#<ordinal>".
fn imports(dll: &[u8]) -> Vec<(String, String, u64)> {
    let lf = le32(dll, 0x3c) as usize;
    let dir_rva = le32(dll, lf + 24 + 112 + 8);
    let cstr = |rva: u32| -> String {
        let o = rva_off(dll, rva);
        let e = dll[o..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&dll[o..o + e]).to_string()
    };
    let mut v = Vec::new();
    let mut d = rva_off(dll, dir_rva);
    loop {
        let (oft, name, ft) = (le32(dll, d), le32(dll, d + 12), le32(dll, d + 16));
        if name == 0 && ft == 0 {
            break;
        }
        let dname = cstr(name);
        let mut t = rva_off(dll, if oft != 0 { oft } else { ft });
        let mut k = 0u32;
        loop {
            let e = u64::from_le_bytes(dll[t..t + 8].try_into().unwrap());
            if e == 0 {
                break;
            }
            let fname = if e >> 63 != 0 { format!("#{}", e & 0xFFFF) } else { cstr(e as u32 + 2) };
            v.push((dname.clone(), fname, 0x400000 + ft as u64 + 8 * k as u64));
            t += 8;
            k += 1;
        }
        d += 20;
    }
    v
}

/// The target of the `jmp [rip+disp32]` thunk at `va` (the IAT slot).
fn thunk_slot(dll: &[u8], va: u64) -> u64 {
    let t = bytes_at(dll, va, 6);
    assert_eq!(&t[..2], [0xff, 0x25], "{va:#x}: jmp [rip+disp32]");
    (va as i64 + 6 + i32::from_le_bytes(t[2..6].try_into().unwrap()) as i64) as u64
}

/// The three call sites of the thunk 0x424d60, from the objdump of the DLL: (call site, the DLL function around it).
const SITES: [(u64, &str); 3] = [(0x84364c, "f_843630"), (0x86521d, "f_8651a0"), (0xa16502, "f_a164f0")];

#[test]
fn the_thunk_jumps_to_the_slot_which_is_user32_getkeyboardlayout() {
    let Some(dll) = real_dll() else { return };
    let imps = imports(&dll);
    let slot = 0x1971120u64;
    let named: Vec<_> = imps.iter().filter(|(_, _, s)| *s == slot).collect();
    assert_eq!(named.len(), 1);
    assert_eq!((named[0].0.to_lowercase(), named[0].1.as_str()), ("user32.dll".to_string(), "GetKeyboardLayout"));
    assert_eq!(thunk_slot(&dll, 0x424d60), slot);
    // the neighbours that the HKL must agree with, and the next stop of the harness, are other imports (not answered)
    let find = |n: &str| imps.iter().find(|(_, f, _)| f == n).map(|(d, _, s)| (d.to_lowercase(), *s));
    assert_eq!(find("GetKeyboardLayoutList"), Some(("user32.dll".to_string(), 0x1970ab8)));
    assert_eq!(find("EnumDisplayMonitors"), Some(("user32.dll".to_string(), 0x1971058)));
    assert_eq!(thunk_slot(&dll, 0x424d70), 0x1970ab8);
    assert_eq!(thunk_slot(&dll, 0x5dca50), 0x1971058);
    // exactly one import with the name, and no ANSI variant
    let kl: Vec<_> = imps.iter().filter(|(_, n, _)| n == "GetKeyboardLayout" || n == "GetKeyboardLayoutA").collect();
    assert_eq!(kl.len(), 1, "{kl:?}");
}

#[test]
fn the_guest_passes_thread_id_zero_at_all_three_call_sites() {
    let Some(dll) = real_dll() else { return };
    for (site, _f) in SITES {
        let c = bytes_at(&dll, site, 5);
        assert_eq!(c[0], 0xe8, "{site:#x}: call");
        let target = (site as i64 + 5 + i32::from_le_bytes(c[1..5].try_into().unwrap()) as i64) as u64;
        assert_eq!(target, 0x424d60, "{site:#x}: target");
        // `xor ecx, ecx` (33 c9) immediately before the call: idThread = 0 at every site
        assert_eq!(bytes_at(&dll, site - 2, 2), [0x33, 0xc9], "{site:#x}: idThread = 0");
    }
}

#[test]
fn nothing_else_in_the_file_calls_or_points_at_the_thunk() {
    let Some(dll) = real_dll() else { return };
    // every `e8 rel32` in .text whose target is 0x424d60 is one of the three sites (a byte scan: a false hit would show up here)
    let lf = le32(&dll, 0x3c) as usize;
    let nsec = le16(&dll, lf + 6) as usize;
    let osz = le16(&dll, lf + 20) as usize;
    let mut hits = Vec::new();
    for i in 0..nsec {
        let o = lf + 24 + osz + 40 * i;
        if &dll[o..o + 5] != b".text" {
            continue;
        }
        let (rs, rp, sva) = (le32(&dll, o + 16) as usize, le32(&dll, o + 20) as usize, le32(&dll, o + 12) as u64);
        for k in rp..rp + rs - 5 {
            if dll[k] == 0xe8 {
                let va = 0x400000 + sva + (k - rp) as u64;
                let t = (va as i64 + 5 + i32::from_le_bytes(dll[k + 1..k + 5].try_into().unwrap()) as i64) as u64;
                if t == 0x424d60 {
                    hits.push(va);
                }
            }
        }
    }
    let want: Vec<u64> = SITES.iter().map(|(s, _)| *s).collect();
    assert_eq!(hits, want);
    // no absolute VA or RVA of the thunk or the slot is in the file: no table of function pointers holds them
    for va in [0x424d60u64, 0x1971120] {
        for pat in [(va as u32).to_le_bytes().to_vec(), va.to_le_bytes().to_vec(), ((va - 0x400000) as u32).to_le_bytes().to_vec()] {
            assert!(!dll.windows(pat.len()).any(|w| w == pat.as_slice()), "{va:#x}: {pat:02x?}");
        }
    }
}

#[test]
fn the_guest_uses_the_whole_hkl_the_low_word_and_nothing_inside_the_high_word() {
    let Some(dll) = real_dll() else { return };
    // 0x86521d (f_8651a0): the whole rax is stored at [rcx+0x90] (mov [rcx+0x90], rax: 48 89 81 90 00 00 00), no masking or shifting
    assert_eq!(bytes_at(&dll, 0x865222, 11), [0x48, 0x8b, 0x4d, 0x60, 0x48, 0x89, 0x81, 0x90, 0x00, 0x00, 0x00]);
    // 0x865e2f (f_865c90): `cmp rax, [rcx+0x90]` against an element of the GetKeyboardLayoutList result (48 3b 81 90 00 00 00)
    assert_eq!(bytes_at(&dll, 0x865e2f, 7), [0x48, 0x3b, 0x81, 0x90, 0x00, 0x00, 0x00]);
    // 0xa16502 (f_a164f0): `mov edx, eax; and edx, 0xffff` keeps the low word (the LANGID), the high word is dropped
    assert_eq!(bytes_at(&dll, 0xa16507, 11), [0x48, 0x89, 0xd9, 0x8b, 0xd0, 0x81, 0xe2, 0xff, 0xff, 0x00, 0x00]);
    // 0x84364c (f_843630): `mov rbx, rax; cmp rbx, [rip+disp]` compares the whole value with the previous one (0x14075a0)
    assert_eq!(bytes_at(&dll, 0x843651, 3), [0x48, 0x89, 0xc3]);
    assert_eq!(bytes_at(&dll, 0x843654, 3), [0x48, 0x3b, 0x1d]);
    assert_eq!(0x843654u64 + 7 + 0xbc3f45, 0x14075a0);
}
