//! Issue #1370: the function that calls the guest's setjmp in each Fruity Delphi plug-in is its entry (DllMain), and it is the one the `omit`
//! column of the plug-in's plugin.tsv names (the automatic part is `setjmp::caller_entries`, the `setjmp-entries` of fe-decode). The DLLs
//! are the installer's files (lift/out-app/host-extra/plugins, the resources task); with a file missing the check is skipped.

use fe_decode::setjmp::{caller_entries, landings};
use fe_decode::{analyze_with_pdata, Text};
use fe_pe::Pe;
use std::path::PathBuf;

fn load(rel: &str) -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-app/host-extra/plugins/Plugins/Fruity/Effects").join(rel);
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

#[test]
fn the_setjmp_caller_of_each_fruity_delphi_plugin_is_its_entry() {
    for (rel, entry_rva) in [("Fruity Balance/Fruity Balance_x64.dll", 0x2c13a0u64), ("Fruity Filter/Fruity Filter_x64.dll", 0x2cdf20)] {
        let Some(d) = load(rel) else { continue };
        let pe = Pe::parse(&d).unwrap();
        let text = Text::from_pe(&pe).unwrap();
        let a = analyze_with_pdata(&pe).unwrap();
        let l = landings(&text, &a.insns);
        assert_eq!(l.len(), 1, "{rel}: {l:?}");
        let want = pe.optional.image_base + entry_rva;
        assert_eq!(caller_entries(&a.functions, &l).into_iter().collect::<Vec<u64>>(), vec![want], "{rel}");
    }
}
