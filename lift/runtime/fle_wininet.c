/* wininet!InternetGetConnectedState: BOOL f(LPDWORD lpdwFlags rcx, DWORD dwReserved edx) (decision 0126, PROVISIONAL, wall 85). NOT A NETWORK MODEL: the definition of the virtual environment, "this machine has no
 * Internet connection". Learn (InternetGetConnectedState): "Returns TRUE if there is an active modem or a LAN Internet connection, or FALSE if there is no Internet connection, or if all possible Internet connections are
 * not currently active"; lpdwFlags "receives the connection description. This parameter may return a valid flag even when the function returns FALSE" (INTERNET_CONNECTION_MODEM 0x01, LAN 0x02, PROXY 0x04, ... OFFLINE 0x20:
 * "Local system is in offline mode" = the user's offline setting, which is not what is meant here); dwReserved "is reserved and must be 0" (a nonzero value: the page gives no behaviour, it stops). S-inet-offline, a CHOICE
 * WITHOUT A SOURCE: the result is FALSE and no flag is set (0): no modem, no LAN, no proxy, not offline mode; the last error is left alone (the page: "the application can call GetLastError" without naming a code).
 * Withdrawn when a network lane defines a virtual connection. Every other wininet / winhttp / socket call stays unanswered ((d') network). */
#include <stdint.h>

#include "fle_runtime.h"
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_wininet.h"

void fle_imp_InternetGetConnectedState(void)
{
    if ((uint32_t)R[2] != 0) fle_stop("InternetGetConnectedState", "dwReserved is not 0: the page says it must be 0 and gives no behaviour for another value (decision 0126)");
    if (R[1] != 0) fle_st(R[1], 4, 0);
    R[0] = 0;
}

/* ---- wininet!InternetOpenW, InternetSetOptionW, InternetOpenUrlW, InternetCloseHandle (decision 0132, PROVISIONAL, wall 23) ----
 * NOT A NETWORK MODEL: an OFFLINE session. The news-download thread of the guest ("ILNewsDownload", f_64bfa0) opens a session, sets the receive time-out, and opens a URL; with no Internet connection
 * (S-inet-offline, wall 85) the open of the URL fails and the guest closes the session. Learn (InternetOpenW): "Initializes an application's use of the WinINet functions"; "Returns a valid handle that the application
 * passes to subsequent WinINet functions. If InternetOpen fails, it returns NULL"; dwAccessType INTERNET_OPEN_TYPE_DIRECT / PRECONFIG / PRECONFIG_WITH_NO_AUTOPROXY / PROXY (the page names no numbers; wininet.h: PRECONFIG 0,
 * DIRECT 1, PROXY 3, PRECONFIG_WITH_NO_AUTOPROXY 4: UNOFFICIAL here); dwFlags INTERNET_FLAG_ASYNC (0x10000000), FROM_CACHE / OFFLINE (0x01000000) (numbers also from wininet.h). InternetOpen makes no network request,
 * so it succeeds with no connection (S-inet-open-offline, a CHOICE WITHOUT A SOURCE: the page says nothing about a machine without a connection). Answered: lpszAgent a string, dwAccessType 0, 1 or 4, lpszProxy and
 * lpszProxyBypass NULL ("ignored and should be NULL" when the type is not PROXY), dwFlags 0: a new handle of the HINTERNET region (4 * (n + 1) above FLE_INET_HANDLE_BASE, never reused). Every other form stops (a proxy, the
 * asynchronous mode and the cache-only modes need a model of their own).
 * Learn (InternetSetOptionW): "Returns TRUE if successful, or FALSE otherwise"; "GetLastError will return the error ERROR_INVALID_PARAMETER if an option flag that cannot be set is specified"; the option numbers are those of
 * the page "Option Flags" (docs/investigations/seh2/learn/WinINet-option-flags.txt): CONNECT_TIMEOUT 2, SEND_TIMEOUT / CONTROL_SEND_TIMEOUT 5, RECEIVE_TIMEOUT / CONTROL_RECEIVE_TIMEOUT 6: "an unsigned long integer value
 * that contains the time-out value, in milliseconds". Answered: a live session handle and one of these three options with a 4-byte value: the value is kept, TRUE. Any other option or size stops.
 * Learn (InternetOpenUrlW): "Opens a resource specified by a complete FTP or HTTP URL" (the page was read for the error list only); the result is a handle, or NULL with GetLastError. S-inet-open-url-fails (a CHOICE
 * WITHOUT A SOURCE): with no Internet connection the result is NULL and the last error ERROR_INTERNET_NAME_NOT_RESOLVED (12007, "The server name could not be resolved", Learn Appendix: WinINet Error Codes): a machine
 * with no connection cannot resolve a host name. A live session handle and a non-NULL URL are required (anything else stops). Withdrawn when a network lane defines a virtual connection.
 * Learn (InternetCloseHandle): "Closes a single Internet handle"; TRUE if closed. A live session handle is closed (TRUE); a handle that is not one stops (the page names no error code). */
int fle_act1_close_handle(uint64_t h) __attribute__((weak));
void fle_act1_note_open_url(uint64_t url, uint64_t headers, uint32_t flags) __attribute__((weak));
#define FLE_INET_HANDLE_BASE UINT64_C(0x7FFAB0000000)
#define FLE_INET_MAX 256u
#define FLE_ERROR_INTERNET_NAME_NOT_RESOLVED 12007u

