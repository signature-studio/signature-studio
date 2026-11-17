//! Tests for int_more2.rs: bsf, bsr, bts, btr, btc, rol, ror, xchg, cmpxchg16b, pause,
//! the fences, jecxz and jrcxz.
//!
//! Every vector runs on the host CPU (inline machine code, ground truth) and as lifted C
//! (clang -O0) on the same many inputs (random, boundary, and per-vector special values).
//! Registers, the 64-byte buffer, and every flag that the C side defines are compared.
//! The set of flags that C leaves `FLE_UNDEF` must be exactly the set that the SDM leaves
//! undefined (see docs/decisions/0023-int-more2.md), so that "not compared" cannot hide a
//! defined flag. Outputs go under lift/out-int2/test.

mod etxt_common;

use fe_lift::{generate, lift_body};
use std::path::PathBuf;
use std::process::Command;

const FLAG_BITS: [u64; 6] = [0x1, 0x4, 0x10, 0x40, 0x80, 0x800]; // cf pf af zf sf of
const UNDEF: u64 = 0xff;
const HOST_MASK: u64 = 0x8d5;

/// State order: rax, rcx, rdx, rbx, rsi, rdi.
const REG_NUMS: [usize; 6] = [0, 1, 2, 3, 6, 7];
const LOAD: [[u8; 4]; 6] = [
    [0x48, 0x8B, 0x45, 0x00],
    [0x48, 0x8B, 0x4D, 0x08],
    [0x48, 0x8B, 0x55, 0x10],
    [0x48, 0x8B, 0x5D, 0x18],
    [0x48, 0x8B, 0x75, 0x20],
    [0x48, 0x8B, 0x7D, 0x28],
];
const STORE: [[u8; 4]; 6] = [
    [0x48, 0x89, 0x45, 0x00],
    [0x48, 0x89, 0x4D, 0x08],
    [0x48, 0x89, 0x55, 0x10],
    [0x48, 0x89, 0x5D, 0x18],
    [0x48, 0x89, 0x75, 0x20],
    [0x48, 0x89, 0x7D, 0x28],
];

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

