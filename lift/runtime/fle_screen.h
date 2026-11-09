/* The size of the virtual screen as a value of the RUN (decision 0212, screen1, PROVISIONAL, opt-in).
 *
 * Until now the virtual display was a constant of the build: FLE_GDC_HORZRES x FLE_GDC_VERTRES = 1920 x 1080 (fle_gdc.h, decision 0053, a choice without a source) and
 * every other place that answers a screen size (GetSystemMetrics SM_CXSCREEN.., GetDeviceCaps HORZRES.., the monitor rectangle and its work area of fle_monitor.h, EnumDisplaySettingsW,
 * SPI_GETWORKAREA, the cursor clip, the rectangle of the desktop window, the default window size) is that pair by NAME. This header makes the pair a value read once from the
 * engine's environment, like the refresh rate of decision 0166 (FLE_DWM_HZ): the page measures the room it has for the screen and passes
 *     FLE_MONITOR_W=<pixels>   FLE_MONITOR_H=<pixels>
 * through the manifest's engine.env. Unset (the default of every bundle and of every test) they are 1920 and 1080 and nothing differs from the constant. A value that is not a whole
 * number of decimal digits, or is outside FLE_SCREEN_MIN_*..FLE_SCREEN_MAX_*, is ignored (the default is used for that one dimension); the page validates before it passes.
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

/* The DPI of the virtual display: 96 (100%), what GetDeviceCaps(LOGPIXELSX / LOGPIXELSY) and shcore!GetDpiForMonitor answer, and the physical size in millimetres follows it (decision 0243: the page has no DPI choice any more; the process is DPI-unaware, fle_proc.c). */
#define FLE_SCREEN_DPI_DEFAULT 96u
static inline uint32_t fle_screen_dpi(void) { return FLE_SCREEN_DPI_DEFAULT; }

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
/* The physical size in millimetres at the DPI of the display, rounded to the nearest (decision 0121): pixels * 25.4 / dpi = pixels * 254 / (dpi * 10). At 96 DPI 1920 -> 508 and 1080 -> 286 (285.75), as the constants were. */
#define FLE_SCREEN_HORZSIZE_MM ((int32_t)(((uint32_t)FLE_SCREEN_PX_W * 254u + fle_screen_dpi() * 5u) / (fle_screen_dpi() * 10u)))
#define FLE_SCREEN_VERTSIZE_MM ((int32_t)(((uint32_t)FLE_SCREEN_PX_H * 254u + fle_screen_dpi() * 5u) / (fle_screen_dpi() * 10u)))

#endif
