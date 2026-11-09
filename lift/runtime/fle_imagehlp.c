#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_imagehlp.h"

/* imagehlp!ImageNtHeader (docs/decisions/0029-apis-a6-a12.md). Microsoft Learn, ImageNtHeader (dbghelp.h):
 * "Locates the IMAGE_NT_HEADERS structure in a PE image and returns a pointer to the data." Parameter Base: "The
 * base address of an image that is mapped into memory by a call to the MapViewOfFile function." Return: a pointer
 * to the structure on success, NULL on failure. Layout of the PE file header: Microsoft Learn, PE Format: the
 * 4-byte value at offset 0x3c of the DOS header is the file offset of the "PE\0\0" signature, which is where
 * IMAGE_NT_HEADERS starts.
 *
 * What is answered: Base equal to FLE_EXE_HANDLE (0x400000), the lifted DLL, which the harness maps at its
 * ImageBase (decision 0015). The result is Base + e_lfanew. The headers are checked first (MZ at Base, e_lfanew
 * from 0x40, which is the size of the DOS header, to 0xFFF, the harness keeps the headers in the first 4 KiB, and
 * "PE\0\0" at the target); a mapped image that fails the check is a harness fault and stops the run.
 *
 * What is not: any other Base stops. The page says the call returns NULL on failure but names no input that
 * fails, and the runtime cannot tell a readable image from an arbitrary pointer (the guest's memory is the host's).
 * Returning NULL for a Base that is not the lifted image would be a value this runtime made up: the two guest
 * callers (0x63b714, 0x63bb02) pass a MapViewOfFile result and take NULL as "not a PE file", so the run would
 * continue on a path that no measured behaviour supports. The handles of the other virtual modules point at
 * nothing (decision 0015), NULL is not a mapped image, and MapViewOfFile itself is not answered yet (A13).
 * Success does not touch the last error (the page does not say it does). */
#define IMG_LFANEW_MIN 0x40u
#define IMG_LFANEW_MAX 0xFFFu

void fle_imp_ImageNtHeader(void)
{
    static const char api[] = "ImageNtHeader";
    const uint64_t base = R[1];
    uint32_t lfanew;

    if (base != FLE_EXE_HANDLE)
        fle_stop(api, "Base is not the lifted image (0x400000); the page does not say which Base fails, so no NULL is invented");
    if ((uint16_t)fle_ld(base, 2) != 0x5a4d) fle_stop(api, "no MZ signature at the lifted image's base");
    lfanew = (uint32_t)fle_ld(base + 0x3c, 4);
    if (lfanew < IMG_LFANEW_MIN || lfanew > IMG_LFANEW_MAX) fle_stop(api, "e_lfanew of the lifted image is out of range");
    if ((uint32_t)fle_ld(base + lfanew, 4) != 0x4550) fle_stop(api, "no PE signature at e_lfanew of the lifted image");
    R[0] = base + lfanew;
}
