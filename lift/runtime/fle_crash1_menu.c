/* Decision 0209 (crash1): user32's menu family. See fle_crash1_menu.h for the pages and for what is Wine's reading. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_crash1_menu.h"

#define MF_GRAYED 0x1u
#define MF_DISABLED 0x2u
#define MF_BITMAP 0x4u
#define MF_CHECKED 0x8u
#define MF_POPUP 0x10u
#define MF_OWNERDRAW 0x100u
#define MF_BYPOSITION 0x400u
#define MF_SEPARATOR 0x800u
#define MF_DEFAULT 0x1000u
#define MF_HELP 0x4000u
#define MF_MOUSESELECT 0x8000u
#define MIIM_STATE 0x1u
#define MIIM_ID 0x2u
#define MIIM_SUBMENU 0x4u
#define MIIM_CHECKMARKS 0x8u
#define MIIM_TYPE 0x10u
#define MIIM_DATA 0x20u
#define MIIM_STRING 0x40u
#define MIIM_BITMAP 0x80u
#define MIIM_FTYPE 0x100u
#define TYPE_MII_MASK (0x4u | 0x100u | 0x800u | 0x20u | 0x40u | 0x200u | 0x2000u | 0x4000u)   /* MFT_BITMAP OWNERDRAW SEPARATOR MENUBARBREAK MENUBREAK RADIOCHECK RIGHTORDER RIGHTJUSTIFY (MFT_STRING is 0) */
#define TYPE_MASK (TYPE_MII_MASK | MF_POPUP | 0x2000u)
#define STATE_MASK (~TYPE_MASK)
#define STATE_MII_MASK (STATE_MASK & ~(MF_BYPOSITION | MF_MOUSESELECT))
#define IS_STRING_ITEM(f) (((f) & (0x4u | 0x100u | 0x800u)) == 0)
#define MENU_DEPTH_MAX 30u   /* MAXMENUDEPTH of win32u/menu.c */
#define MENU_TEXT_MAX 4096u
#define MII_SIZE 80u       /* MENUITEMINFOW on x64 */
#define MII_SIZE_OLD 72u   /* without hbmpItem */

typedef struct { uint32_t type, state, id; uint64_t sub, chk, unchk, data, bmp; uint16_t *text; } fle_mitem;
typedef struct { int live; uint32_t flags; fle_mitem *items; uint32_t n; } fle_menu;
/* MENUITEMINFOW in local form */
typedef struct { uint32_t cbSize, mask, type, state, id; uint64_t sub, chk, unchk, data, typedata; uint32_t cch; uint64_t bmp; } fle_mii;

static fle_menu **mt;
static uint32_t mn, mcap;

static fle_menu *get_menu(uint64_t h, int seterr)
{
    uint64_t k;

    if (h > FLE_CMENU_HANDLE_BASE && ((h - FLE_CMENU_HANDLE_BASE) & 3u) == 0) {
        k = (h - FLE_CMENU_HANDLE_BASE) / 4u - 1u;
        if (k < mn && mt[k]->live) return mt[k];
    }
    if (seterr) fle_set_last_error(FLE_ERROR_INVALID_MENU_HANDLE);
    return NULL;
}

int fle_crash1_menu_owns(uint64_t h) { return get_menu(h, 0) != NULL; }

static uint64_t new_menu(uint32_t flags)
{
    fle_menu *m;

    if (mn >= FLE_CMENU_MAX) fle_stop("CreateMenu", "more than 100000 menus were created (the handle region is full)");
    if (mn == mcap) {
        uint32_t nc = mcap ? mcap * 2u : 64u;
        fle_menu **p = realloc(mt, nc * sizeof *p);
        if (!p) fle_stop("CreateMenu", "out of memory");
        mt = p;
        mcap = nc;
    }
    m = calloc(1, sizeof *m);
    if (!m) fle_stop("CreateMenu", "out of memory");
    m->live = 1;
    m->flags = flags;
    mt[mn++] = m;
    return FLE_CMENU_HANDLE_BASE + 4ull * mn;
}