fn rnd_val(r: &mut Rng) -> u64 {
    const EDGE: [u64; 12] = [
        0x7f,
        0x80,
        0xff,
        0x7fff,
        0x8000,
        0xffff,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        u64::MAX,
    ];
    match r.below(12) {
        0..=2 => r.next(),
        3 => r.below(17),
        4 | 5 => 1u64 << r.below(64),
        6 => !(1u64 << r.below(64)),
        7 => 0,
        8 => EDGE[r.below(12) as usize],
        9 => r.next() >> r.below(64),
        10 => r.next() << r.below(64),
        _ => (1u64 << r.below(64)) | (1u64 << r.below(64)),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Fam {
    Scan,
    BitRmw,
    Rot,
    Plain,
    Cmpxchg16b,
}

#[derive(Clone)]
struct Vector {
    name: String,
    bytes: Vec<u8>,
    fam: Fam,
    /// rdi is the address of the 64-byte buffer + 16 (both sides).
    mem: bool,
    /// (state index, width) of a register bit offset into memory: a small signed value in the low bits.
    bitoff: Option<(usize, u32)>,
    /// Operand width in bits.
    bits: u32,
    /// Rotate count that is an immediate (or the implicit 1).
    imm_cnt: Option<u64>,
    /// Rotate count in CL.
    cl: bool,
    /// cmpxchg16b: make RDX:RAX equal to the memory operand in some inputs.
    eq16: bool,
}

fn vec_(label: &str, bytes: &[u8], fam: Fam) -> Vector {
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Vector {
        name: format!("{label} [{}]", hex.join(" ")),
        bytes: bytes.to_vec(),
        fam,
        mem: false,
        bitoff: None,
        bits: 32,
        imm_cnt: None,
        cl: false,
        eq16: false,
    }
}

impl Vector {
    fn mem(mut self) -> Self {
        self.mem = true;
        self
    }
    fn bits(mut self, b: u32) -> Self {
        self.bits = b;
        self
    }
    fn bitoff(mut self, reg: usize, w: u32) -> Self {
        self.bitoff = Some((reg, w));
        self
    }
    fn imm(mut self, c: u64) -> Self {
        self.imm_cnt = Some(c);
        self
    }
    fn cl(mut self) -> Self {
        self.cl = true;
        self
    }
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

const WIDTHS: [(&[u8], &[u8], u32); 3] = [(&[], &[], 32), (&[], &[0x48], 64), (&[0x66], &[], 16)];

fn scan_vectors() -> Vec<Vector> {
    let mut v = Vec::new();
    for (nm, op) in [("bsf", 0xBCu8), ("bsr", 0xBD)] {
        for (pfx, rex, w) in WIDTHS {
            // reg, reg: (dest, source) = (eax, esi), (ecx, ecx), (edx, ebx)
            for (d, s) in [(0u8, 6u8), (1, 1), (2, 3)] {
                let b = cat(&[pfx, rex, &[0x0F, op, 0xC0 | (d << 3) | s]]);
                v.push(vec_(&format!("{nm} reg{w}"), &b, Fam::Scan).bits(w));
            }
            // reg, [rdi + disp]
            for (d, disp) in [(0u8, None), (1, Some(8u8)), (2, Some(2))] {
                let b = match disp {
                    None => cat(&[pfx, rex, &[0x0F, op, (d << 3) | 7]]),
                    Some(x) => cat(&[pfx, rex, &[0x0F, op, 0x40 | (d << 3) | 7, x]]),
                };
                v.push(vec_(&format!("{nm} mem{w}"), &b, Fam::Scan).bits(w).mem());
            }
        }
    }
    v
}

fn bit_vectors() -> Vec<Vector> {
    let mut v = Vec::new();
    let ops = [("bts", 0xABu8, 5u8), ("btr", 0xB3, 6), ("btc", 0xBB, 7)];
    for (nm, op, ext) in ops {
        for (pfx, rex, w) in WIDTHS {
            for (off, base) in [(1u8, 0u8), (2, 3), (0, 0)] {
                let b = cat(&[pfx, rex, &[0x0F, op, 0xC0 | (off << 3) | base]]);
                v.push(vec_(&format!("{nm} reg,reg{w}"), &b, Fam::BitRmw).bits(w));
            }
            for imm in [0u8, 5, 15, 16, 31, 32, 63, 64, 0x41, 0xff] {
                let b = cat(&[pfx, rex, &[0x0F, 0xBA, 0xC0 | (ext << 3), imm]]);
                v.push(vec_(&format!("{nm} reg,imm{w}"), &b, Fam::BitRmw).bits(w));
            }
            for lock in [&[][..], &[0xF0][..]] {
                let b = cat(&[lock, pfx, rex, &[0x0F, op, 0x0F]]);
                v.push(vec_(&format!("{nm} [rdi],reg{w}"), &b, Fam::BitRmw).bits(w).mem().bitoff(1, w));
                let b = cat(&[lock, pfx, rex, &[0x0F, op, 0x4F, 0x04]]);
                v.push(vec_(&format!("{nm} [rdi+4],reg{w}"), &b, Fam::BitRmw).bits(w).mem().bitoff(1, w));
                for imm in [0u8, 9, 31, 32, 37, 60, 63, 64, 255] {
                    let b = cat(&[lock, pfx, rex, &[0x0F, 0xBA, (ext << 3) | 7, imm]]);
                    v.push(vec_(&format!("{nm} [rdi],imm{w}"), &b, Fam::BitRmw).bits(w).mem());
                    let b = cat(&[lock, pfx, rex, &[0x0F, 0xBA, 0x40 | (ext << 3) | 7, 0x08, imm]]);
                    v.push(vec_(&format!("{nm} [rdi+8],imm{w}"), &b, Fam::BitRmw).bits(w).mem());
                }
            }
        }
    }
    v
}

fn rot_vectors() -> Vec<Vector> {
    let mut v = Vec::new();
    let widths: [(&[u8], &[u8], u32); 4] = [(&[], &[], 32), (&[], &[0x48], 64), (&[0x66], &[], 16), (&[], &[], 8)];
    for (nm, ext) in [("rol", 0u8), ("ror", 1)] {
        for (pfx, rex, w) in widths {
            let (opc_cl, opc_1, opc_imm) = if w == 8 { (0xD2u8, 0xD0u8, 0xC0u8) } else { (0xD3, 0xD1, 0xC1) };
            for r in [0u8, 1] {
                let m = 0xC0 | (ext << 3) | r;
                v.push(vec_(&format!("{nm} reg{w},cl"), &cat(&[pfx, rex, &[opc_cl, m]]), Fam::Rot).bits(w).cl());
                v.push(vec_(&format!("{nm} reg{w},1"), &cat(&[pfx, rex, &[opc_1, m]]), Fam::Rot).bits(w).imm(1));
            }
            for imm in [0u8, 1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 0xff] {
                let m = 0xC0 | (ext << 3) | 2;
                v.push(vec_(&format!("{nm} reg{w},imm"), &cat(&[pfx, rex, &[opc_imm, m, imm]]), Fam::Rot).bits(w).imm(imm as u64));
            }
            let m = (ext << 3) | 7;
            v.push(vec_(&format!("{nm} [rdi],cl{w}"), &cat(&[pfx, rex, &[opc_cl, m]]), Fam::Rot).bits(w).mem().cl());
            v.push(vec_(&format!("{nm} [rdi],1 {w}"), &cat(&[pfx, rex, &[opc_1, m]]), Fam::Rot).bits(w).mem().imm(1));
            for imm in [0u8, 3, 8, 17, 33] {
                v.push(vec_(&format!("{nm} [rdi],imm{w}"), &cat(&[pfx, rex, &[opc_imm, m, imm]]), Fam::Rot).bits(w).mem().imm(imm as u64));
                v.push(vec_(
                    &format!("{nm} [rdi+8],imm{w}"),
                    &cat(&[pfx, rex, &[opc_imm, 0x40 | m, 0x08, imm]]),
                    Fam::Rot,
                )
                .bits(w)
                .mem()
                .imm(imm as u64));
            }
        }
    }
    v
}

fn xchg_vectors() -> Vec<Vector> {
    let p = |name: &str, b: &[u8]| vec_(name, b, Fam::Plain);
    vec![
        p("xchg eax,edx (87 /r)", &[0x87, 0xD0]),
        p("xchg edx,eax (92)", &[0x92]),
        p("xchg rdx,rax (48 92)", &[0x48, 0x92]),
        p("xchg dx,ax (66 92)", &[0x66, 0x92]),
        p("xchg bl,al", &[0x86, 0xC3]),
        p("xchg eax,eax (87 c0, not nop)", &[0x87, 0xC0]),
        p("xchg ebx,ebx", &[0x87, 0xDB]),
        p("xchg rax,rbx", &[0x48, 0x87, 0xD8]),
        p("xchg ax,bx", &[0x66, 0x87, 0xD8]),
        p("xchg ecx,eax", &[0x87, 0xC1]),
        p("xchg [rdi],eax", &[0x87, 0x07]).mem(),
        p("lock xchg [rdi],eax", &[0xF0, 0x87, 0x07]).mem(),
        p("xchg [rdi],rcx", &[0x48, 0x87, 0x0F]).mem(),
        p("lock xchg [rdi],rcx", &[0xF0, 0x48, 0x87, 0x0F]).mem(),
        p("xchg [rdi],al", &[0x86, 0x07]).mem(),
        p("xchg [rdi+1],bl", &[0x86, 0x5F, 0x01]).mem(),
        p("xchg [rdi+2],dx", &[0x66, 0x87, 0x57, 0x02]).mem(),
        p("xchg [rdi+4],ebx", &[0x87, 0x5F, 0x04]).mem(),
        p("xchg [rdi+8],rdx", &[0x48, 0x87, 0x57, 0x08]).mem(),
        p("lock xchg [rdi+8],rdx", &[0xF0, 0x48, 0x87, 0x57, 0x08]).mem(),
    ]
}

fn fence_vectors() -> Vec<Vector> {
    let p = |name: &str, b: &[u8]| vec_(name, b, Fam::Plain);
    vec![
        p("pause", &[0xF3, 0x90]),
        p("sfence", &[0x0F, 0xAE, 0xF8]),
        p("lfence", &[0x0F, 0xAE, 0xE8]),
        p("mfence", &[0x0F, 0xAE, 0xF0]),
    ]
}

fn cmpxchg16b_vectors() -> Vec<Vector> {
    let mut a = vec_("cmpxchg16b [rdi]", &[0x48, 0x0F, 0xC7, 0x0F], Fam::Cmpxchg16b).mem();
    a.eq16 = true;
    let mut b = vec_("lock cmpxchg16b [rdi]", &[0xF0, 0x48, 0x0F, 0xC7, 0x0F], Fam::Cmpxchg16b).mem();
    b.eq16 = true;
    vec![a, b]
}

/// One input row: [vector, rax, rcx, rdx, rbx, rsi, rdi, flags, mem0..mem7].
type Row = [u64; 16];

fn gen_row(v: &Vector, r: &mut Rng, k: usize) -> Row {
    let mut row = [0u64; 16];
    row[0] = k as u64;
    for j in 0..6 {
        row[1 + j] = rnd_val(r);
    }
    if v.cl {
        const CNT: [u64; 18] = [0, 1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 128, 129];
        if r.below(2) == 0 {
            row[2] = (row[2] & !0xff) | CNT[r.below(18) as usize];
        }
    }
    let mut f = 0;
    for b in FLAG_BITS {
        if r.next() & 1 == 1 {
            f |= b;
        }
    }
    row[7] = f;
    for j in 0..8 {
        row[8 + j] = rnd_val(r);
    }
    if let Some((reg, w)) = v.bitoff {
        // byte offset -16..31 from rdi (the buffer starts 16 bytes below rdi): bit offset -128..255.
        // The host touches a whole operand-size unit, so the range keeps that inside the 64-byte buffer.
        let s = r.below(384) as i64 - 128;
        let (low, hi) = if w == 64 { (s as u64, 0) } else { ((s as u64) & ((1u64 << w) - 1), rnd_val(r) & !((1u64 << w) - 1)) };
        row[1 + reg] = low | hi;
    }
    if v.eq16 {
        match r.below(4) {
            0 | 1 => {
                row[1] = row[8 + 2];
                row[3] = row[8 + 3];
            }
            2 => row[1] = row[8 + 2],
            _ => {}
        }
    }
    row[6] = 0;
    row
}

#[repr(C, align(16))]
struct Buf([u64; 8]);

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

/// Host stub: state pointer in rdi, u64 [rax, rcx, rdx, rbx, rsi, rdi, flags_in, flags_out].
fn stub(insn: &[u8]) -> Vec<u8> {
    let mut c = vec![0x55, 0x53, 0x48, 0x89, 0xFD];
    for l in LOAD {
        c.extend_from_slice(&l);
    }
    c.extend_from_slice(&[0xFF, 0x75, 0x30, 0x9D]); // push [rbp+0x30]; popfq
    c.extend_from_slice(insn);
    for s in STORE {
        c.extend_from_slice(&s);
    }
    c.extend_from_slice(&[0x9C, 0x8F, 0x45, 0x38, 0x5B, 0x5D, 0xC3]); // pushfq; pop [rbp+0x38]; pop rbx; pop rbp; ret
    c
}

struct Out {
    regs: [u64; 6],
    /// C: 0, 1 or UNDEF per flag. Host: 0 or 1.
    flags: [u64; 6],
    mem: [u64; 8],
}

fn host_run(vs: &[Vector], rows: &[Row]) -> Vec<Out> {
    let mut res = Vec::with_capacity(rows.len());
    unsafe {
        let codes: Vec<*mut u8> = vs
            .iter()
            .map(|v| {
                let code = stub(&v.bytes);
                let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
                assert!(!p.is_null() && p as isize != -1, "mmap failed");
                std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
                p
            })
            .collect();
        for row in rows {
            let k = row[0] as usize;
            let mut buf = Buf([0; 8]);
            buf.0.copy_from_slice(&row[8..16]);
            let mut st = [0u64; 8];
            st[..6].copy_from_slice(&row[1..7]);
            st[6] = row[7] | 0x202;
            if vs[k].mem {
                st[5] = buf.0.as_ptr() as u64 + 16;
            }
            let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut u64)>(codes[k]);
            f(st.as_mut_ptr());
            let mut regs = [0u64; 6];
            regs.copy_from_slice(&st[..6]);
            let mut flags = [0u64; 6];
            for (i, b) in FLAG_BITS.iter().enumerate() {
                flags[i] = ((st[7] & HOST_MASK) & b != 0) as u64;
            }
            res.push(Out { regs, flags, mem: buf.0 });
        }
        for p in codes {
            munmap(p, 4096);
        }
    }
    res
}

fn dirs() -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-int2/test");
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    (dir, rt)
}

fn compile(dir: &PathBuf, rt: &PathBuf, group: &str, src: &str) -> PathBuf {
    let uname = etxt_common::uniq(group);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
        .arg(rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    exe
}

fn c_run(group: &str, vs: &[Vector], rows: &[Row]) -> Vec<Out> {
    let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nstatic uint8_t M[64] __attribute__((aligned(16)));\n\n");
    for (k, v) in vs.iter().enumerate() {
        src += &format!("static void c{k}(void)\n{{\n{}\n}}\n\n", lift_body(&v.bytes, 0x1000 + k as u64 * 0x10).unwrap_or_else(|e| panic!("lift {}: {e}", v.name)));
    }
    src += "static void (*const FN[])(void) = {";
    src += &(0..vs.len()).map(|k| format!("c{k}")).collect::<Vec<_>>().join(",");
    src += "};\nstatic const uint8_t MEMV[] = {";
    src += &vs.iter().map(|v| (v.mem as u8).to_string()).collect::<Vec<_>>().join(",");
    src += "};\nstatic const uint64_t IN[][16] = {\n";
    for row in rows {
        src += "{";
        src += &row.iter().map(|x| format!("{x:#x}ull")).collect::<Vec<_>>().join(",");
        src += "},\n";
    }
    src += "};\n\nint main(void)\n{\n    for (size_t n = 0; n < sizeof IN / sizeof IN[0]; n++) {\n        const uint64_t *r = IN[n];\n        unsigned k = (unsigned)r[0];\n";
    src += "        memcpy(M, &r[8], 64);\n";
    for (j, rn) in REG_NUMS.iter().enumerate() {
        src += &format!("        R[{rn}] = r[{}];\n", 1 + j);
    }
    src += "        if (MEMV[k]) R[7] = (uint64_t)(uintptr_t)(M + 16);\n";
    src += "        FL.cf = !!(r[7] & 0x1); FL.pf = !!(r[7] & 0x4); FL.af = !!(r[7] & 0x10); FL.zf = !!(r[7] & 0x40); FL.sf = !!(r[7] & 0x80); FL.of = !!(r[7] & 0x800);\n";
    src += "        FN[k]();\n        printf(\"%u\", k);\n";
    for rn in REG_NUMS {
        src += &format!("        printf(\" %llx\", (unsigned long long)R[{rn}]);\n");
    }
    src += "        printf(\" %u %u %u %u %u %u\", FL.cf, FL.pf, FL.af, FL.zf, FL.sf, FL.of);\n";
    src += "        for (int j = 0; j < 8; j++) { uint64_t w; memcpy(&w, M + 8 * j, 8); printf(\" %llx\", (unsigned long long)w); }\n";
    src += "        printf(\"\\n\");\n    }\n    return 0;\n}\n";

    let (dir, rt) = dirs();
    let exe = compile(&dir, &rt, group, &src);
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed: {}", String::from_utf8_lossy(&run.stderr));
    String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(t.len(), 21, "bad line {line}");
            let mut regs = [0u64; 6];
            for j in 0..6 {
                regs[j] = u64::from_str_radix(t[1 + j], 16).unwrap();
            }
            let mut flags = [0u64; 6];
            for j in 0..6 {
                flags[j] = t[7 + j].parse().unwrap();
            }
            let mut mem = [0u64; 8];
            for j in 0..8 {
                mem[j] = u64::from_str_radix(t[13 + j], 16).unwrap();
            }
            Out { regs, flags, mem }
        })
        .collect()
}

