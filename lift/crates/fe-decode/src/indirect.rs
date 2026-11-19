//! Resolution of indirect `call` / `jmp` targets.
//!
//! Forms, tried in this order:
//! - IAT slot: `[rip+disp]` whose address is an import slot. Named, not followed
//!   (the callee has no code in this image).
//! - RIP table: `[rip+disp]` whose 8-byte file content is a VA inside `.text`.
//! - Register constant: `reg` or `[reg+disp]` where `reg` was set by the
//!   instruction immediately before the branch: `lea reg,[rip+X]`,
//!   `mov reg,imm`, or `mov reg,[rip+X]`.
//!
//! Anything else is `Unresolved`. The register forms are only tried when the
//! branch is not the first instruction of its block (`block != va`), because a
//! block start can be reached from other paths where the register is unknown.

use crate::Text;
use fe_pe::{ImportSymbol, Pe};
use iced_x86::{FlowControl, Instruction, Mnemonic, OpKind, Register};
use std::collections::BTreeMap;

#[path = "dataflow.rs"]
pub mod dataflow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    RipTable,
    RegConst,
    RegTable,
}

impl How {
    pub fn as_str(self) -> &'static str {
        match self {
            How::RipTable => "rip_table",
            How::RegConst => "reg_const",
            How::RegTable => "reg_table",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Code { target: u64, how: How },
    Import { name: String },
    Unresolved,
}

/// Counts per kind (`call:rip_table`, `jmp:iat`, `call:unresolved`, ...) and
/// named imports reached through an IAT slot.
#[derive(Debug, Default)]
pub struct Stats {
    pub by_kind: BTreeMap<String, usize>,
    pub imports: BTreeMap<String, usize>,
}

impl Stats {
    /// Moves one register-based resolution to `unresolved`, after the leader check.
    pub fn demote(&mut self, kind: &str, how: How) {
        if let Some(n) = self.by_kind.get_mut(&format!("{kind}:{}", how.as_str())) {
            *n -= 1;
        }
        *self.by_kind.entry(format!("{kind}:unresolved")).or_insert(0) += 1;
    }

    /// Moves one count from `from` to `to` (both `kind:how` keys).
    pub fn move_kind(&mut self, from: &str, to: &str) {
        if let Some(n) = self.by_kind.get_mut(from) {
            *n -= 1;
        }
        *self.by_kind.entry(to.to_string()).or_insert(0) += 1;
    }

    pub fn record(&mut self, kind: &str, r: &Resolved) {
        let key = match r {
            Resolved::Code { how, .. } => format!("{kind}:{}", how.as_str()),
            Resolved::Import { name } => {
                *self.imports.entry(name.clone()).or_insert(0) += 1;
                format!("{kind}:iat")
            }
            Resolved::Unresolved => format!("{kind}:unresolved"),
        };
        *self.by_kind.entry(key).or_insert(0) += 1;
    }
}

#[derive(Clone, Copy)]
enum Val {
    Const(u64),
    /// Holds the contents of this IAT slot.
    Import(u64),
}

pub struct Resolver<'r, 'a> {
    pe: &'r Pe<'a>,
    text: &'r Text<'a>,
    imports: BTreeMap<u64, String>,
    leaders: std::collections::BTreeSet<u64>,
}

impl<'r, 'a> Resolver<'r, 'a> {
    pub fn new(pe: &'r Pe<'a>, text: &'r Text<'a>) -> Result<Self, String> {
        let mut imports = BTreeMap::new();
        for imp in pe.imports().map_err(|e| format!("imports: {e}"))? {
            for e in &imp.entries {
                let sym = match &e.symbol {
                    ImportSymbol::Name { name, .. } => name.clone(),
                    ImportSymbol::Ordinal(o) => format!("#{o}"),
                };
                imports.insert(e.iat_va, format!("{}!{}", imp.dll, sym));
            }
        }
        let leaders = dataflow::leaders(text);
        Ok(Resolver { pe, text, imports, leaders })
    }

