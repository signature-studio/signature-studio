//! Tests for the virtual registry (decision 0018): runtime/fle_reg.c. Expected values come from the Microsoft pages
//! (RegOpenKeyExW, RegQueryValueExW, RegCreateKeyExW, RegSetValueExW, RegEnumKeyExW, RegEnumValueW, RegDeleteKeyW,
//! RegDeleteValueW, RegQueryInfoKeyW, Registry Element Size Limits, Registry Value Types), listed in
//! docs/investigations/import-vfs-spec.md. Items marked "inference" in the C comments are not tested as facts.
//! No host registry exists on Linux; nothing is compared with a real Windows.

mod fsx_common;
use fsx_common::*;

const HKCU: &str = "0xFFFFFFFF80000001ull";
const HKLM: &str = "0xFFFFFFFF80000002ull";

#[test]
fn registry_starts_empty() {
    // Decision 0018: initial state empty, a missing key is ERROR_FILE_NOT_FOUND (2) (MSDN RegOpenKeyExW example).
    // Decision 0085 (provisional): the one exception is HKEY_LOCAL_MACHINE\Software, an empty key from the start.
    let l = lines(
        "reg-empty",
        &format!(
            r#"
    uint64_t k = 7;
    printf("%llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("SOFTWARE\\Microsoft\\Windows\\CurrentVersion"), 0, 0x20019, A(&k))));
    printf("%llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software"), 0, 0x20019, A(&k))));
    printf("%llu\n", U(CALL(fle_imp_RegOpenKeyExW, 0xFFFFFFFF80000000ull, W("CLSID"), 0, 0x20019, A(&k))));
    printf("%llu\n", U(CALL(fle_imp_RegOpenKeyExA, {HKLM}, A("HARDWARE"), 0, 0x20019, A(&k))));
    printf("k=%llu\n", U(k));
    {{
        uint64_t sw = 0;
        uint32_t subs = 99, vals = 99;
        printf("sw-open %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("Software"), 0, 0x20019, A(&sw))));
        printf("sw-info %llu\n", U(CALL(fle_imp_RegQueryInfoKeyW, sw, 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0)));
        printf("sw-counts subs=%u vals=%u\n", subs, vals);
    }}
    printf("hkcu-sw %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software"), 0, 0x20019, A(&k))));
"#
        ),
    );
    // the output handle is left alone on failure (the page says nothing: only the status is checked here)
    assert_eq!(&l[..4], ["2", "2", "2", "2"]);
    // decision 0085: HKLM\Software opens, with no subkey and no value; HKCU\Software is still missing
    assert_eq!(&l[5..], ["sw-open 0", "sw-info 0", "sw-counts subs=0 vals=0", "hkcu-sw 2"]);
}

