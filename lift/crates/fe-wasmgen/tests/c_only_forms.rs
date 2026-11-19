//! Decision 0252: the forms of the hybrid mode that call the C path's function for the instruction every time (RSQRTPS, RSQRTSS, RCPPS, ROUNDPS, ROUNDPD: `c.fle_hyb_mx` kinds 61..65 of lift/harness/hyb1/hybrid_glue.c) and
//! the plain MOVSLDUP / MOVSHDUP. The C path's function is a JS stub here that computes a recognisable function of its operands (destination, source, the immediate), so that the test checks the marshalling: the
//! destination and the source in the right slots (register and memory sources), the immediate, the kind, the lane order, the upper lanes of RSQRTSS kept, and the result written back to the destination register. The
//! tables of the hardware (what RSQRTPS answers) are the C path's own and are tested with it. MOVSLDUP / MOVSHDUP are compared with the host CPU. Skipped (and passing) when node is missing or the host is not x86-64.
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


struct Form {
    name: &'static str,
    insn: Vec<u8>,
    /// the C-path kind this form must call (None: the plain lowering, compared with the host)
    kind: Option<u32>,
    imm: u32,
    /// the source is the 16 bytes at rcx+16 in memory (else xmm1, loaded from the same place)
    sbytes: usize,
}

fn forms() -> Vec<Form> {
    let mut v = Vec::new();
    for (mem, tail) in [(false, vec![0xc1u8]), (true, vec![0x41, 0x10])] {
        let nm = |s: &'static str| -> &'static str { Box::leak(format!("{s}{}", if mem { " m128" } else { "" }).into_boxed_str()) };
        v.push(Form { name: nm("rsqrtps"), insn: [vec![0x0f, 0x52], tail.clone()].concat(), kind: Some(61), imm: 0, sbytes: 16 });
        v.push(Form { name: nm("rsqrtss"), insn: [vec![0xf3, 0x0f, 0x52], tail.clone()].concat(), kind: Some(62), imm: 0, sbytes: 4 });
        v.push(Form { name: nm("rcpps"), insn: [vec![0x0f, 0x53], tail.clone()].concat(), kind: Some(63), imm: 0, sbytes: 16 });
        for imm in [0u32, 1, 2, 3, 9] {
            v.push(Form { name: nm("roundps"), insn: [vec![0x66, 0x0f, 0x3a, 0x08], tail.clone(), vec![imm as u8]].concat(), kind: Some(64), imm, sbytes: 16 });
            v.push(Form { name: nm("roundpd"), insn: [vec![0x66, 0x0f, 0x3a, 0x09], tail.clone(), vec![imm as u8]].concat(), kind: Some(65), imm, sbytes: 16 });
        }
        v.push(Form { name: nm("movsldup"), insn: [vec![0xf3, 0x0f, 0x12], tail.clone()].concat(), kind: None, imm: 0, sbytes: 16 });
        v.push(Form { name: nm("movshdup"), insn: [vec![0xf3, 0x0f, 0x16], tail.clone()].concat(), kind: None, imm: 0, sbytes: 16 });
    }
    v
}

fn lane32(b: &[u8], k: usize) -> u32 { u32::from_le_bytes(b[4 * k..4 * k + 4].try_into().unwrap()) }
fn lane64(b: &[u8], k: usize) -> u64 { u64::from_le_bytes(b[8 * k..8 * k + 8].try_into().unwrap()) }

/// what the JS stub of kind `kind` returns for the operands d, s (16 bytes each) and the immediate
fn stub(kind: u32, d: &[u8], s: &[u8], imm: u32) -> [u8; 16] {
    let mut r = [0u8; 16];
    match kind {
        61 => (0..4).for_each(|k| r[4 * k..4 * k + 4].copy_from_slice(&(lane32(s, k) ^ (0x1111_1111u32.wrapping_mul(k as u32 + 1))).to_le_bytes())),
        62 => {
            r.copy_from_slice(d);
            r[0..4].copy_from_slice(&(lane32(s, 0) ^ 0x3333_3333).to_le_bytes());
        }
        63 => (0..4).for_each(|k| r[4 * k..4 * k + 4].copy_from_slice(&lane32(s, k).wrapping_add(k as u32 + 7).to_le_bytes())),
        64 => (0..4).for_each(|k| r[4 * k..4 * k + 4].copy_from_slice(&lane32(s, k).wrapping_add(imm + 100).to_le_bytes())),
        _ => (0..2).for_each(|k| r[8 * k..8 * k + 8].copy_from_slice(&lane64(s, k).wrapping_add(imm as u64 + 200).to_le_bytes())),
    }
    r
}

