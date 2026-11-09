//! Tests for the oleaut32 answers (BSTR, VARIANT, SAFEARRAY, GetErrorInfo) in runtime/fle_oleaut.c and the ole32 answers
//! (CoInitialize family, CoTaskMem*, StringFromCLSID, ProgIDFromCLSID, CLSIDFromString, IsEqualGUID) in runtime/fle_ole.c
//! (docs/decisions/0037-apis-oleaut-ole.md; the pages read: docs/investigations/apis-oleaut-ole-run.md).
//!
//! Expected values are written here from the Microsoft pages (BSTR, SysAllocStringLen, SysFreeString, SysReAllocStringLen,
//! VARENUM, VARIANT, VariantInit/Clear/Copy/CopyInd/ChangeType, SAFEARRAY, SAFEARRAYBOUND, SafeArrayCreate/GetLBound/GetUBound/
//! AccessData/UnaccessData/GetElement/PutElement/PtrOfIndex/GetElemsize/Destroy, GetErrorInfo, CoInitialize, CoUninitialize,
//! OleInitialize, OleUninitialize, CoTaskMemAlloc/Free, StringFromCLSID, CLSIDFromString, ProgIDFromCLSID, IsEqualGUID, GUID,
//! COM Error Codes), not read back from the C code. The HRESULT numbers are literals here, taken from those pages. A value
//! that no page fixes is a setting of the virtual machine and is marked "setting". The VARIANT and SAFEARRAY structs are
//! mirrored in the test programs with natural alignment on x64. The host is Linux: nothing is compared with a real Windows.
//!
//! The C sources come from runtime/ when fle_ole.c (the last file of the unit) is there, otherwise from out-oleaut/work/
//! (development).

mod etxt_common;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn runtime_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if runtime_dir().join("fle_ole.c").exists() {
        runtime_dir()
    } else {
        manifest().join("../../out-oleaut/work")
    }
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-oleaut/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

fn clang_c(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-c"]);
    if strict {
        cmd.arg("-Werror");
    }
    let cc = cmd
        .arg("-I")
        .arg(src_dir())
        .arg("-I")
        .arg(runtime_dir())
        .arg(src)
        .arg("-o")
        .arg(obj)
        .env("TMPDIR", work_dir().join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

fn objs() -> &'static Vec<PathBuf> {
    static O: OnceLock<Vec<PathBuf>> = OnceLock::new();
    O.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("obj"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut v = Vec::new();
        for (name, ours) in [
            ("fle_oleaut.c", true),
            ("fle_ole.c", true),
            ("fle_reg.c", false),
            ("fle_imports.c", false),
            ("fle_env.c", false),
        ] {
            let base = if ours { src_dir() } else { runtime_dir() };
            let o = dir.join(name.replace(".c", ".o"));
            clang_c(&base.join(name), &o, ours);
            v.push(o);
        }
        v
    })
}

pub struct Out {
    stdout: String,
    stderr: String,
    signal: Option<i32>,
    ok: bool,
}

const PRELUDE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_reg.h"
#include "fle_oleaut.h"
#include "fle_ole.h"

uint64_t R[16];
static uint64_t stk[32];

static uint64_t callv(void (*f)(void), const uint64_t *a)
{
    int i;
    memset(stk, 0, sizeof stk);
    stk[0] = 0xdead0000u;
    R[1] = a[0]; R[2] = a[1]; R[8] = a[2]; R[9] = a[3];
    for (i = 4; i < 12; i++) stk[i + 1] = a[i];
    R[4] = (uint64_t)(uintptr_t)stk;
    R[0] = 0xBADBADBADull;
    f();
    return R[0];
}
#define CALL(f, ...) callv((void (*)(void))(f), (uint64_t[12]){ __VA_ARGS__ })
#define A(x) ((uint64_t)(uintptr_t)(x))
#define U(x) ((unsigned long long)(x))
#define H(x) ((unsigned)(uint32_t)(x))

/* VARIANT on x64 as the VARIANT page lays it out: vt, three reserved words, a union of 16 bytes (BRECORD, DECIMAL). */
typedef struct { uint16_t vt, r1, r2, r3; union { int64_t ll; int32_t l; int16_t i; uint8_t b; uint64_t u; void *p; double d; } v; uint64_t pad; } V;
/* SAFEARRAY and SAFEARRAYBOUND as the pages give them. */
typedef struct { uint32_t cElements; int32_t lLbound; } SAB;
typedef struct { uint16_t cDims, fFeatures; uint32_t cbElements, cLocks; void *pvData; SAB b[1]; } SA;

enum { VT_EMPTY_ = 0, VT_NULL_ = 1, VT_I2_ = 2, VT_I4_ = 3, VT_R8_ = 5, VT_BSTR_ = 8, VT_DISPATCH_ = 9, VT_BOOL_ = 11,
       VT_VARIANT_ = 12, VT_UNKNOWN_ = 13, VT_UI1_ = 17, VT_RECORD_ = 36, VT_ARRAY_ = 0x2000, VT_BYREF_ = 0x4000 };

static uint64_t bs(const char *s)
{
    uint16_t w[64];
    size_t n = strlen(s), i;
    for (i = 0; i < n; i++) w[i] = (uint8_t)s[i];
    return CALL(fle_imp_SysAllocStringLen, A(w), n);
}
static int bs_is(uint64_t p, const char *s)
{
    size_t n = strlen(s), i;
    if (p == 0) return n == 0;
    if ((uint32_t)fle_ld(p - 4, 4) != 2 * n) return 0;
    for (i = 0; i < n; i++) if ((uint16_t)fle_ld(p + 2 * i, 2) != (uint8_t)s[i]) return 0;
    return (uint16_t)fle_ld(p + 2 * n, 2) == 0;
}
static SA *mk(unsigned vt, uint32_t n, int32_t lb)
{
    SAB b = { n, lb };
    return (SA *)CALL(fle_imp_SafeArrayCreate, vt, 1, A(&b));
}
"#;

fn run_src(name: &str, top: &str, body: &str) -> Out {
    let dir = work_dir();
    let uname = etxt_common::uniq(name);
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    let src = format!("{PRELUDE}\n{top}\nint main(void)\n{{\n{body}\n    return 0;\n}}\n");
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(src_dir()).arg("-I").arg(runtime_dir());
    for o in objs() {
        cmd.arg(o);
    }
    let cc = cmd.arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).env("TMPDIR", dir.join("tmp")).output().expect("run failed");
    Out {
        stdout: String::from_utf8_lossy(&r.stdout).to_string(),
        stderr: String::from_utf8_lossy(&r.stderr).to_string(),
        signal: r.status.signal(),
        ok: r.status.success(),
    }
}

fn run(name: &str, body: &str) -> Out {
    run_src(name, "", body)
}

