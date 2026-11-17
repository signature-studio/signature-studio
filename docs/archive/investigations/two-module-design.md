# dsp_ippv2_x64.dll を 2 つ目のモジュールとして載せる設計（選択肢の比較）

実行日: 2026-10-23。HEAD c99a0f6（ワークツリーには他の subagent の未コミット変更があるため、測定は `git archive HEAD` を `lift/out-design/stage` に展開したコピーで行った）。**調査と設計のみ**。`lift/` の既存ファイル、runtime、生成器、テストは変更していない。決定 0073〜0077、`dsp-ippv2-lift-feasibility.md`（以下「実現性調査」）の続き。**この文書は何も決めない**。推奨は理由つきで書くが、決めるのは PM。作業物は `lift/out-design/`（gitignore 対象の `lift/out*/`）。

## 0. 凡例

- **実測** = 今回コマンドを実行して得た値（出力は `lift/out-design/` にある）。
- **読み** = ソースや逆アセンブルを読んだ結果。実行して確かめていない。
- **仮説** = 実測や読みから言えそうだが、確認していない。
- **未確認** = 確認できていない。
- 外部仕様は Microsoft Learn（DllMain、PE Format。2026-10-23 取得）と WebAssembly JS API 仕様（Implementation-defined Limits。同日取得）だけを引いた。

## 1. 要約

