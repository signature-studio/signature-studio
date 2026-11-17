/* Decision 0215 (sweep1, PROVISIONAL, opt-in): the user32 keyboard-layout family (fle_sweep1.h). Found by the systematic sweep: pressing the right Alt key (VK_RMENU, 0xA5) in the main window stops the engine
 * at `user32!GetKeyboardLayoutNameW` (the engine asks for the layout name to decide whether the right Alt is an AltGr key); the Microsoft pages of the whole family are implemented, not only that call.
 * Every answer is that of the ONE input locale of this environment, the virtual one of decision 0063 (fle_kbd.h: HKL 0x04090409 with the emulated locale en-US, the US layout of S-us-layout). Nothing is read
 * from the host. Where a page is silent the call stops by name (fle_stop), as the rest of the runtime does; where a page defines a failure it is returned.
 * Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), the fifth and later at [R[4] + 0x28 + 8 * (n - 5)] (fle_arg); the Win32 return value in R[0]. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_kbd.h"
#include "fle_sweep1.h"
#include "fle_api2.h"

#define KL_NAMELENGTH 9u
#define ERR_INVALID_KEYBOARD_HANDLE 1457u
#define KLF_ACTIVATE 0x1u
#define KLF_SUBSTITUTE_OK 0x2u
#define KLF_REORDER 0x8u
#define KLF_REPLACELANG 0x10u
#define KLF_NOTELLSHELL 0x80u
#define KLF_SETFORPROCESS 0x100u
#define KLF_RESET 0x40000000u
#define KLF_SHIFTLOCK 0x10000u
#define HKL_PREV 0u
#define HKL_NEXT 1u

/* The name of the input locale identifier of this environment: Learn LoadKeyboardLayout: "a string composed of the hexadecimal value of the Language Identifier (low word) and a device identifier (high word).
 * For example, U.S. English has a language identifier of 0x0409, so the primary U.S. English layout is named "00000409"". The virtual layout is the primary layout of its language (device identifier 0). */
static void klid(char out[KL_NAMELENGTH])
{
    snprintf(out, KL_NAMELENGTH, "%08X", (unsigned)(fle_kbd_hkl() & 0xFFFFu));
}

/* user32!GetKeyboardLayoutNameW: BOOL f(LPWSTR pwszKLID rcx). Learn: "Retrieves the name of the active input locale identifier (formerly called the keyboard layout) for the calling thread"; "The buffer (of at least
 * KL_NAMELENGTH characters in length) that receives the name of the input locale identifier, including the terminating null character. This will be a copy of the string provided to the LoadKeyboardLayout
 * function, unless layout substitution took place"; nonzero on success. The active layout is the virtual one: 8 hexadecimal digits and the terminator (9 units, KL_NAMELENGTH). A NULL buffer: the page gives no
 * behaviour, the call stops. The last error is not touched on success. */
void fle_imp_GetKeyboardLayoutNameW(void)
{
    uint64_t p = R[1];
    char k[KL_NAMELENGTH];
    unsigned i;

    if (p == 0) fle_stop("GetKeyboardLayoutNameW", "pwszKLID is NULL: the page gives no behaviour");
    klid(k);
    for (i = 0; i < KL_NAMELENGTH; i++) fle_st(p + 2u * i, 2, (unsigned char)k[i]);
    R[0] = 1;
}

/* user32!GetKeyboardLayoutNameA: the same with an ANSI buffer (Learn: the A variant of the same page; the string is ASCII). */
void fle_imp_GetKeyboardLayoutNameA(void)
{
    uint64_t p = R[1];
    char k[KL_NAMELENGTH];
    unsigned i;

    if (p == 0) fle_stop("GetKeyboardLayoutNameA", "pwszKLID is NULL: the page gives no behaviour");
    klid(k);
    for (i = 0; i < KL_NAMELENGTH; i++) fle_st(p + i, 1, (unsigned char)k[i]);
    R[0] = 1;
}

