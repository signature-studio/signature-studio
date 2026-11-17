/* The virtual window class table (docs/decisions/0070-virtual-window-classes.md, PROVISIONAL; the pages read and the guest's call
 * sites are in docs/investigations/getclassinfo-caller.md). Single guest thread. A class is {name, owner hInstance, the 72 bytes
 * of the WNDCLASSW, atom}; nothing is ever removed (UnregisterClassW is not answered). A case the Microsoft pages leave open and
 * every argument the guest does not use stops the run through fle_stop (fle_env.h). Nothing here reads the host: no real class,
 * no real window, no module; lpfnWndProc is kept as an address and is never called. The last error is never touched. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_realmod.h"
#include "fle_class.h"
#include "fle_win.h"

#define CS_GLOBALCLASS 0x4000u

typedef struct {
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS];
    uint16_t nlen;
    uint16_t atom;
    uint64_t hinst;
    uint8_t wc[FLE_CLASS_WNDCLASSW_SIZE];
    int removed;                /* decision 0123: UnregisterClassW (the entry stays so that an atom is never reused) */
} cls;

static cls *tab;
static uint64_t tab_n, tab_cap;
static int64_t fle_class_owner_tid;

static void fle_class_check_thread(const char *api)
{
    int64_t tid = (int64_t)fle_host_tid();

    if (fle_class_owner_tid == 0) fle_class_owner_tid = tid;
    if (tid != fle_class_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* The module handles that own classes: the lifted DLL, and (decision 0104) the base of a real module (fle_realmod.h), which is that
 * module's hInstance. */
int fle_class_hinst_known(uint64_t h)
{
    return h == FLE_EXE_HANDLE || (FLE_REALMOD_PRESENT() && fle_realmod_find_handle(h) >= 0);
}

/* lpClassName / lpszClassName as a string: IS_INTRESOURCE values (an atom) and an empty name are not answered; the name is at most
 * FLE_CLASS_NAME_MAX_UNITS units (Learn WNDCLASSW) and ASCII only (the case folding of fle_w_fold is ASCII). */
static uint16_t read_name(const char *api, uint64_t p, uint16_t *out)
{
    uint16_t i;

    if ((p >> 16) == 0) fle_stop(api, "the class name is an atom or NULL, not a string: not answered");
    for (i = 0; i <= FLE_CLASS_NAME_MAX_UNITS; i++) {
        uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);

        if (c == 0) {
            if (i == 0) fle_stop(api, "the class name is empty: not answered");
            return i;
        }
        if (i == FLE_CLASS_NAME_MAX_UNITS) fle_stop(api, "the class name is longer than 256 units (the page says RegisterClass fails, no value for it): not answered");
        if (c > 0x7F) fle_stop(api, "the class name has a unit above 0x7F: not answered");
        out[i] = c;
    }
    return 0;
}

/* Names are compared unit for unit (ci == 0). The pages do not say whether a class name is compared without regard to case, so a
 * name that matches a registered one only without regard to case (ci == 1, ASCII folding of fle_w_icmp) is a form that stops. */
static cls *find(const uint16_t *name, uint16_t nlen, uint64_t hinst, int ci)
{
    uint64_t i;

    for (i = 0; i < tab_n; i++) {
        if (tab[i].removed || tab[i].hinst != hinst) continue;
        if (ci ? fle_w_icmp(tab[i].name, tab[i].nlen, name, nlen) == 0 : (tab[i].nlen == nlen && memcmp(tab[i].name, name, (size_t)nlen * 2u) == 0))
            return &tab[i];
    }
    return 0;
}

/* The class name as ASCII for a stop message (decision 0123: the message names the class that was asked for). */
static void name_for_message(const uint16_t *name, uint16_t nlen, char *out, size_t cap)
{
    size_t i;

    for (i = 0; i < nlen && i + 1 < cap; i++) out[i] = name[i] < 128 ? (char)name[i] : '?';
    out[i] = 0;
}

/* Learn "About Window Classes", System Classes: the classes that the system registers (Button, ComboBox, Edit, ListBox, MDIClient, ScrollBar, Static, ComboLBox, DDEMLEvent, Message, #32768..): GetClassInfoW answers from the
 * system, not from this table. They are not modelled: such a name stops (decision 0215 keeps this; every other unregistered name is "not found"). */
static int is_system_class_name(const uint16_t *name, uint16_t nlen)
{
    /* Learn About Window Classes, System Classes: the classes for all processes and the classes for the system only. A name of this list is a class that the system registered: GetClassInfoW answers from the system. */
    static const char *const sys[] = { "Button", "ComboBox", "Edit", "ListBox", "MDIClient", "ScrollBar", "Static", "ComboLBox", "DDEMLEvent", "Message" };
    size_t k;

    if (nlen >= 1 && name[0] == '#') return 1;     /* #32768 .. #32772: the menu, the desktop, the dialog box, the task switch window, the icon titles */
    for (k = 0; k < sizeof sys / sizeof sys[0]; k++) {
        size_t n = strlen(sys[k]), i;

        if (n != nlen) continue;
        for (i = 0; i < n; i++)
            if (fle_w_fold(name[i]) != fle_w_fold((uint16_t)(uint8_t)sys[k][i])) break;
        if (i == n) return 1;
    }
    return 0;
}

/* Decision 0215 (sweep1, replaces decisions 0123 / 0121's name lists): a class name that is not registered here and is not a system class is "not found". Learn GetClassInfoW: the return value is zero when no class
 * matches ("If the function fails, the return value is zero"); the lookup of CreateWindowEx is the same. The name's SHAPE decides nothing: the earlier code answered "not found" only for a name of the form T + an upper-case
 * letter (a Delphi class name: a reading of FL's naming, "a CHOICE WITHOUT A SOURCE (S-classname)") and for 7 names of the guest, and stopped for every other name. A predefined class (Button, Edit, ...) is a class that the
 * system registered, whose data the system gives: it is not modelled and still stops. A class of another module (RICHEDIT20W ...) is registered by that module when it is loaded; no such module is loaded here, so "not found"
 * is the answer. */
static int is_own_static_name(const uint16_t *name, uint16_t nlen)
{
    return !is_system_class_name(name, nlen);
}

/* user32!RegisterClassW: ATOM f(const WNDCLASSW *lpWndClass rcx). Learn: the return value is the class atom, 0 on failure
 * (GetLastError, no value given). Answered: lpfnWndProc non-NULL, hInstance the lifted DLL (FLE_EXE_HANDLE), no CS_GLOBALCLASS,
 * cbClsExtra and cbWndExtra in 0..40, a string name that is not registered by that hInstance yet. The n-th class gets the atom
 * 0xC000 + n (provisional). Everything else stops. */
void fle_imp_RegisterClassW(void)
{
    static const char api[] = "RegisterClassW";
    uint64_t p = R[1];
    uint8_t wc[FLE_CLASS_WNDCLASSW_SIZE];
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS], nlen;
    uint32_t style;
    int32_t cb_cls, cb_wnd;
    uint64_t proc, hinst, nameptr;
    unsigned i;
    cls *c;

    fle_class_check_thread(api);
    if (p == 0) fle_stop(api, "lpWndClass is NULL: not answered");
    for (i = 0; i < FLE_CLASS_WNDCLASSW_SIZE; i += 8) {
        uint64_t v = fle_ld(p + i, 8);

        memcpy(wc + i, &v, 8);
    }
    memcpy(&style, wc + 0, 4);
    memcpy(&proc, wc + 8, 8);
    memcpy(&cb_cls, wc + 16, 4);
    memcpy(&cb_wnd, wc + 20, 4);
    memcpy(&hinst, wc + 24, 8);
    memcpy(&nameptr, wc + 64, 8);
    if (proc == 0) fle_stop(api, "lpfnWndProc is NULL: the page gives no value for the failure, not answered");
    if (!fle_class_hinst_known(hinst)) fle_stop(api, "hInstance is not the lifted DLL (0x400000): not answered");
    if (style & CS_GLOBALCLASS) fle_stop(api, "CS_GLOBALCLASS: application global classes (lookup from other modules) are not modelled");
    if (cb_cls < 0 || cb_cls > FLE_CLASS_EXTRA_MAX || cb_wnd < 0 || cb_wnd > FLE_CLASS_EXTRA_MAX)
        fle_stop(api, "cbClsExtra or cbWndExtra is outside 0..40: not answered");
    nlen = read_name(api, nameptr, name);
    if (find(name, nlen, hinst, 0)) fle_stop(api, "a class of this name is already registered by this hInstance: the page gives no value for the failure, not answered");
    if (find(name, nlen, hinst, 1)) fle_stop(api, "a class whose name differs only in case is registered: the pages do not say if names are case-sensitive, not answered");
    if (tab_n >= (uint64_t)(FLE_CLASS_ATOM_LAST - FLE_CLASS_ATOM_FIRST + 1u)) fle_stop(api, "the atoms 0xC000..0xFFFF are used up");
    if (tab_n >= tab_cap) {
        uint64_t nc = tab_cap ? tab_cap * 2 : 16;
        cls *n = (cls *)realloc(tab, nc * sizeof *tab);

        if (!n) fle_stop(api, "out of memory");
        tab = n;
        tab_cap = nc;
    }
    c = &tab[tab_n];
    memset(c, 0, sizeof *c);
    memcpy(c->name, name, (size_t)nlen * 2u);
    c->nlen = nlen;
    c->hinst = hinst;
    memcpy(c->wc, wc, sizeof wc);
    c->atom = (uint16_t)(FLE_CLASS_ATOM_FIRST + tab_n);
    tab_n++;
    R[0] = c->atom;
}

