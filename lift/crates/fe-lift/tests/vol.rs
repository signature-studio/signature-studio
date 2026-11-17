//! Tests for GetStdHandle, GetDriveTypeW, GetLogicalDrives, GetLogicalDriveStringsW, GetVolumeInformationW,
//! QueryDosDeviceW and DuplicateHandle in runtime/fle_vol.c (docs/decisions/0024-apis-a3-a5-a13.md; sources:
//! docs/investigations/apis-a3-a5-a13-run.md).
//!
//! Expected values are written here from the rules on the Microsoft pages (GetStdHandle, GetDriveTypeW,
//! GetLogicalDrives, GetLogicalDriveStringsW, GetVolumeInformationW, QueryDosDeviceW, DuplicateHandle, System Error
//! Codes), not read back from the C code. A value the pages do not give is marked "assumption", "inference" or "choice"
//! and is the only thing such a test pins down. The host is Linux: nothing is compared with a real Windows; no host
//! drive, volume or handle is read.

mod apis_common;
use apis_common::*;

const A5: [&str; 7] = ["GetStdHandle", "GetDriveTypeW", "GetLogicalDrives", "GetLogicalDriveStringsW", "GetVolumeInformationW", "QueryDosDeviceW", "DuplicateHandle"];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_vol.h")).unwrap();
    for n in A5 {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_vol.h");
    }
}

#[test]
fn handle_regions_do_not_overlap_anything_else() {
    let vol = header_const("fle_vol.h", "FLE_VOL_HANDLE_BASE");
    let vsz = header_const("fle_vol.h", "FLE_VOL_REGION_SIZE");
    let map = header_const("fle_vmap.h", "FLE_VMAP_HANDLE_BASE");
    let msz = header_const("fle_vmap.h", "FLE_VMAP_REGION_SIZE");
    let glob = header_const("fle_glob.h", "FLE_GLOBAL_HANDLE_BASE");
    let gsz = header_const("fle_glob.h", "FLE_GLOBAL_REGION_SIZE");
    let fsz = header_const("fle_vfs.h", "FLE_HANDLE_REGION_SIZE");
    let mut regions: Vec<(String, u64, u64)> = vec![("vol".into(), vol, vsz), ("vmap".into(), map, msz), ("glob".into(), glob, gsz)];
    for n in ["FLE_VFS_FILE_BASE", "FLE_VFS_FIND_BASE", "FLE_REG_KEY_BASE"] {
        regions.push((n.into(), header_const("fle_vfs.h", n), fsz));
    }
    let heap = 0x7FF7_0000_0000u64;
    let mods_end = 0x7FF8_0000_0000u64 + 0x10000 * 34;
    let resp_end = 0x7FF9_0000_0000u64 + 0x10 * 5000;
    for (n, b, s) in &regions {
        assert!(*b > heap + 0x1000_0000 && *b > mods_end && *b > resp_end, "{n} below another token region");
        assert!(b + s < (1u64 << 47), "{n} outside user address space");
    }
    for i in 0..regions.len() {
        for j in i + 1..regions.len() {
            let (a, b) = (&regions[i], &regions[j]);
            assert!(a.1 + a.2 <= b.1 || b.1 + b.2 <= a.1, "{} and {} overlap", a.0, b.0);
        }
    }
}

// ---- GetStdHandle ----

#[test]
fn get_std_handle_returns_null_for_the_three_valid_values() {
    // page: STD_INPUT_HANDLE (DWORD)-10, STD_OUTPUT_HANDLE (DWORD)-11, STD_ERROR_HANDLE (DWORD)-12; with no associated
    // standard handles and none redirected the return value is NULL (the process has none: decision 0024).
    let l = lines(
        "vol-stdh",
        r#"
    uint64_t r;
    SETLE(41);
    r = CALL(fle_imp_GetStdHandle, 0xFFFFFFF6u); printf("in %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GetStdHandle, 0xFFFFFFF5u); printf("out %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_GetStdHandle, 0xFFFFFFF4u); printf("err %llu le %u\n", U(r), LE());
    /* only the DWORD counts: a sign-extended -10 and garbage in the high half give the same answer */
    r = CALL(fle_imp_GetStdHandle, 0xFFFFFFFFFFFFFFF6ull); printf("in_sext %llu\n", U(r));
    r = CALL(fle_imp_GetStdHandle, 0x1234567800000000ull | 0xFFFFFFF5u); printf("out_hi %llu\n", U(r));
"#,
    );
    assert_eq!(l, ["in 0 le 41", "out 0 le 41", "err 0 le 41", "in_sext 0", "out_hi 0"]);
}

