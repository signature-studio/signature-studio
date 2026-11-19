//! Decision 0234 (mt3, 0254): three small instructions that kept a function out of the direct wasm of the hybrid, in the style of mx_hybrid.rs (compile in hybrid mode, run under node, compare):
//!   (a) JECXZ / JRCXZ (a terminator that reads (r|e)cx and no flag; the 32-bit address size of JECXZ is the low half only), in a straight line and as the header of a loop;
//!   (b) CPUID (the answer depends only on the leaf: compared with runtime/fle_cpuid.c, compiled and run natively, for a list of leaves with garbage in the high halves);
//!   (c) FNSTCW / FLDCW (the control word is state of the C path: the direct code calls `c.fle_x87_store_cw` / `c.fle_x87_load_cw` with the effective address; the stub here implements what fle_x87ctl.c does, and the test pins that
//!       source text); every one of the 65,536 control words goes through FLDCW and FNSTCW, and the helpers are imported only by a module that has the instructions.
//! Skipped (and passing) when node or clang is missing, or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::{compile, Config};
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
    pe_with(code, None)
}

/// as `pe`, with a second function at 0x401100 (`callee`)
fn pe_with(code: &[u8], callee: Option<&[u8]>) -> Vec<u8> {
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
    w32(&mut b, opt + 112 + 3 * 8 + 4, if callee.is_some() { 24 } else { 12 });
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
    if let Some(c) = callee {
        b[0x500..0x500 + c.len()].copy_from_slice(c);
        w32(&mut b, 0x80c, 0x1100);
        w32(&mut b, 0x810, 0x1100 + c.len() as u32);
        w32(&mut b, 0x814, 0x2100);
    }
    b[0x900..0x904].copy_from_slice(&[1, 0, 0, 0]);
    b
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}


fn conf() -> Config {
    let mut ci: HashMap<String, String> = HashMap::new();
    ci.insert("dummy".into(), "fle_imp_dummy".into());
    Config {
        hybrid_r: Some(0x2000),
        tls_ctx: Some(0x2000),
        hybrid_x: Some(0x2200),
        hybrid_mx: Some((0x3000, 0x3100)),
        c_imports: Some(ci),
        mem_import: Some(("c".into(), "memory".into())),
        teb: Some(0x1000),
        ..Config::default()
    }
}

/// the wasm of the function at 0x401000 that holds `code`
fn wasm_of(code: &[u8]) -> Vec<u8> {
    let comp = compile(&pe(code), &[], &[], &conf()).unwrap_or_else(|e| panic!("compile: {e}"));
    assert!(comp.fidx.contains_key(&0x401000), "the function was not lowered (it stays on the C path): {code:02x?}");
    comp.wasm
}

const JS: &str = r#"
const fs=require('fs');
const [wasmF, casesF, outF] = process.argv.slice(2);
const mem=new WebAssembly.Memory({initial:16,maximum:32768,shared:true});
const dv=new DataView(mem.buffer);
const m=new WebAssembly.Module(fs.readFileSync(wasmF));
const R=0x2000;
let cw=0x037F; const calls=[];
const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};
const names=[];
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind==='function') names.push(i.module+'.'+i.name);
  if (i.kind!=='function') continue;
  imports[i.module]=imports[i.module]||{};
  imports[i.module][i.name]=()=>{ throw new Error('unexpected call of '+i.module+'.'+i.name); };
}
// fle_x87ctl.c: store: the control word to the 2 bytes at the address; load: (v & 0x1F3F) | 0x0040
imports.c.fle_x87_store_cw=(a)=>{ calls.push(['store',Number(a)]); dv.setUint16(Number(a),cw,true); };
imports.c.fle_x87_load_cw=(a)=>{ calls.push(['load',Number(a)]); const v=dv.getUint16(Number(a),true); cw=(v&0x1F3F)|0x0040; };
// the C path's function at 0x401100 in the test of a caller (mov eax,ecx; jecxz +2; inc eax; ret): what the lifted C does with the registers in the context
if (imports.c.f_401100) imports.c.f_401100=()=>{ const ecx=Number(dv.getBigUint64(R+8,true)&0xffffffffn); let eax=ecx; if (ecx!==0) eax=(eax+1)>>>0; dv.setBigUint64(R,BigInt(eax),true); };
const inst=new WebAssembly.Instance(m,imports);
const cases=JSON.parse(fs.readFileSync(casesF,'utf8'));
const out=[];
for (const c of cases) {
  for (let r=0;r<16;r++) dv.setBigUint64(R+8*r, BigInt(c.regs && c.regs[r]!==undefined ? c.regs[r] : '0'), true);
  dv.setBigUint64(R+8*4, 0x80000n, true);
  for (const [a,v] of (c.mem||[])) dv.setUint16(a, v, true);
  if (c.cw!==undefined) cw=c.cw;
  calls.length=0;
  inst.exports.f_401000();
  const regs=[]; for (let r=0;r<16;r++) regs.push(dv.getBigUint64(R+8*r,true).toString());
  out.push({regs, cw, calls:calls.slice(), mem:(c.read||[]).map(a=>dv.getUint16(a,true)), memd:(c.readd||[]).map(a=>dv.getUint32(a,true))});
}
fs.writeFileSync(outF, JSON.stringify({names, out}));
"#;

