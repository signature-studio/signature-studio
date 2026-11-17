//! Decision 0234 (mt3, B-prime stage 1): `--tls-ctx`, the only form of the hybrid (decision 0243 layer e removed the fixed-address form). The register page of the C path (R[] at hybrid_r, X[] at hybrid_x = hybrid_r + 0x200) is a block at `global.get c.__tls_base` + OFFSET. The program is the one of
//! xabi_passthrough.rs (A is lowered, the trampoline T + g stays on the C path and is a JS function that reads X[0] the way the C path does); it is compiled with `tls_ctx` and run under node, where the
//! "C path" keeps R and X in a block at TLSBASE + OFFSET and exports TLSBASE as the global `__tls_base`. It must give the host CPU's answer and the wasm must import the one global c.__tls_base (`WebAssembly.Module.imports`); compiling `hybrid_r` without `tls_ctx` (and the reverse) is an error.
//! Skipped (and passing) when node is missing or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
use std::arch::x86_64::*;
use std::collections::{HashMap, HashSet};
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

/// A PE whose .text holds `funcs` (offset in .text, bytes); every function is in .pdata (the entry points of the decoder).
fn pe(funcs: &[(usize, Vec<u8>)]) -> Vec<u8> {
    let mut b = vec![0u8; 0x1000];
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
    w32(&mut b, opt + 56, 0x4000);
    w32(&mut b, opt + 60, 0x400);
    w16(&mut b, opt + 68, 3);
    w32(&mut b, opt + 108, 16);
    w32(&mut b, opt + 112 + 3 * 8, 0x3000);
    w32(&mut b, opt + 112 + 3 * 8 + 4, 12 * funcs.len() as u32);
    let mut section = |i: usize, name: &[u8], vs: u32, rva: u32, rs: u32, rp: u32, ch: u32| {
        let s = sec + 40 * i;
        b[s..s + name.len()].copy_from_slice(name);
        w32(&mut b, s + 8, vs);
        w32(&mut b, s + 12, rva);
        w32(&mut b, s + 16, rs);
        w32(&mut b, s + 20, rp);
        w32(&mut b, s + 36, ch);
    };
    section(0, b".text", 0x1000, 0x1000, 0x400, 0x400, 0x60000020);
    section(1, b".pdata", 0x200, 0x3000, 0x400, 0x800, 0x40000040);
    for (k, (off, code)) in funcs.iter().enumerate() {
        b[0x400 + off..0x400 + off + code.len()].copy_from_slice(code);
        w32(&mut b, 0x800 + 12 * k, 0x1000 + *off as u32);
        w32(&mut b, 0x800 + 12 * k + 4, 0x1000 + (*off + code.len()) as u32);
        w32(&mut b, 0x800 + 12 * k + 8, 0x3100);
    }
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

fn rel(from_next: usize, to: usize) -> [u8; 4] {
    (to as i32 - from_next as i32).to_le_bytes()
}

/// A at .text+0: movups xmm0,[rcx]; movups xmm1,[rcx+16]; addss xmm0,xmm1; call T; movups xmm0,[rcx]; ret   (xmm0 is written again after the call, so it is not live after it: a register that is live after the call and that the callee may write is flushed anyway, as a possible reload)
fn code_a(t: usize) -> Vec<u8> {
    let mut c = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10, 0xf3, 0x0f, 0x58, 0xc1, 0xe8];
    c.extend_from_slice(&rel(c.len() + 4, t));
    c.extend_from_slice(&[0x0f, 0x10, 0x01, 0xc3]);
    c
}
/// T (call form): sub rsp,0x28; call g; add rsp,0x28; ret
fn code_t_call(at: usize, g: usize) -> Vec<u8> {
    let mut c = vec![0x48, 0x83, 0xec, 0x28, 0xe8];
    c.extend_from_slice(&rel(at + c.len() + 4, g));
    c.extend_from_slice(&[0x48, 0x83, 0xc4, 0x28, 0xc3]);
    c
}
/// T (tail form): jmp g
fn code_t_tail(at: usize, g: usize) -> Vec<u8> {
    let mut c = vec![0xe9];
    c.extend_from_slice(&rel(at + 5, g));
    c
}
/// g: cvtss2sd xmm0,xmm0; ret
fn code_g() -> Vec<u8> {
    vec![0xf3, 0x0f, 0x5a, 0xc0, 0xc3]
}