/* user32!GetKeyboardLayoutList: int f(int nBuff ecx, HKL *lpList rdx). Learn: "the return value is the number of input locale identifiers copied to the buffer or, if nBuff is zero, the return value is the size, in
 * array elements, of the buffer needed to receive all current input locale identifiers"; "If the function fails, the return value is zero". The set of input locales is the one virtual layout of fle_kbd.h: 1 for nBuff
 * 0 (lpList is not touched), otherwise the identifier is copied to the first element and 1 is returned. A negative nBuff, or a NULL list with a positive count, is not described by the page: the call stops. A buffer
 * that can hold the one element is the only case where the page says what is copied; nBuff 1 or more all give 1. */
void fle_imp_GetKeyboardLayoutList(void)
{
    int32_t n = (int32_t)R[1];
    uint64_t list = R[2];

    if (n < 0) fle_stop("GetKeyboardLayoutList", "nBuff is negative: the page gives no behaviour");
    if (n == 0) {
        R[0] = 1;
        return;
    }
    if (list == 0) fle_stop("GetKeyboardLayoutList", "lpList is NULL with nBuff > 0: the page gives no behaviour");
    fle_st(list, 8, fle_kbd_hkl());
    R[0] = 1;
}

static void load_layout(const char *api)
{
    uint64_t p = R[1];
    uint32_t fl = (uint32_t)R[2];
    const uint32_t known = KLF_ACTIVATE | KLF_SUBSTITUTE_OK | KLF_REORDER | KLF_REPLACELANG | KLF_NOTELLSHELL | KLF_SETFORPROCESS;

    if (p == 0) fle_stop(api, "pwszKLID is NULL: the page gives no behaviour");
    if (fl & ~known) fle_stop(api, "Flags has a bit that the page does not list (KLF_UNLOADPREVIOUS is 'unsupported', and KLF_RESET / KLF_SHIFTLOCK belong to ActivateKeyboardLayout)");
    R[0] = fle_kbd_hkl();
}

/* user32!LoadKeyboardLayoutW: HKL f(LPCWSTR pwszKLID rcx, UINT Flags edx). Learn: "Loads a new input locale identifier"; "If the function succeeds, the return value is the input locale identifier corresponding to the
 * name specified in pwszKLID. If no matching locale is available, the return value is the default language of the system. If the function fails, the return value is NULL." The system has one input locale
 * (fle_kbd.h), so the identifier of "00000409" is that locale, and every other name (a well-formed name of a layout that is not loaded, or a string that is no name at all) has "no matching locale", whose answer is
 * "the default language of the system" = the same one (the name is therefore not parsed). A NULL name: the page gives no behaviour, the call stops. The
 * flags only say how the identifier is loaded and activated, and there is nothing to load or reorder or activate (one locale, already active); they must be among those of the page. Wine 10.0 gives the same
 * (tests: tests/sweep1). KLF_REPLACELANG's NULL (a second variant of a language that is already loaded) cannot happen with one locale. */
void fle_imp_LoadKeyboardLayoutW(void)
{
    load_layout("LoadKeyboardLayoutW");
}

/* user32!LoadKeyboardLayoutA: the same page with an ANSI name. */
void fle_imp_LoadKeyboardLayoutA(void)
{
    load_layout("LoadKeyboardLayoutA");
}

/* user32!ActivateKeyboardLayout: HKL f(HKL hkl rcx, UINT Flags edx). Learn: "Sets the input locale identifier for the calling thread or the current process"; "The input locale identifier must have been loaded by a previous
 * call to the LoadKeyboardLayout function. This parameter must be either the handle to a keyboard layout or one of the following values": HKL_NEXT (1) "Selects the next locale identifier in the circular list of
 * loaded locale identifiers", HKL_PREV (0) "Selects the previous"; "If the function succeeds, the return value is the previous input locale identifier. Otherwise, it is zero." With one loaded locale the circular
 * list has one element: the next and the previous are that locale, activating it changes nothing, and the previous identifier is the same. An identifier that is not the loaded one was not loaded by a previous
 * call: 0 with ERROR_INVALID_KEYBOARD_HANDLE (1457, System Error Codes "Invalid keyboard layout handle"; the page names no code: S-sweep1-kbd-1, a CHOICE; Wine 10.0 answers 120 ERROR_CALL_NOT_IMPLEMENTED because
 * its function is a stub, so that is not a reference). Flags: KLF_REORDER, KLF_RESET, KLF_SETFORPROCESS, KLF_SHIFTLOCK (the page); another bit is not described: the call stops. KLF_SETFORPROCESS would send
 * WM_INPUTLANGCHANGE to the focus window "for the entire process": the locale does not change, so the message is not sent (S-sweep1-kbd-2: the message is about a change). */
