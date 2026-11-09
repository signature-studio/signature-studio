//! Synthetic PE32+ images and a tiny assembler for the MSVC switch tests (decision 0089). Shared by
//! fe-decode/tests/msvc_switch.rs and fe-lift/tests/msvc_switch.rs (`#[path]`).
//!
//! Layout (ImageBase `IB` = 0x1_4000_0000):
//!
//! | section | RVA    | file offset | size  | content |
//! |---------|--------|-------------|-------|---------|
//! | .text   | 0x1000 | 0x400       | 0x800 | the functions |
//! | .rdata  | 0x2000 | 0xc00       | 0x400 | RVA tables, index byte tables |
//! | .xdata  | 0x3000 | 0x1000      | 0x200 | UNWIND_INFO |
//! | .pdata  | 0x4000 | 0x1200      | 0x200 | RUNTIME_FUNCTION array (data directory 3) |
//!
//! The registers are named by their number (8..=15 here: the encodings below use a REX prefix for them).
#![allow(dead_code)]

use std::collections::BTreeMap;

pub const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
pub const FILE_LEN: usize = 0x1400;

pub const TEXT_RVA: u32 = 0x1000;
pub const RDATA_RVA: u32 = 0x2000;
pub const XDATA_RVA: u32 = 0x3000;
pub const PDATA_RVA: u32 = 0x4000;
/// UNWIND_INFO without handler or chain (version 1, no codes).
pub const UNW_PLAIN: u32 = 0x3000;
/// UNWIND_INFO with UNW_FLAG_CHAININFO whose parent is `chain_parent` (set with `Img::chain_parent`).
pub const UNW_CHAIN: u32 = 0x3010;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// What goes into the image.
#[derive(Default)]
pub struct Img {
    /// (RVA, bytes) in `.text`
    pub text: Vec<(u32, Vec<u8>)>,
    /// (RVA, bytes) in `.rdata`
    pub rdata: Vec<(u32, Vec<u8>)>,
    /// RUNTIME_FUNCTION entries: (begin, end, unwind RVA)
    pub pdata: Vec<(u32, u32, u32)>,
    /// The RUNTIME_FUNCTION stored in the chained UNWIND_INFO at `UNW_CHAIN`
    pub chain_parent: Option<(u32, u32, u32)>,
    /// no exception directory at all
    pub no_pdata: bool,
    pub entry: u32,
    /// `.rdata` (the tables) has IMAGE_SCN_MEM_WRITE too (decision 0109 checks that the tables of a scheduled switch are read-only)
    pub rdata_writable: bool,
}

impl Img {
    pub fn build(&self) -> Vec<u8> {
        let mut b = vec![0u8; FILE_LEN];
        w16(&mut b, 0, 0x5a4d);
        w32(&mut b, 0x3c, LFANEW as u32);
        w32(&mut b, LFANEW, 0x4550);
        let f = LFANEW + 4;
        w16(&mut b, f, 0x8664);
        w16(&mut b, f + 2, 4);
        w16(&mut b, f + 16, OPT_SIZE as u16);
        w16(&mut b, f + 18, 0x2022);
        let o = LFANEW + 24;
        w16(&mut b, o, 0x20b);
        w32(&mut b, o + 16, self.entry);
        w64(&mut b, o + 24, IB);
        w32(&mut b, o + 32, 0x1000);
        w32(&mut b, o + 36, 0x200);
        w32(&mut b, o + 56, 0x5000);
        w32(&mut b, o + 60, 0x400);
        w16(&mut b, o + 68, 3);
        w32(&mut b, o + 108, 16);
        if !self.no_pdata {
            w32(&mut b, o + 112 + 8 * 3, PDATA_RVA);
            w32(&mut b, o + 116 + 8 * 3, 12 * self.pdata.len() as u32);
        }
        let sec = |b: &mut Vec<u8>, i: usize, name: &[u8], rva: u32, size: u32, raw: u32, flags: u32| {
            let s = SEC_TABLE + 40 * i;
            b[s..s + name.len()].copy_from_slice(name);
            w32(b, s + 8, size);
            w32(b, s + 12, rva);
            w32(b, s + 16, size);
            w32(b, s + 20, raw);
            w32(b, s + 36, flags);
        };
        sec(&mut b, 0, b".text", TEXT_RVA, 0x800, 0x400, 0x6000_0020);
        sec(&mut b, 1, b".rdata", RDATA_RVA, 0x400, 0xc00, if self.rdata_writable { 0xc000_0040 } else { 0x4000_0040 });
        sec(&mut b, 2, b".xdata", XDATA_RVA, 0x200, 0x1000, 0x4000_0040);
        sec(&mut b, 3, b".pdata", PDATA_RVA, 0x200, 0x1200, 0x4000_0040);
        for (rva, bytes) in &self.text {
            assert!(*rva >= TEXT_RVA && *rva as usize + bytes.len() <= (TEXT_RVA + 0x800) as usize, "text at {rva:#x}");
            let at = 0x400 + (rva - TEXT_RVA) as usize;
            b[at..at + bytes.len()].copy_from_slice(bytes);
        }
        for (rva, bytes) in &self.rdata {
            assert!(*rva >= RDATA_RVA && *rva as usize + bytes.len() <= (RDATA_RVA + 0x400) as usize, "rdata at {rva:#x}");
            let at = 0xc00 + (rva - RDATA_RVA) as usize;
            b[at..at + bytes.len()].copy_from_slice(bytes);
        }
        // .xdata: plain UNWIND_INFO (version 1) at 0x3000; chained one at 0x3010 (flags 4, no codes, then the parent)
        b[0x1000] = 1;
        b[0x1010] = 1 | (4 << 3);
        if let Some((pb, pe, pu)) = self.chain_parent {
            w32(&mut b, 0x1010 + 4, pb);
            w32(&mut b, 0x1010 + 8, pe);
            w32(&mut b, 0x1010 + 12, pu);
        }
        for (k, (begin, end, unw)) in self.pdata.iter().enumerate() {
            let at = 0x1200 + 12 * k;
            w32(&mut b, at, *begin);
            w32(&mut b, at + 4, *end);
            w32(&mut b, at + 8, *unw);
        }
        b
    }
}

