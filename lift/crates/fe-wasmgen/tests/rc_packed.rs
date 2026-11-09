//! Differential test of the packed directed-rounding helpers of `rcgen.rs` (decision 0181): addps/subps, mulps, addpd/subpd, mulpd as wasm SIMD functions, run under node on random
//! operands in and around their windows with RC = 1, 2, 3 (and FTZ, DAZ at random) and compared with the instruction run on the host CPU with that rounding control. A case where every
//! lane of both operands is in the window must give the host's 128 bits and ok = 1; any other case must give ok = 0. Skipped (and passing) when node is missing or the host is not x86-64.
//! FLE_RC_CASES sets the number of cases per helper (default 300,000).
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::rcgen::add_rc_packed;
use fe_wasmgen::wasm::{Body, Code, FuncType, Module, Ty};
use std::arch::asm;
use std::arch::x86_64::__m128;
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

fn host(op: u8, w64: bool, a: [u8; 16], b: [u8; 16], mx: u32) -> [u8; 16] {
    let mut r: __m128 = unsafe { std::mem::transmute(a) };
    let bb: __m128 = unsafe { std::mem::transmute(b) };
    unsafe {
        let v = mx;
        asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack));
        match (op, w64) {
            (0, false) => asm!("addps {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
            (1, false) => asm!("subps {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
            (2, false) => asm!("mulps {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
            (0, true) => asm!("addpd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
            (1, true) => asm!("subpd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
            _ => asm!("mulpd {0}, {1}", inout(xmm_reg) r, in(xmm_reg) bb, options(nostack)),
        }
        let d = 0x1f80u32;
        asm!("ldmxcsr [{0}]", in(reg) &d, options(nostack));
    }
    unsafe { std::mem::transmute(r) }
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

/// a lane: mostly zero or inside the window (exponents spread over it, many near the same exponent), sometimes at its edges or outside
fn lane64(r: &mut Rng, lo: i64, hi: i64, center: i64) -> u64 {
    let e = match r.below(40) {
        0 => lo - 2 + r.below(5) as i64,
        1 => hi - 2 + r.below(5) as i64,
        2 => 1 + r.below(2046) as i64,
        3 => 2047,
        4..=15 => lo + r.below((hi - lo + 1) as u64) as i64,
        _ => center - 30 + r.below(61) as i64,
    };
    let z = r.below(14) == 0;
    ((r.next() & 1) << 63) | (if z { 0 } else { (e.clamp(0, 2047) as u64) << 52 | mant(r, 52) } & 0x7fff_ffff_ffff_ffff)
}
fn lane32(r: &mut Rng, lo: i64, hi: i64, center: i64) -> u32 {
    let e = match r.below(40) {
        0 => lo - 2 + r.below(5) as i64,
        1 => hi - 2 + r.below(5) as i64,
        2 => 1 + r.below(254) as i64,
        3 => 255,
        4..=15 => lo + r.below((hi - lo + 1) as u64) as i64,
        _ => center - 12 + r.below(25) as i64,
    };
    let z = r.below(14) == 0;
    (((r.next() & 1) as u32) << 31) | (if z { 0 } else { (e.clamp(0, 255) as u32) << 23 | mant(r, 23) as u32 } & 0x7fff_ffff)
}

fn in64(x: u64, lo: u64, hi: u64) -> bool {
    let t = x & 0x7fff_ffff_ffff_ffff;
    t == 0 || ((t >> 52) >= lo && (t >> 52) <= hi)
}
fn in32(x: u32, lo: u32, hi: u32) -> bool {
    let t = x & 0x7fff_ffff;
    t == 0 || ((t >> 23) >= lo && (t >> 23) <= hi)
}

#[test]
fn packed_helpers_match_the_host_cpu() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is missing: skipped");
        return;
    }
    let n: usize = std::env::var("FLE_RC_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(300_000);
    let mut m = Module::default();
    m.import_memory("env", "mem", 1, None);
    let h = add_rc_packed(&mut m);
    // wrapper k: (pa pb pr rc) -> ok : loads a, b from memory, calls the helper, stores the result
    for (k, f) in [h.addps, h.mulps, h.addpd, h.mulpd].into_iter().enumerate() {
        let mut c = Code::new();
        c.local_get(2);
        c.local_get(0);
        c.v128_load(0);
        c.local_get(1);
        c.v128_load(0);
        c.local_get(3);
        c.call(f);
        // stack: result, ok -> ok on top
        c.raw(&[0x21, 4]); // local.set 4 (ok)
        c.v128_store(0);
        c.local_get(4);
        let idx = m.add_func(FuncType { params: vec![Ty::I32; 4], results: vec![Ty::I32] }, Body { locals: vec![(1, Ty::I32)], code: c });
        m.exports.push((format!("w{k}"), 0, idx));
    }
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rc_packed");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("rcp.wasm"), m.encode()).unwrap();
    // case = kind u32 (0 addps 1 mulps 2 addpd 3 mulpd), rc u32, expected ok u32, pad u32, a[16], b[16] (as passed to the helper), expected[16] = 64 bytes
    let mut r = Rng(0x9e3779b97f4a7c15);
    let mut buf: Vec<u8> = Vec::with_capacity(n * 4 * 64);
    for kind in 0..4u32 {
        for _ in 0..n {
            let rc = 1 + r.below(3) as u32;
            let mx = 0x1f80u32 | rc << 13 | (r.below(2) as u32) << 15 | (r.below(2) as u32) << 6;
            let sub = kind != 1 && kind != 3 && r.below(2) == 1;
            let (mut a, mut b) = ([0u8; 16], [0u8; 16]);
            let (lo, hi, w64) = match kind {
                0 | 1 => (65i64, 189i64, false),
                2 => (513, 1533, true),
                _ => (623, 1423, true),
            };
            let center = if w64 { 1023 } else { 127 } + [0, 0, -300, 300, -20, 20][r.below(6) as usize] * if w64 { 1 } else { 0 };
            let center = if w64 { center } else { center + [0, 0, -40, 40, -10, 10][r.below(6) as usize] };
            let nl = if w64 { 2 } else { 4 };
            let mut ok = true;
            for l in 0..nl {
                if w64 {
                    let x = lane64(&mut r, lo, hi, center);
                    let mut y = lane64(&mut r, lo, hi, center);
                    if r.below(4) == 0 {
                        y = x ^ (1 << 63);
                    }
                    a[8 * l..8 * l + 8].copy_from_slice(&x.to_le_bytes());
                    b[8 * l..8 * l + 8].copy_from_slice(&y.to_le_bytes());
                    ok &= in64(x, lo as u64, hi as u64) && in64(y, lo as u64, hi as u64);
                } else {
                    let x = lane32(&mut r, lo, hi, center);
                    let mut y = lane32(&mut r, lo, hi, center);
                    if r.below(4) == 0 {
                        y = x ^ (1 << 31);
                    }
                    a[4 * l..4 * l + 4].copy_from_slice(&x.to_le_bytes());
                    b[4 * l..4 * l + 4].copy_from_slice(&y.to_le_bytes());
                    ok &= in32(x, lo as u32, hi as u32) && in32(y, lo as u32, hi as u32);
                }
            }
            let op = match kind {
                0 | 2 => if sub { 1 } else { 0 },
                _ => 2,
            };
            let exp = host(op, w64, a, b, mx);
            // the helper computes a + b': a subtraction passes the second operand with the sign of every lane flipped
            let mut bp = b;
            if sub {
                for l in 0..nl {
                    if w64 { bp[8 * l + 7] ^= 0x80 } else { bp[4 * l + 3] ^= 0x80 }
                }
            }
            buf.extend_from_slice(&kind.to_le_bytes());
            buf.extend_from_slice(&rc.to_le_bytes());
            buf.extend_from_slice(&(ok as u32).to_le_bytes());
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.extend_from_slice(&a);
            buf.extend_from_slice(&bp);
            buf.extend_from_slice(&exp);
        }
    }
    std::fs::write(dir.join("cases.bin"), &buf).unwrap();
    let js = r#"
const fs = require('fs');
const dir = process.argv[2];
const mem = new WebAssembly.Memory({ initial: 1 });
const inst = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(dir + '/rcp.wasm')), { env: { mem } });
const e = inst.exports;
const b = fs.readFileSync(dir + '/cases.bin');
const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
const u8 = new Uint8Array(mem.buffer);
const names = ['addps', 'mulps', 'addpd', 'mulpd'];
const stat = names.map(() => ({ cases: 0, ok: 0, mism: 0, okbad: 0, byrc: [0, 0, 0, 0] }));
for (let o = 0; o + 64 <= b.length; o += 64) {
  const kind = dv.getUint32(o, true), rc = dv.getUint32(o + 4, true), eok = dv.getUint32(o + 8, true);
  u8.set(b.subarray(o + 16, o + 32), 0);
  u8.set(b.subarray(o + 32, o + 48), 16);
  const ok = e['w' + kind](0, 16, 32, rc);
  const s = stat[kind];
  s.cases++;
  if (ok !== eok) { s.okbad++; if (s.okbad <= 3) console.log('OKBAD', names[kind], rc, Buffer.from(b.subarray(o + 16, o + 32)).toString('hex'), Buffer.from(b.subarray(o + 32, o + 48)).toString('hex'), 'ok', ok, 'expected', eok); continue; }
  if (ok) {
    s.ok++; s.byrc[rc]++;
    let bad = false;
    for (let i = 0; i < 16; i++) if (u8[32 + i] !== b[o + 48 + i]) bad = true;
    if (bad) { s.mism++; if (s.mism <= 5) console.log('MISMATCH', names[kind], 'rc', rc, Buffer.from(b.subarray(o + 16, o + 32)).toString('hex'), Buffer.from(b.subarray(o + 32, o + 48)).toString('hex'), 'got', Buffer.from(u8.subarray(32, 48)).toString('hex'), 'host', Buffer.from(b.subarray(o + 48, o + 64)).toString('hex')); }
  }
}
let bad = 0;
for (let k = 0; k < 4; k++) { const s = stat[k]; console.log('RCPACKED', names[k], 'cases', s.cases, 'ok', s.ok, 'rc1', s.byrc[1], 'rc2', s.byrc[2], 'rc3', s.byrc[3], 'mismatches', s.mism, 'window_disagreements', s.okbad); bad += s.mism + s.okbad; }
process.exit(bad ? 1 : 0);
"#;
    std::fs::File::create(dir.join("run.js")).unwrap().write_all(js.as_bytes()).unwrap();
    let out = Command::new("node").arg(dir.join("run.js")).arg(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{}", text);
    assert!(out.status.success(), "node: {} {}", text, String::from_utf8_lossy(&out.stderr));
}