const JS: &str = r#"
const fs=require('fs');
const [wasmF, inF, outF, tva] = process.argv.slice(2);
const TLSBASE = 0x30000, OFF = 0x40;
const RB = TLSBASE + OFF, XB = RB + 0x200;
const mem=new WebAssembly.Memory({initial:16,maximum:32768,shared:true});
const u8=new Uint8Array(mem.buffer); const dv=new DataView(mem.buffer);
const m=new WebAssembly.Module(fs.readFileSync(wasmF));
const imports={c:{memory:mem}};
const mi = WebAssembly.Module.imports(m).filter((i)=>i.kind==='global');
if (mi.length!==1 || mi[0].name!=='__tls_base') throw new Error('expected the global import c.__tls_base'); imports.c.__tls_base = new WebAssembly.Global({value:'i32', mutable:true}, TLSBASE);
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind!=='function') continue;
  imports[i.module]=imports[i.module]||{};
  imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); };
}
// the C path's T + g: reads X[0] from memory, as the C path does, and leaves (double)(float)xmm0 at 0x20000 (where the compiled code cannot overwrite it by storing its own registers at its return)
imports.c['f_'+tva]=()=>{ dv.setFloat64(0x20000,dv.getFloat32(XB,true),true); };
dv.setUint32(0x3000,0x1fa0,true); // MXCSR in the state the window needs
imports.c.fle_hyb_mx=(kind)=>{ if(kind!==0) throw new Error('kind '+kind); const B=0x3100; u8.copyWithin(B+32,B,B+16); dv.setFloat32(B+32, dv.getFloat32(B,true)+dv.getFloat32(B+16,true), true); }; // the C path's addss (fle_hyb_mx kind 0) for operands outside the window
const inst=new WebAssembly.Instance(m,imports);
const inp=fs.readFileSync(inF); const n=inp.length/32; const out=Buffer.alloc(16*n);
for (let k=0;k<n;k++){
  u8.set(inp.subarray(32*k,32*k+32),0x10000); u8.fill(0,0x20000,0x20010);
  // X[0] holds a marker (stale), X[1] too; the compiled function loads its own xmm0/xmm1 from the data at rcx
  u8.fill(0x5a,XB,XB+0x20);
  dv.setBigUint64(RB+8*4, 0x80000n, true); dv.setBigUint64(RB+8*1, 0x10000n, true); dv.setBigUint64(RB+8*2, 0x20000n, true);
  inst.exports.f_401000();
  out.set(u8.subarray(0x20000,0x20010),16*k);
}
fs.writeFileSync(outF,out);
"#;

fn want(b: &[u8; 32]) -> [u8; 16] {
    unsafe {
        let a: __m128 = std::mem::transmute::<[u8; 16], __m128>(b[0..16].try_into().unwrap());
        let c: __m128 = std::mem::transmute::<[u8; 16], __m128>(b[16..32].try_into().unwrap());
        let s = _mm_add_ss(a, c);
        let mut out = [0u8; 16];
        out[0..8].copy_from_slice(&(_mm_cvtss_f32(s) as f64).to_le_bytes());
        out
    }
}

