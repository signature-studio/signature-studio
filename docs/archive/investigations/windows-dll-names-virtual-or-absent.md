# Windows の DLL 12 名（avrt、powrprof、psapi、riched20、urlmon、ws2_32、mswsock、iphlpapi、wship6、fwpuclnt、idndl、normaliz）を仮想モジュールにするか absent にするか: 事実の整理

**調査のみ。この文書は何も決めない。** 意見は §9 に別枠で置く。

## 0. 基準と凡例

- 実行日: 2026-10-24。**HEAD `c75acc4`**（依頼の HEAD は `8b2fc8e`。その後に別作業が `docs/progress.md` だけを変更するコミットを 1 つ足した。`git diff --stat 8b2fc8e HEAD -- lift/runtime lift/crates docs/decisions docs/investigations/loadlibrary-names-classification.md docs/investigations/external-modules-usage.md` は空。実測）。
- 入力: `binaries/win64/FLEngine_x64.dll` SHA-256 `b2c64da4…496d`（実測。前回の分類文書と一致）。読んだソースは `lift/runtime/fle_modules.c`、`fle_load.c`、`lift/crates/fe-lift/src/imports_table.rs` で、いずれも HEAD とワークツリーが一致（`git diff --quiet HEAD -- FILE`。実測）。
- 凡例: **実測** = このセッションで実行したコマンドの出力（出力ファイルは §10）。**読み** = capstone の線形逆アセンブルとソースを読んだ結果で、実行していない。**引用** = 前回の文書（`loadlibrary-names-classification.md`、以下「分類」。コミット 7a9af0f）または決定の記述をそのまま引いたもの。**未確認** = 確認していない。**意見** = §9 だけ。
- **導出** = Learn の Requirements 欄の DLL 名と最小対応 OS から「Windows 10 にある」と推論したもの。ページが Windows 10 に必ずあると書いているわけではない（決定 0055 と同じ扱い）。
- 「unit N」は初期化表（`units_ptr` 0x13768e0、942 件、16 バイト/件）の 0 始まりの index。ゲストの InitCount は index + 1（分類 §0）。今回、DLL から表を直接読み直し、前回の `units.tsv` の 895 行と **0 件の不一致**を確認した（`units-own.txt`。実測）。
- 静的到達の限界（分類 §0.2 と同じ）: 直接 call/jmp と `lea reg,[rip+関数]` の辺だけのグラフ（前回の `edges-direct.tsv`、`edges-ptr.tsv` を再利用。今回は再生成していない）。vtable、レジスタ経由、コールバックは辺に無いので、「unit が無い」は「実行されない」を意味しない。
- 仕様の出典は Microsoft Learn だけ（取得は 2026-10-24。生ページは `lift/out-wdll/learn/`）。非公式資料は使っていない。
- 現行 runtime（HEAD）: `fle_modules[]` は 30 名（N=1..24 が PE の import 表、25 ntdll、26 msctf、27..30 が api-ms の 4 名。`fle_modules.c:16-50`）。`fle_load_absent[]` は 9 名（`fle_load.c:53-56`）。応答表 `TABLE` は 260 行（`imports_table.rs`。`grep -c '^    ("'` = 260。実測）。
- LoadLibrary 系の解決順（`fle_load.c:140-175`、読み）: (1) 実在モジュール表 `fle_realmod`（2 モジュール構成のときだけ）、(2) `fle_modules[]`、(3) `fle_load_absent[]`（NULL と 126）、(4) それ以外は `fle_stop`。GetProcAddress（`fle_modules.c:168-228`）は、応答表の `dll` 欄がそのモジュール名の行だけ擬似アドレスを返し、無ければ NULL と 127。hModule が仮想ハンドルでない値（NULL を含む）なら abort（`fle_modules.c:199-205`）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 12 名のうち DllMain の後に実行が届く名前 | **avrt と PowrProf.dll の 2 名**（unit 145 の `f_65c9f0`、dwmapi と shcore と同じ関数）。残り 10 名は静的な辺では unit に届かない | 読み（静的）。順序は前回の仮の値つき gdb 測定の引用 |
| 2 | 仮想にしたときに 0 番地を呼ぶ経路 | **今回読んだ範囲では見つからなかった**。ただし avrt の 8 スロット、fwpuclnt の 6、idndl の 3、normaliz の 3 は読み手が見つからず、検査の有無は未確認 | 読み／未確認 |
| 3 | absent と仮想で結果が違う名前 | **wship6 だけ**（absent だと `GetProcAddress(NULL,…)` で runtime が abort。仮想なら内蔵の代替へ）。riched20、urlmon、ws2_32、mswsock は両方で**同じ名前付き停止**に届く。avrt、powrprof、iphlpapi、fwpuclnt、idndl、normaliz は両方で進む。psapi は到達しない見込み | 読み |
| 4 | NULL で RaiseException に届く名前 | ws2_32（110/110 点）、mswsock、urlmon（使われれば）。0 番地は呼ばない（呼ぶ前に例外） | 実測（ws2_32 の 110 点の分岐）＋読み |
| 5 | 応答表の行 | **12 名の行は 0**（260 行の `dll` 欄に無い）。`fle_imp_*` も 0。仮想にしても行を足さなければ全 GetProcAddress が NULL と 127 | 実測 |
| 6 | Windows 10 にあると導出できる名前 | avrt、powrprof、ws2_32、mswsock、iphlpapi、fwpuclnt、normaliz（Requirements の DLL 欄と最小 OS）。弱い: psapi、urlmon。一次に無い: riched20、wship6。標準ではない: idndl | 実測（Learn 取得）＋導出 |
| 7 | 決定 0015/0029 と 0055 の基準 | 文面が違う（名簿方式と (a)(b)(c) の確認方式）。0055 の (c) を dwmapi、shcore に当てると、同じ導出で「Windows 10 にある」側に入る。**どの決定も相手の基準で書き直されていない**。矛盾と呼ぶかは PM の判断 | 決定の本文（引用） |
| 8 | 12 名は PE の import 表にも遅延 import にも無い | 0015 の「import 表で区切る」基準の外側 | 実測 |
| 9 | 仮想にしたときの影響範囲 | 固定数 30／31 を決め打ちするテストは 3 ファイル 7 つの検査（`modules.rs:376`、`377`〜`383`、`530`。`msctf.rs:258`、`269`、`342`、`429`。`load.rs:446`。うち `msctf.rs:269` と `429` は同じ `handle-31` の場面）、「止まる」を期待する `load.rs` の 7 名。他に `mods_end = … * 30` を持つ `event.rs:261`、`glob.rs:205`、`vol.rs:41`、`modules.rs:217`（N が増えても不等式は保たれる。算術）。ハンドルは N=31..42 で衝突しない（算術）。生成 C の SHA は、応答表に行を足さなければ変わらない（読み） | 読み／算術 |
| 10 | FLEngine の生成 C の SHA | 既存ビルドで `43985d62…`（決定 0078 の記録と一致。実測）。ビルドが現在のソースから作られたかは未確認 | 実測／未確認 |

**この文書は何も決めない**。論点と選択肢の事実は §6、意見は §9、未確認は §8。

## 2. 決定 0015 / 0029 / 0055 / 0081 / 0086 の判断基準

引用は決定の本文（HEAD）。行番号は `docs/decisions/` 内。

### 2.1 dwmapi と shcore を absent にした理由（決定 0015、0029）

- **決定 0015**（行 22、PM の決定のもとでの「作業者の判断」の節）: 「遅延 import だけの DLL（dwmapi、imm32、msimg32、propsys、rpcrt4、shcore、uxtheme、windowscodecs、wtsapi32）は表に入れない。『PE の import 表にある 24 DLL』の『normal』を選んだ。これらの名前を GetModuleHandleW に渡す呼び出しが出たら、126 を返す（今はロードされていない扱い）。必要が出たら末尾に足す。」
  - 基準として本文に書かれているのは、**FLEngine の PE の import 表にあるか（normal か、遅延 import だけか）**。Windows 10 にその DLL があるか、ゲストが NULL を許容するかは、この本文に書かれていない。
- **決定 0029**（行 40）: 「決定 0015 が『ロードされていない』とした 9 DLL（上記の遅延 import だけの DLL）: NULL と ERROR_MOD_NOT_FOUND（126）。… この名前表は `fle_load.c` の `fle_load_absent[]`（9 行）。**9 行はこの決定の範囲で追加も削除もしない**。」
- **決定 0029**（行 71）は、今回の 12 名のうち 5 名について、absent にしなかった理由を書く: 「`avrt`、`powrprof`、`psapi`、`riched20`、`urlmon` に『存在しない』（126）を返す案は、これらの呼び出しの先へ実行を進められるが、決定 0015 が名指しした 9 DLL の範囲を越え、環境についての主張を新しく作ることになるので採らなかった。足す場合は `fle_load_absent[]` に 1 行ずつ足す（PM の決定が先）。」
  - 基準は **「決定 0015 が名指しした 9 DLL の範囲か」**（名簿方式）と、「環境についての主張を新しく作らない」。
- 外部調査 `external-modules-usage.md` §2.3（引用）: Learn の Requirements 欄から `AvSetMmThreadCharacteristicsW` は Vista 以降（Avrt.dll）、`PowerGetActiveScheme` は Vista 以降（PowrProf.dll）、`GetDpiForMonitor` は 8.1 以降（Shcore.dll）、`DwmIsCompositionEnabled` は Vista 以降（Dwmapi.dll）。「ページから導ける範囲では、4 つとも Windows 10 にある」。今回、`DwmIsCompositionEnabled` と `GetDpiForMonitor` のページを再取得し、同じ値を確認した（`learn-run-4.txt`。実測: `req.target-min-winverclnt: Windows Vista`、`req.dll: Dwmapi.dll`／`Windows 8.1`、`Shcore.dll`）。

### 2.2 msctf.dll を仮想モジュールにした理由と (c)（決定 0055）

- **(a)(b)(c) とは**（決定 0055 行 11、15）: PM の方針は、msctf.dll を `fle_load_absent[]` に加えてよいのは **(a) ゲストが NULL を許容、(b) NULL の経路が自然、(c) 環境の定義と矛盾しない**がすべて確認できたときだけ、というもの。表の列見出しは「(a) ゲストが NULL を許容」「(b) NULL の経路が自然」「(c) 環境の定義と矛盾しない」。
- **(c) の導出**（行 19）: 「決定 0014 のエミュレート OS は Windows 10 22H2。msctf.dll の SetInputScope ページ（Learn）は要件を『Windows XP 以降』（DLL は Msctf.dll）と書き、TSF の概要ページは『Windows XP or later versions of the operating system』と書く。Windows 10 は XP 以降なので、**ページから導ける範囲で『Windows 10 には msctf.dll がある』**（ページが Windows 10 に msctf.dll があるとそのまま書いているわけではない。導いた）。A はこれと逆の主張（『この環境は msctf.dll を持たない』）になる。**A は採らない**。PM の方針の『矛盾する場合は欠いているとは定義せず、仮想モジュールとして応答させる』に従い B を採る。」
  - つまり (c) は **Learn の Requirements 欄の最小対応 OS と DLL 名から「Windows 10 にある」を導き、「absent（無い）」がそれと逆の主張になるか**を見る基準。決定 0055 では msctf は (a)(b) を満たしたが (c) を満たさず、仮想モジュールになった。
- **決定 0081**（行 34、40）は同じ (c) を api-ms の 4 名に当て、「(b) の却下理由: 『この環境に無い』という主張になる。決定 0055 の基準 (c)（環境の定義 0014、Windows 10 22H2 と矛盾しない）を、**一次資料で確認できない**（Learn は個別の `api-ms-win-*` 名が Windows 10 22H2 で `LoadLibrary` により解決できるかを書かない）」と書く。ここで基準は「**absent が環境の定義と矛盾しないことを確認できるか**」（確認できなければ absent にしない）の向きで使われている。
- **決定 0086**（行 71）は、今回の 12 名を「扱わないもの」とし、「特に unit 145 の `avrt`、`PowrProf.dll` の停止は、unit 149 より前に現れる別件」と書く。0086 は absent にも仮想モジュールにもしない方向を、インストーラに実在する同梱 DLL（Blend2D、QuickFontCache）にだけ断定する。Windows の DLL の扱いは決めていない。

