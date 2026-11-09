//! Tests for decision 0047 (A2a remaining instructions): PSUBW, PCMPGTD, ANDNPD, RCPPS (xmm forms), BSWAP r32/r64,
//! TEST and AND (and every other 8-bit instruction) with AH, CH, DH, BH, and XGETBV as a named stop.
//!
//! Method as in tests/sse_int2.rs and tests/int_more2.rs: the host CPU runs the real machine code of the instruction (an
//! mmap'ed stub, not intrinsics) and the lifted C (clang -O0) runs the same case; xmm0-15, nine GPRs (rax rcx rdx rbx rsi
//! rdi r8 r9 r10), 64 bytes of memory and the six status flags must match. Flags the SDM leaves undefined are
//! `FLE_UNDEF` in C: they are not compared, but the set of undefined flags must be inside (and for TEST/AND exactly) the
//! SDM's set, so "not compared" cannot hide a defined flag. Cases travel in binary files (no 100 MB C source).
//! Outputs go under lift/out-a2a/test.

#![allow(dead_code)]

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

/// C register numbers of the state slots: rax rcx rdx rbx rsi rdi r8 r9 r10.
const GREG: [usize; 9] = [0, 1, 2, 3, 6, 7, 8, 9, 10];
/// EFLAGS bits of cf pf af zf sf of.
const FLAG_BITS: [u64; 6] = [0x1, 0x4, 0x10, 0x40, 0x80, 0x800];
const HOST_MASK: u64 = 0x8d5;
const UNDEF: u8 = 0xff;
const CF: u8 = 1;
const PF: u8 = 2;
const AF: u8 = 4;
const ZF: u8 = 8;
const SF: u8 = 16;
const OF: u8 = 32;
const ALL: u8 = 63;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Plain,
    /// The count is in CL, limited to 0..=8 (modulo 32), because 8-bit shifts with a masked count above 8 are an
    /// undefined result that the lifted C stops on.
    ShiftCl,
    /// DIV r/m8 with the divisor in the high byte of GPR slot `n`; the dividend AX is made to fit the quotient.
    DivU(usize),
    DivS(usize),
}

#[derive(Clone)]
struct Vector {
    name: String,
    bytes: Vec<u8>,
    /// rdi is the address of the 64-byte buffer + 16 (both sides).
    mem: bool,
    /// Flags C may leave undefined, and flags it must leave undefined (0 = no requirement).
    allowed: u8,
    required: u8,
    kind: Kind,
}

fn named(name: String, bytes: Vec<u8>, allowed: u8, required: u8) -> Vector {
    Vector { name, bytes, mem: false, allowed, required, kind: Kind::Plain }
}

impl Vector {
    fn mem(mut self) -> Self {
        self.mem = true;
        self
    }
    fn kind(mut self, k: Kind) -> Self {
        self.kind = k;
        self
    }
}

#[derive(Clone)]
struct Row {
    v: u32,
    flags: u32,
    gpr: [u64; 9],
    xmm: [[u64; 2]; 16],
    mem: [u8; 64],
}

impl Row {
    fn bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(400);
        b.extend_from_slice(&self.v.to_le_bytes());
        b.extend_from_slice(&self.flags.to_le_bytes());
        for g in self.gpr {
            b.extend_from_slice(&g.to_le_bytes());
        }
        for x in self.xmm {
            b.extend_from_slice(&x[0].to_le_bytes());
            b.extend_from_slice(&x[1].to_le_bytes());
        }
        b.extend_from_slice(&self.mem);
        b
    }
}

#[derive(Debug, PartialEq)]
struct Out {
    xmm: [[u64; 2]; 16],
    gpr: [u64; 9],
    /// C: 0, 1 or UNDEF. Host: 0 or 1.
    fl: [u8; 6],
    mem: [u8; 64],
}

#[repr(C, align(16))]
struct State {
    mem: [u8; 64],
    gpr: [u64; 9],
    flags_in: u64,
    flags_out: u64,
    xmm: [[u64; 2]; 16],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_FIN: usize = std::mem::offset_of!(State, flags_in);
const OFF_FOUT: usize = std::mem::offset_of!(State, flags_out);
const OFF_XMM: usize = std::mem::offset_of!(State, xmm);

/// Host stub: state pointer in rdi. Loads xmm, GPRs and flags, runs `body`, saves flags, GPRs and xmm. Keeps rbx and rbp.
fn stub(body: &[u8]) -> Vec<u8> {
    let d = |o: usize| (o as i32).to_le_bytes();
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xfd]; // push rbp; push rbx; mov rbp, rdi
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x6f, 0x80 | (((k & 7) as u8) << 3) | 5]); // movdqu xmm_k, [rbp+disp32]
        c.extend_from_slice(&d(OFF_XMM + 16 * k));
    }
    // (REX, modrm) of mov r64, [rbp+disp32] / mov [rbp+disp32], r64 for rax rcx rdx rbx rsi rdi r8 r9 r10
    const RM: [(u8, u8); 9] =
        [(0x48, 0x85), (0x48, 0x8d), (0x48, 0x95), (0x48, 0x9d), (0x48, 0xb5), (0x48, 0xbd), (0x4c, 0x85), (0x4c, 0x8d), (0x4c, 0x95)];
    for (k, (rex, m)) in RM.iter().enumerate() {
        c.extend_from_slice(&[*rex, 0x8b, *m]);
        c.extend_from_slice(&d(OFF_GPR + 8 * k));
    }
    c.extend_from_slice(&[0xff, 0xb5]); // push [rbp+flags_in]
    c.extend_from_slice(&d(OFF_FIN));
    c.push(0x9d); // popfq
    c.extend_from_slice(body);
    c.push(0x9c); // pushfq
    c.extend_from_slice(&[0x8f, 0x85]); // pop [rbp+flags_out]
    c.extend_from_slice(&d(OFF_FOUT));
    for (k, (rex, m)) in RM.iter().enumerate() {
        c.extend_from_slice(&[*rex, 0x89, *m]);
        c.extend_from_slice(&d(OFF_GPR + 8 * k));
    }
    for k in 0..16usize {
        c.push(0xf3);
        if k >= 8 {
            c.push(0x44);
        }
        c.extend_from_slice(&[0x0f, 0x7f, 0x80 | (((k & 7) as u8) << 3) | 5]); // movdqu [rbp+disp32], xmm_k
        c.extend_from_slice(&d(OFF_XMM + 16 * k));
    }
    c.extend_from_slice(&[0x5b, 0x5d, 0xc3]); // pop rbx; pop rbp; ret
    c
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

