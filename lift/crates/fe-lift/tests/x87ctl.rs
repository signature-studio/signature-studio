//! FNSTCW、FLDCW、FNSTSW、FNCLEX、FNINIT、FWAIT のテスト。3 つを比べる。
//! - 生成 C（`lift_body` の出力）を clang -O0 でコンパイルして実行した結果
//! - ホストの x86_64 CPU で同じ命令を実行した結果（インライン asm）
//! - 期待値（表に書いた値。既定の制御ワード 037FH、ステータス 0、タグ FFFFH は SDM の FINIT/FNINIT。
//!   FLDCW の予約ビットの値は、ホストでの実測 `(v & 0x1F3F) | 0x0040`）
//!
//! 生成 C はステータスワードの例外フラグを作れない（x87 演算は未実装）。そのため `SetSw` の段では
//! 生成 C が `fle_x87.sw` を直接書き、ホストは演算（0 除算、比較）で同じ値を作る。

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::{Command, Output};

const FNSTCW_RDI: [u8; 2] = [0xd9, 0x3f];
const FLDCW_RDI: [u8; 2] = [0xd9, 0x2f];
const FNSTSW_RDI: [u8; 2] = [0xdd, 0x3f];
const FNSTSW_AX: [u8; 2] = [0xdf, 0xe0];
const FNCLEX: [u8; 2] = [0xdb, 0xe2];
const FNINIT: [u8; 2] = [0xdb, 0xe3];
const FWAIT: [u8; 1] = [0x9b];

const AX0: u64 = 0x1122_3344_5566_7788;

#[derive(Clone, Copy, Debug)]
enum Step {
    Stcw,
    Ldcw(u16),
    StswMem,
    StswAx,
    Clex,
    Init,
    Wait,
    /// Only right after `Init` (control word 037FH, status 0). 0x0004 is ZE, 0x0100 is C0.
    SetSw(u16),
}

/// Expected state after the step: control word, status word, tag word, ax (whole RAX).
#[derive(Clone, Copy, Debug)]
struct Exp {
    cw: u16,
    sw: u16,
    tw: u16,
    ax: u64,
}

const fn e(cw: u16, sw: u16, ax: u64) -> Exp {
    Exp { cw, sw, tw: 0xFFFF, ax }
}

const A_0000: u64 = 0x1122_3344_5566_0000;
const A_0004: u64 = 0x1122_3344_5566_0004;
const A_0104: u64 = 0x1122_3344_5566_0104;

fn cases() -> Vec<(Step, Exp)> {
    use Step::*;
    vec![
        (Stcw, e(0x037F, 0, AX0)),
        (StswMem, e(0x037F, 0, AX0)),
        (StswAx, e(0x037F, 0, A_0000)),
        (Ldcw(0x027F), e(0x027F, 0, A_0000)),
        (Stcw, e(0x027F, 0, A_0000)),
        (Ldcw(0x0C7F), e(0x0C7F, 0, A_0000)),
        (Stcw, e(0x0C7F, 0, A_0000)),
        (Ldcw(0xFFFF), e(0x1F7F, 0, A_0000)),
        (Stcw, e(0x1F7F, 0, A_0000)),
        (Ldcw(0x0000), e(0x0040, 0, A_0000)),
        (Stcw, e(0x0040, 0, A_0000)),
        (Ldcw(0xE000), e(0x0040, 0, A_0000)),
        (Ldcw(0x1000), e(0x1040, 0, A_0000)),
        (Stcw, e(0x1040, 0, A_0000)),
        (Init, e(0x037F, 0, A_0000)),
        (Stcw, e(0x037F, 0, A_0000)),
        (SetSw(0x0104), e(0x037F, 0x0104, A_0000)),
        (StswMem, e(0x037F, 0x0104, A_0000)),
        (StswAx, e(0x037F, 0x0104, A_0104)),
        (Clex, e(0x037F, 0x0100, A_0104)),
        (StswMem, e(0x037F, 0x0100, A_0104)),
        (Init, e(0x037F, 0, A_0104)),
        (SetSw(0x0004), e(0x037F, 0x0004, A_0104)),
        (StswAx, e(0x037F, 0x0004, A_0004)),
        (Wait, e(0x037F, 0x0004, A_0004)),
        (Clex, e(0x037F, 0, A_0004)),
        (SetSw(0x0100), e(0x037F, 0x0100, A_0004)),
        (StswMem, e(0x037F, 0x0100, A_0004)),
        (Init, e(0x037F, 0, A_0004)),
        (StswMem, e(0x037F, 0, A_0004)),
    ]
}

fn mem_preset(s: Step) -> u32 {
    match s {
        Step::Ldcw(v) => 0xCAFE_0000 | v as u32,
        _ => 0xCAFE_DEAD,
    }
}

