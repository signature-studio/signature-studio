/* The common item dialog COM model (decision 0178, PROVISIONAL). See fle_filedlg.h for the purpose, the opt-in rule, the vtable orders and the host picker.
 *
 * Sources. Microsoft Learn: IModalWindow::Show (S_OK, HRESULT_FROM_WIN32(ERROR_CANCELLED) when the user cancels), IFileDialog and its methods (SetFileTypes: E_UNEXPECTED if called twice or with
 * FOS_PICKFOLDERS, E_INVALIDARG for a NULL array; SetFileTypeIndex / GetFileTypeIndex: one-based; GetResult / GetResults: only after Show returned a success code or during OnFileOk, otherwise they fail; GetResult
 * fails when several items were chosen; Advise: a cookie that Unadvise takes), FILEOPENDIALOGOPTIONS (FOS_PATHMUSTEXIST 'a default value', FOS_FILEMUSTEXIST 'a default value for the Open dialog'),
 * IFileDialogEvents::OnFileOk (S_OK accepts, S_FALSE refuses and the dialog stays open), IShellItem::GetDisplayName (CoTaskMemFree), GetAttributes (S_OK when the attributes match the mask exactly, S_FALSE
 * otherwise), Compare (S_OK same, S_FALSE different), IShellItemArray::GetItemAt (E_FAIL out of bounds), SHCreateItemFromParsingName, SHGetKnownFolderPath (the path without a trailing backslash, CoTaskMemFree;
 * E_INVALIDARG for an id not present), SIGDN values, SFGAO values. HRESULT values: Common HRESULT Values (S_OK, S_FALSE, E_NOINTERFACE 0x80004002, E_FAIL 0x80004005, E_UNEXPECTED, E_INVALIDARG), COM Error Codes
 * (CLASS_E_NOAGGREGATION 0x80040110), HRESULT_FROM_WIN32(ERROR_CANCELLED = 1223) = 0x800704C7 (System Error Codes + the macro). GUIDs, vtable orders and COMDLG_FILTERSPEC are the SDK headers' (shobjidl.h of
 * mingw-w64 and shobjidl.idl; knownfolders.h): they are not on the Learn pages. What the pages leave open is a choice named S-dlg1-N in docs/0178-filedialog-and-flp-open.md.
 *
 * Objects are host calloc blocks whose first 8 bytes are the address of a static vtable of response-table VAs (a guest `call [vtbl + 8*slot]` reaches fle_dispatch -> the fle_imp_ function of the row). */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "fle_test_env.h"
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_com.h"
#include "fle_modules.h"
#include "fle_vfs.h"
#include "fle_callback.h"
#include "fle_ole.h"
#include "fle_filedlg.h"

#define HR_E_NOINTERFACE 0x80004002u
#define HR_E_FAIL 0x80004005u
#define HR_CLASS_E_NOAGGREGATION 0x80040110u
#define HR_CANCELLED 0x800704C7u        /* HRESULT_FROM_WIN32(ERROR_CANCELLED) */
#define HR_FILE_NOT_FOUND 0x80070002u   /* HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND): S-dlg1-parse-notfound */
#define FOS_OVERWRITEPROMPT 0x2u
#define FOS_PICKFOLDERS 0x20u
#define FOS_ALLOWMULTISELECT 0x200u
#define FOS_FILEMUSTEXIST 0x1000u
#define FOS_PATHMUSTEXIST 0x800u
#define FOS_NOREADONLYRETURN 0x8000u
#define FDEOR_ACCEPT 1u   /* FDE_OVERWRITE_RESPONSE: FDEOR_DEFAULT 0, FDEOR_ACCEPT 1, FDEOR_REFUSE 2 */
#define FDEOR_REFUSE 2u
#define SIGDN_NORMALDISPLAY 0x00000000u
#define SIGDN_PARENTRELATIVEPARSING 0x80018001u
#define SIGDN_DESKTOPABSOLUTEPARSING 0x80028000u
#define SIGDN_PARENTRELATIVEEDITING 0x80031001u
#define SIGDN_DESKTOPABSOLUTEEDITING 0x8004c000u
#define SIGDN_FILESYSPATH 0x80058000u
#define SIGDN_URL 0x80068000u
#define SIGDN_PARENTRELATIVE 0x80080001u
#define SFGAO_STREAM 0x00400000u
#define SFGAO_FOLDER 0x20000000u
#define SFGAO_FILESYSTEM 0x40000000u
#define ATTR_DIRECTORY 0x10u
#define APPDATA_SUBDIR "\\AppData\\Roaming"        /* as fle_env.c, SHGetFolderPathW */
#define LOCALAPPDATA_SUBDIR "\\AppData\\Local"

extern const fle_resp_entry fle_resp_table[] __attribute__((weak));
extern const uint64_t fle_resp_count __attribute__((weak));

typedef struct { uint32_t d1; uint16_t d2, d3; uint8_t d4[8]; } guid_t;
static const guid_t CLSID_FileSaveDialog = { 0xc0b4e2f3, 0xba21, 0x4773, { 0x8d, 0xba, 0x33, 0x5e, 0xc9, 0x46, 0xeb, 0x8b } };
static const guid_t IID_IFileSaveDialog = { 0x84bccd23, 0x5fde, 0x4cdb, { 0xae, 0xa4, 0xaf, 0x64, 0xb8, 0x3d, 0x78, 0xab } };
static const guid_t CLSID_FileOpenDialog = { 0xdc1c5a9c, 0xe88a, 0x4dde, { 0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7 } };
static const guid_t IID_IUnknown = { 0x00000000, 0x0000, 0x0000, { 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46 } };
static const guid_t IID_IModalWindow = { 0xb4db1657, 0x70d7, 0x485e, { 0x8e, 0x3e, 0x6f, 0xcb, 0x5a, 0x5c, 0x18, 0x02 } };
static const guid_t IID_IFileDialog = { 0x42f85136, 0xdb7e, 0x439c, { 0x85, 0xf1, 0xe4, 0x07, 0x5d, 0x13, 0x5f, 0xc8 } };
static const guid_t IID_IFileOpenDialog = { 0xd57c7288, 0xd4ad, 0x4768, { 0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60 } };
static const guid_t IID_IShellItem = { 0x43826d1e, 0xe718, 0x42ee, { 0xbc, 0x55, 0xa1, 0xe2, 0x61, 0xc3, 0x7b, 0xfe } };
static const guid_t IID_IShellItemArray = { 0xb63ea76d, 0x1f85, 0x456f, { 0xa1, 0x9c, 0x48, 0x15, 0x9e, 0xfa, 0x85, 0x8b } };
static const guid_t IID_IFileDialogCustomize = { 0xe6fdd21a, 0x163f, 0x4975, { 0x9c, 0x8c, 0xa6, 0x9f, 0x1b, 0xa3, 0x70, 0x34 } };
static const guid_t FOLDERID_Desktop = { 0xb4bfcc3a, 0xdb2c, 0x424c, { 0xb0, 0x29, 0x7f, 0xe9, 0x9a, 0x87, 0xc6, 0x41 } };
static const guid_t FOLDERID_Documents = { 0xfdd39ad0, 0x238f, 0x46af, { 0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7 } };
static const guid_t FOLDERID_Downloads = { 0x374de290, 0x123f, 0x4565, { 0x91, 0x64, 0x39, 0xc4, 0x92, 0x5e, 0x46, 0x7b } };
static const guid_t FOLDERID_LocalAppData = { 0xf1b32785, 0x6fba, 0x4fcf, { 0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91 } };
static const guid_t FOLDERID_Profile = { 0x5e6c858f, 0x0e22, 0x4760, { 0x9a, 0xfe, 0xea, 0x33, 0x17, 0xb6, 0x71, 0x73 } };
static const guid_t FOLDERID_ProgramFiles = { 0x905e63b6, 0xc1bf, 0x494e, { 0xb2, 0x9c, 0x65, 0xb7, 0x32, 0xd3, 0xd2, 0x1a } };
static const guid_t FOLDERID_RoamingAppData = { 0x3eb685db, 0x65f9, 0x4cf6, { 0xa0, 0x3a, 0xe3, 0xef, 0x65, 0x72, 0x9f, 0x3d } };
/* Decision 0183 (wrp1, PROVISIONAL): the Program Files folders (Learn, KNOWNFOLDERID: GUIDs and default paths; for a 64-bit application on a 64-bit system ProgramFiles and ProgramFilesX64 are %SystemDrive%\Program Files,
 * ProgramFilesX86 is %SystemDrive%\Program Files (x86), the Common Files folders are below them). The Fruity Wrapper asked for ProgramFilesX64 at CreatePlugInstance. */
static const guid_t FOLDERID_ProgramFilesX64 = { 0x6d809377, 0x6af0, 0x444b, { 0x89, 0x57, 0xa3, 0x77, 0x3f, 0x02, 0x20, 0x0e } };
static const guid_t FOLDERID_ProgramFilesX86 = { 0x7c5a40ef, 0xa0fb, 0x4bfc, { 0x87, 0x4a, 0xc0, 0xf2, 0xe0, 0xb9, 0xfa, 0x8e } };
static const guid_t FOLDERID_ProgramFilesCommon = { 0xf7f1ed05, 0x9f6d, 0x47a2, { 0xaa, 0xae, 0x29, 0xd3, 0x17, 0xc6, 0xf0, 0x66 } };
static const guid_t FOLDERID_ProgramFilesCommonX64 = { 0x6365d5a7, 0x0f0d, 0x45e5, { 0x87, 0xf6, 0x0d, 0xa5, 0x6b, 0x6a, 0x4f, 0x7d } };
static const guid_t FOLDERID_ProgramFilesCommonX86 = { 0xde974d24, 0xd9c6, 0x4d3e, { 0xbf, 0x91, 0xf4, 0x45, 0x51, 0x20, 0xb9, 0x17 } };
/* FOLDERID_ProgramData {62AB5D82-FDC1-4DC3-A9DD-070D1D495D97} = CSIDL_COMMON_APPDATA (Learn KNOWNFOLDERID); its path is the one fle_env.c gives for that CSIDL (decision 0126 S-commonappdata: C:\\ProgramData, a choice without a source). Serum 2 asks for it (decision 0165). */
static const guid_t FOLDERID_ProgramData = { 0x62ab5d82, 0xfdc1, 0x4dc3, { 0xa9, 0xdd, 0x07, 0x0d, 0x1d, 0x49, 0x5d, 0x97 } };

static void guid_read(uint64_t a, guid_t *g)
{
    uint8_t b[16];
    unsigned i;

    for (i = 0; i < 16; i++) b[i] = (uint8_t)fle_ld(a + i, 1);
    memcpy(g, b, 16);
}
static int guid_eq(const guid_t *a, const guid_t *b) { return memcmp(a, b, sizeof *a) == 0; }
static void guid_text(const guid_t *g, char *out)
{
    snprintf(out, 40, "{%08X-%04X-%04X-%02X%02X-%02X%02X%02X%02X%02X%02X}", g->d1, g->d2, g->d3, g->d4[0], g->d4[1], g->d4[2], g->d4[3], g->d4[4], g->d4[5], g->d4[6], g->d4[7]);
}

