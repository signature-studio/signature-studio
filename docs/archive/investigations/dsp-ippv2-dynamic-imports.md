# dsp_ippv2_x64.dll が動的に引くモジュールと API（D4 の事実集め）

実行日: 2026-10-23。HEAD e410484。**調査のみ。この文書は何も決めない**（推奨は §6.3 に理由つきで書くが、決めるのは PM）。`lift/` の既存ファイル、runtime、生成器、テストは変更していない。背景は `two-module-design.md` §5.4、§6 D4 と `dsp-ippv2-lift-feasibility.md` §6。作業物は `lift/out-d4/`（gitignore 対象）。

## 0. 凡例

- **実測** = 今回コマンドを実行して得た値（出力は `lift/out-d4/` にある）。
- **読み** = 逆アセンブル（objdump、capstone）とデータの読み。実行して確かめていない。
- **仮説** = 実測や読みから言えそうだが、確認していない。
- **未確認** = 確認できていない。
- **一次に無い** = Microsoft Learn の取得ページに書かれていない。
- 外部仕様は Microsoft Learn だけ（取得は 2026-10-23。URL は §4、§5）。**非公式資料（Geoff Chappell、ReactOS、Wine）は使っていない。**
- 「RVA」は PE のイメージ内オフセット。ファイル上の VA は 0x180000000 + RVA、リベース 0x2000000 の VA は 0x2000000 + RVA。入力は `lift/out-ext/dsp_ippv2_x64.dll`（39,942,656 B）。
- Learn の「api_location」はページ先頭のメタデータ（front matter）の欄で、表示される Requirements 表の「DLL」欄とは別物。両方を区別して書く。

## 1. 要約

| 項目 | 結果 | 区分 |
|---|---|---|
| `LoadLibraryExW` の呼び出し点 | 4 か所（RVA 0x17d080d、0x17d0845、0x17d739f、0x17d73f3）。**どれも名前は動的（表の添字で選ぶ）が、名前の表は 2 つだけで中身を全部読めた**。`LoadLibraryW`/`LoadLibraryA` は DLL の import に無い | 読み（import 72 名はファイルから実測） |
| `GetModuleHandleW` | 1 か所（0x17d19b3）。**引数は NULL**（直前が `xor ecx,ecx`） | 読み |
| `GetModuleHandleExW` | 1 か所（0x17d1add）。flags 0、名前は `L"mscoree.dll"`。**exit 経路**で、FLEngine は .NET イメージでないので通らない | 読み＋実測（FLEngine の DataDirectory[14] = 0） |
| `GetProcAddress` | 3 か所。うち 2 か所は CRT の遅延バインドの関数（`try_get_function` 形）で、ここから渡る名前は **9 種**（Fls 4、`InitializeCriticalSectionEx`、`AppPolicyGetProcessTerminationMethod`、`AreFileApisANSI`、`LCMapStringEx`、`LocaleNameToLCID`）。残る 1 か所は `"CorExitProcess"`（mscoree.dll、exit 経路） | 読み |
| DLL が `LoadLibraryExW` に渡しうるモジュール名 | `api-ms-win-core-fibers-l1-1-1`、`api-ms-win-core-synch-l1-2-0`、`api-ms-win-core-localization-l1-2-1`、`api-ms-win-appmodel-runtime-l1-1-2`、`kernel32` の **5 名**。実現性調査 §6.1 が列挙した 19 名のうち、ucrt の表 B（19 名）に載るだけで呼び出し点のないものが 15 名ある | 読み |
| NULL を返したときの DLL の分岐 | **どの呼び出し点も NULL を許す**（代替関数、固定値、または既存の別経路）。DLL 自身は止まらない | 読み＋一部実測 |
| ラッパーの分岐（重要） | `LoadLibraryExW` が成功し `GetProcAddress` が NULL のとき、**同じ名前の `kernel32` は試さずに失敗を記録する**。`LoadLibraryExW` が NULL のときだけ、リストの次（`kernel32`）へ進む | 読み＋実測（gdb） |
| 現行 runtime（HEAD）の応答 | 実測: `api-ms-win-core-synch-l1-2-0.dll` で停止（exit 5） | 実測 |
| 一次資料で言えること | 9 API は Learn の Requirements 表で DLL = Kernel32.dll（`AppPolicyGetProcessTerminationMethod` だけ DLL = `Api-ms-win-appmodel-runtime-l1-1-2.dll`）。API セット名と API の対応は、**各 API ページの api_location に載る範囲だけ**言える。実機 Windows 10 でその API セット名が `LoadLibrary` で解決できるか（どの名前がスキーマにあるか）は**一次に無い** | 実測（Learn 取得） |

## 2. LoadLibrary / GetModuleHandle 系の呼び出し点（項目 1）

### 2.1 方法

1. import 表（kernel32 の 72 名）から、対象 API の IAT スロットの RVA を取った: `GetModuleHandleW` 0x1ebd0f0、`FreeLibrary` 0x1ebd170、`GetProcAddress` 0x1ebd178、`LoadLibraryExW` 0x1ebd180、`GetModuleHandleExW` 0x1ebd190、`GetModuleFileNameW` 0x1ebd198（`out-ipp/imports-scan.txt` と同じ）。
2. `.text` と `IPPCODE` の全バイトで、rip 相対 disp32 がこのスロットに解決される位置を 4 通りの整列で走査した（`d4_scan.py` → `scan-iat-sites.tsv`）。命令境界の判定に線形逆アセンブルを使わないので、データ混在の影響を受けない。結果 16 件。
3. 別に `objdump -d` 全体（`dis-all.txt`、255 MB）で `call [rip+slot]` を grep した（`xref-raw.txt`）。14 件で、走査の 16 件から次の 2 件を除いたものに一致する。**除いた 2 件は偽陽性**: IPPCODE 0x1dbd07e は `cmp rsi,0x1000`（`48 81 fe 00 10 00 00`）のバイトの途中、`.text` 0x33acd5 は `movsd xmm0,[rip+…]` のバイトの途中。
4. `LoadLibraryW`、`LoadLibraryA`、`LoadLibraryExA`、`GetModuleHandleA`、`GetModuleHandleExA` は **import に無い**（72 名の一覧。実測）。
5. 呼び出し点の周辺を `d4_rd.py`（データ読み）、`d4_wrappers.py`（引数の復号）、`d4_callers.py`（`E8`/`E9` rel32 の呼び元の走査）で読んだ。データ中の 8 バイトポインタ（`.data`、`.rdata`、`IPPDATA`、`_RDATA`）が関数 0x17d0788、0x17d72fc を指していないことも走査した（0 件。つまり関数ポインタ経由の呼び出しは無い）。

### 2.2 呼び出し点の一覧（実測の走査＋読み）

