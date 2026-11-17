/* x87 FPU control state for fe-lift output. See fle_x87ctl.h. */
#include <stdint.h>

#include "fle_runtime.h"
#include "fle_x87ctl.h"

fle_x87_state fle_x87 = { 0x037F, 0x0000, 0xFFFF };

void fle_x87_store_cw(uint64_t a) {
    fle_st(a, 2, fle_x87.cw);
}

void fle_x87_load_cw(uint64_t a) {
    uint16_t v = (uint16_t)fle_ld(a, 2);
    fle_x87.cw = (uint16_t)((v & 0x1F3Fu) | 0x0040u);
}

void fle_x87_store_sw(uint64_t a) {
    fle_st(a, 2, fle_x87.sw);
}

uint16_t fle_x87_sw(void) {
    return fle_x87.sw;
}

void fle_x87_clex(void) {
    fle_x87.sw &= (uint16_t)~0x80FFu;
}

void fle_x87_init(void) {
    fle_x87.cw = 0x037F;
    fle_x87.sw = 0x0000;
    fle_x87.tw = 0xFFFF;
}
