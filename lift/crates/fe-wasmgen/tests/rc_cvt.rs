//! Decision 0181: the directed-rounding helper of CVTSD2SS (`rcgen::install` records it as H_CVTSD2SS): run under node on random doubles in and around its window (zero, 2^-100..2^100) with
//! RC = 1, 2, 3 (FTZ, DAZ at random) and compared with the host CPU's cvtsd2ss. A case in the window must give the host's 32 bits and ok = 1, any other case ok = 0. Skipped (and passing)
//! when node is missing or the host is not x86-64. FLE_RC_CASES sets the number of cases (default 3,000,000).
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

fn host(a: u64, mx: u32) -> u32 {
    let x = f64::from_bits(a);
    let mut r = f32::from_bits(0);
    unsafe {
        let v = mx;
        asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack));
        asm!("cvtsd2ss {0}, {1}", inout(xmm_reg) r, in(xmm_reg) x, options(nostack));
        let d = 0x1f80u32;
        asm!("ldmxcsr [{0}]", in(reg) &d, options(nostack));
    }
    r.to_bits()
}

#[test]
fn cvtsd2ss_helper_matches_the_host_cpu() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is missing: skipped");
        return;
    }
    let n: usize = std::env::var("FLE_RC_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(3_000_000);
    let mut m = Module::default();
    rcgen::install(&mut m);
    m.exports.push(("cvtsd2ss".into(), 0, rcgen::helper(rcgen::H_CVTSD2SS)));
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rc_cvt");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("c.wasm"), m.encode()).unwrap();
    let mut r = Rng(0x1234567887654321);
    let mut buf: Vec<u8> = Vec::with_capacity(n * 24);
    for _ in 0..n {
        let rc = 1 + r.below(3) as u32;
        let mx = 0x1f80u32 | rc << 13 | (r.below(2) as u32) << 15 | (r.below(2) as u32) << 6;
        let e = match r.below(12) {
            0 => 923 - 2 + r.below(5),
            1 => 1123 - 2 + r.below(5),
            2 => 1 + r.below(2046),
            3 => 2047,
            4 => 0,
            _ => 923 + r.below(201),
        };
        let m52 = match r.below(6) {
            0 => 0,
            1 => (1 << 52) - 1,
            2 => 1 << 28,
            3 => (1 << 28) - 1,
            4 => (1u64 << 29) + r.below(3) - 1,
            _ => r.next() & ((1 << 52) - 1),
        };
        let a = ((r.next() & 1) << 63) | if r.below(14) == 0 { 0 } else { e << 52 | m52 };
        let t = a & 0x7fff_ffff_ffff_ffff;
        let ok = (t == 0 || ((t >> 52) >= 923 && (t >> 52) <= 1123)) as u32;
        buf.extend_from_slice(&a.to_le_bytes());
        buf.extend_from_slice(&rc.to_le_bytes());
        buf.extend_from_slice(&ok.to_le_bytes());
        buf.extend_from_slice(&host(a, mx).to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
    }
    std::fs::write(dir.join("cases.bin"), &buf).unwrap();
    let js = r#"
const fs = require('fs');
const dir = process.argv[2];
const e = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(dir + '/c.wasm')), {}).exports;
const b = fs.readFileSync(dir + '/cases.bin'); const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
let ok = 0, mism = 0, okbad = 0, n = 0; const byrc = [0, 0, 0, 0];
for (let o = 0; o + 24 <= b.length; o += 24) {
  const a = dv.getBigInt64(o, true), rc = dv.getUint32(o + 8, true), eok = dv.getUint32(o + 12, true), exp = dv.getUint32(o + 16, true);
  const r = e.cvtsd2ss(a, rc); n++;
  if (r[1] !== eok) { okbad++; if (okbad <= 3) console.log('OKBAD', a.toString(16), rc, r[1], eok); continue; }
  if (r[1]) { ok++; byrc[rc]++; if ((r[0] >>> 0) !== exp) { mism++; if (mism <= 5) console.log('MISMATCH', a.toString(16), rc, (r[0] >>> 0).toString(16), exp.toString(16)); } }
}
console.log('RCCVT cvtsd2ss cases', n, 'ok', ok, 'rc1', byrc[1], 'rc2', byrc[2], 'rc3', byrc[3], 'mismatches', mism, 'window_disagreements', okbad);
process.exit(mism + okbad ? 1 : 0);
"#;
    std::fs::write(dir.join("run.js"), js).unwrap();
    let out = Command::new("node").arg(dir.join("run.js")).arg(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{}", text);
    assert!(out.status.success(), "node: {} {}", text, String::from_utf8_lossy(&out.stderr));
}
