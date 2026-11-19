//! Decision 0181 (hyb2, --mx-packed): differential test of the SSE forms of `sse_x2` (andn, unpack, shuffle, half moves, integer compares, shifts, pack, pmuludq, movmsk, pextrw).
//! Same harness as sse_forms.rs. The earlier text follows: differential test of the SSE/SSE2 forms that the OTT pilot added (decision 0164): one tiny function per instruction (`movups xmm0,[rcx]; movups xmm1,[rcx+16]; <insn>; store; ret`) is
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
    let f64s: [u64; 24] = [
        0, 1 << 63, 0x3ff0000000000000, 0xbff0000000000000, 0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000000, 0xfff8000000000001, 0x7ff0000000000001, 1, 0x000fffffffffffff, 0x8000000000000001,
        0x41dfffffffc00000, 0x41e0000000000000, 0xc1e0000000000000, 0xc1e0000000200000, 0x4330000000000000, 0x43e0000000000000, 0xc3e0000000000000, 0x3fe0000000000000, 0x7fefffffffffffff,
        0x0010000000000000, 0x4024000000000000, 0xc024000000000000,
    ];
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut v = Vec::new();
    for _ in 0..n {
        let mut b = [0u8; 32];
        for l in 0..4 {
            let x = rng(&mut s);
            let w = match x % 4 {
                0 => f64s[(rng(&mut s) % 24) as usize],
                1 => {
                    // two f32 specials
                    let a = (f64s[(rng(&mut s) % 24) as usize] >> 32) as u32;
                    let c = (f64s[(rng(&mut s) % 24) as usize] >> 32) as u32;
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

fn f(a: __m128i) -> __m128 {
    unsafe { _mm_castsi128_ps(a) }
}
fn d(a: __m128i) -> __m128d {
    unsafe { _mm_castsi128_pd(a) }
}
fn bi(a: __m128) -> __m128i {
    unsafe { _mm_castps_si128(a) }
}
fn bd(a: __m128d) -> __m128i {
    unsafe { _mm_castpd_si128(a) }
}

fn cases() -> Vec<Case> {
    let mut v: Vec<Case> = Vec::new();
    #[allow(unused_unsafe)]
    unsafe {
        v.push(op2!("andnps", [0x0f, 0x55, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_andnot_ps(f(a), f(b)))));
        v.push(op2!("andnpd", [0x66, 0x0f, 0x55, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_andnot_pd(d(a), d(b)))));
        v.push(op2!("pandn", [0x66, 0x0f, 0xdf, 0xc1], Cmp::Bits, |a, b| _mm_andnot_si128(a, b)));
        v.push(op2!("unpcklps", [0x0f, 0x14, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_unpacklo_ps(f(a), f(b)))));
        v.push(op2!("unpckhps", [0x0f, 0x15, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_unpackhi_ps(f(a), f(b)))));
        v.push(op2!("unpcklpd", [0x66, 0x0f, 0x14, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_unpacklo_pd(d(a), d(b)))));
        v.push(op2!("unpckhpd", [0x66, 0x0f, 0x15, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_unpackhi_pd(d(a), d(b)))));
        v.push(op2!("movlhps", [0x0f, 0x16, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_movelh_ps(f(a), f(b)))));
        v.push(op2!("movhlps", [0x0f, 0x12, 0xc1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_movehl_ps(f(a), f(b)))));
        v.push(op2!("shufps 0x1b", [0x0f, 0xc6, 0xc1, 0x1b], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_shuffle_ps::<0x1b>(f(a), f(b)))));
        v.push(op2!("shufps 0xe4", [0x0f, 0xc6, 0xc1, 0xe4], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_shuffle_ps::<0xe4>(f(a), f(b)))));
        v.push(op2!("shufps 0x44", [0x0f, 0xc6, 0xc1, 0x44], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_shuffle_ps::<0x44>(f(a), f(b)))));
        v.push(op2!("shufps 0xb1", [0x0f, 0xc6, 0xc1, 0xb1], Cmp::Bits, move |a: __m128i, b: __m128i| bi(_mm_shuffle_ps::<0xb1>(f(a), f(b)))));
        v.push(op2!("shufpd 0", [0x66, 0x0f, 0xc6, 0xc1, 0x00], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_shuffle_pd::<0>(d(a), d(b)))));
        v.push(op2!("shufpd 1", [0x66, 0x0f, 0xc6, 0xc1, 0x01], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_shuffle_pd::<1>(d(a), d(b)))));
        v.push(op2!("shufpd 2", [0x66, 0x0f, 0xc6, 0xc1, 0x02], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_shuffle_pd::<2>(d(a), d(b)))));
        v.push(op2!("shufpd 3", [0x66, 0x0f, 0xc6, 0xc1, 0x03], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_shuffle_pd::<3>(d(a), d(b)))));
        v.push(op2!("pshufd 0x1b", [0x66, 0x0f, 0x70, 0xc1, 0x1b], Cmp::Bits, |_a, b| _mm_shuffle_epi32::<0x1b>(b)));
        v.push(op2!("pshufd 0x4e", [0x66, 0x0f, 0x70, 0xc1, 0x4e], Cmp::Bits, |_a, b| _mm_shuffle_epi32::<0x4e>(b)));
        v.push(op2!("pshufd 0x00", [0x66, 0x0f, 0x70, 0xc1, 0x00], Cmp::Bits, |_a, b| _mm_shuffle_epi32::<0x00>(b)));
        v.push(op2!("pshuflw 0x1b", [0xf2, 0x0f, 0x70, 0xc1, 0x1b], Cmp::Bits, |_a, b| _mm_shufflelo_epi16::<0x1b>(b)));
        v.push(op2!("pshuflw 0xe1", [0xf2, 0x0f, 0x70, 0xc1, 0xe1], Cmp::Bits, |_a, b| _mm_shufflelo_epi16::<0xe1>(b)));
        v.push(op2!("pshufhw 0x1b", [0xf3, 0x0f, 0x70, 0xc1, 0x1b], Cmp::Bits, |_a, b| _mm_shufflehi_epi16::<0x1b>(b)));
        v.push(op2!("punpckldq", [0x66, 0x0f, 0x62, 0xc1], Cmp::Bits, |a, b| _mm_unpacklo_epi32(a, b)));
        v.push(op2!("punpcklwd", [0x66, 0x0f, 0x61, 0xc1], Cmp::Bits, |a, b| _mm_unpacklo_epi16(a, b)));
        v.push(op2!("punpckhdq", [0x66, 0x0f, 0x6a, 0xc1], Cmp::Bits, |a, b| _mm_unpackhi_epi32(a, b)));
        v.push(op2!("punpckhwd", [0x66, 0x0f, 0x69, 0xc1], Cmp::Bits, |a, b| _mm_unpackhi_epi16(a, b)));
        v.push(op2!("punpckhbw", [0x66, 0x0f, 0x68, 0xc1], Cmp::Bits, |a, b| _mm_unpackhi_epi8(a, b)));
        v.push(op2!("pcmpeqb", [0x66, 0x0f, 0x74, 0xc1], Cmp::Bits, |a, b| _mm_cmpeq_epi8(a, b)));
        v.push(op2!("pcmpeqw", [0x66, 0x0f, 0x75, 0xc1], Cmp::Bits, |a, b| _mm_cmpeq_epi16(a, b)));
        v.push(op2!("pcmpeqd", [0x66, 0x0f, 0x76, 0xc1], Cmp::Bits, |a, b| _mm_cmpeq_epi32(a, b)));
        v.push(op2!("pcmpgtb", [0x66, 0x0f, 0x64, 0xc1], Cmp::Bits, |a, b| _mm_cmpgt_epi8(a, b)));
        v.push(op2!("pcmpgtw", [0x66, 0x0f, 0x65, 0xc1], Cmp::Bits, |a, b| _mm_cmpgt_epi16(a, b)));
        v.push(op2!("pcmpgtd", [0x66, 0x0f, 0x66, 0xc1], Cmp::Bits, |a, b| _mm_cmpgt_epi32(a, b)));
        v.push(op2!("psrlw 3", [0x66, 0x0f, 0x71, 0xd0, 0x03], Cmp::Bits, |a, _b| _mm_srli_epi16::<3>(a)));
        v.push(op2!("psrlw 16", [0x66, 0x0f, 0x71, 0xd0, 0x10], Cmp::Bits, |a, _b| _mm_srli_epi16::<16>(a)));
        v.push(op2!("psraw 5", [0x66, 0x0f, 0x71, 0xe0, 0x05], Cmp::Bits, |a, _b| _mm_srai_epi16::<5>(a)));
        v.push(op2!("psraw 40", [0x66, 0x0f, 0x71, 0xe0, 0x28], Cmp::Bits, |a, _b| _mm_srai_epi16::<40>(a)));
        v.push(op2!("psllw 7", [0x66, 0x0f, 0x71, 0xf0, 0x07], Cmp::Bits, |a, _b| _mm_slli_epi16::<7>(a)));
        v.push(op2!("psrld 3", [0x66, 0x0f, 0x72, 0xd0, 0x03], Cmp::Bits, |a, _b| _mm_srli_epi32::<3>(a)));
        v.push(op2!("psrld 31", [0x66, 0x0f, 0x72, 0xd0, 0x1f], Cmp::Bits, |a, _b| _mm_srli_epi32::<31>(a)));
        v.push(op2!("psrld 32", [0x66, 0x0f, 0x72, 0xd0, 0x20], Cmp::Bits, |a, _b| _mm_srli_epi32::<32>(a)));
        v.push(op2!("psrad 9", [0x66, 0x0f, 0x72, 0xe0, 0x09], Cmp::Bits, |a, _b| _mm_srai_epi32::<9>(a)));
        v.push(op2!("psrad 33", [0x66, 0x0f, 0x72, 0xe0, 0x21], Cmp::Bits, |a, _b| _mm_srai_epi32::<33>(a)));
        v.push(op2!("pslld 13", [0x66, 0x0f, 0x72, 0xf0, 0x0d], Cmp::Bits, |a, _b| _mm_slli_epi32::<13>(a)));
        v.push(op2!("psrlw xmm", [0x66, 0x0f, 0xd1, 0xc1], Cmp::Bits, |a, b| _mm_srl_epi16(a, b)));
        v.push(op2!("psrld xmm", [0x66, 0x0f, 0xd2, 0xc1], Cmp::Bits, |a, b| _mm_srl_epi32(a, b)));
        v.push(op2!("psraw xmm", [0x66, 0x0f, 0xe1, 0xc1], Cmp::Bits, |a, b| _mm_sra_epi16(a, b)));
        v.push(op2!("psrad xmm", [0x66, 0x0f, 0xe2, 0xc1], Cmp::Bits, |a, b| _mm_sra_epi32(a, b)));
        v.push(op2!("psllw xmm", [0x66, 0x0f, 0xf1, 0xc1], Cmp::Bits, |a, b| _mm_sll_epi16(a, b)));
        v.push(op2!("pslld xmm", [0x66, 0x0f, 0xf2, 0xc1], Cmp::Bits, |a, b| _mm_sll_epi32(a, b)));
        v.push(op2!("packuswb", [0x66, 0x0f, 0x67, 0xc1], Cmp::Bits, |a, b| _mm_packus_epi16(a, b)));
        v.push(op2!("packsswb", [0x66, 0x0f, 0x63, 0xc1], Cmp::Bits, |a, b| _mm_packs_epi16(a, b)));
        v.push(op2!("packssdw", [0x66, 0x0f, 0x6b, 0xc1], Cmp::Bits, |a, b| _mm_packs_epi32(a, b)));
        v.push(op2!("pmuludq", [0x66, 0x0f, 0xf4, 0xc1], Cmp::Bits, |a, b| _mm_mul_epu32(a, b)));
        // half moves with a memory source ([rcx+16] holds the second input)
        v.push(op2!("movlps m64", [0x0f, 0x12, 0x41, 0x10], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_loadl_pd(d(a), &b as *const __m128i as *const f64))));
        v.push(op2!("movhps m64", [0x0f, 0x16, 0x41, 0x10], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_loadh_pd(d(a), &b as *const __m128i as *const f64))));
        v.push(op2!("movlpd m64", [0x66, 0x0f, 0x12, 0x41, 0x10], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_loadl_pd(d(a), &b as *const __m128i as *const f64))));
        v.push(op2!("movhpd m64", [0x66, 0x0f, 0x16, 0x41, 0x10], Cmp::Bits, move |a: __m128i, b: __m128i| bd(_mm_loadh_pd(d(a), &b as *const __m128i as *const f64))));
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
        v.push(gp("movmskps", &[0x0f, 0x50, 0xc0], |a, _b| unsafe { _mm_movemask_ps(_mm_castsi128_ps(a)) as u32 as u64 }));
        v.push(gp("movmskpd", &[0x66, 0x0f, 0x50, 0xc0], |a, _b| unsafe { _mm_movemask_pd(_mm_castsi128_pd(a)) as u32 as u64 }));
        v.push(gp("pmovmskb", &[0x66, 0x0f, 0xd7, 0xc0], |a, _b| unsafe { _mm_movemask_epi8(a) as u32 as u64 }));
        v.push(gp("pextrw 5", &[0x66, 0x0f, 0xc5, 0xc0, 0x05], |a, _b| unsafe { _mm_extract_epi16::<5>(a) as u32 as u64 }));
        v.push(gp("pextrw 0", &[0x66, 0x0f, 0xc5, 0xc0, 0x00], |a, _b| unsafe { _mm_extract_epi16::<0>(a) as u32 as u64 }));
    }
    v
}

#[test]
fn sse_x2_forms_match_the_host_cpu() {
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
    eprintln!("sse_x2_forms: {} instruction forms x {} inputs compared with the host CPU", cases().len(), ins.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
