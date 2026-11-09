/* Walls of the second wave of user operations (decision 0191, PROVISIONAL). See fle_wall_ia2.h. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_wall_ia2.h"

/* ---- user32!GetKeyNameTextW (wall ia2-W18) ----
 * Learn (winuser.h, "GetKeyNameTextW function"): lParam "the second parameter of the keyboard message": bits 16-23 the scan code, bit 24 "whether the key is an extended key, such as the right-hand ALT and CTRL
 * keys", bit 25 a "do not care" bit that "the application calling this function sets ... to indicate that the function should not distinguish between left and right CTRL and SHIFT keys, for example"; lpString
 * receives "a null-terminated string", the return value is "the length of the string, in characters, not counting the terminating null character", 0 on failure. Remarks: "The format of the key-name string depends
 * on the current keyboard layout. The keyboard layout maintains a list of names in the form of character strings for keys with names longer than a single character ... The name of a character key is the character
 * itself." The virtual machine's layout is the US layout (decision 0016, S-us-layout): the names are the strings of aKeyNames[] and aKeyNamesExt[] of Microsoft's sample layout kbdus.c (microsoft/Windows-driver-samples,
 * input/layout/kbdus/kbdus.c, "Only the names of Extended, NumPad, Dead and Non-Printable keys are here. (Keys producing printable characters are named by that character)"; MS-PL; the file read for this decision has
 * SHA-256 5a271ed235e0034fd4be0d919e22d90d795fc1a4cc07f44f749f0d4375fc3f96). A key that produces a character is named by the character it produces without a modifier (a letter: the upper case letter, Learn: "Character keys that
 * are mapped to the 'A'..'Z' virtual-key codes are translated to the ... characters 'A'..'Z'"; a digit or an OEM key: the column "no modifier" of kbdus.c's aVkToWch2 / aVkToWch3 / aVkToWch4). The scan code of each such
 * key is the one MapVirtualKeyW of fle_input.c gives (set 1; the printable block of the PC keyboard, S-ia2-scan-set1: not read from a Microsoft page here, it is the table fle_input.c already uses).
 * Choices without a source (S-ia2-keyname-*): (1) the "do not care" bit makes the right Ctrl / Shift / Alt (scan 0x1D, 0x2A, 0x36, 0x38, extended or not) answer the name of the left key ("Ctrl", "Shift", "Alt"); the page
 * names Ctrl and Shift, Alt by analogy; (2) a key that has no name in these tables (and no character) stops by name: the page gives no failure code for it; (3) a buffer that cannot hold the name and its terminating NUL
 * stops by name: the page says only that the parameter "should be equal to the size of the buffer". */
typedef struct { uint8_t scan; const char *name; } keyname_t;
static const keyname_t names_plain[] = {
    { 0x01, "Esc" }, { 0x0e, "Backspace" }, { 0x0f, "Tab" }, { 0x1c, "Enter" }, { 0x1d, "Ctrl" }, { 0x2a, "Shift" }, { 0x36, "Right Shift" }, { 0x37, "Num *" }, { 0x38, "Alt" }, { 0x39, "Space" },
    { 0x3a, "Caps Lock" }, { 0x3b, "F1" }, { 0x3c, "F2" }, { 0x3d, "F3" }, { 0x3e, "F4" }, { 0x3f, "F5" }, { 0x40, "F6" }, { 0x41, "F7" }, { 0x42, "F8" }, { 0x43, "F9" }, { 0x44, "F10" },
    { 0x45, "Pause" }, { 0x46, "Scroll Lock" }, { 0x47, "Num 7" }, { 0x48, "Num 8" }, { 0x49, "Num 9" }, { 0x4a, "Num -" }, { 0x4b, "Num 4" }, { 0x4c, "Num 5" }, { 0x4d, "Num 6" }, { 0x4e, "Num +" },
    { 0x4f, "Num 1" }, { 0x50, "Num 2" }, { 0x51, "Num 3" }, { 0x52, "Num 0" }, { 0x53, "Num Del" }, { 0x54, "Sys Req" }, { 0x57, "F11" }, { 0x58, "F12" }, { 0x7c, "F13" }, { 0x7d, "F14" }, { 0x7e, "F15" },
    { 0x7f, "F16" }, { 0x80, "F17" }, { 0x81, "F18" }, { 0x82, "F19" }, { 0x83, "F20" }, { 0x84, "F21" }, { 0x85, "F22" }, { 0x86, "F23" }, { 0x87, "F24" },
};
static const keyname_t names_ext[] = {
    { 0x1c, "Num Enter" }, { 0x1d, "Right Ctrl" }, { 0x35, "Num /" }, { 0x37, "Prnt Scrn" }, { 0x38, "Right Alt" }, { 0x45, "Num Lock" }, { 0x46, "Break" }, { 0x47, "Home" }, { 0x48, "Up" }, { 0x49, "Page Up" },
    { 0x4b, "Left" }, { 0x4d, "Right" }, { 0x4f, "End" }, { 0x50, "Down" }, { 0x51, "Page Down" }, { 0x52, "Insert" }, { 0x53, "Delete" }, { 0x54, "<00>" }, { 0x56, "Help" }, { 0x5b, "Left Windows" },
    { 0x5c, "Right Windows" }, { 0x5d, "Application" },
};
/* scan 0x02.. : the character of the key without a modifier (US layout) */
static const char main_row_1[] = "1234567890-=";          /* 0x02 .. 0x0D */
static const char main_row_2[] = "QWERTYUIOP[]";          /* 0x10 .. 0x1B */
static const char main_row_3[] = "ASDFGHJKL;'`";          /* 0x1E .. 0x29 */
static const char main_row_4[] = "ZXCVBNM,./";            /* 0x2C .. 0x35 */

