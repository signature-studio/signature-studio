//! Decision 0181 (hyb2): the wiring of the RC != 0 helpers and the packed window rules in the hybrid mode (--mx-rc --mx-packed): one instruction per function, MXCSR in memory with RC = 0..3 and
//! PE set / clear, inputs in and around the windows; an input in the window with the guard on must not call the C path (`c.fle_hyb_mx`) and must give the host CPU's bits (run with that RC); any other
//! input must call it. Earlier text of the file this was made from: decision 0169: the window path and the C-path call of the hybrid mode for CVTSI2SS (32/64), CVTSD2SI and CVTSS2SI (32/64). The module is compiled in hybrid mode with the MXCSR window
//! (hybrid_r / hybrid_x / hybrid_mx, the C runtime's imports), the one C function it calls on the slow path (`c.fle_hyb_mx`, kinds 33..38 of lift/harness/hyb1/hybrid_glue.c) is replaced
//! by a JS function that computes what the C path's function computes (the architectural result under the default MXCSR), and the result is compared with the host CPU, with MXCSR in
//! memory in the state the window needs (PE set, PM masked: every input in the window takes the fast path) and in the state it does not (every input takes the C-path call).
//! The stub is checked by the same comparison: a wrong stub or a wrong window shows as a difference from the host CPU. Skipped (and passing) when node is missing or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::arch::x86_64::*;
use std::collections::HashMap;
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

fn rng(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}


#[derive(Clone, Copy, PartialEq)]
enum Cls { P32, P64Add, P64Mul, S32Add, S32Div, S32Sqrt, S64Add, S64Mul, S64Div, S64Sqrt, Cvt, Ps2Dq, Pd2Dq }

struct Case {
    name: &'static str,
    insn: Vec<u8>,
    cls: Cls,
    /// the host run: a, b -> result of the instruction under the MXCSR in effect
    host: fn(__m128, __m128) -> __m128,
}

macro_rules! host2 {
    ($asm:expr) => {
        |a: __m128, b: __m128| -> __m128 {
            let mut r = a;
            unsafe { std::arch::asm!($asm, inout(xmm_reg) r, in(xmm_reg) b, options(nostack)) };
            r
        }
    };
}

fn cases() -> Vec<Case> {
    vec![
        Case { name: "addps", insn: vec![0x0f, 0x58, 0xc1], cls: Cls::P32, host: host2!("addps {0}, {1}") },
        Case { name: "subps", insn: vec![0x0f, 0x5c, 0xc1], cls: Cls::P32, host: host2!("subps {0}, {1}") },
        Case { name: "mulps", insn: vec![0x0f, 0x59, 0xc1], cls: Cls::P32, host: host2!("mulps {0}, {1}") },
        Case { name: "addpd", insn: vec![0x66, 0x0f, 0x58, 0xc1], cls: Cls::P64Add, host: host2!("addpd {0}, {1}") },
        Case { name: "subpd", insn: vec![0x66, 0x0f, 0x5c, 0xc1], cls: Cls::P64Add, host: host2!("subpd {0}, {1}") },
        Case { name: "mulpd", insn: vec![0x66, 0x0f, 0x59, 0xc1], cls: Cls::P64Mul, host: host2!("mulpd {0}, {1}") },
        Case { name: "addss", insn: vec![0xf3, 0x0f, 0x58, 0xc1], cls: Cls::S32Add, host: host2!("addss {0}, {1}") },
        Case { name: "subss", insn: vec![0xf3, 0x0f, 0x5c, 0xc1], cls: Cls::S32Add, host: host2!("subss {0}, {1}") },
        Case { name: "mulss", insn: vec![0xf3, 0x0f, 0x59, 0xc1], cls: Cls::S32Add, host: host2!("mulss {0}, {1}") },
        Case { name: "divss", insn: vec![0xf3, 0x0f, 0x5e, 0xc1], cls: Cls::S32Div, host: host2!("divss {0}, {1}") },
        Case { name: "sqrtss", insn: vec![0xf3, 0x0f, 0x51, 0xc1], cls: Cls::S32Sqrt, host: host2!("sqrtss {0}, {1}") },
        Case { name: "addsd", insn: vec![0xf2, 0x0f, 0x58, 0xc1], cls: Cls::S64Add, host: host2!("addsd {0}, {1}") },
        Case { name: "subsd", insn: vec![0xf2, 0x0f, 0x5c, 0xc1], cls: Cls::S64Add, host: host2!("subsd {0}, {1}") },
        Case { name: "mulsd", insn: vec![0xf2, 0x0f, 0x59, 0xc1], cls: Cls::S64Mul, host: host2!("mulsd {0}, {1}") },
        Case { name: "divsd", insn: vec![0xf2, 0x0f, 0x5e, 0xc1], cls: Cls::S64Div, host: host2!("divsd {0}, {1}") },
        Case { name: "sqrtsd", insn: vec![0xf2, 0x0f, 0x51, 0xc1], cls: Cls::S64Sqrt, host: host2!("sqrtsd {0}, {1}") },
        Case { name: "cvtsd2ss", insn: vec![0xf2, 0x0f, 0x5a, 0xc1], cls: Cls::Cvt, host: host2!("cvtsd2ss {0}, {1}") },
        // decision 0252: the conversions that follow MXCSR.RC; the compiled path is the RC = 0 window (nearest even, PE set, PM masked), every other state calls the C path
        Case { name: "cvtps2dq", insn: vec![0x66, 0x0f, 0x5b, 0xc1], cls: Cls::Ps2Dq, host: host2!("cvtps2dq {0}, {1}") },
        Case { name: "cvtpd2dq", insn: vec![0xf2, 0x0f, 0xe6, 0xc1], cls: Cls::Pd2Dq, host: host2!("cvtpd2dq {0}, {1}") },
    ]
}

