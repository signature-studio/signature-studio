//! Tests for kernel32!GetThreadPreferredUILanguages (docs/decisions/0022-thread-preferred-ui-languages.md),
//! runtime/fle_uipref.c. Generator: thunk and response-table row; SetThreadPreferredUILanguages stays unanswered.
//! Runtime: the lifted caller at 0x4193c4 (flags 0x38, size query then fetch), format and filter flags, buffer
//! rules, last error, and the undefined cases that stop the run. Linux host; nothing is compared with Windows.
//! The language "en-US" is the setting of this lift environment (decision 0016), not a value from the MSDN page.

mod etxt_common;

use fe_lift::generate;
use std::path::PathBuf;
use std::process::Command;

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

/// A PE with kernel32.dll imports `names` (at most 3); entry 0x1000 calls the thunks at 0x1040, 0x1050, 0x1060.
fn build(names: &[&str]) -> Vec<u8> {
    assert!(names.len() <= 3);
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
        let name_rva = 0x2400 + 0x40 * k as u32;
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
        code.push((at, [&[0xe8][..], &rel(at + 5, 0x1040 + 0x10 * k as u32)].concat()));
        code.push((0x1040 + 0x10 * k as u32, [&[0xff, 0x25][..], &rel(0x1046 + 0x10 * k as u32, 0x2200 + 8 * k as u32)].concat()));
    }
    code.push((0x1000 + 5 * names.len() as u32, vec![0xc3]));
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn dirs() -> (PathBuf, PathBuf) {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-null0/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (rt, dir)
}

