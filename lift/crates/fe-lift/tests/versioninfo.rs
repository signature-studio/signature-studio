//! The VerifyVersionInfoW stop after W1 (docs/decisions/0039-versioninfo-stop.md).
//!
//! The guest function f_454030 (0x454030..0x454081) builds an OSVERSIONINFOEXW on its frame at [rbp+0x24], zeroes
//! 0x11c bytes with the memset at 0x40d210, stores wProductType = 1 at [rbp+0x13e] (= struct + 282) and calls
//! VerifyVersionInfoW (the IAT thunk at 0x423cc0) with dwTypeMask 0x80 and a VER_EQUAL product-type condition. Nothing in
//! it writes dwOSVersionInfoSize (struct + 0). The runtime's stop on that value is therefore the guest's own input, not a
//! lifter defect. These tests read the bytes of the real FLEngine_x64.dll ($FLENGINE_PE, else
//! <repo root>/binaries/win64/FLEngine_x64.dll; skipped when only the default is missing) and decode them independently of
//! the generator.
//!
//! The runtime side changed after this was written: decision 0040 lets a size-0 call through (provisional, from unofficial
//! sources, not checked on a real Windows); see `size_0_is_not_checked` and `undefined_cases_abort` in osver.rs. These
//! disassembly checks are unchanged: the guest still never stores the size.

use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register};
use std::path::PathBuf;

const IMAGE_BASE: u64 = 0x40_0000;

fn dll() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

fn file_offset(d: &[u8], va: u64) -> usize {
    let lf = u32::from_le_bytes(d[0x3c..0x40].try_into().unwrap()) as usize;
    let ns = u16::from_le_bytes(d[lf + 6..lf + 8].try_into().unwrap()) as usize;
    let osz = u16::from_le_bytes(d[lf + 20..lf + 22].try_into().unwrap()) as usize;
    let rva = (va - IMAGE_BASE) as u32;
    for k in 0..ns {
        let s = lf + 24 + osz + 40 * k;
        let u = |o: usize| u32::from_le_bytes(d[s + o..s + o + 4].try_into().unwrap());
        let (vs, sva, rs, rp) = (u(8), u(12), u(16), u(20));
        if rva >= sva && rva < sva + vs.max(rs) {
            return (rp + (rva - sva)) as usize;
        }
    }
    panic!("va {va:#x} not in a section");
}

fn decode(d: &[u8], start: u64, end: u64) -> Vec<Instruction> {
    let o = file_offset(d, start);
    let bytes = &d[o..o + (end - start) as usize];
    let mut dec = Decoder::with_ip(64, bytes, start, DecoderOptions::NONE);
    let mut v = Vec::new();
    while dec.can_decode() {
        let i = dec.decode();
        assert!(!i.is_invalid(), "invalid instruction at {:#x}", i.ip());
        v.push(i);
    }
    v
}

fn ends_at(v: &[Instruction], end: u64) {
    let last = v.last().unwrap();
    assert_eq!(last.next_ip(), end, "decoding did not end on the function boundary");
}

/// Memory writes whose destination is `[rbp + disp]` with `disp` in `lo..hi`.
fn rbp_writes(v: &[Instruction], lo: i64, hi: i64) -> Vec<(u64, Mnemonic)> {
    v.iter()
        .filter(|i| {
            i.mnemonic() != Mnemonic::Lea
                && i.mnemonic() != Mnemonic::Cmp
                && i.mnemonic() != Mnemonic::Test
                && i.op0_kind() == OpKind::Memory
                && i.memory_base() == Register::RBP
                && i.memory_index() == Register::None
                && (lo..hi).contains(&(i.memory_displacement64() as i64))
        })
        .map(|i| (i.ip(), i.mnemonic()))
        .collect()
}