| 項目 | 結果 | 区分 |
|---|---|---|
| ファイル段階のリベース（生成前に `.reloc` の DIR64 を適用） | 基底 0x1cc0000、0x2000000、0x8000000 で fe-lift の統計（関数 32,162、命令 6,504,638、未対応 973,919、bad_nodes 0）が、リベースなしと同一。生成 C 中の `0x18xxxxxxx` 形の即値は 1,135,257 → 0 | 実測 |
| ロード時リベース | 生成 C は VA を即値で埋めるので、実行時に基底を変えるには生成器の変更が要る。成立させる最小の形は「生成時の基底と同じ値に置く」で、これはファイル段階と同値 | 読みと実測 |
| 2 つの生成 C の重複定義 | 強い定義の重複は **7 個**: `R`、`X`、`FL`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_table`、`fle_resp_count`。`f_<VA>` の衝突は 0。`MM`、`YH`、`fle_teb`、`fle_mxcsr`、`fle_x87_mmx_active`、`fle_teb_set_stack` は weak で 1 つに統合される | 実測（`ld -r` で確認） |
| 2 モジュールのリンク | 状態共有（R/FL/X を 1 つ、dispatch 表を 1 つにマージ）の試作をネイティブと wasm で作り、リンクと実行ができた | 実測（縮小 DLL） |
| **全部入りの DLL は wasm が実行できない** | DLL 全部（32,162 関数）を `-O0` で wasm にすると 533,108,454 B になりリンクは通るが、node の wasm コンパイルが `local count too large` で失敗する。13 関数が仕様の上限（1 関数 50,000 locals）を超え、最大は 389,948 | 実測 |
| 縮小 DLL（`table-seeds-idx0` の 428 種から。1,891 関数） | wasm 156,393,882 B。最大 locals 41,126（上限の 82%）。node で FLEngine と共に動き、FLEngine の停止点は単独のときと同じ | 実測 |
| DllMain だけを実行 | リベースした縮小 DLL を DllMain から実行すると、native と wasm の両方が **`LoadLibraryExW("api-ms-win-core-synch-l1-2-0.dll")`**（CRT の遅延 API 解決）で停止する | 実測 |
| 一番大きい未決 | (1) 基底の値、(2) DLL が呼ぶ `api-ms-win-*` に何と答えるか、(3) 全部入りを wasm にする方法（locals 上限）、(4) モジュールハンドルの値の方針 | §6 |

## 2. アドレス空間の実マップ

### 2.1 ゲストのポインタは何か（恒等写像の意味）

- ゲストの命令は `fle_ld(a, n)` と `fle_st` でメモリを読み書きし、アドレス `a` は 64 ビットの値。`fle_ld` はそれをそのままホストのポインタにする（`fle_runtime.h`: `memcpy(&v, (const void *)fle_addr(a, n, 0), n)`）。**ゲストの値 = ホストのアドレス**（読み）。
- 値の出どころは 2 種類ある。(a) **イメージ内のアドレス**はファイルの VA（`image.c` がイメージを ImageBase のホストアドレスに置くので、VA = ホストアドレス）。(b) **実行時に得るポインタ**（`HeapAlloc` は `malloc` の戻りを `R[0]` に入れる: `fle_sys.c:778`、スタックはハーネスが確保した 16 MiB、`VirtualAlloc` は native が `mmap`、wasm がアリーナ）は**ホストのアドレスそのもの**で、VA ではない。native では 47 ビットの値（実測: スタック 0x7772…、ヒープ 0x7396…）、wasm では 32 ビット以下。
- wasm32 では `fle_addr` が `a >> 32` を検査し、非ゼロなら `[fle_addr_hi32]` で `abort()`（決定 0045 d、0049 a）。native の既定は検査なしの `(uintptr_t)(a)`。したがって**生成 C に 4 GiB 以上の VA の即値があると wasm で止まる**。ImageBase 0x180000000 の DLL は、リベースなしだと 1,135,257 個の即値（`0x18xxxxxxx`）が該当する（実測。実現性調査 §2.3 の 1,173,132 は数え方が違う。今回は HEAD のコピーと `UINT64_C(0x18…)` 形だけを数えた）。
- `FLE_CHECKED_MEM`（既定オフ）は単一のイメージ範囲 `[fle_mem_img_lo, +fle_mem_img_size)` と `__global_base` 以上だけを許す（`fle_host.c:13-14`、`fle_runtime.h:50`）。2 つ目のイメージを許すには範囲が足りない。

### 2.2 実マップ（出典つき）

「N」= native、「W」= wasm。値は 16 進。

| 領域 | アドレス / サイズ | 出典 | 区分 |
|---|---|---|---|
| FLEngine のイメージ | 0x400000 〜 0x1cc0000（SizeOfImage 0x18c0000） | N: `image.c:101` の `mmap` を ImageBase に固定。W: `image.c:98` で `ibase` のリニアメモリに `memset`（`ibase + size > __global_base` なら失敗） | 実測（実行ログ `image: base=0x400000 size=0x18c0000`） |
| 未使用の隙間 | 0x1cc0000 〜 0x10000000（0xe340000 = 227 MiB） | 上の 2 行から | 導出 |
| W: `GLOBAL_BASE` | 0x10000000 | `build.sh:35`。リンク後のメモリ配置を直接読んだ（`wasm_layout.py`）: データセグメントは 0x10000000 から | 実測 |
| W: データ + .bss | 0x10000000 〜 `__data_end` 0x1044b745（単独ビルド）。続けて C スタック 8 MiB（`-sSTACK_SIZE=8MB`）。`__heap_base` 0x10c4b750 | 実測（`__data_end`、`__heap_base` を export して読んだ。差が 8 MiB ちょうど） | 実測 |
| W: `R`、`X`、`FL`、`fle_teb`（0x2000）、`fle_tls_values`（1088 個 = 0x2200）、`MM`、`YH` | 上のデータ領域の中。2 モジュール（縮小）のリンクマップでは `fle_teb` 0x1035bc20、`R` 0x1035dc20、`X` 0x1035dca0、`fle_tls_values` 0x1035ded0。**リンクごとに変わる** | `-Wl,-Map` の出力 | 実測 |
| TEB | **固定アドレスなし**。ホスト側の静的バッファ `fle_teb[0x2000]`（weak、0 初期化）。依頼文の「TEB 0x1D00000 付近」は、`0x1D00000`、`FLE_TEB` を `lift/` と `docs/` で検索して決定 0032 の記述（検索 0 件）以外に定義が見つからなかった | `fle_vex.h:94-105`、決定 0032 | 実測（検索） |
| ハーネスのスタック | 16 MiB（`HARNESS_STACK_BYTES`）。N: `mmap`。W: `aligned_alloc(4096, 16 MiB)`。**固定アドレスではない**。W の実行ログの `rsp` は単独 0x11c51fc0、2 モジュール 0x11c67fc0 | `harness/main.c:56`、`harness/wasm/main.c:50` | 実測 |
| 仮想メモリのアリーナ | 512 MiB（`FLE_VMEM_ARENA_BYTES`）。W だけ。最初の使用時に `aligned_alloc(65536)` で 1 つ取る。**固定アドレスではない**（ホストヒープのどこか）。ハーネスの他の割り当てより後なので、スタックの上になる（試作プログラムで 0x11810000〜0x31810000。FLEngine そのものでの値は未測定） | `fle_sys.c:314-338`、決定 0045 c | 実測（試作）+ 読み |
| ファイルマッピング | W: アリーナの一部（`fle_vmap.c` の `FLE_VMEM_ARENA` 分岐）。N: `memfd` + `mmap`（`fle_vmap.c:520,528`） | 決定 0045 b | 読み |
| ホストヒープ（`HeapAlloc`、`LocalAlloc`、`GlobalAlloc`） | `malloc`。W では `__heap_base` より上、N では mmap 領域 | `fle_sys.c:776` | 読み |
| N: アドレス空間の上限 | 4 GiB（`RLIMIT_AS`、`HARNESS_MAX_AS_BYTES`） | `harness/boot.h:13`、`harness/main.c:27` | 読み |
| W: メモリの上限 | 初期 640 MiB（10,240 ページ）、最大 2 GiB（32,768 ページ） | リンク後のメモリセクションを直接読んだ | 実測 |
| トークン（実メモリではない）: ウィンドウ 0x7F990000、スクリーン DC 0x7FA90000、GDI 0x7FBA0000、標準カーソル 0x7FCA0000、DLL のカーソルとアイコン 0x7FDA0000、モニタ 0x7FEA0000（各 1 MiB） | `fle_win.h`、`fle_gdc.h`、`fle_gdi.h`、`fle_cursor.h`、`fle_loadimage.h`、`fle_monitor.h` | 読み |
| トークン（4 GiB 以上） | プロセスヒープ 0x7FF700000000、モジュールハンドル 0x7FF800000000 + 0x10000 × N（N ≤ 26）、応答表の擬似 VA 0x7FF900000000 + 0x10 × (k+1)（k は 227 行。最大 0x7ff900000e30）、VFS ファイル 0x7FFA00000000、検索 0x7FFA10000000、レジストリキー 0x7FFA20000000、グローバル 0x7FFA50000000、ボリューム 0x7FFA60000000、vmap 0x7FFA70000000、イベント 0x7FFA80000000（各 0x10000000） | `fle_sys.h:37`、`fle_modules.h:14-15`、`imports_table.rs:282-283`、`fle_vfs.h`、`fle_glob.h`、`fle_vol.h`、`fle_vmap.h`、`fle_event.h` | 読み。生成 C の `fle_resp_table` で擬似 VA の最大値を確認 |
| 番兵 | コールバックの戻りアドレス 0xCB000000（`FLE_CB_RETURN_ADDRESS`）、ハーネスの戻り 0xdead0000 | `fle_callback.h:30`、`boot.h:15` | 読み |
| 仮想 FS | **ゲストのメモリ領域を持たない**（ハンドルはトークン。内容はホスト側の表） | 決定 0018 | 読み |
| `IMP_VA_BASE` | **存在しない**（決定 0015 §4 のとおり。実際の定数は `RESP_VA_BASE` = 0x7FF9_0000_0000） | grep | 実測 |

補足:
- 0x7F990000〜0x7FF9FFFF（トークン）は 4 GiB 未満で、**wasm の最大メモリ 2 GiB（0x80000000）の内側の上端 26 MiB に当たる**（導出）。現在のメモリ使用量は数百 MiB なので重ならないが、最大メモリを 2 GiB 以上に上げる場合は、トークンがリニアメモリの実アドレスに重なる（`malloc` の戻りがトークンと同じ値になりうる）。**将来の最大メモリの引き上げに対する注意**。
- native では `HeapAlloc` などの戻り値は 47 ビットのホストアドレスで、ゲストの `uint64_t` にそのまま入る。DLL が 32 ビットに切り詰めるコードを持つかは未確認。
## 3. DLL の置き場所

DLL: SizeOfImage 0x261f000（39,972,864 B ≒ 38.1 MiB）、ImageBase 0x180000000、`.reloc` は DIR64 のみ 15,602 件（ABSOLUTE 22 件）、Characteristics 0x2022（`RELOCS_STRIPPED` なし）、DllCharacteristics 0x0160（DYNAMIC_BASE あり。実現性調査 §2.3、external-modules-usage.md §1.1）。PE Format（Learn）は ImageBase を「64 K の倍数」と定めるので、基底は 0x10000 の倍数でなければならない。

### 3.1 DIR64 スロットの中身（実測）

- 15,602 スロットの値はすべて `[ImageBase, ImageBase + SizeOfImage)` の中を指す（範囲外 0）。指す先のセクション: `.text` 14,077、`IPPCODE` 784、`.rdata` 707、`.data` 34。
- `.data`、`.rdata`、`IPPDATA` の 8 バイト整列の値のうち、イメージ範囲に入るのに `.reloc` に載っていないものが **2 つ**（`.rdata` 0x1ec7070 と 0x1ec7110。値は 0x180000029）。アドレスなのか定数なのかは確認していない（仮説: 定数。Windows のローダも `.reloc` に無い値は変えないので、ファイル段階のリベースも同じ結果になる）。
- したがってファイル段階のリベースは「Windows のローダが基底を変えるときの処理」（DIR64 に差分を足す）と同じ操作で、`ipp_rebase.py`（30 行）が全 15,602 件を適用した（ファイルに裏付けのない位置 0）。

### 3.2 リベース先の候補

「回避すべき範囲」は §2.2 の実マップから: FLEngine 0x400000〜0x1cc0000、`GLOBAL_BASE` 0x10000000 以上（wasm のデータ、スタック、ヒープ、アリーナ）、トークン 0x7F990000 以上、ゼロページ側（0x10000 未満。`FLE_MIN_APP_ADDR`）。

| 案 | 基底 | 終端 | 備考 |
|---|---|---|---|
| **P1** | 0x2000000 | 0x461f000 | FLEngine の終端（0x1cc0000）の直後に 3.25 MiB の隙間を置いた丸い値。**fe-lift 統計は同一（実測）。native で FLEngine を先に置いたあと固定 `mmap` が成功（実測）。wasm でリンクして DllMain を実行（実測: `image: base=0x2000000 size=0x261f000 entry=0x37caca8 imports=72 exports=446`）** |
| **P2** | 0x8000000（128 MiB） | 0xa61f000 | FLEngine から 99 MiB 離す。`GLOBAL_BASE` までの残りは約 90 MiB。fe-lift 統計は同一（実測）、native の固定 `mmap` は成功（実測）。**wasm では今回は実行していない**（P1 だけ実行した） |
| **P3** | 0x10000000（`GLOBAL_BASE` を 0x20000000 などへ上げる） | 0x1261f000 | FLEngine と DLL に余裕（約 227 MiB と約 38 MiB の隙間）を取れる。**今回は実行していない（仮説）**。wasm の初期メモリ 640 MiB に対し、`GLOBAL_BASE` を上げた分だけデータ、スタック、アリーナが後ろへ動くので、`INITMEM` を上げるか成長に任せるかの再測定が要る。native の固定 `mmap` は 0x10000000 でも成功した（実測。P3 の wasm 側とは別の確認） |

補足（実測）:
- 0x1cc0000（FLEngine の終端ぴったり）も測った: fe-lift 統計は P1 と同じ（`cmp` で一致を確認）。native の固定 `mmap`（FLEngine を先に置いた状態）は 7 つの候補（0x1cc0000、0x1d00000、0x2000000、0x8000000、0xa000000、0x10000000、0x180000000）すべて成功。0x10000000 に置くと終端 0x1261f000 は `GLOBAL_BASE`（0x10000000）を越えるので、wasm では P3 の前提（`GLOBAL_BASE` を上げる）が要る。`GLOBAL_BASE` 未満に収まる最大の基底は 0x10000000 − 0x261f000 = 0xd9e1000 以下で 64 K 整列なら 0xd9e0000（導出。測定していない）。
- 最初の試作（`native_probe.c`）では 0x1d00000 と 0x2000000 が失敗した。**これは試作が 0x400000 に 0x261f000 バイトを先に取って解放しなかったため**で、ホストの制約ではない（`native_probe2.c` で FLEngine のサイズ 0x18c0000 だけを先に取る形に直すと全候補が成功）。
- native: `vm.mmap_min_addr` は 65536。PIE の実行ファイルは 0x55…、`malloc` と匿名 `mmap(NULL)` は 0x7…（47 ビットの高位）に置かれ、低位の候補とは重ならない（実測、3 回の実行）。`harness/build.sh` は `-fPIE`。非 PIE でリンクすると実行ファイルが 0x400000 付近に来る（既存の注意で、FLEngine の置き場所と同じ問題。今回は未検証）。

### 3.3 runtime が返すアドレスとの衝突

- **wasm**: `VirtualAlloc` とファイルマッピングはアリーナ（`__heap_base` より上。試作で 0x11810000〜）の中だけで、固定アドレスの要求もアリーナ外なら失敗する（決定 0045 c）。ゲストの要求した固定アドレスが DLL のイメージに当たることはない。`HeapAlloc` は `malloc`（`__heap_base` より上）。いずれも `GLOBAL_BASE` 未満の DLL と重ならない（P1、P2）。P3 は DLL が `GLOBAL_BASE` の下に収まらなくなるため、`GLOBAL_BASE` を上げる前提。
- **native**: `VirtualAlloc(want)` は `MAP_FIXED_NOREPLACE`（`fle_sys.c:460`）なので、DLL のイメージ範囲を要求すると「使用中」で失敗する（意味として正しい）。`want` なしは `mmap(NULL)` で高位に来る。
- `VirtualQuery`、`VirtualProtect` は runtime が追跡していないメモリ（イメージ、ホストのヒープとスタック）で停止する（`fle_vmap.c:192`、`fle_vmap.c:203` のコメント）。DLL の import 72 名に `VirtualQuery` と `VirtualProtect` は無い（実現性調査 §6 の一覧）ので DLL 自身は影響しないが、FLEngine が DLL の領域を問い合わせるかは**未確認**。
- 上位 32 ビット検査: 候補はすべて 4 GiB 未満。リベース後の生成 C は 4 GiB 以上の即値を DLL の VA としては持たない（`0x18…` 形 0 個。残る 85,608 個は 64 ビット定数で、リベースの有無で変わらない）。

### 3.4 ファイル段階か、ロード時か

| 観点 | (A) ファイル段階（生成前にリベース） | (B) ロード時（実行時に `.reloc` を適用） |
|---|---|---|
| 生成 C | VA は即値。リベース後は即値の桁が短くなり 565,516,290 → 536,004,496 B（−5.2%）。`0x18…` 形の即値 1,135,257 → 0 | 元の VA（0x18…）のまま生成すると、生成 C の即値 1,135,257 個（全即値 3,254,589 個の 35%）が wasm32 で `[fle_addr_hi32]` になる。実行時に `.reloc` を適用してもコードの中の即値は変わらない |
| dispatch 表 | VA は即値。表に載る VA と、リベース後のメモリ内ポインタ（`.reloc` の差分が足された値）が一致する | 表の VA は元の値（0x18…）、メモリ内のポインタは新しい値になり**一致しない**。実行時に表の VA にも差分を足す仕組みが要る |
| 生成器の変更 | 不要（fe-pe、fe-decode、fe-lift の `src/` に基底のハードコードなし。実現性調査 §2.3 と同じ結論を今回の測定でも確認） | 大きい（アドレスを「基底 + オフセット」の式にする、表を実行時に補正する、など）。**試作していない（読み）** |
| 検証のしやすさ | 入出力が決定的。リベース後のファイルは「元のバイト列 + 差分（15,602 か所）」で検証できる | 基底ごとに実行時の状態が変わる |
| 基底を変えるとき | 生成 C を作り直す（fe-lift 約 5.6 秒 + コンパイル）。native と wasm は同じ基底で揃える | 再生成は不要（利点） |
| 元ファイルとの関係 | リベース後のファイルは別のバイト列（SHA が違う）。出力は元の SHA と基底だけで決まる | 元ファイルをそのまま使う |
| Windows の動作との対応 | ローダが基底を変えるときの処理と同じ | 同じ処理を実行時に行う |

読み: (B) が (A) に勝つのは「同じ生成 C を異なる基底で使いたい」場合だけで、そのための生成器の変更は大きい。(A) は現行の 3 つの crate をそのまま通る（実測）。

**推奨（PM が決める）**: (A) ファイル段階。基底は P1 か P2（wasm での検証が済んでいるのは P1）。理由: 生成器を変えずに済み、native と wasm で同じ C が使え、`image.c` が ImageBase をヘッダから読むのでリベース後のファイルをそのまま渡せる（改造不要。ただし 2 イメージ対応は要る: §4）。

## 4. 生成物の分け方とリンク

### 4.1 現行の emit 出力の構成（実測。HEAD のコピーで `fe-lift lift ... --pdata` を実行）

| 項目 | FLEngine | DLL（`--pdata`、リベース 0x2000000） |
|---|---|---|
| 生成 C | 215,700,923 B、2,885,313 行（SHA-256 67827424…15584d。決定 0076、0077 の基準と同一） | 536,004,496 B、6,698,311 行 |
| 関数 / 命令（emit） | 59,297 / 2,528,830 | 32,162 / 6,504,638 |
| 未対応 | 531 | 973,919 |
| `fle_dispatch_table` の行数 | 59,524（関数 59,297 + 擬似 VA 227） | 32,389（関数 32,162 + 擬似 VA 227） |
| `fle_resp_table` | 227 行（`fle_resp_count = 227`） | 同じ 227 行（**2 つの表は内容が同一**。マージ用の試作が一致を表明で確認） |
| 間接呼び出し | `call_ind:dispatch` 40,746 | `call_ind:dispatch` 989、`jmp_ind:reg_dispatch` 365 |
| 所要時間 | 2.5 秒 | 5.6 秒 |

先頭部分（どちらも同じ 8 行）:

```
/* generated by fe-lift. Do not edit. */
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"

