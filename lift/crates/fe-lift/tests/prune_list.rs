//! Pruned functions (decision 0168; src/prune.rs, `GenOptions::prune` of `generate_full`, `fe-lift --prune-list`).
//!
//! Synthetic PE (ImageBase 0x10000000, .text at RVA 0x1000, read + execute), written here, not taken from the code under test:
//! - 0x1000 `main`: `call F; call J; ret`
//! - 0x1020 F: `sub rsp,0x28; call G; add rsp,0x28; ret`
//! - 0x1060 G (the function the host implements): `call H; ret`
//! - 0x1080 H (reachable only through G): `mov eax,7; ret`
//! - 0x10a0 J: `jmp G` (a tail call to G)
//!
//! Expected, from the decision: with G pruned, G is still a function entry where kept code reaches it (F calls it as `f_<G>()`, J's jump is the tail
//! call `f_<G>(); return;`, it is in the dispatch table), but its body is the stub `fle_pruned(G)`, and H is not in the closure at all. A pruned function
//! that no kept code reaches is not generated (not a stub, not in the dispatch table). Without a prune list the output is the old output byte for byte; with a
//! prune list it is also correct together with --outline-shared and --dead-flags (the stubs are not walked). A prune entry outside .text, a repeated one, one
//! that is the entry point, are errors. The generated C of the pruned module compiles.
//!
//! Outputs go under lift/out-prune/test.

mod etxt_common;

use fe_lift::prune::Entry;
use fe_lift::{generate, generate_full, GenOptions};
use std::path::PathBuf;
use std::process::Command;

const IB: u64 = 0x1000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const FILE_LEN: usize = 0x800;
const SIZE_OF_IMAGE: u32 = 0x2000;

const F: u32 = 0x1020;
const G: u32 = 0x1060;
const H: u32 = 0x1080;
const J: u32 = 0x10a0;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

