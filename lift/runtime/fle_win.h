/* The virtual window table: user32!CreateWindowExW, user32!SetWindowLongPtrW (GWLP_WNDPROC only) and user32!DefWindowProcW
 * (docs/decisions/0071-virtual-windows.md, PROVISIONAL; the pages read and the guest's call sites are in
 * docs/investigations/window-design.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8),
 * R[9] (r9) and, from the fifth on, on the guest stack (fle_arg, fle_env.h); the Win32 return value in R[0]; the guest return
 * address stays on the stack for the caller's ret. Single guest thread (a call from a second thread stops the run).
 *
 * This is the first window of this environment and the base of the rest of user32. A window is {handle, class atom, hInstance,
 * style, exstyle, window procedure, live}; nothing is ever removed. CreateWindowExW sends WM_NCCREATE, WM_NCCALCSIZE and
 * WM_CREATE to the window procedure of the window through fle_call_guest (fle_callback.h, decision 0064), as the Learn page of
 * CreateWindowEx says, before it returns. Nothing here reads the host: no real window, no display, no message queue, no clock.
 * Every form the pages leave open, and every form the guest does not use at the point reached, stops the run by name through
 * fle_stop (fle_env.h): "fle: <api>: <why>" and abort(). The last error is never touched (the pages give no value for a failure). */
#ifndef FLE_WIN_H
#define FLE_WIN_H

#include <stdint.h>

/* The handle of the i-th window (i = 0, 1, ...) is FLE_WIN_HANDLE_BASE + 4 * (i + 1), never reused (a PROVISIONAL choice, in the
 * way of decisions 0053 and 0054: Learn gives no value for an HWND). The region (1 MiB) lies directly before the one of the
 * virtual screen DCs (fle_gdc.h: 0x7FA90000) and overlaps none of the other virtual handle regions (the GDI objects 0x7FBA0000, the
 * standard cursors 0x7FCA0000, the cursors and icons of the DLL 0x7FDA0000, the monitor 0x7FEA0000; static assertions in fle_win.c).
 * Below 0x80000000, so that truncation to 32 bits and sign extension give the same 64-bit value (Learn "Interprocess communication":
 * "64-bit versions of Windows use 32-bit handles ... only the lower 32 bits are significant ... Handles that can be shared include
 * ... windows (HWND)"). Never 0. */
#define FLE_WIN_HANDLE_BASE UINT64_C(0x7F990000)
#define FLE_WIN_REGION_SIZE UINT64_C(0x00100000)
#define FLE_WIN_HANDLE_STEP 4u

/* The messages CreateWindowExW sends (Learn CreateWindowExW, Remarks: "sends WM_NCCREATE, WM_NCCALCSIZE, and WM_CREATE messages to
 * the window being created"; Learn "Managing Application State": WM_NCCREATE, then WM_CREATE, "in the order listed"). */
#define FLE_WM_CREATE     0x0001u
#define FLE_WM_NCCREATE   0x0081u
#define FLE_WM_NCCALCSIZE 0x0083u
/* Decision 0119, wall 14: WM_SETICON (Learn WM_SETICON: #define WM_SETICON 0x0080; ICON_BIG 1, ICON_SMALL 0), answered by DefWindowProcW. */
#define FLE_WM_SETICON    0x0080u
#define FLE_ICON_BIG      1u
#define FLE_ICON_SMALL    0u

/* The one form of CreateWindowExW that is answered (decision 0071; the call at 0x52a8be): WS_EX_TOOLWINDOW, WS_POPUP, position and
 * size 0, no parent, no menu, no creation parameter, an empty or NULL window name. Learn Window Styles / Extended Window Styles:
 * WS_POPUP 0x80000000, WS_EX_TOOLWINDOW 0x00000080. */
#define FLE_WS_POPUP         0x80000000u
#define FLE_WS_EX_TOOLWINDOW 0x00000080u

/* Decision 0119 (PROVISIONAL), wall 10: the second form of CreateWindowExW that is answered: the hidden application window of the guest's
 * TApplication (the call at 0x867f7f): WS_EX_TOOLWINDOW, dwStyle 0x84ca0000 = WS_POPUP 0x80000000 | WS_CLIPSIBLINGS 0x04000000 | WS_CAPTION
 * 0x00C00000 | WS_SYSMENU 0x00080000 | WS_MINIMIZEBOX 0x00020000 (Learn Window Styles), a position (x, y) of any value but CW_USEDEFAULT, size 0 x 0,
 * a window name of at most FLE_WIN_TEXT_MAX_UNITS units, no parent, no menu, no creation parameter. No WS_VISIBLE, no WS_THICKFRAME. */
#define FLE_WS_TAPPLICATION    0x84CA0000u
#define FLE_CW_USEDEFAULT      0x80000000u
#define FLE_WIN_TEXT_MAX_UNITS 256u

