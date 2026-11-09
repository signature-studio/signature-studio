/* Decision 0210 (api2, PROVISIONAL, opt-in): shell32 answers (fle_api2.h): ShellExecuteA and ShellExecuteExW. The file drag-and-drop family (DragQueryFileW, DragQueryPoint, DragFinish) is NOT here: lift/out-crash1 (decision 0209,
 * fle_crash1_drag.c) provides it; harness/api2/api2_drag.c is the Wine-checked oracle for it. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_api2.h"

#define ERR_NO_ASSOCIATION 1155u
#define SE_ERR_NOASSOC 31u

/* ---- ShellExecute, the answer of the virtual machine: no association ----
 * S-ia2-shellexecute-noassoc (decision 0171, fle_wall_ia2.c, kept): no program is associated with a URL or a file type in the virtual machine's registry (decision 0129) and no process is ever started, so the call answers the failure the page defines for that case; it
 * is NOT "the browser opened". Learn (ShellExecuteA): the return value is "greater than 32 if successful, or an error value that is less than or equal to 32"; SE_ERR_NOASSOC 31 "There is no application associated with the given file name extension". Learn
 * (ShellExecuteExW): "Returns TRUE if successful; otherwise, FALSE. Call GetLastError for extended error information"; hInstApp "If the function succeeds, it sets this member to a value greater than 32. If the function fails, it is set to an SE_ERR_XXX error value that
 * indicates the cause of the failure"; ERROR_NO_ASSOCIATION 1155 (System Error Codes). A NULL lpFile stops (the page gives no behaviour). */
static void ascii_of(char *dst, size_t cap, uint64_t p)
{
    size_t i = 0;

    if (p) for (; i + 1 < cap; i++) {
        uint32_t c = (uint32_t)fle_ld(p + i, 1);

        if (c == 0) break;
        dst[i] = (char)(c >= 0x20 && c < 0x7F ? c : '?');
    }
    dst[i] = 0;
}

static void ascii_of_w(char *dst, size_t cap, uint64_t p)
{
    size_t i = 0;

    if (p) for (; i + 1 < cap; i++) {
        uint32_t c = (uint32_t)fle_ld(p + 2u * i, 2);

        if (c == 0) break;
        dst[i] = (char)(c >= 0x20 && c < 0x7F ? c : '?');
    }
    dst[i] = 0;
}

/* shell32!ShellExecuteA: HINSTANCE f(HWND hwnd rcx, LPCSTR lpOperation rdx, LPCSTR lpFile r8, LPCSTR lpParameters r9, LPCSTR lpDirectory [5], INT nShowCmd [6]). */
void fle_imp_ShellExecuteA(void)
{
    char op[40], file[300], par[200];

    if (R[8] == 0) fle_stop("ShellExecuteA", "lpFile is NULL: the page gives no behaviour");
    ascii_of(op, sizeof op, R[2]);
    ascii_of(file, sizeof file, R[8]);
    ascii_of(par, sizeof par, R[9]);
    fprintf(stderr, "fle: ShellExecuteA NOT CARRIED OUT (no association in the virtual machine; S-ia2-shellexecute-noassoc): verb \"%s\", file \"%s\", parameters \"%s\"; answered SE_ERR_NOASSOC (31)\n", op, file, par);
    R[0] = SE_ERR_NOASSOC;
}

/* shell32!ShellExecuteExW: BOOL f(SHELLEXECUTEINFOW *pExecInfo rcx). SHELLEXECUTEINFOW (shellapi.h, x64): cbSize @0, fMask @4, hwnd @8, lpVerb @16, lpFile @24, lpParameters @32, lpDirectory @40, nShow @48, hInstApp @56, ... (S-api2-5: the page lists the
 * members in this order; the offsets are the x64 layout of that order). */
void fle_imp_ShellExecuteExW(void)
{
    static const char api[] = "ShellExecuteExW";
    uint64_t p = R[1];
    char op[40], file[300], par[200];

    if (p == 0) fle_stop(api, "pExecInfo is NULL: the page gives no behaviour");
    if (fle_ld(p, 4) < 112u) fle_stop(api, "cbSize is smaller than the 112 bytes of SHELLEXECUTEINFOW: the page gives no behaviour");
    if (fle_ld(p + 24, 8) == 0) fle_stop(api, "lpFile is NULL: the page gives no behaviour (a class or ID list may be used instead)");
    ascii_of_w(op, sizeof op, fle_ld(p + 16, 8));
    ascii_of_w(file, sizeof file, fle_ld(p + 24, 8));
    ascii_of_w(par, sizeof par, fle_ld(p + 32, 8));
    fprintf(stderr, "fle: ShellExecuteExW NOT CARRIED OUT (no association in the virtual machine; S-ia2-shellexecute-noassoc): verb \"%s\", file \"%s\", parameters \"%s\"; hInstApp = SE_ERR_NOASSOC (31), ERROR_NO_ASSOCIATION (1155)\n", op, file, par);
    fle_st(p + 56, 8, SE_ERR_NOASSOC);
    fle_set_last_error(ERR_NO_ASSOCIATION);
    R[0] = 0;
}
