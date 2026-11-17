/* dwmapi additions of decision 0171 (PROVISIONAL, wall ia1-W13: delay-loaded by the engine at a click on the menu bar). Convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx); the return value in R[0]. */
#include <stdint.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_dwm.h"
#include "fle_sys.h"
#include "fle_thread.h"

void fle_sys_clock_advance_to(uint64_t ticks);   /* fle_sys.c */

/* dwmapi!DwmIsCompositionEnabled: HRESULT f(BOOL *pfEnabled rcx). Learn: "Obtains a value that indicates whether Desktop Window Manager (DWM) composition is enabled"; pfEnabled "receives TRUE if DWM composition is
 * enabled; otherwise, FALSE"; S_OK on success. Remarks: "As of Windows 8, DWM composition is always enabled. If an app declares Windows 8 compatibility in their manifest, this function will receive a value of TRUE
 * through pfEnabled. If no such manifest entry is found, Windows 8 compatibility is not assumed and this function receives a value of FALSE". The host executable's manifest (FL64.exe, read from the file by this
 * work) declares supportedOS {4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38} (Windows 8) and {1f676c76-80e1-4239-95bb-83d0f6d0da78} (Windows 8.1) among its four entries, so TRUE (S-ia1-dwm-1: the manifest is the host's,
 * which this environment imitates; the GUID-to-version table is not on the page). A NULL pointer: the page gives no behaviour, the call stops. */
void fle_imp_DwmIsCompositionEnabled(void)
{
    uint64_t p = R[1];

    if (p == 0) fle_stop("DwmIsCompositionEnabled", "pfEnabled is NULL: the page gives no behaviour");
    fle_st(p, 4, 1);
    R[0] = 0; /* S_OK */
}

/* dwmapi!DwmExtendFrameIntoClientArea: HRESULT f(HWND hWnd rcx, const MARGINS *pMarInset rdx). Learn: "Extends the window frame into the client area"; pMarInset "a pointer to a MARGINS structure (cxLeftWidth,
 * cxRightWidth, cyTopHeight, cyBottomHeight: four 32-bit values) that describes the margins to use when extending the frame into the client area"; S_OK if successful, otherwise an HRESULT error code. Here: the
 * margins are kept as the window's state (decision 0244; the window model has no frame of its own, decision 0123 nc_zero, so what is drawn is the client area: fle_desk.c composes the extended-frame part of the client area with its alpha): a live
 * window of the window table and a readable MARGINS give S_OK (S-ia1-dwm-2); a window that is not live or a NULL pointer stop by name (the page names no HRESULT for them). */
void fle_imp_DwmExtendFrameIntoClientArea(void)
{
    static const char api[] = "DwmExtendFrameIntoClientArea";
    fle_win_view v;

    int32_t m[4];

    if (R[2] == 0) fle_stop(api, "pMarInset is NULL: the page gives no behaviour");
    if (!fle_win_get(R[1], &v)) fle_stop(api, "hWnd is not a window of the window table: the page names no HRESULT for it");
    /* decision 0244: MARGINS is cxLeftWidth, cxRightWidth, cyTopHeight, cyBottomHeight (four 32-bit values). The window keeps them; the compositor reads them (fle_desk.c) and the drawing of the window keeps the alpha byte (fle_gdi.c). */
    m[0] = (int32_t)fle_ld(R[2], 4);
    m[1] = (int32_t)fle_ld(R[2] + 4, 4);
    m[2] = (int32_t)fle_ld(R[2] + 8, 4);
    m[3] = (int32_t)fle_ld(R[2] + 12, 4);
    fle_win_set_frame_margins(R[1], m);
    R[0] = 0; /* S_OK */
}

/* dwmapi!DwmGetCompositionTimingInfo (decision 0302, PROVISIONAL): HRESULT f(HWND hwnd rcx, DWM_TIMING_INFO *pTimingInfo rdx). Learn (DwmGetCompositionTimingInfo): "Retrieves the current composition timing information for a specified
 * window"; hwnd: "Starting with Windows 8.1, this parameter must be set to NULL. If this parameter is not set to NULL, DwmGetCompositionTimingInfo returns E_INVALIDARG"; pTimingInfo "receives the current composition timing
 * information", "The cbSize member of this structure must be set before this function is called"; "If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code". Learn (DWM_TIMING_INFO): the members
 * and their one-line meanings (copied to docs/investigations/mt3-0302). The virtual DWM is the one DwmFlush waits for: one display of fle_display_hz() hertz, vblank k at ceil(k * 10^7 / hz) ticks of the virtual clock (fle_thread.c,
 * fle_thread_next_frame_tick), QueryPerformanceCounter = the tick count and QueryPerformanceFrequency = FLE_QPF_HZ (fle_sys.c), so every QPC_TIME below is in ticks and the values agree with what DwmFlush did: after DwmFlush returns at the
 * tick of vblank k, cRefresh is k and qpcVBlank that tick.
 * LAYOUT (S-dwm-ti-layout): the page gives the members in order and not the packing. The real caller (QuickFontCache, f_1e857b0) sets cbSize = 0x124 = 292, which is the sum of the member sizes with 1-byte packing (the SDK header
 * includes pshpack1.h; with natural alignment the structure would be 0x140 = 320), so the offsets below are the packed ones; tests/dwm_timing.rs recomputes them from the member list.
 * VALUES (S-dwm-ti-values; the page defines the members in one line each and gives no value for a machine without a GPU): the virtual DWM composes every refresh, a composed frame is displayed and confirmed at once, nothing is late,
 * dropped or missed, no DirectX present exists (no application draws through DirectX here), and the counters that Learn says count "since DwmSetPresentParameters was last called" count from the virtual boot (nothing calls it). The
 * members that Learn says are "valid only after a second call" (cFramesDisplayed, cFramesAvailable, cFramesDropped) and the "new" frame counts are the change since the previous call of this function (0 for the first). cbSize other
 * than 0x124 is E_INVALIDARG (S-dwm-ti-cbsize: the page only says it must be set; the HRESULT is the generic one of the page's own hwnd rule). A NULL pTimingInfo: the page gives no behaviour, the call stops (as DwmIsCompositionEnabled). */