| # | RVA（call の先頭） | API | 引数（読み） | 名前 | 所属する関数 |
|---|---|---|---|---|---|
| 1 | 0x17d080d | `LoadLibraryExW` | `lpLibFileName` = 表 A[i]、`hFile` = NULL、`dwFlags` = **0x800**（`LOAD_LIBRARY_SEARCH_SYSTEM32`） | **動的**（表 A、§2.3） | 0x17d0788（以下「ラッパー A」） |
| 2 | 0x17d0845 | `LoadLibraryExW` | 同じ名前、`hFile` = NULL、`dwFlags` = **0**（再試行） | 動的（同上） | ラッパー A |
| 3 | 0x17d739f | `LoadLibraryExW` | 表 B[i]、NULL、**0x800** | 動的（表 B、§2.3） | 0x17d72fc（以下「ラッパー B」） |
| 4 | 0x17d73f3 | `LoadLibraryExW` | 同じ名前、NULL、**0** | 動的（同上） | ラッパー B |
| 5 | 0x17d19b3 | `GetModuleHandleW` | `lpModuleName` = **NULL**（`xor ecx,ecx`） | NULL | 0x17d198c（プロセス終了の中心関数） |
| 6 | 0x17d1add | `GetModuleHandleExW` | `dwFlags` = 0（`xor ecx,ecx`）、`lpModuleName` = RVA 0x1ec82e8、`phModule` = スタック | **`L"mscoree.dll"`**（UTF-16 の文字列リテラル） | 0x17d1ab8（`CorExitProcess` の探索） |
| 7 | 0x17d088b | `FreeLibrary` | 直前に `LoadLibraryExW` が返したハンドル | — | ラッパー A（§2.4） |
| 8 | 0x17d747b | `FreeLibrary` | 同上 | — | ラッパー B |
| 9 | 0x17d1b1a | `FreeLibrary` | 6 が返したハンドル（非 NULL のときだけ） | — | 0x17d1ab8 |
| 10 | 0x17d7787 | `FreeLibrary` | 表 0x2561e90…0x2561f28（ucrt のモジュール キャッシュ）の各スロット。値が 0 と -1 以外のとき | — | 0x17d7768（`test cl,cl; jne` で飛ばす形の後始末。呼ばれ方は読み切っていない） |
| (参考) | 0x17d637b | `GetModuleFileNameW` | `hModule` = **NULL**（唯一の呼び元 0x17d1dc9 が `xor ecx,ecx`）、バッファ 0x2561650、`nSize` = 0x104 | — | 0x17d6340（0x17d1d74 から。§2.5） |

「動的」と書いたものも、名前は 2 つの定数の表（§2.3）から選ばれ、表の各行の使われ方は §3 で復号した。**計算で組み立てる名前（文字列連結など）は見つからなかった**（読み）。

### 2.3 名前の表（実測のデータ。`wrappers-decoded.txt`）

表 A（RVA 0x1ec7f98、vcruntime 側。3 名の UTF-16 ポインタ表）:

| 添字 | 名前 |
|---|---|
| 0 | `api-ms-win-core-fibers-l1-1-1` |
| 1 | `api-ms-win-core-synch-l1-2-0` |
| 2 | `kernel32` |

表 B（RVA 0x1ec90c0、ucrt 側。19 名）:

| 添字 | 名前 | 添字 | 名前 |
|---|---|---|---|
| 0 | `api-ms-win-core-datetime-l1-1-1` | 10 | `api-ms-win-rtcore-ntuser-window-l1-1-0` |
| 1 | `api-ms-win-core-file-l1-2-2` | 11 | `api-ms-win-security-systemfunctions-l1-1-0` |
| 2 | `api-ms-win-core-localization-l1-2-1` | 12 | `ext-ms-win-ntuser-dialogbox-l1-1-0` |
| 3 | `api-ms-win-core-localization-obsolete-l1-2-0` | 13 | `ext-ms-win-ntuser-windowstation-l1-1-0` |
| 4 | `api-ms-win-core-processthreads-l1-1-2` | 14 | `advapi32` |
| 5 | `api-ms-win-core-string-l1-1-0` | 15 | `kernel32` |
| 6 | `api-ms-win-core-synch-l1-2-0` | 16 | `ntdll` |
| 7 | `api-ms-win-core-sysinfo-l1-2-1` | 17 | `api-ms-win-appmodel-runtime-l1-1-2` |
| 8 | `api-ms-win-core-winrt-l1-1-0` | 18 | `user32` |
| 9 | `api-ms-win-core-xstate-l2-1-0` | | |

どの名前も拡張子なし（`.dll` なし）で渡る（UTF-16 の文字列そのもの）。runtime の `LoadLibraryExW` は拡張子のない名前に `.dll` を足して表を引く（`fle_load.c` の `normalize`。実測: 停止メッセージが `"api-ms-win-core-synch-l1-2-0.dll"`）。

**実際に使う添字は表 B の 4 行（synch 6、localization 2、kernel32 15、appmodel 17）と表 A の 3 行**（§3.1。`api-ms-win-core-synch-l1-2-0` は両方の表に載る）。DLL が `LoadLibraryExW` に渡しうる名前は、重複を除いて **5 名**（fibers-l1-1-1、synch-l1-2-0、localization-l1-2-1、appmodel-runtime-l1-1-2、kernel32）。表 B に載るだけで、ラッパー B の 5 つの呼び出し（§3.1）のどれも添字に使わない名前が **15 名**ある（表 B の 19 名 − 使う 4 名）。実現性調査 §6.1 が列挙した名前のうち、この 15 名は実行されても `LoadLibraryExW` に渡らない（読み。表 B の添字を呼び元が計算で作る形は見つからなかった: 添字は 5 つの呼び出し点の即値 0x0e、0x10、0x12、0x18 と 0x00 だけ）。

### 2.4 ラッパーのプロトコル（読み。`dis-d0600.txt`）

ラッパー A（RVA 0x17d0788）の引数は rcx = 関数スロットの添字、rdx = API 名（ASCII）、r8/r9 = モジュール添字（dword）のリストの [begin, end)。ラッパー B（0x17d72fc）は同じ形で、関数スロットは暗号化（cookie `[0x2543588]` と `ror`）して持つ点だけ違う。

1. 関数スロットのキャッシュを見る（A は `[0x2561608 + idx*8]`、B は `[0x2561f30 + idx*8]`）。非 0 かつ -1 以外ならそれを返す（B は復号）。-1 は「前に失敗」で 0 を返す。
2. リストのモジュール添字を順に: モジュール ハンドルのキャッシュ（A は `[0x25615f0 + i*8]`、B は `[0x2561e90 + i*8]`）が 0 なら `LoadLibraryExW(名前, NULL, 0x800)`。
3. 結果が NULL のとき、`GetLastError()` が **0x57（ERROR_INVALID_PARAMETER）** で、かつ名前が `"api-ms-"`（B は `"ext-ms-"` も）で**始まらない**ときだけ、`LoadLibraryExW(名前, NULL, 0)` を再試行する（`0x17d0834` の比較関数 0x17d2660 は `wcsncmp` 形で、等しい（eax = 0）と `je` で失敗側へ飛ぶ。読み。実測: §7 の `err87` で、`kernel32` だけが `dwFlags=0` で再試行され、`api-ms-*` は再試行されなかった）。それでも NULL ならこのモジュールを -1（失敗）としてキャッシュし、**リストの次のモジュールへ進む**。エラー値が 0x57 以外（例: 126）なら再試行せずにそのまま失敗側へ進む。
4. 結果が非 NULL ならハンドルをキャッシュに入れる（xchg）。以前のキャッシュが非 0 なら（他のスレッドが先に入れた形）、自分が得たハンドルを `FreeLibrary`（上の 7、8）。
5. `GetProcAddress(ハンドル, 名前)`。**NULL なら、関数スロットを -1 にして 0 を返す。リストの次のモジュールは試さない**（`je 0x17d08af`（A）、`je 0x17d7420`（B）。実測でも確認: §3.3）。非 NULL ならスロットに入れて返す。
6. リストを使い切ったら関数スロットを -1 にして 0 を返す。

