//! Decision 0172 (prototype): WebAssembly helper functions for scalar floating-point operations under a guest rounding control other than
//! round-to-nearest (RC = 1 down, 2 up, 3 toward zero). wasm has no dynamic rounding mode, so the nearest-even result `s` and an error
//! term `e` = exact - s (an error-free transformation) decide whether `s` is one ulp too far; the same arithmetic as `runtime/fle_mxcsr_rc.h`
//! (see its head for the proofs and the windows). A helper is a function of the module (no host call):
//!
//!   rc_addsub64(a: i64, b: i64, rc: i32) -> (i64, i32)   a + b with the bits of two doubles (a subtraction passes b with its sign flipped);
//!                                                        TwoSum; window: zero or 2^-510 <= |x| <= 2^510
//!   rc_mul64(a: i64, b: i64, rc: i32) -> (i64, i32)      Dekker's TwoProduct; window: zero or 2^-400 <= |x| <= 2^400
//!   rc_addsub32(a: i32, b: i32, rc: i32) -> (i32, i32)   binary32: TwoSum in binary32; window: zero or 2^-62 <= |x| <= 2^62
//!   rc_mul32(a: i32, b: i32, rc: i32) -> (i32, i32)      the double product (exact) against the binary32 result; the same window
//!
//! The second result is 1 when the operands were inside the window and the first result is the instruction's result; 0 when the caller has
//! to take the C path's function (the first result is then 0). The caller (the window rule of `lower.rs`, hybrid mode) has checked the guard:
//! RC != 0, PE set and PM masked in the C path's MXCSR, so no flag changes. These are the only floating-point forms with a helper here:
//! division, square root, the conversions and the packed forms stay on the C path (which has the same arithmetic in `fle_mxcsr_rc.h`).
//! Not wired into `lower.rs` (decision 0172, `rz1-wasmgen.diff`): the file is new, the hook is described in the decision document.

use crate::wasm::{Body, Code, FuncType, Module, Ty};

/// function indices of the helpers in the module
#[derive(Clone, Copy, Debug)]
pub struct RcHelpers {
    pub addsub64: u32,
    pub mul64: u32,
    pub addsub32: u32,
    pub mul32: u32,
}

const F64_REINTERPRET_I64: u8 = 0xbf;
const I64_REINTERPRET_F64: u8 = 0xbd;
const F32_REINTERPRET_I32: u8 = 0xbe;
const I32_REINTERPRET_F32: u8 = 0xbc;
const F64_ADD: u8 = 0xa0;
const F64_SUB: u8 = 0xa1;
const F64_MUL: u8 = 0xa2;
const F32_ADD: u8 = 0x92;
const F32_SUB: u8 = 0x93;
const F64_PROMOTE_F32: u8 = 0xbb;
const F32_DEMOTE_F64: u8 = 0xb6;
const SPLIT: f64 = 134217729.0; // 2^27 + 1

/// push the window test of a 64-bit pattern in local `t`: zero, or biased exponent in [lo, hi] (i32 on the stack)
fn win64(c: &mut Code, t: u32, lo: i64, hi: i64) {
    c.local_get(t);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(lo << 53);
    c.i64_sub();
    c.i64_const((hi - lo + 1) << 53);
    c.i64_lt_u();
    c.local_get(t);
    c.i64_const(1);
    c.i64_shl();
    c.i64_eqz();
    c.i32_or();
}

fn win32(c: &mut Code, t: u32, lo: i32, hi: i32) {
    c.local_get(t);
    c.i32_const(1);
    c.i32_shl();
    c.i32_const(lo << 24);
    c.i32_sub();
    c.i32_const((hi - lo + 1) << 24);
    c.i32_lt_u();
    c.local_get(t);
    c.i32_const(1);
    c.i32_shl();
    c.i32_eqz();
    c.i32_or();
}

/// i64 on the stack := the step of the bits `sb` by the sign of the error `eb` (locals), for rc in local `rc`: the formulas of fle_mxr_step64
/// locals used: ssg, esg, nz, delta (i64)
fn step64(c: &mut Code, rc: u32, sb: u32, eb: u32, ssg: u32, esg: u32, nz: u32) {
    // nz = 0 - (i64)((eb << 1) != 0); esg = eb >> 63; ssg = sb >> 63
    c.i64_const(0);
    c.local_get(eb);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(0);
    c.i64_ne();
    c.i64_extend_i32_u();
    c.i64_sub();
    c.local_set(nz);
    c.local_get(eb);
    c.i64_const(63);
    c.i64_shr_s();
    c.local_set(esg);
    c.local_get(sb);
    c.i64_const(63);
    c.i64_shr_s();
    c.local_set(ssg);
    // delta
    c.local_get(rc);
    c.i32_const(3);
    c.i32_eq();
    c.if_ty(Ty::I64);
    c.local_get(esg);
    c.local_get(ssg);
    c.i64_xor();
    c.local_get(nz);
    c.i64_and();
    c.else_();
    c.local_get(rc);
    c.i32_const(1);
    c.i32_eq();
    c.if_ty(Ty::I64);
    // RD: esg & nz & ((ssg & 2) - 1)
    c.local_get(esg);
    c.local_get(nz);
    c.i64_and();
    c.local_get(ssg);
    c.i64_const(2);
    c.i64_and();
    c.i64_const(1);
    c.i64_sub();
    c.i64_and();
    c.else_();
    // RU: ~esg & nz & (1 - (ssg & 2))
    c.local_get(esg);
    c.i64_const(-1);
    c.i64_xor();
    c.local_get(nz);
    c.i64_and();
    c.i64_const(1);
    c.local_get(ssg);
    c.i64_const(2);
    c.i64_and();
    c.i64_sub();
    c.i64_and();
    c.end();
    c.end();
    c.local_get(sb);
    c.i64_add();
}

