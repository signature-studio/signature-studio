//! Decision 0100 (docs/decisions/0100-wall-lift-gaps.md): the virtual-call thunks outside `.pdata` (`adjustor::vcall_thunks`, `vcall_seeds`)
//! and the landings of the guest's setjmp (`setjmp::is_setjmp_entry`, `setjmp::landings`).
//!
//! Part 1 decodes hand-made bytes. Part 2 checks the real FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll;
//! with FLENGINE_PE set and the file missing the tests fail, with the default missing they are skipped). The 82 thunks were found and counted
//! independently with the Python survey lift/out-wallL/py/survey5.py (own PE reader + capstone), which does not use the Rust code.

use fe_decode::adjustor::{classify_vcall, decode_vcall_thunk, vcall_seeds, vcall_thunks, Reject};
use fe_decode::setjmp::{is_setjmp_entry, landings, Landing};
use fe_decode::{analyze_with_pdata_adjustors, analyze_with_seeds, pdata_seeds, Text};
use fe_pe::Pe;
use std::path::PathBuf;

fn text(bytes: &[u8]) -> Text<'_> {
    Text::single(0x1000, bytes)
}

// ---- 1. the thunk shape on bytes ----

#[test]
fn add_mov_jmp_through_the_loaded_register_is_a_thunk() {
    // 0x477e73 of the DLL: add rcx,-0x10 ; mov rax,[rcx] ; jmp [rax+0x10]
    let b = [0x48, 0x83, 0xc1, 0xf0, 0x48, 0x8b, 0x01, 0xff, 0x60, 0x10, 0xcc];
    let t = decode_vcall_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((t.pre, t.len, t.shape.as_str()), (1, 10, "add+mov+jmp"));
    // add rcx,-0x550 (imm32) ; mov rax,[rcx] ; jmp [rax+0x298] (disp32): the longest form seen, 16 bytes
    let b = [0x48, 0x81, 0xc1, 0xb0, 0xfa, 0xff, 0xff, 0x48, 0x8b, 0x01, 0xff, 0xa0, 0x98, 0x02, 0x00, 0x00];
    let t = decode_vcall_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((t.pre, t.len), (1, 16));
    // no adjustment: mov rax,[rcx] ; jmp [rax+8]
    let b = [0x48, 0x8b, 0x01, 0xff, 0x60, 0x08];
    let t = decode_vcall_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((t.pre, t.len, t.shape.as_str()), (0, 6, "mov+jmp"));
    // two adjustments, and another register pair: sub rcx,8 ; add rcx,-8 ; mov rdx,[rcx+8] ; jmp [rdx]
    let b = [0x48, 0x83, 0xe9, 0x08, 0x48, 0x83, 0xc1, 0xf8, 0x48, 0x8b, 0x51, 0x08, 0xff, 0x22];
    let t = decode_vcall_thunk(&text(&b), 0x1000).unwrap();
    assert_eq!((t.pre, t.len, t.shape.as_str()), (2, 14, "sub+add+mov+jmp"));
}

#[test]
fn everything_else_is_not_a_virtual_call_thunk() {
    let cases: [(&str, &[u8]); 14] = [
        ("jmp through a register", &[0x48, 0x8b, 0x01, 0xff, 0xe0]),
        ("jmp through a rip-relative slot", &[0x48, 0x8b, 0x01, 0xff, 0x25, 0, 0, 0, 0]),
        ("jmp base is not the loaded register", &[0x48, 0x8b, 0x01, 0xff, 0x62, 0x08]),
        ("jmp with an index", &[0x48, 0x8b, 0x01, 0xff, 0x24, 0xc8]),
        ("32-bit load", &[0x8b, 0x01, 0xff, 0x60, 0x08]),
        ("load with an index", &[0x48, 0x8b, 0x04, 0xd1, 0xff, 0x60, 0x08]),
        ("rip-relative load", &[0x48, 0x8b, 0x05, 0, 0, 0, 0, 0xff, 0x60, 0x08]),
        ("segment-prefixed load", &[0x65, 0x48, 0x8b, 0x01, 0xff, 0x60, 0x08]),
        ("three adjustments", &[0x48, 0x83, 0xc1, 1, 0x48, 0x83, 0xc1, 1, 0x48, 0x83, 0xc1, 1, 0x48, 0x8b, 0x01, 0xff, 0x60, 8]),
        ("a call in the middle", &[0x48, 0x8b, 0x01, 0xe8, 0, 0, 0, 0, 0xff, 0x60, 0x08]),
        ("direct jmp (the decision 0051 family)", &[0x48, 0x83, 0xc1, 0xf0, 0xe9, 0, 0, 0, 0]),
        ("adjusting after the load", &[0x48, 0x8b, 0x01, 0x48, 0x83, 0xc0, 8, 0xff, 0x60, 0x08]),
        ("load without a jmp", &[0x48, 0x8b, 0x01, 0xc3]),
        ("invalid bytes", &[0x06, 0x06, 0x06]),
    ];
    for (what, b) in cases {
        assert_eq!(decode_vcall_thunk(&text(b), 0x1000), Err(Reject::NotAThunk), "{what}");
    }
    assert_eq!(decode_vcall_thunk(&text(&[0x90]), 0x2000), Err(Reject::NotInText));
}

