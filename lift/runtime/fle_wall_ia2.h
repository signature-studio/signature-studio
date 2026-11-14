/* Walls of the second wave of user operations (decision 0191, PROVISIONAL). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0].
 * Each function is reached only through a row of harness/seeds/ia2-imports.txt (layer 4; an earlier build has no row and stops at the import thunk as before). */
#ifndef FLE_WALL_IA2_H
#define FLE_WALL_IA2_H

#include <stdint.h>
#include <stddef.h>

/* user32!GetKeyNameTextW: int f(LONG lParam ecx, LPWSTR lpString rdx, int cchSize r8d) (wall ia2-W18: File > Save / Save as / Save new version in the shipped bundle).
 * The key name of the US layout (kbdus.c's aKeyNames / aKeyNamesExt, Microsoft's keyboard layout sample), or the key's own character for a key that produces one. */
void fle_imp_GetKeyNameTextW(void);

/* (decision 0284: the primary sort weights of CompareStringW, fle_ia2_cmp_weight / fle_ia2_cmp_primary, live in fle_locale.c now, as static functions) */
/* LoadLibraryW with a relative path that the virtual volume is read as not holding (wall ia2-W21): 1 when the name (n code units) is in the list */
int fle_ia2_path_absent(const uint16_t *u, size_t n);

/* kernel32!GetPrivateProfileStringW (wall ia2-W17), the parser alone (fle_wall_ia2v.c): data = the file's bytes (ASCII), app / key NULL = list the sections / the keys; writes at most nsize units (nsize >= 3) as the Learn page says
 * and returns the count it gives. */
size_t fle_ia2_ini_get(const uint8_t *data, size_t n, const uint16_t *app, const uint16_t *key, const uint16_t *def, uint16_t *out, uint32_t nsize);
void fle_imp_GetPrivateProfileStringW(void);
/* kernel32!WritePrivateProfileStringW (wall ia2-W30): the text editing alone (out gets the new file; returns its size) and the API; see fle_wall_ia2v.c */
size_t fle_ia2_ini_put(const uint8_t *in, size_t n, const uint16_t *app, const uint16_t *key, const uint16_t *str, uint8_t *out, size_t cap);
void fle_imp_WritePrivateProfileStringW(void);

/* user32!MessageBoxW (wall ia2-W28): the default button is answered, no window is shown (S-ia2-msgbox-default); see fle_wall_ia2.c */
void fle_imp_MessageBoxW(void);
/* kernel32!GetTimeZoneInformationForYear (wall ia2-W21), fle_wall_ia2t.c */
void fle_imp_GetTimeZoneInformationForYear(void);
/* shell32!ShellExecuteW (walls ia2-W16/W29): answers SE_ERR_NOASSOC, nothing is started (S-ia2-shellexecute-noassoc) */
void fle_imp_ShellExecuteW(void);

/* user32!ScrollWindowEx (wall ia2-W23) and ole32!DoDragDrop (wall ia2-W24): see fle_wall_ia2.c */
void fle_imp_ScrollWindowEx(void);
void fle_imp_ScrollWindow(void);   /* decision 0319 */
/* user32!InvalidateRgn (wall ia2-W31): the region's bounding box is added to the update region; see fle_wall_ia2w.c */
void fle_imp_InvalidateRgn(void);
void fle_imp_DoDragDrop(void);

#endif

