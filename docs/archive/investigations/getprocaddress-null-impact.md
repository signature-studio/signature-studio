# GetProcAddress の NULL(229 回・90 組)は、ゲストの分岐に影響しているか

作業日: 2026-10-30。基準 HEAD b8ed117(ブランチ main)。**調査(読み取りと静的分析、既存バイナリへの gdb 計測のみ)。ソース・runtime・応答表は変更していない。何も決めない。**

凡例: **[実測]**＝トレースの記録・gdb の計測・コマンド出力そのもの／**[読み]**＝逆アセンブルや記録からの私の解釈／**[未確認]**。

## 0. 結果(先に)

NULL 229 回(90 組)の分類: **A 22 組(54 回)、B 25 組(59 回)、C 42 組(113 回)、D 1 組(3 回)、E 0 組(0 回)**。A は A1(NULL を検査して別経路)17 組(41 回)と A2(検査なしで call するセルが残り、NULL のまま到達すると `call 0` で止まる)5 組(13 回)に分けた。

| 項目 | 結果 |
|---|---|
| a4 の完走中に、NULL によって**実際に別の枝が実行された**もの | **GetDpiForMonitor の 1 名だけが A**(FLEngine 3 回、ILTools 2 回、NULL の枝を毎回通った)[実測]。他に、CRT のフォールバック(InitializeCriticalSectionEx、Fls*、LCMapStringEx、LocaleNameToLCID、AreFileApisANSI)、タッチ可否の門、VarBstrFromBool の既定実装が実行されたが、どれも B(同等の経路)と読んだ [読み] |
| A の 22 名のうち、消費側の関数が a4 で**1 回も入られていない**もの | **20 名**(消費側の関数の入口を gdb で数えて 0 回)[実測]。入られたのは **GetDpiForMonitor**(NULL の枝を実行)と **SetThreadPreferredUILanguages**(関数は 1 回ずつ実行されたが、検査なしの call は通っていない。呼び出し回数からの推定)の 2 名。前者は応答を足すと a4 の実行が変わる。後者と残り 20 名は、足しても a4 の実行は変わらない見込み(保存されるだけ)[読み]。ただし完走後(InitUnits の後)の動作は測っていない |
| 「Learn が Windows 10 に在ると書く 65 名」の内訳 | A 21 名、B 16 名、C 28 名(残り 24 名＝最小 OS 不明の (e) 側は A 1、B 9、C 14)[実測: 名簿の突き合わせ] |
| 依頼文の例の検証 | (1) Blend2D の `WaitOnAddress` 系 3 名が NULL のとき、セル 0x48f31a0 = 0 は**スピンではなく SRWLock＋条件変数の実装**を選ぶ [読み]。セルの値は実測で 0。(2) InitializeSRWLock が NULL のときの別のロック実装は**イベントを使うエミュレーション**で、a4 では入られない [読み・実測]。(3) GetFileSizeEx が NULL のとき別経路は GetFileSize で、結果は同じ(B)[読み] |
| 見つけた副作用(名前ごとの分類とは別) | NULL を返すたびに last error が 127 になり、**QFC の 2 回と ILTools の 1 回の `GetLastError()` が 127 を読み、モジュールのディレクトリ文字列を作る枝を飛ばした** [実測・読み]。後述 §5 |
| 最優先で足すなら | **CompareStringOrdinal**(A2)。ゲストが OS バージョン ≥ 6 を確かめたうえで保存し、検査せずに呼ぶ(環境の自己矛盾)。足しても a4 の実行は変わらない見込み [読み] |
| 足さなくてよい | C 42 名(セルに保存するだけで読み手が無い)、B の大部分、A のうち機能のオン・オフだけのもの(avrt、powrprof、Ole*、AnimateWindow、CoCreateInstanceEx) |

## 1. 方法