const JS: &str = r#"
const fs=require('fs');
const [wasmF, inF, outF] = process.argv.slice(2);
const mem=new WebAssembly.Memory({initial:16,maximum:32768,shared:true});
const u8=new Uint8Array(mem.buffer); const dv=new DataView(mem.buffer);
const BUF=0x3100;
const m=new WebAssembly.Module(fs.readFileSync(wasmF));
const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};
for (const i of WebAssembly.Module.imports(m)) { if (i.kind!=='function') continue; imports[i.module]=imports[i.module]||{}; imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); }; }
let calls=[];
imports.c.fle_hyb_mx=(kind,imm)=>{
  calls.push([kind,imm]);
  const d=u8.subarray(BUF,BUF+16), s=u8.subarray(BUF+16,BUF+32), r=u8.subarray(BUF+32,BUF+48);
  const dd=new DataView(d.buffer,d.byteOffset,16), sd=new DataView(s.buffer,s.byteOffset,16), rd=new DataView(r.buffer,r.byteOffset,16);
  if (kind===61) for (let k=0;k<4;k++) rd.setUint32(4*k,(sd.getUint32(4*k,true)^Math.imul(0x11111111,k+1))>>>0,true);
  else if (kind===62) { r.set(d); rd.setUint32(0,(sd.getUint32(0,true)^0x33333333)>>>0,true); }
  else if (kind===63) for (let k=0;k<4;k++) rd.setUint32(4*k,(sd.getUint32(4*k,true)+k+7)>>>0,true);
  else if (kind===64) for (let k=0;k<4;k++) rd.setUint32(4*k,(sd.getUint32(4*k,true)+imm+100)>>>0,true);
  else if (kind===65) for (let k=0;k<2;k++) rd.setBigUint64(8*k,(sd.getBigUint64(8*k,true)+BigInt(imm)+200n)&0xffffffffffffffffn,true);
  else throw new Error('kind '+kind);
};
dv.setUint32(0x3000,0x1fa0,true);
const inst=new WebAssembly.Instance(m,imports);
const inp=fs.readFileSync(inF); const n=inp.length/32; const out=Buffer.alloc(16*n); const info=[];
for (let k=0;k<n;k++){
  u8.set(inp.subarray(32*k,32*k+32),0x10000); u8.fill(0,0x20000,0x20010);
  dv.setBigUint64(0x2000+8*4, 0x80000n, true); dv.setBigUint64(0x2000+8*1, 0x10000n, true); dv.setBigUint64(0x2000+8*2, 0x20000n, true);
  calls=[]; inst.exports.f_401000();
  out.set(u8.subarray(0x20000,0x20010),16*k); info.push(calls.length===1?calls[0][0]*1000+calls[0][1]:(calls.length===0?-1:-2));
}
fs.writeFileSync(outF,out); fs.writeFileSync(outF+'.calls',JSON.stringify(info));
"#;