void fle_imp_ActivateKeyboardLayout(void)
{
    uint64_t h = R[1];
    uint32_t fl = (uint32_t)R[2];
    const uint32_t known = KLF_REORDER | KLF_RESET | KLF_SETFORPROCESS | KLF_SHIFTLOCK;

    if (fl & ~known) fle_stop("ActivateKeyboardLayout", "Flags has a bit that the page does not list");
    if (h == HKL_NEXT || h == HKL_PREV || h == fle_kbd_hkl()) {
        R[0] = fle_kbd_hkl();
        return;
    }
    fle_set_last_error(ERR_INVALID_KEYBOARD_HANDLE);
    R[0] = 0;
}

/* ------------------------------------------------------------------------------------------------------------- the US layout (kbdus.c)
 * The character tables of Microsoft's sample layout kbdus.c (microsoft/Windows-driver-samples, input/layout/kbdus/kbdus.c, MS-PL, the file read for this decision; the same source as fle_wall_ia2.c's key names): aVkToWch3 (3 shift
 * states), aVkToWch4 (4), aVkToWch2 (2), aVkToWch1 (1), in this order (the order matters for VkKeyScan: "Put this last so that VkKeyScan interprets number characters as coming from the main section of the kbd").
 * Columns of a row: the characters for the modification numbers 0 (no modifier), 1 (Shift), 2 (Ctrl), 3 (Shift+Ctrl); CharModifiers of kbdus.c: aVkToBits = Shift -> KBDSHIFT (1), Control -> KBDCTRL (2), Menu -> KBDALT (4),
 * wMaxModBits = 3 with the modification numbers 0, 1, 2, 3 for the bit sets 0..3: a state with the Alt bit has no modification number (SHFT_INVALID): the US layout has no AltGr and no Alt column, so no character.
 * CAPLOK: "CAPS-LOCK affect this key like SHIFT". WCH_NONE: no character. */
