//! `jmp r64` whose target `fe-decode` could not resolve (decision 0036, W1).
//!
//! The target is a function entry (or a pseudo-VA of the response table) only known at run time, so
//! the jump becomes a tail dispatch: `fle_dispatch(R[reg]); return;`. The guest return address stays
//! on the stack, as for a direct tail call, and the dispatched function's `ret` returns to our caller.
//! A value that is not an entry aborts in `fle_dispatch_miss`, as for an unresolved indirect call.
//!
//! The jump of a table (`jumptab.rs`) is not handled here.
//!
//! `jmp qword ptr [mem]` (decision 0100) is the same tail dispatch of the 64-bit value read from the effective address, which
//! is evaluated before anything changes: `{ fle_dispatch(fle_ld(ea, 8)); return; }`. It was left unsupported by decision 0036 (W1) because the
//! only unresolved site then, the `jmp [rcx]` that ends the `longjmp` of the guest, does not jump to a function entry; the value that is
//! not an entry still stops in `fle_dispatch_miss`, as for the register form.

use crate::{Ctl, Lifted};
use iced_x86::{Instruction, OpKind};

pub fn lift(i: &Instruction) -> Result<Lifted, String> {
    if i.op_kind(0) == OpKind::Memory {
        if i.memory_size().size() != 8 {
            return Err("indirect jmp memory operand is not 64-bit".into());
        }
        let t = format!("fle_ld({}, 8)", crate::ea(i)?);
        return Ok(Lifted { body: format!("{{ fle_dispatch({t}); return; }}"), ctl: Ctl::Ret, handler: "jmp_ind:mem_dispatch" });
    }
    if i.op_kind(0) != OpKind::Register {
        return Err("indirect target unresolved".into());
    }
    let r = i.op_register(0);
    if r.size() != 8 {
        return Err("indirect jmp register is not 64-bit".into());
    }
    let t = crate::reg_read(r)?;
    Ok(Lifted { body: format!("{{ fle_dispatch({t}); return; }}"), ctl: Ctl::Ret, handler: "jmp_ind:reg_dispatch" })
}
