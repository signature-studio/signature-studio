/* The WebView2 loader of the virtual machine (Issue #1355, the first step of the Browser panel: a record of what FLEngine hands to WebView2).
 *
 * FLEngine's Browser panel is a TEdgeBrowser, Delphi's wrapper of WebView2. The wrapper loads WebView2Loader.dll with LoadLibraryW and takes
 * CreateCoreWebView2EnvironmentWithOptions from it by GetProcAddress. The module webview2loader.dll is a virtual module (fle_modules.c, N = 42), and its
 * four exports are rows of the response table (fle_imports.c's TABLE). This file answers them, and the COM interfaces that the wrapper calls:
 * ICoreWebView2Environment, ICoreWebView2Controller, ICoreWebView2 with its ICoreWebView2Settings, and the event arguments of the four events that a navigation
 * fires (NavigationStarting, SourceChanged, ContentLoading, NavigationCompleted). Nothing is drawn: the model answers, and it writes what the guest hands over.
 *
 * Each call of a modelled method writes one line to the engine's stderr: `webview2: <method> (<interface>) <arguments>`. The arguments are the URL, the HTML
 * (its length in UTF-16 units and its first 300 units), the script, the message, the bounds and the values. After 600 lines the log stops with one line of
 * its own ("fle: webview2 log stops after 600 lines").
 *
 * The vtable slots are pseudo-VAs of a range of their own (FLE_WV2_SLOT_BASE + FLE_WV2_SLOT_STEP * slot, S-wv2-slot-range, as fle_dsound.c's). The guest's
 * own `call [reg+disp]` leaves in fle_dispatch_miss, which asks fle_win_try_thunk (fle_win.c), which asks fle_wv2_try_slot. The slot order is the order of the
 * WebView2 interfaces as the go-webview2 package's vtables give it (ICoreWebView2, ICoreWebView2Controller, ICoreWebView2Environment, ICoreWebView2Settings,
 * and their IDL order, which those vtables follow). A completion handler is called at once, by the call that asks for it (fle_call_guest): the model has no
 * message loop of its own (CHOICE S-wv2-sync).
 *
 * The first navigation is about:blank, made when the controller is ready: after the controller's completed handler has returned, the four events of a navigation
 * (NavigationStarting, SourceChanged, ContentLoading, NavigationCompleted, in that order: the order the ICoreWebView2 docs give) are called on the handlers the page
 * registered (a handler that is not registered is skipped). The docs do not say that the first about:blank raises them; that is unconfirmed (webview2-calls-1355.md).
 * A Navigate or NavigateToString after that is logged (its URI or HTML) and fires nothing; the page stays as it was (CHOICE, #1355). The reason is the probe of
 * FL's Browser tab (tools/browser-panel/probe-navigate.mjs): the cloud tab navigates to FL's error-offline.html, FL's NavigationStarting handler then calls a
 * window procedure that is a MakeObjectInstance thunk on a page the runtime does not run (decision 0119: not called), and the engine exits. The four events of
 * the first about:blank are fired (their arguments: IsSuccess TRUE, WebErrorStatus 0, NavigationId 1, IsNewDocument TRUE, IsErrorPage FALSE, Cancel FALSE,
 * RequestHeaders with no header: CHOICE). Each fired event is one log line: `webview2: fire <event> (<interface>) <arguments>`.
 * Choices without a source (each is a CHOICE): the runtime version is "131.0.0.0"; ExecuteScript completes with the JSON "null";
 * AddScriptToExecuteOnDocumentCreated completes with the id "1"; the zoom factor reads 1.0; the settings read TRUE and the counts read 0; a method that the
 * model does not answer returns E_NOTIMPL. A QueryInterface for an IID other than IUnknown returns E_NOINTERFACE (the IID is in the log). The objects live as long
 * as the run; the strings are CoTaskMemAlloc blocks (fle_ole.c's task memory) that the guest frees with CoTaskMemFree, as the docs say.
 * The vtables of the request headers and of their iterator follow the go-webview2 package's (wailsapp/go-webview2, pkg/edge); the vtables of the event arguments of
 * NavigationStarting, SourceChanged and ContentLoading follow the IDL order as the author recalls it (unconfirmed: pm-work/findings/webview2-calls-1355.md).
 *
 * Plain C: the guest memory is the only host state the model reads and writes (fle_ld / fle_st). */
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_ole.h"
#include "fle_webview2.h"

uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9) __attribute__((weak));
/* Issue #1362 (fle_win.c): the screen position of the client area of a window of the table; 0 when `h` is not a live window */
int fle_win_client_origin(uint64_t h, int32_t *ox, int32_t *oy) __attribute__((weak));
/* Issue #1376 (fle_win.c): the windows painted over window h within [l, r) x [t, b) (screen coordinates), 4 words each into out; weak: a link without fle_win.c has none */
unsigned fle_win_front_rects(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b, int32_t *out, unsigned cap) __attribute__((weak));

#define WV_S_OK           0x00000000u
#define WV_E_NOINTERFACE  0x80004002u
#define WV_E_NOTIMPL      0x80004001u
#define WV_E_INVALIDARG   0x80070057u
#define WV_LOG_MAX        600u
#define WV_HEAD           300u           /* units of a long text that the log shows */
#define WV_TEXT_MAX       (WV_HEAD * 6 + 16)   /* a unit that is written as \uXXXX takes six characters */
#define WV_SCAN_MAX       (1u << 22)     /* a guest text longer than this is read up to here */
#define WV_SLOTS_MAX      64u
#define WV_VERSION        "131.0.0.0"
#define IID_IUNKNOWN      "{00000000-0000-0000-c000-000000000046}"

