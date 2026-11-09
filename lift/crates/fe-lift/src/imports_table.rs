//! Import thunks that the runtime answers (runtime/fle_imports.c).
//!
//! A thunk is `jmp [IAT slot]`. When fe-decode names the slot (`Resolved::Import`,
//! as `dll!name`, from fe-pe's import table), and the name is in `TABLE`, the thunk
//! calls the runtime function and returns. The guest return address stays on the
//! stack for the caller's `ret`, as runtime/fle_imports.h describes. Any other
//! import keeps the unsupported path.
//!
//! A `call [IAT slot]` (or `call reg` with the slot's content, resolved to the same import) uses the
//! same row: `indirect::call` pushes the return VA, calls the same `fle_imp_*` and pops it, the
//! shape of every direct call (decision 0077). `lookup_call` gives the handler `call_imp:<API>`.
//!
//! Only APIs whose runtime function is implemented and recorded in
//! docs/decisions/ are listed here.

/// (DLL, API name, runtime function, handler name in `Stats::handled`)
const TABLE: &[(&str, &str, &str, &str)] = &[
    ("kernel32.dll", "GetCurrentThreadId", "fle_imp_GetCurrentThreadId", "jmp_imp:GetCurrentThreadId"),
    ("kernel32.dll", "TlsAlloc", "fle_imp_TlsAlloc", "jmp_imp:TlsAlloc"),
    ("kernel32.dll", "LocalAlloc", "fle_imp_LocalAlloc", "jmp_imp:LocalAlloc"),
    ("kernel32.dll", "TlsSetValue", "fle_imp_TlsSetValue", "jmp_imp:TlsSetValue"),
    ("kernel32.dll", "TlsGetValue", "fle_imp_TlsGetValue", "jmp_imp:TlsGetValue"),
    ("kernel32.dll", "GetLastError", "fle_imp_GetLastError", "jmp_imp:GetLastError"),
    ("kernel32.dll", "VerSetConditionMask", "fle_imp_VerSetConditionMask", "jmp_imp:VerSetConditionMask"),
    ("kernel32.dll", "LocalFree", "fle_imp_LocalFree", "jmp_imp:LocalFree"),
    ("kernel32.dll", "VerifyVersionInfoW", "fle_imp_VerifyVersionInfoW", "jmp_imp:VerifyVersionInfoW"),
    ("kernel32.dll", "GetVersion", "fle_imp_GetVersion", "jmp_imp:GetVersion"),
    ("kernel32.dll", "GetVersionExW", "fle_imp_GetVersionExW", "jmp_imp:GetVersionExW"),
    ("kernel32.dll", "GetModuleHandleW", "fle_imp_GetModuleHandleW", "jmp_imp:GetModuleHandleW"),
    ("kernel32.dll", "GetProcAddress", "fle_imp_GetProcAddress", "jmp_imp:GetProcAddress"),
    ("kernel32.dll", "Sleep", "fle_imp_Sleep", "jmp_imp:Sleep"),
    ("kernel32.dll", "InitializeCriticalSection", "fle_imp_InitializeCriticalSection", "jmp_imp:InitializeCriticalSection"),
    ("kernel32.dll", "EnterCriticalSection", "fle_imp_EnterCriticalSection", "jmp_imp:EnterCriticalSection"),
    ("kernel32.dll", "LeaveCriticalSection", "fle_imp_LeaveCriticalSection", "jmp_imp:LeaveCriticalSection"),
    ("kernel32.dll", "DeleteCriticalSection", "fle_imp_DeleteCriticalSection", "jmp_imp:DeleteCriticalSection"),
    ("kernel32.dll", "GetTickCount", "fle_imp_GetTickCount", "jmp_imp:GetTickCount"),
    ("winmm.dll", "timeGetTime", "fle_imp_timeGetTime", "jmp_imp:timeGetTime"),
    ("kernel32.dll", "QueryPerformanceCounter", "fle_imp_QueryPerformanceCounter", "jmp_imp:QueryPerformanceCounter"),
    ("kernel32.dll", "QueryPerformanceFrequency", "fle_imp_QueryPerformanceFrequency", "jmp_imp:QueryPerformanceFrequency"),
    ("kernel32.dll", "GetSystemInfo", "fle_imp_GetSystemInfo", "jmp_imp:GetSystemInfo"),
    ("kernel32.dll", "VirtualAlloc", "fle_imp_VirtualAlloc", "jmp_imp:VirtualAlloc"),
    ("kernel32.dll", "VirtualFree", "fle_imp_VirtualFree", "jmp_imp:VirtualFree"),
    ("kernel32.dll", "GetProcessHeap", "fle_imp_GetProcessHeap", "jmp_imp:GetProcessHeap"),
    ("kernel32.dll", "HeapAlloc", "fle_imp_HeapAlloc", "jmp_imp:HeapAlloc"),
    ("kernel32.dll", "HeapFree", "fle_imp_HeapFree", "jmp_imp:HeapFree"),
    ("kernel32.dll", "SetThreadLocale", "fle_imp_SetThreadLocale", "jmp_imp:SetThreadLocale"),
    ("kernel32.dll", "GetThreadLocale", "fle_imp_GetThreadLocale", "jmp_imp:GetThreadLocale"),
    ("kernel32.dll", "IsValidLocale", "fle_imp_IsValidLocale", "jmp_imp:IsValidLocale"),
    ("kernel32.dll", "GetUserDefaultUILanguage", "fle_imp_GetUserDefaultUILanguage", "jmp_imp:GetUserDefaultUILanguage"),
    ("kernel32.dll", "GetSystemDefaultUILanguage", "fle_imp_GetSystemDefaultUILanguage", "jmp_imp:GetSystemDefaultUILanguage"),
    ("kernel32.dll", "GetACP", "fle_imp_GetACP", "jmp_imp:GetACP"),
    ("kernel32.dll", "IsDBCSLeadByteEx", "fle_imp_IsDBCSLeadByteEx", "jmp_imp:IsDBCSLeadByteEx"),
    ("kernel32.dll", "GetCPInfo", "fle_imp_GetCPInfo", "jmp_imp:GetCPInfo"),
    ("kernel32.dll", "GetLocaleInfoW", "fle_imp_GetLocaleInfoW", "jmp_imp:GetLocaleInfoW"),
    ("kernel32.dll", "LCMapStringW", "fle_imp_LCMapStringW", "jmp_imp:LCMapStringW"),
    ("kernel32.dll", "CompareStringW", "fle_imp_CompareStringW", "jmp_imp:CompareStringW"),
    ("kernel32.dll", "MultiByteToWideChar", "fle_imp_MultiByteToWideChar", "jmp_imp:MultiByteToWideChar"),
    ("kernel32.dll", "WideCharToMultiByte", "fle_imp_WideCharToMultiByte", "jmp_imp:WideCharToMultiByte"),
    // Virtual file system, registry and environment (decision 0018): runtime/fle_vfs.c, fle_reg.c, fle_env.c.
    ("kernel32.dll", "CreateFileW", "fle_imp_CreateFileW", "jmp_imp:CreateFileW"),
    ("kernel32.dll", "ReadFile", "fle_imp_ReadFile", "jmp_imp:ReadFile"),
    ("kernel32.dll", "WriteFile", "fle_imp_WriteFile", "jmp_imp:WriteFile"),
    ("kernel32.dll", "CloseHandle", "fle_imp_CloseHandle", "jmp_imp:CloseHandle"),
    ("kernel32.dll", "SetFilePointer", "fle_imp_SetFilePointer", "jmp_imp:SetFilePointer"),
    ("kernel32.dll", "GetFileSize", "fle_imp_GetFileSize", "jmp_imp:GetFileSize"),
    ("kernel32.dll", "SetFileTime", "fle_imp_SetFileTime", "jmp_imp:SetFileTime"),
    ("kernel32.dll", "FindFirstFileW", "fle_imp_FindFirstFileW", "jmp_imp:FindFirstFileW"),
    ("kernel32.dll", "FindNextFileW", "fle_imp_FindNextFileW", "jmp_imp:FindNextFileW"),
    ("kernel32.dll", "FindClose", "fle_imp_FindClose", "jmp_imp:FindClose"),
    ("kernel32.dll", "GetFileAttributesW", "fle_imp_GetFileAttributesW", "jmp_imp:GetFileAttributesW"),
    ("kernel32.dll", "GetFileAttributesExW", "fle_imp_GetFileAttributesExW", "jmp_imp:GetFileAttributesExW"),
    ("kernel32.dll", "SetFileAttributesW", "fle_imp_SetFileAttributesW", "jmp_imp:SetFileAttributesW"),
    ("kernel32.dll", "CreateDirectoryW", "fle_imp_CreateDirectoryW", "jmp_imp:CreateDirectoryW"),
    ("kernel32.dll", "RemoveDirectoryW", "fle_imp_RemoveDirectoryW", "jmp_imp:RemoveDirectoryW"),
    ("kernel32.dll", "DeleteFileW", "fle_imp_DeleteFileW", "jmp_imp:DeleteFileW"),
    ("kernel32.dll", "CopyFileW", "fle_imp_CopyFileW", "jmp_imp:CopyFileW"),
    ("kernel32.dll", "MoveFileW", "fle_imp_MoveFileW", "jmp_imp:MoveFileW"),
    ("kernel32.dll", "GetFullPathNameW", "fle_imp_GetFullPathNameW", "jmp_imp:GetFullPathNameW"),
    ("kernel32.dll", "GetCurrentDirectoryW", "fle_imp_GetCurrentDirectoryW", "jmp_imp:GetCurrentDirectoryW"),
    ("kernel32.dll", "SetCurrentDirectoryW", "fle_imp_SetCurrentDirectoryW", "jmp_imp:SetCurrentDirectoryW"),
    ("kernel32.dll", "GetTempPathW", "fle_imp_GetTempPathW", "jmp_imp:GetTempPathW"),
    ("kernel32.dll", "GetModuleFileNameW", "fle_imp_GetModuleFileNameW", "jmp_imp:GetModuleFileNameW"),
    ("kernel32.dll", "GetWindowsDirectoryW", "fle_imp_GetWindowsDirectoryW", "jmp_imp:GetWindowsDirectoryW"),
    ("kernel32.dll", "GetSystemDirectoryW", "fle_imp_GetSystemDirectoryW", "jmp_imp:GetSystemDirectoryW"),
    ("advapi32.dll", "RegOpenKeyExW", "fle_imp_RegOpenKeyExW", "jmp_imp:RegOpenKeyExW"),
    ("advapi32.dll", "RegOpenKeyExA", "fle_imp_RegOpenKeyExA", "jmp_imp:RegOpenKeyExA"),
    ("advapi32.dll", "RegQueryValueExW", "fle_imp_RegQueryValueExW", "jmp_imp:RegQueryValueExW"),
    ("advapi32.dll", "RegCloseKey", "fle_imp_RegCloseKey", "jmp_imp:RegCloseKey"),
    ("advapi32.dll", "RegCreateKeyExW", "fle_imp_RegCreateKeyExW", "jmp_imp:RegCreateKeyExW"),
    ("advapi32.dll", "RegSetValueExW", "fle_imp_RegSetValueExW", "jmp_imp:RegSetValueExW"),
    ("advapi32.dll", "RegEnumKeyExW", "fle_imp_RegEnumKeyExW", "jmp_imp:RegEnumKeyExW"),
    ("advapi32.dll", "RegEnumValueW", "fle_imp_RegEnumValueW", "jmp_imp:RegEnumValueW"),
    ("advapi32.dll", "RegDeleteKeyW", "fle_imp_RegDeleteKeyW", "jmp_imp:RegDeleteKeyW"),
    ("advapi32.dll", "RegDeleteValueW", "fle_imp_RegDeleteValueW", "jmp_imp:RegDeleteValueW"),
    ("advapi32.dll", "RegQueryInfoKeyW", "fle_imp_RegQueryInfoKeyW", "jmp_imp:RegQueryInfoKeyW"),
    ("advapi32.dll", "RegFlushKey", "fle_imp_RegFlushKey", "jmp_imp:RegFlushKey"),
    ("kernel32.dll", "GetEnvironmentVariableW", "fle_imp_GetEnvironmentVariableW", "jmp_imp:GetEnvironmentVariableW"),
    ("kernel32.dll", "SetEnvironmentVariableW", "fle_imp_SetEnvironmentVariableW", "jmp_imp:SetEnvironmentVariableW"),
    ("kernel32.dll", "ExpandEnvironmentStringsW", "fle_imp_ExpandEnvironmentStringsW", "jmp_imp:ExpandEnvironmentStringsW"),
    ("kernel32.dll", "GetCommandLineW", "fle_imp_GetCommandLineW", "jmp_imp:GetCommandLineW"),
    ("kernel32.dll", "GetStartupInfoW", "fle_imp_GetStartupInfoW", "jmp_imp:GetStartupInfoW"),
    // Thread UI language (decision 0021): runtime/fle_uilang.c.
    ("kernel32.dll", "GetThreadUILanguage", "fle_imp_GetThreadUILanguage", "jmp_imp:GetThreadUILanguage"),
    // Thread preferred UI languages (decision 0022): runtime/fle_uipref.c.
    ("kernel32.dll", "GetThreadPreferredUILanguages", "fle_imp_GetThreadPreferredUILanguages", "jmp_imp:GetThreadPreferredUILanguages"),
    // Process and thread answers (decision 0020): runtime/fle_proc.c.
    ("kernel32.dll", "GetCurrentProcess", "fle_imp_GetCurrentProcess", "jmp_imp:GetCurrentProcess"),
    ("kernel32.dll", "GetCurrentProcessId", "fle_imp_GetCurrentProcessId", "jmp_imp:GetCurrentProcessId"),
    ("kernel32.dll", "GetCurrentThread", "fle_imp_GetCurrentThread", "jmp_imp:GetCurrentThread"),
    ("kernel32.dll", "SetLastError", "fle_imp_SetLastError", "jmp_imp:SetLastError"),
    ("kernel32.dll", "SetErrorMode", "fle_imp_SetErrorMode", "jmp_imp:SetErrorMode"),
    ("kernel32.dll", "SetThreadErrorMode", "fle_imp_SetThreadErrorMode", "jmp_imp:SetThreadErrorMode"),
    ("kernel32.dll", "SetPriorityClass", "fle_imp_SetPriorityClass", "jmp_imp:SetPriorityClass"),
    ("kernel32.dll", "SetThreadPriority", "fle_imp_SetThreadPriority", "jmp_imp:SetThreadPriority"),
    ("kernel32.dll", "SwitchToThread", "fle_imp_SwitchToThread", "jmp_imp:SwitchToThread"),
    ("kernel32.dll", "OutputDebugStringW", "fle_imp_OutputDebugStringW", "jmp_imp:OutputDebugStringW"),
    ("kernel32.dll", "ExitProcess", "fle_imp_ExitProcess", "jmp_imp:ExitProcess"),
    ("kernel32.dll", "TerminateProcess", "fle_imp_TerminateProcess", "jmp_imp:TerminateProcess"),
    ("kernel32.dll", "RaiseException", "fle_imp_RaiseException", "jmp_imp:RaiseException"),
    // Emulated time, time zone and date (decision 0020): runtime/fle_time.c.
    ("kernel32.dll", "GetSystemTime", "fle_imp_GetSystemTime", "jmp_imp:GetSystemTime"),
    ("kernel32.dll", "GetLocalTime", "fle_imp_GetLocalTime", "jmp_imp:GetLocalTime"),
    ("kernel32.dll", "GetSystemTimeAsFileTime", "fle_imp_GetSystemTimeAsFileTime", "jmp_imp:GetSystemTimeAsFileTime"),
    ("kernel32.dll", "GetTickCount64", "fle_imp_GetTickCount64", "jmp_imp:GetTickCount64"),
    ("kernel32.dll", "GetTimeZoneInformation", "fle_imp_GetTimeZoneInformation", "jmp_imp:GetTimeZoneInformation"),
    ("kernel32.dll", "FileTimeToSystemTime", "fle_imp_FileTimeToSystemTime", "jmp_imp:FileTimeToSystemTime"),
    ("kernel32.dll", "SystemTimeToFileTime", "fle_imp_SystemTimeToFileTime", "jmp_imp:SystemTimeToFileTime"),
    ("kernel32.dll", "FileTimeToLocalFileTime", "fle_imp_FileTimeToLocalFileTime", "jmp_imp:FileTimeToLocalFileTime"),
    ("kernel32.dll", "LocalFileTimeToFileTime", "fle_imp_LocalFileTimeToFileTime", "jmp_imp:LocalFileTimeToFileTime"),
    ("kernel32.dll", "SystemTimeToTzSpecificLocalTime", "fle_imp_SystemTimeToTzSpecificLocalTime", "jmp_imp:SystemTimeToTzSpecificLocalTime"),
    ("kernel32.dll", "DosDateTimeToFileTime", "fle_imp_DosDateTimeToFileTime", "jmp_imp:DosDateTimeToFileTime"),
    ("kernel32.dll", "FileTimeToDosDateTime", "fle_imp_FileTimeToDosDateTime", "jmp_imp:FileTimeToDosDateTime"),
    ("kernel32.dll", "GetDateFormatW", "fle_imp_GetDateFormatW", "jmp_imp:GetDateFormatW"),
    // Resources of the lifted DLL itself (decision 0025): runtime/fle_res.c.
    ("kernel32.dll", "FindResourceW", "fle_imp_FindResourceW", "jmp_imp:FindResourceW"),
    ("kernel32.dll", "LoadResource", "fle_imp_LoadResource", "jmp_imp:LoadResource"),
    ("kernel32.dll", "LockResource", "fle_imp_LockResource", "jmp_imp:LockResource"),
    ("kernel32.dll", "SizeofResource", "fle_imp_SizeofResource", "jmp_imp:SizeofResource"),
    ("user32.dll", "LoadStringW", "fle_imp_LoadStringW", "jmp_imp:LoadStringW"),
    // Library loader over the virtual module table and ImageNtHeader (decision 0029): runtime/fle_load.c, fle_imagehlp.c.
    ("kernel32.dll", "LoadLibraryA", "fle_imp_LoadLibraryA", "jmp_imp:LoadLibraryA"),
    ("kernel32.dll", "LoadLibraryW", "fle_imp_LoadLibraryW", "jmp_imp:LoadLibraryW"),
    ("kernel32.dll", "LoadLibraryExW", "fle_imp_LoadLibraryExW", "jmp_imp:LoadLibraryExW"),
    ("kernel32.dll", "FreeLibrary", "fle_imp_FreeLibrary", "jmp_imp:FreeLibrary"),
    ("kernel32.dll", "SetDllDirectoryW", "fle_imp_SetDllDirectoryW", "jmp_imp:SetDllDirectoryW"),
    ("imagehlp.dll", "ImageNtHeader", "fle_imp_ImageNtHeader", "jmp_imp:ImageNtHeader"),
    // user32 answers that need no window (decision 0030): runtime/fle_user_nowin.c.
    ("user32.dll", "CharLowerW", "fle_imp_CharLowerW", "jmp_imp:CharLowerW"),
    ("user32.dll", "CharLowerBuffW", "fle_imp_CharLowerBuffW", "jmp_imp:CharLowerBuffW"),
    ("user32.dll", "CharUpperW", "fle_imp_CharUpperW", "jmp_imp:CharUpperW"),
    ("user32.dll", "CharUpperBuffW", "fle_imp_CharUpperBuffW", "jmp_imp:CharUpperBuffW"),
    ("user32.dll", "CharNextW", "fle_imp_CharNextW", "jmp_imp:CharNextW"),
    ("user32.dll", "SetRect", "fle_imp_SetRect", "jmp_imp:SetRect"),
    ("user32.dll", "IntersectRect", "fle_imp_IntersectRect", "jmp_imp:IntersectRect"),
    ("user32.dll", "OffsetRect", "fle_imp_OffsetRect", "jmp_imp:OffsetRect"),
    ("user32.dll", "UnionRect", "fle_imp_UnionRect", "jmp_imp:UnionRect"),
    ("user32.dll", "RegisterWindowMessageW", "fle_imp_RegisterWindowMessageW", "jmp_imp:RegisterWindowMessageW"),
    ("user32.dll", "RegisterClipboardFormatW", "fle_imp_RegisterClipboardFormatW", "jmp_imp:RegisterClipboardFormatW"),
    ("user32.dll", "GetClipboardFormatNameW", "fle_imp_GetClipboardFormatNameW", "jmp_imp:GetClipboardFormatNameW"),
    // winmm: no MIDI device, multimedia-timer bookkeeping (decision 0030): runtime/fle_winmm.c.
    ("winmm.dll", "midiInGetNumDevs", "fle_imp_midiInGetNumDevs", "jmp_imp:midiInGetNumDevs"),
    ("winmm.dll", "midiOutGetNumDevs", "fle_imp_midiOutGetNumDevs", "jmp_imp:midiOutGetNumDevs"),
    ("winmm.dll", "midiInGetDevCapsW", "fle_imp_midiInGetDevCapsW", "jmp_imp:midiInGetDevCapsW"),
    ("winmm.dll", "midiOutGetDevCapsW", "fle_imp_midiOutGetDevCapsW", "jmp_imp:midiOutGetDevCapsW"),
    ("winmm.dll", "midiOutGetErrorTextW", "fle_imp_midiOutGetErrorTextW", "jmp_imp:midiOutGetErrorTextW"),
    ("winmm.dll", "midiOutOpen", "fle_imp_midiOutOpen", "jmp_imp:midiOutOpen"),
    ("winmm.dll", "midiOutShortMsg", "fle_imp_midiOutShortMsg", "jmp_imp:midiOutShortMsg"),
    ("winmm.dll", "timeBeginPeriod", "fle_imp_timeBeginPeriod", "jmp_imp:timeBeginPeriod"),
    ("winmm.dll", "timeEndPeriod", "fle_imp_timeEndPeriod", "jmp_imp:timeEndPeriod"),
    ("winmm.dll", "timeGetDevCaps", "fle_imp_timeGetDevCaps", "jmp_imp:timeGetDevCaps"),
    // Global memory, atoms, MulDiv, lstrlenW (decision 0024): runtime/fle_glob.c.
    ("kernel32.dll", "GlobalAlloc", "fle_imp_GlobalAlloc", "jmp_imp:GlobalAlloc"),
    ("kernel32.dll", "GlobalFree", "fle_imp_GlobalFree", "jmp_imp:GlobalFree"),
    ("kernel32.dll", "GlobalLock", "fle_imp_GlobalLock", "jmp_imp:GlobalLock"),
    ("kernel32.dll", "GlobalUnlock", "fle_imp_GlobalUnlock", "jmp_imp:GlobalUnlock"),
    ("kernel32.dll", "AddAtomW", "fle_imp_AddAtomW", "jmp_imp:AddAtomW"),
    ("kernel32.dll", "DeleteAtom", "fle_imp_DeleteAtom", "jmp_imp:DeleteAtom"),
    ("kernel32.dll", "FindAtomW", "fle_imp_FindAtomW", "jmp_imp:FindAtomW"),
    ("kernel32.dll", "GlobalAddAtomW", "fle_imp_GlobalAddAtomW", "jmp_imp:GlobalAddAtomW"),
    ("kernel32.dll", "GlobalFindAtomW", "fle_imp_GlobalFindAtomW", "jmp_imp:GlobalFindAtomW"),
    ("kernel32.dll", "MulDiv", "fle_imp_MulDiv", "jmp_imp:MulDiv"),
    ("kernel32.dll", "lstrlenW", "fle_imp_lstrlenW", "jmp_imp:lstrlenW"),
    // Volumes, standard handle, DuplicateHandle (decision 0024): runtime/fle_vol.c.
    ("kernel32.dll", "GetStdHandle", "fle_imp_GetStdHandle", "jmp_imp:GetStdHandle"),
    ("kernel32.dll", "GetDriveTypeW", "fle_imp_GetDriveTypeW", "jmp_imp:GetDriveTypeW"),
    ("kernel32.dll", "GetLogicalDrives", "fle_imp_GetLogicalDrives", "jmp_imp:GetLogicalDrives"),
    ("kernel32.dll", "GetLogicalDriveStringsW", "fle_imp_GetLogicalDriveStringsW", "jmp_imp:GetLogicalDriveStringsW"),
    ("kernel32.dll", "GetVolumeInformationW", "fle_imp_GetVolumeInformationW", "jmp_imp:GetVolumeInformationW"),
    ("kernel32.dll", "QueryDosDeviceW", "fle_imp_QueryDosDeviceW", "jmp_imp:QueryDosDeviceW"),
    ("kernel32.dll", "DuplicateHandle", "fle_imp_DuplicateHandle", "jmp_imp:DuplicateHandle"),
    // Memory protection and file mapping (decision 0024): runtime/fle_vmap.c.
    ("kernel32.dll", "VirtualProtect", "fle_imp_VirtualProtect", "jmp_imp:VirtualProtect"),
    ("kernel32.dll", "VirtualQuery", "fle_imp_VirtualQuery", "jmp_imp:VirtualQuery"),
    ("kernel32.dll", "CreateFileMappingW", "fle_imp_CreateFileMappingW", "jmp_imp:CreateFileMappingW"),
    ("kernel32.dll", "MapViewOfFile", "fle_imp_MapViewOfFile", "jmp_imp:MapViewOfFile"),
    ("kernel32.dll", "UnmapViewOfFile", "fle_imp_UnmapViewOfFile", "jmp_imp:UnmapViewOfFile"),
    // System information (decision 0034): runtime/fle_sysinfo.c. GetNativeSystemInfo, GetLogicalProcessorInformation and
    // IsWow64Process are delay-loaded by the DLL: they are reached through GetProcAddress (the response table), not a jmp thunk.
    ("kernel32.dll", "GetNativeSystemInfo", "fle_imp_GetNativeSystemInfo", "jmp_imp:GetNativeSystemInfo"),
    ("kernel32.dll", "GetLogicalProcessorInformation", "fle_imp_GetLogicalProcessorInformation", "jmp_imp:GetLogicalProcessorInformation"),
    ("kernel32.dll", "IsWow64Process", "fle_imp_IsWow64Process", "jmp_imp:IsWow64Process"),
    ("kernel32.dll", "GlobalMemoryStatus", "fle_imp_GlobalMemoryStatus", "jmp_imp:GlobalMemoryStatus"),
    ("kernel32.dll", "GetProcessAffinityMask", "fle_imp_GetProcessAffinityMask", "jmp_imp:GetProcessAffinityMask"),
    ("kernel32.dll", "SetProcessAffinityMask", "fle_imp_SetProcessAffinityMask", "jmp_imp:SetProcessAffinityMask"),
    // oleaut32 BSTR, VARIANT, SAFEARRAY, GetErrorInfo (decision 0037): runtime/fle_oleaut.c.
    ("oleaut32.dll", "SysAllocStringLen", "fle_imp_SysAllocStringLen", "jmp_imp:SysAllocStringLen"),
    ("oleaut32.dll", "SysFreeString", "fle_imp_SysFreeString", "jmp_imp:SysFreeString"),
    ("oleaut32.dll", "SysReAllocStringLen", "fle_imp_SysReAllocStringLen", "jmp_imp:SysReAllocStringLen"),
    ("oleaut32.dll", "VariantInit", "fle_imp_VariantInit", "jmp_imp:VariantInit"),
    ("oleaut32.dll", "VariantClear", "fle_imp_VariantClear", "jmp_imp:VariantClear"),
    ("oleaut32.dll", "VariantCopy", "fle_imp_VariantCopy", "jmp_imp:VariantCopy"),
    ("oleaut32.dll", "VariantCopyInd", "fle_imp_VariantCopyInd", "jmp_imp:VariantCopyInd"),
    ("oleaut32.dll", "VariantChangeType", "fle_imp_VariantChangeType", "jmp_imp:VariantChangeType"),
    ("oleaut32.dll", "SafeArrayCreate", "fle_imp_SafeArrayCreate", "jmp_imp:SafeArrayCreate"),
    ("oleaut32.dll", "SafeArrayGetLBound", "fle_imp_SafeArrayGetLBound", "jmp_imp:SafeArrayGetLBound"),
    ("oleaut32.dll", "SafeArrayGetUBound", "fle_imp_SafeArrayGetUBound", "jmp_imp:SafeArrayGetUBound"),
    ("oleaut32.dll", "SafeArrayAccessData", "fle_imp_SafeArrayAccessData", "jmp_imp:SafeArrayAccessData"),
    ("oleaut32.dll", "SafeArrayUnaccessData", "fle_imp_SafeArrayUnaccessData", "jmp_imp:SafeArrayUnaccessData"),
    ("oleaut32.dll", "SafeArrayGetElement", "fle_imp_SafeArrayGetElement", "jmp_imp:SafeArrayGetElement"),
    ("oleaut32.dll", "SafeArrayPutElement", "fle_imp_SafeArrayPutElement", "jmp_imp:SafeArrayPutElement"),
    ("oleaut32.dll", "SafeArrayPtrOfIndex", "fle_imp_SafeArrayPtrOfIndex", "jmp_imp:SafeArrayPtrOfIndex"),
    ("oleaut32.dll", "SafeArrayGetElemsize", "fle_imp_SafeArrayGetElemsize", "jmp_imp:SafeArrayGetElemsize"),
    ("oleaut32.dll", "GetErrorInfo", "fle_imp_GetErrorInfo", "jmp_imp:GetErrorInfo"),
    // ole32 COM initialization, task memory, CLSID strings, IsEqualGUID (decision 0037): runtime/fle_ole.c.
    ("ole32.dll", "CoInitialize", "fle_imp_CoInitialize", "jmp_imp:CoInitialize"),
    ("ole32.dll", "CoUninitialize", "fle_imp_CoUninitialize", "jmp_imp:CoUninitialize"),
    ("ole32.dll", "OleInitialize", "fle_imp_OleInitialize", "jmp_imp:OleInitialize"),
    ("ole32.dll", "OleUninitialize", "fle_imp_OleUninitialize", "jmp_imp:OleUninitialize"),
    ("ole32.dll", "CoTaskMemAlloc", "fle_imp_CoTaskMemAlloc", "jmp_imp:CoTaskMemAlloc"),
    ("ole32.dll", "CoTaskMemFree", "fle_imp_CoTaskMemFree", "jmp_imp:CoTaskMemFree"),
    ("ole32.dll", "StringFromCLSID", "fle_imp_StringFromCLSID", "jmp_imp:StringFromCLSID"),
    ("ole32.dll", "ProgIDFromCLSID", "fle_imp_ProgIDFromCLSID", "jmp_imp:ProgIDFromCLSID"),
    ("ole32.dll", "CLSIDFromString", "fle_imp_CLSIDFromString", "jmp_imp:CLSIDFromString"),
    ("ole32.dll", "IsEqualGUID", "fle_imp_IsEqualGUID", "jmp_imp:IsEqualGUID"),
    // Event objects (decision 0050): runtime/fle_event.c. CloseHandle is answered by fle_vfs.c, which asks this file's hook.
    ("kernel32.dll", "CreateEventW", "fle_imp_CreateEventW", "jmp_imp:CreateEventW"),
    ("kernel32.dll", "SetEvent", "fle_imp_SetEvent", "jmp_imp:SetEvent"),
    ("kernel32.dll", "ResetEvent", "fle_imp_ResetEvent", "jmp_imp:ResetEvent"),
    ("kernel32.dll", "WaitForSingleObject", "fle_imp_WaitForSingleObject", "jmp_imp:WaitForSingleObject"),
    // The virtual screen DC, GetDC(NULL) only (decision 0053): runtime/fle_gdc.c.
    ("user32.dll", "GetDC", "fle_imp_GetDC", "jmp_imp:GetDC"),
    ("user32.dll", "ReleaseDC", "fle_imp_ReleaseDC", "jmp_imp:ReleaseDC"),
    ("gdi32.dll", "GetDeviceCaps", "fle_imp_GetDeviceCaps", "jmp_imp:GetDeviceCaps"),
    // The virtual GDI object table (decision 0054): runtime/fle_gdi.c. Only the guest's forms are answered.
    ("gdi32.dll", "CreatePalette", "fle_imp_CreatePalette", "jmp_imp:CreatePalette"),
    ("gdi32.dll", "DeleteObject", "fle_imp_DeleteObject", "jmp_imp:DeleteObject"),
    ("gdi32.dll", "GetStockObject", "fle_imp_GetStockObject", "jmp_imp:GetStockObject"),
    ("user32.dll", "LoadIconW", "fle_imp_LoadIconW", "jmp_imp:LoadIconW"),
    // msctf.dll is a virtual module (decision 0055). SetInputScope is found by GetProcAddress and stops by name when called.
    ("msctf.dll", "SetInputScope", "fle_imp_SetInputScope", "jmp_imp:SetInputScope"),
    // Virtual system metrics (decision 0057): runtime/fle_sysmetrics.c. Only the indexes the guest uses and that have a decided value.
    ("user32.dll", "GetSystemMetrics", "fle_imp_GetSystemMetrics", "jmp_imp:GetSystemMetrics"),
    // Virtual SystemParametersInfoW (decision 0058): runtime/fle_spi.c. Only SPI_GETWHEELSCROLLLINES is answered; the other uiActions stop by name.
    ("user32.dll", "SystemParametersInfoW", "fle_imp_SystemParametersInfoW", "jmp_imp:SystemParametersInfoW"),
    // Virtual standard cursor (decision 0059): runtime/fle_cursor.c. Only LoadCursorW(NULL, IDC_ARROW) is answered; every other form stops by name.
    ("user32.dll", "LoadCursorW", "fle_imp_LoadCursorW", "jmp_imp:LoadCursorW"),
    // Virtual LoadImageW (decision 0061): runtime/fle_loadimage.c. Only (IMAGE_CURSOR, 32, 32, LR_SHARED) with the guest's identifiers; every other form stops by name.
    ("user32.dll", "LoadImageW", "fle_imp_LoadImageW", "jmp_imp:LoadImageW"),
    // Virtual input locale (decision 0063, provisional): runtime/fle_kbd.c. Only GetKeyboardLayout(0) is answered; every other idThread stops by name.
    ("user32.dll", "GetKeyboardLayout", "fle_imp_GetKeyboardLayout", "jmp_imp:GetKeyboardLayout"),
    // Virtual monitor (decision 0065, provisional): runtime/fle_monitor.c. EnumDisplayMonitors(NULL, NULL, proc, data) calls the guest's callback through fle_call_guest (decision 0064); GetMonitorInfoW answers the one virtual monitor with MONITORINFO (cbSize 0x28). Every other form stops by name.
    ("user32.dll", "EnumDisplayMonitors", "fle_imp_EnumDisplayMonitors", "jmp_imp:EnumDisplayMonitors"),
    ("user32.dll", "GetMonitorInfoW", "fle_imp_GetMonitorInfoW", "jmp_imp:GetMonitorInfoW"),
    // Virtual system fonts (decision 0066, provisional): CreateFontIndirectW adds the kind "font" to the GDI object table of runtime/fle_gdi.c; the fonts of SPI_GETICONTITLELOGFONT / SPI_GETNONCLIENTMETRICS are in runtime/fle_spi.c.
    ("gdi32.dll", "CreateFontIndirectW", "fle_imp_CreateFontIndirectW", "jmp_imp:CreateFontIndirectW"),
    // Virtual GetObjectW (decision 0068, provisional): runtime/fle_gdi.c. Only a live created font with c == 0x5c and a non-NULL pv is answered (the 92-byte LOGFONTW copy of decision 0066); every other form stops by name.
    ("gdi32.dll", "GetObjectW", "fle_imp_GetObjectW", "jmp_imp:GetObjectW"),
    // Virtual window class table (decision 0070, provisional): runtime/fle_class.c. RegisterClassW keeps a copy of the 72-byte WNDCLASSW (the window procedure is never called); GetClassInfoW answers the guest's own module (0x400000) only. UnregisterClassW and every other form stop by name.
    ("user32.dll", "RegisterClassW", "fle_imp_RegisterClassW", "jmp_imp:RegisterClassW"),
    ("user32.dll", "GetClassInfoW", "fle_imp_GetClassInfoW", "jmp_imp:GetClassInfoW"),
    // Virtual windows (decision 0071, provisional): runtime/fle_win.c. CreateWindowExW answers only the hidden WS_POPUP tool window of size 0 (the guest's AllocateHWnd) and sends WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE to the window procedure through fle_call_guest (decision 0064); SetWindowLongPtrW answers GWLP_WNDPROC only; DefWindowProcW answers those three messages only. Every other form stops by name.
    ("user32.dll", "CreateWindowExW", "fle_imp_CreateWindowExW", "jmp_imp:CreateWindowExW"),
    ("user32.dll", "SetWindowLongPtrW", "fle_imp_SetWindowLongPtrW", "jmp_imp:SetWindowLongPtrW"),
    ("user32.dll", "DefWindowProcW", "fle_imp_DefWindowProcW", "jmp_imp:DefWindowProcW"),
    // dsp_ippv2_x64.dll's KERNEL32 imports (decision 0078, provisional): the ones that DLL_PROCESS_ATTACH reaches are answered from the Learn
    // pages: InitializeCriticalSectionAndSpinCount, InitializeSListHead, InterlockedFlushSList (runtime/fle_sys.c), FlsAlloc, FlsFree,
    // FlsGetValue, FlsSetValue (fle_imports.c), GetCommandLineA, GetEnvironmentStringsW, FreeEnvironmentStringsW (fle_env.c), GetOEMCP,
    // IsValidCodePage (fle_locale.c). Two implemented functions are NOT rows, because an existing test asserts they are absent (decision 0078,
    // section 5): TlsFree (fle_imports.c; tests/imports_table.rs uses it as the "not in the table" name) and IsProcessorFeaturePresent
    // (fle_sysinfo.c; tests/sysinfo.rs asserts that GetProcAddress does not find it).
    ("kernel32.dll", "InitializeCriticalSectionAndSpinCount", "fle_imp_InitializeCriticalSectionAndSpinCount", "jmp_imp:InitializeCriticalSectionAndSpinCount"),
    ("kernel32.dll", "InitializeSListHead", "fle_imp_InitializeSListHead", "jmp_imp:InitializeSListHead"),
    ("kernel32.dll", "InterlockedFlushSList", "fle_imp_InterlockedFlushSList", "jmp_imp:InterlockedFlushSList"),
    ("kernel32.dll", "FlsAlloc", "fle_imp_FlsAlloc", "jmp_imp:FlsAlloc"),
    ("kernel32.dll", "FlsFree", "fle_imp_FlsFree", "jmp_imp:FlsFree"),
    ("kernel32.dll", "FlsGetValue", "fle_imp_FlsGetValue", "jmp_imp:FlsGetValue"),
    ("kernel32.dll", "FlsSetValue", "fle_imp_FlsSetValue", "jmp_imp:FlsSetValue"),
    ("kernel32.dll", "GetCommandLineA", "fle_imp_GetCommandLineA", "jmp_imp:GetCommandLineA"),
    ("kernel32.dll", "GetEnvironmentStringsW", "fle_imp_GetEnvironmentStringsW", "jmp_imp:GetEnvironmentStringsW"),
    ("kernel32.dll", "FreeEnvironmentStringsW", "fle_imp_FreeEnvironmentStringsW", "jmp_imp:FreeEnvironmentStringsW"),
    ("kernel32.dll", "GetOEMCP", "fle_imp_GetOEMCP", "jmp_imp:GetOEMCP"),
    ("kernel32.dll", "IsValidCodePage", "fle_imp_IsValidCodePage", "jmp_imp:IsValidCodePage"),
    // The same DLL's names that are called only on error, exception, console or file-descriptor paths (or where no value is defined) stop by
    // name when called (decision 0078): runtime/fle_proc.c, fle_vol.c, fle_sys.c, fle_locale.c, fle_modules.c.
    ("kernel32.dll", "RtlCaptureContext", "fle_imp_RtlCaptureContext", "jmp_imp:RtlCaptureContext"),
    ("kernel32.dll", "RtlLookupFunctionEntry", "fle_imp_RtlLookupFunctionEntry", "jmp_imp:RtlLookupFunctionEntry"),
    ("kernel32.dll", "RtlVirtualUnwind", "fle_imp_RtlVirtualUnwind", "jmp_imp:RtlVirtualUnwind"),
    ("kernel32.dll", "RtlUnwindEx", "fle_imp_RtlUnwindEx", "jmp_imp:RtlUnwindEx"),
    ("kernel32.dll", "RtlPcToFileHeader", "fle_imp_RtlPcToFileHeader", "jmp_imp:RtlPcToFileHeader"),
    ("kernel32.dll", "UnhandledExceptionFilter", "fle_imp_UnhandledExceptionFilter", "jmp_imp:UnhandledExceptionFilter"),
    ("kernel32.dll", "SetUnhandledExceptionFilter", "fle_imp_SetUnhandledExceptionFilter", "jmp_imp:SetUnhandledExceptionFilter"),
    ("kernel32.dll", "IsDebuggerPresent", "fle_imp_IsDebuggerPresent", "jmp_imp:IsDebuggerPresent"),
    ("kernel32.dll", "EncodePointer", "fle_imp_EncodePointer", "jmp_imp:EncodePointer"),
    ("kernel32.dll", "GetFileType", "fle_imp_GetFileType", "jmp_imp:GetFileType"),
    ("kernel32.dll", "SetStdHandle", "fle_imp_SetStdHandle", "jmp_imp:SetStdHandle"),
    ("kernel32.dll", "GetConsoleMode", "fle_imp_GetConsoleMode", "jmp_imp:GetConsoleMode"),
    ("kernel32.dll", "GetConsoleOutputCP", "fle_imp_GetConsoleOutputCP", "jmp_imp:GetConsoleOutputCP"),
    ("kernel32.dll", "WriteConsoleW", "fle_imp_WriteConsoleW", "jmp_imp:WriteConsoleW"),
    ("kernel32.dll", "FlushFileBuffers", "fle_imp_FlushFileBuffers", "jmp_imp:FlushFileBuffers"),
    ("kernel32.dll", "SetFilePointerEx", "fle_imp_SetFilePointerEx", "jmp_imp:SetFilePointerEx"),
    ("kernel32.dll", "FindFirstFileExW", "fle_imp_FindFirstFileExW", "jmp_imp:FindFirstFileExW"),
    ("kernel32.dll", "HeapSize", "fle_imp_HeapSize", "jmp_imp:HeapSize"),
    ("kernel32.dll", "HeapReAlloc", "fle_imp_HeapReAlloc", "jmp_imp:HeapReAlloc"),
    ("kernel32.dll", "GetStringTypeW", "fle_imp_GetStringTypeW", "jmp_imp:GetStringTypeW"),
    ("kernel32.dll", "GetModuleHandleExW", "fle_imp_GetModuleHandleExW", "jmp_imp:GetModuleHandleExW"),
    // GetModuleHandleA (decision 0091): the same module table, handles and rules as GetModuleHandleW; a byte above 0x7F stops. Appended at the
    // end so that no existing row's pseudo-VA moves (decision 0015). runtime/fle_modules.c.
    ("kernel32.dll", "GetModuleHandleA", "fle_imp_GetModuleHandleA", "jmp_imp:GetModuleHandleA"),
    // SRW locks and condition variables (decision 0092, provisional): a state table keyed by the lock's address; Wake does nothing; every
    // SleepConditionVariableSRW stops by name. Appended at the end so that no existing row's pseudo-VA moves (decision 0015). runtime/fle_sys.c.
    ("kernel32.dll", "AcquireSRWLockExclusive", "fle_imp_AcquireSRWLockExclusive", "jmp_imp:AcquireSRWLockExclusive"),
    ("kernel32.dll", "ReleaseSRWLockExclusive", "fle_imp_ReleaseSRWLockExclusive", "jmp_imp:ReleaseSRWLockExclusive"),
    ("kernel32.dll", "AcquireSRWLockShared", "fle_imp_AcquireSRWLockShared", "jmp_imp:AcquireSRWLockShared"),
    ("kernel32.dll", "ReleaseSRWLockShared", "fle_imp_ReleaseSRWLockShared", "jmp_imp:ReleaseSRWLockShared"),
    ("kernel32.dll", "WakeConditionVariable", "fle_imp_WakeConditionVariable", "jmp_imp:WakeConditionVariable"),
    ("kernel32.dll", "WakeAllConditionVariable", "fle_imp_WakeAllConditionVariable", "jmp_imp:WakeAllConditionVariable"),
    ("kernel32.dll", "SleepConditionVariableSRW", "fle_imp_SleepConditionVariableSRW", "jmp_imp:SleepConditionVariableSRW"),
    // The deferred modules (decision 0096, provisional): the export names of blend2d.dll (1) and quickfontcache_x64.dll (10, all of them, in
    // the order of the export directory's ordinals). They are not imports of any lifted image: GetProcAddress on the handle of N = 33 or 34
    // (runtime/fle_modules.c) finds the row by its dll column and hands out the pseudo-VA, and calling it stops by name (fle_imp_deferred_*).
    // Appended at the end so that no existing row's pseudo-VA moves (decision 0015).
    ("blend2d.dll", "AggCreateInstance", "fle_imp_deferred_blend2d_AggCreateInstance", "jmp_imp:AggCreateInstance"),
    ("quickfontcache_x64.dll", "dbkFCallWrapperAddr", "fle_imp_deferred_qfc_dbkFCallWrapperAddr", "jmp_imp:dbkFCallWrapperAddr"),
    ("quickfontcache_x64.dll", "__dbk_fcall_wrapper", "fle_imp_deferred_qfc___dbk_fcall_wrapper", "jmp_imp:__dbk_fcall_wrapper"),
    ("quickfontcache_x64.dll", "InitSharedPython", "fle_imp_deferred_qfc_InitSharedPython", "jmp_imp:InitSharedPython"),
    ("quickfontcache_x64.dll", "CreateVST3Instance", "fle_imp_deferred_qfc_CreateVST3Instance", "jmp_imp:CreateVST3Instance"),
    ("quickfontcache_x64.dll", "LoadSharedElastiqueDll", "fle_imp_deferred_qfc_LoadSharedElastiqueDll", "jmp_imp:LoadSharedElastiqueDll"),
    ("quickfontcache_x64.dll", "AcquireSharedAnimTimer", "fle_imp_deferred_qfc_AcquireSharedAnimTimer", "jmp_imp:AcquireSharedAnimTimer"),
    ("quickfontcache_x64.dll", "AcquireSharedThreadBank", "fle_imp_deferred_qfc_AcquireSharedThreadBank", "jmp_imp:AcquireSharedThreadBank"),
    ("quickfontcache_x64.dll", "FontCache_GetStats", "fle_imp_deferred_qfc_FontCache_GetStats", "jmp_imp:FontCache_GetStats"),
    ("quickfontcache_x64.dll", "FontCache_CleanUp", "fle_imp_deferred_qfc_FontCache_CleanUp", "jmp_imp:FontCache_CleanUp"),
    ("quickfontcache_x64.dll", "FontCache_AcquireFont", "fle_imp_deferred_qfc_FontCache_AcquireFont", "jmp_imp:FontCache_AcquireFont"),
    // Wall responses, first batch (decision 0098, provisional). Appended at the end so that no existing row's pseudo-VA moves (decision 0015).
    // msacm32!acmMetrics (runtime/fle_winmm.c): only (NULL, ACM_METRIC_MAX_SIZE_FORMAT, &dword) is answered; the largest WAVEFORMATEX is 18 bytes.
    ("msacm32.dll", "acmMetrics", "fle_imp_acmMetrics", "jmp_imp:acmMetrics"),
    // shell32!SHGetMalloc (runtime/fle_ole.c) returns S_OK and an IMalloc object over the task allocator. The next four rows are the slots of that
    // object's vtable, not exports: "IMalloc::Release", "IMalloc::Free", "IMalloc::DidAlloc" and "IMalloc::unimplemented" (the other six slots share it).
    // No import thunk can name them; they are rows only so that the lifted code reaches them through fle_dispatch (a GetProcAddress for such a name
    // in shell32.dll would find them too, which no guest does).
    ("shell32.dll", "SHGetMalloc", "fle_imp_SHGetMalloc", "jmp_imp:SHGetMalloc"),
    ("shell32.dll", "IMalloc::Release", "fle_imp_IMalloc_Release", "jmp_imp:IMalloc::Release"),
    ("shell32.dll", "IMalloc::Free", "fle_imp_IMalloc_Free", "jmp_imp:IMalloc::Free"),
    ("shell32.dll", "IMalloc::DidAlloc", "fle_imp_IMalloc_DidAlloc", "jmp_imp:IMalloc::DidAlloc"),
    ("shell32.dll", "IMalloc::unimplemented", "fle_imp_IMalloc_unimplemented", "jmp_imp:IMalloc::unimplemented"),
    // user32!EnumDisplaySettingsW (runtime/fle_monitor.c): only (NULL, ENUM_CURRENT_SETTINGS) is answered, the current mode of the virtual display.
    ("user32.dll", "EnumDisplaySettingsW", "fle_imp_EnumDisplaySettingsW", "jmp_imp:EnumDisplaySettingsW"),
    // user32!GetClipCursor (runtime/fle_cursor.c): the cursor is never confined, so the RECT is the screen.
    ("user32.dll", "GetClipCursor", "fle_imp_GetClipCursor", "jmp_imp:GetClipCursor"),
    // gdi32!EnumFontFamiliesExW (runtime/fle_gdi.c): one virtual family, the system font of decision 0066, one callback call.
    ("gdi32.dll", "EnumFontFamiliesExW", "fle_imp_EnumFontFamiliesExW", "jmp_imp:EnumFontFamiliesExW"),
    // version!GetFileVersionInfoSizeW (runtime/fle_vfs.c): a path that is not in the virtual file system has no version information (0 and 2 or 3).
    ("version.dll", "GetFileVersionInfoSizeW", "fle_imp_GetFileVersionInfoSizeW", "jmp_imp:GetFileVersionInfoSizeW"),
    // Census wall batch 2 (decision 0099, provisional). These are the delay-loaded functions behind the two RaiseException(0xc06d007e / 7f)
    // walls: kernel32!GetSystemDefaultLocaleName (runtime/fle_locale.c) and shcore!GetProcessDpiAwareness (runtime/fle_proc.c; shcore.dll is
    // virtual module N = 35, after the deferred modules of decision 0096). Then the nine shell32 functions that the guest fetches by ordinal
    // (runtime/fle_modules.c: found by GetProcAddress, by ordinal or by name, and stop by name when called). Appended at the end so that no
    // existing row's pseudo-VA moves (decision 0015).
    ("kernel32.dll", "GetSystemDefaultLocaleName", "fle_imp_GetSystemDefaultLocaleName", "jmp_imp:GetSystemDefaultLocaleName"),
    ("shcore.dll", "GetProcessDpiAwareness", "fle_imp_GetProcessDpiAwareness", "jmp_imp:GetProcessDpiAwareness"),
    ("shell32.dll", "ILFindLastID", "fle_imp_ILFindLastID", "jmp_imp:ILFindLastID"),
    ("shell32.dll", "ILRemoveLastID", "fle_imp_ILRemoveLastID", "jmp_imp:ILRemoveLastID"),
    ("shell32.dll", "ILClone", "fle_imp_ILClone", "jmp_imp:ILClone"),
    ("shell32.dll", "ILCloneFirst", "fle_imp_ILCloneFirst", "jmp_imp:ILCloneFirst"),
    ("shell32.dll", "ILIsEqual", "fle_imp_ILIsEqual", "jmp_imp:ILIsEqual"),
    ("shell32.dll", "ILCombine", "fle_imp_ILCombine", "jmp_imp:ILCombine"),
    ("shell32.dll", "ILGetSize", "fle_imp_ILGetSize", "jmp_imp:ILGetSize"),
    ("shell32.dll", "ILGetNext", "fle_imp_ILGetNext", "jmp_imp:ILGetNext"),
    ("shell32.dll", "ILFree", "fle_imp_ILFree", "jmp_imp:ILFree"),
    // Decision 0102 (provisional): answers for the walls of the 4-module measurement configuration, appended at the end (decision 0015).
    ("kernel32.dll", "GetVersionExA", "fle_imp_GetVersionExA", "jmp_imp:GetVersionExA"),
    ("kernel32.dll", "HeapCreate", "fle_imp_HeapCreate", "jmp_imp:HeapCreate"),
    ("kernel32.dll", "HeapSetInformation", "fle_imp_HeapSetInformation", "jmp_imp:HeapSetInformation"),
    ("kernel32.dll", "GetStartupInfoA", "fle_imp_GetStartupInfoA", "jmp_imp:GetStartupInfoA"),
    ("kernel32.dll", "GetModuleFileNameA", "fle_imp_GetModuleFileNameA", "jmp_imp:GetModuleFileNameA"),
    ("kernel32.dll", "GetEnvironmentStrings", "fle_imp_GetEnvironmentStrings", "jmp_imp:GetEnvironmentStrings"),
    ("kernel32.dll", "FreeEnvironmentStringsA", "fle_imp_FreeEnvironmentStringsA", "jmp_imp:FreeEnvironmentStringsA"),
    ("kernel32.dll", "DecodePointer", "fle_imp_DecodePointer", "jmp_imp:DecodePointer"),
    ("kernel32.dll", "SetHandleCount", "fle_imp_SetHandleCount", "jmp_imp:SetHandleCount"),
    // Decision 0103 (provisional): shell32!SHGetFolderPathW (runtime/fle_env.c), the font folder (CSIDL_FONTS) only, which the DllMain of the real
    // module quickfontcache_x64.dll asks for. Appended at the end so that no existing row's pseudo-VA moves (decision 0015). The other rows of
    // decision 0103 (GetVersionExA, GetModuleFileNameA, HeapCreate, HeapSetInformation, GetStartupInfoA, SetHandleCount, DecodePointer) were
    // already appended by decision 0102 and are not repeated.
    ("shell32.dll", "SHGetFolderPathW", "fle_imp_SHGetFolderPathW", "jmp_imp:SHGetFolderPathW"),
    // gdi32!CreateBitmap (runtime/fle_gdi.c, decision 0106): the guest's monochrome cursor mask, no pixels kept.
    ("gdi32.dll", "CreateBitmap", "fle_imp_CreateBitmap", "jmp_imp:CreateBitmap"),
    // Decision 0107 (provisional): shfolder.dll!SHGetFolderPathW, the same function as the shell32 row of decision 0103
    // (runtime/fle_env.c). Unit 830 and 880 fetch it with
    // LoadLibraryW(L"SHFolder.dll") + GetProcAddress and call the result without a NULL test; FLEngine also imports it directly (IAT 0x1971820,
    // thunk 0x4262e0). The handler name differs from the shell32 row so that the two thunks are counted apart. Appended at the end (after the
    // decision 0106 row) so that no existing row's pseudo-VA moves (decision 0015).
    ("shfolder.dll", "SHGetFolderPathW", "fle_imp_SHGetFolderPathW", "jmp_imp:SHFolder.SHGetFolderPathW"),
    // Decision 0108 (provisional): the memory DC model of runtime/fle_gdi.c, 19 rows appended in this order. The first 14 answer (DC, bitmaps, DIB section, selection, colours, brush, halftone palette, system
    // colour 8, icon or cursor from bitmaps); the last 5 (BitBlt, PatBlt, FillRect, GetDIBits, RealizePalette) stop by name. Appended at the end so that no existing row's pseudo-VA moves (decision 0015).
    ("gdi32.dll", "CreateCompatibleDC", "fle_imp_CreateCompatibleDC", "jmp_imp:CreateCompatibleDC"),
    ("gdi32.dll", "DeleteDC", "fle_imp_DeleteDC", "jmp_imp:DeleteDC"),
    ("gdi32.dll", "SelectObject", "fle_imp_SelectObject", "jmp_imp:SelectObject"),
    ("gdi32.dll", "SelectPalette", "fle_imp_SelectPalette", "jmp_imp:SelectPalette"),
    ("gdi32.dll", "CreateCompatibleBitmap", "fle_imp_CreateCompatibleBitmap", "jmp_imp:CreateCompatibleBitmap"),
    ("gdi32.dll", "CreateDIBSection", "fle_imp_CreateDIBSection", "jmp_imp:CreateDIBSection"),
    ("gdi32.dll", "GetDIBColorTable", "fle_imp_GetDIBColorTable", "jmp_imp:GetDIBColorTable"),
    ("gdi32.dll", "SetDIBColorTable", "fle_imp_SetDIBColorTable", "jmp_imp:SetDIBColorTable"),
    ("gdi32.dll", "SetTextColor", "fle_imp_SetTextColor", "jmp_imp:SetTextColor"),
    ("gdi32.dll", "SetBkColor", "fle_imp_SetBkColor", "jmp_imp:SetBkColor"),
    ("gdi32.dll", "CreateBrushIndirect", "fle_imp_CreateBrushIndirect", "jmp_imp:CreateBrushIndirect"),
    ("gdi32.dll", "CreateHalftonePalette", "fle_imp_CreateHalftonePalette", "jmp_imp:CreateHalftonePalette"),
    ("user32.dll", "GetSysColor", "fle_imp_GetSysColor", "jmp_imp:GetSysColor"),
    ("user32.dll", "CreateIconIndirect", "fle_imp_CreateIconIndirect", "jmp_imp:CreateIconIndirect"),
    ("gdi32.dll", "BitBlt", "fle_imp_BitBlt", "jmp_imp:BitBlt"),
    ("gdi32.dll", "PatBlt", "fle_imp_PatBlt", "jmp_imp:PatBlt"),
    ("user32.dll", "FillRect", "fle_imp_FillRect", "jmp_imp:FillRect"),
    ("gdi32.dll", "GetDIBits", "fle_imp_GetDIBits", "jmp_imp:GetDIBits"),
    ("gdi32.dll", "RealizePalette", "fle_imp_RealizePalette", "jmp_imp:RealizePalette"),
    // Decision 0111 (provisional): the gdi32 and user32 calls of the ILTools chain that are not pixels (docs/decisions/0111-gdi-bulk.md). 96 rows appended in this order (decision 0015: no pseudo-VA moves).
    // The first 27 answer (DC attributes, the DC stack, solid brushes, pens, palette entries, icon and cursor objects, the cursor state; runtime/fle_gdi.c); the last 69 stop by name (rasterization, glyphs, regions, metafiles, system data).
    ("gdi32.dll", "MoveToEx", "fle_imp_MoveToEx", "jmp_imp:MoveToEx"),
    ("gdi32.dll", "GetCurrentPositionEx", "fle_imp_GetCurrentPositionEx", "jmp_imp:GetCurrentPositionEx"),
    ("gdi32.dll", "SetBkMode", "fle_imp_SetBkMode", "jmp_imp:SetBkMode"),
    ("gdi32.dll", "SetROP2", "fle_imp_SetROP2", "jmp_imp:SetROP2"),
    ("gdi32.dll", "SetMapMode", "fle_imp_SetMapMode", "jmp_imp:SetMapMode"),
    ("gdi32.dll", "SetStretchBltMode", "fle_imp_SetStretchBltMode", "jmp_imp:SetStretchBltMode"),
    ("gdi32.dll", "GetStretchBltMode", "fle_imp_GetStretchBltMode", "jmp_imp:GetStretchBltMode"),
    ("gdi32.dll", "SetWindowOrgEx", "fle_imp_SetWindowOrgEx", "jmp_imp:SetWindowOrgEx"),
    ("gdi32.dll", "GetWindowOrgEx", "fle_imp_GetWindowOrgEx", "jmp_imp:GetWindowOrgEx"),
    ("gdi32.dll", "SetViewportOrgEx", "fle_imp_SetViewportOrgEx", "jmp_imp:SetViewportOrgEx"),
    ("gdi32.dll", "LPtoDP", "fle_imp_LPtoDP", "jmp_imp:LPtoDP"),
    ("gdi32.dll", "SetBrushOrgEx", "fle_imp_SetBrushOrgEx", "jmp_imp:SetBrushOrgEx"),
    ("gdi32.dll", "GetBrushOrgEx", "fle_imp_GetBrushOrgEx", "jmp_imp:GetBrushOrgEx"),
    ("gdi32.dll", "SaveDC", "fle_imp_SaveDC", "jmp_imp:SaveDC"),
    ("gdi32.dll", "RestoreDC", "fle_imp_RestoreDC", "jmp_imp:RestoreDC"),
    ("gdi32.dll", "GdiFlush", "fle_imp_GdiFlush", "jmp_imp:GdiFlush"),
    ("gdi32.dll", "GetClipBox", "fle_imp_GetClipBox", "jmp_imp:GetClipBox"),
    ("gdi32.dll", "UnrealizeObject", "fle_imp_UnrealizeObject", "jmp_imp:UnrealizeObject"),
    ("gdi32.dll", "CreateSolidBrush", "fle_imp_CreateSolidBrush", "jmp_imp:CreateSolidBrush"),
    ("gdi32.dll", "CreatePenIndirect", "fle_imp_CreatePenIndirect", "jmp_imp:CreatePenIndirect"),
    ("gdi32.dll", "GetPaletteEntries", "fle_imp_GetPaletteEntries", "jmp_imp:GetPaletteEntries"),
    ("user32.dll", "DestroyIcon", "fle_imp_DestroyIcon", "jmp_imp:DestroyIcon"),
    ("user32.dll", "DestroyCursor", "fle_imp_DestroyCursor", "jmp_imp:DestroyCursor"),
    ("user32.dll", "GetIconInfo", "fle_imp_GetIconInfo", "jmp_imp:GetIconInfo"),
    ("user32.dll", "SetCursor", "fle_imp_SetCursor", "jmp_imp:SetCursor"),
    ("user32.dll", "GetCursor", "fle_imp_GetCursor", "jmp_imp:GetCursor"),
    ("user32.dll", "ShowCursor", "fle_imp_ShowCursor", "jmp_imp:ShowCursor"),
    ("gdi32.dll", "LineTo", "fle_imp_LineTo", "jmp_imp:LineTo"),
    ("gdi32.dll", "Rectangle", "fle_imp_Rectangle", "jmp_imp:Rectangle"),
    ("gdi32.dll", "RoundRect", "fle_imp_RoundRect", "jmp_imp:RoundRect"),
    ("gdi32.dll", "Ellipse", "fle_imp_Ellipse", "jmp_imp:Ellipse"),
    ("gdi32.dll", "Pie", "fle_imp_Pie", "jmp_imp:Pie"),
    ("gdi32.dll", "Chord", "fle_imp_Chord", "jmp_imp:Chord"),
    ("gdi32.dll", "Arc", "fle_imp_Arc", "jmp_imp:Arc"),
    ("gdi32.dll", "ArcTo", "fle_imp_ArcTo", "jmp_imp:ArcTo"),
    ("gdi32.dll", "AngleArc", "fle_imp_AngleArc", "jmp_imp:AngleArc"),
    ("gdi32.dll", "Polyline", "fle_imp_Polyline", "jmp_imp:Polyline"),
    ("gdi32.dll", "Polygon", "fle_imp_Polygon", "jmp_imp:Polygon"),
    ("gdi32.dll", "PolyBezier", "fle_imp_PolyBezier", "jmp_imp:PolyBezier"),
    ("gdi32.dll", "PolyBezierTo", "fle_imp_PolyBezierTo", "jmp_imp:PolyBezierTo"),
    ("gdi32.dll", "ExtFloodFill", "fle_imp_ExtFloodFill", "jmp_imp:ExtFloodFill"),
    ("gdi32.dll", "FrameRgn", "fle_imp_FrameRgn", "jmp_imp:FrameRgn"),
    ("gdi32.dll", "GetPixel", "fle_imp_GetPixel", "jmp_imp:GetPixel"),
    ("gdi32.dll", "SetPixel", "fle_imp_SetPixel", "jmp_imp:SetPixel"),
    ("gdi32.dll", "SetPixelV", "fle_imp_SetPixelV", "jmp_imp:SetPixelV"),
    ("gdi32.dll", "GetBitmapBits", "fle_imp_GetBitmapBits", "jmp_imp:GetBitmapBits"),
    ("gdi32.dll", "SetDIBits", "fle_imp_SetDIBits", "jmp_imp:SetDIBits"),
    ("gdi32.dll", "SetDIBitsToDevice", "fle_imp_SetDIBitsToDevice", "jmp_imp:SetDIBitsToDevice"),
    ("gdi32.dll", "StretchDIBits", "fle_imp_StretchDIBits", "jmp_imp:StretchDIBits"),
    ("gdi32.dll", "StretchBlt", "fle_imp_StretchBlt", "jmp_imp:StretchBlt"),
    ("gdi32.dll", "MaskBlt", "fle_imp_MaskBlt", "jmp_imp:MaskBlt"),
    ("gdi32.dll", "CreateDIBitmap", "fle_imp_CreateDIBitmap", "jmp_imp:CreateDIBitmap"),
    ("gdi32.dll", "ExtTextOutW", "fle_imp_ExtTextOutW", "jmp_imp:ExtTextOutW"),
    ("gdi32.dll", "GetTextMetricsW", "fle_imp_GetTextMetricsW", "jmp_imp:GetTextMetricsW"),
    ("gdi32.dll", "GetTextExtentPoint32W", "fle_imp_GetTextExtentPoint32W", "jmp_imp:GetTextExtentPoint32W"),
    ("gdi32.dll", "GetTextExtentPointW", "fle_imp_GetTextExtentPointW", "jmp_imp:GetTextExtentPointW"),
    ("gdi32.dll", "SelectClipRgn", "fle_imp_SelectClipRgn", "jmp_imp:SelectClipRgn"),
    ("gdi32.dll", "ExtSelectClipRgn", "fle_imp_ExtSelectClipRgn", "jmp_imp:ExtSelectClipRgn"),
    ("gdi32.dll", "IntersectClipRect", "fle_imp_IntersectClipRect", "jmp_imp:IntersectClipRect"),
    ("gdi32.dll", "ExcludeClipRect", "fle_imp_ExcludeClipRect", "jmp_imp:ExcludeClipRect"),
    ("gdi32.dll", "GetClipRgn", "fle_imp_GetClipRgn", "jmp_imp:GetClipRgn"),
    ("gdi32.dll", "RectVisible", "fle_imp_RectVisible", "jmp_imp:RectVisible"),
    ("gdi32.dll", "GetRandomRgn", "fle_imp_GetRandomRgn", "jmp_imp:GetRandomRgn"),
    ("gdi32.dll", "CreateRectRgn", "fle_imp_CreateRectRgn", "jmp_imp:CreateRectRgn"),
    ("gdi32.dll", "CreateRectRgnIndirect", "fle_imp_CreateRectRgnIndirect", "jmp_imp:CreateRectRgnIndirect"),
    ("gdi32.dll", "SetRectRgn", "fle_imp_SetRectRgn", "jmp_imp:SetRectRgn"),
    ("gdi32.dll", "CombineRgn", "fle_imp_CombineRgn", "jmp_imp:CombineRgn"),
    ("gdi32.dll", "GetRgnBox", "fle_imp_GetRgnBox", "jmp_imp:GetRgnBox"),
    ("gdi32.dll", "OffsetRgn", "fle_imp_OffsetRgn", "jmp_imp:OffsetRgn"),
    ("gdi32.dll", "EqualRgn", "fle_imp_EqualRgn", "jmp_imp:EqualRgn"),
    ("gdi32.dll", "RectInRegion", "fle_imp_RectInRegion", "jmp_imp:RectInRegion"),
    ("gdi32.dll", "GetRegionData", "fle_imp_GetRegionData", "jmp_imp:GetRegionData"),
    ("gdi32.dll", "GetNearestPaletteIndex", "fle_imp_GetNearestPaletteIndex", "jmp_imp:GetNearestPaletteIndex"),
    ("gdi32.dll", "ResizePalette", "fle_imp_ResizePalette", "jmp_imp:ResizePalette"),
    ("gdi32.dll", "GetSystemPaletteEntries", "fle_imp_GetSystemPaletteEntries", "jmp_imp:GetSystemPaletteEntries"),
    ("gdi32.dll", "GetDCOrgEx", "fle_imp_GetDCOrgEx", "jmp_imp:GetDCOrgEx"),
    ("gdi32.dll", "GetEnhMetaFileHeader", "fle_imp_GetEnhMetaFileHeader", "jmp_imp:GetEnhMetaFileHeader"),
    ("gdi32.dll", "PlayEnhMetaFile", "fle_imp_PlayEnhMetaFile", "jmp_imp:PlayEnhMetaFile"),
    ("gdi32.dll", "GetEnhMetaFileBits", "fle_imp_GetEnhMetaFileBits", "jmp_imp:GetEnhMetaFileBits"),
    ("gdi32.dll", "GetEnhMetaFilePaletteEntries", "fle_imp_GetEnhMetaFilePaletteEntries", "jmp_imp:GetEnhMetaFilePaletteEntries"),
    ("gdi32.dll", "GetWinMetaFileBits", "fle_imp_GetWinMetaFileBits", "jmp_imp:GetWinMetaFileBits"),
    ("gdi32.dll", "GetEnhMetaFileDescriptionW", "fle_imp_GetEnhMetaFileDescriptionW", "jmp_imp:GetEnhMetaFileDescriptionW"),
    ("gdi32.dll", "DeleteEnhMetaFile", "fle_imp_DeleteEnhMetaFile", "jmp_imp:DeleteEnhMetaFile"),
    ("gdi32.dll", "SetEnhMetaFileBits", "fle_imp_SetEnhMetaFileBits", "jmp_imp:SetEnhMetaFileBits"),
    ("gdi32.dll", "CopyEnhMetaFileW", "fle_imp_CopyEnhMetaFileW", "jmp_imp:CopyEnhMetaFileW"),
    ("gdi32.dll", "SetWinMetaFileBits", "fle_imp_SetWinMetaFileBits", "jmp_imp:SetWinMetaFileBits"),
    ("user32.dll", "FrameRect", "fle_imp_FrameRect", "jmp_imp:FrameRect"),
    ("user32.dll", "DrawEdge", "fle_imp_DrawEdge", "jmp_imp:DrawEdge"),
    ("user32.dll", "DrawFocusRect", "fle_imp_DrawFocusRect", "jmp_imp:DrawFocusRect"),
    ("user32.dll", "DrawFrameControl", "fle_imp_DrawFrameControl", "jmp_imp:DrawFrameControl"),
    ("user32.dll", "DrawTextW", "fle_imp_DrawTextW", "jmp_imp:DrawTextW"),
    ("user32.dll", "DrawTextExW", "fle_imp_DrawTextExW", "jmp_imp:DrawTextExW"),
    ("user32.dll", "DrawIcon", "fle_imp_DrawIcon", "jmp_imp:DrawIcon"),
    ("user32.dll", "DrawIconEx", "fle_imp_DrawIconEx", "jmp_imp:DrawIconEx"),
    ("user32.dll", "CreateIcon", "fle_imp_CreateIcon", "jmp_imp:CreateIcon"),
    ("user32.dll", "LoadBitmapW", "fle_imp_LoadBitmapW", "jmp_imp:LoadBitmapW"),
    // Decision 0112 (provisional): the machine-name wall of the census (units 830 and 880: kernel32!GetComputerNameExW, 26 call sites) and its neighbours, appended in this
    // order so that no existing row's pseudo-VA moves (decision 0015). GetComputerNameExW and GetComputerNameW (runtime/fle_env.c; both are imported by FLEngine), the
    // two version.dll calls behind GetFileVersionInfoSizeW (runtime/fle_vfs.c; GetFileVersionInfoW fails as the size call does, VerQueryValueW always stops by name),
    // kernel32!LocaleNameToLCID (runtime/fle_locale.c; a delay-loaded import of FLEngine, reached through GetProcAddress), and the two rows that no module imports at
    // HEAD (advapi32!GetUserNameW, kernel32!GetComputerNameExA; runtime/fle_env.c), kept last so that they can be dropped without moving the others.
    ("kernel32.dll", "GetComputerNameExW", "fle_imp_GetComputerNameExW", "jmp_imp:GetComputerNameExW"),
    ("kernel32.dll", "GetComputerNameW", "fle_imp_GetComputerNameW", "jmp_imp:GetComputerNameW"),
    ("version.dll", "GetFileVersionInfoW", "fle_imp_GetFileVersionInfoW", "jmp_imp:GetFileVersionInfoW"),
    ("version.dll", "VerQueryValueW", "fle_imp_VerQueryValueW", "jmp_imp:VerQueryValueW"),
    ("kernel32.dll", "LocaleNameToLCID", "fle_imp_LocaleNameToLCID", "jmp_imp:LocaleNameToLCID"),
    ("advapi32.dll", "GetUserNameW", "fle_imp_GetUserNameW", "jmp_imp:GetUserNameW"),
    ("kernel32.dll", "GetComputerNameExA", "fle_imp_GetComputerNameExA", "jmp_imp:GetComputerNameExA"),
    // Decision 0116 (provisional): the walls of CreateFruityInstance (docs/decisions/0116-fruity-walls.md), appended in the order they were reached.
    // Wall 2: the Delphi helper f_63ab90 of FLEngine, SHGetSpecialFolderLocation + SHGetPathFromIDListW (runtime/fle_env.c; the PIDL is a CoTaskMemAlloc block).
    ("shell32.dll", "SHGetSpecialFolderLocation", "fle_imp_SHGetSpecialFolderLocation", "jmp_imp:SHGetSpecialFolderLocation"),
    ("shell32.dll", "SHGetPathFromIDListW", "fle_imp_SHGetPathFromIDListW", "jmp_imp:SHGetPathFromIDListW"),
    // Decision 0117 (provisional): wall 4 of CreateFruityInstance, the engine's check of its own DLL's Authenticode signature (docs/decisions/0117-winverifytrust-and-walls.md),
    // appended in the order the guest reaches them (runtime/fle_vfs.c). WinVerifyTrust answers 0 only for a path that has an ESTABLISHED claim in the table that the harness
    // registers (harness/authenticode_claims.h); with an empty table (every build whose harness registers nothing) every one of them stops by name.
    ("wintrust.dll", "WinVerifyTrust", "fle_imp_WinVerifyTrust", "jmp_imp:WinVerifyTrust"),
    ("crypt32.dll", "CryptQueryObject", "fle_imp_CryptQueryObject", "jmp_imp:CryptQueryObject"),
    ("crypt32.dll", "CryptMsgGetParam", "fle_imp_CryptMsgGetParam", "jmp_imp:CryptMsgGetParam"),
    ("crypt32.dll", "CertFindCertificateInStore", "fle_imp_CertFindCertificateInStore", "jmp_imp:CertFindCertificateInStore"),
    ("crypt32.dll", "CertGetNameStringW", "fle_imp_CertGetNameStringW", "jmp_imp:CertGetNameStringW"),
    ("crypt32.dll", "CertFreeCertificateContext", "fle_imp_CertFreeCertificateContext", "jmp_imp:CertFreeCertificateContext"),
    ("crypt32.dll", "CertCloseStore", "fle_imp_CertCloseStore", "jmp_imp:CertCloseStore"),
    ("crypt32.dll", "CryptMsgClose", "fle_imp_CryptMsgClose", "jmp_imp:CryptMsgClose"),
    // Wall 5 (after the signature check): the Delphi helper f_9df190 gets the user's UI language through a delay-loaded kernel32!GetUserPreferredUILanguages (runtime/fle_uipref.c).
    ("kernel32.dll", "GetUserPreferredUILanguages", "fle_imp_GetUserPreferredUILanguages", "jmp_imp:GetUserPreferredUILanguages"),
    // Decision 0118 wall 7: CreateFruityInstance's helper f_11b2450 delay-loads rpcrt4!UuidCreate (rpcrt4.dll is virtual module N = 36; runtime/fle_proc.c).
    ("rpcrt4.dll", "UuidCreate", "fle_imp_UuidCreate", "jmp_imp:UuidCreate"),
    // Decision 0119 (provisional) wall 9: user32!SendMessageW and SetClassLongPtrW (runtime/fle_win.c). A hWnd that is not a live window of the virtual table (NULL) answers 0 and last error 1400 (a choice without a source).
    ("user32.dll", "SendMessageW", "fle_imp_SendMessageW", "jmp_imp:SendMessageW"),
    ("user32.dll", "SetClassLongPtrW", "fle_imp_SetClassLongPtrW", "jmp_imp:SetClassLongPtrW"),
    // Decision 0119 wall 11: the TApplication window delay-loads wtsapi32!WTSRegisterSessionNotification (wtsapi32.dll is virtual module N = 37; runtime/fle_win.c).
    ("wtsapi32.dll", "WTSRegisterSessionNotification", "fle_imp_WTSRegisterSessionNotification", "jmp_imp:WTSRegisterSessionNotification"),
    // Decision 0119 wall 12: the TApplication window delay-loads uxtheme!BufferedPaintInit (uxtheme.dll is virtual module N = 38; runtime/fle_win.c).
    ("uxtheme.dll", "BufferedPaintInit", "fle_imp_BufferedPaintInit", "jmp_imp:BufferedPaintInit"),
    // Decision 0119 wall 16: the TApplication window takes the system menu copy and deletes SC_MAXIMIZE, SC_SIZE, SC_MOVE (runtime/fle_win.c).
    ("user32.dll", "GetSystemMenu", "fle_imp_GetSystemMenu", "jmp_imp:GetSystemMenu"),
    ("user32.dll", "DeleteMenu", "fle_imp_DeleteMenu", "jmp_imp:DeleteMenu"),
    // Decision 0119 wall 17: the TApplication window reads and writes its extended style and sets its text (runtime/fle_win.c).
    ("user32.dll", "GetWindowLongPtrW", "fle_imp_GetWindowLongPtrW", "jmp_imp:GetWindowLongPtrW"),
    ("user32.dll", "SetWindowTextW", "fle_imp_SetWindowTextW", "jmp_imp:SetWindowTextW"),
    // Decision 0122 (provisional): cooperative guest threads (runtime/fle_thread.c) and the message queues (runtime/fle_win.c), 15 rows appended in the order the walls reach them
    // (decision 0015: no pseudo-VA moves). CreateThread, ResumeThread, SuspendThread, ExitThread, GetExitCodeThread and WaitForMultipleObjects are kernel32 (fle_thread.c); the
    // message functions are user32 (fle_win.c); DwmFlush is dwmapi (virtual module N = 39, fle_modules.c; fle_thread.c).
    ("kernel32.dll", "CreateThread", "fle_imp_CreateThread", "jmp_imp:CreateThread"),
    ("kernel32.dll", "ResumeThread", "fle_imp_ResumeThread", "jmp_imp:ResumeThread"),
    ("kernel32.dll", "SuspendThread", "fle_imp_SuspendThread", "jmp_imp:SuspendThread"),
    ("kernel32.dll", "ExitThread", "fle_imp_ExitThread", "jmp_imp:ExitThread"),
    ("kernel32.dll", "GetExitCodeThread", "fle_imp_GetExitCodeThread", "jmp_imp:GetExitCodeThread"),
    ("dwmapi.dll", "DwmFlush", "fle_imp_DwmFlush", "jmp_imp:DwmFlush"),
    ("user32.dll", "PostMessageW", "fle_imp_PostMessageW", "jmp_imp:PostMessageW"),
    ("user32.dll", "GetMessageW", "fle_imp_GetMessageW", "jmp_imp:GetMessageW"),
    ("user32.dll", "PeekMessageW", "fle_imp_PeekMessageW", "jmp_imp:PeekMessageW"),
    ("user32.dll", "WaitMessage", "fle_imp_WaitMessage", "jmp_imp:WaitMessage"),
    ("user32.dll", "PostQuitMessage", "fle_imp_PostQuitMessage", "jmp_imp:PostQuitMessage"),
    ("user32.dll", "TranslateMessage", "fle_imp_TranslateMessage", "jmp_imp:TranslateMessage"),
    ("user32.dll", "DispatchMessageW", "fle_imp_DispatchMessageW", "jmp_imp:DispatchMessageW"),
    ("kernel32.dll", "WaitForMultipleObjects", "fle_imp_WaitForMultipleObjects", "jmp_imp:WaitForMultipleObjects"),
    ("user32.dll", "MsgWaitForMultipleObjects", "fle_imp_MsgWaitForMultipleObjects", "jmp_imp:MsgWaitForMultipleObjects"),
    // Decision 0123 (provisional): window management, a state-only model (runtime/fle_win.c; UnregisterClassW in runtime/fle_class.c). 49 rows appended in the order of decision 0123's list (decision 0015: no pseudo-VA moves).
    ("user32.dll", "IsWindowVisible", "fle_imp_IsWindowVisible", "jmp_imp:IsWindowVisible"),
    ("user32.dll", "ShowWindow", "fle_imp_ShowWindow", "jmp_imp:ShowWindow"),
    ("user32.dll", "SetWindowPos", "fle_imp_SetWindowPos", "jmp_imp:SetWindowPos"),
    ("user32.dll", "MoveWindow", "fle_imp_MoveWindow", "jmp_imp:MoveWindow"),
    ("user32.dll", "GetWindowRect", "fle_imp_GetWindowRect", "jmp_imp:GetWindowRect"),
    ("user32.dll", "GetClientRect", "fle_imp_GetClientRect", "jmp_imp:GetClientRect"),
    ("user32.dll", "ClientToScreen", "fle_imp_ClientToScreen", "jmp_imp:ClientToScreen"),
    ("user32.dll", "ScreenToClient", "fle_imp_ScreenToClient", "jmp_imp:ScreenToClient"),
    ("user32.dll", "MapWindowPoints", "fle_imp_MapWindowPoints", "jmp_imp:MapWindowPoints"),
    ("user32.dll", "EnableWindow", "fle_imp_EnableWindow", "jmp_imp:EnableWindow"),
    ("user32.dll", "IsWindowEnabled", "fle_imp_IsWindowEnabled", "jmp_imp:IsWindowEnabled"),
    ("user32.dll", "IsWindow", "fle_imp_IsWindow", "jmp_imp:IsWindow"),
    ("user32.dll", "GetParent", "fle_imp_GetParent", "jmp_imp:GetParent"),
    ("user32.dll", "GetWindow", "fle_imp_GetWindow", "jmp_imp:GetWindow"),
    ("user32.dll", "SetParent", "fle_imp_SetParent", "jmp_imp:SetParent"),
    ("user32.dll", "GetAncestor", "fle_imp_GetAncestor", "jmp_imp:GetAncestor"),
    ("user32.dll", "GetWindowThreadProcessId", "fle_imp_GetWindowThreadProcessId", "jmp_imp:GetWindowThreadProcessId"),
    ("user32.dll", "SetFocus", "fle_imp_SetFocus", "jmp_imp:SetFocus"),
    ("user32.dll", "GetFocus", "fle_imp_GetFocus", "jmp_imp:GetFocus"),
    ("user32.dll", "SetActiveWindow", "fle_imp_SetActiveWindow", "jmp_imp:SetActiveWindow"),
    ("user32.dll", "GetActiveWindow", "fle_imp_GetActiveWindow", "jmp_imp:GetActiveWindow"),
    ("user32.dll", "GetForegroundWindow", "fle_imp_GetForegroundWindow", "jmp_imp:GetForegroundWindow"),
    ("user32.dll", "SetForegroundWindow", "fle_imp_SetForegroundWindow", "jmp_imp:SetForegroundWindow"),
    ("user32.dll", "SetCapture", "fle_imp_SetCapture", "jmp_imp:SetCapture"),
    ("user32.dll", "GetCapture", "fle_imp_GetCapture", "jmp_imp:GetCapture"),
    ("user32.dll", "ReleaseCapture", "fle_imp_ReleaseCapture", "jmp_imp:ReleaseCapture"),
    ("user32.dll", "WindowFromPoint", "fle_imp_WindowFromPoint", "jmp_imp:WindowFromPoint"),
    ("user32.dll", "GetCursorPos", "fle_imp_GetCursorPos", "jmp_imp:GetCursorPos"),
    ("user32.dll", "FindWindowW", "fle_imp_FindWindowW", "jmp_imp:FindWindowW"),
    ("user32.dll", "UnregisterClassW", "fle_imp_UnregisterClassW", "jmp_imp:UnregisterClassW"),
    ("user32.dll", "SetTimer", "fle_imp_SetTimer", "jmp_imp:SetTimer"),
    ("user32.dll", "KillTimer", "fle_imp_KillTimer", "jmp_imp:KillTimer"),
    ("user32.dll", "InvalidateRect", "fle_imp_InvalidateRect", "jmp_imp:InvalidateRect"),
    ("user32.dll", "ValidateRect", "fle_imp_ValidateRect", "jmp_imp:ValidateRect"),
    ("user32.dll", "UpdateWindow", "fle_imp_UpdateWindow", "jmp_imp:UpdateWindow"),
    ("user32.dll", "BeginPaint", "fle_imp_BeginPaint", "jmp_imp:BeginPaint"),
    ("user32.dll", "EndPaint", "fle_imp_EndPaint", "jmp_imp:EndPaint"),
    ("user32.dll", "EnumWindows", "fle_imp_EnumWindows", "jmp_imp:EnumWindows"),
    ("user32.dll", "DestroyWindow", "fle_imp_DestroyWindow", "jmp_imp:DestroyWindow"),
    ("user32.dll", "IsChild", "fle_imp_IsChild", "jmp_imp:IsChild"),
    ("user32.dll", "IsIconic", "fle_imp_IsIconic", "jmp_imp:IsIconic"),
    ("user32.dll", "IsZoomed", "fle_imp_IsZoomed", "jmp_imp:IsZoomed"),
    ("user32.dll", "GetDesktopWindow", "fle_imp_GetDesktopWindow", "jmp_imp:GetDesktopWindow"),
    ("user32.dll", "GetTopWindow", "fle_imp_GetTopWindow", "jmp_imp:GetTopWindow"),
    ("user32.dll", "BringWindowToTop", "fle_imp_BringWindowToTop", "jmp_imp:BringWindowToTop"),
    ("user32.dll", "GetWindowTextW", "fle_imp_GetWindowTextW", "jmp_imp:GetWindowTextW"),
    ("user32.dll", "EnumChildWindows", "fle_imp_EnumChildWindows", "jmp_imp:EnumChildWindows"),
    ("user32.dll", "EnumThreadWindows", "fle_imp_EnumThreadWindows", "jmp_imp:EnumThreadWindows"),
    ("user32.dll", "GetUpdateRect", "fle_imp_GetUpdateRect", "jmp_imp:GetUpdateRect"),
    // Decision 0123 wall 26: FreeResource (runtime/fle_res.c) always returns FALSE.
    ("kernel32.dll", "FreeResource", "fle_imp_FreeResource", "jmp_imp:FreeResource"),
    // Decision 0123 wall 29: the VCL window procedure asks IsWindowUnicode; GetClassNameW is its sibling (runtime/fle_win.c).
    ("user32.dll", "IsWindowUnicode", "fle_imp_IsWindowUnicode", "jmp_imp:IsWindowUnicode"),
    ("user32.dll", "GetClassNameW", "fle_imp_GetClassNameW", "jmp_imp:GetClassNameW"),
    // Decision 0123 wall 30: the VCL window procedure keeps its object in the window's property list (R7).
    ("user32.dll", "SetPropW", "fle_imp_SetPropW", "jmp_imp:SetPropW"),
    ("user32.dll", "GetPropW", "fle_imp_GetPropW", "jmp_imp:GetPropW"),
    ("user32.dll", "RemovePropW", "fle_imp_RemovePropW", "jmp_imp:RemovePropW"),
    // Decision 0123 wall 32: the VCL window procedure passes the messages it does not handle to the previous procedure.
    ("user32.dll", "CallWindowProcW", "fle_imp_CallWindowProcW", "jmp_imp:CallWindowProcW"),
    // Decision 0123 wall 33: the form's Menu property (NULL only: menus are not modelled).
    ("user32.dll", "SetMenu", "fle_imp_SetMenu", "jmp_imp:SetMenu"),
    ("user32.dll", "GetMenu", "fle_imp_GetMenu", "jmp_imp:GetMenu"),
    ("user32.dll", "DrawMenuBar", "fle_imp_DrawMenuBar", "jmp_imp:DrawMenuBar"),
    // Decision 0123 wall 36: flat scroll bars (comctl32, runtime/fle_win.c, state only).
    ("comctl32.dll", "InitializeFlatSB", "fle_imp_InitializeFlatSB", "jmp_imp:InitializeFlatSB"),
    ("comctl32.dll", "FlatSB_SetScrollProp", "fle_imp_FlatSB_SetScrollProp", "jmp_imp:FlatSB_SetScrollProp"),
    // Decision 0123 wall 37: the rest of the flat scroll bar family and the standard scroll bar information (state only).
    ("comctl32.dll", "FlatSB_SetScrollInfo", "fle_imp_FlatSB_SetScrollInfo", "jmp_imp:FlatSB_SetScrollInfo"),
    ("comctl32.dll", "FlatSB_GetScrollInfo", "fle_imp_FlatSB_GetScrollInfo", "jmp_imp:FlatSB_GetScrollInfo"),
    ("comctl32.dll", "FlatSB_SetScrollPos", "fle_imp_FlatSB_SetScrollPos", "jmp_imp:FlatSB_SetScrollPos"),
    ("comctl32.dll", "FlatSB_GetScrollPos", "fle_imp_FlatSB_GetScrollPos", "jmp_imp:FlatSB_GetScrollPos"),
    ("user32.dll", "SetScrollInfo", "fle_imp_SetScrollInfo", "jmp_imp:SetScrollInfo"),
    ("user32.dll", "GetScrollInfo", "fle_imp_GetScrollInfo", "jmp_imp:GetScrollInfo"),
    // Decision 0123 wall 38: the show state and the restored rectangle of a window (runtime/fle_win.c, state only).
    ("user32.dll", "GetWindowPlacement", "fle_imp_GetWindowPlacement", "jmp_imp:GetWindowPlacement"),
    // Decision 0123 wall 39: positioning several windows at once (runtime/fle_win.c).
    ("user32.dll", "BeginDeferWindowPos", "fle_imp_BeginDeferWindowPos", "jmp_imp:BeginDeferWindowPos"),
    ("user32.dll", "DeferWindowPos", "fle_imp_DeferWindowPos", "jmp_imp:DeferWindowPos"),
    ("user32.dll", "EndDeferWindowPos", "fle_imp_EndDeferWindowPos", "jmp_imp:EndDeferWindowPos"),
    // Decision 0123 wall 44: RedrawWindow (runtime/fle_win.c, state and WM_PAINT/WM_ERASEBKGND only).
    ("user32.dll", "RedrawWindow", "fle_imp_RedrawWindow", "jmp_imp:RedrawWindow"),
    // Decision 0123 wall 49: PtInRegion (runtime/fle_gdi.c; the region objects: CreateRectRgn ... CombineRgn are rows of decision 0111 that now answer).
    ("gdi32.dll", "PtInRegion", "fle_imp_PtInRegion", "jmp_imp:PtInRegion"),
    // Decision 0125 (PROVISIONAL): the DCs of a window, state only (runtime/fle_gdc.c): GetWindowDC and GetDCEx (GetDC and ReleaseDC are older rows and now take a window). Appended at the end: no pseudo-VA moves (decision 0015).
    ("user32.dll", "GetWindowDC", "fle_imp_GetWindowDC", "jmp_imp:GetWindowDC"),
    ("user32.dll", "GetDCEx", "fle_imp_GetDCEx", "jmp_imp:GetDCEx"),
    // Decision 0126 (PROVISIONAL): the pixel model (runtime/fle_raster.c, fle_gdi.c). gdi32!CreatePen is the one new row: FLEngine imports it (a pen made by CreatePen is selected into a DC that LineTo, Rectangle and
    // the other drawing calls then use); every other drawing call of the chain has a row from decisions 0108, 0111 or 0113 whose body now draws. Appended at the end: no pseudo-VA moves (decision 0015).
    ("gdi32.dll", "CreatePen", "fle_imp_CreatePen", "jmp_imp:CreatePen"),
    // Decision 0126 wall 61: the key state (runtime/fle_win.c): no input exists, every key is up.
    ("user32.dll", "GetKeyState", "fle_imp_GetKeyState", "jmp_imp:GetKeyState"),
    ("user32.dll", "GetAsyncKeyState", "fle_imp_GetAsyncKeyState", "jmp_imp:GetAsyncKeyState"),
    // Decision 0126 wall 63: CheckMenuItem and EnableMenuItem of a menu that does not exist (runtime/fle_win.c).
    ("user32.dll", "CheckMenuItem", "fle_imp_CheckMenuItem", "jmp_imp:CheckMenuItem"),
    ("user32.dll", "EnableMenuItem", "fle_imp_EnableMenuItem", "jmp_imp:EnableMenuItem"),
    // Decision 0126 wall 64: LockWindowUpdate (runtime/fle_win.c).
    ("user32.dll", "LockWindowUpdate", "fle_imp_LockWindowUpdate", "jmp_imp:LockWindowUpdate"),
    // Decision 0126 wall 66: WaitForMultipleObjectsEx (runtime/fle_thread.c): no APC can be queued, so it is WaitForMultipleObjects.
    ("kernel32.dll", "WaitForMultipleObjectsEx", "fle_imp_WaitForMultipleObjectsEx", "jmp_imp:WaitForMultipleObjectsEx"),
    // Decision 0126 wall 74: GetPhysicalCursorPos (runtime/fle_win.c), a GetProcAddress target of the guest.
    ("user32.dll", "GetPhysicalCursorPos", "fle_imp_GetPhysicalCursorPos", "jmp_imp:GetPhysicalCursorPos"),
    ("user32.dll", "SetPhysicalCursorPos", "fle_imp_SetPhysicalCursorPos", "jmp_imp:SetPhysicalCursorPos"),
    ("user32.dll", "SetCursorPos", "fle_imp_SetCursorPos", "jmp_imp:SetCursorPos"),
    // Decision 0126 wall 75: FormatMessageW for the system messages (runtime/fle_imports.c).
    ("kernel32.dll", "FormatMessageW", "fle_imp_FormatMessageW", "jmp_imp:FormatMessageW"),
    // Decision 0126 wall 77: GetCursorInfo (runtime/fle_win.c).
    ("user32.dll", "GetCursorInfo", "fle_imp_GetCursorInfo", "jmp_imp:GetCursorInfo"),
    // Decision 0126 wall 79 (PM's decision, S-ds-none): the virtual machine has no DirectSound device: DirectSoundEnumerateA calls nothing and returns DS_OK (runtime/fle_dsound.c). Not an audio implementation.
    ("dsound.dll", "DirectSoundEnumerateA", "fle_imp_DirectSoundEnumerateA", "jmp_imp:DirectSoundEnumerateA"),
    // Decision 0126 wall 82: the singly linked list calls QuickFontCache uses (runtime/fle_sys.c; the state is kept outside the SLIST_HEADER).
    ("kernel32.dll", "InterlockedPushEntrySList", "fle_imp_InterlockedPushEntrySList", "jmp_imp:InterlockedPushEntrySList"),
    ("kernel32.dll", "InterlockedPopEntrySList", "fle_imp_InterlockedPopEntrySList", "jmp_imp:InterlockedPopEntrySList"),
    // Decision 0126 wall 84: AdjustWindowRectEx (runtime/fle_win.c; the non-client area of the model's windows is 0, so the rectangle is returned as it is).
    ("user32.dll", "AdjustWindowRectEx", "fle_imp_AdjustWindowRectEx", "jmp_imp:AdjustWindowRectEx"),
    // Decision 0126 wall 85 (S-inet-offline): the virtual machine has no Internet connection: InternetGetConnectedState returns FALSE with no flag (runtime/fle_wininet.c). Not a network model.
    ("wininet.dll", "InternetGetConnectedState", "fle_imp_InternetGetConnectedState", "jmp_imp:InternetGetConnectedState"),
    // Decision 0126 walls 87 to 89 (runtime/fle_ole.c, fle_vfs.c): ILCreateFromPathW (ITEMIDLIST of a path of the virtual file system), SHChangeNotifyRegister and SHChangeNotifyDeregister (a registration that never delivers).
    ("shell32.dll", "ILCreateFromPathW", "fle_imp_ILCreateFromPathW", "jmp_imp:ILCreateFromPathW"),
    ("shell32.dll", "SHChangeNotifyRegister", "fle_imp_SHChangeNotifyRegister", "jmp_imp:SHChangeNotifyRegister"),
    ("shell32.dll", "SHChangeNotifyDeregister", "fle_imp_SHChangeNotifyDeregister", "jmp_imp:SHChangeNotifyDeregister"),
    // Decision 0126 wall 90 (runtime/fle_ole.c): RegisterDragDrop and RevokeDragDrop of a window of the window table (AddRef / Release called in the guest).
    ("ole32.dll", "RegisterDragDrop", "fle_imp_RegisterDragDrop", "jmp_imp:RegisterDragDrop"),
    ("ole32.dll", "RevokeDragDrop", "fle_imp_RevokeDragDrop", "jmp_imp:RevokeDragDrop"),
    // Decision 0126 wall 93: GetUpdateRgn (runtime/fle_win.c; the update region is one rectangle, copied into a region of fle_gdi.c).
    ("user32.dll", "GetUpdateRgn", "fle_imp_GetUpdateRgn", "jmp_imp:GetUpdateRgn"),
    // Decision 0126 wall 96: ValidateRgn (runtime/fle_win.c).
    ("user32.dll", "ValidateRgn", "fle_imp_ValidateRgn", "jmp_imp:ValidateRgn"),
    // Decision 0126 wall 98: the RECT helpers InflateRect, EqualRect, PtInRect and SubtractRect (runtime/fle_user_nowin.c).
    ("user32.dll", "InflateRect", "fle_imp_InflateRect", "jmp_imp:InflateRect"),
    ("user32.dll", "EqualRect", "fle_imp_EqualRect", "jmp_imp:EqualRect"),
    ("user32.dll", "PtInRect", "fle_imp_PtInRect", "jmp_imp:PtInRect"),
    ("user32.dll", "SubtractRect", "fle_imp_SubtractRect", "jmp_imp:SubtractRect"),
    // Decision 0126 wall 100: MonitorFromWindow (runtime/fle_win.c), MonitorFromPoint and MonitorFromRect (runtime/fle_monitor.c): one monitor, (0, 0, 1920, 1080).
    ("user32.dll", "MonitorFromWindow", "fle_imp_MonitorFromWindow", "jmp_imp:MonitorFromWindow"),
    ("user32.dll", "MonitorFromPoint", "fle_imp_MonitorFromPoint", "jmp_imp:MonitorFromPoint"),
    ("user32.dll", "MonitorFromRect", "fle_imp_MonitorFromRect", "jmp_imp:MonitorFromRect"),
    // Decision 0126 wall 101: SetWindowsHookExW (WH_MOUSE / WH_KEYBOARD for a thread of the process: recorded, never called) and UnhookWindowsHookEx (runtime/fle_win.c).
    ("user32.dll", "SetWindowsHookExW", "fle_imp_SetWindowsHookExW", "jmp_imp:SetWindowsHookExW"),
    ("user32.dll", "UnhookWindowsHookEx", "fle_imp_UnhookWindowsHookEx", "jmp_imp:UnhookWindowsHookEx"),
    // Decision 0129 wall 9: GetLastActivePopup (runtime/fle_win.c; S-lastactivepopup).
    ("user32.dll", "GetLastActivePopup", "fle_imp_GetLastActivePopup", "jmp_imp:GetLastActivePopup"),
    // Decision 0129 wall 10: GetThreadPriority (runtime/fle_proc.c): the value SetThreadPriority kept.
    ("kernel32.dll", "GetThreadPriority", "fle_imp_GetThreadPriority", "jmp_imp:GetThreadPriority"),
    // Decision 0129 wall 12: GetGuiResources (runtime/fle_win.c): GDI objects = the live objects of the virtual GDI table, USER objects = the live windows (S-guires-count).
    ("user32.dll", "GetGuiResources", "fle_imp_GetGuiResources", "jmp_imp:GetGuiResources"),
    // Decision 0129 wall 14: CoCreateInstance (runtime/fle_ole.c): a CLSID that the virtual registry does not hold is REGDB_E_CLASSNOTREG with *ppv = NULL (S-com-registry-empty).
    ("ole32.dll", "CoCreateInstance", "fle_imp_CoCreateInstance", "jmp_imp:CoCreateInstance"),
    // Decision 0132 wall 19: CreatePatternBrush (runtime/fle_gdi.c): the 8-by-8 monochrome pattern of the guest; a fill with it stops by name (S-patternbrush).
    ("gdi32.dll", "CreatePatternBrush", "fle_imp_CreatePatternBrush", "jmp_imp:CreatePatternBrush"),
    // Decision 0132 wall 20: ShowScrollBar (runtime/fle_win.c): state only (S-showscrollbar-state).
    ("user32.dll", "ShowScrollBar", "fle_imp_ShowScrollBar", "jmp_imp:ShowScrollBar"),
    // Decision 0132 wall 22: GetTempFileNameW (runtime/fle_vfs.c): a nonzero uUnique forms the name <path>\<pre><uuuu>.TMP (S-gettempfilename-name).
    ("kernel32.dll", "GetTempFileNameW", "fle_imp_GetTempFileNameW", "jmp_imp:GetTempFileNameW"),
    // Decision 0132 wall 23: an offline WinINet session (runtime/fle_wininet.c; S-inet-open-offline, S-inet-open-url-fails): the open of a URL fails, the rest is bookkeeping.
    ("wininet.dll", "InternetOpenW", "fle_imp_InternetOpenW", "jmp_imp:InternetOpenW"),
    ("wininet.dll", "InternetSetOptionW", "fle_imp_InternetSetOptionW", "jmp_imp:InternetSetOptionW"),
    ("wininet.dll", "InternetOpenUrlW", "fle_imp_InternetOpenUrlW", "jmp_imp:InternetOpenUrlW"),
    ("wininet.dll", "InternetCloseHandle", "fle_imp_InternetCloseHandle", "jmp_imp:InternetCloseHandle"),
    // Decision 0136 (PROVISIONAL): the input round trip (runtime/fle_input.c, fle_comctl.c). comctl32!_TrackMouseEvent is the wall of decision 0128 (WM_MOUSEMOVE); the others are the user32 input state
    // the VCL reads while it handles a mouse or key message. Appended at the end: no pseudo-VA moves (decision 0015).
    ("comctl32.dll", "_TrackMouseEvent", "fle_imp__TrackMouseEvent", "jmp_imp:_TrackMouseEvent"),
    ("user32.dll", "GetMessageTime", "fle_imp_GetMessageTime", "jmp_imp:GetMessageTime"),
    ("user32.dll", "GetMessagePos", "fle_imp_GetMessagePos", "jmp_imp:GetMessagePos"),
    ("user32.dll", "GetMessageExtraInfo", "fle_imp_GetMessageExtraInfo", "jmp_imp:GetMessageExtraInfo"),
    ("user32.dll", "GetDoubleClickTime", "fle_imp_GetDoubleClickTime", "jmp_imp:GetDoubleClickTime"),
    ("user32.dll", "GetKeyboardState", "fle_imp_GetKeyboardState", "jmp_imp:GetKeyboardState"),
    ("user32.dll", "MapVirtualKeyW", "fle_imp_MapVirtualKeyW", "jmp_imp:MapVirtualKeyW"),
    ("user32.dll", "MessageBeep", "fle_imp_MessageBeep", "jmp_imp:MessageBeep"),
    // Decision 0136 wall 8 (runtime/fle_token.c): the guest's elevation check after the Welcome wizard closes: GetCurrentProcess, OpenProcessToken(TOKEN_QUERY), GetTokenInformation(TokenElevation), CloseHandle (S-token-not-elevated).
    ("advapi32.dll", "OpenProcessToken", "fle_imp_OpenProcessToken", "jmp_imp:OpenProcessToken"),
    ("advapi32.dll", "GetTokenInformation", "fle_imp_GetTokenInformation", "jmp_imp:GetTokenInformation"),
    // Decision 0139 (audio lane S0-S2, PROVISIONAL): the DirectSound entry points other than DirectSoundEnumerateA (runtime/fle_dsound.c). FLEngine imports DirectSoundCreate only; Create8 and
    // EnumerateW are the other entry points of the same dsound.dll that a GetProcAddress could return. The COM methods of the objects they return are NOT rows: their slots are pseudo-VAs of
    // fle_dsound.c that fle_win_try_thunk hands to fle_com_try_slot (S-com-slot-range).
    ("dsound.dll", "DirectSoundCreate", "fle_imp_DirectSoundCreate", "jmp_imp:DirectSoundCreate"),
    ("dsound.dll", "DirectSoundCreate8", "fle_imp_DirectSoundCreate8", "jmp_imp:DirectSoundCreate8"),
    ("dsound.dll", "DirectSoundEnumerateW", "fle_imp_DirectSoundEnumerateW", "jmp_imp:DirectSoundEnumerateW"),
    // Decision 0140 (layered windows; runtime/fle_layered.c): the guest takes UpdateLayeredWindow (unit init at 0x81c8a0), UpdateLayeredWindowIndirect (0x65ccba) and SetLayeredWindowAttributes (0x86cabb) with GetProcAddress
    // (no IAT slot), and calls the first two from f_71e570 (TLightweightLayeredControl). GetLayeredWindowAttributes is not asked for by the guest; it completes the family (the page ties it to SetLayeredWindowAttributes).
    ("user32.dll", "UpdateLayeredWindow", "fle_imp_UpdateLayeredWindow", "jmp_imp:UpdateLayeredWindow"),
    ("user32.dll", "UpdateLayeredWindowIndirect", "fle_imp_UpdateLayeredWindowIndirect", "jmp_imp:UpdateLayeredWindowIndirect"),
    ("user32.dll", "SetLayeredWindowAttributes", "fle_imp_SetLayeredWindowAttributes", "jmp_imp:SetLayeredWindowAttributes"),
    ("user32.dll", "GetLayeredWindowAttributes", "fle_imp_GetLayeredWindowAttributes", "jmp_imp:GetLayeredWindowAttributes"),
    // Decision 0144 (G8, PROVISIONAL; runtime/fle_appfile.c): the walls of opening a project. Wall 1: shell32!SHAddToRecentDocs (f_11d9340 after the project is read; S-g8a-recentdocs).
    ("shell32.dll", "SHAddToRecentDocs", "fle_imp_SHAddToRecentDocs", "jmp_imp:SHAddToRecentDocs"),
    // Wall 3 (Sampler.flp): shcore!GetDpiForMonitor, delay-loaded by the engine after MonitorFromWindow (f_85dfcd); without the row the delay-load helper raised 0xC06D007F (S-g8a-dpi).
    ("shcore.dll", "GetDpiForMonitor", "fle_imp_GetDpiForMonitor", "jmp_imp:GetDpiForMonitor"),
    // Decision 0147 (provisional): user32!PostThreadMessageW (runtime/fle_win.c): the queue of the guest thread with that id; WM_QUIT is the kept WM_QUIT of PostQuitMessage. Appended after the rows of decision 0144.
    ("user32.dll", "PostThreadMessageW", "fle_imp_PostThreadMessageW", "jmp_imp:PostThreadMessageW"),
    // Decision 0267 (alpha10b; appended: decision 0015, append only): user32!SetWindowPlacement, the show state and the restored rectangle set (runtime/fle_win.c); FL calls it for its main form and one more window when they are created (SW_HIDE and the rectangle) once GetWindowPlacement answers.
    ("user32.dll", "SetWindowPlacement", "fle_imp_SetWindowPlacement", "jmp_imp:SetWindowPlacement"),
];

