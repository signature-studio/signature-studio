//! Tests for vex.rs (decision 0032): VMOVUPS, VMOVAPS, VMOVNTPS, VZEROUPPER (VEX.128 and
//! VEX.256), LEA with an FS or GS prefix, and the GS:[0x30] / GS:[0x58] loads.
//!
//! The lifted C (clang -O0, header-only runtime fle_sse.h -> fle_vex.h) and the host CPU run
//! the same instruction on the same state. All sixteen YMM registers (both halves), seven
//! general registers and 128 bytes of memory must match bit for bit. These instructions
//! change no flags. The host instruction is real machine code run on the CPU (not
//! intrinsics); the host stub loads and stores the full 256 bits of every YMM register and
//! executes VZEROUPPER before it returns, so the SSE/AVX state is clean afterwards.
//! The host must have AVX (checked against /proc/cpuinfo; the tests fail if it does not).
//! Inputs: random upper and lower halves (so a VEX.128 zeroing shows), +-0, +-inf, quiet and
//! signalling NaN, denormals; 2-byte and 3-byte VEX encodings; registers 0-15; every aligned
//! and unaligned displacement that stays inside the buffer.
//! The GS loads have no host comparison (the Linux GS base is not the guest TEB): they are
//! checked against the model of runtime/fle_vex.h.

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

fn require_avx() {
    let t = std::fs::read_to_string("/proc/cpuinfo").expect("read /proc/cpuinfo");
    let ok = t.lines().filter(|l| l.starts_with("flags")).any(|l| l.split_whitespace().any(|f| f == "avx"));
    assert!(ok, "the host CPU has no AVX (/proc/cpuinfo): the host comparison cannot run");
}

#[derive(Clone, Debug)]
struct Case {
    name: String,
    bytes: Vec<u8>,
    /// ymm[k] = [xmm lo, xmm hi, upper lo, upper hi]
    ymm: [[u64; 4]; 16],
    /// rax, rcx, rdx, rbx, rsi, rdi, r10
    gpr: [u64; 7],
    /// bit k set: gpr[k] is an offset from the start of `mem` (a pointer into the buffer)
    rel: u32,
    mem: [u8; 128],
}

#[derive(Debug, PartialEq)]
struct Out {
    ymm: [[u64; 4]; 16],
    /// pointer registers are reported as offsets from the buffer
    gpr: [u64; 7],
    mem: [u8; 128],
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
    /// XMM or YMM register 0..15.
    X(u8),
    /// [rdi + disp8].
    Mem(i8),
}

/// VEX encoding with vvvv = 1111 (unused), W = 0. `l`: 0 = 128-bit, 1 = 256-bit. `pp`: 0
/// (no prefix). The 2-byte form is used when it can encode the operands and `force3` is off.
fn enc(opc: u8, reg: u8, rm: Rm, l: u8, force3: bool) -> Vec<u8> {
    let (rm_low, rm_hi) = match rm {
        Rm::X(n) => (n & 7, n >> 3),
        Rm::Mem(_) => (7, 0),
    };
    let r_bar = (!(reg >> 3)) & 1;
    let mut b = Vec::new();
    if rm_hi == 0 && !force3 {
        b.push(0xc5);
        b.push((r_bar << 7) | (0xf << 3) | (l << 2));
    } else {
        b.push(0xc4);
        b.push((r_bar << 7) | (1 << 6) | (((!rm_hi) & 1) << 5) | 0x01);
        b.push((0xf << 3) | (l << 2));
    }
    b.push(opc);
    match rm {
        Rm::X(_) => b.push(0xc0 | ((reg & 7) << 3) | rm_low),
        Rm::Mem(d) => {
            b.push(0x40 | ((reg & 7) << 3) | 7);
            b.push(d as u8);
        }
    }
    b
}