### 2.3 両者の基準は同じか、違うか（事実）

| 項目 | 決定 0015 / 0029（dwmapi、shcore ほか 9 名） | 決定 0055 / 0081（msctf、api-ms 4 名） |
|---|---|---|
| 本文が挙げる基準 | FLEngine の PE の import 表にあるか（遅延 import だけなら表に入れない）。0029 は「0015 が名指しした 9 DLL の範囲」 | (a) NULL 許容、(b) NULL 経路が自然、(c) 環境の定義（Windows 10 22H2）と矛盾しない |
| Windows 10 にあるかの導出を使うか | 決定本文では使っていない（0015、0029 の本文に Requirements の引用は無い） | 使う。(c) の確認に Learn の Requirements を引く |
| ゲストの NULL 許容を使うか | 0015 の本文に無い。0029 は「環境についての主張を新しく作る」ことを理由にする | 使う（(a)(b)） |
| 同じ導出を当てると | dwmapi（Vista 以降、Dwmapi.dll）と shcore（8.1 以降、Shcore.dll）は Windows 10 にあることになる（導出。§2.1 で再確認） | msctf（XP 以降）は Windows 10 にあることになる |
| 結果 | absent（NULL と 126） | 仮想モジュール |

- **事実として言えること**: (1) 0015/0029 が挙げた基準と 0055 が挙げた基準は**文面が違う**（名簿方式と、(a)(b)(c) の確認方式）。(2) 0055 の (c) を 0015 の 9 名のうち dwmapi と shcore にそのまま当てると、同じ導出で「Windows 10 にある」ことになり、「absent」は (c) と矛盾する形になる。この指摘は `external-modules-usage.md` §2.3 に既にあり、分類 §6.4 と §8 も同じ。(3) 決定 0055 は「`fle_load_absent[]`（決定 0029 の 9 名）は変えない」と書き（行 26）、9 名を見直していない。決定 0081 も同じ。(4) 今回の 12 名のうち、決定 0029 は 5 名（avrt、powrprof、psapi、riched20、urlmon）について「足す場合は PM の決定が先」と書く。
- **矛盾か矛盾でないか**: 決定の本文の語だけで言えるのは、「0015 の 9 名は 0055 の (c) を適用して選ばれたものではない（0015 は import 表の normal / 遅延で区切った）」ことと、「0055 の (c) を 9 名に当てると dwmapi、shcore は absent にできない側に入る（導出）」こと。**どちらの決定も、相手の基準で書き直されていない**。「矛盾」と呼ぶか、「決定の時期と基準が違うだけ」と呼ぶかは、PM の判断（§6 の論点 1）。この文書は呼び分けない。意見は §9。
- 備考（**実測**。`wdll_imports12.py`、出力 `imports12.txt`）: FLEngine の normal import 表は 24 DLL、遅延 import 表は 17 記述子（重複を除いて 14 DLL。うち 9 名が決定 0015 の「遅延 import だけ」の名簿で、dwmapi と shcore を含む）。**今回の 12 名は、どちらの表にも無い**。したがって 0015 の「import 表で区切る」基準は 12 名には**当てはまらない**（12 名は 0015 が名指しした 9 名の外側）。

## 3. 12 名の表

列の読み方（共通）:
- **LoadLibrary 点**: 呼び出し命令の VA と、それを含む関数。`f_44db40` は SetErrorMode → LoadLibraryW → SetErrorMode のラッパー（分類 §2。引用）。VA は今回 `wdll_gpa.py` で取り直した（`gpa-map-1.txt`。実測）。
- **届く unit**: 静的到達（下限＝直接辺 D、上限＝`lea` 込み P。`wdll_reach.py`、出力 `reach-12.txt`。前回のグラフを再利用）。`-` は「静的な辺では unit に届かない」で、実行されないことを意味しない。
- **ハンドルの保存先**: 戻り値を書き込む .bss/.data のセル（`xref-avrt-powrprof.txt`、`xref-others.txt`。実測）。
- **GetProcAddress の名前**: 呼び出し直前の `lea rdx,[rip+X]` が指す UTF-16 文字列（`names12.tsv`。実測。名前は全部 `.text` 内の Delphi 風の文字列定数）。
- **NULL 検査**: ハンドルを検査してから GetProcAddress に渡すか／結果のスロットを使う側が検査するか。「読み」。

### 3.1 一覧

| # | 名前（ゲストが LoadLibrary に渡す文字列） | LoadLibrary 点 | 届く unit | ハンドルの保存先 | GetProcAddress の名前の数（呼び出し点） | ロード直後の NULL 検査 | 結果を使う側の NULL 検査 |
|---|---|---|---|---|---|---|---|
| 1 | `avrt` | 0x65ccd1（LoadLibraryW）in `f_65c9f0` | **145**（D、P とも） | 0x15f4288（.bss） | 11（0x65ccf9〜0x65cdfd） | あり（0x65ccdd `cmp [0x15f4288],0; je 0x65ce09`） | 4 関数を読み直した: `f_bb0920`、`f_12b00f0` は呼ぶ前に `cmp [cell],0`、`f_bb1520`、`f_12b0190` は `[obj+0x48]` が非 0 のときだけ呼ぶ（§4.1） |
| 2 | `PowrProf.dll` | 0x65cf38（LoadLibraryW）in `f_65c9f0` | **145** | 0x15f42a8（.bss） | 2（0x65cf5c、0x65cf76） | あり（0x65cf44 `cmp …,0; je 0x65cf82`） | `f_65c6c0` が 0x65c6ca で `cmp [0x15f4258],0; je` してから使う（§4.2） |
| 3 | `PSAPI.dll`（と `KERNEL32.dll`） | 0x4263e1（PSAPI.dll）、0x4263f6（KERNEL32.dll）in `f_426310` | -（D、P とも） | 0x15ec320（.bss） | 2（0x42642f と 0x42645d。名前は呼び出し元の引数。K32 を前置） | あり（0x426402 `cmp [0x15ec320],0; jne`。NULL なら false を返す） | 呼び出し元 `f_4264f0`、`f_426570` は `f_426310` の戻り（al）が 0 なら API を呼ばない（§4.3） |
| 4 | `RICHED20.DLL` | 0x6d644d（LoadLibraryW）in `f_6d6430` | - | 0x15f6020（.bss） | **0**（`GetProcAddress` を呼ばない） | `cmp [0x15f6020],0x20; ja`。0x20 以下なら 0 を保存。どちらでも次の処理へ進む | 該当なし（ハンドルを使う側は FreeLibrary だけ。`f_6d75b0` の 0x6d75df `cmp …,0; je`） |
| 5 | `URLMON.DLL` | 0x707bfa（LoadLibraryW）in `f_707bd0` | - | 0x13f3e78（.data） | 1（`CopyStgMedium`、0x707c1e） | あり（0x707c06 `cmp …,0; je 0x707c2a`） | **結果スロット 0x13f3e80 が 0 なら `0x448d40` で例外オブジェクトを作り `0x411e80` を呼ぶ**（0x707c2a〜0x707c59）。非 0 ならそのまま `call [0x13f3e80]`（§4.5） |
| 6 | `WS2_32.DLL` | 0xc07a0f（`f_44db40`）in `f_c079e0` | -（`f_c19370` 経由。§4.6） | 0x14fd348（.data） | **110 呼び出し点・98 名**（0xc07f7a〜0xc127fe。`WSAStartup`、`socket`、`connect` など） | あり（0xc07a1b `cmp …,0; je 0xc07a52`。NULL なら `GetLastError` から例外を作る） | **全 110 点で NULL なら `0x411e80` に届く**（`stubnull.tsv`。実測: 110/110 点で、`test rax,rax; jne` の fall-through に `call 0x411e80` がある） |
| 7 | `MSWSOCK.DLL` | 0xc12afb（`f_44db40`）in `f_c12ab0` | -（D）／**575**（P。`f_c13010` が `lea` で参照） | 0x14fd350（.data） | 1（`WSARecvEx`、0xc12bcf） | ハンドルが NULL なら `GetLastError`、メッセージ生成、`0xc07b60`、`0x411e80`（例外）へ（0xc12b0f `cmp …,0; jne 0xc12b66`） | 結果 NULL なら同じく例外（0xc12bd7 `test rax,rax; jne 0xc12c2a`） |
| 8 | `iphlpapi.dll` | 0xc192fa（`f_44db40`）in `f_c192e0` | -（`f_c19370` 経由） | 0x14fee78（.data） | 5（`GetIpAddrTable`、`GetUniDirectionalAdapterInfo`、`GetAdaptersInfo`、`GetAdaptersAddresses`、`ConvertLengthToIpv4Mask`） | **ローダーに検査なし**（0xc192ff で保存するだけ） | 5 関数すべてが `cmp [0x14fee78],0; je` で NULL を見て、さらに `test rax,rax` で結果 NULL なら内蔵の代替関数（`f_c18f30` など）へ（§4.8） |
| 9 | `Wship6.dll` | 0xc14b47（`f_44db40`）in `f_c14b00` | -（`f_c19370` 経由） | 0x14fea40（.data）。その値を 0x14fea48 にコピー（0xc14b5a） | `0xc14b61` の `GetAddrInfoW`（1 回目は ws2_32 のハンドルで 0xc14b2e） | ws2_32 の `GetAddrInfoW` が NULL のときだけ到達（0xc14b39 `jne 0xc14b77`）。その後 `test rbx,rbx; je 0xc14d16`（内蔵の代替へ） | 同左（§4.9） |
| 10 | `Fwpuclnt.dll` | 0xc14c5e（`f_44db40`）in `f_c14b00` | -（`f_c19370` 経由） | 0x14fea50（.data） | 6（0xc14c86〜0xc14d08。`WSASetSocketSecurity` ほか） | あり（0xc14c6a `cmp …,0; je 0xc14d4c`） | 結果 6 スロットを使う側は **未確認**（読んでいない。§4.10） |
| 11 | `IdnDL.dll` | 0xc171fa（`f_44db40`）in `f_c171e0` | -（`f_c19370` 経由） | 0x14fedd0（.data） | 3（0xc1721e、0xc17238、0xc17252。`DownlevelGetLocaleScripts` ほか） | あり（0xc17206 `cmp …,0; je 0xc1725e`） | `f_c170a0`、`f_c170e0` が IdnToAscii 側のスロットを検査（§4.11）。idndl の 3 スロット（0x14fed90〜0x14feda0）の読み手は**今回の走査で見つからなかった** |
| 12 | `Normaliz.dll` | 0xc17278（`f_44db40`）in `f_c171e0` | -（`f_c19370` 経由） | 0x14fedd8（.data） | 5（0xc172a0〜0xc17308。`IdnToUnicode` ほか） | あり（0xc17284 `cmp …,0; je 0xc17314`） | `f_c170a0` が 0x14fedc8、0x14fedb8 を `cmp …,0` で検査。`f_c170e0` は 0x14fedc8 を検査してから `call [0x14fedc8]`（§4.12） |

注: `names12.tsv` の `test_after` 列は「call の直後 2 命令以内に `test rax,rax` がある」だけを見る。avrt、powrprof、normaliz、idndl、fwpuclnt の `n` は、結果を**スロットに保存して使う側で検査する**形（上の「結果を使う側」列で別に読んだ）であり、「検査しない」を意味しない。

### 3.2 Learn の Requirements、runtime の現状の行

