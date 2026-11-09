/* Shared declarations of the vst3a harness (decision 0165, PROVISIONAL): the Serum 2 first run. */
#ifndef VST3A_H
#define VST3A_H

#include <stddef.h>
#include <stdint.h>
#include "fle_runtime.h"

/* Argument n (1-based, n >= 5) of the Windows x64 call: the guest return address is on the stack at R[4], then the 32-byte shadow space. */
static inline uint64_t vst3a_arg(unsigned n) { return fle_ld(R[4] + 8u + 8u * (n - 1u), 8); }

/* A named stop for an import that has no answer yet: prints "vst3a: UNANSWERED dll!API called ..." and exits with 97. */
void vst3a_unanswered(const char *dll, const char *api) __attribute__((noreturn));

/* A named stop of the harness's own (a state that is not modelled). */
void vst3a_stop(const char *what, const char *why) __attribute__((noreturn));

/* The counters of the answers: how often each answered API was called (printed at the end). */
void vst3a_note(const char *api);

/* IEventList of the host (vst_host3a.c) */
void vst_ev_clear(void);
void vst_ev_note(int on, int pitch, float vel, int sample_offset, int note_id);
int vst_ev_count(void);
unsigned long long vst_ev_added(void);

/* The ordinal imports (rows cannot name them: the extra-imports parser reads '#' as a comment; generated table in vst3a_stubs.c): slot patching and pseudo-VAs */
typedef struct { const char *dll; int ordinal; uint64_t va; void (*fn)(void); } vst3a_ord_entry;
extern const vst3a_ord_entry vst3a_ord_table[];
extern const unsigned vst3a_ord_count;

/* read-only IBStream over a memory copy of a file (vst_stream3a.c); its methods are dispatch-table rows (gen_vst3a.sh) */
uint64_t vst3a_stream_new(const void *data, size_t n);
void vst3a_stream_report(void);

void vst3a_cv_report(void);

/* host IMessage / IAttributeList (vst_msg3a.c) */
uint64_t vst3a_msg_try_create(uint64_t cid_p, uint64_t iid_p);
void vst3a_msg_report(void);

#endif
