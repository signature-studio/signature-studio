//! Tests for x87.rs (decision 0033): x87 loads, stores, arithmetic, compares and constants on
//! the software f80 registers of runtime/fle_x87.c, against the host CPU.
//!
//! The host runs real machine code (mmap). Per case: FNINIT, FLDCW, FDECSTP (0-7 times), FLD of
//! 0-8 random 10-byte operands, optional EFLAGS, the instruction, then PUSHFQ and FNSAVE (which
//! reads all 8 registers, the tag word and the status word, and resets the x87 state). The lifted C
//! (clang -O0, fle_x87.c, fle_x87ctl.c) starts from the same sequence and runs the lifted body of the
//! same bytes. Compared bit for bit: ST(0..7) (80 bits; empty registers are skipped), the status
//! word (TOP, C0-C3, exception flags), the tag word, EFLAGS CF/PF/ZF/AF/SF/OF, and 64 bytes of
//! memory. The control words have all exceptions masked (a model stop on an unmasked
//! exception is tested separately). Operands are random and edge values: zeros, denormals,
//! pseudo-denormals, unnormals, infinities, QNaNs, SNaNs, pseudo-NaNs, extreme exponents, rounding
//! boundaries.

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[derive(Clone, Debug)]
struct Case {
    name: String,
    bytes: Vec<u8>,
    /// Loaded with FLD in this order, so the last one is ST(0).
    ops: Vec<[u8; 10]>,
    top_dec: u8,
    /// Control word (PC, RC; all exceptions masked).
    cw: u16,
    /// EFLAGS image before the instruction (CF PF AF ZF SF OF bits only).
    flags: u64,
    /// The memory operand base is rsi = mem + 16.
    mem: [u8; 64],
}

#[derive(Debug, PartialEq, Clone)]
struct Out {
    /// ST(0..7) in stack order; None when the tag says empty.
    st: [Option<[u8; 10]>; 8],
    sw: u16,
    tw: u16,
    /// CF, PF, AF, ZF, SF, OF as bits 0..5.
    fl: u8,
    mem: [u8; 64],
}

const FLAG_MASK: u64 = 0x8d5; // CF PF AF ZF SF OF

#[repr(C, align(16))]
struct State {
    mem: [u8; 64],
    ops: [[u8; 16]; 8],
    cw: u64,
    flags_in: u64,
    flags_out: u64,
    sw0: u64,
    save: [u8; 112],
}

const OFF_OPS: usize = std::mem::offset_of!(State, ops);
const OFF_CW: usize = std::mem::offset_of!(State, cw);
const OFF_FIN: usize = std::mem::offset_of!(State, flags_in);
const OFF_FOUT: usize = std::mem::offset_of!(State, flags_out);
const OFF_SW0: usize = std::mem::offset_of!(State, sw0);
const OFF_SAVE: usize = std::mem::offset_of!(State, save);

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

fn stub(c: &Case) -> Vec<u8> {
    let d = |o: usize| (o as i32).to_le_bytes();
    // push rbp; mov rbp, rdi; lea rsi, [rbp+16]
    let mut s = vec![0x55, 0x48, 0x89, 0xfd, 0x48, 0x8d, 0x75, 0x10];
    s.extend_from_slice(&[0xdb, 0xe3]); // fninit
    s.extend_from_slice(&[0xd9, 0xad]); // fldcw [rbp+disp32]
    s.extend_from_slice(&d(OFF_CW));
    for _ in 0..c.top_dec {
        s.extend_from_slice(&[0xd9, 0xf6]); // fdecstp
    }
    for k in 0..c.ops.len() {
        s.extend_from_slice(&[0xdb, 0xad]); // fld tbyte [rbp+disp32]
        s.extend_from_slice(&d(OFF_OPS + 16 * k));
    }
    s.extend_from_slice(&[0xdd, 0xbd]); // fnstsw [rbp+disp32]
    s.extend_from_slice(&d(OFF_SW0));
    s.extend_from_slice(&[0xff, 0xb5]); // push qword [rbp+disp32]
    s.extend_from_slice(&d(OFF_FIN));
    s.push(0x9d); // popfq
    s.extend_from_slice(&c.bytes);
    s.push(0x9c); // pushfq
    s.extend_from_slice(&[0x8f, 0x85]); // pop qword [rbp+disp32]
    s.extend_from_slice(&d(OFF_FOUT));
    s.extend_from_slice(&[0xdd, 0xb5]); // fnsave [rbp+disp32]
    s.extend_from_slice(&d(OFF_SAVE));
    s.extend_from_slice(&[0x5d, 0xc3]); // pop rbp; ret
    s
}

struct Host {
    out: Out,
    sw0: u16,
}

fn parse_save(save: &[u8], fl: u64, mem: [u8; 64]) -> Out {
    let sw = u16::from_le_bytes([save[4], save[5]]);
    let tw = u16::from_le_bytes([save[8], save[9]]);
    let top = ((sw >> 11) & 7) as usize;
    let mut st = [None; 8];
    for (i, slot) in st.iter_mut().enumerate() {
        let p = (top + i) & 7;
        if (tw >> (2 * p)) & 3 != 3 {
            let mut b = [0u8; 10];
            b.copy_from_slice(&save[28 + 10 * i..38 + 10 * i]);
            *slot = Some(b);
        }
    }
    let f = |m: u64, bit: u32| ((fl & m != 0) as u8) << bit;
    let fl8 = f(1, 0) | f(4, 1) | f(0x10, 2) | f(0x40, 3) | f(0x80, 4) | f(0x800, 5);
    Out { st, sw, tw, fl: fl8, mem }
}

