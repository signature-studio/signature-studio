/* advapi32!OpenProcessToken and advapi32!GetTokenInformation (decision 0136, PROVISIONAL; the wall of the first-run wizard: after the OK button of the error dialog the guest closes the Welcome wizard
 * and asks whether the process is elevated, f_10e6260: GetCurrentProcess, OpenProcessToken(h, TOKEN_QUERY, &t), GetTokenInformation(t, 20, &e, 4, &n), e != 0, CloseHandle(t)).
 * Learn (OpenProcessToken): "opens the access token associated with a process"; ProcessHandle "a handle to the process whose access token is opened"; DesiredAccess "an access mask that specifies the
 * requested types of access to the access token"; TokenHandle "a pointer to a handle that identifies the newly opened access token when the function returns"; the return value is nonzero on success;
 * "to close the access token handle returned through the TokenHandle parameter, call CloseHandle".
 * Learn (GetTokenInformation): TokenHandle "must have TOKEN_QUERY access" for every class but TokenSource; TokenInformationClass "a value from the TOKEN_INFORMATION_CLASS enumerated type"; ReturnLength
 * "receives the number of bytes needed for the buffer ... If this value is larger than the value specified in the TokenInformationLength parameter, the function fails and stores no data in the buffer";
 * the return value is nonzero on success. Learn (TOKEN_ELEVATION): { DWORD TokenIsElevated }, "a nonzero value if the token has elevated privileges; otherwise, a zero value".
 * Answered: ProcessHandle the pseudo handle of the current process (GetCurrentProcess, fle_proc.h), DesiredAccess exactly TOKEN_QUERY (0x8; the number is winnt.h: the Learn page lists the right by name),
 * TokenHandle not NULL: a new handle of the token region (4 * (n + 1) above FLE_TOKEN_HANDLE_BASE, never reused), closed by CloseHandle through the hook of fle_vfs.c. GetTokenInformation of a live
 * token with class 20 (TokenElevation; the number is winnt.h, the Learn enumeration page lists names): TokenIsElevated = 0 (S-token-not-elevated, a CHOICE WITHOUT A SOURCE: the page does not say what
 * a given process's token is; a standard, non-elevated user is the least privileged and the safest choice), ReturnLength = 4; a buffer smaller than 4 bytes (or NULL) fails with ERROR_INSUFFICIENT_BUFFER
 * (122; the page names no code: S-token-short-buffer, a CHOICE). Every other class, access mask, process handle or token handle stops by name. Withdrawn when a security model exists. */
#include <stdint.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_proc.h"
#include "fle_vfs.h"
#include "fle_token.h"

#define FLE_TOKEN_HANDLE_BASE UINT64_C(0x7FFAC0000000)
#define FLE_TOKEN_MAX 64u
#define FLE_TOKEN_QUERY 0x0008u
#define FLE_TOKEN_CLASS_ELEVATION 20u
#define FLE_ERROR_INVALID_HANDLE_ 6u
#define FLE_ERROR_INSUFFICIENT_BUFFER_ 122u

static int tok_live[FLE_TOKEN_MAX];
static unsigned tok_n;
static int tok_hooked;

static int tok_in_region(uint64_t h)
{
    return h > FLE_TOKEN_HANDLE_BASE && (h - FLE_TOKEN_HANDLE_BASE) % 4u == 0 && (h - FLE_TOKEN_HANDLE_BASE) / 4u <= tok_n;
}

static int tok_close_hook(uint64_t h)
{
    uint64_t k;

    if (!tok_in_region(h)) return 0;
    k = (h - FLE_TOKEN_HANDLE_BASE) / 4u - 1u;
    if (!tok_live[k]) {
        fle_set_last_error(FLE_ERROR_INVALID_HANDLE_);
        R[0] = 0;
        return 1;
    }
    tok_live[k] = 0;
    R[0] = 1;
    return 1;
}

/* advapi32!OpenProcessToken: BOOL f(HANDLE ProcessHandle rcx, DWORD DesiredAccess edx, PHANDLE TokenHandle r8). */
void fle_imp_OpenProcessToken(void)
{
    static const char api[] = "OpenProcessToken";

    if (R[1] != FLE_PSEUDO_PROCESS_HANDLE) fle_stop(api, "ProcessHandle is not the pseudo handle of the current process: no other process exists (decision 0136)");
    if ((uint32_t)R[2] != FLE_TOKEN_QUERY) fle_stop(api, "DesiredAccess is not exactly TOKEN_QUERY (0x8): the page gives no behaviour for another access mask (decision 0136)");
    if (R[8] == 0) fle_stop(api, "TokenHandle is NULL: the page gives no behaviour (decision 0136)");
    if (tok_n == FLE_TOKEN_MAX) fle_stop(api, "more tokens than the runtime holds");
    if (!tok_hooked) {
        fle_vfs_add_close_hook(tok_close_hook);
        tok_hooked = 1;
    }
    tok_live[tok_n] = 1;
    fle_st(R[8], 8, FLE_TOKEN_HANDLE_BASE + 4u * (tok_n + 1u));
    tok_n++;
    R[0] = 1;
}

/* advapi32!GetTokenInformation: BOOL f(HANDLE TokenHandle rcx, TOKEN_INFORMATION_CLASS edx, LPVOID TokenInformation r8, DWORD TokenInformationLength r9d, PDWORD ReturnLength [5]). */
void fle_imp_GetTokenInformation(void)
{
    static const char api[] = "GetTokenInformation";
    uint64_t pret = fle_arg(5);
    uint32_t cls = (uint32_t)R[2], len = (uint32_t)R[9];

    if (!tok_in_region(R[1]) || !tok_live[(R[1] - FLE_TOKEN_HANDLE_BASE) / 4u - 1u]) fle_stop(api, "TokenHandle is not a live token of this runtime: the page names no error code (decision 0136)");
    if (cls != FLE_TOKEN_CLASS_ELEVATION) fle_stop(api, "TokenInformationClass is not TokenElevation (20): only the guest's elevation check is answered (decision 0136)");
    if (pret == 0) fle_stop(api, "ReturnLength is NULL: the page does not mark it optional and gives no behaviour (decision 0136)");
    fle_st(pret, 4, 4);
    if (R[8] == 0 || len < 4u) {
        fle_set_last_error(FLE_ERROR_INSUFFICIENT_BUFFER_);
        R[0] = 0;
        return;
    }
    fle_st(R[8], 4, 0);
    R[0] = 1;
}
