/* The virtual machine has no network connection (decision 0126, PROVISIONAL, S-inet-offline): wininet!InternetGetConnectedState returns FALSE with no flag set. Nothing else of wininet is answered. */
#ifndef FLE_WININET_H
#define FLE_WININET_H

#include <stdint.h>

void fle_imp_InternetGetConnectedState(void);

/* Decision 0132 (PROVISIONAL), wall 23: an OFFLINE session (fle_wininet.c): InternetOpenW makes a handle, InternetSetOptionW keeps the three time-outs, InternetOpenUrlW fails with
 * ERROR_INTERNET_NAME_NOT_RESOLVED (S-inet-open-url-fails), InternetCloseHandle closes it. Everything else of wininet stays unanswered. */
void fle_imp_InternetOpenW(void);
void fle_imp_InternetSetOptionW(void);
void fle_imp_InternetOpenUrlW(void);
void fle_imp_InternetCloseHandle(void);

/* Decision 0213 (act1): 1 when h is a live session handle made by InternetOpenW (for fle_act1_net.c) */
int fle_inet_session_live(uint64_t h);

#endif
