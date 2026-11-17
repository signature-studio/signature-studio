use fe_pe::{ImportSymbol, Pe, PeError};

const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const NSEC: usize = 2;
const HEADERS_END: usize = LFANEW + 24 + OPT_SIZE + 40 * NSEC;
const IMAGE_BASE: u64 = 0x1_4000_0000;
const DATA_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x1000;
const DATA_LEN: usize = 0x800;
const FILE_LEN: usize = DATA_RAW + DATA_LEN;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put_str(b: &mut [u8], rva: u32, s: &str) {
    let o = at(rva);
    b[o..o + s.len()].copy_from_slice(s.as_bytes());
    b[o + s.len()] = 0;
}

/// Sections: .rdata (RVA 0x1000, 0x800 bytes file-backed) and .bss (RVA 0x2000, 0x1000 virtual, no file bytes).
///
/// Exports: base 5, funcs [Alpha @0x1500, <empty>, Beta -> forwarder "other.Beta"].
/// Imports: a.dll {Foo (hint 7), ordinal 0x10}, b.dll {Bar (hint 1)} with OFT == 0 on the second.
fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, NSEC as u16);
    w32(&mut b, f + 4, 0x1234_5678);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1500);
    w32(&mut b, o + 20, 0x1000);
    w64(&mut b, o + 24, IMAGE_BASE);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w16(&mut b, o + 70, 0x160);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 112, DATA_RVA);
    w32(&mut b, o + 116, 0x100);
    w32(&mut b, o + 120, 0x1200);
    w32(&mut b, o + 124, 60);

    let s0 = o + OPT_SIZE;
    b[s0..s0 + 6].copy_from_slice(b".rdata");
    w32(&mut b, s0 + 8, DATA_LEN as u32);
    w32(&mut b, s0 + 12, DATA_RVA);
    w32(&mut b, s0 + 16, DATA_LEN as u32);
    w32(&mut b, s0 + 20, DATA_RAW as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);
    let s1 = s0 + 40;
    b[s1..s1 + 4].copy_from_slice(b".bss");
    w32(&mut b, s1 + 8, 0x1000);
    w32(&mut b, s1 + 12, 0x2000);
    w32(&mut b, s1 + 36, 0xc000_0000);

    let e = at(0x1000);
    w32(&mut b, e + 12, 0x1028);
    w32(&mut b, e + 16, 5);
    w32(&mut b, e + 20, 3);
    w32(&mut b, e + 24, 2);
    w32(&mut b, e + 28, 0x1040);
    w32(&mut b, e + 32, 0x1050);
    w32(&mut b, e + 36, 0x1058);
    put_str(&mut b, 0x1028, "test.dll");
    w32(&mut b, at(0x1040), 0x1500);
    w32(&mut b, at(0x1044), 0);
    w32(&mut b, at(0x1048), 0x10f0);
    w32(&mut b, at(0x1050), 0x1060);
    w32(&mut b, at(0x1054), 0x1068);
    w16(&mut b, at(0x1058), 0);
    w16(&mut b, at(0x105a), 2);
    put_str(&mut b, 0x1060, "Alpha");
    put_str(&mut b, 0x1068, "Beta");
    put_str(&mut b, 0x10f0, "other.Beta");

    let d0 = at(0x1200);
    w32(&mut b, d0, 0x1300);
    w32(&mut b, d0 + 12, 0x1280);
    w32(&mut b, d0 + 16, 0x1340);
    let d1 = d0 + 20;
    w32(&mut b, d1 + 12, 0x1288);
    w32(&mut b, d1 + 16, 0x1360);
    put_str(&mut b, 0x1280, "a.dll");
    put_str(&mut b, 0x1288, "b.dll");
    for base in [0x1300u32, 0x1340] {
        w64(&mut b, at(base), 0x1380);
        w64(&mut b, at(base + 8), 0x8000_0000_0000_0010);
    }
    w64(&mut b, at(0x1360), 0x1390);
    w16(&mut b, at(0x1380), 7);
    put_str(&mut b, 0x1382, "Foo");
    w16(&mut b, at(0x1390), 1);
    put_str(&mut b, 0x1392, "Bar");
    b
}