static struct {
    int live;
    uint32_t timeouts[3];   /* CONNECT 2, SEND 5, RECEIVE 6: the last value set (0 = never set) */
} inet_sess[FLE_INET_MAX];
static unsigned inet_n;

static int inet_slot(uint64_t h)
{
    uint64_t k;

    if (h <= FLE_INET_HANDLE_BASE || (h - FLE_INET_HANDLE_BASE) % 4u) return -1;
    k = (h - FLE_INET_HANDLE_BASE) / 4u - 1u;
    if (k >= inet_n || !inet_sess[k].live) return -1;
    return (int)k;
}

/* Decision 0213 (act1): 1 when h is a live session handle of this file (read by fle_act1_net.c, whose connection handles hang off a session). */
int fle_inet_session_live(uint64_t h)
{
    return inet_slot(h) >= 0;
}

/* wininet!InternetOpenW: HINTERNET f(LPCWSTR lpszAgent rcx, DWORD dwAccessType edx, LPCWSTR lpszProxy r8, LPCWSTR lpszProxyBypass r9, DWORD dwFlags [5]). */
void fle_imp_InternetOpenW(void)
{
    static const char api[] = "InternetOpenW";
    uint32_t type = (uint32_t)R[2], flags = (uint32_t)fle_arg(5);

    if (R[1] == 0) fle_stop(api, "lpszAgent is NULL: the page gives no behaviour (decision 0132)");
    if (type != 0 && type != 1 && type != 4) fle_stop(api, "dwAccessType is not DIRECT, PRECONFIG or PRECONFIG_WITH_NO_AUTOPROXY: a proxy needs a model of its own (decision 0132)");
    if (R[8] != 0 || R[9] != 0) fle_stop(api, "lpszProxy or lpszProxyBypass is not NULL: the page says they are ignored unless the type is PROXY and gives no behaviour for a value (decision 0132)");
    if (flags != 0) fle_stop(api, "dwFlags is not 0: the asynchronous and the cache-only modes need a model of their own (decision 0132)");
    if (inet_n == FLE_INET_MAX) fle_stop(api, "more sessions than the runtime holds");
    inet_sess[inet_n].live = 1;
    memset(inet_sess[inet_n].timeouts, 0, sizeof inet_sess[inet_n].timeouts);
    R[0] = FLE_INET_HANDLE_BASE + 4u * (inet_n + 1u);
    inet_n++;
}

/* wininet!InternetSetOptionW: BOOL f(HINTERNET rcx, DWORD dwOption edx, LPVOID lpBuffer r8, DWORD dwBufferLength r9d). */
void fle_imp_InternetSetOptionW(void)
{
    static const char api[] = "InternetSetOptionW";
    int k = inet_slot(R[1]);
    uint32_t opt = (uint32_t)R[2], len = (uint32_t)R[9];

    if (k < 0) fle_stop(api, "hInternet is not a live session of this runtime: the page names no error code (decision 0132)");
    if ((opt != 2 && opt != 5 && opt != 6) || len != 4 || R[8] == 0) fle_stop(api, "only the time-out options CONNECT_TIMEOUT (2), SEND_TIMEOUT (5) and RECEIVE_TIMEOUT (6) with a 4-byte value are answered (decision 0132)");
    inet_sess[k].timeouts[opt == 2 ? 0 : opt == 5 ? 1 : 2] = (uint32_t)fle_ld(R[8], 4);
    R[0] = 1;
}

/* wininet!InternetOpenUrlW: HINTERNET f(HINTERNET rcx, LPCWSTR lpszUrl rdx, LPCWSTR lpszHeaders r8, DWORD dwHeadersLength r9d, DWORD dwFlags [5], DWORD_PTR dwContext [6]). */
void fle_imp_InternetOpenUrlW(void)
{
    static const char api[] = "InternetOpenUrlW";

    if (inet_slot(R[1]) < 0) fle_stop(api, "hInternet is not a live session of this runtime: the page names no error code (decision 0132)");
    if (R[2] == 0) fle_stop(api, "lpszUrl is NULL: the page gives no behaviour (decision 0132)");
    if (fle_act1_note_open_url) fle_act1_note_open_url(R[2], R[8], (uint32_t)fle_arg(5));   /* decision 0213 (act1): one record line when fle_act1_net.c is linked */
    fle_set_last_error(FLE_ERROR_INTERNET_NAME_NOT_RESOLVED);
    R[0] = 0;
}

/* wininet!InternetCloseHandle: BOOL f(HINTERNET rcx). */
void fle_imp_InternetCloseHandle(void)
{
    int k = inet_slot(R[1]);

    /* decision 0213 (act1): a connection or request handle of fle_act1_net.c (a weak reference; absent in a build without that file) */
    if (k < 0 && fle_act1_close_handle && fle_act1_close_handle(R[1])) { R[0] = 1; return; }
    if (k < 0) fle_stop("InternetCloseHandle", "hInternet is not a live session of this runtime: the page names no error code (decision 0132)");
    inet_sess[k].live = 0;
    R[0] = 1;
}