/// Flags the SDM leaves undefined for this vector and input, as a bit set in FLAG_BITS order.
fn expected_undef(v: &Vector, row: &Row, host: &Out) -> u8 {
    match v.fam {
        // bsf, bsr: PF is defined only for a zero source (ZF = 1); see decision 0023.
        Fam::Scan => {
            if host.flags[3] == 1 {
                0
            } else {
                1 << 1
            }
        }
        Fam::BitRmw => (1 << 1) | (1 << 2) | (1 << 4) | (1 << 5),
        Fam::Rot => {
            let c = v.imm_cnt.unwrap_or(row[2] & 0xff);
            let c = c & if v.bits == 64 { 0x3f } else { 0x1f };
            if c >= 2 {
                1 << 5
            } else {
                0
            }
        }
        Fam::Plain | Fam::Cmpxchg16b => 0,
    }
}

struct Summary {
    vectors: usize,
    rows: usize,
    reg_cmp: usize,
    flag_cmp: usize,
    undef_flags: usize,
    zero_src: usize,
    rot_count0: usize,
    rot_count1: usize,
    eq16_equal: usize,
}

fn check(group: &str, vs: &[Vector], per_vector: usize, seed: u64) -> Summary {
    let mut rng = Rng(seed);
    let mut rows = Vec::new();
    for (k, v) in vs.iter().enumerate() {
        for _ in 0..per_vector {
            rows.push(gen_row(v, &mut rng, k));
        }
    }
    let c = c_run(group, vs, &rows);
    let h = host_run(vs, &rows);
    assert_eq!(c.len(), rows.len());
    let mut s = Summary { vectors: vs.len(), rows: rows.len(), reg_cmp: 0, flag_cmp: 0, undef_flags: 0, zero_src: 0, rot_count0: 0, rot_count1: 0, eq16_equal: 0 };
    for (i, row) in rows.iter().enumerate() {
        let v = &vs[row[0] as usize];
        let (co, ho) = (&c[i], &h[i]);
        let ctx = format!("{} row {i}: in regs {:x?} flags {:#x} mem {:x?}", v.name, &row[1..7], row[7], &row[8..16]);
        for j in 0..6 {
            if v.mem && j == 5 {
                continue;
            }
            assert_eq!(co.regs[j], ho.regs[j], "{ctx}: reg {j} (C left, host right)");
            s.reg_cmp += 1;
        }
        assert_eq!(co.mem, ho.mem, "{ctx}: memory (C left, host right)");
        let mut undef = 0u8;
        for j in 0..6 {
            if co.flags[j] == UNDEF {
                undef |= 1 << j;
                s.undef_flags += 1;
            } else {
                assert_eq!(co.flags[j], ho.flags[j], "{ctx}: flag {j} (cf pf af zf sf of) (C left, host right)");
                s.flag_cmp += 1;
            }
        }
        assert_eq!(undef, expected_undef(v, row, ho), "{ctx}: set of undefined flags (bit j = flag j)");
        if v.fam == Fam::Scan && ho.flags[3] == 1 {
            s.zero_src += 1;
        }
        if v.fam == Fam::Rot {
            let c = v.imm_cnt.unwrap_or(row[2] & 0xff) & if v.bits == 64 { 0x3f } else { 0x1f };
            s.rot_count0 += (c == 0) as usize;
            s.rot_count1 += (c == 1) as usize;
        }
        if v.fam == Fam::Cmpxchg16b && ho.flags[3] == 1 {
            s.eq16_equal += 1;
        }
    }
    let (dir, _) = dirs();
    std::fs::write(
        dir.join(format!("summary-{group}.txt")),
        format!(
            "group {group}\nvectors {}\nrows {}\nregister comparisons {}\nflag comparisons (C defined) {}\nflags left undefined by C {}\nzero-source rows (scan) {}\nrotate rows with masked count 0 {}\nrotate rows with masked count 1 {}\ncmpxchg16b rows with ZF=1 {}\n",
            s.vectors, s.rows, s.reg_cmp, s.flag_cmp, s.undef_flags, s.zero_src, s.rot_count0, s.rot_count1, s.eq16_equal
        ),
    )
    .unwrap();
    s
}

