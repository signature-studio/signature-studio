//! File-stage rebase (decision 0080, `fe_pe::rebase::rebase`). Part 1 uses a synthetic PE32+; part 2 uses the
//! real dsp_ippv2_x64.dll ($DSP_IPPV2_PE, else lift/out-ext/dsp_ippv2_x64.dll; skipped when absent, as jt_cpu.rs)
//! and FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll).
//!
//! Synthetic layout (file 0xa00 bytes, SizeOfHeaders 0x400, SizeOfImage 0x4000, ImageBase IB = 0x1_8000_0000):
//!
//! | section | RVA    | VirtualSize | raw    | SizeOfRawData | note |
//! |---------|--------|-------------|--------|---------------|------|
//! | .text   | 0x1000 | 0x200       | 0x400  | 0x200         | 0x1000: a DIR64 slot (code-section pointer) |
//! | .data   | 0x2000 | 0x400       | 0x600  | 0x200         | 0x2000..0x2200 file-backed, 0x2200..0x2400 .bss side |
//! | .reloc  | 0x3000 | 0x200       | 0x800  | 0x200         | the relocation blocks |

use fe_pe::rebase::{rebase, RebaseError};
use fe_pe::{Pe, PeError};
use std::path::PathBuf;

const IB: u64 = 0x1_8000_0000;
const NEW: u64 = 0x200_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const FILE_LEN: usize = 0xa00;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn r64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

fn section(b: &mut [u8], i: usize, name: &[u8], vsize: u32, rva: u32, rawsize: u32, raw: u32, flags: u32) {
    let s = OPT + 240 + 40 * i;
    b[s..s + name.len()].copy_from_slice(name);
    w32(b, s + 8, vsize);
    w32(b, s + 12, rva);
    w32(b, s + 16, rawsize);
    w32(b, s + 20, raw);
    w32(b, s + 36, flags);
}

fn block(page: u32, entries: &[u16]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&page.to_le_bytes());
    v.extend_from_slice(&(8 + 2 * entries.len() as u32).to_le_bytes());
    for e in entries {
        v.extend_from_slice(&e.to_le_bytes());
    }
    v
}

const DIR64: u16 = 0xa000;
const ABS: u16 = 0x0000;

/// A PE with the DIR64 slot values below at file offsets 0x400 (.text, RVA 0x1000) and 0x600.. (.data, RVA 0x2000..),
/// CheckSum 0x1234_5678, DllCharacteristics 0x0160, Characteristics 0x2022, and `reloc` as the base relocation blocks.
fn build(reloc: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    w16(&mut b, LFANEW + 4, 0x8664);
    w16(&mut b, LFANEW + 6, 3);
    w16(&mut b, LFANEW + 20, 240);
    w16(&mut b, LFANEW + 22, 0x2022);
    w16(&mut b, OPT, 0x20b);
    w64(&mut b, OPT + 24, IB);
    w32(&mut b, OPT + 32, 0x1000);
    w32(&mut b, OPT + 36, 0x200);
    w32(&mut b, OPT + 56, 0x4000);
    w32(&mut b, OPT + 60, 0x400);
    w32(&mut b, OPT + 64, 0x1234_5678);
    w16(&mut b, OPT + 68, 2);
    w16(&mut b, OPT + 70, 0x0160);
    w32(&mut b, OPT + 108, 16);
    w32(&mut b, OPT + 112 + 8 * 5, 0x3000);
    w32(&mut b, OPT + 116 + 8 * 5, reloc.len() as u32);
    section(&mut b, 0, b".text", 0x200, 0x1000, 0x200, 0x400, 0x6000_0020);
    section(&mut b, 1, b".data", 0x400, 0x2000, 0x200, 0x600, 0xc000_0040);
    section(&mut b, 2, b".reloc", 0x200, 0x3000, 0x200, 0x800, 0x4200_0040);
    w64(&mut b, 0x400, IB + 0x1010); // .text RVA 0x1000
    w64(&mut b, 0x600, IB + 0x1000); // .data RVA 0x2000
    w64(&mut b, 0x608, IB + 0x2000); // .data RVA 0x2008
    w64(&mut b, 0x610, 0x1122_3344_5566_7788); // .data RVA 0x2010, not in .reloc: must not change
    w64(&mut b, 0x7f8, IB + 0x3000); // .data RVA 0x21f8, last slot of the file-backed part
    b[0x800..0x800 + reloc.len()].copy_from_slice(reloc);
    b
}