#define FLE_DWM_TIMING_INFO_SIZE 0x124u

void fle_imp_DwmGetCompositionTimingInfo(void)
{
    static const char api[] = "DwmGetCompositionTimingInfo";
    static uint64_t prev_k;
    static int have_prev;
    uint64_t hwnd = R[1], p = R[2], now, hz, k, vb, period, d, qpf = FLE_QPF_HZ;

    if (hwnd != 0) {   /* Windows 8.1 and later: hwnd must be NULL */
        R[0] = 0x80070057u; /* E_INVALIDARG */
        return;
    }
    if (p == 0) fle_stop(api, "pTimingInfo is NULL: the page gives no behaviour");
    if ((uint32_t)fle_ld(p, 4) != FLE_DWM_TIMING_INFO_SIZE) {
        R[0] = 0x80070057u; /* E_INVALIDARG */
        return;
    }
    /* now = the value QueryPerformanceCounter would return; reading it moves the clock on, as every read of the clock does (decision 0017) */
    now = fle_sys_clock_ticks();
    fle_sys_clock_advance_to(now + FLE_CLOCK_READ_TICKS);
    hz = fle_display_hz ? fle_display_hz() : FLE_DWM_HZ;
    k = now * hz / qpf;                              /* the last vblank at or before now: ceil(k * qpf / hz) <= now  <=>  k <= now * hz / qpf */
    vb = (k * qpf + (hz - 1u)) / hz;                 /* its tick (the boundary DwmFlush waits for) */
    period = (qpf + hz / 2u) / hz;                   /* the refresh period in ticks, rounded to nearest */
    d = have_prev ? k - prev_k : 0;
    prev_k = k;
    have_prev = 1;
    fle_st(p + 4, 4, hz);          fle_st(p + 8, 4, 1);          /* rateRefresh (UNSIGNED_RATIO: uiNumerator, uiDenominator) */
    fle_st(p + 12, 8, period);                                   /* qpcRefreshPeriod */
    fle_st(p + 20, 4, hz);         fle_st(p + 24, 4, 1);         /* rateCompose: composed once per refresh */
    fle_st(p + 28, 8, vb);                                       /* qpcVBlank */
    fle_st(p + 36, 8, k);                                        /* cRefresh */
    fle_st(p + 44, 4, k);                                        /* cDXRefresh (UINT: the low 32 bits) */
    fle_st(p + 48, 8, vb);                                       /* qpcCompose: the composition pass is at the vblank */
    fle_st(p + 56, 8, k);                                        /* cFrame */
    fle_st(p + 64, 4, 0);                                        /* cDXPresent: no DirectX present */
    fle_st(p + 68, 8, k);                                        /* cRefreshFrame */
    fle_st(p + 76, 8, k);                                        /* cFrameSubmitted */
    fle_st(p + 84, 4, 0);                                        /* cDXPresentSubmitted */
    fle_st(p + 88, 8, k);                                        /* cFrameConfirmed */
    fle_st(p + 96, 4, 0);                                        /* cDXPresentConfirmed */
    fle_st(p + 100, 8, k);                                       /* cRefreshConfirmed */
    fle_st(p + 108, 4, k);                                       /* cDXRefreshConfirmed (UINT) */
    fle_st(p + 112, 8, 0);                                       /* cFramesLate */
    fle_st(p + 120, 4, 0);                                       /* cFramesOutstanding (UINT) */
    fle_st(p + 124, 8, k);                                       /* cFrameDisplayed */
    fle_st(p + 132, 8, vb);                                      /* qpcFrameDisplayed */
    fle_st(p + 140, 8, k);                                       /* cRefreshFrameDisplayed */
    fle_st(p + 148, 8, k);                                       /* cFrameComplete */
    fle_st(p + 156, 8, vb);                                      /* qpcFrameComplete */
    fle_st(p + 164, 8, k);                                       /* cFramePending */
    fle_st(p + 172, 8, vb);                                      /* qpcFramePending */
    fle_st(p + 180, 8, d);                                       /* cFramesDisplayed: the unique frames since the previous call */
    fle_st(p + 188, 8, d);                                       /* cFramesComplete: the new completed frames since the previous call */
    fle_st(p + 196, 8, 0);                                       /* cFramesPending */
    fle_st(p + 204, 8, 0);                                       /* cFramesAvailable */
    fle_st(p + 212, 8, 0);                                       /* cFramesDropped */
    fle_st(p + 220, 8, 0);                                       /* cFramesMissed */
    fle_st(p + 228, 8, k + 1u);                                  /* cRefreshNextDisplayed */
    fle_st(p + 236, 8, k + 1u);                                  /* cRefreshNextPresented */
    fle_st(p + 244, 8, k);                                       /* cRefreshesDisplayed: since the virtual boot */
    fle_st(p + 252, 8, 0);                                       /* cRefreshesPresented */
    fle_st(p + 260, 8, 0);                                       /* cRefreshStarted */
    fle_st(p + 268, 8, 0);                                       /* cPixelsReceived */
    fle_st(p + 276, 8, 0);                                       /* cPixelsDrawn */
    fle_st(p + 284, 8, 0);                                       /* cBuffersEmpty */
    R[0] = 0; /* S_OK */
}
