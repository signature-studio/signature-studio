//! Tests for sse_ssse3.rs (decision 0190, wrp2, PROVISIONAL): PALIGNR, PABSB, PABSW, PABSD, PSHUFB (xmm forms) lifted to C (clang -O0, header-only runtime fle_sse.h -> fle_ssse3.h) against the REAL
//! instruction run on the host CPU (machine code in an mmap'd page, not intrinsics), on the same state: all sixteen XMM registers, rax, rcx, rdx, rbx, rsi and 64 bytes of memory must match bit for bit
//! (compared as an FNV-1a hash of the whole state per case, printed by the C program; a mismatch is reported with the case). The instructions change no flags.
//! Inputs: >= 100,000 random states per instruction group (random bytes with every byte value class, forced 0x00/0x7f/0x80/0xff and the minimum signed value of each lane width for PABS*, masks with
//! and without bit 7 for PSHUFB), PALIGNR with EVERY imm8 0..31 and 32, 33, 64, 128, 255 (zero results), register and 16-byte aligned memory sources ([rdi+disp8], disp -16, 0, 16, 32), destination equal to
//! source and different, xmm0-xmm15.
//! Mutation check: tools/mutate-ssse3.sh (in the results directory).

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
    let dir = crate_dir().join("../../out-wrp2/test");
    std::fs::create_dir_all(dir.join("tmp")).expect("create test output dir");
    dir
}

/// Compiles `src` with clang -O0 into out-wrp2/test, and returns the path. The runtime is
/// header-only for these instructions (fle_sse.h includes fle_sse_ssse3.h).
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

/// FNV-1a over the state in the order of Out (xmm 16 x 2 qwords, gpr 5 qwords, mem 64 bytes).
fn fnv(o: &Out) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut put = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    for x in o.xmm.iter() {
        for q in x.iter() {
            for k in 0..8 {
                put((q >> (8 * k)) as u8);
            }
        }
    }
    for g in o.gpr.iter() {
        for k in 0..8 {
            put((g >> (8 * k)) as u8);
        }
    }
    for m in o.mem.iter() {
        put(*m);
    }
    h
}

