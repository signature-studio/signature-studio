//! Tests of decision 0132 (docs/decisions/0132-seh-s1.md, PROVISIONAL): the response rows that the walls after wall 16 append (tests/rows_0132/mod.rs) and the answers that need no window or GDI driver.
//! The runtime half of the SEH itself is tests/seh_rt.rs (fle_seh.c on the real tables of FLEngine_x64.dll) and tests/seh.rs (the generator: wrappers and landings).
//!
//! 1. Generator: the rows of decision 0132 are the last rows, in the order given, right after the 581 of decision 0129, and no pseudo-VA moved.
//! 2. Each thunk is answered under its own dll only.
//!
//! The values of the answers themselves are pinned where their drivers are: CreateBitmap with initial bits and CreatePatternBrush in tests/gdi.rs, SHGetSpecialFolderLocation(CSIDL_SYSTEM) in tests/wall_fr3.rs,
//! ShowScrollBar in tests/wall_fr9.rs.

mod etxt_common;
mod rows_0132;
mod rows_0139;
mod rows_0140;
mod rows_0144;
mod rows_0147;
mod rows_0267;

use fe_lift::generate;
use std::path::PathBuf;


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


#[test]
fn the_rows_of_decision_0132_are_the_last_rows_in_the_order_given_and_no_pseudo_va_moved() {
    let (st, c) = gen(&GUEST_IMPORTS);
    let rows = resp_rows(&c);
    assert_eq!(rows.len(), st.resp_entries);
    let k = rows_0132::ROWS_0132.len();
    // decision 0136 appends 10 rows after these (tests/wall_fr14.rs) and decision 0139 appends 3 more (tests/rows_0139/mod.rs): the table of this test ends where decision 0132's rows end
    let n = rows.len() - 10 - rows_0139::ROWS_0139.len() - rows_0140::ROWS_0140.len() - rows_0144::ROWS_0144.len() - rows_0147::ROWS_0147.len() - rows_0267::ROWS_0267.len(); // decision 0267 appends 1 more after those of decision 0147; decision 0147 appends 1 more after those of decision 0144; decision 0140 appends 4 more after those of decision 0139, decision 0144 2 more after them
    // HEAD 3ce8e7f has 581 rows (544 of decision 0125, 33 of 0126, 4 of 0129); decision 0132 appends k (decision 0015: append only)
    assert_eq!(n, 581 + k, "the 581 rows of HEAD 3ce8e7f and the {k} of decision 0132");
    assert_eq!((rows[n - k - 1].0.as_str(), rows[n - k - 1].1.as_str()), ("ole32.dll", "CoCreateInstance"), "the row before the new ones is the last row of decision 0129");
    for (i, (dll, name)) in rows_0132::ROWS_0132.iter().enumerate() {
        assert_eq!((rows[n - k + i].0.as_str(), rows[n - k + i].1.as_str()), (*dll, *name), "row {}", n - k + i);
    }
    for (i, (_, _, va)) in rows.iter().enumerate() {
        assert_eq!(*va, 0x7FF9_0000_0000u64 + 0x10 * (i as u64 + 1), "row {i}: the pseudo-VA of a row never moves");
    }
    for (_, name) in rows_0132::ROWS_0132.iter() {
        assert_eq!(rows.iter().filter(|(_, n2, _)| n2 == name).count(), 1, "{name}: exactly one row");
        assert!(c.contains(&format!("void fle_imp_{name}(void);")), "{name} is declared in the generated C");
    }
}

#[test]
fn each_new_thunk_is_answered_under_its_own_dll_and_no_other_dll() {
    for (dll, name) in rows_0132::ROWS_0132.iter() {
        let other = if *dll == "user32.dll" { "kernel32.dll" } else { "user32.dll" };
        let (st, c) = gen(&[(dll, name), (other, name), (dll, "SendMessageA"), ("kernel32.dll", "GetProcAddress")]);
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{dll}!{name}");
        assert_eq!(c.matches(&format!("fle_imp_{name}(); return;")).count(), 1, "{dll}!{name}: only the thunk of its own dll is answered");
    }
}

// ---------------------------------------------------------------------------------------------
// 3. The offline WinINet session (runtime/fle_wininet.c, wall 23): a small C driver
// ---------------------------------------------------------------------------------------------

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

fn runtime_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-seh/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path) {
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-fPIE", "-c", "-I"])
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// fle_wininet.c (warnings are errors) with fle_imports.c and fle_env.c (the last error, fle_stop).
fn inet_objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj-inet"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for name in ["fle_wininet.c", "fle_imports.c", "fle_env.c"] {
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&runtime_dir().join(name), &o);
            v.push(o);
        }
        v
    })
}

