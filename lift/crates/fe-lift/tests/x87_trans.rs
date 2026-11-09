//! Tests for F2XM1 and FSINCOS (decision 0043): correctly rounded soft implementations in
//! runtime/fle_x87.c, with no host floating point.
//!
//! (a) Reference table: tests/x87_trans/gen_ref.py computes 2^x - 1, sin and cos with Python's decimal
//!     and fractions only (240 digits; the SDM's 66-bit Pi for the FSINCOS reduction), rounds them by RC
//!     and writes the expected state after the instruction (status word, tag word, 8 physical registers)
//!     for every row: all four RC, random PC, 0..7 registers below the operand, preset C0/C2/C3, and
//!     boundary operands (+-0, +-1, denormals, tiny, multiples of Pi/2, 2^63, inf, NaN, unsupported
//!     encodings) plus random ones. The lifted C (clang -O0, fle_x87.c) runs every row; every row must equal.
//! (b) Host comparison (reference values only, x86_64): the SDM allows errors below 1 ulp (nearest) and
//!     below 1.5 ulp (other modes), so the host and the correctly rounded result may differ by one step;
//!     the test counts the differences and requires at most 1 step (nearest) and 2 steps (other RC).
//! (c) Stack, tag word, TOP, C1/C2 and exception flags are part of every row; the named aborts are tested apart.
//! (d) Mutation tests: out-x87c/mutate_trans.py (docs/investigations/x87-trans-run.md).

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-x87c/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

const HEADER: &str = "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_x87ctl.h\"\n#include \"fle_x87.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\n\n";

fn compile(name: &str, src: &str) -> PathBuf {
    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).expect("write C source");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-ffp-contract=off", "-w", "-I"]).arg(&rt).arg("-o").arg(&exe).arg(&c);
    // FLE_X87_C: a mutated copy of fle_x87.c (mutate_trans.py)
    let x87c = std::env::var("FLE_X87_C").map(PathBuf::from).unwrap_or_else(|_| rt.join("fle_x87.c"));
    cmd.arg(rt.join("fle_x87ctl.c")).arg(x87c);
    let cc = cmd.env("TMPDIR", dir.join("tmp")).output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

const F2XM1: [u8; 2] = [0xd9, 0xf0];
const FSINCOS: [u8; 2] = [0xd9, 0xfb];

fn bodies() -> String {
    let b0 = lift_body(&F2XM1, 0x1000).expect("lift f2xm1");
    let b1 = lift_body(&FSINCOS, 0x1000).expect("lift fsincos");
    format!("static void c0(void)\n{{\n{b0}\n}}\nstatic void c1(void)\n{{\n{b1}\n}}\n")
}

// ---------------------------------------------------------------- (a) the reference table

struct Table {
    path: PathBuf,
    rows: Vec<String>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let path = out_dir().join(format!("{}.txt", etxt_common::uniq("x87_trans_ref")));
        let gen = crate_dir().join("tests/x87_trans/gen_ref.py");
        let run = Command::new("python3").arg(&gen).arg(&path).output().expect("run python3 (needed for the reference table)");
        assert!(run.status.success(), "gen_ref.py failed: {}", String::from_utf8_lossy(&run.stderr));
        let text = std::fs::read_to_string(&path).expect("read reference table");
        Table { path, rows: text.lines().map(str::to_string).collect() }
    })
}