/// One C program: one function per distinct instruction bytes, one state per case; prints "k hash" per case.
fn c_hashes(group: &str, cs: &[Case]) -> Vec<u64> {
    let mut src = String::from(HEADER);
    let mut fn_of: std::collections::HashMap<Vec<u8>, usize> = std::collections::HashMap::new();
    let mut fns: Vec<Vec<u8>> = Vec::new();
    for c in cs {
        if !fn_of.contains_key(&c.bytes) {
            fn_of.insert(c.bytes.clone(), fns.len());
            fns.push(c.bytes.clone());
        }
    }
    for (k, b) in fns.iter().enumerate() {
        let body = lift_body(b, 0x1000).unwrap_or_else(|e| panic!("{group}: {b:02x?}: {e}"));
        src += &format!("static void c{k}(void)\n{{\n{body}\n}}\n");
    }
    src += "static void (*const FN[])(void) = {";
    for k in 0..fns.len() {
        src += &format!(" c{k},");
    }
    src += " };\n";
    src += "struct In { uint32_t f; uint64_t xmm[16][2]; uint64_t gpr[5]; uint8_t mem[64]; };\nstatic const struct In IN[] = {\n";
    for c in cs {
        let xmm: Vec<String> = c.xmm.iter().map(|p| format!("{{ {}ULL, {}ULL }}", p[0], p[1])).collect();
        let gpr: Vec<String> = c.gpr.iter().map(|g| format!("{g}ULL")).collect();
        let mem: Vec<String> = c.mem.iter().map(|b| b.to_string()).collect();
        src += &format!("{{ {}, {{ {} }}, {{ {} }}, {{ {} }} }},\n", fn_of[&c.bytes], xmm.join(", "), gpr.join(", "), mem.join(", "));
    }
    src += "};\n";
    src += r#"static uint64_t H;
static void put(uint8_t b) { H ^= b; H *= 0x100000001b3ULL; }
static void put64(uint64_t q) { int k; for (k = 0; k < 8; k++) put((uint8_t)(q >> (8 * k))); }
int main(void)
{
    unsigned long k;
    unsigned j;
    for (k = 0; k < NCASES; k++) {
        for (j = 0; j < 16; j++) { X[j].u64[0] = IN[k].xmm[j][0]; X[j].u64[1] = IN[k].xmm[j][1]; }
        R[0] = IN[k].gpr[0]; R[1] = IN[k].gpr[1]; R[2] = IN[k].gpr[2]; R[3] = IN[k].gpr[3]; R[6] = IN[k].gpr[4];
        R[7] = (uint64_t)(uintptr_t)(M + 16);
        memcpy(M, IN[k].mem, sizeof M);
        FN[IN[k].f]();
        H = 0xcbf29ce484222325ULL;
        for (j = 0; j < 16; j++) { put64(X[j].u64[0]); put64(X[j].u64[1]); }
        put64(R[0]); put64(R[1]); put64(R[2]); put64(R[3]); put64(R[6]);
        for (j = 0; j < 64; j++) put(M[j]);
        printf("%lu %llx\n", k, (unsigned long long)H);
    }
    return 0;
}
"#;
    src = src.replace("NCASES", &cs.len().to_string());
    let exe = compile(&format!("sse_ssse3_{group}"), &src);
    let run = Command::new(&exe).output().expect("run test binary");
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(exe.with_extension("c"));
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    let text = String::from_utf8(run.stdout).expect("utf8 output");
    text.lines().map(|l| u64::from_str_radix(l.split_whitespace().nth(1).unwrap(), 16).unwrap()).collect()
}

fn check(group: &str, cs: &[Case]) {
    assert!(!cs.is_empty(), "{group}: no cases");
    let c = c_hashes(group, cs);
    assert_eq!(c.len(), cs.len(), "{group}: C output line count");
    let (mut fails, mut changed) = (0usize, 0usize);
    for (case, ch) in cs.iter().zip(c.iter()) {
        let h = run_host(case);
        if h.xmm != case.xmm {
            changed += 1;
        }
        if fnv(&h) != *ch {
            fails += 1;
            if fails <= 5 {
                eprintln!("{group}: {}\n  bytes {:02x?}\n  in   {:x?}\n  host {:x?}", case.name, case.bytes, case.xmm, h.xmm);
            }
        }
    }
    eprintln!("{group}: cases {} failures {fails} host_changed_xmm {changed}", cs.len());
    assert!(changed * 10 >= cs.len() * 8, "{group}: only {changed} of {} cases changed an XMM register", cs.len());
    assert_eq!(fails, 0, "{group}: {fails} of {} cases differ from the host", cs.len());
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

/// Fill a 16-byte value with a mix of random bytes and boundary bytes.
fn bytes16(rng: &mut Rng, lane: usize) -> [u64; 2] {
    let mut b = [0u8; 16];
    for x in b.iter_mut() {
        *x = rng.next_u64() as u8;
    }
    match rng.next_u64() % 8 {
        0 => {
            // the minimum signed value of every lane, mixed with zero and the maximum
            for k in (0..16).step_by(lane) {
                let pick = rng.next_u64() % 4;
                for j in 0..lane {
                    b[k + j] = match pick {
                        0 => 0,
                        1 => if j == lane - 1 { 0x80 } else { 0 },
                        2 => if j == lane - 1 { 0x7f } else { 0xff },
                        _ => 0xff,
                    };
                }
            }
        }
        1 => {
            for x in b.iter_mut() {
                *x &= 0x8f;
            }
        }
        2 => {
            for x in b.iter_mut() {
                *x &= 0x0f;
            }
        }
        _ => {}
    }
    [u64::from_le_bytes(b[..8].try_into().unwrap()), u64::from_le_bytes(b[8..].try_into().unwrap())]
}

/// Cases for one instruction: `opc` the opcode bytes after the 66 prefix, `imm` an optional immediate byte; `n` states per register/memory shape.
fn cases(name: &str, opc: &[u8], imms: &[Option<u8>], n: usize, lane: usize, seed: u64) -> Vec<Case> {
    let mut rng = Rng(seed);
    let mut cs = Vec::new();
    let shapes: Vec<(u8, Rm)> = {
        let mut v = Vec::new();
        for k in 0..16u8 {
            v.push((k, Rm::X((k * 5 + 3) % 16)));
        }
        for k in 0..16u8 {
            v.push((k, Rm::X(k))); // destination = source
        }
        for d in [-16i8, 0, 16, 32] {
            for k in [0u8, 7, 8, 15] {
                v.push((k, Rm::Mem(d)));
            }
        }
        v
    };
    let mut made = 0usize;
    'outer: loop {
        for (reg, rm) in shapes.iter() {
            for imm in imms.iter() {
                let mut bytes = enc(Some(0x66), opc, *reg, *rm);
                if let Some(i) = imm {
                    bytes.push(*i);
                }
                let mut c = base(format!("{name} reg {reg} rm {:?} imm {imm:?}", match rm { Rm::X(x) => format!("x{x}"), Rm::Mem(d) => format!("[rdi{d:+}]") }), bytes, &mut rng);
                for x in c.xmm.iter_mut() {
                    *x = bytes16(&mut rng, lane);
                }
                for off in (0..64).step_by(16) {
                    let v = bytes16(&mut rng, lane);
                    put128(&mut c.mem, off, v);
                }
                cs.push(c);
                made += 1;
                if made >= n {
                    break 'outer;
                }
            }
        }
    }
    cs
}

