/* comctl32 image lists, as far as the DRAG of an image needs them (decision 0319 section 14, alpha1, PROVISIONAL; PM 09:1x, O1 'the picture of the clip that is dragged is not drawn'; design: docs/decisions/0313 section 11).
 * Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9) and from the fifth on on the guest stack (fle_arg); the Win32 return value in R[0]; the guest return address stays on the stack.
 * Learn names no GetLastError value for any ImageList_ function: the last error is never touched.
 *
 * Implemented (the state of a list, the pixels the drag needs, the drag itself):
 *   ImageList_Create, Destroy, GetImageCount, GetIconSize, SetIconSize, GetBkColor, SetBkColor            (layer A of 0313 section 11)
 *   ImageList_Add, Replace                                                                                 (layer B: the pixels, copied out of the bitmaps; the drag image is made of them)
 *   ImageList_BeginDrag, DragEnter, DragMove, DragLeave, DragShowNolock, EndDrag, GetDragImage             (layer E)
 * Named stops (the engine and the plug-ins import them, nothing observed calls them yet): Remove, SetImageCount, GetIcon, Draw, DrawEx, Read, Write.
 * Not imported by any module of the reference install (api-coverage.tsv): SetDragCursorImage (no row; not implemented). */
#ifndef FLE_IMAGELIST_H
#define FLE_IMAGELIST_H

#include <stdint.h>

/* Handles of the lists: FLE_IL_HANDLE_BASE + 4 * (slot + 1), never reused (the pages give no value; 32-bit clean like the other handle regions of this runtime). */
/* The region [0x7FB90000, 0x7FBA0000) is the 64 KiB gap between the DC region (fle_gdc.h, up to 0x7FB90000) and the GDI region (fle_gdi.h, from 0x7FBA0000); 64 lists need 260 bytes. It overlaps no other virtual handle region below 2 GiB (lift/crates/fe-lift/tests/window.rs checks it). */
#define FLE_IL_HANDLE_BASE UINT64_C(0x7FB90000)
#define FLE_IL_REGION_SIZE UINT64_C(0x00010000)

void fle_imp_ImageList_Create(void);
void fle_imp_ImageList_Destroy(void);
void fle_imp_ImageList_GetImageCount(void);
void fle_imp_ImageList_GetIconSize(void);
void fle_imp_ImageList_SetIconSize(void);
void fle_imp_ImageList_GetBkColor(void);
void fle_imp_ImageList_SetBkColor(void);
void fle_imp_ImageList_Add(void);
void fle_imp_ImageList_Replace(void);
void fle_imp_ImageList_BeginDrag(void);
void fle_imp_ImageList_DragEnter(void);
void fle_imp_ImageList_DragMove(void);
void fle_imp_ImageList_DragLeave(void);
void fle_imp_ImageList_DragShowNolock(void);
void fle_imp_ImageList_EndDrag(void);
void fle_imp_ImageList_GetDragImage(void);
void fle_imp_ImageList_Remove(void);
void fle_imp_ImageList_SetImageCount(void);
void fle_imp_ImageList_GetIcon(void);
void fle_imp_ImageList_Draw(void);
void fle_imp_ImageList_DrawEx(void);
void fle_imp_ImageList_Read(void);
void fle_imp_ImageList_Write(void);

/* Inspection (tests and tools; no state moves): 1 and the state of a live list, 0 for any other handle. */
typedef struct {
    int32_t cx, cy, count;
    uint32_t flags, bk;
} fle_il_info;
int fle_il_query(uint64_t himl, fle_il_info *out);
/* 1 while a drag is begun (ImageList_BeginDrag without the EndDrag), and whether its image is shown. */
int fle_il_drag_active(int *shown);

#endif
