/* Decision 0136 (PROVISIONAL): the process access token, only as far as the guest asks (fle_token.c): advapi32!OpenProcessToken and advapi32!GetTokenInformation(TokenElevation). The guest's
 * "is the process elevated" check (f_10e6260) is the caller. Everything else of the token API stays unanswered. */
#ifndef FLE_TOKEN_H
#define FLE_TOKEN_H

void fle_imp_OpenProcessToken(void);
void fle_imp_GetTokenInformation(void);

#endif