uint64_t R[16];
fle_eflags FL = { FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF, FLE_UNDEF };
fle_xmm X[16];
```

続いて全関数のプロトタイプ（`void f_<VA>(void);`）、`#include "fle_modules.h"` と `fle_imp_*` のプロトタイプ、`fle_dispatch_table[]`、`fle_dispatch_count`、`fle_resp_table[]`、`fle_resp_count`、そして関数本体の順（`fe-lift/src/lib.rs:770`、`imports_table.rs:295-312`）。`split_c.py` は part-00 だけがこれらを定義し、part-01 以降は `R`、`FL`、`X` を `extern` にして表を持たない（`split_c.py:14-16`）。

### 4.2 重複定義の列挙（実測。`ld -r` で FLEngine part-00 と DLL の小さい生成物をつなげた）

強い定義が重複して `multiple definition` になるもの（7 個）:

| シンボル | 型 | 備考 |
|---|---|---|
| `R` | `uint64_t[16]` | ゲストのレジスタ |
| `X` | `fle_xmm[16]` | XMM レジスタ（下位 128 ビット） |
| `FL` | `fle_eflags` | フラグ（遅延評価の状態） |
| `fle_dispatch_table` | `fle_fn_entry[]` | VA の昇順。2 分探索 |
| `fle_dispatch_count` | `uint64_t` | |
| `fle_resp_table` | `fle_resp_entry[]` | 227 行、内容同一 |
| `fle_resp_count` | `uint64_t` | 227 |

重複しても問題ないもの（`fle_sse.h` が取り込む `fle_vex.h`、`fle_mmx.h`、`fle_mxcsr_flags.h` の weak 定義。リンクで 1 つに統合される）: `MM`、`YH`、`fle_teb`、`fle_mxcsr`、`fle_x87_mmx_active`、`fle_teb_set_stack`。`fle_tls_values` は `fle_imports.c` が 1 つ持つ（runtime）。

その他:
- `f_<VA>` の名前の衝突: **0**（FLEngine の part-00 3,700 関数と DLL の小さい生成物 689 関数で `T` シンボルの共通なし）。両モジュールの VA の範囲が重ならない限り衝突しない（FLEngine 0x403579〜0x13768a0、DLL は基底 + 0x1000 以降）。
- `imp_name_table` という名前は**生成 C にも runtime にも存在しない**（`lift/` の検索 0 件）。近いものはハーネスの `image.c` の静的な `imports[]` と `exports[]`（`SYM_MAX` 8192、`nimports`、`nexports`）だけで、これは 2 回目の `image_load` でも引き続き足されるだけで壊れない（実測: 2 つ目を読んだ後 `nimports=810`、`nexports=449`、DLL の ippFree が名前で引ける）。
- 生成 C の外では `harness/main.c`、`harness/wasm/main.c`、`harness/a4.c`、`harness/trace.c`、`harness/wasm/trace*.c` が `fle_dispatch_table` を直接読む。マージした 1 つの表なら変更なしで動く（実測: 試作の出力が単独の FLEngine と 1 行を除いて一致）。

### 4.3 (i) 共有するか、分けるか

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **S（状態を共有）** | `R`、`X`、`FL` を 1 つにし、dispatch 表を 1 つにマージ、`fle_dispatch` は 1 つ | モジュールをまたぐ呼び出し（ゲスト → DLL の `call rax`、runtime → DLL の `fle_call_guest`）が追加の橋渡しなしで動く。ゲストの呼び出し規約（`R[]`、`X[]`、スタックはゲストのメモリ）を共有しているので、IPP の引数（整数は `R[1]`、`R[2]`、`R[8]`、`R[9]`、浮動小数は `X[]`）がそのまま渡る | 2 つ目の生成 C を `extern` にする処理が要る（試作は Python で実現） |
| **D（状態を分ける）** | `R` などをモジュールごとに別の名前（例: マクロで置換）にする | 名前の衝突が起きない | モジュールの境界ごとに `R`、`X`、（`YH`）の値をコピーする橋渡しが要る。`fle_call_guest`（決定 0064）、`fle_dispatch`、`GetProcAddress` の戻り値の呼び出し、すべてが境界になる。runtime 側の `R[...]` 参照は 737 か所（36 ファイル）あり、どちらの `R` を指すか決める必要が出る |