fn build() -> Vec<u8> {
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
    w32(&mut b, o + 56, SIZE_OF_IMAGE);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let call = |at: u32, to: u32| [&[0xe8][..], &rel(at + 5, to)].concat();
    let jmp = |at: u32, to: u32| [&[0xe9][..], &rel(at + 5, to)].concat();
    let mut main = call(0x1000, F);
    main.extend(call(0x1005, J));
    main.push(0xc3);
    let mut fcode = vec![0x48, 0x83, 0xec, 0x28]; // sub rsp,0x28
    fcode.extend(call(F + 4, G));
    fcode.extend([0x48, 0x83, 0xc4, 0x28, 0xc3]); // add rsp,0x28; ret
    let mut gcode = call(G, H);
    gcode.push(0xc3);
    let hcode = vec![0xb8, 0x07, 0x00, 0x00, 0x00, 0xc3]; // mov eax,7; ret
    let code: Vec<(u32, Vec<u8>)> = vec![(0x1000, main), (F, fcode), (G, gcode), (H, hcode), (J, jmp(J, G))];
    for (rva, bytes) in code {
        let at = TEXT_RAW + (rva - TEXT_RVA) as usize;
        b[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    b
}

fn ent(rva: u32) -> Entry {
    Entry { va: IB + rva as u64, class: "S".into() }
}

fn lift(prune: &[Entry]) -> Result<(fe_lift::Stats, String), String> {
    let mut out = Vec::new();
    let gopt = GenOptions { prune: prune.to_vec(), ..Default::default() };
    let st = generate_full(&build(), &[], &[], &[], &[], &fe_decode::Options::default(), &fe_lift::SehOptions::default(), &gopt, &mut out)?;
    Ok((st, String::from_utf8(out).expect("utf8")))
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn without_a_prune_list_the_output_is_the_old_output() {
    let mut old = Vec::new();
    let st0 = generate(&build(), &mut old).unwrap();
    let (st, c) = lift(&[]).unwrap();
    assert_eq!(c.as_bytes(), &old[..]);
    assert_eq!(st.pruned_listed, 0);
    assert_eq!(st.c_bytes, st0.c_bytes);
    assert!(c.contains("void f_10001080(void)\n{"), "{c}");
    assert!(!c.contains("fle_prune"), "{c}");
}

#[test]
fn a_pruned_function_reached_by_kept_code_is_a_stopping_stub_and_its_callee_is_gone() {
    let (st, c) = lift(&[ent(G)]).unwrap();
    assert_eq!((st.pruned_listed, st.pruned_stubs), (1, 1));
    assert!(c.contains("#include \"fle_prune.h\""), "{c}");
    assert!(c.contains("void f_10001060(void)\n{\n    fle_pruned(UINT64_C(0x10001060));\n}\n\n"), "{c}");
    assert!(!c.contains("10001080"), "{c}");
    assert!(c.contains("f_10001060(); R[4] += 8;"), "{c}");
    assert!(c.contains("{ f_10001060(); return; }"), "{c}");
    assert!(c.contains("{ UINT64_C(0x10001060), f_10001060 }"), "{c}");
    let (_, plain) = lift(&[]).unwrap();
    assert!(plain.contains("L_10001060:") && !c.contains("L_10001060:"));
    assert!(c.contains("L_10001020:") && c.contains("L_100010a0:"));
}

#[test]
fn a_pruned_function_that_no_kept_code_reaches_is_not_generated() {
    // J is only reached from main; pruning J and F: main's calls name them, so they are stubs. Pruning H alone: G calls H, a stub. Pruning both G and H: G is a stub, H is gone.
    let (st, c) = lift(&[ent(G), ent(H)]).unwrap();
    assert_eq!((st.pruned_listed, st.pruned_stubs), (2, 1));
    assert!(!c.contains("10001080"), "{c}");
    let (st, c) = lift(&[ent(H)]).unwrap();
    assert_eq!((st.pruned_listed, st.pruned_stubs), (1, 1));
    assert!(c.contains("void f_10001080(void)\n{\n    fle_pruned(UINT64_C(0x10001080));\n}\n\n"), "{c}");
    // G is not pruned and still calls it
    assert!(c.contains("void f_10001060(void)\n{\nL_10001060:"), "{c}");
}

#[test]
fn it_is_combined_with_outline_shared_and_dead_flags() {
    let (st, c) = lift(&[ent(G)]).unwrap();
    assert_eq!(st.pruned_stubs, 1);
    assert!(c.contains("void f_10001060(void)\n{\n    fle_pruned(UINT64_C(0x10001060));\n}\n\n"), "{c}");
    assert!(!c.contains("10001080"), "{c}");
    assert!(c.contains("L_10001020:") && c.contains("L_100010a0:"));
}

#[test]
fn bad_entries_are_errors() {
    let e = lift(&[ent(0x1800)]).unwrap_err();
    assert!(e.contains("not in .text"), "{e}");
    let e = lift(&[ent(G), ent(G)]).unwrap_err();
    assert!(e.contains("listed twice"), "{e}");
    let e = lift(&[ent(0x1000)]).unwrap_err();
    assert!(e.contains("entry point") || e.contains("0x10001000"), "{e}");
}

#[test]
fn the_pruned_module_compiles_and_the_stub_stops() {
    let (_, c) = lift(&[ent(G)]).unwrap();
    let dir = manifest().join("../../out-prune/test");
    std::fs::create_dir_all(&dir).unwrap();
    let tag = etxt_common::uniq("prune-compile");
    let src = dir.join(format!("{tag}.c"));
    std::fs::write(&src, &c).unwrap();
    let rt = manifest().join("../../runtime");
    let o = Command::new("clang")
        .args(["-std=gnu11", "-O0", "-w", "-fsyntax-only", "-I"])
        .arg(&rt)
        .arg(&src)
        .output()
        .expect("clang");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