**帰結**（読み＋実測）: 「api-ms-* の `LoadLibraryExW` が成功し、`GetProcAddress` だけ NULL」にすると、`kernel32` は試されず、その API は「無い」として代替に進む。「api-ms-* の `LoadLibraryExW` が NULL」にすると `kernel32` で `GetProcAddress` が呼ばれる。

### 2.5 `GetModuleHandleW(NULL)` と `GetModuleFileNameW(NULL)`（参考）

- 0x17d198c（呼び出し点 5）は、`GetModuleHandleW(NULL)` の戻り値を PE として検査する: 先頭が `MZ`（0x5a4d）、`e_lfanew`、`PE\0\0`、Optional Header の Magic が 0x20b、`NumberOfRvaAndSizes`（+0x84）が 14 より大きい、`DataDirectory[14]`（+0xf8、COM ディスクリプタ）が非 0 のときだけ 0x17d1ab8（`CorExitProcess` の探索）へ進む。**FLEngine_x64.dll の DataDirectory[14] は (0, 0)**（`NumberOfRvaAndSizes` = 16。`lift/` のスクリプトで `binaries/win64/FLEngine_x64.dll` のヘッダから実測）。runtime の `GetModuleHandleW(NULL)` は `FLE_EXE_HANDLE`（0x400000、FLEngine のイメージ）を返す（`fle_modules.c`、決定 0015）ので、**この検査は成立せず 0x17d1ab8 には進まない**（読み）。
- 0x17d1ab8 の直接の呼び元はもう 1 か所ある（0x17d1a78。0x17d1a54 の終了処理）。0x17d1a54 は `TerminateProcess` の後に `ExitProcess` を呼ぶ関数で、`DllMain` の ATTACH の直線上には無い（呼び元の鎖は 0x17d1b30/0x17d1b40 → 0x17d198c で、0x17cb466 の `__cexit` 形と 0x17d25f0、0x17d7bbf から。読み）。したがって `GetModuleHandleExW(L"mscoree.dll")` は **プロセス終了の経路**でだけ呼ばれる（読み）。
- 0x17d637b の `GetModuleFileNameW(NULL, buf, 0x104)` は、**ATTACH の直線上にある**（読み）: 0x17ca9d8 → 0x17cb354 → 0x17d1d74（引数 ecx = 1）→ 0x17d6340。`GetModuleFileNameW` は runtime に行がある（`fle_vfs.c`、仮想パス）。これは D4（LoadLibrary の名前）の対象ではないが、ATTACH で呼ばれる事実として書く。

## 3. GetProcAddress の呼び出し点と渡す API 名（項目 2）

### 3.1 呼び出し点（実測の走査＋読み）

| # | RVA | 呼び出す関数 | hModule の出どころ | 名前 |
|---|---|---|---|---|
| G1 | 0x17d0897 | ラッパー A（0x17d0788）の中 | 同じラッパーの `LoadLibraryExW`（呼び出し点 1 または 2）の戻り（キャッシュ経由） | 呼び元が rdx に渡す ASCII 文字列（下の A1〜A5） |
| G2 | 0x17d7489 | ラッパー B（0x17d72fc）の中 | 同上（呼び出し点 3 または 4） | 呼び元が rdx に渡す ASCII 文字列（下の B1〜B5） |
| G3 | 0x17d1af5 | 0x17d1ab8 の中 | 呼び出し点 6（`GetModuleHandleExW(0, L"mscoree.dll", &h)`）が返した `h` | `"CorExitProcess"`（RVA 0x1ec8300） |

### 3.2 ラッパー A・B の呼び出し（引数を復号した。`wrappers-decoded.txt`）

ラッパー A の呼び元は 5 か所（0x17d08f8、0x17d0942、0x17d098a、0x17d09d9、0x17d0a35）、ラッパー B の呼び元は 5 か所（0x17d74df、0x17d752f、0x17d75a9、0x17d7621、0x17d76fd）。**`E8` rel32 の走査で、これ以外の呼び元は見つからなかった**（callers-wrappers.txt。データ中の関数ポインタ経由の呼び出しも 0 件）。

| 記号 | ラッパー | 渡す API 名（ASCII） | モジュールのリスト（この順に試す） | 失敗時の代替（読み） |
|---|---|---|---|---|
| A1 | A、スロット 0 | `FlsAlloc` | `api-ms-win-core-fibers-l1-1-1`、`kernel32` | `TlsAlloc`（`jmp [IAT 0x1ebd150]`） |
| A2 | A、スロット 1 | `FlsFree` | 同上 | `TlsFree`（`jmp [IAT 0x1ebd168]`） |
| A3 | A、スロット 2 | `FlsGetValue` | 同上 | `TlsGetValue`（`jmp [IAT 0x1ebd158]`） |
| A4 | A、スロット 3 | `FlsSetValue` | 同上 | `TlsSetValue`（`call [IAT 0x1ebd160]`） |
| A5 | A、スロット 4 | `InitializeCriticalSectionEx` | `api-ms-win-core-synch-l1-2-0`、`kernel32` | `InitializeCriticalSectionAndSpinCount`（`call [IAT 0x1ebd148]`。第 3 引数 flags は捨てる） |
| B1 | B、スロット 0x18 | `AppPolicyGetProcessTerminationMethod` | `api-ms-win-appmodel-runtime-l1-1-2` **だけ** | 代替なし。`mov eax,0xc0000225`（STATUS_NOT_FOUND）を返す |
| B2 | B、スロット 0x00 | `AreFileApisANSI` | `kernel32` **だけ** | 代替なし。**1 を返す**（`mov eax,1`） |
| B3 | B、スロット 0x0e | `InitializeCriticalSectionEx` | `api-ms-win-core-synch-l1-2-0`、`kernel32` | `InitializeCriticalSectionAndSpinCount` |
| B4 | B、スロット 0x10 | `LCMapStringEx` | `api-ms-win-core-localization-l1-2-1`、`kernel32` | 0x17d76d4（`LocaleNameToLCID` を引く別のラッパー呼び出し B5）で LCID を求め、`LCMapStringW`（`call [IAT 0x1ebd038]`）を呼ぶ |
| B5 | B、スロット 0x12 | `LocaleNameToLCID` | `api-ms-win-core-localization-l1-2-1`、`kernel32` | 0x17d9f1c（DLL 内の関数。中身は**未精査**） |

つまり**この DLL が GetProcAddress に渡す API 名は 9 種**（Fls 4、`InitializeCriticalSectionEx`、`AppPolicyGetProcessTerminationMethod`、`AreFileApisANSI`、`LCMapStringEx`、`LocaleNameToLCID`）と、exit 経路の `CorExitProcess` の計 10 種（読み。`InitializeCriticalSectionEx` は A5 と B3 の 2 か所）。

### 3.3 「遅延バインド付きの関数ポインタ表」形か（読み＋実測）

**はい。MSVC の CRT の `try_get_function` 形**と読める根拠（事実だけ）:

- 名前（`FlsAlloc`…）、モジュールのリスト、関数スロットの表がデータで、同じ関数（A と B）が全 API を共通に扱う。
- 結果の関数ポインタは B では**暗号化して保持**する（cookie `[0x2543588]` を使った `ror` と `xor`。A は暗号化なし）。この 2 つのラッパー自身は `EncodePointer` を呼ばない（ラッパー A、B のコードに `call [IAT 0x1ebd128]` は無い。読み）。DLL の他の場所に `EncodePointer` の `call [IAT]` は 2 か所ある（決定 0077 §4 の一覧。呼び先の関数は今回読んでいない）。
- 結果のキャッシュは 0（未解決）、-1（失敗）、それ以外（有効）の 3 状態で、`xchg`（`lock` 暗黙）で書く。
- 呼び出し側は、結果が非 NULL なら `call [0x1ebd260]`（`__guard_dispatch_icall` 形のスロット。値は RVA 0x17de9d0 → `jmp [0x1ebd258]` → RVA 0x17de9b0 = `jmp rax`）で呼び、NULL なら代替を直接呼ぶ。
- これらの 0x1ebd248〜0x1ebd268 の 5 スロットは import ではなく、`.rdata` の定数（ファイル上の値 0x1817cb228 など）。

**引数が NULL か**: 呼び出し点のうち、(i) `lpLibFileName` はデータ表の要素（NULL にならない）、(ii) `hFile` は `xor edx,edx`（NULL）、(iii) `GetProcAddress` の名前は ASCII のデータへの `lea`。序数（上位 0）で引く呼び出しは無い（読み）。

## 4. 各 API 名の Learn 照合、現行応答表、NULL のときの分岐（項目 3）

取得した Learn のページ（2026-10-23。生のページと抜き出しは `lift/out-d4/learn/`）:

- FlsAlloc: https://learn.microsoft.com/en-us/windows/win32/api/fibersapi/nf-fibersapi-flsalloc （FlsFree、FlsGetValue、FlsSetValue も同じ `fibersapi` の `nf-fibersapi-<小文字名>`）
- InitializeCriticalSectionEx: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsectionex
- InitializeCriticalSectionAndSpinCount: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsectionandspincount
- AreFileApisANSI: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-arefileapisansi
- LCMapStringEx: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-lcmapstringex
- LocaleNameToLCID: https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-localenametolcid
- AppPolicyGetProcessTerminationMethod: https://learn.microsoft.com/en-us/windows/win32/api/appmodel/nf-appmodel-apppolicygetprocessterminationmethod
- CorExitProcess: https://learn.microsoft.com/en-us/dotnet/framework/unmanaged-api/hosting/corexitprocess-function
- GetModuleHandleExW: https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandleexw （GetProcAddress、FreeLibrary、LoadLibraryW、LoadLibraryExW、GetModuleHandleW も同じ `libloaderapi`）

### 4.1 表（Requirements は表示される Requirements 表の値。「api_location」はページ先頭のメタデータ欄）

| 名前 | Requirements の DLL | 最小の Windows（client / server） | 現行応答表（`TABLE`）の行 | NULL のときの DLL の分岐（読み） | ATTACH の直線上か |
|---|---|---|---|---|---|
| `FlsAlloc` | Kernel32.dll | Vista / Server 2003 | HEAD: **無い**。作業ツリー（他者の未コミット変更）: 行あり（`fle_imp_FlsAlloc`） | `TlsAlloc` に落ちる。DLL は **Fls と Tls の両方を静的 import している**（IAT に FlsAlloc/FlsFree/FlsGetValue/FlsSetValue/TlsAlloc/TlsGetValue/TlsSetValue がある。実測）。FlsAlloc の成否で CRT の TLS 方式が変わるだけ | **はい**（0x17cd59c。実測: 1 回目の `FlsAlloc` 照会） |
| `FlsFree` | Kernel32.dll | Vista / Server 2003 | HEAD 無し / 作業ツリー有り | `TlsFree`（HEAD の runtime に `fle_imp_TlsFree` は無い（`git grep`）。作業ツリーには実装が出ているが応答表の行ではない（`imports_table.rs` のコメント）） | 失敗・DETACH の経路（0x17cd5e4）。ATTACH の成功経路には無い（読み） |
| `FlsGetValue` | Kernel32.dll | Vista / Server 2003 | HEAD 無し / 作業ツリー有り | `TlsGetValue`（行あり） | 静的な到達範囲にある（0x17cd470、0x17cd4dc）。**実行されるかは未確認** |
| `FlsSetValue` | Kernel32.dll | Vista / Server 2003 | HEAD 無し / 作業ツリー有り | `TlsSetValue`（行あり） | **はい**（実測: 2 回目の照会） |
| `InitializeCriticalSectionEx` | Kernel32.dll | Vista / Server 2008 | **無い**（HEAD も作業ツリーも） | `InitializeCriticalSectionAndSpinCount`（HEAD: 行も runtime も無い → NULL 側に落ちた直後に `fle_unsupported(0x37d0a4f)`（実測）。作業ツリーは行あり） | **はい**（A5: 最初の照会。実測）。B3 は初期化関数表から（静的到達。実行は未確認） |
| `InitializeCriticalSectionAndSpinCount` | Kernel32.dll | XP / Server 2003 | HEAD **無い** / 作業ツリー有り | （代替の側。DLL は IAT でも静的に import する） | はい（上の代替先） |
| `AreFileApisANSI` | Kernel32.dll | XP / Server 2003 | **無い** | **1 を返す**（ANSI）。DLL は止まらない | 0x17d6340（ATTACH の直線上の関数）内の 0x17d63ed から。**その分岐に入るかは未確認** |
| `LCMapStringEx` | Kernel32.dll | Vista / Server 2008 | **無い**（`LCMapStringW` は HEAD に行あり） | `LocaleNameToLCID` を照会（B5）して `LCMapStringW` を呼ぶ | 静的な到達範囲（0x17d9a94 ← 0x17d9e20）。実行は未確認 |
| `LocaleNameToLCID` | Kernel32.dll | Vista / Server 2008 | **無い** | DLL 内の 0x17d9f1c を呼ぶ（**未精査**） | LCMapStringEx の代替の中 |
| `AppPolicyGetProcessTerminationMethod` | **Api-ms-win-appmodel-runtime-l1-1-2.dll**（Requirements の DLL 欄。kernel32 ではない） | **Requirements 表に最小 OS の行が無い**（一次に無い） | **無い** | `STATUS_NOT_FOUND`（0xc0000225）を返し、呼び元 0x17d5730 は結果が 1 でないとき 1 を返す。DLL は止まらない | **いいえ**（終了処理 0x17d1a54 の経路。読み） |
| `CorExitProcess` | `mscoree.dll`（api_location。Requirements に OS の行なし） | 一次に無い | **無い**（`GetModuleHandleExW` は HEAD に行無し。作業ツリー: 行あり、呼ばれたら abort） | 到達しない: `GetModuleHandleW(NULL)` が返すイメージの DataDirectory[14] が 0（FLEngine）で、0x17d198c が 0x17d1ab8 に進まない | **いいえ**（終了処理） |
| `GetModuleHandleExW`（`L"mscoree.dll"`） | Kernel32.dll | XP / Server 2003 | HEAD 無し / 作業ツリー有り（呼ばれたら abort） | 0 を返せば DLL は続行（`CorExitProcess` を飛ばす）。上の理由で通常は呼ばれない | いいえ |

「最小の Windows」は Requirements 表の値のまま。**Windows 10 22H2（決定 0014）にこれらがあることは、Vista/XP 以降という記述から導ける（決定 0055 と同じ導き方）が、Learn が「Windows 10 に存在する」と書いているわけではない**。`AppPolicyGetProcessTerminationMethod` はその導出もできない（最小 OS の記載なし）。

### 4.2 呼び出し側から見た NULL の意味（読み）

