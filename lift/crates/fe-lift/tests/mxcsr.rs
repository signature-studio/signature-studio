//! STMXCSR と LDMXCSR のテスト。3 つを比べる。
//! - 生成 C（`lift_body` の出力）を clang -O0 でコンパイルして実行した結果
//! - ホストの x86_64 CPU で同じ命令を実行した結果（インライン asm）
//! - SDM の期待値（表に書いた値）
//!
//! 予約ビット（31:16）の LDMXCSR は、生成 C が abort し、ホストは #GP で止まる
//! （Linux では SIGSEGV）ことを確かめる。

mod etxt_common;

use fe_lift::lift_body;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const STMXCSR_RDI: [u8; 3] = [0x0f, 0xae, 0x1f];
const LDMXCSR_RDI: [u8; 3] = [0x0f, 0xae, 0x17];
const SIGABRT: i32 = 6;
const SIGSEGV: i32 = 11;

#[derive(Clone, Copy, Debug)]
enum Op {
    Store,
    Load,
}

#[derive(Clone, Copy, Debug)]
struct Case {
    op: Op,
    /// MXCSR before the instruction. None keeps the value left by the previous case.
    init: Option<u32>,
    /// The 32-bit memory operand before the instruction. For LDMXCSR, the value loaded.
    mem: u32,
    /// Expected MXCSR and memory after the instruction (SDM: MXCSR reset 1F80H; STMXCSR stores all 32 bits).
    exp_mx: u32,
    exp_mem: u32,
}

fn cases() -> Vec<Case> {
    use Op::*;
    vec![
        Case { op: Store, init: Some(0x1F80), mem: 0xdead_beef, exp_mx: 0x1F80, exp_mem: 0x1F80 },
        Case { op: Store, init: Some(0x7FBF), mem: 0, exp_mx: 0x7FBF, exp_mem: 0x7FBF },
        Case { op: Store, init: Some(0), mem: 0xffff_ffff, exp_mx: 0, exp_mem: 0 },
        Case { op: Store, init: Some(0xFFFF), mem: 0, exp_mx: 0xFFFF, exp_mem: 0xFFFF },
        Case { op: Load, init: Some(0x1F80), mem: 0x7FBF, exp_mx: 0x7FBF, exp_mem: 0x7FBF },
        Case { op: Store, init: None, mem: 0x1234_5678, exp_mx: 0x7FBF, exp_mem: 0x7FBF },
        Case { op: Load, init: None, mem: 0x40, exp_mx: 0x40, exp_mem: 0x40 },
        Case { op: Store, init: None, mem: 0, exp_mx: 0x40, exp_mem: 0x40 },
        Case { op: Load, init: None, mem: 0, exp_mx: 0, exp_mem: 0 },
        Case { op: Store, init: None, mem: 0xdead_beef, exp_mx: 0, exp_mem: 0 },
        Case { op: Load, init: None, mem: 0x0000_FFFF, exp_mx: 0xFFFF, exp_mem: 0x0000_FFFF },
        Case { op: Store, init: None, mem: 0, exp_mx: 0xFFFF, exp_mem: 0xFFFF },
    ]
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-mxcsr (not /tmp). clang's temporary files go to out-mxcsr/test/tmp.
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-mxcsr/test");
    std::fs::create_dir_all(d.join("tmp")).expect("create test output dir");
    d
}

/// The C driver. `lifted` selects the generated body (true) or the host instruction (false).
fn driver(cs: &[Case], lifted: bool) -> String {
    let mut s = String::new();
    if lifted {
        s += "#include \"fle_runtime.h\"\n#include \"fle_mxcsr.h\"\n\nuint64_t R[16];\n";
        for (k, c) in cs.iter().enumerate() {
            let bytes = match c.op {
                Op::Store => STMXCSR_RDI,
                Op::Load => LDMXCSR_RDI,
            };
            let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("case {k}: {e}"));
            s += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
        }
        s += "static void (*const FN[])(void) = {";
        for k in 0..cs.len() {
            s += &format!(" c{k},");
        }
        s += " };\n";
    } else {
        s += "#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n#include <xmmintrin.h>\n\n";
        s += "static void run_host(int op, uint32_t *p)\n{\n";
        s += "    if (op == 0) __asm__ volatile(\"stmxcsr %0\" : \"=m\"(*p));\n";
        s += "    else __asm__ volatile(\"ldmxcsr %0\" :: \"m\"(*p));\n}\n";
    }
    s += "_Alignas(16) static uint8_t M[64];\n";
    s += "struct In { int has_init; uint32_t init; int op; uint32_t mem; };\n";
    s += "static const struct In IN[] = {\n";
    for c in cs {
        let (hi, init) = match c.init {
            Some(v) => (1, v),
            None => (0, 0),
        };
        let op = match c.op {
            Op::Store => 0,
            Op::Load => 1,
        };
        s += &format!("    {{ {hi}, 0x{init:x}u, {op}, 0x{:x}u }},\n", c.mem);
    }
    s += "};\n\n";
    s += &format!("#define N {}\n\n", cs.len());
    s += "int main(void)\n{\n    unsigned k;\n";
    if lifted {
        s += "    printf(\"reset %x\\n\", (unsigned)fle_mxcsr);\n";
    } else {
        s += "    printf(\"reset %x\\n\", _mm_getcsr());\n";
    }
    s += "    for (k = 0; k < N; k++) {\n        uint32_t v;\n";
    s += "        memcpy(M + 16, &IN[k].mem, 4);\n";
    if lifted {
        s += "        if (IN[k].has_init) fle_mxcsr = IN[k].init;\n";
        s += "        R[7] = (uint64_t)(uintptr_t)(M + 16);\n";
        s += "        FN[k]();\n";
        s += "        memcpy(&v, M + 16, 4);\n";
        s += "        printf(\"%u %x %x\\n\", k, (unsigned)fle_mxcsr, (unsigned)v);\n";
    } else {
        s += "        if (IN[k].has_init) _mm_setcsr(IN[k].init);\n";
        s += "        run_host(IN[k].op, (uint32_t *)(M + 16));\n";
        s += "        memcpy(&v, M + 16, 4);\n";
        s += "        printf(\"%u %x %x\\n\", k, _mm_getcsr(), (unsigned)v);\n";
    }
    s += "    }\n    return 0;\n}\n";
    s
}