**推奨（PM が決める）**: S。理由: 橋渡しが要らず、runtime の 737 か所の `R[...]` を変えずに済む。D の利点は名前の分離だけで、今回の重複は 7 個に限られ、S の `extern` 化で解消できる（実測）。

S の実現方法（生成器の側）:
- **試作（実測）**: Python の補助ツールで実現した。`split_mod.py`（全部の part を `R`、`FL`、`X` の `extern` にし、表の部分を別ファイルに出す）と `merge_tables.py`（2 つの表の VA を昇順にマージして 1 つの `common.c`（`R`、`FL`、`X` の定義と表を持つ）にする。同じ VA の行は同じ関数を指すことを表明する。`fle_resp_table` は同一であることを表明）。マージ後の表の行数: 全部入り 91,686（重複 227）、縮小 61,415（同）。
- **本番の案（未実装）**: (a) 上の補助ツールをそのままビルドの一部にする（生成器は不変。FLEngine の SHA は変わらない）、(b) `fe-lift` に「状態を extern にする」「表を `fle_dispatch_table_<module>` の名前で出す」オプションを足し、`fle_dispatch` が表のリストを順に引く。(b) の方が将来の 3 つ目のモジュールに素直だが、`fle_dispatch.h`（runtime）と生成器を変える。**どちらでも FLEngine 単独のときの出力は同一に保てる**（既定の動作を変えない場合。要件として SHA の一致を検査する）。

### 4.4 (ii) 呼び出しの境界

- **FLEngine → DLL**: `GetProcAddress` が返す値は、DLL の export の VA（リベース後。基底 + RVA）。ゲストは `mov rax,[cell]; call [rax]` の形（977 アクセス。external-modules-usage.md §1.2）で呼び、生成 C では `fle_dispatch(t)`（`call_ind:dispatch` 40,746 か所）になる。dispatch 表に DLL の関数が入っていれば `f_<VA>` が呼ばれる。**446 個の export はすべて表に載る**（実測: 全部入り 446/446、縮小 446/446）。同じアドレスを指す export が 1 組ある（`ippiResizeCenter_8u_C3R` と `ippiResizeCenter_8u_C4R`。実測）。ゲストが引く 420 名はすべて DLL の export にある（実測、`ipp_names.txt` と照合）。
- **決定 0015 の擬似 VA との関係**: 擬似 VA（0x7FF9_0000_0000 + 0x10 × (k+1)）は「runtime が実装する API」の行（`fle_imp_*`）で、DLL の export とは別。**同じ表に共存できる**（VA が重ならない。マージ後の表で確認）。区別は `GetProcAddress` の `hModule` が決める: 仮想モジュールのトークンなら応答表（擬似 VA）、DLL なら export の実アドレス。
- **DLL → FLEngine（呼び戻し）**: DLL の import は KERNEL32 の 72 名のみ（実測）で、ゲストの関数を指す import はない。DLL の export 446 は `ipps*` 366、`ippi*` 71、`ipp*` 8、`AggCreateInstance` 1 で、ゲストが引くのは 420（実測）。**IPP の関数が引数としてコールバックを受け取るか**は、関数の引数型を調べていないため**未確認**。もし受け取れば `fle_dispatch`（共有の表）で FLEngine の関数を呼べる。
- **DLL → runtime**: DLL は IAT 経由で `fle_imp_*` を呼ぶ（`call_imp:*` 84 か所、決定 0077）。runtime の関数が DLL のモジュールの名前を知る必要があるもの（`GetModuleHandleW(NULL)` は FLEngine を返す: §5）。
- 呼び出し規約の共有は `fle_call_guest`（決定 0064）の `R[4]` の扱いと同じで、`fle_dispatch(va)` で DLL の関数も呼べる（`va` が表にあれば）。

### 4.5 (iii) build.sh、split_c.py、harness の変更点

| 対象 | 現状 | 必要な変更（案） |
|---|---|---|
| `harness/wasm/build.sh` | `DLL` が 1 つ（`:32`）。`stage_gen`（1 本）、`stage_split`（`split_c.py ... 3700`）、`stage_gen_c`（`out/wasm-pdata-part-*.c` の glob）、`stage_link`（`obj-$VARIANT/gen/*.o` の glob） | モジュールの列（例: `fle dll`）ごとに gen / split を回す。DLL には前段（リベース）と `--seeds`（縮小時）を足す。**glob の罠**: 試作で `dll-[0-9]*.c` が `dll-2000000.c`（リベース後の未分割の C。536 MB）にも一致し、これを誤ってコンパイルした（`/usr/bin/time` の最大 RSS が 53 GB と表示され、13 GiB の RAM + 4 GiB のスワップでは実現できない値のため**この RSS の値は信用しない**。生成物は `two/quarantine/` に移した）。ファイル名は桁数の固定した連番にして、glob を厳しくする |
| `harness/wasm/split_c.py` | part-00 が定義と表、part-01 以降は `extern`（1 モジュール前提） | 2 つ目のモジュールの全 part を `extern` にし、表は共有の 1 つの TU（`common.c`）に出す（試作の `split_mod.py`、`merge_tables.py`）。part の大きさは DLL では 800 関数で足りる（試作: 最大 part 45 秒。巨大関数を含む part が 9 GB 級になる点は §4.6） |
| `harness/image.c` | 静的な `imports[]`、`exports[]` を共有。`image_load` は 1 回呼べば 1 つの PE を置く | 2 回呼んでも壊れない（実測）。wasm の分岐は `ibase + size > __global_base` で失敗する条件のみ。DLL の P1、P2 は通る。戻り値の `nimports` は累積値 |
| `harness/main.c`、`harness/wasm/main.c` | `argv[1]` が 1 つの DLL、`argv[2]` がエントリの VA | DLL のパスを足す引数（または環境変数）。`fle_mem_set_image`（`FLE_CHECKED_MEM` 用）の 2 範囲化。DLL を runtime に登録する関数（§5） |
| `harness/a4.c`、`a4.h` | `a4_set_image` が 1 イメージ。FLEngine の固定 VA（InitContext 0x15EAC90）を読む | 変更不要（DLL は FLEngine の InitContext を持たない）。ただし DLL だけを読み込んだときは `a4_report` が FLEngine のアドレスを読むのでレポートが出ない（native で実測: DllMain だけの実行は終了コード 5 で、停止レポートが出なかった。原因は未調査。仮説: 停止レポートが FLEngine のアドレス 0x15EAC90 を読んで落ちる） |
| `harness/a4-survey.sh`、`build_native.sh` | 1 モジュール | モジュールを足す。現行の基準（停止点が `LoadLibraryW "dsp_ippv2_x64.dll"`、init_count 145/942、関数実行数 821）は DLL を渡さないときに変えない |
| `runtime/fle_host.c`、`fle_runtime.h` | `fle_mem_img_lo`、`fle_mem_img_size`（単一範囲） | `FLE_CHECKED_MEM` を使う場合に 2 範囲化（既定オフ。オフなら変更不要） |

### 4.6 (iv) wasm のサイズ、リンク時間、メモリの実測

すべて `emcc 6.0.12`、`build.sh` と同じフラグ（`-O0 -finstrument-functions`）、この環境（16 コア、RAM 13 GiB、スワップ 4 GiB）。

| 構成 | 生成 C | wasm | コンパイル（emcc `-c` の時間の合計。同時 3 本） | リンク | node で FLEngine を実行（LoadLibraryW の停止まで） | 区分 |
|---|---|---|---|---|---|---|
| FLEngine のみ（HEAD のコピー） | 215.7 MB | 115,903,130 B（旧ログの 115,721,618 B と近い） | 70.8 秒（17 part。最大 6.1 秒、最大 RSS 1.76 GB） | 4.9 秒、612 MB | 103.7 秒（`--liftoff-only` で 103.4 秒） | 実測 |
| + 縮小 DLL（`table-seeds-idx0` の 428 種。1,891 関数。リベース 0x2000000） | +40.8 MB | **156,393,882 B**（DLL 分は約 40.5 MB） | DLL 分 19.7 秒（3 part: 9.6、8.0、2.1 秒。最大 RSS 2.3 GB）。common.c 0.6 秒 | 1.1 秒、770 MB | 108.4 秒。停止点と出力は単独と同じ（`calling entry` の `rsp` 以外の差なし） | 実測 |
| + 全部入りの DLL（`--pdata`。32,162 関数） | +536.0 MB | **533,108,454 B**（DLL 分は約 417 MB） | DLL 分 233.8 秒（41 part × 800 関数。最大の part 45.2 秒 / 9.3 GB、巨大関数を含まない part は 1〜7 秒 / 0.3〜1.8 GB）。common.c 5.6 MB | 2.9 秒、2.2 GB | **実行できない**（下） | 実測 |

