/* Decision 0215 (sweep1, opt-in test tool): the cases of the user32 keyboard-layout family (runtime/fle_sweep1_kbd.c) run against Wine 10.0's real APIs (SWEEP1_WINE: mingw build, `wine sweep1_wine.exe`)
 * and against the runtime's fle_imp_* functions (native, lift/harness/sweep1/build.sh). Both print one line per case; the lines must be equal except those listed in expected/known-divergence.txt (with a reason).
 * Nothing printed depends on addresses, times or the host. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef SWEEP1_WINE
#include <windows.h>
#define LASTERR() ((unsigned)GetLastError())
#define SETERR(e) SetLastError(e)
#else
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_api2.h"
#include "fle_proc.h"
#include "fle_kbd.h"
#include "fle_sweep1.h"
uint64_t R[16];
/* fle_input.c (the US layout table of ToUnicode) references the window layer, which these cases never reach: link stubs, not behaviour */
void fle_win_input_activate(void) {}
void fle_win_input_active(void) {}
void fle_win_input_capture_any(void) {}
void fle_win_input_hit(void) {}
void fle_win_input_key_target(void) {}
void fle_win_input_send(void) {}
void fle_win_input_view(void) {}
void fle_win_input_toplevels(void) {}   /* decision 0212 (screen1): fle_input.c apply_display() */
void fle_win_input_remaximize(void) {}
void fle_win_input_showwindow(void) {}
void fle_win_pointer_get(void) {}
void fle_win_pointer_set(void) {}
typedef unsigned int UINT, DWORD, BOOL;
typedef int INT;
typedef uint16_t WCHAR, WORD;
typedef uint8_t BYTE;
typedef uint64_t HKL_T;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define I(x) ((uint64_t)(int64_t)(x))
#define C(f, ...) fle_api2_call(fle_imp_##f, __VA_ARGS__)
static unsigned LASTERR(void) { return (unsigned)C(GetLastError, 0, 0, 0, 0, 0, 0, 0, 0); }
static void SETERR(unsigned e) { (void)C(SetLastError, e, 0, 0, 0, 0, 0, 0, 0); }
#endif