enum { IF_ENV, IF_CTRL, IF_VIEW, IF_SET, IF_NAVARGS, IF_NAVSTART, IF_SRC, IF_CONTENT, IF_HEADERS, IF_HITER, IF_COUNT };
static const char *const iface_name[IF_COUNT] = {
    "ICoreWebView2Environment", "ICoreWebView2Controller", "ICoreWebView2", "ICoreWebView2Settings", "ICoreWebView2NavigationCompletedEventArgs",
    "ICoreWebView2NavigationStartingEventArgs", "ICoreWebView2SourceChangedEventArgs", "ICoreWebView2ContentLoadingEventArgs", "ICoreWebView2HttpRequestHeaders",
    "ICoreWebView2HttpHeadersCollectionIterator",
};

/* the events of a navigation that the model fires, and the view's slot of each handler */
enum { EV_NAV_STARTING, EV_SOURCE, EV_CONTENT, EV_NAV_DONE, EV_COUNT };

/* what a slot does (the arguments are the Microsoft x64 ones: this in R[1], the rest in R[2], R[8], R[9]) */
enum {
    K_QI, K_ADDREF, K_RELEASE,                       /* IUnknown */
    K_NOTIMPL, K_NOP,                                /* E_NOTIMPL / S_OK, the arguments are only logged */
    K_CREATE_CTRL,                                   /* ICoreWebView2Environment::CreateCoreWebView2Controller */
    K_GET_WSTR, K_GET_OBJ, K_GET_VISIBLE, K_PUT_VISIBLE, K_GET_BOOL, K_PUT_BOOL, K_GET_U32, K_GET_U64,
    K_GET_RECT, K_PUT_RECT, K_GET_HWND, K_PUT_HWND, K_GET_DBL, K_PUT_DBL, K_GET_URI,
    K_HDR_VALUE, K_HDR_CONTAINS,                     /* header lookups by name: the name is R[2], the result is written through R[8] */
    K_ADD, K_ADD_EV, K_ADD_SCRIPT, K_REMOVE,         /* event registrations (a token is written; K_ADD_EV keeps the handler: arg is its EV_) and their removal */
    K_NAVIGATE, K_NAV_HTML, K_EXEC, K_POST,
};

struct slot {
    const char *name;     /* the method's name (IDL) */
    unsigned kind;
    unsigned arg;         /* K_GET_BOOL / K_GET_U32 / K_GET_U64: the value; K_GET_OBJ: the interface of the result; K_ADD_EV: the EV_ */
    const char *text;     /* K_GET_WSTR: the text */
};

#define IU { "QueryInterface", K_QI, 0, 0 }, { "AddRef", K_ADDREF, 0, 0 }, { "Release", K_RELEASE, 0, 0 }
#define SB(n) { "get_" n, K_GET_BOOL, 1, 0 }, { "put_" n, K_PUT_BOOL, 0, 0 }

/* ICoreWebView2Environment: IUnknown, then the vtable of WebView2.idl (slots 3..7) */
static const struct slot tab_env[] = {
    IU,
    { "CreateCoreWebView2Controller", K_CREATE_CTRL, 0, 0 },
    { "CreateWebResourceResponse", K_NOTIMPL, 0, 0 },
    { "get_BrowserVersionString", K_GET_WSTR, 0, WV_VERSION },
    { "add_NewBrowserVersionAvailable", K_ADD, 0, 0 },
    { "remove_NewBrowserVersionAvailable", K_REMOVE, 0, 0 },
};

/* ICoreWebView2Controller (slots 3..25) */
static const struct slot tab_ctrl[] = {
    IU,
    { "get_IsVisible", K_GET_VISIBLE, 0, 0 },
    { "put_IsVisible", K_PUT_VISIBLE, 0, 0 },
    { "get_Bounds", K_GET_RECT, 0, 0 },
    { "put_Bounds", K_PUT_RECT, 0, 0 },
    { "get_ZoomFactor", K_GET_DBL, 0, 0 },
    { "put_ZoomFactor", K_PUT_DBL, 0, 0 },
    { "add_ZoomFactorChanged", K_ADD, 0, 0 },
    { "remove_ZoomFactorChanged", K_REMOVE, 0, 0 },
    { "SetBoundsAndZoomFactor", K_NOP, 0, 0 },
    { "MoveFocus", K_NOP, 0, 0 },
    { "add_MoveFocusRequested", K_ADD, 0, 0 },
    { "remove_MoveFocusRequested", K_REMOVE, 0, 0 },
    { "add_GotFocus", K_ADD, 0, 0 },
    { "remove_GotFocus", K_REMOVE, 0, 0 },
    { "add_LostFocus", K_ADD, 0, 0 },
    { "remove_LostFocus", K_REMOVE, 0, 0 },
    { "add_AcceleratorKeyPressed", K_ADD, 0, 0 },
    { "remove_AcceleratorKeyPressed", K_REMOVE, 0, 0 },
    { "get_ParentWindow", K_GET_HWND, 0, 0 },
    { "put_ParentWindow", K_PUT_HWND, 0, 0 },
    { "NotifyParentWindowPositionChanged", K_NOP, 0, 0 },
    { "Close", K_NOP, 0, 0 },
    { "get_CoreWebView2", K_GET_OBJ, IF_VIEW, 0 },
};

