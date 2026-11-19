//! `call [IAT slot]` in lift/out-ext/dsp_ippv2_x64.dll, by API name (decision 0077).
//!
//! The DLL is not in the repository (lift/out-ext is ignored): the test returns early when it is absent.
//! It counts the `call` sites whose target fe-decode resolved to an import, per `dll!name`, and checks them
//! against the generator: every site with a response row is `call_imp:<API>` and no longer unsupported; the
//! others are the "indirect through IAT slot (not lifted)" count. Run with `--nocapture` for the table.

use fe_decode::Resolved;
use iced_x86::FlowControl;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn dsp_ippv2_iat_calls_by_api_name() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-ext/dsp_ippv2_x64.dll");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skipped: {} not present", path.display());
        return;
    };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let seeds = fe_decode::pdata_and_adjustor_seeds(&pe).unwrap();
    let a = fe_decode::analyze_with_seeds(&pe, &seeds).unwrap();

    let mut sites: BTreeMap<String, usize> = BTreeMap::new();
    for (va, i) in &a.insns {
        if i.flow_control() == FlowControl::IndirectCall {
            if let Some(Resolved::Import { name }) = a.resolved.get(va) {
                *sites.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }
    let total: usize = sites.values().sum();

    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut std::io::sink()).unwrap();
    let mut answered_total = 0;
    println!("{:<44} {:>6} {:>9} {:>11}", "API", "sites", "call_imp", "unanswered");
    for (name, n) in &sites {
        let api = name.rsplit_once('!').unwrap().1;
        let answered = st.handled.get(format!("call_imp:{api}").as_str()).copied().unwrap_or(0);
        answered_total += answered;
        println!("{name:<44} {n:>6} {answered:>9} {:>11}", n - answered);
        assert!(answered == 0 || answered == *n, "{name}: {answered} of {n}");
    }
    let unanswered = st.unsupported.get("Call: indirect through IAT slot (not lifted)").copied().unwrap_or(0);
    println!("{:<44} {total:>6} {answered_total:>9} {unanswered:>11}", "total");

    assert_eq!(total, 138);
    assert_eq!(answered_total + unanswered, total);
}
