/* user32 answers that need no window (docs/decisions/0030-apis-a9-a10.md). Every rule below is from the
 * Microsoft Learn page named in the comment; what a page leaves open stops the run through fle_stop (fle_env.h). */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_user_nowin.h"

/* ---- Case conversion and CharNextW ---- */
#define FLE_UNW_SINGLE_CHAR_LIMIT UINT64_C(0x10000)

/* Decision 0209 (crash1): the case table of the Unicode Character Database, file system rules (fle_crash1_lcmap.c; the same table as LCMapStringW, which Wine's CharUpperW / CharLowerW and LCMapStringW share). Weak: a link without that
 * file behaves as before (a unit of 0x80 or above stops by name). */
extern uint32_t fle_crash1_case(uint32_t cp, int upper, int linguistic) __attribute__((weak));

static uint16_t fle_unw_conv(uint16_t c, int up)
{
    if (c >= 0x80 && fle_crash1_case != NULL) return (uint16_t)fle_crash1_case(c, up, 0);
    if (up) return (c >= 'a' && c <= 'z') ? (uint16_t)(c - 32) : c;
    return (c >= 'A' && c <= 'Z') ? (uint16_t)(c + 32) : c;
}

/* Decision 0191 (PROVISIONAL; decision 0284: no longer opt-in; wall ia2-W25): a unit of the Private Use Area U+E000..U+F8FF is not changed (Microsoft's Windows 8 Upper Case Mapping Table lists none of them: see fle_locale.c). */
static void fle_unw_need_ascii(const char *api, uint16_t c)
{
    if (c >= 0xE000u && c <= 0xF8FFu) return;
    if (c >= 0x80 && fle_crash1_case == NULL) fle_stop(api, "UTF-16 unit of 0x80 or above: the page gives no case table for it");
}

/* MSDN CharLowerW, CharUpperW: a string is converted in place and the pointer is returned; when the high-order
 * word of the argument is zero the low-order word is one character and the converted character is returned (high
 * word zero). "There is no indication of success or failure." Mapping i <-> I is the same in every locale (the
 * pages' Turkish and Azerbaijani note), so ASCII needs no locale. The 64-bit reading of "high-order word is
 * zero" is: every bit above bit 15 is zero (inference). */
static void fle_unw_char(const char *api, int up)
{
    uint64_t p = R[1];

    if (p < FLE_UNW_SINGLE_CHAR_LIMIT) {
        uint16_t c = (uint16_t)p;
        fle_unw_need_ascii(api, c);
        R[0] = fle_unw_conv(c, up);
        return;
    }
    {
        uint64_t i;
        for (i = 0;; i++) {
            uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);
            if (c == 0) break;
            fle_unw_need_ascii(api, c);
        }
        for (i = 0;; i++) {
            uint16_t c = (uint16_t)fle_ld(p + 2u * i, 2);
            if (c == 0) break;
            fle_st(p + 2u * i, 2, fle_unw_conv(c, up));
        }
    }
    R[0] = p;
}

void fle_imp_CharLowerW(void) { fle_unw_char("CharLowerW", 0); }
void fle_imp_CharUpperW(void) { fle_unw_char("CharUpperW", 1); }

/* MSDN CharLowerBuffW, CharUpperBuffW: converts cchLength characters of lpsz, "even if one or more characters are
 * null characters"; returns the number of characters processed (cchLength). With cchLength 0 no character is
 * examined and 0 is returned (inference from "examines the number of characters indicated by cchLength"). A NULL
 * buffer with a positive length is not described, so it stops. */
static void fle_unw_buf(const char *api, int up)
{
    uint64_t p = R[1];
    uint32_t n = (uint32_t)R[2], i;

    if (n == 0) { R[0] = 0; return; }
    if (p == 0) fle_stop(api, "lpsz is NULL with cchLength > 0 (not described by the page)");
    for (i = 0; i < n; i++) fle_unw_need_ascii(api, (uint16_t)fle_ld(p + 2u * i, 2));
    for (i = 0; i < n; i++) fle_st(p + 2u * i, 2, fle_unw_conv((uint16_t)fle_ld(p + 2u * i, 2), up));
    R[0] = n;
}

