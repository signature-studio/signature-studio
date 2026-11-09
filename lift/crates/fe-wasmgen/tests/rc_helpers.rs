//! Differential test of the directed-rounding helpers of `rcgen.rs` (decision 0172): the four wasm functions (scalar add/sub and mul, binary64 and binary32) are run under node on
//! random operands in and around their windows with RC = 1, 2, 3 and compared with the instruction run on the host CPU with that rounding control (and FTZ, DAZ at random). Skipped
//! (and passing) when node is missing or the host is not x86-64. FLE_RC_CASES sets the number of cases per helper (default 400,000).
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::rcgen::add_rc_helpers;
use fe_wasmgen::wasm::Module;
use std::arch::asm;
use std::io::Write;
use std::process::Command;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545f4914f6cdd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 11) % n
    }
}

fn host64(op: u8, a: u64, b: u64, mx: u32) -> u64 {
    let (mut r, bb) = (f64::from_bits(a), f64::from_bits(b));
    unsafe {
        let v = mx;
        asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack));
        if op == 0 {
            asm!("addsd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else if op == 2 {
            asm!("subsd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else {
            asm!("mulsd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        }
        let d = 0x1f80u32;
        asm!("ldmxcsr [{0}]", in(reg) &d, options(nostack));
    }
    r.to_bits()
}

fn host32(op: u8, a: u32, b: u32, mx: u32) -> u32 {
    let (mut r, bb) = (f32::from_bits(a), f32::from_bits(b));
    unsafe {
        let v = mx;
        asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack));
        if op == 0 {
            asm!("addss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else if op == 2 {
            asm!("subss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else {
            asm!("mulss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        }
        let d = 0x1f80u32;
        asm!("ldmxcsr [{0}]", in(reg) &d, options(nostack));
    }
    r.to_bits()
}

fn mant(r: &mut Rng, bits: u32) -> u64 {
    let full = (1u64 << bits) - 1;
    match r.below(10) {
        0 => 0,
        1 => full,
        2 => 1,
        3 => 1 << (bits - 1),
        4 => (1 << (bits - 1)) - 1,
        5 => r.next() & full & (r.next() | r.next()),
        6 => (r.next() & full) >> r.below(bits as u64),
        _ => r.next() & full,
    }
}

/// an operand: zero, or an exponent near/inside/outside the window [lo, hi] (biased exponents), either sign
fn operand64(r: &mut Rng, lo: i64, hi: i64) -> u64 {
    let e = match r.below(10) {
        0 => 0,
        1 => lo - 2 + r.below(5) as i64,
        2 => hi - 2 + r.below(5) as i64,
        3 => 1 + r.below(2046) as i64,
        4 => 2047,
        _ => 1023 - 40 + r.below(81) as i64,
    };
    let m = mant(r, 52);
    let z = r.below(12) == 0;
    ((r.next() & 1) << 63) | (if z { 0 } else { (e.clamp(0, 2047) as u64) << 52 | m } & 0x7fff_ffff_ffff_ffff)
}

fn operand32(r: &mut Rng, lo: i64, hi: i64) -> u32 {
    let e = match r.below(10) {
        0 => 0,
        1 => lo - 2 + r.below(5) as i64,
        2 => hi - 2 + r.below(5) as i64,
        3 => 1 + r.below(254) as i64,
        4 => 255,
        _ => 127 - 20 + r.below(41) as i64,
    };
    let m = mant(r, 23);
    let z = r.below(12) == 0;
    (((r.next() & 1) as u32) << 31) | (if z { 0 } else { (e.clamp(0, 255) as u32) << 23 | m as u32 } & 0x7fff_ffff)
}

fn win64(x: u64, lo: u64, hi: u64) -> bool {
    let t = x & 0x7fff_ffff_ffff_ffff;
    t == 0 || ((t >> 52) >= lo && (t >> 52) <= hi)
}
fn win32(x: u32, lo: u32, hi: u32) -> bool {
    let t = x & 0x7fff_ffff;
    t == 0 || ((t >> 23) >= lo && (t >> 23) <= hi)
}

#[test]
fn helpers_match_the_host_cpu() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is missing: skipped");
        return;
    }
    let n: usize = std::env::var("FLE_RC_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(400_000);
    let mut m = Module::default();
    let h = add_rc_helpers(&mut m);
    m.exports.push(("addsub64".into(), 0, h.addsub64));
    m.exports.push(("mul64".into(), 0, h.mul64));
    m.exports.push(("addsub32".into(), 0, h.addsub32));
    m.exports.push(("mul32".into(), 0, h.mul32));
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rc_helpers");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("rc.wasm"), m.encode()).unwrap();
    // cases: kind u32 (0 addsub64, 1 mul64, 2 addsub32, 3 mul32), rc u32, a u64, b u64, expected u64, expected ok u32, pad u32  = 40 bytes
    let mut r = Rng(0x9e3779b97f4a7c15);
    let mut buf: Vec<u8> = Vec::with_capacity(n * 4 * 40);
    for kind in 0..4u32 {
        for _ in 0..n {
            let rc = 1 + r.below(3) as u32;
            let mx = 0x1f80u32 | rc << 13 | (r.below(2) as u32) << 15 | (r.below(2) as u32) << 6;
            let (a, b, exp, ok): (u64, u64, u64, u32) = match kind {
                0 => {
                    let (a, mut b) = (operand64(&mut r, 513, 1533), operand64(&mut r, 513, 1533));
                    if r.below(3) == 0 {
                        b = a ^ (1 << 63); // an exact cancellation
                    }
                    let sub = r.below(2) == 1;
                    // a subtraction passes the second operand with its sign flipped: the helper computes a + b' where b' = -b, the host runs subsd a, b
                    let (bp, host) = if sub { (b ^ (1 << 63), host64(2, a, b, mx)) } else { (b, host64(0, a, b, mx)) };
                    (a, bp, host, (win64(a, 513, 1533) && win64(bp, 513, 1533)) as u32)
                }
                1 => {
                    let (a, b) = (operand64(&mut r, 623, 1423), operand64(&mut r, 623, 1423));
                    (a, b, host64(1, a, b, mx), (win64(a, 623, 1423) && win64(b, 623, 1423)) as u32)
                }
                2 => {
                    let (a, mut b) = (operand32(&mut r, 65, 189), operand32(&mut r, 65, 189));
                    if r.below(3) == 0 {
                        b = a ^ (1 << 31);
                    }
                    let sub = r.below(2) == 1;
                    let (bp, host) = if sub { (b ^ (1 << 31), host32(2, a, b, mx)) } else { (b, host32(0, a, b, mx)) };
                    (a as u64, bp as u64, host as u64, (win32(a, 65, 189) && win32(bp, 65, 189)) as u32)
                }
                _ => {
                    let (a, b) = (operand32(&mut r, 65, 189), operand32(&mut r, 65, 189));
                    (a as u64, b as u64, host32(1, a, b, mx) as u64, (win32(a, 65, 189) && win32(b, 65, 189)) as u32)
                }
            };
            buf.extend_from_slice(&kind.to_le_bytes());
            buf.extend_from_slice(&rc.to_le_bytes());
            buf.extend_from_slice(&a.to_le_bytes());
            buf.extend_from_slice(&b.to_le_bytes());
            buf.extend_from_slice(&exp.to_le_bytes());
            buf.extend_from_slice(&ok.to_le_bytes());
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
    }
    std::fs::write(dir.join("cases.bin"), &buf).unwrap();
    let js = r#"
const fs = require('fs');
const dir = process.argv[2];
const inst = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(dir + '/rc.wasm')), {});
const e = inst.exports;
const b = fs.readFileSync(dir + '/cases.bin');
const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
const names = ['addsub64', 'mul64', 'addsub32', 'mul32'];
const stat = names.map(() => ({ cases: 0, ok: 0, mism: 0, okbad: 0, byrc: [0, 0, 0, 0] }));
for (let o = 0; o + 40 <= b.length; o += 40) {
  const kind = dv.getUint32(o, true), rc = dv.getUint32(o + 4, true);
  const a = dv.getBigUint64(o + 8, true), bb = dv.getBigUint64(o + 16, true), exp = dv.getBigUint64(o + 24, true), eok = dv.getUint32(o + 32, true);
  let r;
  if (kind < 2) r = e[names[kind]](BigInt.asIntN(64, a), BigInt.asIntN(64, bb), rc);
  else r = e[names[kind]](Number(a) | 0, Number(bb) | 0, rc);
  const val = kind < 2 ? BigInt.asUintN(64, r[0]) : BigInt(r[0] >>> 0);
  const ok = r[1];
  const s = stat[kind];
  s.cases++;
  if (ok !== eok) { s.okbad++; if (s.okbad <= 3) console.log('OKBAD', names[kind], rc, a.toString(16), bb.toString(16), 'ok', ok, 'expected', eok); continue; }
  if (ok) { s.ok++; s.byrc[rc]++; if (val !== exp) { s.mism++; if (s.mism <= 5) console.log('MISMATCH', names[kind], 'rc', rc, a.toString(16), bb.toString(16), 'got', val.toString(16), 'host', exp.toString(16)); } }
}
let bad = 0;
for (let k = 0; k < 4; k++) { const s = stat[k]; console.log('RCHELPER', names[k], 'cases', s.cases, 'ok', s.ok, 'rc1', s.byrc[1], 'rc2', s.byrc[2], 'rc3', s.byrc[3], 'mismatches', s.mism, 'window_disagreements', s.okbad); bad += s.mism + s.okbad; }
process.exit(bad ? 1 : 0);
"#;
    std::fs::File::create(dir.join("run.js")).unwrap().write_all(js.as_bytes()).unwrap();
    let out = Command::new("node").arg(dir.join("run.js")).arg(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{}", text);
    assert!(out.status.success(), "node: {} {}", text, String::from_utf8_lossy(&out.stderr));
}

/// Cost of a helper call in a wasm loop against the plain `f64.add` (the RC = 0 inline operation): ns per operation under node, printed, not asserted.
#[test]
fn helper_cost_in_a_loop() {
    use fe_wasmgen::wasm::{Body, Code, FuncType, Ty};
    if Command::new("node").arg("--version").output().is_err() {
        return;
    }
    let mut m = Module::default();
    let h = add_rc_helpers(&mut m);
    // (func (param n i32) (param rc i32) (param which i32) (result i64)): which = 0 f64.add of the two patterns, 1 the add helper, 2 the mul helper
    // locals: 3 i (i32), 4 x (i64), 5 y (i64), 6 acc (i64)
    let mut c = Code::new();
    c.block();
    c.loop_();
    c.local_get(3);
    c.local_get(0);
    c.i32_ge_u();
    c.br_if(1);
    // x = 0x3fe0000000000000 | (((i+1) * 0x9e3779b97f4a7c15) >> 12 & 0xfffffffffffff) ; y = 0x3fd0000000000000 | (((i+7) * 0xd1342543de82ef95) >> 12 & 0xfffffffffffff)
    for (dst, add, mulc, hi) in [(4u32, 1i64, 0x9e3779b97f4a7c15u64 as i64, 0x3fe0000000000000i64), (5, 7, 0xd1342543de82ef95u64 as i64, 0x3fd0000000000000i64)] {
        c.local_get(3);
        c.i64_extend_i32_u();
        c.i64_const(add);
        c.i64_add();
        c.i64_const(mulc);
        c.i64_mul();
        c.i64_const(12);
        c.i64_shr_u();
        c.i64_const(0xf_ffff_ffff_ffff);
        c.i64_and();
        c.i64_const(hi);
        c.i64_or();
        c.local_set(dst);
    }
    c.local_get(2);
    c.i32_eqz();
    c.if_ty(Ty::I64);
    c.local_get(4);
    c.raw(&[0xbf]);
    c.local_get(5);
    c.raw(&[0xbf, 0xa0, 0xbd]);
    c.else_();
    c.local_get(2);
    c.i32_const(1);
    c.i32_eq();
    c.if_ty(Ty::I64);
    c.local_get(4);
    c.local_get(5);
    c.local_get(1);
    c.call(h.addsub64);
    c.drop();
    c.else_();
    c.local_get(4);
    c.local_get(5);
    c.local_get(1);
    c.call(h.mul64);
    c.drop();
    c.end();
    c.end();
    c.local_get(6);
    c.i64_xor();
    c.local_set(6);
    c.local_get(3);
    c.i32_const(1);
    c.i32_add();
    c.local_set(3);
    c.br(0);
    c.end();
    c.end();
    c.local_get(6);
    let f = m.add_func(FuncType { params: vec![Ty::I32, Ty::I32, Ty::I32], results: vec![Ty::I64] }, Body { locals: vec![(1, Ty::I32), (3, Ty::I64)], code: c });
    m.exports.push(("loop".into(), 0, f));
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rc_helpers");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("loop.wasm"), m.encode()).unwrap();
    let js = r#"
const fs = require('fs');
const e = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(process.argv[2] + '/loop.wasm')), {}).exports;
const N = 20000000;
function t(rc, which) { e.loop(100000, rc, which); let best = 1e30; for (let r = 0; r < 5; r++) { const t0 = process.hrtime.bigint(); e.loop(N, rc, which); const d = Number(process.hrtime.bigint() - t0) / N; if (d < best) best = d; } return best; }
const base = t(3, 0);
console.log('RCLOOP f64.add inline (RC=0 operation)', base.toFixed(2), 'ns');
for (const rc of [1, 3]) console.log('RCLOOP helper add rc', rc, t(rc, 1).toFixed(2), 'ns; mul', t(rc, 2).toFixed(2), 'ns');
"#;
    std::fs::write(dir.join("loop.js"), js).unwrap();
    let out = Command::new("node").arg(dir.join("loop.js")).arg(&dir).output().unwrap();
    eprintln!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
}