#ifdef SWEEP1_WINE
typedef HKL HKL_T;
#define KBD_GetKeyboardLayoutNameW(b) GetKeyboardLayoutNameW(b)
#define KBD_GetKeyboardLayoutNameA(b) GetKeyboardLayoutNameA(b)
#define KBD_GetKeyboardLayoutList(n, l) GetKeyboardLayoutList(n, l)
#define KBD_LoadKeyboardLayoutW(s, f) ((uint64_t)(uintptr_t)LoadKeyboardLayoutW(s, f))
#define KBD_ActivateKeyboardLayout(h, f) ((uint64_t)(uintptr_t)ActivateKeyboardLayout((HKL)(uintptr_t)(h), f))
#define KBD_GetKeyboardLayout() ((uint64_t)(uintptr_t)GetKeyboardLayout(0))
#define KBD_ToUnicode(vk, sc, ks, b, n, f) ToUnicode(vk, sc, ks, b, n, f)
#define KBD_ToAscii(vk, sc, ks, b, f) ToAscii(vk, sc, ks, b, f)
#define KBD_ToUnicodeEx(vk, sc, ks, b, n, f, h) ToUnicodeEx(vk, sc, ks, b, n, f, (HKL)(uintptr_t)(h))
#define KBD_ToAsciiEx(vk, sc, ks, b, f, h) ToAsciiEx(vk, sc, ks, b, f, (HKL)(uintptr_t)(h))
#define KBD_VkKeyScanW(c) ((int)(short)VkKeyScanW(c))
#define KBD_VkKeyScanExW(c, h) ((int)(short)VkKeyScanExW(c, (HKL)(uintptr_t)(h)))
#define KBD_VkKeyScanA(c) ((int)(short)VkKeyScanA(c))
#define KBD_OemKeyScan(c) ((unsigned)OemKeyScan(c))
#define KBD_MapVirtualKeyExW(c, t, h) ((unsigned)MapVirtualKeyExW(c, t, (HKL)(uintptr_t)(h)))
#define KBD_MapVirtualKeyA(c, t) ((unsigned)MapVirtualKeyA(c, t))
#define KBD_MapVirtualKeyW(c, t) ((unsigned)MapVirtualKeyW(c, t))
#else
#define KBD_GetKeyboardLayoutNameW(b) ((BOOL)C(GetKeyboardLayoutNameW, A(b), 0, 0, 0, 0, 0, 0, 0))
#define KBD_GetKeyboardLayoutNameA(b) ((BOOL)C(GetKeyboardLayoutNameA, A(b), 0, 0, 0, 0, 0, 0, 0))
#define KBD_GetKeyboardLayoutList(n, l) ((int)C(GetKeyboardLayoutList, I(n), A(l), 0, 0, 0, 0, 0, 0))
#define KBD_LoadKeyboardLayoutW(s, f) C(LoadKeyboardLayoutW, A(s), f, 0, 0, 0, 0, 0, 0)
#define KBD_ActivateKeyboardLayout(h, f) C(ActivateKeyboardLayout, h, f, 0, 0, 0, 0, 0, 0)
#define KBD_GetKeyboardLayout() C(GetKeyboardLayout, 0, 0, 0, 0, 0, 0, 0, 0)
#define KBD_ToUnicode(vk, sc, ks, b, n, f) ((int)C(ToUnicode, vk, sc, A(ks), A(b), I(n), f, 0, 0))
#define KBD_ToAscii(vk, sc, ks, b, f) ((int)C(ToAscii, vk, sc, A(ks), A(b), f, 0, 0, 0))
#define KBD_ToUnicodeEx(vk, sc, ks, b, n, f, h) ((int)C(ToUnicodeEx, vk, sc, A(ks), A(b), I(n), f, h, 0))
#define KBD_ToAsciiEx(vk, sc, ks, b, f, h) ((int)C(ToAsciiEx, vk, sc, A(ks), A(b), f, h, 0, 0))
#define KBD_VkKeyScanW(c) ((int)(short)C(VkKeyScanW, c, 0, 0, 0, 0, 0, 0, 0))
#define KBD_VkKeyScanExW(c, h) ((int)(short)C(VkKeyScanExW, c, h, 0, 0, 0, 0, 0, 0))
#define KBD_VkKeyScanA(c) ((int)(short)C(VkKeyScanA, (unsigned char)(c), 0, 0, 0, 0, 0, 0, 0))
#define KBD_OemKeyScan(c) ((unsigned)C(OemKeyScan, c, 0, 0, 0, 0, 0, 0, 0))
#define KBD_MapVirtualKeyExW(c, t, h) ((unsigned)C(MapVirtualKeyExW, c, t, h, 0, 0, 0, 0, 0))
#define KBD_MapVirtualKeyA(c, t) ((unsigned)C(MapVirtualKeyA, c, t, 0, 0, 0, 0, 0, 0))
#define KBD_MapVirtualKeyW(c, t) ((unsigned)C(MapVirtualKeyW, c, t, 0, 0, 0, 0, 0, 0))
#endif

static void wstr(const WCHAR *w, int max)
{
    int i;
    printf("\"");
    for (i = 0; i < max && w[i]; i++) printf("%c", w[i] >= 32 && w[i] < 127 ? (char)w[i] : '?');
    printf("\"");
}

static void name_w(const char *label, int fill_len)
{
    WCHAR b[24];
    BOOL r;
    int i;
    for (i = 0; i < 24; i++) b[i] = 0xABAB;
    SETERR(0);
    r = KBD_GetKeyboardLayoutNameW(b);
    printf("GetKeyboardLayoutNameW %s = %d", label, (int)r);
    if (r) { printf(" "); wstr(b, 9); printf(" terminator@%d next=%04x", (int)(b[8] == 0 ? 8 : -1), (unsigned)b[9]); }
    else printf(" err=%u", LASTERR());
    printf("\n");
    (void)fill_len;
}