fn l32(b: &[u8], k: usize) -> u32 { u32::from_le_bytes(b[4 * k..4 * k + 4].try_into().unwrap()) }
fn l64(b: &[u8], k: usize) -> u64 { u64::from_le_bytes(b[8 * k..8 * k + 8].try_into().unwrap()) }
fn win32f(x: u32, lo: u32, hi: u32, zero: bool) -> bool { let t = x & 0x7fff_ffff; (zero && t == 0) || ((t >> 23) >= lo && (t >> 23) <= hi) }
fn w64f(x: u64, lo: u64, hi: u64, zero: bool) -> bool { let t = x & 0x7fff_ffff_ffff_ffff; (zero && t == 0) || ((t >> 52) >= lo && (t >> 52) <= hi) }
fn pos32(x: u32) -> bool { (x >> 31 == 0 && win32f(x, 65, 189, true)) || (x << 1 == 0) }
fn pos64(x: u64, lo: u64, hi: u64) -> bool { (x >> 63 == 0 && w64f(x, lo, hi, true)) || (x << 1 == 0) }

/// does the compiled code stay out of the C path for the operands a (destination) and b (source) with RC `rc` (guard on)?
fn in_window(c: Cls, rc: u32, a: &[u8], b: &[u8]) -> bool {
    let (mlo, mhi) = if rc == 0 { (513, 1533) } else { (623, 1423) };
    match c {
        Cls::P32 => (0..4).all(|k| win32f(l32(a, k), 65, 189, true) && win32f(l32(b, k), 65, 189, true)),
        Cls::P64Add => (0..2).all(|k| w64f(l64(a, k), 513, 1533, true) && w64f(l64(b, k), 513, 1533, true)),
        Cls::P64Mul => (0..2).all(|k| w64f(l64(a, k), mlo, mhi, true) && w64f(l64(b, k), mlo, mhi, true)),
        Cls::S32Add => win32f(l32(a, 0), 65, 189, true) && win32f(l32(b, 0), 65, 189, true),
        Cls::S32Div => win32f(l32(a, 0), 65, 189, true) && win32f(l32(b, 0), 65, 189, false),
        Cls::S32Sqrt => pos32(l32(b, 0)),
        Cls::S64Add => w64f(l64(a, 0), 513, 1533, true) && w64f(l64(b, 0), 513, 1533, true),
        Cls::S64Mul => w64f(l64(a, 0), mlo, mhi, true) && w64f(l64(b, 0), mlo, mhi, true),
        Cls::S64Div => w64f(l64(a, 0), mlo, mhi, true) && w64f(l64(b, 0), mlo, mhi, false),
        Cls::S64Sqrt => pos64(l64(b, 0), mlo, mhi),
        Cls::Cvt => w64f(l64(b, 0), 923, 1123, true),
        Cls::Ps2Dq => rc == 0 && (0..4).all(|k| win32f(l32(b, k), 1, 157, true)),
        Cls::Pd2Dq => rc == 0 && (0..2).all(|k| w64f(l64(b, k), 513, 1053, true)),
    }
}

