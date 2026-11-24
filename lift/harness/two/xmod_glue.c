/* Issue #1342: the page loads a real module's separate wasm module (app/src/engine/xmod.ts). This is the fle_xmod_load that main_two.c calls: it asks the page through the import
 * fle_js_xmod_load (harness/web/fruity/fle_idle_lib.js; the page's hook Module.fleXmodLoad). It is defined here and not only weak in main_two.c: a weak undefined function is 0 in wasm,
 * so main_two.c would not call the page at all. Only the web link has this object (harness-inputs.txt; the native harness does not compile it). */
int fle_js_xmod_load(const char *dll_path);

int fle_xmod_load(const char *dll_path) { return fle_js_xmod_load(dll_path); }
