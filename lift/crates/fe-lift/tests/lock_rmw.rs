//! Tests for the atomic read-modify-write forms of the lifter (mt4 stage A): LOCK-prefixed ADD, ADC, SUB, SBB, AND, OR, XOR, INC, DEC, NEG, NOT, BTS, BTR, BTC with a memory operand (a compare-and-swap loop around fle_cas), XCHG, XADD and
//! CMPXCHG with a memory operand (fle_xchg, fle_xadd, fle_cas), in every operand width and with a misaligned operand (the interlock path).
//!
//! Each vector runs on the host CPU (ground truth: the instruction with its LOCK prefix) and as lifted C (clang -O0, the runtime's fle_rmw.h, native `__atomic` builtins). The six registers, the 64-byte buffer and the six defined flags
//! (CF, PF, AF, ZF, SF, OF; a flag the lifted code leaves undefined is not compared) must match. The host side reads the flags with `pushfq; pop r11` after the instruction.

mod etxt_common;

use fe_lift::lift_body;
use std::path::PathBuf;
use std::process::Command;

/// Vector state order: rax, rcx, rdx, rbx, rsi, rdi.
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

struct Vector {
    name: String,
    /// optional prefix run on both sides before the instruction (stc / clc), lifted separately on the C side
    pre: &'static [u8],
    bytes: Vec<u8>,
    regs: [u64; 6],
}

fn v(name: &str, pre: &'static [u8], bytes: &[u8], regs: [u64; 6]) -> Vector {
    Vector { name: name.to_string(), pre, bytes: bytes.to_vec(), regs }
}

fn pattern() -> [u8; 64] {
    let mut m = [0u8; 64];
    for (j, b) in m.iter_mut().enumerate().skip(16) {
        *b = ((j * 73 + 0x81) & 0xff) as u8;
    }
    m
}

const STC: &[u8] = &[0x48, 0x83, 0xFA, 0x01]; // cmp rdx, 1 (rdx = 0): CF = 1
const CLC: &[u8] = &[0x48, 0x83, 0xFA, 0x00]; // cmp rdx, 0: CF = 0
const NONE: &[u8] = &[];