fn lines(name: &str, body: &str) -> Vec<String> {
    let o = run(name, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

/// Each (label, C statements) must stop the run with SIGABRT, a "fle:" message and no output after it. `setup` runs first.
fn assert_aborts(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run(&format!("{prefix}-{label}"), &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

fn header_has(file: &str, decl: &str) -> bool {
    std::fs::read_to_string(src_dir().join(file)).unwrap().contains(decl)
}

// HRESULTs as the Microsoft pages print them (Common HRESULT Values, COM Error Codes).
const S_OK: &str = "00000000";
const S_FALSE: &str = "00000001";
const E_UNEXPECTED: &str = "8000ffff";
const E_INVALIDARG: &str = "80070057";
const DISP_E_BADVARTYPE: &str = "80020008";
const DISP_E_OVERFLOW: &str = "8002000a";
const DISP_E_BADINDEX: &str = "8002000b";
const DISP_E_ARRAYISLOCKED: &str = "8002000d";
const CO_E_CLASSSTRING: &str = "800401f3";
const REGDB_E_CLASSNOTREG: &str = "80040154";

// ---- 1. Table and headers ----

const OLEAUT: [&str; 18] = [
    "SysAllocStringLen",
    "SysFreeString",
    "SysReAllocStringLen",
    "VariantInit",
    "VariantClear",
    "VariantCopy",
    "VariantCopyInd",
    "VariantChangeType",
    "SafeArrayCreate",
    "SafeArrayGetLBound",
    "SafeArrayGetUBound",
    "SafeArrayAccessData",
    "SafeArrayUnaccessData",
    "SafeArrayGetElement",
    "SafeArrayPutElement",
    "SafeArrayPtrOfIndex",
    "SafeArrayGetElemsize",
    "GetErrorInfo",
];
const OLE: [&str; 10] = [
    "CoInitialize",
    "CoUninitialize",
    "OleInitialize",
    "OleUninitialize",
    "CoTaskMemAlloc",
    "CoTaskMemFree",
    "StringFromCLSID",
    "ProgIDFromCLSID",
    "CLSIDFromString",
    "IsEqualGUID",
];

#[test]
fn rows_are_in_the_table_and_declared_in_the_headers() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    for n in OLEAUT {
        let row = format!("(\"oleaut32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(header_has("fle_oleaut.h", &format!("void fle_imp_{n}(void)")), "{n} is declared in fle_oleaut.h");
    }
    for n in OLE {
        let row = format!("(\"ole32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(header_has("fle_ole.h", &format!("void fle_imp_{n}(void)")), "{n} is declared in fle_ole.h");
    }
}

/// With the real DLL and the all-pdata seeds: every API of the unit is a handled thunk (skipped when either is absent).
#[test]
fn real_dll_handles_the_rows() {
    let Ok(data) = std::fs::read(manifest().join("../../../binaries/win64/FLEngine_x64.dll")) else { return };
    let Ok(seeds_txt) = std::fs::read_to_string(manifest().join("../../out-pdata/seeds-pdata.txt")) else { return };
    let seeds: Vec<u64> =
        seeds_txt.lines().filter(|l| !l.trim().is_empty()).map(|l| u64::from_str_radix(l.trim().trim_start_matches("0x"), 16).unwrap()).collect();
    let st = fe_lift::generate_with_seeds(&data, &seeds, &mut std::io::sink()).unwrap();
    for n in OLEAUT.iter().chain(OLE.iter()) {
        assert!(st.handled.get(format!("jmp_imp:{n}").as_str()).copied().unwrap_or(0) >= 1, "{n} is a handled thunk");
    }
}

// ---- 2. BSTR ----

#[test]
fn sys_alloc_string_len_layout() {
    let l = lines(
        "ole-bstr-layout",
        r#"
    static uint16_t abc[] = { 'a', 'b', 'c' };
    static const uint16_t emb[] = { 'a', 0, 'b', 'c', 'X' };
    uint64_t p = CALL(fle_imp_SysAllocStringLen, A(abc), 3), q, z, e;
    /* BSTR page: the pointer is at the first character, 4 bytes before it hold the byte count (6), a NUL WCHAR ends it */
    printf("a %d prefix %u c0 %x c2 %x term %x\n", p != 0, H(fle_ld(p - 4, 4)), H(fle_ld(p, 2)), H(fle_ld(p + 4, 2)), H(fle_ld(p + 6, 2)));
    /* SysAllocStringLen page: embedded NULs are copied; ui characters only (the 5th unit is not copied) */
    q = CALL(fle_imp_SysAllocStringLen, A(emb), 4);
    printf("emb prefix %u %x %x %x %x term %x\n", H(fle_ld(q - 4, 4)), H(fle_ld(q, 2)), H(fle_ld(q + 2, 2)), H(fle_ld(q + 4, 2)), H(fle_ld(q + 6, 2)), H(fle_ld(q + 8, 2)));
    /* ui 0: ui + 1 characters are allocated, a valid empty BSTR */
    e = CALL(fle_imp_SysAllocStringLen, A(abc), 0);
    printf("empty %d prefix %u term %x\n", e != 0, H(fle_ld(e - 4, 4)), H(fle_ld(e, 2)));
    /* strIn NULL: length ui, contents not given by the page; zero (setting) */
    z = CALL(fle_imp_SysAllocStringLen, 0, 3);
    printf("null_in %d prefix %u c %x %x %x term %x\n", z != 0, H(fle_ld(z - 4, 4)), H(fle_ld(z, 2)), H(fle_ld(z + 2, 2)), H(fle_ld(z + 4, 2)), H(fle_ld(z + 6, 2)));
    printf("distinct %d\n", p != q && q != e && e != z && p != z);
    printf("live %llu\n", U(fle_oleaut_bstr_live()));
    /* the data is its own copy */
    ((uint16_t *)abc)[0] = 'Z';
    printf("copy %x\n", H(fle_ld(p, 2)));
"#,
    );
    assert_eq!(
        l,
        [
            "a 1 prefix 6 c0 61 c2 63 term 0",
            "emb prefix 8 61 0 62 63 term 0",
            "empty 1 prefix 0 term 0",
            "null_in 1 prefix 6 c 0 0 0 term 0",
            "distinct 1",
            "live 4",
            "copy 61",
        ]
    );
}

#[test]
fn sys_free_string() {
    let l = lines(
        "ole-bstr-free",
        r#"
    uint64_t p = bs("hello"), q = bs("x");
    printf("live %llu\n", U(fle_oleaut_bstr_live()));
    CALL(fle_imp_SysFreeString, 0);            /* page: NULL simply returns */
    printf("after_null %llu\n", U(fle_oleaut_bstr_live()));
    CALL(fle_imp_SysFreeString, p);
    printf("after_free %llu\n", U(fle_oleaut_bstr_live()));
    CALL(fle_imp_SysFreeString, q);
    printf("after_free2 %llu\n", U(fle_oleaut_bstr_live()));
"#,
    );
    assert_eq!(l, ["live 2", "after_null 2", "after_free 1", "after_free2 0"]);
}

#[test]
fn sys_realloc_string_len() {
    let l = lines(
        "ole-bstr-realloc",
        r#"
    static const uint16_t hello[] = { 'h', 'e', 'l', 'l', 'o' };
    uint64_t p = bs("abc"), pb = p, r;
    /* page: a new BSTR of len characters copied from psz, the old one freed, *pbstr reset, TRUE */
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), A(hello), 5);
    printf("r %llu changed %d is_hello %d live %llu\n", U(r), pb != p, bs_is(pb, "hello"), U(fle_oleaut_bstr_live()));
    /* shorter */
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), A(hello), 2);
    printf("r %llu is_he %d live %llu\n", U(r), bs_is(pb, "he"), U(fle_oleaut_bstr_live()));
    /* psz NULL: allocated, not initialized (page); length 4, zero contents (setting) */
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), 0, 4);
    printf("r %llu prefix %u c0 %x term %x live %llu\n", U(r), H(fle_ld(pb - 4, 4)), H(fle_ld(pb, 2)), H(fle_ld(pb + 8, 2)), U(fle_oleaut_bstr_live()));
    /* psz inside the old string (page: "cannot be part of the string"; read-before-free keeps it safe when it stays inside) */
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), A(hello), 5);
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), pb + 4, 2);
    printf("inner r %llu is_ll %d\n", U(r), bs_is(pb, "ll"));
    /* a NULL BSTR in *pbstr: nothing to free (String Manipulation Functions page: a null pointer is a valid BSTR) */
    CALL(fle_imp_SysFreeString, pb);
    pb = 0;
    r = CALL(fle_imp_SysReAllocStringLen, A(&pb), A(hello), 3);
    printf("from_null r %llu is_hel %d live %llu\n", U(r), bs_is(pb, "hel"), U(fle_oleaut_bstr_live()));
    CALL(fle_imp_SysFreeString, pb);
    printf("end %llu\n", U(fle_oleaut_bstr_live()));
"#,
    );
    assert_eq!(
        l,
        [
            "r 1 changed 1 is_hello 1 live 1",
            "r 1 is_he 1 live 1",
            "r 1 prefix 8 c0 0 term 0 live 1",
            "inner r 1 is_ll 1",
            "from_null r 1 is_hel 1 live 1",
            "end 0",
        ]
    );
}

#[test]
fn bstr_inputs_the_pages_leave_open_abort() {
    assert_aborts(
        "ole-bstr-abort",
        "static uint16_t lit[] = { 'a', 0 }; uint64_t p = bs(\"abc\"), pb = p;",
        &[
            ("free-twice", "CALL(fle_imp_SysFreeString, p); CALL(fle_imp_SysFreeString, p);"),
            ("free-plain-string", "CALL(fle_imp_SysFreeString, A(lit));"),
            ("free-stray", "CALL(fle_imp_SysFreeString, 0x1234);"),
            ("realloc-null-pbstr", "CALL(fle_imp_SysReAllocStringLen, 0, A(lit), 1);"),
            ("realloc-freed-old", "CALL(fle_imp_SysFreeString, p); CALL(fle_imp_SysReAllocStringLen, A(&pb), A(lit), 1);"),
            ("realloc-reads-past-old", "CALL(fle_imp_SysReAllocStringLen, A(&pb), p + 2, 9);"),
        ],
    );
}

#[test]
fn bstr_requests_above_the_limit_return_null() {
    let l = lines(
        "ole-bstr-limit",
        r#"
    uint64_t pb = bs("ab"), p = CALL(fle_imp_SysAllocStringLen, 0, 0xFFFFFFF0u);
    uint64_t r = CALL(fle_imp_SysReAllocStringLen, A(&pb), 0, 0xFFFFFFF0u);
    /* page: NULL / FALSE when memory is insufficient; *pbstr unchanged (inference) */
    printf("alloc %llu realloc %llu unchanged %d\n", U(p), U(r), bs_is(pb, "ab"));
"#,
    );
    assert_eq!(l, ["alloc 0 realloc 0 unchanged 1"]);
}

// ---- 3. VARIANT ----

#[test]
fn variant_init_sets_only_vt() {
    let l = lines(
        "ole-var-init",
        r#"
    V v;
    memset(&v, 0xAA, sizeof v);
    CALL(fle_imp_VariantInit, A(&v));
    /* page: sets the vt field to VT_EMPTY; the rest is not interpreted */
    printf("vt %u r1 %x val %llx pad %llx\n", v.vt, v.r1, U(v.v.u), U(v.pad));
"#,
    );
    assert_eq!(l, ["vt 0 r1 aaaa val aaaaaaaaaaaaaaaa pad aaaaaaaaaaaaaaaa"]);
}

#[test]
fn variant_clear_plain_bstr_and_array() {
    let l = lines(
        "ole-var-clear",
        r#"
    V v; SA *a; uint32_t hr; uint64_t s;
    /* a value type: vt becomes VT_EMPTY */
    memset(&v, 0, sizeof v); v.vt = VT_I4_; v.v.l = 7;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("i4 %08x vt %u\n", hr, v.vt);
    memset(&v, 0, sizeof v); v.vt = VT_EMPTY_;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("empty %08x\n", hr);
    memset(&v, 0, sizeof v); v.vt = VT_NULL_;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("null %08x vt %u\n", hr, v.vt);
    /* page: VT_BSTR: the string is freed */
    s = bs("abc");
    memset(&v, 0, sizeof v); v.vt = VT_BSTR_; v.v.u = s;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("bstr %08x vt %u live %llu\n", hr, v.vt, U(fle_oleaut_bstr_live()));
    /* a NULL BSTR has nothing to free */
    memset(&v, 0, sizeof v); v.vt = VT_BSTR_;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("null_bstr %08x vt %u\n", hr, v.vt);
    /* page: the VT_ARRAY bit: the array is freed; a locked one is DISP_E_ARRAYISLOCKED and the variant keeps its type */
    a = mk(VT_I4_, 3, 0);
    memset(&v, 0, sizeof v); v.vt = VT_ARRAY_ | VT_I4_; v.v.p = a;
    printf("arrays %llu\n", U(fle_oleaut_array_live()));
    { void *d; CALL(fle_imp_SafeArrayAccessData, A(a), A(&d)); }
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("locked %08x vt %x arrays %llu\n", hr, v.vt, U(fle_oleaut_array_live()));
    CALL(fle_imp_SafeArrayUnaccessData, A(a));
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("unlocked %08x vt %x arrays %llu\n", hr, v.vt, U(fle_oleaut_array_live()));
    /* page: VT_BYREF | VT_UNKNOWN / VT_DISPATCH: the object is not released, clearing succeeds */
    memset(&v, 0, sizeof v); v.vt = VT_UNKNOWN_ | VT_BYREF_; v.v.u = 0x1234;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("byref_unk %08x vt %u\n", hr, v.vt);
    memset(&v, 0, sizeof v); v.vt = VT_DISPATCH_ | VT_BYREF_; v.v.u = 0x1234;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("byref_disp %08x vt %u\n", hr, v.vt);
    /* by-reference value types own nothing */
    { int16_t x = 5; memset(&v, 0, sizeof v); v.vt = VT_I2_ | VT_BYREF_; v.v.p = &x;
      hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("byref_i2 %08x vt %u x %d\n", hr, v.vt, x); }
    /* an object type whose pointer is NULL: nothing to release (inference) */
    memset(&v, 0, sizeof v); v.vt = VT_UNKNOWN_;
    hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v)); printf("null_unk %08x vt %u\n", hr, v.vt);
