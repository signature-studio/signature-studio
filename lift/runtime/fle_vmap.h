/* Memory protection and file-mapping answers for IAT slots reached by a tail jump
 * (docs/decisions/0024-apis-a3-a5-a13.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx),
 * R[8] (r8), R[9] (r9), then [rsp + 0x28 + 8 * (n - 5)]; the Win32 return value in R[0]; the guest return address stays
 * on the stack for the caller's ret. Single guest thread only. A case the Microsoft pages do not define stops the run
 * through fle_stop (fle_env.h). VirtualProtect and VirtualQuery work on the regions of fle_sys.c (VirtualAlloc) and on
 * the views made here; any other address stops (VirtualProtect also answers for the 5-byte inline hook points of a registered
 * image, decision 0105, below). File mapping objects are backed by an anonymous in-memory file; the
 * host file system is never touched. */
#ifndef FLE_VMAP_H
#define FLE_VMAP_H

#include <stdint.h>

/* Handles of file mapping objects (decision 0024): FLE_VMAP_HANDLE_BASE + 4 * (i + 1), never reused, in a region of
 * FLE_VMAP_REGION_SIZE bytes above the process handles of fle_vol.h. */
#define FLE_VMAP_HANDLE_BASE UINT64_C(0x7FFA70000000)
#define FLE_VMAP_REGION_SIZE UINT64_C(0x10000000)

/* sizeof(MEMORY_BASIC_INFORMATION) on x64 (the layout follows from the member types on the page; fle_vmap.c). */
#define FLE_MBI_SIZE 48u

/* The largest file mapping object this runtime makes (a runtime limit; a larger one stops the run). */
#define FLE_VMAP_MAX_BYTES (UINT64_C(1) << 30)

/* Decision 0123 (PROVISIONAL), wall 25: the loaded images that VirtualQuery describes. The harness registers every image that it mapped (the main module and each real module), at its base, with the PE headers in memory
 * (fle_vmap_register_image(base)): the runtime reads the SizeOfImage and the section table FROM MEMORY at that base. A query for an address inside a registered image answers (Learn MEMORY_BASIC_INFORMATION):
 * AllocationBase = the image base, State MEM_COMMIT (0x1000), Type MEM_IMAGE (0x1000000), AllocationProtect = the protection of the first page (the headers: PAGE_READONLY, 0x02), BaseAddress = the page that holds
 * the address rounded down, Protect = the protection of that page from its section's characteristics (IMAGE_SCN_MEM_READ 0x40000000, WRITE 0x80000000, EXECUTE 0x20000000: Memory Protection Constants:
 * R = PAGE_READONLY 0x02, RW = PAGE_READWRITE 0x04, X = PAGE_EXECUTE 0x10, RX = PAGE_EXECUTE_READ 0x20, RWX = PAGE_EXECUTE_READWRITE 0x40; no flag = PAGE_NOACCESS 0x01; a shared section is not modelled), RegionSize =
 * the number of bytes from BaseAddress to the end of the run of pages with the same protection (the headers are one run, each section is a run; two neighbouring sections with the same protection are one run). The
 * pages between the end of the last section and SizeOfImage are the headers' protection... they do not exist: the image ends at SizeOfImage. The rule of the page that the VirtualAlloc regions and the views keep.
 * Anything else that the runtime does not track (host heap blocks, the free address space between regions) is answered as MEM_FREE (0x10000) with BaseAddress = the page, AllocationBase 0, AllocationProtect 0,
 * RegionSize = the bytes up to the next tracked region or image above it (or to the end of the address space: FLE_MAX_APP_ADDR + 1), Protect 0, Type 0 (the page: for free pages "the information in the AllocationBase,
 * AllocationProtect, Protect, and Type members is undefined"; zero is a choice: S-vq-free). The stack and TEB of a guest thread are VirtualAlloc regions or the host buffers: the first thread's stack (the harness's
 * mmap) is registered with fle_vmap_register_stack(lo, hi) as MEM_PRIVATE / MEM_COMMIT / PAGE_READWRITE, so that a query inside it answers (not free). */
