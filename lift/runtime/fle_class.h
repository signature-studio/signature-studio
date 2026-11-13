/* The virtual window class table: user32!RegisterClassW and user32!GetClassInfoW (docs/decisions/0070-virtual-window-classes.md,
 * PROVISIONAL; the pages read and the guest's call sites are in docs/investigations/getclassinfo-caller.md). Same convention as
 * fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8); the Win32 return value in R[0]; the guest return address stays on
 * the stack for the caller's ret. Single guest thread (a call from a second thread stops the run).
 *
 * The table is a pure function of the calls the guest made: it reads nothing from the host (no real class, no real window, no
 * module). RegisterClassW keeps a copy of the 72-byte WNDCLASSW; lpfnWndProc is kept as an address and is never called here (a
 * window procedure is called only by CreateWindowExW and later, which this runtime does not answer). Every form the pages leave
 * open stops the run through fle_stop (fle_env.h): "fle: <api>: <why>" and abort(). No value is made up for it. The last error
 * is never touched (the pages give no value for a failure). */
#ifndef FLE_CLASS_H
#define FLE_CLASS_H

#include <stdint.h>

/* sizeof(WNDCLASSW) on x64 (Learn WNDCLASSW): UINT style @0, WNDPROC @8, int cbClsExtra @16, int cbWndExtra @20, HINSTANCE @24,
 * HICON @32, HCURSOR @40, HBRUSH @48, LPCWSTR lpszMenuName @56, LPCWSTR lpszClassName @64 = 72 (OFFICIAL layout, 8-byte pointers). */
#define FLE_CLASS_WNDCLASSW_SIZE 72

/* Learn WNDCLASSW: "The maximum length for lpszClassName is 256. If lpszClassName is greater than the maximum length, the
 * RegisterClass function will fail." The failure has no GetLastError value on the page, so a longer name stops the run. */
#define FLE_CLASS_NAME_MAX_UNITS 256

/* Learn About Window Classes: "RegisterClassEx fails if the amount of extra class memory requested is greater than 40 bytes"
 * (and the same for extra window memory). A value outside 0..40 stops the run. */
#define FLE_CLASS_EXTRA_MAX 40

/* Atoms of the registered classes (a PROVISIONAL choice): the n-th class (n = 0, 1, ...) gets 0xC000 + n, never reused. Learn
 * "About Atom Tables": the values of string atoms are 0xC000 (MAXINTATOM) through 0xFFFF; "registered classes are stored in an
 * internal atom table ... an application adds atoms to this atom table using RegisterClass". The page gives no value for the
 * first atom. When 0xFFFF has been issued, the next RegisterClassW stops the run. */
#define FLE_CLASS_ATOM_FIRST 0xC000u
#define FLE_CLASS_ATOM_LAST  0xFFFFu

/* user32!RegisterClassW: ATOM f(const WNDCLASSW *lpWndClass). rcx.
 * user32!GetClassInfoW: BOOL f(HINSTANCE hInstance, LPCWSTR lpClassName, LPWNDCLASSW lpWndClass). rcx, rdx, r8. */
void fle_imp_RegisterClassW(void);
void fle_imp_GetClassInfoW(void);

/* Decision 0104: 1 when h is a module handle that can own classes and windows: the lifted DLL (FLE_EXE_HANDLE) or the base of a real module
 * (fle_realmod.h); 0 otherwise. */
int fle_class_hinst_known(uint64_t h);

/* Inspection (for tests and tools; no state moves): the number of registered classes. */
uint64_t fle_class_count(void);

/* Added by decision 0071 (virtual windows, fle_win.c): the question CreateWindowExW asks of the table. `name` is the guest's
 * lpClassName, `hinst` the hInstance of the window being created, `api` the name used in a stop message. Returns 1 and fills
 * *atom and *wndproc (the lpfnWndProc of the registered WNDCLASSW, as a number) when a class of exactly that name is registered by
 * hinst; returns 0 when the name is one of the guest's own static names (the list of GetClassInfoW) and is not registered yet
 * (CreateWindowExW then fails with NULL). Every other form stops the run by name: hinst other than FLE_EXE_HANDLE, an atom or
 * NULL or empty or too long or non-ASCII name, a name that differs from a registered one only in case, any other unregistered
 * name (a predefined class? another module's class?). No state moves, the last error is not touched. */
int fle_class_lookup(const char *api, uint64_t name, uint64_t hinst, uint16_t *atom, uint64_t *wndproc);

/* Decision 0119 (wall 15): WNDCLASSW.hIcon is at offset 32 (the layout above), and the one value of a registered class that fle_win.c's SetClassLongPtrW replaces:
 * *prev = the previous hIcon, the class `atom` has `value` as its hIcon from now on (GetClassInfoW reports it). An atom that is not in the table stops by name. */
#define FLE_CLASS_WC_OFF_HICON 32
void fle_class_set_hicon(const char *api, uint16_t atom, uint64_t value, uint64_t *prev);

/* Decision 0123 (PROVISIONAL): the name of the class that has `atom` (fle_win.c: FindWindowW compares class names): 1 and the name units when the atom is in the table, 0 otherwise. No state moves.
 * user32!UnregisterClassW: BOOL f(LPCWSTR lpClassName rcx, HINSTANCE hInstance rdx). Learn UnregisterClassW: nonzero on success; zero "If the class could not be found or if a window still exists that was created with
 * the specified class" (the pages name no code: 1411 ERROR_CLASS_DOES_NOT_EXIST and 1412 ERROR_CLASS_HAS_WINDOWS are a choice from the table of System Error Codes); "No window classes registered by a DLL are
 * unregistered when the .dll is unloaded". The class entry is marked removed (its atom is not reused: the table never reuses an atom, as RegisterClassW). A name that fle_class.c does not hold stops when it may be a
 * predefined class ("System classes ... cannot be unregistered": a name of the list of About Window Classes is answered 0 with 1411 as a class that is not in the table of the module: a choice). */
int fle_class_name_of(uint16_t atom, uint16_t *out, uint16_t *len);
/* Decision 0125 (PROVISIONAL): the `style` member of the WNDCLASSW that registered `atom` (fle_win.c: GetDC and the other DC calls read CS_OWNDC, CS_CLASSDC and CS_PARENTDC from it): 1 and the style when the atom is in the
 * table, 0 otherwise. No state moves. */
int fle_class_style_of(uint16_t atom, uint32_t *style);
/* Decision 0323: the `hbrBackground` member of the WNDCLASSW that registered `atom` (offset 48): 1 and the value (a brush handle, a system colour index plus 1, or 0 = none) when the atom is in the table, 0 otherwise.
 * fle_win.c: DefWindowProc(WM_ERASEBKGND) erases with it. No state moves. */
#define FLE_CLASS_WC_OFF_HBRBACKGROUND 48
int fle_class_hbrbackground_of(uint16_t atom, uint64_t *hbr);
void fle_imp_UnregisterClassW(void);
/* fle_win.c: the number of live windows that have the class `atom`. A weak reference in fle_class.c: a link without fle_win.c (the unit tests of the class table) has no windows, so the count is 0. */
uint32_t fle_win_count_of_atom(uint16_t atom) __attribute__((weak));

#endif
