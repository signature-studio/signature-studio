# 停止の一覧(lift/runtime の fle_stop)

Issue #1411 の作業で作った。入力は `grep -rn 'fle_stop(' lift/runtime --include=*.c` の出力である(この版で 1,395 行。内訳は、呼び出し 1,394 行と、定義 1 行 `lift/runtime/fle_env.c:10`)。表は、その全ての行を覆う。同じファイル・関数・API・文言の行は、1 行にまとめて、行の列に並べて書いた。

- 件数: 呼び出し 1,394 行を、同じ文言の行をまとめて 1,364 行にした。表には、定義の 1 行(lift/runtime/fle_env.c:10)も足して、計 1,365 行。分類は (a) 133 件、(b) 1,261 件(定義の行は分類しない)。
- 直さない。この表は、後で停止の見直しをする時の入力である。

## 分類の規則

機械的に当てた(境界の行は、見直しで人が見る)。

- (a) 内部の不変条件・資源・ホストの限界(止めてよい)。文言が「out of memory」「exhausted」「internal」「is not linked」「no … table is linked」「cannot happen」「corrupt」を含む行、ホストのスタックや入れ子の上限、実行の上限(10 万フレーム、仮想の巻き戻しのループ)の行。
- (b) Windows の挙動が未確認、または未実装のため、暫定で止めている行。この Issue では直さない。
- 「(組み立ての文言)」は、その関数で理由の文字列を組む snprintf か代入の文言。「(動的な文言 変数名)」は、関数の中に文言が無い行。

## この Issue で変えた箇所

| ファイル | 変更前の行 | 変更前 | 変更後 |
| --- | --- | --- | --- |
| lift/runtime/fle_reg.c | 341(open_common) | 途中の空の要素で停止 | 161(ERROR_BAD_PATHNAME)を返す。警告の 1 行を出す |
| lift/runtime/fle_reg.c | 288(split_key) | 空の要素で停止(RegCreateKeyExW・RegDeleteKeyW も経由) | split_key は止めない。RegOpenKeyExW・RegCreateKeyExW・RegDeleteKeyW が、それぞれの値を返す |
| lift/runtime/fle_reg.c | (新規) | warn_empty_component の領域の確保 | (a) の out of memory。表の行に入る |

## 実機の記録との対応

- 指紋 2(RegOpenKeyExW の空の要素。版 2578ea1e23f6、17 記録・2 セッション)は、この Issue で止めなくした(変更前の fle_reg.c:341)。
- 指紋 5(RaiseException、版 2578ea1e23f6、3 記録・1 セッション)は、表の fle_seh.c の 467 行。戻り番地 0x411e4d の発生元は推定のまま。
- 指紋 1・3(戻り番地 0x43ba94・0x680e3d2 を含む)は、記録に fle: の行が無く、表の行とは対応させられない(「—」)。

## 直さない

表の全ての行は、この Issue では直さない。Windows の見当の列に書いた値は、確かめた資料の出所を括弧に書いた。

## 表