void fle_imp_CharLowerBuffW(void) { fle_unw_buf("CharLowerBuffW", 0); }
void fle_imp_CharUpperBuffW(void) { fle_unw_buf("CharUpperBuffW", 1); }

/* MSDN CharNextW: a pointer to the next character, or to the terminating NUL at the end of the string; a pointer
 * to the NUL is returned unchanged. The page says a base letter followed by combining marks is one character
 * ("U+0061 U+030A advances two code points"), so the answer for p[0] depends on p[1]. Only a pair that cannot be
 * such a cluster is answered: p[0] a non-NUL ASCII unit, p[1] NUL or ASCII. A unit of 0x80 or above (combining
 * marks, surrogates) needs the Unicode tables the page does not give, and CR LF is not described as one unit or
 * two, so those stop. */
void fle_imp_CharNextW(void)
{
    static const char *api = "CharNextW";
    uint64_t p = R[1];
    uint16_t c0, c1;

    if (p == 0) fle_stop(api, "lpsz is NULL (not described by the page)");
    c0 = (uint16_t)fle_ld(p, 2);
    if (c0 == 0) { R[0] = p; return; }
    if (c0 >= 0x80) fle_stop(api, "character of 0x80 or above: grapheme rules (combining marks, surrogates) are not given by the page");
    c1 = (uint16_t)fle_ld(p + 2, 2);
    if (c1 >= 0x80) fle_stop(api, "next unit of 0x80 or above could be a combining mark: not decidable from the page");
    if (c0 == 0x0D && c1 == 0x0A) fle_stop(api, "CR LF: the page does not say whether it is one character");
    R[0] = p + 2;
}

/* ---- RECT (MSDN RECT: four LONG: left, top, right, bottom) ---- */
#define FLE_UNW_RECT_SIZE 16u

typedef struct { int32_t l, t, r, b; } fle_unw_rect;

static uint64_t fle_unw_need(const char *api, uint64_t p, const char *what)
{
    if (p == 0) fle_stop(api, what);
    return p;
}

static fle_unw_rect fle_unw_get(uint64_t p)
{
    fle_unw_rect q;
    q.l = (int32_t)(uint32_t)fle_ld(p, 4);
    q.t = (int32_t)(uint32_t)fle_ld(p + 4, 4);
    q.r = (int32_t)(uint32_t)fle_ld(p + 8, 4);
    q.b = (int32_t)(uint32_t)fle_ld(p + 12, 4);
    return q;
}

static void fle_unw_put(uint64_t p, fle_unw_rect q)
{
    fle_st(p, 4, (uint32_t)q.l);
    fle_st(p + 4, 4, (uint32_t)q.t);
    fle_st(p + 8, 4, (uint32_t)q.r);
    fle_st(p + 12, 4, (uint32_t)q.b);
}

/* MSDN IsRectEmpty: empty means right <= left or bottom <= top. */
static int fle_unw_empty(fle_unw_rect q) { return q.r <= q.l || q.b <= q.t; }

/* MSDN SetRect: the four members are assigned; nonzero on success. A NULL pointer is not described: stop. */
void fle_imp_SetRect(void)
{
    uint64_t p = fle_unw_need("SetRect", R[1], "lprc is NULL (not described by the page)");
    fle_unw_rect q;

    q.l = (int32_t)(uint32_t)R[2];
    q.t = (int32_t)(uint32_t)R[8];
    q.r = (int32_t)(uint32_t)R[9];
    q.b = (int32_t)(uint32_t)fle_arg(5);
    fle_unw_put(p, q);
    R[0] = 1;
}