1. **90 組の取り出し**: `a4-six-module-trace/trace.tsv.gz` の GetProcAddress 1,612 回のうち戻り値 0 の 229 回(列「ゲストの戻り番地」と呼び出し元モジュールつき)。`summary/getprocaddress.tsv` の NULL 90 組と一致 [実測]。
2. **呼び出し元の特定**: 戻り番地は FLEngine/QFC/ILTools では小さなラッパー(FLEngine `0x4235f0` が `GetProcAddress` の薄いラッパーで、戻り番地 `0x42364e`)の中を指すので、名前の文字列(ASCII/UTF-16)への RIP 相対 `lea` を全イメージから探し、そこから「取得してセルに保存する関数(ローダー)」を特定した(`scripts/litx.py`、`scripts/cells.py`)。dsp/Blend2D は戻り番地がそのまま CRT のラッパー内。
3. **セルの読み手の探索**(`scripts/ripmap2.py`、`relocs.py`、`slots.py`、`consumers.py`): セルを参照する RIP 相対命令(mov/cmp/call/lea)と、セルのアドレスを持つデータ(再配置エントリ)を経由する参照(`mov rax,[slot]; call [rax]`)の両方を数え、`cmp [..],0` の有無で「検査あり」と「検査なしの call」を分けた(`scripts/unchecked.py`)。呼び出し元の関数の逆アセンブルは全部手で読んだものではない(読んだ関数は §4 と `data/facts.tsv` の根拠欄に関数 VA で書いた)。
4. **実行された関数の計測**(`scripts/gpa_gdb.py`): 既存の 6 モジュール構成の native バイナリ(§6)を gdb で走らせ、ローダー・消費側の関数 230 個の入口に breakpoint を置いて入口の回数と InitCount を数え、終了時に 217 個のセルの値を読んだ。`fle_imp_GetProcAddress` の入口は 1,612 回(トレースと同数)。a4 の最終行(`init_count=942 of 942`、`runnable_started=895 completed=895`、`all_units_done=yes`)は計測中も同じだった [実測]。
5. **Learn と実機**: `win-replay-triage/mismatch-161.tsv` の Learn 欄(最小クライアント)と、`worklog/1/observed.tsv`(実機の GetProcAddress の戻り)を名前で突き合わせた(`data/learn.tsv`)。**実機は Windows 11 26H2 であって Windows 10 22H2 ではない**(`worklog/1/env.md`)。「Windows 10 に在る」は Learn の最小 OS から導いた読みである。

## 2. 分類の定義

| 区分 | 意味 |
|---|---|
| A1 | NULL を検査し、別の経路に入る。別の経路は同等ではない(機能が無効になる、または別の実装になる) |
| A2 | A1 と同様だが、保存したセルを**検査せずに** call する箇所が残る。NULL のまま到達すると `fle_dispatch(0)` で止まる(実行されれば。a4 では 5 名とも到達していない) |
| B | NULL は検査されるか既定値が入り、別経路が実機の経路と等価(旧 API による代替、同じ結果を返す) |
| C | NULL のセルを保存するだけで、読み手が無い |
| D | 実機でも NULL |
| E | 読めなかった(0 件。ただし §7 の限界) |

## 3. 90 組の分類(要約。全行は `classification.tsv`)

| 区分 | 組 | 回 | 名前 |
|---|--:|--:|---|
| A1 | 17 | 41 | WaitOnAddress、WakeByAddressSingle、WakeByAddressAll、AvSetMmThreadCharacteristicsW、AvSetMmThreadPriority、AvRevertMmThreadCharacteristics、InitializeSRWLock、CoCreateInstanceEx、OleCreateFontIndirect、OleCreatePictureIndirect、OleLoadPicture、PowerGetActiveScheme、PowerSetActiveScheme、GetDpiForMonitor、SHGetKnownFolderPath、AnimateWindow、UpdateLayeredWindowIndirect |
| A2 | 5 | 13 | CompareStringOrdinal、SetThreadPreferredUILanguages、CoInitializeEx、SetLayeredWindowAttributes、UpdateLayeredWindow |
| B | 25 | 59 | FlsAlloc、FlsGetValue、FlsSetValue、InitializeCriticalSectionEx、LCMapStringEx、LocaleNameToLCID、AreFileApisANSI、SleepConditionVariableCS、WakeAllConditionVariable、GetFileSizeEx、GlobalMemoryStatusEx、VariantChangeTypeEx、VarI4FromStr、VarR8FromStr、VarDateFromStr、VarCyFromStr、VarBoolFromStr、VarBstrFromCy、VarBstrFromDate、VarBstrFromBool、GetTouchInputInfo、CloseTouchInputHandle、RegisterTouchWindow、UnregisterTouchWindow、IsTouchWindow |
| C | 42 | 113 | CheckTokenMembership、RegCopyTreeW、TryAcquireSRWLockExclusive、TryAcquireSRWLockShared、InitializeConditionVariable、Av の 7 名(AvQuerySystemResponsiveness、AvRtCreate/Delete/Join/Leave/WaitOnThreadOrderingGroup、AvSetMmMaxThreadCharacteristicsW)、FlatSB 系 12 名、Co の 4 名(CoAddRefServerProcess ほか)、OleCreatePropertyFrame、Var の演算 13 名(VarAdd、VarAnd、VarCmp、VarDiv、VarIdiv、VarMod、VarMul、VarNeg、VarNot、VarOr、VarSub、VarXor、VarR4FromStr) |
| D | 1 | 3 | AvRtCreateThreadOrderingGroupEx(実機 Win11 でも NULL・127。`observed.tsv` C01141。Learn のページは 404) |
| E | 0 | 0 | — |

