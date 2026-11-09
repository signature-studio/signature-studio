/* layout_check.c - compile-time check of the Windows structure layouts that replay.c / gen_cases.py hard-code.
 *   mingw-w64:  x86_64-w64-mingw32-gcc -O0 -o layout_check.exe layout_check.c      (compiles only if every size / offset holds)
 *   MSVC:       cl /nologo /c layout_check.c    (C11 _Static_assert needs /std:c11; if it is rejected, the file is a list of the values to check by hand)
 * Found while writing the tool: LOGFONTW byte 25 is lfClipPrecision (lfQuality is byte 26) and sizeof(CPINFO) is 20. */
#include <windows.h>
#include <stddef.h>
#define CHK(c) _Static_assert(c, #c)
CHK(sizeof(OSVERSIONINFOEXW)==284); CHK(offsetof(OSVERSIONINFOEXW,wServicePackMajor)==276); CHK(offsetof(OSVERSIONINFOEXW,wServicePackMinor)==278);
CHK(offsetof(OSVERSIONINFOEXW,wSuiteMask)==280); CHK(offsetof(OSVERSIONINFOEXW,wProductType)==282); CHK(sizeof(OSVERSIONINFOW)==276);
CHK(sizeof(OSVERSIONINFOA)==148); CHK(sizeof(OSVERSIONINFOEXA)==156); CHK(offsetof(OSVERSIONINFOEXA,wServicePackMajor)==148);
CHK(offsetof(SYSTEM_INFO,dwPageSize)==4); CHK(offsetof(SYSTEM_INFO,lpMinimumApplicationAddress)==8); CHK(offsetof(SYSTEM_INFO,lpMaximumApplicationAddress)==16);
CHK(offsetof(SYSTEM_INFO,dwActiveProcessorMask)==24); CHK(offsetof(SYSTEM_INFO,dwNumberOfProcessors)==32); CHK(offsetof(SYSTEM_INFO,dwProcessorType)==36);
CHK(offsetof(SYSTEM_INFO,dwAllocationGranularity)==40); CHK(offsetof(SYSTEM_INFO,wProcessorLevel)==44); CHK(offsetof(SYSTEM_INFO,wProcessorRevision)==46); CHK(sizeof(SYSTEM_INFO)==48);
CHK(sizeof(TIME_ZONE_INFORMATION)==172); CHK(offsetof(TIME_ZONE_INFORMATION,StandardName)==4); CHK(offsetof(TIME_ZONE_INFORMATION,DaylightName)==88);
CHK(sizeof(STARTUPINFOW)==104); CHK(sizeof(STARTUPINFOA)==104);
CHK(sizeof(LOGFONTW)==92); CHK(offsetof(LOGFONTW,lfWeight)==16); CHK(offsetof(LOGFONTW,lfItalic)==20); CHK(offsetof(LOGFONTW,lfCharSet)==23); CHK(offsetof(LOGFONTW,lfClipPrecision)==25); CHK(offsetof(LOGFONTW,lfQuality)==26); CHK(offsetof(LOGFONTW,lfFaceName)==28);
CHK(sizeof(NONCLIENTMETRICSW)==504); CHK(offsetof(NONCLIENTMETRICSW,iBorderWidth)==4); CHK(offsetof(NONCLIENTMETRICSW,lfCaptionFont)==24);
CHK(sizeof(WIN32_FIND_DATAW)==592); CHK(sizeof(WNDCLASSW)==72); CHK(sizeof(BITMAP)==32); CHK(offsetof(BITMAP,bmBits)==24); CHK(sizeof(DIBSECTION)==104);
CHK(offsetof(DIBSECTION,dsBmih)==32); CHK(offsetof(DIBSECTION,dsBitfields)==72); CHK(offsetof(DIBSECTION,dshSection)==88); CHK(offsetof(DIBSECTION,dsOffset)==96);
CHK(offsetof(BITMAPINFOHEADER,biWidth)==4); CHK(offsetof(BITMAPINFOHEADER,biHeight)==8); CHK(offsetof(BITMAPINFOHEADER,biPlanes)==12); CHK(offsetof(BITMAPINFOHEADER,biBitCount)==14);
CHK(sizeof(BITMAPINFOHEADER)==40); CHK(sizeof(ICONINFO)==32); CHK(offsetof(ICONINFO,hbmMask)==16); CHK(offsetof(ICONINFO,hbmColor)==24);
CHK(sizeof(LOGBRUSH)==16); CHK(sizeof(LOGPEN)==16); CHK(offsetof(LOGPALETTE,palPalEntry)==4); CHK(sizeof(CRITICAL_SECTION)==40); CHK(sizeof(SRWLOCK)==8);
CHK(sizeof(MONITORINFO)==40); CHK(offsetof(MONITORINFO,rcWork)==20); CHK(offsetof(MONITORINFO,dwFlags)==36); CHK(sizeof(WIN32_FILE_ATTRIBUTE_DATA)==36);
CHK(sizeof(CPINFO)==20); CHK(offsetof(CPINFO,DefaultChar)==4); CHK(offsetof(CPINFO,LeadByte)==6); CHK(sizeof(DEVMODEW)==220); CHK(offsetof(DEVMODEW,dmSize)==68);
CHK(offsetof(DEVMODEW,dmBitsPerPel)==168); CHK(offsetof(DEVMODEW,dmPelsWidth)==172); CHK(offsetof(DEVMODEW,dmPelsHeight)==176); CHK(offsetof(DEVMODEW,dmDisplayFlags)==180); CHK(offsetof(DEVMODEW,dmDisplayFrequency)==184);
CHK(sizeof(SYSTEM_LOGICAL_PROCESSOR_INFORMATION)==32); CHK(offsetof(SYSTEM_LOGICAL_PROCESSOR_INFORMATION,Relationship)==8);
CHK(sizeof(SYSTEMTIME)==16); CHK(offsetof(SYSTEMTIME,wDayOfWeek)==4); CHK(offsetof(SYSTEMTIME,wDay)==6);
int main(void){ SYSTEM_LOGICAL_PROCESSOR_INFORMATION s; (void)s; return (int)((char*)&s.Cache.Level-(char*)&s)*0 + (int)((char*)&s.Cache.Size-(char*)&s) - 20 + (int)((char*)&s.Cache.Type-(char*)&s) - 24 + (int)((char*)&s.Cache.LineSize-(char*)&s) - 18 + (int)((char*)&s.Cache.Associativity-(char*)&s) - 17 + (int)((char*)&s.Cache.Level-(char*)&s) - 16; }