/// i32 on the stack := the step of the binary32 bits `sb` for an error with sign mask `esg` and nonzero mask `nz` (i32 locals)
fn step32(c: &mut Code, rc: u32, sb: u32, esg: u32, nz: u32, ssg: u32) {
    c.local_get(sb);
    c.i32_const(31);
    c.i32_shr_s();
    c.local_set(ssg);
    c.local_get(rc);
    c.i32_const(3);
    c.i32_eq();
    c.if_ty(Ty::I32);
    c.local_get(esg);
    c.local_get(ssg);
    c.i32_xor();
    c.local_get(nz);
    c.i32_and();
    c.else_();
    c.local_get(rc);
    c.i32_const(1);
    c.i32_eq();
    c.if_ty(Ty::I32);
    c.local_get(esg);
    c.local_get(nz);
    c.i32_and();
    c.local_get(ssg);
    c.i32_const(2);
    c.i32_and();
    c.i32_const(1);
    c.i32_sub();
    c.i32_and();
    c.else_();
    c.local_get(esg);
    c.i32_const(-1);
    c.i32_xor();
    c.local_get(nz);
    c.i32_and();
    c.i32_const(1);
    c.local_get(ssg);
    c.i32_const(2);
    c.i32_and();
    c.i32_sub();
    c.i32_and();
    c.end();
    c.end();
    c.local_get(sb);
    c.i32_add();
}

fn ty2(a: Ty, b: Ty, r: Ty) -> FuncType {
    FuncType { params: vec![a, b, Ty::I32], results: vec![r, Ty::I32] }
}

fn addsub64_body() -> Body {
    // params 0 a, 1 b, 2 rc; locals i64: 3 sb, 4 eb, 5 ssg, 6 esg, 7 nz; f64: 8 A, 9 B, 10 S, 11 V, 12 E
    let mut c = Code::new();
    win64(&mut c, 0, 513, 1533);
    win64(&mut c, 1, 513, 1533);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i64_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    let rd = |c: &mut Code, p: u32, l: u32| {
        c.local_get(p);
        c.raw(&[F64_REINTERPRET_I64]);
        c.local_set(l);
    };
    rd(&mut c, 0, 8);
    rd(&mut c, 1, 9);
    // S = A + B; V = S - A; E = (A - (S - V)) + (B - V)
    c.local_get(8);
    c.local_get(9);
    c.raw(&[F64_ADD]);
    c.local_set(10);
    c.local_get(10);
    c.local_get(8);
    c.raw(&[F64_SUB]);
    c.local_set(11);
    c.local_get(8);
    c.local_get(10);
    c.local_get(11);
    c.raw(&[F64_SUB, F64_SUB]);
    c.local_get(9);
    c.local_get(11);
    c.raw(&[F64_SUB, F64_ADD]);
    c.local_set(12);
    c.local_get(10);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(3);
    c.local_get(12);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(4);
    step64(&mut c, 2, 3, 4, 5, 6, 7);
    c.local_set(3);
    // RD: an exact zero sum of operands of opposite sign is -0
    c.local_get(2);
    c.i32_const(1);
    c.i32_eq();
    c.local_get(3);
    c.i64_const(1);
    c.i64_shl();
    c.i64_eqz();
    c.i32_and();
    c.local_get(0);
    c.local_get(1);
    c.i64_xor();
    c.i64_const(0);
    c.i64_lt_s();
    c.i32_and();
    c.if_();
    c.i64_const(i64::MIN);
    c.local_set(3);
    c.end();
    c.local_get(3);
    c.i32_const(1);
    Body { locals: vec![(5, Ty::I64), (5, Ty::F64)], code: c }
}

fn mul64_body() -> Body {
    // params 0 a, 1 b, 2 rc; locals i64: 3 sb, 4 eb, 5 ssg, 6 esg, 7 nz; f64: 8 A, 9 B, 10 P, 11 AH, 12 AL, 13 BH, 14 BL, 15 T, 16 E
    let mut c = Code::new();
    win64(&mut c, 0, 623, 1423);
    win64(&mut c, 1, 623, 1423);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i64_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    c.local_get(0);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(8);
    c.local_get(1);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(9);
    c.local_get(8);
    c.local_get(9);
    c.raw(&[F64_MUL]);
    c.local_set(10);
    // split x: ca = SPLIT * x; hi = ca - (ca - x); lo = x - hi
    for (x, h, l) in [(8u32, 11u32, 12u32), (9, 13, 14)] {
        c.f64_const(SPLIT);
        c.local_get(x);
        c.raw(&[F64_MUL]);
        c.local_set(15);
        c.local_get(15);
        c.local_get(15);
        c.local_get(x);
        c.raw(&[F64_SUB, F64_SUB]);
        c.local_set(h);
        c.local_get(x);
        c.local_get(h);
        c.raw(&[F64_SUB]);
        c.local_set(l);
    }
    // E = ((AH*BH - P) + AH*BL + AL*BH) + AL*BL
    c.local_get(11);
    c.local_get(13);
    c.raw(&[F64_MUL]);
    c.local_get(10);
    c.raw(&[F64_SUB]);
    c.local_get(11);
    c.local_get(14);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(12);
    c.local_get(13);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(12);
    c.local_get(14);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_set(16);
    c.local_get(10);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(3);
    c.local_get(16);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(4);
    step64(&mut c, 2, 3, 4, 5, 6, 7);
    c.i32_const(1);
    Body { locals: vec![(5, Ty::I64), (9, Ty::F64)], code: c }
}