fn eflags(f: u32) -> u64 {
    FLAG_BITS.iter().enumerate().fold(0, |a, (i, b)| if (f >> i) & 1 == 1 { a | b } else { a })
}

fn host_run(vs: &[Vector], rows: &[Row]) -> Vec<Out> {
    let mut res = Vec::with_capacity(rows.len());
    unsafe {
        let codes: Vec<*mut u8> = vs
            .iter()
            .map(|v| {
                let code = stub(&v.bytes);
                assert!(code.len() < 4096);
                let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
                assert!(!p.is_null() && p as isize != -1, "mmap failed");
                std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
                p
            })
            .collect();
        for row in rows {
            let v = &vs[row.v as usize];
            let mut st = Box::new(State { mem: row.mem, gpr: row.gpr, flags_in: eflags(row.flags) | 0x202, flags_out: 0, xmm: row.xmm });
            if v.mem {
                st.gpr[5] = st.mem.as_ptr() as u64 + 16;
            }
            let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut State)>(codes[row.v as usize]);
            f(&mut *st);
            let mut fl = [0u8; 6];
            for (i, b) in FLAG_BITS.iter().enumerate() {
                fl[i] = ((st.flags_out & HOST_MASK) & b != 0) as u8;
            }
            res.push(Out { xmm: st.xmm, gpr: st.gpr, fl, mem: st.mem });
        }
        for p in codes {
            munmap(p, 4096);
        }
    }
    res
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let dir = crate_dir().join("../../out-a2a/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

fn runtime_dir() -> PathBuf {
    crate_dir().join("../../runtime")
}

/// Compiles `src` (and the runtime .c files in `extra`) with clang -O0 under out-a2a/test; returns the executable.
fn compile(name: &str, src: &str, extra: &[&str]) -> PathBuf {
    let dir = out_dir();
    let rt = runtime_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).expect("write C source");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-ffp-contract=off", "-w", "-I"]).arg(&rt);
    for e in extra {
        cmd.arg(rt.join(e));
    }
    cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp"));
    let cc = cmd.output().expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

fn c_run(group: &str, vs: &[Vector], rows: &[Row]) -> Vec<Out> {
    let mut src = String::from(HEADER);
    for (k, v) in vs.iter().enumerate() {
        let body = lift_body(&v.bytes, 0x1000).unwrap_or_else(|e| panic!("lift {}: {e}", v.name));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n\n");
    }
    src += "static void (*const FN[])(void) = {";
    src += &(0..vs.len()).map(|k| format!("c{k}")).collect::<Vec<_>>().join(",");
    src += "};\nstatic const uint8_t MEMV[] = {";
    src += &vs.iter().map(|v| (v.mem as u8).to_string()).collect::<Vec<_>>().join(",");
    src += "};\n";
    src += r#"struct In { uint32_t v; uint32_t flags; uint64_t gpr[9]; uint64_t xmm[16][2]; uint8_t mem[64]; };
struct Out { uint64_t xmm[16][2]; uint64_t gpr[9]; uint8_t fl[8]; uint8_t mem[64]; };
static const int GR[9] = {0, 1, 2, 3, 6, 7, 8, 9, 10};

int main(int argc, char **argv)
{
    FILE *fi = fopen(argv[1], "rb"), *fo = fopen(argv[2], "wb");
    struct In in;
    struct Out out;
    int j;
    if (!fi || !fo) return 2;
    while (fread(&in, sizeof in, 1, fi) == 1) {
        for (j = 0; j < 16; j++) { X[j].u64[0] = in.xmm[j][0]; X[j].u64[1] = in.xmm[j][1]; }
        for (j = 0; j < 9; j++) R[GR[j]] = in.gpr[j];
        memcpy(M, in.mem, 64);
        if (MEMV[in.v]) R[7] = (uint64_t)(uintptr_t)(M + 16);
        FL.cf = !!(in.flags & 1); FL.pf = !!(in.flags & 2); FL.af = !!(in.flags & 4);
        FL.zf = !!(in.flags & 8); FL.sf = !!(in.flags & 16); FL.of = !!(in.flags & 32);
        FN[in.v]();
        for (j = 0; j < 16; j++) { out.xmm[j][0] = X[j].u64[0]; out.xmm[j][1] = X[j].u64[1]; }
        for (j = 0; j < 9; j++) out.gpr[j] = R[GR[j]];
        out.fl[0] = FL.cf; out.fl[1] = FL.pf; out.fl[2] = FL.af; out.fl[3] = FL.zf; out.fl[4] = FL.sf; out.fl[5] = FL.of;
        out.fl[6] = out.fl[7] = 0;
        memcpy(out.mem, M, 64);
        fwrite(&out, sizeof out, 1, fo);
    }
    fclose(fo);
    return 0;
}
"#;
    let exe = compile(&format!("a2a_{group}"), &src, &["fle_int_ext.c", "fle_int_more.c"]);
    let dir = out_dir();
    let inp = dir.join(format!("{}.in", etxt_common::uniq(group)));
    let outp = dir.join(format!("{}.out", etxt_common::uniq(group)));
    let mut data = Vec::with_capacity(rows.len() * 400);
    for r in rows {
        data.extend(r.bytes());
    }
    std::fs::write(&inp, data).expect("write rows");
    let run = Command::new(&exe).arg(&inp).arg(&outp).output().expect("run test binary");
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let b = std::fs::read(&outp).expect("read results");
    assert_eq!(b.len(), rows.len() * 400, "{group}: C result size");
    let u64_at = |c: &[u8], o: usize| u64::from_le_bytes(c[o..o + 8].try_into().unwrap());
    b.chunks(400)
        .map(|c| {
            let mut xmm = [[0u64; 2]; 16];
            for (j, x) in xmm.iter_mut().enumerate() {
                x[0] = u64_at(c, 16 * j);
                x[1] = u64_at(c, 16 * j + 8);
            }
            let mut gpr = [0u64; 9];
            for (j, g) in gpr.iter_mut().enumerate() {
                *g = u64_at(c, 256 + 8 * j);
            }
            let mut fl = [0u8; 6];
            fl.copy_from_slice(&c[328..334]);
            let mut mem = [0u8; 64];
            mem.copy_from_slice(&c[336..400]);
            Out { xmm, gpr, fl, mem }
        })
        .collect()
}

#[derive(Default)]
struct Summary {
    vectors: usize,
    rows: usize,
    failures: usize,
    flag_cmp: usize,
    undef_flags: usize,
    host_changed: usize,
    host_flag_seen: [[bool; 2]; 6],
}

/// Runs every row on the host and as C and compares. Returns the summary; the caller asserts on it.
fn check(group: &str, vs: &[Vector], rows: &[Row]) -> Summary {
    assert!(!rows.is_empty() && !vs.is_empty(), "{group}: no cases");
    let c = c_run(group, vs, rows);
    let h = host_run(vs, rows);
    assert_eq!(c.len(), rows.len());
    let mut s = Summary { vectors: vs.len(), rows: rows.len(), ..Default::default() };
    for (i, row) in rows.iter().enumerate() {
        let v = &vs[row.v as usize];
        let (co, ho) = (&c[i], &h[i]);
        let mut bad = Vec::new();
        if co.xmm != ho.xmm {
            bad.push("xmm".to_string());
        }
        for j in 0..9 {
            if v.mem && j == 5 {
                continue;
            }
            if co.gpr[j] != ho.gpr[j] {
                bad.push(format!("gpr[{j}] C {:#x} host {:#x}", co.gpr[j], ho.gpr[j]));
            }
        }
        if co.mem != ho.mem {
            bad.push("mem".to_string());
        }
        let mut undef = 0u8;
        for j in 0..6 {
            if co.fl[j] == UNDEF {
                undef |= 1 << j;
                s.undef_flags += 1;
            } else {
                s.flag_cmp += 1;
                if co.fl[j] != ho.fl[j] {
                    bad.push(format!("flag {j} (cf pf af zf sf of) C {} host {}", co.fl[j], ho.fl[j]));
                }
            }
            s.host_flag_seen[j][ho.fl[j] as usize] = true;
        }
        if undef & !v.allowed != 0 {
            bad.push(format!("undefined flags {undef:#x} outside the allowed set {:#x}", v.allowed));
        }
        if v.required != 0 && undef != v.required {
            bad.push(format!("undefined flags {undef:#x}, required exactly {:#x}", v.required));
        }
        if ho.xmm != row.xmm || ho.gpr != row.gpr || ho.mem != row.mem {
            s.host_changed += 1;
        }
        if !bad.is_empty() {
            s.failures += 1;
            if s.failures <= 8 {
                eprintln!(
                    "{group}: {} row {i}: {}\n  bytes {:02x?}\n  in gpr {:x?} flags {:#x}\n  C    gpr {:x?} fl {:?}\n  host gpr {:x?} fl {:?}",
                    v.name, bad.join("; "), v.bytes, row.gpr, row.flags, co.gpr, co.fl, ho.gpr, ho.fl
                );
            }
        }
    }
    eprintln!(
        "{group}: vectors {} rows {} failures {} flag_comparisons {} undefined_flags {} host_changed_state {}",
        s.vectors, s.rows, s.failures, s.flag_cmp, s.undef_flags, s.host_changed
    );
    let dir = out_dir();
    let _ = std::fs::write(
        dir.join(format!("summary-{group}.txt")),
        format!(
            "group {group}\nvectors {}\nrows {}\nfailures {}\nflag comparisons {}\nflags left undefined by C {}\nrows where the host changed state {}\n",
            s.vectors, s.rows, s.failures, s.flag_cmp, s.undef_flags, s.host_changed
        ),
    );
    assert_eq!(s.failures, 0, "{group}: {} of {} rows differ from the host", s.failures, s.rows);
    s
}

// ---------------------------------------------------------------------------------------------------------------
// Row generation

const BYTE_POOL: [u8; 11] = [0, 1, 0x7f, 0x80, 0xff, 0x0f, 0xf0, 0x55, 0xaa, 0xfe, 0x81];

/// A 64-bit value whose bytes come from the edge pool half the time, so that AH and AL differ and hit the edges.
fn rnd_gpr(r: &mut Rng) -> u64 {
    if r.below(4) == 0 {
        return r.next();
    }
    let mut b = [0u8; 8];
    for x in b.iter_mut() {
        *x = if r.below(2) == 0 { BYTE_POOL[r.below(11) as usize] } else { r.next() as u8 };
    }
    u64::from_le_bytes(b)
}

fn base_row(v: usize, r: &mut Rng) -> Row {
    let mut xmm = [[0u64; 2]; 16];
    for x in xmm.iter_mut() {
        x[0] = r.next();
        x[1] = r.next();
    }
    let mut gpr = [0u64; 9];
    for g in gpr.iter_mut() {
        *g = rnd_gpr(r);
    }
    let mut mem = [0u8; 64];
    for b in mem.iter_mut() {
        *b = if r.below(2) == 0 { BYTE_POOL[r.below(11) as usize] } else { r.next() as u8 };
    }
    Row { v: v as u32, flags: r.below(64) as u32, gpr, xmm, mem }
}

fn int_row(vs: &[Vector], v: usize, r: &mut Rng) -> Row {
    let mut row = base_row(v, r);
    match vs[v].kind {
        Kind::Plain => {}
        Kind::ShiftCl => {
            let cnt = r.below(9);
            row.gpr[1] = (row.gpr[1] & !0xff) | cnt | (r.below(8) << 5);
        }
        Kind::DivU(slot) => {
            let b = 1 + r.below(255);
            row.gpr[slot] = (row.gpr[slot] & !0xff00) | (b << 8);
            let ax = (r.below(b) << 8) | (r.next() & 0xff);
            row.gpr[0] = (row.gpr[0] & !0xffff) | ax;
        }
        Kind::DivS(slot) => {
            let m = 1 + r.below(127);
            let b = if r.below(2) == 0 { m } else { 0u64.wrapping_sub(m) & 0xff };
            row.gpr[slot] = (row.gpr[slot] & !0xff00) | (b << 8);
            let range = 127 * m as i64;
            let ax = (r.below(2 * range as u64 + 1) as i64 - range) as i16 as u16 as u64;
            row.gpr[0] = (row.gpr[0] & !0xffff) | ax;
        }
    }
    row
}

fn int_rows(vs: &[Vector], per_vector: usize, seed: u64) -> Vec<Row> {
    let mut r = Rng(seed);
    let mut rows = Vec::with_capacity(vs.len() * per_vector);
    for v in 0..vs.len() {
        for _ in 0..per_vector {
            rows.push(int_row(vs, v, &mut r));
        }
    }
    rows
}

/// [prefix] [REX] 0F opc modrm [disp8] for xmm forms. `reg` is modrm.reg.
#[derive(Clone, Copy)]
enum Rm {
    X(u8),
    Mem(i8),
}

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

/// The seven forms of a two-operand xmm instruction: reg-reg (0,1), (9,14), same register (3,3), memory [rdi+disp].
/// Returns the vectors and, per vector, (dest xmm, source xmm or None, disp).
fn xmm_forms(name: &str, prefix: Option<u8>, opc: &[u8]) -> (Vec<Vector>, Vec<(u8, Option<u8>, i8)>) {
    let mut vs = Vec::new();
    let mut shape = Vec::new();
    for (d, s) in [(0u8, 1u8), (9, 14), (3, 3)] {
        vs.push(named(format!("{name} x{d}, x{s}"), enc(prefix, opc, d, Rm::X(s)), 0, 0));
        shape.push((d, Some(s), 0));
    }
    for disp in [-16i8, 0, 16, 32] {
        vs.push(named(format!("{name} x2, [rdi{disp:+}]"), enc(prefix, opc, 2, Rm::Mem(disp)), 0, 0).mem());
        shape.push((2, None, disp));
    }
    (vs, shape)
}

fn put128(m: &mut [u8; 64], off: usize, v: [u64; 2]) {
    m[off..off + 8].copy_from_slice(&v[0].to_le_bytes());
    m[off + 8..off + 16].copy_from_slice(&v[1].to_le_bytes());
}

fn pool_lanes(bits: u32) -> Vec<u64> {
    let m = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
    let top = 1u64 << (bits - 1);
    let mut v = vec![0, 1, 2, top - 1, top, top + 1, m, m - 1, top >> 1, (m >> 4) << 4, 0x55 & m, 0xaa & m];
    if bits >= 16 {
        v.extend([0x7f, 0x80, 0xff, 0x100, 0x0101, 0xff00, 0x7f7f, 0x8080].map(|x| x & m));
    }
    v.sort_unstable();
    v.dedup();
    v
}

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

fn fill_all(x: &mut [u64; 2], bits: u32, pool: &[u64], r: &mut Rng) {
    for lane in 0..(128 / bits as usize) {
        let v = if r.below(2) == 0 { pool[r.below(pool.len() as u64) as usize] } else { r.next() };
        fill_lane(x, lane, v, bits);
    }
}

/// (destination, source) value pairs: random lanes, then all combinations of edge values with lane rotation.
fn pair_set(bits: u32, pool: &[u64], random: usize, seed: u64) -> Vec<([u64; 2], [u64; 2])> {
    let mut r = Rng(seed);
    let mut out = Vec::new();
    for _ in 0..random {
        let (mut a, mut b) = ([0u64; 2], [0u64; 2]);
        fill_all(&mut a, bits, pool, &mut r);
        fill_all(&mut b, bits, pool, &mut r);
        out.push((a, b));
    }
    let lanes = 128 / bits as usize;
    for a in 0..pool.len() {
        for b in 0..pool.len() {
            let (mut x, mut y) = ([0u64; 2], [0u64; 2]);
            for lane in 0..lanes {
                fill_lane(&mut x, lane, pool[(a + lane) % pool.len()], bits);
                fill_lane(&mut y, lane, pool[(b + lane * 3) % pool.len()], bits);
            }
            out.push((x, y));
        }
    }
    out
}

fn xmm_rows(vs: &[Vector], shape: &[(u8, Option<u8>, i8)], pairs: &[([u64; 2], [u64; 2])], seed: u64) -> Vec<Row> {
    let mut r = Rng(seed);
    let mut rows = Vec::new();
    for (v, &(d, s, disp)) in shape.iter().enumerate() {
        assert!(v < vs.len());
        for &(a, b) in pairs {
            let mut row = base_row(v, &mut r);
            row.xmm[d as usize] = a;
            match s {
                Some(s) if s != d => row.xmm[s as usize] = b,
                Some(_) => {}
                None => put128(&mut row.mem, (16 + disp as i32) as usize, b),
            }
            rows.push(row);
        }
    }
    rows
}

fn xmm_family(name: &str, prefix: Option<u8>, opc: &[u8], bits: u32, random: usize, seed: u64) -> Summary {
    let (vs, shape) = xmm_forms(name, prefix, opc);
    let pairs = pair_set(bits, &pool_lanes(bits), random, seed);
    let rows = xmm_rows(&vs, &shape, &pairs, seed ^ 0x5a5a);
    let s = check(name, &vs, &rows);
    assert!(s.host_changed * 10 >= rows.len() * 8, "{name}: the host changed state in only {} of {} rows", s.host_changed, rows.len());
    assert_eq!(s.undef_flags, 0, "{name}: flags are unchanged by this instruction");
    s
}

#[test]
fn psubw_matches_host() {
    xmm_family("psubw", Some(0x66), &[0x0f, 0xf9], 16, 300, 0xa001);
}

#[test]
fn pcmpgtd_matches_host() {
    xmm_family("pcmpgtd", Some(0x66), &[0x0f, 0x66], 32, 300, 0xa002);
}

#[test]
fn andnpd_matches_host() {
    xmm_family("andnpd", Some(0x66), &[0x0f, 0x55], 64, 300, 0xa003);
}

/// Float edge patterns for RCPPS lanes: zeros, denormals, the exponent boundaries where the result becomes tiny
/// (source exponent field 253, 254), the smallest normals, infinities, SNaN and QNaN of both signs.
fn rcp_pool() -> Vec<u64> {
    [
        0x0000_0000u32, 0x8000_0000, 0x0000_0001, 0x007f_ffff, 0x8000_0001, 0x8040_0000, 0x0040_0000, 0x0080_0000, 0x8080_0000, 0x0080_0001,
        0x7e00_0000, 0x7e7f_ffff, 0x7e80_0000, 0x7e80_0001, 0x7eff_ffff, 0x7f00_0000, 0x7f00_0001, 0x7f7f_ffff, 0xff7f_ffff, 0xfe80_0000,
        0xff00_0000, 0x7f80_0000, 0xff80_0000, 0x7f80_0001, 0x7fc0_0000, 0xffc0_0001, 0xff80_0001, 0x7fff_ffff, 0xffff_ffff, 0x3f80_0000,
        0xbf80_0000, 0x3fff_ffff, 0x4000_0000, 0x3f7f_ffff, 0x3f80_0001, 0x4b00_0000, 0x1234_5678, 0xabcd_ef01,
    ]
    .iter()
    .map(|&x| x as u64)
    .collect()
}

fn rcp_pairs(seed: u64) -> Vec<([u64; 2], [u64; 2])> {
    let mut r = Rng(seed);
    let mut out = Vec::new();
    let pool = rcp_pool();
    let lane4 = |l: [u32; 4]| [l[0] as u64 | (l[1] as u64) << 32, l[2] as u64 | (l[3] as u64) << 32];
    // every edge value in every lane position
    for start in 0..pool.len() {
        let l = [0, 1, 2, 3].map(|j| pool[(start + j * 7) % pool.len()] as u32);
        out.push(([r.next(), r.next()], lane4(l)));
    }
    // all 4096 mantissa indices, random sign, exponent 1..=252 and random low 11 bits
    for idx in (0..4096u32).step_by(4) {
        let l = [0, 1, 2, 3].map(|j| {
            let e = 1 + r.below(252) as u32;
            ((r.below(2) as u32) << 31) | (e << 23) | ((idx + j) << 11) | r.below(2048) as u32
        });
        out.push(([r.next(), r.next()], lane4(l)));
    }
    // every exponent field value 0..=255 with random mantissa and sign
    for e in 0..256u32 {
        let l = [0, 1, 2, 3].map(|_| ((r.below(2) as u32) << 31) | (e << 23) | r.below(1 << 23) as u32);
        out.push(([r.next(), r.next()], lane4(l)));
    }
    for _ in 0..300 {
        out.push(([r.next(), r.next()], [r.next(), r.next()]));
    }
    out
}

#[test]
fn rcpps_matches_host() {
    let (vs, shape) = xmm_forms("rcpps", None, &[0x0f, 0x53]);
    let pairs = rcp_pairs(0xa004);
    let rows = xmm_rows(&vs, &shape, &pairs, 0xa004 ^ 0x5a5a);
    let s = check("rcpps", &vs, &rows);
    assert!(s.host_changed * 10 >= rows.len() * 8);
    assert_eq!(s.undef_flags, 0);
}

/// All 2^32 float bit patterns, the lifted helper (fle_i2_rcpps) against the host RCPPS. About 10 seconds.
#[test]
fn rcpps_all_2e32_inputs_match_host() {
    let src = std::fs::read_to_string(crate_dir().join("tests/rcp/rcp_exhaustive.c")).expect("read rcp_exhaustive.c");
    // -O0 is slow for 2^32 inputs: build with -O2.
    let dir = out_dir();
    let exe2 = dir.join(etxt_common::uniq("rcp_exhaustive_o2"));
    let c = dir.join(format!("{}.c", etxt_common::uniq("rcp_exhaustive_src")));
    std::fs::write(&c, &src).unwrap();
    let cc = Command::new("clang")
        .args(["-O2", "-std=gnu11", "-w", "-I"])
        .arg(runtime_dir())
        .arg("-o")
        .arg(&exe2)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe2).output().expect("run");
    let text = String::from_utf8_lossy(&run.stdout).to_string();
    eprintln!("{text}");
    assert!(run.status.success(), "mismatches against the host: {text}");
    assert!(text.contains("rcpps_exhaustive inputs 4294967296 mismatches 0"), "{text}");
}

