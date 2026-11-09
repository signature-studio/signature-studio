//! Indirect `call` whose target is not known statically (`fe-decode` gave
//! `Unresolved`, or no entry at all).
//!
//! The target is read into a temporary before RSP changes, as on hardware, so
//! `call [rsp+8]` reads the same slot as before the push. `fle_dispatch(t)` then
//! calls the function whose entry VA is `t`, or aborts if `t` is not one. The
//! call returns to the next instruction, the same as a direct call.
//!
//! The table is emitted once, after all functions, by `table`.

use crate::{Ctl, Lifted, RSP};
use iced_x86::{Instruction, OpKind};
use std::collections::BTreeSet;

pub fn call(i: &Instruction, va: u64) -> Result<Lifted, String> {
    let target = match i.op_kind(0) {
        OpKind::Register => {
            if i.op_register(0).size() != 8 {
                return Err("indirect call register is not 64-bit".into());
            }
            crate::reg_read(i.op_register(0))?
        }
        OpKind::Memory => {
            if i.memory_size().size() != 8 {
                return Err("indirect call memory operand is not 64-bit".into());
            }
            format!("fle_ld({}, 8)", crate::ea(i)?)
        }
        _ => return Err("indirect call operand kind".into()),
    };
    let next = va + i.len() as u64;
    Ok(Lifted {
        body: format!(
            "{{ uint64_t t = {target}; R[{RSP}] -= 8; fle_st(R[{RSP}], 8, UINT64_C({next:#x})); fle_dispatch(t); R[{RSP}] += 8; }}"
        ),
        ctl: Ctl::Seq,
        handler: "call_ind:dispatch",
    })
}

/// Dispatch table for every function entry, in ascending VA order (the
/// header does a binary search over it).
pub fn table(entries: &BTreeSet<u64>) -> String {
    let mut s = String::from("const fle_fn_entry fle_dispatch_table[] = {\n");
    for &f in entries {
        s.push_str(&format!("    {{ UINT64_C({f:#x}), f_{f:x} }},\n"));
    }
    s.push_str(&format!("}};\nconst uint64_t fle_dispatch_count = {};\n\n", entries.len()));
    s
}
