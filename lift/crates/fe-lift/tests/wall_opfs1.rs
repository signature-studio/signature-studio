//! Tests of decision 0216 stage 2 (opfs1, PROVISIONAL): two additions to the browser branch of the file dialog model (lift/runtime/fle_filedlg.c, decision 0193):
//!   (1) the page's answer may carry `TYPE <n>`: the file type the person picked in the page's window; GetFileTypeIndex returns it after Show (before, it returned only what the engine itself had set);
//!   (2) a folder dialog (FOS_PICKFOLDERS) is no longer cancelled before it reaches the page: the answer's `D` line is the folder (made on the volume), `E` lines may put files in it.
//! The "page" is a C stub of `fle_filedlg_host_pick` written in each test (as in wall_fpick1.rs, whose helpers are copied below): THE ANSWERS ARE THE TEST'S INPUT (a simulation of the person at the page),
//! not an output of the engine, and no browser is involved. The real volume (fle_vfs.c) is linked. FLE_OPFS1_RT_DIR (a full copy of runtime/) replaces runtime/ for mutation runs.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt() -> PathBuf {
    match std::env::var_os("FLE_OPFS1_RT_DIR") {
        Some(d) => PathBuf::from(d),
        None => manifest().join("../../runtime"),
    }
}
fn work() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("opfs1");
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



const GUIDS_C: &str = r#"
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


"#;

const PRE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_com.h"
#include "fle_modules.h"
#include "fle_vfs.h"
#include "fle_callback.h"
#include "fle_filedlg.h"
uint64_t R[16];
#define U(x) ((unsigned long long)(x))
static unsigned task_allocs;
void fle_imp_CoTaskMemAlloc(void) { task_allocs++; R[0] = (uint64_t)(uintptr_t)calloc(1, R[1]); }
uint64_t fle_call_guest(uint64_t va, uint64_t a, uint64_t b, uint64_t c, uint64_t d) { (void)a; (void)b; (void)c; (void)d; return va == 0x3000 ? 0 : 1; }
static uint16_t *W(const char *s) { return fle_w_from_ascii(s); }
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static uint64_t call(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d) { R[0] = 0xdead; R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0]; }
static uint64_t stk7[32];
/* a call with up to 7 arguments the way the lifted code makes it: rcx rdx r8 r9 in R, the others at [rsp + 0x28 + 8 k] */
static uint64_t call7(void (*f)(void), uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t g, uint64_t h) {
    memset(stk7, 0, sizeof stk7); stk7[0] = 0xdead0000u; stk7[5] = e; stk7[6] = g; stk7[7] = h; R[4] = P(stk7);
    R[0] = 0xdead; R[1] = a; R[2] = b; R[8] = c; R[9] = d; f(); return R[0];
}
void fle_imp_CoCreateInstance(void);
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

void fle_imp_CoCreateInstance(void)
{
    uint64_t g = R[1], ppv = fle_arg(5); uint32_t hr;
    if (ppv == 0) fle_stop("CoCreateInstance", "ppv is NULL");
    fle_st(ppv, 8, 0);
    if (fle_filedlg_cocreate(g, R[2], (uint32_t)R[8], R[9], ppv, &hr)) { R[0] = hr; return; }
    R[0] = FLE_HR_REGDB_E_CLASSNOTREG;
}
static void show_path(const char *label, uint64_t wp) { const uint16_t *w = (const uint16_t *)(uintptr_t)wp; char b[300]; unsigned i; for (i = 0; w[i] && i < 299; i++) b[i] = (char)w[i]; b[i] = 0; printf("%s %s\n", label, b); }
static const uint8_t *bytes_of(const char *path, uint64_t *size) { uint16_t *w = W(path); const uint8_t *data = NULL; unsigned opens = 0; uint64_t wt = 0; *size = 0; if (!fle_vfs_file_state(w, size, &opens, &wt, &data)) return NULL; return data; }
"#;

fn rows_table() -> String {
    let names = row_names(true);
    let mut s = String::from("const fle_resp_entry fle_resp_table[] = {\n");
    for (k, n) in names.iter().enumerate() {
        let f = format!("fle_imp_{}", n.replace("::", "_"));
        s.push_str(&format!("    {{ \"shell32.dll\", \"{n}\", UINT64_C({:#x}), {f} }},\n", 0x7F00_0000u64 + 16 * (k as u64 + 1)));
    }
    s.push_str(&format!("}};\nconst uint64_t fle_resp_count = {};\n", names.len()));
    s
}

