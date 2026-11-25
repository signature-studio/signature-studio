//! Issue #1418: calls that the runtime does not model answer with the Windows failure value and one warning line per API
//! (stderr, "fle: warn: <API>: not modelled, returned <value> (caller ...)"), instead of stopping the run. The counts of the
//! warned calls are printed on stderr as "fle-warns: {...}" (per API, in the order they were first warned) before a stop.
//! Expected values come from the Microsoft pages and from the Wine sources named in the comments; nothing is compared with
//! a real Windows.

mod fsx_common;
use fsx_common::*;

#[test]
fn version_info_of_a_name_without_a_directory_fails_with_file_not_found() {
    // dlls/kernelbase/version.c (Wine, GetFileVersionInfoSizeExW): a name that LoadLibraryExW does not find is opened as a file
    // in the current directory; when that fails the answer is 0 with the file's error (ERROR_FILE_NOT_FOUND, 2). The page
    // (GetFileVersionInfoSizeW) says "zero on failure" and names no code; the code is the one of the file that is not there.
    // The LoadLibrary search sequence is not modelled, so the warning line is printed once per API.
    let body = r#"
    uint32_t hd = 9;
    static uint8_t buf[64];
    printf("size %llu %u\n", U(CALL(fle_imp_GetFileVersionInfoSizeW, W("ILSvg_x64.dll"), A(&hd), 0, 0)), LE());
    printf("handle %u\n", hd);
    printf("size-again %llu %u\n", U(CALL(fle_imp_GetFileVersionInfoSizeW, W("ILSvg_x64.dll"), 0, 0, 0)), LE());
    printf("info %llu %u\n", U(CALL(fle_imp_GetFileVersionInfoW, W("ILSvg_x64.dll"), 0, 16, A(buf))), LE());
    printf("buf %u\n", buf[0]);
"#;
    let o = run("version-no-dir", body);
    assert!(o.ok, "did not exit normally: {}\n{}", o.stderr, o.stdout);
    assert_eq!(
        o.stdout.lines().collect::<Vec<_>>(),
        ["size 0 2", "handle 0", "size-again 0 2", "info 0 2", "buf 0"]
    );
    let warns: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("fle: warn:")).collect();
    assert_eq!(
        warns,
        [
            "fle: warn: GetFileVersionInfoSizeW: not modelled, returned 0 (caller 0xdead0000)",
            "fle: warn: GetFileVersionInfoW: not modelled, returned 0 (caller 0xdead0000)",
        ]
    );
}

#[test]
fn warned_call_counts_are_printed_before_a_stop() {
    // The beacon's warns (app/src/perf/warns.ts) read the last "fle-warns:" line of the engine's log. The line lists every API
    // that was warned about, with its number of calls, and comes before the stop line that ends the run.
    let body = r#"
    uint32_t hd = 0;
    uint64_t hk = 0;
    CALL(fle_imp_GetFileVersionInfoSizeW, W("ILSvg_x64.dll"), A(&hd), 0, 0);
    CALL(fle_imp_GetFileVersionInfoSizeW, W("ILSvg_x64.dll"), A(&hd), 0, 0);
    CALL(fle_imp_GetFileVersionInfoW, W("ILSvg_x64.dll"), 0, 16, 0);
    CALL(fle_imp_RegOpenKeyExW, 0x1234, W("T"), 0, 0x20019, A(&hk));
    printf("returned\n");
"#;
    let o = run("warn-counts-stop", body);
    assert!(!o.ok, "expected a stop, got stdout {}", o.stdout);
    assert!(!o.stdout.contains("returned"), "{}", o.stdout);
    let counts = o.stderr.lines().position(|l| l == "fle-warns: {\"GetFileVersionInfoSizeW\":2,\"GetFileVersionInfoW\":1}");
    let stop = o.stderr.lines().position(|l| l.starts_with("fle: RegOpenKeyExW: hKey is neither"));
    assert!(counts.is_some(), "no counts line: {}", o.stderr);
    assert!(stop.is_some() && counts < stop, "the counts line comes before the stop line: {}", o.stderr);
}
