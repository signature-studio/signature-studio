//! Tests for sse_fp_flags.rs and prefetch.rs (decision 0038): MXCSR exception flags, RC, FTZ and DAZ for the SSE
//! floating-point instructions, and PREFETCHh.
//!
//! Method as in sse_int2.rs / sse_scalar.rs: the lifted C (`lift_body`, clang -O0, header-only runtime) and the host
//! CPU run the same instruction (real machine code in an mmap'd stub, not intrinsics) on the same state. Compared
//! bit for bit: the XMM registers the case sets, the GPRs, the flags of (U)COMIS*, the XMM registers and memory the
//! instruction must not touch, and MXCSR (status bits and control bits) after the instruction.
//!
//! The host stub sets MXCSR from the case before the instruction, reads it back after, and restores the MXCSR the
//! thread had (read at the start of the stub). The C side sets `fle_mxcsr` from the case. Inputs: NaN (quiet, signaling,
//! with payload, negative), +-0, +-inf, subnormals, the largest and smallest normals, overflow and underflow
//! boundaries, ties, inexact values, integer edges, lane-wise random mixes; every RC, FTZ and DAZ combination;
//! sticky flags already set before the instruction.
//!
//! A second set of cases checks the flags against the SDM (Vol. 2 "SIMD Floating-Point Exceptions" sections) with
//! values written out by hand, on the C side only. A third checks that a raised flag unmasked in the guest MXCSR is a
//! named abort, and that PREFETCHh never faults and changes nothing.

mod etxt_common;

use fe_lift::lift_body;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

/// XMM registers that cases set; the others hold fixed values and must not change.
const SP: [usize; 5] = [0, 1, 2, 3, 9];

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[derive(Clone, Copy)]
enum Rm {
    X(u8),
    G(u8),
    Mem,
}

/// [prefix] [REX] opc modrm [imm8]. The memory form is [rdi].
fn enc(prefix: Option<u8>, w: bool, opc: &[u8], reg: u8, rm: Rm, imm: Option<u8>) -> Vec<u8> {
    let (rm_low, rm_hi) = match rm {
        Rm::X(n) | Rm::G(n) => (n & 7, n >> 3),
        Rm::Mem => (7, 0),
    };
    let rex = 0x40 | ((w as u8) << 3) | (((reg >> 3) & 1) << 2) | rm_hi;
    let mut b: Vec<u8> = prefix.into_iter().collect();
    if rex != 0x40 {
        b.push(rex);
    }
    b.extend_from_slice(opc);
    match rm {
        Rm::X(_) | Rm::G(_) => b.push(0xc0 | ((reg & 7) << 3) | rm_low),
        Rm::Mem => b.push(((reg & 7) << 3) | 7),
    }
    b.extend(imm);
    b
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    F32,
    F64,
    I32,
}

struct Enc {
    name: String,
    bytes: Vec<u8>,
    comi: bool,
    /// kind of the destination and source values, and whether the instruction is scalar (low element only)
    dk: Kind,
    sk: Kind,
    scalar: bool,
}

#[derive(Clone)]
struct Case {
    enc: usize,
    xmm: [[u64; 2]; 5],
    gpr: [u64; 5],
    /// mem[16..32], the memory operand
    mem: [u64; 2],
    mx: u32,
}

#[derive(Debug, PartialEq, Clone)]
struct Out {
    xmm: [[u64; 2]; 5],
    gpr: [u64; 5],
    mx: u32,
    flags: u64,
    other_ok: bool,
}

fn pool32() -> Vec<u64> {
    [
        0x0000_0000u32, 0x8000_0000, 0x3f80_0000, 0xbf80_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 0xffc0_0000,
        0x7f80_0001, 0x7fa0_0000, 0xff80_0001, 0x7fc1_2345, 0x0000_0001, 0x007f_ffff, 0x8000_0001, 0x0080_0000,
        0x0080_0001, 0x7f7f_ffff, 0xff7f_ffff, 0x7f7f_fffe, 0x3f00_0000, 0x3fc0_0000, 0x4020_0000, 0x4040_0000,
        0x3eaa_aaab, 0x3f80_0001, 0x4f00_0000, 0xcf00_0000, 0x4b80_0000, 0x4b00_0001, 0x3380_0000, 0x3400_0000,
        0x1f80_0000, 0x2000_0000, 0x5f00_0000, 0x7e96_7699, 0x4effffff, 0xceffffff, 0x4b7fffff, 0x3f7f_ffff,
    ]
    .iter()
    .map(|&v| v as u64)
    .collect()
}

fn pool64() -> Vec<u64> {
    vec![
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0x7ff4_0000_0000_0000,
        0xfff0_0000_0000_0001,
        0x7ff8_0000_1234_5678,
        0x0000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        0x8000_0000_0000_0001,
        0x0010_0000_0000_0000,
        0x0010_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x7fef_ffff_ffff_fffe,
        0x3fe0_0000_0000_0000,
        0x3ff8_0000_0000_0000,
        0x4004_0000_0000_0000,
        0x4008_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        0x3ff0_0000_0000_0001,
        0x41e0_0000_0000_0000,
        0xc1e0_0000_0000_0000,
        0x43e0_0000_0000_0000,
        0xc3e0_0000_0000_0000,
        0x4340_0000_0000_0000,
        0x4330_0000_0000_0001,
        0x3ca0_0000_0000_0000,
        0x3cb0_0000_0000_0000,
        0x1ff0_0000_0000_0000,
        0x2000_0000_0000_0000,
        0x5fe0_0000_0000_0000,
        0x47ef_ffff_e000_0000,
        0x47f0_0000_0000_0000,
        0x36a0_0000_0000_0000,
        0x41df_ffff_ffc0_0000,
        0xc1df_ffff_ffc0_0000,
        0x3eff_ffff_ffff_ffff,
        0x7e37_e43c_8800_759c,
    ]
}

fn pool_i32() -> Vec<u64> {
    [0u32, 1, 2, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, 0x0100_0001, 0x0100_0000, 0xff00_0001, 0x00ff_ffff, 0x1234_5679]
        .iter()
        .map(|&v| v as u64)
        .collect()
}

fn pool_i64() -> Vec<u64> {
    vec![
        0, 1, u64::MAX, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff_8000_0000, 0x7fff_ffff_ffff_ffff, 0x8000_0000_0000_0000,
        0x0020_0000_0000_0001, 0x0020_0000_0000_0000, 0x0100_0000, 0x0100_0001, 0xffe0_0000_0000_0001, 0x1fff_ffff_ffff_ffff,
    ]
}

fn pick(rng: &mut Rng, p: &[u64]) -> u64 {
    p[rng.below(p.len())]
}

fn lane_pool(k: Kind) -> Vec<u64> {
    match k {
        Kind::F32 => pool32(),
        Kind::F64 => pool64(),
        Kind::I32 => pool_i32(),
    }
}

/// Fill an XMM value: scalar sets the low element and leaves random bits above; packed sets every lane.
/// Three quarters of the lanes come from the edge pool, the rest are random.
fn fill(x: &mut [u64; 2], k: Kind, scalar: bool, rng: &mut Rng) {
    let p = lane_pool(k);
    x[0] = rng.next_u64();
    x[1] = rng.next_u64();
    let lanes = if k == Kind::F64 { 2 } else { 4 };
    let w = if k == Kind::F64 { 64 } else { 32 };
    for l in 0..(if scalar { 1 } else { lanes }) {
        let v = if rng.below(4) != 0 { pick(rng, &p) } else if w == 64 { rng.next_u64() } else { rng.next_u64() & 0xffff_ffff };
        if w == 64 {
            x[l] = v;
        } else {
            let (word, sh) = (l / 2, (l % 2) * 32);
            x[word] = (x[word] & !(0xffff_ffffu64 << sh)) | (v << sh);
        }
    }
}