#[test]
fn bsf_bsr_match_host() {
    let s = check("scan", &scan_vectors(), 400, 0x1234_5678_9abc_def1);
    assert!(s.zero_src > 100, "zero-source rows: {}", s.zero_src);
    assert!(s.flag_cmp > 0);
}

#[test]
fn bts_btr_btc_match_host() {
    let s = check("bit", &bit_vectors(), 120, 0x0fed_cba9_8765_4321);
    assert!(s.flag_cmp > 0);
}

#[test]
fn rol_ror_match_host() {
    let s = check("rot", &rot_vectors(), 160, 0x2468_ace0_1357_9bdf);
    assert!(s.rot_count0 > 100 && s.rot_count1 > 100, "count 0 rows {} count 1 rows {}", s.rot_count0, s.rot_count1);
}

#[test]
fn xchg_matches_host() {
    check("xchg", &xchg_vectors(), 400, 0x1111_2222_3333_4445);
}

#[test]
fn pause_and_fences_match_host() {
    check("fence", &fence_vectors(), 400, 0x5555_6666_7777_8889);
}

#[test]
fn cmpxchg16b_matches_host() {
    let s = check("cmpxchg16b", &cmpxchg16b_vectors(), 800, 0x9999_aaaa_bbbb_cccd);
    assert!(s.eq16_equal > 100, "equal rows: {}", s.eq16_equal);
}

