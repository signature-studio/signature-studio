//! Integer instructions that do not write the status flags: movzx, movsxd,
//! nop, lea with a 32-bit address, and mov with a high-byte register.
//!
//! Width rules (SDM Vol. 1, general-purpose register writes): a 32-bit write
//! zero-extends to 64 bits; an 8- or 16-bit write keeps the other bits.
//! AH, BH, CH and DH are bits 8..15 of RAX, RBX, RCX and RDX.

use iced_x86::{Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

/// Lifts the instructions owned by this module. `None` leaves the instruction to `lib.rs`.
pub fn lift(i: &Instruction) -> Option<Lift> {
    match i.mnemonic() {
        Mnemonic::Movzx => Some(movzx(i)),
        Mnemonic::Movsxd => Some(movsxd(i)),
        Mnemonic::Nop => Some(Ok(("nop", String::new()))),
        Mnemonic::Lea if is_addr32(i) => Some(lea32(i)),
        Mnemonic::Mov if (0..i.op_count()).any(|k| is_high_op(i, k)) => Some(mov_high(i)),
        _ => None,
    }
}

fn movzx(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "movzx operand form")?;
    let d = i.op_register(0);
    need(matches!(d.size(), 2 | 4 | 8), "movzx destination size")?;
    let s = opsize(i, 1)?;
    need(matches!(s, 1 | 2), "movzx source size")?;
    Ok(("movzx", reg_wr(d, &rd(i, 1, s)?)?))
}

fn movsxd(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "movsxd operand form")?;
    let d = i.op_register(0);
    need(matches!(d.size(), 4 | 8), "movsxd destination size")?;
    need(opsize(i, 1)? == 4, "movsxd source size")?;
    let v = format!("(uint64_t)(int64_t)(int32_t)({})", rd(i, 1, 4)?);
    Ok(("movsxd", reg_wr(d, &v)?))
}

/// lea with a 32-bit address: the sum is computed modulo 2^32, then written to the destination.
fn lea32(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register, "lea operand form")?;
    let d = i.op_register(0);
    need(matches!(d.size(), 4 | 8), "lea destination size")?;
    Ok(("lea", reg_wr(d, &ea(i)?)?))
}

fn mov_high(i: &Instruction) -> Lift {
    need(i.op_count() == 2, "mov operand count")?;
    need(opsize(i, 0)? == 1, "mov high-byte destination size")?;
    if !is_imm(i.op_kind(1)) {
        need(opsize(i, 1)? == 1, "mov high-byte source size")?;
    }
    Ok(("mov", wr(i, 0, 1, &rd(i, 1, 1)?)?))
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn is_imm(k: OpKind) -> bool {
    matches!(
        k,
        OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64
    )
}

fn is_high(r: Register) -> bool {
    matches!(r, Register::AH | Register::BH | Register::CH | Register::DH)
}

fn is_high_op(i: &Instruction, k: u32) -> bool {
    i.op_kind(k) == OpKind::Register && is_high(i.op_register(k))
}

fn is_addr32(i: &Instruction) -> bool {
    i.op_count() == 2
        && i.op_kind(1) == OpKind::Memory
        && (is32(i.memory_base()) || is32(i.memory_index()))
}

fn is32(r: Register) -> bool {
    r != Register::None && r.size() == 4
}

fn reg_num(r: Register) -> Result<usize, String> {
    if !r.is_gpr() {
        return Err("non-GPR register".into());
    }
    Ok(r.full_register().number())
}

fn reg_rd(r: Register) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high(r) {
        return Ok(format!("((uint8_t)(R[{n}] >> 8))"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}]"),
        4 => format!("((uint32_t)R[{n}])"),
        2 => format!("((uint16_t)R[{n}])"),
        _ => format!("((uint8_t)R[{n}])"),
    })
}

fn reg_wr(r: Register, v: &str) -> Result<String, String> {
    let n = reg_num(r)?;
    if is_high(r) {
        return Ok(format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff00)) | ((uint64_t)(uint8_t)({v}) << 8);"));
    }
    Ok(match r.size() {
        8 => format!("R[{n}] = {v};"),
        4 => format!("R[{n}] = (uint32_t)({v});"),
        2 => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xffff)) | (uint16_t)({v});"),
        _ => format!("R[{n}] = (R[{n}] & ~UINT64_C(0xff)) | (uint8_t)({v});"),
    })
}

