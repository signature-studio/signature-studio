#ifndef APIW_H
#define APIW_H
#include <stdint.h>
enum { APIW_F_PROBE = 1, APIW_F_EVENT = 2 };
enum { APIW_D_NONE = 0, APIW_D_W1, APIW_D_A1, APIW_D_A2, APIW_D_W2, APIW_D_W23, APIW_D_W3, APIW_D_G1G4, APIW_D_IDX, APIW_D_DEVCAPS, APIW_D_SPI, APIW_D_PTR1U32, APIW_D_CREATEWIN, APIW_D_H1, APIW_D_SWP, APIW_D_SLWA, APIW_D_ULW, APIW_D_BITBLT, APIW_D_STRETCHBLT, APIW_D_CREATEDIB, APIW_D_SELOBJ, APIW_D_MARGINS, APIW_D_STK };
typedef struct { const char *name; uint32_t flags; uint32_t dec; uint64_t n, last; } apiw_t;
void apiw_pre(unsigned id);
void apiw_post(unsigned id);
void apiw_dump(void);
#endif