- A1〜A5、B3〜B5: **NULL を許す**（代替がある）。
- B1、B2: NULL を許す（固定値）。
- G3: `GetModuleHandleExW` が 0（失敗）でも許す。
- どの呼び出し点も `GetLastError` の値で分岐しない（例外は §2.4 の `LoadLibraryExW` 失敗直後の 0x57 の判定だけ）。

### 4.3 非 NULL を返したとき（読み＋実測）

非 NULL（擬似アドレス）を返すと、DLL は `call [0x1ebd260]`（`jmp rax`）で呼ぶ。生成 C では `fle_dispatch` になる。擬似 VA が `fle_dispatch_table` にあれば `fle_imp_*` が呼ばれる。実測（`all_ok_hit`、偽の値 0x7ff9dead0010 を返した）: `fle: indirect call to 0x7ff9dead0010 is not a known function entry`（表に無い値だから）。**応答表に行を足すと、`fle_imp_*` の実装が要る**。HEAD に `fle_imp_InitializeCriticalSectionEx`、`fle_imp_AreFileApisANSI`、`fle_imp_LCMapStringEx`、`fle_imp_LocaleNameToLCID`、`fle_imp_AppPolicyGetProcessTerminationMethod` の実装は無い（`git grep` 0 件）。

### 4.4 GetProcAddress の hModule と応答表の結び付き（読み。`fle_modules.c`）

`fle_imp_GetProcAddress` は、`hModule` から仮想モジュール名（例 `kernel32.dll`）を引き、**応答表の `dll` 欄がそのモジュール名と一致する行だけ**を探す（`strcmp(fle_resp_table[i].dll, mod)`）。したがって `api-ms-win-core-synch-l1-2-0.dll` を仮想モジュールに足しても、応答表に `dll = "api-ms-win-core-synch-l1-2-0.dll"` の行を足さない限り、`GetProcAddress` は常に NULL と 127 を返す。`kernel32.dll` の行は `api-ms-win-*` のハンドルでは引かれない。

## 5. api-ms-win-* 名と API の対応（項目 4）

### 5.1 Learn の API セットのページから言えること（一次）

- https://learn.microsoft.com/en-us/windows/win32/apiindex/windows-apisets （更新 2026-09、「All versions of Windows 10, and later」に適用）: API セット名は契約の名前でファイルではない。`LoadLibrary` に API セット名を渡せる。「The operation succeeds when the schema on the current device maps that contract to a usable host; there isn't necessarily an actual file with that name」「If the contract isn't mapped on the current device, a direct **LoadLibrary** fails」。`api-` と `ext-` の接頭辞に意味はない（「Don't infer availability from the prefix」）。名前の末尾 `.dll` は契約名の一部ではない。
- https://learn.microsoft.com/en-us/windows/win32/apiindex/api-set-loader-operation : 同趣旨。ローダーが `api-` / `ext-` で始まる名前を API セット ランタイムへ回す。レガシー モジュール名（`kernel32.dll` など）は、完全なデスクトップ版ではそのまま実モジュールに結び付く。
- https://learn.microsoft.com/en-us/windows/win32/apiindex/detect-api-set-availability : `LoadLibrary`/`GetProcAddress` は API セットの可用性の確認に信頼できない（逆転送のため）。`IsApiSetImplemented` を使う。
- 各 API ページの「どの API セットに属するか」: windows-apisets ページは「API が API セットに属するなら Requirements 表に API セット名と導入された Windows の版が載る」と書く。**今回の API ページのうち、Requirements 表に API セット名が載るのは `AppPolicyGetProcessTerminationMethod` の 1 つだけ**（DLL 欄 `Api-ms-win-appmodel-runtime-l1-1-2.dll`）。他の 8 API は Requirements の DLL 欄が Kernel32.dll で、API セット名は表に出ない。

### 5.2 ページ先頭メタデータ（api_location）に載る API セット名

メタデータ欄の意味（どの OS の版から存在するか）は、取得したページに説明が無い。以下は「その名前が欄に載る」という事実で、「その OS 版の `LoadLibrary` で解決できる」とは別。

| DLL が引く名前 | 載っている API（api_location に名前がある API ページ） | 載っていない API |
|---|---|---|
| `api-ms-win-core-fibers-l1-1-1` | FlsAlloc、FlsFree、FlsGetValue、FlsSetValue（l1-1-0、l1-1-1、l1-1-2 が載る） | — |
| `api-ms-win-core-synch-l1-2-0` | InitializeCriticalSectionEx、InitializeCriticalSectionAndSpinCount（l1-1-0、l1-2-0、l1-2-1 が載る） | — |
| `api-ms-win-core-localization-l1-2-1` | LCMapStringEx、LocaleNameToLCID（l1-1-0、l1-2-0、l1-2-1、l1-2-2、l1-2-3、l1-2-4 が載る） | — |
| `api-ms-win-appmodel-runtime-l1-1-2` | AppPolicyGetProcessTerminationMethod（l1-1-2〜l1-1-7 が載る。Requirements の DLL 欄にもある） | — |
| `kernel32` | 上の 9 API すべて（Requirements の DLL 欄が Kernel32.dll、または api_location に `Kernel32.dll`/`kernel32.dll`）。`AppPolicyGetProcessTerminationMethod` は api_location に `kernel32.dll` が載るが、DLL の B1 のモジュールのリストに kernel32 は無い | — |

補足: `AreFileApisANSI` の api_location には `API-MS-Win-Core-file-l1-2-2.dll`（表 B の添字 1）が載るが、DLL は `kernel32` だけを試す（B2）。

### 5.3 一次に無いもの

| 項目 | 状態 |
|---|---|
| Windows 10 22H2 で `api-ms-win-core-synch-l1-2-0`、`-fibers-l1-1-1`、`-localization-l1-2-1`、`-appmodel-runtime-l1-1-2` の各名前が `LoadLibrary` で解決できるか | **一次に無い**。windows-apisets は「スキーマが契約をホストに結び付けているときに成功する」と一般論を書くだけで、個別の名前の有無を書かない |
| 各名前が最初に導入された Windows の版 | **一次に無い**（`AppPolicy…` の Requirements は最小 OS の行を持たない。他のページは Kernel32.dll を前提にした版しか書かない） |
| `two-module-design.md` D4 の「`api-ms-win-core-synch-l1-2-0` は Windows 8 以降にある」 | **一次に無い**（今回取得した Learn のページに該当の記述は見つからなかった。出典は確認できていない） |
| `LoadLibraryExW` の `dwFlags` = 0x800（`LOAD_LIBRARY_SEARCH_SYSTEM32`）と API セット名の組み合わせの扱い | **一次に無い**（LoadLibraryExW のページは API セット名に触れない） |
| API セット名のモジュールに対する `GetProcAddress` の結果（kernel32 に転送されるか、NULL か） | **一次に無い**（windows-apisets、api-set-loader-operation は「ローダーが契約をホストにリダイレクトする」と書き、ホストの export に対する `GetProcAddress` の戻りは書かない） |
| 実機の API セット スキーマの内容 | **一次に無い**。非公式の資料（Geoff Chappell、ReactOS、Wine）は今回使っていない |

## 6. FLEngine 側の既存の扱いとの整合（項目 5）

### 6.1 現行の表と決定の基準（HEAD e410484 を `lift/out-d4/stage` に展開して確認）