/* ---- finding an item (win32u/menu.c find_menu_item): by position inside this menu, or by command id in this menu and, recursively, in the submenus of its popup items; a popup item whose own id matches is the fallback */
static int find_rec(fle_menu *m, uint32_t id, uint32_t flags, fle_menu **pm, uint32_t *ppos, unsigned depth)
{
    uint32_t i, fallback = 0xFFFFFFFFu;

    if (flags & MF_BYPOSITION) {
        if (id >= m->n) return 0;
        *pm = m;
        *ppos = id;
        return 1;
    }
    for (i = 0; i < m->n; i++) {
        fle_mitem *it = &m->items[i];
        if (it->type & MF_POPUP) {
            fle_menu *sub = depth < MENU_DEPTH_MAX ? get_menu(it->sub, 0) : NULL;
            if (sub && find_rec(sub, id, flags, pm, ppos, depth + 1u)) return 1;
            if (it->id == id) fallback = i;
        } else if (it->id == id) {
            *pm = m;
            *ppos = i;
            return 1;
        }
    }
    if (fallback != 0xFFFFFFFFu) {
        *pm = m;
        *ppos = fallback;
        return 1;
    }
    return 0;
}

static int find_item(uint64_t h, uint32_t id, uint32_t flags, fle_menu **pm, uint32_t *ppos)
{
    fle_menu *m = get_menu(h, 0);

    return m != NULL && find_rec(m, id, flags, pm, ppos, 0);
}

static void free_item_text(fle_mitem *it)
{
    free(it->text);
    it->text = NULL;
}

static void remove_at(fle_menu *m, uint32_t pos)
{
    free_item_text(&m->items[pos]);
    memmove(&m->items[pos], &m->items[pos + 1], (m->n - pos - 1u) * sizeof *m->items);
    m->n--;
}

static void destroy_menu(fle_menu *m, unsigned depth)
{
    uint32_t i;

    if (!m->live) return;
    m->live = 0;   /* the handle is invalid from now on, which also ends a loop in the hierarchy */
    for (i = 0; i < m->n; i++) {
        if ((m->items[i].type & MF_POPUP) && depth < MENU_DEPTH_MAX) {
            fle_menu *sub = get_menu(m->items[i].sub, 0);
            if (sub) destroy_menu(sub, depth + 1u);
        }
        free_item_text(&m->items[i]);
    }
    free(m->items);
    m->items = NULL;
    m->n = 0;
}

static unsigned menu_depth(fle_menu *m, unsigned depth)
{
    uint32_t i;
    unsigned best = depth;

    if (depth > MENU_DEPTH_MAX) return depth;
    for (i = 0; i < m->n && best <= MENU_DEPTH_MAX; i++) {   /* any item with a submenu handle counts, the POPUP type bit is not looked at (Wine) */
        fle_menu *sub = m->items[i].sub ? get_menu(m->items[i].sub, 0) : NULL;
        if (sub) {
            unsigned d = menu_depth(sub, depth + 1u);
            if (d > best) best = d;
        }
    }
    return best;
}

/* the text at guest address p (NUL-terminated UTF-16); NULL for p == 0 */
static uint16_t *read_text(const char *api, uint64_t p)
{
    size_t len = 0;

    if (p == 0) return NULL;
    return fle_w_read_alloc(api, p, MENU_TEXT_MAX, &len);
}

/* win32u/menu.c set_menu_item_info */
static int set_item_info(const char *api, fle_mitem *it, const fle_mii *info)
{
    if (info->mask & MIIM_FTYPE) {
        it->type &= ~TYPE_MII_MASK;
        it->type |= info->type & TYPE_MII_MASK;
    }
    if (info->mask & MIIM_STRING) {
        free_item_text(it);
        it->text = read_text(api, info->typedata);
    }
    if (info->mask & MIIM_STATE) it->state = info->state & STATE_MII_MASK;
    if (info->mask & MIIM_ID) it->id = info->id;
    if (info->mask & MIIM_SUBMENU) {
        it->sub = info->sub;
        if (it->sub) {
            fle_menu *sub = get_menu(it->sub, 0);
            if (!sub) {
                fle_set_last_error(87u);
                return 0;
            }
            if (menu_depth(sub, 0) > MENU_DEPTH_MAX) {
                it->sub = 0;
                return 0;
            }
            sub->flags |= MF_POPUP;
            it->type |= MF_POPUP;
        } else {
            it->type &= ~MF_POPUP;
        }
    }
    if (info->mask & MIIM_CHECKMARKS) {
        it->chk = info->chk;
        it->unchk = info->unchk;
    }
    if (info->mask & MIIM_DATA) it->data = info->data;
    if (info->mask & MIIM_BITMAP) it->bmp = info->bmp;
    if (!it->text && !(it->type & MF_OWNERDRAW) && !it->bmp) it->type |= MF_SEPARATOR;
    return 1;
}