/* ICoreWebView2 (slots 3..60) */
static const struct slot tab_view[] = {
    IU,
    { "get_Settings", K_GET_OBJ, IF_SET, 0 },
    { "get_Source", K_GET_URI, 0, 0 },
    { "Navigate", K_NAVIGATE, 0, 0 },
    { "NavigateToString", K_NAV_HTML, 0, 0 },
    { "add_NavigationStarting", K_ADD_EV, EV_NAV_STARTING, 0 },
    { "remove_NavigationStarting", K_REMOVE, 0, 0 },
    { "add_ContentLoading", K_ADD_EV, EV_CONTENT, 0 },
    { "remove_ContentLoading", K_REMOVE, 0, 0 },
    { "add_SourceChanged", K_ADD_EV, EV_SOURCE, 0 },
    { "remove_SourceChanged", K_REMOVE, 0, 0 },
    { "add_HistoryChanged", K_ADD, 0, 0 },
    { "remove_HistoryChanged", K_REMOVE, 0, 0 },
    { "add_NavigationCompleted", K_ADD_EV, EV_NAV_DONE, 0 },
    { "remove_NavigationCompleted", K_REMOVE, 0, 0 },
    { "add_FrameNavigationStarting", K_ADD, 0, 0 },
    { "remove_FrameNavigationStarting", K_REMOVE, 0, 0 },
    { "add_FrameNavigationCompleted", K_ADD, 0, 0 },
    { "remove_FrameNavigationCompleted", K_REMOVE, 0, 0 },
    { "add_ScriptDialogOpening", K_ADD, 0, 0 },
    { "remove_ScriptDialogOpening", K_REMOVE, 0, 0 },
    { "add_PermissionRequested", K_ADD, 0, 0 },
    { "remove_PermissionRequested", K_REMOVE, 0, 0 },
    { "add_ProcessFailed", K_ADD, 0, 0 },
    { "remove_ProcessFailed", K_REMOVE, 0, 0 },
    { "AddScriptToExecuteOnDocumentCreated", K_ADD_SCRIPT, 0, 0 },
    { "RemoveScriptToExecuteOnDocumentCreated", K_NOP, 0, 0 },
    { "ExecuteScript", K_EXEC, 0, 0 },
    { "CapturePreview", K_NOTIMPL, 0, 0 },
    { "Reload", K_NOP, 0, 0 },
    { "PostWebMessageAsJson", K_POST, 0, 0 },
    { "PostWebMessageAsString", K_POST, 0, 0 },
    { "add_WebMessageReceived", K_ADD, 0, 0 },
    { "remove_WebMessageReceived", K_REMOVE, 0, 0 },
    { "CallDevToolsProtocolMethod", K_NOTIMPL, 0, 0 },
    { "get_BrowserProcessId", K_GET_U32, 0, 0 },
    { "get_CanGoBack", K_GET_BOOL, 0, 0 },
    { "get_CanGoForward", K_GET_BOOL, 0, 0 },
    { "GoBack", K_NOP, 0, 0 },
    { "GoForward", K_NOP, 0, 0 },
    { "GetDevToolsProtocolEventReceiver", K_NOTIMPL, 0, 0 },
    { "Stop", K_NOP, 0, 0 },
    { "add_NewWindowRequested", K_ADD, 0, 0 },
    { "remove_NewWindowRequested", K_REMOVE, 0, 0 },
    { "add_DocumentTitleChanged", K_ADD, 0, 0 },
    { "remove_DocumentTitleChanged", K_REMOVE, 0, 0 },
    { "get_DocumentTitle", K_GET_WSTR, 0, "" },
    { "AddHostObjectToScript", K_NOP, 0, 0 },
    { "RemoveHostObjectFromScript", K_NOP, 0, 0 },
    { "OpenDevToolsWindow", K_NOP, 0, 0 },
    { "add_ContainsFullScreenElementChanged", K_ADD, 0, 0 },
    { "remove_ContainsFullScreenElementChanged", K_REMOVE, 0, 0 },
    { "get_ContainsFullScreenElement", K_GET_BOOL, 0, 0 },
    { "add_WebResourceRequested", K_ADD, 0, 0 },
    { "remove_WebResourceRequested", K_REMOVE, 0, 0 },
    { "AddWebResourceRequestedFilter", K_NOP, 0, 0 },
    { "RemoveWebResourceRequestedFilter", K_NOP, 0, 0 },
    { "add_WindowCloseRequested", K_ADD, 0, 0 },
    { "remove_WindowCloseRequested", K_REMOVE, 0, 0 },
};

/* ICoreWebView2Settings (slots 3..32): the defaults of WebView2 read TRUE; the user agent is the runtime's own (empty here) */
static const struct slot tab_set[] = {
    IU,
    SB("IsScriptEnabled"), SB("IsWebMessageEnabled"), SB("AreDefaultScriptDialogsEnabled"), SB("IsStatusBarEnabled"), SB("AreDevToolsEnabled"),
    SB("AreDefaultContextMenusEnabled"), SB("AreHostObjectsAllowed"), SB("IsZoomControlEnabled"), SB("IsBuiltInErrorPageEnabled"),
    { "get_UserAgent", K_GET_WSTR, 0, "" },
    { "put_UserAgent", K_POST, 0, 0 },
    SB("AreBrowserAcceleratorKeysEnabled"), SB("IsPasswordAutosaveEnabled"), SB("IsGeneralAutofillEnabled"), SB("IsPinchZoomEnabled"),
    SB("IsSwipeNavigationEnabled"),
};

/* ICoreWebView2NavigationCompletedEventArgs (slots 3..5) */
static const struct slot tab_navargs[] = {
    IU,
    { "get_IsSuccess", K_GET_BOOL, 1, 0 },
    { "get_WebErrorStatus", K_GET_U32, 0, 0 },
    { "get_NavigationId", K_GET_U64, 1, 0 },
};

/* ICoreWebView2NavigationStartingEventArgs (slots 3..9, in the IDL's order) */
static const struct slot tab_navstart[] = {
    IU,
    { "get_Uri", K_GET_URI, 0, 0 },
    { "get_IsUserInitiated", K_GET_BOOL, 0, 0 },
    { "get_IsRedirected", K_GET_BOOL, 0, 0 },
    { "get_RequestHeaders", K_GET_OBJ, IF_HEADERS, 0 },
    { "get_Cancel", K_GET_BOOL, 0, 0 },
    { "put_Cancel", K_PUT_BOOL, 0, 0 },
    { "get_NavigationId", K_GET_U64, 1, 0 },
};

/* ICoreWebView2SourceChangedEventArgs (slot 3) */
static const struct slot tab_src[] = {
    IU,
    { "get_IsNewDocument", K_GET_BOOL, 1, 0 },
};