- `fle_modules[]`: **26 名**（`advapi32.dll`、`api-ms-win-crt-string-l1-1-0.dll`、`comctl32.dll`、`comdlg32.dll`、`crypt32.dll`、`dsound.dll`、`gdi32.dll`、`imagehlp.dll`、`kernel32.dll`、`mpr.dll`、`msacm32.dll`、`oleacc.dll`、`ole32.dll`、`oleaut32.dll`、`opengl32.dll`、`shell32.dll`、`shfolder.dll`、`shlwapi.dll`、`user32.dll`、`version.dll`、`wininet.dll`、`winmm.dll`、`winspool.drv`、`wintrust.dll`、`ntdll.dll`、`msctf.dll`）。実測（ファイルを読んで数えた）。**`advapi32.dll`、`kernel32.dll`、`user32.dll`、`ntdll.dll` は既に入っている**。DLL の表 B のうち `advapi32`、`kernel32`、`ntdll`、`user32` は、拡張子を足した名前で既存の仮想モジュールに当たる（`fle_load.c` の `normalize`）。
- `fle_load_absent[]`: **9 名**（`dwmapi.dll`、`imm32.dll`、`msimg32.dll`、`propsys.dll`、`rpcrt4.dll`、`shcore.dll`、`uxtheme.dll`、`windowscodecs.dll`、`wtsapi32.dll`）。
- 決定 0015（PM）: 仮想モジュールは名前と固定ハンドルだけ。`GetProcAddress` は応答表（`fle_resp_table`）にある API だけ擬似アドレスで答え、無ければ NULL と 127。「存在すると偽らない」。未知のモジュール名は NULL と 126。
- 決定 0029: `fle_load_absent[]` は「決定 0015 が『ロードされていない』と名指しした 9 DLL」で、「それ以外のすべて」は `fle_stop`。**足す場合は 1 行ずつ、PM の決定が先**。
- 決定 0055（msctf.dll）: 「環境の定義（決定 0014: Windows 10 22H2）と矛盾する場合は欠いているとは定義しない」。`fle_load_absent[]` に足してよいのは (a) ゲストが NULL を許容、(b) NULL の経路が自然、(c) 環境の定義と矛盾しない、のすべてが確認できたときだけ。msctf は (c) が確認できず仮想モジュールにした。GetProcAddress は「ゲストが引く名前」だけ行を足し、他の export は主張しない。
- 参考: `modules.rs` の既存テストは `api-ms-win-core-libraryloader-l1-1-0.dll` を「モジュールでない（GetModuleHandleW が NULL）」とする行を持つ（`crates/fe-lift/tests/modules.rs` の `rules` 表）。この行は `api-ms-win-core-synch-l1-2-0` などとは別の名前だが、「`api-ms-win-*` を足すとテストの前提を変える」ことは**ない**（別の名前）。ただし `api-ms-win-core-libraryloader-l1-1-0` 自体は DLL の呼び出し点に出ない。

### 6.2 選択肢を DLL の各名前に当てた表（読み＋一部実測。決めない）

前提: ラッパーは §2.4 のとおり。「(a)」は仮想モジュールに足す（`LoadLibraryExW` が非 NULL を返し、`GetProcAddress` は応答表の行がなければ NULL と 127）。「(b)」は `fle_load_absent[]`（NULL と 126）。「(c)」は現状どおり停止。応答表の行は足さないものとする（足す場合は §4.3）。

| DLL の名前（使う添字） | (a) 仮想モジュール | (b) absent | (c) 停止のまま |
|---|---|---|---|
| `api-ms-win-core-synch-l1-2-0`（A 添字 1、B 添字 6）→ `InitializeCriticalSectionEx` | **実測**（`all_ok`）: `GetProcAddress(…, "InitializeCriticalSectionEx")` が NULL → `kernel32` は試されない → `InitializeCriticalSectionAndSpinCount` に落ちる（HEAD は runtime に無く `fle_unsupported(0x37d0a4f)`。作業ツリーの未コミット変更で応答表に行あり） | **実測**（`null_ms`）: `LoadLibraryExW` が NULL → リストの次の `kernel32` で `GetProcAddress` が NULL → 同じく `InitializeCriticalSectionAndSpinCount` に落ちる。結果は (a) と同じ代替 | **実測**: 停止（exit 5）。ATTACH が完走しない |
| `api-ms-win-core-fibers-l1-1-1`（A 添字 0）→ `FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue` | **実測**: `GetProcAddress` NULL → `Tls*` に落ちる。`FlsSetValue` の代替は `call [IAT TlsSetValue]`。HEAD は `TlsSetValue` に行ありだが、**ATTACH の続きで `f_37d7570`（`jmp [IAT 0x1ebd048]` の FlsSetValue の薄いスタブ）が `fle_unsupported(0x37d7570)`**（実測: §7.3 の呼び元の鎖） | **実測**: NULL → `kernel32` を試す → `GetProcAddress("FlsAlloc")` が NULL（応答表に行なし）→ 同じ代替 | 停止 |
| `api-ms-win-core-localization-l1-2-1`（B 添字 2）→ `LCMapStringEx`、`LocaleNameToLCID` | `GetProcAddress` NULL → LCMapStringEx の代替（`LocaleNameToLCID` を探して `LCMapStringW`）。**`LCMapStringW` は HEAD に行あり**（`fle_locale.c`）。ATTACH で実行されるかは**未確認** | NULL → `kernel32` で NULL → 同じ代替 | 呼ばれれば停止。呼ばれるかは未確認 |
| `api-ms-win-appmodel-runtime-l1-1-2`（B 添字 17）→ `AppPolicyGetProcessTerminationMethod` | `GetProcAddress` NULL → `STATUS_NOT_FOUND` を返し DLL は止まらない。**終了経路でだけ** | NULL → リストはこの 1 つだけ → 同じ | 呼ばれれば停止。終了経路なので ATTACH では呼ばれない（読み） |
| `kernel32`（A 添字 2、B 添字 15）→ 上の各 API | **既に仮想モジュール**（`kernel32.dll`）。`LoadLibraryExW("kernel32", NULL, 0x800)` は現行の runtime で成功（実測: dwFlags 0x800 は `LLEX_MODELLED` にある）。応答表に行がある API だけ非 NULL | — | — |
| 表 B のうち添字を使う呼び出しが無い 15 名（datetime、file、localization-obsolete、processthreads、string、sysinfo、winrt、xstate、ntuser-window、security-systemfunctions、ntuser-dialogbox、ntuser-windowstation、`advapi32`、`ntdll`、`user32`。後ろの 3 名は既に仮想モジュール） | 添字を使う呼び出しが無い（§2.3）ので**何も起きない**（読み） | 同じ | 同じ |
| `mscoree.dll`（`GetModuleHandleExW`） | DLL は `GetModuleHandleW(NULL)` の結果が .NET イメージでないので到達しない（読み） | 同じ | 同じ |

(b) の実機整合: absent は「この環境は欠く」という主張になる。Learn は、各名前が Windows 10 22H2 にあるかを**一次に書いていない**（§5.3）。決定 0055 の (c) の確認（「環境の定義と矛盾しない」）は、`api-ms-win-core-synch-l1-2-0` などについて**一次資料から確認できない**。

### 6.3 推奨（PM が決める。ここは意見）

事実の整理だけでは、(a) と (b) は ATTACH の結果が **同じ代替 API に落ちる**（上の表）。違いは (i) 環境の主張の種類、(ii) どのモジュール名が `GetProcAddress` の `hModule` として DLL に渡るか、(iii) `kernel32` の行のある API がどう答えるか。