static char *w2a(const uint16_t *w, char *buf, size_t cap)
{
    size_t i = 0;

    if (w == NULL) { snprintf(buf, cap, "(null)"); return buf; }
    for (; w[i] != 0 && i + 1 < cap; i++) buf[i] = w[i] < 0x80 ? (char)w[i] : '?';
    buf[i] = '\0';
    return buf;
}

/* ---- wide-string helpers ---- */
static uint16_t *wdup(const uint16_t *s, size_t n)
{
    uint16_t *d = malloc((n + 1) * sizeof *d);

    if (d == NULL) fle_stop("filedlg", "out of memory");
    if (n) memcpy(d, s, n * sizeof *d);
    d[n] = 0;
    return d;
}
static uint16_t *wdup_guest(const char *api, uint64_t g)
{
    size_t n;

    return g == 0 ? NULL : fle_w_read_alloc(api, g, 32768, &n);
}
static uint16_t *wdup_utf8(const char *s)
{
    size_t n = strlen(s), k = 0, i;
    uint16_t *d = malloc((n + 1) * sizeof *d);

    if (d == NULL) fle_stop("filedlg", "out of memory");
    for (i = 0; i < n;) {
        unsigned char c = (unsigned char)s[i];
        uint32_t cp;
        unsigned extra;

        if (c < 0x80) { cp = c; extra = 0; } else if ((c >> 5) == 6) { cp = c & 0x1f; extra = 1; } else if ((c >> 4) == 14) { cp = c & 0x0f; extra = 2; } else if ((c >> 3) == 30) { cp = c & 7; extra = 3; } else fle_stop("filedlg", "the picker's path is not UTF-8");
        i++;
        while (extra--) { if (i >= n || ((unsigned char)s[i] & 0xc0) != 0x80) fle_stop("filedlg", "the picker's path is not UTF-8"); cp = (cp << 6) | ((unsigned char)s[i++] & 0x3f); }
        if (cp >= 0x10000) { cp -= 0x10000; d[k++] = (uint16_t)(0xd800 + (cp >> 10)); d[k++] = (uint16_t)(0xdc00 + (cp & 0x3ff)); } else d[k++] = (uint16_t)cp;
    }
    d[k] = 0;
    return d;
}

/* ---- the virtual volume: GetFileAttributesW through the guest-facing stub ---- */
static uint32_t vfs_attrs(const uint16_t *w)
{
    uint64_t save[6] = { R[0], R[1], R[2], R[8], R[9], R[4] };
    uint32_t a;

    R[1] = (uint64_t)(uintptr_t)w;
    fle_imp_GetFileAttributesW();
    a = (uint32_t)R[0];
    R[0] = save[0]; R[1] = save[1]; R[2] = save[2]; R[8] = save[3]; R[9] = save[4]; R[4] = save[5];
    return a;
}

/* CoTaskMemAlloc through the guest-facing stub, so that the guest's CoTaskMemFree frees it */
static uint64_t task_alloc(size_t bytes)
{
    uint64_t save[3] = { R[0], R[1], R[4] }, p;

    R[1] = bytes;
    fle_imp_CoTaskMemAlloc();
    p = R[0];
    R[0] = save[0]; R[1] = save[1]; R[4] = save[2];
    if (p == 0) fle_stop("filedlg", "CoTaskMemAlloc returned NULL");
    return p;
}
static uint64_t task_wstr(const uint16_t *s, size_t n)
{
    uint64_t p = task_alloc((n + 1) * 2);

    fle_w_write(p, s, n + 1);
    return p;
}

/* ---- object model ---- */
enum { K_DIALOG = 1, K_ITEM = 2, K_ARRAY = 3, K_CUSTOM = 4 };
#define OBJ_MAGIC 0x444C4731u   /* 'DLG1' */
#define MAX_SINKS 4u

enum { C_MENU = 1, C_BUTTON, C_COMBO, C_RADIO, C_CHECK, C_EDIT, C_SEP, C_TEXT, C_GROUP, C_OPENDROP };
typedef struct { uint32_t id, state; uint16_t *label; } citem;
typedef struct {
    uint32_t id; int type; uint32_t state; uint16_t *label, *text; int checked, prominent;
    citem *items; unsigned nitems, cap; uint32_t selected; int has_selected;
} ctl;
#define CDCS_ENABLEDVISIBLE 3u   /* the default state of every control and item: 'enabled and visible' (the Add* pages) */

typedef struct obj {
    uint64_t vtbl;              /* must stay first: the guest reads it */
    uint32_t magic, kind, refs;
    struct obj *owner;          /* K_CUSTOM: the dialog whose IFileDialogCustomize interface this is (references are counted on the owner) */
    struct obj *custom;         /* K_DIALOG: its IFileDialogCustomize sub-object, created at the first QueryInterface (COM identity: the same pointer every time) */
    /* K_ITEM */
    uint16_t *path;             /* UTF-16 guest path, no trailing backslash except "C:\" */
    /* K_ARRAY */
    struct obj **items; unsigned nitems;
    /* K_DIALOG */
    uint32_t options, opt_set;
    int types_set; unsigned nfilters; uint16_t *fname[FLE_FILEDLG_MAX_FILTERS], *fspec[FLE_FILEDLG_MAX_FILTERS];
    uint32_t type_index;
    uint16_t *title, *ok_label, *name_label, *file_name, *default_ext;
    struct obj *folder, *default_folder;
    uint64_t sink[MAX_SINKS]; uint32_t cookie[MAX_SINKS];
    uint32_t next_cookie;
    unsigned nplaces;
    guid_t client_guid; int has_client_guid;
    int shown, result_ok;
    int save;                   /* K_DIALOG: 1 = created from CLSID_FileSaveDialog (decision 0188): IFileSaveDialog vtable, Save defaults */
    struct obj **results; unsigned nresults;
    ctl *ctls; unsigned nctls, ctl_cap; int group_depth;   /* IFileDialogCustomize controls */
} obj;

static unsigned live_objects;
unsigned fle_filedlg_live_objects(void) { return live_objects; }

static uint64_t vt_dialog[29], vt_item[8], vt_array[10], vt_custom[30], vt_save[32];
static int vt_ready, vt_save_ready;

static uint64_t row_va(const char *api, const char *name)
{
    uint64_t i;

    if (fle_resp_table != NULL)
        for (i = 0; i < fle_resp_count; i++)
            if (strcmp(fle_resp_table[i].dll, "shell32.dll") == 0 && strcmp(fle_resp_table[i].name, name) == 0) return fle_resp_table[i].va;
    fle_stop(api, "the response table has no row for a method of the file dialog model (decision 0178: build with harness/seeds/dlg1-imports.txt)");
}
static int rows_present(void)
{
    uint64_t i;

    if (fle_resp_table == NULL) return 0;
    for (i = 0; i < fle_resp_count; i++)
        if (strcmp(fle_resp_table[i].dll, "shell32.dll") == 0 && strcmp(fle_resp_table[i].name, "IFileOpenDialog::Show") == 0) return 1;
    return 0;
}
static void vt_init(const char *api)
{
    static const char *dm[] = { "Show", "SetFileTypes", "SetFileTypeIndex", "GetFileTypeIndex", "Advise", "Unadvise", "SetOptions", "GetOptions", "SetDefaultFolder", "SetFolder", "GetFolder",
        "GetCurrentSelection", "SetFileName", "GetFileName", "SetTitle", "SetOkButtonLabel", "SetFileNameLabel", "GetResult", "AddPlace", "SetDefaultExtension", "Close", "SetClientGuid",
        "ClearClientData", "SetFilter", "GetResults", "GetSelectedItems" };
    static const char *im[] = { "BindToHandler", "GetParent", "GetDisplayName", "GetAttributes", "Compare" };
    static const char *am[] = { "BindToHandler", "GetPropertyStore", "GetPropertyDescriptionList", "GetAttributes", "GetCount", "GetItemAt", "EnumItems" };
    char nm[96];
    unsigned i;

    if (vt_ready) return;
    static const char *cm[] = { "EnableOpenDropDown", "AddMenu", "AddPushButton", "AddComboBox", "AddRadioButtonList", "AddCheckButton", "AddEditBox", "AddSeparator", "AddText", "SetControlLabel",
        "GetControlState", "SetControlState", "GetEditBoxText", "SetEditBoxText", "GetCheckButtonState", "SetCheckButtonState", "AddControlItem", "RemoveControlItem", "RemoveAllControlItems",
        "GetControlItemState", "SetControlItemState", "GetSelectedControlItem", "SetSelectedControlItem", "StartVisualGroup", "EndVisualGroup", "MakeProminent", "SetControlItemText" };
    vt_dialog[0] = vt_item[0] = vt_array[0] = vt_custom[0] = row_va(api, "ShellObj::QueryInterface");
    vt_dialog[1] = vt_item[1] = vt_array[1] = vt_custom[1] = row_va(api, "ShellObj::AddRef");
    vt_dialog[2] = vt_item[2] = vt_array[2] = vt_custom[2] = row_va(api, "ShellObj::Release");
    for (i = 0; i < 26; i++) { snprintf(nm, sizeof nm, "IFileOpenDialog::%s", dm[i]); vt_dialog[3 + i] = row_va(api, nm); }
    for (i = 0; i < 5; i++) { snprintf(nm, sizeof nm, "IShellItem::%s", im[i]); vt_item[3 + i] = row_va(api, nm); }
    for (i = 0; i < 7; i++) { snprintf(nm, sizeof nm, "IShellItemArray::%s", am[i]); vt_array[3 + i] = row_va(api, nm); }
    for (i = 0; i < 27; i++) { snprintf(nm, sizeof nm, "IFileDialogCustomize::%s", cm[i]); vt_custom[3 + i] = row_va(api, nm); }
    vt_ready = 1;
}

static int save_rows_present(void)
{
    uint64_t i;

    if (fle_resp_table == NULL) return 0;
    for (i = 0; i < fle_resp_count; i++)
        if (strcmp(fle_resp_table[i].dll, "shell32.dll") == 0 && strcmp(fle_resp_table[i].name, "IFileSaveDialog::SetSaveAsItem") == 0) return 1;
    return 0;
}
/* IFileSaveDialog = the 27 slots of IFileDialog (IUnknown 0-2, Show 3, IFileDialog 4-26: the same rows as IFileOpenDialog) + SetSaveAsItem 27, SetProperties 28, SetCollectedProperties 29, GetProperties 30,
 * ApplyProperties 31 (decision 0188; tables/vtbl-order-check.txt) */
static void vt_save_init(const char *api)
{
    static const char *sm[] = { "SetSaveAsItem", "SetProperties", "SetCollectedProperties", "GetProperties", "ApplyProperties" };
    char nm[96];
    unsigned i;

    if (vt_save_ready) return;
    vt_init(api);
    memcpy(vt_save, vt_dialog, 27 * sizeof vt_save[0]);
    for (i = 0; i < 5; i++) { snprintf(nm, sizeof nm, "IFileSaveDialog::%s", sm[i]); vt_save[27 + i] = row_va(api, nm); }
    vt_save_ready = 1;
}