/// The table in the header is the host's: regenerating it from the host (as tests/rcp/gen_rcp_table.py does) gives the same file.
#[test]
fn rcp_table_header_matches_host_dump() {
    let dir = out_dir();
    let exe = dir.join(etxt_common::uniq("host_dump"));
    let cc = Command::new("clang")
        .args(["-O1", "-w", "-o"])
        .arg(&exe)
        .arg(crate_dir().join("tests/rcp/host_dump.c"))
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run host_dump");
    assert!(run.status.success());
    let host: Vec<u32> = String::from_utf8(run.stdout).unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
    assert_eq!(host.len(), 4096);
    let hdr = std::fs::read_to_string(runtime_dir().join("fle_rcp_table.h")).expect("read table header");
    let body = hdr.split("fle_rcp_tab[4096] = {").nth(1).expect("table start").split("};").next().unwrap();
    let tab: Vec<u32> = body.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse().unwrap()).collect();
    assert_eq!(tab, host, "runtime/fle_rcp_table.h differs from the host RCPPS table");
}

// ---------------------------------------------------------------------------------------------------------------
// BSWAP

#[test]
fn bswap_matches_host() {
    let nums = [0u8, 1, 2, 3, 6, 7, 8, 9, 10];
    let mut vs = Vec::new();
    for (slot, n) in nums.iter().enumerate() {
        let _ = slot;
        let rexb = if *n >= 8 { 1u8 } else { 0 };
        let mut b32 = Vec::new();
        if rexb == 1 {
            b32.push(0x41);
        }
        b32.extend_from_slice(&[0x0f, 0xc8 + (n & 7)]);
        vs.push(named(format!("bswap r{n}d"), b32, 0, 0));
        vs.push(named(format!("bswap r{n}"), vec![0x48 | rexb, 0x0f, 0xc8 + (n & 7)], 0, 0));
    }
    let mut r = Rng(0xb001);
    let mut rows = Vec::new();
    let fixed: [u64; 8] = [0, u64::MAX, 0x0102_0304_0506_0708, 0xffff_ffff_0000_0001, 0x1_0000_0000, 0x8000_0000, 0x0000_0000_8000_0000, 0xdead_beef_cafe_f00d];
    for v in 0..vs.len() {
        for k in 0..80 {
            let mut row = base_row(v, &mut r);
            let slot = v / 2;
            if k < fixed.len() {
                row.gpr[slot] = fixed[k];
            }
            rows.push(row);
        }
    }
    let s = check("bswap", &vs, &rows);
    assert_eq!(s.undef_flags, 0);
    assert!(s.host_changed * 10 >= rows.len() * 8);
    for j in 0..6 {
        assert!(s.host_flag_seen[j][0] && s.host_flag_seen[j][1], "flag {j}: inputs did not cover both values");
    }
}

