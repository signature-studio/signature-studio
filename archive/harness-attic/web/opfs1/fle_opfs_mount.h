/* Stage 3 of decision 0216 (opfs1, PROVISIONAL): which guest paths live in the browser's OPFS, and what the OPFS path is. Pure C, no OS and no browser: the table of mounts and the translation of a Windows path (UTF-16)
 * into the OPFS-relative path (UTF-8, '/'-separated) that fle_opfs.h (harness/web/opfs1/fle_opfs_lib.js) takes. NOT wired into fle_vfs.c yet (dec1 is working in the volume). It lives in harness/web/opfs1 and not in runtime/ ON PURPOSE: build_nmod_wasm.sh compiles every runtime/fle_*.c into the engine and many tests link them all; the files move to runtime/ with the wiring.
 * The rules are the Windows ones the engine relies on, written down once (docs/0216-opfs-files-s3.md section 3): case-insensitive prefix match on whole components; '/' and '\' both separate; '.' and '..' are resolved BEFORE the mount lookup (so a path cannot leave a mount: it
 * simply becomes another path, C:\a\..\b = C:\b); repeated separators collapse; the \\?\ prefix lifts the 259 limit (and there '.', '..', trailing dots and spaces are literal; '.' and '..' are refused because OPFS has no such name); a component ends in no space or dot (Win32 strips them); no < > : " | ? * or control character; at most 255 UTF-16 units a component; device
 * names (CON, PRN, AUX, NUL, COM1-9, LPT1-9, with or without an extension) are not files and are reported as such. OPFS itself is case-sensitive: the library (fle_opfs_lib.js) looks a missing name up case-insensitively. */
#ifndef FLE_OPFS_MOUNT_H
#define FLE_OPFS_MOUNT_H
#include <stddef.h>
#include <stdint.h>

#define FLE_OPFS_MAX_MOUNTS 16
#define FLE_OPFS_MAX_PATH 259u            /* MAX_PATH - 1 (UTF-16 units) without the \\?\ prefix */
#define FLE_OPFS_MAX_PATH_LONG 32767u     /* with the \\?\ prefix */
#define FLE_OPFS_MAX_COMPONENT 255u
#define FLE_OPFS_MAX_MOUNT_PREFIX 519u    /* a mount's guest folder is at most this long (UTF-16 units) */

/* results of fle_opfs_translate */
#define FLE_OPFS_NOT_MOUNTED 0            /* the path is not under any mount: the volume of fle_vfs.c handles it */
#define FLE_OPFS_MAPPED 1                 /* out = the OPFS-relative path ("" for the mount's root) */
#define FLE_OPFS_E_INVALID (-1)           /* bad character, a component that is only dots / spaces, lone surrogate, '.' or '..' in the \\?\ form (ERROR_INVALID_NAME / ERROR_BAD_PATHNAME) */
#define FLE_OPFS_E_TOO_LONG (-2)          /* over 259 UTF-16 units (no \\?\) / 32767, or a component over 255 (ERROR_FILENAME_EXCED_RANGE) */
#define FLE_OPFS_E_DEVICE (-3)            /* a device name (CON, NUL, COM1 ...): not a file of the OPFS */
#define FLE_OPFS_E_SPACE (-4)             /* the output buffer is too small */
#define FLE_OPFS_E_HIDDEN (-6)            /* the top-level OPFS folder "fl" (the page's own: /fl/state, /fl/license, the C: tree) under a mount of the OPFS root: to the guest it does not exist (ERROR_PATH_NOT_FOUND) */

void fle_opfs_mount_clear(void);
/* guest_prefix: an absolute Windows folder, e.g. C:\Users\user\Documents\Image-Line (UTF-16, with or without a trailing '\'); opfs_prefix: where it lives in the OPFS, e.g. "fl/C/Users/user/Documents/Image-Line" ('' = the OPFS root).
 * Returns 0, or a negative FLE_OPFS_E_* / -5 when the table is full. The longest matching prefix wins. */
int fle_opfs_mount_add(const uint16_t *guest_prefix, const char *opfs_prefix);
int fle_opfs_translate(const uint16_t *guest_path, size_t len, char *out, size_t cap);
/* the reverse, for a listing: the guest path of an OPFS-relative path under mount `index` (UTF-16, NUL-terminated); returns the length or a negative error */
int fle_opfs_mount_count(void);
int fle_opfs_guest_path(int index, const char *opfs_path, uint16_t *out, size_t cap);

#endif
