//! Lifting of indirect `call` / `jmp` whose target `fe-decode` resolved to code.
//!
//! A resolved call becomes a direct call to `f_<target>()`, and a resolved jmp
//! becomes `Ctl::Jmp(target)`. A `call` through an IAT slot whose import has a row in
//! `imports_table` calls that row's `fle_imp_*` (decision 0077). Other IAT slots (no code in
//! the image), and unresolved targets, stay unsupported. Each is counted under its own reason.

use crate::{Ctl, Lifted, RSP};
use fe_decode::{How, Resolved};
use iced_x86::Instruction;
use std::collections::{BTreeMap, BTreeSet};

fn target(va: u64, resolved: &BTreeMap<u64, Resolved>) -> Result<(u64, How), String> {
    match resolved.get(&va) {
        Some(Resolved::Code { target, how }) => Ok((*target, *how)),
        Some(Resolved::Import { .. }) => Err("indirect through IAT slot (not lifted)".into()),
        Some(Resolved::Unresolved) | None => Err("indirect target unresolved".into()),
    }
}

fn handler(kind: &str, how: How) -> &'static str {
    match (kind, how) {
        ("call", How::RipTable) => "call_ind:rip_table",
        ("call", How::RegConst) => "call_ind:reg_const",
        ("call", How::RegTable) => "call_ind:reg_table",
        ("jmp", How::RipTable) => "jmp_ind:rip_table",
        ("jmp", How::RegConst) => "jmp_ind:reg_const",
        _ => "jmp_ind:reg_table",
    }
}

pub fn call(i: &Instruction, va: u64, entries: &BTreeSet<u64>, resolved: &BTreeMap<u64, Resolved>) -> Result<Lifted, String> {
    if let Some(Resolved::Import { name }) = resolved.get(&va) {
        if let Some((f, h)) = crate::imports_table::lookup_call(name) {
            let next = va + i.len() as u64;
            return Ok(Lifted {
                body: format!("{{ void {f}(void); R[{RSP}] -= 8; fle_st(R[{RSP}], 8, UINT64_C({next:#x})); {f}(); R[{RSP}] += 8; }}"),
                ctl: Ctl::Seq,
                handler: h,
            });
        }
    }
    let (t, how) = target(va, resolved)?;
    if !entries.contains(&t) {
        return Err("indirect call target is not a function entry".into());
    }
    let next = va + i.len() as u64;
    Ok(Lifted {
        body: format!("{{ R[{RSP}] -= 8; fle_st(R[{RSP}], 8, UINT64_C({next:#x})); f_{t:x}(); R[{RSP}] += 8; }}"),
        ctl: Ctl::Seq,
        handler: handler("call", how),
    })
}

pub fn jmp(va: u64, resolved: &BTreeMap<u64, Resolved>) -> Result<Lifted, String> {
    let (t, how) = target(va, resolved)?;
    Ok(Lifted { body: String::new(), ctl: Ctl::Jmp(t), handler: handler("jmp", how) })
}

#[cfg(test)]
mod tests {
    use crate::generate;

    const IB: u64 = 0x1_4000_0000;
    const LFANEW: usize = 0x80;
    const OPT: usize = LFANEW + 24;
    const SEC: usize = OPT + 240;
    const TEXT_RVA: u32 = 0x1000;
    const TEXT_RAW: usize = 0x400;
    const DATA_RVA: u32 = 0x2000;
    const DATA_RAW: usize = 0x800;
    const FILE_LEN: usize = 0x1000;