const REF_RUNNER: &str = r#"int main(int argc, char **argv)
{
    char line[1024];
    FILE *f = fopen(argv[1], "r");
    if (!f) return 2;
    while (fgets(line, sizeof line, f)) {
        int op, fill, k, p;
        unsigned cw, pre, se, top;
        unsigned long long m;
        if (sscanf(line, "%d %x %x %d %llx %x", &op, &cw, &pre, &fill, &m, &se) != 6) return 3;
        fle_x87_init();
        fle_x87.cw = (uint16_t)cw;
        for (k = 0; k < fill; k++) {
            unsigned char b[10];
            unsigned long long fm = 0x8000000000000000ull | (((unsigned long long)(k + 1) * 0x0123456789ABCDEFull) & 0x7fffffffffffffffull);
            unsigned fe = 0x4000u + (unsigned)k;
            memcpy(b, &fm, 8);
            memcpy(b + 8, &fe, 2);
            fle_x87_fld_m((uint64_t)(uintptr_t)b, 10);
        }
        {
            unsigned char b[10];
            memcpy(b, &m, 8);
            memcpy(b + 8, &se, 2);
            fle_x87_fld_m((uint64_t)(uintptr_t)b, 10);
        }
        fle_x87.sw = (uint16_t)((fle_x87.sw & ~0x4700u) | (pre & 0x4500u));
        if (op) c1(); else c0();
        printf("%04x %04x", (unsigned)fle_x87.sw, (unsigned)fle_x87.tw);
        top = (fle_x87.sw >> 11) & 7;
        (void)top;
        for (p = 0; p < 8; p++) {
            if (((fle_x87.tw >> (2 * p)) & 3) == 3) printf(" -");
            else printf(" %016llx:%04x", (unsigned long long)fle_x87_r[p].m, (unsigned)fle_x87_r[p].se);
        }
        printf("\n");
    }
    return 0;
}
"#;

#[test]
fn x87_trans_matches_the_correctly_rounded_reference() {
    let t = table();
    let src = format!("{HEADER}{}{REF_RUNNER}", bodies());
    let exe = compile("x87_trans_ref", &src);
    let run = Command::new(&exe).arg(&t.path).output().expect("run reference runner");
    assert!(run.status.success(), "reference runner failed: {}", String::from_utf8_lossy(&run.stderr));
    let got: Vec<String> = String::from_utf8(run.stdout).expect("utf8").lines().map(str::to_string).collect();
    assert_eq!(got.len(), t.rows.len(), "output line count");
    let mut bad = 0usize;
    for (row, g) in t.rows.iter().zip(got.iter()) {
        let (input, expect) = row.split_once(" | ").expect("row format");
        if expect != g {
            bad += 1;
            if bad <= 8 {
                eprintln!("MISMATCH input {input}\n  expected {expect}\n  got      {g}");
            }
        }
    }
    // coverage of the table
    let mut by_op = [0usize; 2];
    let mut by_rc = [[0usize; 4]; 2];
    let (mut zero, mut one, mut den, mut tiny, mut inf, mut nan, mut unsup, mut c2set, mut big) = (0, 0, 0, 0, 0, 0, 0, 0, 0);
    for row in &t.rows {
        let f: Vec<&str> = row.split_whitespace().collect();
        let op: usize = f[0].parse().unwrap();
        let cw = u16::from_str_radix(f[1], 16).unwrap();
        let m = u64::from_str_radix(f[4], 16).unwrap();
        let se = u16::from_str_radix(f[5], 16).unwrap();
        by_op[op] += 1;
        by_rc[op][((cw >> 10) & 3) as usize] += 1;
        let e = se & 0x7fff;
        if e == 0 && m == 0 {
            zero += 1;
        } else if e == 0 {
            den += 1;
        } else if e == 0x7fff && m == 1 << 63 {
            inf += 1;
        } else if e == 0x7fff && m >> 63 == 1 {
            nan += 1;
        } else if (m >> 63) == 0 {
            unsup += 1;
        } else {
            if e < 0x3fff - 70 {
                tiny += 1;
            }
            if m == 1 << 63 && e == 0x3fff {
                one += 1;
            }
            if e >= 0x3fff + 63 {
                big += 1;
            }
        }
        let sw = u16::from_str_radix(f[7], 16).unwrap();
        if op == 1 && sw & 0x400 != 0 {
            c2set += 1;
        }
    }
    eprintln!(
        "reference table: {} rows (f2xm1 {}, fsincos {}); rc counts f2xm1 {:?} fsincos {:?}; zero {zero} one {one} denormal {den} tiny {tiny} inf {inf} nan {nan} unsupported {unsup} >=2^63 {big} (c2=1 rows {c2set}); mismatches {bad}",
        t.rows.len(),
        by_op[0],
        by_op[1],
        by_rc[0],
        by_rc[1]
    );
    assert!(by_op[0] >= 7000 && by_op[1] >= 8000, "table too small: {by_op:?}");
    for op in 0..2 {
        for rc in 0..4 {
            assert!(by_rc[op][rc] >= 1500, "op {op} rc {rc}: only {} rows", by_rc[op][rc]);
        }
    }
    assert!(zero >= 10 && one >= 10 && den >= 100 && tiny >= 100 && inf >= 8 && nan >= 20 && unsup >= 10 && big >= 20 && c2set >= 20, "table lacks a class: zero {zero} one {one} den {den} tiny {tiny} inf {inf} nan {nan} unsup {unsup} big {big} c2 {c2set}");
    assert_eq!(bad, 0, "{bad} of {} rows differ from the reference", t.rows.len());
}