fn good_reloc() -> Vec<u8> {
    let mut r = block(0x1000, &[DIR64, ABS]);
    r.extend(block(0x2000, &[DIR64, DIR64 | 0x008, DIR64 | 0x1f8, ABS]));
    r
}

fn good() -> Vec<u8> {
    build(&good_reloc())
}

#[test]
fn dir64_gets_the_difference_and_image_base_is_rewritten() {
    let img = good();
    let (out, r) = rebase(&img, NEW).unwrap();
    let delta = NEW.wrapping_sub(IB);
    assert_eq!((r.old_base, r.new_base, r.delta), (IB, NEW, delta));
    assert_eq!((r.blocks, r.dir64, r.absolute), (2, 4, 2));
    assert_eq!(r64(&out, 0x400), NEW + 0x1010);
    assert_eq!(r64(&out, 0x600), NEW + 0x1000);
    assert_eq!(r64(&out, 0x608), NEW + 0x2000);
    assert_eq!(r64(&out, 0x7f8), NEW + 0x3000);
    assert_eq!(r64(&out, 0x610), 0x1122_3344_5566_7788, "a value that .reloc does not name is not touched");
    assert_eq!(r64(&out, OPT + 24), NEW);
    let pe = Pe::parse(&out).unwrap();
    assert_eq!(pe.optional.image_base, NEW);
    assert_eq!(pe.dir64_relocation_vas().unwrap(), Pe::parse(&img).unwrap().dir64_relocation_vas().unwrap().iter().map(|v| v - IB + NEW).collect::<Vec<_>>());
}

#[test]
fn nothing_else_changes() {
    let img = good();
    let (out, _) = rebase(&img, NEW).unwrap();
    assert_eq!(out.len(), img.len());
    let changed: Vec<usize> = (0..img.len()).filter(|&i| img[i] != out[i]).collect();
    let allowed = |i: usize| {
        (0x400..0x408).contains(&i)
            || (0x600..0x610).contains(&i)
            || (0x7f8..0x800).contains(&i)
            || (OPT + 24..OPT + 32).contains(&i)
    };
    assert!(!changed.is_empty() && changed.iter().all(|&i| allowed(i)), "changed offsets: {changed:x?}");
    // CheckSum, DllCharacteristics, Characteristics and the .reloc bytes are copied as they are.
    assert_eq!(r64(&out, OPT + 64) & 0xffff_ffff, 0x1234_5678);
    assert_eq!(&out[OPT + 70..OPT + 72], &0x0160u16.to_le_bytes());
    assert_eq!(&out[LFANEW + 22..LFANEW + 24], &0x2022u16.to_le_bytes());
    assert_eq!(&out[0x800..0xa00], &img[0x800..0xa00]);
    let pe = Pe::parse(&out).unwrap();
    assert_eq!(pe.optional.dll_characteristics, 0x0160);
    assert_eq!(pe.file_header.characteristics, 0x2022);
}

#[test]
fn same_base_is_byte_identical() {
    let img = good();
    let (out, r) = rebase(&img, IB).unwrap();
    assert_eq!(out, img);
    assert_eq!((r.delta, r.dir64, r.absolute), (0, 4, 2));
}

#[test]
fn round_trip_and_determinism() {
    let img = good();
    let (a, _) = rebase(&img, NEW).unwrap();
    let (b, _) = rebase(&img, NEW).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, img);
    let (back, _) = rebase(&a, IB).unwrap();
    assert_eq!(back, img);
    // via a third base and from a base below the old one
    let (c, _) = rebase(&a, 0x5000_0000).unwrap();
    let (d, _) = rebase(&c, IB).unwrap();
    assert_eq!(d, img);
    let (e, _) = rebase(&img, IB + 0x20_0000).unwrap();
    assert_eq!(r64(&e, 0x600), IB + 0x20_0000 + 0x1000);
    assert_eq!(rebase(&e, IB).unwrap().0, img);
}

#[test]
fn highlow_is_an_error_not_ignored() {
    let mut r = block(0x2000, &[DIR64, 0x3000 | 0x008]); // IMAGE_REL_BASED_HIGHLOW
    r.extend(block(0x1000, &[DIR64]));
    let img = build(&r);
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::UnsupportedType { ty: 3, rva: 0x2008 });
    // the type is checked also when the base does not change
    assert_eq!(rebase(&img, IB).unwrap_err(), RebaseError::UnsupportedType { ty: 3, rva: 0x2008 });
}

