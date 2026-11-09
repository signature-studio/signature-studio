//! Executable ranges from the section flags and the reloc seeds (decision 0074). Synthetic PE32+ only
//! (no real DLL needed). Layout, ImageBase 0x1_4000_0000 (IB):
//!
//! | section | RVA    | flags       | content |
//! |---------|--------|-------------|---------|
//! | .text   | 0x1000 | 0x60000020  | 0x1000 `call 0x2000; call 0x4000; ret` (the entry) |
//! | IPPCODE | 0x2000 | 0x60000020  | 0x2000 `ret` (called), 0x2010 `nop; ret`, 0x2020 `ret`, 0x2030 `ret` |
//! | .data   | 0x3000 | 0xc0000040  | DIR64 slots: 0x3000 -> 0x2010, 0x3008 -> 0x2020, 0x3010 -> 0x4000, 0x3018 -> 0x3000, 0x3020 -> 0x2010 |
//! | NOEXEC  | 0x4000 | 0x40000020  | 0x4000 `ret` (CNT_CODE and READ, no MEM_EXECUTE) |
//! | .xdata  | 0x5000 | 0x40000040  | UNWIND_INFO (version 1, no codes) at 0x5000 |
//! | .pdata  | 0x6000 | 0x40000040  | starts 0x1000, 0x2000, 0x2010, 0x2030, 0x4000 |
//! | .reloc  | 0x7000 | 0x42000040  | one block, page 0x3000, five DIR64 entries and one ABSOLUTE |
//!
//! 0x2020 is in code but is no `.pdata` start; 0x2030 is a `.pdata` start that no relocation names; 0x4000 is a
//! `.pdata` start in a section without MEM_EXECUTE.

use fe_decode::{analyze, analyze_with_pdata, analyze_with_reloc_seeds, pdata_seeds, relocseeds, Text};
use fe_pe::{Pe, IMAGE_SCN_CNT_CODE, IMAGE_SCN_MEM_EXECUTE};

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const NSEC: usize = 7;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
const FILE_LEN: usize = 0x1200;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

fn section(b: &mut [u8], i: usize, name: &[u8], rva: u32, raw: u32, flags: u32) {
    let s = SEC_TABLE + 40 * i;
    b[s..s + name.len()].copy_from_slice(name);
    w32(b, s + 8, 0x200);
    w32(b, s + 12, rva);
    w32(b, s + 16, 0x200);
    w32(b, s + 20, raw);
    w32(b, s + 36, flags);
}

fn set_flags(b: &mut [u8], i: usize, flags: u32) {
    w32(b, SEC_TABLE + 40 * i + 36, flags);
}

fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, NSEC as u16);
    w16(&mut b, f + 16, OPT_SIZE as u16);
    w16(&mut b, f + 18, 0x2022);
    let o = LFANEW + 24;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, 0x1000); // entry point
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x8000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 2);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 112 + 8 * 3, 0x6000); // exception directory
    w32(&mut b, o + 116 + 8 * 3, 12 * 5);
    w32(&mut b, o + 112 + 8 * 5, 0x7000); // base relocations
    w32(&mut b, o + 116 + 8 * 5, 20);

    section(&mut b, 0, b".text", 0x1000, 0x400, 0x6000_0020);
    section(&mut b, 1, b"IPPCODE", 0x2000, 0x600, 0x6000_0020);
    section(&mut b, 2, b".data", 0x3000, 0x800, 0xc000_0040);
    section(&mut b, 3, b"NOEXEC", 0x4000, 0xa00, 0x4000_0020);
    section(&mut b, 4, b".xdata", 0x5000, 0xc00, 0x4000_0040);
    section(&mut b, 5, b".pdata", 0x6000, 0xe00, 0x4000_0040);
    section(&mut b, 6, b".reloc", 0x7000, 0x1000, 0x4200_0040);

    // .text: call 0x2000 ; call 0x4000 ; ret
    let t = 0x400;
    b[t] = 0xe8;
    w32(&mut b, t + 1, 0x2000 - 0x1005);
    b[t + 5] = 0xe8;
    w32(&mut b, t + 6, 0x4000 - 0x100a);
    b[t + 10] = 0xc3;
    // IPPCODE
    let i = 0x600;
    b[i] = 0xc3;
    b[i + 0x10] = 0x90;
    b[i + 0x11] = 0xc3;
    b[i + 0x20] = 0xc3;
    b[i + 0x30] = 0xc3;
    // .data slots
    let d = 0x800;
    w64(&mut b, d, IB + 0x2010);
    w64(&mut b, d + 8, IB + 0x2020);
    w64(&mut b, d + 0x10, IB + 0x4000);
    w64(&mut b, d + 0x18, IB + 0x3000);
    w64(&mut b, d + 0x20, IB + 0x2010);
    // NOEXEC
    b[0xa00] = 0xc3;
    // .xdata: UNWIND_INFO version 1, no flags, no codes
    b[0xc00] = 1;
    // .pdata
    for (k, (begin, end)) in [(0x1000u32, 0x100bu32), (0x2000, 0x2001), (0x2010, 0x2012), (0x2030, 0x2031), (0x4000, 0x4001)]
        .into_iter()
        .enumerate()
    {
        let at = 0xe00 + 12 * k;
        w32(&mut b, at, begin);
        w32(&mut b, at + 4, end);
        w32(&mut b, at + 8, 0x5000);
    }
    // .reloc: page 0x3000, DIR64 at +0, +8, +0x10, +0x18, +0x20, then one ABSOLUTE
    let r = 0x1000;
    w32(&mut b, r, 0x3000);
    w32(&mut b, r + 4, 20);
    for (k, off) in [0u16, 8, 0x10, 0x18, 0x20].into_iter().enumerate() {
        w16(&mut b, r + 8 + 2 * k, 0xa000 | off);
    }
    w16(&mut b, r + 18, 0x0000);
    b
}