"#,
    );
    assert_eq!(
        l,
        [
            format!("i4 {S_OK} vt 0").as_str(),
            format!("empty {S_OK}").as_str(),
            format!("null {S_OK} vt 0").as_str(),
            format!("bstr {S_OK} vt 0 live 0").as_str(),
            format!("null_bstr {S_OK} vt 0").as_str(),
            "arrays 1",
            format!("locked {DISP_E_ARRAYISLOCKED} vt 2003 arrays 1").as_str(),
            format!("unlocked {S_OK} vt 0 arrays 0").as_str(),
            format!("byref_unk {S_OK} vt 0").as_str(),
            format!("byref_disp {S_OK} vt 0").as_str(),
            format!("byref_i2 {S_OK} vt 0 x 5").as_str(),
            format!("null_unk {S_OK} vt 0").as_str(),
        ]
    );
}

#[test]
fn variant_clear_invalid_types_and_null() {
    let l = lines(
        "ole-var-clear-bad",
        r#"
    V v; uint32_t hr;
    static const uint16_t bad[] = { 0x1000 /* VT_VECTOR: property sets only */, 0x8000 /* VT_RESERVED */, 0xFFFF /* VT_ILLEGAL */, 0x0777, 25 /* VT_HRESULT */, 31 /* VT_LPWSTR */, 0x4000 | 25 };
    int i;
    for (i = 0; i < 7; i++) {
        memset(&v, 0, sizeof v); v.vt = bad[i];
        hr = (uint32_t)CALL(fle_imp_VariantClear, A(&v));
        printf("%04x %08x vt %04x\n", bad[i], hr, v.vt);
    }
    hr = (uint32_t)CALL(fle_imp_VariantClear, 0); printf("null %08x\n", hr);
"#,
    );
    let mut want: Vec<String> = [(0x1000u32, 0x1000u32), (0x8000, 0x8000), (0xffff, 0xffff), (0x0777, 0x0777), (25, 25), (31, 31), (0x4019, 0x4019)]
        .iter()
        .map(|(t, k)| format!("{t:04x} {DISP_E_BADVARTYPE} vt {k:04x}"))
        .collect();
    want.push(format!("null {E_INVALIDARG}"));
    assert_eq!(l, want);
}

#[test]
fn variant_clear_inputs_that_need_a_callback_abort() {
    assert_aborts(
        "ole-var-clear-abort",
        "V v; SA *a; memset(&v, 0, sizeof v);",
        &[
            ("unknown", "v.vt = VT_UNKNOWN_; v.v.u = 0x1234; CALL(fle_imp_VariantClear, A(&v));"),
            ("dispatch", "v.vt = VT_DISPATCH_; v.v.u = 0x1234; CALL(fle_imp_VariantClear, A(&v));"),
            ("record", "v.vt = VT_RECORD_; CALL(fle_imp_VariantClear, A(&v));"),
            (
                "array-of-unknown-with-object",
                "a = mk(VT_UNKNOWN_, 2, 0); ((uint64_t *)a->pvData)[1] = 0x1234; v.vt = VT_ARRAY_ | VT_UNKNOWN_; v.v.p = a; CALL(fle_imp_VariantClear, A(&v));",
            ),
            ("bstr-not-live", "v.vt = VT_BSTR_; v.v.u = 0x1234; CALL(fle_imp_VariantClear, A(&v));"),
            ("array-null", "v.vt = VT_ARRAY_ | VT_I4_; CALL(fle_imp_VariantClear, A(&v));"),
            ("array-stray", "v.vt = VT_ARRAY_ | VT_I4_; v.v.u = 0x1234; CALL(fle_imp_VariantClear, A(&v));"),
            ("variant-not-byref", "v.vt = VT_VARIANT_; CALL(fle_imp_VariantClear, A(&v));"),
            ("i8-undecided", "v.vt = 20; CALL(fle_imp_VariantClear, A(&v));"),
            ("byref-null-type", "v.vt = VT_NULL_ | VT_BYREF_; CALL(fle_imp_VariantClear, A(&v));"),
        ],
    );
}

#[test]
fn variant_copy() {
    let l = lines(
        "ole-var-copy",
        r#"
    V s, d; SA *a, *c; uint32_t hr; uint64_t p;
    /* page: the destination receives an exact copy */
    memset(&s, 0, sizeof s); s.vt = VT_R8_; s.v.d = 2.5; s.r1 = 0x1111; s.pad = 0x77;
    memset(&d, 0xCC, sizeof d); d.vt = VT_EMPTY_;
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("r8 %08x same %d\n", hr, memcmp(&d, &s, sizeof s) == 0);
    /* VT_BSTR: a copy of the string is made, the old destination contents are freed first */
    memset(&d, 0, sizeof d); d.vt = VT_BSTR_; d.v.u = bs("old");
    memset(&s, 0, sizeof s); s.vt = VT_BSTR_; s.v.u = bs("new!");
    printf("live_before %llu\n", U(fle_oleaut_bstr_live()));
    p = d.v.u;
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("bstr %08x vt %u distinct %d is_new %d src_ok %d live %llu\n", hr, d.vt, d.v.u != s.v.u, bs_is(d.v.u, "new!"), bs_is(s.v.u, "new!"), U(fle_oleaut_bstr_live()));
    /* a NULL BSTR source stays NULL (a null pointer is a BSTR with zero characters; inference for the copy) */
    CALL(fle_imp_VariantClear, A(&s)); CALL(fle_imp_VariantClear, A(&d));
    memset(&s, 0, sizeof s); s.vt = VT_BSTR_;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("null_bstr %08x vt %u val %llx live %llu\n", hr, d.vt, U(d.v.u), U(fle_oleaut_bstr_live()));
    /* page: a VT_ARRAY source: the entire array is copied */
    a = mk(VT_I4_, 3, 1);
    { int32_t *e = (int32_t *)a->pvData; e[0] = 10; e[1] = 20; e[2] = 30; }
    memset(&s, 0, sizeof s); s.vt = VT_ARRAY_ | VT_I4_; s.v.p = a;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    c = (SA *)d.v.p;
    printf("array %08x vt %x distinct %d data_distinct %d dims %u cb %u b %u,%d e %d %d %d arrays %llu\n", hr, d.vt, c != a, c->pvData != a->pvData, c->cDims, c->cbElements,
           c->b[0].cElements, c->b[0].lLbound, ((int32_t *)c->pvData)[0], ((int32_t *)c->pvData)[1], ((int32_t *)c->pvData)[2], U(fle_oleaut_array_live()));
    CALL(fle_imp_VariantClear, A(&d)); CALL(fle_imp_VariantClear, A(&s));
    printf("arrays_after %llu\n", U(fle_oleaut_array_live()));
    /* page: VT_BYREF | VT_UNKNOWN: the reference count is not changed, the variant is copied as it is */
    memset(&s, 0, sizeof s); s.vt = VT_UNKNOWN_ | VT_BYREF_; s.v.u = 0x1234;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("byref_unk %08x vt %x val %llx\n", hr, d.vt, U(d.v.u));
    /* an object type with a NULL pointer: nothing to AddRef (inference) */
    memset(&s, 0, sizeof s); s.vt = VT_DISPATCH_;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("null_disp %08x vt %u\n", hr, d.vt);
    /* an invalid source type: DISP_E_BADVARTYPE and the destination is not touched */
    memset(&s, 0, sizeof s); s.vt = 0x1000;
    memset(&d, 0, sizeof d); d.vt = VT_I4_; d.v.l = 9;
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("bad_src %08x dst_vt %u dst_val %d\n", hr, d.vt, d.v.l);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, 0, A(&s)); printf("null_dst %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), 0); printf("null_src %08x\n", hr);
    /* a locked array in the destination: the clear fails and the copy is not made */
    a = mk(VT_I4_, 1, 0);
    { void *dd; CALL(fle_imp_SafeArrayAccessData, A(a), A(&dd)); }
    memset(&d, 0, sizeof d); d.vt = VT_ARRAY_ | VT_I4_; d.v.p = a;
    memset(&s, 0, sizeof s); s.vt = VT_I4_; s.v.l = 3;
    hr = (uint32_t)CALL(fle_imp_VariantCopy, A(&d), A(&s));
    printf("locked_dst %08x dst_vt %x\n", hr, d.vt);
"#,
    );
    assert_eq!(
        l,
        [
            format!("r8 {S_OK} same 1").as_str(),
            "live_before 2",
            format!("bstr {S_OK} vt 8 distinct 1 is_new 1 src_ok 1 live 2").as_str(),
            format!("null_bstr {S_OK} vt 8 val 0 live 0").as_str(),
            format!("array {S_OK} vt 2003 distinct 1 data_distinct 1 dims 1 cb 4 b 3,1 e 10 20 30 arrays 2").as_str(),
            "arrays_after 0",
            format!("byref_unk {S_OK} vt 400d val 1234").as_str(),
            format!("null_disp {S_OK} vt 9").as_str(),
            format!("bad_src {DISP_E_BADVARTYPE} dst_vt 3 dst_val 9").as_str(),
            format!("null_dst {E_INVALIDARG}").as_str(),
            format!("null_src {E_INVALIDARG}").as_str(),
            format!("locked_dst {DISP_E_ARRAYISLOCKED} dst_vt 2003").as_str(),
        ]
    );
}