/* user32/menu.c MENU_NormalizeMenuItemInfoStruct: the size (80, or 72 without hbmpItem), the bit combinations, MIIM_TYPE turned into MIIM_FTYPE / MIIM_STRING / MIIM_BITMAP. 0 with the last error set. */
static int read_mii(uint64_t p, fle_mii *o)
{
    uint32_t cb;

    memset(o, 0, sizeof *o);
    if (p == 0) {
        fle_set_last_error(87u);
        return 0;
    }
    cb = (uint32_t)fle_ld(p, 4);
    if (cb != MII_SIZE && cb != MII_SIZE_OLD) {
        fle_set_last_error(87u);
        return 0;
    }
    o->cbSize = MII_SIZE;
    o->mask = (uint32_t)fle_ld(p + 4, 4);
    o->type = (uint32_t)fle_ld(p + 8, 4);
    o->state = (uint32_t)fle_ld(p + 12, 4);
    o->id = (uint32_t)fle_ld(p + 16, 4);
    o->sub = fle_ld(p + 24, 8);
    o->chk = fle_ld(p + 32, 8);
    o->unchk = fle_ld(p + 40, 8);
    o->data = fle_ld(p + 48, 8);
    o->typedata = fle_ld(p + 56, 8);
    o->cch = (uint32_t)fle_ld(p + 64, 4);
    o->bmp = cb == MII_SIZE ? fle_ld(p + 72, 8) : 0;
    return 1;
}

static int normalize_mii(uint64_t p, fle_mii *o)
{
    if (!read_mii(p, o)) return 0;
    if (((o->mask & MIIM_TYPE) && (o->mask & (MIIM_STRING | MIIM_FTYPE | MIIM_BITMAP))) || ((o->mask & MIIM_FTYPE) && (o->type & 0x4u))) {
        fle_set_last_error(87u);
        return 0;
    }
    if (o->mask & MIIM_TYPE) {
        o->mask |= MIIM_FTYPE;
        if (IS_STRING_ITEM(o->type)) o->mask |= MIIM_STRING;
        else if (o->type & 0x4u) {
            o->mask |= MIIM_BITMAP;
            o->bmp = o->typedata & 0xFFFFu;
        }
    }
    return 1;
}

/* user32/menu.c MENU_mnu2mnuii */
static void mnu2mii(uint32_t flags, uint64_t id, uint64_t str, fle_mii *o)
{
    memset(o, 0, sizeof *o);
    o->cbSize = MII_SIZE;
    o->mask = MIIM_STATE | MIIM_ID | MIIM_FTYPE;
    if (IS_STRING_ITEM(flags)) {
        o->mask |= MIIM_STRING | MIIM_BITMAP;
        if (str == 0) flags |= MF_SEPARATOR;
        else {
            if ((uint16_t)fle_ld(str, 2) == '\b') {   /* an item that begins with a backspace is a help item */
                flags |= MF_HELP;
                str += 2;
            }
            o->typedata = str;
        }
    } else if (flags & 0x4u) {
        o->mask |= MIIM_BITMAP | MIIM_STRING;
        o->bmp = str;
    }
    if (flags & MF_OWNERDRAW) {
        o->mask |= MIIM_DATA;
        o->data = str;
    }
    if (flags & MF_POPUP) {   /* IsMenu(id): on a handle that is not a menu Wine (and, a reading, Windows) sets ERROR_INVALID_MENU_HANDLE although the insertion goes on, as a plain item */
        if (get_menu(id, 1)) {
            o->mask |= MIIM_SUBMENU;
            o->sub = id;
        }
    }
    if (flags & MF_SEPARATOR) flags |= MF_GRAYED | MF_DISABLED;
    o->state = flags & STATE_MII_MASK & ~MF_DEFAULT;
    o->type = flags & TYPE_MII_MASK;
    o->id = (uint32_t)id;
    o->mask |= MIIM_CHECKMARKS;
}

