# プロセス・時刻 API の仕様確認（A1 と A4、および時刻の関連 4 件）

作業者（M1、サブエージェント）の記録。判断は `docs/decisions/0020-process-and-time.md`。実装は `lift/runtime/fle_proc.h/.c`、`fle_time.h/.c`。実行記録は `docs/investigations/proctime-run.md`。ページの取得は `lift/out-proctime/fetch_spec.py`、`fetch_spec2.py`、`fetch_spec3.py`（結果は `lift/out-proctime/spec/*.txt`。取得日 2026-10-20、learn.microsoft.com）。

書き方: **「ページ」** = 取得したページに書かれていること。**「仮定」「推測」** = ページに書かれておらず、実装が選んだこと。

## 1. 担当 API の一覧と件数

閉包（896 seed）の未対応 thunk の件数は `lift/out-usage/unhandled-imports-usage-v3.tsv`（`unsupported_jmp_count`）と `lift/out-remaining/no-answer-v3.tsv` で確かめた。A1 は 13 API・13 thunk、A4 は 9 API・9 thunk（`remaining-unsupported.md` §5.1 の A1、A4 と一致）。

| 単位 | API | 閉包の thunk | 呼び出し箇所（call_sites） |
|---|---|---|---|
| A1 | GetCurrentProcess | 1 | 6 |
| A1 | GetCurrentProcessId | 1 | 4 |
| A1 | GetCurrentThread | 1 | 1 |
| A1 | SetLastError | 1 | 2 |
| A1 | SetErrorMode | 1 | 3 |
| A1 | SetThreadErrorMode | 1 | 2 |
| A1 | SetPriorityClass | 1 | 1 |
| A1 | SetThreadPriority | 1 | 1 |
| A1 | SwitchToThread | 1 | 1 |
| A1 | OutputDebugStringW | 1 | 3 |
| A1 | ExitProcess | 1 | 1（0x412a0d） |
| A1 | TerminateProcess | 1 | 1 |
| A1 | RaiseException | 1 | 3（0x41da25、0x41db65、0x41dc6a。ecx = 0xc06d0057） |
| A4 | DosDateTimeToFileTime | 1 | 1 |
| A4 | FileTimeToDosDateTime | 1 | 1 |
| A4 | FileTimeToLocalFileTime | 1 | 2 |
| A4 | FileTimeToSystemTime | 1 | 4 |
| A4 | LocalFileTimeToFileTime | 1 | 1 |
| A4 | SystemTimeToTzSpecificLocalTime | 1 | 1 |
| A4 | GetTimeZoneInformation | 1 | 1 |
| A4 | GetLocalTime | 1 | 4 |
| A4 | GetDateFormatW | 1 | 2（0x4431bd、0x4433c2。lpFormat は非 NULL） |

追加の 4 API（依頼の A4 の例に挙がっていて、閉包の thunk は 0 件）: `GetSystemTime`、`GetSystemTimeAsFileTime`、`GetTickCount64`、`SystemTimeToFileTime`。実測: 4 つとも IAT の import 名にはない（`SystemTimeToFileTime` は `.idata` の名前表にあるが、IAT slot の使用は 0 件。`GetTickCount64` と `GetSystemTimes` は `.text` 内の名前文字列として現れる＝GetProcAddress に渡される名前の可能性。**推測**）。`GetSystemTime` と `GetSystemTimeAsFileTime` はバイナリのどこにも文字列がない。応答表に入れる理由は、GetProcAddress が引かれたときに、時刻の源が 1 つで済むようにするため（決定 0020）。

`GetCommandLineW` は A1 に含まれない（`fle_env.c` で既に対応）。`GetCurrentThreadId` も既存（`fle_imports.c`）。

## 2. 取得したページと確認状況

取得できたもの（`lift/out-proctime/spec/`）:

- processthreadsapi: GetCurrentProcess、GetCurrentProcessId、GetCurrentThread、SetPriorityClass、SetThreadPriority、SwitchToThread、ExitProcess、TerminateProcess
- errhandlingapi: SetLastError、SetErrorMode、SetThreadErrorMode、RaiseException、GetLastError
- debugapi: OutputDebugStringW
- timezoneapi: FileTimeToSystemTime、SystemTimeToFileTime、SystemTimeToTzSpecificLocalTime、GetTimeZoneInformation、構造体 TIME_ZONE_INFORMATION
- fileapi: FileTimeToLocalFileTime、LocalFileTimeToFileTime
- winbase: DosDateTimeToFileTime、FileTimeToDosDateTime（fileapi の名前空間は 404。winbase で取得）
- sysinfoapi: GetLocalTime、GetSystemTime、GetSystemTimeAsFileTime、GetTickCount、GetTickCount64
- datetimeapi: GetDateFormatW、GetDateFormatEx
- minwinbase: SYSTEMTIME、FILETIME
- 概説: File Times、MS-DOS Date and Time、Time Functions、Scheduling Priorities、Terminating a Process、Process and Thread Functions、Structured Exception Handling、Process Security and Access Rights、System Error Codes (0-499)、Day, Month, Year, and Era Format Pictures、EXCEPTION_RECORD