/// The 4-byte little-endian entries of a table.
pub fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A tiny assembler for the instruction forms of the tests. `B` is a register number 8..=15.
pub struct Asm {
    pub at: u32,
    pub code: Vec<u8>,
    labels: BTreeMap<&'static str, u32>,
    fix: Vec<(usize, &'static str, bool)>,
}

impl Asm {
    pub fn new(at: u32) -> Asm {
        Asm { at, code: Vec::new(), labels: BTreeMap::new(), fix: Vec::new() }
    }
    pub fn here(&self) -> u32 {
        self.at + self.code.len() as u32
    }
    pub fn label(&mut self, n: &'static str) -> &mut Self {
        assert!(self.labels.insert(n, self.here()).is_none(), "label {n} twice");
        self
    }
    pub fn raw(&mut self, b: &[u8]) -> &mut Self {
        self.code.extend_from_slice(b);
        self
    }
    /// `jcc rel8` (opcode `op`: 0x77 ja, 0x74 je, 0x75 jne) or `jmp rel8` (0xeb)
    pub fn jcc8(&mut self, op: u8, l: &'static str) -> &mut Self {
        self.code.extend_from_slice(&[op, 0]);
        self.fix.push((self.code.len() - 1, l, false));
        self
    }
    pub fn jmp8(&mut self, l: &'static str) -> &mut Self {
        self.jcc8(0xeb, l)
    }
    /// `jmp rel32` to a label
    pub fn jmp32(&mut self, l: &'static str) -> &mut Self {
        self.code.push(0xe9);
        self.code.extend_from_slice(&[0; 4]);
        self.fix.push((self.code.len() - 4, l, true));
        self
    }
    /// `jmp rel32` / `call rel32` to an RVA
    pub fn jmp_rva(&mut self, target: u32) -> &mut Self {
        let end = self.here() + 5;
        self.raw(&[0xe9]).raw(&(target.wrapping_sub(end)).to_le_bytes())
    }
    pub fn call_rva(&mut self, target: u32) -> &mut Self {
        let end = self.here() + 5;
        self.raw(&[0xe8]).raw(&(target.wrapping_sub(end)).to_le_bytes())
    }
    /// `lea B,[rip+d]` whose value is `IB + delta`
    pub fn lea_img(&mut self, b: u8, delta: i32) -> &mut Self {
        let end = self.here() as i64 + 7;
        let d = delta as i64 - end;
        self.raw(&[0x4c, 0x8d, 0x05 | ((b & 7) << 3)]).raw(&(d as i32).to_le_bytes())
    }
    pub fn push(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x41, 0x50 | (b & 7)])
    }
    pub fn pop(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x41, 0x58 | (b & 7)])
    }
    /// `xor Bd,Bd`
    pub fn xor_b(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x45, 0x31, 0xc0 | ((b & 7) << 3) | (b & 7)])
    }
    /// `mov B,rcx`
    pub fn mov_b_rcx(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x49, 0x89, 0xc8 | (b & 7)])
    }
    /// `movzx Bd,byte [rdx+0xe]`
    pub fn movzx_b_rdx(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x44, 0x0f, 0xb6, 0x42 | ((b & 7) << 3), 0x0e])
    }
    /// `and eax,3`
    pub fn and_eax_3(&mut self) -> &mut Self {
        self.raw(&[0x83, 0xe0, 0x03])
    }
    /// `jmp rdx`
    pub fn jmp_rdx(&mut self) -> &mut Self {
        self.raw(&[0xff, 0xe2])
    }
    /// `jmp qword ptr [rdx]`
    pub fn jmp_mem_rdx(&mut self) -> &mut Self {
        self.raw(&[0xff, 0x22])
    }
    pub fn mov_eax_ecx(&mut self) -> &mut Self {
        self.raw(&[0x8b, 0xc1])
    }
    pub fn mov_eax(&mut self, k: u32) -> &mut Self {
        self.raw(&[0xb8]).raw(&k.to_le_bytes())
    }
    pub fn cmp_eax(&mut self, n: u8) -> &mut Self {
        self.raw(&[0x83, 0xf8, n])
    }
    pub fn cmp_rax(&mut self, n: u8) -> &mut Self {
        self.raw(&[0x48, 0x83, 0xf8, n])
    }
    pub fn cmp_edx(&mut self, n: u8) -> &mut Self {
        self.raw(&[0x83, 0xfa, n])
    }
    pub fn cdqe(&mut self) -> &mut Self {
        self.raw(&[0x48, 0x98])
    }
    pub fn test_ecx(&mut self) -> &mut Self {
        self.raw(&[0x85, 0xc9])
    }
    /// `movzx eax, byte [rax + B + t1]`
    pub fn movzx_eax_byte(&mut self, b: u8, t1: u32) -> &mut Self {
        self.raw(&[0x42, 0x0f, 0xb6, 0x84, (b & 7) << 3]).raw(&t1.to_le_bytes())
    }
    /// `mov ecx, [B + rax*4 + t2]`
    pub fn mov_ecx_table(&mut self, b: u8, t2: u32) -> &mut Self {
        self.raw(&[0x41, 0x8b, 0x8c, 0x80 | (b & 7)]).raw(&t2.to_le_bytes())
    }
    pub fn add_rcx_b(&mut self, b: u8) -> &mut Self {
        self.raw(&[0x4c, 0x01, 0xc1 | ((b & 7) << 3)])
    }
    pub fn jmp_rcx(&mut self) -> &mut Self {
        self.raw(&[0xff, 0xe1])
    }
    pub fn ret(&mut self) -> &mut Self {
        self.raw(&[0xc3])
    }
    pub fn addr(&self, l: &str) -> u32 {
        self.labels[l]
    }
    pub fn finish(mut self) -> (Vec<u8>, BTreeMap<&'static str, u32>) {
        for (pos, l, wide) in std::mem::take(&mut self.fix) {
            let target = self.labels[l] as i64;
            if wide {
                let end = self.at as i64 + pos as i64 + 4;
                self.code[pos..pos + 4].copy_from_slice(&((target - end) as i32).to_le_bytes());
            } else {
                let end = self.at as i64 + pos as i64 + 1;
                let d = target - end;
                assert!((-128..128).contains(&d), "rel8 out of range to {l}");
                self.code[pos] = d as i8 as u8;
            }
        }
        (self.code, self.labels)
    }
}

// ---------------------------------------------------------------------------------------------
// The switch function of the tests
// ---------------------------------------------------------------------------------------------

/// Where the `lea B,[ImageBase]` is, and what else writes B.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lea {
    /// right after the `push B`, in the entry block (a special case of dominance; decision 0089's simple condition)
    Entry,
    /// in the entry block, but the value is `IB + delta` (not the image base)
    Wrong(i32),
    /// after a `test; je` to the next instruction (the entry block ends before it; both edges lead to the `lea`, so it
    /// is outside the entry block and still on every path: decision 0090 resolves it, decision 0089's condition did not)
    AfterBranch,
    /// after a diamond (`test ecx,ecx; jne other; mov edx,1; jmp join; other: mov edx,2; join: lea B`): outside the entry
    /// block, on every path (the shape of Blend2D's `f_4709ea0`)
    AfterDiamond,
    /// `test ecx,ecx; je skip; lea B,[ImageBase]; skip:` the `lea` is on one of the two paths only: not proven
    OneSided,
    /// `test ecx,ecx; je other; lea B; jmp join; other: lea B; join:` two writes, one on each path: B is written twice
    BothSides,
    /// in the entry block, followed by a second write (`xor Bd,Bd`)
    Twice,
    /// no `lea`: `mov B,rcx` (B comes from an argument)
    FromArg,
    /// decision 0094: `test ecx,ecx; je other; lea B,[ImageBase]; jmp join; other: mov B,rcx; join:` the definitions that
    /// reach the jump are a `lea` and a `mov`: not all `lea`
    LeaOrArg,
    /// decision 0094: B is never written (only pushed and popped when `push_pop`): the function entry's B reaches the jump
    Never,
}

/// A second write to B that is not the `lea` (decision 0094: it only matters if it reaches the jump).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SideWrite {
    None,
    /// `mov B,rcx` at the start of the `def` block (the `ja` target; it falls into `end`: `[pop B]; ret`). The shape of
    /// Blend2D `f_48a2060`'s `mov r10,rdx`: a write on a branch that ends in `ret` and never comes back to the jump
    InDef,
    /// `mov B,rcx` at the start of case block `t2` (a target of the jump itself; goes to `end`)
    InCase,
    /// `mov B,rcx` at the start of `t2`, then `jmp guard`: the write comes back to the jump through the jump's own table
    InCaseLoop,
}

/// A call (to `HELPER`) that is not the one of `Sw::call` (right after the `lea`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CallAt {
    None,
    /// at the function entry, before the `lea` (it does not lie between the `lea` and the jump)
    BeforeLea,
    /// at the start of the `def` block (it falls into `end`: ret; it never comes back to the jump)
    InDef,
    /// at the start of `t2`, then `jmp guard`: a path `lea` -> jump -> call -> jump
    InCaseLoop,
}

/// Another jump in the function besides the switch's `jmp rcx` (decision 0090, item 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OtherJmp {
    None,
    /// `jmp rdx` in a block after the final `ret` (it is in the function's pieces; no branch reaches it)
    Reg,
    /// `jmp qword ptr [rdx]` in a block after the final `ret`
    Mem,
    /// the `def` block ends in a direct `jmp rel32` to the helper function (a tail call; not an indirect jump)
    TailCall,
    /// the `def` block is a second (c1) switch (`mov eax,ecx; cmp eax,3; ja def2; mov ecx,[B+rax*4+TABLE2]; add rcx,B; jmp
    /// rcx`; 4 entries over t0 and t1), reachable on its own from the `ja def` of the first one
    SecondSwitch,
    /// decision 0094: the `def` block (reached from the `ja`) ends in `jmp rdx`, with only the `lea` reaching it
    ReachedReg,
    /// decision 0094: the `def` block is `movzx Bd,byte [rdx+0xe]; jmp rdx`: a definition that is not `lea` reaches another
    /// indirect jump
    ReachedAfterWrite,
    /// decision 0094: the same with `jmp qword ptr [rdx]`
    ReachedAfterWriteMem,
    /// decision 0094: `test ecx,ecx; jne ind` at the function entry, before the `lea`; `ind` (after the `ret`) is `jmp rdx`.
    /// The entry's undefined B reaches that indirect jump
    ReachedBeforeLea,
    /// decision 0094: the `def` block is a table jump whose index comes from `and eax,3` (no `cmp; ja`: N is not derived),
    /// the shape of the three `jmp r11` of Blend2D `f_48a2060`; its 4 entries are over t0 and t1. Label `jmp2`
    UnguardedSwitch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Guard {
    /// `cmp eax,N; ja default`
    Eax,
    /// `cmp rax,N; ja default`
    Rax,
    /// `cmp edx,N; ja default` (not the table index)
    OtherReg,
    /// no guard
    Missing,
}

