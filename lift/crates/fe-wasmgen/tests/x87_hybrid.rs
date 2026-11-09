//! Decision 0259 (mt3x): x87 in the direct wasm of the hybrid = a call of the C runtime's own function (src/x87.rs), the same function with the same constant arguments that the lifted C calls (fe-lift/src/x87.rs, x87ctl.rs).
//! The C runtime's x87 is tested against the host CPU by fe-lift/tests/x87.rs, x87_trans.rs and x87ctl.rs (random operands, special values, every rounding and precision control); what is new here is only the choice of the function and its
//! arguments, so these tests pin down exactly that:
//!   1. for EVERY x87 encoding (opcodes D8-DF with every ModRM byte: register forms, and memory forms [rdi] and [rdi+disp8]; plus WAIT), the call that the direct wasm makes is textually the call of the lifted C, with the memory address
//!      argument written ADDR; an encoding that the lifted C does not take is not taken by the direct wasm either; the forms that only the lifted C takes (FCOMI, FUCOMI, FCMOV, FNSTCW, FLDCW) are exactly those listed;
//!   2. the code that is generated: the effective address (base + index * scale + displacement) reaches the helper, the arguments come in order, FNSTSW AX merges the 16 bits into RAX, FWAIT emits nothing, and a module
//!      imports only the helpers its instructions call (run under node with recording stubs).
//! Skipped (and passing) when node is missing, or the host is not x86-64.
#![cfg(target_arch = "x86_64")]

use fe_wasmgen::lower::x87::{call_text, helper_names};
use fe_wasmgen::{compile, Config};
use std::collections::{BTreeSet, HashMap};
use std::process::Command;

/// the lifted C's helper call of the instruction, written like `call_text`: ("fle_x87_fld_m(ADDR,8)", "fle_x87_sw()->AX"); None: the lifted C has no x87 helper call for it
fn lifted_call(bytes: &[u8]) -> Option<Result<String, String>> {
    let body = match fe_lift::lift_body(bytes, 0x401000) {
        Ok(b) => b,
        Err(e) => return Some(Err(e)),
    };
    // the last occurrence of each name is the call (the prototypes come first, in the same braces)
    let mut best: Option<(usize, String)> = None;
    let mut from = 0;
    while let Some(p) = body[from..].find("fle_x87_") {
        let start = from + p;
        let name_end = body[start..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).map(|e| start + e)?;
        if body.as_bytes().get(name_end) == Some(&b'(') {
            best = Some((start, body[start..name_end].to_string()));
            // a prototype is "void name(...);" / "unsigned name(...);" / "uint16_t name(...);" / "void name(uint64_t);": the call is the last occurrence of the name
        }
        from = name_end;
    }
    let (start, name) = best?;
    // every distinct helper name in the text: take the last occurrence of the LAST distinct name in the text order (the call is after its prototype, and a body has one call)
    let open = start + name.len();
    let mut depth = 0;
    let mut args: Vec<String> = vec![];
    let mut cur = String::new();
    let mut end = open;
    for (k, ch) in body[open..].char_indices() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 1 {
                    cur.push(ch);
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open + k;
                    break;
                }
                cur.push(ch);
            }
            ',' if depth == 1 => args.push(std::mem::take(&mut cur)),
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() || !args.is_empty() {
        args.push(cur);
    }
    let _ = end;
    // which parameters are uint64_t (the guest address): by the prototype in the text
    let proto_at = body.find(&format!("{name}(")).unwrap();
    let proto_end = body[proto_at..].find(')').unwrap() + proto_at;
    let params: Vec<&str> = body[proto_at + name.len() + 1..proto_end].split(',').map(|s| s.trim()).filter(|s| !s.is_empty() && *s != "void").collect();
    let shown: Vec<String> = args.iter().enumerate().map(|(k, a)| if params.get(k).map_or(false, |t| t.starts_with("uint64_t")) { "ADDR".to_string() } else { a.trim().to_string() }).collect();
    let ax = if name == "fle_x87_sw" { "->AX" } else { "" };
    Some(Ok(format!("{name}({}){ax}", shown.join(","))))
}

/// the encodings: every opcode D8..DF with every ModRM; the memory forms with base rdi (rm = 7), [rdi] and [rdi+0x10]; and WAIT
fn encodings() -> Vec<Vec<u8>> {
    let mut v = vec![vec![0x9b]];
    for opc in 0xd8u8..=0xdf {
        for modrm in 0u16..=255 {
            let modrm = modrm as u8;
            match modrm >> 6 {
                3 => v.push(vec![opc, modrm]),
                0 if modrm & 7 == 7 => v.push(vec![opc, modrm]),
                1 if modrm & 7 == 7 => v.push(vec![opc, modrm, 0x10]),
                _ => {}
            }
        }
    }
    v
}