#[test]
fn variant_copy_inputs_that_need_a_callback_abort() {
    assert_aborts(
        "ole-var-copy-abort",
        "V s, d; SA *a; memset(&s, 0, sizeof s); memset(&d, 0, sizeof d);",
        &[
            ("src-unknown", "s.vt = VT_UNKNOWN_; s.v.u = 0x1234; CALL(fle_imp_VariantCopy, A(&d), A(&s));"),
            ("src-dispatch", "s.vt = VT_DISPATCH_; s.v.u = 0x1234; CALL(fle_imp_VariantCopy, A(&d), A(&s));"),
            ("src-record", "s.vt = VT_RECORD_; CALL(fle_imp_VariantCopy, A(&d), A(&s));"),
            ("dst-unknown", "d.vt = VT_UNKNOWN_; d.v.u = 0x1234; s.vt = VT_I4_; CALL(fle_imp_VariantCopy, A(&d), A(&s));"),
            (
                "src-array-of-unknown-with-object",
                "a = mk(VT_UNKNOWN_, 1, 0); ((uint64_t *)a->pvData)[0] = 0x1234; s.vt = VT_ARRAY_ | VT_UNKNOWN_; s.v.p = a; CALL(fle_imp_VariantCopy, A(&d), A(&s));",
            ),
            ("same-variant", "s.vt = VT_I4_; CALL(fle_imp_VariantCopy, A(&s), A(&s));"),
            ("src-bstr-not-live", "s.vt = VT_BSTR_; s.v.u = 0x1234; CALL(fle_imp_VariantCopy, A(&d), A(&s));"),
        ],
    );
}

#[test]
fn variant_copy_ind() {
    let l = lines(
        "ole-var-copyind",
        r#"
    V s, d, inner, inner2; uint32_t hr; int16_t x = -3; int32_t y = 77; double z = 1.5; uint64_t b = bs("by-ref");
    /* page: (VT_BYREF | VT_I2) becomes a by-value VT_I2 */
    memset(&s, 0, sizeof s); s.vt = VT_I2_ | VT_BYREF_; s.v.p = &x;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("i2 %08x vt %u val %d\n", hr, d.vt, d.v.i);
    s.vt = VT_I4_ | VT_BYREF_; s.v.p = &y;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("i4 %08x vt %u val %d\n", hr, d.vt, d.v.l);
    s.vt = VT_R8_ | VT_BYREF_; s.v.p = &z;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("r8 %08x vt %u val %.1f\n", hr, d.vt, d.v.d);
    /* VT_BYREF | VT_BSTR: the string is copied */
    s.vt = VT_BSTR_ | VT_BYREF_; s.v.p = &b;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("bstr %08x vt %u distinct %d is %d live %llu\n", hr, d.vt, d.v.u != b, bs_is(d.v.u, "by-ref"), U(fle_oleaut_bstr_live()));
    CALL(fle_imp_VariantClear, A(&d));
    /* (VT_BYREF | VT_VARIANT): the contained variant is copied */
    memset(&inner, 0, sizeof inner); inner.vt = VT_I4_; inner.v.l = 1234;
    s.vt = VT_VARIANT_ | VT_BYREF_; s.v.p = &inner;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("var %08x vt %u val %d\n", hr, d.vt, d.v.l);
    /* page: when the contained variant is VT_BYREF, it is dereferenced too */
    memset(&inner2, 0, sizeof inner2); inner2.vt = VT_I2_ | VT_BYREF_; inner2.v.p = &x;
    s.v.p = &inner2;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("var_byref %08x vt %u val %d\n", hr, d.vt, d.v.i);
    /* a source that is not by reference is copied as VariantCopy does */
    memset(&s, 0, sizeof s); s.vt = VT_I4_; s.v.l = 5;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("plain %08x vt %u val %d\n", hr, d.vt, d.v.l);
    /* the old destination contents are freed ("frees any existing contents of pvarDest") */
    memset(&d, 0, sizeof d); d.vt = VT_BSTR_; d.v.u = bs("gone");
    printf("live_before %llu\n", U(fle_oleaut_bstr_live()));
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s));
    printf("freed %08x live %llu\n", hr, U(fle_oleaut_bstr_live()));
    memset(&s, 0, sizeof s); s.vt = 0x1000;
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, A(&d), A(&s)); printf("bad %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_VariantCopyInd, 0, A(&s)); printf("null %08x\n", hr);
"#,
    );
    assert_eq!(
        l,
        [
            format!("i2 {S_OK} vt 2 val -3").as_str(),
            format!("i4 {S_OK} vt 3 val 77").as_str(),
            format!("r8 {S_OK} vt 5 val 1.5").as_str(),
            format!("bstr {S_OK} vt 8 distinct 1 is 1 live 2").as_str(),
            format!("var {S_OK} vt 3 val 1234").as_str(),
            format!("var_byref {S_OK} vt 2 val -3").as_str(),
            format!("plain {S_OK} vt 3 val 5").as_str(),
            "live_before 2",
            format!("freed {S_OK} live 1").as_str(),
            format!("bad {DISP_E_BADVARTYPE}").as_str(),
            format!("null {E_INVALIDARG}").as_str(),
        ]
    );
}

#[test]
fn variant_copy_ind_inputs_the_pages_leave_open_abort() {
    assert_aborts(
        "ole-var-copyind-abort",
        "V s, d, inner; int32_t y = 1; memset(&s, 0, sizeof s); memset(&d, 0, sizeof d); memset(&inner, 0, sizeof inner);",
        &[
            ("byref-null-pointer", "s.vt = VT_I4_ | VT_BYREF_; CALL(fle_imp_VariantCopyInd, A(&d), A(&s));"),
            ("byref-unknown-with-object", "uint64_t o = 0x1234; s.vt = VT_UNKNOWN_ | VT_BYREF_; s.v.p = &o; CALL(fle_imp_VariantCopyInd, A(&d), A(&s));"),
            (
                "variant-inside-variant-byref",
                "inner.vt = VT_VARIANT_ | VT_BYREF_; inner.v.p = &inner; s.vt = VT_VARIANT_ | VT_BYREF_; s.v.p = &inner; CALL(fle_imp_VariantCopyInd, A(&d), A(&s));",
            ),
            ("src-unknown-with-object", "s.vt = VT_UNKNOWN_; s.v.u = 0x1234; CALL(fle_imp_VariantCopyInd, A(&d), A(&s));"),
            ("same-variant", "s.vt = VT_I4_; CALL(fle_imp_VariantCopyInd, A(&s), A(&s));"),
        ],
    );
}

#[test]
fn variant_change_type() {
    let l = lines(
        "ole-var-change",
        r#"
    V s, d; uint32_t hr; int16_t x = 12; uint64_t p;
    /* the page: on S_OK the vt of the destination is the requested type; same type: the value is unchanged (inference) */
    memset(&s, 0, sizeof s); s.vt = VT_I4_; s.v.l = -9;
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_);
    printf("same %08x vt %u val %d\n", hr, d.vt, d.v.l);
    /* dest == src converts in place */
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&s), A(&s), 0, VT_I4_);
    printf("inplace %08x vt %u val %d\n", hr, s.vt, s.v.l);
    /* page: a VT_BYREF source is coerced by reading the referenced value */
    memset(&s, 0, sizeof s); s.vt = VT_I2_ | VT_BYREF_; s.v.p = &x;
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I2_);
    printf("byref %08x vt %u val %d\n", hr, d.vt, d.v.i);
    /* a BSTR to BSTR is a copy */
    memset(&s, 0, sizeof s); s.vt = VT_BSTR_; s.v.u = bs("txt");
    memset(&d, 0, sizeof d);
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_BSTR_);
    printf("bstr %08x vt %u distinct %d is %d\n", hr, d.vt, d.v.u != s.v.u, bs_is(d.v.u, "txt"));
    /* invalid types: DISP_E_BADVARTYPE */
    memset(&s, 0, sizeof s); s.vt = VT_I4_;
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, 0x1000); printf("bad_target %08x\n", hr);
    s.vt = 0x1000;
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_); printf("bad_source %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, 0, A(&s), 0, VT_I4_); printf("null_dst %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_VariantChangeType, A(&d), 0, 0, VT_I4_); printf("null_src %08x\n", hr);
"#,
    );
    assert_eq!(
        l,
        [
            format!("same {S_OK} vt 3 val -9").as_str(),
            format!("inplace {S_OK} vt 3 val -9").as_str(),
            format!("byref {S_OK} vt 2 val 12").as_str(),
            format!("bstr {S_OK} vt 8 distinct 1 is 1").as_str(),
            format!("bad_target {DISP_E_BADVARTYPE}").as_str(),
            format!("bad_source {DISP_E_BADVARTYPE}").as_str(),
            format!("null_dst {E_INVALIDARG}").as_str(),
            format!("null_src {E_INVALIDARG}").as_str(),
        ]
    );
}

#[test]
fn variant_change_type_coercions_the_page_does_not_specify_abort() {
    assert_aborts(
        "ole-var-change-abort",
        "V s, d; int16_t x = 1; memset(&s, 0, sizeof s); memset(&d, 0, sizeof d);",
        &[
            ("i4-to-bstr", "s.vt = VT_I4_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_BSTR_);"),
            ("bstr-to-i4", "s.vt = VT_BSTR_; s.v.u = bs(\"12\"); CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_);"),
            ("bool-to-uint", "s.vt = VT_BOOL_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, 23);"),
            ("flags-nonzero", "s.vt = VT_I4_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 4, VT_I4_);"),
            ("unknown-to-i4", "s.vt = VT_UNKNOWN_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_);"),
            ("byref-i2-to-i4", "s.vt = VT_I2_ | VT_BYREF_; s.v.p = &x; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_);"),
            ("array-source", "s.vt = VT_ARRAY_ | VT_I4_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_I4_);"),
            ("array-target", "s.vt = VT_I4_; CALL(fle_imp_VariantChangeType, A(&d), A(&s), 0, VT_ARRAY_ | VT_I4_);"),
        ],
    );
}

// ---- 4. SAFEARRAY ----