- **全部入りは node が `local count too large` でコンパイルを拒否する**（実測: `CompileError: WebAssembly.instantiate(): Compiling function #80523 failed: local count too large @+294969179`、0.98 秒で失敗）。WebAssembly JS API の上限は「1 関数の locals は 50,000」「1 関数の本体は 7,654,321 B」「モジュールは 1 GiB」（仕様の Implementation-defined Limits、取得 2026-10-23）。全部入りの wasm（98,011 関数）の locals が上限を超える関数は **13 個、最大は 389,948**（本体 6,048,053 B は本体の上限の 79%）。FLEngine は最大 20,136 で 0 個（実測）。最大の 2 関数（`f_3d3a420`、`f_3d04cf0`。生成 C で 25,743 行、1,838,161 B と同じ大きさ。中身の同一性は未確認）の locals が多い原因は、生成 C が命令ごとに `{ uint64_t t = ...; }` の一時変数を作り、`-O0` ではそれぞれが別の local になるため、と読める（読み。`-O1` で 11〜19 に減ることと整合するが、個々の local の由来は見ていない）。この 2 関数が AVX-512 の関数かどうかは確認していない。
- 対策の予備測定（最大の 2 関数だけを取り出した TU と、それを含む part-39）: `-O1` にすると locals は最大 11〜19 になるが、**2 関数で 872 秒、part-39 全体で 1,130 秒**（`-O0` は 10 秒と 28 秒）。リンク済みの 12 MB の wasm に `wasm-opt --coalesce-locals` をかけたが、**30 分で終わらず打ち切った**（結果なし）。
- 縮小 DLL は最大 locals 41,126（上限の 82%。余裕は 18%）。**この余裕が小さい点に注意**: 種を少し足すと上限を超えうる（仮説）。
- 実現性調査 §3.3 の推測（全部入りの wasm は約 303 MB、FLEngine と合わせて約 420 MB）に対し、実測は DLL 分が約 417 MB（FLEngine と合わせて 533 MB）で、推測より DLL 分が約 38% 大きい。ただし縮小でなければ**動かない**ので、サイズは問題の本質ではない。
- **node の実行時間**: 単独の FLEngine が LoadLibraryW の停止まで 103.7 秒かかる。native は同じ停止点まで 0.04 秒（2 モジュールの native 版で実測）。旧ビルド（`lift/out-wasm2/bin-base`。GetLocaleInfoW で停止する古い生成 C）は 1.6 秒。**原因は調査していない**（未確認）。テストの回転に効く観察として記録する。
- ブラウザの wasm の大きさの上限は確認していない（未確認）。node の既定の上限は 1 GiB（実現性調査 §3.3）。

**推奨（PM が決める）**: 当面は縮小 DLL（SSE2 系統のみ）で進め、全部入りは W8（AVX の方針）の決定後に再検討する。理由: 全部入りは今の生成器のままでは wasm として実行できない（実測）。縮小は FLEngine と同じツールチェーンで動き、DllMain の途中まで実行できた（§5.4）。縮小の前提は「選択値が 0 になる」仮説（実現性調査 §5.4）で、**未確認**。
## 5. LoadLibraryW / GetProcAddress / FreeLibrary / GetModuleHandleW / GetModuleFileNameW の接続

### 5.1 「FLE_EXE_HANDLE = ゲスト自身（0x400000）」前提の箇所（`lift/runtime` の grep。HEAD のワークツリーの行番号）

`FLE_EXE_HANDLE` を使う箇所:

| ファイル:行 | 現状 | DLL を実在モジュールにするとき必要な変更（案） |
|---|---|---|
| `fle_modules.h:19` | `#define FLE_EXE_HANDLE UINT64_C(0x400000)` | 定数のまま「プロセスを作ったファイル = FLEngine」を表す（`GetModuleHandleW(NULL)` は FLEngine のまま: 実機の `.exe` の位置づけと同じ）。**DLL のハンドルは別の値**（§5.3） |
| `fle_modules.c:81` | `GetModuleHandleW(NULL)` → `FLE_EXE_HANDLE` | 変更なし。名前つきの検索（`:111-116`）に DLL を足す |
| `fle_modules.c:153-156` | `GetProcAddress(FLE_EXE_HANDLE, ...)` を abort | FLEngine 自身の export（3 個。実測: `exports=3`）は引き続き未対応のままでよい（ゲストが自分の export を引くかは**未確認**）。DLL のハンドルなら export 表を引く（§5.2） |
| `fle_modules.c:157-165` | ハンドルが仮想モジュールのトークン（0x7FF8… + 0x10000 × N）でなければ abort | DLL のハンドルを先に判定する分岐を足す。トークンの範囲検査（`> FLE_MODULE_N`）に DLL が入らないことを確認 |
| `fle_modules.c:15-43,123-124,176-183` | 仮想モジュール 26 名の表と、`fle_resp_table`（weak）の線形探索 | 変更なし。DLL の export は別の表（§5.2） |
| `fle_load.c:226` | `FreeLibrary(FLE_EXE_HANDLE)` → stop | 変更なし（FLEngine は解放しない）。DLL のハンドルなら参照カウントを減らす（0 になったら DLL_PROCESS_DETACH を呼ぶ。§5.4） |
| `fle_load.c:139-167`（`load_by_name`） | 名前が仮想モジュール表にあれば参照 +1、`fle_load_absent[]` にあれば NULL と 126、それ以外は stop | 表に無い `dsp_ippv2_x64.dll` を、新しい「実在モジュール表」から引く。**初回だけ DllMain（reason 1）を呼ぶ**。2 回目以降は参照 +1 だけ（Learn: 参照カウントを増やす。DllMain は未ロードのときだけ呼ぶ） |
| `fle_load.c:58-69` | `refs[LOAD_MODULES_MAX = 64]`。初期値は全仮想モジュールが 1 | DLL 用の参照カウントを別に持つ（初期値は 0。ゲストが LoadLibrary するまでロードされていない） |
| `fle_vfs.c:1453` | `GetModuleFileNameW(h)` は NULL か `FLE_EXE_HANDLE` のときだけ。値は `FLE_VIRTUAL_DLL_PATH`（`"C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll"`、`fle_env.h:39`） | DLL のハンドルなら DLL の仮想パスを返す。**パスの値は決定が要る**（候補: FLEngine と同じディレクトリの `Shared\dsp_ippv2_x64.dll`。ゲストが `SetDllDirectoryW("Shared\\")` を呼ぶ実測と整合するが、値は創作になる） |
| `fle_vfs.c`（`FLE_VIRTUAL_DLL_PATH`）、`fle_env.c:261-272`（`GetCommandLineW`） | 「プロセス = FLEngine」の仮想パスを引用符つきで返す | 変更なし（DLL はプロセスではない）。DLL の `GetCommandLineW` 呼び出し（`call_imp:GetCommandLineW` 1）は FLEngine のコマンドラインを見る。実機でも同じ |
| `fle_res.c:46-63,65-90` | リソース API はイメージ `FLE_EXE_HANDLE` の `.rsrc` だけ。他の仮想モジュールは「リソースなし」で stop | ゲストが DLL のリソースを引くかは**未確認**。DLL の `.rsrc` は 0x1e0 B（バージョン情報程度と読めるが、中身は未確認）。引くなら DLL のハンドルを `img_open` の引数にする（現状は定数 0x400000 を直接使う: `:67`） |
| `fle_imagehlp.c:38` | `ImageNtHeader(Base)` は `Base == FLE_EXE_HANDLE` だけ | DLL の基底も許すなら 1 行足す。ゲストが DLL に対して呼ぶかは**未確認** |
| `fle_class.c:130,173,206`、`fle_win.c:154`、`fle_loadimage.c:84` | `hInstance` が `FLE_EXE_HANDLE` でなければ stop | DLL が `hInstance` を渡す形かは**未確認**（DLL の import に user32 系は無い。72 名はすべて KERNEL32。実測）。変更不要と読める |