/// jecxz and jrcxz: ECX (RCX) zero. The upper half of RCX matters only for jrcxz.
#[test]
fn jecxz_jrcxz_match_host() {
    let (dir, rt) = dirs();
    let cases: [(&str, &[u8]); 2] = [("jecxz", &[0x67, 0xE3, 0x05]), ("jrcxz", &[0xE3, 0x05])];
    let mut vals: Vec<u64> = vec![0, 1, 0xffff_ffff, 0x1_0000_0000, 0x1_0000_0001, 0x8000_0000_0000_0000, u64::MAX, 0xffff_ffff_0000_0000, 0xffff, 0x1_0000];
    let mut rng = Rng(0x7777_1234_5555_9999);
    for _ in 0..200 {
        vals.push(rnd_val(&mut rng));
    }
    let (mut taken, mut not_taken) = (0, 0);
    for (nm, bytes) in cases {
        let target = 0x1000 + bytes.len() as u64 + 5;
        let body = lift_body(bytes, 0x1000).unwrap_or_else(|e| panic!("lift {nm}: {e}"));
        let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\n\n");
        src += &format!("static void c(void)\n{{\n    R[0] = 0;\n    {body}\n    R[0] = 1;\nL_{target:x}: ;\n}}\n\n");
        src += "int main(void)\n{\n    static const uint64_t V[] = {";
        src += &vals.iter().map(|x| format!("{x:#x}ull")).collect::<Vec<_>>().join(",");
        src += "};\n    for (size_t n = 0; n < sizeof V / sizeof V[0]; n++) {\n        R[1] = V[n];\n        c();\n        printf(\"%llu\\n\", (unsigned long long)R[0]);\n    }\n    return 0;\n}\n";
        let exe = compile(&dir, &rt, nm, &src);
        let run = Command::new(&exe).output().expect("run failed");
        assert!(run.status.success());
        let cres: Vec<u64> = String::from_utf8(run.stdout).unwrap().lines().map(|l| l.parse().unwrap()).collect();
        assert_eq!(cres.len(), vals.len());

        let mut insn = vec![0x31, 0xC0];
        insn.extend_from_slice(bytes);
        insn.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        let code = stub(&insn);
        for (i, &x) in vals.iter().enumerate() {
            let mut st = [0u64, x, 0, 0, 0, 0, 0x202, 0];
            unsafe {
                let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
                assert!(!p.is_null() && p as isize != -1, "mmap failed");
                std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
                let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut u64)>(p);
                f(st.as_mut_ptr());
                munmap(p, 4096);
            }
            assert_eq!(cres[i], st[0], "{nm} rcx={x:#x} (C left, host right)");
            if st[0] == 0 {
                taken += 1;
            } else {
                not_taken += 1;
            }
        }
    }
    assert!(taken > 10 && not_taken > 10, "taken {taken}, not taken {not_taken}");
    std::fs::write(dir.join("summary-jcxz.txt"), format!("rcx values {}\ntaken {taken}\nnot taken {not_taken}\n", vals.len())).unwrap();
}

