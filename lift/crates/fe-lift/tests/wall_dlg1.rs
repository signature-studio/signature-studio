//! Tests of decision 0178 (docs/0178-filedialog-and-flp-open.md, PROVISIONAL): the common item dialog COM model (runtime/fle_filedlg.c) behind ole32!CoCreateInstance(CLSID_FileOpenDialog).
//!
//! The expected values are written here from the Microsoft Learn pages named in fle_filedlg.c (IModalWindow::Show, IFileDialog::*, IFileOpenDialog::GetResults, IFileDialogEvents::OnFileOk, IShellItem::*,
//! IShellItemArray::GetItemAt, SHCreateItemFromParsingName, SHGetKnownFolderPath, FILEOPENDIALOGOPTIONS, SIGDN, HRESULT tables), from the vtable ORDER of the SDK interface definition (shobjidl.idl, which the
//! Learn pages do not give) and from the GUIDs of the SDK headers - not read back from the C code. The choices the pages leave open are the S-dlg1-N of the decision.
//!
//! The C programs call the `fle_imp_*` functions the way the lifted code reaches them: arguments in R[1], R[2], R[8], R[9], the result in R[0]; a vtable slot is a response-table VA (the test builds the
//! response table from the row names of harness/seeds/dlg1-imports.txt, in the order of that file). The guest-side calls the model makes (`fle_call_guest`: AddRef/Release of the event handler, OnFileOk) are
//! recorded. The virtual volume (`fle_imp_GetFileAttributesW`) and the task allocator are stubs of the program.
//!
//! EVERY ANSWER IS PROVISIONAL. Nothing is compared with a real Windows. FLE_DLG1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_DLG1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-dlg1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const DIALOG_METHODS: [&str; 26] = [
    "Show", "SetFileTypes", "SetFileTypeIndex", "GetFileTypeIndex", "Advise", "Unadvise", "SetOptions", "GetOptions", "SetDefaultFolder", "SetFolder", "GetFolder", "GetCurrentSelection", "SetFileName",
    "GetFileName", "SetTitle", "SetOkButtonLabel", "SetFileNameLabel", "GetResult", "AddPlace", "SetDefaultExtension", "Close", "SetClientGuid", "ClearClientData", "SetFilter", "GetResults", "GetSelectedItems",
];
const ITEM_METHODS: [&str; 5] = ["BindToHandler", "GetParent", "GetDisplayName", "GetAttributes", "Compare"];
const ARRAY_METHODS: [&str; 7] = ["BindToHandler", "GetPropertyStore", "GetPropertyDescriptionList", "GetAttributes", "GetCount", "GetItemAt", "EnumItems"];
const CUSTOM_METHODS: [&str; 27] = [
    "EnableOpenDropDown", "AddMenu", "AddPushButton", "AddComboBox", "AddRadioButtonList", "AddCheckButton", "AddEditBox", "AddSeparator", "AddText", "SetControlLabel", "GetControlState", "SetControlState",
    "GetEditBoxText", "SetEditBoxText", "GetCheckButtonState", "SetCheckButtonState", "AddControlItem", "RemoveControlItem", "RemoveAllControlItems", "GetControlItemState", "SetControlItemState",
    "GetSelectedControlItem", "SetSelectedControlItem", "StartVisualGroup", "EndVisualGroup", "MakeProminent", "SetControlItemText",
];

/// The names of the vtable rows in the order of the rows file.
fn row_names() -> Vec<String> {
    let mut v: Vec<String> = ["QueryInterface", "AddRef", "Release"].iter().map(|m| format!("ShellObj::{m}")).collect();
    v.extend(DIALOG_METHODS.iter().map(|m| format!("IFileOpenDialog::{m}")));
    v.extend(ITEM_METHODS.iter().map(|m| format!("IShellItem::{m}")));
    v.extend(ARRAY_METHODS.iter().map(|m| format!("IShellItemArray::{m}")));
    v.extend(CUSTOM_METHODS.iter().map(|m| format!("IFileDialogCustomize::{m}")));
    v
}

