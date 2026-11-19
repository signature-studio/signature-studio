//! Tests for the virtual file system (docs/decisions/0018-virtual-fs-reg-env.md): runtime/fle_vfs.c.
//! Each expected value is written from the Microsoft page named in the comment (listed in
//! docs/investigations/import-vfs-spec.md), as a literal or by a Rust expression that does not use the C code.
//! Values that the pages do not give (marked "inference" in the C comments) are only checked where a test says so.
//! The host is Linux: nothing is compared with a real Windows, and no host file is read or written.

mod fsx_common;
use fsx_common::*;

const GR: &str = "0x80000000u";
const GW: &str = "0x40000000u";
const GRW: &str = "0xC0000000u";
const INVALID_HANDLE: u64 = u64::MAX;

fn create(path: &str, access: &str, share: u32, disp: u32, flags: u32) -> String {
    format!("CALL(fle_imp_CreateFileW, {}, {access}, {share}, 0, {disp}, {flags}, 0)", cw(path))
}

// ---- handle regions (decision 0018) ----

#[test]
fn handle_regions_do_not_overlap_anything_else() {
    let file = header_const("fle_vfs.h", "FLE_VFS_FILE_BASE");
    let find = header_const("fle_vfs.h", "FLE_VFS_FIND_BASE");
    let key = header_const("fle_vfs.h", "FLE_REG_KEY_BASE");
    let size = header_const("fle_vfs.h", "FLE_HANDLE_REGION_SIZE");
    let heap = header_const("fle_sys.h", "FLE_PROCESS_HEAP");
    let mod_base = header_const("fle_modules.h", "FLE_MODULE_HANDLE_BASE");
    let mod_step = header_const("fle_modules.h", "FLE_MODULE_HANDLE_STEP");
    let resp_base: u64 = 0x7FF9_0000_0000; // imports_table.rs RESP_VA_BASE (decision 0015)
    let resp_end = resp_base + 0x10 * 1000; // generous bound for the response table rows
    let image_end: u64 = 0x400000 + 0x18c0000;
    let mods_end = mod_base + mod_step * 34;
    let regions = [("file", file), ("find", find), ("key", key)];
    for (n, b) in regions {
        assert!(b > heap + 0x1000_0000, "{n} region not above the process heap token");
        assert!(b > mods_end, "{n} region not above the module handles");
        assert!(b > resp_end, "{n} region not above the response pseudo-VAs");
        assert!(b > image_end);
        assert!(b + size < (1u64 << 47), "{n} region outside user address space");
    }
    for i in 0..3 {
        for j in 0..3 {
            if i != j {
                let (a, b) = (regions[i].1, regions[j].1);
                assert!(a + size <= b || b + size <= a, "regions {} and {} overlap", regions[i].0, regions[j].0);
            }
        }
    }
}

#[test]
fn issued_handles_are_in_their_region_distinct_and_multiples_of_four() {
    let l = lines(
        "vfs-handles",
        &format!(
            r#"
    uint64_t d = CALL(fle_imp_CreateDirectoryW, {d}, 0);
    uint64_t h1 = {c1}, h2 = {c2};
    static uint64_t fd[74];
    uint64_t f = CALL(fle_imp_FindFirstFileW, {pat}, A(fd));
    uint64_t k = 0;
    CALL(fle_imp_RegCreateKeyExW, FLE_HKEY_CURRENT_USER, W("K"), 0, 0, 0, 0x20019, 0, A(&k), 0);
    printf("%llx\n%llx\n%llx\n%llx\n", U(h1), U(h2), U(f), U(k));
"#,
            d = cw("C:\\h"),
            c1 = create("C:\\h\\a", GRW, 0, 2, 0x80),
            c2 = create("C:\\h\\b", GRW, 0, 2, 0x80),
            pat = cw("C:\\h\\*"),
        ),
    );
    let v: Vec<u64> = l.iter().map(|s| u64::from_str_radix(s, 16).unwrap()).collect();
    let file = header_const("fle_vfs.h", "FLE_VFS_FILE_BASE");
    let find = header_const("fle_vfs.h", "FLE_VFS_FIND_BASE");
    let key = header_const("fle_vfs.h", "FLE_REG_KEY_BASE");
    let size = header_const("fle_vfs.h", "FLE_HANDLE_REGION_SIZE");
    assert!(v[0] > file && v[0] < file + size && v[1] > file && v[1] < file + size);
    assert_ne!(v[0], v[1]);
    assert!(v[2] > find && v[2] < find + size);
    assert!(v[3] > key && v[3] < key + size);
    assert!(v.iter().all(|h| h % 4 == 0));
    assert_eq!(v[1], v[0] + 4, "values are handed out in order");
}

#[test]
fn runtime_never_uses_host_file_environment_or_registry_apis() {
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c"] {
        let t = std::fs::read_to_string(runtime_dir().join(f)).unwrap();
        for bad in [
            "fopen", "getenv", "opendir", "readdir", "char **environ", "<unistd.h>", "<fcntl.h>", "<dirent.h>", "<sys/stat.h>",
            "<sys/types.h>", "popen", "system(", "realpath", "getcwd", "/proc", "/etc", "/home",
        ] {
            assert!(!t.contains(bad), "{f} mentions {bad}");
        }
    }
}

// ---- host isolation ----

