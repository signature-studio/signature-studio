/* x87 arithmetic for fe-lift output (x87.rs): software extended precision (f80).
 * See docs/decisions/0033-x87.md.
 *
 * State: the 8 physical registers fle_x87_r[0..7] (sign, 15-bit exponent, 64-bit
 * significand, bit-exact 10-byte image), and TOP, the status word and the tag word in
 * fle_x87 (fle_x87ctl.h). ST(i) is physical register (TOP + i) & 7. The tag word is
 * kept up to date at every register write (00 valid, 01 zero, 10 special, 11 empty),
 * so it equals what FNSTENV reports on the host. MMX (fle_mmx.h) uses a separate MM[8]
 * and is not aliased to these registers.
 *
 * Fail-closed (named abort "fle: x87: ..."): a stack overflow or underflow, an
 * exception that the control word leaves unmasked (#MF is not modelled), a pending
 * unmasked flag at the start of an instruction, an x87 instruction while MMX state is
 * live (no EMMS yet) and transcendental/partial-remainder instructions that are not
 * implemented. Masked exceptions set the status word flags and give the SDM's masked
 * result. */
#ifndef FLE_X87_H
#define FLE_X87_H

#include <stdint.h>

typedef struct {
    uint64_t m;  /* significand, bit 63 is the integer bit */
    uint16_t se; /* sign (bit 15) and biased exponent */
} fle_f80;

extern fle_f80 fle_x87_r[8];

/* Set by fle_mmx.h on every MMX instruction, cleared by EMMS. */
extern uint8_t fle_x87_mmx_active;

enum { FLE_X87_ADD, FLE_X87_SUB, FLE_X87_SUBR, FLE_X87_MUL, FLE_X87_DIV, FLE_X87_DIVR };

/* FLD m32/m64/m80 (size 4, 8, 10). */
void fle_x87_fld_m(uint64_t a, unsigned size);
/* FILD m16/m32/m64 (size 2, 4, 8). */
void fle_x87_fild(uint64_t a, unsigned size);
/* FLD ST(i). */
void fle_x87_fld_st(unsigned i);
/* FLD1, FLDL2T, FLDL2E, FLDPI, FLDLG2, FLDLN2, FLDZ for k = 0..6. */
void fle_x87_fld_const(unsigned k);
/* FST/FSTP m32/m64/m80. */
void fle_x87_fst_m(uint64_t a, unsigned size, unsigned pop);
/* FST/FSTP ST(i). */
void fle_x87_fst_st(unsigned i, unsigned pop);
/* FIST/FISTP/FISTTP m16/m32/m64 (trunc: FISTTP, always truncates). */
void fle_x87_fist(uint64_t a, unsigned size, unsigned pop, unsigned trunc);
/* ST(dst) := ST(dst) op ST(src), then pop when pop != 0. op is FLE_X87_*. */
void fle_x87_arith_st(unsigned op, unsigned dst, unsigned src, unsigned pop);
/* ST(0) := ST(0) op m32fp/m64fp (size 4, 8). */
void fle_x87_arith_m(unsigned op, uint64_t a, unsigned size);
/* ST(0) := ST(0) op m16int/m32int (size 2, 4). */
void fle_x87_arith_i(unsigned op, uint64_t a, unsigned size);
/* FCOM/FCOMP/FCOMPP/FUCOM/FUCOMP/FUCOMPP: compares ST(0) with ST(i), sets C0, C2, C3. */
void fle_x87_com_st(unsigned i, unsigned pops, unsigned unordered);
/* FCOM/FCOMP m32/m64 (size 4, 8). */
void fle_x87_com_m(uint64_t a, unsigned size, unsigned pop);
/* FICOM/FICOMP m16/m32 (size 2, 4). */
void fle_x87_com_i(uint64_t a, unsigned size, unsigned pop);
/* FCOMI/FCOMIP/FUCOMI/FUCOMIP: returns bit 0 = CF, bit 1 = PF, bit 2 = ZF. */
unsigned fle_x87_comi(unsigned i, unsigned pop, unsigned unordered);
void fle_x87_ftst(void);
void fle_x87_fxam(void);
void fle_x87_fxch(unsigned i);
/* FCMOVcc ST(0), ST(i): copies when cond != 0. */
void fle_x87_fcmov(unsigned i, unsigned cond);
void fle_x87_fchs(void);
void fle_x87_fabs(void);
void fle_x87_fsqrt(void);
void fle_x87_frndint(void);
void fle_x87_fscale(void);
/* F2XM1: ST(0) := 2^ST(0) - 1, correctly rounded by RC (decision 0043). An operand outside
 * -1.0 .. +1.0 stops the run (the SDM leaves that result undefined). */
void fle_x87_f2xm1(void);
/* FSINCOS: ST(0) := sin, then push cos, correctly rounded by RC after reduction by the SDM's
 * 66-bit Pi. |x| >= 2^63: C2 = 1, ST(0) unchanged, no push (decision 0043). */
void fle_x87_fsincos(void);

#endif
