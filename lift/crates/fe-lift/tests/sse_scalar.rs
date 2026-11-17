//! U-C のテスト。スカラー SSE の命令について、生成 C（`lift_body` の出力）を
//! clang -O0 でコンパイルして実行し、ホストの x86_64 CPU で同じ命令を実行した
//! 結果と比べる。比べるのは XMM0〜15 の全 128 bit、rax・rcx・rdx・rbx・rsi、
//! メモリの 64 byte、そして ucomi / comi のフラグ 6 個（CF PF AF ZF SF OF）。
//!
//! 入力は境界値（±0、±無限大、NaN（静・準・符号付き・ペイロード付き）、非正規数、
//! 丸めの境界、整数の端）の組と、乱数の組。上位レーンと未使用の XMM は乱数で埋める。

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
    cmp_flags: bool,
}

#[derive(Debug, PartialEq)]
struct Out {
    xmm: [[u64; 2]; 16],
    gpr: [u64; 5],
    mem: [u8; 64],
    /// CF, PF, AF, ZF, SF, OF
    flags: [u8; 6],
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
    /// GPR, x86 number (0 rax, 1 rcx, 2 rdx, 3 rbx, 6 rsi).
    G(u8),
    /// [rdi + 0].
    Mem,
}

/// [prefix] [REX] 0F opc modrm. `reg` is the modrm.reg field.
fn enc(prefix: Option<u8>, w: bool, opc: &[u8], reg: u8, rm: Rm) -> Vec<u8> {
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
        Rm::Mem => {
            b.push(0x80 | ((reg & 7) << 3) | 7);
            b.extend_from_slice(&0i32.to_le_bytes());
        }
    }
    b
}

#[repr(C, align(16))]
struct State {
    /// rax, rcx, rdx, rbx, rsi, rdi
    gpr: [u64; 8],
    xmm: [[u64; 2]; 16],
    flags: u64,
    mem: [u8; 64],
}

const OFF_GPR: usize = std::mem::offset_of!(State, gpr);
const OFF_XMM: usize = std::mem::offset_of!(State, xmm);
const OFF_FLAGS: usize = std::mem::offset_of!(State, flags);

/// Host stub: loads the state into the registers, runs `body`, saves the
/// registers and the flags back into the state. It keeps rbx and rbp (callee-saved).
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
    // pushfq; pop qword [rbp+disp32]
    c.push(0x9c);
    c.extend_from_slice(&[0x8f, 0x85]);
    c.extend_from_slice(&disp(OFF_FLAGS));
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

fn bit(v: u64, b: u32) -> u8 {
    ((v >> b) & 1) as u8
}

