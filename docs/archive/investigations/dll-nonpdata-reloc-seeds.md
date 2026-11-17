# dsp_ippv2_x64.dll の「`.pdata` 開始でない reloc 値」（CRT 初期化表の 5 個ほか）の調査

実行日: 2026-10-23（UTC。JST では 10-24）。HEAD `9ae09a03d57bc6ad9f60c54bd0ee89333c380b6f`（main）。決定 0074、0079、`stage23-two-module-real-dll.md`（以下「stage23」）§3.1・§9 の 1、`two-module-post0083.md`（以下「post0083」）§6・§9 の 4 の続き。**この文書は何も決めない**。決定が要る点は §8 に列挙した。

作業物は `lift/out-dll-seeds/`（`.gitignore` の `lift/out*/` に該当）。fe-lift・fe-decode・fe-pe のソース、lift/runtime、docs/decisions、既存のテスト、既存の docs は編集していない。新規ファイルはこの文書と `lift/out-dll-seeds/` の下だけ。

## 0. 凡例と対象

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-dll-seeds/logs/`、データは `lift/out-dll-seeds/data/`、解析スクリプトは `lift/out-dll-seeds/py/`。
- **読み** = capstone の逆アセンブル、ソース、Learn のページを読んだ結果。実行して確かめていない。
- **推測** = 読みからさらに推し量ったもの。
- **未確認** = 確認できていない。
- 対象 DLL: 原本 `lift/out-ext/dsp_ippv2_x64.dll`（SHA-256 `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352`、ImageBase 0x180000000）。リベース済み `lift/out-two-post0083/dll/dsp_ippv2_x64.2000000.dll`（SHA-256 `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`、ImageBase 0x2000000。`sha256sum` で両方確認）。**アドレスは断りがない限りリベース後の VA**（原本の VA は 0x17E000000 を足したもの）。
- 補助対象: `binaries/win64/FLEngine_x64.dll`（SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`）。§7 の影響の見積もりでだけ使った。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 5 個は `.pdata` に含まれるか | **含まれない**。`.pdata` 全 50,529 エントリの線形走査で、どの RUNTIME_FUNCTION の範囲にも入らず、連鎖エントリの親（UNWIND_INFO 内の RUNTIME_FUNCTION）の範囲にも入らない。隣り合うエントリの間の 11〜128 バイトの隙間にある | 実測 |
| 2 | 5 個の中身 | 4 個は `call` もスタック操作も無い 3〜25 バイトの関数（`mov al,1; ret` など）。1 個（0x37d243c）は `xor ecx,ecx; jmp` で `.pdata` 開始の関数へ飛ぶ。リーフ関数として説明できる | 読み |
| 3 | 5 個の参照元 | 5 個ともコードからの直接参照（`call`/`jmp rel32`、RIP 相対 `lea`）は **0 件**。参照は CRT 初期化表のスロット（DIR64）だけ | 実測 |
| 4 | `.pdata` 開始でない reloc 値 | **2,750**（`fe-decode reloc-seeds --not-function` と自作解析が一致）。うち (i) 既に `.pdata` の範囲内 1,206、(ii) どの範囲にも入らない **1,544**。5 個は (ii) | 実測 |
| 5 | デコード | 2,750 個すべてで capstone が先頭 16 命令（または最初の `ret`/`jmp`/`int3`/`ud2`/`hlt`）まで不正バイト無しで読めた | 実測 |
| 6 | 閉包（`--pdata`、リベース済み DLL） | (A) 基準 32,648 関数 / (B) +5 個で 32,653 / (C) +(ii) 1,544 個で **33,638**。`bad_nodes` は全構成で **0**。`unsupported_total` は A・B が 974,527、C が 992,445 | 実測 |
| 7 | DllMain(ATTACH) で呼ばれる 5 個 | 0x37d23b4 は ATTACH の走査が呼ぼうとした**実測**（種 428 個の実行が、0x37d23b4 を `fle_dispatch` に渡した所で止まる。stage23 §3.1、post0083 §6。本体は実行されていない）。0x37d23b0、0x37da928 は、ATTACH で走る走査関数が読むスロットにあるので呼ばれる**読み**。0x37d243c、0x37d77c8 は、DETACH 側の走査関数が読むスロットだけにあるので、ATTACH の成功経路では呼ばれない**読み**。5 個それぞれの実行回数は**未確認**（関数単位の計数をしていない） | 実測・読み・未確認 |
| 8 | 規則「pdata 範囲に入らない reloc 値を種にする」を生成器に足した影響 | この DLL は上の (C)。**FLEngine_x64.dll に同じ規則を無条件に適用すると `bad_nodes` が 0 → 113,485**（関数 59,297 → 115,378）。規則を DLL だけに限る条件が要る | 実測 |
| 9 | PE のメタデータで CRT 初期化表の位置が取れるか | 表 B（`.CRT$XIA`〜`XIZ` ほか）は、デバッグディレクトリの型 13（先頭 `GCTL`）の中に名前つきの範囲として載る。**表 A（5 個のうち 4 個を持つ 16 バイト組の表）は名前が無く、取れない**。TLS ディレクトリなし、CFG の関数表なし、COFF シンボルなし | 実測（データの存在）、未確認（型 13 の仕様） |

## 2. 手順（実行したコマンド）

すべて repo ルートから実行。`export TMPDIR=$PWD/lift/out-dll-seeds/tmp`（`/tmp` は使っていない）、python は `PYTHONDONTWRITEBYTECODE=1`、cargo は `CARGO_TARGET_DIR=lift/out-dll-seeds/target`、`--offline`。

```
git rev-parse HEAD                                              # 9ae09a03…
cd lift && cargo build --release --offline -p fe-pe -p fe-decode -p fe-lift
fe-pe pe-info / pe-pdata-info  <リベース済み DLL>               # entries 50529, chained 19713, function_starts 30816
fe-decode reloc-seeds <DLL> --not-function > data/reloc-not-function.txt   # 2,750 行
fe-decode reloc-seeds <DLL>                 > data/reloc-seeds.txt         # 7,731 行
fe-lift lift <DLL> /dev/null --pdata                                     > logs/A-pdata.txt
fe-lift lift <DLL> /dev/null --pdata --seeds data/seeds-five.txt         > logs/B-pdata-five.txt
fe-lift lift <DLL> /dev/null --pdata --seeds data/seeds-ii.txt           > logs/C-pdata-ii.txt
```

- 出力 C は `/dev/null`（0074 と同じ。統計の `c_bytes` は書かずに数えた値）。**生成 C は保存していない**（最大のものは 553 MB で、保存すると 2 GB 級にはならないが、統計だけで足りた）。関数の集合を比べるため、`fe-lift lift … /dev/stdout` の出力を `grep` に通して `void f_<VA>(void) {` の行だけを `data/funcs-*.txt` に保存した（C 本体はファイルに残していない）。
- 解析スクリプト（`lift/out-dll-seeds/py/`）は、pefile でヘッダとセクションだけを読み、`.pdata`、UNWIND_INFO（連鎖）、`.reloc` は自前で解析した。fe-pe とのクロスチェック: エントリ 50,529、連鎖 19,713、関数開始（連鎖を除く）30,816 は `fe-pe pe-pdata-info` と一致。2,750 の集合と 7,731 の集合は `fe-decode reloc-seeds` の出力と一致（`assert`）。原本（ImageBase 0x180000000）の 2,750 は、0x17E000000 を引くとリベース済みの 2,750 と一致。
- 各スクリプトのログ: `logs/q1_*.txt`（5 個）、`q2_*.txt`（分類）、`q3_*.txt`（閉包の比較）、`q4_*.txt`（呼び出し経路）、`q5_*.txt`（メタデータと FLEngine）。
## 3. 項目 1: 5 個はなぜ `.pdata` 開始でないのか

### 3.1 `.pdata` との関係（実測。`logs/q1_five.txt`、`q1_fullscan.txt`）

- `.pdata`（EXCEPTION ディレクトリ、RVA 0x2563000、サイズ 0x9408c）は 50,529 エントリ（余りなし）。**エントリ同士の重なり 0、`end <= begin` のエントリ 0、UNWIND_INFO の下位ビット（間接）が立つエントリ 0**。
- 5 個それぞれについて、全 50,529 エントリを線形に走査して `begin <= rva < end` を調べた: **該当 0**。連鎖エントリ 19,713 個の UNWIND_INFO が持つ親 RUNTIME_FUNCTION の範囲も調べた: **該当 0**。`begin == rva` のエントリも 0。
- つまり 5 個は**関数の途中でもない**。`.pdata` が覆っていない隙間にある:

| VA | 直前のエントリ（idx、begin–end） | 次のエントリ（idx、begin） | 隙間 |
|---|---|---|---|
| 0x37d23b0 | 43851、0x37d2354–0x37d23af | 43852、0x37d23c8 | 0x37d23af〜0x37d23c8（25 B）。0x37d23b0 と 0x37d23b4 が両方ここ |
| 0x37d23b4 | 同上 | 同上 | 同上 |
| 0x37d243c | 43854、0x37d23fc–0x37d2439 | 43855、0x37d2444 | 0x37d2439〜0x37d2444（11 B）。前後は `cc` |
| 0x37d77c8 | 43963、0x37d77ac–0x37d77c8 | 43964、0x37d77d4 | 0x37d77c8〜0x37d77d4（12 B）。直前のエントリの end と一致 |
| 0x37da928 | 44016、0x37da8c0–0x37da8d0 | 44017、0x37da950 | 0x37da8d0〜0x37da950（128 B）。隙間の中に 0x37da928 を含め小さな関数が 5 個並ぶ（下） |

### 3.2 先頭の逆アセンブル（読み。capstone 5.0.7。`logs/q1_context.txt`）

「読み」は命令列の読み取りで、実行していない。

```
0x37d23b0: mov al, 1 ; ret                                              (3 B)
0x37d23b4: lea rax,[rip+0xd71335]   (= 0x45436f0 .data)
           mov [rip+0xd8f406], rax  (= 0x45617c8 .data)
           mov al, 1 ; ret                                              (17 B)
0x37d243c: xor ecx, ecx ; jmp 0x37cd260                                 (7 B)
0x37d77c8: and qword [rip+0xd8a850], 0   (= 0x4562020 .data)
           mov al, 1 ; ret                                              (11 B)
0x37da928: mov eax,[rip+0xd8773a]   (= 0x4562068 .data)
           mov ecx, 0x4000 ; test eax,eax ; cmove eax,ecx
           mov [rip+0xd8772a], eax  (= 0x4562068)
           xor eax, eax ; ret                                           (25 B)
```

- 5 個とも、`call` を含まず、`rsp` を変えず、`push`/`pop` も無い（`logs/q1_leaf_ii.txt`、`q_final_stats.txt`）。0x37d243c だけは末尾が `jmp`（tail call）で、飛び先 0x37cd260 は `.pdata` 開始（idx 43761、`sub rsp,0x28` で始まる関数）。
- **リーフ関数としての説明（読み）**: Microsoft Learn「x64 exception handling」（2026-10-23 に取得）は「テーブル方式の例外処理は、スタックを確保するか他の関数を呼ぶ関数（non-leaf）すべてに関数テーブルのエントリを要求する」と書き、「テーブルエントリが見つからなければ、そのコードはリーフ関数の一部とみなし、`[RSP]` が戻りアドレスだと仮定する」と書く。「x64 stack usage」は、リーフ関数を「関数テーブルのエントリを要しない関数。呼び出しもスタック確保もできない」と定義する。5 個は、命令列がこの 2 ページの条件（呼ばない、スタックを確保しない）に合う。したがって `.pdata` にエントリが無いことと矛盾しない。**コンパイラ（またはアセンブラ）が実際にそう判断して省いたかは確認できない**（推測）。
- 0x37da928 の隙間（0x37da8d0〜0x37da950、128 B）には、同様に小さな関数が並ぶ（読み）: `ldmxcsr` で `ret`（0x37da8d0）、`stmxcsr; and; ldmxcsr; ret`（0x37da8da）、`ucomisd … cvtsd2si/cvtsi2sd; ret`（0x37da8ee）、1 バイトずつ数える文字列長のループ（0x37da910）、0x37da928。0x37da8d0、0x37da8da、0x37da8ee、0x37da910 の 4 個は DIR64 の値になっていない（実測。`reloc-not-function.txt` にも `reloc-seeds.txt` にも無い）ので、reloc 値を使う規則では種にならない。0x37da928 だけが DIR64 の値（表 B の 0x3ebd2a0）。
- 5 個は 16 バイト境界ではない（0x37d23b0 だけが境界）。前後が `cc`（int3）の詰め物で、各関数が 4 バイト境界などの細かい配置で並ぶ。

### 3.3 どこから参照されるか（実測。`logs/q1_slots.txt`、`q4_refs.txt`）

- 5 個を指す DIR64 スロットはすべて `.rdata`（リベース後の VA）: 0x37d23b0 ×6（0x3ec8390、0x3ec8398、0x3ec83f8、0x3ec8418、0x3ec8440、0x3ec8458）、0x37d23b4 ×1（0x3ec8360）、0x37d243c ×1（0x3ec83c8）、0x37d77c8 ×1（0x3ec83b8）、0x37da928 ×1（0x3ebd2a0）。
- **コード側からの直接参照は 0 件**: 実行セクション全体のバイト列を走査し、`e8`/`e9`（rel32 の `call`/`jmp`）の飛び先と、RIP 相対 `lea` の指す先が 5 個に一致するものを探した（0 件）。見つけられるのは `rel32` と RIP 相対 `lea` の 2 形だけ。`mov reg, imm64`、データ経由の間接参照などは走査していない。
- 5 個は 2 つの表の項目として現れる:
  - **表 A**（`.rdata` 0x3ec8360〜0x3ec8460、16 バイトの組が 16 個）。組の第 1 スロットと第 2 スロットで、5 個のうち 4 個を持つ。0x3ec8360 側から数えた組の番号:

| 組 | 第 1 スロット | 第 2 スロット |
|---:|---|---|
| 0 | **0x37d23b4** | 0 |
| 1 | 0x37d23fc | 0 |
| 2 | 0x37d7734 | 0x37d7768 |
| 3 | **0x37d23b0** | **0x37d23b0** |
| 4 | 0x37d5678 | 0x37d56dc |
| 5 | 0x37d77ac | **0x37d77c8** |
| 6 | 0 | **0x37d243c** |
| 7 | 0x37d33a8 | 0x37d33e4 |
| 8 | 0x37d36ac | 0x37d36e8 |
| 9 | 0x37d7020 | **0x37d23b0** |
| 10 | 0x37d6c5c | 0 |
| 11 | 0 | **0x37d23b0** |
| 12 | 0 | 0x37d2354 |
| 13 | 0 | 0x37d2444 |
| 14 | **0x37d23b0** | 0x37d23ec |
| 15 | 0x37d23c8 | **0x37d23b0** |

  - **表 B**（`.rdata` 0x3ebd270〜0x3ebd2e8 付近の `.CRT$X*` 領域）。0x3ebd288〜0x3ebd2a0 の 4 スロットが 0x37d2aa4、0x37d4f70、0x37d9e4c、**0x37da928**。
- 表 A の非 0 スロットは 25（組の第 1 が 12、第 2 が 13）、表 B の項目は 4。コードを指す distinct な値は表 A が 20、表 B が 4、合計 24。24 個のうち `.pdata` 開始は 19、そうでないのが 5 個（今回の対象。`logs/q_extra2.txt`）。

### 3.3.1 走査関数との対応（読み）