`FLE_EXE_HANDLE` を使わず 0x400000 や 0x18c0000 を直接書いている箇所:
- `fle_res.c:71,77`（エラーメッセージ内の「0x400000」）、`fle_win.c:34`（`_Static_assert` のコメント）、各ヘッダのコメント。動作には影響しないが、基底を変える作業でコメントの文言を揃える。
- `fle_host.c:13-14`（`fle_mem_img_lo`/`size` の既定値）。`FLE_CHECKED_MEM` でだけ使う。
- `fle_callback.h:30`（`FLE_CB_RETURN_ADDRESS` 0xCB000000）は「イメージの終端 0x18C0000 を越えた値」という前提のコメント。**P1 の DLL（0x2000000〜0x461f000）と 0xCB000000 は重ならない**（導出）。P2 の 0x8000000〜0xa61f000 とも重ならない。P3（0x10000000〜）も重ならない。

DLL の import から runtime が呼ばれる API で、モジュール単位の前提を持つもの（読み、実現性調査 §6 と決定 0077 の表）:
- `GetModuleHandleW`（`call_imp` 1。DLL は `0x37d19b3` で呼ぶ。引数は R[1] = 0 とは限らない: 該当の関数 `f_37d198c` の前後は読んだが、引数が NULL かは**未確認**）、`GetModuleFileNameW`（1。`0x37d637b`。バッファ 0x105 文字、引数 `hModule` は R[1] を `xor ecx,ecx` 直後ではないので NULL とは限らない: **未確認**）、`GetModuleHandleExW`（`jmp`/`call` 形で応答表に行なし。実現性調査 §6.1 で 1 か所）。
- `LoadLibraryExW`（4）、`FreeLibrary`（4）、`GetProcAddress`（3）: DLL の CRT が `api-ms-win-*` を遅延解決するために使う（§5.4。実測で ATTACH の最初の停止原因）。

### 5.2 GetProcAddress が DLL の export に返す値

- 値は **export の実アドレス**: リベース後の `基底 + export RVA`。全部入り、縮小のどちらでも 446/446 が dispatch 表にあり、`fle_dispatch(va)` で呼べる（実測。基底 0x2000000 のとき `ippFree` は 0x273c600、`AggCreateInstance` は 0x37c82f0）。
- ゲストは返された値を `[保存先]` に置き、`call [rax]` で呼ぶ（読み）。**値が表にあれば呼べる**（擬似 VA か実アドレスかをゲストは区別しない）。
- 決定 0015 の擬似 VA 方式は「runtime が実装する API の関数アドレス」の方式。DLL の export は**実コード**なので擬似 VA にしない（決定 0073 の「実在のコードで答える」と整合）。擬似 VA のまま `fle_resp_table` に DLL の export を足す案もありうる（DLL の関数を `fle_resp_table` の `fn` に指す）が、`va` が 0x7FF9… の値になる。**ゲストが戻り値をアドレスとして使う（DLL 内の関数のアドレスと比較する、DLL の範囲内かを検査する）場合に差が出る**。使うかは**未確認**。実在のアドレスを返す案を推奨する（理由: 実機と同じ種類の値になる。PM が決める）。
- export 表の持ち方: (a) 実行時に DLL のメモリ上の export ディレクトリを読む（`image.c` は既にファイルから読んで名前と VA の表 `exports[]` を作っている。449 個: 実測。名前つき 446 + FLEngine 3）、(b) 生成器が `fle_dll_exports[]`（名前と VA）を出力する。(a) は runtime から `image.c` の静的な表に届くアクセサが要る。(b) は生成器の変更。どちらも必要な情報はファイルにある（実測）。
- 序数指定（`lpProcName` の上位が 0）は DLL の export にも序数ベース 1 の序数がある（実測: 446 件すべて名前あり、フォワーダ 0）。ゲストは名前で引く（`ipp_names.txt` の 420 名）ので序数対応は必須ではない。
- 同じアドレスの export が 1 組あるので、「アドレスから名前を引く」実装（trace など）は 1 対 1 でない（実測）。

### 5.3 モジュールハンドルの値

選択肢:

| 案 | 値 | 利点 | 欠点 |
|---|---|---|---|
| H1 | DLL の基底（実イメージのアドレス。P1 なら 0x2000000） | Windows の仕様と同じ（DllMain の `hinstDLL` は基底アドレス。Learn DllMain「The value is the base address of the DLL」）。`ImageNtHeader(base)`、`GetProcAddress` の `hModule`、リソース API がメモリ上のヘッダを読める。DllMain の第 1 引数（R[1]）に渡す値と同じ | 仮想モジュールのトークン（0x7FF8…）と別の判定が要る。`FLE_EXE_HANDLE` = 0x400000 と同じ種類の値で素直 |
| H2 | 仮想モジュール表の続き（0x7FF8… + 0x10000 × 27） | 既存の判定コード（`fle_load.c`、`fle_modules.c`）が使える | 「ハンドル = 基底」でなくなる。DllMain に渡す値を別にしないと仕様と合わない |

読み: H1 が仕様に合う。`FLE_EXE_HANDLE` も基底そのものなので、方針が揃う。

### 5.4 DllMain（reason = 1）の呼び出し順序と再入

Learn の DllMain の記述（取得済み）から:
- `LoadLibrary` が新規に DLL を読むと、システムはその DLL のエントリポイントを `DLL_PROCESS_ATTACH`（1）で、`LoadLibrary` を呼んだスレッドの中で呼ぶ。戻り値が FALSE なら `LoadLibrary` は NULL を返し、DLL は PROCESS_DETACH（0）を受けて解放される。
- `lpvReserved` は動的ロード（`LoadLibrary`）なら NULL。
- DllMain は `LoadLibrary` や `FreeLibrary` を呼んではならない（Learn。DLL の CRT が `api-ms-win-*` を引くのはこの禁止の対象外の経路かもしれない。**DLL の CRT が `LoadLibraryExW` を呼ぶのは実測**）。
- `THREAD_ATTACH`/`THREAD_DETACH` は、スレッドが 1 つなので呼ばれない（決定 0017、0045）。ゲストが `DisableThreadLibraryCalls` を呼ぶかは未確認（DLL の import には無い）。

接続の案（`fle_imp_LoadLibraryW` の中で）:
1. 名前を正規化し、実在モジュール表を引く（新規）。
2. 未ロードなら、ロードの状態を「初期化中」にして `fle_call_guest(entry, hinstDLL = 基底, DLL_PROCESS_ATTACH = 1, lpvReserved = NULL, 0)` を呼ぶ（決定 0064 の関数。引数は rcx、rdx、r8、r9 の 4 つ。DllMain の 3 引数は収まる）。`fle_call_guest` は `R[4]` を呼び出し点より下げて、callee-saved レジスタ（rbx、rbp、rsi、rdi、r12〜r15）と `R[4]` の復帰を検査し、深さ 16 を超えると止まる。
3. 戻り値（`eax`）が 0 なら、`LoadLibrary` は NULL（`SetLastError` は DLL が設定した値のまま。Learn は設定を明示していない）、DLL は PROCESS_DETACH を受けて解放される（実装するか、stop にするかは決定が要る）。
4. 戻り値が非 0 ならハンドルを返す。
5. **`LoadLibraryW` はゲストのコード（`f_44db40` → `f_6539e0`）から呼ばれる**ので、`fle_call_guest` の再入になる（LoadLibraryW のスタブは `R[4]` がゲストの戻りアドレスを指した状態で呼ばれ、その下を使う）。深さは 1 段で足りる（DllMain の中でさらに `fle_imp_*` を呼ぶが、それらは `fle_call_guest` を呼ばない。ただし DLL の CRT の `LoadLibraryExW` が実在モジュール表を引く場合、そのモジュール（`api-ms-win-*`）の DllMain は呼ばない（仮想モジュールなので））。
6. `fle_imp_*` はゲストの `R[1]`、`R[2]` を読む規約なので、`fle_call_guest` の呼び出し前後で `R[0]`（戻り値）と `R[4]` の整合、`fle_teb` の StackLimit（決定 0076: `R[4]` が登録範囲内であること）が保たれる。`fle_call_guest` は同じスタックの下を使うので `fle_teb_set_stack` の更新は不要（決定 0076 の表）。

**実測（縮小 DLL を P1 に置いた 2 モジュールの native と wasm で、DllMain を直接呼んだ結果）**: native も wasm も、DllMain の ATTACH の中で `LoadLibraryExW("api-ms-win-core-synch-l1-2-0.dll")` に当たって停止した（`gdb` のバックトレース: `f_37d0788` ← `f_37d0a04` ← `f_37d071c` ← `f_37cd210` ← `f_37cb484` ← `f_37ca988`。ファイルの RVA 0x17d0788 など。逆アセンブルで確認: `r15 = [表+k*8+0x1ec7f98]` の名前を `LoadLibraryExW(name, 0, 0x800)`（`LOAD_LIBRARY_SEARCH_SYSTEM32`）で引き、失敗なら `GetLastError` が 0x57 のとき `LoadLibraryExW(name, 0, 0)` を再試行、さらに失敗なら失敗を表に記録して `GetProcAddress` へ進む）。この関数の呼び出し元は `InitializeCriticalSectionAndSpinCount` を遅延解決する CRT のラッパー（実現性調査 §6.1 の `0x17cb484 → 0x17cd210 → 0x17d071c → 0x17d0a04` の鎖と一致）。DLL の名前表（RVA 0x1ec7f98）の中身は `api-ms-win-core-fibers-l1-1-1`、`api-ms-win-core-synch-l1-2-0`、`kernel32` の 3 件（実測）。

