//! Tests for sse_legacy.rs (decision 0075): the legacy SSE/SSE2 forms that dsp_ippv2_x64.dll uses.
//!
//! The lifted C (clang -O0, header-only runtime fle_sse.h) and the host CPU run the same instruction on the
//! same state. All sixteen XMM registers, rax, rcx, rdx, rbx and rsi, and 64 bytes of memory must match bit for
//! bit (so a GPR result, its zero extension and a store are compared too). These instructions change no flags.
//! Inputs: lane edge values mixed with random lanes; every imm8 for the byte shifts, PEXTRW and PINSRW; every
//! sign pattern of MOVMSKPS and MOVMSKPD and many PMOVMSKB masks; r32 and r64 forms (REX.W); the memory operand
//! forms (16-byte aligned for m128, any address for m16 and MOVNTI). The host instruction is real machine code
//! run on the CPU (not intrinsics).

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
    let dir = crate_dir().join("../../out-sse2/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// Compiles `src` with clang -O0 into out-sse2/test, and returns the path. The runtime is
/// header-only for these instructions (fle_sse.h includes fle_sse_int2.h).
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

    let exe = compile(&format!("sse_int2_{group}"), &src);
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

/// Counts a case as changed when the host changed an XMM register, a GPR or memory (stores and GPR results too).
/// At least 80% of the cases must change something (the instruction really ran).
fn check(group: &str, cs: &[Case]) {
    check_min(group, cs, 80)
}

/// `min_pct`: the least percentage of cases that must change a register or memory. The exhaustive byte-pair sets of
/// PMAXUB and PMINUB leave the destination as it is in about half of the pairs (a >= b for max, a <= b for min), so
/// they use 50; every other group uses 80.
fn check_min(group: &str, cs: &[Case], min_pct: usize) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h.xmm != case.xmm || h.gpr != case.gpr || h.mem != case.mem {
            changed += 1;
        }
        if h != *cout {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?}\n  in   {:x?} {:x?}\n  C    {:x?} {:x?} {:?}\n  host {:x?} {:x?} {:?}",
                    case.name, case.bytes, case.xmm, case.gpr, cout.xmm, cout.gpr, cout.mem, h.xmm, h.gpr, h.mem
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed {changed}", cs.len());
    assert!(changed * 100 >= cs.len() * min_pct, "{group}: only {changed} of {} cases changed a register or memory (need {min_pct}%)", cs.len());
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

/// Lane edge values for a lane width: zero, one, the signed and unsigned edges, the
/// neighbours of the edges, and the sign-bit patterns.
fn pool(bits: u32) -> Vec<u64> {
    let m = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
    let top = 1u64 << (bits - 1);
    let mut v = vec![0, 1, 2, top - 1, top, top + 1, m, m - 1, m >> 1 >> 1, top >> 1, (m >> 4) << 4, 0x55 & m, 0xaa & m];
    if bits >= 16 {
        v.extend([0x7f, 0x80, 0xff, 0x100, 0x0101, 0xff00, 0x7f7f, 0x8080].map(|x| x & m));
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Sets lane `lane` (of width `bits`) of a 128-bit value. `lane` counts from the low end.
fn fill_lane(x: &mut [u64; 2], lane: usize, v: u64, bits: u32) {
    if bits == 64 {
        x[lane] = v;
        return;
    }
    let per = (64 / bits) as usize;
    let (word, sh) = (lane / per, (lane % per) as u32 * bits);
    let mask = ((1u64 << bits) - 1) << sh;
    x[word] = (x[word] & !mask) | ((v << sh) & mask);
}

/// Fills every lane of a 128-bit value: half the lanes from the edge pool, half random.
fn fill_all(x: &mut [u64; 2], bits: u32, rng: &mut Rng) {
    let p = pool(bits);
    for lane in 0..(128 / bits as usize) {
        let v = if rng.next_u64() & 1 == 0 { p[(rng.next_u64() % p.len() as u64) as usize] } else { rng.next_u64() };
        fill_lane(x, lane, v, bits);
    }
}

/// Two-operand form `op xmm_d, xmm_s` and `op xmm_d, [rdi + disp]`. `n` rounds; each round
/// gives one register case and one aligned memory case. The memory disp is one of -16, 0, 16, 32.
fn bin_cases(name: &str, prefix: Option<u8>, opc: &[u8], bits: u32, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..n {
        let d = (rng.next_u64() % 16) as u8;
        let s = if k % 7 == 0 { d } else { (rng.next_u64() % 16) as u8 };
        let mut c = base(format!("{name} x{d}, x{s} #{k}"), enc(prefix, opc, d, Rm::X(s)), &mut rng);
        fill_all(&mut c.xmm[d as usize], bits, &mut rng);
        if s != d {
            fill_all(&mut c.xmm[s as usize], bits, &mut rng);
        }
        out.push(c);
        let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
        let mut m = base(format!("{name} x{d}, [rdi{disp:+}] #{k}"), enc(prefix, opc, d, Rm::Mem(disp)), &mut rng);
        fill_all(&mut m.xmm[d as usize], bits, &mut rng);
        let mut v = [0u64; 2];
        fill_all(&mut v, bits, &mut rng);
        put128(&mut m.mem, (disp as i32 + 16) as usize, v);
        out.push(m);
    }
    out
}

/// Pairs that fix both operands lane by lane (all combinations of edge values for the lane
/// width), so that every saturation boundary is hit for sure: lane j of the destination
/// takes pool[(j + k) % len] and lane j of the source takes pool[(j * 3 + k / len) % len].
fn edge_cases(name: &str, prefix: Option<u8>, opc: &[u8], bits: u32) -> Vec<Case> {
    let p = pool(bits);
    let lanes = 128 / bits as usize;
    let mut rng = Rng(0xabcd_ef01 ^ opc[opc.len() - 1] as u64);
    let mut out = Vec::new();
    for k in 0..(p.len() * p.len()) {
        let (a, b) = (k % p.len(), k / p.len());
        let mut c = base(format!("{name} edge #{k}"), enc(prefix, opc, 0, Rm::X(1)), &mut rng);
        for lane in 0..lanes {
            fill_lane(&mut c.xmm[0], lane, p[(a + lane) % p.len()], bits);
            fill_lane(&mut c.xmm[1], lane, p[(b + lane * 3) % p.len()], bits);
        }
        out.push(c);
    }
    out
}

/// Like `enc`, with REX.W when `w` is set (the 64-bit GPR forms).
fn encw(prefix: Option<u8>, w: bool, opc: &[u8], reg: u8, rm: Rm) -> Vec<u8> {
    let (rm_low, rm_hi) = match rm {
        Rm::X(n) => (n & 7, n >> 3),
        Rm::Mem(_) => (7, 0),
    };
    let rex = 0x40 | ((w as u8) << 3) | (((reg >> 3) & 1) << 2) | rm_hi;
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

/// GPRs the test state keeps and compares: rax, rcx, rdx, rbx, rsi (rdi is the memory base).
const GPRS: [u8; 5] = [0, 1, 2, 3, 6];

/// Exhaustive byte pairs: 4096 cases, lane j of case k holds the pair index k * 16 + j (a = high byte, b = low
/// byte), so every (a, b) of 8 bits meets in some lane, once, for the register form.
fn byte_pair_cases(name: &str, opc: u8, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..4096usize {
        let mut c = base(format!("{name} bytes #{k}"), enc(Some(0x66), &[0x0F, opc], 0, Rm::X(1)), &mut rng);
        for lane in 0..16usize {
            let idx = k * 16 + lane;
            fill_lane(&mut c.xmm[0], lane, (idx >> 8) as u64, 8);
            fill_lane(&mut c.xmm[1], lane, (idx & 255) as u64, 8);
        }
        out.push(c);
    }
    out
}

/// A register case, a case with the destination as the source, and an aligned memory case, per round.
fn packed_group(name: &str, prefix: Option<u8>, opc: u8, bits: u32, seed: u64) -> Vec<Case> {
    let mut v = bin_cases(name, prefix, &[0x0F, opc], bits, 400, seed);
    v.extend(edge_cases(name, prefix, &[0x0F, opc], bits));
    v
}

macro_rules! packed_test {
    ($fn:ident, $name:expr, $prefix:expr, $opc:expr, $bits:expr, $seed:expr) => {
        #[test]
        fn $fn() {
            check($name, &packed_group($name, $prefix, $opc, $bits, $seed));
        }
    };
}

packed_test!(orps_matches_host, "orps", None, 0x56, 32, 0x5001);
packed_test!(andnps_matches_host, "andnps", None, 0x55, 32, 0x5002);
packed_test!(pcmpeqb_matches_host, "pcmpeqb", Some(0x66), 0x74, 8, 0x5003);
packed_test!(pcmpeqw_matches_host, "pcmpeqw", Some(0x66), 0x75, 16, 0x5004);
packed_test!(pcmpgtb_matches_host, "pcmpgtb", Some(0x66), 0x64, 8, 0x5005);
packed_test!(pcmpgtw_matches_host, "pcmpgtw", Some(0x66), 0x65, 16, 0x5006);
packed_test!(paddb_matches_host, "paddb", Some(0x66), 0xFC, 8, 0x5007);
packed_test!(psubb_matches_host, "psubb", Some(0x66), 0xF8, 8, 0x5008);
packed_test!(psubq_matches_host, "psubq", Some(0x66), 0xFB, 64, 0x5009);
packed_test!(pmaxub_matches_host, "pmaxub", Some(0x66), 0xDE, 8, 0x500a);
packed_test!(pminub_matches_host, "pminub", Some(0x66), 0xDA, 8, 0x500b);
packed_test!(pmaddwd_matches_host, "pmaddwd", Some(0x66), 0xF5, 16, 0x500c);
packed_test!(pmuludq_matches_host, "pmuludq", Some(0x66), 0xF4, 32, 0x500d);

macro_rules! bytes_test {
    ($fn:ident, $name:expr, $opc:expr, $seed:expr) => {
        bytes_test!($fn, $name, $opc, $seed, 80);
    };
    ($fn:ident, $name:expr, $opc:expr, $seed:expr, $pct:expr) => {
        #[test]
        fn $fn() {
            check_min($name, &byte_pair_cases($name, $opc, $seed), $pct);
        }
    };
}

bytes_test!(pcmpeqb_all_byte_pairs_match_host, "pcmpeqb_all", 0x74, 0x5101);
bytes_test!(pcmpgtb_all_byte_pairs_match_host, "pcmpgtb_all", 0x64, 0x5102);
bytes_test!(paddb_all_byte_pairs_match_host, "paddb_all", 0xFC, 0x5103);
bytes_test!(psubb_all_byte_pairs_match_host, "psubb_all", 0xF8, 0x5104);
bytes_test!(pmaxub_all_byte_pairs_match_host, "pmaxub_all", 0xDE, 0x5105, 50);
bytes_test!(pminub_all_byte_pairs_match_host, "pminub_all", 0xDA, 0x5106, 50);

/// PMADDWD corner: 0x8000 * 0x8000 in both words of a pair wraps to 0x80000000 (the only wrapping sum).
#[test]
fn pmaddwd_wrap_corner_matches_host() {
    let mut rng = Rng(0x5201);
    let mut out = Vec::new();
    for k in 0..64usize {
        let mut c = base(format!("pmaddwd corner #{k}"), enc(Some(0x66), &[0x0F, 0xF5], 0, Rm::X(1)), &mut rng);
        for lane in 0..8usize {
            let v = [0x8000u64, 0x7fff, 0xffff, 0x0001, 0x8001][(k + lane) % 5];
            let w = [0x8000u64, 0x7fff, 0x8000, 0xffff, 0x0000][(k / 5 + lane) % 5];
            fill_lane(&mut c.xmm[0], lane, v, 16);
            fill_lane(&mut c.xmm[1], lane, w, 16);
        }
        out.push(c);
    }
    check("pmaddwd_corner", &out);
}

/// PSRLDQ (/3) and PSLLDQ (/7): every imm8 (0..=255, so the count above 15 is covered), random destination.
fn byteshift_group(name: &str, ext: u8, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for imm in 0..=255u8 {
        for rep in 0..2 {
            let d = (rng.next_u64() % 16) as u8;
            let mut bytes = enc(Some(0x66), &[0x0F, 0x73], ext, Rm::X(d));
            bytes.push(imm);
            let mut c = base(format!("{name} x{d}, {imm} #{rep}"), bytes, &mut rng);
            // distinct non-zero bytes so that a misplaced byte shows up
            c.xmm[d as usize] = [rng.next_u64() | 0x0101_0101_0101_0101, rng.next_u64() | 0x0101_0101_0101_0101];
            out.push(c);
        }
    }
    out
}

#[test]
fn psrldq_matches_host() {
    check("psrldq", &byteshift_group("psrldq", 3, 0x5301));
}

#[test]
fn pslldq_matches_host() {
    check("pslldq", &byteshift_group("pslldq", 7, 0x5302));
}

/// reg, xmm masks. `lane_bits`: width of the lane whose top bit is the mask bit (8, 32 or 64). `masks` are the
/// sign patterns to build; the other bits of each lane are random.
fn mask_group(name: &str, prefix: Option<u8>, opc: u8, lane_bits: u32, masks: &[u32], seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for (k, &m) in masks.iter().enumerate() {
        for w in [false, true] {
            let r = GPRS[(rng.next_u64() % 5) as usize];
            let s = (rng.next_u64() % 16) as u8;
            let mut c = base(format!("{name} r{}{}, x{s} mask {m:#x} #{k}", r, if w { "(64)" } else { "(32)" }), encw(prefix, w, &[0x0F, opc], r, Rm::X(s)), &mut rng);
            let mut x = [rng.next_u64(), rng.next_u64()];
            let lanes = 128 / lane_bits;
            for lane in 0..lanes {
                let top = if (m >> lane) & 1 == 1 { 1u64 } else { 0 };
                let cur = {
                    let per = (64 / lane_bits) as usize;
                    (x[lane as usize / per] >> ((lane as usize % per) as u32 * lane_bits)) & ((1u64 << lane_bits.min(63)) - 1 | if lane_bits == 64 { 1 << 63 } else { 0 })
                };
                let v = (cur & !(1u64 << (lane_bits - 1))) | (top << (lane_bits - 1));
                fill_lane(&mut x, lane as usize, v, lane_bits);
            }
            c.xmm[s as usize] = x;
            out.push(c);
        }
    }
    out
}

fn mask_set(bits: u32, extra_random: usize, seed: u64) -> Vec<u32> {
    let full = if bits == 32 { u32::MAX } else { (1u32 << bits) - 1 };
    let mut v: Vec<u32> = Vec::new();
    if bits <= 4 {
        v.extend(0..=full);
    } else {
        v.extend([0, full, 0x5555 & full, 0xaaaa & full, 0x00ff & full, 0xff00 & full]);
        for b in 0..bits {
            v.push(1 << b);
            v.push(full & !(1 << b));
        }
        let mut rng = Rng(seed);
        for _ in 0..extra_random {
            v.push((rng.next_u64() as u32) & full);
        }
    }
    v
}

#[test]
fn pmovmskb_matches_host() {
    check("pmovmskb", &mask_group("pmovmskb", Some(0x66), 0xD7, 8, &mask_set(16, 600, 1), 0x5401));
}

#[test]
fn movmskps_matches_host() {
    // all 16 sign patterns, each many times (random other bits and registers)
    let mut m = Vec::new();
    for rep in 0..20 {
        let _ = rep;
        m.extend(mask_set(4, 0, 0));
    }
    check("movmskps", &mask_group("movmskps", None, 0x50, 32, &m, 0x5402));
}

#[test]
fn movmskpd_matches_host() {
    let mut m = Vec::new();
    for rep in 0..40 {
        let _ = rep;
        m.extend(mask_set(2, 0, 0));
    }
    check("movmskpd", &mask_group("movmskpd", Some(0x66), 0x50, 64, &m, 0x5403));
}

/// PEXTRW r32/r64, xmm, imm8 (66 0F C5): every imm8 (the index is imm8[2:0]), r32 and r64 forms.
#[test]
fn pextrw_matches_host() {
    let mut rng = Rng(0x5501);
    let mut out = Vec::new();
    for imm in 0..=255u8 {
        for w in [false, true] {
            let r = GPRS[(rng.next_u64() % 5) as usize];
            let s = (rng.next_u64() % 16) as u8;
            let mut bytes = encw(Some(0x66), w, &[0x0F, 0xC5], r, Rm::X(s));
            bytes.push(imm);
            let mut c = base(format!("pextrw r{r}{}, x{s}, {imm}", if w { "(64)" } else { "(32)" }), bytes, &mut rng);
            // distinct words, with the sign bit and the high byte set in some
            c.xmm[s as usize] = [rng.next_u64() | 0x8001_0001_0001_0001, rng.next_u64() | 0x0001_0001_0001_8001];
            out.push(c);
        }
    }
    check("pextrw", &out);
}

/// PINSRW xmm, r32 / m16, imm8 (66 0F C4): every imm8, the register form (also with REX.W, which is ignored), and the
/// m16 form at aligned and unaligned addresses (no alignment requirement).
#[test]
fn pinsrw_matches_host() {
    let mut rng = Rng(0x5601);
    let mut out = Vec::new();
    for imm in 0..=255u8 {
        let d = (rng.next_u64() % 16) as u8;
        let r = GPRS[(rng.next_u64() % 5) as usize];
        for w in [false, true] {
            let mut bytes = encw(Some(0x66), w, &[0x0F, 0xC4], d, Rm::X(r));
            bytes.push(imm);
            let mut c = base(format!("pinsrw x{d}, r{r}{}, {imm}", if w { "(64)" } else { "(32)" }), bytes, &mut rng);
            c.gpr[GPRS.iter().position(|&g| g == r).unwrap()] = rng.next_u64() | 0xffff_0000_0000_0000;
            out.push(c);
        }
        let disp = (rng.next_u64() % 47) as i8 - 16;
        let mut bytes = enc(Some(0x66), &[0x0F, 0xC4], d, Rm::Mem(disp));
        bytes.push(imm);
        out.push(base(format!("pinsrw x{d}, [rdi{disp:+}], {imm}"), bytes, &mut rng));
    }
    check("pinsrw", &out);
}

/// MOVNTPS (0F 2B), MOVNTPD (66 0F 2B), MOVNTDQ (66 0F E7): m128, xmm at 16-byte aligned addresses.
fn movnt_group(name: &str, prefix: Option<u8>, opc: u8, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..300usize {
        let s = (rng.next_u64() % 16) as u8;
        let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
        let mut c = base(format!("{name} [rdi{disp:+}], x{s} #{k}"), enc(prefix, &[0x0F, opc], s, Rm::Mem(disp)), &mut rng);
        fill_all(&mut c.xmm[s as usize], 32, &mut rng);
        out.push(c);
    }
    out
}

#[test]
fn movntps_matches_host() {
    check("movntps", &movnt_group("movntps", None, 0x2B, 0x5701));
}

#[test]
fn movntpd_matches_host() {
    check("movntpd", &movnt_group("movntpd", Some(0x66), 0x2B, 0x5702));
}

#[test]
fn movntdq_matches_host() {
    check("movntdq", &movnt_group("movntdq", Some(0x66), 0xE7, 0x5703));
}

/// MOVNTI m32, r32 and m64, r64 (0F C3, REX.W for 64 bits) at any address, including unaligned ones.
#[test]
fn movnti_matches_host() {
    let mut rng = Rng(0x5801);
    let mut out = Vec::new();
    for k in 0..400usize {
        let r = GPRS[(rng.next_u64() % 5) as usize];
        let w = k % 2 == 1;
        let disp = (rng.next_u64() % 41) as i8 - 16;
        let c = base(format!("movnti [rdi{disp:+}], r{r}{} #{k}", if w { "(64)" } else { "(32)" }), encw(None, w, &[0x0F, 0xC3], r, Rm::Mem(disp)), &mut rng);
        out.push(c);
    }
    check("movnti", &out);
}

/// Float edge patterns for RSQRT lanes: zeros, denormals, smallest and largest normals, infinities, SNaN and QNaN of
/// both signs, negative normals, and exponents of both parities.
fn rsqrt_pool() -> Vec<u32> {
    vec![
        0x0000_0000, 0x8000_0000, 0x0000_0001, 0x007f_ffff, 0x8000_0001, 0x807f_ffff, 0x0040_0000, 0x0080_0000, 0x8080_0000,
        0x0080_0001, 0x7f7f_ffff, 0xff7f_ffff, 0x7f80_0000, 0xff80_0000, 0x7f80_0001, 0x7fc0_0000, 0xff80_0001, 0xffc0_0000,
        0x7fff_ffff, 0xffff_ffff, 0x3f80_0000, 0xbf80_0000, 0x4000_0000, 0x3f7f_ffff, 0x3f80_0001, 0x4b00_0000, 0x1234_5678,
        0xabcd_ef01, 0x0100_0000, 0x0180_0000, 0x7e80_0000, 0x7f00_0000,
    ]
}

fn rsqrt_lanes(rng: &mut Rng, k: usize) -> [u64; 2] {
    let p = rsqrt_pool();
    let mut l = [0u32; 4];
    for (j, x) in l.iter_mut().enumerate() {
        *x = match (k + j) % 4 {
            0 => p[(k * 5 + j * 7) % p.len()],
            1 => ((rng.next_u64() as u32) & 0x807f_ffff) | ((1 + (rng.next_u64() % 254) as u32) << 23),
            2 => rng.next_u64() as u32,
            _ => ((1 + (rng.next_u64() % 254) as u32) << 23) | ((rng.next_u64() as u32) & 0x7f_ffff),
        };
    }
    [l[0] as u64 | (l[1] as u64) << 32, l[2] as u64 | (l[3] as u64) << 32]
}

/// RSQRTPS xmm, xmm and xmm, [aligned]: edge lanes, every exponent field value 0..=255 and random lanes.
#[test]
fn rsqrtps_matches_host() {
    let mut rng = Rng(0x5901);
    let mut out = Vec::new();
    for k in 0..1500usize {
        let d = (rng.next_u64() % 16) as u8;
        let s = (rng.next_u64() % 16) as u8;
        let mut c = base(format!("rsqrtps x{d}, x{s} #{k}"), enc(None, &[0x0F, 0x52], d, Rm::X(s)), &mut rng);
        c.xmm[s as usize] = rsqrt_lanes(&mut rng, k);
        out.push(c);
        let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
        let mut m = base(format!("rsqrtps x{d}, [rdi{disp:+}] #{k}"), enc(None, &[0x0F, 0x52], d, Rm::Mem(disp)), &mut rng);
        put128(&mut m.mem, (disp as i32 + 16) as usize, rsqrt_lanes(&mut rng, k + 1));
        out.push(m);
    }
    for e in 0..256u32 {
        let mut c = base(format!("rsqrtps exp {e}"), enc(None, &[0x0F, 0x52], 0, Rm::X(1)), &mut rng);
        let l = [0, 1, 2, 3].map(|_| ((rng.next_u64() as u32) & 0x807f_ffff) | (e << 23));
        c.xmm[1] = [l[0] as u64 | (l[1] as u64) << 32, l[2] as u64 | (l[3] as u64) << 32];
        out.push(c);
    }
    check("rsqrtps", &out);
}

/// RSQRTSS xmm, xmm and xmm, [m32] (any address): only the low float changes.
#[test]
fn rsqrtss_matches_host() {
    let mut rng = Rng(0x5902);
    let mut out = Vec::new();
    for k in 0..1500usize {
        let d = (rng.next_u64() % 16) as u8;
        let s = (rng.next_u64() % 16) as u8;
        let mut c = base(format!("rsqrtss x{d}, x{s} #{k}"), enc(Some(0xF3), &[0x0F, 0x52], d, Rm::X(s)), &mut rng);
        c.xmm[s as usize] = rsqrt_lanes(&mut rng, k);
        out.push(c);
        let disp = (rng.next_u64() % 45) as i8 - 16;
        let mut m = base(format!("rsqrtss x{d}, [rdi{disp:+}] #{k}"), enc(Some(0xF3), &[0x0F, 0x52], d, Rm::Mem(disp)), &mut rng);
        let v = rsqrt_lanes(&mut rng, k + 1)[0] as u32;
        m.mem[(disp as i32 + 16) as usize..(disp as i32 + 20) as usize].copy_from_slice(&v.to_le_bytes());
        out.push(m);
    }
    check("rsqrtss", &out);
}

fn runtime_dir() -> PathBuf {
    crate_dir().join("../../runtime")
}

/// All 2^32 float bit patterns, the lifted helper (fle_lg_rsqrtps) against the host RSQRTPS, under six MXCSR values
/// (default, FTZ+DAZ and all masks, RC = 01, 10 and 11, and all masks clear). Also checks that the host sets no
/// MXCSR status flag. About a minute when run on one core.
///
/// Decision 0149: the same program and the same check, run in parallel. The 2^32 patterns are cut into RSQRT_SLICES slices of equal length (the
/// program takes RSQRT_FROM / RSQRT_TO from the environment) and every (MXCSR value, slice) pair is one run of the program; at most FLE_TEST_THREADS runs at a time
/// (default: half of the available cores). The test asserts that the slices touch (the first starts at 0, the last ends at 2^32, every end is the next start,
/// every boundary is a multiple of 4: no gap, no overlap), that each run reports exactly `end - start` inputs, and, as before, that every MXCSR value has
/// seen 4294967296 inputs with 0 mismatches and 0 flag changes in the sum of its slices (so a missing or doubled slice fails the last assertion too).
const RSQRT_SLICES: u64 = 16;
const RSQRT_MXCSR: [&str; 6] = ["1f80", "9fc0", "7f80", "3f80", "5f80", "0000"];

fn rsqrt_slices() -> Vec<(u64, u64)> {
    (0..RSQRT_SLICES).map(|i| (i * (1u64 << 32) / RSQRT_SLICES, (i + 1) * (1u64 << 32) / RSQRT_SLICES)).collect()
}

/// FLE_TEST_THREADS, else half of the available cores (at least 1).
fn test_threads() -> usize {
    std::env::var("FLE_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2) / 2).max(1))
}

#[test]
fn rsqrtps_all_2e32_inputs_match_host() {
    let dir = out_dir();
    let exe = dir.join(etxt_common::uniq("rsqrt_exhaustive_o2"));
    let cc = Command::new("clang")
        .args(["-O2", "-std=gnu11", "-w", "-I"])
        .arg(runtime_dir())
        .arg("-o")
        .arg(&exe)
        .arg(crate_dir().join("tests/rsqrt/rsqrt_exhaustive.c"))
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let slices = rsqrt_slices();
    assert_eq!(slices.len() as u64, RSQRT_SLICES);
    assert_eq!(slices.first().unwrap().0, 0, "the first slice starts at pattern 0");
    assert_eq!(slices.last().unwrap().1, 1u64 << 32, "the last slice ends at 2^32");
    for w in slices.windows(2) {
        assert_eq!(w[0].1, w[1].0, "slices must touch: no gap and no overlap");
    }
    for &(a, b) in &slices {
        assert!(a < b && a % 4 == 0 && b % 4 == 0, "slice {a}..{b}: the program takes four lanes per step");
    }
    let items: Vec<(usize, usize)> = (0..RSQRT_MXCSR.len()).flat_map(|m| (0..slices.len()).map(move |s| (m, s))).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(vec![None::<(bool, String)>; items.len()]);
    std::thread::scope(|sc| {
        for _ in 0..test_threads().min(items.len()) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= items.len() {
                    break;
                }
                let (m, s) = items[i];
                let run = Command::new(&exe)
                    .arg(RSQRT_MXCSR[m])
                    .env("RSQRT_FROM", slices[s].0.to_string())
                    .env("RSQRT_TO", slices[s].1.to_string())
                    .output()
                    .expect("run");
                results.lock().unwrap()[i] = Some((run.status.success(), String::from_utf8_lossy(&run.stdout).to_string()));
            });
        }
    });
    let results = results.into_inner().unwrap();
    let mut text = String::new();
    let mut all_ok = true;
    for m in 0..RSQRT_MXCSR.len() {
        let (mut inputs, mut bad, mut flag) = (0u64, 0u64, 0u64);
        let mut mx = String::new();
        for s in 0..slices.len() {
            let (ok, out) = results[m * slices.len() + s].as_ref().expect("every run has finished");
            all_ok &= *ok;
            for line in out.lines() {
                if let Some(rest) = line.strip_prefix("rsqrtps_exhaustive mxcsr ") {
                    // "<mxcsr> inputs <n> mismatches <k> flag_changes <f>"
                    let f: Vec<&str> = rest.split_whitespace().collect();
                    assert!(f.len() == 7 && f[1] == "inputs" && f[3] == "mismatches" && f[5] == "flag_changes", "odd line {line:?}");
                    let (n, k, c): (u64, u64, u64) = (f[2].parse().unwrap(), f[4].parse().unwrap(), f[6].parse().unwrap());
                    assert_eq!(n, slices[s].1 - slices[s].0, "mxcsr {} slice {s}: the run checked another number of inputs than its range", RSQRT_MXCSR[m]);
                    mx = f[0].to_string();
                    inputs += n;
                    bad += k;
                    flag += c;
                } else {
                    text.push_str(line);
                    text.push('\n');
                }
            }
        }
        text.push_str(&format!("rsqrtps_exhaustive mxcsr {mx} inputs {inputs} mismatches {bad} flag_changes {flag}\n"));
    }
    eprintln!("{text}");
    assert!(all_ok, "mismatches against the host: {text}");
    assert_eq!(text.matches("inputs 4294967296 mismatches 0 flag_changes 0").count(), 6, "{text}");
}

/// A 128-bit memory operand that is not 16-byte aligned is an error (SDM: #GP(0)): the generated C aborts with the
/// 16-byte message and does not read or write the memory. The host would raise #GP, so only the C side is run.
#[test]
fn misaligned_memory_operand_aborts() {
    for (name, bytes) in [
        ("orps", enc(None, &[0x0F, 0x56], 0, Rm::Mem(1))),
        ("andnps", enc(None, &[0x0F, 0x55], 2, Rm::Mem(8))),
        ("pcmpeqb", enc(Some(0x66), &[0x0F, 0x74], 0, Rm::Mem(4))),
        ("pcmpgtw", enc(Some(0x66), &[0x0F, 0x65], 1, Rm::Mem(-1))),
        ("paddb", enc(Some(0x66), &[0x0F, 0xFC], 1, Rm::Mem(2))),
        ("psubq", enc(Some(0x66), &[0x0F, 0xFB], 1, Rm::Mem(12))),
        ("pmaxub", enc(Some(0x66), &[0x0F, 0xDE], 1, Rm::Mem(3))),
        ("pmaddwd", enc(Some(0x66), &[0x0F, 0xF5], 1, Rm::Mem(5))),
        ("pmuludq", enc(Some(0x66), &[0x0F, 0xF4], 1, Rm::Mem(7))),
        ("rsqrtps", enc(None, &[0x0F, 0x52], 1, Rm::Mem(6))),
        ("movntps", enc(None, &[0x0F, 0x2B], 0, Rm::Mem(8))),
        ("movntpd", enc(Some(0x66), &[0x0F, 0x2B], 0, Rm::Mem(1))),
        ("movntdq", enc(Some(0x66), &[0x0F, 0xE7], 0, Rm::Mem(15))),
    ] {
        let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = compile(&format!("sse_legacy_misaligned_{name}"), &src);
        let run = Command::new(&exe).output().expect("run test binary");
        assert!(!run.status.success(), "{name}: a misaligned operand must abort");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(err.contains("misaligned 16-byte access"), "{name}: stderr {err}");
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}: returned");
    }
}

/// Forms that stay unsupported: the MMX forms (not in this unit), the VEX forms, PEXTRW's SSE4.1 forms (66 0F 3A 15,
/// which CPUID does not claim) and PINSRW and friends with an operand shape that does not exist.
#[test]
fn other_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 11] = [
        ("pcmpeqb mm0, mm1", &[0x0F, 0x74, 0xC1]),
        ("pmovmskb eax, mm0", &[0x0F, 0xD7, 0xC0]),
        ("pextrw eax, mm0, 1", &[0x0F, 0xC5, 0xC0, 0x01]),
        ("pinsrw mm0, eax, 1", &[0x0F, 0xC4, 0xC0, 0x01]),
        ("psubq mm0, mm1", &[0x0F, 0xFB, 0xC1]),
        ("pextrw eax, xmm0, 1 (66 0F 3A 15 /r, SSE4.1)", &[0x66, 0x0F, 0x3A, 0x15, 0xC0, 0x01]),
        ("pextrw [rdi], xmm0, 1 (66 0F 3A 15 /r, SSE4.1)", &[0x66, 0x0F, 0x3A, 0x15, 0x07, 0x01]),
        ("vpmovmskb eax, xmm0", &[0xC5, 0xF9, 0xD7, 0xC0]),
        ("vpcmpeqb xmm0, xmm1, xmm2", &[0xC5, 0xF1, 0x74, 0xC2]),
        ("vorps xmm0, xmm1, xmm2", &[0xC5, 0xF0, 0x56, 0xC2]),
        // decision 0190: the xmm form of PSHUFB is lifted now (sse_ssse3.rs; Elastique executes SSSE3 without asking CPUID), so the pin keeps the MMX form (no 66 prefix), which stays unsupported
        ("pshufb mm0, mm1 (SSSE3, MMX form)", &[0x0F, 0x38, 0x00, 0xC1]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}

/// The case sets reach the edges the checks rely on.
#[test]
fn case_sets_cover_the_edges() {
    assert_eq!(byte_pair_cases("t", 0x74, 1).len(), 4096);
    // every (a, b) pair of bytes appears once in the register form
    let cs = byte_pair_cases("t", 0x74, 1);
    let mut seen = vec![false; 65536];
    for c in &cs {
        for lane in 0..16usize {
            let get = |x: &[u64; 2]| ((x[lane / 8] >> ((lane % 8) * 8)) & 0xff) as usize;
            seen[get(&c.xmm[0]) << 8 | get(&c.xmm[1])] = true;
        }
    }
    assert!(seen.iter().all(|&s| s), "all 65536 byte pairs");
    assert_eq!(byteshift_group("psrldq", 3, 1).len(), 512);
    assert_eq!(mask_set(4, 0, 0).len(), 16);
    assert!(mask_set(16, 600, 1).len() >= 600 + 38);
}
