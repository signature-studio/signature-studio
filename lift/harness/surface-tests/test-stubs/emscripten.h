/* A stand-in for <emscripten.h>, ONLY for test_fruity_surface_layer.c (decision 0145): fruity_surface.c is compiled natively there, so EM_JS becomes an empty C function. */
#ifndef STUB_EMSCRIPTEN_H
#define STUB_EMSCRIPTEN_H
#define EM_JS(ret, name, params, ...) ret name params { }
#endif