/* MSDN IntersectRect: lprcDst "cannot be NULL"; the intersection goes to lprcDst; when the rectangles do not
 * intersect "an empty rectangle (all coordinates set to zero)" is stored and the result is zero; else nonzero.
 * The intersection is max of the lefts and tops, min of the rights and bottoms; it has an area only when both
 * sources have one, so an empty source gives the empty result (no case is left to the page). Both sources are read
 * before dst is written (the closure passes dst == source). */
void fle_imp_IntersectRect(void)
{
    static const char *api = "IntersectRect";
    uint64_t pd = fle_unw_need(api, R[1], "lprcDst is NULL (the page: cannot be NULL)");
    uint64_t pa = fle_unw_need(api, R[2], "lprcSrc1 is NULL (not described by the page)");
    uint64_t pb = fle_unw_need(api, R[8], "lprcSrc2 is NULL (not described by the page)");
    fle_unw_rect a = fle_unw_get(pa), b = fle_unw_get(pb), q;

    q.l = a.l > b.l ? a.l : b.l;
    q.t = a.t > b.t ? a.t : b.t;
    q.r = a.r < b.r ? a.r : b.r;
    q.b = a.b < b.b ? a.b : b.b;
    if (fle_unw_empty(q)) {
        q.l = q.t = q.r = q.b = 0;
        fle_unw_put(pd, q);
        R[0] = 0;
        return;
    }
    fle_unw_put(pd, q);
    R[0] = 1;
}

/* MSDN UnionRect: the smallest rectangle containing both sources; "the system ignores the dimensions of an empty
 * rectangle"; nonzero when the result is nonempty. One empty source: the result is the other source. Both empty:
 * the page does not say what is stored: (0, 0, 0, 0) and zero (decision 0144, S-g8a-unionrect-empty; it stopped before). */
void fle_imp_UnionRect(void)
{
    static const char *api = "UnionRect";
    uint64_t pd = fle_unw_need(api, R[1], "lprcDst is NULL (not described by the page)");
    uint64_t pa = fle_unw_need(api, R[2], "lprcSrc1 is NULL (not described by the page)");
    uint64_t pb = fle_unw_need(api, R[8], "lprcSrc2 is NULL (not described by the page)");
    fle_unw_rect a = fle_unw_get(pa), b = fle_unw_get(pb), q;
    int ea = fle_unw_empty(a), eb = fle_unw_empty(b);

    if (ea && eb) {
        /* Decision 0144 (wall 2, S-g8a-unionrect-empty, a choice without a source in the Learn page: "If the specified structure does not contain a nonempty rectangle, the return value is zero" and
         * nothing about lprcDst): the empty rectangle (0, 0, 0, 0), the same value IntersectRect above stores for an empty result, and zero. */
        q.l = q.t = q.r = q.b = 0;
        fle_unw_put(pd, q);
        R[0] = 0;
        return;
    }
    if (ea) q = b;
    else if (eb) q = a;
    else {
        q.l = a.l < b.l ? a.l : b.l;
        q.t = a.t < b.t ? a.t : b.t;
        q.r = a.r > b.r ? a.r : b.r;
        q.b = a.b > b.b ? a.b : b.b;
    }
    fle_unw_put(pd, q);
    R[0] = 1;
}

/* MSDN OffsetRect: the rectangle moves by dx and dy; nonzero on success. The page names no failing case and says
 * nothing about a coordinate that leaves the 32-bit range, so a NULL pointer and an overflow stop. */
void fle_imp_OffsetRect(void)
{
    static const char *api = "OffsetRect";
    uint64_t p = fle_unw_need(api, R[1], "lprc is NULL (not described by the page)");
    int32_t dx = (int32_t)(uint32_t)R[2], dy = (int32_t)(uint32_t)R[8];
    fle_unw_rect q = fle_unw_get(p), o;

    if (__builtin_add_overflow(q.l, dx, &o.l) || __builtin_add_overflow(q.r, dx, &o.r) ||
        __builtin_add_overflow(q.t, dy, &o.t) || __builtin_add_overflow(q.b, dy, &o.b))
        fle_stop(api, "a coordinate leaves the 32-bit range (not described by the page)");
    fle_unw_put(p, o);
    R[0] = 1;
}