/* ICoreWebView2ContentLoadingEventArgs (slots 3..4) */
static const struct slot tab_content[] = {
    IU,
    { "get_IsErrorPage", K_GET_BOOL, 0, 0 },
    { "get_NavigationId", K_GET_U64, 1, 0 },
};

/* ICoreWebView2HttpRequestHeaders (slots 3..8, the order of the go-webview2 package's vtable, which follows the IDL): the request's headers are none (CHOICE:
 * a header reads as "", none is contained); GetHeaders is not answered (E_NOTIMPL). GetIterator gives an empty iterator (the FL's loop ends at once). */
static const struct slot tab_headers[] = {
    IU,
    { "GetHeader", K_HDR_VALUE, 0, 0 },
    { "GetHeaders", K_NOTIMPL, 0, 0 },
    { "Contains", K_HDR_CONTAINS, 0, 0 },
    { "SetHeader", K_NOP, 0, 0 },
    { "RemoveHeader", K_NOP, 0, 0 },
    { "GetIterator", K_GET_OBJ, IF_HITER, 0 },
};

/* ICoreWebView2HttpHeadersCollectionIterator (slots 3..5, the same source): empty, so get_HasCurrentHeader and MoveNext read FALSE; GetCurrentHeader is not
 * answered (E_NOTIMPL: it fails when there is no current header, as the docs say) */
static const struct slot tab_hiter[] = {
    IU,
    { "GetCurrentHeader", K_NOTIMPL, 0, 0 },
    { "get_HasCurrentHeader", K_GET_BOOL, 0, 0 },
    { "MoveNext", K_GET_BOOL, 0, 0 },
};

static const struct slot *const tabs[IF_COUNT] = {
    tab_env, tab_ctrl, tab_view, tab_set, tab_navargs, tab_navstart, tab_src, tab_content, tab_headers, tab_hiter,
};
static const unsigned ntabs[IF_COUNT] = {
    sizeof tab_env / sizeof tab_env[0], sizeof tab_ctrl / sizeof tab_ctrl[0], sizeof tab_view / sizeof tab_view[0],
    sizeof tab_set / sizeof tab_set[0], sizeof tab_navargs / sizeof tab_navargs[0], sizeof tab_navstart / sizeof tab_navstart[0],
    sizeof tab_src / sizeof tab_src[0], sizeof tab_content / sizeof tab_content[0], sizeof tab_headers / sizeof tab_headers[0],
    sizeof tab_hiter / sizeof tab_hiter[0],
};

static uint64_t vtab[IF_COUNT][WV_SLOTS_MAX];   /* the vtables the guest reads: the slot pseudo-VAs */
static unsigned slot_base[IF_COUNT];
static unsigned slot_total;
static int vt_ready;

static void vt_init(void)
{
    unsigned i, k, b = 0;

    if (vt_ready) return;
    for (i = 0; i < IF_COUNT; i++) {
        if (ntabs[i] > WV_SLOTS_MAX) fle_stop("WebView2", "a vtable of the model is larger than WV_SLOTS_MAX");
        slot_base[i] = b;
        for (k = 0; k < ntabs[i]; k++) vtab[i][k] = FLE_WV2_SLOT_BASE + FLE_WV2_SLOT_STEP * (uint64_t)(b + k);
        b += ntabs[i];
    }
    slot_total = b;
    vt_ready = 1;
}

/* a COM object: its first 8 bytes are the vtable pointer (the guest reads them); the rest is the model's state of the object */
typedef struct wv2_obj {
    uint64_t vtbl;
    uint32_t refs;
    uint32_t iface;
    uint32_t visible;          /* controller: IsVisible */
    uint64_t hwnd;             /* controller: ParentWindow */
    int32_t bounds[4];         /* controller: Bounds (left, top, right, bottom) */
    struct wv2_obj *view;      /* controller: its CoreWebView2, made by get_CoreWebView2 */
    uint64_t ev[EV_COUNT];     /* view: the handlers of the events of a navigation (0 when none) */
    uint64_t uri;              /* view: the URI of the last navigation; NavigationStarting args: its URI. A UTF-16 copy (0 when none) */
    uint64_t nav_uri;          /* view (Issue #1362): the URI of the navigation that Navigate asked for, a UTF-16 copy (0 when none) */
    int nav_pending;           /* view (Issue #1362): the navigation has been asked and its events are not fired yet */
} wv2_obj;

/* Issue #1376: the window the face is put on (the controller's parent) and the bounds its controller was last given: fle_webview2_front reports the windows painted over them */
static uint64_t face_hwnd;
static int32_t face_bounds[4];
static int face_set;

static wv2_obj *new_obj(unsigned iface)
{
    wv2_obj *o;

    vt_init();
    o = calloc(1, sizeof *o);
    if (o == NULL) fle_stop("WebView2", "no memory for a WebView2 object");
    o->vtbl = (uint64_t)(uintptr_t)vtab[iface];
    o->refs = 1;
    o->iface = iface;
    o->visible = 1;
    return o;
}

static unsigned wv_lines;
static uint64_t wv_token;
/* Issue #1362: the view that the controller gave (get_CoreWebView2), and whether the page has loaded the document of its navigation (fle_wv2_page_loaded) */
static wv2_obj *wv_view;
static int wv_loaded;

/* one line of the log: `webview2: <method> (<interface>) <arguments>`; after WV_LOG_MAX lines the log stops */
static void wlog(const char *method, const char *iface, const char *fmt, ...)
{
    va_list ap;

    if (++wv_lines > WV_LOG_MAX) {
        if (wv_lines == WV_LOG_MAX + 1) fprintf(stderr, "fle: webview2 log stops after %u lines\n", (unsigned)WV_LOG_MAX);
        return;
    }
    fprintf(stderr, "webview2: %s (%s)", method, iface);
    if (fmt[0] != 0) {
        fputc(' ', stderr);
        va_start(ap, fmt);
        vfprintf(stderr, fmt, ap);
        va_end(ap);
    }
    fputc('\n', stderr);
}