**取得できなかったもの**（確認できていない）:

- RaiseException の winnt 側のページ（404）。errhandlingapi 側のページで足りた。
- `EXCEPTION_MAXIMUM_PARAMETERS` の値。EXCEPTION_RECORD のページは名前だけで値を書かない。そのため RaiseException の引数個数の上限検査は実装しない（ExitProcess と同じく止めるため不要）。
- GetCurrentThread の値（下記）、GetCurrentProcessId の値、GetDateFormatW の en-US の既定の picture と日・月の名前、時刻の既定の起点。いずれもページにない。

## 3. API ごとの規則（ページ）と実装の選択

### 3.1 A1

| API | ページ | 実装 |
|---|---|---|
| GetCurrentProcess | 擬似ハンドル。「現在は (HANDLE)-1」。CloseHandle は効果なし。 | -1 を返す（ページ） |
| GetCurrentProcessId | DWORD。プロセスが終わるまで一意。値は書かれていない。 | 0x1000（**環境の設定**。ホストの pid は使わない） |
| GetCurrentThread | 擬似ハンドル。「特別な定数」とだけあり、**値は書かれていない**。 | -2 を返す（**仮定**。一般に知られた値で、ページでは確認していない。ヘッダに明記） |
| SetLastError | VOID。呼び出しスレッドの最終エラーを設定。32 ビット値。ビット 29 はアプリ定義用。 | 下位 32 ビットを `fle_set_last_error` に渡す（ページ）。R[0] は触らない |
| SetErrorMode | 値 0、SEM_FAILCRITICALERRORS 1、SEM_NOGPFAULTERRORBOX 2、SEM_NOALIGNMENTFAULTEXCEPT 4、SEM_NOOPENFILEERRORBOX 0x8000。戻り値は直前のモード。SEM_NOALIGNMENTFAULTEXCEPT は一度設定すると、解除の試みは無視される。 | 直前のモードを返す。4 の保持を実装（ページ）。初期値 0 は「システム既定」（ページの 0 の説明から）。4 つの値以外のビットは止める（ページが書かない） |
| SetThreadErrorMode | BOOL。値 0、1、2、0x8000（**4 は一覧にない**）。lpOldMode は NULL 可で、スレッドの直前のモードを受ける。「スレッドはプロセスのエラーモードを継承する」。 | 4 や他のビットは止める。直前のモードは、スレッドが未設定の間はプロセスのモード（上の一文の**読み**） |
| SetPriorityClass | BOOL。クラス: ABOVE_NORMAL 0x8000、BELOW_NORMAL 0x4000、HIGH 0x80、IDLE 0x40、NORMAL 0x20、REALTIME 0x100。PROCESS_MODE_BACKGROUND_BEGIN 0x100000 / END 0x200000 は現在のプロセスのみ。すでに background のとき BEGIN は失敗、background でないとき END は失敗。エラー番号は System Error Codes の ERROR_PROCESS_MODE_ALREADY_BACKGROUND 402、ERROR_PROCESS_MODE_NOT_BACKGROUND 403（例のコードが名前を使い、System Error Codes ページが番号を与える）。 | 実装。擬似ハンドル以外は止める（OpenProcess がないため他のハンドルが存在しない）。一覧にない値は止める |
| SetThreadPriority | BOOL。値: IDLE -15、LOWEST -2、BELOW_NORMAL -1、NORMAL 0、ABOVE_NORMAL 1、HIGHEST 2、TIME_CRITICAL 15。-7〜-3 と 3〜6 は REALTIME のときだけ。THREAD_MODE_BACKGROUND_BEGIN 0x10000 / END 0x20000 は現在のスレッドのみ。ERROR_THREAD_MODE_ALREADY_BACKGROUND 400、ERROR_THREAD_MODE_NOT_BACKGROUND 401。 | 実装。擬似ハンドル以外は止める。REALTIME 専用値は、クラスが REALTIME でなければ止める（ページは「失敗する」とも書いていない。止めるのは安全側の選択） |
| SwitchToThread | BOOL。他のスレッドへ切り替えたら非 0、他に実行可能なスレッドがなければ 0。 | 常に 0（スレッドが 1 つ）。100 万回を超えたら止める（**実装の選択**。存在しないスレッドを待つループが回り続けるのを防ぐ） |
| OutputDebugStringW | VOID。「デバッガがなく、システムデバッガも動いていなければ、何もしない」。引数は optional。 | 何もしない。引数は読まない |
| ExitProcess | VOID。呼び出しプロセスと全スレッドを終了。DLL に DLL_PROCESS_DETACH が通知される。戻らない。 | 止める（下記 §3.3、決定 0020） |
| TerminateProcess | BOOL。自分自身を終了するときは戻らない。 | 止める |
| RaiseException | VOID。ハンドラが見つからなければ「既定の処理は ExitProcess の呼び出し」。 | 止める（SEH はモデル化していない） |