#[test]
fn bswap_16bit_and_other_forms_stay_unsupported() {
    // The SDM: "When the BSWAP instruction references a 16-bit register, the result is undefined."
    assert!(lift_body(&[0x66, 0x0f, 0xc8], 0x1000).is_err(), "bswap ax");
    // MMX forms of the SSE integer instructions are not part of this unit.
    assert!(lift_body(&[0x0f, 0xf9, 0xc1], 0x1000).is_err(), "psubw mm0, mm1");
    assert!(lift_body(&[0x0f, 0x66, 0xc1], 0x1000).is_err(), "pcmpgtd mm0, mm1");
    // VEX forms are separate mnemonics.
    assert!(lift_body(&[0xc5, 0xf1, 0x55, 0xc2], 0x1000).is_err(), "vandnpd xmm0, xmm1, xmm2");
    assert!(lift_body(&[0xc5, 0xf8, 0x53, 0xc1], 0x1000).is_err(), "vrcpps xmm0, xmm1");
}

// ---------------------------------------------------------------------------------------------------------------
// High-byte registers

const R8: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
const IMMS: [u8; 9] = [0x00, 0x01, 0x02, 0x04, 0x41, 0x7f, 0x80, 0xe0, 0xff];
const ALU: [(&str, u8, u8); 8] =
    [("add", 0x00, 0), ("or", 0x08, 1), ("adc", 0x10, 2), ("sbb", 0x18, 3), ("and", 0x20, 4), ("sub", 0x28, 5), ("xor", 0x30, 6), ("cmp", 0x38, 7)];

