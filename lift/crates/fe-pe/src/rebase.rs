//! File-stage rebase (decision 0080): applies the base relocation directory (data directory 5) of a
//! PE32+ image for a new ImageBase and writes the new ImageBase into the header. Standard library only.
//!
//! The operation is the one the Windows loader performs when it moves an image
//! (Microsoft Learn, "PE Format", The .reloc Section): every IMAGE_REL_BASED_DIR64 entry adds the
//! difference (new base - old base) to the 64-bit field at its offset; IMAGE_REL_BASED_ABSOLUTE is
//! padding and is skipped. Any other type is an error (nothing is ignored silently). A slot with no
//! backing bytes in the file is an error. Nothing else in the file is changed: DllCharacteristics,
//! Characteristics, CheckSum, the sections and the .reloc itself stay as they are.

use crate::{
    dir, le16, le32, le64, Pe, PeError, IMAGE_REL_BASED_ABSOLUTE, IMAGE_REL_BASED_DIR64,
    MAX_RELOC_BLOCKS_BYTES,
};
use std::fmt;

/// IMAGE_FILE_RELOCS_STRIPPED (Microsoft Learn, "PE Format", Characteristics, 0x0001).
const IMAGE_FILE_RELOCS_STRIPPED: u16 = 0x0001;
/// ImageBase "must be a multiple of 64 K" (Microsoft Learn, "PE Format", Optional Header Windows-Specific Fields).
const IMAGE_BASE_ALIGN: u64 = 0x1_0000;
/// Offset of ImageBase inside the PE32+ optional header.
const OPT_IMAGE_BASE: usize = 24;
/// Offset of the optional header after the PE signature (4) and the COFF file header (20).
const OPT_OFFSET: usize = 24;
const SLOT_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseError {
    Pe(PeError),
    /// The new base is not usable as an ImageBase (not a multiple of 64 K, or base + SizeOfImage overflows 64 bits).
    BadNewBase { base: u64, why: &'static str },
    /// The base changes but the file has no base relocations to apply (no directory, or
    /// IMAGE_FILE_RELOCS_STRIPPED).
    NoRelocations(&'static str),
    /// A relocation entry that is neither IMAGE_REL_BASED_DIR64 nor IMAGE_REL_BASED_ABSOLUTE.
    UnsupportedType { ty: u16, rva: u32 },
    /// A DIR64 slot whose 8 bytes are not all backed by one section's file data (.bss side, past the raw data,
    /// unmapped RVA or straddling the end of the raw data).
    NotFileBacked { rva: u32 },
    /// A DIR64 slot inside the headers.
    InHeaders { rva: u32 },
    /// Two DIR64 entries cover overlapping bytes (this includes the same slot twice).
    Overlap { rva: u32 },
    Malformed(&'static str),
}

impl fmt::Display for RebaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RebaseError::Pe(e) => write!(f, "{e}"),
            RebaseError::BadNewBase { base, why } => write!(f, "new base {base:#x}: {why}"),
            RebaseError::NoRelocations(s) => write!(f, "cannot rebase: {s}"),
            RebaseError::UnsupportedType { ty, rva } => {
                write!(f, "unsupported base relocation type {ty} at RVA {rva:#x} (only DIR64 and ABSOLUTE are handled)")
            }
            RebaseError::NotFileBacked { rva } => {
                write!(f, "DIR64 slot at RVA {rva:#x} is not file-backed (8 bytes in one section's raw data)")
            }
            RebaseError::InHeaders { rva } => write!(f, "DIR64 slot at RVA {rva:#x} is inside the headers"),
            RebaseError::Overlap { rva } => write!(f, "DIR64 slot at RVA {rva:#x} overlaps another DIR64 slot"),
            RebaseError::Malformed(s) => write!(f, "malformed base relocation directory: {s}"),
        }
    }
}

impl std::error::Error for RebaseError {}

impl From<PeError> for RebaseError {
    fn from(e: PeError) -> Self {
        RebaseError::Pe(e)
    }
}

/// What `rebase` did. The counts are of the directory as read, also when the base does not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebaseReport {
    pub old_base: u64,
    pub new_base: u64,
    /// `new_base - old_base` modulo 2^64 (the value added to every DIR64 slot).
    pub delta: u64,
    pub blocks: usize,
    pub dir64: usize,
    pub absolute: usize,
    /// The CheckSum field, which is copied as it is.
    pub checksum: u32,
}

/// The file offsets of the DIR64 slots, in directory order, with the entry counts.
struct Scan {
    slots: Vec<(u32, usize)>,
    blocks: usize,
    absolute: usize,
}