    fn read_u64(&self, va: u64) -> Option<u64> {
        let rva = self.pe.va_to_rva(va).ok()?;
        let b = self.pe.slice_at_rva(rva, 8).ok()?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Some(u64::from_le_bytes(a))
    }

    fn through_slot(&self, slot: u64, how: How) -> Resolved {
        if let Some(name) = self.imports.get(&slot) {
            return Resolved::Import { name: name.clone() };
        }
        match self.read_u64(slot) {
            Some(v) if self.text.contains(v) => Resolved::Code { target: v, how },
            _ => Resolved::Unresolved,
        }
    }

    /// The value that `prev` assigns to `reg`, if `prev` is a `lea`/`mov` of the
    /// recognised form and writes the full register.
    fn def_of(&self, p: &Instruction, reg: Register) -> Option<Val> {
        if p.flow_control() != FlowControl::Next || p.op_kind(0) != OpKind::Register {
            return None;
        }
        let d = p.op_register(0);
        if d.size() != 8 || d.full_register().number() != reg.full_register().number() {
            return None;
        }
        match p.mnemonic() {
            Mnemonic::Lea if p.op_kind(1) == OpKind::Memory && p.is_ip_rel_memory_operand() => {
                Some(Val::Const(p.ip_rel_memory_address()))
            }
            Mnemonic::Mov => match p.op_kind(1) {
                OpKind::Immediate32to64 | OpKind::Immediate64 => Some(Val::Const(p.immediate(1))),
                OpKind::Memory if p.is_ip_rel_memory_operand() && p.memory_size().size() == 8 => {
                    let slot = p.ip_rel_memory_address();
                    if self.imports.contains_key(&slot) {
                        Some(Val::Import(slot))
                    } else {
                        self.read_u64(slot).map(Val::Const)
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn reg_val(&self, block: u64, va: u64, prev: Option<&Instruction>, reg: Register) -> Option<Val> {
        if va == block {
            return None;
        }
        let p = prev?;
        if p.next_ip() != va {
            return None;
        }
        self.def_of(p, reg)
    }

    /// `i` is the indirect `call`/`jmp` at `va`, inside the straight-line run
    /// that starts at `block`. `prev` is the decoded instruction ending at `va`.
    pub fn resolve(&self, block: u64, va: u64, i: &Instruction, prev: Option<&Instruction>) -> Resolved {
        match self.resolve_local(block, va, i, prev) {
            Resolved::Unresolved => dataflow::resolve(self.pe, self.text, &self.imports, &self.leaders, block, va, i).resolved,
            r => r,
        }
    }

    fn resolve_local(&self, block: u64, va: u64, i: &Instruction, prev: Option<&Instruction>) -> Resolved {
        match i.op_kind(0) {
            OpKind::Memory if i.is_ip_rel_memory_operand() => {
                self.through_slot(i.ip_rel_memory_address(), How::RipTable)
            }
            OpKind::Memory => {
                let base = i.memory_base();
                if base == Register::None || i.memory_index() != Register::None || !base.is_gpr64() {
                    return Resolved::Unresolved;
                }
                match self.reg_val(block, va, prev, base) {
                    Some(Val::Const(c)) => {
                        self.through_slot(c.wrapping_add(i.memory_displacement64()), How::RegTable)
                    }
                    _ => Resolved::Unresolved,
                }
            }
            OpKind::Register => {
                let r = i.op_register(0);
                if !r.is_gpr64() {
                    return Resolved::Unresolved;
                }
                match self.reg_val(block, va, prev, r) {
                    Some(Val::Const(v)) if self.text.contains(v) => Resolved::Code { target: v, how: How::RegConst },
                    Some(Val::Import(slot)) => match self.imports.get(&slot) {
                        Some(name) => Resolved::Import { name: name.clone() },
                        None => Resolved::Unresolved,
                    },
                    _ => Resolved::Unresolved,
                }
            }
            _ => Resolved::Unresolved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze;

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
    fn text_at(rva: u32) -> usize {
        TEXT_RAW + (rva - TEXT_RVA) as usize
    }
    fn data_at(rva: u32) -> usize {
        DATA_RAW + (rva - DATA_RVA) as usize
    }
    /// disp32 for a rip-relative operand whose instruction ends at `next`.
    fn rel(next: u32, target: u32) -> [u8; 4] {
        ((target as i64 - next as i64) as i32).to_le_bytes()
    }

    /// Synthetic PE32+ with `.text` (RVA 0x1000) and `.data` (RVA 0x2000).
    ///
    /// `.data`: import of `k.dll!Fn` (IAT slot at 0x2200); rip table slot 0x2500 -> F2;
    /// jump slot 0x2508 -> 0x1030.
    ///
    /// `.text`:
    /// - 0x1000 `call [rip+0x2500]`      (rip table -> 0x1040)
    /// - 0x1006 `call [rip+0x2200]`      (IAT)
    /// - 0x100c `lea rax,[rip+0x1080]`; 0x1013 `call rax`   (reg const -> 0x1080)
    /// - 0x1015 `lea rcx,[rip+0x2500]`; 0x101c `call [rcx]`  (reg table -> 0x1040)
    /// - 0x101f `call rbx`               (unresolved)
    /// - 0x1021 `jmp [rip+0x2508]`       (rip table -> 0x1030, same function)
    /// - 0x1030 `ret`, 0x1040 `ret`, 0x1080 `ret`
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

        // import descriptor: OFT 0x2100, Name 0x2300, FirstThunk 0x2200
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

        let t = text_at(TEXT_RVA);
        let code: &[(u32, Vec<u8>)] = &[
            (0x1000, [&[0xff, 0x15][..], &rel(0x1006, 0x2500)].concat()),
            (0x1006, [&[0xff, 0x15][..], &rel(0x100c, 0x2200)].concat()),
            (0x100c, [&[0x48, 0x8d, 0x05][..], &rel(0x1013, 0x1080)].concat()),
            (0x1013, vec![0xff, 0xd0]),
            (0x1015, [&[0x48, 0x8d, 0x0d][..], &rel(0x101c, 0x2500)].concat()),
            (0x101c, vec![0xff, 0x51, 0x00]),
            (0x101f, vec![0xff, 0xd3]),
            (0x1021, [&[0xff, 0x25][..], &rel(0x1027, 0x2508)].concat()),
            (0x1030, vec![0xc3]),
            (0x1040, vec![0xc3]),
            (0x1080, vec![0xc3]),
        ];
        for (rva, bytes) in code {
            let o = t + (rva - TEXT_RVA) as usize;
            b[o..o + bytes.len()].copy_from_slice(bytes);
        }
        b
    }

    #[test]
    fn rip_relative_and_iat_forms() {
        let img = build();
        let pe = Pe::parse(&img).unwrap();
        let a = analyze(&pe).unwrap();
        let f2 = IB + 0x1040;
        let f3 = IB + 0x1080;

        let fns: Vec<u64> = a.functions.keys().copied().collect();
        assert_eq!(fns, vec![IB + 0x1000, f2, f3]);
        assert_eq!(a.closure.instructions, 11);

        assert_eq!(a.resolved[&(IB + 0x1000)], Resolved::Code { target: f2, how: How::RipTable });
        assert_eq!(a.resolved[&(IB + 0x1006)], Resolved::Import { name: "k.dll!Fn".into() });
        assert_eq!(a.resolved[&(IB + 0x1013)], Resolved::Code { target: f3, how: How::RegConst });
        assert_eq!(a.resolved[&(IB + 0x101c)], Resolved::Code { target: f2, how: How::RegTable });
        assert_eq!(a.resolved[&(IB + 0x101f)], Resolved::Unresolved);
        assert_eq!(a.resolved[&(IB + 0x1021)], Resolved::Code { target: IB + 0x1030, how: How::RipTable });

        let s = &a.closure.indirect;
        assert_eq!(s.by_kind["call:rip_table"], 1);
        assert_eq!(s.by_kind["call:reg_const"], 1);
        assert_eq!(s.by_kind["call:reg_table"], 1);
        assert_eq!(s.by_kind["call:iat"], 1);
        assert_eq!(s.by_kind["call:unresolved"], 1);
        assert_eq!(s.by_kind["jmp:rip_table"], 1);
        assert_eq!(s.imports["k.dll!Fn"], 1);
        assert_eq!(a.closure.import_calls, 1);
        assert_eq!(a.closure.indirect_calls_other, 4);
        assert_eq!(a.closure.import_jumps, 0);
        assert_eq!(a.closure.indirect_jumps_other, 1);
    }

    /// Counts on the real DLL (`$FLENGINE_DLL`, else the repo copy). Skipped when absent.
    #[test]
    fn real_dll_resolution_counts() {
        let path = std::env::var("FLENGINE_DLL").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll")
        });
        let Ok(data) = std::fs::read(&path) else {
            eprintln!("skip: {} not found", path.display());
            return;
        };
        let pe = Pe::parse(&data).unwrap();
        let a = analyze(&pe).unwrap();
        let s = &a.closure.indirect;
        eprintln!("functions {}  instructions {}", a.closure.functions, a.closure.instructions);
        for (k, v) in &s.by_kind {
            eprintln!("  {k:<22} {v}");
        }
        for (k, v) in &s.imports {
            eprintln!("  iat {k} {v}");
        }
        let total: usize = s.by_kind.values().sum();
        assert_eq!(total, a.closure.import_calls + a.closure.indirect_calls_other + a.closure.import_jumps + a.closure.indirect_jumps_other);
        assert_eq!(total, a.resolved.len());
    }

    /// `call rax` at 0x1013 is also the target of a `jmp` at 0x1021, so it is a
    /// block leader and its register resolution must be demoted to unresolved.
    #[test]
    fn leader_demotes_register_resolution() {
        // .text: 0x1000 `jne +2` (-> 0x1004 leader target), 0x1002 `lea rax,[rip+0x1010]`
        // would need a longer span, so use: 0x1000 `jz rel8 -> 0x1009`,
        // 0x1002 `lea rax,[rip+...]` (-> 0x1080), 0x1009 `call rax` (also jump target),
        // 0x100b `ret`, 0x1080 `ret`.
        let mut b = build();
        let t = text_at(TEXT_RVA);
        let code: &[(u32, Vec<u8>)] = &[
            (0x1000, vec![0x74, 0x07]),
            (0x1002, [&[0x48, 0x8d, 0x05][..], &rel(0x1009, 0x1080)].concat()),
            (0x1009, vec![0xff, 0xd0]),
            (0x100b, vec![0xc3]),
            (0x1080, vec![0xc3]),
        ];
        for (rva, bytes) in code {
            let o = t + (rva - TEXT_RVA) as usize;
            b[o..o + bytes.len()].copy_from_slice(bytes);
        }
        let pe = Pe::parse(&b).unwrap();
        let a = analyze(&pe).unwrap();
        assert_eq!(a.resolved[&(IB + 0x1009)], Resolved::Unresolved);
        assert_eq!(a.closure.indirect.by_kind.get("call:reg_const").copied().unwrap_or(0), 0);
        assert_eq!(a.closure.indirect.by_kind["call:unresolved"], 1);
    }

    #[test]
    fn block_start_is_not_resolved_by_register() {
        let img = build();
        let pe = Pe::parse(&img).unwrap();
        let text = crate::Text::from_pe(&pe).unwrap();
        let r = Resolver::new(&pe, &text).unwrap();
        let call_rax = match text.decode(IB + 0x1013) {
            crate::Decoded::Ok(i) => i,
            _ => panic!("decode"),
        };
        assert_eq!(r.resolve(IB + 0x1013, IB + 0x1013, &call_rax, None), Resolved::Unresolved);
    }
}