/* Decision 0121 (PROVISIONAL), wall 20: the third form of CreateWindowExW that is answered: a message-only window. Learn (CreateWindowExW, hWndParent): "To create a message-only window,
 * supply HWND_MESSAGE or a handle to an existing message-only window"; (Window Features, Message-Only Windows): "A message-only window enables you to send and receive messages. It is
 * not visible, has no z-order, cannot be enumerated, and does not receive broadcast messages. The window simply dispatches messages." The number of HWND_MESSAGE is not on those pages:
 * (HWND)-3 is the value of the SDK header (windows-sys docs.rs HWND_MESSAGE = 0xfffffffffffffffd: UNOFFICIAL), and it is what the guest passes at 0x1e85a18 (mov qword ptr [rsp+0x38],
 * 0xfffffffffffffffd). The form: dwExStyle 0, dwStyle 0, position and size 0, no menu, no creation parameter, an empty or NULL window name, a class that fle_class_lookup resolves. */
#define FLE_HWND_MESSAGE UINT64_C(0xFFFFFFFFFFFFFFFD)

/* Decision 0119 (PROVISIONAL), wall 13: the 8 bytes `41 5B FF 25 00 00 00 00` (pop r11; jmp qword ptr [rip + 0]) read as a little-endian uint64: the template that the
 * guest's Delphi MakeObjectInstance copies (from 0x13be804 of the image) into every block it VirtualAllocs. A window procedure that lies in a VirtualAlloc region is
 * called only when it is the `E8 rel32` slot of such a block (fle_win.c, call_wndproc). */
#define FLE_THUNK_TEMPLATE UINT64_C(0x0000000025FF5B41)

/* Decision 0119 (PROVISIONAL), wall 16: the window menu (system menu) of a window, user32!GetSystemMenu and user32!DeleteMenu. The handle of the menu of window index i
 * is FLE_MENU_HANDLE_BASE + 4 * (i + 1) (a CHOICE WITHOUT A SOURCE: Learn gives no value for an HMENU; the region (64 KiB) lies directly before the window handles and
 * overlaps none of the other regions). The items of a fresh window menu are the six SC_ commands of the standard window menu (a choice: Learn lists SC_CLOSE, SC_MOVE
 * and SC_SIZE as examples and gives no complete list); DeleteMenu(..., MF_BYCOMMAND) removes one; nothing is displayed. Learn WM_SYSCOMMAND / About Menus: SC_SIZE 0xF000,
 * SC_MOVE 0xF010, SC_MINIMIZE 0xF020, SC_MAXIMIZE 0xF030, SC_CLOSE 0xF060, SC_RESTORE 0xF120; WS_SYSMENU 0x00080000 (Window Styles); MF_BYCOMMAND 0, MF_BYPOSITION 0x400. */
#define FLE_MENU_HANDLE_BASE UINT64_C(0x7F980000)
#define FLE_MENU_REGION_SIZE UINT64_C(0x00010000)
#define FLE_WS_SYSMENU       0x00080000u
#define FLE_SC_SIZE          0xF000u
#define FLE_SC_MOVE          0xF010u
#define FLE_SC_MINIMIZE      0xF020u
#define FLE_SC_MAXIMIZE      0xF030u
#define FLE_SC_CLOSE         0xF060u
#define FLE_SC_RESTORE       0xF120u
#define FLE_MF_BYCOMMAND     0x00000000u
#define FLE_MF_BYPOSITION    0x00000400u
/* Decision 0119 (PROVISIONAL), wall 17: user32!GetWindowLongPtrW: LONG_PTR f(HWND hWnd rcx, int nIndex rdx); user32!SetWindowTextW: BOOL f(HWND hWnd rcx, LPCWSTR lpString rdx);
 * WM_SETTEXT (Learn: #define WM_SETTEXT 0x000C; "The DefWindowProc function sets and displays the window text"; the return value is TRUE if the text is set). */
#define FLE_WM_SETTEXT 0x000Cu
void fle_imp_GetWindowLongPtrW(void);
void fle_imp_SetWindowTextW(void);
void fle_imp_GetSystemMenu(void);
void fle_imp_DeleteMenu(void);
/* Inspection: the six SC_ commands in a window's menu as a bit set (bit k = item k of the order SIZE, MOVE, MINIMIZE, MAXIMIZE, CLOSE, RESTORE), or -1 when `hwnd` has no menu yet. */
int fle_win_menu_items(uint64_t hwnd);

/* SetWindowLongPtrW / GetWindowLongPtrW indexes (Learn): GWLP_WNDPROC -4, GWLP_HINSTANCE -6, GWLP_HWNDPARENT -8, GWL_STYLE -16, GWL_EXSTYLE -20, GWLP_USERDATA -21. */
#define FLE_GWLP_WNDPROC (-4)
#define FLE_GWLP_HINSTANCE   (-6)
#define FLE_GWLP_HWNDPARENT  (-8)
#define FLE_GWLP_ID          (-12)
#define FLE_GWL_STYLE        (-16)
#define FLE_GWL_EXSTYLE      (-20)
#define FLE_GWLP_USERDATA    (-21)

/* CREATESTRUCTW on x64 (Learn lists the fields; the offsets and the size are derived from the types, 8-byte pointers):
 * LPVOID lpCreateParams @0, HINSTANCE hInstance @8, HMENU hMenu @16, HWND hwndParent @24, int cy @32, int cx @36, int y @40,
 * int x @44, LONG style @48 (4 bytes of padding @52), LPCWSTR lpszName @56, LPCWSTR lpszClass @64, DWORD dwExStyle @72 (4 bytes
 * of tail padding @76) = 80 bytes. Learn gives no sizeof. The structure is built on the guest stack, below the API's frame, for the
 * duration of the three messages; the RECT of WM_NCCALCSIZE (16 bytes) follows it. */
