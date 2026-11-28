//! Tests for the virtual environment (decision 0018): runtime/fle_env.c. Expected values come from the Microsoft pages
//! (GetEnvironmentVariableW, SetEnvironmentVariable, ExpandEnvironmentStringsW, GetCommandLineW, GetStartupInfoW,
//! STARTUPINFOW, Environment Variables), listed in docs/investigations/import-vfs-spec.md. Items marked "inference"
//! in the C comments are not tested as facts. The host environment is never read.

mod fsx_common;
use fsx_common::*;

// Issue #1412: the block starts with the Windows session's standard variables. The values are the virtual paths and the
// SHGetFolderPathW answers of this environment (fle_env.h); PATHEXT, OS, NUMBER_OF_PROCESSORS and PROCESSOR_ARCHITECTURE are
// the Windows values of this environment (choices: no Microsoft page is read for them here).
const STANDARD_ENV: [(&str, &str); 22] = [
    ("SystemRoot", r"C:\Windows"),
    ("SystemDrive", "C:"),
    ("windir", r"C:\Windows"),
    ("USERNAME", "user"),
    ("USERPROFILE", r"C:\Users\user"),
    ("APPDATA", r"C:\Users\user\AppData\Roaming"),
    ("LOCALAPPDATA", r"C:\Users\user\AppData\Local"),
    ("ProgramData", r"C:\ProgramData"),
    ("ProgramFiles", r"C:\Program Files"),
    ("ProgramFiles(x86)", r"C:\Program Files"),
    ("CommonProgramFiles", r"C:\Program Files\Common Files"),
    ("TEMP", r"C:\Users\user\AppData\Local\Temp"),
    ("TMP", r"C:\Users\user\AppData\Local\Temp"),
    ("COMPUTERNAME", "DESKTOP-0000001"),
    ("OS", "Windows_NT"),
    ("PATH", r"C:\Windows\System32;C:\Windows"),
    ("PATHEXT", ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC"),
    ("NUMBER_OF_PROCESSORS", "1"),
    ("PROCESSOR_ARCHITECTURE", "AMD64"),
    ("HOMEDRIVE", "C:"),
    ("HOMEPATH", r"\Users\user"),
    ("USERDOMAIN", "DESKTOP-0000001"),
];

#[test]
fn environment_starts_with_the_windows_session_variables() {
    // Decision 0018 (the host is never read): the names that only other systems have are not in the block.
    assert!(std::env::var_os("PATH").is_some());
    let mut body = String::from("    static uint16_t b[512];\n");
    for (name, _) in STANDARD_ENV {
        body.push_str(&format!("    {{ uint64_t n = CALL(fle_imp_GetEnvironmentVariableW, W(\"{name}\"), A(b), 512); printf(\"{name} %llu %s\\n\", U(n), AS(b)); }}\n"));
    }
    body.push_str("    { uint64_t n = CALL(fle_imp_GetEnvironmentVariableW, W(\"systemroot\"), A(b), 512); printf(\"lower %llu %s\\n\", U(n), AS(b)); }\n");
    body.push_str("    { int i; static const char *none[] = { \"HOME\", \"USER\", \"SHELL\", \"PWD\", \"LANG\" }; for (i = 0; i < 5; i++) { uint16_t n[16]; int k; for (k = 0; none[i][k]; k++) n[k] = (uint8_t)none[i][k]; n[k] = 0; SETLE(0); printf(\"%s %llu %u\\n\", none[i], U(CALL(fle_imp_GetEnvironmentVariableW, A(n), A(b), 512)), LE()); } }\n");
    let l = lines("env-standard", &body);
    let mut want: Vec<String> = STANDARD_ENV.iter().map(|(n, v)| format!("{n} {} {v}", v.len())).collect();
    want.push(format!("lower {} {}", r"C:\Windows".len(), r"C:\Windows"));
    for n in ["HOME", "USER", "SHELL", "PWD", "LANG"] {
        want.push(format!("{n} 0 203")); // ERROR_ENVVAR_NOT_FOUND
    }
    assert_eq!(l, want);
}

#[test]
fn standard_values_are_the_shgetfolderpath_answers() {
    // The values of the block come from the same constants as the SHGetFolderPathW answers (fle_env.h): each pair must agree.
    let l = lines(
        "env-shgfp",
        r#"
    static uint16_t sh[260], ev[260];
    static const struct { int csidl; const char *name; } pairs[] = {
        { 0x1a, "APPDATA" }, { 0x1c, "LOCALAPPDATA" }, { 0x23, "ProgramData" }, { 0x26, "ProgramFiles" }, { 0x2a, "ProgramFiles(x86)" },
        { 0x2b, "CommonProgramFiles" }, { 0x24, "SystemRoot" }, { 0x28, "USERPROFILE" },
    };
    int i;
    for (i = 0; i < 8; i++) {
        uint16_t n[32]; int k;
        for (k = 0; pairs[i].name[k]; k++) n[k] = (uint8_t)pairs[i].name[k];
        n[k] = 0;
        CALL(fle_imp_SHGetFolderPathW, 0, pairs[i].csidl, 0, 0, A(sh));
        CALL(fle_imp_GetEnvironmentVariableW, A(n), A(ev), 260);
        printf("%s|%s\n", AS(sh), AS(ev));
    }
"#,
    );
    assert_eq!(l.len(), 8);
    for line in &l {
        let (sh, ev) = line.split_once('|').unwrap();
        assert_eq!(sh, ev, "the SHGetFolderPathW answer and the block value differ");
    }
}

#[test]
fn environment_names_are_case_insensitive_and_only_windows_names_are_set() {
    // The names of the test: HOME, USER, SHELL, PWD and LANG are the names of other systems (none is set here).
    let l = lines(
        "env-case",
        r#"
    static uint16_t b[64];
    uint64_t n;
    n = CALL(fle_imp_GetEnvironmentVariableW, W("PATH"), A(b), 64); printf("%llu\n", U(n));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("path"), A(b), 64); printf("%llu %s\n", U(n), AS(b));
"#,
    );
    assert_eq!(l, ["30".to_string(), r"30 C:\Windows\System32;C:\Windows".to_string()]);
}