#define WCH_NONE 0xF000u
typedef struct { uint8_t vk; uint8_t caplok; uint8_t n; uint16_t wch[4]; } vkw;
static const vkw us_tab[] = {
    /* aVkToWch3 */
    { 0xDB, 0, 3, { '[', '{', 0x1b } }, { 0xDD, 0, 3, { ']', '}', 0x1d } }, { 0xDC, 0, 3, { '\\', '|', 0x1c } }, { 0xE2, 0, 3, { '\\', '|', 0x1c } },
    { 0x08, 0, 3, { '\b', '\b', 0x7f } }, { 0x1B, 0, 3, { 0x1b, 0x1b, 0x1b } }, { 0x0D, 0, 3, { '\r', '\r', '\n' } }, { 0x20, 0, 3, { ' ', ' ', ' ' } }, { 0x03, 0, 3, { 3, 3, 3 } },
    /* aVkToWch4 */
    { '2', 0, 4, { '2', '@', WCH_NONE, 0x00 } }, { '6', 0, 4, { '6', '^', WCH_NONE, 0x1e } }, { 0xBD, 0, 4, { '-', '_', WCH_NONE, 0x1f } },
    /* aVkToWch2 */
    { 0xC0, 0, 2, { '`', '~' } }, { '1', 0, 2, { '1', '!' } }, { '3', 0, 2, { '3', '#' } }, { '4', 0, 2, { '4', '$' } }, { '5', 0, 2, { '5', '%' } }, { '7', 0, 2, { '7', '&' } },
    { '8', 0, 2, { '8', '*' } }, { '9', 0, 2, { '9', '(' } }, { '0', 0, 2, { '0', ')' } }, { 0xBB, 0, 2, { '=', '+' } },
    { 'Q', 1, 2, { 'q', 'Q' } }, { 'W', 1, 2, { 'w', 'W' } }, { 'E', 1, 2, { 'e', 'E' } }, { 'R', 1, 2, { 'r', 'R' } }, { 'T', 1, 2, { 't', 'T' } }, { 'Y', 1, 2, { 'y', 'Y' } },
    { 'U', 1, 2, { 'u', 'U' } }, { 'I', 1, 2, { 'i', 'I' } }, { 'O', 1, 2, { 'o', 'O' } }, { 'P', 1, 2, { 'p', 'P' } }, { 'A', 1, 2, { 'a', 'A' } }, { 'S', 1, 2, { 's', 'S' } },
    { 'D', 1, 2, { 'd', 'D' } }, { 'F', 1, 2, { 'f', 'F' } }, { 'G', 1, 2, { 'g', 'G' } }, { 'H', 1, 2, { 'h', 'H' } }, { 'J', 1, 2, { 'j', 'J' } }, { 'K', 1, 2, { 'k', 'K' } },
    { 'L', 1, 2, { 'l', 'L' } }, { 0xBA, 0, 2, { ';', ':' } }, { 0xDE, 0, 2, { '\'', '"' } }, { 'Z', 1, 2, { 'z', 'Z' } }, { 'X', 1, 2, { 'x', 'X' } }, { 'C', 1, 2, { 'c', 'C' } },
    { 'V', 1, 2, { 'v', 'V' } }, { 'B', 1, 2, { 'b', 'B' } }, { 'N', 1, 2, { 'n', 'N' } }, { 'M', 1, 2, { 'm', 'M' } }, { 0xBC, 0, 2, { ',', '<' } }, { 0xBE, 0, 2, { '.', '>' } },
    { 0xBF, 0, 2, { '/', '?' } }, { 0x6E, 0, 2, { '.', '.' } }, { 0x09, 0, 2, { '\t', '\t' } }, { 0x6B, 0, 2, { '+', '+' } }, { 0x6F, 0, 2, { '/', '/' } }, { 0x6A, 0, 2, { '*', '*' } },
    { 0x6D, 0, 2, { '-', '-' } },
    /* aVkToWch1 */
    { 0x60, 0, 1, { '0' } }, { 0x61, 0, 1, { '1' } }, { 0x62, 0, 1, { '2' } }, { 0x63, 0, 1, { '3' } }, { 0x64, 0, 1, { '4' } }, { 0x65, 0, 1, { '5' } }, { 0x66, 0, 1, { '6' } },
    { 0x67, 0, 1, { '7' } }, { 0x68, 0, 1, { '8' } }, { 0x69, 0, 1, { '9' } },
};
#define US_TAB_N (sizeof us_tab / sizeof us_tab[0])

/* The character of virtual key `vk` with the modifier bits `bits` (1 Shift, 2 Control, 4 Alt) and the Caps Lock toggle `caps`; -1 when there is none (WCH_NONE, no entry, a state the layout does not have). Rules:
 *  - kbdus.c's CharModifiers has wMaxModBits 3 (modification numbers only for the bit sets 0..3 of Shift and Control): Control together with Alt (the AltGr state) has no modification number, and the US layout has no AltGr
 *    column, so it gives no character (S-sweep1-kbd-3, UNOFFICIAL: from the structure of the sample; Wine 10.0 ignores the Alt bit there);
 *  - Alt without Control does not change the character: it is the state of Alt+letter, whose WM_SYSCHAR carries the letter (Learn WM_SYSCHAR: "Posted to the window with the keyboard focus when a WM_SYSKEYDOWN message is
 *    translated by the TranslateMessage function"; the page lists no table) (S-sweep1-kbd-4, UNOFFICIAL: Wine 10.0 agrees);
 *  - a key whose row has fewer shift states than the state asks for has no character there; the one exception is the control characters: a letter key (a CAPLOK row, A..Z) with Control, with or without Shift, gives the
 *    control character vk - 'A' + 1 (Ctrl+A = 0x01), which kbdus.c's two-state letter rows do not list: the system's rule for the control characters; the page of ToUnicode does not state it (S-sweep1-kbd-5, UNOFFICIAL: Wine
 *    10.0 does the same);
 *  - Caps Lock acts like Shift on a CAPLOK row (kbd.h: "CAPS-LOCK affect this key like SHIFT") when no Control is down (S-sweep1-kbd-6, UNOFFICIAL: the sample does not say when the toggle is ignored; with Control the control
 *    character does not depend on the case). */
