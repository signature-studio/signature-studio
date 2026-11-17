//! Decision 0227 (hyb5, `--sse-int`): differential test of the packed-integer, shuffle and bit-move SSE forms that `fe-wasmgen` lowers to wasm v128 instructions.
//! One function per instruction form is lowered: a loop `L: movups xmm0,[rcx]; movups xmm1,[rcx+16]; <insn>; store; add rcx,32; add rdx,16; dec r8; jnz L; ret`. Node runs it over
//! N input pairs (random vectors, vectors whose lanes are drawn from the boundary values, every pair of byte values, every pair of boundary 16/32-bit values) and the result of
//! each is compared bit for bit with the host CPU's own result of the same instruction (core::arch). FE_SIMD_N sets the number of random vectors (default 100000; the logged run of
//! the decision used 1000000). The second test breaks the lowered code on purpose (one opcode, one shuffle lane or one constant of the wasm body is changed) and requires that the same
//! comparison finds a difference for every instruction form. The third test (ignored by default; `cargo test -p fe-wasmgen --release --test simd_int_forms -- --ignored`) runs every 16-bit
//! lane pair (all 2^32 ordered pairs) through the instructions whose lanes are 16 bits wide and compares a 64-bit digest of the result stream with the host CPU's. Skipped (and passing) when
//! node is missing or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::arch::x86_64::*;
use std::process::Command;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// A minimal PE32+ with .text (the code at RVA 0x1000) and .pdata (one function), image base 0x400000.
fn pe(code: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 0xc00];
    let (lf, opt) = (0x80usize, 0x80 + 24);
    let sec = opt + 240;
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, lf as u32);
    w32(&mut b, lf, 0x4550);
    let f = lf + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    w16(&mut b, opt, 0x20b);
    w32(&mut b, opt + 16, 0x1000);
    w64(&mut b, opt + 24, 0x400000);
    w32(&mut b, opt + 32, 0x1000);
    w32(&mut b, opt + 36, 0x200);
    w32(&mut b, opt + 56, 0x3000);
    w32(&mut b, opt + 60, 0x400);
    w16(&mut b, opt + 68, 3);
    w32(&mut b, opt + 108, 16);
    w32(&mut b, opt + 112 + 3 * 8, 0x2000);
    w32(&mut b, opt + 112 + 3 * 8 + 4, 12);
    let mut section = |i: usize, name: &[u8], vs: u32, rva: u32, rs: u32, rp: u32, ch: u32| {
        let s = sec + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(&mut b, s + 8, vs);
        w32(&mut b, s + 12, rva);
        w32(&mut b, s + 16, rs);
        w32(&mut b, s + 20, rp);
        w32(&mut b, s + 36, ch);
    };
    section(0, b".text", 0x400, 0x1000, 0x400, 0x400, 0x60000020);
    section(1, b".pdata", 0x200, 0x2000, 0x400, 0x800, 0x40000040);
    b[0x400..0x400 + code.len()].copy_from_slice(code);
    w32(&mut b, 0x800, 0x1000);
    w32(&mut b, 0x804, 0x1000 + code.len() as u32);
    w32(&mut b, 0x808, 0x2100);
    b[0x900..0x904].copy_from_slice(&[1, 0, 0, 0]);
    b
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// How the lowered wasm body is broken for the mutation test (same-length edits only: the section sizes stay valid).
#[derive(Clone, Copy)]
enum Mut {
    /// the SIMD opcode `from` becomes `to` (both of the same LEB length)
    Op(u32, u32),
    /// a lane index of the first i8x16.shuffle is changed (lane k xor 1)
    Shuf(usize),
    /// a byte of the first 16-byte v128.const that has `from` in lane 0 becomes `to`
    Const(u8, u8),
    /// the lane number of the first i16x8.replace_lane
    Lane,
}

struct Case {
    name: &'static str,
    /// code after the two loads (e.g. `mov eax,[rcx+16]`)
    pre: Vec<u8>,
    insn: Vec<u8>,
    xmm_out: bool,
    /// the 16-bit-lane forms of the exhaustive test: the host function is applied to all lanes
    lanes16: bool,
    want: Box<dyn Fn(__m128i, __m128i) -> [u8; 16]>,
    mutation: Mut,
}

fn to_bytes(v: __m128i) -> [u8; 16] {
    unsafe { std::mem::transmute(v) }
}
fn from_bytes(b: [u8; 16]) -> __m128i {
    unsafe { std::mem::transmute(b) }
}

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn leb(mut v: u32) -> Vec<u8> {
    let mut o = Vec::new();
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            o.push(b);
            return o;
        }
        o.push(b | 0x80);
    }
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

/// applies the mutation to the wasm bytes; false when it found nothing to change
fn mutate(w: &mut Vec<u8>, m: Mut) -> bool {
    match m {
        Mut::Op(from, to) => {
            let mut a = vec![0xfd];
            a.extend(leb(from));
            let mut b = vec![0xfd];
            b.extend(leb(to));
            assert_eq!(a.len(), b.len());
            match find(w, &a) {
                Some(p) => {
                    w[p..p + b.len()].copy_from_slice(&b);
                    true
                }
                None => false,
            }
        }
        Mut::Shuf(k) => match find(w, &[0xfd, 0x0d]) {
            Some(p) => {
                w[p + 2 + k] ^= 1;
                true
            }
            None => false,
        },
        Mut::Const(from, to) => {
            let mut p = 0;
            while let Some(q) = find(&w[p..], &[0xfd, 0x0c]) {
                let q = p + q;
                if w.len() >= q + 18 && w[q + 2] == from {
                    for k in 0..16 {
                        if w[q + 2 + k] == from {
                            w[q + 2 + k] = to;
                        }
                    }
                    return true;
                }
                p = q + 2;
            }
            false
        }
        Mut::Lane => match find(w, &[0xfd, 0x1a]) {
            Some(p) => {
                w[p + 2] ^= 1;
                true
            }
            None => false,
        },
    }
}