#define FLE_CREATESTRUCTW_SIZE  80
#define FLE_CS_OFF_CREATEPARAMS 0
#define FLE_CS_OFF_HINSTANCE    8
#define FLE_CS_OFF_MENU         16
#define FLE_CS_OFF_PARENT       24
#define FLE_CS_OFF_CY           32
#define FLE_CS_OFF_CX           36
#define FLE_CS_OFF_Y            40
#define FLE_CS_OFF_X            44
#define FLE_CS_OFF_STYLE        48
#define FLE_CS_OFF_NAME         56
#define FLE_CS_OFF_CLASS        64
#define FLE_CS_OFF_EXSTYLE      72

/* user32!CreateWindowExW: HWND f(DWORD dwExStyle rcx, LPCWSTR lpClassName rdx, LPCWSTR lpWindowName r8, DWORD dwStyle r9, int X,
 * int Y, int nWidth, int nHeight, HWND hWndParent, HMENU hMenu, HINSTANCE hInstance, LPVOID lpParam: arguments 5..12 on the stack).
 * user32!SetWindowLongPtrW: LONG_PTR f(HWND hWnd rcx, int nIndex rdx, LONG_PTR dwNewLong r8).
 * user32!DefWindowProcW: LRESULT f(HWND hWnd rcx, UINT Msg rdx, WPARAM wParam r8, LPARAM lParam r9). */
void fle_imp_CreateWindowExW(void);
void fle_imp_SetWindowLongPtrW(void);
void fle_imp_DefWindowProcW(void);

/* Decision 0119 (PROVISIONAL), wall 9: user32!SendMessageW: LRESULT f(HWND hWnd rcx, UINT Msg rdx, WPARAM wParam r8, LPARAM lParam r9) and
 * user32!SetClassLongPtrW: ULONG_PTR f(HWND hWnd rcx, int nIndex rdx, LONG_PTR dwNewLong r8). A hWnd that is not a live window of this table
 * (NULL included) answers 0 and the last error ERROR_INVALID_WINDOW_HANDLE (1400): a CHOICE WITHOUT A SOURCE (the pages of both functions, and
 * of WM_SETICON, say nothing about a NULL or invalid hWnd; System Error Codes lists 1400 "Invalid window handle." without naming an API). */
#define FLE_ERROR_INVALID_WINDOW_HANDLE 1400u
#define FLE_GCLP_HICON (-14)   /* Learn SetClassLongPtrW: GCLP_HICON -14 "Replaces a handle to the icon associated with the class" */
void fle_imp_SendMessageW(void);
void fle_imp_SendMessageA(void);
void fle_imp_SetClassLongPtrW(void);

/* Decision 0119 (PROVISIONAL), wall 11: wtsapi32!WTSRegisterSessionNotification: BOOL f(HWND hWnd rcx, DWORD dwFlags edx) (wtsapi32.dll is virtual module N = 37).
 * dwFlags 0 and 1 are NOTIFY_FOR_THIS_SESSION and NOTIFY_FOR_ALL_SESSIONS in the SDK header (the Learn page names them and gives no number: unconfirmed here). */
#define FLE_NOTIFY_FOR_THIS_SESSION 0u
#define FLE_NOTIFY_FOR_ALL_SESSIONS 1u
void fle_imp_WTSRegisterSessionNotification(void);

/* Decision 0119 (PROVISIONAL), wall 12: uxtheme!BufferedPaintInit: HRESULT f(void) (uxtheme.dll is virtual module N = 38): S_OK, counted; the other buffered-paint
 * functions are not answered. fle_win_buffered_paint_init_count: inspection (for tests and tools). */
void fle_imp_BufferedPaintInit(void);
uint32_t fle_win_buffered_paint_init_count(void);

/* Decision 0122 (PROVISIONAL): the message queue of each guest thread (fle_thread.c holds the queues; docs/decisions/0122-cooperative-threads.md). The window table gives every window an owner, the
 * thread that created it (Learn: "the thread that created the window"), and these functions use the queue of that thread. No message is made up: a queue holds only what the guest posts
 * (PostMessageW, PostQuitMessage); WM_TIMER and WM_PAINT are not generated (nothing here calls SetTimer or paints), and WM_QUIT comes only from PostQuitMessage.
 * user32!PostMessageW: BOOL f(HWND rcx, UINT edx, WPARAM r8, LPARAM r9); user32!GetMessageW: BOOL f(LPMSG rcx, HWND rdx, UINT r8d, UINT r9d);
 * user32!PeekMessageW: BOOL f(LPMSG rcx, HWND rdx, UINT r8d, UINT r9d, UINT wRemoveMsg [5]); user32!WaitMessage: BOOL f(void); user32!PostQuitMessage: VOID f(int rcx);
 * user32!TranslateMessage: BOOL f(const MSG *rcx); user32!DispatchMessageW: LRESULT f(const MSG *rcx). */
