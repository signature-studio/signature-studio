# 上位 import の仕様の確認状況（2026-10-20）

作業者（M1）が、Microsoft Learn（learn.microsoft.com）の公式ページで確認した結果。件数は `lift/out-usage/iat-slot-counts-v3.tsv` による。「応答の方針」は、ランタイムに応答を作れるかの判断で、実装の有無ではない。

凡例:
- 確認: 公式ページを取得し、仕様の要点を読んだ。
- 未確認: ページが取得できなかった（404 など）。仕様は確かめていない。
- 応答の方針:
  - 「仕様から決まる」: 戻り値・副作用が OS 状態を読まず、仕様だけで応答を定められる。
  - 「仕様から決まらない」: 戻り値が実行中の環境（OS・ロケール・ファイル・ウィンドウ等）に依存し、FLEngine が要求する値は仕様からは決まらない。
  - 「保留」: 仕様は確認したが、次の判断（呼び出し元の使い方）が必要。

今回は応答を 1 件追加した: kernel32.dll!LocalFree（仕様から決まる範囲のみ。`fle_imports.c` の `fle_imp_LocalFree`）。対応表は 8 件（`imports_table.rs`）。

## 件数 3

| import | 件数 | 確認 | 出典 URL | 仕様の要点 | 応答の方針 |
|---|---|---|---|---|---|
| user32.dll!SendMessageW | 3 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagew | ウィンドウ（HWND）へメッセージを送り、処理が終わるまで戻らない。戻り値はメッセージ依存 | 仕様から決まらない（相手のウィンドウプロシージャに依存） |
| user32.dll!SystemParametersInfoW | 3 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow | システム全体の設定（uiAction）を取得・設定する。戻り値は設定値 | 仕様から決まらない（OS の設定値を読む） |
| kernel32.dll!FreeLibrary | 3 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary | モジュールの参照カウントを減らす。成功で非 0、失敗で 0 | 保留（ハンドルの管理が必要。ランタイムに DLL のローダーがない） |
| kernel32.dll!CompareStringW | 3 | 未確認 | （https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-comparestringw は 404。stringapiset 側の URL は取得できた。下記） | 下記 | 下記 |
| kernel32.dll!SetCurrentDirectoryW | 3 | 未確認 | （`winbase` 側、`processenv` 側、ANSI 版の URL は 404） | 未取得 | 未確認 |
| kernel32.dll!GetProcAddress | 3 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress | モジュール内のエクスポート名（またはオーディナル）から関数アドレスを返す。失敗で NULL | 保留（モジュールのハンドルと実アドレスが要る。ランタイムに仕組みがない） |
| kernel32.dll!GetLocaleInfoW | 3 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getlocaleinfow | ロケール（LCID）の情報（LCType）を返す。戻り値は文字数 | 仕様から決まらない（ロケールの値が実行環境に依存） |

### CompareStringW（stringapiset 側で確認）

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringw （確認）
- 要点: 2 つの文字列を、ロケール（LCID）の規則で比較する。戻り値は CompareStringEx の規則に従う、とページにある。戻り値の具体的な値（CSTR_* の数値）は、このページには書かれていない。CompareStringEx のページは未読（確認していない）。
- 応答の方針: 仕様から決まらない（ロケールの指定（LCID）が実行環境の既定値を使うことがあるため。今回のページは詳細を読んでいない）。

## 件数 2（一部。確認できたもの）

