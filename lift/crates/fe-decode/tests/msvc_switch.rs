//! MSVC switch tables with a proven base (decisions 0089 and 0090), fe-decode level. Synthetic PE32+ images built by
//! `msvc_common` (a function with `[push B; lea B,[ImageBase];] mov eax,ecx; cmp eax,N; ja def; [movzx eax,byte
//! [rax+B+T1];] mov ecx,[B+rax*4+T2]; add rcx,B; jmp rcx` and three target blocks).
//!
//! The default base proof is decision 0090's dominance: the one write to B is `lea B,[ImageBase]` and it runs before the
//! jump on every path from the function entry, and the function has no other indirect jump. The entry-block condition
//! (decision 0089's simple condition, comparison only) is used by the tests that say so.
//!
//! Resolved: (c1) one stage, (c2) two stages, `cmp rax`, a `cdqe` after the guard, B callee-saved with a call in the
//! function, a target in a chained RUNTIME_FUNCTION of the same function, a `lea` outside the entry block that is on
//! every path (a `je` to the next instruction, a diamond), a tail call by a direct `jmp rel32`.
//! Not resolved (fail closed; the jump stays `Unresolved` and the reason is in `msvc_rejected`): the `lea` is on one path
//! only, B is written twice (also one `lea` on each path), B comes from an argument, the `lea` is not the image base, B is
//! caller-saved and the function calls, `pop B` without `push B`, another indirect jump in the function (`jmp reg`,
//! `jmp [mem]`, a second resolved-looking switch), no guard, the guard is on another register, a branch lands inside the
//! idiom, another function jumps into this one, a target in another function / not executable / not an instruction start,
//! no exception directory.
//!
//! The real Blend2D (`$BLEND2D_PE`, else lift/out-seed58/s58/dll/Blend2D.4700000.dll; skipped when absent) and the real
//! FLEngine_x64.dll are in the last tests.

mod msvc_common;

use fe_decode::{analyze_with_options, jumptab, Options, Resolved, Select};
use fe_pe::Pe;
use iced_x86::{Mnemonic, OpKind};
use msvc_common::*;
use std::collections::BTreeSet;

const AT: u32 = 0x1100;

fn run(o: &Sw) -> (Built, fe_decode::Analysis) {
    let built = build_switch(AT, o);
    let img = built.img.build();
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
    (built, a)
}

fn jmp_va(b: &Built) -> u64 {
    IB + b.labels["jmp"] as u64
}

fn targets(b: &Built) -> BTreeSet<u64> {
    let e = b.entries;
    let tab: Vec<u32> = if b.byte_table.is_empty() { b.table[..e].to_vec() } else { b.table[..e].to_vec() };
    tab.into_iter().map(|r| IB + r as u64).collect()
}

fn resolved_ok(o: &Sw) {
    let (b, a) = run(o);
    assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (1, 1), "{o:?}");
    let t = &a.jumptabs[&jmp_va(&b)];
    assert_eq!(t.select, Select::Target);
    let cases: BTreeSet<u64> = t.cases.iter().map(|&(c, tg)| {
        assert_eq!(c, tg, "a case is the target VA the register holds");
        c
    }).collect();
    assert_eq!(cases, targets(&b), "{o:?}");
    // every target is a decoded instruction start of the closure
    for tg in &cases {
        assert!(a.insns.contains_key(tg), "target {tg:#x} not in the closure");
    }
    // the jump itself stays Unresolved in `resolved` (like the annotated tables)
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert_eq!(a.closure.indirect.by_kind["jmp:msvc_table"], 1);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 0);
}

fn rejected(o: &Sw, why: &str) {
    rejected_unresolved(o, why, 1);
}

/// `unresolved`: the `jmp:unresolved` count when the site is rejected: 1 for the site itself, more when the function has other
/// indirect jumps that the closure reaches (decision 0094 tests).
fn rejected_unresolved(o: &Sw, why: &str, unresolved: usize) {
    let (b, a) = run(o);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?} must not be resolved");
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)), "{o:?}");
    assert_eq!(a.closure.msvc_rejected.len(), 1, "{o:?}: {:?}", a.closure.msvc_rejected);
    let (va, msg) = &a.closure.msvc_rejected[0];
    assert_eq!(*va, jmp_va(&b), "{o:?}");
    assert!(msg.contains(why), "{o:?}: wanted {why:?}, got {msg:?}");
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], unresolved, "{o:?}");
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
}

// ---- resolved ----

#[test]
fn c1_one_stage_with_base_from_the_entry_block() {
    resolved_ok(&Sw::c1());
    let (b, a) = run(&Sw::c1());
    // N = 3: 4 entries, 3 distinct targets (t0 appears twice)
    assert_eq!(b.entries, 4);
    assert_eq!(a.jumptabs[&jmp_va(&b)].cases.len(), 3);
}

#[test]
fn c2_two_stages_entry_count_is_the_largest_index_byte_plus_one() {
    resolved_ok(&Sw::c2());
    let (b, a) = run(&Sw::c2());
    // N = 5: 6 index bytes [0,2,1,2,0,1]; the RVA table has max+1 = 3 entries read; a fourth is in the file, never read
    assert_eq!((b.byte_table.len(), b.entries), (6, 3));
    assert_eq!(a.jumptabs[&jmp_va(&b)].cases.len(), 3);
    let o = a.jumptabs[&jmp_va(&b)].origin;
    assert!(matches!(o, fe_decode::Origin::Msvc { n: 5, t1: Some(BYTES), t2: TABLE, .. }), "{o:?}");
}

#[test]
fn guard_on_the_64_bit_index_and_a_cdqe_after_it() {
    resolved_ok(&Sw { guard: Guard::Rax, ..Sw::c1() });
    resolved_ok(&Sw { cdqe: true, ..Sw::c1() });
    resolved_ok(&Sw { cdqe: true, guard: Guard::Rax, ..Sw::c2() });
}

#[test]
fn sub_before_the_compare_gives_n_from_the_compared_register() {
    // `sub eax,K; cmp eax,N; ja def; ... [rax*4]`: the register that is compared is the one the table is indexed with
    resolved_ok(&Sw { sub_k: 5, ..Sw::c1() });
    resolved_ok(&Sw { sub_k: 5, ..Sw::c2() });
    resolved_ok(&Sw { sub_k: 0x20, guard: Guard::Rax, cdqe: true, ..Sw::c2() });
}

#[test]
fn push_pop_of_the_base_is_not_a_write_and_a_callee_saved_base_survives_a_call() {
    resolved_ok(&Sw { push_pop: true, ..Sw::c1() });
    resolved_ok(&Sw { push_pop: false, ..Sw::c1() });
    resolved_ok(&Sw { call: true, ..Sw::c1() }); // r12 is callee-saved
    resolved_ok(&Sw { call: true, ..Sw::c2() }); // r13
}

#[test]
fn volatile_base_without_a_call_is_proven() {
    resolved_ok(&Sw { base: 9, ..Sw::c1() }); // r9
    resolved_ok(&Sw { base: 10, ..Sw::c2() }); // r10
}

#[test]
fn a_target_in_a_chained_piece_of_the_same_function_is_inside() {
    // entry 1 points into the second RUNTIME_FUNCTION, chained to this function (UNW_FLAG_CHAININFO)
    resolved_ok(&Sw { bad: Bad::ChainedPiece, ..Sw::c1() });
}

// ---- not resolved ----

// decision 0090: the base proof is dominance. A `lea` outside the entry block that is on every path to the jump is proven;
// the entry block's simple condition (decision 0089) was a comparison-only proof and is deleted (decision 0243).

#[test]
fn a_tail_call_by_a_direct_jmp_is_not_an_indirect_jump() {
    resolved_ok(&Sw { other_jmp: OtherJmp::TailCall, ..Sw::c1() });
    resolved_ok(&Sw { other_jmp: OtherJmp::TailCall, lea: Lea::AfterDiamond, ..Sw::c2() });
}

#[test]
fn base_from_an_argument_is_not_proven() {
    rejected(&Sw { lea: Lea::FromArg, ..Sw::c1() }, "is not `lea");
    rejected(&Sw { lea: Lea::FromArg, ..Sw::c2() }, "is not `lea");
}

#[test]
fn base_lea_of_another_address_is_not_the_image_base() {
    rejected(&Sw { lea: Lea::Wrong(0x10), ..Sw::c1() }, "is not `lea");
    rejected(&Sw { lea: Lea::Wrong(-0x1000), ..Sw::c2() }, "is not `lea");
}

#[test]
fn caller_saved_base_in_a_function_with_a_call_is_not_proven() {
    rejected(&Sw { base: 8, call: true, ..Sw::c1() }, "caller-saved");
    rejected(&Sw { base: 11, call: true, ..Sw::c2() }, "caller-saved");
}

#[test]
fn index_bound_not_derived() {
    rejected(&Sw { guard: Guard::Missing, ..Sw::c1() }, "N not derived");
    rejected(&Sw { guard: Guard::Missing, ..Sw::c2() }, "N not derived");
    // the compare is on another register than the index
    rejected(&Sw { guard: Guard::OtherReg, ..Sw::c1() }, "N not derived");
}

#[test]
fn a_branch_landing_inside_the_idiom_skips_the_guard() {
    rejected(&Sw { landing: true, ..Sw::c1() }, "inside the switch idiom");
    rejected(&Sw { landing: true, ..Sw::c2() }, "inside the switch idiom");
}

#[test]
fn another_function_jumping_into_the_middle_means_the_entry_block_may_not_run() {
    rejected(&Sw { foreign: true, ..Sw::c1() }, "outside the function");
}

#[test]
fn target_in_another_function_makes_the_whole_site_unresolved() {
    rejected(&Sw { bad: Bad::OtherFunction, ..Sw::c1() }, "target outside the function: 1 of 3 distinct target(s)");
    rejected(&Sw { bad: Bad::OtherFunction, ..Sw::c2() }, "target outside the function: 1 of 3 distinct target(s)");
    // the message names the other function
    let (_, a) = run(&Sw { bad: Bad::OtherFunction, ..Sw::c1() });
    let msg = &a.closure.msvc_rejected[0].1;
    assert!(msg.contains(&format!("{:#x} (in the function at {:#x})", IB + OTHER_FN as u64, IB + OTHER_FN as u64)), "{msg}");
}

#[test]
fn target_not_executable_is_outside() {
    rejected(&Sw { bad: Bad::NotExecutable, ..Sw::c1() }, "in no .pdata function, not executable");
}

#[test]
fn target_not_at_an_instruction_start_is_not_resolved() {
    rejected(&Sw { bad: Bad::Misaligned, ..Sw::c1() }, "not an instruction start");
}

#[test]
fn without_an_exception_directory_nothing_is_resolved() {
    rejected(&Sw { no_pdata: true, ..Sw::c1() }, "not inside a .pdata function");
}

#[test]
fn the_dispatch_form_is_unchanged_for_rejected_sites() {
    // the site stays an unresolved `jmp reg`: counted as a register jump, no jump table (the `lea` is on one path only)
    let (b, a) = run(&Sw { lea: Lea::OneSided, ..Sw::c1() });
    assert_eq!(a.closure.register_jumps, 1);
    assert!(a.insns[&jmp_va(&b)].mnemonic() == Mnemonic::Jmp && a.insns[&jmp_va(&b)].op_kind(0) == OpKind::Register);
    // the targets are not in the closure (no switch edges): the blocks t1 and t2 are never reached
    assert!(!a.insns.contains_key(&(IB + b.labels["t1"] as u64)));
}

// ---- decision 0094: the base proof by reaching definitions ----
//
// B's definitions that reach the jump must all be `lea B,[rip+d]` with rip+d = ImageBase, and one at least must reach it. A
// write on a branch that never comes back to the jump is harmless; every other indirect jump is a wildcard that lands at any
// instruction and carries the definitions that reach it. A call is a definition of a caller-saved B, so only a call on a path
// from the `lea` to the jump counts.

#[test]
fn a_write_that_comes_back_to_the_jump_through_the_jumps_own_table_is_rejected() {
    // `t2: mov B,rcx; jmp guard`: the path jump -> t2 -> write -> guard -> jump carries the `mov` to the jump. The jump's own
    // table targets are edges (decision 0094 item 2, as in 0090)
    rejected(&Sw { side_write: SideWrite::InCaseLoop, ..Sw::c1() }, "reaches the jump");
    rejected(&Sw { side_write: SideWrite::InCaseLoop, ..Sw::c2() }, "reaches the jump");
    // the message names the instruction
    let (_, a) = run(&Sw { side_write: SideWrite::InCaseLoop, ..Sw::c1() });
    let msg = &a.closure.msvc_rejected[0].1;
    assert!(msg.contains("the definition of R12 at") && msg.contains("`Mov`") && msg.contains("decision 0094"), "{msg}");
}