/* user32!GetClassInfoW: BOOL f(HINSTANCE hInstance rcx, LPCWSTR lpClassName rdx, LPWNDCLASSW lpWndClass r8). Learn: nonzero if a
 * matching class is found and its data copied; 0 on failure (GetLastError, no value given); hInstance NULL asks for the system
 * classes. Answered: hInstance the lifted DLL (FLE_EXE_HANDLE) and a string name. A class registered by RegisterClassW (this
 * file) with that name and hInstance: the 72 bytes as registered are copied, 1 is returned. A name that is not registered but
 * is one the guest registers itself at a static site: 0, the buffer untouched. Any other name stops. hInstance NULL or any other
 * value stops. */
void fle_imp_GetClassInfoW(void)
{
    static const char api[] = "GetClassInfoW";
    uint64_t hinst = R[1], np = R[2], out = R[8];
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS], nlen;
    unsigned i;
    cls *c;

    fle_class_check_thread(api);
    if (hinst == 0) fle_stop(api, "hInstance is NULL: the system classes are not modelled");
    if (!fle_class_hinst_known(hinst)) fle_stop(api, "hInstance is not the lifted DLL (0x400000): not answered");
    if (out == 0) fle_stop(api, "lpWndClass is NULL: not answered");
    nlen = read_name(api, np, name);
    c = find(name, nlen, hinst, 0);
    if (!c) {
        if (find(name, nlen, hinst, 1)) fle_stop(api, "a class whose name differs only in case is registered: the pages do not say if names are case-sensitive, not answered");
        if (!is_own_static_name(name, nlen)) {
            char nm[300], why[420];

            name_for_message(name, nlen, nm, sizeof nm);
            snprintf(why, sizeof why, "the name \"%s\" is a system class (Button, Edit ...): the system registered it and gives its data; not modelled, not answered", nm);
            fle_stop(api, why);
        }
        R[0] = 0;
        return;
    }
    for (i = 0; i < FLE_CLASS_WNDCLASSW_SIZE; i += 8) {
        uint64_t v;

        memcpy(&v, c->wc + i, 8);
        fle_st(out + i, 8, v);
    }
    R[0] = 1;
}

