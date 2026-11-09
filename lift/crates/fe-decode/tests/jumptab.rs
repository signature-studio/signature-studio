//! Annotated jump tables in the closure (`analyze_with_specs`, decision 0036). Synthetic PE for the
//! checks and the rejections; the real DLL for the four annotated jumps ($FLENGINE_DLL, else
//! binaries/win64/FLEngine_x64.dll; skipped when absent, as extra_seed.rs).

use fe_decode::jumptab::{Kind, Spec};
use fe_decode::{analyze_with_specs, Resolved, Select};
use fe_pe::Pe;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// `.text` at 0x1000 (VA IB+0x1000), 0x400 bytes.
/// 0x1000 `jmp rcx` (ff e1); 0x1010 table of 3 x i32 relative to 0x1010 -> 0x1020, 0x1024, 0x1028;
/// 0x1020 `ret`, 0x1024 `ret`, 0x1028 `ret`. The function is 0x1000..0x1030.
fn build(entry2: i32) -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 1);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x400);
    w32(&mut b, o + 56, 0x2000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let t = TEXT_RAW;
    b[t] = 0xff;
    b[t + 1] = 0xe1;
    w32(&mut b, t + 0x10, 0x10);
    w32(&mut b, t + 0x14, 0x14);
    w32(&mut b, t + 0x18, entry2 as u32);
    b[t + 0x20] = 0xc3;
    b[t + 0x24] = 0xc3;
    b[t + 0x28] = 0xc3;
    b
}

fn spec() -> Spec {
    Spec {
        jmp_va: IB + 0x1000,
        code: &[0xff, 0xe1],
        func: (IB + 0x1000, IB + 0x1030),
        table: IB + 0x1010,
        count: 3,
        kind: Kind::Rel32 { base: IB + 0x1010 },
    }
}

#[test]
fn table_targets_enter_the_closure_and_are_recorded() {
    let img = build(0x18);
    let pe = Pe::parse(&img).unwrap();
    let none = analyze_with_specs(&pe, &[], &[]).unwrap();
    assert_eq!(none.closure.instructions, 1);
    assert_eq!(none.resolved[&(IB + 0x1000)], Resolved::Unresolved);
    assert!(none.jumptabs.is_empty());

    let sp = [spec()];
    let a = analyze_with_specs(&pe, &[], &sp).unwrap();
    assert_eq!(a.closure.instructions, 4);
    assert_eq!(a.closure.jumptabs, 1);
    assert!(a.closure.jumptab_rejected.is_empty());
    assert_eq!(a.closure.register_jumps, 1);
    assert_eq!(a.closure.indirect.by_kind["jmp:table"], 1);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 0);
    let t = &a.jumptabs[&(IB + 0x1000)];
    assert_eq!(t.select, Select::Target);
    assert_eq!(t.cases, vec![(IB + 0x1020, IB + 0x1020), (IB + 0x1024, IB + 0x1024), (IB + 0x1028, IB + 0x1028)]);
}

#[test]
fn a_target_outside_the_function_drops_the_table() {
    // entry 2 -> 0x1010 + 0x40 = 0x1050, outside 0x1000..0x1030 (and past the bytes written)
    let img = build(0x40);
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_specs(&pe, &[], &[spec()]).unwrap();
    assert_eq!(a.closure.jumptabs, 0);
    assert_eq!(a.closure.instructions, 1);
    assert_eq!(a.closure.jumptab_rejected.len(), 1);
    assert!(a.closure.jumptab_rejected[0].1.contains("outside the annotated function"), "{:?}", a.closure.jumptab_rejected);
    assert!(a.jumptabs.is_empty());
}

