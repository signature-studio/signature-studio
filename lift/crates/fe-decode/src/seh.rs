//! The except landings of the Delphi exception handler (decision 0132, `docs/investigations/seh-model.md`).
//!
//! FLEngine_x64.dll (and QFC, ILTools) have ONE language-specific handler (RVA 0x4112c0 in FLEngine; 0x40e590 and 0x40cc60 in QFC and
//! ILTools, the same 452 instructions) for every function with an unwind handler. The handler's data, which follows the handler RVA in
//! the UNWIND_INFO, is a scope table (the shape the handler reads; reading of its code, not a Microsoft format):
//! ```text
//! u32 count;  count x { u32 begin, end, f2, f3 }            (RVAs; begin <= pc < end)
//!   f2 == 0   try..finally : f3 = the RVA of the finally funclet (a .pdata function)
//!   f2 == 2   try..except (any exception) : f3 = the RVA of the landing, inside the function
//!   f2 >  2   try..except on ... : f2 = the RVA of  u32 count; count x { u32 type-cell RVA, u32 landing RVA }
//! ```
//! This module lists the landings (f2 == 2 and the `on` tables) of the functions whose UNWIND_INFO names a given handler RVA. The caller
//! gives the handler RVAs: nothing here decides that a handler is the Delphi one. Every range is checked (`begin < end` inside the
//! function, a landing inside the function and inside its scope's function); one that fails is an error, not a guess.

use fe_pe::Pe;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Clone)]
pub struct SehLandings {
    /// Function start VA (the primary RUNTIME_FUNCTION's BeginAddress) -> its landings (VA, ascending, distinct).
    pub by_func: BTreeMap<u64, Vec<u64>>,
    /// Functions with the handler (finally-only ones included).
    pub handler_functions: usize,
    pub scopes: usize,
    pub finally_scopes: usize,
    pub bare_except_scopes: usize,
    pub typed_except_scopes: usize,
    /// `on` table entries.
    pub typed_entries: usize,
}

impl SehLandings {
    pub fn landing_vas(&self) -> BTreeSet<u64> {
        self.by_func.values().flatten().copied().collect()
    }
}

fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// The landings of every function whose UNWIND_INFO has UNW_FLAG_EHANDLER or UNW_FLAG_UHANDLER and the handler RVA in `handlers`.
/// An empty `handlers` gives an empty result. A function with such a handler that is a chained piece, a scope that is not inside the
/// function, or a landing outside it is an error.
pub fn delphi_landings(pe: &Pe<'_>, handlers: &[u32]) -> Result<SehLandings, String> {
    let mut out = SehLandings::default();
    if handlers.is_empty() {
        return Ok(out);
    }
    let rfs = pe.runtime_functions().map_err(|e| format!(".pdata: {e}"))?;
    let begins: BTreeSet<u32> = rfs.iter().map(|r| r.begin_rva).collect();
    for rf in rfs {
        let ui = pe.unwind_info(rf.unwind_rva).map_err(|e| format!("unwind info {:#x}: {e}", rf.unwind_rva))?;
        match ui.handler_rva {
            Some(h) if handlers.contains(&h) => {}
            _ => continue,
        }
        let tail = rf.unwind_rva + 4 + 2 * ((ui.count_of_codes as u32 + 1) & !1);
        let fva = pe.rva_to_va(rf.begin_rva);
        let hdr = pe.slice_at_rva(tail + 4, 4).map_err(|e| format!("scope table of {fva:#x}: {e}"))?;
        let n = le32(hdr, 0) as usize;
        if n > 4096 {
            return Err(format!("scope table of {fva:#x}: {n} scopes"));
        }
        let body = pe.slice_at_rva(tail + 8, 16 * n).map_err(|e| format!("scope table of {fva:#x}: {e}"))?;
        out.handler_functions += 1;
        for i in 0..n {
            let (b, e, f2, f3) = (le32(body, 16 * i), le32(body, 16 * i + 4), le32(body, 16 * i + 8), le32(body, 16 * i + 12));
            if !(rf.begin_rva <= b && b < e && e <= rf.end_rva) {
                return Err(format!("function {fva:#x}: scope {i} [{b:#x},{e:#x}) is not inside the function"));
            }
            out.scopes += 1;
            let land = |l: u32, out: &mut SehLandings| -> Result<(), String> {
                if !(rf.begin_rva < l && l < rf.end_rva) {
                    return Err(format!("function {fva:#x}: scope {i}: the landing {l:#x} is not inside the function"));
                }
                let v = out.by_func.entry(fva).or_default();
                let lv = pe.rva_to_va(l);
                if !v.contains(&lv) {
                    v.push(lv);
                }
                Ok(())
            };
            match f2 {
                0 => {
                    if !begins.contains(&f3) {
                        return Err(format!("function {fva:#x}: scope {i}: the finally funclet {f3:#x} is not a .pdata function start"));
                    }
                    out.finally_scopes += 1;
                }
                1 => return Err(format!("function {fva:#x}: scope {i}: f2 == 1 is not a shape of the reading")),
                2 => {
                    land(f3, &mut out)?;
                    out.bare_except_scopes += 1;
                }
                _ => {
                    let t = pe.slice_at_rva(f2, 4).map_err(|e| format!("function {fva:#x}: scope {i}: on table {f2:#x}: {e}"))?;
                    let k = le32(t, 0) as usize;
                    if k > 256 {
                        return Err(format!("function {fva:#x}: scope {i}: on table with {k} entries"));
                    }
                    let ents = pe.slice_at_rva(f2 + 4, 8 * k).map_err(|e| format!("function {fva:#x}: scope {i}: on table {f2:#x}: {e}"))?;
                    for j in 0..k {
                        land(le32(ents, 8 * j + 4), &mut out)?;
                        out.typed_entries += 1;
                    }
                    out.typed_except_scopes += 1;
                }
            }
        }
    }
    for v in out.by_func.values_mut() {
        v.sort_unstable();
    }
    Ok(out)
}
