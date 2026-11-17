# Learn pages used by decision 0323 (quotes; read 2026-11-07 by ci1, webfetch of learn.microsoft.com)

Pages already in the repository (text copies of earlier decisions): `docs/investigations/alpha6/tables/learn/wm-erasebkgnd.txt`, `.../wm-nchittest.txt`, `docs/investigations/pin1/logs/learn/SetWindowsHookExW.txt`.
Pages fetched for this decision (not in the repository before): WNDCLASSW, MouseProc, KeyboardProc. The quotes below are the whole of what the decision rests on.

## WM_ERASEBKGND (https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-erasebkgnd)
- "Return value: An application should return nonzero if it erases the background; otherwise, it should return zero."
- "The DefWindowProc function erases the background by using the class background brush specified by the hbrBackground member of the WNDCLASS structure. If hbrBackground is NULL, the application should process the WM_ERASEBKGND message and erase the background."
- "An application should return nonzero in response to WM_ERASEBKGND if it processes the message and erases the background; this indicates that no further erasing is required. If the application returns zero, the window will remain marked for erasing. (Typically, this indicates that the fErase member of the PAINTSTRUCT structure will be TRUE.)"
- wParam: "A handle to the device context."

## WNDCLASSW, hbrBackground (https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-wndclassw)
- "A handle to the class background brush. This member can be a handle to the physical brush to be used for painting the background, or it can be a color value. A color value must be one of the following standard system colors (the value 1 must be added to the chosen color)." (COLOR_ACTIVEBORDER ... COLOR_WINDOWTEXT)
- "When this member is NULL, an application must paint its own background whenever it is requested to paint in its client area. To determine whether the background must be painted, an application can either process the WM_ERASEBKGND message or test the fErase member of the PAINTSTRUCT structure filled by the BeginPaint function."

## WM_PAINT / BeginPaint (pages read by decisions 0123 and 0142; the sentences used here)
- WM_PAINT: "The DefWindowProc function validates the update region. The function may also send the WM_NCPAINT message to the window procedure if the window frame must be painted and send the WM_ERASEBKGND message if the window background must be erased."
- BeginPaint: "automatically sets the clipping region of the device context to exclude any area outside the update region"; "If the update region is marked for erasing, BeginPaint sends a WM_ERASEBKGND message to the window".

## WM_NCHITTEST, return value (docs/investigations/alpha6/tables/learn/wm-nchittest.txt:58-59)
- "HTTRANSPARENT -1: In a window currently covered by another window in the same thread (the message will be sent to underlying windows in the same thread until one of them returns a code that is not HTTRANSPARENT)."

## MouseProc (https://learn.microsoft.com/en-us/windows/win32/winmsg/mouseproc)
- "The system calls this function whenever an application calls the GetMessage or PeekMessage function and there is a mouse message to be processed."
- nCode: HC_ACTION 0 "The wParam and lParam parameters contain information about a mouse message."; HC_NOREMOVE 3 "... and the mouse message has not been removed from the message queue. (An application called the PeekMessage function, specifying the PM_NOREMOVE flag.)"; "If nCode is less than zero, the hook procedure must pass the message to the CallNextHookEx function without further processing and should return the value returned by CallNextHookEx."
- wParam: "The identifier of the mouse message."; lParam: "A pointer to a MOUSEHOOKSTRUCT structure."
- Returns: "If the hook procedure processed the message, it may return a nonzero value to prevent the system from passing the message to the target window procedure." "If nCode is greater than or equal to zero, and the hook procedure did not process the message, it is highly recommended that you call CallNextHookEx and return the value it returns."
- "This hook may be called in the context of the thread that installed it. The call is made by sending a message to the thread that installed the hook."

## KeyboardProc (https://learn.microsoft.com/en-us/windows/win32/winmsg/keyboardproc)
- "The system calls this function whenever an application calls the GetMessage or PeekMessage function and there is a keyboard message (WM_KEYUP or WM_KEYDOWN) to be processed."
- code: HC_ACTION 0, HC_NOREMOVE 3 (same wording as MouseProc, "keystroke message").
- wParam: "The virtual-key code of the key that generated the keystroke message."; lParam: "The repeat count, scan code, extended-key flag, context code, previous key-state flag, and transition-state flag" (bits 0-15 repeat count, 16-23 scan code, 24 extended, 29 context, 30 previous state, 31 transition state).
- Returns: "If the hook procedure processed the message, it may return a nonzero value to prevent the system from passing the message to the rest of the hook chain or the target window procedure."

## Wine 10.0 (UNOFFICIAL; wine-10.0 tag, gitlab.winehq.org/wine/wine/-/raw/wine-10.0/…; line numbers of those files)
- `dlls/win32u/defwnd.c` 2581-2602 (`WM_ERASEBKGND`/`WM_ICONERASEBKGND`: no brush → result 0; CS_PARENTDC → client rectangle; else `NtGdiGetAppClipBox`; `fill_rect` with a system-colour brush for hbr ≤ COLOR_MENUBAR + 1; return 1), 2516-2536 (`WM_PAINT`: `NtUserBeginPaint` … `NtUserEndPaint`), 73-80 (`fill_rect`).
- `dlls/win32u/dce.c` 1590-1620 (`send_erase`: `NtUserGetDCEx(hwnd, client_rgn, DCX_INTERSECTRGN | DCX_USESTYLE)`, no message when the clip box is empty, `need_erase = !send_message(WM_ERASEBKGND)`; `UPDATE_DELAYED_ERASE`), 1685-1710 (`NtUserBeginPaint`: `ps->fErase = erase`), 1729-1750 (`erase_now`).
- `dlls/win32u/window.c` 2572-2620 (`window_from_point`: disabled → `HTERROR`; a window of another thread → `HTCLIENT`; `WM_NCHITTEST`; `HTTRANSPARENT` → "continue search with next window in z-order").
- `server/window.c` 891-905 (`is_point_in_window`: a disabled child is skipped), 982-1003 (`get_window_children_from_point`: the children that contain the point come before the window itself).
- `dlls/win32u/message.c` 2419-2428 (`process_keyboard_message`: the message is removed before the hook when `remove`; `WH_KEYBOARD` with `HC_ACTION` / `HC_NOREMOVE`, wParam = LOWORD(wParam), lParam; nonzero → the message is dropped), 2570-2587 (`process_mouse_message`: `WH_MOUSE` with the `MOUSEHOOKSTRUCTEX`, nonzero → dropped).