    fn w16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn w32(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn w64(b: &mut [u8], o: usize, v: u64) {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn data_at(rva: u32) -> usize {
        DATA_RAW + (rva - DATA_RVA) as usize
    }
    fn rel(next: u32, target: u32) -> [u8; 4] {
        ((target as i64 - next as i64) as i32).to_le_bytes()
    }

    /// Same layout as the fe-decode test: `.text` at 0x1000, `.data` at 0x2000.
    /// `call [rip+0x2500]` (rip table -> 0x1040), `call [rip+0x2200]` (IAT),
    /// `jmp [rip+0x2508]` (rip table -> 0x1030), `call rbx` (unresolved).
    fn build() -> Vec<u8> {
        let mut b = vec![0u8; FILE_LEN];
        w16(&mut b, 0, 0x5a4d);
        w32(&mut b, 0x3c, LFANEW as u32);
        w32(&mut b, LFANEW, 0x4550);
        let f = LFANEW + 4;
        w16(&mut b, f, 0x8664);
        w16(&mut b, f + 2, 2);
        w16(&mut b, f + 16, 240);
        w16(&mut b, f + 18, 0x2022);
        let o = OPT;
        w16(&mut b, o, 0x20b);
        w32(&mut b, o + 16, TEXT_RVA);
        w64(&mut b, o + 24, IB);
        w32(&mut b, o + 32, 0x1000);
        w32(&mut b, o + 36, 0x200);
        w32(&mut b, o + 56, 0x3000);
        w32(&mut b, o + 60, 0x400);
        w16(&mut b, o + 68, 3);
        w32(&mut b, o + 108, 16);
        w32(&mut b, o + 120, DATA_RVA);
        w32(&mut b, o + 124, 40);

        b[SEC..SEC + 5].copy_from_slice(b".text");
        w32(&mut b, SEC + 8, 0x400);
        w32(&mut b, SEC + 12, TEXT_RVA);
        w32(&mut b, SEC + 16, 0x400);
        w32(&mut b, SEC + 20, TEXT_RAW as u32);
        w32(&mut b, SEC + 36, 0x6000_0020);
        let s1 = SEC + 40;
        b[s1..s1 + 5].copy_from_slice(b".data");
        w32(&mut b, s1 + 8, 0x800);
        w32(&mut b, s1 + 12, DATA_RVA);
        w32(&mut b, s1 + 16, 0x800);
        w32(&mut b, s1 + 20, DATA_RAW as u32);
        w32(&mut b, s1 + 36, 0xc000_0040);

        let d = data_at(DATA_RVA);
        w32(&mut b, d, 0x2100);
        w32(&mut b, d + 12, 0x2300);
        w32(&mut b, d + 16, 0x2200);
        w64(&mut b, data_at(0x2100), 0x2400);
        w64(&mut b, data_at(0x2200), 0x2400);
        b[data_at(0x2300)..data_at(0x2300) + 6].copy_from_slice(b"k.dll\0");
        b[data_at(0x2402)..data_at(0x2402) + 3].copy_from_slice(b"Fn\0");
        w64(&mut b, data_at(0x2500), IB + 0x1040);
        w64(&mut b, data_at(0x2508), IB + 0x1030);

        let t = TEXT_RAW;
        let code: &[(u32, Vec<u8>)] = &[
            (0x1000, [&[0xff, 0x15][..], &rel(0x1006, 0x2500)].concat()),
            (0x1006, [&[0xff, 0x15][..], &rel(0x100c, 0x2200)].concat()),
            (0x100c, [&[0xff, 0x25][..], &rel(0x1012, 0x2508)].concat()),
            (0x1030, vec![0xc3]),
            (0x1040, vec![0xc3]),
        ];
        for (rva, bytes) in code {
            let o = t + (rva - TEXT_RVA) as usize;
            b[o..o + bytes.len()].copy_from_slice(bytes);
        }
        b
    }

    #[test]
    fn rip_table_call_and_jmp_are_direct_and_iat_stays_unsupported() {
        let img = build();
        let mut out = Vec::new();
        let st = generate(&img, &mut out).unwrap();
        let c = String::from_utf8(out).unwrap();

        assert_eq!(st.handled["call_ind:rip_table"], 1);
        assert_eq!(st.handled["jmp_ind:rip_table"], 1);
        assert!(c.contains(&format!("f_{:x}(); R[4] += 8;", IB + 0x1040)));
        assert!(c.contains(&format!("goto L_{:x};", IB + 0x1030)));
        assert!(c.contains(&format!("fle_unsupported(UINT64_C({:#x}));", IB + 0x1006)));
        assert_eq!(st.unsupported["Call: indirect through IAT slot (not lifted)"], 1);
        assert!(!st.unsupported.contains_key("Call: indirect target unresolved"));
    }
}