const INET_TOP: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_wininet.h"
void fle_imp_FormatMessageW(void);
uint64_t R[16];
static uint64_t stk[256];
static uint64_t cN(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u; stk[5] = a5;
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
/* seven arguments: rcx rdx r8 r9 and [rsp+0x28] [rsp+0x30] [rsp+0x38] */
static uint64_t cN7(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7)
{
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u; stk[5] = a5; stk[6] = a6; stk[7] = a7;
    R[1] = a1; R[2] = a2; R[8] = a3; R[9] = a4;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
/* the virtual module list as fle_modules.c has it, for the names only (fle_imports.c asks fle_module_count / fle_module_name through weak references) */
uint64_t fle_module_count(void) { return 40; }
const char *fle_module_name(uint64_t i) { return i == 0 ? "kernel32.dll" : i == 20 ? "wininet.dll" : "other.dll"; }
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
static const uint16_t agent[] = { 'F', 'L', ' ', 'S', 0 }, url[] = { 'h', 't', 't', 'p', ':', '/', '/', 'x', '.', 'y', 0 };
"#;

struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

fn inet_run(name: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, format!("{INET_TOP}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n")).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"]).arg(runtime_dir());
    for o in inet_objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(&c);
    Out { stdout: String::from_utf8_lossy(&r.stdout).to_string(), stderr: String::from_utf8_lossy(&r.stderr).to_string(), signal: r.status.signal(), ok: r.status.success() }
}

fn inet_stops(name: &str, body: &str, words: &[&str]) {
    let o = inet_run(name, body);
    assert!(!o.ok && !o.stdout.contains("returned"), "{name}: did not stop: {}", o.stdout);
    assert_eq!(o.signal, Some(6), "{name}: {}", o.stderr);
    let msg = o.stderr.lines().find(|l| l.starts_with("fle: ")).unwrap_or_else(|| panic!("{name}: no message: {}", o.stderr));
    for w in words {
        assert!(msg.contains(w), "{name}: message lacks {w:?}: {msg}");
    }
}

#[test]
fn an_offline_session_opens_keeps_its_time_outs_fails_to_open_a_url_and_closes() {
    // decision 0132 (wall 23, PROVISIONAL). The guest's sequence (f_64bfa0): InternetOpenW("FL Studio", 0, NULL, NULL, 0); InternetSetOptionW(h, 6, &60000, 4); InternetOpenUrlW(h, url, NULL, 0, 0x80000000, 0) -> NULL;
    // InternetCloseHandle(h). Learn (InternetOpenW): a valid handle, NULL on failure; (InternetSetOptionW): TRUE; (InternetCloseHandle): TRUE. S-inet-open-url-fails (a choice without a source): NULL with 12007
    // (ERROR_INTERNET_NAME_NOT_RESOLVED, Learn Appendix: WinINet Error Codes: "The server name could not be resolved") because the machine has no connection.
    let o = inet_run(
        "inet-seq",
        r#"
    uint32_t t = 60000;
    uint64_t h, h2, u;
    fle_set_last_error(0x1234);
    h = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0);
    h2 = cN(fle_imp_InternetOpenW, A(agent), 1, 0, 0, 0);
    printf("two sessions: distinct %d, multiples of 4 %d, above the base %d, last error untouched %u\n", h != h2, (h % 4) == 0 && (h2 % 4) == 0, h > 0x7FFAB0000000ull && h2 > h, LE());
    printf("set receive time-out: %llu\n", U(cN(fle_imp_InternetSetOptionW, h, 6, A(&t), 4, 0)));
    printf("set connect time-out: %llu\n", U(cN(fle_imp_InternetSetOptionW, h, 2, A(&t), 4, 0)));
    u = cN(fle_imp_InternetOpenUrlW, h, A(url), 0, 0, 0x80000000u);
    printf("open url: %llu le %u\n", U(u), LE());
    printf("close: %llu\n", U(cN(fle_imp_InternetCloseHandle, h, 0, 0, 0, 0)));
    printf("close the other: %llu\n", U(cN(fle_imp_InternetCloseHandle, h2, 0, 0, 0, 0)));
    h = cN(fle_imp_InternetOpenW, A(agent), 4, 0, 0, 0);
    printf("a later session is a new handle: %d\n", h != 0 && h > h2);
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(
        l,
        [
            "two sessions: distinct 1, multiples of 4 1, above the base 1, last error untouched 4660",
            "set receive time-out: 1",
            "set connect time-out: 1",
            "open url: 0 le 12007",
            "close: 1",
            "close the other: 1",
            "a later session is a new handle: 1"
        ]
    );
}

#[test]
fn the_forms_of_wininet_the_guest_does_not_use_stop_by_name() {
    inet_stops("inet-agent-null", "cN(fle_imp_InternetOpenW, 0, 0, 0, 0, 0);", &["InternetOpenW", "lpszAgent is NULL"]);
    inet_stops("inet-type-proxy", "cN(fle_imp_InternetOpenW, A(agent), 3, A(agent), 0, 0);", &["InternetOpenW", "dwAccessType"]);
    inet_stops("inet-proxy-string", "cN(fle_imp_InternetOpenW, A(agent), 0, A(agent), 0, 0);", &["InternetOpenW", "lpszProxy"]);
    inet_stops("inet-async", "cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0x10000000);", &["InternetOpenW", "dwFlags"]);
    inet_stops("inet-option-other", "uint32_t t = 1; uint64_t h = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0); cN(fle_imp_InternetSetOptionW, h, 3, A(&t), 4, 0);", &["InternetSetOptionW", "time-out options"]);
    inet_stops("inet-option-size", "uint32_t t = 1; uint64_t h = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0); cN(fle_imp_InternetSetOptionW, h, 6, A(&t), 2, 0);", &["InternetSetOptionW", "4-byte"]);
    inet_stops("inet-setoption-bad-handle", "uint32_t t = 1; cN(fle_imp_InternetSetOptionW, 0x1234, 6, A(&t), 4, 0);", &["InternetSetOptionW", "live session"]);
    inet_stops("inet-url-null", "uint64_t h = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0); cN(fle_imp_InternetOpenUrlW, h, 0, 0, 0, 0);", &["InternetOpenUrlW", "lpszUrl is NULL"]);
    inet_stops("inet-url-bad-handle", "cN(fle_imp_InternetOpenUrlW, 0, A(url), 0, 0, 0);", &["InternetOpenUrlW", "live session"]);
    inet_stops("inet-close-twice", "uint64_t h = cN(fle_imp_InternetOpenW, A(agent), 0, 0, 0, 0); cN(fle_imp_InternetCloseHandle, h, 0, 0, 0, 0); cN(fle_imp_InternetCloseHandle, h, 0, 0, 0, 0);", &["InternetCloseHandle", "live session"]);
}

#[test]
fn format_message_w_reads_the_message_table_of_a_virtual_module_with_the_learn_text_and_falls_back_to_the_system_table() {
    // decision 0132 (wall 24, PROVISIONAL). The guest: FormatMessageW(0x1a00 = FROM_SYSTEM | FROM_HMODULE | IGNORE_INSERTS, hModule of wininet.dll, 12007, 0, buf, 511, NULL) after the failed InternetOpenUrlW. Learn
    // (FormatMessageW): FROM_HMODULE "The lpSource parameter is a module handle containing the message-table resource(s) to search"; with FROM_SYSTEM too, "the function searches the system message table if the message is not found in the
    // module specified by lpSource". The text of 12007 is the description of Learn's Appendix: WinINet Error Codes (S-errmsg-wininet-learn-text: a reading, not a measurement). The module list of the driver is a stub with the
    // numbers of fle_modules.h (handle = 0x7FF800000000 + 0x10000 * (index + 1)): kernel32.dll is index 0, wininet.dll index 20, as in the table of fle_modules.c.
    let o = inet_run(
        "fmt-module",
        r#"
    uint16_t b[600];
    uint64_t n, wininet = 0x7FF800000000ull + 0x10000ull * 21, k32 = 0x7FF800000000ull + 0x10000ull * 1;
    unsigned i;
    memset(b, 0xAA, sizeof b);
    n = cN7(fle_imp_FormatMessageW, 0x1a00, wininet, 12007, 0, A(b), 511, 0);
    printf("12007: %llu [", U(n)); for (i = 0; b[i]; i++) putchar((int)b[i]); printf("] units %u\n", i);
    n = cN7(fle_imp_FormatMessageW, 0x1a00, wininet, 5, 0, A(b), 511, 0);
    printf("5 falls back to the system table: %llu [", U(n)); for (i = 0; b[i]; i++) putchar((int)b[i]); printf("]\n");
    fle_set_last_error(0);
    n = cN7(fle_imp_FormatMessageW, 0x0a00, wininet, 5, 0, A(b), 511, 0);
    printf("5 from the module only: %llu le %u\n", U(n), LE());
    n = cN7(fle_imp_FormatMessageW, 0x0a00, wininet, 12007, 0, A(b), 10, 0);
    printf("a buffer of 10: %llu le %u\n", U(n), LE());
    n = cN7(fle_imp_FormatMessageW, 0x1200, 0, 5, 0, A(b), 511, 0);
    printf("system only, as before: %llu\n", U(n));
"#,
    );
    assert!(o.ok, "{}\n{}", o.stderr, o.stdout);
    let l: Vec<&str> = o.stdout.lines().collect();
    assert_eq!(l[0], "12007: 38 [The server name could not be resolved.] units 38");
    assert_eq!(l[1], "5 falls back to the system table: 17 [Access is denied.]");
    assert_eq!(l[2], "5 from the module only: 0 le 317");
    assert_eq!(l[3], "a buffer of 10: 0 le 122");
    assert_eq!(l[4], "system only, as before: 17");
    inet_stops("fmt-module-null", "cN7(fle_imp_FormatMessageW, 0x1a00, 0, 12007, 0, A(stk), 511, 0);", &["FormatMessageW", "lpSource is not the handle of a virtual module"]);
    inet_stops("fmt-module-kernel32", "cN7(fle_imp_FormatMessageW, 0x1a00, 0x7FF800000000ull + 0x10000ull, 12007, 0, A(stk), 511, 0);", &["FormatMessageW", "message table of this module"]);
    inet_stops("fmt-module-no-source", "cN7(fle_imp_FormatMessageW, 0x0200, 0, 5, 0, A(stk), 511, 0);", &["FormatMessageW", "neither FORMAT_MESSAGE_FROM_SYSTEM nor FORMAT_MESSAGE_FROM_HMODULE"]);
}
