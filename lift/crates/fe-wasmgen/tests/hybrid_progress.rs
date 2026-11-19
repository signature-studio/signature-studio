//! Decision 0179: `Config::hybrid_progress` (`--hybrid-progress ADDR`). A `jmp` that fe-decode resolved to the entry of an opaque function (one that fe-wasmgen cannot analyse and that is not lowered) goes back to the
//! run-time lookup of the C path (`fle_hyb_dispatch`), because the opaque function cannot be called through a stub. The C path jumps to such a target without counting an indirect call of the guest, while
//! `fle_dispatch` counts one (`fle_progress_count`, the progress meter of the cooperative threads, which the 16-snapshot comparison prints): the snapshot line g3-progress differed by +2 at the first
//! idle and by +34 afterwards on the real engine (dsp's IPP dispatch stubs `mov rax,[slot]; jmp rax`). With the address of the counter the compiled code takes the count back by one before such a dispatch.
//! The test runs the stub function with the C runtime's dispatch replaced by a JS function that does the count, and compares the count before and after. Skipped (passing) without node.
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

#[test]
fn a_demoted_resolved_jump_does_not_change_the_progress_count() {
    // 0x1000: mov rax,[rip+0x1179] (slot at RVA 0x2180 = 0x401010 in the file); jmp rax   0x1010: an undecodable byte (the function is opaque), ret
    let mut code = vec![0x48, 0x8b, 0x05, 0x79, 0x11, 0x00, 0x00, 0xff, 0xe0];
    code.extend(std::iter::repeat(0xcc).take(7));
    code.extend([0x06, 0xc3]);
    let mut data = pe(&code);
    w64(&mut data, 0x980, 0x401010);
    w32(&mut data, 0x804, 0x1010);
    w32(&mut data, 0x80c, 0x1010);
    w32(&mut data, 0x810, 0x1020);
    w32(&mut data, 0x814, 0x2100);
    w32(&mut data, 0x80 + 24 + 112 + 3 * 8 + 4, 24); // the exception directory holds two functions
    if !have("node") {
        eprintln!("SKIPPED node: not installed");
        return;
    }
    let dir = std::env::temp_dir().join(format!("fe-wasmgen-prog-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut results = Vec::new();
    for progress in [None, Some(0x3200u32)] {
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
            hybrid_progress: progress,
            ..Config::default()
        };
        let c = compile(&data, &[0x401000, 0x401010], &[], &conf).expect("compile");
        let row = c.report.rows.iter().find(|r| r.va == 0x401000).expect("the function is in the report");
        assert_eq!(row.status, "ok", "{}", row.status);
        assert_eq!(c.report.demoted_resolved_jumps, 1, "the jump to the opaque function is demoted to the lookup");
        let wasm = dir.join(format!("p{}.wasm", progress.is_some() as u8));
        std::fs::write(&wasm, &c.wasm).unwrap();
        let js = dir.join("run.cjs");
        std::fs::write(
            &js,
            "const fs=require('fs');const mem=new WebAssembly.Memory({initial:128,maximum:32768,shared:true});const dv=new DataView(mem.buffer);const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));\
             const imports={c:{memory:mem,__tls_base:new WebAssembly.Global({value:'i32',mutable:true},0)}};for(const i of WebAssembly.Module.imports(m)){if(i.kind!=='function')continue;imports[i.module]=imports[i.module]||{};imports[i.module][i.name]=()=>{throw new Error('unexpected '+i.name)};}\
             let target=0n;imports.c.fle_hyb_dispatch=(va)=>{target=va;dv.setBigUint64(0x3200,dv.getBigUint64(0x3200,true)+1n,true);};\
             const i=new WebAssembly.Instance(m,imports);dv.setBigUint64(0x3200,100n,true);dv.setBigUint64(0x402180,0x401010n,true);dv.setBigUint64(0x2000+8*4,0x80000n,true);i.exports.f_401000();\
             console.log(dv.getBigUint64(0x3200,true).toString()+' '+target.toString(16));",
        )
        .unwrap();
        let o = Command::new("node").arg(&js).arg(&wasm).output().unwrap();
        results.push(if o.status.success() { String::from_utf8_lossy(&o.stdout).trim().to_string() } else { format!("trap: {}", String::from_utf8_lossy(&o.stderr).chars().take(400).collect::<String>()) });
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(results[0], "101 401010", "without the address the dispatch counts (the behaviour of decision 0169); results {results:?}");
    assert_eq!(results[1], "100 401010", "with the address the count is taken back; results {results:?}");
}