| # | 名前 | Learn（取得 2026-10-24。URL とページの語） | Windows 10 にあると導出できるか | 現行 runtime の応答表の行（`fle_resp_table` 相当） | `fle_imp_*` |
|---|---|---|---|---|---|
| 1 | avrt | `…/win32/api/avrt/nf-avrt-avsetmmthreadcharacteristicsw`、`…avsetmmthreadpriority`、`…avrevertmmthreadcharacteristics`: `req.target-min-winverclnt: Windows Vista [desktop apps only]`、`req.dll: Avrt.dll`、`api_location: Avrt.dll`（3 ページとも実測） | **導出できる**（Vista 以降、Avrt.dll）。ページは Windows 10 とは書かない。残り 8 名のページは未取得 | **無い**（`dll` 欄に avrt の行 0。`grep` 0 件） | 無い |
| 2 | powrprof | `…/powersetting/nf-powersetting-powergetactivescheme`、`…powersetactivescheme`: Vista、`req.dll: PowrProf.dll`。`api_location` に `api-ms-win-power-setting-l1-1-1.dll` も並ぶ（API セットの話は本調査の範囲外） | 導出できる | 無い | 無い |
| 3 | psapi | `…/psapi/nf-psapi-getprocessmemoryinfo`、`…getmappedfilenamew`: `req.target-min-winverclnt: Windows XP`。`req.dll: Kernel32.dll on Windows 7 and Windows Server 2008 R2; Psapi.dll (if PSAPI_VERSION=1) on Windows 7 and Windows Server 2008 R2; Psapi.dll on Windows Server 2008, Windows Vista, Windows Server 2003 and Windows XP`。Windows 8 以降の記述は無い | **導出が弱い**（記述は Windows 7 まで。Windows 10 に Psapi.dll があるとは書かれていない） | 無い（K32 版の行も無い: `K32GetMappedFileNameW`、`K32GetProcessMemoryInfo` は 0 件） | 無い |
| 4 | riched20 | `…/controls/about-rich-edit-controls`: 表で「2.0 / 3.0 → Riched20.dll」「4.1 → Msftedit.dll」。OS ごとの有無は書かれていない（実測。`grep` で Riched20、Msftedit の 3 行だけ。Windows 10 の語は無い） | **一次に無い** | 無い（GetProcAddress を呼ばない名前なので行は不要） | 無い |
| 5 | urlmon | `…/previous-versions/windows/internet-explorer/ie-developer/platform-apis/ms775093(v=vs.85)`（CopyStgMedium の archive ページ）: `Minimum supported client: Windows XP`、`Product: Internet Explorer 4.0`、`DLL: Urlmon.dll`。通常版 `…/win32/api/urlmon/nf-urlmon-copystgmedium` は **404**（今回も） | **導出が弱い**（archive ページ） | 無い | 無い |
| 6 | ws2_32 | `…/winsock2/nf-winsock2-wsastartup`: `req.target-min-winverclnt: Windows 8.1, Windows Vista`、`req.dll: Ws2_32.dll`。`…/ws2tcpip/nf-ws2tcpip-getaddrinfo`、`…getaddrinfow`: `Windows XP, Windows 8.1`、`Ws2_32.dll` | 導出できる | 無い（`dll` 欄に ws2_32 の行 0） | 無い |
| 7 | mswsock | `…/mswsock/nf-mswsock-wsarecvex`: `req.target-min-winverclnt: Windows 2000 Professional`、`req.dll: Mswsock.dll` | 導出できる | 無い | 無い |
| 8 | iphlpapi | `…/iphlpapi/nf-iphlpapi-getadaptersinfo`: `Windows 2000 Professional`、`Iphlpapi.dll`。`…getadaptersaddresses`: `Windows XP`。`…getipaddrtable`: `Windows 2000 Professional`。`…netioapi/nf-netioapi-convertlengthtoipv4mask`: `Windows Vista`。`…getunidirectionaladapterinfo`: 最小 OS 欄は空、`DLL: Iphlpapi.dll` | 導出できる（`GetUniDirectionalAdapterInfo` だけ最小 OS の欄が空） | 無い | 無い |
| 9 | wship6 | Requirements 欄に wship6 のページは無い。`…/ws2tcpip/nf-ws2tcpip-getaddrinfo` の本文（実測、行 564）: 「the Wship6.dll (the file containing **getaddrinfo** in the IPv6 Technology Preview for Windows 2000)」 | **一次に無い**（Windows 2000 向けの記述だけ。Windows 10 に「ある」とも「無い」とも書かれていない） | 無い | 無い |
| 10 | fwpuclnt | `…/ws2tcpip/nf-ws2tcpip-wsasetsocketsecurity`: `req.target-min-winverclnt: Windows Vista`、`req.dll: Fwpuclnt.dll` | 導出できる | 無い | 無い |
| 11 | idndl | `…/win32/intl/downlevelverifyscripts`、`…downlevelgetlocalescripts`: `api_location: Idndl.dll`。本文: 「This function is used only by applications that run on pre-Windows Vista operating systems. Its use requires the download package.」Redistributable: 「Microsoft Internationalized Domain Name (IDN) Mitigation APIs on Windows XP with SP2, Windows Server 2003 with SP1, or Windows Vista」（実測）。Requirements の最小 OS 欄は無い | **Windows 10 の標準とは導出できない**（Learn は Vista より前の OS 向けの再頒布物と書く）。「Windows 10 に無い」と書いたページも取得していない | 無い | 無い |
| 12 | normaliz | `…/winnls/nf-winnls-idntoascii`、`…normalizestring`: `req.target-min-winverclnt: Windows Vista`、`req.dll: Normaliz.dll`、`req.redist: Microsoft Internationalized Domain Name (IDN) Mitigation APIs on Windows XP with SP2 and later…`。`api_location` に `kernel32.dll`、`KernelBase.dll` も並ぶ。`…winnls/nf-winnls-verifyscripts`: `req.dll: Kernel32.dll` | 導出できる（DLL 欄は Normaliz.dll、Vista 以降） | 無い | 無い |

- 応答表に行が無いことの確認（**実測**）: `imports_table.rs` の `dll` 欄の内訳は kernel32 173、user32 28、oleaut32 18、advapi32 12、winmm 11、ole32 10、gdi32 6、msctf 1、imagehlp 1（計 260）。12 名の DLL 名は 1 件も無い（`grep -c -i` で 0）。API 名（`AvSetMm…`、`PowerGet…`、`WSAStartup`、`CopyStgMedium`、`GetAddrInfoW`、`GetAdaptersInfo`、`IdnToAscii`、`NormalizeString`、`DownlevelVerifyScripts` ほか）の行も 0 件。`lift/runtime` に対応する `fle_imp_*` も 0 件。
- 取得できなかったページ（**未取得**）: avrt の残り 8 名（`AvSetMmMaxThreadCharacteristicsW`、`AvQuerySystemResponsiveness`、`AvRtCreateThreadOrderingGroup` ほか）、ws2_32 の 98 名のうち `WSAStartup`、`getaddrinfo`、`GetAddrInfoW` 以外、mswsock の `WSARecvEx` 以外（これは取得済み）、`CopyStgMedium` の通常版（404）。個別 API のページが同じ DLL を指すかは未確認。

## 4. 名前ごとの詳細

以下の「ゲストの振る舞い」は**すべて読み**（逆アセンブルを読んだもので、実行していない）。行きの経路は `wdll_dumpfn.py`、`wdll_xref.py`、`wdll_cells.py` の出力（`fn-*.txt`、`xref-*.txt`、`cells-*.txt`）で確かめた。

**用語**: 「仮想にした場合」= LoadLibraryW がそのモジュールの仮想ハンドル（0x7FF800000000 + 0x10000 × N）を返し、last error に触れない。GetProcAddress は応答表に行が無い名前で NULL と 127。「absent にした場合」= LoadLibraryW が NULL と 126。**どちらの場合も、保存先のセルはゲストから見て「GetProcAddress の結果が 0」か「ハンドルが 0」のどちらかになる**。

### 4.1 avrt（unit 145）