### 3.2 A4

| API | ページ | 実装 |
|---|---|---|
| FileTimeToSystemTime | FILETIME が 0x8000000000000000 未満でなければ失敗。 | その規則。結果の年が 30827 を超える場合は**止める**（下の注 1） |
| SystemTimeToFileTime | wDayOfWeek は無視。「False は無効な日付を表すことがある」（うるう年が例）。 | 実在する日付と、各メンバーの範囲（SYSTEMTIME ページ: 年 1601〜30827、月 1〜12、日 1〜31、時 0〜23、分 0〜59、秒 0〜59、ミリ秒 0〜999）で検査。失敗は 0（エラー番号は**仮定** 87） |
| FileTimeToLocalFileTime / LocalFileTimeToFileTime | 出力は入力と同じにできない。現在のタイムゾーンと DST の設定を使う。 | 同じなら止める。バイアス 0（UTC）なので値は同じ。範囲外は 0（仮定 87） |
| SystemTimeToTzSpecificLocalTime | NULL は現在のタイムゾーン。DST を考慮する。**Remarks に「年が違う場合に誤ることがある」の注意**。 | NULL は UTC。与えられた構造体は、遷移日が両方とも未指定（wMonth が 0）のときだけ Bias を使う。遷移日があれば**止める**（実装していない）。計算は FILETIME 上で行い、1601 年より前になれば失敗（仮定 87） |
| GetTimeZoneInformation | 戻り値: UNKNOWN 0（DST を使わない）、STANDARD 1、DAYLIGHT 2。UTC = ローカル時刻 + bias。StandardDate、DaylightDate の wMonth が 0 なら DST なし。構造体は 172 バイト（4 + 64 + 16 + 4 + 64 + 16 + 4。**ページの型からの計算**）。 | bias 0、名前 "UTC"、DaylightName 空、日付 0、戻り値 0 |
| GetLocalTime | VOID。現在のローカル日時。NULL は書かれていない。 | 仮想時計 + バイアス。NULL は止める |
| GetSystemTime | VOID。UTC。NULL は「アクセス違反」（ページ）。 | NULL は止める |
| GetSystemTimeAsFileTime | VOID。UTC の FILETIME。 | 同上 |
| GetTickCount64 | ミリ秒、64 ビット（49.7 日で折り返さない）。 | `fle_sys.c` と同じ仮想時計（ページに「GetTickCount と同じ時計」とはない。整合のための選択） |
| DosDateTimeToFileTime | 日付: ビット 0-4 日、5-8 月、9-15 年 - 1980。時刻: 0-4 秒/2、5-10 分、11-15 時。結果は UTC ベース（MS-DOS Date and Time ページ）。 | その配置で復号。実在しない日付・時刻は失敗（ページは失敗の条件を挙げない。**推測**。仮定 87） |
| FileTimeToDosDateTime | 1/1/1980〜12/31/2107 のみ表せる。範囲外は失敗。 | その範囲。奇数秒は切り捨て（ページに丸めの規則がない。**仮定**） |
| GetDateFormatW | 下記 §3.4 | 一部のみ |

注 1: SYSTEMTIME ページは wYear の有効値を 1601〜30827 とする。FILETIME が 0x8000000000000000 未満でも、年が 30828 以上になる値がある。FileTimeToSystemTime のページはこの場合の結果を書かない。呼ばれたら止める（黙って値を作らない）。ゲストに現れる値ではない。

### 3.3 ExitProcess、TerminateProcess、RaiseException の扱い

- ページ: ExitProcess は「全スレッドを終了し、DLL に PROCESS_DETACH を通知し、戻らない」。Terminating a Process は終了コードの扱いを書く。RaiseException は、ハンドラがなければ既定で ExitProcess。
- 閉包の実測: ExitProcess の呼び出しは 1 か所（0x412a0d。直前に `mov ecx,[0x13a5000]`＝終了コードの読み出し）。RaiseException は 3 か所（0x41da25、0x41db65、0x41dc6a。`mov ecx,0xc06d0057`）。**0xC06D0057 の意味はページでは確認していない**（`C06D0057` は Delay-load の「プロシージャが見つからない」例外として知られる値だが、このページ群で取得していないので**推測**）。
- 実行がここに来る意味は「プログラムが終了を要求した」こと。戻ると実行が続いてしまい、成功の終了にすると要求の事実を隠す。そのため、3 つとも `fle_stop` で止め、API 名と引数（終了コード、例外コードとフラグ、引数の数）を出す（決定 0020）。

