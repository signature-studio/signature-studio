//! Register and status-flag effects of instructions.

use iced_x86::{ConditionCode, Instruction, InstructionInfoFactory, Mnemonic, OpAccess, OpKind, Register};

pub const RSP: usize = 4;
pub const RAX: usize = 0;
pub const RCX: usize = 1;
pub const RDX: usize = 2;

/// The mask of general registers other than RSP (RSP is a parameter of every function and is not tracked by the liveness).
pub const ALL_REGS: u16 = 0xffff & !(1 << RSP);

pub const CF: u8 = 1;
pub const PF: u8 = 2;
pub const AF: u8 = 4;
pub const ZF: u8 = 8;
pub const SF: u8 = 16;
pub const OF: u8 = 32;
pub const ALL_FLAGS: u8 = 63;

pub fn flag_index(f: u8) -> usize {
    f.trailing_zeros() as usize
}

pub fn gpr(r: Register) -> Option<usize> {
    if r.is_gpr() {
        Some(r.full_register().number())
    } else {
        None
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Eff {
    /// Registers read (RSP excluded). A write to an 8/16-bit register or a conditional write also reads the register.
    pub reads: u16,
    /// Registers fully overwritten on every path (RSP excluded).
    pub defs: u16,
    /// Registers written on some path (including partial and conditional writes).
    pub writes: u16,
    /// XMM registers read, fully written, and written on some path (a scalar operation keeps the upper lanes: it reads its destination).
    pub xr: u16,
    pub xd: u16,
    pub xw: u16,
    pub flags_r: u8,
    /// Flags the instruction modifies (written, cleared, set or left undefined).
    pub flags_w: u8,
    /// Flags that hold a defined value afterwards (written, cleared or set).
    pub flags_def: u8,
}

fn map_flags(b: u32) -> u8 {
    use iced_x86::RflagsBits as R;
    let mut m = 0u8;
    if b & R::CF != 0 {
        m |= CF;
    }
    if b & R::PF != 0 {
        m |= PF;
    }
    if b & R::AF != 0 {
        m |= AF;
    }
    if b & R::ZF != 0 {
        m |= ZF;
    }
    if b & R::SF != 0 {
        m |= SF;
    }
    if b & R::OF != 0 {
        m |= OF;
    }
    m
}

pub fn cc_flags(cc: ConditionCode) -> u8 {
    use ConditionCode::*;
    match cc {
        o | no => OF,
        b | ae => CF,
        e | ne => ZF,
        be | a => CF | ZF,
        s | ns => SF,
        p | np => PF,
        l | ge => SF | OF,
        le | g => ZF | SF | OF,
        None => 0,
    }
}

pub struct Fx {
    fac: InstructionInfoFactory,
}

impl Fx {
    pub fn new() -> Self {
        Fx { fac: InstructionInfoFactory::new() }
    }

    /// True when the instruction writes RSP other than through push/pop/call/ret.
    pub fn writes_rsp(&mut self, i: &Instruction) -> bool {
        use Mnemonic as M;
        if matches!(i.mnemonic(), M::Push | M::Pop | M::Call | M::Ret | M::Leave | M::Nop) {
            return false;
        }
        self.fac.info(i).used_registers().iter().any(|ur| {
            gpr(ur.register()) == Some(RSP) && !matches!(ur.access(), OpAccess::Read | OpAccess::CondRead | OpAccess::NoMemAccess)
        })
    }

    pub fn eff(&mut self, i: &Instruction) -> Eff {
        let mut e = Eff::default();
        if i.mnemonic() == Mnemonic::Nop {
            return e;
        }
        let info = self.fac.info(i);
        let full_w = xmm_full_write(i);
        for ur in info.used_registers() {
            let r = ur.register();
            if r.is_xmm() {
                let bit = 1u16 << r.number();
                match ur.access() {
                    OpAccess::Read | OpAccess::CondRead => e.xr |= bit,
                    OpAccess::Write => {
                        e.xw |= bit;
                        if full_w {
                            e.xd |= bit;
                        } else {
                            e.xr |= bit;
                        }
                    }
                    _ => {
                        e.xr |= bit;
                        e.xw |= bit;
                    }
                }
                continue;
            }
            let Some(n) = gpr(r) else { continue };
            if n == RSP {
                continue;
            }
            let bit = 1u16 << n;
            let full = r.size() >= 4;
            if !matches!(ur.access(), OpAccess::Read | OpAccess::CondRead | OpAccess::NoMemAccess) {
                e.writes |= bit;
            }
            match ur.access() {
                OpAccess::Read | OpAccess::CondRead => e.reads |= bit,
                OpAccess::Write => {
                    if full {
                        e.defs |= bit;
                    } else {
                        e.reads |= bit;
                    }
                }
                OpAccess::CondWrite => e.reads |= bit,
                OpAccess::ReadWrite => {
                    e.reads |= bit;
                    if full {
                        e.defs |= bit;
                    }
                }
                OpAccess::ReadCondWrite => e.reads |= bit,
                OpAccess::NoMemAccess => e.reads |= bit,
                OpAccess::None => {}
            }
        }
        // scalar conversions and register-to-register scalar moves replace the low lane and keep the rest: the old value of the destination is
        // not an input of the value that matters (the upper lanes of an undefined register are garbage anyway)
        {
            use Mnemonic as M;
            if i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm() {
                let weak = match i.mnemonic() {
                    M::Cvtsi2sd | M::Cvtsi2ss | M::Cvtsd2ss | M::Cvtss2sd | M::Sqrtsd | M::Sqrtss => true,
                    M::Movsd | M::Movss => i.op_kind(1) == OpKind::Register,
                    _ => false,
                };
                if weak && !crate::xabi::XMEM.load(std::sync::atomic::Ordering::Relaxed) && i.op_register(0) != i.op_register(1) {
                    e.xr &= !(1u16 << i.op_register(0).number());
                    e.xd |= 1u16 << i.op_register(0).number();
                }
            }
        }
        // idioms that do not depend on the old value of the register
        {
            use Mnemonic as M;
            let dst32 = i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_register(0).size() >= 4 && gpr(i.op_register(0)).map_or(false, |n| n != RSP);
            if dst32 {
                let bit = 1u16 << gpr(i.op_register(0)).unwrap();
                let same = i.op_kind(1) == OpKind::Register && i.op_register(0) == i.op_register(1);
                let imm = matches!(i.op_kind(1), OpKind::Immediate8 | OpKind::Immediate8to32 | OpKind::Immediate8to64 | OpKind::Immediate32 | OpKind::Immediate32to64);
                let all_ones = imm && (if i.op_register(0).size() == 8 { i.immediate(1) == u64::MAX } else { i.immediate(1) as u32 == u32::MAX });
                let zero = imm && i.immediate(1) == 0;
                if (matches!(i.mnemonic(), M::Xor | M::Sub) && same) || (i.mnemonic() == M::Or && all_ones) || (i.mnemonic() == M::And && zero) {
                    e.reads &= !bit;
                    e.defs |= bit;
                }
            }
        }
        e.flags_r = map_flags(i.rflags_read());
        e.flags_w = map_flags(i.rflags_modified());
        e.flags_def = map_flags(i.rflags_written() | i.rflags_cleared() | i.rflags_set()) & !map_flags(i.rflags_undefined());
        e
    }
}

/// True when the instruction has a memory operand (explicit).
/// Instructions that overwrite all 128 bits of the XMM destination (the others keep the upper lanes, or are not modelled).
pub fn xmm_full_write(i: &Instruction) -> bool {
    use Mnemonic as M;
    match i.mnemonic() {
        M::Movups | M::Movaps | M::Movdqa | M::Movdqu | M::Movapd | M::Movupd | M::Lddqu => true,
        M::Cvtdq2ps | M::Cvtdq2pd | M::Cvtps2pd | M::Cvtpd2ps | M::Cvttpd2dq | M::Cvtpd2dq | M::Sqrtpd | M::Sqrtps => i.op_kind(0) == OpKind::Register,
        M::Movd | M::Movq => i.op_kind(0) == OpKind::Register && i.op_register(0).is_xmm(),
        M::Movsd | M::Movss => i.op_count() == 2 && i.op_kind(1) == OpKind::Memory,
        M::Xorps | M::Xorpd | M::Pxor | M::Psubb | M::Psubd => {
            i.op_count() == 2 && i.op_kind(0) == OpKind::Register && i.op_kind(1) == OpKind::Register && i.op_register(0) == i.op_register(1)
        }
        _ => false,
    }
}

pub fn has_mem(i: &Instruction) -> bool {
    (0..i.op_count()).any(|k| i.op_kind(k) == OpKind::Memory)
}

pub fn is_nop(m: Mnemonic) -> bool {
    matches!(m, Mnemonic::Nop | Mnemonic::Fnop | Mnemonic::Pause)
}
