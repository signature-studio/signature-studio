//! CPUID のテスト。
//! - 命令 0f a2 の生成 C の本体（lift_body）が runtime の fle_cpuid を呼ぶこと
//! - 生成 C を clang -O0 でコンパイルして実行した応答が、docs/decisions/0009-cpuid.md の表と一致すること
//!
//! 応答の値は仕様から決まらない部分を含む（作業者の判断）。ホストの CPUID とは比べない。
//! Decision 0118 (provisional): leaf 1 EDX is the x86-64 baseline of the psABI that CPUID reports: FPU (0), CX8 (8), CMOV (15), MMX (23), FXSR (24) and the
//! SSE (25), SSE2 (26) of decision 0009. EDX = 0x0780_8101.

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::{Command, Output};

const CPUID: [u8; 2] = [0x0f, 0xa2];

/// Leaf 1 EDX of decision 0118: bits 0 FPU, 8 CX8, 15 CMOV, 23 MMX, 24 FXSR, 25 SSE, 26 SSE2 (Intel SDM Vol. 1 Table 21-10).
const EDX1: u32 = (1 << 0) | (1 << 8) | (1 << 15) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26);

/// (EAX の入力, ECX の入力, 期待する EAX, EBX, ECX, EDX)
const CASES: &[(u64, u64, u32, u32, u32, u32)] = &[
    (0x0, 0, 1, 0, 0, 0),
    (0x1, 0, 0, 0, 0, EDX1),
    (0x1, 5, 0, 0, 0, EDX1),
    (0x2, 0, 0, 0, 0, EDX1),
    (0x7, 0, 0, 0, 0, EDX1),
    (0x7, 1, 0, 0, 0, EDX1),
    (0x7fff_ffff, 0, 0, 0, 0, EDX1),
    (0x8000_0000, 0, 0, 0, 0, 0),
    (0x8000_0008, 0, 0, 0, 0, 0),
    (0x1_0000_0001, 0, 0, 0, 0, EDX1),
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-cpuid/test (not /tmp). clang's temporary files go to out-cpuid/test/tmp.
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-cpuid/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

#[test]
fn cpuid_body_calls_runtime() {
    let body = lift_body(&CPUID, 0x40f47b).expect("lift");
    assert_eq!(body, "{ void fle_cpuid(void); fle_cpuid(); }");
}

/// The C driver. Calls the lifted body for each case with R[0] = EAX and R[1] = ECX, and prints EAX EBX ECX EDX.
fn driver() -> String {
    let body = lift_body(&CPUID, 0x1000).expect("lift");
    let mut s = String::new();
    s += "#include \"fle_runtime.h\"\n#include \"fle_cpuid.h\"\n\nuint64_t R[16];\n";
    s += &format!("static void c(void)\n{{\n{body}\n}}\n");
    s += "struct In { uint64_t eax; uint64_t ecx; };\n";
    s += "static const struct In IN[] = {\n";
    for &(a, c, ..) in CASES {
        s += &format!("    {{ UINT64_C({a:#x}), UINT64_C({c:#x}) }},\n");
    }
    s += "};\n\n";
    s += &format!("#define N {}\n\n", CASES.len());
    s += "int main(void)\n{\n    unsigned k;\n";
    s += "    for (k = 0; k < N; k++) {\n";
    s += "        R[0] = IN[k].eax;\n        R[1] = IN[k].ecx;\n";
    s += "        c();\n";
    s += "        printf(\"%u %x %x %x %x\\n\", k, (unsigned)R[0], (unsigned)R[3], (unsigned)R[1], (unsigned)R[2]);\n";
    s += "    }\n    return 0;\n}\n";
    s
}

fn build_and_run(src: &str) -> Output {
    let dir = out_dir();
    let tag = etxt_common::uniq("cpuid_lifted");
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, src).expect("write C source");
    let rt = crate_dir().join("../../runtime");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE"]);
    cmd.arg("-I").arg(&rt).arg(rt.join("fle_cpuid.c"));
    cmd.arg("-o").arg(&exe).arg(&c);
    cmd.env("TMPDIR", dir.join("tmp"));
    let cc = cmd.output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    Command::new(&exe).output().expect("run test binary")
}

#[test]
fn cpuid_response_matches_table() {
    let out = build_and_run(&driver());
    assert!(out.status.success(), "lifted program failed: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    let rows: Vec<Vec<u32>> = text
        .lines()
        .map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(t.len(), 5, "bad line: {l}");
            t.iter()
                .enumerate()
                .map(|(j, x)| {
                    if j == 0 {
                        x.parse::<u32>().expect("index")
                    } else {
                        u32::from_str_radix(x, 16).expect("hex")
                    }
                })
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), CASES.len(), "row count");
    for (k, &(a, c, ea, eb, ec, ed)) in CASES.iter().enumerate() {
        assert_eq!(rows[k], vec![k as u32, ea, eb, ec, ed], "case {k}: eax {a:#x} ecx {c:#x}");
    }
}

/// The bits one by one (decision 0118), from the table of Intel SDM Vol. 1 Table 21-10 (EDX[0] FPU, [8] CMPXCHG8B, [15] CMOV, [23] MMX, [24] FXSR, [25] SSE,
/// [26] SSE2). The mask is written out here in hex and not computed from the constants of the C file, so a changed bit shows.
#[test]
fn leaf1_edx_is_exactly_the_x86_64_baseline_bits() {
    assert_eq!(EDX1, 0x0780_8101);
    let src = std::fs::read_to_string(crate_dir().join("../../runtime/fle_cpuid.c")).unwrap();
    for (name, bit) in [("FPU", 0), ("CX8", 8), ("CMOV", 15), ("MMX", 23), ("FXSR", 24), ("SSE", 25), ("SSE2", 26)] {
        let def = format!("#define FLE_CPUID_EDX_{name:<6}(UINT64_C(1) << {bit})");
        assert!(src.contains(&def), "{name} must be bit {bit}: expected `{def}`");
    }
    for (name, bit) in [("TSC", 4), ("SEP", 11), ("PAE", 6), ("MSR", 5), ("APIC", 9), ("MTRR", 12), ("PGE", 13), ("CLFSH", 19), ("HTT", 28)] {
        assert_eq!(EDX1 & (1 << bit), 0, "{name} (bit {bit}) is not in the baseline: stays 0");
    }
}

/// Every leaf from 1 up to 0x7fffffff gives the same EDX; ECX never reports SSE3 or later (decisions 0032, 0075), at any ECX input; the vendor, the
/// version and the brand stay 0 (decision 0009); the extended leaves stay 0.
#[test]
fn only_edx_changed_and_ecx_ebx_eax_stay_zero() {
    let out = build_and_run(&driver());
    let text = String::from_utf8(out.stdout).unwrap();
    for l in text.lines() {
        let t: Vec<&str> = l.split_whitespace().collect();
        let k: usize = t[0].parse().unwrap();
        let (eax, ebx, ecx, edx) = (
            u32::from_str_radix(t[1], 16).unwrap(),
            u32::from_str_radix(t[2], 16).unwrap(),
            u32::from_str_radix(t[3], 16).unwrap(),
            u32::from_str_radix(t[4], 16).unwrap(),
        );
        if CASES[k].0 == 0 {
            assert_eq!((eax, ebx, ecx, edx), (1, 0, 0, 0), "leaf 0");
        } else {
            assert_eq!((eax, ebx, ecx), (0, 0, 0), "case {k}");
            assert_eq!(edx, if CASES[k].0 >= 0x8000_0000 && CASES[k].0 < 0x1_0000_0000 { 0 } else { EDX1 }, "case {k}");
        }
    }
}