/// An 8-bit ALU instruction (or TEST) in every form with every pair of the eight 8-bit registers, immediates, and
/// memory operands [rdi+1].
fn alu8(name: &str, op_mr: u8, ext: u8, test: bool, allowed: u8, required: u8) -> Vec<Vector> {
    let mut v = Vec::new();
    for reg in 0..8u8 {
        for rm in 0..8u8 {
            v.push(named(format!("{name} {}, {}", R8[rm as usize], R8[reg as usize]), vec![op_mr, 0xc0 | (reg << 3) | rm], allowed, required));
            if !test {
                v.push(named(format!("{name} {}, {}", R8[reg as usize], R8[rm as usize]), vec![op_mr + 2, 0xc0 | (reg << 3) | rm], allowed, required));
            }
        }
    }
    for rm in 0..8u8 {
        for imm in IMMS {
            let b = if test { vec![0xf6, 0xc0 | rm, imm] } else { vec![0x80, 0xc0 | (ext << 3) | rm, imm] };
            v.push(named(format!("{name} {}, {imm:#x}", R8[rm as usize]), b, allowed, required));
        }
    }
    for reg in 0..8u8 {
        let m = 0x40 | (reg << 3) | 7;
        v.push(named(format!("{name} [rdi+1], {}", R8[reg as usize]), vec![op_mr, m, 1], allowed, required).mem());
        if !test {
            v.push(named(format!("{name} {}, [rdi+1]", R8[reg as usize]), vec![op_mr + 2, m, 1], allowed, required).mem());
        }
    }
    for imm in IMMS {
        let b = if test { vec![0xf6, 0x47, 0x01, imm] } else { vec![0x80, 0x40 | (ext << 3) | 7, 0x01, imm] };
        v.push(named(format!("{name} [rdi+1], {imm:#x}"), b, allowed, required).mem());
    }
    v
}

