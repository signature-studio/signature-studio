/* user32!SystemParametersInfoW (docs/decisions/0058-virtual-spi.md; the page read is listed there and in
 * docs/investigations/spi-caller.md). Stateless. One uiAction is answered; the others the guest uses are named and stop, and
 * every other uiAction stops. Nothing here reads the host: no settings, no registry, no mouse, no theme, no font, no window,
 * no clock, no file. Nothing is written either: no parameter is set and no user profile is touched. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_spi.h"
#include "fle_monitor.h"

/* The uiActions that the guest's call sites pass (the literal in `mov ecx, imm`) and that are not answered. The names and the
 * numbers are the page's. `set` is 1 for a SPI_SET* action: it would change a system parameter, which this environment does
 * not keep (decision 0058). A GET action is listed because the page gives no default value for it. */
static const struct {
    uint32_t action;
    const char *name;
    int set;
} listed[] = {
    { 0x0026, "SPI_GETDRAGFULLWINDOWS", 0 },
    { 0x004B, "SPI_SETFONTSMOOTHING", 1 }, { 0x1016, "SPI_GETTOOLTIPANIMATION", 0 }, { 0x1018, "SPI_GETTOOLTIPFADE", 0 },
};

/* iMinAnimate of SPI_GETANIMATION / SPI_SETANIMATION (decision 0242): the value of this process, starting at the desktop default. */
static uint32_t min_animate = FLE_SPI_MINANIMATE;

/* The provisional system font of decision 0066 as the 92 bytes of a LOGFONTW (little-endian fields, lfFaceName as UTF-16). */
static void build_font(uint8_t *lf)
{
    static const char face[] = FLE_SPI_FONT_FACE;
    uint32_t h = (uint32_t)(int32_t)FLE_SPI_FONT_HEIGHT, w = FLE_SPI_FONT_WEIGHT;
    unsigned i;

    for (i = 0; i < FLE_SPI_LOGFONTW_SIZE; i++) lf[i] = 0;
    for (i = 0; i < 4; i++) {
        lf[i] = (uint8_t)(h >> (8 * i));       /* lfHeight */
        lf[16 + i] = (uint8_t)(w >> (8 * i));  /* lfWeight; lfWidth, lfEscapement, lfOrientation stay 0 */
    }
    lf[20] = 0; lf[21] = 0; lf[22] = 0;        /* lfItalic, lfUnderline, lfStrikeOut */
    lf[23] = FLE_SPI_FONT_CHARSET;
    lf[24] = FLE_SPI_FONT_OUTPRECISION;
    lf[25] = FLE_SPI_FONT_CLIPPRECISION;
    lf[26] = FLE_SPI_FONT_QUALITY;
    lf[27] = FLE_SPI_FONT_PITCHANDFAMILY;
    for (i = 0; face[i]; i++) lf[28 + 2 * i] = (uint8_t)face[i];   /* ASCII only: the high byte of each unit stays 0 */
}

/* Writes n bytes (a multiple of 4) at the guest address a. */
static void put_bytes(uint64_t a, const uint8_t *b, unsigned n)
{
    unsigned i;

    for (i = 0; i < n; i += 4)
        fle_st(a + i, 4, (uint64_t)b[i] | (uint64_t)b[i + 1] << 8 | (uint64_t)b[i + 2] << 16 | (uint64_t)b[i + 3] << 24);
}