// ---- 2. the setjmp shape on bytes ----

fn setjmp_body() -> Vec<u8> {
    let mut b = vec![
        0x48, 0x8b, 0x04, 0x24, // mov rax,[rsp]
        0x48, 0x8d, 0x54, 0x24, 0x08, // lea rdx,[rsp+8]
        0x48, 0x89, 0x01, // mov [rcx],rax
        0x48, 0x89, 0x51, 0x08, // mov [rcx+8],rdx
        0x48, 0x89, 0x59, 0x10, // rbx
        0x48, 0x89, 0x69, 0x18, // rbp
        0x48, 0x89, 0x71, 0x20, // rsi
        0x48, 0x89, 0x79, 0x28, // rdi
        0x4c, 0x89, 0x61, 0x30, // r12
        0x4c, 0x89, 0x69, 0x38, // r13
        0x4c, 0x89, 0x71, 0x40, // r14
        0x4c, 0x89, 0x79, 0x48, // r15
    ];
    b.extend([0x31, 0xc0, 0xc3]); // xor eax,eax ; ret
    b
}

#[test]
fn the_setjmp_shape_is_recognised_directly_and_behind_a_trampoline() {
    let b = setjmp_body();
    assert!(is_setjmp_entry(&text(&b), 0x1000));
    // the trampoline of the guest: lea rcx,[rcx+0x38] ; jmp setjmp
    let mut t = vec![0x48, 0x8d, 0x49, 0x38, 0xe9, 0x07, 0x00, 0x00, 0x00]; // jmp 0x1010
    t.resize(0x10, 0xcc);
    t.extend(&b);
    assert!(is_setjmp_entry(&text(&t), 0x1000));
    assert!(is_setjmp_entry(&text(&t), 0x1010));
    // a bare jmp trampoline
    let mut t = vec![0xe9, 0x0b, 0x00, 0x00, 0x00];
    t.resize(0x10, 0xcc);
    t.extend(&b);
    assert!(is_setjmp_entry(&text(&t), 0x1000));
}

#[test]
fn near_misses_of_the_setjmp_shape_are_not_setjmp() {
    let good = setjmp_body();
    // each store changed in turn: wrong slot, wrong register
    for (at, what) in [(11usize, "[rcx] store"), (14, "[rcx+8] store"), (18, "rbx slot"), (30, "rdi slot"), (34, "r12 slot"), (46, "r15 slot")] {
        let mut b = good.clone();
        b[at] ^= 0x08;
        assert!(!is_setjmp_entry(&text(&b), 0x1000), "{what}");
    }
    // no xor eax,eax before the ret
    let mut b = good.clone();
    let n = b.len();
    b[n - 3] = 0x90;
    b[n - 2] = 0x90;
    assert!(!is_setjmp_entry(&text(&b), 0x1000), "no xor");
    // a call before the ret
    let mut b = good[..good.len() - 3].to_vec();
    b.extend([0xe8, 0, 0, 0, 0, 0x31, 0xc0, 0xc3]);
    assert!(!is_setjmp_entry(&text(&b), 0x1000), "call in the tail");
    // too many hops: three trampolines in front
    let mut t = Vec::new();
    for k in 0..3u8 {
        t.extend([0xe9, 0x0b, 0x00, 0x00, 0x00]);
        t.resize(0x10 * (k as usize + 1), 0xcc);
    }
    t.extend(&good);
    assert!(!is_setjmp_entry(&text(&t), 0x1000), "three hops");
    // not code
    assert!(!is_setjmp_entry(&text(&[0xc3]), 0x1000));
    assert!(!is_setjmp_entry(&text(&good), 0x3000));
}

// ---- 3. the real DLL ----

const THUNKS: usize = 82;
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
fn the_stop_point_of_unit_296_is_a_virtual_call_thunk_seed() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let all = vcall_thunks(&pe).unwrap();
    let t = classify_vcall(&pe, 0x477e73).unwrap();
    assert_eq!((t.pre, t.len, t.shape.as_str()), (1, 10, "add+mov+jmp"));
    assert!(all.contains(&t));
    // not in .pdata: not a start, not inside a range
    let p = pdata_seeds(&pe).unwrap();
    assert!(!p.contains(&0x477e73));
    // 0x477e74 reads as the same shape without the REX prefix (add ecx,-0x10 ; mov rax,[rcx] ; jmp [rax+0x10]); no byte before it is int3
    // and no thunk ends there, so it is not an instruction boundary that the decode alone can prove, and no slot holds it
    assert_eq!(classify_vcall(&pe, 0x477e74), Err(Reject::NotAnchored));
    assert!(!all.iter().any(|t| t.va == 0x477e74));
    // a .pdata start, and an address outside the code
    assert_eq!(classify_vcall(&pe, 0x41c1a0), Err(Reject::IsPdataStart));
    assert_eq!(classify_vcall(&pe, 0x1000), Err(Reject::NotInText));
}