fn run_host(c: &Case) -> Host {
    let mut st = Box::new(State { mem: c.mem, ops: [[0; 16]; 8], cw: c.cw as u64, flags_in: 2 | (c.flags & FLAG_MASK), flags_out: 0, sw0: 0, save: [0; 112] });
    for (k, o) in c.ops.iter().enumerate() {
        st.ops[k][..10].copy_from_slice(o);
    }
    let code = stub(c);
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(p);
        f(&mut *st);
        munmap(p, 4096);
    }
    Host { out: parse_save(&st.save, st.flags_out, st.mem), sw0: st.sw0 as u16 }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-x87/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n#include \"fle_x87ctl.h\"\n#include \"fle_x87.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

fn compile(name: &str, src: &str, extra_c: &[&str]) -> PathBuf {
    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let c = dir.join(format!("{name}.c"));
    let exe = dir.join(name);
    std::fs::write(&c, src).expect("write C source");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-ffp-contract=off", "-w", "-I"]).arg(&rt).arg("-o").arg(&exe).arg(&c);
    // FLE_X87_C: a work copy of fle_x87.c (used while the runtime file is not in place yet)
    let x87c = std::env::var("FLE_X87_C").map(PathBuf::from).unwrap_or_else(|_| rt.join("fle_x87.c"));
    cmd.arg(rt.join("fle_x87ctl.c")).arg(x87c);
    for e in extra_c {
        cmd.arg(e);
    }
    let cc = cmd.env("TMPDIR", dir.join("tmp")).output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).expect("hex")
}

