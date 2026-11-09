/* The common item dialog COM model (decision 0178, PROVISIONAL): ole32!CoCreateInstance(CLSID_FileOpenDialog) answers an IFileOpenDialog object, and the shell items it hands out.
 *
 * WHY: the engine's File -> Open... (and Ctrl+O) calls CoCreateInstance(CLSID_FileOpenDialog, NULL, CLSCTX_INPROC_SERVER, IID_IFileOpenDialog) once (decision 0171, U1); the virtual registry holds no
 * shell class, so the answer was REGDB_E_CLASSNOTREG and the engine gave up silently. This file is the object it asks for.
 *
 * Same convention as fle_imports.h: arguments in R[1] (rcx = `this` for a method), R[2] (rdx), R[8] (r8), R[9] (r9), the fifth on the guest stack (fle_arg); the HRESULT / ULONG in R[0]; the guest return
 * address stays on the stack for the caller's ret.
 *
 * OPT-IN: the vtable slots of the objects are response-table rows (the precedent is shell32 'IMalloc::Release', decision 0098), added only by fe-lift --extra-imports (harness/seeds/dlg1-imports.txt). A build
 * whose rows do not hold them (every build of an earlier decision) declines in fle_filedlg_cocreate: CoCreateInstance then answers what it answered before (REGDB_E_CLASSNOTREG).
 *
 * The dialog's 'user' is the HOST: Show() asks a picker for the selection. Native harness: environment FLE_FILEDLG_PICK = the guest path(s) of the selection ('|' between several), which stands for the user's
 * input (NOT for the engine's output; every Show prints that the selection came from the harness). Browser: fle_filedlg_set_picker() installs the function that asks showOpenFilePicker (design in
 * docs/0178-filedialog-and-flp-open.md). No picker and no environment: the user cancelled (HRESULT_FROM_WIN32(ERROR_CANCELLED)).
 *
 * Vtable orders (shobjidl.idl, Wine's copy of the SDK interface definitions; Learn lists the methods alphabetically): IFileOpenDialog = IUnknown 0-2, IModalWindow::Show 3, IFileDialog 4-26 (SetFileTypes,
 * SetFileTypeIndex, GetFileTypeIndex, Advise, Unadvise, SetOptions, GetOptions, SetDefaultFolder, SetFolder, GetFolder, GetCurrentSelection, SetFileName, GetFileName, SetTitle, SetOkButtonLabel,
 * SetFileNameLabel, GetResult, AddPlace, SetDefaultExtension, Close, SetClientGuid, ClearClientData, SetFilter), IFileOpenDialog 27-28 (GetResults, GetSelectedItems). IShellItem = IUnknown 0-2,
 * BindToHandler 3, GetParent 4, GetDisplayName 5, GetAttributes 6, Compare 7. IShellItemArray = IUnknown 0-2, BindToHandler 3, GetPropertyStore 4, GetPropertyDescriptionList 5, GetAttributes 6, GetCount 7,
 * GetItemAt 8, EnumItems 9. IFileDialogCustomize = IUnknown 0-2, EnableOpenDropDown 3, AddMenu, AddPushButton, AddComboBox, AddRadioButtonList, AddCheckButton, AddEditBox, AddSeparator, AddText,
 * SetControlLabel, GetControlState, SetControlState, GetEditBoxText, SetEditBoxText, GetCheckButtonState, SetCheckButtonState, AddControlItem, RemoveControlItem, RemoveAllControlItems,
 * GetControlItemState, SetControlItemState, GetSelectedControlItem, SetSelectedControlItem, StartVisualGroup, EndVisualGroup, MakeProminent, SetControlItemText 29 (a separate sub-object of the dialog,
 * as in a real multiple-inheritance object: its own `this` and vtable). IFileDialogEvents (implemented by the guest) = IUnknown 0-2, OnFileOk 3, OnFolderChanging 4, OnFolderChange 5, OnSelectionChange 6, OnShareViolation 7, OnTypeChange 8, OnOverwrite 9. */
#ifndef FLE_FILEDLG_H
#define FLE_FILEDLG_H

#include <stdint.h>

/* The request that Show() hands to the picker (UTF-16 strings or NULL; owned by the dialog). */
#define FLE_FILEDLG_MAX_FILTERS 32u
typedef struct {
    uint32_t options;                    /* FILEOPENDIALOGOPTIONS */
    unsigned nfilters;
    const uint16_t *filter_name[FLE_FILEDLG_MAX_FILTERS];
    const uint16_t *filter_spec[FLE_FILEDLG_MAX_FILTERS];
    uint32_t filter_index;               /* 1-based */
    const uint16_t *title, *ok_label, *file_name, *default_ext;
    const uint16_t *folder;              /* the guest path of the folder set by SetFolder / SetDefaultFolder, or NULL */
    int save;                            /* decision 0188: 1 = a Save dialog (CLSID_FileSaveDialog); the picker returns one guest path that need not exist (its folder must) */
} fle_filedlg_request;

/* A picker returns 1 and *paths = a malloc'd array of *n malloc'd UTF-16 guest paths (the dialog frees them), or 0 for 'the user cancelled'. */
typedef int (*fle_filedlg_picker)(const fle_filedlg_request *rq, uint16_t ***paths, unsigned *n);
void fle_filedlg_set_picker(fle_filedlg_picker p);