- (a) の特徴: 「ある」と答える。応答表の行が無い API は NULL と 127 になる（決定 0015 のとおり）。**`api-ms-win-*` の名前の下に応答表の行を作らない限り、DLL が API セット名のモジュールから得る関数ポインタは常に NULL**（§4.4）。つまり「モジュールはあるが中身は空」。`InitializeCriticalSectionEx` ページの api_location には `API-MS-Win-Core-Synch-l1-2-0.dll` が載る。この欄が「その名前の `GetProcAddress` で取れる」ことまで意味するかは**一次に無い**（§5.3）ので、実機との差は断言しない。
- (b) の特徴: 「無い」と答える。DLL は `kernel32` に進む。`kernel32` の `GetProcAddress` は、応答表に行がある API（`kernel32.dll` の行）だけ非 NULL を返す。**応答表に `kernel32.dll` の行を足した API（例: `InitializeCriticalSectionEx`、`FlsAlloc`。Requirements の DLL が Kernel32.dll）は、(b) でだけ DLL に非 NULL で渡る**（(a) では DLL は `api-ms-*` のハンドルで NULL を受け、`kernel32` を試さない: §2.4）。ただし (b) は「この環境に `api-ms-win-core-synch-l1-2-0` は無い」という主張になる。Learn の windows-apisets は「スキーマが契約を結び付ければ成功する」と書くだけで、Windows 10 22H2 にその名前があるかは**一次に無い**（§5.3）。(b) が決定 0055 の (c) を満たすとは**確認できない**。
- (c): 現状。D4 を決めない選択。ATTACH が完走しない。

**事実として言えること（2 点）**:
1. 現行の応答表（`kernel32.dll` の行なし）では、(a) と (b) のどちらでも、DLL は同じ代替（`Tls*`、`InitializeCriticalSectionAndSpinCount`）に落ちる（§6.2、実測）。ATTACH の次の停止点（§7.3）は D4 の選択に依らない。差が出るのは、`kernel32.dll` の下に `InitializeCriticalSectionEx` や `FlsAlloc` の行を足したとき（(b) だけが非 NULL を DLL に渡す）。
2. 決定 0055 の基準に照らすと、(b) の「無い」は一次資料で確認できない主張。(a) の「ある」は、Learn が書く API セットの性質（契約名で `LoadLibrary` に渡せる）とは矛盾しないが、個別の名前がスキーマにあるかは一次に無い。

**推奨（意見。PM が決める）**: (a)。理由は、環境について新しく主張する量が少ないこと（モジュールの存在だけで、API の存在は応答表の行ごとに別に決められる）と、決定 0055 の流儀（「欠いている」とは定義しない）に沿うこと。弱点も書く: (a) では `api-ms-*` の下の応答表の行を作らない限り、DLL は `kernel32` を試さず代替に落ち続ける（§2.4）。DLL に `InitializeCriticalSectionEx` や `FlsAlloc` を非 NULL で渡したい場合は、`api-ms-win-core-synch-l1-2-0.dll`、`api-ms-win-core-fibers-l1-1-1.dll` の `dll` 欄で応答表に行を足す必要がある（§4.4）。それを望むかは Learn からは決まらず、PM の判断。

(a) を選ぶ場合の追加事実: 仮想モジュール表に足すとき、`api-ms-win-core-synch-l1-2-0.dll` などの**ハンドル値は N = 27 以降**（決定 0015 の「追加は末尾」）。既存テストが 26 名を固定している箇所は、今回の確認では `msctf_is_the_26th_virtual_module` と `load.rs` の `N_CRT_STRING` の表（番号は固定済み）。`fle_module_count()` を数に使うテストは `modules.rs`、`msctf.rs` にある（数の期待値を持つかは未精査）。

## 7. 実測の記録（gdb。`lift/out-d4/gdb*.txt`）

### 7.1 方法

既存の native 2 モジュール バイナリ（`lift/out-design/nat/two-native`: FLEngine と縮小 DLL（`table-seeds-idx0` の 428 種、1,891 関数）、リベース 0x2000000。`two-module-design.md` §4.6 のもの）を、gdb で DllMain の ATTACH から動かした。runtime のソースは編集していない。gdb スクリプト（`gdb_d4b.py`）は `fle_imp_LoadLibraryExW`、`fle_imp_GetProcAddress`、`fle_imp_FreeLibrary` に breakpoint を置き、`return` で runtime 関数を飛ばして R[0] に答えを入れる。**これは「DLL が何を尋ねるか」を見る実験で、D4 の答えの提案ではない**。

実験のために入れた他の注入（D4 の答えではない）:
- `f_37d0a04` 内の abort（`InitializeCriticalSectionAndSpinCount` に HEAD の runtime の行が無いため）を、戻り値 1 で継続。
- `fle_dispatch_miss`（縮小 DLL の dispatch 表に `0x37d23b4` などが無い）を、戻り値 1 で継続（`gdb5-all_ok.txt` に 9 回）。これは縮小の種が足りないための副次的な停止で、D4 とは別。

### 7.2 シナリオと結果（実測）

| シナリオ | `LoadLibraryExW` の答え | `GetProcAddress` の答え | DLL が呼んだ順序 |
|---|---|---|---|
| 無修正（HEAD の runtime） | runtime のまま | runtime のまま | `LoadLibraryExW("api-ms-win-core-synch-l1-2-0", 0, 0x800)` → 停止（`gdb-all_ok` の前の native ハーネスの出力。exit 5） |
| `all_ok` | 全名に非 NULL（偽のハンドル） | NULL | `LoadLibraryExW(api-ms-win-core-synch-l1-2-0, 0x800)` → `GetProcAddress(…, "InitializeCriticalSectionEx")` → `LoadLibraryExW(api-ms-win-core-fibers-l1-1-1, 0x800)` → `GetProcAddress(…, "FlsAlloc")` → `GetProcAddress(…, "FlsSetValue")`（**`kernel32` の `LoadLibraryExW` は 1 回も呼ばれない**） |
| `null_ms` | `api-ms-*`/`ext-ms-*` は NULL（last error 126）、他は非 NULL | NULL | `LoadLibraryExW(api-ms-win-core-synch-l1-2-0, 0x800)`→NULL → `LoadLibraryExW(kernel32, 0x800)`→非 NULL → `GetProcAddress(kernel32, "InitializeCriticalSectionEx")` → `LoadLibraryExW(api-ms-win-core-fibers-l1-1-1, 0x800)`→NULL → `GetProcAddress(kernel32, "FlsAlloc")`（キャッシュ済み kernel32）→ `GetProcAddress(kernel32, "FlsSetValue")` |
| `err87` | `dwFlags != 0` の呼び出しは NULL と ERROR_INVALID_PARAMETER（87）、`dwFlags == 0` は非 NULL | NULL | `api-ms-win-core-synch-l1-2-0` を 0x800 で NULL（87）→ **再試行されない** → `kernel32` を 0x800 で NULL（87）→ **`kernel32` は `dwFlags=0` で再試行され非 NULL** → `GetProcAddress(kernel32, …)`（§2.4 手順 3 の根拠） |
| `all_ok_hit`（偽ポインタを返す） | 全名に非 NULL | `InitializeCriticalSectionEx` に偽の擬似アドレス | DLL が `call` → `fle: indirect call to 0x7ff9dead0010 is not a known function entry` |

