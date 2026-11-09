# LoadLibrary 系が受け取る名前の分類（FLEngine_x64.dll と dsp_ippv2_x64.dll）

実行日: 2026-10-23〜24。HEAD c68dbce。`lift/runtime/fle_load.c` と `fle_modules.c` は HEAD と作業ツリーが一致（`git show HEAD:… | diff` で確認）。**調査のみ。この文書は何も決めない**（意見は §8 に別枠で置く）。`lift/` 配下のファイルは読み取りだけで、編集していない。作業物は `lift/out-loadlib/`（`git status` に出ていない。ルートの `.gitignore` にこのディレクトリを名指しする行は無く、どの規則で無視されているかは確認していない）。

この作業は 1 回目がレート制限で途中終了している。前回の途中成果物（`lift/out-loadlib/` の約 100 ファイル）は読み取って再利用したが、**結論に使う事実は今回自分で取り直した**（§0.2）。

## 0. 凡例と前提

- **実測** = このセッションで実行した出力。**読み** = 逆アセンブル（capstone の線形走査）とソースの読み。**導出** = Learn の Requirements 欄などから「Windows 10 にある」と推論したもの（ページが Windows 10 にあるとそのまま書いているわけではない。決定 0055 と同じ扱い）。**一次に無い** = 取得した Microsoft Learn のページに書かれていない。**未確認** = 確認していない。
- 外部仕様は Microsoft Learn のみ（取得は 2026-10-23。`lift/out-loadlib/learn/` に生のページ、`learn-recheck-*.txt` に Requirements 欄）。非公式資料（Wine、Geoff Chappell など）は結論に使っていない。検索結果の抜粋に Learn 以外が混じる場合は「参考」と明記した。
- 「unit N」は `units.tsv`（`fe-pe pe-init-table` の出力。0 始まり）の index。ゲストの InitCount は **index + 1**（決定 0060。gdb の InitCount=146 は unit 145）。
- 基準の決定: 0015（仮想モジュール。未知の名前は NULL と 126、ハンドルは `0x7FF800000000 + 0x10000 × N`）、0029（`fle_load_absent[]` の 9 名。それ以外は止める。外部 DLL は M2）、0055（absent に足してよいのは (a) ゲストが NULL を許容、(b) NULL の経路が自然、(c) 環境の定義と矛盾しない、のすべてが確認できたときだけ）、0081（api-ms の 4 名を仮想モジュール N=27..30）、0014（Windows 10 22H2）、0073（dsp_ippv2 を「無い」と定義しない）。
- 依頼文の「決定 0010（外部モジュールは M2）」は、リポジトリの 0010 が「符号拡張、IDIV、1 オペランドの IMUL」で、外部モジュールの決定ではない（決定 0029 も同じ指摘をしている）。「外部モジュールは M2」の根拠は 0046 の方針 5、0029 の判断 2、0073 の決定 5。

### 0.1 現行 runtime の応答（読み。`fle_load.c`、`fle_modules.c`）