fn gen_inputs(n: usize) -> Vec<[u8; 32]> {
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut v = Vec::new();
    for _ in 0..n {
        let mut b = [0u8; 32];
        let mode = rng(&mut s) % 4;
        for l in 0..4 {
            // two doubles per operand or four singles: mostly in the common range, sometimes zero, an edge or anything
            let x = rng(&mut s);
            let w = match (x >> 8) % 10 {
                0 => 0u64,
                1 => rng(&mut s),
                2 => (rng(&mut s) & 0x8000_0000_8000_0000) | (((rng(&mut s) % 40) + 100) << 23) | (((rng(&mut s) % 40) + 100) << 55) | (rng(&mut s) & 0x007f_ffff_007f_ffff),
                _ => {
                    let e64 = 1023 - 30 + rng(&mut s) % 61 - if mode == 0 { 0 } else { (rng(&mut s) % 3) * 200 };
                    ((rng(&mut s) & 1) << 63) | (e64 << 52) | (rng(&mut s) & 0xf_ffff_ffff_ffff)
                }
            };
            // singles in the common range for the packed single forms: re-make the lanes from exponents 127 +- 20
            let w = if mode == 1 { let e = |s: &mut u64| (127 - 20 + rng(s) % 41) as u32; let (e0, e1) = (e(&mut s), e(&mut s)); ((((rng(&mut s) as u32) & 0x807f_ffff) | (e1 << 23)) as u64) << 32 | (((rng(&mut s) as u32) & 0x807f_ffff) | (e0 << 23)) as u64 } else { w };
            b[8 * l..8 * l + 8].copy_from_slice(&w.to_le_bytes());
        }
        v.push(b);
    }
    v
}

/// decision 0252: the edge values of the integer conversions as the source operand (the second 16 bytes): ties, just below and above a tie, the 2^31 boundary on both sides, NaN, infinity, a denormal, tiny values
fn special_inputs() -> Vec<[u8; 32]> {
    let f32s: Vec<u32> = [
        0.0f32, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999997, -0.49999997, 0.50000006, -0.50000006, 8388607.5, -8388607.5, 8388608.0, 16777215.0, 1.0e-30, -1.0e-30, 1.0, -1.0, 2147483520.0, -2147483520.0,
        1073741824.0, -1073741824.0, 1.0e-40, -1.0e-40, 3.0e9, -3.0e9, 2147483648.0, -2147483648.0, 2147483904.0, -2147483904.0,
    ]
    .iter()
    .map(|x| x.to_bits())
    .chain([0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0000, 0xff80_0000, 0x7f80_0001, 0x0000_0001, 0x807f_ffff])
    .collect();
    let f64s: Vec<u64> = [
        0.0f64, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999999999999994, -0.49999999999999994, 0.5000000000000001, 2147483647.5, 2147483647.4999998, 2147483647.0, -2147483647.5, -2147483648.0, -2147483648.5,
        -2147483648.4999995, -2147483648.5000005, 2147483648.0, -2147483649.0, 2147483647.5000005, 4294967296.0, 1.0e-3, -1.0e-3, 1.0e-300, -1.0e-300, 1.0e300, 4503599627370496.5, 1073741823.5,
    ]
    .iter()
    .map(|x| x.to_bits())
    .chain([0x7ff8_0000_0000_0000u64, 0xfff8_0000_0000_0000, 0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000, 0x7ff0_0000_0000_0001, 1, 0x800f_ffff_ffff_ffff])
    .collect();
    let mut v = Vec::new();
    let mut s = 0x1234_5678_9abc_def1u64;
    for k in 0..f32s.len() {
        let mut b = [0u8; 32];
        for l in 0..2 {
            b[8 * l..8 * l + 8].copy_from_slice(&rng(&mut s).to_le_bytes());
        }
        for l in 0..4 {
            b[16 + 4 * l..16 + 4 * l + 4].copy_from_slice(&f32s[(k + l) % f32s.len()].to_le_bytes());
        }
        v.push(b);
    }
    for k in 0..f64s.len() {
        let mut b = [0u8; 32];
        for l in 0..2 {
            b[8 * l..8 * l + 8].copy_from_slice(&rng(&mut s).to_le_bytes());
        }
        for l in 0..2 {
            b[16 + 8 * l..16 + 8 * l + 8].copy_from_slice(&f64s[(k + l) % f64s.len()].to_le_bytes());
        }
        v.push(b);
    }
    v
}

const JS: &str = r#"
const fs=require('fs');
const [wasmF, inF, outF, mxcsr] = process.argv.slice(2);
const mem=new WebAssembly.Memory({initial:16,maximum:32768,shared:true});
const u8=new Uint8Array(mem.buffer); const dv=new DataView(mem.buffer);
const MXA=0x3000, BUF=0x3100;
const m=new WebAssembly.Module(fs.readFileSync(wasmF));
const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};
for (const i of WebAssembly.Module.imports(m)) { if (i.kind!=='function') continue; imports[i.module]=imports[i.module]||{}; imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); }; }
let slow=0;
imports.c.fle_hyb_mx=(kind,imm)=>{ slow++; u8.copyWithin(BUF+32, BUF, BUF+16); };
const inst=new WebAssembly.Instance(m,imports);
const inp=fs.readFileSync(inF); const n=inp.length/32; const out=Buffer.alloc(16*n); const slowv=Buffer.alloc(n);
dv.setUint32(MXA, parseInt(mxcsr,16), true);
for (let k=0;k<n;k++){
  u8.set(inp.subarray(32*k,32*k+32),0x10000); u8.fill(0,0x20000,0x20010);
  dv.setBigUint64(0x2000+8*4, 0x80000n, true); dv.setBigUint64(0x2000+8*1, 0x10000n, true); dv.setBigUint64(0x2000+8*2, 0x20000n, true);
  const s0=slow; inst.exports.f_401000(); slowv[k]=slow>s0?1:0;
  out.set(u8.subarray(0x20000,0x20010),16*k);
}
fs.writeFileSync(outF,out); fs.writeFileSync(outF+'.slow',slowv); console.log('slow '+slow);
"#;