fn pre(with_rows: bool) -> String {
    let mut s = String::from(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_com.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_filedlg.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
void fle_stop(const char *api, const char *why) { printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }
/* wide-string helpers of fle_env.c (the program does not link fle_env.c) */
size_t fle_w_len(const uint16_t *s) { size_t n = 0; while (s[n]) n++; return n; }
uint16_t fle_w_fold(uint16_t c) { return (c >= 'A' && c <= 'Z') ? (uint16_t)(c + 32) : c; }
int fle_w_icmp(const uint16_t *a, size_t an, const uint16_t *b, size_t bn) { size_t i; for (i = 0; i < an && i < bn; i++) { uint16_t x = fle_w_fold(a[i]), y = fle_w_fold(b[i]); if (x != y) return x < y ? -1 : 1; } return an == bn ? 0 : an < bn ? -1 : 1; }
uint16_t *fle_w_read_alloc(const char *api, uint64_t g, size_t max, size_t *len) { const uint16_t *s = (const uint16_t *)(uintptr_t)g; size_t n = fle_w_len(s); (void)api; if (n > max) fle_stop(api, "too long"); *len = n; uint16_t *d = malloc((n + 1) * 2); memcpy(d, s, (n + 1) * 2); return d; }
void fle_w_write(uint64_t dst, const uint16_t *src, size_t units) { memcpy((void *)(uintptr_t)dst, src, units * 2); }
uint16_t *fle_w_from_ascii(const char *s) { size_t n = strlen(s), i; uint16_t *d = malloc((n + 1) * 2); for (i = 0; i <= n; i++) d[i] = (unsigned char)s[i]; return d; }
/* the task allocator (the program frees nothing it gets) */
static unsigned task_allocs;
void fle_imp_CoTaskMemAlloc(void) { task_allocs++; R[0] = (uint64_t)(uintptr_t)calloc(1, R[1]); }
/* the virtual volume: a list of (path, attributes) */
static struct { const char *path; uint32_t attr; } vol[16];
static unsigned nvol;
static void vol_add(const char *p, uint32_t a) { vol[nvol].path = p; vol[nvol].attr = a; nvol++; }
void fle_imp_GetFileAttributesW(void) {
    const uint16_t *w = (const uint16_t *)(uintptr_t)R[1]; unsigned i;
    for (i = 0; i < nvol; i++) { size_t n = strlen(vol[i].path), k; int eq = (fle_w_len(w) == n); for (k = 0; eq && k < n; k++) if (fle_w_fold(w[k]) != fle_w_fold((unsigned char)vol[i].path[k])) eq = 0; if (eq) { R[0] = vol[i].attr; return; } }
    R[0] = 0xFFFFFFFFu;
}
/* the guest calls the model makes: recorded; OnFileOk (slot 3 of the handler's table) answers `onfileok_hr`, AddRef/Release answer 1 */
static uint64_t gcalls[64][3];
static unsigned ngcalls;
static uint32_t onfileok_hr;
uint64_t fle_call_guest(uint64_t va, uint64_t a, uint64_t b, uint64_t c, uint64_t d) { (void)c; (void)d; if (ngcalls < 64) { gcalls[ngcalls][0] = va; gcalls[ngcalls][1] = a; gcalls[ngcalls][2] = b; } ngcalls++; return va == 0x3000 ? onfileok_hr : 1; }
/* a guest event handler: object { vtable }, vtable { QI, AddRef (0x1000), Release (0x2000), OnFileOk (0x3000) ... } */
static uint64_t handler_vt[10] = { 0x500, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000, 0x9000 };
static uint64_t handler_obj[1] = { 0 };
static void nop(void) {}
static uint16_t *W(const char *s) { return fle_w_from_ascii(s); }
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static uint64_t call(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { R[0] = 0xdead; R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
static const guid_dummy_t_unused;
"#,
    );
    s = s.replace("static const guid_dummy_t_unused;\n", "");
    // GUIDs written from the SDK headers (shobjidl.h / knownfolders.h), as 16 bytes in memory order
    s.push_str(
        r#"
void fle_imp_CoCreateInstance(void);
#define GUID(d1, d2, d3, a, b, c, d, e, f, g, h) { (d1) & 0xff, ((d1) >> 8) & 0xff, ((d1) >> 16) & 0xff, ((d1) >> 24) & 0xff, (d2) & 0xff, (d2) >> 8, (d3) & 0xff, (d3) >> 8, a, b, c, d, e, f, g, h }
static const uint8_t CLSID_FOD[16] = GUID(0xdc1c5a9cu, 0xe88a, 0x4dde, 0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7);
static const uint8_t CLSID_FSD[16] = GUID(0xc0b4e2f3u, 0xba21, 0x4773, 0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b);
static const uint8_t IID_Unknown[16] = GUID(0x00000000u, 0x0000, 0x0000, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46);
static const uint8_t IID_FOD[16] = GUID(0xd57c7288u, 0xd4ad, 0x4768, 0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60);
static const uint8_t IID_FD[16] = GUID(0x42f85136u, 0xdb7e, 0x439c, 0x85, 0xf1, 0xe4, 0x07, 0x5d, 0x13, 0x5f, 0xc8);
static const uint8_t IID_Modal[16] = GUID(0xb4db1657u, 0x70d7, 0x485e, 0x8e, 0x3e, 0x6f, 0xcb, 0x5a, 0x5c, 0x18, 0x02);
static const uint8_t IID_Item[16] = GUID(0x43826d1eu, 0xe718, 0x42ee, 0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe);
static const uint8_t IID_Array[16] = GUID(0xb63ea76du, 0x1f85, 0x456f, 0xa1, 0x9c, 0x48, 0x15, 0x9e, 0xfa, 0x85, 0x8b);
static const uint8_t IID_Customize[16] = GUID(0xe6fdd21au, 0x163f, 0x4975, 0x9c, 0x8c, 0xa6, 0x9f, 0x1b, 0xa3, 0x70, 0x34);
static const uint8_t FOLDERID_Docs[16] = GUID(0xfdd39ad0u, 0x238f, 0x46af, 0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7);
static const uint8_t FOLDERID_Bogus[16] = GUID(0x12345678u, 0x1234, 0x1234, 1, 2, 3, 4, 5, 6, 7, 8);
/* methods through the vtable of an object: the VA in the slot -> the row name */
static const char *va_name(uint64_t va) { uint64_t i; for (i = 0; i < fle_resp_count; i++) if (fle_resp_table[i].va == va) return fle_resp_table[i].name; return "?"; }
static uint64_t slot(uint64_t obj, unsigned k) { return fle_ld(fle_ld(obj, 8) + 8u * k, 8); }
/* call a method through the vtable the way the lifted code does: the VA is looked up in the response table and the function is called */
static uint64_t vcall(uint64_t obj, unsigned k, uint64_t b, uint64_t c, uint64_t d) {
    uint64_t va = slot(obj, k), i;
    for (i = 0; i < fle_resp_count; i++) if (fle_resp_table[i].va == va) { void (*f)(void) = fle_resp_table[i].fn; R[0] = 0xdead; R[1] = obj; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
    printf("no row for the VA in slot %u\n", k); exit(4);
}
static uint64_t cocreate(const uint8_t *clsid, uint64_t outer, const uint8_t *iid, uint64_t *out) { uint64_t stk[8] = { 0 }; stk[5] = P(out); R[4] = P(stk); return call(fle_imp_CoCreateInstance, P(clsid), outer, 1, P(iid)); }
"#,
    );
    if with_rows {
        let names = row_names();
        s.push_str("const fle_resp_entry fle_resp_table[] = {\n");
        for (k, n) in names.iter().enumerate() {
            let f = format!("fle_imp_{}", n.replace("::", "_"));
            s.push_str(&format!("    {{ \"shell32.dll\", \"{n}\", UINT64_C({:#x}), {f} }},\n", 0x7F00_0000u64 + 16 * (k as u64 + 1)));
        }
        s.push_str(&format!("}};\nconst uint64_t fle_resp_count = {};\n", names.len()));
    } else {
        // a build of an earlier decision: a response table without the dialog rows
        s.push_str("const fle_resp_entry fle_resp_table[] = { { \"kernel32.dll\", \"Sleep\", UINT64_C(0x7F000010), nop } };\nconst uint64_t fle_resp_count = 1;\n");
    }
    s
}

/// Compiles `pre + main` with fle_filedlg.c (CoCreateInstance here is the hook call only), runs it, returns (exit code, stdout lines).
fn run(name: &str, with_rows: bool, main: &str) -> (i32, Vec<String>) {
    run_with_globals(name, with_rows, "", main)
}

const DLG: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;

#[test]
fn without_the_rows_the_class_stays_unregistered_as_before() {
    // a build whose response table holds no dialog rows (every build of an earlier decision): CoCreateInstance answers what decision 0129 made it answer
    let (rc, l) = run(
        "dlg-norows",
        false,
        r#"
    uint64_t d = 0xAAAA;
    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);
    printf("hr %#llx ppv %#llx\n", U(hr), U(d));
"#,
    );
    assert_eq!(rc, 0);
    // REGDB_E_CLASSNOTREG = 0x80040154 (COM Error Codes); *ppv is NULL on failure (CoCreateInstance page)
    assert_eq!(l, ["hr 0x80040154 ppv 0"]);
}

#[test]
fn the_vtable_slots_are_the_methods_of_the_sdk_order() {
    let mut main = String::from(DLG);
    main.push_str("    printf(\"dialog\");\n");
    // IFileOpenDialog: IUnknown 0-2, Show 3, IFileDialog 4-26, GetResults 27, GetSelectedItems 28 (shobjidl.idl)
    let mut expect = vec!["ShellObj::QueryInterface".to_string(), "ShellObj::AddRef".to_string(), "ShellObj::Release".to_string()];
    expect.extend(DIALOG_METHODS.iter().map(|m| format!("IFileOpenDialog::{m}")));
    assert_eq!(expect.len(), 29);
    main.push_str("    {\n        static const char *want[] = {");
    for e in &expect {
        main.push_str(&format!("\"{e}\", "));
    }
    main.push_str("};\n        unsigned k; int bad = 0;\n        for (k = 0; k < 29; k++) if (strcmp(va_name(slot(d, k)), want[k]) != 0) { printf(\"slot %u is %s, want %s\\n\", k, va_name(slot(d, k)), want[k]); bad++; }\n        printf(\" bad %d\\n\", bad);\n    }\n");
    // the order of IFileDialog's own methods, written independently of the table above: Show, SetFileTypes, SetFileTypeIndex, GetFileTypeIndex, Advise, Unadvise, SetOptions, GetOptions, SetDefaultFolder, SetFolder,
    // GetFolder, GetCurrentSelection, SetFileName, GetFileName, SetTitle, SetOkButtonLabel, SetFileNameLabel, GetResult, AddPlace, SetDefaultExtension, Close, SetClientGuid, ClearClientData, SetFilter
    main.push_str("    printf(\"GetResult at %u, SetFilter at %u, GetResults at %u\\n\", 3u + 17u, 3u + 23u, 3u + 24u);\n");
    let (rc, l) = run("dlg-order", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["dialog bad 0", "GetResult at 20, SetFilter at 26, GetResults at 27"]);
}

#[test]
fn cocreate_checks_class_aggregation_and_interface() {
    let (rc, l) = run(
        "dlg-cocreate",
        true,
        r#"
    uint64_t d = 0xAAAA, outer = 0x1234;
    printf("fod %#llx ppv-set %d\n", U(cocreate(CLSID_FOD, 0, IID_FOD, &d)), d != 0 && d != 0xAAAA);
    printf("live %u\n", fle_filedlg_live_objects());
    d = 0xAAAA;
    printf("fd %#llx\n", U(cocreate(CLSID_FOD, 0, IID_FD, &d)));
    d = 0xAAAA;
    printf("unknown %#llx\n", U(cocreate(CLSID_FOD, 0, IID_Unknown, &d)));
    d = 0xAAAA;
    printf("aggregated %#llx ppv %#llx\n", U(cocreate(CLSID_FOD, outer, IID_FOD, &d)), U(d));
    d = 0xAAAA;
    printf("save-dialog %#llx ppv %#llx\n", U(cocreate(CLSID_FSD, 0, IID_FOD, &d)), U(d));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    // CLASS_E_NOAGGREGATION = 0x80040110 (COM Error Codes). CLSID_FileSaveDialog is not a class of the model: the answer stays REGDB_E_CLASSNOTREG 0x80040154.
    assert_eq!(l, ["fod 0 ppv-set 1", "live 1", "fd 0", "unknown 0", "aggregated 0x80040110 ppv 0", "save-dialog 0x80040154 ppv 0"]);
}

#[test]
fn another_interface_of_the_class_stops_by_name_instead_of_e_nointerface() {
    let (rc, l) = run("dlg-cocreate-iid", true, "    uint64_t d = 0;\n    cocreate(CLSID_FOD, 0, IID_Customize, &d);\n    printf(\"not reached\\n\");\n");
    assert_eq!(rc, 3);
    assert!(l[0].starts_with("STOP CoCreateInstance: CLSID_FileOpenDialog asked for an interface other than"), "{l:?}");
}

#[test]
fn query_interface_and_reference_counting() {
    let mut main = String::from(DLG);
    main.push_str(
        r#"
    uint64_t q = 0;
    printf("qi-fod %#llx same %d\n", U(vcall(d, 0, P(IID_FOD), P(&q), 0)), q == d);
    printf("qi-modal %#llx same %d\n", U(vcall(d, 0, P(IID_Modal), P(&q), 0)), q == d);
    printf("qi-unknown %#llx\n", U(vcall(d, 0, P(IID_Unknown), P(&q), 0)));
    printf("addref %llu\n", U(vcall(d, 1, 0, 0, 0)));
    printf("release %llu\n", U(vcall(d, 2, 0, 0, 0)));
    printf("release %llu\n", U(vcall(d, 2, 0, 0, 0)));
    printf("release %llu\n", U(vcall(d, 2, 0, 0, 0)));
    printf("release %llu\n", U(vcall(d, 2, 0, 0, 0)));
    printf("live %u\n", fle_filedlg_live_objects());
"#,
    );
    let (rc, l) = run("dlg-refs", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // 1 (create) + 3 QueryInterface successes = 4 references; AddRef returns the new count (5); then 4,3,2,1... : release returns the new count
    assert_eq!(l, ["qi-fod 0 same 1", "qi-modal 0 same 1", "qi-unknown 0", "addref 5", "release 4", "release 3", "release 2", "release 1", "live 1"]);
}

#[test]
fn the_last_release_frees_the_object() {
    let mut main = String::from(DLG);
    main.push_str("    printf(\"live %u\\n\", fle_filedlg_live_objects());\n    printf(\"last %llu live %u\\n\", U(vcall(d, 2, 0, 0, 0)), fle_filedlg_live_objects());\n");
    let (rc, l) = run("dlg-release", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // IUnknown::Release returns the new count: 0 after the only reference; the object is gone
    assert_eq!(l, ["live 1", "last 0 live 0"]);
}

#[test]
fn options_default_to_the_two_flags_the_page_calls_defaults_and_round_trip() {
    // GetOptions is slot 10 and SetOptions slot 9 of the SDK order (Show 3, SetFileTypes 4, SetFileTypeIndex 5, GetFileTypeIndex 6, Advise 7, Unadvise 8, SetOptions 9, GetOptions 10)
    let mut main = String::from(DLG);
    main.push_str(
        r#"
    uint32_t o = 0xFFFFFFFFu;
    printf("get %#llx value %#x\n", U(vcall(d, 10, P(&o), 0, 0)), o);
    printf("set %#llx\n", U(vcall(d, 9, 0x2020, 0, 0)));
    printf("get %#llx value %#x\n", U(vcall(d, 10, P(&o), 0, 0)), o);
"#,
    );
    let (rc, l) = run("dlg-options", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // FOS_PATHMUSTEXIST 0x800 and FOS_FILEMUSTEXIST 0x1000 are the defaults of the Open dialog (FILEOPENDIALOGOPTIONS page): 0x1800
    assert_eq!(l, ["get 0 value 0x1800", "set 0", "get 0 value 0x2020"]);
}

#[test]
fn set_file_types_follows_the_error_table_of_the_page() {
    let mut main = String::from(DLG);
    main.push_str(
        r#"
    uint16_t *n1 = W("FL Studio projects"), *s1 = W("*.flp"), *n2 = W("All files"), *s2 = W("*.*");
    uint64_t spec[4] = { P(n1), P(s1), P(n2), P(s2) };
    printf("null %#llx\n", U(vcall(d, 4, 2, 0, 0)));
    printf("ok %#llx\n", U(vcall(d, 4, 2, P(spec), 0)));
    printf("twice %#llx\n", U(vcall(d, 4, 2, P(spec), 0)));
    uint32_t idx = 0xFFFF;
    printf("index-default %#llx %u\n", U(vcall(d, 6, P(&idx), 0, 0)), idx);
    printf("set-index %#llx\n", U(vcall(d, 5, 2, 0, 0)));
    printf("index %#llx %u\n", U(vcall(d, 6, P(&idx), 0, 0)), idx);
    uint64_t d2 = 0;
    cocreate(CLSID_FOD, 0, IID_FOD, &d2);
    vcall(d2, 9, 0x20, 0, 0);
    printf("pickfolders %#llx\n", U(vcall(d2, 4, 2, P(spec), 0)));
"#,
    );
    let (rc, l) = run("dlg-types", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // E_INVALIDARG 0x80070057 (NULL array), E_UNEXPECTED 0x8000FFFF (called twice; FOS_PICKFOLDERS set); the index is one-based (SetFileTypeIndex / GetFileTypeIndex pages)
    assert_eq!(l, ["null 0x80070057", "ok 0", "twice 0x8000ffff", "index-default 0 1", "set-index 0", "index 0 2", "pickfolders 0x8000ffff"]);
}

#[test]
fn show_without_a_picker_or_selection_is_a_cancel_and_the_result_is_not_available() {
    let mut main = String::from(DLG);
    main.push_str(
        r#"
    uint64_t item = 0xAAAA, arr = 0xAAAA;
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("getresult %#llx item %#llx\n", U(vcall(d, 20, P(&item), 0, 0)), U(item));
    printf("getresults %#llx arr %#llx\n", U(vcall(d, 27, P(&arr), 0, 0)), U(arr));
"#,
    );
    let (rc, l) = run("dlg-cancel", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // HRESULT_FROM_WIN32(ERROR_CANCELLED = 1223) = 0x800704C7 (IModalWindow::Show page); GetResult / GetResults before a successful Show fail (their pages; E_UNEXPECTED is the model's choice S-dlg1-fail-code)
    assert_eq!(l, ["show 0x800704c7", "getresult 0x8000ffff item 0", "getresults 0x8000ffff arr 0"]);
}

const PICK: &str = r#"
static const char *pick_paths[4];
static unsigned pick_n;
static fle_filedlg_request last_rq;
static unsigned pick_calls;
static int picker(const fle_filedlg_request *rq, uint16_t ***paths, unsigned *n)
{
    unsigned i;
    pick_calls++;
    last_rq = *rq;
    if (pick_n == 0) return 0;
    *paths = calloc(pick_n, sizeof(uint16_t *));
    for (i = 0; i < pick_n; i++) (*paths)[i] = fle_w_from_ascii(pick_paths[i]);
    *n = pick_n;
    return 1;
}
"#;

#[test]
fn show_with_a_selection_gives_the_item_and_its_names() {
    // The picker stands for the host (the user's input); the model must hand the item over the way IFileDialog::GetResult and IShellItem::GetDisplayName describe.
    let main = format!(
        r#"{DLG}
    static const char *paths[1] = {{ "C:\\Users\\user\\Downloads\\serina1-serum2\\serina1-serum2.flp" }};
    vol_add("C:\\Users\\user\\Downloads\\serina1-serum2\\serina1-serum2.flp", 0x20);
    vol_add("C:\\Users\\user\\Downloads\\serina1-serum2", 0x10);
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_n = 1;
    uint64_t item = 0, name = 0;
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("sigdn-filesyspath %#llx\n", U(vcall(item, 5, 0x80058000u, P(&name), 0)));
    {{ uint16_t *w = (uint16_t *)(uintptr_t)name; char b[200]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
    printf("sigdn-normal %#llx\n", U(vcall(item, 5, 0, P(&name), 0)));
    {{ uint16_t *w = (uint16_t *)(uintptr_t)name; char b[200]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
    printf("sigdn-url %#llx\n", U(vcall(item, 5, 0x80068000u, P(&name), 0)));
    {{ uint16_t *w = (uint16_t *)(uintptr_t)name; char b[200]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
    uint32_t at = 0;
    printf("attrs-file %#llx %#x\n", U(vcall(item, 6, 0x40000000u | 0x00400000u, P(&at), 0)), at);
    printf("attrs-folder-ask %#llx %#x\n", U(vcall(item, 6, 0x20000000u, P(&at), 0)), at);
    uint64_t parent = 0;
    printf("parent %#llx\n", U(vcall(item, 4, P(&parent), 0, 0)));
    {{ uint32_t o = 77; printf("compare-self %#llx %u\n", U(vcall(item, 7, item, 0, P(&o))), o); printf("compare-parent %#llx %u\n", U(vcall(item, 7, parent, 0, P(&o))), o); }}
    printf("getfilename %#llx\n", U(vcall(d, 16, P(&name), 0, 0)));
    {{ uint16_t *w = (uint16_t *)(uintptr_t)name; char b[200]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
    printf("request filters %u options %#x\n", last_rq.nfilters, last_rq.options);
"#
    );
    let (rc, l) = run_with_globals("dlg-show", true, PICK, &main);
    assert_eq!(rc, 0, "{l:?}");
    // SIGDN_FILESYSPATH 0x80058000 -> the file system path; SIGDN_NORMALDISPLAY 0 -> the name relative to the parent (S-dlg1-sigdn); SIGDN_URL 0x80068000 -> file:///... (S-dlg1-sigdn). The item is a file with
    // SFGAO_FILESYSTEM 0x40000000 and SFGAO_STREAM 0x00400000 (S_OK: the attributes match the mask exactly); asking SFGAO_FOLDER of a file gives S_FALSE with the bit clear; Compare: same -> S_OK and 0,
    // different -> S_FALSE and nonzero (IShellItem::Compare page); the parent of ...\serina1-serum2.flp is the folder.
    assert_eq!(
        l,
        [
            "show 0",
            "getresult 0",
            "sigdn-filesyspath 0",
            "  C:\\Users\\user\\Downloads\\serina1-serum2\\serina1-serum2.flp",
            "sigdn-normal 0",
            "  serina1-serum2.flp",
            "sigdn-url 0",
            "  file:///C:/Users/user/Downloads/serina1-serum2/serina1-serum2.flp",
            "attrs-file 0 0x40400000",
            "attrs-folder-ask 0x1 0",
            "parent 0",
            "compare-self 0 0",
            "compare-parent 0x1 1",
            "getfilename 0",
            "  serina1-serum2.flp",
            "request filters 0 options 0x1800",
        ]
    );
}

/// Like `run`, with extra file-scope C text between the preamble and `main`.
fn run_with_globals(name: &str, with_rows: bool, globals: &str, main: &str) -> (i32, Vec<String>) {
    run_env(name, with_rows, globals, main, &[])
}

/// Like `run_with_globals`, with environment variables for the program.
fn run_env(name: &str, with_rows: bool, globals: &str, main: &str, envs: &[(&str, &str)]) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let hook = r#"
void fle_imp_CoCreateInstance(void)
{
    uint64_t g = R[1], ppv = fle_arg(5); uint32_t hr;
    if (ppv == 0) fle_stop("CoCreateInstance", "ppv is NULL");
    fle_st(ppv, 8, 0);
    if (fle_filedlg_cocreate(g, R[2], (uint32_t)R[8], R[9], ppv, &hr)) { R[0] = hr; return; }
    R[0] = FLE_HR_REGDB_E_CLASSNOTREG;
}
"#;
    std::fs::write(&c, format!("{}{hook}\n{globals}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n", pre(with_rows))).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"])
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_filedlg.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut cmd = Command::new(&exe);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let r = cmd.output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}

#[test]
fn the_selection_must_exist_when_the_file_must_exist() {
    let main = format!(
        r#"{DLG}
    static const char *paths[1] = {{ "C:\\nowhere\\x.flp" }};
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_n = 1;
    vcall(d, 3, 0, 0, 0);
    printf("not reached\n");
"#
    );
    let (rc, l) = run_with_globals("dlg-mustexist", true, PICK, &main);
    assert_eq!(rc, 3, "{l:?}");
    // the default options include FOS_FILEMUSTEXIST: a real dialog would not accept it; the model has no user to retry, so the harness's mistake stops by name
    assert!(l[0].starts_with("STOP IModalWindow::Show: FOS_FILEMUSTEXIST: the selection the harness gave is not a file"), "{l:?}");
}

#[test]
fn several_items_need_get_results_and_get_result_fails() {
    let main = format!(
        r#"{DLG}
    static const char *paths[2] = {{ "C:\\a\\1.flp", "C:\\a\\2.flp" }};
    vol_add("C:\\a\\1.flp", 0x20); vol_add("C:\\a\\2.flp", 0x20);
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_paths[1] = paths[1]; pick_n = 2;
    vcall(d, 9, 0x1800 | 0x200, 0, 0);
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    uint64_t item = 0xAAAA, arr = 0;
    printf("getresult %#llx item %#llx\n", U(vcall(d, 20, P(&item), 0, 0)), U(item));
    printf("getresults %#llx\n", U(vcall(d, 27, P(&arr), 0, 0)));
    uint32_t cnt = 0;
    printf("count %#llx %u\n", U(vcall(arr, 7, P(&cnt), 0, 0)), cnt);
    uint64_t it0 = 0, it5 = 0xAAAA;
    printf("item0 %#llx\n", U(vcall(arr, 8, 0, P(&it0), 0)));
    printf("item5 %#llx ptr %#llx\n", U(vcall(arr, 8, 5, P(&it5), 0)), U(it5));
    uint64_t nm = 0;
    vcall(it0, 5, 0x80058000u, P(&nm), 0);
    {{ uint16_t *w = (uint16_t *)(uintptr_t)nm; char b[100]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
"#
    );
    let (rc, l) = run_with_globals("dlg-multi", true, PICK, &main);
    assert_eq!(rc, 0, "{l:?}");
    // IFileDialog::GetResult: 'If multiple items were chosen, this method will fail' (E_FAIL 0x80004005 is the model's choice); IShellItemArray::GetItemAt: E_FAIL out of bounds (its page)
    assert_eq!(l, ["show 0", "getresult 0x80004005 item 0", "getresults 0", "count 0 2", "item0 0", "item5 0x80004005 ptr 0", "  C:\\a\\1.flp"]);
}

#[test]
fn the_event_handler_gets_add_ref_on_advise_on_file_ok_and_release_on_unadvise() {
    let main = format!(
        r#"{DLG}
    static const char *paths[1] = {{ "C:\\a\\1.flp" }};
    vol_add("C:\\a\\1.flp", 0x20);
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_n = 1;
    handler_obj[0] = P(handler_vt);
    uint32_t ck = 0;
    printf("advise %#llx cookie %u\n", U(vcall(d, 7, P(handler_obj), P(&ck), 0)), ck);
    printf("calls-after-advise %u first-va %#llx\n", ngcalls, U(gcalls[0][0]));
    onfileok_hr = 0;
    printf("show-ok %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("calls %u last-va %#llx arg %d\n", ngcalls, U(gcalls[ngcalls - 1][0]), gcalls[ngcalls - 1][2] == d);
    onfileok_hr = 1;   /* S_FALSE: refuse */
    printf("show-refused %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    uint64_t item = 0xAAAA;
    printf("getresult-after-refuse %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("unadvise %#llx\n", U(vcall(d, 8, ck, 0, 0)));
    printf("calls %u last-va %#llx\n", ngcalls, U(gcalls[ngcalls - 1][0]));
"#
    );
    let (rc, l) = run_with_globals("dlg-events", true, PICK, &main);
    assert_eq!(rc, 0, "{l:?}");
    // Advise: the model keeps its own reference (AddRef = slot 1 of the handler: VA 0x1000, S-dlg1-advise-ref); OnFileOk = slot 3 (0x3000) with the dialog as pfd; S_FALSE refuses and 'the dialog should remain open'
    // (the model ends it as a cancel, S-dlg1-onfileok-refuse: 0x800704C7); Unadvise releases (slot 2: 0x2000)
    assert_eq!(
        l,
        [
            "advise 0 cookie 1",
            "calls-after-advise 1 first-va 0x1000",
            "show-ok 0",
            "calls 2 last-va 0x3000 arg 1",
            "show-refused 0x800704c7",
            "getresult-after-refuse 0x8000ffff",
            "unadvise 0",
            "calls 4 last-va 0x2000",
        ]
    );
}

#[test]
fn the_picked_folder_must_be_a_directory_with_pick_folders() {
    let main = format!(
        r#"{DLG}
    static const char *paths[1] = {{ "C:\\a" }};
    vol_add("C:\\a", 0x10);
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_n = 1;
    vcall(d, 9, 0x20 | 0x800, 0, 0);
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    uint64_t item = 0;
    vcall(d, 20, P(&item), 0, 0);
    uint32_t at = 0;
    printf("attrs-folder %#llx %#x\n", U(vcall(item, 6, 0x20000000u | 0x40000000u, P(&at), 0)), at);
"#
    );
    let (rc, l) = run_with_globals("dlg-folder", true, PICK, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0", "attrs-folder 0 0x60000000"]);
}

#[test]
fn sh_create_item_from_parsing_name_follows_the_page_and_the_volume() {
    let (rc, l) = run(
        "dlg-sh-create",
        true,
        r#"
    uint64_t item = 0xAAAA, nm = 0;
    uint16_t *ok = W("C:\\Users\\user\\Documents"), *none = W("C:\\no\\such"), *rel = W("foo.flp");
    vol_add("C:\\Users\\user\\Documents", 0x10);
    printf("ok %#llx\n", U(call(fle_imp_SHCreateItemFromParsingName, P(ok), 0, P(IID_Item), P(&item))));
    printf("is-item %d\n", item != 0 && item != 0xAAAA);
    vcall(item, 5, 0x80058000u, P(&nm), 0);
    { uint16_t *w = (uint16_t *)(uintptr_t)nm; char b[100]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }
    item = 0xAAAA;
    printf("missing %#llx ppv %#llx\n", U(call(fle_imp_SHCreateItemFromParsingName, P(none), 0, P(IID_Item), P(&item))), U(item));
    printf("live %u\n", fle_filedlg_live_objects());
    call(fle_imp_SHCreateItemFromParsingName, P(rel), 0, P(IID_Item), P(&item));
    printf("not reached\n");
"#,
    );
    assert_eq!(rc, 3, "{l:?}");
    // the missing path gives HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) = 0x80070002 (S-dlg1-parse-notfound: a guess of the real behaviour, not from the page); a relative path is outside the model and stops by name
    assert_eq!(l[0..5], ["ok 0", "is-item 1", "  C:\\Users\\user\\Documents", "missing 0x80070002 ppv 0", "live 1"]);
    assert!(l[5].starts_with("STOP SHCreateItemFromParsingName: a parsing name that is not a drive-absolute file system path"), "{l:?}");
}

#[test]
fn sh_get_known_folder_path_returns_the_virtual_profile_paths() {
    let (rc, l) = run(
        "dlg-kf",
        true,
        r#"
    uint64_t pp = 0xAAAA;
    printf("docs %#llx\n", U(call(fle_imp_SHGetKnownFolderPath, P(FOLDERID_Docs), 0, 0, P(&pp))));
    { uint16_t *w = (uint16_t *)(uintptr_t)pp; char b[100]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }
    call(fle_imp_SHGetKnownFolderPath, P(FOLDERID_Bogus), 0, 0, P(&pp));
    printf("not reached\n");
"#,
    );
    assert_eq!(rc, 3, "{l:?}");
    // the path has no trailing backslash and is allocated with CoTaskMemAlloc (the page); the virtual profile is C:\Users\user (decision 0126), Documents below it (a choice there and here)
    assert_eq!(l[0..2], ["docs 0", "  C:\\Users\\user\\Documents"]);
    assert!(l[2].starts_with("STOP SHGetKnownFolderPath: a KNOWNFOLDERID that the model does not know"), "{l:?}");
}

/// Decision 0183 (wrp1): the Program Files folders. GUIDs and default paths are those of the Learn page KNOWNFOLDERID (written here from the page, not from the C code): for a 64-bit application on a 64-bit system
/// ProgramFiles and ProgramFilesX64 are C:\Program Files, ProgramFilesX86 is C:\Program Files (x86), the Common Files folders are below them.
#[test]
fn sh_get_known_folder_path_knows_the_program_files_folders() {
    let (rc, l) = run(
        "dlg-kf-pf",
        true,
        r#"
    static const uint8_t X64[16] = GUID(0x6d809377u, 0x6af0, 0x444b, 0x89, 0x57, 0xa3, 0x77, 0x3f, 0x02, 0x20, 0x0e);
    static const uint8_t X86[16] = GUID(0x7c5a40efu, 0xa0fb, 0x4bfc, 0x87, 0x4a, 0xc0, 0xf2, 0xe0, 0xb9, 0xfa, 0x8e);
    static const uint8_t COM[16] = GUID(0xf7f1ed05u, 0x9f6d, 0x47a2, 0xaa, 0xae, 0x29, 0xd3, 0x17, 0xc6, 0xf0, 0x66);
    static const uint8_t COM64[16] = GUID(0x6365d5a7u, 0x0f0d, 0x45e5, 0x87, 0xf6, 0x0d, 0xa5, 0x6b, 0x6a, 0x4f, 0x7d);
    static const uint8_t COM86[16] = GUID(0xde974d24u, 0xd9c6, 0x4d3e, 0xbf, 0x91, 0xf4, 0x45, 0x51, 0x20, 0xb9, 0x17);
    static const uint8_t PD[16] = GUID(0x62ab5d82u, 0xfdc1, 0x4dc3, 0xa9, 0xdd, 0x07, 0x0d, 0x1d, 0x49, 0x5d, 0x97);
    const uint8_t *g[6] = { X64, X86, COM, COM64, COM86, PD };
    unsigned k;
    for (k = 0; k < 6; k++) {
        uint64_t pp = 0xAAAA;
        uint64_t hr = call(fle_imp_SHGetKnownFolderPath, P(g[k]), 0, 0, P(&pp));
        uint16_t *w = (uint16_t *)(uintptr_t)pp; char b[100]; unsigned i;
        for (i = 0; w[i]; i++) b[i] = (char)w[i];
        b[i] = 0;
        printf("%u %llu %s\n", k, U(hr), b);
    }
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(
        l,
        ["0 0 C:\\Program Files", "1 0 C:\\Program Files (x86)", "2 0 C:\\Program Files\\Common Files", "3 0 C:\\Program Files\\Common Files", "4 0 C:\\Program Files (x86)\\Common Files", "5 0 C:\\ProgramData"]
    );
}

#[test]
fn the_rows_file_names_every_vtable_slot_and_every_function_exists() {
    // harness/seeds/dlg1-imports.txt: decision 0171's rows first and unchanged, then the rows of this decision; every row names a function that fle_filedlg.h declares
    let rows = std::fs::read_to_string(manifest().join("../../harness/seeds/dlg1-imports.txt")).unwrap();
    let ia1 = std::fs::read_to_string(manifest().join("../../harness/seeds/ia1-imports.txt")).unwrap();
    assert!(rows.starts_with(&ia1), "the rows of decision 0171 must come first and unchanged");
    let h = std::fs::read_to_string(rt().join("fle_filedlg.h")).unwrap();
    let own: Vec<&str> = rows[ia1.len()..].lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).collect();
    let mut want: Vec<String> = vec![
        "shell32.dll SHCreateItemFromParsingName fle_imp_SHCreateItemFromParsingName".into(),
        "shell32.dll SHGetKnownFolderPath fle_imp_SHGetKnownFolderPath".into(),
    ];
    for n in row_names() {
        want.push(format!("shell32.dll {n} fle_imp_{}", n.replace("::", "_")));
    }
    want.push("mpr.dll WNetGetUniversalNameW fle_imp_WNetGetUniversalNameW".into());
    want.push("kernel32.dll GetFileInformationByHandle fle_imp_GetFileInformationByHandle".into());
    want.push("kernel32.dll SetEndOfFile fle_imp_SetEndOfFile".into());
    assert_eq!(own, want.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    // every function is declared by name in the header: the explicit ones, or through the method lists
    let wall_h = std::fs::read_to_string(rt().join("fle_wall_dlg1.h")).unwrap();
    for w in &want {
        let f = w.split(' ').nth(2).unwrap();
        if f == "fle_imp_WNetGetUniversalNameW" || f == "fle_imp_GetFileInformationByHandle" || f == "fle_imp_SetEndOfFile" {
            assert!(wall_h.contains(&format!("void {f}(void);")), "{f}");
            continue;
        }
        let declared = h.contains(&format!("{f}(void)")) || {
            let tail = f.strip_prefix("fle_imp_").unwrap();
            let (iface, m) = tail.split_once('_').unwrap();
            let list = match iface {
                "IFileOpenDialog" => "FLE_FILEDLG_FOR_EACH_DIALOG_METHOD",
                "IShellItem" => "FLE_FILEDLG_FOR_EACH_ITEM_METHOD",
                "IShellItemArray" => "FLE_FILEDLG_FOR_EACH_ARRAY_METHOD",
                "IFileDialogCustomize" => "FLE_FILEDLG_FOR_EACH_CUSTOM_METHOD",
                _ => "",
            };
            !list.is_empty() && h.contains(&format!("X({m})"))
        };
        assert!(declared, "{f} is not declared in fle_filedlg.h");
    }
}

#[test]
fn the_native_harness_picker_reads_the_environment_and_says_that_the_harness_gave_the_selection() {
    // No picker installed: Show asks the environment (FLE_FILEDLG_PICK, '|' between items). The selection stands for the user's input; stderr says so (the program's stderr is not captured, the behaviour is).
    let main = format!(
        r#"{DLG}
    vol_add("C:\\p\\a b.flp", 0x20);
    uint64_t item = 0, nm = 0;
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    vcall(item, 5, 0x80058000u, P(&nm), 0);
    {{ uint16_t *w = (uint16_t *)(uintptr_t)nm; char b[100]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
"#
    );
    let (rc, l) = run_env("dlg-envpick", true, "", &main, &[("FLE_FILEDLG_PICK", "C:\\p\\a b.flp")]);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0", "getresult 0", "  C:\\p\\a b.flp"]);
    // the same through a list file (a path with spaces does not survive the harness script's setenv)
    let list = work().join(etxt_common::uniq("pick-list.txt"));
    std::fs::write(&list, "C:\\p\\a b.flp\n").unwrap();
    let (rc, l) = run_env("dlg-filepick", true, "", &main, &[("FLE_FILEDLG_PICK_FILE", list.to_str().unwrap())]);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0", "getresult 0", "  C:\\p\\a b.flp"]);
}

#[test]
fn the_customize_interface_is_a_separate_sub_object_with_the_sdk_order_and_shares_the_dialog_references() {
    // IFileDialogCustomize: IUnknown 0-2, then the 27 methods of shobjidl.idl (EnableOpenDropDown ... SetControlItemText); its `this` differs from the dialog's (multiple inheritance), the same pointer every
    // time (COM identity), and its references are the dialog's
    let mut main = String::from(DLG);
    main.push_str("    uint64_t c = 0, c2 = 0, back = 0;\n    static const char *want[] = {\"ShellObj::QueryInterface\", \"ShellObj::AddRef\", \"ShellObj::Release\", ");
    for m in CUSTOM_METHODS {
        main.push_str(&format!("\"IFileDialogCustomize::{m}\", "));
    }
    main.push_str(
        r#"};
    printf("qi %#llx\n", U(vcall(d, 0, P(IID_Customize), P(&c), 0)));
    printf("distinct %d\n", c != 0 && c != d);
    vcall(d, 0, P(IID_Customize), P(&c2), 0);
    printf("same %d\n", c == c2);
    { unsigned k; int bad = 0; for (k = 0; k < 30; k++) if (strcmp(va_name(slot(c, k)), want[k]) != 0) { printf("slot %u is %s\n", k, va_name(slot(c, k))); bad++; } printf("bad %d\n", bad); }
    printf("qi-dialog-from-custom %#llx same-as-d %d\n", U(vcall(c, 0, P(IID_FOD), P(&back), 0)), back == d);
    printf("addref %llu\n", U(vcall(c, 1, 0, 0, 0)));
    printf("release %llu\n", U(vcall(c, 2, 0, 0, 0)));
    printf("live %u\n", fle_filedlg_live_objects());
"#,
    );
    let (rc, l) = run("dlg-custom", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // references: 1 (create) + QI + QI + QI(IFileOpenDialog from the sub-object) = 4, AddRef 5, Release 4; two live objects (the dialog and its sub-object)
    assert_eq!(l, ["qi 0", "distinct 1", "same 1", "bad 0", "qi-dialog-from-custom 0 same-as-d 1", "addref 5", "release 4", "live 2"]);
}

const CUSTOM_PRE: &str = r#"
    uint64_t c = 0;
    vcall(d, 0, P(IID_Customize), P(&c), 0);
    uint16_t *menu = W("Recent"), *i1 = W("First"), *i2 = W("Second"), *chk = W("Read only"), *txt = W("name");
    /* IFileDialogCustomize slots: EnableOpenDropDown 3, AddMenu 4, AddPushButton 5, AddComboBox 6, AddRadioButtonList 7, AddCheckButton 8, AddEditBox 9, AddSeparator 10, AddText 11, SetControlLabel 12,
       GetControlState 13, SetControlState 14, GetEditBoxText 15, SetEditBoxText 16, GetCheckButtonState 17, SetCheckButtonState 18, AddControlItem 19, RemoveControlItem 20, RemoveAllControlItems 21,
       GetControlItemState 22, SetControlItemState 23, GetSelectedControlItem 24, SetSelectedControlItem 25, StartVisualGroup 26, EndVisualGroup 27, MakeProminent 28, SetControlItemText 29 */
"#;

#[test]
fn customize_controls_are_kept_with_the_default_state_enabled_and_visible() {
    let main = format!(
        r#"{DLG}{CUSTOM_PRE}
    uint32_t st = 77;
    printf("menu %#llx\n", U(vcall(c, 4, 100, P(menu), 0)));
    printf("item1 %#llx\n", U(vcall(c, 19, 100, 1, P(i1))));
    printf("item2 %#llx\n", U(vcall(c, 19, 100, 2, P(i2))));
    printf("state %#llx %u\n", U(vcall(c, 13, 100, P(&st), 0)), st);
    printf("set-state %#llx\n", U(vcall(c, 14, 100, 2, 0)));
    printf("state %#llx %u\n", U(vcall(c, 13, 100, P(&st), 0)), st);
    printf("item-state %#llx %u\n", U(vcall(c, 22, 100, 2, P(&st))), st);
    printf("combo %#llx\n", U(vcall(c, 6, 200, 0, 0)));
    vcall(c, 19, 200, 11, P(i1)); vcall(c, 19, 200, 12, P(i2));
    uint32_t sel = 0;
    printf("sel-default %#llx %u\n", U(vcall(c, 24, 200, P(&sel), 0)), sel);
    printf("set-sel %#llx\n", U(vcall(c, 25, 200, 12, 0)));
    printf("sel %#llx %u\n", U(vcall(c, 24, 200, P(&sel), 0)), sel);
    printf("check %#llx\n", U(vcall(c, 8, 300, P(chk), 1)));
    printf("checked %#llx %u\n", U(vcall(c, 17, 300, P(&st), 0)), st);
    printf("uncheck %#llx\n", U(vcall(c, 18, 300, 0, 0)));
    printf("checked %#llx %u\n", U(vcall(c, 17, 300, P(&st), 0)), st);
    printf("edit %#llx\n", U(vcall(c, 9, 400, P(txt), 0)));
    uint64_t out = 0;
    printf("edit-text %#llx\n", U(vcall(c, 15, 400, P(&out), 0)));
    {{ uint16_t *w = (uint16_t *)(uintptr_t)out; char b[40]; unsigned i; for (i = 0; w[i]; i++) b[i] = (char)w[i]; b[i] = 0; printf("  %s\n", b); }}
    printf("group %#llx end %#llx\n", U(vcall(c, 26, 500, P(menu), 0)), U(vcall(c, 27, 0, 0, 0)));
    printf("prominent %#llx\n", U(vcall(c, 28, 100, 0, 0)));
    printf("remove %#llx\n", U(vcall(c, 20, 100, 1, 0)));
    printf("remove-all %#llx\n", U(vcall(c, 21, 100, 0, 0)));
"#
    );
    let (rc, l) = run("dlg-custom-controls", true, &main);
    assert_eq!(rc, 0, "{l:?}");
    // the Add* pages: 'The default state for this control is enabled and visible' (CDCS_ENABLEDVISIBLE = 3); an option-less combo box answers its first item as the selection (S-dlg1-custom-sel)
    assert_eq!(
        l,
        [
            "menu 0", "item1 0", "item2 0", "state 0 3", "set-state 0", "state 0 2", "item-state 0 3", "combo 0", "sel-default 0 11", "set-sel 0", "sel 0 12", "check 0", "checked 0 1", "uncheck 0",
            "checked 0 0", "edit 0", "edit-text 0", "  name", "group 0 end 0", "prominent 0", "remove 0", "remove-all 0",
        ]
    );
}

#[test]
fn customize_stops_by_name_where_the_pages_name_no_result() {
    // 'This method cannot be called on menus' (GetSelectedControlItem page); an unknown id, a used id and an item in a non-container have no HRESULT on the pages
    let cases: [(&str, &str, &str); 4] = [
        ("dlg-custom-menu-sel", "vcall(c, 4, 100, P(menu), 0); uint32_t s; vcall(c, 24, 100, P(&s), 0);", "STOP IFileDialogCustomize::GetSelectedControlItem: the page: 'This method cannot be called on menus'"),
        ("dlg-custom-unknown", "uint32_t s; vcall(c, 13, 999, P(&s), 0);", "STOP IFileDialogCustomize::GetControlState: control id 999 does not exist"),
        ("dlg-custom-dup", "vcall(c, 4, 100, P(menu), 0); vcall(c, 5, 100, P(menu), 0);", "STOP IFileDialogCustomize::AddPushButton: control id 100 is already used"),
        ("dlg-custom-nocontainer", "vcall(c, 8, 300, P(chk), 0); vcall(c, 19, 300, 1, P(i1));", "STOP IFileDialogCustomize::AddControlItem: the control is not a container"),
    ];
    for (name, body, want) in cases {
        let main = format!("{DLG}{CUSTOM_PRE}\n    {body}\n    printf(\"not reached\\n\");\n");
        let (rc, l) = run(name, true, &main);
        assert_eq!(rc, 3, "{name}: {l:?}");
        assert!(l[0].starts_with(want), "{name}: {l:?}");
    }
}

#[test]
fn a_handler_that_answers_e_notimpl_to_on_file_ok_lets_the_dialog_go_on_and_other_values_stop() {
    let body = |hr: &str| {
        format!(
            r#"{DLG}
    static const char *paths[1] = {{ "C:\\a\\1.flp" }};
    vol_add("C:\\a\\1.flp", 0x20);
    fle_filedlg_set_picker(picker);
    pick_paths[0] = paths[0]; pick_n = 1;
    handler_obj[0] = P(handler_vt);
    uint32_t ck = 0;
    vcall(d, 7, P(handler_obj), P(&ck), 0);
    onfileok_hr = {hr};
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
"#
        )
    };
    // E_NOTIMPL = 0x80004001 (Common HRESULT Values): the engine's own handler answers it; the model treats it as 'not handled' (S-dlg1-onfileok-notimpl, a guess of the shell's behaviour)
    let (rc, l) = run_with_globals("dlg-onfileok-notimpl", true, PICK, &body("0x80004001u"));
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0"]);
    // any other failure value is outside the page and stops by name
    let (rc, l) = run_with_globals("dlg-onfileok-other", true, PICK, &body("0x80004005u"));
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP IFileDialogEvents::OnFileOk: the handler returned neither S_OK, S_FALSE nor E_NOTIMPL"), "{l:?}");
}

fn wall(name: &str, main: &str) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let src = format!(
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_wall_dlg1.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
void fle_stop(const char *api, const char *why) {{ printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }}
void fle_set_last_error(uint32_t c) {{ (void)c; }}
int fle_vfs_file_stat(const char *api, uint64_t h, const void **node, uint32_t *attrs, uint64_t *ct, uint64_t *at, uint64_t *wt, uint64_t *size) {{ (void)api; (void)h; (void)node; (void)attrs; (void)ct; (void)at; (void)wt; (void)size; return 0; }}
int fle_vfs_file_set_end(const char *api, uint64_t h) {{ (void)api; (void)h; return 0; }}
static uint16_t *W(const char *s) {{ size_t n = strlen(s), i; uint16_t *d = malloc((n + 1) * 2); for (i = 0; i <= n; i++) d[i] = (unsigned char)s[i]; return d; }}
static uint64_t P(const void *p) {{ return (uint64_t)(uintptr_t)p; }}
int main(void)
{{
{main}
    return 0;
}}
"#
    );
    std::fs::write(&c, src).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-fPIE", "-pie"])
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_wall_dlg1.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}

#[test]
fn wnet_get_universal_name_says_not_connected_for_the_local_drive_and_stops_for_what_the_page_leaves_open() {
    let (rc, l) = wall(
        "wall-wnet",
        r#"
    uint16_t *p = W("C:\\Users\\user\\Downloads\\a.zip");
    uint32_t size = 1024;
    uint8_t buf[1024];
    memset(buf, 0xAA, sizeof buf);
    R[1] = P(p); R[2] = 1; R[8] = P(buf); R[9] = P(&size);
    fle_imp_WNetGetUniversalNameW();
    printf("ret %llu size %u buf0 %#x\n", U(R[0]), size, buf[0]);
    R[1] = P(W("C:")); R[2] = 2; R[8] = P(buf); R[9] = P(&size);
    fle_imp_WNetGetUniversalNameW();
    printf("ret %llu\n", U(R[0]));
    R[1] = P(W("relative\\path")); R[2] = 1; R[8] = P(buf); R[9] = P(&size);
    fle_imp_WNetGetUniversalNameW();
    printf("not reached\n");
"#,
    );
    // ERROR_NOT_CONNECTED = 2250 ('The device specified by the lpLocalPath parameter is not redirected'); the buffer and the size are not touched (the structure is stored on success only)
    assert_eq!(rc, 3, "{l:?}");
    assert_eq!(l[0..2], ["ret 2250 size 1024 buf0 0xaa", "ret 2250"]);
    assert!(l[2].starts_with("STOP WNetGetUniversalNameW: lpLocalPath is not a drive-based path"), "{l:?}");
}

fn cmp_run(name: &str, env: Option<&str>) -> (i32, String, String) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let src = r#"
#include <stdio.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
uint64_t R[16];
static uint64_t st[16];
#define P(...) ((uint64_t)(uintptr_t)(__VA_ARGS__))
static uint64_t cmpf(const uint16_t *a, int na, const uint16_t *b, int nb, uint32_t flags)
{ R[1] = 0x409; R[2] = flags; R[8] = P(a); R[9] = (uint64_t)(int64_t)na; st[5] = P(b); st[6] = (uint64_t)(int64_t)nb; R[4] = P(st); R[0] = 0xDEAD; fle_imp_CompareStringW(); return R[0]; }
static uint64_t cmp(const char *a, const char *b, uint32_t flags)
{
    uint16_t x[40], y[40]; int i, n = 0, m = 0;
    for (i = 0; a[i]; i++) x[n++] = (unsigned char)a[i] == 0xE3 ? 0x30aa : (unsigned char)a[i];   /* "\xE3" stands for U+30AA */
    for (i = 0; b[i]; i++) y[m++] = (unsigned char)b[i];
    return cmpf(x, n, y, m, flags);
}
int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    (void)argc;
    printf("backup-vs-same %llu\n", (unsigned long long)cmp("\\Backup\\", "\\bAcKuP\\", 1));
    printf("%s\n", argv[1]);
    printf("backup-vs-serum %llu\n", (unsigned long long)cmp("\\Backup\\", "-serum2\\", 1));
    printf("serum-vs-backup %llu\n", (unsigned long long)cmp("-serum2\\", "\\Backup\\", 1));
    printf("coop-vs-co-op %llu\n", (unsigned long long)cmp("coop", "co-op", 0));
    printf("co-op-vs-coop %llu\n", (unsigned long long)cmp("co-op", "coop", 0));
    printf("con-vs-co-op %llu\n", (unsigned long long)cmp("con", "co-op", 0));
    printf("bills-vs-bill's %llu\n", (unsigned long long)cmp("bills", "bill's", 0));
    printf("can't-vs-cant %llu\n", (unsigned long long)cmp("can't", "cant", 0));
    printf("can't-vs-cannot %llu\n", (unsigned long long)cmp("can't", "cannot", 0));
    printf("kana-vs-ascii %llu\n", (unsigned long long)cmp("\xE3", "UNSORTED", 8));
    printf("ascii-vs-kana %llu\n", (unsigned long long)cmp("UNSORTED", "\xE3", 8));
    printf("a-b-vs-ab- %llu\n", (unsigned long long)cmp("a-b", "ab-", 0));
    return 0;
}
"#;
    std::fs::write(&c, src).unwrap();
    let rtd = rt();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pie"])
        .arg("-I")
        .arg(&rtd)
        .arg(&c)
        .arg(rtd.join("fle_locale.c"))
        .arg(rtd.join("fle_imports.c"))
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", dir.join("tmp"))
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let mut cmd = Command::new(&exe);
    cmd.arg("-").env_remove("FLE_CMP_EXTEND");
    if let Some(v) = env {
        cmd.env("FLE_CMP_EXTEND", v);
    }
    let r = cmd.output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).to_string(), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn the_hyphen_rule_of_compare_string_follows_the_word_sort_page_and_hyphens_at_different_places_stop_by_name() {
    // Decision 0229 (ref1, PROVISIONAL): the hyphen and apostrophe rule (1) is no longer behind FLE_CMP_EXTEND (old: the default stopped at the first hyphen case, "backup-vs-same 2" was the only line); the rule (2) for a unit above
    // U+007F against ASCII has no source and stays opt-in: without the variable the run goes through the hyphen cases and stops at the kana case.
    let (rc, out, err) = cmp_run("cmp-default", None);
    assert_ne!(rc, 0, "{out} {err}");
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["backup-vs-same 2", "-", "backup-vs-serum 1", "serum-vs-backup 3", "coop-vs-co-op 1", "co-op-vs-coop 3", "con-vs-co-op 1", "bills-vs-bill's 1", "can't-vs-cant 3", "can't-vs-cannot 3", "kana-vs-ascii 1", "ascii-vs-kana 3"]
    );
    // decision 0284 (B): old: the run stopped at the kana case ("CompareStringW: character other than an ASCII letter or digit in unequal strings"); the sorting weights answer it in every build now (U+30AA is Script Member 3,
    // "U" is 14: the kana is the smaller, 1 / 3 with flags NORM_IGNOREKANATYPE). The stop is now the last case, "a-b" against "ab-": equal without their hyphens, with as many of them, in different places.
    assert!(err.contains("CompareStringW: strings equal without their hyphens and apostrophes and with as many of them"), "{err}");
    // decision 0243: FLE_CMP_EXTEND (a unit above U+007F is the greater, S-dlg1-cmp-nonascii, no source) is deleted; the stop by name above is the only behaviour
}

fn vfs_seed_run(name: &str, list: &[u8], body: &str) -> (i32, String, String) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let tsv = dir.join(format!("{u}.tsv"));
    std::fs::write(&tsv, list).unwrap();
    let src = format!(
        r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "vfs_seed.h"
#include "fle_wall_dlg1.h"
uint64_t R[16];
static uint64_t attrs(const uint16_t *w) {{ return VS_CALL(fle_imp_GetFileAttributesW, VS_A(w)); }}
int main(void)
{{
    setvbuf(stdout, NULL, _IONBF, 0);
    setenv("FLE_VFS_SEED", "{tsv}", 1);
    int rc = vfs_seed_from_env();
    printf("rc %d\n", rc);
{body}
    return 0;
}}
"#,
        tsv = tsv.display()
    );
    std::fs::write(&c, src).unwrap();
    let rtd = rt();
    let harness = manifest().join("../../harness");
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-w", "-fPIE", "-pie", "-I"]).arg(&rtd).arg("-I").arg(&harness).arg(&c);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c", "fle_wall_dlg1.c"] {
        cmd.arg(rtd.join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let r = Command::new(&exe).output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).to_string(), String::from_utf8_lossy(&r.stderr).to_string())
}

#[test]
fn the_vfs_seed_list_takes_utf8_names_and_makes_utf16_names_while_ascii_is_unchanged() {
    let dir = work();
    let host = dir.join(format!("{}.bin", etxt_common::per_process("dlg1-seed-host")));
    std::fs::write(&host, b"hello").unwrap();
    let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"; // SHA-256("hello"), FIPS 180-4
    // "C:\p\" + U+30AA U+30FC (katakana, one UTF-16 unit each) + " " + U+1F600 (a 4-byte UTF-8 sequence = 2 UTF-16 units) + ".wav"
    let list = format!("C:\\p\\\u{30aa}\u{30fc} \u{1f600}.wav\t{}\t{sha}\nC:\\p\\plain.wav\t{}\t{sha}\n", host.display(), host.display());
    let (rc, out, err) = vfs_seed_run(
        "seed-utf8",
        list.as_bytes(),
        r#"
    static const uint16_t jp[] = { 'C', ':', '\\', 'p', '\\', 0x30aa, 0x30fc, ' ', 0xd83d, 0xde00, '.', 'w', 'a', 'v', 0 };
    static const uint16_t asc[] = { 'C', ':', '\\', 'p', '\\', 'p', 'l', 'a', 'i', 'n', '.', 'w', 'a', 'v', 0 };
    static const uint16_t other[] = { 'C', ':', '\\', 'p', '\\', 0x30aa, 0x30fd, 0 };
    printf("attrs jp %llx ascii %llx other %llx\n", (unsigned long long)attrs(jp), (unsigned long long)attrs(asc), (unsigned long long)attrs(other));
"#,
    );
    assert_eq!(rc, 0, "{out} {err}");
    // both files exist as normal files (0x80); a name that differs in the second katakana does not
    let l: Vec<&str> = out.lines().collect();
    assert!(l.contains(&"vfs seed: done, 2 files, 1 directories created"), "{out}");
    assert_eq!(*l.last().unwrap(), "attrs jp 80 ascii 80 other ffffffff", "{out}");
    assert!(l.iter().any(|s| s.starts_with("vfs seed: C:\\p\\\u{30aa}\u{30fc} \u{1f600}.wav <- ")), "{out}");
    // invalid UTF-8 (a lone continuation byte, an overlong form, a surrogate) is refused by name, nothing is written
    for (label, bytes) in [("lone-continuation", &b"C:\\p\\a\x80b.wav"[..]), ("overlong", &b"C:\\p\\a\xc0\x80.wav"[..]), ("surrogate", &b"C:\\p\\a\xed\xa0\x80.wav"[..])] {
        let mut l = Vec::new();
        l.extend_from_slice(bytes);
        l.extend_from_slice(format!("\t{}\t{sha}\n", host.display()).as_bytes());
        let (rc, out, err) = vfs_seed_run(&format!("seed-bad-{label}"), &l, "");
        assert_eq!(rc, 0, "{label}: {out} {err}");
        assert!(out.contains("rc -1"), "{label}: {out}");
        assert!(err.contains("is not valid UTF-8"), "{label}: {err}");
    }
    // the length limit counts UTF-16 units: 259 units of 3-byte katakana is accepted (777 bytes), 260 is refused with the old message
    // three components (a component is limited to 255 units): 3 ("C:\\") + a + 1 + 100 + 1 + 100 = n units
    let name = |n: usize| format!("C:\\{}\\{}\\{}", "\u{30aa}".repeat(n - 3 - 202), "\u{30aa}".repeat(100), "\u{30aa}".repeat(100));
    for (n, ok) in [(259usize, true), (260, false)] {
        let l = format!("{}\t{}\t{sha}\n", name(n), host.display());
        let (rc, out, err) = vfs_seed_run(&format!("seed-len-{n}"), l.as_bytes(), "");
        assert_eq!(rc, 0, "{out} {err}");
        if ok {
            assert!(out.contains("rc 0"), "{n}: {out} {err}");
        } else {
            assert!(out.contains("rc -1") && err.contains("guest path longer than 259 characters"), "{n}: {out} {err}");
        }
    }
    // ASCII rules are as before: a control character, a forbidden character and 0x7f are still refused with the old message
    for bad in ["C:\\p\\a\u{7f}b", "C:\\p\\a?b", "C:\\p\\a\u{1}b"] {
        let l = format!("{bad}\t{}\t{sha}\n", host.display());
        let (_rc, out, err) = vfs_seed_run("seed-ascii-bad", l.as_bytes(), "");
        assert!(out.contains("rc -1") && err.contains("has a character outside printable ASCII or one of < > : \" / | ? *"), "{bad:?}: {out} {err}");
    }
}

#[test]
fn a_file_handle_truncated_to_32_bits_is_still_the_handle_but_a_stranger_stops_by_name() {
    // Learn, Interprocess communication: "64-bit versions of Windows use 32-bit handles ... only the lower 32 bits are significant, so it is safe to truncate the handle" - the engine's zip reader keeps the handle of
    // CreateFileW in a 32-bit field (decision 0178). The volume's handles are 0x7FFA0000xxxx: the zero-extended low half must reach the same file.
    let dir = work();
    let host = dir.join(format!("{}.bin", etxt_common::per_process("dlg1-handle-host")));
    std::fs::write(&host, b"hello").unwrap();
    let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    let list = format!("C:\\p\\a.bin\t{}\t{sha}\n", host.display());
    let body = |h: &str| {
        format!(
            r#"
    static const uint16_t f[] = {{ 'C', ':', '\\', 'p', '\\', 'a', '.', 'b', 'i', 'n', 0 }};
    uint8_t buf[8] = {{ 0 }};
    uint32_t got = 0;
    uint64_t h = VS_CALL(fle_imp_CreateFileW, VS_A(f), 0x80000000u, 1, 0, 3, 0x80, 0);
    uint64_t t = {h};
    printf("handle %llx truncated %llx\n", (unsigned long long)h, (unsigned long long)t);
    printf("read %llu got %u '%.5s'\n", (unsigned long long)VS_CALL(fle_imp_ReadFile, t, VS_A(buf), 5, VS_A(&got), 0), got, (char*)buf);
    printf("close %llu\n", (unsigned long long)VS_CALL(fle_imp_CloseHandle, t));
"#
        )
    };
    let (rc, out, err) = vfs_seed_run("handle32", list.as_bytes(), &body("h & 0xFFFFFFFFull"));
    assert_eq!(rc, 0, "{out} {err}");
    let l: Vec<&str> = out.lines().collect();
    assert!(l.iter().any(|s| s.starts_with("handle 7ffa0000") && s.contains(" truncated ") && !s.contains("truncated 7ffa")), "{out}");
    assert!(l.contains(&"read 1 got 5 'hello'") && l.contains(&"close 1"), "{out}");
    // a small value that is not the low half of a live handle is as before: not issued by the virtual file system
    let (rc, out, err) = vfs_seed_run("handle32-stranger", list.as_bytes(), &body("0x12345"));
    assert_ne!(rc, 0, "{out} {err}");
    assert!(err.contains("ReadFile: handle was not issued by the virtual file system"), "{out} {err}");
}

#[test]
fn get_file_information_by_handle_and_set_end_of_file_work_on_the_volume() {
    let dir = work();
    let host = dir.join(format!("{}.bin", etxt_common::per_process("dlg1-info-host")));
    std::fs::write(&host, b"hello").unwrap();
    let sha = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    let list = format!("C:\\p\\a.bin\t{}\t{sha}\nC:\\p\\b.bin\t{}\t{sha}\n", host.display(), host.display());
    let (rc, out, err) = vfs_seed_run(
        "fileinfo",
        list.as_bytes(),
        r#"
    static const uint16_t fa[] = { 'C', ':', '\\', 'p', '\\', 'a', '.', 'b', 'i', 'n', 0 };
    static const uint16_t fb[] = { 'C', ':', '\\', 'p', '\\', 'b', '.', 'b', 'i', 'n', 0 };
    uint32_t info[13];
    uint64_t ha = VS_CALL(fle_imp_CreateFileW, VS_A(fa), 0xC0000000u, 1, 0, 3, 0x80, 0);
    uint64_t hb = VS_CALL(fle_imp_CreateFileW, VS_A(fb), 0x80000000u, 1, 0, 3, 0x80, 0);
    memset(info, 0xAA, sizeof info);
    printf("info-a %llu attrs %x serial %x size %u/%u links %u id %u/%u\n", (unsigned long long)VS_CALL(fle_imp_GetFileInformationByHandle, ha, VS_A(info)), info[0], info[7], info[8], info[9], info[10], info[11], info[12]);
    printf("info-b %llu id %u/%u\n", (unsigned long long)VS_CALL(fle_imp_GetFileInformationByHandle, hb, VS_A(info)), info[11], info[12]);
    printf("info-a-again %llu id %u/%u\n", (unsigned long long)VS_CALL(fle_imp_GetFileInformationByHandle, ha, VS_A(info)), info[11], info[12]);
    printf("info-closed %llu\n", (unsigned long long)(VS_CALL(fle_imp_CloseHandle, hb), VS_CALL(fle_imp_GetFileInformationByHandle, hb, VS_A(info))));
    /* SetEndOfFile: move the pointer to 2, cut; then to 8, extend (zero filled) */
    printf("seek %llu\n", (unsigned long long)VS_CALL(fle_imp_SetFilePointer, ha, 2, 0, 0));
    printf("eof %llu\n", (unsigned long long)VS_CALL(fle_imp_SetEndOfFile, ha));
    VS_CALL(fle_imp_GetFileInformationByHandle, ha, VS_A(info));
    printf("size-after-cut %u\n", info[9]);
    VS_CALL(fle_imp_SetFilePointer, ha, 8, 0, 0);
    VS_CALL(fle_imp_SetEndOfFile, ha);
    VS_CALL(fle_imp_GetFileInformationByHandle, ha, VS_A(info));
    printf("size-after-extend %u\n", info[9]);
    {
        uint8_t buf[16] = { 0 };
        uint32_t got = 0;
        VS_CALL(fle_imp_SetFilePointer, ha, 0, 0, 0);
        VS_CALL(fle_imp_ReadFile, ha, VS_A(buf), 8, VS_A(&got), 0);
        printf("bytes %02x %02x %02x %02x %02x %02x %02x %02x got %u\n", buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], got);
    }
"#,
    );
    assert_eq!(rc, 0, "{out} {err}");
    let l: Vec<&str> = out.lines().skip_while(|s| !s.starts_with("rc ")).skip(1).collect();
    // GetFileInformationByHandle returns nonzero; FILE_ATTRIBUTE_NORMAL 0x80 for a plain file; the size is the file's (5 = 0/5); one link; the file id is the order of the first question (S-dlg1-fileinfo), stable per file
    assert_eq!(
        l,
        [
            "info-a 1 attrs 80 serial 464c5601 size 0/5 links 1 id 0/1",
            "info-b 1 id 0/2",
            "info-a-again 1 id 0/1",
            "info-closed 0",
            "seek 2",
            "eof 1",
            "size-after-cut 2",
            "size-after-extend 8",
            // 'h' 'e' kept, the cut bytes ('l' 'l' 'o') are gone: zeros
            "bytes 68 65 00 00 00 00 00 00 got 8",
        ]
    );
}

#[test]
fn exploration_acm_driver_enum_reports_no_driver_and_checks_its_arguments() {
    let (rc, l) = wall(
        "wall-acm",
        r#"
    R[1] = 0x1234; R[2] = 0; R[8] = 0;
    fle_imp_acmDriverEnum();
    printf("ok %llu\n", U(R[0]));
    R[1] = 0; R[2] = 0; R[8] = 0;
    fle_imp_acmDriverEnum();
    printf("null-callback %llu\n", U(R[0]));
    R[1] = 0x1234; R[2] = 0; R[8] = 0x4;
    fle_imp_acmDriverEnum();
    printf("bad-flag %llu\n", U(R[0]));
    R[1] = 0x1234; R[2] = 0; R[8] = 0xC0000000u;
    fle_imp_acmDriverEnum();
    printf("good-flags %llu\n", U(R[0]));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    // no ACM driver: the callback is never called and the call returns 0 (MMSYSERR_NOERROR); MMSYSERR_INVALPARAM 11, MMSYSERR_INVALFLAG 10 (mmsystem.h)
    assert_eq!(l, ["ok 0", "null-callback 11", "bad-flag 10", "good-flags 0"]);
}