#[test]
fn flags_decide_not_the_name() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let got: Vec<(&str, bool, bool)> =
        pe.sections.iter().map(|s| (s.name.as_str(), s.is_executable(), s.has_code_content())).collect();
    assert_eq!(
        got,
        vec![
            (".text", true, true),
            ("IPPCODE", true, true),
            (".data", false, false),
            ("NOEXEC", false, true),
            (".xdata", false, false),
            (".pdata", false, false),
            (".reloc", false, false),
        ]
    );
    assert_eq!((IMAGE_SCN_CNT_CODE, IMAGE_SCN_MEM_EXECUTE), (0x20, 0x2000_0000));
}

#[test]
fn text_has_one_range_per_executable_section_in_va_order() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let t = Text::from_pe(&pe).unwrap();
    let got: Vec<(u64, usize)> = t.ranges().iter().map(|r| (r.va, r.bytes.len())).collect();
    assert_eq!(got, vec![(IB + 0x1000, 0x200), (IB + 0x2000, 0x200)]);
    assert!(t.contains(IB + 0x1000) && t.contains(IB + 0x11ff));
    assert!(t.contains(IB + 0x2000) && t.contains(IB + 0x21ff));
    assert!(!t.contains(IB + 0x1200) && !t.contains(IB + 0x1fff) && !t.contains(IB + 0x2200));
    // no executable flag: not code, even with IMAGE_SCN_CNT_CODE, and not data either
    assert!(!t.contains(IB + 0x3000) && !t.contains(IB + 0x4000) && !t.contains(IB + 0x6000));
    assert_eq!(t.slice(IB + 0x2000, 2).unwrap(), &[0xc3, 0x00]);
    assert!(t.slice(IB + 0x21ff, 2).is_none(), "a slice does not cross the end of its range");
}

#[test]
fn closure_enters_the_second_executable_section_and_not_the_unflagged_one() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    let a = analyze(&pe).unwrap();
    let fns: Vec<u64> = a.functions.keys().copied().collect();
    assert_eq!(fns, vec![IB + 0x1000, IB + 0x2000]);
    assert_eq!(a.functions[&(IB + 0x1000)], vec![IB + 0x1000, IB + 0x1005, IB + 0x100a]);
    assert_eq!(a.functions[&(IB + 0x2000)], vec![IB + 0x2000]);
    // the call into NOEXEC is out of code; nothing there is decoded
    assert_eq!(a.closure.out_of_text_calls, 1);
    assert_eq!(a.closure.out_of_text_flow, 0);
    assert_eq!(a.closure.decode_failures, 0);
    assert!(!a.insns.contains_key(&(IB + 0x4000)));
    assert_eq!(a.closure.instructions, 4);
}