/// Runs every case in the lifted C. Returns (sw before, Out) per case.
fn c_run(group: &str, cs: &[Case]) -> Vec<(u16, Out)> {
    let mut src = String::from(HEADER);
    for (k, c) in cs.iter().enumerate() {
        let body = lift_body(&c.bytes, 0x1000).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "struct In { unsigned n; unsigned char ops[8][10]; unsigned top_dec, cw, flags; uint8_t mem[64]; };\n";
    src += "static const struct In IN[] = {\n";
    for c in cs {
        let ops: Vec<String> = (0..8)
            .map(|k| {
                let o = c.ops.get(k).copied().unwrap_or([0; 10]);
                format!("{{ {} }}", o.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "))
            })
            .collect();
        let mem: Vec<String> = c.mem.iter().map(|b| b.to_string()).collect();
        src += &format!("{{ {}, {{ {} }}, {}, {}, {}, {{ {} }} }},\n", c.ops.len(), ops.join(", "), c.top_dec, c.cw, c.flags & FLAG_MASK, mem.join(", "));
    }
    src += "};\n";
    src += "static void (*const FN[])(void) = {";
    for k in 0..cs.len() {
        src += &format!(" c{k},");
    }
    src += " };\n\n";
    src += r#"int main(void)
{
    unsigned k, j, i;
    for (k = 0; k < NCASES; k++) {
        const struct In *in = &IN[k];
        unsigned top;
        fle_x87_init();
        fle_x87.cw = (uint16_t)in->cw;
        fle_x87.sw = (uint16_t)(((8 - in->top_dec) & 7) << 11);
        for (j = 0; j < in->n; j++) fle_x87_fld_m((uint64_t)(uintptr_t)in->ops[j], 10);
        printf("%u %x", k, (unsigned)fle_x87.sw);
        R[6] = (uint64_t)(uintptr_t)(M + 16);
        memcpy(M, in->mem, sizeof M);
        FL.cf = in->flags & 1; FL.pf = (in->flags >> 2) & 1; FL.af = (in->flags >> 4) & 1;
        FL.zf = (in->flags >> 6) & 1; FL.sf = (in->flags >> 7) & 1; FL.of = (in->flags >> 11) & 1;
        FN[k]();
        top = (fle_x87.sw >> 11) & 7;
        printf(" %x %x %x", (unsigned)fle_x87.sw, (unsigned)fle_x87.tw,
               FL.cf | (FL.pf << 1) | (FL.af << 2) | (FL.zf << 3) | (FL.sf << 4) | (FL.of << 5));
        for (i = 0; i < 8; i++) {
            unsigned p = (top + i) & 7;
            printf(" %llx:%x", (unsigned long long)fle_x87_r[p].m, (unsigned)fle_x87_r[p].se);
        }
        for (j = 0; j < 64; j++) printf(" %02x", M[j]);
        printf("\n");
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());
    let exe = compile(&format!("x87_{group}"), &src, &[]);
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed ({group}): {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8 output");
    text.lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let sw0 = hex(t[1]) as u16;
            let sw = hex(t[2]) as u16;
            let tw = hex(t[3]) as u16;
            let fl = hex(t[4]) as u8;
            let mut st = [None; 8];
            for (i, slot) in st.iter_mut().enumerate() {
                let (m, se) = t[5 + i].split_once(':').expect("reg");
                let p = (((sw >> 11) & 7) as usize + i) & 7;
                if (tw >> (2 * p)) & 3 != 3 {
                    let mut b = [0u8; 10];
                    b[..8].copy_from_slice(&hex(m).to_le_bytes());
                    b[8..].copy_from_slice(&(hex(se) as u16).to_le_bytes());
                    *slot = Some(b);
                }
            }
            let mut mem = [0u8; 64];
            for (j, m) in mem.iter_mut().enumerate() {
                *m = u8::from_str_radix(t[13 + j], 16).expect("byte");
            }
            (sw0, Out { st, sw, tw, fl, mem })
        })
        .collect()
}

/// Returns the number of cases.
fn check(group: &str, cs: &[Case]) -> usize {
    assert!(!cs.is_empty(), "{group}: no cases");
    let hosts: Vec<Host> = cs.iter().map(run_host).collect();
    // one C program per 4000 cases (a single translation unit with tens of thousands of
    // functions crashes clang at X87_ROUNDS=8)
    let mut c: Vec<(u16, Out)> = Vec::new();
    for (n, chunk) in cs.chunks(4000).enumerate() {
        c.extend(c_run(&format!("{group}_{n}"), chunk));
    }
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    let mut changed = 0usize;
    for ((case, (csw0, cout)), h) in cs.iter().zip(c.iter()).zip(hosts.iter()) {
        assert_eq!(*csw0, h.sw0, "{group}: {}: status word before the instruction (test setup)", case.name);
        if h.out.sw != h.sw0 || h.out.mem != case.mem || h.out.st != c_before(case) {
            changed += 1;
        }
        if h.out != *cout {
            fails += 1;
            if fails <= 8 {
                eprintln!(
                    "{group}: {}\n  bytes {:02x?} top_dec {} cw {:04x} flags {:x}\n  ops {:02x?}\n  C    {:x?}\n  host {:x?}",
                    case.name, case.bytes, case.top_dec, case.cw, case.flags, case.ops, cout, h.out
                );
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed_state {changed}", cs.len());
    assert!(changed * 10 >= cs.len() * 6, "{group}: only {changed} of {} cases changed the state", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
    cs.len()
}

/// The register contents as loaded (for the "did the instruction change anything" count).
fn c_before(c: &Case) -> [Option<[u8; 10]>; 8] {
    let mut st = [None; 8];
    for (i, o) in c.ops.iter().rev().enumerate() {
        st[i] = Some(*o);
    }
    st
}

// ---------------------------------------------------------------- operand generation

fn f80(m: u64, se: u16) -> [u8; 10] {
    let mut b = [0u8; 10];
    b[..8].copy_from_slice(&m.to_le_bytes());
    b[8..].copy_from_slice(&se.to_le_bytes());
    b
}

fn split(b: &[u8; 10]) -> (u64, u16) {
    (u64::from_le_bytes(b[..8].try_into().unwrap()), u16::from_le_bytes(b[8..].try_into().unwrap()))
}

const TOPBIT: u64 = 1 << 63;

fn rand_m(rng: &mut Rng) -> u64 {
    match rng.below(8) {
        0 => TOPBIT,
        1 => u64::MAX,
        2 => TOPBIT | 1,
        3 => TOPBIT | (rng.next_u64() & 0xff),
        4 => u64::MAX << rng.below(64) | TOPBIT,
        5 => TOPBIT | (1 << rng.below(63)),
        6 => (TOPBIT | rng.next_u64()) & !(((1u64 << rng.below(40)) - 1) | 0),
        _ => TOPBIT | rng.next_u64(),
    }
}

fn sign(rng: &mut Rng) -> u16 {
    if rng.below(2) == 0 {
        0
    } else {
        0x8000
    }
}

/// One operand. `near` is an exponent to stay close to (for add/sub), when given.
fn gen_f80(rng: &mut Rng, near: Option<i32>) -> [u8; 10] {
    let s = sign(rng);
    let class = rng.below(20);
    match class {
        0 => f80(0, s),
        1 => f80(rng.next_u64() >> rng.below(64) & !TOPBIT, s), // denormal
        2 => f80(TOPBIT | (rng.next_u64() >> rng.below(64)), s), // pseudo-denormal
        3 => f80(rng.next_u64() & !TOPBIT, s | (1 + rng.below(0x7ffe) as u16)), // unnormal
        4 => f80(TOPBIT, s | 0x7fff),                           // infinity
        5 => f80(TOPBIT | 0x4000_0000_0000_0000 | (rng.next_u64() >> 2 >> rng.below(60)), s | 0x7fff), // QNaN
        6 => f80(TOPBIT | (1 + (rng.next_u64() >> 2 >> rng.below(60)) % (1 << 62)), s | 0x7fff), // SNaN (bit 62 clear)
        7 => f80(rng.next_u64() & !TOPBIT, s | 0x7fff),         // pseudo-NaN / pseudo-infinity
        8 => f80(rand_m(rng), s | [1u16, 2, 0x7ffe, 0x7ffd, 0x3fff, 0x3ffe, 0x4000, 0x3fc0, 0x403e, 0x403f][rng.below(10) as usize]),
        9 | 10 | 11 | 12 | 13 => {
            if let Some(n) = near {
                let e = (n + rng.below(150) as i32 - 75).clamp(0, 0x7ffe);
                f80(rand_m(rng), s | e as u16)
            } else {
                f80(rand_m(rng), s | (0x3fff - 70 + rng.below(140) as u16))
            }
        }
        14 | 15 => near_int(rng),
        _ => f80(rand_m(rng), s | (1 + rng.below(0x7ffe) as u16)),
    }
}

/// Values near integers and halves: exponent 0x3ffd..=0x403f, fraction patterns at the rounding bit.
fn near_int(rng: &mut Rng) -> [u8; 10] {
    let s = sign(rng);
    let e = 0x3ffc + rng.below(0x40 + 4) as u16;
    let ue = e as i32 - 0x3fff; // unbiased
    let m = if ue < 0 {
        rand_m(rng)
    } else {
        let frac_bits = 63 - ue.min(63) as u32; // bits below the binary point
        if frac_bits == 0 {
            rand_m(rng)
        } else {
            let hi = (TOPBIT | rng.next_u64()) & !((1u64 << frac_bits) - 1);
            let half = 1u64 << (frac_bits - 1);
            let frac = match rng.below(6) {
                0 => 0,
                1 => half,
                2 => half + 1,
                3 => half.wrapping_sub(1),
                4 => (1u64 << frac_bits) - 1,
                _ => rng.next_u64() & ((1u64 << frac_bits) - 1),
            };
            hi | frac
        }
    };
    f80(m | if ue >= 0 { 0 } else { TOPBIT }, s | e)
}

fn exp_of(b: &[u8; 10]) -> i32 {
    (split(b).1 & 0x7fff) as i32
}

fn rand_cw(rng: &mut Rng) -> u16 {
    // PC (bits 8-9) and RC (10-11) free; all exceptions masked (bits 0-5); bit 6 as the CPU reads it back
    0x007f | ((rng.below(4) as u16) << 8) | ((rng.below(4) as u16) << 10)
}

fn rand_mem(rng: &mut Rng) -> [u8; 64] {
    let mut m = [0u8; 64];
    for b in m.iter_mut() {
        *b = rng.next_u64() as u8;
    }
    m
}

const DISPS: [i8; 8] = [-16, -9, 0, 3, 8, 16, 24, 30];

fn disp(rng: &mut Rng) -> i8 {
    DISPS[rng.below(DISPS.len() as u64) as usize]
}

fn base(name: String, bytes: Vec<u8>, nops: usize, rng: &mut Rng) -> Case {
    let mut ops: Vec<[u8; 10]> = Vec::new();
    for _ in 0..nops {
        let near = ops.last().map(exp_of);
        ops.push(gen_f80(rng, near));
    }
    Case { name, bytes, ops, top_dec: rng.below(8) as u8, cw: rand_cw(rng), flags: rng.next_u64() & FLAG_MASK, mem: rand_mem(rng) }
}

/// [opcode] modrm with [rsi + disp8].
fn mem_enc(opc: u8, reg: u8, d: i8) -> Vec<u8> {
    vec![opc, 0x40 | (reg << 3) | 6, d as u8]
}

fn put_mem(c: &mut Case, d: i8, v: &[u8]) {
    let o = (16 + d as i32) as usize;
    c.mem[o..o + v.len()].copy_from_slice(v);
}

fn f32_bits(rng: &mut Rng) -> u32 {
    let s = (rng.below(2) as u32) << 31;
    match rng.below(10) {
        0 => s,
        1 => s | (rng.next_u64() as u32 & 0x7f_ffff),                // denormal
        2 => s | 0x7f80_0000,                                        // inf
        3 => s | 0x7f80_0000 | (rng.next_u64() as u32 & 0x7f_ffff),  // NaN of either kind
        4 => s | 0x7f00_0000 | (rng.next_u64() as u32 & 0x80_ffff),  // near max
        5 => s | 0x0080_0000 | (rng.next_u64() as u32 & 0xf),        // near min normal
        _ => s | (rng.next_u64() as u32 & 0x7fff_ffff),
    }
}

fn f64_bits(rng: &mut Rng) -> u64 {
    let s = rng.below(2) << 63;
    match rng.below(10) {
        0 => s,
        1 => s | (rng.next_u64() & 0xf_ffff_ffff_ffff),
        2 => s | 0x7ff0_0000_0000_0000,
        3 => s | 0x7ff0_0000_0000_0000 | (rng.next_u64() & 0xf_ffff_ffff_ffff),
        4 => s | 0x7fe0_0000_0000_0000 | (rng.next_u64() & 0x1f_ffff_ffff_ffff),
        5 => s | 0x0010_0000_0000_0000 | (rng.next_u64() & 0xf),
        _ => s | (rng.next_u64() & 0x7fff_ffff_ffff_ffff),
    }
}

fn int_val(rng: &mut Rng, bits: u32) -> i64 {
    let v = match rng.below(6) {
        0 => 0,
        1 => 1,
        2 => -1,
        3 => i64::MIN >> (64 - bits),
        4 => (i64::MAX >> (64 - bits)) - rng.below(2) as i64,
        _ => (rng.next_u64() as i64) >> (64 - bits + rng.below(bits as u64) as u32),
    };
    (v << (64 - bits)) >> (64 - bits)
}

// ---------------------------------------------------------------- groups

fn rounds() -> usize {
    std::env::var("X87_ROUNDS").ok().and_then(|s| s.parse().ok()).unwrap_or(1)
}

fn loads(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..300 * rounds() {
        let n = rng.below(8) as usize; // 0..7 loaded: room for the push
        let d = disp(rng);
        let mut c = base(format!("fld m32 #{k}"), mem_enc(0xd9, 0, d), n, rng);
        put_mem(&mut c, d, &f32_bits(rng).to_le_bytes());
        v.push(c);
        let mut c = base(format!("fld m64 #{k}"), mem_enc(0xdd, 0, d), n, rng);
        put_mem(&mut c, d, &f64_bits(rng).to_le_bytes());
        v.push(c);
        let mut c = base(format!("fld m80 #{k}"), mem_enc(0xdb, 5, d), n, rng);
        let o = gen_f80(rng, None);
        put_mem(&mut c, d, &o);
        v.push(c);
        let n1 = 1 + rng.below(7) as usize;
        let i = rng.below(n1 as u64) as u8;
        v.push(base(format!("fld st({i}) #{k}"), vec![0xd9, 0xc0 | i], n1, rng));
        let mut c = base(format!("fild m16 #{k}"), mem_enc(0xdf, 0, d), n, rng);
        put_mem(&mut c, d, &(int_val(rng, 16) as i16).to_le_bytes());
        v.push(c);
        let mut c = base(format!("fild m32 #{k}"), mem_enc(0xdb, 0, d), n, rng);
        put_mem(&mut c, d, &(int_val(rng, 32) as i32).to_le_bytes());
        v.push(c);
        let mut c = base(format!("fild m64 #{k}"), mem_enc(0xdf, 5, d), n, rng);
        put_mem(&mut c, d, &int_val(rng, 64).to_le_bytes());
        v.push(c);
    }
    for (k, opc) in [[0xd9u8, 0xe8], [0xd9, 0xe9], [0xd9, 0xea], [0xd9, 0xeb], [0xd9, 0xec], [0xd9, 0xed], [0xd9, 0xee]].iter().enumerate() {
        for r in 0..40 * rounds() {
            let n = rng.below(8) as usize;
            v.push(base(format!("const{k} #{r}"), opc.to_vec(), n, rng));
        }
    }
    v
}

fn stores(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..300 * rounds() {
        let n = 1 + rng.below(8) as usize;
        let d = disp(rng);
        for (name, opc, reg) in [("fst m32", 0xd9u8, 2u8), ("fstp m32", 0xd9, 3), ("fst m64", 0xdd, 2), ("fstp m64", 0xdd, 3), ("fstp m80", 0xdb, 7)] {
            let mut c = base(format!("{name} #{k}"), mem_enc(opc, reg, d), n, rng);
            if rng.below(2) == 0 {
                // an operand converted without overflow: exponent range of f32/f64 or round-near-edge
                let (m, _) = split(&c.ops[n - 1]);
                let e = if opc == 0xd9 { 0x3fff - 150 + rng.below(300) as u16 } else { 0x3fff - 1080 + rng.below(2100) as u16 };
                c.ops[n - 1] = f80(m | TOPBIT, sign(rng) | e);
            }
            v.push(c);
        }
        let i = rng.below(8) as u8;
        v.push(base(format!("fst st({i}) #{k}"), vec![0xdd, 0xd0 | i], n.max(i as usize + 1), rng));
        v.push(base(format!("fstp st({i}) #{k}"), vec![0xdd, 0xd8 | i], n.max(i as usize + 1), rng));
    }
    v
}

fn fists(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..400 * rounds() {
        let n = 1 + rng.below(8) as usize;
        let d = disp(rng);
        for (name, opc, reg) in [
            ("fist m16", 0xdfu8, 2u8),
            ("fist m32", 0xdb, 2),
            ("fistp m16", 0xdf, 3),
            ("fistp m32", 0xdb, 3),
            ("fistp m64", 0xdf, 7),
            ("fisttp m16", 0xdf, 1),
            ("fisttp m32", 0xdb, 1),
            ("fisttp m64", 0xdd, 1),
        ] {
            let mut c = base(format!("{name} #{k}"), mem_enc(opc, reg, d), n, rng);
            if rng.below(3) != 0 {
                c.ops[n - 1] = near_int(rng);
            }
            v.push(c);
        }
    }
    v
}

const ARITH: [(&str, u8); 6] = [("add", 0), ("mul", 1), ("sub", 4), ("subr", 5), ("div", 6), ("divr", 7)];

fn arith_st(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..250 * rounds() {
        for (name, r) in ARITH {
            let n = 2 + rng.below(7) as usize; // 2..8
            let i = (rng.below(n as u64 - 1) + 1) as u8; // 1..n-1 (st(i) valid)
            // op st, st(i)
            v.push(base(format!("f{name} st, st({i}) #{k}"), vec![0xd8, 0xc0 | (r << 3) | i], n, rng));
            // op st(i), st : DC/DE swap the sub/subr and div/divr encodings
            let rr = match r {
                4 => 5,
                5 => 4,
                6 => 7,
                7 => 6,
                x => x,
            };
            v.push(base(format!("f{name} st({i}), st #{k}"), vec![0xdc, 0xc0 | (rr << 3) | i], n, rng));
            v.push(base(format!("f{name}p st({i}), st #{k}"), vec![0xde, 0xc0 | (rr << 3) | i], n, rng));
            // st(0) with st(0)
            v.push(base(format!("f{name} st, st #{k}"), vec![0xd8, 0xc0 | (r << 3)], n, rng));
        }
        // zero and infinity heavy pairs
        for (name, r) in ARITH {
            let mut c = base(format!("f{name} st, st(1) special #{k}"), vec![0xd8, 0xc0 | (r << 3) | 1], 2, rng);
            let sp = [f80(0, 0), f80(0, 0x8000), f80(TOPBIT, 0x7fff), f80(TOPBIT, 0xffff), f80(TOPBIT, 0x3fff), f80(TOPBIT, 0xbfff)];
            c.ops[0] = sp[rng.below(6) as usize];
            c.ops[1] = sp[rng.below(6) as usize];
            v.push(c);
        }
    }
    v
}

fn arith_mem(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..250 * rounds() {
        for (name, r) in ARITH {
            let n = 1 + rng.below(8) as usize;
            let d = disp(rng);
            let mut c = base(format!("f{name} m32 #{k}"), mem_enc(0xd8, r, d), n, rng);
            put_mem(&mut c, d, &f32_bits(rng).to_le_bytes());
            v.push(c);
            let mut c = base(format!("f{name} m64 #{k}"), mem_enc(0xdc, r, d), n, rng);
            put_mem(&mut c, d, &f64_bits(rng).to_le_bytes());
            v.push(c);
            let mut c = base(format!("fi{name} m32 #{k}"), mem_enc(0xda, r, d), n, rng);
            put_mem(&mut c, d, &(int_val(rng, 32) as i32).to_le_bytes());
            v.push(c);
            let mut c = base(format!("fi{name} m16 #{k}"), mem_enc(0xde, r, d), n, rng);
            put_mem(&mut c, d, &(int_val(rng, 16) as i16).to_le_bytes());
            v.push(c);
        }
    }
    v
}

/// Makes equal and signed-zero operand pairs common in the compare cases.
fn pair(c: &mut Case, i: u8, rng: &mut Rng) {
    let l = c.ops.len();
    match rng.below(6) {
        0 => c.ops[l - 1] = c.ops[l - 1 - i as usize],
        1 => {
            let a = sign(rng);
            let b = sign(rng);
            c.ops[l - 1] = f80(0, a);
            c.ops[l - 1 - i as usize] = f80(0, b);
        }
        _ => {}
    }
}

fn compares(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..300 * rounds() {
        let n = 2 + rng.below(7) as usize;
        let i = (rng.below(n as u64 - 1) + 1) as u8;
        for (name, bytes) in [
            ("fcom", vec![0xd8, 0xd0 | i]),
            ("fcomp", vec![0xd8, 0xd8 | i]),
            ("fucom", vec![0xdd, 0xe0 | i]),
            ("fucomp", vec![0xdd, 0xe8 | i]),
            ("fcomi", vec![0xdb, 0xf0 | i]),
            ("fcomip", vec![0xdf, 0xf0 | i]),
            ("fucomi", vec![0xdb, 0xe8 | i]),
            ("fucomip", vec![0xdf, 0xe8 | i]),
        ] {
            let mut c = base(format!("{name} st({i}) #{k}"), bytes, n, rng);
            pair(&mut c, i, rng);
            v.push(c);
        }
        let mut c = base(format!("fcompp #{k}"), vec![0xde, 0xd9], n, rng);
        pair(&mut c, i, rng);
        v.push(c);
        let mut c = base(format!("fucompp #{k}"), vec![0xda, 0xe9], n, rng);
        pair(&mut c, i, rng);
        v.push(c);
        let nn = 1 + rng.below(8) as usize;
        let d = disp(rng);
        for (name, opc, reg) in [("fcom m32", 0xd8u8, 2u8), ("fcomp m32", 0xd8, 3), ("fcom m64", 0xdc, 2), ("fcomp m64", 0xdc, 3)] {
            let mut c = base(format!("{name} #{k}"), mem_enc(opc, reg, d), nn, rng);
            if opc == 0xd8 {
                put_mem(&mut c, d, &f32_bits(rng).to_le_bytes());
            } else {
                put_mem(&mut c, d, &f64_bits(rng).to_le_bytes());
            }
            v.push(c);
        }
        for (name, opc, reg, w) in [("ficom m16", 0xdeu8, 2u8, 16u32), ("ficomp m16", 0xde, 3, 16), ("ficom m32", 0xda, 2, 32), ("ficomp m32", 0xda, 3, 32)] {
            let mut c = base(format!("{name} #{k}"), mem_enc(opc, reg, d), nn, rng);
            let iv = int_val(rng, w);
            if rng.below(3) == 0 {
                // an operand equal to the integer, so the equal case is common
                let l = c.ops.len();
                let mag = iv.unsigned_abs();
                c.ops[l - 1] = if mag == 0 {
                    f80(0, 0)
                } else {
                    let s = mag.leading_zeros();
                    f80(mag << s, ((iv < 0) as u16) << 15 | (0x3fff + 63 - s as u16))
                };
            }
            if w == 16 {
                put_mem(&mut c, d, &(iv as i16).to_le_bytes());
            } else {
                put_mem(&mut c, d, &(iv as i32).to_le_bytes());
            }
            v.push(c);
        }
        // ftst and fxam, with empty ST(0) too
        let nz = rng.below(9) as usize;
        v.push(base(format!("ftst #{k}"), vec![0xd9, 0xe4], nz.max(1), rng));
        v.push(base(format!("fxam #{k}"), vec![0xd9, 0xe5], nz, rng));
    }
    v
}

fn misc(rng: &mut Rng) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..300 * rounds() {
        let n = 2 + rng.below(7) as usize;
        let i = (rng.below(n as u64 - 1) + 1) as u8;
        v.push(base(format!("fxch st({i}) #{k}"), vec![0xd9, 0xc8 | i], n, rng));
        v.push(base(format!("fxch #{k}"), vec![0xd9, 0xc9], n, rng));
        for (name, bytes) in [
            ("fcmovb", vec![0xda, 0xc0 | i]),
            ("fcmove", vec![0xda, 0xc8 | i]),
            ("fcmovbe", vec![0xda, 0xd0 | i]),
            ("fcmovu", vec![0xda, 0xd8 | i]),
            ("fcmovnb", vec![0xdb, 0xc0 | i]),
            ("fcmovne", vec![0xdb, 0xc8 | i]),
            ("fcmovnbe", vec![0xdb, 0xd0 | i]),
            ("fcmovnu", vec![0xdb, 0xd8 | i]),
        ] {
            v.push(base(format!("{name} st({i}) #{k}"), bytes, n, rng));
        }
        let n1 = 1 + rng.below(8) as usize;
        for (name, bytes) in [("fchs", vec![0xd9, 0xe0]), ("fabs", vec![0xd9, 0xe1]), ("fsqrt", vec![0xd9, 0xfa]), ("frndint", vec![0xd9, 0xfc])] {
            let mut c = base(format!("{name} #{k}"), bytes, n1, rng);
            if name == "frndint" && rng.below(2) == 0 {
                let l = c.ops.len();
                c.ops[l - 1] = near_int(rng);
            }
            if name == "fsqrt" && rng.below(2) == 0 {
                let l = c.ops.len();
                let (m, se) = split(&c.ops[l - 1]);
                c.ops[l - 1] = f80(m, se & 0x7fff);
            }
            v.push(c);
        }
        let mut c = base(format!("fscale #{k}"), vec![0xd9, 0xfd], n.max(2), rng);
        let l = c.ops.len();
        // the scale operand: small integers and fractions most of the time
        if rng.below(4) != 0 {
            let e = 0x3fff - 3 + rng.below(18) as u16;
            c.ops[l - 2] = f80(rand_m(rng), sign(rng) | e);
        }
        v.push(c);
    }
    v
}

fn run_group(name: &str, f: fn(&mut Rng) -> Vec<Case>, seed: u64) -> usize {
    // X87_SEED: xor into every group's seed (extra random cases on demand)
    let extra: u64 = std::env::var("X87_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut rng = Rng(seed ^ extra.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    check(name, &f(&mut rng))
}

#[test]
fn x87_loads() {
    run_group("loads", loads, 0x1234_5678_9abc_def1);
}

#[test]
fn x87_stores() {
    run_group("stores", stores, 0x2234_5678_9abc_def1);
}

#[test]
fn x87_integer_stores() {
    run_group("fists", fists, 0x3234_5678_9abc_def1);
}

#[test]
fn x87_arith_register_forms() {
    run_group("arith_st", arith_st, 0x4234_5678_9abc_def1);
}

#[test]
fn x87_arith_memory_forms() {
    run_group("arith_mem", arith_mem, 0x5234_5678_9abc_def1);
}

#[test]
fn x87_compares() {
    run_group("compares", compares, 0x6234_5678_9abc_def1);
}

#[test]
fn x87_misc() {
    run_group("misc", misc, 0x7234_5678_9abc_def1);
}

// ---------------------------------------------------------------- fail-closed behaviour

/// Compiles and runs a C `main` body against the runtime. Returns (exit ok, stdout, stderr).
fn run_c(name: &str, body: &str) -> (bool, String, String) {
    let src = format!("{HEADER}int main(void)\n{{\n{body}\n    return 0;\n}}\n");
    let exe = compile(&format!("x87fc_{name}"), &src, &[]);
    let run = Command::new(&exe).output().expect("run test binary");
    (run.status.success(), String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

const ONE: &str = "static const unsigned char ONE[10] = { 0,0,0,0,0,0,0,0x80,0xff,0x3f };\n    static const unsigned char BIG[10] = { 0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xfe,0x7f };\n    static const unsigned char TINY[10] = { 0,0,0,0,0,0,0,0x80,1,0 };\n    static const unsigned char ZERO[10] = { 0 };\n    static const unsigned char SNAN[10] = { 1,0,0,0,0,0,0,0x80,0xff,0x7f };\n";

#[test]
fn x87_unmasked_exception_stops_with_a_named_abort() {
    // overflow with OE unmasked (cw 0x0377), invalid with IE unmasked, zero divide with ZE unmasked
    for (name, cw, a, b, op) in [("oe", 0x0377, "BIG", "BIG", "FLE_X87_ADD"), ("ie", 0x037e, "SNAN", "ONE", "FLE_X87_ADD"), ("ze", 0x037b, "ONE", "ZERO", "FLE_X87_DIV")] {
        let body = format!(
            "{ONE}    fle_x87_init(); fle_x87.cw = {cw};\n    fle_x87_fld_m((uint64_t)(uintptr_t){b}, 10); fle_x87_fld_m((uint64_t)(uintptr_t){a}, 10);\n    fle_x87_arith_st({op}, 0, 1, 0);\n    printf(\"survived\\n\");"
        );
        let (ok, out, err) = run_c(&format!("unmasked_{name}"), &body);
        assert!(!ok && !out.contains("survived"), "{name}: the unmasked exception did not stop the run");
        assert!(err.contains("fle: x87: unmasked exception"), "{name}: stderr was {err:?}");
    }
    // the same operations with all exceptions masked give the masked result and set the flag
    let body = format!(
        "{ONE}    fle_x87_init();\n    fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10); fle_x87_fld_m((uint64_t)(uintptr_t)ZERO, 10);\n    fle_x87_arith_st(FLE_X87_DIV, 1, 0, 0);\n    printf(\"%04x %x\\n\", (unsigned)fle_x87_r[(((fle_x87.sw >> 11) & 7) + 1) & 7].se, (unsigned)(fle_x87.sw & 0x3f));"
    );
    let (ok, out, _) = run_c("masked_ze", &body);
    assert!(ok && out.trim() == "7fff 4", "masked divide by zero gives infinity and ZE: {out:?}");
}

#[test]
fn x87_unmasked_pending_flag_stops_at_the_next_instruction() {
    // the status word has IE set and the control word then unmasks IE (FLDCW): the next x87
    // instruction would raise #MF on the CPU; the model stops
    let body = format!(
        "{ONE}    fle_x87_init(); fle_x87.sw |= 1; fle_x87.cw = 0x037e;\n    fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10);\n    printf(\"survived\\n\");"
    );
    let (ok, out, err) = run_c("pending", &body);
    assert!(!ok && !out.contains("survived"), "pending unmasked flag did not stop the run");
    assert!(err.contains("fle: x87: pending exception flag is unmasked"), "stderr was {err:?}");
}

#[test]
fn x87_stack_overflow_and_underflow_stop() {
    let body = format!("{ONE}    int k;\n    fle_x87_init();\n    for (k = 0; k < 9; k++) fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10);\n    printf(\"survived\\n\");");
    let (ok, out, err) = run_c("overflow", &body);
    assert!(!ok && !out.contains("survived") && err.contains("fle: x87: stack overflow"), "overflow: {err:?}");
    let body = format!("{ONE}    fle_x87_init();\n    fle_x87_fchs();\n    printf(\"survived\\n\");");
    let (ok, out, err) = run_c("underflow", &body);
    assert!(!ok && !out.contains("survived") && err.contains("fle: x87: stack underflow"), "underflow: {err:?}");
    let body = format!("{ONE}    fle_x87_init();\n    fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10);\n    fle_x87_arith_st(FLE_X87_ADD, 0, 1, 0);\n    printf(\"survived\\n\");");
    let (ok, out, err) = run_c("underflow_st1", &body);
    assert!(!ok && !out.contains("survived") && err.contains("fle: x87: stack underflow"), "underflow ST(1): {err:?}");
}

#[test]
fn x87_after_mmx_without_emms_stops_and_after_emms_runs() {
    // MMX instruction (paddw mm0, mm1 = 0F FD C1) then an x87 load: MM is not aliased to ST, stop
    let body = format!(
        "{ONE}    fle_x87_init();\n    MM[0] = 1; MM[1] = 2; MM[0] = fle_mmx_paddw(MM[0], MM[1]); fle_mmx_touch();\n    fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10);\n    printf(\"survived\\n\");"
    );
    let (ok, out, err) = run_c("mmx_mix", &body);
    assert!(!ok && !out.contains("survived") && err.contains("fle: x87: x87 instruction while MMX state is live"), "mmx mix: {err:?}");
    let body = format!(
        "{ONE}    fle_x87_init();\n    MM[0] = 1; fle_mmx_touch(); fle_mmx_emms();\n    fle_x87_fld_m((uint64_t)(uintptr_t)ONE, 10);\n    printf(\"%x %x\\n\", (unsigned)fle_x87.tw, (unsigned)((fle_x87.sw >> 11) & 7));"
    );
    let (ok, out, err) = run_c("mmx_emms", &body);
    // tag word 3fff: physical register 7 (the new ST(0), TOP 7) is valid, the other seven are empty
    assert!(ok && out.trim() == "3fff 7", "after EMMS the x87 runs: {out:?} {err:?}");
}

#[test]
fn x87_forms_not_implemented_stay_unsupported() {
    for (name, b) in [
        ("fsin", vec![0xd9u8, 0xfe]),
        ("fcos", vec![0xd9, 0xff]),
        ("fptan", vec![0xd9, 0xf2]),
        ("fpatan", vec![0xd9, 0xf3]),
        ("fyl2x", vec![0xd9, 0xf1]),
        ("fyl2xp1", vec![0xd9, 0xf9]),
        ("fprem", vec![0xd9, 0xf8]),
        ("fprem1", vec![0xd9, 0xf5]),
        ("fxtract", vec![0xd9, 0xf4]),
        ("fnstenv", vec![0xd9, 0x37]),
        ("fnsave", vec![0xdd, 0x37]),
        ("fldenv", vec![0xd9, 0x27]),
        ("frstor", vec![0xdd, 0x27]),
        ("fbld", vec![0xdf, 0x27]),
        ("fbstp", vec![0xdf, 0x37]),
        ("fdecstp", vec![0xd9, 0xf6]),
        ("fincstp", vec![0xd9, 0xf7]),
        ("ffree", vec![0xdd, 0xc0]),
    ] {
        assert!(lift_body(&b, 0x1000).is_err(), "{name} must stay unsupported");
    }
}

#[test]
fn x87_every_closure_form_lifts() {
    // the 49 operand forms of the all-pdata closure (docs/decisions/0033-x87.md): all lift
    let forms: &[(&str, &[u8])] = &[
        ("fld m32", &[0xd9, 0x04, 0x82]),
        ("fld m64", &[0xdd, 0x01]),
        ("fld st(i)", &[0xd9, 0xc1]),
        ("fmul st,st(i)", &[0xd8, 0xc9]),
        ("fmul st(i),st", &[0xdc, 0xc9]),
        ("fmulp", &[0xde, 0xc9]),
        ("faddp", &[0xde, 0xc1]),
        ("fadd st,st(i)", &[0xd8, 0xc1]),
        ("fadd st(i),st", &[0xdc, 0xc1]),
        ("fadd m32", &[0xd8, 0x00]),
        ("fxch", &[0xd9, 0xc9]),
        ("fmul m32", &[0xd8, 0x08]),
        ("fmul m64", &[0xdc, 0x08]),
        ("fstp m32", &[0xd9, 0x18]),
        ("fstp m64", &[0xdd, 0x18]),
        ("fstp st(i)", &[0xdd, 0xd9]),
        ("fst m32", &[0xd9, 0x10]),
        ("fst m64", &[0xdd, 0x10]),
        ("fst st(i)", &[0xdd, 0xd1]),
        ("fcompp", &[0xde, 0xd9]),
        ("fsubp", &[0xde, 0xe9]),
        ("fsub st,st(i)", &[0xd8, 0xe1]),
        ("fsub st(i),st", &[0xdc, 0xe9]),
        ("fsub m32", &[0xd8, 0x20]),
        ("fild m16", &[0xdf, 0x00]),
        ("fild m32", &[0xdb, 0x00]),
        ("fild m64", &[0xdf, 0x28]),
        ("fabs", &[0xd9, 0xe1]),
        ("fdivp", &[0xde, 0xf9]),
        ("fchs", &[0xd9, 0xe0]),
        ("fistp m16", &[0xdf, 0x18]),
        ("fistp m32", &[0xdb, 0x18]),
        ("fistp m64", &[0xdf, 0x38]),
        ("fist m32", &[0xdb, 0x10]),
        ("fldz", &[0xd9, 0xee]),
        ("fld1", &[0xd9, 0xe8]),
        ("fldpi", &[0xd9, 0xeb]),
        ("fldl2e", &[0xd9, 0xea]),
        ("fimul m32", &[0xda, 0x08]),
        ("fisub m32", &[0xda, 0x20]),
        ("fcomp m32", &[0xd8, 0x18]),
        ("fucomi", &[0xdb, 0xe9]),
        ("fucomip", &[0xdf, 0xe9]),
        ("fcmovb", &[0xda, 0xc1]),
        ("fcmovnb", &[0xdb, 0xc1]),
        ("frndint", &[0xd9, 0xfc]),
        ("fscale", &[0xd9, 0xfd]),
        // decision 0043 (tests/x87_trans.rs runs them)
        ("f2xm1", &[0xd9, 0xf0]),
        ("fsincos", &[0xd9, 0xfb]),
    ];
    for (name, b) in forms {
        let body = lift_body(b, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(body.contains("fle_x87_") || body.contains("fle_cc") || body.contains("FL."), "{name}: {body}");
    }
}

#[test]
fn x87_lifted_text_of_a_few_forms() {
    let t = |b: &[u8]| lift_body(b, 0x1000).expect("lift");
    assert_eq!(t(&[0xd9, 0x00]), "{ void fle_x87_fld_m(uint64_t, unsigned); fle_x87_fld_m((R[0]), 4); }");
    assert_eq!(t(&[0xde, 0xc1]), "{ void fle_x87_arith_st(unsigned, unsigned, unsigned, unsigned); fle_x87_arith_st(0, 1, 0, 1); }");
    assert_eq!(t(&[0xde, 0xe9]), "{ void fle_x87_arith_st(unsigned, unsigned, unsigned, unsigned); fle_x87_arith_st(1, 1, 0, 1); }");
    assert_eq!(t(&[0xd9, 0xe8]), "{ void fle_x87_fld_const(unsigned); fle_x87_fld_const(0); }");
}