/// The site `jmp` is in `jumptabs`, and `msvc_rejected` holds nothing but (at most) the other indirect jump of the function.
fn resolved_ok_or_only_the_other_rejected(o: &Sw) {
    let (b, a) = run(o);
    assert!(a.jumptabs.contains_key(&jmp_va(&b)), "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(a.closure.msvc_rejected.iter().all(|(v, _)| *v != jmp_va(&b)), "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
}

#[test]
fn a_second_switch_is_a_wildcard_jump_and_both_are_resolved_by_reaching_definitions() {
    // decision 0094 item 2: another indirect jump is a wildcard even when it is a resolved switch (P4, not P3). Both switches
    // of the function are resolved: the only definition that reaches either is the `lea`
    let o = Sw { other_jmp: OtherJmp::SecondSwitch, ..Sw::c1() };
    let (b, a) = run(&o);
    let jmp2 = IB + b.labels["jmp2"] as u64;
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (2, 2), "{:?}", a.closure.msvc_rejected);
    assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
    assert!(a.jumptabs.contains_key(&jmp_va(&b)) && a.jumptabs.contains_key(&jmp2));
    // with a non-`lea` write after the first switch's table that reaches the second one only: the second is rejected
    // and so is the first (the write reaches the second jump, which is a wildcard for the first)
    let o = Sw { other_jmp: OtherJmp::SecondSwitch, side_write: SideWrite::InCaseLoop, ..Sw::c1() };
    let (_, a) = run(&o);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{:?}", a.closure.msvc_rejected);
    assert_eq!(a.closure.msvc_rejected.len(), 2, "{:?}", a.closure.msvc_rejected);
}

#[test]
fn a_definition_that_is_not_lea_and_reaches_another_indirect_jump_rejects_the_proof() {
    // (c) `movzx Bd,byte [rdx+0xe]; jmp rdx` in the `ja` target block: the write reaches the other indirect jump, which lands
    // anywhere, so it reaches the switch's jump too (the 7 sites of the investigation §2.2). `jmp [mem]` the same
    // (the other indirect jump is reached by the closure and stays unresolved too: `jmp:unresolved` is 2)
    let reject2 = |o: Sw| rejected_unresolved(&o, "reaches the jump", 2);
    for other in [OtherJmp::ReachedAfterWrite, OtherJmp::ReachedAfterWriteMem] {
        reject2(Sw { other_jmp: other, ..Sw::c1() });
        reject2(Sw { other_jmp: other, ..Sw::c2() });
        reject2(Sw { other_jmp: other, lea: Lea::AfterDiamond, ..Sw::c1() });
    }
    let (_, a) = run(&Sw { other_jmp: OtherJmp::ReachedAfterWrite, ..Sw::c1() });
    let msg = &a.closure.msvc_rejected[0].1;
    assert!(msg.contains("the definition of R12 at") && msg.contains("`Movzx`") && msg.contains("through another indirect jump"), "{msg}");
    // the same write, but the `def` block ends in `ret` instead of an indirect jump: it reaches nothing (case (a))
    resolved_ok(&Sw { side_write: SideWrite::InDef, ..Sw::c1() });
}

#[test]
fn a_definition_that_does_not_come_from_a_lea_of_the_image_base_among_the_reaching_ones_rejects_it() {
    // (d) a branch that makes B a `lea` on one side and another value (`mov B,rcx`) on the other, joined before the jump
    rejected(&Sw { lea: Lea::LeaOrArg, ..Sw::c1() }, "reaches the jump");
    rejected(&Sw { lea: Lea::LeaOrArg, ..Sw::c2() }, "reaches the jump");
    let (_, a) = run(&Sw { lea: Lea::LeaOrArg, ..Sw::c1() });
    assert!(a.closure.msvc_rejected[0].1.contains("`Mov`"), "{:?}", a.closure.msvc_rejected);
    // a `lea` of another address on the other side is not the image base either
    rejected(&Sw { lea: Lea::Wrong(0x10), ..Sw::c1() }, "reaches the jump");
}

#[test]
fn b_that_is_undefined_on_some_path_to_the_jump_is_rejected() {
    // (e) no definition at all; the `lea` on one path only; the entry's B reaches an indirect jump that lands anywhere
    rejected(&Sw { lea: Lea::Never, ..Sw::c1() }, "no definition of R12 reaches the jump");
    rejected(&Sw { lea: Lea::Never, push_pop: false, ..Sw::c2() }, "no definition of R13 reaches the jump");
    rejected(&Sw { lea: Lea::OneSided, ..Sw::c1() }, "is not defined on every path");
    rejected(&Sw { lea: Lea::OneSided, ..Sw::c2() }, "is not defined on every path");
    // `test ecx,ecx; jne ind` before the `lea`: the entry's (undefined) B reaches the other indirect jump and so the switch
    // (the other indirect jump is reached by the closure and stays unresolved: `jmp:unresolved` is 2)
    for o in [Sw { other_jmp: OtherJmp::ReachedBeforeLea, ..Sw::c1() }, Sw { other_jmp: OtherJmp::ReachedBeforeLea, ..Sw::c2() }] {
        rejected_unresolved(&o, "is not defined on every path", 2);
    }
    // `lea` on both sides: defined on every path
    resolved_ok(&Sw { lea: Lea::BothSides, ..Sw::c1() });
}

// ---- decision 0093: padding `nop` between the guard's `ja` and the table idiom ----

fn nops(list: &[&[u8]]) -> Vec<Vec<u8>> {
    list.iter().map(|b| b.to_vec()).collect()
}

fn decode_one(bytes: &[u8]) -> iced_x86::Instruction {
    let mut d = iced_x86::Decoder::with_ip(64, bytes, 0x1000, iced_x86::DecoderOptions::NONE);
    let i = d.decode();
    assert!(!i.is_invalid(), "{bytes:02x?}");
    assert_eq!(i.len(), bytes.len(), "{bytes:02x?} is one instruction");
    i
}

#[test]
fn which_instructions_are_padding_nops() {
    // the nop family (`Mnemonic::Nop`) that changes no register, flag or memory: 1 to 15 bytes
    for (b, what) in [
        (NOP_1, "90"),
        (NOP_2_XCHG_AX_AX, "66 90 = xchg ax,ax"),
        (NOP_3, "0f 1f 00"),
        (NOP_4, "0f 1f 40 00"),
        (NOP_5, "0f 1f 44 00 00"),
        (NOP_6, "66 0f 1f 44 00 00"),
        (NOP_7, "0f 1f 80 00000000"),
        (NOP_8, "0f 1f 84 00 00000000"),
        (NOP_9, "66 0f 1f 84 00 00000000"),
        (NOP_12, "data16 x3, nop word [rax+rax] (Blend2D f_48a2060)"),
        (NOP_15, "six data16, cs, nop word [rax+rax]"),
        (&[0x48, 0x90][..], "48 90 (rex.w nop)"),
        (&[0x0f, 0x1f, 0xc0][..], "0f 1f c0 (nop eax: a register operand that is only named)"),
    ] {
        let i = decode_one(b);
        assert_eq!(i.mnemonic(), Mnemonic::Nop, "{what}");
        assert!(jumptab::is_padding_nop(&i), "{what} must be padding");
    }
    // not padding: other mnemonics (they write a register or flag, or mean something else)
    for (b, what) in [
        (MOV_EDX_1, "mov edx,1"),
        (MOV_EDX_EDX, "mov edx,edx"),
        (XCHG_R8_RAX, "49 90 = xchg r8,rax"),
        (&[0x41, 0x90][..], "41 90 = xchg r8d,eax"),
        (XCHG_EAX_EAX, "87 c0 = xchg eax,eax (zero-extends rax)"),
        (PAUSE, "f3 90 = pause"),
        (INT3, "int3"),
        (UD2, "ud2"),
        (LEA_R11_RIP, "lea r11,[rip+0]"),
        (RESERVED_NOP, "0f 19 00 (reserved nop opcode)"),
        (&[0xf4][..], "hlt"),
        (&[0x0f, 0x1e, 0xfa][..], "endbr64 (f3 0f 1e fa is endbr64; without f3 it is a reserved nop)"),
        (&[0xf3, 0x0f, 0x1e, 0xfa][..], "endbr64"),
    ] {
        let i = decode_one(b);
        assert!(!jumptab::is_padding_nop(&i), "{what} must not be padding (mnemonic {:?})", i.mnemonic());
    }
}

#[test]
fn a_padding_nop_between_the_ja_and_the_table_idiom_is_resolved() {
    // every length of padding nop, (c1) and (c2); the table, the cases and the stats are the ones of the unpadded function
    for nop in ALL_NOPS {
        resolved_ok(&Sw { pad: nops(&[nop]), ..Sw::c1() });
        resolved_ok(&Sw { pad: nops(&[nop]), ..Sw::c2() });
    }
    let (b, a) = run(&Sw { pad: nops(&[NOP_12]), ..Sw::c1() });
    assert_eq!(b.entries, 4);
    assert_eq!(a.jumptabs[&jmp_va(&b)].cases.len(), 3);
    // `from` (the instruction after the `ja`) is the padding itself: the `ja` is 2 bytes (rel8) and the nop follows it
    let fe_decode::Origin::Msvc { from, guard_ja, .. } = a.jumptabs[&jmp_va(&b)].origin else { panic!("not an MSVC table") };
    assert_eq!(from, guard_ja + 2);
    assert_eq!(a.insns[&from].mnemonic(), Mnemonic::Nop);
    assert_eq!(a.insns[&from].len(), 12);
}

#[test]
fn padding_nop_with_cdqe_and_the_other_forms_of_the_idiom() {
    // the padding after the `cdqe` (right in front of the table instructions) and before it
    for before in [false, true] {
        resolved_ok(&Sw { cdqe: true, guard: Guard::Rax, pad: nops(&[NOP_9]), pad_before_cdqe: before, ..Sw::c1() });
        resolved_ok(&Sw { cdqe: true, guard: Guard::Rax, pad: nops(&[NOP_8]), pad_before_cdqe: before, ..Sw::c2() });
    }
    // with a `sub` in front of the compare, a call in the function, the `lea` after a diamond (the dominance proof)
    resolved_ok(&Sw { sub_k: 5, pad: nops(&[NOP_4]), ..Sw::c1() });
    resolved_ok(&Sw { call: true, pad: nops(&[NOP_12]), ..Sw::c2() });
    resolved_ok(&Sw { lea: Lea::AfterDiamond, pad: nops(&[NOP_7]), ..Sw::c2() });
    // a volatile base (r10 as in Blend2D f_48a2060) without a call
    resolved_ok(&Sw { base: 10, push_pop: false, pad: nops(&[NOP_12]), ..Sw::c1() });
}

#[test]
fn scheduled_the_instructions_of_decision_0093_that_write_the_index_are_still_rejected_by_default() {
    // the default (reaching definitions) reads the site again with the guard from the CFG (decision 0109): an instruction in the gap
    // that does not write the checked index (rax) does not stop the derivation, one that writes it does (`xchg r8,rax`,
    // `xchg eax,eax` zero-extends rax), and the reason of decision 0093 is still the first part of the message
    let why = "no `ja` before the table idiom (only `cdqe`, `lea B,[rip+d]` and `nop` may sit between)";
    // (`xchg eax,eax`, 87 c0, is left out: whether iced reports its operand as written is not known here)
    for bytes in [XCHG_R8_RAX] {
        rejected(&Sw { pad: nops(&[bytes]), ..Sw::c1() }, why);
        rejected(&Sw { pad: nops(&[bytes]), ..Sw::c2() }, why);
        rejected(&Sw { pad: nops(&[NOP_5, bytes]), ..Sw::c1() }, "scheduled form, decision 0109");
        rejected(&Sw { pad: nops(&[bytes, NOP_5]), ..Sw::c1() }, "is not a sign extension of the checked index");
    }
    for bytes in [MOV_EDX_1, MOV_EDX_EDX, PAUSE, LEA_R11_RIP, RESERVED_NOP] {
        resolved_ok(&Sw { pad: nops(&[bytes]), ..Sw::c1() });
        resolved_ok(&Sw { pad: nops(&[bytes]), ..Sw::c2() });
        resolved_ok(&Sw { pad: nops(&[NOP_5, bytes]), ..Sw::c1() });
        resolved_ok(&Sw { pad: nops(&[bytes, NOP_5]), ..Sw::c1() });
    }
    // no limit of two instructions any more (MSVC_SKIP belongs to the standard walk back); the walk of the scheduled form is
    // limited by MSVC_SCHED_WALK
    resolved_ok(&Sw { pad: nops(&[NOP_5, NOP_3, NOP_1]), ..Sw::c1() });
    resolved_ok(&Sw { cdqe: true, guard: Guard::Rax, pad: nops(&[NOP_5, NOP_3]), pad_before_cdqe: true, ..Sw::c2() });
    let long: Vec<&[u8]> = vec![NOP_1; jumptab::MSVC_SCHED_WALK + 1];
    rejected(&Sw { pad: nops(&long), ..Sw::c1() }, "does not end in 16 instructions");
}

#[test]
fn int3_and_ud2_in_the_padding_end_the_block_so_the_idiom_is_not_reached() {
    // `int3` and `ud2` stop the closure's block: the instructions after them (the table idiom) are never decoded through the
    // `ja`'s fall-through, so there is no `jmp reg` to recognise and nothing is resolved (they are not padding either)
    for bytes in [INT3, UD2] {
        for o in [Sw { pad: nops(&[bytes]), ..Sw::c1() }, Sw { pad: nops(&[NOP_5, bytes]), ..Sw::c2() }] {
            let (b, a) = run(&o);
            assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?}");
            assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
            assert!(!a.insns.contains_key(&jmp_va(&b)), "the jump was decoded: {o:?}");
        }
    }
}

