/* See fle_opfs_state.h. */
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include "fle_opfs.h"
#include "fle_opfs_state.h"

int fle_opfs_state_put(const char *path, const void *data, size_t size)
{
    char tmp[1024];
    int fd, r;
    if (strlen(path) + 5 > sizeof tmp) return -22;
    snprintf(tmp, sizeof tmp, "%s.tmp", path);
    fd = fle_opfs_open(tmp, FLE_OPFS_WRITE | FLE_OPFS_CREATE | FLE_OPFS_TRUNC);
    if (fd < 0) return fd;
    if (size > 0 && fle_opfs_write(fd, data, (int)size, 0) != (int)size) { fle_opfs_close(fd); fle_opfs_remove(tmp); return -5; }
    r = fle_opfs_flush(fd);
    if (r == 0) r = fle_opfs_close(fd); else fle_opfs_close(fd);
    if (r != 0) { fle_opfs_remove(tmp); return r; }
    r = fle_opfs_rename(tmp, path, 1);
    if (r != 0) fle_opfs_remove(tmp);
    return r;
}

void *fle_opfs_state_get(const char *path, size_t *size, int *err)
{
    int fd = fle_opfs_open(path, FLE_OPFS_READ), n;
    double sz;
    char *buf;
    *size = 0;
    if (fd < 0) { *err = fd; return NULL; }
    sz = fle_opfs_size(fd);
    buf = malloc((size_t)sz + 1);
    if (!buf) { fle_opfs_close(fd); *err = -12; return NULL; }
    n = sz > 0 ? fle_opfs_read(fd, buf, (int)sz, 0) : 0;
    fle_opfs_close(fd);
    if (n < 0 || (double)n != sz) { free(buf); *err = n < 0 ? n : -5; return NULL; }
    buf[n] = 0;
    *size = (size_t)n;
    *err = 0;
    return buf;
}