fn target_vectors() -> Vec<Vector> {
    let mut v = alu8("test", 0x84, 0, true, AF, AF);
    v.extend(alu8("and", 0x20, 4, false, AF, AF));
    v
}

fn audit_vectors() -> Vec<Vector> {
    let mut v = Vec::new();
    for (name, op, ext) in ALU {
        match name {
            "and" => {}
            "or" | "xor" => v.extend(alu8(name, op, ext, false, AF, AF)),
            _ => v.extend(alu8(name, op, ext, false, 0, 0)),
        }
    }
    for reg in 0..8u8 {
        let r = R8[reg as usize];
        let rr = 0xc0 | reg;
        v.push(named(format!("inc {r}"), vec![0xfe, rr], 0, 0));
        v.push(named(format!("dec {r}"), vec![0xfe, 0xc8 | reg], 0, 0));
        v.push(named(format!("not {r}"), vec![0xf6, 0xd0 | reg], 0, 0));
        v.push(named(format!("neg {r}"), vec![0xf6, 0xd8 | reg], 0, 0));
        v.push(named(format!("mul {r}"), vec![0xf6, 0xe0 | reg], PF | AF | SF | ZF, 0));
        v.push(named(format!("imul {r}"), vec![0xf6, 0xe8 | reg], PF | AF | SF | ZF, 0));
        for (nm, ext) in [("rol", 0u8), ("ror", 1), ("shl", 4), ("shr", 5), ("sar", 7)] {
            let allowed = if ext < 2 { OF } else { AF | OF };
            let m = 0xc0 | (ext << 3) | reg;
            v.push(named(format!("{nm} {r}, 1"), vec![0xd0, m], allowed, 0));
            v.push(named(format!("{nm} {r}, cl"), vec![0xd2, m], allowed, 0).kind(Kind::ShiftCl));
            for imm in [0u8, 1, 2, 3, 7, 8, 0x21, 0x28] {
                v.push(named(format!("{nm} {r}, {imm:#x}"), vec![0xc0, m, imm], allowed, 0));
            }
        }
        // moves and extensions
        for src in 0..8u8 {
            v.push(named(format!("mov {r}, {}", R8[src as usize]), vec![0x88, 0xc0 | (src << 3) | reg], 0, 0));
            v.push(named(format!("xchg {r}, {}", R8[src as usize]), vec![0x86, 0xc0 | (src << 3) | reg], 0, 0));
            v.push(named(format!("xadd {r}, {}", R8[src as usize]), vec![0x0f, 0xc0, 0xc0 | (src << 3) | reg], 0, 0));
            v.push(named(format!("cmpxchg {r}, {}", R8[src as usize]), vec![0x0f, 0xb0, 0xc0 | (src << 3) | reg], 0, 0));
        }
        v.push(named(format!("mov {r}, imm"), vec![0xb0 + reg, 0x9c], 0, 0));
        for d in 0..4u8 {
            v.push(named(format!("movzx e{}x, {r}", (b'a' + [0, 2, 3, 1][d as usize]) as char), vec![0x0f, 0xb6, 0xc0 | (d << 3) | reg], 0, 0));
            v.push(named(format!("movsx e{}x, {r}", (b'a' + [0, 2, 3, 1][d as usize]) as char), vec![0x0f, 0xbe, 0xc0 | (d << 3) | reg], 0, 0));
        }
        for cc in 0..16u8 {
            v.push(named(format!("set{cc:x} {r}"), vec![0x0f, 0x90 + cc, rr], 0, 0));
        }
        // memory forms of the unary and exchange group
        v.push(named(format!("xchg [rdi+1], {r}"), vec![0x86, 0x40 | (reg << 3) | 7, 1], 0, 0).mem());
        v.push(named(format!("xadd [rdi+1], {r}"), vec![0x0f, 0xc0, 0x40 | (reg << 3) | 7, 1], 0, 0).mem());
        v.push(named(format!("cmpxchg [rdi+1], {r}"), vec![0x0f, 0xb0, 0x40 | (reg << 3) | 7, 1], 0, 0).mem());
        v.push(named(format!("mov [rdi+1], {r}"), vec![0x88, 0x40 | (reg << 3) | 7, 1], 0, 0).mem());
        v.push(named(format!("mov {r}, [rdi+1]"), vec![0x8a, 0x40 | (reg << 3) | 7, 1], 0, 0).mem());
    }
    // DIV and IDIV with a divisor in BH, CH, DH (the dividend AX is constructed so that the quotient fits)
    for (rm, slot, nm) in [(7u8, 3usize, "bh"), (5, 1, "ch"), (6, 2, "dh")] {
        v.push(named(format!("div {nm}"), vec![0xf6, 0xf0 | rm], ALL, 0).kind(Kind::DivU(slot)));
        v.push(named(format!("idiv {nm}"), vec![0xf6, 0xf8 | rm], ALL, 0).kind(Kind::DivS(slot)));
    }
    v
}