/* a guest UTF-16 text: its first WV_HEAD units into out (the units outside 0x20..0x7E and the quote as \uXXXX, the backslash doubled, "..." when cut), its length in units into *total */
static void text_of(uint64_t p, char *out, size_t cap, size_t *total)
{
    size_t k, o = 0;
    char tmp[8];

    *total = 0;
    out[0] = 0;
    if (p == 0) {
        snprintf(out, cap, "NULL");
        return;
    }
    for (k = 0; k < WV_SCAN_MAX; k++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * k, 2);
        if (c == 0) break;
        if (k < WV_HEAD) {
            if (c == '\\') {
                strcpy(tmp, "\\\\");
            } else if (c >= 0x20 && c < 0x7f && c != '"') {
                tmp[0] = (char)c;
                tmp[1] = 0;
            } else {
                snprintf(tmp, sizeof tmp, "\\u%04x", (unsigned)c);
            }
            if (o + strlen(tmp) + 4 < cap) {
                strcpy(out + o, tmp);
                o += strlen(tmp);
            }
        }
        *total = k + 1;
    }
    if (*total > WV_HEAD) strcpy(out + o, "...");
}

/* an IID as its text ({8-4-4-4-12} in lower case) */
static void guid_of(uint64_t p, char *out, size_t cap)
{
    if (p == 0) {
        snprintf(out, cap, "NULL");
        return;
    }
    snprintf(out, cap, "{%08x-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x}", (unsigned)fle_ld(p, 4), (unsigned)fle_ld(p + 4, 2), (unsigned)fle_ld(p + 6, 2),
             (unsigned)fle_ld(p + 8, 1), (unsigned)fle_ld(p + 9, 1), (unsigned)fle_ld(p + 10, 1), (unsigned)fle_ld(p + 11, 1), (unsigned)fle_ld(p + 12, 1),
             (unsigned)fle_ld(p + 13, 1), (unsigned)fle_ld(p + 14, 1), (unsigned)fle_ld(p + 15, 1));
}

/* a block of the task memory: CoTaskMemAlloc of the guest-facing stub (fle_ole.c), so that the guest's CoTaskMemFree frees it (the docs: the caller of a method that
 * returns a string frees it with CoTaskMemFree; a block that is not from CoTaskMemAlloc stops the run) */
static uint16_t *task_wstr(size_t units)
{
    uint64_t save[3] = { R[0], R[1], R[4] }, p;

    R[1] = (uint64_t)(units * sizeof(uint16_t));
    fle_imp_CoTaskMemAlloc();
    p = R[0];
    R[0] = save[0]; R[1] = save[1]; R[4] = save[2];
    if (p == 0) fle_stop("WebView2", "no task memory for a string of the model");
    return (uint16_t *)(uintptr_t)p;
}

/* a new UTF-16 string for the guest (a task-memory block) */
static uint64_t wstr_new(const char *s)
{
    size_t n = strlen(s), k;
    uint16_t *w = task_wstr(n + 1);

    for (k = 0; k <= n; k++) w[k] = (uint16_t)(unsigned char)s[k];
    return (uint64_t)(uintptr_t)w;
}

/* a copy of a guest UTF-16 text (the guest's own may be freed after the call); 0 for 0 */
static uint64_t utf16_copy(uint64_t p)
{
    size_t k, n = 0;
    uint16_t *w;

    if (p == 0) return 0;
    while (n < WV_SCAN_MAX && fle_ld(p + 2u * n, 2) != 0) n++;
    w = task_wstr(n + 1);
    for (k = 0; k < n; k++) w[k] = (uint16_t)fle_ld(p + 2u * k, 2);
    w[n] = 0;
    return (uint64_t)(uintptr_t)w;
}

/* A completion handler of the guest (ICoreWebView2...CompletedHandler, or the NavigationCompleted handler): its Invoke is the slot after IUnknown (slot 3),
 * called as Invoke(this, a, b) through fle_call_guest. The guest's own vtable gives the function. */
static void complete(uint64_t handler, uint64_t a, uint64_t b)
{
    uint64_t vt, fn;

    if (handler == 0) return;
    if (fle_call_guest == NULL) fle_stop("WebView2", "the guest callback mechanism is not linked: a completion handler cannot be called");
    vt = fle_ld(handler, 8);
    fn = fle_ld(vt + 3u * 8u, 8);
    fle_call_guest(fn, handler, a, b, 0);
}

/* Issue #1362: a handler that is registered (add_*) is kept by WebView2 until it is removed: the object holds a reference, which the guest's own release does not drop. Without it the
 * guest frees the handler when its own reference ends (FL's local interface variable), and the model then calls freed memory: the vtable reads 0, or the thunk of a freed block (the
 * page does not run it). The reference is the IUnknown::AddRef of the handler (slot 1 of its vtable). A removal is not counted back: the handler then lives as long as the run (a choice). */
static void hold_handler(uint64_t handler)
{
    uint64_t vt, fn;

    if (handler == 0 || fle_call_guest == NULL) return;
    vt = fle_ld(handler, 8);
    fn = fle_ld(vt + 1u * 8u, 8);
    fle_call_guest(fn, handler, 0, 0, 0);
}

/* a navigation of the view to `uri` (a UTF-16 copy) has ended at once: its four events, in WebView2's order, each to the handler the page registered (a handler
 * that is not registered is skipped, and its line is not written). Each fired event writes `webview2: fire <event> (<interface>) <arguments>` */