#[test]
fn host_paths_do_not_exist_in_the_virtual_volume() {
    // The virtual volume starts empty: a path that exists on any Linux host is simply absent here.
    let l = lines(
        "vfs-host",
        &format!(
            r#"
    const char *names[] = {{ "etc", "proc", "home", "tmp", "usr" }};
    int i;
    uint64_t h = {a};
    printf("%llx %u\n", U(h), LE());
    h = {b};
    printf("%llx %u\n", U(h), LE());
    printf("%llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {c})), LE());
    printf("%llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {d})), LE());
    printf("%llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {e})), LE());
"#,
            a = create("/etc/hostname", GR, 7, 3, 0x80),
            b = create("C:\\proc\\self\\status", GR, 7, 3, 0x80),
            c = cw("C:\\"),
            d = cw("/etc"),
            e = cw("C:\\Windows"),
        ),
    );
    assert_eq!(l[0], "ffffffffffffffff 3"); // missing directory: ERROR_PATH_NOT_FOUND
    assert_eq!(l[1], "ffffffffffffffff 3");
    // The root directory has FILE_ATTRIBUTE_DIRECTORY (0x10). GetLastError after a success is not specified.
    assert!(l[2].starts_with("10 "), "{}", l[2]);
    assert_eq!(l[3], "ffffffff 2"); // "\etc" is C:\etc, missing at the volume root: ERROR_FILE_NOT_FOUND
    assert_eq!(l[4], "ffffffff 2"); // nothing is pre-installed, not even C:\Windows
}

// ---- CreateFileW ----

#[test]
fn create_file_dispositions_and_last_error() {
    // MSDN CreateFileW, dwCreationDisposition: the table of the five values and their last-error codes.
    let l = lines(
        "vfs-create",
        &format!(
            r#"
    uint64_t h;
    CALL(fle_imp_CreateDirectoryW, {d}, 0);
    /* CREATE_NEW on a missing file creates it; the last error is not touched by the page (55 stays) */
    SETLE(55); h = {cn1}; printf("new1 %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    /* CREATE_NEW on an existing file fails with ERROR_FILE_EXISTS (80) */
    SETLE(0); h = {cn1}; printf("new2 %llx %u\n", U(h), LE());
    /* OPEN_EXISTING: missing file 2, missing directory 3, existing succeeds */
    h = {oe_missing}; printf("oe_missing %llx %u\n", U(h), LE());
    h = {oe_nodir}; printf("oe_nodir %llx %u\n", U(h), LE());
    SETLE(55); h = {oe}; printf("oe %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    /* CREATE_ALWAYS: missing -> created, last error 0; existing -> succeeds, last error 183 */
    SETLE(55); h = {ca_new}; printf("ca_new %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    SETLE(0); h = {ca_new}; printf("ca_old %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    /* OPEN_ALWAYS: existing -> 183, missing -> 0 */
    SETLE(0); h = {oa_old}; printf("oa_old %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    SETLE(55); h = {oa_new}; printf("oa_new %d %u\n", h != {inv}ull, LE());
    CALL(fle_imp_CloseHandle, h);
    /* TRUNCATE_EXISTING: missing file 2 */
    h = {te_missing}; printf("te_missing %llx %u\n", U(h), LE());
"#,
            d = cw("C:\\dd"),
            inv = INVALID_HANDLE,
            cn1 = create("C:\\dd\\one", GRW, 0, 1, 0x80),
            oe_missing = create("C:\\dd\\none", GR, 7, 3, 0x80),
            oe_nodir = create("C:\\nodir\\x", GR, 7, 3, 0x80),
            oe = create("C:\\dd\\one", GR, 7, 3, 0x80),
            ca_new = create("C:\\dd\\two", GRW, 0, 2, 0x80),
            oa_old = create("C:\\dd\\two", GRW, 0, 4, 0x80),
            oa_new = create("C:\\dd\\three", GRW, 0, 4, 0x80),
            te_missing = create("C:\\dd\\four", GRW, 0, 5, 0x80),
        ),
    );
    assert_eq!(
        l,
        [
            "new1 1 55",
            "new2 ffffffffffffffff 80",
            "oe_missing ffffffffffffffff 2",
            "oe_nodir ffffffffffffffff 3",
            "oe 1 55",
            "ca_new 1 0",
            "ca_old 1 183",
            "oa_old 1 183",
            "oa_new 1 0",
            "te_missing ffffffffffffffff 2",
        ]
    );
}

#[test]
fn create_file_is_case_insensitive_and_accepts_forward_slashes() {
    // MSDN Naming Files: "Do not assume case sensitivity"; CreateFileW: "You may use either forward slashes (/) or backslashes".
    let l = lines(
        "vfs-case",
        &format!(
            r#"
    uint64_t h;
    CALL(fle_imp_CreateDirectoryW, {d}, 0);
    h = {c1}; CALL(fle_imp_CloseHandle, h);
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {q1})));
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {q2})));
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {q3})));
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {q4})));
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {q5})));
"#,
            d = cw("C:\\MixedCase"),
            c1 = create("c:/mixedcase/File.TXT", GRW, 0, 1, 0x80),
            q1 = cw("C:\\MIXEDCASE\\file.txt"),
            q2 = cw("c:\\mixedcase\\FILE.txt"),
            q3 = cw("C:\\MixedCase\\.\\File.TXT"),
            q4 = cw("C:\\MixedCase\\x\\..\\File.TXT"),
            q5 = cw("\\mixedcase\\file.txt"),
        ),
    );
    assert_eq!(l, ["80", "80", "80", "80", "80"]);
}

#[test]
fn sharing_rules_between_opens() {
    // MSDN CreateFileW, dwShareMode: a share mode of 0 prevents later opens; a later open that asks for an access
    // the earlier handle does not share fails with ERROR_SHARING_VIOLATION (32).
    let l = lines(
        "vfs-share",
        &format!(
            r#"
    uint64_t h1, h2;
    h1 = {excl}; printf("open1 %d\n", h1 != {inv}ull);
    h2 = {again}; printf("again %llx %u\n", U(h2), LE());
    CALL(fle_imp_CloseHandle, h1);
    h1 = {rshare}; printf("open2 %d\n", h1 != {inv}ull);
    h2 = {read2}; printf("read2 %d\n", h2 != {inv}ull); CALL(fle_imp_CloseHandle, h2);
    h2 = {write2}; printf("write2 %llx %u\n", U(h2), LE());
    CALL(fle_imp_CloseHandle, h1);
"#,
            inv = INVALID_HANDLE,
            excl = create("C:\\s.bin", GRW, 0, 2, 0x80),
            again = create("C:\\s.bin", GR, 7, 3, 0x80),
            rshare = create("C:\\s.bin", GR, 1, 3, 0x80),
            read2 = create("C:\\s.bin", GR, 1, 3, 0x80),
            write2 = create("C:\\s.bin", GW, 7, 3, 0x80),
        ),
    );
    assert_eq!(
        l,
        ["open1 1", "again ffffffffffffffff 32", "open2 1", "read2 1", "write2 ffffffffffffffff 32"]
    );
}

// ---- ReadFile, WriteFile, SetFilePointer, GetFileSize ----

