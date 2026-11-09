//! Adjustor thunks as closure roots in the generated C (decision 0051, docs/investigations/adjustor-thunks-run.md).
//! Real DLL ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; with FLENGINE_PE set and the file
//! missing the tests fail, with the default missing they are skipped). The generated C is about 200 MB: it is
//! scanned through a writer and not kept. Expected values: `fe-lift lift FILE OUT.c --pdata` with and without
//! `--no-adjustors` (lift/out-adj/gen/gen-pdata.txt, lift/out-adj/gen-noadj.txt).

use std::path::PathBuf;

const WITHOUT_FUNCTIONS: usize = 58_695;
const WITH_FUNCTIONS: usize = 59_297;
const THUNKS: usize = 602;

fn load() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

/// Finds fixed byte patterns across write boundaries without keeping the output.
struct Scan {
    tail: Vec<u8>,
    pats: Vec<Vec<u8>>,
    found: Vec<bool>,
}

impl Scan {
    fn new(pats: &[String]) -> Scan {
        Scan { tail: Vec::new(), pats: pats.iter().map(|p| p.as_bytes().to_vec()).collect(), found: vec![false; pats.len()] }
    }
}

impl std::io::Write for Scan {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tail.extend_from_slice(buf);
        for (i, p) in self.pats.iter().enumerate() {
            if !self.found[i] && self.tail.windows(p.len()).any(|w| w == p.as_slice()) {
                self.found[i] = true;
            }
        }
        let keep = self.tail.len().saturating_sub(256);
        self.tail.drain(..keep);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn adjustor_roots_add_602_functions_and_no_bad_nodes() {
    let Some(data) = load() else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let pdata = fe_decode::pdata_seeds(&pe).unwrap();
    let mut with = pdata.clone();
    with.extend(fe_decode::adjustor::adjustor_seeds(&pe).unwrap());
    assert_eq!(with.len() - pdata.len(), THUNKS);

    let pats: Vec<String> = [0x403579u64, 0x403582, 0x40358b]
        .iter()
        .flat_map(|v| [format!("void f_{v:x}(void)\n{{\n"), format!("{{ f_41c1a0(); return; }}")])
        .collect();

    let mut w0 = Scan::new(&pats);
    let st0 = fe_lift::generate_with_seeds(&data, &pdata, &mut w0).unwrap();
    let mut w1 = Scan::new(&pats);
    let st1 = fe_lift::generate_with_seeds(&data, &with, &mut w1).unwrap();

    // the outlining adds function entries of its own (decision 0243: always applied); the reference counts are those without them
    let (f0, f1) = (st0.functions - st0.outlined_heads, st1.functions - st1.outlined_heads);
    assert_eq!(f0, WITHOUT_FUNCTIONS);
    assert_eq!(f1, WITH_FUNCTIONS);
    assert_eq!(f1 - f0, THUNKS);
    assert_eq!(st0.bad_nodes, 0);
    assert_eq!(st1.bad_nodes, 0, "no undecodable node from the thunk roots");
    assert_eq!(st1.unsupported_total, st0.unsupported_total, "the thunks add no unsupported instruction");
    assert_eq!(st1.closure_insns - st0.closure_insns, 2 * THUNKS);
    assert_eq!(st1.tail_calls - st0.tail_calls, THUNKS, "each thunk is one tail call");
    assert_eq!(st1.gotos, st0.gotos);
    // the three thunks of the stop point: a function each, with the tail call into 0x41c1a0 for 0x403582
    for k in [0usize, 2, 4] {
        assert!(!w0.found[k], "function {} exists without the roots", pats[k].trim());
        assert!(w1.found[k], "function {} is generated with the roots", pats[k].trim());
    }
    assert!(w1.found[3], "f_403582 tail-calls f_41c1a0");
}

#[test]
fn dispatch_table_has_the_thunks_with_the_roots() {
    let Some(data) = load() else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let mut seeds = fe_decode::pdata_seeds(&pe).unwrap();
    seeds.extend(fe_decode::adjustor::adjustor_seeds(&pe).unwrap());
    let pats: Vec<String> = [0x403579u64, 0x403582, 0x40358b].iter().map(|v| format!("{{ UINT64_C({v:#x}), f_{v:x} }},")).collect();
    let mut w = Scan::new(&pats);
    fe_lift::generate_with_seeds(&data, &seeds, &mut w).unwrap();
    assert!(w.found.iter().all(|&f| f), "{:?}", w.found);
}
