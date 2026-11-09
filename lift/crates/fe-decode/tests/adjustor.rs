//! Adjustor thunks outside `.pdata` (decision 0051, docs/investigations/adjustor-thunks.md, adjustor-thunks-run.md).
//!
//! Part 1 decodes hand-made bytes. Part 2 checks the real FLEngine_x64.dll ($FLENGINE_PE, else
//! <repo root>/binaries/win64/FLEngine_x64.dll; with FLENGINE_PE set and the file missing the tests fail,
//! with the default missing they are skipped). Expected values were measured with the Python survey
//! lift/out-adj/survey2.py (pefile + capstone, independent of the Rust code) and with `fe-decode closure --pdata`.

use fe_decode::adjustor::{adjustor_seeds, adjustors, classify, decode_thunk, Reject};
use fe_decode::{analyze_with_pdata, analyze_with_pdata_adjustors, pdata_seeds, Text};
use fe_pe::Pe;
use std::path::PathBuf;

fn text(bytes: &[u8]) -> Text<'_> {
    Text::single(0x1000, bytes)
}

// ---- 1. the shape check on bytes ----

#[test]
fn add_rcx_minus_0x10_jmp_is_a_thunk() {
    // 0x403582 of the DLL: add rcx,-0x10 ; jmp +0x18c15
    let b = [0x48, 0x83, 0xc1, 0xf0, 0xe9, 0x15, 0x8c, 0x01, 0x00, 0xcc];
    let a = decode_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((a.pre, a.len, a.shape.as_str()), (1, 9, "add+jmp"));
    assert_eq!(a.target, 0x1009 + 0x18c15);
}

#[test]
fn sub_lea_and_mov_load_forms_are_thunks() {
    // sub rcx,8 ; jmp +0
    let b = [0x48, 0x83, 0xe9, 0x08, 0xe9, 0, 0, 0, 0];
    assert_eq!(decode_thunk(&text(&b), 0x1000).unwrap().shape, "sub+jmp");
    // lea rcx,[rcx-0x10] ; jmp +0
    let b = [0x48, 0x8d, 0x49, 0xf0, 0xe9, 0, 0, 0, 0];
    assert_eq!(decode_thunk(&text(&b), 0x1000).unwrap().shape, "lea+jmp");
    // mov rcx,[rcx+8] ; add rcx,-0x10 ; jmp +0
    let b = [0x48, 0x8b, 0x49, 0x08, 0x48, 0x83, 0xc1, 0xf0, 0xe9, 0, 0, 0, 0];
    let a = decode_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((a.pre, a.shape.as_str()), (2, "mov+add+jmp"));
}

#[test]
fn everything_else_is_not_a_thunk() {
    let cases: [(&str, &[u8]); 9] = [
        ("bare jmp", &[0xe9, 0, 0, 0, 0]),
        ("jmp [rip+x] (import thunk)", &[0xff, 0x25, 0, 0, 0, 0]),
        ("add then indirect jmp", &[0x48, 0x83, 0xc1, 0xf0, 0xff, 0xe0]),
        ("add then ret", &[0x48, 0x83, 0xc1, 0xf0, 0xc3]),
        ("add then call", &[0x48, 0x83, 0xc1, 0xf0, 0xe8, 0, 0, 0, 0]),
        ("add to memory", &[0x48, 0x83, 0x01, 0xf0, 0xe9, 0, 0, 0, 0]),
        ("lea with index", &[0x48, 0x8d, 0x0c, 0x41, 0xe9, 0, 0, 0, 0]),
        ("lea rip-relative", &[0x48, 0x8d, 0x0d, 0, 0, 0, 0, 0xe9, 0, 0, 0, 0]),
        ("invalid bytes", &[0x06, 0x06, 0x06]),
    ];
    for (what, b) in cases {
        assert_eq!(decode_thunk(&text(b), 0x1000), Err(Reject::NotAThunk), "{what}");
    }
    // four adjusting instructions: more than the three allowed
    let b = [0x48, 0x83, 0xc1, 1, 0x48, 0x83, 0xc1, 1, 0x48, 0x83, 0xc1, 1, 0x48, 0x83, 0xc1, 1, 0xe9, 0, 0, 0, 0];
    assert_eq!(decode_thunk(&text(&b), 0x1000), Err(Reject::NotAThunk), "four adjusting instructions");
    // a thunk that runs past the 16-byte window is cut off, so it is not a thunk
    let b = [0x48, 0x81, 0xc1, 1, 0, 0, 0, 0x48, 0x81, 0xc1, 1, 0, 0, 0, 0x48, 0x81, 0xc1, 1, 0, 0, 0, 0xe9, 0, 0, 0, 0];
    assert_eq!(decode_thunk(&text(&b), 0x1000), Err(Reject::NotAThunk), "longer than 16 bytes");
    assert_eq!(decode_thunk(&text(&[0x90]), 0x2000), Err(Reject::NotInText));
}

