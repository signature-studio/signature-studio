//! `fe-decode jmp-reg FILE [--pdata]`: every `jmp reg` that the closure reaches, with its shape (the idiom of an MSVC switch table or
//! not) and its outcome (issue #1413). The shape is read from the decoded instructions in front of the jump (`jumptab::idiom_shape`,
//! or a table form that `recognize_msvc` names); the outcome from `Analysis::resolved`, `jumptabs`, `msvc_rejected` and
//! `jumptab_rejected`. `misrecognized` counts the table targets that are not a decoded instruction of the function of their table:
//! outside its `.pdata` pieces, not an instruction start, or inside another decoded instruction.

use crate::{indirect::Resolved, jumptab, Analysis};
use fe_pe::Pe;
use iced_x86::{FlowControl, OpKind};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub sites: usize,
    pub resolved_tables: usize,
    pub unrecognized_table_shape: usize,
    pub unrecognized_other: usize,
    pub misrecognized: usize,
}

pub fn report(pe: &Pe<'_>, a: &Analysis) -> Result<(Vec<String>, Summary), String> {
    let fm = jumptab::FuncMap::from_pe(pe)?;
    let mut lines = Vec::new();
    let mut s = Summary::default();
    for (&va, r) in &a.resolved {
        let i = &a.insns[&va];
        if i.flow_control() != FlowControl::IndirectBranch || i.op_kind(0) != OpKind::Register {
            continue;
        }
        s.sites += 1;
        let rva = pe.va_to_rva(va).unwrap_or(0);
        let reason = |list: &[(u64, String)]| list.iter().find(|(v, _)| *v == va).map(|(_, e)| e.clone());
        if let Some(t) = a.jumptabs.get(&va) {
            s.resolved_tables += 1;
            let function = match t.origin {
                jumptab::Origin::Msvc { function, .. } => Some(function),
                _ => None,
            };
            for &(_, tg) in &t.cases {
                let inside = match function {
                    Some(f) => pe.va_to_rva(tg).ok().and_then(|r| fm.function_at(r)).is_some_and(|(p, _)| p == f),
                    None => true,
                };
                let within = a.insns.range(..tg).next_back().is_some_and(|(&p, i)| p + i.len() as u64 > tg);
                if !inside || !a.insns.contains_key(&tg) || within {
                    s.misrecognized += 1;
                }
            }
            lines.push(format!("site {va:#x} rva {rva:#x} resolved {} cases {}", t.kind_key(), t.cases.len()));
            continue;
        }
        if *r != Resolved::Unresolved {
            lines.push(format!("site {va:#x} rva {rva:#x} resolved {r:?}"));
            continue;
        }
        let why = reason(&a.closure.msvc_rejected).or_else(|| reason(&a.closure.jumptab_rejected));
        let shape_table = jumptab::idiom_shape(&a.insns, va, i)
            || matches!(jumptab::recognize_msvc(&a.insns, va, i), Ok(Some(_)) | Err(_));
        if shape_table {
            s.unrecognized_table_shape += 1;
        } else {
            s.unrecognized_other += 1;
        }
        let shape = if shape_table { "table" } else { "other" };
        let why = why.unwrap_or_else(|| "no table idiom".to_string());
        lines.push(format!("site {va:#x} rva {rva:#x} unresolved shape {shape} | {why}"));
    }
    Ok((lines, s))
}