- 表 A: `0x37d2484`（`lea rdx,[0x3ec8460]; lea rcx,[0x3ec8360]; jmp 0x37d77d4`）が順方向の走査関数 0x37d77d4（`.pdata` 開始）へ入る。0x37d77d4 は 16 バイト刻みで各組の**第 1 スロット**を読み、非 0 なら呼ぶ（`call [0x3ebd260]` → `0x37de9d0` → `jmp [0x3ebd258]` → `0x37de9b0: jmp rax`。ガード付き間接呼び出しの経路）。`al` が 0 を返すと止まって巻き戻す。
- 0x37d7868（`.pdata` 開始）は逆方向の走査関数で、末尾から 16 バイト刻みで各組の**第 2 スロット**（`[rbx-8]`）を `ecx = 0` で呼ぶ。入口は `0x37d24bc`（`lea rdx,[0x3ec8460]; lea rcx,[0x3ec8360]; jmp 0x37d7868`。引数 `cl` が 0 のとき）で、0x37d24bc の直接の呼び元は 0x37cb618 内の 0x37cb62d のみ。0x37cb618 の直接の呼び元は 0x37caaf0 内の 0x37cab52 のみで、0x37caaf0 は DllMain の `reason == 0`（DETACH）の経路（0x37ca988 の `test edx,edx; je` の先）と、ATTACH の失敗時の巻き戻し（0x37cac25）から呼ばれる。したがって**第 2 スロットは DETACH か、ATTACH の失敗の巻き戻しで呼ばれる**。
- 5 個のうち 0x37d243c（組 6 の第 2）と 0x37d77c8（組 5 の第 2）は**第 2 スロットだけ**にある。ATTACH が成功する経路では呼ばれないと読める（読み。呼ばれたことは確認していない）。
- 0x37d23b4（組 0 の第 1）と 0x37d23b0（組 3 と 14 の第 1）は第 1 スロットにあり、順方向の走査で呼ばれる。post0083 §3.1 のスタックでは、仮の値なしの停止が組 10 の第 1（0x37d6c5c）の中なので、組 0〜9 の第 1 は走査で呼ばれていた（読み。走査は逐次）。種 428 個だけの run が組 0 の 0x37d23b4 で止まるのは実測（`lift/out-two-post0083/run/idx0-imports.txt` 48 行目: rax = 0x37d23b4、rcx = 0x3ec8360、rdx = 0x3ec8460）。
- 表 B: `0x37ca9d8` 内の `lea` が表 B の範囲を 2 つの走査関数に渡す（実測のアドレス、読みの対応）。0x37caa46 の `lea rcx` が **0x3ebd280**（`.CRT$XIA`）、0x37caa3f の `lea rdx` が **0x3ebd2a8**（`.CRT$XIZ`）で `call 0x37d1694`、0x37caa66 の `lea rcx` が **0x3ebd270**（`.CRT$XCA`）、0x37caa5f の `lea rdx` が **0x3ebd278**（`.CRT$XCZ`）で `call 0x37d1650`（`logs/q1_walkers.txt`）。0x37d1694 は 8 バイト刻みで各スロットを呼び、**戻り値 `eax` が非 0 なら止める**形（`_initterm_e` 相当の形。読み）。0x37da928 は `xor eax,eax` で 0 を返すので、この形に合う。0x37da928 が ATTACH で実際に呼ばれたかは未確認。なお post0083 の run（仮の値なし）は表 A の組 10 で止まり、表 B の走査の前。stage23 の仮の値 5 段の run は DllMain が `eax = 1` で戻った（stage23 §5.1）ので、その走査を通った可能性が高いが、0x37da928 の計数は無い。
## 4. 項目 2: `.pdata` 開始でない reloc 値 2,750 個の分類

### 4.1 集合（実測。`logs/q2_classify.txt`、`q_final_stats.txt`）

| 項目 | 値 |
|---|---|
| DIR64 スロット | 15,602（ABSOLUTE 22。DIR64 と ABSOLUTE 以外の型は 0） |
| 値が実行範囲内（`.text` 0x2001000〜0x37df140、`IPPCODE` 0x37e0000〜0x3ebc771。いずれもファイル上のバイトがある範囲） | distinct **10,481**（`.text` 9,841、`IPPCODE` 640） |
| うち `.pdata` 関数開始（連鎖エントリは主エントリの開始で数える） | **7,731**（`.text` 7,091、`IPPCODE` 640） |
| うち `.pdata` 開始でない（本調査の対象） | **2,750**（すべて `.text`。`IPPCODE` は 0） |
| 実行範囲外を指すスロット | 741 スロット |
| 対象 2,750 個を指すスロットの所在 | `.rdata`、`.data`、`.text`（§4.3） |
| DIR64 スロットの所在（全 15,602） | `.rdata` 8,983、`.data` 6,163、`.text` 456 |

### 4.2 分類の定義

`.pdata` の RUNTIME_FUNCTION（連鎖の有無を問わず全 50,529 エントリ）の範囲 `begin <= rva < end` を使う。

| 分類 | 定義 | 備考 |
|---|---|---|
| (i-a) | 連鎖エントリ（UNW_FLAG_CHAININFO）の `begin` と一致する | 連鎖エントリは関数の途中の断片（0074 は開始に数えない）。主エントリは別 |
| (i-b) | 連鎖でないエントリの範囲の途中（`begin` より後）に入る | 関数の途中 |
| (i-c) | 連鎖エントリの範囲の途中に入る | 関数の断片の途中 |
| (ii) | どの RUNTIME_FUNCTION の範囲にも入らない | リーフ関数の候補（Learn の読み、§3.2） |

(i) は 3 つに分けた。依頼の分類 (i)「既に `.pdata` 開始の関数の途中を指すもの」は (i-a)+(i-b)+(i-c)。(i-a) は、`.pdata` のエントリの `begin` そのものを指すが、そのエントリが連鎖で、0074 の `function_starts` には数えられないもの。

### 4.3 件数（実測。`data/nonpdata-classified.tsv`、`data/cat-*.txt`）

| 分類 | 件数 | 値の所在 | 値を指すスロットの所在 | 16 バイト境界 | 先頭 16 命令（capstone）の結果 |
|---|---:|---|---|---:|---|
| (i-a) 連鎖エントリの先頭 | 69 | `.text` 69 | `.rdata` 69 | 2 | 不正バイト 0 |
| (i-b) 関数の途中 | 271 | `.text` 271 | `.rdata` 271 | 24 | 不正バイト 0 |
| (i-c) 断片の途中 | 866 | `.text` 866 | `.rdata` 866 | 62 | 不正バイト 0 |
| **(i) 小計** | **1,206** | | | 88 | |
| **(ii) 範囲に入らない** | **1,544** | `.text` 1,544 値の数で `.data` 930、`.rdata` 554、`.text` 60（その値を指す全スロットが同じセクションにあるもの。混在は 0） | 1,534 | 不正バイト 0 |
| **合計** | **2,750** | | | | |

- 件数の突き合わせ: 69 + 271 + 866 + 1,544 = 2,750。`fe-decode reloc-seeds --not-function` の 2,750 と値が一致（`assert`）。
- 「先頭 16 命令」は、各値から capstone で読み、最初の `ret`/`jmp`/`int3`/`ud2`/`hlt` か 16 命令のどちらか早い方まで進んで、不正バイトで止まったものを数えた。**2,750 個すべてで不正バイト 0**（この読みは「命令の列として読める」ことを示すだけで、関数の入口であることの証明ではない）。
- (i) の 1,206 個は、各値を含む RUNTIME_FUNCTION（(i-a) は自分自身）の `begin` から線形走査（capstone）して、命令境界に当たるかを確認した: (i-a) 69/69、(i-b) 271/271、(i-c) 866/866 が命令境界。
- (i) の値はすべて表（DIR64 が 3 つ以上連続）の中にあり（`q2_more.txt`: 69/69、271/271、866/866）、`jmp [R + I*8 + disp32]` の形の表（基底が ImageBase の `disp32` つき。決定 0079 §6 の「155 か所」の形に近いもの）の移動先でもある: (i-a) 69/69、(i-b) 271/271、(i-c) 866/866、(ii) 0/1,544（`q_extra2.txt` の自作の走査。形を `ff a4 <SIB: scale=8>` と基底 = ImageBase + disp32 の位置が `.rdata`/`.data` にある形に限った近似。0079 の 155 か所とは同じ数え方ではない）。つまり **(i) の値は、関数の途中に入る switch の case ラベル（飛び先）である読み**（推測。確認していない）。決定 0079 §6 は、この形の jmp をまだ解いていない（`Jmp: indirect target unresolved` 183 か所の主要部分）。

### 4.4 (ii) 1,544 個の内訳（実測）

由来で分けた（`stub_hits` = 決定 0079 のスタブ形 `movsxd; lea; jmp [r10+rax*8]` を 434 個再現して、それとの関係を取った）:

| 由来 | 件数 | 内容 |
|---|---:|---|
| `stub-16` | 434 | スタブの 16 バイト手前の初期化 `call`（決定 0079 §1 の添字 −1 のスロットの値）。`call rel32` で始まり、`jmp [mem]` で終わる。434 個すべてが 16 バイト境界。**どの `.pdata` の範囲にも入らず、export でもない**（`q_last.txt`: stub-16 の値が export VA に一致 0/434。stub 自身（+16）が export VA に一致 432/434） |
| `stub-table-entry` | 548 | スタブの表の添字 0 以上のエントリで `.pdata` 開始でないもの（決定 0079 §1 の「905」の一部。§4.5） |
| `other` | 557 | 上のどれでもない（スロットは `.rdata` 549、`.data` 8。16 バイト境界 551） |
| `five`（今回の 5 個） | 5 | `other` と別に数えた（5 個は `other` の定義にも該当するが、区別して数えている） |
| **合計** | **1,544** | 434 + 548 + 557 + 5 |

- (ii) のうち、`--pdata` の閉包（A）に**既に関数として入っているもの**: 558（`stub-table-entry` 548 全部、`other` 10）。**入っていないもの 986**（`stub-16` 434、`other` 547、`five` 5）。`stub-16` の 434 個は 0 個が閉包 A に入っている（添字 −1 のスロットは、決定 0079 §3 のとおり `switch` の外で、閉包の種にもならない）。
- (ii) の先頭の形（読み。`q1_leaf_ii.txt`）:
  - 呼び出しもスタック操作も無い（最初の `ret`/`jmp` まで、最大 24 命令）: 1,098 / 1,544。
  - `call` を含む: 434（すべて `stub-16`）。`rsp` を変える命令を含む: 12（`stub-table-entry`。`push -6; pop rax` の形（読み。`push imm8; pop` で値を作っているように見えるが、用途は確認していない）。
  - 最初の `ret`/`jmp` で止まった形: `ret` 271+12、`jmp`（即値）50、`jmp`（間接）873（`stub-16` の 434 は `jmp [mem]`、`stub-table-entry` の 432 は `jmp reg`（CPU 振り分けの本体へ入る形）、7 件は `other` の `jmp [mem]`）、24 命令で未終了 338。
- **リーフ関数として説明できるか（読み）**: 閉包 A に入っていない 986 個のうち `stub-16` の 434 個は、決定 0079 §1 の形（`call rel32; nop（11 バイト）; movsxd; lea; jmp [r10+rax*8]`）の先頭、つまり**スタブの 16 バイト手前の `call` の位置**。命令列は `call`、`nop`、`movsxd`、`lea`、`jmp [r10+rax*8]` の 5 命令で、`call` を含む。Learn の定義（リーフは他の関数を呼ばない）には合わない。この `call` の飛び先 4 個（0x37e08c0、0x3839810、0x3b30cb0、0x3cb9f80。(C) の閉包に入る 4 個。§5.1）は、どれも 10 命令の同じ形（`push rcx; push rdx; push r8; push r9; call 本体; pop r9; pop r8; pop rdx; pop rcx; ret`）で、これも `rsp` を変える。`call` の先（0x20034d0、0x2001040、0x335fd10、0x35b45a0）は、4 つとも `.pdata` 開始（`logs/q3_stub16_callees.txt`、`q3_stub16_callee_targets.txt`。読み）。0x2001040 は stage23 §6 が選択値の変数を書く関数と同じ。したがって「`.pdata` に無い (ii) はすべてリーフ関数」とは言えない。無い理由は未確認（推測: アセンブリで書かれた thunk に `.pdata` を作らなかった。確認していない）。
- (ii) の 49 個（`stub-16`、すべて 16 バイト境界、スロットは `.text`）は、`.pdata` の隙間の先頭からの線形走査で命令境界に当たらない（`q2_extra.txt`）。これは**隙間の途中にデータ（`.text` 内の DIR64 スロット = 表）が埋まっているため**（3 つの隙間（0x335de80〜0x335e0e0、0x3363b50〜0x33642c0、0x35b34d0〜0x35b4340）に `.text` の DIR64 スロットが含まれる。`q2_sweep49.txt`）。個々の値の先頭は不正バイト無しで読める（上の「不正バイト 0」）。ほかの (ii) 1,495 個は、隙間の先頭からの線形走査でも命令境界に当たる。
- 16 バイト境界でない (ii) は 10 個: 5 個（今回の対象）と、0x37cb214、0x37cb228、0x37d161c、0x37d16dc、0x37d2658、0x37db31c（`logs/q_final_checks.txt`）。後者のうち 0x37cb228 は `ret 0`（`c2 00 00`）で、GuardCFCheckFunctionPointer のスロット 0x3ebd248 が指す関数（LoadConfig の値。§6.2）。

### 4.5 決定 0079 の「905」との関係（実測。`logs/q2_check0079.txt`、`q2_why0079.txt`）

決定 0079 §1 は、スタブの表の distinct な移動先 2,555 のうち「`.pdata` の開始 1,650、開始でない 905」と書く。本調査の定義（連鎖エントリの主エントリの開始を数える）では、**2,007 が開始、548 が開始でない**（原本とリベース済みで同じ）。差は、0079 の測定スクリプト（`lift/out-w3d/tabscan.py`。pefile の `DIRECTORY_ENTRY_EXCEPTION` を使う）の数え方による。pefile（2024.8.26）の `DIRECTORY_ENTRY_EXCEPTION` は 29,274 エントリで終わった（実測。生の `.pdata` は 50,529 エントリ。29,274 番目以降の 21,255 件を pefile は返さない。`logs/q4_callers.txt` の最後）。再現: pefile の `BeginAddress` 集合を「`.pdata` 開始」として使うと 1,650 / 905 になり、0079 の値と一致する（`q2_why0079.txt` の最後の再計算）。**pefile が 29,274 で止まった理由は未確認**（`parse_exceptions_directory` のソースに `break` が複数あることを読んだだけ。どれが働いたかは調べていない）。したがって 0079 の「905」は、読み残した後半のエントリの開始を「開始でない」と数えた分を含む、と言えるところまで（その分の内訳は未確認）。**0079 §4 の閉包の増加（+486 関数）は fe-pe の `function_starts`（全 50,529 を読む）を使う `--pdata` の結果なので、その数値には影響しない**（今回の `--pdata` の閉包 A = 32,648 関数は 0079 §4 の「変更後」と一致）。
## 5. 項目 3: 種を足した閉包の大きさ（fe-lift で測定）

### 5.1 依頼の 3 構成（実測。リベース済み DLL、`--pdata`、`--seeds` は下表のとおり。出力 C は `/dev/null`）

| 項目 | (A) `--pdata` のみ（基準） | (B) + 5 個 | (C) + (ii) 全件（1,544 個） |
|---|---:|---:|---:|
| 種ファイル | なし | `data/seeds-five.txt`（5） | `data/seeds-ii.txt`（1,544） |
| `functions` | **32,648** | **32,653**（+5） | **33,638**（+990） |
| `closure_instructions`（閉包の命令数） | **6,464,799** | **6,464,817**（+18） | **6,489,933**（+25,134） |
| `emitted_instructions` | 6,511,234 | 6,511,252 | 6,536,368 |
| `bad_nodes` | **0** | **0** | **0** |
| `unsupported_total` | **974,527** | **974,527** | **992,445**（+17,918） |
| `tail_calls` | 22,594 | 22,595 | 23,046 |
| `gotos` | 450,886 | 450,886 | 451,080 |
| `c_lines` | 6,707,922 | 6,707,970 | 6,738,996 |
| `c_bytes`（数えた値。書いていない） | 536,742,456 | 536,744,446 | 538,754,704 |
| `jumptabs`（決定 0079） | 434 | 434 | 434 |
| `jumptab_rejected` の行 | 0 | 0 | 0 |
| 所要時間 / 最大 RSS（`/usr/bin/time -v`） | 7.83 s / 728,992 KB | 7.68 s / 729,200 KB | 7.62 s / 731,880 KB |

