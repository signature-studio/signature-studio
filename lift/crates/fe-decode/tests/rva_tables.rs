//! The RVA tables of `--rva-tables` (Issue #1395): the rows of a table file (`rva_tables`) and the code addresses that the entries of the
//! tables hold (`rva_table_seeds`). Synthetic PE32+ images (`msvc_common`); no real DLL is needed.

mod msvc_common;

use fe_decode::relocseeds::{rva_table_seeds, rva_tables};
use fe_pe::Pe;
use msvc_common::*;

#[test]
fn the_rows_are_hex_with_or_without_0x_and_the_comments_and_blank_lines_are_skipped() {
    let rows = rva_tables("# the switch tables of the module\n\n0x2000 4\n3010 12\n\n  0x3020   1  \n").unwrap();
    assert_eq!(rows, vec![(0x2000, 4), (0x3010, 12), (0x3020, 1)]);
}

#[test]
fn a_row_that_is_not_hex_or_has_no_count_or_has_more_fields_is_an_error() {
    assert!(rva_tables("zz 4\n").is_err());
    assert!(rva_tables("0x2000\n").is_err());
    assert!(rva_tables("0x2000 n\n").is_err());
    assert!(rva_tables("0x2000 -4\n").is_err());
    assert!(rva_tables("0x2000 4 extra\n").is_err());
}

#[test]
fn the_entries_that_are_code_are_the_seeds_ascending_and_once() {
    let img = Img {
        text: vec![(TEXT_RVA, vec![0xc3; 0x80])],
        rdata: vec![(RDATA_RVA, u32s(&[0x1040, 0x1010, 0x1040, 0x5000, 0xfff, 0x1020]))],
        ..Default::default()
    }
    .build();
    let pe = Pe::parse(&img).unwrap();
    // 0x5000 is in .pdata (no MEM_EXECUTE), 0xfff is in the header: neither is a seed
    assert_eq!(rva_table_seeds(&pe, &[(RDATA_RVA, 6)]).unwrap(), vec![IB + 0x1010, IB + 0x1020, IB + 0x1040]);
    assert!(rva_table_seeds(&pe, &[(0x9000, 4)]).is_err(), "a table that is not in the file");
}