呼び出し元(ローダー関数、モジュール名つき)は `classification.tsv` の `static_string_xref_fn` 列と `loader` 列。例: FLEngine `f_65c9f0`(unit 146。avrt 11 名、SRW 3 名、powrprof 2 名、CheckTokenMembership、RegCopyTreeW、GetDpiForMonitor、UpdateLayeredWindowIndirect を一括で取得)、QFC `f_1e48e00`(unit 159)、ILTools `f_61ae320`(unit 246)は同じ形のコード。

## 4. A の一覧と根拠

「Learn」は最小クライアント(`data/learn.tsv`。取得は triage の 2026-10-30)。実機は全て非 NULL(Win11 26H2、実測)。**足す優先度は私の読みで、決めていない。** 「a4」欄は、消費側の関数が a4 で入られたかの計測(`data/hits-run3.tsv`)。

| 名前 | モジュール(ゲストが渡した) | 回 | Learn | NULL 後の動作(読み) | a4 | 足したときの見込み | 優先度 |
|---|---|--:|---|---|---|---|---|
| CompareStringOrdinal | kernel32.dll | 3 | Vista | FLEngine `f_44b7e0`(unit 18)が `f_44ad60(6,0)`(GetVersion ≥ 6)を確かめて GetProcAddress の結果を**検査せず**セル 0x15ec808 に保存。消費側 `f_44b8e0`、`f_44b9b0` は検査せず call。NULL なら `call 0`。QFC・ILTools はセルのみで読み手なし | 消費側 0 回 | 保存されるだけなので a4 は変わらない見込み。`fle_imp_` を 1 つ(序数比較)足す必要がある | **高** |
| UpdateLayeredWindow | user32 | 2 | 2000 | `f_71e570`(`UpdateLayeredWindowIndirect` のセルが NULL のとき)、`f_71ec10`、`f_732aa0`、`f_80da00`、`f_8165c0` が検査せず/Indirect の検査だけで call(A2) | 0 回 | `fle_imp_` が要る。3 名(下 2 名＋この名)を一緒に足さないと止まる | 中 |
| SetLayeredWindowAttributes | user32 | 2 | 2000 | `f_864260` は検査。`f_622b50`(FLEngine、データポインタ経由で到達)と ILTools `f_61851c0` は検査せず call(A2) | 0 回 | 同上 | 中 |
| UpdateLayeredWindowIndirect | user32 | 3 | (404) | `f_71e570` 0x71e606: NULL ならプレーンな UpdateLayeredWindow へ(上の A2 に合流) | 0 回 | 単独では止まりを除けない | 中 |
| CoInitializeEx | ole32 | 3 | 2000 | `f_5e0d90` は検査して `CoInitialize`(`fle_imp_CoInitialize` あり)へ。`f_1331c80`(データポインタ経由)は検査せず `call [slot]`(A2) | 0 回 | 小さな `fle_imp_`(CoInitialize と同じ答え) | 中 |
| SHGetKnownFolderPath | shell32 | 3 | Vista | `f_63b970`: NULL なら文字列を空のまま返す。呼び元 `f_653130` などが文字列を検査し `SHGetFolderPathW(csidl 5)` に落ちる。`fle_imp_SHGetFolderPathW` は csidl 0x14、0x1a だけ答え、5 は名前付き停止(`fle_env.c:471-486`)[読み] | 0 回 | 行を足す方法と、SHGetFolderPathW に csidl 5 を足す方法がある | 中 |
| SetThreadPreferredUILanguages | kernel32.dll | 3 | Vista | `f_419400` が検査せず call(0x41948a、0x4194f6、0x419535)。ただし前段に `GetThreadUILanguage() == 引数` の比較(0x41941f)がある | **関数は 1 回ずつ実行**。call は実行されていない(GetThreadPreferredUILanguages が 1 入口あたり 2 回＝等しい経路。不等の経路なら 4 回以上)[読み+実測の回数] | a4 は変わらない | 低 |
| GetDpiForMonitor | shcore.dll | 3 | 8.1 | `f_812e80` 0x812f2f: セルが NULL ならモニター DPI のブロックを飛ばし、セル 0x15f6f48/0x15f6f4c は 0 のまま。`f_812870` は 0 なら基準値を使う | **FLEngine 3 回、ILTools 2 回、NULL の枝を実行**(終了時のセル値 0)[実測] | **a4 の実行が変わる**。`fle_imp_` とモニターオブジェクトの呼び出し(`f_8657b0` など)が走る。環境の DPI が 96 なら結果は同じ見込み [読み] | 低 |
| InitializeSRWLock | kernel32.dll | 3 | Vista | `f_6627f0`: セルが NULL ならイベントを使うロックのエミュレーション(`f_44cfb0` CreateEventW、`f_44d1c0` ResetEvent/WaitForSingleObject/GetCurrentThreadId、`f_44d2a0` SetEvent)。使う API は全て `fle_imp_` あり | 0 回(`f_6627f0` は `f_11c2d80` からデータポインタ 0x1170b4e 経由でのみ到達) | 小さな `fle_imp_`(8 バイトをゼロにする) | 低 |
| WaitOnAddress / WakeByAddressSingle / WakeByAddressAll | **api-ms-win-core-synch-l1-2-0** | 1 ずつ | 8 | Blend2D `f_4899d90`(DLL アタッチ。実行された)が 3 つをセル 0x48f3280/88/90 に保存し、3 つとも非 NULL のときだけ 0x48f31a0 を 1 にする(0x489a040-0x489a05b)。**0 のとき**ワーカー/ジョブのオブジェクトは SRWLock＋条件変数の実装(ワーカー `f_489c5e0`)、1 のとき WaitOnAddress の実装(ワーカー `f_489c7f0`)。どちらもワーカー増設は `f_48a4888` の **CreateThread**(`fle_imp_` なし)に達する | 設定は実行(セル 0x48f31a0 = 0、実測)。消費側 9 関数は 0 回 | 行を **api-ms モジュール名**で足す必要がある(決定 0081 は当面作らないとした)。新規 `fle_imp_` が 3 つ。CreateThread が答えられない限り、どちらの値でもスレッド作成で止まる | 低(足さない方がよい) |
| AvSetMmThreadCharacteristicsW / AvSetMmThreadPriority / AvRevertMmThreadCharacteristics | avrt | 3 ずつ | Vista | `f_12b00f0`(全文を読んだ)が `cmp [AvSetMm…W],0` で NULL なら MMCSS 登録を飛ばす。`f_bb0920` は同じ形(全文は読んでいない) | 0 回 | 効かない | 不要 |
| PowerGetActiveScheme / PowerSetActiveScheme | powrprof | 3 ずつ | Vista | `f_65c6c0`: Set の検査で電源プラン切替を飛ばす | 0 回 | 効かない | 不要 |
| CoCreateInstanceEx | ole32 | 3 | 2000 | `f_5df7a0` 0x5df7dd: NULL なら Delphi 例外(`f_448ee0`)。リモート COM | 0 回 | 効かない | 不要 |
| OleCreateFontIndirect / OleCreatePictureIndirect / OleLoadPicture | oleaut32 | 2 ずつ | 2000 | ラッパー `f_6f6a30/50/70` が NULL なら `0x8000FFFF` を返す | 0 回 | 効かない | 不要 |
| AnimateWindow | user32 | 2 | 2000 | `f_61b600` 0x61b89f: NULL ならアニメーションを省く | 0 回 | 効かない(SetWindowPos、ShowWindow には `fle_imp_` が無い) | 不要 |