/* Called by ole32!CoCreateInstance (fle_ole.c) after its argument checks: 1 = handled (*hr is the HRESULT, *ppv written), 0 = not a class of this model (or the rows are not in this build). */
int fle_filedlg_cocreate(uint64_t rclsid, uint64_t punkouter, uint32_t clsctx, uint64_t riid, uint64_t ppv, uint32_t *hr);

/* shell32!SHCreateItemFromParsingName: HRESULT f(PCWSTR pszPath rcx, IBindCtx *pbc rdx, REFIID riid r8, void **ppv r9).
 * shell32!SHGetKnownFolderPath: HRESULT f(REFKNOWNFOLDERID rfid rcx, DWORD dwFlags rdx, HANDLE hToken r8, PWSTR *ppszPath r9). */
void fle_imp_SHCreateItemFromParsingName(void);
void fle_imp_SHGetKnownFolderPath(void);

/* The vtable slots (rows 'ShellObj::<m>', 'IFileOpenDialog::<m>', 'IShellItem::<m>', 'IShellItemArray::<m>' of harness/seeds/dlg1-imports.txt). */
void fle_imp_ShellObj_QueryInterface(void);
void fle_imp_ShellObj_AddRef(void);
void fle_imp_ShellObj_Release(void);

#define FLE_FILEDLG_FOR_EACH_DIALOG_METHOD(X) \
    X(Show) X(SetFileTypes) X(SetFileTypeIndex) X(GetFileTypeIndex) X(Advise) X(Unadvise) X(SetOptions) X(GetOptions) X(SetDefaultFolder) X(SetFolder) X(GetFolder) \
    X(GetCurrentSelection) X(SetFileName) X(GetFileName) X(SetTitle) X(SetOkButtonLabel) X(SetFileNameLabel) X(GetResult) X(AddPlace) X(SetDefaultExtension) X(Close) \
    X(SetClientGuid) X(ClearClientData) X(SetFilter) X(GetResults) X(GetSelectedItems)
/* decision 0188: IFileSaveDialog = the IFileDialog slots 0-26 above (same rows as IFileOpenDialog) + these at 27-31 (rows 'IFileSaveDialog::<m>' of harness/seeds/exp1-imports.txt) */
#define FLE_FILEDLG_FOR_EACH_SAVE_METHOD(X) X(SetSaveAsItem) X(SetProperties) X(SetCollectedProperties) X(GetProperties) X(ApplyProperties)
#define FLE_FILEDLG_FOR_EACH_ITEM_METHOD(X) X(BindToHandler) X(GetParent) X(GetDisplayName) X(GetAttributes) X(Compare)
#define FLE_FILEDLG_FOR_EACH_ARRAY_METHOD(X) X(BindToHandler) X(GetPropertyStore) X(GetPropertyDescriptionList) X(GetAttributes) X(GetCount) X(GetItemAt) X(EnumItems)

#define FLE_FILEDLG_FOR_EACH_CUSTOM_METHOD(X) \
    X(EnableOpenDropDown) X(AddMenu) X(AddPushButton) X(AddComboBox) X(AddRadioButtonList) X(AddCheckButton) X(AddEditBox) X(AddSeparator) X(AddText) X(SetControlLabel) X(GetControlState) \
    X(SetControlState) X(GetEditBoxText) X(SetEditBoxText) X(GetCheckButtonState) X(SetCheckButtonState) X(AddControlItem) X(RemoveControlItem) X(RemoveAllControlItems) \
    X(GetControlItemState) X(SetControlItemState) X(GetSelectedControlItem) X(SetSelectedControlItem) X(StartVisualGroup) X(EndVisualGroup) X(MakeProminent) X(SetControlItemText)

#define FLE_FILEDLG_DECL_DIALOG(m) void fle_imp_IFileOpenDialog_##m(void);
#define FLE_FILEDLG_DECL_ITEM(m) void fle_imp_IShellItem_##m(void);
#define FLE_FILEDLG_DECL_ARRAY(m) void fle_imp_IShellItemArray_##m(void);
FLE_FILEDLG_FOR_EACH_DIALOG_METHOD(FLE_FILEDLG_DECL_DIALOG)
FLE_FILEDLG_FOR_EACH_ITEM_METHOD(FLE_FILEDLG_DECL_ITEM)
FLE_FILEDLG_FOR_EACH_ARRAY_METHOD(FLE_FILEDLG_DECL_ARRAY)
#define FLE_FILEDLG_DECL_SAVE(m) void fle_imp_IFileSaveDialog_##m(void);
FLE_FILEDLG_FOR_EACH_SAVE_METHOD(FLE_FILEDLG_DECL_SAVE)
#define FLE_FILEDLG_DECL_CUSTOM(m) void fle_imp_IFileDialogCustomize_##m(void);
FLE_FILEDLG_FOR_EACH_CUSTOM_METHOD(FLE_FILEDLG_DECL_CUSTOM)

/* Inspection for tests and diagnostics: live objects of the model (dialogs, items, arrays). */
unsigned fle_filedlg_live_objects(void);

/* Decision 0193 (fpick1, PROVISIONAL), see fle_filedlg.c browser_picker: the Worker calls this at the engine's idle. NULL, or a line
 * `<size> TAB <address of the bytes> TAB <guest path, UTF-8>` for a Save target whose file the engine has finished writing (no open handle; size and last-write time unchanged between two calls); a file
 * written again is reported again. The bytes are valid until the next write to that file. */
const char *fle_filedlg_saved_poll(void);

#endif