static int32_t us_char(uint32_t vk, unsigned bits, int caps)
{
    unsigned i, nmod;

    if (vk > 0xFFu) return -1;
    if (bits & 4u) {
        if (bits & 2u) return -1;
        bits &= ~4u;
    }
    nmod = bits & 3u;
    for (i = 0; i < US_TAB_N; i++) {
        const vkw *e = &us_tab[i];

        if (e->vk != vk) continue;
        if (e->caplok && caps && !(nmod & 2u)) nmod ^= 1u;
        if (nmod < e->n) return e->wch[nmod] == WCH_NONE ? -1 : (int32_t)e->wch[nmod];
        if ((nmod & 2u) && e->caplok) return (int32_t)(vk - 'A' + 1u);
        return -1;
    }
    return -1;
}

static unsigned mod_bits(uint64_t ks, int *caps)
{
    unsigned b = 0;

    *caps = 0;
    if (ks == 0) return 0;
    if (fle_ld(ks + 0x10, 1) & 0x80) b |= 1u;
    if (fle_ld(ks + 0x11, 1) & 0x80) b |= 2u;
    if (fle_ld(ks + 0x12, 1) & 0x80) b |= 4u;
    *caps = (fle_ld(ks + 0x14, 1) & 1) != 0;
    return b;
}

/* The input locale of the "Ex" variants: Learn ToUnicodeEx: "dwhkl: The input locale identifier used to translate the specified code. This parameter can be any input locale identifier previously returned by the
 * LoadKeyboardLayout function." The only loaded locale is the virtual one (fle_kbd_hkl()); another value is not described: the call stops. */
static void check_hkl(const char *api, uint64_t hkl)
{
    if (hkl != fle_kbd_hkl()) fle_stop(api, "dwhkl is not the input locale identifier of this environment (the only one loaded, GetKeyboardLayout): the page gives no behaviour");
}

static void to_unicode(const char *api, int has_hkl)
{
    uint32_t vk = (uint32_t)R[1];
    int32_t c, cch = (int32_t)fle_arg(5);
    uint64_t ks = R[8], buf = R[9];
    int caps;
    unsigned bits;

    if (has_hkl) check_hkl(api, fle_arg(7));
    if (buf == 0) fle_stop(api, "pwszBuff is NULL: the page gives no behaviour");
    bits = mod_bits(ks, &caps);
    c = us_char(vk, bits, caps);
    if (c < 0 || cch < 1) {
        R[0] = 0;
        return;
    }
    fle_st(buf, 2, c);
    R[0] = 1;
}

/* user32!ToUnicode: int f(UINT wVirtKey ecx, UINT wScanCode edx, const BYTE *lpKeyState r8, LPWSTR pwszBuff r9, int cchBuff [5], UINT wFlags [6]). Learn: "Translates the specified virtual-key code and keyboard state to the
 * corresponding Unicode character or characters"; 0: "The specified virtual key has no translation for the current state of the keyboard. Nothing was written to the buffer"; > 0: "One or more UTF-16 code units were
 * written"; < 0: a dead key (the US layout has none). lpKeyState: "If the high-order bit of a byte is set, the key is down. The low bit, if set, indicates that the key is toggled on. In this function, only the toggle bit of the
 * CAPS LOCK key is relevant." The modifier bits are those of the generic VK_SHIFT (0x10), VK_CONTROL (0x11) and VK_MENU (0x12) entries (kbdus.c's aVkToBits names those three). lpKeyState is "[in, optional]": NULL is read as
 * "no key down, Caps Lock off" (S-sweep1-kbd-7, a CHOICE: the page does not say what NULL means). The scan code is not used ("Typically, ToUnicode performs the translation based on the virtual-key code"; bit 15, key up,
 * matters only for ALT+numpad entry, which the layout does not have); wFlags (bit 0 menu active: Alt+numpad is not handled; bit 2: the keyboard state is not changed) change nothing, there is no dead key and no
 * kernel-mode keyboard buffer. cchBuff < 1 cannot hold a character: 0 and the buffer is not touched (S-sweep1-kbd-8: the page does not say; Wine 10.0 gives 0). A NULL buffer: the page gives no behaviour, the call stops. */
void fle_imp_ToUnicode(void)
{
    to_unicode("ToUnicode", 0);
}

