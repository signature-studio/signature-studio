//! Synthetic PE32+ image for the inline absolute jump table (decision 0102 W3, `recognize_inline` in fe-decode/src/jumptab.rs; the builder is
//! the one of decision 0103, whose own recogniser `recognize_abs` was not taken in the merge: the facts it checked are checked here against
//! the inline-table mechanism). Shared by fe-decode/tests/inline_run.rs and fe-lift/tests/wall_q_lift.rs (`#[path]`).
//!
//! ImageBase `IB` = 0x1_4000_0000. Sections: .text RVA 0x1000 (raw 0x400), .xdata 0x2000 (raw 0x800), .pdata 0x3000 (raw 0x900), .reloc
//! 0x4000 (raw 0xa00). The function F at 0x1000..0x1080 (one RUNTIME_FUNCTION) is the shape of the memset tail of FLEngine
//! (0x40d210) and quickfontcache_x64.dll (0x40c120):
//!
//! ```text
//! 0x1000  lea r10,[rip+5]            4c 8d 15 05 00 00 00     r10 = 0x100c = the end of the jump
//! 0x1007  jmp qword [r10+rdx*8+0x10] 41 ff 64 d2 10          slot of rdx = 0 is r10 + 0x10 = 0x101c
//! 0x100c  table: three DIR64 pointers to T0, T1, T2 (0x1040, 0x1050, 0x1060), then a fourth slot (0x1024)
//! 0x1040  T0: mov eax,1; ret         0x1050  T1: mov eax,2; ret         0x1060  T2: mov eax,3; ret
//! 0x1100  H: mov eax,9; ret          (a second RUNTIME_FUNCTION, outside F)
//! ```
#![allow(dead_code)]

pub const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT_SIZE: usize = 240;
const SEC_TABLE: usize = LFANEW + 24 + OPT_SIZE;
pub const FILE_LEN: usize = 0xb00;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

#[derive(Clone, Copy, Default)]
pub struct Variant {
    /// the `lea` loads 0x1000 + 7 + 0x25 (an address inside the function, 0x102c, with no relocated slot) instead of the end of the jump:
    /// no table is found there
    pub far_lea: bool,
    /// the first slot of the table is not in the relocation directory
    pub first_unrelocated: bool,
    /// the third slot points at H (another function)
    pub third_outside: bool,
    /// the fourth slot (0x1024) is a relocated pointer to T0
    pub fourth_relocated: bool,
    /// the `jmp` has no `lea` in front of it (a `nop` stands there instead)
    pub no_lea: bool,
}

pub fn build(v: Variant) -> Vec<u8> {
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
    w32(&mut b, o + 16, 0x1000);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x5000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 112 + 8 * 3, 0x3000);
    w32(&mut b, o + 116 + 8 * 3, 24);
    w32(&mut b, o + 112 + 8 * 5, 0x4000);
    w32(&mut b, o + 116 + 8 * 5, 0x14);
    let sec = |b: &mut Vec<u8>, i: usize, name: &[u8], rva: u32, size: u32, raw: u32, flags: u32| {
        let s = SEC_TABLE + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(b, s + 8, size);
        w32(b, s + 12, rva);
        w32(b, s + 16, size);
        w32(b, s + 20, raw);
        w32(b, s + 36, flags);
    };
    sec(&mut b, 0, b".text", 0x1000, 0x400, 0x400, 0x6000_0020);
    sec(&mut b, 1, b".xdata", 0x2000, 0x100, 0x800, 0x4000_0040);
    sec(&mut b, 2, b".pdata", 0x3000, 0x100, 0x900, 0x4000_0040);
    sec(&mut b, 3, b".reloc", 0x4000, 0x100, 0xa00, 0x4200_0040);
    let t = |rva: u32| 0x400 + (rva - 0x1000) as usize;
    let put = |b: &mut Vec<u8>, rva: u32, bytes: &[u8]| {
        let at = t(rva);
        b[at..at + bytes.len()].copy_from_slice(bytes);
    };
    if v.no_lea {
        put(&mut b, 0x1000, &[0x90; 7]);
    } else {
        let disp: u8 = if v.far_lea { 0x25 } else { 5 };
        put(&mut b, 0x1000, &[0x4c, 0x8d, 0x15, disp, 0, 0, 0]);
    }
    put(&mut b, 0x1007, &[0x41, 0xff, 0x64, 0xd2, 0x10]);
    let third = if v.third_outside { 0x1100u64 } else { 0x1060 };
    let slots = [0x1040u64, 0x1050, third, 0x1040];
    for (k, rva) in slots.iter().enumerate() {
        put(&mut b, 0x100c + 8 * k as u32, &(IB + rva).to_le_bytes());
    }
    put(&mut b, 0x1040, &[0xb8, 1, 0, 0, 0, 0xc3]);
    put(&mut b, 0x1050, &[0xb8, 2, 0, 0, 0, 0xc3]);
    put(&mut b, 0x1060, &[0xb8, 3, 0, 0, 0, 0xc3]);
    put(&mut b, 0x1100, &[0xb8, 9, 0, 0, 0, 0xc3]);
    // .xdata: a plain UNWIND_INFO (version 1, no codes) at 0x2000
    b[0x800] = 1;
    // .pdata: F = [0x1000, 0x1080), H = [0x1100, 0x1110)
    for (k, (begin, end)) in [(0x1000u32, 0x1080u32), (0x1100, 0x1110)].iter().enumerate() {
        let at = 0x900 + 12 * k;
        w32(&mut b, at, *begin);
        w32(&mut b, at + 4, *end);
        w32(&mut b, at + 8, 0x2000);
    }
    // .reloc: one block for page 0x1000 with the DIR64 slots (type 10) of the table, padded with an ABSOLUTE entry
    let mut entries: Vec<u16> = Vec::new();
    for k in 0..4u32 {
        if (k == 0 && v.first_unrelocated) || (k == 3 && !v.fourth_relocated) {
            continue;
        }
        entries.push(0xA000 | (0x00c + 8 * k) as u16);
    }
    while entries.len() % 2 != 0 {
        entries.push(0);
    }
    let size = 8 + 2 * entries.len() as u32;
    w32(&mut b, 0xa00, 0x1000);
    w32(&mut b, 0xa04, size);
    for (k, e) in entries.iter().enumerate() {
        w16(&mut b, 0xa08 + 2 * k, *e);
    }
    w32(&mut b, o + 116 + 8 * 5, size);
    b
}