/// registers: rax = the operand register / accumulator, rcx = another, rdx, rbx unused, rsi = bit offset
fn vectors() -> Vec<Vector> {
    let mut vs = Vec::new();
    let vals: [u64; 4] = [0, 1, 0x7F, 0xFFFF_FFFF_FFFF_FFFF];
    // two-operand ALU: opcode bytes for `lock OP [rdi+DISP], reg` at widths 8/16/32/64 (the register is al/ax/eax/rax)
    for (nm, op8, op) in [("add", 0x00u8, 0x01u8), ("sub", 0x28, 0x29), ("and", 0x20, 0x21), ("or", 0x08, 0x09), ("xor", 0x30, 0x31)] {
        for (w, disp) in [(1usize, 1u8), (2, 2), (4, 4), (8, 8)] {
            for a in vals {
                let mut b = vec![0xF0];
                if w == 2 {
                    b.insert(0, 0x66);
                }
                if w == 8 {
                    b.push(0x48);
                }
                b.push(if w == 1 { op8 } else { op });
                b.extend_from_slice(&[0x47, disp]);
                vs.push(v(&format!("lock {nm} [rdi+{disp}], {}-bit reg (rax={a:#x})", w * 8), NONE, &b, [a, 0, 0, 0, 0, 0]));
            }
        }
    }
    for (nm, op8, op) in [("adc", 0x10u8, 0x11u8), ("sbb", 0x18, 0x19)] {
        for (pre, cf) in [(STC, "CF=1"), (CLC, "CF=0")] {
            for (w, disp) in [(1usize, 1u8), (4, 4), (8, 8)] {
                let mut b = vec![0xF0];
                if w == 8 {
                    b.push(0x48);
                }
                b.push(if w == 1 { op8 } else { op });
                b.extend_from_slice(&[0x47, disp]);
                vs.push(v(&format!("lock {nm} [rdi+{disp}], reg {cf} {}-bit", w * 8), pre, &b, [0x80, 0, 0, 0, 0, 0]));
                vs.push(v(&format!("lock {nm} [rdi+{disp}], reg {cf} {}-bit (wrap)", w * 8), pre, &b, [u64::MAX, 0, 0, 0, 0, 0]));
            }
        }
    }
    // misaligned operand (the interlock path): lock add [rdi+1], eax
    vs.push(v("lock add [rdi+1], eax (misaligned)", NONE, &[0xF0, 0x01, 0x47, 0x01], [0x1234_5678, 0, 0, 0, 0, 0]));
    vs.push(v("lock add [rdi+3], rax (misaligned, crosses 8)", NONE, &[0xF0, 0x48, 0x01, 0x47, 0x03], [0x1122_3344_5566_7788, 0, 0, 0, 0, 0]));
    vs.push(v("lock cmpxchg [rdi+1], ecx (misaligned, equal)", NONE, &[0xF0, 0x0F, 0xB1, 0x4F, 0x01], [0, 0x77, 0, 0, 0, 0]));
    vs.push(v("xchg [rdi+2], eax (misaligned)", NONE, &[0x87, 0x47, 0x02], [0xDEAD_BEEF, 0, 0, 0, 0, 0]));
    // unary: inc dec neg not
    for (nm, b8, b) in [("inc", [0xFEu8, 0x47], [0xFFu8, 0x47]), ("dec", [0xFE, 0x4F], [0xFF, 0x4F])] {
        for (w, disp) in [(1usize, 1u8), (2, 2), (4, 4), (8, 8)] {
            let mut bytes = vec![0xF0];
            if w == 2 {
                bytes.insert(0, 0x66);
            }
            if w == 8 {
                bytes.push(0x48);
            }
            let o = if w == 1 { b8 } else { b };
            bytes.extend_from_slice(&[o[0], o[1], disp]);
            vs.push(v(&format!("lock {nm} [rdi+{disp}] {}-bit", w * 8), STC, &bytes, [0, 0, 0, 0, 0, 0]));
        }
    }
    for (nm, modrm) in [("not", 0x57u8), ("neg", 0x5F)] {
        for (w, disp) in [(1usize, 1u8), (4, 4), (8, 8)] {
            let mut bytes = vec![0xF0];
            if w == 8 {
                bytes.push(0x48);
            }
            bytes.extend_from_slice(&[if w == 1 { 0xF6 } else { 0xF7 }, modrm, disp]);
            vs.push(v(&format!("lock {nm} [rdi+{disp}] {}-bit", w * 8), NONE, &bytes, [0, 0, 0, 0, 0, 0]));
        }
    }
    // bit test and set / reset / complement: register offset (the byte at EA + offset/8) and immediate offset
    for (nm, opr) in [("bts", 0xABu8), ("btr", 0xB3), ("btc", 0xBB)] {
        for off in [0u64, 5, 13, 40, 63] {
            vs.push(v(&format!("lock {nm} [rdi], eax (offset {off})"), NONE, &[0xF0, 0x0F, opr, 0x07], [off, 0, 0, 0, 0, 0]));
            vs.push(v(&format!("lock {nm} [rdi+8], rax (offset {off}, 64-bit)"), NONE, &[0xF0, 0x48, 0x0F, opr, 0x47, 0x08], [off, 0, 0, 0, 0, 0]));
        }
    }
    // immediate bit forms (reg field /5 /6 /7 of 0F BA with mod=01 [rdi+disp8])
    for (nm, modrm) in [("bts", 0x6Fu8), ("btr", 0x77), ("btc", 0x7F)] {
        for off in [0u8, 7, 21, 31] {
            vs.push(v(&format!("lock {nm} dword [rdi+4], {off}"), NONE, &[0xF0, 0x0F, 0xBA, modrm, 0x04, off], [0; 6]));
            vs.push(v(&format!("lock {nm} qword [rdi+8], {off}"), NONE, &[0xF0, 0x48, 0x0F, 0xBA, modrm, 0x08, off], [0; 6]));
        }
    }
    // xchg, xadd, cmpxchg
    for (w, disp) in [(1usize, 1u8), (2, 2), (4, 4), (8, 8)] {
        for a in [0u64, 0x1234_5678_9ABC_DEF0, u64::MAX] {
            let mut x = vec![];
            if w == 2 {
                x.push(0x66);
            }
            if w == 8 {
                x.push(0x48);
            }
            x.push(if w == 1 { 0x86 } else { 0x87 });
            x.extend_from_slice(&[0x47, disp]);
            vs.push(v(&format!("xchg [rdi+{disp}], {}-bit reg (rax={a:#x})", w * 8), NONE, &x, [a, 0, 0, 0, 0, 0]));
            let mut xa = vec![0xF0];
            if w == 2 {
                xa.insert(0, 0x66);
            }
            if w == 8 {
                xa.push(0x48);
            }
            xa.extend_from_slice(&[0x0F, if w == 1 { 0xC0 } else { 0xC1 }, 0x47, disp]);
            vs.push(v(&format!("lock xadd [rdi+{disp}], {}-bit reg (rax={a:#x})", w * 8), NONE, &xa, [a, 0, 0, 0, 0, 0]));
        }
        // cmpxchg [rdi+disp], rcx-width: equal (rax = the memory value) and not equal
        let p = pattern();
        let mut mem = 0u64;
        for k in 0..w {
            mem |= (p[16 + disp as usize + k] as u64) << (8 * k);
        }
        let m = if w == 8 { u64::MAX } else { (1u64 << (8 * w)) - 1 };
        let mut c = vec![0xF0];
        if w == 2 {
            c.insert(0, 0x66);
        }
        if w == 8 {
            c.push(0x48);
        }
        c.extend_from_slice(&[0x0F, if w == 1 { 0xB0 } else { 0xB1 }, 0x4F, disp]);
        vs.push(v(&format!("lock cmpxchg [rdi+{disp}], {}-bit rcx (equal)", w * 8), NONE, &c, [mem & m, 0x55AA_55AA_55AA_55AA, 0, 0, 0, 0]));
        vs.push(v(&format!("lock cmpxchg [rdi+{disp}], {}-bit rcx (not equal)", w * 8), NONE, &c, [(mem ^ 1) & m, 0x55AA_55AA_55AA_55AA, 0, 0, 0, 0]));
    }
    vs
}

extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

/// (registers, buffer, flags as the six defined bits in the order cf pf af zf sf of)
fn host_run(v: &Vector) -> ([u64; 6], [u8; 64], [u8; 6]) {
    let mut code = vec![0x55, 0x48, 0x89, 0xFD];
    for l in LOAD {
        code.extend_from_slice(&l);
    }
    code.extend_from_slice(v.pre);
    code.extend_from_slice(&v.bytes);
    code.extend_from_slice(&[0x9C, 0x41, 0x5B]); // pushfq; pop r11
    for s in STORE {
        code.extend_from_slice(&s);
    }
    code.extend_from_slice(&[0x4C, 0x89, 0x5D, 0x30]); // mov [rbp+0x30], r11
    code.extend_from_slice(&[0x5D, 0xC3]);
    let buf = pattern();
    let mut st = [0u64; 7];
    st[..6].copy_from_slice(&v.regs);
    st[5] = buf.as_ptr() as u64 + 16;
    unsafe {
        let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
        assert!(!p.is_null() && p as isize != -1, "mmap failed");
        std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
        let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut u64)>(p);
        f(st.as_mut_ptr());
        munmap(p, 4096);
    }
    let fl = st[6];
    let bit = |b: u32| ((fl >> b) & 1) as u8;
    let mut regs = [0u64; 6];
    regs.copy_from_slice(&st[..6]);
    (regs, buf, [bit(0), bit(2), bit(4), bit(6), bit(7), bit(11)])
}

