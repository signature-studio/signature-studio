/* user32!SystemParametersInfoW for the uiActions the guest asks and for which this environment has a value
 * (docs/decisions/0058-virtual-spi.md; the page read and the source of every number and value are listed there and in
 * docs/investigations/spi-caller.md). Same convention as fle_imports.h: the arguments in R[1] (ecx, uiAction), R[2] (edx,
 * uiParam), R[8] (r8, pvParam), R[9] (r9d, fWinIni); the Win32 return value in R[0]; the guest return address stays on the
 * stack for the caller's ret. Stateless, so there is no thread rule.
 *
 * Nothing is read from the host: no settings, no registry, no mouse, no theme, no font. The values below are values of this
 * lift environment, fixed, so that a run does not depend on the machine it runs on. Nothing is written either: a call that
 * would set a parameter or write the user profile stops.
 *
 * Every other call stops the run through fle_stop (fle_env.h): "fle: SystemParametersInfoW: <why>" and abort(). A value is
 * never made up for it. */
#ifndef FLE_SPI_H
#define FLE_SPI_H

#include <stdint.h>

#include "fle_gdc.h"

/* SPI_* numbers: the Learn page of SystemParametersInfoW lists each name with its number (OFFICIAL). Only the one answered
 * has a macro here; the names the guest uses and the run stops on are in fle_spi.c. */
#define FLE_SPI_GETWHEELSCROLLLINES 0x0068
/* Decision 0102 W13 (provisional): SPI_GETFONTSMOOTHINGCONTRAST and the page's default value. */
#define FLE_SPI_GETFONTSMOOTHINGCONTRAST 0x200C
#define FLE_SPI_FONTSMOOTHINGCONTRAST    1400

/* Decision 0103 (PROVISIONAL): SPI_GETFONTSMOOTHINGTYPE (0x200A), the Learn table: "Retrieves the type of font smoothing. The pvParam
 * parameter must point to a UINT that receives the information. The possible values are FE_FONTSMOOTHINGSTANDARD and
 * FE_FONTSMOOTHINGCLEARTYPE." The GET entry names no default; the SET entry (0x200B) says "The default is FE_FONTSMOOTHINGSTANDARD", which
 * is the value answered. The number is the header's (mingw-w64 winuser.h: FE_FONTSMOOTHINGSTANDARD 0x0001, FE_FONTSMOOTHINGCLEARTYPE 0x0002;
 * the Learn page text read does not give it). A real Windows 10 desktop may report ClearType (2): not checked, no real machine. */
#define FLE_SPI_GETFONTSMOOTHINGTYPE     0x200A
#define FLE_SPI_FONTSMOOTHING_STANDARD   1

/* Decision 0103 (PROVISIONAL, a value with NO default on the page): SPI_GETFONTSMOOTHINGORIENTATION (0x2012), the Learn table:
 * "Retrieves the font smoothing orientation. The pvParam parameter must point to a UINT that receives the information. The possible
 * values are FE_FONTSMOOTHINGORIENTATIONBGR (blue-green-red) and FE_FONTSMOOTHINGORIENTATIONRGB (red-green-blue)." Neither value is
 * named the default. FE_FONTSMOOTHINGORIENTATIONRGB (1) is answered: a choice made here, not read from a primary source (the numbers
 * are mingw-w64 winuser.h's: BGR 0x0000, RGB 0x0001). It is a user setting of a real machine that was not read. */
#define FLE_SPI_GETFONTSMOOTHINGORIENTATION 0x2012
#define FLE_SPI_FONTSMOOTHINGORIENTATION_RGB 1

/* Decision 0106 (PROVISIONAL, a value with NO default on the page): SPI_GETCURSORSHADOW (0x101A), the Learn table: "Determines whether the cursor has
 * a shadow around it. The pvParam parameter must point to a BOOL variable that receives TRUE if the shadow is enabled, FALSE if it is disabled. This
 * effect appears only if the system has a color depth of more than 256 colors." The SET entry (0x101B) names no default either. FALSE is answered: a
 * choice made here, not read from a primary source. It is a user setting of a real machine that was not read; the virtual display has 32 bits per pixel
 * (decision 0053), so the colour-depth condition of the page is met either way. */
#define FLE_SPI_GETCURSORSHADOW     0x101A
#define FLE_SPI_GETANIMATION        0x0048   /* decision 0126 wall 92 */
#define FLE_SPI_ANIMATIONINFO_SIZE  8        /* sizeof(ANIMATIONINFO) = UINT cbSize + int iMinAnimate */
#define FLE_SPI_SETANIMATION        0x0049   /* decision 0242: the engine's ShowWindow wrapper (va 0x8672a0) reads SPI_GETANIMATION and, when iMinAnimate is not 0, sets it to 0 around ShowWindow and back */
#define FLE_SPI_MINANIMATE          1        /* decision 0242 (S-pick1-spi-2, replaces S-spi-animation = 0): the initial iMinAnimate; Learn names no default, a Windows desktop and Wine 10.0 start with minimize / restore animation on; SPI_SETANIMATION changes it for this process */
#define FLE_SPI_CURSORSHADOW        0

