/* The virtual input locale identifier: user32!GetKeyboardLayout(0) (docs/decisions/0063-virtual-keyboard-layout.md; the page
 * read and the source of every number are listed there and in docs/investigations/getkeyboardlayout-caller.md). Same convention
 * as fle_imports.h: the argument in R[1] (ecx, idThread); the Win32 return value in R[0]; the guest return address stays on the
 * stack for the caller's ret. Stateless: the value is a function of fle_emulated_locale (fle_locale.h) and one constant, so
 * there is no thread rule.
 *
 * Nothing is read from the host: no keyboard, no keyboard layout, no input method, no X11 or Wayland setting, no registry. The
 * value is a value of this lift environment, fixed, so that a run does not depend on the machine it runs on.
 *
 * Only the form the guest uses is answered: idThread 0 (the current thread). Every other idThread stops the run through
 * fle_stop (fle_env.h): "fle: GetKeyboardLayout: <why>" and abort(). The last error is never touched. */
#ifndef FLE_KBD_H
#define FLE_KBD_H

#include <stdint.h>

/* The high word of the HKL (decision 0063, a PROVISIONAL choice). Learn (GetKeyboardLayout): "the high word contains a device
 * handle to the physical layout of the keyboard" (OFFICIAL, but it gives no value). 0x0409 is the convention of a real
 * Windows for the primary US English layout (UNOFFICIAL: third-party text only, not a Microsoft page; the PM's choice, B1).
 * It is a fixed constant, not derived from the LCID: when fle_emulated_locale.lcid changes, the low word follows it and this
 * word does not. The guest does not look inside this word (it stores the 64 bits, compares them with the elements of
 * GetKeyboardLayoutList, and uses the low 16 bits as a LANGID). */
#define FLE_KBD_HKL_HIGH_WORD 0x0409u

/* user32!GetKeyboardLayout: HKL f(DWORD idThread). ecx. Answers idThread == 0 only (the low DWORD of rcx is the argument). */
void fle_imp_GetKeyboardLayout(void);

/* Inspection (for tests and tools; pure). The HKL of the one virtual input locale: (FLE_KBD_HKL_HIGH_WORD << 16) |
 * (fle_emulated_locale.lcid & 0xFFFF), as a 64-bit handle value whose upper 32 bits are 0. */
uint64_t fle_kbd_hkl(void);

#endif