static void name_a(void)
{
    char b[24];
    BOOL r;
    memset(b, 'x', sizeof b);
    SETERR(0);
    r = KBD_GetKeyboardLayoutNameA(b);
    printf("GetKeyboardLayoutNameA = %d", (int)r);
    if (r) printf(" \"%.8s\" b8=%02x b9=%02x", b, (unsigned char)b[8], (unsigned char)b[9]);
    else printf(" err=%u", LASTERR());
    printf("\n");
}

static void list(const char *label, int n, int want_buf)
{
    HKL_T l[4];
    int r, i;
    for (i = 0; i < 4; i++) l[i] = 0x1111;
    SETERR(0);
    r = KBD_GetKeyboardLayoutList(n, want_buf ? l : NULL);
    printf("GetKeyboardLayoutList %s = %d", label, r);
    if (r == 0) printf(" err=%u", LASTERR());
    if (r > 0 && want_buf && n > 0) {
        unsigned long long cur = (unsigned long long)KBD_GetKeyboardLayout();
        printf(" first==GetKeyboardLayout:%d second-untouched:%d", (unsigned long long)l[0] == cur, n > 1 ? (unsigned long long)l[1] == 0x1111 : 1);
    }
    printf("\n");
}

static void loadl(const char *label, const WCHAR *s, unsigned f)
{
    uint64_t r;
    SETERR(0);
    r = KBD_LoadKeyboardLayoutW(s, f);
    printf("LoadKeyboardLayoutW %s = %s", label, r == 0 ? "NULL" : ((unsigned long long)r == (unsigned long long)KBD_GetKeyboardLayout() ? "the current layout" : "another layout"));
    if (r == 0) printf(" err=%u", LASTERR());
    printf("\n");
}

static void act(const char *label, uint64_t h, unsigned f)
{
    uint64_t r;
    SETERR(0);
    r = KBD_ActivateKeyboardLayout(h, f);
    printf("ActivateKeyboardLayout %s = %s", label, r == 0 ? "NULL" : ((unsigned long long)r == (unsigned long long)KBD_GetKeyboardLayout() ? "the previous layout (= the current one)" : "another"));
    if (r == 0) printf(" err=%u", LASTERR());
    printf(" after: same=%d\n", (unsigned long long)KBD_GetKeyboardLayout() == (unsigned long long)KBD_GetKeyboardLayout());
}

static void tou(const char *label, unsigned vk, unsigned sc, int shift, int ctrl, int caps, int cch)
{
    BYTE ks[256];
    WCHAR b[8];
    int r, i;
    memset(ks, 0, sizeof ks);
    if (shift) ks[0x10] = 0x80;
    if (ctrl) ks[0x11] = 0x80;
    if (caps) ks[0x14] = 1;
    for (i = 0; i < 8; i++) b[i] = 0xABAB;
    SETERR(0);
    r = KBD_ToUnicode(vk, sc, ks, b, cch, 0);
    printf("ToUnicode %s = %d", label, r);
    if (r > 0) { printf(" ["); for (i = 0; i < r && i < 8; i++) printf("%04x%s", (unsigned)b[i], i + 1 < r ? " " : ""); printf("]"); }
    printf(" b0=%04x\n", (unsigned)b[0]);
}

static void toa(const char *label, unsigned vk, unsigned sc, int shift, int ctrl, int caps)
{
    BYTE ks[256];
    WORD w[2];
    int r;
    memset(ks, 0, sizeof ks);
    if (shift) ks[0x10] = 0x80;
    if (ctrl) ks[0x11] = 0x80;
    if (caps) ks[0x14] = 1;
    w[0] = w[1] = 0xABAB;
    SETERR(0);
    r = KBD_ToAscii(vk, sc, ks, w, 0);
    printf("ToAscii %s = %d", label, r);
    if (r > 0) printf(" [%02x]", (unsigned)(w[0] & 0xFF));
    printf("\n");
}