/// Which table entries are not in the function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bad {
    None,
    /// entry 1 points to the start of the function at `OTHER_FN`
    OtherFunction,
    /// entry 1 points into `.rdata` (not executable, in no function)
    NotExecutable,
    /// entry 1 points one byte into an instruction of this function
    Misaligned,
    /// entry 1 points into the second piece (chained RUNTIME_FUNCTION at `PIECE2`) of this function
    ChainedPiece,
}

#[derive(Clone, Debug)]
pub struct Sw {
    pub base: u8,
    pub two_stage: bool,
    pub n: u8,
    pub lea: Lea,
    pub guard: Guard,
    pub cdqe: bool,
    pub push_pop: bool,
    /// a `call` (to `HELPER`) in the entry block after the `lea`
    pub call: bool,
    /// `ja` goes through a block that jumps to the `mov` of the idiom (a landing inside the idiom)
    pub landing: bool,
    pub bad: Bad,
    /// `pop B` at the end, without a `push B` at the start
    pub pop_only: bool,
    /// the other function (`OTHER_FN`) jumps into the middle of this one (to block `t0`) instead of returning
    pub foreign: bool,
    /// no UNWIND_INFO/RUNTIME_FUNCTION at all (the image has no exception directory)
    pub no_pdata: bool,
    /// `sub eax,K` before the guard (K = 0: none): the form `sub idx,K; cmp idx,N; ja`
    pub sub_k: u8,
    /// another jump in the function (decision 0090)
    pub other_jmp: OtherJmp,
    /// instructions (raw bytes, one entry each) between the guard's `ja` and the first instruction of the table idiom
    /// (decision 0093: padding `nop`s; other instructions to show they are not allowed)
    pub pad: Vec<Vec<u8>>,
    /// the padding comes before the `cdqe` (default: after it, right in front of the table instructions)
    pub pad_before_cdqe: bool,
    /// `ja` goes through a block that jumps to the byte `k` of the padding (`pad` concatenated; k = 0 is its first
    /// instruction, a k inside an instruction is a branch into the middle of a multi-byte `nop`)
    pub pad_landing: Option<usize>,
    /// a second write to B (decision 0094)
    pub side_write: SideWrite,
    /// a call besides `call` (decision 0094)
    pub call_at: CallAt,
}

