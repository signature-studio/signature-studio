#!/usr/bin/env python3
"""alpha1 (decision 0244; forked from alpha1 gen-apiwrap.py): generate the link-time wrappers of every runtime import for the native build.
usage: gen-apiwrap.py NM_FILE OUT.c OUT.ldflags
NM_FILE = `nm --defined-only` of the runtime objects (the names fle_imp_*). One wrapper per name: a1_pre / __real_ / a1_post. The probe and event lists below are the ONLY choice made here;
they decide which calls keep their arguments (counts are kept for every call)."""
import re, sys
names = sorted({m.group(1) for ln in open(sys.argv[1]) for m in [re.search(r'\sT\s+(fle_imp_\w+)$', ln)] if m})
PROBE = {  # name -> decoder
 'LoadLibraryW': 'W1', 'LoadLibraryA': 'A1', 'LoadLibraryExW': 'W1', 'LoadLibraryExA': 'A1', 'GetModuleHandleW': 'W1', 'GetModuleHandleA': 'A1', 'GetModuleHandleExW': 'W3',
 'GetProcAddress': 'A2', 'CoCreateInstance': 'G1G4', 'CoCreateInstanceEx': 'G1G4',
 'RegOpenKeyExW': 'W2', 'RegOpenKeyExA': 'NONE', 'RegQueryValueExW': 'W2', 'RegCreateKeyExW': 'W2', 'RegGetValueW': 'W23', 'RegEnumKeyExW': 'NONE', 'RegQueryValueExA': 'NONE',
 'SystemParametersInfoW': 'SPI', 'SystemParametersInfoA': 'SPI', 'GetSystemMetrics': 'IDX', 'GetSystemMetricsForDpi': 'IDX', 'GetDeviceCaps': 'DEVCAPS',
 'DwmIsCompositionEnabled': 'PTR1U32', 'DwmGetColorizationColor': 'NONE', 'DwmGetWindowAttribute': 'NONE', 'DwmSetWindowAttribute': 'NONE', 'DwmEnableBlurBehindWindow': 'NONE',
 'DwmExtendFrameIntoClientArea': 'NONE', 'DwmEnableComposition': 'NONE', 'DwmFlush': 'NONE',
 'IsThemeActive': 'NONE', 'IsAppThemed': 'NONE', 'OpenThemeData': 'W2', 'GetThemeAppProperties': 'NONE', 'SetWindowTheme': 'NONE', 'BufferedPaintInit': 'NONE', 'BufferedPaintUnInit': 'NONE',
 'BeginBufferedPaint': 'NONE', 'EndBufferedPaint': 'NONE', 'BufferedPaintSetAlpha': 'NONE', 'GetBufferedPaintBits': 'NONE', 'DrawThemeParentBackground': 'NONE',
 'IsProcessDPIAware': 'NONE', 'SetProcessDPIAware': 'NONE', 'SetProcessDpiAwareness': 'NONE', 'SetProcessDpiAwarenessContext': 'NONE', 'GetDpiForMonitor': 'NONE', 'GetDpiForWindow': 'NONE', 'GetDpiForSystem': 'NONE',
 'GetAwarenessFromDpiAwarenessContext': 'NONE', 'GetThreadDpiAwarenessContext': 'NONE', 'SetThreadDpiAwarenessContext': 'NONE', 'AreDpiAwarenessContextsEqual': 'NONE',
 'GetVersionExW': 'NONE', 'GetVersionExA': 'NONE', 'RtlGetVersion': 'NONE', 'VerifyVersionInfoW': 'NONE', 'IsWindows8OrGreater': 'NONE',
 'EnumDisplayMonitors': 'NONE', 'GetMonitorInfoW': 'NONE', 'MonitorFromWindow': 'NONE', 'MonitorFromPoint': 'NONE', 'EnumDisplaySettingsW': 'NONE', 'EnumDisplayDevicesW': 'NONE',
 'GetKeyboardLayout': 'NONE', 'GetACP': 'NONE', 'IsDebuggerPresent': 'NONE', 'GetSystemInfo': 'NONE', 'GetNativeSystemInfo': 'NONE', 'IsWow64Process': 'NONE', 'IsProcessorFeaturePresent': 'IDX',
 'GetStockObject': 'IDX', 'CreateCompatibleBitmap': 'NONE', 'SetProcessWorkingSetSize': 'NONE', 'GetSysColor': 'IDX', 'GetSysColorBrush': 'IDX',
 'SetWindowCompositionAttribute': 'NONE', 'CreateDXGIFactory': 'NONE', 'D3D11CreateDevice': 'NONE', 'D2D1CreateFactory': 'NONE', 'DWriteCreateFactory': 'NONE',
 'CoInitializeEx': 'NONE', 'CoInitialize': 'NONE', 'OleInitialize': 'NONE', 'CoGetClassObject': 'G1G4', 'CLSIDFromProgID': 'W1', 'GetUserDefaultUILanguage': 'NONE',
 'WICConvertBitmapSource': 'NONE', 'GetFontData': 'NONE', 'EnumFontFamiliesExW': 'NONE', 'CreateFontIndirectW': 'NONE', 'GetTextMetricsW': 'NONE',
 'SHGetKnownFolderPath': 'NONE', 'SHGetFolderPathW': 'NONE', 'GetEnvironmentVariableW': 'W1', 'ExpandEnvironmentStringsW': 'W1',
}
EVENT = {
 'CreateWindowExW': 'CREATEWIN', 'CreateWindowExA': 'CREATEWIN', 'DestroyWindow': 'H1', 'ShowWindow': 'H1', 'SetWindowPos': 'SWP', 'MoveWindow': 'H1', 'SetParent': 'H1', 'EnableWindow': 'H1',
 'UpdateLayeredWindow': 'ULW', 'UpdateLayeredWindowIndirect': 'ULWI', 'SetLayeredWindowAttributes': 'SLWA', 'GetLayeredWindowAttributes': 'H1', 'AnimateWindow': 'ANIMATE', 'SetForegroundWindow': 'H1', 'SetActiveWindow': 'H1',
 'DwmEnableBlurBehindWindow': 'DWMBB', 'DwmExtendFrameIntoClientArea': 'MARGINS', 'DwmSetWindowAttribute': 'DWMATTR', 'SetWindowCompositionAttribute': 'WCA',
 'AlphaBlend': 'ALPHABLEND', 'GradientFill': 'GRAD', 'TransparentBlt': 'H1', 'BeginBufferedPaint': 'BPAINT', 'EndBufferedPaint': 'H1', 'BufferedPaintSetAlpha': 'BPALPHA', 'GetBufferedPaintBits': 'H1',
 'BufferedPaintInit': 'H1', 'BufferedPaintUnInit': 'H1', 'DrawThemeParentBackground': 'H1', 'DrawThemeTextEx': 'H1', 'DrawThemeBackground': 'H1', 'DrawThemeBackgroundEx': 'H1',
 'SetWindowRgn': 'H1', 'SetWindowLongW': 'SETWL', 'SetWindowLongPtrW': 'SETWL', 'SetClassLongPtrW': 'SETWL', 'SetClassLongW': 'SETWL', 'ReleaseCapture': 'H1', 'SetCapture': 'H1',
 'CreateDIBSection': 'DIBSEC', 'BitBlt': 'BITBLT', 'StretchBlt': 'STRBLT', 'SetDIBitsToDevice': 'SDIBTD', 'StretchDIBits': 'SDIBTD', 'MaskBlt': 'BITBLT', 'DwmFlush': 'H1', 'DwmGetColorizationColor': 'H1', 'DwmIsCompositionEnabled': 'PTR1U32',
}
DEC = {'NONE': 'A1_D_H1'}
dec_c = lambda d: {'NONE': 'A1_D_H1', 'H1': 'A1_D_H1'}.get(d, 'A1_D_' + d)
out = ['/* generated by tools/gen-apiwrap.py (alpha1, decision 0242) */', '#include "alpha1_rec.h"', '']
tab = ['a1_t a1_tab[] = {']
flds = []
for i, n in enumerate(names):
    short = n[len('fle_imp_'):]
    fl = []
    dec = 'NONE'
    if short in PROBE: fl.append('A1_F_PROBE'); dec = PROBE[short]
    if short in EVENT:
        fl.append('A1_F_EVENT')
        if short not in PROBE or PROBE[short] == 'NONE': dec = EVENT[short]
    tab.append('  {"%s", %s, %s, 0, 0},' % (short, '|'.join(fl) or '0', dec_c(dec) if dec != 'NONE' or fl else 'A1_D_NONE'))
    out.append('void __real_%s(void);\nvoid __wrap_%s(void) { a1_pre(%d); __real_%s(); a1_post(%d); }' % (n, n, i, n, i))
    flds.append('-Wl,--wrap=%s' % n)
tab.append('};'); tab.append('unsigned a1_n = %d;' % len(names))
open(sys.argv[2], 'w').write('\n'.join(out + [''] + tab) + '\n')
open(sys.argv[3], 'w').write('\n'.join(flds + ['-Wl,--wrap=getrusage']) + '\n')
print('wrappers: %d imports, %d probe names, %d event names' % (len(names), sum(1 for n in names if n[8:] in PROBE), sum(1 for n in names if n[8:] in EVENT)))
