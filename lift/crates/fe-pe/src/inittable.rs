//! The InitTable of FLEngine_x64.dll: the unit table that DllMain's InitUnits (f_4120c0) walks
//! (docs/investigations/a4-survey-tool.md, decision 0060).
//!
//! Layout (read from the DLL bytes; f_4120c0, objdump 0x4120c0..0x41214b):
//! - InitContext (.bss, VA 0x15EAC90): `+0` pointer to the PackageUnitHeader, `+8` u32 InitCount (the loop writes index+1
//!   before it calls a unit). The context is written at run time (0x4121bb) and cannot be read from the file.
//! - PackageUnitHeader (VA 0x13768B0, in .text; DllMain passes it as `lea rdx,[rip+0x51]` at 0x1376858): `+0` u32 UnitCount,
//!   `+8` u64 UnitsPtr.
//! - Unit i at `UnitsPtr + 16*i`: `+0` u64 proc, `+8` u64 a second procedure (the finalizer walk f_412020 calls it).
//! - The init loop calls `proc` only when `proc != 0` and `LD64(proc) != 0`.

use crate::{Pe, PeError};

/// InitContext in .bss (runtime state, not in the file).
pub const INIT_CONTEXT_VA: u64 = 0x15EAC90;
/// InitContext+8: u32 InitCount.
pub const INIT_COUNT_VA: u64 = 0x15EAC98;
/// The PackageUnitHeader that DllMain passes (static, in .text).
pub const INIT_TABLE_HEADER_VA: u64 = 0x13768B0;
/// Upper bound on UnitCount accepted by the reader (the real table has 942).
pub const MAX_UNITS: u32 = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// `proc != 0` and `LD64(proc) != 0`: the init loop calls it.
    Runnable,
    /// `proc != 0` and `LD64(proc) == 0`: the init loop skips it.
    Empty,
    /// `proc == 0`.
    NoProc,
    /// `proc` points outside the image: the first 8 bytes cannot be read from the file.
    Unreadable,
}

impl UnitKind {
    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Runnable => "runnable",
            UnitKind::Empty => "empty",
            UnitKind::NoProc => "noproc",
            UnitKind::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub index: u32,
    pub proc_va: u64,
    /// Element `+8`.
    pub second_va: u64,
    pub kind: UnitKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitTable {
    pub header_va: u64,
    pub unit_count: u32,
    pub units_va: u64,
    pub units: Vec<Unit>,
}

impl InitTable {
    pub fn count(&self, k: UnitKind) -> usize {
        self.units.iter().filter(|u| u.kind == k).count()
    }

    /// Distinct `proc` values of the units of kind `k`.
    pub fn distinct_procs(&self, k: UnitKind) -> usize {
        let mut v: Vec<u64> = self.units.iter().filter(|u| u.kind == k).map(|u| u.proc_va).collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    }

    /// TSV, one line per unit: `index proc_va kind second_va run_rank`. `run_rank` is the 1-based position among the
    /// runnable units (the order in which the loop calls them), `-` for the others.
    pub fn to_tsv(&self) -> String {
        let mut s = String::from("index\tproc_va\tkind\tsecond_va\trun_rank\n");
        let mut rank = 0usize;
        for u in &self.units {
            let r = if u.kind == UnitKind::Runnable {
                rank += 1;
                rank.to_string()
            } else {
                "-".to_string()
            };
            s.push_str(&format!("{}\t{:#x}\t{}\t{:#x}\t{}\n", u.index, u.proc_va, u.kind.name(), u.second_va, r));
        }
        s
    }
}

impl<'a> Pe<'a> {
    /// A u64 as the loader would see it: zero-initialized bytes of a section's virtual tail read as 0.
    /// `Err` for an address that no section maps.
    fn ld64_image(&self, va: u64) -> Result<u64, PeError> {
        let rva = self.va_to_rva(va)?;
        match self.slice_at_rva(rva, 8) {
            Ok(b) => Ok(u64::from_le_bytes(b.try_into().unwrap())),
            Err(PeError::RvaNotFileBacked(_)) => Ok(0),
            Err(e) => Err(e),
        }
    }

    /// Reads the PackageUnitHeader at `header_va` and classifies every unit as f_4120c0 does.
    pub fn init_table(&self, header_va: u64) -> Result<InitTable, PeError> {
        let hdr = self.slice_at_rva(self.va_to_rva(header_va)?, 16)?;
        let unit_count = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        let units_va = u64::from_le_bytes(hdr[8..16].try_into().unwrap());
        if unit_count > MAX_UNITS {
            return Err(PeError::LimitExceeded("InitTable unit count"));
        }
        let mut units = Vec::with_capacity(unit_count as usize);
        for i in 0..unit_count {
            let e = self.slice_at_rva(self.va_to_rva(units_va + 16 * i as u64)?, 16)?;
            let proc_va = u64::from_le_bytes(e[0..8].try_into().unwrap());
            let second_va = u64::from_le_bytes(e[8..16].try_into().unwrap());
            let kind = if proc_va == 0 {
                UnitKind::NoProc
            } else {
                match self.ld64_image(proc_va) {
                    Ok(0) => UnitKind::Empty,
                    Ok(_) => UnitKind::Runnable,
                    Err(_) => UnitKind::Unreadable,
                }
            };
            units.push(Unit { index: i, proc_va, second_va, kind });
        }
        Ok(InitTable { header_va, unit_count, units_va, units })
    }

    /// The header VA that the entry point's code loads with `lea rdx,[rip+disp32]` (48 8D 15), found in the first
    /// 0x80 bytes of the entry: `(address of the lea, referenced VA)`. For FLEngine_x64.dll: (0x1376858, 0x13768b0).
    pub fn entry_header_reference(&self) -> Option<(u64, u64)> {
        let entry = self.rva_to_va(self.optional.address_of_entry_point);
        let code = self.slice_at_rva(self.optional.address_of_entry_point, 0x80).ok()?;
        let at = code.windows(3).position(|w| w == [0x48, 0x8d, 0x15])?;
        let disp = i32::from_le_bytes(code.get(at + 3..at + 7)?.try_into().ok()?) as i64;
        let lea = entry + at as u64;
        Some((lea, (lea + 7).wrapping_add(disp as u64)))
    }
}