#[test]
fn a_section_with_only_the_execute_flag_counts_and_one_without_it_does_not() {
    // IPPCODE with MEM_EXECUTE but without CNT_CODE: still code
    let mut img = build();
    set_flags(&mut img, 1, 0x2000_0000);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(Text::from_pe(&pe).unwrap().ranges().len(), 2);
    assert!(analyze(&pe).unwrap().functions.contains_key(&(IB + 0x2000)));

    // IPPCODE with CNT_CODE and READ only: not code, the call to it is out of code
    let mut img = build();
    set_flags(&mut img, 1, 0x4000_0020);
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(Text::from_pe(&pe).unwrap().ranges().len(), 1);
    let a = analyze(&pe).unwrap();
    assert!(!a.functions.contains_key(&(IB + 0x2000)));
    assert_eq!(a.closure.out_of_text_calls, 2);

    // a section named .text without MEM_EXECUTE is not code
    let mut img = build();
    set_flags(&mut img, 0, 0x4000_0040);
    let pe = Pe::parse(&img).unwrap();
    let t = Text::from_pe(&pe).unwrap();
    assert_eq!(t.ranges().len(), 1);
    assert_eq!(t.ranges()[0].va, IB + 0x2000);

    // no executable section at all is an error
    let mut img = build();
    set_flags(&mut img, 0, 0x4000_0040);
    set_flags(&mut img, 1, 0x4000_0040);
    let pe = Pe::parse(&img).unwrap();
    assert!(Text::from_pe(&pe).is_err());
}

#[test]
fn pdata_seeds_are_the_starts_inside_executable_ranges() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    // 0x4000 is a .pdata start, but its section has no MEM_EXECUTE
    assert_eq!(pe.function_start_vas().unwrap().len(), 5);
    assert_eq!(pdata_seeds(&pe).unwrap(), vec![IB + 0x1000, IB + 0x2000, IB + 0x2010, IB + 0x2030]);
    let a = analyze_with_pdata(&pe).unwrap();
    let fns: Vec<u64> = a.functions.keys().copied().collect();
    assert_eq!(fns, vec![IB + 0x1000, IB + 0x2000, IB + 0x2010, IB + 0x2030]);
}

#[test]
fn reloc_seeds_are_only_the_values_that_are_pdata_starts_in_code() {
    let img = build();
    let pe = Pe::parse(&img).unwrap();
    assert_eq!(pe.dir64_relocation_vas().unwrap().len(), 5);
    let r = relocseeds::reloc_seeds(&pe).unwrap();
    assert_eq!(r.slots, 5);
    assert_eq!(r.unreadable_slots, 0);
    // distinct values in code: 0x2010 (twice) and 0x2020; 0x4000 and 0x3000 are not in code
    assert_eq!(r.in_code, 2);
    assert_eq!(r.seeds, vec![IB + 0x2010]);
    assert_eq!(r.not_function, vec![IB + 0x2020]);
    assert_eq!(relocseeds::reloc_pdata_seeds(&pe).unwrap(), vec![IB + 0x2010]);

    let a = analyze_with_reloc_seeds(&pe).unwrap();
    let fns: Vec<u64> = a.functions.keys().copied().collect();
    // entry, its callee, and the reloc seed. Not 0x2020 (no .pdata start) and not 0x2030 (no relocation names it)
    assert_eq!(fns, vec![IB + 0x1000, IB + 0x2000, IB + 0x2010]);
    assert!(!a.functions.contains_key(&(IB + 0x2020)));
    assert!(!a.functions.contains_key(&(IB + 0x2030)));
    assert!(!a.functions.contains_key(&(IB + 0x4000)));
    assert_eq!(a.functions[&(IB + 0x2010)], vec![IB + 0x2010, IB + 0x2011]);
    // the seed shows up as a seed, not as a call target
    assert!(a.closure.seeds_code.iter().any(|(_, v)| *v == IB + 0x2010));
}

#[test]
fn no_relocation_directory_gives_no_reloc_seeds() {
    let mut img = build();
    let o = LFANEW + 24;
    w32(&mut img, o + 116 + 8 * 5, 0);
    let pe = Pe::parse(&img).unwrap();
    let r = relocseeds::reloc_seeds(&pe).unwrap();
    assert_eq!((r.slots, r.in_code, r.seeds.len(), r.not_function.len()), (0, 0, 0, 0));
    let a = analyze_with_reloc_seeds(&pe).unwrap();
    assert_eq!(a.functions.len(), 2);
}
