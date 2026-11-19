//! Tests for sse_shuf.rs (decision 0027): SSE floating-point shuffles and half moves on XMM
//! registers: SHUFPS, SHUFPD, UNPCKLPS/HPS/LPD/HPD, MOVHLPS, MOVLHPS, MOVLPS/HPS/LPD/HPD.
//!
//! The lifted C (clang -O0, header-only runtime fle_sse.h) and the host CPU run the same
//! instruction on the same state. All sixteen XMM registers, rax, rcx, rdx, rbx and rsi, and
//! 64 bytes of memory must match bit for bit. These instructions change no flags.
//! Inputs: every imm8 (256 values) for SHUFPS and SHUFPD; operands built from edge values
//! (+-0, +-inf, quiet and signalling NaN with payloads, denormals, extremes), random bits and
//! all-distinct dwords; register and 16-byte aligned memory sources; the 64-bit half moves at
//! every displacement from -16 to +40 (aligned and misaligned), load and store.
//! The host instruction is real machine code run on the CPU (not intrinsics).

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
struct Case {
    name: String,
    bytes: Vec<u8>,
    xmm: [[u64; 2]; 16],
    /// rax, rcx, rdx, rbx, rsi
    gpr: [u64; 5],
    /// The memory operand base is rdi = mem + 16, so disp 0 is mem[16..].
    mem: [u8; 64],
}

#[derive(Debug, PartialEq)]
struct Out {
    xmm: [[u64; 2]; 16],
    gpr: [u64; 5],
    mem: [u8; 64],
}

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[derive(Clone, Copy)]
enum Rm {
    /// XMM register 0..15.
    X(u8),
    /// [rdi + disp8].
    Mem(i8),
}

/// [prefix] [REX] 0F opc modrm [disp8]. `reg` is the modrm.reg field.
fn enc(prefix: Option<u8>, opc: &[u8], reg: u8, rm: Rm) -> Vec<u8> {
    let (rm_low, rm_hi) = match rm {
        Rm::X(n) => (n & 7, n >> 3),
        Rm::Mem(_) => (7, 0),
    };
    let rex = 0x40 | (((reg >> 3) & 1) << 2) | rm_hi;
    let mut b: Vec<u8> = prefix.into_iter().collect();
    if rex != 0x40 {
        b.push(rex);
    }
    b.extend_from_slice(opc);
    match rm {
        Rm::X(_) => b.push(0xc0 | ((reg & 7) << 3) | rm_low),
        Rm::Mem(d) => {
            b.push(0x40 | ((reg & 7) << 3) | 7);
            b.push(d as u8);
        }
    }
    b
}

