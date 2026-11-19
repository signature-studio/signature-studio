//! Decision 0181: the directed-rounding helpers of DIVSD, SQRTSD, DIVSS, SQRTSS (`rcgen::install`; H_DIV64, H_SQRT64, H_DIV32, H_SQRT32) run under node on random operands in and around
//! their windows with RC = 1, 2, 3 (FTZ, DAZ at random) and compared with the host CPU's instruction. A case in the window must give the host's bits and ok = 1, any other ok = 0.
//! Skipped (and passing) when node is missing or the host is not x86-64. FLE_RC_CASES sets the number of cases per helper (default 2,000,000).
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::rcgen;
use fe_wasmgen::wasm::Module;
use std::arch::asm;
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
            asm!("divsd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else {
            asm!("sqrtsd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) f64::from_bits(a), options(nostack));
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
            asm!("divss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack));
        } else {
            asm!("sqrtss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) f32::from_bits(a), options(nostack));
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
fn w64(x: u64, lo: u64, hi: u64, zero: bool) -> bool {
    let t = x & 0x7fff_ffff_ffff_ffff;
    (zero && t == 0) || ((t >> 52) >= lo && (t >> 52) <= hi)
}
fn w32(x: u32, lo: u32, hi: u32, zero: bool) -> bool {
    let t = x & 0x7fff_ffff;
    (zero && t == 0) || ((t >> 23) >= lo && (t >> 23) <= hi)
}

#[test]
fn div_sqrt_helpers_match_the_host_cpu() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is missing: skipped");
        return;
    }
    let n: usize = std::env::var("FLE_RC_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(2_000_000);
    let mut m = Module::default();
    rcgen::install(&mut m);
    for (nm, h) in [("div64", rcgen::H_DIV64), ("sqrt64", rcgen::H_SQRT64), ("div32", rcgen::H_DIV32), ("sqrt32", rcgen::H_SQRT32)] {
        m.exports.push((nm.into(), 0, rcgen::helper(h)));
    }
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rc_divsqrt");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("c.wasm"), m.encode()).unwrap();
    // case: kind u32, rc u32, a u64, b u64, expected u64, ok u32, pad u32 = 40 bytes
    let mut r = Rng(0xfeedface12345678);
    let mut buf: Vec<u8> = Vec::with_capacity(n * 4 * 40);
    for kind in 0..4u32 {
        for _ in 0..n {
            let rc = 1 + r.below(3) as u32;
            let mx = 0x1f80u32 | rc << 13 | (r.below(2) as u32) << 15 | (r.below(2) as u32) << 6;
            let (a, b, exp, ok): (u64, u64, u64, u32) = match kind {
                0 => {
                    let (a, b) = (operand64(&mut r, 623, 1423), operand64(&mut r, 623, 1423));
                    (a, b, host64(0, a, b, mx), (w64(a, 623, 1423, true) && w64(b, 623, 1423, false)) as u32)
                }
                1 => {
                    let a = operand64(&mut r, 623, 1423);
                    (a, 0, host64(1, a, 0, mx), ((a >> 63 == 0 && w64(a, 623, 1423, true)) || (a << 1 == 0)) as u32)
                }
                2 => {
                    let (a, b) = (operand32(&mut r, 65, 189), operand32(&mut r, 65, 189));
                    (a as u64, b as u64, host32(0, a, b, mx) as u64, (w32(a, 65, 189, true) && w32(b, 65, 189, false)) as u32)
                }
                _ => {
                    let a = operand32(&mut r, 65, 189);
                    (a as u64, 0, host32(1, a, 0, mx) as u64, ((a >> 31 == 0 && w32(a, 65, 189, true)) || (a << 1 == 0)) as u32)
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
const e = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(dir + '/c.wasm')), {}).exports;
const b = fs.readFileSync(dir + '/cases.bin'); const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
const names = ['div64', 'sqrt64', 'div32', 'sqrt32'];
const st = names.map(() => ({ n: 0, ok: 0, mism: 0, okbad: 0, byrc: [0, 0, 0, 0] }));
for (let o = 0; o + 40 <= b.length; o += 40) {
  const kind = dv.getUint32(o, true), rc = dv.getUint32(o + 4, true);
  const a = dv.getBigUint64(o + 8, true), bb = dv.getBigUint64(o + 16, true), exp = dv.getBigUint64(o + 24, true), eok = dv.getUint32(o + 32, true);
  let r;
  if (kind === 0) r = e.div64(BigInt.asIntN(64, a), BigInt.asIntN(64, bb), rc);
  else if (kind === 1) r = e.sqrt64(BigInt.asIntN(64, a), rc);
  else if (kind === 2) r = e.div32(Number(a) | 0, Number(bb) | 0, rc);
  else r = e.sqrt32(Number(a) | 0, rc);
  const val = kind < 2 ? BigInt.asUintN(64, r[0]) : BigInt(r[0] >>> 0);
  const s = st[kind]; s.n++;
  if (r[1] !== eok) { s.okbad++; if (s.okbad <= 3) console.log('OKBAD', names[kind], rc, a.toString(16), bb.toString(16), 'ok', r[1], 'expected', eok); continue; }
  if (r[1]) { s.ok++; s.byrc[rc]++; if (val !== exp) { s.mism++; if (s.mism <= 5) console.log('MISMATCH', names[kind], 'rc', rc, a.toString(16), bb.toString(16), 'got', val.toString(16), 'host', exp.toString(16)); } }
}
let bad = 0;
for (let k = 0; k < 4; k++) { const s = st[k]; console.log('RCDIVSQRT', names[k], 'cases', s.n, 'ok', s.ok, 'rc1', s.byrc[1], 'rc2', s.byrc[2], 'rc3', s.byrc[3], 'mismatches', s.mism, 'window_disagreements', s.okbad); bad += s.mism + s.okbad; }
process.exit(bad ? 1 : 0);
"#;
    std::fs::write(dir.join("run.js"), js).unwrap();
    let out = Command::new("node").arg(dir.join("run.js")).arg(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{}", text);
    assert!(out.status.success(), "node: {} {}", text, String::from_utf8_lossy(&out.stderr));
}
