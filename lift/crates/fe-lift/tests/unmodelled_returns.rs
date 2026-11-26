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
    // #1415: the name is searched in the LoadLibrary order (fle_load.c); none of its folders holds the file, so the answer is
    // the failure above and the volume's log line (missing-path), with no warning.
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
    assert!(warns.is_empty(), "{warns:?}");
    let missing: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("missing-path:")).collect();
    assert_eq!(
        missing,
        [
            "missing-path: GetFileVersionInfoSizeW 2 \"ILSvg_x64.dll\"",
            "missing-path: GetFileVersionInfoSizeW 2 \"ILSvg_x64.dll\"",
            "missing-path: GetFileVersionInfoW 2 \"ILSvg_x64.dll\"",
        ]
    );
}

#[test]
fn warned_call_counts_are_printed_before_a_stop() {
    // The beacon's warns (app/src/perf/warns.ts) read the last "fle-warns:" line of the engine's log. The line lists every API
    // that was warned about, with its number of calls, and comes before the stop line that ends the run. RegSetValueExW with
    // NULL data and a size warns (Wine's advapi32 tests, ERROR_NOACCESS); the version calls of a bare name no longer warn (#1415).
    let body = r#"
    uint64_t hk = 0;
    CALL(fle_imp_RegSetValueExW, 0x1234, W("T"), 0, 1, 0, 4);
    CALL(fle_imp_RegSetValueExW, 0x1234, W("T"), 0, 1, 0, 4);
    CALL(fle_imp_RegOpenKeyExW, 0x1234, W("T"), 0, 0x20019, A(&hk));
    printf("returned\n");
"#;
    let o = run("warn-counts-stop", body);
    assert!(!o.ok, "expected a stop, got stdout {}", o.stdout);
    assert!(!o.stdout.contains("returned"), "{}", o.stdout);
    let counts = o.stderr.lines().position(|l| l == "fle-warns: {\"RegSetValueExW\":2}");
    let stop = o.stderr.lines().position(|l| l.starts_with("fle: RegOpenKeyExW: hKey is neither"));
    assert!(counts.is_some(), "no counts line: {}", o.stderr);
    assert!(stop.is_some() && counts < stop, "the counts line comes before the stop line: {}", o.stderr);
}