/// First pseudo-VA handed out by GetProcAddress (docs/decisions/0015-virtual-modules.md). Row `k` of
/// `TABLE` has VA `RESP_VA_BASE + RESP_VA_STEP * (k + 1)`. The region is far above the image
/// (ImageBase 0x400000, SizeOfImage 0x18c0000) and above the module handles
/// (runtime/fle_modules.h: 0x7FF800000000 + 0x10000 * N), and it is appended after every function
/// entry, so `fle_dispatch_table` stays sorted by VA.
const RESP_VA_BASE: u64 = 0x7FF9_0000_0000;
const RESP_VA_STEP: u64 = 0x10;

/// True when the closure has an import that GetProcAddress can be asked about, that is, when
/// kernel32!GetProcAddress is reached through an IAT slot. Only then are `fle_resp_table` and the
/// pseudo-VA entries of `fle_dispatch_table` emitted (any other output is unchanged).
pub fn resp_wanted<'a>(mut imports: impl Iterator<Item = &'a str>) -> bool {
    imports.any(|n| lookup(n).is_some_and(|(f, _)| f == "fle_imp_GetProcAddress"))
}

/// Decision 0156 (PROVISIONAL): extra import rows given by `fe-lift lift ... --extra-imports FILE`, appended after the rows of `TABLE` (row `TABLE.len() + j`).
/// Without the option nothing below differs from the version without it: `extra()` is empty and every function reads `TABLE` only.
type ExtraRow = (String, String, &'static str, &'static str);
static EXTRA: std::sync::OnceLock<Vec<ExtraRow>> = std::sync::OnceLock::new();