/* every virtual-key code 0..0xFF against 10 keyboard states (none, Shift, Ctrl, Shift+Ctrl, Alt, Ctrl+Alt, Caps, Caps+Shift, Caps+Ctrl, Shift+Alt): one line per code, the character (hex) or '-' for no translation, and the result count */
static void state_ks(BYTE *ks, int st)
{
    memset(ks, 0, 256);
    if (st == 1 || st == 3 || st == 7 || st == 9) ks[0x10] = 0x80;
    if (st == 2 || st == 3 || st == 5 || st == 8) ks[0x11] = 0x80;
    if (st == 4 || st == 5 || st == 9) ks[0x12] = 0x80;
    if (st >= 6 && st <= 8) ks[0x14] = 1;
}

static void matrix_unicode(void)
{
    unsigned vk;
    int st;
    BYTE ks[256];
    WCHAR b[4];
    for (vk = 0; vk < 256; vk++) {
        printf("ToUnicode matrix vk=%02x:", vk);
        for (st = 0; st < 10; st++) {
            int r;
            if (st == 5) continue;   /* Ctrl+Alt: its own lines below (the runtime differs from Wine there on purpose) */
            state_ks(ks, st);
            b[0] = b[1] = b[2] = b[3] = 0xABAB;
            r = KBD_ToUnicode(vk, 0, ks, b, 4, 0);
            if (r > 0) printf(" %d:%04x", r, (unsigned)b[0]); else printf(" %d:-", r);
        }
        printf("\n");
    }
}

static void matrix_ascii(void)
{
    unsigned vk;
    int st;
    BYTE ks[256];
    WORD w[2];
    for (vk = 0; vk < 256; vk++) {
        printf("ToAscii matrix vk=%02x:", vk);
        for (st = 0; st < 10; st++) {
            int r;
            if (st == 5) continue;
            state_ks(ks, st);
            w[0] = w[1] = 0xABAB;
            r = KBD_ToAscii(vk, 0, ks, w, 0);
            if (r > 0) printf(" %d:%02x", r, (unsigned)(w[0] & 0xFF)); else printf(" %d:-", r);
        }
        printf("\n");
    }
}

static void matrix_ctrlalt(void)
{
    unsigned vk;
    BYTE ks[256];
    WCHAR b[4];
    WORD w[2];
    for (vk = 0; vk < 256; vk++) {
        int r, r2;
        state_ks(ks, 5);
        b[0] = 0xABAB; w[0] = 0xABAB;
        r = KBD_ToUnicode(vk, 0, ks, b, 4, 0);
        r2 = KBD_ToAscii(vk, 0, ks, w, 0);
        printf("ToUnicode ctrlalt vk=%02x: %d:%04x  ToAscii %d:%02x\n", vk, r, r > 0 ? (unsigned)b[0] : 0u, r2, r2 > 0 ? (unsigned)(w[0] & 0xFF) : 0u);
    }
}