fn addsub32_body() -> Body {
    // params 0 a, 1 b, 2 rc (i32); locals i32: 3 sb, 4 eb, 5 ssg, 6 esg, 7 nz, 8 S, 9 V, 10 E
    let mut c = Code::new();
    win32(&mut c, 0, 65, 189);
    win32(&mut c, 1, 65, 189);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i32_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    let f = |c: &mut Code, l: u32| {
        c.local_get(l);
        c.raw(&[F32_REINTERPRET_I32]);
    };
    let back = |c: &mut Code| c.raw(&[I32_REINTERPRET_F32]);
    // S = a + b
    f(&mut c, 0);
    f(&mut c, 1);
    c.raw(&[F32_ADD]);
    back(&mut c);
    c.local_set(8);
    // V = S - a
    f(&mut c, 8);
    f(&mut c, 0);
    c.raw(&[F32_SUB]);
    back(&mut c);
    c.local_set(9);
    // E = (a - (S - V)) + (b - V)
    f(&mut c, 0);
    f(&mut c, 8);
    f(&mut c, 9);
    c.raw(&[F32_SUB, F32_SUB]);
    f(&mut c, 1);
    f(&mut c, 9);
    c.raw(&[F32_SUB, F32_ADD]);
    back(&mut c);
    c.local_set(10);
    c.local_get(8);
    c.local_set(3);
    // masks: esg = (int)e >> 31; nz = -(int)((e << 1) != 0)
    c.local_get(10);
    c.i32_const(31);
    c.i32_shr_s();
    c.local_set(6);
    c.i32_const(0);
    c.local_get(10);
    c.i32_const(1);
    c.i32_shl();
    c.i32_const(0);
    c.i32_ne();
    c.i32_sub();
    c.local_set(7);
    step32(&mut c, 2, 3, 6, 7, 5);
    c.local_set(3);
    c.local_get(2);
    c.i32_const(1);
    c.i32_eq();
    c.local_get(3);
    c.i32_const(1);
    c.i32_shl();
    c.i32_eqz();
    c.i32_and();
    c.local_get(0);
    c.local_get(1);
    c.i32_xor();
    c.i32_const(0);
    c.i32_lt_s();
    c.i32_and();
    c.if_();
    c.i32_const(i32::MIN);
    c.local_set(3);
    c.end();
    c.local_get(3);
    c.i32_const(1);
    Body { locals: vec![(8, Ty::I32)], code: c }
}

fn mul32_body() -> Body {
    // params 0 a, 1 b, 2 rc (i32); locals i32: 3 sb, 4 s, 5 ssg, 6 esg, 7 nz; i64: 8 eb; f64: 9 D, 10 E
    let mut c = Code::new();
    win32(&mut c, 0, 65, 189);
    win32(&mut c, 1, 65, 189);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i32_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    // D = (double)a * (double)b; s = (float)D (bits); E = D - (double)s
    c.local_get(0);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32]);
    c.local_get(1);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_MUL]);
    c.local_set(9);
    c.local_get(9);
    c.raw(&[F32_DEMOTE_F64, I32_REINTERPRET_F32]);
    c.local_set(3);
    c.local_get(9);
    c.local_get(3);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_SUB]);
    c.local_set(10);
    c.local_get(10);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(8);
    // masks for the binary32 s from the binary64 error
    c.i32_const(0);
    c.local_get(8);
    c.i64_const(63);
    c.i64_shr_u();
    c.i32_wrap_i64();
    c.i32_sub();
    c.local_set(6);
    c.i32_const(0);
    c.local_get(8);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(0);
    c.i64_ne();
    c.i32_sub();
    c.local_set(7);
    step32(&mut c, 2, 3, 6, 7, 5);
    c.i32_const(1);
    Body { locals: vec![(5, Ty::I32), (1, Ty::I64), (2, Ty::F64)], code: c }
}

/// Adds the four helpers to `m` and returns their function indices.
pub fn add_rc_helpers(m: &mut Module) -> RcHelpers {
    let addsub64 = m.add_func(ty2(Ty::I64, Ty::I64, Ty::I64), addsub64_body());
    let mul64 = m.add_func(ty2(Ty::I64, Ty::I64, Ty::I64), mul64_body());
    let addsub32 = m.add_func(ty2(Ty::I32, Ty::I32, Ty::I32), addsub32_body());
    let mul32 = m.add_func(ty2(Ty::I32, Ty::I32, Ty::I32), mul32_body());
    RcHelpers { addsub64, mul64, addsub32, mul32 }
}

// ---------------------------------------------------------------------------------------------------------------------------------------------
// Decision 0181 (hyb2): the packed forms (addps/subps/mulps, addpd/subpd/mulpd) under RC != 0, and the switches that connect the scalar and the packed
// helpers to `lower.rs` (hybrid mode, opt-in). The helpers of the packed forms take the two operands as v128, check the window of EVERY lane (zero, or a
// biased exponent in [lo, hi]: the same windows as the scalar helpers), and return (result, 1); outside the window they return (0, 0) and the caller takes the
// C path's function. A subtraction passes the second operand with its sign flipped (the caller does it). Same arithmetic as `fle_mxcsr_rc.h` (TwoSum,
// Dekker's TwoProduct, the double product of a binary32 pair), lane by lane in SIMD.
// ---------------------------------------------------------------------------------------------------------------------------------------------

use std::sync::atomic::{AtomicU32, Ordering};

/// function indices of the helpers in the module being built (set by `install`)
pub static HELPER: [AtomicU32; 13] = [const { AtomicU32::new(0) }; 13];
pub const H_ADDSUB64: usize = 0;
pub const H_MUL64: usize = 1;
pub const H_ADDSUB32: usize = 2;
pub const H_MUL32: usize = 3;
pub const H_ADDPS: usize = 4;
pub const H_MULPS: usize = 5;
pub const H_ADDPD: usize = 6;
pub const H_MULPD: usize = 7;
pub const H_CVTSD2SS: usize = 8;
pub const H_DIV64: usize = 9;
pub const H_SQRT64: usize = 10;
pub const H_DIV32: usize = 11;
pub const H_SQRT32: usize = 12;

