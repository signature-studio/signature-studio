//! Tests for mmx.rs (decision 0028): MMX instructions on the mm registers, plus the x87
//! status and tag side effects (SDM Vol. 3A 15.2).
//!
//! The lifted C (clang -O0, header-only fle_mmx.h; fle_x87ctl.c for the x87 state) and the host
//! CPU run the same instruction on the same state. Compared bit for bit: mm0-mm7, rax, rcx,
//! rdx, rbx, rsi, r8, r9, r10, 64 bytes of memory, the x87 status word after the instruction,
//! which tags are empty after the instruction, and the same two after a following EMMS.
//! The host runs real machine code. Before the instruction the host x87 TOP is moved with
//! FDECSTP (0-7 times) and the tags are made empty with EMMS in half of the cases, so that
//! the TOP reset and the tag reset are both observable. The model starts from the status word
//! the host reports just before the instruction. The host stub always ends with EMMS.
//! Memory operands are 8 (or 4) bytes at unaligned offsets: no alignment rule for MMX.

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

const GP: [u8; 8] = [0, 1, 2, 3, 6, 8, 9, 10];

#[derive(Clone, Debug)]
struct Case {
    name: String,
    bytes: Vec<u8>,
    mm: [u64; 8],
    /// rax, rcx, rdx, rbx, rsi, r8, r9, r10
    gpr: [u64; 8],
    /// The memory operand base is rdi = mem + 16, so disp 0 is mem[16..].
    mem: [u8; 64],
    /// FDECSTP count before the instruction.
    top_dec: u8,
    /// EMMS before the instruction (all tags empty).
    pre_empty: bool,
}

#[derive(Debug, PartialEq)]
struct Out {
    mm: [u64; 8],
    gpr: [u64; 8],
    mem: [u8; 64],
    /// x87 status word after the instruction, and after the EMMS that follows.
    sw: [u16; 2],
    /// Bit k is set when the tag of physical register k is empty (11B), after the instruction
    /// and after the EMMS that follows.
    empty: [u8; 2],
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
    /// Register direct: an mm register or the low three bits of a GPR number.
    R(u8),
    /// [rdi + disp8].
    Mem(i8),
}

const REX_W: u8 = 8;
const REX_B: u8 = 1;