static void vkscan(void)
{
    unsigned c;
    static const unsigned extra[] = { 0x80, 0xA0, 0xE9, 0xFF, 0x100, 0x20AC, 0xFFFF };
    unsigned i;
    for (c = 0; c < 0x80; c++) {
        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) continue;
        printf("VkKeyScanW %04x = %04x\n", c, (unsigned)KBD_VkKeyScanW(c) & 0xFFFF);
    }
    {   /* the letters in two lines: the shift state of 'a' and 'A' is the one that Wine's session shows with Caps Lock on, so these two lines are listed as divergent */
        char up[27 * 5], lo[27 * 5];
        up[0] = lo[0] = 0;
        for (c = 0; c < 26; c++) {
            sprintf(up + strlen(up), " %04x", (unsigned)KBD_VkKeyScanW('A' + c) & 0xFFFF);
            sprintf(lo + strlen(lo), " %04x", (unsigned)KBD_VkKeyScanW('a' + c) & 0xFFFF);
        }
        printf("VkKeyScanW letters A-Z =%s\n", up);
        printf("VkKeyScanW letters a-z =%s\n", lo);
    }
    for (i = 0; i < sizeof extra / sizeof extra[0]; i++) printf("VkKeyScanW %04x = %04x\n", extra[i], (unsigned)KBD_VkKeyScanW(extra[i]) & 0xFFFF);
    for (c = 0x20; c < 0x7F; c += 7) printf("VkKeyScanA %02x = %04x\n", c, (unsigned)KBD_VkKeyScanA(c) & 0xFFFF);
    printf("VkKeyScanA e9 = %04x\n", (unsigned)KBD_VkKeyScanA(0xE9) & 0xFFFF);
    printf("VkKeyScanExW 61 current = %04x\n", (unsigned)KBD_VkKeyScanExW(0x61, (unsigned long long)KBD_GetKeyboardLayout()) & 0xFFFF);
    printf("VkKeyScanExW 41 current = %04x\n", (unsigned)KBD_VkKeyScanExW(0x41, (unsigned long long)KBD_GetKeyboardLayout()) & 0xFFFF);
}

static void oemscan(void)
{
    unsigned c;
    for (c = 0; c < 256; c++) printf("OemKeyScan %02x = %08x\n", c, KBD_OemKeyScan(c));
}

static void mapvk(void)
{
    unsigned c, t;
    unsigned long long h = (unsigned long long)KBD_GetKeyboardLayout();
    static const unsigned codes[] = { 0x08, 0x09, 0x0D, 0x10, 0x1B, 0x20, 0x25, 0x30, 0x41, 0x5A, 0x60, 0x70, 0x7B, 0xBA, 0xBC, 0xC0, 0xDB, 0xDE };
    for (c = 0; c < sizeof codes / sizeof codes[0]; c++)
        for (t = 0; t < 3; t++)
            printf("MapVirtualKeyExW %02x,%u = %x same-as-W:%d same-as-A:%d\n", codes[c], t, KBD_MapVirtualKeyExW(codes[c], t, h), KBD_MapVirtualKeyExW(codes[c], t, h) == KBD_MapVirtualKeyW(codes[c], t), KBD_MapVirtualKeyA(codes[c], t) == KBD_MapVirtualKeyW(codes[c], t));
}

static void exvariants(void)
{
    BYTE ks[256];
    WCHAR b[4];
    WORD w[2];
    unsigned long long h = (unsigned long long)KBD_GetKeyboardLayout();
    int r;
    memset(ks, 0, sizeof ks);
    ks[0x10] = 0x80;
    r = KBD_ToUnicodeEx('A', 0x1E, ks, b, 4, 0, h);
    printf("ToUnicodeEx A shift current = %d [%04x]\n", r, (unsigned)b[0]);
    r = KBD_ToAsciiEx('B', 0x30, ks, w, 0, h);
    printf("ToAsciiEx B shift current = %d [%02x]\n", r, (unsigned)(w[0] & 0xFF));
}

static const WCHAR K0409[] = { '0', '0', '0', '0', '0', '4', '0', '9', 0 };
static const WCHAR K0411[] = { '0', '0', '0', '0', '0', '4', '1', '1', 0 };
static const WCHAR KBAD[] = { 'z', 'z', 'z', 0 };
static const WCHAR KEMPTY[] = { 0 };

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
#ifdef SWEEP1_WINE
    /* Wine's session may start with Caps Lock toggled on (VkKeyScan and OemKeyScan read the toggle); switch it off so that the reference is the plain state */
    fprintf(stderr, "caps toggle at start: %d\n", GetKeyState(VK_CAPITAL) & 1);
    if (GetKeyState(VK_CAPITAL) & 1) {
        keybd_event(VK_CAPITAL, 0x3a, 0, 0);
        keybd_event(VK_CAPITAL, 0x3a, KEYEVENTF_KEYUP, 0);
        fprintf(stderr, "caps toggle after: %d\n", GetKeyState(VK_CAPITAL) & 1);
    }