fn imm_lit(v: u64, size: usize) -> String {
    let m = if size >= 8 { u64::MAX } else { (1u64 << (size * 8)) - 1 };
    format!("UINT64_C({:#x})", v & m)
}

fn opsize(i: &Instruction, n: u32) -> Result<usize, String> {
    match i.op_kind(n) {
        OpKind::Register => {
            let r = i.op_register(n);
            reg_num(r)?;
            Ok(r.size())
        }
        OpKind::Memory => Ok(i.memory_size().size()),
        _ => Err("immediate operand has no size".into()),
    }
}

/// Source operand value as a C expression of width `size` (zero-extended).
fn rd(i: &Instruction, n: u32, size: usize) -> Result<String, String> {
    let k = i.op_kind(n);
    if k == OpKind::Register {
        reg_rd(i.op_register(n))
    } else if k == OpKind::Memory {
        Ok(format!("fle_ld({}, {size})", ea(i)?))
    } else if is_imm(k) {
        Ok(imm_lit(i.immediate(n), size))
    } else {
        Err("operand kind".into())
    }
}

fn wr(i: &Instruction, n: u32, size: usize, v: &str) -> Result<String, String> {
    match i.op_kind(n) {
        OpKind::Register => reg_wr(i.op_register(n), v),
        OpKind::Memory => Ok(format!("fle_st({}, {size}, {v});", ea(i)?)),
        _ => Err("operand kind".into()),
    }
}

