//! Tests for sse_more.rs: PACKUSWB, PADDW, ADDPS, SUBPS, MULPS, MINPS and MAXPS
//! (xmm, xmm and xmm, m128).
//!
//! The lifted C (clang -O0, with runtime/fle_sse_more.c) and the host CPU run the
//! same instruction on the same state. The XMM registers, rax, rcx, rdx, rbx and rsi,
//! and 64 bytes of memory must match. These instructions change no flags.
//! The inputs are the lane edge values and random values. Float lanes include
//! NaN, infinities, zeros of both signs, and subnormals.

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

/// [prefix] [REX] 0F opc modrm [disp8]. `reg` is the modrm.reg field (an XMM register).
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

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_sse_more.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

/// Compiles `src` with clang -O0 and the runtime helpers into out-more/test, and returns the path.
fn compile(name: &str, src: &str) -> PathBuf {
    let dir = crate_dir().join("../../out-more/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    let rt = crate_dir().join("../../runtime");
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).expect("write C source");
    let cc = Command::new("clang")
        .args(["-O0", "-std=c11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg(rt.join("fle_sse_more.c"))
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

/// Builds one C program that runs every case, and returns its stdout.
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

    let exe = compile(&format!("sse_more_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed");
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
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h != *cout {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?}\n  C    {:x?} {:x?} {:?}\n  host {:x?} {:x?} {:?}",
                    case.name, case.bytes, cout.xmm, cout.gpr, cout.mem, h.xmm, h.gpr, h.mem
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails}", cs.len());
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

fn pick(rng: &mut Rng, pool: &[u64]) -> u64 {
    pool[(rng.next_u64() % pool.len() as u64) as usize]
}

/// 16-bit lane values: zero, one, the signed and unsigned edges, and the saturation edges.
fn pool16() -> Vec<u64> {
    vec![0, 1, 0x7f, 0x80, 0xff, 0x100, 0x7fff, 0x8000, 0x8001, 0xffff, 0xfffe, 0x00ff, 0xff00, 0x0101]
}

/// Single-precision inputs as bit patterns: zeros of both signs, ties, the edges of
/// the integer range, NaN (quiet, signalling, negative), infinities, subnormals.
fn pool32() -> Vec<u64> {
    let mut v: Vec<u64> = [
        0.0f32, -0.0, 0.5, -0.5, 1.5, 2.5, -2.5, 3.5, 16777217.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, 1e-40,
        0.1, 1.0, -1.0, 3.0, 9.223372e18,
    ]
    .iter()
    .map(|x| x.to_bits() as u64)
    .collect();
    v.extend([0x7fc0_0000u64, 0xffc0_0001, 0x7f80_0001, 0x7f80_0000, 0xff80_0000, 0x0000_0001, 0x8000_0001, 0x7f7f_ffff]);
    v
}

/// Random vectors for one instruction. The xmm inputs are drawn from `pool` lane by
/// lane (lane values fill all eight 16-bit or all four 32-bit lanes). Each round
/// gives one register-source case and, when the memory operand is 16-byte aligned,
/// one memory-source case (rdi + 0 or rdi + 16 is the aligned operand).
fn cases(name: &str, prefix: Option<u8>, opc: &[u8], pool: &[u64], bits: u32, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let lanes = 128 / bits as usize;
    let mut out = Vec::new();
    for k in 0..n {
        let d = (rng.next_u64() % 8) as u8;
        let s = (rng.next_u64() % 8) as u8 + 8;
        let mut c = base(format!("{name} x{d}, x{} #{k}", s & 7), enc(prefix, opc, d, Rm::X(s)), &mut rng);
        for lane in 0..lanes {
            fill_lane(&mut c.xmm[d as usize], lane, pick(&mut rng, pool), bits);
            fill_lane(&mut c.xmm[(s & 15) as usize], lane, pick(&mut rng, pool), bits);
        }
        out.push(c);
        // rdi + 0 or rdi + 16 is 16-byte aligned (rdi = mem + 16).
        let disp = [0i8, 16][(rng.next_u64() % 2) as usize];
        let mut m = base(format!("{name} x{d}, [rdi{disp:+}] #{k}"), enc(prefix, opc, d, Rm::Mem(disp)), &mut rng);
        for lane in 0..lanes {
            fill_lane(&mut m.xmm[d as usize], lane, pick(&mut rng, pool), bits);
        }
        // The memory source is the 16 bytes at rdi + disp, that is mem[disp + 16 ..].
        let off = (disp as i32 + 16) as usize;
        let mut v = [0u64; 2];
        for lane in 0..lanes {
            fill_lane(&mut v, lane, pick(&mut rng, pool), bits);
        }
        put128(&mut m.mem, off, v);
        out.push(m);
    }
    out
}



/// Sets lane `lane` (of width `bits`) of a 128-bit value. `lane` counts from the low end.
fn fill_lane(x: &mut [u64; 2], lane: usize, v: u64, bits: u32) {
    let per = (64 / bits) as usize;
    let (word, sh) = (lane / per, (lane % per) as u32 * bits);
    let mask = ((1u64 << bits) - 1) << sh;
    x[word] = (x[word] & !mask) | ((v << sh) & mask);
}

fn packuswb_cases() -> Vec<Case> {
    let mut v = cases("packuswb", Some(0x66), &[0x0F, 0x67], &pool16(), 16, 24, 0x1111);
    v.push(fixed("packuswb x0, x1 (saturation edges)", &[0x66, 0x0F, 0x67, 0xC1], [0x8000_7fff_0100_ffff, 0x8001_0000_00ff_fffe], [0x0001_8000_ffff_7fff, 0xff00_0101_0000_0001]));
    v
}

fn paddw_cases() -> Vec<Case> {
    let mut v = cases("paddw", Some(0x66), &[0x0F, 0xFD], &pool16(), 16, 24, 0x2222);
    v.push(fixed("paddw x0, x1 (wrap)", &[0x66, 0x0F, 0xFD, 0xC1], [0xffff_8000_7fff_0001, 0x8001_fffe_0100_0000], [0x0001_0001_0001_0001, 0x0001_0002_0100_0000]));
    v
}

fn addps_cases() -> Vec<Case> {
    cases("addps", None, &[0x0F, 0x58], &pool32(), 32, 24, 0x3333)
}

fn subps_cases() -> Vec<Case> {
    cases("subps", None, &[0x0F, 0x5C], &pool32(), 32, 24, 0x4444)
}

fn mulps_cases() -> Vec<Case> {
    cases("mulps", None, &[0x0F, 0x59], &pool32(), 32, 24, 0x5555)
}

fn minps_cases() -> Vec<Case> {
    cases("minps", None, &[0x0F, 0x5D], &pool32(), 32, 24, 0x6666)
}

fn maxps_cases() -> Vec<Case> {
    cases("maxps", None, &[0x0F, 0x5F], &pool32(), 32, 24, 0x7777)
}

/// A case whose xmm0 and xmm1 are given directly. The memory is zero.
fn fixed(name: &str, bytes: &[u8], lo: [u64; 2], hi: [u64; 2]) -> Case {
    let mut rng = Rng(0x9e37_79b9);
    let mut c = base(name.to_string(), bytes.to_vec(), &mut rng);
    c.xmm[0] = lo;
    c.xmm[1] = hi;
    c
}

#[test]
fn packuswb_matches_host() {
    check("packuswb", &packuswb_cases());
}

#[test]
fn paddw_matches_host() {
    check("paddw", &paddw_cases());
}

#[test]
fn addps_matches_host() {
    check("addps", &addps_cases());
}

#[test]
fn subps_matches_host() {
    check("subps", &subps_cases());
}

#[test]
fn mulps_matches_host() {
    check("mulps", &mulps_cases());
}

#[test]
fn minps_matches_host() {
    check("minps", &minps_cases());
}

#[test]
fn maxps_matches_host() {
    check("maxps", &maxps_cases());
}
