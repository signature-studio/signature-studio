//! VEX-encoded moves and VZEROUPPER, and the FS/GS forms of LEA and MOV that the closure
//! contains (decision 0032).
//!
//! Lifted: VMOVUPS, VMOVAPS (xmm or ymm register, register copy, 128/256-bit load and
//! store), VMOVNTPS (store), VZEROUPPER; `lea r, fs:[...]` (the segment does not take part
//! in LEA); `mov r64, gs:[0x30]`, `mov r64, gs:[0x58]` and `mov r64, gs:[0x10]` (StackLimit,
//! decision 0076; reads only, a store to gs:[0x10] stays unsupported). Everything else with an FS or GS
//! memory operand, and the EVEX encodings, stay unsupported (`Err`).
//!
//! Semantics: Intel SDM Vol. 2 (325383-093US), the VEX.128 and VEX.256 pseudo-code of each
//! instruction. A VEX.128 write zeroes bits MAXVL-1:128 of the destination, a VEX.256
//! write sets bits 255:0 and zeroes the rest, a store changes no register. The helpers
//! are in runtime/fle_vex.h: YMMn is the pair (X[n], YH[n]).
//! Alignment (SDM): VMOVAPS and VMOVNTPS need a 16-byte (xmm) or 32-byte (ymm) aligned
//! memory operand (#GP otherwise: the generated C aborts); VMOVUPS has no alignment rule.
//! No flags change.

use iced_x86::{EncodingKind, Instruction, Mnemonic, OpKind, Register};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let m = i.mnemonic();
    if matches!(m, Mnemonic::Vmovups | Mnemonic::Vmovaps | Mnemonic::Vmovntps | Mnemonic::Vzeroupper) {
        if i.encoding() != EncodingKind::VEX {
            return Some(Err("EVEX encoding".into()));
        }
        return Some(match m {
            Mnemonic::Vmovups => mov(i, "vmovups", false, true),
            Mnemonic::Vmovaps => mov(i, "vmovaps", true, true),
            Mnemonic::Vmovntps => mov(i, "vmovntps", true, false),
            _ => zeroupper(i),
        });
    }
    if m == Mnemonic::Lea && has_fsgs_mem(i) {
        return Some(lea(i));
    }
    if m == Mnemonic::Mov && has_fsgs_mem(i) {
        return Some(mov_seg(i));
    }
    None
}

fn need(c: bool, msg: &str) -> Result<(), String> {
    if c {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn has_fsgs_mem(i: &Instruction) -> bool {
    (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Memory) && matches!(i.memory_segment(), Register::FS | Register::GS)
}

/// (register number, wide) of an XMM or YMM register 0-15.
fn vreg(r: Register) -> Result<(usize, bool), String> {
    need((r.is_xmm() || r.is_ymm()) && r.number() < 16, "register is not XMM0-15 or YMM0-15")?;
    Ok((r.number(), r.is_ymm()))
}

/// Effective address of the memory operand as a C expression. Never loads from memory.
/// `seg_ok`: the caller (LEA) ignores the segment, so FS/GS is allowed.
fn ea(i: &Instruction, seg_ok: bool) -> Result<String, String> {
    need(seg_ok || !matches!(i.memory_segment(), Register::FS | Register::GS), "FS/GS segment")?;
    if i.is_ip_rel_memory_operand() {
        return Ok(format!("UINT64_C({:#x})", i.ip_rel_memory_address()));
    }
    let mut terms: Vec<String> = Vec::new();
    let base = i.memory_base();
    if base != Register::None {
        need(base.size() == 8, "32-bit addressing")?;
        terms.push(format!("R[{}]", base.full_register().number()));
    }
    let idx = i.memory_index();
    if idx != Register::None {
        need(idx.size() == 8, "32-bit addressing")?;
        terms.push(format!("(R[{}]*{})", idx.full_register().number(), i.memory_index_scale()));
    }
    let d = i.memory_displacement64();
    if d != 0 || terms.is_empty() {
        terms.push(format!("UINT64_C({d:#x})"));
    }
    Ok(format!("({})", terms.join(" + ")))
}

/// VMOVUPS, VMOVAPS (`reg_ok`), VMOVNTPS (store only). `aligned`: the memory operand must be
/// aligned to its size.
fn mov(i: &Instruction, name: &'static str, aligned: bool, reg_ok: bool) -> Lift {
    need(i.op_count() == 2, "operand count")?;
    let al = u8::from(aligned);
    let body = match (i.op_kind(0), i.op_kind(1)) {
        (OpKind::Register, OpKind::Memory) => {
            need(reg_ok, "load form")?;
            let (d, wide) = vreg(i.op_register(0))?;
            need(i.memory_size().size() == if wide { 32 } else { 16 }, "memory operand size")?;
            format!("fle_vld({d}, {}, {}, {al});", ea(i, false)?, u8::from(wide))
        }
        (OpKind::Memory, OpKind::Register) => {
            let (s, wide) = vreg(i.op_register(1))?;
            need(i.memory_size().size() == if wide { 32 } else { 16 }, "memory operand size")?;
            format!("fle_vst({}, {s}, {}, {al});", ea(i, false)?, u8::from(wide))
        }
        (OpKind::Register, OpKind::Register) => {
            need(reg_ok, "register form")?;
            let (d, wd) = vreg(i.op_register(0))?;
            let (s, ws) = vreg(i.op_register(1))?;
            need(wd == ws, "register width mismatch")?;
            format!("fle_vcopy({d}, {s}, {});", u8::from(wd))
        }
        _ => return Err("operand form".into()),
    };
    Ok((name, body))
}

fn zeroupper(i: &Instruction) -> Lift {
    need(i.op_count() == 0, "operand count")?;
    Ok(("vzeroupper", "fle_vzeroupper();".to_string()))
}

/// LEA with an FS or GS prefix. SDM LEA: the source is "a memory address (offset part)", so
/// the segment base is not added. Same destination rules as the generic LEA in lib.rs.
fn lea(i: &Instruction) -> Lift {
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Memory, "lea operand form")?;
    let d = i.op_register(0);
    need(d.is_gpr64() || d.is_gpr32(), "lea destination size")?;
    let e = ea(i, true)?;
    let n = d.full_register().number();
    let body = if d.size() == 8 { format!("R[{n}] = {e};") } else { format!("R[{n}] = (uint32_t)({e});") };
    Ok(("lea_seg", body))
}

/// `mov r64, gs:[disp32]` with no base and no index, for the two TEB offsets of
/// runtime/fle_vex.h. Anything else through FS or GS is not modeled.
fn mov_seg(i: &Instruction) -> Lift {
    need(i.memory_segment() == Register::GS, "FS segment")?;
    need(i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Memory, "gs mov operand form")?;
    let d = i.op_register(0);
    need(d.is_gpr64() && i.memory_size().size() == 8, "gs mov must be a 64-bit load")?;
    need(i.memory_base() == Register::None && i.memory_index() == Register::None, "gs access with base or index")?;
    need(!i.is_ip_rel_memory_operand(), "gs access is rip-relative")?;
    let off = i.memory_displacement64();
    need(off == 0x30 || off == 0x58 || off == 0x10, "gs offset is not modeled (only 0x30, 0x58 and 0x10)")?;
    Ok(("mov_gs", format!("R[{}] = fle_gs_ld8(UINT64_C({off:#x}));", d.full_register().number())))
}