void fle_imp_PostMessageW(void);
void fle_imp_GetMessageW(void);
void fle_imp_PeekMessageW(void);
void fle_imp_WaitMessage(void);
void fle_imp_PostQuitMessage(void);
void fle_imp_TranslateMessage(void);
void fle_imp_DispatchMessageW(void);
/* decision 0222: user32!DispatchMessageA (an extra import row: lift/harness/seeds/win2-imports.txt); see fle_win.c. */
void fle_imp_DispatchMessageA(void);
/* Decision 0147 (PROVISIONAL): user32!PostThreadMessageW: BOOL f(DWORD idThread ecx, UINT Msg edx, WPARAM wParam r8, LPARAM lParam r9). The queue of the thread; WM_QUIT is the kept WM_QUIT of PostQuitMessage. */
void fle_imp_PostThreadMessageW(void);
/* MSG on x64 (Learn lists the members; the offsets and the size are derived from the member types, 8-byte HWND, WPARAM and LPARAM): hwnd @0, message @8, wParam @16, lParam @24, time @32,
 * pt.x @36, pt.y @40, lPrivate @44 = 48 bytes. Learn gives no sizeof. */
#define FLE_MSG_SIZE        48
#define FLE_MSG_OFF_HWND    0
#define FLE_MSG_OFF_MESSAGE 8
#define FLE_MSG_OFF_WPARAM  16
#define FLE_MSG_OFF_LPARAM  24
#define FLE_MSG_OFF_TIME    32
#define FLE_MSG_OFF_PTX     36
#define FLE_MSG_OFF_PTY     40
#define FLE_MSG_OFF_PRIVATE 44
/* PeekMessageW wRemoveMsg (Learn): PM_NOREMOVE 0, PM_REMOVE 1, PM_NOYIELD 2; the high word is a union of the PM_QS_* values (QS_INPUT 0x1C07, QS_POSTMESSAGE|QS_HOTKEY|QS_TIMER 0x98, QS_PAINT 0x20, QS_SENDMESSAGE 0x40: decision 0126). */
#define FLE_PM_NOREMOVE 0x0000u
#define FLE_PM_REMOVE   0x0001u
#define FLE_PM_NOYIELD  0x0002u
#define FLE_PM_QS_ALL_HI 0x1CFFu
/* Called by fle_thread.c in the receiving thread's context: the messages that other threads sent to this thread's windows are answered by the windows' procedures (decision 0122). */
void fle_win_process_sent(const char *api);
/* The window table for the harness's stop report: handle, owner thread, class atom, procedure, styles, rectangle, message-only. */
#include <stdio.h>
void fle_win_report(FILE *f);

/* ---- Decision 0123 (PROVISIONAL): window management, a STATE-ONLY model (nothing is drawn, no real input) ----
 * Message numbers (the Learn pages of each message give the #define, 2026-10-31): WM_DESTROY 0x0002, WM_MOVE 0x0003, WM_SIZE 0x0005, WM_ACTIVATE 0x0006, WM_SETFOCUS 0x0007, WM_KILLFOCUS 0x0008, WM_ENABLE 0x000A,
 * WM_CLOSE 0x0010, WM_ERASEBKGND 0x0014, WM_SHOWWINDOW 0x0018, WM_CANCELMODE 0x001F, WM_GETMINMAXINFO 0x0024, WM_WINDOWPOSCHANGING 0x0046, WM_WINDOWPOSCHANGED 0x0047, WM_NCDESTROY 0x0082,
 * WM_NCCALCSIZE 0x0083, WM_NCACTIVATE 0x0086, WM_TIMER 0x0113, WM_CAPTURECHANGED 0x0215. WM_PAINT 0x000F and WM_NCPAINT 0x0085 are NOT on the pages read (the WM_PAINT and WM_NCPAINT pages give no #define): they are the values of the
 * SDK header (windows-sys docs.rs, UNOFFICIAL). */