| import | 件数 | 確認 | 出典 URL | 仕様の要点 | 応答の方針 |
|---|---|---|---|---|---|
| kernel32.dll!LocalFree | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-localfree | NULL を渡すと何もせず NULL を返す。成功で NULL、失敗で hMem を返す | 仕様から決まる。実装済み（NULL は無視して NULL。それ以外は free して NULL）。LocalAlloc（0003）が返すポインタだけを扱う前提（仮定。LMEM_MOVEABLE は未対応） |
| kernel32.dll!GetACP | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getacp | 現在の ANSI コードページ（UINT）を返す | 仕様から決まらない（コードページが環境に依存） |
| kernel32.dll!GetTickCount | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount | 起動からの経過ミリ秒（DWORD、49.7 日で折り返す） | 仕様から決まらない（起動時刻に依存） |
| kernel32.dll!Sleep | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-sleep | 指定時間スレッドを止める。戻り値なし | 保留（指定時間の待ちで、実時間に依存する。応答を作るなら、待ちの扱い（実待ちか、即時戻りか）を決める必要がある） |
| kernel32.dll!VirtualAlloc | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc | 仮想アドレス領域を予約・コミットする。成功でベースアドレス、失敗で NULL。確保は 0 で初期化 | 保留（アドレスの選択がホストに依存。ゲストのメモリ配置と合わせる設計が要る） |
| kernel32.dll!GetModuleHandleW | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew | モジュール名からハンドルを返す。失敗で NULL | 保留（ハンドルの値をどう定めるかが要る） |
| kernel32.dll!VerifyVersionInfoW | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow | 実行中の OS が条件を満たすかを返す。満たさないと 0 と ERROR_OLD_WIN_VERSION | 仕様から決まらない（OS の状態に依存。0012 で応答を作らないと決定） |
| kernel32.dll!VerSetConditionMask | 2 | 確認は 0011 の記録による（今回は再取得していない）。戻り値は条件マスク、ビット配置は公式に書かれていない | https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask | 条件マスクを返す。ビット配置は公式に書かれていない | 実装済み（0011）。配置は Wine 由来で、公式では確認できていない |
| kernel32.dll!GetVersionExW | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversionexw | OS のバージョン情報（OSVERSIONINFOW）を返す。マニフェストで値が変わる | 仕様から決まらない（OS の状態に依存） |
| kernel32.dll!GetVersion | 2 | 未確認 | （未取得） | — | 未確認 |
| kernel32.dll!EnterCriticalSection / LeaveCriticalSection / InitializeCriticalSection | 2 each | 未確認 | （未取得） | — | 未確認 |
| kernel32.dll!GetSystemInfo | 2 | 未確認 | （未取得） | — | 未確認 |
| kernel32.dll!FindClose | 2 | 確認 | https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-findclose | 検索ハンドルを閉じる。成功で非 0 | 保留（FindFirstFileW の検索ハンドル管理が要る） |
| kernel32.dll!GetModuleFileNameW / GetEnvironmentVariableW / FindFirstFileW / WriteFile / WideCharToMultiByte / IsValidLocale / QueryPerformanceCounter | 2 each | 未確認 | （未取得） | — | 未確認 |
| user32.dll!CharNextW / DrawTextW / DispatchMessageW / GetWindowThreadProcessId / InvalidateRect / GetWindow | 2 each | 未確認 | （未取得） | — | 未確認（いずれも GUI 関係。仕様から決まらない可能性が高いが、確認していない） |
| advapi32.dll!RegOpenKeyExW / RegQueryValueExW / RegCloseKey | 2 each | 未確認 | （未取得） | — | 未確認（レジストリの値は環境に依存する可能性が高いが、確認していない） |

## 確認できなかった・時間の都合で未確認のもの

- 件数 3: CompareStringW（stringapiset 側は確認。詳細は未読）、SetCurrentDirectoryW（3 つの URL を試し、いずれも 404）。
- 件数 2（未確認 21 件）: GetVersion、EnterCriticalSection、LeaveCriticalSection、InitializeCriticalSection、GetSystemInfo、QueryPerformanceCounter、GetModuleFileNameW、GetEnvironmentVariableW、FindFirstFileW、WriteFile、WideCharToMultiByte、IsValidLocale、CharNextW、DrawTextW、DispatchMessageW、GetWindowThreadProcessId、InvalidateRect、GetWindow、RegOpenKeyExW、RegQueryValueExW、RegCloseKey。
- 件数 1 以下: 未確認（件数順の上位を優先した）。

## 結果

- 公式ページで仕様を確認できたもの: 件数 3 で 6 件（SendMessageW、SystemParametersInfoW、FreeLibrary、GetProcAddress、GetLocaleInfoW、CompareStringW）。件数 2 で 10 件（LocalFree、GetACP、GetTickCount、Sleep、VirtualAlloc、GetModuleHandleW、VerifyVersionInfoW、VerSetConditionMask、GetVersionExW、FindClose）。
- 件数 3 の 7 件のうち、確認できなかったのは SetCurrentDirectoryW の 1 件。
- 件数 2 は tsv で 31 件（rank 8 から 38）。確認できたのは 10 件（VerSetConditionMask を含む）。確認できなかったのは 21 件（上の一覧の「未確認」）。
- 仕様から応答を決められるもの: LocalFree（実装済み）。それ以外は「仕様から決まらない」か「保留」、または未確認。
- 実装の追加: LocalFree の 1 件（`fle_imports.c` と `.h` に追記、対応表に 1 件）。