const S8: [u8; 12] = [0, 1, 2, 0x3f, 0x7e, 0x7f, 0x80, 0x81, 0xc0, 0xfe, 0xff, 0x55];
const S16: [u16; 24] = [
    0, 1, 2, 3, 0xff, 0x100, 0x101, 0x7f, 0x80, 0x7ffe, 0x7fff, 0x8000, 0x8001, 0xfffe, 0xffff, 0x1234, 0xedcc, 0x5555, 0xaaaa, 0x00ff, 0xff00, 0x4000, 0xc000, 0x8080,
];
const S32: [u32; 16] = [0, 1, 2, 31, 32, 33, 0x7fffffff, 0x80000000, 0x80000001, 0xffffffff, 0xfffffffe, 0x0000ffff, 0xffff0000, 0x00010001, 0x7fff7fff, 0x80008000];
const S64: [u64; 16] = [0, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 0x1_0000_0000, u64::MAX];

/// the inputs: boundary sets first (every pair of byte values, every pair of boundary 16-bit values, 32-bit values, shift counts), then `n` random vectors, half of them with lanes drawn from the boundary values
fn inputs(n: usize) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    // every ordered pair of byte values: 65536 pairs over 16 lanes
    for k in 0..4096usize {
        let mut b = [0u8; 32];
        for j in 0..16 {
            let p = k * 16 + j;
            b[j] = (p >> 8) as u8;
            b[16 + j] = p as u8;
        }
        v.push(b);
    }
    // the same pairs with the lanes rotated (a lane meets another neighbour)
    for k in 0..4096usize {
        let mut b = [0u8; 32];
        for j in 0..16 {
            let p = k * 16 + j;
            b[j] = (p >> 8) as u8;
            b[16 + (j + 5) % 16] = p as u8;
        }
        v.push(b);
    }
    // 16-bit boundary values: all pairs of the 24
    for k in 0..(24 * 24 / 8) {
        let mut b = [0u8; 32];
        for j in 0..8 {
            let p = k * 8 + j;
            b[2 * j..2 * j + 2].copy_from_slice(&S16[p / 24].to_le_bytes());
            b[16 + 2 * j..16 + 2 * j + 2].copy_from_slice(&S16[p % 24].to_le_bytes());
        }
        v.push(b);
    }
    // 32-bit boundary values: all pairs of the 16
    for k in 0..(16 * 16 / 4) {
        let mut b = [0u8; 32];
        for j in 0..4 {
            let p = k * 4 + j;
            b[4 * j..4 * j + 4].copy_from_slice(&S32[p / 16].to_le_bytes());
            b[16 + 4 * j..16 + 4 * j + 4].copy_from_slice(&S32[p % 16].to_le_bytes());
        }
        v.push(b);
    }
    let mut s = 0x9e3779b97f4a7c15u64;
    // shift counts: the low 64 bits of the second operand
    for c in S64 {
        for _ in 0..8 {
            let mut b = [0u8; 32];
            for k in 0..4 {
                b[8 * k..8 * k + 8].copy_from_slice(&rng(&mut s).to_le_bytes());
            }
            b[16..24].copy_from_slice(&c.to_le_bytes());
            v.push(b);
        }
    }
    for i in 0..n {
        let mut b = [0u8; 32];
        if i % 2 == 0 {
            for k in 0..4 {
                b[8 * k..8 * k + 8].copy_from_slice(&rng(&mut s).to_le_bytes());
            }
        } else {
            for l in 0..32 {
                let x = rng(&mut s);
                b[l] = match x % 3 {
                    0 => S8[((x >> 8) % 12) as usize],
                    1 => (S16[((x >> 8) % 24) as usize] >> (8 * ((x >> 20) & 1))) as u8,
                    _ => (x >> 16) as u8,
                };
            }
        }
        v.push(b);
    }
    v
}

// ---- the host side -------------------------------------------------------------------------------------------------------------

#[target_feature(enable = "ssse3")]
unsafe fn h_pshufb(a: __m128i, b: __m128i) -> __m128i {
    _mm_shuffle_epi8(a, b)
}
#[target_feature(enable = "ssse3")]
unsafe fn h_palignr<const N: i32>(a: __m128i, b: __m128i) -> __m128i {
    _mm_alignr_epi8::<N>(a, b)
}
#[target_feature(enable = "ssse3")]
unsafe fn h_pabsb(_a: __m128i, b: __m128i) -> __m128i {
    _mm_abs_epi8(b)
}
#[target_feature(enable = "ssse3")]
unsafe fn h_pabsw(_a: __m128i, b: __m128i) -> __m128i {
    _mm_abs_epi16(b)
}
#[target_feature(enable = "ssse3")]
unsafe fn h_pabsd(_a: __m128i, b: __m128i) -> __m128i {
    _mm_abs_epi32(b)
}

macro_rules! c {
    ($name:expr, $insn:expr, $mut:expr, $f:expr) => {
        Case { name: $name, pre: vec![], insn: $insn.to_vec(), xmm_out: true, lanes16: false, mutation: $mut, want: Box::new(|a, b| to_bytes(unsafe { $f(a, b) })) }
    };
}
macro_rules! c16 {
    ($name:expr, $insn:expr, $mut:expr, $f:expr) => {
        Case { name: $name, pre: vec![], insn: $insn.to_vec(), xmm_out: true, lanes16: true, mutation: $mut, want: Box::new(|a, b| to_bytes(unsafe { $f(a, b) })) }
    };
}