// ---- 2. the real DLL ----

const THUNKS: usize = 602;
const PDATA_FUNCTIONS: usize = 58_695;
const PDATA_INSTRUCTIONS: usize = 2_526_826;
const ADJ_FUNCTIONS: usize = 59_297;
const ADJ_INSTRUCTIONS: usize = 2_528_030;

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
fn the_three_thunks_of_the_stop_point_are_adjustor_seeds() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let seeds = adjustor_seeds(&pe).unwrap();
    for (va, target) in [(0x403579u64, 0x41c180u64), (0x403582, 0x41c1a0), (0x40358b, 0x41c1b0)] {
        assert!(seeds.contains(&va), "{va:#x}");
        let a = classify(&pe, va).unwrap();
        assert_eq!((a.target, a.pre, a.len, a.shape.as_str()), (target, 1, 9, "add+jmp"), "{va:#x}");
    }
    // not in .pdata: not a start, not inside a range (so no pdata seed names them)
    let p = pdata_seeds(&pe).unwrap();
    for va in [0x403579u64, 0x403582, 0x40358b] {
        assert!(!p.contains(&va));
    }
    // 0x403594 is padding: not a thunk. 0x403583 is inside 0x403582 and, without the REX prefix, reads as
    // `add ecx,-0x10; jmp` with the same target: the shape alone does not give the instruction boundary. What does is
    // that a DIR64 slot holds the address, and no slot holds 0x403583 (it is not a seed).
    assert_eq!(classify(&pe, 0x403594), Err(Reject::NotAThunk));
    assert_eq!(classify(&pe, 0x403583).unwrap().target, 0x41c1a0);
    assert!(!seeds.contains(&0x403583));
    assert!(!seeds.contains(&0x40357a));
    // a pdata start is not an adjustor (0x41c1a0 is the jmp target)
    assert_eq!(classify(&pe, 0x41c1a0), Err(Reject::IsPdataStart));
    assert_eq!(classify(&pe, 0x1000), Err(Reject::NotInText));
}

#[test]
fn adjustor_seed_set_is_602_ascending_add_thunks_to_pdata_starts() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let all = adjustors(&pe).unwrap();
    assert_eq!(all.len(), THUNKS);
    assert!(all.windows(2).all(|w| w[0].va < w[1].va));
    let p = pdata_seeds(&pe).unwrap();
    for a in &all {
        assert!(p.binary_search(&a.target).is_ok(), "{:#x} -> {:#x} is not a pdata start", a.va, a.target);
        assert!(p.binary_search(&a.va).is_err(), "{:#x} is a pdata start", a.va);
        assert_eq!((a.pre, a.shape.as_str()), (1, "add+jmp"), "{:#x}", a.va);
        assert!(a.len == 9 || a.len == 12, "{:#x} len {}", a.va, a.len);
    }
    assert_eq!(all[0].va, 0x403579);
    assert_eq!(all.last().unwrap().va, 0x136bf04);
}

#[test]
fn closure_with_adjustors_has_no_bad_decode_and_grows_by_the_thunks_only() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let base = analyze_with_pdata(&pe).unwrap();
    let adj = analyze_with_pdata_adjustors(&pe).unwrap();
    assert_eq!(base.closure.functions, PDATA_FUNCTIONS);
    assert_eq!(base.closure.instructions, PDATA_INSTRUCTIONS);
    assert_eq!(adj.closure.functions, ADJ_FUNCTIONS);
    assert_eq!(adj.closure.instructions, ADJ_INSTRUCTIONS);
    assert_eq!(adj.closure.decode_failures, 0);
    assert_eq!(adj.closure.out_of_text_flow, 0);
    assert_eq!(adj.closure.out_of_text_calls, 0);
    // roots: +602 seeds, and the new functions are exactly those 602 thunks: every jump target was already a function
    let seeds = adjustor_seeds(&pe).unwrap();
    assert_eq!(adj.closure.seeds_code.len() - base.closure.seeds_code.len(), THUNKS);
    assert_eq!(adj.closure.functions - base.closure.functions, THUNKS);
    let new_fns: Vec<u64> = adj.functions.keys().copied().filter(|v| !base.functions.contains_key(v)).collect();
    assert_eq!(new_fns, seeds, "the new functions are the thunks and nothing else");
    // two instructions each (add, jmp), and every old instruction is kept
    assert_eq!(adj.closure.instructions - base.closure.instructions, 2 * THUNKS);
    for v in base.insns.keys() {
        assert!(adj.insns.contains_key(v), "{v:#x} lost");
    }
    for f in &seeds {
        assert_eq!(adj.functions[f].len(), 2, "{f:#x}");
    }
    // the thunk of the stop point jumps to a known function
    assert_eq!(adj.functions[&0x403582], vec![0x403582, 0x403586]);
}
