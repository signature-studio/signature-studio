use fe_decode::indirect::dataflow::take_log;
use fe_decode::analyze;
use fe_pe::Pe;

#[test]
fn dump() {
    let img = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../binaries/win64/FLEngine_x64.dll")).ok();
    let _ = img;
    let _ = take_log();
    let _ = analyze;
    let _ = Pe::parse;
}