pub fn helper(i: usize) -> u32 {
    HELPER[i].load(Ordering::Relaxed)
}

/// adds the eight helpers to the module and records their indices for `lower.rs`
pub fn install(m: &mut Module) {
    let s = add_rc_helpers(m);
    let p = add_rc_packed(m);
    let cvt = m.add_func(FuncType { params: vec![Ty::I64, Ty::I32], results: vec![Ty::I32, Ty::I32] }, cvtsd2ss_body());
    let div64 = m.add_func(ty2(Ty::I64, Ty::I64, Ty::I64), div64_body());
    let sqrt64 = m.add_func(FuncType { params: vec![Ty::I64, Ty::I32], results: vec![Ty::I64, Ty::I32] }, sqrt64_body());
    let div32 = m.add_func(ty2(Ty::I32, Ty::I32, Ty::I32), div32_body());
    let sqrt32 = m.add_func(FuncType { params: vec![Ty::I32, Ty::I32], results: vec![Ty::I32, Ty::I32] }, sqrt32_body());
    for (i, v) in [s.addsub64, s.mul64, s.addsub32, s.mul32, p.addps, p.mulps, p.addpd, p.mulpd, cvt, div64, sqrt64, div32, sqrt32].into_iter().enumerate() {
        HELPER[i].store(v, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RcPacked {
    pub addps: u32,
    pub mulps: u32,
    pub addpd: u32,
    pub mulpd: u32,
}

pub fn splat32(v: u32) -> [u8; 16] {
    let b = v.to_le_bytes();
    let mut o = [0u8; 16];
    for l in 0..4 {
        o[4 * l..4 * l + 4].copy_from_slice(&b);
    }
    o
}
pub fn splat64(v: u64) -> [u8; 16] {
    let b = v.to_le_bytes();
    let mut o = [0u8; 16];
    o[..8].copy_from_slice(&b);
    o[8..].copy_from_slice(&b);
    o
}

const V_AND: u32 = 0x4e;
const V_ANDNOT: u32 = 0x4f;
const V_OR: u32 = 0x50;
const V_XOR: u32 = 0x51;

/// lane width dependent SIMD opcodes
struct Lw {
    w64: bool,
}
impl Lw {
    fn shl(&self) -> u32 { if self.w64 { 0xcb } else { 0xab } }
    fn shr_s(&self) -> u32 { if self.w64 { 0xcc } else { 0xac } }
    fn add(&self) -> u32 { if self.w64 { 0xce } else { 0xae } }
    fn sub(&self) -> u32 { if self.w64 { 0xd1 } else { 0xb1 } }
    fn eq(&self) -> u32 { if self.w64 { 0xd6 } else { 0x37 } }
    fn fne(&self) -> u32 { if self.w64 { 0x48 } else { 0x42 } }
    fn all_true(&self) -> u32 { if self.w64 { 0xc3 } else { 0xa3 } }
    fn fadd(&self) -> u32 { if self.w64 { 0xf0 } else { 0xe4 } }
    fn fsub(&self) -> u32 { if self.w64 { 0xf1 } else { 0xe5 } }
    fn fmul(&self) -> u32 { if self.w64 { 0xf2 } else { 0xe6 } }
    fn sh(&self) -> i32 { if self.w64 { 63 } else { 31 } }
    fn splat(&self, v: u64) -> [u8; 16] { if self.w64 { splat64(v) } else { splat32(v as u32) } }
}

/// v128 mask on the stack: lanes of local `x` that are zero or have a biased exponent in [lo, hi]; `t` is a v128 scratch local
fn pk_win(c: &mut Code, lw: &Lw, x: u32, t: u32, lo: u64, hi: u64) {
    c.local_get(x);
    c.i32_const(1);
    c.simd(lw.shl());
    c.local_tee(t);
    let sh = if lw.w64 { 53 } else { 24 };
    c.v128_const(lw.splat(lo << sh));
    c.simd(lw.sub());
    let n = (hi - lo + 1) << sh;
    if lw.w64 {
        // no unsigned 64-bit compare in SIMD: flip the top bit of both sides and compare signed
        c.v128_const(splat64(1 << 63));
        c.simd(V_XOR);
        c.v128_const(splat64(n ^ (1 << 63)));
        c.simd(0xd8);
    } else {
        c.v128_const(splat32(n as u32));
        c.simd(0x3a);
    }
    c.local_get(t);
    c.v128_const([0; 16]);
    c.simd(lw.eq());
    c.simd(V_OR);
}

/// emits the early return (zero, 0) unless every lane of `a` and `b` is in the window
fn pk_window_or_return(c: &mut Code, lw: &Lw, a: u32, b: u32, t: u32, lo: u64, hi: u64) {
    pk_win(c, lw, a, t, lo, hi);
    pk_win(c, lw, b, t, lo, hi);
    c.simd(V_AND);
    c.simd(lw.all_true());
    c.i32_eqz();
    c.if_();
    c.v128_const([0; 16]);
    c.i32_const(0);
    c.return_();
    c.end();
}

/// v128 on the stack := s + delta, delta = the step of the sign-magnitude bits of `s` (local) by the lanes where the mask of the mode applies. `m3`, `m1`, `m2` push the mask
/// (all ones in a lane that needs the step) for RC = 3 (toward zero: -1), RC = 1 (down) and RC = 2 (up): the formulas of fle_mxr_step. `sg` is a v128 scratch local.
fn pk_step(c: &mut Code, lw: &Lw, rc: u32, s: u32, sg: u32, m3: impl FnOnce(&mut Code), m1: impl FnOnce(&mut Code), m2: impl FnOnce(&mut Code)) {
    c.local_get(s);
    c.local_get(rc);
    c.i32_const(3);
    c.i32_eq();
    c.if_ty(Ty::V128);
    m3(c);
    c.else_();
    // sg = (s >> sh) & 2
    c.local_get(s);
    c.i32_const(lw.sh());
    c.simd(lw.shr_s());
    c.v128_const(lw.splat(2));
    c.simd(V_AND);
    c.local_set(sg);
    c.local_get(rc);
    c.i32_const(1);
    c.i32_eq();
    c.if_ty(Ty::V128);
    // RD: m1 & ((sg) - 1)
    m1(c);
    c.local_get(sg);
    c.v128_const(lw.splat(1));
    c.simd(lw.sub());
    c.simd(V_AND);
    c.else_();
    // RU: m2 & (1 - sg)
    m2(c);
    c.v128_const(lw.splat(1));
    c.local_get(sg);
    c.simd(lw.sub());
    c.simd(V_AND);
    c.end();
    c.end();
    c.simd(lw.add());
}

fn pkty() -> FuncType {
    FuncType { params: vec![Ty::V128, Ty::V128, Ty::I32], results: vec![Ty::V128, Ty::I32] }
}

/// addps/addpd: params 0 a, 1 b, 2 rc; v128 locals 3 s, 4 v, 5 e, 6 t, 7 sg, 8 u
fn addp_body(w64: bool) -> Body {
    let lw = Lw { w64 };
    let mut c = Code::new();
    let (lo, hi) = if w64 { (513, 1533) } else { (65, 189) };
    pk_window_or_return(&mut c, &lw, 0, 1, 6, lo, hi);
    // S = A + B; V = S - A; E = (A - (S - V)) + (B - V)
    c.local_get(0);
    c.local_get(1);
    c.simd(lw.fadd());
    c.local_set(3);
    c.local_get(3);
    c.local_get(0);
    c.simd(lw.fsub());
    c.local_set(4);
    c.local_get(0);
    c.local_get(3);
    c.local_get(4);
    c.simd(lw.fsub());
    c.simd(lw.fsub());
    c.local_get(1);
    c.local_get(4);
    c.simd(lw.fsub());
    c.simd(lw.fadd());
    c.local_set(5);
    // nz(e) is `f.ne(e, 0)`
    let nz = |c: &mut Code, lw: &Lw| {
        c.local_get(5);
        c.v128_const([0; 16]);
        c.simd(lw.fne());
    };
    let (w3, w1, w2) = (Lw { w64 }, Lw { w64 }, Lw { w64 });
    pk_step(
        &mut c,
        &lw,
        2,
        3,
        7,
        // RZ: ((e ^ s) >> sh) & nz
        |c| {
            c.local_get(5);
            c.local_get(3);
            c.simd(V_XOR);
            c.i32_const(w3.sh());
            c.simd(w3.shr_s());
            nz(c, &w3);
            c.simd(V_AND);
        },
        // RD: (e >> sh) & nz
        |c| {
            c.local_get(5);
            c.i32_const(w1.sh());
            c.simd(w1.shr_s());
            nz(c, &w1);
            c.simd(V_AND);
        },
        // RU: nz & ~(e >> sh)
        |c| {
            nz(c, &w2);
            c.local_get(5);
            c.i32_const(w2.sh());
            c.simd(w2.shr_s());
            c.simd(V_ANDNOT);
        },
    );
    c.local_set(8);
    // RD: an exact zero sum of operands of opposite sign is -0: u |= (s<<1 == 0) & ((a ^ b) >> sh) & signbit
    c.local_get(2);
    c.i32_const(1);
    c.i32_eq();
    c.if_();
    c.local_get(8);
    c.local_get(3);
    c.i32_const(1);
    c.simd(lw.shl());
    c.v128_const([0; 16]);
    c.simd(lw.eq());
    c.local_get(0);
    c.local_get(1);
    c.simd(V_XOR);
    c.i32_const(lw.sh());
    c.simd(lw.shr_s());
    c.simd(V_AND);
    c.v128_const(lw.splat(1u64 << lw.sh()));
    c.simd(V_AND);
    c.simd(V_OR);
    c.local_set(8);
    c.end();
    c.local_get(8);
    c.i32_const(1);
    Body { locals: vec![(6, Ty::V128)], code: c }
}

/// mulpd: Dekker's TwoProduct; params 0 a, 1 b, 2 rc; v128 locals 3 s(P), 4.. ah, al, bh, bl, t, e, sg, u
fn mulpd_body() -> Body {
    let lw = Lw { w64: true };
    let mut c = Code::new();
    let (p, ah, al, bh, bl, t, e, sg, u) = (3u32, 4u32, 5u32, 6u32, 7u32, 8u32, 9u32, 10u32, 11u32);
    pk_window_or_return(&mut c, &lw, 0, 1, t, 623, 1423);
    c.local_get(0);
    c.local_get(1);
    c.simd(0xf2);
    c.local_set(p);
    for (x, h, l) in [(0u32, ah, al), (1, bh, bl)] {
        // ca = SPLIT * x; hi = ca - (ca - x); lo = x - hi
        c.v128_const(splat64(SPLIT.to_bits()));
        c.local_get(x);
        c.simd(0xf2);
        c.local_set(t);
        c.local_get(t);
        c.local_get(t);
        c.local_get(x);
        c.simd(0xf1);
        c.simd(0xf1);
        c.local_set(h);
        c.local_get(x);
        c.local_get(h);
        c.simd(0xf1);
        c.local_set(l);
    }
    // E = ((AH*BH - P) + AH*BL + AL*BH) + AL*BL
    c.local_get(ah);
    c.local_get(bh);
    c.simd(0xf2);
    c.local_get(p);
    c.simd(0xf1);
    c.local_get(ah);
    c.local_get(bl);
    c.simd(0xf2);
    c.simd(0xf0);
    c.local_get(al);
    c.local_get(bh);
    c.simd(0xf2);
    c.simd(0xf0);
    c.local_get(al);
    c.local_get(bl);
    c.simd(0xf2);
    c.simd(0xf0);
    c.local_set(e);
    let nz = |c: &mut Code| {
        c.local_get(e);
        c.v128_const([0; 16]);
        c.simd(0x48);
    };
    pk_step(
        &mut c,
        &lw,
        2,
        p,
        sg,
        |c| {
            c.local_get(e);
            c.local_get(p);
            c.simd(V_XOR);
            c.i32_const(63);
            c.simd(0xcc);
            nz(c);
            c.simd(V_AND);
        },
        |c| {
            c.local_get(e);
            c.i32_const(63);
            c.simd(0xcc);
            nz(c);
            c.simd(V_AND);
        },
        |c| {
            nz(c);
            c.local_get(e);
            c.i32_const(63);
            c.simd(0xcc);
            c.simd(V_ANDNOT);
        },
    );
    c.local_set(u);
    c.local_get(u);
    c.i32_const(1);
    Body { locals: vec![(9, Ty::V128)], code: c }
}

/// mulps: the exact double product of the lane pairs against the binary32 result; params 0 a, 1 b, 2 rc; v128 locals 3 dl, 4 dh, 5 s, 6 pl, 7 ph, 8 t, 9 sg, 10 u
fn mulps_body() -> Body {
    let lw = Lw { w64: false };
    let mut c = Code::new();
    let (dl, dh, s, pl, ph, t, sg, u) = (3u32, 4u32, 5u32, 6u32, 7u32, 8u32, 9u32, 10u32);
    pk_window_or_return(&mut c, &lw, 0, 1, t, 65, 189);
    let swap: [u8; 16] = [8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7];
    // low halves
    c.local_get(0);
    c.simd(0x5f);
    c.local_get(1);
    c.simd(0x5f);
    c.simd(0xf2);
    c.local_set(dl);
    // high halves
    c.local_get(0);
    c.local_get(0);
    c.simd_shuffle(swap);
    c.simd(0x5f);
    c.local_get(1);
    c.local_get(1);
    c.simd_shuffle(swap);
    c.simd(0x5f);
    c.simd(0xf2);
    c.local_set(dh);
    // s = the binary32 results (round to nearest even), lanes [dl0 dl1 dh0 dh1]
    c.local_get(dl);
    c.simd(0x5e);
    c.local_get(dh);
    c.simd(0x5e);
    c.simd_shuffle([0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23]);
    c.local_set(s);
    c.local_get(s);
    c.simd(0x5f);
    c.local_set(pl);
    c.local_get(s);
    c.local_get(s);
    c.simd_shuffle(swap);
    c.simd(0x5f);
    c.local_set(ph);
    let pack: [u8; 16] = [0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 24, 25, 26, 27];
    // mask of the lanes whose exact product is below / above the result (strictly): `cmp` is the f64x2 compare opcode (lt 0x49, gt 0x4a); abs = true compares magnitudes
    let mk = |c: &mut Code, cmp: u32, abs: bool| {
        for (d, p) in [(dl, pl), (dh, ph)] {
            c.local_get(d);
            if abs {
                c.simd(0xec);
            }
            c.local_get(p);
            if abs {
                c.simd(0xec);
            }
            c.simd(cmp);
        }
        c.simd_shuffle(pack);
    };
    pk_step(&mut c, &lw, 2, s, sg, |c| mk(c, 0x49, true), |c| mk(c, 0x49, false), |c| mk(c, 0x4a, false));
    c.local_set(u);
    c.local_get(u);
    c.i32_const(1);
    Body { locals: vec![(8, Ty::V128)], code: c }
}

pub fn add_rc_packed(m: &mut Module) -> RcPacked {
    let addps = m.add_func(pkty(), addp_body(false));
    let mulps = m.add_func(pkty(), mulps_body());
    let addpd = m.add_func(pkty(), addp_body(true));
    let mulpd = m.add_func(pkty(), mulpd_body());
    RcPacked { addps, mulps, addpd, mulpd }
}

/// for `lower.rs`: v128 mask on the stack of the lanes of local `x` that are zero or have a biased exponent in [lo, hi] (`t`: a v128 scratch local)
pub fn window_mask(c: &mut Code, w64: bool, x: u32, t: u32, lo: u64, hi: u64) {
    pk_win(c, &Lw { w64 }, x, t, lo, hi);
}

/// for `lower.rs`: v128 mask on the stack of the lanes of local `x` that are zero, normal or infinite (no NaN, no denormal): the operands of max/min that raise no flag
pub fn ord_mask(c: &mut Code, w64: bool, x: u32, t: u32) {
    let lw = Lw { w64 };
    c.local_get(x);
    c.i32_const(1);
    c.simd(lw.shl());
    c.local_tee(t);
    if w64 {
        c.v128_const(splat64(1 << 53));
        c.simd(lw.sub());
        c.v128_const(splat64(1 << 63));
        c.simd(V_XOR);
        c.v128_const(splat64((0xffe0_0000_0000_0000u64 - (1u64 << 53) + 1) ^ (1 << 63)));
        c.simd(0xd8);
    } else {
        c.v128_const(splat32(1 << 24));
        c.simd(lw.sub());
        c.v128_const(splat32(0xff00_0000u32 - (1u32 << 24) + 1));
        c.simd(0x3a);
    }
    c.local_get(t);
    c.v128_const([0; 16]);
    c.simd(lw.eq());
    c.simd(V_OR);
}

/// for `lower.rs`: i32 on the stack := all lanes of the v128 mask on the stack are set
pub fn mask_all(c: &mut Code, w64: bool) {
    c.simd(Lw { w64 }.all_true());
}
pub fn mask_and(c: &mut Code) {
    c.simd(V_AND);
}

/// cvtsd2ss under RC != 0: rc_cvtsd2ss(a: i64 bits of a double, rc: i32) -> (i32 bits of the binary32 result, 1); (0, 0) outside the window (zero, or 2^-100 <= |x| <= 2^100 as for the
/// C path's window of the conversion). s = (float)x (nearest even), e = x - (double)s is exact in binary64, the step by the sign of e as in the other helpers.
fn cvtsd2ss_body() -> Body {
    // params 0 a, 1 rc; locals i32: 2 sb, 3 ssg, 4 esg, 5 nz; i64: 6 eb; f64: 7 X, 8 E
    let mut c = Code::new();
    win64(&mut c, 0, 923, 1123);
    c.i32_eqz();
    c.if_();
    c.i32_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    c.local_get(0);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(7);
    c.local_get(7);
    c.raw(&[F32_DEMOTE_F64, I32_REINTERPRET_F32]);
    c.local_set(2);
    c.local_get(7);
    c.local_get(2);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_SUB]);
    c.local_set(8);
    c.local_get(8);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(6);
    c.i32_const(0);
    c.local_get(6);
    c.i64_const(63);
    c.i64_shr_u();
    c.i32_wrap_i64();
    c.i32_sub();
    c.local_set(4);
    c.i32_const(0);
    c.local_get(6);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(0);
    c.i64_ne();
    c.i32_sub();
    c.local_set(5);
    step32(&mut c, 1, 2, 4, 5, 3);
    c.i32_const(1);
    Body { locals: vec![(4, Ty::I32), (1, Ty::I64), (2, Ty::F64)], code: c }
}

/// zero-excluding window test of the bits in local `t`: biased exponent in [lo, hi] (i32 on the stack)
fn win64nz(c: &mut Code, t: u32, lo: i64, hi: i64) {
    c.local_get(t);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(lo << 53);
    c.i64_sub();
    c.i64_const((hi - lo + 1) << 53);
    c.i64_lt_u();
}
fn win32nz(c: &mut Code, t: u32, lo: i32, hi: i32) {
    c.local_get(t);
    c.i32_const(1);
    c.i32_shl();
    c.i32_const(lo << 24);
    c.i32_sub();
    c.i32_const((hi - lo + 1) << 24);
    c.i32_lt_u();
}

const F64_DIV: u8 = 0xa3;
const F64_SQRT: u8 = 0x9f;
const F32_DIV: u8 = 0x95;
const F32_SQRT: u8 = 0x91;

/// emits: f64 locals (x, h, l) := the Veltkamp split of the f64 local `v` with the scratch `t`:  t = SPLIT * v; h = t - (t - v); l = v - h
fn split_f64(c: &mut Code, v: u32, t: u32, h: u32, l: u32) {
    c.f64_const(SPLIT);
    c.local_get(v);
    c.raw(&[F64_MUL]);
    c.local_set(t);
    c.local_get(t);
    c.local_get(t);
    c.local_get(v);
    c.raw(&[F64_SUB, F64_SUB]);
    c.local_set(h);
    c.local_get(v);
    c.local_get(h);
    c.raw(&[F64_SUB]);
    c.local_set(l);
}

/// rc_div64(a, b, rc): a zero or 2^-400..2^400, b nonzero in the window; q = a / b, the remainder r = (a - q*b) by Dekker's TwoProduct of (q, b); the sign of the error of q is sign(r) * sign(b)
fn div64_body() -> Body {
    // params 0 a, 1 b, 2 rc; locals i64: 3 sb, 4 eb, 5 ssg, 6 esg, 7 nz; f64: 8 A, 9 B, 10 Q, 11 P, 12 QH, 13 QL, 14 BH, 15 BL, 16 T, 17 E, 18 R
    let mut c = Code::new();
    win64(&mut c, 0, 623, 1423);
    win64nz(&mut c, 1, 623, 1423);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i64_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    c.local_get(0);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(8);
    c.local_get(1);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(9);
    c.local_get(8);
    c.local_get(9);
    c.raw(&[F64_DIV]);
    c.local_set(10);
    c.local_get(10);
    c.local_get(9);
    c.raw(&[F64_MUL]);
    c.local_set(11);
    split_f64(&mut c, 10, 16, 12, 13);
    split_f64(&mut c, 9, 16, 14, 15);
    // E = ((QH*BH - P) + QH*BL + QL*BH) + QL*BL
    c.local_get(12);
    c.local_get(14);
    c.raw(&[F64_MUL]);
    c.local_get(11);
    c.raw(&[F64_SUB]);
    c.local_get(12);
    c.local_get(15);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(13);
    c.local_get(14);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(13);
    c.local_get(15);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_set(17);
    // R = (A - P) - E
    c.local_get(8);
    c.local_get(11);
    c.raw(&[F64_SUB]);
    c.local_get(17);
    c.raw(&[F64_SUB]);
    c.local_set(18);
    c.local_get(10);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(3);
    // eb = bits(R) ^ (b & sign)
    c.local_get(18);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_get(1);
    c.i64_const(i64::MIN);
    c.i64_and();
    c.i64_xor();
    c.local_set(4);
    step64(&mut c, 2, 3, 4, 5, 6, 7);
    c.i32_const(1);
    Body { locals: vec![(5, Ty::I64), (11, Ty::F64)], code: c }
}

/// rc_sqrt64(b, rc): +-0 or 2^-400 <= x <= 2^400 (positive: the sign bit is part of the test)
fn sqrt64_body() -> Body {
    // params 0 b, 1 rc; locals i64: 2 sb, 3 eb, 4 ssg, 5 esg, 6 nz; f64: 7 B, 8 Q, 9 P, 10 QH, 11 QL, 12 T, 13 E, 14 R
    let mut c = Code::new();
    c.local_get(0);
    c.i64_const(623 << 52);
    c.i64_sub();
    c.i64_const(801 << 52);
    c.i64_lt_u();
    c.local_get(0);
    c.i64_const(1);
    c.i64_shl();
    c.i64_eqz();
    c.i32_or();
    c.i32_eqz();
    c.if_();
    c.i64_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    c.local_get(0);
    c.raw(&[F64_REINTERPRET_I64]);
    c.local_set(7);
    c.local_get(7);
    c.raw(&[F64_SQRT]);
    c.local_set(8);
    c.local_get(8);
    c.local_get(8);
    c.raw(&[F64_MUL]);
    c.local_set(9);
    split_f64(&mut c, 8, 12, 10, 11);
    // E = ((QH*QH - P) + QH*QL + QL*QH) + QL*QL
    c.local_get(10);
    c.local_get(10);
    c.raw(&[F64_MUL]);
    c.local_get(9);
    c.raw(&[F64_SUB]);
    c.local_get(10);
    c.local_get(11);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(11);
    c.local_get(10);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_get(11);
    c.local_get(11);
    c.raw(&[F64_MUL, F64_ADD]);
    c.local_set(13);
    c.local_get(7);
    c.local_get(9);
    c.raw(&[F64_SUB]);
    c.local_get(13);
    c.raw(&[F64_SUB]);
    c.local_set(14);
    c.local_get(8);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(2);
    c.local_get(14);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(3);
    step64(&mut c, 1, 2, 3, 4, 5, 6);
    c.i32_const(1);
    Body { locals: vec![(5, Ty::I64), (8, Ty::F64)], code: c }
}

/// masks esg / nz (i32 locals) from the binary64 error bits in the i64 local `eb`
fn masks32_from64(c: &mut Code, eb: u32, esg: u32, nz: u32) {
    c.i32_const(0);
    c.local_get(eb);
    c.i64_const(63);
    c.i64_shr_u();
    c.i32_wrap_i64();
    c.i32_sub();
    c.local_set(esg);
    c.i32_const(0);
    c.local_get(eb);
    c.i64_const(1);
    c.i64_shl();
    c.i64_const(0);
    c.i64_ne();
    c.i32_sub();
    c.local_set(nz);
}

/// rc_div32(a, b, rc): a zero or in 2^-62..2^62, b nonzero in it; q = a / b in binary32, r = (double)a - (double)q * (double)b exact
fn div32_body() -> Body {
    // params 0 a, 1 b, 2 rc (i32); locals i32: 3 sb, 4 ssg, 5 esg, 6 nz; i64: 7 eb; f64: 8 R
    let mut c = Code::new();
    win32(&mut c, 0, 65, 189);
    win32nz(&mut c, 1, 65, 189);
    c.i32_and();
    c.i32_eqz();
    c.if_();
    c.i32_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    // q (bits) = f32.div(a, b)
    c.local_get(0);
    c.raw(&[F32_REINTERPRET_I32]);
    c.local_get(1);
    c.raw(&[F32_REINTERPRET_I32, F32_DIV, I32_REINTERPRET_F32]);
    c.local_set(3);
    // R = (double)a - (double)q * (double)b
    c.local_get(0);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32]);
    c.local_get(3);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32]);
    c.local_get(1);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_MUL, F64_SUB]);
    c.local_set(8);
    // eb = bits(R) ^ ((b << 32) & sign64)
    c.local_get(8);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_get(1);
    c.i64_extend_i32_u();
    c.i64_const(32);
    c.i64_shl();
    c.i64_const(i64::MIN);
    c.i64_and();
    c.i64_xor();
    c.local_set(7);
    masks32_from64(&mut c, 7, 5, 6);
    step32(&mut c, 2, 3, 5, 6, 4);
    c.i32_const(1);
    Body { locals: vec![(4, Ty::I32), (1, Ty::I64), (1, Ty::F64)], code: c }
}