#[test]
fn the_c_path_forms_marshal_their_operands_and_the_plain_forms_match_the_host() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let mut s = 0x9e3779b97f4a7c15u64;
    let ins: Vec<[u8; 32]> = (0..200).map(|_| { let mut b = [0u8; 32]; for l in 0..4 { b[8 * l..8 * l + 8].copy_from_slice(&rng(&mut s).to_le_bytes()); } b }).collect();
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-conly-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flat: Vec<u8> = ins.iter().flat_map(|b| b.iter().copied()).collect();
    let inp = dir.join("in.bin");
    std::fs::write(&inp, &flat).unwrap();
    let js = dir.join("run.cjs");
    std::fs::write(&js, JS).unwrap();
    let mut failures: Vec<String> = Vec::new();
    for f in forms() {
        let mut code: Vec<u8> = vec![0x0f, 0x10, 0x01, 0x0f, 0x10, 0x49, 0x10];   // movups xmm0,[rcx]; movups xmm1,[rcx+16]
        code.extend_from_slice(&f.insn);
        code.extend_from_slice(&[0x0f, 0x11, 0x02, 0xc3]);                          // movups [rdx],xmm0; ret
        let mut ci: HashMap<String, String> = HashMap::new();
        ci.insert("dummy".into(), "fle_imp_dummy".into());
        let conf = Config { hybrid_r: Some(0x2000), tls_ctx: Some(0x2000), hybrid_x: Some(0x2200), hybrid_mx: Some((0x3000, 0x3100)), c_imports: Some(ci), mem_import: Some(("c".into(), "memory".into())), teb: Some(0x1000), ..Config::default() };
        let comp = match compile(&pe(&code), &[], &[], &conf) {
            Ok(x) => x,
            Err(e) => {
                failures.push(format!("{}: compile: {e}", f.name));
                continue;
            }
        };
        if !comp.fidx.contains_key(&0x401000) {
            let st = comp.report.rows.iter().find(|r| r.va == 0x401000).map(|r| r.status.clone()).unwrap_or_default();
            failures.push(format!("{}: not lowered: {st}", f.name));
            continue;
        }
        let w = dir.join("t.wasm");
        std::fs::write(&w, &comp.wasm).unwrap();
        if have("wasm-validate") {
            let o = Command::new("wasm-validate").arg("--enable-all").arg(&w).output().unwrap();
            if !o.status.success() {
                failures.push(format!("{}: wasm-validate: {}", f.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
                continue;
            }
        }
        let outp = dir.join("out.bin");
        let o = Command::new("node").arg(&js).arg(&w).arg(&inp).arg(&outp).output().unwrap();
        if !o.status.success() {
            failures.push(format!("{}: node: {}", f.name, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")));
            continue;
        }
        let got = std::fs::read(&outp).unwrap();
        let calls: Vec<i64> = serde_like(&std::fs::read_to_string(dir.join("out.bin.calls")).unwrap());
        let (mut bad, mut first) = (0, String::new());
        for (k, b) in ins.iter().enumerate() {
            let (d, sraw) = (&b[0..16], &b[16..32]);
            let want: [u8; 16] = match f.kind {
                Some(kind) => {
                    if calls[k] != (kind as i64) * 1000 + f.imm as i64 {
                        bad += 1;
                        if first.is_empty() { first = format!("call {} (want kind {} imm {})", calls[k], kind, f.imm); }
                        continue;
                    }
                    let mut sv = [0u8; 16];
                    sv[..f.sbytes].copy_from_slice(&sraw[..f.sbytes]);
                    // the C path's function gets the whole 16 bytes of a register source; the scalar form's upper bytes are not defined by the test
                    if f.sbytes == 4 { sv[4..].copy_from_slice(&sraw[4..]); }
                    stub(kind, d, &sv, f.imm)
                }
                None => {
                    if calls[k] != -1 {
                        bad += 1;
                        if first.is_empty() { first = "the C path was called".into(); }
                        continue;
                    }
                    let x: __m128 = unsafe { std::mem::transmute::<[u8; 16], __m128>(sraw.try_into().unwrap()) };
                    let r = unsafe { if f.name.starts_with("movsldup") { _mm_moveldup_ps(x) } else { _mm_movehdup_ps(x) } };
                    unsafe { std::mem::transmute::<__m128, [u8; 16]>(r) }
                }
            };
            let g: [u8; 16] = got[16 * k..16 * k + 16].try_into().unwrap();
            if g != want {
                bad += 1;
                if first.is_empty() { first = format!("in {:02x?} want {:02x?} got {:02x?}", b, want, g); }
            }
        }
        if bad > 0 {
            failures.push(format!("{} (imm {}): {bad} of {} inputs differ; first: {first}", f.name, f.imm, ins.len()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// the one JSON shape of the stub's call log: a flat array of integers
fn serde_like(t: &str) -> Vec<i64> {
    t.trim().trim_start_matches('[').trim_end_matches(']').split(',').filter(|x| !x.trim().is_empty()).map(|x| x.trim().parse().unwrap()).collect()
}