/// Padding forms (decision 0093). Lengths 1, 2, 3, 4, 5, 6, 7, 8, 9, 12 (Blend2D `f_48a2060`) and 15.
pub const NOP_1: &[u8] = &[0x90];
pub const NOP_2_XCHG_AX_AX: &[u8] = &[0x66, 0x90];
pub const NOP_3: &[u8] = &[0x0f, 0x1f, 0x00];
pub const NOP_4: &[u8] = &[0x0f, 0x1f, 0x40, 0x00];
pub const NOP_5: &[u8] = &[0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const NOP_6: &[u8] = &[0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const NOP_7: &[u8] = &[0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00];
pub const NOP_8: &[u8] = &[0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const NOP_9: &[u8] = &[0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
/// `data16 data16 data16 nop word [rax+rax]`: the 12 bytes of Blend2D `f_48a2060` at 0x48a2074
pub const NOP_12: &[u8] = &[0x66, 0x66, 0x66, 0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
/// the longest x86 instruction: six `66`, a `cs` override, `0f 1f 84` and a 32-bit displacement
pub const NOP_15: &[u8] = &[0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const ALL_NOPS: &[&[u8]] = &[NOP_1, NOP_2_XCHG_AX_AX, NOP_3, NOP_4, NOP_5, NOP_6, NOP_7, NOP_8, NOP_9, NOP_12, NOP_15];

/// Instructions that are not padding (decision 0093 does not allow them between the `ja` and the table idiom).
pub const MOV_EDX_1: &[u8] = &[0xba, 0x01, 0x00, 0x00, 0x00];
/// `mov edx,edx`
pub const MOV_EDX_EDX: &[u8] = &[0x8b, 0xd2];
/// `xchg r8,rax` (`49 90`: not a `nop`; it changes rax and r8)
pub const XCHG_R8_RAX: &[u8] = &[0x49, 0x90];
/// `xchg eax,eax` (`87 c0`: zero-extends rax; not the `nop` encoding)
pub const XCHG_EAX_EAX: &[u8] = &[0x87, 0xc0];
/// `pause` (`f3 90`)
pub const PAUSE: &[u8] = &[0xf3, 0x90];
pub const INT3: &[u8] = &[0xcc];
pub const UD2: &[u8] = &[0x0f, 0x0b];
/// `lea r11,[rip+0]` (a `lea` of a register that is not the base)
pub const LEA_R11_RIP: &[u8] = &[0x4c, 0x8d, 0x1d, 0x00, 0x00, 0x00, 0x00];
/// `0f 19 /0` (`nop` hint space, reserved-nop: not the `nop` mnemonic)
pub const RESERVED_NOP: &[u8] = &[0x0f, 0x19, 0x00];

pub const OTHER_FN: u32 = 0x1400;
pub const HELPER: u32 = 0x1500;
pub const MAIN: u32 = 0x1600;
pub const PIECE2: u32 = 0x1180;
pub const TABLE: u32 = 0x2000;
pub const BYTES: u32 = 0x2100;
pub const TABLE2: u32 = 0x2300;

impl Sw {
    /// (c1) as the decision writes it: B = r12, `cmp eax,3; ja`, 4 entries, 3 distinct targets
    pub fn c1() -> Sw {
        Sw {
            base: 12,
            two_stage: false,
            n: 3,
            lea: Lea::Entry,
            guard: Guard::Eax,
            cdqe: false,
            push_pop: true,
            call: false,
            landing: false,
            bad: Bad::None,
            pop_only: false,
            foreign: false,
            no_pdata: false,
            sub_k: 0,
            other_jmp: OtherJmp::None,
            pad: Vec::new(),
            pad_before_cdqe: false,
            pad_landing: None,
            side_write: SideWrite::None,
            call_at: CallAt::None,
        }
    }
    /// (c2): B = r13, `cmp eax,5; ja`, a 6-byte index table over 3 RVA entries
    pub fn c2() -> Sw {
        Sw { base: 13, two_stage: true, n: 5, ..Sw::c1() }
    }
}

/// What `build_switch` produced.
pub struct Built {
    pub img: Img,
    pub labels: BTreeMap<&'static str, u32>,
    /// the RVA table (as written to `.rdata`), the number of entries read
    pub table: Vec<u32>,
    pub byte_table: Vec<u8>,
    pub entries: usize,
    pub func: (u32, u32),
}

/// The function at `at` with the switch described by `o`, its tables, a helper function and the other function; the
/// RUNTIME_FUNCTIONs of all of them. Blocks: `t0`, `t1`, `t2` (`mov eax,0x100+k`, jump to `end`), `def` (`mov eax,0xdead`),
/// `end` (`pop B; ret`). One-stage table: entries `[t0, t1, t0, t2, ...]`; two-stage: index bytes `[0, 2, 1, 2, 0, 1]` over
/// the RVA entries `[t0, t1, t2]`, followed by one more RVA (never read: the entry count is max(bytes)+1).
pub fn build_switch(at: u32, o: &Sw) -> Built {
    let b = o.base;
    let mut a = Asm::new(at);
    if o.push_pop {
        a.push(b);
    }
    if o.other_jmp == OtherJmp::ReachedBeforeLea {
        a.test_ecx().jcc8(0x75, "ind");
    }
    if o.call_at == CallAt::BeforeLea {
        a.call_rva(HELPER);
    }
    match o.lea {
        Lea::LeaOrArg => {
            a.test_ecx().jcc8(0x74, "lao_other").lea_img(b, 0).jmp8("lao_join");
            a.label("lao_other").mov_b_rcx(b).label("lao_join");
        }
        Lea::Never => {}
        Lea::Entry => {
            a.lea_img(b, 0);
        }
        Lea::Wrong(d) => {
            a.lea_img(b, d);
        }
        Lea::AfterBranch => {
            a.test_ecx().jcc8(0x74, "afterbr").label("afterbr").lea_img(b, 0);
        }
        Lea::AfterDiamond => {
            a.test_ecx().jcc8(0x75, "dother").raw(&[0xba, 1, 0, 0, 0]).jmp8("djoin");
            a.label("dother").raw(&[0xba, 2, 0, 0, 0]).label("djoin").lea_img(b, 0);
        }
        Lea::OneSided => {
            a.test_ecx().jcc8(0x74, "skip").lea_img(b, 0).label("skip");
        }
        Lea::BothSides => {
            a.test_ecx().jcc8(0x74, "bsother").lea_img(b, 0).jmp8("bsjoin");
            a.label("bsother").lea_img(b, 0).label("bsjoin");
        }
        Lea::Twice => {
            a.lea_img(b, 0).xor_b(b);
        }
        Lea::FromArg => {
            a.mov_b_rcx(b);
        }
    }
    if o.call {
        a.call_rva(HELPER);
    }
    a.mov_eax_ecx();
    if o.sub_k != 0 {
        a.raw(&[0x83, 0xe8, o.sub_k]);
    }
    let landing = o.landing || o.pad_landing.is_some();
    match o.guard {
        Guard::Eax => {
            a.label("guard").cmp_eax(o.n).jcc8(0x77, if landing { "land" } else { "def" });
        }
        Guard::Rax => {
            a.label("guard").cmp_rax(o.n).jcc8(0x77, if landing { "land" } else { "def" });
        }
        Guard::OtherReg => {
            a.label("guard").cmp_edx(o.n).jcc8(0x77, if landing { "land" } else { "def" });
        }
        Guard::Missing => {}
    }
    let pad: Vec<u8> = o.pad.iter().flatten().copied().collect();
    let put_pad = |a: &mut Asm| {
        match o.pad_landing {
            Some(k) => {
                assert!(k < pad.len(), "padland {k} outside the {} padding bytes", pad.len());
                a.raw(&pad[..k]).label("padland").raw(&pad[k..]);
            }
            None => {
                a.raw(&pad);
            }
        }
    };
    if o.pad_before_cdqe {
        put_pad(&mut a);
    }
    if o.cdqe {
        a.cdqe();
    }
    if !o.pad_before_cdqe {
        put_pad(&mut a);
    }
    a.label("idiom");
    if o.two_stage {
        a.movzx_eax_byte(b, BYTES);
    }
    a.mov_ecx_table(b, TABLE).add_rcx_b(b).label("jmp").jmp_rcx();
    let mut kk = 0;
    for l in ["t0", "t1", "t2"] {
        a.label(l);
        let loops = l == "t2" && (o.side_write == SideWrite::InCaseLoop || o.call_at == CallAt::InCaseLoop);
        if l == "t2" {
            if matches!(o.side_write, SideWrite::InCase | SideWrite::InCaseLoop) {
                a.mov_b_rcx(b);
            }
            if o.call_at == CallAt::InCaseLoop {
                a.call_rva(HELPER);
            }
        }
        if loops {
            // back to the guard: a path jump -> t2 -> (write, call) -> jump through the jump's own table
            a.jmp8("guard");
        } else {
            a.mov_eax(0x100 + kk).jmp8("end");
        }
        kk += 1;
    }
    if landing {
        a.label("land").jmp8(if o.pad_landing.is_some() { "padland" } else { "idiom" });
    }
    match o.other_jmp {
        OtherJmp::ReachedReg => {
            a.label("def").mov_eax(0xdead).label("ind").jmp_rdx();
        }
        OtherJmp::ReachedAfterWrite => {
            a.label("def").movzx_b_rdx(b).label("ind").jmp_rdx();
        }
        OtherJmp::ReachedAfterWriteMem => {
            a.label("def").movzx_b_rdx(b).label("ind").jmp_mem_rdx();
        }
        OtherJmp::UnguardedSwitch => {
            // the index comes from `and eax,3`: there is no `cmp; ja`, N is not derived (the shape of the `jmp r11` of
            // Blend2D f_48a2060, whose index comes from `shr`)
            a.label("def").mov_eax_ecx().and_eax_3().mov_ecx_table(b, TABLE2).add_rcx_b(b).label("jmp2").jmp_rcx();
        }
        OtherJmp::SecondSwitch => {
            a.label("def").mov_eax_ecx().cmp_eax(3).jcc8(0x77, "def2").mov_ecx_table(b, TABLE2).add_rcx_b(b).label("jmp2").jmp_rcx();
            a.label("def2").mov_eax(0xdead);
        }
        OtherJmp::TailCall => {
            // a real tail call: the frame is restored first (`pop B`), then a direct `jmp rel32` to the helper, which returns
            a.label("def").mov_eax(0xdead);
            if o.push_pop {
                a.pop(b);
            }
            a.jmp_rva(HELPER);
        }
        _ => {
            a.label("def");
            if o.side_write == SideWrite::InDef {
                a.mov_b_rcx(b);
            }
            if o.call_at == CallAt::InDef {
                a.call_rva(HELPER);
            }
            a.mov_eax(0xdead);
        }
    }
    a.label("end");
    if o.push_pop || o.pop_only {
        a.pop(b);
    }
    a.ret();
    match o.other_jmp {
        OtherJmp::Reg | OtherJmp::ReachedBeforeLea => {
            a.label("ind").raw(&[0xff, 0xe2]);
        }
        OtherJmp::Mem => {
            a.label("ind").raw(&[0xff, 0x22]);
        }
        _ => {}
    }
    let (code, labels) = a.finish();
    let end = at + code.len() as u32;
    let t = |l: &str| labels[l];
    let mut img = Img { entry: MAIN, no_pdata: o.no_pdata, ..Default::default() };
    let bad1 = match o.bad {
        Bad::None => t("t1"),
        Bad::OtherFunction => OTHER_FN,
        Bad::NotExecutable => RDATA_RVA + 0x200,
        Bad::Misaligned => t("t2") + 1,
        Bad::ChainedPiece => PIECE2,
    };
    let (table, byte_table, entries) = if o.two_stage {
        let bytes = vec![0u8, 2, 1, 2, 0, 1];
        let tab = vec![t("t0"), bad1, t("t2"), 0x1000];
        let k = *bytes.iter().max().unwrap() as usize + 1;
        // entry 1 is reached by index byte 2 (index 2 -> byte 1 -> RVA entry 1): bytes[2] = 1
        (tab, bytes, k)
    } else {
        let tab = vec![t("t0"), bad1, t("t0"), t("t2")];
        (tab, Vec::new(), o.n as usize + 1)
    };
    img.rdata.push((TABLE, u32s(&table)));
    if matches!(o.other_jmp, OtherJmp::SecondSwitch | OtherJmp::UnguardedSwitch) {
        img.rdata.push((TABLE2, u32s(&[t("t0"), t("t1"), t("t0"), t("t1")])));
    }
    if o.two_stage {
        img.rdata.push((BYTES, byte_table.clone()));
    }
    img.text.push((at, code));
    // the other function, the helper and main (`call f; call other; ret`, the entry of the image)
    if o.foreign {
        let mut j = Asm::new(OTHER_FN);
        j.jmp_rva(t("t0"));
        img.text.push((OTHER_FN, j.code));
        img.pdata.push((OTHER_FN, OTHER_FN + 5, UNW_PLAIN));
    } else {
        img.text.push((OTHER_FN, vec![0xb8, 0x77, 0, 0, 0, 0xc3]));
        img.pdata.push((OTHER_FN, OTHER_FN + 6, UNW_PLAIN));
    }
    img.text.push((HELPER, vec![0xc3]));
    let mut m = Asm::new(MAIN);
    m.call_rva(at).call_rva(OTHER_FN).ret();
    img.text.push((MAIN, m.code));
    img.pdata.push((at, end, UNW_PLAIN));
    img.pdata.push((HELPER, HELPER + 1, UNW_PLAIN));
    img.pdata.push((MAIN, MAIN + 11, UNW_PLAIN));
    if o.bad == Bad::ChainedPiece {
        img.text.push((PIECE2, vec![0xb8, 0x03, 0x01, 0, 0, 0xc3]));
        img.pdata.push((PIECE2, PIECE2 + 6, UNW_CHAIN));
        img.chain_parent = Some((at, end, UNW_PLAIN));
    }
    img.pdata.sort();
    Built { img, labels, table, byte_table, entries, func: (at, end) }
}

// ---------------------------------------------------------------------------------------------
// The swapped form of decision 0095: `lea B,[ImageBase]; mov eX,[B+I*4+T]; add B,rX; <gap>; jmp B`, N from the guard's branch
// ---------------------------------------------------------------------------------------------

/// How the guard of a swapped-form function is written. I = rcx, N = `Swp::n`. The table block is the code from `tbl` on; the
/// default block (`def`) is `mov eax,0xdead; ret`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwGuard {
    /// `cmp rcx,N; jbe tbl`: the table block is the taken side (the shape of Blend2D `f_489f7d0`); `def` follows the branch
    JbeTaken,
    /// `cmp rcx,N; ja def`: the table block follows (the `ja` not taken)
    JaFall,
    /// `cmp ecx,N; ja def` (32-bit compare)
    JaFall32,
    /// `cmp rcx,N+1; jae def`: not taken means rcx < N+1
    JaeFall,
    /// `cmp rcx,N+1; jb tbl`: taken means rcx < N+1
    JbTaken,
    /// `cmp rcx,N; ja tbl`: the table block is the I > N side (wrong side)
    JaTaken,
    /// `cmp rcx,N; jbe def`: the table block follows the `jbe` not taken (wrong side)
    JbeFall,
    /// `cmp rcx,N+1; jae tbl` (wrong side)
    JaeTaken,
    /// `cmp rcx,N+1; jb def`, the table block follows (wrong side)
    JbFall,
    /// `cmp rdx,N; jbe tbl`: the compare is on another register than the index
    OtherReg,
    /// `cmp rcx,N; mov ecx,ecx; jbe tbl`: the index is written between the `cmp` and the branch
    WriteBetween,
    /// `cmp rcx,N; jbe tbl`, and `mov ecx,ecx` is the first instruction of the table block
    WriteAfter,
    /// a second way into the table block: `test edx,edx; jne tbl` before the guard
    ExtraBranch,
    /// the default block does not end in `ret` and falls into the table block
    DefFallsIn,
    /// `test rcx,rcx; jbe tbl`: the flags come from a `test`
    NoCmp,
    /// `cmp rcx,rdx; jbe tbl`: not a compare with an immediate
    CmpReg,
}

/// Where B gets the image base.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwLea {
    /// `lea B,[ImageBase]` as the first instruction of the table block (as in `f_489f7d0`)
    InTable,
    /// `lea B,[ImageBase]` at the function entry, before the guard
    AtEntry,
    /// at the entry: `lea B; test edx,edx; je s; mov B,rdx; s:`: a `mov` and the `lea` both reach the table load
    AtEntryOrArg,
    /// in the table block but of the address IB + delta
    Wrong(i32),
    /// no write to B at all
    Never,
    /// at the entry, before the guard: `lea B,[ImageBase]; call HELPER` (a call on the path from the `lea` to the table load)
    AtEntryThenCall,
}

/// The instructions of the idiom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwIdiom {
    /// `mov eX,[B+rcx*4+T]; <gap_before>; add B,rX; <gap_after>; jmp B`
    Swapped,
    /// the load is through another register than B: `mov eX,[r10+rcx*4+T]; add B,rX; jmp B` (r13 when B is r10)
    LoadBaseOther,
    /// the add and the jump are on another register than the base: `mov eX,[B+rcx*4+T]; add r10,rX; jmp r10`
    JmpOther,
    /// `movsxd rX,dword [B+rcx*4+T]; add B,rX; jmp B`
    Movsxd,
    /// the F2 form of Blend2D `0x47513d7`: `mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; jmp rcx`
    F2,
    /// X is the index register: `mov ecx,[B+rcx*4+T]; add B,rcx; jmp B`
    XIsIndex,
    /// the two-stage table with the swapped add: `movzx edx,byte [rcx+B+T1]; mov eX,[B+rdx*4+T2]; add B,rX; jmp B`
    TwoStage,
}

/// What block `t2` does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwTarget {
    /// `mov eax,0x102; ret`
    Plain,
    /// `jmp guard`
    LoopToGuard,
    /// `jmp load`: a way into the middle of the table block
    JmpToLoad,
    /// `jmp add`
    JmpToAdd,
    /// `jmp padl`: the first padding instruction (the table block when there is none)
    JmpToPad,
    /// `jmp rdx`: another indirect jump of the function, reached through the table
    JmpRdx,
}

/// What the default block does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwDef {
    /// `mov eax,0xdead; ret`
    Plain,
    /// `mov eax,0xdead; mov B,rdx; ret`: a write to B on a branch that never comes back to the table load
    WriteThenRet,
    /// `mov eax,0xdead; mov B,rdx; jmp rdx`: a non-`lea` definition of B reaches another indirect jump
    WriteThenJmpRdx,
}

#[derive(Clone, Debug)]
pub struct Swp {
    /// B, a register number 8..=15 (not 11 when the gap has `mov rax,r11`)
    pub base: u8,
    /// X, the register the table value is loaded to (0 = rax)
    pub x: u8,
    pub n: u32,
    pub guard: SwGuard,
    pub lea: SwLea,
    pub idiom: SwIdiom,
    /// instructions (raw bytes) between the load and the `add`, and between the `add` and the `jmp`
    pub gap_before: Vec<Vec<u8>>,
    pub gap_after: Vec<Vec<u8>>,
    /// instructions in front of the table block (the guard's branch lands behind them, or runs through them)
    pub pad: Vec<Vec<u8>>,
    pub bad: Bad,
    /// `push B` at the entry, `pop B` before every `ret`
    pub push_pop: bool,
    pub target2: SwTarget,
    pub def: SwDef,
    pub foreign: bool,
    pub no_pdata: bool,
}

pub const ADD_RDX_RCX: &[u8] = &[0x48, 0x01, 0xca];
pub const MOV_RAX_R11: &[u8] = &[0x4c, 0x89, 0xd8];
pub const MOV_RDX_R11: &[u8] = &[0x4c, 0x89, 0xda];
/// writes the index register rcx
pub const MOV_RCX_RDX: &[u8] = &[0x48, 0x89, 0xd1];
/// reads the carry flag
pub const ADC_RDX_RDX: &[u8] = &[0x48, 0x11, 0xd2];
/// writes memory
pub const MOV_MEM_RSP_RDX: &[u8] = &[0x48, 0x89, 0x54, 0x24, 0xf8];
pub const MOV_ECX_ECX: &[u8] = &[0x89, 0xc9];
/// `test edx,edx`
const TEST_EDX: &[u8] = &[0x85, 0xd2];

/// `mov rax,rR` (reads rR; R = 8..=15)
pub fn mov_rax_from(r: u8) -> Vec<u8> {
    vec![0x4c, 0x89, 0xc0 | ((r & 7) << 3)]
}
/// `mov rR,rdx` (writes rR; R = 8..=15)
pub fn mov_to_from_rdx(r: u8) -> Vec<u8> {
    vec![0x49, 0x89, 0xd0 | (r & 7)]
}

impl Asm {
    /// `lea rR,[rip+d]` whose value is `IB + delta`, for R in 0..=7 (`lea_img` is for r8..r15)
    pub fn lea_img_low(&mut self, r: u8, delta: i32) -> &mut Self {
        assert!(r < 8);
        let end = self.here() as i64 + 7;
        let d = delta as i64 - end;
        self.raw(&[0x48, 0x8d, 0x05 | (r << 3)]).raw(&(d as i32).to_le_bytes())
    }
    /// `mov eX,dword [Bs + I*4 + t]` (registers by number 0..=15; SIB with a 32-bit displacement)
    pub fn mov_load(&mut self, x: u8, bs: u8, i: u8, t: u32) -> &mut Self {
        let rex = 0x40 | ((x >> 3) << 2) | ((i >> 3) << 1) | (bs >> 3);
        self.raw(&[rex, 0x8b, 0x84 | ((x & 7) << 3), 0x80 | ((i & 7) << 3) | (bs & 7)]).raw(&t.to_le_bytes())
    }
    /// `movsxd rX,dword [Bs + I*4 + t]`
    pub fn movsxd_load(&mut self, x: u8, bs: u8, i: u8, t: u32) -> &mut Self {
        let rex = 0x48 | ((x >> 3) << 2) | ((i >> 3) << 1) | (bs >> 3);
        self.raw(&[rex, 0x63, 0x84 | ((x & 7) << 3), 0x80 | ((i & 7) << 3) | (bs & 7)]).raw(&t.to_le_bytes())
    }
    /// `add rD,rS`
    pub fn add_rr(&mut self, d: u8, s: u8) -> &mut Self {
        self.raw(&[0x48 | ((s >> 3) << 2) | (d >> 3), 0x01, 0xc0 | ((s & 7) << 3) | (d & 7)])
    }
    /// `jmp rR`
    pub fn jmp_reg(&mut self, r: u8) -> &mut Self {
        if r >= 8 {
            self.raw(&[0x41]);
        }
        self.raw(&[0xff, 0xe0 | (r & 7)])
    }
    /// `cmp rR,imm` / `cmp eR,imm` (R = 1 rcx, 2 rdx)
    pub fn cmp_imm(&mut self, wide: bool, r: u8, imm: u32) -> &mut Self {
        if wide {
            self.raw(&[0x48]);
        }
        if imm < 0x80 {
            self.raw(&[0x83, 0xf8 | r, imm as u8])
        } else {
            self.raw(&[0x81, 0xf8 | r]).raw(&imm.to_le_bytes())
        }
    }
    pub fn mov_eax_k(&mut self, k: u32) -> &mut Self {
        self.mov_eax(k)
    }
}

impl Swp {
    /// The shape of Blend2D `f_489f7d0`: B = r9, X = rax, I = rcx, `cmp rcx,0x10; jbe tbl` (17 entries), a 12-byte `nop` in front
    /// of the table block, `lea B` first in the table block, `add rdx,rcx; mov rax,r11` between the `add` and the `jmp`
    pub fn real() -> Swp {
        Swp {
            base: 9,
            x: 0,
            n: 16,
            guard: SwGuard::JbeTaken,
            lea: SwLea::InTable,
            idiom: SwIdiom::Swapped,
            gap_before: Vec::new(),
            gap_after: vec![ADD_RDX_RCX.to_vec(), MOV_RAX_R11.to_vec()],
            pad: vec![NOP_12.to_vec()],
            bad: Bad::None,
            push_pop: false,
            target2: SwTarget::Plain,
            def: SwDef::Plain,
            foreign: false,
            no_pdata: false,
        }
    }
}

/// The function at `at` with the swapped switch described by `o`, its table (N+1 entries, then one entry that points into
/// another function and is never read), a helper, the other function and main. Blocks `t0`, `t1`, `t2` return 0x100 + k, `def`
/// returns 0xdead. Labels: `guard` (the `cmp`), `tbl` and `idiom` (the table block), `padl` (the first padding instruction), `load`,
/// `add`, `jmp`, `def`. Entry k of the table is `t[[0,1,0,2,1,2][k % 6]]` (`built.table`, N+1 entries read).
pub fn build_swapped(at: u32, o: &Swp) -> Built {
    use SwGuard::*;
    let (b, x) = (o.base, o.x);
    let mut a = Asm::new(at);
    let epilogue = |a: &mut Asm| {
        if o.push_pop {
            a.pop(b);
        }
        a.ret();
    };
    if o.push_pop {
        a.push(b);
    }
    match o.lea {
        SwLea::AtEntry => {
            a.lea_img(b, 0);
        }
        SwLea::AtEntryThenCall => {
            a.lea_img(b, 0).call_rva(HELPER);
        }
        SwLea::AtEntryOrArg => {
            a.lea_img(b, 0).raw(TEST_EDX).jcc8(0x74, "eoa").raw(&mov_to_from_rdx(b)).label("eoa");
        }
        _ => {}
    }
    if o.guard == ExtraBranch {
        a.raw(TEST_EDX).jcc8(0x75, "tbl");
    }
    a.label("guard");
    let n = o.n;
    let (reg, wide, imm, op, taken) = match o.guard {
        JbeTaken | ExtraBranch | WriteBetween | WriteAfter | DefFallsIn | NoCmp | CmpReg => (1, true, n, 0x76, true),
        JaFall => (1, true, n, 0x77, false),
        JaFall32 => (1, false, n, 0x77, false),
        JaeFall => (1, true, n + 1, 0x73, false),
        JbTaken => (1, true, n + 1, 0x72, true),
        JaTaken => (1, true, n, 0x77, true),
        JbeFall => (1, true, n, 0x76, false),
        JaeTaken => (1, true, n + 1, 0x73, true),
        JbFall => (1, true, n + 1, 0x72, false),
        OtherReg => (2, true, n, 0x76, true),
    };
    match o.guard {
        NoCmp => {
            a.raw(&[0x48, 0x85, 0xc9]);
        }
        CmpReg => {
            a.raw(&[0x48, 0x39, 0xd1]);
        }
        _ => {
            a.cmp_imm(wide, reg, imm);
        }
    }
    if o.guard == WriteBetween {
        a.raw(MOV_ECX_ECX);
    }
    a.jcc8(op, if taken { "tbl" } else { "def" });
    let def_block = |a: &mut Asm| {
        a.label("def");
        a.mov_eax(0xdead);
        match o.def {
            SwDef::Plain => {}
            SwDef::WriteThenRet | SwDef::WriteThenJmpRdx => {
                a.raw(&mov_to_from_rdx(b));
            }
        }
        if o.def == SwDef::WriteThenJmpRdx {
            a.jmp_rdx();
        } else if o.guard != DefFallsIn {
            epilogue(a);
        }
    };
    if taken {
        def_block(&mut a);
    }
    a.label("padl");
    for p in &o.pad {
        a.raw(p);
    }
    a.label("tbl").label("idiom");
    if o.guard == WriteAfter {
        a.raw(MOV_ECX_ECX);
    }
    match o.lea {
        SwLea::InTable => {
            a.lea_img(b, 0);
        }
        SwLea::Wrong(d) => {
            a.lea_img(b, d);
        }
        _ => {}
    }
    let other = if b == 10 { 13 } else { 10 };
    let gaps = |a: &mut Asm, g: &Vec<Vec<u8>>| {
        for i in g {
            a.raw(i);
        }
    };
    match o.idiom {
        SwIdiom::Swapped => {
            a.label("load").mov_load(x, b, 1, TABLE);
            gaps(&mut a, &o.gap_before);
            a.label("add").add_rr(b, x);
            gaps(&mut a, &o.gap_after);
            a.label("jmp").jmp_reg(b);
        }
        SwIdiom::LoadBaseOther => {
            a.label("load").mov_load(x, other, 1, TABLE);
            gaps(&mut a, &o.gap_before);
            a.label("add").add_rr(b, x);
            gaps(&mut a, &o.gap_after);
            a.label("jmp").jmp_reg(b);
        }
        SwIdiom::JmpOther => {
            a.label("load").mov_load(x, b, 1, TABLE);
            a.label("add").add_rr(other, x);
            a.label("jmp").jmp_reg(other);
        }
        SwIdiom::Movsxd => {
            a.label("load").movsxd_load(x, b, 1, TABLE);
            a.label("add").add_rr(b, x);
            a.label("jmp").jmp_reg(b);
        }
        SwIdiom::F2 => {
            // mov ecx,[rcx+rax*4+T]; lea rax,[ImageBase]; add rcx,rax; jmp rcx
            a.label("load").mov_load(1, 1, 0, TABLE);
            a.lea_img_low(0, 0);
            a.label("add").add_rr(1, 0);
            a.label("jmp").jmp_reg(1);
        }
        SwIdiom::XIsIndex => {
            a.label("load").mov_load(1, b, 1, TABLE);
            a.label("add").add_rr(b, 1);
            a.label("jmp").jmp_reg(b);
        }
        SwIdiom::TwoStage => {
            // movzx edx,byte [rcx + B + BYTES]
            a.raw(&[0x40 | ((b >> 3) << 1), 0x0f, 0xb6, 0x94, ((b & 7) << 3) | 1]).raw(&BYTES.to_le_bytes());
            a.label("load").mov_load(x, b, 2, TABLE);
            a.label("add").add_rr(b, x);
            a.label("jmp").jmp_reg(b);
        }
    }
    for (k, l) in ["t0", "t1", "t2"].into_iter().enumerate() {
        a.label(l);
        if l == "t2" {
            match o.target2 {
                SwTarget::Plain => {
                    a.mov_eax(0x100 + k as u32);
                    epilogue(&mut a);
                }
                SwTarget::LoopToGuard => {
                    a.jmp8("guard");
                }
                SwTarget::JmpToLoad => {
                    a.jmp8("load");
                }
                SwTarget::JmpToAdd => {
                    a.jmp8("add");
                }
                SwTarget::JmpToPad => {
                    a.jmp8("padl");
                }
                SwTarget::JmpRdx => {
                    a.jmp_rdx();
                }
            }
        } else {
            a.mov_eax(0x100 + k as u32);
            epilogue(&mut a);
        }
    }
    if !taken {
        def_block(&mut a);
    }
    let (code, labels) = a.finish();
    let end = at + code.len() as u32;
    let t = |l: &str| labels[l];
    let mut img = Img { entry: MAIN, no_pdata: o.no_pdata, ..Default::default() };
    let bad1 = match o.bad {
        Bad::None => t("t1"),
        Bad::OtherFunction => OTHER_FN,
        Bad::NotExecutable => RDATA_RVA + 0x200,
        Bad::Misaligned => t("t2") + 1,
        Bad::ChainedPiece => PIECE2,
    };
    let entries = n as usize + 1;
    let pattern = [0usize, 1, 0, 2, 1, 2];
    let names = [t("t0"), t("t1"), t("t2")];
    let mut table: Vec<u32> = (0..entries).map(|k| names[pattern[k % 6]]).collect();
    table[1] = bad1;
    // the entry after the last one: never read (another function)
    let mut raw = table.clone();
    raw.push(OTHER_FN);
    img.rdata.push((TABLE, u32s(&raw)));
    let mut byte_table = Vec::new();
    if o.idiom == SwIdiom::TwoStage {
        byte_table = (0..entries).map(|k| (k % 3) as u8).collect();
        img.rdata.push((BYTES, byte_table.clone()));
        // the RVA table of a two-stage form holds the three targets
        img.rdata.push((TABLE, u32s(&[t("t0"), t("t1"), t("t2"), OTHER_FN])));
    }
    img.text.push((at, code));
    if o.foreign {
        let mut j = Asm::new(OTHER_FN);
        j.jmp_rva(t("t0"));
        img.text.push((OTHER_FN, j.code));
        img.pdata.push((OTHER_FN, OTHER_FN + 5, UNW_PLAIN));
    } else {
        img.text.push((OTHER_FN, vec![0xb8, 0x77, 0, 0, 0, 0xc3]));
        img.pdata.push((OTHER_FN, OTHER_FN + 6, UNW_PLAIN));
    }
    img.text.push((HELPER, vec![0xc3]));
    let mut m = Asm::new(MAIN);
    m.call_rva(at).call_rva(OTHER_FN).ret();
    img.text.push((MAIN, m.code));
    img.pdata.push((at, end, UNW_PLAIN));
    img.pdata.push((HELPER, HELPER + 1, UNW_PLAIN));
    img.pdata.push((MAIN, MAIN + 11, UNW_PLAIN));
    if o.bad == Bad::ChainedPiece {
        img.text.push((PIECE2, vec![0xb8, 0x03, 0x01, 0, 0, 0xc3]));
        img.pdata.push((PIECE2, PIECE2 + 6, UNW_CHAIN));
        img.chain_parent = Some((at, end, UNW_PLAIN));
    }
    img.pdata.sort();
    Built { img, labels, table, byte_table, entries, func: (at, end) }
}

// ---------------------------------------------------------------------------------------------
// The scheduled form of decision 0109: the standard idiom `mov ecx,[B+rax*4+T]; add rcx,B; jmp rcx` whose guard is not in front of
// it. `cmp R,N; <p>; ja def; <q>; [head:] <idiom>`, and optionally a second guard at the bottom of a loop (`t2`) that jumps to `head`.
// ---------------------------------------------------------------------------------------------

/// One instruction of the gap between the guard's branch and the idiom.
#[derive(Clone, Debug)]
pub enum SchQ {
    Raw(Vec<u8>),
    /// `lea B,[ImageBase]`
    Lea,
    /// `lea B,[IB + delta]`
    LeaAt(i32),
    /// `call HELPER` (a callee may write the checked register)
    Call,
}

/// The compare of the (first) guard: `cmp eax,N`, `cmp edx,N` (the index reaches the table through `movsxd rax,edx`) or `cmp rax,N`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchCmp {
    Eax,
    Edx,
    Rax,
}