#[test]
fn high_byte_test_and_match_host() {
    let vs = target_vectors();
    let rows = int_rows(&vs, 40, 0xc001);
    let s = check("high_byte_target", &vs, &rows);
    // CF and OF are always 0 for TEST and AND; PF, ZF and SF must have taken both values
    for j in [1usize, 3, 4] {
        assert!(s.host_flag_seen[j][0] && s.host_flag_seen[j][1], "flag {j}: results did not cover both values");
    }
    assert!(!s.host_flag_seen[0][1] && !s.host_flag_seen[5][1], "CF and OF must be 0 after TEST/AND");
    assert!(s.undef_flags >= rows.len(), "AF must be left undefined by every TEST/AND row");
    // the exact instructions of the all-pdata closure
    let sites: [(&str, &[u8]); 6] = [
        ("0x5e8f8b test ah,2", &[0xf6, 0xc4, 0x02]),
        ("0x5ea243 test bh,0xe0", &[0xf6, 0xc7, 0xe0]),
        ("0x5ea960 test bh,2", &[0xf6, 0xc7, 0x02]),
        ("0x5eabd8 test ah,4", &[0xf6, 0xc4, 0x04]),
        ("0x5eac8c test ah,2", &[0xf6, 0xc4, 0x02]),
        ("0x6a044d and ah,0x41", &[0x80, 0xe4, 0x41]),
    ];
    for (n, b) in sites {
        assert!(lift_body(b, 0x1000).is_ok(), "{n}");
    }
}

