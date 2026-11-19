//! Decision 0132: the except landings of the Delphi exception handler (`fe_decode::seh::delphi_landings`) on the real FLEngine_x64.dll
//! ($FLENGINE_DLL, else binaries/win64/FLEngine_x64.dll; skipped when absent, as pdata_seeds.rs). The expected numbers are those of the
//! static reading of docs/investigations/seh-model.md (measured there with its own Python PE reader, capstone and `pdata_inv.py` /
//! `scope_census.py` / `landing_class.py`, not with this crate); the wall-16 facts are those of `sim_scopes.py`. The test re-reads the scope
//! tables of two functions with its own few lines (not through `delphi_landings`) and compares.

use fe_decode::seh::delphi_landings;
use fe_pe::Pe;

const HANDLER_RVA: u32 = 0x112c0; // VA 0x4112c0 at ImageBase 0x400000

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

fn u32at(pe: &Pe<'_>, rva: u32) -> u32 {
    let b = pe.slice_at_rva(rva, 4).unwrap();
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// The scopes {begin, end, f2, f3} (VA/RVA as stored: RVAs) of the function that starts at `fva`, read straight from the UNWIND_INFO.
fn scopes_of(pe: &Pe<'_>, fva: u64) -> Vec<(u32, u32, u32, u32)> {
    let rf = pe.runtime_functions().unwrap().into_iter().find(|r| pe.rva_to_va(r.begin_rva) == fva).expect("a RUNTIME_FUNCTION starts there");
    let ui = rf.unwind_rva & !1;
    let ncodes = pe.slice_at_rva(ui + 2, 1).unwrap()[0] as u32;
    let tail = ui + 4 + 2 * ((ncodes + 1) & !1);
    assert_eq!(u32at(pe, tail), HANDLER_RVA, "the handler RVA is stored in the slot after the unwind codes");
    let n = u32at(pe, tail + 4);
    (0..n).map(|i| (0..4).map(|j| u32at(pe, tail + 8 + 16 * i + 4 * j)).collect::<Vec<_>>()).map(|v| (v[0], v[1], v[2], v[3])).collect()
}

#[test]
fn counts_of_the_real_tables_are_the_ones_of_the_static_reading() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let l = delphi_landings(&pe, &[HANDLER_RVA]).unwrap();
    // seh-model.md section 2.1 and 2.2 (tsv/handler-summary.tsv, flengine-landing-kinds.tsv)
    assert_eq!(l.handler_functions, 10_837);
    assert_eq!(l.scopes, 16_940);
    assert_eq!(l.finally_scopes, 14_035);
    assert_eq!(l.bare_except_scopes, 2_722);
    assert_eq!(l.typed_except_scopes, 183);
    assert_eq!(l.typed_entries, 216);
    assert_eq!(l.landing_vas().len(), 2_938, "the distinct except landings");
    assert_eq!(l.by_func.len(), 2_816, "the functions that own an except landing (E+U in the reading)");
    // every landing is inside its function and none is the function start
    let rfs = pe.runtime_functions().unwrap();
    for (f, ls) in &l.by_func {
        let rf = rfs.iter().find(|r| pe.rva_to_va(r.begin_rva) == *f).unwrap();
        for &x in ls {
            assert!(*f < x && x < pe.rva_to_va(rf.end_rva), "{x:#x} not inside {f:#x}");
        }
        assert!(ls.windows(2).all(|w| w[0] < w[1]), "ascending and distinct");
    }
}

#[test]
fn a_handler_rva_that_nothing_names_gives_nothing() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    assert_eq!(delphi_landings(&pe, &[]).unwrap().by_func.len(), 0);
    let l = delphi_landings(&pe, &[0x112c4]).unwrap();
    assert_eq!((l.handler_functions, l.by_func.len(), l.scopes), (0, 0, 0));
}

#[test]
fn wall_16_frames_have_the_landings_and_funclets_of_sim_scopes() {
    let Some(data) = load() else { return };
    let pe = Pe::parse(&data).unwrap();
    let l = delphi_landings(&pe, &[HANDLER_RVA]).unwrap();
    // sim_scopes.py: raise 1 is caught by `on EOleSysError` in f_5df610 (scope [0x5df63c,0x5df672), landing 0x5df673); raise 2 is caught by `on Exception` in f_63ade0
    // (scope [0x63ae23,0x63ae4d), landing 0x63ae4e, NOT 0x63ae4c, which is a nop); f_5df610's finally funclets are 0x5df720 and 0x5df730.
    assert!(l.by_func[&0x5df610].contains(&0x5df673));
    assert!(l.by_func[&0x63ade0].contains(&0x63ae4e));
    assert!(!l.by_func[&0x63ade0].contains(&0x63ae4c));
    let s = scopes_of(&pe, 0x5df610);
    let on = s.iter().find(|x| 0x400000 + x.0 as u64 == 0x5df63c).expect("the scope that starts at 0x5df63c");
    assert_eq!(0x400000 + on.1 as u64, 0x5df672);
    assert!(on.2 > 2, "an `on` scope");
    let n = u32at(&pe, on.2);
    let lands: Vec<u64> = (0..n).map(|j| 0x400000 + u32at(&pe, on.2 + 8 + 8 * j) as u64).collect();
    assert_eq!(lands, vec![0x5df673]);
    let funclets: Vec<u64> = s.iter().filter(|x| x.2 == 0).map(|x| 0x400000 + x.3 as u64).collect();
    assert!(funclets.contains(&0x5df720) && funclets.contains(&0x5df730), "{funclets:x?}");
    let s2 = scopes_of(&pe, 0x63ade0);
    let sc = s2.iter().find(|x| 0x400000 + x.0 as u64 == 0x63ae23).expect("the scope that starts at 0x63ae23");
    assert_eq!(0x400000 + sc.1 as u64, 0x63ae4d);
}