- ログ: `logs/A-pdata.txt`、`B-pdata-five.txt`、`C-pdata-ii.txt`（と `.time`）。
- **基準 (A) の突き合わせ**: 決定 0079 §4 の「変更後」（`fe-lift lift … --pdata`）は functions 32,648、closure_instructions 6,464,799、emitted_instructions 6,511,234、tail_calls 22,594、gotos 450,886、bad_nodes 0、`unsupported_total` 974,582、c_lines 6,707,823 と書いた。今回は上のとおり functions、closure_instructions、emitted_instructions、tail_calls、gotos、bad_nodes が**一致**し、`unsupported_total` は 974,527（0079 の 974,582 と 55 差）、`c_lines` は 6,707,922（0079 の 6,707,823 と 99 差）。**差の出どころは未確認**（0079 の測定後に fe-lift の生成が変わった: 決定 0077、0078 の実装 `7b693f5` が `fle_imp_*` の呼び出しを変えており、`unsupported` の数え方と行数に効く、とは読めるが、バイナリを入れ替えて確かめていない）。**原本（ImageBase 0x180000000）でも同じ数**: `logs/G-orig-pdata.txt`（functions 32,648、closure 6,464,799、bad_nodes 0、unsupported_total 974,527。`c_bytes` は 566,300,846 で、リベース済みの 536,742,456 より大きい。VA の桁数が違う文字列長の差と読めるが、確認していない）。
- (B) の +5 は**ちょうどその 5 個**（`funcs-B` − `funcs-A` = 0x37d23b0、0x37d23b4、0x37d243c、0x37d77c8、0x37da928。実測）。命令は +18 個（3 + 17 + 7 + 11 + 25 バイトの命令列）。**(B) で `unsupported_total` は増えない**（5 個の関数の命令はすべて対応済み）。5 個を種にした後の生成 C には `f_37d23b4` の本体（`L_37d23b4: { /* 0x37d23b4 Lea */ R[0] = UINT64_C(0x45436f0);}`）と dispatch 表の行（`{ UINT64_C(0x37d23b4), f_37d23b4 }`）が出る（`logs/B-defs-check.txt`）。種なし（A）の生成 C には両方とも無い（0 行）。stage23 §3.1 の `f_37d23b4`、`f_37d23b0` が 0 行、という記述と一致する。
- (C) の +990 関数の内訳（実測）: (ii) の種 1,544 個のうち **558 個は A に既にある**ので、新しい種は **986 個**（`stub-16` 434、`other` 547、`five` 5）。残り 4 個は、種から呼ばれて入った関数（`call` の閉包）: **0x37e08c0、0x3839810、0x3b30cb0、0x3cb9f80**（`.pdata` 開始ではなく、`.pdata` の範囲にも入らない。`stub-16` の `call` の飛び先 4 通りに一致し、決定 0079 §1 の「初期化の `call` の先は 4 通り（選択値の変数と 1 対 1）」と同じ。読み: 先頭は `push rcx/rdx/r8/r9; call 本体; pop r9/r8/rdx/rcx; ret`。§4.4）。新しい関数の所在: `.text` 986、`IPPCODE` 4。
- (C) で増えた未対応の種類（`logs/q3_unsup_diff.txt`）: 3 種類が**新規**（A に無い）: `Vexpandps` ×4、`Ret: ret imm16` ×2、`Vfmsub132ss` ×1。`ret imm16` の 2 件は、種 0x37b53c0 と 0x37cb228 の `ret 0`（`c2 00 00`）。残りの増加は、既存の種類（`Vmovsd`、`Vmovss`、`Vmovupd` など）の数が増えたもの（増えた種類 80、増分の合計 17,918）。(B) は新規の種類も増加も無い。

### 5.2 `bad_nodes` について（依頼の「(C) で増える場合」）

**この DLL では (C) の `bad_nodes` は増えない（0 のまま）**。依頼の「種を半分ずつに分けて測る（最大 6 回）」は DLL では要らず、行っていない。DLL の原因特定の測定回数は 0 回。

FLEngine_x64.dll では同じ規則で増えた。§7.2 に別に書く（`bad_nodes` の原因の切り分けを 4 回行った）。

### 5.3 追加で測った構成（依頼外。種の切り口を比べるため。実測）