#[test]
fn one_block_for_set_get_expand_and_the_two_environment_strings() {
    // Issue #1412: Set, Get, Expand, GetEnvironmentStringsW and GetEnvironmentStrings (the ANSI export) read and write one block.
    // The block has the 22 standard names and the one that Set adds; the two blocks hold the same characters (ASCII names and values).
    let l = lines(
        "env-one-block",
        r#"
    static uint16_t b[128];
    uint64_t n;
    uint16_t *wb; char *ab;
    size_t i, entries = 0; int same = 1, zed = 0, azed = 0;
    CALL(fle_imp_SetEnvironmentVariableW, W("Zed"), W("x"));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("ZED"), A(b), 128); printf("get %llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%SystemRoot%\\system32\\%zed%"), A(b), 128); printf("expand %llu %s\n", U(n), AS(b));
    wb = (uint16_t *)(uintptr_t)CALL(fle_imp_GetEnvironmentStringsW);
    ab = (char *)(uintptr_t)CALL(fle_imp_GetEnvironmentStrings);
    for (i = 0; wb[i]; i++) {
        size_t s = i;
        entries++;
        if (wb[s] == 'Z' && wb[s + 1] == 'e' && wb[s + 2] == 'd' && wb[s + 3] == '=' && wb[s + 4] == 'x' && wb[s + 5] == 0) zed = 1;
        while (wb[i]) i++;
    }
    for (i = 0;; i++) {
        if ((uint16_t)(uint8_t)ab[i] != wb[i]) { same = 0; break; }
        if (wb[i] == 0 && wb[i + 1] == 0) break;
    }
    for (i = 0; ab[i]; ) { if (ab[i] == 'Z' && ab[i + 1] == 'e' && ab[i + 2] == 'd' && ab[i + 3] == '=' && ab[i + 4] == 'x' && ab[i + 5] == 0) azed = 1; while (ab[i]) i++; i++; }
    printf("entries %zu zed %d ansi-zed %d same %d\n", entries, zed, azed, same);
    CALL(fle_imp_FreeEnvironmentStringsW, A(wb));
    CALL(fle_imp_FreeEnvironmentStringsA, A(ab));
    printf("delete %llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("zed"), 0)));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("Zed"), A(b), 128); printf("gone %llu\n", U(n));
    wb = (uint16_t *)(uintptr_t)CALL(fle_imp_GetEnvironmentStringsW);
    entries = 0;
    for (i = 0; wb[i]; i++) { entries++; while (wb[i]) i++; }
    printf("entries-after %zu\n", entries);
"#,
    );
    assert_eq!(
        l,
        [
            "get 1 x",
            r"expand 22 C:\Windows\system32\x",
            "entries 23 zed 1 ansi-zed 1 same 1",
            "delete 1",
            "gone 0",
            "entries-after 22",
        ]
    );
}