#[test]
fn a_branch_into_the_padding_is_rejected() {
    // a branch that lands on the padding skips the guard `cmp; ja`: the idiom's interior is entered from elsewhere
    // (landings_in_idioms, byte ranges: also the middle of a multi-byte nop, which is not an instruction start of the closure)
    for (pad, k) in [(NOP_8, 0usize), (NOP_8, 3), (NOP_8, 7), (NOP_12, 1), (NOP_12, 5), (NOP_9, 1)] {
        for o in [
            Sw { pad: nops(&[pad]), pad_landing: Some(k), ..Sw::c1() },
            Sw { pad: nops(&[pad]), pad_landing: Some(k), ..Sw::c2() },
        ] {
            rejected(&o, "inside the switch idiom");
        }
    }
    // a branch to the first instruction of the idiom after the padding (the `mov`/`movzx`) also skips the guard
    rejected(&Sw { pad: nops(&[NOP_12]), landing: true, ..Sw::c1() }, "inside the switch idiom");
    rejected(&Sw { pad: nops(&[NOP_12]), landing: true, ..Sw::c2() }, "inside the switch idiom");
    // between two padding nops
    rejected(&Sw { pad: nops(&[NOP_5, NOP_4]), pad_landing: Some(5), ..Sw::c1() }, "inside the switch idiom");
    // and with the `cdqe`
    rejected(&Sw { cdqe: true, guard: Guard::Rax, pad: nops(&[NOP_9]), pad_landing: Some(4), ..Sw::c1() }, "inside the switch idiom");
}

#[test]
fn which_check_rejects_a_branch_into_the_padding() {
    // the message tells the check: the middle of a multi-byte nop (not an instruction start) and the start of the padding
    // are found by `landings_in_idioms` (the branch lands after the guard's `cmp`), and the `mov` after the padding by
    // `verify_msvc` or `landings_in_idioms`; whichever is first, the site is not resolved and `jumptab_rejected` stays empty
    for k in [0usize, 3] {
        let (_, a) = run(&Sw { pad: nops(&[NOP_8]), pad_landing: Some(k), ..Sw::c1() });
        let msg = &a.closure.msvc_rejected[0].1;
        assert!(msg.contains("lands at") && msg.contains("the guard `cmp; ja` is skipped"), "k={k}: {msg}");
    }
}

#[test]
fn verify_msvc_accepts_the_padding_run_and_still_rejects_a_block_start_inside_it() {
    // decision 0093, item 2: `verify_msvc` walks the instructions from the one after the guard's `ja` (`from`) to the jump and
    // refuses a block start after the first. The padding nops lie inside that run, so a nop that is only a fall-through is
    // not a block start (accepted), and a nop that a branch lands on is (rejected). Called directly with a block-start set,
    // because `landings_in_idioms` rejects such a branch earlier in the closure.
    let o = Sw { pad: nops(&[NOP_5, NOP_3]), ..Sw::c1() };
    let (b, a) = run(&o);
    let img = b.img.build();
    let pe = Pe::parse(&img).unwrap();
    let text = fe_decode::Text::from_pe(&pe).unwrap();
    let fm = jumptab::FuncMap::from_pe(&pe).unwrap();
    let (primary, pieces) = fm.function_at(AT).unwrap();
    let sw = jumptab::Sweep::build(&pe, &text, pieces).unwrap();
    let entry = pe.rva_to_va(primary);
    let fe_decode::Origin::Msvc { from, guard_ja, guard_cmp, .. } = a.jumptabs[&jmp_va(&b)].origin else { panic!("not an MSVC table") };
    let jmp = jmp_va(&b);
    let second_nop = from + NOP_5.len() as u64;
    assert_eq!(a.insns[&second_nop].mnemonic(), Mnemonic::Nop);
    let verify = |leaders: &[u64]| {
        let set: std::collections::HashSet<u64> = leaders.iter().copied().collect();
        jumptab::verify_msvc(jmp, from, (guard_ja, guard_cmp), entry, &a.insns, &set, &a.functions, &sw)
    };
    // the fall-through of the `ja` is a block start (`from`) and is allowed; nothing else starts a block
    assert_eq!(verify(&[entry, from]), Ok(()));
    // a block start on the second padding nop, on the table instruction after the padding, or on the jump: rejected
    for at in [second_nop, IB + b.labels["idiom"] as u64, jmp] {
        let e = verify(&[entry, from, at]).unwrap_err();
        assert!(e.contains(&format!("a block starts at {at:#x}, inside the switch idiom")), "{at:#x}: {e}");
    }
}

#[test]
fn the_padding_does_not_change_the_rejections_of_the_other_conditions() {
    // base proof, index bound, targets: the same reasons as without padding (decision 0093 changes only the walk to the `ja`)
    let p = nops(&[NOP_12]);
    rejected(&Sw { lea: Lea::OneSided, pad: p.clone(), ..Sw::c1() }, "is not defined on every path");
    rejected(&Sw { lea: Lea::Twice, pad: p.clone(), ..Sw::c1() }, "reaches the jump");
    rejected(&Sw { lea: Lea::FromArg, pad: p.clone(), ..Sw::c2() }, "is not `lea");
    // a write that reaches another indirect jump (decision 0094; 0090 rejected any other indirect jump)
    rejected_unresolved(&Sw { other_jmp: OtherJmp::ReachedAfterWrite, pad: p.clone(), ..Sw::c1() }, "reaches the jump", 2);
    rejected(&Sw { base: 8, call: true, pad: p.clone(), ..Sw::c1() }, "caller-saved");
    rejected(&Sw { bad: Bad::OtherFunction, pad: p.clone(), ..Sw::c1() }, "target outside the function");
    rejected(&Sw { bad: Bad::Misaligned, pad: p.clone(), ..Sw::c2() }, "not an instruction start");
    rejected(&Sw { foreign: true, pad: p.clone(), ..Sw::c1() }, "outside the function");
    // the index bound: the compare is on another register, or there is no guard at all (the padding is not a guard)
    rejected(&Sw { guard: Guard::OtherReg, pad: p.clone(), ..Sw::c1() }, "N not derived");
    rejected(&Sw { guard: Guard::Missing, pad: p, ..Sw::c1() }, "N not derived");
}

#[test]
fn the_landing_window_covers_two_longest_padding_nops() {
    // `landings_in_idioms` looks for branch targets in (guard cmp, jmp] within MSVC_IDIOM_MAX bytes of the cmp. With up to
    // MSVC_SKIP padding nops of up to 15 bytes, `cmp r64,imm32` (7) + `ja rel32` (6) + 2 x 15 + movzx (10) + mov (10) +
    // add (3) + jmp (2) = 68 bytes from the cmp to the end of the jump: the window must hold the jmp at 66.
    let guard_cmp = 0x1000u64;
    let jmp = guard_cmp + 7 + 6 + 2 * 15 + 10 + 10 + 3;
    assert!(jmp - guard_cmp <= jumptab::MSVC_IDIOM_MAX, "the window ({}) is shorter than the idiom ({})", jumptab::MSVC_IDIOM_MAX, jmp - guard_cmp);
    // a branch from far away to the byte just in front of the `jmp`, to the middle of the padding and to the `jmp` itself
    for tgt in [jmp, jmp - 1, guard_cmp + 7 + 6 + 20] {
        let got = jumptab::landings_in_idioms(&[(guard_cmp, jmp)], [(0x9000u64, tgt)].into_iter());
        assert_eq!(got.get(&jmp), Some(&(0x9000, tgt)), "target {tgt:#x}");
    }
    // outside the idiom: before the cmp's end is not inside (the cmp itself is the guard), after the jmp is not inside
    for tgt in [guard_cmp, jmp + 1] {
        assert!(jumptab::landings_in_idioms(&[(guard_cmp, jmp)], [(0x9000u64, tgt)].into_iter()).is_empty(), "target {tgt:#x}");
    }
}

// ---- real images ----

