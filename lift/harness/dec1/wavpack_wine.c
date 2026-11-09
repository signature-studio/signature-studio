/* Decision 0206 (dec1, opt-in test tool): the REAL wavpackdll_x64.dll (Image-Line's, from the installer) under Wine; same calls and same output lines as the wavpack mode of
 * main_dec1.c: WavpackOpenFileInput(path, err, OPEN_FILE_UTF8) -> NumSamples/NumChannels/SampleRate/BitsPerSample/Mode -> WavpackUnpackSamples in chunks of 4096*8/ch frames -> FNV-1a over the int32 samples.
 * Build: x86_64-w64-mingw32-gcc -O2 -o wavpack_wine.exe wavpack_wine.c      Run (wavpackdll_x64.dll next to the exe): wine wavpack_wine.exe FILE... */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
typedef void *(*open_t)(const char *, char *, int, int);
typedef uint32_t (*u32_t)(void *);
typedef int (*int_t)(void *);
typedef uint32_t (*unpack_t)(void *, int32_t *, uint32_t);
static uint64_t fnv(const void *p, size_t n, uint64_t h) { const unsigned char *b = p; for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 1099511628211ULL; } return h; }
int main(int argc, char **argv) {
    HMODULE h = LoadLibraryA(getenv("DEC1_WV") ? getenv("DEC1_WV") : "wavpackdll_x64.dll");
    if (!h) { fprintf(stderr, "no wavpackdll_x64.dll\n"); return 3; }
    open_t op = (open_t)GetProcAddress(h, "WavpackOpenFileInput");
    u32_t ns = (u32_t)GetProcAddress(h, "WavpackGetNumSamples"), sr = (u32_t)GetProcAddress(h, "WavpackGetSampleRate"), md = (u32_t)GetProcAddress(h, "WavpackGetMode"), ne = (u32_t)GetProcAddress(h, "WavpackGetNumErrors");
    int_t nc = (int_t)GetProcAddress(h, "WavpackGetNumChannels"), bp = (int_t)GetProcAddress(h, "WavpackGetBitsPerSample");
    unpack_t up = (unpack_t)GetProcAddress(h, "WavpackUnpackSamples");
    void (*cl)(void *) = (void (*)(void *))GetProcAddress(h, "WavpackCloseFile");
    static int32_t buf[4096 * 8];
    for (int a = 1; a < argc; a++) {
        char err[256]; err[0] = 0;
        void *w = op(argv[a], err, 0x80, 0);
        printf("file %s", argv[a]);
        if (!w) { printf(" open=NULL error=\"%s\"\n", err); continue; }
        uint32_t n = ns(w), r = sr(w), m = md(w); int ch = nc(w), bits = bp(w);
        printf(" samples=%u ch=%d rate=%u bits=%d mode=%#x", n, ch, r, bits, m);
        uint64_t hh = 14695981039346656037ULL, total = 0;
        if (ch > 0 && ch <= 8) {
            unsigned chunk = (unsigned)(sizeof buf / sizeof buf[0]) / (unsigned)ch;
            for (;;) { uint32_t got = up(w, buf, chunk); if (!got) break; hh = fnv(buf, (size_t)got * ch * 4, hh); total += got; }
        }
        printf(" decoded=%llu errors=%u all=%016llx\n", (unsigned long long)total, ne(w), (unsigned long long)hh);
        cl(w);
    }
    return 0;
}