fn run_node(wasm: &[u8], cases: &str, tag: &str) -> serde_like::Out {
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-small-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.wasm"), wasm).unwrap();
    std::fs::write(dir.join("cases.json"), cases).unwrap();
    std::fs::write(dir.join("run.cjs"), JS).unwrap();
    let o = Command::new("node").arg(dir.join("run.cjs")).arg(dir.join("t.wasm")).arg(dir.join("cases.json")).arg(dir.join("out.json")).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    let txt = std::fs::read_to_string(dir.join("out.json")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    serde_like::parse(&txt)
}

/// a very small reader for the JSON that the harness writes (no serde in this crate): { "names": [..], "out": [ { "regs": [..16 strings..], "cw": n, "calls": [[s,n]..], "mem": [n..] } .. ] }
mod serde_like {
    pub struct Case {
        pub regs: Vec<u64>,
        pub cw: u32,
        pub calls: Vec<(String, u64)>,
        pub mem: Vec<u32>,
        pub memd: Vec<u32>,
    }
    pub struct Out {
        pub names: Vec<String>,
        pub out: Vec<Case>,
    }
    fn strs(s: &str) -> Vec<String> {
        s.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, t)| t.to_string()).collect()
    }
    fn nums(s: &str) -> Vec<u64> {
        s.split(|c: char| !c.is_ascii_digit()).filter(|t| !t.is_empty()).map(|t| t.parse().unwrap()).collect()
    }
    pub fn parse(t: &str) -> Out {
        let names_s = &t[t.find("\"names\":[").unwrap() + 9..];
        let names_end = names_s.find(']').unwrap();
        let names = strs(&names_s[..names_end]);
        let mut out = Vec::new();
        let body = &t[t.find("\"out\":[").unwrap() + 7..];
        for chunk in body.split("{\"regs\":[").skip(1) {
            let re = chunk.find(']').unwrap();
            let regs: Vec<u64> = strs(&chunk[..re]).iter().map(|x| x.parse().unwrap()).collect();
            let rest = &chunk[re..];
            let cw = nums(&rest[rest.find("\"cw\":").unwrap() + 5..rest.find(",\"calls\"").unwrap()])[0] as u32;
            let cs = &rest[rest.find("\"calls\":[").unwrap() + 9..rest.find(",\"mem\":[").unwrap()];
            let mut calls = Vec::new();
            for c in cs.split("[\"").skip(1) {
                let kind = c[..c.find('"').unwrap()].to_string();
                let n = nums(&c[c.find(',').unwrap()..c.find(']').unwrap()])[0];
                calls.push((kind, n));
            }
            let ms = &rest[rest.find("\"mem\":[").unwrap() + 7..];
            let mem = nums(&ms[..ms.find(']').unwrap()]).iter().map(|&x| x as u32).collect();
            let md = &rest[rest.find("\"memd\":[").unwrap() + 8..];
            let memd = nums(&md[..md.find(']').unwrap()]).iter().map(|&x| x as u32).collect();
            out.push(Case { regs, cw, calls, mem, memd });
        }
        Out { names, out }
    }
}

fn skip() -> bool {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return true;
    }
    false
}

fn cases_json(list: &[(u64, u64)]) -> String {
    // (rax, rcx) pairs; the other registers hold garbage
    let v: Vec<String> = list.iter().map(|&(a, c)| format!("{{\"regs\":{{\"0\":\"{a}\",\"1\":\"{c}\",\"2\":\"{}\",\"3\":\"{}\"}}}}", 0xdead_beef_u64 << 20, 0x1234_5678_9abc_u64)).collect();
    format!("[{}]", v.join(","))
}

