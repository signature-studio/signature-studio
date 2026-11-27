//! Seeds from the base relocation directory (decision 0074).
//!
//! A DIR64 base relocation marks an 8-byte slot that holds an address. Where that address lies in an
//! executable range (`Text`) and equals a `.pdata` function start (`fe_pe::Pe::function_starts`, chained
//! entries excluded), it is a seed. A value in an executable range that is not a `.pdata` start is not
//! taken: it is counted and listed in `not_function` only. This is not the adjustor-thunk rule of
//! decision 0051 (`adjustor`), which looks at the same slots for values that are not `.pdata` starts and
//! takes only the short `add`/`jmp` shape; the two sets are disjoint.

use crate::Text;
use fe_pe::Pe;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelocSeeds {
    /// DIR64 slots in the base relocation directory.
    pub slots: usize,
    /// Slots whose 8 bytes are not in the file (no seed, no count).
    pub unreadable_slots: usize,
    /// Distinct slot values that lie in an executable range.
    pub in_code: usize,
    /// `in_code` values that are a `.pdata` function start, ascending. These are the seeds.
    pub seeds: Vec<u64>,
    /// `in_code` values that are not a `.pdata` function start, ascending. Not seeds.
    pub not_function: Vec<u64>,
}

pub fn reloc_seeds(pe: &Pe<'_>) -> Result<RelocSeeds, String> {
    let text = Text::from_pe(pe)?;
    let starts: BTreeSet<u64> = pe.function_start_vas().map_err(|e| format!("pdata: {e}"))?.into_iter().collect();
    let slots = pe.dir64_relocation_vas().map_err(|e| format!("reloc: {e}"))?;
    let mut out = RelocSeeds { slots: slots.len(), ..Default::default() };
    let mut values: BTreeSet<u64> = BTreeSet::new();
    for slot in slots {
        let bytes = pe.va_to_rva(slot).ok().and_then(|rva| pe.slice_at_rva(rva, 8).ok());
        let Some(b) = bytes else {
            out.unreadable_slots += 1;
            continue;
        };
        let v = u64::from_le_bytes(b.try_into().expect("8 bytes"));
        if text.contains(v) {
            values.insert(v);
        }
    }
    out.in_code = values.len();
    for v in values {
        if starts.contains(&v) {
            out.seeds.push(v);
        } else {
            out.not_function.push(v);
        }
    }
    Ok(out)
}

/// `reloc_seeds(pe).seeds`, ascending.
pub fn reloc_pdata_seeds(pe: &Pe<'_>) -> Result<Vec<u64>, String> {
    Ok(reloc_seeds(pe)?.seeds)
}

/// Issue #1395 (`--rva-tables`): the rows of a table file, `RVA COUNT` (hex RVA, decimal count; `#` comments). Each names a table of
/// `COUNT` 32-bit values that hold RVAs from the ImageBase (the switch tables of MSVC code that jumps to `ImageBase + entry`, whose
/// targets are other functions, so the analysis has no function entry for them).
pub fn rva_tables(text: &str) -> Result<Vec<(u32, usize)>, String> {
    let mut out = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let rva = f.first().and_then(|r| u32::from_str_radix(r.trim_start_matches("0x"), 16).ok());
        let n = f.get(1).and_then(|n| n.parse::<usize>().ok());
        let (Some(rva), Some(n), 2) = (rva, n, f.len()) else {
            return Err(format!("bad table row: {l}"));
        };
        out.push((rva, n));
    }
    Ok(out)
}

/// The code addresses that the entries of the tables of `rva_tables` hold (ImageBase + entry, inside `.text`), ascending.
pub fn rva_table_seeds(pe: &Pe<'_>, tables: &[(u32, usize)]) -> Result<Vec<u64>, String> {
    let text = Text::from_pe(pe)?;
    let base = pe.optional.image_base;
    let mut out = BTreeSet::new();
    for &(rva, n) in tables {
        let raw = pe.slice_at_rva(rva, 4 * n).map_err(|e| format!("table {rva:#x}: {e}"))?;
        for k in 0..n {
            let v = u32::from_le_bytes(raw[4 * k..4 * k + 4].try_into().unwrap());
            let va = base + v as u64;
            if text.contains(va) {
                out.insert(va);
            }
        }
    }
    Ok(out.into_iter().collect())
}