static void fire_navigation(wv2_obj *v, uint64_t uri)
{
    char t[WV_TEXT_MAX];
    size_t n = 0;
    wv2_obj *a;

    v->uri = uri;
    text_of(uri, t, sizeof t, &n);
    if (v->ev[EV_NAV_STARTING] != 0) {
        a = new_obj(IF_NAVSTART);
        a->uri = uri;
        wlog("fire NavigationStarting", "ICoreWebView2", "uri=\"%s\"", t);
        complete(v->ev[EV_NAV_STARTING], (uint64_t)(uintptr_t)v, (uint64_t)(uintptr_t)a);
    }
    if (v->ev[EV_SOURCE] != 0) {
        a = new_obj(IF_SRC);
        wlog("fire SourceChanged", "ICoreWebView2", "isNewDocument=1");
        complete(v->ev[EV_SOURCE], (uint64_t)(uintptr_t)v, (uint64_t)(uintptr_t)a);
    }
    if (v->ev[EV_CONTENT] != 0) {
        a = new_obj(IF_CONTENT);
        wlog("fire ContentLoading", "ICoreWebView2", "isErrorPage=0");
        complete(v->ev[EV_CONTENT], (uint64_t)(uintptr_t)v, (uint64_t)(uintptr_t)a);
    }
    if (v->ev[EV_NAV_DONE] != 0) {
        a = new_obj(IF_NAVARGS);
        wlog("fire NavigationCompleted", "ICoreWebView2", "isSuccess=1 webErrorStatus=0");
        complete(v->ev[EV_NAV_DONE], (uint64_t)(uintptr_t)v, (uint64_t)(uintptr_t)a);
    }
}

/* Issue #1362: the page (the Browser's face, app/src/ui/webview2-surface.ts) has loaded the document that a Navigate named. The engine's idle (fle_thread_idle_hook, fle_wv2_pump) fires the
 * navigation's events then, on the guest's own thread: NavigationStarting, SourceChanged, ContentLoading and NavigationCompleted (IsSuccess TRUE), each to the handler the page registered. */
void fle_wv2_page_loaded(void)
{
    wv_loaded = 1;
}

/* Issue #1362: the engine's idle calls this before it waits: when the page has loaded the document of a pending navigation, its events are fired (once). Nothing else is pending otherwise. */
void fle_wv2_pump(void)
{
    if (!wv_loaded) return;
    wv_loaded = 0;
    if (wv_view == NULL || !wv_view->nav_pending) return;
    wv_view->nav_pending = 0;
    fire_navigation(wv_view, wv_view->nav_uri);
}

