/* imagehlp!ImageNtHeader (docs/decisions/0029-apis-a6-a12.md). It reads the PE headers that the guest points
 * at; nothing is read from the host. As in fle_imports.h, the stub sets the Win32 return value in R[0] and leaves
 * the guest return address on the stack for the caller's ret. */
#ifndef FLE_IMAGEHLP_H
#define FLE_IMAGEHLP_H

#include <stdint.h>

/* imagehlp!ImageNtHeader: PIMAGE_NT_HEADERS ImageNtHeader(PVOID Base). rcx = Base. */
void fle_imp_ImageNtHeader(void);

#endif