| 構成 | 種 | functions | closure_instructions | bad_nodes | unsupported_total |
|---|---:|---:|---:|---:|---:|
| (A) `--pdata` | — | 32,648 | 6,464,799 | 0 | 974,527 |
| (A') `--pdata --no-adjustors` | — | 32,645 | 6,464,793 | 0 | 974,527 |
| (B) +5 個 | 5 | 32,653 | 6,464,817 | 0 | 974,527 |
| (F) + `stub-16` のみ | 434 | 33,086 | 6,465,707 | 0 | 974,527 |
| (E) + (ii) から `stub-16` を除く | 1,110 | 33,200 | 6,489,025 | 0 | 992,445 |
| (C) + (ii) 全件 | 1,544 | 33,638 | 6,489,933 | 0 | 992,445 |
| (C2) + (ii) のうち、値を指すスロットの少なくとも 1 つが非実行セクションにあるもの | 1,484 | 33,578 | 6,489,789 | 0 | 992,445 |
| (D) + 2,750 個全部（0074 の「参考」と同じ） | 2,750 | 34,844 | 6,572,807 | 0 | 1,009,730 |

- (D) は 0074 の記録（関数 34,844、命令 6,572,807、`unsupported_total` 1,048,471、bad_nodes 0）と、関数と命令が**一致**。`unsupported_total` は 1,009,730 で、0074 の 1,048,471 と違う（0074 より後のコミット 0075 のレガシー SSE の実装などで減った、と読める。確認していない）。
- (E) の増分 552（33,200 − 32,648）と (F) の増分 438（33,086 − 32,648）の和は 990 で、(C) の増分 990 と一致する（互いに素な集合）。(F) の 438 は種 434 + `stub-16` の `call` の飛び先 4 個（§5.1）。(E) の 552 は新しい種 552（(ii) の新しい種 986 − `stub-16` 434）。
- (C2) は (C) より 60 種少ない: 「値を指す全スロットが実行セクション（`.text`）にある」60 個を除いた。5 個は残る（§7.3）。

### 5.4 種を渡さない lift と、種 428 個（`idx0`）の lift との比較（参考。実測）

`--pdata` なしで、`--seeds` だけを渡す構成。決定 0079 の `switch` が表の全エントリを関数にするので、428 個の `idx0` は種なしと同じ閉包になる（stage23 §1 の 3）。

| 構成（`--pdata` なし） | 種 | functions | closure_instructions | bad_nodes | unsupported_total |
|---|---:|---:|---:|---:|---:|
| 種なし（エントリと export。`logs/H-noseeds.txt`） | 0 | 7,992 | 3,038,109 | 0 | 391,396 |
| `idx0`（`logs/R1-idx0.txt`） | 428 | 7,992 | 3,038,109 | 0 | 391,396 |
| `idx0` + 5 個（`R2`） | 433 | 7,997 | 3,038,127 | 0 | 391,396 |
| `idx0` + (ii) 全件（`R3`） | 1,888 | 9,050 | 3,073,569 | 0 | 409,655 |
| `tbl2`（`idx0` + 表のコードポインタ 24。post0083 の構成。`R4`） | 452 | 8,030 | 3,038,781 | 0 | 391,396 |
| reloc seed（7,731）のみ（`Y1`） | 7,731 | 16,790 | 5,089,080 | 0 | 737,331 |
| reloc seed（7,731）+ (ii) 全件（`Y2`） | 9,275 | 17,796 | 5,115,801 | 0 | 755,393 |

- `tbl2`（post0083 の 8,030 関数、`unsupported_total` 391,396）は post0083 §2 の値（8,030 関数、`bad_nodes` 0）と**一致**。
- CRT 表の項目（コードを指す distinct な値 24 個。表 A の 20 + 表 B の 4）のうち、閉包に**入っていない個数**（`logs/q_extra2.txt`、`q_missing_Y.txt`）:

| 構成 | 閉包に入っていない表項目 |
|---|---:|
| `idx0`（`--pdata` なし） | 22 |
| `idx0` + 5 個 | 17（`.pdata` 開始の 17 個） |
| `idx0` + (ii) 全件 | 17（同じ 17 個） |
| `tbl2`（表の 24 個を種にした） | 0 |
| `--pdata`（A） | 5（今回の 5 個） |
| reloc seed のみ（`Y1`） | 5（今回の 5 個） |
| reloc seed + (ii) 全件（`Y2`） | 0 |

- `--pdata` を使わない種の定義では、5 個を足すだけでは表の項目が揃わない（`.pdata` 開始の 17 個が残る）。`--pdata`（または reloc seed の 7,731 個）を使えば、足りないのはこの 5 個だけ。
## 6. 項目 4: 5 個のうち DllMain から実行時に呼ばれるもの

この節は資料を読んだだけで、実行していない。

### 6.1 資料から言えること

| 5 個 | 実行時に呼ばれたか | 根拠 | 区分 |
|---|---|---|---|
| 0x37d23b4（表 A 組 0 の第 1） | **ATTACH で呼ばれる**（呼ぶ経路で止まった） | 種 428 個の run（`idx0`）が `fle: indirect call to 0x37d23b4 is not a known function entry` で停止（stage23 §3.1、post0083 §6）。停止時のゲストのレジスタ: rax = 0x37d23b4、rcx = 0x3ec8360、rdx = 0x3ec8460（`lift/out-two-post0083/run/idx0-imports.txt` 48 行目）。スタックは `f_37de9b0 ← f_37de9d0 ← f_37d2484 ← f_37cb484 ← f_37ca988 ← f_37caca8`（DllMain） | 実測（呼び出し経路。停止時点で呼ぶ直前まで届いた。本体の実行は未実施） |
| 0x37d23b0（表 A 組 3、14 の第 1。組 3、9、11、15 の第 2） | 第 1 スロットは ATTACH で順方向の走査が呼ぶ読み | §3.3.1。post0083 §3.1 のスタックは、走査が組 10 の第 1（0x37d6c5c）の中まで進んだ状態で停止したことを示す（`f_37d6c5c` が `f_37de9b0` 経由で呼ばれている）。組 3 の第 1 は組 10 より前なので、その走査では呼ばれていたはず、と読める。ただし、仮の値なしの run で `fle_dispatch` が 0x37d23b0 に入った記録は無く、種 428 個の run はその前（組 0）で止まる | 読み |
| 0x37d243c（表 A 組 6 の第 2） | **ATTACH の成功経路では呼ばれない**読み | 第 2 スロットだけ。第 2 スロットを走査する逆方向の関数 0x37d7868 の入口 0x37d24bc は、DETACH 経路の 0x37caaf0 と ATTACH 失敗時の巻き戻しの 0x37cac25 から（0x37cb618 経由で）呼ばれる（§3.3.1） | 読み |
| 0x37d77c8（表 A 組 5 の第 2） | 同上 | 同上 | 読み |
| 0x37da928（表 B の 0x3ebd2a0） | ATTACH で呼ばれる読み | 表 B の走査（0x37d1694。0x37ca9d8 内の 0x37caa4d から、`reason == 1` の経路）。0x37da928 が戻り値 0 で戻れば走査は続く形。post0083 の run（仮の値なし）は、この走査の前の表 A で止まる。stage23 §5.1 の仮の値 5 段の run では DllMain が戻った（`eax = 1`）ので、通過した可能性が高い（推測）。0x37da928 に入った記録は無い | 読み・推測 |

- **5 個の実行回数を数えたログは無い**。`lift/harness/two/gdb_hits.py`（関数ごとの入口カウント）は `HITS_FUNCS` を環境変数で受ける仕組みだが、5 個の関数名（`f_37d23b4` ほか）で走らせた記録は `lift/out-stage2/run/` にも `lift/out-two-post0083/run/` にも無い（5 個の関数名を含むログは `idx0` の停止の記録のみ。`grep` で確認）。
- 5 個を種にした構成（`tbl2`）の run は stage23 §3.2 と post0083 §3 にあるが、5 個の本体の実行は記録されていない（`tbl2` の停止はその後の `MultiByteToWideChar`）。
- DllMain の ATTACH/DETACH の分岐（読み。`logs/q4_dispatch.txt`）: DllMain の入口 0x37caca8 は `reason == 1` のとき 0x37cb22c（セキュリティクッキー初期化）を呼んでから 0x37cab74（本体）へ飛ぶ。0x37cab74 は `reason` に応じて 0x37ca988（CRT の ATTACH/DETACH ハンドラ）を呼び、0x37ca988 の `reason == 1` の経路が 0x37ca9d8 → 0x37cb484 → 0x37d2484（表 A の順方向走査）と、続けて表 B の 2 つの走査（0x37d1694、0x37d1650）を呼ぶ。
- 0x37cab74 は、0x37ca988 の呼び出しの後で `call 0x37c85b0` を呼ぶ（`0x37cabfe`）。0x37c85b0 は `reason == 1`（`cmp edx,1`）のとき `call 0x2001040` を呼ぶ小さな関数（`logs/q4_85b0.txt`）。0x2001040 は stage23 §6 が選択値の変数 0x4541000 を −1 → 0 に書く関数として実測したもの（`f_2001040 ← f_37c85b0 ← f_37caca8`）。つまり、表 A、B の走査（0x37ca988）は選択値の初期化（0x37c85b0）より前に行われる読み（呼び出しの順序は、コードを読んだ結果）。

### 6.2 参考: LoadConfig の CFG ポインタ（実測）

- LoadConfig（RVA 0x244da50、サイズ 0x138、GuardFlags 0x100 = `IMAGE_GUARD_CF_INSTRUMENTED`）の `GuardCFCheckFunctionPointer` のスロット 0x3ebd248 は **0x37cb228**（`ret 0`、(ii)）、`GuardCFDispatchFunctionPointer` のスロット 0x3ebd258 は **0x37de9b0**（`jmp rax`。`.pdata` 開始）。表 A の走査関数が `call [0x3ebd260]`（→ `0x37de9d0: jmp [0x3ebd258]` → `0x37de9b0: jmp rax`）で呼ぶ経路が、まさにこのディスパッチ関数。`GuardCFFunctionTable` は 0（アドレス 0、`GuardCFFunctionCount` 0）。
- **CFG の有効な呼び先の表（GuardCFFunctionTable）が空なので、5 個を含む「間接呼び出しの有効な飛び先」を PE から取ることはできない**（§8 の 3）。
## 7. 項目 5: 生成器の規則を足した場合の影響（実測。実装していない）

### 7.1 規則 R1:「`.reloc` の DIR64 が指す実行範囲内の値のうち、pdata 範囲に入らないものを種にする」

「pdata 範囲に入らない」は、どの RUNTIME_FUNCTION の `begin <= rva < end` にも入らない、の意味（§4.2 の (ii)）。

| | dsp_ippv2_x64.dll（リベース済み） | FLEngine_x64.dll |
|---|---|---|
| (ii) の件数 | **1,544** | **55,586**（DIR64 の実行範囲内の値 80,188、うち `.pdata` 開始 24,592、開始でない 55,596。開始でないもののうち 10 は (i-b)） |
| `--pdata` のみ（基準） | 32,648 関数、`bad_nodes` 0 | 59,297 関数、`bad_nodes` 0（`logs/FLE-A-pdata.txt`。0074 の「変更後」の 59,297 関数と一致。`unsupported_total` は 525 で、0074 の 531 と違う: 0074 より後のコミットによる変更と読める。確認していない） |
| `--pdata` + (ii) 全件 | **33,638 関数**（+990）、命令 6,489,933、**`bad_nodes` 0**、`unsupported_total` 992,445（+17,918） | **115,378 関数**（+56,081）、命令 3,676,911、**`bad_nodes` 113,485**、`unsupported_total` 446,107（`logs/FLE-C-pdata-ii.txt`）、`c_bytes` 631,411,909（基準 215,707,512） |

- 実測として、**規則 R1 を DLL に足すと `bad_nodes` は増えず、FLEngine に足すと 0 → 113,485 になる**。FLEngine では、(ii) の値の多くがコードではない（FLEngine の (ii) 55,586 個のうち、16 バイト境界は 13,761、capstone で先頭 16 命令を読む途中に不正バイトに当たるものが 33,279。`logs/q_extra2.txt`）。FLEngine の (ii) の値を指すスロットの所在は `.text` 125,666 スロット、`.data` 6,341、`.didata` 114（`.text` 内の DIR64 スロットが大多数）。DLL では全部が不正バイト 0（§4.3）。
- **FLEngine の `bad_nodes` の出どころの切り分け**（4 回）。(ii) の種ファイルを 2 通りに分けて測った（`--pdata --seeds`。`logs/FLE-S-*.txt`）:

| 種 | 種の数 | functions | closure_instructions | bad_nodes | unsupported_total |
|---|---:|---:|---:|---:|---:|
| 16 バイト境界のみ | 13,761 | 73,650 | 3,047,631 | 35,504 | 137,207 |
| 16 バイト境界でないもの | 41,825 | 101,431 | 3,585,467 | 97,479 | 381,045 |
| capstone で不正バイト無し（先頭 16 命令 or 最初の `ret`/`jmp`/`int3`/`ud2`/`hlt`） | 22,307 | 81,918 | 3,501,307 | 71,774 | 321,718 |
| capstone で不正バイトあり | 33,279 | 93,183 | 3,171,609 | 66,891 | 216,992 |

  どの分割でも `bad_nodes` が増える。**16 バイト境界でも、不正バイト無しでも増える**ので、単純な絞り込み（境界、先頭の読み）では FLEngine の増加は避けられない。(ii) の値 1 個単位の原因（どの種がどの `bad_nodes` を生むか）は特定していない（依頼の「半分ずつ、最大 6 回」は DLL の (C) で `bad_nodes` が増えた場合の手順で、DLL では不要だった。FLEngine は参考測定で、4 回行った）。
- 規則 R1 の変形の測定（実測）: 「(ii) のうち、値を指すスロットの少なくとも 1 つが非実行セクション（`.data`、`.rdata`、`.didata` など）にあるものだけ」を種にする（C2）:
  - DLL: 1,544 → 1,484 個。functions 33,578、**`bad_nodes` 0**。5 個は全部残る（5 個を指すスロットは `.rdata`）。
  - FLEngine: 55,586 → 6,111 個。functions 65,414、closure_instructions 2,576,458、**`bad_nodes` 16,343**、`unsupported_total` 61,032（`logs/FLE-C2-pdata-ii-slotfilter.txt`）。**FLEngine では増える**。
  - FLEngine の基準（`--pdata` のみ）は `bad_nodes` 0 なので、FLEngine の生成 C は、R1 のどの変形でも**変わる**。FLEngine の `--pdata` の生成 C（基準は `c_bytes` 215,707,512。stage23 §8 の SHA-256 `43985d62…` の C と同じ大きさ。今回は C を書いていないので SHA-256 は取っていない）は、R1 を足すと**変わる**。現行の FLEngine の生成 C を変えない場合は、R1 を DLL だけに適用する条件が要る。
- R1 の適用条件の候補（事実だけ）: 実行範囲外の `.text` 内の DIR64 スロット（`.text` に表が埋まる形）は FLEngine で多い（125,666 スロット）。DLL では 456 スロット（うち (ii) の値を持つのは 60 個の値）。どの条件が正しいかは決めない（§8 の 2）。

### 7.2 規則 R2:「CRT 初期化表の項目を種にする」に必要な情報

必要なのは、表の位置（開始と終了の VA）と、その形（項目の幅、組の形）。

| 取り方 | この DLL でできるか | 事実 |
|---|---|---|
| (a) 表 B（`.CRT$XIA`〜`XIZ`、`XCA`〜`XCZ`、`XPA`〜`XPZ`、`XTA`〜`XTZ`）を **デバッグディレクトリの型 13**（先頭 4 バイトが `GCTL`）から取る | **できる**（この DLL では。仕様は未確認） | デバッグディレクトリ（RVA 0x244d9d8、サイズ 0x70）にエントリが 4 つ: 型 2（CodeView、サイズ 0x72）、型 12（サイズ 0x14）、型 13（サイズ 0x320。先頭 `GCTL`）、型 14（サイズ 0）。型 13 の中身は 800 バイトで、8 バイトずつ（RVA、サイズ）+ NUL 終端の名前（4 バイト境界）の 42 エントリ。**800 バイトを過不足なく読み切れた**（実測）。エントリのうち `.CRT$XCA`（RVA 0x1ebd270、サイズ 8）、`.CRT$XCZ`（0x1ebd278、8）、`.CRT$XIA`（0x1ebd280、8）、`.CRT$XIC`（0x1ebd288、0x20）、`.CRT$XIZ`（0x1ebd2a8、8）、`.CRT$XPA`（0x1ebd2b0、8）、`.CRT$XPX`（0x1ebd2b8、0x10）、`.CRT$XPXA`（0x1ebd2c8、8）、`.CRT$XPZ`（0x1ebd2d0、8）、`.CRT$XTA`（0x1ebd2d8、8）、`.CRT$XTZ`（0x1ebd2e0、0x20）。これらの範囲は、0x37ca9d8 が走査関数に渡す 0x3ebd270、0x3ebd278、0x3ebd280、0x3ebd2a8（リベース後の VA）と一致（`logs/q5_meta.txt`、`q1_walkers.txt`）。**Learn の PE Format のページ（`https://learn.microsoft.com/en-us/windows/win32/debug/pe-format`。ページのメタデータ `updated_at` は 2026-09-10。本作業で取得）のデバッグ型の表には、型 12、13、14 の行が無い**（載っているのは 0〜11、16、17、19、20。取得したページ全体（2,036 行）を `grep` して、`POGO`、`GCTL`、`profile guided` の語も無いことを確認した。取得物は opencode のツール出力置き場にあり、`lift/out-dll-seeds/` には保存していない）。つまり、型 13 の構造は仕様では確認できない（この DLL のバイト列から構造を逆算した。**推測**）。型 13 が POGO（Profile Guided Optimization）の情報であることも、このページからは確認できていない |
| (b) 表 A（16 バイトの組の表）を同じ方法で取る | **できない** | 表 A（0x3ec8360〜0x3ec8460）は `.rdata` の中にあるが、型 13 に名前つきの範囲が無い（`.rdata` は 1 エントリ（RVA 0x1ebd300、サイズ 0x590900）の中）。表 A に対応する名前（`.CRT$XCU` など）は型 13 の 42 エントリに無い（表 B 側の `XIC` の 0x20 バイトは 4 スロット = 表 B の項目 0x3ebd288〜0x3ebd2a0 と一致） |
| (c) TLS ディレクトリのコールバック | **使えない** | TLS ディレクトリが無い（RVA 0、サイズ 0） |
| (d) LoadConfig の `GuardCFFunctionTable`（CFG の有効な飛び先の表） | **使えない** | `GuardCFFunctionTable` = 0、`GuardCFFunctionCount` = 0（GuardFlags = 0x100 = CF_INSTRUMENTED のみ。`IMAGE_GUARD_CF_FUNCTION_TABLE_PRESENT`（0x400）が立っていない） |
| (e) COFF シンボル | **使えない** | `PointerToSymbolTable` = 0、`NumberOfSymbols` = 0（DLL と FLEngine の両方） |
| (f) セクション名 | **使えない** | セクション名に `.CRT` を持つものは無い（`.rdata` に統合されている。FLEngine も同じ） |
| (g) コードから取る（`lea rcx/rdx` で渡される表の範囲） | **できるが、パターンの認識が要る** | `0x37d2484`（`lea rdx,[…]; lea rcx,[…]; jmp 走査関数`）と `0x37ca9d8` 内の 2 組の `lea` の形。走査関数（`cmp rcx,rdx` と 8 または 16 バイト刻みのループ + `call [ガード付きポインタ]`）を認識する必要がある。決定 0079 の `recognize_stub` と同種の形の認識。この DLL の 2 つの表を、人が読んで見つけた（stage23 §3.1 の読み） |
| (h) `DllMain` の `reason == 1` の経路を追って、`call` の引数から取る | (g) と同じ | — |

- FLEngine_x64.dll には、デバッグディレクトリも LoadConfig も TLS も無い（RVA 0、サイズ 0。`logs/q5_more.txt`）。**FLEngine では (a) は使えない**。
- 表 A の形（16 バイトの組、第 1 が初期化、第 2 が終了）の裏づけは、走査関数の読み（0x37d77d4 は 16 バイト刻み、0x37d7868 は `[rbx-8]` と 16 バイト刻み）だけで、PE のメタデータには無い（§3.3.1。読み）。
- **R2 が足す種の数**: 表 B の 4 項目（`.CRT$XIC` の 0x20 バイトにある 0x37d2aa4、0x37d4f70、0x37d9e4c、0x37da928）と、表 A の 25 スロット（20 個の distinct）。5 個はそのうち 5。(a) だけで足りるのは、5 個のうち 0x37da928 の 1 個だけ（他の 4 個は表 A）。

### 7.3 R1 と R2 の比較（事実だけ）

| | R1（pdata 範囲外の reloc 値） | R2（CRT 初期化表） |
|---|---|---|
| DLL で 5 個を足せるか | 足せる（(ii) 1,544 個の一部） | (a) では 1 個（0x37da928）。(g) では 5 個 |
| 追加される種の数（DLL） | 986 個（閉包の関数 +990） | 5 個を含む表の項目 24 個（そのうち閉包 A に無い 5 個。+5 関数） |
| 追加の閉包（DLL、`bad_nodes`） | +990 関数、`bad_nodes` 0 | +5 関数、`bad_nodes` 0 |
| FLEngine への影響 | 無条件に適用すると `bad_nodes` 0 → 113,485（§7.1） | 表のメタデータが無いので (a) は何も足さない（FLEngine の生成 C は変わらない）。(g) は FLEngine に当てはまる形が無ければ何も足さない（未確認） |
| 仕様の裏づけ | リーフ関数の `.pdata` 省略は Learn が記述（§3.2）。reloc 値が関数入口である、とは書いていない | デバッグ型 13 の構造は Learn に記述なし。CRT の `.CRT$X*` グループは Learn「CRT Initialization」に記述あり（`XCA`/`XCZ` が `__xc_a`/`__xc_z`、リンカがグループ名の昇順に並べる）。`XIA`/`XIZ`/`XPA`/`XTA` などは、このページには出てこない |
## 8. 決定が要る点（決めない）

1. **縮小 DLL の種の定義**（stage23 §9 の 1、post0083 §9 の 4。未決のまま）。選択肢と、本調査の事実:
   - (a) 表のコードポインタを手で種に渡す（`tbl2`。`--pdata` なしの縮小 DLL で表の 24 項目は全部届く。§5.4）。表の位置は人が読んで特定する（stage23 §3.1）。
   - (b) 生成器に「CRT 初期化表の項目を種にする」規則（§7.2 の R2）。表 B は DLL のデバッグ型 13 から取れるが、**表 A は PE のメタデータから取れない**。コードから取る方法（§7.2 の (g)）は形の認識が要る。FLEngine には使えるメタデータが無い。
   - (c) 全部入り。`--pdata` だけ（DLL で 32,648 関数）では 5 個が入らない（§5.1 の (A)）。R1 を足すと 33,638 関数、`bad_nodes` 0（§5.1 の (C)）。wasm の locals 上限は決定 0080 D7 と stage23 §7 のとおり別の問題で、本調査では測っていない。**R1 を FLEngine に無条件に適用すると `bad_nodes` が 113,485 になる**。
   - 5 個だけを足す構成は、`--pdata` を使うなら (B) = 32,653 関数（+5、`bad_nodes` 0）。`--pdata` を使わない `idx0` では、`.pdata` 開始の 17 個の表項目が残る（§5.4）。
2. **R1 の適用条件**。規則を「DLL だけ」「`.pdata` が何らかの条件を満たすとき」「`--reloc-seeds-nonpdata` のような明示の指定」のどれにするか。FLEngine の (ii)（55,586 個）は、境界でも命令の読みでも絞れなかった（§7.1。単純な絞り込みでは `bad_nodes` 0 に戻らなかった）。決定 0074 の「一致するものだけ」の方針（実行範囲内で `.pdata` 開始でない値は種にしない）を変える形になる。
3. **5 個が関数入口であることの確認**。5 個は命令列としてはリーフ関数に見え、表 A、B の項目で、`fle_dispatch` が 1 つ（0x37d23b4）で止まったのは実測だが、他の 4 個の実行は未確認（§6）。決定 0074 の「参考（採っていない）」の段落と決定 0079 §4 が、`.pdata` に無い値を「関数入口として扱う」確認が無い点を未確認としたのと同じ性質。
4. **`.pdata` に無い (ii) をリーフ関数として扱うか**。(ii) の 1,544 個のうち、`stub-16`（434）は `push`、`call`、`pop` を含む（Learn のリーフの定義に合わない。§4.4）。
5. **決定 0079 の「905」の数え直し**（§4.5）。pefile の `.pdata` 読みが 29,274 で止まったことで、0079 の「`.pdata` 開始でない 905」は、本調査の数え方では 548。**0079 の文書の訂正を行うか**（本調査は 0079 を編集していない）。
6. **デバッグ型 13（`GCTL`）を使うか**。仕様の記述が確認できない（§7.2）。

## 9. 未確認

- 5 個のうち 0x37d23b4 以外の実行（回数、順序）。`gdb_hits.py` で `HITS_FUNCS` に 5 個を指定する測定は行っていない（依頼は「実行しない」）。
- (ii) の 1,544 個のうち、どれが「本当の関数の入口」か。命令列として読めることと、`.reloc` の値であることだけ。
- 5 個を含め、(ii) が `.pdata` に無い理由（リーフ関数として省略された、手書きの thunk を `.pdata` に入れなかった、など）。
- `stub-16` の飛び先 4 つ（0x37e08c0 など）が呼ぶ関数（0x20034d0、0x2001040、0x335fd10、0x35b45a0）の中身。0x2001040 以外は読んでいない（0x2001040 は stage23 §6 の測定がある）。
- (C) の `unsupported_total` +17,918 の内訳の個別の確認（種類別の増分は `logs/q3_unsup_diff.txt` にある）。
- 決定 0079 の `unsupported_total` 974,582 と今回の 974,527 の 55 差、`c_lines` の 99 差の出どころ（§5.1）。
- pefile が `.pdata` を 29,274 で止めた理由（§4.5）。
- FLEngine の `bad_nodes` 113,485 のうち、(ii) のどの値がどの `bad_nodes` を生むか（§7.1。切り分けは 4 回の粗い分割のみ）。
- **生成 C を clang でコンパイルしていない、実行していない**（統計のみ。`/dev/null` へ出力）。
- 実 Windows での挙動（ホストは Linux）。
- 表 A のレイアウト（第 1 が初期化、第 2 が終了の 16 バイトの組）は、走査関数の読みだけで、MSVC のどの規則に当たるかは確認していない。

## 10. 検証

- HEAD は `9ae09a03d57bc6ad9f60c54bd0ee89333c380b6f`（作業の最初と最後で同じ）。DLL の SHA-256 は §0 のとおり（`sha256sum`）。
- fe-pe、fe-decode、fe-lift のソース、lift/runtime、docs/decisions、既存のテスト、既存の docs は編集していない。`git status --short` と `git diff --stat` は完了報告に載せる。作業前から変更されていた `docs/README.md`、`lift/harness/two/patches_stage1.py`（他のセッションの変更。`git diff` で確認。この調査では触っていない）と、未追跡の `lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`、`docs/investigations/mbtwc-high-bytes.md`（並行の調査）は、触っていない。
- `rm` は使っていない。`__pycache__`、`*.pyc` は `lift/out-dll-seeds/` に無い（`find` で確認）。`/tmp` は使っていない（TMPDIR は `lift/out-dll-seeds/tmp`）。
- `cargo test` は実行していない（ソースを編集していないため）。
- 2 GB 級の単一ファイルは無い。`lift/out-dll-seeds/` は約 39 MB。最大のファイルは `target/` の成果物。生成 C は保存していない。

## 11. 作ったファイル

- 本文書 `docs/investigations/dll-nonpdata-reloc-seeds.md`（新規）。
- `lift/out-dll-seeds/`（`.gitignore` の対象）:
  - `target/`（`fe-pe`、`fe-decode`、`fe-lift` の release ビルド）。
  - `py/`（解析スクリプト。標準ライブラリと衝突する名前は無い）: `seedscan_common.py`（共通。PE、`.pdata`、`.reloc` の読み）、`q1_*.py`（5 個）、`q2_*.py`（分類）、`q3_*.py`（閉包の比較）、`q4_*.py`（呼び出し経路）、`q5_*.py`（メタデータと FLEngine）、`mkseedsets.py`、`q_*.py`。
  - `data/`: `reloc-seeds.txt`（7,731）、`reloc-not-function.txt`（2,750。原本は `.orig-base.txt`）、`nonpdata-classified.tsv`（2,750 行の分類）、`cat-i-a.txt`、`cat-i-b.txt`、`cat-i-c.txt`、`cat-ii.txt`、種ファイル `seeds-*.txt`、関数の集合 `funcs-*.txt`、`fle-ii*.txt`、`pogo-entries.tsv`（デバッグ型 13 の 42 エントリ）。
  - `logs/`: 各 lift の統計（`A-pdata.txt` ほか）と各スクリプトの出力（`q*.txt`）。
  - `tmp/`（TMPDIR。空）。

