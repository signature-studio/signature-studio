#!/usr/bin/env python3
"""pick1 (decision 0242): generate the link-time wrappers of every runtime import for the native build.
usage: gen-apiwrap.py NM_FILE OUT.c OUT.ldflags
NM_FILE = `nm --defined-only` of the runtime objects (the names fle_imp_*). One wrapper per name: apiw_pre / __real_ / apiw_post. The probe and event lists below are the ONLY choice made here;
they decide which calls keep their arguments (counts are kept for every call)."""
import re, sys
names = sorted({m.group(1) for ln in open(sys.argv[1]) for m in [re.search(r'\sT\s+(fle_imp_\w+)$', ln)] if m})
PROBE = {  # name -> decoder
 'LoadLibraryW': 'W1', 'LoadLibraryA': 'A1', 'LoadLibraryExW': 'W1', 'LoadLibraryExA': 'A1', 'GetModuleHandleW': 'W1', 'GetModuleHandleA': 'A1', 'GetModuleHandleExW': 'W3',
 'GetProcAddress': 'A2', 'CoCreateInstance': 'G1G4', 'CoCreateInstanceEx': 'G1G4',
 'RegOpenKeyExW': 'W2', 'RegOpenKeyExA': 'NONE', 'RegQueryValueExW': 'W2', 'RegCreateKeyExW': 'W2', 'RegGetValueW': 'W23', 'RegEnumKeyExW': 'NONE', 'RegQueryValueExA': 'NONE',
 'SystemParametersInfoW': 'SPI', 'SystemParametersInfoA': 'SPI', 'GetSystemMetrics': 'IDX', 'GetSystemMetricsForDpi': 'IDX', 'GetDeviceCaps': 'DEVCAPS',
 'DwmIsCompositionEnabled': 'PTR1U32', 'DwmGetColorizationColor': 'NONE', 'DwmGetWindowAttribute': 'NONE', 'DwmSetWindowAttribute': 'NONE', 'DwmEnableBlurBehindWindow': 'NONE',
 'DwmExtendFrameIntoClientArea': 'MARGINS', 'DwmEnableComposition': 'NONE', 'DwmFlush': 'NONE',
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
 'UpdateLayeredWindow': 'ULW', 'UpdateLayeredWindowIndirect': 'H1', 'SetLayeredWindowAttributes': 'SLWA', 'AnimateWindow': 'H1', 'SetForegroundWindow': 'H1', 'SetActiveWindow': 'H1',
 'DwmEnableBlurBehindWindow': 'H1', 'DwmExtendFrameIntoClientArea': 'MARGINS', 'DwmSetWindowAttribute': 'H1', 'SetWindowCompositionAttribute': 'H1',
 'AlphaBlend': 'H1', 'GradientFill': 'H1', 'TransparentBlt': 'H1', 'BeginBufferedPaint': 'H1', 'CoCreateInstance': 'G1G4', 'LoadLibraryW': 'W1', 'LoadLibraryA': 'A1', 'LoadLibraryExW': 'W1', 'GetProcAddress': 'A2',
 'BitBlt': 'BITBLT', 'StretchBlt': 'STRETCHBLT', 'CreateDIBSection': 'CREATEDIB', 'GetDC': 'H1', 'GetWindowDC': 'H1', 'ReleaseDC': 'H1', 'CreateCompatibleDC': 'H1', 'SelectObject': 'SELOBJ', 'DeleteDC': 'H1', 'BeginPaint': 'H1', 'EndPaint': 'H1', 'TimerProc': 'H1', 'SetTimer': 'H1', 'KillTimer': 'H1',
 'GetActiveWindow': 'H1', 'GetForegroundWindow': 'H1',  'SetFocus': 'H1', 'GetLastActivePopup': 'H1', 'GetParent': 'H1', 'GetAncestor': 'H1', 'GetWindow': 'H1', 'BringWindowToTop': 'H1', 'IsWindowEnabled': 'H1', 'GetCapture': 'H1', 'UpdateWindow': 'H1', 'RedrawWindow': 'H1', 'SetTimer': 'H1', 'KillTimer': 'H1', 'InvalidateRect': 'STK', 'PostMessageW': 'H1',
 'SetWindowRgn': 'H1', 'SetWindowLongW': 'H1', 'SetWindowLongPtrW': 'H1', 'SetClassLongPtrW': 'H1', 'SetClassLongW': 'H1', 'ReleaseCapture': 'H1', 'SetCapture': 'H1',
}
DEC = {'NONE': 'APIW_D_H1'}
dec_c = lambda d: {'NONE': 'APIW_D_H1', 'H1': 'APIW_D_H1'}.get(d, 'APIW_D_' + d)
out = ['/* generated by tools/gen-apiwrap.py (pick1, decision 0242) */', '#include "apiw.h"', '']
tab = ['apiw_t apiw_tab[] = {']
flds = []
for i, n in enumerate(names):
    short = n[len('fle_imp_'):]
    fl = []
    dec = 'NONE'
    if short in PROBE: fl.append('APIW_F_PROBE'); dec = PROBE[short]
    if short in EVENT:
        fl.append('APIW_F_EVENT')
        if short not in PROBE: dec = EVENT[short]
    tab.append('  {"%s", %s, %s, 0, 0},' % (short, '|'.join(fl) or '0', dec_c(dec) if dec != 'NONE' or fl else 'APIW_D_NONE'))
    out.append('void __real_%s(void);\nvoid __wrap_%s(void) { apiw_pre(%d); __real_%s(); apiw_post(%d); }' % (n, n, i, n, i))
    flds.append('-Wl,--wrap=%s' % n)
tab.append('};'); tab.append('unsigned apiw_n = %d;' % len(names))
open(sys.argv[2], 'w').write('\n'.join(out + [''] + tab) + '\n')
open(sys.argv[3], 'w').write('\n'.join(flds + ['-Wl,--wrap=getrusage']) + '\n')
print('wrappers: %d imports, %d probe names, %d event names' % (len(names), sum(1 for n in names if n[8:] in PROBE), sum(1 for n in names if n[8:] in EVENT)))