// ---------------------------------------------------------------------------------------------------------------- (a)

#[test]
fn jrcxz_and_jecxz_branch_on_rcx_and_on_ecx_only() {
    if skip() {
        return;
    }
    // xor eax,eax; j(r|e)cxz +5; mov eax,1; ret
    let jrcxz: Vec<u8> = vec![0x31, 0xc0, 0xe3, 0x05, 0xb8, 1, 0, 0, 0, 0xc3];
    let jecxz: Vec<u8> = vec![0x31, 0xc0, 0x67, 0xe3, 0x05, 0xb8, 1, 0, 0, 0, 0xc3];
    let vals: Vec<u64> = vec![0, 1, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x1_0000_0000, 0x1_0000_0001, 0xffff_ffff_0000_0000, u64::MAX, 0x100, 0x1_0000];
    let cases = cases_json(&vals.iter().map(|&c| (0x1111_1111_1111_1111u64, c)).collect::<Vec<_>>());
    for (name, code, wide) in [("jrcxz", jrcxz, true), ("jecxz", jecxz, false)] {
        let r = run_node(&wasm_of(&code), &cases, name);
        for (k, &c) in vals.iter().enumerate() {
            let zero = if wide { c == 0 } else { c & 0xffff_ffff == 0 };
            let want: u64 = if zero { 0 } else { 1 };
            assert_eq!(r.out[k].regs[0], want, "{name} rcx={c:#x}: eax");
            assert_eq!(r.out[k].regs[1], c, "{name} rcx={c:#x}: rcx unchanged");
        }
    }
}

/// A function that stays on the C path and holds a JECXZ: its signature (which registers it reads and changes) has to be solved over the whole function. Before this change the JECXZ ended the block with no successor, the
/// solver saw only the part before it, and a caller in the direct wasm kept its own rax across the call instead of reading the result back from the context. Decision 0254, found with `tools/wasmgen-same.sh` (the two
/// functions of FLEngine that call f_6a1e50 / f_6a22d0 changed by 14 and 30 bytes).
#[test]
fn a_caller_reads_back_the_result_of_a_c_path_function_that_holds_a_jecxz() {
    if skip() {
        return;
    }
    // caller (0x401000): call 0x401100; add eax,1; ret          callee (0x401100): mov eax,ecx; jecxz +2; inc eax; ret
    let caller: Vec<u8> = vec![0xe8, 0xfb, 0x00, 0x00, 0x00, 0x83, 0xc0, 0x01, 0xc3];
    let callee: Vec<u8> = vec![0x89, 0xc8, 0x67, 0xe3, 0x02, 0xff, 0xc0, 0xc3];
    let mut c = conf();
    c.hybrid_only = Some([0x401000u64].into_iter().collect());
    let comp = compile(&pe_with(&caller, Some(&callee)), &[], &[], &c).unwrap_or_else(|e| panic!("compile: {e}"));
    assert!(comp.fidx.contains_key(&0x401000), "the caller was not lowered");
    let vals: Vec<u64> = vec![0, 1, 5, 0x7fff_ffff, 0xffff_ffff, 0x1_0000_0000, 0x1_0000_0005];
    let cases = cases_json(&vals.iter().map(|&v| (0x1234_5678_9abc_def0u64, v)).collect::<Vec<_>>());
    let r = run_node(&comp.wasm, &cases, "caller-jecxz");
    assert!(r.names.iter().any(|n| n == "c.f_401100"), "the callee is not selected: the caller calls it through the C path's function: {:?}", r.names);
    for (k, &v) in vals.iter().enumerate() {
        let ecx = v & 0xffff_ffff;
        let callee_eax = if ecx == 0 { 0 } else { (ecx + 1) & 0xffff_ffff };
        assert_eq!(r.out[k].regs[0], (callee_eax + 1) & 0xffff_ffff, "rcx={v:#x}: eax after the call and add eax,1");
    }
}