// ---------------------------------------------------------------- (b) host comparison

#[cfg(target_arch = "x86_64")]
const HOST_RUNNER: &str = r#"
static unsigned long long rs = 0x9e3779b97f4a7c15ull;
static unsigned long long rnd(void) { rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17; return rs; }
typedef struct { unsigned long long m; unsigned short se; } f80;
typedef struct { f80 st[8]; unsigned short sw, tw; } out;
#define CLOB : "memory", "st", "st(1)", "st(2)", "st(3)", "st(4)", "st(5)", "st(6)", "st(7)"

static out host(int op, f80 x, unsigned short cw, unsigned short pre_sw) {
    out o; unsigned char save[108], in[10], env[28];
    memcpy(in, &x.m, 8); memcpy(in + 8, &x.se, 2);
    __asm__ volatile("fninit; fnstenv %0" : "=m"(env) :: "memory");
    memcpy(env + 4, &pre_sw, 2);
    __asm__ volatile("fldenv %0; fldcw %1" :: "m"(env), "m"(cw) CLOB);
    if (op == 0) __asm__ volatile("fldt %1; f2xm1; fnsave %0" : "=m"(save) : "m"(in) CLOB);
    else __asm__ volatile("fldt %1; fsincos; fnsave %0" : "=m"(save) : "m"(in) CLOB);
    memcpy(&o.sw, save + 4, 2); memcpy(&o.tw, save + 8, 2);
    for (int i = 0; i < 8; i++) { memcpy(&o.st[i].m, save + 28 + 10 * i, 8); memcpy(&o.st[i].se, save + 36 + 10 * i, 2); }
    return o;
}
static out soft(int op, f80 x, unsigned short cw, unsigned short pre_sw) {
    out o; unsigned char b[10];
    fle_x87_init(); fle_x87.cw = cw;
    memcpy(b, &x.m, 8); memcpy(b + 8, &x.se, 2); fle_x87_fld_m((uint64_t)(uintptr_t)b, 10);
    fle_x87.sw = (unsigned short)((fle_x87.sw & ~0x4700u) | (pre_sw & 0x4700u));
    if (op == 0) c0(); else c1();
    o.sw = fle_x87.sw; o.tw = fle_x87.tw;
    int top = (o.sw >> 11) & 7;
    for (int i = 0; i < 8; i++) { o.st[i].m = fle_x87_r[(top + i) & 7].m; o.st[i].se = fle_x87_r[(top + i) & 7].se; }
    return o;
}
static f80 gen(int op, int cls) {
    f80 x; unsigned long long m = (1ull << 63) | rnd(); unsigned short e; int sg = rnd() & 1;
    switch (cls) {
    case 0: e = op == 0 ? 0x3fff - 1 - (rnd() % 20) : 0x3fff - 8 + (rnd() % 8); break;
    case 1: e = op == 0 ? 0x3fff - 1 - (rnd() % 60) : 0x3fff + (rnd() % 3); break;
    case 2: e = op == 0 ? 0x3fff - 70 + (rnd() % 60) : 0x3fff + 3 + (rnd() % 40); break;
    case 3: e = op == 0 ? 0x3fff - 1 : 0x3fff + 40 + (rnd() % 23); break;
    case 4: e = 0; m &= (rnd() & 1) ? ~(1ull << 63) : ~0ull; break;
    case 5: e = 0x3fff - 16445 + (rnd() % 20); m &= ~(1ull << 63); break;
    default: e = 0x3fff - 30 - (rnd() % 16300); break;
    }
    x.m = m; x.se = (unsigned short)(e | (sg ? 0x8000 : 0)); return x;
}
static unsigned long long ulpd(f80 a, f80 b) {
    if ((a.se ^ b.se) & 0x8000) return 1000000;
    unsigned __int128 A = ((unsigned __int128)(a.se & 0x7fff) << 64) | a.m, B = ((unsigned __int128)(b.se & 0x7fff) << 64) | b.m;
    unsigned __int128 d = A > B ? A - B : B - A;
    return d > 1000000 ? 1000000 : (unsigned long long)d;
}
int main(int argc, char **argv) {
    unsigned N = (unsigned)atoi(argv[1]);
    for (int op = 0; op < 2; op++) {
        for (int cls = 0; cls < 7; cls++) {
            unsigned long long n = 0, rd = 0, maxu = 0, maxn = 0, c1d = 0, strict = 0, twd = 0, topd = 0;
            for (unsigned k = 0; k < N; k++) {
                f80 x = gen(op, cls);
                unsigned short cw = (unsigned short)(0x007f | ((rnd() & 3) << 8) | ((rnd() & 3) << 10));
                unsigned short pre = (unsigned short)(rnd() & 0x4500u);
                out h = host(op, x, cw, pre), c = soft(op, x, cw, pre);
                int nres = op ? ((c.sw & 0x0400) ? 1 : 2) : 1, bad = 0;
                n++;
                for (int i = 0; i < nres; i++) {
                    if (h.st[i].m != c.st[i].m || h.st[i].se != c.st[i].se) {
                        unsigned long long u = ulpd(h.st[i], c.st[i]);
                        bad = 1;
                        if (u > maxu) maxu = u;
                        if (((cw >> 10) & 3) == 0 && u > maxn) maxn = u;
                    }
                }
                rd += bad;
                if ((h.sw ^ c.sw) & 0x0200u) c1d++;
                if ((h.sw ^ c.sw) & ~0x0200u & ~0x3800u & 0x477fu) strict++;
                if (h.tw != c.tw) twd++;
                if ((h.sw ^ c.sw) & 0x3800u) topd++;
            }
            printf("HD %d %d %llu %llu %llu %llu %llu %llu %llu %llu\n", op, cls, n, rd, maxu, maxn, c1d, strict, twd, topd);
        }
    }
    return 0;
}
"#;