/// Parses an extra-imports file: one row per line, `<dll> <API> <runtime function>` (e.g. `kernel32.dll InitializeCriticalSectionEx fle_imp_InitializeCriticalSectionEx`),
/// `#` starts a comment. A row that is already in `TABLE` or repeated in the file is an error (it would change the meaning of an existing row).
pub fn set_extra(text: &str) -> Result<usize, String> {
    let mut v: Vec<ExtraRow> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 3 || !f[2].starts_with("fle_imp_") {
            return Err(format!("line {}: want `<dll> <API> fle_imp_<name>`", n + 1));
        }
        let dup = TABLE.iter().any(|(d, a, _, _)| d.eq_ignore_ascii_case(f[0]) && *a == f[1]) || v.iter().any(|(d, a, _, _)| d.eq_ignore_ascii_case(f[0]) && a == f[1]);
        if dup {
            return Err(format!("line {}: {}!{} already has a row", n + 1, f[0], f[1]));
        }
        v.push((
            f[0].to_string(),
            f[1].to_string(),
            &*Box::leak(f[2].to_string().into_boxed_str()),
            &*Box::leak(format!("jmp_imp:{}", f[1]).into_boxed_str()),
        ));
    }
    let n = v.len();
    EXTRA.set(v).map_err(|_| "extra imports already set".to_string())?;
    Ok(n)
}