/* user32!InflateRect: BOOL f(LPRECT lprc rcx, int dx edx, int dy r8d) (decision 0126, PROVISIONAL, wall 98). Learn: "adds -dx units to the left end and dx to the right end of the rectangle and -dy units to the top and dy to the
 * bottom. The dx and dy parameters are signed values"; nonzero on success, zero on failure (no failing case is named). A NULL lprc and a coordinate that leaves the 32-bit range stop (the page says neither). A rectangle that
 * becomes empty or inverted is stored as the arithmetic gives it (the page says nothing else: a reading). */
void fle_imp_InflateRect(void)
{
    static const char *api = "InflateRect";
    uint64_t p = fle_unw_need(api, R[1], "lprc is NULL (not described by the page)");
    int32_t dx = (int32_t)(uint32_t)R[2], dy = (int32_t)(uint32_t)R[8];
    fle_unw_rect q = fle_unw_get(p), o;

    if (__builtin_sub_overflow(q.l, dx, &o.l) || __builtin_add_overflow(q.r, dx, &o.r) || __builtin_sub_overflow(q.t, dy, &o.t) || __builtin_add_overflow(q.b, dy, &o.b))
        fle_stop(api, "a coordinate leaves the 32-bit range (not described by the page)");
    fle_unw_put(p, o);
    R[0] = 1;
}

/* user32!EqualRect: BOOL f(const RECT *lprc1 rcx, const RECT *lprc2 rdx). Learn: nonzero when "the two rectangles are identical" by "comparing the coordinates of their upper-left and lower-right corners"; "does not treat empty
 * rectangles as equal if their coordinates are different". A NULL pointer stops (not described). */
void fle_imp_EqualRect(void)
{
    static const char *api = "EqualRect";
    uint64_t pa = fle_unw_need(api, R[1], "lprc1 is NULL (not described by the page)");
    uint64_t pb = fle_unw_need(api, R[2], "lprc2 is NULL (not described by the page)");
    fle_unw_rect a = fle_unw_get(pa), b = fle_unw_get(pb);

    R[0] = a.l == b.l && a.t == b.t && a.r == b.r && a.b == b.b;
}

/* user32!PtInRect: BOOL f(const RECT *lprc rcx, POINT pt rdx). POINT (two LONGs, 8 bytes) is passed by value in rdx (x in the low dword, y in the high dword: the x64 convention for an 8-byte aggregate). Learn: "A point is within
 * a rectangle if it lies on the left or top side or is within all four sides. A point on the right or bottom side is considered outside the rectangle"; "If the rectangle is not normalized, a point is never considered inside of the
 * rectangle" (right > left and bottom > top are required). A NULL lprc stops (not described). */
void fle_imp_PtInRect(void)
{
    static const char *api = "PtInRect";
    uint64_t p = fle_unw_need(api, R[1], "lprc is NULL (not described by the page)");
    fle_unw_rect q = fle_unw_get(p);
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)(R[2] >> 32);

    R[0] = (q.r > q.l && q.b > q.t && x >= q.l && x < q.r && y >= q.t && y < q.b) ? 1u : 0u;
}

