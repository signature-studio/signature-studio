/* Heap probe of the wasm harness (decision 0134, PROVISIONAL; measurement only, HEAPPROBE=1 of build_nmod_wasm.sh): the JS side (Worker or node) can ask the C allocator how much of the linear memory is in use at a given
 * moment, e.g. from the print callback at the line "vfs seed: ..." (the seeding reads every host file into the heap before it writes them to the virtual volume). Nothing calls it; it changes no state.
 * fle_heap_probe(0) = bytes in allocated chunks (mallinfo uordblks), 1 = bytes in free chunks inside the heap (fordblks), 2 = bytes the allocator got from the system (arena), 3 = the top of the heap (sbrk(0)),
 * 4 = the size of the linear memory (emscripten_get_heap_size). Values over 4 GiB cannot occur in wasm32. */
#ifndef __EMSCRIPTEN__
#error "heap_probe.c is wasm only"
#endif
#include <emscripten/heap.h>
#include <malloc.h>
#include <stdint.h>
#include <unistd.h>

__attribute__((export_name("fle_heap_probe"))) uint32_t fle_heap_probe(int what)
{
    struct mallinfo mi = mallinfo();
    switch (what) {
    case 0: return (uint32_t)mi.uordblks;
    case 1: return (uint32_t)mi.fordblks;
    case 2: return (uint32_t)mi.arena;
    case 3: return (uint32_t)(uintptr_t)sbrk(0);
    case 4: return (uint32_t)emscripten_get_heap_size();
    default: return 0xFFFFFFFFu;
    }
}