/// [REX] 0F opc modrm [disp8]. `rex` holds the REX bits W, R, B (0 when no REX byte).
fn enc(rex: u8, opc: &[u8], reg: u8, rm: Rm) -> Vec<u8> {
    let mut b = Vec::new();
    if rex != 0 {
        b.push(0x40 | rex);
    }
    b.extend_from_slice(opc);
    match rm {
        Rm::R(n) => b.push(0xc0 | ((reg & 7) << 3) | (n & 7)),
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
    /// rax, rcx, rdx, rbx, rsi, rdi, r8, r9, r10
    gpr: [u64; 9],
    mm: [u64; 8],
    /// status word before the instruction, after it, after the EMMS
    sw: [u64; 3],
    /// FNSTENV images after the instruction and after the EMMS
    env: [[u8; 32]; 2],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_MM: usize = std::mem::offset_of!(State, mm);
const OFF_SW: usize = std::mem::offset_of!(State, sw);
const OFF_ENV: usize = std::mem::offset_of!(State, env);

/// Register numbers of State.gpr, in order.
const HOST_REGS: [u8; 9] = [0, 1, 2, 3, 6, 7, 8, 9, 10];

/// Host stub: loads GPRs and mm registers, adjusts the x87 state, runs `body`, saves the state.
/// It keeps rbx and rbp (callee-saved) and ends with EMMS.
fn stub(body: &[u8], top_dec: u8, pre_empty: bool) -> Vec<u8> {
    let disp = |o: usize| (o as i32).to_le_bytes();
    // push rbp; push rbx; mov rbp, rdi
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xfd];
    let gpr_op = |c: &mut Vec<u8>, opc: u8, k: usize| {
        let n = HOST_REGS[k];
        c.push(if n >= 8 { 0x4c } else { 0x48 });
        c.push(opc);
        c.push(0x80 | ((n & 7) << 3) | 5);
        c.extend_from_slice(&disp(OFF_GPR + 8 * k));
    };
    for k in 0..9 {
        gpr_op(&mut c, 0x8b, k);
    }
    // movq mm_k, [rbp+disp32]
    for k in 0..8usize {
        c.extend_from_slice(&[0x0f, 0x6f, 0x80 | ((k as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_MM + 8 * k));
    }
    if pre_empty {
        c.extend_from_slice(&[0x0f, 0x77]);
    }
    for _ in 0..top_dec {
        c.extend_from_slice(&[0xd9, 0xf6]);
    }
    // fnstsw [rbp+disp32] (before)
    c.extend_from_slice(&[0xdd, 0xbd]);
    c.extend_from_slice(&disp(OFF_SW));
    c.extend_from_slice(body);
    // fnstsw, fnstenv (after the instruction)
    c.extend_from_slice(&[0xdd, 0xbd]);
    c.extend_from_slice(&disp(OFF_SW + 8));
    c.extend_from_slice(&[0xd9, 0xb5]);
    c.extend_from_slice(&disp(OFF_ENV));
    // movq [rbp+disp32], mm_k
    for k in 0..8usize {
        c.extend_from_slice(&[0x0f, 0x7f, 0x80 | ((k as u8) << 3) | 5]);
        c.extend_from_slice(&disp(OFF_MM + 8 * k));
    }
    // emms; fnstsw, fnstenv (after the EMMS)
    c.extend_from_slice(&[0x0f, 0x77]);
    c.extend_from_slice(&[0xdd, 0xbd]);
    c.extend_from_slice(&disp(OFF_SW + 16));
    c.extend_from_slice(&[0xd9, 0xb5]);
    c.extend_from_slice(&disp(OFF_ENV + 32));
    for k in 0..9 {
        gpr_op(&mut c, 0x89, k);
    }
    // pop rbx; pop rbp; ret
    c.extend_from_slice(&[0x5b, 0x5d, 0xc3]);
    c
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

fn empty_mask(env: &[u8; 32]) -> u8 {
    let tw = u16::from_le_bytes([env[8], env[9]]);
    (0..8).fold(0u8, |m, k| if (tw >> (2 * k)) & 3 == 3 { m | (1 << k) } else { m })
}

struct Host {
    out: Out,
    sw_before: u16,
}

/// Runs the case on the host CPU.
fn run_host(c: &Case) -> Host {
    let mut st = Box::new(State { mem: c.mem, gpr: [0; 9], mm: c.mm, sw: [0; 3], env: [[0; 32]; 2] });
    st.gpr[..5].copy_from_slice(&c.gpr[..5]);
    st.gpr[5] = st.mem.as_ptr() as u64 + 16;
    st.gpr[6..].copy_from_slice(&c.gpr[5..]);
    let code = stub(&c.bytes, c.top_dec, c.pre_empty);
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(p);
        f(&mut *st);
        munmap(p, 4096);
    }
    let mut gpr = [0u64; 8];
    gpr[..5].copy_from_slice(&st.gpr[..5]);
    gpr[5..].copy_from_slice(&st.gpr[6..]);
    Host {
        out: Out {
            mm: st.mm,
            gpr,
            mem: st.mem,
            sw: [st.sw[1] as u16, st.sw[2] as u16],
            empty: [empty_mask(&st.env[0]), empty_mask(&st.env[1])],
        },
        sw_before: st.sw[0] as u16,
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_x87ctl.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-mmx/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// Compiles `src` with clang -O0 into out-mmx/test, and returns the path.
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
        .arg(rt.join("fle_x87ctl.c"))
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

/// Builds one C program that runs every case. `sw0[k]` is the status word the host reported
/// just before the instruction of case k.
fn c_run(group: &str, cs: &[Case], sw0: &[u16]) -> Vec<Out> {
    let mut src = String::from(HEADER);
    for (k, c) in cs.iter().enumerate() {
        let body = lift_body(&c.bytes, 0x1000).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "struct In { uint64_t mm[8]; uint64_t gpr[8]; uint8_t mem[64]; unsigned sw0; unsigned tw0; };\n";
    src += "static const struct In IN[] = {\n";
    for (c, sw) in cs.iter().zip(sw0) {
        let mm: Vec<String> = c.mm.iter().map(|p| format!("{p}ULL")).collect();
        let gpr: Vec<String> = c.gpr.iter().map(|g| format!("{g}ULL")).collect();
        let mem: Vec<String> = c.mem.iter().map(|b| b.to_string()).collect();
        let tw0 = if c.pre_empty { 0xffffu32 } else { 0 };
        src += &format!("{{ {{ {} }}, {{ {} }}, {{ {} }}, {sw}, {tw0} }},\n", mm.join(", "), gpr.join(", "), mem.join(", "));
    }
    src += "};\n";
    src += "static void (*const FN[])(void) = {";
    for k in 0..cs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n\n";
    src += r#"static unsigned emask(void)
{
    if (fle_x87.tw == 0xffff) return 0xff;
    if (fle_x87.tw == 0) return 0;
    return 0x55;
}

int main(void)
{
    static const int GP[8] = { 0, 1, 2, 3, 6, 8, 9, 10 };
    unsigned k, j;
    for (k = 0; k < NCASES; k++) {
        for (j = 0; j < 8; j++) { MM[j] = IN[k].mm[j]; R[GP[j]] = IN[k].gpr[j]; }
        R[7] = (uint64_t)(uintptr_t)(M + 16);
        memcpy(M, IN[k].mem, sizeof M);
        fle_x87.sw = (uint16_t)IN[k].sw0;
        fle_x87.tw = (uint16_t)IN[k].tw0;
        FN[k]();
        printf("%u", k);
        for (j = 0; j < 8; j++) printf(" %llx", (unsigned long long)MM[j]);
        for (j = 0; j < 8; j++) printf(" %llx", (unsigned long long)R[GP[j]]);
        for (j = 0; j < 64; j++) printf(" %02x", M[j]);
        printf(" %x %x", (unsigned)fle_x87.sw, emask());
        fle_mmx_emms();
        printf(" %x %x\n", (unsigned)fle_x87.sw, emask());
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());

    let exe = compile(&format!("mmx_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8 output");

    let hex = |s: &str| u64::from_str_radix(s, 16).expect("hex");
    text.lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut p = 1;
            let mut mm = [0u64; 8];
            for x in mm.iter_mut() {
                *x = hex(t[p]);
                p += 1;
            }
            let mut gpr = [0u64; 8];
            for g in gpr.iter_mut() {
                *g = hex(t[p]);
                p += 1;
            }
            let mut mem = [0u8; 64];
            for m in mem.iter_mut() {
                *m = u8::from_str_radix(t[p], 16).expect("byte");
                p += 1;
            }
            let sw = [hex(t[p]) as u16, hex(t[p + 2]) as u16];
            let empty = [hex(t[p + 1]) as u8, hex(t[p + 3]) as u8];
            Out { mm, gpr, mem, sw, empty }
        })
        .collect()
}

fn check(group: &str, cs: &[Case]) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let hosts: Vec<Host> = cs.iter().map(run_host).collect();
    let sw0: Vec<u16> = hosts.iter().map(|h| h.sw_before).collect();
    let c = c_run(group, cs, &sw0);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for ((case, cout), h) in cs.iter().zip(c.iter()).zip(hosts.iter()) {
        // the test harness itself: the host TOP before the instruction is what FDECSTP made
        let top_before = ((h.sw_before >> 11) & 7) as u8;
        assert_eq!(top_before, (8 - case.top_dec) & 7, "{group}: {}: host TOP before the instruction", case.name);
        let pre_empty = if case.pre_empty { 0xff } else { 0 };
        if h.out.mm != case.mm
            || h.out.gpr != case.gpr
            || h.out.mem != case.mem
            || h.out.sw[0] != h.sw_before
            || h.out.empty[0] != pre_empty
        {
            changed += 1;
        }
        if h.out != *cout {
            fails += 1;
            if fails <= 10 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?} top_dec {} pre_empty {} sw_before {:04x}\n  in   mm {:x?} gpr {:x?}\n  C    {:x?}\n  host {:x?}",
                    case.name, case.bytes, case.top_dec, case.pre_empty, h.sw_before, case.mm, case.gpr, cout, h.out
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed_state {changed}", cs.len());
    // the instruction really ran on the host and changed something in most cases
    assert!(changed * 10 >= cs.len() * 8, "{group}: only {changed} of {} cases changed the state", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
}

fn put64(m: &mut [u8; 64], off: usize, v: u64) {
    m[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

fn base(name: String, bytes: Vec<u8>, rng: &mut Rng) -> Case {
    let mut mm = [0u64; 8];
    for x in mm.iter_mut() {
        *x = rng.next_u64();
    }
    let mut gpr = [0u64; 8];
    for g in gpr.iter_mut() {
        *g = rng.next_u64();
    }
    let mut mem = [0u8; 64];
    for b in mem.iter_mut() {
        *b = rng.next_u64() as u8;
    }
    let top_dec = (rng.next_u64() % 8) as u8;
    let pre_empty = rng.next_u64() & 1 == 1;
    Case { name, bytes, mm, gpr, mem, top_dec, pre_empty }
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

/// Sets lane `lane` (of width `bits`) of a 64-bit value. `lane` counts from the low end.
fn fill_lane(x: &mut u64, lane: usize, v: u64, bits: u32) {
    if bits == 64 {
        *x = v;
        return;
    }
    let sh = lane as u32 * bits;
    let mask = ((1u64 << bits) - 1) << sh;
    *x = (*x & !mask) | ((v << sh) & mask);
}

/// Fills every lane of a 64-bit value: half the lanes from the edge pool, half random.
fn fill_all(x: &mut u64, bits: u32, rng: &mut Rng) {
    let p = pool(bits);
    for lane in 0..(64 / bits as usize) {
        let v = if rng.next_u64() & 1 == 0 { p[(rng.next_u64() % p.len() as u64) as usize] } else { rng.next_u64() };
        fill_lane(x, lane, v, bits);
    }
}

const DISPS: [i8; 10] = [-16, -9, -1, 0, 1, 3, 7, 16, 25, 40];

fn disp(rng: &mut Rng) -> i8 {
    DISPS[(rng.next_u64() % DISPS.len() as u64) as usize]
}

/// `op mm_d, mm_s` and `op mm_d, [rdi + disp]`. `n` rounds; each round gives one register case
/// and one memory case (the displacement is mostly unaligned).
fn bin_cases(name: &str, opc: &[u8], bits: u32, n: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..n {
        let d = (rng.next_u64() % 8) as u8;
        let s = if k % 7 == 0 { d } else { (rng.next_u64() % 8) as u8 };
        let mut c = base(format!("{name} mm{d}, mm{s} #{k}"), enc(0, opc, d, Rm::R(s)), &mut rng);
        fill_all(&mut c.mm[d as usize], bits, &mut rng);
        if s != d {
            fill_all(&mut c.mm[s as usize], bits, &mut rng);
        }
        out.push(c);
        let ds = disp(&mut rng);
        let mut m = base(format!("{name} mm{d}, [rdi{ds:+}] #{k}"), enc(0, opc, d, Rm::Mem(ds)), &mut rng);
        fill_all(&mut m.mm[d as usize], bits, &mut rng);
        let mut v = 0u64;
        fill_all(&mut v, bits, &mut rng);
        put64(&mut m.mem, (ds as i32 + 16) as usize, v);
        out.push(m);
    }
    out
}

/// Pairs that fix both operands lane by lane (all combinations of edge values for the lane
/// width), so that every saturation boundary is hit for sure.
fn edge_cases(name: &str, opc: &[u8], bits: u32) -> Vec<Case> {
    let p = pool(bits);
    let lanes = 64 / bits as usize;
    let mut rng = Rng(0xabcd_ef01 ^ opc[opc.len() - 1] as u64);
    let mut out = Vec::new();
    for k in 0..(p.len() * p.len()) {
        let (a, b) = (k % p.len(), k / p.len());
        let mut c = base(format!("{name} edge #{k}"), enc(0, opc, 0, Rm::R(1)), &mut rng);
        for lane in 0..lanes {
            fill_lane(&mut c.mm[0], lane, p[(a + lane) % p.len()], bits);
            fill_lane(&mut c.mm[1], lane, p[(b + lane * 3) % p.len()], bits);
        }
        out.push(c);
    }
    out
}

fn bin_group(name: &str, opc: u8, bits: u32, seed: u64) -> Vec<Case> {
    let mut v = bin_cases(name, &[0x0F, opc], bits, 300, seed);
    v.extend(edge_cases(name, &[0x0F, opc], bits));
    v
}

/// Shift counts: below, at and above the lane width, with bits set in the high half of the
/// 64-bit count, and random values. The whole 64-bit count operand counts.
fn counts(rng: &mut Rng) -> Vec<u64> {
    let mut v: Vec<u64> = vec![
        0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 255, 256, 0xffff, 0x1_0000, 0x7fff_ffff,
        0x8000_0000, 0xffff_ffff, 0x1_0000_0000, 0x1_0000_0001, 0x8000_0000_0000_0000, u64::MAX, 0xffff_ffff_0000_0003,
    ];
    for _ in 0..12 {
        v.push(rng.next_u64() % 40);
    }
    for _ in 0..6 {
        v.push(rng.next_u64());
    }
    v
}

/// PSLLW by an mm count, by an m64 count, and by imm8 (all 256 values).
fn psllw_group(seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    let cs = counts(&mut rng);
    for (k, &cnt) in cs.iter().enumerate() {
        for rep in 0..3 {
            let d = (rng.next_u64() % 8) as u8;
            let mut s = (rng.next_u64() % 8) as u8;
            if s == d {
                s = (s + 1) % 8;
            }
            let mut c = base(format!("psllw mm{d}, mm{s} cnt {cnt:#x} #{k}.{rep}"), enc(0, &[0x0F, 0xF1], d, Rm::R(s)), &mut rng);
            fill_all(&mut c.mm[d as usize], 16, &mut rng);
            c.mm[s as usize] = cnt;
            out.push(c);
            let ds = disp(&mut rng);
            let mut m = base(format!("psllw mm{d}, [rdi{ds:+}] cnt {cnt:#x} #{k}.{rep}"), enc(0, &[0x0F, 0xF1], d, Rm::Mem(ds)), &mut rng);
            fill_all(&mut m.mm[d as usize], 16, &mut rng);
            put64(&mut m.mem, (ds as i32 + 16) as usize, cnt);
            out.push(m);
        }
    }
    for imm in 0..=255u8 {
        let d = (rng.next_u64() % 8) as u8;
        let mut bytes = enc(0, &[0x0F, 0x71], 6, Rm::R(d));
        bytes.push(imm);
        let mut c = base(format!("psllw mm{d}, {imm}"), bytes, &mut rng);
        fill_all(&mut c.mm[d as usize], 16, &mut rng);
        out.push(c);
    }
    out
}

/// PSHUFW: every imm8, register and memory source.
fn pshufw_group(seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for imm in 0..=255u8 {
        let d = (rng.next_u64() % 8) as u8;
        let s = (rng.next_u64() % 8) as u8;
        let mut bytes = enc(0, &[0x0F, 0x70], d, Rm::R(s));
        bytes.push(imm);
        let mut c = base(format!("pshufw mm{d}, mm{s}, {imm:#x}"), bytes, &mut rng);
        // distinct words so that a wrong lane shows up
        c.mm[s as usize] = rng.next_u64() | 0x0001_0001_0001_0001;
        out.push(c);
        let ds = disp(&mut rng);
        let mut bytes = enc(0, &[0x0F, 0x70], d, Rm::Mem(ds));
        bytes.push(imm);
        let mut m = base(format!("pshufw mm{d}, [rdi{ds:+}], {imm:#x}"), bytes, &mut rng);
        put64(&mut m.mem, (ds as i32 + 16) as usize, rng.next_u64());
        out.push(m);
    }
    out
}

fn gp(rng: &mut Rng) -> u8 {
    GP[(rng.next_u64() % GP.len() as u64) as usize]
}

fn edge64(rng: &mut Rng) -> u64 {
    let p = pool(64);
    if rng.next_u64() & 1 == 0 {
        p[(rng.next_u64() % p.len() as u64) as usize]
    } else {
        rng.next_u64()
    }
}

/// MOVD: mm <- r32, r32 <- mm, mm <- m32, m32 <- mm.
fn movd_group(seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..200 {
        let d = (rng.next_u64() % 8) as u8;
        let r = gp(&mut rng);
        let rexb = if r >= 8 { REX_B } else { 0 };
        let mut c = base(format!("movd mm{d}, r{r}d #{k}"), enc(rexb, &[0x0F, 0x6E], d, Rm::R(r)), &mut rng);
        c.gpr[GP.iter().position(|&g| g == r).unwrap()] = edge64(&mut rng);
        out.push(c);
        let mut c = base(format!("movd r{r}d, mm{d} #{k}"), enc(rexb, &[0x0F, 0x7E], d, Rm::R(r)), &mut rng);
        c.mm[d as usize] = edge64(&mut rng);
        out.push(c);
        let ds = disp(&mut rng);
        let mut m = base(format!("movd mm{d}, [rdi{ds:+}] #{k}"), enc(0, &[0x0F, 0x6E], d, Rm::Mem(ds)), &mut rng);
        put64(&mut m.mem, (ds as i32 + 16) as usize, edge64(&mut rng));
        out.push(m);
        let mut m = base(format!("movd [rdi{ds:+}], mm{d} #{k}"), enc(0, &[0x0F, 0x7E], d, Rm::Mem(ds)), &mut rng);
        m.mm[d as usize] = edge64(&mut rng);
        out.push(m);
    }
    out
}

/// MOVQ: mm <- r64, r64 <- mm, mm <- mm/m64, mm/m64 <- mm.
fn movq_group(seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for k in 0..200 {
        let d = (rng.next_u64() % 8) as u8;
        let s = (rng.next_u64() % 8) as u8;
        let r = gp(&mut rng);
        let rexb = REX_W | if r >= 8 { REX_B } else { 0 };
        let mut c = base(format!("movq mm{d}, r{r} #{k}"), enc(rexb, &[0x0F, 0x6E], d, Rm::R(r)), &mut rng);
        c.gpr[GP.iter().position(|&g| g == r).unwrap()] = edge64(&mut rng);
        out.push(c);
        let mut c = base(format!("movq r{r}, mm{d} #{k}"), enc(rexb, &[0x0F, 0x7E], d, Rm::R(r)), &mut rng);
        c.mm[d as usize] = edge64(&mut rng);
        out.push(c);
        out.push(base(format!("movq mm{d}, mm{s} #{k} (0F 6F)"), enc(0, &[0x0F, 0x6F], d, Rm::R(s)), &mut rng));
        out.push(base(format!("movq mm{s}, mm{d} #{k} (0F 7F)"), enc(0, &[0x0F, 0x7F], d, Rm::R(s)), &mut rng));
        let ds = disp(&mut rng);
        let mut m = base(format!("movq mm{d}, [rdi{ds:+}] #{k}"), enc(0, &[0x0F, 0x6F], d, Rm::Mem(ds)), &mut rng);
        put64(&mut m.mem, (ds as i32 + 16) as usize, edge64(&mut rng));
        out.push(m);
        let mut m = base(format!("movq [rdi{ds:+}], mm{d} #{k}"), enc(0, &[0x0F, 0x7F], d, Rm::Mem(ds)), &mut rng);
        m.mm[d as usize] = edge64(&mut rng);
        out.push(m);
    }
    out
}

/// EMMS from every TOP (0-7 FDECSTP steps) and from empty and valid tags.
fn emms_group() -> Vec<Case> {
    let mut rng = Rng(0x7007);
    let mut out = Vec::new();
    for top_dec in 0..8u8 {
        for pre_empty in [false, true] {
            let mut c = base(format!("emms top_dec {top_dec} pre_empty {pre_empty}"), vec![0x0f, 0x77], &mut rng);
            c.top_dec = top_dec;
            c.pre_empty = pre_empty;
            out.push(c);
        }
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

bin_test!(pxor_matches_host, "pxor", 0xEF, 64, 0x1001);
bin_test!(paddw_matches_host, "paddw", 0xFD, 16, 0x1002);
bin_test!(psubsw_matches_host, "psubsw", 0xE9, 16, 0x1003);
bin_test!(pmulhw_matches_host, "pmulhw", 0xE5, 16, 0x1004);
bin_test!(pmulhuw_matches_host, "pmulhuw", 0xE4, 16, 0x1005);
bin_test!(packuswb_matches_host, "packuswb", 0x67, 16, 0x1006);
bin_test!(punpcklbw_matches_host, "punpcklbw", 0x60, 8, 0x1007);

#[test]
fn psllw_matches_host() {
    check("psllw", &psllw_group(0x2001));
}

#[test]
fn pshufw_matches_host() {
    check("pshufw", &pshufw_group(0x3001));
}

#[test]
fn movd_matches_host() {
    check("movd", &movd_group(0x4001));
}

#[test]
fn movq_matches_host() {
    check("movq", &movq_group(0x4002));
}

#[test]
fn emms_matches_host() {
    check("emms", &emms_group());
}

#[test]
fn case_sets_cover_the_edges() {
    let cs = psllw_group(1);
    let big = cs.iter().filter(|c| c.name.contains("cnt") && !c.name.contains("[rdi")).count();
    assert!(big >= 100, "register-count cases: {big}");
    assert!(cs.iter().any(|c| c.name.contains("cnt 0x100000000")));
    assert!(cs.iter().any(|c| c.name.contains("cnt 0xffffffffffffffff")));
    for bits in [8u32, 16, 64] {
        let p = pool(bits);
        let m = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
        let top = 1u64 << (bits - 1);
        for e in [0, top - 1, top, m] {
            assert!(p.contains(&e), "pool({bits}) lacks {e:#x}");
        }
    }
    assert_eq!(pshufw_group(1).len(), 512);
    // the pre-states cover every TOP and both tag states
    let em = emms_group();
    assert_eq!(em.len(), 16);
    let mut tops: Vec<u8> = em.iter().map(|c| c.top_dec).collect();
    tops.dedup();
    assert_eq!(tops, (0..8).collect::<Vec<u8>>());
    assert!(em.iter().any(|c| c.pre_empty) && em.iter().any(|c| !c.pre_empty));
    // memory displacements include unaligned ones
    let bc = bin_cases("paddw", &[0x0F, 0xFD], 16, 100, 5);
    assert!(bc.iter().any(|c| c.name.contains("[rdi+3]") || c.name.contains("[rdi+7]") || c.name.contains("[rdi-1]")));
}

/// The MMX instructions that are not part of this unit stay unsupported, and the xmm forms of
/// the same mnemonics are not taken by mmx.rs.
#[test]
fn other_mmx_forms_stay_unsupported() {
    let cases: [(&str, &[u8]); 9] = [
        ("pand mm0, mm1", &[0x0F, 0xDB, 0xC1]),
        ("paddb mm0, mm1", &[0x0F, 0xFC, 0xC1]),
        ("pcmpeqb mm0, mm1", &[0x0F, 0x74, 0xC1]),
        ("psubd mm0, mm1", &[0x0F, 0xFA, 0xC1]),
        ("punpckldq mm0, mm1", &[0x0F, 0x62, 0xC1]),
        ("pminub mm0, mm1", &[0x0F, 0xDA, 0xC1]),
        ("pavgw mm0, mm1", &[0x0F, 0xE3, 0xC1]),
        ("cvtps2pi mm0, xmm1", &[0x0F, 0x2D, 0xC1]),
        ("movntq [rdi], mm0", &[0x0F, 0xE7, 0x07]),
    ];
    for (name, bytes) in cases {
        assert!(lift_body(bytes, 0x1000).is_err(), "{name} must stay unsupported");
    }
    // 32-bit addressing, FS segment: refused, as for the SSE modules
    assert!(lift_body(&[0x67, 0x0F, 0xFD, 0x07], 0x1000).is_err(), "paddw mm0, [edi]");
    assert!(lift_body(&[0x64, 0x0F, 0xFD, 0x07], 0x1000).is_err(), "paddw mm0, fs:[rdi]");
    for (name, bytes) in [("paddw xmm0, xmm1", &[0x66u8, 0x0F, 0xFD, 0xC1][..]), ("pxor xmm0, xmm1", &[0x66, 0x0F, 0xEF, 0xC1])] {
        let body = lift_body(bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!body.contains("MM[") && !body.contains("fle_mmx"), "{name}: taken by mmx.rs: {body}");
    }
}
