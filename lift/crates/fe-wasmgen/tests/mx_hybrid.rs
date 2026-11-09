//! Decision 0169: the window path and the C-path call of the hybrid mode for CVTSI2SS (32/64), CVTSD2SI and CVTSS2SI (32/64). The module is compiled in hybrid mode with the MXCSR window
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

/// the status flags (IE bit 0, DE bit 1, PE bit 5) that the host CPU raises for case `k` on input `b` from a clean MXCSR (round to nearest, all exceptions masked)
fn host_flags(k: usize, b: &[u8; 32]) -> u32 {
    use std::arch::asm;
    let mut csr: u32 = 0x1f80;
    let (x1, x2): (__m128, __m128d) = unsafe { (std::mem::transmute::<[u8; 16], __m128>(b[16..32].try_into().unwrap()), std::mem::transmute::<[u8; 16], __m128d>(b[16..32].try_into().unwrap())) };
    let (i32v, i64v) = (i32::from_le_bytes(b[16..20].try_into().unwrap()), i64::from_le_bytes(b[16..24].try_into().unwrap()));
    unsafe {
        asm!("ldmxcsr [{c}]", c = in(reg) &csr, options(nostack));
        let mut d: __m128 = std::mem::transmute::<[u8; 16], __m128>(b[0..16].try_into().unwrap());
        let r32: i32;
        let r64: i64;
        match k {
            0 => asm!("cvtsi2ss {d}, {i:e}", d = inout(xmm_reg) d, i = in(reg) i32v, options(nostack)),
            1 => asm!("cvtsi2ss {d}, {i}", d = inout(xmm_reg) d, i = in(reg) i64v, options(nostack)),
            2 => { asm!("cvtsd2si {r:e}, {x}", r = out(reg) r32, x = in(xmm_reg) x2, options(nostack)); let _ = r32; }
            3 => { asm!("cvtsd2si {r}, {x}", r = out(reg) r64, x = in(xmm_reg) x2, options(nostack)); let _ = r64; }
            4 => { asm!("cvtss2si {r:e}, {x}", r = out(reg) r32, x = in(xmm_reg) x1, options(nostack)); let _ = r32; }
            _ => { asm!("cvtss2si {r}, {x}", r = out(reg) r64, x = in(xmm_reg) x1, options(nostack)); let _ = r64; }
        }
        let _ = d;
        asm!("stmxcsr [{c}]", c = in(reg) &mut csr, options(nostack));
    }
    csr & 0x23
}

struct Case {
    name: &'static str,
    /// the instruction (after `mov rax,[rcx+16]` for the integer sources, after the loads of xmm0/xmm1 for the others) and how the result leaves the function
    insn: Vec<u8>,
    gp_in: bool,
    xmm_out: bool,
    want: fn(&[u8; 32]) -> [u8; 16],
}

fn cases() -> Vec<Case> {
    unsafe {
        let xmm = |b: &[u8; 32], o: usize| -> __m128i { std::mem::transmute::<[u8; 16], __m128i>(b[o..o + 16].try_into().unwrap()) };
        let _ = xmm;
    }
    fn x(b: &[u8; 32], o: usize) -> __m128 {
        unsafe { std::mem::transmute::<[u8; 16], __m128>(b[o..o + 16].try_into().unwrap()) }
    }
    fn xd(b: &[u8; 32], o: usize) -> __m128d {
        unsafe { std::mem::transmute::<[u8; 16], __m128d>(b[o..o + 16].try_into().unwrap()) }
    }
    fn out_x(v: __m128) -> [u8; 16] {
        unsafe { std::mem::transmute::<__m128, [u8; 16]>(v) }
    }
    fn out_i(v: u64) -> [u8; 16] {
        let mut o = [0u8; 16];
        o[..8].copy_from_slice(&v.to_le_bytes());
        o
    }
    vec![
        Case { name: "cvtsi2ss r32", insn: vec![0xf3, 0x0f, 0x2a, 0xc0], gp_in: true, xmm_out: true, want: |b| unsafe { out_x(_mm_cvtsi32_ss(x(b, 0), i32::from_le_bytes(b[16..20].try_into().unwrap()))) } },
        Case { name: "cvtsi2ss r64", insn: vec![0xf3, 0x48, 0x0f, 0x2a, 0xc0], gp_in: true, xmm_out: true, want: |b| unsafe { out_x(_mm_cvtsi64_ss(x(b, 0), i64::from_le_bytes(b[16..24].try_into().unwrap()))) } },
        Case { name: "cvtsd2si r32", insn: vec![0xf2, 0x0f, 0x2d, 0xc1], gp_in: false, xmm_out: false, want: |b| unsafe { out_i(_mm_cvtsd_si32(xd(b, 16)) as u32 as u64) } },
        Case { name: "cvtsd2si r64", insn: vec![0xf2, 0x48, 0x0f, 0x2d, 0xc1], gp_in: false, xmm_out: false, want: |b| unsafe { out_i(_mm_cvtsd_si64(xd(b, 16)) as u64) } },
        Case { name: "cvtss2si r32", insn: vec![0xf3, 0x0f, 0x2d, 0xc1], gp_in: false, xmm_out: false, want: |b| unsafe { out_i(_mm_cvtss_si32(x(b, 16)) as u32 as u64) } },
        Case { name: "cvtss2si r64", insn: vec![0xf3, 0x48, 0x0f, 0x2d, 0xc1], gp_in: false, xmm_out: false, want: |b| unsafe { out_i(_mm_cvtss_si64(x(b, 16)) as u64) } },
    ]
}