#[test]
fn get_std_handle_other_values_stop_the_run() {
    assert_aborts(
        "vol-abort-stdh",
        "",
        &[
            ("zero", "CALL(fle_imp_GetStdHandle, 0);"),
            ("minus-9", "CALL(fle_imp_GetStdHandle, 0xFFFFFFF7u);"),
            ("minus-13", "CALL(fle_imp_GetStdHandle, 0xFFFFFFF3u);"),
            ("one", "CALL(fle_imp_GetStdHandle, 1);"),
        ],
    );
}

// ---- GetDriveTypeW, GetLogicalDrives, GetLogicalDriveStringsW ----

#[test]
fn drive_type_distinguishes_the_one_volume_from_missing_roots() {
    let l = lines(
        "vol-drivetype",
        &format!(
            r#"
    printf("C %llu\n", U(CALL(fle_imp_GetDriveTypeW, {c})));
    printf("c %llu\n", U(CALL(fle_imp_GetDriveTypeW, {lc})));
    printf("null %llu\n", U(CALL(fle_imp_GetDriveTypeW, 0)));
    printf("D %llu\n", U(CALL(fle_imp_GetDriveTypeW, {d})));
    printf("A %llu\n", U(CALL(fle_imp_GetDriveTypeW, {a})));
    printf("Z %llu\n", U(CALL(fle_imp_GetDriveTypeW, {z})));
"#,
            c = cw("C:\\"),
            lc = cw("c:\\"),
            d = cw("D:\\"),
            a = cw("A:\\"),
            z = cw("Z:\\"),
        ),
    );
    // page: DRIVE_FIXED 3; DRIVE_NO_ROOT_DIR 1 "there is no volume mounted at the specified path"; NULL = root of the
    // current directory (C:). That C: is fixed is a choice.
    assert_eq!(l, ["C 3", "c 3", "null 3", "D 1", "A 1", "Z 1"]);
}

#[test]
fn drive_type_other_forms_stop_the_run() {
    assert_aborts(
        "vol-abort-drivetype",
        "",
        &[
            ("no-backslash", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw("C:"))),
            ("subdir", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw("C:\\Windows\\"))),
            ("unc", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw("\\\\srv\\share\\"))),
            ("relative", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw("abc"))),
            ("empty", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw(""))),
            ("digit", &format!("CALL(fle_imp_GetDriveTypeW, {});", cw("1:\\"))),
        ],
    );
}

#[test]
fn logical_drives_is_a_bit_mask_with_only_c() {
    let l = lines("vol-drives", "    printf(\"%llx\\n\", U(CALL(fle_imp_GetLogicalDrives, 0)));");
    // page: bit 0 is A, bit 1 is B, bit 2 is C
    assert_eq!(l, ["4"]);
}

#[test]
fn logical_drive_strings_follow_the_buffer_rules() {
    let l = lines(
        "vol-drivestrings",
        r#"
    uint16_t buf[8];
    uint64_t r;
    int i;
    for (i = 0; i < 8; i++) buf[i] = 0x7777;
    /* too small: the required size, the buffer untouched */
    r = CALL(fle_imp_GetLogicalDriveStringsW, 4, A(buf)); printf("small %llu untouched %d\n", U(r), buf[0] == 0x7777 && buf[4] == 0x7777);
    /* nBufferLength 0: lpBuffer is not used */
    r = CALL(fle_imp_GetLogicalDriveStringsW, 0, 0); printf("zero %llu\n", U(r));
    r = CALL(fle_imp_GetLogicalDriveStringsW, 0, A(buf)); printf("zero_buf %llu untouched %d\n", U(r), buf[0] == 0x7777);
    /* exactly enough */
    r = CALL(fle_imp_GetLogicalDriveStringsW, 5, A(buf));
    printf("ok %llu units %04x %04x %04x %04x %04x after %04x\n", U(r), buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]);
    /* larger buffer: same answer, nothing written past the final NUL */
    for (i = 0; i < 8; i++) buf[i] = 0x7777;
    r = CALL(fle_imp_GetLogicalDriveStringsW, 8, A(buf));
    printf("big %llu %04x %04x %04x %04x %04x %04x\n", U(r), buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]);