- LoadLibraryW/A/ExW は、(1) 実モジュール表（`fle_realmod`。2 モジュール構成のときだけ有効）、(2) 仮想モジュール表 `fle_modules[]`（30 名。N=1..24 が PE の import 表、25 ntdll、26 msctf、27..30 が api-ms の 4 名）、(3) `fle_load_absent[]`（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32 の 9 名）の順に名前を引く。どれにも無ければ `fle_stop`。パス（`\`、`/`、`:`）を含む名前は表を引く前に別の理由で停止する。
- GetModuleHandleW は表に無い名前で**停止せず** NULL と 126 を返す。停止するのは LoadLibrary 系だけ。
- GetProcAddress は、応答表（`fle_resp_table`）の `dll` 欄がそのモジュール名の行だけ擬似アドレスを返し、無ければ NULL と 127。hModule が NULL なら（仮想モジュールのハンドルでないので）**abort**。
- テスト側の固定数（記録のみ。編集していない）: `modules.rs:376` と `msctf.rs:342` が `count 30`、`modules.rs:314-317` が N の値、`load.rs:526-539` の `external_and_unnamed_modules_stop_by_name` が `avrt`、`PowrProf.dll`、`PSAPI.dll`、`RICHED20.DLL`、`URLMON.DLL`、`iphlpapi.dll`、`ws2_32.dll` ほかを「止まる」前提で持つ。`msctf.rs:410` のテスト名 `the_nine_lacking_names_still_fail_with_126` は 9 名を前提にする。`LOAD_MODULES_MAX` は 64。

### 0.2 前回の成果物の再確認（何を取り直したか）

| 事実 | 今回の確認 | 結果 |
|---|---|---|
| FLEngine の呼び出し点 698 行（`sites.py list`） | 再実行し前回の `sites-flengine.tsv` と `cmp` | **同一** |
| インストーラの EXTRACTFILE 27,574 名 | `tools/extract_nsis2.py INSTALLER --list ""` を再実行し `cmp` | **同一**（`nsis-list-recheck.txt`）。`7z l -tNsis` は「Cannot open the file as [Nsis] archive」で開けない（`7z-l-nsis-recheck.txt`）。`7z l`（PE として）は 25 エントリ（.text、.rdata、.rsrc ほか）しか出さず、同梱ファイルの一覧にならない（`7z-l-recheck.txt`）。**展開はしていない** |
| dsp_ippv2 の呼び出し点と名前の表 | `dsp_sites.py`、`dsp_tables.py` を再実行（入力 `lift/out-ext/dsp_ippv2_x64.dll`、SHA-256 `497dd4f7…c352` を確認） | **同一**（LoadLibraryExW 4、GetModuleHandleW 1、GetModuleHandleExW 1、GetProcAddress 3、FreeLibrary 4。表 A 3 名、表 B 19 名） |
| 遅延 import 表 | `delayimp.py` を新規に書いて再読 | **同一**（17 記述子、57 名） |
| gdb による順序 | `run_names.sh` を `CHAIN_MAX=5` と `CHAIN_MAX=6` で再実行（`runs/re5.*`、`runs/re6.*`） | **数値一致**（avrt で停止: init_count 146、関数 935、入口 25,215。6 段: init_count 154、関数 950、入口 26,415） |
| NULL 後の分岐 | 全サイトを `nullwin.py`、`prewin.py`、`dumpfn.py` で読み直し | 前回の表と一致。**差分は 2 件**（iphlpapi は「未確認」→「読み: 検査あり」、PSAPI.dll は §1 の 7） |
| Learn の Requirements 欄 | 10 ページを再取得（`learn-recheck-1.txt`、`-2.txt`）。前回保存の 30 ページと突き合わせ | **一致** |
| runtime の表 | `fle_modules.c`、`fle_load.c` を読み、`tally.py` で数え直し | 仮想 30、absent 9 |
| 静的到達（units 列） | 前回生成の `edges-*.tsv`、`units.tsv`、`reach-sites.tsv`、`ancestors.tsv` を**再利用**（グラフは再生成していない）。gdb の実測と 4 点（Msctf=103、imm32=114、avrt=145、Blend2D=149）で整合を確認 | 再利用（下の限界を参照） |

静的到達の限界: 直接 call と jmp、および `lea reg,[rip+関数]` の辺だけのグラフ。vtable、レジスタ経由、コールバックは辺に無いので、「unit が無い」は「実行されない」を意味しない。`units_direct` は下限、`units_incl_lea` は上限（lea は呼び出しではない）。

gdb の測定は、18:15 ビルドの `lift/out-stage2/two/v-tbl2/native/two-native`（決定 0083 の前のビルド。再ビルドしていない）に、`harness/two/patches_chain.py` の**仮の値**を gdb が入れて進めたもの。仮の値は決定でも runtime の挙動でもない（stage 1〜5 は DllMain を完走させる GetStartupInfoW とロケールの値、stage 6 は指定した名前の LoadLibraryW を NULL にする）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | FLEngine の LoadLibrary 系の呼び出し点 | 直接 call: LoadLibraryW 48、LoadLibraryA 1、LoadLibraryExW 1。ラッパー経由: `f_44db40`（SetErrorMode→LoadLibraryW）27、`f_6539e0`（名前を組み立てて `f_44db40`）13。名前が文字列リテラルで分かる点は **68**（直接 44 + `f_44db40` 13 + `f_6539e0` 11）。名前が静的に決まらない点は 22（直接 6、`f_44db40` 14、`f_6539e0` 2） | 実測（点の数） |
| 2 | 68 点の応答 | 応答済み: 仮想 8 名・35 点、absent 3 名・3 点。**未応答 30 名・30 点**（`external-modules-usage.md` §3 と一致。`tally.txt`） | 実測 |
| 3 | 未応答 30 名の内訳（12 + 14 + 4） | Windows の DLL 12 名（avrt、powrprof、psapi、riched20、urlmon、ws2_32、mswsock、iphlpapi、wship6、fwpuclnt、idndl、normaliz）。インストーラに同名がある同梱 DLL 14 名（dsp_ippv2_x64、webview2loader、beat_tracker、blend2d、quickfontcache_x64、ilsvg_x64、ildsp_x64、oggio_x64、iltools_x64、wavpackdll_x64、lameenc_x64、python312、ilvideo_x64、speaker_x64）。インストーラに無い FL の DLL 4 名（declipper、stemextract、ds2wav、ss2wav） | 実測（インストーラ）＋読み（名前の分類） |
| 4 | ゲストが NULL を許容するか（30 名） | 許容 22、許容だが注記つき 2（riched20、iltools_x64）、**許容しない 6**（ws2_32、mswsock、stemextract、beat_tracker は NULL で RaiseException、urlmon は NULL のまま使うと RaiseException、ilsvg_x64 は NULL 検査なしで呼ぶ） | 読み |
| 5 | DllMain 後に最初に止まる名前（実測） | unit 145 の **avrt**、続いて同じ関数の **PowrProf.dll**、unit 149 の **Blend2D.dll**。その次の停止は LoadLibrary ではなく unit 153 の RegCreateKeyExW。それ以降は未測定 | 実測（仮の値つき） |
| 6 | dsp_ippv2_x64.dll 側 | LoadLibrary 系は LoadLibraryExW 4 点・名前 5 つ（api-ms の 4 名と kernel32）。**すべて現行 runtime で応答済み**（決定 0081、仮想モジュール N=27..30 と N=9）。未応答の LoadLibrary 名は無い | 実測＋読み |
| 7 | **PSAPI.dll の LoadLibraryW は、エミュレートした Windows 10 では実行されない**（読み） | `f_426310` は VerifyVersionInfoW（6.1 以上か）を見て、真なら `KERNEL32.dll`（K32 版の関数を引く）、偽のときだけ `PSAPI.dll` を引く。決定 0014 の 10.0 では真。実行して確かめていない | 読み（§2.3） |
| 8 | 遅延 import 経由の LoadLibraryA（`f_41d9a0`） | 名前は遅延 import 表の 14 DLL。**すべて応答済み**（仮想 5、absent 9）。ただし absent の 9 名は、遅延 import の API が呼ばれて NULL になると `RaiseException(0xc06d007e)`（runtime では名前付き停止）に進む形（読み） | 読み |
| 9 | Windows 10 の標準 DLL と言える名前 | Requirements 欄に DLL 名があり「Vista 以降」などと書くもの（導出）: avrt、powrprof、ws2_32、mswsock、iphlpapi、fwpuclnt、normaliz。**弱い**: psapi（Windows 10 の記述なし）、urlmon（archive ページ）。**一次に無い**: riched20、wship6、idndl（Learn は idndl を「Vista より前の OS 向け。ダウンロードが要る」、wship6 を「Windows 2000 の IPv6 Technology Preview のファイル」と書く） | 実測（Learn 取得）＋導出 |

## 2. FLEngine_x64.dll の呼び出し点（項目 1）

### 2.1 方法

- 入力 `binaries/win64/FLEngine_x64.dll`（SHA-256 `b2c64da4…496d`、`win64-extraction.md` と一致を確認）。`.text` を線形逆アセンブルし、kernel32 の IAT thunk（`jmp [rip+disp]` で IAT 名が分かるもの）と、ラッパー `f_44db40`、`f_6539e0` への直接 call を全部拾った（`sites.py list`）。
- 各点の直前 60 バイト以内の `lea reg,[rip+X]` が指す UTF-16 / ASCII 文字列をリテラルとして付けた。呼び出し直後の分岐は 1 点ずつ `nullwin.py`（call から 22 命令）と `prewin.py` で読んだ。
- `f_6539e0`（dl=1 のとき名前の末尾が `_x64.dll` でなければ `_x64.dll` を付け、`f_653660` が 4 つのディレクトリ候補を `GetFileAttributesW` で探し、見つかればパス付き、無ければ名前だけを返し、`SetDllDirectoryW` と `SetErrorMode(0x8000)` を挟んで LoadLibraryW）の**最終文字列は読みで、実測は dsp_ippv2 の 1 例だけ**（裸の `dsp_ippv2_x64.dll`。`after-comparestringw.md`）。他の名前の最終文字列は、`_x64.dll` の付与とパス前置の有無が未確認。特に `Python312`（`r8` に `Python\` を渡す）と `ffmpeg\x64\ilvideo_x64.dll`（リテラルが `\` を含む）は、LoadLibraryW に渡る文字列が `\` を含み、`fle_load.c` がパスを理由に別の停止をする可能性がある（読み。未実測）。

### 2.2 未応答 30 名の表（1 名 1 行。unit の若い順）

列: 名前（ゲストが渡す文字列 → LoadLibraryW に届く名前の読み）／呼び出し点の VA と経路／ゲストの NULL 時の分岐（読み）／静的に届く unit（下限 → 上限）／null。`null` 欄の「許容」は、NULL の後に停止・例外・クラッシュの経路が無いこと。

表中の「例外」「0x411e80」は、`f_411e80`（0x411e80）が `f_411d20` を呼び、`f_411d20` が `[0x15e6030]` を `call` する形を指す（`ecx = 0x0EEDFADE`、`edx = 1`（EXCEPTION_NONCONTINUABLE）、`r8d = 8`、`r9` = 引数配列。0x411e30〜0x411e47）。`[0x15e6030]` に書かれるのは thunk 0x406fd0 で、`ImportName` を引くと `kernel32.dll!RaiseException`（実測。`prewin.py` で書き込み側 0x41ce5e と 0x4121f4 を読み、`thunks()` で名前を確認）。したがって**これらの点は NULL で実際に `RaiseException` に届く**。runtime の `fle_imp_RaiseException`（`fle_proc.c:255`）は、構造化例外を持たないので名前付きで止める。`0x448d40` はメッセージ文字列を持つ例外オブジェクトを作る関数（読み）。

| # | 名前 | 呼び出し点と経路 | ゲストの NULL 時の分岐（読み） | 静的に届く unit | null |
|---|---|---|---|---|---|
| 1 | `dsp_ippv2_x64.dll`（`dsp_ippv2.dll`、dl=1） | 0x653f65 `f_6539e0` ← `f_653f40` ← `f_65c540`（unit 144 の proc、0x65c554）、0x680648、0x11b285c | 0x653f79 `jne`、NULL は `xor ebx,ebx` で 0x6572c7（false）。3 呼び出し元は戻り値を使わない | 144（直接 144、745、895、898） | 許容 |
| 2 | `avrt`（→ `avrt.dll`） | 0x65ccd1 LoadLibraryW ← `f_65c9f0` | 0x65cce5 `je 0x65ce09`（11 名の GetProcAddress を飛ばす）。使用側は読んだ 8 関数すべてがスロットか戻りのハンドルを検査（§2.4） | 145 | 許容 |
| 3 | `PowrProf.dll` | 0x65cf38 LoadLibraryW ← `f_65c9f0` | 0x65cf4c `je 0x65cf82`（2 名を飛ばす）。使用側 `f_65c6c0` は 0x65c6ca で `cmp [0x15f4258],0; je` | 145 | 許容 |
| 4 | `Blend2D.dll` | 0x6641b9 `f_6539e0`（dl=0）← `f_664190` ← `f_664260`（unit 149）、0x6cf2a0（unit 190 の proc の呼び出し元） | 0x6641cd `je`、`AggCreateInstance` のスロットは 0 のまま `setne al` で false | 149、190 | 許容 |
| 5 | `QuickFontCache.dll`（→ `quickfontcache_x64.dll`、dl=1） | 0x665bed `f_6539e0` ← `f_665bd0` | 0x665c01 `setne bl; test; je 0x665c56`（false） | 158、245、279、647 | 許容 |
| 6 | `ILTools`（→ `iltools_x64.dll`、dl=1） | 0x815e33 `f_6539e0` ← `f_815e10` | **NULL 検査なしで** `GetProcAddress(hmod,"CreateCursorFromFont")`。その後 0x815e60 でスロットを検査し、0 なら `OutputDebugStringW("Warning: shared cursor library could not be loaded.")` を出して別経路 | 245、279、280、647、659 | 許容だが注記（下の注 1） |
| 7 | `ildsp`（→ `ildsp_x64.dll`、dl=1） | 0x66ea2a `f_6539e0` ← `f_66ea10` | 0x66ea36 `cmp [0x13e8ed0],0; setne al`（false） | 直接 895、898。lea 込み 614、745、880、895、898 | 許容 |
| 8 | `Python312`（`r8` = `Python\`、dl=0） | 0xc395c5 `f_6539e0` ← `f_c395a0` | 0xc395d9 `jne`、NULL は `xor eax,eax`（false） | 直接 895、898。lea 込み 614 以降 | 許容 |
| 9 | `ss2wav.dll` | 0xf360bc `f_44db40` ← `f_f36060` | 0xf360d0 `jne`、NULL は `or eax,-1`（-1 を返す） | 直接 895、898 | 許容 |
| 10 | `MSWSOCK.DLL` | 0xc12afb `f_44db40` ← `f_c12ab0` | 0xc12b0f `jne 0xc12b66`。NULL は `GetLastError`、メッセージ生成（0xc07b60）、`0x411e80`（`RaiseException` 系）へ | lea 込みのみ 575 | **許容しない**（例外） |
| 11 | `ffmpeg\x64\ilvideo_x64.dll` | 0x66d255 `f_6539e0`（dl=0）← `f_66d230` | 0x66d269 `jne`、NULL は `xor ebx,ebx`（false） | lea 込みのみ 614、745、880、895、898 | 許容 |
| 12 | `oggio.dll`（→ `oggio_x64.dll`、dl=1） | 0x675334 `f_6539e0` ← `f_675310` | 0x675348 `je 0x675449`（GetProcAddress を飛ばす） | lea 込みのみ 614 以降 | 許容 |
| 13 | `WavpackDLL_x64.dll` | 0xa58bbd `f_6539e0` ← `f_a58ba0` | 0xa58bd1 `setne al` を保存し 0xa58be1 `je 0xa5916b`（50 名を飛ばす） | lea 込みのみ 614 以降 | 許容 |
| 14 | `beat_tracker.dll` | 0x11336a4 LoadLibraryW ← `f_1133660`（名前は `f_6535a0` の既定ディレクトリに連結） | 0x11336b8 `jne`。NULL は `"Could not load BeatTracker module."`（0x448d40）と 0x411e80（例外） | なし（根 1、vtable の可能性。未確認） | **許容しない** |
| 15 | `PSAPI.dll` | 0x4263e1 LoadLibraryW ← `f_426310` ← 0x4264f4（GetMappedFileNameW）、0x426571（GetProcessMemoryInfo） | 0x426402〜0x426411: NULL なら false を返し、呼び出し側は API を無いものとして扱う。**この点は state==1（OS が 6.1 未満）の分岐だけ**（注 2） | なし | 許容 |
| 16 | `ILSvg`（→ `ilsvg_x64.dll`、dl=1） | 0x66dc2a `f_6539e0` ← `f_66dc10` ← `f_66dcf0`（0x66dd2b） | **NULL 検査なし**。0x66dc36 で `GetProcAddress(slot,"svgCreate")`、直後に 0x66dd5b `call [0x15f4758]` | なし（根 1） | **許容しない**（注 3） |
| 17 | `RICHED20.DLL` | 0x6d644d LoadLibraryW ← `f_6d6430` | 0x6d6459 `cmp [0x15f6020],0x20; ja`。0x20 以下は 0 を保存。**どちらでも** 0x6d6474 以降の `RICHEDIT20W` 窓クラスの処理へ進む | なし（根 1） | 許容だが注記（注 4） |
| 18 | `URLMON.DLL` | 0x707bfa LoadLibraryW ← `f_707bd0`（CopyStgMedium のラッパー） | 0x707c0e `je 0x707c2a`。0x707c2a でスロット `[0x13f3e80]` が 0 なら 0x41c820、0x448d40、0x411e80（例外）。**ロードの時点では許容、使うと例外** | なし（根 2） | **許容しない**（呼ばれれば） |
| 19 | `LAMEEnc.dll`（→ `lameenc_x64.dll`、dl=1） | 0xa5d914 `f_6539e0` ← `f_a5d8f0` | 0xa5d928 `je 0xa5d9b2`（5 名を飛ばす） | なし（根 5） | 許容 |
| 20 | `Speaker_x64.dll` | 0xa65b24 `f_44db40` ← `f_a65ac0` | 0xa65b3d `jne`、NULL は `[rbp+0x4b]=0` で false | なし（vtable 等の可能性。データ参照の祖先 52） | 許容 |
| 21 | `ds2wav.dll` | 0xb953d6 `f_44db40` ← `f_b95380` | 0xb953ea `jne`、NULL は `or eax,-1` で -1 | なし（データ参照の祖先 51） | 許容 |
| 22 | `WebView2Loader.dll` | 0xbe3539 LoadLibraryW ← `f_be3520` | 0xbe354d `jne`、NULL は `xor eax,eax`（0 を返す） | なし（根 18） | 許容 |
| 23 | `WS2_32.DLL` | 0xc07a0f `f_44db40` ← `f_c079e0`（WSAStartup のローダー） | 0xc07a23 `je 0xc07a52`。NULL は `GetLastError`、メッセージ、0xc07b60、0x411e80（例外） | なし（根 1） | **許容しない** |
| 24 | `Wship6.dll` | 0xc14b47 `f_44db40` ← `f_c14b00` | **ws2_32 の `GetAddrInfoW` が NULL のときだけ**（0xc14b39 `jne 0xc14b77`）。NULL は `GetProcAddress(NULL,…)` → `je 0xc14d16`（内蔵の代替関数を使う） | なし（根 1） | 許容（注 5） |
| 25 | `Fwpuclnt.dll` | 0xc14c5e `f_44db40` ← `f_c14b00` | 0xc14c72 `je 0xc14d4c`（6 名を飛ばす） | なし（根 1） | 許容 |
| 26 | `IdnDL.dll` | 0xc171fa `f_44db40` ← `f_c171e0` | 0xc1720e `je 0xc1725e`（3 名を飛ばす） | なし（根 1） | 許容 |
| 27 | `Normaliz.dll` | 0xc17278 `f_44db40` ← `f_c171e0` | 0xc1728c `je 0xc17314`（5 名を飛ばす） | なし（根 1） | 許容 |
| 28 | `iphlpapi.dll` | 0xc192fa `f_44db40` ← `f_c192e0` | ローダーに検査なし。結果は `[0x14fee78]` に保存。**使用側 6 関数**（0xc18f40、0xc18fe0、0xc19090、0xc19130、0xc19200、0xc19340）が全部 `cmp [0x14fee78],0; je` で NULL を見て、内蔵の代替関数へ（今回読んだ） | なし（根 1） | 許容 |
| 29 | `declipper.dll` | 0xca1008 LoadLibraryW ← `f_ca0f70`（名前を `f_414c70` で組み立て、先に `f_43ba70`（GetFileAttributesW）で存在確認。**無ければ LoadLibraryW に届かず** `al=1` で戻る。0xca0fb5〜0xca0fc6） | 0xca1028 `jne`、NULL は `al=2` で戻る | なし（根 7） | 許容 |
| 30 | `stemextract.dll` | 0xcad9c4 LoadLibraryW ← `f_cad970` | 0xcad9e4 `jne`。NULL は `"Could not load stem extractor module."`（0x448d40）と 0x411e80（例外） | なし（根 2） | **許容しない** |

注:
1. #6 ILTools: ゲストは NULL の後も `GetProcAddress(NULL,…)` を呼ぶ。現行 runtime は hModule が仮想ハンドルでないと abort するので、ゲストが許容しても **runtime 側が別の名前付き abort をする**（`fle_modules.c:199-205`）。Learn の GetProcAddress ページは hModule=NULL の結果を書かない。
2. #15 PSAPI: §2.3。
3. #16 ILSvg: 読んだ範囲で `f_66dcf0` は `svgCreate` のスロットを検査せずに呼ぶ。上位の呼び出し側が別のフラグで守るかは未確認。
4. #17 RICHED20: ロードの結果で窓の作成が変わるわけではなく、NULL（または 0x20 以下）でも `RICHEDIT20W` の窓を作りに行く（読み）。NULL を許容する設計とは言いにくい（窓クラスが無いときの CreateWindowEx の失敗の扱いは読んでいない）。runtime は決定 0070、0071 で `RICHEDIT20W` を名前付きで止める。
5. #24 Wship6: ws2_32 が仮想モジュールで、`GetProcAddress(ws2_32,"GetAddrInfoW")` が NULL（応答表に行が無い）ならこの経路に入る。そこで `GetProcAddress(NULL,…)` は runtime で abort。

**表の裏付けの強さ**: #1〜#14、#18〜#30 は今回 `nullwin-1.txt`、`nullwin-2.txt`、`prewin` の出力で call 直後を読み直した。#2 と #3 は使用側（`slot_users.tsv` の 23 reader のうち、`bb0920`、`bb1520`、`12b00f0`、`12b0190`、`65c6c0` を今回読み直した）まで読んだ。#16 と #17 は呼び出し側の関数の一部だけ読んだ。

### 2.3 PSAPI.dll（#15）の経路（読み）

`f_426310`（GetMappedFileNameW と GetProcessMemoryInfo のラッパーが共有する遅延ローダー、0x426310）の読み:

1. 状態変数 `[0x15ec328]` が 0 のとき一度だけ、`VerifyVersionInfoW` で「major 6、minor 1（Windows 7）以上」を問う（`dwTypeMask` 0x23、`VerSetConditionMask` を 3 回: major=条件 3、minor=条件 3、SP major=条件 3。`0x426356〜0x4263bd`）。真なら状態 2、偽なら状態 1。
2. ハンドル `[0x15ec320]` が 0 のとき、状態 1 なら `LoadLibraryW(L"PSAPI.dll")`（0x4263e1）、状態 2 なら `LoadLibraryW(L"KERNEL32.dll")`（0x4263f6）。状態 2 では名前に `K32` を前置して引く（0x426436〜0x42645d）。
3. 決定 0014 の OS は 10.0.19045 で、`fle_osver_check` は major 10 > 6 なので真を返す（runtime の `fle_osver.c`）。したがって**この環境では 0x4263e1 の PSAPI.dll は実行されない**（読み。`VerifyVersionInfoW` が実際に真を返したことを gdb で確認していない。決定 0014 にある「実行がこの結果で先へ進んだ」は 0x409bd0 の別の呼び出しの記述）。

0x4263f6 の `KERNEL32.dll` は仮想モジュール（N=9）で応答済み。この点の GetProcAddress は `K32GetMappedFileNameW`、`K32GetProcessMemoryInfo`（名前は `"GetMappedFileNameW"`、`"GetProcessMemoryInfo"` の前に `K32` を連結した文字列、0x426548、0x4265c0）で、**応答表に行が無い**（実測: `imports_table.rs` を grep して `K32`、`GetMappedFileName`、`GetProcessMemoryInfo` は 0 件。`dll` 欄の内訳は kernel32 173、user32 28、oleaut32 18、advapi32 12、winmm 11、ole32 10、gdi32 6、msctf 1、imagehlp 1 で、avrt、powrprof、psapi、ws2_32 の行は無い）。行が無いので NULL と 127 になり、`f_426310` は 0x426462〜0x42646c で false を返し、呼び出し側はこの API を無いものとして扱う（読み）。呼び出し元 2 関数（0x4264f4、0x426571）が実行されるかは、静的な unit が無い（`reach-sites.tsv` の `-`）ため未確認。

### 2.4 avrt（#2）と PowrProf（#3）の NULL 経路の読み

- `f_65c9f0`（unit 145 の proc。入口で `sub [0x15f4268],1; cmp -1; jne` の一度だけ初期化）は、LoadLibraryW/GetModuleHandleW の結果を保存し、同じ場所の `cmp [..],0; je` で、そのモジュールの GetProcAddress を全部飛ばす（7 点: dwmapi、user32 の GetModuleHandleW、avrt、kernel32 の GetModuleHandleW、advapi32、PowrProf、SHCore。今回 `nullwin-1.txt` で全点を読んだ）。
- avrt のスロット 11 個（0x15f40f0〜0x15f4140）の使用側（`slot_users.tsv` の reader 23 行）: 直接読みは終了側 `f_65c750` の「スロットを 0 にする」書き込み 11 行（読みではない）。実際に呼ぶのは 8 関数で、今回読み直した `f_bb0920`（0xbb095b で `cmp qword ptr [rax],0; je`、0xbb0962）、`f_12b00f0`（0x12b0125 で同じ検査）は呼ぶ前に検査する。`f_bb1520` と `f_12b0190`（AvRevertMmThreadCharacteristics）は `[rbx+0x48]` が非 0 のときだけスロットを読む（`test rax,rax; je`）。`[rbx+0x48]` は検査付きの `AvSetMmThreadCharacteristicsW` の結果を入れる場所で、それが 0 のままなら呼ばれない。全 reader の網羅は `slot_users.py` の範囲（rip 相対とデータセル）。基底からのオフセットで読む形は走査していない。
- PowrProf のスロット 2 個: `f_65c6c0` が 0x65c6ca で `PowerSetActiveScheme` のスロットを検査してから使う。`PowerGetActiveScheme`（0x15f4250）は同じ関数の中で、先の検査を通った後にだけ到達する。
- 終了側 `f_65c750` は 5 つのハンドルを `cmp [..],0; je` の後で FreeLibrary する（NULL なら呼ばない）。
- 実測（仮の NULL）: avrt、powrprof、blend2d を NULL にして進めた run（`runs/re6.*`）で、クラッシュ（SIGSEGV など）は出ていない。停止は RegCreateKeyExW まで。

## 3. dsp_ippv2_x64.dll の呼び出し点（項目 1）

入力 `lift/out-ext/dsp_ippv2_x64.dll`（SHA-256 `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352`。`sha256sum` で再確認）。import は KERNEL32.dll の 72 名だけ。`LoadLibraryW`、`LoadLibraryA`、`GetModuleHandleA` は import に無い。`dsp_sites.py`（FF 15 / FF 25 の全バイト位置走査＋ capstone の復号確認）の結果を、`dsp-ippv2-dynamic-imports.md` の §2、§3 と突き合わせた（件数と RVA は一致）。

| # | 点の VA（リベース前 0x180000000 基準） | API | 名前 | 呼ばれる経路 | NULL 時の分岐（読み。既存の調査と一致を確認） | 現行 runtime の応答 |
|---|---|---|---|---|---|---|
| D1 | 0x1817d080d（dwFlags 0x800）、0x1817d0845（dwFlags 0） | LoadLibraryExW | 表 A の添字: `api-ms-win-core-fibers-l1-1-1`、`api-ms-win-core-synch-l1-2-0`、`kernel32`（名前に拡張子なし） | ラッパー A（0x1817d0788）。呼び出し元 5（FlsAlloc、FlsFree、FlsGetValue、FlsSetValue、InitializeCriticalSectionEx）。DllMain の ATTACH で到達（実測。`runs/re6.log` の 48、49、50） | 0x1817d0816 `test rax,rax; jne`。NULL のとき GetLastError が 0x57 かつ名前が `api-ms-` で始まらなければ dwFlags 0 で再試行。それでも NULL なら次のモジュール（`kernel32`）へ | 応答済み: api-ms の 3 名は仮想 N=27..29、kernel32 は N=9 |
| D2 | 0x1817d739f（0x800）、0x1817d73f3（0） | LoadLibraryExW | 表 B の添字: `api-ms-win-core-synch-l1-2-0`（6）、`api-ms-win-core-localization-l1-2-1`（2）、`kernel32`（15）、`api-ms-win-appmodel-runtime-l1-1-2`（17）。表 B の他の 15 名は呼び出しの添字に使われない | ラッパー B（0x1817d72fc）。呼び出し元 5（AppPolicyGetProcessTerminationMethod、AreFileApisANSI、InitializeCriticalSectionEx、LCMapStringEx、LocaleNameToLCID）。ATTACH で到達（実測: synch、localization、kernel32 の 3 名。appmodel は引かれなかった） | 同上。`AppPolicyGetProcessTerminationMethod` と `AreFileApisANSI` は 1 名だけのリストで、失敗時は固定値（0xc0000225、1） | 応答済み: 仮想 N=27、29、9、30 |
| D3 | 0x1817d19b3 | GetModuleHandleW | NULL | 終了処理の中心関数 0x1817d198c。DataDirectory[14] が非 0 のときだけ先へ進む。FLEngine は 0（`dsp-ippv2-dynamic-imports.md` §2.5） | — | 応答済み（0x400000）。ATTACH では呼ばれない（実測） |
| D4 | 0x1817d1add | GetModuleHandleExW | `L"mscoree.dll"` | `CorExitProcess` の探索。終了経路だけ | — | 名前付き停止（決定 0078）。ATTACH では呼ばれない |
| D5 | 0x1817d088b、0x1817d747b、0x1817d1b1a、0x1817d7787 | FreeLibrary | 直前の LoadLibraryExW の戻り、他 | ラッパー A、B の「他スレッドが先にキャッシュした」場合など | ATTACH では参照 0 の FreeLibrary に届かない（実測 `stage23` §4） | — |

- GetProcAddress 3 点（0x1817d0897、0x1817d7487、0x1817d1af3）の名前は 9 種＋`CorExitProcess`（`dsp_tables.py` で再確認）。
- 結論: **dsp_ippv2 が LoadLibrary 系に渡す名前は 5 つで、すべて現行 runtime が答えられる**。未応答の LoadLibrary 名は無い。DllMain は 160 回のモジュール内 import で戻る（`stage23` §4。仮の値 5 段つき）。

## 4. 段階 2・3 で見えた名前（項目 1 の続き）

`stage23-two-module-real-dll.md` §5.2 と、今回の再実行（`runs/re5.*`、`runs/re6.*`）。`GetModuleHandleW` と LoadLibrary 系を InitCount つきで並べる。`dsp_ippv2_x64.dll` の LoadLibraryW は DllMain の入口（`f_37caca8`）の前、InitCount 145。

| 順 | gdb の行 | InitCount（= unit + 1） | 名前 | 応答 | 備考 |
|---:|---|---:|---|---|---|
| — | 47 | 145（unit 144） | `dsp_ippv2_x64.dll` | 実モジュール表（2 モジュール構成）。DllMain を ATTACH で実行 | FLEngine 単独構成ではここで名前付き停止（`stage23` §8）。この表は 2 モジュール構成のビルド（決定 0083 の前）で、`CHAIN_MAX=0` では DllMain の中の GetStartupInfoW で止まる（`runs/base0.err`） |
| 1 | 48〜66 | 145 | DllMain 内の LoadLibraryExW 5 回（§3 の D1、D2） | 応答済み | 全部 dwFlags 0x800 |
| 2 | 917 | **146（unit 145）** | `dwmapi` | absent（NULL、126） | 既存 |
| 3 | 918 | 146 | GetModuleHandleW(`user32`) | 仮想 N=19 | 停止しない |
| 4 | 921 | 146 | **`avrt`** | **停止**（仮の NULL で通過） | `0x65ccd6` が戻り |
| 5 | 922 | 146 | GetModuleHandleW(`kernel32`) | 仮想 N=9 | SRWLock 7 名は応答表に無く NULL（読み） |
| 6 | 937 | 146 | `advapi32` | 仮想 N=1 | |
| 7 | 942 | 146 | **`PowrProf.dll`** | **停止**（仮の NULL で通過） | |
| 8 | 943 | 146 | `SHCore.dll` | absent（NULL、126） | 既存 |
| 9 | 944 | **150（unit 149）** | **`Blend2D.dll`** | **停止**（仮の NULL で通過） | 呼び元 `f_664190` ← `f_664260`（unit 149 の proc。`units.tsv` で確認） |
| — | — | 154（unit 153） | （LoadLibrary ではない）RegCreateKeyExW | 停止（決定が要る別件） | `f_422ff0` ← `f_5af5e0` ← `f_664fd0` ← `f_665b90` |

表の gdb の行番号と InitCount は `runs/re6.log` から取った（再実行で一致）。**unit 番号の食い違い**: `stage23-two-module-real-dll.md` §5.2 は Blend2D を「unit 150」と書くが、gdb の `InitCount=150` は index 149 を指す（InitCount = index + 1。決定 0060）。`units.tsv` の index 149 の proc は 0x664260 で、Blend2D の呼び出し元 `f_664260` と一致する。停止する関数は同じで、番号の数え方だけの差（stage23 の記述は InitCount をそのまま unit と書いたものと読める。未確認）。同じ理由で avrt と PowrProf は InitCount 146 = unit 145（`f_65c9f0`）、RegCreateKeyExW は InitCount 154 = unit 153（proc 0x665b90）。

## 5. 名前ごとの 3 点（項目 2）

(i) Windows 10 の標準 DLL か（Learn の Requirements 欄）、(ii) インストーラに同名のファイルがあるか（EXTRACTFILE の名前の末尾の照合。`presence.tsv`）、(iii) ゲストが NULL を許容するか（§2.2 の読み）。

### 5.1 (i) と (ii)

「導出」は、Requirements 欄の DLL 名と最小対応 OS から「Windows 10 にある」と言うもので、**ページが Windows 10 に必ずあると書いているわけではない**（決定 0055 と同じ）。

| 名前 | (i) Learn の Requirements 欄 | 区分 | (ii) インストーラ |
|---|---|---|---|
| avrt.dll | `AvSetMmThreadCharacteristicsW`: DLL = Avrt.dll、クライアント最小 Windows Vista | 導出 | なし |
| powrprof.dll | `PowerGetActiveScheme`、`PowerSetActiveScheme`: DLL = PowrProf.dll、Vista | 導出 | なし |
| psapi.dll | `GetProcessMemoryInfo`: DLL 欄は「Kernel32.dll on Windows 7 and Windows Server 2008 R2; Psapi.dll (if PSAPI_VERSION=1) on Windows 7 …; Psapi.dll on Windows Server 2008, Windows Vista, Windows Server 2003 and Windows XP」。本文は「PSAPI_VERSION が 1 のとき `GetProcessMemoryInfo` は Psapi.lib と Psapi.dll にあり、`K32GetProcessMemoryInfo` を呼ぶラッパー」と書く。Windows 10 の記述は無い | 導出が弱い（記述は Windows 7 まで。Windows 10 に Psapi.dll があるとは言えない） | なし |
| riched20.dll | `about-rich-edit-controls`: Rich Edit 2.0 と 3.0 の DLL は Riched20.dll、4.1 は Msftedit.dll。**ページは OS ごとの有無を書かない** | **一次に無い** | なし |
| urlmon.dll | `CopyStgMedium`（Learn の archive ページ `previous-versions/…/ms775093(v=vs.85)`）: DLL = Urlmon.dll、クライアント最小 Windows XP、製品 IE 4.0。archive ページなので「Windows 10 に今もある」とは言えない | 導出が弱い（archive ページ）。前回の通常版 URL（`nf-urlmon-copystgmedium`）は 404 だったので、今回 archive 版を取得（`learn/ms775093(v=vs.85).md`） | なし |
| ws2_32.dll | `WSAStartup`: DLL = Ws2_32.dll、Windows 8.1 / Vista（クライアント）。`getaddrinfo`: DLL = Ws2_32.dll、Windows XP / 8.1 | 導出 | なし |
| mswsock.dll | `WSARecvEx`: DLL = Mswsock.dll、Windows 2000 Professional | 導出 | なし |
| iphlpapi.dll | `GetAdaptersInfo`: DLL = Iphlpapi.dll、Windows 2000 Professional。`GetAdaptersAddresses`: Iphlpapi.dll、Windows XP | 導出 | なし |
| fwpuclnt.dll | `WSASetSocketSecurity`: DLL = Fwpuclnt.dll、Windows Vista | 導出 | なし |
| normaliz.dll | `IdnToAscii`、`NormalizeString`: DLL = Normaliz.dll、Windows Vista（`api_location` は kernel32.dll、KernelBase.dll ほかも併記） | 導出 | なし（部分一致が 1 件: `Transparent normalizer.fst`。別物） |
| wship6.dll | Learn の `getaddrinfo` ページ本文: 「the Wship6.dll (the file containing getaddrinfo in the IPv6 Technology Preview for Windows 2000)」。Requirements 欄に wship6 は無い | **一次に無い**（Windows 2000 向けの記述だけ） | なし |
| idndl.dll | `DownlevelVerifyScripts`、`DownlevelGetLocaleScripts`: DLL = Idndl.dll、「This function is used only by applications that run on pre-Windows Vista operating systems. Its use requires the download package.」Redistributable = Microsoft Internationalized Domain Name (IDN) Mitigation APIs | **Windows 10 の標準ではない（Learn が Vista より前の OS 向けの再頒布物と書く）** | なし |
| dsp_ippv2_x64.dll | Windows の DLL ではない | — | あり（エントリ 56484、`Shared` 配下。**実測**） |
| webview2loader.dll | Learn（Edge の WebView2 のページ）: 「`WebView2Loader.dll` … needs to be shipped with the app」。Windows の標準ではない | 一次（アプリが同梱する） | あり（エントリ 954 と 57956。同じデータオフセット 0xa56492） |
| beat_tracker.dll、blend2d.dll、quickfontcache_x64.dll、ilsvg_x64.dll、ildsp_x64.dll、oggio_x64.dll、iltools_x64.dll、wavpackdll_x64.dll、lameenc_x64.dll、python312.dll、ilvideo_x64.dll、speaker_x64.dll | Windows の DLL ではない | — | あり（`presence.tsv`。`LAMEenc_x64.dll` など大文字小文字が違うものを含む） |
| declipper.dll、stemextract.dll、ds2wav.dll、ss2wav.dll | 名前から FL / Image-Line の DLL と読めるが、**出自は未確認**。Windows の DLL ではない（Learn に無い） | 一次に無い | **なし**（EXTRACTFILE に無い。別の入手経路は未確認） |

`ilsvg.dll`、`ildsp.dll`、`oggio.dll`、`iltools.dll`、`wavpackdll.dll`、`lameenc.dll`（`_x64` なし）は EXTRACTFILE に無く、`_x64` 付きがある（`presence.tsv`）。`f_6539e0` が dl=1 で付ける読み（§2.1）と整合する。

インストーラに同名のファイルがある 14 名（dsp_ippv2_x64、webview2loader と、上の表の最後から 2 行目に並べた 12 名）は M2（外部モジュール）。ただし**インストーラにあることは、ゲストが実行時にそのファイルを見つけることを意味しない**（`f_6539e0` が探す 4 つのディレクトリ候補は読んでいない。ゲストは `Shared\` を SetDllDirectoryW で渡す）。

### 5.2 インストーラの同梱物について（実測の範囲）

- `python3 tools/extract_nsis2.py INSTALLER --list ""` の EXTRACTFILE は 27,574 件。DLL（`.dll` で終わる名前）は 406 件。`.msi`、`.cab`、`.zip`、`.7z`、`.rar` は `python312.zip` の 1 件だけ（OS の再頒布パッケージは無い）。vcruntime140.dll と vcruntime140_1.dll はある（エントリ 56704、56706）。
- Windows の標準 DLL（avrt、powrprof、psapi、urlmon、ws2_32 など）は同梱されていない。**これは「これらは OS が持つ前提で、インストーラは配らない」という読みであり、確認はしていない**。
- 別インストーラ（EXTRACTFILE に `.exe` が 30 件あり、`FL Cloud Plugins Installer.exe` を含む）、ダウンロード、実行時に配置される DLL は未確認。
- `7z l` で EXTRACTFILE を直接列挙する再現は、この 7-Zip 26.00 では NSIS として開けずできなかった。既存の `tools/extract_nsis2.py`（標準ライブラリだけ。読み取り専用）で列挙した。依頼の「`7z l` で読み取りのみ」は、この方法で代替した。

## 6. 選択肢の表（項目 3。**決めない**）

選択肢: **(a)** 仮想モジュール表の末尾（N=31 以降）に足す。LoadLibrary は非 NULL、GetProcAddress は応答表に行が無ければ NULL と 127（決定 0015、0055、0081 の形）。**(b)** `fle_load_absent[]` に足す（NULL と 126）。**(c)** 停止のまま。**(d)** 外部モジュールとして lift する（M2。決定 0073、0082 の形。実在の DLL が要る）。

基準の当てはめ:
- (b) の条件は決定 0055 の (a) ゲストが NULL を許容、(b) NULL の経路が自然、(c) 環境の定義（Windows 10 22H2、決定 0014）と矛盾しない、のすべて。下の表の「(b) の (a)(b)」は §2.2 の読み、「(b) の (c)」は §5.1 の (i)。
- (a) の条件は、決定 0055 の流儀（「Windows 10 には必ずある」と言える。Learn が存在を明記していなければ「導出」と書く）。
- (d) は dsp_ippv2 だけが決定 0073 で方向が決まっている。他は実在の DLL がインストーラにあるか（§5.1 の (ii)）が前提条件。

### 6.1 Windows の DLL 12 名

| 名前 | (a) 仮想 | (b) absent | (c) 停止のまま | (d) lift |
|---|---|---|---|---|
| avrt | 可（導出: Requirements に Avrt.dll、Vista 以降）。GetProcAddress の 11 名は応答表に行が無いので全部 NULL と 127。**ゲストは 11 名ぶんのスロットが 0 のまま進む**（§2.4。読み）。つまり (a) は avrt が「ある」と答えるが、機能は全部無い状態になる | (a)(b) は満たす（§2.2 #2、§2.4）。**(c) は確認できない**: Avrt.dll が Vista 以降にあるという導出と、「無い」という定義が矛盾する（決定 0055 が msctf.dll で採らなかった理由と同じ形） | 停止（現状。実測） | 該当しない（インストーラに無い。Windows の DLL） |
| powrprof | 可（導出: PowrProf.dll、Vista 以降）。2 名は応答表に行が無く NULL と 127。使用側は検査してから使う（§2.4） | (a)(b) は満たす。(c) は avrt と同じで確認できない | 停止 | 該当しない |
| psapi | 可（導出が弱い。Windows 10 の記述なし）。ただし**この環境ではそもそも実行されない**（§2.3。VerifyVersionInfoW が真なら KERNEL32.dll を引く） | (a)(b) は満たす（§2.2 #15）が、(c) は導出が弱い | 停止。ただし到達しない見込み（読み。実測は無い）。到達するのは OS が 6.1 未満の場合のみ | 該当しない |
| riched20 | **一次に無い**。Learn は DLL 名と版の対応だけ。Windows 10 の記述なし | **(a) が成り立ちにくい**: NULL（または 0x20 以下）でもゲストは `RICHEDIT20W` の窓を作りに行く（§2.2 #17 注 4）。**(c) は一次に無いので確認できない** | 停止。ゲストが実際にここへ届くか（静的な unit なし）は未確認 | 該当しない |
| urlmon | 導出が弱い（archive ページ）。ロードの時点は許容、`CopyStgMedium` を呼ぶと例外（§2.2 #18） | (a) は呼ばれなければ満たす。**呼ばれると例外** | 停止。静的な unit なし。ゲストが `f_707bd0` を実行するかは未確認 | 該当しない |
| ws2_32 | 可（導出: Ws2_32.dll、Vista / 8.1）。**GetProcAddress("WSAStartup") は応答表に行が無く NULL。ゲストは NULL で RaiseException**（0xc07f82 `jne 0xc07fd5`、NULL のとき 0xc07fa3〜0xc07fd0 で例外を作る）。仮想にしても名前付き停止が移るだけ | **(a) を満たさない**（NULL で例外。§2.2 #23） | 停止 | 該当しない |
| mswsock | 可（導出: Mswsock.dll、Windows 2000）。GetProcAddress の行は無し | **(a) を満たさない**（NULL で例外。#10） | 停止（`ws2_32` が先） | 該当しない |
| iphlpapi | 可（導出: Iphlpapi.dll、Windows 2000 / XP）。GetProcAddress は応答表に行が無く NULL と 127。使用側 6 関数は NULL を検査して内蔵の代替へ（読み） | (a)(b) は満たす（#28）。(c) は導出と矛盾 | 停止（静的な unit なし。`ws2_32` の経路の先） | 該当しない |
| wship6 | **一次に無い**（Windows 2000 の IPv6 Technology Preview のファイル。Requirements 欄なし）。ゲストが引くのは ws2_32 の `GetAddrInfoW` が NULL のときだけ | (a)(b) は満たす（#24）。**(c)**: Windows 10 に wship6.dll が**ある**と言える一次が無い。**無いと言える一次もない**（Learn は「IPv6 Technology Preview for Windows 2000 のファイル」と書く）ので、「無い」と定義することが環境の定義と矛盾するかは確認できない | 停止（`GetAddrInfoW` の分岐に到達したときだけ） | 該当しない |
| fwpuclnt | 可（導出: Fwpuclnt.dll、Vista）。6 名は応答表に行が無い | (a)(b) は満たす（#25）。(c) は導出と矛盾 | 停止 | 該当しない |
| idndl | **Windows 10 の標準ではない**（Learn: Vista より前の OS 向け、再頒布物が要る）。(a) は「Windows 10 には必ずある」と言えない | (a)(b) は満たす（#26）。**(c): 「無い」と言うことは Learn の記述と矛盾しない**（Vista 以降は不要と書く）。ただし「Windows 10 に無い」と明記するページは取得していない（一次に無い） | 停止 | 該当しない |
| normaliz | 可（導出: Normaliz.dll、Vista）。5 名は応答表に行が無い | (a)(b) は満たす（#27）。(c) は導出と矛盾 | 停止 | 該当しない |

### 6.2 インストーラに同名のファイルがある 14 名

(a) 仮想、(b) absent は、これらが「Windows 10 に必ずある」とは言えない（Windows の DLL ではない）ので、(a) は決定 0055 の流儀に当たらず、(b) は決定 0073 が dsp_ippv2 について採らなかった理由（実機の DLL の有無について偽の主張になる）が同じ形で当てはまる。**以下は事実だけを置く**。

| 名前 | (a) | (b) | (c) | (d) lift（M2） |
|---|---|---|---|---|
| dsp_ippv2_x64 | 決定 0073 が却下（偽の関数の表） | 決定 0073 が却下 | 却下（決定 0073） | **方向は決定済み**（0073、0082）。LoadLibrary は実モジュールで応答。DllMain の完走には別の保留（ロケールの表、0078、0083） |
| webview2loader | 該当しない（Windows の DLL ではない） | NULL を許容（#22）。読み: ゲストは NULL で 0 を返す。**インストーラに実在する**ので「無い」は実機と異なる主張 | 停止 | 未検討。Microsoft の WebView2 のローダー（Learn: アプリが同梱する）。実機での挙動（WebView2 Runtime の有無）は一次の範囲外 |
| beat_tracker | 同上 | **NULL で例外**（#14）。(a) を満たさない | 停止 | 未検討。DLL はインストーラにあり、エクスポートは未調査 |
| blend2d | 同上 | NULL を許容（#4）。実在する | 停止 | 未検討 |
| quickfontcache_x64 | 同上 | NULL を許容（#5）。実在する | 停止 | 未検討 |
| ilsvg_x64 | 同上 | **NULL 検査なし**（#16）。(a) を満たさない | 停止 | 未検討 |
| ildsp_x64 | 同上 | NULL を許容（#7）。実在する | 停止 | 未検討 |
| oggio_x64 | 同上 | NULL を許容（#12）。実在する | 停止 | 未検討 |
| iltools_x64 | 同上 | NULL を許容（#6、注 1 つき）。実在する。**ゲストが hModule=NULL の GetProcAddress を呼び、runtime は abort** | 停止 | 未検討 |
| wavpackdll_x64 | 同上 | NULL を許容（#13）。実在する | 停止 | 未検討 |
| lameenc_x64 | 同上 | NULL を許容（#19）。実在する | 停止 | 未検討 |
| python312 | 同上 | NULL を許容（#8）。実在する（`python312.zip`、`python312._pth` も） | 停止 | 未検討。Python の組み込み。範囲が大きい |
| ilvideo_x64 | 同上 | NULL を許容（#11）。実在する。**名前が `ffmpeg\x64\` を含むので、runtime は LoadLibraryW でパス付きを理由に別の停止をする可能性**（`fle_load.c` normalize。読み。未実測） | 停止 | 未検討 |
| speaker_x64 | 同上 | NULL を許容（#20）。実在する | 停止 | 未検討 |

### 6.3 インストーラに同名が無い FL の DLL 4 名

| 名前 | (a) | (b) | (c) | (d) |
|---|---|---|---|---|
| declipper | 該当しない | NULL を許容（#29）。**インストーラに無い**（別の入手経路は未確認）。ゲストは先に GetFileAttributesW で存在を見て、無ければ LoadLibraryW に届かず `al=1` で戻る（読み。仮想ファイルシステムに DLL が無ければ届かない見込み。実測無し） | 停止（届かないなら止まらない可能性） | 該当しない（DLL が無い） |
| stemextract | 同上 | **NULL で例外**（#30）。(a) を満たさない。同様に先に存在確認があるかは未読 | 停止 | 該当しない |
| ds2wav | 同上 | NULL を許容（#21） | 停止 | 該当しない |
| ss2wav | 同上 | NULL を許容（#9） | 停止 | 該当しない |

### 6.4 決定 0055 の基準に照らした整理（事実の並べ替えだけ）

(b) の (a)（NULL 許容）と (b)（NULL の経路が自然）を満たす Windows の DLL は avrt、powrprof、psapi、iphlpapi、fwpuclnt、normaliz、wship6、idndl の 8 名。ほかの 4 名（riched20、urlmon、ws2_32、mswsock）は (a) か (b) を満たさない。

そのうち (c)（環境の定義との矛盾）:
- avrt、powrprof、iphlpapi、fwpuclnt、normaliz: Learn の Requirements 欄から Windows 10 にある（導出）。**「無い」と定義すると導出と矛盾する**。これは、決定 0015 の 9 名のうち dwmapi と shcore が、同じ導出では Windows 10 にあることになるのに absent にされている件（`external-modules-usage.md` §2.3 が指摘済み。PM の判断）と同じ形。
- psapi: 導出が弱い。しかも到達しない見込み。
- idndl: Windows 10 の標準ではない（Learn の記述）。「無い」と定義しても Learn と矛盾しないが、「無い」とページが言うわけでもない。
- wship6: 一次に無い（有る、無いのどちらも言えない）。

riched20 は (a)(b) を満たさず、一次にも無い。ws2_32 と mswsock は (a) を満たさない。

## 7. DllMain 後に最初に止まる名前の順序（項目 4）

### 7.1 実測の順序（gdb。仮の値つき。`runs/re6.log`）

| 順 | 名前 | InitCount（unit） | 呼び出し点 | 止まる理由（runtime） |
|---:|---|---|---|---|
| 1 | `avrt` | 146（145） | 0x65ccd1 | LoadLibraryW が `avrt.dll` を引き、仮想にも absent にも無い |
| 2 | `PowrProf.dll` | 146（145） | 0x65cf38 | 同上（1 の後、`advapi32` を挟む。`dwmapi` は absent で通過） |
| 3 | `Blend2D.dll` | 150（149） | 0x6641b9（`f_6539e0`） | 同上 |
| — | （LoadLibrary ではない）`RegCreateKeyExW` | 154（153） | `f_422ff0` ← `f_5af5e0` ← `f_664fd0` ← `f_665b90` | HKLM の直下（決定が要る別件） |

この順序は、`dsp_ippv2_x64.dll`（unit 144）を実モジュールとして DllMain まで実行し、5 つの仮の値（GetStartupInfoW の cb、MultiByteToWideChar、GetStringTypeW、LCMapStringW、WideCharToMultiByte）を入れた構成での実測。**仮の値を入れないと DllMain は完走せず（決定 0083 の前のビルドでは GetStartupInfoW）、avrt に届かない**。決定 0083 の runtime 修正後にビルドし直した構成で DllMain が戻るかは未測定。

### 7.2 RegCreateKeyExW より後（未測定。静的な読みだけ）

RegCreateKeyExW を過ぎた後の LoadLibrary の順序は**測定していない**。静的に届く unit（`reach-sites.tsv`、`ancestors.tsv`。下限・上限の意味は §0.2）の若い順:

| 静的に届く最小 unit | 名前 | 備考 |
|---:|---|---|
| 149（直接） | Blend2D.dll | 実測と一致 |
| 158 | quickfontcache_x64.dll | 直接。NULL 許容 |
| 190 | blend2d.dll の 2 つ目の呼び出し元 | `f_6cf2a0` |
| 245 | iltools_x64.dll（ILTools） | 直接。NULL 検査なしで GetProcAddress(NULL,…) を呼ぶ（runtime は abort） |
| 575（lea 込み） | mswsock.dll | NULL で例外。`ws2_32` の経路の先で、`ws2_32` の LoadLibraryW 自体には静的な unit が無い |
| 614（lea 込み、実行は 895、898 が直接） | ildsp_x64、python312、ss2wav、ilvideo_x64、oggio_x64、wavpackdll_x64 | NULL 許容（ilvideo、oggio、wavpack は lea 込みだけで届く） |
| 静的な unit なし | psapi、riched20、urlmon、webview2loader、beat_tracker、ilsvg_x64、lameenc_x64、speaker_x64、ds2wav、ws2_32、wship6、fwpuclnt、idndl、normaliz、iphlpapi、declipper、stemextract | vtable、コールバック、ユーザー操作の経路。実行されるかは未確認（`ancestors.tsv` のデータ参照の祖先は speaker 52、ds2wav 51 など） |

`init_count` 942 のうち、DllMain の後の最初の停止は unit 145。`mswsock` と `ws2_32` は同じ関数群（`f_c079e0`、`f_c12ab0`）で、起動の unit からは lea 経由の辺しか無い。

## 8. 意見（別枠。決めない。PM の判断の材料）

この節は作業者の意見で、事実の節ではない。

1. **avrt、PowrProf.dll は、決定 0055 の流儀（msctf.dll）なら (a) 仮想モジュール**になる。理由は §6.1 の通り、Learn の Requirements 欄から Windows 10 にある（導出）ので、(b) absent は (c) と矛盾する。ただし 0015、0029 が dwmapi と shcore を同じ導出の下で absent にしているため、**0055 の基準を 0015 の 9 名にも当てて整合させるか、9 名は PM の選択として据え置き、新しい名前は 0055 に従うか**は PM が決める。この矛盾は `external-modules-usage.md` §2.3 でも指摘済み。
2. (a) 仮想にすると GetProcAddress の名前は全部 NULL になり、ゲストは機能無効で進む（avrt の 11 名、PowrProf の 2 名。読み、実測では仮の NULL でクラッシュなし）。決定 0081 の api-ms の 4 名と同じ形で、実装は小さい（`fle_modules.c` の表の末尾に足す。N=31、32）。数を固定するテスト（`count 30`、`msctf.rs`、`modules.rs`）は要更新（0081 の実装作業で同じ扱い）。
3. **Blend2D.dll、dsp_ippv2_x64.dll などの同梱 DLL は (a)、(b) の対象にしない方が、決定 0073 と整合する**。これらは Windows の DLL ではなく、実機で存在するので、「ある」とも「無い」とも runtime は言えない。M2（lift）か、停止のまま、のどちらか。Blend2D は NULL を許容するが、(b) は実在の DLL を「無い」と言う主張になる。
4. **ws2_32、mswsock は (a) 仮想にしても名前付き停止が移るだけ**（ゲストが NULL で例外）。これらは WSAStartup が応答表に無いまま。ネットワーク層（WSAStartup 以降のソケット API）を仮想で作るかは別の決定になる。大きさは見積もっていない。今は届かない（静的な unit なし）ので、**当面の対象にしない**のが妥当と考える。
5. **psapi は到達しない見込み**（§2.3）なので、足す必要が当面は無い。ただし `load.rs` の固定リストにあるので、足すと既存テストの更新が要る。
6. **riched20 は一次に無く、NULL でも窓の作成へ進む**ので、(a)(b) とも決められない。窓クラス（決定 0070、0071）が `RICHEDIT20W` を止めているので、そちらの決定と合わせて扱う方が自然。
7. **idndl、wship6、normaliz、fwpuclnt、iphlpapi** は ws2_32 の経路の中。ws2_32 が止まる限り届かない。ws2_32 の扱いが決まってから決める。
8. 次の停止点（RegCreateKeyExW）は LoadLibrary の外。LoadLibrary 側だけを直すと、そこで止まる（実測）。

## 9. 未確認

- 仮の値なしで DllMain が戻るかは、決定 0083 の後の再ビルドで未測定。`out-stage2` のバイナリは 0083 の前（18:15）。
- RegCreateKeyExW より後の実測の順序（§7.2）。ゲストが各名前の呼び出し点へ実際に届くか（vtable、コールバック、ユーザー操作）。
- `f_6539e0` が dsp_ippv2 以外の名前で最終的に LoadLibraryW に渡す文字列（`_x64.dll` の付与、ディレクトリ前置、`\` を含むパスの有無）。`Python312` と `ilvideo_x64` は `\` を含む可能性があり、runtime の `normalize` が別の理由で止める（読み）。
- `f_43ba70`（GetFileAttributesW による存在確認）が、仮想ファイルシステムでどの結果を返すか。declipper と dsp_ippv2 は存在確認を通らないと LoadLibraryW に届かない（読みの範囲。dsp_ippv2 は実測で届いた）。
- Learn が「Windows 10 22H2 に avrt.dll などが存在する」と明記したページ（無い。導出のみ）。riched20、wship6 について Windows 10 での有無を言う一次（無い）。
- `RICHEDIT20W` の窓クラスが無いときの `CreateWindowEx` の結果とゲストの扱い。
- インストーラ以外の入手経路で declipper.dll、stemextract.dll、ds2wav.dll、ss2wav.dll が来るか。
- ILSvg の `svgCreate` スロットを上位が守るか（#16 注 3）。
- 静的な unit が無い 17 名が実行される条件。
- GetModuleHandleW の名前が分からない 3 点（0x8eb0b9 は NULL、0x8eb0fb と 0x9da828 は動的）と、LoadLibrary 系で名前が分からない点の中身（`f_44db40` の呼び出し元の多く。`dyn-callers-recheck.txt`）。0x9da8a0 は `kernel32.dll` の `RtlCaptureStackBackTrace`、0xc522f0 は `software\asio\` と `clsid`（レジストリの値から DLL 名を取る形）、0xb76250 は `clap_entry` を引く（CLAP プラグイン）、0xb692b0 は `CreateFormatList` を引く、0xac4b50 は `dll loaded` の文字列を持つ、0x95ea70 は `CloudBounceConsoleAPI_login` を引く、0x9b58a0 は `DZ_Exec` を引く（DelZip64。呼び元は `DelZip64.dll` を渡す）、0x84343 系は Windows の `Keyboard Layouts` レジストリの `Layout File` を読む、0x132c680 は `_X64` を連結する。**これらの名前の実体（プラグインの DLL 名、レジストリの値）はホストと環境で決まり、静的には分からない**。

## 10. 作ったファイル

文書: `docs/investigations/loadlibrary-names-classification.md`（本書。新規）。

`lift/out-loadlib/`（新規。前回の成果物を含む。`__pycache__` は作らない設定の `PYTHONDONTWRITEBYTECODE=1` で実行した）:
- 今回新規のスクリプト: `presence_check.py`、`nullwin.py`、`prewin.py`、`delayimp.py`、`tally.py`、`order_table.py`。
- 今回の出力: `nsis-list-recheck.txt`、`7z-l-recheck.txt`、`7z-l-nsis-recheck.txt`、`presence.tsv`、`sites-flengine-recheck.tsv`、`dsp-sites-recheck.tsv`、`dsp-tables-recheck.txt`、`delayimp-recheck.txt`、`tally.txt`、`order.tsv`、`nullwin-1.txt`、`nullwin-2.txt`、`dyn-windows.txt`、`dl-args.txt`、`args-more.txt`、`dyn-null-2.txt`、`dyn-null-3.txt`、`delzip-callers.txt`、`iphl-readers.txt`、`ws2-slot-writers.txt`、`fn-*.txt`（再読の関数ダンプ）、`learn-recheck-*.txt`、`learn/`（Learn の生ページ。前回 30 + 今回 6）、`runs/re5.*`、`runs/re6.*`（gdb 再実行）。
- 前回から再利用（読み取りのみ）: `llpe.py`、`sites.py`、`callgraph.py`、`reach.py`、`ancestors.py`、`edges-direct.tsv`、`edges-ptr.tsv`、`units.tsv`、`reach-sites.tsv`、`ancestors.tsv`、`names.tsv`、`names_table.py`、`slot_users.py`、`slot_users.tsv`、`gdb_names.py`、`run_names.sh`、`runs/base0.*`、`runs/chain5.*`、`runs/chain6.*`、`dsp_sites.py`、`dsp_tables.py`、`dyn_callers.py`、`dumpfn.py`、`nsis-presence.tsv`、`nsis-all-names.txt`、`tmp/old_*.c`（前回の HEAD コピー）。
- 編集していないもの: `lift/` 配下の既存ファイル（`runtime/`、`crates/`、`harness/`、`out-stage2/`、`out-ext/`）。`fle_env.c`、`env.rs` には触れていない。
- 規則: `rm` なし、`>` による既存ファイルの全体上書きなし（新規ファイルに `>` で出力したものはある。`x` モードか新規名）、git の書き込み操作なし（`git show` と `git status` のみ）、`/tmp` 不使用（`TMPDIR=lift/out-loadlib/tmp`）。