/// Memory is first, so that mem + 16 is 16-byte aligned.
#[repr(C, align(16))]
struct State {
    mem: [u8; 64],
    /// rax, rcx, rdx, rbx, rsi, rdi
    gpr: [u64; 8],
    xmm: [[u64; 2]; 16],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_XMM: usize = std::mem::offset_of!(State, xmm);

/// Host stub: loads the state into the registers, runs `body`, saves the
/// registers back into the state. It keeps rbx and rbp (callee-saved).
fn stub(body: &[u8]) -> Vec<u8> {
    let disp = |o: usize| (o as i32).to_le_bytes();
    // push rbp; push rbx; mov rbp, rdi
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xfd];
    // mov r64, [rbp+disp32]: rax, rcx, rdx, rbx, rsi, rdi
    let modrm = [0x85u8, 0x8d, 0x95, 0x9d, 0xb5, 0xbd];
    for (k, m) in modrm.iter().enumerate() {
        c.extend_from_slice(&[0x48, 0x8b, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    // movdqu xmm_k, [rbp+disp32]
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x6f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_XMM + 16 * k));
    }
    c.extend_from_slice(body);
    // movdqu [rbp+disp32], xmm_k
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x7f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_XMM + 16 * k));
    }
    // mov [rbp+disp32], r64
    for (k, m) in modrm.iter().enumerate() {
        c.extend_from_slice(&[0x48, 0x89, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    // pop rbx; pop rbp; ret
    c.extend_from_slice(&[0x5b, 0x5d, 0xc3]);
    c
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

/// Runs the case on the host CPU.
fn run_host(c: &Case) -> Out {
    let mut st = Box::new(State { mem: c.mem, gpr: [0; 8], xmm: c.xmm });
    st.gpr[..5].copy_from_slice(&c.gpr);
    st.gpr[5] = st.mem.as_ptr() as u64 + 16;
    let code = stub(&c.bytes);
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(p);
        f(&mut *st);
        munmap(p, 4096);
    }
    Out { xmm: st.xmm, gpr: st.gpr[..5].try_into().unwrap(), mem: st.mem }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-shuf/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// Compiles `src` with clang -O0 into out-shuf/test, and returns the path. The runtime is
/// header-only for these instructions (fle_sse.h includes fle_sse_shuf.h).
fn compile(name: &str, src: &str) -> PathBuf {
    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).expect("write C source");
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

/// Builds one C program that runs every case, and returns its parsed stdout.
fn c_run(group: &str, cs: &[Case]) -> Vec<Out> {
    let mut src = String::from(HEADER);
    for (k, c) in cs.iter().enumerate() {
        let body = lift_body(&c.bytes, 0x1000).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "struct In { uint64_t xmm[16][2]; uint64_t gpr[5]; uint8_t mem[64]; };\n";
    src += "static const struct In IN[] = {\n";
    for c in cs {
        let xmm: Vec<String> = c.xmm.iter().map(|p| format!("{{ {}ULL, {}ULL }}", p[0], p[1])).collect();
        let gpr: Vec<String> = c.gpr.iter().map(|g| format!("{g}ULL")).collect();
        let mem: Vec<String> = c.mem.iter().map(|b| b.to_string()).collect();
        src += &format!("{{ {{ {} }}, {{ {} }}, {{ {} }} }},\n", xmm.join(", "), gpr.join(", "), mem.join(", "));
    }
    src += "};\n";
    src += "static void (*const FN[])(void) = {";
    for k in 0..cs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n\n";
    src += r#"int main(void)
{
    unsigned k, j;
    for (k = 0; k < NCASES; k++) {
        for (j = 0; j < 16; j++) { X[j].u64[0] = IN[k].xmm[j][0]; X[j].u64[1] = IN[k].xmm[j][1]; }
        R[0] = IN[k].gpr[0]; R[1] = IN[k].gpr[1]; R[2] = IN[k].gpr[2]; R[3] = IN[k].gpr[3]; R[6] = IN[k].gpr[4];
        R[7] = (uint64_t)(uintptr_t)(M + 16);
        memcpy(M, IN[k].mem, sizeof M);
        FN[k]();
        printf("%u", k);
        for (j = 0; j < 16; j++) printf(" %llx %llx", (unsigned long long)X[j].u64[0], (unsigned long long)X[j].u64[1]);
        printf(" %llx %llx %llx %llx %llx", (unsigned long long)R[0], (unsigned long long)R[1], (unsigned long long)R[2], (unsigned long long)R[3], (unsigned long long)R[6]);
        for (j = 0; j < 64; j++) printf(" %02x", M[j]);
        printf("\n");
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());

    let exe = compile(&format!("sse_shuf_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8 output");

    let hex = |s: &str| u64::from_str_radix(s, 16).expect("hex");
    text.lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut p = 1;
            let mut xmm = [[0u64; 2]; 16];
            for x in xmm.iter_mut() {
                x[0] = hex(t[p]);
                x[1] = hex(t[p + 1]);
                p += 2;
            }
            let mut gpr = [0u64; 5];
            for g in gpr.iter_mut() {
                *g = hex(t[p]);
                p += 1;
            }
            let mut mem = [0u8; 64];
            for m in mem.iter_mut() {
                *m = u8::from_str_radix(t[p], 16).expect("byte");
                p += 1;
            }
            Out { xmm, gpr, mem }
        })
        .collect()
}

fn check(group: &str, cs: &[Case]) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h.xmm != case.xmm || h.mem != case.mem {
            changed += 1;
        }
        if h != *cout {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?}\n  in   {:x?}\n  C    {:x?} {:x?} {:?}\n  host {:x?} {:x?} {:?}",
                    case.name, case.bytes, case.xmm, cout.xmm, cout.gpr, cout.mem, h.xmm, h.gpr, h.mem
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed_xmm {changed}", cs.len());
    // the instruction really ran on the host and changed a register or memory in most cases
    assert!(changed * 10 >= cs.len() * 8, "{group}: only {changed} of {} cases changed an XMM register or memory", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
}

fn put128(m: &mut [u8; 64], off: usize, v: [u64; 2]) {
    m[off..off + 8].copy_from_slice(&v[0].to_le_bytes());
    m[off + 8..off + 16].copy_from_slice(&v[1].to_le_bytes());
}

fn base(name: String, bytes: Vec<u8>, rng: &mut Rng) -> Case {
    let mut xmm = [[0u64; 2]; 16];
    for x in xmm.iter_mut() {
        x[0] = rng.next_u64();
        x[1] = rng.next_u64();
    }
    let mut gpr = [0u64; 5];
    for g in gpr.iter_mut() {
        *g = rng.next_u64();
    }
    let mut mem = [0u8; 64];
    for b in mem.iter_mut() {
        *b = rng.next_u64() as u8;
    }
    Case { name, bytes, xmm, gpr, mem }
}

/// Single precision edge values: +-0, +-1, +-inf, quiet and signalling NaNs with payloads, the
/// smallest denormals, the largest finite values.
const POOL32: [u32; 20] = [
    0x0000_0000, 0x8000_0000, 0x3f80_0000, 0xbf80_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 0xffc0_0000,
    0x7f80_0001, 0xff80_0001, 0x7fa0_0000, 0xffbf_ffff, 0x7fff_ffff, 0x0000_0001, 0x8000_0001, 0x007f_ffff,
    0x0080_0000, 0x7f7f_ffff, 0xff7f_ffff, 0x4049_0fdb,
];

/// Double precision edge values, same classes.
const POOL64: [u64; 16] = [
    0x0000_0000_0000_0000, 0x8000_0000_0000_0000, 0x3ff0_0000_0000_0000, 0xbff0_0000_0000_0000,
    0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000, 0x7ff8_0000_0000_0000, 0xfff8_0000_0000_0000,
    0x7ff0_0000_0000_0001, 0xfff0_0000_0000_0001, 0x7ff4_0000_0000_0000, 0x7fff_ffff_ffff_ffff,
    0x0000_0000_0000_0001, 0x000f_ffff_ffff_ffff, 0x7fef_ffff_ffff_ffff, 0x4009_21fb_5444_2d18,
];

/// A 128-bit value whose lanes are edge values or random bits. Four dwords get one of the three
/// treatments at random: single pool, double pool for a whole qword, or random; so that the
/// four dwords differ (a wrong lane shows up) and NaN/zero lanes are common.
fn fp_value(rng: &mut Rng) -> [u64; 2] {
    let mut v = [0u64; 2];
    for q in 0..2 {
        v[q] = match rng.next_u64() % 4 {
            0 => POOL64[(rng.next_u64() % POOL64.len() as u64) as usize],
            1 => rng.next_u64(),
            _ => {
                let lo = POOL32[(rng.next_u64() % POOL32.len() as u64) as usize] as u64;
                let hi = if rng.next_u64() & 1 == 0 { POOL32[(rng.next_u64() % POOL32.len() as u64) as usize] as u64 } else { rng.next_u64() & 0xffff_ffff };
                lo | (hi << 32)
            }
        };
    }
    v
}

/// Distinct dwords (no equal lanes), so that every selector shows up in the output.
fn distinct_value(rng: &mut Rng) -> [u64; 2] {
    [(rng.next_u64() & 0x0fff_ffff_0fff_ffff) | 0x1000_0000_2000_0000, (rng.next_u64() & 0x0fff_ffff_0fff_ffff) | 0x3000_0000_4000_0000]
}

/// `op xmm_d, xmm_s [, imm]` and `op xmm_d, [rdi + disp] [, imm]` (memory aligned: disp is a
/// multiple of 16). `imms`: the immediates to run (empty: no immediate byte).
fn pair_group(name: &str, prefix: Option<u8>, opc: &[u8], imms: &[u8], n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    let mut add = |out: &mut Vec<Case>, rng: &mut Rng, imm: Option<u8>, k: usize| {
        let d = (rng.next_u64() % 16) as u8;
        let s = if k % 7 == 0 { d } else { (rng.next_u64() % 16) as u8 };
        let mut bytes = enc(prefix, opc, d, Rm::X(s));
        if let Some(i) = imm {
            bytes.push(i);
        }
        let mut c = base(format!("{name} x{d}, x{s} imm {imm:?} #{k}"), bytes, rng);
        let (vd, vs) = if k % 3 == 0 { (distinct_value(rng), distinct_value(rng)) } else { (fp_value(rng), fp_value(rng)) };
        c.xmm[d as usize] = vd;
        if s != d {
            c.xmm[s as usize] = vs;
        }
        out.push(c);
        let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
        let mut bytes = enc(prefix, opc, d, Rm::Mem(disp));
        if let Some(i) = imm {
            bytes.push(i);
        }
        let mut m = base(format!("{name} x{d}, [rdi{disp:+}] imm {imm:?} #{k}"), bytes, rng);
        m.xmm[d as usize] = if k % 3 == 0 { distinct_value(rng) } else { fp_value(rng) };
        put128(&mut m.mem, (disp as i32 + 16) as usize, if k % 3 == 0 { distinct_value(rng) } else { fp_value(rng) });
        out.push(m);
    };
    if imms.is_empty() {
        for k in 0..n {
            add(&mut out, &mut rng, None, k);
        }
    } else {
        for (k, &imm) in imms.iter().enumerate() {
            add(&mut out, &mut rng, Some(imm), k);
        }
        // more random operands for the same immediates
        for k in 0..n {
            let imm = imms[(rng.next_u64() % imms.len() as u64) as usize];
            add(&mut out, &mut rng, Some(imm), k + imms.len());
        }
    }
    out
}

/// MOVHLPS / MOVLHPS: register only.
fn regreg_group(name: &str, opc: u8, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..n {
        let d = (rng.next_u64() % 16) as u8;
        let s = if k % 7 == 0 { d } else { (rng.next_u64() % 16) as u8 };
        let mut c = base(format!("{name} x{d}, x{s} #{k}"), enc(None, &[0x0F, opc], d, Rm::X(s)), &mut rng);
        c.xmm[d as usize] = if k % 3 == 0 { distinct_value(&mut rng) } else { fp_value(&mut rng) };
        if s != d {
            c.xmm[s as usize] = if k % 3 == 0 { distinct_value(&mut rng) } else { fp_value(&mut rng) };
        }
        out.push(c);
    }
    out
}

/// MOVLPS/MOVHPS/MOVLPD/MOVHPD, load (`load_opc`) and store (`load_opc + 1`) with an m64 operand
/// at every displacement from -16 to +40 (aligned and misaligned, within the 64-byte buffer).
fn half_group(name: &str, prefix: Option<u8>, load_opc: u8, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..n {
        let d = (rng.next_u64() % 16) as u8;
        let disp = (k as i32 % 57 - 16) as i8;
        for (kind, opc) in [("load", load_opc), ("store", load_opc + 1)] {
            let mut c = base(format!("{name} {kind} x{d}, [rdi{disp:+}] #{k}"), enc(prefix, &[0x0F, opc], d, Rm::Mem(disp)), &mut rng);
            c.xmm[d as usize] = fp_value(&mut rng);
            put128(&mut c.mem, 0, fp_value(&mut rng));
            put128(&mut c.mem, 16, fp_value(&mut rng));
            put128(&mut c.mem, 32, fp_value(&mut rng));
            put128(&mut c.mem, 48, fp_value(&mut rng));
            out.push(c);
        }
    }
    out
}

fn all_imms() -> Vec<u8> {
    (0..=255u8).collect()
}

macro_rules! pair_test {
    ($fn:ident, $name:expr, $prefix:expr, $opc:expr, $imms:expr, $n:expr, $seed:expr) => {
        #[test]
        fn $fn() {
            check($name, &pair_group($name, $prefix, &[0x0F, $opc], &$imms, $n, $seed));
        }
    };
}

pair_test!(shufps_matches_host, "shufps", None, 0xC6, all_imms(), 300, 0x4001);
pair_test!(shufpd_matches_host, "shufpd", Some(0x66), 0xC6, all_imms(), 300, 0x4002);
pair_test!(unpcklps_matches_host, "unpcklps", None, 0x14, [], 400, 0x4003);
pair_test!(unpckhps_matches_host, "unpckhps", None, 0x15, [], 400, 0x4004);
pair_test!(unpcklpd_matches_host, "unpcklpd", Some(0x66), 0x14, [], 400, 0x4005);
pair_test!(unpckhpd_matches_host, "unpckhpd", Some(0x66), 0x15, [], 400, 0x4006);

#[test]
fn movhlps_matches_host() {
    check("movhlps", &regreg_group("movhlps", 0x12, 600, 0x5001));
}

#[test]
fn movlhps_matches_host() {
    check("movlhps", &regreg_group("movlhps", 0x16, 600, 0x5002));
}

#[test]
fn movlps_matches_host() {
    check("movlps", &half_group("movlps", None, 0x12, 171, 0x6001));
}

#[test]
fn movhps_matches_host() {
    check("movhps", &half_group("movhps", None, 0x16, 171, 0x6002));
}

#[test]
fn movlpd_matches_host() {
    check("movlpd", &half_group("movlpd", Some(0x66), 0x12, 171, 0x6003));
}

#[test]
fn movhpd_matches_host() {
    check("movhpd", &half_group("movhpd", Some(0x66), 0x16, 171, 0x6004));
}

/// The register encodings found in the closure's disassembly (exact bytes), run against the
/// host. The closure's memory forms use rcx, rdx and rbx as the base register; the lifting of
/// the base register is the same code as for rdi, which the memory tests above run.
#[test]
fn closure_register_encodings_match_host() {
    let encs: [(&str, &[u8]); 9] = [
        ("movlhps xmm2,xmm2", &[0x0f, 0x16, 0xd2]),
        ("movhlps xmm2,xmm0", &[0x0f, 0x12, 0xd0]),
        ("movhlps xmm3,xmm1", &[0x0f, 0x12, 0xd9]),
        ("movlhps xmm0,xmm1", &[0x0f, 0x16, 0xc1]),
        ("shufps xmm0,xmm0,1", &[0x0f, 0xc6, 0xc0, 0x01]),
        ("shufps xmm0,xmm0,0", &[0x0f, 0xc6, 0xc0, 0x00]),
        ("shufps xmm3,xmm3,1", &[0x0f, 0xc6, 0xdb, 0x01]),
        ("shufps xmm2,xmm2,0", &[0x0f, 0xc6, 0xd2, 0x00]),
        ("shufps xmm1,xmm1,1", &[0x0f, 0xc6, 0xc9, 0x01]),
    ];
    let mut rng = Rng(0x7001);
    let mut cs = Vec::new();
    for (name, bytes) in encs {
        for k in 0..40 {
            let mut c = base(format!("{name} #{k}"), bytes.to_vec(), &mut rng);
            for x in c.xmm.iter_mut() {
                *x = fp_value(&mut rng);
            }
            cs.push(c);
        }
    }
    check("closure-register-forms", &cs);
}

#[test]
fn case_sets_cover_the_edges() {
    assert_eq!(all_imms().len(), 256);
    assert!(pair_group("shufps", None, &[0x0F, 0xC6], &all_imms(), 0, 1).len() >= 512);
    // the value pools hold +-0, quiet and signalling NaN, +-inf for both widths
    for e in [0u32, 0x8000_0000, 0x7fc0_0000, 0x7f80_0001, 0x7f80_0000, 0xff80_0000] {
        assert!(POOL32.contains(&e), "POOL32 lacks {e:#x}");
    }
    for e in [0u64, 0x8000_0000_0000_0000, 0x7ff8_0000_0000_0000, 0x7ff0_0000_0000_0001, 0x7ff0_0000_0000_0000] {
        assert!(POOL64.contains(&e), "POOL64 lacks {e:#x}");
    }
    // the half-move displacements include misaligned ones
    let h = half_group("movlps", None, 0x12, 171, 1);
    assert!(h.iter().any(|c| c.bytes.last().map_or(false, |&d| d & 7 != 0 && c.bytes[1] == 0x12)));
}

/// A memory operand that is not 16-byte aligned is an error for SHUFPS, SHUFPD and UNPCK*
/// (SDM Type 4, #GP(0)): the generated C aborts with the 16-byte message. The host would
/// raise #GP, so only the C side is run.
#[test]
fn misaligned_m128_operand_aborts() {
    for (name, bytes) in [
        ("shufps", {
            let mut b = enc(None, &[0x0F, 0xC6], 0, Rm::Mem(8));
            b.push(0x1b);
            b
        }),
        ("shufpd", {
            let mut b = enc(Some(0x66), &[0x0F, 0xC6], 3, Rm::Mem(1));
            b.push(0x02);
            b
        }),
        ("unpcklps", enc(None, &[0x0F, 0x14], 0, Rm::Mem(4))),
        ("unpckhps", enc(None, &[0x0F, 0x15], 2, Rm::Mem(-8))),
        ("unpcklpd", enc(Some(0x66), &[0x0F, 0x14], 1, Rm::Mem(2))),
        ("unpckhpd", enc(Some(0x66), &[0x0F, 0x15], 5, Rm::Mem(15))),
    ] {
        let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = compile(&format!("sse_shuf_misaligned_{name}"), &src);
        let run = Command::new(&exe).output().expect("run test binary");
        assert!(!run.status.success(), "{name}: a misaligned operand must abort");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(err.contains("misaligned 16-byte access"), "{name}: stderr {err}");
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}: returned");
    }
}

/// The 64-bit half moves accept a misaligned address (SDM Type 5: no alignment #GP): the
/// generated C does not abort (the host side of this is the matching tests above, which use
/// every displacement).
#[test]
fn half_moves_accept_misaligned_addresses() {
    for (name, prefix, opc) in [("movlps", None, 0x12u8), ("movhps", None, 0x16), ("movlpd", Some(0x66), 0x12), ("movhpd", Some(0x66), 0x16)] {
        for opc in [opc, opc + 1] {
            let bytes = enc(prefix, &[0x0F, opc], 1, Rm::Mem(3));
            let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!body.contains("fle_ldxa") && !body.contains("fle_stxa"), "{name}: {body}");
            let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
            let exe = compile(&format!("sse_shuf_half_{name}_{opc:x}"), &src);
            let run = Command::new(&exe).output().expect("run test binary");
            assert!(run.status.success(), "{name} {opc:#x}: {}", String::from_utf8_lossy(&run.stderr));
        }
    }
}

/// The VEX forms stay unsupported (not part of this unit).
#[test]
fn vex_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 5] = [
        ("vshufps xmm0, xmm1, xmm2, 0x1b", &[0xC5, 0xF0, 0xC6, 0xC2, 0x1B]),
        ("vmovlps xmm0, xmm1, [rax]", &[0xC5, 0xF0, 0x12, 0x00]),
        ("vmovhlps xmm0, xmm1, xmm2", &[0xC5, 0xF0, 0x12, 0xC2]),
        ("vunpcklps xmm0, xmm1, xmm2", &[0xC5, 0xF0, 0x14, 0xC2]),
        ("vmovlhps xmm0, xmm1, xmm2", &[0xC5, 0xF0, 0x16, 0xC2]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}