### 3.4 GetDateFormatW

- ページ（GetDateFormatW、GetDateFormatEx、Format Pictures）: picture は d、dd、M、MM、yy、yyyy、yyyyy、g/gg、ddd/dddd、MMM/MMMM。引用符で囲んだ文字はそのまま。`''` は 1 つの引用符。lpFormat が NULL のとき flags が使われ、既定は DATE_SHORTDATE。lpDate の時刻メンバーは無視。日付メンバーが範囲外なら ERROR_INVALID_PARAMETER。曜日が違うときは正しい値に直して成功する。cchDate 0 は必要な大きさ（NUL を含む）を返す。小さければ ERROR_INSUFFICIENT_BUFFER。
- ページが定めない: 日と月の名前（ddd、dddd、MMM、MMMM）、era の文字列、en-US の短い日付の picture、y と yyy の扱い（Format Pictures は「y = 年の最後の桁」、GetDateFormatEx の Remarks は「y は yy として扱う」と書く。**2 つのページが一致しない**）。
- 実装したもの: lpFormat が非 NULL、flags 0、ロケールが en-US 系（0x0409、0x0400、0x0800、0x007F）で、picture が d、dd、M、MM、yy、yyyy、yyyyy、引用符、記号の組。
- ゲストの 2 か所の呼び出し（objdump とバイナリから読んだ。実行はしていない）:
  - 0x4431bd: ecx = GetThreadLocale の戻り値（0x423730 は GetThreadLocale の thunk）、**edx = 4**、r8 = SYSTEMTIME（[rbp+0x258] に wYear、wMonth、wDay だけを書く）、r9 = lpFormat（直前に 0x4432f4 の文字列を文字列オブジェクトへ代入したもの。0x4432f4 のバイトは `g g \0` ＝ **"gg"**）、cch = 0x200。
  - 0x4433c2: 同じ形で、edx = 4、lpFormat は 0x443468（"yy"）または 0x44347c（"yyyy"）（0x443360 の比較で選ぶ）。
  - dwFlags = 4 は、GetDateFormatEx のページの表では DATE_USE_ALT_CALENDAR（代替暦を使う）。ページの flags の説明は「lpFormat が NULL のとき使える」とあり、picture と組のときの扱いを書かない。**この組は実装していない**（flags が 0 でないので止める）。
  - 結論: ゲストの GetDateFormatW は、flags 4 で "gg"（era）と "yy"/"yyyy" を求める形に読める（和暦などの代替暦を引く用途か。**推測**）。実行がここに来れば、`fle_stop` で止まる。era 文字列（CAL_SERASTRING）の値はページにない。
- 実装しなかったもの（呼ばれたら止める）: 名前系、g/gg、y、yyy、flags 付き、lpFormat NULL、他のロケール。

## 4. 時刻の源（決定 0020 の要約）

- 源は 1 つ: `fle_sys.c` の仮想時計（100 ns の tick。決定 0017）。現在時刻（UTC の FILETIME）は、「tick 0 の瞬間」＋ tick。tick 0 の瞬間を 2020-12-31T23:00:00Z（FILETIME 132539292000000000）とし、仮想時計の開始値（システム開始から 1 時間）で、最初の読み出しが 2021-01-01T00:00:00Z（`fle_vfs.h` の FLE_VFS_CLOCK_START と同じ）になる。**この日時は環境の設定で、仕様ではない**。
- 読み出しは `fle_imp_QueryPerformanceCounter` を呼ぶ形で行う。したがって、時刻の読み出しも 1 回につき 10 tick（1 µs）進め、Sleep の分だけ進む。ホストの時計は使わない。
- タイムゾーン: UTC（bias 0、DST なし）。環境の設定で、仕様ではない。

## 5. 確認できなかったこと（まとめ）

- GetCurrentThread の値（-2 は仮定）。GetCurrentProcessId の値（0x1000 は設定）。
- 0xC06D0057 の意味（推測）。
- GetDateFormatW の名前・era・短い日付の picture。
- エラー番号: FILETIME や SYSTEMTIME の変換失敗、DOS 日時の失敗に使う 87 は、ページが書かないので選択。
- Windows 実機との照合は、ホストが Linux のためできていない。