/// Runs the case on the host CPU.
fn run_host(c: &Case) -> Out {
    let mut st = Box::new(State { gpr: [0; 8], xmm: c.xmm, flags: 0, mem: c.mem });
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
    let fl = st.flags;
    Out {
        xmm: st.xmm,
        gpr: st.gpr[..5].try_into().unwrap(),
        mem: st.mem,
        flags: [bit(fl, 0), bit(fl, 2), bit(fl, 4), bit(fl, 6), bit(fl, 7), bit(fl, 11)],
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Test outputs go under lift/out-uC (not /tmp).
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-uC/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

/// Builds one C program that runs every case, and returns the results.
fn c_run(group: &str, cs: &[Case]) -> Vec<Out> {
    let mut src = String::from(
        "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64];\n\n",
    );
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
    src += &format!(
        "int main(void)\n{{\n    unsigned k, j;\n    for (k = 0; k < {}u; k++) {{\n\
         \x20       for (j = 0; j < 16; j++) {{ X[j].u64[0] = IN[k].xmm[j][0]; X[j].u64[1] = IN[k].xmm[j][1]; }}\n\
         \x20       R[0] = IN[k].gpr[0]; R[1] = IN[k].gpr[1]; R[2] = IN[k].gpr[2]; R[3] = IN[k].gpr[3]; R[6] = IN[k].gpr[4];\n\
         \x20       R[7] = (uint64_t)(uintptr_t)(M + 16);\n\
         \x20       memcpy(M, IN[k].mem, sizeof M);\n\
         \x20       FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = 0;\n\
         \x20       FN[k]();\n\
         \x20       printf(\"%u\", k);\n\
         \x20       for (j = 0; j < 16; j++) printf(\" %llx %llx\", (unsigned long long)X[j].u64[0], (unsigned long long)X[j].u64[1]);\n\
         \x20       printf(\" %llx %llx %llx %llx %llx\", (unsigned long long)R[0], (unsigned long long)R[1], (unsigned long long)R[2], (unsigned long long)R[3], (unsigned long long)R[6]);\n\
         \x20       printf(\" %u %u %u %u %u %u\", FL.cf, FL.pf, FL.af, FL.zf, FL.sf, FL.of);\n\
         \x20       for (j = 0; j < 64; j++) printf(\" %02x\", M[j]);\n\
         \x20       printf(\"\\n\");\n\
         \x20   }}\n    return 0;\n}}\n",
        cs.len()
    );

    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let tag = etxt_common::uniq(&format!("sse_{group}"));
    let c = dir.join(format!("{tag}.c"));
    let exe = dir.join(&tag);
    std::fs::write(&c, &src).expect("write sse C source");
    let cc = Command::new("clang")
        .args(["-O0", "-std=c11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .output()
        .expect("run clang (is clang installed?)");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run test binary");
    assert!(run.status.success(), "C program failed");
    let text = String::from_utf8(run.stdout).expect("utf8 output");

    let hex = |s: &str| u64::from_str_radix(s, 16).expect("hex");
    let out: Vec<Out> = text
        .lines()
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
            let mut flags = [0u8; 6];
            for f in flags.iter_mut() {
                *f = t[p].parse().expect("flag");
                p += 1;
            }
            let mut mem = [0u8; 64];
            for m in mem.iter_mut() {
                *m = u8::from_str_radix(t[p], 16).expect("byte");
                p += 1;
            }
            Out { xmm, gpr, mem, flags }
        })
        .collect();
    out
}

fn check(group: &str, cs: &[Case]) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_run(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let mut fails = 0usize;
    for (case, cout) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        let same = h.xmm == cout.xmm
            && h.gpr == cout.gpr
            && h.mem == cout.mem
            && (!case.cmp_flags || h.flags == cout.flags);
        if !same {
            fails += 1;
            if fails <= 10 {
                eprintln!("{group}: {}\n  bytes {:02x?}\n  C    {:x?} {:x?} {:?}\n  host {:x?} {:x?} {:?}",
                    case.name, case.bytes, cout.xmm, cout.gpr, cout.flags, h.xmm, h.gpr, h.flags);
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails}", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
}

fn put_lo(x: &mut [u64; 2], sz: usize, v: u64) {
    if sz == 4 {
        x[0] = (x[0] & !0xffff_ffffu64) | (v & 0xffff_ffff);
    } else {
        x[0] = v;
    }
}

fn put_mem(m: &mut [u8; 64], sz: usize, v: u64) {
    m[16..16 + sz].copy_from_slice(&v.to_le_bytes()[..sz]);
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
    Case { name, bytes, xmm, gpr, mem, cmp_flags: false }
}

fn pick(rng: &mut Rng, pool: &[u64]) -> u64 {
    pool[(rng.next_u64() % pool.len() as u64) as usize]
}

/// Binary form: xmm1 low = a, source = xmm2 low = b (or mem[16..] = b for the
/// memory form). Also random pairs in xmm9 and xmm14, to check REX.R and REX.B.
fn binary(rng: &mut Rng, name: &str, prefix: Option<u8>, opc: &[u8], sz: usize, pool: &[u64], cmp: bool) -> Vec<Case> {
    let mut v = Vec::new();
    for &a in pool {
        for &b in pool {
            let mut c = base(format!("{name} reg a={a:#x} b={b:#x}"), enc(prefix, false, opc, 1, Rm::X(2)), rng);
            put_lo(&mut c.xmm[1], sz, a);
            put_lo(&mut c.xmm[2], sz, b);
            c.cmp_flags = cmp;
            v.push(c);
            let mut c = base(format!("{name} mem a={a:#x} b={b:#x}"), enc(prefix, false, opc, 1, Rm::Mem), rng);
            put_lo(&mut c.xmm[1], sz, a);
            put_mem(&mut c.mem, sz, b);
            c.cmp_flags = cmp;
            v.push(c);
        }
    }
    for _ in 0..64 {
        let (a, b) = (pick(rng, pool), pick(rng, pool));
        let mut c = base(format!("{name} xmm9,xmm14 a={a:#x} b={b:#x}"), enc(prefix, false, opc, 9, Rm::X(14)), rng);
        put_lo(&mut c.xmm[9], sz, a);
        put_lo(&mut c.xmm[14], sz, b);
        c.cmp_flags = cmp;
        v.push(c);
    }
    v
}

/// Unary form: source xmm2 low (or mem[16..]) = a, destination xmm1.
fn unary(rng: &mut Rng, name: &str, prefix: Option<u8>, opc: &[u8], sz: usize, pool: &[u64]) -> Vec<Case> {
    let mut v = Vec::new();
    for &a in pool {
        let mut c = base(format!("{name} reg a={a:#x}"), enc(prefix, false, opc, 1, Rm::X(2)), rng);
        put_lo(&mut c.xmm[2], sz, a);
        v.push(c);
        let mut c = base(format!("{name} mem a={a:#x}"), enc(prefix, false, opc, 1, Rm::Mem), rng);
        put_mem(&mut c.mem, sz, a);
        v.push(c);
        let mut c = base(format!("{name} xmm9,xmm14 a={a:#x}"), enc(prefix, false, opc, 9, Rm::X(14)), rng);
        put_lo(&mut c.xmm[14], sz, a);
        v.push(c);
    }
    v
}

/// MOVSS / MOVSD store forms (0F 11): to memory, and register to register.
fn store(rng: &mut Rng, name: &str, prefix: u8, sz: usize, pool: &[u64]) -> Vec<Case> {
    let mut v = Vec::new();
    for &b in pool {
        let mut c = base(format!("{name} store b={b:#x}"), enc(Some(prefix), false, &[0x0f, 0x11], 2, Rm::Mem), rng);
        put_lo(&mut c.xmm[2], sz, b);
        v.push(c);
        let mut c = base(format!("{name} reg b={b:#x}"), enc(Some(prefix), false, &[0x0f, 0x11], 2, Rm::X(1)), rng);
        put_lo(&mut c.xmm[2], sz, b);
        v.push(c);
    }
    v
}

/// CVTSI2SS / CVTSI2SD: source rbx (32 or 64 bit) or [rdi].
fn cvtsi2(rng: &mut Rng, name: &str, prefix: u8, ints: &[u64]) -> Vec<Case> {
    let mut v = Vec::new();
    for &x in ints {
        for w in [false, true] {
            let sz = if w { 8 } else { 4 };
            let mut c = base(format!("{name} rbx w={} v={x:#x}", w as u8), enc(Some(prefix), w, &[0x0f, 0x2a], 1, Rm::G(3)), rng);
            if w {
                c.gpr[3] = x;
            } else {
                c.gpr[3] = (c.gpr[3] & !0xffff_ffffu64) | (x & 0xffff_ffff);
            }
            v.push(c);
            let mut c = base(format!("{name} mem w={} v={x:#x}", w as u8), enc(Some(prefix), w, &[0x0f, 0x2a], 1, Rm::Mem), rng);
            put_mem(&mut c.mem, sz, x);
            v.push(c);
        }
    }
    v
}

/// CVTTSS2SI, CVTSS2SI and the sd forms: GPR destination rcx (32 or 64 bit).
fn cvt2si(rng: &mut Rng, name: &str, prefix: u8, opc: &[u8], sz: usize, pool: &[u64]) -> Vec<Case> {
    let mut v = Vec::new();
    for &a in pool {
        for w in [false, true] {
            let mut c = base(format!("{name} reg w={} a={a:#x}", w as u8), enc(Some(prefix), w, opc, 1, Rm::X(2)), rng);
            put_lo(&mut c.xmm[2], sz, a);
            v.push(c);
            let mut c = base(format!("{name} mem w={} a={a:#x}", w as u8), enc(Some(prefix), w, opc, 1, Rm::Mem), rng);
            put_mem(&mut c.mem, sz, a);
            v.push(c);
        }
    }
    v
}

/// Single-precision inputs as bit patterns (upper 32 bits zero).
fn pool32() -> Vec<u64> {
    let mut v: Vec<u64> = [
        0.0f32, -0.0, 1.0, -1.0, 2.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.25, 3.0, 2147483520.0, 2147483648.0,
        -2147483648.0, -2147483904.0, 9.223372e18, f32::MAX, f32::MIN_POSITIVE, f32::INFINITY, f32::NEG_INFINITY,
        1e-40, 16777216.0, 16777217.0, 0.1, 3.0e38,
    ]
    .iter()
    .map(|x| x.to_bits() as u64)
    .collect();
    // qNaN, negative NaN with payload, sNaN, smallest subnormal, largest subnormal,
    // 1 + ulp, 2^25, 2^31 as bits.
    v.extend([0x7fc0_0000u64, 0xffc0_0001, 0x7f80_0001, 1, 0x007f_ffff, 0x3f80_0001, 0x4b80_0001, 0x4f00_0000]);
    v
}

/// Double-precision inputs as bit patterns. Includes the rounding boundaries of
/// the integer and single-precision conversions.
fn pool64() -> Vec<u64> {
    let mut v: Vec<u64> = [
        0.0f64, -0.0, 1.0, -1.0, 2.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 3.5, 0.25, 2147483647.5, 2147483648.0,
        -2147483648.0, -2147483648.5, 2147483647.0, -2147483649.0, 9223372036854775808.0, -9223372036854775808.0,
        1e19, 0.1, 16777217.0, 1e300, f64::MAX, f64::MIN_POSITIVE, f64::INFINITY, f64::NEG_INFINITY,
    ]
    .iter()
    .map(|x| x.to_bits())
    .collect();
    v.extend([
        0x7ff8_0000_0000_0000u64,
        0xfff8_0000_0000_0001,
        0x7ff0_0000_0000_0001,
        1,
        0x000f_ffff_ffff_ffff,
        0x3ff0_0000_1000_0000, // 1 + 2^-24: a tie for single precision
        0x3ff0_0000_1000_0001,
        0x3ff0_0000_3000_0000,
        0x3ff0_0000_2000_0000,
        0x43e0_0000_0000_0000, // 2^63
        0x4340_0000_0000_0001,
    ]);
    v
}

fn ints() -> Vec<u64> {
    let mut v = vec![
        0u64,
        1,
        u64::MAX,
        2,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        16777217,
        16777215,
        16777219,
        9007199254740993,
        9007199254740995,
        0x1234_5678_9abc_def0,
        123456789,
    ];
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..32 {
        v.push(rng.next_u64());
    }
    v
}

#[test]
fn arith_ss_matches_host() {
    let p = pool32();
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut cs = Vec::new();
    cs.extend(binary(&mut rng, "addss", Some(0xf3), &[0x0f, 0x58], 4, &p, false));
    cs.extend(binary(&mut rng, "subss", Some(0xf3), &[0x0f, 0x5c], 4, &p, false));
    cs.extend(binary(&mut rng, "mulss", Some(0xf3), &[0x0f, 0x59], 4, &p, false));
    cs.extend(binary(&mut rng, "divss", Some(0xf3), &[0x0f, 0x5e], 4, &p, false));
    check("arith_ss", &cs);
}

#[test]
fn arith_sd_matches_host() {
    let p = pool64();
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    let mut cs = Vec::new();
    cs.extend(binary(&mut rng, "addsd", Some(0xf2), &[0x0f, 0x58], 8, &p, false));
    cs.extend(binary(&mut rng, "subsd", Some(0xf2), &[0x0f, 0x5c], 8, &p, false));
    cs.extend(binary(&mut rng, "mulsd", Some(0xf2), &[0x0f, 0x59], 8, &p, false));
    cs.extend(binary(&mut rng, "divsd", Some(0xf2), &[0x0f, 0x5e], 8, &p, false));
    check("arith_sd", &cs);
}

#[test]
fn minmax_matches_host() {
    let p32 = pool32();
    let p64 = pool64();
    let mut rng = Rng(0x6a09_e667_f3bc_c908);
    let mut cs = Vec::new();
    cs.extend(binary(&mut rng, "minss", Some(0xf3), &[0x0f, 0x5d], 4, &p32, false));
    cs.extend(binary(&mut rng, "maxss", Some(0xf3), &[0x0f, 0x5f], 4, &p32, false));
    cs.extend(binary(&mut rng, "minsd", Some(0xf2), &[0x0f, 0x5d], 8, &p64, false));
    cs.extend(binary(&mut rng, "maxsd", Some(0xf2), &[0x0f, 0x5f], 8, &p64, false));
    check("minmax", &cs);
}

#[test]
fn sqrt_matches_host() {
    let mut rng = Rng(0xbb67_ae85_84ca_a73b);
    let mut cs = Vec::new();
    cs.extend(unary(&mut rng, "sqrtss", Some(0xf3), &[0x0f, 0x51], 4, &pool32()));
    cs.extend(unary(&mut rng, "sqrtsd", Some(0xf2), &[0x0f, 0x51], 8, &pool64()));
    check("sqrt", &cs);
}

#[test]
fn mov_matches_host() {
    let p32 = pool32();
    let p64 = pool64();
    let mut rng = Rng(0x3c6e_f372_fe94_f82b);
    let mut cs = Vec::new();
    cs.extend(binary(&mut rng, "movss", Some(0xf3), &[0x0f, 0x10], 4, &p32, false));
    cs.extend(binary(&mut rng, "movsd", Some(0xf2), &[0x0f, 0x10], 8, &p64, false));
    cs.extend(store(&mut rng, "movss", 0xf3, 4, &p32));
    cs.extend(store(&mut rng, "movsd", 0xf2, 8, &p64));
    check("mov", &cs);
}

#[test]
fn cvt_fp_matches_host() {
    let mut rng = Rng(0x510e_527f_ade6_82d1);
    let mut cs = Vec::new();
    cs.extend(unary(&mut rng, "cvtss2sd", Some(0xf3), &[0x0f, 0x5a], 4, &pool32()));
    cs.extend(unary(&mut rng, "cvtsd2ss", Some(0xf2), &[0x0f, 0x5a], 8, &pool64()));
    check("cvt_fp", &cs);
}

#[test]
fn cvtsi2_matches_host() {
    let v = ints();
    let mut rng = Rng(0x9b05_688c_2b3e_6c1f);
    let mut cs = Vec::new();
    cs.extend(cvtsi2(&mut rng, "cvtsi2ss", 0xf3, &v));
    cs.extend(cvtsi2(&mut rng, "cvtsi2sd", 0xf2, &v));
    check("cvtsi2", &cs);
}

#[test]
fn cvt2si_matches_host() {
    let mut rng = Rng(0x1f83_d9ab_fb41_bd6b);
    let mut cs = Vec::new();
    cs.extend(cvt2si(&mut rng, "cvttss2si", 0xf3, &[0x0f, 0x2c], 4, &pool32()));
    cs.extend(cvt2si(&mut rng, "cvtss2si", 0xf3, &[0x0f, 0x2d], 4, &pool32()));
    cs.extend(cvt2si(&mut rng, "cvttsd2si", 0xf2, &[0x0f, 0x2c], 8, &pool64()));
    cs.extend(cvt2si(&mut rng, "cvtsd2si", 0xf2, &[0x0f, 0x2d], 8, &pool64()));
    check("cvt2si", &cs);
}

#[test]
fn comi_matches_host() {
    let p32 = pool32();
    let p64 = pool64();
    let mut rng = Rng(0x5be0_cd19_137e_2179);
    let mut cs = Vec::new();
    cs.extend(binary(&mut rng, "ucomiss", None, &[0x0f, 0x2e], 4, &p32, true));
    cs.extend(binary(&mut rng, "comiss", None, &[0x0f, 0x2f], 4, &p32, true));
    cs.extend(binary(&mut rng, "ucomisd", Some(0x66), &[0x0f, 0x2e], 8, &p64, true));
    cs.extend(binary(&mut rng, "comisd", Some(0x66), &[0x0f, 0x2f], 8, &p64, true));
    check("comi", &cs);
}