### 機械語の抜粋(全体で 28 行)

Blend2D のフラグ設定(`f_4899d90`):

```
4899ff3 call [GetModuleHandleA]      ; "api-ms-win-core-synch-l1-2-0.dll"
489a005 call [GetProcAddress]        ; "WaitOnAddress"
489a00b mov  [0x48f3280], rax        ; 同様に WakeByAddressSingle -> 0x48f3288, WakeByAddressAll -> 0x48f3290
489a040 cmp  [0x48f3280], rdi(=0)    ; je 489a059
489a049 cmp  [0x48f3288], rdi        ; je 489a059
489a052 test rax, rax                ; mov eax, esi(=1) ; jne 489a05b
489a059 mov  eax, edi                ; eax = 0
489a05b mov  [0x48f31a0], eax        ; 実測: 終了時 0
```

そのフラグを読む側(`f_489ccf0`。ワーカー関数を選ぶ):

```
489cd6b mov  r14d, [0x48f31a0]
489cd7f cmovne ecx, edx(=0xc0)       ; 確保サイズ 0xc0 / 0x100
489cdc2 lea  rcx,[0x489c7f0] ; mov [rbx+0x10], rcx   ; フラグ != 0: WaitOnAddress 版ワーカー
489ce0c lea  rcx,[0x489c5e0] ; mov [rbx+0x10], rcx   ; フラグ == 0: AcquireSRWLockExclusive + SleepConditionVariableSRW
```

