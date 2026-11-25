//! The guard of an MSVC switch that compares the index in its stack slot (decision 0430): `cmp dword [rsp+S],N; ja def;
//! movsxd rax,dword [rsp+R]; lea r10,[ImageBase]; mov ecx,[r10+rax*4+T]; add rcx,r10; jmp rcx`. The index is reloaded from R.
//! N is the bound of the guard only when R is the slot the guard compares (S = R). Synthetic PE32+ image (msvc_common).

mod msvc_common;

use fe_decode::{analyze_with_options, jumptab, Options, Origin};
use fe_pe::Pe;
use msvc_common::*;

const AT: u32 = 0x1100;

/// The image: the function at AT (the guard compares `[rsp+cmp_slot]`, the reload reads `[rsp+reload_slot]`), `n + 1` table entries
/// that all point at the case block `t0`, and MAIN calling AT. Returns the image bytes and the VA of the `jmp rcx`.
fn image(cmp_slot: u32, reload_slot: u32, n: u8) -> (Vec<u8>, u64) {
    let mut a = Asm::new(AT);
    a.raw(&[0x83, 0xbc, 0x24]).raw(&cmp_slot.to_le_bytes()).raw(&[n]);
    a.jcc8(0x77, "def");
    a.raw(&[0x48, 0x63, 0x84, 0x24]).raw(&reload_slot.to_le_bytes());
    a.lea_img(10, 0);
    a.mov_ecx_table(10, TABLE);
    a.add_rcx_b(10);
    a.label("jmp").jmp_rcx();
    a.label("t0").mov_eax(1).ret();
    a.label("def").mov_eax(0xdead).ret();
    let (code, labels) = a.finish();
    let end = AT + code.len() as u32;
    let jmp = IB + labels["jmp"] as u64;

    let mut m = Asm::new(MAIN);
    m.call_rva(AT).ret();
    let mut img = Img { entry: MAIN, ..Default::default() };
    img.text.push((AT, code));
    img.text.push((MAIN, m.code));
    img.rdata.push((TABLE, u32s(&vec![labels["t0"]; n as usize + 1])));
    img.pdata.push((AT, end, UNW_PLAIN));
    img.pdata.push((MAIN, MAIN + 6, UNW_PLAIN));
    img.pdata.sort();
    (img.build(), jmp)
}

fn analyse(bytes: &[u8]) -> fe_decode::Analysis {
    let pe = Pe::parse(bytes).unwrap();
    analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap()
}

#[test]
fn the_reloaded_index_from_the_guarded_slot_is_bounded_by_the_guard() {
    let (bytes, jmp) = image(0x88, 0x88, 5);
    let a = analyse(&bytes);
    let t = a.jumptabs.get(&jmp).unwrap_or_else(|| panic!("not a table: {:?}", a.closure.msvc_rejected));
    assert!(matches!(t.origin, Origin::Msvc { n: 5, rel: false, .. }), "{:?}", t.origin);
    assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
}

#[test]
fn the_index_reloaded_from_another_slot_is_not_bounded_by_the_guard() {
    let (bytes, jmp) = image(0x88, 0x90, 5);
    let a = analyse(&bytes);
    assert!(!a.jumptabs.contains_key(&jmp));
    let msg = a.closure.msvc_rejected.iter().find(|(v, _)| *v == jmp).map(|(_, m)| m.clone()).unwrap_or_default();
    assert!(msg.starts_with("N not derived"), "{msg}");
}