fn read_if_present(env: &str, default: &str) -> Option<Vec<u8>> {
    let explicit = std::env::var_os(env);
    let path = match &explicit {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("{env}={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

#[test]
fn flengine_has_no_msvc_table_of_this_form() {
    let Some(data) = read_if_present("FLENGINE_PE", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = fe_decode::analyze_with_pdata_adjustors(&pe).unwrap();
    assert_eq!(a.closure.msvc_tables, 0);
    assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
    assert_eq!(a.closure.jumptabs, 4, "only the four annotated tables of decision 0036");
}

fn analyze_blend2d(data: &[u8]) -> fe_decode::Analysis {
    let pe = Pe::parse(data).unwrap();
    assert_eq!(pe.optional.image_base, 0x4700000);
    let seeds = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    analyze_with_options(&pe, &seeds, jumptab::FLENGINE, &Options::default()).unwrap()
}

/// The two sites that decision 0109 resolves in the closure of `--pdata` of the rebased Blend2D (the scheduled form: `cmp edx,N; ja;
/// movsxd rax,edx; lea B,[ImageBase]; mov r,[B+rax*4+T]; add; jmp`). Predicted by an independent scan (not yet run).
const B2D_SCHEDULED_0109: [u64; 2] = [0x4756752, 0x47596c2];

/// The 27 sites that decision 0094 resolves in the closure of `--pdata` (no seeds) of the rebased Blend2D, in addition to the
/// three of decision 0090; ascending VA. Each is a site of the form (c1)/(c2) whose B has 2..407 writes, all but one of the
/// reaching definitions being `lea B,[ImageBase]`'s (docs/investigations/blend2d-switch-reaching-defs.md §3).
const B2D_NEW_0094: [u64; 27] = [
    0x470749d, 0x4707d3a, 0x470b983, 0x470c607, 0x470d019, 0x47210ff, 0x4722765, 0x4724748, 0x4725144, 0x4725db7, 0x4728561, 0x47290af, 0x4760a33,
    0x476b168, 0x476cf0a, 0x47802d9, 0x478129c, 0x47aa0bc, 0x47aa54e, 0x48421c1, 0x48654a6, 0x48712cf, 0x487d636, 0x4889443, 0x48a208b, 0x48aef10,
    0x48af10a,
];

// ---- decision 0093 on the real image: Blend2D f_48a2060 (original 0x1801a2060), the site 0x48a208b ----

/// The 16 targets of the RVA table at 0x49fd050 (r8 = 0..15), from `objdump -d` of the rebased Blend2D
/// (lift/out-nop93/logs/fn48a2060_facts.txt); each is the first byte of an instruction of the function.
const F48A2060_TARGETS: [u64; 16] = [
    0x48a208e, 0x48a2174, 0x48a20c8, 0x48a20ff, 0x48a217a, 0x48a215f, 0x48a2150, 0x48a20d0, 0x48a216d, 0x48a2135, 0x48a2126, 0x48a20b0,
    0x48a2143, 0x48a2110, 0x48a20e8, 0x48a2090,
];

fn decode_range(pe: &Pe<'_>, from: u64, to: u64) -> std::collections::BTreeMap<u64, iced_x86::Instruction> {
    let rva = pe.va_to_rva(from).unwrap();
    let bytes = pe.slice_at_rva(rva, (to - from) as usize).unwrap();
    let mut d = iced_x86::Decoder::with_ip(64, bytes, from, iced_x86::DecoderOptions::NONE);
    let mut m = std::collections::BTreeMap::new();
    while d.can_decode() {
        let i = d.decode();
        assert!(!i.is_invalid());
        m.insert(i.ip(), i);
    }
    m
}

#[test]
fn blend2d_f_48a2060_the_padded_site_is_recognised_with_the_form_the_bytes_give() {
    let Some(data) = read_if_present("BLEND2D_PE", "../../out-seed58/s58/dll/Blend2D.4700000.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    // the bytes of the head of the function, read from the file
    let head = pe.slice_at_rva(0x1a2060 + 0x14, 12).unwrap();
    assert_eq!(head, &[0x66, 0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00], "the 12-byte padding at 0x48a2074");
    let insns = decode_range(&pe, 0x48a2060, 0x48a208e);
    let pad = &insns[&0x48a2074];
    assert_eq!((pad.mnemonic(), pad.len()), (Mnemonic::Nop, 12));
    assert!(jumptab::is_padding_nop(pad));
    let jmp = &insns[&0x48a208b];
    let form = jumptab::recognize_msvc(&insns, 0x48a208b, jmp).expect("recognised").expect("a table idiom");
    assert_eq!(
        form,
        jumptab::MsvcForm {
            first: 0x48a2080,
            from: 0x48a2074,
            guard_ja: 0x48a206e,
            guard_cmp: 0x48a206a,
            base: iced_x86::Register::R10,
            n: 15,
            t1: None,
            t2: 0x2fd050,
            swapped: None
        }
    );
}

#[test]
fn blend2d_f_48a2060_the_sixteen_table_entries_are_instruction_starts_in_the_function() {
    let Some(data) = read_if_present("BLEND2D_PE", "../../out-seed58/s58/dll/Blend2D.4700000.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    // the RVA table at 0x49fd050 (RVA 0x2fd050): 16 entries, read from the file, equal to the objdump list
    let raw = pe.slice_at_rva(0x2fd050, 64).unwrap();
    let got: Vec<u64> = raw.chunks_exact(4).map(|c| 0x4700000 + u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as u64).collect();
    assert_eq!(got, F48A2060_TARGETS);
    assert_eq!(got.iter().collect::<BTreeSet<_>>().len(), 16, "16 distinct targets");
    // the function is the one RUNTIME_FUNCTION [0x48a2060, 0x48a26d5); every target is inside it and an instruction start of
    // the linear sweep of the whole function (decoded here from the file bytes, independently of the generator)
    let insns = decode_range(&pe, 0x48a2060, 0x48a26d5);
    for (r8, t) in got.iter().enumerate() {
        assert!((0x48a2060..0x48a26d5).contains(t), "r8={r8}: {t:#x} outside the function");
        assert!(insns.contains_key(t), "r8={r8}: {t:#x} is not an instruction start");
    }
    // the function table: one RUNTIME_FUNCTION, no chain
    let fm = jumptab::FuncMap::from_pe(&pe).unwrap();
    let (primary, pieces) = fm.function_at(0x48a208b - 0x4700000).unwrap();
    assert_eq!((primary, pieces), (0x1a2060, &[(0x1a2060u32, 0x1a26d5u32)][..]));
}

/// The 16 targets of the RVA table 0x2615000 of dsp_ippv2 `f_37cc7e0` (base 0x2000000), r8 = 0..15; objdump and capstone agree
/// (lift/out-rdefs/logs/table-facts.txt). The same function as Blend2D's `f_48a2060` at another address.
const F37CC7E0_TARGETS: [u64; 16] = [
    0x37cc80e, 0x37cc8f4, 0x37cc848, 0x37cc87f, 0x37cc8fa, 0x37cc8df, 0x37cc8d0, 0x37cc850, 0x37cc8ed, 0x37cc8b5, 0x37cc8a6, 0x37cc830,
    0x37cc8c3, 0x37cc890, 0x37cc868, 0x37cc810,
];

#[test]
fn dsp_ippv2_f_37cc7e0_is_resolved_by_reaching_definitions_with_the_sixteen_entries_of_the_table() {
    // the same function as `f_48a2060` (base 0x2000000): `jmp r9` at 0x37cc80b, three `jmp r11` (N not derived)
    let explicit = std::env::var_os("DSP_IPPV2_PE");
    let path = match &explicit {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll"),
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            return;
        }
    };
    let pe = Pe::parse(&data).unwrap();
    if pe.optional.image_base != 0x2000000 {
        eprintln!("SKIPPED: the image base is {:#x}, not the rebased 0x2000000", pe.optional.image_base);
        return;
    }
    let a = fe_decode::analyze_with_pdata_adjustors(&pe).unwrap();
    assert!(a.closure.msvc_rejected.iter().all(|(va, _)| *va != 0x37cc80b), "{:?}", a.closure.msvc_rejected);
    let t = &a.jumptabs[&0x37cc80b];
    let cases: BTreeSet<u64> = t.cases.iter().map(|&(c, tg)| {
        assert_eq!(c, tg);
        c
    }).collect();
    assert_eq!(cases, F37CC7E0_TARGETS.iter().copied().collect::<BTreeSet<_>>());
    assert_eq!(cases.len(), 16);
    assert!(matches!(t.origin, fe_decode::Origin::Msvc { n: 15, t1: None, t2: 0x2615000, .. }), "{:?}", t.origin);
    for tg in &cases {
        assert!(a.insns.contains_key(tg), "{tg:#x}");
    }
    // the four sites of dsp_ippv2 (--pdata) that decision 0094 adds to the one of 0089 (0x37c69eb), and the swapped site of
    // decision 0095 (`0x37cd3eb`, `f_37cd2c0`): 5 -> 6, and the two scheduled sites of decision 0109 (`0x37b5cea`, `0x37b6554`): 6 -> 8
    let got: BTreeSet<u64> = a.jumptabs.iter().filter(|(_, t)| matches!(t.origin, fe_decode::Origin::Msvc { .. })).map(|(&v, _)| v).collect();
    assert_eq!(got, BTreeSet::from([0x37b25bd, 0x37b5cea, 0x37b6554, 0x37c69eb, 0x37cc80b, 0x37cd3eb, 0x37d9670, 0x37d986a]));
}

// ---- decision 0095: the swapped form `lea B; mov eX,[B+I*4+T]; add B,rX; <gap>; jmp B`, N from the guard's branch ----
//
// Synthetic images (`build_swapped`): I = rcx. The table block is the code after the guard's branch (`tbl`), preceded by an
// optional padding (`pad`) that no branch skips. The default block is `mov eax,0xdead; ret`.

fn swp_run(o: &Swp) -> (Built, fe_decode::Analysis) {
    let built = build_swapped(AT, o);
    let img = built.img.build();
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
    (built, a)
}

fn swp_resolved(o: &Swp) -> (Built, fe_decode::Analysis) {
    let (b, a) = swp_run(o);
    assert!(a.closure.msvc_rejected.is_empty(), "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (1, 1), "{o:?}");
    let t = &a.jumptabs[&jmp_va(&b)];
    assert_eq!(t.select, Select::Target);
    let cases: BTreeSet<u64> = t.cases.iter().map(|&(c, tg)| {
        assert_eq!(c, tg, "a case is the target VA the register holds");
        c
    }).collect();
    let want: BTreeSet<u64> = b.table[..b.entries].iter().map(|&r| IB + r as u64).collect();
    assert_eq!(cases, want, "{o:?}");
    for tg in &cases {
        assert!(a.insns.contains_key(tg), "target {tg:#x} not in the closure");
    }
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert_eq!(a.closure.indirect.by_kind["jmp:msvc_table"], 1);
    match t.origin {
        fe_decode::Origin::Msvc { n, swapped: Some(lo), from, guard_cmp, guard_ja, function, .. } => {
            assert_eq!(n, o.n as u64, "{o:?}");
            assert_eq!(IB + b.labels["guard"] as u64, guard_cmp, "{o:?}");
            assert!(guard_ja > guard_cmp && lo <= from, "{o:?}");
            // the table block starts at the first instruction after the branch when it is the fall-through side (the padding
            // `nop` then belongs to the block), and at the branch's target (behind the padding) when it is the taken side
            let taken = matches!(o.guard, SwGuard::JbeTaken | SwGuard::JbTaken);
            assert_eq!(from, IB + b.labels[if taken { "tbl" } else { "padl" }] as u64, "{o:?}");
            assert_eq!(lo, IB + b.labels["padl"] as u64, "{o:?}");
            assert_eq!(function, AT);
        }
        other => panic!("{o:?}: not a swapped table: {other:?}"),
    }
    (b, a)
}

fn swp_rejected(o: &Swp, why: &str) -> (Built, fe_decode::Analysis) {
    let (b, a) = swp_run(o);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?} must not be resolved: {:?}", a.closure.msvc_rejected);
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)), "{o:?}");
    let r: Vec<&(u64, String)> = a.closure.msvc_rejected.iter().filter(|(v, _)| *v == jmp_va(&b)).collect();
    assert_eq!(r.len(), 1, "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(r[0].1.contains(why), "{o:?}: wanted {why:?}, got {:?}", r[0].1);
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    (b, a)
}

/// Neither resolved nor reported: `recognize_msvc` finds no table idiom at all.
fn swp_not_recognised(o: &Swp) {
    let (b, a) = swp_run(o);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?}");
    assert!(a.closure.msvc_rejected.iter().all(|(v, _)| *v != jmp_va(&b)), "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)), "{o:?}");
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
}

#[test]
fn swapped_the_shape_of_blend2d_f_489f7d0_is_resolved_with_n_from_the_jbe_taken_side() {
    // `cmp rcx,16; jbe tbl`, a 12-byte nop in front of the table block, `lea r9; mov eax,[r9+rcx*4+T]; add r9,rax; add rdx,rcx;
    // mov rax,r11; jmp r9`: 17 entries (N + 1), 3 distinct targets
    let (b, a) = swp_resolved(&Swp::real());
    assert_eq!(b.entries, 17);
    assert_eq!(a.jumptabs[&jmp_va(&b)].cases.len(), 3);
    // the guard's branch is the `jbe`; the table block starts after the nop, which is inside the protected run
    let fe_decode::Origin::Msvc { swapped: Some(lo), from, .. } = a.jumptabs[&jmp_va(&b)].origin else { panic!() };
    assert_eq!(from - lo, 12, "the nop is inside the run that only the guard's branch may enter");
}

#[test]
fn swapped_n_comes_from_every_unsigned_i_le_n_side() {
    // `ja` not taken, `jbe` taken, `jae` not taken (cmp N+1), `jb` taken (cmp N+1: N = imm - 1), 32-bit compare
    for guard in [SwGuard::JbeTaken, SwGuard::JaFall, SwGuard::JaFall32, SwGuard::JaeFall, SwGuard::JbTaken] {
        for n in [2u32, 5, 16, 0x7f, 0x80, 200] {
            let o = Swp { guard, n, pad: Vec::new(), ..Swp::real() };
            let (b, _) = swp_resolved(&o);
            assert_eq!(b.entries, n as usize + 1, "{guard:?} {n}");
        }
        swp_resolved(&Swp { guard, ..Swp::real() });
    }
    // the padding in front of the table block is allowed after a fall-through guard too
    swp_resolved(&Swp { guard: SwGuard::JaFall, pad: vec![NOP_5.to_vec(), NOP_3.to_vec()], ..Swp::real() });
}

#[test]
fn swapped_the_gaps_before_and_after_the_add_hold_instructions_that_touch_none_of_b_x_i() {
    // before the `add`: untouched B, X, I (rdx; r10 and r12 are not any of them). After it: only B may not be touched
    let neutral_before = mov_to_from_rdx(12);
    swp_resolved(&Swp { gap_before: vec![neutral_before.clone()], gap_after: vec![MOV_RAX_R11.to_vec()], ..Swp::real() });
    swp_resolved(&Swp { gap_before: vec![neutral_before.clone(), mov_to_from_rdx(10)], gap_after: Vec::new(), ..Swp::real() });
    swp_resolved(&Swp { gap_before: Vec::new(), gap_after: Vec::new(), ..Swp::real() });
    swp_resolved(&Swp { gap_before: Vec::new(), gap_after: vec![ADD_RDX_RCX.to_vec()], ..Swp::real() });
    // after the `add` X and I are free: `add rdx,rcx` reads I, `mov rax,r11` writes X (the real form)
    swp_resolved(&Swp::real());
    // other registers for B, X
    swp_resolved(&Swp { base: 10, x: 3, ..Swp::real() });
    swp_resolved(&Swp { base: 13, x: 0, push_pop: true, ..Swp::real() });
    // the limit: MSVC_SWAP_SKIP instructions in both gaps together
    assert_eq!(jumptab::MSVC_SWAP_SKIP, 2);
    assert_ne!(jumptab::MSVC_SWAP_SKIP, jumptab::MSVC_SKIP + 1, "a separate constant, not 0093's");
}

#[test]
fn swapped_the_lea_may_sit_at_the_entry_before_the_guard() {
    swp_resolved(&Swp { lea: SwLea::AtEntry, pad: Vec::new(), ..Swp::real() });
    swp_resolved(&Swp { lea: SwLea::AtEntry, push_pop: true, ..Swp::real() });
    // a write to B on a branch that never comes back to the table load is not a reaching definition (0094's rule)
    swp_resolved(&Swp { def: SwDef::WriteThenRet, ..Swp::real() });
    swp_resolved(&Swp { lea: SwLea::AtEntry, def: SwDef::WriteThenRet, ..Swp::real() });
}

#[test]
fn swapped_the_wrong_side_of_the_guard_is_not_a_bound() {
    // the table block is entered from the I > N side: N is not derived
    for guard in [SwGuard::JaTaken, SwGuard::JbeFall, SwGuard::JaeTaken, SwGuard::JbFall] {
        swp_rejected(&Swp { guard, pad: Vec::new(), ..Swp::real() }, "N not derived");
        swp_rejected(&Swp { guard, ..Swp::real() }, "N not derived");
    }
    let (_, a) = swp_run(&Swp { guard: SwGuard::JaTaken, ..Swp::real() });
    let msg = &a.closure.msvc_rejected[0].1;
    assert!(msg.contains("decision 0095") && msg.contains("unsigned I <= N side"), "{msg}");
}

#[test]
fn swapped_the_compare_must_be_on_the_index_with_an_immediate_and_nothing_may_write_it() {
    // `cmp` on another register: the flags are last written by a `cmp` that is not on the index
    swp_rejected(&Swp { guard: SwGuard::OtherReg, ..Swp::real() }, "not by a `cmp RCX,imm`");
    // the flags come from a `test`, from a compare with a register
    swp_rejected(&Swp { guard: SwGuard::NoCmp, ..Swp::real() }, "not by a `cmp");
    swp_rejected(&Swp { guard: SwGuard::CmpReg, ..Swp::real() }, "not by a `cmp RCX,imm`");
    // (b) I written between the `cmp` and the branch (the `mov` leaves the flags alone, so the branch still tests the `cmp`)
    swp_rejected(&Swp { guard: SwGuard::WriteBetween, ..Swp::real() }, "is written at");
    // (c) I written after the branch, before the load
    swp_rejected(&Swp { guard: SwGuard::WriteAfter, ..Swp::real() }, "between the guard's branch and the table load");
}

#[test]
fn swapped_the_table_block_needs_exactly_one_predecessor() {
    // a second branch into the table block
    swp_rejected(&Swp { guard: SwGuard::ExtraBranch, ..Swp::real() }, "predecessors");
    // the default block falls into the table block (two predecessors: the `jbe` and the fall-through)
    swp_rejected(&Swp { guard: SwGuard::DefFallsIn, ..Swp::real() }, "predecessors");
    // a table target that jumps back into the table block (to the first padding instruction / the load / the `add`)
    swp_rejected(&Swp { target2: SwTarget::JmpToPad, ..Swp::real() }, "predecessors");
    swp_rejected(&Swp { target2: SwTarget::JmpToPad, pad: Vec::new(), ..Swp::real() }, "predecessors");
    // a table target that jumps to the load: the load is a branch target, the block of the load starts there and has two
    // predecessors (the jump and the instruction before it)
    swp_rejected(&Swp { target2: SwTarget::JmpToLoad, ..Swp::real() }, "predecessors");
    // a table target that jumps to the `add`: the block (and N) are fine, the branch lands inside the idiom after the guard
    swp_rejected(&Swp { target2: SwTarget::JmpToAdd, ..Swp::real() }, "inside the switch idiom");
    // a table target that jumps back to the guard: the guard runs again and the `lea` in the table block re-defines B, so
    // nothing is skipped and the add's definition does not reach the load
    swp_resolved(&Swp { target2: SwTarget::LoopToGuard, ..Swp::real() });
}

#[test]
fn swapped_f2_and_the_other_table_loads_are_not_taken() {
    // Blend2D F2 (`mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; jmp rcx`): no `add B,rX` with B = the jump register
    // and a load through B
    swp_not_recognised(&Swp { idiom: SwIdiom::F2, ..Swp::real() });
    // `movsxd`
    swp_not_recognised(&Swp { idiom: SwIdiom::Movsxd, ..Swp::real() });
    // the load is through a register that is not the jump register (`mov eX,[r10+I*4]; add B,rX; jmp B`)
    swp_not_recognised(&Swp { idiom: SwIdiom::LoadBaseOther, base: 9, ..Swp::real() });
    swp_not_recognised(&Swp { idiom: SwIdiom::LoadBaseOther, base: 10, ..Swp::real() });
    // the add and the jump are on another register than the base of the load: that is the standard order, with B = r10... no
    // `add rT,B` (the register of the load is not the second operand), so neither form
    swp_not_recognised(&Swp { idiom: SwIdiom::JmpOther, ..Swp::real() });
    // X is the index register
    swp_not_recognised(&Swp { idiom: SwIdiom::XIsIndex, ..Swp::real() });
    // the two-stage table with the swapped add is not taken (`mov eX,[B+rdx*4]` is preceded by a `movzx`, the `movzx` makes
    // the index rdx: this is not the form (the load must index by the register the `cmp` is on))
    let (b, a) = swp_run(&Swp { idiom: SwIdiom::TwoStage, ..Swp::real() });
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0));
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)));
}