/// the forms that only the lifted C takes: the instructions that read or write EFLAGS (FCOMI, FUCOMI, FCMOV: flags.rs would need a definition kind) and the control word (decision 0254)
const ONLY_LIFTED: [&str; 4] = ["fle_x87_comi", "fle_x87_fcmov", "fle_x87_store_cw", "fle_x87_load_cw"];

#[test]
fn every_x87_encoding_makes_the_call_the_lifted_c_makes() {
    let mut same = 0;
    let mut only_lifted: BTreeSet<String> = BTreeSet::new();
    let mut neither = 0;
    let mut helpers_seen: BTreeSet<String> = BTreeSet::new();
    for bytes in encodings() {
        let wasm = call_text(&bytes);
        let lifted = lifted_call(&bytes);
        match (&wasm, &lifted) {
            (Ok(w), Some(Ok(l))) => {
                assert_eq!(w, l, "bytes {bytes:02x?}: the direct wasm calls {w}, the lifted C calls {l}");
                same += 1;
                helpers_seen.insert(w.split('(').next().unwrap().to_string());
            }
            (Ok(w), None) | (Ok(w), Some(Err(_))) => {
                // FWAIT: the lifted C emits nothing for it
                assert!(w.is_empty() && bytes == [0x9b], "bytes {bytes:02x?}: the direct wasm takes {w:?} but the lifted C has no x87 call for it ({lifted:?})");
                same += 1;
            }
            (Err(_), Some(Ok(l))) => {
                let name = l.split('(').next().unwrap().to_string();
                assert!(ONLY_LIFTED.contains(&name.as_str()), "bytes {bytes:02x?}: only the lifted C takes this ({l}) and it is not in the list of the forms left to the C path");
                only_lifted.insert(name);
            }
            (Err(_), _) => neither += 1,
        }
    }
    // every helper of the table is exercised, every form listed as left to the C path exists, and the enumeration is not empty
    let all: BTreeSet<String> = helper_names().into_iter().map(String::from).collect();
    assert_eq!(helpers_seen, all, "every helper is the answer for at least one encoding");
    assert_eq!(only_lifted.iter().map(String::as_str).collect::<Vec<_>>(), { let mut o = ONLY_LIFTED.to_vec(); o.sort(); o });
    eprintln!("x87 encodings: {} enumerated, {same} identical to the lifted C (WAIT included), {} left to the C path ({:?}), {neither} taken by neither", encodings().len(), only_lifted.len(), only_lifted);
    assert!(same > 150, "compared {same} encodings");
    assert!(neither > 100, "the encodings that neither takes: {neither}");
}

// ---------------------------------------------------------------------------------------------------------------- generated code, under node

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

const JS: &str = r#"
const fs = require('fs');
const [wasmF, casesF, outF] = process.argv.slice(2);
const mem = new WebAssembly.Memory({ initial: 16, maximum: 32768, shared: true });
const dv = new DataView(mem.buffer);
const m = new WebAssembly.Module(fs.readFileSync(wasmF));
const R = 0x2000;
let log = [];
const imports = { c: { memory: mem, __tls_base: new WebAssembly.Global({ value: 'i32', mutable: true }, 0) } };
const names = [];
let swReturn = 0;
for (const i of WebAssembly.Module.imports(m)) {
  if (i.kind !== 'function') continue;
  names.push(i.name);
  imports[i.module] = imports[i.module] || {};
  imports[i.module][i.name] = (...a) => {
    if (!i.name.startsWith('fle_x87_')) throw new Error('unexpected call of ' + i.module + '.' + i.name);
    log.push([i.name, ...a.map((x) => Number(x))]);
    return i.name === 'fle_x87_sw' ? swReturn : undefined;
  };
}
const inst = new WebAssembly.Instance(m, imports);
const cases = JSON.parse(fs.readFileSync(casesF, 'utf8'));
const out = [];
for (const c of cases) {
  for (let r = 0; r < 16; r++) dv.setBigUint64(R + 8 * r, BigInt(c.regs && c.regs[r] !== undefined ? c.regs[r] : '0'), true);
  dv.setBigUint64(R + 8 * 4, 0x80000n, true);
  swReturn = c.sw >>> 0;
  log = [];
  inst.exports.f_401000();
  const regs = []; for (let r = 0; r < 16; r++) regs.push(dv.getBigUint64(R + 8 * r, true).toString());
  out.push({ regs, log });
}
fs.writeFileSync(outF, JSON.stringify({ names, out }));
"#;