fn cases() -> Vec<Case> {
    use Mut::*;
    let mut v: Vec<Case> = Vec::new();
    #[allow(unused_unsafe)]
    unsafe {
        // ---- saturating, multiply, average, min/max, sum of absolute differences (new in decision 0227)
        v.push(c!("paddusb", [0x66, 0x0f, 0xdc, 0xc1], Op(0x70, 0x6f), |a, b| _mm_adds_epu8(a, b)));
        v.push(c16!("paddusw", [0x66, 0x0f, 0xdd, 0xc1], Op(0x90, 0x8f), |a, b| _mm_adds_epu16(a, b)));
        v.push(c!("psubusb", [0x66, 0x0f, 0xd8, 0xc1], Op(0x73, 0x72), |a, b| _mm_subs_epu8(a, b)));
        v.push(c16!("psubusw", [0x66, 0x0f, 0xd9, 0xc1], Op(0x93, 0x92), |a, b| _mm_subs_epu16(a, b)));
        v.push(c!("paddsb", [0x66, 0x0f, 0xec, 0xc1], Op(0x6f, 0x70), |a, b| _mm_adds_epi8(a, b)));
        v.push(c16!("paddsw", [0x66, 0x0f, 0xed, 0xc1], Op(0x8f, 0x90), |a, b| _mm_adds_epi16(a, b)));
        v.push(c!("psubsb", [0x66, 0x0f, 0xe8, 0xc1], Op(0x72, 0x73), |a, b| _mm_subs_epi8(a, b)));
        v.push(c16!("psubsw", [0x66, 0x0f, 0xe9, 0xc1], Op(0x92, 0x93), |a, b| _mm_subs_epi16(a, b)));
        v.push(c16!("pmullw", [0x66, 0x0f, 0xd5, 0xc1], Op(0x95, 0x91), |a, b| _mm_mullo_epi16(a, b)));
        v.push(c16!("pmulhw", [0x66, 0x0f, 0xe5, 0xc1], Op(0xbc, 0xbe), |a, b| _mm_mulhi_epi16(a, b)));
        v.push(c16!("pmulhuw", [0x66, 0x0f, 0xe4, 0xc1], Op(0xbe, 0xbc), |a, b| _mm_mulhi_epu16(a, b)));
        v.push(c!("pmaddwd", [0x66, 0x0f, 0xf5, 0xc1], Op(0xba, 0xb5), |a, b| _mm_madd_epi16(a, b)));
        v.push(c!("pavgb", [0x66, 0x0f, 0xe0, 0xc1], Op(0x7b, 0x77), |a, b| _mm_avg_epu8(a, b)));
        v.push(c16!("pavgw", [0x66, 0x0f, 0xe3, 0xc1], Op(0x9b, 0x97), |a, b| _mm_avg_epu16(a, b)));
        v.push(c!("pminub", [0x66, 0x0f, 0xda, 0xc1], Op(0x77, 0x79), |a, b| _mm_min_epu8(a, b)));
        v.push(c!("pmaxub", [0x66, 0x0f, 0xde, 0xc1], Op(0x79, 0x77), |a, b| _mm_max_epu8(a, b)));
        v.push(c16!("pminsw", [0x66, 0x0f, 0xea, 0xc1], Op(0x96, 0x98), |a, b| _mm_min_epi16(a, b)));
        v.push(c16!("pmaxsw", [0x66, 0x0f, 0xee, 0xc1], Op(0x98, 0x96), |a, b| _mm_max_epi16(a, b)));
        v.push(c!("psadbw", [0x66, 0x0f, 0xf6, 0xc1], Op(0x73, 0x71), |a, b| _mm_sad_epu8(a, b)));
        // memory sources ([rcx+16] holds the second input)
        v.push(c!("psubusb m128", [0x66, 0x0f, 0xd8, 0x41, 0x10], Op(0x73, 0x72), |a, b| _mm_subs_epu8(a, b)));
        v.push(c!("pmulhw m128", [0x66, 0x0f, 0xe5, 0x41, 0x10], Op(0xbc, 0xbe), |a, b| _mm_mulhi_epi16(a, b)));
        v.push(c!("pmulhuw m128", [0x66, 0x0f, 0xe4, 0x41, 0x10], Op(0xbe, 0xbc), |a, b| _mm_mulhi_epu16(a, b)));
        v.push(c!("psadbw m128", [0x66, 0x0f, 0xf6, 0x41, 0x10], Op(0x73, 0x71), |a, b| _mm_sad_epu8(a, b)));
        v.push(c!("pavgb m128", [0x66, 0x0f, 0xe0, 0x41, 0x10], Op(0x7b, 0x77), |a, b| _mm_avg_epu8(a, b)));
        v.push(c!("paddusb m128", [0x66, 0x0f, 0xdc, 0x41, 0x10], Op(0x70, 0x6f), |a, b| _mm_adds_epu8(a, b)));
        // 64-bit shifts by an xmm count (the low 64 bits of the source) and by an immediate
        v.push(c!("psrlq xmm", [0x66, 0x0f, 0xd3, 0xc1], Op(0xcd, 0xcb), |a, b| _mm_srl_epi64(a, b)));
        v.push(c!("psllq xmm", [0x66, 0x0f, 0xf3, 0xc1], Op(0xcb, 0xcd), |a, b| _mm_sll_epi64(a, b)));
        v.push(c!("psrlq 1", [0x66, 0x0f, 0x73, 0xd0, 0x01], Op(0xcd, 0xcb), |a, _b| _mm_srli_epi64::<1>(a)));
        v.push(c!("psrlq 33", [0x66, 0x0f, 0x73, 0xd0, 0x21], Op(0xcd, 0xcb), |a, _b| _mm_srli_epi64::<33>(a)));
        v.push(c!("psrlq 64", [0x66, 0x0f, 0x73, 0xd0, 0x40], Mut::Const(0, 1), |a, _b| _mm_srli_epi64::<64>(a)));
        v.push(c!("psllq 7", [0x66, 0x0f, 0x73, 0xf0, 0x07], Op(0xcb, 0xcd), |a, _b| _mm_slli_epi64::<7>(a)));
        v.push(c!("psllq 63", [0x66, 0x0f, 0x73, 0xf0, 0x3f], Op(0xcb, 0xcd), |a, _b| _mm_slli_epi64::<63>(a)));
        v.push(c!("psllq 200", [0x66, 0x0f, 0x73, 0xf0, 0xc8], Mut::Const(0, 1), |a, _b| _mm_slli_epi64::<200>(a)));
        // memory sources of the forms that were reached only through a register before ([rcx+16] holds the second input)
        v.push(c!("psrlq m128", [0x66, 0x0f, 0xd3, 0x41, 0x10], Op(0xcd, 0xcb), |a, b| _mm_srl_epi64(a, b)));
        v.push(c!("pslld m128", [0x66, 0x0f, 0xf2, 0x41, 0x10], Op(0xab, 0xad), |a, b| _mm_sll_epi32(a, b)));
        v.push(c!("pshufd m128", [0x66, 0x0f, 0x70, 0x41, 0x10, 0x1b], Shuf(0), |_a, b| _mm_shuffle_epi32::<0x1b>(b)));
        v.push(c!("packuswb m128", [0x66, 0x0f, 0x67, 0x41, 0x10], Op(0x66, 0x65), |a, b| _mm_packus_epi16(a, b)));
        v.push(c!("pmaddwd m128", [0x66, 0x0f, 0xf5, 0x41, 0x10], Op(0xba, 0xb5), |a, b| _mm_madd_epi16(a, b)));
        v.push(c!("punpcklwd m128", [0x66, 0x0f, 0x61, 0x41, 0x10], Shuf(2), |a, b| _mm_unpacklo_epi16(a, b)));
        // pinsrw: from a general register (eax loaded from [rcx+16]) and from memory
        let mut p = c!("pinsrw eax 0", [0x66, 0x0f, 0xc4, 0xc0, 0x00], Lane, |a, b| _mm_insert_epi16::<0>(a, *(&b as *const __m128i as *const i32)));
        p.pre = vec![0x8b, 0x41, 0x10];
        v.push(p);
        let mut p = c!("pinsrw eax 5", [0x66, 0x0f, 0xc4, 0xc0, 0x05], Lane, |a, b| _mm_insert_epi16::<5>(a, *(&b as *const __m128i as *const i32)));
        p.pre = vec![0x8b, 0x41, 0x10];
        v.push(p);
        v.push(c!("pinsrw m16 3", [0x66, 0x0f, 0xc4, 0x41, 0x10, 0x03], Lane, |a, b| _mm_insert_epi16::<3>(a, *(&b as *const __m128i as *const u16) as i32)));
        v.push(c!("pinsrw m16 15", [0x66, 0x0f, 0xc4, 0x41, 0x10, 0x0f], Lane, |a, b| _mm_insert_epi16::<7>(a, *(&b as *const __m128i as *const u16) as i32)));
        // SSSE3
        v.push(c!("pshufb", [0x66, 0x0f, 0x38, 0x00, 0xc1], Const(0x8f, 0xff), |a, b| h_pshufb(a, b)));
        v.push(c!("pshufb m128", [0x66, 0x0f, 0x38, 0x00, 0x41, 0x10], Const(0x8f, 0xff), |a, b| h_pshufb(a, b)));
        v.push(c!("palignr 3", [0x66, 0x0f, 0x3a, 0x0f, 0xc1, 0x03], Shuf(0), |a, b| h_palignr::<3>(a, b)));
        v.push(c!("palignr 8", [0x66, 0x0f, 0x3a, 0x0f, 0xc1, 0x08], Shuf(0), |a, b| h_palignr::<8>(a, b)));
        v.push(c!("palignr 16", [0x66, 0x0f, 0x3a, 0x0f, 0xc1, 0x10], Shuf(0), |a, b| h_palignr::<16>(a, b)));
        v.push(c!("palignr 21", [0x66, 0x0f, 0x3a, 0x0f, 0xc1, 0x15], Shuf(0), |a, b| h_palignr::<21>(a, b)));
        v.push(c!("palignr 32", [0x66, 0x0f, 0x3a, 0x0f, 0xc1, 0x20], Const(0, 1), |a, b| h_palignr::<32>(a, b)));
        v.push(c!("palignr m128 5", [0x66, 0x0f, 0x3a, 0x0f, 0x41, 0x10, 0x05], Shuf(0), |a, b| h_palignr::<5>(a, b)));
        v.push(c!("pabsb", [0x66, 0x0f, 0x38, 0x1c, 0xc1], Op(0x60, 0x61), |a, b| h_pabsb(a, b)));
        v.push(c16!("pabsw", [0x66, 0x0f, 0x38, 0x1d, 0xc1], Op(0x80, 0x81), |a, b| h_pabsw(a, b)));
        v.push(c!("pabsd", [0x66, 0x0f, 0x38, 0x1e, 0xc1], Op(0xa0, 0xa1), |a, b| h_pabsd(a, b)));
        // ---- the forms that decision 0181 added behind --mx-packed (andn, unpack, shuffle, half moves, integer compare, shifts, pack, pmuludq), now also behind --sse-int
        v.push(c!("pandn", [0x66, 0x0f, 0xdf, 0xc1], Op(0x4f, 0x4e), |a, b| _mm_andnot_si128(a, b)));
        v.push(c!("punpckldq", [0x66, 0x0f, 0x62, 0xc1], Shuf(4), |a, b| _mm_unpacklo_epi32(a, b)));
        v.push(c!("punpcklwd", [0x66, 0x0f, 0x61, 0xc1], Shuf(2), |a, b| _mm_unpacklo_epi16(a, b)));
        v.push(c!("punpckhdq", [0x66, 0x0f, 0x6a, 0xc1], Shuf(4), |a, b| _mm_unpackhi_epi32(a, b)));
        v.push(c!("punpckhwd", [0x66, 0x0f, 0x69, 0xc1], Shuf(2), |a, b| _mm_unpackhi_epi16(a, b)));
        v.push(c!("punpckhbw", [0x66, 0x0f, 0x68, 0xc1], Shuf(1), |a, b| _mm_unpackhi_epi8(a, b)));
        v.push(c!("pshufd 0x1b", [0x66, 0x0f, 0x70, 0xc1, 0x1b], Shuf(0), |_a, b| _mm_shuffle_epi32::<0x1b>(b)));
        v.push(c!("pshufd 0x4e", [0x66, 0x0f, 0x70, 0xc1, 0x4e], Shuf(0), |_a, b| _mm_shuffle_epi32::<0x4e>(b)));
        v.push(c!("pshufd 0x00", [0x66, 0x0f, 0x70, 0xc1, 0x00], Shuf(4), |_a, b| _mm_shuffle_epi32::<0x00>(b)));
        v.push(c!("pshuflw 0x1b", [0xf2, 0x0f, 0x70, 0xc1, 0x1b], Shuf(0), |_a, b| _mm_shufflelo_epi16::<0x1b>(b)));
        v.push(c!("pshuflw 0xe1", [0xf2, 0x0f, 0x70, 0xc1, 0xe1], Shuf(2), |_a, b| _mm_shufflelo_epi16::<0xe1>(b)));
        v.push(c!("pshufhw 0x1b", [0xf3, 0x0f, 0x70, 0xc1, 0x1b], Shuf(8), |_a, b| _mm_shufflehi_epi16::<0x1b>(b)));
        v.push(c!("pcmpeqb", [0x66, 0x0f, 0x74, 0xc1], Op(0x23, 0x24), |a, b| _mm_cmpeq_epi8(a, b)));
        v.push(c16!("pcmpeqw", [0x66, 0x0f, 0x75, 0xc1], Op(0x2d, 0x2e), |a, b| _mm_cmpeq_epi16(a, b)));
        v.push(c!("pcmpeqd", [0x66, 0x0f, 0x76, 0xc1], Op(0x37, 0x38), |a, b| _mm_cmpeq_epi32(a, b)));
        v.push(c!("pcmpgtb", [0x66, 0x0f, 0x64, 0xc1], Op(0x27, 0x28), |a, b| _mm_cmpgt_epi8(a, b)));
        v.push(c16!("pcmpgtw", [0x66, 0x0f, 0x65, 0xc1], Op(0x31, 0x32), |a, b| _mm_cmpgt_epi16(a, b)));
        v.push(c!("pcmpgtd", [0x66, 0x0f, 0x66, 0xc1], Op(0x3b, 0x3c), |a, b| _mm_cmpgt_epi32(a, b)));
        v.push(c!("psrlw 3", [0x66, 0x0f, 0x71, 0xd0, 0x03], Op(0x8d, 0x8c), |a, _b| _mm_srli_epi16::<3>(a)));
        v.push(c!("psrlw 16", [0x66, 0x0f, 0x71, 0xd0, 0x10], Const(0, 1), |a, _b| _mm_srli_epi16::<16>(a)));
        v.push(c!("psraw 5", [0x66, 0x0f, 0x71, 0xe0, 0x05], Op(0x8c, 0x8d), |a, _b| _mm_srai_epi16::<5>(a)));
        v.push(c!("psraw 40", [0x66, 0x0f, 0x71, 0xe0, 0x28], Op(0x8c, 0x8d), |a, _b| _mm_srai_epi16::<40>(a)));
        v.push(c!("psllw 7", [0x66, 0x0f, 0x71, 0xf0, 0x07], Op(0x8b, 0x8d), |a, _b| _mm_slli_epi16::<7>(a)));
        v.push(c!("psrld 3", [0x66, 0x0f, 0x72, 0xd0, 0x03], Op(0xad, 0xac), |a, _b| _mm_srli_epi32::<3>(a)));
        v.push(c!("psrad 9", [0x66, 0x0f, 0x72, 0xe0, 0x09], Op(0xac, 0xad), |a, _b| _mm_srai_epi32::<9>(a)));
        v.push(c!("pslld 13", [0x66, 0x0f, 0x72, 0xf0, 0x0d], Op(0xab, 0xad), |a, _b| _mm_slli_epi32::<13>(a)));
        v.push(c!("psrlw xmm", [0x66, 0x0f, 0xd1, 0xc1], Op(0x8d, 0x8c), |a, b| _mm_srl_epi16(a, b)));
        v.push(c!("psrld xmm", [0x66, 0x0f, 0xd2, 0xc1], Op(0xad, 0xac), |a, b| _mm_srl_epi32(a, b)));
        v.push(c!("psraw xmm", [0x66, 0x0f, 0xe1, 0xc1], Op(0x8c, 0x8d), |a, b| _mm_sra_epi16(a, b)));
        v.push(c!("psrad xmm", [0x66, 0x0f, 0xe2, 0xc1], Op(0xac, 0xad), |a, b| _mm_sra_epi32(a, b)));
        v.push(c!("psllw xmm", [0x66, 0x0f, 0xf1, 0xc1], Op(0x8b, 0x8d), |a, b| _mm_sll_epi16(a, b)));
        v.push(c!("pslld xmm", [0x66, 0x0f, 0xf2, 0xc1], Op(0xab, 0xad), |a, b| _mm_sll_epi32(a, b)));
        v.push(c!("packuswb", [0x66, 0x0f, 0x67, 0xc1], Op(0x66, 0x65), |a, b| _mm_packus_epi16(a, b)));
        v.push(c!("packsswb", [0x66, 0x0f, 0x63, 0xc1], Op(0x65, 0x66), |a, b| _mm_packs_epi16(a, b)));
        v.push(c!("packssdw", [0x66, 0x0f, 0x6b, 0xc1], Op(0x85, 0x86), |a, b| _mm_packs_epi32(a, b)));
        v.push(c!("pmuludq", [0x66, 0x0f, 0xf4, 0xc1], Op(0xd5, 0xd1), |a, b| _mm_mul_epu32(a, b)));
        v.push(c!("movlhps", [0x0f, 0x16, 0xc1], Shuf(8), |a, b| to_bytes_ps(_mm_movelh_ps(f(a), f(b)))));
        v.push(c!("movhlps", [0x0f, 0x12, 0xc1], Shuf(0), |a, b| to_bytes_ps(_mm_movehl_ps(f(a), f(b)))));
        v.push(c!("unpcklps", [0x0f, 0x14, 0xc1], Shuf(4), |a, b| to_bytes_ps(_mm_unpacklo_ps(f(a), f(b)))));
        v.push(c!("shufps 0x1b", [0x0f, 0xc6, 0xc1, 0x1b], Shuf(0), |a, b| to_bytes_ps(_mm_shuffle_ps::<0x1b>(f(a), f(b)))));
        v.push(c!("shufps 0xb1", [0x0f, 0xc6, 0xc1, 0xb1], Shuf(8), |a, b| to_bytes_ps(_mm_shuffle_ps::<0xb1>(f(a), f(b)))));
        v.push(c!("unpcklpd", [0x66, 0x0f, 0x14, 0xc1], Shuf(8), |a, b| to_bytes_pd(_mm_unpacklo_pd(d(a), d(b)))));
        v.push(c!("shufpd 1", [0x66, 0x0f, 0xc6, 0xc1, 0x01], Shuf(0), |a, b| to_bytes_pd(_mm_shuffle_pd::<1>(d(a), d(b)))));
        let gp = |name: &'static str, insn: &[u8], m: Mut, f: fn(__m128i, __m128i) -> u64| Case {
            name,
            pre: vec![],
            insn: insn.to_vec(),
            xmm_out: false,
            lanes16: false,
            mutation: m,
            want: Box::new(move |a, b| {
                let mut o = [0u8; 16];
                o[..8].copy_from_slice(&f(a, b).to_le_bytes());
                o
            }),
        };
        v.push(gp("movmskps", &[0x0f, 0x50, 0xc0], Op(0xa4, 0xc4), |a, _b| unsafe { _mm_movemask_ps(_mm_castsi128_ps(a)) as u32 as u64 }));
        v.push(gp("pmovmskb", &[0x66, 0x0f, 0xd7, 0xc0], Op(0x64, 0x63), |a, _b| unsafe { _mm_movemask_epi8(a) as u32 as u64 }));
        v.push(gp("pextrw 5", &[0x66, 0x0f, 0xc5, 0xc0, 0x05], Op(0x19, 0x18), |a, _b| unsafe { _mm_extract_epi16::<5>(a) as u32 as u64 }));
    }
    v
}

