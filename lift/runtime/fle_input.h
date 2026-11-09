/* Decision 0136 (PROVISIONAL): the input round trip. The "system's input thread" of the virtual machine: the hardware events that the host (a script in a native run, the browser's pointer and key events in a wasm
 * run) feeds in are queued here and turned into the messages Windows makes of them when the owning thread retrieves them (hit test, WM_NCHITTEST, WM_SETCURSOR, WM_MOUSEACTIVATE, the button, move and key
 * messages, WM_MOUSELEAVE / WM_MOUSEHOVER of comctl32!_TrackMouseEvent). Same convention as fle_imports.h for the rows: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), the Win32 return value in R[0].
 * Nothing here reads the host: the events, their order and their time (the virtual clock) are what the host passes.
 *
 * Window-table access: fle_win.c holds the table; the accessors below (fle_win_input_*) are defined there at the end of the file. fle_thread.c asks fle_input_has_event / fle_input_next_tick (weak). */
#ifndef FLE_INPUT_H
#define FLE_INPUT_H

#include <stdint.h>
#include <stdio.h>

#include "fle_thread.h"

#ifdef FLE_INPUT_IMPL
#define FLE_INPUT_REF
#else
#define FLE_INPUT_REF __attribute__((weak))
#endif

/* ---- what the host feeds in ---- */
enum { FLE_IN_MOVE = 1, FLE_IN_BDOWN = 2, FLE_IN_BUP = 3, FLE_IN_WHEEL = 4, FLE_IN_KDOWN = 5, FLE_IN_KUP = 6, FLE_IN_DISPLAY = 7, FLE_IN_SYSCMD = 8 };
#define FLE_IN_BTN_L 0u
#define FLE_IN_BTN_R 1u
#define FLE_IN_BTN_M 2u
/* A mouse event at the screen point (x, y) of the virtual screen (0..1919, 0..1079; a point outside is clamped). kind MOVE; BDOWN / BUP with data = FLE_IN_BTN_*; WHEEL with data = the signed delta (a multiple
 * of WHEEL_DELTA = 120). Returns 1 when queued, 0 when the event queue is full or the kind is unknown. The pointer position and the button state of GetCursorPos / GetAsyncKeyState change at once. */
int fle_input_host_mouse(int kind, int32_t x, int32_t y, int32_t data);
/* A key event: kind KDOWN / KUP, the virtual-key code, the scan code (0 = from the US table), extended = bit 24 of lParam. A KDOWN of a key that is down is an auto-repeat. */
int fle_input_host_key(int kind, uint32_t vk, uint32_t scan, int extended);
/* Decision 0212 (screen1, PROVISIONAL, opt-in): the display changed to w x h pixels (the browser window was resized). Queued with the input; when the GUI thread (thread 0) retrieves messages the new size becomes the size of the virtual
 * screen (fle_screen.h) and every top-level window is SENT WM_DISPLAYCHANGE (wParam the colour depth in bits, lParam LOWORD width / HIWORD height) and then WM_SETTINGCHANGE (wParam SPI_SETWORKAREA, lParam 0). Returns 1 when queued, 0 when
 * the queue is full or the size is outside the limits of fle_screen.h. Nothing is moved by the runtime: what the guest does with the message is the guest's. */
int fle_input_host_display(uint32_t w, uint32_t h);
/* Decision 0267 (alpha10, PROVISIONAL): the TASK BAR of the virtual machine. The host (the page) shows a button for each window that has one and, when the user presses it, tells the machine what the task bar tells a window in Windows:
 * fle_win_taskbar_buttons lists the buttons (top-level, visible, WS_EX_APPWINDOW or unowned without WS_EX_TOOLWINDOW; creation order; fle_taskbar_button below; no class name, no window kind); fle_input_host_syscommand(hwnd, cmd)
 * queues WM_SYSCOMMAND with wParam = cmd and lParam 0 for the window (cmd one of SC_MINIMIZE 0xF020, SC_MAXIMIZE 0xF030, SC_CLOSE 0xF060, SC_RESTORE 0xF120; the task bar's restore of a minimized window is SC_RESTORE). The message is SENT
 * to the window when its thread next retrieves messages (like the display change above), so the window procedure sees it as it does from the shell; DefWindowProcW turns SC_MINIMIZE / SC_MAXIMIZE / SC_RESTORE into ShowWindow and
 * SC_CLOSE into WM_CLOSE (Learn WM_SYSCOMMAND). Returns 1 when queued; 0 when `hwnd` is not a live top-level window, `cmd` is none of the four, or the event queue is full. */