- 経路: `f_65c9f0`（unit 145 の proc。unit 表の index 145、`wdll_units.py` で再確認。実測）。入口で `sub [0x15f4268],1; cmp -1; jne` の一度だけ初期化（0x65c9f6〜0x65ca04）。0x65ccd1 の `LoadLibraryW(L"avrt")` の結果を 0x15f4288 に保存。
- **仮想にした場合**: LoadLibraryW は非 NULL なので 0x65ccdd の検査を通り、11 回の GetProcAddress（`AvSetMmMaxThreadCharacteristicsW`、`AvSetMmThreadCharacteristicsW`、`AvSetMmThreadPriority`、`AvQuerySystemResponsiveness`、`AvRevertMmThreadCharacteristics`、`AvRtCreateThreadOrderingGroup`、`AvRtCreateThreadOrderingGroupEx`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`）は全部 NULL と 127 を返す。結果は 0x15f40f0〜0x15f4140 の 11 スロットに **0 のまま**書かれる（`gpa-map-1.txt`、`names12.tsv` の `result_slot`）。0x65ccf9 の呼び出しは `f_4235f0`（ゲストの GetProcAddress ラッパー）→ thunk 0x4235e0 で、ハンドルは仮想ハンドルなので runtime の abort（`fle_modules.c:199-205`）には当たらない。
- **absent にした場合**: 0x65ccdd `je 0x65ce09` で 11 回を飛ばす。スロットは 0 のまま。
- **どちらでも同じ状態**: avrt の 11 スロットは 0。ハンドル 0x15f4288 だけが「非 0（仮想）」と「0（absent）」で違う。
- **スロットを使う側**（読み直した 4 関数。`fn-avrt-users.txt`）:
  - `f_bb0920`: 0xbb095b `mov rax,[0x15e1fe8]`（セルが `AvSetMmThreadCharacteristicsW` のスロット 0x15f40f8 を指す）、0xbb0962 `cmp qword ptr [rax],0; je 0xbb09ed` で、**呼ぶ前にスロットを検査**。0 なら 0xbb09ed へ飛ぶ。その先の 2 回目の `call [0x15dfee8 の指す先]`（`AvSetMmThreadPriority`、0xbb09a4）は 0xbb099a `test rax,rax; je` の後、つまり `AvSetMmThreadCharacteristicsW` の戻り値が非 0 のときだけ届く。
  - `f_12b00f0`: 0x12b011e〜0x12b0129 で同じ検査（`cmp [rax],0; je 0x12b016c`）。
  - `f_bb1520`、`f_12b0190`（`AvRevertMmThreadCharacteristics`）: `[rbx+0x48]` が非 0 のときだけ `call [slot]`。`[rbx+0x48]` は検査付きの `AvSetMmThreadCharacteristicsW` の結果を入れる場所で、スロットが 0 ならそこは 0 のまま。
  - 他のスロット（`AvSetMmMaxThreadCharacteristicsW`、`AvQuerySystemResponsiveness`、`AvRtCreateThreadOrderingGroup` など 8 つ。読み手が見つかったのは `AvSetMmThreadCharacteristicsW`、`AvSetMmThreadPriority`、`AvRevertMmThreadCharacteristics` の 3 つで、上の 4 関数がそれ）の読み手は、`wdll_cells.py` の走査（rip 相対とデータセル）では**終了側 `f_65c750` の 0 書き込みと、`f_65c9f0` の書き込みだけ**だった（`cells-avrt-powrprof.txt`。`AvSetMmMaxThreadCharacteristicsW` ほか 8 スロットで `via-cell`、`direct CALL`、`direct C`、`direct R` が 0 行。実測）。基底からのオフセットで読む形は走査していない（未確認）。
- 終了側 `f_65c750` は avrt のハンドルを `cmp [0x15f4288],0; je` の後で FreeLibrary する（0x65c8ff〜0x65c910）。仮想にした場合 FreeLibrary(仮想ハンドル) は runtime の `refs[idx]` が 1 なので「the reference count would reach zero」で停止する（`fle_load.c:246`）。LoadLibrary が refs を +1 するので、ゲストが LoadLibrary 1 回・FreeLibrary 1 回なら 2→1 で停止しない（読み。FreeLibrary に届くかは未確認。この関数が実行される unit は未調査）。
- **NULL を検査せずに呼ぶ経路**: 読んだ範囲では無い。仮想でも 0 番地を呼ぶ危険は、読んだ 4 関数には無い。残り 8 スロットの読み手は見つからなかった（未確認）。

### 4.2 powrprof（unit 145）

- 経路: 同じ `f_65c9f0`。0x65cf38 `LoadLibraryW(L"PowrProf.dll")` → 0x15f42a8。GetProcAddress は 2 名（`PowerGetActiveScheme` → 0x15f4250、`PowerSetActiveScheme` → 0x15f4258）。
- 仮想にした場合: 2 スロットは 0。absent にした場合: 0x65cf44 `je 0x65cf82` で飛ばす。
- 使う側 `f_65c6c0`（`fn-more-1.txt`）: 0x65c6ca `cmp [0x15f4258],0; je 0x65c740`（スロットが 0 なら何もせず戻る）。`PowerGetActiveScheme`（0x15f4250）の `call` は、その検査を通った後（0x65c6e7）にだけ届く。`PowerSetActiveScheme` の他の 2 つの call（0x65c719、0x65c733）も同じ検査の後。つまり**スロットが 0 でも 0 番地は呼ばれない**（読み）。
- この関数の呼び出し元は `f_11c7dc0`、`f_1357150`（unit に届く辺は無い。`reach-12.txt` の `f_65c6c0`: units D=-、P=-）。
- NULL を検査せずに呼ぶ経路: 読んだ範囲では無い。

### 4.3 psapi（unit に届く辺は無い）

- `f_426310`（`fn-more-1.txt`、分類 §2.3 を再読）: `[0x15ec328]`（OS 状態）が 0 のとき一度だけ `VerifyVersionInfoW`（`VerSetConditionMask` を 3 回: major、minor、service pack major。dwTypeMask 0x23。比較対象は 6.1 SP0）で「6.1 以上か」を問い、真なら状態 2、偽なら状態 1。
  - 状態 1（OS が 6.1 未満）: 0x4263e1 `LoadLibraryW(L"PSAPI.dll")`。
  - 状態 2: 0x4263f6 `LoadLibraryW(L"KERNEL32.dll")` を引き、GetProcAddress に `K32` を前置した名前（`f_414c70` で連結。0x426436〜0x42645d）を渡す。
- 現行 runtime の `fle_osver_check`（`fle_osver.c`、読み）は major 10 > 6 なので真を返す（`fle_emulated_os` は 10.0.19045。決定 0014）。**したがって 0x4263e1 の PSAPI.dll は、エミュレートした環境では実行されない**（読み。実行して確かめていない）。
- 状態 2 で引かれる `KERNEL32.dll` は仮想モジュール N=9 で応答済み。GetProcAddress の `K32GetMappedFileNameW`、`K32GetProcessMemoryInfo` は応答表に行が無く NULL と 127。0x426462〜0x42646c で `test rax,rax; jne` の fall-through は `al=0` を返す。呼び出し元 `f_4264f0`（`GetMappedFileNameW`）と `f_426570`（`GetProcessMemoryInfo`）は `test al,al; jne` で 0 なら API を呼ばず `eax=0` を返す（0x426518、0x426593。読み）。
- この 2 つのスロット（0x13a9760、0x13a9768）の初期値は、ファイル上で `f_4264f0`、`f_426570` のアドレス（実測 `psapi-slots.txt`）で、他の読み手 `f_4265f0`、`f_426600` が `call [slot]` で呼ぶ。`f_4265f0` は unit 895、898 に D、P 614、745、880、895、898 に届く（`xref-psapi-slots.txt` と `reach` の再実行。実測）。**つまり PSAPI 系の API を実際に呼ぶ関数 `f_4265f0` は、静的には起動の unit に届く**（実行されるかは未確認）。その先は `f_4264f0`（NULL 検査あり）。仮想／absent どちらでも `K32…` が NULL なら API は「無い」扱いで 0 を返す。
- 仮想にした場合の追加の影響: psapi をモジュール名として LoadLibrary するのは状態 1 の点だけで、状態 2 の点は kernel32 を引く。したがって psapi を仮想にしても absent にしても、**この環境の実行は psapi の LoadLibrary に届かない**（読み）。
- NULL を検査せずに呼ぶ経路: 無い（読み）。

### 4.4 riched20（unit に届く辺は無い）

- `f_6d6430`（`fn-net-1.txt`）: 0x6d643c `cmp [0x15f6020],0; jne 0x6d646e`、0x6d644d `LoadLibraryW(L"RICHED20.DLL")` → 0x15f6020、0x6d6459 `cmp [0x15f6020],0x20; ja 0x6d646e`、0x20 以下なら 0x6d6463 で 0 を保存。**GetProcAddress は呼ばない**。どちらの場合も 0x6d6474 `call 0x6326c0`、0x6d6486 `call 0x611550`（`RICHEDIT20W` を引数に）へ進む。
- `f_611550`（`fn-riched-follow.txt`）は `GetClassInfoW` 相当の `call 0x424ba0`（thunk は user32!GetClassInfoW。実測）を `hInstance` を 4 通り変えて呼ぶ（0x611574、0x61158e、0x6115a0、0x6115b4）。runtime の `fle_imp_GetClassInfoW` は、hInstance が 0x400000 以外なら `fle_stop`（`fle_class.c:172-173`）。つまり riched20 を仮想にしても absent にしても、**次の停止は GetClassInfoW**（`hInstance is NULL: the system classes are not modelled` か `hInstance is not the lifted DLL`）に移る（読み。実行していない）。
- 呼び出し元 `f_6d6430` は、unit 表から辺が無く、データ参照（`.text` 内の vtable 風の領域 0x6d23f0 のスロット。`rawref-1.txt`）からだけ参照される。実行されるかは未確認。
- NULL を検査せずに呼ぶ経路: 該当なし（GetProcAddress を呼ばない）。ただし「0 番地を呼ぶ」代わりに GetClassInfoW の停止に届く。

### 4.5 urlmon（unit に届く辺は無い）

- `f_707bd0`（`fn-net-1.txt`）: 0x707bfa `LoadLibraryW(L"URLMON.DLL")` → 0x13f3e78、0x707c06 `cmp …,0; je 0x707c2a`、0x707c1e GetProcAddress(`CopyStgMedium`) → 0x13f3e80。0x707c2a `cmp [0x13f3e80],0; jne 0x707c5e`。**0 のとき**: 0x41c820、0x448d40（例外オブジェクト）、0x411e80 に進む（0x707c34〜0x707c59）。非 0 のとき 0x707c66 `call [0x13f3e80]`。
- `0x411e80` は `f_411d20` を呼び、0x411d93 `cmp [0x15e6040],0; je 0x411e30`、0x411e30〜0x411e47 で `ecx=0x0EEDFADE`、`edx=1`、`r8d=8` として `call [0x15e6030]` を呼ぶ。`[0x15e6030]` に書かれるのは thunk 0x406fd0（`kernel32.dll!RaiseException`。実測 `raise-path.txt`）で、書き込み側は 0x4121f4 と 0x41ce5e の 2 つ。runtime の `fle_imp_RaiseException`（`fle_proc.c:255`）は `fle_stop` で止まる。
- **仮想にした場合**: `CopyStgMedium` が NULL と 127 → スロット 0 → **`f_707bd0` が呼ばれれば RaiseException（runtime では名前付き停止）**。**absent にした場合**: `LoadLibraryW` が NULL → 0x707c0e `je 0x707c2a` → スロット 0 → 同じ例外。**どちらでも同じ**（NULL は検査されるが、0 のままだと例外に進む）。0 番地は呼ばない。
- 呼び出し元: `f_7076f0`、`f_707740`（根）から `f_707650`、`f_707680` を経て到達する（`rawref-4.txt`: データ参照あり）。実行されるかは未確認。

### 4.6 ws2_32（unit に届く辺は無い。`f_c19370` が入口）

- ローダー `f_c079e0`（`fn-c079e0.txt`。ローダーは 0xc079f5 `cmp [0x14fd348],0; jne 0xc07aa7` で一度だけ）: 0xc07a0f `f_44db40(L"WS2_32.DLL", 0x8000)` → 0x14fd348。NULL（0xc07a1b `je 0xc07a52`）なら 0xc07a52 `GetLastError`、メッセージ生成（`0xc07b60`）、`0x411e80`（例外）。非 NULL なら 0xc07a2d `call [0x14fcf88]`（`WSAStartup(0x202, &data)`）。0x14fcf88 は `f_c07ef0` が `GetProcAddress("WSAStartup")` の結果を書くセル（0xc07fd5）。
- `f_c079e0` の呼び出し元は `f_c14b00`（0xc14b0f）と `f_c19370`（0xc193b2）。`f_c19370` は 0xc193b2〜0xc193c1 で `f_c079e0`、`f_c14b00`、`f_c171e0`、`f_c192e0` の順に呼ぶ（`fn-ip-1.txt`。実測）。つまり **ws2_32、wship6、fwpuclnt、idndl、normaliz、iphlpapi の 6 名は `f_c19370` の 1 回の呼び出しで順に引かれる**。
- `f_c19370` の参照元（**実測** `rawref-1.txt`、`fo2va-1.txt`、`vtab-c19370*.txt`）: `call`／`jmp`／`lea` の命令は 0 件。8 バイトの絶対アドレスとして 0xc17c98（`.text` 内のデータ）に 1 件、そこは `0xc17ba8`（`IdStackWindows`、`TIdSocketListWindows` の名前文字列が直前にある。`ripscan-idstack.txt`）の近くのポインタ表の 1 スロット。表の先頭（0xc17ba8）を読む関数は 1 つだけ、`f_c217b0`（unit **585**。`wdll_units.py` で index 585 の proc と確認。実測）で、0xc217c4 `mov rax,[0xc17ba8]` → 0x14ff838 に保存し、`0x481790`（クラスの登録）を呼ぶ。**読み**: これは Delphi の `TIdStackWindows` のクラス表で、`f_c19370` はそのメソッドのスロットから呼ばれる。ゲストが実際にそのメソッドを呼ぶ条件は静的には追えない。
- **ws2_32 を NULL にした場合（absent）**: 0xc07a1b で `je 0xc07a52` → 例外 → RaiseException に届き runtime で名前付き停止（`f_c079e0` が呼ばれれば）。
- **セル 0x14fcf88（`WSAStartup` のスロット）の初期化**（**実測** `c13010-ws2.txt`、`xref-ws2.txt`）: このセルを書く命令は 2 つだけ。0xc13645（`f_c13010` 内。直前の 0xc1363e `lea rax,[f_c07ef0]` の結果を書く）と、0xc07fd5（`f_c07ef0` 内。`GetProcAddress("WSAStartup")` の結果を書く）。読む（`call`）のは 0xc07a2d（`f_c079e0` 内）と 0xc07fe5（`f_c07ef0` 内）。`f_c13010` は unit 575 の proc `f_c137c0` が 0xc13b1a で呼ぶ（`rawref-3.txt`、`reach-c13010.txt`: units D=575）。**読み**: セルは unit 575 の実行後は `f_c07ef0`（`WSAStartup` の遅延ローダー）を指し、`f_c079e0` の `call [0x14fcf88]` は 0 番地ではなくこのローダーを呼ぶ。unit 575 より前に `f_c079e0` が実行される経路があるか、セルが ws2_32 のロード前に 0 のままか、は**未確認**（`f_c19370` が実行される時期が未確認のため）。
- **ws2_32 を仮想にした場合**: LoadLibraryW は非 NULL なので 0xc07a1b の検査を通り、0xc07a2d `call [0x14fcf88]`（上のとおり `f_c07ef0` を呼ぶ）。`f_c07ef0` は 0xc07f7a `f_c05820(handle, "WSAStartup")` で GetProcAddress を呼び、**応答表に行が無いので NULL と 127** → 0xc07f82 `test rax,rax; jne 0xc07fd5` の fall-through（0xc07f84〜0xc07fcd）で例外を作り 0xc07fd0 `call 0x411e80` → RaiseException（runtime では名前付き停止）。
- **結果の使い方**: 110 回の GetProcAddress（98 名）はすべて `f_c05820`（Indy のラッパー）経由で、**全部 `test rax,rax; jne` の fall-through に `call 0x411e80` がある**（`stubnull.tsv`。実測: 110/110）。名前の一覧は §4.6.1。**仮想にしても、GetProcAddress は応答表に行が無い名前で NULL と 127 を返すので、その呼び出しの時点で RaiseException に届く**。0 番地は呼ばない（呼ぶ前に例外が出る）。つまり **ws2_32 を仮想にしても、名前付き停止が LoadLibraryW から `RaiseException`（WSAStartup）へ移るだけ**（読み。分類 §6.1 と同じ結論）。

#### 4.6.1 ws2_32 の GetProcAddress 名（98 名。`names12.tsv`）

`accept bind closesocket connect gethostbyaddr gethostbyname gethostname getpeername getprotobyname getprotobynumber getservbyname getservbyport getsockname getsockopt htonl htons inet_addr inet_ntoa ioctlsocket listen ntohl ntohs recv recvfrom select send sendto setsockopt shutdown socket` と、`WSAAccept WSAAddressToStringA/W WSAAsyncGetHostByAddr/Name WSAAsyncGetProtoByName/Number WSAAsyncGetServByName/Port WSAAsyncSelect WSACancelAsyncRequest WSACancelBlockingCall WSACleanup WSACloseEvent WSAConnect WSACreateEvent WSADuplicateSocketA/W WSAEnumNameSpaceProvidersA/W WSAEnumNetworkEvents WSAEnumProtocolsA/W WSAEventSelect __WSAFDIsSet WSAGetLastError WSAGetOverlappedResult WSAGetQOSByName WSAGetServiceClassInfoA/W WSAGetServiceClassNameByClassIdA/W WSAHtonl WSAHtons WSAInstallServiceClassA/W WSAIoctl WSAIsBlocking WSAJoinLeaf WSALookupServiceBeginA/W WSALookupServiceEnd WSALookupServiceNextA/W WSANSPIoctl WSANtohl WSANtohs WSAProviderConfigChange WSARecv WSARecvDisconnect WSARecvFrom WSARemoveServiceClass WSAResetEvent WSASend WSASendDisconnect WSASendTo WSASetBlockingHook WSASetEvent WSASetLastError WSASetServiceA/W WSASocketA/W WSAStartup WSAStringToAddressA/W WSAUnhookBlockingHook WSAWaitForMultipleEvents`。呼び出し点は 0xc07f7a（`WSAStartup`）から 0xc127fe（`WSAProviderConfigChange`）の 110 点（A/W の対で同じ名前の語尾が違うものを別名として数えた。`sort -u` で 98）。

### 4.7 mswsock（`f_c13010` 経由。unit 575 の P）

- `f_c12ab0`（`fn-net-1.txt`）は WSARecvEx の遅延ローダー。0xc12afb `f_44db40(L"MSWSOCK.DLL")` → 0x14fd350。ロード後 0xc12b0f `jne 0xc12b66`（NULL なら 0xc12b11〜0xc12b61 で `GetLastError`、メッセージ、`0xc07b60`、`0x411e80`＝例外）。0xc12b66 でもう一度ハンドルを検査（NULL なら 0x276d のメッセージで例外）。0xc12bcf GetProcAddress(`WSARecvEx`)、0xc12bd7 `test rax,rax; jne 0xc12c2a`（NULL なら例外）。0xc12c2a でセル 0x14fd308 に保存して 0xc12c41 `call [0x14fd308]`。
- `f_c12ab0` は直接 call からは届かず、`f_c13010` が 0xc13542 `lea rax,[f_c12ab0]` → 0xc13549 `mov [0x14fd308],rax` で関数ポインタとして保存する（**実測** `c13010-ws2.txt`、`xref-others.txt`、`rawref-2.txt`）。`f_c13010` は unit 575 の proc `f_c137c0` が 0xc13b1a で呼ぶので、**`lea` を含めると unit 575 に届く**（P=575、D は `-`）。`f_c12ab0` 自体が呼ばれるのは、セル 0x14fd308 経由で `WSARecvEx` を使うときで、実行されるかは未確認。なお 0x14fd308 は `f_c12ab0` 自身が 0xc12c2a で `GetProcAddress("WSARecvEx")` の結果で上書きする（`xref-others.txt`）。
- 仮想にした場合: ハンドル非 NULL。`WSARecvEx` が NULL と 127 → 0xc12bd7 の fall-through で例外 → RaiseException（名前付き停止）。absent にした場合: 0xc12b0f の fall-through で例外 → 同じ停止。**どちらでも同じ（使われれば停止）**。0 番地は呼ばない。

### 4.8 iphlpapi（`f_c19370` 経由）

- `f_c192e0`（`fn-ip-1.txt`）: 0xc192e4 `cmp [0x14fee78],0; jne` で一度だけ、0xc192fa `f_44db40(L"iphlpapi.dll")`、0xc192ff で 0x14fee78 に保存。**ローダーに NULL 検査は無い**。
- 使う側 5 関数（`fn-iphl-stubs.txt`、`fn-ip-1.txt`）: `f_c18f40`（`GetIpAddrTable`）、`f_c18fe0`（`GetUniDirectionalAdapterInfo`）、`f_c19090`（`GetAdaptersInfo`）、`f_c19130`（`GetAdaptersAddresses`）、`f_c19200`（`ConvertLengthToIpv4Mask`）は、いずれも `xor eax,eax; cmp [0x14fee78],0; je` でハンドルが 0 なら `GetProcAddress` を飛ばし、`test rax,rax; jne` で結果が 0 なら**内蔵の代替**（`f_c18f30`、`f_c18fd0`、`f_c19080`、`f_c19110`、`f_c191e0`。それぞれ `[rdx]=0; eax=0x32`（ERROR_NOT_SUPPORTED）などを返す小さい関数）をスロット（0x14fee80〜0x14feea0）に入れて `call`。
- **仮想にした場合**: ハンドル非 NULL。5 名は NULL と 127 → 結果 0 → 内蔵の代替。**absent にした場合**: ハンドル 0 → 同じ代替。**どちらでも同じ状態**（`call [slot]` の slot には内蔵の代替のアドレスが入る）。0 番地は呼ばない。
- 終了側 `f_c19340`: 0xc19344 `cmp [0x14fee78],0; je`、非 0 なら FreeLibrary。仮想の場合は LoadLibrary で +1 した参照を戻すので、参照 2→1（読み）。

### 4.9 wship6（`f_c19370` 経由。**ws2_32 の GetAddrInfoW が NULL のときだけ**）

- `f_c14b00`（`fn-net-1.txt`）: 0xc14b06 `call 0xc079d0`（ws2_32 が読み込み済みかを `setne al` で返す）、`je` の場合 0xc14b0f `f_c079e0` を呼ぶ。0xc14b14 `call 0xc079c0`（ws2_32 のハンドルを返す）→ 0x14fea48。0xc14b2e GetProcAddress(`GetAddrInfoW`) の結果が NULL のときだけ（0xc14b39 `jne 0xc14b77`）、0xc14b47 `f_44db40(L"Wship6.dll")` → 0x14fea40、0xc14b5a でコピーして 0xc14b6f にもう一度 `GetAddrInfoW`。その結果も NULL なら 0xc14b7a `je 0xc14d16`（内蔵の代替の設定: 0xc14d1b〜0xc14d45）。
- **ws2_32 を仮想にして wship6 を未応答のまま**にすると: ws2_32 のハンドルは非 NULL で、0xc14b2e の `GetAddrInfoW` が**応答表に行が無く NULL**。0xc14b47 の `f_44db40(L"Wship6.dll")` は `fle_stop`（現状）。つまり **ws2_32 を仮想にすると wship6 の LoadLibrary に必ず届く**（`f_c14b00` が呼ばれれば）。
- **wship6 を仮想にした場合**: 0xc14b5a で 0x14fea48 に wship6 のハンドル、0xc14b6f の `GetAddrInfoW` が NULL → 0xc14b7a `je 0xc14d16` → 内蔵の代替。**absent にした場合**: 0x14fea40 が 0 → 0xc14b53 `mov rax,[0x14fea40]` → 0x14fea48 に 0 → 0xc14b61 `mov rcx,[0x14fea48]` → `f_c05820(0, "GetAddrInfoW")` → ゲストのラッパー `f_4235f0` が `GetProcAddress(hModule=0, …)` を呼ぶ → **runtime の `fle_imp_GetProcAddress` は hModule が仮想ハンドルでない（NULL）ため abort**（`fle_modules.c:199-205`。分類 §2.2 の注 5 と同じ形）。つまり **wship6 を absent にすると、ゲストが NULL を許容する設計でも runtime が abort する**（読み。実行していない）。
- 現行（ws2_32 が未応答のまま）では、この経路には届かない。

### 4.10 fwpuclnt（`f_c19370` 経由）

- 0xc14c5e `f_44db40(L"Fwpuclnt.dll")` → 0x14fea50。0xc14c6a `cmp …,0; je 0xc14d4c`（NULL なら 6 名の GetProcAddress を飛ばす）。6 名（0xc14c86〜0xc14d08）: `WSASetSocketSecurity`、`WSAQuerySocketSecurity`、`WSASetSocketPeerTargetName`、`WSADeleteSocketPeerTargetName`、`WSAImpersonateSocketPeer`、`WSARevertImpersonation`。結果は 0x14fea08〜0x14fea30 の 6 スロット。
- 仮想にした場合: 6 スロットが 0。absent にした場合: 飛ばして 0 のまま（`f_c13bf0`（0xc13c89〜0xc13cb5）が同じ 5 スロットを 0 にする初期化も持つ。実測 `fn-c13bf0.txt`）。
- 6 スロットの読み手: `wdll_cells.py` の走査では、書き込み（`W`）以外の読み手が**見つからなかった**（`cells-net.txt`: `slot 0x14fea08〜0x14fea30` の行に `direct W` 以外は無い。`grep -v "direct W"` で空）。基底からのオフセットで読む形、データセル経由の読みは走査していない（未確認）。
- NULL を検査せずに呼ぶ経路: **見つからなかった**（読み手が見つからなかったので、検査の有無は言えない）。

### 4.11 idndl（`f_c19370` 経由）

- 0xc171fa `f_44db40(L"IdnDL.dll")` → 0x14fedd0。0xc17206 `cmp …,0; je 0xc1725e`（NULL なら 3 名を飛ばす）。3 名（0xc1721e、0xc17238、0xc17252）: `DownlevelGetLocaleScripts`（0x14fed90）、`DownlevelGetStringScripts`（0x14fed98）、`DownlevelVerifyScripts`（0x14feda0）。
- 仮想にした場合: 3 スロットが 0。absent にした場合: 飛ばして 0。
- 3 スロットの読み手: **見つからなかった**（`cells-net.txt`。`f_c171e0` の書き込みと `f_c17510` の 0 書き込みだけ）。つまりゲストが `Downlevel*` を呼ぶ経路は、走査の範囲では無い（未確認の範囲を含む）。
- NULL を検査せずに呼ぶ経路: 見つからなかった。

### 4.12 normaliz（`f_c19370` 経由）

- 0xc17278 `f_44db40(L"Normaliz.dll")` → 0x14fedd8。0xc17284 `cmp …,0; je 0xc17314`。5 名（0xc172a0〜0xc17308）: `IdnToUnicode`（0x14fedb8）、`IdnToNameprepUnicode`（0x14fedc0）、`IdnToAscii`（0x14fedc8）、`IsNormalizedString`（0x14feda8）、`NormalizeString`（0x14fedb0）。
- 使う側（`fn-more-1.txt`）: `f_c170a0`: 0xc170bc `cmp [0x14fedc8],0; je`、0xc170c6 `cmp [0x14fedb8],0`、両方非 0 のときだけ `al=1` を返す（ゲストの「IDN が使えるか」を返す関数。`f_c1b710` が 0xc1b7ac で呼び、真なら `f_c170e0`、偽なら 0xc1b7c7 の代替（`call 0x413480`）へ進む）。`f_c170e0`: 0xc17107 `cmp [0x14fedc8],0; je 0xc171a0`（0 なら何もせず戻る）。非 0 のとき `call [0x14fedc8]`（0xc17142、0xc17182）。
- 仮想にした場合: 5 スロットが 0 → `f_c170a0` が 0 を返す → `f_c1b710` が代替の経路へ。absent にした場合: 同じ。**どちらでも同じ状態**。0 番地は呼ばない（読み）。
- 他の 3 スロット（`IdnToNameprepUnicode`、`IsNormalizedString`、`NormalizeString`）の読み手は、`f_c17510` の 0 書き込み以外は**見つからなかった**（`cells-net.txt`）。
- `f_c1b710` の参照元: `.text` 内のポインタ表（0x817070）の 1 スロットだけ（`rawref-2.txt`。実測）。実行されるかは未確認。

## 5. 分類

依頼の分類語: 「ゲストが NULL を許容し、absent でも仮想でも通る」「仮想にすると危険（NULL 非検査の呼び出しに届く）」「absent だと RaiseException」「到達しない」。以下は **読み**に基づく。「到達しない」は静的な辺で unit に届かない（前述の限界）ことを指し、実行されないとは言えない。

| # | 名前 | 分類 | 根拠（節） |
|---|---|---|---|
| 1 | avrt | **ゲストが NULL を許容し、absent でも仮想でも通る**。仮想にしても、11 スロットは 0 のまま、使う側は 4 関数とも検査してから呼ぶ。到達する unit は **145**（DllMain の次の最初の停止点） | §4.1 |
| 2 | powrprof | **同上**（absent でも仮想でも通る）。unit **145** | §4.2 |
| 3 | psapi | **到達しない**（エミュレート OS が 10.0 なら PSAPI.dll の LoadLibrary 点は実行されない。読み）。仮想でも absent でも、K32 版の GetProcAddress が NULL でも API は「無い」扱いで 0 を返す（NULL 許容） | §4.3 |
| 4 | riched20 | **absent でも仮想でも GetProcAddress は呼ばれない**。ただし NULL でも次の `GetClassInfoW(RICHEDIT20W)` に進み、runtime がそこで停止する（`fle_class.c:172-173`）。到達する unit は無い | §4.4 |
| 5 | urlmon | **absent でも仮想でも、使われれば RaiseException**（NULL は検査されるが、0 のままだと例外）。到達する unit は無い | §4.5 |
| 6 | ws2_32 | **absent だと RaiseException**（`f_c079e0` の NULL 経路）。**仮想でも RaiseException**（`GetProcAddress("WSAStartup")` が NULL で例外。110/110 点が NULL で例外に届く）。0 番地は呼ばない。到達する unit は無い（`f_c19370` はクラス表から参照される形） | §4.6 |
| 7 | mswsock | **absent でも仮想でも RaiseException**（NULL ハンドルも、`WSARecvEx` が NULL も例外）。静的には unit 575 の `lea` 経由 | §4.7 |
| 8 | iphlpapi | **ゲストが NULL を許容し、absent でも仮想でも通る**（5 関数がハンドルと結果の NULL を検査し、内蔵の代替へ）。到達する unit は無い | §4.8 |
| 9 | wship6 | **仮想なら通る（内蔵の代替へ）。absent だとゲストは許容でも runtime が abort する**（`GetProcAddress(NULL,…)`、`fle_modules.c:199-205`）。ws2_32 が仮想で `GetAddrInfoW` が NULL のときにだけ到達する | §4.9 |
| 10 | fwpuclnt | **ゲストが NULL を許容し、absent でも仮想でも通る**（ハンドルの検査あり）。結果 6 スロットを使う側は**見つからなかった**（未確認）。到達する unit は無い | §4.10 |
| 11 | idndl | **同上**（ハンドルの検査あり。結果 3 スロットの読み手は見つからなかった）。到達する unit は無い | §4.11 |
| 12 | normaliz | **ゲストが NULL を許容し、absent でも仮想でも通る**（`f_c170a0` と `f_c170e0` が検査）。到達する unit は無い | §4.12 |

**「NULL を検査せずに呼ぶ経路」と「仮想にすると 0 番地を呼ぶ危険」**（依頼の確認点）:
- 12 名のうち、**今回読んだ範囲で、仮想にしたときに 0 番地を呼ぶ経路は見つからなかった**。ws2_32 と urlmon と mswsock は NULL を検査して例外に進む（0 番地は呼ばない）。avrt、powrprof、iphlpapi、normaliz、psapi は呼ぶ前に検査する。
- ただし **見つからなかった**ものがある（未確認）: avrt の 8 スロット、fwpuclnt の 6 スロット、idndl の 3 スロット、normaliz の 3 スロットは、`wdll_cells.py` の走査（rip 相対とデータセル）では書き込み以外の読み手が見つからなかった。基底＋オフセットで読む形は走査していない。見つからないことは「検査される」ことを意味しない。
- 比較（分類 §6.2、決定 0086 が引く）: 同梱 DLL のうち Blend2D（`f_6cb0b0`）と QuickFontCache の 4 名は NULL 検査なしで 0 番地を呼ぶ。Windows の DLL 12 名には、今回読んだ範囲でその形は無い。

**DllMain 完走後に実行が届く順（引用。分類 §4、§7.1。仮の値つきの gdb 測定。今回は再測定していない）**: unit 145 の `f_65c9f0` で、`dwmapi`（absent）→ `user32`（GetModuleHandleW）→ **`avrt`** → `kernel32`（GetModuleHandleW）→ `advapi32` → **`PowrProf.dll`** → `SHCore.dll`（absent）の順。その次の LoadLibrary は unit 149 の `Blend2D.dll`。12 名のうち**この順で届くのは avrt と PowrProf.dll の 2 名だけ**。残り 10 名は、今回の静的到達では unit に届く辺が無い（ws2_32 ほか 6 名は `f_c19370`、psapi と riched20 と urlmon も辺なし。mswsock は `lea` 込みで unit 575）。

## 6. 決定が要る点（選択肢と事実。決めない）

選択肢: **(a)** 仮想モジュール表の末尾（N=31 以降）に足す（LoadLibraryW は成功、GetProcAddress は応答表に行が無ければ NULL と 127。決定 0015、0055、0081 の形）。**(b)** `fle_load_absent[]` に足す（NULL と 126）。**(c)** 停止のまま（現状）。

### 6.1 論点（PM が決めること）

1. **12 名を 1 つの方針で扱えるか、名前ごとに分けるか。** 事実: 12 名は同じ性質ではない。(i) 実行が先に届く名前（avrt、powrprof。unit 145）と、静的に届かない名前（残り 10）。(ii) Windows 10 にあると導出できる名前（avrt、powrprof、ws2_32、mswsock、iphlpapi、fwpuclnt、normaliz）、導出が弱い名前（psapi、urlmon）、一次に無い名前（riched20、wship6）、Windows 10 の標準ではない名前（idndl）。(iii) absent でも仮想でも同じ結果になる名前と、違う名前（§6.2 の表）。
2. **決定 0015/0029 の 9 名（dwmapi、shcore ほか）と 0055 の (c) の整合を取るか。** 事実は §2.3。この文書は「矛盾」と呼ぶかどうかを決めない。12 名を 0055 の (c) で決める場合、dwmapi と shcore は同じ導出で「Windows 10 にある」側に入る（unit 145 の `f_65c9f0` の同じ関数内で、avrt、PowrProf.dll と並んで引かれる。§5 の順）。
3. **GetProcAddress の行（`fle_imp_*`）を足すか。** 仮想にして行を足さなければ、全部 NULL と 127（0081 の api-ms と同じ）。足す場合は `fle_imp_*` の実装が要る（決定 0081 の「帰結」、`imports_table.rs` の先頭コメント「Only APIs whose runtime function is implemented and recorded in docs/decisions/ are listed here」）。
4. ws2_32 を仮想にする場合、ネットワーク層（WSAStartup 以降）をどうするか。事実は §4.6（110 点すべてが NULL で例外）。

### 6.2 選択肢を名前に当てた表（事実のみ）

「同じ」は、LoadLibraryW が NULL か非 NULL かの違いを除き、ゲストが GetProcAddress の結果として得る値とその後の経路が同じ、の意味（読み）。

| 名前 | (a) 仮想にした場合 | (b) absent にした場合 | (c) 停止のまま | 仮想にする場合に `fle_imp_*` の行は要るか |
|---|---|---|---|---|
| avrt | LoadLibraryW 成功。11 名 NULL と 127。ゲストは 11 スロット 0 で進む（使う側は検査）。**進む** | NULL。11 名を飛ばす。**進む** | unit 145 で `LoadLibraryW "avrt"` により停止（実測、引用） | 行が無くても進む（ゲストは検査してから呼ぶ。読み） |
| powrprof | 2 名 NULL。**進む**（検査あり） | NULL。**進む** | unit 145 で停止（実測、引用） | 行が無くても進む |
| psapi | 到達しない見込み（読み）。到達しても K32 が NULL で API「無し」 | 同左 | 到達しない見込み。ただし `load.rs:535` の固定リストにある | 要らない |
| riched20 | 成功。GetProcAddress は呼ばれない。次に `GetClassInfoW(RICHEDIT20W)` で停止（読み） | NULL。同じく `GetClassInfoW` で停止（読み） | 停止 | 要らない |
| urlmon | 成功。使われれば `CopyStgMedium` NULL → RaiseException | NULL。使われれば同じ RaiseException | 停止 | `CopyStgMedium` の行を作るなら、その関数の意味を runtime が作る必要がある（事実: `CopyStgMedium` は `STGMEDIUM` を複製する。Learn の archive ページ。実装の有無は未調査） |
| ws2_32 | 成功。`WSAStartup` が NULL → RaiseException（名前付き停止が移るだけ） | NULL → `f_c079e0` の例外経路 → RaiseException | 停止 | 98 名の行と実装が要る（ネットワーク層。大きさは見積もっていない） |
| mswsock | 成功。`WSARecvEx` が NULL → 例外 | NULL → 例外 | 停止 | 行と実装が要る（`WSARecvEx`） |
| iphlpapi | 成功。5 名 NULL。ゲストは内蔵の代替へ。**進む** | NULL。ゲストは内蔵の代替へ。**進む** | 停止 | 行が無くても進む |
| wship6 | 成功。`GetAddrInfoW` が NULL → 内蔵の代替。**進む** | NULL → **`GetProcAddress(NULL,…)` で runtime が abort**（ゲストは許容でも） | 停止 | 行が無くても進む |
| fwpuclnt | 成功。6 名 NULL。**進む**（結果 6 スロットの読み手は見つからなかった） | NULL。飛ばす。**進む** | 停止 | 要らない（読み手が見つからなかった） |
| idndl | 成功。3 名 NULL。**進む**（読み手は見つからなかった） | NULL。**進む** | 停止 | 要らない |
| normaliz | 成功。5 名 NULL。ゲストは `f_c170a0` が 0 を返し代替へ。**進む** | NULL。同じ。**進む** | 停止 | 行が無くても進む |

- **absent と仮想で結果が違う名前**: wship6（absent は runtime が abort、仮想は進む）。riched20、urlmon、ws2_32、mswsock は、どちらでも**同じ名前付き停止に届く**（停止の場所が移る）。
- **absent と仮想で同じ名前**: avrt、powrprof、iphlpapi、fwpuclnt、idndl、normaliz（進む）、psapi（到達しない）。

### 6.3 absent を選ぶときの前提（事実）

- 決定 0055 の (c) を満たすには、「Windows 10 に無い」と言えることが要る。12 名のうち、Learn が「Windows 10 に無い」と書いたページは**取得していない**（一次に無い）。
- 決定 0029 は、absent に足す場合を「PM の決定が先」と書く（行 71）。

### 6.4 仮想を選ぶときの前提（事実）

- 決定 0055 の流儀は、「Windows 10 には必ずある」と言えること（導出でよい。「導出」と明記）。12 名のうち導出できるのは avrt、powrprof、ws2_32、mswsock、iphlpapi、fwpuclnt、normaliz（Requirements の DLL 欄と最小 OS）。psapi、urlmon は導出が弱く、riched20、wship6 は一次に無く、idndl は Windows 10 の標準とは導出できない（§3.2）。
- 仮想にするなら N=31 以降に**末尾追加**し、並べ替えない（決定 0015）。順序がハンドルを決める。

## 7. 実装するとしたらの影響範囲（読み。列挙のみ）

実装はしていない。**何かを決めたとしたら影響を受ける場所**だけを挙げる。行番号は HEAD（`c75acc4`）。`grep -n` の出力で確認した。

### 7.1 仮想モジュールにする（(a)）場合に影響を受ける場所

固定数 30 や N の値を決め打ちしているテスト（`lift/crates/fe-lift/tests/`）:

| 場所 | 内容 | 仮想モジュールを足すと |
|---|---|---|
| `modules.rs:376` | `assert_eq!(lines[0], "count 30")` | 数が変わる（31 以上）。要更新 |
| `modules.rs:377`、`383` | `lines[1..31]`、`lines[31]`（表の終わりが nil） | 範囲が変わる。要更新 |
| `modules.rs:217`（コメント 214） | `*v > 0x7FF8_0000_0000 + 0x10000 * 30`（擬似 VA がハンドル領域より上） | N=30 を上限と仮定。下の 7.3 のとおり、N=42 でも不等式は成り立つ（算術）が、値が現状の上限ではなくなる |
| `modules.rs:530` | `handle-past-table` が `h(30)`（表の外のハンドルで abort する期待） | N=30 は表の中になる。`h(最終 N + 1)` へ要更新 |
| `msctf.rs:342` | `assert_eq!(got[0], "count 30")` | 要更新 |
| `msctf.rs:258` | `fle_module_name(30)`（表の外が NULL の期待） | 要更新 |
| `msctf.rs:269`、`429` | `handle-31`（`M(31)` が表の外で abort する期待） | N=31 は表の中になる。要更新 |
| `msctf.rs:178`、`339`、`343`〜`345` | `N_MSCTF = 26`、名前 `name25`、`name24`、`handle25` | 追加は末尾なので値は変わらない（読み） |
| `load.rs:446` | `fle_load_refcount(30)` が 0（表の外の期待） | 要更新 |
| `load.rs:522`〜`539` | `external_and_unnamed_modules_stop_by_name` の 14 名のうち **7 名が 12 名に含まれる**（`avrt`、`PowrProf.dll`、`PSAPI.dll`、`RICHED20.DLL`、`URLMON.DLL`、`iphlpapi.dll`、`ws2_32.dll`）。「止まる」を期待 | (a) でも (b) でも、この 7 名のうち決めた名前は期待が変わる。要更新 |
| `event.rs:261`、`glob.rs:205`、`vol.rs:41` | `mods_end = 0x7FF8_0000_0000 + 0x10000 * 30`（他のハンドル領域が上にあること） | N=30 を上限と仮定。N が増えても領域の大小関係は保たれる（下の 7.3） |
| `realmod.rs:30`（コメント 29）、`28` | `API_SET_HANDLES`（N=27..30）、`KERNEL32_HANDLE`（N=9） | 追加は末尾なので値は変わらない（読み） |
| `tests/realmod_s1/check_runs.py:7` | `KERNEL32_HANDLE = 0x7FF800000000 + 0x10000 * 9` | 同上 |

- テストが「N の個数」ではなく表を引く形のもの: `modules.rs:365`（ループ）、`msctf.rs:254`、`load.rs:482`（`fle_module_count()` を使う）。個数を直接書く行は上の表のもの。
- 他に 0x7FF800010000 を hInstance／hModule に使うテスト（`class.rs:635`、`cursor.rs:399`、`loadimage.rs:524`）は、N=1（`advapi32.dll`）のハンドルで、追加で値は変わらない（読み）。
- 取りこぼしの可能性: `grep` の式に当たらない書き方（変数経由）はある。網羅したとは言えない（未確認）。

runtime（`lift/runtime/`）:
- `fle_modules.c:16-50` の `fle_modules[]` の末尾に名前を足す（順序がハンドルを決める。コメント 9-15、決定 0015）。
- `fle_load.c:33` `LOAD_MODULES_MAX 64`: 30 + 12 = 42 で収まる（算術）。`refs_init`（`fle_load.c:67`）は超過で `fle_stop`。
- `fle_realmod.c:51-56`: 実在モジュールの名前が仮想モジュールの名前と重なると `fle_stop`（実在モジュールが同名を持たないこと。12 名は同梱 DLL ではないので重ならない見込み。読み）。

### 7.2 absent にする（(b)）場合に影響を受ける場所

- `fle_load.c:53-57` の `fle_load_absent[]` に行を足す（`LOAD_ABSENT_N` は `sizeof` から出るので数のテストは無い）。
- `msctf.rs:410` `the_nine_lacking_names_still_fail_with_126`（テスト名と `["0 126"; 9]` が 9 名を前提にする。決定 0055 の「撤回条件」も同じテスト名を挙げる）。`load.rs:373`〜`387` `load_library_w_of_a_delay_only_dll_fails_with_126`（10 個の入力、9 名＋ `DWMAPI.DLL` 別表記）。
- `load.rs:522`〜`539`（上の 7 名）。
- `fle_load.c:47-52` のコメント（「Nothing else is added here: … avrt, powrprof, psapi, riched20, urlmon, ...」）。

### 7.3 ハンドル値の衝突（算術と読み）

- 仮想モジュールのハンドルは `0x7FF800000000 + 0x10000 × N`（`fle_modules.h:14-15`）。N=31..42 は `0x7FF8001F0000`〜`0x7FF8002A0000`。応答の擬似 VA の領域は `0x7FF900000000` 以降（`imports_table.rs` の `RESP_VA_BASE`）で、N の上限（`LOAD_MODULES_MAX` 64 なら `0x7FF800400000`）より大きい。`0x7FF900000000` に達する N は 65536（算術）。**衝突しない**。
- 他のトークン領域（`FLE_VFS_FILE_BASE 0x7FFA00000000`、`FLE_VOL_HANDLE_BASE 0x7FFA60000000` ほか）は 0x7FFA… 以上で、N が増えても 0x7FF900000000 未満なので衝突しない（算術。`event.rs`、`glob.rs`、`vol.rs` のテストが `mods_end` との大小を検査する。N が増えると `mods_end` の定数 `* 30` が実態とずれる）。
- 実在モジュールの基底は `base < FLE_MODULE_HANDLE_BASE`（`fle_realmod.c:57`）。0x7FF800000000 未満なので、仮想ハンドルと重ならない（読み）。
- `fle_vfs.h:13` のコメントは「N <= 25」と古い（現在は 30）。コメントだけで挙動には影響しない（読み）。

### 7.4 FLEngine の生成 C の SHA

- 今回、既存のビルド `lift/target/debug/fe-lift`（2026-10-23 13:13 のビルド。再ビルドしていない）で `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata` を実行した。出力 `lift/out-wdll/gen/flengine-head.c` の SHA-256 は **`43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`**（実測。決定 0078 が記録した値と一致）。サイズ 215,707,512 B。
  - 注意（実測）: このバイナリのビルド時刻は、ジェネレータのソース（`lift/crates/fe-lift/src`、`fe-decode`、`fe-pe`）の最終コミット `7b693f5`（2026-10-23 15:31。決定 0078 の応答表 33 行）より**前**。つまり今回の SHA は、現在のソースから作ったものとは言い切れない。ただし値は決定 0078 の記録（`7b693f5` の時点のソースで作った値）と一致しており、`git diff --stat 87bc74d HEAD -- lift/crates/fe-lift/src lift/crates/fe-decode lift/crates/fe-pe` は空（実測。`7b693f5` 以降ソースの変更なし）。バイナリがどのソースから作られたかは未確認（再ビルドしていない）。「現在のソースの出力が `43985d62…` である」は、この実行からは**未確認**で、決定 0078 の記録との一致が根拠。
- **応答表の行を足すと、この SHA は変わる**。前例は決定 0078（`lift/out-w6/…`）: 「応答表に 33 行を足すと、FLEngine の `fle_resp_table` と `fle_dispatch_table` の擬似 VA の末尾にも 33 行が付く」ため `67827424…` から `43985d62…` に変わった（0078 本文。引用）。
- **仮想モジュールを足すだけ（応答表に行を足さない）なら、生成 C は変わらない**（読み。`fle_modules[]` は runtime で、生成 C は runtime に依存しない。決定 0081 の実装コミット `87bc74d` の題「FLEngine C unchanged」と、0078 の「生成 C は runtime に依存しない」から）。`absent` に足すのも `fle_load.c` だけなので同じ（読み）。
- 応答表（`imports_table.rs` の `TABLE`）に GetProcAddress の行を足す場合は SHA が変わる。足す行数分、`fle_resp_table`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_count` が変わり、その API の `jmp [IAT]` が FLEngine に在れば thunk も変わる（0078 の差分の内訳）。12 名の API は FLEngine の normal import 表（IAT）に無い（実測: `AvSetMmThreadCharacteristicsW`、`PowerGetActiveScheme`、`PowerSetActiveScheme`、`WSAStartup`、`CopyStgMedium`、`GetAddrInfoW`、`GetAdaptersInfo`、`IdnToAscii`、`NormalizeString`、`DownlevelVerifyScripts`、`K32GetProcessMemoryInfo`、`GetProcessMemoryInfo`、`GetMappedFileNameW` の 13 名を `wdll_pe.iat_names()` で引いて 0 件）ので、`jmp [IAT]` の thunk は変わらない見込み（読み。決定 0078 の thunk 6 つは kernel32 の名前で、12 名の API とは別）。
- `tests/` で SHA を固定しているものは、`grep -rn 43985d62 tests/*.rs` で **0 件**（実測）。SHA は決定 0078 と `docs/` の記録にあるだけ。