#[test]
fn fl_variables_set_by_the_guest_are_seen_by_get_and_expand() {
    // Issue #1412 (seen in the engine's log of the DEDZII open, run 45541813190): FL sets its own variables with SetEnvironmentVariableW when it starts. The
    // values have no trailing backslash (FLStudioFactoryData is the install folder, so %FLStudioFactoryData%\Data\... has one backslash).
    // Get and ExpandEnvironmentStringsW read them from the same block.
    let l = lines(
        "env-fl",
        r#"
    static uint16_t b[256];
    uint64_t n;
    CALL(fle_imp_SetEnvironmentVariableW, W("FLStudioFactoryData"), W("C:\\Program Files\\Image-Line\\FL Studio 21"));
    CALL(fle_imp_SetEnvironmentVariableW, W("FLStudioUserData"), W("C:\\Users\\user\\Documents\\Image-Line\\FL Studio"));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%FLStudioFactoryData%\\Data\\Patches\\x.wav"), A(b), 256); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%FLStudioUserData%\\Presets\\"), A(b), 256); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("flstudiouserdata"), A(b), 256); printf("%llu %s\n", U(n), AS(b));
    printf("%llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("FLStudioUserData"), 0)));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%FLStudioUserData%"), A(b), 256); printf("%llu %s\n", U(n), AS(b));
"#,
    );
    assert_eq!(
        l,
        [
            r"60 C:\Program Files\Image-Line\FL Studio 21\Data\Patches\x.wav",
            r"54 C:\Users\user\Documents\Image-Line\FL Studio\Presets\",
            r"44 C:\Users\user\Documents\Image-Line\FL Studio",
            "1",
            "19 %FLStudioUserData%",
        ]
    );
}

