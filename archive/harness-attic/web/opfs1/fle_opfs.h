/* Stage 3 of decision 0216 (opfs1, PROVISIONAL): the guest's files as OPFS files. The implementation is JavaScript (fle_opfs_lib.js, an emscripten library built with -sJSPI); this header is the C view.
 * Functions marked ASYNC suspend the caller through JSPI (they may only be called where the engine may suspend, as the file dialog's Show does). The others are synchronous.
 * Paths: UTF-8, '/'-separated, relative to the OPFS root or to the subtree set by fle_opfs_set_root. Return values: >= 0 or a negative errno (ENOENT 2, EIO 5, EBADF 9, EACCES 13, EBUSY 16, EEXIST 17, EISDIR 21,
 * EINVAL 22, ENOSPC 28, ENOTEMPTY 39). One FileSystemSyncAccessHandle is kept per path and shared by all descriptors of that path; the last close releases it (a sync handle is exclusive: while it is open the page's
 * file manager and other tabs get NoModificationAllowedError for that file). Writes are buffered (up to the block size) until flush/close/read/size changes; fle_opfs_size is the size the guest sees. */
#ifndef FLE_OPFS_H
#define FLE_OPFS_H
#include <stddef.h>
#include <stdint.h>

#define FLE_OPFS_READ 1
#define FLE_OPFS_WRITE 2
#define FLE_OPFS_CREATE 4
#define FLE_OPFS_TRUNC 8
#define FLE_OPFS_EXCL 16

int fle_opfs_set_root(const char *path);                              /* ASYNC */
int fle_opfs_open(const char *path, int flags);                       /* ASYNC; returns a descriptor >= 3 */
int fle_opfs_close(int fd);
int fle_opfs_read(int fd, void *buf, int n, double at);
int fle_opfs_write(int fd, const void *buf, int n, double at);
int fle_opfs_flush(int fd);
int fle_opfs_truncate(int fd, double size);
double fle_opfs_size(int fd);
int fle_opfs_stat(const char *path, double out[2]);                   /* ASYNC; 1 file, 2 directory, 0 missing; out = {size, mtime ms} */
char *fle_opfs_list(const char *path, int *err);                      /* ASYNC; malloc'd JSON [[name,kind,size,mtime],...] or NULL */
char *fle_opfs_list_names(const char *path, int *err);                /* ASYNC; malloc'd JSON [[name,kind],...] without sizes and dates (much cheaper than fle_opfs_list) */
int fle_opfs_mkdir(const char *path);                                 /* ASYNC; creates the folders on the way */
int fle_opfs_remove(const char *path);                                /* ASYNC; a file or an empty folder */
int fle_opfs_rename(const char *from, const char *to, int flags);     /* ASYNC; flags & 1: replace an existing destination, else EEXIST; EBUSY while the file is open; a.txt -> A.txt is allowed */
int fle_opfs_flush_all(void);                                         /* flushes every open file; returns how many had buffered writes (for a periodic call) */
void fle_opfs_set_fsync_on_close(int on);                             /* default off: the last close writes the buffered bytes and closes, without fsync (as CloseHandle); fle_opfs_flush / fle_opfs_flush_all fsync */
void fle_opfs_set_casefold(int on);                                   /* default on: a name that is not found is looked up again ignoring case (Windows names are case-insensitive, OPFS names are not) */
char *fle_opfs_stats(void);                                           /* malloc'd JSON of the cache counters (tests) */
void fle_opfs_set_block(int n);                                       /* the cache block size in bytes (tests; 0 turns the cache off) */
double fle_opfs_now(void);

#endif
