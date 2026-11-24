/* The size of the virtual screen as a value of the RUN (decision 0212, screen1, PROVISIONAL, opt-in).
 *
 * Until now the virtual display was a constant of the build: FLE_GDC_HORZRES x FLE_GDC_VERTRES = 1920 x 1080 (fle_gdc.h, decision 0053, a choice without a source) and
 * every other place that answers a screen size (GetSystemMetrics SM_CXSCREEN.., GetDeviceCaps HORZRES.., the monitor rectangle and its work area of fle_monitor.h, EnumDisplaySettingsW,
 * SPI_GETWORKAREA, the cursor clip, the rectangle of the desktop window, the default window size) is that pair by NAME. This header makes the pair a value read once from the
 * engine's environment, like the refresh rate of decision 0166 (FLE_DWM_HZ): the page measures the room it has for the screen and passes
 *     FLE_MONITOR_W=<pixels>   FLE_MONITOR_H=<pixels>   FLE_MONITOR_DPR=<ratio x 10000>
 * through the manifest's engine.env. Unset (the default of every bundle and of every test) they are 1920, 1080 and 10000 (DPR 1: 100%, DPI 96) and nothing differs from the constants.
 * The ratio is the page's device pixel ratio as a whole number of ten-thousandths (DPR 1.5 is 15000). The scale and the DPI of the display follow from it (fle_screen_scale_percent and
 * fle_screen_dpi below): the guest's screen is the device pixels of the room, and the guest is told the DPI of those pixels. A value that is not a whole number of decimal digits, or is
 * outside its limits (FLE_SCREEN_MIN_*..FLE_SCREEN_MAX_* for the sizes, FLE_SCREEN_DPR_MIN_E4..FLE_SCREEN_DPR_MAX_E4 for the ratio), is ignored (the default is used for that one dimension);
 * the page validates before it passes.
 *
 * Header only (static inline functions and one weak variable, the same answer in every translation unit): the runtime files that answer a screen size are linked into tests and builds
 * that do not all link one more file. Single guest thread. The environment is read at the first use and is not looked at again; fle_screen_set changes the pair afterwards (section 3 of the
 * decision). The environment is the engine's own configuration, not the host's display: no display, no monitor, no DPI setting of the host is read. */
#ifndef FLE_SCREEN_SIZE_H
#define FLE_SCREEN_SIZE_H

#include <stdint.h>
#include <stdlib.h>

#define FLE_SCREEN_DEFAULT_W 1920u   /* = FLE_GDC_HORZRES of fle_gdc.h and FLE_MONITOR_RIGHT of fle_monitor.h (the checks are in those headers) */
#define FLE_SCREEN_DEFAULT_H 1080u
/* S-screen1-limits (a choice without a source): 8K at the most. The lower bound is what the guest's own layout needs to have any room (FL's minimum window); not measured. */
#define FLE_SCREEN_MIN_W 640u
#define FLE_SCREEN_MIN_H 480u
#define FLE_SCREEN_MAX_W 7680u
#define FLE_SCREEN_MAX_H 4320u

static inline uint32_t fle_screen_dim(const char *name, uint32_t def, uint32_t lo, uint32_t hi)
{
    const char *e = getenv(name);
    uint32_t v = 0;

    if (e == NULL || *e == '\0') return def;
    for (; *e != '\0'; e++) {
        if (*e < '0' || *e > '9' || v > 100000u) return def;
        v = v * 10u + (uint32_t)(*e - '0');
    }
    return v >= lo && v <= hi ? v : def;
}

/* The one pair of numbers every translation unit shares: a WEAK definition in the header (the linker keeps one), so that no file has to be added to a link line (the tests link fixed lists of runtime
 * files) and a later change (fle_screen_set) is seen everywhere. 0 = not read from the environment yet. */
__attribute__((weak)) uint32_t fle_screen_px[2];

static inline uint32_t fle_screen_w(void)
{
    if (fle_screen_px[0] == 0) fle_screen_px[0] = fle_screen_dim("FLE_MONITOR_W", FLE_SCREEN_DEFAULT_W, FLE_SCREEN_MIN_W, FLE_SCREEN_MAX_W);
    return fle_screen_px[0];
}