#[test]
fn swapped_an_instruction_in_a_gap_that_touches_b_x_i_or_the_flags_or_memory_or_the_control_flow_is_refused() {
    let cases: Vec<(&str, Swp)> = vec![
        // before the `add`: X written (the `add` would add a different value), I written, B written, X read is fine?
        ("x written before the add (mov rax,r11)", Swp { gap_before: vec![MOV_RAX_R11.to_vec()], gap_after: Vec::new(), ..Swp::real() }),
        ("i written before the add", Swp { gap_before: vec![MOV_RCX_RDX.to_vec()], gap_after: Vec::new(), ..Swp::real() }),
        ("i read before the add", Swp { gap_before: vec![ADD_RDX_RCX.to_vec()], gap_after: Vec::new(), ..Swp::real() }),
        ("flags read before the add", Swp { gap_before: vec![ADC_RDX_RDX.to_vec()], gap_after: Vec::new(), ..Swp::real() }),
        ("memory written before the add", Swp { gap_before: vec![MOV_MEM_RSP_RDX.to_vec()], gap_after: Vec::new(), ..Swp::real() }),
        // after the `add`: B touched
        ("b read after the add", Swp { gap_before: Vec::new(), gap_after: vec![mov_rax_from(9)], ..Swp::real() }),
        ("b written after the add", Swp { gap_before: Vec::new(), gap_after: vec![mov_to_from_rdx(9)], ..Swp::real() }),
        ("flags read after the add", Swp { gap_before: Vec::new(), gap_after: vec![ADC_RDX_RDX.to_vec()], ..Swp::real() }),
        ("memory written after the add", Swp { gap_before: Vec::new(), gap_after: vec![MOV_MEM_RSP_RDX.to_vec()], ..Swp::real() }),
    ];
    for (name, o) in cases {
        let (b, a) = swp_run(&o);
        assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{name}");
        assert!(!a.jumptabs.contains_key(&jmp_va(&b)), "{name}");
        // either not recognised at all, or reported as rejected for the gap: never resolved
        assert!(a.closure.msvc_rejected.len() <= 1, "{name}: {:?}", a.closure.msvc_rejected);
    }
    // the gap error names the instruction (a gap instruction before the `add` that is neither neutral nor refused silently)
    let (_, a) = swp_run(&Swp { gap_before: vec![MOV_RCX_RDX.to_vec()], gap_after: Vec::new(), ..Swp::real() });
    let msg = a.closure.msvc_rejected.iter().map(|(_, m)| m.as_str()).collect::<Vec<_>>().join("|");
    assert!(msg.contains("between the table load and the `add"), "{msg}");
}

#[test]
fn swapped_the_limit_on_the_instructions_in_the_gaps_is_not_loosened() {
    // three instructions after the `add`, or two after plus one before, or three before: more than MSVC_SWAP_SKIP (2)
    let n1 = mov_to_from_rdx(12);
    let n2 = mov_to_from_rdx(10);
    let n3 = mov_to_from_rdx(14);
    swp_not_recognised(&Swp { gap_before: Vec::new(), gap_after: vec![n1.clone(), n2.clone(), n3.clone()], ..Swp::real() });
    swp_not_recognised(&Swp { gap_before: vec![n3.clone()], gap_after: vec![n1.clone(), n2.clone()], ..Swp::real() });
    swp_not_recognised(&Swp { gap_before: vec![n1, n2, n3], gap_after: Vec::new(), ..Swp::real() });
    // exactly the limit is accepted
    swp_resolved(&Swp { gap_before: vec![mov_to_from_rdx(12)], gap_after: vec![mov_to_from_rdx(10)], ..Swp::real() });
}

#[test]
fn swapped_the_base_proof_of_decision_0094_is_not_loosened() {
    // B has no `lea`: no definition reaches the load
    swp_rejected(&Swp { lea: SwLea::Never, ..Swp::real() }, "no definition of R9 reaches");
    // a `lea` of another address
    swp_rejected(&Swp { lea: SwLea::Wrong(0x10), ..Swp::real() }, "is not `lea");
    // a `mov` and the `lea` both reach the load
    swp_rejected(&Swp { lea: SwLea::AtEntryOrArg, ..Swp::real() }, "reaches the table load");
    // the `lea` and a non-`lea` write that reaches another indirect jump (a wildcard, so it reaches the load)
    swp_rejected(&Swp { def: SwDef::WriteThenJmpRdx, lea: SwLea::AtEntry, ..Swp::real() }, "reaches");
    // caller-saved B and a call between the `lea` and the load (decision 0094 item 3); callee-saved B survives it
    swp_rejected(&Swp { lea: SwLea::AtEntryThenCall, pad: Vec::new(), ..Swp::real() }, "caller-saved");
    swp_resolved(&Swp { lea: SwLea::AtEntryThenCall, base: 12, push_pop: true, pad: Vec::new(), ..Swp::real() });
    // the idiom's own `add B,rX` is a definition of B. When the `lea` is in front of the guard (not in the table block), a table
    // target that goes back to the guard brings that definition (B = a target) to the load through the jump's own table: it is
    // not `lea`, so the proof rejects it. With the `lea` in the table block the `lea` replaces it on every path to the load
    swp_rejected(&Swp { target2: SwTarget::LoopToGuard, lea: SwLea::AtEntry, ..Swp::real() }, "the definition of R9 at");
    let (_, a) = swp_run(&Swp { target2: SwTarget::LoopToGuard, lea: SwLea::AtEntry, ..Swp::real() });
    assert!(a.closure.msvc_rejected[0].1.contains("`Add`") && a.closure.msvc_rejected[0].1.contains("the table load"), "{:?}", a.closure.msvc_rejected);
}

#[test]
fn swapped_the_targets_must_be_inside_the_function_and_instruction_starts() {
    // the table of `build_swapped` has 4 distinct targets (t0, t1, t2 and the bad entry 1)
    swp_rejected(&Swp { bad: Bad::OtherFunction, ..Swp::real() }, "target outside the function: 1 of 4 distinct target(s)");
    swp_rejected(&Swp { bad: Bad::NotExecutable, ..Swp::real() }, "in no .pdata function, not executable");
    swp_rejected(&Swp { bad: Bad::Misaligned, ..Swp::real() }, "not an instruction start");
    swp_resolved(&Swp { bad: Bad::ChainedPiece, ..Swp::real() });
    swp_rejected(&Swp { no_pdata: true, ..Swp::real() }, "not inside a .pdata function");
    // another function jumps into the middle of this one
    swp_rejected(&Swp { foreign: true, ..Swp::real() }, "outside the function");
}

#[test]
fn swapped_does_not_change_what_the_standard_form_does() {
    // the standard form is recognised first: all the (c1)/(c2) results of the earlier tests are unchanged. A function that has
    // both forms resolves both
    resolved_ok(&Sw::c1());
    resolved_ok(&Sw::c2());
    rejected(&Sw { guard: Guard::Missing, ..Sw::c1() }, "N not derived");
    // a standard idiom whose guard is missing is not turned into a swapped one: `add rcx,B` has B as the second operand
    let (b, a) = run(&Sw { guard: Guard::Missing, ..Sw::c1() });
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)));
}

#[test]
fn swapped_the_landing_window_function_is_exact() {
    // (lo, jmp, branch, from): only (branch, from) may enter [lo, jmp]
    let r = [(0x1000u64, 0x1040u64, 0x0f00u64, 0x100cu64)];
    for tgt in [0x1000u64, 0x1005, 0x100c, 0x1020, 0x1040] {
        assert!(jumptab::landings_in_swapped_idioms(&r, [(0x9000u64, tgt)].into_iter()).contains_key(&0x1040), "{tgt:#x}");
    }
    // the guard's own branch to the table block is the way in
    assert!(jumptab::landings_in_swapped_idioms(&r, [(0x0f00u64, 0x100cu64)].into_iter()).is_empty());
    // the guard's branch to anywhere else inside is not
    assert!(!jumptab::landings_in_swapped_idioms(&r, [(0x0f00u64, 0x1010u64)].into_iter()).is_empty());
    // outside the range: nothing
    for tgt in [0x0fffu64, 0x1041] {
        assert!(jumptab::landings_in_swapped_idioms(&r, [(0x9000u64, tgt)].into_iter()).is_empty(), "{tgt:#x}");
    }
}

// ---- decision 0095 on the real images ----

/// The 17 targets of the RVA table 0x2fd000 (`f_489f7d0`, r8 = 0..16), from `objdump -d` and capstone of the rebased Blend2D
/// (lift/out-swapped/logs/facts-b2d.txt); each is the first byte of an instruction of the function [0x489f7d0, 0x489f960).
const F489F7D0_TARGETS: [u64; 17] = [
    0x489f90e, 0x489f90b, 0x489f937, 0x489f907, 0x489f914, 0x489f924, 0x489f934, 0x489f904, 0x489f93c, 0x489f918, 0x489f950, 0x489f940, 0x489f910,
    0x489f920, 0x489f930, 0x489f900, 0x489f958,
];

/// The 17 targets of dsp_ippv2's `f_37cd2c0` (RVA table 0x26150b0, base 0x2000000), r8 = 0..16 (lift/out-swapped/logs/facts-dsp.txt).
const F37CD2C0_TARGETS: [u64; 17] = [
    0x37cd3fe, 0x37cd3fb, 0x37cd427, 0x37cd3f7, 0x37cd404, 0x37cd414, 0x37cd424, 0x37cd3f4, 0x37cd42c, 0x37cd408, 0x37cd440, 0x37cd430, 0x37cd400,
    0x37cd410, 0x37cd420, 0x37cd3f0, 0x37cd448,
];