#[test]
fn hklm_software_exists_and_is_empty_and_nothing_else_was_added() {
    // Decision 0085 (provisional): HKEY_LOCAL_MACHINE\Software is there from the start, with no value and no
    // subkey; key names are not case sensitive (MSDN Structure of the Registry). Microsoft and below are not made.
    // The other hives start as before: HKCU\Software is still missing (2), and no other root has a key.
    let l = lines(
        "reg-hklm-software",
        &format!(
            r#"
    uint64_t k = 7, st;
    uint32_t i, n, subs = 99, vals = 99, maxsub = 99, maxvn = 99, maxv = 99;
    static uint16_t name[32];
    static const uint64_t roots[5] = {{ 0xFFFFFFFF80000000ull, 0xFFFFFFFF80000001ull, {HKLM}, 0xFFFFFFFF80000003ull, 0xFFFFFFFF80000005ull }};
    printf("open %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("Software"), 0, 0x20019, A(&k))));
    st = CALL(fle_imp_RegQueryInfoKeyW, k, 0, 0, 0, A(&subs), A(&maxsub), 0, A(&vals), A(&maxvn), A(&maxv), 0, 0);
    printf("info %llu subs=%u maxsub=%u vals=%u maxvn=%u maxv=%u\n", U(st), subs, maxsub, vals, maxvn, maxv);
    n = 32; printf("enumkey %llu\n", U(CALL(fle_imp_RegEnumKeyExW, k, 0, A(name), A(&n), 0, 0, 0, 0)));
    n = 32; printf("enumval %llu\n", U(CALL(fle_imp_RegEnumValueW, k, 0, A(name), A(&n), 0, 0, 0, 0)));
    printf("close %llu\n", U(CALL(fle_imp_RegCloseKey, k)));
    printf("upper %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("SOFTWARE"), 0, 0x20019, A(&k))));
    printf("lower %llu\n", U(CALL(fle_imp_RegOpenKeyExA, {HKLM}, A("software"), 0, 0x20019, A(&k))));
    printf("microsoft %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("Software\\Microsoft"), 0, 0x20019, A(&k))));
    printf("classes %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("SOFTWARE\\Classes"), 0, 0x20019, A(&k))));
    printf("hkcu %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software"), 0, 0x20019, A(&k))));
    printf("hku %llu\n", U(CALL(fle_imp_RegOpenKeyExW, 0xFFFFFFFF80000003ull, W("Software"), 0, 0x20019, A(&k))));
    printf("hkcr %llu\n", U(CALL(fle_imp_RegOpenKeyExW, 0xFFFFFFFF80000000ull, W("Software"), 0, 0x20019, A(&k))));
    printf("hkcc %llu\n", U(CALL(fle_imp_RegOpenKeyExW, 0xFFFFFFFF80000005ull, W("Software"), 0, 0x20019, A(&k))));
    for (i = 0; i < 5; i++) {{
        uint32_t c = 0;
        for (;; c++) {{
            n = 32;
            st = CALL(fle_imp_RegEnumKeyExW, roots[i], c, A(name), A(&n), 0, 0, 0, 0);
            if (st) break;
            printf("root%u key %s %u\n", i, AS(name), n);
        }}
        printf("root%u end %llu keys=%u\n", i, U(st), c);
        st = CALL(fle_imp_RegQueryInfoKeyW, roots[i], 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0);
        printf("root%u info %llu subs=%u vals=%u\n", i, U(st), subs, vals);
    }}
"#
        ),
    );
    assert_eq!(
        l,
        [
            "open 0",
            "info 0 subs=0 maxsub=0 vals=0 maxvn=0 maxv=0",
            "enumkey 259",
            "enumval 259",
            "close 0",
            "upper 0",
            "lower 0",
            "microsoft 2",
            "classes 2",
            "hkcu 2",
            "hku 2",
            "hkcr 2",
            "hkcc 2",
            "root0 end 259 keys=0",
            "root0 info 0 subs=0 vals=0",
            "root1 end 259 keys=0",
            "root1 info 0 subs=0 vals=0",
            "root2 key Software 8",
            "root2 end 259 keys=1",
            "root2 info 0 subs=1 vals=0",
            "root3 end 259 keys=0",
            "root3 info 0 subs=0 vals=0",
            "root4 end 259 keys=0",
            "root4 info 0 subs=0 vals=0",
        ]
    );
}

#[test]
fn creating_the_fonts_key_like_unit_153_works_on_the_initial_registry() {
    // Decision 0085 (provisional). unit 153 of FLEngine calls RegCreateKeyExW(HKLM,
    // L"Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts", 0, NULL, 0, KEY_ALL_ACCESS, NULL, &h, &disp), then
    // RegQueryInfoKeyW and RegCloseKey (docs/investigations/registry-unit153.md section 2.2, 2.3). The page says all
    // missing keys of the path are created; disposition 1 = REG_CREATED_NEW_KEY, 2 = REG_OPENED_EXISTING_KEY.
    let l = lines(
        "reg-fonts",
        &format!(
            r#"
    uint64_t h = 0, h2 = 0, st;
    uint32_t disp = 77, subs = 99, vals = 99, i;
    static const uint16_t *const mid[3] = {{ u"Software\\Microsoft", u"Software\\Microsoft\\Windows NT", u"Software\\Microsoft\\Windows NT\\CurrentVersion" }};
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("create %llu disp=%u first-handle=%d\n", U(st), disp, h == FLE_REG_KEY_BASE + 4);
    st = CALL(fle_imp_RegQueryInfoKeyW, h, 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0);
    printf("info %llu subs=%u vals=%u\n", U(st), subs, vals);
    printf("close %llu\n", U(CALL(fle_imp_RegCloseKey, h)));
    disp = 77;
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("again %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegQueryInfoKeyW, h, 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0);
    printf("info2 %llu subs=%u vals=%u\n", U(st), subs, vals);
    printf("close2 %llu\n", U(CALL(fle_imp_RegCloseKey, h)));
    disp = 77;
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("software\\MICROSOFT\\windows nt\\currentversion\\FONTS"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("case %llu disp=%u\n", U(st), disp);
    CALL(fle_imp_RegCloseKey, h);
    for (i = 0; i < 3; i++) {{
        st = CALL(fle_imp_RegOpenKeyExW, {HKLM}, A(mid[i]), 0, 0x20019, A(&h2));
        subs = 99; vals = 99;
        if (!st) CALL(fle_imp_RegQueryInfoKeyW, h2, 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0);
        printf("mid%u open %llu subs=%u vals=%u\n", i, U(st), subs, vals);
        if (!st) CALL(fle_imp_RegCloseKey, h2);
    }}
    st = CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("Software"), 0, 0x20019, A(&h2));
    CALL(fle_imp_RegQueryInfoKeyW, h2, 0, 0, 0, A(&subs), 0, 0, A(&vals), 0, 0, 0, 0);
    printf("software %llu subs=%u vals=%u\n", U(st), subs, vals);
    printf("hkcu %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts"), 0, 0x20019, A(&h2))));
"#
        ),
    );
    assert_eq!(
        l,
        [
            "create 0 disp=1 first-handle=1",
            "info 0 subs=0 vals=0",
            "close 0",
            "again 0 disp=2",
            "info2 0 subs=0 vals=0",
            "close2 0",
            "case 0 disp=2",
            "mid0 open 0 subs=1 vals=0",
            "mid1 open 0 subs=1 vals=0",
            "mid2 open 0 subs=1 vals=0",
            "software 0 subs=1 vals=0",
            "hkcu 2",
        ]
    );
}