#[test]
fn read_write_seek_and_size() {
    let text = "hello world"; // 11 bytes
    let l = lines(
        "vfs-rw",
        &format!(
            r#"
    static char buf[256];
    uint32_t n = 99;
    uint64_t h = {c};
    uint64_t ok;
    ok = CALL(fle_imp_WriteFile, h, A("{text}"), 11, A(&n), 0); printf("write %llu %u\n", U(ok), n);
    /* file pointer is now 11: reading there is the end of the file (MSDN ReadFile: ERROR_HANDLE_EOF 38) */
    n = 99; ok = CALL(fle_imp_ReadFile, h, A(buf), 10, A(&n), 0); printf("eof %llu %u %u\n", U(ok), n, LE());
    printf("seek %llu\n", U(CALL(fle_imp_SetFilePointer, h, 0, 0, 0)));
    n = 99; ok = CALL(fle_imp_ReadFile, h, A(buf), 5, A(&n), 0); buf[n] = 0; printf("r1 %llu %u %s\n", U(ok), n, buf);
    /* a read that crosses the end succeeds with what is left (MSDN ReadFile) */
    n = 99; ok = CALL(fle_imp_ReadFile, h, A(buf), 100, A(&n), 0); buf[n] = 0; printf("r2 %llu %u [%s]\n", U(ok), n, buf);
    printf("size %llu\n", U(CALL(fle_imp_GetFileSize, h, 0)));
    /* seek beyond the end is allowed; a write there zero-fills the gap (MSDN SetFilePointer) */
    printf("seek100 %llu\n", U(CALL(fle_imp_SetFilePointer, h, 100, 0, 0)));
    ok = CALL(fle_imp_WriteFile, h, A("x"), 1, A(&n), 0); printf("w2 %llu %u\n", U(ok), n);
    printf("size2 %llu\n", U(CALL(fle_imp_GetFileSize, h, 0)));
    CALL(fle_imp_SetFilePointer, h, 11, 0, 0);
    n = 0; ok = CALL(fle_imp_ReadFile, h, A(buf), 89, A(&n), 0);
    {{ int z = 1, i; for (i = 0; i < 89; i++) if (buf[i] != 0) z = 0; printf("gap %u %d\n", n, z); }}
    /* FILE_CURRENT and FILE_END */
    printf("cur %llu\n", U(CALL(fle_imp_SetFilePointer, h, (uint64_t)(uint32_t)-1, 0, 1)));
    printf("end %llu\n", U(CALL(fle_imp_SetFilePointer, h, 0, 0, 2)));
    printf("end-1 %llu\n", U(CALL(fle_imp_SetFilePointer, h, (uint64_t)(uint32_t)-1, 0, 2)));
    /* a negative result fails with ERROR_NEGATIVE_SEEK (131) and leaves the pointer */
    printf("neg %llx %u\n", U(CALL(fle_imp_SetFilePointer, h, (uint64_t)(uint32_t)-1000, 0, 0)), LE());
    printf("still %llu\n", U(CALL(fle_imp_SetFilePointer, h, 0, 0, 1)));
    /* 64-bit distance through lpDistanceToMoveHigh: 1<<32 | 5, new position high word is returned */
    {{
        int32_t hi = 1; uint64_t lo;
        SETLE(9);
        lo = CALL(fle_imp_SetFilePointer, h, 5, A(&hi), 0);
        printf("hi %llu %d %u\n", U(lo), hi, LE());
    }}
    CALL(fle_imp_CloseHandle, h);
"#,
            c = create("C:\\rw.txt", GRW, 0, 2, 0x80),
        ),
    );
    assert_eq!(
        l,
        [
            "write 1 11",
            "eof 1 0 0", // decision 0243: ReadFile at the end of a file returns TRUE with 0 bytes read (Learn "Testing for the End of a File")
            "seek 0",
            "r1 1 5 hello",
            "r2 1 6 [ world]",
            "size 11",
            "seek100 100",
            "w2 1 1",
            "size2 101",
            "gap 89 1",
            "cur 99",
            "end 101",
            "end-1 100",
            "neg ffffffff 131",
            "still 100",
            "hi 5 1 0",
        ]
    );
    assert_eq!(text.len(), 11);
}

#[test]
fn get_file_size_high_word_and_error_on_bad_handle() {
    let l = lines(
        "vfs-size",
        &format!(
            r#"
    uint32_t hi = 77;
    uint64_t h = {c};
    uint64_t lo;
    lo = CALL(fle_imp_GetFileSize, h, A(&hi)); printf("%llu %u\n", U(lo), hi);
    CALL(fle_imp_CloseHandle, h);
    /* a closed handle: INVALID_FILE_SIZE (0xffffffff) and an error other than NO_ERROR (MSDN GetFileSize) */
    SETLE(0);
    lo = CALL(fle_imp_GetFileSize, h, A(&hi)); printf("%llx %d\n", U(lo), LE() != 0);
"#,
            c = create("C:\\z.bin", GRW, 0, 2, 0x80),
        ),
    );
    assert_eq!(l, ["0 0", "ffffffff 1"]);
}

#[test]
fn access_rights_are_enforced_on_read_and_write() {
    // MSDN ReadFile/WriteFile: hFile must have read / write access. The error code is an inference (5).
    let l = lines(
        "vfs-access",
        &format!(
            r#"
    static char b[8];
    uint32_t n = 5;
    uint64_t r = {ro};
    uint64_t w = {wo};
    printf("%llu %u\n", U(CALL(fle_imp_WriteFile, r, A("a"), 1, A(&n), 0)), n);
    printf("%llu %u\n", U(CALL(fle_imp_ReadFile, w, A(b), 1, A(&n), 0)), n);
    printf("%llu\n", U(CALL(fle_imp_WriteFile, w, A("abc"), 3, A(&n), 0)));
    printf("n=%u\n", n);
"#,
            ro = create("C:\\acc", GR, 7, 4, 0x80),
            wo = create("C:\\acc", GW, 7, 3, 0x80),
        ),
    );
    // both count outputs are zeroed first (the pages: "sets this value to zero before doing any work or error checking")
    assert_eq!(l, ["0 0", "0 0", "1", "n=3"]);
}

// ---- directories, attributes, delete, copy, move ----