static int key_char(unsigned scan)
{
    if (scan >= 0x02 && scan <= 0x0d) return main_row_1[scan - 0x02];
    if (scan >= 0x10 && scan <= 0x1b) return main_row_2[scan - 0x10];
    if (scan >= 0x1e && scan <= 0x29) return main_row_3[scan - 0x1e];
    if (scan == 0x2b) return '\\';
    if (scan >= 0x2c && scan <= 0x35) return main_row_4[scan - 0x2c];
    return 0;
}

void fle_imp_GetKeyNameTextW(void)
{
    static const char api[] = "GetKeyNameTextW";
    uint32_t lp = (uint32_t)R[1];
    uint64_t buf = R[2];
    int32_t cch = (int32_t)R[8];
    unsigned scan = (lp >> 16) & 0xFFu, ext = (lp >> 24) & 1u, dontcare = (lp >> 25) & 1u;
    const char *name = NULL;
    char one[2] = { 0, 0 };
    size_t i, n;

    if (dontcare && (scan == 0x1d || scan == 0x2a || scan == 0x36 || scan == 0x38)) {
        name = scan == 0x1d ? "Ctrl" : scan == 0x38 ? "Alt" : "Shift";
    } else if (ext) {
        for (i = 0; i < sizeof names_ext / sizeof names_ext[0]; i++) if (names_ext[i].scan == scan) name = names_ext[i].name;
    } else {
        for (i = 0; i < sizeof names_plain / sizeof names_plain[0]; i++) if (names_plain[i].scan == scan) name = names_plain[i].name;
        if (name == NULL && key_char(scan)) { one[0] = (char)key_char(scan); name = one; }
    }
    if (name == NULL) fle_stop(api, "the scan code / extended flag has no name in the tables of the US layout (kbdus.c) and produces no character here: the page gives no failure code");
    if (buf == 0) fle_stop(api, "lpString is NULL: the page gives no behaviour");
    n = strlen(name);
    if (cch < 0 || (size_t)cch < n + 1) fle_stop(api, "cchSize cannot hold the key name and its NUL: the page gives no behaviour");
    for (i = 0; i < n; i++) fle_st(buf + 2 * i, 2, (uint8_t)name[i]);
    fle_st(buf + 2 * n, 2, 0);
    R[0] = n;
}


/* ---- kernel32!LoadLibraryW with a relative path (wall ia2-W21: the channel rack's settings icon asks for "Python\\Python312.dll") ----
 * Learn (LoadLibraryW): a relative name with a path is searched by the standard search order; "If the function fails, the return value is NULL. To get extended error information, call GetLastError"; System Error Codes:
 * ERROR_MOD_NOT_FOUND 126 "The specified module could not be found." The product's installer does not hold Python312.dll (docs/investigations/external-modules-usage.md, section 3: python312.dll, "Python\" prepended: no
 * EXTRACTFILE entry), the virtual volume holds no module file, so the module is not found: NULL and 126. Only the names of this list are answered so; any other name with a path stops as before (S-ia2-loadlib-path:
 * a CHOICE of the list; a real machine that has the file would load a third-party module that is not lifted). (Decision 0284: no longer behind the layer-4 marker row; every shipped configuration had it.) */