#[test]
fn safe_array_create_descriptor_and_bounds() {
    let l = lines(
        "ole-sa-create",
        r#"
    SAB b = { 5, -2 };
    SA *a = (SA *)CALL(fle_imp_SafeArrayCreate, VT_I4_, 1, A(&b));
    int32_t lo = 99, hi = 99; uint32_t hr; int i, zero = 1;
    /* SAFEARRAY page: cDims, cbElements, cLocks, pvData, rgsabound[] (SAFEARRAYBOUND: cElements, lLbound) */
    printf("a %d dims %u cb %u locks %u bound %u,%d feat_vartype %d\n", a != 0, a->cDims, a->cbElements, a->cLocks, a->b[0].cElements, a->b[0].lLbound, (a->fFeatures & 0x0080) != 0);
    /* SafeArrayCreate page: the data is allocated and initialized (zero here: inference) */
    for (i = 0; i < 5 * 4; i++) if (((uint8_t *)a->pvData)[i]) zero = 0;
    printf("data %d zero %d arrays %llu\n", a->pvData != 0, zero, U(fle_oleaut_array_live()));
    printf("elemsize %llu\n", U(CALL(fle_imp_SafeArrayGetElemsize, A(a))));
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetLBound, A(a), 1, A(&lo)); printf("lb %08x %d\n", hr, lo);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetUBound, A(a), 1, A(&hi)); printf("ub %08x %d\n", hr, hi);
    /* page: DISP_E_BADINDEX when the dimension is out of bounds (the first dimension is 1) */
    lo = hi = 99;
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetLBound, A(a), 0, A(&lo)); printf("lb_dim0 %08x %d\n", hr, lo);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetUBound, A(a), 2, A(&hi)); printf("ub_dim2 %08x %d\n", hr, hi);
    /* E_INVALIDARG for NULL arguments */
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetLBound, 0, 1, A(&lo)); printf("lb_null_psa %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetUBound, A(a), 1, 0); printf("ub_null_out %08x\n", hr);
    /* page: DISP_E_OVERFLOW when the upper bound does not fit a LONG */
    { SAB o = { 2, 0x7FFFFFFF }; SA *c = (SA *)CALL(fle_imp_SafeArrayCreate, VT_UI1_, 1, A(&o));
      hr = (uint32_t)CALL(fle_imp_SafeArrayGetUBound, A(c), 1, A(&hi)); printf("ub_overflow %08x\n", hr);
      hr = (uint32_t)CALL(fle_imp_SafeArrayGetLBound, A(c), 1, A(&lo)); printf("lb_big %08x %d\n", hr, lo); }
    /* a one-element array whose upper bound is the lower bound */
    { SAB o = { 1, 7 }; SA *c = (SA *)CALL(fle_imp_SafeArrayCreate, VT_I2_, 1, A(&o));
      hr = (uint32_t)CALL(fle_imp_SafeArrayGetUBound, A(c), 1, A(&hi)); printf("ub_one %08x %d\n", hr, hi); }
"#,
    );
    assert_eq!(
        l,
        [
            "a 1 dims 1 cb 4 locks 0 bound 5,-2 feat_vartype 1".to_string(),
            "data 1 zero 1 arrays 1".to_string(),
            "elemsize 4".to_string(),
            format!("lb {S_OK} -2"),
            // lLbound + cElements - 1 = -2 + 5 - 1
            format!("ub {S_OK} 2"),
            format!("lb_dim0 {DISP_E_BADINDEX} 99"),
            format!("ub_dim2 {DISP_E_BADINDEX} 99"),
            format!("lb_null_psa {E_INVALIDARG}"),
            format!("ub_null_out {E_INVALIDARG}"),
            format!("ub_overflow {DISP_E_OVERFLOW}"),
            format!("lb_big {S_OK} 2147483647"),
            format!("ub_one {S_OK} 7"),
        ]
    );
}

#[test]
fn safe_array_create_element_types_sizes_and_failures() {
    let l = lines(
        "ole-sa-types",
        r#"
    static const struct { unsigned vt; uint32_t size; unsigned feat; } t[] = {
        { 17, 1, 0x80 }, { 16, 1, 0x80 }, { 2, 2, 0x80 }, { 18, 2, 0x80 }, { 11, 2, 0x80 }, { 3, 4, 0x80 }, { 19, 4, 0x80 },
        { 22, 4, 0x80 }, { 23, 4, 0x80 }, { 4, 4, 0x80 }, { 10, 4, 0x80 }, { 5, 8, 0x80 }, { 6, 8, 0x80 }, { 7, 8, 0x80 },
        { 14, 16, 0x80 }, { 8, 8, 0x100 }, { 13, 8, 0x200 }, { 9, 8, 0x400 }, { 12, 24, 0x800 },
    };
    int i;
    SAB b = { 2, 0 };
    for (i = 0; i < 19; i++) {
        SA *a = (SA *)CALL(fle_imp_SafeArrayCreate, t[i].vt, 1, A(&b));
        printf("%u %d size %u feat %x\n", t[i].vt, a != 0, a ? H(CALL(fle_imp_SafeArrayGetElemsize, A(a))) : 0, a ? a->fFeatures : 0);
    }
    /* SafeArrayCreate page: VT_EMPTY, VT_NULL, VT_ARRAY and VT_BYREF are not valid: NULL */
    printf("empty %llu null %llu array %llu byref %llu\n", U(CALL(fle_imp_SafeArrayCreate, 0, 1, A(&b))), U(CALL(fle_imp_SafeArrayCreate, 1, 1, A(&b))),
           U(CALL(fle_imp_SafeArrayCreate, 0x2000 | 3, 1, A(&b))), U(CALL(fle_imp_SafeArrayCreate, 0x4000 | 3, 1, A(&b))));
    /* a request above the limit of the virtual machine: NULL ("could not be created") */
    { SAB big = { 0xFFFFFFFFu, 0 }; printf("huge %llu\n", U(CALL(fle_imp_SafeArrayCreate, 12, 1, A(&big)))); }
    printf("null_bounds %llu\n", U(CALL(fle_imp_SafeArrayCreate, 3, 1, 0)));
"#,
    );
    let want = [
        (17, 1, 0x80),
        (16, 1, 0x80),
        (2, 2, 0x80),
        (18, 2, 0x80),
        (11, 2, 0x80),
        (3, 4, 0x80),
        (19, 4, 0x80),
        (22, 4, 0x80),
        (23, 4, 0x80),
        (4, 4, 0x80),
        (10, 4, 0x80),
        (5, 8, 0x80),
        (6, 8, 0x80),
        (7, 8, 0x80),
        (14, 16, 0x80),
        (8, 8, 0x100),
        (13, 8, 0x200),
        (9, 8, 0x400),
        (12, 24, 0x800),
    ];
    let mut w: Vec<String> = want.iter().map(|(vt, sz, f)| format!("{vt} 1 size {sz} feat {f:x}")).collect();
    w.push("empty 0 null 0 array 0 byref 0".into());
    w.push("huge 0".into());
    w.push("null_bounds 0".into());
    assert_eq!(l, w);
}

#[test]
fn safe_array_create_inputs_the_pages_leave_open_abort() {
    assert_aborts(
        "ole-sa-create-abort",
        "SAB b = { 2, 0 }; SAB z = { 0, 0 };",
        &[
            ("two-dims", "CALL(fle_imp_SafeArrayCreate, VT_I4_, 2, A(&b));"),
            ("zero-dims", "CALL(fle_imp_SafeArrayCreate, VT_I4_, 0, A(&b));"),
            ("zero-elements", "CALL(fle_imp_SafeArrayCreate, VT_I4_, 1, A(&z));"),
            ("record", "CALL(fle_imp_SafeArrayCreate, VT_RECORD_, 1, A(&b));"),
            ("i8", "CALL(fle_imp_SafeArrayCreate, 20, 1, A(&b));"),
            ("vt-hresult", "CALL(fle_imp_SafeArrayCreate, 25, 1, A(&b));"),
        ],
    );
}

#[test]
fn safe_array_access_lock_count() {
    let l = lines(
        "ole-sa-lock",
        r#"
    SA *a = mk(VT_I4_, 4, 0); void *d1 = 0, *d2 = 0; uint32_t hr;
    hr = (uint32_t)CALL(fle_imp_SafeArrayAccessData, A(a), A(&d1)); printf("acc1 %08x locks %u same %d\n", hr, a->cLocks, d1 == a->pvData);
    hr = (uint32_t)CALL(fle_imp_SafeArrayAccessData, A(a), A(&d2)); printf("acc2 %08x locks %u same %d\n", hr, a->cLocks, d2 == d1);
    /* the pointer is usable */
    ((int32_t *)d1)[3] = 41;
    hr = (uint32_t)CALL(fle_imp_SafeArrayUnaccessData, A(a)); printf("un1 %08x locks %u\n", hr, a->cLocks);
    hr = (uint32_t)CALL(fle_imp_SafeArrayUnaccessData, A(a)); printf("un2 %08x locks %u\n", hr, a->cLocks);
    /* SAFEARRAY page: an unlock without a lock leaves an inconsistent count and E_UNEXPECTED */
    hr = (uint32_t)CALL(fle_imp_SafeArrayUnaccessData, A(a)); printf("un3 %08x locks %u\n", hr, a->cLocks);
    hr = (uint32_t)CALL(fle_imp_SafeArrayAccessData, 0, A(&d1)); printf("null_psa %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayAccessData, A(a), 0); printf("null_out %08x locks %u\n", hr, a->cLocks);
    hr = (uint32_t)CALL(fle_imp_SafeArrayUnaccessData, 0); printf("un_null %08x\n", hr);
    a->cLocks = 0xFFFFFFFFu;
    hr = (uint32_t)CALL(fle_imp_SafeArrayAccessData, A(a), A(&d1)); printf("overflow %08x\n", hr);
    a->cLocks = 0;
"#,
    );
    assert_eq!(
        l,
        [
            format!("acc1 {S_OK} locks 1 same 1").as_str(),
            format!("acc2 {S_OK} locks 2 same 1").as_str(),
            format!("un1 {S_OK} locks 1").as_str(),
            format!("un2 {S_OK} locks 0").as_str(),
            format!("un3 {E_UNEXPECTED} locks 0").as_str(),
            format!("null_psa {E_INVALIDARG}").as_str(),
            format!("null_out {E_INVALIDARG} locks 0").as_str(),
            format!("un_null {E_INVALIDARG}").as_str(),
            format!("overflow {E_UNEXPECTED}").as_str(),
        ]
    );
}

#[test]
fn safe_array_get_put_ptr_of_index_plain_elements() {
    let l = lines(
        "ole-sa-elems",
        r#"
    SA *a = mk(VT_I4_, 3, 10); int32_t i, v, *pv; uint32_t hr; void *pp = 0;
    for (i = 10; i <= 12; i++) { v = i * 100; hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), A(&v)); printf("put %d %08x\n", i, hr); }
    for (i = 10; i <= 12; i++) { v = 0; hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), A(&v)); printf("get %d %08x %d\n", i, hr, v); }
    /* the elements are laid out in index order in pvData */
    printf("raw %d %d %d\n", ((int32_t *)a->pvData)[0], ((int32_t *)a->pvData)[1], ((int32_t *)a->pvData)[2]);
    i = 11; hr = (uint32_t)CALL(fle_imp_SafeArrayPtrOfIndex, A(a), A(&i), A(&pp)); printf("ptr %08x %d\n", hr, pp == (char *)a->pvData + 4);
    pv = (int32_t *)pp; *pv = -5; v = 0; CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), A(&v)); printf("via_ptr %d\n", v);
    /* DISP_E_BADINDEX outside [lLbound, lLbound + cElements - 1] */
    i = 9; v = 7; pp = 0;
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), A(&v)); printf("get_lo %08x %d\n", hr, v);
    i = 13;
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), A(&v)); printf("get_hi %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), A(&v)); printf("put_hi %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayPtrOfIndex, A(a), A(&i), A(&pp)); printf("ptr_hi %08x %d\n", hr, pp == 0);
    /* E_INVALIDARG for NULL arguments */
    i = 10;
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, 0, A(&i), A(&v)); printf("get_null_psa %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), 0, A(&v)); printf("get_null_idx %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), 0); printf("get_null_pv %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), 0); printf("put_null_pv %08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_SafeArrayPtrOfIndex, A(a), A(&i), 0); printf("ptr_null_out %08x\n", hr);
    /* elements can be put while the array is locked (page note) */
    { void *d; CALL(fle_imp_SafeArrayAccessData, A(a), A(&d)); v = 1; hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), A(&v)); printf("put_locked %08x\n", hr); CALL(fle_imp_SafeArrayUnaccessData, A(a)); }
    /* a 2-byte and an 8-byte element type use their own width */
    { SA *s = mk(VT_I2_, 2, 0), *q = mk(VT_R8_, 2, 0); int16_t h = -2; double dd = 6.25, o = 0; int32_t one = 1;
      CALL(fle_imp_SafeArrayPutElement, A(s), A(&one), A(&h)); CALL(fle_imp_SafeArrayPutElement, A(q), A(&one), A(&dd));
      h = 0; CALL(fle_imp_SafeArrayGetElement, A(s), A(&one), A(&h)); CALL(fle_imp_SafeArrayGetElement, A(q), A(&one), A(&o));
      printf("widths %d %.2f %d %d\n", h, o, ((int16_t *)s->pvData)[0], ((int16_t *)s->pvData)[1]); }