CompareStringOrdinal(FLEngine):

```
44b7e4 mov ecx,6 ; xor edx,edx ; call f_44ad60       ; OS 6.0 以上か(GetVersion = 0x4a65000a: 該当)
44b80a call f_4235f0                                   ; GetProcAddress(kernel32, "CompareStringOrdinal")
44b80f mov [0x15ec808], rax                            ; 検査なしで保存
44b928 call [0x15ec808]                                ; f_44b8e0: 検査なしの call
```

last error の門(QFC `f_1e22390`。ILTools `f_61a46e0` も同形):

```
1e223dd call f_1d1d730   ; GetModuleFileNameW(自分, buf, 0x208)
1e223e2 call f_1d1d6f0   ; GetLastError
1e223e7 test eax, eax
1e223e9 jne 1e2241b      ; != 0: ディレクトリ文字列(1e223eb-1e22416)を作らない
```

GetDpiForMonitor(`f_812e80`):

```
812f2f mov rax,[0x15e3c80] ; cmp qword [rax],0 ; je 81300c   ; NULL: 0x812f40-0x813006 を飛ばす
812f82 call [rax]          ; GetDpiForMonitor(hmon, 2, &x, &y)   ; 結果を 96..400 に丸めて 0x15f6f48 へ
```

SetThreadPreferredUILanguages(`f_419400`):

```
419419 call [0x15eaef0]    ; GetThreadUILanguage
41941f cmp ax, di ; 419422 je 419490                   ; 等しければ 0x41948a の call を飛ばす
41948a call [0x15eaee8]    ; 検査なし(セルは NULL)
```

タッチ(`f_80e3f0`)と InitializeSRWLock(`f_6627f0`):

```
80e47b cmp [0x15f6ed8],0 ; je 80e4a3 ... (4 つの名前を検査)
80e4a3 mov [0x15f6ee8], 0        ; どれかが NULL なら RegisterTouchWindow のセルを 0 に
6627f8 mov rax,[0x15e0d48] ; cmp [rax],0 ; je 662813   ; NULL: f_44cfb0(CreateEventW)のエミュレーション
```

## 5. last error 127 の漏れ(名前ごとの分類とは別の影響)