#[test]
fn header_fields() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.file_header.machine, 0x8664);
    assert_eq!(pe.file_header.number_of_sections, 2);
    assert_eq!(pe.file_header.time_date_stamp, 0x1234_5678);
    assert_eq!(pe.file_header.characteristics, 0x2022);
    assert_eq!(pe.optional.magic, 0x20b);
    assert_eq!(pe.optional.image_base, IMAGE_BASE);
    assert_eq!(pe.optional.address_of_entry_point, 0x1500);
    assert_eq!(pe.optional.size_of_image, 0x3000);
    assert_eq!(pe.optional.size_of_headers, 0x400);
    assert_eq!(pe.optional.section_alignment, 0x1000);
    assert_eq!(pe.optional.file_alignment, 0x200);
    assert_eq!(pe.optional.subsystem, 2);
    assert_eq!(pe.optional.dll_characteristics, 0x160);
    assert_eq!(pe.optional.number_of_rva_and_sizes, 16);
    assert_eq!(pe.data_dirs.len(), 16);
    assert_eq!((pe.data_dirs[1].rva, pe.data_dirs[1].size), (0x1200, 60));
}

#[test]
fn section_table() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.sections.len(), 2);
    assert_eq!(pe.sections[0].name, ".rdata");
    assert_eq!(pe.sections[0].virtual_address, 0x1000);
    assert_eq!(pe.sections[0].pointer_to_raw_data, 0x400);
    assert_eq!(pe.sections[0].size_of_raw_data, 0x800);
    assert_eq!(pe.sections[0].characteristics, 0x4000_0040);
    assert_eq!(pe.sections[1].name, ".bss");
    assert_eq!(pe.sections[1].virtual_size, 0x1000);
    assert_eq!(pe.sections[1].size_of_raw_data, 0);
}

#[test]
fn exports_with_gap_and_forwarder() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let ex = pe.exports().unwrap();
    assert_eq!(ex.len(), 2);
    assert_eq!(ex[0].name.as_deref(), Some("Alpha"));
    assert_eq!(ex[0].ordinal, 5);
    assert_eq!(ex[0].rva, 0x1500);
    assert_eq!(ex[0].va, IMAGE_BASE + 0x1500);
    assert_eq!(ex[0].forwarder, None);
    assert_eq!(ex[1].name.as_deref(), Some("Beta"));
    assert_eq!(ex[1].ordinal, 7);
    assert_eq!(ex[1].forwarder.as_deref(), Some("other.Beta"));
}

#[test]
fn imports_names_ordinals_and_iat_slots() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let im = pe.imports().unwrap();
    assert_eq!(im.len(), 2);
    assert_eq!(im[0].dll, "a.dll");
    assert_eq!(im[0].original_first_thunk, 0x1300);
    assert_eq!(im[0].first_thunk, 0x1340);
    assert_eq!(im[0].entries.len(), 2);
    assert_eq!(im[0].entries[0].symbol, ImportSymbol::Name { hint: 7, name: "Foo".into() });
    assert_eq!(im[0].entries[0].iat_rva, 0x1340);
    assert_eq!(im[0].entries[0].iat_va, IMAGE_BASE + 0x1340);
    assert_eq!(im[0].entries[1].symbol, ImportSymbol::Ordinal(0x10));
    assert_eq!(im[0].entries[1].iat_va, IMAGE_BASE + 0x1348);
    assert_eq!(im[1].dll, "b.dll");
    assert_eq!(im[1].original_first_thunk, 0);
    assert_eq!(im[1].entries.len(), 1);
    assert_eq!(im[1].entries[0].symbol, ImportSymbol::Name { hint: 1, name: "Bar".into() });
    assert_eq!(im[1].entries[0].iat_va, IMAGE_BASE + 0x1360);
}