/* user32!ToUnicodeEx: the same with the input locale identifier dwhkl as the seventh argument (stack). */
void fle_imp_ToUnicodeEx(void)
{
    to_unicode("ToUnicodeEx", 1);
}

static void to_ascii(const char *api, int has_hkl)
{
    uint32_t vk = (uint32_t)R[1];
    int32_t c;
    uint64_t ks = R[8], buf = R[9];
    int caps;
    unsigned bits;

    if (has_hkl) check_hkl(api, fle_arg(6));
    if (buf == 0) fle_stop(api, "lpChar is NULL: the page gives no behaviour");
    bits = mod_bits(ks, &caps);
    c = us_char(vk, bits, caps);
    if (c < 0) {
        R[0] = 0;
        return;
    }
    fle_st(buf, 2, (uint32_t)c & 0xFFu);
    R[0] = 1;
}

/* user32!ToAscii: int f(UINT uVirtKey ecx, UINT uScanCode edx, const BYTE *lpKeyState r8, LPWORD lpChar r9, UINT uFlags [5]). Learn (the same contract as ToUnicode with an ANSI result): 0 no translation, 1 one
 * character copied to the buffer, 2 two characters (a dead key and a character that cannot combine: not on the US layout). The character is in the low-order byte of the WORD; the page does not say what the high-order byte
 * holds when one character is returned: it is written as 0 (S-sweep1-kbd-9, a CHOICE; Wine 10.0 leaves it as it was). Every character of the US layout is below 0x80, the same in every ANSI code page. */
void fle_imp_ToAscii(void)
{
    to_ascii("ToAscii", 0);
}

/* user32!ToAsciiEx: ToAscii with dwhkl as the sixth argument (stack). */
void fle_imp_ToAsciiEx(void)
{
    to_ascii("ToAsciiEx", 1);
}

/* user32!VkKeyScanW: SHORT f(WCHAR ch ecx). Learn: "Translates a character to the corresponding virtual-key code and shift state for the current keyboard"; "the low-order byte contains the virtual-key code and the
 * high-order byte contains the shift state": 1 "Either SHIFT key is pressed", 2 "Either CTRL key is pressed", 4 "Either ALT key is pressed"; "If the function finds no key that translates to the passed character code,
 * both the low-order and high-order bytes contain -1." The search is by modification number first (no modifier, then Shift, Ctrl, Shift+Ctrl), and in the order of the tables above inside one number, so that a character that
 * a key makes without a modifier is that key (Escape for 0x1b, not Ctrl+[) and the digits come from the main block before the numeric pad (the sample's own comment, S-sweep1-kbd-10, UNOFFICIAL: the page does not say
 * which key is meant when two make the character; Wine 10.0 gives the same for the printable characters). A control character that no row lists but a letter key makes with Control (0x01..0x1A: Ctrl+A .. Ctrl+Z) is that
 * key with the Control bit (the rule of us_char). The keys of the numeric pad (0x60..0x6F) answer only when no other key makes the character: '*' and '+' are Shift+8 and Shift+= (the sample lists VK_MULTIPLY and VK_ADD in its
 * two-state table, whose first column would otherwise win) (S-sweep1-kbd-11, UNOFFICIAL: the sample's comment puts the numeric pad last "so that VkKeyScan interprets number characters as coming from the main section";
 * Wine 10.0 gives Shift+8 for '*'). */
static int32_t vk_key_scan(uint32_t ch)
{
    unsigned i, s, pass;

    for (pass = 0; pass < 2; pass++)
        for (s = 0; s < 4; s++)
            for (i = 0; i < US_TAB_N; i++) {
                unsigned pad = us_tab[i].vk >= 0x60 && us_tab[i].vk <= 0x6F;

                if (pad != pass) continue;
                if (s < us_tab[i].n && us_tab[i].wch[s] == ch && ch != WCH_NONE) return (int32_t)((s << 8) | us_tab[i].vk);
            }
    if (ch >= 1 && ch <= 26) return (int32_t)(0x200u | ('A' + ch - 1u));
    return -1;
}

void fle_imp_VkKeyScanW(void)
{
    R[0] = (uint64_t)(uint16_t)vk_key_scan((uint16_t)R[1]);
}