**[実測]** NULL の GetProcAddress は last error を 127 にし、実機で成功した GetProcAddress は last error を変えない(実機 114 ケースのうち 113 は変えず、変えた 1 つは NULL だった C01141)。runtime の NULL 応答の後、5 回の `GetLastError()` が 127 を読んだ(`data/last-error-127-gates.txt`):

| seq | モジュール | 127 を立てた NULL(窓の先頭) | その後の動作 |
|---|---|---|---|
| 702 | dsp_ippv2 | InitializeCriticalSectionEx、FlsAlloc、FlsSetValue | CRT が GetLastError を保存し `SetLastError` で戻す(`0x37d32cf`)。分岐に影響しない [読み。Blend2D の同形コード `0x48a6c91` は全文確認、dsp は呼び出しの存在のみ確認] |
| 2232 | Blend2D | 同上 | 同上(`SetLastError(ebx)` `0x48a6c91`)[読み] |
| 2741 | QFC | InitializeConditionVariable ほか計 8 名(CoXxx、SHGetKnownFolderPath) | **`jne 0x1e2241b` でモジュールのディレクトリ文字列の作成を飛ばす** |
| 4129 | QFC | UpdateLayeredWindowIndirect から GetDpiForMonitor まで計 20 名 | 同上 |
| 5950 | ILTools | InitializeFlatSB から SHGetKnownFolderPath まで計 13 名 | 同上(`jne 0x61a476b`) |

**[実測]** 門で飛ばされたとき、直後の `GetFileAttributesW` はディレクトリなしの名前 `"dsp_ippv2_x64.dll"`(seq 2742、5951)、`"freetype_x64.dll"`(seq 4130)を探した。le が 0 だった側は完全パス(FLEngine seq 636 `"C:\Program Files\Image-Line\FL Studio 21\dsp_ippv2_x64.dll"`、ILTools seq 7409 `"C:\…\Shared\Blend2D.dll"`)。**どちらの探索も全て失敗し、この構成では結果は同じ**(ファイルが無いため)。実機(Win10)ならこの窓の NULL が全て非 NULL なら 127 は立たず、門は通る [読み。窓の全名が非 NULL になる場合に限る]。個々の名前を足しても、窓の中に NULL が 1 つでも残れば 127 は立つので、この漏れは名前単位では解消しない。

## 6. 方法の限界(および使ったバイナリ)

- **計測に使ったバイナリは、トレースのものと別のビルド**(同じ 6 モジュール構成)。トレースの native SHA-256 は `9c76bc6a…`(`a4-six-module-trace/logs/md5.txt`)。run1(172 関数)は `lift/out-x1/o1/…/two-native`(SHA `f469d29f…`、a4: functions 5,306、entries 1,007,942。トレースと同数)、run2〜4(230 関数。他に GetDpiForMonitor 等)は `lift/out-fruity6/od2off1/…/two-native`(SHA `34fb3c4f…`、functions 5,308、entries 1,007,947)。**どちらの a4 も 942/942、895/895 で完走**。run1 と run2 で、共通の 172 関数の入口の回数は全て一致 [実測]。ただし他の作業者のビルドを借りた読み取り専用の実行で、`lift/out-x1/o1` と `od2off1` のバイナリは作業の途中で別の作業者の再ビルドにより消えた(私は触れていない)。そのため**最後に計画した 1 回(delay-import スタブ `f_865050`、`f_611c70`、`f_446ad0` の入口の計測)は実行できていない**。
- 「消費側 0 回」は**私が breakpoint を置いた関数だけ**の結果。置いていない関数(例: QFC・ILTools の消費側)は FLEngine の同形の関数から読んだもので、**QFC・ILTools の消費側は一部しか個別に読んでいない**(セルの参照数と検査の形で同形と確認)。
- 「読み手が無い(C)」は、RIP 相対参照とデータ中のアドレス(再配置)の探索で見つからなかったという意味。ベース＋添字で表を引くコード(セルの隣のスロットへの加算)や、実行時に計算したアドレスは見ていない [未確認]。Var 演算の 13 名は 0x15f1038 以降の連続したセル(表)で、表の範囲(0x15f1030-0x15f10f0)を参照する他の関数は参照カウンタ(0x15f1030、0x15f10e8)だけだった [実測]。
- **実機は Windows 11 26H2 の 1 台**。Windows 10 22H2 で GetProcAddress が非 NULL かは未測定。「Windows 10 に在る」は Learn の最小 OS(Vista〜8.1。oleaut32 の Var 21 名は最小 OS 欄が空、UpdateLayeredWindowIndirect はページが 404)からの読み。
- delay-import 経由(iv): FLEngine と ILTools には GetDpiForMonitor、RegisterTouchWindow、LocaleNameToLCID、SHGetKnownFolderPath の delay-import があり、スタブ(FLEngine 0x80fa30、0x425290、0x4237d0)がある。**NULL を返した GetProcAddress の戻り番地は全て Delphi のローダー(`0x42364e`、`0x457c5f`、`0x418e85`)か CRT のラッパー内**で、delay-load ヘルパー由来の NULL は 0 件 [実測]。スタブが他に入られたかは未計測 [未確認]。
- `unchecked-call-sites.tsv` の「検査なしの call」には、Var 系のように、ローダーが NULL のとき既定の実装を保存する(セルが NULL にならない)ため安全なものが含まれる。VariantChangeTypeEx のセルは終了時に既定のスタブ(0x4572c0 など)が入っている [実測]。
- A2 の SetThreadPreferredUILanguages は、「call が実行されなかった」を GetThreadPreferredUILanguages の呼び出し回数(1 入口あたり 2 回)から**推定**した。gdb で `R[8]`(第 3 引数)を読んだ値(0x20019)はコードと合わず、使っていない。
- 最終的な分類の文章(`data/facts.tsv`)は私が書いたもの。a4 の実行が NULL で変わるかの検証(実際に応答を足して走らせる)は**していない**(依頼の範囲外)。