/// Builds one C program with every vector, runs it, and returns (registers, buffer, flags; 0xff = undefined) per vector.
fn c_run(group: &str, vs: &[Vector]) -> Vec<([u64; 6], [u8; 64], [u8; 6])> {
    let p = pattern();
    let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nstatic uint8_t M[64];\n");
    src += "static const uint8_t P[64] = {";
    src += &p.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
    src += "};\n\n";
    for (k, v) in vs.iter().enumerate() {
        let pre = if v.pre.is_empty() { String::new() } else { lift_body(v.pre, 0x1000 + k as u64 * 0x20).expect("lift pre") };
        src += &format!("static void c{k}(void)\n{{\n{pre}\n{}\n}}\n\n", lift_body(&v.bytes, 0x1010 + k as u64 * 0x20).expect("lift"));
    }
    src += "int main(void)\n{\n";
    for (k, v) in vs.iter().enumerate() {
        src += "    memcpy(M, P, sizeof M);\n    FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;\n";
        for (j, r) in REG_NUMS.iter().enumerate() {
            src += &format!("    R[{r}] = UINT64_C({:#x});\n", v.regs[j]);
        }
        src += "    R[7] = (uint64_t)(uintptr_t)(M + 16);\n";
        src += &format!("    c{k}();\n    printf(\"%d\", {k});\n");
        for r in REG_NUMS {
            src += &format!("    printf(\" %llx\", (unsigned long long)R[{r}]);\n");
        }
        src += "    for (int j = 0; j < 64; j++) printf(\" %02x\", M[j]);\n";
        src += "    printf(\" %x %x %x %x %x %x\", FL.cf, FL.pf, FL.af, FL.zf, FL.sf, FL.of);\n    printf(\"\\n\");\n";
    }
    src += "    return 0;\n}\n";

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-more/test");
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    let uname = etxt_common::uniq(group);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, &src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-I"])
        .arg(&rt)
        .arg("-o")
        .arg(&exe)
        .arg(&c)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed");
    String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let t: Vec<&str> = line.split_whitespace().collect();
            let mut st = [0u64; 6];
            for j in 0..6 {
                st[j] = u64::from_str_radix(t[1 + j], 16).unwrap();
            }
            let mut m = [0u8; 64];
            for j in 0..64 {
                m[j] = u8::from_str_radix(t[7 + j], 16).unwrap();
            }
            let mut f = [0u8; 6];
            for j in 0..6 {
                f[j] = u8::from_str_radix(t[71 + j], 16).unwrap();
            }
            (st, m, f)
        })
        .collect()
}

fn check(group: &str, vs: Vec<Vector>) {
    let c = c_run(group, &vs);
    assert_eq!(c.len(), vs.len());
    let mut undefined = 0;
    for (v, (mut cst, cm, cf)) in vs.iter().zip(c) {
        let (mut hst, hm, hf) = host_run(v);
        cst[5] = 0;
        hst[5] = 0;
        assert_eq!(cst, hst, "{}: registers (C left, host right)", v.name);
        assert_eq!(cm, hm, "{}: memory", v.name);
        for k in 0..6 {
            if cf[k] == 0xff {
                undefined += 1;
                continue;
            }
            assert_eq!(cf[k], hf[k], "{}: flag {} (C left, host right)", v.name, ["cf", "pf", "af", "zf", "sf", "of"][k]);
        }
    }
    eprintln!("{group}: {} vectors, {undefined} flag values left undefined by the lifted code (not compared)", vs.len());
}

#[test]
fn lock_forms_match_host() {
    let all = vectors();
    // the vectors in four programs so that one failure names a small group
    let n = all.len();
    let mut groups: Vec<Vec<Vector>> = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for (k, x) in all.into_iter().enumerate() {
        groups[k * 4 / n].push(x);
    }
    for (g, vs) in groups.into_iter().enumerate() {
        check(&format!("lock_rmw{g}"), vs);
    }
}