typedef struct { uint64_t hwnd; uint32_t iconic; uint32_t zoomed; uint32_t lid; uint32_t pad; } fle_taskbar_button;
unsigned fle_win_taskbar_buttons(fle_taskbar_button *out, unsigned max);
int fle_input_host_syscommand(uint64_t hwnd, uint32_t cmd);
/* The number of events not yet turned into messages. */
unsigned fle_input_host_pending(void);
/* Decision 0280, L1c (PROVISIONAL): 1 when every event that the host queued has been taken out of the queue and every message made of it has been retrieved, and the thread that retrieved the last one waits in its message loop again (the guest has handled the input and is waiting for the next one); a read only, no state is changed */
int fle_input_settled(void);

/* ---- what the runtime asks (weak in the callers) ---- */
int fle_input_has_event(unsigned thread) FLE_INPUT_REF;                         /* an event for the thread is waiting (or a hover is due) */
uint64_t fle_input_next_tick(unsigned thread) FLE_INPUT_REF;                    /* the earliest hover due time of a window of the thread (virtual ticks), 0 = none */
int fle_input_synth(unsigned t, uint64_t fh, uint32_t fmin, uint32_t fmax, int remove, uint32_t qs, fle_tmsg *out) FLE_INPUT_REF;   /* input messages for GetMessage / PeekMessage; 1 = *out filled */
int fle_input_defproc(const char *api, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp, uint64_t *result) FLE_INPUT_REF;      /* DefWindowProcW default of the mouse and key messages; 1 = handled */
int fle_input_translate(const char *api, uint64_t pmsg) FLE_INPUT_REF;          /* TranslateMessage: 1 when a character message was posted */
void fle_input_note_message(unsigned thread, uint32_t time, int32_t ptx, int32_t pty) FLE_INPUT_REF;                                  /* the message GetMessage / PeekMessage just returned */
uint64_t fle_input_key_state(uint32_t vk, int async) FLE_INPUT_REF;             /* GetKeyState / GetAsyncKeyState value */
void fle_input_pointer(int32_t *x, int32_t *y) FLE_INPUT_REF;                   /* the pointer position */
void fle_input_window_gone(uint64_t hwnd) FLE_INPUT_REF;                        /* a window was destroyed or hidden: drop its tracking */
/* comctl32!_TrackMouseEvent's state (fle_comctl.c is the API): TME_* in flags; returns 1 and fills *out for TME_QUERY */
typedef struct { uint32_t flags; uint64_t hwnd; uint32_t hover_ms; } fle_input_track_t;
int fle_input_track(uint32_t flags, uint64_t hwnd, uint32_t hover_ms, fle_input_track_t *query);
/* the WM_TIMER log (the delivery times of the window timers, for the G3 record) */
typedef struct { uint64_t hwnd, id; uint32_t period_ms; uint64_t due_ticks, delivered_ticks; } fle_timer_log_t;
unsigned fle_win_timer_log_count(void);
int fle_win_timer_log_get(unsigned i, fle_timer_log_t *out);