/// rc_sqrt32(b, rc): +0 / -0 or a positive number in 2^-62..2^62; q = (float)sqrt((double)b), r = (double)b - (double)q * (double)q
fn sqrt32_body() -> Body {
    // params 0 b, 1 rc; locals i32: 2 sb, 3 ssg, 4 esg, 5 nz; i64: 6 eb; f64: 7 R
    let mut c = Code::new();
    c.local_get(0);
    c.i32_const(65 << 23);
    c.i32_sub();
    c.i32_const(125 << 23);
    c.i32_lt_u();
    c.local_get(0);
    c.i32_const(1);
    c.i32_shl();
    c.i32_eqz();
    c.i32_or();
    c.i32_eqz();
    c.if_();
    c.i32_const(0);
    c.i32_const(0);
    c.return_();
    c.end();
    c.local_get(0);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_SQRT, F32_DEMOTE_F64, I32_REINTERPRET_F32]);
    c.local_set(2);
    c.local_get(0);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32]);
    c.local_get(2);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32]);
    c.local_get(2);
    c.raw(&[F32_REINTERPRET_I32, F64_PROMOTE_F32, F64_MUL, F64_SUB]);
    c.local_set(7);
    c.local_get(7);
    c.raw(&[I64_REINTERPRET_F64]);
    c.local_set(6);
    masks32_from64(&mut c, 6, 4, 5);
    step32(&mut c, 1, 2, 4, 5, 3);
    c.i32_const(1);
    Body { locals: vec![(4, Ty::I32), (1, Ty::I64), (1, Ty::F64)], code: c }
}