fn scan(pe: &Pe) -> Result<Scan, RebaseError> {
    let mut scan = Scan { slots: Vec::new(), blocks: 0, absolute: 0 };
    let d = match pe.data_dirs.get(dir::BASERELOC) {
        Some(d) if d.rva != 0 && d.size != 0 => *d,
        _ => return Ok(scan),
    };
    if d.size > MAX_RELOC_BLOCKS_BYTES {
        return Err(PeError::LimitExceeded("base relocation directory size").into());
    }
    let b = pe.slice_at_rva(d.rva, d.size as usize)?;
    let mut o = 0usize;
    while o < b.len() {
        if o + 8 > b.len() {
            return Err(RebaseError::Malformed("trailing bytes after the last block"));
        }
        let page = le32(b, o);
        let size = le32(b, o + 4) as usize;
        if size < 8 || size % 2 != 0 || o + size > b.len() {
            return Err(RebaseError::Malformed("block size"));
        }
        scan.blocks += 1;
        for k in 0..(size - 8) / 2 {
            let e = le16(b, o + 8 + 2 * k);
            let ty = e >> 12;
            let off = (e & 0x0fff) as u32;
            let rva = page.checked_add(off).ok_or(RebaseError::Malformed("page RVA + offset overflows"))?;
            match ty {
                IMAGE_REL_BASED_ABSOLUTE => scan.absolute += 1,
                IMAGE_REL_BASED_DIR64 => {
                    if (rva as u64) < pe.optional.size_of_headers as u64 {
                        return Err(RebaseError::InHeaders { rva });
                    }
                    let (file_off, avail) = match pe.locate(rva) {
                        Ok(v) => v,
                        Err(PeError::RvaNotFileBacked(_)) | Err(PeError::RvaUnmapped(_)) => {
                            return Err(RebaseError::NotFileBacked { rva })
                        }
                        Err(e) => return Err(e.into()),
                    };
                    if avail < SLOT_SIZE {
                        return Err(RebaseError::NotFileBacked { rva });
                    }
                    scan.slots.push((rva, file_off));
                }
                _ => return Err(RebaseError::UnsupportedType { ty, rva }),
            }
        }
        o += size;
    }
    let mut by_off = scan.slots.clone();
    by_off.sort_by_key(|&(_, off)| off);
    for w in by_off.windows(2) {
        if w[1].1 < w[0].1 + SLOT_SIZE {
            return Err(RebaseError::Overlap { rva: w[1].0 });
        }
    }
    Ok(scan)
}

/// Returns the image with every DIR64 slot increased by `new_base - old_base` (modulo 2^64, as the loader does)
/// and ImageBase set to `new_base`, and a report. `new_base` must be a multiple of 64 K and `new_base +
/// SizeOfImage` must not overflow. A `new_base` equal to the current ImageBase returns a byte-identical copy
/// (the directory is still checked). When the base changes, a file with no base relocation directory or with
/// IMAGE_FILE_RELOCS_STRIPPED is an error, since its absolute addresses cannot be found.
pub fn rebase(data: &[u8], new_base: u64) -> Result<(Vec<u8>, RebaseReport), RebaseError> {
    let pe = Pe::parse(data)?;
    let old_base = pe.optional.image_base;
    if new_base % IMAGE_BASE_ALIGN != 0 {
        return Err(RebaseError::BadNewBase { base: new_base, why: "not a multiple of 64 K" });
    }
    if new_base.checked_add(pe.optional.size_of_image as u64).is_none() {
        return Err(RebaseError::BadNewBase { base: new_base, why: "base + SizeOfImage overflows 64 bits" });
    }
    let scan = scan(&pe)?;
    let delta = new_base.wrapping_sub(old_base);
    let lfanew = le32(data, 0x3c) as usize;
    let checksum = le32(data, lfanew + OPT_OFFSET + 64);
    let report = RebaseReport {
        old_base,
        new_base,
        delta,
        blocks: scan.blocks,
        dir64: scan.slots.len(),
        absolute: scan.absolute,
        checksum,
    };
    let mut out = data.to_vec();
    if new_base == old_base {
        return Ok((out, report));
    }
    if pe.file_header.characteristics & IMAGE_FILE_RELOCS_STRIPPED != 0 {
        return Err(RebaseError::NoRelocations("IMAGE_FILE_RELOCS_STRIPPED is set"));
    }
    if !matches!(pe.data_dirs.get(dir::BASERELOC), Some(d) if d.rva != 0 && d.size != 0) {
        return Err(RebaseError::NoRelocations("no base relocation directory"));
    }
    for &(_, off) in &scan.slots {
        let v = le64(&out, off).wrapping_add(delta);
        out[off..off + SLOT_SIZE].copy_from_slice(&v.to_le_bytes());
    }
    let at = lfanew + OPT_OFFSET + OPT_IMAGE_BASE;
    out[at..at + 8].copy_from_slice(&new_base.to_le_bytes());
    Ok((out, report))
}