#[test]
fn blend2d_f_489f7d0_is_recognised_with_the_swapped_form_the_bytes_give() {
    let Some(data) = read_if_present("BLEND2D_PE", "../../out-seed58/s58/dll/Blend2D.4700000.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    // the bytes, read from the file
    for (va, want) in [
        (0x489f7e4u64, &[0x49u8, 0x83, 0xf8, 0x10][..]),
        (0x489f7e8, &[0x0f, 0x86, 0xf2, 0x00, 0x00, 0x00]),
        (0x489f8e3, &[0x4c, 0x8d, 0x0d, 0x16, 0x07, 0xe6, 0xff]),
        (0x489f8ea, &[0x43, 0x8b, 0x84, 0x81, 0x00, 0xd0, 0x2f, 0x00]),
        (0x489f8f2, &[0x4c, 0x03, 0xc8]),
        (0x489f8f5, &[0x49, 0x03, 0xc8]),
        (0x489f8f8, &[0x49, 0x8b, 0xc3]),
        (0x489f8fb, &[0x41, 0xff, 0xe1]),
    ] {
        assert_eq!(pe.slice_at_rva((va - 0x4700000) as u32, want.len()).unwrap(), want, "bytes at {va:#x}");
    }
    let insns = decode_range(&pe, 0x489f8e0, 0x489f8fe);
    let jmp = &insns[&0x489f8fb];
    let form = jumptab::recognize_msvc(&insns, 0x489f8fb, jmp).expect("recognised").expect("a table idiom");
    assert_eq!(form.base, iced_x86::Register::R9);
    assert_eq!(form.t2, 0x2fd000);
    assert_eq!(form.t1, None);
    assert_eq!(form.swapped, Some(jumptab::SwapForm { load: 0x489f8ea, add: 0x489f8f2, index: iced_x86::Register::R8, scheduled: false }));
}

#[test]
fn blend2d_f_489f7d0_the_seventeen_table_entries_are_instruction_starts_in_the_function() {
    let Some(data) = read_if_present("BLEND2D_PE", "../../out-seed58/s58/dll/Blend2D.4700000.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let raw = pe.slice_at_rva(0x2fd000, 17 * 4).unwrap();
    let got: Vec<u64> = raw.chunks_exact(4).map(|c| 0x4700000 + u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as u64).collect();
    assert_eq!(got, F489F7D0_TARGETS);
    assert_eq!(got.iter().collect::<BTreeSet<_>>().len(), 17, "17 distinct targets");
    // the 18th..20th dwords are 0, 0, 0 and the next one is the next table's first entry (0x1a208e): the length is N + 1, not a distance
    let more = pe.slice_at_rva(0x2fd000 + 17 * 4, 16).unwrap();
    assert_eq!(more, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x8e, 0x20, 0x1a, 0]);
    let insns = decode_range(&pe, 0x489f7d0, 0x489f960);
    for (r8, t) in got.iter().enumerate() {
        assert!((0x489f7d0..0x489f960).contains(t), "r8={r8}: {t:#x} outside the function");
        assert!(insns.contains_key(t), "r8={r8}: {t:#x} is not an instruction start");
    }
    let fm = jumptab::FuncMap::from_pe(&pe).unwrap();
    let (primary, pieces) = fm.function_at(0x489f8fb - 0x4700000).unwrap();
    assert_eq!((primary, pieces), (0x19f7d0, &[(0x19f7d0u32, 0x19f960u32)][..]));
    // the only branch into 0x489f8e0 is the `jbe` at 0x489f7e8, and the instruction before it is a padding nop after a `jmp`
    let sources: Vec<u64> = insns
        .values()
        .filter(|i| matches!(i.flow_control(), iced_x86::FlowControl::UnconditionalBranch | iced_x86::FlowControl::ConditionalBranch | iced_x86::FlowControl::Call))
        .filter(|i| matches!(i.op0_kind(), OpKind::NearBranch64) && i.near_branch_target() == 0x489f8e0)
        .map(|i| i.ip())
        .collect();
    assert_eq!(sources, vec![0x489f7e8]);
    assert_eq!(insns[&0x489f8d2].flow_control(), iced_x86::FlowControl::UnconditionalBranch);
    assert_eq!(insns[&0x489f8d4].mnemonic(), Mnemonic::Nop);
    assert_eq!(insns[&0x489f8d4].next_ip(), 0x489f8e0);
}

#[test]
fn dsp_ippv2_f_37cd2c0_is_resolved_with_seventeen_entries() {
    let explicit = std::env::var_os("DSP_IPPV2_PE");
    let path = match &explicit {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-jtb2d90/three/dll/dsp_ippv2_x64.2000000.dll"),
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if explicit.is_some() => panic!("DSP_IPPV2_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            return;
        }
    };
    let pe = Pe::parse(&data).unwrap();
    if pe.optional.image_base != 0x2000000 {
        eprintln!("SKIPPED: the image base is {:#x}, not the rebased 0x2000000", pe.optional.image_base);
        return;
    }
    let a = fe_decode::analyze_with_pdata_adjustors(&pe).unwrap();
    assert!(a.closure.msvc_rejected.iter().all(|(va, _)| *va != 0x37cd3eb), "{:?}", a.closure.msvc_rejected);
    let t = &a.jumptabs[&0x37cd3eb];
    let cases: Vec<u64> = t.cases.iter().map(|&(c, tg)| {
        assert_eq!(c, tg);
        c
    }).collect();
    assert_eq!(cases.iter().copied().collect::<BTreeSet<_>>(), F37CD2C0_TARGETS.iter().copied().collect::<BTreeSet<_>>());
    assert_eq!(cases.len(), 17);
    let fe_decode::Origin::Msvc { n, t2, swapped, .. } = t.origin else { panic!("{:?}", t.origin) };
    assert_eq!((n, t2), (0x10, 0x26150b0));
    assert!(swapped.is_some());
    for tg in &cases {
        assert!(a.insns.contains_key(tg), "{tg:#x}");
    }
    // the eight sites of dsp_ippv2 (--pdata): the five of decision 0094, this one, and the two scheduled of decision 0109 (0x37b5cea,
    // 0x37b6554: predicted by an independent scan, not yet run)
    let got: BTreeSet<u64> = a.jumptabs.iter().filter(|(_, t)| matches!(t.origin, fe_decode::Origin::Msvc { .. })).map(|(&v, _)| v).collect();
    assert_eq!(got, BTreeSet::from([0x37b25bd, 0x37b5cea, 0x37b6554, 0x37c69eb, 0x37cc80b, 0x37cd3eb, 0x37d9670, 0x37d986a]));
    // F2 of dsp (0x37c08f9) is not taken
    assert!(!a.jumptabs.contains_key(&0x37c08f9));
}

#[test]
fn flengine_has_no_swapped_table() {
    let Some(data) = read_if_present("FLENGINE_PE", "../../../binaries/win64/FLEngine_x64.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = fe_decode::analyze_with_pdata_adjustors(&pe).unwrap();
    assert_eq!(a.closure.msvc_tables, 0);
    assert!(a.closure.msvc_rejected.is_empty(), "{:?}", a.closure.msvc_rejected);
    assert_eq!(a.closure.jumptabs, 4);
}

#[test]
fn swapped_verify_msvc_swapped_accepts_the_run_and_rejects_a_block_start_inside_it() {
    // `landings_in_swapped_idioms` rejects a branch into the idiom earlier in the closure, so the walk of `verify_msvc_swapped`
    // is called directly with a set of block starts (as the test of decision 0093 does for `verify_msvc`)
    let (b, a) = swp_resolved(&Swp::real());
    let img = b.img.build();
    let pe = Pe::parse(&img).unwrap();
    let text = fe_decode::Text::from_pe(&pe).unwrap();
    let fm = jumptab::FuncMap::from_pe(&pe).unwrap();
    let (primary, pieces) = fm.function_at(AT).unwrap();
    let sw = jumptab::Sweep::build(&pe, &text, pieces).unwrap();
    let entry = pe.rva_to_va(primary);
    let fe_decode::Origin::Msvc { from, guard_ja, guard_cmp, swapped: Some(lo), .. } = a.jumptabs[&jmp_va(&b)].origin else { panic!("not a swapped table") };
    let jmp = jmp_va(&b);
    let verify = |leaders: &[u64]| {
        let set: std::collections::HashSet<u64> = leaders.iter().copied().collect();
        jumptab::verify_msvc_swapped(jmp, from, lo, (guard_ja, guard_cmp), entry, &a.insns, &set, &a.functions, &sw)
    };
    // the table block starts a block (the `jbe`'s target) and the guard's branch is the only way in: accepted
    assert_eq!(verify(&[entry, from]), Ok(()));
    // a block start on the load, on the `add`, on the jump: rejected
    for at in [IB + b.labels["load"] as u64, IB + b.labels["add"] as u64, jmp] {
        let e = verify(&[entry, from, at]).unwrap_err();
        assert!(e.contains(&format!("a block starts at {at:#x}, inside the swapped switch idiom")), "{at:#x}: {e}");
    }
    // the guard as recognised: a branch that does not lead to the run, a `cmp` after the branch, or an instruction that was not decoded
    let set: std::collections::HashSet<u64> = [entry, from].into_iter().collect();
    for bad in [(guard_ja + 1, guard_cmp), (guard_ja, guard_ja), (0x9999_9999, guard_cmp)] {
        let e = jumptab::verify_msvc_swapped(jmp, from, lo, bad, entry, &a.insns, &set, &a.functions, &sw).unwrap_err();
        assert!(e.contains("did not decode the guard"), "{bad:?}: {e}");
    }
    // the run must end at the jump and be decoded in sequence
    let e = jumptab::verify_msvc_swapped(jmp, from + 1, lo, (guard_ja, guard_cmp), entry, &a.insns, &set, &a.functions, &sw).unwrap_err();
    assert!(e.contains("did not decode the guard"), "{e}");
    // the fall-through side: the table block follows the branch, the run starts at `lo`
    let (b2, a2) = swp_resolved(&Swp { guard: SwGuard::JaFall, ..Swp::real() });
    let img2 = b2.img.build();
    let pe2 = Pe::parse(&img2).unwrap();
    let text2 = fe_decode::Text::from_pe(&pe2).unwrap();
    let fm2 = jumptab::FuncMap::from_pe(&pe2).unwrap();
    let (primary2, pieces2) = fm2.function_at(AT).unwrap();
    let sw2 = jumptab::Sweep::build(&pe2, &text2, pieces2).unwrap();
    let entry2 = pe2.rva_to_va(primary2);
    let fe_decode::Origin::Msvc { from: from2, guard_ja: ja2, guard_cmp: cmp2, swapped: Some(lo2), .. } = a2.jumptabs[&jmp_va(&b2)].origin else { panic!() };
    let jmp2 = jmp_va(&b2);
    let set2: std::collections::HashSet<u64> = [entry2].into_iter().collect();
    // the padding nop is the first instruction of the table block (the `ja`'s fall-through): `lo == from`, and the run starts there
    assert_eq!(lo2, from2);
    assert_eq!(a2.insns[&lo2].mnemonic(), Mnemonic::Nop);
    assert_eq!(jumptab::verify_msvc_swapped(jmp2, from2, lo2, (ja2, cmp2), entry2, &a2.insns, &set2, &a2.functions, &sw2), Ok(()));
    for at in [IB + b2.labels["load"] as u64, jmp2] {
        let set3: std::collections::HashSet<u64> = [entry2, at].into_iter().collect();
        let e = jumptab::verify_msvc_swapped(jmp2, from2, lo2, (ja2, cmp2), entry2, &a2.insns, &set3, &a2.functions, &sw2).unwrap_err();
        assert!(e.contains(&format!("a block starts at {at:#x}, inside the swapped switch idiom")), "{at:#x}: {e}");
    }
}

// ---- decision 0109: the scheduled form `cmp I,N; <p>; ja def; <q>; [head:] <idiom>`, N from every way into the idiom ----
//
// Synthetic images (`build_sched`): the index arrives in ecx and is copied to eax and edx; the first guard compares eax (or edx, or
// rax), `p` stands between the `cmp` and the branch, `q` between the branch and the idiom (the `lea B`, nops, a sign extension,
// stores), and a loop block (`t2`) may jump back to `head` behind a second guard.

fn sch_run(o: &Sch) -> (Built, fe_decode::Analysis) {
    let built = build_sched(AT, o);
    let img = built.img.build();
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
    (built, a)
}

/// The targets that the table of `o` selects: the three blocks (two-stage: the three RVAs), or the first `entries` of the table.
fn sch_want(b: &Built, o: &Sch) -> BTreeSet<u64> {
    if o.two_stage {
        ["t0", "t1", "t2"].iter().map(|l| IB + b.labels[l] as u64).collect()
    } else {
        b.table[..b.entries].iter().map(|&r| IB + r as u64).collect()
    }
}

fn sch_resolved(o: &Sch) -> (Built, fe_decode::Analysis) {
    let (b, a) = sch_run(o);
    assert!(a.closure.msvc_rejected.is_empty(), "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (1, 1), "{o:?}");
    let t = &a.jumptabs[&jmp_va(&b)];
    assert_eq!(t.select, Select::Target);
    let cases: BTreeSet<u64> = t.cases.iter().map(|&(c, tg)| {
        assert_eq!(c, tg, "a case is the target VA the register holds");
        c
    }).collect();
    assert_eq!(cases, sch_want(&b, o), "{o:?}");
    for tg in &cases {
        assert!(a.insns.contains_key(tg), "target {tg:#x} not in the closure");
    }
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert_eq!(a.closure.indirect.by_kind["jmp:msvc_table"], 1);
    assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 0);
    (b, a)
}

fn sch_rejected(o: &Sch, why: &str) -> (Built, fe_decode::Analysis) {
    let (b, a) = sch_run(o);
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?} must not be resolved: {:?}", a.closure.msvc_rejected);
    assert!(!a.jumptabs.contains_key(&jmp_va(&b)), "{o:?}");
    let r: Vec<&(u64, String)> = a.closure.msvc_rejected.iter().filter(|(v, _)| *v == jmp_va(&b)).collect();
    assert_eq!(r.len(), 1, "{o:?}: {:?}", a.closure.msvc_rejected);
    assert!(r[0].1.contains(why), "{o:?}: wanted {why:?}, got {:?}", r[0].1);
    assert_eq!(a.resolved[&jmp_va(&b)], Resolved::Unresolved);
    assert!(a.closure.jumptab_rejected.is_empty(), "{:?}", a.closure.jumptab_rejected);
    (b, a)
}

fn sch_origin(a: &fe_decode::Analysis, b: &Built) -> (u64, u64, u64, u64, u64, Option<u64>, Option<(u64, u64)>) {
    match a.jumptabs[&jmp_va(b)].origin {
        fe_decode::Origin::Msvc { n, from, guard_ja, guard_cmp, swapped, entry2, .. } => (n, from, guard_ja, guard_cmp, 0, swapped, entry2),
        other => panic!("not an MSVC table: {other:?}"),
    }
}

#[test]
fn scheduled_the_shape_of_freetype_ft_stream_readfields_is_resolved_with_two_guards() {
    // freetype f_4b166e0, site 0x4b1674d: `cmp eax,0x15; mov [rsp+..],r14; ja def; lea r14,[ImageBase]; nop; nop; cdqe; mov ecx,[r14+rax*4+T];
    // add rcx,r14; jmp rcx`, and at the bottom of the loop `cmp eax,0x15; jbe` to the `cdqe`: 22 entries, two ways into the idiom
    let o = Sch::real();
    let (b, a) = sch_resolved(&o);
    assert_eq!(b.entries, 22);
    assert_eq!(a.jumptabs[&jmp_va(&b)].cases.len(), 3);
    let (n, from, ja, cmp, _, swapped, entry2) = sch_origin(&a, &b);
    assert_eq!(n, 21);
    // the guard in front of the idiom: the `cmp`, the `ja` (not taken), the table block starts right after it with the `lea`
    assert_eq!(cmp, IB + b.labels["guard"] as u64);
    assert_eq!(ja, IB + b.labels["br"] as u64);
    assert_eq!(from, IB + b.labels["tbl"] as u64);
    assert_eq!(swapped, Some(from), "the run that only the guards may enter starts at the lea");
    // the second guard: the `jbe` of the loop block, landing on the `cdqe` (the aligned head of the loop)
    assert_eq!(entry2, Some((IB + b.labels["br2"] as u64, IB + b.labels["head"] as u64)));
    assert!(IB + b.labels["head"] as u64 > from);
}

#[test]
fn scheduled_n_is_the_largest_bound_of_the_guards() {
    for n2 in [3u32, 21, 30] {
        let o = Sch { n2, ..Sch::real() };
        let (b, a) = sch_resolved(&o);
        let (n, ..) = sch_origin(&a, &b);
        assert_eq!(n, 21u32.max(n2) as u64, "n2 = {n2}");
        assert_eq!(b.entries, 22usize.max(n2 as usize + 1), "n2 = {n2}");
    }
    // `jb head` with the bound N + 1: taken means I <= N
    let o = Sch { second_op: 0x72, n2: 22, ..Sch::real() };
    let (b, a) = sch_resolved(&o);
    assert_eq!(sch_origin(&a, &b).0, 21);
}

#[test]
fn scheduled_one_guard_with_nothing_but_a_sign_extension_in_the_gap() {
    let (b, a) = sch_resolved(&Sch::simple());
    let (n, from, ja, cmp, _, swapped, entry2) = sch_origin(&a, &b);
    assert_eq!((n, entry2), (5, None));
    assert_eq!(cmp, IB + b.labels["guard"] as u64);
    assert_eq!(ja, IB + b.labels["br"] as u64);
    assert_eq!((from, swapped), (IB + b.labels["tbl"] as u64, Some(from)));
    // the same with the idiom on the taken side of `cmp; jbe`
    sch_resolved(&Sch { taken: true, ..Sch::simple() });
    // two-stage
    sch_resolved(&Sch { two_stage: true, ..Sch::simple() });
    sch_resolved(&Sch { two_stage: true, ..Sch::real() });
    // a plain `cmp eax,N; ja; idiom` is still read by the standard form (decisions 0089 and 0093), not by this one
    let o = Sch { q: Vec::new(), p: Vec::new(), head: None, ..Sch::simple() };
    let (b, a) = sch_run(&o);
    assert_eq!(a.closure.msvc_tables, 1, "{:?}", a.closure.msvc_rejected);
    let fe_decode::Origin::Msvc { swapped, entry2, .. } = a.jumptabs[&jmp_va(&b)].origin else { panic!() };
    assert_eq!((swapped, entry2), (None, None));
}

#[test]
fn scheduled_stores_leas_and_nops_may_stand_in_either_gap() {
    let store = MOV_MEM_RSP_M16_RDX.to_vec();
    for (p, q) in [
        // a store, a copy to another register, between the `cmp` and the branch
        (vec![store.clone()], vec![SchQ::Lea, SchQ::Raw(CDQE.to_vec())]),
        (vec![store.clone(), MOV_RCX_RAX.to_vec(), store.clone()], vec![SchQ::Lea, SchQ::Raw(CDQE.to_vec())]),
        // `lea` of other registers (r13, r15; B is r14) and a `cdqe` (freetype 0x4b2044e is the 6-instruction gap `lea rbp; lea r13; lea r14;
        // lea r15; lea r8; cdqe`)
        (
            Vec::new(),
            vec![SchQ::Lea, SchQ::Raw(vec![0x4c, 0x8d, 0x2d, 0, 0, 0, 0]), SchQ::Raw(vec![0x4c, 0x8d, 0x3d, 0, 0, 0, 0]), SchQ::Raw(CDQE.to_vec())],
        ),
        // a store after the `cdqe` (freetype 0x4b1a9cc), an instruction that writes flags and another register (freetype 0x4b1f91d: `xor edi,edi`)
        (Vec::new(), vec![SchQ::Lea, SchQ::Raw(CDQE.to_vec()), SchQ::Raw(store.clone())]),
        (Vec::new(), vec![SchQ::Lea, SchQ::Raw(XOR_R9D_R9D.to_vec()), SchQ::Raw(NOP_3.to_vec()), SchQ::Raw(NOP_4.to_vec()), SchQ::Raw(CDQE.to_vec())]),
    ] {
        let o = Sch { entry_lea: false, p, q, head: None, t2: SchT2::Plain, ..Sch::simple() };
        sch_resolved(&o);
        sch_resolved(&Sch { two_stage: true, ..o });
    }
}

#[test]
fn scheduled_the_checked_register_may_reach_the_index_through_a_sign_extension() {
    // `cmp eax,N; ja; cdqe` and `cmp rax,N; ja` (no extension)
    sch_resolved(&Sch { cmp: SchCmp::Eax, ..Sch::simple() });
    sch_resolved(&Sch { cmp: SchCmp::Rax, q: Vec::new(), ..Sch::simple() });
    // `cmp eax,N; ja; movsxd rax,eax`
    sch_resolved(&Sch { q: vec![SchQ::Raw(MOVSXD_RAX_EAX.to_vec())], ..Sch::simple() });
    // `cmp edx,N; ja; movsxd rax,edx; mov ecx,[B+rax*4+T]`: the register that is compared is not the one that indexes (freetype 0x4b2f423, 0x4b4f316)
    let alias = Sch { cmp: SchCmp::Edx, q: vec![SchQ::Raw(MOVSXD_RAX_EDX.to_vec())], ..Sch::simple() };
    sch_resolved(&alias);
    sch_resolved(&Sch { two_stage: true, ..alias.clone() });
    sch_resolved(&Sch { q: vec![SchQ::Lea, SchQ::Raw(NOP_3.to_vec()), SchQ::Raw(MOVSXD_RAX_EDX.to_vec())], entry_lea: false, ..alias });
}

#[test]
fn scheduled_a_write_to_the_checked_register_that_is_not_a_sign_extension_stops_the_derivation() {
    let why = "is not a sign extension of the checked index";
    // between the branch and the idiom: a copy, an offset, a load of a different value
    for bytes in [MOV_EAX_EDX, MOV_RAX_RDX, LEA_EAX_EAX_1] {
        sch_rejected(&Sch { q: vec![SchQ::Raw(bytes.to_vec()), SchQ::Raw(CDQE.to_vec())], ..Sch::simple() }, why);
        sch_rejected(&Sch { q: vec![SchQ::Raw(CDQE.to_vec()), SchQ::Raw(bytes.to_vec())], ..Sch::simple() }, why);
    }
    // the compare is on edx and a `mov rax,rdx` copies it: a copy by `mov` is not a sign extension (decision 0109 keeps the rule narrow)
    sch_rejected(&Sch { cmp: SchCmp::Edx, q: vec![SchQ::Raw(MOV_RAX_RDX.to_vec())], ..Sch::simple() }, why);
    // the sign extension of another register than the compared one
    sch_rejected(&Sch { cmp: SchCmp::Edx, q: vec![SchQ::Raw(MOVSXD_RAX_EAX.to_vec())], ..Sch::simple() }, "scheduled form, decision 0109");
    // between the `cmp` and the branch
    sch_rejected(&Sch { p: vec![MOV_EAX_EDX.to_vec()], ..Sch::simple() }, "written at");
    // a write of the flags between the `cmp` and the branch: the branch does not test the `cmp`
    sch_rejected(&Sch { p: vec![XOR_R9D_R9D.to_vec()], ..Sch::simple() }, "are last written by");
}

#[test]
fn scheduled_a_call_in_the_gap_or_between_the_cmp_and_the_branch_is_rejected() {
    // a callee may write the checked register (rax holds a return value), so the gap between the branch and the idiom and the run
    // between the `cmp` and the branch hold no call (the walk back stops at it); the call is to a function outside the switch
    let gap = "falls into the idiom and is not a conditional branch";
    sch_rejected(&Sch { q: vec![SchQ::Call, SchQ::Raw(CDQE.to_vec())], ..Sch::simple() }, gap);
    sch_rejected(&Sch { q: vec![SchQ::Raw(CDQE.to_vec()), SchQ::Call], ..Sch::simple() }, gap);
    sch_rejected(&Sch { q: vec![SchQ::Lea, SchQ::Call, SchQ::Raw(CDQE.to_vec())], entry_lea: false, ..Sch::simple() }, gap);
    sch_rejected(&Sch { p_call: true, ..Sch::simple() }, "in its own block");
    sch_rejected(&Sch { p_call: true, p: Vec::new(), ..Sch::simple() }, "in its own block");
}

#[test]
fn scheduled_the_guard_must_be_an_unsigned_less_or_equal_side_and_a_compare_with_an_immediate() {
    // the second guard on the wrong side (`ja head`, `jae head`): taken means I > N
    for op in [0x77u8, 0x73] {
        sch_rejected(&Sch { second_op: op, ..Sch::real() }, "is not an unsigned I <= N side");
    }
    // the second guard's branch tests flags that nothing in its block set (a `call` in between)
    sch_rejected(&Sch { t2: SchT2::NoCmp, ..Sch::real() }, "sets the flags of the");
    // an unconditional way in
    sch_rejected(&Sch { t2: SchT2::JmpHead, ..Sch::real() }, "is not a conditional branch");
    // no guard at all: the walk back from the idiom reaches the function entry
    sch_rejected(&Sch { unguarded: true, taken: false, ..Sch::simple() }, "reaches the function entry");
    // block `t0` jumps into the gap
    sch_rejected(&Sch { t0_jump: Some(0), ..Sch::simple() }, "is not a conditional branch");
    sch_rejected(
        &Sch { t0_jump: Some(1), q: vec![SchQ::Lea, SchQ::Raw(NOP_3.to_vec()), SchQ::Raw(CDQE.to_vec())], entry_lea: false, ..Sch::simple() },
        "is not a conditional branch",
    );
}

#[test]
fn scheduled_at_most_two_guards() {
    // `t1` loops back to the head too: three ways into the idiom
    sch_rejected(&Sch { t1_loop: true, ..Sch::real() }, "3 ways into the idiom; at most 2 are read");
}

#[test]
fn scheduled_the_base_proof_and_the_targets_are_checked_as_before() {
    // no write to B at all, the `lea` of another address, a write that reaches the idiom through the loop's second way in
    sch_rejected(&Sch { q: vec![SchQ::Raw(CDQE.to_vec())], entry_lea: false, ..Sch::simple() }, "is never written");
    sch_rejected(&Sch { q: vec![SchQ::LeaAt(0x10), SchQ::Raw(CDQE.to_vec())], entry_lea: false, ..Sch::simple() }, "is not `lea");
    sch_rejected(&Sch { loop_clobbers_base: true, ..Sch::real() }, "decision 0094");
    // the targets: another function, not executable, not an instruction start
    sch_rejected(&Sch { bad: Bad::OtherFunction, ..Sch::real() }, "target outside the function");
    sch_rejected(&Sch { bad: Bad::NotExecutable, ..Sch::real() }, "not executable");
    sch_rejected(&Sch { bad: Bad::Misaligned, ..Sch::simple() }, "not an instruction start");
}

#[test]
fn scheduled_a_site_that_stays_rejected_for_lack_of_n_reports_both_reasons() {
    // the reason of the standard form first (what tests and logs of decisions 0089-0095 show), then the reason of the second reading
    let (_, a) = sch_run(&Sch { p: vec![MOV_EAX_EDX.to_vec()], ..Sch::simple() });
    let m = &a.closure.msvc_rejected[0].1;
    assert!(m.starts_with("N not derived: no `cmp idx,imm` right before the `ja`"), "{m}");
    assert!(m.contains(" [N not derived (scheduled form, decision 0109): "), "{m}");
}

#[test]
fn scheduled_landings_and_block_starts_inside_the_run_are_rejected_except_the_second_guards() {
    // `landings_in_swapped_idioms` with the two tuples of one jump: the second guard's branch is allowed to land at its target, nothing else
    let r = [(0x1000u64, 0x1060u64, 0x0ff0u64, 0x1000u64), (0x1000, 0x1060, 0x1080, 0x1030)];
    assert!(jumptab::landings_in_swapped_idioms(&r, [(0x1080u64, 0x1030u64)].into_iter()).is_empty());
    assert!(jumptab::landings_in_swapped_idioms(&r, [(0x0ff0u64, 0x1000u64)].into_iter()).is_empty());
    for (src, tgt) in [(0x9000u64, 0x1030u64), (0x1080, 0x1034), (0x1080, 0x1000), (0x0ff0, 0x1030), (0x9000, 0x1060)] {
        assert!(jumptab::landings_in_swapped_idioms(&r, [(src, tgt)].into_iter()).contains_key(&0x1060), "{src:#x} -> {tgt:#x}");
    }
    // one tuple per jump: the rule of decision 0095
    assert!(!jumptab::landings_in_swapped_idioms(&r[..1], [(0x1080u64, 0x1030u64)].into_iter()).is_empty());
    // `verify_msvc_swapped_entries`: a block start at the second guard's landing is accepted only when it is named
    let o = Sch::real();
    let (b, a) = sch_resolved(&o);
    let img = b.img.build();
    let pe = Pe::parse(&img).unwrap();
    let text = fe_decode::Text::from_pe(&pe).unwrap();
    let fm = jumptab::FuncMap::from_pe(&pe).unwrap();
    let (primary, pieces) = fm.function_at(AT).unwrap();
    let sw = jumptab::Sweep::build(&pe, &text, pieces).unwrap();
    let entry = pe.rva_to_va(primary);
    let (_, from, guard_ja, guard_cmp, _, swapped, entry2) = sch_origin(&a, &b);
    let (lo, head) = (swapped.unwrap(), entry2.unwrap().1);
    let jmp = jmp_va(&b);
    let verify = |leaders: &[u64], extra: Option<u64>| {
        let set: std::collections::HashSet<u64> = leaders.iter().copied().collect();
        jumptab::verify_msvc_swapped_entries(jmp, from, lo, (guard_ja, guard_cmp), extra, entry, &a.insns, &set, &a.functions, &sw)
    };
    assert_eq!(verify(&[entry, from], None), Ok(()));
    assert_eq!(verify(&[entry, from, head], Some(head)), Ok(()));
    let e = verify(&[entry, from, head], None).unwrap_err();
    assert!(e.contains(&format!("a block starts at {head:#x}, inside the swapped switch idiom")), "{e}");
    // any other block start is still an error
    for at in [IB + b.labels["load"] as u64, IB + b.labels["add"] as u64, jmp] {
        let e = verify(&[entry, from, head, at], Some(head)).unwrap_err();
        assert!(e.contains(&format!("a block starts at {at:#x}, inside the swapped switch idiom")), "{at:#x}: {e}");
    }
}

#[test]
fn scheduled_a_table_in_a_writable_section_is_rejected_because_the_gap_may_hold_stores() {
    // the gap of a scheduled idiom may hold stores (decision 0109), so the tables it reads from the file image must lie in sections
    // without IMAGE_SCN_MEM_WRITE (the index byte table is looked at first, then the RVA table). The same images with a read-only
    // `.rdata` are resolved: `sch_resolved` below is the control
    for (o, which) in [
        (Sch::real(), "RVA table"),
        (Sch::simple(), "RVA table"),
        (Sch { taken: true, ..Sch::simple() }, "RVA table"),
        (Sch { two_stage: true, ..Sch::simple() }, "index byte table"),
        (Sch { two_stage: true, ..Sch::real() }, "index byte table"),
    ] {
        let mut built = build_sched(AT, &o);
        built.img.rdata_writable = true;
        let img = built.img.build();
        let pe = Pe::parse(&img).unwrap();
        assert!(pe.sections.iter().any(|s| s.name == ".rdata" && s.is_writable()), "{o:?}");
        let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
        assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (0, 0), "{o:?} must not be resolved");
        assert!(!a.jumptabs.contains_key(&jmp_va(&built)), "{o:?}");
        let r: Vec<&(u64, String)> = a.closure.msvc_rejected.iter().filter(|(v, _)| *v == jmp_va(&built)).collect();
        assert_eq!(r.len(), 1, "{o:?}: {:?}", a.closure.msvc_rejected);
        assert!(
            r[0].1.contains(&format!("the {which} at")) && r[0].1.contains("lies in the writable section `.rdata`"),
            "{o:?}: wanted the {which} in a writable section, got {:?}",
            r[0].1
        );
        assert_eq!(a.resolved[&jmp_va(&built)], Resolved::Unresolved);
        assert_eq!(a.closure.indirect.by_kind["jmp:unresolved"], 1, "{o:?}");
        sch_resolved(&o);
    }
}