uint64_t fle_class_count(void)
{
    return tab_n;
}

/* Added by decision 0071 (virtual windows, fle_win.c). The lookup that CreateWindowExW makes; see fle_class.h. It reads the same
 * table as GetClassInfoW and has the same rules for the name; the lpfnWndProc is returned as a number and is not called here. */
int fle_class_lookup(const char *api, uint64_t np, uint64_t hinst, uint16_t *atom, uint64_t *wndproc)
{
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS], nlen;
    cls *c;

    fle_class_check_thread(api);
    if (!fle_class_hinst_known(hinst)) fle_stop(api, "the window's hInstance is not the lifted DLL (0x400000): the class lookup is not answered");
    nlen = read_name(api, np, name);
    c = find(name, nlen, hinst, 0);
    if (!c) {
        if (find(name, nlen, hinst, 1)) fle_stop(api, "the class name differs from a registered one only in case: the pages do not say if names are case-sensitive");
        if (!is_own_static_name(name, nlen)) fle_stop(api, "the class is a system class (Button, Edit ...): the system registered it and gives its data; not modelled, not answered");
        return 0;
    }
    *atom = c->atom;
    memcpy(wndproc, c->wc + 8, 8);
    return 1;
}

/* Added by decision 0119 (wall 15, PROVISIONAL; fle_win.c SetClassLongPtrW): replaces one value of the WNDCLASSW kept for the class `atom`. Learn (SetClassLongPtrW):
 * GCLP_HICON (-14) "Replaces a handle to the icon associated with the class"; "the return value is the previous value of the specified offset"; WNDCLASSW.hIcon is at
 * offset 32 (fle_class.h). No other index is answered (GCLP_HCURSOR, GCLP_HBRBACKGROUND, GCLP_WNDPROC, the extra class memory, ... stop by name: the guest asks only
 * GCLP_HICON, and each of the others has a side that the table does not model). GetClassInfoW then reports the new hIcon (it copies the same 72 bytes). */
