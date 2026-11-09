//! Tests of the .rsrc directory parser (Pe::resources, resource_data, strings, parse_string_block).
//!
//! Format source: "PE Format", section "The .rsrc Section" (Microsoft Learn,
//! https://learn.microsoft.com/en-us/windows/win32/debug/pe-format) and, for RT_STRING blocks,
//! "Resource Types" (https://learn.microsoft.com/en-us/windows/win32/menurc/resource-types).
//! Part 1 builds a PE by hand with a three-level tree. Part 2 checks the real FLEngine_x64.dll
//! ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; if FLENGINE_PE is set and the file is
//! missing the tests fail, if only the default path is missing they print SKIPPED and pass, as real_dll.rs
//! does). The real-DLL numbers were measured with an independent Python parser (lift/out-a2/work/dump_rsrc.py).

use fe_pe::{parse_string_block, Pe, PeError, ResourceId, StringEntry, RT_STRING};
use std::path::PathBuf;

const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const RSRC_RVA: u32 = 0x1000;
const RSRC_RAW: usize = 0x400;
const RSRC_LEN: usize = 0x800;
const FILE_LEN: usize = RSRC_RAW + RSRC_LEN;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// Writes a resource directory table header at `off` (relative to the directory) and its entries.
/// `entries`: (name-or-id field, offset field).
fn table(r: &mut [u8], off: usize, n_name: u16, n_id: u16, entries: &[(u32, u32)]) {
    w16(r, off + 12, n_name);
    w16(r, off + 14, n_id);
    for (i, (a, b)) in entries.iter().enumerate() {
        w32(r, off + 16 + 8 * i, *a);
        w32(r, off + 20 + 8 * i, *b);
    }
}

fn data_entry(r: &mut [u8], off: usize, data_off: u32, size: u32, cp: u32) {
    w32(r, off, RSRC_RVA + data_off);
    w32(r, off + 4, size);
    w32(r, off + 8, cp);
}