/// Every runtime source in runtime/ (fle_*.c), sorted: the response table can name any `fle_imp_*` function.
fn runtime_sources(rt: &std::path::Path) -> Vec<PathBuf> {
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

fn gen(names: &[&str]) -> (fe_lift::Stats, String) {
    let img = build(names);
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    (st, String::from_utf8(out).unwrap())
}

// ---- 1. Generator ----

#[test]
fn thunk_calls_runtime() {
    let (st, c) = gen(&["GetThreadPreferredUILanguages"]);
    assert_eq!(st.handled["jmp_imp:GetThreadPreferredUILanguages"], 1);
    assert!(c.contains("fle_imp_GetThreadPreferredUILanguages(); return;"));
    assert!(!c.contains("fle_resp_table"));
}

#[test]
fn response_table_has_a_row_for_kernel32_get_thread_preferred_ui_languages() {
    let (_, c) = gen(&["GetProcAddress"]);
    let row = c
        .lines()
        .find(|l| l.contains("\"GetThreadPreferredUILanguages\""))
        .expect("no GetThreadPreferredUILanguages row in fle_resp_table");
    assert!(row.contains("\"kernel32.dll\""), "{row}");
    assert!(row.contains("fle_imp_GetThreadPreferredUILanguages }"), "{row}");
    let va = row.split("UINT64_C(").nth(1).unwrap().split(')').next().unwrap();
    assert!(c.contains(&format!("{{ UINT64_C({va}), fle_imp_GetThreadPreferredUILanguages }}")), "{va}");
}

#[test]
fn set_thread_preferred_ui_languages_is_still_not_answered() {
    // Decision 0022: only the Get function is answered. The Set function has no row (its result and state
    // are not decided), so GetProcAddress still returns NULL for it.
    let (_, c) = gen(&["GetProcAddress"]);
    assert!(!c.contains("\"SetThreadPreferredUILanguages\""));
}

// ---- 2. Runtime ----

const MAIN: &str = r#"#include <stdio.h>
#include <string.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_uipref.h"

static uint64_t gpa(uint64_t module, const char *name)
{
    R[1] = module; R[2] = (uint64_t)(uintptr_t)name;
    fle_imp_GetProcAddress();
    return R[0];
}

static uint32_t last_error(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }

/* Calls through fle_dispatch when va != 0, as the lifted code does. */
static uint64_t call(uint64_t va, uint32_t flags, uint32_t *num, uint16_t *buf, uint32_t *cch)
{
    R[1] = flags; R[2] = (uint64_t)(uintptr_t)num; R[8] = (uint64_t)(uintptr_t)buf; R[9] = (uint64_t)(uintptr_t)cch;
    if (va) fle_dispatch(va); else fle_imp_GetThreadPreferredUILanguages();
    return R[0] & 0xFFFFFFFFu;
}

/* prints the multi-string as name1|name2|| */
static void show(const char *tag, const uint16_t *b, uint32_t n)
{
    uint32_t i;
    printf("%s ", tag);
    for (i = 0; i < n; i++) putchar(b[i] ? (char)b[i] : '|');
    putchar('\n');
}

static void query(uint64_t va, const char *tag, uint32_t flags)
{
    uint32_t num = 99, cch = 0;
    uint16_t buf[64];
    uint64_t ok = call(va, flags, &num, NULL, &cch);
    printf("%s size ok=%llu num=%u cch=%u\n", tag, (unsigned long long)ok, num, cch);
    memset(buf, 0xAA, sizeof buf);
    num = 99;
    ok = call(va, flags, &num, buf, &cch);
    printf("%s get ok=%llu num=%u cch=%u\n", tag, (unsigned long long)ok, num, cch);
    show(tag, buf, cch);
    printf("%s after-end %04x\n", tag, buf[cch]);
}

int main(int argc, char **argv)
{
    static const uint16_t k[] = { 'k','e','r','n','e','l','3','2','.','d','l','l', 0 };
    uint64_t va, kern;
    uint32_t num, cch;
    uint16_t buf[64];

    if (argc > 1) {
        num = 0; cch = 0;
        if (!strcmp(argv[1], "stop-thread")) call(0, 0x40, &num, NULL, &cch);
        if (!strcmp(argv[1], "stop-both")) call(0, 0x4 | 0x8, &num, NULL, &cch);
        if (!strcmp(argv[1], "stop-sys-only")) call(0, 0x10, &num, NULL, &cch);
        if (!strcmp(argv[1], "stop-null-num")) call(0, 0x38, NULL, NULL, &cch);
        if (!strcmp(argv[1], "stop-null-cch")) call(0, 0x38, &num, NULL, NULL);
        if (!strcmp(argv[1], "stop-bit")) call(0, 0x1000, &num, NULL, &cch);
        puts("returned");
        return 0;
    }

    R[1] = (uint64_t)(uintptr_t)k; fle_imp_GetModuleHandleW(); kern = R[0];
    va = gpa(kern, "GetThreadPreferredUILanguages");
    printf("found %d\n", va != 0);

    /* the lifted code at 0x4193c4: MUI_UI_FALLBACK | MUI_LANGUAGE_NAME */
    query(va, "ui-name", 0x38);
    query(va, "user-name", 0x28);
    query(va, "default", 0);
    query(va, "ui-id", 0x34);
    query(va, "user-id", 0x24);

    /* errors: too small, or NULL with a size; outputs unchanged; last error 122 */
    fle_set_last_error(0);
    num = 77; cch = 3; memset(buf, 0xAA, sizeof buf);
    printf("small ok=%llu num=%u cch=%u buf0=%04x err=%u\n", (unsigned long long)call(va, 0x38, &num, buf, &cch), num, cch, buf[0], last_error());
    fle_set_last_error(0);
    num = 77; cch = 5;
    printf("nullbuf ok=%llu num=%u cch=%u err=%u\n", (unsigned long long)call(va, 0x38, &num, NULL, &cch), num, cch, last_error());
    /* exact size works: 10 for "en-US","en" */
    fle_set_last_error(0);
    num = 77; cch = 10;
    printf("exact ok=%llu num=%u cch=%u err=%u\n", (unsigned long long)call(va, 0x38, &num, buf, &cch), num, cch, last_error());
    num = 77; cch = 9; fle_set_last_error(0);
    printf("one-less ok=%llu cch=%u err=%u\n", (unsigned long long)call(va, 0x38, &num, buf, &cch), cch, last_error());

    /* success leaves the last error alone */
    fle_set_last_error(77);
    num = 0; cch = 0;
    call(va, 0x38, &num, NULL, &cch);
    printf("lasterror %u\n", last_error());

    /* exact spelling, module */
    fle_set_last_error(0);
    printf("lowercase %llu\n", (unsigned long long)gpa(kern, "getthreadpreferreduilanguages"));
    printf("user32 %llu\n", (unsigned long long)gpa(fle_module_handle(18), "GetThreadPreferredUILanguages"));
    printf("set %llu\n", (unsigned long long)gpa(kern, "SetThreadPreferredUILanguages"));
    return 0;
}
"#;

fn build_exe(dir: &std::path::Path, rt: &std::path::Path) -> PathBuf {
    let (_, c) = gen(&["GetModuleHandleW", "GetProcAddress"]);
    let tag = etxt_common::uniq("null0");
    let src = dir.join(format!("{tag}-unit.c"));
    std::fs::write(&src, &c).unwrap();
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, MAIN).unwrap();
    let exe = dir.join(format!("{tag}-run"));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(rt);
    for p in runtime_sources(rt) {
        cmd.arg(p);
    }
    let cc = cmd
        .arg(&src)
        .arg(&mainc)
        .arg("-lm")
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

#[test]
fn runtime_answers() {
    let (rt, dir) = dirs();
    let exe = build_exe(&dir, &rt);
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "found 1",
            // 0x38: MUI_LANGUAGE_NAME | MUI_UI_FALLBACK: language and its neutral form, two NULs at the end
            "ui-name size ok=1 num=2 cch=10",
            "ui-name get ok=1 num=2 cch=10",
            "ui-name en-US|en||",
            "ui-name after-end aaaa",
            // 0x28: MUI_LANGUAGE_NAME | MUI_MERGE_USER_FALLBACK: the language only
            "user-name size ok=1 num=1 cch=7",
            "user-name get ok=1 num=1 cch=7",
            "user-name en-US||",
            "user-name after-end aaaa",
            // 0: default format NAME, default filter MUI_MERGE_USER_FALLBACK
            "default size ok=1 num=1 cch=7",
            "default get ok=1 num=1 cch=7",
            "default en-US||",
            "default after-end aaaa",
            // 0x34: MUI_LANGUAGE_ID | MUI_UI_FALLBACK
            "ui-id size ok=1 num=2 cch=11",
            "ui-id get ok=1 num=2 cch=11",
            "ui-id 0409|0009||",
            "ui-id after-end aaaa",
            "user-id size ok=1 num=1 cch=6",
            "user-id get ok=1 num=1 cch=6",
            "user-id 0409||",
            "user-id after-end aaaa",
            "small ok=0 num=77 cch=3 buf0=aaaa err=122",
            "nullbuf ok=0 num=77 cch=5 err=122",
            "exact ok=1 num=2 cch=10 err=0",
            "one-less ok=0 cch=9 err=122",
            "lasterror 77",
            "lowercase 0",
            "user32 0",
            "set 0",
        ]
    );
}