#[test]
fn jecxz_as_the_header_of_a_loop_counts_down_ecx() {
    if skip() {
        return;
    }
    // xor eax,eax; L: jecxz done; inc eax; dec ecx; jmp L; nop; nop; done: ret
    let code: Vec<u8> = vec![0x31, 0xc0, 0x67, 0xe3, 0x08, 0xff, 0xc0, 0xff, 0xc9, 0xeb, 0xf7, 0x90, 0x90, 0xc3];
    let vals: Vec<u64> = vec![0, 1, 2, 3, 7, 100, 1000, 0x1_0000_0000, 0x1_0000_0003];
    let cases = cases_json(&vals.iter().map(|&c| (0u64, c)).collect::<Vec<_>>());
    let r = run_node(&wasm_of(&code), &cases, "jecxz-loop");
    for (k, &c) in vals.iter().enumerate() {
        // ecx counts down from its low half (dec ecx wraps in 32 bits, so a value with a zero low half does not enter the loop)
        let want = c & 0xffff_ffff;
        assert_eq!(r.out[k].regs[0], want, "ecx={c:#x}: eax");
    }
}

// ---------------------------------------------------------------------------------------------------------------- (b)

fn cpuid_c_reference(leaves: &[(u64, u64)]) -> Vec<[u64; 4]> {
    let rt = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-cpuid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut inputs = String::new();
    for &(a, c) in leaves {
        inputs.push_str(&format!("    {{ UINT64_C({a}), UINT64_C({c}) }},\n"));
    }
    let src = format!(
        "#include <stdio.h>\n#include <stdint.h>\n#include \"{}\"\nuint64_t R[16];\nstatic const uint64_t IN[][2] = {{\n{inputs}}};\nint main(void)\n{{\n    for (unsigned k = 0; k < sizeof IN / sizeof IN[0]; k++) {{\n        R[0] = IN[k][0]; R[1] = IN[k][1]; R[2] = 0x1234; R[3] = 0x5678;\n        fle_cpuid();\n        printf(\"%llu %llu %llu %llu\\n\", (unsigned long long)R[0], (unsigned long long)R[3], (unsigned long long)R[1], (unsigned long long)R[2]);\n    }}\n    return 0;\n}}\n",
        rt.join("fle_cpuid.c").display()
    );
    std::fs::write(dir.join("c.c"), src).unwrap();
    let cc = Command::new("clang").args(["-O0", "-std=gnu11", "-w", "-I"]).arg(&rt).arg("-o").arg(dir.join("c")).arg(dir.join("c.c")).output().expect("clang");
    assert!(cc.status.success(), "clang: {}", String::from_utf8_lossy(&cc.stderr));
    let o = Command::new(dir.join("c")).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&o.stdout).lines().map(|l| {
        let v: Vec<u64> = l.split(' ').map(|x| x.parse().unwrap()).collect();
        [v[0], v[1], v[2], v[3]]
    }).collect()
}

#[test]
fn cpuid_gives_what_the_c_function_gives() {
    if skip() || !have("clang") {
        return;
    }
    let mut leaves: Vec<(u64, u64)> = vec![];
    for l in [0u64, 1, 2, 3, 4, 5, 6, 7, 0xb, 0xd, 0x14, 0x15, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, 0x8000_0008, 0xc000_0000, 0xffff_ffff] {
        for hi in [0u64, 0xdead_beef_0000_0000] {
            leaves.push((hi | l, 0));
            leaves.push((hi | l, 0x1_0000_0003));
        }
    }
    let want = cpuid_c_reference(&leaves);
    // cpuid; mov [rdi],ebx; mov [rdi+4],ecx; mov [rdi+8],edx; ret   (eax is the result of the function; the other three go to memory)
    let code: Vec<u8> = vec![0x0f, 0xa2, 0x89, 0x1f, 0x89, 0x4f, 0x04, 0x89, 0x57, 0x08, 0xc3];
    let cs: Vec<String> = leaves.iter().map(|&(a, c)| format!("{{\"regs\":{{\"0\":\"{a}\",\"1\":\"{c}\",\"2\":\"{}\",\"3\":\"{}\",\"7\":\"327680\"}},\"readd\":[327680,327684,327688]}}", 0xdead_beef_u64 << 20, 0x1234_5678_9abc_u64)).collect();
    let r = run_node(&wasm_of(&code), &format!("[{}]", cs.join(",")), "cpuid");
    for (k, &(a, c)) in leaves.iter().enumerate() {
        let g = &r.out[k];
        // the C function: R[0] = eax, R[3] = ebx, R[1] = ecx, R[2] = edx (each a 32-bit value)
        assert_eq!([g.regs[0], g.memd[0] as u64, g.memd[1] as u64, g.memd[2] as u64], want[k], "cpuid leaf {a:#x} ecx {c:#x}");
    }
}

