/* A minimal stand-in for gdiplus.dll for a plugin whose component constructor draws nothing that anyone sees (decision 0156, PROVISIONAL, S-vst-gdip). It is NOT GDI+: bitmaps are
 * pixel buffers, nothing is ever drawn, fonts and paths are opaque handles, and text metrics are fixed numbers. It exists so that a plugin that prepares its editor's images
 * and fonts while its component is created can be created at all. Functions are added one by one when a run reaches them; any gdiplus.dll function without an entry here has no
 * row and stops the run by name. The rows are in lift/harness/vst/imports-vst.txt (this file is harness code: it is compiled only by build_vst_native.sh / build_vst_wasm.sh) (fe-lift --extra-imports); no other module refers to this file.
 * The flat API and its Status codes are those of the Learn pages (gdiplusflat.h: GpStatus Ok = 0, GenericError = 1, InvalidParameter = 2, OutOfMemory = 3, NotImplemented = 6). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_runtime.h"

#define GP_OK 0u
#define GP_INVALID 2u

typedef struct { uint32_t magic; uint32_t kind; uint32_t w, h, stride, fmt; uint64_t scan0; uint8_t own; } vgp;
enum { VK_BITMAP = 1, VK_GRAPHICS, VK_FONT, VK_FAMILY, VK_BRUSH, VK_PEN, VK_PATH, VK_MATRIX, VK_ATTR, VK_GENERIC };
#define VGP_MAGIC 0x47445056u
static const char *vgp_trace;

static uint64_t vgp_new(uint32_t kind)
{
    vgp *o = (vgp *)calloc(1, sizeof *o + 64);
    o->magic = VGP_MAGIC;
    o->kind = kind;
    return (uint64_t)(uintptr_t)o;
}
static vgp *vgp_of(const char *api, uint64_t h, uint32_t kind)
{
    vgp *o = (vgp *)(uintptr_t)h;
    if (!h || o->magic != VGP_MAGIC || (kind && o->kind != kind)) fle_stop(api, "a handle that this stand-in did not hand out (or of another kind)");
    return o;
}
static void vgp_note(const char *api)
{
    if (!vgp_trace) vgp_trace = getenv("VST_TRACE") ? "1" : "0";
    if (vgp_trace[0] == '1') fprintf(stderr, "vst: gdiplus %s\n", api);
}
#define NOTE(a) vgp_note(a)
#define ARG(n) fle_arg(n)

/* kinds of simple functions */
#define GP_OK_FN(name) void fle_imp_##name(void) { NOTE(#name); R[0] = GP_OK; }
#define GP_NEW_FN(name, outarg, kind) void fle_imp_##name(void) { uint64_t out = ARG(outarg); NOTE(#name); if (!out) { R[0] = GP_INVALID; return; } fle_st(out, 8, vgp_new(kind)); R[0] = GP_OK; }
#define GP_DEL_FN(name, kind) void fle_imp_##name(void) { vgp *o; NOTE(#name); o = vgp_of(#name, R[1], kind); if (o->own) free((void *)(uintptr_t)o->scan0); o->magic = 0; free(o); R[0] = GP_OK; }

/* ---- images ---- */
void fle_imp_GdipCreateBitmapFromScan0(void)
{
    uint32_t w = (uint32_t)R[1], h = (uint32_t)R[2];
    int32_t stride = (int32_t)R[8];
    uint32_t fmt = (uint32_t)R[9];
    uint64_t scan0 = ARG(5), out = ARG(6);
    vgp *o;
    NOTE("GdipCreateBitmapFromScan0");
    if (!out || w == 0 || h == 0) { R[0] = GP_INVALID; return; }
    o = (vgp *)(uintptr_t)vgp_new(VK_BITMAP);
    o->w = w; o->h = h; o->fmt = fmt;
    o->stride = (uint32_t)(stride ? stride : (int32_t)(w * 4u));
    if (scan0) o->scan0 = scan0;
    else { o->scan0 = (uint64_t)(uintptr_t)calloc(1, (size_t)o->stride * h); o->own = 1; }
    fle_st(out, 8, (uint64_t)(uintptr_t)o);
    R[0] = GP_OK;
}
void fle_imp_GdipGetImageWidth(void) { vgp *o; NOTE("GdipGetImageWidth"); o = vgp_of("GdipGetImageWidth", R[1], VK_BITMAP); fle_st(R[2], 4, o->w); R[0] = GP_OK; }
void fle_imp_GdipGetImageHeight(void) { vgp *o; NOTE("GdipGetImageHeight"); o = vgp_of("GdipGetImageHeight", R[1], VK_BITMAP); fle_st(R[2], 4, o->h); R[0] = GP_OK; }
GP_DEL_FN(GdipDisposeImage, VK_BITMAP)
/* GdipBitmapLockBits(bitmap, const Rect *rect, UINT flags, PixelFormat format, BitmapData *data): BitmapData = { UINT Width; UINT Height; INT Stride; INT PixelFormat; void *Scan0; UINT_PTR Reserved } */
void fle_imp_GdipBitmapLockBits(void)
{
    vgp *o;
    uint64_t rect = R[2], data = ARG(5);
    int32_t x = 0, y = 0;
    uint32_t w, h;
    NOTE("GdipBitmapLockBits");
    o = vgp_of("GdipBitmapLockBits", R[1], VK_BITMAP);
    w = o->w; h = o->h;
    if (rect) { x = (int32_t)fle_ld(rect, 4); y = (int32_t)fle_ld(rect + 4, 4); w = (uint32_t)fle_ld(rect + 8, 4); h = (uint32_t)fle_ld(rect + 12, 4); }
    if (x < 0 || y < 0 || (uint64_t)x + w > o->w || (uint64_t)y + h > o->h || !data) { R[0] = GP_INVALID; return; }
    fle_st(data, 4, w); fle_st(data + 4, 4, h); fle_st(data + 8, 4, o->stride); fle_st(data + 12, 4, (uint32_t)R[9]);
    fle_st(data + 16, 8, o->scan0 + (uint64_t)y * o->stride + (uint64_t)x * 4u);
    fle_st(data + 24, 8, 0);
    R[0] = GP_OK;
}
void fle_imp_GdipBitmapUnlockBits(void) { NOTE("GdipBitmapUnlockBits"); vgp_of("GdipBitmapUnlockBits", R[1], VK_BITMAP); R[0] = GP_OK; }