fn build_and_run(name: &str, src: &str, lifted: bool) -> Output {
    let dir = out_dir();
    let tag = etxt_common::uniq(&format!("mxcsr_{name}"));
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, src).expect("write C source");
    let rt = crate_dir().join("../../runtime");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=c11", "-w", "-msse"]);
    if lifted {
        cmd.arg("-I").arg(&rt).arg(rt.join("fle_mxcsr.c"));
    }
    cmd.arg("-o").arg(&exe).arg(&c);
    cmd.env("TMPDIR", dir.join("tmp"));
    let cc = cmd.output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    Command::new(&exe).output().expect("run test binary")
}

/// Parses the driver output: the reset line, then one "k mxcsr mem" line per case.
fn parse(out: &str, n: usize) -> (u32, Vec<(u32, u32)>) {
    let mut lines = out.lines();
    let reset_line = lines.next().expect("reset line");
    let reset = u32::from_str_radix(reset_line.strip_prefix("reset ").expect("reset prefix"), 16).expect("reset hex");
    let rows: Vec<(u32, u32)> = lines
        .map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(t.len(), 3, "bad line: {l}");
            (u32::from_str_radix(t[1], 16).expect("mx hex"), u32::from_str_radix(t[2], 16).expect("mem hex"))
        })
        .collect();
    assert_eq!(rows.len(), n, "row count");
    (reset, rows)
}

#[test]
fn stmxcsr_ldmxcsr_match_host_and_sdm() {
    let cs = cases();
    let lifted = build_and_run("lifted", &driver(&cs, true), true);
    assert!(lifted.status.success(), "lifted program failed: {}", String::from_utf8_lossy(&lifted.stderr));
    let host = build_and_run("host", &driver(&cs, false), false);
    assert!(host.status.success(), "host program failed");

    let (l_reset, l_rows) = parse(&String::from_utf8(lifted.stdout).unwrap(), cs.len());
    let (h_reset, h_rows) = parse(&String::from_utf8(host.stdout).unwrap(), cs.len());

    assert_eq!(l_reset, 0x1F80, "generated C: reset MXCSR");
    assert_eq!(h_reset, 0x1F80, "host: reset MXCSR");
    for (k, c) in cs.iter().enumerate() {
        assert_eq!(l_rows[k], (c.exp_mx, c.exp_mem), "case {k} {c:?}: generated C");
        assert_eq!(h_rows[k], (c.exp_mx, c.exp_mem), "case {k} {c:?}: host");
    }
    assert_eq!(l_rows, h_rows, "generated C and host differ");
}

/// Load of a reserved-bit value. Generated C aborts with a runtime message. Host raises #GP (SIGSEGV on Linux).
fn reserved_case(v: u32) {
    let cs = [Case { op: Op::Load, init: None, mem: v, exp_mx: 0, exp_mem: 0 }];
    let name = format!("reserved_{v:08x}");
    let lifted = build_and_run(&format!("{name}_lifted"), &driver(&cs, true), true);
    assert_eq!(lifted.status.signal(), Some(SIGABRT), "generated C: expected abort for {v:#x}");
    let err = String::from_utf8_lossy(&lifted.stderr);
    assert!(err.contains("reserved bits"), "generated C: stderr {err}");
    let host = build_and_run(&format!("{name}_host"), &driver(&cs, false), false);
    assert_eq!(host.status.signal(), Some(SIGSEGV), "host: expected #GP (SIGSEGV) for {v:#x}");
}

/// The stop site 0x40a3b2 is `stmxcsr [rsp]` (0f ae 1c 24). Its generated body is used by the measurement copy.
#[test]
fn stmxcsr_rsp_body_text() {
    let body = lift_body(&[0x0f, 0xae, 0x1c, 0x24], 0x40a3b2).expect("lift");
    assert_eq!(body, "{ void fle_mxcsr_store(uint64_t); fle_mxcsr_store((R[4])); }");
}

#[test]
fn ldmxcsr_reserved_bit_16_aborts() {
    reserved_case(0x0001_0000);
}

#[test]
fn ldmxcsr_reserved_bit_31_aborts() {
    reserved_case(0x8000_0000);
}