int fle_ia2_path_absent(const uint16_t *u, size_t n)
{
    static const char *const absent[] = { "python\\python312.dll" };
    size_t k, i;

    for (k = 0; k < sizeof absent / sizeof absent[0]; k++) {
        size_t m = strlen(absent[k]);
        if (n != m) continue;
        for (i = 0; i < m; i++) {
            uint16_t c = u[i];
            if (c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
            if (c != (uint16_t)(unsigned char)absent[k][i]) break;
        }
        if (i == m) return 1;
    }
    return 0;
}

/* ---- user32!MessageBoxW (wall ia2-W28: Tools > Riff machine) ----
 * int f(HWND hWnd rcx, LPCWSTR lpText rdx, LPCWSTR lpCaption r8, UINT uType r9d). Learn (winuser.h, "MessageBox function"): "Displays a modal dialog box that contains a system icon, a set of buttons, and a brief
 * application-specific message"; uType: the buttons MB_OK 0x0, MB_OKCANCEL 0x1, MB_ABORTRETRYIGNORE 0x2, MB_YESNOCANCEL 0x3, MB_YESNO 0x4, MB_RETRYCANCEL 0x5, MB_CANCELTRYCONTINUE 0x6 (+ MB_HELP 0x4000 adds Help); the default
 * button MB_DEFBUTTON1 0x0, MB_DEFBUTTON2 0x100, MB_DEFBUTTON3 0x200, MB_DEFBUTTON4 0x300; the return value is the ID of the button the user chose: IDOK 1, IDCANCEL 2, IDABORT 3, IDRETRY 4, IDIGNORE 5, IDYES 6, IDNO 7, IDHELP 9,
 * IDTRYAGAIN 10, IDCONTINUE 11. The buttons of each type, in the order they are shown, are the ones the Remarks and the constant list give.
 * S-ia2-msgbox-default (a CHOICE WITHOUT A SOURCE for what the virtual machine does; alternatives: show the text on the page, or fix the answer to Cancel/No): NO WINDOW IS SHOWN and nobody chooses. The call answers at once with the ID of the DEFAULT button, as if the user pressed Enter, and writes one
 * line with the caller VA (the guest return address), the type, the caption and the text on stderr EVERY TIME (a failure the engine reports through a message box must not pass unseen). A message box that the user is meant to read and decide on is therefore decided by this rule: the engine goes on as if the default button had been chosen. The text, the caption and
 * the type are real (they are the engine's own); the choice is not. (The browser build needs a modal-dialog host to replace this.) MB_HELP's Help button is the 4th button; a default-button index beyond the buttons of the
 * type is the first button (the pages give no rule). The last error and the window handle are not touched or checked. */
static void msg_ascii(char *out, size_t cap, uint64_t p)
{
    size_t i = 0;

    if (p == 0) { snprintf(out, cap, "(NULL)"); return; }
    for (; i + 1 < cap; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2 * i, 2);
        if (c == 0) break;
        out[i] = c >= 0x20 && c < 0x7f ? (char)c : (c == '\n' ? '/' : '?');
    }
    out[i] = 0;
}

void fle_imp_MessageBoxW(void)
{
    static const uint8_t btn[7][4] = { { 1, 0, 0, 0 }, { 1, 2, 0, 0 }, { 3, 4, 5, 0 }, { 6, 7, 2, 0 }, { 6, 7, 0, 0 }, { 4, 2, 0, 0 }, { 2, 10, 11, 0 } };
    static const uint8_t nbtn[7] = { 1, 2, 3, 3, 2, 2, 3 };
    uint32_t type = (uint32_t)R[9], kind = type & 0xFu, def = (type >> 8) & 3u;
    char text[300], cap[120];
    uint32_t id;

    if (kind > 6) fle_stop("MessageBoxW", "uType holds a button set that the page does not list");
    msg_ascii(text, sizeof text, R[2]);
    msg_ascii(cap, sizeof cap, R[8]);
    if (def >= nbtn[kind]) def = 0;
    id = btn[kind][def];
    fprintf(stderr, "fle: MessageBoxW NOT SHOWN, answered by the virtual machine with the default button (id %u; S-ia2-msgbox-default), type %#x, caller %#llx, caption \"%s\", text \"%s\"\n", (unsigned)id, (unsigned)type, (unsigned long long)fle_ld(R[4], 8), cap, text);
    R[0] = id;
}

