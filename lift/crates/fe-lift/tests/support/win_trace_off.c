/* Test-only: the window-API trace is off (fle_win.h, fle_win_api_trace_on). A test that links runtime/fle_gdi.c, fle_input.c and the like without fle_win.c calls it without a
 * weak reference; this definition answers "off", the default of fle_win.c. Weak, so a link that also has fle_win.c keeps fle_win.c's definition. */
__attribute__((weak)) int fle_win_api_trace_on(void)
{
    return 0;
}