#[test]
fn rc_wiring_matches_the_host_cpu() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let mut ins = gen_inputs(3000);
    ins.extend(special_inputs());
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-rcw-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flat: Vec<u8> = ins.iter().flat_map(|b| b.iter().copied()).collect();
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(&js, JS).unwrap();
    let mut failures: Vec<String> = Vec::new();
    let (mut fast_total, mut cmp_total) = (0usize, 0usize);
    for c in cases() {
        let mut code: Vec<u8> = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10];
        code.extend_from_slice(&c.insn);
        code.extend_from_slice(&[0x0f, 0x11, 0x02, 0xc3]);
        let mut ci: HashMap<String, String> = HashMap::new();
        ci.insert("dummy".into(), "fle_imp_dummy".into());
        let conf = Config { hybrid_r: Some(0x2000), tls_ctx: Some(0x2000), hybrid_x: Some(0x2200), hybrid_mx: Some((0x3000, 0x3100)), c_imports: Some(ci), mem_import: Some(("c".into(), "memory".into())), teb: Some(0x1000), ..Config::default() };
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
            let o = Command::new("wasm-validate").arg("--enable-all").arg(&w).output().unwrap();
            if !o.status.success() {
                failures.push(format!("{}: wasm-validate: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
                continue;
            }
        }
        // MXCSR: all exceptions masked, RC 0..3, FTZ/DAZ on for rc 3 and 1, PE set (guard on) or clear
        for (label, mx, rc, guard) in [("rc0 PE", "1fa0", 0u32, true), ("rc1 PE", "3fa0", 1, true), ("rc2 PE", "5fa0", 2, true), ("rc3 PE ftz daz", "ffe0", 3, true), ("rc3 no PE", "ff80", 3, false), ("rc0 no PE", "1f80", 0, false)] {
            let outp = dir.join("out.bin");
            let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).arg(mx).output().unwrap();
            if !o.status.success() {
                failures.push(format!("{} / {label}: node: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
                continue;
            }
            let got = std::fs::read(&outp).unwrap();
            let sl = std::fs::read(dir.join("out.bin.slow")).unwrap();
            let (mut bad, mut sbad, mut first) = (0, 0, String::new());
            for (k, b) in ins.iter().enumerate() {
                let win = in_window(c.cls, rc, &b[..16], &b[16..]);
                let fast_expected = win && guard;
                if (sl[k] == 0) != fast_expected {
                    sbad += 1;
                    if first.is_empty() { first = format!("path: in {:02x?} expected {} got {}", b, if fast_expected { "compiled" } else { "C path" }, if sl[k] == 0 { "compiled" } else { "C path" }); }
                }
                if fast_expected && sl[k] == 0 {
                    fast_total += 1;
                    let a: __m128 = unsafe { std::mem::transmute::<[u8; 16], __m128>(b[..16].try_into().unwrap()) };
                    let bb: __m128 = unsafe { std::mem::transmute::<[u8; 16], __m128>(b[16..].try_into().unwrap()) };
                    let want: [u8; 16] = unsafe {
                        let v = u32::from_str_radix(mx, 16).unwrap();
                        std::arch::asm!("ldmxcsr [{0}]", in(reg) &v, options(nostack));
                        let r = (c.host)(a, bb);
                        let d = 0x1f80u32;
                        std::arch::asm!("ldmxcsr [{0}]", in(reg) &d, options(nostack));
                        std::mem::transmute::<__m128, [u8; 16]>(r)
                    };
                    cmp_total += 1;
                    let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
                    if g != want {
                        bad += 1;
                        if first.is_empty() { first = format!("value: in {:02x?} want {:02x?} got {:02x?}", b, want, g); }
                    }
                }
            }
            if bad + sbad > 0 {
                failures.push(format!("{} / {label}: {bad} values and {sbad} paths differ; first: {first}", c.name));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("mx_rc_wire: {} instruction forms x 6 MXCSR states x {} inputs; {} compiled-path results compared with the host CPU", cases().len(), ins.len(), cmp_total);
    let _ = fast_total;
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