#[test]
fn rva_mapping_and_bounds() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.rva_to_offset(0).unwrap(), 0);
    assert_eq!(pe.rva_to_offset(0x3ff).unwrap(), 0x3ff);
    assert_eq!(pe.rva_to_offset(0x1000).unwrap(), 0x400);
    assert_eq!(pe.rva_to_offset(0x17ff).unwrap(), 0xbff);
    assert_eq!(pe.va_to_offset(IMAGE_BASE + 0x1500).unwrap(), at(0x1500));
    assert_eq!(pe.rva_to_va(0x1500), IMAGE_BASE + 0x1500);
    // gap between headers and first section
    assert_eq!(pe.rva_to_offset(0x400), Err(PeError::RvaUnmapped(0x400)));
    assert_eq!(pe.rva_to_offset(0xfff), Err(PeError::RvaUnmapped(0xfff)));
    // between sections and past the end of the image
    assert_eq!(pe.rva_to_offset(0x1800), Err(PeError::RvaUnmapped(0x1800)));
    assert_eq!(pe.rva_to_offset(0x3000), Err(PeError::RvaUnmapped(0x3000)));
    assert_eq!(pe.rva_to_offset(u32::MAX), Err(PeError::RvaUnmapped(u32::MAX)));
    // mapped but no file bytes (.bss)
    assert_eq!(pe.rva_to_offset(0x2000), Err(PeError::RvaNotFileBacked(0x2000)));
    assert_eq!(pe.rva_to_offset(0x2fff), Err(PeError::RvaNotFileBacked(0x2fff)));
    // VA outside the image
    assert_eq!(pe.va_to_offset(IMAGE_BASE - 1), Err(PeError::VaOutOfImage(IMAGE_BASE - 1)));
    assert_eq!(pe.va_to_offset(IMAGE_BASE + 0x1_0000_0000), Err(PeError::VaOutOfImage(IMAGE_BASE + 0x1_0000_0000)));
    assert_eq!(pe.va_to_offset(u64::MAX), Err(PeError::VaOutOfImage(u64::MAX)));
    // a range must not run past the end of its section's file bytes
    assert!(pe.slice_at_rva(0x17f8, 8).is_ok());
    assert!(matches!(pe.slice_at_rva(0x17f8, 9), Err(PeError::Truncated { .. })));
}

#[test]
fn rejects_bad_dos_signature() {
    let mut img = build();
    img[0] = b'Z';
    assert_eq!(Pe::parse(&img).err(), Some(PeError::BadDosSignature(0x5a5a)));
}

#[test]
fn rejects_bad_pe_signature() {
    let mut img = build();
    img[LFANEW] = b'X';
    assert!(matches!(Pe::parse(&img).err(), Some(PeError::BadPeSignature(_))));
}

#[test]
fn rejects_pe32() {
    let mut img = build();
    w16(&mut img, LFANEW + 24, 0x10b);
    assert_eq!(Pe::parse(&img).err(), Some(PeError::NotPe32Plus { magic: 0x10b }));
}

#[test]
fn rejects_non_amd64_machine() {
    let mut img = build();
    w16(&mut img, LFANEW + 4, 0x14c);
    assert_eq!(Pe::parse(&img).err(), Some(PeError::UnsupportedMachine(0x14c)));
}

#[test]
fn rejects_lfanew_out_of_file_without_overflow() {
    for v in [FILE_LEN as u32, u32::MAX, u32::MAX - 3] {
        let mut img = build();
        w32(&mut img, 0x3c, v);
        assert!(matches!(Pe::parse(&img).err(), Some(PeError::Truncated { .. })), "lfanew={v:#x}");
    }
}

#[test]
fn every_header_truncation_is_an_error_not_a_panic() {
    let img = build();
    for len in 0..HEADERS_END {
        assert!(Pe::parse(&img[..len]).is_err(), "prefix of {len} bytes unexpectedly parsed");
    }
    assert!(Pe::parse(&img[..HEADERS_END]).is_ok());
}