fn have(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn run_node(wasm: &[u8], cases: &str, tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-x87-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.wasm"), wasm).unwrap();
    std::fs::write(dir.join("cases.json"), cases).unwrap();
    std::fs::write(dir.join("run.cjs"), JS).unwrap();
    let o = Command::new("node").arg(dir.join("run.cjs")).arg(dir.join("t.wasm")).arg(dir.join("cases.json")).arg(dir.join("out.json")).output().unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    let txt = std::fs::read_to_string(dir.join("out.json")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    txt
}

fn wasm_of(code: &[u8]) -> Vec<u8> {
    let comp = compile(&pe(code), &[], &[], &conf()).unwrap_or_else(|e| panic!("compile: {e}"));
    assert!(comp.fidx.contains_key(&0x401000), "the function was not lowered (it stays on the C path): {code:02x?}");
    comp.wasm
}

#[test]
fn the_effective_address_and_the_arguments_reach_the_helpers_in_order_and_fnstsw_ax_merges_into_rax() {
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    // fld qword [rdi+rsi*8+0x10]; fstp dword [rdi+0x20]; fmul st0,st(3); faddp st(2),st0; fild word [rdi]; fnstsw ax; fnstsw word [rdi+0x30]; wait; fld1; ret
    let code: Vec<u8> = vec![
        0xdd, 0x44, 0xf7, 0x10, // fld qword [rdi+rsi*8+0x10]
        0xd9, 0x5f, 0x20, // fstp dword [rdi+0x20]
        0xd8, 0xcb, // fmul st0, st(3)
        0xde, 0xc2, // faddp st(2), st0
        0xdf, 0x07, // fild word [rdi]
        0xdf, 0xe0, // fnstsw ax
        0xdd, 0x7f, 0x30, // fnstsw word [rdi+0x30]
        0x9b, // wait
        0xd9, 0xe8, // fld1
        0xc3,
    ];
    let wasm = wasm_of(&code);
    // regs: rax = 1, rdi = 0x40000, rsi = 3
    let case = |sw: u32| format!("{{\"regs\":{{\"0\":\"1234605616436508552\",\"6\":\"3\",\"7\":\"262144\"}},\"sw\":{sw}}}");
    let txt = run_node(&wasm, &format!("[{},{}]", case(0x3a80), case(0xdead_0001)), "order");
    // the recorded calls of the first case, in order
    let want = "[[\"fle_x87_fld_m\",262184,8],[\"fle_x87_fst_m\",262176,4,1],[\"fle_x87_arith_st\",3,0,3,0],[\"fle_x87_arith_st\",0,2,0,1],[\"fle_x87_fild\",262144,2],[\"fle_x87_sw\"],[\"fle_x87_store_sw\",262192],[\"fle_x87_fld_const\",0]]";
    assert!(txt.contains(want), "calls differ:\n{txt}\nwant {want}");
    // FNSTSW AX: the low 16 bits of RAX are the status word (garbage in the helper's upper bits is cut), the rest of RAX is kept
    // rax = 0x1122334455667788 -> 0x112233445566_3a80, and with a helper that returns 0xdead0001 -> 0x112233445566_0001
    assert!(txt.contains("\"1234605616436492928\""), "rax after case 1: {txt}");
    assert!(txt.contains("\"1234605616436477953\""), "rax after case 2: {txt}");
}

#[test]
fn a_module_imports_only_the_helpers_its_instructions_call_and_a_function_with_a_form_left_to_the_c_path_is_not_lowered() {
    // fld1; fstp qword [rdi]; ret   -> imports fld_const and fst_m only
    let code: Vec<u8> = vec![0xd9, 0xe8, 0xdd, 0x1f, 0xc3];
    let wasm = wasm_of(&code);
    let mut names: Vec<String> = Vec::new();
    // the import section lists them; look for the names in the binary
    for h in helper_names() {
        if wasm.windows(h.len()).any(|w| w == h.as_bytes()) {
            names.push(h.to_string());
        }
    }
    names.sort();
    assert_eq!(names, vec!["fle_x87_fld_const".to_string(), "fle_x87_fst_m".to_string()]);
    // a module without x87 imports none of them
    let plain: Vec<u8> = vec![0x48, 0x89, 0xf8, 0xc3]; // mov rax, rdi; ret
    let w2 = wasm_of(&plain);
    assert!(!helper_names().iter().any(|h| w2.windows(h.len()).any(|w| w == h.as_bytes())), "a module without x87 imports no x87 helper");
    // fucomi st0, st(1); ret: the flags are written by the helper's result, which the direct wasm does not do: the function stays on the C path
    let fcomi: Vec<u8> = vec![0xdb, 0xe9, 0xc3];
    let comp = compile(&pe(&fcomi), &[], &[], &conf()).unwrap();
    assert!(!comp.fidx.contains_key(&0x401000), "a function with FUCOMI stays on the C path");
}