#[test]
fn directories_and_attributes() {
    let l = lines(
        "vfs-dir",
        &format!(
            r#"
    uint64_t h;
    printf("mk %llu\n", U(CALL(fle_imp_CreateDirectoryW, {a}, 0)));
    /* existing: ERROR_ALREADY_EXISTS (183); missing intermediate: ERROR_PATH_NOT_FOUND (3) (MSDN CreateDirectoryW) */
    printf("again %llu %u\n", U(CALL(fle_imp_CreateDirectoryW, {a}, 0)), LE());
    printf("deep %llu %u\n", U(CALL(fle_imp_CreateDirectoryW, {deep}, 0)), LE());
    printf("sub %llu\n", U(CALL(fle_imp_CreateDirectoryW, {sub}, 0)));
    h = {f}; CALL(fle_imp_CloseHandle, h);
    printf("dirattr %llx\n", U(CALL(fle_imp_GetFileAttributesW, {a})));
    printf("fileattr %llx\n", U(CALL(fle_imp_GetFileAttributesW, {fp})));
    printf("missing %llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {m})), LE());
    printf("nodir %llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {nd})), LE());
    /* READONLY is settable; delete then fails with ERROR_ACCESS_DENIED (5) (MSDN DeleteFile) */
    printf("setro %llu\n", U(CALL(fle_imp_SetFileAttributesW, {fp}, 1)));
    printf("ro %llx\n", U(CALL(fle_imp_GetFileAttributesW, {fp})));
    printf("delro %llu %u\n", U(CALL(fle_imp_DeleteFileW, {fp})), LE());
    printf("setarch %llu\n", U(CALL(fle_imp_SetFileAttributesW, {fp}, 0x22)));
    printf("arch %llx\n", U(CALL(fle_imp_GetFileAttributesW, {fp})));
    printf("setnorm %llu\n", U(CALL(fle_imp_SetFileAttributesW, {fp}, 0x80)));
    printf("norm %llx\n", U(CALL(fle_imp_GetFileAttributesW, {fp})));
    /* RemoveDirectory needs an empty directory (page); the code for "not empty" is an inference (145) */
    printf("rmnotempty %llu %u\n", U(CALL(fle_imp_RemoveDirectoryW, {a})), LE());
    printf("del %llu\n", U(CALL(fle_imp_DeleteFileW, {fp})));
    printf("del2 %llu %u\n", U(CALL(fle_imp_DeleteFileW, {fp})), LE());
    printf("rmsub %llu\n", U(CALL(fle_imp_RemoveDirectoryW, {sub})));
    printf("rm %llu\n", U(CALL(fle_imp_RemoveDirectoryW, {a})));
    printf("gone %llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {a})), LE());
"#,
            a = cw("C:\\adir"),
            deep = cw("C:\\nope\\child"),
            sub = cw("C:\\adir\\sub"),
            f = create("C:\\adir\\f.txt", GRW, 0, 1, 0x80),
            fp = cw("C:\\adir\\f.txt"),
            m = cw("C:\\adir\\missing"),
            nd = cw("C:\\nodir\\x"),
        ),
    );
    assert_eq!(
        l,
        [
            "mk 1",
            "again 0 183",
            "deep 0 3",
            "sub 1",
            "dirattr 10",
            "fileattr 80",
            "missing ffffffff 2",
            "nodir ffffffff 3",
            "setro 1",
            "ro 1",
            "delro 0 5",
            "setarch 1",
            "arch 22",
            "setnorm 1",
            "norm 80",
            "rmnotempty 0 145",
            "del 1",
            "del2 0 2",
            "rmsub 1",
            "rm 1",
            "gone ffffffff 2",
        ]
    );
}

#[test]
fn delete_with_open_handles_and_delete_on_close() {
    // MSDN DeleteFile: fails while another handle was opened without FILE_SHARE_DELETE; otherwise the file is
    // "marked for deletion on close", later CreateFile fails with ERROR_ACCESS_DENIED, and it goes at the last close.
    let l = lines(
        "vfs-delete",
        &format!(
            r#"
    uint64_t h, h2;
    h = {noshare};
    printf("d1 %llu %u\n", U(CALL(fle_imp_DeleteFileW, {p})), LE());
    CALL(fle_imp_CloseHandle, h);
    h = {share};
    printf("d2 %llu\n", U(CALL(fle_imp_DeleteFileW, {p})));
    h2 = {reopen}; printf("reopen %llx %u\n", U(h2), LE());
    CALL(fle_imp_CloseHandle, h);
    printf("gone %llx %u\n", U(CALL(fle_imp_GetFileAttributesW, {p})), LE());
    /* FILE_FLAG_DELETE_ON_CLOSE (0x04000000) */
    h = {doc};
    printf("doc-exists %d\n", CALL(fle_imp_GetFileAttributesW, {q}) != 0xFFFFFFFFull);
    CALL(fle_imp_CloseHandle, h);
    printf("doc-gone %llx\n", U(CALL(fle_imp_GetFileAttributesW, {q})));
"#,
            p = cw("C:\\dl.txt"),
            q = cw("C:\\tmp.bin"),
            noshare = create("C:\\dl.txt", GRW, 3, 2, 0x80),
            share = create("C:\\dl.txt", GRW, 7, 3, 0x80),
            reopen = create("C:\\dl.txt", GR, 7, 3, 0x80),
            doc = create("C:\\tmp.bin", GRW, 4, 2, 0x04000080),
        ),
    );
    assert_eq!(
        l,
        [
            "d1 0 32",
            "d2 1",
            "reopen ffffffffffffffff 5",
            "gone ffffffff 2",
            "doc-exists 1",
            "doc-gone ffffffff",
        ]
    );
}

#[test]
fn copy_and_move() {
    let l = lines(
        "vfs-copy",
        &format!(
            r#"
    static char b[16];
    uint32_t n;
    uint64_t h = {src};
    uint64_t ok;
    CALL(fle_imp_WriteFile, h, A("abcdef"), 6, A(&n), 0);
    CALL(fle_imp_CloseHandle, h);
    CALL(fle_imp_SetFileAttributesW, {s}, 0x2);   /* HIDDEN: attributes are copied (MSDN CopyFile) */
    printf("copy %llu\n", U(CALL(fle_imp_CopyFileW, {s}, {d}, 1)));
    printf("copyattr %llx\n", U(CALL(fle_imp_GetFileAttributesW, {d})));
    /* destination exists: bFailIfExists TRUE fails (code is an inference: 80) */
    printf("copy2 %llu %u\n", U(CALL(fle_imp_CopyFileW, {s}, {d}, 1)), LE());
    /* destination hidden/read-only and bFailIfExists FALSE: ERROR_ACCESS_DENIED (MSDN CopyFile) */
    printf("copy3 %llu %u\n", U(CALL(fle_imp_CopyFileW, {s}, {d}, 0)), LE());
    CALL(fle_imp_SetFileAttributesW, {d}, 0x80);
    printf("copy4 %llu\n", U(CALL(fle_imp_CopyFileW, {s}, {d}, 0)));
    /* missing source: ERROR_FILE_NOT_FOUND */
    printf("copy5 %llu %u\n", U(CALL(fle_imp_CopyFileW, {m}, {d}, 0)), LE());
    h = {dr}; ok = CALL(fle_imp_ReadFile, h, A(b), 16, A(&n), 0); b[n] = 0; printf("data %u %s\n", n, b);
    CALL(fle_imp_CloseHandle, h);
    /* MoveFile: the new name must not exist; the old one disappears */
    printf("mv1 %llu %u\n", U(CALL(fle_imp_MoveFileW, {s}, {d})), LE());
    printf("mv2 %llu\n", U(CALL(fle_imp_MoveFileW, {s}, {e})));
    printf("old %llx\n", U(CALL(fle_imp_GetFileAttributesW, {s})));
    printf("new %llx\n", U(CALL(fle_imp_GetFileAttributesW, {e})));
"#,
            src = create("C:\\src.txt", GRW, 0, 2, 0x80),
            s = cw("C:\\src.txt"),
            d = cw("C:\\dst.txt"),
            e = cw("C:\\moved.txt"),
            m = cw("C:\\nosuch.txt"),
            dr = create("C:\\dst.txt", GR, 7, 3, 0x80),
        ),
    );
    assert_eq!(
        l,
        [
            "copy 1",
            "copyattr 2",
            "copy2 0 80",
            "copy3 0 5",
            "copy4 1",
            "copy5 0 2",
            "data 6 abcdef",
            "mv1 0 183",
            "mv2 1",
            "old ffffffff",
            "new 2",
        ]
    );
}

// ---- directory search ----

#[test]
fn find_files_with_wildcards() {
    // MSDN FindFirstFileW / FindNextFileW / WIN32_FIND_DATAW.
    let setup = format!(
        r#"
    uint64_t h;
    CALL(fle_imp_CreateDirectoryW, {d}, 0);
    CALL(fle_imp_CreateDirectoryW, {s}, 0);
    h = {b}; CALL(fle_imp_CloseHandle, h);
    h = {a}; CALL(fle_imp_CloseHandle, h);
    h = {c}; CALL(fle_imp_CloseHandle, h);
"#,
        d = cw("C:\\fd"),
        s = cw("C:\\fd\\s"),
        b = create("C:\\fd\\b.txt", GRW, 0, 1, 0x80),
        a = create("C:\\fd\\A.TXT", GRW, 0, 1, 0x80),
        c = create("C:\\fd\\c.dat", GRW, 0, 1, 0x80),
    );
    let list = |name: &str, pat: &str| -> Vec<String> {
        let body = format!(
            r#"{setup}
    {{
        static uint64_t fd[74];
        uint64_t f = CALL(fle_imp_FindFirstFileW, {p}, A(fd));
        if (f == 0xFFFFFFFFFFFFFFFFull) {{ printf("INVALID %u\n", LE()); }}
        else {{
            do {{ printf("%s %x\n", AS((uint16_t *)((uint8_t *)fd + 44)), *(uint32_t *)fd); }}
            while (CALL(fle_imp_FindNextFileW, f, A(fd)));
            printf("END %u\n", LE());
            printf("close %llu\n", U(CALL(fle_imp_FindClose, f)));
            printf("close2 %llu %u\n", U(CALL(fle_imp_FindClose, f)), LE());
        }}
    }}
"#,
            p = cw(pat)
        );
        lines(name, &body)
    };
    let sorted = |mut v: Vec<String>| {
        let tail = v.split_off(v.len() - 3);
        v.sort();
        v.extend(tail);
        v
    };
    // "*.txt": case-insensitive match; the order is not defined by the page, so sort.
    assert_eq!(
        sorted(list("vfs-find-txt", "C:\\fd\\*.txt")),
        ["A.TXT 80", "b.txt 80", "END 18", "close 1", "close2 0 6"]
    );
    assert_eq!(sorted(list("vfs-find-q", "C:\\fd\\?.dat")), ["c.dat 80", "END 18", "close 1", "close2 0 6"]);
    // "*": "." and ".." (known Windows behaviour, not on the pages read; see import-vfs-spec.md), the files, the directory.
    assert_eq!(
        sorted(list("vfs-find-all", "C:\\fd\\*")),
        [". 10", ".. 10", "A.TXT 80", "b.txt 80", "c.dat 80", "s 10", "END 18", "close 1", "close2 0 6"]
    );
    // an exact name returns that entry; a directory name returns the directory itself (page: "C:\Windows" example)
    assert_eq!(list("vfs-find-exact", "C:\\fd\\b.txt"), ["b.txt 80", "END 18", "close 1", "close2 0 6"]);
    assert_eq!(list("vfs-find-dir", "C:\\fd\\s"), ["s 10", "END 18", "close 1", "close2 0 6"]);
    // no match: INVALID_HANDLE_VALUE and ERROR_FILE_NOT_FOUND (page); missing directory 3 (inference); trailing '\' 123 (inference)
    assert_eq!(list("vfs-find-none", "C:\\fd\\x*"), ["INVALID 2"]);
    assert_eq!(list("vfs-find-nodir", "C:\\nodir\\*"), ["INVALID 3"]);
    assert_eq!(list("vfs-find-trail", "C:\\fd\\"), ["INVALID 123"]);
}

#[test]
fn find_data_fields_and_file_sizes() {
    // WIN32_FIND_DATAW: dwFileAttributes at 0, nFileSizeHigh at 28, nFileSizeLow at 32, cFileName at 44;
    // the time fields are FILETIMEs (page). "." and ".." are skipped here by asking for an exact name.
    let l = lines(
        "vfs-finddata",
        &format!(
            r#"
    static uint64_t fd[74];
    uint32_t n;
    uint64_t h = {c};
    uint64_t f;
    CALL(fle_imp_WriteFile, h, A("0123456789"), 10, A(&n), 0);
    CALL(fle_imp_CloseHandle, h);
    f = CALL(fle_imp_FindFirstFileW, {p}, A(fd));
    printf("%u %u %u\n", *(uint32_t *)((uint8_t *)fd + 28), *(uint32_t *)((uint8_t *)fd + 32), *(uint32_t *)fd);
    printf("%s\n", AS((uint16_t *)((uint8_t *)fd + 44)));
    printf("times %d\n", *(uint64_t *)((uint8_t *)fd + 4) != 0 && *(uint64_t *)((uint8_t *)fd + 20) >= *(uint64_t *)((uint8_t *)fd + 4));
    printf("alt %u\n", *(uint16_t *)((uint8_t *)fd + 44 + 520));
    CALL(fle_imp_FindClose, f);
"#,
            c = create("C:\\ten.bin", GRW, 0, 1, 0x80),
            p = cw("C:\\ten.bin"),
        ),
    );
    assert_eq!(l, ["0 10 128", "ten.bin", "times 1", "alt 0"]);
}

#[test]
fn close_handle_rules() {
    // MSDN CloseHandle: nonzero on success, zero on failure; FindFirstFile handles use FindClose. The error code
    // for an invalid handle is an inference (6).
    let l = lines(
        "vfs-close",
        &format!(
            r#"
    uint64_t h = {c};
    uint64_t f;
    static uint64_t fd[74];
    printf("%llu\n", U(CALL(fle_imp_CloseHandle, h)));
    printf("%llu %u\n", U(CALL(fle_imp_CloseHandle, h)), LE());
    printf("%llu %u\n", U(CALL(fle_imp_CloseHandle, 0)), LE());
    printf("%llu %u\n", U(CALL(fle_imp_CloseHandle, 0xFFFFFFFFFFFFFFFFull)), LE());
    f = CALL(fle_imp_FindFirstFileW, {p}, A(fd));
    printf("find-as-file %llu %u\n", U(CALL(fle_imp_CloseHandle, f)), LE());
    printf("findclose %llu\n", U(CALL(fle_imp_FindClose, f)));
    printf("findclose-file %llu %u\n", U(CALL(fle_imp_FindClose, h)), LE());
"#,
            c = create("C:\\ch.bin", GRW, 0, 1, 0x80),
            p = cw("C:\\ch.bin"),
        ),
    );
    assert_eq!(l, ["1", "0 6", "0 6", "0 6", "find-as-file 0 6", "findclose 1", "findclose-file 0 6"]);
}

// ---- process paths ----

#[test]
fn current_directory_and_relative_paths() {
    let l = lines(
        "vfs-cwd",
        &format!(
            r#"
    uint16_t b[64];
    uint64_t n;
    uint64_t h;
    n = CALL(fle_imp_GetCurrentDirectoryW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    CALL(fle_imp_CreateDirectoryW, {w}, 0);
    printf("set %llu\n", U(CALL(fle_imp_SetCurrentDirectoryW, {w})));
    n = CALL(fle_imp_GetCurrentDirectoryW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    /* too small: the required size including the NUL (page); NULL buffer with length 0 asks for the size */
    n = CALL(fle_imp_GetCurrentDirectoryW, 2, A(b)); printf("small %llu\n", U(n));
    n = CALL(fle_imp_GetCurrentDirectoryW, 0, 0); printf("probe %llu\n", U(n));
    /* a relative name is relative to the current directory */
    h = {rel}; CALL(fle_imp_CloseHandle, h);
    printf("%llx\n", U(CALL(fle_imp_GetFileAttributesW, {abs})));
    /* relative set, then ".." */
    CALL(fle_imp_CreateDirectoryW, {inner}, 0);
    printf("set2 %llu\n", U(CALL(fle_imp_SetCurrentDirectoryW, {innerrel})));
    n = CALL(fle_imp_GetCurrentDirectoryW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    printf("up %llu\n", U(CALL(fle_imp_SetCurrentDirectoryW, {up})));
    n = CALL(fle_imp_GetCurrentDirectoryW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    /* a missing directory and a file are errors; the directory stays */
    printf("miss %llu %u\n", U(CALL(fle_imp_SetCurrentDirectoryW, {miss})), LE());
    /* the current directory cannot be removed (page: it is locked) */
    printf("rm %llu\n", U(CALL(fle_imp_RemoveDirectoryW, {w})) );
"#,
            w = cw("C:\\work"),
            rel = create("r.txt", GRW, 0, 1, 0x80),
            abs = cw("C:\\work\\r.txt"),
            inner = cw("C:\\work\\in"),
            innerrel = cw("in"),
            up = cw(".."),
            miss = cw("C:\\nowhere"),
        ),
    );
    assert_eq!(
        l,
        [
            "3 C:\\",
            "set 1",
            "7 C:\\work",
            "small 8",
            "probe 8",
            "80",
            "set2 1",
            "10 C:\\work\\in",
            "up 1",
            "7 C:\\work",
            "miss 0 2",
            "rm 0",
        ]
    );
}

#[test]
fn get_full_path_name() {
    // MSDN GetFullPathNameW: merges the current directory, resolves . and .., does not check existence, returns the
    // length without the NUL (or the size needed including it), lpFilePart is the last component.
    let l = lines(
        "vfs-fullpath",
        &format!(
            r#"
    uint16_t b[64];
    uint16_t *part = (uint16_t *)1;
    uint64_t n;
    CALL(fle_imp_CreateDirectoryW, {d}, 0);
    CALL(fle_imp_SetCurrentDirectoryW, {d});
    n = CALL(fle_imp_GetFullPathNameW, {rel}, 64, A(b), A(&part));
    printf("%llu %s %s\n", U(n), AS(b), AS(part));
    n = CALL(fle_imp_GetFullPathNameW, {abs}, 64, A(b), A(&part));
    printf("%llu %s %s\n", U(n), AS(b), AS(part));
    n = CALL(fle_imp_GetFullPathNameW, {rel}, 4, A(b), 0);
    printf("small %llu\n", U(n));
    part = (uint16_t *)1;
    n = CALL(fle_imp_GetFullPathNameW, {dirslash}, 64, A(b), A(&part));
    printf("%llu %s part=%llu\n", U(n), AS(b), U((uint64_t)(uintptr_t)part));
"#,
            d = cw("C:\\a"),
            rel = cw("x\\..\\y.txt"),
            abs = cw("c:/b/./c/../d.dat"),
            dirslash = cw("sub\\"),
        ),
    );
    assert_eq!(
        l,
        ["10 C:\\a\\y.txt y.txt", "10 C:\\b\\d.dat d.dat", "small 11", "9 C:\\a\\sub\\ part=0"]
    );
}

#[test]
fn module_file_name_is_the_virtual_exe_path_for_null_and_the_dll_path_for_0x400000() {
    // MSDN GetModuleFileNameW: NULL is the process executable; too-small buffer truncates, returns nSize and sets
    // ERROR_INSUFFICIENT_BUFFER (122); nSize 0 returns 0. The paths are the PM's values: the DLL's (decision 0018) for the handle 0x400000 and, since
    // decision 0118 (provisional), the host exe FL64.exe for NULL (it was the DLL's path for NULL until decision 0117).
    let dll = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
    let exe = "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe";
    let l = lines(
        "vfs-modname",
        r#"
    uint16_t b[260];
    uint16_t s[16];
    uint64_t n;
    n = CALL(fle_imp_GetModuleFileNameW, 0, A(b), 260); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetModuleFileNameW, 0x400000, A(b), 260); printf("%llu %s\n", U(n), AS(b));
    SETLE(0);
    n = CALL(fle_imp_GetModuleFileNameW, 0, A(s), 10); printf("trunc %llu [%s] %u\n", U(n), AS(s), LE());
    SETLE(7);
    n = CALL(fle_imp_GetModuleFileNameW, 0, A(s), 0); printf("zero %llu %u\n", U(n), LE());
    /* exactly the length + 1 of the exe path: fits, no error */
    SETLE(7);
    n = CALL(fle_imp_GetModuleFileNameW, 0, A(b), 50); printf("fit %llu %u\n", U(n), LE());
    /* one less: truncated, nSize returned, 122 */
    SETLE(7);
    n = CALL(fle_imp_GetModuleFileNameW, 0, A(b), 49); printf("short %llu %u\n", U(n), LE());
    /* the DLL's path (57 characters) fits in 58 */
    SETLE(7);
    n = CALL(fle_imp_GetModuleFileNameW, 0x400000, A(b), 58); printf("dllfit %llu %u\n", U(n), LE());
"#,
    );
    // C:\ (3) + Program Files (13) + \ + Image-Line (10) + \ + FL Studio 21 (12) + \ = 41, then FLEngine_x64.dll (16) = 57 or FL64.exe (8) = 49
    assert_eq!(dll.len(), 57);
    assert_eq!(exe.len(), 49);
    assert_eq!(l[0], format!("49 {exe}"));
    assert_eq!(l[1], format!("57 {dll}"));
    assert_eq!(l[2], format!("trunc 10 [{}] 122", &exe[..9]));
    assert_eq!(l[3], "zero 0 7");
    assert_eq!(l[4], "fit 49 7");
    assert_eq!(l[5], "short 49 122");
    assert_eq!(l[6], "dllfit 57 7");
}

#[test]
fn temp_windows_and_system_directories() {
    // MSDN GetTempPathW: TMP, TEMP, USERPROFILE, then the Windows directory, ending with a backslash. The virtual
    // environment starts empty, so the Windows directory (decision 0018: C:\Windows) is the answer at first.
    let l = lines(
        "vfs-temp",
        &format!(
            r#"
    uint16_t b[64];
    uint64_t n;
    n = CALL(fle_imp_GetTempPathW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    CALL(fle_imp_SetEnvironmentVariableW, W("USERPROFILE"), {up});
    n = CALL(fle_imp_GetTempPathW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    CALL(fle_imp_SetEnvironmentVariableW, W("temp"), {t2});
    n = CALL(fle_imp_GetTempPathW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    CALL(fle_imp_SetEnvironmentVariableW, W("TMP"), {t1});
    n = CALL(fle_imp_GetTempPathW, 64, A(b)); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetTempPathW, 3, A(b)); printf("small %llu\n", U(n));
    n = CALL(fle_imp_GetWindowsDirectoryW, A(b), 64); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetSystemDirectoryW, A(b), 64); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetSystemDirectoryW, A(b), 5); printf("small %llu\n", U(n));
"#,
            up = cw("C:\\Users\\u"),
            t2 = cw("C:\\T2\\"),
            t1 = cw("C:\\T1"),
        ),
    );
    assert_eq!(
        l,
        [
            "11 C:\\Windows\\",
            "11 C:\\Users\\u\\",
            "6 C:\\T2\\",
            "6 C:\\T1\\",
            "small 7",
            "10 C:\\Windows",
            "19 C:\\Windows\\System32",
            "small 20",
        ]
    );
}

// ---- GetFileAttributesExW, SetFileTime ----

#[test]
fn get_file_attributes_ex_and_set_file_time() {
    let l = lines(
        "vfs-attrex",
        &format!(
            r#"
    uint64_t data[5];
    uint64_t h = {c};
    uint64_t ft = 0x01D6000000000000ull, zero = 0;
    uint32_t n;
    CALL(fle_imp_WriteFile, h, A("abc"), 3, A(&n), 0);
    printf("set %llu\n", U(CALL(fle_imp_SetFileTime, h, 0, 0, A(&ft))));
    /* a FILETIME of 0/0 leaves the time alone (MSDN SetFileTime) */
    printf("set0 %llu\n", U(CALL(fle_imp_SetFileTime, h, A(&zero), A(&zero), A(&zero))));
    CALL(fle_imp_CloseHandle, h);
    printf("ex %llu\n", U(CALL(fle_imp_GetFileAttributesExW, {p}, 0, A(data))));
    printf("%x %llx %u %u\n", *(uint32_t *)data, *(uint64_t *)((uint8_t *)data + 20), *(uint32_t *)((uint8_t *)data + 28), *(uint32_t *)((uint8_t *)data + 32));
    printf("miss %llu %u\n", U(CALL(fle_imp_GetFileAttributesExW, {m}, 0, A(data))), LE());
"#,
            c = create("C:\\ex.bin", GRW, 0, 1, 0x80),
            p = cw("C:\\ex.bin"),
            m = cw("C:\\nope.bin"),
        ),
    );
    assert_eq!(l, ["set 1", "set0 1", "ex 1", "80 1d6000000000000 0 3", "miss 0 2"]);
}

// ---- stops: input the pages leave undefined ----

#[test]
fn get_temp_file_name_forms_the_name_of_the_page_for_a_nonzero_unique_number() {
    // decision 0132 (wall 22, PROVISIONAL). Learn (GetTempFileNameW): the form is "<path>\\<pre><uuuu>.TMP" with "<pre>: First three letters of the lpPrefixString string" and "<uuuu>: Hexadecimal value of uUnique"; "Only the lower
    // 16 bits of the uUnique parameter are used"; "If the uUnique parameter is nonzero, the return value specifies that same number"; "If uUnique is not zero, you must create the file yourself" (no file is made); failure is zero with
    // ERROR_BUFFER_OVERFLOW "if the length of the string pointed to by the lpPathName parameter is more than MAX_PATH-14 characters". The choices without a source (S-gettempfilename-name): no second backslash after a path that ends with
    // one, %04X (upper case, zero padded), the extension as the page writes it. The guest's call: ("C:\\Windows\\", "ILNewsDownload", 0x755f60ca).
    let long = "C:\\".to_string() + &"a".repeat(244);
    let l = lines(
        "vfs-tempname",
        &format!(
            r#"
    uint16_t b[300];
    uint64_t n;
    SETLE(0x1234);
    n = CALL(fle_imp_GetTempFileNameW, {p1}, {pre}, 0x755f60ca, A(b)); printf("%llx [%s] le %u\n", U(n), AS(b), LE());
    n = CALL(fle_imp_GetTempFileNameW, {p2}, {pre}, 0x1, A(b)); printf("%llx [%s]\n", U(n), AS(b));
    n = CALL(fle_imp_GetTempFileNameW, {p1}, {short}, 0xABCD0000u | 0x0F, A(b)); printf("%llx [%s]\n", U(n), AS(b));
    n = CALL(fle_imp_GetTempFileNameW, {p1}, {pre}, 0x7B, A(b)); printf("%llx [%s]\n", U(n), AS(b));
    printf("no file was made: attributes %llx\n", U(CALL(fle_imp_GetFileAttributesW, {made})));
    n = CALL(fle_imp_GetTempFileNameW, {pl}, {pre}, 5, A(b)); printf("long path: %llu le %u\n", U(n), LE());
    n = CALL(fle_imp_GetTempFileNameW, {pm}, {pre}, 5, A(b)); printf("246 characters: %llx\n", U(n));
"#,
            p1 = cw("C:\\Windows\\"),
            p2 = cw("C:\\Windows"),
            pre = cw("ILNewsDownload"),
            short = cw("A"),
            made = cw("C:\\Windows\\ILN60CA.TMP"),
            pl = cw(&long),
            pm = cw(&("C:\\".to_string() + &"a".repeat(243))),
        ),
    );
    assert_eq!(
        l,
        [
            "755f60ca [C:\\Windows\\ILN60CA.TMP] le 4660",
            "1 [C:\\Windows\\ILN0001.TMP]",
            "abcd000f [C:\\Windows\\A000F.TMP]",
            "7b [C:\\Windows\\ILN007B.TMP]",
            "no file was made: attributes ffffffff",
            "long path: 0 le 111",
            "246 characters: 5",
        ]
    );
}

#[test]
fn undefined_input_stops_the_run() {
    let setup = format!(
        r#"
    uint32_t cnt;
    uint64_t h = {c};
    static char buf[16];
    static uint64_t fd[74];
    CALL(fle_imp_CreateDirectoryW, {d}, 0);
"#,
        c = create("C:\\f.txt", GRW, 0, 2, 0x80),
        d = cw("C:\\dir"),
    );
    let long = "C:\\".to_string() + &"a".repeat(300);
    let cases: Vec<(&str, String)> = vec![
        ("secattr", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 1, 3, 0x80, 0);", cw("C:\\x"))),
        ("template", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 1);", cw("C:\\x"))),
        ("disposition-0", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 0, 0x80, 0);", cw("C:\\x"))),
        ("disposition-9", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 9, 0x80, 0);", cw("C:\\x"))),
        ("overlapped-flag", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x40000080u, 0);", cw("C:\\f.txt"))),
        ("generic-execute", format!("CALL(fle_imp_CreateFileW, {}, 0x20000000u, 7, 0, 3, 0x80, 0);", cw("C:\\f.txt"))),
        ("null-name", "CALL(fle_imp_CreateFileW, 0, 0x80000000u, 7, 0, 3, 0x80, 0);".into()),
        ("empty-name", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw(""))),
        ("unc", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw("\\\\srv\\share\\f"))),
        ("verbatim-prefix", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw("\\\\?\\C:\\f.txt"))),
        ("other-drive", format!("CALL(fle_imp_GetFileAttributesW, {});", cw("D:\\f.txt"))),
        ("drive-relative", format!("CALL(fle_imp_GetFileAttributesW, {});", cw("C:f.txt"))),
        ("device-name", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw("C:\\NUL"))),
        ("device-name-ext", format!("CALL(fle_imp_GetFileAttributesW, {});", cw("C:\\com1.txt"))),
        ("stream", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw("C:\\f.txt:s"))),
        ("wildcard-in-create", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x80, 0);", cw("C:\\f*"))),
        ("trailing-space", format!("CALL(fle_imp_GetFileAttributesW, {});", cw("C:\\f.txt "))),
        ("trailing-dot", format!("CALL(fle_imp_GetFileAttributesW, {});", cw("C:\\f."))),
        ("path-too-long", format!("CALL(fle_imp_GetFileAttributesW, {});", cw(&long))),
        ("dir-handle", format!("CALL(fle_imp_CreateFileW, {}, {GR}, 7, 0, 3, 0x02000000, 0);", cw("C:\\dir"))),
        ("read-overlapped", "CALL(fle_imp_ReadFile, h, A(buf), 4, A(&cnt), 1);".into()),
        ("read-null-count", "CALL(fle_imp_ReadFile, h, A(buf), 4, 0, 0);".into()),
        // decision 0126 (wall 95, S-readfile-zero): old: ("read-zero", ReadFile of 0 bytes) stopped; it follows the two rules of the page now (at or beyond the end: ERROR_HANDLE_EOF, before it: success with 0 bytes; tests/wall_fr11.rs)
        ("write-overlapped", "CALL(fle_imp_WriteFile, h, A(buf), 4, A(&cnt), 1);".into()),
        ("write-null-count", "CALL(fle_imp_WriteFile, h, A(buf), 4, 0, 0);".into()),
        ("seek-method", "CALL(fle_imp_SetFilePointer, h, 0, 0, 3);".into()),
        // 0x7FFFFFFF + 0x7FFFFFFF + 1 = 0xFFFFFFFF: valid as a low part but ambiguous with failure (MSDN SetFilePointer)
        (
            "seek-ffffffff",
            "CALL(fle_imp_SetFilePointer, h, 0x7FFFFFFFull, 0, 0); CALL(fle_imp_SetFilePointer, h, 0x7FFFFFFFull, 0, 1); CALL(fle_imp_SetFilePointer, h, 1, 0, 1);".into(),
        ),
        ("close-foreign-handle", "CALL(fle_imp_CloseHandle, 0x1234);".into()),
        ("file-foreign-handle", "CALL(fle_imp_GetFileSize, 0x77, 0);".into()),
        ("find-null-data", format!("CALL(fle_imp_FindFirstFileW, {}, 0);", cw("C:\\*"))),
        ("find-wildcard-dir", format!("CALL(fle_imp_FindFirstFileW, {}, A(fd));", cw("C:\\d*\\x"))),
        ("setattr-directory-bit", format!("CALL(fle_imp_SetFileAttributesW, {}, 0x10);", cw("C:\\f.txt"))),
        ("setattr-root", format!("CALL(fle_imp_SetFileAttributesW, {}, 1);", cw("C:\\"))),
        ("attrex-level", format!("CALL(fle_imp_GetFileAttributesExW, {}, 1, A(fd));", cw("C:\\f.txt"))),
        ("mkdir-secattr", format!("CALL(fle_imp_CreateDirectoryW, {}, 1);", cw("C:\\y"))),
        ("modname-other-module", "CALL(fle_imp_GetModuleFileNameW, 0x7FF800090000ull, A(buf), 8);".into()),
        ("settime-suppress", "{ uint64_t ff = ~0ull; CALL(fle_imp_SetFileTime, h, 0, 0, A(&ff)); }".into()),
        ("move-into-self", format!("CALL(fle_imp_MoveFileW, {}, {});", cw("C:\\dir"), cw("C:\\dir\\sub"))),
        // decision 0132 (wall 22): GetTempFileNameW answers only a nonzero uUnique and a path that the page describes
        ("tempname-unique-0", format!("CALL(fle_imp_GetTempFileNameW, {}, {}, 0, A(buf));", cw("C:\\"), cw("abc"))),
        ("tempname-null-path", format!("CALL(fle_imp_GetTempFileNameW, 0, {}, 1, A(buf));", cw("abc"))),
        ("tempname-null-prefix", format!("CALL(fle_imp_GetTempFileNameW, {}, 0, 1, A(buf));", cw("C:\\"))),
        ("tempname-null-buffer", format!("CALL(fle_imp_GetTempFileNameW, {}, {}, 1, 0);", cw("C:\\"), cw("abc"))),
        ("tempname-empty-path", format!("CALL(fle_imp_GetTempFileNameW, {}, {}, 1, A(buf));", cw(""), cw("abc"))),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (*a, b.as_str())).collect();
    assert_aborts("vfs-abort", &setup, &cases);
}