#[cfg(target_arch = "x86_64")]
#[test]
fn x87_trans_differs_from_the_host_by_less_than_the_sdm_error_bound() {
    let src = format!("#include <stdlib.h>\n{HEADER}{}{HOST_RUNNER}", bodies());
    let exe = compile("x87_trans_host", &src);
    let n: u32 = std::env::var("X87_TRANS_HOST_N").ok().and_then(|s| s.parse().ok()).unwrap_or(20000);
    let run = Command::new(&exe).arg(n.to_string()).output().expect("run host comparison");
    assert!(run.status.success(), "host comparison failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8");
    let (mut tot, mut diff, mut c1d, mut strict) = (0u64, 0u64, 0u64, 0u64);
    let mut worst = [0u64; 2];
    let mut lines = 0;
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        assert_eq!(f[0], "HD", "{l}");
        let v: Vec<u64> = f[1..].iter().map(|s| s.parse().unwrap()).collect();
        // op cls n differs maxulp maxulp_nearest c1diff strict tagdiff topdiff
        let (op, cls) = (v[0], v[1]);
        lines += 1;
        tot += v[2];
        diff += v[3];
        c1d += v[6];
        strict += v[7];
        worst[0] = worst[0].max(v[4]);
        worst[1] = worst[1].max(v[5]);
        eprintln!("host comparison op {op} class {cls}: n {} result differs {} max ulp steps {} (nearest {}) C1 differs {} other status bits differ {} tag differs {} TOP differs {}", v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9]);
        assert_eq!(v[8], 0, "tag word differs from the host: {l}");
        assert_eq!(v[9], 0, "TOP differs from the host: {l}");
        assert!(v[5] <= 1, "nearest: the host differs by more than 1 ulp step: {l}");
        assert!(v[4] <= 2, "the host differs by more than 2 ulp steps: {l}");
    }
    assert_eq!(lines, 14);
    eprintln!("host comparison total: {tot} runs, {diff} with a different f80 result, max ulp steps {} (nearest {}), C1 differs {c1d}, other status bits differ {strict}", worst[0], worst[1]);
    assert!(diff > 0, "no difference at all: the comparison is probably not running the host instruction");
}

