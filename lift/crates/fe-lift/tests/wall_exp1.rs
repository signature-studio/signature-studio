//! Tests of decision 0188 (docs/0188-export-wave.md, PROVISIONAL): the Save half of the common item dialog COM model (runtime/fle_filedlg.c): ole32!CoCreateInstance(CLSID_FileSaveDialog) -> IFileSaveDialog.
//!
//! The expected values are written here from the Microsoft Learn pages (IFileSaveDialog, IFileDialog::SetDefaultExtension / GetOptions, FILEOPENDIALOGOPTIONS: FOS_OVERWRITEPROMPT 0x2, FOS_PATHMUSTEXIST 0x800,
//! FOS_NOREADONLYRETURN 0x8000 are the defaults of the Save dialog), from the vtable ORDER of mingw-w64 shobjidl.h and Wine's shobjidl.idl (docs/0188 tables/vtbl-order-check.txt) and from the GUIDs of the SDK
//! headers - not read back from the C code. The choices the pages leave open are the S-exp1-N of the decision. Nothing is compared with a real Windows. The harness stand-in for the user's choice of the
//! target is the environment variable FLE_FILEDLG_SAVE_FILE / FLE_FILEDLG_SAVE (the Open dialog's FLE_FILEDLG_PICK* are not read by a Save dialog).
//! FLE_EXP1_RT_DIR (a full copy of runtime/) replaces runtime/ for the mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_EXP1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = manifest().join("../../out-exp1/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const DIALOG_METHODS: [&str; 26] = [
    "Show", "SetFileTypes", "SetFileTypeIndex", "GetFileTypeIndex", "Advise", "Unadvise", "SetOptions", "GetOptions", "SetDefaultFolder", "SetFolder", "GetFolder", "GetCurrentSelection", "SetFileName",
    "GetFileName", "SetTitle", "SetOkButtonLabel", "SetFileNameLabel", "GetResult", "AddPlace", "SetDefaultExtension", "Close", "SetClientGuid", "ClearClientData", "SetFilter", "GetResults", "GetSelectedItems",
];
const SAVE_METHODS: [&str; 5] = ["SetSaveAsItem", "SetProperties", "SetCollectedProperties", "GetProperties", "ApplyProperties"];
const ITEM_METHODS: [&str; 5] = ["BindToHandler", "GetParent", "GetDisplayName", "GetAttributes", "Compare"];
const ARRAY_METHODS: [&str; 7] = ["BindToHandler", "GetPropertyStore", "GetPropertyDescriptionList", "GetAttributes", "GetCount", "GetItemAt", "EnumItems"];
const CUSTOM_METHODS: [&str; 27] = [
    "EnableOpenDropDown", "AddMenu", "AddPushButton", "AddComboBox", "AddRadioButtonList", "AddCheckButton", "AddEditBox", "AddSeparator", "AddText", "SetControlLabel", "GetControlState", "SetControlState",
    "GetEditBoxText", "SetEditBoxText", "GetCheckButtonState", "SetCheckButtonState", "AddControlItem", "RemoveControlItem", "RemoveAllControlItems", "GetControlItemState", "SetControlItemState",
    "GetSelectedControlItem", "SetSelectedControlItem", "StartVisualGroup", "EndVisualGroup", "MakeProminent", "SetControlItemText",
];

/// The names of the vtable rows in the order of the rows file.
fn row_names(save: bool) -> Vec<String> {
    let mut v: Vec<String> = ["QueryInterface", "AddRef", "Release"].iter().map(|m| format!("ShellObj::{m}")).collect();
    v.extend(DIALOG_METHODS.iter().map(|m| format!("IFileOpenDialog::{m}")));
    v.extend(ITEM_METHODS.iter().map(|m| format!("IShellItem::{m}")));
    v.extend(ARRAY_METHODS.iter().map(|m| format!("IShellItemArray::{m}")));
    v.extend(CUSTOM_METHODS.iter().map(|m| format!("IFileDialogCustomize::{m}")));
    if save {
        v.extend(SAVE_METHODS.iter().map(|m| format!("IFileSaveDialog::{m}")));
    }
    v
}

