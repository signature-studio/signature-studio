//! The wasm encoder: LEB128 vectors, a hand-written module, and the structure of an encoded module.

use fe_wasmgen::wasm::{sleb, uleb, Body, Code, FuncType, Module, Ty};

fn u(v: u64) -> Vec<u8> {
    let mut o = Vec::new();
    uleb(&mut o, v);
    o
}

fn s(v: i64) -> Vec<u8> {
    let mut o = Vec::new();
    sleb(&mut o, v);
    o
}

#[test]
fn leb128_vectors() {
    assert_eq!(u(0), [0x00]);
    assert_eq!(u(127), [0x7f]);
    assert_eq!(u(128), [0x80, 0x01]);
    assert_eq!(u(624485), [0xe5, 0x8e, 0x26]);
    assert_eq!(s(0), [0x00]);
    assert_eq!(s(-1), [0x7f]);
    assert_eq!(s(63), [0x3f]);
    assert_eq!(s(64), [0xc0, 0x00]);
    assert_eq!(s(-64), [0x40]);
    assert_eq!(s(-65), [0xbf, 0x7f]);
    assert_eq!(s(-123456), [0xc0, 0xbb, 0x78]);
    assert_eq!(s(i64::MIN).len(), 10);
}

#[test]
fn encodes_a_tiny_module() {
    // (func (param i64 i64) (result i64) local.get 0 local.get 1 i64.add), exported as "add"
    let mut m = Module::default();
    let mut c = Code::new();
    c.local_get(0);
    c.local_get(1);
    c.i64_add();
    let idx = m.add_func(FuncType { params: vec![Ty::I64, Ty::I64], results: vec![Ty::I64] }, Body { locals: vec![], code: c });
    m.exports.push(("add".into(), 0, idx));
    let b = m.encode();
    assert_eq!(&b[..8], [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
    // type section: one type (params i64 i64, result i64)
    assert_eq!(&b[8..17], [0x01, 0x07, 0x01, 0x60, 0x02, 0x7e, 0x7e, 0x01, 0x7e]);
    // the code section ends the module: id, size, 1 body, body size, 0 local runs, local.get 0, local.get 1, i64.add, end
    assert_eq!(&b[b.len() - 11..], [0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x7c, 0x0b]);
}

#[test]
fn type_indices_are_shared() {
    let mut m = Module::default();
    let t = FuncType { params: vec![Ty::I32], results: vec![] };
    let a = m.type_index(t.clone());
    let b = m.type_index(t);
    let c = m.type_index(FuncType { params: vec![], results: vec![Ty::I64] });
    assert_eq!((a, b, c), (0, 0, 1));
}