#[test]
fn a_target_in_the_middle_of_an_instruction_drops_the_table() {
    // 0x1020 is `ret`; make 0x1024 a 3-byte instruction (xor eax,eax = 31 c0 is 2 bytes; use `mov eax,0` is 5).
    // entry 1 -> 0x1010 + 0x14 = 0x1024. Put `mov eax,imm32` at 0x1022: it covers 0x1022..0x1027 (0x1024 inside it),
    // and let entry 0 target 0x1022.
    let mut img = build(0x18);
    let t = TEXT_RAW;
    w32(&mut img, t + 0x10, 0x12);
    img[t + 0x20] = 0x90;
    img[t + 0x21] = 0x90;
    img[t + 0x22] = 0xb8;
    img[t + 0x23] = 0xc3;
    img[t + 0x24] = 0xc3;
    img[t + 0x25] = 0xc3;
    img[t + 0x26] = 0xc3;
    img[t + 0x27] = 0xc3;
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_specs(&pe, &[], &[spec()]).unwrap();
    assert_eq!(a.closure.jumptabs, 0, "{:?}", a.closure.jumptab_rejected);
    assert_eq!(a.closure.jumptab_rejected.len(), 1);
    assert!(a.jumptabs.is_empty());
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 1);
}

#[test]
fn a_spec_whose_bytes_differ_is_rejected() {
    let img = build(0x18);
    let pe = Pe::parse(&img).unwrap();
    let mut s = spec();
    s.code = &[0xff, 0xe0];
    let a = analyze_with_specs(&pe, &[], &[s]).unwrap();
    assert_eq!(a.closure.jumptabs, 0);
    assert_eq!(a.closure.jumptab_rejected.len(), 1);
    assert!(a.closure.jumptab_rejected[0].1.contains("bytes differ"));
}

fn load() -> Option<Vec<u8>> {
    let path = std::env::var("FLENGINE_DLL")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"));
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("skip: {} not found", path.display());
            None
        }
    }
}

#[test]
fn real_dll_four_tables_pass_all_checks() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let with = fe_decode::analyze(&pe).unwrap();
    let without = analyze_with_specs(&pe, &[], &[]).unwrap();
    // seeds of the default closure reach f_40d210 only; the three switch functions are not in it
    assert!(with.closure.jumptab_rejected.is_empty(), "{:?}", with.closure.jumptab_rejected);
    assert!(with.jumptabs.contains_key(&0x40d253));
    assert_eq!(with.jumptabs[&0x40d253].cases.len(), 8);
    assert_eq!(with.jumptabs[&0x40d253].select, Select::Slot);
    // The 10 instructions that the annotation added to this closure (decision 0036) are found without it since decision 0102 W3 (the inline
    // absolute-pointer table of `lea B,[rip+k]; jmp [B+I*8+d]`, read from the relocations; `without` has inline_tables 1): the difference is 0.
    assert_eq!(with.closure.instructions - without.closure.instructions, 0);
    assert_eq!(with.closure.functions, without.closure.functions);

    let all = fe_decode::analyze_with_pdata(&pe).unwrap();
    let all0 = {
        let seeds = fe_decode::pdata_seeds(&pe).unwrap();
        analyze_with_specs(&pe, &seeds, &[]).unwrap()
    };
    assert!(all.closure.jumptab_rejected.is_empty(), "{:?}", all.closure.jumptab_rejected);
    assert_eq!(all.closure.jumptabs, 4);
    assert_eq!(all.jumptabs[&0x5e5eb6].cases.len(), 5, "35 entries, 5 distinct targets");
    assert_eq!(all.jumptabs[&0x5e9ae8].cases.len(), 31);
    assert_eq!(all.jumptabs[&0x5f1499].cases.len(), 6);
    assert_eq!(all.closure.functions, all0.closure.functions);
    assert_eq!(all.closure.decode_failures, 0);
    assert_eq!(all.closure.out_of_text_flow, 0);
    // 3,632 before decision 0102 W3; the 10 instructions of f_40d210's table are in `all0` too now (see the first closure above).
    assert_eq!(all.closure.instructions - all0.closure.instructions, 3_622);
    // every new instruction lies inside one of the four functions
    let spans = [(0x40d210u64, 0x40d31bu64), (0x5e5860, 0x5e6dd0), (0x5e8da0, 0x5eb3d4), (0x5f05f0, 0x5f167c)];
    for va in all.insns.keys().filter(|v| !all0.insns.contains_key(v)) {
        assert!(spans.iter().any(|&(b, e)| *va >= b && *va < e), "new instruction {va:#x} outside the four functions");
    }
}