// ---------------------------------------------------------------- (c) named aborts, masked results

const LD: &str = "static void ld(unsigned long long m, unsigned se) { unsigned char b[10]; memcpy(b, &m, 8); memcpy(b + 8, &se, 2); fle_x87_fld_m((uint64_t)(uintptr_t)b, 10); }\n";

fn aborts(name: &str, setup: &str, op: &str, needle: &str) {
    let body = format!("    fle_x87_init();\n    {setup}\n    {op}();\n    printf(\"survived\\n\");");
    let src = format!("{HEADER}{}{LD}int main(void)\n{{\n{body}\n    return 0;\n}}\n", bodies());
    let exe = compile(&format!("x87_trans_{name}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    let out = String::from_utf8_lossy(&run.stdout);
    let err = String::from_utf8_lossy(&run.stderr);
    assert!(!run.status.success() && !out.contains("survived"), "{name}: the run was not stopped (stdout {out:?})");
    assert!(err.contains(needle), "{name}: stderr was {err:?}, expected {needle:?}");
}

#[test]
fn x87_trans_f2xm1_outside_minus_one_to_one_stops_with_a_named_abort() {
    // the SDM leaves the result undefined; the model stops (decision 0043)
    for (name, m, se) in [
        ("one_half_over", "0xC000000000000000ull", "0x3fff"),
        ("two", "0x8000000000000000ull", "0x4000"),
        ("minus_two", "0x8000000000000000ull", "0xc000"),
        ("just_over_one", "0x8000000000000001ull", "0x3fff"),
        ("just_under_minus_one", "0x8000000000000001ull", "0xbfff"),
        ("big", "0x8000000000000000ull", "0x7ffe"),
        ("inf", "0x8000000000000000ull", "0x7fff"),
        ("minus_inf", "0x8000000000000000ull", "0xffff"),
    ] {
        aborts(&format!("f2xm1_range_{name}"), &format!("ld({m}, {se});"), "fle_x87_f2xm1", "fle: x87: F2XM1: operand outside -1.0 .. +1.0");
    }
}

#[test]
fn x87_trans_stack_faults_and_unmasked_exceptions_stop() {
    for op in ["fle_x87_f2xm1", "fle_x87_fsincos"] {
        aborts(&format!("{op}_empty"), "", op, "fle: x87: stack underflow");
    }
    // FSINCOS pushes: a full stack stops (also for |x| >= 2^63: the host reports the overflow first)
    let fill = "{ int k; for (k = 0; k < 7; k++) ld(0x8000000000000000ull, 0x3fff); }";
    aborts("fsincos_full", &format!("{fill} ld(0x8000000000000000ull, 0x3ffe);"), "fle_x87_fsincos", "fle: x87: stack overflow");
    aborts("fsincos_full_big", &format!("{fill} ld(0x8000000000000000ull, 0x403e);"), "fle_x87_fsincos", "fle: x87: stack overflow");
    // PE unmasked (cw 0x035f): every inexact result stops
    aborts("f2xm1_pe", "fle_x87.cw = 0x035f; ld(0x8000000000000000ull, 0x3ffe);", "fle_x87_f2xm1", "fle: x87: unmasked exception");
    aborts("fsincos_pe", "fle_x87.cw = 0x035f; ld(0x8000000000000000ull, 0x3ffe);", "fle_x87_fsincos", "fle: x87: unmasked exception");
    // IE unmasked: SNaN and infinity (FSINCOS)
    aborts("f2xm1_ie", "fle_x87.cw = 0x037e; ld(0xA000000000000001ull, 0x7fff);", "fle_x87_f2xm1", "fle: x87: unmasked exception");
    aborts("fsincos_ie_inf", "fle_x87.cw = 0x037e; ld(0x8000000000000000ull, 0x7fff);", "fle_x87_fsincos", "fle: x87: unmasked exception");
    // DE unmasked: a denormal operand
    aborts("fsincos_de", "fle_x87.cw = 0x037d; ld(0x0000000000000001ull, 0x0000);", "fle_x87_fsincos", "fle: x87: unmasked exception");
    aborts("f2xm1_de", "fle_x87.cw = 0x037d; ld(0x0000000000000001ull, 0x0000);", "fle_x87_f2xm1", "fle: x87: unmasked exception");
    // a pending unmasked flag stops at the instruction
    aborts("f2xm1_pending", "ld(0x8000000000000000ull, 0x3ffe); fle_x87.sw |= 0x20; fle_x87.cw = 0x035f;", "fle_x87_f2xm1", "fle: x87: pending exception flag is unmasked");
}

#[test]
fn x87_trans_out_of_range_fsincos_with_unmasked_pe_still_runs() {
    // |x| >= 2^63: no result, no inexact: even PE unmasked does not stop; C2 = 1, ST(0) unchanged, TOP unchanged
    let body = "    fle_x87_init(); fle_x87.cw = 0x035f;\n    ld(0x8000000000000000ull, 0x403e);\n    c1();\n    printf(\"%04x %04x %016llx:%04x\\n\", (unsigned)fle_x87.sw, (unsigned)fle_x87.tw, (unsigned long long)fle_x87_r[7].m, (unsigned)fle_x87_r[7].se);";
    let src = format!("{HEADER}{}{LD}int main(void)\n{{\n{body}\n    return 0;\n}}\n", bodies());
    let exe = compile("x87_trans_big", &src);
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "3c00 3fff 8000000000000000:403e");
}

// ---------------------------------------------------------------- monotonicity (SDM Vol. 1 8.3.10)

const MONO_RUNNER: &str = r#"
typedef struct { unsigned long long m; unsigned short se; } f80;
/* sign-magnitude to a monotone key */
static unsigned __int128 key(f80 v) {
    unsigned __int128 mag = ((unsigned __int128)(v.se & 0x7fff) << 64) | v.m;
    if (v.se == 0x8000 && v.m == 0) return (unsigned __int128)1 << 100;
    if (v.se == 0 && v.m == 0) return (unsigned __int128)1 << 100;
    return (v.se & 0x8000) ? ((unsigned __int128)1 << 100) - 1 - mag : ((unsigned __int128)1 << 100) + 1 + mag;
}
static f80 rd(int p) { f80 v; v.m = fle_x87_r[p].m; v.se = fle_x87_r[p].se; return v; }
static void run1(int op, f80 x, f80 *a, f80 *b) {
    unsigned char t[10]; int top;
    fle_x87_init();
    memcpy(t, &x.m, 8); memcpy(t + 8, &x.se, 2); fle_x87_fld_m((uint64_t)(uintptr_t)t, 10);
    if (op == 0) c0(); else c1();
    top = (fle_x87.sw >> 11) & 7;
    *a = rd(top); *b = rd((top + 1) & 7);
}
int main(void) {
    /* f2xm1 is increasing on -1..1; sine increasing and cosine decreasing on 0 < x < pi/2 (Pi66/2 = 0x3243F6A8885A308D3 * 2^-65) */
    struct { int op; unsigned long long m; unsigned short se; } S[] = {
        {0, 0xc000000000000000ull, 0xbffe}, {0, 0x8000000000000000ull, 0xbffd}, {0, 0xfffffffffffff000ull, 0xbffe},
        {0, 0x8000000000000000ull, 0x3ffd}, {0, 0xfffffffffffff000ull, 0x3ffe}, {0, 0x8000000000000001ull, 0x0001}, {0, 0x8000000000000000ull, 0xbffe},
        {1, 0xc90fdaa020000000ull, 0x3ffe}, {1, 0xc90fdaa22168c000ull, 0x3ffd}, {1, 0x8000000000000000ull, 0x3ff0}, {1, 0xa000000000000000ull, 0x3ffe},
        {1, 0xc90fdaa22168b000ull, 0x3ffe}, {1, 0x96cbe3f9990e91a7ull, 0x3ffe}, {1, 0xe000000000000000ull, 0x3ffd},
    };
    unsigned long long viol = 0, pts = 0;
    int s;
    for (s = 0; s < (int)(sizeof S / sizeof S[0]); s++) {
        int i, stepbits;
        for (stepbits = 0; stepbits < 3; stepbits++) {
            f80 x, prev_a, prev_b, a, b;
            unsigned long long step = stepbits == 0 ? 1 : stepbits == 1 ? 3 : 4099;
            int have = 0;
            x.m = S[s].m; x.se = S[s].se;
            for (i = 0; i < 6000; i++) {
                unsigned long long nm = x.m + step;
                if (nm < x.m) break; /* stop at the end of the binade */
                x.m = nm;
                run1(S[s].op, x, &a, &b);
                pts++;
                if (have) {
                    /* a larger significand is a larger x when positive and a smaller x when negative */
                    if (S[s].op == 0) { if (S[s].se & 0x8000) { if (key(a) > key(prev_a)) viol++; } else if (key(a) < key(prev_a)) viol++; }
                    else { /* after FSINCOS ST(0) = cos (a), ST(1) = sin (b) */
                        if (key(b) < key(prev_b)) viol++;
                        if (key(a) > key(prev_a)) viol++;
                    }
                }
                prev_a = a; prev_b = b; have = 1;
            }
        }
    }
    printf("%llu %llu\n", pts, viol);
    return 0;
}
"#;

#[test]
fn x87_trans_is_monotonic() {
    let src = format!("{HEADER}{}{MONO_RUNNER}", bodies());
    let exe = compile("x87_trans_mono", &src);
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let v: Vec<u64> = out.split_whitespace().map(|s| s.parse().unwrap()).collect();
    eprintln!("monotonicity: {} points, {} violations", v[0], v[1]);
    assert!(v[0] > 100_000);
    assert_eq!(v[1], 0, "monotonicity violated");
}

// ---------------------------------------------------------------- 256-bit intermediates, rounding window

/// Compiles a C probe that #includes fle_x87.c itself (its static helpers are called directly).
fn compile_internal(name: &str, body: &str) -> PathBuf {
    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let x87c = std::env::var("FLE_X87_C").map(PathBuf::from).unwrap_or_else(|_| rt.join("fle_x87.c"));
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    // no R / FL / X globals here: fle_x87.c has a static X of its own (the exception bits)
    let src = format!("#include \"{}\"\n{body}", x87c.display());
    std::fs::write(&c, src).expect("write C source");
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .arg(rt.join("fle_x87ctl.c"))
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

const BF_PROBE: &str = r#"
static void pbf(const bf *a) {
    int i;
    printf(" %d %ld", a->sign, (long)a->e);
    for (i = BL - 1; i >= 0; i--) printf(" %08x", (unsigned)a->m[i]);
}
static const u128 ARGS[] = {
    (((u128)0x0123456789abcdefull) << 64) | 0xfedcba9876543210ull, (((u128)0xffffffffffffffffull) << 64) | 0xffffffffffffffffull,
    (((u128)0x8000000000000000ull) << 64) | 0x1ull, (((u128)0xc90fdaa22168c234ull) << 64) | 0xc4c6628b80dc1cd1ull,
    (((u128)0x00000000deadbeefull) << 64) | 0x0123456789abcdefull, (((u128)0x9e3779b97f4a7c15ull) << 64) | 0xf39cc0605cedc834ull,
};
int main(void) {
    unsigned k, sg;
    printf("LN2"); pbf(&BF_LN2); printf("\n");
    for (k = 0; k < sizeof ARGS / sizeof ARGS[0]; k++) {
        for (sg = 0; sg < 2; sg++) {
            bf y, s, c, sn;
            /* y = ARGS[k] * 2^-128 (|y| < 1); and a smaller one, y = ARGS[k] * 2^-190 */
            int sh;
            for (sh = 128; sh <= 190; sh += 62) {
                bf_from_u128(&y, (int)sg, ARGS[k], -sh);
                bf_expm1(&s, &y);
                printf("EXPM1 %u %llx%016llx %d |", sg, (unsigned long long)(ARGS[k] >> 64), (unsigned long long)ARGS[k], -sh); pbf(&s); printf("\n");
                bf_sincos(&sn, &c, &y);
                printf("SINCOS %u %llx%016llx %d |", sg, (unsigned long long)(ARGS[k] >> 64), (unsigned long long)ARGS[k], -sh); pbf(&sn); printf(" |"); pbf(&c); printf("\n");
            }
        }
    }
    return 0;
}
"#;

#[test]
fn x87_trans_256_bit_intermediates_match_python_decimal() {
    let exe = compile_internal("x87_trans_bf", BF_PROBE);
    let run = Command::new(&exe).output().expect("run probe");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let check = crate_dir().join("tests/x87_trans/check_bf.py");
    let mut child = Command::new("python3").arg(&check).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().expect("python3");
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&run.stdout).unwrap();
    }
    let out = child.wait_with_output().expect("python3");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "check_bf.py: {}", String::from_utf8_lossy(&out.stderr));
    eprintln!("256-bit intermediates: {}", text.trim());
    assert!(text.starts_with("ok "), "{text}");
}