## 8. 限界・未確認

**未完了の項目**（時間と確認手段の都合で取り切れなかったもの。事実としては書いていない）:

1. **実行していない**。この文書のゲストの振る舞いはすべて静的な読み。gdb でのビルド・実行は依頼で不要とされ、していない。分類 §7.1 の「DllMain 後の最初の停止点」の順（avrt、PowrProf.dll）は前回の仮の値つきの測定の引用で、今回は再測定していない。仮の値なしで DllMain が完走するか、完走後にこの順で届くかは未測定（決定 0086 の「現在」の記述も、DllMain が `GetStringTypeW` で止まると書く）。
2. **「読み手が見つからなかった」スロット**: avrt の **8 スロット**（`AvSetMmMaxThreadCharacteristicsW`（0x15f40f0）、`AvQuerySystemResponsiveness`（0x15f4108）、`AvRtCreateThreadOrderingGroup`（0x15f4118）、`AvRtCreateThreadOrderingGroupEx`（0x15f4120）、`AvRtDeleteThreadOrderingGroup`（0x15f4128）、`AvRtJoinThreadOrderingGroup`（0x15f4130）、`AvRtLeaveThreadOrderingGroup`（0x15f4138）、`AvRtWaitOnThreadOrderingGroup`（0x15f4140）。`cells-avrt-powrprof.txt` で `via-cell`、`direct CALL`、`direct C`、`direct R` の行が 0 件。読み手が見つかったのは `AvSetMmThreadCharacteristicsW`（0x15f40f8）、`AvSetMmThreadPriority`（0x15f4100）、`AvRevertMmThreadCharacteristics`（0x15f4110）の 3 つ）、fwpuclnt の 6 スロット、idndl の 3 スロット、normaliz の 3 スロット（`IdnToNameprepUnicode`、`IsNormalizedString`、`NormalizeString`）。走査は rip 相対とデータセル（`wdll_cells.py`）だけで、基底＋オフセットで読む形は走査していない。見つからないことは「検査される」ことを意味しない。**これらの NULL 検査の有無は未確認**。
3. **`f_c19370`（ws2_32、wship6、fwpuclnt、idndl、normaliz、iphlpapi の入口）が実行される時期と条件**。参照元は `.text` 内のポインタ表（Delphi のクラス表。`TIdStackWindows` の名前文字列が近くにある）の 1 スロットだけで、静的に unit に届く辺は無い。ゲストがこのメソッドを呼ぶ条件（ネットワーク機能の使用）は未確認。ws2_32 のセル `0x14fcf88` が 0 のまま `f_c079e0` が実行される経路があるかも未確認。
4. **`f_426310` の VerifyVersionInfoW が実際に真を返すか**（psapi の読み）。`fle_osver_check` を読んで major 10 > 6 から導いたが、実行していない。決定 0014 の本文にある「実行がこの結果で先へ進んだ」は別の呼び出し（0x409bd0）。
5. **Learn のページ**: 取得できなかった（404）のは `…/urlmon/nf-urlmon-copystgmedium`、`…/winnls/nf-winnls-downlevelverifyscripts`、`…downlevelgetlocalescripts`（後 2 つは `…/intl/…` の URL で取得できた）。**未取得**: avrt の 8 名、ws2_32 の `WSAStartup`、`getaddrinfo` 以外の名前、iphlpapi の `GetUniDirectionalAdapterInfo` の最小 OS 欄（ページは取得したが空欄）。Learn が「Windows 10 22H2 に avrt.dll などが存在する」と明記したページは無い（導出のみ）。riched20 と wship6 について Windows 10 での有無を言う一次は無い。
6. **インストーラ以外の入手経路**（別インストーラ、ダウンロード、実行時に配置）で 12 名の DLL がゲストに見える可能性は未確認（Windows の DLL なのでインストーラは配らない読みで、分類 §5.2 の確認の範囲）。
7. **取りこぼし**: 静的到達のグラフは前回の `edges-direct.tsv`、`edges-ptr.tsv` の再利用（再生成していない）。vtable、レジスタ経由の呼び出し、コールバックは辺に無い。`grep` による固定数のテストの列挙は網羅の保証が無い（変数経由の書き方）。
8. **`lift/target/debug/fe-lift` が現在のソースから作られたか**は未確認（§7.4）。SHA `43985d62…` は決定 0078 の記録との一致が根拠。
9. **実機の Windows 10 22H2 での挙動**（これらの DLL が実際にあるか、`LoadLibraryW` が成功するか）は、ホストが Linux のため確認していない。決定 0015、0055、0081 も「実機 Windows 未確認」と書く。
10. 12 名以外の名前（`SetThreadErrorMode` など `f_44db40` の中の呼び出し、ゲストの動的な名前）、および `GetModuleHandleW` が引く名前（12 名のどれも GetModuleHandleW のリテラルには無い。実測: `sites-flengine-recheck.tsv` を 12 名で grep した結果は avrt の GetProcAddress の名前が引っ掛かった 1 行だけ）は扱っていない。