static obj *obj_new(const char *api, unsigned kind)
{
    obj *o;

    vt_init(api);
    o = calloc(1, sizeof *o);
    if (o == NULL) fle_stop(api, "out of memory");
    o->magic = OBJ_MAGIC;
    o->kind = kind;
    o->refs = 1;
    o->vtbl = (uint64_t)(uintptr_t)(kind == K_DIALOG ? vt_dialog : kind == K_ITEM ? vt_item : kind == K_CUSTOM ? vt_custom : vt_array);
    live_objects++;
    return o;
}

/* `this` of a method: it must be an object of the model with the expected kind */
static obj *self(const char *api, unsigned kind)
{
    obj *o = (obj *)(uintptr_t)R[1];

    if (o == NULL || o->magic != OBJ_MAGIC || o->kind != kind) fle_stop(api, "`this` is not an object of the file dialog model of the expected interface");
    return o;
}
/* an interface pointer that the guest passes in: it must be one of ours (the guest cannot implement IShellItem for this) */
static obj *foreign(const char *api, uint64_t p, unsigned kind)
{
    obj *o = (obj *)(uintptr_t)p;

    if (o == NULL || fle_ld(p + 8, 4) != OBJ_MAGIC || o->magic != OBJ_MAGIC || o->kind != kind) fle_stop(api, "the interface pointer is not an object of this model (a guest-implemented IShellItem is not supported)");
    return o;
}

static void com_release_guest(uint64_t o)
{
    uint64_t vt = fle_ld(o, 8), fn = fle_ld(vt + 16, 8);

    fle_call_guest(fn, o, 0, 0, 0);
}
static void com_addref_guest(uint64_t o)
{
    uint64_t vt = fle_ld(o, 8), fn = fle_ld(vt + 8, 8);

    fle_call_guest(fn, o, 0, 0, 0);
}

static void obj_free(obj *o);
static void obj_release_internal(obj *o)
{
    if (o == NULL) return;
    if (o->refs == 0) fle_stop("filedlg", "internal reference count underflow");
    if (--o->refs == 0) obj_free(o);
}
static void free_results(obj *d)
{
    unsigned i;

    for (i = 0; i < d->nresults; i++) obj_release_internal(d->results[i]);
    free(d->results);
    d->results = NULL;
    d->nresults = 0;
}
static void obj_free(obj *o)
{
    unsigned i;

    switch (o->kind) {
    case K_ITEM: free(o->path); break;
    case K_ARRAY: for (i = 0; i < o->nitems; i++) obj_release_internal(o->items[i]); free(o->items); break;
    case K_DIALOG:
        for (i = 0; i < o->nfilters; i++) { free(o->fname[i]); free(o->fspec[i]); }
        free(o->title); free(o->ok_label); free(o->name_label); free(o->file_name); free(o->default_ext);
        obj_release_internal(o->folder); obj_release_internal(o->default_folder);
        for (i = 0; i < MAX_SINKS; i++) if (o->sink[i] != 0) com_release_guest(o->sink[i]);
        free_results(o);
        if (o->custom != NULL) { o->custom->magic = 0; free(o->custom); o->custom = NULL; live_objects--; }
        for (i = 0; i < o->nctls; i++) {
            unsigned k;

            free(o->ctls[i].label); free(o->ctls[i].text);
            for (k = 0; k < o->ctls[i].nitems; k++) free(o->ctls[i].items[k].label);
            free(o->ctls[i].items);
        }
        free(o->ctls);
        break;
    case K_CUSTOM: fle_stop("filedlg", "internal error: a sub-object is freed with its owner");
    }
    o->magic = 0;
    free(o);
    live_objects--;
}

static obj *item_new(const char *api, const uint16_t *path, size_t n)
{
    obj *o = obj_new(api, K_ITEM);

    o->path = wdup(path, n);
    return o;
}
static const uint16_t *base_name(const uint16_t *p)
{
    const uint16_t *b = p, *q;

    for (q = p; *q; q++) if (*q == '\\') b = q + 1;
    return b;
}

/* ---- IUnknown (one set of rows for all three kinds) ---- */
static int iid_is_save(const guid_t *g, unsigned kind, int save)
{
    if (guid_eq(g, &IID_IUnknown)) return 1;
    if (kind == K_DIALOG) return guid_eq(g, &IID_IModalWindow) || guid_eq(g, &IID_IFileDialog) || guid_eq(g, save ? &IID_IFileSaveDialog : &IID_IFileOpenDialog);
    if (kind == K_ITEM) return guid_eq(g, &IID_IShellItem);
    return guid_eq(g, &IID_IShellItemArray);
}

void fle_imp_ShellObj_QueryInterface(void)
{
    static const char api[] = "ShellObj::QueryInterface";
    obj *o = (obj *)(uintptr_t)R[1];
    uint64_t riid = R[2], ppv = R[8];
    guid_t g;
    char t[40];

    if (o == NULL || o->magic != OBJ_MAGIC) fle_stop(api, "`this` is not an object of the file dialog model");
    if (riid == 0 || ppv == 0) fle_stop(api, "NULL riid or ppvObject: the page does not give the behaviour");
    guid_read(riid, &g);
    guid_text(&g, t);
    if (o->kind == K_CUSTOM) o = o->owner;   /* every interface of the dialog answers for the same object */
    if (o->kind == K_DIALOG && guid_eq(&g, &IID_IFileDialogCustomize)) {
        if (o->custom == NULL) {
            obj *c = obj_new(api, K_CUSTOM);

            c->owner = o;
            o->custom = c;
        }
        o->refs++;   /* the references of the sub-object are counted on the dialog */
        fle_st(ppv, 8, (uint64_t)(uintptr_t)o->custom);
        R[0] = FLE_HR_S_OK;
        return;
    }
    if (iid_is_save(&g, o->kind, o->save)) {
        o->refs++;
        fle_st(ppv, 8, (uint64_t)(uintptr_t)o);
        R[0] = FLE_HR_S_OK;
        return;
    }
    /* A real file dialog also answers IFileDialog2, IObjectWithSite, IOleWindow ... which this model does not have: an interface it cannot tell apart from 'the real object has it' stops by name, so
     * that E_NOINTERFACE is never a silent difference (S-dlg1-qi). */
    {
        char why[320];

        snprintf(why, sizeof why, "interface %s was asked of an object of kind %u: not modelled (IUnknown / IModalWindow / IFileDialog / IFileOpenDialog / IFileDialogCustomize for the dialog, IShellItem for an item, IShellItemArray for an array), and E_NOINTERFACE would hide it", t, o->kind);
        fle_stop(api, why);
    }
}
void fle_imp_ShellObj_AddRef(void)
{
    static const char api[] = "ShellObj::AddRef";
    obj *o = (obj *)(uintptr_t)R[1];

    if (o == NULL || o->magic != OBJ_MAGIC) fle_stop(api, "`this` is not an object of the file dialog model");
    if (o->kind == K_CUSTOM) o = o->owner;
    o->refs++;
    R[0] = o->refs;
}
void fle_imp_ShellObj_Release(void)
{
    static const char api[] = "ShellObj::Release";
    obj *o = (obj *)(uintptr_t)R[1];
    uint32_t r;

    if (o == NULL || o->magic != OBJ_MAGIC) fle_stop(api, "`this` is not an object of the file dialog model");
    if (o->kind == K_CUSTOM) o = o->owner;
    if (o->refs == 0) fle_stop(api, "no reference outstanding");
    r = --o->refs;
    if (r == 0) obj_free(o);
    R[0] = r;
}

/* ---- IFileDialogCustomize: the controls the guest adds to the dialog ----
 * The real dialog has this interface and the engine uses it (decision 0178: File -> Open... calls AddMenu first). The model keeps the controls (type, label, items, state, selection) so that the guest's
 * calls are answered as the pages say and the request that Show hands to the picker can show them; nothing is drawn (the dialog has no window here). Pages: IFileDialogCustomize::AddMenu / AddPushButton /
 * AddComboBox / AddRadioButtonList / AddCheckButton / AddEditBox / AddSeparator / AddText / SetControlLabel / Get-SetControlState / Get-SetEditBoxText / Get-SetCheckButtonState / AddControlItem /
 * RemoveControlItem / RemoveAllControlItems / Get-SetControlItemState / Get-SetSelectedControlItem / StartVisualGroup / EndVisualGroup / MakeProminent / SetControlItemText / EnableOpenDropDown: they all say
 * 'S_OK, otherwise an HRESULT error code' without naming one. What a page leaves open (an id that is already used or unknown, a call on a menu where the page forbids it, a selection nobody made) stops by
 * name (S-dlg1-custom); the default state of a control and an item is 'enabled and visible' (CDCS_ENABLEDVISIBLE = 3, the Add* pages' remark). */
static void set_str(uint16_t **slot, uint64_t g, const char *api);
static obj *cdlg(const char *api)
{
    obj *c = self(api, K_CUSTOM);

    return c->owner;
}
static ctl *ctl_find(obj *d, uint32_t id)
{
    unsigned i;

    for (i = 0; i < d->nctls; i++) if (d->ctls[i].id == id) return &d->ctls[i];
    return NULL;
}
static ctl *ctl_get(const char *api, obj *d, uint32_t id)
{
    ctl *c = ctl_find(d, id);
    char why[120];

    if (c == NULL) { snprintf(why, sizeof why, "control id %u does not exist: the page names no HRESULT for it (S-dlg1-custom)", id); fle_stop(api, why); }
    return c;
}
static ctl *ctl_add(const char *api, obj *d, uint32_t id, int type, uint64_t label_g)
{
    ctl *c;
    char why[160];

    if (ctl_find(d, id) != NULL) { snprintf(why, sizeof why, "control id %u is already used: the page names no HRESULT for it (S-dlg1-custom)", id); fle_stop(api, why); }
    if (d->nctls == d->ctl_cap) {
        d->ctl_cap = d->ctl_cap ? d->ctl_cap * 2 : 8;
        d->ctls = realloc(d->ctls, d->ctl_cap * sizeof *d->ctls);
        if (d->ctls == NULL) fle_stop(api, "out of memory");
    }
    c = &d->ctls[d->nctls++];
    memset(c, 0, sizeof *c);
    c->id = id; c->type = type; c->state = CDCS_ENABLEDVISIBLE;
    c->label = wdup_guest(api, label_g);
    return c;
}
static citem *item_find(ctl *c, uint32_t id)
{
    unsigned i;

    for (i = 0; i < c->nitems; i++) if (c->items[i].id == id) return &c->items[i];
    return NULL;
}
static citem *item_get(const char *api, ctl *c, uint32_t id)
{
    citem *it = item_find(c, id);
    char why[120];

    if (it == NULL) { snprintf(why, sizeof why, "item id %u does not exist in control %u: the page names no HRESULT for it (S-dlg1-custom)", id, c->id); fle_stop(api, why); }
    return it;
}
static int is_container(const ctl *c) { return c->type == C_MENU || c->type == C_COMBO || c->type == C_RADIO || c->type == C_OPENDROP; }