"#,
    );
    assert_eq!(
        l,
        [
            format!("put 10 {S_OK}").as_str(),
            format!("put 11 {S_OK}").as_str(),
            format!("put 12 {S_OK}").as_str(),
            format!("get 10 {S_OK} 1000").as_str(),
            format!("get 11 {S_OK} 1100").as_str(),
            format!("get 12 {S_OK} 1200").as_str(),
            "raw 1000 1100 1200",
            format!("ptr {S_OK} 1").as_str(),
            "via_ptr -5",
            format!("get_lo {DISP_E_BADINDEX} 7").as_str(),
            format!("get_hi {DISP_E_BADINDEX}").as_str(),
            format!("put_hi {DISP_E_BADINDEX}").as_str(),
            format!("ptr_hi {DISP_E_BADINDEX} 1").as_str(),
            format!("get_null_psa {E_INVALIDARG}").as_str(),
            format!("get_null_idx {E_INVALIDARG}").as_str(),
            format!("get_null_pv {E_INVALIDARG}").as_str(),
            format!("put_null_pv {E_INVALIDARG}").as_str(),
            format!("ptr_null_out {E_INVALIDARG}").as_str(),
            format!("put_locked {S_OK}").as_str(),
            "widths -2 6.25 0 -2",
        ]
    );
}

#[test]
fn safe_array_bstr_and_variant_elements() {
    let l = lines(
        "ole-sa-owned",
        r#"
    SA *a = mk(VT_BSTR_, 2, 0), *w = mk(VT_VARIANT_, 2, 0); int32_t i = 0, j = 1; uint32_t hr; uint64_t got = 1, s1 = bs("first"), s2 = bs("second");
    V v, out;
    printf("live0 %llu\n", U(fle_oleaut_bstr_live()));
    /* SafeArrayPutElement page: for VT_BSTR, pv is the pointer itself; the string is copied */
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), s1); printf("put0 %08x live %llu\n", hr, U(fle_oleaut_bstr_live()));
    /* replacing an existing element clears it correctly (the old string is freed) */
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&i), s2); printf("put0_again %08x live %llu is %d\n", hr, U(fle_oleaut_bstr_live()), bs_is(((uint64_t *)a->pvData)[0], "second"));
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(a), A(&j), 0); printf("put_null_bstr %08x elem %llx\n", hr, U(((uint64_t *)a->pvData)[1]));
    /* SafeArrayGetElement page: a string is copied in the correct way (a new BSTR) */
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&i), A(&got));
    printf("get0 %08x distinct %d is %d live %llu\n", hr, got != ((uint64_t *)a->pvData)[0], bs_is(got, "second"), U(fle_oleaut_bstr_live()));
    CALL(fle_imp_SysFreeString, got);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(a), A(&j), A(&got)); printf("get_null %08x val %llx\n", hr, U(got));
    /* VARIANT elements: a copy goes in, the old element is cleared, a copy comes out */
    memset(&v, 0, sizeof v); v.vt = VT_BSTR_; v.v.u = bs("in-variant");
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(w), A(&i), A(&v)); printf("vput %08x vt %u live %llu\n", hr, ((V *)w->pvData)[0].vt, U(fle_oleaut_bstr_live()));
    CALL(fle_imp_VariantClear, A(&v));
    memset(&out, 0, sizeof out);
    hr = (uint32_t)CALL(fle_imp_SafeArrayGetElement, A(w), A(&i), A(&out)); printf("vget %08x vt %u distinct %d is %d\n", hr, out.vt, out.v.u != ((V *)w->pvData)[0].v.u, bs_is(out.v.u, "in-variant"));
    /* page: elements default to a cleared (VT_EMPTY) VARIANT */
    printf("vempty %u\n", ((V *)w->pvData)[1].vt);
    memset(&v, 0, sizeof v); v.vt = VT_I4_; v.v.l = 3;
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(w), A(&i), A(&v)); printf("vput_over %08x vt %u live %llu\n", hr, ((V *)w->pvData)[0].vt, U(fle_oleaut_bstr_live()));
    memset(&v, 0, sizeof v); v.vt = 0x1000;
    hr = (uint32_t)CALL(fle_imp_SafeArrayPutElement, A(w), A(&i), A(&v)); printf("vput_bad %08x\n", hr);
    /* SafeArrayDestroy semantics through VariantClear: BSTR elements are freed, VARIANT elements are cleared */
    { V c; memset(&c, 0, sizeof c); c.vt = VT_ARRAY_ | VT_BSTR_; c.v.p = a;
      hr = (uint32_t)CALL(fle_imp_VariantClear, A(&c)); printf("destroy_bstr %08x live %llu arrays %llu\n", hr, U(fle_oleaut_bstr_live()), U(fle_oleaut_array_live())); }
    memset(&v, 0, sizeof v); v.vt = VT_BSTR_; v.v.u = bs("x");
    CALL(fle_imp_SafeArrayPutElement, A(w), A(&j), A(&v));
    CALL(fle_imp_VariantClear, A(&v));
    { V c; memset(&c, 0, sizeof c); c.vt = VT_ARRAY_ | VT_VARIANT_; c.v.p = w;
      hr = (uint32_t)CALL(fle_imp_VariantClear, A(&c)); printf("destroy_variant %08x live %llu arrays %llu\n", hr, U(fle_oleaut_bstr_live()), U(fle_oleaut_array_live())); }
    CALL(fle_imp_SysFreeString, s1); CALL(fle_imp_SysFreeString, s2); CALL(fle_imp_VariantClear, A(&out));
    printf("end %llu\n", U(fle_oleaut_bstr_live()));
"#,
    );
    assert_eq!(
        l,
        [
            "live0 2",
            format!("put0 {S_OK} live 3").as_str(),
            format!("put0_again {S_OK} live 3 is 1").as_str(),
            format!("put_null_bstr {S_OK} elem 0").as_str(),
            // s1, s2, the array's copy of "second", and the copy GetElement returned (freed right after)
            format!("get0 {S_OK} distinct 1 is 1 live 4").as_str(),
            format!("get_null {S_OK} val 0").as_str(),
            // s1, s2, a[0], the temporary "in-variant" and the copy inside w[0]
            format!("vput {S_OK} vt 8 live 5").as_str(),
            format!("vget {S_OK} vt 8 distinct 1 is 1").as_str(),
            "vempty 0",
            // s1, s2, a[0] and out (w[0] no longer owns a string)
            format!("vput_over {S_OK} vt 3 live 4").as_str(),
            format!("vput_bad {DISP_E_BADVARTYPE}").as_str(),
            // a[0] is freed with the array
            format!("destroy_bstr {S_OK} live 3 arrays 1").as_str(),
            // w[1]'s string is freed with the array; s1, s2 and out remain
            format!("destroy_variant {S_OK} live 3 arrays 0").as_str(),
            "end 0",
        ]
    );
}

