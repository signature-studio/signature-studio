//! Tests for the emulated OS (docs/decisions/0014-emulated-os.md): kernel32!VerifyVersionInfoW,
//! GetVersion and GetVersionExW in runtime/fle_osver.c.
//!
//! 1. The three thunks call the runtime (table in src/imports_table.rs).
//! 2. The runtime against the emulated OS 10.0.19045 (workstation, SP 0.0, suite 0). Expected values are
//!    written as Rust expressions from the MSDN rules, independent of the C code: a plain comparison for
//!    build, platform, suite and product type; for major, minor, SP major, SP minor, the first member that
//!    differs decides (MSDN: VerifyVersionInfoW, Remarks). The condition mask is built the way
//!    VerSetConditionMask does (3 bits at 3 * k, decision 0011).
//! 3. Cases the MSDN pages leave open stop the run (abort); the C program is run and its exit is checked.
//!
//! The host is Linux, so the Windows APIs cannot run here; there is no comparison with a real Windows.

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

/// A PE with kernel32.dll!{VerifyVersionInfoW, GetVersion, GetVersionExW}; entry 0x1000 calls the three thunks.
fn build() -> Vec<u8> {
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
    put(&mut b, 0x2400, b"\0\0VerifyVersionInfoW\0");
    put(&mut b, 0x2430, b"\0\0GetVersion\0");
    put(&mut b, 0x2440, b"\0\0GetVersionExW\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2108, 0x2430), (0x2110, 0x2440), (0x2200, 0x2400), (0x2208, 0x2430), (0x2210, 0x2440)] {
        w64(&mut b, data_at(rva), v);
    }

    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1050)].concat()),
        (0x100a, [&[0xe8][..], &rel(0x100f, 0x1060)].concat()),
        (0x100f, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
        (0x1050, [&[0xff, 0x25][..], &rel(0x1056, 0x2208)].concat()),
        (0x1060, [&[0xff, 0x25][..], &rel(0x1066, 0x2210)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn thunks_call_runtime() {
    let img = build();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    for (name, va) in [("VerifyVersionInfoW", IB + 0x1040), ("GetVersion", IB + 0x1050), ("GetVersionExW", IB + 0x1060)] {
        assert_eq!(st.handled[format!("jmp_imp:{name}").as_str()], 1, "{name}");
        assert!(c.contains(&format!("fle_imp_{name}(); return;")), "{name}");
        assert!(!c.contains(&format!("L_{va:x}: {{ /* {va:#x} Jmp */ fle_unsupported")), "{name}");
    }
}

// ---- Expected values: the spec rules as Rust expressions (independent of the C code) ----

const EMU: Os = Os { major: 10, minor: 0, build: 19045, platform: 2, spmaj: 0, spmin: 0, suite: 0, prod: 1 };

#[derive(Clone, Copy)]
struct Os {
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    spmaj: u32,
    spmin: u32,
    suite: u32,
    prod: u32,
}

const V_MINOR: u32 = 0x01;
const V_MAJOR: u32 = 0x02;
const V_BUILD: u32 = 0x04;
const V_PLATFORM: u32 = 0x08;
const V_SPMIN: u32 = 0x10;
const V_SPMAJ: u32 = 0x20;
const V_SUITE: u32 = 0x40;
const V_PROD: u32 = 0x80;

fn cmp(cur: u32, req: u32, c: u32) -> bool {
    match c {
        1 => cur == req,
        2 => cur > req,
        3 => cur >= req,
        4 => cur < req,
        5 => cur <= req,
        _ => panic!("condition {c}"),
    }
}

/// Condition mask as VerSetConditionMask builds it: 3 bits at 3 * (bit index of the flag).
fn mask(items: &[(u32, u32)]) -> u64 {
    items.iter().fold(0u64, |m, &(flag, c)| m | (c as u64) << (3 * flag.trailing_zeros()))
}

/// The spec's expected result. `req` is the OSVERSIONINFOEXW content, `items` are (flag, condition).
/// Hierarchy (major, minor, SP major, SP minor): walk in that order over the tested members; the first
/// member whose values differ decides (its condition must hold); while equal, the condition must hold on
/// equality and the walk goes on.
fn expect(os: Os, req: Os, items: &[(u32, u32)]) -> bool {
    let cond = |flag: u32| items.iter().find(|i| i.0 == flag).map(|i| i.1);
    let tm: u32 = items.iter().fold(0, |a, i| a | i.0);
    let mut ok = true;
    if tm & V_PROD != 0 {
        ok &= cmp(os.prod, req.prod, cond(V_PROD).unwrap());
    }
    if tm & V_SUITE != 0 {
        let want = req.suite;
        ok &= match cond(V_SUITE).unwrap() {
            6 => want & os.suite == want,
            7 => want & os.suite != 0,
            c => panic!("suite condition {c}"),
        };
    }
    if tm & V_PLATFORM != 0 {
        ok &= cmp(os.platform, req.platform, cond(V_PLATFORM).unwrap());
    }
    if tm & V_BUILD != 0 {
        ok &= cmp(os.build, req.build, cond(V_BUILD).unwrap());
    }
    let chain = [(V_MAJOR, os.major, req.major), (V_MINOR, os.minor, req.minor), (V_SPMAJ, os.spmaj, req.spmaj), (V_SPMIN, os.spmin, req.spmin)];
    for (flag, cur, want) in chain {
        if tm & flag == 0 {
            continue;
        }
        let c = cond(flag).unwrap();
        if cur != want {
            ok &= cmp(cur, want, c);
            break;
        }
        ok &= cmp(cur, want, c);
    }
    ok
}

/// One row of the C test: required OS content, TypeMask, condition mask, expected result.
struct Case {
    name: String,
    req: Os,
    type_mask: u32,
    cmask: u64,
    want: bool,
}

fn case(name: &str, req: Os, items: &[(u32, u32)]) -> Case {
    Case {
        name: name.to_string(),
        req,
        type_mask: items.iter().fold(0, |a, i| a | i.0),
        cmask: mask(items),
        want: expect(EMU, req, items),
    }
}

fn base() -> Os {
    Os { major: 0, minor: 0, build: 0, platform: 0, spmaj: 0, spmin: 0, suite: 0, prod: 0 }
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    // Plain statements of the task: "major >= 9" holds, "major >= 11" does not.
    for (m, c) in [(9u32, 3u32), (10, 3), (11, 3), (10, 1), (9, 1), (9, 2), (10, 2), (11, 4), (10, 4), (10, 5), (11, 5), (9, 5), (9, 4)] {
        let r = Os { major: m, ..base() };
        v.push(case(&format!("major {c} {m}"), r, &[(V_MAJOR, c)]));
    }
    // Build number, platform id, product type, suite.
    for (b, c) in [(19044u32, 3u32), (19045, 3), (19046, 3), (19045, 1), (19044, 1), (19045, 2), (19044, 2), (19046, 4), (19045, 4), (19045, 5), (19046, 5), (19044, 5)] {
        let r = Os { build: b, ..base() };
        v.push(case(&format!("build {c} {b}"), r, &[(V_BUILD, c)]));
    }
    for (p, c) in [(2u32, 1u32), (1, 1), (2, 3), (3, 3), (1, 2)] {
        v.push(case(&format!("platform {c} {p}"), Os { platform: p, ..base() }, &[(V_PLATFORM, c)]));
    }
    for (p, c) in [(1u32, 1u32), (2, 1), (3, 1), (1, 3), (1, 4)] {
        v.push(case(&format!("product {c} {p}"), Os { prod: p, ..base() }, &[(V_PROD, c)]));
    }
    for (s, c) in [(0x1u32, 6u32), (0x1, 7), (0x300, 6), (0x300, 7)] {
        v.push(case(&format!("suite {c} {s:#x}"), Os { suite: s, ..base() }, &[(V_SUITE, c)]));
    }
    // Hierarchy: major, minor, SP major, SP minor (all >=). Real Windows rules from the MSDN examples.
    for (ma, mi, spa, spi) in [(10u32, 0u32, 0u32, 0u32), (10, 0, 1, 0), (10, 1, 0, 0), (9, 5, 9, 9), (11, 0, 0, 0), (10, 0, 0, 1), (6, 1, 1, 0), (5, 1, 1, 0)] {
        let r = Os { major: ma, minor: mi, spmaj: spa, spmin: spi, ..base() };
        v.push(case(&format!("hier>= {ma}.{mi} sp{spa}.{spi}"), r, &[(V_MAJOR, 3), (V_MINOR, 3), (V_SPMAJ, 3), (V_SPMIN, 3)]));
    }
    // Hierarchy with equal on the higher members and a different condition below.
    for (ma, mi, c) in [(10u32, 0u32, 1u32), (10, 1, 5), (10, 0, 5), (9, 7, 5), (10, 0, 4), (10, 1, 4), (11, 0, 1), (10, 0, 2)] {
        let r = Os { major: ma, minor: mi, ..base() };
        v.push(case(&format!("hier maj==,min{c} {ma}.{mi}"), r, &[(V_MAJOR, 1), (V_MINOR, c), (V_SPMAJ, c)]));
    }
    // f_409b30: TypeMask 2, 1, 0x20, all VER_GREATER_EQUAL; dwTypeMask 0x23; SP minor not tested.
    for (ma, mi, sp) in [(6u32, 1u32, 1u32), (10, 0, 0), (10, 0, 1), (10, 1, 0), (11, 0, 0), (6, 3, 9), (5, 1, 1)] {
        let r = Os { major: ma, minor: mi, spmaj: sp, ..base() };
        v.push(case(&format!("f_409b30 {ma}.{mi} sp{sp}"), r, &[(V_MAJOR, 3), (V_MINOR, 3), (V_SPMAJ, 3)]));
    }
    // Mixed: members outside the hierarchy together with it.
    v.push(case("mixed major+build+prod", Os { major: 10, build: 19045, prod: 1, ..base() }, &[(V_MAJOR, 3), (V_BUILD, 3), (V_PROD, 1)]));
    v.push(case("mixed major ok, build fails", Os { major: 10, build: 19046, prod: 1, ..base() }, &[(V_MAJOR, 3), (V_BUILD, 3), (V_PROD, 1)]));
    v.push(case("mixed major ok, prod fails", Os { major: 10, build: 19045, prod: 3, ..base() }, &[(V_MAJOR, 3), (V_BUILD, 3), (V_PROD, 1)]));
    v
}

fn c_str(cs: &[Case]) -> String {
    let mut s = String::new();
    s.push_str("#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_osver.h\"\n\nuint64_t R[16];\n\n");
    s.push_str("static void mk(uint8_t *b, uint32_t size, uint32_t ma, uint32_t mi, uint32_t bu, uint32_t pl, uint32_t spa, uint32_t spi, uint32_t su, uint32_t pr)\n{\n");
    s.push_str("    memset(b, 0, 284);\n    memcpy(b, &size, 4); memcpy(b + 4, &ma, 4); memcpy(b + 8, &mi, 4); memcpy(b + 12, &bu, 4); memcpy(b + 16, &pl, 4);\n");
    s.push_str("    { uint16_t x = (uint16_t)spa; memcpy(b + 276, &x, 2); x = (uint16_t)spi; memcpy(b + 278, &x, 2); x = (uint16_t)su; memcpy(b + 280, &x, 2); }\n    b[282] = (uint8_t)pr;\n}\n\n");
    s.push_str("int main(void)\n{\n    uint8_t b[284];\n    int bad = 0;\n");
    for c in cs {
        s.push_str(&format!(
            "    mk(b, 284, {}, {}, {}, {}, {}, {}, {}, {});\n    R[1] = (uint64_t)(uintptr_t)b; R[2] = {}; R[8] = UINT64_C({}); R[0] = 99; fle_set_last_error(0); fle_imp_VerifyVersionInfoW();\n    if (R[0] != {} || fle_osver_check(&fle_emulated_os, b, {}, UINT64_C({})) != {}) {{ printf(\"FAIL {}\\n\"); bad++; }}\n    {{ uint64_t le; fle_imp_GetLastError(); le = R[0]; if (le != {}) {{ printf(\"FAIL lasterror {}\\n\"); bad++; }} }}\n",
            c.req.major, c.req.minor, c.req.build, c.req.platform, c.req.spmaj, c.req.spmin, c.req.suite, c.req.prod,
            c.type_mask, c.cmask,
            c.want as u32, c.type_mask, c.cmask, c.want as u32, c.name,
            if c.want { 0 } else { 1150 }, c.name
        ));
    }
    s.push_str(&format!("    printf(\"cases {} bad %d\\n\", bad);\n    return bad != 0;\n}}\n", cs.len()));
    s
}

fn dirs() -> (PathBuf, PathBuf) {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-osver/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (rt, dir)
}

fn build_c(name: &str, src: &str) -> (PathBuf, PathBuf) {
    let (rt, dir) = dirs();
    let c = dir.join(format!("{name}.c"));
    let exe = dir.join(name);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_osver.c"))
        .arg(rt.join("fle_imports.c"))
        .arg(rt.join("fle_string.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    (exe, dir)
}

#[test]
fn verify_version_info_matches_spec_rules_on_emulated_os() {
    let cs = cases();
    let n = cs.len();
    assert!(n >= 60, "{n}");
    // The expected results must contain both outcomes, and the two named examples.
    assert!(cs.iter().any(|c| c.want) && cs.iter().any(|c| !c.want));
    let find = |nm: &str| cs.iter().find(|c| c.name == nm).unwrap().want;
    assert!(find("major 3 9"));
    assert!(!find("major 3 11"));
    assert!(find("f_409b30 6.1 sp1"));
    assert!(find("f_409b30 10.0 sp0"));
    assert!(!find("f_409b30 11.0 sp0"));
    assert!(find("f_409b30 6.3 sp9"));

    let (exe, _) = build_c("verify-cases", &c_str(&cs));
    let run = Command::new(&exe).output().expect("run failed");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "C test failed:\n{out}");
    assert!(out.contains(&format!("cases {n} bad 0")), "{out}");
}

/// The spec's own examples on the hierarchy (MSDN: VerifyVersionInfoW, Remarks), against synthetic OS values:
/// ">= 5.1 SP1" succeeds on 6.0 and on 5.2, fails on 5.0 SP2. Checked through fle_osver_check directly.
#[test]
fn hierarchy_examples_from_remarks() {
    let src = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_osver.h\"\n\nuint64_t R[16];\n\
static int run(uint32_t ma, uint32_t mi, uint32_t spa)\n{\n\
    fle_osver_t os = { ma, mi, 1, 2, (uint16_t)spa, 0, 0, 1 };\n\
    uint8_t b[284] = { 0 };\n\
    uint32_t sz = 284, rma = 5, rmi = 1; uint16_t rspa = 1;\n\
    uint64_t m = (3u << 3) | 3u | ((uint64_t)3 << 15);\n\
    memcpy(b, &sz, 4); memcpy(b + 4, &rma, 4); memcpy(b + 8, &rmi, 4); memcpy(b + 276, &rspa, 2);\n\
    return fle_osver_check(&os, b, 0x23, m);\n}\n\
int main(void)\n{\n\
    printf(\"%d %d %d %d %d\\n\", run(6, 0, 0), run(5, 2, 0), run(5, 0, 2), run(5, 1, 1), run(5, 1, 0));\n\
    return 0;\n}\n";
    let (exe, _) = build_c("hier-examples", src);
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success());
    // 6.0 -> success, 5.2 -> success, 5.0 SP2 -> fail, 5.1 SP1 -> success, 5.1 SP0 -> fail
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "1 1 0 1 0");
}

/// GetVersion and GetVersionExW on the emulated OS.
#[test]
fn get_version_and_get_version_ex() {
    let src = "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_osver.h\"\n\nuint64_t R[16];\n\
int main(void)\n{\n\
    uint8_t b[284]; uint32_t v; uint16_t w; uint32_t sz;\n\
    fle_imp_GetVersion(); printf(\"gv %llx\\n\", (unsigned long long)R[0]);\n\
    memset(b, 0xAB, sizeof b); sz = 276; memcpy(b, &sz, 4); R[1] = (uint64_t)(uintptr_t)b; R[0] = 7; fle_imp_GetVersionExW();\n\
    printf(\"w ret %llu\\n\", (unsigned long long)R[0]);\n\
    memcpy(&v, b + 4, 4); printf(\"w major %u\\n\", v); memcpy(&v, b + 8, 4); printf(\"w minor %u\\n\", v);\n\
    memcpy(&v, b + 12, 4); printf(\"w build %u\\n\", v); memcpy(&v, b + 16, 4); printf(\"w plat %u\\n\", v);\n\
    memcpy(&w, b + 20, 2); printf(\"w csd0 %u\\n\", w); memcpy(&w, b + 274, 2); printf(\"w csdlast %u\\n\", w);\n\
    printf(\"w tail %x\\n\", b[276]);\n\
    memset(b, 0xAB, sizeof b); sz = 284; memcpy(b, &sz, 4); R[0] = 7; fle_imp_GetVersionExW();\n\
    printf(\"x ret %llu\\n\", (unsigned long long)R[0]);\n\
    memcpy(&v, b + 4, 4); printf(\"x major %u\\n\", v); memcpy(&w, b + 276, 2); printf(\"x spmaj %u\\n\", w);\n\
    memcpy(&w, b + 278, 2); printf(\"x spmin %u\\n\", w); memcpy(&w, b + 280, 2); printf(\"x suite %u\\n\", w);\n\
    printf(\"x prod %u\\n\", b[282]); printf(\"x res %x\\n\", b[283]);\n\
    memset(b, 0, sizeof b); sz = 100; memcpy(b, &sz, 4); R[0] = 7; fle_imp_GetVersionExW();\n\
    printf(\"bad ret %llu\\n\", (unsigned long long)R[0]); memcpy(&v, b + 4, 4); printf(\"bad major %u\\n\", v);\n\
    fle_imp_GetLastError(); printf(\"bad le %llu\\n\", (unsigned long long)R[0]);\n\
    return 0;\n}\n";
    let (exe, _) = build_c("getversion", src);
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success());
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    // GetVersion: build in the high word, minor in bit 8..15, major in bit 0..7 (MSDN Examples code).
    let gv = (EMU.build << 16) | (EMU.minor << 8) | EMU.major;
    assert_eq!(got[0], format!("gv {gv:x}"));
    assert_eq!(gv, 0x4A65_000A); // 19045 = 0x4A65
    let want = [
        "w ret 1", "w major 10", "w minor 0", "w build 19045", "w plat 2", "w csd0 0", "w csdlast 0",
        // OSVERSIONINFOW is 276 bytes: the byte after it was not written (still 0xAB).
        "w tail ab",
        "x ret 1", "x major 10", "x spmaj 0", "x spmin 0", "x suite 0", "x prod 1",
        // wReserved is not written.
        "x res ab",
        "bad ret 0", "bad major 0", "bad le 87",
    ];
    assert_eq!(&got[1..], &want[..], "{out}");
}

/// Cases the pages leave open stop the run (abort). Each row is a body for main, and the C program must die by SIGABRT.
#[test]
fn undefined_cases_abort() {
    let rows: &[(&str, &str)] = &[
        ("type-mask-0", "R[2] = 0; R[8] = 0x18;"),
        ("type-mask-high-bits", "R[2] = 0x102; R[8] = 0x18;"),
        ("condition-0", "R[2] = 0x02; R[8] = 0;"),
        ("condition-6-on-major", "R[2] = 0x02; R[8] = 6u << 3;"),
        ("suite-condition-3", "R[2] = 0x40; R[8] = 3u << 18; b[280] = 1;"),
        ("suite-empty", "R[2] = 0x40; R[8] = 6u << 18;"),
        ("bad-size", "R[2] = 0x02; R[8] = 3u << 3; b[0] = 0x14;"),
        // Size 0 alone is let through (decision 0040, test size_0_is_not_checked); other sizes still stop.
        // "bad-size" above is 0x114 (low byte 0x14, high byte 0x01 from the initializer) = sizeof(OSVERSIONINFOW).
        ("size-1", "R[2] = 0x80; R[8] = UINT64_C(1) << 21; b[0] = 1; b[1] = 0; b[282] = 1;"),
        ("size-0x11d", "R[2] = 0x80; R[8] = UINT64_C(1) << 21; b[0] = 0x1d; b[282] = 1;"),
        ("size-high-bit", "R[2] = 0x80; R[8] = UINT64_C(1) << 21; b[0] = 0; b[1] = 0; b[3] = 0x80; b[282] = 1;"),
        ("null-info", "R[1] = 0; R[2] = 0x02; R[8] = 3u << 3;"),
        // major >, minor <= : the page says the request is changed without defining the result.
        ("mixed-conditions", "R[2] = 0x03; R[8] = (2u << 3) | 5u;"),
    ];
    for (name, body) in rows {
        let src = format!(
            "#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_osver.h\"\n\nuint64_t R[16];\n\
int main(void)\n{{\n    uint8_t b[284] = {{ 0x1c, 0x01 }};\n    R[1] = (uint64_t)(uintptr_t)b; R[0] = 99;\n    {body}\n    fle_imp_VerifyVersionInfoW();\n    printf(\"returned %llu\\n\", (unsigned long long)R[0]);\n    return 0;\n}}\n"
        );
        let (exe, _) = build_c(&format!("abort-{name}"), &src);
        let run = Command::new(&exe).output().expect("run failed");
        assert!(!run.status.success(), "{name}: expected abort, got exit success; stdout: {}", String::from_utf8_lossy(&run.stdout));
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}");
        assert!(String::from_utf8_lossy(&run.stderr).contains("fle:"), "{name}: {}", String::from_utf8_lossy(&run.stderr));
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(run.status.signal(), Some(6), "{name}");
    }
}