/* user32!VkKeyScanExW: SHORT f(WCHAR ch ecx, HKL dwhkl rdx): the same for the input locale dwhkl (only the virtual one is loaded). */
void fle_imp_VkKeyScanExW(void)
{
    check_hkl("VkKeyScanExW", R[2]);
    R[0] = (uint64_t)(uint16_t)vk_key_scan((uint16_t)R[1]);
}

/* user32!VkKeyScanA: SHORT f(CHAR ch cl): the character is converted from the ANSI code page to Unicode; the US layout holds ASCII only, so a character of 0x80 or above has no key (-1). */
void fle_imp_VkKeyScanA(void)
{
    uint32_t ch = (uint32_t)R[1] & 0xFFu;

    R[0] = (uint64_t)(uint16_t)(ch >= 0x80 ? -1 : vk_key_scan(ch));
}

/* user32!VkKeyScanExA: SHORT f(CHAR ch cl, HKL dwhkl rdx). */
void fle_imp_VkKeyScanExA(void)
{
    uint32_t ch = (uint32_t)R[1] & 0xFFu;

    check_hkl("VkKeyScanExA", R[2]);
    R[0] = (uint64_t)(uint16_t)(ch >= 0x80 ? -1 : vk_key_scan(ch));
}

/* user32!OemKeyScan: DWORD f(WORD wOemChar ecx). Learn: "Maps OEM ASCII codes 0 through 0x0FF into the OEM scan codes and shift states. The function provides information that allows a program to send OEM text to
 * another program by simulating keyboard input"; the low-order word of the return value has the scan code, the high-order word the shift state (1 SHIFT, 2 CTRL); "If the function does not find a key ... the return value is
 * 0xFFFFFFFF". An OEM code below 0x80 is the ASCII character (every OEM code page is ASCII there); the key is VkKeyScan's and the scan code is MapVirtualKey(vk, MAPVK_VK_TO_VSC); an OEM code of 0x80 or above is not on
 * the US keyboard (S-sweep1-kbd-12, a CHOICE: the page gives no rule for them; the characters of the OEM code page 437 above 0x7F are accented letters and box drawing, none has a key on the US layout). */
void fle_imp_OemKeyScan(void)
{
    uint32_t oem = (uint32_t)R[1] & 0xFFFFu, sc;
    int32_t v;

    v = oem < 0x80u ? vk_key_scan(oem) : -1;
    if (v < 0) {
        R[0] = 0xFFFFFFFFu;
        return;
    }
    sc = (uint32_t)fle_api2_call(fle_imp_MapVirtualKeyW, (uint32_t)v & 0xFFu, 0, 0, 0, 0, 0, 0, 0);
    R[0] = (((uint32_t)v >> 8) & 0xFFu) << 16 | (sc & 0xFFFFu);
}

/* user32!MapVirtualKeyExW: UINT f(UINT uCode ecx, UINT uMapType edx, HKL dwhkl r8): MapVirtualKeyW for the input locale dwhkl (Learn: "dwhkl: Input locale identifier used to translate the specified code"; only the virtual one is
 * loaded). */
void fle_imp_MapVirtualKeyExW(void)
{
    check_hkl("MapVirtualKeyExW", R[8]);
    R[0] = fle_api2_call(fle_imp_MapVirtualKeyW, R[1], R[2], 0, 0, 0, 0, 0, 0);
}

/* user32!MapVirtualKeyA: UINT f(UINT uCode ecx, UINT uMapType edx): the same page as MapVirtualKeyW; the characters of MAPVK_VK_TO_CHAR are ASCII on the US layout, the same in the ANSI code page. */
void fle_imp_MapVirtualKeyA(void)
{
    R[0] = fle_api2_call(fle_imp_MapVirtualKeyW, R[1], R[2], 0, 0, 0, 0, 0, 0);
}

/* user32!MapVirtualKeyExA: MapVirtualKeyA with dwhkl as the third argument. */
void fle_imp_MapVirtualKeyExA(void)
{
    check_hkl("MapVirtualKeyExA", R[8]);
    R[0] = fle_api2_call(fle_imp_MapVirtualKeyW, R[1], R[2], 0, 0, 0, 0, 0, 0);
}