fn f(a: __m128i) -> __m128 {
    unsafe { _mm_castsi128_ps(a) }
}
fn d(a: __m128i) -> __m128d {
    unsafe { _mm_castsi128_pd(a) }
}
fn to_bytes_ps(a: __m128) -> __m128i {
    unsafe { _mm_castps_si128(a) }
}
fn to_bytes_pd(a: __m128d) -> __m128i {
    unsafe { _mm_castpd_si128(a) }
}

// ---- the node side -------------------------------------------------------------------------------------------------------------

fn code_of(c: &Case) -> Vec<u8> {
    let mut code: Vec<u8> = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10]; // movups xmm0,[rcx]; movups xmm1,[rcx+16]
    code.extend_from_slice(&c.pre);
    code.extend_from_slice(&c.insn);
    if c.xmm_out {
        code.extend_from_slice(&[0x0f, 0x11, 0x02]); // movups [rdx],xmm0
    } else {
        code.extend_from_slice(&[0x48, 0x89, 0x02]); // mov [rdx],rax
    }
    code.extend_from_slice(&[0x48, 0x83, 0xc1, 0x20, 0x48, 0x83, 0xc2, 0x10, 0x49, 0xff, 0xc8]); // add rcx,32; add rdx,16; dec r8
    let back = -(code.len() as i32 + 2);
    assert!(back >= -128);
    code.extend_from_slice(&[0x75, back as i8 as u8, 0xc3]); // jnz L; ret
    code
}