fn inputs(n: usize) -> Vec<[u8; 32]> {
    let f64s: [u64; 30] = [
        0, 1 << 63, 0x3ff0000000000000, 0xbff0000000000000, 0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000000, 0xfff8000000000001, 0x7ff0000000000001, 1, 0x000fffffffffffff, 0x8000000000000001,
        0x41dfffffffc00000, 0x41e0000000000000, 0xc1e0000000000000, 0xc1e0000000200000, 0x4330000000000000, 0x43e0000000000000, 0xc3e0000000000000, 0x3fe0000000000000, 0x7fefffffffffffff, 0x0010000000000000,
        0x4004000000000000, 0x400c000000000000, 0xbfe0000000000000, 0x3ff8000000000000, 0x41cfffffffc00000, 0x4320000000000000, 0xc320000000000000, 0x432fffffffffffff,
    ];
    let ints: [u64; 22] = [
        0, 1, u64::MAX, 0x7fffffff, 0x80000000, 0xffffffff80000000, 0x1000000, 0x1000001, 0xffffffffff000000, 0xfffffffffeffffff, 0x1fffffe, 0x1ffffff, 0x2000001, 0x7fffffffffffffff, 0x8000000000000000,
        0x20000000000001, 0x1fffffffffffff, 0xffe0000000000001, 0x4000008000000000, 0x4000004000000001, 0x8000008000000001, 12345678,
    ];
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut v = Vec::new();
    for _ in 0..n {
        let mut b = [0u8; 32];
        for l in 0..4 {
            let x = rng(&mut s);
            let w = match x % 6 {
                0 => f64s[(rng(&mut s) % 30) as usize],
                1 => {
                    let a = (f64s[(rng(&mut s) % 30) as usize] >> 32) as u32;
                    let c = (f64s[(rng(&mut s) % 30) as usize] >> 32) as u32;
                    (a as u64) << 32 | c as u64
                }
                2 | 3 => ints[(rng(&mut s) % 22) as usize],
                4 => ((rng(&mut s) as i64) >> (rng(&mut s) % 64)) as u64,
                _ => rng(&mut s),
            };
            b[8 * l..8 * l + 8].copy_from_slice(&w.to_le_bytes());
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
const f32=new Float32Array(1), u32=new Uint32Array(f32.buffer), f64=new Float64Array(1), u64=new BigUint64Array(f64.buffer);
function rneBig(x){ // x a finite double -> BigInt, round half to even
  const fl=Math.floor(x); const d=x-fl; let r=BigInt(fl);
  if (d>0.5 || (d===0.5 && (r & 1n)===1n)) r+=1n; return r; }
function toI(x,bits){ // cvt*2si: the integer indefinite for NaN / out of range
  const ind = bits===32 ? 0x80000000n : 0x8000000000000000n;
  if (!Number.isFinite(x)) return ind;
  const r=rneBig(x); const lim=1n<<BigInt(bits-1);
  if (r>=lim || r< -lim) return ind;
  return BigInt.asUintN(bits,r); }
function i2f(v){ // v BigInt (signed) -> binary32 bits, round to nearest even
  if (v===0n) return 0; const neg=v<0n; let a=neg?-v:v; const len=a.toString(2).length;
  let mant=a, sh=0; if (len>24){ sh=len-24; mant=a>>BigInt(sh); const rem=a&((1n<<BigInt(sh))-1n), half=1n<<BigInt(sh-1); if (rem>half || (rem===half && (mant&1n)===1n)) mant+=1n; }
  f32[0]=Number(mant)*Math.pow(2,sh); let b=u32[0]; if (neg) b=(b|0x80000000)>>>0; return b; }
const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind!=='function') continue;
  imports[i.module]=imports[i.module]||{};
  imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); };
}
let slow=0;
imports.c.fle_hyb_mx=(kind,imm)=>{
  slow++;
  const d=new Uint8Array(mem.buffer,BUF,16), s=new Uint8Array(mem.buffer,BUF+16,16), r=new Uint8Array(mem.buffer,BUF+32,16);
  r.set(d);
  const sdv=new DataView(mem.buffer,BUF+16,16);
  const rdv=new DataView(mem.buffer,BUF+32,16);
  switch(kind){
    case 33: rdv.setUint32(0, i2f(BigInt(sdv.getInt32(0,true))), true); break;
    case 34: rdv.setUint32(0, i2f(sdv.getBigInt64(0,true)), true); break;
    case 35: rdv.setBigUint64(0, toI(sdv.getFloat64(0,true),32), true); break;
    case 36: rdv.setBigUint64(0, toI(sdv.getFloat64(0,true),64), true); break;
    case 37: rdv.setBigUint64(0, toI(sdv.getFloat32(0,true),32), true); break;
    case 38: rdv.setBigUint64(0, toI(sdv.getFloat32(0,true),64), true); break;
    default: throw new Error('kind '+kind);
  }
};
const slowv=[]; const inst=new WebAssembly.Instance(m,imports);
const inp=fs.readFileSync(inF); const n=inp.length/32; const out=Buffer.alloc(16*n);
dv.setUint32(MXA, parseInt(mxcsr,16), true);
for (let k=0;k<n;k++){
  u8.set(inp.subarray(32*k,32*k+32),0x10000); u8.fill(0,0x20000,0x20010);
  dv.setBigUint64(0x2000+8*4, 0x80000n, true); dv.setBigUint64(0x2000+8*1, 0x10000n, true); dv.setBigUint64(0x2000+8*2, 0x20000n, true);
  const s0=slow; inst.exports.f_401000(); slowv.push(slow>s0?1:0);
  out.set(u8.subarray(0x20000,0x20010),16*k);
}
fs.writeFileSync(outF,out); fs.writeFileSync(outF+'.slow',Buffer.from(slowv)); console.log('slow '+slow);
"#;

