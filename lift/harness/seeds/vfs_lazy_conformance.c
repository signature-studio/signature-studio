/* Conformance run of the lazy files against an index made by ANOTHER tool (tools/vfs-build.py of opt1, the format of decision 0314): the seed list with its "@lazy:" row is loaded, then every path of a list is read through the guest-facing
 * stubs (CreateFileW / GetFileSize / ReadFile, as the lifted code calls them) and compared with the original host file.
 *
 *   cd lift && clang -std=gnu11 -O2 -Wall -Wextra -I runtime -I harness runtime/fle_vfs.c runtime/fle_reg.c runtime/fle_env.c runtime/fle_imports.c runtime/fle_modules.c runtime/fle_string.c \
 *       harness/seeds/vfs_lazy_conformance.c -o vfs_lazy_conformance
 *   FLE_VFS_SEED=<seed list> FLE_VFS_SEED_ROOT=<dir that holds vfs/index.bin and the stream files> vfs_lazy_conformance <paths.tsv>
 *
 * paths.tsv: <guest path (UTF-8, C:\...)> TAB <original host file>. Prints the load time, the number of lazy files, then one line per mismatch and a summary; the exit code is the number of mismatches (or 99). */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "vfs_seed.h"

uint64_t R[16];

static double now_s(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec * 1e-9;
}

int main(int argc, char **argv)
{
    FILE *f;
    char line[8192];
    uint16_t w[VS_GUEST_MAX + 1];
    unsigned long n = 0, bad = 0, missing = 0;
    uint64_t lazy_bytes = 0, total = 0, lazy_before_reads, lazy_after_reads;
    double t0, t1;

    if (argc < 2) { fprintf(stderr, "usage: %s <paths.tsv>   (FLE_VFS_SEED and FLE_VFS_SEED_ROOT in the environment)\n", argv[0]); return 99; }
    t0 = now_s();
    if (vfs_seed_from_env() != 0) return 99;
    t1 = now_s();
    lazy_before_reads = fle_vfs_lazy_files(&lazy_bytes);
    printf("conformance: seed + lazy index loaded in %.3f s; lazy files %llu, %llu bytes\n", t1 - t0, (unsigned long long)lazy_before_reads, (unsigned long long)lazy_bytes);
    f = fopen(argv[1], "r");
    if (!f) { perror(argv[1]); return 99; }
    while (fgets(line, sizeof line, f)) {
        char *tab = strchr(line, '\t'), *nl;
        FILE *h;
        uint8_t *want = NULL, *got = NULL;
        long sz;
        uint32_t cnt = 0, size;
        uint64_t hd;
        size_t units;
        if (!tab) continue;
        *tab = 0;
        nl = strchr(tab + 1, '\n');
        if (nl) *nl = 0;
        if (vs_utf8_units(line, strlen(line), &units) != 0 || units > VS_GUEST_MAX) { printf("BAD PATH %s\n", line); bad++; continue; }
        vs_wide(w, line, strlen(line));
        h = fopen(tab + 1, "rb");
        if (!h) { missing++; continue; }
        fseek(h, 0, SEEK_END);
        sz = ftell(h);
        fseek(h, 0, SEEK_SET);
        want = malloc(sz ? (size_t)sz : 1);
        if (sz && fread(want, 1, (size_t)sz, h) != (size_t)sz) { printf("cannot read the original %s\n", tab + 1); fclose(h); free(want); bad++; continue; }
        fclose(h);
        n++;
        hd = VS_CALL(fle_imp_CreateFileW, VS_A(w), 0x80000000u, 1, 0, 3, 0x80, 0);
        if (hd == FLE_INVALID_HANDLE_VALUE) { printf("MISSING in the volume: %s\n", line); bad++; free(want); continue; }
        size = (uint32_t)VS_CALL(fle_imp_GetFileSize, hd, 0);
        if ((long)size != sz) { printf("SIZE %s: volume %u, original %ld\n", line, size, sz); bad++; VS_CALL(fle_imp_CloseHandle, hd); free(want); continue; }
        got = malloc(size ? size : 1);
        if (size && (VS_CALL(fle_imp_ReadFile, hd, VS_A(got), size, VS_A(&cnt), 0) != 1 || cnt != size)) { printf("READ FAILED %s\n", line); bad++; }
        else if (size && memcmp(got, want, size) != 0) { printf("BYTES DIFFER %s\n", line); bad++; }
        total += size;
        VS_CALL(fle_imp_CloseHandle, hd);
        free(got);
        free(want);
    }
    fclose(f);
    lazy_after_reads = fle_vfs_lazy_files(NULL);
    printf("conformance: %lu files compared (%llu bytes), %lu mismatches, %lu originals not found on the host; lazy files %llu -> %llu after the reads (reads must not materialize); read phase %.3f s\n", n, (unsigned long long)total, bad, missing,
           (unsigned long long)lazy_before_reads, (unsigned long long)lazy_after_reads, now_s() - t1);
    return bad > 99 ? 99 : (int)bad;
}