/* user32!SubtractRect: BOOL f(LPRECT lprcDst rcx, const RECT *lprcSrc1 rdx, const RECT *lprcSrc2 r8). Learn: "the resulting rectangle is the bounding box of the geometric difference"; it subtracts "only ... when the rectangles
 * intersect completely in either the x- or y-direction": (10,10,100,100) minus (50,50,150,150) is (10,10,100,100); (10,10,100,100) minus (50,10,150,150) is (10,10,50,100). Return value: zero when the resulting rectangle is empty,
 * nonzero otherwise. Reading: src2 is clipped to src1 first; when the clipped src2 covers src1 completely in y (top <= src1.top and bottom >= src1.bottom) the x range of src1 loses the covered part if the covered part touches an
 * edge (left <= src1.left: the left edge moves to src2.right; right >= src1.right: the right edge moves to src2.left; both: the result is empty); the same with the roles of x and y exchanged; otherwise (and when src2 does not
 * intersect src1) the result is src1. An empty src1 or src2 gives src1 (a reading: nothing to subtract). A result that is empty is stored as the arithmetic gives it, with the return value 0 (the page names no stored value).
 * S-subtractrect-reading: the rule for a covered strip in the middle (the difference is not a rectangle: the bounding box is src1) is the page's "bounding box of the geometric difference"; the case of both edges covered gives the
 * empty rectangle (left = right of the clipped bounds), a reading. NULL pointers stop. */
void fle_imp_SubtractRect(void)
{
    static const char *api = "SubtractRect";
    uint64_t pd = fle_unw_need(api, R[1], "lprcDst is NULL (not described by the page)");
    uint64_t pa = fle_unw_need(api, R[2], "lprcSrc1 is NULL (not described by the page)");
    uint64_t pb = fle_unw_need(api, R[8], "lprcSrc2 is NULL (not described by the page)");
    fle_unw_rect a = fle_unw_get(pa), b = fle_unw_get(pb), c, q = a;

    if (!fle_unw_empty(a) && !fle_unw_empty(b)) {
        c.l = a.l > b.l ? a.l : b.l;
        c.t = a.t > b.t ? a.t : b.t;
        c.r = a.r < b.r ? a.r : b.r;
        c.b = a.b < b.b ? a.b : b.b;
        if (!fle_unw_empty(c)) {
            if (c.t <= a.t && c.b >= a.b) {          /* covers a completely in y: a strip of the x range may go */
                if (c.l <= a.l && c.r >= a.r) q.r = q.l;
                else if (c.l <= a.l) q.l = c.r;
                else if (c.r >= a.r) q.r = c.l;
            } else if (c.l <= a.l && c.r >= a.r) {   /* covers a completely in x: a strip of the y range may go */
                if (c.t <= a.t && c.b >= a.b) q.b = q.t;
                else if (c.t <= a.t) q.t = c.b;
                else if (c.b >= a.b) q.b = c.t;
            }
        }
    }
    fle_unw_put(pd, q);
    R[0] = fle_unw_empty(q) ? 0u : 1u;
}

/* ---- Registered window messages and clipboard formats ----
 * One counter hands out the values, so a message and a clipboard format never share a value. Each API keeps its
 * own name table: the pages do not say whether the two share the name space (setting of this environment). */
#define FLE_UNW_KIND_MESSAGE   0u
#define FLE_UNW_KIND_CLIPBOARD 1u

typedef struct {
    uint16_t *name;
    uint16_t len;
    uint8_t kind;
} fle_unw_reg;

static fle_unw_reg *fle_unw_regs;
static uint32_t fle_unw_nregs;

static uint16_t *fle_unw_read_name(const char *api, uint64_t p, uint16_t *len)
{
    uint16_t buf[FLE_UNW_NAME_MAX + 1];
    uint16_t *copy;
    size_t n, i;

    if (p == 0) fle_stop(api, "name is NULL (not described by the page)");
    n = fle_w_read(api, p, buf, FLE_UNW_NAME_MAX + 1);
    if (n == 0) fle_stop(api, "empty name (not described by the page)");
    for (i = 0; i < n; i++)
        if (buf[i] >= 0x80) fle_stop(api, "name has a unit of 0x80 or above: its case folding is not given by the pages");
    copy = malloc((n + 1) * sizeof *copy);
    if (!copy) fle_stop(api, "out of memory");
    memcpy(copy, buf, (n + 1) * sizeof *copy);
    *len = (uint16_t)n;
    return copy;
}

