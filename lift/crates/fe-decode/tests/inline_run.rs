//! The inline absolute jump table `lea B,[rip+k]; jmp [B+I*8+disp]` found in the memset tail of quickfontcache_x64.dll (0x40c163) and of
//! FLEngine_x64.dll (0x40d253, which the hand annotation of decision 0036 keeps), on the synthetic PE32+ image of tests/inline_common/mod.rs
//! (the builder of decision 0103). The mechanism under test is `recognize_inline`/`resolve_inline`/`verify_inline` of decision 0102 W3
//! (`Origin::Inline`); decision 0103 first wrote its own recogniser (`recognize_abs`, `Origin::AbsRun`), which the merge did not take, and these
//! are the facts its tests checked, stated for the mechanism that stayed: the table is the run of DIR64-relocated pointers from the `lea` target
//! that lie in the `.pdata` function of the jump, a pointer into another function or an unrelocated slot ends the run, and a `lea` that loads
//! something that is not such a table gives no table and no report (the difference from 0103's first version, which reported an empty run).
//! tests/inline_table.rs has the same family of cases with its own image; the two images are different on purpose (here the table follows the
//! jump at once, which is also the only form 0103 accepted).

#[path = "inline_common/mod.rs"]
mod inline_common;

use fe_decode::{analyze_with_seeds, Analysis, Origin, Select};
use fe_pe::Pe;
use inline_common::{build, Variant, IB};

fn run(v: Variant) -> Analysis {
    let data = build(v);
    let pe = Pe::parse(&data).expect("parse");
    analyze_with_seeds(&pe, &[]).expect("analyze")
}

const JMP: u64 = IB + 0x1007;

fn slot(k: u64) -> u64 {
    IB + 0x100c + 8 * k
}

#[test]
fn the_table_is_the_relocated_run_that_starts_at_the_lea_target_here_the_end_of_the_jump() {
    let a = run(Variant::default());
    assert_eq!(a.closure.decode_failures, 0);
    assert_eq!(a.closure.jumptabs, 1);
    assert_eq!(a.closure.inline_tables, 1);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    let t = a.jumptabs.get(&JMP).expect("table at the jmp");
    assert_eq!(t.select, Select::Slot);
    assert_eq!(t.origin, Origin::Inline { table: IB + 0x100c });
    // slot 3 (0x1024) holds T0 but is not relocated: the run ends before it
    assert_eq!(t.cases, vec![(slot(0), IB + 0x1040), (slot(1), IB + 0x1050), (slot(2), IB + 0x1060)]);
    assert_eq!(a.closure.indirect.by_kind["jmp:inline_table"], 1);
    // the targets entered the closure as blocks of the function
    for rva in [0x1040u64, 0x1050, 0x1060] {
        assert!(a.insns.contains_key(&(IB + rva)), "target {rva:#x} not decoded");
    }
}

#[test]
fn a_fourth_relocated_pointer_into_the_function_extends_the_run() {
    let a = run(Variant { fourth_relocated: true, ..Variant::default() });
    let t = a.jumptabs.get(&JMP).expect("table");
    assert_eq!(t.cases.len(), 4);
    assert_eq!(t.cases[3], (slot(3), IB + 0x1040));
}

#[test]
fn a_pointer_into_another_function_ends_the_run() {
    // slot 2 points at H (0x1100), which is in .text but in another RUNTIME_FUNCTION
    let a = run(Variant { third_outside: true, ..Variant::default() });
    let t = a.jumptabs.get(&JMP).expect("table");
    assert_eq!(t.cases, vec![(slot(0), IB + 0x1040), (slot(1), IB + 0x1050)]);
}

#[test]
fn an_unrelocated_first_slot_gives_no_table_and_reports_nothing() {
    // decision 0102 W3: `lea B,[rip+k]; jmp [B+I*8+d]` is a common shape (a table of pointers to other functions in .rdata is the same), so a jump
    // that is not this table is left unresolved without a report. Decision 0103's own first version reported "no relocated code pointer of the
    // function"; reporting broke the test that dsp_ippv2_x64.dll has no rejected jump table, so the mechanism that stayed does not.
    let a = run(Variant { first_unrelocated: true, ..Variant::default() });
    assert_eq!(a.closure.jumptabs, 0);
    assert_eq!(a.closure.inline_tables, 0);
    assert!(a.jumptabs.is_empty());
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
}

#[test]
fn a_lea_that_loads_an_address_inside_the_function_without_a_relocated_run_is_not_the_table() {
    // the lea loads 0x102c (inside F): the recogniser is satisfied, the resolver finds no relocated slot there, and nothing is claimed
    let a = run(Variant { far_lea: true, ..Variant::default() });
    assert_eq!(a.closure.jumptabs, 0);
    assert!(a.closure.jumptab_rejected.is_empty());
    assert_eq!(a.closure.indirect.by_kind.get("jmp:inline_table"), None);
}

#[test]
fn a_jump_without_the_lea_in_front_is_not_the_form() {
    let a = run(Variant { no_lea: true, ..Variant::default() });
    assert_eq!(a.closure.jumptabs, 0);
    assert!(a.closure.jumptab_rejected.is_empty());
}
