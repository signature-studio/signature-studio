//! An annotated jump table (`fe_decode::jumptab`, decision 0036, W2) as a C `switch`.
//!
//! The value that selects the case is the one the hardware jumps with: the effective address of the
//! slot for `jmp [ea]` over 8-byte absolute VAs, or the jump register for `jmp reg` over 4-byte offsets
//! (the register already holds base + entry). Each case is a `goto` to the block of the target, or a tail
//! call when the target is a function entry. Any other value stops in `fle_jumptab_miss`.
//!
//! An MSVC switch table with a proven base (decision 0089) is the `jmp reg` form: the cases are the target VAs, the
//! register holds base + RVA, so an index above N, or a base that is not the image base, ends in `fle_jumptab_miss`.

use crate::{Ctl, Cx, Lifted, Stats};
use fe_decode::{JumpTable, Select};
use iced_x86::{Instruction, OpKind};

pub fn lift(i: &Instruction, va: u64, t: &JumpTable, cx: &Cx<'_>, st: &mut Stats) -> Result<Lifted, String> {
    let sel = match t.select {
        Select::Slot => {
            if i.op_kind(0) != OpKind::Memory || i.memory_size().size() != 8 {
                return Err("jump table: operand is not a 64-bit memory operand".into());
            }
            crate::ea(i)?
        }
        Select::Target => {
            if i.op_kind(0) != OpKind::Register || i.op_register(0).size() != 8 {
                return Err("jump table: operand is not a 64-bit register".into());
            }
            crate::reg_read(i.op_register(0))?
        }
    };
    let mut s = format!("{{ uint64_t t = {sel}; switch (t) {{ ");
    for &(case, target) in &t.cases {
        s.push_str(&format!("case UINT64_C({:#x}): {}", case, crate::succ(cx, target, None, st)));
    }
    s.push_str(&format!("default: fle_jumptab_miss(UINT64_C({va:#x}), t); }} }}"));
    let handler = match t.origin {
        fe_decode::Origin::Annotated => "jmp_ind:table",
        fe_decode::Origin::Stub { .. } => "jmp_ind:sel_table",
        fe_decode::Origin::Inline { .. } => "jmp_ind:inline_table",
        fe_decode::Origin::Msvc { .. } => "jmp_ind:msvc_table",
    };
    Ok(Lifted { body: s, ctl: Ctl::Ret, handler })
}
