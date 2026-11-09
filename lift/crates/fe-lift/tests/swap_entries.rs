//! Swapped entries (decision 0160; src/swap.rs, `GenOptions::swap` of `generate_full`, `fe-lift --swap-entries`).
//!
//! Synthetic PE (ImageBase 0x10000000, .text at RVA 0x1000, read + execute), written here, not taken from the code under test:
//! - 0x1000 `main`: `call F; call J; ret`
//! - 0x1020 F: `sub rsp,0x28; call G; add rsp,0x28; ret`
//! - 0x1060 G (the function the host implements): `call H; ret`
//! - 0x1080 H (reachable only through G): `mov eax,7; ret`
//! - 0x10a0 J: `jmp G` (a tail call to G)
//!
//! Expected, from the decision: with G swapped, G is still a function entry (a call reaches it as `f_<G>()`, it is in the dispatch table, J's jump is
//! the tail call `f_<G>(); return;`), but its body is the stub of the host function, and H is not in the closure at all. Without a swap entry the
//! output is the old output byte for byte. A swap entry outside .text, a repeated one, are errors. The generated C of the swapped module compiles.
//!
//! Outputs go under lift/out-swap/test.

mod etxt_common;

use fe_lift::swap::Entry;
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

fn ent(rva: u32, name: &str) -> Entry {
    Entry { va: IB + rva as u64, name: name.into() }
}

fn lift(swap: &[Entry]) -> Result<(fe_lift::Stats, String), String> {
    let mut out = Vec::new();
    let gopt = GenOptions { swap: swap.to_vec(), ..Default::default() };
    let st = generate_full(&build(), &[], &[], &[], &[], &fe_decode::Options::default(), &fe_lift::SehOptions::default(), &gopt, &mut out)?;
    Ok((st, String::from_utf8(out).expect("utf8")))
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn without_an_entry_the_output_is_the_old_output() {
    let mut old = Vec::new();
    let st0 = generate(&build(), &mut old).unwrap();
    let (st, c) = lift(&[]).unwrap();
    assert_eq!(c.as_bytes(), &old[..]);
    assert_eq!(st.swap_entries, 0);
    assert_eq!(st.c_bytes, st0.c_bytes);
    // G calls H, so H is in the closure of the unswapped module
    assert!(c.contains("void f_10001080(void)\n{"), "{c}");
    assert!(!c.contains("fle_swap_"), "{c}");
}

#[test]
fn a_swapped_function_is_a_stub_and_its_callee_is_not_in_the_closure() {
    let (st, c) = lift(&[ent(G, "g_host")]).unwrap();
    assert_eq!(st.swap_entries, 1);
    // the stub is the whole body of G
    assert!(c.contains("void f_10001060(void)\n{\n    extern void fle_swap_g_host(void);\n    fle_swap_g_host();\n}\n\n"), "{c}");
    // H is reachable only through G: it is neither declared, nor in the dispatch table, nor emitted
    assert!(!c.contains("10001080"), "{c}");
    // G stays a function entry: F calls it, J's jump is a tail call, the dispatch table has it
    assert!(c.contains("f_10001060(); R[4] += 8;"), "{c}");
    assert!(c.contains("{ f_10001060(); return; }"), "{c}");
    assert!(c.contains("{ UINT64_C(0x10001060), f_10001060 }"), "{c}");
    // the callers are unchanged: the unswapped output with G's block and H's function taken out is this output
    let (_, plain) = lift(&[]).unwrap();
    assert!(plain.contains("L_10001060:") && !c.contains("L_10001060:"));
    assert!(c.contains("L_10001020:") && c.contains("L_100010a0:"));
}

#[test]
fn an_entry_nothing_calls_is_still_an_entry() {
    // H swapped while G still calls it: H's body is the stub; a function that nothing in the image calls (an exported or vtable function) is the same through the seeds
    let (_, c) = lift(&[ent(H, "h_host")]).unwrap();
    assert!(c.contains("void f_10001080(void)\n{\n    extern void fle_swap_h_host(void);\n    fle_swap_h_host();\n}\n\n"), "{c}");
    assert!(c.contains("void f_10001060(void)\n{\nL_10001060:"), "{c}");
}

#[test]
fn a_module_with_swap_stubs_is_lifted_without_the_outlining_and_the_dead_flags() {
    let (st, c) = lift(&[ent(G, "g_host")]).unwrap();
    assert_eq!(st.outlined_heads, 0);
    assert_eq!(st.flag_dead_full, 0);
    assert!(!c.contains("fle_nf_"), "{c}");
}

#[test]
fn bad_entries_are_errors() {
    let e = lift(&[ent(0x1800, "x")]).unwrap_err();
    assert!(e.contains("not in .text"), "{e}");
    let e = lift(&[ent(G, "a"), ent(G, "b")]).unwrap_err();
    assert!(e.contains("listed twice"), "{e}");
}

#[test]
fn the_swapped_module_compiles_and_the_stub_needs_the_host_function() {
    let (_, c) = lift(&[ent(G, "g_host")]).unwrap();
    let dir = manifest().join("../../out-swap/test");
    std::fs::create_dir_all(&dir).unwrap();
    let tag = etxt_common::uniq("swap-compile");
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
