//! `Pe::dir64_relocation_vas` (decision 0051). Part 1 builds a PE with one hand-made relocation block,
//! part 2 checks the real FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll).
//! The real count was measured with pefile (lift/out-adj/survey3.py): 278,531 DIR64 entries, 993 ABSOLUTE padding.

use fe_pe::Pe;
use std::path::PathBuf;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// One section `.reloc` (RVA 0x2000, raw 0x400) holding `reloc`; data directory 5 = that section.
fn build(reloc: &[u8], dir_size: u32) -> Vec<u8> {
    let mut b = vec![0u8; 0x400 + 0x200];
    b[0..2].copy_from_slice(b"MZ");
    w32(&mut b, 0x3c, 0x80);
    let nt = 0x80;
    b[nt..nt + 4].copy_from_slice(b"PE\0\0");
    w16(&mut b, nt + 4, 0x8664);
    w16(&mut b, nt + 6, 1);
    w16(&mut b, nt + 20, 240);
    let o = nt + 24;
    w16(&mut b, o, 0x20b);
    b[o + 24..o + 32].copy_from_slice(&0x1_4000_0000u64.to_le_bytes());
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 112 + 8 * 5, 0x2000);
    w32(&mut b, o + 112 + 8 * 5 + 4, dir_size);
    let s = o + 240;
    b[s..s + 6].copy_from_slice(b".reloc");
    w32(&mut b, s + 8, 0x200);
    w32(&mut b, s + 12, 0x2000);
    w32(&mut b, s + 16, 0x200);
    w32(&mut b, s + 20, 0x400);
    b[0x400..0x400 + reloc.len()].copy_from_slice(reloc);
    b
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

#[test]
fn dir64_entries_only_in_file_order() {
    let mut r = block(0x1000, &[0xa008, 0x3010, 0x0000, 0xa018]);
    r.extend(block(0x5000, &[0xaff8, 0x1000]));
    let img = build(&r, r.len() as u32);
    let pe = Pe::parse(&img).unwrap();
    let base = 0x1_4000_0000u64;
    assert_eq!(pe.dir64_relocation_vas().unwrap(), vec![base + 0x1008, base + 0x1018, base + 0x5ff8]);
}

#[test]
fn no_directory_is_empty_and_bad_block_is_malformed() {
    let img = build(&[], 0);
    assert!(Pe::parse(&img).unwrap().dir64_relocation_vas().unwrap().is_empty());
    let mut r = block(0x1000, &[0xa008]);
    r[4] = 7; // odd size
    let img = build(&r, 10);
    assert!(Pe::parse(&img).unwrap().dir64_relocation_vas().is_err());
    let mut r = block(0x1000, &[0xa008]);
    r[4] = 0x40; // past the directory
    let img = build(&r, 10);
    assert!(Pe::parse(&img).unwrap().dir64_relocation_vas().is_err());
}

fn load() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

#[test]
fn real_dll_has_278531_dir64_slots_and_one_holds_0x403582() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let v = pe.dir64_relocation_vas().unwrap();
    assert_eq!(v.len(), 278_531);
    // the third entry of the table at 0x403598 (+8 each): 0x403598 holds 0x403579, 0x4035a0 holds 0x403582
    assert!(v.contains(&0x403598) && v.contains(&0x4035a0) && v.contains(&0x4035a8));
    let rva = pe.va_to_rva(0x4035a0).unwrap();
    let b = pe.slice_at_rva(rva, 8).unwrap();
    assert_eq!(u64::from_le_bytes(b.try_into().unwrap()), 0x403582);
}
