/* ole32 answers: COM initialization state, task memory, CLSID strings, IsEqualGUID
 * (docs/decisions/0037-apis-oleaut-ole.md; the pages read are listed in docs/investigations/apis-oleaut-ole-run.md).
 * Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx); the return value in R[0] (an HRESULT in eax,
 * zero-extended; IsEqualGUID a BOOL); the guest return address stays on the stack for the caller's ret. Single guest
 * thread only. A case the Microsoft pages do not define stops the run through fle_stop (fle_env.h). */
#ifndef FLE_OLE_H
#define FLE_OLE_H

#include <stdint.h>

#include "fle_com.h"

/* The largest block CoTaskMemAlloc hands out. Above it the call returns NULL, which the page allows ("insufficient
 * memory"); the limit is a limit of the virtual machine. */
#define FLE_COTASK_MAX UINT64_C(0x40000000)

/* ole32!CoInitialize: HRESULT f(LPVOID pvReserved). OleInitialize: HRESULT f(LPVOID pvReserved).
 * CoUninitialize, OleUninitialize: void f(void). The apartment is always single-threaded (decision 0037). */
void fle_imp_CoInitialize(void);
void fle_imp_CoUninitialize(void);
void fle_imp_OleInitialize(void);
void fle_imp_OleUninitialize(void);

/* ole32!CoTaskMemAlloc: LPVOID f(SIZE_T cb). CoTaskMemFree: void f(LPVOID). */
void fle_imp_CoTaskMemAlloc(void);
void fle_imp_CoTaskMemFree(void);

/* ole32!StringFromCLSID: HRESULT f(REFCLSID, LPOLESTR *). ProgIDFromCLSID: HRESULT f(REFCLSID, LPOLESTR *).
 * CLSIDFromString: HRESULT f(LPCOLESTR, LPCLSID). IsEqualGUID: BOOL f(REFGUID, REFGUID). */
void fle_imp_StringFromCLSID(void);
void fle_imp_ProgIDFromCLSID(void);
/* Decision 0129 (PROVISIONAL, S-com-registry-empty): ole32!CoCreateInstance: HRESULT f(REFCLSID rcx, LPUNKNOWN rdx, DWORD r8d, REFIID r9, LPVOID *ppv [5]); a CLSID that the virtual registry does not hold: *ppv = NULL, REGDB_E_CLASSNOTREG. See fle_ole.c. */
void fle_imp_CoCreateInstance(void);
void fle_imp_CLSIDFromString(void);
void fle_imp_IsEqualGUID(void);

/* shell32!SHGetMalloc (decision 0098, PROVISIONAL): HRESULT f(IMalloc **ppMalloc). rcx. Returns S_OK and an IMalloc pointer to one object of this
 * runtime whose allocator is the task allocator of CoTaskMemAlloc above (the Learn page of SHGetMalloc says CoTaskMemAlloc / CoTaskMemFree are
 * "the equivalent"). The object is { vtable pointer } followed by the nine IMalloc slots in the order of the interface (QueryInterface, AddRef,
 * Release, Alloc, Realloc, Free, GetSize, DidAlloc, HeapMinimize: the order of objidl.h, which the Learn pages do not print; the guest's call
 * offsets 0x10, 0x28 and 0x38 are Release, Free and DidAlloc in it). The slots hold the pseudo-VAs of five rows of the response table
 * (shell32.dll, "IMalloc::Release", "IMalloc::Free", "IMalloc::DidAlloc", "IMalloc::unimplemented"), found through fle_resp_table, so the
 * lifted code reaches them through fle_dispatch. Each SHGetMalloc counts one reference; Release answers the count after the decrement.
 * Free and DidAlloc are those of the pages: Free(NULL) has no effect, a live block is freed; DidAlloc(NULL) is -1, a live block 1, any other
 * pointer 0. The six other slots share one function that stops by name: the guest's call sites use Release, Free and DidAlloc only. */
void fle_imp_SHGetMalloc(void);
void fle_imp_IMalloc_Release(void);
void fle_imp_IMalloc_Free(void);
void fle_imp_IMalloc_DidAlloc(void);
void fle_imp_IMalloc_unimplemented(void) __attribute__((noreturn));

/* shell32!SHGetSpecialFolderLocation: HRESULT f(HWND hwnd, int csidl, PIDLIST_ABSOLUTE *ppidl) and shell32!SHGetPathFromIDListW: BOOL f(PCIDLIST_ABSOLUTE pidl,
 * LPWSTR pszPath) (decision 0116, PROVISIONAL; the notes are in fle_ole.c). CSIDL_FONTS and CSIDL_APPDATA only, the paths of SHGetFolderPathW; the ITEMIDLIST is a
 * CoTaskMemAlloc block of 12 bytes that the guest frees with CoTaskMemFree. */
void fle_imp_SHGetSpecialFolderLocation(void);
void fle_imp_SHGetPathFromIDListW(void);

/* Decision 0126 (walls 87 to 89, PROVISIONAL; the notes are in fle_ole.c): shell32!ILCreateFromPathW: PIDLIST_ABSOLUTE f(PCWSTR) (NULL for a path that the virtual file system does not hold), shell32!SHChangeNotifyRegister:
 * ULONG f(HWND, int, LONG, UINT, int, const SHChangeNotifyEntry *) (a registration that never delivers: S-shchange-no-events), shell32!SHChangeNotifyDeregister: BOOL f(ULONG). ILFree is fle_ole_il_free (called by the row of
 * fle_modules.c through a weak reference). fle_ole_shchange_live: the registrations not yet deregistered. */
void fle_imp_ILCreateFromPathW(void);
void fle_imp_SHChangeNotifyRegister(void);
void fle_imp_SHChangeNotifyDeregister(void);
void fle_ole_il_free(const char *api, uint64_t pidl);
uint32_t fle_ole_shchange_live(void);

/* Decision 0126 (wall 90, PROVISIONAL; the notes are in fle_ole.c): ole32!RegisterDragDrop: HRESULT f(HWND, LPDROPTARGET) and ole32!RevokeDragDrop: HRESULT f(HWND): the registration of a window of the window table, with IUnknown::AddRef /
 * Release called in the guest. fle_ole_dragdrop_live: the registrations not yet revoked. */
void fle_imp_RegisterDragDrop(void);
void fle_imp_RevokeDragDrop(void);
uint32_t fle_ole_dragdrop_live(void);

/* References of the SHGetMalloc object not yet released (for tests and tools). */
uint32_t fle_ole_shell_malloc_refs(void);

/* Calls of CoInitialize/OleInitialize not yet balanced (including those that returned S_FALSE), calls of
 * OleInitialize not yet balanced, and live CoTaskMemAlloc blocks (for tests and tools). */
uint32_t fle_ole_com_depth(void);
uint32_t fle_ole_ole_depth(void);
uint64_t fle_ole_task_live(void);

#endif