#define FLE_WM_DESTROY            0x0002u
#define FLE_WM_MOVE               0x0003u
#define FLE_WM_SIZE               0x0005u
#define FLE_WM_ACTIVATE           0x0006u
#define FLE_WM_SETFOCUS           0x0007u
#define FLE_WM_KILLFOCUS          0x0008u
#define FLE_WM_ENABLE             0x000Au
#define FLE_WM_PAINT              0x000Fu
#define FLE_WM_CLOSE              0x0010u
#define FLE_WM_ERASEBKGND         0x0014u
#define FLE_WM_SHOWWINDOW         0x0018u
#define FLE_WM_CANCELMODE         0x001Fu
#define FLE_WM_GETMINMAXINFO      0x0024u
#define FLE_WM_WINDOWPOSCHANGING  0x0046u
#define FLE_WM_WINDOWPOSCHANGED   0x0047u
#define FLE_WM_NCDESTROY          0x0082u
#define FLE_WM_NCPAINT            0x0085u
#define FLE_WM_NCACTIVATE         0x0086u
#define FLE_WM_TIMER              0x0113u
#define FLE_WM_CAPTURECHANGED     0x0215u
/* Window styles (Learn Window Styles / Extended Window Styles give the values). */
#define FLE_WS_CHILD       0x40000000u
#define FLE_WS_VISIBLE     0x10000000u
#define FLE_WS_DISABLED    0x08000000u
#define FLE_WS_MINIMIZE    0x20000000u
#define FLE_WS_MAXIMIZE    0x01000000u
#define FLE_WS_EX_TOPMOST  0x00000008u
#define FLE_WS_EX_NOACTIVATE 0x08000000u
#define FLE_WS_EX_NOPARENTNOTIFY 0x00000004u
/* SetWindowPos flags and special hWndInsertAfter values (Learn SetWindowPos / WINDOWPOS). */
#define FLE_SWP_NOSIZE         0x0001u
#define FLE_SWP_NOMOVE         0x0002u
#define FLE_SWP_NOZORDER       0x0004u
#define FLE_SWP_NOREDRAW       0x0008u
#define FLE_SWP_NOACTIVATE     0x0010u
#define FLE_SWP_FRAMECHANGED   0x0020u
#define FLE_SWP_SHOWWINDOW     0x0040u
#define FLE_SWP_HIDEWINDOW     0x0080u
#define FLE_SWP_NOCOPYBITS     0x0100u
#define FLE_SWP_NOOWNERZORDER  0x0200u
#define FLE_SWP_NOSENDCHANGING 0x0400u
#define FLE_SWP_DEFERERASE     0x2000u
#define FLE_SWP_ASYNCWINDOWPOS 0x4000u
#define FLE_HWND_TOP       UINT64_C(0)
#define FLE_HWND_BOTTOM    UINT64_C(1)
#define FLE_HWND_TOPMOST   UINT64_C(0xFFFFFFFFFFFFFFFF)
#define FLE_HWND_NOTOPMOST UINT64_C(0xFFFFFFFFFFFFFFFE)
/* ShowWindow commands (Learn ShowWindow). */
#define FLE_SW_HIDE            0
#define FLE_SW_SHOWNORMAL      1
#define FLE_SW_SHOWMINIMIZED   2
#define FLE_SW_SHOWMAXIMIZED   3
#define FLE_SW_SHOWNOACTIVATE  4
#define FLE_SW_SHOW            5
#define FLE_SW_MINIMIZE        6
#define FLE_SW_SHOWMINNOACTIVE 7
#define FLE_SW_SHOWNA          8
#define FLE_SW_RESTORE         9
/* GetWindow commands, GetAncestor flags (Learn). */
#define FLE_GW_HWNDFIRST 0u
#define FLE_GW_HWNDLAST  1u
#define FLE_GW_HWNDNEXT  2u
#define FLE_GW_HWNDPREV  3u
#define FLE_GW_OWNER     4u
#define FLE_GW_CHILD     5u
#define FLE_GW_ENABLEDPOPUP 6u
#define FLE_GA_PARENT    1u
#define FLE_GA_ROOT      2u
#define FLE_GA_ROOTOWNER 3u
/* The desktop window (Learn GetDesktopWindow: "a handle to the desktop window"; the value is not on a page): a CHOICE WITHOUT A SOURCE, in the window handle region, below every window of the table. */
#define FLE_DESKTOP_HANDLE (FLE_WIN_HANDLE_BASE)
#define FLE_ERROR_INVALID_PARAMETER_W 87u
#define FLE_ERROR_ACCESS_DENIED_W 5u
/* System Error Codes 1300-1699 (read 2026-10-31, learn/System-Error-Codes-1300-1699.txt): 1400 ERROR_INVALID_WINDOW_HANDLE "Invalid window handle.", 1408 ERROR_WINDOW_OF_OTHER_THREAD "Invalid window; it belongs to
 * other thread.", 1411 ERROR_CLASS_DOES_NOT_EXIST "Class does not exist.", 1412 ERROR_CLASS_HAS_WINDOWS "Class still has open windows.". The pages of the APIs name NO code for these failures: using these codes is a CHOICE
 * WITHOUT A SOURCE (decision 0123), made because the code table names the condition. */
#define FLE_ERROR_INVALID_WINDOW_HANDLE_W 1400u
#define FLE_ERROR_WINDOW_OF_OTHER_THREAD 1408u
#define FLE_ERROR_CLASS_DOES_NOT_EXIST 1411u
#define FLE_ERROR_CLASS_HAS_WINDOWS 1412u
/* user32!ShowWindow, IsWindowVisible, SetWindowPos, MoveWindow, GetWindowRect, GetClientRect, ClientToScreen, ScreenToClient, MapWindowPoints, EnableWindow, IsWindowEnabled, IsWindow, GetParent, GetWindow, SetParent,
 * GetAncestor, GetWindowThreadProcessId, SetFocus, GetFocus, SetActiveWindow, GetActiveWindow, GetForegroundWindow, SetForegroundWindow, SetCapture, GetCapture, ReleaseCapture, WindowFromPoint, GetCursorPos, FindWindowW,
 * UnregisterClassW, SetTimer, KillTimer, InvalidateRect, ValidateRect, UpdateWindow, BeginPaint, EndPaint, EnumWindows, DestroyWindow, IsChild, IsIconic, IsZoomed, GetDesktopWindow, GetTopWindow, BringWindowToTop,
 * GetWindowTextW, EnumChildWindows, EnumThreadWindows, GetUpdateRect (the other forms that a wall shows are added in decision 0123 with the same convention as above). */