/// What block `t2` (and `t1` when `Sch::t1_loop`) does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchT2 {
    /// `mov eax,0x102; ret`
    Plain,
    /// the bottom of a loop, a second guard: `sub edx,1; mov eax,edx; cmp eax,N2; <second_op> head; mov eax,0x102; ret`
    Loop,
    /// `sub edx,1; mov eax,edx; jmp head`: a way into the idiom that bounds nothing
    JmpHead,
    /// `sub edx,1; mov eax,edx; call HELPER; jbe head`: the flags of the branch come from nowhere
    NoCmp,
}

#[derive(Clone, Debug)]
pub struct Sch {
    /// B, a register number 8..=15
    pub base: u8,
    /// the bound of the first guard and of the second one (`SchT2::Loop`); the tables have `max(n, n2) + 1` entries
    pub n: u32,
    pub n2: u32,
    pub two_stage: bool,
    /// `lea B,[ImageBase]` at the function entry
    pub entry_lea: bool,
    pub cmp: SchCmp,
    /// `cmp; jbe tbl` (the idiom is the taken side; `def` comes first) instead of `cmp; ja def`
    pub taken: bool,
    /// instructions between the `cmp` and the branch (raw bytes)
    pub p: Vec<Vec<u8>>,
    /// a `call HELPER` right after the `cmp` (before `p`)
    pub p_call: bool,
    /// instructions between the branch and the idiom
    pub q: Vec<SchQ>,
    /// the second guard lands before `q[k]` (`k == q.len()`: on the idiom's first instruction)
    pub head: Option<usize>,
    pub t2: SchT2,
    pub t1_loop: bool,
    /// the opcode of the second guard's branch (0x76 `jbe`: taken is I <= N)
    pub second_op: u8,
    pub bad: Bad,
    pub push_pop: bool,
    /// the loop block also writes B (`mov B,rdx` after the `sub`): a non-`lea` definition on the second way into the idiom
    pub loop_clobbers_base: bool,
    /// block `t0` jumps to `q[k]` (an unconditional way into the gap) instead of returning
    pub t0_jump: Option<usize>,
    /// no compare and no branch: the gap and the idiom follow the function's first instructions (the walk back reaches the entry)
    pub unguarded: bool,
}

