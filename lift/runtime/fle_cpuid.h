/* CPUID for fe-lift output. See runtime/fle_cpuid.c and docs/decisions/0009-cpuid.md.
 * Input: EAX in R[0], ECX in R[1]. Output: EAX in R[0], EBX in R[3], ECX in R[1], EDX in R[2]. */
#ifndef FLE_CPUID_H
#define FLE_CPUID_H

void fle_cpuid(void);

#endif