/* the method of a slot: the arguments are read first, because a completion handler (guest code) changes R */
static void call_slot(unsigned iface, unsigned k)
{
    const struct slot *s = &tabs[iface][k];
    wv2_obj *o = (wv2_obj *)(uintptr_t)R[1];
    uint64_t a2 = R[2], a8 = R[8], ret = WV_S_OK;
    const char *in = iface_name[iface];
    char t[WV_TEXT_MAX], g[48];
    size_t n = 0;
    unsigned i;

    switch (s->kind) {
    case K_QI:
        guid_of(a2, g, sizeof g);
        if (strcmp(g, IID_IUNKNOWN) == 0 && a8 != 0) {
            o->refs++;
            fle_st(a8, 8, R[1]);
            wlog(s->name, in, "iid=%s -> this", g);
        } else {
            if (a8 != 0) fle_st(a8, 8, 0);
            wlog(s->name, in, "iid=%s -> E_NOINTERFACE", g);
            ret = WV_E_NOINTERFACE;
        }
        break;
    case K_ADDREF:
        o->refs++;
        ret = o->refs;
        wlog(s->name, in, "refs=%u", o->refs);
        break;
    case K_RELEASE:
        if (o->refs > 0) o->refs--;
        ret = o->refs;
        wlog(s->name, in, "refs=%u", o->refs);
        break;
    case K_NOTIMPL:
        wlog(s->name, in, "-> E_NOTIMPL");
        ret = WV_E_NOTIMPL;
        break;
    case K_NOP:
        wlog(s->name, in, "");
        break;
    case K_CREATE_CTRL: {
        wv2_obj *c = new_obj(IF_CTRL);
        c->hwnd = a2;
        wlog(s->name, in, "hwnd=%#llx handler=%#llx", (unsigned long long)a2, (unsigned long long)a8);
        complete(a8, WV_S_OK, (uint64_t)(uintptr_t)c);
        if (c->view != NULL) fire_navigation(c->view, wstr_new("about:blank"));
        break;
    }
    case K_GET_WSTR:
        wlog(s->name, in, "-> \"%s\"", s->text);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, wstr_new(s->text));
        break;
    case K_GET_OBJ: {
        wv2_obj *x;
        if (o->iface == IF_CTRL && s->arg == IF_VIEW) {
            if (o->view == NULL) o->view = new_obj(IF_VIEW);
            x = o->view;
            if (wv_view == NULL) wv_view = x;
        } else {
            x = new_obj(s->arg);
        }
        wlog(s->name, in, "-> %s %#llx", iface_name[s->arg], (unsigned long long)(uintptr_t)x);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, (uint64_t)(uintptr_t)x);
        break;
    }
    case K_GET_VISIBLE:
        wlog(s->name, in, "-> %u", o->visible);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 4, o->visible);
        break;
    case K_PUT_VISIBLE:
        o->visible = (uint32_t)a2 != 0;
        wlog(s->name, in, "isVisible=%u", o->visible);
        break;
    case K_PUT_BOOL:
        wlog(s->name, in, "value=%u", (uint32_t)a2);
        break;
    case K_GET_BOOL:
        wlog(s->name, in, "-> %u", s->arg);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 4, s->arg);
        break;
    case K_GET_U32:
        wlog(s->name, in, "-> %u", s->arg);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 4, s->arg);
        break;
    case K_GET_U64:
        wlog(s->name, in, "-> %u", s->arg);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, s->arg);
        break;
    case K_GET_RECT:
        wlog(s->name, in, "-> %d,%d,%d,%d", o->bounds[0], o->bounds[1], o->bounds[2], o->bounds[3]);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else for (i = 0; i < 4; i++) fle_st(a2 + 4u * i, 4, (uint32_t)o->bounds[i]);
        break;
    case K_PUT_RECT: {
        int32_t ox = 0, oy = 0;

        if (a2 == 0) {
            wlog(s->name, in, "bounds=NULL");
            ret = WV_E_INVALIDARG;
            break;
        }
        for (i = 0; i < 4; i++) o->bounds[i] = (int32_t)(uint32_t)fle_ld(a2 + 4u * i, 4);
        face_hwnd = o->hwnd;
        for (i = 0; i < 4; i++) face_bounds[i] = o->bounds[i];
        face_set = 1;
        /* Issue #1362: the window's client origin on the guest's screen, so that the page can place the face over the desktop (origin=none: the handle is not a live window) */
        if (fle_win_client_origin && fle_win_client_origin(o->hwnd, &ox, &oy))
            wlog(s->name, in, "bounds=%d,%d,%d,%d origin=%d,%d", o->bounds[0], o->bounds[1], o->bounds[2], o->bounds[3], ox, oy);
        else
            wlog(s->name, in, "bounds=%d,%d,%d,%d origin=none", o->bounds[0], o->bounds[1], o->bounds[2], o->bounds[3]);
        break;
    }
    case K_GET_HWND:
        wlog(s->name, in, "-> %#llx", (unsigned long long)o->hwnd);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, o->hwnd);
        break;
    case K_PUT_HWND:
        o->hwnd = a2;
        wlog(s->name, in, "hwnd=%#llx", (unsigned long long)a2);
        break;
    case K_GET_DBL:
        wlog(s->name, in, "-> 1.0");
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, UINT64_C(0x3FF0000000000000));
        break;
    case K_PUT_DBL:
        wlog(s->name, in, "(the value is in xmm; not read)");
        break;
    case K_GET_URI:
        /* Issue #1362: get_Source answers the URI of the last navigation (the first about:blank); before any, the empty string */
        if (o->uri != 0) text_of(o->uri, t, sizeof t, &n);
        else snprintf(t, sizeof t, "%s", "");
        wlog(s->name, in, "-> \"%s\"", t);
        if (a2 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a2, 8, o->uri != 0 ? utf16_copy(o->uri) : wstr_new(""));
        break;
    case K_HDR_VALUE:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "name=\"%s\" -> \"\"", t);
        if (a8 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a8, 8, wstr_new(""));
        break;
    case K_HDR_CONTAINS:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "name=\"%s\" -> 0", t);
        if (a8 == 0) ret = WV_E_INVALIDARG;
        else fle_st(a8, 4, 0);
        break;
    case K_ADD:
        hold_handler(a2);
        if (a8 != 0) fle_st(a8, 8, ++wv_token);
        wlog(s->name, in, "handler=%#llx token=%llu", (unsigned long long)a2, (unsigned long long)(a8 != 0 ? wv_token : 0));
        break;
    case K_ADD_EV:
        hold_handler(a2);
        if (a8 != 0) fle_st(a8, 8, ++wv_token);
        o->ev[s->arg] = a2;
        wlog(s->name, in, "handler=%#llx token=%llu", (unsigned long long)a2, (unsigned long long)(a8 != 0 ? wv_token : 0));
        break;
    case K_ADD_SCRIPT:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "script=\"%s\" units=%zu handler=%#llx", t, n, (unsigned long long)a8);
        complete(a8, WV_S_OK, wstr_new("1"));
        break;
    case K_REMOVE:
        wlog(s->name, in, "token=%llu", (unsigned long long)a2);
        break;
    case K_NAVIGATE:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "uri=\"%s\" units=%zu", t, n);
        /* Issue #1362: where the guest asked for the navigation: the return address of the call ([rsp] on the slot's entry), then the values above it on the guest's stack that lie in
         * the image's range (the return addresses of the callers; the analysis checks them against the code's labels) */
        {
            char st[1200];
            size_t k = 0;
            unsigned j;

            st[0] = 0;
            for (j = 1; j < 128u; j++) {
                uint64_t v = fle_ld(R[4] + 8u * j, 8);

                if (v < 0x400000u || v >= 0x13000000u || k + 20u >= sizeof st) continue;
                k += (size_t)snprintf(st + k, sizeof st - k, "%s%#llx", k ? "," : "", (unsigned long long)v);
            }
            wlog("caller", in, "at=%#llx stack=%s", (unsigned long long)fle_ld(R[4], 8), st);
        }
        /* Issue #1362: the navigation is asked; its events are fired when the page has loaded the document (fle_wv2_pump) */
        if (a2 != 0 && wv_view != NULL) {
            wv_view->nav_uri = utf16_copy(a2);
            wv_view->nav_pending = 1;
        }
        break;
    case K_NAV_HTML:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "html=\"%s\" units=%zu", t, n);
        break;
    case K_EXEC:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "script=\"%s\" units=%zu handler=%#llx", t, n, (unsigned long long)a8);
        complete(a8, WV_S_OK, wstr_new("null"));
        break;
    case K_POST:
        text_of(a2, t, sizeof t, &n);
        wlog(s->name, in, "message=\"%s\" units=%zu", t, n);
        break;
    default:
        fle_stop("WebView2", "a slot of the model has no kind");
    }
    R[0] = ret;
}

int fle_wv2_try_slot(uint64_t va)
{
    uint64_t id;
    unsigned i;

    vt_init();
    if (va < FLE_WV2_SLOT_BASE || va >= FLE_WV2_SLOT_BASE + FLE_WV2_SLOT_STEP * (uint64_t)slot_total || (va - FLE_WV2_SLOT_BASE) % FLE_WV2_SLOT_STEP != 0) return 0;
    id = (va - FLE_WV2_SLOT_BASE) / FLE_WV2_SLOT_STEP;
    for (i = 0; i < IF_COUNT; i++) {
        if (id >= slot_base[i] && id < slot_base[i] + ntabs[i]) {
            call_slot(i, (unsigned)(id - slot_base[i]));
            return 1;
        }
    }
    return 0;
}