/// Effective address as a C expression. Never loads from memory. With a 32-bit
/// address (the 0x67 prefix), the sum is truncated to 32 bits.
fn ea(i: &Instruction) -> Result<String, String> {
    if matches!(i.memory_segment(), Register::FS | Register::GS) {
        return Err("FS/GS segment".into());
    }
    if i.is_ip_rel_memory_operand() {
        need(i.memory_base() == Register::RIP, "EIP-relative addressing")?;
        return Ok(format!("UINT64_C({:#x})", i.ip_rel_memory_address()));
    }
    let (base, idx) = (i.memory_base(), i.memory_index());
    let mut terms: Vec<String> = Vec::new();
    if base != Register::None {
        terms.push(reg_rd(base)?);
    }
    if idx != Register::None {
        terms.push(format!("({}*{})", reg_rd(idx)?, i.memory_index_scale()));
    }
    let d = i.memory_displacement64();
    if d != 0 || terms.is_empty() {
        terms.push(format!("UINT64_C({d:#x})"));
    }
    let sum = terms.join(" + ");
    Ok(if is32(base) || is32(idx) { format!("((uint32_t)({sum}))") } else { format!("({sum})") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_x86::{Decoder, DecoderOptions};
    use std::fs;
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
        name: &'static str,
        bytes: &'static [u8],
        mnemonic: Mnemonic,
        regs: [u64; 6],
        /// rdi is replaced by the address of the 64-byte buffer + 16.
        mem: bool,
    }

    fn ins(name: &'static str, bytes: &'static [u8], mnemonic: Mnemonic, regs: [u64; 6]) -> Vector {
        Vector { name, bytes, mnemonic, regs, mem: false }
    }

    fn ins_mem(name: &'static str, bytes: &'static [u8], mnemonic: Mnemonic, regs: [u64; 6]) -> Vector {
        Vector { name, bytes, mnemonic, regs, mem: true }
    }

    fn pattern() -> [u8; 64] {
        let mut m = [0u8; 64];
        for (j, b) in m.iter_mut().enumerate().skip(16) {
            *b = ((j * 73 + 0x81) & 0xff) as u8;
        }
        m
    }

    fn movzx_vectors() -> Vec<Vector> {
        use Mnemonic::Movzx as M;
        vec![
            ins("movzx eax, bl", &[0x0F, 0xB6, 0xC3], M, [u64::MAX, 0, 0, 0xDEAD_BEEF_1234_5680, 0, 0]),
            ins("movzx eax, bh", &[0x0F, 0xB6, 0xC7], M, [u64::MAX, 0, 0, 0xDEAD_BEEF_1234_5680, 0, 0]),
            ins("movzx ecx, ah", &[0x0F, 0xB6, 0xCC], M, [0xFFFF_FFFF_1234_FF00, u64::MAX, 0, 0, 0, 0]),
            ins("movzx ecx, ah (ah=0)", &[0x0F, 0xB6, 0xCC], M, [0x1234_0012, u64::MAX, 0, 0, 0, 0]),
            ins("movzx rax, bl", &[0x48, 0x0F, 0xB6, 0xC3], M, [u64::MAX, 0, 0, 0xDEAD_BEEF_1234_5680, 0, 0]),
            ins("movzx esi, dil", &[0x40, 0x0F, 0xB6, 0xF7], M, [0, 0, 0, 0, u64::MAX, 0x1234_5699]),
            ins("movzx eax, dl", &[0x0F, 0xB6, 0xC2], M, [u64::MAX, 0, 0x7F, 0, 0, 0]),
            ins("movzx ax, bl", &[0x66, 0x0F, 0xB6, 0xC3], M, [0xFFFF_FFFF_FFFF_1234, 0, 0, 0xDEAD_BEEF_1234_5680, 0, 0]),
            ins("movzx cx, dl", &[0x66, 0x0F, 0xB6, 0xCA], M, [0, 0xAAAA_BBBB_CCCC_DDDD, 0xFE, 0, 0, 0]),
            ins_mem("movzx eax, word [rdi+6]", &[0x0F, 0xB7, 0x47, 0x06], M, [u64::MAX, 0, 0, 0, 0, 0]),
            ins_mem("movzx rcx, byte [rdi+1]", &[0x48, 0x0F, 0xB6, 0x4F, 0x01], M, [0, u64::MAX, 0, 0, 0, 0]),
            ins_mem("movzx edx, byte [rdi+3]", &[0x0F, 0xB6, 0x57, 0x03], M, [0, 0, u64::MAX, 0, 0, 0]),
            ins_mem("movzx eax, byte [rdi]", &[0x0F, 0xB6, 0x07], M, [u64::MAX, 0, 0, 0, 0, 0]),
            ins_mem("movzx ax, byte [rdi+4]", &[0x66, 0x0F, 0xB6, 0x47, 0x04], M, [u64::MAX, 0, 0, 0, 0, 0]),
        ]
    }

    fn movsxd_vectors() -> Vec<Vector> {
        use Mnemonic::Movsxd as M;
        vec![
            ins("movsxd rax, ebx (negative)", &[0x48, 0x63, 0xC3], M, [0, 0, 0, 0x0000_0000_8000_0000, 0, 0]),
            ins("movsxd rcx, edx (positive)", &[0x48, 0x63, 0xCA], M, [0, 0, 0xFFFF_FFFF_7FFF_FFFF, 0, 0, 0]),
            ins("movsxd rsi, edi (-1)", &[0x48, 0x63, 0xF7], M, [0, 0, 0, 0, 0, 0x0000_0000_FFFF_FFFF]),
            ins("movsxd eax, ebx (32-bit dst)", &[0x63, 0xC3], M, [u64::MAX, 0, 0, 0x1_8000_0001, 0, 0]),
            ins("movsxd eax, ebx (positive, 32-bit dst)", &[0x63, 0xC3], M, [u64::MAX, 0, 0, 0x0000_0000_7FFF_FFFF, 0, 0]),
            ins_mem("movsxd rax, dword [rdi+4]", &[0x48, 0x63, 0x47, 0x04], M, [0, 0, 0, 0, 0, 0]),
            ins_mem("movsxd rdx, dword [rdi+1]", &[0x48, 0x63, 0x57, 0x01], M, [0, 0, 0, 0, 0, 0]),
        ]
    }

    fn nop_vectors() -> Vec<Vector> {
        use Mnemonic::Nop as M;
        let r = [1, 2, 3, 4, 5, 6];
        vec![
            ins("nop", &[0x90], M, r),
            ins("nop dword [rax]", &[0x0F, 0x1F, 0x00], M, r),
            ins("nop dword [rax+rax*1+0]", &[0x0F, 0x1F, 0x44, 0x00, 0x00], M, r),
            ins("nop word [rax+rax*1+0]", &[0x66, 0x0F, 0x1F, 0x44, 0x00, 0x00], M, r),
            ins("nop dword [rax+disp32]", &[0x0F, 0x1F, 0x80, 0x00, 0x00, 0x00, 0x00], M, r),
            ins("cs nop word [rax+rax*1+disp32]", &[0x66, 0x2E, 0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00], M, r),
            ins_mem("nop dword [rdi+0x12]", &[0x0F, 0x1F, 0x47, 0x12], M, r),
        ]
    }

    fn lea32_vectors() -> Vec<Vector> {
        use Mnemonic::Lea as M;
        vec![
            ins("lea eax, [ebx]", &[0x67, 0x8D, 0x03], M, [u64::MAX, 0, 0, 0xFFFF_FFFF_0000_0010, 0, 0]),
            ins("lea eax, [ebx+ecx*4+8]", &[0x67, 0x8D, 0x44, 0x8B, 0x08], M, [0, 3, 0, 0x10, 0, 0]),
            ins("lea rax, [ebx+ecx*2-1] (wraps to 32 bits)", &[0x67, 0x48, 0x8D, 0x44, 0x4B, 0xFF], M, [u64::MAX, 0, 0, 0, 0, 0]),
            ins("lea edx, [esi+edi*8+0x12345]", &[0x67, 0x8D, 0x94, 0xFE, 0x45, 0x23, 0x01, 0x00], M, [0, 0, u64::MAX, 0, 0xFFFF_FFF0, 0x1]),
            ins("lea eax, [ebx+ecx*4] (wraps)", &[0x67, 0x8D, 0x04, 0x8B], M, [0, 8, 0, 0xFFFF_FFF0, 0, 0]),
            ins("lea rax, [ebx] (0x8000_0000, zero-extended)", &[0x67, 0x48, 0x8D, 0x03], M, [0, 0, 0, 0x8000_0000, 0, 0]),
            ins("lea eax, [ebx-1] (0 - 1)", &[0x67, 0x8D, 0x43, 0xFF], M, [0, 0, 0, 0, 0, 0]),
        ]
    }

    fn mov_high_vectors() -> Vec<Vector> {
        use Mnemonic::Mov as M;
        vec![
            ins("mov ah, bl", &[0x88, 0xDC], M, [0x1122_3344_5566_7788, 0, 0, 0xAB, 0, 0]),
            ins("mov ah, bl (bl=0)", &[0x88, 0xDC], M, [u64::MAX, 0, 0, 0, 0, 0]),
            ins("mov bl, ah", &[0x8A, 0xDC], M, [0x0000_0000_0000_CD00, 0, 0, 0x1122_3344_5566_7788, 0, 0]),
            ins("mov ch, al", &[0x88, 0xC5], M, [0x12, 0x1122_3344_5566_7788, 0, 0, 0, 0]),
            ins("mov bh, cl", &[0x88, 0xCF], M, [0, 0x80, 0, 0x1122_3344_5566_7788, 0, 0]),
            ins("mov ah, bh", &[0x88, 0xFC], M, [0x1122_3344_5566_7788, 0, 0, 0x0000_0000_0000_FE00, 0, 0]),
            ins("mov ah, 0x5a", &[0xB4, 0x5A], M, [u64::MAX, 0, 0, 0, 0, 0]),
            ins_mem("mov dh, [rdi+1]", &[0x8A, 0x77, 0x01], M, [0, 0, 0x1122_3344_5566_7788, 0, 0, 0]),
            ins_mem("mov [rdi+2], ah", &[0x88, 0x67, 0x02], M, [0xFFFF_FFFF_FFFF_5A00, 0, 0, 0, 0, 0]),
            ins_mem("mov ah, [rdi]", &[0x8A, 0x27], M, [0x1122_3344_5566_7788, 0, 0, 0, 0, 0]),
        ]
    }

    fn lift_body(v: &Vector) -> String {
        let i = Decoder::with_ip(64, v.bytes, 0x1000, DecoderOptions::NONE).decode();
        assert!(!i.is_invalid(), "{}: invalid encoding", v.name);
        assert_eq!(i.len(), v.bytes.len(), "{}: length", v.name);
        assert_eq!(i.mnemonic(), v.mnemonic, "{}: mnemonic", v.name);
        match lift(&i) {
            Some(Ok((_, body))) => body,
            other => panic!("{}: not lifted: {other:?}", v.name),
        }
    }

    extern "C" {
        fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
        fn munmap(addr: *mut u8, len: usize) -> i32;
    }

    /// Runs the instruction on the host CPU. Ground truth for the C output.
    fn host_run(v: &Vector) -> ([u64; 6], [u8; 64]) {
        let mut code = vec![0x55, 0x48, 0x89, 0xFD];
        for l in LOAD {
            code.extend_from_slice(&l);
        }
        code.extend_from_slice(v.bytes);
        for s in STORE {
            code.extend_from_slice(&s);
        }
        code.extend_from_slice(&[0x5D, 0xC3]);
        let buf = pattern();
        let mut st = v.regs;
        if v.mem {
            st[5] = buf.as_ptr() as u64 + 16;
        }
        unsafe {
            let p = mmap(std::ptr::null_mut(), 4096, 7, 0x22, -1, 0);
            assert!(!p.is_null() && p as isize != -1, "mmap failed");
            std::ptr::copy_nonoverlapping(code.as_ptr(), p, code.len());
            let f = std::mem::transmute::<*mut u8, extern "C" fn(*mut u64)>(p);
            f(st.as_mut_ptr());
            munmap(p, 4096);
        }
        (st, buf)
    }

    /// Compiles one C program with clang -O0 that runs every vector, and returns the results.
    fn c_run(group: &str, vs: &[Vector]) -> Vec<([u64; 6], [u8; 64])> {
        let p = pattern();
        let mut src = String::from("#include \"fle_runtime.h\"\n\nuint64_t R[16];\nfle_eflags FL;\nstatic uint8_t M[64];\n");
        src += "static const uint8_t P[64] = {";
        src += &p.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
        src += "};\n\n";
        for (k, v) in vs.iter().enumerate() {
            src += &format!("static void c{k}(void)\n{{\n{}\n}}\n\n", lift_body(v));
        }
        src += "int main(void)\n{\n";
        for (k, v) in vs.iter().enumerate() {
            src += "    memcpy(M, P, sizeof M);\n";
            for (j, r) in REG_NUMS.iter().enumerate() {
                src += &format!("    R[{r}] = UINT64_C({:#x});\n", v.regs[j]);
            }
            if v.mem {
                src += "    R[7] = (uint64_t)(uintptr_t)(M + 16);\n";
            }
            src += &format!("    c{k}();\n    printf(\"%d\", {k});\n");
            for r in REG_NUMS {
                src += &format!("    printf(\" %llx\", (unsigned long long)R[{r}]);\n");
            }
            src += "    for (int j = 0; j < 64; j++) printf(\" %02x\", M[j]);\n    printf(\"\\n\");\n";
        }
        src += "    return 0;\n}\n";

        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/insn-int-test");
        let rt = concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtime");
        fs::create_dir_all(dir).unwrap();
        let c = format!("{dir}/{group}.c");
        let exe = format!("{dir}/{group}");
        fs::write(&c, &src).unwrap();
        let cc = Command::new("clang")
            .args(["-O0", "-std=c11", "-I", rt, "-o", &exe, &c])
            .output()
            .expect("clang not found");
        assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
        let run = Command::new(&exe).output().expect("run failed");
        assert!(run.status.success(), "C program failed");
        let out = String::from_utf8(run.stdout).unwrap();
        out.lines()
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
                (st, m)
            })
            .collect()
    }

    fn check(group: &str, vs: &[Vector]) {
        let c = c_run(group, vs);
        assert_eq!(c.len(), vs.len());
        for (v, (mut cst, cm)) in vs.iter().zip(c) {
            let (mut hst, hm) = host_run(v);
            if v.mem {
                cst[5] = 0;
                hst[5] = 0;
            }
            assert_eq!(cst, hst, "{}: registers (C left, host right)", v.name);
            assert_eq!(cm, hm, "{}: memory", v.name);
        }
    }

    #[test]
    fn movzx_matches_host() {
        check("movzx", &movzx_vectors());
    }

    #[test]
    fn movsxd_matches_host() {
        check("movsxd", &movsxd_vectors());
    }

    #[test]
    fn nop_matches_host() {
        check("nop", &nop_vectors());
    }

    #[test]
    fn lea32_matches_host() {
        check("lea32", &lea32_vectors());
    }

    #[test]
    fn mov_high_byte_matches_host() {
        check("mov_high", &mov_high_vectors());
    }

    #[test]
    fn other_forms_are_left_to_lib() {
        let cases: [&[u8]; 3] = [&[0x01, 0xC8], &[0x89, 0xC8], &[0x48, 0x8D, 0x03]];
        for bytes in cases {
            let i = Decoder::with_ip(64, bytes, 0x1000, DecoderOptions::NONE).decode();
            assert!(lift(&i).is_none(), "{:02x?} should not be claimed", bytes);
        }
    }
}
