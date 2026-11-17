//! A small WebAssembly binary encoder: the sections the generator needs (type, import, function, table, memory, global,
//! export, element, code, data), LEB128, and an instruction-byte builder. No dependency; the encoder is about 300 lines.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Ty {
    I32,
    I64,
    V128,
    F64,
}

impl Ty {
    pub fn byte(self) -> u8 {
        match self {
            Ty::I32 => 0x7f,
            Ty::I64 => 0x7e,
            Ty::V128 => 0x7b,
            Ty::F64 => 0x7c,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct FuncType {
    pub params: Vec<Ty>,
    pub results: Vec<Ty>,
}

pub fn uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

pub fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        let done = (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0);
        if done {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn name(out: &mut Vec<u8>, s: &str) {
    uleb(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Instruction bytes of one function body (without the locals header).
#[derive(Default, Clone)]
pub struct Code {
    pub b: Vec<u8>,
}

use std::cell::Cell;
/// Decision 0234 (mt3, B-prime stage 1; the only form since decision 0243 layer e): the register file R/FL/X of the C path is the thread-local block `fle_ctx` of the C part. TLS_ON != 0 (the module has a C part: `hybrid_r`, with `tls_ctx`) means every access to the
/// register page (addresses TLS_RB .. TLS_RB + 0x300) is `global.get <the imported __tls_base>` + a memarg offset (address - TLS_RB + TLS_OFF). NIMPG = the number of imported
/// globals (1 with it): the globals this module defines come after them. Off (all 0): the module has no C part and no register page.
// (thread-local: a compile runs on one thread, and the tests compile on many at once.) TLS_LOCAL: the local that the function being lowered keeps `__tls_base` in (u32::MAX = none: the global is read).
thread_local! {
    pub static TLS_ON: Cell<u32> = Cell::new(0);
    pub static TLS_RB: Cell<u32> = Cell::new(0);
    pub static TLS_OFF: Cell<u32> = Cell::new(0);
    pub static NIMPG: Cell<u32> = Cell::new(0);
    pub static TLS_LOCAL: Cell<u32> = Cell::new(u32::MAX);
}
fn nimpg() -> u32 {
    NIMPG.with(|c| c.get())
}

macro_rules! simple {
    ($($name:ident = $op:expr),* $(,)?) => {
        $(pub fn $name(&mut self) { self.b.push($op); })*
    };
}

impl Code {
    pub fn new() -> Self {
        Code { b: Vec::new() }
    }
    pub fn len(&self) -> usize {
        self.b.len()
    }
    pub fn global_get(&mut self, i: u32) {
        self.b.push(0x23);
        uleb(&mut self.b, (i + nimpg()) as u64);
    }
    pub fn global_set(&mut self, i: u32) {
        self.b.push(0x24);
        uleb(&mut self.b, (i + nimpg()) as u64);
    }
    /// `global.get` of the imported `__tls_base` (global index 0)
    pub fn tls_base_get(&mut self) {
        self.b.push(0x23);
        uleb(&mut self.b, 0);
    }
    /// Pushes the base operand for an access to the register page at address `a` and returns the memarg offset of the load/store that follows it: `global.get <__tls_base>` (or the local that keeps it) and `a - TLS_RB + TLS_OFF`.
    pub fn reg_base(&mut self, a: u32) -> u32 {
        let l = TLS_LOCAL.with(|c| c.get());
        if l == u32::MAX {
            self.tls_base_get();
        } else {
            self.local_get(l);
        }
        a - TLS_RB.with(|c| c.get()) + TLS_OFF.with(|c| c.get())
    }
    pub fn local_get(&mut self, i: u32) {
        self.b.push(0x20);
        uleb(&mut self.b, i as u64);
    }
    pub fn local_set(&mut self, i: u32) {
        self.b.push(0x21);
        uleb(&mut self.b, i as u64);
    }
    pub fn local_tee(&mut self, i: u32) {
        self.b.push(0x22);
        uleb(&mut self.b, i as u64);
    }
    pub fn f64_const(&mut self, v: f64) {
        self.b.push(0x44);
        self.b.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i32_const(&mut self, v: i32) {
        self.b.push(0x41);
        sleb(&mut self.b, v as i64);
    }
    pub fn i64_const(&mut self, v: i64) {
        self.b.push(0x42);
        sleb(&mut self.b, v);
    }
    pub fn call(&mut self, f: u32) {
        self.b.push(0x10);
        uleb(&mut self.b, f as u64);
    }
    pub fn call_indirect(&mut self, ty: u32, table: u32) {
        self.b.push(0x11);
        uleb(&mut self.b, ty as u64);
        uleb(&mut self.b, table as u64);
    }
    pub fn br(&mut self, d: u32) {
        self.b.push(0x0c);
        uleb(&mut self.b, d as u64);
    }
    pub fn br_if(&mut self, d: u32) {
        self.b.push(0x0d);
        uleb(&mut self.b, d as u64);
    }
    pub fn br_table(&mut self, targets: &[u32], default: u32) {
        self.b.push(0x0e);
        uleb(&mut self.b, targets.len() as u64);
        for &t in targets {
            uleb(&mut self.b, t as u64);
        }
        uleb(&mut self.b, default as u64);
    }
    /// blocktype: None = empty (0x40)
    pub fn block(&mut self) {
        self.b.extend_from_slice(&[0x02, 0x40]);
    }
    pub fn loop_(&mut self) {
        self.b.extend_from_slice(&[0x03, 0x40]);
    }
    pub fn if_(&mut self) {
        self.b.extend_from_slice(&[0x04, 0x40]);
    }
    pub fn if_ty(&mut self, t: Ty) {
        self.b.extend_from_slice(&[0x04, t.byte()]);
    }
    simple! {
        else_ = 0x05, end = 0x0b, return_ = 0x0f, unreachable = 0x00, nop = 0x01, drop = 0x1a, select = 0x1b,
        i32_eqz = 0x45, i32_eq = 0x46, i32_ne = 0x47, i32_lt_s = 0x48, i32_lt_u = 0x49, i32_gt_s = 0x4a, i32_gt_u = 0x4b,
        i32_le_s = 0x4c, i32_le_u = 0x4d, i32_ge_s = 0x4e, i32_ge_u = 0x4f,
        i64_eqz = 0x50, i64_eq = 0x51, i64_ne = 0x52, i64_lt_s = 0x53, i64_lt_u = 0x54, i64_gt_s = 0x55, i64_gt_u = 0x56,
        i64_le_s = 0x57, i64_le_u = 0x58, i64_ge_s = 0x59, i64_ge_u = 0x5a,
        i32_clz = 0x67, i32_ctz = 0x68, i32_popcnt = 0x69,
        i32_add = 0x6a, i32_sub = 0x6b, i32_mul = 0x6c, i32_div_s = 0x6d, i32_div_u = 0x6e, i32_rem_s = 0x6f, i32_rem_u = 0x70,
        i32_and = 0x71, i32_or = 0x72, i32_xor = 0x73, i32_shl = 0x74, i32_shr_s = 0x75, i32_shr_u = 0x76, i32_rotl = 0x77, i32_rotr = 0x78,
        i64_clz = 0x79, i64_ctz = 0x7a, i64_popcnt = 0x7b,
        i64_add = 0x7c, i64_sub = 0x7d, i64_mul = 0x7e, i64_div_s = 0x7f, i64_div_u = 0x80, i64_rem_s = 0x81, i64_rem_u = 0x82,
        i64_and = 0x83, i64_or = 0x84, i64_xor = 0x85, i64_shl = 0x86, i64_shr_s = 0x87, i64_shr_u = 0x88, i64_rotl = 0x89, i64_rotr = 0x8a,
        i32_wrap_i64 = 0xa7, i64_extend_i32_s = 0xac, i64_extend_i32_u = 0xad,
        i64_reinterpret_f64 = 0xbd, f64_sqrt = 0x9f, f64_convert_i32_s = 0xb7, f64_convert_i64_s = 0xb9, f32_convert_i32_s = 0xb2, f32_convert_i64_s = 0xb4, f32_demote_f64 = 0xb6, f64_promote_f32 = 0xbb,
        i32_extend8_s = 0xc0, i32_extend16_s = 0xc1, i64_extend8_s = 0xc2, i64_extend16_s = 0xc3, i64_extend32_s = 0xc4,
    }
    fn mem(&mut self, op: u8, align: u32, off: u32) {
        self.b.push(op);
        uleb(&mut self.b, align as u64);
        uleb(&mut self.b, off as u64);
    }
    pub fn i32_load(&mut self, off: u32) {
        self.mem(0x28, 0, off)
    }
    pub fn i64_load(&mut self, off: u32) {
        self.mem(0x29, 0, off)
    }
    pub fn i32_load8_u(&mut self, off: u32) {
        self.mem(0x2d, 0, off)
    }
    pub fn i32_load16_u(&mut self, off: u32) {
        self.mem(0x2f, 0, off)
    }
    pub fn i32_store(&mut self, off: u32) {
        self.mem(0x36, 0, off)
    }
    pub fn i64_store(&mut self, off: u32) {
        self.mem(0x37, 0, off)
    }
    pub fn i32_store8(&mut self, off: u32) {
        self.mem(0x3a, 0, off)
    }
    pub fn i32_store16(&mut self, off: u32) {
        self.mem(0x3b, 0, off)
    }
    pub fn v128_load(&mut self, off: u32) {
        self.b.extend_from_slice(&[0xfd]);
        uleb(&mut self.b, 0);
        uleb(&mut self.b, 0);
        uleb(&mut self.b, off as u64);
    }
    pub fn v128_store(&mut self, off: u32) {
        self.b.extend_from_slice(&[0xfd]);
        uleb(&mut self.b, 11);
        uleb(&mut self.b, 0);
        uleb(&mut self.b, off as u64);
    }
    pub fn v128_const(&mut self, bytes: [u8; 16]) {
        self.b.extend_from_slice(&[0xfd]);
        uleb(&mut self.b, 12);
        self.b.extend_from_slice(&bytes);
    }
    /// SIMD instruction with one lane-index immediate.
    pub fn simd_lane(&mut self, op: u32, lane: u8) {
        self.b.push(0xfd);
        uleb(&mut self.b, op as u64);
        self.b.push(lane);
    }
    /// SIMD load/store family with a memarg (align 0).
    pub fn simd_mem(&mut self, op: u32, off: u32) {
        self.b.push(0xfd);
        uleb(&mut self.b, op as u64);
        uleb(&mut self.b, 0);
        uleb(&mut self.b, off as u64);
    }
    pub fn f64_load(&mut self, off: u32) {
        self.mem(0x2b, 0, off)
    }
    pub fn f32_load(&mut self, off: u32) {
        self.mem(0x2a, 0, off)
    }
    pub fn f64_store(&mut self, off: u32) {
        self.mem(0x39, 0, off)
    }
    pub fn f32_store(&mut self, off: u32) {
        self.mem(0x38, 0, off)
    }
    pub fn simd_shuffle(&mut self, lanes: [u8; 16]) {
        self.b.push(0xfd);
        uleb(&mut self.b, 0x0d);
        self.b.extend_from_slice(&lanes);
    }
    pub fn simd(&mut self, op: u32) {
        self.b.push(0xfd);
        uleb(&mut self.b, op as u64);
    }
    pub fn i32_trunc_sat_f64_s(&mut self) {
        self.b.extend_from_slice(&[0xfc, 0x02]);
    }
    pub fn i64_trunc_sat_f64_s(&mut self) {
        self.b.extend_from_slice(&[0xfc, 0x06]);
    }
    pub fn memory_fill(&mut self) {
        self.b.extend_from_slice(&[0xfc, 0x0b, 0x00]);
    }
    pub fn memory_copy(&mut self) {
        self.b.extend_from_slice(&[0xfc, 0x0a, 0x00, 0x00]);
    }
    pub fn raw(&mut self, bytes: &[u8]) {
        self.b.extend_from_slice(bytes);
    }
}

pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

pub enum ImportKind {
    Func(u32),
    /// min pages; `shared` (decision 0234, opt-in `--mem-shared`): the import is declared shared (limits flag 0x03, a maximum is required); false for every module that was made before
    Memory { min: u32, max: Option<u32>, shared: bool },
    /// a funcref table (decision 0169: the C part's `__indirect_function_table`, for the inline dispatch); min elements
    Table { min: u32 },
    /// decision 0234 (mt3, B-prime stage 1): a mutable i32 global (the C part's `__tls_base`)
    Global,
}

pub struct Body {
    /// (count, type) runs
    pub locals: Vec<(u32, Ty)>,
    pub code: Code,
}

#[derive(Default)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub imports: Vec<Import>,
    /// type index of each defined function
    pub funcs: Vec<u32>,
    pub bodies: Vec<Body>,
    pub table_size: Option<u32>,
    pub exports: Vec<(String, u8, u32)>,
    /// element segment: (offset, function indices) into table 0
    pub elems: Vec<(u32, Vec<u32>)>,
    pub n_imported_funcs: u32,
    /// function index -> name (the `name` custom section), only when not empty
    pub names: Vec<(u32, String)>,
    /// mutable i32 globals: (init value, export name)
    pub globals: Vec<(i32, Option<String>)>,
}

impl Module {
    pub fn type_index(&mut self, t: FuncType) -> u32 {
        if let Some(p) = self.types.iter().position(|x| *x == t) {
            return p as u32;
        }
        self.types.push(t);
        (self.types.len() - 1) as u32
    }

    pub fn import_func(&mut self, module: &str, nm: &str, t: FuncType) -> u32 {
        let ti = self.type_index(t);
        self.imports.push(Import { module: module.into(), name: nm.into(), kind: ImportKind::Func(ti) });
        let n = self.n_imported_funcs;
        self.n_imported_funcs += 1;
        n
    }

    /// decision 0234 (mt3, B-prime stage 1): imports a mutable i32 global; it is global index 0 (the defined globals are shifted by NIMPG)
    pub fn import_global(&mut self, module: &str, nm: &str) {
        self.imports.push(Import { module: module.into(), name: nm.into(), kind: ImportKind::Global });
        NIMPG.with(|c| c.set(1));
    }

    pub fn import_table(&mut self, module: &str, nm: &str, min: u32) {
        self.imports.push(Import { module: module.into(), name: nm.into(), kind: ImportKind::Table { min } });
    }

    pub fn import_memory(&mut self, module: &str, nm: &str, min: u32, max: Option<u32>) {
        self.imports.push(Import { module: module.into(), name: nm.into(), kind: ImportKind::Memory { min, max, shared: false } });
    }

    /// decision 0234: a shared memory import (the C part of a build with -matomics and a shared memory); `max` is in pages and is part of the type
    pub fn import_memory_shared(&mut self, module: &str, nm: &str, min: u32, max: u32) {
        self.imports.push(Import { module: module.into(), name: nm.into(), kind: ImportKind::Memory { min, max: Some(max), shared: true } });
    }

    /// Adds a defined function; returns its function index (imports counted).
    pub fn add_func(&mut self, t: FuncType, body: Body) -> u32 {
        let ti = self.type_index(t);
        self.funcs.push(ti);
        self.bodies.push(body);
        self.n_imported_funcs + (self.funcs.len() as u32) - 1
    }

    fn section(out: &mut Vec<u8>, id: u8, payload: &[u8]) {
        out.push(id);
        uleb(out, payload.len() as u64);
        out.extend_from_slice(payload);
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        // type
        let mut p = Vec::new();
        uleb(&mut p, self.types.len() as u64);
        for t in &self.types {
            p.push(0x60);
            uleb(&mut p, t.params.len() as u64);
            for x in &t.params {
                p.push(x.byte());
            }
            uleb(&mut p, t.results.len() as u64);
            for x in &t.results {
                p.push(x.byte());
            }
        }
        Self::section(&mut out, 1, &p);
        // import
        if !self.imports.is_empty() {
            let mut p = Vec::new();
            uleb(&mut p, self.imports.len() as u64);
            for im in &self.imports {
                name(&mut p, &im.module);
                name(&mut p, &im.name);
                match &im.kind {
                    ImportKind::Func(t) => {
                        p.push(0x00);
                        uleb(&mut p, *t as u64);
                    }
                    ImportKind::Global => {
                        p.push(0x03);
                        p.push(0x7f);
                        p.push(0x01);
                    }
                    ImportKind::Table { min } => {
                        p.push(0x01);
                        p.push(0x70);
                        p.push(0x00);
                        uleb(&mut p, *min as u64);
                    }
                    ImportKind::Memory { min, max, shared } => {
                        p.push(0x02);
                        match max {
                            Some(m) => {
                                p.push(if *shared { 0x03 } else { 0x01 });
                                uleb(&mut p, *min as u64);
                                uleb(&mut p, *m as u64);
                            }
                            None => {
                                p.push(0x00);
                                uleb(&mut p, *min as u64);
                            }
                        }
                    }
                }
            }
            Self::section(&mut out, 2, &p);
        }
        // function
        let mut p = Vec::new();
        uleb(&mut p, self.funcs.len() as u64);
        for &t in &self.funcs {
            uleb(&mut p, t as u64);
        }
        Self::section(&mut out, 3, &p);
        // table
        if let Some(n) = self.table_size {
            let mut p = Vec::new();
            uleb(&mut p, 1);
            p.push(0x70);
            p.push(0x00);
            uleb(&mut p, n as u64);
            Self::section(&mut out, 4, &p);
        }
        // global
        if !self.globals.is_empty() {
            let mut p = Vec::new();
            uleb(&mut p, self.globals.len() as u64);
            for (v, _) in &self.globals {
                p.push(0x7f);
                p.push(0x01);
                p.push(0x41);
                sleb(&mut p, *v as i64);
                p.push(0x0b);
            }
            Self::section(&mut out, 6, &p);
        }
        // export
        let mut p = Vec::new();
        let gl_exports: Vec<(String, u32)> = self.globals.iter().enumerate().filter_map(|(i, (_, n))| n.clone().map(|n| (n, i as u32 + nimpg()))).collect();
        uleb(&mut p, (self.exports.len() + gl_exports.len()) as u64);
        for (n, i) in &gl_exports {
            name(&mut p, n);
            p.push(3);
            uleb(&mut p, *i as u64);
        }
        for (n, k, i) in &self.exports {
            name(&mut p, n);
            p.push(*k);
            uleb(&mut p, *i as u64);
        }
        Self::section(&mut out, 7, &p);
        // element
        if !self.elems.is_empty() {
            let mut p = Vec::new();
            uleb(&mut p, self.elems.len() as u64);
            for (off, fs) in &self.elems {
                p.push(0x00);
                p.push(0x41);
                sleb(&mut p, *off as i64);
                p.push(0x0b);
                uleb(&mut p, fs.len() as u64);
                for &f in fs {
                    uleb(&mut p, f as u64);
                }
            }
            Self::section(&mut out, 9, &p);
        }
        // code
        let mut p = Vec::new();
        uleb(&mut p, self.bodies.len() as u64);
        for b in &self.bodies {
            let mut f = Vec::new();
            uleb(&mut f, b.locals.len() as u64);
            for (n, t) in &b.locals {
                uleb(&mut f, *n as u64);
                f.push(t.byte());
            }
            f.extend_from_slice(&b.code.b);
            f.push(0x0b);
            uleb(&mut p, f.len() as u64);
            p.extend_from_slice(&f);
        }
        Self::section(&mut out, 10, &p);
        if !self.names.is_empty() {
            let mut sub = Vec::new();
            let mut v = self.names.clone();
            v.sort();
            uleb(&mut sub, v.len() as u64);
            for (i, n) in &v {
                uleb(&mut sub, *i as u64);
                name(&mut sub, n);
            }
            let mut p = Vec::new();
            name(&mut p, "name");
            p.push(1);
            uleb(&mut p, sub.len() as u64);
            p.extend_from_slice(&sub);
            Self::section(&mut out, 0, &p);
        }
        out
    }
}