/// Decision 0040 (provisional; unofficial sources; not checked on a real Windows): dwOSVersionInfoSize = 0 is
/// not checked and the comparison goes on. The call FLEngine's f_454030 makes (0x45406e): a 284-byte
/// structure zeroed, wProductType = 1, dwOSVersionInfoSize never stored, TypeMask VER_PRODUCT_TYPE,
/// VER_EQUAL. The expected result is computed by `expect` (the spec rules), not fixed by hand. Only the
/// members named by dwTypeMask are read: the other members hold values that would change the result if read.
#[test]
fn size_0_is_not_checked() {
    let prod = |p: u32| Os { prod: p, ..base() };
    // (name, requested wProductType, TypeMask items, extra C to poison members not named by TypeMask)
    let rows: Vec<(&str, Os, Vec<(u32, u32)>)> = vec![
        ("f_454030", prod(1), vec![(V_PROD, 1)]),
        ("product-3", prod(3), vec![(V_PROD, 1)]),
        ("product-1-greater-equal", prod(1), vec![(V_PROD, 3)]),
        ("product-2-less", prod(2), vec![(V_PROD, 4)]),
    ];
    let mut s = String::new();
    s.push_str("#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n#include \"fle_osver.h\"\n\nuint64_t R[16];\n\n");
    s.push_str("int main(void)\n{\n    uint8_t b[284];\n");
    let mut wants = Vec::new();
    for (name, req, items) in &rows {
        let tm: u32 = items.iter().fold(0, |a, i| a | i.0);
        let want = expect(EMU, *req, items);
        wants.push((name.to_string(), want));
        // size 0; every member not named by TypeMask is filled with 0xFF (major, minor, build, platform, SP, suite).
        s.push_str(&format!(
            "    memset(b, 0, sizeof b); memset(b + 4, 0xFF, 278); b[282] = {};\n    R[1] = (uint64_t)(uintptr_t)b; R[2] = {}; R[8] = UINT64_C({}); R[0] = 99; fle_set_last_error(0); fle_imp_VerifyVersionInfoW();\n    printf(\"{} ret %llu\", (unsigned long long)R[0]);\n    fle_imp_GetLastError(); printf(\" le %llu\\n\", (unsigned long long)R[0]);\n",
            req.prod, tm, mask(items), name
        ));
    }
    s.push_str("    return 0;\n}\n");
    assert!(wants.iter().any(|w| w.1) && wants.iter().any(|w| !w.1));
    assert!(wants[0].1, "f_454030 on the emulated workstation");
    let (exe, _) = build_c("size-0-checked", &s);
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "stderr: {}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), wants.len(), "{out}");
    for ((name, want), line) in wants.iter().zip(got) {
        let le = if *want { 0 } else { 1150 };
        assert_eq!(line, format!("{name} ret {} le {le}", *want as u32));
    }
}

/// The emulated OS is defined in one place only (runtime/fle_osver.c). No other runtime file holds its values.
#[test]
fn emulated_os_defined_once() {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let c = std::fs::read_to_string(rt.join("fle_osver.c")).unwrap();
    assert!(c.contains(".major = 10,") && c.contains(".build = 19045,") && c.contains(".product_type = 1,"));
    for f in ["fle_imports.c", "fle_imports.h", "fle_osver.h"] {
        let t = std::fs::read_to_string(rt.join(f)).unwrap();
        assert!(!t.contains("19045"), "{f}");
    }
}