/* ---- fle_win.c accessors ---- */
typedef struct {
    int live, enabled, visible;
    unsigned owner;
    uint32_t style, exstyle, class_style;
    int32_t ox, oy, cx, cy;          /* the client origin on the screen and the size */
    uint64_t parent, root, hcursor;  /* the parent window (0 = top-level), the top-level ancestor, the class cursor */
    uint32_t lid;                    /* the logical window id (the trace) */
    char cls[40];                    /* the class name, ASCII (the trace) */
} fle_win_iview;
int fle_win_input_view(uint64_t hwnd, fle_win_iview *v);
uint64_t fle_win_input_hit(int32_t x, int32_t y);                 /* the deepest visible window that contains the point (disabled windows are found), 0 = none */
uint64_t fle_win_input_send(const char *api, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp);
void fle_win_input_showwindow(const char *api, uint64_t hwnd, int cmd);   /* decision 0267: the ShowWindow that DefWindowProcW makes of WM_SYSCOMMAND SC_MINIMIZE / SC_MAXIMIZE / SC_RESTORE */
unsigned fle_win_input_remaximize(int32_t dw, int32_t dh);        /* decision 0212: the zoomed top-level windows grow by the change of the screen; returns how many moved */
uint64_t fle_win_input_capture_any(unsigned *thread);             /* the window that holds the mouse capture of some thread, 0 = none */
uint64_t fle_win_input_active(void);                              /* the active window of the foreground thread, 0 = none */
uint64_t fle_win_input_key_target(unsigned *thread);              /* the focus window of the foreground thread (else its active window), 0 = none */
void fle_win_input_activate(const char *api, uint64_t hwnd, uint64_t how);   /* activate the top-level window of hwnd (WM_NCACTIVATE, WM_ACTIVATE how) */
unsigned fle_win_input_toplevels(uint64_t *handles, unsigned max);   /* decision 0141: the live top-level windows in z-order, top first (read-only) */
void fle_win_input_text(uint64_t hwnd, char *out, unsigned cap, uint32_t *owner_lid);   /* decision 0141: the window text (ASCII) and the lid of its owner window (0 = none) */
int fle_win_host_timer(uint64_t hwnd, uint64_t id, uint32_t ms);   /* a timer for a window of the table made by the host (a probe, not a guest call) */
void fle_win_pointer_set(int32_t x, int32_t y);
void fle_win_pointer_get(int32_t *x, int32_t *y);
uint64_t fle_class_hcursor_of(uint16_t atom);                     /* fle_class.c */

#define FLE_WM_NCHITTEST      0x0084u
#define FLE_WM_SETCURSOR      0x0020u
#define FLE_WM_MOUSEACTIVATE  0x0021u
#define FLE_WM_NCMOUSEMOVE    0x00A0u
#define FLE_WM_MOUSEMOVE      0x0200u
#define FLE_WM_LBUTTONDOWN    0x0201u
#define FLE_WM_MOUSEWHEEL     0x020Au
#define FLE_WM_NCMOUSELEAVE   0x02A2u
#define FLE_WM_MOUSEHOVER     0x02A1u
#define FLE_WM_MOUSELEAVE     0x02A3u
#define FLE_WM_KEYDOWN        0x0100u
#define FLE_WM_KEYUP          0x0101u
#define FLE_WM_CHAR           0x0102u
#define FLE_WM_SYSKEYDOWN     0x0104u
#define FLE_WM_SYSKEYUP       0x0105u
#define FLE_WM_SYSCHAR        0x0106u
#define FLE_WM_SYSCOMMAND     0x0112u
#define FLE_WM_DISPLAYCHANGE  0x007Eu   /* Learn WM_DISPLAYCHANGE (winuser.h value) */
#define FLE_WM_SETTINGCHANGE  0x001Au   /* Learn WM_SETTINGCHANGE; WM_WININICHANGE is the same number */
#define FLE_SPI_SETWORKAREA   0x002Fu
#define FLE_HTERROR           (-2)
#define FLE_HTTRANSPARENT     (-1)
#define FLE_HTNOWHERE         0
#define FLE_HTCLIENT          1
#define FLE_MA_ACTIVATE       1u
#define FLE_MA_ACTIVATEANDEAT 2u
#define FLE_MA_NOACTIVATE     3u
#define FLE_MA_NOACTIVATEANDEAT 4u
#define FLE_TME_HOVER         0x00000001u
#define FLE_TME_LEAVE         0x00000002u
#define FLE_TME_NONCLIENT     0x00000010u
#define FLE_TME_QUERY         0x40000000u
#define FLE_TME_CANCEL        0x80000000u
#define FLE_HOVER_DEFAULT     0xFFFFFFFFu
#define FLE_QS_INPUT_BITS     0x0007u


/* Decision 0171 (PROVISIONAL): user32!SetKeyboardState: BOOL f(LPBYTE rcx); copies 256 bytes into the table that GetKeyState and GetKeyboardState read. user32!GetKeyboardState: BOOL f(PBYTE rcx). See fle_input.c. */
void fle_imp_SetKeyboardState(void);
void fle_imp_GetKeyboardState(void);

#endif