"#,
    );
    assert_eq!(
        l,
        [
            // page: "If the buffer is not large enough, the return value is greater than nBufferLength. It is the size
            // of the buffer required": "C:\" NUL NUL is 5 units
            "small 5 untouched 1",
            "zero 5",
            "zero_buf 5 untouched 1",
            // page: success returns the length of the strings copied "not including the terminating null character"
            // (the final additional NUL): "C:\" and its NUL are 4 units (a reading; the page gives no example)
            "ok 4 units 0043 003a 005c 0000 0000 after 7777",
            "big 4 0043 003a 005c 0000 0000 7777",
        ]
    );
}

#[test]
fn logical_drive_strings_null_buffer_with_a_size_stops_the_run() {
    assert_aborts("vol-abort-drivestrings", "", &[("null-buffer", "CALL(fle_imp_GetLogicalDriveStringsW, 10, 0);")]);
}

#[test]
fn drive_strings_are_accepted_by_get_drive_type() {
    // page (GetLogicalDriveStrings): "Each string in the buffer may be used wherever a root directory is required,
    // such as for the GetDriveType".
    let l = lines(
        "vol-drivestrings-roundtrip",
        r#"
    uint16_t buf[8];
    uint64_t r;
    CALL(fle_imp_GetLogicalDriveStringsW, 8, A(buf));
    r = CALL(fle_imp_GetDriveTypeW, A(buf)); printf("%llu\n", U(r));
"#,
    );
    assert_eq!(l, ["3"]);
}

// ---- GetVolumeInformationW ----

#[test]
fn volume_information_fills_every_requested_output() {
    // Arguments: root, volume buffer, volume buffer size, serial*, max component*, flags*, fs buffer, fs buffer size.
    let l = lines(
        "vol-volinfo",
        &format!(
            r#"
    uint16_t vb[261], fb[261];
    uint32_t serial = 0xAAAAAAAA, maxc = 0xAAAAAAAA, flags = 0xAAAAAAAA;
    uint64_t r;
    int i;
    for (i = 0; i < 261; i++) {{ vb[i] = 0x7777; fb[i] = 0x7777; }}
    SETLE(55);
    r = CALL(fle_imp_GetVolumeInformationW, {c}, A(vb), 261, A(&serial), A(&maxc), A(&flags), A(fb), 261);
    printf("ret %llu le %u\n", U(r), LE());
    printf("label_empty %d next %04x\n", vb[0] == 0, vb[1]);
    printf("fs %c%c%c%c%c nul %04x next %04x\n", fb[0], fb[1], fb[2], fb[3], fb[4], fb[5], fb[6]);
    printf("serial %x maxc %u flags %x\n", serial, maxc, flags);
    /* NULL root is the root of the current directory; every output is optional; sizes of absent buffers are ignored */
    serial = 0; r = CALL(fle_imp_GetVolumeInformationW, 0, 0, 99999, A(&serial), 0, 0, 0, 99999);
    printf("null_root %llu serial_set %d\n", U(r), serial != 0);
    r = CALL(fle_imp_GetVolumeInformationW, {c}, 0, 0, 0, 0, 0, 0, 0); printf("none %llu\n", U(r));
    /* only the volume name, in a 1-unit buffer (room for the NUL) */
    vb[0] = 0x7777; r = CALL(fle_imp_GetVolumeInformationW, {c}, A(vb), 1, 0, 0, 0, 0, 0); printf("one_unit %llu %04x\n", U(r), vb[0]);
    /* the file system name in exactly 6 units */
    for (i = 0; i < 8; i++) fb[i] = 0x7777;
    r = CALL(fle_imp_GetVolumeInformationW, {c}, 0, 0, 0, 0, 0, A(fb), 6); printf("fs6 %llu %04x %04x\n", U(r), fb[4], fb[5]);
"#,
            c = cw("C:\\"),
        ),
    );
    assert_eq!(
        l,
        [
            // page: nonzero when everything asked for was retrieved. The last error after a success is not specified,
            // and this runtime leaves it alone.
            "ret 1 le 55",
            // choice: the volume has an empty label
            "label_empty 1 next 7777",
            // choice: the file system is "FAT32" (the page names FAT and NTFS as examples)
            "fs FAT32 nul 0000 next 7777",
            "serial 464c4531 maxc 255 flags 6",
            "null_root 1 serial_set 1",
            "none 1",
            "one_unit 1 0000",
            "fs6 1 0032 0000",
        ]
    );
}