#[test]
fn hybrid_window_and_c_call_match_the_host_cpu() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let ins = inputs(1500);
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-mxh-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut flat: Vec<u8> = Vec::new();
    for b in &ins {
        flat.extend_from_slice(b);
    }
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(&js, JS).unwrap();
    let mut failures: Vec<String> = Vec::new();
    let mut cmps = 0;
    for (idx, c) in cases().into_iter().enumerate() {
        // mov rax,[rcx+16]  (gp source), else movups xmm0,[rcx]; movups xmm1,[rcx+16]
        let mut code: Vec<u8> = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10];
        if c.gp_in {
            code.extend_from_slice(&[0x48, 0x8b, 0x41, 0x10]);
        }
        code.extend_from_slice(&c.insn);
        if c.xmm_out {
            code.extend_from_slice(&[0x0f, 0x11, 0x02]);
        } else {
            code.extend_from_slice(&[0x48, 0x89, 0x02]);
        }
        code.push(0xc3);
        let mut ci: HashMap<String, String> = HashMap::new();
        ci.insert("dummy".into(), "fle_imp_dummy".into());
        let conf = Config {
            hybrid_r: Some(0x2000),
            tls_ctx: Some(0x2000),
            hybrid_x: Some(0x2200),
            hybrid_mx: Some((0x3000, 0x3100)),
            c_imports: Some(ci),
            mem_import: Some(("c".into(), "memory".into())),
            teb: Some(0x1000),
            ..Config::default()
        };
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
        for (label, mx) in [("window (PE set)", "1fa0"), ("no window (PE clear)", "1f80")] {
            let outp = dir.join("out.bin");
            let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).arg(mx).output().unwrap();
            if !o.status.success() {
                failures.push(format!("{} / {label}: node: {}", c.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
                continue;
            }
            let slow = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let got = std::fs::read(&outp).unwrap();
            let (mut bad, mut first) = (0, String::new());
            for (k, b) in ins.iter().enumerate() {
                let want = (c.want)(b);
                let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
                let eq = if c.xmm_out { (0..4).all(|l| { let (p, q) = (&g[4 * l..4 * l + 4], &want[4 * l..4 * l + 4]); p == q || (f32::from_le_bytes(p.try_into().unwrap()).is_nan() && f32::from_le_bytes(q.try_into().unwrap()).is_nan()) }) } else { g[..8] == want[..8] };
                if !eq {
                    bad += 1;
                    if first.is_empty() {
                        first = format!("in {:02x?} want {:02x?} got {:02x?}", b, want, g);
                    }
                }
            }
            // the flag invariants: an input that the fast path took must not raise IE or DE on the host (and cvtsi2ss no PE either: its window is the exact range); without the window the
            // guarded forms (cvt*2si) take no input on the fast path
            let sl = std::fs::read(dir.join("out.bin.slow")).unwrap();
            let (mut fbad, mut ffirst) = (0, String::new());
            for (k, b) in ins.iter().enumerate() {
                let fl = host_flags(idx, b);
                let fast = sl[k] == 0;
                let guarded = idx >= 2;
                let viol = (fast && (fl & 0x3) != 0) || (fast && !guarded && (fl & 0x20) != 0) || (fast && guarded && mx == "1f80");
                if viol {
                    fbad += 1;
                    if ffirst.is_empty() {
                        ffirst = format!("in {:02x?}: fast path with host flags {:#x}", b, fl);
                    }
                }
            }
            if fbad > 0 {
                failures.push(format!("{} / {label}: {fbad} inputs took the fast path although the host raises IE/DE/PE or the guard is off; first: {ffirst}", c.name));
            }
            cmps += ins.len();
            eprintln!("mx_hybrid: {:14} {:22} {} C-path calls of {} inputs, {} differ", c.name, label, slow, ins.len(), bad);
            if bad > 0 {
                failures.push(format!("{} / {label}: {bad} of {} inputs differ; first: {first}", c.name, ins.len()));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("mx_hybrid: {cmps} comparisons with the host CPU");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
