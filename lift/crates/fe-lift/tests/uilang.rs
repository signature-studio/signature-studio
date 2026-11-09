//! Tests for kernel32!GetThreadUILanguage (docs/decisions/0021-uilang.md), runtime/fle_uilang.c.
//!
//! 1. Generator: the thunk calls the runtime, and the response table has a row that GetProcAddress can find.
//! 2. Runtime: the answer is the user/system UI language of the emulated locale en-US (0x0409), it does not
//!    depend on register state, and it does not touch the last error. Reached through
//!    GetModuleHandleW -> GetProcAddress -> fle_dispatch, as the lifted code at 0x418ea7/0x419419 does.
//! 3. GetProcAddress keeps exact spelling and module matching for the new row.
//! 4. The source holds no locale value of its own (the value lives in fle_locale.c only).
//!
//! The host is Linux; Windows cannot run here, so nothing is compared with a real Windows. The value 0x0409 is
//! the setting of this lift environment (decision 0016), not a value given by the MSDN page.

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
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-uilang/test");
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
    let (st, c) = gen(&["GetThreadUILanguage"]);
    assert_eq!(st.handled["jmp_imp:GetThreadUILanguage"], 1);
    assert!(c.contains("fle_imp_GetThreadUILanguage(); return;"));
    // No GetProcAddress in the closure: no response table.
    assert!(!c.contains("fle_resp_table"));
}

#[test]
fn response_table_has_a_row_for_kernel32_get_thread_ui_language() {
    let (_, c) = gen(&["GetProcAddress"]);
    let row = c
        .lines()
        .find(|l| l.contains("\"GetThreadUILanguage\""))
        .expect("no GetThreadUILanguage row in fle_resp_table");
    assert!(row.contains("\"kernel32.dll\""), "{row}");
    assert!(row.contains("fle_imp_GetThreadUILanguage }"), "{row}");
    // The same pseudo-VA is a dispatch entry.
    let va = row.split("UINT64_C(").nth(1).unwrap().split(')').next().unwrap();
    assert!(c.contains(&format!("{{ UINT64_C({va}), fle_imp_GetThreadUILanguage }}")), "{va}");
}

// ---- 2. Runtime ----

const MAIN: &str = r#"#include <stdio.h>
#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_modules.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_uilang.h"

static uint64_t gpa(uint64_t module, const char *name)
{
    R[1] = module; R[2] = (uint64_t)(uintptr_t)name;
    fle_imp_GetProcAddress();
    return R[0];
}

int main(void)
{
    static const uint16_t k[] = { 'k','e','r','n','e','l','3','2','.','d','l','l', 0 };
    uint64_t va, kern, i;

    /* direct call: any register state, last error untouched */
    fle_set_last_error(77);
    for (i = 0; i < 16; i++) R[i] = 0xdeadbeefdeadbeefull;
    R[4] = 0;
    fle_imp_GetThreadUILanguage();
    printf("direct %llx\n", (unsigned long long)R[0]);
    fle_imp_GetLastError();
    printf("lasterror %llu\n", (unsigned long long)R[0]);

    /* same value as the other UI language answers */
    fle_imp_GetUserDefaultUILanguage(); printf("user %llx\n", (unsigned long long)R[0]);
    fle_imp_GetSystemDefaultUILanguage(); printf("system %llx\n", (unsigned long long)R[0]);
    printf("emulated %x\n", (unsigned)(fle_emulated_locale.lcid & 0xFFFFu));

    /* GetModuleHandleW -> GetProcAddress -> fle_dispatch */
    R[1] = (uint64_t)(uintptr_t)k; fle_imp_GetModuleHandleW(); kern = R[0];
    va = gpa(kern, "GetThreadUILanguage");
    printf("found %d\n", va != 0);
    R[0] = 0x1234;
    fle_dispatch(va);
    printf("dispatched %llx\n", (unsigned long long)R[0]);

    /* exact spelling, and the module must be kernel32 (index 8 of the virtual module table is kernel32.dll;
       index 18 is user32.dll) */
    fle_set_last_error(0);
    printf("lowercase %llu\n", (unsigned long long)gpa(kern, "getthreaduilanguage"));
    fle_imp_GetLastError(); printf("lowercase-err %llu\n", (unsigned long long)R[0]);
    printf("suffixed %llu\n", (unsigned long long)gpa(kern, "GetThreadUILanguageX"));
    fle_set_last_error(0);
    printf("user32 %llu\n", (unsigned long long)gpa(fle_module_handle(18), "GetThreadUILanguage"));
    fle_imp_GetLastError(); printf("user32-err %llu\n", (unsigned long long)R[0]);
    return 0;
}
"#;

#[test]
fn runtime_answer_and_lookup() {
    let (_, c) = gen(&["GetModuleHandleW", "GetProcAddress"]);
    let (rt, dir) = dirs();
    let tag = etxt_common::uniq("uilang");
    let src = dir.join(format!("{tag}-unit.c"));
    std::fs::write(&src, &c).unwrap();
    let mainc = dir.join(format!("{tag}-main.c"));
    std::fs::write(&mainc, MAIN).unwrap();
    let exe = dir.join(format!("{tag}-run"));
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fno-builtin", "-fPIE", "-I"]).arg(&rt);
    for p in runtime_sources(&rt) {
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
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "{out}\n{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "direct 409",
            "lasterror 77",
            "user 409",
            "system 409",
            "emulated 409",
            "found 1",
            "dispatched 409",
            "lowercase 0",
            "lowercase-err 127",
            "suffixed 0",
            "user32 0",
            "user32-err 127",
        ]
    );
}

// ---- 4. Source ----

#[test]
fn source_holds_no_locale_value_of_its_own() {
    let (rt, _) = dirs();
    let c = std::fs::read_to_string(rt.join("fle_uilang.c")).unwrap();
    let code: String = {
        // strip /* ... */ comments
        let mut s = String::new();
        let mut rest = c.as_str();
        while let Some(i) = rest.find("/*") {
            s.push_str(&rest[..i]);
            rest = &rest[i + 2..];
            rest = &rest[rest.find("*/").unwrap() + 2..];
        }
        s.push_str(rest);
        s
    };
    assert!(code.contains("fle_emulated_locale.lcid"), "{code}");
    for lit in ["0x0409", "0x409", "1033"] {
        assert!(!code.contains(lit), "{lit} in fle_uilang.c code");
    }
}