const WINDOW_PROBE: &str = r#"
static void nofail_probe(void) {}
int main(int argc, char **argv) {
    tv t;
    int up = 0, which = argc > 1 ? atoi(argv[1]) : 0;
    fle_f80 r;
    memset(&t, 0, sizeof t);
    t.v.z = 0; t.v.sign = 0; t.v.e = 0; t.kind = 0;
    /* significand 0x8000000000000000 | one half unit of the 64th bit, then 0 or a 2^20-unit excess */
    t.v.m[7] = 0x80000000u; t.v.m[5] = 0x80000000u;
    if (which == 1) t.v.m[0] = 0x00100000u; /* above the half-way point by 2^20 units of the 256-bit significand: outside the +-2^16 window */
    if (which == 2) t.v.m[0] = 0x00000100u; /* 2^8 units: inside the window */
    if (which == 3) t.v.m[0] = 0x00100000u, t.v.sign = 1;
    r = tv_round(&t, &up, "window: rounding is not decided at 256 bits");
    printf("%016llx %04x %d %u\n", (unsigned long long)r.m, (unsigned)r.se, up, X & 0x3f);
    return 0;
}
"#;

#[test]
fn x87_trans_rounding_window_stops_a_value_too_close_to_a_boundary() {
    let exe = compile_internal("x87_trans_window", WINDOW_PROBE);
    let run = |arg: &str| Command::new(&exe).arg(arg).output().expect("run probe");
    // exactly on the half-way point, and 2^8 units above it: the window straddles the boundary: named abort
    for arg in ["0", "2"] {
        let r = run(arg);
        assert!(!r.status.success(), "case {arg}: a value inside the window of a rounding boundary was rounded");
        assert!(String::from_utf8_lossy(&r.stderr).contains("window: rounding is not decided at 256 bits"), "case {arg}: {:?}", String::from_utf8_lossy(&r.stderr));
    }
    // 2^20 units above it (positive and negative): decided, rounds away from the half-way point
    let r = run("1");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), "8000000000000001 3fff 1 32");
    let r = run("3");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), "8000000000000001 bfff 1 32");
}

// ---------------------------------------------------------------- lifted text

#[test]
fn x87_trans_lifted_text() {
    assert_eq!(lift_body(&F2XM1, 0x1000).unwrap(), "{ void fle_x87_f2xm1(void); fle_x87_f2xm1(); }");
    assert_eq!(lift_body(&FSINCOS, 0x1000).unwrap(), "{ void fle_x87_fsincos(void); fle_x87_fsincos(); }");
}
