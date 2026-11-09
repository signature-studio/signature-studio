/* decision 0209 (crash1): the menu family on Wine (or Windows) and on the runtime: the same calls, the same printed lines. Build for Wine: x86_64-w64-mingw32-gcc -O1 -x c -o menu_battery.exe menu_battery.inc ;
 * for the runtime: clang -x c -DFLE_RUNTIME -I<runtime> menu_battery.inc fle_crash1_menu.c fle_imports.c fle_env.c -o menu_battery_rt (a stub of fle_win_get is in the file). Handles are printed as h1, h2 ... in the order they were first seen. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdarg.h>
#include <stdlib.h>
typedef struct { uint32_t cbSize, fMask, fType, fState, wID; uint64_t hSubMenu, hbmpChecked, hbmpUnchecked, dwItemData; uint16_t *dwTypeData; uint32_t cch; uint64_t hbmpItem; } MII;
#ifdef FLE_RUNTIME
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_win.h"
#include "fle_crash1_menu.h"
uint64_t R[16];
static uint64_t st[16];
void fle_stop(const char *api, const char *why);
int fle_win_get(uint64_t hwnd, fle_win_view *v) { memset(v, 0, sizeof *v); v->live = hwnd == 0x7f990004ull; return hwnd == 0x7f990004ull || hwnd == 0x7f990008ull; }
#define P(x) ((uint64_t)(uintptr_t)(x))
static void setle(uint32_t v) { fle_set_last_error(v); }
static uint32_t getle(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
#define CALL(fn, a1, a2, a3, a4, a5, a6, a7) (R[1] = (uint64_t)(a1), R[2] = (uint64_t)(a2), R[8] = (uint64_t)(a3), R[9] = (uint64_t)(a4), st[5] = (uint64_t)(a5), st[6] = (uint64_t)(a6), st[7] = (uint64_t)(a7), R[4] = P(st), R[0] = 0xDEAD, fn(), R[0])
static uint64_t X_CreateMenu(void) { return CALL(fle_imp_CreateMenu, 0, 0, 0, 0, 0, 0, 0); }
static uint64_t X_CreatePopupMenu(void) { return CALL(fle_imp_CreatePopupMenu, 0, 0, 0, 0, 0, 0, 0); }
static uint32_t X_DestroyMenu(uint64_t h) { return (uint32_t)CALL(fle_imp_DestroyMenu, h, 0, 0, 0, 0, 0, 0); }
static uint32_t X_InsertMenuW(uint64_t h, uint32_t pos, uint32_t fl, uint64_t id, const void *s) { return (uint32_t)CALL(fle_imp_InsertMenuW, h, pos, fl, id, P(s), 0, 0); }
static uint32_t X_InsertMenuItemW(uint64_t h, uint32_t it, int bypos, const MII *m) { return (uint32_t)CALL(fle_imp_InsertMenuItemW, h, it, bypos, P(m), 0, 0, 0); }
static uint32_t X_SetMenuItemInfoW(uint64_t h, uint32_t it, int bypos, const MII *m) { return (uint32_t)CALL(fle_imp_SetMenuItemInfoW, h, it, bypos, P(m), 0, 0, 0); }
static uint32_t X_GetMenuItemInfoW(uint64_t h, uint32_t it, int bypos, MII *m) { return (uint32_t)CALL(fle_imp_GetMenuItemInfoW, h, it, bypos, P(m), 0, 0, 0); }
static int X_GetMenuItemCount(uint64_t h) { return (int)(uint32_t)CALL(fle_imp_GetMenuItemCount, h, 0, 0, 0, 0, 0, 0); }
static uint32_t X_GetMenuItemID(uint64_t h, int pos) { return (uint32_t)CALL(fle_imp_GetMenuItemID, h, pos, 0, 0, 0, 0, 0); }
static uint32_t X_GetMenuState(uint64_t h, uint32_t id, uint32_t fl) { return (uint32_t)CALL(fle_imp_GetMenuState, h, id, fl, 0, 0, 0, 0); }
static int X_GetMenuStringW(uint64_t h, uint32_t id, uint16_t *b, int n, uint32_t fl) { return (int)(uint32_t)CALL(fle_imp_GetMenuStringW, h, id, b, n, fl, 0, 0); }
static uint64_t X_GetSubMenu(uint64_t h, int pos) { return CALL(fle_imp_GetSubMenu, h, pos, 0, 0, 0, 0, 0); }
static uint32_t X_RemoveMenu(uint64_t h, uint32_t id, uint32_t fl) { return (uint32_t)CALL(fle_imp_RemoveMenu, h, id, fl, 0, 0, 0, 0); }
static uint32_t X_CheckMenuItem(uint64_t h, uint32_t id, uint32_t fl) { return fle_crash1_menu_check_item(h, id, fl); }
static uint32_t X_EnableMenuItem(uint64_t h, uint32_t id, uint32_t fl) { return fle_crash1_menu_enable_item(h, id, fl); }
static uint32_t X_DeleteMenu(uint64_t h, uint32_t id, uint32_t fl) { return fle_crash1_menu_delete_item(h, id, fl); }
#else
#include <windows.h>
static void setle(uint32_t v) { SetLastError(v); }
static uint32_t getle(void) { return GetLastError(); }
static uint64_t X_CreateMenu(void) { return (uint64_t)(uintptr_t)CreateMenu(); }
static uint64_t X_CreatePopupMenu(void) { return (uint64_t)(uintptr_t)CreatePopupMenu(); }
static uint32_t X_DestroyMenu(uint64_t h) { return DestroyMenu((HMENU)(uintptr_t)h); }
static uint32_t X_InsertMenuW(uint64_t h, uint32_t pos, uint32_t fl, uint64_t id, const void *s) { return InsertMenuW((HMENU)(uintptr_t)h, pos, fl, (UINT_PTR)id, (LPCWSTR)s); }
static uint32_t X_InsertMenuItemW(uint64_t h, uint32_t it, int bypos, const MII *m) { return InsertMenuItemW((HMENU)(uintptr_t)h, it, bypos, (const MENUITEMINFOW *)m); }
static uint32_t X_SetMenuItemInfoW(uint64_t h, uint32_t it, int bypos, const MII *m) { return SetMenuItemInfoW((HMENU)(uintptr_t)h, it, bypos, (const MENUITEMINFOW *)m); }
static uint32_t X_GetMenuItemInfoW(uint64_t h, uint32_t it, int bypos, MII *m) { return GetMenuItemInfoW((HMENU)(uintptr_t)h, it, bypos, (MENUITEMINFOW *)m); }
static int X_GetMenuItemCount(uint64_t h) { return GetMenuItemCount((HMENU)(uintptr_t)h); }
static uint32_t X_GetMenuItemID(uint64_t h, int pos) { return GetMenuItemID((HMENU)(uintptr_t)h, pos); }
static uint32_t X_GetMenuState(uint64_t h, uint32_t id, uint32_t fl) { return GetMenuState((HMENU)(uintptr_t)h, id, fl); }
static int X_GetMenuStringW(uint64_t h, uint32_t id, uint16_t *b, int n, uint32_t fl) { return GetMenuStringW((HMENU)(uintptr_t)h, id, b, n, fl); }
static uint64_t X_GetSubMenu(uint64_t h, int pos) { return (uint64_t)(uintptr_t)GetSubMenu((HMENU)(uintptr_t)h, pos); }
static uint32_t X_RemoveMenu(uint64_t h, uint32_t id, uint32_t fl) { return RemoveMenu((HMENU)(uintptr_t)h, id, fl); }
static uint32_t X_CheckMenuItem(uint64_t h, uint32_t id, uint32_t fl) { return CheckMenuItem((HMENU)(uintptr_t)h, id, fl); }
static uint32_t X_EnableMenuItem(uint64_t h, uint32_t id, uint32_t fl) { return EnableMenuItem((HMENU)(uintptr_t)h, id, fl); }
static uint32_t X_DeleteMenu(uint64_t h, uint32_t id, uint32_t fl) { return DeleteMenu((HMENU)(uintptr_t)h, id, fl); }
#endif
#define MF_GRAYED 1
#define MF_DISABLED 2
#define MF_BITMAP 4
#define MF_CHECKED 8
#define MF_POPUP 0x10
#define MF_OWNERDRAW 0x100
#define MF_BYPOSITION 0x400
#define MF_SEPARATOR 0x800
#define MF_STRING 0
#define MIIM_STATE 1
#define MIIM_ID 2
#define MIIM_SUBMENU 4
#define MIIM_CHECKMARKS 8
#define MIIM_TYPE 0x10
#define MIIM_DATA 0x20
#define MIIM_STRING 0x40
#define MIIM_BITMAP 0x80
#define MIIM_FTYPE 0x100
#define M1 (-1)
static uint64_t seen[64]; static int nseen;
static int hn(uint64_t h) { int i; if (!h) return 0; for (i = 0; i < nseen; i++) if (seen[i] == h) return i + 1; if (nseen < 64) seen[nseen++] = h; return nseen; }
static uint16_t *W(const char *s) { size_t n = strlen(s), i; uint16_t *d = calloc(n + 1, 2); for (i = 0; i < n; i++) d[i] = (uint8_t)s[i]; return d; }
static uint32_t pre(void) { setle(0xAAAA); return 0; }
static void L(const char *label, long ret) { uint32_t le = getle(); printf("%s: ret=%ld le=%s", label, ret, le == 0xAAAA ? "-" : "");
    if (le != 0xAAAA) printf("%u", le); printf("\n"); }
static void LH(const char *label, uint64_t h) { uint32_t le = getle(); printf("%s: h%d le=", label, hn(h)); if (le == 0xAAAA) printf("-"); else printf("%u", le); printf("\n"); }
static void mii0(MII *m, uint32_t mask) { memset(m, 0, sizeof *m); m->cbSize = sizeof *m; m->fMask = mask; }
static void dump(const char *label, uint64_t h, uint32_t item, int bypos, uint32_t mask, uint16_t *buf, uint32_t cch)
{
    MII m; uint32_t ok; uint32_t le;
    mii0(&m, mask); m.dwTypeData = buf; m.cch = cch; m.wID = 0x77; m.fType = 0x55; m.fState = 0x66; m.dwItemData = 0x99; m.hSubMenu = 0xdead; m.hbmpItem = 0x88;
    if (buf) buf[0] = 0x5555;
    pre(); ok = X_GetMenuItemInfoW(h, item, bypos, &m); le = getle();
    printf("%s: ok=%u le=", label, ok); if (le == 0xAAAA) printf("-"); else printf("%u", le);
    if (ok) { { int hi = 0, k; for (k = 0; k < nseen; k++) if ((uint32_t)seen[k] == m.wID) hi = k + 1; if (hi) printf(" type=%x state=%x id=h%d sub=h%d", m.fType, m.fState, hi, hn(m.hSubMenu == 0xdead ? 0 : m.hSubMenu)); else printf(" type=%x state=%x id=%x sub=h%d", m.fType, m.fState, m.wID, hn(m.hSubMenu == 0xdead ? 0 : m.hSubMenu)); }  printf(" chk=%d unchk=%d data=%llx cch=%u bmp=%llx", (int)(m.hbmpChecked != 0), (int)(m.hbmpUnchecked != 0), (unsigned long long)m.dwItemData, m.cch, (unsigned long long)m.hbmpItem);
        if (buf && (mask & (MIIM_STRING | MIIM_TYPE))) { uint32_t i; printf(" text='"); for (i = 0; buf[i] && i < 40; i++) putchar(buf[i]); printf("'"); } }
    printf("\n");
}
int main(void)
{
    uint64_t m, p, q, bogus = 0x12345678ull;
    uint16_t buf[64]; MII mi;
    int i;
    setvbuf(stdout, NULL, _IONBF, 0);
    m = X_CreateMenu(); p = X_CreatePopupMenu(); q = X_CreatePopupMenu();
    { int a = hn(m), b = hn(p), c = hn(q); printf("handles: h%d h%d h%d\n", a, b, c); }
    pre(); L("count(m)", X_GetMenuItemCount(m)); pre(); L("count(bogus)", X_GetMenuItemCount(bogus));
    pre(); L("insert File", X_InsertMenuW(m, M1, MF_BYPOSITION | MF_STRING, 100, W("File")));
    pre(); L("insert &Edit popup(p)", X_InsertMenuW(m, M1, MF_BYPOSITION | MF_POPUP | MF_STRING, p, W("&Edit")));
    pre(); L("insert Cut into p", X_InsertMenuW(p, 0, MF_BYPOSITION | MF_STRING, 201, W("Cut")));
    pre(); L("insert Copy checked", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_STRING | MF_CHECKED, 202, W("Copy")));
    pre(); L("insert separator", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_SEPARATOR, 0, NULL));
    pre(); L("insert Paste grayed", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_STRING | MF_GRAYED, 203, W("Paste")));
    pre(); L("insert ownerdraw", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_OWNERDRAW, 204, (const void *)0xBEEF));
    pre(); L("insert bitmap item", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_BITMAP, 205, (const void *)0x55));
    pre(); L("insert NULL string (separator)", X_InsertMenuW(p, M1, MF_BYPOSITION | MF_STRING, 206, NULL));
    pre(); L("insert before id 202 by command", X_InsertMenuW(p, 202, 0 | MF_STRING, 207, W("Before Copy")));
    pre(); L("insert BYPOSITION 99 (append)", X_InsertMenuW(q, 99, MF_BYPOSITION | MF_STRING, 301, W("Q1")));
    pre(); L("insert into bogus", X_InsertMenuW(bogus, 0, MF_BYPOSITION | MF_STRING, 1, W("x")));
    pre(); L("insert popup with bad handle", X_InsertMenuW(q, M1, MF_BYPOSITION | MF_POPUP | MF_STRING, 0x999999, W("BadSub")));
    pre(); L("insert help item \\b", X_InsertMenuW(q, M1, MF_BYPOSITION | MF_STRING, 302, W("\bHelp")));
    pre(); L("count(m)", X_GetMenuItemCount(m)); pre(); L("count(p)", X_GetMenuItemCount(p)); pre(); L("count(q)", X_GetMenuItemCount(q));
    for (i = 0; i <= 11; i++) { char lab[40]; sprintf(lab, "id(p,%d)", i); pre(); L(lab, (long)(int)X_GetMenuItemID(p, i)); }
    pre(); L("id(m,1) popup", (long)(int)X_GetMenuItemID(m, 1)); pre(); L("id(bogus,0)", (long)(int)X_GetMenuItemID(bogus, 0));
    for (i = 0; i <= 3; i++) { char lab[40]; sprintf(lab, "sub(m,%d)", i); pre(); LH(lab, X_GetSubMenu(m, i)); }
    pre(); LH("sub(p,0) not popup", X_GetSubMenu(p, 0)); pre(); LH("sub(bogus,0)", X_GetSubMenu(bogus, 0));
    for (i = 0; i <= 10; i++) { char lab[40]; sprintf(lab, "state(p,pos %d)", i); pre(); L(lab, (long)(int)X_GetMenuState(p, i, MF_BYPOSITION)); }
    pre(); L("state(m,pos 1) popup", (long)(int)X_GetMenuState(m, 1, MF_BYPOSITION));
    pre(); L("state(m, cmd 202) found in submenu", (long)(int)X_GetMenuState(m, 202, 0));
    pre(); L("state(m, cmd 999)", (long)(int)X_GetMenuState(m, 999, 0));
    pre(); L("state(bogus)", (long)(int)X_GetMenuState(bogus, 0, MF_BYPOSITION));
    pre(); L("state(m, cmd 0 = the popup's id)", (long)(int)X_GetMenuState(m, (uint32_t)p, 0));
    { int n[] = { 0, 1, 2, 4, 100 }; int k; for (k = 0; k < 5; k++) { char lab[40]; sprintf(lab, "string(p,0,max %d)", n[k]); memset(buf, 0x55, sizeof buf); pre(); L(lab, X_GetMenuStringW(p, 0, n[k] ? buf : buf, n[k], MF_BYPOSITION)); printf("  buf: %x %x %x %x\n", buf[0], buf[1], buf[2], buf[3]); } }
    pre(); L("string(p,pos 2 separator)", X_GetMenuStringW(p, 3, buf, 64, MF_BYPOSITION));
    pre(); L("string(p,pos 6 ownerdraw)", X_GetMenuStringW(p, 6, buf, 64, MF_BYPOSITION));
    pre(); L("string(p, cmd 203)", X_GetMenuStringW(p, 203, buf, 64, 0)); printf("  text='"); for (i = 0; buf[i] && i < 20; i++) putchar(buf[i]); printf("'\n");
    pre(); L("string(p, cmd 999)", X_GetMenuStringW(p, 999, buf, 64, 0));
    pre(); L("string(NULL buf, cmd 202)", X_GetMenuStringW(p, 202, NULL, 64, 0));
    pre(); L("string(bogus)", X_GetMenuStringW(bogus, 0, buf, 64, MF_BYPOSITION));
    pre(); L("string(m, cmd 201) found in submenu", X_GetMenuStringW(m, 201, buf, 64, 0));
    dump("info(p,0) all", p, 0, 1, MIIM_STATE | MIIM_ID | MIIM_SUBMENU | MIIM_CHECKMARKS | MIIM_DATA | MIIM_FTYPE | MIIM_STRING | MIIM_BITMAP, buf, 64);
    dump("info(m,1) popup", m, 1, 1, MIIM_STATE | MIIM_ID | MIIM_SUBMENU | MIIM_FTYPE | MIIM_STRING, buf, 64);
    dump("info(p,1) checked", p, 1, 1, MIIM_STATE | MIIM_ID | MIIM_FTYPE, NULL, 0);
    dump("info(p,2) separator TYPE", p, 3, 1, MIIM_TYPE, buf, 64);
    dump("info(p,4) ownerdraw TYPE", p, 5, 1, MIIM_TYPE, buf, 64);
    dump("info(p,5) bitmap TYPE", p, 6, 1, MIIM_TYPE, buf, 64);
    dump("info(p,0) STRING length query", p, 0, 1, MIIM_STRING, NULL, 0);
    dump("info(p,0) STRING small buf 2", p, 0, 1, MIIM_STRING, buf, 2);
    dump("info(p,0) STRING buf 4", p, 0, 1, MIIM_STRING, buf, 4);
    dump("info(p,0) STRING buf 1", p, 0, 1, MIIM_STRING, buf, 1);
    dump("info(p,0) TYPE|STRING invalid", p, 0, 1, MIIM_TYPE | MIIM_STRING, buf, 64);
    dump("info(p,0) TYPE|FTYPE invalid", p, 0, 1, MIIM_TYPE | MIIM_FTYPE, buf, 64);
    dump("info(p,99) not found", p, 99, 1, MIIM_ID, NULL, 0);
    dump("info(bogus)", bogus, 0, 1, MIIM_ID, NULL, 0);
    dump("info(m,cmd 203 in submenu)", m, 203, 0, MIIM_ID | MIIM_STATE, NULL, 0);
    mii0(&mi, MIIM_ID); mi.cbSize = 48; pre(); { uint32_t ok = X_GetMenuItemInfoW(p, 0, 1, &mi); printf("info cbSize 48: ok=%u le=%u\n", ok, getle()); }
    mii0(&mi, MIIM_ID); mi.cbSize = 72; pre(); { uint32_t ok = X_GetMenuItemInfoW(p, 0, 1, &mi); printf("info cbSize 72: ok=%u le=%u\n", ok, getle()); }
    /* InsertMenuItemW */
    mii0(&mi, MIIM_ID | MIIM_STRING | MIIM_STATE | MIIM_FTYPE); mi.wID = 401; mi.dwTypeData = W("Via MII"); mi.fState = 8; pre(); L("insertitem by pos 0", X_InsertMenuItemW(q, 0, 1, &mi));
    mii0(&mi, MIIM_TYPE); mi.fType = 0; mi.dwTypeData = W("Old style TYPE"); pre(); L("insertitem MIIM_TYPE string", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_TYPE | MIIM_STRING); pre(); L("insertitem TYPE|STRING invalid", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_FTYPE); mi.fType = MF_BITMAP; pre(); L("insertitem FTYPE bitmap invalid", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_ID); mi.cbSize = 72; mi.wID = 402; pre(); L("insertitem cbSize 72", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_ID); mi.cbSize = 40; pre(); L("insertitem cbSize 40", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_ID | MIIM_SUBMENU | MIIM_STRING | MIIM_FTYPE); mi.wID = 403; mi.hSubMenu = p; mi.dwTypeData = W("Sub p again"); pre(); L("insertitem popup p into q", X_InsertMenuItemW(q, M1, 1, &mi));
    mii0(&mi, MIIM_ID | MIIM_SUBMENU); mi.wID = 404; mi.hSubMenu = 0x777777; pre(); L("insertitem bad submenu", X_InsertMenuItemW(q, M1, 1, &mi));
    pre(); L("count(q)", X_GetMenuItemCount(q));
    mii0(&mi, MIIM_ID); mi.wID = 405; pre(); L("insertitem by command 401 (before it)", X_InsertMenuItemW(q, 401, 0, &mi));
    mii0(&mi, MIIM_ID); mi.wID = 406; pre(); L("insertitem by command 999 (append)", X_InsertMenuItemW(q, 999, 0, &mi));
    for (i = 0; i < 12; i++) { char lab[40]; sprintf(lab, "q pos %d", i); dump(lab, q, i, 1, MIIM_ID | MIIM_STATE | MIIM_FTYPE | MIIM_SUBMENU | MIIM_STRING, buf, 64); }
    /* SetMenuItemInfoW */
    mii0(&mi, MIIM_STATE | MIIM_ID | MIIM_DATA | MIIM_STRING); mi.fState = 3; mi.wID = 501; mi.dwItemData = 0x1234; mi.dwTypeData = W("Renamed Cut"); pre(); L("setinfo p,0", X_SetMenuItemInfoW(p, 0, 1, &mi));
    dump("after set p,0", p, 0, 1, MIIM_STATE | MIIM_ID | MIIM_DATA | MIIM_STRING | MIIM_FTYPE, buf, 64);
    mii0(&mi, MIIM_STRING); mi.dwTypeData = NULL; pre(); L("setinfo p,0 text NULL", X_SetMenuItemInfoW(p, 0, 1, &mi));
    dump("after text NULL (becomes a separator)", p, 0, 1, MIIM_FTYPE | MIIM_STRING, buf, 64);
    mii0(&mi, MIIM_SUBMENU); mi.hSubMenu = q; pre(); L("setinfo p,1 submenu q", X_SetMenuItemInfoW(p, 1, 1, &mi));
    dump("after submenu", p, 1, 1, MIIM_SUBMENU | MIIM_FTYPE, NULL, 0);
    mii0(&mi, MIIM_SUBMENU); mi.hSubMenu = 0; pre(); L("setinfo p,1 submenu NULL", X_SetMenuItemInfoW(p, 1, 1, &mi));
    dump("after submenu NULL", p, 1, 1, MIIM_SUBMENU | MIIM_FTYPE, NULL, 0);
    mii0(&mi, MIIM_SUBMENU); mi.hSubMenu = 0x4242; pre(); L("setinfo bad submenu", X_SetMenuItemInfoW(p, 1, 1, &mi));
    mii0(&mi, MIIM_ID); pre(); L("setinfo not found", X_SetMenuItemInfoW(p, 99, 1, &mi));
    mii0(&mi, MIIM_ID); pre(); L("setinfo bogus menu", X_SetMenuItemInfoW(bogus, 0, 1, &mi));
    mii0(&mi, MIIM_CHECKMARKS); mi.hbmpChecked = 0x31; mi.hbmpUnchecked = 0x32; pre(); L("setinfo checkmarks", X_SetMenuItemInfoW(p, 2, 1, &mi));
    dump("after checkmarks", p, 2, 1, MIIM_CHECKMARKS, NULL, 0);
    mii0(&mi, MIIM_BITMAP); mi.hbmpItem = 0x41; pre(); L("setinfo bitmap", X_SetMenuItemInfoW(p, 2, 1, &mi));
    dump("after bitmap TYPE", p, 2, 1, MIIM_TYPE, buf, 64);
    /* CheckMenuItem / EnableMenuItem / state bits */
    pre(); L("check 202 uncheck", (long)(int)X_CheckMenuItem(m, 202, 0)); pre(); L("state 202", (long)(int)X_GetMenuState(m, 202, 0));
    pre(); L("check 202 check", (long)(int)X_CheckMenuItem(m, 202, MF_CHECKED)); pre(); L("state 202", (long)(int)X_GetMenuState(m, 202, 0));
    pre(); L("enable 203 enable", (long)(int)X_EnableMenuItem(m, 203, 0)); pre(); L("state 203", (long)(int)X_GetMenuState(m, 203, 0));
    pre(); L("enable 203 disabled", (long)(int)X_EnableMenuItem(m, 203, MF_DISABLED)); pre(); L("state 203", (long)(int)X_GetMenuState(m, 203, 0));
    pre(); L("check 999", (long)(int)X_CheckMenuItem(m, 999, 0)); pre(); L("enable bogus", (long)(int)X_EnableMenuItem(bogus, 1, 0));
    /* RemoveMenu / DeleteMenu / DestroyMenu */
    pre(); L("remove cmd 207 found in submenu", X_RemoveMenu(m, 207, 0)); pre(); L("count(p)", X_GetMenuItemCount(p));
    pre(); L("remove pos 99", X_RemoveMenu(p, 99, MF_BYPOSITION)); pre(); L("remove cmd 999", X_RemoveMenu(p, 999, 0)); pre(); L("remove bogus", X_RemoveMenu(bogus, 0, MF_BYPOSITION));
    pre(); L("remove popup item pos 1 of m (submenu stays alive)", X_RemoveMenu(m, 1, MF_BYPOSITION)); pre(); L("count(m)", X_GetMenuItemCount(m)); pre(); L("count(p) still", X_GetMenuItemCount(p));
    pre(); L("insert p back as popup", X_InsertMenuW(m, M1, MF_BYPOSITION | MF_POPUP | MF_STRING, p, W("Edit again")));
    pre(); L("delete popup item pos 1 of m (destroys p)", X_DeleteMenu(m, 1, MF_BYPOSITION)); pre(); L("count(p) after delete", X_GetMenuItemCount(p)); pre(); L("count(m)", X_GetMenuItemCount(m));
    pre(); L("destroy q", X_DestroyMenu(q)); pre(); L("destroy q again", X_DestroyMenu(q)); pre(); L("count(q)", X_GetMenuItemCount(q));
    pre(); L("destroy bogus", X_DestroyMenu(bogus));
    pre(); L("destroy m", X_DestroyMenu(m));
    return 0;
}