#[test]
fn scheduled_the_writable_check_is_for_the_scheduled_form_only() {
    // decision 0109 adds the check where it allows stores in the gap. A standard-form site (decision 0089) in the same writable `.rdata`
    // is resolved as before, also when it is the plain `cmp; ja; idiom` of the scheduled builder
    for o in [Sw::c1(), Sw::c2()] {
        let mut built = build_switch(AT, &o);
        built.img.rdata_writable = true;
        let img = built.img.build();
        let pe = Pe::parse(&img).unwrap();
        let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
        assert!(a.closure.msvc_rejected.is_empty(), "{o:?}: {:?}", a.closure.msvc_rejected);
        assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (1, 1), "{o:?}");
        let fe_decode::Origin::Msvc { swapped, entry2, .. } = a.jumptabs[&jmp_va(&built)].origin else { panic!() };
        assert_eq!((swapped, entry2), (None, None));
    }
    let o = Sch { q: Vec::new(), p: Vec::new(), head: None, ..Sch::simple() };
    let mut built = build_sched(AT, &o);
    built.img.rdata_writable = true;
    let img = built.img.build();
    let pe = Pe::parse(&img).unwrap();
    let a = analyze_with_options(&pe, &[], jumptab::FLENGINE, &Options::default()).unwrap();
    assert_eq!((a.closure.jumptabs, a.closure.msvc_tables), (1, 1), "{:?}", a.closure.msvc_rejected);
}