/// compiles A / T / g in the given form and returns the number of differing inputs out of `n` and a description of the first one
fn run(form: &str, dir: &std::path::Path, ins: &[[u8; 32]]) -> Result<(usize, String), String> {
    let (t_at, g_at) = (0x100usize, 0x200usize);
    let t = if form == "call" { code_t_call(t_at, g_at) } else { code_t_tail(t_at, g_at) };
    let funcs = vec![(0usize, code_a(t_at)), (t_at, t), (g_at, code_g())];
    let mut ci: HashMap<String, String> = HashMap::new();
    ci.insert("dummy".into(), "fle_imp_dummy".into());
    let conf = Config {
        hybrid_r: Some(0x2000),
        hybrid_x: Some(0x2200),
        hybrid_mx: Some((0x3000, 0x3100)),
        c_imports: Some(ci),
        mem_import: Some(("c".into(), "memory".into())),
        teb: Some(0x1000),
        tls_ctx: Some(0x40),
        hybrid_only: Some(HashSet::from([0x401000u64])),
        ..Config::default()
    };
    let comp = compile(&pe(&funcs), &[0x401000, 0x401000 + t_at as u64, 0x401000 + g_at as u64], &[], &conf).map_err(|e| format!("compile: {e}"))?;
    if !comp.fidx.contains_key(&0x401000) {
        return Err("A not lowered".into());
    }
    let w = dir.join(format!("t-{form}.wasm"));
    std::fs::write(&w, &comp.wasm).unwrap();
    if have("wasm-validate") {
        let o = Command::new("wasm-validate").arg("--enable-all").arg(&w).output().unwrap();
        if !o.status.success() {
            return Err(format!("wasm-validate: {}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
        }
    }
    let mut flat: Vec<u8> = Vec::new();
    for b in ins {
        flat.extend_from_slice(b);
    }
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(&js, JS).unwrap();
    let outp = dir.join(format!("out-{form}.bin"));
    let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).arg(format!("{:x}", 0x401000 + t_at)).output().unwrap();
    if !o.status.success() {
        return Err(format!("node: {}", String::from_utf8_lossy(&o.stderr).chars().take(500).collect::<String>()));
    }
    let got = std::fs::read(&outp).unwrap();
    let (mut bad, mut first) = (0, String::new());
    for (k, b) in ins.iter().enumerate() {
        let wv = want(b);
        let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
        // NaN payloads: compare the bits, the host and the wasm both quiet a signalling NaN the same way (cvtss2sd) and addss of two NaNs picks the first operand in both
        if g != wv {
            bad += 1;
            if first.is_empty() {
                first = format!("in {:02x?} want {:02x?} got {:02x?}", b, wv, g);
            }
        }
    }
    Ok((bad, first))
}

#[test]
fn the_tls_form_gives_the_host_cpus_answer() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut ins: Vec<[u8; 32]> = Vec::new();
    for _ in 0..300 {
        let mut b = [0u8; 32];
        for l in 0..4 {
            let x = rng(&mut s);
            let w = if l % 2 == 0 {
                let f = ((rng(&mut s) % 2_000_001) as f32 - 1_000_000.0) / 64.0;
                (f.to_bits() as u64) | (rng(&mut s) & 0xffff_ffff_0000_0000)
            } else {
                x
            };
            b[8 * l..8 * l + 8].copy_from_slice(&w.to_le_bytes());
        }
        ins.push(b);
    }
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-tls-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut failures: Vec<String> = Vec::new();
    for form in ["call", "tail"] {
        match run(form, &dir, &ins) {
            Ok((0, _)) => {}
            Ok((bad, first)) => failures.push(format!("{form}: {bad} of {} inputs differ from the host CPU; first: {first}", ins.len())),
            Err(e) => failures.push(format!("{form}: {e}")),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn hybrid_r_and_tls_ctx_need_each_other() {
    let pe = pe(&[(0, vec![0xc3])]);
    let base = || Config { hybrid_x: Some(0x2200), mem_import: Some(("c".into(), "memory".into())), ..Config::default() };
    let e = compile(&pe, &[0x401000], &[], &Config { hybrid_r: Some(0x2000), ..base() }).err().expect("hybrid_r without tls_ctx must fail");
    assert!(e.contains("--hybrid-r needs --tls-ctx"), "{e}");
    let e = compile(&pe, &[0x401000], &[], &Config { tls_ctx: Some(0x40), ..base() }).err().expect("tls_ctx without hybrid_r must fail");
    assert!(e.contains("--tls-ctx needs --hybrid-r"), "{e}");
}