/// A RT_STRING block: 16 length-prefixed strings; `strs` gives (index in block, text).
fn string_block(strs: &[(usize, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..16 {
        match strs.iter().find(|(k, _)| *k == i) {
            Some((_, s)) => {
                let u: Vec<u16> = s.encode_utf16().collect();
                out.extend_from_slice(&(u.len() as u16).to_le_bytes());
                for c in u {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
            None => out.extend_from_slice(&0u16.to_le_bytes()),
        }
    }
    out
}

const HI: u32 = 0x8000_0000;

/// Tree (offsets relative to the directory start):
///   root @0x000: Name entry "TEXTX" -> table 0x100, ID entry 6 (RT_STRING) -> table 0x120
///   TEXTX @0x100: ID 5 -> lang table 0x140: ID 0x409 -> data entry 0x1a0 (6 bytes, codepage 1252)
///   6 @0x120: ID 1 -> lang table 0x160: ID 0x409 -> data entry 0x1b0 (block 1, ids 0..15)
///             ID 2 -> lang table 0x180: ID 0x409 -> 0x1c0 (block 2, ids 16..31), ID 0x804 -> 0x1d0
///   string "TEXTX" @0x200 (Resource Directory String: WORD length + UTF-16)
///   data @0x300 (TEXTX), 0x400, 0x500, 0x600
fn build(patch: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w64(&mut b, o + 24, 0x40_0000);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x2000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 128, RSRC_RVA);
    w32(&mut b, o + 132, RSRC_LEN as u32);
    let s0 = o + OPT_SIZE;
    b[s0..s0 + 5].copy_from_slice(b".rsrc");
    w32(&mut b, s0 + 8, RSRC_LEN as u32);
    w32(&mut b, s0 + 12, RSRC_RVA);
    w32(&mut b, s0 + 16, RSRC_LEN as u32);
    w32(&mut b, s0 + 20, RSRC_RAW as u32);
    w32(&mut b, s0 + 36, 0x4000_0040);

    let r = &mut b[RSRC_RAW..];
    table(r, 0x000, 1, 1, &[(HI | 0x200, HI | 0x100), (6, HI | 0x120)]);
    table(r, 0x100, 0, 1, &[(5, HI | 0x140)]);
    table(r, 0x120, 0, 2, &[(1, HI | 0x160), (2, HI | 0x180)]);
    table(r, 0x140, 0, 1, &[(0x409, 0x1a0)]);
    table(r, 0x160, 0, 1, &[(0x409, 0x1b0)]);
    table(r, 0x180, 0, 2, &[(0x409, 0x1c0), (0x804, 0x1d0)]);
    data_entry(r, 0x1a0, 0x300, 6, 1252);
    let blk1 = string_block(&[(1, "Hello"), (15, "x\0y")]);
    let blk2a = string_block(&[(0, "A")]);
    let blk2b = string_block(&[(0, "\u{3042}\u{1F600}"), (3, "zz")]);
    data_entry(r, 0x1b0, 0x400, blk1.len() as u32, 0);
    data_entry(r, 0x1c0, 0x500, blk2a.len() as u32, 0);
    data_entry(r, 0x1d0, 0x600, blk2b.len() as u32, 0);
    w16(r, 0x200, 5);
    for (i, c) in "TEXTX".encode_utf16().enumerate() {
        w16(r, 0x202 + 2 * i, c);
    }
    r[0x300..0x306].copy_from_slice(b"\x01\x02\x03\x04\x05\x06");
    r[0x400..0x400 + blk1.len()].copy_from_slice(&blk1);
    r[0x500..0x500 + blk2a.len()].copy_from_slice(&blk2a);
    r[0x600..0x600 + blk2b.len()].copy_from_slice(&blk2b);
    patch(&mut b);
    b
}

fn rs(off: usize) -> usize {
    RSRC_RAW + off
}

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[test]
fn three_level_tree_is_walked_in_directory_order() {
    let img = build(|_| {});
    let pe = Pe::parse(&img).unwrap();
    let leaves = pe.resources().unwrap();
    assert_eq!(leaves.len(), 4);
    assert_eq!(leaves[0].type_id, ResourceId::Name("TEXTX".into()));
    assert_eq!(leaves[0].name, ResourceId::Id(5));
    assert_eq!(leaves[0].language, ResourceId::Id(0x409));
    assert_eq!((leaves[0].size, leaves[0].codepage, leaves[0].data_rva), (6, 1252, RSRC_RVA + 0x300));
    assert_eq!(leaves[0].entry_rva, RSRC_RVA + 0x1a0);
    assert_eq!(pe.resource_data(&leaves[0]).unwrap(), b"\x01\x02\x03\x04\x05\x06");
    let tail: Vec<_> = leaves[1..]
        .iter()
        .map(|l| (l.type_id.clone(), l.name.clone(), l.language.clone()))
        .collect();
    assert_eq!(
        tail,
        vec![
            (ResourceId::Id(6), ResourceId::Id(1), ResourceId::Id(0x409)),
            (ResourceId::Id(6), ResourceId::Id(2), ResourceId::Id(0x409)),
            (ResourceId::Id(6), ResourceId::Id(2), ResourceId::Id(0x804)),
        ]
    );
}

#[test]
fn strings_follow_the_block_number_rule() {
    let img = build(|_| {});
    let pe = Pe::parse(&img).unwrap();
    let got = pe.strings().unwrap();
    // Block N holds IDs (N - 1) * 16 ..= (N - 1) * 16 + 15; unused slots (zero length) are not returned.
    let want = vec![
        StringEntry { id: 1, language: 0x409, text: u16s("Hello") },
        StringEntry { id: 15, language: 0x409, text: vec![b'x' as u16, 0, b'y' as u16] },
        StringEntry { id: 16, language: 0x409, text: u16s("A") },
        StringEntry { id: 16, language: 0x804, text: u16s("\u{3042}\u{1F600}") },
        StringEntry { id: 19, language: 0x804, text: u16s("zz") },
    ];
    assert_eq!(got, want);
    // The surrogate pair of U+1F600 is two UTF-16 units: counted as 2 in the length word.
    assert_eq!(got[3].text.len(), 3);
}

#[test]
fn string_block_edges() {
    // Block 0 does not exist (numbering starts at 1).
    assert!(matches!(parse_string_block(0, &[0; 32]), Err(PeError::Malformed(_))));
    // 16 empty strings.
    assert_eq!(parse_string_block(7, &[0; 32]).unwrap(), vec![]);
    // Padding after the 16th string is ignored (the real DLL has 2 bytes of it in 9 blocks).
    assert_eq!(parse_string_block(1, &[0; 34]).unwrap(), vec![]);
    // Fewer than 16 length words.
    assert!(matches!(parse_string_block(1, &[0; 30]), Err(PeError::Malformed(_))));
    // A length that runs past the end of the block.
    let mut b = vec![0u8; 42];
    b[0] = 5;
    assert_eq!(parse_string_block(1, &b).unwrap().len(), 1, "5 units + 15 length words fit into 42 bytes");
    let mut b = vec![0u8; 32];
    b[0] = 5;
    assert!(matches!(parse_string_block(1, &b), Err(PeError::Malformed(_))), "5 units leave no room for 15 length words");
    b[0] = 40;
    assert!(matches!(parse_string_block(1, &b), Err(PeError::Malformed(_))));
    // Block number overflow of (block - 1) * 16.
    assert!(matches!(parse_string_block(u32::MAX, &[0; 32]), Err(PeError::Malformed(_))));
    // ID of a string: (block - 1) * 16 + index.
    let blk = string_block(&[(4, "d")]);
    assert_eq!(parse_string_block(100, &blk).unwrap(), vec![(99 * 16 + 4, u16s("d"))]);
}

#[test]
fn no_resource_directory_is_an_empty_list() {
    let img = build(|b| {
        w32(b, LFANEW + 24 + 128, 0);
        w32(b, LFANEW + 24 + 132, 0);
    });
    let pe = Pe::parse(&img).unwrap();
    assert!(pe.resources().unwrap().is_empty());
    assert!(pe.strings().unwrap().is_empty());
}

#[test]
fn malformed_trees_are_errors_not_panics() {
    // A Name entry without the string flag.
    let img = build(|b| w32(b, rs(0x10), 0x200));
    assert!(matches!(Pe::parse(&img).unwrap().resources(), Err(PeError::Malformed(_))));
    // An ID entry with the string flag.
    let img = build(|b| w32(b, rs(0x18), HI | 6));
    assert!(matches!(Pe::parse(&img).unwrap().resources(), Err(PeError::Malformed(_))));
    // A data entry where the Type level needs a subdirectory.
    let img = build(|b| w32(b, rs(0x1c), 0x1a0));
    assert!(matches!(Pe::parse(&img).unwrap().resources(), Err(PeError::Malformed(_))));
    // A subdirectory at the language level.
    let img = build(|b| w32(b, rs(0x154), HI | 0x100));
    assert!(matches!(Pe::parse(&img).unwrap().resources(), Err(PeError::Malformed(_))));
    // An offset outside the resource directory size.
    let img = build(|b| w32(b, rs(0x1c), HI | 0x7f8));
    assert!(matches!(Pe::parse(&img).unwrap().resources(), Err(PeError::Malformed(_))));
    // A name string that runs past the directory.
    let img = build(|b| w16(b, rs(0x200), 0x7fff));
    assert!(Pe::parse(&img).unwrap().resources().is_err());
    // An entry count that runs past the directory.
    let img = build(|b| w16(b, rs(0x0e), 0xffff));
    assert!(Pe::parse(&img).unwrap().resources().is_err());
    // A data entry whose data lies outside the file: resources() lists it, resource_data() rejects it.
    let img = build(|b| w32(b, rs(0x1b0), 0x9000));
    let pe = Pe::parse(&img).unwrap();
    let l = &pe.resources().unwrap()[1];
    assert!(pe.resource_data(l).is_err());
    assert!(pe.strings().is_err());
    // A string block that is cut short inside the data.
    let img = build(|b| w32(b, rs(0x1b4), 10));
    assert!(matches!(Pe::parse(&img).unwrap().strings(), Err(PeError::Malformed(_))));
}

#[test]
fn every_truncation_of_the_resource_section_is_an_error_or_ok_never_a_panic() {
    let img = build(|_| {});
    for cut in (RSRC_RAW..FILE_LEN).step_by(7) {
        if let Ok(pe) = Pe::parse(&img[..cut]) {
            let _ = pe.resources();
            let _ = pe.strings();
        }
    }
}

// ---- real DLL ----

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
fn real_dll_resource_tree() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    assert_eq!(pe.data_dirs[2].rva, 0x16b1000);
    assert_eq!(pe.data_dirs[2].size, 0x20e400);
    let leaves = pe.resources().unwrap();
    let count = |t: u32| leaves.iter().filter(|l| l.type_id == ResourceId::Id(t)).count();
    // Leaves per type (independent Python parser): every name has exactly one language leaf.
    let want = [(1, 60), (2, 25), (3, 13), (5, 1), (6, 40), (10, 128), (12, 57), (14, 1), (16, 1), (24, 1)];
    for (t, n) in want {
        assert_eq!(count(t), n, "type {t}");
    }
    assert_eq!(leaves.len(), 327);
    // The 40 RT_STRING blocks are all language 0 (neutral); other types use 0, 0x409, 0x407, 0x80c, 0x2000.
    assert!(leaves
        .iter()
        .filter(|l| l.type_id == ResourceId::Id(RT_STRING))
        .all(|l| l.language == ResourceId::Id(0)));
    assert_eq!(leaves.iter().filter(|l| l.language == ResourceId::Id(0x409)).count(), 21 + 11 + 7 + 7 + 1 + 1);
    for l in &leaves {
        assert_eq!(pe.resource_data(l).unwrap().len(), l.size as usize);
    }
}

#[test]
fn real_dll_strings() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let strs = pe.strings().unwrap();
    assert_eq!(strs.len(), 625);
    assert!(strs.windows(2).all(|w| w[0].id < w[1].id), "ordered by ID, no duplicates in a one-language DLL");
    let get = |id: u32| strs.iter().find(|s| s.id == id).map(|s| String::from_utf16(&s.text).unwrap());
    // Values read by the independent Python parser (block (id >> 4) + 1, index id & 15).
    assert_eq!(get(64896).as_deref(), Some("Set Size Exceeded."));
    assert_eq!(get(64912).as_deref(), Some("Socket is not connected."));
    assert_eq!(get(64921).as_deref(), Some("Directory not empty"));
    assert_eq!(get(64922).as_deref(), Some("Host not found."));
    assert_eq!(get(64923).as_deref(), Some("%s is not a valid service."));
    assert_eq!(get(64925).as_deref(), Some("The requested IPVersion / Address family is not supported."));
    assert_eq!(get(64900), None);
}

#[test]
fn real_dll_string_blocks_are_blocks_of_sixteen() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    for l in pe.resources().unwrap() {
        if l.type_id == ResourceId::Id(RT_STRING) {
            let ResourceId::Id(block) = l.name else { panic!("string block with a name") };
            assert!(block >= 1);
            // Every block parses to 16 length-prefixed strings that fit into the data (padding at most 2 bytes
            // is seen in this DLL; the parser itself allows any padding).
            let data = pe.resource_data(&l).unwrap();
            let v = parse_string_block(block, data).unwrap();
            let used: usize = v.iter().map(|(_, t)| 2 * t.len()).sum();
            assert!(32 + used <= data.len() || 2 * 16 + used == data.len(), "block {block}");
            for (id, _) in &v {
                assert_eq!(id >> 4, block - 1);
            }
        }
    }
}