static inline uint32_t fle_screen_h(void)
{
    if (fle_screen_px[1] == 0) fle_screen_px[1] = fle_screen_dim("FLE_MONITOR_H", FLE_SCREEN_DEFAULT_H, FLE_SCREEN_MIN_H, FLE_SCREEN_MAX_H);
    return fle_screen_px[1];
}

/* The device pixel ratio of the page, in the engine's environment as FLE_MONITOR_DPR=<ratio x 10000> (a whole number: 1.5 is 15000; the page passes Math.round(dpr * 10000)), read with the same
 * reader as the sizes. Unset, or outside FLE_SCREEN_DPR_MIN_E4..FLE_SCREEN_DPR_MAX_E4 (a choice without a source: 0.25 to 5, the browser's zoom range the page can pass), it is
 * FLE_SCREEN_DPR_DEFAULT_E4 (100%). */
#define FLE_SCREEN_DPR_DEFAULT_E4 10000u
#define FLE_SCREEN_DPR_MIN_E4     2500u
#define FLE_SCREEN_DPR_MAX_E4     50000u
__attribute__((weak)) uint32_t fle_screen_dpr_e4[1];

static inline uint32_t fle_screen_dpr(void)
{
    if (fle_screen_dpr_e4[0] == 0) fle_screen_dpr_e4[0] = fle_screen_dim("FLE_MONITOR_DPR", FLE_SCREEN_DPR_DEFAULT_E4, FLE_SCREEN_DPR_MIN_E4, FLE_SCREEN_DPR_MAX_E4);
    return fle_screen_dpr_e4[0];
}

/* The scale in percent: round(DPR x 100) (Windows' scale factor for this DPR; the rounding is half up). */
static inline uint32_t fle_screen_scale_percent(void) { return (fle_screen_dpr() + 50u) / 100u; }

/* The DPI of the display, the value that GetDeviceCaps(LOGPIXELSX / LOGPIXELSY), shcore!GetDpiForMonitor and the scaled system metrics answer: MulDiv(96, scale percent, 100)
 * (96 is the DPI of 100%, FLE_SCREEN_DPI_DEFAULT). MulDiv rounds to the nearest whole number. Unset: 96, as before. */
#define FLE_SCREEN_DPI_DEFAULT 96u
static inline uint32_t fle_screen_dpi(void) { return (FLE_SCREEN_DPI_DEFAULT * fle_screen_scale_percent() + 50u) / 100u; }

/* Sets the size of the run (a display change while the engine runs, decision 0212 section 3); returns 0 and changes nothing when a dimension is outside the limits. The caller (fle_input.c, apply_display)
 * tells the guest; this function only changes the numbers every answer is made from. */
static inline int fle_screen_set(uint32_t w, uint32_t h)
{
    if (w < FLE_SCREEN_MIN_W || w > FLE_SCREEN_MAX_W || h < FLE_SCREEN_MIN_H || h > FLE_SCREEN_MAX_H) return 0;
    fle_screen_px[0] = w;
    fle_screen_px[1] = h;
    return 1;
}

#define FLE_SCREEN_PX_DPI ((int32_t)fle_screen_dpi())

#define FLE_SCREEN_PX_W ((int32_t)fle_screen_w())
#define FLE_SCREEN_PX_H ((int32_t)fle_screen_h())
/* The physical size in millimetres, rounded to the nearest (decision 0121): pixels * 25.4 / 96 = pixels * 254 / (96 * 10), at the 96 DPI of 100%. The size of the monitor does not change with the
 * scale, so the scale does not enter here. At 96 DPI 1920 -> 508 and 1080 -> 286 (285.75), as the constants were. */
#define FLE_SCREEN_HORZSIZE_MM ((int32_t)(((uint32_t)FLE_SCREEN_PX_W * 254u + FLE_SCREEN_DPI_DEFAULT * 5u) / (FLE_SCREEN_DPI_DEFAULT * 10u)))
#define FLE_SCREEN_VERTSIZE_MM ((int32_t)(((uint32_t)FLE_SCREEN_PX_H * 254u + FLE_SCREEN_DPI_DEFAULT * 5u) / (FLE_SCREEN_DPI_DEFAULT * 10u)))

#endif
