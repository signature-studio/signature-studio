#ifndef ALPHA1_REC_H
#define ALPHA1_REC_H
#include <stdint.h>
enum { A1_F_PROBE = 1, A1_F_EVENT = 2 };
enum { A1_D_NONE = 0, A1_D_W1, A1_D_A1, A1_D_A2, A1_D_W2, A1_D_W23, A1_D_W3, A1_D_G1G4, A1_D_IDX, A1_D_DEVCAPS, A1_D_SPI, A1_D_PTR1U32, A1_D_CREATEWIN, A1_D_H1, A1_D_SWP, A1_D_SLWA, A1_D_ULW, A1_D_ULWI,
       A1_D_DWMBB, A1_D_MARGINS, A1_D_ALPHABLEND, A1_D_ANIMATE, A1_D_GRAD, A1_D_BPAINT, A1_D_BPALPHA, A1_D_DWMATTR, A1_D_WCA, A1_D_SETWL, A1_D_DIBSEC, A1_D_BITBLT, A1_D_STRBLT, A1_D_SDIBTD };
typedef struct { const char *name; uint32_t flags; uint32_t dec; uint64_t n, last; } a1_t;
void a1_pre(unsigned id);
void a1_post(unsigned id);
void a1_dump(void);
#endif