// ---- decision 0109 on the real image: freetype_x64.dll rebased to 0x4b00000 (a measurement base) ----

/// `(site, N, distinct targets)` of the eleven sites that decision 0109 resolves in freetype, from an independent scan with capstone
/// (lift/out-git/t1-results/freetype-msvc-rejected.tsv). Predicted, not yet run with this implementation.
const FT_SCHEDULED_0109: [(u64, u64, usize); 11] = [
    (0x4b07cce, 45, 4),
    (0x4b0ef6a, 37, 35),
    (0x4b1674d, 21, 10),
    (0x4b1a9cc, 5, 4),
    (0x4b1f22d, 9, 10),
    (0x4b1f91d, 9, 10),
    (0x4b2044e, 13, 14),
    (0x4b31175, 55, 4),
    (0x4b46d08, 143, 114),
    (0x4b4f316, 45, 46),
    (0x4b58c24, 7, 8),
];

/// The five rejections that remain: the base of 0x4b2f423 (`r10`, caller-saved, a call between its `lea` and the jump), of 0x4b2ffb4 and
/// 0x4b30485 (`r10`/`r8` are written by a non-`lea` that reaches the other indirect jump of the function), and the two jumps that are
/// outside every `.pdata` function (0x4b3dcfe, 0x4b434ce).
const FT_STILL_REJECTED: [u64; 5] = [0x4b2f423, 0x4b2ffb4, 0x4b30485, 0x4b3dcfe, 0x4b434ce];

#[test]
fn freetype_the_stop_point_of_the_ft_stream_readfields_switch_and_ten_more_scheduled_sites_are_resolved() {
    let Some(data) = read_if_present("FREETYPE_PE", "../../out-wallQ/freetype/freetype_x64.4b00000.dll") else { return };
    let pe = Pe::parse(&data).unwrap();
    assert_eq!(pe.optional.image_base, 0x4b00000);
    let mut seeds = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    let file = std::fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/seeds/ft-seeds-4b00000.txt")).unwrap();
    seeds.extend(file.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()));
    let a = analyze_with_options(&pe, &seeds, jumptab::FLENGINE, &Options::default()).unwrap();
    assert_eq!(a.closure.decode_failures, 0);
    // before decision 0109: 17 resolved (all of the standard form) and 16 rejected, 15 of them for lack of an N (53 register jumps)
    for &(site, n, distinct) in &FT_SCHEDULED_0109 {
        let t = a.jumptabs.get(&site).unwrap_or_else(|| panic!("{site:#x} not resolved: {:?}", a.closure.msvc_rejected.iter().find(|(v, _)| *v == site)));
        let fe_decode::Origin::Msvc { n: got_n, swapped, .. } = t.origin else { panic!("{site:#x}: {:?}", t.origin) };
        assert_eq!((got_n, t.cases.len()), (n, distinct), "{site:#x}");
        assert!(swapped.is_some(), "{site:#x}: the guard is read from the CFG");
        for &(c, tg) in &t.cases {
            assert_eq!(c, tg);
            assert!(a.insns.contains_key(&tg), "{site:#x}: target {tg:#x} is not in the closure");
        }
    }
    // the stop point: the loop-rotated switch, two ways into the idiom, `cdqe` at 0x4b16740 is the head of the loop
    let fe_decode::Origin::Msvc { n, entry2, from, guard_ja, guard_cmp, function, .. } = a.jumptabs[&0x4b1674d].origin else { panic!() };
    assert_eq!((n, function), (21, 0x166e0));
    assert_eq!((guard_cmp, guard_ja, from), (0x4b16724, 0x4b1672c, 0x4b16732));
    assert_eq!(entry2, Some((0x4b168df, 0x4b16740)));
    // the four loop-rotated sites have the second guard, the others have one
    for (site, second) in [(0x4b1674du64, 0x4b16740u64), (0x4b1f22d, 0x4b1f220), (0x4b1f91d, 0x4b1f910), (0x4b2044e, 0x4b20442)] {
        let fe_decode::Origin::Msvc { entry2, .. } = a.jumptabs[&site].origin else { panic!() };
        assert_eq!(entry2.map(|e| e.1), Some(second), "{site:#x}");
    }
    // the 17 sites of the standard form keep their tables, not read from the CFG
    for &site in &[0x4b0b5fau64, 0x4b0eecd, 0x4b0f19e, 0x4b0f23f, 0x4b1a6dc, 0x4b25718, 0x4b27339, 0x4b2c591, 0x4b2d871, 0x4b30dba, 0x4b31309, 0x4b3147f, 0x4b3ebdc, 0x4b3f150, 0x4b4d657, 0x4b5bdf9, 0x4b6009e] {
        let fe_decode::Origin::Msvc { swapped, entry2, .. } = a.jumptabs[&site].origin else { panic!() };
        assert_eq!((swapped, entry2), (None, None), "{site:#x}");
    }
    // the rest stays rejected, for the reasons above
    let rej: BTreeSet<u64> = a.closure.msvc_rejected.iter().map(|(v, _)| *v).collect();
    for va in FT_STILL_REJECTED {
        assert!(rej.contains(&va), "{va:#x}: {:?}", a.closure.msvc_rejected);
        assert!(!a.jumptabs.contains_key(&va), "{va:#x}");
    }
    // measured: 29 = 17 + the 11 above + 0x4b58f68, a standard-form switch that only the new code reaches (the table of 0x4b58c24 lands in
    // its function); 5 rejected
    assert_eq!((a.closure.msvc_tables, a.closure.msvc_rejected.len()), (29, 5), "{:?}", a.closure.msvc_rejected);
    let t = &a.jumptabs[&0x4b58f68];
    assert!(matches!(t.origin, fe_decode::Origin::Msvc { n: 55, swapped: None, entry2: None, .. }), "{:?}", t.origin);
    assert_eq!(t.cases.len(), 15);
    let by_va: std::collections::BTreeMap<u64, &str> = a.closure.msvc_rejected.iter().map(|(v, m)| (*v, m.as_str())).collect();
    assert!(by_va[&0x4b2f423].contains("caller-saved"), "{}", by_va[&0x4b2f423]);
    assert!(by_va[&0x4b434ce].contains("not inside a .pdata function"), "{}", by_va[&0x4b434ce]);
}