/* The environment is handed to the completed handler (Invoke(hr, environment)), and the call returns S_OK (Learn: the result comes in the handler). A NULL handler is
 * not defined by the page (E_INVALIDARG here, CHOICE). */
static void make_env(uint64_t handler)
{
    wv2_obj *env;

    if (handler == 0) {
        R[0] = WV_E_INVALIDARG;
        return;
    }
    env = new_obj(IF_ENV);
    complete(handler, WV_S_OK, (uint64_t)(uintptr_t)env);
    R[0] = WV_S_OK;
}

/* webview2loader.dll!CreateCoreWebView2EnvironmentWithOptions(PCWSTR browserExecutableFolder rcx, PCWSTR userDataFolder rdx, options r8, handler r9) */
void fle_imp_CreateCoreWebView2EnvironmentWithOptions(void)
{
    uint64_t folder = R[1], data = R[2], options = R[8], handler = R[9];
    char f[WV_TEXT_MAX], d[WV_TEXT_MAX];
    size_t n = 0;

    text_of(folder, f, sizeof f, &n);
    text_of(data, d, sizeof d, &n);
    wlog("CreateCoreWebView2EnvironmentWithOptions", "WebView2Loader", "browser=\"%s\" userData=\"%s\" options=%#llx handler=%#llx", f, d,
         (unsigned long long)options, (unsigned long long)handler);
    make_env(handler);
}

/* webview2loader.dll!CreateCoreWebView2Environment(handler rcx) */
void fle_imp_CreateCoreWebView2Environment(void)
{
    wlog("CreateCoreWebView2Environment", "WebView2Loader", "handler=%#llx", (unsigned long long)R[1]);
    make_env(R[1]);
}

/* webview2loader.dll!GetAvailableCoreWebView2BrowserVersionString(PCWSTR browserExecutableFolder rcx, LPWSTR *versionInfo rdx): the runtime's version, in a string of its own */
void fle_imp_GetAvailableCoreWebView2BrowserVersionString(void)
{
    uint64_t folder = R[1], out = R[2];
    char f[WV_TEXT_MAX];
    size_t n = 0;

    text_of(folder, f, sizeof f, &n);
    wlog("GetAvailableCoreWebView2BrowserVersionString", "WebView2Loader", "browser=\"%s\" -> \"%s\"", f, WV_VERSION);
    if (out == 0) {
        R[0] = WV_E_INVALIDARG;
        return;
    }
    fle_st(out, 8, wstr_new(WV_VERSION));
    R[0] = WV_S_OK;
}

/* the version as four numbers (a dotted decimal of at most four parts); 0 when the text is not one */
static int read_version(uint64_t p, unsigned long v[4])
{
    unsigned long cur = 0;
    int digits = 0;
    unsigned part = 0;
    size_t k;

    if (p == 0) return 0;
    for (k = 0;; k++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * k, 2);
        if (c == 0) break;
        if (k >= 64) return 0;
        if (c >= '0' && c <= '9') {
            cur = cur * 10 + (c - '0');
            digits = 1;
        } else if (c == '.' && digits && part < 3) {
            v[part++] = cur;
            cur = 0;
            digits = 0;
        } else {
            return 0;
        }
    }
    if (!digits) return 0;
    v[part] = cur;
    for (part++; part < 4; part++) v[part] = 0;
    return 1;
}

/* webview2loader.dll!CompareBrowserVersions(PCWSTR v1 rcx, PCWSTR v2 rdx, int *result r8): -1, 0 or 1 by the parts in order (missing parts count as 0) */
void fle_imp_CompareBrowserVersions(void)
{
    uint64_t a = R[1], b = R[2], out = R[8];
    unsigned long va[4], vb[4];
    char ta[WV_TEXT_MAX], tb[WV_TEXT_MAX];
    int r = 0;
    size_t n = 0;
    unsigned i;

    text_of(a, ta, sizeof ta, &n);
    text_of(b, tb, sizeof tb, &n);
    if (!read_version(a, va) || !read_version(b, vb) || out == 0) {
        wlog("CompareBrowserVersions", "WebView2Loader", "v1=\"%s\" v2=\"%s\" -> E_INVALIDARG", ta, tb);
        R[0] = WV_E_INVALIDARG;
        return;
    }
    for (i = 0; i < 4; i++) {
        if (va[i] != vb[i]) {
            r = va[i] < vb[i] ? -1 : 1;
            break;
        }
    }
    wlog("CompareBrowserVersions", "WebView2Loader", "v1=\"%s\" v2=\"%s\" -> %d", ta, tb, r);
    fle_st(out, 4, (uint64_t)(int64_t)r);
    R[0] = WV_S_OK;
}

/* Issue #1376: the windows painted over the face (its window, and the bounds its controller was given last), for the page (app/src/ui/webview2-surface.ts clips the face by them):
 * word 0 = the number of windows, then per window l, t, r, b on the guest's screen (the same space as the bounds' origin). The page asks when the picture or the face has changed. */
#define WV_FRONT_MAX 64u
static int32_t front_words[1 + 4 * WV_FRONT_MAX];

__attribute__((used)) const int32_t *fle_webview2_front(void)
{
    int32_t ox = 0, oy = 0;
    unsigned n;

    front_words[0] = 0;
    if (!face_set || !fle_win_front_rects || !fle_win_client_origin) return front_words;
    if (!fle_win_client_origin(face_hwnd, &ox, &oy)) return front_words;
    n = fle_win_front_rects(face_hwnd, ox + face_bounds[0], oy + face_bounds[1], ox + face_bounds[2], oy + face_bounds[3], front_words + 1, WV_FRONT_MAX);
    front_words[0] = (int32_t)n;
    return front_words;
}