#[test]
fn creating_software_directly_under_hklm_opens_the_existing_key() {
    // Decision 0085 (provisional): HKLM\Software exists, so naming it as the first component is not the creation
    // of a direct child of HKEY_LOCAL_MACHINE: disposition 2, and a key under it is created (1).
    let l = lines(
        "reg-hklm-sw-create",
        &format!(
            r#"
    uint64_t h = 0, h2 = 0, st;
    uint32_t disp = 77;
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("Software"), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp));
    printf("software %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegCreateKeyExW, h, W("Foo"), 0, 0, 0, 0xF003F, 0, A(&h2), A(&disp));
    printf("via-handle %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("Software\\Foo\\Bar"), 0, 0, 0, 0xF003F, 0, A(&h2), A(&disp));
    printf("path %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegCreateKeyExW, {HKLM}, W("Software\\Baz"), 0, 0, 0, 0xF003F, 0, A(&h2), A(&disp));
    printf("baz %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("SOFTWARE\\foo\\bar"), 0, 0x20019, A(&h2));
    printf("reopen %llu\n", U(st));
"#
        ),
    );
    // "path": Foo already exists (made through the handle), so only Bar is new, and that is disposition 1.
    assert_eq!(l, ["software 0 disp=2", "via-handle 0 disp=1", "path 0 disp=1", "baz 0 disp=1", "reopen 0"]);
}

#[test]
fn the_initial_software_key_does_not_move_the_virtual_clock() {
    // Decision 0085: adding HKLM\Software at start must not change the time stamps of later keys. The virtual clock
    // starts at 2021-01-01 (132_539_328_000_000_000, tests/time.rs) and moves 1 s (10_000_000) per change; the first
    // key the guest creates gets start + 1 s, as it did before 0085. Software's own write time is the clock start
    // (a choice of 0085, not a value from a page).
    let l = lines(
        "reg-hklm-sw-clock",
        &format!(
            r#"
    uint64_t sw = 0, t = 0, st, ft = 0;
    printf("sw-open %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKLM}, W("Software"), 0, 0x20019, A(&sw))));
    st = CALL(fle_imp_RegQueryInfoKeyW, sw, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, A(&ft));
    printf("sw-time %llu %llu\n", U(st), U(ft));
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("T"), 0, 0, 0, 0xF003F, 0, A(&t), 0);
    st = CALL(fle_imp_RegQueryInfoKeyW, t, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, A(&ft));
    printf("first-key-time %llu %llu\n", U(st), U(ft));
"#
        ),
    );
    assert_eq!(l, ["sw-open 0", "sw-time 0 132539328000000000", "first-key-time 0 132539328010000000"]);
}

#[test]
fn creating_a_direct_child_of_hklm_or_hku_still_stops_with_the_named_message() {
    // Decision 0085 keeps the rule (fle_reg.c): the page says a key cannot be a direct child of HKEY_USERS or
    // HKEY_LOCAL_MACHINE and names no code, so the run stops. Only the first missing component is looked at: Foo is
    // missing under HKLM, so Foo\Software stops too; HKU has no Software, so HKU\Software\Foo stops.
    let cases: Vec<(&str, String)> = vec![
        ("hklm-foo", format!("CALL(fle_imp_RegCreateKeyExW, {HKLM}, W(\"Foo\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("hklm-foo-software", format!("CALL(fle_imp_RegCreateKeyExW, {HKLM}, W(\"Foo\\\\Software\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("hklm-hardware", format!("CALL(fle_imp_RegCreateKeyExW, {HKLM}, W(\"HARDWARE\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("hku-foo", "CALL(fle_imp_RegCreateKeyExW, 0xFFFFFFFF80000003ull, W(\"Foo\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);".into()),
        ("hku-software-foo", "CALL(fle_imp_RegCreateKeyExW, 0xFFFFFFFF80000003ull, W(\"Software\\\\Foo\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);".into()),
    ];
    for (label, stmts) in &cases {
        let body = format!("uint64_t k = 0;\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("reg-direct-child-{label}"), &body);
        assert!(!o.ok && o.signal == Some(6), "{label}: expected SIGABRT, got {:?} {}", o.signal, o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("RegCreateKeyExW") && o.stderr.contains("direct child of HKEY_LOCAL_MACHINE or HKEY_USERS"), "{label}: {}", o.stderr);
    }
}

#[test]
fn create_open_set_query_close() {
    // RegCreateKeyExW: creates every missing key; disposition 1 = REG_CREATED_NEW_KEY, 2 = REG_OPENED_EXISTING_KEY.
    // Key names are not case sensitive. RegQueryValueExW: size first with lpData NULL, ERROR_MORE_DATA (234) with
    // the needed size, data as stored.
    let l = lines(
        "reg-basic",
        &format!(
            r#"
    uint64_t k = 0, k2 = 0;
    uint32_t disp = 0, type = 0, cb = 0;
    static uint16_t buf[32];
    uint32_t dw = 0xCAFEF00D;
    uint32_t cap;
    uint64_t st;
    st = CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("Software\\Acme\\App"), 0, 0, 0, 0x20006 | 0x20019, 0, A(&k), A(&disp));
    printf("create %llu disp=%u\n", U(st), disp);
    st = CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("SOFTWARE\\acme\\APP"), 0, 0, 0, 0x20006 | 0x20019, 0, A(&k2), A(&disp));
    printf("again %llu disp=%u\n", U(st), disp);
    CALL(fle_imp_RegCloseKey, k2);
    /* REG_SZ "hi" with its NUL: 6 bytes */
    printf("setsz %llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("Name"), 0, 1, W("hi"), 6)));
    printf("setdw %llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("Count"), 0, 4, A(&dw), 4)));
    /* query sizes: lpData NULL returns the size and ERROR_SUCCESS */
    cb = 0; st = CALL(fle_imp_RegQueryValueExW, k, W("Name"), 0, A(&type), 0, A(&cb));
    printf("size %llu type=%u cb=%u\n", U(st), type, cb);
    /* too small -> ERROR_MORE_DATA and the required size */
    cap = 2; cb = cap; st = CALL(fle_imp_RegQueryValueExW, k, W("name"), 0, A(&type), A(buf), A(&cb));
    printf("small %llu cb=%u\n", U(st), cb);
    cb = sizeof buf; st = CALL(fle_imp_RegQueryValueExW, k, W("NAME"), 0, A(&type), A(buf), A(&cb));
    printf("get %llu type=%u cb=%u %s\n", U(st), type, cb, AS(buf));
    dw = 0; cb = 4; st = CALL(fle_imp_RegQueryValueExW, k, W("Count"), 0, A(&type), A(&dw), A(&cb));
    printf("dw %llu type=%u cb=%u %x\n", U(st), type, cb, dw);
    /* a value that is not there: ERROR_FILE_NOT_FOUND */
    printf("missing %llu\n", U(CALL(fle_imp_RegQueryValueExW, k, W("Nope"), 0, 0, 0, 0)));
    /* open the key by its path from the predefined root, in another case */
    printf("open %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("software\\ACME"), 0, 0x20019, A(&k2))));
    printf("openfile %llu\n", U(CALL(fle_imp_RegOpenKeyExW, k2, W("App"), 0, 0x20019, A(&k2))));
    cb = sizeof buf; st = CALL(fle_imp_RegQueryValueExW, k2, W("Name"), 0, 0, A(buf), A(&cb));
    printf("seen %llu %s\n", U(st), AS(buf));
    /* a predefined key with a NULL or empty subkey returns the same handle (MSDN RegOpenKeyExW) */
    printf("same %llu %d\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, 0, 0, 0x20019, A(&k2))), k2 == {HKCU});
    printf("same2 %llu %d\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(""), 0, 0x20019, A(&k2))), k2 == {HKCU});
    printf("close %llu\n", U(CALL(fle_imp_RegCloseKey, k)));
"#
        ),
    );
    assert_eq!(
        l,
        [
            "create 0 disp=1",
            "again 0 disp=2",
            "setsz 0",
            "setdw 0",
            "size 0 type=1 cb=6",
            "small 234 cb=6",
            "get 0 type=1 cb=6 hi",
            "dw 0 type=4 cb=4 cafef00d",
            "missing 2",
            "open 0",
            "openfile 0",
            "seen 0 hi",
            "same 0 1",
            "same2 0 1",
            "close 0",
        ]
    );
}

#[test]
fn unnamed_value_and_overwrite() {
    // RegSetValueExW: a NULL or "" name sets the unnamed (default) value; setting an existing name replaces type
    // and data. RegQueryValueExW: the same name rule.
    let l = lines(
        "reg-unnamed",
        &format!(
            r#"
    uint64_t k = 0;
    uint32_t type, cb, v = 5;
    static uint16_t b[16];
    uint64_t st;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("U"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    CALL(fle_imp_RegSetValueExW, k, 0, 0, 1, W("a"), 4);
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W(""), 0, A(&type), A(b), A(&cb));
    printf("%llu %u %u %s\n", U(st), type, cb, AS(b));
    CALL(fle_imp_RegSetValueExW, k, W(""), 0, 4, A(&v), 4);
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, 0, 0, A(&type), A(b), A(&cb));
    printf("%llu %u %u\n", U(st), type, cb);
    printf("del %llu\n", U(CALL(fle_imp_RegDeleteValueW, k, 0)));
    printf("gone %llu\n", U(CALL(fle_imp_RegQueryValueExW, k, W(""), 0, 0, 0, 0)));
    printf("del2 %llu\n", U(CALL(fle_imp_RegDeleteValueW, k, W("zz"))));
"#
        ),
    );
    assert_eq!(l, ["0 1 4 a", "0 4 4", "del 0", "gone 2", "del2 2"]);
}

#[test]
fn value_types_and_size_rules() {
    let l = lines(
        "reg-types",
        &format!(
            r#"
    uint64_t k = 0, q = 0x1122334455667788ull;
    uint32_t type, cb;
    static uint8_t b[64];
    static const uint8_t bin[3] = {{ 1, 2, 3 }};
    static const uint16_t multi[] = {{ 'a', 0, 'b', 'c', 0, 0 }};
    uint64_t st;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("TypeTest"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    printf("%llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("bin"), 0, 3, A(bin), 3)));
    printf("%llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("none"), 0, 0, 0, 0)));
    printf("%llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("multi"), 0, 7, A(multi), sizeof multi)));
    printf("%llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("qw"), 0, 11, A(&q), 8)));
    printf("%llu\n", U(CALL(fle_imp_RegSetValueExW, k, W("exp"), 0, 2, W("%PATH%"), 14)));
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W("bin"), 0, A(&type), A(b), A(&cb)); printf("bin %llu %u %u %u%u%u\n", U(st), type, cb, b[0], b[1], b[2]);
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W("none"), 0, A(&type), A(b), A(&cb)); printf("none %llu %u %u\n", U(st), type, cb);
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W("multi"), 0, A(&type), A(b), A(&cb)); printf("multi %llu %u %u\n", U(st), type, cb);
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W("qw"), 0, A(&type), A(b), A(&cb)); printf("qw %llu %u %u %llx\n", U(st), type, cb, U(*(uint64_t *)b));
    cb = sizeof b; st = CALL(fle_imp_RegQueryValueExW, k, W("exp"), 0, A(&type), A(b), A(&cb)); printf("exp %llu %u %u\n", U(st), type, cb);
"#
        ),
    );
    // MSDN Registry Value Types gives the names; the numbers (REG_NONE 0, REG_SZ 1, REG_EXPAND_SZ 2, REG_BINARY 3,
    // REG_DWORD 4, REG_MULTI_SZ 7, REG_QWORD 11) are winnt.h values, not on the page (see fle_reg.h).
    assert_eq!(
        l,
        [
            "0", "0", "0", "0", "0",
            "bin 0 3 3 123",
            "none 0 0 0",
            "multi 0 7 12",
            "qw 0 11 8 1122334455667788",
            "exp 0 2 14",
        ]
    );
}

#[test]
fn string_data_is_returned_as_stored_without_extra_terminators() {
    // MSDN RegQueryValueExW: "the value returned is NOT guaranteed to be null-terminated": the data is returned as stored.
    // RegSetValueExW demands cbData that includes the NUL, so this runtime never stores an unterminated string
    // (it stops instead); the size therefore always includes the terminator.
    let l = lines(
        "reg-sz",
        &format!(
            r#"
    uint64_t k = 0;
    uint32_t cb;
    static uint16_t b[8];
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("S"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    CALL(fle_imp_RegSetValueExW, k, W("v"), 0, 1, W("abc"), 8);
    cb = sizeof b; CALL(fle_imp_RegQueryValueExW, k, W("v"), 0, 0, A(b), A(&cb));
    printf("%u %u %u %u %u\n", cb, b[0], b[2], b[3], (unsigned)cb / 2);
"#
        ),
    );
    assert_eq!(l, ["8 97 99 0 4"]);
}

#[test]
fn enumeration_and_info() {
    // RegEnumKeyExW / RegEnumValueW: index from 0 until ERROR_NO_MORE_ITEMS (259); a too-small name buffer is
    // ERROR_MORE_DATA (234). RegQueryInfoKeyW: counts, longest names in characters without the NUL, longest value
    // data in bytes. The page says the order is arbitrary, so the test sorts.
    let l = lines(
        "reg-enum",
        &format!(
            r#"
    uint64_t k = 0, st;
    uint32_t i, n, cb, type;
    static uint16_t name[32];
    static uint8_t data[64];
    uint32_t subs, maxsub, vals, maxvn, maxv;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("E"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    static uint64_t tmp[2];
    CALL(fle_imp_RegCreateKeyExW, k, W("alpha"), 0, 0, 0, 0xF003F, 0, A(&tmp[0]), 0);
    CALL(fle_imp_RegCreateKeyExW, k, W("be"), 0, 0, 0, 0xF003F, 0, A(&tmp[1]), 0);
    CALL(fle_imp_RegSetValueExW, k, W("v1"), 0, 1, W("xyz"), 8);
    CALL(fle_imp_RegSetValueExW, k, W("longer"), 0, 3, A("0123456789"), 10);
    for (i = 0;; i++) {{
        n = 32; st = CALL(fle_imp_RegEnumKeyExW, k, i, A(name), A(&n), 0, 0, 0, 0);
        if (st) {{ printf("endk %llu\n", U(st)); break; }}
        printf("key %s %u\n", AS(name), n);
    }}
    n = 3; st = CALL(fle_imp_RegEnumKeyExW, k, 0, A(name), A(&n), 0, 0, 0, 0);
    printf("small %llu\n", U(st));
    for (i = 0;; i++) {{
        n = 32; cb = sizeof data;
        st = CALL(fle_imp_RegEnumValueW, k, i, A(name), A(&n), 0, A(&type), A(data), A(&cb));
        if (st) {{ printf("endv %llu\n", U(st)); break; }}
        printf("val %s %u type=%u cb=%u\n", AS(name), n, type, cb);
    }}
    n = 32; cb = 2;
    st = CALL(fle_imp_RegEnumValueW, k, 0, A(name), A(&n), 0, A(&type), A(data), A(&cb));
    printf("smalldata %llu cb=%u\n", U(st), cb);
    st = CALL(fle_imp_RegQueryInfoKeyW, k, 0, 0, 0, A(&subs), A(&maxsub), 0, A(&vals), A(&maxvn), A(&maxv), 0, 0);
    printf("info %llu subs=%u maxsub=%u vals=%u maxvn=%u maxv=%u\n", U(st), subs, maxsub, vals, maxvn, maxv);
"#
        ),
    );
    let mut keys: Vec<&String> = l.iter().filter(|s| s.starts_with("key ")).collect();
    keys.sort();
    assert_eq!(keys, ["key alpha 5", "key be 2"]);
    let mut vals: Vec<&String> = l.iter().filter(|s| s.starts_with("val ")).collect();
    vals.sort();
    assert_eq!(vals, ["val longer 6 type=3 cb=10", "val v1 2 type=1 cb=8"]);
    let others: Vec<&String> =
        l.iter().filter(|s| !s.starts_with("key ") && !s.starts_with("val ")).collect();
    assert_eq!(
        others,
        // "smalldata": index 0 is "v1" (8 bytes) because this runtime enumerates in creation order (the page leaves the
        // order open); the size reported with ERROR_MORE_DATA is that value's size.
        ["endk 259", "small 234", "endv 259", "smalldata 234 cb=8", "info 0 subs=2 maxsub=5 vals=2 maxvn=6 maxv=10"]
    );
}

#[test]
fn delete_key_rules() {
    // RegDeleteKeyW: the subkey must have no subkeys; key names are not case sensitive. A deleted key is gone.
    let l = lines(
        "reg-delkey",
        &format!(
            r#"
    uint64_t k = 0, c = 0;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("D"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    CALL(fle_imp_RegCreateKeyExW, k, W("child\\grand"), 0, 0, 0, 0xF003F, 0, A(&c), 0);
    CALL(fle_imp_RegCloseKey, c);
    printf("withkids %d\n", CALL(fle_imp_RegDeleteKeyW, k, W("child")) != 0);
    printf("grand %llu\n", U(CALL(fle_imp_RegDeleteKeyW, k, W("CHILD\\Grand"))));
    printf("child %llu\n", U(CALL(fle_imp_RegDeleteKeyW, k, W("child"))));
    printf("again %llu\n", U(CALL(fle_imp_RegDeleteKeyW, k, W("child"))));
    CALL(fle_imp_RegCloseKey, k);
    printf("top %llu\n", U(CALL(fle_imp_RegDeleteKeyW, {HKCU}, W("d"))));
    printf("open %llu\n", U(CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("D"), 0, 0x20019, A(&k))));
"#
        ),
    );
    assert_eq!(l, ["withkids 1", "grand 0", "child 0", "again 2", "top 0", "open 2"]);
}

#[test]
fn access_rights_and_handles() {
    // RegSetValueExW needs KEY_SET_VALUE; RegQueryValueExW needs KEY_QUERY_VALUE (pages). The error code 5 is an
    // inference. A closed handle is invalid (6, inference); closing a predefined key succeeds.
    let l = lines(
        "reg-access",
        &format!(
            r#"
    uint64_t k = 0, ro = 0, wo = 0;
    uint32_t cb = 0;
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("A"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    CALL(fle_imp_RegSetValueExW, k, W("v"), 0, 1, W("x"), 4);
    CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("A"), 0, 0x1, A(&ro));     /* KEY_QUERY_VALUE */
    CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("A"), 0, 0x2, A(&wo));     /* KEY_SET_VALUE */
    printf("ro-set %llu\n", U(CALL(fle_imp_RegSetValueExW, ro, W("w"), 0, 1, W("x"), 4)));
    printf("ro-query %llu\n", U(CALL(fle_imp_RegQueryValueExW, ro, W("v"), 0, 0, 0, A(&cb))));
    printf("wo-query %llu\n", U(CALL(fle_imp_RegQueryValueExW, wo, W("v"), 0, 0, 0, A(&cb))));
    printf("wo-set %llu\n", U(CALL(fle_imp_RegSetValueExW, wo, W("w"), 0, 1, W("x"), 4)));
    printf("close %llu\n", U(CALL(fle_imp_RegCloseKey, ro)));
    printf("closed %llu\n", U(CALL(fle_imp_RegQueryValueExW, ro, W("v"), 0, 0, 0, A(&cb))));
    printf("closeagain %llu\n", U(CALL(fle_imp_RegCloseKey, ro)));
    printf("predef %llu\n", U(CALL(fle_imp_RegCloseKey, {HKCU})));
    printf("flush %llu\n", U(CALL(fle_imp_RegFlushKey, k)));
"#
        ),
    );
    assert_eq!(
        l,
        ["ro-set 5", "ro-query 0", "wo-query 5", "wo-set 0", "close 0", "closed 6", "closeagain 6", "predef 0", "flush 0"]
    );
}

#[test]
fn predefined_keys_accept_zero_and_sign_extended_forms() {
    // FLEngine passes 0x80000001 zero-extended (objdump 0x419a12); winnt.h defines the sign-extended value.
    let l = lines(
        "reg-predef",
        &format!(
            r#"
    uint64_t k = 0, r;
    CALL(fle_imp_RegCreateKeyExW, 0x80000001ull, W("Z"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    r = CALL(fle_imp_RegOpenKeyExW, {HKCU}, W("Z"), 0, 0x20019, A(&k));
    printf("%llu\n", U(r));
    r = CALL(fle_imp_RegOpenKeyExW, 0x80000002ull, W("Z"), 0, 0x20019, A(&k));
    printf("%llu\n", U(r));
"#
        ),
    );
    assert_eq!(l, ["0", "2"]);
}

// ---- stops ----

#[test]
fn undefined_input_stops_the_run() {
    let setup = format!(
        r#"
    uint64_t k = 0;
    uint32_t cb = 4, type;
    static uint8_t b[16];
    CALL(fle_imp_RegCreateKeyExW, {HKCU}, W("T"), 0, 0, 0, 0xF003F, 0, A(&k), 0);
    CALL(fle_imp_RegSetValueExW, k, W("v"), 0, 1, W("x"), 4);
"#
    );
    let cases: Vec<(&str, String)> = vec![
        ("open-link-option", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"T\"), 2, 0x20019, A(&k));")),
        ("open-null-out", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"T\"), 0, 0x20019, 0);")),
        // decision 0126 (wall 81, S-wow64-views): old: ("open-wow64-view", samDesired 0x20019 | 0x100) stopped; KEY_WOW64_64KEY / KEY_WOW64_32KEY are a choice of view and are accepted now (both at once: 87; tests/wall_fr11.rs)
        ("open-max-allowed", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"T\"), 0, 0x02000000, A(&k));")),
        ("open-leading-backslash", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"\\\\T\"), 0, 0x20019, A(&k));")),
        ("open-trailing-backslash", format!("CALL(fle_imp_RegOpenKeyExW, {HKCU}, W(\"T\\\\\"), 0, 0x20019, A(&k));")),
        ("open-foreign-handle", "CALL(fle_imp_RegOpenKeyExW, 0x1234, W(\"T\"), 0, 0x20019, A(&k));".into()),
        ("open-perf-data", "CALL(fle_imp_RegOpenKeyExW, 0x80000004ull, W(\"T\"), 0, 0x20019, A(&k));".into()),
        ("openA-non-ascii", format!("CALL(fle_imp_RegOpenKeyExA, {HKCU}, A(\"\\xe9\"), 0, 0x20019, A(&k));")),
        ("create-null-subkey", format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, 0, 0, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("create-reserved", format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, W(\"Q\"), 1, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("create-link", format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, W(\"Q\"), 0, 0, 2, 0xF003F, 0, A(&k), 0);")),
        ("create-secattr", format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, W(\"Q\"), 0, 0, 0, 0xF003F, 1, A(&k), 0);")),
        ("create-under-hklm", format!("CALL(fle_imp_RegCreateKeyExW, {HKLM}, W(\"Q\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")),
        ("create-33-levels", {
            let p = vec!["a"; 33].join("\\\\");
            format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, W(\"{p}\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")
        }),
        ("create-long-component", {
            let p = "a".repeat(256);
            format!("CALL(fle_imp_RegCreateKeyExW, {HKCU}, W(\"{p}\"), 0, 0, 0, 0xF003F, 0, A(&k), 0);")
        }),
        ("set-reserved", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 1, 1, W(\"x\"), 4);".into()),
        ("set-link-type", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 6, W(\"x\"), 4);".into()),
        ("set-big-endian", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 5, A(b), 4);".into()),
        ("set-unknown-type", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 99, A(b), 4);".into()),
        ("set-sz-unterminated", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 1, W(\"ab\"), 4);".into()),
        ("set-sz-odd-size", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 1, W(\"ab\"), 5);".into()),
        ("set-multi-one-nul", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 7, W(\"ab\"), 6);".into()),
        ("set-dword-size", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 4, A(b), 3);".into()),
        ("set-null-data-size", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 3, 0, 4);".into()),
        ("set-too-big", "CALL(fle_imp_RegSetValueExW, k, W(\"v\"), 0, 3, A(b), 0x200000);".into()),
        ("query-reserved", "CALL(fle_imp_RegQueryValueExW, k, W(\"v\"), 1, 0, 0, 0);".into()),
        ("query-data-without-size", "CALL(fle_imp_RegQueryValueExW, k, W(\"v\"), 0, 0, A(b), 0);".into()),
        ("enumkey-reserved", "CALL(fle_imp_RegEnumKeyExW, k, 0, A(b), A(&cb), 1, 0, 0, 0);".into()),
        ("enumkey-null-name", "CALL(fle_imp_RegEnumKeyExW, k, 0, 0, A(&cb), 0, 0, 0, 0);".into()),
        ("enumvalue-null-name", "CALL(fle_imp_RegEnumValueW, k, 0, 0, A(&cb), 0, 0, 0, 0);".into()),
        ("info-security-descriptor", "CALL(fle_imp_RegQueryInfoKeyW, k, 0, 0, 0, 0, 0, 0, 0, 0, 0, A(&cb), 0);".into()),
        ("deletekey-null", format!("CALL(fle_imp_RegDeleteKeyW, {HKCU}, 0);")),
        ("deletekey-empty", format!("CALL(fle_imp_RegDeleteKeyW, {HKCU}, W(\"\"));")),
        ("deletekey-open-handle", format!("CALL(fle_imp_RegDeleteKeyW, {HKCU}, W(\"T\"));")),
    ];
    let cases: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (*a, b.as_str())).collect();
    assert_aborts("reg-abort", &setup, &cases);
}
