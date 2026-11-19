//! U-D のテスト。パック SSE の命令について、生成 C（`lift_body` の出力）を
//! clang -O0 でコンパイルして実行し、ホストの x86_64 CPU で同じ命令を実行した
//! 結果と比べる。比べるのは XMM0〜15 の全 128 bit、rax・rcx・rdx・rbx・rsi、
//! メモリの 64 byte。パック命令はフラグを変えないので、フラグは比べない。
//!
//! 入力は各レーンの境界値（0、最大値、符号ビット、NaN、無限大、丸めの境界）と
//! 乱数の組。メモリ形式は 16 byte 境界（disp 0 と 16）と、非整列（disp 4）を使う。
//! アラインメント違反の扱いは、ホストの実行結果（SIGSEGV など）と生成 C の abort を突き合わせる。

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::{Command, Output};

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
    /// GPR, x86 number (0 rax, 1 rcx, 2 rdx, 3 rbx, 6 rsi).
    G(u8),
    /// [rdi + disp8].
    Mem(i8),
}

/// [prefix] [REX] 0F opc modrm [disp8]. `reg` is the modrm.reg field.
fn enc(prefix: Option<u8>, w: bool, opc: &[u8], reg: u8, rm: Rm) -> Vec<u8> {
    let (rm_low, rm_hi) = match rm {
        Rm::X(n) | Rm::G(n) => (n & 7, n >> 3),
        Rm::Mem(_) => (7, 0),
    };
    let rex = 0x40 | ((w as u8) << 3) | (((reg >> 3) & 1) << 2) | rm_hi;
    let mut b: Vec<u8> = prefix.into_iter().collect();
    if rex != 0x40 {
        b.push(rex);
    }
    b.extend_from_slice(opc);
    match rm {
        Rm::X(_) | Rm::G(_) => b.push(0xc0 | ((reg & 7) << 3) | rm_low),
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

/// Test outputs go under lift/out-uD (not /tmp).
fn out_dir() -> PathBuf {
    let d = crate_dir().join("../../out-uD/test");
    std::fs::create_dir_all(&d).expect("create test output dir");
    d
}

const HEADER: &str = "#include \"fle_runtime.h\"\n#include \"fle_sse.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nfle_xmm X[16];\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n";

/// Compiles `src` with clang -O0 into out-uD/test/`name`, and returns the path.
fn compile(name: &str, src: &str) -> PathBuf {
    let dir = out_dir();
    let rt = crate_dir().join("../../runtime");
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).expect("write C source");
    let cc = Command::new("clang")
        .args(["-O0", "-std=c11", "-ffp-contract=off", "-w", "-I"])
        .arg(&rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
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

    let exe = compile(&format!("sse_pack_{group}"), &src);
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

/// Integer edge values for one lane of `esz` bytes (1, 2, 4 or 8).
fn pool_for(esz: usize) -> Vec<u64> {
    match esz {
        1 => vec![0, 1, 0x7f, 0x80, 0xfe, 0xff, 0x0f, 0xf0],
        2 => vec![0, 1, 0x7fff, 0x8000, 0xffff, 0x8001, 0x00ff, 0xff00, 0x0100],
        4 => vec![
            0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x8000_0001, 0xffff_fffe, 0x0001_0000, 0x0000_ffff,
            0x1234_5678,
        ],
        _ => ints(),
    }
}

/// Single-precision inputs as bit patterns (upper 32 bits zero): zeros, ties,
/// the 32-bit integer range edges, NaN (quiet, negative, signaling), infinities,
/// subnormals.
fn pool32() -> Vec<u64> {
    let mut v: Vec<u64> = [
        0.0f32, -0.0, 0.5, -0.5, 1.5, 2.5, -2.5, 3.5, 2147483520.0, 2147483648.0, -2147483648.0, -2147483904.0,
        2147483647.0, 16777217.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, 1e-40, 0.1, 9.223372e18,
    ]
    .iter()
    .map(|x| x.to_bits() as u64)
    .collect();
    v.extend([0x7fc0_0000u64, 0xffc0_0001, 0x7f80_0001, 0x4f00_0000, 0xcf00_0000, 0x3f80_0001, 0x7f7f_ffff, 1]);
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
        9007199254740993,
        0x1234_5678_9abc_def0,
        123456789,
    ];
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..16 {
        v.push(rng.next_u64());
    }
    v
}

/// A 128-bit value whose lanes of `esz` bytes are taken from `pool`, with a
/// quarter of the lanes random.
fn vec128(rng: &mut Rng, pool: &[u64], esz: usize) -> [u64; 2] {
    let mut b = [0u8; 16];
    for k in 0..16 / esz {
        let v = if rng.next_u64() % 4 == 0 { rng.next_u64() } else { pick(rng, pool) };
        b[k * esz..(k + 1) * esz].copy_from_slice(&v.to_le_bytes()[..esz]);
    }
    [u64::from_le_bytes(b[..8].try_into().unwrap()), u64::from_le_bytes(b[8..].try_into().unwrap())]
}

/// Two-source form. xmm1 = a and the second source = b, in three forms: the
/// register form (xmm2), the memory form (disp from `disps`), and xmm9, xmm14.
/// Every fourth pair has b = a, or b = a with one half changed, so that equal lanes occur.
fn bin128(rng: &mut Rng, name: &str, prefix: Option<u8>, opc: &[u8], esz: usize, pool: &[u64], disps: &[i8], n: usize) -> Vec<Case> {
    let mut v = Vec::new();
    for k in 0..n {
        let a = vec128(rng, pool, esz);
        let b = match k % 4 {
            0 => a,
            1 => [a[0], rng.next_u64()],
            _ => vec128(rng, pool, esz),
        };
        let d = disps[k % disps.len()];
        let mut c = base(format!("{name} reg #{k}"), enc(prefix, false, opc, 1, Rm::X(2)), rng);
        c.xmm[1] = a;
        c.xmm[2] = b;
        v.push(c);
        let mut c = base(format!("{name} mem d={d} #{k}"), enc(prefix, false, opc, 1, Rm::Mem(d)), rng);
        c.xmm[1] = a;
        put128(&mut c.mem, (16 + d as i32) as usize, b);
        v.push(c);
        let mut c = base(format!("{name} xmm9,xmm14 #{k}"), enc(prefix, false, opc, 9, Rm::X(14)), rng);
        c.xmm[9] = a;
        c.xmm[14] = b;
        v.push(c);
    }
    v
}

/// Store forms (opcode 0F 11 / 0F 29 / 7F): xmm2 is stored to memory (disp from
/// `disps`), copied to xmm1, and stored from xmm9 to memory.
fn store128(rng: &mut Rng, name: &str, prefix: Option<u8>, opc: &[u8], disps: &[i8], n: usize) -> Vec<Case> {
    let p = pool_for(8);
    let mut v = Vec::new();
    for k in 0..n {
        let a = vec128(rng, &p, 8);
        let d = disps[k % disps.len()];
        let mut c = base(format!("{name} mem d={d} #{k}"), enc(prefix, false, opc, 2, Rm::Mem(d)), rng);
        c.xmm[2] = a;
        v.push(c);
        let mut c = base(format!("{name} reg #{k}"), enc(prefix, false, opc, 2, Rm::X(1)), rng);
        c.xmm[2] = a;
        v.push(c);
        let mut c = base(format!("{name} xmm9 mem d={d} #{k}"), enc(prefix, false, opc, 9, Rm::Mem(d)), rng);
        c.xmm[9] = a;
        v.push(c);
    }
    v
}

/// PSHUFD with each of the 256 immediates, register and memory source.
fn pshufd_cases(rng: &mut Rng) -> Vec<Case> {
    let p = pool_for(4);
    let mut v = Vec::new();
    for imm in 0..=255u8 {
        for k in 0..2 {
            let a = vec128(rng, &p, 4);
            let mut bytes = enc(Some(0x66), false, &[0x0f, 0x70], 1, Rm::X(2));
            bytes.push(imm);
            let mut c = base(format!("pshufd reg imm={imm:#x} #{k}"), bytes, rng);
            c.xmm[2] = a;
            v.push(c);
            let mut bytes = enc(Some(0x66), false, &[0x0f, 0x70], 1, Rm::Mem(0));
            bytes.push(imm);
            let mut c = base(format!("pshufd mem imm={imm:#x} #{k}"), bytes, rng);
            put128(&mut c.mem, 16, a);
            v.push(c);
        }
    }
    v
}

/// MOVD and MOVQ in every form. `how`: 'g' sets rbx, 'x' sets the low 64 bits of
/// xmm2, '4' and '8' set 4 or 8 bytes at mem[16..].
fn movdq_cases(rng: &mut Rng, n: usize) -> Vec<Case> {
    let p = ints();
    let forms: Vec<(&str, Vec<u8>, char)> = vec![
        ("movd xmm1,ebx", enc(Some(0x66), false, &[0x0f, 0x6e], 1, Rm::G(3)), 'g'),
        ("movq xmm1,rbx", enc(Some(0x66), true, &[0x0f, 0x6e], 1, Rm::G(3)), 'g'),
        ("movd xmm1,m32", enc(Some(0x66), false, &[0x0f, 0x6e], 1, Rm::Mem(0)), '4'),
        ("movq xmm1,m64", enc(Some(0xf3), false, &[0x0f, 0x7e], 1, Rm::Mem(0)), '8'),
        ("movq xmm1,xmm2", enc(Some(0xf3), false, &[0x0f, 0x7e], 1, Rm::X(2)), 'x'),
        ("movd ebx,xmm2", enc(Some(0x66), false, &[0x0f, 0x7e], 2, Rm::G(3)), 'x'),
        ("movq rbx,xmm2", enc(Some(0x66), true, &[0x0f, 0x7e], 2, Rm::G(3)), 'x'),
        ("movd m32,xmm2", enc(Some(0x66), false, &[0x0f, 0x7e], 2, Rm::Mem(0)), 'x'),
        ("movq m64,xmm2", enc(Some(0x66), false, &[0x0f, 0xd6], 2, Rm::Mem(0)), 'x'),
        ("movq xmm1,xmm2 (D6)", enc(Some(0x66), false, &[0x0f, 0xd6], 2, Rm::X(1)), 'x'),
    ];
    let mut v = Vec::new();
    for (name, bytes, how) in forms {
        for k in 0..n {
            let x = pick(rng, &p);
            let mut c = base(format!("{name} #{k}"), bytes.clone(), rng);
            match how {
                'g' => c.gpr[3] = x,
                'x' => c.xmm[2][0] = x,
                '4' => c.mem[16..20].copy_from_slice(&x.to_le_bytes()[..4]),
                _ => c.mem[16..24].copy_from_slice(&x.to_le_bytes()),
            }
            v.push(c);
        }
    }
    v
}

#[test]
fn mov_load_matches_host() {
    let mut rng = Rng(0x243f_6a88_85a3_08d3);
    let p = pool_for(8);
    let mut cs = Vec::new();
    cs.extend(bin128(&mut rng, "movaps", None, &[0x0f, 0x28], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "movapd", Some(0x66), &[0x0f, 0x28], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "movdqa", Some(0x66), &[0x0f, 0x6f], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "movups", None, &[0x0f, 0x10], 8, &p, &[4, 0], 100));
    cs.extend(bin128(&mut rng, "movupd", Some(0x66), &[0x0f, 0x10], 8, &p, &[4, 0], 100));
    cs.extend(bin128(&mut rng, "movdqu", Some(0xf3), &[0x0f, 0x6f], 8, &p, &[4, 0], 100));
    check("mov_load", &cs);
}

/// Decision 0113 wall 3: LDDQU xmm, m128 (SSE3, F2 0F F0 /r) loads 16 bytes from any address, like MOVDQU. The host runs the same bytes; the cases use
/// destination xmm1 and xmm9 (REX.R), displacements 0, 16 (aligned) and 4, -4, 1, 15, -16, 32 (misaligned, at the edges of the 64-byte buffer).
#[test]
fn lddqu_matches_host() {
    let mut rng = Rng(0x452f_821e_638d_0137);
    let p = pool_for(8);
    let disps: [i8; 8] = [0, 16, 4, -4, 1, 15, -16, 32];
    let mut cs = Vec::new();
    for k in 0..160usize {
        let d = disps[k % disps.len()];
        let a = vec128(&mut rng, &p, 8);
        let mut c = base(format!("lddqu xmm1,[rdi{d:+}] #{k}"), enc(Some(0xf2), false, &[0x0f, 0xf0], 1, Rm::Mem(d)), &mut rng);
        put128(&mut c.mem, (16 + d as i32) as usize, a);
        cs.push(c);
        let a = vec128(&mut rng, &p, 8);
        let mut c = base(format!("lddqu xmm9,[rdi{d:+}] #{k}"), enc(Some(0xf2), false, &[0x0f, 0xf0], 9, Rm::Mem(d)), &mut rng);
        put128(&mut c.mem, (16 + d as i32) as usize, a);
        cs.push(c);
    }
    check("lddqu", &cs);
}

/// Only the memory form of the legacy LDDQU exists: the register form (modrm.mod = 11) does not decode, and the VEX forms (VLDDQU, AVX) stay unsupported.
#[test]
fn lddqu_other_forms() {
    assert!(lift_body(&[0xf2, 0x0f, 0xf0, 0xc1], 0x1000).is_err(), "lddqu xmm0, xmm1 does not exist");
    assert!(lift_body(&[0xc5, 0xfb, 0xf0, 0x07], 0x1000).is_err(), "vlddqu xmm0, [rdi] (AVX) stays unsupported");
    assert!(lift_body(&[0xc5, 0xff, 0xf0, 0x07], 0x1000).is_err(), "vlddqu ymm0, [rdi] (AVX) stays unsupported");
    let b = lift_body(&[0xf2, 0x0f, 0xf0, 0x07], 0x1000).unwrap();
    assert!(b.contains("X[0] = fle_ldx("), "{b}");
    // the aligned load is for the legacy packed forms; LDDQU takes any address, so it must not use it
    assert!(!b.contains("fle_ldxa"), "{b}");
}

#[test]
fn mov_store_matches_host() {
    let mut rng = Rng(0x13198a2e_03707344);
    let mut cs = Vec::new();
    cs.extend(store128(&mut rng, "movaps", None, &[0x0f, 0x29], &[0, 16], 100));
    cs.extend(store128(&mut rng, "movapd", Some(0x66), &[0x0f, 0x29], &[0, 16], 100));
    cs.extend(store128(&mut rng, "movdqa", Some(0x66), &[0x0f, 0x7f], &[0, 16], 100));
    cs.extend(store128(&mut rng, "movups", None, &[0x0f, 0x11], &[4, 0], 100));
    cs.extend(store128(&mut rng, "movupd", Some(0x66), &[0x0f, 0x11], &[4, 0], 100));
    cs.extend(store128(&mut rng, "movdqu", Some(0xf3), &[0x0f, 0x7f], &[4, 0], 100));
    check("mov_store", &cs);
}

#[test]
fn logic_matches_host() {
    let mut rng = Rng(0xa409_3822_299f_31d0);
    let p = pool_for(8);
    let mut cs = Vec::new();
    cs.extend(bin128(&mut rng, "pand", Some(0x66), &[0x0f, 0xdb], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "pandn", Some(0x66), &[0x0f, 0xdf], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "por", Some(0x66), &[0x0f, 0xeb], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "pxor", Some(0x66), &[0x0f, 0xef], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "andps", None, &[0x0f, 0x54], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "andpd", Some(0x66), &[0x0f, 0x54], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "orpd", Some(0x66), &[0x0f, 0x56], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "xorps", None, &[0x0f, 0x57], 8, &p, &[0, 16], 100));
    cs.extend(bin128(&mut rng, "xorpd", Some(0x66), &[0x0f, 0x57], 8, &p, &[0, 16], 100));
    check("logic", &cs);
}

#[test]
fn int_matches_host() {
    let mut rng = Rng(0x082e_fa98_ec4e_6c89);
    let mut cs = Vec::new();
    cs.extend(bin128(&mut rng, "paddd", Some(0x66), &[0x0f, 0xfe], 4, &pool_for(4), &[0, 16], 100));
    cs.extend(bin128(&mut rng, "psubd", Some(0x66), &[0x0f, 0xfa], 4, &pool_for(4), &[0, 16], 100));
    cs.extend(bin128(&mut rng, "pcmpeqd", Some(0x66), &[0x0f, 0x76], 4, &pool_for(4), &[0, 16], 100));
    cs.extend(bin128(&mut rng, "pmulhuw", Some(0x66), &[0x0f, 0xe4], 2, &pool_for(2), &[0, 16], 100));
    check("int", &cs);
}

#[test]
fn shuffle_matches_host() {
    let mut rng = Rng(0x452821e6_38d01377);
    let mut cs = pshufd_cases(&mut rng);
    cs.extend(bin128(&mut rng, "punpcklbw", Some(0x66), &[0x0f, 0x60], 1, &pool_for(1), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpcklwd", Some(0x66), &[0x0f, 0x61], 2, &pool_for(2), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpckldq", Some(0x66), &[0x0f, 0x62], 4, &pool_for(4), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpcklqdq", Some(0x66), &[0x0f, 0x6c], 8, &pool_for(8), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpckhbw", Some(0x66), &[0x0f, 0x68], 1, &pool_for(1), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpckhwd", Some(0x66), &[0x0f, 0x69], 2, &pool_for(2), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpckhdq", Some(0x66), &[0x0f, 0x6a], 4, &pool_for(4), &[0, 16], 80));
    cs.extend(bin128(&mut rng, "punpckhqdq", Some(0x66), &[0x0f, 0x6d], 8, &pool_for(8), &[0, 16], 80));
    check("shuffle", &cs);
}

#[test]
fn cvt_packed_matches_host() {
    let mut rng = Rng(0x38d0_1377_be54_66cf);
    let p = pool32();
    let mut cs = Vec::new();
    cs.extend(bin128(&mut rng, "cvtps2dq", Some(0x66), &[0x0f, 0x5b], 4, &p, &[0, 16], 200));
    cs.extend(bin128(&mut rng, "cvttps2dq", Some(0xf3), &[0x0f, 0x5b], 4, &p, &[0, 16], 200));
    check("cvt_packed", &cs);
}

#[test]
fn movdq_matches_host() {
    let mut rng = Rng(0x34e9_0c6c_c0ac_29b7);
    let cs = movdq_cases(&mut rng, 60);
    check("movdq", &cs);
}

/// The aligned forms fault on a misaligned 16-byte operand. The host is the
/// reference: each program below is run on the host (asm) and as generated C.
/// The generated C must fault exactly where the host does.
#[test]
fn misaligned_fault_matches_host() {
    // (name, asm, bytes of the same instruction, expect fault)
    let cases: Vec<(&str, &str, Vec<u8>, bool)> = vec![
        ("movaps", "movaps 4(%0), %%xmm1", enc(None, false, &[0x0f, 0x28], 1, Rm::Mem(4)), true),
        ("pand", "pand 4(%0), %%xmm1", enc(Some(0x66), false, &[0x0f, 0xdb], 1, Rm::Mem(4)), true),
        ("movups", "movups 4(%0), %%xmm1", enc(None, false, &[0x0f, 0x10], 1, Rm::Mem(4)), false),
        ("movaps aligned", "movaps 16(%0), %%xmm1", enc(None, false, &[0x0f, 0x28], 1, Rm::Mem(16)), false),
    ];
    for (k, (name, asm, bytes, fault)) in cases.iter().enumerate() {
        // Host.
        let src = format!(
            "#include <stdint.h>\nstatic uint8_t M[64] __attribute__((aligned(16)));\nint main(void) {{\n    __asm__ volatile(\"{asm}\" : : \"r\"(M + 16) : \"xmm1\", \"memory\");\n    return 0;\n}}\n"
        );
        let exe = {
            let dir = out_dir();
            let tag = etxt_common::uniq(&format!("align_host_{k}"));
            let c = dir.join(format!("{tag}.c"));
            let exe = dir.join(&tag);
            std::fs::write(&c, &src).expect("write host align source");
            let cc = Command::new("clang").args(["-O0", "-w", "-o"]).arg(&exe).arg(&c).output().expect("run clang");
            assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
            exe
        };
        let host = Command::new(&exe).output().expect("run host align test");
        assert_eq!(!host.status.success(), *fault, "{name}: host fault behaviour: {:?}", host.status);

        // Generated C.
        let body = lift_body(bytes, 0x1000).unwrap_or_else(|e| panic!("{name}: {e}"));
        let src = format!(
            "{HEADER}static void c0(void)\n{{\n{body}\n}}\n\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 16);\n    c0();\n    printf(\"ok\\n\");\n    return 0;\n}}\n"
        );
        let exe = compile(&format!("align_c_{k}"), &src);
        let out: Output = Command::new(&exe).output().expect("run generated align test");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(!out.status.success(), *fault, "{name}: generated C fault behaviour: {:?} {stderr}", out.status);
        if *fault {
            assert!(stderr.contains("misaligned"), "{name}: abort message missing: {stderr}");
        } else {
            assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n", "{name}");
        }
    }
}