pub const CDQE: &[u8] = &[0x48, 0x98];
pub const MOVSXD_RAX_EAX: &[u8] = &[0x48, 0x63, 0xc0];
pub const MOVSXD_RAX_EDX: &[u8] = &[0x48, 0x63, 0xc2];
pub const MOV_EAX_EDX: &[u8] = &[0x8b, 0xc2];
pub const MOV_RAX_RDX: &[u8] = &[0x48, 0x89, 0xd0];
/// `lea eax,[rax+1]`
pub const LEA_EAX_EAX_1: &[u8] = &[0x8d, 0x40, 0x01];
/// `mov qword [rsp-0x10],rdx`: a store below the stack pointer
pub const MOV_MEM_RSP_M16_RDX: &[u8] = &[0x48, 0x89, 0x54, 0x24, 0xf0];
/// `mov rcx,rax` (writes rcx, which the idiom writes anyway)
pub const MOV_RCX_RAX: &[u8] = &[0x48, 0x89, 0xc1];
/// `xor r9d,r9d` (writes r9 and the flags)
pub const XOR_R9D_R9D: &[u8] = &[0x45, 0x31, 0xc9];
/// `call HELPER` is built with `Asm::call_rva`
const SUB_EDX_1: &[u8] = &[0x83, 0xea, 0x01];
const LBL_Q: [&str; 8] = ["q0", "q1", "q2", "q3", "q4", "q5", "q6", "q7"];
const LBL_P: [&str; 4] = ["p0", "p1", "p2", "p3"];