#[test]
fn the_virtual_call_thunk_set_is_82_ascending_anchored_thunks_outside_pdata() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let all = vcall_thunks(&pe).unwrap();
    assert_eq!(all.len(), THUNKS);
    assert!(all.windows(2).all(|w| w[0].va + w[0].len as u64 <= w[1].va));
    let p = pdata_seeds(&pe).unwrap();
    for t in &all {
        assert!(p.binary_search(&t.va).is_err(), "{:#x} is a pdata start", t.va);
        assert_eq!((t.pre, t.shape.as_str()), (1, "add+mov+jmp"), "{:#x}", t.va);
        assert!([9, 10, 13, 16].contains(&t.len), "{:#x} len {}", t.va, t.len);
    }
    assert_eq!(all[0].va, 0x458841);
    assert_eq!(all.last().unwrap().va, 0x136b7c1);
    // the lengths: 7 x 9, 47 x 10, 25 x 13, 3 x 16 (the Python survey's count)
    let n = |l: usize| all.iter().filter(|t| t.len == l).count();
    assert_eq!((n(9), n(10), n(13), n(16)), (7, 47, 25, 3));
}

#[test]
fn closure_with_the_thunk_seeds_grows_by_the_thunks_only_and_old_functions_are_unchanged() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let base = analyze_with_pdata_adjustors(&pe).unwrap();
    assert_eq!(base.closure.functions, ADJ_FUNCTIONS);
    assert_eq!(base.closure.instructions, ADJ_INSTRUCTIONS);
    let seeds = vcall_seeds(&pe, &base).unwrap();
    assert_eq!(seeds.len(), THUNKS, "no thunk is reached by the closure of the other seeds");
    let mut extra = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    extra.extend(&seeds);
    let with = analyze_with_seeds(&pe, &extra).unwrap();
    assert_eq!(with.closure.decode_failures, 0);
    assert_eq!(with.closure.out_of_text_flow, 0);
    assert_eq!(with.closure.out_of_text_calls, 0);
    assert_eq!(with.closure.functions - base.closure.functions, THUNKS);
    assert_eq!(with.closure.instructions - base.closure.instructions, 3 * THUNKS);
    let new_fns: Vec<u64> = with.functions.keys().copied().filter(|v| !base.functions.contains_key(v)).collect();
    assert_eq!(new_fns, seeds, "the new functions are the thunks and nothing else");
    for f in &seeds {
        assert_eq!(with.functions[f].len(), 3, "{f:#x}");
    }
    // every old function has the same instructions, and no old instruction changed owner
    for (f, insns) in &base.functions {
        assert_eq!(&with.functions[f], insns, "{f:#x}");
    }
    assert_eq!(base.insns.len() + 3 * THUNKS, with.insns.len());
    // the `jmp [rax+d]` of a thunk is an indirect jump that the closure does not follow: nothing else was added
    assert_eq!(with.closure.indirect_jumps_other - base.closure.indirect_jumps_other, THUNKS);
}

#[test]
fn a_thunk_that_is_already_an_entry_is_not_a_new_seed() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let mut extra = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    extra.push(0x477e73);
    let base = analyze_with_seeds(&pe, &extra).unwrap();
    let seeds = vcall_seeds(&pe, &base).unwrap();
    assert_eq!(seeds.len(), THUNKS - 1);
    assert!(!seeds.contains(&0x477e73));
}

#[test]
fn the_guest_setjmp_and_its_one_landing() {
    let Some(d) = load() else { return };
    let pe = Pe::parse(&d).unwrap();
    let text = Text::from_pe(&pe).unwrap();
    // 0x412680 setjmp, 0x412760 its trampoline (lea rcx,[rcx+0x38] ; jmp 0x412680); the longjmp and its caller are not
    for (va, want) in [(0x412680u64, true), (0x412760, true), (0x4126f0, false), (0x412770, false), (0x4128b0, false), (0x1376820, false)] {
        assert_eq!(is_setjmp_entry(&text, va), want, "{va:#x}");
    }
    let base = analyze_with_pdata_adjustors(&pe).unwrap();
    let l = landings(&text, &base.insns);
    assert_eq!(l.len(), 1, "{l:?}");
    assert_eq!(l[&0x1376840], Landing { call_va: 0x137683b, callee: 0x412760 });
    // the landing is an instruction of the entry function, and not an entry itself
    assert!(base.insns.contains_key(&0x1376840));
    assert!(!base.functions.contains_key(&0x1376840));
    assert!(base.functions[&0x1376820].contains(&0x1376840));
}