// ---------------------------------------------------------------------------------------------------------------- (c)

#[test]
fn fldcw_then_fnstcw_round_trips_every_control_word_through_the_c_helpers() {
    if skip() {
        return;
    }
    // fldcw [rcx+8]; fnstcw [rcx+0x10]; ret
    let code: Vec<u8> = vec![0xd9, 0x69, 0x08, 0xd9, 0x79, 0x10, 0xc3];
    let base = 0x40000u64;
    let mut cases = Vec::new();
    for v in 0..65536u32 {
        cases.push(format!("{{\"regs\":{{\"1\":\"{base}\"}},\"mem\":[[{},{v}]],\"read\":[{}]}}", base + 8, base + 0x10));
    }
    let r = run_node(&wasm_of(&code), &format!("[{}]", cases.join(",")), "x87cw");
    assert!(r.names.iter().any(|n| n == "c.fle_x87_load_cw") && r.names.iter().any(|n| n == "c.fle_x87_store_cw"), "the helpers are imported: {:?}", r.names);
    for v in 0..65536u32 {
        let want = (v & 0x1F3F) | 0x40;
        let c = &r.out[v as usize];
        assert_eq!(c.mem[0], want, "control word {v:#06x}: stored");
        assert_eq!(c.cw, want, "control word {v:#06x}: state");
        assert_eq!(c.calls, vec![("load".to_string(), base + 8), ("store".to_string(), base + 0x10)], "control word {v:#06x}: the calls (effective addresses)");
    }
}

#[test]
fn fnstcw_stores_the_initial_control_word_and_the_effective_address_is_base_index_scale_disp() {
    if skip() {
        return;
    }
    // fnstcw [rcx+rdx*2+4]; ret
    let a: Vec<u8> = vec![0xd9, 0x7c, 0x51, 0x04, 0xc3];
    let cases = "[{\"regs\":{\"1\":\"262144\",\"2\":\"100\"},\"read\":[262348]},{\"regs\":{\"1\":\"262144\",\"2\":\"0\"},\"read\":[262148]}]";
    for (name, code) in [("fnstcw", a)] {
        let r = run_node(&wasm_of(&code), cases, name);
        assert_eq!(r.out[0].mem[0], 0x037F, "{name}: the initial control word");
        assert_eq!(r.out[0].calls, vec![("store".to_string(), 262144 + 200 + 4)], "{name}: effective address rcx+rdx*2+4");
        assert_eq!(r.out[1].calls, vec![("store".to_string(), 262144 + 4)], "{name}: effective address with rdx = 0");
    }
}

#[test]
fn fldcw_and_fnstcw_with_an_absolute_address() {
    if skip() {
        return;
    }
    // fldcw [0x41000]; fnstcw [0x41010]; ret
    let code: Vec<u8> = vec![0xd9, 0x2c, 0x25, 0x00, 0x10, 0x04, 0x00, 0xd9, 0x3c, 0x25, 0x10, 0x10, 0x04, 0x00, 0xc3];
    let r = run_node(&wasm_of(&code), "[{\"mem\":[[266240,2431]],\"read\":[266256]}]", "x87abs");
    let want = (2431u32 & 0x1F3F) | 0x40;
    assert_eq!(r.out[0].mem[0], want);
    assert_eq!(r.out[0].calls, vec![("load".to_string(), 0x41000), ("store".to_string(), 0x41010)]);
}

#[test]
fn the_helpers_are_imported_only_by_a_module_that_has_the_instructions() {
    if skip() {
        return;
    }
    let r = run_node(&wasm_of(&[0x31, 0xc0, 0xc3]), "[{}]", "nox87");
    assert!(!r.names.iter().any(|n| n.contains("x87")), "no x87 control word instruction, no helper import: {:?}", r.names);
}

#[test]
fn the_stub_here_is_what_fle_x87ctl_c_does() {
    // the JS stub implements (v & 0x1F3F) | 0x40 and the store of the control word; pin the C text so that a change there fails this test
    let t = std::fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/fle_x87ctl.c")).unwrap();
    assert!(t.contains("fle_x87.cw = (uint16_t)((v & 0x1F3Fu) | 0x0040u);"), "load_cw changed");
    assert!(t.contains("fle_st(a, 2, fle_x87.cw);"), "store_cw changed");
    assert!(t.contains("fle_x87_state fle_x87 = { 0x037F"), "the initial control word changed");
}