fn extra() -> &'static [ExtraRow] {
    EXTRA.get().map(|v| v.as_slice()).unwrap_or(&[])
}

/// Every row in order: `TABLE`, then the extra rows. (dll, API, runtime function, jmp handler)
fn rows() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static str)> {
    TABLE.iter().copied().chain(extra().iter().map(|(d, a, f, h)| (d.as_str(), a.as_str(), *f, *h)))
}

/// Prototypes, `fle_dispatch_table` (the function entries followed by one pseudo-VA entry per row
/// of `TABLE`) and `fle_resp_table`. Replaces `dispatch::table` when `resp_wanted`. The table rows
/// are the only copy of the API names: runtime/fle_modules.c reads `fle_resp_table`.
pub fn dispatch_and_resp_tables(entries: &std::collections::BTreeSet<u64>) -> String {
    let mut s = String::from("#include \"fle_modules.h\"\n");
    for (_, _, f, _) in rows() {
        s.push_str(&format!("void {f}(void);\n"));
    }
    s.push_str("\nconst fle_fn_entry fle_dispatch_table[] = {\n");
    for &f in entries {
        s.push_str(&format!("    {{ UINT64_C({f:#x}), f_{f:x} }},\n"));
    }
    for (k, (_, _, f, _)) in rows().enumerate() {
        s.push_str(&format!("    {{ UINT64_C({:#x}), {f} }},\n", resp_va(k)));
    }
    s.push_str(&format!("}};\nconst uint64_t fle_dispatch_count = {};\n\n", entries.len() + resp_count()));
    s.push_str("const fle_resp_entry fle_resp_table[] = {\n");
    for (k, (dll, name, f, _)) in rows().enumerate() {
        s.push_str(&format!("    {{ \"{}\", \"{name}\", UINT64_C({:#x}), {f} }},\n", dll.to_ascii_lowercase(), resp_va(k)));
    }
    s.push_str(&format!("}};\nconst uint64_t fle_resp_count = {};\n\n", resp_count()));
    s
}