/// cmpxchg16b on an address that is not 16-byte aligned is #GP in the SDM: the run stops.
#[test]
fn cmpxchg16b_misaligned_aborts() {
    let (dir, rt) = dirs();
    let body = lift_body(&[0x48, 0x0F, 0xC7, 0x0F], 0x1000).expect("lift");
    let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nstatic uint8_t M[64] __attribute__((aligned(16)));\n");
    src += &format!("static void c(void)\n{{\n{body}\n}}\nint main(void)\n{{\n    R[7] = (uint64_t)(uintptr_t)(M + 8);\n    c();\n    return 0;\n}}\n");
    let exe = compile(&dir, &rt, "cmpxchg16b_mis", &src);
    let run = Command::new(&exe).output().expect("run failed");
    assert!(!run.status.success(), "expected an abort");
    assert!(String::from_utf8_lossy(&run.stderr).contains("not 16-byte aligned"));
}

/// Not lifted on purpose: 16-bit jcxz and xchg through GS. Decision 0047 changed two of the old entries:
/// xgetbv is now a named stop (lifted as a call to fle_xgetbv_stop, see tests/a2a_remaining.rs), and a
/// high-byte register operand is handled (rol ah,cl matches the host in tests/a2a_remaining.rs).
#[test]
fn unsupported_stay_unsupported() {
    assert!(lift_body(&[0x67, 0xE3, 0x05], 0x1000).is_ok(), "jecxz");
    assert!(lift_body(&[0xD2, 0xC4], 0x1000).is_ok(), "rol ah,cl (high byte register, decision 0047)");
    assert!(lift_body(&[0x65, 0x48, 0x87, 0x07], 0x1000).is_err(), "xchg with GS segment");
}

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