**限界**:
- ハンドルは実メモリの住所ではない（決定 0015）。ゲストが仮想ハンドルからヘッダを読むと SIGSEGV になる。12 名の経路はハンドルを `GetProcAddress` と `FreeLibrary` と NULL 検査にだけ使う（読み）ので、この限界に当たる経路は見つけていない。
- API セット／forwarder の話（`api_location` に並ぶ `api-ms-win-*`、`KernelBase.dll`）は api-ms 調査の範囲なので書いていない。
- 非公式の資料（Wine、Geoff Chappell など）は使っていない。

## 9. 意見（別枠。事実ではない。決めない。PM の判断の材料）

この節は作業者の意見で、事実の節ではない。§2〜§8 の事実から導いたもので、確認していない推論を含む。

1. **avrt と PowrProf.dll は、決定 0055 の流儀（msctf.dll）に従うなら (a) 仮想モジュール**が自然だと考える。理由: (i) Learn の Requirements から Windows 10 にあると導出でき、(b) absent は (c) と逆の主張になる。(ii) 仮想にしても、読んだ範囲ではゲストは検査してから使い（0 番地を呼ぶ形は見つからなかった）、結果は absent と同じ状態（スロット 0）になる。(iii) 決定 0081 の api-ms の 4 名と同じ形で、実装は表の末尾 2 行（N=31、32）。ただし **avrt の 8 スロットと PowrProf の使う側の一部は読み手が見つからなかった**ので、「検査される」とは言い切れない（§8 の 2）。仮想にした場合の実行の確認（gdb）は、この文書では行っていない。
2. **dwmapi と shcore の整合は PM が決める点**（§2.3、§6.1 の 2）。今回の 12 名のうち avrt と PowrProf.dll は、同じ関数 `f_65c9f0`（unit 145）の中で dwmapi、shcore と並んで引かれる。avrt と PowrProf を仮想、dwmapi と shcore を absent にすると、同じ関数の中で扱いが分かれる。実行は通る（どれも NULL を検査する）が、環境の主張としては揃わない。揃えるなら (i) 12 名のうち導出できるものは仮想にし、0015 の 9 名のうち導出できるもの（dwmapi、shcore）も仮想に直す、(ii) 0015 の 9 名は据え置いて、新しい名前だけ 0055 に従う、のどちらかになる。どちらも PM の決定が先（決定 0029「足す場合は PM の決定が先」）。
3. **wship6 は absent にしない方がよい**と考える。理由: absent だと、ゲストが NULL を許容するのに runtime が `GetProcAddress(NULL,…)` で abort する（§4.9。読み）。これは決定 0029 の「NULL を許容するなら absent でよい」の前提を壊す。仮想にすれば内蔵の代替へ進む。ただし wship6 は Windows 10 に「ある」とも「無い」とも一次に無く、ws2_32 が仮想で `GetAddrInfoW` が NULL のときだけ到達する（ws2_32 の扱いが先）。
4. **ws2_32 と mswsock は、仮想にしても停止が移るだけ**（RaiseException）。ネットワーク層（98 名の `ws2_32` の API、`WSARecvEx`）を応答表に足すかは別の決定で、規模が大きい。今は静的に unit に届かない（`f_c19370` はクラス表から参照される形）ので、**当面は触らない**のが妥当と考える。ws2_32 を仮想にすると、**同じ `f_c19370` 経由の wship6、fwpuclnt、idndl、normaliz、iphlpapi の 5 名にも届くので、それらを決めてからでないと ws2_32 の仮想化は進まない**。順序は逆で、先に 5 名を決める方が安全（ws2_32 が止まる限り 5 名は届かない）。
5. **psapi は到達しない見込み**（§4.3。エミュレート OS が 10.0 のため）なので、足す必要が当面は無い。riched20 は GetProcAddress を呼ばず、次に `GetClassInfoW(RICHEDIT20W)` で止まるので、窓クラスの決定（0070、0071）と一緒に扱う方が自然。urlmon は使われれば `CopyStgMedium` を引く（行を作るなら関数の意味を runtime が作る）。
6. **idndl は「無い」と定義しても Learn と矛盾しない**（Vista より前の OS 向けの再頒布物）が、「Windows 10 に無い」と書いたページは取得していない。結果 3 スロットの読み手も見つからなかったので、仮想でも absent でも実行の差は見つかっていない。
7. **12 名を 1 つの方針にはできない**と考える。事実（§6.2）として absent と仮想で結果が違うのは wship6 だけだが、(c) の根拠の強さが名前ごとに違う（導出できる 7 名、弱い 2 名、一次に無い 2 名、標準ではない 1 名）。1 つの方針（例: 全部仮想）にすると、一次に無い名前（riched20、wship6）や標準ではない名前（idndl）を「Windows 10 にある」と主張することになる。
8. **すぐに必要なのは avrt と PowrProf.dll の 2 名だけ**（DllMain の後に届く。引用）。この 2 名と、同じ関数に並ぶ dwmapi、shcore の整合が決まれば、DllMain の先は次の停止点（`Blend2D.dll`、決定 0086）へ進む。残り 10 名は、実行が届いたときに決めてよい（決定 0086 の 11 名の同梱 DLL と同じ扱い）。