→ **ATTACH を通すには、`api-ms-win-*` と `kernel32` に `LoadLibraryExW` がどう答えるかを決める必要がある**（§6 の決定 D4）。選択肢は §6。

### 5.5 `GetModuleHandleW` と `GetModuleFileNameW` と `FreeLibrary` の DLL 側の呼び出し

- DLL の `GetModuleHandleW` の呼び出し（`0x37d19b3`、基底 0x2000000 の値。ファイルの RVA 0x17d19b3）と `GetModuleFileNameW`（`0x37d637b`、RVA 0x17d637b）は、CRT の例外処理系（`0x17d06f7` など）か致命エラーの経路に見える（実現性調査 §3.4 の読み）。ATTACH の直線上にあるかは**未確認**。
- DLL の `FreeLibrary`（4 か所）は、`api-ms-win-*` を引いた直後の後始末（上の `0x17d088b`: 他のスレッドが同じ表のスロットを先に埋めていたときに、自分が引いたハンドルを解放する経路）。`FreeLibrary` の仮想モジュールの規則（参照が 1 のときは stop: `fle_load.c:231`）に当たる可能性がある。**`api-ms-win-*` を仮想モジュールにするなら、`LoadLibraryExW` の参照 +1 と `FreeLibrary` の −1 が釣り合うので stop しない**（読み）。

## 6. 未決事項（値の決定が要るもの）

| # | 決めること | 選択肢（事実） | 推奨（PM が決める） |
|---|---|---|---|
| D1 | リベース先の基底 | §3.2 の P1（0x2000000。wasm 実行済み）、P2（0x8000000）、P3（`GLOBAL_BASE` を上げる） | P1。理由: wasm で DllMain の途中まで動いた（実測）。値の意味は「FLEngine の終端の直後」以外にない（創作の値ではなく、衝突しない範囲の選択） |
| D2 | リベースの方式 | §3.4 の (A) ファイル段階 / (B) ロード時 | (A) |
| D3 | 状態の共有 | §4.3 の S / D | S |
| D4 | DLL の CRT が引く `api-ms-win-*`（実測: `api-ms-win-core-synch-l1-2-0`、`api-ms-win-core-fibers-l1-1-1`）と `kernel32`、`user32`、`advapi32` などへの `LoadLibraryExW` の答え | (a) 仮想モジュール表に足す（ロード成功にする）。`GetProcAddress` の応答表に行が無い API は NULL・127 になり、DLL は `GetProcAddress` の失敗を許す（読み: 失敗を記録して先へ進む）。(b) `fle_load_absent[]` に足す（NULL と 126）。実機で `api-ms-win-core-synch-l1-2-0` は Windows 8 以降にあるので「無い」は偽の主張（決定 0014 の Windows 10）。(c) stop のまま。決定 0055（msctf.dll）の基準は「環境の定義と矛盾する場合は、欠いているとは定義しない」で、(a) と整合する | (a)。**ただし `api-ms-win-*` は API セットの名前で、実機は `kernel32`/`kernelbase` に転送する。どの名前をどの API の集合として答えるかは未確認。DLL が引く API 名（`GetProcAddress` の引数）を実際に確認してから決める** |
| D5 | モジュールハンドルの値 | §5.3 の H1（基底）/ H2 | H1 |
| D6 | DLL の仮想パス（`GetModuleFileNameW`） | 値は創作になる。FLEngine と同じディレクトリの `Shared\`（`SetDllDirectoryW("Shared\\")` の実測と整合）が候補 | 決定が要る（PM の値） |
| D7 | 全部入りの DLL を扱うか、縮小 DLL に留めるか | §4.6: 全部入りは今のままでは wasm 実行不可（locals 上限）。解決の候補: (a) 生成器が巨大関数の一時変数を再利用する（locals を減らす）、(b) 巨大関数を分割して生成する、(c) `-O1`（実測: 872〜1,130 秒/part。locals は解決）、(d) 縮小（SSE2 系統の種）に留める | (d) をまず。全部入りは AVX の方針（W8）の決定後 |
| D8 | CPU 振り分け（CPUID）と XGETBV | 現行のまま（選択値 0 の仮説）。変えるなら XCR0 の値の決定が先（決定 0047） | 現行のまま（仮説の確認を最初の実行で行う） |
| D9 | スレッドの扱い | wasm は単一スレッド前提（決定 0017、0045）。DLL は `CreateThread` を持たない（実測）が、`ippSetNumThreads` がある | 現行のまま。DLL が `DLL_THREAD_*` を受ける経路は作らない |
| D10 | メモリ | wasm の最大メモリ 2 GiB は現状の使用量（node の最大 RSS 約 0.4〜0.5 GB）で足りるが、アリーナ 512 MiB が `aligned_alloc` できる余裕は**FLEngine が DllMain を完走した後の使用量が未知**。DLL の `HeapAlloc` が増やす分も未測定 | 測定してから（段階 4 で）決める |
| D11 | DLL を `FreeLibrary` したときの扱い | 参照 0 で DLL_PROCESS_DETACH を呼ぶ（`lpvReserved` NULL）か、ロード済みのまま残す（stop）か | 段階 5 で（ゲストが `FreeLibrary` を呼ぶ経路は `f_65c4f0`。実行時に届くかは未確認） |

## 7. 実装の段取り案

毎段階で a4（`harness/a4-survey.sh` の停止点: LoadLibraryW "dsp_ippv2_x64.dll"、init_count 145/942、関数実行数 821）と `cargo test --workspace`（基準は決定 0077 の 1,044 件）が緑のまま進める。FLEngine の生成 C の SHA-256（67827424…15584d）が変わらないことを毎回確認する。

| 段階 | 内容 | 検証 | 既存への影響 |
|---|---|---|---|
| 0 | ファイル段階のリベースを入れる（`fe-pe` の隣の補助ツール。`ipp_rebase.py` の Rust 版。出力は新しいファイル） | 同じ DLL を 2 回リベースして SHA が決定的。`fe-lift` の統計がリベースなしと同一（今回の実測を回帰テスト化）。15,602 件すべてがファイルの裏付けあり（`not file-backed 0`） | なし（新規） |
| 1 | **合成 PE で 2 モジュールの機構だけを検証する**（実 DLL なし） | 下の §7.1 | runtime に実在モジュール表を追加（FLEngine だけのときは空） |
| 2 | 2 つの生成 C のリンク（S 案）。縮小 DLL（`table-seeds-idx0` 相当、種は基底に合わせてリベース）を FLEngine と同じバイナリ（native と wasm）にリンク | 今回の実測の再現: FLEngine の停止点と出力が単独のときと同じ（`rsp` 以外）。dispatch 表のマージで同一 VA に別関数が無い（表明）。`R`/`FL`/`X` の重複定義がない（リンクが通る） | `build.sh` を 2 モジュール対応（DLL を渡さないときは従来どおり） |
| 3 | `LoadLibraryW`/`GetProcAddress`/`FreeLibrary` の接続（§5.1）。まず合成 DLL、次に実 DLL（縮小）。DllMain を呼ぶところまで | 合成 DLL: 参照カウント、DllMain の戻り値 0 で NULL、戻り値 1 でハンドル、`GetProcAddress` が表の VA を返し `fle_dispatch` で呼べる。実 DLL: 縮小 DLL で ATTACH が D4 の停止まで | `fle_load.c`、`fle_modules.c`、`fle_vfs.c` を変更（FLEngine のみの既存テストは通ること） |
| 4 | D4（`api-ms-win-*`）と、`InitializeCriticalSectionAndSpinCount` ほか W6 の API。DllMain を完走させる | DllMain の戻り値 1。FLEngine の a4 が `LoadLibraryW` を越えて進む（次の停止点は決定 0073 の記述では `RegCreateKeyExW`） | runtime の API の追加 |
| 5 | 420 名の `GetProcAddress`（ゲスト `f_653f40` → 423 回） | 保存先の 420 スロットが非 NULL。選択値が 0 になる仮説の実行確認 | なし |
| 6 | 縮小の外へ（全部入り、AVX の方針） | W8 の決定後 | wasm の locals 上限（D7）の解決が前提 |

### 7.1 段階 1 の合成 PE（既存テストの作り方に倣う）

- 作り方: `crates/fe-decode/tests/multisec.rs` が合成 PE32+ を手で組んでいる（`LFANEW`、`OPT_SIZE`、`section()`、`w16/w32/w64` の補助関数。7 セクション、`.pdata`、`.reloc` の DIR64 を持つ）。`crates/fe-pe/tests/synthetic.rs` が export ディレクトリを持つ合成 PE（`Alpha @0x1500`、`Beta` のフォワーダ）を作る。**この 2 つを合わせれば、export 2〜3 個と DllMain（`mov eax,1; ret` のような数バイト）を持つ合成 DLL が作れる**（今回は作っていない。読み）。`tests/call_iat.rs` は合成 PE から生成した C を `clang` で runtime とリンクして native で実行し、値を固定する流儀（`R[0] = 80` など）。段階 1 のテストはこの流儀に合わせる。
- 確かめられること: (a) リベース: `.reloc` の DIR64 に差分を足した合成 DLL で、データ中のポインタ（`.data` の関数ポインタ）が dispatch 表の VA と一致する。(b) 2 つの生成 C のリンク（`R`、`FL`、`X`、表の重複なし）。(c) `LoadLibraryW` が DllMain を呼ぶ順序（DllMain が `R[1]`、`R[2]` を `[データ]` に記録し、テストが読む）。(d) `GetProcAddress` の戻り値を `call [rax]` で呼べる。(e) DllMain が 0 を返したときの NULL。(f) `fle_call_guest` の再入（LoadLibraryW を呼ぶゲストのコードから）。(g) `FreeLibrary` の参照カウント。
- 限界: 合成 DLL は CRT も `__chkstk` も SSE も持たないので、実 DLL の問題（`api-ms-win-*`、AVX、locals 上限）は検出できない。それらは段階 2 以降の縮小 DLL で測る。**合成 DLL は機構の正しさだけを保証し、実 DLL の動作は保証しない**。

**可否**: 合成 DLL で 2 モジュール機構を先に検証してから実 DLL に進む案は**可能**（部品が揃っている。読み）。推奨する。理由: 実 DLL は ATTACH で `api-ms-win-*` の決定（D4）に当たるので、先に機構を固めてから D4 を決められる。

## 8. 実測 / 読み / 未確認 / 仮説

### 実測（今回のコマンドの出力）

- fe-lift の統計（関数 32,162、emit 命令 6,504,638、未対応 973,919、bad_nodes 0）は、DLL をリベース 0x1cc0000、0x2000000、0x8000000 に置いても、リベースなしと同一。`UINT64_C(0x18…)` の即値は 1,135,257 → 0。生成 C は 565,516,290 → 536,004,496 B。
- FLEngine の生成 C（HEAD のコピー）は 215,700,923 B、SHA-256 67827424…15584d（決定 0076、0077 の基準と同一）。
- 重複定義 7 個（`R`、`X`、`FL`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_table`、`fle_resp_count`）と `f_<VA>` の衝突 0。`imp_name_table` は存在しない。
- `fle_resp_table` は両モジュールで同一（227 行）。dispatch 表のマージ: 全部入り 91,686 行、縮小 61,415 行（共通 227）。DLL の export 446 すべてが表に載る。ゲストが引く 420 名はすべて export にある。
- DLL の DIR64 15,602 スロットの値はすべてイメージ内。`.reloc` に無いがイメージ範囲に入る 8 バイト値が 2 つ。
- `image.c` は 2 回呼べる（native。2 つ目のイメージの export 名が引ける）。native の固定 `mmap`（FLEngine を先に置いた状態）は 7 つの候補基底すべてで成功。
- wasm のメモリ配置: `GLOBAL_BASE` 0x10000000、データの後ろに 8 MiB の C スタック、`__heap_base`、最大 2 GiB / 初期 640 MiB（リンク後のバイナリを直接読んだ）。
- 2 モジュール（縮小 DLL）の native と wasm のビルド、リンク、実行。FLEngine の停止点（LoadLibraryW）、関数実行数 821、エントリ数 15,685 が単独のときと同じ。
- DllMain を P1 の基底で単独実行 → native と wasm とも `LoadLibraryExW("api-ms-win-core-synch-l1-2-0.dll")` で停止。
- 全部入りの wasm（533,108,454 B）は node が `local count too large`（関数 #80523）で拒否。locals が 50,000 を超える関数 13 個、最大 389,948。縮小は最大 41,126。FLEngine は最大 20,136。
- `-O1` は locals を 11〜19 に減らすが、2 関数で 872 秒、part-39 で 1,130 秒（`-O0` は 10 秒、28 秒）。
- wasm のサイズ、コンパイル時間、リンク時間（§4.6 の表）。FLEngine 単独の node 実行は停止まで 103.7 秒。

