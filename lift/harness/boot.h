/* Shared declarations for the native harness. Not part of the generated output. */
#ifndef FLE_HARNESS_BOOT_H
#define FLE_HARNESS_BOOT_H

#include <stddef.h>
#include <stdint.h>

/* Run limits. The call limit stops deep or endless call chains. The wall-clock
 * limit (alarm) also stops loops that make no calls. The address-space limit
 * stops runaway allocation. */
#define HARNESS_MAX_CALLS     UINT64_C(20000000)
#define HARNESS_MAX_SECONDS   120u
#define HARNESS_MAX_AS_BYTES  (UINT64_C(4) << 30)
#define HARNESS_STACK_BYTES   (UINT64_C(16) << 20)
#define HARNESS_SENTINEL_RET  UINT64_C(0xdead0000)

typedef struct {
    uint64_t base;   /* ImageBase. The image is mapped at this exact VA. */
    uint64_t size;   /* SizeOfImage */
    uint64_t entry;  /* AddressOfEntryPoint as a VA */
    size_t nimports; /* IAT slots filled with pseudo-VA */
    size_t nexports; /* named exports */
} harness_image;

typedef struct {
    uint64_t va;
    const char *name;
} harness_sym;

int image_load(const char *path, harness_image *img, char *err, size_t errlen);
const char *image_import_name(uint64_t va);
const char *image_export_name(uint64_t va);

void trace_init(void);
void trace_report(const char *reason);

#endif
