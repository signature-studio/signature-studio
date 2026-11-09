//! Decision 0238 (load2, PROVISIONAL): kernel32!GetFileType (runtime/fle_vol.c), SetFilePointerEx, FlushFileBuffers and FindFirstFileExW (runtime/fle_vfs.c) for the files of the virtual volume. They stopped by name (decision 0078) and were
//! answered only by the opt-in harness wraps of decision 0206 (harness/dec1/dec1_imp.c); the statically linked CRT of oggio_x64.dll and Elastique_x64.dll reaches them (F2xH6).
//!
//! Expected values are written here from the Microsoft Learn pages (GetFileType, SetFilePointerEx, SetFilePointer, FlushFileBuffers, FindFirstFileExW, System Error Codes; saved in docs/investigations/load2/tables/contracts/), never read back
//! from the C code. A code the pages do not name is marked "inference": 6 (ERROR_INVALID_HANDLE) for an invalid handle, 5 (ERROR_ACCESS_DENIED) for a flush of a handle without write access, 131 (ERROR_NEGATIVE_SEEK, which the SetFilePointer
//! page names) for a position before the beginning. The host is Linux; nothing is compared with a real Windows.

mod apis_common;
use apis_common::*;

/// C text: a file C:\<name> created for writing (CREATE_ALWAYS) and for reading (OPEN_EXISTING), the handles in `hw` and `hr`.
fn open_file(name: &str) -> String {
    format!(
        "    uint64_t hw = CALL(fle_imp_CreateFileW, {p}, 0x40000000u, 7, 0, 2, 0x80, 0);\n    uint8_t data[10] = {{ 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 }}; uint32_t wr = 0;\n    CALL(fle_imp_WriteFile, hw, A(data), 10, A(&wr), 0);\n    uint64_t hr = CALL(fle_imp_CreateFileW, {p}, 0x80000000u, 7, 0, 3, 0x80, 0);\n",
        p = cw(&format!("C:\\{name}"))
    )
}

#[test]
fn load2_get_file_type_is_disk_for_an_open_file_and_unknown_with_an_error_for_a_handle_that_is_not() {
    // Learn: FILE_TYPE_DISK 0x0001 "The specified file is a disk file"; FILE_TYPE_UNKNOWN 0x0000 "Either the type of the specified file is unknown, or the function failed" and "If the function returned FILE_TYPE_UNKNOWN due to an error in calling
    // GetFileType, GetLastError will return the error code" (6 is an inference: the page names none)
    let body = format!(
        "{}#define WILD {}\n{}",
        open_file("t.bin"),
        cw("C:\\*"),
        r#"
    uint64_t r, f, hf;
    uint8_t fd[600];
    SETLE(5); r = CALL(fle_imp_GetFileType, hw); printf("write %llu le %u\n", U(r), LE());
    SETLE(5); r = CALL(fle_imp_GetFileType, hr); printf("read %llu le %u\n", U(r), LE());
    CALL(fle_imp_CloseHandle, hw);
    SETLE(5); r = CALL(fle_imp_GetFileType, hw); printf("closed %llu le %u\n", U(r), LE());
    SETLE(5); r = CALL(fle_imp_GetFileType, 0); printf("null %llu le %u\n", U(r), LE());
    SETLE(5); r = CALL(fle_imp_GetFileType, INVH); printf("invalid %llu le %u\n", U(r), LE());
    hf = CALL(fle_imp_FindFirstFileW, WILD, A(fd));
    SETLE(5); r = CALL(fle_imp_GetFileType, hf); printf("find %llu le %u\n", U(r), LE());
"#
    );
    let got = lines("load2-gft", &body);
    assert_eq!(got, ["write 1 le 5", "read 1 le 5", "closed 0 le 6", "null 0 le 6", "invalid 0 le 6", "find 0 le 6"]);
}

