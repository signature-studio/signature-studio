/* Real modules: a second PE that the build linked into the same program (rebased, lifted; decision 0080), next to the virtual
 * module table of fle_modules.c (decision 0015). Semantics: docs/decisions/0082-two-module-load-semantics.md; the mechanism was
 * measured on a synthetic PE in docs/investigations/stage1-two-module.md.
 *
 * A real module is a name and a base address that the link supplies in fle_realmod_table (below). Nothing is registered at run time
 * and the generator is not involved. Without a table (the default: fle_realmod.c holds weak definitions with count 0) no real
 * module exists, and the loader, GetModuleHandleW and GetProcAddress behave exactly as they did before this file.
 *
 * The handle of a real module is its base (decision 0080 D5; DllMain's hinstDLL is the same value). The image must be mapped at that
 * base before the first LoadLibrary of the module; its PE headers are read from memory then.
 *
 * fle_load.c and fle_modules.c reach these functions through weak references (FLE_REALMOD_PRESENT): a program that links neither
 * fle_realmod.c nor a table (the unit tests of the single-module runtime) is unchanged. */
#ifndef FLE_REALMOD_H
#define FLE_REALMOD_H

#include <stdint.h>
#include <stddef.h>

/* One real module. `name` is lower-case with the extension, the spelling that normalize() of fle_load.c produces
 * ("dsp_ippv2_x64.dll"). `base` is the ImageBase of the mapped image (a multiple of 64 K, not FLE_EXE_HANDLE, not a virtual module
 * handle). The name must not be a virtual module name (except a deferred module's, decision 0096: blend2d.dll and
 * quickfontcache_x64.dll are in fle_modules[] until they are lifted, and a real module of the same name wins over them everywhere);
 * at most 8 modules (RM_MAX of fle_realmod.c), names and bases pairwise different. A table that breaks this stops the run on first use. */
typedef struct {
    const char *name;
    uint64_t base;
} fle_realmod_entry;

/* The link supplies these (a generated or hand-written unit). */
extern const fle_realmod_entry fle_realmod_table[];
extern const uint64_t fle_realmod_count;

#ifdef FLE_REALMOD_IMPL
#define FLE_REALMOD_REF
#else
#define FLE_REALMOD_REF __attribute__((weak))
#define FLE_REALMOD_PRESENT() (fle_realmod_find != 0)
#endif

/* Index of the real module with this normalized name, or -1. */
int fle_realmod_find(const char *name) FLE_REALMOD_REF;
/* Index of the real module whose handle (= base) is h, or -1. */
int fle_realmod_find_handle(uint64_t h) FLE_REALMOD_REF;

/* The number of table entries, and the name and base of entry idx (decision 0103). For files that must not name the table symbols
 * themselves: a program that links neither fle_realmod.c nor a table (the unit tests of the single-module runtime) still links
 * fle_vfs.c and fle_res.c, which reach the table only through weak references (guarded by FLE_REALMOD_PRESENT). Since decision 0104
 * fle_vfs.c and fle_res.c use fle_realmod_find_handle, fle_realmod_base_of and fle_realmod_file_name for this, so nothing in the tree calls
 * these three; fle_realmod_base(idx) is fle_realmod_base_of(idx) without the range check. */
uint64_t fle_realmod_total(void) FLE_REALMOD_REF;
const char *fle_realmod_name(int idx) FLE_REALMOD_REF;
uint64_t fle_realmod_base(int idx) FLE_REALMOD_REF;

/* kernel32!LoadLibraryW / LoadLibraryExW of a real module. Not loaded: DllMain(hinst = base, DLL_PROCESS_ATTACH, NULL) runs before
 * this returns; a zero result runs DllMain(hinst = base, DLL_PROCESS_DETACH, NULL), leaves the module not loaded and returns 0 (the
 * last error stays what DllMain set). Otherwise returns the base and the reference count is one. Already loaded (or being loaded by
 * its own DllMain): one more reference, DllMain is not entered again. */
uint64_t fle_realmod_load(const char *api, int idx) FLE_REALMOD_REF;

/* kernel32!FreeLibrary of a real module. A module that is not loaded stops, and so does a count that would reach zero (decision
 * 0082 D11: fail-closed, as for a virtual module). Otherwise one reference less and 1. */
uint64_t fle_realmod_free(const char *api, int idx) FLE_REALMOD_REF;

/* kernel32!GetModuleHandleW of a real module: its base when loaded, 0 otherwise (the caller reports 126). While the module's own
 * DllMain runs it counts as not loaded (stage 1 prototype; not decided). */
uint64_t fle_realmod_module_handle(const char *api, int idx) FLE_REALMOD_REF;

/* kernel32!GetProcAddress by name over the export directory of the mapped image: base + export RVA, or 0 when the name is not
 * exported. A forwarder, a module that is not loaded and a malformed export table stop. */
uint64_t fle_realmod_getproc(const char *api, int idx, const char *name) FLE_REALMOD_REF;

/* Decision 0122 (PM item 6): DLL_THREAD_ATTACH (2) / DLL_THREAD_DETACH (3) for the real modules that are loaded now: DllMain(hinst = base, reason, NULL) in the context of the running thread. Attach goes in load order,
 * detach in the reverse order (a choice: Learn gives no order). With the environment variable FLE_THREAD_NO_DLL_NOTIFY=1 nothing is delivered (the "not delivered" fallback of the decision, which is NOT what Windows does). */
void fle_realmod_thread_notify(uint32_t reason) FLE_REALMOD_REF;

/* Base of the real module with index idx (0-based, in table order), or 0 when idx is not below the number of real modules. */
uint64_t fle_realmod_base_of(int idx) FLE_REALMOD_REF;

/* The module's own file name: the Name field of its export directory (IMAGE_EXPORT_DIRECTORY.Name, ASCII), copied to out (n bytes
 * including the NUL). Returns its length, or -1 when the image has no export directory or the name does not fit or is not ASCII.
 * A module that is not loaded stops. Decision 0104 (provisional): used for GetModuleFileNameW of a real module (fle_vfs.c). */
int fle_realmod_file_name(const char *api, int idx, char *out, unsigned n) FLE_REALMOD_REF;

/* Decision 0183 (wrp1, PROVISIONAL, OPT-IN: a link that supplies no path table behaves as before): a module requested by a PATH
 * ("C:\...\Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll") is the real module `name` when the link's path table has an entry whose `suffix`
 * (lower-case, backslashes, no drive) is the tail of the requested path (ASCII case and '/' folded to '\'; the character before the tail is a separator or there is none).
 * Learn (LoadLibraryW): "If the string specifies a full path, the function searches only that path for the module"; the caller (fle_load.c) therefore also requires that the
 * path names a file of the virtual volume. Two files of the same name in different directories (the Generators and the Effects Fruity Wrapper) are two real modules with two table names. */
typedef struct {
    const char *suffix;
    const char *name;
} fle_realmod_path_entry;
extern const fle_realmod_path_entry fle_realmod_path_table[];
extern const uint64_t fle_realmod_path_count;
/* Index of the real module for the path u[0..n), or -1 (no entry matches, or the path has a non-ASCII unit). */
int fle_realmod_find_path(const uint16_t *u, size_t n) FLE_REALMOD_REF;

#endif