fn resp_va(k: usize) -> u64 {
    RESP_VA_BASE + RESP_VA_STEP * (k as u64 + 1)
}

/// Number of rows in the response table (the size of `fle_resp_table` when emitted).
pub fn resp_count() -> usize {
    TABLE.len() + extra().len()
}

/// The runtime function and handler name for `import` (`dll!name`), or None.
/// The DLL name is compared without case, as Windows does.
pub fn lookup(import: &str) -> Option<(&'static str, &'static str)> {
    let (dll, name) = import.rsplit_once('!')?;
    rows().find(|(d, n, _, _)| d.eq_ignore_ascii_case(dll) && *n == name).map(|(_, _, f, h)| (f, h))
}

/// Handler names `call_imp:<API>` of the `call [IAT slot]` form (decision 0077), one per row of
/// `TABLE` (then the extra rows), in the same order. Built once (the names are leaked once, one per row).
fn call_handlers() -> &'static [&'static str] {
    static H: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    H.get_or_init(|| rows().map(|r| &*Box::leak(format!("call_imp:{}", r.1).into_boxed_str())).collect())
}

/// Like `lookup`, for a `call` that reaches the IAT slot: the runtime function (the same row, so the
/// same `fle_imp_*` as the `jmp [IAT]` thunk) and the handler name `call_imp:<API>`.
pub fn lookup_call(import: &str) -> Option<(&'static str, &'static str)> {
    let (dll, name) = import.rsplit_once('!')?;
    let k = rows().position(|(d, n, _, _)| d.eq_ignore_ascii_case(dll) && n == name)?;
    Some((rows().nth(k)?.2, call_handlers()[k]))
}