/* Decision 0098 (PROVISIONAL): SPI_GETWORKAREA 0x0030 (the Learn table lists the number): "Retrieves the size of the work area on the
 * primary display monitor ... The pvParam parameter must point to a RECT structure that receives the coordinates of the work area". */
#define FLE_SPI_GETWORKAREA         0x0030

/* Decided value (decision 0058). Learn, SPI_GETWHEELSCROLLLINES: "The pvParam parameter must point to a UINT variable that
 * receives the number of lines. The default value is 3." (OFFICIAL.) */
#define FLE_SPI_WHEELSCROLLLINES    3

/* Decision 0066 (PROVISIONAL): the two font actions. The numbers are the Learn table's (OFFICIAL): SPI_GETICONTITLELOGFONT 0x001F
 * ("The uiParam parameter specifies the size of a LOGFONT structure, and the pvParam parameter must point to the LOGFONT
 * structure to fill in"), SPI_GETNONCLIENTMETRICS 0x0029 ("pvParam must point to a NONCLIENTMETRICS structure ... Set the cbSize
 * member of this structure and the uiParam parameter to sizeof(NONCLIENTMETRICS)"). */
#define FLE_SPI_GETICONTITLELOGFONT 0x001F
#define FLE_SPI_GETNONCLIENTMETRICS 0x0029

/* Sizes and offsets, computed from the Learn definitions of LOGFONTW (5 x 4 + 8 x 1 + 32 x 2 = 92) and NONCLIENTMETRICSW (OFFICIAL
 * types; the sums are ours). cbSize 0x1F4 is the structure without iPaddedBorderWidth ("4 bytes larger" with it). */
#define FLE_SPI_LOGFONTW_SIZE       0x5C
#define FLE_SPI_NCM_SIZE            0x1F8
#define FLE_SPI_NCM_SIZE_NO_PADDED  0x1F4
#define FLE_SPI_NCM_BORDERWIDTH     0x04
#define FLE_SPI_NCM_CAPTIONFONT     0x18
#define FLE_SPI_NCM_SMCAPTIONFONT   0x7C
#define FLE_SPI_NCM_MENUFONT        0xE0
#define FLE_SPI_NCM_STATUSFONT      0x13C
#define FLE_SPI_NCM_MESSAGEFONT     0x198
#define FLE_SPI_NCM_PADDEDBORDER    0x1F4

/* OFFICIAL values: NONCLIENTMETRICS "iBorderWidth ... The default is 1 pixel", "iPaddedBorderWidth ... The default value is 4
 * pixels". */
#define FLE_SPI_BORDERWIDTH         1
#define FLE_SPI_PADDEDBORDERWIDTH   4

/* The system font (decision 0066, PROVISIONAL; the table of the five fonts and the source of each field are in the decision). One
 * LOGFONTW for the icon-title, caption, status, menu and message fonts. lfHeight is from the Learn LOGFONT formula
 * -MulDiv(PointSize, LOGPIXELSY, 72) with the 96 DPI of decision 0053 (-12 for 9 pt). The 9 pt and the face are NOT in a Learn page for
 * Windows 10 (the Fonts page gives them for Windows Vista and 7); the other fields are choices (unofficial hints in the decision). */
#define FLE_SPI_FONT_POINTS         9
#define FLE_SPI_FONT_HEIGHT         (-(FLE_SPI_FONT_POINTS * FLE_GDC_LOGPIXELSY / 72))
#define FLE_SPI_FONT_WEIGHT         400     /* FW_NORMAL (Learn: "400 is normal") */
#define FLE_SPI_FONT_CHARSET        1       /* DEFAULT_CHARSET */
#define FLE_SPI_FONT_OUTPRECISION   0       /* OUT_DEFAULT_PRECIS */
#define FLE_SPI_FONT_CLIPPRECISION  0       /* CLIP_DEFAULT_PRECIS */
#define FLE_SPI_FONT_QUALITY        5       /* CLEARTYPE_QUALITY (decision 0067: was 0 in 0066; UNOFFICIAL, registry exports of real Windows) */
#define FLE_SPI_FONT_PITCHANDFAMILY 0       /* DEFAULT_PITCH | FF_DONTCARE (0066 kept by 0067: the same exports hold 0, not 0x22) */
#define FLE_SPI_FONT_FACE           "Segoe UI"

/* user32!SystemParametersInfoW: BOOL f(UINT uiAction, UINT uiParam, PVOID pvParam, UINT fWinIni). rcx, rdx, r8, r9 (the
 * low DWORD of rcx, rdx, r9 is the argument; r8 is a pointer). */
void fle_imp_SystemParametersInfoW(void);

#endif