/* win32u/menu.c insert_menu_item + the InsertMenuItem case of NtUserThunkedMenuItemInfo */
static int insert_with(const char *api, uint64_t h, uint32_t id, uint32_t flags, const fle_mii *info)
{
    fle_menu *m;
    uint32_t pos;
    fle_mitem *ni;
    int ok;

    if (!find_item(h, id, flags, &m, &pos)) {
        m = get_menu(h, 0);
        if (!m) return 0;
        pos = m->n;
    }
    ni = realloc(m->items, (m->n + 1u) * sizeof *ni);
    if (!ni) fle_stop(api, "out of memory");
    m->items = ni;
    memmove(&ni[pos + 1], &ni[pos], (m->n - pos) * sizeof *ni);
    memset(&ni[pos], 0, sizeof *ni);
    m->n++;
    ok = set_item_info(api, &ni[pos], info);
    if (!ok) {
        /* Wine does `if (!ret) RemoveMenu(handle, pos, flags)` with the ARGUMENTS of the insertion: the item that is removed is the one that find_item finds for them (for an append at -1, none: the half-made item stays) */
        fle_menu *rm;
        uint32_t rp;
        if (find_item(h, id, flags, &rm, &rp)) remove_at(rm, rp);
    }
    return ok;
}

void fle_imp_CreateMenu(void) { R[0] = new_menu(0); }
void fle_imp_CreatePopupMenu(void) { R[0] = new_menu(MF_POPUP); }

void fle_imp_DestroyMenu(void)
{
    fle_menu *m = get_menu(R[1], 0);

    if (!m) {
        R[0] = 0;
        return;
    }
    destroy_menu(m, 0);
    R[0] = 1;
}

void fle_imp_InsertMenuW(void)
{
    fle_mii mii;
    uint32_t flags = (uint32_t)R[8];

    mnu2mii(flags, R[9], fle_arg(5), &mii);
    R[0] = insert_with("InsertMenuW", R[1], (uint32_t)R[2], flags, &mii) ? 1u : 0u;
}

void fle_imp_InsertMenuItemW(void)
{
    fle_mii mii;
    uint64_t h = R[1], p = R[9];
    uint32_t item = (uint32_t)R[2], bypos = (uint32_t)R[8] != 0;

    if (!normalize_mii(p, &mii)) {
        R[0] = 0;
        return;
    }
    R[0] = insert_with("InsertMenuItemW", h, item, bypos ? MF_BYPOSITION : 0u, &mii) ? 1u : 0u;
}

void fle_imp_SetMenuItemInfoW(void)
{
    fle_mii mii;
    fle_menu *m;
    uint32_t pos;
    uint64_t h = R[1], p = R[9];
    uint32_t item = (uint32_t)R[2], bypos = (uint32_t)R[8] != 0;

    if (!normalize_mii(p, &mii)) {
        R[0] = 0;
        return;
    }
    if (!find_item(h, item, bypos ? MF_BYPOSITION : 0u, &m, &pos)) {
        R[0] = 0;
        return;
    }
    R[0] = set_item_info("SetMenuItemInfoW", &m->items[pos], &mii) ? 1u : 0u;
}