`GetProcAddress` に渡った API 名は、どのシナリオでも **`InitializeCriticalSectionEx`、`FlsAlloc`、`FlsSetValue` の 3 つ**（この順。`FlsSetValue` は 2 回目のラッパー呼び出し、`FlsGetValue`、`FlsFree`、B 系の 5 名は ATTACH のこの範囲では呼ばれなかった）。DLL の `hFile` はどの呼び出しでも 0、`dwFlags` は 0x800（`err87` の再試行だけ 0）。

### 7.3 ATTACH の次の停止点（実測）

D4 を `all_ok` で答えたときの次の停止（`gdb5-all_ok.txt`、`gdb4-all_ok.txt`）: `fle: unsupported instruction at 0x37d7570`（`f_37d7570` は `FlsSetValue` の IAT スロット（`jmp [0x1ebd048]`）の薄いスタブで、HEAD の応答表に `FlsSetValue` の行が無い。呼び元の鎖（gdb のバックトレース）: `f_37cb484` → `f_37d2484` → `f_37de9d0` → `f_37de9b0`（`jmp rax` の guard dispatch スロット）→ `fle_dispatch` → `f_37d6c5c` → `f_37d317c` → `f_37d7570`。`f_37d6c5c`（RVA 0x17d6c5c）は DLL のデータ表（RVA 0x1ec8400）に関数ポインタとして載る初期化関数で、その中身の用途は今回読んでいない）。その手前で、`InitializeCriticalSectionAndSpinCount` の `fle_unsupported(0x37d0a4f)` が出る（上の注入で飛ばした）。つまり **D4 の答えで ATTACH が先へ進んでも、HEAD の runtime では次に `InitializeCriticalSectionAndSpinCount` と `FlsSetValue` で止まる**。これは D4 とは別の（W6、決定 0078）項目。

## 8. 実測 / 読み / 未確認 / 仮説

### 実測
- HEAD の runtime で DLL の ATTACH が `LoadLibraryExW("api-ms-win-core-synch-l1-2-0.dll")` で停止（exit 5。native 単独）。
- DLL の名前表 A（3 名）、B（19 名）、API 名の文字列、モジュール添字リストのバイト列（ファイルから読んだ）。
- `LoadLibraryExW`、`GetModuleHandleW`、`GetModuleHandleExW`、`GetProcAddress`、`FreeLibrary`、`GetModuleFileNameW` の IAT 参照 16 件の走査と、うち 14 件の `call` の確認。
- gdb の 5 シナリオ（§7.2）の呼び出し順序、引数（名前、`dwFlags`）。
- `FLEngine_x64.dll` の `DataDirectory[14]` が (0, 0)。
- `fle_modules[]` = 26 名、`fle_load_absent[]` = 9 名（HEAD のファイル）。
- 応答表（HEAD の `imports_table.rs`）に `InitializeCriticalSectionEx`、`AreFileApisANSI`、`LCMapStringEx`、`LocaleNameToLCID`、`AppPolicyGetProcessTerminationMethod`、`FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue`、`InitializeCriticalSectionAndSpinCount`、`GetModuleHandleExW`、`TlsFree` の行が**無い**。`LCMapStringW`、`TlsAlloc`、`TlsGetValue`、`TlsSetValue`、`GetModuleHandleW`、`GetProcAddress`、`LoadLibraryExW`、`FreeLibrary`、`GetModuleFileNameW` は有る。
- Learn の Requirements と api_location（§4、§5。取得日 2026-10-23）。

### 読み
- 呼び出し点ごとの引数（`hFile` = NULL、`dwFlags` = 0x800 / 0、`GetModuleHandleW(NULL)`、`L"mscoree.dll"`）。
- ラッパー A、B のプロトコル（§2.4）。特に「`GetProcAddress` が NULL のとき次のモジュールを試さない」。
- 各 API の NULL のときの分岐（§3.2、§4.1）。
- 名前が「計算」で作られる呼び出し点は無い。
- 到達性（ATTACH の直線上か、終了経路か）。

### 未確認
- 縮小 DLL の ATTACH が、上の 3 回の照会の後にどこまで進むか（`FlsSetValue` と `InitializeCriticalSectionAndSpinCount` の停止の先）。したがって `FlsGetValue`、`FlsFree`、B 系 5 名が実行されるかは未確認。
- 表 B の添字 1（`api-ms-win-core-file-l1-2-2`）、ほか 14 名は `LoadLibraryExW` に渡らない、という読みの確認（実行はしていない）。
- 0x17d9f1c（`LocaleNameToLCID` の代替）の中身、0x17d7768 の `FreeLibrary` の呼ばれ方。
- 全部入りの DLL（`--pdata`）での実行。実測は縮小 DLL（SSE2 系統の種）だけ。ATTACH の直線は共通の関数だが、別の種で関数が増えても名前表は同じ（ファイルのデータ）。
- 実機 Windows 10 で各 API セット名が `LoadLibrary` で解決できるか、`GetProcAddress` の戻り（一次に無い）。
- `two-module-design.md` D4 の「実機で `api-ms-win-core-synch-l1-2-0` は Windows 8 以降にある」の出典。

### 仮説
- ATTACH の通常経路で `LoadLibraryExW` に渡る名前は §7.2 の 3 名（`api-ms-win-core-synch-l1-2-0`、`api-ms-win-core-fibers-l1-1-1`、`kernel32`）だけで、他の 2 名（`api-ms-win-core-localization-l1-2-1`、`api-ms-win-appmodel-runtime-l1-1-2`）は ATTACH では呼ばれない。根拠はロケール初期化とプロセス終了の経路が ATTACH の `FlsSetValue` の先にあるという読みだが、**実行していない**。

## 9. 作成物

- `docs/investigations/dsp-ippv2-dynamic-imports.md`（この文書。新規）。
- `lift/out-d4/`（gitignore 対象。約 245 MB。`dis-all.txt` 255 MB が大半。2 GB 級の単一ファイルは無い）:
  - 補助ツール: `d4_scan.py`（IAT 参照の全走査）、`d4_xref.py`（初期の試作。ファイルは残したが結果に使っていない。逆アセンブルの整列に依存）、`d4_rd.py`（データ読み）、`d4_callers.py`（`E8`/`E9` の呼び元）、`d4_wrappers.py`（名前表の復号）、`gdb_d4.py`、`gdb_d4b.py`（gdb ドライバ）、`learn_fetch.py`（Learn の取得）。
  - 出力: `scan-iat-sites.tsv`、`xref-raw.txt`、`callers-wrappers.txt`、`wrappers-decoded.txt`、`dis-d0600.txt`、`dis-d1960.txt`、`dis-all.txt`、`reach-entry-only-funcs.txt`、`reach-entry-only.summary.txt`、`gdb-*.txt`、`gdb2-*.txt`〜`gdb6-*.txt`（`*.err` を含む）。
  - `learn/`（取得した Learn のページ 21 件）、`stage/lift/`（`git archive HEAD` から取り出した `imports_table.rs`、`fle_load.c`、`fle_modules.c`、`fle_modules.h`）、`tmp/`（空）。
- 既存ファイル、runtime、生成器、テスト、git の状態は変更していない。`rm` は使っていない。

注記: `gdb_d4b.py` の初期の版（`gdb_d4.py`）と `gdb-*`、`gdb2-*`、`gdb3-*` の出力は、`SIGABRT` の注入が未完成の段階のもので、§7 の根拠には `gdb4-all_ok`、`gdb5-*`、`gdb6-err87` を使った。`gdb-all_ok_hit.txt` は偽ポインタの実験（§4.3）に使った。