#[test]
fn every_other_type_is_an_error() {
    for ty in [1u16, 2, 3, 4, 5, 6, 7, 8, 9, 11, 15] {
        let img = build(&block(0x2000, &[(ty << 12) | 0x008]));
        assert_eq!(
            rebase(&img, NEW).unwrap_err(),
            RebaseError::UnsupportedType { ty, rva: 0x2008 },
            "type {ty}"
        );
    }
}

#[test]
fn slot_without_file_backing_is_an_error() {
    // .bss side of .data (RVA 0x2200..0x2400)
    let img = build(&block(0x2000, &[DIR64 | 0x200]));
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::NotFileBacked { rva: 0x2200 });
    // straddles the end of the raw data (RVA 0x21fc..0x2204)
    let img = build(&block(0x2000, &[DIR64 | 0x1fc]));
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::NotFileBacked { rva: 0x21fc });
    // beyond the section (RVA 0x2ff8, between .data and .reloc)
    let img = build(&block(0x2000, &[DIR64 | 0xff8]));
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::NotFileBacked { rva: 0x2ff8 });
    // in the headers
    let img = build(&block(0x0000, &[DIR64 | 0x100]));
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::InHeaders { rva: 0x100 });
    // the check does not depend on the base: it also applies when the base is unchanged
    let img = build(&block(0x2000, &[DIR64 | 0x200]));
    assert!(rebase(&img, IB).is_err());
}

#[test]
fn overlapping_slots_are_an_error() {
    let img = build(&block(0x2000, &[DIR64, DIR64 | 0x004]));
    assert_eq!(rebase(&img, NEW).unwrap_err(), RebaseError::Overlap { rva: 0x2004 });
    let img = build(&block(0x2000, &[DIR64, DIR64]));
    assert!(matches!(rebase(&img, NEW).unwrap_err(), RebaseError::Overlap { .. }));
}

#[test]
fn bad_blocks_are_errors() {
    let mut r = block(0x2000, &[DIR64]);
    r[4] = 7; // odd size
    assert!(matches!(rebase(&build(&r), NEW).unwrap_err(), RebaseError::Malformed(_)));
    let mut r = block(0x2000, &[DIR64]);
    r[4] = 0x40; // past the directory
    assert!(matches!(rebase(&build(&r), NEW).unwrap_err(), RebaseError::Malformed(_)));
    let mut r = block(0x2000, &[DIR64]);
    r.extend_from_slice(&[0, 0, 0, 0]); // trailing bytes shorter than a block header
    assert!(matches!(rebase(&build(&r), NEW).unwrap_err(), RebaseError::Malformed(_)));
}

#[test]
fn new_base_must_be_a_multiple_of_64k_and_fit() {
    let img = good();
    assert!(matches!(rebase(&img, 0x200_1000).unwrap_err(), RebaseError::BadNewBase { .. }));
    // SizeOfImage 0x4000 fits below 2^64 even at the highest 64 K multiple; with SizeOfImage 0x20000 it does not
    assert!(rebase(&img, 0xffff_ffff_ffff_0000).is_ok());
    let mut big = good();
    w32(&mut big, OPT + 56, 0x2_0000);
    assert!(matches!(rebase(&big, 0xffff_ffff_ffff_0000).unwrap_err(), RebaseError::BadNewBase { .. }));
    assert!(matches!(rebase(&big, 0xffff_ffff_fffe_0000).unwrap_err(), RebaseError::BadNewBase { .. }), "base + size == 2^64");
    assert!(rebase(&big, 0xffff_ffff_fffd_0000).is_ok());
}

#[test]
fn a_file_without_base_relocations_cannot_change_base() {
    let img = build(&[]);
    assert_eq!(rebase(&img, IB).unwrap().0, img);
    assert!(matches!(rebase(&img, NEW).unwrap_err(), RebaseError::NoRelocations(_)));
    let mut stripped = good();
    w16(&mut stripped, LFANEW + 22, 0x2023); // IMAGE_FILE_RELOCS_STRIPPED
    assert!(matches!(rebase(&stripped, NEW).unwrap_err(), RebaseError::NoRelocations(_)));
    assert_eq!(rebase(&stripped, IB).unwrap().0, stripped);
}

#[test]
fn not_a_pe_is_a_pe_error() {
    assert!(matches!(rebase(&[0u8; 16], NEW).unwrap_err(), RebaseError::Pe(PeError::Truncated { .. })));
}