impl Sch {
    /// The shape of freetype `f_4b166e0` (`FT_Stream_ReadFields`, site 0x4b1674d): B = r14, `cmp eax,0x15; <store>; ja def`, then
    /// `lea r14,[ImageBase]; nop; nop; cdqe`, the loop head before the `cdqe`, and a second guard `cmp eax,0x15; jbe head` at the
    /// bottom of the loop (`t2`)
    pub fn real() -> Sch {
        Sch {
            base: 14,
            n: 21,
            n2: 21,
            two_stage: false,
            entry_lea: false,
            cmp: SchCmp::Eax,
            taken: false,
            p: vec![MOV_MEM_RSP_M16_RDX.to_vec()],
            p_call: false,
            q: vec![SchQ::Lea, SchQ::Raw(NOP_3.to_vec()), SchQ::Raw(NOP_4.to_vec()), SchQ::Raw(CDQE.to_vec())],
            head: Some(3),
            t2: SchT2::Loop,
            t1_loop: false,
            second_op: 0x76,
            bad: Bad::None,
            push_pop: true,
            loop_clobbers_base: false,
            t0_jump: None,
            unguarded: false,
        }
    }
    /// One guard, `lea B` at the entry, `cmp eax,N; <store>; ja def; cdqe` (a store between the `cmp` and the `ja` is what the standard form's
    /// walk back does not accept; with `p` empty this is read by the standard form)
    pub fn simple() -> Sch {
        Sch { n: 5, n2: 5, entry_lea: true, q: vec![SchQ::Raw(CDQE.to_vec())], head: None, t2: SchT2::Plain, ..Sch::real() }
    }
    pub fn entries(&self) -> usize {
        let n = if self.t2 == SchT2::Loop || self.t1_loop { self.n.max(self.n2) } else { self.n };
        n as usize + 1
    }
}

