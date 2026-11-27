//! The command line of `--rva-tables FILE` and `--reloc-seeds` (Issue #1395). A table file that is missing or has a bad row stops the lift and
//! names the problem; a good table and the option are accepted. The image is the synthetic one of fe-decode/tests/msvc_common (`ret` at
//! the entry, which the table names too). The scratch folder is in the repository (`.tmp-rva-table-cli`) and is removed at the end.

#[path = "../../fe-decode/tests/msvc_common/mod.rs"]
mod msvc_common;

use msvc_common::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fe-lift")).args(args).output().expect("run fe-lift")
}

fn image() -> Vec<u8> {
    Img {
        text: vec![(TEXT_RVA, vec![0xc3; 0x10])],
        rdata: vec![(RDATA_RVA, u32s(&[TEXT_RVA]))],
        pdata: vec![(TEXT_RVA, TEXT_RVA + 1, UNW_PLAIN)],
        entry: TEXT_RVA,
        ..Default::default()
    }
    .build()
}

fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn the_rva_table_file_is_read_and_a_missing_or_bad_one_stops_the_lift() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-rva-table-cli"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pe = dir.join("image.dll");
    std::fs::write(&pe, image()).unwrap();
    let out = dir.join("out.c");

    let missing = dir.join("missing.txt");
    let o = cli(&["lift", text(&pe), text(&out), "--rva-tables", text(&missing)]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(!o.status.success(), "{err}");
    assert!(err.contains("missing.txt"), "{err}");

    let bad = dir.join("bad.txt");
    std::fs::write(&bad, "zz 4\n").unwrap();
    let o = cli(&["lift", text(&pe), text(&out), "--rva-tables", text(&bad)]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(!o.status.success(), "{err}");
    assert!(err.contains("bad table row"), "{err}");

    let good = dir.join("good.txt");
    std::fs::write(&good, "# the entry's table\n0x2000 1\n").unwrap();
    let o = cli(&["lift", text(&pe), text(&out), "--rva-tables", text(&good), "--reloc-seeds"]);
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(o.status.success(), "{err}");
    assert!(out.is_file(), "the lift writes its output");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without the options the output is the output of a run that adds no seed: a table file that names no table and `--reloc-seeds` on an
/// image without base relocations add nothing, so the two outputs are the same bytes.
#[test]
fn without_the_options_the_output_is_the_output_of_a_run_that_adds_no_seed() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-rva-table-plain"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pe = dir.join("image.dll");
    std::fs::write(&pe, image()).unwrap();
    let none = dir.join("none.txt");
    std::fs::write(&none, "# no table\n").unwrap();
    let plain = dir.join("plain.c");
    let added = dir.join("added.c");
    let o = cli(&["lift", text(&pe), text(&plain)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let o = cli(&["lift", text(&pe), text(&added), "--reloc-seeds", "--rva-tables", text(&none)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&plain).unwrap(), std::fs::read(&added).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}
