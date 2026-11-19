//! PREFETCHT0, PREFETCHT1, PREFETCHT2, PREFETCHNTA (decision 0038), PREFETCHW (decision 0156: OTT.vst3 has one, in its MSVC STL; SDM Vol. 2 PREFETCHW "Move data from m8 closer to the processor in anticipation of a write": a hint with no architectural effect, so the same body).
//!
//! SDM Vol. 2 (PREFETCHh): "merely a hint and does not affect program behavior". The effective address is
//! computed (from registers only, nothing is loaded or stored) and discarded, so an address that is not mapped
//! cannot fault. No register, flag or memory changes. Declared from sse_more.rs with `#[path]`.

use iced_x86::{Instruction, Mnemonic, OpKind};

type Lift = Result<(&'static str, String), String>;

pub fn lift(i: &Instruction) -> Option<Lift> {
    let name = match i.mnemonic() {
        Mnemonic::Prefetcht0 => "prefetcht0",
        Mnemonic::Prefetcht1 => "prefetcht1",
        Mnemonic::Prefetcht2 => "prefetcht2",
        Mnemonic::Prefetchnta => "prefetchnta",
        Mnemonic::Prefetchw => "prefetchw",
        _ => return None,
    };
    Some(body(i, name))
}

fn body(i: &Instruction, name: &'static str) -> Lift {
    if i.op_count() != 1 || i.op_kind(0) != OpKind::Memory {
        return Err("prefetch operand must be m8".into());
    }
    let a = crate::ea(i)?;
    Ok((name, format!("(void)({a});")))
}