/// The function at `at` with the scheduled switch described by `o`. Labels: `guard` (the `cmp`), `br` (the branch), `tbl` (the first
/// instruction after the branch on the idiom's side), `q0`.., `head`, `idiom` (the first instruction of the idiom), `load`, `add`,
/// `jmp`, `def`, `t0`, `t1`, `t2`. Block k returns 0x100 + k, `def` returns 0xdead. The index arrives in ecx.
pub fn build_sched(at: u32, o: &Sch) -> Built {
    let b = o.base;
    let mut a = Asm::new(at);
    let epilogue = |a: &mut Asm| {
        if o.push_pop {
            a.pop(b);
        }
        a.ret();
    };
    if o.push_pop {
        a.push(b);
    }
    if o.entry_lea {
        a.lea_img(b, 0);
    }
    if !o.unguarded {
        a.mov_eax_ecx().raw(&[0x8b, 0xd1]);
        a.label("guard");
        match o.cmp {
            SchCmp::Eax => a.cmp_imm(false, 0, o.n),
            SchCmp::Edx => a.cmp_imm(false, 2, o.n),
            SchCmp::Rax => a.cmp_imm(true, 0, o.n),
        };
        if o.p_call {
            a.call_rva(HELPER);
        }
        for (k, i) in o.p.iter().enumerate() {
            a.label(LBL_P[k]).raw(i);
        }
        a.label("br").jcc8(if o.taken { 0x76 } else { 0x77 }, if o.taken { "tbl" } else { "def" });
    }
    let def_block = |a: &mut Asm| {
        a.label("def").mov_eax(0xdead);
        epilogue(a);
    };
    if o.taken {
        def_block(&mut a);
    }
    a.label("tbl");
    for (k, q) in o.q.iter().enumerate() {
        if o.head == Some(k) {
            a.label("head");
        }
        a.label(LBL_Q[k]);
        match q {
            SchQ::Raw(r) => {
                a.raw(r);
            }
            SchQ::Lea => {
                a.lea_img(b, 0);
            }
            SchQ::LeaAt(d) => {
                a.lea_img(b, *d);
            }
            SchQ::Call => {
                a.call_rva(HELPER);
            }
        }
    }
    if o.head == Some(o.q.len()) {
        a.label("head");
    }
    a.label("idiom");
    if o.two_stage {
        a.movzx_eax_byte(b, BYTES);
    }
    a.label("load").mov_ecx_table(b, TABLE);
    a.label("add").add_rcx_b(b);
    a.label("jmp").jmp_rcx();
    for (k, l) in ["t0", "t1", "t2"].into_iter().enumerate() {
        a.label(l);
        if let (0, Some(j)) = (k, o.t0_jump) {
            a.jmp8(LBL_Q[j]);
            continue;
        }
        let lp = (l == "t2" && o.t2 != SchT2::Plain) || (l == "t1" && o.t1_loop);
        if !lp {
            a.mov_eax(0x100 + k as u32);
            epilogue(&mut a);
            continue;
        }
        a.raw(SUB_EDX_1).raw(MOV_EAX_EDX);
        if o.loop_clobbers_base {
            a.raw(&mov_to_from_rdx(b));
        }
        match o.t2 {
            SchT2::JmpHead => {
                a.jmp8("head");
            }
            SchT2::NoCmp => {
                a.call_rva(HELPER).jcc8(0x76, "head");
            }
            _ => {
                a.cmp_imm(false, 0, o.n2).label(if l == "t2" { "br2" } else { "br1" }).jcc8(o.second_op, "head");
            }
        }
        a.mov_eax(0x100 + k as u32);
        epilogue(&mut a);
    }
    if !o.taken {
        def_block(&mut a);
    }
    let (code, labels) = a.finish();
    let end = at + code.len() as u32;
    let t = |l: &str| labels[l];
    let mut img = Img { entry: MAIN, ..Default::default() };
    let bad1 = match o.bad {
        Bad::None => t("t1"),
        Bad::OtherFunction => OTHER_FN,
        Bad::NotExecutable => RDATA_RVA + 0x200,
        Bad::Misaligned => t("t2") + 1,
        Bad::ChainedPiece => PIECE2,
    };
    let entries = o.entries();
    let pattern = [0usize, 1, 0, 2, 1, 2];
    let names = [t("t0"), t("t1"), t("t2")];
    let mut table: Vec<u32> = (0..entries).map(|k| names[pattern[k % 6]]).collect();
    table[1] = bad1;
    let mut raw = table.clone();
    raw.push(OTHER_FN);
    img.rdata.push((TABLE, u32s(&raw)));
    let mut byte_table = Vec::new();
    if o.two_stage {
        byte_table = (0..entries).map(|k| (k % 3) as u8).collect();
        img.rdata.push((BYTES, byte_table.clone()));
        img.rdata.push((TABLE, u32s(&[t("t0"), t("t1"), t("t2"), OTHER_FN])));
    }
    img.text.push((at, code));
    img.text.push((OTHER_FN, vec![0xb8, 0x77, 0, 0, 0, 0xc3]));
    img.pdata.push((OTHER_FN, OTHER_FN + 6, UNW_PLAIN));
    img.text.push((HELPER, vec![0xc3]));
    let mut m = Asm::new(MAIN);
    m.call_rva(at).call_rva(OTHER_FN).ret();
    img.text.push((MAIN, m.code));
    img.pdata.push((at, end, UNW_PLAIN));
    img.pdata.push((HELPER, HELPER + 1, UNW_PLAIN));
    img.pdata.push((MAIN, MAIN + 11, UNW_PLAIN));
    if o.bad == Bad::ChainedPiece {
        img.text.push((PIECE2, vec![0xb8, 0x03, 0x01, 0, 0, 0xc3]));
        img.pdata.push((PIECE2, PIECE2 + 6, UNW_CHAIN));
        img.chain_parent = Some((at, end, UNW_PLAIN));
    }
    img.pdata.sort();
    Built { img, labels, table, byte_table, entries, func: (at, end) }
}