## 7. 結論(推奨。決めない)

**応答を足すべき名前の最小集合(推奨)**:

1. **CompareStringOrdinal**(kernel32.dll)。OS ≥ 6 の確認の後で検査せずに呼ぶ A2 で、環境が「Windows 10 だが API が無い」と言っているのと同じ。足しても a4 の実行は変わらない見込み(保存されるだけ。消費側 0 回)。小さな `fle_imp_` が要る。
2. 完走後に UI を動かす段階で、レイヤードウィンドウ(`UpdateLayeredWindow`、`SetLayeredWindowAttributes`、`UpdateLayeredWindowIndirect`)と `CoInitializeEx`、`SHGetKnownFolderPath`(または `SHGetFolderPathW(csidl 5)`)。到達するかは未確認。実際に止まった時点で足す方針でもよい。

**足さなくてよい**: C の 42 名、B の大部分(CRT・RTL が同等の代替を持つ)、A のうち機能の有無だけのもの(avrt、powrprof、Ole*、AnimateWindow、CoCreateInstanceEx)、WaitOnAddress 系(足しても CreateThread が答えられない限り同じ場所で止まる。行は api-ms モジュール名で足す必要があり、決定 0081 と衝突する)。

**足すと a4 が変わるもの(注意)**: GetDpiForMonitor、CRT のフォールバックの名前(InitializeCriticalSectionEx 81 回ほか。非 NULL にすると擬似 VA 経由で呼ばれ、`fle_imp_` が無い名前は止まる)、VarBstrFromBool などの RTL 既定の置き換え、InitializeConditionVariable(非 NULL だとローダーが追加で 3 名を取りに行く)。

**読めなかったもの**: E は 0 件。ただし §6 の限界(QFC・ILTools の消費側を個別に全部は読んでいない、delay-import スタブ未計測)のとおり。

決定 0015(応答表に無い名前は NULL＋127)は維持する前提。足す場合の案: `imports_table.rs` の TABLE の**末尾**に行を追記(擬似 VA は動かない。`fle_resp_table` は末尾追加、`win-replay-triage/fix-plan.md` §1.3 と同じ運用)し、名前付き停止の関数ではなく、上の最小集合だけ実装する。

## 8. 成果物