/* user32!SystemParametersInfoW: BOOL f(UINT uiAction ecx, UINT uiParam edx, PVOID pvParam r8, UINT fWinIni r9d). Learn: the
 * return value is nonzero on success; on failure 0 and GetLastError gives the extended error information. The page does not say
 * which argument forms fail, so a form it does not define stops instead of returning 0.
 * Answered:
 *  - SPI_GETWHEELSCROLLLINES (0x68): "The pvParam parameter must point to a UINT variable that receives the number of lines.
 *    The default value is 3." uiParam is not used ("If not otherwise indicated, you must specify zero"); fWinIni is only for a
 *    set action. Writes the 4 bytes at pvParam and returns 1. The last error is left alone.
 *  - SPI_GETICONTITLELOGFONT (0x1f), decision 0066 (PROVISIONAL values): "The uiParam parameter specifies the size of a LOGFONT
 *    structure, and the pvParam parameter must point to the LOGFONT structure to fill in." The guest passes 0x5c and a LOGFONTW;
 *    uiParam other than 0x5c, a NULL pvParam and a fWinIni other than 0 are forms the page does not define: they stop. Writes the 92
 *    bytes of the provisional system font and returns 1. The last error is left alone.
 *  - SPI_GETNONCLIENTMETRICS (0x29), decision 0066: "pvParam must point to a NONCLIENTMETRICS structure ... Set the cbSize member of
 *    this structure and the uiParam parameter to sizeof(NONCLIENTMETRICS)". cbSize must be 0x1f8 or 0x1f4 (the structure with and
 *    without iPaddedBorderWidth); uiParam is cbSize (the page's rule) or 0 (the guest's form at 0x840045, 0x866493 and 0x991d3d;
 *    the page does not say 0 fails); another uiParam stops. Writes the four fonts the guest reads (caption, menu, status, message),
 *    iBorderWidth = 1 and, for cbSize 0x1f8 only, iPaddedBorderWidth = 4 (both the page's defaults). Every other field is
 *    undecided (the guest reads none of them as far as the static reading goes) and is written as 0 so that no stack garbage
 *    is read; cbSize and the bytes after the structure are not touched.
 * Only the low DWORD of rcx, rdx and r9 is an argument (UINT). */
