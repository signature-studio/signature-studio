/* dwmapi additions of decision 0171 (see fle_dwm.c). */
#ifndef FLE_DWM_H
#define FLE_DWM_H

void fle_imp_DwmIsCompositionEnabled(void);
/* dwmapi!DwmExtendFrameIntoClientArea: HRESULT f(HWND rcx, const MARGINS *rdx). */
void fle_imp_DwmExtendFrameIntoClientArea(void);

/* dwmapi!DwmGetCompositionTimingInfo (decision 0302): HRESULT f(HWND rcx = NULL, DWM_TIMING_INFO *rdx). */
void fle_imp_DwmGetCompositionTimingInfo(void);

#endif
