/* Virtual module table and the kernel32 loader answers GetModuleHandleW and GetProcAddress
 * (docs/decisions/0015-virtual-modules.md). No DLL is loaded: a module is a name and a fixed
 * handle value, and a procedure address is a pseudo-VA that fle_dispatch routes to the runtime
 * answer of that API. As in fle_imports.h, the stubs set the Win32 return value in R[0] and leave
 * the guest return address on the stack for the caller's ret. */
#ifndef FLE_MODULES_H
#define FLE_MODULES_H

#include <stdint.h>

/* Handle of virtual module number N (1-based, position in fle_modules[] + 1):
 * FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP * N. A handle is only a token (nothing is
 * mapped there); a guest that dereferences it faults. */
#define FLE_MODULE_HANDLE_BASE UINT64_C(0x7FF800000000)
#define FLE_MODULE_HANDLE_STEP UINT64_C(0x10000)

/* GetModuleHandleW(NULL): the handle of the file that created the calling process. Here that is
 * the lifted DLL, mapped at its ImageBase (decision 0015). */
#define FLE_EXE_HANDLE UINT64_C(0x400000)

#define FLE_ERROR_MOD_NOT_FOUND  126u
#define FLE_ERROR_PROC_NOT_FOUND 127u

/* One answered API. The generator (fe-lift, src/imports_table.rs) emits fle_resp_table, one entry
 * per row of its import table, and the same pseudo-VAs as entries of fle_dispatch_table, so a
 * pointer returned by GetProcAddress can be called through fle_dispatch. The table is emitted only
 * when the closure reaches kernel32!GetProcAddress. When it is absent (weak reference) every
 * GetProcAddress lookup finds nothing. */
typedef struct {
    const char *dll;     /* lower-case module name, e.g. "kernel32.dll" */
    const char *name;    /* exported API name, case-sensitive */
    uint64_t va;         /* pseudo-VA returned by GetProcAddress */
    void (*fn)(void);    /* the answer, same convention as the jmp [IAT] stubs */
} fle_resp_entry;

extern const fle_resp_entry fle_resp_table[];
extern const uint64_t fle_resp_count;

/* The virtual module table (read-only view, for tests and tools). Names are lower-case with the
 * extension. Index i has handle FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP * (i + 1). */
uint64_t fle_module_count(void);
const char *fle_module_name(uint64_t index);
uint64_t fle_module_handle(uint64_t index);
/* Nonzero for a deferred module (decision 0096): installed with the product, not lifted yet. LoadLibrary succeeds without DllMain, the
 * export names are in the response table, and calling one stops by name. A real module (fle_realmod.h) of the same name wins. */
int fle_module_is_deferred(uint64_t index);

/* API set contract names (decision 0238, provisional; Microsoft Learn "Windows API sets", "API set loader operation"). A name that begins with "api-" or "ext-" is a contract name: the loader does not look for a file of that name
 * but resolves it through the API set schema to the host DLL. `name` is the lower-case spelling with its extension (".dll") as the name rules of GetModuleHandleW / LoadLibrary make it. The schema is a table of this environment
 * (fle_apiset_schema.inc, Wine 10.0, UNOFFICIAL) that is looked up by the name without its last "-<n>" component (the sub-version of l<n>-<n>-<n> does not take part in the lookup). */
#define FLE_APISET_NOT_A_CONTRACT 0   /* the name does not begin with "api-" / "ext-" (or has four characters or fewer) */
#define FLE_APISET_UNKNOWN        1   /* routed to the API set runtime, but this environment's schema has no entry for it */
#define FLE_APISET_NO_HOST        2   /* the schema has the contract and maps it to no host */
#define FLE_APISET_HOST           3   /* the schema maps the contract to the DLL *host (a lower-case name with the extension) */
int fle_apiset_resolve(const char *name, const char **host);

/* The export functions of the deferred modules (decision 0096): each stops the run by name with the four register arguments. Their
 * pseudo-VAs come from the response-table rows under the dll names blend2d.dll and quickfontcache_x64.dll. */
void fle_imp_deferred_blend2d_AggCreateInstance(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_dbkFCallWrapperAddr(void) __attribute__((noreturn));
void fle_imp_deferred_qfc___dbk_fcall_wrapper(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_InitSharedPython(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_CreateVST3Instance(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_LoadSharedElastiqueDll(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_AcquireSharedAnimTimer(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_AcquireSharedThreadBank(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_FontCache_GetStats(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_FontCache_CleanUp(void) __attribute__((noreturn));
void fle_imp_deferred_qfc_FontCache_AcquireFont(void) __attribute__((noreturn));

/* kernel32!GetModuleHandleW: HMODULE GetModuleHandleW(LPCWSTR lpModuleName). rcx = name (R[1]). */
void fle_imp_GetModuleHandleW(void);

/* kernel32!GetModuleHandleA: HMODULE GetModuleHandleA(LPCSTR lpModuleName). rcx = name (R[1]). Decision 0091: the same table, handles and
 * rules as GetModuleHandleW; a byte above 0x7F stops. */
void fle_imp_GetModuleHandleA(void);

/* kernel32!GetProcAddress: FARPROC GetProcAddress(HMODULE hModule, LPCSTR lpProcName).
 * rcx = module (R[1]), rdx = name (R[2]). */
void fle_imp_GetProcAddress(void);

/* kernel32!GetModuleHandleExW: stops by name (decision 0078). Decision 0190 (opt-in): a link that answers this API (harness/two/wrp1_vst3_imp.c wraps it for Serum2.vst3's UCRT thread start-up) compiles
 * the lifted units that call it with -DFLE_GMHEX_RETURNS: with the noreturn attribute below, clang drops the code that follows the call (the caller's `R[4] += 8` and the next instructions of
 * the lifted block), so the guest stack lost 8 bytes in _beginthreadex and the CreateThread that followed read a wrong dwCreationFlags (wall W12 of decision 0183). Without the macro nothing changes. */
void fle_imp_GetModuleHandleExW(void);

/* shell32 PIDL functions that the guest fetches by ordinal (decision 0099): rows of the response table, and each stops by name when called. */
void fle_imp_ILFindLastID(void) __attribute__((noreturn));
void fle_imp_ILRemoveLastID(void) __attribute__((noreturn));
void fle_imp_ILClone(void) __attribute__((noreturn));
void fle_imp_ILCloneFirst(void) __attribute__((noreturn));
void fle_imp_ILIsEqual(void) __attribute__((noreturn));
void fle_imp_ILCombine(void) __attribute__((noreturn));
void fle_imp_ILGetSize(void) __attribute__((noreturn));
void fle_imp_ILGetNext(void) __attribute__((noreturn));
void fle_imp_ILFree(void); /* decision 0126 wall 87: frees an ITEMIDLIST of fle_ole.c; stops by name only without fle_ole.c */

#endif