#[test]
fn safe_array_inputs_that_need_a_callback_or_are_not_arrays_abort() {
    assert_aborts(
        "ole-sa-abort",
        "SA *a = mk(VT_I4_, 2, 0), *u = mk(VT_UNKNOWN_, 2, 0), *d = mk(VT_DISPATCH_, 1, 0); int32_t i = 0, v = 1; uint64_t o = 0; SA fake; memset(&fake, 0, sizeof fake);",
        &[
            ("put-unknown-object", "CALL(fle_imp_SafeArrayPutElement, A(u), A(&i), 0x1234);"),
            ("put-dispatch-object", "CALL(fle_imp_SafeArrayPutElement, A(d), A(&i), 0x1234);"),
            ("put-over-unknown-object", "((uint64_t *)u->pvData)[0] = 0x1234; CALL(fle_imp_SafeArrayPutElement, A(u), A(&i), 0);"),
            ("get-unknown-object", "((uint64_t *)u->pvData)[0] = 0x1234; CALL(fle_imp_SafeArrayGetElement, A(u), A(&i), A(&o));"),
            ("getlbound-guest-built-descriptor", "int32_t lo; CALL(fle_imp_SafeArrayGetLBound, A(&fake), 1, A(&lo));"),
            ("getelement-guest-built-descriptor", "CALL(fle_imp_SafeArrayGetElement, A(&fake), A(&i), A(&v));"),
            ("access-guest-built-descriptor", "void *p; CALL(fle_imp_SafeArrayAccessData, A(&fake), A(&p));"),
            ("elemsize-null", "CALL(fle_imp_SafeArrayGetElemsize, 0);"),
            ("elemsize-guest-built-descriptor", "CALL(fle_imp_SafeArrayGetElemsize, A(&fake));"),
            ("variant-clear-freed-array", "V c; memset(&c, 0, sizeof c); c.vt = VT_ARRAY_ | VT_I4_; c.v.p = a; CALL(fle_imp_VariantClear, A(&c)); CALL(fle_imp_VariantClear, A(&c)); c.vt = VT_ARRAY_ | VT_I4_; CALL(fle_imp_VariantClear, A(&c));"),
        ],
    );
}

#[test]
fn safe_array_elemsize_values() {
    let l = lines(
        "ole-sa-elemsize",
        r#"
    SA *a = mk(VT_VARIANT_, 1, 0), *b = mk(VT_UI1_, 1, 0);
    printf("%llu %llu\n", U(CALL(fle_imp_SafeArrayGetElemsize, A(a))), U(CALL(fle_imp_SafeArrayGetElemsize, A(b))));
"#,
    );
    assert_eq!(l, ["24 1"]);
}

// ---- 5. GetErrorInfo ----

#[test]
fn get_error_info() {
    let l = lines(
        "ole-errorinfo",
        r#"
    uint64_t out = 0x1234; uint32_t hr;
    /* page: S_FALSE when there is no error object to return; the environment has no SetErrorInfo caller */
    hr = (uint32_t)CALL(fle_imp_GetErrorInfo, 0, A(&out));
    printf("%08x out %llx\n", hr, U(out));
"#,
    );
    assert_eq!(l, [format!("{S_FALSE} out 0")]);
}

#[test]
fn get_error_info_inputs_the_page_leaves_open_abort() {
    assert_aborts(
        "ole-errorinfo-abort",
        "uint64_t out = 0;",
        &[("reserved-nonzero", "CALL(fle_imp_GetErrorInfo, 1, A(&out));"), ("null-out", "CALL(fle_imp_GetErrorInfo, 0, 0);")],
    );
}

// ---- 6. COM initialization ----

#[test]
fn co_initialize_s_ok_then_s_false_and_balance() {
    let l = lines(
        "ole-coinit",
        r#"
    uint32_t hr;
    hr = (uint32_t)CALL(fle_imp_CoInitialize, 0); printf("co1 %08x depth %u\n", hr, fle_ole_com_depth());
    /* page: a later call on the same thread succeeds with S_FALSE, and must be balanced as well */
    hr = (uint32_t)CALL(fle_imp_CoInitialize, 0); printf("co2 %08x depth %u\n", hr, fle_ole_com_depth());
    hr = (uint32_t)CALL(fle_imp_OleInitialize, 0); printf("ole3 %08x depth %u ole %u\n", hr, fle_ole_com_depth(), fle_ole_ole_depth());
    hr = (uint32_t)CALL(fle_imp_OleInitialize, 0); printf("ole4 %08x depth %u ole %u\n", hr, fle_ole_com_depth(), fle_ole_ole_depth());
    CALL(fle_imp_OleUninitialize); printf("un_ole depth %u ole %u\n", fle_ole_com_depth(), fle_ole_ole_depth());
    CALL(fle_imp_OleUninitialize); printf("un_ole depth %u ole %u\n", fle_ole_com_depth(), fle_ole_ole_depth());
    CALL(fle_imp_CoUninitialize); printf("un_co depth %u\n", fle_ole_com_depth());
    CALL(fle_imp_CoUninitialize); printf("un_co depth %u\n", fle_ole_com_depth());
    /* fully uninitialized: the next call initializes again */
    hr = (uint32_t)CALL(fle_imp_OleInitialize, 0); printf("again %08x depth %u\n", hr, fle_ole_com_depth());
    CALL(fle_imp_OleUninitialize); printf("end %u\n", fle_ole_com_depth());
"#,
    );
    assert_eq!(
        l,
        [
            format!("co1 {S_OK} depth 1").as_str(),
            format!("co2 {S_FALSE} depth 2").as_str(),
            format!("ole3 {S_FALSE} depth 3 ole 1").as_str(),
            format!("ole4 {S_FALSE} depth 4 ole 2").as_str(),
            "un_ole depth 3 ole 1",
            "un_ole depth 2 ole 0",
            "un_co depth 1",
            "un_co depth 0",
            format!("again {S_OK} depth 1").as_str(),
            "end 0",
        ]
    );
}

#[test]
fn ole_initialize_first_is_s_ok() {
    let l = lines(
        "ole-oleinit-first",
        r#"
    uint32_t hr = (uint32_t)CALL(fle_imp_OleInitialize, 0);
    printf("%08x\n", hr);
    hr = (uint32_t)CALL(fle_imp_CoInitialize, 0);
    printf("%08x\n", hr);
"#,
    );
    // OleInitialize page: S_OK on success; a later CoInitialize on the same STA apartment is S_FALSE
    assert_eq!(l, [S_OK, S_FALSE]);
}

#[test]
fn com_initialization_inputs_the_pages_leave_open_abort() {
    assert_aborts(
        "ole-coinit-abort",
        "",
        &[
            ("coinit-reserved", "CALL(fle_imp_CoInitialize, 1);"),
            ("oleinit-reserved", "CALL(fle_imp_OleInitialize, 0x10);"),
            ("couninit-unbalanced", "CALL(fle_imp_CoUninitialize);"),
            ("oleuninit-unbalanced", "CALL(fle_imp_OleUninitialize);"),
            ("oleuninit-after-coinit", "CALL(fle_imp_CoInitialize, 0); CALL(fle_imp_OleUninitialize);"),
            ("couninit-takes-ole-count", "CALL(fle_imp_OleInitialize, 0); CALL(fle_imp_CoUninitialize);"),
            ("couninit-twice", "CALL(fle_imp_CoInitialize, 0); CALL(fle_imp_CoUninitialize); CALL(fle_imp_CoUninitialize);"),
        ],
    );
}

// ---- 7. CoTaskMem ----

#[test]
fn co_task_mem_alloc_and_free() {
    let l = lines(
        "ole-cotask",
        r#"
    uint64_t p = CALL(fle_imp_CoTaskMemAlloc, 100), z = CALL(fle_imp_CoTaskMemAlloc, 0), q, big;
    printf("a %d aligned16 %d zero_len %d live %llu\n", p != 0, (p & 15) == 0, z != 0, U(fle_ole_task_live()));
    /* the block is usable for its whole size; contents undefined on the page, zero here (setting) */
    { uint8_t *b = (uint8_t *)p; int i, zero = 1; for (i = 0; i < 100; i++) if (b[i]) zero = 0; memset(b, 0x5A, 100); printf("zero %d\n", zero); }
    q = CALL(fle_imp_CoTaskMemAlloc, 100); printf("distinct %d\n", p != q && q != z && p != z);
    CALL(fle_imp_CoTaskMemFree, 0);  /* page: NULL has no effect */
    printf("after_null %llu\n", U(fle_ole_task_live()));
    CALL(fle_imp_CoTaskMemFree, p); CALL(fle_imp_CoTaskMemFree, z); CALL(fle_imp_CoTaskMemFree, q);
    printf("after_free %llu\n", U(fle_ole_task_live()));
    /* page: NULL when memory is insufficient */
    big = CALL(fle_imp_CoTaskMemAlloc, 0xFFFFFFFFFFFFFF00ull); printf("big %llu live %llu\n", U(big), U(fle_ole_task_live()));
"#,
    );
    assert_eq!(l, ["a 1 aligned16 1 zero_len 1 live 2", "zero 1", "distinct 1", "after_null 3", "after_free 0", "big 0 live 0"]);
}

#[test]
fn co_task_mem_free_of_other_pointers_aborts() {
    assert_aborts(
        "ole-cotask-abort",
        "uint64_t p = CALL(fle_imp_CoTaskMemAlloc, 8); static char local[8]; uint64_t s = bs(\"x\");",
        &[
            ("twice", "CALL(fle_imp_CoTaskMemFree, p); CALL(fle_imp_CoTaskMemFree, p);"),
            ("stack-pointer", "CALL(fle_imp_CoTaskMemFree, A(local));"),
            ("stray", "CALL(fle_imp_CoTaskMemFree, 0x1234);"),
            ("bstr", "CALL(fle_imp_CoTaskMemFree, s);"),
        ],
    );
}

// ---- 8. GUID strings ----

// {6B29FC40-CA47-1067-B31D-00DD010662DA} is the example GUID printed on the GUID page.
const GUID_C: &str = r#"
    static const uint8_t g[16] = { 0x40, 0xFC, 0x29, 0x6B, 0x47, 0xCA, 0x67, 0x10, 0xB3, 0x1D, 0x00, 0xDD, 0x01, 0x06, 0x62, 0xDA };
"#;

#[test]
fn string_from_clsid() {
    let body = format!(
        r#"{GUID_C}
    uint64_t out = 0; uint32_t hr; int i; char s[64]; static const uint8_t nul[16] = {{0}};
    hr = (uint32_t)CALL(fle_imp_StringFromCLSID, A(g), A(&out));
    for (i = 0; i < 40; i++) {{ uint16_t c = (uint16_t)fle_ld(out + 2 * i, 2); s[i] = (char)c; if (!c) break; }}
    printf("%08x %s len %d live %llu\n", hr, s, i, U(fle_ole_task_live()));
    /* the string is a CoTaskMemAlloc block: CoTaskMemFree releases it (page) */
    CALL(fle_imp_CoTaskMemFree, out); printf("freed %llu\n", U(fle_ole_task_live()));
    hr = (uint32_t)CALL(fle_imp_StringFromCLSID, A(nul), A(&out));
    for (i = 0; i < 40; i++) {{ uint16_t c = (uint16_t)fle_ld(out + 2 * i, 2); s[i] = (char)c; if (!c) break; }}
    printf("%08x %s\n", hr, s);
    CALL(fle_imp_CoTaskMemFree, out);
"#
    );
    let l = lines("ole-strfromclsid", &body);
    // the page: enclosing braces; 8-4-4-4-12 groups (GUID page)
    assert_eq!(
        l,
        [
            format!("{S_OK} {{6B29FC40-CA47-1067-B31D-00DD010662DA}} len 38 live 1").as_str(),
            "freed 0",
            format!("{S_OK} {{00000000-0000-0000-0000-000000000000}}").as_str(),
        ]
    );
}