#[test]
fn truncated_data_makes_tables_fail_but_headers_parse() {
    let img = build();
    // cut inside the import descriptors (0x1200..0x123c), keep exports intact
    let cut = at(0x1210);
    let pe = Pe::parse(&img[..cut]).unwrap();
    assert!(pe.exports().is_ok());
    assert!(matches!(pe.imports(), Err(PeError::Truncated { .. })));
    // cut before the export directory's name table
    let pe = Pe::parse(&img[..at(0x1048)]).unwrap();
    assert!(pe.exports().is_err());
    // cut so the section has no bytes at all
    let pe = Pe::parse(&img[..DATA_RAW]).unwrap();
    assert!(pe.exports().is_err());
    assert!(pe.imports().is_err());
}

#[test]
fn rejects_directories_pointing_at_unmapped_rvas() {
    let mut img = build();
    w32(&mut img, LFANEW + 24 + 112, 0x9000);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.exports().err(), Some(PeError::RvaUnmapped(0x9000)));

    let mut img = build();
    w32(&mut img, LFANEW + 24 + 120, 0x2000);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.imports().err(), Some(PeError::RvaNotFileBacked(0x2000)));
}

#[test]
fn rejects_import_name_and_thunk_in_unmapped_memory() {
    let mut img = build();
    w32(&mut img, at(0x1200) + 12, 0x7000);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.imports().err(), Some(PeError::RvaUnmapped(0x7000)));

    let mut img = build();
    w64(&mut img, at(0x1300), 0x7000);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.imports().err(), Some(PeError::RvaUnmapped(0x7000)));
}

#[test]
fn rejects_unterminated_string() {
    // 0x1400..0x1800 is filled with 'A' up to the end of the file-backed section, no NUL
    let mut img = build();
    for i in 0..0x400usize {
        img[at(0x1400) + i] = b'A';
    }
    w32(&mut img, at(0x1050), 0x1400);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.exports().err(), Some(PeError::BadString { rva: 0x1400 }));

    // a terminated string longer than the 4096-byte limit is also rejected
    let mut big = vec![0u8; 0x400 + 0x2000];
    big.copy_from_slice(&{
        let mut v = build();
        v.resize(0x400 + 0x2000, 0);
        v
    });
    w32(&mut big, 0x80 + 24 + 240 + 8, 0x2000);
    w32(&mut big, 0x80 + 24 + 240 + 16, 0x2000);
    for i in 0..0x1800usize {
        big[at(0x1400) + i] = b'A';
    }
    big[at(0x1400) + 0x1800] = 0;
    w32(&mut big, at(0x1050), 0x1400);
    let pe = Pe::parse(&big).unwrap();
    assert_eq!(pe.exports().err(), Some(PeError::BadString { rva: 0x1400 }));
}

#[test]
fn rejects_export_ordinal_index_out_of_range() {
    let mut img = build();
    w16(&mut img, at(0x105a), 9);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.exports().err(), Some(PeError::Malformed(_))));
}

#[test]
fn rejects_oversized_counts() {
    let mut img = build();
    w32(&mut img, LFANEW + 24 + 108, 17);
    assert!(matches!(Pe::parse(&img).err(), Some(PeError::LimitExceeded(_))));

    let mut img = build();
    w32(&mut img, at(0x1000) + 20, u32::MAX);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.exports().err(), Some(PeError::LimitExceeded(_))));
}

#[test]
fn rejects_reserved_bits_in_ordinal_thunk() {
    let mut img = build();
    w64(&mut img, at(0x1308), 0x8000_0001_0000_0010);
    let pe = Pe::parse(&img).unwrap();
    assert!(matches!(pe.imports().err(), Some(PeError::Malformed(_))));
}

#[test]
fn no_directories_means_empty_not_error() {
    let mut img = build();
    w32(&mut img, LFANEW + 24 + 112, 0);
    w32(&mut img, LFANEW + 24 + 120, 0);
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.exports().unwrap().is_empty());
    assert!(pe.imports().unwrap().is_empty());
}