#[test]
fn volume_information_fails_for_a_root_without_a_volume() {
    let l = lines(
        "vol-volinfo-missing",
        &format!(
            r#"
    uint32_t serial = 0x55555555;
    uint64_t r;
    SETLE(0);
    r = CALL(fle_imp_GetVolumeInformationW, {d}, 0, 0, A(&serial), 0, 0, 0, 0);
    printf("%llu le %u serial %x\n", U(r), LE(), serial);
"#,
            d = cw("D:\\"),
        ),
    );
    // page: zero when the information could not be retrieved; the code is not named: 3 is an assumption
    assert_eq!(l, ["0 le 3 serial 55555555"]);
}

#[test]
fn volume_information_cases_outside_the_page_stop_the_run() {
    let c = cw("C:\\");
    assert_aborts(
        "vol-abort-volinfo",
        "uint16_t b[400]; uint16_t fb[400];",
        &[
            ("no-backslash", &format!("CALL(fle_imp_GetVolumeInformationW, {}, 0, 0, 0, 0, 0, 0, 0);", cw("C:"))),
            ("unc", &format!("CALL(fle_imp_GetVolumeInformationW, {}, 0, 0, 0, 0, 0, 0, 0);", cw("\\\\MyServer\\MyShare\\"))),
            ("volname-buffer-0", &format!("CALL(fle_imp_GetVolumeInformationW, {c}, A(b), 0, 0, 0, 0, 0, 0);")),
            ("fsname-buffer-short", &format!("CALL(fle_imp_GetVolumeInformationW, {c}, 0, 0, 0, 0, 0, A(fb), 5);")),
            ("volname-over-max-path", &format!("CALL(fle_imp_GetVolumeInformationW, {c}, A(b), 262, 0, 0, 0, 0, 0);")),
            ("fsname-over-max-path", &format!("CALL(fle_imp_GetVolumeInformationW, {c}, 0, 0, 0, 0, 0, A(fb), 262);")),
        ],
    );
}

// ---- QueryDosDeviceW ----

fn dos_target_units() -> usize {
    // the QueryDosDevice page's example form of a volume device name
    "\\Device\\HarddiskVolume1".len()
}

#[test]
fn query_dos_device_returns_the_mapping_of_a_drive_letter() {
    let n = dos_target_units();
    let l = lines(
        "vol-dosdev",
        &format!(
            r#"
    uint16_t buf[64];
    uint64_t r;
    int i;
    for (i = 0; i < 64; i++) buf[i] = 0x7777;
    r = CALL(fle_imp_QueryDosDeviceW, {c}, A(buf), 64);
    printf("ret %llu\n", U(r));
    printf("text ");
    for (i = 0; buf[i]; i++) printf("%c", (char)buf[i]);
    printf("\n");
    printf("final_nul %d after %04x\n", buf[{n} + 1] == 0, buf[{n} + 2]);
"#,
            c = cw("C:"),
            n = n,
        ),
    );
    // page: the first string is the mapping, the last string is followed by an additional NUL; success returns the
    // number of TCHARs stored. The target is the page's example form (the number 1 is a choice).
    assert_eq!(l[0], format!("ret {}", n + 2));
    assert_eq!(l[1], "text \\Device\\HarddiskVolume1");
    assert_eq!(l[2], "final_nul 1 after 7777");
}