fn lower(c: &Case) -> Result<Vec<u8>, String> {
    let conf = Config { export_funcs: true, ..Config::default() };
    let comp = compile(&pe(&code_of(c)), &[], &[], &conf).map_err(|e| format!("{}: compile: {e}", c.name))?;
    if !comp.fidx.contains_key(&0x401000) {
        return Err(format!("{}: not lowered", c.name));
    }
    Ok(comp.wasm)
}

const DRIVER: &str = "const fs=require('fs');const [,,wasm,inp,outp]=process.argv;const mod=new WebAssembly.Module(fs.readFileSync(wasm));const data=fs.readFileSync(inp);const n=data.length/32;\
    const IN=0x100000,OUT=IN+32*n+0x1000;const mem=new WebAssembly.Memory({initial:Math.ceil((OUT+16*n+0x10000)/65536)});const inst=new WebAssembly.Instance(mod,{env:{mem}});\
    const u8=new Uint8Array(mem.buffer);u8.set(data,IN);inst.exports.f_401000(0x80000n,BigInt(IN),BigInt(OUT),BigInt(n),0n);fs.writeFileSync(outp,u8.subarray(OUT,OUT+16*n));";

struct Work {
    dir: std::path::PathBuf,
}

impl Work {
    fn new(tag: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("fe-wasmgen-simdint-{}-{}", tag, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("run.cjs"), DRIVER).unwrap();
        Work { dir }
    }
    fn input(&self, ins: &[[u8; 32]]) -> std::path::PathBuf {
        let mut flat: Vec<u8> = Vec::with_capacity(ins.len() * 32);
        for b in ins {
            flat.extend_from_slice(b);
        }
        let p = self.dir.join("in.bin");
        std::fs::write(&p, &flat).unwrap();
        p
    }
    fn run(&self, wasm: &[u8], inp: &std::path::Path) -> Result<Vec<u8>, String> {
        let w = self.dir.join("t.wasm");
        std::fs::write(&w, wasm).unwrap();
        if have("wasm-validate") {
            let o = Command::new("wasm-validate").arg(&w).output().unwrap();
            if !o.status.success() {
                return Err(format!("wasm-validate: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            }
        }
        let outp = self.dir.join("out.bin");
        let o = Command::new("node").arg(self.dir.join("run.cjs")).arg(&w).arg(inp).arg(&outp).output().unwrap();
        if !o.status.success() {
            return Err(format!("node: {}", String::from_utf8_lossy(&o.stderr).lines().filter(|l| l.contains("Error") || l.contains("error")).take(2).collect::<Vec<_>>().join(" / ")));
        }
        Ok(std::fs::read(&outp).unwrap())
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// number of inputs whose result differs from the host CPU's, and a description of the first
fn compare(c: &Case, ins: &[[u8; 32]], got: &[u8]) -> (usize, String) {
    let mut bad = 0;
    let mut first = String::new();
    for (k, b) in ins.iter().enumerate() {
        let a = from_bytes(b[..16].try_into().unwrap());
        let bb = from_bytes(b[16..].try_into().unwrap());
        let want = (c.want)(a, bb);
        let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
        let ok = if c.xmm_out { g == want } else { g[..8] == want[..8] };
        if !ok {
            bad += 1;
            if first.is_empty() {
                first = format!("in {:02x?} {:02x?} want {:02x?} got {:02x?}", &b[..16], &b[16..], want, g);
            }
        }
    }
    (bad, first)
}

fn n_random() -> usize {
    std::env::var("FE_SIMD_N").ok().and_then(|s| s.parse().ok()).unwrap_or(100_000)
}

#[test]
fn simd_int_forms_match_the_host_cpu() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs(n_random());
    let w = Work::new("m");
    let inp = w.input(&ins);
    let mut failures: Vec<String> = Vec::new();
    let cs = cases();
    for c in &cs {
        let wasm = match lower(c) {
            Ok(x) => x,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };
        match w.run(&wasm, &inp) {
            Ok(got) => {
                let (bad, first) = compare(c, &ins, &got);
                if bad > 0 {
                    failures.push(format!("{}: {bad} of {} inputs differ; first: {first}", c.name, ins.len()));
                } else {
                    eprintln!("simd_int {:<16} {} inputs identical to the host CPU", c.name, ins.len());
                }
            }
            Err(e) => failures.push(format!("{}: {e}", c.name)),
        }
    }
    eprintln!("simd_int_forms: {} instruction forms x {} inputs compared with the host CPU", cs.len(), ins.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn simd_int_mutants_are_detected() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs(20_000);
    let w = Work::new("x");
    let inp = w.input(&ins);
    let mut failures: Vec<String> = Vec::new();
    let cs = cases();
    for c in &cs {
        let mut wasm = match lower(c) {
            Ok(x) => x,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };
        if !mutate(&mut wasm, c.mutation) {
            failures.push(format!("{}: the mutation found nothing to change", c.name));
            continue;
        }
        match w.run(&wasm, &inp) {
            Ok(got) => {
                let (bad, _) = compare(c, &ins, &got);
                if bad == 0 {
                    failures.push(format!("{}: the broken lowering is NOT detected", c.name));
                } else {
                    eprintln!("simd_int mutant {:<16} detected by {bad} of {} inputs", c.name, ins.len());
                }
            }
            // a broken body that no longer validates is also a detection, but we want the runtime comparison: report it
            Err(e) => failures.push(format!("{}: mutant did not run: {e}", c.name)),
        }
    }
    eprintln!("simd_int_mutants: {} instruction forms, each lowered code broken once and the comparison run", cs.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ---- every 16-bit lane pair --------------------------------------------------------------------------------------------------

const DRIVER16: &str = "const fs=require('fs');const [,,wasm,aFrom,aTo]=process.argv;const mod=new WebAssembly.Module(fs.readFileSync(wasm));\
    const n=65536*8;const IN=0x100000,OUT=IN+32*n+0x1000;const mem=new WebAssembly.Memory({initial:Math.ceil((OUT+16*n+0x10000)/65536)});const inst=new WebAssembly.Instance(mod,{env:{mem}});\
    const u8=new Uint8Array(mem.buffer);const u16=new Uint16Array(mem.buffer);const u32=new Uint32Array(mem.buffer);\
    /* vector t of a batch: the b lanes j are j*8192+(t%8192) and the a lanes are all A0+floor(t/8192); the 64 vectors of the batch with the same t%8192 ... */\
    for(let t=0;t<n;t++){const bt=t&8191;for(let j=0;j<8;j++)u16[(IN+32*t+16)/2+j]=j*8192+bt;}\
    let h1=0x12345678|0,h2=0x9abcdef0|0;const K=0x9e3779b1|0,K2=0x85ebca6b|0;\
    for(let A=+aFrom;A<+aTo;A+=64){for(let t=0;t<n;t++){const A_=(A+(t>>13))&0xffff;const w=A_|(A_<<16);const o=(IN+32*t)/4;u32[o]=w;u32[o+1]=w;u32[o+2]=w;u32[o+3]=w;}\
    inst.exports.f_401000(0x80000n,BigInt(IN),BigInt(OUT),BigInt(n),0n);const r=OUT/4;\
    for(let i=0;i<4*n;i+=4){h1=Math.imul(h1^u32[r+i],K);h1^=h1>>>15;h1=Math.imul((h1+u32[r+i+1])|0,K2);h1^=h1>>>13;h2=Math.imul(h2^u32[r+i+2],K);h2^=h2>>>15;h2=Math.imul((h2+u32[r+i+3])|0,K2);h2^=h2>>>13;h1^=h2>>>7;}}\
    console.log((h1>>>0).toString(16).padStart(8,'0')+(h2>>>0).toString(16).padStart(8,'0'));";

/// the digest of the host CPU's result stream over the same pairs (same order as the driver; a = A0 + t/8192 in all lanes, b lane j = j*8192 + t%8192)
fn host_digest16(c: &Case, a_from: u32, a_to: u32) -> String {
    let n = 65536 * 8usize;
    let (mut h1, mut h2) = (0x12345678u32, 0x9abcdef0u32);
    let (k, k2) = (0x9e3779b1u32, 0x85ebca6bu32);
    let step = |h: u32, w0: u32, w1: u32| -> u32 {
        let mut h = (h ^ w0).wrapping_mul(k);
        h ^= h >> 15;
        h = h.wrapping_add(w1).wrapping_mul(k2);
        h ^ (h >> 13)
    };
    let mut bs: Vec<[u16; 8]> = Vec::with_capacity(8192);
    for bt in 0..8192u32 {
        let mut l = [0u16; 8];
        for (j, x) in l.iter_mut().enumerate() {
            *x = (j as u32 * 8192 + bt) as u16;
        }
        bs.push(l);
    }
    let mut a0 = a_from;
    while a0 < a_to {
        for t in 0..n {
            let aa = ((a0 + (t >> 13) as u32) & 0xffff) as u16;
            let av: [u16; 8] = [aa; 8];
            let a = unsafe { std::mem::transmute::<[u16; 8], __m128i>(av) };
            let b = unsafe { std::mem::transmute::<[u16; 8], __m128i>(bs[t & 8191]) };
            let r = (c.want)(a, b);
            let u = |i: usize| u32::from_le_bytes(r[4 * i..4 * i + 4].try_into().unwrap());
            h1 = step(h1, u(0), u(1));
            h2 = step(h2, u(2), u(3));
            h1 ^= h2 >> 7;
        }
        a0 += 64;
    }
    format!("{:08x}{:08x}", h1, h2)
}

#[test]
#[ignore]
fn simd_int_every_16bit_lane_pair_matches_the_host_cpu() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // FE_SIMD_A=from..to restricts the first operand values (multiples of 64; default all 65536)
    let (a_from, a_to) = match std::env::var("FE_SIMD_A") {
        Ok(s) => {
            let (x, y) = s.split_once("..").unwrap();
            (x.parse::<u32>().unwrap(), y.parse::<u32>().unwrap())
        }
        Err(_) => (0, 65536),
    };
    let only = std::env::var("FE_SIMD_ONLY").ok();
    let w = Work::new("e");
    std::fs::write(w.dir.join("run16.cjs"), DRIVER16).unwrap();
    let mut failures: Vec<String> = Vec::new();
    for c in cases().into_iter().filter(|c| c.lanes16 && only.as_deref().map_or(true, |o| o == c.name)) {
        let wasm = match lower(&c) {
            Ok(x) => x,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };
        let wp = w.dir.join("t16.wasm");
        std::fs::write(&wp, &wasm).unwrap();
        let t0 = std::time::Instant::now();
        let o = Command::new("node").arg(w.dir.join("run16.cjs")).arg(&wp).arg(a_from.to_string()).arg(a_to.to_string()).output().unwrap();
        if !o.status.success() {
            failures.push(format!("{}: node: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            continue;
        }
        let got = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let want = host_digest16(&c, a_from, a_to);
        let pairs = (a_to - a_from) as u64 * 65536;
        eprintln!("simd_int16 {:<10} {} ordered 16-bit pairs: digest wasm {got} host {want} {} ({:.1} s)", c.name, pairs, if got == want { "IDENTICAL" } else { "DIFFERENT" }, t0.elapsed().as_secs_f64());
        if got != want {
            failures.push(format!("{}: digest of {pairs} pairs differs: wasm {got} host {want}", c.name));
        }
        // the digest must be able to see a broken lowering: the same run over the mutated wasm has to give another digest
        let mut mw = wasm.clone();
        if !mutate(&mut mw, c.mutation) {
            failures.push(format!("{}: the mutation found nothing to change", c.name));
            continue;
        }
        let mp = w.dir.join("t16m.wasm");
        std::fs::write(&mp, &mw).unwrap();
        let o = Command::new("node").arg(w.dir.join("run16.cjs")).arg(&mp).arg(a_from.to_string()).arg(a_to.to_string()).output().unwrap();
        let mgot = String::from_utf8_lossy(&o.stdout).trim().to_string();
        eprintln!("simd_int16 {:<10} mutant digest {mgot} {}", c.name, if mgot != want { "DETECTED" } else { "NOT DETECTED" });
        if mgot == want {
            failures.push(format!("{}: the broken lowering gives the digest of the host CPU", c.name));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