## 10. 作ったファイルと実行したコマンド

### 10.1 作ったファイル

文書: `docs/investigations/windows-dll-names-virtual-or-absent.md`（本書。新規）。

`lift/out-wdll/`（新規。`.gitignore` の `lift/out*/` で無視される。`__pycache__` は 0 個。`PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=lift/out-wdll/tmp`）:
- スクリプト（読み取り専用。DLL は `rb` で開くだけ）: `wdll_pe.py`（前回の `lift/out-loadlib/llpe.py` のコピー。capstone と PE 読み）、`wdll_gpa.py`（関数内の LoadLibrary／GetProcAddress 呼び出し点）、`wdll_gpa_all.py`（GetProcAddress ラッパー全呼び出し点）、`wdll_names12.py`（12 名の保存先ごとの名前一覧）、`wdll_stubnull.py`（ws2_32 の 110 点の NULL 経路が `0x411e80` に届くか）、`wdll_xref.py`（スロットの rip 相対参照）、`wdll_cells.py`（スロット＋データセル経由の読み手）、`wdll_reach.py`（unit への静的到達。前回のグラフを読む）、`wdll_rawref.py`（関数開始アドレスの生バイト参照: rel32、abs64、rva32、lea）、`wdll_fo2va.py`、`wdll_vtab.py`、`wdll_ripscan.py`（ポインタ表の周辺）、`wdll_units.py`（初期化表を DLL から直接読み、前回の `units.tsv` と照合）、`wdll_imports12.py`（import 表と遅延 import 表の DLL 名）、`wdll_slotinit.py`（スロットの初期値）、`wdll_dumpfn.py`（`.pdata` 関数の逆アセンブル）、`wdll_learn.py`（Learn の取得。新規ファイルだけ書く）。
- 出力: `names12.tsv`（12 名の保存先ごとの GetProcAddress 呼び出し 155 点＋ヘッダ。実測）、`gpa-all.tsv`（586 点＋ヘッダ）、`stubnull.tsv`（ws2_32 の 110 点）、`gpa-map-1.txt`、`reach-12.txt`、`xref-*.txt`、`cells-*.txt`、`fn-*.txt`（関数ダンプ）、`rawref-*.txt`、`units-own.txt`（895 行を照合、不一致 0）、`imports12.txt`、`slotinit.tsv`、`learn-run-1..4.txt`（Learn の取得ログ）、`learn/`（Learn の生ページ 26 ファイル）、`c13010-ws2.txt`、`raise-path.txt`、`gen/flengine-head.c`（215,707,512 B。FLEngine の生成 C。SHA の確認用）ほか。
- 前回の `lift/out-loadlib/` の出力（`units.tsv`、`edges-direct.tsv`、`edges-ptr.tsv`、`sites-flengine-recheck.tsv`）は読み取りだけで、編集していない。