/// Compiles `PRE + rows + globals + main` with the real volume and fle_filedlg.c, runs it with `envs`, returns (exit code, stdout lines).
fn run_env(name: &str, globals: &str, main: &str, envs: &[(&str, &str)]) -> (i32, Vec<String>) {
    let dir = work();
    let u = etxt_common::uniq(name);
    let c = dir.join(format!("{u}.c"));
    let exe = dir.join(&u);
    let src = format!("{PRE}{GUIDS_C}{rows}\n{globals}\nint main(void)\n{{\n{main}\n    return 0;\n}}\n", rows = rows_table());
    std::fs::write(&c, src).unwrap();
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-Wall", "-Wextra", "-Wno-unused-function", "-Wno-unused-parameter", "-fPIE", "-pie"]);
    cmd.arg("-I").arg(rt()).arg(&c);
    for f in ["fle_vfs.c", "fle_reg.c", "fle_env.c", "fle_imports.c", "fle_modules.c", "fle_string.c", "fle_filedlg.c"] {
        cmd.arg(rt().join(f));
    }
    let cc = cmd.arg("-o").arg(&exe).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {name}: {}", String::from_utf8_lossy(&cc.stderr));
    let mut run = Command::new(&exe);
    for (k, v) in envs {
        run.env(k, v);
    }
    let r = run.output().expect("run failed");
    (r.status.code().unwrap_or(-1), String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect())
}
fn run(name: &str, globals: &str, main: &str) -> (i32, Vec<String>) {
    run_env(name, globals, main, &[])
}

const OPEN: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FOD, 0, IID_FOD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;
const SAVE: &str = r#"
    uint64_t d = 0;
    uint64_t hr = cocreate(CLSID_FSD, 0, IID_FSD, &d);
    if (hr != 0 || d == 0) { printf("cocreate failed %#llx\n", U(hr)); return 5; }
"#;

// ---- the C text of the "page" stubs ----
/// A page that answers with the text of the C string `ans` (set by the test before Show), prints the request it was given, and counts its calls.
const PAGE_TEXT: &str = r#"
static int page_calls;
static const char *ans;
const char *fle_filedlg_host_pick(const char *req)
{
    page_calls++;
    printf("REQUEST %s\n", req);
    return ans == NULL ? NULL : strdup(ans);
}
"#;

fn filters(n: usize) -> String {
    let mut s = String::new();
    for i in 1..=n {
        s.push_str(&format!("    {{ uint16_t *a = W(\"Type {i} (*.t{i})\"), *b = W(\"*.t{i}\"); specs[{k}] = P(a); specs[{k1}] = P(b); }}\n", k = 2 * (i - 1), k1 = 2 * (i - 1) + 1));
    }
    s
}

