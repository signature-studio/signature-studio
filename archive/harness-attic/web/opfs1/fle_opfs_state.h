/* Stage 3 of decision 0216 (opfs1, PROVISIONAL), for act1's requirement R4 (the registry subtree, the licence files): small whole-file state kept in the OPFS, written so that a Worker killed at any moment leaves
 * either the old file or the new one. Built on fle_opfs.h; the files live below the root set by fle_opfs_set_root (the page's own folders are /fl/state and /fl/license).
 * fle_opfs_state_put: writes `path`.tmp, fsyncs it, closes it and moves it over `path` (the replace of FileSystemFileHandle.move, measured in Chrome 156 and Firefox 158). ASYNC.
 * fle_opfs_state_get: the whole file in a malloc'd buffer (*size = its size; the buffer is NUL-terminated for the caller's convenience, the NUL is not counted); NULL with *err = -ENOENT when there is no file. ASYNC. */
#ifndef FLE_OPFS_STATE_H
#define FLE_OPFS_STATE_H
#include <stddef.h>
int fle_opfs_state_put(const char *path, const void *data, size_t size);
void *fle_opfs_state_get(const char *path, size_t *size, int *err);
#endif