#[test]
fn query_dos_device_buffer_rules_and_missing_devices() {
    let n = dos_target_units() + 2;
    let l = lines(
        "vol-dosdev-buf",
        &format!(
            r#"
    uint16_t buf[64];
    uint64_t r;
    int i;
    for (i = 0; i < 64; i++) buf[i] = 0x7777;
    SETLE(0); r = CALL(fle_imp_QueryDosDeviceW, {c}, A(buf), {small}); printf("small %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_QueryDosDeviceW, {c}, A(buf), 0); printf("zero %llu le %u\n", U(r), LE());
    printf("untouched %d\n", buf[0] == 0x7777);
    SETLE(0); r = CALL(fle_imp_QueryDosDeviceW, {c}, A(buf), {exact}); printf("exact %llu\n", U(r));
    SETLE(0); r = CALL(fle_imp_QueryDosDeviceW, {lc}, A(buf), 64); printf("lower %llu\n", U(r));
    SETLE(0); r = CALL(fle_imp_QueryDosDeviceW, {d}, A(buf), 64); printf("missing %llu le %u\n", U(r), LE());
"#,
            c = cw("C:"),
            lc = cw("c:"),
            d = cw("D:"),
            small = n - 1,
            exact = n,
        ),
    );
    assert_eq!(
        l,
        [
            // page: "If the buffer is too small, the function fails and the last error code is ERROR_INSUFFICIENT_BUFFER"
            "small 0 le 122".to_string(),
            "zero 0 le 122".to_string(),
            "untouched 1".to_string(),
            format!("exact {n}"),
            format!("lower {n}"),
            // page: failure returns zero; the code is not named for a letter with no mapping: 2 is an assumption
            "missing 0 le 2".to_string(),
        ]
    );
}

#[test]
fn query_dos_device_other_forms_stop_the_run() {
    assert_aborts(
        "vol-abort-dosdev",
        "uint16_t b[64];",
        &[
            ("null-name-list", "CALL(fle_imp_QueryDosDeviceW, 0, A(b), 64);"),
            ("trailing-backslash", &format!("CALL(fle_imp_QueryDosDeviceW, {}, A(b), 64);", cw("C:\\"))),
            ("physical-drive", &format!("CALL(fle_imp_QueryDosDeviceW, {}, A(b), 64);", cw("PhysicalDrive0"))),
            ("long-form", &format!("CALL(fle_imp_QueryDosDeviceW, {}, A(b), 64);", cw("C:x"))),
            ("empty", &format!("CALL(fle_imp_QueryDosDeviceW, {}, A(b), 64);", cw(""))),
            ("digit-drive", &format!("CALL(fle_imp_QueryDosDeviceW, {}, A(b), 64);", cw("1:"))),
            ("null-buffer", &format!("CALL(fle_imp_QueryDosDeviceW, {}, 0, 64);", cw("C:"))),
        ],
    );
}

// ---- DuplicateHandle ----

const SAME_ACCESS: u32 = 2;
const CLOSE_SOURCE: u32 = 1;

#[test]
fn duplicate_handle_makes_real_handles_from_the_pseudo_handles() {
    let l = lines(
        "vol-dup-pseudo",
        &format!(
            r#"
    uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), tp = CALL(fle_imp_GetCurrentThread, 0);
    uint64_t h1 = 0, h2 = 0, h3 = 0, r;
    uint64_t base = FLE_VOL_HANDLE_BASE;
    /* process: DUPLICATE_SAME_ACCESS, as in the page's example */
    r = CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h1), 0, 0, {same}); printf("p %llu\n", U(r));
    r = CALL(fle_imp_DuplicateHandle, pp, tp, pp, A(&h2), 0, 0, {same}); printf("t %llu\n", U(r));
    /* a real handle is not the pseudo handle, and each call makes a new one */
    printf("real %d %d distinct %d\n", h1 != 0 && h1 != INVH && h1 != pp, h2 != 0 && h2 != INVH && h2 != tp, h1 != h2);
    printf("in_region %d %d mult4 %d\n", h1 > base && h1 < base + FLE_VOL_REGION_SIZE, h2 > base && h2 < base + FLE_VOL_REGION_SIZE, (h1 % 4) == 0 && (h2 % 4) == 0);
    printf("count %llu\n", U(fle_vol_handle_count()));
    /* the access and inherit arguments are accepted without effect when SAME_ACCESS is not set */
    r = CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h3), 0x1F0FFF, 1, 0); printf("access %llu %d\n", U(r), h3 != 0);
    /* lpTargetHandle NULL: duplicated, the value is not returned (page, backward compatibility) */
    r = CALL(fle_imp_DuplicateHandle, pp, pp, pp, 0, 0, 0, {same}); printf("nullout %llu count %llu\n", U(r), U(fle_vol_handle_count()));
    /* a real handle can itself be the source and as the process handle */
    r = CALL(fle_imp_DuplicateHandle, h1, h2, h1, A(&h3), 0, 0, {same}); printf("real_src %llu\n", U(r));
    printf("count %llu\n", U(fle_vol_handle_count()));
