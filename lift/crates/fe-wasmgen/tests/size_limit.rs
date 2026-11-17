//! Decision 0185: the size limit of the dry-run wasm body of a function (Config::max_fn_bytes / max_fn_locals / max_fn_cost = bytes x locals). A function over the limit keeps the status
//! "over the size limit (hybrid): B wasm bytes, L locals" in the report and is not compiled (in the hybrid it stays on the C path); without the variables nothing changes. V8's Liftoff needs a Zone (malloc) of roughly
//! (wasm bytes x locals) bytes to compile a function, which for the engine's 5 largest functions was 0.1 to 1.5 GB each (decision 0185).
use fe_wasmgen::{compile, Config};
use std::collections::HashMap;

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



fn status(data: &[u8], bytes: u64, locals: u64, cost: u64) -> String {
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
        max_fn_bytes: bytes,
        max_fn_locals: locals,
        max_fn_cost: cost,
        ..Config::default()
    };
    let c = compile(data, &[0x401000], &[], &conf).expect("compile");
    c.report.rows.iter().find(|r| r.va == 0x401000).expect("the function is in the report").status.clone()
}

#[test]
fn a_function_over_the_size_limit_stays_on_the_c_path() {
    // mov rax,rcx; add rax,rdx; ret
    let code = [0x48, 0x89, 0xc8, 0x48, 0x01, 0xd0, 0xc3];
    let data = pe(&code);
    let big = 100000;
    assert_eq!(status(&data, u64::MAX, u64::MAX, fe_wasmgen::MAX_FN_COST), "ok", "the built-in limit holds a small function");
    assert_eq!(status(&data, big, big, 1_000_000_000), "ok", "limits above the function change nothing");
    assert!(status(&data, 1, big, 1_000_000_000).starts_with("over the size limit (hybrid): "), "bytes");
    assert!(status(&data, big, 0, 1_000_000_000).starts_with("over the size limit (hybrid): "), "locals");
    assert!(status(&data, big, big, 1).starts_with("over the size limit (hybrid): "), "cost");
}