fn pre(rows: u8) -> String {
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
static const uint8_t IID_FSD[16] = GUID(0x84bccd23u, 0x5fde, 0x4cdb, 0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab);
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
    if rows >= 1 {
        let names = row_names(rows >= 2);
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
fn run(name: &str, rows: u8, main: &str) -> (i32, Vec<String>) {
    run_with_globals(name, rows, "", main)
}

const DLG: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;


const SAVE: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FSD, 0, IID_FSD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;

fn wstr_print(var: &str) -> String {
    format!("    {{ uint16_t *w = (uint16_t *)(uintptr_t){var}; char b[260]; unsigned i; for (i = 0; w[i] && i < 259; i++) b[i] = (char)w[i]; b[i] = 0; printf(\"  %s\\n\", b); }}\n")
}

#[test]
fn without_the_save_rows_the_save_class_stays_unregistered_as_before() {
    // a build with decision 0178's rows only: CLSID_FileSaveDialog answers REGDB_E_CLASSNOTREG 0x80040154 (COM Error Codes) and *ppv is NULL
    let (rc, l) = run("exp1-nosave", 1, "    uint64_t d = 0xAAAA;\n    uint64_t hr = cocreate(CLSID_FSD, 0, IID_FSD, &d);\n    printf(\"hr %#llx ppv %#llx\\n\", U(hr), U(d));\n");
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["hr 0x80040154 ppv 0"]);
    // the Open class is still served by the same build
    let (rc, l) = run("exp1-nosave-open", 1, "    uint64_t d = 0xAAAA;\n    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);\n    printf(\"hr %#llx ppv-set %d\\n\", U(hr), d != 0 && d != 0xAAAA);\n");
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["hr 0 ppv-set 1"]);
}