/// The 32-bit word at M+16 after the step. Stores write 16 bits and leave the upper 16 bits.
fn mem_expected(s: Step, x: Exp) -> u32 {
    match s {
        Step::Stcw => 0xCAFE_0000 | x.cw as u32,
        Step::StswMem => 0xCAFE_0000 | x.sw as u32,
        _ => mem_preset(s),
    }
}

fn bytes(s: Step) -> Option<&'static [u8]> {
    Some(match s {
        Step::Stcw => &FNSTCW_RDI,
        Step::Ldcw(_) => &FLDCW_RDI,
        Step::StswMem => &FNSTSW_RDI,
        Step::StswAx => &FNSTSW_AX,
        Step::Clex => &FNCLEX,
        Step::Init => &FNINIT,
        Step::Wait => &FWAIT,
        Step::SetSw(_) => return None,
    })
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-x87ctl/test (not /tmp). clang's temporary files go to out-x87ctl/test/tmp.
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-x87ctl/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

const REPORT_FMT: &str = "printf(\"%u %x %x %x %x %llx\\n\", K, CW, SW, TW, V, AX);";

fn host_step(s: Step) -> String {
    let p = "(uint16_t *)(M + 16)";
    match s {
        Step::Stcw => format!("__asm__ volatile(\"fnstcw %0\" : \"=m\"(*{p}));"),
        Step::Ldcw(_) => format!("__asm__ volatile(\"fldcw %0\" :: \"m\"(*{p}));"),
        Step::StswMem => format!("__asm__ volatile(\"fnstsw %0\" : \"=m\"(*{p}));"),
        Step::StswAx => "__asm__ volatile(\"fnstsw %%ax\" : \"+a\"(rax));".to_string(),
        Step::Clex => "__asm__ volatile(\"fnclex\");".to_string(),
        Step::Init => "__asm__ volatile(\"fninit\");".to_string(),
        Step::Wait => "__asm__ volatile(\"fwait\");".to_string(),
        Step::SetSw(0x0004) => {
            "__asm__ volatile(\"fld1; fldz; fdivrp %%st, %%st(1); fstp %%st(0)\" ::: \"st\", \"st(1)\");".to_string()
        }
        Step::SetSw(0x0100) => "__asm__ volatile(\"fld1; fldz; fcompp\" ::: \"st\", \"st(1)\");".to_string(),
        Step::SetSw(0x0104) => format!(
            "{} {}",
            host_step(Step::SetSw(0x0004)),
            host_step(Step::SetSw(0x0100))
        ),
        Step::SetSw(v) => panic!("host cannot make status word {v:#x}"),
    }
}

/// The C driver. `lifted` selects the generated body (true) or the host instruction (false).
fn driver(cs: &[(Step, Exp)], lifted: bool) -> String {
    let mut s = String::new();
    if lifted {
        s += "#include \"fle_runtime.h\"\n#include \"fle_x87ctl.h\"\n\nuint64_t R[16];\n";
        for (k, (st, _)) in cs.iter().enumerate() {
            if let Some(b) = bytes(*st) {
                let body = lift_body(b, 0x1000).unwrap_or_else(|er| panic!("case {k} {st:?}: {er}"));
                s += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
            }
        }
    } else {
        s += "#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n\n";
        s += "static uint16_t host_cw(void) { uint16_t v; __asm__ volatile(\"fnstcw %0\" : \"=m\"(v)); return v; }\n";
        s += "static uint16_t host_sw(void) { uint16_t v; __asm__ volatile(\"fnstsw %0\" : \"=m\"(v)); return v; }\n";
        s += "static uint16_t host_tw(void) { uint32_t env[7]; __asm__ volatile(\"fnstenv %0; fldenv %0\" : \"+m\"(env)); return (uint16_t)env[2]; }\n";
    }
    s += "_Alignas(16) static uint8_t M[64];\n\n";
    s += "int main(void)\n{\n    uint32_t v;\n";
    if lifted {
        s += "    printf(\"reset %x %x %x\\n\", (unsigned)fle_x87.cw, (unsigned)fle_x87.sw, (unsigned)fle_x87.tw);\n";
        s += &format!("    R[0] = UINT64_C({AX0:#x});\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n");
    } else {
        s += "    uint64_t rax = UINT64_C(0x1122334455667788);\n";
        s += "    printf(\"reset %x %x %x\\n\", host_cw(), host_sw(), host_tw());\n";
    }
    for (k, (st, _)) in cs.iter().enumerate() {
        let pre = mem_preset(*st);
        s += &format!("    {{ uint32_t m = {pre:#x}u; memcpy(M + 16, &m, 4); }}\n");
        if lifted {
            match st {
                Step::SetSw(sw) => s += &format!("    fle_x87.sw = {sw:#x};\n"),
                _ => s += &format!("    c{k}();\n"),
            }
            s += "    memcpy(&v, M + 16, 4);\n";
            s += &format!(
                "    {}\n",
                REPORT_FMT
                    .replace("K", &k.to_string())
                    .replace("CW", "(unsigned)fle_x87.cw")
                    .replace("SW", "(unsigned)fle_x87.sw")
                    .replace("TW", "(unsigned)fle_x87.tw")
                    .replace("V", "(unsigned)v")
                    .replace("AX", "(unsigned long long)R[0]")
            );
        } else {
            s += &format!("    {}\n", host_step(*st));
            s += "    memcpy(&v, M + 16, 4);\n";
            s += &format!(
                "    {}\n",
                REPORT_FMT
                    .replace("K", &k.to_string())
                    .replace("CW", "(unsigned)host_cw()")
                    .replace("SW", "(unsigned)host_sw()")
                    .replace("TW", "(unsigned)host_tw()")
                    .replace("V", "(unsigned)v")
                    .replace("AX", "(unsigned long long)rax")
            );
        }
    }
    if !lifted {
        s += "    __asm__ volatile(\"fninit\");\n";
    }
    s += "    return 0;\n}\n";
    s
}