/* win32u/menu.c get_menu_item_info for the unicode form; `io` is read and filled, strings go to the guest address io->typedata */
static int get_info(uint64_t h, uint32_t id, uint32_t flags, fle_mii *io)
{
    fle_menu *m;
    uint32_t pos;
    fle_mitem *it;

    if (!find_item(h, id, flags, &m, &pos)) {
        fle_set_last_error(FLE_ERROR_MENU_ITEM_NOT_FOUND);   /* also for a handle that is not a menu (Wine) */
        return 0;
    }
    it = &m->items[pos];
    if (io->mask & MIIM_TYPE) {
        if (io->mask & (MIIM_STRING | MIIM_FTYPE | MIIM_BITMAP)) {
            fle_set_last_error(87u);
            return 0;
        }
        io->type = it->type & TYPE_MII_MASK;
        if (it->bmp && !(it->bmp >= 1 && it->bmp <= 11)) io->type |= 0x4u;   /* HBMMENU_* "magic" bitmaps are the values 1 to 11 */
        io->bmp = it->bmp;
        if (io->type & 0x4u) {
            io->typedata = it->bmp;
            io->cch = 0;
        } else if (io->type & (MF_OWNERDRAW | MF_SEPARATOR)) {
            io->typedata = 0;
            io->cch = 0;
        }
    }
    if (io->mask & (MIIM_TYPE | MIIM_STRING)) {
        if (!it->text) {
            if (io->typedata && io->cch) fle_st(io->typedata, 2, 0);
            io->cch = 0;
        } else {
            uint32_t len = 0, k;

            while (it->text[len]) len++;
            if (io->typedata && io->cch) {
                uint32_t n = len < io->cch - 1u ? len : io->cch - 1u;

                for (k = 0; k < n; k++) fle_st(io->typedata + 2u * k, 2, it->text[k]);
                fle_st(io->typedata + 2u * n, 2, 0);
                io->cch = io->cch <= len + 1u ? io->cch - 1u : len;
            } else {
                io->cch = len;
            }
        }
    }
    if (io->mask & MIIM_FTYPE) io->type = it->type & TYPE_MII_MASK;
    if (io->mask & MIIM_BITMAP) io->bmp = it->bmp;
    if (io->mask & MIIM_STATE) io->state = it->state & STATE_MII_MASK;
    if (io->mask & MIIM_ID) io->id = it->id;
    if (io->mask & MIIM_DATA) io->data = it->data;
    if (io->mask & MIIM_SUBMENU) io->sub = it->sub;
    else io->sub = 0;   /* hSubMenu is always cleared when not asked for */
    if (io->mask & MIIM_CHECKMARKS) {
        io->chk = it->chk;
        io->unchk = it->unchk;
    }
    return 1;
}

void fle_imp_GetMenuItemInfoW(void)
{
    fle_mii io;
    uint64_t h = R[1], p = R[9];
    uint32_t item = (uint32_t)R[2], bypos = (uint32_t)R[8] != 0;

    uint32_t cb = p ? (uint32_t)fle_ld(p, 4) : 0;

    if (cb != MII_SIZE && cb != MII_SIZE_OLD) {   /* the size without hbmpItem is accepted, too (user32 normalizes it, as for the insertion) */
        fle_set_last_error(87u);
        R[0] = 0;
        return;
    }
    (void)read_mii(p, &io);
    if (!get_info(h, item, bypos ? MF_BYPOSITION : 0u, &io)) {
        R[0] = 0;
        return;
    }
    /* back to the guest: only what Wine's get_menu_item_info writes */
    fle_st(p + 4, 4, io.mask);
    if (io.mask & (MIIM_TYPE | MIIM_FTYPE)) fle_st(p + 8, 4, io.type);
    if (io.mask & MIIM_STATE) fle_st(p + 12, 4, io.state);
    if (io.mask & MIIM_ID) fle_st(p + 16, 4, io.id);
    fle_st(p + 24, 8, io.sub);
    if (io.mask & MIIM_CHECKMARKS) {
        fle_st(p + 32, 8, io.chk);
        fle_st(p + 40, 8, io.unchk);
    }
    if (io.mask & MIIM_DATA) fle_st(p + 48, 8, io.data);
    if (io.mask & MIIM_TYPE) fle_st(p + 56, 8, io.typedata);
    if (io.mask & (MIIM_TYPE | MIIM_STRING)) fle_st(p + 64, 4, io.cch);
    if (cb == MII_SIZE && (io.mask & (MIIM_TYPE | MIIM_BITMAP))) fle_st(p + 72, 8, io.bmp);
    R[0] = 1;
}

void fle_imp_GetMenuStringW(void)
{
    fle_mii io;

    memset(&io, 0, sizeof io);
    io.cbSize = MII_SIZE;
    io.mask = MIIM_STRING;
    io.typedata = R[8];
    io.cch = (uint32_t)R[9];
    if (io.cch > 0x7FFFFFFFu) io.cch = 0x7FFFFFFFu;   /* count is an int */
    R[0] = get_info(R[1], (uint32_t)R[2], (uint32_t)fle_arg(5), &io) ? io.cch : 0u;
}