fn set_lo(x: &mut [u64; 2], k: Kind, v: u64) {
    if k == Kind::F64 {
        x[0] = v;
    } else {
        x[0] = (x[0] & !0xffff_ffffu64) | (v & 0xffff_ffff);
    }
}

const fn ctl_bits(rc: u32, ftz: bool, daz: bool) -> u32 {
    0x1f80 | (rc << 13) | ((ftz as u32) << 15) | ((daz as u32) << 6)
}

fn random_mx(rng: &mut Rng) -> u32 {
    let c = rng.below(16) as u32;
    let mut m = ctl_bits(c & 3, c & 4 != 0, c & 8 != 0);
    if rng.below(4) == 0 {
        m |= (rng.next_u64() & 0x3f) as u32;
    }
    m
}

// ---------------------------------------------------------------- host

#[repr(C, align(16))]
struct State {
    mem: [u8; 64],
    gpr: [u64; 8],
    xmm: [[u64; 2]; 16],
    fl: u64,
    /// in, out, saved, pad
    mx: [u32; 4],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_XMM: usize = std::mem::offset_of!(State, xmm);
const OFF_FL: usize = std::mem::offset_of!(State, fl);
const OFF_MX: usize = std::mem::offset_of!(State, mx);

fn stub(body: &[u8]) -> Vec<u8> {
    let disp = |o: usize| (o as i32).to_le_bytes();
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xfd]; // push rbp; push rbx; mov rbp, rdi
    c.extend_from_slice(&[0x0f, 0xae, 0x9d]); // stmxcsr [rbp+saved]
    c.extend_from_slice(&disp(OFF_MX + 8));
    let modrm = [0x85u8, 0x8d, 0x95, 0x9d, 0xb5, 0xbd]; // rax rcx rdx rbx rsi rdi
    for (k, m) in modrm.iter().enumerate() {
        c.extend_from_slice(&[0x48, 0x8b, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x6f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_XMM + 16 * k));
    }
    c.extend_from_slice(&[0x0f, 0xae, 0x95]); // ldmxcsr [rbp+in]
    c.extend_from_slice(&disp(OFF_MX));
    c.extend_from_slice(body);
    c.extend_from_slice(&[0x9c, 0x41, 0x58]); // pushfq; pop r8
    c.extend_from_slice(&[0x4c, 0x89, 0x85]); // mov [rbp+fl], r8
    c.extend_from_slice(&disp(OFF_FL));
    c.extend_from_slice(&[0x0f, 0xae, 0x9d]); // stmxcsr [rbp+out]
    c.extend_from_slice(&disp(OFF_MX + 4));
    c.extend_from_slice(&[0x0f, 0xae, 0x95]); // ldmxcsr [rbp+saved]
    c.extend_from_slice(&disp(OFF_MX + 8));
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x7f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_XMM + 16 * k));
    }
    for (k, m) in modrm.iter().enumerate() {
        c.extend_from_slice(&[0x48, 0x89, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    c.extend_from_slice(&[0x5b, 0x5d, 0xc3]);
    c
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

struct Fixed {
    xmm: [[u64; 2]; 16],
    mem: [u8; 64],
}

fn fixed() -> Fixed {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut xmm = [[0u64; 2]; 16];
    for x in xmm.iter_mut() {
        x[0] = rng.next_u64();
        x[1] = rng.next_u64();
    }
    let mut mem = [0u8; 64];
    for b in mem.iter_mut() {
        *b = rng.next_u64() as u8;
    }
    Fixed { xmm, mem }
}

const FLAG_MASK: u64 = 0x8d5;

fn run_host_group(e: &Enc, cs: &[&Case]) -> Vec<Out> {
    let fx = fixed();
    let code = stub(&e.bytes);
    let mut outs = Vec::new();
    unsafe {
        let p = mmap(std::ptr::null_mut(), 8192, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(p);
        for c in cs {
            let mut st = Box::new(State { mem: fx.mem, gpr: [0; 8], xmm: fx.xmm, fl: 0, mx: [c.mx, 0, 0, 0] });
            st.mem[16..24].copy_from_slice(&c.mem[0].to_le_bytes());
            st.mem[24..32].copy_from_slice(&c.mem[1].to_le_bytes());
            let init_mem = st.mem;
            st.gpr[..5].copy_from_slice(&c.gpr);
            st.gpr[5] = st.mem.as_ptr() as u64 + 16;
            for (k, &r) in SP.iter().enumerate() {
                st.xmm[r] = c.xmm[k];
            }
            f(&mut *st);
            let mut other_ok = st.mem == init_mem;
            for r in 0..16 {
                if !SP.contains(&r) && st.xmm[r] != fx.xmm[r] {
                    other_ok = false;
                }
            }
            let mut xmm = [[0u64; 2]; 5];
            for (k, &r) in SP.iter().enumerate() {
                xmm[k] = st.xmm[r];
            }
            outs.push(Out {
                xmm,
                gpr: st.gpr[..5].try_into().unwrap(),
                mx: st.mx[1],
                flags: if e.comi { st.fl & FLAG_MASK } else { 0 },
                other_ok,
            });
        }
        munmap(p, 8192);
    }
    outs
}

// ---------------------------------------------------------------- C side

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-mxsoft/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// The runtime directory: runtime/, or the one named by MXCSR_MUTANT_RT (mutation testing).
fn rt_dir() -> PathBuf {
    match std::env::var("MXCSR_MUTANT_RT") {
        Ok(p) => PathBuf::from(p),
        Err(_) => crate_dir().join("../../runtime"),
    }
}

fn compile(name: &str, src: &str) -> PathBuf {
    let dir = out_dir();
    let rt = rt_dir();
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
        .expect("run clang");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_mxcsr.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).expect("hex")
}

fn c_run(group: &str, encs: &[Enc], cs: &[Case]) -> Vec<Out> {
    let fx = fixed();
    let mut src = String::from(HEADER);
    for (k, e) in encs.iter().enumerate() {
        let body = lift_body(&e.bytes, 0x1000).unwrap_or_else(|er| panic!("{}: {er}", e.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "struct In { unsigned enc; unsigned mx; uint64_t xmm[5][2]; uint64_t gpr[5]; uint64_t mem[2]; };\n";
    src += "static const struct In IN[] = {\n";
    for c in cs {
        let xmm: Vec<String> = c.xmm.iter().map(|p| format!("{{{}ULL,{}ULL}}", p[0], p[1])).collect();
        let gpr: Vec<String> = c.gpr.iter().map(|g| format!("{g}ULL")).collect();
        src += &format!("{{{},{}u,{{{}}},{{{}}},{{{}ULL,{}ULL}}}},\n", c.enc, c.mx, xmm.join(","), gpr.join(","), c.mem[0], c.mem[1]);
    }
    src += "};\n";
    let bx: Vec<String> = fx.xmm.iter().map(|p| format!("{{{}ULL,{}ULL}}", p[0], p[1])).collect();
    src += &format!("static const uint64_t BASEX[16][2] = {{{}}};\n", bx.join(","));
    let bm: Vec<String> = fx.mem.iter().map(|b| b.to_string()).collect();
    src += &format!("static const uint8_t BASEM[64] = {{{}}};\n", bm.join(","));
    src += "static void (*const FN[])(void) = {";
    for k in 0..encs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n";
    let comi: Vec<String> = encs.iter().map(|e| (e.comi as u8).to_string()).collect();
    src += &format!("static const uint8_t COMI[] = {{{}}};\n", comi.join(","));
    src += "static const int SP[5] = {0,1,2,3,9};\n";
    src += r#"
static uint64_t flagbits(void) {
    return ((uint64_t)FL.cf << 0) | ((uint64_t)FL.pf << 2) | ((uint64_t)FL.af << 4) | ((uint64_t)FL.zf << 6) |
           ((uint64_t)FL.sf << 7) | ((uint64_t)FL.of << 11);
}

int main(void)
{
    unsigned k, j, i;
    for (k = 0; k < NCASES; k++) {
        uint8_t initm[64];
        int ok = 1;
        for (j = 0; j < 16; j++) { X[j].u64[0] = BASEX[j][0]; X[j].u64[1] = BASEX[j][1]; }
        for (j = 0; j < 5; j++) { X[SP[j]].u64[0] = IN[k].xmm[j][0]; X[SP[j]].u64[1] = IN[k].xmm[j][1]; }
        memset(R, 0, sizeof R);
        R[0] = IN[k].gpr[0]; R[1] = IN[k].gpr[1]; R[2] = IN[k].gpr[2]; R[3] = IN[k].gpr[3]; R[6] = IN[k].gpr[4];
        R[7] = (uint64_t)(uintptr_t)(M + 16);
        memcpy(M, BASEM, sizeof M);
        memcpy(M + 16, &IN[k].mem[0], 8);
        memcpy(M + 24, &IN[k].mem[1], 8);
        memcpy(initm, M, sizeof M);
        FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;
        fle_mxcsr = IN[k].mx;
        FN[IN[k].enc]();
        if (memcmp(initm, M, sizeof M)) ok = 0;
        for (j = 0; j < 16; j++) {
            int sp = 0;
            for (i = 0; i < 5; i++) if (SP[i] == (int)j) sp = 1;
            if (!sp && (X[j].u64[0] != BASEX[j][0] || X[j].u64[1] != BASEX[j][1])) ok = 0;
        }
        printf("%u", k);
        for (j = 0; j < 5; j++) printf(" %llx %llx", (unsigned long long)X[SP[j]].u64[0], (unsigned long long)X[SP[j]].u64[1]);
        printf(" %llx %llx %llx %llx %llx", (unsigned long long)R[0], (unsigned long long)R[1], (unsigned long long)R[2], (unsigned long long)R[3], (unsigned long long)R[6]);
        printf(" %x %llx %d\n", (unsigned)fle_mxcsr, (unsigned long long)(COMI[IN[k].enc] ? flagbits() : 0), ok);
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());
    let exe = compile(&format!("mxf_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "{group}: C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8");
    text.lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut p = 1;
            let mut xmm = [[0u64; 2]; 5];
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
            Out { xmm, gpr, mx: hex(t[p]) as u32, flags: hex(t[p + 1]), other_ok: t[p + 2] == "1" }
        })
        .collect()
}

/// Runs every case on the C side and the host, and compares. Returns the OR of the status flags the host raised
/// and the number of cases with a non-default control field.
fn check(group: &str, encs: &[Enc], cs: &[Case]) -> u32 {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, encs, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut by_enc: Vec<Vec<usize>> = vec![Vec::new(); encs.len()];
    for (k, case) in cs.iter().enumerate() {
        by_enc[case.enc].push(k);
    }
    let mut host: Vec<Option<Out>> = vec![None; cs.len()];
    for (e, idxs) in by_enc.iter().enumerate() {
        if idxs.is_empty() {
            continue;
        }
        let list: Vec<&Case> = idxs.iter().map(|&k| &cs[k]).collect();
        for (k, o) in idxs.iter().zip(run_host_group(&encs[e], &list)) {
            host[*k] = Some(o);
        }
    }
    let (mut fails, mut raised, mut ctl_cases, mut changed_mx) = (0usize, 0u32, 0usize, 0usize);
    for (k, case) in cs.iter().enumerate() {
        let h = host[k].as_ref().unwrap();
        raised |= h.mx & 0x3f;
        if case.mx & 0xe040 != 0 {
            ctl_cases += 1;
        }
        if (h.mx ^ case.mx) & 0x3f != 0 {
            changed_mx += 1;
        }
        assert!(h.other_ok, "{group}: host changed state it must not touch: {}", encs[case.enc].name);
        if *h != c[k] {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {} #{k}\n  bytes {:02x?} mx_in {:#x}\n  in   {:x?} gpr {:x?} mem {:x?}\n  C    {:x?} {:x?} mx {:#x} fl {:#x} other_ok {}\n  host {:x?} {:x?} mx {:#x} fl {:#x}",
                    encs[case.enc].name, encs[case.enc].bytes, case.mx, case.xmm, case.gpr, case.mem, c[k].xmm, c[k].gpr, c[k].mx, c[k].flags,
                    c[k].other_ok, h.xmm, h.gpr, h.mx, h.flags
                );
            }
        }
    }
    eprintln!(
        "{group}: encodings {} cases {} failures {fails} nondefault_ctl_cases {ctl_cases} cases_that_raised_a_new_flag {changed_mx} host_raised_union {raised:#x}",
        encs.len(),
        cs.len()
    );
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
    raised
}

// ---------------------------------------------------------------- case generation

fn forms(prefix: Option<u8>, opc: &[u8], imm: Option<u8>, w: bool, name: &str, dk: Kind, sk: Kind, scalar: bool, comi: bool) -> Vec<Enc> {
    let mk = |tag: &str, reg: u8, rm: Rm| Enc {
        name: format!("{name} {tag}"),
        bytes: enc(prefix, w, opc, reg, rm, imm),
        comi,
        dk,
        sk,
        scalar,
    };
    vec![mk("x0,x1", 0, Rm::X(1)), mk("x2,x2", 2, Rm::X(2)), mk("x9,x3", 9, Rm::X(3)), mk("x3,[rdi]", 3, Rm::Mem)]
}

/// (dst register slot, src register slot, is memory) of each form above, as indexes into SP.
fn form_slots(f: usize) -> (usize, Option<usize>) {
    match f % 4 {
        0 => (0, Some(1)),
        1 => (2, Some(2)),
        2 => (4, Some(3)),
        _ => (3, None),
    }
}

fn rand_case(e: &Enc, f: usize, id: usize, rng: &mut Rng) -> Case {
    let mut c = Case { enc: id, xmm: [[0; 2]; 5], gpr: [0; 5], mem: [0; 2], mx: random_mx(rng) };
    for x in c.xmm.iter_mut() {
        x[0] = rng.next_u64();
        x[1] = rng.next_u64();
    }
    for g in c.gpr.iter_mut() {
        *g = rng.next_u64();
    }
    let (d, s) = form_slots(f);
    fill(&mut c.xmm[d], e.dk, e.scalar, rng);
    match s {
        Some(s) if s != d => fill(&mut c.xmm[s], e.sk, e.scalar, rng),
        Some(_) => {}
        None => {
            let mut m = [0u64; 2];
            fill(&mut m, e.sk, e.scalar, rng);
            c.mem = m;
        }
    }
    c
}

/// n random cases for each encoding, plus (for scalar arithmetic in the register form) every pair of pool values
/// with the default control bits.
fn build(encs: &[Enc], n: usize, cross: bool, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut cs = Vec::new();
    for (id, e) in encs.iter().enumerate() {
        let f = id % 4;
        for _ in 0..n {
            cs.push(rand_case(e, f, id, &mut rng));
        }
        if cross && e.scalar && f == 0 {
            let (pd, ps) = (lane_pool(e.dk), lane_pool(e.sk));
            for &a in &pd {
                for &b in &ps {
                    let mut c = rand_case(e, f, id, &mut rng);
                    c.mx = 0x1f80;
                    set_lo(&mut c.xmm[0], e.dk, a);
                    set_lo(&mut c.xmm[1], e.sk, b);
                    cs.push(c);
                }
            }
        }
    }
    cs
}

const IE: u32 = 1;
const DE: u32 = 2;
const ZE: u32 = 4;
const OE: u32 = 8;
const UE: u32 = 16;
const PE: u32 = 32;

#[test]
fn arith_matches_host() {
    let ops: [(&str, u8); 7] = [("add", 0x58), ("mul", 0x59), ("sub", 0x5c), ("min", 0x5d), ("div", 0x5e), ("max", 0x5f), ("sqrt", 0x51)];
    let mut encs = Vec::new();
    for (n, op) in ops {
        encs.extend(forms(Some(0xf3), &[0x0f, op], None, false, &format!("{n}ss"), Kind::F32, Kind::F32, true, false));
        encs.extend(forms(Some(0xf2), &[0x0f, op], None, false, &format!("{n}sd"), Kind::F64, Kind::F64, true, false));
        encs.extend(forms(None, &[0x0f, op], None, false, &format!("{n}ps"), Kind::F32, Kind::F32, false, false));
        encs.extend(forms(Some(0x66), &[0x0f, op], None, false, &format!("{n}pd"), Kind::F64, Kind::F64, false, false));
    }
    let cs = build(&encs, 160, true, 0x1111);
    let raised = check("arith", &encs, &cs);
    assert_eq!(raised, IE | DE | ZE | OE | UE | PE, "every status flag must have been raised by some case");
}

/// Decision 0106: HADDPD (SSE3, 66 0F 7C /r), the one horizontal add lifted. The host CPU runs the same instruction on 4,000 random cases per
/// form (the register forms x0,x1 / x2,x2 / x9,x3 and the memory form), with random RC, FTZ, DAZ and sticky flags, and every XMM result, GPR,
/// the untouched registers and memory, and MXCSR must be equal. (`check` prints how many cases raised a flag that was not already set.)
#[test]
fn haddpd_matches_host() {
    let encs = forms(Some(0x66), &[0x0f, 0x7c], None, false, "haddpd", Kind::F64, Kind::F64, false, false);
    let cs = build(&encs, 4000, false, 0x7c7c);
    let _ = check("haddpd", &encs, &cs);
}

/// Decision 0191: the SSE3 instructions that the census of the engine and its five DLLs holds and that were unsupported before (dsp_ippv2: MOVDDUP 3,154 sites, HADDPS 925, ADDSUBPS 438, ADDSUBPD 348, MOVSLDUP 322, MOVSHDUP 266):
/// HADDPS F2 0F 7C, ADDSUBPS F2 0F D0, ADDSUBPD 66 0F D0, and the data moves MOVDDUP F2 0F 12, MOVSLDUP F3 0F 12, MOVSHDUP F3 0F 16. The host CPU runs the same instruction on 8,000 random cases per form (the register forms
/// x0,x1 / x2,x2 / x9,x3 and the memory form: 32,000 per instruction, 192,000 in all), with random RC, FTZ, DAZ and sticky flags; every XMM result, the registers and memory the instruction must not touch, and MXCSR must be equal.
#[test]
fn sse3_ia2_matches_host() {
    let mut encs = Vec::new();
    encs.extend(forms(Some(0xf2), &[0x0f, 0x7c], None, false, "haddps", Kind::F32, Kind::F32, false, false));
    encs.extend(forms(Some(0xf2), &[0x0f, 0xd0], None, false, "addsubps", Kind::F32, Kind::F32, false, false));
    encs.extend(forms(Some(0x66), &[0x0f, 0xd0], None, false, "addsubpd", Kind::F64, Kind::F64, false, false));
    encs.extend(forms(Some(0xf2), &[0x0f, 0x12], None, false, "movddup", Kind::F64, Kind::F64, false, false));
    encs.extend(forms(Some(0xf3), &[0x0f, 0x12], None, false, "movsldup", Kind::F32, Kind::F32, false, false));
    encs.extend(forms(Some(0xf3), &[0x0f, 0x16], None, false, "movshdup", Kind::F32, Kind::F32, false, false));
    let cs = build(&encs, 8000, false, 0x1910);
    assert_eq!(cs.len(), 8000 * encs.len(), "192,000 cases in all");
    let _ = check("sse3_ia2", &encs, &cs);
}

#[test]
fn cmp_matches_host() {
    let mut encs = Vec::new();
    for imm in 0..8u8 {
        encs.extend(forms(Some(0xf3), &[0x0f, 0xc2], Some(imm), false, &format!("cmpss {imm}"), Kind::F32, Kind::F32, true, false));
        encs.extend(forms(Some(0xf2), &[0x0f, 0xc2], Some(imm), false, &format!("cmpsd {imm}"), Kind::F64, Kind::F64, true, false));
        encs.extend(forms(None, &[0x0f, 0xc2], Some(imm), false, &format!("cmpps {imm}"), Kind::F32, Kind::F32, false, false));
        encs.extend(forms(Some(0x66), &[0x0f, 0xc2], Some(imm), false, &format!("cmppd {imm}"), Kind::F64, Kind::F64, false, false));
    }
    let cs = build(&encs, 120, false, 0x2222);
    let raised = check("cmp", &encs, &cs);
    assert_eq!(raised & (IE | DE), IE | DE);
}

#[test]
fn comi_matches_host() {
    let mut encs = Vec::new();
    encs.extend(forms(Some(0x66), &[0x0f, 0x2f], None, false, "comisd", Kind::F64, Kind::F64, true, true));
    encs.extend(forms(None, &[0x0f, 0x2f], None, false, "comiss", Kind::F32, Kind::F32, true, true));
    encs.extend(forms(Some(0x66), &[0x0f, 0x2e], None, false, "ucomisd", Kind::F64, Kind::F64, true, true));
    encs.extend(forms(None, &[0x0f, 0x2e], None, false, "ucomiss", Kind::F32, Kind::F32, true, true));
    let cs = build(&encs, 300, true, 0x3333);
    let raised = check("comi", &encs, &cs);
    assert_eq!(raised & (IE | DE), IE | DE);
}

#[test]
fn round_matches_host() {
    let mut encs = Vec::new();
    for imm in 0..16u8 {
        let o = [0x0f, 0x3a];
        encs.extend(forms(Some(0x66), &[o[0], o[1], 0x0a], Some(imm), false, &format!("roundss {imm}"), Kind::F32, Kind::F32, true, false));
        encs.extend(forms(Some(0x66), &[o[0], o[1], 0x0b], Some(imm), false, &format!("roundsd {imm}"), Kind::F64, Kind::F64, true, false));
        encs.extend(forms(Some(0x66), &[o[0], o[1], 0x08], Some(imm), false, &format!("roundps {imm}"), Kind::F32, Kind::F32, false, false));
        encs.extend(forms(Some(0x66), &[o[0], o[1], 0x09], Some(imm), false, &format!("roundpd {imm}"), Kind::F64, Kind::F64, false, false));
    }
    let cs = build(&encs, 100, false, 0x4444);
    let raised = check("round", &encs, &cs);
    assert_eq!(raised & (IE | PE), IE | PE);
}

#[test]
fn convert_fp_matches_host() {
    let mut encs = Vec::new();
    encs.extend(forms(Some(0xf3), &[0x0f, 0x5a], None, false, "cvtss2sd", Kind::F64, Kind::F32, true, false));
    encs.extend(forms(Some(0xf2), &[0x0f, 0x5a], None, false, "cvtsd2ss", Kind::F32, Kind::F64, true, false));
    encs.extend(forms(None, &[0x0f, 0x5a], None, false, "cvtps2pd", Kind::F64, Kind::F32, false, false));
    encs.extend(forms(Some(0x66), &[0x0f, 0x5a], None, false, "cvtpd2ps", Kind::F32, Kind::F64, false, false));
    encs.extend(forms(None, &[0x0f, 0x5b], None, false, "cvtdq2ps", Kind::F32, Kind::I32, false, false));
    encs.extend(forms(Some(0x66), &[0x0f, 0x5b], None, false, "cvtps2dq", Kind::I32, Kind::F32, false, false));
    encs.extend(forms(Some(0xf3), &[0x0f, 0x5b], None, false, "cvttps2dq", Kind::I32, Kind::F32, false, false));
    encs.extend(forms(Some(0xf3), &[0x0f, 0xe6], None, false, "cvtdq2pd", Kind::F64, Kind::I32, false, false));
    encs.extend(forms(Some(0xf2), &[0x0f, 0xe6], None, false, "cvtpd2dq", Kind::I32, Kind::F64, false, false));
    encs.extend(forms(Some(0x66), &[0x0f, 0xe6], None, false, "cvttpd2dq", Kind::I32, Kind::F64, false, false));
    let cs = build(&encs, 400, false, 0x5555);
    let raised = check("cvt_fp", &encs, &cs);
    assert_eq!(raised & (IE | DE | OE | UE | PE), IE | DE | OE | UE | PE);
}

/// cvtsi2ss/sd (GPR and memory, 32 and 64 bit) and cvt[t]ss/sd2si (32 and 64 bit result).
#[test]
fn convert_int_matches_host() {
    let mut encs: Vec<Enc> = Vec::new();
    let mut gen: Vec<(usize, bool)> = Vec::new(); // (enc id, is the int operand a 64-bit one)
    let mk = |name: String, bytes: Vec<u8>, dk: Kind, sk: Kind| Enc { name, bytes, comi: false, dk, sk, scalar: true };
    for (n, pf, dk) in [("cvtsi2ss", 0xf3u8, Kind::F32), ("cvtsi2sd", 0xf2, Kind::F64)] {
        for w in [false, true] {
            for (g, reg) in [(0u8, 0u8), (1, 2), (6, 9)] {
                let _ = reg;
                encs.push(mk(format!("{n} w{w} x0, g{g}"), enc(Some(pf), w, &[0x0f, 0x2a], 0, Rm::G(g), None), dk, Kind::I32));
                gen.push((encs.len() - 1, w));
            }
            encs.push(mk(format!("{n} w{w} x9, g3"), enc(Some(pf), w, &[0x0f, 0x2a], 9, Rm::G(3), None), dk, Kind::I32));
            gen.push((encs.len() - 1, w));
            encs.push(mk(format!("{n} w{w} x3, [rdi]"), enc(Some(pf), w, &[0x0f, 0x2a], 3, Rm::Mem, None), dk, Kind::I32));
            gen.push((encs.len() - 1, w));
        }
    }
    let n_cvtsi = encs.len();
    for (n, pf, opc, sk) in [
        ("cvttss2si", 0xf3u8, 0x2cu8, Kind::F32),
        ("cvttsd2si", 0xf2, 0x2c, Kind::F64),
        ("cvtss2si", 0xf3, 0x2d, Kind::F32),
        ("cvtsd2si", 0xf2, 0x2d, Kind::F64),
    ] {
        for w in [false, true] {
            encs.push(mk(format!("{n} w{w} g0, x1"), enc(Some(pf), w, &[0x0f, opc], 0, Rm::X(1), None), Kind::I32, sk));
            encs.push(mk(format!("{n} w{w} g2, x9"), enc(Some(pf), w, &[0x0f, opc], 2, Rm::X(9), None), Kind::I32, sk));
            encs.push(mk(format!("{n} w{w} g6, [rdi]"), enc(Some(pf), w, &[0x0f, opc], 6, Rm::Mem, None), Kind::I32, sk));
        }
    }
    let mut rng = Rng(0x6666);
    let mut cs = Vec::new();
    let (p64, p32) = (pool_i64(), pool_i32());
    for (id, w) in &gen {
        for _ in 0..300 {
            let mut c = rand_case(&encs[*id], 0, *id, &mut rng);
            let v = if *w { if rng.below(3) != 0 { pick(&mut rng, &p64) } else { rng.next_u64() } } else { pick(&mut rng, &p32) | (rng.next_u64() << 32 & if rng.below(2) == 0 { 0 } else { u64::MAX }) };
            c.gpr = [v, v, v, v, v];
            c.mem = [v, rng.next_u64()];
            cs.push(c);
        }
    }
    for id in n_cvtsi..encs.len() {
        let f = (id - n_cvtsi) % 3;
        for _ in 0..400 {
            let mut c = rand_case(&encs[id], 0, id, &mut rng);
            let sk = encs[id].sk;
            let p = lane_pool(sk);
            let v = if rng.below(4) != 0 { pick(&mut rng, &p) } else { rng.next_u64() };
            match f {
                0 => set_lo(&mut c.xmm[1], sk, v),
                1 => set_lo(&mut c.xmm[4], sk, v),
                _ => {
                    c.mem = [0, 0];
                    set_lo(&mut c.mem, sk, v);
                    c.mem[1] = rng.next_u64();
                }
            }
            cs.push(c);
        }
    }
    let raised = check("cvt_int", &encs, &cs);
    assert_eq!(raised & (IE | DE | PE), IE | DE | PE);
}

// ---------------------------------------------------------------- random cases, pure C versus the host reference

/// tests/ref/mxcsr_diff.c runs the portable runtime/fle_mxcsr_flags.h and the host reference
/// tests/ref/fle_mxcsr_flags_host.h (the instruction itself on the CPU) on the same inputs and compares the XMM
/// result, the GPR result, the whole MXCSR and the COMIS flags. 166 operation variants (every cmp predicate, round
/// imm8 values 0..16, 130, 245, 255), 100,000 random 32/64-bit pattern cases each, random RC / FTZ / DAZ and sticky bits.
/// `FLE_MXCSR_DIFF_SCALE` (cases per operation, default 100000) overrides the count; MXCSR_MUTANT_RT names a runtime
/// directory to use instead of runtime/ (mutation testing).
#[test]
fn random_pure_matches_host_ref() {
    let dir = out_dir();
    let rt = rt_dir();
    let refdir = crate_dir().join("tests/ref");
    let exe = dir.join(etxt_common::uniq("mxf_random"));
    let cc = Command::new("clang")
        .args(["-O1", "-std=gnu11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg("-I")
        .arg(&refdir)
        .arg("-o")
        .arg(&exe)
        .arg(refdir.join("mxcsr_diff.c"))
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let scale = std::env::var("FLE_MXCSR_DIFF_SCALE").unwrap_or_else(|_| "100000".into());
    let run = Command::new(&exe).arg(&scale).output().expect("run");
    let text = String::from_utf8_lossy(&run.stdout);
    let total = text.lines().find(|l| l.starts_with("TOTAL")).unwrap_or("(no TOTAL line)");
    eprintln!("random pure vs host: {total}");
    let mut union_ok = 0;
    for l in text.lines().filter(|l| l.starts_with("OP ")) {
        let t: Vec<&str> = l.split_whitespace().collect();
        if t[7] == "0x3f" {
            union_ok += 1;
        }
    }
    for l in text.lines().filter(|l| l.starts_with("MISMATCH") || l.starts_with("  ")).take(40) {
        eprintln!("{l}");
    }
    assert!(run.status.success(), "pure C differs from the host reference: {total}");
    assert!(total.contains("mismatches 0 ops 166"), "{total}");
    let cases: u64 = total.split_whitespace().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    if scale == "100000" {
        assert!(cases >= 166 * 100_000, "{total}");
    }
    assert!(union_ok >= 150, "most operations must have raised every status flag somewhere (got {union_ok} of 166)");
}

// ---------------------------------------------------------------- SDM expectations (C side only)

struct Sdm {
    name: &'static str,
    bytes: Vec<u8>,
    /// low element values of x0 (dst) and x1 (src), or the GPR in rax
    a: u64,
    b: u64,
    gpr: u64,
    mx: u32,
    /// flags expected to be set after the instruction (status bits 0..5, including those set before)
    want: u32,
}

fn sdm_cases() -> Vec<Sdm> {
    let d = 0x1f80;
    let f = |name, bytes: Vec<u8>, a, b, want| Sdm { name, bytes, a, b, gpr: 0, mx: d, want };
    let xx = |p, n| enc(Some(p), false, &[0x0f, n], 0, Rm::X(1), None);
    let ps_ = |n| enc(None, false, &[0x0f, n], 0, Rm::X(1), None);
    let one = 0x3ff0_0000_0000_0000u64;
    let max = 0x7fef_ffff_ffff_ffffu64;
    let inf = 0x7ff0_0000_0000_0000u64;
    let qnan = 0x7ff8_0000_0000_0000u64;
    let snan = 0x7ff0_0000_0000_0001u64;
    let den = 1u64;
    let tiny = 0x1000_0000_0000_0000u64; // 2^-767, tiny * tiny = 2^-1534 underflows to zero
    vec![
        f("divsd 1/0", xx(0xf2, 0x5e), one, 0, ZE),
        f("divsd 0/0", xx(0xf2, 0x5e), 0, 0, IE),
        f("divsd inf/inf", xx(0xf2, 0x5e), inf, inf, IE),
        f("divsd 1/3", xx(0xf2, 0x5e), one, 0x4008_0000_0000_0000, PE),
        f("divss 1/0", xx(0xf3, 0x5e), 0x3f80_0000, 0, ZE),
        f("addsd inf + -inf", xx(0xf2, 0x58), inf, 0xfff0_0000_0000_0000, IE),
        f("addsd snan + 1", xx(0xf2, 0x58), snan, one, IE),
        f("addsd qnan + 1", xx(0xf2, 0x58), qnan, one, 0),
        f("addsd 1 + 1", xx(0xf2, 0x58), one, one, 0),
        f("addsd max + max", xx(0xf2, 0x58), max, max, OE | PE),
        f("addsd denormal + 1", xx(0xf2, 0x58), den, one, DE | PE),
        f("mulsd 0 * inf", xx(0xf2, 0x59), 0, inf, IE),
        f("mulsd tiny * tiny", xx(0xf2, 0x59), tiny, tiny, UE | PE),
        f("mulsd max * 2", xx(0xf2, 0x59), max, 0x4000_0000_0000_0000, OE | PE),
        f("mulss 1 * 1", xx(0xf3, 0x59), 0x3f80_0000, 0x3f80_0000, 0),
        f("subsd max - -max", xx(0xf2, 0x5c), max, max | (1 << 63), OE | PE),
        f("sqrtsd -1", xx(0xf2, 0x51), 0, 0xbff0_0000_0000_0000, IE),
        f("sqrtsd 2", xx(0xf2, 0x51), 0, 0x4000_0000_0000_0000, PE),
        f("sqrtsd 4", xx(0xf2, 0x51), 0, 0x4010_0000_0000_0000, 0),
        f("minsd qnan,1", xx(0xf2, 0x5d), qnan, one, IE),
        f("maxsd 1,qnan", xx(0xf2, 0x5f), one, qnan, IE),
        f("minsd 1,2", xx(0xf2, 0x5d), one, 0x4000_0000_0000_0000, 0),
        f("ucomisd qnan,1", enc(Some(0x66), false, &[0x0f, 0x2e], 0, Rm::X(1), None), qnan, one, 0),
        f("ucomisd snan,1", enc(Some(0x66), false, &[0x0f, 0x2e], 0, Rm::X(1), None), snan, one, IE),
        f("comisd qnan,1", enc(Some(0x66), false, &[0x0f, 0x2f], 0, Rm::X(1), None), qnan, one, IE),
        f("comisd 1,2", enc(Some(0x66), false, &[0x0f, 0x2f], 0, Rm::X(1), None), one, 0x4000_0000_0000_0000, 0),
        f("ucomisd denormal,1", enc(Some(0x66), false, &[0x0f, 0x2e], 0, Rm::X(1), None), den, one, DE),
        f("cmpltsd qnan,1", enc(Some(0xf2), false, &[0x0f, 0xc2], 0, Rm::X(1), Some(1)), qnan, one, IE),
        f("cmpeqsd qnan,1", enc(Some(0xf2), false, &[0x0f, 0xc2], 0, Rm::X(1), Some(0)), qnan, one, 0),
        f("cmpeqsd snan,1", enc(Some(0xf2), false, &[0x0f, 0xc2], 0, Rm::X(1), Some(0)), snan, one, IE),
        f("cmpunordsd qnan,1", enc(Some(0xf2), false, &[0x0f, 0xc2], 0, Rm::X(1), Some(3)), qnan, one, 0),
        f("cvtsd2ss 1e300", xx(0xf2, 0x5a), 0, 0x7e37_e43c_8800_759c, OE | PE),
        f("cvtsd2ss 1e-300", xx(0xf2, 0x5a), 0, 0x01a5_6e1f_c2f8_f359, UE | PE),
        f("cvtsd2ss 1", xx(0xf2, 0x5a), 0, one, 0),
        f("cvtsd2ss 1/3", xx(0xf2, 0x5a), 0, 0x3fd5_5555_5555_5555, PE),
        f("cvtss2sd 1", xx(0xf3, 0x5a), 0, 0x3f80_0000, 0),
        f("cvtss2sd snan", xx(0xf3, 0x5a), 0, 0x7f80_0001, IE),
        f("cvtss2sd denormal", xx(0xf3, 0x5a), 0, 1, DE),
        f("cvtdq2ps 16777217", ps_(0x5b), 0, 16777217, PE),
        f("cvtdq2ps 16777216", ps_(0x5b), 0, 16777216, 0),
        f("cvtps2dq 1.5", enc(Some(0x66), false, &[0x0f, 0x5b], 0, Rm::X(1), None), 0, 0x3fc0_0000, PE),
        f("cvtps2dq 2^31", enc(Some(0x66), false, &[0x0f, 0x5b], 0, Rm::X(1), None), 0, 0x4f00_0000, IE),
        f("cvttps2dq 1.5", enc(Some(0xf3), false, &[0x0f, 0x5b], 0, Rm::X(1), None), 0, 0x3fc0_0000, PE),
        f("cvttps2dq nan", enc(Some(0xf3), false, &[0x0f, 0x5b], 0, Rm::X(1), None), 0, 0x7fc0_0000, IE),
        f("roundsd 1.5 imm0", enc(Some(0x66), false, &[0x0f, 0x3a, 0x0b], 0, Rm::X(1), Some(0)), 0, 0x3ff8_0000_0000_0000, PE),
        f("roundsd 1.5 imm8", enc(Some(0x66), false, &[0x0f, 0x3a, 0x0b], 0, Rm::X(1), Some(8)), 0, 0x3ff8_0000_0000_0000, 0),
        f("roundsd snan imm8", enc(Some(0x66), false, &[0x0f, 0x3a, 0x0b], 0, Rm::X(1), Some(8)), 0, snan, IE),
        f("roundsd qnan imm0", enc(Some(0x66), false, &[0x0f, 0x3a, 0x0b], 0, Rm::X(1), Some(0)), 0, qnan, 0),
        Sdm { name: "cvtsd2si 1.5 (32)", bytes: enc(Some(0xf2), false, &[0x0f, 0x2d], 0, Rm::X(1), None), a: 0, b: 0x3ff8_0000_0000_0000, gpr: 0, mx: d, want: PE },
        Sdm { name: "cvtsd2si 2^40 (32)", bytes: enc(Some(0xf2), false, &[0x0f, 0x2d], 0, Rm::X(1), None), a: 0, b: 0x4270_0000_0000_0000, gpr: 0, mx: d, want: IE },
        Sdm { name: "cvttsd2si nan (64)", bytes: enc(Some(0xf2), true, &[0x0f, 0x2c], 0, Rm::X(1), None), a: 0, b: qnan, gpr: 0, mx: d, want: IE },
        Sdm { name: "cvttsd2si 1.5 (64)", bytes: enc(Some(0xf2), true, &[0x0f, 0x2c], 0, Rm::X(1), None), a: 0, b: 0x3ff8_0000_0000_0000, gpr: 0, mx: d, want: PE },
        Sdm { name: "cvttsd2si 3 (32)", bytes: enc(Some(0xf2), false, &[0x0f, 0x2c], 0, Rm::X(1), None), a: 0, b: 0x4008_0000_0000_0000, gpr: 0, mx: d, want: 0 },
        Sdm { name: "cvtss2si denormal", bytes: enc(Some(0xf3), false, &[0x0f, 0x2d], 0, Rm::X(1), None), a: 0, b: 1, gpr: 0, mx: d, want: PE },
        Sdm { name: "cvtsi2sd 2^53+1 (64)", bytes: enc(Some(0xf2), true, &[0x0f, 0x2a], 0, Rm::G(0), None), a: 0, b: 0, gpr: (1u64 << 53) + 1, mx: d, want: PE },
        Sdm { name: "cvtsi2sd 2^53 (64)", bytes: enc(Some(0xf2), true, &[0x0f, 0x2a], 0, Rm::G(0), None), a: 0, b: 0, gpr: 1u64 << 53, mx: d, want: 0 },
        Sdm { name: "cvtsi2sd -1 (32)", bytes: enc(Some(0xf2), false, &[0x0f, 0x2a], 0, Rm::G(0), None), a: 0, b: 0, gpr: 0xffff_ffff, mx: d, want: 0 },
        Sdm { name: "cvtsi2ss 16777217 (32)", bytes: enc(Some(0xf3), false, &[0x0f, 0x2a], 0, Rm::G(0), None), a: 0, b: 0, gpr: 16777217, mx: d, want: PE },
        // sticky: a flag set before stays set, and a new one is added
        Sdm { name: "addsd 1+1, PE set before", bytes: xx(0xf2, 0x58), a: one, b: one, gpr: 0, mx: d | PE, want: PE },
        Sdm { name: "divsd 1/0, PE set before", bytes: xx(0xf2, 0x5e), a: one, b: 0, gpr: 0, mx: d | PE, want: PE | ZE },
        // all masks clear (0): a flag-free instruction runs normally and leaves MXCSR unchanged
        Sdm { name: "addsd 1+1, all unmasked", bytes: xx(0xf2, 0x58), a: one, b: one, gpr: 0, mx: 0, want: 0 },
    ]
}

#[test]
fn flags_match_sdm_expectations() {
    let list = sdm_cases();
    let mut encs = Vec::new();
    let mut cs = Vec::new();
    for (k, s) in list.iter().enumerate() {
        let comi = s.name.contains("comi");
        encs.push(Enc { name: s.name.to_string(), bytes: s.bytes.clone(), comi, dk: Kind::F64, sk: Kind::F64, scalar: true });
        let mut c = Case { enc: k, xmm: [[0; 2]; 5], gpr: [s.gpr; 5], mem: [0; 2], mx: s.mx };
        // Packed forms convert every lane, so their upper source lanes are zero; scalar forms keep a marker there.
        let packed = ["cvtdq2ps", "cvtps2dq", "cvttps2dq"].iter().any(|p| s.name.starts_with(p));
        c.xmm[0] = [s.a, 0x1111_2222_3333_4444];
        c.xmm[1] = [s.b, if packed { 0 } else { 0x5555_6666_7777_8888 }];
        cs.push(c);
    }
    let out = c_run("sdm", &encs, &cs);
    let mut bad = 0;
    for ((s, o), c) in list.iter().zip(out.iter()).zip(cs.iter()) {
        let got = o.mx & 0x3f;
        let want = s.want | (c.mx & 0x3f);
        if got != want || (o.mx & !0x3f) != (c.mx & !0x3f) {
            bad += 1;
            eprintln!("{}: mxcsr flags {:#x}, SDM says {:#x} (mx in {:#x} out {:#x})", s.name, got, want, c.mx, o.mx);
        }
    }
    eprintln!("sdm expectations: {} cases, {} differ", list.len(), bad);
    assert_eq!(bad, 0);
}

// ---------------------------------------------------------------- unmasked exceptions abort

fn run_c_main(name: &str, main_body: &str, bytes: &[u8]) -> std::process::Output {
    let body = lift_body(bytes, 0x1000).expect("lift");
    let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n{main_body}\n c0();\n printf(\"done mxcsr=%x\\n\", (unsigned)fle_mxcsr);\n return 0;\n}}\n");
    let exe = compile(name, &src);
    Command::new(exe).output().expect("run")
}

#[test]
fn unmasked_raised_flag_is_a_named_abort() {
    let divsd = enc(Some(0xf2), false, &[0x0f, 0x5e], 0, Rm::X(1), None);
    let set = |mx: u32| format!("X[0].u64[0] = 0x3ff0000000000000ull; X[1].u64[0] = 0; fle_mxcsr = {mx:#x};");
    // ZM clear (bit 9) and 1/0: ZE raised and unmasked
    let o = run_c_main("mxf_abort_ze", &set(0x1f80 & !(1 << 9)), &divsd);
    assert_eq!(o.status.signal(), Some(6), "expected SIGABRT");
    assert!(String::from_utf8_lossy(&o.stderr).contains("unmasked in guest MXCSR"), "{}", String::from_utf8_lossy(&o.stderr));
    // ZM set: continues, ZE is sticky
    let o = run_c_main("mxf_ok_ze", &set(0x1f80), &divsd);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("done mxcsr=1f84"), "{}", String::from_utf8_lossy(&o.stdout));
    // IM clear and 0/0
    let o = run_c_main("mxf_abort_ie", "X[0].u64[0] = 0; X[1].u64[0] = 0; fle_mxcsr = 0x1f00;", &divsd);
    assert_eq!(o.status.signal(), Some(6));
    // PM clear and an inexact result
    let o = run_c_main("mxf_abort_pe", "X[0].u64[0] = 0x3ff0000000000000ull; X[1].u64[0] = 0x4008000000000000ull; fle_mxcsr = 0x1f80 & ~(1u << 12);", &divsd);
    assert_eq!(o.status.signal(), Some(6));
    // UM clear (bit 11): an exact but tiny result (2^-1022 * 0.5 = 2^-1023) is still an underflow, so it aborts;
    // with UM set it is silent
    let mulsd = enc(Some(0xf2), false, &[0x0f, 0x59], 0, Rm::X(1), None);
    let tiny = "X[0].u64[0] = 0x0010000000000000ull; X[1].u64[0] = 0x3fe0000000000000ull;";
    let o = run_c_main("mxf_abort_ue_exact", &format!("{tiny} fle_mxcsr = 0x1f80 & ~(1u << 11);"), &mulsd);
    assert_eq!(o.status.signal(), Some(6), "exact tiny result with UM clear must abort");
    // only UE: FTZ does not apply while UM is clear, so no PE is made up
    assert!(String::from_utf8_lossy(&o.stderr).contains("raised flags 0x10,"), "{}", String::from_utf8_lossy(&o.stderr));
    let o = run_c_main("mxf_abort_ue_ftz_um_clear", &format!("{tiny} fle_mxcsr = 0x8000 | (0x1f80 & !(1u << 11));"), &mulsd);
    assert_eq!(o.status.signal(), Some(6));
    assert!(String::from_utf8_lossy(&o.stderr).contains("raised flags 0x10,"), "{}", String::from_utf8_lossy(&o.stderr));
    let o = run_c_main("mxf_ok_ue_exact", &format!("{tiny} fle_mxcsr = 0x1f80;"), &mulsd);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("done mxcsr=1f80"), "{}", String::from_utf8_lossy(&o.stdout));
    // FTZ with UM set: tiny result becomes zero and UE, PE are raised
    let o = run_c_main("mxf_ftz", &format!("{tiny} fle_mxcsr = 0x1f80 | 0x8000;"), &mulsd);
    assert!(String::from_utf8_lossy(&o.stdout).contains("done mxcsr=9fb0"), "{}", String::from_utf8_lossy(&o.stdout));
    // DM clear (bit 8) and a denormal operand: DE is raised and unmasked
    let addsd = enc(Some(0xf2), false, &[0x0f, 0x58], 0, Rm::X(1), None);
    let o = run_c_main("mxf_abort_de", "X[0].u64[0] = 1; X[1].u64[0] = 0x3ff0000000000000ull; fle_mxcsr = 0x1f80 & ~(1u << 8);", &addsd);
    assert_eq!(o.status.signal(), Some(6));
    // no flag raised, nothing unmasked-and-raised: no abort even with every mask clear
    let o = run_c_main("mxf_ok_none", "X[0].u64[0] = 0x3ff0000000000000ull; X[1].u64[0] = 0x4000000000000000ull; fle_mxcsr = 0;", &divsd);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).contains("done mxcsr=0"));
}

// ---------------------------------------------------------------- PREFETCHh

/// [prefetch group] with the addressing forms the closure uses: [reg+disp8], [base+index*scale+disp8], rip-relative,
/// REX.B base (0x69d90c: `prefetcht0 [r8+0x10]`), no base.
fn prefetch_encodings() -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    for (name, ext) in [("t0", 1u8), ("t1", 2), ("t2", 3), ("nta", 0)] {
        v.push((format!("prefetch{name} [rdi+0x10]"), vec![0x0f, 0x18, 0x40 | (ext << 3) | 7, 0x10]));
        v.push((format!("prefetch{name} [rcx+rbx*8+8]"), vec![0x0f, 0x18, 0x44 | (ext << 3), 0xd9, 0x08]));
        v.push((format!("prefetch{name} [r8+0x10]"), vec![0x41, 0x0f, 0x18, 0x40 | (ext << 3), 0x10]));
        v.push((format!("prefetch{name} [r9+r10*4-0x80]"), vec![0x43, 0x0f, 0x18, 0x44 | (ext << 3), 0x91, 0x80]));
        v.push((format!("prefetch{name} [rip+0x12345678]"), vec![0x0f, 0x18, (ext << 3) | 5, 0x78, 0x56, 0x34, 0x12]));
        v.push((format!("prefetch{name} [rip-0x7fffffff]"), vec![0x0f, 0x18, (ext << 3) | 5, 0x01, 0x00, 0x00, 0x80]));
        v.push((format!("prefetch{name} [rax+rax*1+0x7fffffff]"), vec![0x0f, 0x18, 0x84 | (ext << 3), 0x00, 0xff, 0xff, 0xff, 0x7f]));
        v.push((format!("prefetch{name} [rsi]"), vec![0x0f, 0x18, (ext << 3) | 6]));
    }
    v
}

#[test]
fn prefetch_never_faults_and_changes_nothing() {
    let encs = prefetch_encodings();
    let mut src = String::from(HEADER);
    for (k, (name, bytes)) in encs.iter().enumerate() {
        let body = lift_body(bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(body.contains("(void)("), "{name}: {body}");
        assert!(!body.contains("fle_ld") && !body.contains("fle_st"), "{name}: prefetch must not read or write memory: {body}");
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "static void (*const FN[])(void) = {";
    for k in 0..encs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n";
    src += r#"
int main(void)
{
    /* addresses that cannot be read: null, near null, non-canonical, kernel half, the top of the address space */
    static const uint64_t BAD[] = { 0, 1, 0x10, 0xdead0000dead0000ull, 0x8000000000000000ull, 0xffffffffffffffffull,
                                    0xffff800000000000ull, 0x00007fffffffffffull, 0x0000700000000000ull };
    unsigned k, b, j;
    unsigned long long sum = 0;
    for (k = 0; k < NENC; k++) {
        for (b = 0; b < sizeof BAD / sizeof BAD[0]; b++) {
            uint64_t before[16];
            fle_xmm xb[16];
            fle_eflags fb;
            for (j = 0; j < 16; j++) { R[j] = BAD[(b + j) % (sizeof BAD / sizeof BAD[0])]; before[j] = R[j]; X[j].u64[0] = 0x1111 * (j + 1); X[j].u64[1] = ~X[j].u64[0]; xb[j] = X[j]; }
            FL.cf = 1; FL.pf = 0; FL.af = 1; FL.zf = 0; FL.sf = 1; FL.of = 0; fb = FL;
            fle_mxcsr = 0x1f80;
            FN[k]();
            for (j = 0; j < 16; j++) {
                if (R[j] != before[j] || memcmp(&X[j], &xb[j], sizeof X[j])) { printf("CHANGED %u %u\n", k, b); return 1; }
            }
            if (memcmp(&FL, &fb, sizeof FL) || fle_mxcsr != 0x1f80) { printf("FLAGS CHANGED %u %u\n", k, b); return 1; }
            sum++;
        }
    }
    printf("ok %llu\n", sum);
    return 0;
}
"#;
    src = src.replace("NENC", &encs.len().to_string());
    let exe = compile("mxf_prefetch", &src);
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "prefetch C program failed (signal {:?}): {}{}", run.status.signal(), String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
    let expect = encs.len() * 9;
    assert!(String::from_utf8_lossy(&run.stdout).contains(&format!("ok {expect}")), "{}", String::from_utf8_lossy(&run.stdout));

    // host: the machine instruction itself does not fault at the same addresses (every case runs to the end)
    let bad: [u64; 9] = [0, 1, 0x10, 0xdead_0000_dead_0000, 0x8000_0000_0000_0000, u64::MAX, 0xffff_8000_0000_0000, 0x0000_7fff_ffff_ffff, 0x0000_7000_0000_0000];
    let mut ran = 0usize;
    for (name, bytes) in &encs {
        // mov r64, imm64 for rax, rcx, rbx, rsi, rdi, r8, r9, r10 (not rsp or rbp), then the instruction, then ret
        for b in 0..bad.len() {
            let mut code: Vec<u8> = Vec::new();
            for (k, rex_reg) in [(0usize, (0x48u8, 0xb8u8)), (1, (0x48, 0xb9)), (2, (0x48, 0xbb)), (3, (0x48, 0xbe)), (4, (0x48, 0xbf)), (5, (0x49, 0xb8)), (6, (0x49, 0xb9)), (7, (0x49, 0xba))] {
                code.extend_from_slice(&[rex_reg.0, rex_reg.1]);
                code.extend_from_slice(&bad[(b + k) % bad.len()].to_le_bytes());
            }
            code.extend_from_slice(bytes);
            code.push(0xc3);
            unsafe {
                let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
                assert!(!p.is_null() && p as isize != -1, "mmap failed");
                std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
                // rbx is callee-saved: save and restore it around the call
                let f = std::mem::transmute::<*mut u8, extern "C" fn()>(p);
                let _ = name;
                call_saving_rbx(f);
                munmap(p, 4096);
            }
            ran += 1;
        }
    }
    eprintln!("prefetch: {} encodings, C side {} cases without fault or state change, host {ran} runs without fault", encs.len(), expect);
    assert_eq!(ran, expect);
}

/// Calls a stub that clobbers rbx, rsi, rdi and r8-r10 (rbx is callee-saved).
#[inline(never)]
fn call_saving_rbx(f: extern "C" fn()) {
    unsafe {
        std::arch::asm!(
            "push rbx",
            "call {f}",
            "pop rbx",
            f = in(reg) f,
            out("rax") _, out("rcx") _, out("rdx") _, out("rsi") _, out("rdi") _,
            out("r8") _, out("r9") _, out("r10") _, out("r11") _,
            clobber_abi("C"),
        );
    }
}