"#,
            same = SAME_ACCESS,
        ),
    );
    assert_eq!(
        l,
        [
            "p 1",
            "t 1",
            "real 1 1 distinct 1",
            "in_region 1 1 mult4 1",
            "count 2",
            "access 1 1",
            "nullout 1 count 4",
            "real_src 1",
            "count 5",
        ]
    );
}

#[test]
fn duplicated_handles_are_closed_with_close_handle() {
    let l = lines(
        "vol-dup-close",
        &format!(
            r#"
    uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), h = 0, r;
    CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h), 0, 0, {same});
    r = CALL(fle_imp_CloseHandle, h); printf("close %llu\n", U(r));
    SETLE(0); r = CALL(fle_imp_CloseHandle, h); printf("again %llu le %u\n", U(r), LE());
    printf("count %llu\n", U(fle_vol_handle_count()));
    /* a closed handle cannot be a source */
    SETLE(0); r = CALL(fle_imp_DuplicateHandle, pp, h, pp, A(&h), 0, 0, {same}); printf("dup_closed %llu le %u\n", U(r), LE());
    /* file handles still close through the same entry point */
    {{
        uint64_t f = CALL(fle_imp_CreateFileW, {name}, 0xC0000000u, 0, 0, 2, 0x80, 0);
        printf("file_ok %d\n", f != INVH);
        r = CALL(fle_imp_CloseHandle, f); printf("file_close %llu\n", U(r));
        SETLE(0); r = CALL(fle_imp_CloseHandle, f); printf("file_again %llu le %u\n", U(r), LE());
    }}
    SETLE(0); r = CALL(fle_imp_CloseHandle, 0); printf("null %llu le %u\n", U(r), LE());
"#,
            same = SAME_ACCESS,
            name = cw("C:\\dupclose.bin"),
        ),
    );
    // CloseHandle failures: 0 with ERROR_INVALID_HANDLE (6), as fle_vfs.c (inference: the page names no code)
    assert_eq!(
        l,
        [
            "close 1",
            "again 0 le 6",
            "count 0",
            "dup_closed 0 le 6",
            "file_ok 1",
            "file_close 1",
            "file_again 0 le 6",
            "null 0 le 6",
        ]
    );
}

#[test]
fn duplicate_close_source_closes_the_source_handle() {
    let l = lines(
        "vol-dup-closesrc",
        &format!(
            r#"
    uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), a = 0, b = 0, r;
    CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&a), 0, 0, {same});
    /* page: DUPLICATE_CLOSE_SOURCE closes the source handle; the duplicate is new */
    r = CALL(fle_imp_DuplicateHandle, pp, a, pp, A(&b), 0, 0, {same} | {close}); printf("moved %llu new %d old_differs %d\n", U(r), b != 0, b != a);
    printf("count %llu\n", U(fle_vol_handle_count()));
    SETLE(0); r = CALL(fle_imp_CloseHandle, a); printf("old_closed %llu le %u\n", U(r), LE());
    /* page: hTargetProcessHandle may be NULL with DUPLICATE_CLOSE_SOURCE: the source is only closed */
    r = CALL(fle_imp_DuplicateHandle, pp, b, 0, 0, 0, 0, {close}); printf("only_close %llu count %llu\n", U(r), U(fle_vol_handle_count()));
    SETLE(0); r = CALL(fle_imp_CloseHandle, b); printf("b_closed %llu le %u\n", U(r), LE());