| ファイル | 内容 |
|---|---|
| `getprocaddress-null-impact/classification.tsv` | 90 組の分類(モジュール、名前、回数、呼び出し元、unit、区分、優先度、ローダー、NULL 後の動作、a4 で踏まれたか、実機の経路、Learn の最小 OS、実機の戻り) |
| `getprocaddress-null-impact/classify.py` | 上の TSV の生成(`data/facts.tsv` と `data/learn.tsv` とトレースから) |
| `getprocaddress-null-impact/data/` | `facts.tsv`(名前ごとの根拠)、`learn.tsv`、`loader-fn.tsv`、`consumer-kind.tsv`、`unchecked-call-sites.tsv`、`hits-run1.tsv`/`hits-run3.tsv`(関数入口の計測)、`cells-at-exit-run3.tsv`、`last-error-127-gates.txt`、`pairs90.tsv`、`watch-list.tsv`、`learn-from-mismatch-161.tsv` |
| `getprocaddress-null-impact/scripts/` | 解析スクリプト(逆アセンブルは capstone、PE の読みは pefile。gdb 計測は `gpa_gdb.py`) |

**スクリプトの注意**: `scripts/` のうち解析用(`pedis.py`、`litx.py`、`ripmap2.py`、`relocs.py`、`slots.py`、`consumers.py`、`cells.py`、`uses.py`、`around.py`、`fn.py`、`xref.py`、`apis_of2.py`、`unchecked.py`)は、作業用ディレクトリ `/tmp/opencode/gpa/work/` と `/tmp/opencode/gpa/trace.tsv`(トレースの展開)、DLL の絶対パスを**コードに直書き**している。作業領域は報告後に削除したので、そのままでは動かない(パスの書き換えと、`trace.tsv.gz` の展開が要る)。再生成できない中間ファイルは `data/` に残した(`cells_*.txt`＝名前文字列の参照元とセルの対応、`dumpcells.tsv`＝計測で読むセル一覧、`have_imp.txt`＝計測に使ったバイナリの `fle_imp_*` 名)。`mkfacts.py`(`python3 scripts/mkfacts.py OUT.tsv`。`data/facts.tsv` を再生成)と `classify.py` は、成果物内の入力と `trace.tsv.gz` だけで動く。2 つとも、実行して `data/facts.tsv` と `classification.tsv` とバイト単位で一致することを確かめた [実測]。

再現: `python3 getprocaddress-null-impact/classify.py data/facts.tsv data/learn.tsv ../a4-six-module-trace/trace.tsv.gz ../a4-six-module-trace/summary/getprocaddress.tsv classification.tsv data/loader-fn.tsv`(docs/investigations/ で実行)。計測は `scripts/gpa_gdb.py`(環境変数は先頭のコメント参照)。

## 9. 規則違反の自己申告

- 計測に他の作業者の領域(`lift/out-x1/o1`、`lift/out-fruity6/od2off1`)の native バイナリを gdb で**読み取り専用に**使った(規則は「読み取りのみ」。実行は既存バイナリへの gdb として許された範囲と解した)。実行の出力は `/tmp/opencode/gpa/` に書いた。それらのバイナリは途中で他の作業者により消えた(私の操作ではない)。
- python: `PYTHONDONTWRITEBYTECODE=1` を付けなかった呼び出しが少数ある(標準入力からの `python3 -` や、標準ライブラリだけのスクリプト)。スクリプトの先頭で `sys.dont_write_bytecode = True` を設定した。`/tmp/opencode/gpa` とリポジトリ内の成果物の領域に `__pycache__` が無いことを確認した。site-packages 側は確認していない。
- 外部アクセス: Microsoft Learn の 1 ページ(AvRtCreateThreadOrderingGroupEx。404)。
- `sleep`、`timeout`(コマンド)、`nohup`、末尾の `&` は書いていない(シェルツールの `timeout` パラメータは使った)。`git` の変更系操作・commit はしていない。`git status` の `docs/investigations/README-0101-line.txt` と `lift/iat-unsup-closure.txt`(未追跡)は作業開始時からあった(私のものではない)。
- AGENTS.md の「作業は task agent 経由」は、サブエージェントとして起動され task ツールが無いので、自分で作業した。資格情報・ブラウザのプロファイルには触れていない。