void fle_imp_IFileDialogCustomize_EnableOpenDropDown(void)
{
    ctl_add("IFileDialogCustomize::EnableOpenDropDown", cdlg("IFileDialogCustomize::EnableOpenDropDown"), (uint32_t)R[2], C_OPENDROP, 0);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_AddMenu(void) { ctl_add("IFileDialogCustomize::AddMenu", cdlg("IFileDialogCustomize::AddMenu"), (uint32_t)R[2], C_MENU, R[8]); R[0] = FLE_HR_S_OK; }
void fle_imp_IFileDialogCustomize_AddPushButton(void) { ctl_add("IFileDialogCustomize::AddPushButton", cdlg("IFileDialogCustomize::AddPushButton"), (uint32_t)R[2], C_BUTTON, R[8]); R[0] = FLE_HR_S_OK; }
void fle_imp_IFileDialogCustomize_AddComboBox(void) { ctl_add("IFileDialogCustomize::AddComboBox", cdlg("IFileDialogCustomize::AddComboBox"), (uint32_t)R[2], C_COMBO, 0); R[0] = FLE_HR_S_OK; }
void fle_imp_IFileDialogCustomize_AddRadioButtonList(void) { ctl_add("IFileDialogCustomize::AddRadioButtonList", cdlg("IFileDialogCustomize::AddRadioButtonList"), (uint32_t)R[2], C_RADIO, 0); R[0] = FLE_HR_S_OK; }
void fle_imp_IFileDialogCustomize_AddCheckButton(void)
{
    ctl *c = ctl_add("IFileDialogCustomize::AddCheckButton", cdlg("IFileDialogCustomize::AddCheckButton"), (uint32_t)R[2], C_CHECK, R[8]);

    c->checked = (uint32_t)R[9] != 0;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_AddEditBox(void)
{
    ctl *c = ctl_add("IFileDialogCustomize::AddEditBox", cdlg("IFileDialogCustomize::AddEditBox"), (uint32_t)R[2], C_EDIT, 0);

    c->text = wdup_guest("IFileDialogCustomize::AddEditBox", R[8]);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_AddSeparator(void) { ctl_add("IFileDialogCustomize::AddSeparator", cdlg("IFileDialogCustomize::AddSeparator"), (uint32_t)R[2], C_SEP, 0); R[0] = FLE_HR_S_OK; }
void fle_imp_IFileDialogCustomize_AddText(void)
{
    ctl *c = ctl_add("IFileDialogCustomize::AddText", cdlg("IFileDialogCustomize::AddText"), (uint32_t)R[2], C_TEXT, 0);

    c->text = wdup_guest("IFileDialogCustomize::AddText", R[8]);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetControlLabel(void)
{
    static const char api[] = "IFileDialogCustomize::SetControlLabel";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    set_str(&c->label, R[8], api);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_GetControlState(void)
{
    static const char api[] = "IFileDialogCustomize::GetControlState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    if (R[8] == 0) fle_stop(api, "pdwState is NULL: the page gives no behaviour");
    fle_st(R[8], 4, c->state);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetControlState(void)
{
    static const char api[] = "IFileDialogCustomize::SetControlState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    c->state = (uint32_t)R[8];
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_GetEditBoxText(void)
{
    static const char api[] = "IFileDialogCustomize::GetEditBoxText";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    const uint16_t *s;

    if (c->type != C_EDIT) fle_stop(api, "the control is not an edit box");
    if (R[8] == 0) fle_stop(api, "ppszText is NULL: the page gives no behaviour");
    s = c->text != NULL ? c->text : (const uint16_t *)u"";
    fle_st(R[8], 8, task_wstr(s, fle_w_len(s)));
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetEditBoxText(void)
{
    static const char api[] = "IFileDialogCustomize::SetEditBoxText";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    if (c->type != C_EDIT) fle_stop(api, "the control is not an edit box");
    set_str(&c->text, R[8], api);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_GetCheckButtonState(void)
{
    static const char api[] = "IFileDialogCustomize::GetCheckButtonState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    if (c->type != C_CHECK) fle_stop(api, "the control is not a check button");
    if (R[8] == 0) fle_stop(api, "pbChecked is NULL: the page gives no behaviour");
    fle_st(R[8], 4, c->checked ? 1u : 0u);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetCheckButtonState(void)
{
    static const char api[] = "IFileDialogCustomize::SetCheckButtonState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    if (c->type != C_CHECK) fle_stop(api, "the control is not a check button");
    c->checked = (uint32_t)R[8] != 0;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_AddControlItem(void)
{
    static const char api[] = "IFileDialogCustomize::AddControlItem";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    uint32_t id = (uint32_t)R[8];
    citem *it;

    if (!is_container(c)) fle_stop(api, "the control is not a container (menu, combo box, option button group, open drop-down): the page names no HRESULT for it");
    if (item_find(c, id) != NULL) fle_stop(api, "the item id is already used in the control: the page names no HRESULT for it (S-dlg1-custom)");
    if (c->nitems == c->cap) {
        c->cap = c->cap ? c->cap * 2 : 8;
        c->items = realloc(c->items, c->cap * sizeof *c->items);
        if (c->items == NULL) fle_stop(api, "out of memory");
    }
    it = &c->items[c->nitems++];
    it->id = id; it->state = CDCS_ENABLEDVISIBLE; it->label = wdup_guest(api, R[9]);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_RemoveControlItem(void)
{
    static const char api[] = "IFileDialogCustomize::RemoveControlItem";
    ctl *c;
    citem *it;
    unsigned k;

    /* Decision 0188 (S-exp1-removeitem-unknown): the engine's SECOND file dialog of a session (Export twice, or Open then Export) calls RemoveControlItem(1900, 1901) on its new dialog's customize interface BEFORE
     * it adds the menu 1900: the control does not exist there. The page names no HRESULT for an unknown control or item ('S_OK, otherwise an HRESULT error code'), so this is a choice: E_INVALIDARG
     * (0x80070057, 'one or more arguments are invalid': the id names nothing), printed on stderr. A real dialog also lacks the control (it is a new object), and the real engine goes on, so it must be able
     * to take a failure there; whether it ignores it or reacts to the code is what the run shows. */
    c = ctl_find(cdlg(api), (uint32_t)R[2]);
    it = c != NULL ? item_find(c, (uint32_t)R[8]) : NULL;
    if (c == NULL || it == NULL) {
        fprintf(stderr, "filedlg: RemoveControlItem(control %u, item %u): %s does not exist on this dialog: answering E_INVALIDARG (0x80070057), a choice (S-exp1-removeitem-unknown)\n", (unsigned)R[2], (unsigned)R[8], c == NULL ? "the control" : "the item");
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    k = (unsigned)(it - c->items);

    free(it->label);
    memmove(&c->items[k], &c->items[k + 1], (c->nitems - k - 1) * sizeof *c->items);
    c->nitems--;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_RemoveAllControlItems(void)
{
    static const char api[] = "IFileDialogCustomize::RemoveAllControlItems";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    unsigned k;

    for (k = 0; k < c->nitems; k++) free(c->items[k].label);
    c->nitems = 0;
    c->has_selected = 0;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_GetControlItemState(void)
{
    static const char api[] = "IFileDialogCustomize::GetControlItemState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    citem *it = item_get(api, c, (uint32_t)R[8]);

    if (R[9] == 0) fle_stop(api, "pdwState is NULL: the page gives no behaviour");
    fle_st(R[9], 4, it->state);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetControlItemState(void)
{
    static const char api[] = "IFileDialogCustomize::SetControlItemState";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    citem *it = item_get(api, c, (uint32_t)R[8]);

    it->state = (uint32_t)R[9];
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_GetSelectedControlItem(void)
{
    static const char api[] = "IFileDialogCustomize::GetSelectedControlItem";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    if (c->type == C_MENU) fle_stop(api, "the page: 'This method cannot be called on menus'");
    if (!is_container(c)) fle_stop(api, "the control is not a container: the page names no HRESULT for it");
    if (R[8] == 0) fle_stop(api, "pdwIDItem is NULL: the page gives no behaviour");
    /* S-dlg1-custom-sel: nobody chose (no host input): a combo box / open drop-down answers its first item (the drop-down 'takes the text of the first item', EnableOpenDropDown page); an option button
     * group with no selection stops by name */
    if (c->has_selected) fle_st(R[8], 4, c->selected);
    else if ((c->type == C_COMBO || c->type == C_OPENDROP) && c->nitems > 0) fle_st(R[8], 4, c->items[0].id);
    else fle_stop(api, "no item is selected and the page does not say what is returned (S-dlg1-custom-sel)");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetSelectedControlItem(void)
{
    static const char api[] = "IFileDialogCustomize::SetSelectedControlItem";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    (void)item_get(api, c, (uint32_t)R[8]);
    c->selected = (uint32_t)R[8];
    c->has_selected = 1;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_StartVisualGroup(void)
{
    obj *d = cdlg("IFileDialogCustomize::StartVisualGroup");

    ctl_add("IFileDialogCustomize::StartVisualGroup", d, (uint32_t)R[2], C_GROUP, R[8]);
    d->group_depth++;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_EndVisualGroup(void)
{
    obj *d = cdlg("IFileDialogCustomize::EndVisualGroup");

    if (d->group_depth == 0) fle_stop("IFileDialogCustomize::EndVisualGroup", "no visual group is open: the page names no HRESULT for it (S-dlg1-custom)");
    d->group_depth--;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_MakeProminent(void)
{
    static const char api[] = "IFileDialogCustomize::MakeProminent";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);

    c->prominent = 1;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileDialogCustomize_SetControlItemText(void)
{
    static const char api[] = "IFileDialogCustomize::SetControlItemText";
    ctl *c = ctl_get(api, cdlg(api), (uint32_t)R[2]);
    citem *it = item_get(api, c, (uint32_t)R[8]);

    set_str(&it->label, R[9], api);
    R[0] = FLE_HR_S_OK;
}

/* ---- the host picker ---- */
static fle_filedlg_picker picker;
void fle_filedlg_set_picker(fle_filedlg_picker p) { picker = p; }

/* Native harness picker: FLE_FILEDLG_PICK = guest path(s) separated by '|', or FLE_FILEDLG_PICK_FILE = a host text file with one guest path per line (a path with spaces does not survive the harness script's `setenv`).
 * It stands for the USER'S input; the line printed says so. */
static int env_picker(const fle_filedlg_request *rq, uint16_t ***paths, unsigned *n)
{
    const char *e = FLE_TEST_GETENV(rq->save ? "FLE_FILEDLG_SAVE" : "FLE_FILEDLG_PICK"), *pf = FLE_TEST_GETENV(rq->save ? "FLE_FILEDLG_SAVE_FILE" : "FLE_FILEDLG_PICK_FILE");   /* a Save dialog reads its own variables (decision 0188) */
    char *copy = NULL, *tok, *save = NULL;
    unsigned k = 0;
    uint16_t **v;

    if (pf != NULL && *pf != '\0') {
        FILE *f = fopen(pf, "rb");
        size_t cap = 65536, len;

        if (f == NULL) fle_stop("filedlg", "FLE_FILEDLG_PICK_FILE cannot be opened");
        copy = malloc(cap);
        if (copy == NULL) fle_stop("filedlg", "out of memory");
        len = fread(copy, 1, cap - 1, f);
        fclose(f);
        if (len == 0) fle_stop("filedlg", "FLE_FILEDLG_PICK_FILE is empty or not a readable file (the harness script's setenv keeps only 63 characters of a value: use a short path)");
        copy[len] = '\0';
        for (tok = copy; *tok; tok++) if (*tok == '\n' || *tok == '\r') *tok = '|';
        e = copy;
    } else if (e != NULL && *e != '\0') {
        copy = strdup(e);
    } else {
        return 0;
    }
    {
        char *work = strdup(e);

        v = calloc(64, sizeof *v);
        if (work == NULL || v == NULL) fle_stop("filedlg", "out of memory");
        for (tok = strtok_r(work, "|", &save); tok != NULL && k < 64; tok = strtok_r(NULL, "|", &save)) v[k++] = wdup_utf8(tok);
        fprintf(stderr, "filedlg: THE SELECTION WAS GIVEN BY THE HARNESS (%s), standing for the user's input; it is not the engine's output: %s\n", pf != NULL && *pf != '\0' ? (rq->save ? "FLE_FILEDLG_SAVE_FILE" : "FLE_FILEDLG_PICK_FILE") : (rq->save ? "FLE_FILEDLG_SAVE" : "FLE_FILEDLG_PICK"), e);
        free(work);
    }
    free(copy);
    *paths = v;
    *n = k;
    return k > 0;
}

/* ---- the browser picker (decision 0193, fpick1, PROVISIONAL; always compiled since decision 0243) ----
 * The user is the person at the page. Show() hands the request to the page through fle_filedlg_host_pick (an ASYNC import, JSPI: harness/web/filedlg/fle_filedlg_lib.js; the native test supplies a stub), the engine's
 * stack is suspended while the person chooses, and the answer comes back as text, one line per item: `<role> TAB <ptr> TAB <size> TAB <guest path, UTF-8>`, after a first line `OK` (or `CANCEL`, then the lines are ignored).
 * role R = a chosen file: its bytes were read into a malloc'd block (ptr, size) by the page's script and are put on the virtual volume here; E = an extra file of the same choice (a sibling of the project, put on
 * the volume, not part of the dialog's result); S = a Save target (no bytes yet; the engine writes it). The environment variables of the native harness (FLE_FILEDLG_PICK*, FLE_FILEDLG_SAVE*) are read first and win. */
#ifdef __EMSCRIPTEN__
extern const char *fle_filedlg_host_pick(const char *request_json);   /* the page's JS library (fle_filedlg_lib.js) */
#define FLE_HAS_HOST_PICK 1
#else
int fle_vfs_make_dirs(const uint16_t *wdir) __attribute__((weak));
int fle_vfs_put_file(const uint16_t *wpath, uint8_t *data, uint64_t size) __attribute__((weak));
int fle_vfs_file_state(const uint16_t *wpath, uint64_t *size, unsigned *opens, uint64_t *wt, const uint8_t **data) __attribute__((weak));
extern const char *fle_filedlg_host_pick(const char *request_json) __attribute__((weak));   /* a native link list may supply a stub (the tests); without it the harness environment answers */
#define FLE_HAS_HOST_PICK (fle_filedlg_host_pick != NULL)
#endif

static void json_str(FILE *f, const uint16_t *w)
{
    fputc('"', f);
    for (; *w; w++) {
        if (*w == '"' || *w == '\\') { fputc('\\', f); fputc((char)*w, f); }
        else if (*w < 0x20 || *w >= 0x7f) fprintf(f, "\\u%04x", *w);
        else fputc((char)*w, f);
    }
    fputc('"', f);
}
static void json_opt(FILE *f, const uint16_t *w)
{
    if (w == NULL) fputs("null", f); else json_str(f, w);
}

static char *utf16_to_utf8(const uint16_t *w, size_t *outlen)
{
    size_t n = fle_w_len(w), i, k = 0;
    char *d = malloc(n * 3 + 1);

    if (d == NULL) fle_stop("filedlg", "out of memory");
    for (i = 0; i < n; i++) {
        uint32_t cp = w[i];

        if (cp >= 0xd800 && cp < 0xdc00 && i + 1 < n && w[i + 1] >= 0xdc00 && w[i + 1] < 0xe000) { cp = 0x10000 + ((cp - 0xd800) << 10) + (w[i + 1] - 0xdc00); i++; }
        if (cp < 0x80) d[k++] = (char)cp;
        else if (cp < 0x800) { d[k++] = (char)(0xc0 | (cp >> 6)); d[k++] = (char)(0x80 | (cp & 0x3f)); }
        else if (cp < 0x10000) { d[k++] = (char)(0xe0 | (cp >> 12)); d[k++] = (char)(0x80 | ((cp >> 6) & 0x3f)); d[k++] = (char)(0x80 | (cp & 0x3f)); }
        else { d[k++] = (char)(0xf0 | (cp >> 18)); d[k++] = (char)(0x80 | ((cp >> 12) & 0x3f)); d[k++] = (char)(0x80 | ((cp >> 6) & 0x3f)); d[k++] = (char)(0x80 | (cp & 0x3f)); }
    }
    d[k] = 0;
    if (outlen != NULL) *outlen = k;
    return d;
}

/* The Save targets the person chose; fle_filedlg_saved_poll tells the page when the engine has finished writing one (no open handle, the size and the last-write time unchanged between two polls). */
#define MAX_SAVED 16u
static struct { uint16_t *path; uint64_t size, wt, rep_size, rep_wt; int seen, reported; } saved[MAX_SAVED];
static unsigned nsaved;
static char saved_line[1024];
static void saved_track(const uint16_t *w)
{
    unsigned i;
    uint16_t *c = wdup(w, fle_w_len(w));

    for (i = 0; i < nsaved; i++)
        if (fle_w_icmp(saved[i].path, fle_w_len(saved[i].path), c, fle_w_len(c)) == 0) { free(c); saved[i].seen = 0; return; }
    if (nsaved == MAX_SAVED) { free(saved[0].path); memmove(&saved[0], &saved[1], (MAX_SAVED - 1u) * sizeof saved[0]); nsaved--; }
    memset(&saved[nsaved], 0, sizeof saved[0]);
    saved[nsaved++].path = c;
}

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#define FPICK_EXPORT EMSCRIPTEN_KEEPALIVE
#else
#define FPICK_EXPORT
#endif
/* For the Worker, at the engine's idle: NULL, or a line `<size> TAB <ptr of the bytes> TAB <guest path, UTF-8>` for a Save target whose file the engine has finished writing (the bytes stay valid until the next
 * write to the file; the caller copies them at once). A file that is written again later is reported again. */
FPICK_EXPORT const char *fle_filedlg_saved_poll(void)
{
    unsigned i;

    for (i = 0; i < nsaved; i++) {
        uint64_t size = 0, wt = 0;
        unsigned opens = 0;
        const uint8_t *data = NULL;
        int same;

        if (!fle_vfs_file_state(saved[i].path, &size, &opens, &wt, &data) || opens != 0 || size == 0) { saved[i].seen = 0; continue; }
        same = saved[i].seen && saved[i].size == size && saved[i].wt == wt;
        saved[i].size = size; saved[i].wt = wt; saved[i].seen = 1;
        if (!same || (saved[i].reported && saved[i].rep_size == size && saved[i].rep_wt == wt)) continue;
        saved[i].reported = 1; saved[i].rep_size = size; saved[i].rep_wt = wt;
        {
            char *u = utf16_to_utf8(saved[i].path, NULL);

            snprintf(saved_line, sizeof saved_line, "%llu\t%llu\t%s", (unsigned long long)size, (unsigned long long)(uintptr_t)data, u);
            free(u);
        }
        return saved_line;
    }
    return NULL;
}

/* decision 0216 (opfs1, PROVISIONAL): the answer may also carry `TYPE TAB <n>` (the file type the person picked in the page's window, 1-based; GetFileTypeIndex returns it after Show) and, for a folder dialog
 * (FOS_PICKFOLDERS), a line `D TAB 0 TAB 0 TAB <guest folder>` (the folder, created on the volume here; E lines may put its files). An older page never sends either, an older bundle's JS library never forwards them. */
static uint32_t browser_type_index;
/* A path from the page is acceptable when the volume takes it; the page has sanitised it already (app/src/engine/filedlg.ts), this is the second check: drive-absolute, components valid. */
static int browser_picker(const fle_filedlg_request *rq, uint16_t ***paths, unsigned *n)
{
    char *buf = NULL, *ans, *copy, *line, *save = NULL;
    size_t len = 0;
    FILE *f;
    unsigned i, k = 0;
    uint16_t **v;
    int cancelled = 0;

    browser_type_index = 0;
    if (env_picker(rq, paths, n)) return 1;   /* the native harness's selection wins (and says so); with neither variable set the person is asked */
    f = open_memstream(&buf, &len);
    if (f == NULL) fle_stop("filedlg", "open_memstream failed");
    fprintf(f, "{\"save\":%d,\"options\":%u,\"filterIndex\":%u,\"title\":", rq->save ? 1 : 0, rq->options, rq->filter_index);
    json_opt(f, rq->title);
    fputs(",\"okLabel\":", f); json_opt(f, rq->ok_label);
    fputs(",\"fileName\":", f); json_opt(f, rq->file_name);
    fputs(",\"defaultExt\":", f); json_opt(f, rq->default_ext);
    fputs(",\"folder\":", f); json_opt(f, rq->folder);
    fputs(",\"filters\":[", f);
    for (i = 0; i < rq->nfilters; i++) {
        fprintf(f, "%s[", i ? "," : "");
        json_opt(f, rq->filter_name[i]);
        fputc(',', f);
        json_opt(f, rq->filter_spec[i]);
        fputc(']', f);
    }
    fputs("]}", f);
    fclose(f);
    ans = (char *)fle_filedlg_host_pick(buf);   /* suspends the engine until the person has answered */
    free(buf);
    if (ans == NULL) return 0;
    copy = ans;
    v = calloc(64, sizeof *v);
    if (v == NULL) fle_stop("filedlg", "out of memory");
    for (line = strtok_r(copy, "\n", &save); line != NULL; line = strtok_r(NULL, "\n", &save)) {
        char role, *p1, *p2, *p3, *end;
        unsigned long long ptr, size;
        uint16_t *w;
        int rc;

        if (strncmp(line, "CANCEL", 6) == 0) { cancelled = 1; break; }
        if (strcmp(line, "OK") == 0) continue;
        if (strncmp(line, "TYPE\t", 5) == 0) { unsigned long t = strtoul(line + 5, &end, 10); if (t >= 1 && t <= rq->nfilters) browser_type_index = (uint32_t)t; continue; }
        role = line[0];
        p1 = strchr(line, '\t');
        p2 = p1 != NULL ? strchr(p1 + 1, '\t') : NULL;
        p3 = p2 != NULL ? strchr(p2 + 1, '\t') : NULL;
        if ((role != 'R' && role != 'E' && role != 'S' && role != 'D') || p3 == NULL) { fprintf(stderr, "filedlg: the page's answer has a line this model does not know: cancelled\n"); cancelled = 1; break; }
        ptr = strtoull(p1 + 1, &end, 10);
        size = strtoull(p2 + 1, &end, 10);
        w = wdup_utf8(p3 + 1);
        if (role == 'D') {
            rc = (rq->options & FOS_PICKFOLDERS) ? fle_vfs_make_dirs(w) : -1;   /* the folder the person chose in the page: it exists on the volume from now on (its files, if any, come as E lines) */
            if (rc != 0) { fprintf(stderr, "filedlg: the folder the page chose is not one the volume takes (%d) or the dialog is not a folder dialog: cancelled\n", rc); free(w); cancelled = 1; break; }
        } else if (role == 'S') {
            size_t pn = (size_t)(base_name(w) - w);
            uint16_t *par = wdup(w, pn > 3 ? pn - 1 : pn);

            rc = fle_vfs_make_dirs(par);
            free(par);
            if (rc != 0 || fle_w_len(w) < 4 || base_name(w)[0] == 0) { fprintf(stderr, "filedlg: the Save target is not a path the volume takes (%d): cancelled\n", rc); free(w); cancelled = 1; break; }
            saved_track(w);
        } else if (role == 'R' && (rq->options & FOS_PICKFOLDERS)) {
            fprintf(stderr, "filedlg: the page answered a folder dialog with a file: cancelled\n"); free(w); if (ptr != 0) free((void *)(uintptr_t)ptr); cancelled = 1; break;
        } else {
            rc = fle_vfs_put_file(w, (uint8_t *)(uintptr_t)ptr, size);   /* the volume owns the block from here, also when it refuses */
            if (rc != 0) { fprintf(stderr, "filedlg: the volume did not take the chosen file (%d: 1 invalid path, 2 directory, 3 open, 4 over 256 MiB, 5 a directory component is a file): cancelled\n", rc); free(w); cancelled = 1; break; }
        }
        if (role == 'E') { free(w); continue; }
        if (k < 64) v[k++] = w; else free(w);
    }
    free(ans);
    if (cancelled || k == 0) {
        for (i = 0; i < k; i++) free(v[i]);
        free(v);
        return 0;
    }
    fprintf(stderr, "filedlg: THE %s WAS MADE BY THE PERSON AT THE PAGE (%u item%s); the file stays in this tab's memory, it is not uploaded\n", rq->save ? "SAVE TARGET" : "SELECTION", k, k == 1 ? "" : "s");
    *paths = v;
    *n = k;
    return 1;
}

/* ---- the dialog ---- */
void fle_imp_IFileOpenDialog_SetFileTypes(void)
{
    static const char api[] = "IFileDialog::SetFileTypes";
    obj *d = self(api, K_DIALOG);
    uint32_t n = (uint32_t)R[2];
    uint64_t spec = R[8];
    unsigned i;

    if (d->types_set) { R[0] = FLE_HR_E_UNEXPECTED; return; }
    if (d->options & FOS_PICKFOLDERS) { R[0] = FLE_HR_E_UNEXPECTED; return; }
    if (spec == 0) { R[0] = FLE_HR_E_INVALIDARG; return; }
    if (n > FLE_FILEDLG_MAX_FILTERS) fle_stop(api, "more file types than this model keeps (a limit of the model)");
    for (i = 0; i < n; i++) {
        d->fname[i] = wdup_guest(api, fle_ld(spec + 16u * i, 8));
        d->fspec[i] = wdup_guest(api, fle_ld(spec + 16u * i + 8, 8));
    }
    d->nfilters = n;
    d->types_set = 1;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetFileTypeIndex(void)
{
    obj *d = self("IFileDialog::SetFileTypeIndex", K_DIALOG);

    d->type_index = (uint32_t)R[2];
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetFileTypeIndex(void)
{
    static const char api[] = "IFileDialog::GetFileTypeIndex";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "piFileType is NULL: the page gives no behaviour");
    fle_st(R[2], 4, d->type_index != 0 ? d->type_index : 1u);   /* S-dlg1-fti: one-based; the first type is selected when none was set */
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_Advise(void)
{
    static const char api[] = "IFileDialog::Advise";
    obj *d = self(api, K_DIALOG);
    uint64_t sink = R[2], pck = R[8];
    unsigned i;

    if (sink == 0 || pck == 0) fle_stop(api, "NULL pfde or pdwCookie: the page gives no behaviour");
    for (i = 0; i < MAX_SINKS && d->sink[i] != 0; i++) {}
    if (i == MAX_SINKS) fle_stop(api, "more event handlers than this model keeps (a limit of the model)");
    com_addref_guest(sink);   /* S-dlg1-advise-ref: the dialog keeps its own reference to the handler, as COM practice for a stored interface pointer */
    d->sink[i] = sink;
    d->cookie[i] = ++d->next_cookie;
    fle_st(pck, 4, d->cookie[i]);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_Unadvise(void)
{
    static const char api[] = "IFileDialog::Unadvise";
    obj *d = self(api, K_DIALOG);
    uint32_t ck = (uint32_t)R[2];
    unsigned i;

    for (i = 0; i < MAX_SINKS; i++)
        if (d->sink[i] != 0 && d->cookie[i] == ck) {
            uint64_t s = d->sink[i];
            d->sink[i] = 0;
            com_release_guest(s);
            R[0] = FLE_HR_S_OK;
            return;
        }
    fle_stop(api, "a cookie that Advise did not hand out: the page gives no behaviour");
}
void fle_imp_IFileOpenDialog_SetOptions(void)
{
    obj *d = self("IFileDialog::SetOptions", K_DIALOG);

    d->options = (uint32_t)R[2];
    d->opt_set = 1;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetOptions(void)
{
    static const char api[] = "IFileDialog::GetOptions";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "pfos is NULL: the page gives no behaviour");
    /* S-dlg1-opt-default: a new Open dialog starts with FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST (the two flags the FILEOPENDIALOGOPTIONS page calls defaults); a real dialog may start with more */
    fle_st(R[2], 4, d->opt_set ? d->options : d->save ? (FOS_OVERWRITEPROMPT | FOS_NOREADONLYRETURN | FOS_PATHMUSTEXIST) : (FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST));   /* S-exp1-opt-default: Save = the three flags the FILEOPENDIALOGOPTIONS page calls defaults for the Save dialog (and for both) */
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetDefaultFolder(void)
{
    static const char api[] = "IFileDialog::SetDefaultFolder";
    obj *d = self(api, K_DIALOG);
    obj *it = foreign(api, R[2], K_ITEM);

    it->refs++;
    obj_release_internal(d->default_folder);
    d->default_folder = it;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetFolder(void)
{
    static const char api[] = "IFileDialog::SetFolder";
    obj *d = self(api, K_DIALOG);
    obj *it = foreign(api, R[2], K_ITEM);

    it->refs++;
    obj_release_internal(d->folder);
    d->folder = it;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetFolder(void)
{
    static const char api[] = "IFileDialog::GetFolder";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "ppsi is NULL: the page gives no behaviour");
    if (d->folder == NULL) fle_stop(api, "no folder was set and the page does not say which one a dialog would show (the user's last folder)");
    d->folder->refs++;
    fle_st(R[2], 8, (uint64_t)(uintptr_t)d->folder);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetCurrentSelection(void)
{
    fle_stop("IFileDialog::GetCurrentSelection", "not modelled: the model has no selection while a dialog is open (Show returns at once)");
}
static void set_str(uint16_t **slot, uint64_t g, const char *api)
{
    uint16_t *n = wdup_guest(api, g);

    free(*slot);
    *slot = n;
}
void fle_imp_IFileOpenDialog_SetFileName(void)
{
    obj *d = self("IFileDialog::SetFileName", K_DIALOG);

    set_str(&d->file_name, R[2], "IFileDialog::SetFileName");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetFileName(void)
{
    static const char api[] = "IFileDialog::GetFileName";
    obj *d = self(api, K_DIALOG);
    const uint16_t *nm;

    if (R[2] == 0) fle_stop(api, "pszName is NULL: the page gives no behaviour");
    /* S-dlg1-getfilename: after a successful Show the name of the (first) chosen item, else the name set by SetFileName, else an empty string */
    if (d->result_ok && d->nresults > 0) nm = base_name(d->results[0]->path); else nm = d->file_name != NULL ? d->file_name : (const uint16_t *)u"";
    fle_st(R[2], 8, task_wstr(nm, fle_w_len(nm)));
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetTitle(void)
{
    obj *d = self("IFileDialog::SetTitle", K_DIALOG);

    set_str(&d->title, R[2], "IFileDialog::SetTitle");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetOkButtonLabel(void)
{
    obj *d = self("IFileDialog::SetOkButtonLabel", K_DIALOG);

    set_str(&d->ok_label, R[2], "IFileDialog::SetOkButtonLabel");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetFileNameLabel(void)
{
    obj *d = self("IFileDialog::SetFileNameLabel", K_DIALOG);

    set_str(&d->name_label, R[2], "IFileDialog::SetFileNameLabel");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_SetDefaultExtension(void)
{
    obj *d = self("IFileDialog::SetDefaultExtension", K_DIALOG);

    set_str(&d->default_ext, R[2], "IFileDialog::SetDefaultExtension");
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_AddPlace(void)
{
    static const char api[] = "IFileDialog::AddPlace";
    obj *d = self(api, K_DIALOG);

    (void)foreign(api, R[2], K_ITEM);
    d->nplaces++;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_Close(void)
{
    fle_stop("IFileDialog::Close", "not modelled: Show never returns to the guest while a dialog is open, so there is no moment at which the guest could close it");
}
void fle_imp_IFileOpenDialog_SetClientGuid(void)
{
    static const char api[] = "IFileDialog::SetClientGuid";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "guid is NULL: the page gives no behaviour");
    guid_read(R[2], &d->client_guid);
    d->has_client_guid = 1;
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_ClearClientData(void)
{
    (void)self("IFileDialog::ClearClientData", K_DIALOG);
    R[0] = FLE_HR_S_OK;   /* the model persists nothing between dialogs */
}
void fle_imp_IFileOpenDialog_SetFilter(void)
{
    fle_stop("IFileDialog::SetFilter", "the page: 'SetFilter is no longer available for use as of Windows 7' and gives no return value");
}

/* Calls IFileDialogEvents::OnFileOk(pfd) of every handler; returns 0 when one of them refuses with S_FALSE. */
static int events_on_file_ok(obj *d)
{
    unsigned i;

    for (i = 0; i < MAX_SINKS; i++) {
        uint64_t s = d->sink[i], vt, fn;
        uint32_t hr;

        if (s == 0) continue;
        vt = fle_ld(s, 8);
        fn = fle_ld(vt + 8u * 3u, 8);   /* OnFileOk = slot 3 */
        d->refs++;                       /* the pfd parameter is a borrowed pointer; keep the dialog alive across the call */
        hr = (uint32_t)fle_call_guest(fn, s, (uint64_t)(uintptr_t)d, 0, 0);
        d->refs--;
        if (hr == FLE_HR_S_FALSE) return 0;
        /* S-dlg1-onfileok-notimpl (a GUESS of the shell's behaviour, not from a page): E_NOTIMPL (0x80004001, Common HRESULT Values) from a handler that does not implement the event is 'not handled' and the
         * dialog goes on, as with S_OK. The engine's handler answers it (decision 0178, trace). */
        if (hr == 0x80004001u) continue;
        if (hr != FLE_HR_S_OK) fle_stop("IFileDialogEvents::OnFileOk", "the handler returned neither S_OK, S_FALSE nor E_NOTIMPL: the page names no other value");
    }
    return 1;
}

/* A Save dialog's chosen path (decision 0188): the picker's text with the default extension added when the name has none (the SetDefaultExtension page: the extension is appended to a name the user typed
 * without one; whether a leading '.' is allowed is not on the page: both forms are accepted, S-exp1-defext), the folder must exist (FOS_PATHMUSTEXIST, a default of the Save dialog: a real dialog keeps
 * itself open, no user is there to choose again, so the model stops by name), and a name that is an existing directory is refused the same way. An existing FILE with FOS_OVERWRITEPROMPT: the real dialog asks
 * the user; the harness's choice of an existing path stands for 'the user chose it and confirmed the overwrite' and says so (S-exp1-overwrite; the page names IFileDialogEvents::OnOverwrite for the
 * application's own answer: the model does not call it, because the engine's handler is not known to implement it). Returns the (possibly reallocated) path. */
static uint16_t *save_target(const char *api, obj *d, uint16_t *w, uint32_t attrs_of_w, const fle_filedlg_request *rq)
{
    size_t n = fle_w_len(w);
    const uint16_t *b = base_name(w), *q;
    int has_dot = 0;

    if (w[0] == 0 || n < 3 || !((w[0] >= 'A' && w[0] <= 'Z') || (w[0] >= 'a' && w[0] <= 'z')) || w[1] != ':' || w[2] != '\\') fle_stop(api, "the Save target the harness gave is not a drive-absolute guest path (C:\\...)");
    for (q = b; *q; q++) if (*q == '.') has_dot = 1;
    if (!has_dot && d->default_ext != NULL && d->default_ext[0] != 0) {
        const uint16_t *e = d->default_ext;
        size_t en;
        uint16_t *nw;

        if (e[0] == '.') e++;
        en = fle_w_len(e);
        nw = malloc((n + 1 + en + 1) * sizeof *nw);
        if (nw == NULL) fle_stop(api, "out of memory");
        memcpy(nw, w, n * sizeof *nw);
        nw[n] = '.';
        memcpy(nw + n + 1, e, en * sizeof *nw);
        nw[n + 1 + en] = 0;
        free(w);
        w = nw;
        n = n + 1 + en;
        attrs_of_w = vfs_attrs(w);
    }
    {
        size_t pn = (size_t)(base_name(w) - w);
        uint16_t *par;
        uint32_t pa;

        par = wdup(w, pn > 3 ? pn - 1 : pn);
        pa = vfs_attrs(par);
        if (rq->options & FOS_PATHMUSTEXIST) {
            if (pa == 0xFFFFFFFFu || !(pa & ATTR_DIRECTORY)) {
                free(par);
                fle_stop(api, "FOS_PATHMUSTEXIST: the folder of the Save target the harness gave does not exist on the virtual volume (a real dialog keeps itself open)");
            }
        }
        free(par);
    }
    if (attrs_of_w != 0xFFFFFFFFu && (attrs_of_w & ATTR_DIRECTORY)) fle_stop(api, "the Save target the harness gave is an existing directory (a real dialog would open it)");
    if (attrs_of_w != 0xFFFFFFFFu) fprintf(stderr, "filedlg: the Save target exists on the virtual volume; with FOS_OVERWRITEPROMPT a real dialog asks the user: the harness's choice stands for 'the user confirmed the overwrite' (S-exp1-overwrite)\n");
    return w;
}

void fle_imp_IFileOpenDialog_Show(void)
{
    static const char api[] = "IModalWindow::Show";
    obj *d = self(api, K_DIALOG);
    fle_filedlg_request rq;
    uint16_t **paths = NULL;
    unsigned n = 0, i;
    int picked;

    memset(&rq, 0, sizeof rq);
    rq.save = d->save;
    rq.options = d->opt_set ? d->options : d->save ? (FOS_OVERWRITEPROMPT | FOS_NOREADONLYRETURN | FOS_PATHMUSTEXIST) : (FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST);
    rq.nfilters = d->nfilters;
    for (i = 0; i < d->nfilters; i++) { rq.filter_name[i] = d->fname[i]; rq.filter_spec[i] = d->fspec[i]; }
    rq.filter_index = d->type_index != 0 ? d->type_index : 1u;
    rq.title = d->title; rq.ok_label = d->ok_label; rq.file_name = d->file_name; rq.default_ext = d->default_ext;
    rq.folder = d->folder != NULL ? d->folder->path : d->default_folder != NULL ? d->default_folder->path : NULL;
    free_results(d);
    d->shown = 1;
    d->result_ok = 0;
    picked = picker != NULL ? picker(&rq, &paths, &n) : FLE_HAS_HOST_PICK ? browser_picker(&rq, &paths, &n) : env_picker(&rq, &paths, &n);
    if (picked && n != 0 && picker == NULL && browser_type_index != 0) { d->type_index = browser_type_index; }
    if (!picked || n == 0) {
        R[0] = HR_CANCELLED;
        return;
    }
    if (d->save && n != 1) fle_stop(api, "the picker returned other than one path for a Save dialog");
    if (n > 1 && !(rq.options & FOS_ALLOWMULTISELECT)) fle_stop(api, "the picker returned several items for a dialog without FOS_ALLOWMULTISELECT");
    d->results = calloc(n, sizeof *d->results);
    if (d->results == NULL) fle_stop(api, "out of memory");
    for (i = 0; i < n; i++) {
        uint32_t a = vfs_attrs(paths[i]);
        char pa[400], pb[200];

        if (d->save) {
            paths[i] = save_target(api, d, paths[i], a, &rq);
        } else if (rq.options & FOS_PICKFOLDERS) {
            if (a == 0xFFFFFFFFu || !(a & ATTR_DIRECTORY)) fle_stop(api, "FOS_PICKFOLDERS: the selection the harness gave is not a directory of the virtual volume");
        } else if (rq.options & FOS_FILEMUSTEXIST) {
            if (a == 0xFFFFFFFFu || (a & ATTR_DIRECTORY)) {
                snprintf(pa, sizeof pa, "FOS_FILEMUSTEXIST: the selection the harness gave is not a file of the virtual volume (a real dialog keeps itself open): %s", w2a(paths[i], pb, sizeof pb));
                fle_stop(api, pa);
            }
        }
        d->results[d->nresults++] = item_new(api, paths[i], fle_w_len(paths[i]));
        free(paths[i]);
    }
    free(paths);
    d->result_ok = 1;
    if (!events_on_file_ok(d)) {
        /* S_FALSE: 'the dialog should remain open'. No user is there to choose again: the model ends it as a cancel (S-dlg1-onfileok-refuse). */
        free_results(d);
        d->result_ok = 0;
        R[0] = HR_CANCELLED;
        return;
    }
    R[0] = FLE_HR_S_OK;
}

void fle_imp_IFileOpenDialog_GetResult(void)
{
    static const char api[] = "IFileDialog::GetResult";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "ppsi is NULL: the page gives no behaviour");
    fle_st(R[2], 8, 0);
    if (!d->result_ok) { R[0] = FLE_HR_E_UNEXPECTED; return; }   /* 'Calling this method at any other time will fail' (S-dlg1-fail-code: E_UNEXPECTED) */
    if (d->nresults != 1) { R[0] = HR_E_FAIL; return; }          /* 'If multiple items were chosen, this method will fail' (S-dlg1-fail-code: E_FAIL) */
    d->results[0]->refs++;
    fle_st(R[2], 8, (uint64_t)(uintptr_t)d->results[0]);
    R[0] = FLE_HR_S_OK;
}
static obj *array_of(const char *api, obj *d)
{
    obj *a = obj_new(api, K_ARRAY);
    unsigned i;

    a->items = calloc(d->nresults ? d->nresults : 1, sizeof *a->items);
    if (a->items == NULL) fle_stop(api, "out of memory");
    for (i = 0; i < d->nresults; i++) { a->items[i] = d->results[i]; d->results[i]->refs++; }
    a->nitems = d->nresults;
    return a;
}
void fle_imp_IFileOpenDialog_GetResults(void)
{
    static const char api[] = "IFileOpenDialog::GetResults";
    obj *d = self(api, K_DIALOG);

    if (R[2] == 0) fle_stop(api, "ppenum is NULL: the page gives no behaviour");
    fle_st(R[2], 8, 0);
    if (!d->result_ok) { R[0] = FLE_HR_E_UNEXPECTED; return; }
    fle_st(R[2], 8, (uint64_t)(uintptr_t)array_of(api, d));
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IFileOpenDialog_GetSelectedItems(void)
{
    fle_stop("IFileOpenDialog::GetSelectedItems", "not modelled: the model has no selection while a dialog is open");
}

/* ---- IFileSaveDialog's own methods (decision 0188): the engine's calls decide which are needed; the others stop by name ---- */
#define SAVE_STOP(m) void fle_imp_IFileSaveDialog_##m(void) { fle_stop("IFileSaveDialog::" #m, "not modelled: the engine has not been seen to call it (decision 0188)"); }
SAVE_STOP(SetSaveAsItem)
SAVE_STOP(SetProperties)
SAVE_STOP(SetCollectedProperties)
SAVE_STOP(GetProperties)
SAVE_STOP(ApplyProperties)

/* ---- CoCreateInstance ---- */
int fle_filedlg_cocreate(uint64_t rclsid, uint64_t punkouter, uint32_t clsctx, uint64_t riid, uint64_t ppv, uint32_t *hr)
{
    static const char api[] = "CoCreateInstance";
    guid_t c, i;
    char ct[40], it[40];
    obj *d;
    int is_save;

    (void)clsctx;
    if (rclsid == 0 || riid == 0) return 0;
    guid_read(rclsid, &c);
    guid_read(riid, &i);
    guid_text(&c, ct);
    guid_text(&i, it);
    is_save = guid_eq(&c, &CLSID_FileSaveDialog);
    if (!is_save && !guid_eq(&c, &CLSID_FileOpenDialog)) return 0;
    if (!rows_present()) return 0;
    if (is_save && !save_rows_present()) return 0;   /* a build with decision 0178's rows only: the Save class stays unregistered as before */
    if (punkouter != 0) { *hr = HR_CLASS_E_NOAGGREGATION; return 1; }
    if (!iid_is_save(&i, K_DIALOG, is_save)) fle_stop(api, is_save ? "CLSID_FileSaveDialog asked for an interface other than IUnknown / IModalWindow / IFileDialog / IFileSaveDialog: not modelled" : "CLSID_FileOpenDialog asked for an interface other than IUnknown / IModalWindow / IFileDialog / IFileOpenDialog: not modelled");
    d = obj_new(api, K_DIALOG);
    if (is_save) { vt_save_init(api); d->save = 1; d->vtbl = (uint64_t)(uintptr_t)vt_save; }
    fle_st(ppv, 8, (uint64_t)(uintptr_t)d);
    *hr = FLE_HR_S_OK;
    return 1;
}

/* ---- shell items ---- */
void fle_imp_IShellItem_BindToHandler(void) { fle_stop("IShellItem::BindToHandler", "not modelled: no handler (BHID_*) of the shell is implemented"); }
void fle_imp_IShellItem_GetParent(void)
{
    static const char api[] = "IShellItem::GetParent";
    obj *it = self(api, K_ITEM);
    const uint16_t *b = base_name(it->path);
    size_t n = (size_t)(b - it->path);

    if (R[2] == 0) fle_stop(api, "ppsi is NULL: the page gives no behaviour");
    if (n <= 3) fle_stop(api, "the item is a drive root: the parent is the shell's 'This PC', which is not modelled");
    fle_st(R[2], 8, (uint64_t)(uintptr_t)item_new(api, it->path, n - 1 == 2 ? n : n - 1));   /* "C:\a" -> "C:\"; "C:\a\b" -> "C:\a" */
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IShellItem_GetDisplayName(void)
{
    static const char api[] = "IShellItem::GetDisplayName";
    obj *it = self(api, K_ITEM);
    uint32_t sigdn = (uint32_t)R[2];
    const uint16_t *s;
    char url[600];
    uint16_t *tmp = NULL;
    uint64_t p;
    size_t n;

    if (R[8] == 0) fle_stop(api, "ppszName is NULL: the page gives no behaviour");
    switch (sigdn) {
    case SIGDN_FILESYSPATH:
    case SIGDN_DESKTOPABSOLUTEPARSING:
    case SIGDN_DESKTOPABSOLUTEEDITING:
        s = it->path; break;   /* S-dlg1-sigdn: a file system item's desktop-absolute parsing/editing name is its path (the page does not give the strings) */
    case SIGDN_NORMALDISPLAY:
    case SIGDN_PARENTRELATIVEPARSING:
    case SIGDN_PARENTRELATIVEEDITING:
    case SIGDN_PARENTRELATIVE:
        s = base_name(it->path); break;   /* S-dlg1-sigdn: the name relative to the parent is the file name; the file name extension is shown (the user's Explorer option 'hide extensions' is not modelled) */
    case SIGDN_URL: {
        size_t i, k = 0;
        char a[300];
        w2a(it->path, a, sizeof a);
        k += (size_t)snprintf(url, sizeof url, "file:///");
        for (i = 0; a[i] && k + 4 < sizeof url; i++) url[k++] = a[i] == '\\' ? '/' : a[i];
        url[k] = '\0';
        tmp = wdup_utf8(url);
        s = tmp;
        break;   /* S-dlg1-sigdn: 'file:///C:/dir/name' without percent-encoding */
    }
    default:
        fle_stop(api, "a SIGDN value that the model does not produce (the page does not give the strings)");
    }
    n = fle_w_len(s);
    p = task_wstr(s, n);
    free(tmp);
    fle_st(R[8], 8, p);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IShellItem_GetAttributes(void)
{
    static const char api[] = "IShellItem::GetAttributes";
    obj *it = self(api, K_ITEM);
    uint32_t mask = (uint32_t)R[2], have, a;

    if (R[8] == 0) fle_stop(api, "psfgaoAttribs is NULL: the page gives no behaviour");
    a = vfs_attrs(it->path);
    have = SFGAO_FILESYSTEM | ((a != 0xFFFFFFFFu && (a & ATTR_DIRECTORY)) ? SFGAO_FOLDER : SFGAO_STREAM);
    if (mask & ~(SFGAO_FILESYSTEM | SFGAO_FOLDER | SFGAO_STREAM)) fle_stop(api, "an SFGAO bit that the model does not answer (S-dlg1-sfgao: FILESYSTEM, FOLDER and STREAM only)");
    fle_st(R[8], 4, have & mask);
    R[0] = (have & mask) == mask ? FLE_HR_S_OK : FLE_HR_S_FALSE;
}
void fle_imp_IShellItem_Compare(void)
{
    static const char api[] = "IShellItem::Compare";
    obj *a = self(api, K_ITEM), *b = foreign(api, R[2], K_ITEM);
    int same;

    if (R[9] == 0) fle_stop(api, "piOrder is NULL: the page gives no behaviour");
    same = fle_w_icmp(a->path, fle_w_len(a->path), b->path, fle_w_len(b->path)) == 0;
    fle_st(R[9], 4, same ? 0u : 1u);
    R[0] = same ? FLE_HR_S_OK : FLE_HR_S_FALSE;
}

/* ---- shell item arrays ---- */
void fle_imp_IShellItemArray_BindToHandler(void) { fle_stop("IShellItemArray::BindToHandler", "not modelled"); }
void fle_imp_IShellItemArray_GetPropertyStore(void) { fle_stop("IShellItemArray::GetPropertyStore", "not modelled"); }
void fle_imp_IShellItemArray_GetPropertyDescriptionList(void) { fle_stop("IShellItemArray::GetPropertyDescriptionList", "not modelled"); }
void fle_imp_IShellItemArray_GetAttributes(void) { fle_stop("IShellItemArray::GetAttributes", "not modelled"); }
void fle_imp_IShellItemArray_GetCount(void)
{
    static const char api[] = "IShellItemArray::GetCount";
    obj *a = self(api, K_ARRAY);

    if (R[2] == 0) fle_stop(api, "pdwNumItems is NULL: the page gives no behaviour");
    fle_st(R[2], 4, a->nitems);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IShellItemArray_GetItemAt(void)
{
    static const char api[] = "IShellItemArray::GetItemAt";
    obj *a = self(api, K_ARRAY);
    uint32_t idx = (uint32_t)R[2];

    if (R[8] == 0) fle_stop(api, "ppsi is NULL: the page gives no behaviour");
    fle_st(R[8], 8, 0);
    if (idx >= a->nitems) { R[0] = HR_E_FAIL; return; }   /* the page: 'returns E_FAIL if the requested index is out of bounds' */
    a->items[idx]->refs++;
    fle_st(R[8], 8, (uint64_t)(uintptr_t)a->items[idx]);
    R[0] = FLE_HR_S_OK;
}
void fle_imp_IShellItemArray_EnumItems(void) { fle_stop("IShellItemArray::EnumItems", "not modelled"); }

/* ---- shell32 functions ---- */
void fle_imp_SHCreateItemFromParsingName(void)
{
    static const char api[] = "SHCreateItemFromParsingName";
    uint64_t pw = R[1], pbc = R[2], riid = R[8], ppv = R[9];
    uint16_t *w;
    size_t n;
    guid_t g;
    uint32_t a;

    if (pw == 0 || riid == 0 || ppv == 0) fle_stop(api, "NULL pszPath / riid / ppv: the page gives no behaviour");
    if (pbc != 0) fle_stop(api, "a bind context was passed: not modelled (the page leaves its effect to the data source)");
    fle_st(ppv, 8, 0);
    w = fle_w_read_alloc(api, pw, 32768, &n);
    guid_read(riid, &g);
    if (!guid_eq(&g, &IID_IShellItem)) fle_stop(api, "an interface other than IShellItem was asked (IShellItem2 is not modelled)");
    if (n < 3 || !((w[0] >= 'A' && w[0] <= 'Z') || (w[0] >= 'a' && w[0] <= 'z')) || w[1] != ':' || w[2] != '\\') fle_stop(api, "a parsing name that is not a drive-absolute file system path: the shell namespace is not modelled");
    while (n > 3 && w[n - 1] == '\\') w[--n] = 0;
    a = vfs_attrs(w);
    if (a == 0xFFFFFFFFu) {
        /* S-dlg1-parse-notfound (a guess of the real behaviour, not from a page): a path that does not exist gives HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) */
        free(w);
        R[0] = HR_FILE_NOT_FOUND;
        return;
    }
    fle_st(ppv, 8, (uint64_t)(uintptr_t)item_new(api, w, n));
    free(w);
    R[0] = FLE_HR_S_OK;
}

void fle_imp_SHGetKnownFolderPath(void)
{
    static const char api[] = "SHGetKnownFolderPath";
    uint64_t prfid = R[1], pp = R[9];
    uint32_t flags = (uint32_t)R[2];
    guid_t g;
    const char *text = NULL;
    char gt[40];
    uint16_t *w;

    if (prfid == 0 || pp == 0) fle_stop(api, "NULL rfid / ppszPath: the page gives no behaviour");
    if (R[8] != 0) fle_stop(api, "hToken is not NULL: another user's folders are not modelled");
    if (flags != 0) fle_stop(api, "KNOWN_FOLDER_FLAG values are not modelled (dwFlags must be 0)");
    guid_read(prfid, &g);
    guid_text(&g, gt);
    /* S-dlg1-kf: the same virtual profile as SHGetFolderPathW (fle_env.c, decision 0126): C:\Users\user; Documents = Profile\Documents; Desktop and Downloads below it (a choice, as there) */
    if (guid_eq(&g, &FOLDERID_Profile)) text = FLE_VIRTUAL_USER_PROFILE_DIR;
    else if (guid_eq(&g, &FOLDERID_Documents)) text = FLE_VIRTUAL_USER_PROFILE_DIR "\\Documents";
    else if (guid_eq(&g, &FOLDERID_Desktop)) text = FLE_VIRTUAL_USER_PROFILE_DIR "\\Desktop";
    else if (guid_eq(&g, &FOLDERID_Downloads)) text = FLE_VIRTUAL_USER_PROFILE_DIR "\\Downloads";
    else if (guid_eq(&g, &FOLDERID_RoamingAppData)) text = FLE_VIRTUAL_USER_PROFILE_DIR APPDATA_SUBDIR;
    else if (guid_eq(&g, &FOLDERID_LocalAppData)) text = FLE_VIRTUAL_USER_PROFILE_DIR LOCALAPPDATA_SUBDIR;
    else if (guid_eq(&g, &FOLDERID_ProgramFiles)) text = "C:\\Program Files";
    else if (guid_eq(&g, &FOLDERID_ProgramFilesX64)) text = "C:\\Program Files";
    else if (guid_eq(&g, &FOLDERID_ProgramFilesX86)) text = "C:\\Program Files (x86)";
    else if (guid_eq(&g, &FOLDERID_ProgramFilesCommon) || guid_eq(&g, &FOLDERID_ProgramFilesCommonX64)) text = "C:\\Program Files\\Common Files";
    else if (guid_eq(&g, &FOLDERID_ProgramFilesCommonX86)) text = "C:\\Program Files (x86)\\Common Files";
    else if (guid_eq(&g, &FOLDERID_ProgramData)) text = "C:\\ProgramData";
    if (text == NULL) fle_stop(api, "a KNOWNFOLDERID that the model does not know (Profile, Documents, Desktop, Downloads, RoamingAppData, LocalAppData, ProgramFiles[X64|X86|Common|CommonX64|CommonX86], ProgramData only)");
    w = fle_w_from_ascii(text);
    fle_st(pp, 8, task_wstr(w, fle_w_len(w)));
    free(w);
    R[0] = FLE_HR_S_OK;
}