fn put128(m: &mut [u8; 64], off: usize, v: [u64; 2]) {
    m[off..off + 8].copy_from_slice(&v[0].to_le_bytes());
    m[off + 8..off + 16].copy_from_slice(&v[1].to_le_bytes());
}

const N: usize = 120_000;

#[test]
fn pabsb_matches_the_host() {
    check("pabsb", &cases("pabsb", &[0x0f, 0x38, 0x1c], &[None], N, 1, 0x1900_0001));
}

#[test]
fn pabsw_matches_the_host() {
    check("pabsw", &cases("pabsw", &[0x0f, 0x38, 0x1d], &[None], N, 2, 0x1900_0002));
}

#[test]
fn pabsd_matches_the_host() {
    check("pabsd", &cases("pabsd", &[0x0f, 0x38, 0x1e], &[None], N, 4, 0x1900_0003));
}

#[test]
fn pshufb_matches_the_host() {
    check("pshufb", &cases("pshufb", &[0x0f, 0x38, 0x00], &[None], N, 1, 0x1900_0004));
}

#[test]
fn palignr_matches_the_host_for_every_immediate() {
    let mut imms: Vec<Option<u8>> = (0u8..=32).map(Some).collect();
    imms.extend([33u8, 47, 63, 64, 100, 128, 200, 255].map(Some));
    check("palignr", &cases("palignr", &[0x0f, 0x3a, 0x0f], &imms, N, 1, 0x1900_0005));
}

#[test]
fn the_mmx_forms_are_not_lifted() {
    // PSHUFB mm0, mm1 (0F 38 00 C1) and PABSB mm0, mm1 (0F 38 1C C1): the xmm-only lifter must refuse them (they stay a named stop)
    for b in [[0x0f, 0x38, 0x00, 0xc1], [0x0f, 0x38, 0x1c, 0xc1]] {
        assert!(lift_body(&b, 0x1000).is_err(), "MMX form {b:02x?} must not be lifted by sse_ssse3.rs");
    }
}