#[test]
fn observation_lines_for_the_first_200_calls_of_each_api() {
    // Issue #1412 (observation, always on): the first 200 calls of each environment API write one line to stderr: the API, the name
    // (or the expanded text), what happened and the length. Only the first 200 of each API are written.
    let o = run(
        "env-observe",
        r#"
    static uint16_t b[64];
    int i;
    for (i = 0; i < 205; i++) CALL(fle_imp_GetEnvironmentVariableW, W("PATH"), A(b), 64);
    for (i = 0; i < 3; i++) CALL(fle_imp_ExpandEnvironmentStringsW, W("x"), A(b), 64);
    CALL(fle_imp_SetEnvironmentVariableW, W("FLStudioFactoryData"), W("C:\\FL"));
    CALL(fle_imp_SetEnvironmentVariableW, W("FLStudioFactoryData"), 0);
"#,
    );
    assert!(o.ok, "{}", o.stderr);
    let get: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("env: GetEnvironmentVariableW")).collect();
    assert_eq!(get.len(), 200);
    assert_eq!(get[0], r#"env: GetEnvironmentVariableW "PATH" found 30"#);
    let expand: Vec<&str> = o.stderr.lines().filter(|l| l.starts_with("env: ExpandEnvironmentStringsW")).collect();
    assert_eq!(expand, [r#"env: ExpandEnvironmentStringsW "x" expanded 1"#; 3]);
    assert!(o.stderr.contains(r#"env: SetEnvironmentVariableW "FLStudioFactoryData" set 5"#), "{}", o.stderr);
    assert!(o.stderr.contains(r#"env: SetEnvironmentVariableW "FLStudioFactoryData" deleted 0"#), "{}", o.stderr);
}

#[test]
fn set_get_replace_delete() {
    // GetEnvironmentVariableW: length without the NUL on success; too small -> required size including the NUL.
    // SetEnvironmentVariable: creates if missing; NULL value deletes. Names are compared without case (Windows
    // environment names are case-insensitive: MSDN Environment Variables page does not say it; this runtime
    // follows the commonly known behaviour, unverified here).
    let l = lines(
        "env-basic",
        r#"
    static uint16_t b[64];
    uint64_t n;
    printf("set %llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("Foo"), W("bar"))));
    SETLE(0); n = CALL(fle_imp_GetEnvironmentVariableW, W("Foo"), A(b), 64); printf("get %llu %s %u\n", U(n), AS(b), LE());
    n = CALL(fle_imp_GetEnvironmentVariableW, W("FOO"), A(b), 64); printf("case %llu %s\n", U(n), AS(b));
    /* buffer exactly the length: too small by the NUL -> returns 4 */
    n = CALL(fle_imp_GetEnvironmentVariableW, W("foo"), A(b), 3); printf("small %llu\n", U(n));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("foo"), A(b), 4); printf("fit %llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("foo"), 0, 0); printf("probe %llu\n", U(n));
    printf("replace %llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("FOO"), W("longer value"))));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("Foo"), A(b), 64); printf("get2 %llu %s\n", U(n), AS(b));
    printf("del %llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("foo"), 0)));
    SETLE(0); n = CALL(fle_imp_GetEnvironmentVariableW, W("Foo"), A(b), 64); printf("gone %llu %u\n", U(n), LE());
"#,
    );
    assert_eq!(
        l,
        [
            "set 1",
            "get 3 bar 0",
            "case 3 bar",
            "small 4",
            "fit 3 bar",
            "probe 4",
            "replace 1",
            "get2 12 longer value",
            "del 1",
            "gone 0 203",
        ]
    );
}

#[test]
fn many_variables_and_a_long_value() {
    // The documented maximum for a user variable is 32,767 characters.
    let long = "x".repeat(32767);
    let body = format!(
        r#"
    static uint16_t name[16];
    static uint16_t val[32768];
    static uint16_t out[40000];
    int i; uint64_t n; int bad = 0;
    for (i = 0; i < 200; i++) {{
        int k;
        char t[16]; snprintf(t, sizeof t, "VAR%d", i);
        for (k = 0; t[k]; k++) name[k] = (uint8_t)t[k];
        name[k] = 0;
        val[0] = (uint16_t)('a' + i % 26); val[1] = 0;
        CALL(fle_imp_SetEnvironmentVariableW, A(name), A(val));
    }}
    for (i = 0; i < 200; i++) {{
        int k;
        char t[16]; snprintf(t, sizeof t, "var%d", i);
        for (k = 0; t[k]; k++) name[k] = (uint8_t)t[k];
        name[k] = 0;
        n = CALL(fle_imp_GetEnvironmentVariableW, A(name), A(out), 8);
        if (n != 1 || out[0] != (uint16_t)('a' + i % 26)) bad++;
    }}
    printf("bad=%d\n", bad);
    for (i = 0; i < 32767; i++) val[i] = 'x';
    val[32767] = 0;
    printf("set %llu\n", U(CALL(fle_imp_SetEnvironmentVariableW, W("BIG"), A(val))));
    n = CALL(fle_imp_GetEnvironmentVariableW, W("BIG"), A(out), 40000);
    printf("big %llu %d\n", U(n), out[32766] == 'x' && out[32767] == 0);
"#
    );
    let l = lines("env-many", &body);
    assert_eq!(l, ["bad=0", "set 1", format!("big {} 1", long.len()).as_str()]);
}

#[test]
fn expand_environment_strings() {
    // ExpandEnvironmentStringsW: %name% replaced by the value, case ignored; an unknown name stays as written;
    // the return value counts the NUL; a buffer that is too small gets the required size.
    let l = lines(
        "env-expand",
        r#"
    static uint16_t b[128];
    uint64_t n;
    CALL(fle_imp_SetEnvironmentVariableW, W("Root"), W("C:\\Data"));
    CALL(fle_imp_SetEnvironmentVariableW, W("Sub"), W("x"));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%ROOT%\\file.txt"), A(b), 128); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%root%%sub%%Sub%"), A(b), 128); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("a %Unknown% b"), A(b), 128); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("plain"), A(b), 128); printf("%llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W(""), A(b), 128); printf("%llu [%s]\n", U(n), AS(b));
    /* too small: nothing is written, the size needed (with the NUL) is returned */
    b[0] = 'Q'; b[1] = 0;
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%ROOT%"), A(b), 3); printf("small %llu %s\n", U(n), AS(b));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%ROOT%"), 0, 0); printf("probe %llu\n", U(n));
    n = CALL(fle_imp_ExpandEnvironmentStringsW, W("%ROOT%"), A(b), 8); printf("exact %llu %s\n", U(n), AS(b));
"#,
    );
    assert_eq!(
        l,
        [
            // "C:\Data\file.txt" is 16 characters; the return value counts the NUL (MSDN ExpandEnvironmentStringsW)
            "17 C:\\Data\\file.txt",
            "10 C:\\Dataxx",
            "14 a %Unknown% b",
            "6 plain",
            "1 []",
            "small 8 Q",
            "probe 8",
            "exact 8 C:\\Data",
        ]
    );
}

#[test]
fn command_line_and_startup_info() {
    // GetCommandLineW: a pointer owned by the system (stable); here the quoted virtual DLL path (decision 0018).
    // GetStartupInfoW: no creating process, so every member is zero except cb (0x68 on x64: members of STARTUPINFOW).
    let path = "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll";
    let l = lines(
        "env-cmdline",
        r#"
    uint64_t p1 = CALL(fle_imp_GetCommandLineW), p2 = CALL(fle_imp_GetCommandLineW);
    uint8_t si[0x68];
    int i, nz = 0;
    printf("same %d\n", p1 == p2 && p1 != 0);
    printf("%s\n", AS((uint16_t *)p1));
    memset(si, 0xAA, sizeof si);
    *(uint32_t *)si = 0x68;
    CALL(fle_imp_GetStartupInfoW, A(si));
    printf("cb %x\n", *(uint32_t *)si);
    for (i = 4; i < 0x68; i++) nz += si[i] != 0;
    printf("nonzero %d\n", nz);
"#,
    );
    assert_eq!(l, ["same 1".to_string(), format!("\"{path}\""), "cb 68".into(), "nonzero 0".into()]);
}

#[test]
fn startup_info_does_not_read_the_incoming_cb() {
    // Decision 0083 (provisional): the GetStartupInfoW page makes lpStartupInfo an [out] parameter and states no rule
    // for the incoming cb, so whatever cb holds (the CRT of dsp_ippv2_x64.dll passes an uninitialised stack buffer) the
    // whole structure (0x68 bytes) is zeroed and cb is set to 0x68. cbReserved2 (+0x42) = 0 and lpReserved2 (+0x48) =
    // NULL (no inherited handles). Offsets from the member types (see sizeof_startupinfow_from_member_types).
    // 16 guard bytes after the structure must stay untouched.
    let l = lines(
        "env-startup-cb",
        r#"
    static const struct { const char *label; uint32_t cb; int set; } cs[] = {
        { "cb44", 0x44, 1 }, { "cb0", 0, 1 }, { "cb-uninit", 0, 0 }, { "cb1000", 0x1000, 1 },
    };
    int i;
    for (i = 0; i < 4; i++) {
        uint8_t buf[0x68 + 16];
        uint16_t cbres2; uint64_t lpres2;
        unsigned k, nz = 0, guard = 0;
        memset(buf, 0xAA, sizeof buf);
        if (cs[i].set) memcpy(buf, &cs[i].cb, 4);
        CALL(fle_imp_GetStartupInfoW, A(buf));
        for (k = 4; k < 0x68; k++) nz += buf[k] != 0;
        for (k = 0x68; k < sizeof buf; k++) guard += buf[k] != 0xAA;
        memcpy(&cbres2, buf + 0x42, 2);
        memcpy(&lpres2, buf + 0x48, 8);
        printf("%s cb %x nonzero %u cbReserved2 %u lpReserved2 %llx guard %u\n", cs[i].label, *(uint32_t *)buf, nz, cbres2, U(lpres2), guard);
    }
"#,
    );
    assert_eq!(
        l,
        [
            "cb44 cb 68 nonzero 0 cbReserved2 0 lpReserved2 0 guard 0",
            "cb0 cb 68 nonzero 0 cbReserved2 0 lpReserved2 0 guard 0",
            "cb-uninit cb 68 nonzero 0 cbReserved2 0 lpReserved2 0 guard 0",
            "cb1000 cb 68 nonzero 0 cbReserved2 0 lpReserved2 0 guard 0",
        ]
    );
}

#[test]
fn sizeof_startupinfow_from_member_types() {
    // STARTUPINFOW (MSDN): DWORD cb; LPWSTR x3; DWORD x8; WORD x2; LPBYTE; HANDLE x3. On x64 with natural alignment:
    // cb@0 (4) pad@4 (4), lpReserved@8, lpDesktop@16, lpTitle@24, dwX@32 ... dwFlags@64? computed independently:
    let mut off = 0u32;
    off += 4; // cb
    off += 4; // padding to 8
    off += 3 * 8; // lpReserved, lpDesktop, lpTitle
    off += 8 * 4; // dwX dwY dwXSize dwYSize dwXCountChars dwYCountChars dwFillAttribute dwFlags
    off += 2 * 2; // wShowWindow cbReserved2
    off += 4; // padding to 8
    off += 8; // lpReserved2
    off += 3 * 8; // hStdInput hStdOutput hStdError
    assert_eq!(off, 0x68);
}

#[test]
fn undefined_input_stops_the_run() {
    let setup = r#"
    static uint16_t b[8];
    CALL(fle_imp_SetEnvironmentVariableW, W("Here"), W("v"));
"#;
    let long = "a".repeat(40000);
    let cases: Vec<(&str, String)> = vec![
        ("get-null-name", "CALL(fle_imp_GetEnvironmentVariableW, 0, A(b), 8);".into()),
        ("get-empty-name", "CALL(fle_imp_GetEnvironmentVariableW, W(\"\"), A(b), 8);".into()),
        ("get-equals-in-name", "CALL(fle_imp_GetEnvironmentVariableW, W(\"A=B\"), A(b), 8);".into()),
        ("get-null-buffer-room", "CALL(fle_imp_GetEnvironmentVariableW, W(\"Here\"), 0, 8);".into()),
        ("set-null-name", "CALL(fle_imp_SetEnvironmentVariableW, 0, W(\"v\"));".into()),
        ("set-empty-name", "CALL(fle_imp_SetEnvironmentVariableW, W(\"\"), W(\"v\"));".into()),
        ("set-equals-in-name", "CALL(fle_imp_SetEnvironmentVariableW, W(\"A=B\"), W(\"v\"));".into()),
        ("delete-missing", "CALL(fle_imp_SetEnvironmentVariableW, W(\"Missing\"), 0);".into()),
        ("set-too-long", format!("{{ static uint16_t v[40001]; int i; for (i = 0; i < 40000; i++) v[i] = 'a'; CALL(fle_imp_SetEnvironmentVariableW, W(\"L\"), A(v)); }} /* {} */", long.len())),
        ("expand-null-src", "CALL(fle_imp_ExpandEnvironmentStringsW, 0, A(b), 8);".into()),
        ("expand-unclosed", "CALL(fle_imp_ExpandEnvironmentStringsW, W(\"%Here\"), A(b), 8);".into()),
        ("expand-empty-name", "CALL(fle_imp_ExpandEnvironmentStringsW, W(\"a%%b\"), A(b), 8);".into()),
        ("expand-null-dst-with-size", "CALL(fle_imp_ExpandEnvironmentStringsW, W(\"x\"), 0, 4);".into()),
        ("expand-same-buffer", "{ static uint16_t s[8] = { 'x', 0 }; CALL(fle_imp_ExpandEnvironmentStringsW, A(s), A(s), 8); }".into()),
        // NULL has no rule on the page: still a stop. (The old "startup-wrong-cb" case, cb = 0x44, is now a normal
        // return: see startup_info_does_not_read_the_incoming_cb, decision 0083.)
        ("startup-null", "CALL(fle_imp_GetStartupInfoW, 0);".into()),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (*a, b.as_str())).collect();
    assert_aborts("env-abort", setup, &cases);
}