void fle_imp_GetMenuItemCount(void)
{
    fle_menu *m = get_menu(R[1], 0);

    R[0] = m ? (uint64_t)m->n : 0xFFFFFFFFull;
}

void fle_imp_GetMenuItemID(void)
{
    fle_menu *m;
    uint32_t pos;

    if (!find_item(R[1], (uint32_t)R[2], MF_BYPOSITION, &m, &pos)) {
        R[0] = 0xFFFFFFFFull;
        return;
    }
    R[0] = (m->items[pos].type & MF_POPUP) ? 0xFFFFFFFFull : (uint64_t)m->items[pos].id;
}

void fle_imp_GetSubMenu(void)
{
    fle_menu *m;
    uint32_t pos;

    if (!find_item(R[1], (uint32_t)R[2], MF_BYPOSITION, &m, &pos)) {
        R[0] = 0;
        return;
    }
    R[0] = (m->items[pos].type & MF_POPUP) ? m->items[pos].sub : 0;
}

void fle_imp_GetMenuState(void)
{
    fle_menu *m;
    uint32_t pos;
    fle_mitem *it;

    if (!find_item(R[1], (uint32_t)R[2], (uint32_t)R[8], &m, &pos)) {
        R[0] = 0xFFFFFFFFull;
        return;
    }
    it = &m->items[pos];
    if (it->type & MF_POPUP) {
        fle_menu *sub = get_menu(it->sub, 0);

        R[0] = sub ? (uint64_t)((sub->n << 8) | ((it->state | it->type) & 0xFFu)) : 0xFFFFFFFFull;
    } else {
        R[0] = (uint64_t)(it->type | it->state);
    }
}

void fle_imp_RemoveMenu(void)
{
    fle_menu *m;
    uint32_t pos;

    if (!find_item(R[1], (uint32_t)R[2], (uint32_t)R[8], &m, &pos)) {
        R[0] = 0;
        return;
    }
    remove_at(m, pos);
    R[0] = 1;
}

uint32_t fle_crash1_menu_delete_item(uint64_t h, uint32_t id, uint32_t flags)
{
    fle_menu *m;
    uint32_t pos;

    if (!find_item(h, id, flags, &m, &pos)) return 0;
    if (m->items[pos].type & MF_POPUP) {
        fle_menu *sub = get_menu(m->items[pos].sub, 0);
        if (sub) destroy_menu(sub, 0);
    }
    remove_at(m, pos);
    return 1;
}

uint32_t fle_crash1_menu_check_item(uint64_t h, uint32_t id, uint32_t flags)
{
    fle_menu *m;
    uint32_t pos, old;

    if (!find_item(h, id, flags, &m, &pos)) return 0xFFFFFFFFu;
    old = m->items[pos].state & MF_CHECKED;
    if (flags & MF_CHECKED) m->items[pos].state |= MF_CHECKED;
    else m->items[pos].state &= ~MF_CHECKED;
    return old;
}

uint32_t fle_crash1_menu_enable_item(uint64_t h, uint32_t id, uint32_t flags)
{
    fle_menu *m;
    uint32_t pos, old;

    if (!find_item(h, id, flags, &m, &pos)) return 0xFFFFFFFFu;
    old = m->items[pos].state & (MF_GRAYED | MF_DISABLED);
    m->items[pos].state ^= (old ^ flags) & (MF_GRAYED | MF_DISABLED);
    return old;
}

/* TrackPopupMenu(hMenu, uFlags, x, y, nReserved, hWnd, prcRect): nothing is displayed and nobody chooses; the menu is dismissed. The page: with TPM_RETURNCMD "If the user cancels the menu without making a selection, or if an error
 * occurs, the return value is zero"; without it "the return value is nonzero if the function succeeds". No message is sent to the window (S-crash1-menu-track). */
void fle_imp_TrackPopupMenu(void)
{
    uint64_t h = R[1], hwnd = fle_arg(6);
    uint32_t flags = (uint32_t)R[2];
    fle_win_view v;

    if (!get_menu(h, 1)) {
        R[0] = 0;
        return;
    }
    if (!fle_win_get(hwnd, &v) || !v.live) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE);
        R[0] = 0;
        return;
    }
    R[0] = (flags & 0x100u) ? 0u : 1u;
}
