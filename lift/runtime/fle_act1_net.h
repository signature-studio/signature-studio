/* Decision 0213 (act1, PROVISIONAL): the rest of the WinINet calls that FLEngine's own HTTP code makes (the unit around f_64d520: license, account, FL Cloud, analytics), answered as "this machine
 * cannot reach the server" with the DOCUMENTED failure, instead of a stop. NOT A NETWORK MODEL and NEVER a success: no request is ever sent, no response ever exists. Same convention as
 * fle_wininet.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), then fle_arg(5..); the Win32 result in R[0]; the last error through fle_set_last_error.
 * Each function is reached only through a row of harness/seeds/act1-imports.txt (layer 5: wininet.dll <name> fle_imp_<name>); a build without the rows is unchanged (the calls stop at the import thunk as before).
 * Every call writes one tab-separated record on stderr ("fle: act1-net<TAB>...") with the host, port, verb, object name, the NAMES of the query parameters and of the request headers and the byte counts, and NEVER
 * a header value, a query value or a body byte: the engine puts account e-mail, password and tokens there (decision 0213 section 4). */
#ifndef FLE_ACT1_NET_H
#define FLE_ACT1_NET_H

#include <stdint.h>

void fle_imp_InternetConnectW(void);
void fle_imp_HttpOpenRequestW(void);
void fle_imp_HttpAddRequestHeadersW(void);
void fle_imp_HttpSendRequestExW(void);
void fle_imp_InternetWriteFile(void);
void fle_imp_HttpEndRequestW(void);
void fle_imp_HttpQueryInfoW(void);
void fle_imp_HttpQueryInfoA(void);
void fle_imp_InternetReadFile(void);
void fle_imp_InternetGetLastResponseInfoW(void);

/* Called by fle_wininet.c's InternetCloseHandle (a weak reference): 1 when h is a connection or request handle of this file (it is closed), 0 when it is not one of ours. */
int fle_act1_close_handle(uint64_t h);

/* Called by fle_wininet.c's InternetOpenUrlW (a weak reference): writes one record (the URL's query values replaced by their lengths). */
void fle_act1_note_open_url(uint64_t url, uint64_t headers, uint32_t flags);

#endif