#endif
    name_w("", 0);
    name_a();
    list("n=0 (size query)", 0, 0);
    list("n=1", 1, 1);
    list("n=2", 2, 1);
    loadl("\"00000409\" flags 0", K0409, 0);
    loadl("\"00000409\" KLF_ACTIVATE", K0409, 1);
    loadl("\"00000409\" KLF_ACTIVATE|KLF_SETFORPROCESS", K0409, 0x101);
    loadl("\"00000409\" KLF_NOTELLSHELL", K0409, 0x80);
    loadl("\"00000411\" flags 0", K0411, 0);
    loadl("\"zzz\" flags 0", KBAD, 0);
    loadl("\"\" flags 0", KEMPTY, 0);
    act("current HKL flags 0", KBD_GetKeyboardLayout(), 0);
    act("current HKL KLF_SETFORPROCESS", KBD_GetKeyboardLayout(), 0x100);
    act("HKL_NEXT", 1, 0);
    act("HKL_PREV", 0, 0);
    act("0x04110411", 0x04110411u, 0);
    tou("A", 'A', 0x1E, 0, 0, 0, 4);
    tou("A shift", 'A', 0x1E, 1, 0, 0, 4);
    tou("A caps", 'A', 0x1E, 0, 0, 1, 4);
    tou("A ctrl", 'A', 0x1E, 0, 1, 0, 4);
    tou("1", '1', 0x02, 0, 0, 0, 4);
    tou("1 shift", '1', 0x02, 1, 0, 0, 4);
    tou("space", 0x20, 0x39, 0, 0, 0, 4);
    tou("return", 0x0D, 0x1C, 0, 0, 0, 4);
    tou("F1", 0x70, 0x3B, 0, 0, 0, 4);
    tou("shift key", 0x10, 0x2A, 0, 0, 0, 4);
    tou("A cch=1", 'A', 0x1E, 0, 0, 0, 1);
    tou("A cch=0", 'A', 0x1E, 0, 0, 0, 0);
    tou("numpad1", 0x61, 0x4F, 0, 0, 0, 4);
    tou("numpad1 shift", 0x61, 0x4F, 1, 0, 0, 4);
    tou("numpad9", 0x69, 0x49, 0, 0, 0, 4);
    tou("multiply", 0x6A, 0x37, 0, 0, 0, 4);
    tou("add", 0x6B, 0x4E, 0, 0, 0, 4);
    tou("separator", 0x6C, 0x00, 0, 0, 0, 4);
    tou("subtract", 0x6D, 0x4A, 0, 0, 0, 4);
    tou("decimal", 0x6E, 0x53, 0, 0, 0, 4);
    tou("divide", 0x6F, 0x35, 0, 0, 0, 4);
    tou("oem ;", 0xBA, 0x27, 0, 0, 0, 4);
    tou("oem ; shift", 0xBA, 0x27, 1, 0, 0, 4);
    tou("escape", 0x1B, 0x01, 0, 0, 0, 4);
    tou("tab", 0x09, 0x0F, 0, 0, 0, 4);
    tou("backspace", 0x08, 0x0E, 0, 0, 0, 4);
    tou("Z ctrl+shift", 'Z', 0x2C, 1, 1, 0, 4);
    toa("numpad5", 0x65, 0x4C, 0, 0, 0);
    toa("A", 'A', 0x1E, 0, 0, 0);
    exvariants();
    matrix_unicode();
    matrix_ascii();
    matrix_ctrlalt();
    vkscan();
    oemscan();
    mapvk();
    toa("A shift", 'A', 0x1E, 1, 0, 0);
    toa("2 shift", '2', 0x03, 1, 0, 0);
    toa("F1", 0x70, 0x3B, 0, 0, 0);
    return 0;
}