#[test]
fn load2_set_file_pointer_ex_moves_the_pointer_of_an_open_file_like_set_file_pointer() {
    // Learn: FILE_BEGIN (the distance is unsigned), FILE_CURRENT, FILE_END; "If lpNewFilePointer is NULL, the new file pointer is not returned"; a negative position fails (SetFilePointer page: ERROR_NEGATIVE_SEEK 131) and the pointer stays;
    // past the end is allowed; nonzero on success
    let body = format!(
        "{}{}",
        open_file("p.bin"),
        r#"
    uint64_t r, np = 7777;
    SETLE(5); r = CALL(fle_imp_SetFilePointerEx, hr, 4, A(&np), 0); printf("begin %llu np %llu le %u\n", U(r), U(np), LE());
    r = CALL(fle_imp_SetFilePointerEx, hr, 3, A(&np), 1); printf("current %llu np %llu\n", U(r), U(np));
    r = CALL(fle_imp_SetFilePointerEx, hr, (uint64_t)-2, A(&np), 2); printf("end %llu np %llu\n", U(r), U(np));
    r = CALL(fle_imp_SetFilePointerEx, hr, 20, A(&np), 0); printf("past %llu np %llu\n", U(r), U(np));
    np = 5555; SETLE(5); r = CALL(fle_imp_SetFilePointerEx, hr, (uint64_t)-21, A(&np), 1); printf("negative %llu np %llu le %u\n", U(r), U(np), LE());
    r = CALL(fle_imp_SetFilePointerEx, hr, 0, A(&np), 1); printf("stayed %llu np %llu\n", U(r), U(np));
    r = CALL(fle_imp_SetFilePointerEx, hr, 0, 0, 0); printf("nullnew %llu\n", U(r));
    r = CALL(fle_imp_SetFilePointerEx, hr, (uint64_t)1 << 40, A(&np), 0); printf("big %llu np %llx\n", U(r), U(np));
    CALL(fle_imp_CloseHandle, hr);
    SETLE(5); r = CALL(fle_imp_SetFilePointerEx, hr, 0, A(&np), 0); printf("closed %llu le %u\n", U(r), LE());
    SETLE(5); r = CALL(fle_imp_SetFilePointerEx, INVH, 0, A(&np), 0); printf("invalid %llu le %u\n", U(r), LE());
"#
    );
    let got = lines("load2-sfpex", &body);
    assert_eq!(
        got,
        [
            "begin 1 np 4 le 5",
            "current 1 np 7",
            "end 1 np 8",
            "past 1 np 20",
            "negative 0 np 5555 le 131",
            "stayed 1 np 20",
            "nullnew 1",
            "big 1 np 10000000000",
            "closed 0 le 6",
            "invalid 0 le 6",
        ]
    );
    // the pointer set by SetFilePointerEx is the one ReadFile uses: position 8 of the 10 bytes leaves 2
    let body = format!(
        "{}{}",
        open_file("q.bin"),
        r#"
    uint8_t b[10]; uint32_t got = 99; uint64_t r;
    CALL(fle_imp_SetFilePointerEx, hr, 8, 0, 0);
    r = CALL(fle_imp_ReadFile, hr, A(b), 10, A(&got), 0); printf("read %llu got %u b0 %u\n", U(r), got, b[0]);
"#
    );
    assert_eq!(lines("load2-sfpex-read", &body), ["read 1 got 2 b0 9"]);
    assert_aborts(
        "load2-sfpex-stop",
        &open_file("s.bin"),
        &[
            ("method-3", "CALL(fle_imp_SetFilePointerEx, hr, 0, 0, 3);"),
            ("begin-above-2-63", "CALL(fle_imp_SetFilePointerEx, hr, (uint64_t)1 << 63, 0, 0);"),
        ],
    );
}