#[test]
fn clsid_from_string() {
    let body = format!(
        r#"{GUID_C}
    uint8_t out[16]; uint32_t hr; int ok;
    static const uint16_t good[] = {{ '{{','6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','A','}}', 0 }};
    static const uint16_t lower[] = {{ '{{','6','b','2','9','f','c','4','0','-','c','a','4','7','-','1','0','6','7','-','b','3','1','d','-','0','0','d','d','0','1','0','6','6','2','d','a','}}', 0 }};
    static const uint16_t nobrace[] = {{ '6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','A', 0 }};
    static const uint16_t nodash[] = {{ '{{','6','B','2','9','F','C','4','0','x','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','A','}}', 0 }};
    static const uint16_t nothex[] = {{ '{{','6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','G','}}', 0 }};
    static const uint16_t shortc[] = {{ '{{','6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','}}', 0 }};
    static const uint16_t longc[] = {{ '{{','6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','A','A','}}', 0 }};
    static const uint16_t open_only[] = {{ '{{', 0 }};
    memset(out, 0xEE, 16);
    hr = (uint32_t)CALL(fle_imp_CLSIDFromString, A(good), A(out)); ok = memcmp(out, g, 16) == 0; printf("good %08x same %d\n", hr, ok);
    memset(out, 0xEE, 16);
    hr = (uint32_t)CALL(fle_imp_CLSIDFromString, A(lower), A(out)); ok = memcmp(out, g, 16) == 0; printf("lower %08x same %d\n", hr, ok);
    /* page: NULL results in GUID_NULL */
    memset(out, 0xEE, 16);
    hr = (uint32_t)CALL(fle_imp_CLSIDFromString, 0, A(out)); {{ static const uint8_t z[16] = {{0}}; printf("null %08x guid_null %d\n", hr, memcmp(out, z, 16) == 0); }}
    /* page: CO_E_CLASSSTRING when improperly formatted; the output is not written (setting) */
    {{ const uint16_t *bad[] = {{ nodash, nothex, shortc, longc, open_only }}; const char *nm[] = {{ "nodash", "nothex", "short", "long", "open_only" }}; int i;
      for (i = 0; i < 5; i++) {{ memset(out, 0xEE, 16); hr = (uint32_t)CALL(fle_imp_CLSIDFromString, A(bad[i]), A(out)); printf("%s %08x untouched %d\n", nm[i], hr, out[0] == 0xEE && out[15] == 0xEE); }} }}
    hr = (uint32_t)CALL(fle_imp_CLSIDFromString, A(good), 0); printf("null_out %08x\n", hr);
    /* a round trip through StringFromCLSID */
    {{ uint64_t s = 0; uint8_t back[16]; CALL(fle_imp_StringFromCLSID, A(g), A(&s)); hr = (uint32_t)CALL(fle_imp_CLSIDFromString, s, A(back)); printf("roundtrip %08x same %d\n", hr, memcmp(back, g, 16) == 0); CALL(fle_imp_CoTaskMemFree, s); }}
    (void)nobrace;
"#
    );
    let l = lines("ole-clsidfromstr", &body);
    assert_eq!(
        l,
        [
            format!("good {S_OK} same 1").as_str(),
            format!("lower {S_OK} same 1").as_str(),
            format!("null {S_OK} guid_null 1").as_str(),
            format!("nodash {CO_E_CLASSSTRING} untouched 1").as_str(),
            format!("nothex {CO_E_CLASSSTRING} untouched 1").as_str(),
            format!("short {CO_E_CLASSSTRING} untouched 1").as_str(),
            format!("long {CO_E_CLASSSTRING} untouched 1").as_str(),
            format!("open_only {CO_E_CLASSSTRING} untouched 1").as_str(),
            format!("null_out {E_INVALIDARG}").as_str(),
            format!("roundtrip {S_OK} same 1").as_str(),
        ]
    );
}

#[test]
fn clsid_from_string_progid_aborts() {
    assert_aborts(
        "ole-clsidfromstr-progid",
        "uint8_t out[16]; static const uint16_t progid[] = { 'W','o','r','d','.','D','o','c','u','m','e','n','t','.','6', 0 };",
        &[("progid", "CALL(fle_imp_CLSIDFromString, A(progid), A(out));")],
    );
}

#[test]
fn is_equal_guid() {
    let body = format!(
        r#"{GUID_C}
    uint8_t same[16], d0[16], d15[16], d8[16];
    memcpy(same, g, 16); memcpy(d0, g, 16); memcpy(d15, g, 16); memcpy(d8, g, 16); d0[0] ^= 1; d15[15] ^= 0x80; d8[8] ^= 4;
    printf("same_obj %llu copy %llu d0 %llu d15 %llu d8 %llu\n", U(CALL(fle_imp_IsEqualGUID, A(g), A(g))), U(CALL(fle_imp_IsEqualGUID, A(g), A(same))),
           U(CALL(fle_imp_IsEqualGUID, A(g), A(d0))), U(CALL(fle_imp_IsEqualGUID, A(g), A(d15))), U(CALL(fle_imp_IsEqualGUID, A(g), A(d8))));
    /* the result is a BOOL in eax: only 0 and 1 */
    printf("full_rax %llx\n", U(CALL(fle_imp_IsEqualGUID, A(g), A(same))));
"#
    );
    let l = lines("ole-isequalguid", &body);
    assert_eq!(l, ["same_obj 1 copy 1 d0 0 d15 0 d8 0", "full_rax 1"]);
}

#[test]
fn guid_functions_null_arguments_abort() {
    assert_aborts(
        "ole-guid-abort",
        "static const uint8_t g[16] = {1}; uint64_t o = 0;",
        &[
            ("isequal-null1", "CALL(fle_imp_IsEqualGUID, 0, A(g));"),
            ("isequal-null2", "CALL(fle_imp_IsEqualGUID, A(g), 0);"),
            ("strfromclsid-null-guid", "CALL(fle_imp_StringFromCLSID, 0, A(&o));"),
            ("strfromclsid-null-out", "CALL(fle_imp_StringFromCLSID, A(g), 0);"),
            ("progid-null-guid", "CALL(fle_imp_ProgIDFromCLSID, 0, A(&o));"),
            ("progid-null-out", "CALL(fle_imp_ProgIDFromCLSID, A(g), 0);"),
        ],
    );
}

// ---- 9. ProgIDFromCLSID ----

#[test]
fn prog_id_from_clsid_on_the_empty_virtual_registry() {
    let body = format!(
        r#"{GUID_C}
    uint64_t out = 0x1234; uint32_t hr;
    /* page: REGDB_E_CLASSNOTREG when the class is not registered; the virtual registry starts empty */
    hr = (uint32_t)CALL(fle_imp_ProgIDFromCLSID, A(g), A(&out)); printf("%08x out %llx\n", hr, U(out));
    /* the query leaves the registry as it was: a key the guest creates afterwards is the only content */
    {{ uint64_t h = 0, disp = 0; static const uint16_t k[] = {{ 'C','L','S','I','D', 0 }};
       hr = (uint32_t)CALL(fle_imp_RegCreateKeyExW, 0x80000000ull, A(k), 0, 0, 0, 0xF003F, 0, A(&h), A(&disp)); printf("create %08x disp %llu\n", hr, U(disp)); }}
    hr = (uint32_t)CALL(fle_imp_ProgIDFromCLSID, A(g), A(&out)); printf("%08x\n", hr);
"#
    );
    let l = lines("ole-progid", &body);
    assert_eq!(l, [format!("{REGDB_E_CLASSNOTREG} out 1234"), format!("create {S_OK} disp 1"), REGDB_E_CLASSNOTREG.to_string()]);
}

#[test]
fn prog_id_from_clsid_with_a_registered_class_aborts() {
    let body = format!(
        r#"{GUID_C}
    uint64_t out = 0, h = 0, h2 = 0; static const uint16_t k[] = {{ 'C','L','S','I','D', 0 }};
    static const uint16_t k2[] = {{ '{{','6','B','2','9','F','C','4','0','-','C','A','4','7','-','1','0','6','7','-','B','3','1','D','-','0','0','D','D','0','1','0','6','6','2','D','A','}}', 0 }};
    CALL(fle_imp_RegCreateKeyExW, 0x80000000ull, A(k), 0, 0, 0, 0xF003F, 0, A(&h), 0);
    CALL(fle_imp_RegCreateKeyExW, h, A(k2), 0, 0, 0, 0xF003F, 0, A(&h2), 0);
    CALL(fle_imp_ProgIDFromCLSID, A(g), A(&out));
    printf("returned\n");
"#
    );
    let o = run("ole-progid-registered", &body);
    assert!(!o.ok && o.signal == Some(6) && o.stderr.contains("fle:") && !o.stdout.contains("returned"), "{} {}", o.stdout, o.stderr);
}

// ---- 10. Source hygiene ----

#[test]
fn sources_use_no_host_process_time_environment_or_file_state() {
    for f in ["fle_oleaut.c", "fle_ole.c"] {
        let code = strip_comments(&std::fs::read_to_string(src_dir().join(f)).unwrap());
        for bad in [
            "getpid", "getppid", "clock_gettime", "gettimeofday", "getenv", "environ", "fopen", "open(", "<time.h>", "mmap", "sysinfo", "/proc",
            "/etc", "/home", "exit(", "_exit", "CoCreateGuid", "uuid", "rand(",
        ] {
            assert!(!code.contains(bad), "{f} must not use {bad}");
        }
    }
}

/// C text without comments (strings are kept).
fn strip_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut o = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}