fn build_and_run(name: &str, src: &str, lifted: bool) -> Output {
    let dir = out_dir();
    let c = dir.join(format!("x87ctl_{name}.c"));
    let exe = dir.join(format!("x87ctl_{name}"));
    std::fs::write(&c, src).expect("write C source");
    let rt = crate_dir().join("../../runtime");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=c11", "-w"]);
    if lifted {
        cmd.arg("-I").arg(&rt).arg(rt.join("fle_x87ctl.c"));
    }
    cmd.arg("-o").arg(&exe).arg(&c);
    cmd.env("TMPDIR", dir.join("tmp"));
    let cc = cmd.output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    Command::new(&exe).output().expect("run test binary")
}

type Row = (u32, u32, u32, u32, u64);

/// Parses the driver output: the reset line, then one "k cw sw tw mem ax" line per case.
fn parse(out: &str, n: usize) -> ((u32, u32, u32), Vec<Row>) {
    let mut lines = out.lines();
    let r = lines.next().expect("reset line");
    let t: Vec<&str> = r.split_whitespace().collect();
    assert_eq!(t[0], "reset", "reset prefix");
    let h = |x: &str| u32::from_str_radix(x, 16).expect("hex");
    let reset = (h(t[1]), h(t[2]), h(t[3]));
    let rows: Vec<Row> = lines
        .map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(t.len(), 6, "bad line: {l}");
            (h(t[1]), h(t[2]), h(t[3]), h(t[4]), u64::from_str_radix(t[5], 16).expect("ax hex"))
        })
        .collect();
    assert_eq!(rows.len(), n, "row count");
    (reset, rows)
}

#[test]
fn control_instructions_match_host_and_expected() {
    let cs = cases();
    let lifted = build_and_run("lifted", &driver(&cs, true), true);
    assert!(lifted.status.success(), "lifted program failed: {}", String::from_utf8_lossy(&lifted.stderr));
    let host = build_and_run("host", &driver(&cs, false), false);
    assert!(host.status.success(), "host program failed: {}", String::from_utf8_lossy(&host.stderr));

    let (l_reset, l_rows) = parse(&String::from_utf8(lifted.stdout).unwrap(), cs.len());
    let (h_reset, h_rows) = parse(&String::from_utf8(host.stdout).unwrap(), cs.len());

    assert_eq!(l_reset, (0x037F, 0, 0xFFFF), "generated C: reset state");
    assert_eq!(h_reset, (0x037F, 0, 0xFFFF), "host: reset state");
    for (k, (st, x)) in cs.iter().enumerate() {
        let want: Row = (x.cw as u32, x.sw as u32, x.tw as u32, mem_expected(*st, *x), x.ax);
        assert_eq!(l_rows[k], want, "case {k} {st:?}: generated C");
        assert_eq!(h_rows[k], want, "case {k} {st:?}: host");
    }
    assert_eq!(l_rows, h_rows, "generated C and host differ");
}