static uint32_t fle_unw_register(const char *api, unsigned kind)
{
    uint16_t len;
    uint16_t *name = fle_unw_read_name(api, R[1], &len);
    uint32_t i;

    for (i = 0; i < fle_unw_nregs; i++) {
        fle_unw_reg *e = &fle_unw_regs[i];
        if (e->kind != kind || fle_w_icmp(e->name, e->len, name, len) != 0) continue;
        /* RegisterClipboardFormat: "the format name comparison is case-insensitive". RegisterWindowMessage: the
         * page says only "the same message string"; a name equal only up to case is not decided by it. */
        if (kind == FLE_UNW_KIND_MESSAGE && memcmp(e->name, name, (size_t)len * sizeof *name) != 0)
            fle_stop(api, "a name equal to a registered one except for case: the page does not say if it is the same message");
        free(name);
        return FLE_UNW_REG_FIRST + i;
    }
    if (fle_unw_nregs >= FLE_UNW_REG_MAX) fle_stop(api, "all identifiers 0xC000 through 0xFFFF are in use (the page names no error code)");
    if (!fle_unw_regs) {
        fle_unw_regs = calloc(FLE_UNW_REG_MAX, sizeof *fle_unw_regs);
        if (!fle_unw_regs) fle_stop(api, "out of memory");
    }
    fle_unw_regs[fle_unw_nregs].name = name;
    fle_unw_regs[fle_unw_nregs].len = len;
    fle_unw_regs[fle_unw_nregs].kind = (uint8_t)kind;
    return FLE_UNW_REG_FIRST + fle_unw_nregs++;
}

/* MSDN RegisterWindowMessageW: a message identifier in 0xC000 through 0xFFFF; the same string gives the same value
 * (the page: "If two different applications register the same message string, the applications return the same
 * message value"). */
void fle_imp_RegisterWindowMessageW(void) { R[0] = fle_unw_register("RegisterWindowMessageW", FLE_UNW_KIND_MESSAGE); }

/* MSDN RegisterClipboardFormatW: values 0xC000 through 0xFFFF; an existing name (compared without case) returns
 * the existing identifier. */
void fle_imp_RegisterClipboardFormatW(void) { R[0] = fle_unw_register("RegisterClipboardFormatW", FLE_UNW_KIND_CLIPBOARD); }

/* MSDN GetClipboardFormatNameW: the length of the name copied; zero when the format does not exist or is
 * predefined (the last-error code is not named by the page: ERROR_INVALID_PARAMETER is a runtime choice, an
 * "assumption"). The page says an over-long name is truncated but not whether cchMaxCount counts the NUL, so only
 * a name that fits under both readings (length + 1 <= cchMaxCount) is answered. A value that this runtime handed
 * out through RegisterWindowMessageW is not described as a clipboard format: it stops. */
void fle_imp_GetClipboardFormatNameW(void)
{
    static const char *api = "GetClipboardFormatNameW";
    uint32_t fmt = (uint32_t)R[1];
    uint64_t out = R[2];
    int32_t cch = (int32_t)(uint32_t)R[8];

    if (out == 0) fle_stop(api, "lpszFormatName is NULL (not described by the page)");
    if (cch <= 0) fle_stop(api, "cchMaxCount is not positive (not described by the page)");
    if (fmt >= FLE_UNW_REG_FIRST && fmt <= FLE_UNW_REG_LAST && fmt - FLE_UNW_REG_FIRST < fle_unw_nregs) {
        fle_unw_reg *e = &fle_unw_regs[fmt - FLE_UNW_REG_FIRST];

        if (e->kind != FLE_UNW_KIND_CLIPBOARD)
            fle_stop(api, "the value is a registered window message: the page does not say it is visible as a clipboard format");
        if ((uint32_t)e->len + 1u > (uint32_t)cch)
            fle_stop(api, "the name does not fit: the page does not say if cchMaxCount counts the terminating NUL");
        fle_w_write(out, e->name, (size_t)e->len + 1);
        R[0] = e->len;
        return;
    }
    fle_set_last_error(FLE_W32_ERR_INVALID_PARAMETER);
    R[0] = 0;
}
