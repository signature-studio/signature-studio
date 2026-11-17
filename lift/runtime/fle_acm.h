/* Decision 0229 (ref1, PROVISIONAL): msacm32 (Audio Compression Manager) with one driver, an MPEG Layer-3 decoder (see fle_acm.c). */
#ifndef FLE_ACM_H
#define FLE_ACM_H
void fle_imp_acm_DriverEnum(void);
void fle_imp_acm_DriverOpen(void);
void fle_imp_acm_DriverClose(void);
void fle_imp_acm_DriverDetailsA(void);
void fle_imp_acm_FormatChooseW(void);
void fle_imp_acm_FormatSuggest(void);
void fle_imp_acm_StreamOpen(void);
void fle_imp_acm_StreamClose(void);
void fle_imp_acm_StreamSize(void);
void fle_imp_acm_StreamPrepareHeader(void);
void fle_imp_acm_StreamUnprepareHeader(void);
void fle_imp_acm_StreamConvert(void);
void fle_imp_acm_StreamMessage(void);
#endif