void fle_class_set_hicon(const char *api, uint16_t atom, uint64_t value, uint64_t *prev)
{
    uint64_t i;

    fle_class_check_thread(api);
    for (i = 0; i < tab_n; i++) {
        if (tab[i].atom != atom) continue;
        memcpy(prev, tab[i].wc + FLE_CLASS_WC_OFF_HICON, 8);
        memcpy(tab[i].wc + FLE_CLASS_WC_OFF_HICON, &value, 8);
        return;
    }
    fle_stop(api, "the class of the window is not in the class table");
}

/* Decision 0123: see fle_class.h. */
int fle_class_name_of(uint16_t atom, uint16_t *out, uint16_t *len)
{
    uint64_t i;

    for (i = 0; i < tab_n; i++) {
        if (tab[i].atom != atom) continue;
        memcpy(out, tab[i].name, (size_t)tab[i].nlen * 2u);
        *len = tab[i].nlen;
        return 1;
    }
    return 0;
}

/* Decision 0125: see fle_class.h. */
int fle_class_style_of(uint16_t atom, uint32_t *style)
{
    uint64_t i;

    for (i = 0; i < tab_n; i++) {
        if (tab[i].atom != atom) continue;
        memcpy(style, tab[i].wc + 0, 4);
        return 1;
    }
    return 0;
}

int fle_class_hbrbackground_of(uint16_t atom, uint64_t *hbr)
{
    uint64_t i;

    for (i = 0; i < tab_n; i++) {
        if (tab[i].atom != atom) continue;
        memcpy(hbr, tab[i].wc + FLE_CLASS_WC_OFF_HBRBACKGROUND, 8);
        return 1;
    }
    return 0;
}

/* Decision 0136: the hCursor of the WNDCLASSW that registered `atom` (offset 40 of the layout above), 0 when the atom is not in the table or the class has none. No state moves. */
uint64_t fle_class_hcursor_of(uint16_t atom)
{
    uint64_t i, h;

    for (i = 0; i < tab_n; i++) {
        if (tab[i].atom != atom) continue;
        memcpy(&h, tab[i].wc + 40, 8);
        return h;
    }
    return 0;
}

void fle_imp_UnregisterClassW(void)
{
    static const char api[] = "UnregisterClassW";
    uint64_t np = R[1], hinst = R[2];
    uint16_t name[FLE_CLASS_NAME_MAX_UNITS], nlen;
    uint64_t i;
    cls *c = NULL;

    fle_class_check_thread(api);
    if (hinst != 0 && !fle_class_hinst_known(hinst)) fle_stop(api, "hInstance is not the lifted DLL (0x400000) nor NULL: not answered");
    if ((np >> 16) == 0) {
        /* an atom: "it must be a class atom created by a previous call to RegisterClass or RegisterClassEx" */
        for (i = 0; i < tab_n; i++)
            if (!tab[i].removed && tab[i].atom == (uint16_t)np && (hinst == 0 || tab[i].hinst == hinst)) c = &tab[i];
    } else {
        nlen = read_name(api, np, name);
        if (hinst != 0) c = find(name, nlen, hinst, 0);
        else {
            for (i = 0; i < tab_n; i++)
                if (!tab[i].removed && tab[i].nlen == nlen && memcmp(tab[i].name, name, (size_t)nlen * 2u) == 0) c = &tab[i];
        }
    }
    if (!c) {
        fle_set_last_error(FLE_ERROR_CLASS_DOES_NOT_EXIST);
        R[0] = 0;
        return;
    }
    if (fle_win_count_of_atom && fle_win_count_of_atom(c->atom) != 0) {
        fle_set_last_error(FLE_ERROR_CLASS_HAS_WINDOWS);
        R[0] = 0;
        return;
    }
    c->removed = 1;
    R[0] = 1;
}
