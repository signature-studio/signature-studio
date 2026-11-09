/* Decision 0209 (crash1): user32's menu family, from the Learn pages and checked against Wine 10.0 (wine/ of the decision): CreateMenu, CreatePopupMenu, DestroyMenu, InsertMenuW, InsertMenuItemW, SetMenuItemInfoW,
 * GetMenuItemInfoW, GetMenuItemCount, GetMenuItemID, GetMenuState, GetMenuStringW, GetSubMenu, RemoveMenu, TrackPopupMenu (+ the hooks that CheckMenuItem, EnableMenuItem and DeleteMenu of fle_win.c use for these menus).
 * Learn pages (winuser.h, read 2026-11-04): nf-winuser-createmenu, -createpopupmenu, -destroymenu, -insertmenuw, -insertmenuitemw, -setmenuiteminfow, -getmenuiteminfow, -getmenuitemcount, -getmenuitemid,
 * -getmenustate, -getmenustringw, -getsubmenu, -removemenu, -trackpopupmenu, ns-winuser-menuiteminfow. Where a page is silent (the exact errors, the position rules) the behaviour is Wine 10.0's (win32u/menu.c, user32/menu.c): UNOFFICIAL.
 * Nothing is displayed: a menu is a list of items. TrackPopupMenu shows nothing and the menu is dismissed without a choice (S-crash1-menu-track).
 * HMENU values are 0x7F900000 + 4 * (n + 1), never reused. The system menu of fle_win.c (GetSystemMenu) is a different object and is not one of these. */
#ifndef FLE_CRASH1_MENU_H
#define FLE_CRASH1_MENU_H

#include <stdint.h>

#define FLE_CMENU_HANDLE_BASE 0x7F900000ull
#define FLE_CMENU_MAX 100000u   /* the region 0x7F900000 .. 0x7F96FFFF lies before the system-menu region (fle_win.h) */
#define FLE_ERROR_INVALID_MENU_HANDLE 1401u
#define FLE_ERROR_MENU_ITEM_NOT_FOUND 1456u

void fle_imp_CreateMenu(void);
void fle_imp_CreatePopupMenu(void);
void fle_imp_DestroyMenu(void);
void fle_imp_InsertMenuW(void);
void fle_imp_InsertMenuItemW(void);
void fle_imp_SetMenuItemInfoW(void);
void fle_imp_GetMenuItemInfoW(void);
void fle_imp_GetMenuItemCount(void);
void fle_imp_GetMenuItemID(void);
void fle_imp_GetMenuState(void);
void fle_imp_GetMenuStringW(void);
void fle_imp_GetSubMenu(void);
void fle_imp_RemoveMenu(void);
void fle_imp_TrackPopupMenu(void);

/* 1 when `h` is a live menu of this table (IsMenu). */
int fle_crash1_menu_owns(uint64_t h);
/* CheckMenuItem / EnableMenuItem / DeleteMenu on one of these menus: the result of the API (CheckMenuItem: the previous MF_CHECKED or MF_UNCHECKED, or 0xFFFFFFFF; EnableMenuItem: the previous
 * MF_GRAYED/MF_DISABLED bits, or 0xFFFFFFFF; DeleteMenu: 1 or 0). */
uint32_t fle_crash1_menu_check_item(uint64_t h, uint32_t id, uint32_t flags);
uint32_t fle_crash1_menu_enable_item(uint64_t h, uint32_t id, uint32_t flags);
uint32_t fle_crash1_menu_delete_item(uint64_t h, uint32_t id, uint32_t flags);

#endif