### 10.2 実行した主なコマンド（要約）

- `git rev-parse HEAD`、`git log`、`git status --short`、`git diff --stat 8b2fc8e HEAD -- …`、`git diff --quiet HEAD -- FILE`（変更系の git 操作は無し）。
- `sha256sum binaries/win64/FLEngine_x64.dll lift/out-wdll/gen/flengine-head.c`。
- `python3 wdll_*.py …`（上のスクリプト。`cwd` は `lift/out-wdll`。`lift/out-cs/` では実行していない）。
- `lift/target/debug/fe-lift lift binaries/win64/FLEngine_x64.dll lift/out-wdll/gen/flengine-head.c --pdata`（既存のビルド。再ビルドしていない。emcc は使っていない）。
- `grep -n`／`grep -rn`／`sed -n`（テストと runtime の行の確認）。
- Learn の取得（`wdll_learn.py`。`urllib` で匿名の GET。`learn.microsoft.com` だけ。資格情報、cookie、ブラウザは使っていない）。

### 10.3 規則の遵守

- `rm` は使っていない。既存ファイルの全体上書き（`>`）は無し（新規ファイルへのリダイレクトだけ）。`__pycache__` は作っていない。`/tmp` は使っていない。
- `docs/decisions`、`docs/README.md`、`docs/progress.md`、既存の `docs/investigations/*`、`lift/runtime`、`lift/crates`、fe-lift、fe-decode、fe-pe は編集していない（`git status` は §10.4）。
- 触っていない: ctype 調査（`docs/investigations/dsp-ippv2-ctype-locale-data.md`、`lift/out-ctype/`）、`lift/out-a5-m85/`。

### 10.4 `git status`

文書の完成時点（HEAD `c75acc4`）の `git status --short`:

```
?? docs/investigations/dsp-ippv2-ctype-locale-data.md       ← 別作業（ctype 調査）の未追跡ファイル。私のものではない
?? docs/investigations/windows-dll-names-virtual-or-absent.md  ← 本書（私が作った）
?? lift/harness/wasm/trace.c.new                            ← 別作業の未追跡ファイル。私のものではない
?? lift/harness/wasm/trace_native.c.new                     ← 同上
?? lift/iat-unsup-closure.txt                               ← 同上
```

`git diff --stat`（追跡ファイルの変更）は **空**。私は追跡ファイルを変更していない。`lift/out-wdll/` は `.gitignore` の `lift/out*/` により `git status` に出ない。`docs/progress.md` は読んでいない（`git status` には変更が出ていない。作業の途中で一時的に ` M docs/progress.md` が出た時点があったが、その後 HEAD の `c75acc4` に取り込まれた。別作業の変更で、私のものではない）。