### 読み

- ゲストのポインタは 2 種類（イメージ内の VA、ホストのアドレス）で、恒等写像。
- ロード時リベース（B）に必要な生成器の変更は大きい（試作なし）。
- S 案の橋渡し不要の利点、D 案の欠点（`R[...]` 参照 737 か所）。
- `FLE_EXE_HANDLE` の使用箇所と、DLL を実在モジュールにするときの変更（§5.1）。
- DllMain の `LoadLibraryW` の中での呼び出しと `fle_call_guest` の再入（§5.4。仕様は Learn の DllMain）。

### 未確認

- 縮小 DLL の「選択値が 0 になる」仮説（DllMain が完走していない）。
- ゲストが DLL の `HMODULE`（リソース、`ImageNtHeader`、アドレスの範囲比較）をどう使うか。
- DLL の IPP 関数がコールバックを受け取るか。
- `api-ms-win-*` 名が `GetProcAddress` で引く API 名の集合（D4）。
- FLEngine が DllMain 完走後に使うメモリ量（D10）。アリーナの FLEngine 上での位置。
- P2、P3 の wasm での実行。非 PIE の native で P1 が置けるか。
- ブラウザの wasm の大きさと locals の上限。`wasm-opt --coalesce-locals` が locals を減らせるか（30 分で打ち切り）。
- 全部入りの最大 2 関数が AVX-512 の関数か、中身が同じか。
- node で FLEngine 単独が停止まで 103.7 秒かかる原因（旧ビルドは 1.6 秒）。native は 0.04 秒。
- DllMain だけの実行で停止レポートが出なかった原因（終了コード 5）。
- ゲストが `FreeLibrary(DLL)` を呼ぶ経路が実行されるか。

### 仮説

- `.reloc` に無い 2 つの値（0x180000029）は定数。
- 縮小の種を足すと locals の上限（41,126 / 50,000）を超えうる。
- DllMain のレポートが出ない原因は FLEngine のアドレスの読み取り（a4）。

## 9. 作ったファイル

変更・作成したのは次の 2 か所だけ（`lift/` の既存ファイル、runtime、生成器、テストは変更していない）。

- `docs/investigations/two-module-design.md`（この文書。新規）。
- `lift/out-design/`（gitignore 対象。合計 約 6.5 GB）:
  - `stage/`（`git archive HEAD` の展開）、`target/`（fe-lift/fe-decode/fe-pe の release）、`tmp/`、`emcache/`。
  - `tools/`（補助ツール: `dd_tables.py`、`pe_exports.py`、`wasm_layout.py`、`wasm_func.py`、`wasm_locals.py`、`big_funcs.py`、`count_imm.py`、`extract_funcs.py`、`split_mod.py`、`merge_tables.py`）。
  - `dll/`（リベースしたコピー: `dsp_ippv2_x64.base{1cc0000,1d00000,2000000,8000000}.dll`。**実在の DLL ではなく測定用の改変コピー**）。
  - `gen/`（FLEngine の生成 C `flengine.c` と統計）、`meas/`（統計、実行ログ、export 表）、`link/`（`ld -r` の重複定義の確認）、`probe/`（`layout_probe.c`、`native_probe*.c`、`two_image.c`）、`mapprobe/`（リンクマップ）。
  - `wb/`（FLEngine 単独の wasm ビルド）、`two/`（全部入り DLL との 2 モジュールのビルド。`two/bin/two.wasm` 533 MB、`two/quarantine/`（誤ってコンパイルした 586 MB の `.o` 2 個とログ））、`red/`、`red2/`（縮小 DLL の生成と wasm）、`nat/`（native の 2 モジュールのバイナリ）、`r2/`。

注記: 規則により削除をしていないため、誤って作った `two/quarantine/` の 1.1 GB と、不要になった大きな中間物（`two/out/dll-2000000.c` 536 MB、`two/bin/two.wasm` 533 MB、`nat/two-native` 311 MB など）はそのまま残っている。PM が不要と判断したら `lift/out-design/` ごと整理できる。