#[test]
fn the_file_type_the_page_reports_is_what_get_file_type_index_returns_after_show() {
    let n = 5;
    let main = format!(
        r#"{SAVE}
    uint64_t specs[{m}];
{f}
    uint32_t idx = 0;
    printf("settypes %#llx\n", U(vcall(d, 4, {n}, P(specs), 0)));
    printf("setindex %#llx\n", U(vcall(d, 5, 2, 0, 0)));
    vcall(d, 6, P(&idx), 0, 0); printf("index-before %u\n", idx);
    /* the page reports type 4 */
    ans = "OK\nTYPE\t4\nS\t0\t0\tC:\\Users\\user\\Downloads\\x\\Out.t4";
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    vcall(d, 6, P(&idx), 0, 0); printf("index-after %u\n", idx);
    /* a second Show without TYPE does not inherit it: the engine's own index (2) is what GetFileTypeIndex says... the model keeps d->type_index as the page set it (4): the engine sets its index again before every Show */
    vcall(d, 5, 3, 0, 0);
    ans = "OK\nS\t0\t0\tC:\\Users\\user\\Downloads\\x\\Out2.t3";
    printf("show2 %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    vcall(d, 6, P(&idx), 0, 0); printf("index-after2 %u\n", idx);
    /* an index outside the list is ignored */
    ans = "OK\nTYPE\t9\nS\t0\t0\tC:\\Users\\user\\Downloads\\x\\Out3.t3";
    printf("show3 %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    vcall(d, 6, P(&idx), 0, 0); printf("index-after3 %u\n", idx);
    ans = "OK\nTYPE\t0\nS\t0\t0\tC:\\Users\\user\\Downloads\\x\\Out4.t3";
    printf("show4 %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    vcall(d, 6, P(&idx), 0, 0); printf("index-after4 %u\n", idx);
    /* a cancelled dialog changes nothing */
    ans = "CANCEL";
    printf("show5 %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    vcall(d, 6, P(&idx), 0, 0); printf("index-after5 %u\n", idx);
"#,
        m = 2 * n,
        f = filters(n)
    );
    let (rc, l) = run("opfs1-type", PAGE_TEXT, &main);
    assert_eq!(rc, 0, "{l:?}");
    let rest: Vec<&String> = l.iter().filter(|x| !x.starts_with("REQUEST ")).collect();
    assert_eq!(
        rest,
        ["settypes 0", "setindex 0", "index-before 2", "show 0 calls 1", "index-after 4", "show2 0", "index-after2 3", "show3 0", "index-after3 3", "show4 0", "index-after4 3", "show5 0x800704c7", "index-after5 3"]
    );
}

#[test]
fn a_folder_dialog_reaches_the_page_and_the_d_line_is_the_folder_with_its_files_on_the_volume() {
    let main = format!(
        r#"{OPEN}
    printf("setoptions %#llx\n", U(vcall(d, 9, 0x20 | 0x800, 0, 0)));
    ans = "OK\nD\t0\t0\tC:\\Users\\user\\Downloads\\samples";
    {{ /* the page also puts one file of the folder on the volume (an E line); the block is the page's malloc */ }}
    uint64_t item = 0, name = 0;
    printf("show %#llx calls %d\n", U(vcall(d, 3, 0, 0, 0)), page_calls);
    printf("getresult %#llx\n", U(vcall(d, 20, P(&item), 0, 0)));
    printf("filesyspath %#llx\n", U(vcall(item, 5, 0x80058000u, P(&name), 0)));
    show_path("  folder", name);
    printf("attrs %#llx\n", U(call(fle_imp_GetFileAttributesW, P(W("C:\\Users\\user\\Downloads\\samples")), 0, 0, 0)));
"#
    );
    let (rc, l) = run("opfs1-folder", PAGE_TEXT, &main);
    assert_eq!(rc, 0, "{l:?}");
    let req = l.iter().find(|x| x.starts_with("REQUEST ")).expect("a folder dialog must reach the page now (decision 0193 cancelled it in the model)");
    assert!(req.contains("\"options\":2080"), "{req}");
    let rest: Vec<&String> = l.iter().filter(|x| !x.starts_with("REQUEST ")).collect();
    // FILE_ATTRIBUTE_DIRECTORY = 0x10
    assert_eq!(rest, ["setoptions 0", "show 0 calls 1", "getresult 0", "filesyspath 0", "  folder C:\\Users\\user\\Downloads\\samples", "attrs 0x10"]);
}

#[test]
fn a_folder_answer_with_files_puts_them_in_the_folder_and_a_wrong_kind_of_answer_cancels_without_stopping_the_engine() {
    let page = r#"
static int page_calls;
static const char *ans;
static int give_files;
const char *fle_filedlg_host_pick(const char *req)
{
    char *a = malloc(1024);
    page_calls++;
    (void)req;
    if (give_files) {
        uint8_t *d1 = malloc(3);
        memcpy(d1, "abc", 3);
        snprintf(a, 1024, "OK\nD\t0\t0\tC:\\Users\\user\\Downloads\\samples\nE\t%llu\t3\tC:\\Users\\user\\Downloads\\samples\\k.wav", (unsigned long long)(uintptr_t)d1);
    } else snprintf(a, 1024, "%s", ans);
    return a;
}
"#;
    let main = format!(
        r#"{OPEN}
    vcall(d, 9, 0x20 | 0x800, 0, 0);
    give_files = 1;
    printf("show %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    uint64_t sz; const uint8_t *b = bytes_of("C:\\Users\\user\\Downloads\\samples\\k.wav", &sz);
    printf("file %llu %.3s\n", U(sz), b ? (const char *)b : "(none)");
    give_files = 0;
    /* a file (R) in a folder dialog: cancelled, not a stop of the engine */
    {{ uint8_t *x = malloc(1); x[0] = 1; static char r[200]; snprintf(r, sizeof r, "OK\nR\t%llu\t1\tC:\\Users\\user\\Downloads\\f.txt", U(P(x))); ans = r; }}
    printf("show-r %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    /* a folder (D) in an Open dialog for files: cancelled */
    vcall(d, 9, 0x800 | 0x1000, 0, 0);
    ans = "OK\nD\t0\t0\tC:\\Users\\user\\Downloads\\samples2";
    printf("show-d %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    /* a folder path the volume does not take: cancelled */
    vcall(d, 9, 0x20 | 0x800, 0, 0);
    ans = "OK\nD\t0\t0\tD:\\nope";
    printf("show-bad %#llx\n", U(vcall(d, 3, 0, 0, 0)));
    printf("alive %d\n", page_calls);
"#
    );
    let (rc, l) = run("opfs1-folder-files", page, &main);
    assert_eq!(rc, 0, "{l:?}");
    assert_eq!(l, ["show 0", "file 3 abc", "show-r 0x800704c7", "show-d 0x800704c7", "show-bad 0x800704c7", "alive 4"]);
}

