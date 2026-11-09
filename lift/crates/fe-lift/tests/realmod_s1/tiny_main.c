/* A program that asks the loader for one module name: used to see how the runtime treats (a) a build without a real-module table,
 * (b) a table that breaks the rules of fle_realmod.h (the table comes from realmod_table.c with -DRM_ENTRIES), and (c) a name that
 * the table does not hold.
 *   tiny llw NAME     LoadLibraryW(NAME)
 *   tiny gmh NAME     GetModuleHandleW(NAME)
 * Prints "returned <rax hex> le <last error>" when the call returns. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_load.h"

static uint64_t stk[16];

int main(int argc, char **argv)
{
    uint16_t n[300];
    size_t i, len;

    if (argc < 3 || strlen(argv[2]) >= 299) return 64;
    len = strlen(argv[2]);
    for (i = 0; i < len; i++) n[i] = (uint16_t)(unsigned char)argv[2][i];
    n[len] = 0;
    R[4] = (uint64_t)(uintptr_t)stk;
    R[1] = (uint64_t)(uintptr_t)n;
    R[2] = 0;
    R[8] = 0;
    fle_set_last_error(5);
    if (strcmp(argv[1], "llw") == 0) {
        fle_imp_LoadLibraryW();
    } else if (strcmp(argv[1], "gmh") == 0) {
        fle_imp_GetModuleHandleW();
    } else {
        return 64;
    }
    {
        uint64_t r = R[0];
        fle_imp_GetLastError();
        printf("returned %llx le %llu\n", (unsigned long long)r, (unsigned long long)R[0]);
    }
    return 0;
}