#[test]
fn high_byte_audit_of_all_8bit_forms_matches_host() {
    let vs = audit_vectors();
    let rows = int_rows(&vs, 14, 0xc002);
    let s = check("high_byte_audit", &vs, &rows);
    assert!(s.vectors > 1500, "vectors {}", s.vectors);
    assert!(s.flag_cmp > 50_000);
}

// ---------------------------------------------------------------------------------------------------------------
// XGETBV

/// XGETBV is a named stop: the lifted body calls fle_xgetbv_stop, which prints the VA and aborts.
#[test]
fn xgetbv_is_a_named_stop() {
    let body = lift_body(&[0x0f, 0x01, 0xd0], 0x639a03).expect("xgetbv lifts to a named stop");
    assert!(body.contains("fle_xgetbv_stop(UINT64_C(0x639a03))"), "{body}");
    let src = format!(
        "{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[1] = 0;\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n"
    );
    let exe = compile("a2a_xgetbv", &src, &["fle_cpuid.c"]);
    let run = Command::new(&exe).output().expect("run");
    assert!(!run.status.success(), "xgetbv must stop the run");
    let err = String::from_utf8_lossy(&run.stderr);
    assert!(err.contains("fle: xgetbv at 0x639a03: XCR0 is not defined"), "stderr {err}");
    assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"));
    // other forms of opcode 0F 01 stay unsupported (xsetbv, xend, ...)
    assert!(lift_body(&[0x0f, 0x01, 0xd1], 0x1000).is_err(), "xsetbv");
}

/// The precondition of "XGETBV is not reached": the guest's check `CPUID.1:ECX & 0x18000000 == 0x18000000` is false for
/// every CPUID input of the runtime answer.
#[test]
fn runtime_cpuid_never_reports_osxsave_and_avx() {
    let leaves = [0u64, 1, 2, 7, 0xd, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, 0xffff_ffff, 0x1_0000_0001];
    let mut src = String::from("#include \"fle_runtime.h\"\n#include \"fle_cpuid.h\"\n\nuint64_t R[16];\n\nint main(void)\n{\n");
    src += "    static const uint64_t L[] = {";
    src += &leaves.iter().map(|l| format!("{l:#x}ull")).collect::<Vec<_>>().join(",");
    src += "};\n    for (unsigned i = 0; i < sizeof L / sizeof L[0]; i++) for (unsigned c = 0; c < 4; c++) {\n";
    src += "        R[0] = L[i]; R[1] = c; fle_cpuid();\n";
    src += "        if (((uint32_t)R[1] & 0x18000000u) == 0x18000000u) { printf(\"reports OSXSAVE+AVX leaf %llx\\n\", (unsigned long long)L[i]); return 1; }\n";
    src += "        if ((uint32_t)R[1] & 0x04000000u) { printf(\"reports XSAVE leaf %llx\\n\", (unsigned long long)L[i]); return 1; }\n    }\n";
    src += "    printf(\"ok\\n\");\n    return 0;\n}\n";
    let exe = compile("a2a_cpuid_bits", &src, &["fle_cpuid.c"]);
    let run = Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stdout));
}

// ---------------------------------------------------------------------------------------------------------------
// Memory operand alignment (SDM Table 2-21, Type 4: #GP when not 16-byte aligned)

#[test]
fn misaligned_memory_operand_aborts() {
    for (name, bytes) in [
        ("psubw", enc(Some(0x66), &[0x0f, 0xf9], 0, Rm::Mem(8))),
        ("pcmpgtd", enc(Some(0x66), &[0x0f, 0x66], 3, Rm::Mem(1))),
        ("andnpd", enc(Some(0x66), &[0x0f, 0x55], 1, Rm::Mem(4))),
        ("rcpps", enc(None, &[0x0f, 0x53], 2, Rm::Mem(-1))),
    ] {
        let body = lift_body(&bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!(
            "{HEADER}static void c0(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"returned\\n\");\n    return 0;\n}}\n"
        );
        let exe = compile(&format!("a2a_misaligned_{name}"), &src, &[]);
        let run = Command::new(&exe).output().expect("run");
        assert!(!run.status.success(), "{name}: a misaligned operand must abort");
        assert!(String::from_utf8_lossy(&run.stderr).contains("misaligned 16-byte access"), "{name}: stderr");
        assert!(!String::from_utf8_lossy(&run.stdout).contains("returned"), "{name}: returned");
    }
}