"#,
            same = SAME_ACCESS,
            close = CLOSE_SOURCE,
        ),
    );
    assert_eq!(l, ["moved 1 new 1 old_differs 1", "count 1", "old_closed 0 le 6", "only_close 1 count 0", "b_closed 0 le 6"]);
}

#[test]
fn duplicate_handle_cases_outside_the_page_stop_the_run() {
    let create = format!("f = CALL(fle_imp_CreateFileW, {}, 0xC0000000u, 0, 0, 2, 0x80, 0);", cw("C:\\dupabort.bin"));
    let vfs_file = format!("{create} CALL(fle_imp_DuplicateHandle, pp, f, pp, A(&h), 0, 0, 2);");
    assert_aborts(
        "vol-abort-dup",
        "uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), tp = CALL(fle_imp_GetCurrentThread, 0), h = 0, f;",
        &[
            ("source-process-foreign", "CALL(fle_imp_DuplicateHandle, 0x1234, pp, pp, A(&h), 0, 0, 2);"),
            ("source-process-thread-pseudo", "CALL(fle_imp_DuplicateHandle, tp, pp, pp, A(&h), 0, 0, 2);"),
            ("target-process-foreign", "CALL(fle_imp_DuplicateHandle, pp, pp, 0x1234, A(&h), 0, 0, 2);"),
            ("target-null-without-close", "CALL(fle_imp_DuplicateHandle, pp, pp, 0, A(&h), 0, 0, 2);"),
            ("options-bit", "CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h), 0, 0, 4);"),
            ("options-high-bit", "CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h), 0, 0, 0x80000002u);"),
            ("close-source-pseudo-process", "CALL(fle_imp_DuplicateHandle, pp, pp, pp, A(&h), 0, 0, 3);"),
            ("close-source-pseudo-thread", "CALL(fle_imp_DuplicateHandle, pp, tp, 0, 0, 0, 0, 1);"),
            ("source-foreign", "CALL(fle_imp_DuplicateHandle, pp, 0x1234, pp, A(&h), 0, 0, 2);"),
            ("source-vfs-file", &vfs_file),
            ("source-never-issued", "CALL(fle_imp_DuplicateHandle, pp, FLE_VOL_HANDLE_BASE + 4 * 77, pp, A(&h), 0, 0, 2);"),
            ("source-not-aligned", "CALL(fle_imp_DuplicateHandle, pp, FLE_VOL_HANDLE_BASE + 5, pp, A(&h), 0, 0, 2);"),
        ],
    );
}

#[test]
fn a_second_thread_stops_the_run() {
    let o = run_src(
        "vol-thread",
        "static void *th(void *x) { (void)x; CALL(fle_imp_GetLogicalDrives, 0); printf(\"returned-from-thread\\n\"); return 0; }",
        "    pthread_t t;\n    CALL(fle_imp_GetLogicalDrives, 0);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: GetLogicalDrives") && o.stderr.contains("second thread"), "{}", o.stderr);
}

#[test]
fn source_uses_no_host_drive_volume_process_or_environment_state() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_vol.c")).unwrap());
    for bad in [
        "getpid", "getppid", "getenv", "environ", "fopen", "open(", "statfs", "statvfs", "stat(", "mntent", "getmntent", "/proc", "/etc", "/home",
        "/dev", "<mntent.h>", "<sys/stat.h>", "<sys/statvfs.h>", "exit(", "_exit", "dup(", "dup2(",
    ] {
        assert!(!code.contains(bad), "fle_vol.c must not use {bad}");
    }
}