// ---------------------------------------------------------------------------------------------
// Real files
// ---------------------------------------------------------------------------------------------

fn load(var: &str, default: &str) -> Option<Vec<u8>> {
    let explicit = std::env::var_os(var);
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("{var}={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

fn dll() -> Option<Vec<u8>> {
    load("DSP_IPPV2_PE", "../../out-ext/dsp_ippv2_x64.dll")
}

#[test]
fn real_dsp_ippv2_counts_determinism_identity_round_trip() {
    let Some(d) = dll() else { return };
    let pe = Pe::parse(&d).unwrap();
    assert_eq!(pe.optional.image_base, 0x1_8000_0000);
    let (a, r) = rebase(&d, 0x200_0000).unwrap();
    assert_eq!((r.dir64, r.absolute), (15_602, 22));
    assert_eq!(r.delta, 0x200_0000u64.wrapping_sub(0x1_8000_0000));
    let (a2, _) = rebase(&d, 0x200_0000).unwrap();
    assert!(a == a2, "rebase is not deterministic");
    assert!(a != d);
    assert_eq!(a.len(), d.len());
    let (same, _) = rebase(&d, 0x1_8000_0000).unwrap();
    assert!(same == d, "rebase to the old base is not the identity");
    let (back, _) = rebase(&a, 0x1_8000_0000).unwrap();
    assert!(back == d, "rebase there and back is not the identity");
    // the number of changed bytes is bounded by 8 per DIR64 slot plus the 8 bytes of ImageBase
    let changed = (0..d.len()).filter(|&i| d[i] != a[i]).count();
    assert!(changed > 0 && changed <= 8 * 15_602 + 8, "changed bytes: {changed}");
}

#[test]
fn real_dsp_ippv2_structure_follows_the_new_base() {
    let Some(d) = dll() else { return };
    let (a, _) = rebase(&d, 0x200_0000).unwrap();
    let (po, pn) = (Pe::parse(&d).unwrap(), Pe::parse(&a).unwrap());
    assert_eq!(po.optional.image_base, 0x1_8000_0000);
    assert_eq!(pn.optional.image_base, 0x200_0000);
    assert_eq!(pn.optional.size_of_image, po.optional.size_of_image);
    assert_eq!(pn.optional.dll_characteristics, po.optional.dll_characteristics);
    assert_eq!(pn.file_header.characteristics, po.file_header.characteristics);
    assert_eq!(pn.sections, po.sections);
    assert_eq!(pn.data_dirs, po.data_dirs);
    assert_eq!(pn.function_starts().unwrap(), po.function_starts().unwrap());
    let (eo, en) = (po.exports().unwrap(), pn.exports().unwrap());
    assert_eq!(en.len(), eo.len());
    assert_eq!(en.len(), 446);
    for (x, y) in eo.iter().zip(&en) {
        assert_eq!((x.rva, &x.name), (y.rva, &y.name));
        assert_eq!(y.va, 0x200_0000 + y.rva as u64);
    }
    // every DIR64 slot value that pointed into the old image points into the new one at the same RVA
    let (vo, vn) = (po.dir64_relocation_vas().unwrap(), pn.dir64_relocation_vas().unwrap());
    assert_eq!(vo.len(), 15_602);
    let size = po.optional.size_of_image as u64;
    for (x, y) in vo.iter().zip(&vn) {
        let (a_, b_) = (le_at(&po, *x), le_at(&pn, *y));
        assert!((0x1_8000_0000..0x1_8000_0000 + size).contains(&a_));
        assert_eq!(b_, a_ - 0x1_8000_0000 + 0x200_0000);
    }
}

fn le_at(pe: &Pe, va: u64) -> u64 {
    let b = pe.slice_at_rva(pe.va_to_rva(va).unwrap(), 8).unwrap();
    u64::from_le_bytes(b.try_into().unwrap())
}

#[test]
fn real_flengine_round_trip() {
    let Some(d) = load("FLENGINE_PE", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let (a, r) = rebase(&d, 0x1000_0000).unwrap();
    assert_eq!((r.dir64, r.absolute), (278_531, 993));
    assert_eq!(r.checksum, 0x153cf0c);
    let (back, _) = rebase(&a, 0x40_0000).unwrap();
    assert!(back == d, "FLEngine round trip");
    assert!(rebase(&d, 0x40_0000).unwrap().0 == d);
}
