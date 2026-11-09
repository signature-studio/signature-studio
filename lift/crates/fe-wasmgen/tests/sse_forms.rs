//! Differential test of the SSE/SSE2 forms that the OTT pilot added (decision 0164): one tiny function per instruction (`movups xmm0,[rcx]; movups xmm1,[rcx+16]; <insn>; store; ret`) is
//! lowered, run under node on random and special inputs (zeros, denormals, infinities, NaNs, values beyond the integer range), and compared with the host CPU's own result of the same
//! instruction (core::arch). Float results are compared bit for bit except that two NaNs are taken as equal (wasm leaves the payload of a NaN result open). Skipped (and passing) when
//! node is missing or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::arch::x86_64::*;
use std::io::Write;
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


#[derive(Clone, Copy)]
enum Cmp {
    Bits,
    F32,
    F64,
}

struct Case {
    name: &'static str,
    insn: Vec<u8>,
    /// the result is stored by `movups [rdx], xmm0` (true) or is in rax/eax (stored as 8 bytes)
    xmm_out: bool,
    cmp: Cmp,
    want: Box<dyn Fn(__m128i, __m128i) -> [u8; 16]>,
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

fn inputs(n: usize) -> Vec<[u8; 32]> {
    let f64s: [u64; 34] = [
        0, 1 << 63, 0x3ff0000000000000, 0xbff0000000000000, 0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000000, 0xfff8000000000001, 0x7ff0000000000001, 1, 0x000fffffffffffff, 0x8000000000000001,
        0x41dfffffffc00000, 0x41e0000000000000, 0xc1e0000000000000, 0xc1e0000000200000, 0x4330000000000000, 0x43e0000000000000, 0xc3e0000000000000, 0x3fe0000000000000, 0x7fefffffffffffff,
        0x0010000000000000, 0x4024000000000000, 0xc024000000000000,
        // decision 0252: ties and the neighbours of the 2^31 boundary for the rounding conversions (1.5, 2.5, -0.5, 0.5 - ulp, 0.5 + ulp, 2^31 - 0.5, below it, -2^31 - 0.5, either side of it)
        0x3ff8000000000000, 0x4004000000000000, 0xbfe0000000000000, 0x3fdfffffffffffff, 0x3fe0000000000001, 0x41dfffffffe00000, 0x41dfffffffdfffff, 0xc1e0000000100000, 0xc1e00000000fffff, 0xc1e0000000100001,
    ];
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut v = Vec::new();
    for _ in 0..n {
        let mut b = [0u8; 32];
        for l in 0..4 {
            let x = rng(&mut s);
            let w = match x % 4 {
                0 => f64s[(rng(&mut s) % f64s.len() as u64) as usize],
                1 => {
                    // two f32 specials
                    let a = (f64s[(rng(&mut s) % f64s.len() as u64) as usize] >> 32) as u32;
                    let c = (f64s[(rng(&mut s) % f64s.len() as u64) as usize] >> 32) as u32;
                    (a as u64) << 32 | c as u64
                }
                _ => rng(&mut s),
            };
            b[8 * l..8 * l + 8].copy_from_slice(&w.to_le_bytes());
        }
        v.push(b);
    }
    v
}

fn same(c: Cmp, a: &[u8; 16], b: &[u8; 16]) -> bool {
    match c {
        Cmp::Bits => a == b,
        Cmp::F32 => (0..4).all(|l| {
            let x = u32::from_le_bytes(a[4 * l..4 * l + 4].try_into().unwrap());
            let y = u32::from_le_bytes(b[4 * l..4 * l + 4].try_into().unwrap());
            x == y || (f32::from_bits(x).is_nan() && f32::from_bits(y).is_nan())
        }),
        Cmp::F64 => (0..2).all(|l| {
            let x = u64::from_le_bytes(a[8 * l..8 * l + 8].try_into().unwrap());
            let y = u64::from_le_bytes(b[8 * l..8 * l + 8].try_into().unwrap());
            x == y || (f64::from_bits(x).is_nan() && f64::from_bits(y).is_nan())
        }),
    }
}

macro_rules! op2 {
    ($name:expr, $insn:expr, $cmp:expr, $f:expr) => {
        Case { name: $name, insn: $insn.to_vec(), xmm_out: true, cmp: $cmp, want: Box::new(|a, b| to_bytes(unsafe { $f(a, b) })) }
    };
}

fn cases() -> Vec<Case> {
    let mut v: Vec<Case> = Vec::new();
    #[allow(unused_unsafe)]
    unsafe {
        v.push(op2!("cvtdq2ps", [0x0f, 0x5b, 0xc1], Cmp::F32, |_a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtepi32_ps(b))));
        v.push(op2!("cvtdq2pd", [0xf3, 0x0f, 0xe6, 0xc1], Cmp::F64, |_a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtepi32_pd(b))));
        v.push(op2!("cvtps2pd", [0x0f, 0x5a, 0xc1], Cmp::F64, |_a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtps_pd(_mm_castsi128_ps(b)))));
        v.push(op2!("cvtpd2ps", [0x66, 0x0f, 0x5a, 0xc1], Cmp::F32, |_a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtpd_ps(_mm_castsi128_pd(b)))));
        v.push(op2!("cvttpd2dq", [0x66, 0x0f, 0xe6, 0xc1], Cmp::Bits, |_a: __m128i, b: __m128i| _mm_cvttpd_epi32(_mm_castsi128_pd(b))));
        // decision 0252: the plain lowering rounds to nearest even (the host runs with the default MXCSR); a NaN or a value whose rounded form does not fit gives 0x80000000
        v.push(op2!("cvtpd2dq", [0xf2, 0x0f, 0xe6, 0xc1], Cmp::Bits, |_a: __m128i, b: __m128i| _mm_cvtpd_epi32(_mm_castsi128_pd(b))));
        v.push(op2!("sqrtpd", [0x66, 0x0f, 0x51, 0xc1], Cmp::F64, |_a: __m128i, b: __m128i| _mm_castpd_si128(_mm_sqrt_pd(_mm_castsi128_pd(b)))));
        v.push(op2!("sqrtps", [0x0f, 0x51, 0xc1], Cmp::F32, |_a: __m128i, b: __m128i| _mm_castps_si128(_mm_sqrt_ps(_mm_castsi128_ps(b)))));
        v.push(op2!("sqrtss", [0xf3, 0x0f, 0x51, 0xc1], Cmp::F32, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_move_ss(_mm_castsi128_ps(a), _mm_sqrt_ss(_mm_castsi128_ps(b))))));
        v.push(op2!("maxsd", [0xf2, 0x0f, 0x5f, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_max_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("minsd", [0xf2, 0x0f, 0x5d, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_min_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("maxss", [0xf3, 0x0f, 0x5f, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_max_ss(_mm_castsi128_ps(a), _mm_castsi128_ps(b)))));
        v.push(op2!("minss", [0xf3, 0x0f, 0x5d, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_min_ss(_mm_castsi128_ps(a), _mm_castsi128_ps(b)))));
        v.push(op2!("maxpd", [0x66, 0x0f, 0x5f, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_max_pd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("minpd", [0x66, 0x0f, 0x5d, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_min_pd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("maxps", [0x0f, 0x5f, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_max_ps(_mm_castsi128_ps(a), _mm_castsi128_ps(b)))));
        v.push(op2!("minps", [0x0f, 0x5d, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_min_ps(_mm_castsi128_ps(a), _mm_castsi128_ps(b)))));
        v.push(op2!("paddb", [0x66, 0x0f, 0xfc, 0xc1], Cmp::Bits, |a, b| _mm_add_epi8(a, b)));
        v.push(op2!("paddw", [0x66, 0x0f, 0xfd, 0xc1], Cmp::Bits, |a, b| _mm_add_epi16(a, b)));
        v.push(op2!("paddd", [0x66, 0x0f, 0xfe, 0xc1], Cmp::Bits, |a, b| _mm_add_epi32(a, b)));
        v.push(op2!("paddq", [0x66, 0x0f, 0xd4, 0xc1], Cmp::Bits, |a, b| _mm_add_epi64(a, b)));
        v.push(op2!("psubb", [0x66, 0x0f, 0xf8, 0xc1], Cmp::Bits, |a, b| _mm_sub_epi8(a, b)));
        v.push(op2!("psubw", [0x66, 0x0f, 0xf9, 0xc1], Cmp::Bits, |a, b| _mm_sub_epi16(a, b)));
        v.push(op2!("psubd", [0x66, 0x0f, 0xfa, 0xc1], Cmp::Bits, |a, b| _mm_sub_epi32(a, b)));
        v.push(op2!("psubq", [0x66, 0x0f, 0xfb, 0xc1], Cmp::Bits, |a, b| _mm_sub_epi64(a, b)));
        v.push(op2!("punpcklbw", [0x66, 0x0f, 0x60, 0xc1], Cmp::Bits, |a, b| _mm_unpacklo_epi8(a, b)));
        v.push(op2!("punpcklqdq", [0x66, 0x0f, 0x6c, 0xc1], Cmp::Bits, |a, b| _mm_unpacklo_epi64(a, b)));
        v.push(op2!("punpckhqdq", [0x66, 0x0f, 0x6d, 0xc1], Cmp::Bits, |a, b| _mm_unpackhi_epi64(a, b)));
        v.push(op2!("movq xmm,xmm", [0xf3, 0x0f, 0x7e, 0xc1], Cmp::Bits, |_a, b| _mm_move_epi64(b)));
        v.push(op2!("psrlq 3", [0x66, 0x0f, 0x73, 0xd0, 0x03], Cmp::Bits, |a, _b| _mm_srli_epi64::<3>(a)));
        v.push(op2!("psrlq 63", [0x66, 0x0f, 0x73, 0xd0, 0x3f], Cmp::Bits, |a, _b| _mm_srli_epi64::<63>(a)));
        v.push(op2!("psrlq 64", [0x66, 0x0f, 0x73, 0xd0, 0x40], Cmp::Bits, |a, _b| _mm_srli_epi64::<64>(a)));
        v.push(op2!("psllq 5", [0x66, 0x0f, 0x73, 0xf0, 0x05], Cmp::Bits, |a, _b| _mm_slli_epi64::<5>(a)));
        v.push(op2!("psrldq 3", [0x66, 0x0f, 0x73, 0xd8, 0x03], Cmp::Bits, |a, _b| _mm_srli_si128::<3>(a)));
        v.push(op2!("psrldq 15", [0x66, 0x0f, 0x73, 0xd8, 0x0f], Cmp::Bits, |a, _b| _mm_srli_si128::<15>(a)));
        v.push(op2!("pslldq 4", [0x66, 0x0f, 0x73, 0xf8, 0x04], Cmp::Bits, |a, _b| _mm_slli_si128::<4>(a)));
        v.push(op2!("cmpsd 0", [0xf2, 0x0f, 0xc2, 0xc1, 0x00], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpeq_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 1", [0xf2, 0x0f, 0xc2, 0xc1, 0x01], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmplt_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 2", [0xf2, 0x0f, 0xc2, 0xc1, 0x02], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmple_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 3", [0xf2, 0x0f, 0xc2, 0xc1, 0x03], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpunord_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 4", [0xf2, 0x0f, 0xc2, 0xc1, 0x04], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpneq_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 5", [0xf2, 0x0f, 0xc2, 0xc1, 0x05], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpnlt_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 6", [0xf2, 0x0f, 0xc2, 0xc1, 0x06], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpnle_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cmpsd 7", [0xf2, 0x0f, 0xc2, 0xc1, 0x07], Cmp::Bits, |a, b| _mm_castpd_si128(_mm_cmpord_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        // integer to float (decision 0169): the integer comes from the second input (`mov eax/rax,[rcx+16]` is part of the form), the destination keeps its upper lanes
        v.push(op2!("cvtsi2ss r32", [0x8b, 0x41, 0x10, 0xf3, 0x0f, 0x2a, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsi32_ss(_mm_castsi128_ps(a), _mm_cvtsi128_si32(b)))));
        v.push(op2!("cvtsi2ss r64", [0x48, 0x8b, 0x41, 0x10, 0xf3, 0x48, 0x0f, 0x2a, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsi64_ss(_mm_castsi128_ps(a), _mm_cvtsi128_si64(b)))));
        v.push(op2!("cvtsi2sd r32", [0x8b, 0x41, 0x10, 0xf2, 0x0f, 0x2a, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtsi32_sd(_mm_castsi128_pd(a), _mm_cvtsi128_si32(b)))));
        v.push(op2!("cvtsi2sd r64", [0x48, 0x8b, 0x41, 0x10, 0xf2, 0x48, 0x0f, 0x2a, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtsi64_sd(_mm_castsi128_pd(a), _mm_cvtsi128_si64(b)))));
        // 0x721280 of the engine (the function the bisection of the hybrid ends in): REX.W-prefixed cvtsd2ss/cvtss2sd (the hardware ignores W), xmm8 as destination and source
        v.push(op2!("cvtsd2ss", [0xf2, 0x0f, 0x5a, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsd_ss(_mm_castsi128_ps(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cvtsd2ss rex.W", [0xf2, 0x48, 0x0f, 0x5a, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsd_ss(_mm_castsi128_ps(a), _mm_castsi128_pd(b)))));
        v.push(op2!("cvtss2sd", [0xf3, 0x0f, 0x5a, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtss_sd(_mm_castsi128_pd(a), _mm_castsi128_ps(b)))));
        v.push(op2!("cvtss2sd rex.W", [0xf3, 0x48, 0x0f, 0x5a, 0xc1], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtss_sd(_mm_castsi128_pd(a), _mm_castsi128_ps(b)))));
        v.push(op2!("cvtsd2ss xmm8,xmm1", [0x44, 0x0f, 0x28, 0xc0, 0xf2, 0x44, 0x0f, 0x5a, 0xc1, 0x41, 0x0f, 0x28, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsd_ss(_mm_castsi128_ps(a), _mm_castsi128_pd(b)))));
        v.push(op2!("mulss xmm0,xmm8", [0x44, 0x0f, 0x28, 0xc1, 0xf3, 0x41, 0x0f, 0x59, 0xc0], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_mul_ss(_mm_castsi128_ps(a), _mm_castsi128_ps(b)))));
        v.push(op2!("subsd xmm0,[mem]", [0xf2, 0x0f, 0x5c, 0x41, 0x10], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_sub_sd(_mm_castsi128_pd(a), _mm_castsi128_pd(b)))));
        // the memory-operand forms (the engine has `cvtsi2ss xmm0, dword [rbx+0x1c]` at 0x812b20)
        v.push(op2!("cvtsi2ss m32", [0xf3, 0x0f, 0x2a, 0x41, 0x10], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsi32_ss(_mm_castsi128_ps(a), _mm_cvtsi128_si32(b)))));
        v.push(op2!("cvtsi2ss m64", [0xf3, 0x48, 0x0f, 0x2a, 0x41, 0x10], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castps_si128(_mm_cvtsi64_ss(_mm_castsi128_ps(a), _mm_cvtsi128_si64(b)))));
        v.push(op2!("cvtsi2sd m32", [0xf2, 0x0f, 0x2a, 0x41, 0x10], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtsi32_sd(_mm_castsi128_pd(a), _mm_cvtsi128_si32(b)))));
        v.push(op2!("cvtsi2sd m64", [0xf2, 0x48, 0x0f, 0x2a, 0x41, 0x10], Cmp::Bits, |a: __m128i, b: __m128i| _mm_castpd_si128(_mm_cvtsi64_sd(_mm_castsi128_pd(a), _mm_cvtsi128_si64(b)))));
        // results in a general register (stored as 8 bytes)
        let gp = |name: &'static str, insn: &[u8], f: fn(__m128i, __m128i) -> u64| Case {
            name,
            insn: insn.to_vec(),
            xmm_out: false,
            cmp: Cmp::Bits,
            want: Box::new(move |a, b| {
                let mut o = [0u8; 16];
                o[..8].copy_from_slice(&f(a, b).to_le_bytes());
                o
            }),
        };
        v.push(gp("cvttss2si r32", &[0xf3, 0x0f, 0x2c, 0xc1], |_a, b| unsafe { _mm_cvttss_si32(_mm_castsi128_ps(b)) as u32 as u64 }));
        v.push(gp("cvttss2si r64", &[0xf3, 0x48, 0x0f, 0x2c, 0xc1], |_a, b| unsafe { _mm_cvttss_si64(_mm_castsi128_ps(b)) as u64 }));
        v.push(gp("cvttsd2si r32", &[0xf2, 0x0f, 0x2c, 0xc1], |_a, b| unsafe { _mm_cvttsd_si32(_mm_castsi128_pd(b)) as u32 as u64 }));
        v.push(gp("cvttsd2si r64", &[0xf2, 0x48, 0x0f, 0x2c, 0xc1], |_a, b| unsafe { _mm_cvttsd_si64(_mm_castsi128_pd(b)) as u64 }));
        v.push(gp("movd r32,xmm", &[0x66, 0x0f, 0x7e, 0xc0], |a, _b| unsafe { _mm_cvtsi128_si32(a) as u32 as u64 }));
        v.push(gp("movq r64,xmm", &[0x66, 0x48, 0x0f, 0x7e, 0xc0], |a, _b| unsafe { _mm_cvtsi128_si64(a) as u64 }));
    }
    v
}

#[test]
fn sse_forms_match_the_host_cpu() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs(600);
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-sse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut flat: Vec<u8> = Vec::new();
    for b in &ins {
        flat.extend_from_slice(b);
    }
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(
        &js,
        "const fs=require('fs');const mem=new WebAssembly.Memory({initial:16});const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
         const i=new WebAssembly.Instance(m,{env:{mem}});const inp=fs.readFileSync(process.argv[3]);const n=inp.length/32;const u8=new Uint8Array(mem.buffer);const out=Buffer.alloc(16*n);\
         for(let k=0;k<n;k++){u8.set(inp.subarray(32*k,32*k+32),0x10000);u8.fill(0,0x20000,0x20010);i.exports.f_401000(0x80000n,0x10000n,0x20000n,0n,0n);out.set(u8.subarray(0x20000,0x20010),16*k);}\
         fs.writeFileSync(process.argv[4],out);",
    )
    .unwrap();
    let mut failures: Vec<String> = Vec::new();
    for c in cases() {
        let mut code: Vec<u8> = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10]; // movups xmm0,[rcx]; movups xmm1,[rcx+16]
        code.extend_from_slice(&c.insn);
        if c.xmm_out {
            code.extend_from_slice(&[0x0f, 0x11, 0x02]); // movups [rdx],xmm0
        } else {
            code.extend_from_slice(&[0x48, 0x89, 0x02]); // mov [rdx],rax
        }
        code.push(0xc3);
        let conf = Config { export_funcs: true, ..Config::default() };
        let comp = match compile(&pe(&code), &[], &[], &conf) {
            Ok(x) => x,
            Err(e) => {
                failures.push(format!("{}: compile: {e}", c.name));
                continue;
            }
        };
        if !comp.fidx.contains_key(&0x401000) {
            failures.push(format!("{}: not lowered", c.name));
            continue;
        }
        let w = dir.join("t.wasm");
        std::fs::write(&w, &comp.wasm).unwrap();
        if have("wasm-validate") {
            let o = Command::new("wasm-validate").arg(&w).output().unwrap();
            if !o.status.success() {
                failures.push(format!("{}: wasm-validate: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
                continue;
            }
        }
        let outp = dir.join("out.bin");
        let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).output().unwrap();
        if !o.status.success() {
            failures.push(format!("{}: node: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            continue;
        }
        let got = std::fs::read(&outp).unwrap();
        let mut bad = 0;
        let mut first = String::new();
        for (k, b) in ins.iter().enumerate() {
            let a = from_bytes(b[..16].try_into().unwrap());
            let bb = from_bytes(b[16..].try_into().unwrap());
            let want = (c.want)(a, bb);
            let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
            let ok = if c.xmm_out { same(c.cmp, &g, &want) } else { g[..8] == want[..8] };
            if !ok {
                bad += 1;
                if first.is_empty() {
                    first = format!("in {:02x?} {:02x?} want {:02x?} got {:02x?}", &b[..16], &b[16..], want, g);
                }
            }
        }
        if bad > 0 {
            failures.push(format!("{}: {bad} of {} inputs differ; first: {first}", c.name, ins.len()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::io::stdout().flush();
    eprintln!("sse_forms: {} instruction forms x {} inputs compared with the host CPU", cases().len(), ins.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
