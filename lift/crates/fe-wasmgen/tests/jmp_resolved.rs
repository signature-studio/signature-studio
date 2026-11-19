//! Decision 0169 (a bug found by the bisection of the 'all' hybrid): a `jmp [rip+slot]` that fe-decode resolved to code (the delay-load thunk of the engine: the slot holds the address of a
//! stub that is not a function entry) is lifted by fe-lift as a plain jump to the resolved target. fe-wasmgen made it a `TailIndirect` (a lookup of the slot's runtime value in the table of
//! function entries), which aborts with "indirect call to 0x... is not a known function entry" for the stub address. The test runs with memory whose slot is zero: the plain jump reaches the
//! stub and stores 7, a lookup of the runtime value cannot. Skipped (passing) without node.
use fe_wasmgen::{compile, Config};
use std::io::Write;
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

/// A minimal PE32+ with .text (the code at RVA 0x1000) and .pdata (one function), image base 0x400000.
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

/// 0x1000: jmp [rip+0x117a] (slot at RVA 0x2180 = 0x401010, in the file range of .pdata); 0x1010: mov dword [rcx],7; ret (a stub that is not a function entry)
fn code() -> Vec<u8> {
    let mut c = vec![0xff, 0x25, 0x7a, 0x11, 0x00, 0x00];
    c.extend(std::iter::repeat(0xcc).take(10));
    c.extend([0xc7, 0x01, 0x07, 0x00, 0x00, 0x00, 0xc3]);
    c
}

#[test]
fn a_jmp_through_a_slot_that_resolves_to_code_is_a_plain_jump() {
    let mut data = pe(&code());
    w64(&mut data, 0x980, 0x401010);
    let conf = Config { export_funcs: true, ..Config::default() };
    let c = compile(&data, &[], &[], &conf).expect("compile");
    let row = c.report.rows.iter().find(|r| r.va == 0x401000).expect("the function is in the report");
    assert_eq!(row.status, "ok", "{}", row.status);
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-jmpres-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = dir.join("j.wasm");
    std::fs::write(&wasm, &c.wasm).unwrap();
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let js = dir.join("run.cjs");
    let mut f = std::fs::File::create(&js).unwrap();
    write!(
        f,
        "const fs=require('fs');const mem=new WebAssembly.Memory({{initial:16}});const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
         const i=new WebAssembly.Instance(m,{{env:{{mem}}}});i.exports.f_401000(0x80000n,0x10000n,0n,0n,0n);console.log(new DataView(mem.buffer).getUint32(0x10000,true));"
    )
    .unwrap();
    drop(f);
    let o = Command::new("node").arg(&js).arg(&wasm).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "7");
}

/// Decision 0169: an indirect call that fe-decode resolved to the entry of a function is a direct call of it, as fe-lift lifts it for the C path,
/// whatever the slot holds at run time. 0x1000: call [rip+0x117a] (slot at RVA 0x2180 = 0x401010 in the file; zero in the memory the module runs on), ret; 0x1010: mov dword [rcx],9; ret.
#[test]
fn static_calls_call_the_resolved_function_whatever_the_slot_holds() {
    let mut code = vec![0xff, 0x15, 0x7a, 0x11, 0x00, 0x00, 0xc3];
    code.extend(std::iter::repeat(0xcc).take(9));
    code.extend([0xc7, 0x01, 0x09, 0x00, 0x00, 0x00, 0xc3]);
    let mut data = pe(&code);
    w64(&mut data, 0x980, 0x401010);
    // two functions in .pdata: [0x1000, 0x1010) and [0x1010, 0x1020)
    w32(&mut data, 0x804, 0x1010);
    w32(&mut data, 0x80c, 0x1010);
    w32(&mut data, 0x810, 0x1020);
    w32(&mut data, 0x814, 0x2100);
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-statcall-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut results = Vec::new();
    {
        let conf = Config { export_funcs: true, ..Config::default() };
        let c = compile(&data, &[], &[], &conf).expect("compile");
        let wasm = dir.join("c1.wasm");
        std::fs::write(&wasm, &c.wasm).unwrap();
        let js = dir.join("run.cjs");
        std::fs::write(
            &js,
            "const fs=require('fs');const mem=new WebAssembly.Memory({initial:16});const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
             const i=new WebAssembly.Instance(m,{env:{mem}});i.exports.f_401000(0x80000n,0x10000n,0n,0n,0n);console.log(new DataView(mem.buffer).getUint32(0x10000,true));",
        )
        .unwrap();
        let o = Command::new("node").arg(&js).arg(&wasm).output().unwrap();
        results.push(if o.status.success() { String::from_utf8_lossy(&o.stdout).trim().to_string() } else { "trap".to_string() });
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(results[0], "9", "the resolved function runs; results {results:?}");
}
