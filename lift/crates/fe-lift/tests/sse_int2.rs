//! Tests for sse_int2.rs (decision 0026): packed SSE2 integer instructions on XMM registers.
//!
//! The lifted C (clang -O0, header-only runtime fle_sse.h) and the host CPU run the same
//! instruction on the same state. All sixteen XMM registers, rax, rcx, rdx, rbx and rsi, and
//! 64 bytes of memory must match bit for bit. These instructions change no flags.
//! Inputs: lane edge values (saturation, sign, zero) mixed with random lanes; shift counts
//! below, at and above the lane width, with the high half of the 64-bit count set; every
//! imm8 for the shifts and the two word shuffles; the memory operand forms (16-byte aligned).
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

fn check(group: &str, cs: &[Case]) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h.xmm != case.xmm {
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
    // the instruction really ran on the host and changed a register in most cases
    assert!(changed * 10 >= cs.len() * 8, "{group}: only {changed} of {} cases changed an XMM register", cs.len());
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

fn bin_group(name: &str, opc: u8, bits: u32, seed: u64) -> Vec<Case> {
    let mut v = bin_cases(name, Some(0x66), &[0x0F, opc], bits, 300, seed);
    v.extend(edge_cases(name, Some(0x66), &[0x0F, opc], bits));
    v
}

/// Shift counts: below, at and above each lane width, with bits set in the high half of the
/// 64-bit count, and random values. Only the low 64 bits of the count operand count.
fn counts(rng: &mut Rng) -> Vec<u64> {
    let mut v: Vec<u64> = vec![
        0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 255, 256, 0xffff, 0x1_0000, 0x7fff_ffff,
        0x8000_0000, 0xffff_ffff, 0x1_0000_0000, 0x1_0000_0001, 0x8000_0000_0000_0000, u64::MAX, 0xffff_ffff_0000_0003,
    ];
    for _ in 0..12 {
        v.push(rng.next_u64() % 72);
    }
    for _ in 0..6 {
        v.push(rng.next_u64());
    }
    v
}

/// Shift by xmm count, by m128 count (aligned), and by imm8 (all 256 values).
/// `ext` is the modrm.reg extension of the imm8 form, `imm_opc` its opcode byte.
fn shift_group(name: &str, count_opc: u8, imm_opc: u8, ext: u8, bits: u32, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    let cs = counts(&mut rng);
    for (k, &cnt) in cs.iter().enumerate() {
        for rep in 0..3 {
            let d = (rng.next_u64() % 16) as u8;
            let mut s = (rng.next_u64() % 16) as u8;
            if s == d {
                s = (s + 1) % 16;
            }
            let mut c = base(format!("{name} x{d}, x{s} cnt {cnt:#x} #{k}.{rep}"), enc(Some(0x66), &[0x0F, count_opc], d, Rm::X(s)), &mut rng);
            fill_all(&mut c.xmm[d as usize], bits, &mut rng);
            c.xmm[s as usize][0] = cnt;
            out.push(c);
            let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
            let mut m = base(format!("{name} x{d}, [rdi{disp:+}] cnt {cnt:#x} #{k}.{rep}"), enc(Some(0x66), &[0x0F, count_opc], d, Rm::Mem(disp)), &mut rng);
            fill_all(&mut m.xmm[d as usize], bits, &mut rng);
            put128(&mut m.mem, (disp as i32 + 16) as usize, [cnt, rng.next_u64()]);
            out.push(m);
        }
    }
    for imm in 0..=255u8 {
        let d = (rng.next_u64() % 16) as u8;
        let mut bytes = enc(Some(0x66), &[0x0F, imm_opc], ext, Rm::X(d));
        bytes.push(imm);
        let mut c = base(format!("{name} x{d}, {imm}"), bytes, &mut rng);
        fill_all(&mut c.xmm[d as usize], bits, &mut rng);
        out.push(c);
    }
    out
}

/// PSHUFLW (F2) and PSHUFHW (F3): every imm8, register and aligned memory source.
fn shuf_group(name: &str, prefix: u8, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for imm in 0..=255u8 {
        let d = (rng.next_u64() % 16) as u8;
        let s = (rng.next_u64() % 16) as u8;
        let mut bytes = enc(Some(prefix), &[0x0F, 0x70], d, Rm::X(s));
        bytes.push(imm);
        let mut c = base(format!("{name} x{d}, x{s}, {imm:#x}"), bytes, &mut rng);
        // distinct words so that a wrong lane shows up
        c.xmm[s as usize] = [rng.next_u64() | 0x0001_0001_0001_0001, rng.next_u64() | 0x0001_0001_0001_0001];
        out.push(c);
        let disp = [-16i8, 0, 16, 32][(rng.next_u64() % 4) as usize];
        let mut bytes = enc(Some(prefix), &[0x0F, 0x70], d, Rm::Mem(disp));
        bytes.push(imm);
        let mut m = base(format!("{name} x{d}, [rdi{disp:+}], {imm:#x}"), bytes, &mut rng);
        put128(&mut m.mem, (disp as i32 + 16) as usize, [rng.next_u64(), rng.next_u64()]);
        out.push(m);
    }
    out
}

macro_rules! bin_test {
    ($fn:ident, $name:expr, $opc:expr, $bits:expr, $seed:expr) => {
        #[test]
        fn $fn() {
            check($name, &bin_group($name, $opc, $bits, $seed));
        }
    };
}

bin_test!(paddq_matches_host, "paddq", 0xD4, 64, 0x1001);
bin_test!(paddsb_matches_host, "paddsb", 0xEC, 8, 0x1002);
bin_test!(paddsw_matches_host, "paddsw", 0xED, 16, 0x1003);
bin_test!(paddusb_matches_host, "paddusb", 0xDC, 8, 0x1004);
bin_test!(paddusw_matches_host, "paddusw", 0xDD, 16, 0x1005);
bin_test!(psubsb_matches_host, "psubsb", 0xE8, 8, 0x1006);
bin_test!(psubsw_matches_host, "psubsw", 0xE9, 16, 0x1007);
bin_test!(psubusb_matches_host, "psubusb", 0xD8, 8, 0x1008);
bin_test!(psubusw_matches_host, "psubusw", 0xD9, 16, 0x1009);
bin_test!(pmaxsw_matches_host, "pmaxsw", 0xEE, 16, 0x100a);
bin_test!(pminsw_matches_host, "pminsw", 0xEA, 16, 0x100b);
bin_test!(pmulhw_matches_host, "pmulhw", 0xE5, 16, 0x100c);
bin_test!(pmullw_matches_host, "pmullw", 0xD5, 16, 0x100d);
bin_test!(packsswb_matches_host, "packsswb", 0x63, 16, 0x100e);
bin_test!(packssdw_matches_host, "packssdw", 0x6B, 32, 0x100f);

macro_rules! shift_test {
    ($fn:ident, $name:expr, $copc:expr, $iopc:expr, $ext:expr, $bits:expr, $seed:expr) => {
        #[test]
        fn $fn() {
            check($name, &shift_group($name, $copc, $iopc, $ext, $bits, $seed));
        }
    };
}

shift_test!(psllw_matches_host, "psllw", 0xF1, 0x71, 6, 16, 0x2001);
shift_test!(pslld_matches_host, "pslld", 0xF2, 0x72, 6, 32, 0x2002);
shift_test!(psllq_matches_host, "psllq", 0xF3, 0x73, 6, 64, 0x2003);
shift_test!(psrlw_matches_host, "psrlw", 0xD1, 0x71, 2, 16, 0x2004);
shift_test!(psrld_matches_host, "psrld", 0xD2, 0x72, 2, 32, 0x2005);
shift_test!(psrlq_matches_host, "psrlq", 0xD3, 0x73, 2, 64, 0x2006);
shift_test!(psraw_matches_host, "psraw", 0xE1, 0x71, 4, 16, 0x2007);
shift_test!(psrad_matches_host, "psrad", 0xE2, 0x72, 4, 32, 0x2008);

#[test]
fn pshuflw_matches_host() {
    check("pshuflw", &shuf_group("pshuflw", 0xF2, 0x3001));
}

#[test]
fn pshufhw_matches_host() {
    check("pshufhw", &shuf_group("pshufhw", 0xF3, 0x3002));
}

#[test]
fn case_sets_cover_the_edges() {
    // The shift counts reach every region (below, at, above the width; high half set).
    let cs = shift_group("psllw", 0xF1, 0x71, 6, 16, 1);
    let big = cs.iter().filter(|c| c.name.contains("cnt") && !c.name.contains("[rdi")).count();
    assert!(big >= 100, "register-count cases: {big}");
    assert!(cs.iter().any(|c| c.name.contains("cnt 0x100000000")));
    assert!(cs.iter().any(|c| c.name.contains("cnt 0xffffffffffffffff")));
    // every lane-width pool holds the signed and unsigned edges
    for bits in [8u32, 16, 32, 64] {
        let p = pool(bits);
        let m = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
        let top = 1u64 << (bits - 1);
        for e in [0, top - 1, top, m] {
            assert!(p.contains(&e), "pool({bits}) lacks {e:#x}");
        }
    }
    assert_eq!(shuf_group("pshuflw", 0xF2, 1).len(), 512);
}

/// A memory operand that is not 16-byte aligned is an error (SDM: #GP(0)): the generated C
/// aborts with the 16-byte message; it does not read the memory. The host would raise #GP, so
/// only the C side is run. Both the packed and the shift (m128 count) forms.
#[test]
fn misaligned_memory_operand_aborts() {
    for (name, bytes) in [
        ("paddsw", enc(Some(0x66), &[0x0F, 0xED], 0, Rm::Mem(8))),
        ("pmullw", enc(Some(0x66), &[0x0F, 0xD5], 3, Rm::Mem(1))),
        ("psllw", enc(Some(0x66), &[0x0F, 0xF1], 0, Rm::Mem(4))),
        ("psrad", enc(Some(0x66), &[0x0F, 0xE2], 2, Rm::Mem(-1))),
        ("pshuflw", {
            let mut b = enc(Some(0xF2), &[0x0F, 0x70], 0, Rm::Mem(2));
            b.push(0x1b);
            b
        }),
    ] {
        let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = compile(&format!("sse_int2_misaligned_{name}"), &src);
        let run = Command::new(&exe).output().expect("run test binary");
        assert!(!run.status.success(), "{name}: a misaligned operand must abort");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(err.contains("misaligned 16-byte access"), "{name}: stderr {err}");
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}: returned");
    }
}

/// The MMX forms and the VEX forms stay unsupported (not part of this unit).
#[test]
fn mmx_and_vex_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 6] = [
        ("paddusb mm0, mm1", &[0x0F, 0xDC, 0xC1]),
        ("psrlw mm0, mm1", &[0x0F, 0xD1, 0xC1]),
        ("psraw mm0, mm1", &[0x0F, 0xE1, 0xC1]),
        ("packssdw mm0, mm1", &[0x0F, 0x6B, 0xC1]),
        ("vpaddsb xmm0, xmm1, xmm2", &[0xC5, 0xF1, 0xEC, 0xC2]),
        ("vpsllw xmm0, xmm1, 3", &[0xC5, 0xF9, 0x71, 0xF1, 0x03]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}