void fle_imp_SystemParametersInfoW(void)
{
    static const char api[] = "SystemParametersInfoW";
    uint32_t action = (uint32_t)R[1];
    uint32_t ui = (uint32_t)R[2];
    uint32_t ini = (uint32_t)R[9];
    char why[200];
    unsigned i;

    if (action == FLE_SPI_GETWHEELSCROLLLINES) {
        if (R[8] == 0) fle_stop(api, "SPI_GETWHEELSCROLLLINES with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETWHEELSCROLLLINES with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETWHEELSCROLLLINES with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, FLE_SPI_WHEELSCROLLLINES);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETWORKAREA) {
        /* Decision 0098 (PROVISIONAL; the condition is the one decision 0065 wrote down): the RECT is the work area of the primary monitor,
         * the same four numbers as rcWork of GetMonitorInfoW (fle_monitor.h), which has no taskbar. */
        if (R[8] == 0) fle_stop(api, "SPI_GETWORKAREA with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETWORKAREA with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETWORKAREA with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, (uint32_t)FLE_MONITOR_WORK_LEFT);
        fle_st(R[8] + 4, 4, (uint32_t)FLE_MONITOR_WORK_TOP);
        fle_st(R[8] + 8, 4, (uint32_t)FLE_MONITOR_WORK_RIGHT);
        fle_st(R[8] + 12, 4, (uint32_t)FLE_MONITOR_WORK_BOTTOM);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETFONTSMOOTHINGCONTRAST) {
        /* Decision 0102 W13 (provisional): "Retrieves a contrast value that is used in ClearType smoothing. The pvParam parameter must
         * point to a UINT that receives the information. Valid contrast values are from 1000 to 2200. The default value is 1400." The
         * default is written (nothing in this environment sets another); the value of a real machine is not known. uiParam and fWinIni
         * are not defined for a get action, as for the wheel lines. */
        if (R[8] == 0) fle_stop(api, "SPI_GETFONTSMOOTHINGCONTRAST with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGCONTRAST with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGCONTRAST with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, FLE_SPI_FONTSMOOTHINGCONTRAST);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETCURSORSHADOW) {
        /* Decision 0106 (PROVISIONAL value, none on the page): a BOOL at pvParam (4 bytes). FALSE (see fle_spi.h). The same argument rules as the
         * contrast: uiParam and fWinIni are not defined for a get action. */
        if (R[8] == 0) fle_stop(api, "SPI_GETCURSORSHADOW with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETCURSORSHADOW with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETCURSORSHADOW with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, FLE_SPI_CURSORSHADOW);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETFONTSMOOTHINGTYPE) {
        /* Decision 0103 (PROVISIONAL value): "Retrieves the type of font smoothing. The pvParam parameter must point to a UINT that
         * receives the information. The possible values are FE_FONTSMOOTHINGSTANDARD and FE_FONTSMOOTHINGCLEARTYPE." The GET entry names
         * no default (decision 0102 left this action a stop for that reason); the SET entry (0x200B) says "The default is
         * FE_FONTSMOOTHINGSTANDARD", which is the value answered (see fle_spi.h). The same argument rules as the contrast. */
        if (R[8] == 0) fle_stop(api, "SPI_GETFONTSMOOTHINGTYPE with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGTYPE with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGTYPE with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, FLE_SPI_FONTSMOOTHING_STANDARD);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETFONTSMOOTHINGORIENTATION) {
        /* Decision 0103 (PROVISIONAL value, none on the page): "Retrieves the font smoothing orientation. The pvParam parameter must point
         * to a UINT that receives the information. The possible values are FE_FONTSMOOTHINGORIENTATIONBGR and
         * FE_FONTSMOOTHINGORIENTATIONRGB." Neither is named the default; RGB (1) is answered (see fle_spi.h). The same argument rules. */
        if (R[8] == 0) fle_stop(api, "SPI_GETFONTSMOOTHINGORIENTATION with pvParam NULL: the page does not define it");
        if (ui != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGORIENTATION with uiParam not zero: the page says to specify zero, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETFONTSMOOTHINGORIENTATION with fWinIni not zero: the page gives fWinIni for a set action only");
        fle_st(R[8], 4, FLE_SPI_FONTSMOOTHINGORIENTATION_RGB);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETANIMATION) {
        /* Decision 0126 wall 92 (PROVISIONAL, S-spi-animation, a CHOICE WITHOUT A SOURCE): Learn SPI_GETANIMATION: "Retrieves the animation effects associated with user actions. The pvParam parameter must point to an
         * ANIMATIONINFO structure that receives the information. Set the cbSize member of this structure and the uiParam parameter to sizeof(ANIMATIONINFO)"; ANIMATIONINFO { UINT cbSize; int iMinAnimate }: "If this
         * member is nonzero, minimize and restore animation is enabled; otherwise it is disabled". The pages give no default. 0 is answered: the virtual machine has no minimize / restore animation (nothing in the
         * model animates a window; withdrawn when a measurement of a real configuration is available). uiParam and cbSize other than 8, a NULL pvParam and fWinIni not 0 stop (the page does not define them). */
        if (R[8] == 0) fle_stop(api, "SPI_GETANIMATION with pvParam NULL: the page does not define it");
        if (ui != FLE_SPI_ANIMATIONINFO_SIZE) fle_stop(api, "SPI_GETANIMATION with uiParam not sizeof(ANIMATIONINFO) = 8: the page says to set it, the result is not defined");
        if (ini != 0) fle_stop(api, "SPI_GETANIMATION with fWinIni not zero: the page gives fWinIni for a set action only");
        if ((uint32_t)fle_ld(R[8], 4) != FLE_SPI_ANIMATIONINFO_SIZE) fle_stop(api, "SPI_GETANIMATION with cbSize not 8: the page says the caller sets it to sizeof(ANIMATIONINFO)");
        fle_st(R[8] + 4, 4, min_animate);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_SETANIMATION) {
        /* Decision 0242 (pick1): Learn SPI_SETANIMATION: "Sets the animation effects associated with user actions. The pvParam parameter must point to an ANIMATIONINFO structure that contains the new
         * parameters. Set the cbSize member of this structure and the uiParam parameter to sizeof(ANIMATIONINFO), and set the iMinAnimate member to specify whether minimize/restore animation is enabled";
         * the return value is nonzero on success. The new iMinAnimate is kept for SPI_GETANIMATION (this process only: fWinIni's SPIF_UPDATEINIFILE / SPIF_SENDCHANGE write no user profile and send no
         * broadcast, as everywhere in this file). The same argument rules as the get action. */
        if (R[8] == 0) fle_stop(api, "SPI_SETANIMATION with pvParam NULL: the page does not define it");
        if (ui != FLE_SPI_ANIMATIONINFO_SIZE) fle_stop(api, "SPI_SETANIMATION with uiParam not sizeof(ANIMATIONINFO) = 8: the page says to set it, the result is not defined");
        if ((uint32_t)fle_ld(R[8], 4) != FLE_SPI_ANIMATIONINFO_SIZE) fle_stop(api, "SPI_SETANIMATION with cbSize not 8: the page says the caller sets it to sizeof(ANIMATIONINFO)");
        min_animate = (uint32_t)fle_ld(R[8] + 4, 4) != 0;
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETICONTITLELOGFONT) {
        uint8_t lf[FLE_SPI_LOGFONTW_SIZE];

        if (R[8] == 0) fle_stop(api, "SPI_GETICONTITLELOGFONT with pvParam NULL: the page does not define it");
        if (ui != FLE_SPI_LOGFONTW_SIZE) fle_stop(api, "SPI_GETICONTITLELOGFONT with uiParam not sizeof(LOGFONTW) = 0x5c: the page does not define it");
        if (ini != 0) fle_stop(api, "SPI_GETICONTITLELOGFONT with fWinIni not zero: the page gives fWinIni for a set action only");
        build_font(lf);
        put_bytes(R[8], lf, FLE_SPI_LOGFONTW_SIZE);
        R[0] = 1;
        return;
    }
    if (action == FLE_SPI_GETNONCLIENTMETRICS) {
        uint8_t lf[FLE_SPI_LOGFONTW_SIZE], zero[FLE_SPI_LOGFONTW_SIZE];
        uint64_t p = R[8];
        uint32_t cb;
        static const uint32_t fonts[4] = { FLE_SPI_NCM_CAPTIONFONT, FLE_SPI_NCM_MENUFONT, FLE_SPI_NCM_STATUSFONT, FLE_SPI_NCM_MESSAGEFONT };

        if (p == 0) fle_stop(api, "SPI_GETNONCLIENTMETRICS with pvParam NULL: the page does not define it");
        if (ini != 0) fle_stop(api, "SPI_GETNONCLIENTMETRICS with fWinIni not zero: the page gives fWinIni for a set action only");
        cb = (uint32_t)fle_ld(p, 4);
        if (cb != FLE_SPI_NCM_SIZE && cb != FLE_SPI_NCM_SIZE_NO_PADDED)
            fle_stop(api, "SPI_GETNONCLIENTMETRICS with cbSize not 0x1f8 or 0x1f4: the page says cbSize is sizeof(NONCLIENTMETRICS) and defines no other");
        if (ui != 0 && ui != cb)
            fle_stop(api, "SPI_GETNONCLIENTMETRICS with uiParam neither 0 (the guest's form) nor cbSize (the page's): not defined");
        build_font(lf);
        for (i = 0; i < FLE_SPI_LOGFONTW_SIZE; i++) zero[i] = 0;
        fle_st(p + FLE_SPI_NCM_BORDERWIDTH, 4, FLE_SPI_BORDERWIDTH);
        for (i = 8; i < FLE_SPI_NCM_CAPTIONFONT; i += 4) fle_st(p + i, 4, 0);         /* iScrollWidth .. iCaptionHeight: undecided */
        put_bytes(p + FLE_SPI_NCM_SMCAPTIONFONT - 8, zero, 8);                        /* iSmCaptionWidth, iSmCaptionHeight: undecided */
        put_bytes(p + FLE_SPI_NCM_SMCAPTIONFONT, zero, FLE_SPI_LOGFONTW_SIZE);        /* lfSmCaptionFont: undecided */
        put_bytes(p + FLE_SPI_NCM_MENUFONT - 8, zero, 8);                             /* iMenuWidth, iMenuHeight: undecided */
        for (i = 0; i < 4; i++) put_bytes(p + fonts[i], lf, FLE_SPI_LOGFONTW_SIZE);
        if (cb == FLE_SPI_NCM_SIZE) fle_st(p + FLE_SPI_NCM_PADDEDBORDER, 4, FLE_SPI_PADDEDBORDERWIDTH);
        R[0] = 1;
        return;
    }
    for (i = 0; i < sizeof listed / sizeof listed[0]; i++) {
        if (listed[i].action != action) continue;
        if (listed[i].set)
            snprintf(why, sizeof why, "uiAction 0x%x (%s) is used by the guest and sets a system parameter: this environment keeps and writes no setting (decision 0058)", (unsigned)action, listed[i].name);
        else
            snprintf(why, sizeof why, "uiAction 0x%x (%s) is used by the guest but no value is decided (decision 0058)", (unsigned)action, listed[i].name);
        fle_stop(api, why);
    }
    snprintf(why, sizeof why, "uiAction 0x%x is not answered (the guest's call sites do not use it)", (unsigned)action);
    fle_stop(api, why);
}