/// All 65536 FLDCW values: the generated C reads back what the host reads back.
#[test]
fn fldcw_every_value_matches_host() {
    let mut s = String::new();
    s += "#include \"fle_runtime.h\"\n#include \"fle_x87ctl.h\"\n\nuint64_t R[16];\n";
    s += "static void ldcw(void)\n{\n";
    s += &lift_body(&FLDCW_RDI, 0x1000).expect("fldcw");
    s += "\n}\nstatic void stcw(void)\n{\n";
    s += &lift_body(&FNSTCW_RDI, 0x1000).expect("fnstcw");
    s += "\n}\n_Alignas(16) static uint8_t M[64];\n";
    s += "int main(void)\n{\n    uint32_t v;\n    R[7] = (uint64_t)(uintptr_t)M;\n";
    s += "    for (v = 0; v < 0x10000; v++) {\n        uint16_t m = (uint16_t)v, out;\n";
    s += "        memcpy(M, &m, 2);\n        ldcw();\n        stcw();\n        memcpy(&out, M, 2);\n";
    s += "        printf(\"%x %x\\n\", (unsigned)v, (unsigned)out);\n    }\n    return 0;\n}\n";
    let lifted = build_and_run("fldcw_all_lifted", &s, true);
    assert!(lifted.status.success(), "lifted program failed: {}", String::from_utf8_lossy(&lifted.stderr));

    let mut h = String::new();
    h += "#include <stdint.h>\n#include <stdio.h>\n\nint main(void)\n{\n    uint32_t v;\n";
    h += "    for (v = 0; v < 0x10000; v++) {\n        uint16_t m = (uint16_t)v, out;\n";
    h += "        __asm__ volatile(\"fldcw %0\" :: \"m\"(m));\n        __asm__ volatile(\"fnstcw %0\" : \"=m\"(out));\n";
    h += "        printf(\"%x %x\\n\", (unsigned)v, (unsigned)out);\n    }\n";
    h += "    __asm__ volatile(\"fninit\");\n    return 0;\n}\n";
    let host = build_and_run("fldcw_all_host", &h, false);
    assert!(host.status.success(), "host program failed");

    let l = String::from_utf8(lifted.stdout).unwrap();
    let hs = String::from_utf8(host.stdout).unwrap();
    assert_eq!(l.lines().count(), 0x10000, "row count");
    for (a, b) in l.lines().zip(hs.lines()) {
        assert_eq!(a, b, "generated C and host differ");
    }
    for line in l.lines() {
        let t: Vec<u32> = line.split(' ').map(|x| u32::from_str_radix(x, 16).unwrap()).collect();
        assert_eq!(t[1], (t[0] & 0x1F3F) | 0x0040, "value {:#x}", t[0]);
    }
}

#[test]
fn fnstcw_rsp_body_text() {
    // The stop site 0x40a332 is `fnstcw WORD PTR [rsp]` (d9 3c 24).
    let body = lift_body(&[0xd9, 0x3c, 0x24], 0x40a332).expect("lift");
    assert_eq!(body, "{ void fle_x87_store_cw(uint64_t); fle_x87_store_cw((R[4])); }");
}

#[test]
fn fldcw_rip_body_text() {
    // 0x40a329: fldcw WORD PTR [rip+0xf9ad21] = 0x13a5050 (d9 2d 21 ad f9 00)
    let body = lift_body(&[0xd9, 0x2d, 0x21, 0xad, 0xf9, 0x00], 0x40a329).expect("lift");
    assert_eq!(body, "{ void fle_x87_load_cw(uint64_t); fle_x87_load_cw(UINT64_C(0x13a5050)); }");
}

#[test]
fn fnstsw_rbp_body_text() {
    // 0x6398d5: fnstsw WORD PTR [rbp+0xc] (dd 7d 0c)
    let body = lift_body(&[0xdd, 0x7d, 0x0c], 0x6398d5).expect("lift");
    assert_eq!(body, "{ void fle_x87_store_sw(uint64_t); fle_x87_store_sw((R[5] + UINT64_C(0xc))); }");
}

#[test]
fn wait_forms_have_no_side_effect_and_store_forms_equal_no_wait_forms() {
    assert_eq!(lift_body(&FWAIT, 0x1000).expect("fwait"), "");
}

/// What is still not lifted stops at fle_unsupported instead of being approximated. The
/// arithmetic forms (fld, fmulp, fstp, ...) are lifted since decision 0033 (tests/x87.rs).
#[test]
fn x87_arithmetic_stays_unsupported() {
    // fnstenv [rdi] and fnsave [rdi] are not modelled
    assert!(lift_body(&[0xd9, 0x37], 0x1000).is_err());
    assert!(lift_body(&[0xdd, 0x37], 0x1000).is_err());
    // fsin, fprem, fpatan (transcendental / partial remainder: not implemented, decision 0033;
    // f2xm1 and fsincos are implemented since decision 0043)
    assert!(lift_body(&[0xd9, 0xfe], 0x1000).is_err());
    assert!(lift_body(&[0xd9, 0xf8], 0x1000).is_err());
    assert!(lift_body(&[0xd9, 0xf3], 0x1000).is_err());
}
