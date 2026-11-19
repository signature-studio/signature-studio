/* Decision 0144 (PROVISIONAL): how a file reaches the engine. FL64.exe never hands a file to FLEngine (decision 0144's investigation, docs/investigations/g8-project-load.md section 2): the engine itself reads the
 * process command line (ParamStr, at start-up) and receives WM_COPYDATA from a second FL64.exe (dwData 10 = a file path, UTF-16; the sender is the engine's own f_117b930). This file gives the virtual machine those
 * two inputs and nothing else:
 *   A. the command line: FLE_CMDLINE (environment; the wasm manifest's engine.env is the same thing) is the whole text GetCommandLineW / GetCommandLineA return. Unset or empty: the old value (fle_env.c), the build behaves as before.
 *   B. WM_COPYDATA: fle_appfile_open(path) puts a COPYDATASTRUCT and the UTF-16 path in guest-readable memory and SENDS (sent-message semantics of decision 0122: the window's procedure runs in the context of the window's own
 *      thread, when that thread executes its message retrieval) WM_COPYDATA to the top-level window of class TFruityLoopsMainForm. The sender is the host, so wParam (the sender's window) is 0 and nobody waits.
 * Nothing here reads a file: the file is whatever the guest finds on the virtual volume (a VFS seed, decision 0110) at the path it is given. Plain C, no thread, no host clock; the same for native and wasm. */
#ifndef FLE_APPFILE_H
#define FLE_APPFILE_H

#include <stdint.h>

#define FLE_APPFILE_MAIN_CLASS "TFruityLoopsMainForm"
#define FLE_WM_COPYDATA 0x004Au
#define FLE_APPFILE_DWDATA_PATH 10u   /* f_117b930 sends 0xA for a path and 0xC for a URL "flstudio:..."; f_11d1c80 (the receiver) takes dwData 10 as a path (docs/investigations/g8-project-load.md) */
#define FLE_APPFILE_MAX_PENDING 8u
#define FLE_APPFILE_PATH_MAX 520u     /* UTF-16 units; MAX_PATH-sized paths of the virtual volume (fle_vfs.c) are shorter */

/* ---- A: the command line (fle_env.c asks these; weak there) ---- */
/* The text of FLE_CMDLINE as UTF-16 (made once, never freed), NULL when FLE_CMDLINE is unset or empty. */
const uint16_t *fle_appfile_cmdline_w(void);
/* The same as the 8-bit process code page text (every character must be ASCII: a character above 0x7F stops the run by name at GetCommandLineA), NULL when FLE_CMDLINE is unset or empty. */
const char *fle_appfile_cmdline_a(void);

/* ---- B: WM_COPYDATA ---- */
/* Sends WM_COPYDATA (dwData = FLE_APPFILE_DWDATA_PATH, lpData = the UTF-16 text of `path_utf8` with its NUL, cbData = (units + 1) * 2, wParam = 0) to the main window. Returns the request number (>= 1), or a negative error:
 *   -1 there is no live top-level window of class TFruityLoopsMainForm yet   -2 the path is empty, not UTF-8 or longer than FLE_APPFILE_PATH_MAX   -3 FLE_APPFILE_MAX_PENDING requests are still waiting   -4 out of memory.
 * The message is processed when the window's thread next retrieves messages; the call returns at once. */
int fle_appfile_open(const char *path_utf8);
/* 0 = unknown request number, 1 = sent and not yet processed, 2 = processed (*result, when not NULL, is the window procedure's answer: TMessage.Result, 1 when the engine took the request). */
int fle_appfile_state(int request, uint64_t *result);
/* The handle of the main window (class TFruityLoopsMainForm), 0 when there is none. */
uint64_t fle_appfile_main_window(void);

/* ---- the APIs that opening a project reaches (rows at the end of crates/fe-lift/src/imports_table.rs) ---- */
void fle_imp_SHAddToRecentDocs(void);   /* shell32!SHAddToRecentDocs: void f(UINT uFlags rcx, LPCVOID pv rdx); no effect the guest can see (S-g8a-recentdocs) */
void fle_imp_GetDpiForMonitor(void);   /* shcore!GetDpiForMonitor: HRESULT f(HMONITOR rcx, MONITOR_DPI_TYPE edx, UINT *dpiX r8, UINT *dpiY r9); 96 / 96 (S-g8a-dpi) */

#endif