void fle_vmap_register_image(uint64_t base);
void fle_vmap_register_range(uint64_t base, uint64_t size, uint32_t protect, uint32_t type);

/* kernel32!VirtualProtect: BOOL f(LPVOID, SIZE_T, DWORD flNewProtect, PDWORD lpflOldProtect).
 * kernel32!VirtualQuery: SIZE_T f(LPCVOID, PMEMORY_BASIC_INFORMATION, SIZE_T dwLength). */
void fle_imp_VirtualProtect(void);
void fle_imp_VirtualQuery(void);

/* Inline hook points (docs/decisions/0105-inline-hook-points.md). The guest of a lifted module may hook one of its own functions by
 * overwriting the first 5 bytes with `jmp rel32`. The lifted C does not read instructions from memory, so fe-lift (--hook-points) names
 * these functions in advance:
 *  - the generated unit of a module with hook points includes this header, defines a const table of fle_hook_point and registers it from a
 *    constructor: fle_hook_register(image base, SizeOfImage, table, count);
 *  - the first instruction of such a function is guarded by fle_hook_target(va, orig5): 0 when the image still holds the original 5 bytes
 *    (orig5, byte k in bits 8k..8k+7), the target of the `jmp rel32` when the guest wrote one (the lifted function then does a tail
 *    dispatch to it), and a stop by name for anything else;
 *  - VirtualProtect on an address inside a registered image succeeds for exactly a registered hook point with dwSize <= 5 and stops for any
 *    other address of the image. `prot` is the protection of the page when the module is loaded (PAGE_* value, from the characteristics of
 *    the section); VirtualProtect records the new protection per page (it is returned as the old one by the next call; nothing is enforced). */
typedef struct {
    uint64_t va;
    uint64_t orig5;
    uint32_t prot;
} fle_hook_point;

void fle_hook_register(uint64_t base, uint64_t size, const fle_hook_point *pts, uint64_t n);
uint64_t fle_hook_target(uint64_t va, uint64_t orig5);

/* Call-site memory overrides (docs/decisions/0120-blend2d-no-jit.md; fe-lift --site-overrides). The generated unit calls this just before the
 * instruction at `va`: the 32-bit word at `addr` (a register of the guest plus a displacement, computed by the generated C) is `expect` (what the
 * guest wrote) and becomes `nw`; it is left as it is when it already is `nw` (the instruction ran before, or a loop); any other value stops the run
 * by name (fle_stop), so a change of the guest's code is not hidden. */
void fle_site_store32(uint64_t va, uint64_t addr, uint32_t expect, uint32_t nw);

/* kernel32!CreateFileMappingW: HANDLE f(HANDLE hFile, LPSECURITY_ATTRIBUTES, DWORD flProtect, DWORD sizeHigh,
 * DWORD sizeLow, LPCWSTR name). kernel32!MapViewOfFile: LPVOID f(HANDLE, DWORD access, DWORD offHigh, DWORD offLow,
 * SIZE_T bytes). kernel32!UnmapViewOfFile: BOOL f(LPCVOID). */
void fle_imp_CreateFileMappingW(void);
void fle_imp_MapViewOfFile(void);
void fle_imp_UnmapViewOfFile(void);

/* For fle_vol.c (DuplicateHandle). is_handle: 1 when `h` is in the file-mapping handle region (live or not).
 * dup_handle: 1 and the new handle in *out, or 0 when `h` is not live. close_handle: 1, or 0 when `h` is not live. */
int fle_vmap_is_handle(uint64_t h);
int fle_vmap_dup_handle(uint64_t h, uint64_t *out);
int fle_vmap_close_handle(uint64_t h);

/* Live file mapping handles, live views, and live mapping objects (for tests and tools). */
uint64_t fle_vmap_handle_count(void);
uint64_t fle_vmap_view_count(void);
uint64_t fle_vmap_object_count(void);

#endif
