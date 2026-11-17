//! Extra seeds (`analyze_with_seeds`). The real-DLL checks are skipped when the
//! DLL is absent (`$FLENGINE_DLL`, else `binaries/win64/FLEngine_x64.dll`).
//! Expected counts are the measured values of the default closure (docs/investigations/closure-count.md)
//! and of the closure with 0x41d050 added (docs/investigations/extra-seed-run.md).

use fe_decode::{analyze, analyze_with_seeds};
use fe_pe::Pe;

const SEED_41D050: u64 = 0x41d050;
const DEFAULT_FUNCTIONS: usize = 6_969;
// +10 instructions since decision 0036: the targets of the f_40d210 jump table (0x40d2ba..0x40d2d0)
// enter the closure. They were 460_479 / 460_492 / 460_495 before.
const DEFAULT_INSTRUCTIONS: usize = 460_489;
const EXTRA_FUNCTIONS: usize = 6_971;
const EXTRA_INSTRUCTIONS: usize = 460_502;

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
fn default_closure_is_unchanged() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze(&pe).unwrap();
    assert_eq!(a.closure.seeds_code.len(), 3);
    assert_eq!(a.closure.functions, DEFAULT_FUNCTIONS);
    assert_eq!(a.closure.instructions, DEFAULT_INSTRUCTIONS);
    assert!(!a.functions.contains_key(&SEED_41D050));
}

#[test]
fn empty_extra_equals_analyze() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze(&pe).unwrap();
    let b = analyze_with_seeds(&pe, &[]).unwrap();
    assert_eq!(a.closure.functions, b.closure.functions);
    assert_eq!(a.closure.instructions, b.closure.instructions);
    assert_eq!(a.closure.seeds_code, b.closure.seeds_code);
    assert_eq!(a.functions, b.functions);
}

#[test]
fn extra_seed_41d050_enters_closure() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_seeds(&pe, &[SEED_41D050]).unwrap();
    assert!(a.functions.contains_key(&SEED_41D050), "0x41d050 is a function entry");
    assert_eq!(a.closure.seeds_code.len(), 4);
    assert!(a.closure.seeds_code.iter().any(|(n, v)| *v == SEED_41D050 && n == "<seed:0x41d050>"));
    assert_eq!(a.closure.functions, EXTRA_FUNCTIONS);
    assert_eq!(a.closure.instructions, EXTRA_INSTRUCTIONS);
}

#[test]
fn extra_seed_outside_text_is_not_code() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_seeds(&pe, &[0x1000]).unwrap();
    assert!(!a.functions.contains_key(&0x1000));
    assert_eq!(a.closure.functions, DEFAULT_FUNCTIONS);
}

/// Added 2026-10-20 (docs/decisions/0008-extra-seed-0x41e7e0.md). Counts are measured
/// with `seed-both closure` (lift/out-seed-both), 0x41d050 and 0x41e7e0 as extra seeds.
#[test]
fn extra_seed_41e7e0_enters_closure() {
    const SEED_41E7E0: u64 = 0x41e7e0;
    const BOTH_FUNCTIONS: usize = 6_972;
    const BOTH_INSTRUCTIONS: usize = 460_505; // 460_495 before decision 0036 (+10, see above)
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let a = analyze_with_seeds(&pe, &[SEED_41D050, SEED_41E7E0]).unwrap();
    assert!(a.functions.contains_key(&SEED_41E7E0), "0x41e7e0 is a function entry");
    assert!(a.functions.contains_key(&SEED_41D050), "0x41d050 is still a function entry");
    assert_eq!(a.closure.seeds_code.len(), 5);
    assert!(a.closure.seeds_code.iter().any(|(n, v)| *v == SEED_41E7E0 && n == "<seed:0x41e7e0>"));
    assert_eq!(a.closure.functions, BOTH_FUNCTIONS);
    assert_eq!(a.closure.instructions, BOTH_INSTRUCTIONS);
}
