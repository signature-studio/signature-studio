//! Decision 0252 (what decision 0181 had as the opt-in `--opaque-stub`, now the behaviour): a function whose flow analysis fails only because a flag has no recipe (`mul ecx` followed by `jae`: CF of a multiplication)
//! used to be opaque, and every caller of it stayed on the C path. It is an ordinary function of the C path for its callers: A (lowered) calls T as a stub with the conservative signature (every register
//! passed and returned). T stays on the C path: here a JS function that does what T does (the low half of edx * ecx, plus 1 when the high half is not zero). The result is compared with the host's arithmetic; A must be lowered
//! (the report says "ok") and must call T. Skipped (and passing) when node is missing or the host is not x86-64.
//! (Decision 0270 gave the shift by cl a recipe, so the example of this test is a multiplication; T with `shr eax, cl; jae` alone reads a flag that is live into it when cl == 0, and stays opaque.)
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

/// A: mov edx,[rcx]; mov ecx,[rcx+4]; call T; mov [rdi],eax; ret      (the value and the count come from the data at rcx, the result goes to the buffer at rdi: a register that the callee preserves)
fn code_a(t: usize) -> Vec<u8> {
    let mut c = vec![0x8b, 0x11, 0x8b, 0x49, 0x04, 0xe8];
    c.extend_from_slice(&rel(c.len() + 4, t));
    c.extend_from_slice(&[0x89, 0x07, 0xc3]);
    c
}
/// T: mov eax,edx; mul ecx; jae L; inc eax; L: ret      (jae reads CF of a multiplication: no recipe, so no flow graph)
fn code_t() -> Vec<u8> {
    vec![0x89, 0xd0, 0xf7, 0xe1, 0x73, 0x02, 0xff, 0xc0, 0xc3]
}

const JS: &str = r#"
const fs=require('fs');
const [wasmF, inF, outF, tva] = process.argv.slice(2);
const mem=new WebAssembly.Memory({initial:16,maximum:32768,shared:true});
const u8=new Uint8Array(mem.buffer); const dv=new DataView(mem.buffer);
const m=new WebAssembly.Module(fs.readFileSync(wasmF));
const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};
let called=0;
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind!=='function') continue;
  imports[i.module]=imports[i.module]||{};
  imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); };
}
// the C path's T: reads edx (R[2]) and ecx (R[1]) from the register file, as the C path does, and leaves the result in rax (R[0])
imports.c['f_'+tva]=()=>{ called++; const v=BigInt(dv.getUint32(0x2000+8*2,true)); const c=BigInt(dv.getUint32(0x2000+8*1,true)); const p=v*c; let r=Number(p&0xffffffffn); if ((p>>32n)!==0n) r=(r+1)>>>0; dv.setBigUint64(0x2000, BigInt(r), true); };
dv.setUint32(0x3000,0x1fa0,true);
const inst=new WebAssembly.Instance(m,imports);
const inp=fs.readFileSync(inF); const n=inp.length/8; const out=Buffer.alloc(4*n);
for (let k=0;k<n;k++){
  u8.set(inp.subarray(8*k,8*k+8),0x10000); u8.fill(0,0x20000,0x20010);
  dv.setBigUint64(0x2000+8*4, 0x80000n, true); dv.setBigUint64(0x2000+8*1, 0x10000n, true); dv.setBigUint64(0x2000+8*7, 0x20000n, true);
  inst.exports.f_401000();
  out.set(u8.subarray(0x20000,0x20004),4*k);
}
if (called!==n) throw new Error('T was called '+called+' times for '+n+' inputs');
fs.writeFileSync(outF,out);
"#;

#[test]
fn a_caller_of_a_function_without_a_flag_recipe_is_lowered_and_calls_it_as_a_stub() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let t_at = 0x100usize;
    let funcs = vec![(0usize, code_a(t_at)), (t_at, code_t())];
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
        hybrid_only: Some(HashSet::from([0x401000u64])),
        ..Config::default()
    };
    let comp = compile(&pe(&funcs), &[0x401000, 0x401000 + t_at as u64], &[], &conf).expect("compile");
    assert!(comp.fidx.contains_key(&0x401000), "A is lowered although its callee T has no flow graph: {:?}", comp.report.rows.iter().find(|r| r.va == 0x401000).map(|r| r.status.clone()));
    let tstatus = comp.report.rows.iter().find(|r| r.va == 0x401000 + t_at as u64).map(|r| r.status.clone()).unwrap_or_default();
    assert!(!tstatus.is_empty(), "T is in the report");
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-opq-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let w = dir.join("t.wasm");
    std::fs::write(&w, &comp.wasm).unwrap();
    if have("wasm-validate") {
        let o = Command::new("wasm-validate").arg("--enable-all").arg(&w).output().unwrap();
        assert!(o.status.success(), "wasm-validate: {}", String::from_utf8_lossy(&o.stderr));
    }
    let mut s = 0x9e3779b97f4a7c15u64;
    let mut ins: Vec<(u32, u32)> = vec![(0xffff_ffff, 1), (0xffff_fffe, 1), (1, 1), (0x8000_0000, 2), (0x8000_0001, 2), (0, 5), (0x1234_5678, 4), (0xffff_ffff, 0xffff_ffff), (0xffff_ffff, 33), (0x7fff_ffff, 32 + 7)];
    for _ in 0..2000 {
        let v = rng(&mut s) as u32;
        let c = if rng(&mut s) % 2 == 0 { rng(&mut s) as u32 } else { (rng(&mut s) % 31) as u32 + 1 };
        ins.push((v, c));
    }
    let mut flat: Vec<u8> = Vec::new();
    for (v, c) in &ins {
        flat.extend_from_slice(&v.to_le_bytes());
        flat.extend_from_slice(&c.to_le_bytes());
    }
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(&js, JS).unwrap();
    let outp = dir.join("out.bin");
    let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).arg(format!("{:x}", 0x401000 + t_at)).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr).chars().take(500).collect::<String>());
    let got = std::fs::read(&outp).unwrap();
    let mut bad = Vec::new();
    for (k, (v, c)) in ins.iter().enumerate() {
        let p = *v as u64 * *c as u64;
        let want = (p as u32).wrapping_add(((p >> 32) != 0) as u32);
        let g = u32::from_le_bytes(got[4 * k..4 * k + 4].try_into().unwrap());
        if g != want {
            bad.push(format!("v {v:#x} count {c}: want {want:#x} got {g:#x}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(bad.is_empty(), "{} of {} differ; first: {:?}", bad.len(), ins.len(), bad.first());
}