/// A synthetic PE whose functions 0x1020 (jecxz) and 0x1030 (jrcxz) jump to the function entry 0x1010.
fn build_pe() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x400);
    w32(&mut b, o + 36, 0);
    w32(&mut b, o + 56, 0x2000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);

    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1010)].concat()),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1020)].concat()),
        (0x100a, [&[0xe8][..], &rel(0x100f, 0x1030)].concat()),
        (0x100f, vec![0xc3]),
        (0x1010, vec![0xb8, 0x09, 0x00, 0x00, 0x00, 0xc3]),
        // jecxz 0x1010 (rel8 = 0x1010 - 0x1023); mov eax, 7; ret
        (0x1020, vec![0x67, 0xe3, 0xed, 0xb8, 0x07, 0x00, 0x00, 0x00, 0xc3]),
        // jrcxz 0x1010 (rel8 = 0x1010 - 0x1032); mov eax, 8; ret
        (0x1030, vec![0xe3, 0xde, 0xb8, 0x08, 0x00, 0x00, 0x00, 0xc3]),
    ];
    for (rva, bytes) in code {
        let o = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

/// A jecxz or jrcxz whose target is another function entry becomes a tail call, as for a Jcc.
#[test]
fn jcxz_to_function_entry_is_a_tail_call() {
    let mut out = Vec::new();
    let st = generate(&build_pe(), &mut out).expect("generate");
    assert_eq!(st.functions, 4, "functions");
    assert_eq!(st.unsupported_total, 0, "unsupported: {:?}", st.unsupported);
    assert_eq!(st.handled.get("jecxz"), Some(&1));
    assert_eq!(st.handled.get("jrcxz"), Some(&1));
    let c = String::from_utf8(out).expect("utf8");
    assert!(c.contains(&format!("if ((uint32_t)R[1] == 0) {{ f_{:x}(); return; }}", IB + 0x1010)), "jecxz tail call missing");
    assert!(c.contains(&format!("if (R[1] == 0) {{ f_{:x}(); return; }}", IB + 0x1010)), "jrcxz tail call missing");

    let (dir, rt) = dirs();
    let f20 = IB + 0x1020;
    let f30 = IB + 0x1030;
    let main_src = format!(
        "#include <stdio.h>\n#include <stdint.h>\nextern uint64_t R[16];\nvoid f_{f20:x}(void);\nvoid f_{f30:x}(void);\n\
         int main(void) {{\n    static const uint64_t V[] = {{0, 1, 0x100000000ull, 0x100000001ull}};\n    for (int i = 0; i < 4; i++) {{\n\
         R[1] = V[i]; R[0] = 0; f_{f20:x}(); printf(\"%llu \", (unsigned long long)R[0]);\n\
         R[1] = V[i]; R[0] = 0; f_{f30:x}(); printf(\"%llu\\n\", (unsigned long long)R[0]);\n    }}\n    return 0;\n}}\n"
    );
    let tag = etxt_common::uniq("jcxz_pe");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &out_c(&c)).unwrap();
    let msrc = dir.join(format!("{tag}_main.c"));
    std::fs::write(&msrc, main_src).unwrap();
    let bin = dir.join(&tag);
    let cc = Command::new("clang")
        .env("TMPDIR", dir.join("tmp"))
        .args(["-O0", "-mno-red-zone", "-w", "-I"])
        .arg(&rt)
        .args(["-o", bin.to_str().unwrap(), src.to_str().unwrap(), msrc.to_str().unwrap()])
        .output()
        .expect("run clang");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&bin).output().expect("run");
    assert!(r.status.success());
    // rcx = 0: both taken (9 9). rcx = 1: neither (7 8).
    // rcx = 2^32: ecx is 0 so jecxz is taken (9), rcx is not 0 so jrcxz is not (8).
    // rcx = 2^32 + 1: neither (7 8).
    assert_eq!(String::from_utf8_lossy(&r.stdout), "9 9\n7 8\n9 8\n7 8\n");
}

fn out_c(c: &str) -> String {
    c.to_string()
}