/// f_454030: the structure at [rbp+0x24] is memset to 0x11c bytes, byte 0x13e (struct + 282) is set to 1, and no
/// instruction writes dwOSVersionInfoSize (struct + 0 = [rbp+0x24..0x28]).
#[test]
fn f_454030_never_stores_the_size_member() {
    let Some(d) = dll() else { return };
    let v = decode(&d, 0x454030, 0x454081);
    ends_at(&v, 0x454081);

    // The call at 0x45406e is the VerifyVersionInfoW thunk, its first argument is the structure.
    let call = v.iter().find(|i| i.ip() == 0x45406e).unwrap();
    assert_eq!(call.mnemonic(), Mnemonic::Call);
    assert_eq!(call.near_branch_target(), 0x423cc0);
    let lea = v.iter().find(|i| i.ip() == 0x454062).unwrap();
    assert_eq!((lea.mnemonic(), lea.op0_register(), lea.memory_base(), lea.memory_displacement64()), (Mnemonic::Lea, Register::RCX, Register::RBP, 0x24));
    let mask = v.iter().find(|i| i.ip() == 0x454066).unwrap();
    assert_eq!((mask.mnemonic(), mask.op0_register(), mask.immediate32()), (Mnemonic::Mov, Register::EDX, 0x80));

    // memset(struct, 0x11c, 0) at 0x45403b..0x454047 (0x40d210 is the memset: rcx = dst, rdx = count, r8b = value).
    let lea0 = v.iter().find(|i| i.ip() == 0x45403b).unwrap();
    assert_eq!((lea0.op0_register(), lea0.memory_base(), lea0.memory_displacement64()), (Register::RCX, Register::RBP, 0x24));
    let n = v.iter().find(|i| i.ip() == 0x45403f).unwrap();
    assert_eq!((n.op0_register(), n.immediate32()), (Register::EDX, 0x11c));
    let c = v.iter().find(|i| i.ip() == 0x454047).unwrap();
    assert_eq!(c.near_branch_target(), 0x40d210);

    // wProductType = 1 at [rbp+0x13e] = struct + 0x11a (282).
    let pt = v.iter().find(|i| i.ip() == 0x45404c).unwrap();
    assert_eq!((pt.mnemonic(), pt.memory_displacement64(), pt.immediate8()), (Mnemonic::Mov, 0x13e, 1));
    assert_eq!(0x13e - 0x24, 282);

    // Every write to the frame at [rbp+disp]: only the product type. Nothing lands in struct+0..struct+4.
    let w = rbp_writes(&v, 0, 0x200);
    assert_eq!(w, vec![(0x45404c, Mnemonic::Mov)], "{w:x?}");
    assert!(rbp_writes(&v, 0x24, 0x28).is_empty());
}

/// The control: a different FLEngine function (f_426310, the version helper that calls with TypeMask 0x23) stores 0x11c
/// into its structure. The detector sees it, so the empty result above is a property of f_454030, not of the detector.
#[test]
fn f_426310_stores_the_size_member() {
    let Some(d) = dll() else { return };
    let v = decode(&d, 0x426310, 0x4263c7);
    ends_at(&v, 0x4263c7);
    let lea = v.iter().find(|i| i.ip() == 0x42633e).unwrap();
    assert_eq!((lea.op0_register(), lea.memory_base(), lea.memory_displacement64()), (Register::RCX, Register::RBP, 0x44));
    let w = rbp_writes(&v, 0x44, 0x48);
    assert_eq!(w.len(), 1, "{w:x?}");
    let st = v.iter().find(|i| i.ip() == w[0].0).unwrap();
    assert_eq!((st.mnemonic(), st.immediate32(), st.memory_displacement64()), (Mnemonic::Mov, 0x11c, 0x44));
}

/// f_454030's IAT thunk 0x423cc0 is `jmp [0x1971620]`, the VerifyVersionInfoW slot (0x423cd0 is the VerSetConditionMask slot).
#[test]
fn thunks_used_by_f_454030() {
    let Some(d) = dll() else { return };
    for (va, slot) in [(0x423cc0u64, 0x197_1620u64), (0x423cd0, 0x197_1658)] {
        let v = decode(&d, va, va + 6);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].mnemonic(), Mnemonic::Jmp);
        assert_eq!(v[0].memory_base(), Register::RIP);
        assert_eq!(v[0].memory_displacement64(), slot, "{va:#x}");
    }
}