| ファイル | 行 | API | 止める条件(文言) | 分類 | Windows の挙動の見当 | 実機の記録 |
| --- | --- | --- | --- | --- | --- | --- |
| lift/runtime/fle_env.c | 10 | — [fle_stop] | 定義(停止の本体。fle: の行を出して abort する) | (定義) | — | — |
| lift/runtime/fle_acm.c | 306 | acmStreamConvert [fle_imp_acm_StreamConvert] | a Layer-3 frame has a sample rate other than the one of the stream's source format: the pages define no conversion | (b) | — | — |
| lift/runtime/fle_acm.c | 307 | acmStreamConvert [fle_imp_acm_StreamConvert] | a Layer-3 frame has a channel mode other than the one of the stream's source format (mono vs stereo): the pages define no conversion | (b) | — | — |
| lift/runtime/fle_acm.c | 311 | acmStreamConvert [fle_imp_acm_StreamConvert] | minimp3 returned a sample count that is not the frame's | (b) | — | — |
| lift/runtime/fle_acm.c | 355 | acmFormatChooseW [fle_imp_acm_FormatChooseW] | the format-choice dialog of the Audio Compression Manager is not modelled (S-ref1-acm-no-dialog) | (b) | — | — |
| lift/runtime/fle_acm.c | 360 | acmStreamMessage [fle_imp_acm_StreamMessage] | the page: 'returns the value returned by the ACM device driver'; this environment's one driver (an MPEG Layer-3 decoder) defines no driver-specific message | (b) | — | — |
| lift/runtime/fle_act1_net.c | 65 | — [ent_room] | more connection and request handles than the runtime holds (65536, none reused) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 67 | — [ent_room] | out of memory | (a) | — | — |
| lift/runtime/fle_act1_net.c | 166 | InternetConnectW [fle_imp_InternetConnectW] | hInternet is not a live session of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 167 | InternetConnectW [fle_imp_InternetConnectW] | lpszServerName is NULL: the page gives no behaviour (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 168 | InternetConnectW [fle_imp_InternetConnectW] | dwService is not INTERNET_SERVICE_HTTP: FTP opens a real connection and needs a model of its own (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 169 | InternetConnectW [fle_imp_InternetConnectW] | a user name or password is passed: the guest has not been seen to do it and the page gives no HTTP behaviour (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 170 | InternetConnectW [fle_imp_InternetConnectW] | dwFlags is not 0: the guest has not been seen to pass flags (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 192 | HttpOpenRequestW [fle_imp_HttpOpenRequestW] | hConnect is not a connection handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 193 | HttpOpenRequestW [fle_imp_HttpOpenRequestW] | lpszObjectName is NULL: the page gives no behaviour (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 219 | HttpAddRequestHeadersW [fle_imp_HttpAddRequestHeadersW] | hRequest is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 220 | HttpAddRequestHeadersW [fle_imp_HttpAddRequestHeadersW] | lpszHeaders is NULL: the page gives no behaviour (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 221 | HttpAddRequestHeadersW [fle_imp_HttpAddRequestHeadersW] | dwModifiers is not HTTP_ADDREQ_FLAG_ADD: the guest has not been seen to pass another modifier (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 236 | HttpSendRequestExW [fle_imp_HttpSendRequestExW] | hRequest is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 237 | HttpSendRequestExW [fle_imp_HttpSendRequestExW] | lpBuffersIn, lpBuffersOut or dwFlags is not 0: the guest has not been seen to pass them (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 238 | HttpSendRequestExW [fle_imp_HttpSendRequestExW] | the request was already sent once: the page gives no behaviour for a second send (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 261 | InternetWriteFile [fle_imp_InternetWriteFile] | hFile is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 275 | HttpEndRequestW [fle_imp_HttpEndRequestW] | hRequest is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 285 | HttpEndRequestW [query_info] | hRequest is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 301 | InternetReadFile [fle_imp_InternetReadFile] | hFile is not a request handle of this runtime: the page names no error code (decision 0213) | (b) | — | — |
| lift/runtime/fle_act1_net.c | 312 | InternetGetLastResponseInfoW [fle_imp_InternetGetLastResponseInfoW] | lpdwError or lpdwBufferLength is NULL: the page gives no behaviour (decision 0213) | (b) | — | — |
| lift/runtime/fle_api2_shell.c | 52 | ShellExecuteA [fle_imp_ShellExecuteA] | lpFile is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_shell.c | 68 | ShellExecuteExW [fle_imp_ShellExecuteExW] | pExecInfo is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_shell.c | 69 | ShellExecuteExW [fle_imp_ShellExecuteExW] | cbSize is smaller than the 112 bytes of SHELLEXECUTEINFOW: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_shell.c | 70 | ShellExecuteExW [fle_imp_ShellExecuteExW] | lpFile is NULL: the page gives no behaviour (a class or ID list may be used instead) | (b) | — | — |
| lift/runtime/fle_api2_str.c | 311 | TzSpecificLocalTimeToSystemTime [fle_imp_TzSpecificLocalTimeToSystemTime] | lpLocalTime is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_str.c | 312 | TzSpecificLocalTimeToSystemTime [fle_imp_TzSpecificLocalTimeToSystemTime] | lpUniversalTime is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_sys.c | 78 | CreateSymbolicLinkW [fle_imp_CreateSymbolicLinkW] | lpSymlinkFileName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_sys.c | 79 | CreateSymbolicLinkW [fle_imp_CreateSymbolicLinkW] | lpTargetFileName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_api2_sys.c | 80 | CreateSymbolicLinkW [fle_imp_CreateSymbolicLinkW] | dwFlags has a bit other than SYMBOLIC_LINK_FLAG_DIRECTORY: the page ties 0x2 to Developer Mode, which the virtual machine does not model | (b) | — | — |
| lift/runtime/fle_appfile.c | 84 | GetCommandLineW [fle_appfile_cmdline_w] | out of memory (FLE_CMDLINE) | (a) | — | — |
| lift/runtime/fle_appfile.c | 85 | GetCommandLineW [fle_appfile_cmdline_w] | FLE_CMDLINE is not valid UTF-8 | (b) | — | — |
| lift/runtime/fle_appfile.c | 96 | GetCommandLineA [fle_appfile_cmdline_a] | FLE_CMDLINE has a character above 0x7F: the conversion to the 8-bit process code page is not modelled for it (decision 0144) | (b) | — | — |
| lift/runtime/fle_appfile.c | 225 | SHAddToRecentDocs [fle_imp_SHAddToRecentDocs] | this SHARD form of the item is not modelled (decision 0144: only SHARD_PATHA 2, SHARD_PATHW 3 and a NULL item) | (b) | — | — |
| lift/runtime/fle_callback.c | 46 | fle_call_guest [fle_call_guest] | the callback address is 0 | (b) | — | — |
| lift/runtime/fle_callback.c | 61 | fle_call_guest [fle_call_guest] | (組み立ての文言) callback nesting %u: the guest stack is used up (rsp %# | (a) | — | — |
| lift/runtime/fle_callback.c | 70 | fle_call_guest [fle_call_guest] | (組み立ての文言) callback nesting %u: the host (C) stack of the thread is used up (% | (a) | — | — |
| lift/runtime/fle_callback.c | 75 | fle_call_guest [fle_call_guest] | (組み立ての文言) callback nesting is deeper than FLE_CB_MAX_DEPTH (%u), the backstop against a runaway recursion: decision 0319 | (a) | — | — |
| lift/runtime/fle_callback.c | 97 | fle_call_guest [fle_call_guest] | the callback returned with R[4] not at its return address: the stack is unbalanced | (b) | — | — |
| lift/runtime/fle_callback.c | 98 | fle_call_guest [fle_call_guest] | the callback changed its return address slot | (b) | — | — |
| lift/runtime/fle_callback.c | 102 | fle_call_guest [fle_call_guest] | (組み立ての文言) the callback changed the callee-saved register %s | (b) | — | — |
| lift/runtime/fle_class.c | 41 | — [fle_class_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_class.c | 57 | — [read_name] | the class name is an atom or NULL, not a string: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 62 | — [read_name] | the class name is empty: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 65 | — [read_name] | the class name is longer than 256 units (the page says RegisterClass fails, no value for it): not answered | (b) | — | — |
| lift/runtime/fle_class.c | 66 | — [read_name] | the class name has a unit above 0x7F: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 142 | RegisterClassW [fle_imp_RegisterClassW] | lpWndClass is NULL: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 154 | RegisterClassW [fle_imp_RegisterClassW] | lpfnWndProc is NULL: the page gives no value for the failure, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 155 | RegisterClassW [fle_imp_RegisterClassW] | hInstance is not the lifted DLL (0x400000): not answered | (b) | — | — |
| lift/runtime/fle_class.c | 156 | RegisterClassW [fle_imp_RegisterClassW] | CS_GLOBALCLASS: application global classes (lookup from other modules) are not modelled | (b) | — | — |
| lift/runtime/fle_class.c | 158 | RegisterClassW [fle_imp_RegisterClassW] | cbClsExtra or cbWndExtra is outside 0..40: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 160 | RegisterClassW [fle_imp_RegisterClassW] | a class of this name is already registered by this hInstance: the page gives no value for the failure, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 161 | RegisterClassW [fle_imp_RegisterClassW] | a class whose name differs only in case is registered: the pages do not say if names are case-sensitive, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 162 | RegisterClassW [fle_imp_RegisterClassW] | the atoms 0xC000..0xFFFF are used up | (b) | — | — |
| lift/runtime/fle_class.c | 167 | RegisterClassW [fle_imp_RegisterClassW] | out of memory | (a) | — | — |
| lift/runtime/fle_class.c | 197 | GetClassInfoW [fle_imp_GetClassInfoW] | hInstance is NULL: the system classes are not modelled | (b) | — | — |
| lift/runtime/fle_class.c | 198 | GetClassInfoW [fle_imp_GetClassInfoW] | hInstance is not the lifted DLL (0x400000): not answered | (b) | — | — |
| lift/runtime/fle_class.c | 199 | GetClassInfoW [fle_imp_GetClassInfoW] | lpWndClass is NULL: not answered | (b) | — | — |
| lift/runtime/fle_class.c | 203 | GetClassInfoW [fle_imp_GetClassInfoW] | a class whose name differs only in case is registered: the pages do not say if names are case-sensitive, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 209 | GetClassInfoW [fle_imp_GetClassInfoW] | (組み立ての文言) the name \"%s\" is a system class (Button, Edit ...): the system registered it and gives its data; not modelled, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 236 | GetClassInfoW [fle_class_lookup] | the window's hInstance is not the lifted DLL (0x400000): the class lookup is not answered | (b) | — | — |
| lift/runtime/fle_class.c | 240 | GetClassInfoW [fle_class_lookup] | the class name differs from a registered one only in case: the pages do not say if names are case-sensitive | (b) | — | — |
| lift/runtime/fle_class.c | 241 | GetClassInfoW [fle_class_lookup] | the class is a system class (Button, Edit ...): the system registered it and gives its data; not modelled, not answered | (b) | — | — |
| lift/runtime/fle_class.c | 264 | GetClassInfoW [fle_class_set_hicon] | the class of the window is not in the class table | (b) | — | — |
| lift/runtime/fle_class.c | 328 | UnregisterClassW [fle_imp_UnregisterClassW] | hInstance is not the lifted DLL (0x400000) nor NULL: not answered | (b) | — | — |
| lift/runtime/fle_clip.c | 315 | GetClipboardData [fle_imp_GetClipboardData] | delayed rendering (SetClipboardData with a NULL handle) needs WM_RENDERFORMAT to the owner: not implemented | (b) | — | — |
| lift/runtime/fle_clip.c | 329 | GetClipboardData [fle_imp_GetClipboardData] | a synthesized format (the system converts the data): not implemented | (b) | — | — |
| lift/runtime/fle_clip.c | 361 | SetClipboardData [fle_imp_SetClipboardData] | more than 64 formats on the clipboard (a limit of this runtime) | (b) | — | — |
| lift/runtime/fle_comctl.c | 25 | _TrackMouseEvent [fle_imp__TrackMouseEvent] | lpEventTrack is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_crash1_drag.c | 84 | DragQueryFileW [fle_imp_DragQueryFileW] | an ANSI file list with a byte of 0x80 or above: the code page conversion of the list is not done (DROPFILES.fWide is 0) | (b) | — | — |
| lift/runtime/fle_crash1_menu.c | 70 | CreateMenu [new_menu] | more than 100000 menus were created (the handle region is full) | (b) | — | — |
| lift/runtime/fle_crash1_menu.c | 74, 79 | CreateMenu [new_menu] | out of memory | (a) | — | — |
| lift/runtime/fle_crash1_menu.c | 320 | — [insert_with] | out of memory | (a) | — | — |
| lift/runtime/fle_crypt.c | 105 | — [new_ent] | out of memory | (a) | — | — |
| lift/runtime/fle_crypt.c | 109 | — [new_ent] | crypt handle region exhausted | (a) | — | — |
| lift/runtime/fle_crypt.c | 121, 124 | — [get_ent] | handle was not issued by this runtime's CryptoAPI | (b) | — | — |
| lift/runtime/fle_crypt.c | 139 | CryptAcquireContextA [fle_imp_CryptAcquireContextA] | phProv is NULL | (b) | — | — |
| lift/runtime/fle_crypt.c | 140 | CryptAcquireContextA [fle_imp_CryptAcquireContextA] | szContainer is not NULL (named key containers are not modelled) | (b) | — | — |
| lift/runtime/fle_crypt.c | 141 | CryptAcquireContextA [fle_imp_CryptAcquireContextA] | szProvider is not NULL (named providers are not modelled) | (b) | — | — |
| lift/runtime/fle_crypt.c | 142 | CryptAcquireContextA [fle_imp_CryptAcquireContextA] | dwProvType is not PROV_RSA_FULL / PROV_RSA_AES | (b) | — | — |
| lift/runtime/fle_crypt.c | 143 | CryptAcquireContextA [fle_imp_CryptAcquireContextA] | dwFlags is not exactly CRYPT_VERIFYCONTEXT (persisted containers are not modelled) | (b) | — | — |
| lift/runtime/fle_crypt.c | 154 | CryptReleaseContext [fle_imp_CryptReleaseContext] | dwFlags is not 0 | (b) | — | — |
| lift/runtime/fle_crypt.c | 170 | CryptCreateHash [fle_imp_CryptCreateHash] | phHash is NULL | (b) | — | — |
| lift/runtime/fle_crypt.c | 171 | CryptCreateHash [fle_imp_CryptCreateHash] | Algid is not CALG_MD5 (only MD5 is implemented) | (b) | — | — |
| lift/runtime/fle_crypt.c | 172 | CryptCreateHash [fle_imp_CryptCreateHash] | hKey is not 0 (keyed hashes are not implemented) | (b) | — | — |
| lift/runtime/fle_crypt.c | 173 | CryptCreateHash [fle_imp_CryptCreateHash] | dwFlags is not 0 | (b) | — | — |
| lift/runtime/fle_crypt.c | 193 | CryptHashData [fle_imp_CryptHashData] | dwFlags is not 0 | (b) | — | — |
| lift/runtime/fle_crypt.c | 197 | CryptHashData [fle_imp_CryptHashData] | pbData is NULL with a nonzero length | (b) | — | — |
| lift/runtime/fle_crypt.c | 199 | CryptHashData [fle_imp_CryptHashData] | out of memory | (a) | — | — |
| lift/runtime/fle_crypt.c | 217 | CryptGetHashParam [fle_imp_CryptGetHashParam] | dwFlags is not 0 | (b) | — | — |
| lift/runtime/fle_crypt.c | 218 | CryptGetHashParam [fle_imp_CryptGetHashParam] | pdwDataLen is NULL | (b) | — | — |
| lift/runtime/fle_crypt.c | 224 | CryptGetHashParam [fle_imp_CryptGetHashParam] | dwParam is none of HP_ALGID / HP_HASHSIZE / HP_HASHVAL | (b) | — | — |
| lift/runtime/fle_cursor.c | 99 | LoadCursorW [fle_imp_LoadCursorW] | hInstance is not NULL: cursor resources of a module are not implemented (decision 0059; only IDC_ARROW is answered) | (b) | — | — |
| lift/runtime/fle_cursor.c | 104 | LoadCursorW [fle_imp_LoadCursorW] | lpCursorName is a pointer to a name, not an integer identifier: not answered (decision 0059) | (b) | — | — |
| lift/runtime/fle_cursor.c | 107 | LoadCursorW [fle_imp_LoadCursorW] | (組み立ての文言) lpCursorName MAKEINTRESOURCE(%u) is not IDC_ARROW (32512): not answered (decision 0059) | (b) | — | — |
| lift/runtime/fle_cursor.c | 122 | GetClipCursor [fle_imp_GetClipCursor] | lpRect is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_desk.c | 336 | fle_desk_gather [gather] | the top-level windows reach the limit of the window list (4096): a window past it would be left out (decision 0392) | (b) | — | — |
| lift/runtime/fle_desk.c | 399 | fle_desk_gather [gather] | more than 64 windows are drawn (FLE_DESK_MAX_WINDOWS): the compositor and the scene hold 64 (decision 0392) | (b) | — | — |
| lift/runtime/fle_dsound.c | 610 | — [this_dev] | `this` is not a DirectSound object that this runtime made | (b) | — | — |
| lift/runtime/fle_dsound.c | 611 | — [this_dev] | the object was released (use after the last Release) | (b) | — | — |
| lift/runtime/fle_dsound.c | 620 | — [this_buf] | `this` is not a DirectSound buffer that this runtime made | (b) | — | — |
| lift/runtime/fle_dsound.c | 621 | — [this_buf] | the object was released (use after the last Release) | (b) | — | — |
| lift/runtime/fle_dsound.c | 622 | — [this_buf] | the interface pointer is not the one this method belongs to | (b) | — | — |
| lift/runtime/fle_dsound.c | 672 | — [m_ds_release] | no reference outstanding | (b) | — | — |
| lift/runtime/fle_dsound.c | 735 | — [m_ds_createsoundbuffer] | DSBCAPS_CTRL3D (3D buffers) is not modeled (S-ds-no-3d) | (b) | — | — |
| lift/runtime/fle_dsound.c | 736 | — [m_ds_createsoundbuffer] | DSBCAPS_CTRLFX (effects) is not modeled (S-ds-no-fx) | (b) | — | — |
| lift/runtime/fle_dsound.c | 737 | — [m_ds_createsoundbuffer] | DSBCAPS_LOCDEFER / DSBCAPS_STATIC are not modeled (S-ds-no-voice-manager) | (b) | — | — |
| lift/runtime/fle_dsound.c | 754 | — [m_ds_createsoundbuffer] | a second primary buffer on one device (no page defines it: S-ds-primary-twice) | (b) | — | — |
| lift/runtime/fle_dsound.c | 789 | — [m_ds_createsoundbuffer] | a secondary buffer whose rate differs from the device's: resampling is not modeled (S-ds-no-resample) | (b) | — | — |
| lift/runtime/fle_dsound.c | 849 | — [m_ds_setcoop] | DSSCL_WRITEPRIMARY (the application writes the primary buffer; no secondary buffer can play) is not modeled (S-ds-no-writeprimary) | (b) | — | — |
| lift/runtime/fle_dsound.c | 873 | — [m_b_qi] | `this` is not a live DirectSound buffer | (b) | — | — |
| lift/runtime/fle_dsound.c | 904 | — [m_b_addref] | `this` is not a live DirectSound buffer | (b) | — | — |
| lift/runtime/fle_dsound.c | 913 | — [m_b_release] | `this` is not a live DirectSound buffer | (b) | — | — |
| lift/runtime/fle_dsound.c | 914 | — [m_b_release] | no reference outstanding | (b) | — | — |
| lift/runtime/fle_dsound.c | 1078 | — [m_b_setfreq] | a frequency other than the buffer's own: resampling is not modeled (S-ds-no-resample) | (b) | — | — |
| lift/runtime/fle_dsound.c | 1270 | — [m_b_setformat] | the device rate is changed while a buffer plays (S-ds-rate-change) | (b) | — | — |
| lift/runtime/fle_dsound.c | 1378 | — [m_stop] | (組み立ての文言) %s is not modeled (decision 0139: the engine does not call it; a call is a finding) | (b) | — | — |
| lift/runtime/fle_dsound.c | 1437 | — [enumerate] | the guest callback mechanism is not linked: the callback cannot be called | (a) | — | — |
| lift/runtime/fle_dsound.c | 1473 | — [create] | more DirectSound objects than the model holds (8) | (b) | — | — |
| lift/runtime/fle_dwm.c | 23 | DwmIsCompositionEnabled [fle_imp_DwmIsCompositionEnabled] | pfEnabled is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_dwm.c | 39 | DwmExtendFrameIntoClientArea [fle_imp_DwmExtendFrameIntoClientArea] | pMarInset is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_dwm.c | 40 | DwmExtendFrameIntoClientArea [fle_imp_DwmExtendFrameIntoClientArea] | hWnd is not a window of the window table: the page names no HRESULT for it | (b) | — | — |
| lift/runtime/fle_dwm.c | 75 | DwmGetCompositionTimingInfo [fle_imp_DwmGetCompositionTimingInfo] | pTimingInfo is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_env.c | 42 | — [fle_w_read] | NULL string pointer | (b) | — | — |
| lift/runtime/fle_env.c | 44 | — [fle_w_read] | string longer than the supported limit | (b) | — | — |
| lift/runtime/fle_env.c | 54 | — [fle_w_read_alloc] | NULL string pointer | (b) | — | — |
| lift/runtime/fle_env.c | 57 | — [fle_w_read_alloc] | string longer than the supported limit | (b) | — | — |
| lift/runtime/fle_env.c | 60 | — [fle_w_read_alloc] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 75 | runtime [fle_w_from_ascii] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 114 | — [env_name] | empty variable name is not defined | (b) | — | — |
| lift/runtime/fle_env.c | 116 | — [env_name] | variable name containing '=' is not defined | (b) | — | — |
| lift/runtime/fle_env.c | 142 | GetEnvironmentVariableW [fle_imp_GetEnvironmentVariableW] | lpBuffer is NULL but nSize leaves room for the value | (b) | — | — |
| lift/runtime/fle_env.c | 165 | SetEnvironmentVariableW [fle_imp_SetEnvironmentVariableW] | deleting a variable that does not exist is not defined | (b) | — | — |
| lift/runtime/fle_env.c | 182 | SetEnvironmentVariableW [fle_imp_SetEnvironmentVariableW] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 210 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | lpSrc is NULL | (b) | — | — |
| lift/runtime/fle_env.c | 211 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | lpDst equals lpSrc (the page forbids it) | (b) | — | — |
| lift/runtime/fle_env.c | 212 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | lpDst is NULL with nSize > 0 | (b) | — | — |
| lift/runtime/fle_env.c | 215, 244 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 227 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | '%' without a closing '%' is not defined | (b) | — | — |
| lift/runtime/fle_env.c | 228 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | empty %% is not defined | (b) | — | — |
| lift/runtime/fle_env.c | 239 | ExpandEnvironmentStringsW [fle_imp_ExpandEnvironmentStringsW] | expansion longer than the supported limit | (b) | — | — |
| lift/runtime/fle_env.c | 291, 298 | GetCommandLineA [fle_imp_GetCommandLineA] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 339 | GetEnvironmentStringsW [fle_imp_GetEnvironmentStringsW] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 361 | FreeEnvironmentStringsW [fle_imp_FreeEnvironmentStringsW] | a pointer that GetEnvironmentStringsW did not return (or that was already freed); the page names no error code | (b) | — | — |
| lift/runtime/fle_env.c | 378 | GetStartupInfoW [fle_imp_GetStartupInfoW] | lpStartupInfo is NULL | (b) | — | — |
| lift/runtime/fle_env.c | 391 | GetStartupInfoA [fle_imp_GetStartupInfoA] | lpStartupInfo is NULL | (b) | — | — |
| lift/runtime/fle_env.c | 418 | GetEnvironmentStrings [fle_imp_GetEnvironmentStrings] | an environment variable name has a character at or above 0x80 (the ANSI conversion is not modelled) | (b) | — | — |
| lift/runtime/fle_env.c | 423 | GetEnvironmentStrings [fle_imp_GetEnvironmentStrings] | an environment variable value has a character at or above 0x80 (the ANSI conversion is not modelled) | (b) | — | — |
| lift/runtime/fle_env.c | 432 | GetEnvironmentStrings [fle_imp_GetEnvironmentStrings] | out of memory | (a) | — | — |
| lift/runtime/fle_env.c | 454 | FreeEnvironmentStringsA [fle_imp_FreeEnvironmentStringsA] | a pointer that GetEnvironmentStrings did not return (or that was already freed); the page names no error code | (b) | — | — |
| lift/runtime/fle_env.c | 538 | SHGetFolderPathW [fle_imp_SHGetFolderPathW] | (組み立ての文言) csidl %#x is not answered (answered: 0x05 PERSONAL, 0x14 FONTS, 0x1a APPDATA, 0x1c LOCAL_APPDATA, 0x23 COMMON_APPDATA, 0x24 WINDOWS, 0x26 PR… | (b) | — | — |
| lift/runtime/fle_env.c | 540 | SHGetFolderPathW [fle_imp_SHGetFolderPathW] | hToken is not NULL: access tokens are not modelled | (b) | — | — |
| lift/runtime/fle_env.c | 541 | SHGetFolderPathW [fle_imp_SHGetFolderPathW] | dwFlags is neither SHGFP_TYPE_CURRENT (0) nor SHGFP_TYPE_DEFAULT (1) | (b) | — | — |
| lift/runtime/fle_env.c | 542 | SHGetFolderPathW [fle_imp_SHGetFolderPathW] | pszPath is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_env.c | 578 | SHGetFolderPathW [computer_name_for_format] | (組み立ての文言) NameType %u (a DNS domain or a fully qualified name): the pages do not say what a computer that is not joined to a domain returns | (b) | — | — |
| lift/runtime/fle_env.c | 580 | SHGetFolderPathW [computer_name_for_format] | NameType 8 (ComputerNameMax) is \"Not used\" on the COMPUTER_NAME_FORMAT page and no error is documented for it | (b) | — | — |
| lift/runtime/fle_env.c | 583 | SHGetFolderPathW [computer_name_for_format] | (組み立ての文言) NameType %u is outside the COMPUTER_NAME_FORMAT enumeration (0 to 8) | (b) | — | — |
| lift/runtime/fle_env.c | 609 | SHGetFolderPathW [name_answer] | the size pointer is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_env.c | 611 | SHGetFolderPathW [name_answer] | lpBuffer is NULL but *nSize is not zero: the page says it must be zero | (b) | — | — |
| lift/runtime/fle_env.c | 618 | SHGetFolderPathW [name_answer] | lpBuffer is NULL and the size would hold the name: the page defines no such call | (b) | — | — |
| lift/runtime/fle_event.c | 28 | — [fle_event_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_event.c | 88 | CloseHandle [event_close_hook] | handle value was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_event.c | 119 | — [event_arg] | handle was not issued as an event by this runtime (other handle kinds, pseudo handles and INVALID_HANDLE_VALUE are not defined here) | (b) | — | — |
| lift/runtime/fle_event.c | 142 | CreateEventW [fle_imp_CreateEventW] | lpEventAttributes is not NULL (security descriptors and inheritance are not implemented) | (b) | — | — |
| lift/runtime/fle_event.c | 143 | CreateEventW [fle_imp_CreateEventW] | a named event is not implemented | (b) | — | — |
| lift/runtime/fle_event.c | 149 | CreateEventW [fle_imp_CreateEventW] | out of memory | (a) | — | — |
| lift/runtime/fle_event.c | 153 | CreateEventW [fle_imp_CreateEventW] | event handle region exhausted | (a) | — | — |
| lift/runtime/fle_event.c | 234 | WaitForSingleObject [fle_imp_WaitForSingleObject] | INFINITE time-out on an event that is not signaled would never return (no other thread exists) | (b) | — | — |
| lift/runtime/fle_event.c | 235 | WaitForSingleObject [fle_imp_WaitForSingleObject] | a finite, nonzero time-out on an event that is not signaled needs the virtual clock to advance (decision 0050: not implemented) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 118 | filedlg [wdup] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 134 | filedlg [wdup_utf8] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 140, 142 | filedlg [wdup_utf8] | the picker's path is not UTF-8 | (b) | — | — |
| lift/runtime/fle_filedlg.c | 171 | filedlg [task_alloc] | CoTaskMemAlloc returned NULL | (b) | — | — |
| lift/runtime/fle_filedlg.c | 233 | — [row_va] | the response table has no row for a method of the file dialog model (decision 0178: build with harness/seeds/dlg1-imports.txt) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 298 | — [obj_new] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 312 | — [self] | `this` is not an object of the file dialog model of the expected interface | (b) | — | — |
| lift/runtime/fle_filedlg.c | 320 | — [foreign] | the interface pointer is not an object of this model (a guest-implemented IShellItem is not supported) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 341 | filedlg [obj_release_internal] | internal reference count underflow | (a) | — | — |
| lift/runtime/fle_filedlg.c | 376 | filedlg [obj_free] | internal error: a sub-object is freed with its owner | (a) | — | — |
| lift/runtime/fle_filedlg.c | 415 | ShellObj::QueryInterface [fle_imp_ShellObj_QueryInterface] | `this` is not an object of the file dialog model | (b) | — | — |
| lift/runtime/fle_filedlg.c | 416 | ShellObj::QueryInterface [fle_imp_ShellObj_QueryInterface] | NULL riid or ppvObject: the page does not give the behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 444 | ShellObj::QueryInterface [fle_imp_ShellObj_QueryInterface] | (組み立ての文言) interface %s was asked of an object of kind %u: not modelled (IUnknown / IModalWindow / IFileDialog / IFileOpenDialog / IFileDialogCustomize… | (b) | — | — |
| lift/runtime/fle_filedlg.c | 452 | ShellObj::AddRef [fle_imp_ShellObj_AddRef] | `this` is not an object of the file dialog model | (b) | — | — |
| lift/runtime/fle_filedlg.c | 463 | ShellObj::Release [fle_imp_ShellObj_Release] | `this` is not an object of the file dialog model | (b) | — | — |
| lift/runtime/fle_filedlg.c | 465 | ShellObj::Release [fle_imp_ShellObj_Release] | no reference outstanding | (b) | — | — |
| lift/runtime/fle_filedlg.c | 497 | ShellObj::Release [ctl_get] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 505 | ShellObj::Release [ctl_add] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 509 | ShellObj::Release [ctl_add] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 529 | ShellObj::Release [item_get] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 578 | IFileDialogCustomize::GetControlState [fle_imp_IFileDialogCustomize_GetControlState] | pdwState is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 596 | IFileDialogCustomize::GetEditBoxText [fle_imp_IFileDialogCustomize_GetEditBoxText] | the control is not an edit box | (b) | — | — |
| lift/runtime/fle_filedlg.c | 597 | IFileDialogCustomize::GetEditBoxText [fle_imp_IFileDialogCustomize_GetEditBoxText] | ppszText is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 607 | IFileDialogCustomize::SetEditBoxText [fle_imp_IFileDialogCustomize_SetEditBoxText] | the control is not an edit box | (b) | — | — |
| lift/runtime/fle_filedlg.c | 616 | IFileDialogCustomize::GetCheckButtonState [fle_imp_IFileDialogCustomize_GetCheckButtonState] | the control is not a check button | (b) | — | — |
| lift/runtime/fle_filedlg.c | 617 | IFileDialogCustomize::GetCheckButtonState [fle_imp_IFileDialogCustomize_GetCheckButtonState] | pbChecked is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 626 | IFileDialogCustomize::SetCheckButtonState [fle_imp_IFileDialogCustomize_SetCheckButtonState] | the control is not a check button | (b) | — | — |
| lift/runtime/fle_filedlg.c | 637 | IFileDialogCustomize::AddControlItem [fle_imp_IFileDialogCustomize_AddControlItem] | the control is not a container (menu, combo box, option button group, open drop-down): the page names no HRESULT for it | (b) | — | — |
| lift/runtime/fle_filedlg.c | 638 | IFileDialogCustomize::AddControlItem [fle_imp_IFileDialogCustomize_AddControlItem] | the item id is already used in the control: the page names no HRESULT for it (S-dlg1-custom) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 642 | IFileDialogCustomize::AddControlItem [fle_imp_IFileDialogCustomize_AddControlItem] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 690 | IFileDialogCustomize::GetControlItemState [fle_imp_IFileDialogCustomize_GetControlItemState] | pdwState is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 708 | IFileDialogCustomize::GetSelectedControlItem [fle_imp_IFileDialogCustomize_GetSelectedControlItem] | the page: 'This method cannot be called on menus' | (b) | — | — |
| lift/runtime/fle_filedlg.c | 709 | IFileDialogCustomize::GetSelectedControlItem [fle_imp_IFileDialogCustomize_GetSelectedControlItem] | the control is not a container: the page names no HRESULT for it | (b) | — | — |
| lift/runtime/fle_filedlg.c | 710 | IFileDialogCustomize::GetSelectedControlItem [fle_imp_IFileDialogCustomize_GetSelectedControlItem] | pdwIDItem is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 715 | IFileDialogCustomize::GetSelectedControlItem [fle_imp_IFileDialogCustomize_GetSelectedControlItem] | no item is selected and the page does not say what is returned (S-dlg1-custom-sel) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 740 | IFileDialogCustomize::EndVisualGroup [fle_imp_IFileDialogCustomize_EndVisualGroup] | no visual group is open: the page names no HRESULT for it (S-dlg1-custom) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 779 | filedlg [env_picker] | FLE_FILEDLG_PICK_FILE cannot be opened | (b) | — | — |
| lift/runtime/fle_filedlg.c | 781, 797 | filedlg [env_picker] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 784 | filedlg [env_picker] | FLE_FILEDLG_PICK_FILE is empty or not a readable file (the harness script's setenv keeps only 63 characters of a value: use a short path) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 844 | filedlg [utf16_to_utf8] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 926 | filedlg [browser_picker] | open_memstream failed | (b) | — | — |
| lift/runtime/fle_filedlg.c | 948 | filedlg [browser_picker] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 1010 | IFileDialog::SetFileTypes [fle_imp_IFileOpenDialog_SetFileTypes] | more file types than this model keeps (a limit of the model) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1031 | IFileDialog::GetFileTypeIndex [fle_imp_IFileOpenDialog_GetFileTypeIndex] | piFileType is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1042 | IFileDialog::Advise [fle_imp_IFileOpenDialog_Advise] | NULL pfde or pdwCookie: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1044 | IFileDialog::Advise [fle_imp_IFileOpenDialog_Advise] | more event handlers than this model keeps (a limit of the model) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1066 | IFileDialog::Unadvise [fle_imp_IFileOpenDialog_Unadvise] | a cookie that Advise did not hand out: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1081 | IFileDialog::GetOptions [fle_imp_IFileOpenDialog_GetOptions] | pfos is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1113 | IFileDialog::GetFolder [fle_imp_IFileOpenDialog_GetFolder] | ppsi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1114 | IFileDialog::GetFolder [fle_imp_IFileOpenDialog_GetFolder] | no folder was set and the page does not say which one a dialog would show (the user's last folder) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1121 | IFileDialog::GetCurrentSelection [fle_imp_IFileOpenDialog_GetCurrentSelection] | not modelled: the model has no selection while a dialog is open (Show returns at once) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1143 | IFileDialog::GetFileName [fle_imp_IFileOpenDialog_GetFileName] | pszName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1188 | IFileDialog::Close [fle_imp_IFileOpenDialog_Close] | not modelled: Show never returns to the guest while a dialog is open, so there is no moment at which the guest could close it | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1195 | IFileDialog::SetClientGuid [fle_imp_IFileOpenDialog_SetClientGuid] | guid is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1207 | IFileDialog::SetFilter [fle_imp_IFileOpenDialog_SetFilter] | the page: 'SetFilter is no longer available for use as of Windows 7' and gives no return value | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1229 | IFileDialogEvents::OnFileOk [events_on_file_ok] | the handler returned neither S_OK, S_FALSE nor E_NOTIMPL: the page names no other value | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1245 | IFileDialog::SetClientGuid [save_target] | the Save target the harness gave is not a drive-absolute guest path (C:\\...) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1255 | IFileDialog::SetClientGuid [save_target] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 1275 | IFileDialog::SetClientGuid [save_target] | FOS_PATHMUSTEXIST: the folder of the Save target the harness gave does not exist on the virtual volume (a real dialog keeps itself open) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1280 | IFileDialog::SetClientGuid [save_target] | the Save target the harness gave is an existing directory (a real dialog would open it) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1311 | IModalWindow::Show [fle_imp_IFileOpenDialog_Show] | the picker returned other than one path for a Save dialog | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1312 | IModalWindow::Show [fle_imp_IFileOpenDialog_Show] | the picker returned several items for a dialog without FOS_ALLOWMULTISELECT | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1314 | IModalWindow::Show [fle_imp_IFileOpenDialog_Show] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 1322 | IModalWindow::Show [fle_imp_IFileOpenDialog_Show] | FOS_PICKFOLDERS: the selection the harness gave is not a directory of the virtual volume | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1326 | IModalWindow::Show [fle_imp_IFileOpenDialog_Show] | (動的な文言 pa) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1349 | IFileDialog::GetResult [fle_imp_IFileOpenDialog_GetResult] | ppsi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1363 | IFileDialog::GetResult [array_of] | out of memory | (a) | — | — |
| lift/runtime/fle_filedlg.c | 1373 | IFileOpenDialog::GetResults [fle_imp_IFileOpenDialog_GetResults] | ppenum is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1381 | IFileOpenDialog::GetSelectedItems [fle_imp_IFileOpenDialog_GetSelectedItems] | not modelled: the model has no selection while a dialog is open | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1385 | — [fle_imp_IFileOpenDialog_GetSelectedItems] | not modelled: the engine has not been seen to call it (decision 0188) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1412 | CoCreateInstance [fle_filedlg_cocreate] | (動的な文言 is_save ? "CLSID_FileSaveDialog asked for an interface other than IUnknown / IModalWindow / IFileDialog / IFileSaveDialog: not modelled" : "CLSID_FileOpenDialog asked for an interface other than IUnknown / IModalWindow / IFileDialog / IFileOpenDialog: not modelled") | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1421 | IShellItem::BindToHandler [fle_imp_IShellItem_BindToHandler] | not modelled: no handler (BHID_*) of the shell is implemented | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1429 | IShellItem::GetParent [fle_imp_IShellItem_GetParent] | ppsi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1430 | IShellItem::GetParent [fle_imp_IShellItem_GetParent] | the item is a drive root: the parent is the shell's 'This PC', which is not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1445 | IShellItem::GetDisplayName [fle_imp_IShellItem_GetDisplayName] | ppszName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1468 | IShellItem::GetDisplayName [fle_imp_IShellItem_GetDisplayName] | a SIGDN value that the model does not produce (the page does not give the strings) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1482 | IShellItem::GetAttributes [fle_imp_IShellItem_GetAttributes] | psfgaoAttribs is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1485 | IShellItem::GetAttributes [fle_imp_IShellItem_GetAttributes] | an SFGAO bit that the model does not answer (S-dlg1-sfgao: FILESYSTEM, FOLDER and STREAM only) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1495 | IShellItem::Compare [fle_imp_IShellItem_Compare] | piOrder is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1502 | IShellItemArray::BindToHandler [fle_imp_IShellItemArray_BindToHandler] | not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1503 | IShellItemArray::GetPropertyStore [fle_imp_IShellItemArray_GetPropertyStore] | not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1504 | IShellItemArray::GetPropertyDescriptionList [fle_imp_IShellItemArray_GetPropertyDescriptionList] | not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1505 | IShellItemArray::GetAttributes [fle_imp_IShellItemArray_GetAttributes] | not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1511 | IShellItemArray::GetCount [fle_imp_IShellItemArray_GetCount] | pdwNumItems is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1521 | IShellItemArray::GetItemAt [fle_imp_IShellItemArray_GetItemAt] | ppsi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1528 | IShellItemArray::EnumItems [fle_imp_IShellItemArray_EnumItems] | not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1540 | SHCreateItemFromParsingName [fle_imp_SHCreateItemFromParsingName] | NULL pszPath / riid / ppv: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1541 | SHCreateItemFromParsingName [fle_imp_SHCreateItemFromParsingName] | a bind context was passed: not modelled (the page leaves its effect to the data source) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1545 | SHCreateItemFromParsingName [fle_imp_SHCreateItemFromParsingName] | an interface other than IShellItem was asked (IShellItem2 is not modelled) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1546 | SHCreateItemFromParsingName [fle_imp_SHCreateItemFromParsingName] | a parsing name that is not a drive-absolute file system path: the shell namespace is not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1570 | SHGetKnownFolderPath [fle_imp_SHGetKnownFolderPath] | NULL rfid / ppszPath: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1571 | SHGetKnownFolderPath [fle_imp_SHGetKnownFolderPath] | hToken is not NULL: another user's folders are not modelled | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1572 | SHGetKnownFolderPath [fle_imp_SHGetKnownFolderPath] | KNOWN_FOLDER_FLAG values are not modelled (dwFlags must be 0) | (b) | — | — |
| lift/runtime/fle_filedlg.c | 1588 | SHGetKnownFolderPath [fle_imp_SHGetKnownFolderPath] | a KNOWNFOLDERID that the model does not know (Profile, Documents, Desktop, Downloads, RoamingAppData, LocalAppData, ProgramFiles[X64\|X86\|Common\|CommonX64\|CommonX86], ProgramData only) | (b) | — | — |
| lift/runtime/fle_gdc.c | 34 | — [fle_gdc_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_gdc.c | 146, 174 | — [dc_new] | out of memory | (a) | — | — |
| lift/runtime/fle_gdc.c | 150 | — [dc_new] | DC handle region exhausted | (a) | — | — |
| lift/runtime/fle_gdc.c | 294 | — [window_of] | hWnd is not NULL: the DC of a window needs a window object, not implemented (only the DC of the entire screen is) | (b) | — | — |
| lift/runtime/fle_gdc.c | 358 | GetDCEx [fle_imp_GetDCEx] | DCX_EXCLUDERGN or DCX_INTERSECTRGN: the clip region (and the ownership the system takes of it) is not modelled (decision 0125; clip regions are a stop of decision 0123) | (b) | — | — |
| lift/runtime/fle_gdc.c | 387, 398 | ReleaseDC [fle_imp_ReleaseDC] | hWnd is not NULL: the DC of a window needs a window object, not implemented | (b) | — | — |
| lift/runtime/fle_gdc.c | 392 | ReleaseDC [fle_imp_ReleaseDC] | hDC was not issued by GetDC(NULL) of this runtime (other DC kinds and handle kinds are not defined here) | (b) | — | — |
| lift/runtime/fle_gdc.c | 437 | GetDeviceCaps [fle_imp_GetDeviceCaps] | hdc is not a live DC of GetDC(NULL) of this runtime (NULL, released, other kinds: not defined here) | (b) | — | — |
| lift/runtime/fle_gdc.c | 450 | GetDeviceCaps [fle_imp_GetDeviceCaps] | (組み立ての文言) index %u is not answered (the guest's call sites do not use it) | (b) | — | — |
| lift/runtime/fle_gdi.c | 33 | — [fle_gdi_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_gdi.c | 61 | clip [clip_reserve] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 145 | — [slot_get] | GDI handle region exhausted | (a) | — | — |
| lift/runtime/fle_gdi.c | 152 | — [slot_get] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 318 | — [need_mem_dc] | (動的な文言 screen_why) | (b) | — | — |
| lift/runtime/fle_gdi.c | 319 | — [need_mem_dc] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 320 | — [need_mem_dc] | hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 523 | CreatePalette [fle_imp_CreatePalette] | plpal is NULL: the page does not define this call | (b) | — | — |
| lift/runtime/fle_gdi.c | 526 | CreatePalette [fle_imp_CreatePalette] | palVersion is not 0x300 (the only value the guest passes; the page gives none) | (b) | — | — |
| lift/runtime/fle_gdi.c | 527 | CreatePalette [fle_imp_CreatePalette] | palNumEntries is not 16 (the only count the guest passes) | (b) | — | — |
| lift/runtime/fle_gdi.c | 551 | CreateFontIndirectW [fle_imp_CreateFontIndirectW] | lplf is NULL: the page does not define this call | (b) | — | — |
| lift/runtime/fle_gdi.c | 590 | CreateBitmap [fle_imp_CreateBitmap] | nWidth or nHeight is negative: the page does not define it (its ERROR_INVALID_BITMAP is for a calculated size below zero, not implemented) | (b) | — | — |
| lift/runtime/fle_gdi.c | 591 | CreateBitmap [fle_imp_CreateBitmap] | nPlanes and nBitCount are not 1 and 1: only the monochrome mask of the guest's cursor is answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 593 | CreateBitmap [fle_imp_CreateBitmap] | the calculated size of the bitmap is 2 GiB or more: the page's ERROR_INVALID_BITMAP rule is not implemented | (b) | — | — |
| lift/runtime/fle_gdi.c | 636 | CreateBitmap [get_bitmap] | handle is a bitmap and pv is NULL: the page gives the byte count for it but not for a DIB section (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 639 | CreateBitmap [get_bitmap] | (組み立ての文言) handle is a bitmap: c is 0x%x, below sizeof(BITMAP) = 0x20: the page does not say what is stored for a smaller c | (b) | — | — |
| lift/runtime/fle_gdi.c | 645 | CreateBitmap [get_bitmap] | handle is a DIB section and c is above sizeof(DIBSECTION) = 0x68: the page names sizeof(BITMAP) and sizeof(DIBSECTION) only | (b) | — | — |
| lift/runtime/fle_gdi.c | 650 | CreateBitmap [get_bitmap] | handle is a 24-bit BI_RGB DIB section and c asks for a DIBSECTION: no page gives dsBitfields for it (decision 0113) | (b) | — | — |
| lift/runtime/fle_gdi.c | 688 | CreateBitmap [get_bitmap] | (組み立ての文言) handle is a DIB section and c is 0x%x, neither sizeof(BITMAP) = 0x20 nor sizeof(DIBSECTION) = 0x68: the page names those two | (b) | — | — |
| lift/runtime/fle_gdi.c | 730 | GetObjectW [fle_imp_GetObjectW] | handle was not issued by this runtime's GDI table (NULL, a DC, another object kind or a number never issued): the page says only that failure returns zero | (b) | — | — |
| lift/runtime/fle_gdi.c | 732 | GetObjectW [fle_imp_GetObjectW] | handle was deleted by DeleteObject: the page does not say what GetObject returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 735 | GetObjectW [fle_imp_GetObjectW] | handle is the stock SYSTEM_FONT: its LOGFONT is not decided (decision 0066 gives a value only to the fonts CreateFontIndirectW made) | (b) | — | — |
| lift/runtime/fle_gdi.c | 736 | GetObjectW [fle_imp_GetObjectW] | handle is a palette: the page says GetObject stores a 2-byte entry count, the guest's calls (c = 4) are not decided | (b) | — | — |
| lift/runtime/fle_gdi.c | 737 | GetObjectW [fle_imp_GetObjectW] | handle is a stock brush: its LOGBRUSH is not decided | (b) | — | — |
| lift/runtime/fle_gdi.c | 738 | GetObjectW [fle_imp_GetObjectW] | handle is a stock pen: its LOGPEN is not decided | (b) | — | — |
| lift/runtime/fle_gdi.c | 739 | GetObjectW [fle_imp_GetObjectW] | handle is an icon: it is not an object kind the GetObject page describes | (b) | — | — |
| lift/runtime/fle_gdi.c | 741 | GetObjectW [fle_imp_GetObjectW] | handle is of a kind this table does not describe | (b) | — | — |
| lift/runtime/fle_gdi.c | 745 | GetObjectW [fle_imp_GetObjectW] | (組み立ての文言) c is 0x%x, not sizeof(LOGFONTW) = 0x5c: the page does not say what is stored for another c | (b) | — | — |
| lift/runtime/fle_gdi.c | 747 | GetObjectW [fle_imp_GetObjectW] | pv is NULL: the page returns the byte count for it, the guest never passes NULL for a font | (b) | — | — |
| lift/runtime/fle_gdi.c | 776 | DeleteObject [fle_imp_DeleteObject] | handle was not issued by this runtime's GDI table (other object kinds are not defined here) | (b) | — | — |
| lift/runtime/fle_gdi.c | 788 | DeleteObject [fle_imp_DeleteObject] | handle is an icon: the pages give no rule for DeleteObject on an icon | (b) | — | — |
| lift/runtime/fle_gdi.c | 789 | DeleteObject [fle_imp_DeleteObject] | handle is an icon or a cursor made by CreateIconIndirect: the pages give no rule for DeleteObject on one (DestroyIcon is the call) | (b) | — | — |
| lift/runtime/fle_gdi.c | 790 | DeleteObject [fle_imp_DeleteObject] | handle is a default object of the memory DCs, like a stock object: the page gives no return value for deleting one | (b) | — | — |
| lift/runtime/fle_gdi.c | 791 | DeleteObject [fle_imp_DeleteObject] | handle is a stock object: the page gives no return value for deleting one | (b) | — | — |
| lift/runtime/fle_gdi.c | 793 | DeleteObject [fle_imp_DeleteObject] | handle is the placeholder bitmap of a memory DC: the pages give no rule for deleting it | (b) | — | — |
| lift/runtime/fle_gdi.c | 834 | GetStockObject [fle_imp_GetStockObject] | (組み立ての文言) index %u is not answered (answered: the brushes 0 to 5, the pens 6 to 8 and the system font 13) | (b) | — | — |
| lift/runtime/fle_gdi.c | 869 | GetStockObject [load_icon_of_module] | hInstance is not NULL: icon resources of a module are not implemented (only IDI_APPLICATION is) | (b) | — | — |
| lift/runtime/fle_gdi.c | 871 | GetStockObject [load_icon_of_module] | the module has no RT_GROUP_ICON of that name: the page names no error code for a failed load (decision 0118) | (b) | — | — |
| lift/runtime/fle_gdi.c | 913 | LoadIconW [fle_imp_LoadIconW] | lpIconName is a string longer than 16 UTF-16 units: not answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 920 | LoadIconW [fle_imp_LoadIconW] | lpIconName is a string other than L\"MAINICON\": not answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 924 | LoadIconW [fle_imp_LoadIconW] | lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 958 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | hdc is not a live virtual screen DC (GetDC(NULL), decision 0053) | (b) | — | — |
| lift/runtime/fle_gdi.c | 959 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | lpLogfont is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 960 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | lpProc is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 961 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | dwFlags is not zero: the page says it must be zero (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 962 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | lfPitchAndFamily is not zero: the page says it must be zero (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 963 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | lfCharSet is not DEFAULT_CHARSET: only the call for all character sets is answered (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 973 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | lfFaceName is neither empty nor the face of the system font: no other font is defined (decision 0098) | (b) | — | — |
| lift/runtime/fle_gdi.c | 974 | EnumFontFamiliesExW [fle_imp_EnumFontFamiliesExW] | the guest callback mechanism is not linked | (a) | — | — |
| lift/runtime/fle_gdi.c | 1010 | CreateCompatibleDC [fle_imp_CreateCompatibleDC] | hdc is a memory DC: a DC compatible with a memory DC (the format of the bitmap selected into it) is not modelled, only NULL and a DC of GetDC(NULL) are | (b) | — | — |
| lift/runtime/fle_gdi.c | 1011 | CreateCompatibleDC [fle_imp_CreateCompatibleDC] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1012 | CreateCompatibleDC [fle_imp_CreateCompatibleDC] | hdc is neither NULL nor a DC of this runtime: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1019 | CreateCompatibleDC [fle_imp_CreateCompatibleDC] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 1066 | DeleteDC [fle_imp_DeleteDC] | hdc is a DC of GetDC(NULL): the page says an application must not delete one (ReleaseDC is the call) and not what the call returns if it does | (b) | — | — |
| lift/runtime/fle_gdi.c | 1067 | DeleteDC [fle_imp_DeleteDC] | hdc is not a DC of this runtime (a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1091 | SelectObject [fle_imp_SelectObject] | h was not issued by this runtime's GDI table (NULL, a DC, a cursor or a number never issued): the page names NULL for an error and not which handles are errors | (b) | — | — |
| lift/runtime/fle_gdi.c | 1093 | SelectObject [fle_imp_SelectObject] | h was deleted by DeleteObject: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1103 | SelectObject [fle_imp_SelectObject] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1104 | SelectObject [fle_imp_SelectObject] | hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1124, 1126 | SelectObject [fle_imp_SelectObject] | h is a palette: the page's list of objects has none, SelectPalette is the call | (b) | — | — |
| lift/runtime/fle_gdi.c | 1127 | SelectObject [fle_imp_SelectObject] | h is a DC: the page's list of objects has none | (b) | — | — |
| lift/runtime/fle_gdi.c | 1129 | SelectObject [fle_imp_SelectObject] | h is an icon or a cursor: the page's list of objects has none | (b) | — | — |
| lift/runtime/fle_gdi.c | 1130 | SelectObject [fle_imp_SelectObject] | h is of a kind this table does not describe | (b) | — | — |
| lift/runtime/fle_gdi.c | 1165 | SelectPalette [fle_imp_SelectPalette] | hPal is not a live palette of this runtime (NULL, a number never issued, deleted, or another kind of object): the page names NULL for failure and not which handles fail | (b) | — | — |
| lift/runtime/fle_gdi.c | 1199 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | the bitmap selected into hdc is gone | (b) | — | — |
| lift/runtime/fle_gdi.c | 1200 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | a DIB section is selected into hdc: the page says the result is then a DIB section, which CreateCompatibleBitmap does not make here (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 1204 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1205 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1207 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | cx or cy is negative: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1214 | CreateCompatibleBitmap [fle_imp_CreateCompatibleBitmap] | the size of the bitmap is 2 GiB or more: the page gives no rule for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1259 | CreateDIBSection [fle_imp_CreateDIBSection] | hdc is not a live DC of this runtime: the page uses it for DIB_PAL_COLORS only and does not say whether a DIB_RGB_COLORS call needs a valid one | (b) | — | — |
| lift/runtime/fle_gdi.c | 1262 | CreateDIBSection [fle_imp_CreateDIBSection] | (組み立ての文言) usage is %u, not DIB_RGB_COLORS (0): DIB_PAL_COLORS needs the logical palette of the DC (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 1264 | CreateDIBSection [fle_imp_CreateDIBSection] | pbmi is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1265 | CreateDIBSection [fle_imp_CreateDIBSection] | ppvBits is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1266 | CreateDIBSection [fle_imp_CreateDIBSection] | hSection is not NULL: a DIB over a file-mapping object is not modelled (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 1277 | CreateDIBSection [fle_imp_CreateDIBSection] | (組み立ての文言) biSize is %u, not 40 (BITMAPINFOHEADER): the core, V4 and V5 headers are not modelled | (b) | — | — |
| lift/runtime/fle_gdi.c | 1279 | CreateDIBSection [fle_imp_CreateDIBSection] | biPlanes is not 1: the page says it must be 1 | (b) | — | — |
| lift/runtime/fle_gdi.c | 1280 | CreateDIBSection [fle_imp_CreateDIBSection] | biWidth is not above 0 or biHeight is 0: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1283 | CreateDIBSection [fle_imp_CreateDIBSection] | (組み立ての文言) biBitCount is %u: only 1, 4, 8, 16, 24 and 32 are modelled | (b) | — | — |
| lift/runtime/fle_gdi.c | 1287 | CreateDIBSection [fle_imp_CreateDIBSection] | (組み立ての文言) biCompression is %u: only BI_RGB, and BI_BITFIELDS for 16 and 32 bits, are modelled | (b) | — | — |
| lift/runtime/fle_gdi.c | 1292 | CreateDIBSection [fle_imp_CreateDIBSection] | the size of the bits is 2 GiB or more: the page gives no rule for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1293 | CreateDIBSection [fle_imp_CreateDIBSection] | biSizeImage is neither 0 nor stride * \|biHeight\|: the page does not say what the call does with another value | (b) | — | — |
| lift/runtime/fle_gdi.c | 1296 | CreateDIBSection [fle_imp_CreateDIBSection] | the BI_BITFIELDS masks are not RGB 555, RGB 565 (16 bits) or 8-8-8 (32 bits): the page does not say which masks are valid | (b) | — | — |
| lift/runtime/fle_gdi.c | 1300 | CreateDIBSection [fle_imp_CreateDIBSection] | biClrUsed is above 2^biBitCount: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1302, 1305 | CreateDIBSection [fle_imp_CreateDIBSection] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 1356 | CreateDIBSection [dib_color_table] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1357 | CreateDIBSection [dib_color_table] | hdc is not a DC of this runtime (a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1369 | CreateDIBSection [dib_color_table] | prgbq is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1413 | CreateDIBSection [set_dc_color] | the colour has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette: the page does not describe it for this DC) | (b) | — | — |
| lift/runtime/fle_gdi.c | 1437 | CreateBrushIndirect [fle_imp_CreateBrushIndirect] | plbrush is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1442 | CreateBrushIndirect [fle_imp_CreateBrushIndirect] | (組み立ての文言) lbStyle is %u: only BS_SOLID (0) and BS_HOLLOW (1) are answered, the hatched and pattern styles need patterns and pixels (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 1444 | CreateBrushIndirect [fle_imp_CreateBrushIndirect] | lbColor has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it for a solid brush | (b) | — | — |
| lift/runtime/fle_gdi.c | 1467 | CreateHalftonePalette [fle_imp_CreateHalftonePalette] | hdc was deleted by DeleteDC: the page does not say what the call returns for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1468 | CreateHalftonePalette [fle_imp_CreateHalftonePalette] | hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1521 | CreateIconIndirect [fle_imp_CreateIconIndirect] | piconinfo is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1528 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmMask is not a live bitmap of this runtime: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1529 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmMask is not a monochrome bitmap (1 plane, 1 bit): the page calls it the icon monochrome mask | (b) | — | — |
| lift/runtime/fle_gdi.c | 1530 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmMask is selected into a DC: the page says it should not be and not what the call does then | (b) | — | — |
| lift/runtime/fle_gdi.c | 1534 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmColor is not NULL and not a live bitmap of this runtime: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 1535 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmColor is selected into a DC: the page says it should not be and not what the call does then | (b) | — | — |
| lift/runtime/fle_gdi.c | 1536 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmColor and hbmMask differ in size: the page says that for a colour icon they are the same size | (b) | — | — |
| lift/runtime/fle_gdi.c | 1538 | CreateIconIndirect [fle_imp_CreateIconIndirect] | hbmColor is NULL and the height of hbmMask is odd: the page says the mask of a monochrome icon is twice the height of the icon | (b) | — | — |
| lift/runtime/fle_gdi.c | 1605 | CreateIconIndirect [pixel_stop] | (組み立ての文言) %s: %s (decision 0126) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2403 | ScrollDC [fle_imp_ScrollDC] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 2496 | RealizePalette [fle_imp_RealizePalette] | the number of entries mapped to the system palette, and the colour table of the selected bitmap that a memory DC modifies, are not given by the page (decision 0108) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2527, 2535 | RealizePalette [screen_state] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 2634 | GetCurrentPositionEx [fle_imp_GetCurrentPositionEx] | lppt is NULL: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2652 | SetBkMode [fle_imp_SetBkMode] | (組み立ての文言) mode is %u, neither OPAQUE (2) nor TRANSPARENT (1): the page lists these two | (b) | — | — |
| lift/runtime/fle_gdi.c | 2671 | SetROP2 [fle_imp_SetROP2] | (組み立ての文言) rop2 is %u, not one of the 16 mix modes (1 to 16) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2692 | SetMapMode [fle_imp_SetMapMode] | (組み立ての文言) iMode is %u, not one of the 8 mapping modes (1 to 8) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2713 | SetStretchBltMode [fle_imp_SetStretchBltMode] | (組み立ての文言) mode is %u, not one of the 4 modes of the page (1 to 4): the page allows driver-specific modes and says nothing of them | (b) | — | — |
| lift/runtime/fle_gdi.c | 2761 | GetWindowOrgEx [fle_imp_GetWindowOrgEx] | lppoint is NULL: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2804 | LPtoDP [fle_imp_LPtoDP] | (組み立ての文言) the mapping mode is %u, not MM_TEXT%s: the extents of the other modes are not on the pages | (b) | — | — |
| lift/runtime/fle_gdi.c | 2807 | LPtoDP [fle_imp_LPtoDP] | lppt is NULL: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2808 | LPtoDP [fle_imp_LPtoDP] | c is not positive: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2822 | LPtoDP [fle_imp_LPtoDP] | a converted coordinate exceeds 27 bits: the page calls the results undefined | (b) | — | — |
| lift/runtime/fle_gdi.c | 2843 | SetBrushOrgEx [fle_imp_SetBrushOrgEx] | x or y is outside 0 to 7: the page reduces it by the brush size and gives no size for every brush (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2858 | GetBrushOrgEx [fle_imp_GetBrushOrgEx] | lppt is NULL: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 2905 | SaveDC [fle_imp_SaveDC] | the DC stack is deeper than 1,048,576 states | (b) | — | — |
| lift/runtime/fle_gdi.c | 2907 | SaveDC [fle_imp_SaveDC] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 2923 | SaveDC [restore_object] | an object of the saved state was deleted since: the pages do not say what RestoreDC does then | (b) | — | — |
| lift/runtime/fle_gdi.c | 2924 | SaveDC [restore_object] | the bitmap of the saved state is selected into another DC now: the pages do not say what RestoreDC does then | (b) | — | — |
| lift/runtime/fle_gdi.c | 2940 | RestoreDC [fle_imp_RestoreDC] | nSavedDC is 0: the page defines positive and negative numbers only | (b) | — | — |
| lift/runtime/fle_gdi.c | 2947 | RestoreDC [fle_imp_RestoreDC] | the text colour or the background colour differs from the saved state: the pages do not say whether RestoreDC restores colours | (b) | — | — |
| lift/runtime/fle_gdi.c | 2948 | RestoreDC [fle_imp_RestoreDC] | the current position differs from the saved state: the pages do not say whether RestoreDC restores it | (b) | — | — |
| lift/runtime/fle_gdi.c | 2949 | RestoreDC [fle_imp_RestoreDC] | the brush origin differs from the saved state: the pages do not say whether RestoreDC restores it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3001 | GetClipBox [fle_imp_GetClipBox] | (組み立ての文言) the mapping mode is %u, not MM_TEXT: the extents of the other modes are not on the pages | (b) | — | — |
| lift/runtime/fle_gdi.c | 3003 | GetClipBox [fle_imp_GetClipBox] | lprect is NULL: the page does not define it (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 3005 | GetClipBox [fle_imp_GetClipBox] | the bitmap selected into the DC is gone | (b) | — | — |
| lift/runtime/fle_gdi.c | 3039 | GetClipBox [fle_imp_GetClipBox] | the logical rectangle does not fit 32 bits | (b) | — | — |
| lift/runtime/fle_gdi.c | 3058 | UnrealizeObject [fle_imp_UnrealizeObject] | h is not a live object of this runtime (NULL, deleted, or a number never issued): the page defines no failure for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3060 | UnrealizeObject [fle_imp_UnrealizeObject] | h is not a brush or a palette made by the guest: the page says not to use a stock object and defines brushes and palettes only | (b) | — | — |
| lift/runtime/fle_gdi.c | 3074 | CreateSolidBrush [fle_imp_CreateSolidBrush] | color has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3099 | CreatePenIndirect [fle_imp_CreatePenIndirect] | plpen is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3105 | CreatePenIndirect [fle_imp_CreatePenIndirect] | (組み立ての文言) lopnStyle is %u: only PS_SOLID to PS_INSIDEFRAME (0 to 6) are answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 3107 | CreatePenIndirect [fle_imp_CreatePenIndirect] | lopnWidth.x is negative: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3108 | CreatePenIndirect [fle_imp_CreatePenIndirect] | lopnColor has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3132 | GetPaletteEntries [fle_imp_GetPaletteEntries] | hpal is NULL: the page's sentence for it is garbled and defines no result (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 3133 | GetPaletteEntries [fle_imp_GetPaletteEntries] | pPalEntries is NULL: the page's sentence for it is garbled and defines no result (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 3135 | GetPaletteEntries [fle_imp_GetPaletteEntries] | hpal is not a live palette made by CreatePalette: the default palette of a memory DC and the other handles have no entries here | (b) | — | — |
| lift/runtime/fle_gdi.c | 3136 | GetPaletteEntries [fle_imp_GetPaletteEntries] | hpal is a halftone palette: the page says its entries are an implementation detail | (b) | — | — |
| lift/runtime/fle_gdi.c | 3176 | GetPaletteEntries [destroy_icon_or_cursor] | hIcon was not issued by this runtime's GDI table (a shared cursor, a window, another kind of handle or a number never issued): the page defines no result for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3182 | GetPaletteEntries [destroy_icon_or_cursor] | hIcon is a deleted object of another kind: the page defines no result for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3183 | GetPaletteEntries [destroy_icon_or_cursor] | hIcon is not an icon or cursor made by CreateIconIndirect: the page says not to destroy a shared icon or cursor and gives no result for one | (b) | — | — |
| lift/runtime/fle_gdi.c | 3184 | GetPaletteEntries [destroy_icon_or_cursor] | hCursor is an icon (fIcon TRUE): DestroyCursor is defined for cursors, DestroyIcon for icons (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 3185 | GetPaletteEntries [destroy_icon_or_cursor] | the handle is the current cursor of SetCursor: the page says the cursor must not be in use and gives no result for one that is | (b) | — | — |
| lift/runtime/fle_gdi.c | 3212 | GetIconInfo [fle_imp_GetIconInfo] | piconinfo is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3214 | GetIconInfo [fle_imp_GetIconInfo] | hIcon is not an icon or cursor made by CreateIconIndirect (a shared handle, NULL, a deleted object or another kind): the page defines no result for it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3215 | GetIconInfo [fle_imp_GetIconInfo] | hIcon is an icon (fIcon TRUE): its hot spot is 'always in the center' on the page, with no number (decision 0111) | (b) | — | — |
| lift/runtime/fle_gdi.c | 3265 | SetCursor [fle_imp_SetCursor] | hCursor is not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect (an icon of LoadIconW, a deleted handle, another kind of handle): the page lists those four sources | (b) | — | — |
| lift/runtime/fle_gdi.c | 3372 | GetDeviceCaps [fle_gdi_memdc_caps] | the bitmap selected into the memory DC is gone | (b) | — | — |
| lift/runtime/fle_gdi.c | 3382 | GetDeviceCaps [fle_gdi_memdc_caps] | (組み立ての文言) index %u of a memory DC: the page describes the screen for it (only BITSPIXEL 12 and PLANES 14 are answered, from the bitmap selected into t… | (b) | — | — |
| lift/runtime/fle_gdi.c | 3396 | — [fle_gdi_memdc_caps] | (組み立ての文言) index %u of a memory DC: the page describes the screen for it (only BITSPIXEL 12 and PLANES 14 are answered, from the bitmap selected into t… | (b) | — | — |
| lift/runtime/fle_gdi.c | 3810 | CreatePatternBrush [fle_imp_CreatePatternBrush] | hbm is not a live bitmap of this runtime: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 3811 | CreatePatternBrush [fle_imp_CreatePatternBrush] | hbm is a DIB section: only the 8-by-8 monochrome device-dependent bitmap of the guest is answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 3812 | CreatePatternBrush [fle_imp_CreatePatternBrush] | hbm is not 8 by 8 pixels, 1 plane, 1 bit: only the monochrome pattern of the guest is answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 3813 | CreatePatternBrush [fle_imp_CreatePatternBrush] | out of memory for the pixels of a device-dependent bitmap | (a) | — | — |
| lift/runtime/fle_gdi.c | 4013 | CreatePen [fle_imp_CreatePen] | (組み立ての文言) iStyle is %d: only PS_SOLID to PS_INSIDEFRAME (0 to 6) are answered | (b) | — | — |
| lift/runtime/fle_gdi.c | 4015 | CreatePen [fle_imp_CreatePen] | cWidth is negative: the page does not define it | (b) | — | — |
| lift/runtime/fle_gdi.c | 4016 | CreatePen [fle_imp_CreatePen] | color has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it | (b) | — | — |
| lift/runtime/fle_gdi.c | 4292 | — [fle_gdi_dc_font] | (動的な文言 Why " (decision 0111)") | (b) | — | — |
| lift/runtime/fle_gdi.c | 4384 | region [rgn_reserve] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 4422, 4429 | RectVisible [rgn_set_of] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 4475 | region [rb_push] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 4570 | region [rgn_combine] | out of memory | (a) | — | — |
| lift/runtime/fle_gdi.c | 4657 | CreateRectRgnIndirect [fle_imp_CreateRectRgnIndirect] | lprect is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_gdi.c | 4844 | GetRgnBox [fle_imp_GetRgnBox] | lprc is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_gdi.c | 4955 | RectInRegion [fle_imp_RectInRegion] | lprect is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_gdi.c | 5184 | ExtSelectClipRgn [clip_rect_op] | (組み立ての文言) the mapping mode is %u, not MM_TEXT: the extents of the other modes are not on the pages | (b) | — | — |
| lift/runtime/fle_glob.c | 24 | — [fle_glob_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_glob.c | 86 | GlobalAlloc [fx_insert] | out of memory | (a) | — | — |
| lift/runtime/fle_glob.c | 132 | GlobalAlloc [fle_glob_movable_from] | movable handle region exhausted | (a) | — | — |
| lift/runtime/fle_glob.c | 140 | GlobalAlloc [fle_glob_movable_from] | out of memory | (a) | — | — |
| lift/runtime/fle_glob.c | 190 | GlobalAlloc [fle_imp_GlobalAlloc] | uFlags has a bit that the page does not give a number for | (b) | — | — |
| lift/runtime/fle_glob.c | 194 | GlobalAlloc [fle_imp_GlobalAlloc] | movable handle region exhausted | (a) | — | — |
| lift/runtime/fle_glob.c | 208 | GlobalAlloc [fle_imp_GlobalAlloc] | out of memory | (a) | — | — |
| lift/runtime/fle_glob.c | 256 | GlobalFree [fle_imp_GlobalFree] | handle is not a live GlobalAlloc block (freed, from another allocator, or stray) | (b) | — | — |
| lift/runtime/fle_glob.c | 278 | GlobalLock [fle_imp_GlobalLock] | lock count overflow | (b) | — | — |
| lift/runtime/fle_glob.c | 283 | GlobalLock [fle_imp_GlobalLock] | handle is not a live GlobalAlloc object | (b) | — | — |
| lift/runtime/fle_glob.c | 313 | GlobalUnlock [fle_imp_GlobalUnlock] | handle is not a live GlobalAlloc object | (b) | — | — |
| lift/runtime/fle_glob.c | 353 | atom [atom_tab] | out of memory | (a) | — | — |
| lift/runtime/fle_glob.c | 372 | — [atom_arg] | empty atom name is not defined | (b) | — | — |
| lift/runtime/fle_glob.c | 374 | — [atom_arg] | atom name with a non-ASCII character (case folding of other letters is not defined) | (b) | — | — |
| lift/runtime/fle_glob.c | 442 | — [atom_add] | atom reference count overflow | (b) | — | — |
| lift/runtime/fle_glob.c | 448 | — [atom_add] | atom table full (not defined by the pages) | (b) | — | — |
| lift/runtime/fle_glob.c | 480 | — [atom_findx] | \"#dddd\" outside 1..0xBFFF is not defined for a search | (b) | — | — |
| lift/runtime/fle_imagehlp.c | 39 | ImageNtHeader [fle_imp_ImageNtHeader] | Base is not the lifted image (0x400000); the page does not say which Base fails, so no NULL is invented | (b) | — | — |
| lift/runtime/fle_imagehlp.c | 40 | ImageNtHeader [fle_imp_ImageNtHeader] | no MZ signature at the lifted image's base | (b) | — | — |
| lift/runtime/fle_imagehlp.c | 42 | ImageNtHeader [fle_imp_ImageNtHeader] | e_lfanew of the lifted image is out of range | (b) | — | — |
| lift/runtime/fle_imagehlp.c | 43 | ImageNtHeader [fle_imp_ImageNtHeader] | no PE signature at e_lfanew of the lifted image | (b) | — | — |
| lift/runtime/fle_imagelist.c | 153 | — [il_stop] | (動的な文言 buf) | (b) | — | — |
| lift/runtime/fle_imagelist.c | 281 | ImageList_DragEnter [drag_dc] | GetDCEx is not linked | (a) | — | — |
| lift/runtime/fle_imagelist.c | 333 | ImageList_Replace [drag_show] | out of memory | (a) | — | — |
| lift/runtime/fle_imagelist.c | 334, 336 | ImageList_Replace [drag_show] | the window's drawing surface is not a 32-bit surface that can be written | (b) | — | — |
| lift/runtime/fle_input.c | 322 | TrackMouseEvent [fle_input_track] | more than 32 windows are tracked at once (a limit of this environment) | (b) | — | — |
| lift/runtime/fle_input.c | 1161 | GetKeyboardState [fle_imp_GetKeyboardState] | lpKeyState is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_input.c | 1174 | SetKeyboardState [fle_imp_SetKeyboardState] | lpKeyState is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_input.c | 1203 | MapVirtualKeyW [fle_imp_MapVirtualKeyW] | uMapType is not one of the five MAPVK values of the page | (b) | — | — |
| lift/runtime/fle_kbd.c | 32 | GetKeyboardLayout [fle_imp_GetKeyboardLayout] | idThread is the virtual GetCurrentThreadId value: there is no thread table to resolve it, only idThread 0 is answered (decisions 0056, 0063) | (b) | — | — |
| lift/runtime/fle_kbd.c | 35 | GetKeyboardLayout [fle_imp_GetKeyboardLayout] | (組み立ての文言) idThread %u is not 0: there is no thread table, only idThread 0 (the current thread) is answered (decision 0063) | (b) | — | — |
| lift/runtime/fle_layered.c | 95 | — [stopf] | (動的な文言 buf) | (b) | — | — |
| lift/runtime/fle_load.c | 106 | — [refs_init] | the virtual module table is larger than the loader's reference table | (b) | — | — |
| lift/runtime/fle_load.c | 130 | — [normalize] | internal: a module name with a path reached the name rules (load_with_path takes it) | (a) | — | — |
| lift/runtime/fle_load.c | 151 | — [read_wide_name] | lpLibFileName is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_load.c | 155 | — [read_wide_name] | a module name longer than 256 characters is not supported | (b) | — | — |
| lift/runtime/fle_load.c | 166 | — [read_ansi_name] | lpLibFileName is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_load.c | 170 | — [read_ansi_name] | a module name longer than 256 characters is not supported | (b) | — | — |
| lift/runtime/fle_load.c | 273 | — [stop_file_not_a_module] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_load.c | 315, 328 | — [load_with_path] | a module name with ':' in a form the volume does not model | (b) | — | — |
| lift/runtime/fle_load.c | 316 | — [load_with_path] | a drive-relative module name (C:name) is not modelled | (b) | — | — |
| lift/runtime/fle_load.c | 321 | — [load_with_path] | a UNC or device module path (\\\\...) is not modelled | (b) | — | — |
| lift/runtime/fle_load.c | 336 | — [load_with_path] | a path names a real module but fle_vfs.c is not linked (no virtual volume to check the file in) | (a) | — | — |
| lift/runtime/fle_load.c | 352 | — [load_with_path] | (組み立ての文言) a path to \"%s\", a DLL that a Windows installation ships (the roster of decision 0233), in a folder of Windows: this environment holds no s… | (b) | — | — |
| lift/runtime/fle_load.c | 357 | — [load_with_path] | (組み立ての文言) a path to the virtual module \"%s\": the virtual modules have no path in this environment, so neither a handle nor a failure is true for it … | (b) | — | — |
| lift/runtime/fle_load.c | 390 | — [api_set_load] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_load.c | 453 | — [load_by_name] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_load.c | 497 | LoadLibraryA [fle_imp_LoadLibraryA] | a module path with a byte above 0x7F (the page does not define the ANSI code page; decision 0233) | (b) | — | — |
| lift/runtime/fle_load.c | 519 | LoadLibraryExW [fle_imp_LoadLibraryExW] | hFile is not NULL (the page: reserved, must be NULL) | (b) | — | — |
| lift/runtime/fle_load.c | 520 | LoadLibraryExW [fle_imp_LoadLibraryExW] | dwFlags has a bit that is not modelled (see fle_load.c) | (b) | — | — |
| lift/runtime/fle_load.c | 522 | LoadLibraryExW [fle_imp_LoadLibraryExW] | LOAD_LIBRARY_AS_DATAFILE with LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE (the page says not to) | (b) | — | — |
| lift/runtime/fle_load.c | 539 | FreeLibrary [fle_imp_FreeLibrary] | hLibModule is the lifted DLL itself (not modelled) | (b) | — | — |
| lift/runtime/fle_load.c | 549 | FreeLibrary [fle_imp_FreeLibrary] | hLibModule is not a virtual module handle (the page names no error code for it) | (b) | — | — |
| lift/runtime/fle_load.c | 551 | FreeLibrary [fle_imp_FreeLibrary] | the reference count would reach zero, which would unload a module the process imports (not modelled) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 65 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) type %u is not IMAGE_CURSOR (2): bitmaps and icons are not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 69 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) fuLoad %#x is not exactly LR_SHARED: not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 73 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) size %dx%d is not 32x32: not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 75 | LoadImageW [fle_imp_LoadImageW] | name is a pointer to a string, not an integer identifier: not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 81 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) hInst is NULL and name MAKEINTRESOURCE(%u) is not one of the 13 standard cursors the guest loads: not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 89 | LoadImageW [fle_imp_LoadImageW] | hInst is neither NULL nor the lifted DLL: not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_loadimage.c | 99 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) the lifted DLL has no cursor MAKEINTRESOURCE(%u) (RT_GROUP_CURSOR): real Windows fails with NULL, but Learn gives no GetLastError value for … | (b) | — | — |
| lift/runtime/fle_loadimage.c | 104 | LoadImageW [fle_imp_LoadImageW] | (組み立ての文言) the DLL's cursor MAKEINTRESOURCE(%u) is not one the guest loads (32762..32767): not answered (decision 0061) | (b) | — | — |
| lift/runtime/fle_monitor.c | 54 | EnumDisplayMonitors [fle_imp_EnumDisplayMonitors] | hdc is not NULL: the DC of a monitor is not implemented, only (NULL, NULL) is answered (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 55 | EnumDisplayMonitors [fle_imp_EnumDisplayMonitors] | lprcClip is not NULL: clipping is not implemented, only (NULL, NULL) is answered (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 56 | EnumDisplayMonitors [fle_imp_EnumDisplayMonitors] | lpfnEnum is NULL: the page does not define it (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 85 | EnumDisplayMonitors [fle_monitor_resolve] | (組み立ての文言) dwFlags %#x is not MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY or MONITOR_DEFAULTTONEAREST: the pages list no other value (decision 0126… | (b) | — | — |
| lift/runtime/fle_monitor.c | 113 | MonitorFromRect [fle_imp_MonitorFromRect] | lprc is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_monitor.c | 140 | GetMonitorInfoW [fle_imp_GetMonitorInfoW] | (組み立ての文言) hMonitor 0x% | (b) | — | — |
| lift/runtime/fle_monitor.c | 142 | GetMonitorInfoW [fle_imp_GetMonitorInfoW] | lpmi is NULL: the page does not define it (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 144 | GetMonitorInfoW [fle_imp_GetMonitorInfoW] | cbSize is sizeof(MONITORINFOEXW) 0x68: the device name is not decided, only MONITORINFO (0x28) is answered (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 145 | GetMonitorInfoW [fle_imp_GetMonitorInfoW] | cbSize is 0x48 = sizeof(MONITORINFOEXA), not a form of the Unicode function: only MONITORINFO (0x28) is answered (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 148 | GetMonitorInfoW [fle_imp_GetMonitorInfoW] | (組み立ての文言) cbSize 0x%x is not sizeof(MONITORINFO) 0x28: the page gives no failure value for it (decision 0065) | (b) | — | — |
| lift/runtime/fle_monitor.c | 180 | EnumDisplaySettingsW [fle_imp_EnumDisplaySettingsW] | lpszDeviceName is not NULL: only the current display device is answered (decision 0098) | (b) | — | — |
| lift/runtime/fle_monitor.c | 183 | EnumDisplaySettingsW [fle_imp_EnumDisplaySettingsW] | (組み立ての文言) iModeNum 0x%x is not ENUM_CURRENT_SETTINGS: no other mode is answered (decision 0098) | (b) | — | — |
| lift/runtime/fle_monitor.c | 185 | EnumDisplaySettingsW [fle_imp_EnumDisplaySettingsW] | lpDevMode is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_monitor.c | 189 | EnumDisplaySettingsW [fle_imp_EnumDisplaySettingsW] | (組み立ての文言) dmSize 0x%x is not sizeof(DEVMODEW) 0xdc: the page defines no failure value for it (decision 0098) | (b) | — | — |
| lift/runtime/fle_msctf.c | 12 | SetInputScope [fle_imp_SetInputScope] | msctf.dll is a virtual module that exports it, but this runtime has no window table to attach an input scope to (the page's S_OK and E_INVALIDARG both depend on the window); no HRESULT is invented | (b) | — | — |
| lift/runtime/fle_ole.c | 33 | — [check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_ole.c | 58 | — [com_init] | pvReserved is not NULL (the page: must be NULL) | (b) | — | — |
| lift/runtime/fle_ole.c | 59 | — [com_init] | initialization count overflow | (b) | — | — |
| lift/runtime/fle_ole.c | 77 | CoUninitialize [fle_imp_CoUninitialize] | no successful CoInitialize/OleInitialize to balance | (b) | — | — |
| lift/runtime/fle_ole.c | 78 | CoUninitialize [fle_imp_CoUninitialize] | the remaining initializations come from OleInitialize and need OleUninitialize | (b) | — | — |
| lift/runtime/fle_ole.c | 89 | OleUninitialize [fle_imp_OleUninitialize] | no successful OleInitialize to balance | (b) | — | — |
| lift/runtime/fle_ole.c | 103 | CoInitializeEx [fle_imp_CoInitializeEx] | dwCoInit is not COINIT_APARTMENTTHREADED (the guest is a single-threaded STA: decision 0037) | (b) | — | — |
| lift/runtime/fle_ole.c | 139 | CoTaskMemFree [fle_imp_CoTaskMemFree] | pointer is not a live CoTaskMemAlloc block (freed, from another allocator, or stray) | (b) | — | — |
| lift/runtime/fle_ole.c | 186 | CoInitializeEx [special_folder_text] | (組み立ての文言) csidl %#x is not answered (only CSIDL_FONTS 0x14, CSIDL_APPDATA 0x1a and CSIDL_SYSTEM 0x25 are) | (b) | — | — |
| lift/runtime/fle_ole.c | 196 | SHGetSpecialFolderLocation [fle_imp_SHGetSpecialFolderLocation] | hwnd is not NULL (the page says Reserved) | (b) | — | — |
| lift/runtime/fle_ole.c | 198 | SHGetSpecialFolderLocation [fle_imp_SHGetSpecialFolderLocation] | ppidl is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 221 | SHGetPathFromIDListW [fle_imp_SHGetPathFromIDListW] | pidl is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 222 | SHGetPathFromIDListW [fle_imp_SHGetPathFromIDListW] | pszPath is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 225 | SHGetPathFromIDListW [fle_imp_SHGetPathFromIDListW] | pidl is not an ITEMIDLIST that SHGetSpecialFolderLocation of this runtime issued (and that is still live): other item identifier lists are not modelled | (b) | — | — |
| lift/runtime/fle_ole.c | 275 | ILCreateFromPathW [fle_imp_ILCreateFromPathW] | pszPath is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 276 | ILCreateFromPathW [fle_imp_ILCreateFromPathW] | pszPath is empty: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 277 | ILCreateFromPathW [fle_imp_ILCreateFromPathW] | no virtual file system is linked: the path cannot be looked up | (b) | — | — |
| lift/runtime/fle_ole.c | 303 | ILCreateFromPathW [fle_ole_il_free] | pidl is not a live ITEMIDLIST of this runtime (freed, from another allocator, or stray): the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 320 | SHChangeNotifyRegister [fle_imp_SHChangeNotifyRegister] | cEntries is not 1: the page says the array should always be set to one and gives no behaviour for another count | (b) | — | — |
| lift/runtime/fle_ole.c | 321 | SHChangeNotifyRegister [fle_imp_SHChangeNotifyRegister] | pshcne is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 329 | SHChangeNotifyRegister [fle_imp_SHChangeNotifyRegister] | the entry's pidl is not an ITEMIDLIST that a shell function of this runtime issued and that is still live: other lists are not modelled | (b) | — | — |
| lift/runtime/fle_ole.c | 339 | SHChangeNotifyRegister [fle_imp_SHChangeNotifyRegister] | no window table is linked: the window cannot be checked | (a) | — | — |
| lift/runtime/fle_ole.c | 423 | SHChangeNotifyRegister [dragdrop_unknown] | the guest callback mechanism is not linked: IUnknown cannot be called | (a) | — | — |
| lift/runtime/fle_ole.c | 438 | RegisterDragDrop [fle_imp_RegisterDragDrop] | pDropTarget is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 439 | RegisterDragDrop [fle_imp_RegisterDragDrop] | no window table is linked: the window cannot be checked | (a) | — | — |
| lift/runtime/fle_ole.c | 473 | RevokeDragDrop [fle_imp_RevokeDragDrop] | no window table is linked: the window cannot be checked | (a) | — | — |
| lift/runtime/fle_ole.c | 570 | StringFromCLSID [fle_imp_StringFromCLSID] | NULL argument (the page does not define it) | (b) | — | — |
| lift/runtime/fle_ole.c | 606 | CLSIDFromString [fle_imp_CLSIDFromString] | a string that does not start with '{' is a ProgID; ProgID resolution is not modelled | (b) | — | — |
| lift/runtime/fle_ole.c | 626 | IsEqualGUID [fle_imp_IsEqualGUID] | NULL argument (the page does not define it) | (b) | — | — |
| lift/runtime/fle_ole.c | 659 | CLSIDFromString [reg_has_key] | the virtual registry answered with an unexpected status | (b) | — | — |
| lift/runtime/fle_ole.c | 675 | ProgIDFromCLSID [fle_imp_ProgIDFromCLSID] | NULL argument (the page does not define it) | (b) | — | — |
| lift/runtime/fle_ole.c | 679, 681, 683 | ProgIDFromCLSID [fle_imp_ProgIDFromCLSID] | the CLSID key exists in the virtual registry; reading its ProgID is not defined by the page | (b) | — | — |
| lift/runtime/fle_ole.c | 706 | CoCreateInstance [fle_imp_CoCreateInstance] | rclsid is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_ole.c | 707 | CoCreateInstance [fle_imp_CoCreateInstance] | ppv is NULL: the page's E_POINTER (0x80004003) is winerror.h, not a Learn page (decision 0129) | (b) | — | — |
| lift/runtime/fle_ole.c | 717, 719, 721 | CoCreateInstance [fle_imp_CoCreateInstance] | the CLSID is registered in the virtual registry: no class of this virtual machine is implemented (decision 0129) | (b) | — | — |
| lift/runtime/fle_ole.c | 748 | CoCreateInstance [imalloc_slot_va] | the response table has no row for an IMalloc method (decision 0098) | (b) | — | — |
| lift/runtime/fle_ole.c | 757 | SHGetMalloc [fle_imp_SHGetMalloc] | ppMalloc is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_ole.c | 763 | SHGetMalloc [fle_imp_SHGetMalloc] | out of memory | (a) | — | — |
| lift/runtime/fle_ole.c | 771 | SHGetMalloc [fle_imp_SHGetMalloc] | reference count overflow | (b) | — | — |
| lift/runtime/fle_ole.c | 780 | SHGetMalloc [imalloc_this] | the object is not the one SHGetMalloc returned (decision 0098) | (b) | — | — |
| lift/runtime/fle_ole.c | 789 | IMalloc::Release [fle_imp_IMalloc_Release] | no reference outstanding: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_ole.c | 801 | IMalloc::Free [fle_imp_IMalloc_Free] | pointer is not a live task-allocator block (freed, from another allocator, or stray) | (b) | — | — |
| lift/runtime/fle_ole.c | 817 | IMalloc [fle_imp_IMalloc_unimplemented] | a method other than Release, Free and DidAlloc was called through the SHGetMalloc object: the guest's call sites use no other (decision 0098) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 26 | — [check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_oleaut.c | 123 | — [bstr_len] | pointer is not a live BSTR (freed, never allocated by SysAllocStringLen/SysReAllocStringLen, or a plain string) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 143 | — [bstr_dup] | out of memory while copying a BSTR | (a) | — | — |
| lift/runtime/fle_oleaut.c | 181 | SysReAllocStringLen [fle_imp_SysReAllocStringLen] | pbstr is NULL (the page: access violation) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 187 | SysReAllocStringLen [fle_imp_SysReAllocStringLen] | psz is inside the old BSTR and len reads past its terminator | (b) | — | — |
| lift/runtime/fle_oleaut.c | 223 | SysReAllocStringLen [classify] | VT_RECORD needs IRecordInfo call-backs into guest code | (b) | — | — |
| lift/runtime/fle_oleaut.c | 224 | SysReAllocStringLen [classify] | VT_I8/VT_UI8 in a VARIANT: the VARENUM page does not list them for VARIANT, the VARIANT page has llVal/ullVal | (b) | — | — |
| lift/runtime/fle_oleaut.c | 231 | SysReAllocStringLen [classify] | VT_BYREF with VT_EMPTY/VT_NULL is not defined by the pages | (b) | — | — |
| lift/runtime/fle_oleaut.c | 244 | SysReAllocStringLen [classify] | VT_VARIANT without VT_BYREF: the pages only describe VT_VARIANT \| VT_BYREF | (b) | — | — |
| lift/runtime/fle_oleaut.c | 274 | SysReAllocStringLen [sa_elem] | VT_RECORD arrays need IRecordInfo call-backs into guest code | (b) | — | — |
| lift/runtime/fle_oleaut.c | 275 | SysReAllocStringLen [sa_elem] | VT_I8/VT_UI8 element type: the page calls all other types legal but the VARENUM table does not list them for safe arrays | (b) | — | — |
| lift/runtime/fle_oleaut.c | 276 | SysReAllocStringLen [sa_elem] | element VARTYPE is not in the VARENUM Safe array column and the page's \"all other types are legal\" does not say what it means | (b) | — | — |
| lift/runtime/fle_oleaut.c | 295 | SysReAllocStringLen [sa_vt] | psa is not a descriptor made by SafeArrayCreate (freed, guest-built, or stray) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 310 | SysReAllocStringLen [array_release] | VT_ARRAY variant with a NULL parray is not defined | (b) | — | — |
| lift/runtime/fle_oleaut.c | 322 | SysReAllocStringLen [array_release] | releasing an object in a safe array needs IUnknown::Release (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 344 | SysReAllocStringLen [array_copy] | VT_ARRAY variant with a NULL parray is not defined | (b) | — | — |
| lift/runtime/fle_oleaut.c | 351 | SysReAllocStringLen [array_copy] | out of memory while copying a safe array | (a) | — | — |
| lift/runtime/fle_oleaut.c | 362 | SysReAllocStringLen [array_copy] | copying an object in a safe array needs IUnknown::AddRef (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 364 | SysReAllocStringLen [array_copy] | an element VARIANT of the array is invalid | (b) | — | — |
| lift/runtime/fle_oleaut.c | 385 | SysReAllocStringLen [release_iface] | VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::Release (guest call-back), and the callback mechanism is not linked | (a) | — | — |
| lift/runtime/fle_oleaut.c | 437 | SysReAllocStringLen [var_copy_val] | copying VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::AddRef (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 487 | SysReAllocStringLen [var_deref_into] | a by-reference VARIANT with a NULL pointer is not defined by the pages | (b) | — | — |
| lift/runtime/fle_oleaut.c | 497 | SysReAllocStringLen [var_deref_into] | a contained VT_VARIANT \| VT_BYREF variant (the VARIANT page forbids it) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 503 | SysReAllocStringLen [var_deref_into] | dereferencing VT_UNKNOWN/VT_DISPATCH with an object needs IUnknown::AddRef (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 523 | VariantInit [fle_imp_VariantInit] | pvarg is NULL | (b) | — | — |
| lift/runtime/fle_oleaut.c | 554 | VariantCopy [fle_imp_VariantCopy] | dest equals src (the page does not define it) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 580 | VariantCopyInd [fle_imp_VariantCopyInd] | dest equals src (the page does not define it) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 620 | VariantChangeType [fle_imp_VariantChangeType] | wFlags is not 0 (the flag values are not on the page) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 621 | VariantChangeType [fle_imp_VariantChangeType] | a by-reference or array target type is not defined by the page | (b) | — | — |
| lift/runtime/fle_oleaut.c | 622 | VariantChangeType [fle_imp_VariantChangeType] | array sources are not defined by the page | (b) | — | — |
| lift/runtime/fle_oleaut.c | 624 | VariantChangeType [fle_imp_VariantChangeType] | conversion through a by-reference source to another type needs coercion rules the page does not give | (b) | — | — |
| lift/runtime/fle_oleaut.c | 625 | VariantChangeType [fle_imp_VariantChangeType] | dereferencing the source failed | (b) | — | — |
| lift/runtime/fle_oleaut.c | 630 | VariantChangeType [fle_imp_VariantChangeType] | conversion between different types needs the coercion rules the page does not give | (b) | — | — |
| lift/runtime/fle_oleaut.c | 657 | SafeArrayCreate [fle_imp_SafeArrayCreate] | cDims is not 1: the order of several dimensions is not settled by the pages | (b) | — | — |
| lift/runtime/fle_oleaut.c | 663 | SafeArrayCreate [fle_imp_SafeArrayCreate] | cElements 0 is not defined by the page | (b) | — | — |
| lift/runtime/fle_oleaut.c | 784 | SafeArrayUnaccessData [sa_elem_addr] | descriptor has more than one dimension | (b) | — | — |
| lift/runtime/fle_oleaut.c | 816 | SafeArrayGetElement [fle_imp_SafeArrayGetElement] | copying an object out of a safe array needs IUnknown::AddRef (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 856 | SafeArrayPutElement [fle_imp_SafeArrayPutElement] | storing an object in a safe array needs IUnknown::AddRef (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 857 | SafeArrayPutElement [fle_imp_SafeArrayPutElement] | replacing an object in a safe array needs IUnknown::Release (guest call-back) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 905 | SafeArrayGetElemsize [fle_imp_SafeArrayGetElemsize] | psa is NULL (the page names no result) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 921 | GetErrorInfo [fle_imp_GetErrorInfo] | dwReserved is not zero (the page: must be zero) | (b) | — | — |
| lift/runtime/fle_oleaut.c | 922 | GetErrorInfo [fle_imp_GetErrorInfo] | pperrinfo is NULL | (b) | — | — |
| lift/runtime/fle_proc.c | 30 | — [fle_proc_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_proc.c | 79 | SetErrorMode [fle_imp_SetErrorMode] | a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_proc.c | 101 | SetThreadErrorMode [fle_imp_SetThreadErrorMode] | a bit that neither the page of SetThreadErrorMode nor that of SetErrorMode lists | (b) | — | — |
| lift/runtime/fle_proc.c | 135 | SetPriorityClass [fle_imp_SetPriorityClass] | a handle that GetCurrentProcess did not return | (b) | — | — |
| lift/runtime/fle_proc.c | 162 | SetPriorityClass [fle_imp_SetPriorityClass] | a priority class that the page does not list | (b) | — | — |
| lift/runtime/fle_proc.c | 180 | GetThreadPriority [fle_imp_GetThreadPriority] | a handle that GetCurrentThread or CreateThread did not return | (b) | — | — |
| lift/runtime/fle_proc.c | 203 | SetThreadPriority [fle_imp_SetThreadPriority] | a handle that GetCurrentThread or CreateThread did not return | (b) | — | — |
| lift/runtime/fle_proc.c | 216 | SetThreadPriority [fle_imp_SetThreadPriority] | THREAD_MODE_BACKGROUND_BEGIN with a handle other than GetCurrentThread's: the page says it is for the calling thread only | (b) | — | — |
| lift/runtime/fle_proc.c | 225 | SetThreadPriority [fle_imp_SetThreadPriority] | THREAD_MODE_BACKGROUND_END with a handle other than GetCurrentThread's: the page says it is for the calling thread only | (b) | — | — |
| lift/runtime/fle_proc.c | 240 | SetThreadPriority [fle_imp_SetThreadPriority] | a REALTIME-only priority value while the class is not REALTIME | (b) | — | — |
| lift/runtime/fle_proc.c | 245 | SetThreadPriority [fle_imp_SetThreadPriority] | a priority value that the page does not list | (b) | — | — |
| lift/runtime/fle_proc.c | 284 | ExitProcess [fle_imp_ExitProcess] | (組み立ての文言) the program asked to end the process (exit code %u); stopped here, not exited | (b) | — | — |
| lift/runtime/fle_proc.c | 294 | TerminateProcess [fle_imp_TerminateProcess] | (組み立ての文言) the program asked to terminate itself (exit code %u); stopped here | (b) | — | — |
| lift/runtime/fle_proc.c | 297 | TerminateProcess [fle_imp_TerminateProcess] | (組み立ての文言) handle %#llx is not the current process (no other process exists here) | (b) | — | — |
| lift/runtime/fle_proc.c | 308 | — [fle_imp_TerminateProcess] | (組み立ての文言) handle %#llx is not the current process (no other process exists here) | (b) | — | — |
| lift/runtime/fle_proc.c | 319 | RtlUnwindEx [fle_imp_RtlUnwindEx] | structured exception handling is not modelled (reached only when the lifted DLL unwinds a C++ exception) | (b) | — | — |
| lift/runtime/fle_proc.c | 388 | RaiseException [fle_imp_RaiseException] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_proc.c | 441 | UuidCreate [fle_imp_UuidCreate] | Uuid is NULL: the page does not define it (decision 0118) | (b) | — | — |
| lift/runtime/fle_realmod.c | 44 | — [table_check] | fle_realmod_count is larger than the 192 real modules this runtime holds | (b) | — | — |
| lift/runtime/fle_realmod.c | 49 | — [table_check] | a real module has no name | (b) | — | — |
| lift/runtime/fle_realmod.c | 51 | — [table_check] | a real module name is not lower-case | (b) | — | — |
| lift/runtime/fle_realmod.c | 52 | — [table_check] | a real module name has a path | (b) | — | — |
| lift/runtime/fle_realmod.c | 57 | — [table_check] | (組み立ての文言) real module \"%s\" has the name of a virtual module | (b) | — | — |
| lift/runtime/fle_realmod.c | 63 | — [table_check] | (組み立ての文言) real module \"%s\": base %#llx is not usable (a nonzero multiple of 64 K, not 0x400000, below the virtual module handles) | (b) | — | — |
| lift/runtime/fle_realmod.c | 68 | — [table_check] | (組み立ての文言) real module \"%s\" repeats a name or a base of an earlier entry | (b) | — | — |
| lift/runtime/fle_realmod.c | 170 | — [read_headers] | (組み立ての文言) real module \"%s\": no PE image is mapped at %#llx | (b) | — | — |
| lift/runtime/fle_realmod.c | 173 | — [read_headers] | a real module is not a PE32+ image | (b) | — | — |
| lift/runtime/fle_realmod.c | 175 | — [read_headers] | a real module has no entry point (a DLL without DllMain is not modelled) | (b) | — | — |
| lift/runtime/fle_realmod.c | 252 | — [fle_realmod_free] | FreeLibrary of a real module that is not loaded (not modelled) | (b) | — | — |
| lift/runtime/fle_realmod.c | 274 | — [fle_realmod_file_name] | the file name of a real module that is not loaded | (b) | — | — |
| lift/runtime/fle_realmod.c | 291 | — [fle_realmod_getproc] | GetProcAddress on a real module that is not loaded | (b) | — | — |
| lift/runtime/fle_realmod.c | 293 | — [fle_realmod_getproc] | the export directory of a real module is too small | (b) | — | — |
| lift/runtime/fle_realmod.c | 307 | — [fle_realmod_getproc] | an export name of a real module points past its function table | (b) | — | — |
| lift/runtime/fle_realmod.c | 309 | — [fle_realmod_getproc] | an export of a real module has a zero RVA | (b) | — | — |
| lift/runtime/fle_realmod.c | 310 | — [fle_realmod_getproc] | an export forwarder of a real module is not modelled | (b) | — | — |
| lift/runtime/fle_reg.c | 73 | registry [dup_w] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 90, 99 | registry [kid_add] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 162 | registry [reg_journal] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 213 | registry [predefined] | HKEY_PERFORMANCE_DATA / _TEXT / _NLSTEXT are not implemented | (b) | — | — |
| lift/runtime/fle_reg.c | 230 | — [key_arg] | hKey is neither a predefined key nor a handle issued by the virtual registry | (b) | — | — |
| lift/runtime/fle_reg.c | 244 | registry [kh_alloc] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 248 | registry [kh_alloc] | key handle region exhausted | (a) | — | — |
| lift/runtime/fle_reg.c | 268 | — [check_sam] | samDesired has bits that are not implemented (generic, MAXIMUM_ALLOWED) | (b) | — | — |
| lift/runtime/fle_reg.c | 289 | — [split_key] | key name component longer than 255 characters | (b) | — | — |
| lift/runtime/fle_reg.c | 290 | — [split_key] | key path has more levels than the page allows per call | (b) | — | — |
| lift/runtime/fle_reg.c | 354 | registry [warn_empty_component] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 379 | — [open_common] | ulOptions is not 0 (REG_OPTION_OPEN_LINK is not implemented) | (b) | — | — |
| lift/runtime/fle_reg.c | 380 | — [open_common] | phkResult is NULL | (b) | 87 (Wine: test_reg_open_key, RegOpenKeyA with NULL phkResult: ERROR_INVALID_PARAMETER) | — |
| lift/runtime/fle_reg.c | 447 | RegOpenKeyExA [fle_imp_RegOpenKeyExA] | subkey name longer than the supported limit | (b) | — | — |
| lift/runtime/fle_reg.c | 450 | RegOpenKeyExA [fle_imp_RegOpenKeyExA] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 453 | RegOpenKeyExA [fle_imp_RegOpenKeyExA] | non-ASCII byte in an ANSI key name (code page not modelled) | (b) | — | — |
| lift/runtime/fle_reg.c | 479 | RegCloseKey [fle_imp_RegCloseKey] | hKey is neither a predefined key nor a handle issued by the virtual registry | (b) | — | — |
| lift/runtime/fle_reg.c | 513 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | lpSubKey is NULL (the page says it cannot be) | (b) | — | — |
| lift/runtime/fle_reg.c | 514 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | Reserved is not 0 | (b) | — | — |
| lift/runtime/fle_reg.c | 515 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | dwOptions is not REG_OPTION_NON_VOLATILE or REG_OPTION_VOLATILE | (b) | — | — |
| lift/runtime/fle_reg.c | 516 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | lpSecurityAttributes is not NULL (not implemented) | (b) | — | — |
| lift/runtime/fle_reg.c | 517 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | phkResult is NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 553 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page) | (b) | — | — |
| lift/runtime/fle_reg.c | 554 | RegCreateKeyExW [fle_imp_RegCreateKeyExW] | key tree deeper than 512 levels | (b) | — | — |
| lift/runtime/fle_reg.c | 592 | registry [value_set] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 637 | RegSetValueExW [fle_imp_RegSetValueExW] | Reserved is not 0 | (b) | — | — |
| lift/runtime/fle_reg.c | 638 | RegSetValueExW [fle_imp_RegSetValueExW] | value type is not implemented (REG_DWORD_BIG_ENDIAN, REG_LINK, others) | (b) | — | — |
| lift/runtime/fle_reg.c | 639 | RegSetValueExW [fle_imp_RegSetValueExW] | value larger than 1 MiB | (b) | — | — |
| lift/runtime/fle_reg.c | 640 | RegSetValueExW [fle_imp_RegSetValueExW] | lpData is NULL but cbData is not 0 | (b) | — | — |
| lift/runtime/fle_reg.c | 642 | RegSetValueExW [fle_imp_RegSetValueExW] | string data is not NUL-terminated within cbData | (b) | — | — |
| lift/runtime/fle_reg.c | 645 | RegSetValueExW [fle_imp_RegSetValueExW] | REG_MULTI_SZ data does not end in two NULs within cbData | (b) | — | — |
| lift/runtime/fle_reg.c | 647 | RegSetValueExW [fle_imp_RegSetValueExW] | REG_DWORD with cbData != 4 | (b) | — | — |
| lift/runtime/fle_reg.c | 649 | RegSetValueExW [fle_imp_RegSetValueExW] | REG_QWORD with cbData != 8 | (b) | — | — |
| lift/runtime/fle_reg.c | 664 | RegSetValueExW [fle_imp_RegSetValueExW] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 679 | RegSetValueExW [data_out] | lpcbData is NULL but lpData is not (the page allows NULL only when lpData is NULL) | (b) | — | — |
| lift/runtime/fle_reg.c | 779 | RegQueryValueExW [fle_imp_RegQueryValueExW] | lpReserved is not NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 850 | RegEnumKeyExW [fle_imp_RegEnumKeyExW] | lpReserved is not NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 851 | RegEnumKeyExW [fle_imp_RegEnumKeyExW] | lpName or lpcchName is NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 852 | RegEnumKeyExW [fle_imp_RegEnumKeyExW] | lpClass without lpcchClass | (b) | — | — |
| lift/runtime/fle_reg.c | 899 | RegEnumValueW [fle_imp_RegEnumValueW] | lpReserved is not NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 900 | RegEnumValueW [fle_imp_RegEnumValueW] | lpValueName or lpcchValueName is NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 943 | RegQueryInfoKeyW [fle_imp_RegQueryInfoKeyW] | lpReserved is not NULL | (b) | — | — |
| lift/runtime/fle_reg.c | 944 | RegQueryInfoKeyW [fle_imp_RegQueryInfoKeyW] | lpcbSecurityDescriptor is not NULL (security descriptors are not modelled) | (b) | — | — |
| lift/runtime/fle_reg.c | 1021 | RegDeleteKeyW [fle_imp_RegDeleteKeyW] | lpSubKey is NULL (the page says it cannot be) | (b) | 87 (Wine: test_reg_create_key, RegDeleteKeyA(NULL): ERROR_INVALID_PARAMETER) | — |
| lift/runtime/fle_reg.c | 1023 | RegDeleteKeyW [fle_imp_RegDeleteKeyW] | empty lpSubKey is not defined | (b) | — | — |
| lift/runtime/fle_reg.c | 1050 | RegDeleteKeyW [fle_imp_RegDeleteKeyW] | deleting a key that still has open handles is not implemented | (b) | — | — |
| lift/runtime/fle_reg.c | 1081 | registry [units_of] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 1148 | registry [restore_records] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 1174 | registry [fle_reg_restore_saved] | out of memory | (a) | — | — |
| lift/runtime/fle_reg.c | 1175 | registry [fle_reg_restore_saved] | the saved registry changed between the two reads | (b) | — | — |
| lift/runtime/fle_res.c | 59 | — [check_module] | hInstance is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_res.c | 82 | — [img_open] | (組み立ての文言) no PE image is mapped at %#llx (no MZ signature) | (b) | — | — |
| lift/runtime/fle_res.c | 85 | — [img_open] | e_lfanew of the mapped image is out of range | (b) | — | — |
| lift/runtime/fle_res.c | 87 | — [img_open] | no PE signature in the mapped image | (b) | — | — |
| lift/runtime/fle_res.c | 88 | — [img_open] | the mapped image is not PE32+ | (b) | — | — |
| lift/runtime/fle_res.c | 89 | — [img_open] | ImageBase of the mapped image is not the address it is mapped at | (b) | — | — |
| lift/runtime/fle_res.c | 100 | — [img_open] | resource directory lies outside the image | (b) | — | — |
| lift/runtime/fle_res.c | 107 | — [d32] | malformed resource directory (offset outside the directory) | (b) | — | — |
| lift/runtime/fle_res.c | 113 | — [d16] | malformed resource directory (offset outside the directory) | (b) | — | — |
| lift/runtime/fle_res.c | 133 | — [key_from_guest] | \"#\" without digits as a resource name or type is not defined | (b) | — | — |
| lift/runtime/fle_res.c | 135 | — [key_from_guest] | resource name or type \"#...\" is not a decimal number (not defined) | (b) | — | — |
| lift/runtime/fle_res.c | 137 | — [key_from_guest] | resource name or type \"#<number>\" above 65535 is not defined | (b) | — | — |
| lift/runtime/fle_res.c | 163 | — [table_find] | malformed resource directory (ID entry with the string flag) | (b) | — | — |
| lift/runtime/fle_res.c | 172 | — [table_find] | malformed resource directory (name entry without the string flag) | (b) | — | — |
| lift/runtime/fle_res.c | 189 | — [find_leaf] | malformed resource directory (data entry at the type level) | (b) | — | — |
| lift/runtime/fle_res.c | 191 | — [find_leaf] | malformed resource directory (data entry at the name level) | (b) | — | — |
| lift/runtime/fle_res.c | 195 | — [find_leaf] | several language leaves for one (type, name): the language choice is not defined | (b) | — | — |
| lift/runtime/fle_res.c | 197 | — [find_leaf] | malformed resource directory (subdirectory at the language level) | (b) | — | — |
| lift/runtime/fle_res.c | 206 | — [leaf_data] | resource data lies outside the image | (b) | — | — |
| lift/runtime/fle_res.c | 224 | — [walk] | malformed resource directory (data entry at the type level) | (b) | — | — |
| lift/runtime/fle_res.c | 229 | — [walk] | malformed resource directory (data entry at the name level) | (b) | — | — |
| lift/runtime/fle_res.c | 234 | — [walk] | malformed resource directory (subdirectory at the language level) | (b) | — | — |
| lift/runtime/fle_res.c | 283 | LoadResource [fle_imp_LoadResource] | hResInfo is NULL (the page defines only a handle returned by FindResource) | (b) | — | — |
| lift/runtime/fle_res.c | 285 | LoadResource [fle_imp_LoadResource] | hResInfo is not a resource handle returned by FindResourceW for this module | (b) | — | — |
| lift/runtime/fle_res.c | 303 | LockResource [fle_imp_LockResource] | hResData is NULL (the page allows only a successful LoadResource result) | (b) | — | — |
| lift/runtime/fle_res.c | 311 | LockResource [fle_imp_LockResource] | hResData is not a LoadResource result for the lifted DLL | (b) | — | — |
| lift/runtime/fle_res.c | 325 | SizeofResource [fle_imp_SizeofResource] | hResInfo is NULL (the page defines only a handle returned by FindResource) | (b) | — | — |
| lift/runtime/fle_res.c | 327 | SizeofResource [fle_imp_SizeofResource] | hResInfo is not a resource handle returned by FindResourceW for this module | (b) | — | — |
| lift/runtime/fle_res.c | 365 | LoadStringW [fle_imp_LoadStringW] | cchBufferMax is negative (not defined) | (b) | — | — |
| lift/runtime/fle_res.c | 366 | LoadStringW [fle_imp_LoadStringW] | uID above 0xFFFF (STRINGTABLE IDs are 16-bit; not defined) | (b) | — | — |
| lift/runtime/fle_res.c | 367 | LoadStringW [fle_imp_LoadStringW] | lpBuffer is NULL (it receives the string or a pointer, not defined as optional) | (b) | — | — |
| lift/runtime/fle_res.c | 386 | LoadStringW [fle_imp_LoadStringW] | malformed string block (shorter than its 16 strings) | (b) | — | — |
| lift/runtime/fle_res.c | 388 | LoadStringW [fle_imp_LoadStringW] | malformed string block (a length runs past the block) | (b) | — | — |
| lift/runtime/fle_res.c | 427 | LoadStringW [cursor_group_ok_key] | RT_GROUP_CURSOR is shorter than its NEWHEADER | (b) | — | — |
| lift/runtime/fle_res.c | 428 | LoadStringW [cursor_group_ok_key] | RT_GROUP_CURSOR NEWHEADER is not (Reserved 0, ResType RES_CURSOR) | (b) | — | — |
| lift/runtime/fle_res.c | 430 | LoadStringW [cursor_group_ok_key] | RT_GROUP_CURSOR ResCount does not fit the resource | (b) | — | — |
| lift/runtime/fle_res.c | 491 | LoadStringW [fle_icon_group_pick] | RT_GROUP_ICON is shorter than its NEWHEADER | (b) | — | — |
| lift/runtime/fle_res.c | 492 | LoadStringW [fle_icon_group_pick] | RT_GROUP_ICON NEWHEADER is not (Reserved 0, ResType RES_ICON) | (b) | — | — |
| lift/runtime/fle_res.c | 494 | LoadStringW [fle_icon_group_pick] | RT_GROUP_ICON ResCount does not fit the resource | (b) | — | — |
| lift/runtime/fle_res.c | 502 | LoadStringW [fle_icon_group_pick] | an RT_GROUP_ICON entry has Planes other than 1: the pages do not say how a depth is made of it | (b) | — | — |
| lift/runtime/fle_res.c | 511 | LoadStringW [fle_icon_group_pick] | every entry of the RT_GROUP_ICON exceeds the requested size: the page gives no rule for it | (b) | — | — |
| lift/runtime/fle_res.c | 535 | LoadStringW [fle_icon_group_pick] | two entries of the RT_GROUP_ICON have the same size and the same depth: the page does not say which one is chosen | (b) | — | — |
| lift/runtime/fle_res.c | 540 | LoadStringW [fle_icon_group_pick] | the RT_ICON that the chosen RT_GROUP_ICON entry names does not exist | (b) | — | — |
| lift/runtime/fle_res.c | 542 | LoadStringW [fle_icon_group_pick] | the chosen RT_ICON is PNG-compressed: not read | (b) | — | — |
| lift/runtime/fle_res.c | 545 | LoadStringW [fle_icon_group_pick] | the BITMAPINFOHEADER of the chosen RT_ICON does not agree with the RT_GROUP_ICON entry | (b) | — | — |
| lift/runtime/fle_res.c | 575 | LoadStringW [fle_cursor_load_named] | hInstance is neither the lifted DLL nor a real module: not answered (decision 0125) | (b) | — | — |
| lift/runtime/fle_res.c | 581 | LoadStringW [fle_cursor_load_named] | lpCursorName is an ordinal other than 32762..32767: not answered (decision 0125; the guest loads no other ordinal) | (b) | — | — |
| lift/runtime/fle_res.c | 598 | LoadStringW [fle_cursor_load_named] | malformed resource directory (data entry at the type level) | (b) | — | — |
| lift/runtime/fle_res.c | 604 | LoadStringW [fle_cursor_load_named] | malformed resource directory (name entry without the string flag) | (b) | — | — |
| lift/runtime/fle_res.c | 612 | LoadStringW [fle_cursor_load_named] | more than 256 named RT_GROUP_CURSOR entries: the handle form has room for 256 (decision 0125) | (b) | — | — |
| lift/runtime/fle_res.c | 633 | LoadStringW [fle_res_version_block] | malformed resource directory (data entry at the type level) | (b) | — | — |
| lift/runtime/fle_res.c | 637 | LoadStringW [fle_res_version_block] | several RT_VERSION names: the choice is not defined | (b) | — | — |
| lift/runtime/fle_res.c | 639 | LoadStringW [fle_res_version_block] | malformed resource directory (data entry at the name level) | (b) | — | — |
| lift/runtime/fle_res.c | 643 | LoadStringW [fle_res_version_block] | several language leaves for the RT_VERSION: the language choice is not defined | (b) | — | — |
| lift/runtime/fle_res.c | 645 | LoadStringW [fle_res_version_block] | malformed resource directory (subdirectory at the language level) | (b) | — | — |
| lift/runtime/fle_seh.c | 171 | — [stop_at] | (組み立ての文言) %s (ControlPc %#llx) | (b) | — | — |
| lift/runtime/fle_seh.c | 362 | RtlUnwindEx [fle_seh_restore] | a landing was entered with no pending unwind (internal) | (a) | — | — |
| lift/runtime/fle_seh.c | 398 | RaiseException [fle_seh_raise] | (組み立ての文言) exception code %#x raised while another exception is being dispatched on this thread (collided and nested exceptions are not modelled, decis… | (b) | — | — |
| lift/runtime/fle_seh.c | 402 | RaiseException [fle_seh_raise] | (組み立ての文言) exception code %#x: %u arguments (the page: the value must not exceed EXCEPTION_MAXIMUM_PARAMETERS = 15) | (b) | — | — |
| lift/runtime/fle_seh.c | 435 | RaiseException [fle_seh_raise] | more than 100000 frames searched (a loop in the virtual unwind) | (a) | — | — |
| lift/runtime/fle_seh.c | 450 | RaiseException [fle_seh_raise] | (組み立ての文言) the handler of the frame at %#llx returned ExceptionContinueExecution (resuming after a handled exception is not modelled; decision 0132) | (b) | — | — |
| lift/runtime/fle_seh.c | 453 | RaiseException [fle_seh_raise] | (組み立ての文言) the handler of the frame at %#llx returned the disposition %d (nested and collided exceptions are not modelled) | (b) | — | — |
| lift/runtime/fle_seh.c | 467 | RaiseException [fle_seh_raise] | (組み立ての文言) exception code %#x, flags %#x, %u argument(s) raised; no frame-based handler took it (%u frames searched) | (b) | — | 指紋 5(RaiseException。3 記録・1 セッション、版 2578ea1e23f6)。戻り番地 0x411e4d の発生元は推定 |
| lift/runtime/fle_seh.c | 483 | RtlUnwindEx [fle_seh_unwind_ex] | called with no exception being dispatched on this thread (decision 0132 models RtlUnwindEx only as the continuation of RaiseException) | (b) | — | — |
| lift/runtime/fle_seh.c | 484 | RtlUnwindEx [fle_seh_unwind_ex] | the exception being dispatched belongs to another guest thread (the unwind would cross a thread switch) | (b) | — | — |
| lift/runtime/fle_seh.c | 485 | RtlUnwindEx [fle_seh_unwind_ex] | TargetFrame is NULL (an exit unwind is not modelled) | (b) | — | — |
| lift/runtime/fle_seh.c | 496 | RtlUnwindEx [fle_seh_unwind_ex] | more than 100000 frames unwound (a loop in the virtual unwind) | (a) | — | — |
| lift/runtime/fle_seh.c | 501 | RtlUnwindEx [fle_seh_unwind_ex] | (組み立ての文言) the unwind passed TargetFrame %#llx without meeting a frame with that EstablisherFrame (STATUS_INVALID_UNWIND_TARGET) | (b) | — | — |
| lift/runtime/fle_seh.c | 516, 537 | RtlUnwindEx [fle_seh_unwind_ex] | (組み立ての文言) the handler of the frame at %#llx returned the disposition %d during the unwind (collided unwinds are not modelled) | (b) | — | — |
| lift/runtime/fle_seh.c | 539 | RtlUnwindEx [fle_seh_unwind_ex] | the registration of the target frame belongs to another guest thread (the jump would cross a thread switch) | (b) | — | — |
| lift/runtime/fle_seh.c | 552 | RtlUnwindEx [fle_seh_unwind_ex] | the unwind reached the end of the stack without meeting TargetFrame | (b) | — | — |
| lift/runtime/fle_shell.c | 22 | PathCleanupSpec [fle_imp_PathCleanupSpec] | pszSpec is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_shell.c | 27 | PathCleanupSpec [fle_imp_PathCleanupSpec] | pszSpec holds an illegal character: the page lists the illegal characters but does not say which of them are replaced, which are removed, or what replaces them (decision 0191) | (b) | — | — |
| lift/runtime/fle_shell.c | 29 | PathCleanupSpec [fle_imp_PathCleanupSpec] | pszSpec is longer than MAX_PATH: the truncation rule is not on the page | (b) | — | — |
| lift/runtime/fle_shell.c | 35 | PathCleanupSpec [fle_imp_PathCleanupSpec] | pszDir is longer than MAX_PATH | (b) | — | — |
| lift/runtime/fle_shell.c | 38 | PathCleanupSpec [fle_imp_PathCleanupSpec] | the path would reach MAX_PATH: the truncation rule is not on the page | (b) | — | — |
| lift/runtime/fle_spi.c | 87 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWHEELSCROLLLINES with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 88 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWHEELSCROLLLINES with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 89 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWHEELSCROLLLINES with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 97 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWORKAREA with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 98 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWORKAREA with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 99 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETWORKAREA with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 112 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGCONTRAST with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 113 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGCONTRAST with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 114 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGCONTRAST with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 122 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETCURSORSHADOW with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 123 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETCURSORSHADOW with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 124 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETCURSORSHADOW with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 134 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGTYPE with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 135 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGTYPE with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 136 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGTYPE with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 145 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGORIENTATION with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 146 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGORIENTATION with uiParam not zero: the page says to specify zero, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 147 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETFONTSMOOTHINGORIENTATION with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 157 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETANIMATION with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 158 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETANIMATION with uiParam not sizeof(ANIMATIONINFO) = 8: the page says to set it, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 159 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETANIMATION with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 160 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETANIMATION with cbSize not 8: the page says the caller sets it to sizeof(ANIMATIONINFO) | (b) | — | — |
| lift/runtime/fle_spi.c | 170 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_SETANIMATION with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 171 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_SETANIMATION with uiParam not sizeof(ANIMATIONINFO) = 8: the page says to set it, the result is not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 172 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_SETANIMATION with cbSize not 8: the page says the caller sets it to sizeof(ANIMATIONINFO) | (b) | — | — |
| lift/runtime/fle_spi.c | 180 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETICONTITLELOGFONT with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 181 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETICONTITLELOGFONT with uiParam not sizeof(LOGFONTW) = 0x5c: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 182 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETICONTITLELOGFONT with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 194 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETNONCLIENTMETRICS with pvParam NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_spi.c | 195 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETNONCLIENTMETRICS with fWinIni not zero: the page gives fWinIni for a set action only | (b) | — | — |
| lift/runtime/fle_spi.c | 198 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETNONCLIENTMETRICS with cbSize not 0x1f8 or 0x1f4: the page says cbSize is sizeof(NONCLIENTMETRICS) and defines no other | (b) | — | — |
| lift/runtime/fle_spi.c | 200 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | SPI_GETNONCLIENTMETRICS with uiParam neither 0 (the guest's form) nor cbSize (the page's): not defined | (b) | — | — |
| lift/runtime/fle_spi.c | 219 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | (組み立ての文言) uiAction 0x%x (%s) is used by the guest but no value is decided (decision 0058) | (b) | — | — |
| lift/runtime/fle_spi.c | 222 | SystemParametersInfoW [fle_imp_SystemParametersInfoW] | (組み立ての文言) uiAction 0x%x is not answered (the guest's call sites do not use it) | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 46 | GetKeyboardLayoutNameW [fle_imp_GetKeyboardLayoutNameW] | pwszKLID is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 59 | GetKeyboardLayoutNameA [fle_imp_GetKeyboardLayoutNameA] | pwszKLID is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 74 | GetKeyboardLayoutList [fle_imp_GetKeyboardLayoutList] | nBuff is negative: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 79 | GetKeyboardLayoutList [fle_imp_GetKeyboardLayoutList] | lpList is NULL with nBuff > 0: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 90 | — [load_layout] | pwszKLID is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 91 | — [load_layout] | Flags has a bit that the page does not list (KLF_UNLOADPREVIOUS is 'unsupported', and KLF_RESET / KLF_SHIFTLOCK belong to ActivateKeyboardLayout) | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 125 | ActivateKeyboardLayout [fle_imp_ActivateKeyboardLayout] | Flags has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 213 | — [check_hkl] | dwhkl is not the input locale identifier of this environment (the only one loaded, GetKeyboardLayout): the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 225 | — [to_unicode] | pwszBuff is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sweep1_kbd.c | 263 | — [to_ascii] | lpChar is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 24 | — [fle_sysinfo_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 33 | GetNativeSystemInfo [fle_imp_GetNativeSystemInfo] | lpSystemInfo is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 110 | IsProcessorFeaturePresent [fle_imp_IsProcessorFeaturePresent] | (組み立ての文言) feature %u: the page gives no value and no decision of this runtime covers it | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 150 | GetLogicalProcessorInformation [fle_imp_GetLogicalProcessorInformation] | ReturnedLength is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 158 | GetLogicalProcessorInformation [fle_imp_GetLogicalProcessorInformation] | Buffer is NULL but the length is enough (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 184 | IsWow64Process [fle_imp_IsWow64Process] | a handle that GetCurrentProcess did not return | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 185 | IsWow64Process [fle_imp_IsWow64Process] | Wow64Process is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 202 | GlobalMemoryStatus [fle_imp_GlobalMemoryStatus] | lpBuffer is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 223 | GetProcessAffinityMask [fle_imp_GetProcessAffinityMask] | a handle that GetCurrentProcess did not return | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 224 | GetProcessAffinityMask [fle_imp_GetProcessAffinityMask] | an output pointer is NULL (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 235 | SetProcessAffinityMask [fle_imp_SetProcessAffinityMask] | a handle that GetCurrentProcess did not return | (b) | — | — |
| lift/runtime/fle_sysinfo.c | 236 | SetProcessAffinityMask [fle_imp_SetProcessAffinityMask] | an empty mask (the page does not define it) | (b) | — | — |
| lift/runtime/fle_sysmetrics.c | 88 | GetSystemMetrics [fle_sm_answer] | (組み立ての文言) index %u (%s) is used by the guest but no value is decided (decision 0057) | (b) | — | — |
| lift/runtime/fle_sysmetrics.c | 91 | GetSystemMetrics [fle_sm_answer] | (組み立ての文言) index %u is not answered (the guest's call sites do not use it) | (b) | — | — |
| lift/runtime/fle_thread.c | 318 | — [obj_signaled] | a thread handle value that CreateThread never returned (the handle of the first thread is never issued) | (b) | — | — |
| lift/runtime/fle_thread.c | 636 | — [stop_report] | (組み立ての文言) %s | (b) | — | — |
| lift/runtime/fle_thread.c | 761 | — [fle_thread_sleep] | INFINITE would never return (no other thread exists) | (b) | — | — |
| lift/runtime/fle_thread.c | 981 | CloseHandle [thread_close_hook] | thread handle value was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_thread.c | 1024 | scheduler [thread_finish] | a finished thread was resumed | (b) | — | — |
| lift/runtime/fle_thread.c | 1032 | scheduler [fle_thread_wasm_entry_cur] | a JSPI stack was started for a thread that is not the current one | (b) | — | — |
| lift/runtime/fle_thread.c | 1053 | — [create_thread_core] | the thread handle region is exhausted | (a) | — | — |
| lift/runtime/fle_thread.c | 1158 | CreateThread [fle_imp_CreateThread] | lpThreadAttributes is not NULL: security descriptors and inheritance are not modelled (decision 0122) | (b) | — | — |
| lift/runtime/fle_thread.c | 1163 | CreateThread [fle_imp_CreateThread] | (組み立ての文言) dwCreationFlags %#x has a bit that the page does not list (CREATE_SUSPENDED 0x4, STACK_SIZE_PARAM_IS_A_RESERVATION 0x10000) | (b) | — | — |
| lift/runtime/fle_thread.c | 1188 | CreateThread [thread_of] | handle was not issued as a thread by this runtime (other handle kinds, INVALID_HANDLE_VALUE and NULL are not defined here) | (b) | — | — |
| lift/runtime/fle_thread.c | 1191 | CreateThread [thread_of] | thread handle value was never returned by CreateThread | (b) | — | — |
| lift/runtime/fle_thread.c | 1234 | ResumeThread [fle_imp_ResumeThread] | ResumeThread of a thread that has terminated: the page does not say what it returns | (b) | — | — |
| lift/runtime/fle_thread.c | 1251 | SuspendThread [fle_imp_SuspendThread] | SuspendThread of a thread that has terminated: the page does not say what it returns | (b) | — | — |
| lift/runtime/fle_thread.c | 1252 | SuspendThread [fle_imp_SuspendThread] | the suspend count would pass MAXIMUM_SUSPEND_COUNT: the page gives no error code | (b) | — | — |
| lift/runtime/fle_thread.c | 1257 | SuspendThread [fle_imp_SuspendThread] | the only thread suspends itself: nothing can resume it | (b) | — | — |
| lift/runtime/fle_thread.c | 1268 | ExitThread [fle_imp_ExitThread] | the first thread (the one that entered the guest) called ExitThread: the process would end (not modelled) | (b) | — | — |
| lift/runtime/fle_thread.c | 1278 | GetExitCodeThread [fle_imp_GetExitCodeThread] | lpExitCode is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_thread.c | 1302 | GetExitCodeThread [fle_thread_wait_objects] | the handle array holds the same handle twice: the page says it may not (behaviour not given) | (b) | — | — |
| lift/runtime/fle_thread.c | 1322 | GetExitCodeThread [fle_thread_wait_objects] | (動的な文言 ms == INFINITE_ ? "INFINITE time-out on objects that are not signaled would never return (no other thread exists)" : "a finite time-out on objects that are not signaled: no other thread exists, so only the virtual clock could end it (not modelled for one thread)") | (b) | — | — |
| lift/runtime/fle_thread.c | 1348 | WaitForMultipleObjects [fle_imp_WaitForMultipleObjects] | lpHandles is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_thread.c | 1371 | WaitForMultipleObjectsEx [fle_imp_WaitForMultipleObjectsEx] | lpHandles is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_thread.c | 1387 | MsgWaitForMultipleObjects [fle_imp_MsgWaitForMultipleObjects] | fWaitAll TRUE (all the objects signaled and an input event): not modelled | (b) | — | — |
| lift/runtime/fle_thread.c | 1388 | MsgWaitForMultipleObjects [fle_imp_MsgWaitForMultipleObjects] | pHandles is NULL with nCount > 0: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_thread.c | 1454 | PostMessageW [fle_thread_post] | out of memory | (a) | — | — |
| lift/runtime/fle_thread_wasm.c | 52 | scheduler [check_region] | (組み立ての文言) the canary at the low end of the shadow stack of thread %u was overwritten (%s): the C frames of the thread are deeper than %u bytes | (b) | — | — |
| lift/runtime/fle_thread_wasm.c | 56 | scheduler [check_region] | (組み立ての文言) the shadow stack pointer of thread %u (%# | (b) | — | — |
| lift/runtime/fle_thread_wasm.c | 115 | scheduler [fle_thread_wasm_transfer] | a switch to a thread that has no shadow stack region | (b) | — | — |
| lift/runtime/fle_thread_wasm.c | 125 | scheduler [fle_thread_wasm_transfer] | (組み立ての文言) thread %u resumed with __stack_pointer %# | (b) | — | — |
| lift/runtime/fle_time.c | 149 | — [fle_need_ptr] | (動的な文言 what) | (b) | — | — |
| lift/runtime/fle_time.c | 169 | GetSystemTime [fle_imp_GetSystemTime] | emulated time is outside the SYSTEMTIME range | (b) | — | — |
| lift/runtime/fle_time.c | 182 | GetLocalTime [fle_imp_GetLocalTime] | emulated local time is outside the SYSTEMTIME range | (b) | — | — |
| lift/runtime/fle_time.c | 234 | FileTimeToSystemTime [fle_imp_FileTimeToSystemTime] | result year above 30827, the documented wYear maximum (the page defines no result) | (b) | — | — |
| lift/runtime/fle_time.c | 266 | — [fle_ft_zone] | output is the same as the input (the page forbids it) | (b) | — | — |
| lift/runtime/fle_time.c | 297 | SystemTimeToTzSpecificLocalTime [fle_imp_SystemTimeToTzSpecificLocalTime] | a time zone with daylight-saving transition dates is not implemented | (b) | — | — |
| lift/runtime/fle_time.c | 408 | GetDateFormatW [fle_date_picture] | day-name pictures (ddd, dddd) need the locale's day names, which the pages read do not give | (b) | — | — |
| lift/runtime/fle_time.c | 413 | GetDateFormatW [fle_date_picture] | month-name pictures (MMM, MMMM) need the locale's month names, which the pages read do not give | (b) | — | — |
| lift/runtime/fle_time.c | 418 | GetDateFormatW [fle_date_picture] | y and yyy: the two pages read describe them differently (last digit / treated as yy, yyyy) | (b) | — | — |
| lift/runtime/fle_time.c | 419 | GetDateFormatW [fle_date_picture] | more than five y in a picture is not defined by the pages | (b) | — | — |
| lift/runtime/fle_time.c | 422 | GetDateFormatW [fle_date_picture] | a year above 9999 with yyyy is not defined for the Gregorian calendar by the pages | (b) | — | — |
| lift/runtime/fle_time.c | 427 | GetDateFormatW [fle_date_picture] | era pictures (g, gg) need the era string (CAL_SERASTRING), which the pages read do not give | (b) | — | — |
| lift/runtime/fle_time.c | 429 | GetDateFormatW [fle_date_picture] | an unquoted letter that is not a picture: the page leaves the result to 'the best possible string' | (b) | — | — |
| lift/runtime/fle_time.c | 434 | GetDateFormatW [fle_date_picture] | formatted date longer than the supported limit | (b) | — | — |
| lift/runtime/fle_time.c | 451 | GetDateFormatW [fle_imp_GetDateFormatW] | a locale other than the emulated en-US (decision 0016) | (b) | — | — |
| lift/runtime/fle_time.c | 452 | GetDateFormatW [fle_imp_GetDateFormatW] | lpFormat is NULL: the locale's date picture is not given by the pages read | (b) | — | — |
| lift/runtime/fle_time.c | 453 | GetDateFormatW [fle_imp_GetDateFormatW] | flags with a format picture: the page says the flags apply when lpFormat is NULL | (b) | — | — |
| lift/runtime/fle_time.c | 465 | GetDateFormatW [fle_imp_GetDateFormatW] | emulated local time is outside the SYSTEMTIME range | (b) | — | — |
| lift/runtime/fle_timerq.c | 73 | CreateTimerQueueTimer [start_thread] | no thread could be made for the timer (the thread table is full) | (b) | — | — |
| lift/runtime/fle_timerq.c | 86 | CreateTimerQueueTimer [fle_imp_CreateTimerQueueTimer] | TimerQueue is not NULL: only the default timer queue exists (no CreateTimerQueue is imported) | (b) | — | — |
| lift/runtime/fle_timerq.c | 87 | CreateTimerQueueTimer [fle_imp_CreateTimerQueueTimer] | phNewTimer is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_timerq.c | 88 | CreateTimerQueueTimer [fle_imp_CreateTimerQueueTimer] | Callback is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_timerq.c | 89 | CreateTimerQueueTimer [fle_imp_CreateTimerQueueTimer] | Flags has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_timerq.c | 93 | CreateTimerQueueTimer [fle_imp_CreateTimerQueueTimer] | more than 64 timer-queue timers are alive (a limit of this runtime) | (b) | — | — |
| lift/runtime/fle_timerq.c | 116 | ChangeTimerQueueTimer [fle_imp_ChangeTimerQueueTimer] | TimerQueue is not NULL: only the default timer queue exists | (b) | — | — |
| lift/runtime/fle_timerq.c | 118 | ChangeTimerQueueTimer [fle_imp_ChangeTimerQueueTimer] | Timer is not a handle that CreateTimerQueueTimer returned (or it was deleted): the page gives no failure code | (b) | — | — |
| lift/runtime/fle_timerq.c | 141 | DeleteTimerQueueTimer [fle_imp_DeleteTimerQueueTimer] | TimerQueue is not NULL: only the default timer queue exists | (b) | — | — |
| lift/runtime/fle_timerq.c | 143 | DeleteTimerQueueTimer [fle_imp_DeleteTimerQueueTimer] | Timer is not a handle that CreateTimerQueueTimer returned (or it was already deleted): the page gives no failure code | (b) | — | — |
| lift/runtime/fle_timerq.c | 145 | DeleteTimerQueueTimer [fle_imp_DeleteTimerQueueTimer] | CompletionEvent is not NULL while the callback is running: waiting for a callback is not modelled | (b) | — | — |
| lift/runtime/fle_timerq.c | 152 | DeleteTimerQueueTimer [fle_imp_DeleteTimerQueueTimer] | CompletionEvent is an event handle: signaling it is not modelled | (b) | — | — |
| lift/runtime/fle_token.c | 60 | OpenProcessToken [fle_imp_OpenProcessToken] | ProcessHandle is not the pseudo handle of the current process: no other process exists (decision 0136) | (b) | — | — |
| lift/runtime/fle_token.c | 61 | OpenProcessToken [fle_imp_OpenProcessToken] | DesiredAccess is not exactly TOKEN_QUERY (0x8): the page gives no behaviour for another access mask (decision 0136) | (b) | — | — |
| lift/runtime/fle_token.c | 62 | OpenProcessToken [fle_imp_OpenProcessToken] | TokenHandle is NULL: the page gives no behaviour (decision 0136) | (b) | — | — |
| lift/runtime/fle_token.c | 63 | OpenProcessToken [fle_imp_OpenProcessToken] | more tokens than the runtime holds | (b) | — | — |
| lift/runtime/fle_token.c | 81 | GetTokenInformation [fle_imp_GetTokenInformation] | TokenHandle is not a live token of this runtime: the page names no error code (decision 0136) | (b) | — | — |
| lift/runtime/fle_token.c | 82 | GetTokenInformation [fle_imp_GetTokenInformation] | TokenInformationClass is not TokenElevation (20): only the guest's elevation check is answered (decision 0136) | (b) | — | — |
| lift/runtime/fle_token.c | 83 | GetTokenInformation [fle_imp_GetTokenInformation] | ReturnLength is NULL: the page does not mark it optional and gives no behaviour (decision 0136) | (b) | — | — |
| lift/runtime/fle_uipref.c | 87 | GetThreadPreferredUILanguages [fle_uipref_add] | language list longer than the internal buffer | (a) | — | — |
| lift/runtime/fle_uipref.c | 105 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | dwFlags bit not handled (MUI_THREAD_LANGUAGES or an unknown bit) | (b) | — | — |
| lift/runtime/fle_uipref.c | 107 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | MUI_LANGUAGE_ID with MUI_LANGUAGE_NAME: the page says exclusive, not what fails | (b) | — | — |
| lift/runtime/fle_uipref.c | 112 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | MUI_MERGE_SYSTEM_FALLBACK without MUI_MERGE_USER_FALLBACK: the page does not say what fails | (b) | — | — |
| lift/runtime/fle_uipref.c | 114 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | NULL pulNumLanguages or pcchLanguagesBuffer: the page does not define it | (b) | — | — |
| lift/runtime/fle_uipref.c | 122 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | no language name for the emulated locale | (b) | — | — |
| lift/runtime/fle_uipref.c | 126 | GetThreadPreferredUILanguages [fle_imp_GetThreadPreferredUILanguages] | no neutral language name for the emulated locale | (b) | — | — |
| lift/runtime/fle_uipref.c | 171 | GetUserPreferredUILanguages [fle_imp_GetUserPreferredUILanguages] | dwFlags holds a bit other than MUI_LANGUAGE_ID and MUI_LANGUAGE_NAME: the page lists no other flag (decision 0117) | (b) | — | — |
| lift/runtime/fle_uipref.c | 172 | GetUserPreferredUILanguages [fle_imp_GetUserPreferredUILanguages] | MUI_LANGUAGE_ID with MUI_LANGUAGE_NAME: the page says exclusive, not what fails (decision 0117) | (b) | — | — |
| lift/runtime/fle_uipref.c | 173 | GetUserPreferredUILanguages [fle_imp_GetUserPreferredUILanguages] | NULL pulNumLanguages or pcchLanguagesBuffer: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_uipref.c | 178 | GetUserPreferredUILanguages [fle_imp_GetUserPreferredUILanguages] | no language name for the emulated locale (decision 0117) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 29 | — [fle_unw_need_ascii] | UTF-16 unit of 0x80 or above: the page gives no case table for it | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 76 | — [fle_unw_buf] | lpsz is NULL with cchLength > 0 (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 97 | CharNextW [fle_imp_CharNextW] | lpsz is NULL (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 100 | CharNextW [fle_imp_CharNextW] | character of 0x80 or above: grapheme rules (combining marks, surrogates) are not given by the page | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 102 | CharNextW [fle_imp_CharNextW] | next unit of 0x80 or above could be a combining mark: not decidable from the page | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 103 | CharNextW [fle_imp_CharNextW] | CR LF: the page does not say whether it is one character | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 114 | — [fle_unw_need] | (動的な文言 what) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 223 | OffsetRect [fle_imp_OffsetRect] | a coordinate leaves the 32-bit range (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 239 | InflateRect [fle_imp_InflateRect] | a coordinate leaves the 32-bit range (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 326 | — [fle_unw_read_name] | name is NULL (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 328 | — [fle_unw_read_name] | empty name (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 330 | — [fle_unw_read_name] | name has a unit of 0x80 or above: its case folding is not given by the pages | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 332 | — [fle_unw_read_name] | out of memory | (a) | — | — |
| lift/runtime/fle_user_nowin.c | 350 | — [fle_unw_register] | a name equal to a registered one except for case: the page does not say if it is the same message | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 354 | — [fle_unw_register] | all identifiers 0xC000 through 0xFFFF are in use (the page names no error code) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 357 | — [fle_unw_register] | out of memory | (a) | — | — |
| lift/runtime/fle_user_nowin.c | 386 | GetClipboardFormatNameW [fle_imp_GetClipboardFormatNameW] | lpszFormatName is NULL (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 387 | GetClipboardFormatNameW [fle_imp_GetClipboardFormatNameW] | cchMaxCount is not positive (not described by the page) | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 392 | GetClipboardFormatNameW [fle_imp_GetClipboardFormatNameW] | the value is a registered window message: the page does not say it is visible as a clipboard format | (b) | — | — |
| lift/runtime/fle_user_nowin.c | 394 | GetClipboardFormatNameW [fle_imp_GetClipboardFormatNameW] | the name does not fit: the page does not say if cchMaxCount counts the terminating NUL | (b) | — | — |
| lift/runtime/fle_vfs.c | 98 | — [lazy_fetch] | the host did not give the bytes of a lazy file of the install tree (read function missing or failed) | (b) | — | — |
| lift/runtime/fle_vfs.c | 156 | vfs [attach] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 181 | vfs [set_name] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 192 | vfs [node_new] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 212 | — [node_reserve] | file larger than the 256 MiB virtual limit | (b) | — | — |
| lift/runtime/fle_vfs.c | 219 | — [node_reserve] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 239 | — [node_materialize] | a lazy file larger than the 256 MiB virtual limit cannot be made writable | (b) | — | — |
| lift/runtime/fle_vfs.c | 242 | — [node_materialize] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 274 | vfs [node_path] | directory tree too deep | (b) | — | — |
| lift/runtime/fle_vfs.c | 280 | vfs [node_path] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 310 | — [vp_push] | path has too many components | (b) | — | — |
| lift/runtime/fle_vfs.c | 362 | — [check_component] | path component longer than 255 characters | (b) | — | — |
| lift/runtime/fle_vfs.c | 366 | — [check_component] | reserved character in a path component (streams are not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 368 | — [check_component] | wildcard in a path that does not allow one | (b) | — | — |
| lift/runtime/fle_vfs.c | 372 | — [check_component] | path component ends with a space or a period | (b) | — | — |
| lift/runtime/fle_vfs.c | 373 | — [check_component] | reserved device name in a path (devices are not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 388 | — [parse_path] | empty path is not defined | (b) | — | — |
| lift/runtime/fle_vfs.c | 390 | — [parse_path] | the \\\\?\\ and \\\\.\\ prefixes are not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 393 | — [parse_path] | only drive C: exists | (b) | — | — |
| lift/runtime/fle_vfs.c | 395 | — [parse_path] | drive-relative path (C:name) is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 397 | — [parse_path] | UNC path is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 444 | — [lookup] | a trailing separator on a file name is not defined | (b) | — | — |
| lift/runtime/fle_vfs.c | 517 | — [file_arg] | handle was not issued by the virtual file system | (b) | — | — |
| lift/runtime/fle_vfs.c | 529 | CreateFileW [fh_alloc] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 533 | CreateFileW [fh_alloc] | file handle region exhausted | (a) | — | — |
| lift/runtime/fle_vfs.c | 602 | CreateFileW [fle_imp_CreateFileW] | lpSecurityAttributes is not NULL and not a SECURITY_ATTRIBUTES with a NULL descriptor (not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 603 | CreateFileW [fle_imp_CreateFileW] | hTemplateFile is not NULL (not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 604 | CreateFileW [fle_imp_CreateFileW] | dwShareMode has bits outside FILE_SHARE_READ/WRITE/DELETE | (b) | — | — |
| lift/runtime/fle_vfs.c | 605 | CreateFileW [fle_imp_CreateFileW] | dwCreationDisposition is not one of the five documented values | (b) | — | — |
| lift/runtime/fle_vfs.c | 608 | CreateFileW [fle_imp_CreateFileW] | dwDesiredAccess has bits that are not implemented (GENERIC_EXECUTE, MAXIMUM_ALLOWED, ...) | (b) | — | — |
| lift/runtime/fle_vfs.c | 610 | CreateFileW [fle_imp_CreateFileW] | dwFlagsAndAttributes has flags or attributes that are not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 621 | CreateFileW [fle_imp_CreateFileW] | a trailing separator in a CreateFile name is not defined | (b) | — | — |
| lift/runtime/fle_vfs.c | 636, 658 | CreateFileW [fle_imp_CreateFileW] | TRUNCATE_EXISTING without GENERIC_WRITE | (b) | — | — |
| lift/runtime/fle_vfs.c | 656 | CreateFileW [fle_imp_CreateFileW] | opening a directory handle is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 703 | ReadFile [fle_imp_ReadFile] | lpOverlapped is not NULL (asynchronous I/O is not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 704 | ReadFile [fle_imp_ReadFile] | lpNumberOfBytesRead is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 719 | ReadFile [fle_imp_ReadFile] | lpBuffer is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 759 | WriteFile [fle_imp_WriteFile] | lpOverlapped is not NULL (asynchronous I/O is not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 760 | WriteFile [fle_imp_WriteFile] | lpNumberOfBytesWritten is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 775 | WriteFile [fle_imp_WriteFile] | lpBuffer is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 777 | WriteFile [fle_imp_WriteFile] | file position overflow | (b) | — | — |
| lift/runtime/fle_vfs.c | 799 | CloseHandle [fle_vfs_add_close_hook] | too many handle hooks | (b) | — | — |
| lift/runtime/fle_vfs.c | 883 | SetFilePointer [fle_imp_SetFilePointer] | dwMoveMethod is not FILE_BEGIN, FILE_CURRENT or FILE_END | (b) | — | — |
| lift/runtime/fle_vfs.c | 896 | SetFilePointer [fle_imp_SetFilePointer] | new position 0xFFFFFFFF is ambiguous with failure | (b) | — | — |
| lift/runtime/fle_vfs.c | 924 | SetFilePointerEx [fle_imp_SetFilePointerEx] | dwMoveMethod is not FILE_BEGIN, FILE_CURRENT or FILE_END | (b) | — | — |
| lift/runtime/fle_vfs.c | 925 | SetFilePointerEx [fle_imp_SetFilePointerEx] | FILE_BEGIN with liDistanceToMove above 2^63 (the page reads it as unsigned and names no limit) | (b) | — | — |
| lift/runtime/fle_vfs.c | 991 | SetFileTime [fle_imp_SetFileTime] | FILETIME 0xFFFFFFFF/0xFFFFFFFF is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 1075 | FindFirstFileW [item_push] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 1101 | FindFirstFileW [fle_imp_FindFirstFileW] | lpFindFileData is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 1104 | FindFirstFileW [fle_imp_FindFirstFileW] | a name that resolves to no component is not defined | (b) | — | — |
| lift/runtime/fle_vfs.c | 1154 | FindFirstFileW [fle_imp_FindFirstFileW] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 1158 | FindFirstFileW [fle_imp_FindFirstFileW] | find handle region exhausted | (a) | — | — |
| lift/runtime/fle_vfs.c | 1194 | FindFirstFileExW [fle_imp_FindFirstFileExW] | fInfoLevelId is not FindExInfoStandard or FindExInfoBasic | (b) | — | — |
| lift/runtime/fle_vfs.c | 1195 | FindFirstFileExW [fle_imp_FindFirstFileExW] | fSearchOp is not FindExSearchNameMatch or FindExSearchLimitToDirectories | (b) | — | — |
| lift/runtime/fle_vfs.c | 1196 | FindFirstFileExW [fle_imp_FindFirstFileExW] | lpSearchFilter is not NULL (the page: it must be NULL) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1197 | FindFirstFileExW [fle_imp_FindFirstFileExW] | dwAdditionalFlags has a bit other than FIND_FIRST_EX_LARGE_FETCH (not modelled) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1209 | FindNextFileW [fle_imp_FindNextFileW] | handle was not issued by the virtual file system | (b) | — | — |
| lift/runtime/fle_vfs.c | 1210 | FindNextFileW [fle_imp_FindNextFileW] | lpFindFileData is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 1230 | FindClose [fle_imp_FindClose] | handle was not issued by the virtual file system | (b) | — | — |
| lift/runtime/fle_vfs.c | 1307 | GetFileAttributesExW [fle_imp_GetFileAttributesExW] | fInfoLevelId is not GetFileExInfoStandard | (b) | — | — |
| lift/runtime/fle_vfs.c | 1308 | GetFileAttributesExW [fle_imp_GetFileAttributesExW] | lpFileInformation is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 1344 | SetFileAttributesW [fle_imp_SetFileAttributesW] | dwFileAttributes has values the page does not list as supported | (b) | — | — |
| lift/runtime/fle_vfs.c | 1353 | SetFileAttributesW [fle_imp_SetFileAttributesW] | changing the attributes of the volume root is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 1370 | CreateDirectoryW [fle_imp_CreateDirectoryW] | lpSecurityAttributes is not NULL (not implemented) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1482 | CopyFileW [fle_imp_CopyFileW] | copying a directory is not implemented | (b) | — | — |
| lift/runtime/fle_vfs.c | 1483 | CopyFileW [fle_imp_CopyFileW] | destination names no file | (b) | — | — |
| lift/runtime/fle_vfs.c | 1493 | CopyFileW [fle_imp_CopyFileW] | the destination is a directory | (b) | — | — |
| lift/runtime/fle_vfs.c | 1565 | MoveFileW [fle_imp_MoveFileW] | moving the volume root | (b) | — | — |
| lift/runtime/fle_vfs.c | 1572 | MoveFileW [fle_imp_MoveFileW] | a trailing separator on a file's new name is not defined | (b) | — | — |
| lift/runtime/fle_vfs.c | 1587 | MoveFileW [fle_imp_MoveFileW] | moving a directory into itself | (b) | — | — |
| lift/runtime/fle_vfs.c | 1614 | MoveFileW [fit] | lpBuffer is NULL but nSize leaves room for the string | (b) | — | — |
| lift/runtime/fle_vfs.c | 1651 | GetFullPathNameW [fle_imp_GetFullPathNameW] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 1729 | GetTempFileNameW [fle_imp_GetTempFileNameW] | lpPathName is NULL: the page says the function fails and names no error code | (b) | — | — |
| lift/runtime/fle_vfs.c | 1730 | GetTempFileNameW [fle_imp_GetTempFileNameW] | lpPrefixString is NULL: the page does not describe it | (b) | — | — |
| lift/runtime/fle_vfs.c | 1731 | GetTempFileNameW [fle_imp_GetTempFileNameW] | lpTempFileName is NULL: the page does not describe it | (b) | — | — |
| lift/runtime/fle_vfs.c | 1732 | GetTempFileNameW [fle_imp_GetTempFileNameW] | uUnique is 0: the function would create a file named after the system time (the page's words); only a nonzero uUnique, which only forms a name, is answered | (b) | — | — |
| lift/runtime/fle_vfs.c | 1739 | GetTempFileNameW [fle_imp_GetTempFileNameW] | lpPathName is empty: the page does not describe it | (b) | — | — |
| lift/runtime/fle_vfs.c | 1743 | GetTempFileNameW [fle_imp_GetTempFileNameW] | a character of the prefix is not printable ASCII: the OEM character set is not modelled | (b) | — | — |
| lift/runtime/fle_vfs.c | 1784 | GetTempPathW [fle_imp_GetTempPathW] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 1788 | GetTempPathW [fle_imp_GetTempPathW] | temporary path longer than the page's maximum of 261 characters | (b) | — | — |
| lift/runtime/fle_vfs.c | 1815 | GetTempPathW [module_path] | only the lifted DLL has a path (hModule is not NULL or 0x400000) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1838 | GetModuleFileNameW [fle_imp_GetModuleFileNameW] | lpFilename is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 1867 | GetModuleFileNameA [fle_imp_GetModuleFileNameA] | lpFilename is NULL | (b) | — | — |
| lift/runtime/fle_vfs.c | 1940 | GetFileVersionInfoSizeW [fle_imp_GetFileVersionInfoSizeW] | lptstrFilename is NULL: the page does not define it (decision 0098) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1947 | GetFileVersionInfoSizeW [fle_imp_GetFileVersionInfoSizeW] | a file name with no directory part: the LoadLibrary search sequence is not modelled (decision 0098) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1952 | GetFileVersionInfoSizeW [fle_imp_GetFileVersionInfoSizeW] | the path is the engine's own and no resource reader is linked (fle_res.c): decision 0129 | (b) | — | — |
| lift/runtime/fle_vfs.c | 1968 | GetFileVersionInfoSizeW [fle_imp_GetFileVersionInfoSizeW] | the path exists in the virtual file system: no version resource of a virtual file is modelled (decision 0098) | (b) | — | — |
| lift/runtime/fle_vfs.c | 1994 | GetFileVersionInfoW [fle_imp_GetFileVersionInfoW] | lptstrFilename is NULL: the page does not define it (decision 0112) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2000 | GetFileVersionInfoW [fle_imp_GetFileVersionInfoW] | a file name with no directory part: the LoadLibrary search sequence is not modelled (decisions 0098, 0112) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2005 | GetFileVersionInfoW [fle_imp_GetFileVersionInfoW] | the path is the engine's own and no resource reader is linked (fle_res.c): decision 0129 | (b) | — | — |
| lift/runtime/fle_vfs.c | 2010 | GetFileVersionInfoW [fle_imp_GetFileVersionInfoW] | lpData is NULL or dwLen is 0: the page does not say what the call does (decision 0129) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2024 | GetFileVersionInfoW [fle_imp_GetFileVersionInfoW] | the path exists in the virtual file system: no version resource of a virtual file is modelled (decisions 0098, 0112) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2131 | VerQueryValueW [fle_imp_VerQueryValueW] | pBlock, lpSubBlock, lplpBuffer or puLen is NULL: the page does not define it (decision 0129) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2297 | VerQueryValueW [trust_stop] | (動的な文言 msg) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2304 | authenticode claims [trust_dup] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 2379 | VerQueryValueW [trust_new] | too many WinVerifyTrust states and crypt32 objects in one run (the table holds 64; tokens are never reused) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2416 | VerQueryValueW [trust_claim_for] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 2452 | WinVerifyTrust [fle_imp_WinVerifyTrust] | no file is vouched for: the table of authenticode claims is empty (a build without FLE_FRUITY or without FLE_AUTH_CLAIMS; decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2453 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pgActionID is NULL: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2454 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pgActionID is not WINTRUST_ACTION_GENERIC_VERIFY_V2 {00AAC56B-CD44-11D0-8CC2-00C04FC295EE}: no other action is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2455 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pWVTData is NULL: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2457 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pPolicyCallbackData is not NULL: a policy-specific request is not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2458 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pSIPClientData is not NULL: not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2459 | WinVerifyTrust [fle_imp_WinVerifyTrust] | dwUIChoice is not one of the values 1..4 of the page (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2460 | WinVerifyTrust [fle_imp_WinVerifyTrust] | fdwRevocationChecks asks for revocation checking: the claims table has no revocation result (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2461 | WinVerifyTrust [fle_imp_WinVerifyTrust] | dwUnionChoice is not WTD_CHOICE_FILE: only a file is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2462 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pwszURLReference is not NULL: the page says it is reserved (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2463 | WinVerifyTrust [fle_imp_WinVerifyTrust] | dwProvFlags is neither 0 nor WTD_REVOCATION_CHECK_NONE: other flags change what is verified and are not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2464 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pSignatureSettings is not NULL: not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2474 | WinVerifyTrust [fle_imp_WinVerifyTrust] | dwStateAction is not WTD_STATEACTION_IGNORE, VERIFY or CLOSE: the catalog actions are not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2475 | WinVerifyTrust [fle_imp_WinVerifyTrust] | dwStateAction is VERIFY and hWVTStateData is not NULL: a state is already attached (the page does not define it; decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2477 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pFile is NULL for WTD_CHOICE_FILE: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2478 | WinVerifyTrust [fle_imp_WinVerifyTrust] | WINTRUST_FILE_INFO.cbStruct is not 0x20 (the 64-bit structure; decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2479 | WinVerifyTrust [fle_imp_WinVerifyTrust] | pcwszFilePath is NULL: the page says it cannot be (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2480 | WinVerifyTrust [fle_imp_WinVerifyTrust] | WINTRUST_FILE_INFO.hFile is not NULL: a file handle is not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2481 | WinVerifyTrust [fle_imp_WinVerifyTrust] | WINTRUST_FILE_INFO.pgKnownSubject is not NULL: not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2499 | CryptQueryObject [fle_imp_CryptQueryObject] | dwObjectType is not CERT_QUERY_OBJECT_FILE: a blob is not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2500 | CryptQueryObject [fle_imp_CryptQueryObject] | pvObject is NULL for CERT_QUERY_OBJECT_FILE: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2501 | CryptQueryObject [fle_imp_CryptQueryObject] | dwExpectedContentTypeFlags is not CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED only: no other content type is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2502 | CryptQueryObject [fle_imp_CryptQueryObject] | dwExpectedFormatTypeFlags is not CERT_QUERY_FORMAT_FLAG_BINARY only: no other format is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2503 | CryptQueryObject [fle_imp_CryptQueryObject] | dwFlags is not zero: the page says it is reserved (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2504 | CryptQueryObject [fle_imp_CryptQueryObject] | ppvContext is not NULL: the context for an embedded PKCS #7 message is not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2550 | CryptMsgGetParam [fle_imp_CryptMsgGetParam] | dwParamType is not CMSG_SIGNER_INFO_PARAM: no other parameter is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2551 | CryptMsgGetParam [fle_imp_CryptMsgGetParam] | dwIndex is not 0: the claim names one signer (the page lists CRYPT_E_INVALID_INDEX, whose number it does not give; decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2552 | CryptMsgGetParam [fle_imp_CryptMsgGetParam] | pcbData is NULL: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2616 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | dwCertEncodingType is not X509_ASN_ENCODING \| PKCS_7_ASN_ENCODING (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2617 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | dwFindFlags is not zero: not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2618 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | dwFindType is not CERT_FIND_SUBJECT_CERT: no other search is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2619 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | pvFindPara is NULL for CERT_FIND_SUBJECT_CERT: the page does not define it (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2620 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | pPrevCertContext is not NULL: continuing a search is not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2628 | CertFindCertificateInStore [fle_imp_CertFindCertificateInStore] | out of memory | (a) | — | — |
| lift/runtime/fle_vfs.c | 2645 | CertGetNameStringW [fle_imp_CertGetNameStringW] | dwType is not CERT_NAME_SIMPLE_DISPLAY_TYPE: no other name type is modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2646 | CertGetNameStringW [fle_imp_CertGetNameStringW] | dwFlags is not zero: CERT_NAME_ISSUER_FLAG and the others are not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2647 | CertGetNameStringW [fle_imp_CertGetNameStringW] | pvTypePara is not NULL: the page says it is not used for CERT_NAME_SIMPLE_DISPLAY_TYPE (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2648 | CertGetNameStringW [fle_imp_CertGetNameStringW] | the claim has no common name (the first of CN, OU, O, E is not modelled; decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2653 | CertGetNameStringW [fle_imp_CertGetNameStringW] | cchNameString is smaller than the name: the page does not define a truncated result (decision 0117) | (b) | — | — |
| lift/runtime/fle_vfs.c | 2678 | CertCloseStore [fle_imp_CertCloseStore] | dwFlags is not zero: CERT_CLOSE_STORE_CHECK_FLAG and FORCE_FLAG are not modelled (decision 0117) | (b) | — | — |
| lift/runtime/fle_vmap.c | 32 | — [fle_vmap_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_vmap.c | 212 | VirtualQuery [fle_vmap_register_image] | more images than the runtime holds | (b) | — | — |
| lift/runtime/fle_vmap.c | 213 | VirtualQuery [fle_vmap_register_image] | an image registered for VirtualQuery has no MZ signature at its base | (b) | — | — |
| lift/runtime/fle_vmap.c | 215 | VirtualQuery [fle_vmap_register_image] | an image registered for VirtualQuery has no PE signature | (b) | — | — |
| lift/runtime/fle_vmap.c | 220 | VirtualQuery [fle_vmap_register_image] | an image with more sections than the runtime holds | (b) | — | — |
| lift/runtime/fle_vmap.c | 235 | VirtualQuery [fle_vmap_register_image] | a section whose virtual address is not a multiple of the page size | (b) | — | — |
| lift/runtime/fle_vmap.c | 258 | VirtualQuery [fle_vmap_register_range] | more registered ranges than the runtime holds | (b) | — | — |
| lift/runtime/fle_vmap.c | 359 | VirtualQuery [fle_imp_VirtualQuery] | dwLength is smaller than MEMORY_BASIC_INFORMATION (the page gives no behavior) | (b) | — | — |
| lift/runtime/fle_vmap.c | 360 | VirtualQuery [fle_imp_VirtualQuery] | lpBuffer is NULL | (b) | — | — |
| lift/runtime/fle_vmap.c | 415 | inline hook [fle_hook_register] | more modules with hook points than the runtime holds | (b) | — | — |
| lift/runtime/fle_vmap.c | 441 | inline hook [fle_hook_target] | (組み立ての文言) hook point %#llx: the first 5 bytes are now %s; the original %s and 'e9 rel32' are the only ones modelled (decision 0105) | (b) | — | — |
| lift/runtime/fle_vmap.c | 446 | inline hook [fle_hook_target] | (組み立ての文言) hook point %#llx: the jmp rel32 that the guest wrote (%s) goes to address 0 | (b) | — | — |
| lift/runtime/fle_vmap.c | 460 | site override [fle_site_store32] | (組み立ての文言) site %#llx: the word at %#llx is %#x; the override expects %#x (the guest's value) or %#x (already overridden) (decision 0120) | (b) | — | — |
| lift/runtime/fle_vmap.c | 499, 505 | VirtualQuery [ih_protect] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_vmap.c | 519 | VirtualQuery [ih_protect] | more pages of inline hook points than the runtime records | (b) | — | — |
| lift/runtime/fle_vmap.c | 547 | VirtualProtect [fle_imp_VirtualProtect] | flNewProtect is not one of the six protections modelled | (b) | — | — |
| lift/runtime/fle_vmap.c | 548 | VirtualProtect [fle_imp_VirtualProtect] | dwSize 0 is not defined by the page | (b) | — | — |
| lift/runtime/fle_vmap.c | 549 | VirtualProtect [fle_imp_VirtualProtect] | range overflows the address space (not defined by the page) | (b) | — | — |
| lift/runtime/fle_vmap.c | 561 | VirtualProtect [fle_imp_VirtualProtect] | changing the protection of a file view is not implemented | (b) | — | — |
| lift/runtime/fle_vmap.c | 562 | VirtualProtect [fle_imp_VirtualProtect] | address is not in a VirtualAlloc region (the runtime does not track other memory) | (b) | — | — |
| lift/runtime/fle_vmap.c | 625 | CreateFileMappingW [hand_new] | out of memory | (a) | — | — |
| lift/runtime/fle_vmap.c | 629 | CreateFileMappingW [hand_new] | file mapping handle region exhausted | (a) | — | — |
| lift/runtime/fle_vmap.c | 666 | CloseHandle [vmap_close_hook] | handle value was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_vmap.c | 713 | CreateFileMappingW [fle_imp_CreateFileMappingW] | lpFileMappingAttributes is not NULL (security descriptors are not implemented) | (b) | — | — |
| lift/runtime/fle_vmap.c | 714 | CreateFileMappingW [fle_imp_CreateFileMappingW] | a named file mapping object is not implemented | (b) | — | — |
| lift/runtime/fle_vmap.c | 715 | CreateFileMappingW [fle_imp_CreateFileMappingW] | flProtect is not PAGE_READONLY or PAGE_READWRITE (not implemented) | (b) | — | — |
| lift/runtime/fle_vmap.c | 716 | CreateFileMappingW [fle_imp_CreateFileMappingW] | flProtect has a SEC_* attribute that is not implemented | (b) | — | — |
| lift/runtime/fle_vmap.c | 718 | CreateFileMappingW [fle_imp_CreateFileMappingW] | an anonymous object needs a size; size 0 is a failure the page gives no code for | (b) | — | — |
| lift/runtime/fle_vmap.c | 725 | CreateFileMappingW [fle_imp_CreateFileMappingW] | the file handle has no read access (the page requires GENERIC_READ; failure code not given) | (b) | — | — |
| lift/runtime/fle_vmap.c | 726 | CreateFileMappingW [fle_imp_CreateFileMappingW] | PAGE_READWRITE over a virtual file is not implemented (no write-back to the file) | (b) | — | — |
| lift/runtime/fle_vmap.c | 731 | CreateFileMappingW [fle_imp_CreateFileMappingW] | a maximum size above the file's length is not defined for a read-only object | (b) | — | — |
| lift/runtime/fle_vmap.c | 734 | CreateFileMappingW [fle_imp_CreateFileMappingW] | object larger than the runtime's limit of 1 GiB | (b) | — | — |
| lift/runtime/fle_vmap.c | 746 | CreateFileMappingW [fle_imp_CreateFileMappingW] | memfd_create failed | (b) | — | — |
| lift/runtime/fle_vmap.c | 747 | CreateFileMappingW [fle_imp_CreateFileMappingW] | ftruncate failed | (b) | — | — |
| lift/runtime/fle_vmap.c | 754 | CreateFileMappingW [fle_imp_CreateFileMappingW] | pwrite failed | (b) | — | — |
| lift/runtime/fle_vmap.c | 763 | CreateFileMappingW [fle_imp_CreateFileMappingW] | out of memory | (a) | — | — |
| lift/runtime/fle_vmap.c | 812 | MapViewOfFile [fle_imp_MapViewOfFile] | handle was not issued as a file mapping object | (b) | — | — |
| lift/runtime/fle_vmap.c | 823 | MapViewOfFile [fle_imp_MapViewOfFile] | write access to an object that was not created PAGE_READWRITE (failure code not given) | (b) | — | — |
| lift/runtime/fle_vmap.c | 831 | MapViewOfFile [fle_imp_MapViewOfFile] | dwDesiredAccess is not FILE_MAP_READ, FILE_MAP_WRITE, FILE_MAP_ALL_ACCESS or FILE_MAP_COPY | (b) | — | — |
| lift/runtime/fle_vmap.c | 833 | MapViewOfFile [fle_imp_MapViewOfFile] | file offset is not a multiple of the allocation granularity (failure code not given) | (b) | — | — |
| lift/runtime/fle_vmap.c | 834 | MapViewOfFile [fle_imp_MapViewOfFile] | file offset is outside the mapping (failure code not given) | (b) | — | — |
| lift/runtime/fle_vmap.c | 836 | MapViewOfFile [fle_imp_MapViewOfFile] | dwNumberOfBytesToMap goes beyond the object's size (failure code not given) | (b) | — | — |
| lift/runtime/fle_vmap.c | 854 | MapViewOfFile [fle_imp_MapViewOfFile] | a second live view of one object at the same offset (a view cannot be aliased at another address in wasm) | (b) | — | — |
| lift/runtime/fle_vmap.c | 876 | MapViewOfFile [fle_imp_MapViewOfFile] | out of memory | (a) | — | — |
| lift/runtime/fle_vmap.c | 907 | UnmapViewOfFile [fle_imp_UnmapViewOfFile] | munmap failed | (b) | — | — |
| lift/runtime/fle_vol.c | 27 | — [fle_vol_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_vol.c | 54 | GetStdHandle [fle_imp_GetStdHandle] | nStdHandle is not STD_INPUT_HANDLE, STD_OUTPUT_HANDLE or STD_ERROR_HANDLE | (b) | — | — |
| lift/runtime/fle_vol.c | 67 | — [fle_imp_GetStdHandle] | (動的な文言 why) | (b) | — | — |
| lift/runtime/fle_vol.c | 108 | — [root_letter] | root path is not of the form X:\\ (a trailing backslash is required; other forms are not implemented) | (b) | — | — |
| lift/runtime/fle_vol.c | 110 | — [root_letter] | root path does not start with a drive letter (other forms are not implemented) | (b) | — | — |
| lift/runtime/fle_vol.c | 151 | GetLogicalDriveStringsW [fle_imp_GetLogicalDriveStringsW] | lpBuffer is NULL with a non-zero nBufferLength | (b) | — | — |
| lift/runtime/fle_vol.c | 164 | — [put_name] | buffer size above the page's maximum of MAX_PATH + 1 | (b) | — | — |
| lift/runtime/fle_vol.c | 167 | — [put_name] | buffer too short for the answer (the page gives no behavior) | (b) | — | — |
| lift/runtime/fle_vol.c | 217 | QueryDosDeviceW [fle_imp_QueryDosDeviceW] | lpDeviceName NULL (list of all device names) is not implemented | (b) | — | — |
| lift/runtime/fle_vol.c | 219, 222 | QueryDosDeviceW [fle_imp_QueryDosDeviceW] | device name is not of the form X: (other names are not implemented) | (b) | — | — |
| lift/runtime/fle_vol.c | 234 | QueryDosDeviceW [fle_imp_QueryDosDeviceW] | lpTargetPath is NULL | (b) | — | — |
| lift/runtime/fle_vol.c | 277, 279 | — [vol_get] | handle value was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_vol.c | 289 | DuplicateHandle [vol_new] | out of memory | (a) | — | — |
| lift/runtime/fle_vol.c | 293 | DuplicateHandle [vol_new] | handle region exhausted | (a) | — | — |
| lift/runtime/fle_vol.c | 331, 334 | DuplicateHandle [fle_vol_source] | DUPLICATE_CLOSE_SOURCE on a pseudo handle is not defined | (b) | — | — |
| lift/runtime/fle_vol.c | 351 | DuplicateHandle [fle_vol_source] | duplicating a file handle is not implemented (it needs the file system's handle table) | (b) | — | — |
| lift/runtime/fle_vol.c | 353 | DuplicateHandle [fle_vol_source] | source handle was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_vol.c | 377 | DuplicateHandle [fle_imp_DuplicateHandle] | dwOptions has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_vol.c | 381 | DuplicateHandle [fle_imp_DuplicateHandle] | hSourceProcessHandle is not the current process (no other process exists) | (b) | — | — |
| lift/runtime/fle_vol.c | 384 | DuplicateHandle [fle_imp_DuplicateHandle] | hTargetProcessHandle NULL without DUPLICATE_CLOSE_SOURCE is not defined | (b) | — | — |
| lift/runtime/fle_vol.c | 388 | DuplicateHandle [fle_imp_DuplicateHandle] | hTargetProcessHandle is not the current process (no other process exists) | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 27 | InitializeCriticalSectionEx [fle_imp_InitializeCriticalSectionEx] | Flags other than 0 and CRITICAL_SECTION_NO_DEBUG_INFO: the page marks the RTL_* flags reserved | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 41 | GetFileSizeEx [fle_imp_GetFileSizeEx] | lpFileSize is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 66 | GetFileSizeEx [ex_locale] | a locale name other than NULL and \"en-US\" (the environment emulates one locale) | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 77 | LCMapStringEx [fle_imp_LCMapStringEx] | lpVersionInformation, lpReserved or sortHandle is not NULL: the page gives no behaviour here | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 87 | CompareStringEx [fle_imp_CompareStringEx] | lpVersionInformation, lpReserved or lParam is not NULL: the page gives no behaviour here | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 117 | GetLocaleInfoEx [fle_imp_GetLocaleInfoEx] | LOCALE_RETURN_NUMBER with a type that is not numeric | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 148 | EnumSystemLocalesW [fle_imp_EnumSystemLocalesW] | lpLocaleEnumProc is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 149 | EnumSystemLocalesW [fle_imp_EnumSystemLocalesW] | dwFlags is not LCID_INSTALLED, LCID_SUPPORTED or both | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 166 | CreateFileA [fle_imp_CreateFileA] | lpFileName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 169 | CreateFileA [fle_imp_CreateFileA] | a file name byte above 0x7F: the ANSI code page of the emulated system is not defined | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 173 | CreateFileA [fle_imp_CreateFileA] | a file name of 260 characters or more (MAX_PATH) | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 208 | CharNextA [fle_imp_CharNextA] | lpsz is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 220 | CharLowerBuffA [fle_imp_CharLowerBuffA] | lpsz is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 244 | InitOnceExecuteOnce [fle_imp_InitOnceExecuteOnce] | InitOnce or InitFn is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 247 | InitOnceExecuteOnce [fle_imp_InitOnceExecuteOnce] | more than 64 INIT_ONCE objects (a limit of this table) | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 271 | CompareFileTime [fle_imp_CompareFileTime] | a FILETIME pointer is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 286 | GetDiskFreeSpaceW [fle_imp_GetDiskFreeSpaceW] | a root path other than NULL and \"C:\\\": the volume has one drive and the page defines no other form here | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 306 | MoveFileWithProgressW [fle_imp_MoveFileWithProgressW] | lpProgressRoutine is not NULL: the callback is not modelled | (b) | — | — |
| lift/runtime/fle_wall_dec1.c | 307 | MoveFileWithProgressW [fle_imp_MoveFileWithProgressW] | dwFlags other than MOVEFILE_REPLACE_EXISTING, COPY_ALLOWED and WRITE_THROUGH | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 25 | WNetGetUniversalNameW [fle_imp_WNetGetUniversalNameW] | lpLocalPath is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 26 | WNetGetUniversalNameW [fle_imp_WNetGetUniversalNameW] | dwInfoLevel is neither UNIVERSAL_NAME_INFO_LEVEL (1) nor REMOTE_NAME_INFO_LEVEL (2): the page names no result | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 30 | WNetGetUniversalNameW [fle_imp_WNetGetUniversalNameW] | lpLocalPath is not a drive-based path: the page names no result (ERROR_BAD_DEVICE is 'the string is invalid', not defined further) | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 31 | WNetGetUniversalNameW [fle_imp_WNetGetUniversalNameW] | a drive other than C: (the virtual volume has one drive) | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 52 | GetFileInformationByHandle [fle_imp_GetFileInformationByHandle] | lpFileInformation is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 60 | GetFileInformationByHandle [fle_imp_GetFileInformationByHandle] | more than 256 files asked about (a limit of the file id table) | (b) | — | — |
| lift/runtime/fle_wall_dlg1.c | 88 | SetEndOfFile [fle_imp_SetEndOfFile] | the handle was opened without write access: the page names no error code for it | (b) | — | — |
| lift/runtime/fle_wall_exp1.c | 27 | SetThreadExecutionState [fle_imp_SetThreadExecutionState] | a flag outside ES_SYSTEM_REQUIRED / ES_DISPLAY_REQUIRED / ES_USER_PRESENT / ES_AWAYMODE_REQUIRED / ES_CONTINUOUS: the page names no result | (b) | — | — |
| lift/runtime/fle_wall_exp1.c | 29 | SetThreadExecutionState [fle_imp_SetThreadExecutionState] | ES_AWAYMODE_REQUIRED without ES_CONTINUOUS: the page says it must be specified with it and names no result | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 73 | GetKeyNameTextW [fle_imp_GetKeyNameTextW] | the scan code / extended flag has no name in the tables of the US layout (kbdus.c) and produces no character here: the page gives no failure code | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 74 | GetKeyNameTextW [fle_imp_GetKeyNameTextW] | lpString is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 76 | GetKeyNameTextW [fle_imp_GetKeyNameTextW] | cchSize cannot hold the key name and its NUL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 136 | MessageBoxW [fle_imp_MessageBoxW] | uType holds a button set that the page does not list | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 156 | ShellExecuteW [fle_imp_ShellExecuteW] | lpFile is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 173 | DoDragDrop [fle_imp_DoDragDrop] | pDropSource is NULL: the page gives no behaviour (E_INVALIDARG is a guess) | (b) | — | — |
| lift/runtime/fle_wall_ia2.c | 174 | DoDragDrop [fle_imp_DoDragDrop] | pdwEffect is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2t.c | 19 | GetTimeZoneInformationForYear [fle_imp_GetTimeZoneInformationForYear] | pdtzi is not NULL: a named time zone is not modelled (the virtual machine has one zone) | (b) | — | — |
| lift/runtime/fle_wall_ia2t.c | 20 | GetTimeZoneInformationForYear [fle_imp_GetTimeZoneInformationForYear] | ptzi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 65 | GetPrivateProfileStringW [fle_ia2_ini_get] | the initialization file holds a byte of 0x80 or above: the code page conversion is not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 153 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | lpReturnedString is NULL or nSize is below 3: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 154 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | lpFileName is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 156 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | a string longer than 600 characters is not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 187 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | the initialization file cannot be read | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 188 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | the initialization file is larger than 1 MiB: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 189 | GetPrivateProfileStringW [fle_imp_GetPrivateProfileStringW] | the initialization file starts with a UTF-16 byte-order mark: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 229 | WritePrivateProfileStringW [fle_ia2_ini_put] | a non-ASCII unit in lpAppName: the code page conversion is not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 230 | WritePrivateProfileStringW [fle_ia2_ini_put] | a non-ASCII unit in lpKeyName: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 231 | WritePrivateProfileStringW [fle_ia2_ini_put] | a non-ASCII unit in lpString: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 232 | WritePrivateProfileStringW [fle_ia2_ini_put] | the initialization file holds a byte of 0x80 or above: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 270 | WritePrivateProfileStringW [fle_ia2_ini_put] | the file would exceed the buffer of this model | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 312 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | lpAppName is NULL but lpKeyName or lpString is not: the page defines a NULL lpAppName only for the flush of the cache (all of them NULL) | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 313 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | lpFileName is NULL for a write: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 314 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | a string longer than 600 characters is not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 334 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | the initialization file cannot be read | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 335 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | the initialization file is larger than 256 KiB: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2v.c | 336 | WritePrivateProfileStringW [fle_imp_WritePrivateProfileStringW] | the initialization file starts with a UTF-16 byte-order mark: not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 67 | ScrollWindowEx [scroll_window_ex_simple] | SW_SCROLLCHILDREN: moving the child windows (and the WM_MOVE they get) is not modelled | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 75 | ScrollWindowEx [scroll_window_ex_simple] | prcScroll has a negative size: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 77 | ScrollWindowEx [scroll_window_ex_simple] | prcClip has a negative size: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 87 | ScrollWindowEx [scroll_window_ex_simple] | the drawing surface of the window cannot be written (no surface yet, or a format that is not 24 or 32 bit) | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 174 | ScrollWindowEx [scroll_run] | prcScroll has a negative size: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 179 | ScrollWindowEx [scroll_run] | prcClip has a negative size: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 231 | ScrollWindow [fle_imp_ScrollWindow] | the window model for scrolling is not available in this build (fle_win_scroll answered 0) | (b) | — | — |
| lift/runtime/fle_wall_ia2w.c | 265 | InvalidateRgn [fle_imp_InvalidateRgn] | hRgn is not a live region: the page defines nothing for it | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 54 | CloseHandle [job_close_hook] | job handle value was not issued by this runtime | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 78, 81 | — [new_handle] | out of memory | (a) | — | — |
| lift/runtime/fle_wall_wrp1.c | 85 | — [new_handle] | job handle region exhausted | (a) | — | — |
| lift/runtime/fle_wall_wrp1.c | 107 | CreateJobObjectW [fle_imp_CreateJobObjectW] | lpJobAttributes is not NULL (security descriptors and inheritance are not modelled) | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 110 | CreateJobObjectW [fle_imp_CreateJobObjectW] | a name longer than MAX_PATH characters: the page names no result | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 122 | CreateJobObjectW [fle_imp_CreateJobObjectW] | out of memory | (a) | — | — |
| lift/runtime/fle_wall_wrp1.c | 158 | SetInformationJobObject [fle_imp_SetInformationJobObject] | handle was not issued as a job object by this runtime | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 164 | SetInformationJobObject [fle_imp_SetInformationJobObject] | only JobObjectExtendedLimitInformation (9) is modelled | (b) | — | — |
| lift/runtime/fle_wall_wrp1.c | 165 | SetInformationJobObject [fle_imp_SetInformationJobObject] | the information buffer does not reach LimitFlags: the page names no result | (b) | — | — |
| lift/runtime/fle_webview2.c | 284 | WebView2 [vt_init] | a vtable of the model is larger than WV_SLOTS_MAX | (b) | — | — |
| lift/runtime/fle_webview2.c | 319 | WebView2 [new_obj] | no memory for a WebView2 object | (b) | — | — |
| lift/runtime/fle_webview2.c | 408 | WebView2 [task_wstr] | no task memory for a string of the model | (b) | — | — |
| lift/runtime/fle_webview2.c | 443 | WebView2 [complete] | the guest callback mechanism is not linked: a completion handler cannot be called | (a) | — | — |
| lift/runtime/fle_webview2.c | 726 | WebView2 [call_slot] | a slot of the model has no kind | (b) | — | — |
| lift/runtime/fle_win.c | 144 | — [fle_win_check_thread] | called from a second thread, not supported | (b) | — | — |
| lift/runtime/fle_win.c | 212, 216 | — [need_window] | (組み立ての文言) hwnd 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 242 | — [thunk_target] | the window procedure lies at the end of a VirtualAlloc region: not the thunk of MakeObjectInstance (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 246, 250 | — [thunk_target] | (組み立ての文言) the window procedure 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 254 | — [thunk_target] | the E8 call of the window procedure leaves its VirtualAlloc region: not the thunk of MakeObjectInstance (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 257 | — [thunk_target] | (組み立ての文言) the bytes at 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 532 | window table [zl_insert_at] | out of memory | (a) | — | — |
| lift/runtime/fle_win.c | 874 | SendMessageW [fle_imp_SendMessageW] | hWnd is HWND_BROADCAST (0xffff): sending to all top-level windows is not modelled (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 914 | SendMessageA [fle_imp_SendMessageA] | this message carries text or a character: the conversion between the ANSI form and the Unicode window procedure is not modelled (S-ref1-sendmessagea-text) | (b) | — | — |
| lift/runtime/fle_win.c | 942 | SetClassLongPtrW [fle_imp_SetClassLongPtrW] | (組み立ての文言) nIndex %d is not GCLP_HICON (-14): the other class values (cursor, brush, menu, style, module, procedure, extra memory) are not modelled (de… | (b) | — | — |
| lift/runtime/fle_win.c | 988 | GetSystemMenu [fle_imp_GetSystemMenu] | the window has no WS_SYSMENU: the page does not say what GetSystemMenu returns for it (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1020 | DeleteMenu [fle_imp_DeleteMenu] | (組み立ての文言) hMenu 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 1024 | DeleteMenu [fle_imp_DeleteMenu] | (組み立ての文言) uFlags 0x%x is not MF_BYCOMMAND (0): other forms (MF_BYPOSITION ...) are not modelled (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1031 | DeleteMenu [fle_imp_DeleteMenu] | (組み立ての文言) item 0x%x is not in the window menu: the page gives no failure code and no last error (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1053 | WTSRegisterSessionNotification [fle_imp_WTSRegisterSessionNotification] | dwFlags is neither NOTIFY_FOR_THIS_SESSION nor NOTIFY_FOR_ALL_SESSIONS: the page gives no value for another one (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1138 | CreateWindowExW [fle_imp_CreateWindowExW] | hWndParent is a message-only window (a handle to an existing message-only window): not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 1148 | CreateWindowExW [fle_imp_CreateWindowExW] | (組み立ての文言) a message-only window (hWndParent HWND_MESSAGE) with dwExStyle 0x%x and dwStyle 0x%x: only dwExStyle 0 and dwStyle 0 are answered (decision … | (b) | — | — |
| lift/runtime/fle_win.c | 1151 | CreateWindowExW [fle_imp_CreateWindowExW] | hMenu is not NULL for a window that is not a child: menus are not modelled (decision 0071) | (b) | — | — |
| lift/runtime/fle_win.c | 1152 | CreateWindowExW [fle_imp_CreateWindowExW] | hInstance is not the lifted DLL (0x400000): not answered (decision 0071) | (b) | — | — |
| lift/runtime/fle_win.c | 1162 | CreateWindowExW [fle_imp_CreateWindowExW] | lpWindowName is longer than 256 units: not answered (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1176 | CreateWindowExW [fle_imp_CreateWindowExW] | the window handle region is exhausted | (a) | — | — |
| lift/runtime/fle_win.c | 1181 | CreateWindowExW [fle_imp_CreateWindowExW] | out of memory | (a) | — | — |
| lift/runtime/fle_win.c | 1279 | CreateWindowExW [fle_imp_CreateWindowExW] | (組み立ての文言) WM_NCCALCSIZE (wParam FALSE) returned 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 1403 | SetWindowLongPtrW [fle_imp_SetWindowLongPtrW] | GWLP_HWNDPARENT of a child window: the page says to use SetParent and gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 1412 | SetWindowLongPtrW [fle_imp_SetWindowLongPtrW] | a window as its own owner: the page does not say | (b) | — | — |
| lift/runtime/fle_win.c | 1426 | SetWindowLongPtrW [fle_imp_SetWindowLongPtrW] | (組み立ての文言) nIndex %d is not GWLP_WNDPROC (-4), GWL_EXSTYLE (-20), GWL_STYLE (-16), GWLP_ID (-12), GWLP_HINSTANCE (-6), GWLP_HWNDPARENT (-8) or GWLP_USE… | (b) | — | — |
| lift/runtime/fle_win.c | 1428 | SetWindowLongPtrW [fle_imp_SetWindowLongPtrW] | dwNewLong is 0 for GWLP_WNDPROC: the page does not say what a NULL window procedure means (decision 0071) | (b) | — | — |
| lift/runtime/fle_win.c | 1459 | GetWindowLongPtrW [fle_imp_GetWindowLongPtrW] | GWLP_HWNDPARENT of a message-only window: the pages do not say what the parent of a message-only window is (decision 0121) | (b) | — | — |
| lift/runtime/fle_win.c | 1470 | GetWindowLongPtrW [fle_imp_GetWindowLongPtrW] | (組み立ての文言) nIndex %d is not one of GWL_EXSTYLE, GWL_STYLE, GWLP_ID, GWLP_HINSTANCE, GWLP_HWNDPARENT, GWLP_USERDATA, GWLP_WNDPROC: extra memory and DWLP… | (b) | — | — |
| lift/runtime/fle_win.c | 1525 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_NCCREATE with a NULL lParam: the page says lParam points to the CREATESTRUCT | (b) | — | — |
| lift/runtime/fle_win.c | 1529 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_CREATE with a NULL lParam: the page says lParam points to the CREATESTRUCT | (b) | — | — |
| lift/runtime/fle_win.c | 1533 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_NCCALCSIZE with wParam not FALSE (NCCALCSIZE_PARAMS): not answered (decision 0071) | (b) | — | — |
| lift/runtime/fle_win.c | 1534 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_NCCALCSIZE with a NULL lParam: the page says lParam points to a RECT | (b) | — | — |
| lift/runtime/fle_win.c | 1548 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_SETTEXT with a string longer than 256 units: not answered (decision 0119) | (b) | — | — |
| lift/runtime/fle_win.c | 1566 | DefWindowProcW [fle_imp_DefWindowProcW] | (組み立ての文言) WM_SETICON with wParam 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 1580 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_WINDOWPOSCHANGING with a NULL lParam: the page says lParam points to a WINDOWPOS | (b) | — | — |
| lift/runtime/fle_win.c | 1590 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_WINDOWPOSCHANGED with a NULL lParam: the page says lParam points to a WINDOWPOS | (b) | — | — |
| lift/runtime/fle_win.c | 1633 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_GETTEXT with a NULL buffer: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 1701 | DefWindowProcW [fle_imp_DefWindowProcW] | WM_GETMINMAXINFO with a NULL lParam: the page says lParam points to a MINMAXINFO | (b) | — | — |
| lift/runtime/fle_win.c | 1757 | DefWindowProcW [fle_imp_DefWindowProcW] | (組み立ての文言) message 0x%x: the default processing is not answered (decision 0071: WM_NCCREATE, WM_NCCALCSIZE with wParam FALSE, WM_CREATE; decision 0119:… | (b) | — | — |
| lift/runtime/fle_win.c | 1783 | DefWindowProcW [need_scheduler] | no thread scheduler is linked (fle_thread.c): the message queue is not available | (b) | — | — |
| lift/runtime/fle_win.c | 1816 | PostMessageW [fle_imp_PostMessageW] | hWnd is HWND_BROADCAST (0xffff): posting to all top-level windows is not modelled (decision 0122) | (b) | — | — |
| lift/runtime/fle_win.c | 1817 | PostMessageW [fle_imp_PostMessageW] | WM_QUIT through PostMessageW: the page says to use PostQuitMessage and does not say what the call does (decision 0122) | (b) | — | — |
| lift/runtime/fle_win.c | 1861 | GetMessageW [fle_imp_GetMessageW] | lpMsg is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 1929 | PeekMessageW [fle_imp_PeekMessageW] | lpMsg is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 1934 | PeekMessageW [fle_imp_PeekMessageW] | (組み立ての文言) wRemoveMsg %#x has a bit that is not PM_REMOVE, PM_NOYIELD or a PM_QS_* value of the page: not modelled (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 1945 | PeekMessageW [fle_imp_PeekMessageW] | hWnd is not a live window of the calling thread, NULL or -1: the page gives no failure value for PeekMessage (decision 0122) | (b) | — | — |
| lift/runtime/fle_win.c | 2075 | TranslateMessage [fle_imp_TranslateMessage] | lpMsg is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2079 | TranslateMessage [fle_imp_TranslateMessage] | a key message: the keyboard layout that makes the character message is not modelled (decision 0122) | (b) | — | — |
| lift/runtime/fle_win.c | 2104 | PostThreadMessageW [dispatch_message] | lpMsg is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2114 | PostThreadMessageW [dispatch_message] | an ANSI message that carries text (WM_CREATE, WM_NCCREATE, WM_SETTEXT, WM_GETTEXT, a character above 0x7F): the conversion to the Unicode form the page describes is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 2128 | PostThreadMessageW [dispatch_message] | the message is for a window that another thread owns: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2300, 2357 | GetWindow [fle_imp_GetWindow] | uCmd is not one of the values of the page: not defined | (b) | — | — |
| lift/runtime/fle_win.c | 2401 | GetAncestor [fle_imp_GetAncestor] | gaFlags is not GA_PARENT, GA_ROOT or GA_ROOTOWNER: the page defines no other value | (b) | — | — |
| lift/runtime/fle_win.c | 2453 | GetWindowRect [fle_imp_GetWindowRect] | lpRect is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2484 | MonitorFromWindow [fle_imp_MonitorFromWindow] | no monitor model is linked (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 2567 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | (組み立ての文言) idHook %d is not WH_MOUSE (7), WH_KEYBOARD (2) or WH_CALLWNDPROC (4): the other hooks are called by messages that do occur here and are not … | (b) | — | — |
| lift/runtime/fle_win.c | 2569 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | dwThreadId is 0: a global hook needs a DLL and every thread of the desktop (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 2571 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | dwThreadId is not the id of a thread of this process: the page gives no failure code (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 2572 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | lpfn is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2573 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | hmod is not NULL for a thread of the current process: the page says it must be NULL | (b) | — | — |
| lift/runtime/fle_win.c | 2574 | SetWindowsHookExW [fle_imp_SetWindowsHookExW] | more than 64 hooks are alive: a limit of the virtual machine | (b) | — | — |
| lift/runtime/fle_win.c | 2655 | WH_CALLWNDPROC [callwndproc_hooks] | hook procedures are nested more than 8 deep (a limit of the virtual machine) | (b) | — | — |
| lift/runtime/fle_win.c | 2677 | CallNextHookEx [fle_imp_CallNextHookEx] | called outside a hook procedure: the current hook is unknown (hhk is ignored by the page's text) | (b) | — | — |
| lift/runtime/fle_win.c | 2719 | WH_MOUSE / WH_KEYBOARD [fle_win_input_call_hooks] | hook procedures are nested more than 8 deep (a limit of the virtual machine) | (b) | — | — |
| lift/runtime/fle_win.c | 2743 | GetClientRect [fle_imp_GetClientRect] | lpRect is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2769 | ClientToScreen [fle_imp_ClientToScreen] | lpPoint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2790 | ScreenToClient [fle_imp_ScreenToClient] | lpPoint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2814 | MapWindowPoints [fle_imp_MapWindowPoints] | lpPoints is NULL with cPoints > 0: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 2818, 2824 | MapWindowPoints [fle_imp_MapWindowPoints] | a mirrored window (WS_EX_LAYOUTRTL): not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 3528 | MapWindowPoints [apply_pos] | (組み立ての文言) WM_WINDOWPOSCHANGING returned 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 3620 | MapWindowPoints [apply_pos] | hWndInsertAfter is not a window of this table and not one of HWND_TOP, HWND_BOTTOM, HWND_TOPMOST, HWND_NOTOPMOST: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 3643 | MapWindowPoints [apply_pos] | (組み立ての文言) WM_NCCALCSIZE (wParam FALSE) returned 0x% | (b) | — | — |
| lift/runtime/fle_win.c | 3931 | SetWindowPos [fle_imp_SetWindowPos] | hWnd is the desktop window: the page does not say what moving it does | (b) | — | — |
| lift/runtime/fle_win.c | 3934 | SetWindowPos [fle_imp_SetWindowPos] | SWP_ASYNCWINDOWPOS for a window of another thread: not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 3937 | SetWindowPos [fle_imp_SetWindowPos] | uFlags has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_win.c | 3938 | SetWindowPos [fle_imp_SetWindowPos] | SWP_SHOWWINDOW and SWP_HIDEWINDOW together: the page does not say | (b) | — | — |
| lift/runtime/fle_win.c | 4023 | ShowWindow [fle_imp_ShowWindow] | (組み立ての文言) nCmdShow %d: SW_SHOWDEFAULT (10), SW_FORCEMINIMIZE (11) and values outside the list are not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 4168 | SetParent [fle_imp_SetParent] | hWndNewParent is HWND_MESSAGE: turning a window into a message-only window is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 4176 | SetParent [fle_imp_SetParent] | a window as its own parent: the page does not say | (b) | — | — |
| lift/runtime/fle_win.c | 4181 | SetParent [fle_imp_SetParent] | the new parent is a descendant of the window: the page does not say | (b) | — | — |
| lift/runtime/fle_win.c | 4611 | GetGuiResources [fle_imp_GetGuiResources] | hProcess is not the pseudo handle of GetCurrentProcess: no other process exists (decision 0129) | (b) | — | — |
| lift/runtime/fle_win.c | 4613 | GetGuiResources [fle_imp_GetGuiResources] | no GDI table is linked (decision 0129) | (a) | — | — |
| lift/runtime/fle_win.c | 4622 | GetGuiResources [fle_imp_GetGuiResources] | uiFlags is a peak value or not on the page: the peak counts are not kept (decision 0129) | (b) | — | — |
| lift/runtime/fle_win.c | 4923 | EnumWindows [fle_imp_EnumWindows] | lpEnumFunc is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 4965 | EnumChildWindows [fle_imp_EnumChildWindows] | lpEnumFunc is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 4995 | EnumThreadWindows [fle_imp_EnumThreadWindows] | lpfn is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 5034 | FindWindowW [fle_imp_FindWindowW] | lpClassName is longer than 256 units: not answered | (b) | — | — |
| lift/runtime/fle_win.c | 5043 | FindWindowW [fle_imp_FindWindowW] | lpWindowName is longer than 256 units: not answered | (b) | — | — |
| lift/runtime/fle_win.c | 5423 | SetTimer [fle_imp_SetTimer] | hWnd is not NULL and nIDEvent is 0: the page says the identifier is nonzero and gives no behaviour for 0 | (b) | — | — |
| lift/runtime/fle_win.c | 5585 | InvalidateRect [fle_imp_InvalidateRect] | hWnd is NULL: the page says every window is invalidated and redrawn: not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 5613 | ValidateRect [fle_imp_ValidateRect] | hWnd is NULL: the page says every window is invalidated and redrawn: not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 5639 | ValidateRgn [fle_imp_ValidateRgn] | hWnd is NULL: the page does not define it | (b) | — | — |
| lift/runtime/fle_win.c | 5648 | ValidateRgn [fle_imp_ValidateRgn] | no GDI region table is linked | (a) | — | — |
| lift/runtime/fle_win.c | 5683 | GetUpdateRect [fle_imp_GetUpdateRect] | bErase TRUE with a non-empty update region: the WM_ERASEBKGND the page says is sent is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 5715 | GetUpdateRgn [fle_imp_GetUpdateRgn] | no GDI region table is linked | (a) | — | — |
| lift/runtime/fle_win.c | 5717 | GetUpdateRgn [fle_imp_GetUpdateRgn] | bErase TRUE with a non-empty update region: the erasing the page says is done is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 5791 | BeginPaint [fle_imp_BeginPaint] | lpPaint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 5798 | BeginPaint [fle_imp_BeginPaint] | BeginPaint for a window that is already between BeginPaint and EndPaint: the page says each call must be matched by EndPaint | (b) | — | — |
| lift/runtime/fle_win.c | 5799 | BeginPaint [fle_imp_BeginPaint] | no virtual screen DC is linked (fle_gdc.c): the DC of BeginPaint is not available | (b) | — | — |
| lift/runtime/fle_win.c | 5875 | EndPaint [fle_imp_EndPaint] | lpPaint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 5882 | EndPaint [fle_imp_EndPaint] | EndPaint for a window that is not between BeginPaint and EndPaint: the page says each BeginPaint has one EndPaint | (b) | — | — |
| lift/runtime/fle_win.c | 6051 | GetCursorPos [fle_imp_GetCursorPos] | lpPoint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6063 | GetPhysicalCursorPos [fle_imp_GetPhysicalCursorPos] | lpPoint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6092 | GetCursorInfo [fle_imp_GetCursorInfo] | pci is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6098 | GetCursorInfo [fle_imp_GetCursorInfo] | no GDI cursor state is linked (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 6218 | GetClassNameW [fle_imp_GetClassNameW] | the window's class is not in the class table | (b) | — | — |
| lift/runtime/fle_win.c | 6254 | LockWindowUpdate [prop_key] | lpString is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6263 | LockWindowUpdate [prop_key] | the property name is longer than 256 units: not answered | (b) | — | — |
| lift/runtime/fle_win.c | 6307 | SetPropW [fle_imp_SetPropW] | out of memory | (a) | — | — |
| lift/runtime/fle_win.c | 6373 | CallWindowProcW [fle_imp_CallWindowProcW] | lpPrevWndFunc is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6388 | AdjustWindowRectEx [fle_imp_AdjustWindowRectEx] | lpRect is NULL: the page gives no behaviour (decision 0126) | (b) | — | — |
| lift/runtime/fle_win.c | 6411 | SetMenu [fle_imp_SetMenu] | hMenu is not NULL: menus are not modelled (decision 0123) | (b) | — | — |
| lift/runtime/fle_win.c | 6466 | scroll bar [sb_get] | more windows with scroll bars than the runtime holds | (b) | — | — |
| lift/runtime/fle_win.c | 6495 | SetMenu [sb_read_info] | lpsi is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6502 | SetMenu [sb_read_info] | (組み立ての文言) SCROLLINFO.cbSize %u is not sizeof(SCROLLINFO) = %u: the page says the caller must set it | (b) | — | — |
| lift/runtime/fle_win.c | 6504 | SetMenu [sb_read_info] | SCROLLINFO.fMask has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_win.c | 6535 | SetScrollInfo [fle_imp_SetScrollInfo] | nBar is not SB_HORZ or SB_VERT: the scroll bar control (SB_CTL) is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 6555 | GetScrollInfo [fle_imp_GetScrollInfo] | nBar is not SB_HORZ or SB_VERT: the scroll bar control (SB_CTL) is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 6580 | ShowScrollBar [fle_imp_ShowScrollBar] | wBar is SB_CTL: the scroll bar control is not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 6581 | ShowScrollBar [fle_imp_ShowScrollBar] | wBar is none of SB_HORZ (0), SB_VERT (1), SB_CTL (2) and SB_BOTH (3): the page lists these four | (b) | — | — |
| lift/runtime/fle_win.c | 6625 | FlatSB_SetScrollProp [fle_imp_FlatSB_SetScrollProp] | index is not one WSB_PROP_* value of the SDK header (the page names the values and gives no numbers) | (b) | — | — |
| lift/runtime/fle_win.c | 6650 | FlatSB_SetScrollInfo [fle_imp_FlatSB_SetScrollInfo] | code is not SB_HORZ or SB_VERT: the page lists these two | (b) | — | — |
| lift/runtime/fle_win.c | 6673 | FlatSB_GetScrollInfo [fle_imp_FlatSB_GetScrollInfo] | code is not SB_HORZ or SB_VERT: the page lists these two | (b) | — | — |
| lift/runtime/fle_win.c | 6697 | FlatSB_SetScrollPos [fle_imp_FlatSB_SetScrollPos] | code is not SB_HORZ or SB_VERT: the page lists these two | (b) | — | — |
| lift/runtime/fle_win.c | 6718 | FlatSB_GetScrollPos [fle_imp_FlatSB_GetScrollPos] | code is not SB_HORZ or SB_VERT: the page lists these two | (b) | — | — |
| lift/runtime/fle_win.c | 6741 | GetWindowPlacement [fle_imp_GetWindowPlacement] | lpwndpl is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6783 | SetWindowPlacement [fle_imp_SetWindowPlacement] | lpwndpl is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 6999 | EndDeferWindowPos [def_erase_background] | no GDI is linked (fle_gdi.c, fle_gdc.c): the class background brush of the window cannot be filled (decision 0323) | (b) | — | — |
| lift/runtime/fle_win.c | 7035 | EndDeferWindowPos [erase_now_one] | no virtual screen DC is linked (fle_gdc.c): the DC of WM_ERASEBKGND is not available | (b) | — | — |
| lift/runtime/fle_win.c | 7109 | RedrawWindow [fle_imp_RedrawWindow] | hWnd is NULL (the desktop window): not modelled | (b) | — | — |
| lift/runtime/fle_win.c | 7110 | RedrawWindow [fle_imp_RedrawWindow] | flags has a bit that the page does not list | (b) | — | — |
| lift/runtime/fle_win.c | 7117 | RedrawWindow [fle_imp_RedrawWindow] | no GDI region table is linked | (a) | — | — |
| lift/runtime/fle_win.c | 7118 | RedrawWindow [fle_imp_RedrawWindow] | hrgnUpdate is not a live region of this runtime: the page does not define it | (b) | — | — |
| lift/runtime/fle_win.c | 7590 | RedrawWindow [point_identity] | lpPoint is NULL: the page gives no behaviour | (b) | — | — |
| lift/runtime/fle_win.c | 7619 | SetWindowRgn [fle_imp_SetWindowRgn] | the GDI region model (fle_gdi.c) is not linked | (a) | — | — |
| lift/runtime/fle_win.c | 7620 | SetWindowRgn [fle_imp_SetWindowRgn] | hRgn is not NULL and not a region made by CreateRectRgn / CreateRectRgnIndirect: other regions are not modelled | (b) | — | — |
| lift/runtime/fle_wininet.c | 18 | InternetGetConnectedState [fle_imp_InternetGetConnectedState] | dwReserved is not 0: the page says it must be 0 and gives no behaviour for another value (decision 0126) | (b) | — | — |
| lift/runtime/fle_wininet.c | 72 | InternetOpenW [fle_imp_InternetOpenW] | lpszAgent is NULL: the page gives no behaviour (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 73 | InternetOpenW [fle_imp_InternetOpenW] | dwAccessType is not DIRECT, PRECONFIG or PRECONFIG_WITH_NO_AUTOPROXY: a proxy needs a model of its own (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 74 | InternetOpenW [fle_imp_InternetOpenW] | lpszProxy or lpszProxyBypass is not NULL: the page says they are ignored unless the type is PROXY and gives no behaviour for a value (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 75 | InternetOpenW [fle_imp_InternetOpenW] | dwFlags is not 0: the asynchronous and the cache-only modes need a model of their own (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 76 | InternetOpenW [fle_imp_InternetOpenW] | more sessions than the runtime holds | (b) | — | — |
| lift/runtime/fle_wininet.c | 90 | InternetSetOptionW [fle_imp_InternetSetOptionW] | hInternet is not a live session of this runtime: the page names no error code (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 91 | InternetSetOptionW [fle_imp_InternetSetOptionW] | only the time-out options CONNECT_TIMEOUT (2), SEND_TIMEOUT (5) and RECEIVE_TIMEOUT (6) with a 4-byte value are answered (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 101 | InternetOpenUrlW [fle_imp_InternetOpenUrlW] | hInternet is not a live session of this runtime: the page names no error code (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 102 | InternetOpenUrlW [fle_imp_InternetOpenUrlW] | lpszUrl is NULL: the page gives no behaviour (decision 0132) | (b) | — | — |
| lift/runtime/fle_wininet.c | 115 | InternetCloseHandle [fle_imp_InternetCloseHandle] | hInternet is not a live session of this runtime: the page names no error code (decision 0132) | (b) | — | — |
| lift/runtime/fle_winmm.c | 29 | — [fle_winmm_devcaps] | the caps pointer is NULL: the page does not say if INVALPARAM or BADDEVICEID wins | (b) | — | — |
| lift/runtime/fle_winmm.c | 30 | — [fle_winmm_devcaps] | cb is zero: the page says NOERROR and nothing copied, but not when the identifier is out of range | (b) | — | — |
| lift/runtime/fle_winmm.c | 32 | — [fle_winmm_devcaps] | identifier above 32 bits: could be a device handle, none exists | (b) | — | — |
| lift/runtime/fle_winmm.c | 55 | midiOutGetErrorTextW [fle_imp_midiOutGetErrorTextW] | the error text is a system string that the page does not give, and BADERRNUM has no defined range | (b) | — | — |
| lift/runtime/fle_winmm.c | 69 | midiOutOpen [fle_imp_midiOutOpen] | phmo is NULL (the page does not say what is returned) | (b) | — | — |
| lift/runtime/fle_winmm.c | 70 | midiOutOpen [fle_imp_midiOutOpen] | fdwOpen has a flag other than a callback type (the page lists only those) | (b) | — | — |
| lift/runtime/fle_winmm.c | 73 | midiOutOpen [fle_imp_midiOutOpen] | fdwOpen callback type is not one of the five on the page | (b) | — | — |
| lift/runtime/fle_winmm.c | 112 | timeBeginPeriod [fle_imp_timeBeginPeriod] | too many unmatched calls for one period | (b) | — | — |
| lift/runtime/fle_winmm.c | 119 | timeBeginPeriod [fle_imp_timeBeginPeriod] | too many different periods outstanding (limit of this runtime) | (b) | — | — |
| lift/runtime/fle_winmm.c | 139 | timeEndPeriod [fle_imp_timeEndPeriod] | no matching timeBeginPeriod for this period (the page does not say what is returned) | (b) | — | — |
| lift/runtime/fle_winmm.c | 151 | timeGetDevCaps [fle_imp_timeGetDevCaps] | cbtc is not sizeof(TIMECAPS) = 8: the page does not say which sizes are invalid | (b) | — | — |
| lift/runtime/fle_winmm.c | 171 | acmMetrics [fle_imp_acmMetrics] | uMetric is not ACM_METRIC_MAX_SIZE_FORMAT (50): only the metric the guest asks is answered (decision 0098) | (b) | — | — |
| lift/runtime/fle_winmm.c | 172 | acmMetrics [fle_imp_acmMetrics] | hao is not NULL: no ACM object exists (decision 0098) | (b) | — | — |
| lift/runtime/fle_winmm.c | 173 | acmMetrics [fle_imp_acmMetrics] | pMetric is NULL: the page does not say which error applies (decision 0098) | (b) | — | — |
