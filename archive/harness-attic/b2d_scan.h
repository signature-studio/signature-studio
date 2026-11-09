/* X2 (docs/investigations/blend2d-nojit-slots.md): after DllMain has returned, call ONE entry (or one short sequence) of the guest's own Blend2D
 * Agg2D object, in a process of its own, and print the result. Compiled into the harness only by a build with B2D_SCAN=1
 * (lift/harness/two/build_two_native.sh and build_nmod_wasm.sh; a4-multi.sh does it for FLE_BLEND2D_SCAN=1); main_two.c calls these functions
 * under #ifdef FLE_B2D_SCAN. Portable: the same file is built for native and for wasm (no signal, no alarm, no backtrace, no gdb): a stop of the
 * guest (a name-stopped API, an unsupported instruction, a dispatch miss, a fault) ends the process through the runtime's own stop path.
 *
 * Run time: FLE_B2D_SCAN=<test>[:<arg>]   the test to run (see the table in b2d_scan.c; `list` prints the names and the slots they call and
 *                                         runs nothing else); unset or empty = nothing runs and the harness behaves as a build without it.
 *
 * Each test: AggCreateInstance, attach(buffer, 64, 64, stride 256) with the buffer filled with the pixel B=0x33 G=0x22 R=0x11 A=0xff (a 32-bit
 * PRGB32 word 0xff112233), the calls of the test, then lines that all start with "scan: ":
 *   scan: RESULT <test> status=done hash=<FNV-1a of the 16,384 bytes> changed=<pixels that differ from the start>
 *   scan: CHECK  <test> <label> PASS|FAIL <detail>      (a comparison with a value worked out by hand; the text says from what: SPEC or READING)
 *   scan: VALUE  <test> <label> <number>                (a returned value or a word of the object, no address)
 * Nothing that depends on an address is printed on a "RESULT", "CHECK" or "VALUE" line, so the lines of a native run and of a wasm run compare
 * byte for byte. */
#ifndef FLE_HARNESS_B2D_SCAN_H
#define FLE_HARNESS_B2D_SCAN_H

#include <stdint.h>

/* Before the guest runs: reads FLE_B2D_SCAN. 0 = go on (also when no test is asked for), -1 = refused (message on stderr). `list` prints the table and returns 1. */
int b2d_scan_precheck(void);
/* 1 when a test was asked for (valid only after b2d_scan_precheck returned 0). */
int b2d_scan_requested(void);
/* After DllMain returned and its report was printed. entry_sp = R[4] before the entry call. Returns the exit code of the harness (0 = the test returned). */
int b2d_scan_run(uint64_t entry_sp);

#endif
