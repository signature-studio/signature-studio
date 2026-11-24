/* The WebView2 loader of the virtual machine (Issue #1355). See fle_webview2.c. The module webview2loader.dll (fle_modules.c, N = 42) answers GetProcAddress with the rows of fle_imports.c's TABLE. */
#pragma once

#include <stdint.h>

/* The pseudo-VAs of the method slots of the modelled COM interfaces (S-wv2-slot-range): FLE_WV2_SLOT_BASE + FLE_WV2_SLOT_STEP * slot. The range lies above the response table's rows
 * (0x7FF900000010 + 0x10 * k) and below the DirectSound slots (0x7FF940000000, fle_dsound.h). */
#define FLE_WV2_SLOT_BASE UINT64_C(0x7FF950000000)
#define FLE_WV2_SLOT_STEP UINT64_C(0x10)

/* webview2loader.dll!CreateCoreWebView2EnvironmentWithOptions, CreateCoreWebView2Environment, GetAvailableCoreWebView2BrowserVersionString, CompareBrowserVersions: rows of the response table. */
void fle_imp_CreateCoreWebView2EnvironmentWithOptions(void);
void fle_imp_CreateCoreWebView2Environment(void);
void fle_imp_GetAvailableCoreWebView2BrowserVersionString(void);
void fle_imp_CompareBrowserVersions(void);

/* The method slot hook of fle_win_try_thunk (fle_win.c): 1 and the method has run when `va` is a slot of this model, else 0. */
int fle_wv2_try_slot(uint64_t va);

/* Issue #1362: the page has loaded the document of a Navigate (the engine's export fle_g3_wv2_loaded), and the engine's idle fires the navigation's events
 * (fle_wv2_pump, called by fle_thread_idle_hook before it waits). */
void fle_wv2_page_loaded(void);
void fle_wv2_pump(void);