/* Decision 0125 (PROVISIONAL): what fle_gdc.c asks of the window table to give a DC to a window (GetDC, GetWindowDC, GetDCEx, BeginPaint). `hwnd` is a live window of the table or the desktop window
 * (FLE_DESKTOP_HANDLE): 1 and the answer; any other value: 0 (no state moves, the last error is not touched). root = the top-level window that owns the drawing surface of the window (R10 of
 * docs/investigations/detach-window.md: one surface per top-level window; a child draws on the surface of its top-level ancestor, an owned window on its own; the desktop on itself), class_style = the `style`
 * member of the window's class (CS_OWNDC 0x20, CS_CLASSDC 0x40, CS_PARENTDC 0x80: Learn Window Class Styles; the desktop has none), style and exstyle of the window, thread = the index of the guest
 * thread that owns the window, cx and cy = the size of the window rectangle (the desktop: the virtual display). */
typedef struct {
    uint64_t root;
    uint32_t class_style, style, exstyle, thread, lid, root_lid;
    uint16_t atom;
    int32_t cx, cy;
    /* Decision 0126 (PROVISIONAL): what the pixel model needs. root_cx, root_cy = the size of the top-level window = the size of its drawing surface (the desktop: the virtual display); org_x, org_y = the
     * origin of the window (its client area: the non-client area is 0, nc_zero) in the surface of its top-level window; visible = the window and all its parents have WS_VISIBLE (the desktop: 1);
     * clip_* = the window's client rectangle in the surface coordinates intersected with the client rectangles of all its parents (a rectangle only: siblings and WS_CLIPCHILDREN are not modelled, S-clip-rects). */
    int32_t root_cx, root_cy, org_x, org_y;
    int visible;
    int32_t clip_l, clip_t, clip_r, clip_b;
} fle_win_dcq;
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *out);

/* Decision 0319 (PROVISIONAL): the bits and the update area of ScrollWindowEx / ScrollWindow (fle_win.c, "fle_win_scroll"). `scroll` / `clip` are client-coordinate RECTs (l, t, r, b) or NULL; flags are the SW_* flags of
 * ScrollWindowEx. Returns 1 with *res filled (res->invalid: the handle is not a live window; res->parts: the number of rectangles of the uncovered area; res->unc: its bounding rectangle, res->upd: the bounding rectangle of the
 * uncovered area and the moved update area, both in client coordinates and meaningful when parts > 0 or the rectangle is non-empty), or 0 when the model cannot answer (the caller keeps its former simple path). */
typedef struct {
    int invalid;
    int parts;
    int32_t unc[4];
    int32_t upd[4];
} fle_win_scroll_result;
int fle_win_scroll(const char *api, uint64_t hwnd, int32_t dx, int32_t dy, const int32_t *scroll, const int32_t *clip, uint32_t flags, fle_win_scroll_result *res);
/* Decision 0140: UpdateLayeredWindow moves and resizes a layered window without messages (S-ulw-no-messages); see fle_layered.c. */
int fle_win_layered_rect(uint64_t hwnd, int set_pos, int32_t x, int32_t y, int set_size, int32_t cx, int32_t cy);