/* ---- shell32!ShellExecuteW (wall ia2-W16 of decision 0171, met again by Help and Tools > Cloud items) ----
 * HINSTANCE f(HWND hwnd rcx, LPCWSTR lpOperation rdx, LPCWSTR lpFile r8, LPCWSTR lpParameters r9, LPCWSTR lpDirectory [5], INT nShowCmd [6]). Learn (shellapi.h, "ShellExecuteW function"): "Performs an operation on a specified
 * file"; the return value is "a value greater than 32 if successful, or an error value that is less than or equal to 32": 0 out of memory, ERROR_FILE_NOT_FOUND / SE_ERR_FNF 2, ERROR_PATH_NOT_FOUND / SE_ERR_PNF 3, ERROR_BAD_FORMAT 11,
 * SE_ERR_ACCESSDENIED 5, SE_ERR_OOM 8, SE_ERR_SHARE 26, SE_ERR_ASSOCINCOMPLETE 27, SE_ERR_DDETIMEOUT 28, SE_ERR_DDEFAIL 29, SE_ERR_DDEBUSY 30, SE_ERR_NOASSOC 31 ("There is no application associated with the given file name
 * extension"), SE_ERR_DLLNOTFOUND 32. S-ia2-shellexecute-noassoc (a CHOICE WITHOUT A SOURCE): the virtual machine has no program for a URL or a file type (its registry holds no association: decision 0129) and no process is ever
 * started; the call answers SE_ERR_NOASSOC (31) and writes one line with the verb, the file and the parameters on stderr. This is NOT 'the browser opened': the engine sees a failure and reacts to it. A host that
 * opens URLs would replace it. A NULL lpFile stops (the page gives no behaviour). */
void fle_imp_ShellExecuteW(void)
{
    char op[40], file[300], par[200];

    if (R[8] == 0) fle_stop("ShellExecuteW", "lpFile is NULL: the page gives no behaviour");
    msg_ascii(op, sizeof op, R[2]);
    msg_ascii(file, sizeof file, R[8]);
    msg_ascii(par, sizeof par, R[9]);
    fprintf(stderr, "fle: ShellExecuteW NOT CARRIED OUT (no association in the virtual machine; S-ia2-shellexecute-noassoc): verb \"%s\", file \"%s\", parameters \"%s\"; answered SE_ERR_NOASSOC (31)\n", op, file, par);
    R[0] = 31;
}

/* ---- ole32!DoDragDrop (wall ia2-W24: dragging a pattern from the playlist's pattern list) ----
 * HRESULT f(LPDATAOBJECT pDataObj rcx, LPDROPSOURCE pDropSource rdx, DWORD dwOKEffects r8d, LPDWORD pdwEffect r9). Learn (ole2.h, "DoDragDrop function"): returns S_OK on success; DRAGDROP_S_DROP 0x00040100 "successful",
 * DRAGDROP_S_CANCEL 0x00040101 "canceled"; "The pdwEffect parameter is set only if the operation is not canceled"; the loop calls IDropTarget::DragEnter / DragOver / DragLeave / Drop and IDropSource::QueryContinueDrag /
 * GiveFeedback. S-ia2-dragdrop-cancel (a CHOICE WITHOUT A SOURCE for what the virtual machine does): the loop is NOT modelled; the call answers DRAGDROP_S_CANCEL at once, as if the user cancelled the drag before the
 * pointer entered any target: no IDropSource or IDropTarget method is called, *pdwEffect is not written (the page: set only if not canceled), the data object is not touched. The drag therefore never completes: dropping
 * a pattern into the playlist does nothing (the engine's state is consistent: it sees a cancelled drag). The loop (the pointer and button state from the host input queue, GiveFeedback, DragOver/Drop on the target
 * registered by RegisterDragDrop) is the next step. */
void fle_imp_DoDragDrop(void)
{
    if (R[2] == 0) fle_stop("DoDragDrop", "pDropSource is NULL: the page gives no behaviour (E_INVALIDARG is a guess)");
    if (R[9] == 0) fle_stop("DoDragDrop", "pdwEffect is NULL: the page gives no behaviour");
    R[0] = 0x00040101u;      /* DRAGDROP_S_CANCEL */
}