/// Memory is first, so that mem is 32-byte aligned and mem + 32 is too.
#[repr(C, align(32))]
struct State {
    mem: [u8; 128],
    /// rax, rcx, rdx, rbx, rsi, rdi, r10, pad
    gpr: [u64; 8],
    ymm: [[u64; 4]; 16],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_YMM: usize = std::mem::offset_of!(State, ymm);

/// Host stub: loads the state into the registers, runs `body`, saves the registers back into
/// the state, then VZEROUPPER. It keeps rbx and rbp (callee-saved).
fn stub(body: &[u8]) -> Vec<u8> {
    let disp = |o: usize| (o as i32).to_le_bytes();
    // push rbp; push rbx; mov rbp, rdi
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xfd];
    // mov r64, [rbp+disp32]: rax, rcx, rdx, rbx, rsi, rdi, r10
    let regs: [(u8, u8); 7] = [(0x48, 0x85), (0x48, 0x8d), (0x48, 0x95), (0x48, 0x9d), (0x48, 0xb5), (0x48, 0xbd), (0x4c, 0x95)];
    for (k, (rex, m)) in regs.iter().enumerate() {
        c.extend_from_slice(&[*rex, 0x8b, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    // vmovdqu ymm_k, [rbp+disp32]  (C4 R'1 1 00001 | 0 1111 1 10 | 6F modrm)
    for k in 0..16usize {
        c.extend_from_slice(&[0xc4, ((((!(k >> 3)) & 1) as u8) << 7) | 0x61, 0x7e, 0x6f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_YMM + 32 * k));
    }
    c.extend_from_slice(body);
    // vmovdqu [rbp+disp32], ymm_k
    for k in 0..16usize {
        c.extend_from_slice(&[0xc4, ((((!(k >> 3)) & 1) as u8) << 7) | 0x61, 0x7e, 0x7f, 0x80 | (((k & 7) as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_YMM + 32 * k));
    }
    // mov [rbp+disp32], r64
    for (k, (rex, m)) in regs.iter().enumerate() {
        c.extend_from_slice(&[*rex, 0x89, *m]);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    }
    // vzeroupper; pop rbx; pop rbp; ret
    c.extend_from_slice(&[0xc5, 0xf8, 0x77, 0x5b, 0x5d, 0xc3]);
    c
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

/// Runs the case on the host CPU.
fn run_host(c: &Case) -> Out {
    let mut st = Box::new(State { mem: c.mem, gpr: [0; 8], ymm: c.ymm });
    let base = st.mem.as_ptr() as u64;
    assert_eq!(base % 32, 0, "state memory must be 32-byte aligned");
    for k in 0..7 {
        st.gpr[k] = if c.rel >> k & 1 == 1 { base.wrapping_add(c.gpr[k]) } else { c.gpr[k] };
    }
    let code = stub(&c.bytes);
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(p);
        f(&mut *st);
        munmap(p, 4096);
    }
    let mut gpr = [0u64; 7];
    for k in 0..7 {
        gpr[k] = if c.rel >> k & 1 == 1 { st.gpr[k].wrapping_sub(base) } else { st.gpr[k] };
    }
    Out { ymm: st.ymm, gpr, mem: st.mem }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[128] __attribute__((aligned(32)));\n\n";

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-vex/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// Compiles `src` with clang -O0 into out-vex/test, and returns the path. The runtime is
/// header-only for these instructions (fle_sse.h includes fle_vex.h).
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

/// Lifts every instruction of `bytes` (one or more), one `lift_body` call per instruction.
fn lift_all(bytes: &[u8]) -> Result<String, String> {
    let mut d = iced_x86::Decoder::with_ip(64, bytes, 0x1000, iced_x86::DecoderOptions::NONE);
    let mut body = String::new();
    let mut p = 0usize;
    while d.can_decode() {
        let i = d.decode();
        let n = i.len();
        body += &lift_body(&bytes[p..p + n], 0x1000)?;
        body.push('\n');
        p += n;
    }
    Ok(body)
}

/// Builds one C program that runs every case, and returns its parsed stdout.
fn c_run(group: &str, cs: &[Case]) -> Vec<Out> {
    let mut src = String::from(HEADER);
    for (k, c) in cs.iter().enumerate() {
        let body = lift_all(&c.bytes).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "struct In { uint64_t ymm[16][4]; uint64_t gpr[7]; unsigned rel; uint8_t mem[128]; };\n";
    src += "static const struct In IN[] = {\n";
    for c in cs {
        let ymm: Vec<String> = c.ymm.iter().map(|p| format!("{{ {}ULL, {}ULL, {}ULL, {}ULL }}", p[0], p[1], p[2], p[3])).collect();
        let gpr: Vec<String> = c.gpr.iter().map(|g| format!("{g}ULL")).collect();
        let mem: Vec<String> = c.mem.iter().map(|b| b.to_string()).collect();
        src += &format!("{{ {{ {} }}, {{ {} }}, {}u, {{ {} }} }},\n", ymm.join(", "), gpr.join(", "), c.rel, mem.join(", "));
    }
    src += "};\n";
    src += "static void (*const FN[])(void) = {";
    for k in 0..cs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n\n";
    src += r#"int main(void)
{
    static const int RR[7] = { 0, 1, 2, 3, 6, 7, 10 };
    unsigned k, j;
    uint64_t base = (uint64_t)(uintptr_t)M;
    for (k = 0; k < NCASES; k++) {
        for (j = 0; j < 16; j++) {
            X[j].u64[0] = IN[k].ymm[j][0]; X[j].u64[1] = IN[k].ymm[j][1];
            YH[j].u64[0] = IN[k].ymm[j][2]; YH[j].u64[1] = IN[k].ymm[j][3];
        }
        for (j = 0; j < 7; j++) R[RR[j]] = IN[k].gpr[j] + (((IN[k].rel >> j) & 1) ? base : 0);
        memcpy(M, IN[k].mem, sizeof M);
        FN[k]();
        printf("%u", k);
        for (j = 0; j < 16; j++) printf(" %llx %llx %llx %llx", (unsigned long long)X[j].u64[0], (unsigned long long)X[j].u64[1], (unsigned long long)YH[j].u64[0], (unsigned long long)YH[j].u64[1]);
        for (j = 0; j < 7; j++) printf(" %llx", (unsigned long long)(R[RR[j]] - (((IN[k].rel >> j) & 1) ? base : 0)));
        for (j = 0; j < 128; j++) printf(" %02x", M[j]);
        printf("\n");
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());

    let exe = compile(&format!("vex_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8 output");

    let hex = |s: &str| u64::from_str_radix(s, 16).expect("hex");
    text.lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut p = 1;
            let mut ymm = [[0u64; 4]; 16];
            for y in ymm.iter_mut() {
                for q in y.iter_mut() {
                    *q = hex(t[p]);
                    p += 1;
                }
            }
            let mut gpr = [0u64; 7];
            for g in gpr.iter_mut() {
                *g = hex(t[p]);
                p += 1;
            }
            let mut mem = [0u8; 128];
            for m in mem.iter_mut() {
                *m = u8::from_str_radix(t[p], 16).expect("byte");
                p += 1;
            }
            Out { ymm, gpr, mem }
        })
        .collect()
}

fn check(group: &str, cs: &[Case]) {
    require_avx();
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h.ymm != case.ymm || h.mem != case.mem || h.gpr != case.gpr {
            changed += 1;
        }
        if h != *cout {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?}\n  in   {:x?}\n  C    {:x?} {:x?} {:?}\n  host {:x?} {:x?} {:?}",
                    case.name, case.bytes, case.ymm, cout.ymm, cout.gpr, cout.mem, h.ymm, h.gpr, h.mem
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed_state {changed}", cs.len());
    // the instruction really ran on the host and changed a register, memory or GPR in most cases
    assert!(changed * 10 >= cs.len() * 8, "{group}: only {changed} of {} cases changed the state", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
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

fn fp_q(rng: &mut Rng) -> u64 {
    match rng.next_u64() % 4 {
        0 => POOL64[(rng.next_u64() % POOL64.len() as u64) as usize],
        1 => rng.next_u64(),
        _ => {
            let lo = POOL32[(rng.next_u64() % POOL32.len() as u64) as usize] as u64;
            let hi = if rng.next_u64() & 1 == 0 { POOL32[(rng.next_u64() % POOL32.len() as u64) as usize] as u64 } else { rng.next_u64() & 0xffff_ffff };
            lo | (hi << 32)
        }
    }
}

fn base(name: String, bytes: Vec<u8>, rng: &mut Rng) -> Case {
    let mut ymm = [[0u64; 4]; 16];
    for y in ymm.iter_mut() {
        for q in y.iter_mut() {
            // never all-zero uppers, so that zeroing is visible
            *q = fp_q(rng) | 1 << (rng.next_u64() % 64);
        }
    }
    let mut gpr = [0u64; 7];
    for g in gpr.iter_mut() {
        *g = rng.next_u64();
    }
    let mut mem = [0u8; 128];
    for q in 0..16 {
        let v = fp_q(rng);
        mem[q * 8..q * 8 + 8].copy_from_slice(&v.to_le_bytes());
    }
    // rdi = mem + 32
    gpr[5] = 32;
    Case { name, bytes, ymm, gpr, rel: 1 << 5, mem }
}

/// VMOVUPS / VMOVAPS / VMOVNTPS groups. `load`/`store`: opcodes (None: absent). `aligned`: the
/// displacement is a multiple of the operand size (so that the aligned forms do not fault);
/// otherwise every displacement in range is used. `regcopy`: also register-to-register in
/// both encodings.
fn mov_group(name: &str, load: Option<u8>, store: u8, aligned: bool, regcopy: bool, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..n {
        for l in [0u8, 1u8] {
            let size: i32 = if l == 1 { 32 } else { 16 };
            let d = (rng.next_u64() % 16) as u8;
            let s = if k % 7 == 0 { d } else { (rng.next_u64() % 16) as u8 };
            // rdi = mem + 32: the access is mem[32 + disp ..][..size], inside 0..128
            let lo = -32i32;
            let hi = 96 - size;
            let disp = if aligned {
                let steps = ((hi - lo) / size + 1) as u64;
                lo + size * (rng.next_u64() % steps) as i32
            } else {
                lo + (rng.next_u64() % ((hi - lo + 1) as u64)) as i32
            };
            let force3 = k % 3 == 0;
            let w = if l == 1 { "ymm" } else { "xmm" };
            if let Some(opc) = load {
                let c = base(format!("{name} {w}{d}, [rdi{disp:+}] #{k}"), enc(opc, d, Rm::Mem(disp as i8), l, force3), &mut rng);
                out.push(c);
            }
            let c = base(format!("{name} [rdi{disp:+}], {w}{s} #{k}"), enc(store, s, Rm::Mem(disp as i8), l, force3), &mut rng);
            out.push(c);
            if regcopy {
                let opc = load.unwrap();
                let c = base(format!("{name} {w}{d}, {w}{s} (load form) #{k}"), enc(opc, d, Rm::X(s), l, force3), &mut rng);
                out.push(c);
                let c = base(format!("{name} {w}{d}, {w}{s} (store form) #{k}"), enc(store, s, Rm::X(d), l, force3), &mut rng);
                out.push(c);
            }
        }
    }
    out
}

#[test]
fn vmovups_matches_host() {
    check("vmovups", &mov_group("vmovups", Some(0x10), 0x11, false, true, 400, 0x8001));
}

#[test]
fn vmovups_aligned_addresses_match_host() {
    check("vmovups-aligned", &mov_group("vmovups", Some(0x10), 0x11, true, true, 200, 0x8002));
}

#[test]
fn vmovaps_matches_host() {
    check("vmovaps", &mov_group("vmovaps", Some(0x28), 0x29, true, true, 400, 0x8003));
}

#[test]
fn vmovntps_matches_host() {
    check("vmovntps", &mov_group("vmovntps", None, 0x2b, true, false, 400, 0x8004));
}

#[test]
fn vzeroupper_matches_host() {
    let mut rng = Rng(0x8005);
    let cs: Vec<Case> = (0..60).map(|k| base(format!("vzeroupper #{k}"), vec![0xc5, 0xf8, 0x77], &mut rng)).collect();
    check("vzeroupper", &cs);
}

/// A VEX.256 sequence with a VEX.128 register copy, VZEROUPPER and a ymm copy: each lifted
/// instruction runs in order in one C function, the host runs the same bytes in one stub.
#[test]
fn sequences_match_host() {
    let mut rng = Rng(0x8006);
    let mut cs = Vec::new();
    for k in 0..80 {
        let d = (rng.next_u64() % 16) as u8;
        let s = (rng.next_u64() % 16) as u8;
        let mut bytes = enc(0x10, d, Rm::Mem(0), 1, false); // vmovups ymm_d, [rdi]
        bytes.extend(enc(0x11, s, Rm::Mem(32), 1, k % 2 == 0)); // vmovups [rdi+32], ymm_s
        bytes.extend(enc(0x10, s, Rm::X(d), 0, false)); // vmovups xmm_s, xmm_d
        if k % 4 != 0 {
            bytes.extend([0xc5, 0xf8, 0x77]); // vzeroupper
        }
        bytes.extend(enc(0x10, d, Rm::X(s), 1, true)); // vmovups ymm_d, ymm_s
        cs.push(base(format!("sequence #{k}"), bytes, &mut rng));
    }
    check("sequences", &cs);
}

/// The encodings found in the closure's disassembly (exact bytes, function 0x409c80), run
/// against the host with the base registers pointing into the buffer.
#[test]
fn closure_encodings_match_host() {
    let encs: [(&str, &[u8]); 11] = [
        ("vmovups ymm1,[rcx]", &[0xc5, 0xfc, 0x10, 0x09]),
        ("vmovups ymm2,[rcx+rax*1-0x20]", &[0xc5, 0xfc, 0x10, 0x54, 0x01, 0xe0]),
        ("vmovups ymm0,[rcx+rax*1]", &[0xc5, 0xfc, 0x10, 0x04, 0x01]),
        ("vmovaps [rdx+rax*1],ymm0", &[0xc5, 0xfc, 0x29, 0x04, 0x02]),
        ("vmovups [r10],ymm1", &[0xc4, 0xc1, 0x7c, 0x11, 0x0a]),
        ("vmovups [rdx],ymm2", &[0xc5, 0xfc, 0x11, 0x12]),
        ("vzeroupper", &[0xc5, 0xf8, 0x77]),
        ("vmovntps [rdx+rax*1],ymm0", &[0xc5, 0xfc, 0x2b, 0x04, 0x02]),
        ("vmovups [rdx],ymm1", &[0xc5, 0xfc, 0x11, 0x0a]),
        ("vmovups [r10],ymm2", &[0xc4, 0xc1, 0x7c, 0x11, 0x12]),
        ("vmovaps [rdx+rax*1],ymm0 (again)", &[0xc5, 0xfc, 0x29, 0x04, 0x02]),
    ];
    let mut rng = Rng(0x8007);
    let mut cs = Vec::new();
    for (name, bytes) in encs {
        for k in 0..40 {
            let mut c = base(format!("{name} #{k}"), bytes.to_vec(), &mut rng);
            // rcx = rdx = mem + 64, rax in {-32, 0, 32}, r10 = mem + (0..=64)
            c.gpr[1] = 64;
            c.gpr[2] = 64;
            c.gpr[0] = [(-32i64) as u64, 0, 32][(rng.next_u64() % 3) as usize];
            c.gpr[6] = rng.next_u64() % 65;
            c.rel = (1 << 1) | (1 << 2) | (1 << 5) | (1 << 6);
            cs.push(c);
        }
    }
    check("closure-encodings", &cs);
}

/// LEA with an FS or GS prefix: the segment is not part of the result (SDM LEA: the offset part
/// of the address). The host runs the same bytes.
#[test]
fn lea_with_fs_gs_matches_host() {
    let forms: [(&str, &[u8]); 8] = [
        ("lea rax, fs:[rax+riz*1+0x0] (closure 0x409dda)", &[0x64, 0x48, 0x8d, 0x44, 0x20, 0x00]),
        ("lea rax, fs:[rbx+rcx*4+0x10]", &[0x64, 0x48, 0x8d, 0x44, 0x8b, 0x10]),
        ("lea eax, gs:[rbx+rcx*2-8]", &[0x65, 0x8d, 0x44, 0x4b, 0xf8]),
        ("lea r10, gs:[rdx+rsi*8+0x100]", &[0x65, 0x4c, 0x8d, 0x94, 0xf2, 0x00, 0x01, 0x00, 0x00]),
        ("lea edx, fs:[rsi+rcx*1+0x12345678]", &[0x64, 0x8d, 0x94, 0x0e, 0x78, 0x56, 0x34, 0x12]),
        ("lea rcx, gs:[rbx]", &[0x65, 0x48, 0x8d, 0x0b]),
        ("lea rsi, fs:[0x58]", &[0x64, 0x48, 0x8d, 0x34, 0x25, 0x58, 0x00, 0x00, 0x00]),
        ("lea rax, gs:[rax*8-0x10]", &[0x65, 0x48, 0x8d, 0x04, 0xc5, 0xf0, 0xff, 0xff, 0xff]),
    ];
    let mut rng = Rng(0x8008);
    let mut cs = Vec::new();
    for (name, bytes) in forms {
        for k in 0..60 {
            let mut c = base(format!("{name} #{k}"), bytes.to_vec(), &mut rng);
            c.rel = 0;
            c.gpr[5] = rng.next_u64();
            cs.push(c);
        }
    }
    check("lea-fsgs", &cs);
    // the generated C does not mention the segment
    let b = lift_body(&[0x64, 0x48, 0x8d, 0x44, 0x20, 0x00], 0x1000).unwrap();
    assert!(!b.contains("gs") && !b.contains("fle_teb"), "{b}");
}

/// `mov rax, gs:[0x58]` is the TLS array pointer of the model and `mov rax, gs:[0x30]` the TEB
/// self pointer. No host comparison (the Linux GS base is not the guest TEB).
#[test]
fn gs_loads_follow_the_model() {
    let b58 = lift_body(&[0x65, 0x48, 0x8b, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00], 0x1000).expect("gs:[0x58]");
    let b30 = lift_body(&[0x65, 0x4c, 0x8b, 0x14, 0x25, 0x30, 0x00, 0x00, 0x00], 0x1000).expect("gs:[0x30] into r10");
    assert!(b58.contains("fle_gs_ld8(UINT64_C(0x58))") && b30.contains("fle_gs_ld8(UINT64_C(0x30))"), "{b58} / {b30}");
    let src = format!(
        "{HEADER}uint64_t fle_tls_values[8];\nstatic void c58(void)\n{{\n{b58}\n}}\nstatic void c30(void)\n{{\n{b30}\n}}\nint main(void)\n{{\n    uint64_t tls, teb_self;\n    R[0] = 1; R[10] = 2;\n    c58();\n    c30();\n    tls = R[0]; teb_self = R[10];\n    if (tls != (uint64_t)(uintptr_t)fle_tls_values) {{ printf(\"tls %llx\\n\", (unsigned long long)tls); return 1; }}\n    if (teb_self != (uint64_t)(uintptr_t)fle_teb) {{ printf(\"self %llx\\n\", (unsigned long long)teb_self); return 2; }}\n    /* the TEB buffer holds both values, so a load through Self sees them */\n    if (*(uint64_t *)(uintptr_t)(teb_self + 0x30) != teb_self) return 3;\n    if (*(uint64_t *)(uintptr_t)(teb_self + 0x58) != tls) return 4;\n    /* the slot table is the array the guest indexes: slot 3 */\n    fle_tls_values[3] = UINT64_C(0xabcdef);\n    if (((uint64_t *)(uintptr_t)tls)[3] != UINT64_C(0xabcdef)) return 5;\n    printf(\"ok\\n\");\n    return 0;\n}}\n"
    );
    let exe = compile("vex_gs_model", &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "gs model check failed: code {:?} {}", run.status.code(), String::from_utf8_lossy(&run.stdout));
}

/// Anything else through FS or GS stays unsupported (fail-closed): other offsets, 32-bit loads,
/// stores, base or index registers, FS memory operands, ALU forms.
#[test]
fn other_fs_gs_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 9] = [
        ("mov rax, gs:[0x60]", &[0x65, 0x48, 0x8b, 0x04, 0x25, 0x60, 0x00, 0x00, 0x00]),
        ("mov eax, gs:[0x58]", &[0x65, 0x8b, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00]),
        ("mov rax, fs:[0x58]", &[0x64, 0x48, 0x8b, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00]),
        ("mov rax, gs:[rcx]", &[0x65, 0x48, 0x8b, 0x01]),
        ("mov gs:[0x58], rax", &[0x65, 0x48, 0x89, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00]),
        ("add rax, gs:[0x58]", &[0x65, 0x48, 0x03, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00]),
        ("mov rax, gs:[rax*8+0x58]", &[0x65, 0x48, 0x8b, 0x04, 0xc5, 0x58, 0x00, 0x00, 0x00]),
        ("vmovups xmm0, fs:[rax]", &[0x64, 0xc5, 0xf8, 0x10, 0x00]),
        ("vmovaps ymm0, gs:[rax]", &[0x65, 0xc5, 0xfc, 0x28, 0x00]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}

/// Other VEX and all EVEX forms stay unsupported.
#[test]
fn other_vex_and_evex_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 6] = [
        ("vaddps ymm0, ymm0, ymm1", &[0xc5, 0xfc, 0x58, 0xc1]),
        ("vmovdqu ymm0, [rax]", &[0xc5, 0xfe, 0x6f, 0x00]),
        ("vzeroall", &[0xc5, 0xfc, 0x77]),
        ("vmovups zmm0, [rax] (EVEX)", &[0x62, 0xf1, 0x7c, 0x48, 0x10, 0x00]),
        ("vmovaps xmm0{k1}, [rax] (EVEX)", &[0x62, 0xf1, 0x7c, 0x09, 0x28, 0x00]),
        ("vshufps xmm0, xmm1, xmm2, 0x1b", &[0xc5, 0xf0, 0xc6, 0xc2, 0x1b]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
}

/// VMOVAPS and VMOVNTPS abort on a memory operand that is not 16-byte (xmm) or 32-byte (ymm)
/// aligned (#GP on hardware; only the C side is run). VMOVUPS never aborts.
#[test]
fn misaligned_operands() {
    let aborting: [(&str, Vec<u8>, &str); 6] = [
        ("vmovaps ymm0, [rdi+16]", enc(0x28, 0, Rm::Mem(16), 1, false), "misaligned 32-byte access"),
        ("vmovaps [rdi+8], ymm1", enc(0x29, 1, Rm::Mem(8), 1, false), "misaligned 32-byte access"),
        ("vmovaps xmm0, [rdi+8]", enc(0x28, 0, Rm::Mem(8), 0, false), "misaligned 16-byte access"),
        ("vmovaps [rdi+1], xmm2", enc(0x29, 2, Rm::Mem(1), 0, false), "misaligned 16-byte access"),
        ("vmovntps [rdi+16], ymm0", enc(0x2b, 0, Rm::Mem(16), 1, false), "misaligned 32-byte access"),
        ("vmovntps [rdi+4], xmm0", enc(0x2b, 0, Rm::Mem(4), 0, false), "misaligned 16-byte access"),
    ];
    for (k, (name, bytes, msg)) in aborting.iter().enumerate() {
        let body = lift_body(bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 32);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = compile(&format!("vex_misaligned_{k}"), &src);
        let run = Command::new(&exe).output().expect("run test binary");
        assert!(!run.status.success(), "{name}: a misaligned operand must abort");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(err.contains(msg), "{name}: stderr {err}");
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}: returned");
    }
    // 16-byte aligned but not 32-byte aligned is fine for the xmm form
    let body = lift_body(&enc(0x28, 0, Rm::Mem(16), 0, false), 0x1000).unwrap();
    let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 32);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
    let exe = compile("vex_aligned16_xmm", &src);
    assert!(Command::new(&exe).output().unwrap().status.success());
    // VMOVUPS accepts any address
    for (k, bytes) in [enc(0x10, 0, Rm::Mem(3), 1, false), enc(0x11, 0, Rm::Mem(5), 1, false), enc(0x10, 0, Rm::Mem(1), 0, false)].iter().enumerate() {
        let body = lift_body(bytes, 0x1000).unwrap();
        assert!(body.contains(", 0);"), "{body}");
        let src = format!("{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 32);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n");
        let exe = compile(&format!("vex_unaligned_ok_{k}"), &src);
        assert!(Command::new(&exe).output().unwrap().status.success(), "vmovups unaligned #{k} aborted");
    }
}

/// The case sets reach the edges the host comparison is meant to cover.
#[test]
fn case_sets_cover_the_edges() {
    for e in [0u32, 0x8000_0000, 0x7fc0_0000, 0x7f80_0001, 0x7f80_0000, 0xff80_0000] {
        assert!(POOL32.contains(&e), "POOL32 lacks {e:#x}");
    }
    for e in [0u64, 0x8000_0000_0000_0000, 0x7ff8_0000_0000_0000, 0x7ff0_0000_0000_0001, 0x7ff0_0000_0000_0000] {
        assert!(POOL64.contains(&e), "POOL64 lacks {e:#x}");
    }
    let g = mov_group("vmovups", Some(0x10), 0x11, false, true, 400, 1);
    assert!(g.iter().any(|c| c.bytes[0] == 0xc4) && g.iter().any(|c| c.bytes[0] == 0xc5), "both VEX prefix forms");
    // 128-bit and 256-bit forms, and registers 8-15 as destination and source
    assert!(g.iter().any(|c| c.name.contains("xmm")) && g.iter().any(|c| c.name.contains("ymm")));
    assert!(g.iter().any(|c| c.name.contains("ymm1") && !c.name.contains("ymm1,") || c.name.contains("ymm12")));
    // unaligned and aligned displacements both occur
    assert!(g.iter().any(|c| c.bytes.last().map_or(false, |&d| (d as i8) % 16 != 0)));
}
