//! Generator side of decision 0103 (provisional): (1) `ret imm16` is lifted (the callee adds the imm16 bytes to RSP, the caller's lifted
//! call adds the 8 of the return address; `ret 0` is `ret`, decision 0102 W9), (2) the inline absolute jump table (decision 0102 W3,
//! `fe_decode::Origin::Inline`; decision 0103's own `Origin::AbsRun` was not taken in the merge) is a `switch` on the slot address,
//! (3) rows of the response table that decision 0103 adds or needs. Synthetic images: fe-decode/tests/msvc_common (a function with `ret 16`
//! and one with `ret 0`) and fe-decode/tests/inline_common (the table).

#[path = "../../fe-decode/tests/inline_common/mod.rs"]
mod inline_common;
#[path = "../../fe-decode/tests/msvc_common/mod.rs"]
mod msvc_common;

use fe_lift::generate_with_specs;

fn lift(img: &[u8]) -> (fe_lift::Stats, String) {
    let mut out: Vec<u8> = Vec::new();
    let st = generate_with_specs(img, &[], &[], &mut out).expect("generate");
    (st, String::from_utf8(out).unwrap())
}

#[test]
fn ret_with_an_immediate_pops_that_many_more_bytes_in_the_callee() {
    let mut img = msvc_common::Img::default();
    // f1: mov eax,1; ret 16      f2: mov eax,2; ret 0      f3: mov eax,3; ret 65535
    img.text.push((0x1000, vec![0xb8, 1, 0, 0, 0, 0xc2, 0x10, 0x00]));
    img.text.push((0x1010, vec![0xb8, 2, 0, 0, 0, 0xc2, 0x00, 0x00]));
    img.text.push((0x1020, vec![0xb8, 3, 0, 0, 0, 0xc2, 0xff, 0xff]));
    img.pdata = vec![(0x1000, 0x1008, msvc_common::UNW_PLAIN), (0x1010, 0x1018, msvc_common::UNW_PLAIN), (0x1020, 0x1028, msvc_common::UNW_PLAIN)];
    img.entry = 0x1000;
    let data = img.build();
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let seeds = fe_decode::pdata_seeds(&pe).unwrap();
    let mut out: Vec<u8> = Vec::new();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut out).expect("generate");
    let c = String::from_utf8(out).unwrap();
    assert_eq!(st.unsupported_total, 0, "{:?}", st.unsupported);
    assert_eq!(st.bad_nodes, 0);
    assert!(c.contains("R[4] += 16; return;"), "ret 16:\n{c}");
    assert!(c.contains("R[4] += 65535; return;"), "ret 65535");
    // `ret 0` is `ret`: no stack adjustment
    let f2 = c.split("void f_140001010(void)\n").nth(1).expect("f_140001010");
    let body2 = &f2[..f2.find("\n}\n").unwrap()];
    assert!(body2.contains("return;") && !body2.contains("R[4] +="), "ret 0:\n{body2}");
    // `ret 0` has the handler of `ret` (decision 0102 W9; it is `return;` as well), the other two are the imm16 forms
    assert_eq!(st.handled["ret_imm16"], 2, "ret 16 and ret 65535");
    assert_eq!(st.handled["ret"], 1, "ret 0");
}

#[test]
fn the_inline_absolute_table_is_a_switch_on_the_slot_address() {
    let img = inline_common::build(inline_common::Variant::default());
    let pe = fe_pe::Pe::parse(&img).unwrap();
    let a = fe_decode::analyze_with_seeds(&pe, &[]).unwrap();
    assert_eq!(a.closure.jumptabs, 1);
    let (st, c) = lift(&img);
    assert_eq!(st.handled["jmp_ind:inline_table"], 1);
    // the three slots, in address order, each a goto to the block of the target (the targets are blocks of the function)
    let sw_at = c.find("switch (t)").expect("a switch");
    let at = c[..sw_at].rfind("{ uint64_t t = ").expect("the selector");
    let sw = &c[at..sw_at + c[sw_at..].find("fle_jumptab_miss").unwrap() + 80];
    for slot in ["0x14000100c", "0x140001014", "0x14000101c"] {
        assert!(sw.contains(&format!("case UINT64_C({slot}):")), "case {slot} missing:\n{sw}");
    }
    assert!(!sw.contains("0x140001024"), "the fourth slot is not relocated, so it is not a case");
    assert!(sw.contains("fle_jumptab_miss(UINT64_C(0x140001007), t)"), "{sw}");
    // the selector is the effective address of the memory operand: r10 + rdx*8 + 0x10
    assert!(sw.contains("R[10]") && sw.contains("R[2]"), "{sw}");
}

#[test]
fn the_rows_of_decisions_0102_and_0103_are_in_the_response_table() {
    let table = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/imports_table.rs")).unwrap();
    let all_headers: String = std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "h"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    for (dll, n) in [
        ("kernel32.dll", "GetVersionExA"),
        ("kernel32.dll", "GetModuleFileNameA"),
        ("kernel32.dll", "HeapCreate"),
        ("kernel32.dll", "HeapSetInformation"),
        ("kernel32.dll", "GetStartupInfoA"),
        ("kernel32.dll", "SetHandleCount"),
        ("kernel32.dll", "DecodePointer"),
        ("shell32.dll", "SHGetFolderPathW"),
    ] {
        let row = format!("(\"{dll}\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row");
        assert!(all_headers.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in a runtime header");
    }
    // the names that the CRT of freetype_x64.dll did not reach in the run stay without a row. GetEnvironmentStrings and FreeEnvironmentStringsA
    // are not in this list any more: decision 0103 left them out, decision 0102 W7 has rows for them (freetype's CRT calls them, from 1 and 2 sites)
    for n in ["HeapDestroy", "TlsFree"] {
        assert!(!table.contains(&format!("\"{n}\", \"fle_imp_{n}\"")), "{n} has no row");
    }
}
