//! pdata seeds (`pdata_seeds`, `analyze_with_pdata`) on the real DLL ($FLENGINE_DLL, else
//! binaries/win64/FLEngine_x64.dll; skipped when absent, as extra_seed.rs). Expected values were measured
//! with lift/out-pdata/tool (docs/investigations/pdata-seeds.md, pdata-seeds-run.md).

use fe_decode::{analyze_with_pdata, analyze_with_seeds, pdata_seeds};
use fe_pe::Pe;

const PDATA_STARTS: usize = 57_816;
const PDATA_FUNCTIONS: usize = 58_695;
// 2_523_194 before decision 0036; +3_632 for the targets of the four annotated jump tables.
const PDATA_INSTRUCTIONS: usize = 2_526_826;
const BASE_896_FUNCTIONS: usize = 8_427;

fn load() -> Option<Vec<u8>> {
    let path = std::env::var("FLENGINE_DLL")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"));
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("skip: {} not found", path.display());
            None
        }
    }
}

#[test]
fn pdata_seeds_are_ascending_unique_text_starts() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let s = pdata_seeds(&pe).unwrap();
    assert_eq!(s.len(), PDATA_STARTS);
    assert!(s.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(s[0], 0x407380);
    assert_eq!(*s.last().unwrap(), 0x13768a0);
}

#[test]
fn pdata_closure_has_no_decode_failures_and_contains_every_start() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_pdata(&pe).unwrap();
    assert_eq!(a.closure.decode_failures, 0);
    assert_eq!(a.closure.out_of_text_flow, 0);
    assert_eq!(a.closure.functions, PDATA_FUNCTIONS);
    assert_eq!(a.closure.instructions, PDATA_INSTRUCTIONS);
    for v in pdata_seeds(&pe).unwrap() {
        assert!(a.functions.contains_key(&v), "pdata start {v:#x} is not a closure function");
    }
}

#[test]
fn pdata_closure_contains_the_default_closure_and_the_896_seed_closure() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let all = analyze_with_pdata(&pe).unwrap();
    let base = analyze_with_seeds(&pe, &[]).unwrap();
    for f in base.functions.keys() {
        assert!(all.functions.contains_key(f), "default closure function {f:#x} missing");
    }
    for v in base.insns.keys() {
        assert!(all.insns.contains_key(v), "default closure instruction {v:#x} missing");
    }
    assert!(all.closure.functions > BASE_896_FUNCTIONS);
}