#[test]
fn load2_flush_file_buffers_needs_a_handle_with_write_access() {
    // Learn: "The file handle must have the GENERIC_WRITE access right"; nonzero on success, zero on failure (codes: inference, 5 for a read-only handle, 6 for an invalid one)
    let body = format!(
        "{}{}",
        open_file("f.bin"),
        r#"
    uint64_t r;
    SETLE(9); r = CALL(fle_imp_FlushFileBuffers, hw); printf("write %llu le %u\n", U(r), LE());
    SETLE(9); r = CALL(fle_imp_FlushFileBuffers, hr); printf("read %llu le %u\n", U(r), LE());
    CALL(fle_imp_CloseHandle, hw);
    SETLE(9); r = CALL(fle_imp_FlushFileBuffers, hw); printf("closed %llu le %u\n", U(r), LE());
    SETLE(9); r = CALL(fle_imp_FlushFileBuffers, 0); printf("null %llu le %u\n", U(r), LE());
    SETLE(9); r = CALL(fle_imp_FlushFileBuffers, INVH); printf("invalid %llu le %u\n", U(r), LE());
"#
    );
    assert_eq!(lines("load2-ffb", &body), ["write 1 le 9", "read 0 le 5", "closed 0 le 6", "null 0 le 6", "invalid 0 le 6"]);
}

#[test]
fn load2_find_first_file_ex_is_find_first_file_for_the_plain_cases_and_stops_for_the_rest() {
    // Learn: FindExInfoStandard / FindExInfoBasic, FindExSearchNameMatch / FindExSearchLimitToDirectories (advisory, "silently ignored" when the file system cannot filter), lpSearchFilter must be NULL, FIND_FIRST_EX_LARGE_FETCH a hint.
    // No match: INVALID_HANDLE_VALUE and ERROR_FILE_NOT_FOUND (FindFirstFileW page).
    let setup = format!(
        "    uint8_t fd[600]; uint64_t h, r, n;\n    {{ uint64_t a = CALL(fle_imp_CreateFileW, {a}, 0x40000000u, 7, 0, 2, 0x80, 0); CALL(fle_imp_CloseHandle, a); }}\n    {{ uint64_t b = CALL(fle_imp_CreateFileW, {b}, 0x40000000u, 7, 0, 2, 0x80, 0); CALL(fle_imp_CloseHandle, b); }}\n",
        a = cw("C:\\a.txt"),
        b = cw("C:\\b.txt")
    );
    let count = |lbl: &str, args: &str| {
        format!(
            "    h = CALL(fle_imp_FindFirstFileExW, {pat}, {args});\n    n = h == INVH ? 0 : 1;\n    if (h != INVH) while (CALL(fle_imp_FindNextFileW, h, A(fd))) n++;\n    printf(\"{lbl} n %llu\\n\", U(n));\n",
            pat = cw("C:\\*.txt"),
        )
    };
    let mut body = setup.clone();
    for (lbl, args) in [
        ("standard", "0, A(fd), 0, 0, 0"),         // FindExInfoStandard, FindExSearchNameMatch
        ("basic-dirs-large", "1, A(fd), 1, 0, 2"), // FindExInfoBasic, FindExSearchLimitToDirectories (advisory), FIND_FIRST_EX_LARGE_FETCH
        ("standard-large", "0, A(fd), 0, 0, 2"),
    ] {
        body += &count(lbl, args);
    }
    body += &format!("    SETLE(5); h = CALL(fle_imp_FindFirstFileExW, {}, 0, A(fd), 0, 0, 0); printf(\"nomatch %d le %u\\n\", h == INVH, LE());\n", cw("C:\\*.zzz"));
    let got = lines("load2-ffex", &body);
    assert_eq!(got, ["standard n 2", "basic-dirs-large n 2", "standard-large n 2", "nomatch 1 le 2"]);
    let pat = cw("C:\\*.txt");
    assert_aborts(
        "load2-ffex-stop",
        &setup,
        &[
            ("level-2", &format!("CALL(fle_imp_FindFirstFileExW, {pat}, 2, A(fd), 0, 0, 0);")),
            ("op-2", &format!("CALL(fle_imp_FindFirstFileExW, {pat}, 0, A(fd), 2, 0, 0);")),
            ("filter", &format!("CALL(fle_imp_FindFirstFileExW, {pat}, 0, A(fd), 0, 1, 0);")),
            ("case-sensitive", &format!("CALL(fle_imp_FindFirstFileExW, {pat}, 0, A(fd), 0, 0, 1);")),
            ("on-disk-only", &format!("CALL(fle_imp_FindFirstFileExW, {pat}, 0, A(fd), 0, 0, 4);")),
        ],
    );
}