#[test]
fn the_save_vtable_is_the_idialog_slots_plus_the_five_save_methods_in_the_sdk_order() {
    // IFileSaveDialog (mingw-w64 shobjidl.h IFileSaveDialogVtbl, Wine shobjidl.idl): IUnknown 0-2, Show 3, IFileDialog 4-26 (same rows as IFileOpenDialog), SetSaveAsItem 27, SetProperties 28,
    // SetCollectedProperties 29, GetProperties 30, ApplyProperties 31
    let mut main = String::from(SAVE);
    let mut expect = vec!["ShellObj::QueryInterface".to_string(), "ShellObj::AddRef".to_string(), "ShellObj::Release".to_string()];
    expect.extend(DIALOG_METHODS[..24].iter().map(|m| format!("IFileOpenDialog::{m}")));
    expect.extend(SAVE_METHODS.iter().map(|m| format!("IFileSaveDialog::{m}")));
    assert_eq!(expect.len(), 32);
    main.push_str("    {\n        static const char *want[] = {");
    for e in &expect {
        main.push_str(&format!("\"{e}\", "));
    }
    main.push_str("};\n        unsigned k; int bad = 0;\n        for (k = 0; k < 32; k++) if (strcmp(va_name(slot(d, k)), want[k]) != 0) { printf(\"slot %u is %s, want %s\\n\", k, va_name(slot(d, k)), want[k]); bad++; }\n        printf(\"bad %d\\n\", bad);\n    }\n");
    // the places that the SDK order puts them, written independently: Show 3, GetResult 20, SetDefaultExtension 22, SetFilter 26, SetSaveAsItem 27, ApplyProperties 31
    main.push_str("    printf(\"Show %s, GetResult %s, SetDefaultExtension %s, SetFilter %s, SetSaveAsItem %s, ApplyProperties %s\\n\", va_name(slot(d, 3)), va_name(slot(d, 20)), va_name(slot(d, 22)), va_name(slot(d, 26)), va_name(slot(d, 27)), va_name(slot(d, 31)));\n");
    let (rc, l) = run("exp1-order", 2, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(
        l,
        [
            "bad 0",
            "Show IFileOpenDialog::Show, GetResult IFileOpenDialog::GetResult, SetDefaultExtension IFileOpenDialog::SetDefaultExtension, SetFilter IFileOpenDialog::SetFilter, SetSaveAsItem IFileSaveDialog::SetSaveAsItem, ApplyProperties IFileSaveDialog::ApplyProperties"
        ]
    );
}

#[test]
fn cocreate_save_checks_aggregation_and_interface_and_qi_answers_the_save_iids_only() {
    let (rc, l) = run(
        "exp1-cocreate",
        2,
        r#"
    uint64_t d = 0xAAAA, outer = 0x1234, q = 0;
    printf("fsd %#llx ppv-set %d\n", U(cocreate(CLSID_FSD, 0, IID_FSD, &d)), d != 0 && d != 0xAAAA);
    printf("qi-fsd %#llx same %d\n", U(vcall(d, 0, P(IID_FSD), P(&q), 0)), q == d);
    printf("qi-fd %#llx same %d\n", U(vcall(d, 0, P(IID_FD), P(&q), 0)), q == d);
    printf("qi-modal %#llx same %d\n", U(vcall(d, 0, P(IID_Modal), P(&q), 0)), q == d);
    printf("qi-unknown %#llx same %d\n", U(vcall(d, 0, P(IID_Unknown), P(&q), 0)), q == d);
    d = 0xAAAA;
    printf("fd %#llx\n", U(cocreate(CLSID_FSD, 0, IID_FD, &d)));
    d = 0xAAAA;
    printf("aggregated %#llx ppv %#llx\n", U(cocreate(CLSID_FSD, outer, IID_FSD, &d)), U(d));
    d = 0xAAAA;
    printf("fod-class-fod-iid %#llx\n", U(cocreate(CLSID_FOD, 0, IID_FOD, &d)));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["fsd 0 ppv-set 1", "qi-fsd 0 same 1", "qi-fd 0 same 1", "qi-modal 0 same 1", "qi-unknown 0 same 1", "fd 0", "aggregated 0x80040110 ppv 0", "fod-class-fod-iid 0"]);
}

#[test]
fn a_save_dialog_does_not_answer_the_open_iid_and_an_open_dialog_does_not_answer_the_save_iid() {
    // the class decides the interface: E_NOINTERFACE would hide a difference, so both stop by name
    let (rc, l) = run("exp1-qi-open-on-save", 2, &format!("{SAVE}    uint64_t q = 0;\n    vcall(d, 0, P(IID_FOD), P(&q), 0);\n    printf(\"not reached\\n\");\n"));
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP ShellObj::QueryInterface: interface {D57C7288-D4AD-4768-BE02-9D969532D960} was asked of an object of kind 1"), "{l:?}");
    let (rc, l) = run("exp1-qi-save-on-open", 2, &format!("{DLG}    uint64_t q = 0;\n    vcall(d, 0, P(IID_FSD), P(&q), 0);\n    printf(\"not reached\\n\");\n"));
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP ShellObj::QueryInterface: interface {84BCCD23-5FDE-4CDB-AEA4-AF64B83D78AB}"), "{l:?}");
    let (rc, l) = run("exp1-cocreate-fsd-fod", 2, "    uint64_t d = 0;\n    cocreate(CLSID_FSD, 0, IID_FOD, &d);\n    printf(\"not reached\\n\");\n");
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP CoCreateInstance: CLSID_FileSaveDialog asked for an interface other than"), "{l:?}");
}

#[test]
fn the_save_dialog_starts_with_the_three_flags_the_options_page_calls_defaults_of_the_save_dialog() {
    let (rc, l) = run(
        "exp1-options",
        2,
        &format!(
            r#"{SAVE}
    uint32_t o = 0;
    printf("get %#llx %#x\n", U(vcall(d, 10, P(&o), 0, 0)), o);
    printf("set %#llx\n", U(vcall(d, 9, 0x1234, 0, 0)));
    printf("get %#llx %#x\n", U(vcall(d, 10, P(&o), 0, 0)), o);
"#
        ),
    );
    assert_eq!(rc, 0, "{l:?}");
    // FOS_OVERWRITEPROMPT 0x2 | FOS_PATHMUSTEXIST 0x800 | FOS_NOREADONLYRETURN 0x8000 = 0x8802 (FILEOPENDIALOGOPTIONS page: 'default value for the Save dialog' / 'This is a default value')
    assert_eq!(l, ["get 0 0x8802", "set 0", "get 0 0x1234"]);
}

#[test]
fn show_gives_the_path_the_harness_names_and_never_reads_the_open_variables() {
    let main = format!(
        r#"{SAVE}
    vol_add("C:\\Users\\user\\Documents", 0x10);
    uint64_t item = 0, name = 0;
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("filesyspath %#llx\n", U(vcall(item, 5, 0x80058000u, P(&name), 0)));
{p1}    printf("getfilename %#llx\n", U(vcall(d, 16, P(&name), 0, 0)));
{p1}    uint32_t at = 0;
    printf("attrs %#llx %#x\n", U(vcall(item, 6, 0x40000000u | 0x00400000u, P(&at), 0)), at);
"#,
        p1 = wstr_print("name")
    );
    let (rc, l) = run_env("exp1-show", 2, "", &main, &[("FLE_FILEDLG_SAVE", "C:\\Users\\user\\Documents\\song.wav"), ("FLE_FILEDLG_PICK", "C:\\nowhere\\other.flp")]);
    assert_eq!(rc, 0, "{l:?}");
    // the item need not exist (a file about to be written): SIGDN_FILESYSPATH is the path; GetFileName after a successful Show is the name of the chosen item; the item has SFGAO_FILESYSTEM and, as a non-folder,
    // SFGAO_STREAM (S-dlg1-sfgao)
    assert_eq!(l, ["show 0", "getresult 0", "filesyspath 0", "  C:\\Users\\user\\Documents\\song.wav", "getfilename 0", "  song.wav", "attrs 0 0x40400000"]);
}

#[test]
fn without_a_save_target_from_the_harness_the_user_cancelled_even_when_an_open_selection_is_set() {
    let (rc, l) = run_env(
        "exp1-cancel",
        2,
        "",
        &format!("{SAVE}    uint64_t item = 0xAAAA;\n    printf(\"show %#llx\\n\", U(vcall(d, 3, 0, 0, 0)));\n    printf(\"getresult %#llx item %#llx\\n\", U(vcall(d, 20, P(&item), 0, 0)), U(item));\n"),
        &[("FLE_FILEDLG_PICK", "C:\\a.flp")],
    );
    assert_eq!(rc, 0, "{l:?}");
    // HRESULT_FROM_WIN32(ERROR_CANCELLED) = 0x800704C7 (IModalWindow::Show page); GetResult before a successful Show fails with E_UNEXPECTED 0x8000FFFF (S-dlg1-fail-code) and writes NULL
    assert_eq!(l, ["show 0x800704c7", "getresult 0x8000ffff item 0"]);
}

#[test]
fn the_default_extension_is_added_to_a_name_without_one_with_or_without_the_leading_dot() {
    for (tag, ext, target, want) in [("a", "wav", "C:\\Users\\user\\Documents\\song", "C:\\Users\\user\\Documents\\song.wav"), ("b", ".wav", "C:\\Users\\user\\Documents\\song", "C:\\Users\\user\\Documents\\song.wav"), ("c", "wav", "C:\\Users\\user\\Documents\\song.mp3", "C:\\Users\\user\\Documents\\song.mp3")] {
        let main = format!(
            r#"{SAVE}
    vol_add("C:\\Users\\user\\Documents", 0x10);
    uint64_t item = 0, name = 0;
    uint16_t *ext = W("{ext}");
    printf("setext %#llx\n", U(vcall(d, 22, P(ext), 0, 0)));
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("filesyspath %#llx\n", U(vcall(item, 5, 0x80058000u, P(&name), 0)));
{p}"#,
            p = wstr_print("name")
        );
        let (rc, l) = run_env(&format!("exp1-defext-{tag}"), 2, "", &main, &[("FLE_FILEDLG_SAVE", target)]);
        assert_eq!(rc, 0, "{l:?}");
        // the SetDefaultExtension page: the extension is appended to a name typed without one; a name that has one is left alone (S-exp1-defext: the form with the dot is accepted too)
        assert_eq!(l, ["setext 0".to_string(), "show 0".to_string(), "getresult 0".to_string(), "filesyspath 0".to_string(), format!("  {want}")]);
    }
}

#[test]
fn a_folder_that_does_not_exist_or_a_directory_target_stops_by_name() {
    let main = format!("{SAVE}    vol_add(\"C:\\\\Users\\\\user\", 0x10);\n    vol_add(\"C:\\\\Users\\\\user\\\\Documents\", 0x10);\n    vcall(d, 3, 0, 0, 0);\n    printf(\"not reached\\n\");\n");
    let (rc, l) = run_env("exp1-nofolder", 2, "", &main, &[("FLE_FILEDLG_SAVE", "C:\\Nowhere\\song.wav")]);
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP IModalWindow::Show: FOS_PATHMUSTEXIST: the folder of the Save target"), "{l:?}");
    let (rc, l) = run_env("exp1-dirtarget", 2, "", &main, &[("FLE_FILEDLG_SAVE", "C:\\Users\\user\\Documents")]);
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP IModalWindow::Show: the Save target the harness gave is an existing directory"), "{l:?}");
    let (rc, l) = run_env("exp1-relative", 2, "", &main, &[("FLE_FILEDLG_SAVE", "song.wav")]);
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP IModalWindow::Show: the Save target the harness gave is not a drive-absolute guest path"), "{l:?}");
}

#[test]
fn an_existing_target_is_taken_as_the_users_confirmed_overwrite_and_the_model_says_so() {
    let main = format!("{SAVE}    vol_add(\"C:\\\\Users\\\\user\\\\Documents\", 0x10);\n    vol_add(\"C:\\\\Users\\\\user\\\\Documents\\\\old.wav\", 0x20);\n    printf(\"show %#llx\\n\", U(vcall(d, 3, 0, 0, 0)));\n");
    let dir = work();
    let _ = dir;
    let (rc, l) = run_env("exp1-overwrite", 2, "", &main, &[("FLE_FILEDLG_SAVE", "C:\\Users\\user\\Documents\\old.wav")]);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0"]);
}

#[test]
fn the_five_methods_of_the_save_interface_stop_by_name() {
    for (k, m) in SAVE_METHODS.iter().enumerate() {
        let (rc, l) = run("exp1-savemethod", 2, &format!("{SAVE}    vcall(d, {}, 0, 0, 0);\n    printf(\"not reached\\n\");\n", 27 + k));
        assert_eq!(rc, 3, "{l:?}");
        assert!(l[0].starts_with(&format!("STOP IFileSaveDialog::{m}: not modelled")), "{l:?}");
    }
}

#[test]
fn the_layer_4_rows_file_holds_only_the_new_rows_to_be_appended_after_decision_0178s_file() {
    // harness/seeds/exp1-imports.txt = the five Save rows + the wall rows of the export, and nothing of dlg1-imports.txt (it is appended AFTER that file, which stays first so that its pseudo-VAs do not move)
    let old = std::fs::read_to_string(manifest().join("../../harness/seeds/dlg1-imports.txt")).unwrap();
    let new = std::fs::read_to_string(manifest().join("../../harness/seeds/exp1-imports.txt")).unwrap();
    let rows = |t: &str| -> Vec<String> { t.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(|l| l.to_string()).collect() };
    let mut want: Vec<String> = SAVE_METHODS.iter().map(|m| format!("shell32.dll IFileSaveDialog::{m} fle_imp_IFileSaveDialog_{m}")).collect();
    // the walls the export reached (runtime/fle_wall_exp1.c), after the save rows
    want.extend(["kernel32.dll SetThreadExecutionState fle_imp_SetThreadExecutionState"].iter().map(|s| s.to_string()));
    assert_eq!(rows(&new), want);
    for r in rows(&new) {
        assert!(!rows(&old).contains(&r), "{r} is already a row of dlg1-imports.txt");
    }
    let h = std::fs::read_to_string(rt().join("fle_filedlg.h")).unwrap();
    for m in SAVE_METHODS {
        assert!(h.contains(&format!("X({m})")), "{m} must be in FLE_FILEDLG_FOR_EACH_SAVE_METHOD");
    }
    let w = std::fs::read_to_string(rt().join("fle_wall_exp1.h")).unwrap();
    assert!(w.contains("void fle_imp_SetThreadExecutionState(void);"));
}

/// Like `run`, with extra file-scope C text between the preamble and `main`.
fn run_with_globals(name: &str, rows: u8, globals: &str, main: &str) -> (i32, Vec<String>) {
    run_env(name, rows, globals, main, &[])
}

/// Like `run_with_globals`, with environment variables for the program.
fn run_env(name: &str, rows: u8, globals: &str, main: &str, envs: &[(&str, &str)]) -> (i32, Vec<String>) {
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
    std::fs::write(&c, format!("{}{hook}\n{globals}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n", pre(rows))).unwrap();
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
fn remove_control_item_on_an_unknown_control_or_item_answers_e_invalidarg_instead_of_stopping() {
    // S-exp1-removeitem-unknown: the engine's second dialog of a session calls RemoveControlItem(1900, 1901) on a customize interface that has no control 1900 yet. The pages name no HRESULT, so the model answers
    // E_INVALIDARG (0x80070057, Common HRESULT Values); an existing control and item are removed with S_OK as before (the slot of RemoveControlItem is 20, AddMenu 4, AddControlItem 19: shobjidl.idl order)
    let main = format!(
        r#"{DLG}
    uint64_t c = 0;
    vcall(d, 0, P(IID_Customize), P(&c), 0);
    printf("unknown control %#llx\n", U(vcall(c, 20, 1900, 1901, 0)));
    uint16_t *label = W("Favorites"), *item = W("Add this folder to favorites");
    printf("add menu %#llx\n", U(vcall(c, 4, 1900, P(label), 0)));
    printf("unknown item %#llx\n", U(vcall(c, 20, 1900, 1901, 0)));
    printf("add item %#llx\n", U(vcall(c, 19, 1900, 1901, P(item))));
    printf("remove item %#llx\n", U(vcall(c, 20, 1900, 1901, 0)));
    printf("remove again %#llx\n", U(vcall(c, 20, 1900, 1901, 0)));
"#
    );
    let (rc, l) = run("exp1-removeitem", 2, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["unknown control 0x80070057", "add menu 0", "unknown item 0x80070057", "add item 0", "remove item 0", "remove again 0x80070057"]);
}

/// A small program around runtime/fle_wall_exp1.c (no dialog model): `body` calls `ste(flags)`.
fn run_ste(name: &str, body: &str) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let prog = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_wall_exp1.h"
uint64_t R[16];
void fle_stop(const char *api, const char *why) {{ printf("STOP %s: %s\n", api, why); fflush(stdout); exit(3); }}
static unsigned long long ste(uint32_t f) {{ R[0] = 0xdead; R[1] = f; fle_imp_SetThreadExecutionState(); return (unsigned long long)R[0]; }}
int main(void)
{{
{body}
    return 0;
}}
"#
    );
    std::fs::write(&c, prog).unwrap();
    let cc = Command::new("clang")
        .args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-parameter", "-fPIE", "-pie"])
        .arg("-I")
        .arg(rt())
        .arg(&c)
        .arg(rt().join("fle_wall_exp1.c"))
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
fn set_thread_execution_state_returns_the_previous_state_and_keeps_only_continuous_calls() {
    // Learn: success returns the previous state; without ES_CONTINUOUS the call only resets the idle timer (the state is not changed); ES_USER_PRESENT combined with other flags fails (NULL) and sets nothing.
    // ES_SYSTEM_REQUIRED 1, ES_DISPLAY_REQUIRED 2, ES_USER_PRESENT 4, ES_AWAYMODE_REQUIRED 0x40, ES_CONTINUOUS 0x80000000. The initial state is the model's choice S-exp1-ste-initial (ES_CONTINUOUS).
    let (rc, l) = run_ste(
        "ste",
        r#"
    printf("%#llx\n", ste(0x80000001u));
    printf("%#llx\n", ste(0x00000002u));
    printf("%#llx\n", ste(0x80000000u | 0x40u));
    printf("%#llx\n", ste(0x4u | 0x1u));
    printf("%#llx\n", ste(0x80000000u));
    printf("%#llx\n", ste(0x80000000u));
"#,
    );
    assert_eq!(rc, 0, "{l:?}");
    // 1: prev 0x80000000 (initial); 2: transient call, prev = 0x80000001 (unchanged); 3: prev 0x80000001; 4: fails -> 0; 5: prev = 0x80000040 (state not touched by the failed call); 6: prev 0x80000000
    assert_eq!(l, ["0x80000000", "0x80000001", "0x80000001", "0", "0x80000040", "0x80000000"]);
}

#[test]
fn set_thread_execution_state_stops_by_name_for_a_flag_or_a_combination_the_page_leaves_open() {
    let (rc, l) = run_ste("ste-flag", "    ste(0x100u);\n    printf(\"not reached\\n\");\n");
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP SetThreadExecutionState: a flag outside"), "{l:?}");
    let (rc, l) = run_ste("ste-away", "    ste(0x40u);\n    printf(\"not reached\\n\");\n");
    assert_eq!(rc, 3, "{l:?}");
    assert!(l[0].starts_with("STOP SetThreadExecutionState: ES_AWAYMODE_REQUIRED without ES_CONTINUOUS"), "{l:?}");
}