void fle_imp_ShowWindow(void);
void fle_imp_IsWindowVisible(void);
void fle_imp_SetWindowPos(void);
void fle_imp_MoveWindow(void);
void fle_imp_GetWindowRect(void);
void fle_imp_GetClientRect(void);
void fle_imp_ClientToScreen(void);
void fle_imp_ScreenToClient(void);
void fle_imp_MapWindowPoints(void);
void fle_imp_EnableWindow(void);
void fle_imp_IsWindowEnabled(void);
void fle_imp_IsWindow(void);
void fle_imp_GetParent(void);
void fle_imp_GetWindow(void);
void fle_imp_SetParent(void);
void fle_imp_GetAncestor(void);
void fle_imp_GetWindowThreadProcessId(void);
void fle_imp_SetFocus(void);
void fle_imp_GetFocus(void);
void fle_imp_SetActiveWindow(void);
void fle_imp_GetActiveWindow(void);
/* Decision 0129 (wall 9): user32!GetLastActivePopup: HWND f(HWND rcx); see fle_win.c. */
void fle_imp_GetLastActivePopup(void);
/* Decision 0129 (wall 12): user32!GetGuiResources: DWORD f(HANDLE rcx, DWORD edx); see fle_win.c. */
void fle_imp_GetGuiResources(void);
void fle_imp_GetForegroundWindow(void);
void fle_imp_SetForegroundWindow(void);
void fle_imp_SetCapture(void);
void fle_imp_GetCapture(void);
void fle_imp_ReleaseCapture(void);
void fle_imp_WindowFromPoint(void);
void fle_imp_GetCursorPos(void);
/* Decision 0126 (wall 74): user32!GetPhysicalCursorPos (a GetProcAddress target of the guest): the logical position (no DPI virtualisation). */
void fle_imp_GetPhysicalCursorPos(void);
void fle_imp_SetCursorPos(void);
/* Decision 0126 (wall 77): user32!GetCursorInfo: BOOL f(PCURSORINFO): CURSORINFO of 24 bytes from the cursor state of fle_gdi.c and the pointer position. */
void fle_imp_GetCursorInfo(void);
void fle_imp_SetPhysicalCursorPos(void);
/* Decision 0126 (wall 61): user32!GetKeyState and GetAsyncKeyState: SHORT f(int). No input exists: every key is up and untoggled (0). */
void fle_imp_GetKeyState(void);
/* Decision 0126 (wall 63): user32!CheckMenuItem and EnableMenuItem: a menu that does not exist (NULL or not a menu of this table) has no item: UINT_MAX. */
/* Decision 0126 (wall 64): user32!LockWindowUpdate: BOOL f(HWND). fle_win_lock_attempted(hwnd): fle_gdi.c tells the window table that a drawing call at a DC of the locked window (or a child of it) was discarded. */
void fle_imp_LockWindowUpdate(void);
/* Decision 0126 (wall 84): user32!AdjustWindowRectEx: BOOL f(LPRECT, DWORD, BOOL, DWORD): the rectangle is returned as it is (the non-client area is 0: S-adjustrect-nc-zero). */
void fle_imp_AdjustWindowRectEx(void);
void fle_win_lock_attempted(uint64_t hwnd);
void fle_imp_CheckMenuItem(void);
void fle_imp_EnableMenuItem(void);
void fle_imp_GetAsyncKeyState(void);
void fle_imp_FindWindowW(void);
void fle_imp_UnregisterClassW(void);
void fle_imp_SetTimer(void);
void fle_imp_KillTimer(void);
void fle_imp_InvalidateRect(void);
void fle_imp_ValidateRect(void);
void fle_imp_UpdateWindow(void);
void fle_imp_BeginPaint(void);
void fle_imp_EndPaint(void);
void fle_imp_EnumWindows(void);
void fle_imp_DestroyWindow(void);
void fle_imp_IsChild(void);
void fle_imp_IsIconic(void);
void fle_imp_IsZoomed(void);
void fle_imp_GetDesktopWindow(void);
void fle_imp_GetTopWindow(void);
void fle_imp_BringWindowToTop(void);
void fle_imp_GetWindowTextW(void);
void fle_imp_EnumChildWindows(void);
void fle_imp_EnumThreadWindows(void);
void fle_imp_GetUpdateRect(void);
/* Decision 0126 (wall 93): user32!GetUpdateRgn: int f(HWND, HRGN, BOOL): copies the update region (one bounding rectangle: S-updregion) into a region of fle_gdi.c; SIMPLEREGION / NULLREGION, ERROR 0. */
void fle_imp_GetUpdateRgn(void);
/* Decision 0126 (wall 96): user32!ValidateRgn: BOOL f(HWND, HRGN): removes the region from the update region (one rectangle: emptied when the region covers it). */
void fle_imp_ValidateRgn(void);
/* Decision 0126 (wall 101, PROVISIONAL; the notes are in fle_win.c): user32!SetWindowsHookExW: HHOOK f(int, HOOKPROC, HINSTANCE, DWORD) for WH_MOUSE and WH_KEYBOARD on a thread of this process: recorded and never called (no mouse or
 * keyboard message exists here); user32!UnhookWindowsHookEx: BOOL f(HHOOK). fle_win_hook_live: the hooks not yet removed. */
void fle_imp_SetWindowsHookExW(void);
void fle_imp_UnhookWindowsHookEx(void);
uint32_t fle_win_hook_live(void);
void fle_imp_IsWindowUnicode(void);
void fle_imp_GetClassNameW(void);
void fle_imp_SetPropW(void);
void fle_imp_GetPropW(void);
void fle_imp_RemovePropW(void);
void fle_imp_CallWindowProcW(void);
void fle_imp_SetMenu(void);
void fle_imp_GetMenu(void);
void fle_imp_DrawMenuBar(void);
void fle_imp_InitializeFlatSB(void);
void fle_imp_FlatSB_SetScrollProp(void);
void fle_imp_FlatSB_SetScrollInfo(void);
void fle_imp_FlatSB_GetScrollInfo(void);
void fle_imp_FlatSB_SetScrollPos(void);
void fle_imp_FlatSB_GetScrollPos(void);
void fle_imp_SetScrollInfo(void);
void fle_imp_GetScrollInfo(void);
/* Decision 0132 (PROVISIONAL), wall 20: user32!ShowScrollBar: BOOL f(HWND rcx, int wBar edx, BOOL bShow r8d); state only (fle_win.c). */
void fle_imp_ShowScrollBar(void);
int fle_win_scrollbar_shown(uint64_t hwnd, int bar);
void fle_imp_GetWindowPlacement(void);
void fle_imp_SetWindowPlacement(void);   /* decision 0267 (alpha10) */
void fle_imp_RedrawWindow(void);
void fle_imp_BeginDeferWindowPos(void);
void fle_imp_DeferWindowPos(void);
void fle_imp_EndDeferWindowPos(void);

/* Inspection for the stop report and the tests: the window table as it stands (one line per window, visible top-level windows first by z-order). The G2 evidence (decision 0123) is the first moment a window that
 * is visible and top-level is in the table. fle_win_visible_toplevel_count: how many such windows exist now. fle_win_g2_seen: 1 once one has appeared. */
