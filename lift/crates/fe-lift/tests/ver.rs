//! Tests for kernel32!VerSetConditionMask.
//!
//! 1. The thunk for VerSetConditionMask calls the runtime (table in src/imports_table.rs).
//!    VerifyVersionInfoW (not in the table) stays unsupported.
//! 2. The runtime function (runtime/fle_imports.c) returns the mask that the bit layout in
//!    docs/decisions/0011-verSetConditionMask.md gives. The expected values are written out
//!    by hand. The host is Linux, so the Windows API itself cannot run here.

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

/// A PE with kernel32.dll!VerSetConditionMask (in the table) and kernel32.dll!VerifyVersionInfoW
/// (not in the table), plus k.dll!Fn (not in the table). Same layout as tests/imports_table.rs.
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
    w32(&mut b, d + 20, 0x2120);
    w32(&mut b, d + 32, 0x2320);
    w32(&mut b, d + 36, 0x2220);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    put(&mut b, 0x2320, b"k.dll\0");
    put(&mut b, 0x2400, b"\0\0VerSetConditionMask\0");
    put(&mut b, 0x2430, b"\0\0VerifyVersionInfoW\0");
    put(&mut b, 0x2440, b"\0\0Fn\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2108, 0x2430), (0x2200, 0x2400), (0x2208, 0x2430), (0x2120, 0x2440), (0x2220, 0x2440)] {
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
        (0x1060, [&[0xff, 0x25][..], &rel(0x1066, 0x2220)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn table_thunk_calls_runtime_and_others_stay_unsupported() {
    let img = build();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    let vsc = IB + 0x1040;
    let vvi = IB + 0x1050;
    let fn_thunk = IB + 0x1060;

    assert_eq!(st.handled["jmp_imp:VerSetConditionMask"], 1);
    assert!(!st.handled.contains_key("jmp_imp:VerifyVersionInfoW"));
    assert!(c.contains("fle_imp_VerSetConditionMask(); return;"));
    assert!(!c.contains(&format!("L_{vsc:x}: {{ /* {vsc:#x} Jmp */ fle_unsupported")));
    assert!(c.contains(&format!("L_{vvi:x}: {{ /* {vvi:#x} Jmp */ fle_unsupported(UINT64_C({vvi:#x})); }}")));
    assert!(c.contains(&format!("L_{fn_thunk:x}: {{ /* {fn_thunk:#x} Jmp */ fle_unsupported(UINT64_C({fn_thunk:#x})); }}")));
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 2);
}

/// (ConditionMask in, TypeMask, Condition, expected ConditionMask out).
/// Expected values are computed by hand: field k (bit index of the TypeMask flag) takes
/// Condition << 3k, so VER_MAJORVERSION (k=1) is << 3, VER_SERVICEPACKMAJOR (k=5) is << 15, and so on.
const VECTORS: &[(&str, u64, u32, u8, u64)] = &[
    ("major >=", 0, 0x02, 3, 0x18),
    ("minor ==", 0, 0x01, 1, 0x01),
    ("build <=", 0, 0x04, 5, 0x140),
    ("platform ==", 0, 0x08, 1, 0x200),
    ("sp minor >", 0, 0x10, 2, 0x2000),
    ("sp major >=", 0, 0x20, 3, 0x18000),
    ("suite AND", 0, 0x40, 6, 0x180000),
    ("product type ==", 0, 0x80, 1, 0x200000),
    ("chain minor then major", 0x01, 0x02, 3, 0x19),
    ("chain major then sp major", 0x18, 0x20, 3, 0x18018),
    ("condition masked to 3 bits (0x0f -> 7)", 0, 0x02, 0x0f, 0x38),
    ("multi-flag type: highest flag used (Wine order)", 0, 0x03, 3, 0x18),
];

#[test]
fn runtime_mask_matches_hand_computed_layout() {
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-ver/test");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();

    let mut src = String::from("#include <stdio.h>\n#include \"fle_runtime.h\"\n#include \"fle_imports.h\"\n\nuint64_t R[16];\n");
    src += "int main(void)\n{\n";
    for (_, mask, ty, cond, _) in VECTORS {
        src += &format!("    R[1] = UINT64_C({mask:#x}); R[2] = {ty:#x}; R[8] = {cond:#x};\n");
        src += "    fle_imp_VerSetConditionMask();\n";
        src += "    printf(\"%llx\\n\", (unsigned long long)R[0]);\n";
    }
    src += "    return 0;\n}\n";

    let tag = etxt_common::uniq("ver-mask");
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, &src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_imports.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed");

    let got: Vec<u64> = String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(|l| u64::from_str_radix(l.trim(), 16).unwrap())
        .collect();
    assert_eq!(got.len(), VECTORS.len());
    for ((name, _, _, _, want), g) in VECTORS.iter().zip(got) {
        assert_eq!(g, *want, "{name}: got {g:#x}, want {want:#x}");
    }
}