#[test]
fn undefined_cases_stop_the_run() {
    let (rt, dir) = dirs();
    let exe = build_exe(&dir, &rt);
    for case in ["stop-thread", "stop-both", "stop-sys-only", "stop-null-num", "stop-null-cch", "stop-bit"] {
        let run = Command::new(&exe).arg(case).output().expect("run failed");
        let err = String::from_utf8_lossy(&run.stderr).to_string();
        let out = String::from_utf8_lossy(&run.stdout).to_string();
        assert!(!run.status.success(), "{case}: did not stop: {out}");
        assert!(!out.contains("returned"), "{case}: {out}");
        assert!(err.contains("fle: GetThreadPreferredUILanguages"), "{case}: {err}");
    }
}

// ---- 3. Source ----

#[test]
fn source_holds_no_locale_value_of_its_own() {
    let (rt, _) = dirs();
    let c = std::fs::read_to_string(rt.join("fle_uipref.c")).unwrap();
    let mut code = String::new();
    let mut rest = c.as_str();
    while let Some(i) = rest.find("/*") {
        code.push_str(&rest[..i]);
        rest = &rest[i + 2..];
        rest = &rest[rest.find("*/").unwrap() + 2..];
    }
    code.push_str(rest);
    assert!(code.contains("fle_emulated_locale.lcid"), "{code}");
    for lit in ["0x0409", "0x409", "1033", "\"en", "0409"] {
        assert!(!code.contains(lit), "{lit} in fle_uipref.c code");
    }
}