/* Decision 0126: the window table as the stop reports and the evidence blocks print it: one line per live window (handle, logical id, class, title, rectangle, styles, parent, owner, visible, z position in its group). */
void fle_win_table_report(FILE *f);
uint32_t fle_win_visible_toplevel_count(void);
int fle_win_g2_seen(void);
void fle_win_g2_report(FILE *f);
/* The timers of the model and the WM_PAINT of the invalid windows are synthesised by the message retrieval of the owner thread (fle_win.c): the next virtual-clock time at which a timer of thread `idx` is due, or 0 when
 * the thread has none (used by the scheduler of fle_thread.c: a thread that waits for input with a timer pending is not idle). */
uint64_t fle_win_update_folds(void);   /* decision 0247 (alpha3): update regions folded to their bounding box */
uint64_t fle_win_next_timer_tick(unsigned idx);
int fle_win_has_paint(unsigned idx);
/* Decision 0129: the number of WM_TIMER and WM_PAINT messages made by the retrieval calls so far (for the harness report; no state moves). */
void fle_win_synth_counts(uint64_t *timers, uint64_t *paints);
/* Decision 0129: the live window timers (those whose window is live): how many, and the longest period in ms (0 when none). */
uint32_t fle_win_timer_live_count(void);
uint64_t fle_win_timer_max_period_ms(void);

/* Inspection (for tests and tools; no state moves). fle_win_count: the windows entered in the table so far (the ones whose creation
 * failed included). fle_win_get: 1 and the entry when `hwnd` is a handle that this table issued (a window whose creation failed
 * has live == 0), else 0. */
typedef struct {
    uint64_t wndproc;   /* the current window procedure (the class's lpfnWndProc until SetWindowLongPtrW replaces it) */
    uint64_t hinst;
    uint32_t style;
    uint32_t exstyle;
    uint16_t atom;      /* the atom of the class (fle_class.h) */
    int live;
    int32_t x, y, cx, cy;                       /* decision 0119: the window rectangle given at creation */
    uint16_t text[FLE_WIN_TEXT_MAX_UNITS + 1];  /* decision 0119: the window text (lpWindowName at creation) */
    uint16_t text_len;
    int wts_registered;                         /* decision 0119: WTSRegisterSessionNotification */
    uint32_t wts_flags;
    uint64_t icon_big, icon_small;              /* decision 0119: set by WM_SETICON through DefWindowProcW; 0 = none */
    uint64_t userdata;                          /* decision 0119: GWLP_USERDATA */
    int msgonly;                                /* decision 0121: created with hWndParent HWND_MESSAGE */
    uint32_t owner;                             /* decision 0122: the index of the guest thread that created the window (fle_thread.h) */
} fle_win_view;

uint64_t fle_win_count(void);
int fle_win_get(uint64_t hwnd, fle_win_view *v);
/* Decision 0209 (crash1): set the extended-style bits of `mask` to `value` on a live window; 1 when it exists, else 0. */
int fle_win_set_exstyle_bits(uint64_t hwnd, uint32_t mask, uint32_t value);
/* decision 0222: the end of the process: every window that is left is destroyed without a message (see fle_win.c) */
void fle_win_process_ended(void);


/* Decision 0171 (PROVISIONAL): the system caret (state only), ShowOwnedPopups, LogicalToPhysicalPoint, PhysicalToLogicalPoint. See fle_win.c. fle_win_caret_state: 0 = no caret; else 1, with the position and the hide count. */
void fle_imp_CreateCaret(void);
void fle_imp_DestroyCaret(void);
void fle_imp_HideCaret(void);
void fle_imp_ShowCaret(void);
void fle_imp_SetCaretPos(void);
int fle_win_caret_state(int32_t *x, int32_t *y, int32_t *hide);
void fle_imp_ShowOwnedPopups(void);
void fle_imp_LogicalToPhysicalPoint(void);
void fle_imp_PhysicalToLogicalPoint(void);

/* Decision 0171 (PROVISIONAL): user32!CallNextHookEx: LRESULT f(HHOOK rcx, int edx, WPARAM r8, LPARAM r9); calls the next older WH_CALLWNDPROC hook procedure (see fle_win.c). */
void fle_imp_CallNextHookEx(void);

/* Decision 0171 (PROVISIONAL): user32!SetWindowRgn: int f(HWND rcx, HRGN rdx, BOOL r8d); state only. fle_win_window_rgn: the region handle of a window, 0 = none. */
void fle_imp_SetWindowRgn(void);
uint64_t fle_win_window_rgn(uint64_t hwnd);
/* Decision 0244 (alpha1): the box of the window region in the window's coordinates (1 = there is one); the margins of DwmExtendFrameIntoClientArea (set / get; get gives 1 when some margin is not 0). See fle_win.c. */
int fle_win_window_region_box(uint64_t hwnd, int32_t *l, int32_t *t, int32_t *r, int32_t *b);
void fle_win_set_frame_margins(uint64_t hwnd, const int32_t m[4]);
int fle_win_frame_margins(uint64_t hwnd, int32_t m[4]);

#endif
