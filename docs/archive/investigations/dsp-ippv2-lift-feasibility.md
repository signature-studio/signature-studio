# dsp_ippv2_x64.dll を 2 つ目のモジュールとして lift できるか（実現性調査）

実行日: 2026-10-23。HEAD 218312e。調査のみ。`lift/` の既存ファイル、runtime、生成器、テストは変更していない。決定は `docs/decisions/0073-lift-dsp-ippv2-as-second-module.md`（方向の決定。値は決めていない）。作業物は `lift/out-ipp/`（§11）。

## 0. 凡例と範囲

- **実測** = コマンドの出力。`lift/out-ipp/` の同名ファイルに残してある。
- **読み** = capstone による静的な逆アセンブルの読み。実行して確かめていない。
- **推測 / 仮説** = 実測や読みから言えそうだが、確認していないもの。
- **未確認** = 確認できていない。
- 入力は `lift/out-ext/dsp_ippv2_x64.dll`（39,942,656 B。SHA-256 は external-modules-usage.md §1.1）。ツールは既存の `fe-pe`、`fe-decode`、`fe-lift` を、`lift/out-ipp/target`（`CARGO_TARGET_DIR`）に release ビルドして使った（既存の `lift/target` には触れていない）。
- 作業用の補助ツール（Python と Rust）は読み取り専用で、`lift/out-ipp/` に置いた。**`merged-text.dll` と `merged-rebased-*.dll` は実在の DLL ではなく、測定用に作った改変コピー**で、実機の DLL の代わりにならない（§2.4、§3.1）。
- 依頼文にあった「アリーナ 0x2100000〜0x3100000」は、`docs/` と `lift/runtime`、`lift/harness` を grep しても該当の定義が見つからなかった（`2100000` と `3100000` の grep は無関係な逆アセンブルの行だけ）。実際のアリーナは §2.2 のとおり、ホストのヒープから `aligned_alloc` で取る 512 MiB で、固定アドレスではない。

## 1. 要約

| 項目 | 結果 | 区分 |
|---|---|---|
| 判定 | **条件付き可能**（§8）。不可能とする根拠は見つからなかった | 実測と読みからの判断 |
| アドレス空間 | ImageBase 0x180000000 は wasm32 に置けない。`.reloc` を適用して低い基底に置けば、既存の fe-pe / fe-decode / fe-lift は変更なしで通る（基底 0x2000000、0x4000000、0x8000000 で閉包が同一） | 実測（§2.3） |
| コード規模 | 関数 32,162、命令 6,458,203、誤デコード 0。生成 C は 565,582,720 B（FLEngine の 2.62 倍）。ただし**未変更の fe-decode は `IPPCODE` を見ない**（関数 25,401、命令 4,855,751、誤デコード相当 35、範囲外呼び 2,429） | 実測（§3） |
| 未対応命令 | 生成 C の 15.6% が `fle_unsupported`（1,011,928 / 6,504,638）。FLEngine は 531 件。内訳は AVX/AVX2/AVX-512 系が大半 | 実測（§4） |
| CPU 振り分け | DLL は CPUID から機能マスクを作り、434 個のスタブが `jmp [表 + 選択値*8]` で実装を選ぶ。現行の CPUID 応答では選択値は 0（最下位）になるはず | 読み（§5。仮説を含む） |
| インポート | 応答表に無い 35 名のうち 28 名が DllMain の静的な到達範囲にある。さらに `call [IAT]` 形が 138 か所あり、現行の生成器は未対応 | 実測と読み（§6） |
| 一番大きい作業 | (1) 命令（特に AVX 系）の扱いの方針、(2) kernel32 の追加（35 名）、(3) 2 モジュールの生成とリンク、(4) `IPPCODE` を含めた fe-decode の変更 | 推測（§9） |

## 2. アドレス空間

### 2.1 現行の生成器と runtime が「VA = リニアアドレス」を仮定している箇所（読み）

- 生成 C の命令はメモリ参照を `fle_ld(アドレス, n)` と `fle_st(…)` で書き、アドレスは 64 ビットの C 式である。rip 相対は `UINT64_C(<VA>)` の即値になる（`fe-lift/src/lib.rs` の `ea`、205 行目）。`fle_ld` はアドレスをそのままホストのポインタに変換する（`fle_runtime.h` の `memcpy(&v, (const void *)fle_addr(a, n, 0), n)`）。つまりゲストの VA は、そのままホストのアドレス（wasm ではリニアメモリのオフセット）である。
- wasm32 では `fle_addr(a, n, store)` が `a >> 32` が非ゼロなら `fle_addr_fault`（`[fle_addr_hi32]`）で `abort()` する（決定 0045 d、0049）。native の既定ビルドでは検査なしの `(uintptr_t)(a)`。
- `FLE_CHECKED_MEM`（任意、既定オフ）は、アクセスが「単一のイメージ範囲 [fle_mem_img_lo, +fle_mem_img_size)」または「`__global_base` 以上」に無ければ止める（`fle_runtime.h` 50 行目、`fle_host.c` 13〜19 行目。どちらも `uint32_t` で、既定値は FLEngine の 0x400000 と 0x18c0000）。**範囲が 1 つしか持てない**。
- `harness/image.c` は PE を `ImageBase` にそのまま置く。wasm では `mmap` の代わりに `m = (void *)ibase; memset(m, 0, size)` で、`ibase + size > __global_base` なら失敗する（`image.c` 91〜99 行目）。ImageBase が 4 GiB 以上だと、この前提が成り立たない。
- 生成 C の関数名は `f_<VA の 16 進>`、`fle_dispatch_table` は VA で 2 分探索する（`fle_dispatch.h`）。応答表の擬似 VA は 0x7FF9_0000_0000 から（`imports_table.rs` の `RESP_VA_BASE`）。
- runtime の多くの箇所が「ゲスト自身のモジュール」を `FLE_EXE_HANDLE` = 0x400000 の 1 つだけとして扱う（`fle_class.c`、`fle_res.c`、`fle_vfs.c`、`fle_imagehlp.c`、`fle_win.c`、`fle_load.c`、`fle_modules.c` など。`FLE_EXE_HANDLE` の grep で 8 ファイル）。`GetProcAddress` は `hModule == FLE_EXE_HANDLE` を「ゲスト自身は未対応」として止める（`fle_modules.c` 153〜156 行目）。

### 2.2 既存の領域（実測と読み）

| 領域 | 範囲 | 出典 |
|---|---|---|
| FLEngine のイメージ | 0x400000 〜 0x1cc0000（ImageBase 0x400000、SizeOfImage 0x18c0000） | `image.c`、`fle_host.c`、決定 0045 e |
| `GLOBAL_BASE`（モジュールのデータ領域の先頭） | 0x10000000（256 MiB）。`build.sh` の `-sGLOBAL_BASE`。この下にイメージが入る | `harness/wasm/build.sh` 35 行目 |
| 初期メモリ / 最大メモリ | 640 MiB / 2 GiB（`--initial-memory`、`--max-memory`）。**現行の `flengine.wasm` のメモリセクションを直接読んで、min 10240 ページ（640 MiB）、max 32768 ページ（2048 MiB）と実測** | `out-wasm2/bin-base/flengine.wasm` |
| 仮想メモリのアリーナ | 512 MiB（`FLE_VMEM_ARENA_BYTES`）。ホストのヒープから最初の使用時に `aligned_alloc(65536)` で 1 つ取る。**固定アドレスではない**（`GLOBAL_BASE` より上のどこか） | `fle_sys.c` 308〜338 行目、決定 0045 c |
| ゲストの 4 KiB ページ / 割り当て粒度 | `FLE_PAGE_SIZE` 4096 / `FLE_ALLOC_GRANULARITY` 65536 | `fle_sys.c` |
| `FLE_MIN_APP_ADDR` / `FLE_MAX_APP_ADDR` | 0x10000 / 0x7FFFFFFEFFFF | `fle_sys.h` 33〜34 行目 |
| 仮想ハンドルの領域（実際のメモリではなくトークン） | 0x7F990000（ウィンドウ）、0x7FA90000（スクリーン DC）、0x7FBA0000（GDI）、0x7FCA0000（標準カーソル）、0x7FDA0000（DLL のカーソルとアイコン）、0x7FEA0000（モニタ）。各 1 MiB。上位の領域: 0x7FF700000000（プロセスヒープ）、0x7FF800000000 + 0x10000×N（モジュール）、0x7FF900000000 + 0x10×k（応答表の擬似 VA）、0x7FFA… の各領域 | 各 `fle_*.h` |
| TEB の代用 | `fle_teb`（runtime の静的変数。`fle_vex.h` 98 行目）。データ領域に入る | `fle_vex.h` |

上位 32 ビットを持つトークン（0x7FF7… など）は、ゲストがメモリとして読み書きしない前提のもので、wasm32 のアドレスとは衝突しない。0x7F99xxxx〜0x7FEAxxxx の 6 領域は 4 GiB 未満の数値だが、**現行の最大メモリ 2 GiB（0x80000000）より下にある**。これらはトークンでデリファレンスしない設計（各ヘッダのコメント）なので、リニアメモリの実アドレスと重なるかどうかは、メモリがそこまで伸びたときの問題である。2 GiB のメモリは `GLOBAL_BASE`（256 MiB）より上をヒープ、アリーナ、スタックが使う。実際にメモリがどこまで伸びるか（使用量の実測）は未確認（`mem_probe.cjs` は `HEAPU8` が export されておらず取れなかった）。

### 2.3 リベース方式の測定（実測）

方式 (i): `.reloc` を適用して、DLL を低い基底に置き直したうえで、そのまま lift する。

- 再配置の内訳（`reloc-scan.txt`、`ipp_reloc_scan.py`）: `IMAGE_REL_BASED_DIR64` が 15,602 件、ABSOLUTE（パディング）22 件。**HIGHLOW など他の種別は 0 件**。位置の内訳は `.data` 6,163、`.rdata` 8,983、`.text` 456。**`IPPCODE` と、`.text` の 456 件以外のコードに再配置は無い**（コードは rip 相対で位置独立）。ポインタの先: `.data` → `.text` 5,488、`.data` → `IPPCODE` 634、`.rdata` → `.text` 8,133、`.rdata` → `IPPCODE` 150、`.rdata` → `.rdata` 696 ほか。`.text` 内の 456 件はすべて `.text` を指す（`.text` 内に置かれた 57 個の CPU 振り分けの表（§5.2）のスロットだと読むが、456 という数と表のエントリ数（7）が単純に割り切れないので、対応は未確認）。
- `ipp_rebase.py`: ファイルのバイト列に、`.reloc` の DIR64 の位置へ（新しい基底 − 古い基底）を足し、ヘッダの ImageBase を書き換えた改変コピーを作る（Windows のローダが基底を変えるときの処理）。適用は 15,602 件すべてがファイルに裏付けのある位置だった（裏付けなし 0 件）。
- 結果（`closure-rebased.txt`、`closure-rebased8.txt`、`lift-rebased-4000000.txt`）:
  - 基底 0x2000000 と 0x8000000 で `fe-decode closure --pdata --adjustors` の出力が、ヘッダ行（アドレスの値）を除いて、基底 0x180000000 のものと**同一**（`diff` で差なし）。
  - 基底 0x4000000 で `fe-lift lift --pdata` を実行すると、統計（関数数、命令数、未対応数の全体）が基底 0x180000000 のものと同一。生成 C は 535,995,178 B（基底 0x180000000 のものは 565,582,720 B で、即値の桁が短くなったぶん小さい）。
  - 生成 C の中の `UINT64_C(0x18xxxxxxx)` 形の即値は、リベースなしで 1,173,132 か所、基底 0x4000000 では 0 か所。**リベースなしの生成 C は、そのままだと `fle_addr` の上位 32 ビット検査で止まる VA を 117 万か所持つ**。
- つまり「**ファイルの段階でリベースしてから lift する**」方式は、現行の 3 つの crate を変更せずに通る（ImageBase は `fe-pe` の `rva_to_va` / `va_to_rva` がヘッダの値を引くだけで、`fe-pe`、`fe-decode`、`fe-lift` の `src/` に 0x400000 や 0x180000000 のハードコードは見つからなかった。`image_base` と `0x180000000` の grep の結果。0x400000 は `imports_table.rs` のコメントにだけある。`jumptab::FLENGINE`（FLEngine 用の注釈つき jmp 表）は各 spec をバイト列で照合するので、この DLL には影響しないと読める）。
- **注意**: 上は、改変コピーでの測定である。実際の方式にするには、(a) ビルド時にリベースしたファイルを作る（fe-lift の前段にリベース処理を足すか、別ツール）か、(b) 実行時のローダが同じ処理をするか、の選択が要る。(a) の場合、`harness/image.c` は ImageBase を読んで置くので、リベース済みのファイルを渡せば同じ基底に置ける（image.c の改造は要らない）が、**1 つのハーネスで 2 つの PE を置く改造は要る**（§9 W1）。

### 2.4 リベース先の候補範囲（案。値は決めない）

必要サイズは SizeOfImage = 0x261f000 = 39,972,864 B（約 38.1 MiB。依頼文の「約 39.5MB」は 10 進の 39.97 MB に近い）。

- **現行のリンク設定のまま置ける範囲**: FLEngine のイメージの終わり（0x1cc0000）から `GLOBAL_BASE`（0x10000000）までの 0xe340000 B（約 227 MiB）。ここに 38.1 MiB が入る。ページ境界（4 KiB）と、DLL の SectionAlignment 0x1000 に合わせる。
- 候補の例（案）: 基底 0x2000000（終わり 0x461f000）、基底 0x8000000（終わり 0xa61f000。ここまでで閉包の同一性を実測した）。どちらも FLEngine のイメージ（〜0x1cc0000）、`GLOBAL_BASE`（0x10000000）、既存のトークン領域（0x7F990000 以上）と重ならない。
- **`GLOBAL_BASE` を上げる**（例: 512 MiB）案もある。FLEngine と DLL の 2 つのイメージ（合計 約 63 MiB）の余裕が増える。リンク設定の変更で、生成 C の変更は要らない。
- 範囲が重ならないことの検査: 既存の `FLE_CHECKED_MEM` は範囲が 1 つ（§2.1）なので、2 つのイメージを許すには `fle_mem_set_image` を 2 範囲に拡張する必要がある。
- **メモリの上限**: 現行の最大 2 GiB のうち、アリーナ 512 MiB と、wasm のコードを除くデータを足したものにイメージ 38.1 MiB が加わる。初期メモリ 640 MiB（アリーナ込み）に対して、イメージは `GLOBAL_BASE` より下に置くので、初期メモリの内側にある（0x10000000 = 256 MiB 以下）。追加のヒープは `ALLOW_MEMORY_GROWTH` で足りる見込み（読み。使用量の実測は未確認）。
- 方式 (ii): 他の方式。
  - **wasm64（memory64）にして ImageBase のまま置く**: 実測の事実としては、Chrome 133、Firefox 134、Edge 133 が対応済みで、**Safari は未対応**（web-features-explorer と caniuse の検索結果。2026-10-23 取得。Safari の Technology Preview の対応は caniuse の別の表に載っているが、正式版は未対応）。生成 C と runtime の `uintptr_t` の仮定（`long` の幅、`size_t` の変換。決定 0049 c）が広範囲に影響する。決定 0045 の最終目標は「ブラウザ」なので、Safari の扱いは未決。**採るなら大きな変更**（推測）。
  - **lowbase 方式（アドレス変換）**: 決定 0045 の代替案が「旧実績の lowbase 方式」を挙げ、`fle_ld` / `fle_st` で `LVA = RVA + GLOBAL_BASE` のように 32 ビットに写す。生成 C の再生成か `fle_ld` の変換が要り、ゲストが 64 ビットのポインタを値として持つこと（VirtualAlloc の戻りなど）と衝突する、というのが 0045 の理由。DLL の `call [rax]` 経由の関数ポインタや、`.data` の絶対ポインタ表（5,488 + 634 件など）を変換する必要が出る。**リベース（i）の方が変更が小さい**（推測）。
  - **イメージを分割して複数の低い領域に置く**: セクションごとに別の基底に置くことは、rip 相対の距離が変わり成り立たない（コードと `.data` の距離が rip 相対に焼き込まれている）。却下する理由が確認できる。

### 2.5 リベースに伴って必要になる変更点（読み）

1. ファイル段階のリベース処理（`.reloc` の DIR64 を適用する。上の `ipp_rebase.py` が 30 行ほどの参考）。HIGHLOW が無いので DIR64 だけでよい（この DLL に限る）。
2. `image.c` の 2 イメージ対応（現行は静的変数 `imports[]`、`exports[]`、`nimports` を 1 つの PE で使う。2 回目の呼び出しは同じ表に足されるので、IAT の疑似 VA の対応表を共用する形になる）。
3. `fle_mem_set_image` の 2 範囲化（`FLE_CHECKED_MEM` を使う場合）。
4. `FLE_EXE_HANDLE`（0x400000）の「ゲスト自身」の意味は FLEngine 用のまま残し、DLL 用のハンドル（リベース後の基底、またはモジュール表の値）を別に持つ。
5. 生成 C の `f_<VA>` が FLEngine と衝突しないこと（VA が重ならなければ衝突しない）。`R[16]`、`FL`、`X[16]`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_table` は、生成 C が各 TU の先頭で定義する（`lib.rs` 770 行目。`split_c.py` は part-00 だけが定義を持ち、他は `extern` にする）。**2 回目の生成 C をそのまま足すと、これらの定義が重複してリンクできない**（読み。リンクして確かめてはいない）。

## 3. コード規模

### 3.1 そのまま実行した結果と、改変コピーでの結果（実測）

手順は README.md のとおり（`fe-lift lift FILE OUT.c --pdata`、`fe-decode closure FILE --mnemonics --pdata --adjustors`）。DLL をそのまま入力にできた（PE32+ x64 を `fe-pe` が読み、エラーなし）。**ただし `fe-decode` の `Text::from_pe` は名前が `.text` のセクションだけを命令の範囲として扱う**（`fe-decode/src/lib.rs` 42〜52 行目）。この DLL はコードが 2 つのセクション（`.text` 0x17de140 B と `IPPCODE` 0x6dc771 B）に分かれているので、そのままでは `IPPCODE` が「範囲外」になる。

| 項目 | そのまま（`.text` のみ） | 改変コピー（`.text` と `IPPCODE` を 1 つの `.text` にしたもの） | FLEngine（参考。同じ手順） |
|---|---|---|---|
| 入力 | `dsp_ippv2_x64.dll` | `merged-text.dll`（RVA と ImageBase は不変。`ipp_merge_text.py`。実在の DLL ではない） | `FLEngine_x64.dll` |
| 関数 | 25,401 | 32,162 | 59,297 |
| 命令（閉包） | 4,855,751 | 6,458,203 | 2,528,030 |
| 誤デコード（`decode_failures`） | 0 | 0 | 0 |
| 範囲外の流れ / 範囲外の呼び | 35 / 2,429（distinct の呼び先。呼び出し点は 10,168 か所） | 0 / 0 | — |
| 生成後の `bad_nodes` | 35 | 0 | 0 |
| 生成 C | 432,619,203 B | 565,582,720 B（6,698,311 行） | 215,700,923 B（2,885,313 行） |
| `emitted_instructions` | 4,902,066 | 6,504,638 | 2,528,830 |
| 未対応（`unsupported_total`） | 898,560 | 1,011,928（15.6%） | 531（0.02%） |
| `fe-lift` の所要時間 | 5.8 s | 8.0 s | 3.0 s |

- `.pdata` は 50,529 エントリ、チェーンされたもの 19,713、関数開始 30,816（`fe-pe pe-pdata-info`。external-modules-usage.md §1.1 と同じ）。閉包の関数 32,162 は、`.pdata` の開始に、エントリ、エクスポート、直接 call の目標（`.pdata` に無いもの）を足したもの。
- そのままの結果の「誤デコード」はゼロだが、「範囲外の呼び 2,429」は `IPPCODE` への呼びである。したがって**このツールを DLL にそのまま使うと、`IPPCODE` の 6,000 個ほどの関数が、生成 C に名前だけ（`fle_unsupported`）で入る**。変更点: `Text::from_pe` を複数のコードセクションに対応させる（§9 W3）。改変コピーはこの変更の代わりに作った測定用のもの。
- 閉包の全体: 基本ブロック 634,879、直接 call 45,219、直接 jmp 89,354、条件分岐 360,911、ret 32,773、IAT 経由の call 138（`import_calls`）と jmp 19、`int3` などの割り込み 52、distinct のニーモニック 709（`closure-merged.txt`）。
- 命令あたりの生成 C は 86.95 B（DLL）と 85.30 B（FLEngine）で、ほぼ同じ。

### 3.2 命令の符号化（実測。`ipp-encscan` による閉包の分類）

| 符号化 | 命令数 | 関数数（その符号化を 1 つでも含むもの） |
|---|---|---|
| レガシー（SSE/SSE2 まで、整数、x87 を含む） | 5,404,412 | 25,481（VEX も EVEX も含まない関数） |
| VEX（AVX、AVX2、FMA、BMI を含む） | 834,972 | 5,736（EVEX を含まない） |
| EVEX（AVX-512） | 218,819 | 945 |

- ZMM レジスタを使う命令: 139,433。オペマスク（k レジスタまたは `{k}`）を使う命令: 28,656。BMI / LZCNT 系（`shlx`、`shrx`、`sarx`、`andn`、`blsr`、`mulx`、`rorx`、`lzcnt`）: 1,928。distinct のニーモニック: VEX 287、EVEX 286。
- **AVX、AVX2、AVX-512 の命令は大量にある**（全命令の約 16%）。FLEngine の閉包には、この種の命令は `vmovups`/`vmovaps`/`vmovntps`/`vzeroupper` だけが入っていた（決定 0032。生成器が扱うのもその 4 種だけ。`vex.rs`）。
- 「関数が VEX を含む」ことは「AVX の実装系統である」ことを意味しない。VEX を含む 5,736 関数のうち、CPU 振り分けの表（§5）の指す関数と、表と無関係に VEX を使う関数（例: `vmovss` と `vmovd` を使う `0x3fafa0` は、表を介さない直接 call の目標。§5.4）が混ざる。

### 3.3 wasm への影響（見積もりは推測と明記する）

- 実測の基準: 現行 `flengine.wasm` は 115,721,618 B（`out-wasm2/bin-base/`）。生成 C 215,539,555 B（決定 0045 の測定時）に対して 0.537 倍。リンクは 1.2 s、最大 RSS 635 MB（`log-base/link.time`）。生成 C の 16 TU の `emcc -O0` は、合計 78.2 s、最大の 1 TU が 6.63 s と 1.8 GB（`log-base/gen-*.log.time` を合算した実測）。
- **推測**: 同じ比で、DLL（改変コピーの閉包）の生成 C 565.6 MB は wasm で約 303 MB。FLEngine と合わせて約 420 MB。コンパイルは 16 TU の 2.6 倍、合計で 200 s 前後。`fle_unsupported(va)` の呼び出しは小さい（1 か所あたり数命令）ので、実際の比はこれより低い可能性がある。逆に AVX 系を実装して命令を展開すれば高くなる。
- node（v24.19.0）の既定の wasm モジュールの上限は `--wasm-max-module-size=1073741824`（1 GiB。`node --v8-options` の出力）。約 420 MB はこの内側（推測の大きさでの話）。**ブラウザの上限は確認していない**（未確認）。
- コンパイルと配布の大きさを減らす方法は、§5.5 の「到達しない系統を生成しない」。測定（§5.5）では、SSE2 系統だけに絞ると生成 C が 42.9 MB（全体の 7.6%）まで減る。

## 4. 未対応命令（実測）

`fe-lift` は未対応の命令を `fle_unsupported(va)` に置き換える。到達すると名前付きで停止する（abort）。数は「生成 C に出た `fle_unsupported` の箇所」で、実行される回数ではない。集計は `lift-merged.txt`（`ipp_unsup_summary.py`）。

| 理由 | 箇所数 | distinct のキー数 |
|---|---|---|
| `V*` ニーモニック（VEX または EVEX）が最小集合の外 | 885,938 | 361 |
| `V*` 以外のレガシー命令が最小集合の外 | 73,713 | 82 |
| EVEX 符号化（`vmovups`、`vmovaps` のみが対象で、EVEX 版は不可） | 46,975 | 2 |
| `K*`（AVX-512 のオペマスク命令） | 4,537 | 16 |
| 間接 jmp の目標が未解決（`jmp [表 + 添字*8]`） | 617 | 1 |
| IAT 経由の間接 call / jmp（未対応） | 145（call 138、jmp 7） | 2 |
| `gs:` のオフセットが未モデル（0x30 と 0x58 以外） | 3 | 1 |
| 合計 | 1,011,928 | 465 |

- 上位のニーモニック（箇所数）: `Vmovsd` 47,290、`Vmovss` 37,654、`Vmovupd` 37,374、`Vaddps` 33,868、`Vmovups`（EVEX）33,689、`Vaddpd` 33,295、`Vmulps` 30,371、`Vaddsd` 28,998、`Vmulpd` 28,649、`Vsubpd` 23,096。
- レガシー側（SSE3 以降、SSSE3、SSE4 など）の未対応の上位: `Movddup` 9,167、`Pmaddwd` 7,490、`Psrldq` 5,373、`Pinsrw` 5,160、`Addsubpd` 4,916、`Lddqu` 3,760、`Pshufb` 3,573、`Pslldq` 3,143、`Orps` 2,932、`Pmuludq` 2,689、`Movmskps` 2,267、…（82 種。`unsup-merged.txt`）。BMI の `Shlx` 1,239、`Shrx` 599、`Sarx` 112、`Andn` 65、`Popcnt` 324 も含む。
- `fle_unsupported` に入らない特殊な扱い: `cpuid` 259 か所（`fle_cpuid` を呼ぶ）、`xgetbv` 7 か所（`fle_xgetbv_stop`。到達すると名前付きで停止。決定 0047）、`stmxcsr`/`ldmxcsr`、`fldcw`/`fnstcw`/`fnclex` は対応済み（`handled` の計数）。
- **間接 jmp の目標が未解決の 617 か所**は、§5.2 の CPU 振り分けスタブ 434 個を含む（生成 C の `Jmp ... fle_unsupported` のうち、スタブ形に一致する 434 か所がすべて未解決の集合に入っていることを確認。残り 183 か所は別の形で、未調査）。現行の生成器は `jmp [メモリ]` の目標が未解決のとき止まる実装で（`tailjmp.rs` はレジスタ形 `jmp r64` だけを `fle_dispatch` の末尾呼びにする）、**この DLL の公開関数の多くが入口でこのスタブを通るので、そのままではロードできても、IPP 関数を 1 つ呼んだ時点で止まる**（読み。実行していない）。
- **FLEngine との違い**: FLEngine の未対応は 531 件（IAT 経由の jmp 530 と未解決 1）。この DLL は 3 桁多い。未対応の約 88% が AVX/AVX2/AVX-512 系の `V*` 命令（実測の割合）。
- そのままの（`.text` のみの）生成の未対応: 898,560。差は `IPPCODE` の分（§3.1）。

## 5. IPPCODE の CPU 振り分け

**この節は、DLL の機械語の静的な読み（capstone）と、生成器による数え上げが根拠で、実行して確かめたものではない。** 選ばれる系統は仮説。

### 5.1 CPUID の呼び出し点（読み。実測の数）

- 閉包の `cpuid` は 259 か所（`lift-merged.txt` の `handled`）。`xgetbv` は 7 か所（生成 C の `fle_xgetbv_stop` の呼び出し 7 か所）: RVA 0x17cb93e、0x17d4fa7、0x17e0138、0x17e0168、0x17e0a20、0x1b31420、0x1cba0e0。**最初の 5 つは読んだ。どれも `xgetbv` の前に CPUID 葉 1 の ECX を見て（OSXSAVE=bit 27、AVX=bit 28 など）、立っていなければ飛ばす形**（0x17cb936/0x17cb93a の `bt edi, 0x1b/0x1c`、0x17d4fa3 の `cmp ecx, eax`、0x17e0134、0x17e0164、0x17e0a16。決定 0032、0047 が FLEngine で読んだ形と同じ）。後ろの 2 つ（0x1b31420、0x1cba0e0）は読んでいない（未確認）。
- DllMain から直接 call でたどれる範囲（230 関数。`ipp_reach.py`）にある CPUID の関数: 0x17cb820（MSVC の CRT の ISA 判定に見える。葉 0、葉 1、葉 7 を読む）、0x17d4f70（葉 1 と xgetbv。保存先 0x2561bf0）、0x17e011e / 0x17e014e（AVX と AVX-512 の OS 対応の判定。xgetbv の結果を `&6` と `&0xe0` で見る）、0x17e0388（ベンダー文字列が "GenuineIntel" か。EBX=`Genu`、EDX=`ineI`、ECX=`ntel` を比べて 1 か 0 を返す）、0x17e0890（任意の葉とサブ葉を取る汎用の関数）。
- `0x17e0000` 以降の IPPCODE の先頭は、1 つの機能ビットを返す小さい関数が並ぶ（MMX=EDX bit 23、SSE=EDX bit 25、SSE2=EDX bit 26、SSE3=ECX bit 0、SSSE3=ECX bit 9、SSE4.1=ECX bit 19、SSE4.2=ECX bit 20、MOVBE=ECX bit 22 など。読み）。

### 5.2 機能マスクから「選択値」を作り、表で実装を選ぶ構造（読み）

1. **機能マスクの生成**（0x13c0）: CPUID の葉 0（最大の基本葉）、葉 0x80000000（最大の拡張葉）、葉 1、葉 7（最大の基本葉が 7 以上のときだけ）、葉 0x80000001（最大の拡張葉が 0x80000001 以上のときだけ）から、ビットのマスクを作る。ビットの意味（読み）: bit 0 MMX、bit 1 SSE、bit 2 SSE2、bit 3 SSE3、bit 4 SSSE3、bit 6 SSE4.1、bit 7 SSE4.2、bit 8 AVX（CPUID の ECX bit 28）、bit 9 AVX の OS 対応（0x17e011e）、ほか上位ビットは葉 7 の AVX2、AVX-512 系。結果は `[0x2560ec0]` にキャッシュする（`[0x2560ec8]` が初期化済みフラグ）。
2. **選択値の決定**（0x10c0。0x1040 から呼ばれる）: マスクの上位から順に調べ、`0x7300000` の全ビットが立てば選択値 5、`0xf00000` なら 4、bit 15 なら 4、bit 8 なら 3、bit 7 なら 2、bit 6/5/4 なら 1、bit 3/2 なら 0、それ以外も 0（状態コード 0x24 を付ける）。結果を `[0x2541000]`（`dword`）に格納する。**選択値は 0〜5 の 6 通り**（読み）。
3. **表による実装の選択**: 434 個のスタブ（バイト列 `movsxd rax, [rip+選択値] ; lea r10, [rip+表] ; jmp [r10+rax*8]`、`ipp_stubs_scan.py` が検出）は、選択値で `表 + 選択値*8` のポインタを引いて jmp する。スタブごとに表が 1 つ（434 個の表）。**選択値の変数は 4 つ**: `0x2541000`（377 スタブ）、`0x255d050`（33）、`0x25410a0`（20）、`0x254f270`（4）（実測）。表のエントリ数は 6（376 表）、7（57 表）、9（1 表）（表は背中合わせに並ぶので、次の表の開始までで数えた値）。377 表は `.data`、57 表は `.text` にある。`.text` の再配置 456 件と 57 表の対応は未確認（§2.3）。
4. **遅延初期化**: 選択値の変数はファイル上の初期値が `-1`（4 つとも `0xffffffff`。ファイルの値を読んだ）。表の「添字 −1」の位置には、各スタブの手前にある `call 0x1839810`（`0x1040` を呼ぶ）の位置が入っている（`.data` の表のうち、`entry[-1]` の値を数えたもの。0x1840 と 0x1870 など）。したがって、初期化前に最初に呼ばれると、`0x1040` を呼んでから通常のスタブに落ちる形に読める（読み。実行していない）。**DllMain の `DLL_PROCESS_ATTACH` からも `0x1040` が呼ばれる**（`0x17c85b0` が `cmp edx,1` のときに呼ぶ。戻り値は使わない）。

### 5.3 表の各添字が指す実装（実測）

`ipp-tables` が、閉包の関数について、直接 call / jmp をたどって到達する命令の符号化を分類した（添字ごと。`tables-class.txt`）。

| 添字 | 指す distinct の関数 | EVEX に到達 | VEX に到達（EVEX なし） | VEX も EVEX も到達しない |
|---|---|---|---|---|
| 0 | 354 | 0 | 15 | 345 |
| 1 | 351 | 0 | 7 | 350 |
| 2 | 353 | 0 | 7 | 352 |
| 3 | 343 | 6 | 261 | 83 |
| 4 | 332 | 1 | 240 | 98 |
| 5 | 326 | 113 | 165 | 55 |
| 6〜8 | 57、1、1 | 0 | 52 | 6 |

（数字は「表のエントリ」の数。表ごとに別の関数を指すので、合計は 434 を超える。）

- 添字 0 と 1 と 2 は、主にレガシー命令の実装を指す。添字 3 は AVX、4 は AVX2 または FMA、5 は AVX-512 に当たる系統が多い、という並びが、選択値の決定（§5.2 の 2）と一致する。**添字と命令集合の対応は、読みと上の分類の合致から言えることで、IPP の文書から確認したものではない**。
- 添字の位置に関数のエントリでない値を持つ表エントリが 507 件ある（添字 0〜5 で 74〜101 件ずつ）。うち 501 件は「閉包の命令として復号されていない」アドレス（多くが `0x1800018a0`。上の 6 エントリ表に共通して現れる値。`jmp` で別関数に飛ぶ小さな入口で、`.pdata` に無いために関数として数えられていない形。読み）。**これらは、生成器が `.pdata` の開始に加えて、再配置で指される値も種に加えれば関数になる**（§5.5、§9 W3）。

### 5.4 現行の CPUID 応答で選ばれそうな系統（仮説）

現行の応答（決定 0009、0047）: 最大の基本葉 1、葉 0 のベンダー文字列は 0、葉 1 の EAX と EBX は 0、ECX は 0、EDX は bit 25（SSE）と bit 26（SSE2）のみ、葉 2〜0x7fffffff は葉 1 と同じデータ、0x80000000 以上は 0。

- 機能マスク: bit 1（SSE）と bit 2（SSE2）だけが立つ（MMX はなし。EDX の bit 23 が 0）。葉 7 は「最大の基本葉 < 7」で読まれず、拡張葉も 0 なので読まれない。→ 選択値は **0**（`test r12, 4` のあたり）。**仮説**。
- `0x17e0388`（Intel かの判定）は 0 を返す（ベンダー文字列が 0）。0x1040 はこのとき戻り値に 0x14 を入れる（意味は未確認）。DllMain の呼び出し元は戻り値を使わない。
- `0x17cb820`（CRT の ISA 判定）: ECX = 0 なので、AVX 以降の分岐に入らない（`bt edi, 0x14`（SSE4.2）で飛ぶ）。`xgetbv` は実行されない（読み）。
- **したがって、現行の応答のまま動かすと、選択値は 0 になり、表の添字 0（レガシー命令の実装）が選ばれる**、というのが仮説。**添字 0 でも 345 エントリは VEX にも EVEX にも到達しないが、15 エントリは直接 call で VEX 命令を含む関数に到達する**（実測の分類）。それらは、表を介さない直接 call で AVX 命令の関数に行く、レガシー版の中の経路である。到達の確認（実行）は未確認。
- 表を介さず、直接 call や jmp で VEX / EVEX の関数に行く辺: VEX も EVEX も自分では持たない関数から、持つ関数への直接 call / jmp が 1,631 本（VEX へ 1,347、EVEX へ 284。起点の関数は 1,096）。クラス分けでは、全 32,162 関数のうち、直接の call / jmp の推移閉包で VEX にも EVEX にも到達しないのは 24,186、VEX のみ 6,771、EVEX に到達 1,205（`tables-class.txt`）。

### 5.5 AVX / AVX-512 系の関数を呼ぶ経路に到達しない設計にできるか

- **CPUID の応答をそのままにして、生成する関数を絞る**案（測定した）: 種を「エントリ、エクスポート、表の添字 0〜K のエントリ」だけにし、`.pdata` の全開始を種に入れない。`fe-lift lift ... --seeds FILE`（`--pdata` なし）。K までの添字以外を選んだとき、`fle_dispatch` が `fle_dispatch_miss` で名前付きに止まる（読み。`fle_dispatch.h`）。

| 種 | 関数 | 命令 | 生成 C | 未対応 | 未対応の内訳 |
|---|---|---|---|---|---|
| 添字 0 のみ（428 個の VA。`table-seeds-idx0.txt`） | 1,891 | 476,308 | 42,866,378 B | 4,938 | レガシー 4,074（35 種）、間接 jmp 未解決 464、V* 275、IAT 122、gs 3 |
| 添字 0〜2（1,273 個の VA） | 4,176 | 1,380,184 | 130,549,823 B | 18,347 | レガシー 17,423（63 種）、未解決 524、V* 275、IAT 122、gs 3 |
| 全体（比較） | 32,162 | 6,458,203 | 565,582,720 B | 1,011,928 | §4 |

- 添字 0 のみの閉包には、**V* 命令が 275 箇所残る**（§5.4 の「添字 0 でも VEX に到達する 15 エントリ」と、エクスポートや CRT の関数の VEX 命令。どの関数のどの命令かは未調査）。AVX を実装しなくても済むかは、この 275 箇所が実行時に到達するかによる（未確認）。
- この絞り込みは「生成する」範囲の話で、**選択値が 0 になること（仮説）が崩れると、`fle_dispatch_miss` で止まる**。選択値を 0 に固定する設計（CPUID を変えない）を選ぶなら、ベンダーや機能ビットを返す決定（0009）と整合する（SSE2 まで）。
- **AVX 系を主張する設計**（CPUID の ECX の bit 27、28 を立てる）は、決定 0047 の条件のとおり XGETBV の値の決定が先に要る。また、AVX 以降の約 90 万命令の扱いを決める必要がある（実装するか、`fle_unsupported` のまま到達しないことに賭けるか）。
- **EVEX の `vmovups`/`vmovaps` 33,689 + 13,286 箇所**は、現行の `vex.rs` が「EVEX は不可」として返す（`EVEX encoding`）。添字 5 の系統の中にある。

## 6. インポート（KERNEL32 72 名）

`fe-lift` の応答表（`imports_table.rs` の `TABLE`）に行がある名前は 37、無い名前は 35（`ipp_imports_scan.py`。external-modules-usage.md §1.1 の数と一致。実測）。応答表に行がある 37 名: CreateFileW、GetProcessHeap、LCMapStringW、QueryPerformanceCounter、GetCurrentProcess、TerminateProcess、GetCurrentProcessId、GetCurrentThreadId、GetSystemTimeAsFileTime、GetStartupInfoW、GetModuleHandleW、RaiseException、GetLastError、SetLastError、Enter/Leave/DeleteCriticalSection、TlsAlloc/TlsGetValue/TlsSetValue、FreeLibrary、GetProcAddress、LoadLibraryExW、ExitProcess、GetModuleFileNameW、GetStdHandle、HeapFree、CloseHandle、HeapAlloc、WriteFile、FindClose、FindNextFileW、GetACP、GetCPInfo、GetCommandLineW、MultiByteToWideChar、WideCharToMultiByte。

### 6.1 応答表に無い 35 名と、到達の見込み（読み）

「ipp_chain」は DllMain（0x17caca8）から直接 call / jmp だけをたどった結果、「ipp_reach」は DllMain と、DLL が IAT の近くに持つ関数ポインタ（0x1ebd248〜0x1ebd2a0 のスロットが指す 0x17cb228、0x17de9b0、0x17de9d0、0x17d2aa4、0x17d4f70、0x17d9e4c、0x17da928）と、`DLL_THREAD_ATTACH/DETACH` の枝（0x17cb3a0、0x17cb3c8）を種にした結果（`chain-dllmain.txt`、`reach-dllmain-c.txt`）。いずれも**静的な到達**で、実際に呼ばれるかは未確認。「用途」は、参照する関数の周辺を読んだ範囲の推測で、未精査のものは「未精査」とした。

| 名前 | ipp_chain | ipp_reach | 用途の読み |
|---|---|---|---|
| HeapReAlloc、HeapSize | 到達せず | 到達 | CRT の `realloc` / `_msize`（0x17d9ea0、0x17d9e64） |
| GetEnvironmentStringsW、FreeEnvironmentStringsW | 到達せず | 到達 | CRT の環境ブロック（0x17d7154）。起動時に呼ばれる形は未確認 |
| GetCommandLineA | 到達せず | 到達せず | CRT（0x17d7020）。到達しない |
| FlsAlloc、FlsSetValue、FlsGetValue、FlsFree | 到達せず | 到達（Free/Set/Get）、Alloc は到達せず | CRT のスレッドごとのデータ。`ipp_chain` は参照 0 件と数えたが、`ipp-encscan` は各 1 か所と数えた。**ツール間で食い違う**（未解決。encscan の閉包の数を採る） |
| TlsFree | 到達 | 到達 | TLS の解放（0x17d0920。DllMain のデタッチ側の経路） |
| InitializeCriticalSectionAndSpinCount | 到達 | 到達 | CRT のロック初期化。**`DLL_PROCESS_ATTACH` の直線上**（0x17cb484 → 0x17cd210 → 0x17d071c → 0x17d0a04） |
| InitializeSListHead、InterlockedFlushSList | 到達せず（参照 0 と数えた） | 到達 | 用途未精査（CRT） |
| GetModuleHandleExW | 到達せず | 到達 | 0x17d1ab8（`GetProcAddress` と `FreeLibrary` も呼ぶ小さい関数） |
| GetStringTypeW、IsValidCodePage、GetOEMCP | 到達 | 到達 | CRT のロケール / コードページ（0x17d6c5c 以下の鎖。DllMain から 10 段ほど先） |
| GetConsoleMode、GetConsoleOutputCP、WriteConsoleW、SetStdHandle、GetFileType、FlushFileBuffers、SetFilePointerEx | 到達せず | 到達（SetStdHandle と GetFileType は到達せず） | CRT の標準入出力（0x17d49f8、0x17d40b4、0x17da7e4、0x17d7ecc、0x17d34b4、0x17d8908、0x17d2c38）。ゲストが `printf` 系を呼ばない限り実行されない（推測） |
| FindFirstFileExW | 到達せず | 到達 | CRT（0x17d5ecc）。用途未精査 |
| RtlCaptureContext、RtlLookupFunctionEntry、RtlVirtualUnwind、UnhandledExceptionFilter、SetUnhandledExceptionFilter、IsDebuggerPresent、IsProcessorFeaturePresent | 到達 | 到達 | 致命エラーの経路（0x17cb654。DllMain の状態異常で `mov ecx,7; call 0x17cb654` に入る形が 2 か所ある）。通常の経路では呼ばれない（推測） |
| RtlUnwindEx、RtlPcToFileHeader、EncodePointer | 到達せず | 到達せず | 例外の巻き戻し（0x17cc05c 以下）とポインタの符号化（0x17ce8f8 以下）。C++ の例外や `atexit` を使うときだけ |

- **DllMain の `DLL_PROCESS_ATTACH` の直線上で実際に呼ばれる可能性が高い未登録の API は、確実に言えるものとして `InitializeCriticalSectionAndSpinCount` 1 つだけ**（読み）。それ以外は、CRT の通常の起動が引く名前（FLS、ヒープ、環境）に見えるが、呼ばれる経路を読み切っていない（未確認）。
- 「行がある」37 名にも、**この DLL の使い方で未対応の形がある**: DLL は IAT を `call [rip+IAT]` の形で呼ぶ（`import_calls` 138）。FLEngine は `jmp [IAT]` の薄いスタブ形（`import_jumps` 822、`import_calls` 0）で、`fe-lift` が応答表で処理するのはこの jmp 形だけ（`lib.rs` 585 行目付近。`imports_table::lookup`）。`call [IAT]` は「indirect through IAT slot (not lifted)」で未対応になる（実測: 138 か所）。したがって**応答表に行があっても、この DLL からは呼べない**。生成器の変更が要る（§9 W5）。
- 追加で、API が `fle_load.c` / `fle_modules.c` に並ぶ形（`GetModuleHandleW`、`GetProcAddress`、`LoadLibraryExW`、`FreeLibrary` は既に行がある）が、ゲストの DLL を引く用途ではなく、DLL が自分で別の DLL（api-ms-win-core-fibers など。文字列は UTF-16 で見つかった）を引く用途になる。DLL が `LoadLibraryExW` で引く名前（`api-ms-win-core-datetime-l1-1-1`、`-fibers-l1-1-1`、`-file-l1-2-2`、`-localization-l1-2-1`、`-localization-obsolete-l1-2-0`、`-processthreads-l1-1-2`、`-string-l1-1-0`、`-synch-l1-2-0`、`-sysinfo-l1-2-1`、`-winrt-l1-1-0`、`-xstate-l2-1-0`、`api-ms-win-appmodel-runtime-l1-1-2`、`api-ms-win-security-systemfunctions-l1-1-0`、`api-ms-win-rtcore-ntuser-window-l1-1-0`、`ext-ms-win-ntuser-dialogbox-l1-1-0`、`ext-ms-win-ntuser-windowstation-l1-1-0`、`kernel32`、`user32`、`advapi32`）が `fle_modules[]` の名前の規則で答えられるかは、未確認（`api-ms-win-*` は表に無い。この DLL の UCRT の `__acrt_...` 系の遅延ロードに見える。**読み。どの経路が実行されるかは未確認**）。

## 7. DllMain（エントリ RVA 0x17caca8。読み）

逆アセンブルは `dllmain-*.txt`（`ipp_disasm.py`、`ipp_disasm_range.py`）。MSVC の CRT の DllMain の典型的な形。

- **0x17caca8**: `reason == 1`（`DLL_PROCESS_ATTACH`）のときだけ 0x17cb22c を呼んでから 0x17cab74 に jmp。0x17cb22c は**スタックのクッキー（`__security_init_cookie`）の初期化**: `[0x2543588]` が既定値 0x2b992ddfa232 のときだけ、`GetSystemTimeAsFileTime`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter` の値と、スタックのアドレスを XOR して作り、`[0x2543588]` と `~` を `[0x2543580]` に格納する（読み）。**実行ごとに値が変わる設計で、現行の仮想時刻、スレッド ID、プロセス ID、`QueryPerformanceCounter` の応答から決まる**（FLEngine の同じ形は既に通っている。決定 0017、0020、0056）。
- **0x17cab74**（`dllmain_dispatch`）:
  - `reason == 0`（`DLL_PROCESS_DETACH`）かつ `[0x2560ed4]`（アタッチの回数）が 0 以下なら、0（FALSE）を返す。
  - `reason == 1` か `2`（`THREAD_ATTACH`）: `[0x1ec6f98]` のポインタ（ファイル上は 0。`_pRawDllMain` に当たる形）が NULL なら 1 を採る。NULL でなければそれを呼ぶ。続いて 0x17ca988、0x17c85b0 を呼ぶ。どちらかが 0 を返すと、そこで終了して 0 を返す。
  - `reason == 1` で 0x17c85b0 が 0 を返したとき、`reason 0` で 0x17c85b0 を再度呼んでから `0x17caaf0` を呼ぶ後始末の経路がある（アタッチ失敗の巻き戻し）。
  - `reason == 0` か `3`（`THREAD_DETACH`）では、0x17ca988 を呼び、`[0x1ec6f98]` が NULL でなければ呼ぶ。
- **0x17c85b0**（DLL 自身の `DllMain` 本体）: `reason == 1` のとき **0x1040 を 1 回呼ぶ**。常に 1 を返す。0x1040 は IPP の初期化（§5.2 の機能マスクと選択値の決定。0x1320 → 0x13c0 → 0x10c0 を呼び、結果を `[0x2541000]` に格納し、最後に 0x17e0388（ベンダーが Intel かの判定）を呼ぶ）。
- **0x17ca988**（reason による CRT の分岐）: `reason 0` → 0x17caaf0（デタッチ。アタッチ回数を 1 減らす。0 でなければ状態異常として 0x17cb654 を呼ぶ）、`reason 1` → 0x17ca9d8（プロセスのアタッチ）、`reason 2` → 0x17cb3c8、`reason 3` → 0x17cb3a0。
- **0x17ca9d8**（`DLL_PROCESS_ATTACH` の CRT 初期化）: 0x17cb484（クッキーと CPU の ISA 判定 0x17cb820 を呼び、0x17cd210 の TLS/ロックの初期化を呼ぶ）、0x17cb318（`gs:[0x30]` を読み、`[TEB+8]` を所有権のトークンとして `lock cmpxchg` でロックを取る形）、初期化済みの状態変数 `[0x2561478]`（1 回目だけ 0 → 1。すでに 1 なら 0x17cb654 で致命エラー）。0x17cb388、0x17cb7a0（初期化関数のポインタ表 0x244e320〜0x244e320 を順に呼ぶ。**範囲が空**）、0x17d1694 と 0x17d1650（初期化表の呼び出し。表のスロット 0x1ebd270、0x1ebd278、0x1ebd280、0x1ebd2a8 はファイル上 0。0x1ebd288〜0x1ebd2a0 の 4 つは関数ポインタ）。最後に `[0x2560ed4]` を 1 増やして 1 を返す。
- **TLS / スレッド / ヒープ**:
  - **PE の TLS ディレクトリは無い**（external-modules-usage.md §1.1 の実測。`.tls` セクションなし）。
  - TLS API は `TlsAlloc`、`TlsFree`、`TlsGetValue`、`TlsSetValue` を各 1 か所（CRT 内）、FLS API は 4 種を各 1 か所（`ipp-encscan` の数）。
  - **スレッドを作る API（`CreateThread` など）は 72 名の中に無い**（インポート一覧の実測）。`ippSetNumThreads` / `ippGetNumThreads` のエクスポートはあるが、DLL 自身がスレッドを作る手段を持たない。この DLL はスレッドを作らず、呼び出し側のスレッドで動くと読める（読み）。
  - `GetCurrentThreadId` はクッキーの初期化だけ。`THREAD_ATTACH`/`DETACH` は上のとおり TLS/FLS の確保と解放。wasm の単一スレッド前提（決定 0017、0045）では、DllMain が `DLL_THREAD_*` で呼ばれることはない（ゲストが呼べば別。ゲストが `DisableThreadLibraryCalls` を呼ぶかは未確認）。
  - ヒープ: `GetProcessHeap`、`HeapAlloc`、`HeapFree`、`HeapReAlloc`、`HeapSize`（CRT の `malloc` 系）。`ippMalloc`（0x73c5c0。エクスポート）は、引数の範囲を検査して別の関数に jmp する薄い入口で、割り当ての実体は未精査（未確認）。
  - `gs:` の参照（閉包内に 5 か所）: `gs:[0x30]` 2 か所（TEB の自己ポインタ。`fle_vex.h` が既にモデル化しているオフセット）、`gs:[0x60]` 2 か所（PEB。0x17d1a96 と 0x17d573c。読んだ範囲では `[PEB+0x20]` と `[PEB+0xbc]`）、`gs:[0x10]` 1 か所（0x17dabbc。**`__chkstk` の中で、StackLimit を読んでスタックのガードページを順に触る**。この関数は 150 か所から呼ばれる。0x17daba0）。**現行の生成器は `gs:[0x30]` と `gs:[0x58]` 以外を未対応にする**（実測: `gs offset is not modeled` 3 か所）。`__chkstk` が止まると、フレームの大きい関数（IPP の多く）を 1 つも呼べないので、**StackLimit（TEB+0x10）と PEB のモデル化は、ロードの前に決めるべき項目**（値は決めない。0073 の未決事項に近い。**新しい決定が要る**）。
- DllMain の直接の到達範囲（`ipp_reach`、種を上の 9 つにしたもの）は 230 関数、10,245 命令。その生成 C のうち、`fle_unsupported` が 181 か所（66 関数）、うち `Call` 108（ニーモニックの数。merged 全体で `Call` の未対応は IAT 経由の 138 件だけなので、**IAT 経由の call**と推定。この 108 のうち実際に実行されるのは一部）、`Vmovdqu` 18、`Int3` 12（パディングまたは致命エラー）、`Vmovdqa` 8、`Vmovntdq` 8、`Movntps` 8、`Jmp` 6、`Int` 4（致命エラーの `int 0x29`）、`Mov` 3（`gs:` の 3 か所）、`Pcmpeqb` 3、`Pmovmskb` 3（`dllmain-unsup.txt`）。**最も多い関数は 0x17cc7e0（34 か所）で、`Vmovdqu`、`Movntps` などの `memcpy`/`memmove` 風の関数に見える**（命令列の全体は読んでいない。未確認）。つまり**`DllMain` を通すだけでも、(1) `call [IAT]` の対応、(2) `gs:` の 3 か所、(3) AVX と非一時ストアの一部が要る**（読み。選択値が 0 の `memcpy` が AVX 版を選ぶかは、CRT の ISA 判定 0x17cb820 の結果 `[0x25435a0]` / `[0x25435a4]`、`[0x25614c4]` に依る。未確認）。

## 8. 実現性の判定

**条件付き可能。不可能とする根拠は見つからなかった**（判断。数字は実測）。

可能と判断する根拠:
- DLL は fe-pe が読め、fe-decode の閉包は誤デコード 0（`IPPCODE` を含めた測定）、`fe-lift` は 8 秒で生成を終える。リベースしたコピーでも閉包は同一。**既存の 3 つの crate の構造を変えずに通る**（§2.3、§3）。
- 外部インポートは KERNEL32 の 72 名だけ。ほかの DLL への依存は、GetProcAddress / LoadLibraryExW の動的なもの（§6）だけ。
- 現行の CPUID 応答のまま、実装系統を SSE2 だけに絞って生成すれば、生成 C は 42.9 MB、命令は約 48 万、未対応は 4,938 箇所に収まる（§5.5）。FLEngine の約 1/5 の大きさ。

条件（満たされない場合は「不可能」ではなく「その部分が止まる」）:
1. **`IPPCODE` を fe-decode が読めること**（現行は `.text` の名前だけ。§3.1）。
2. **`call [IAT]` の生成器対応と、35 名の kernel32 の追加**（§6）。
3. **`gs:[0x10]`（StackLimit）と `gs:[0x60]`（PEB）のモデルの決定**（§7）。`__chkstk` が 150 か所から呼ばれる。
4. **CPU 振り分けの方針**（§5）。選択値 0 に固定できるという仮説が実行で確認できること。確認できなければ、AVX 以降の約 90 万命令が問題になる。
5. **間接 jmp（`jmp [表 + 添字*8]`）の対応**（434 のスタブ。現行は未解決として停止）。表が静的に読めるので、`jumptab` の仕組み（決定 0036）に似た形で解ける見込み（推測）。
6. **2 モジュールのリンク**（`R[16]`、`FL`、`X[16]`、dispatch 表の重複定義。§2.5）。
7. **wasm の大きさとメモリ**: SSE2 に絞らず全部を生成すると、wasm が 300 MB 級（推測）。ブラウザでの上限は未確認。

不可能と判断する材料が出る場合の例（今は出ていない）: ブラウザの wasm の大きさの上限が約 420 MB（推測）より小さい（未確認）、選択値が 0 に落ちない（実行で AVX 系の関数に入る）、DLL が実行時に別の DLL（`api-ms-win-*`）に強く依存する（実行経路は未確認）。

## 9. 作業の分割案（案。工数は**推測**）

工数は、このリポジトリの過去の作業（決定 0009〜0072 の 1 件あたり）から見た粗い見積もりで、根拠は弱い。1 件 = 1 人が 1〜数日の作業。

| 番号 | 作業 | 内容 | 規模（推測） |
|---|---|---|---|
| W1 | リベース対応とイメージの置き方 | ファイル段階のリベース（DIR64 のみ。HIGHLOW なし。15,602 件）、基底の決定（§2.4。**値は決定が要る**）、`image.c` の 2 イメージ対応、`fle_mem_set_image` の 2 範囲化 | 中（3〜5 件） |
| W2 | モジュールの出し入れ | `LoadLibraryW("dsp_ippv2_x64.dll")` が実在のモジュールを返し、`GetProcAddress` がその export（446 名。ゲストが引く 420 名）を返し、`FreeLibrary` が参照を数える。`FLE_EXE_HANDLE` の「ゲスト自身」の前提（8 ファイル）を、モジュール表の引数にする。`fle_resp_table` は FLEngine の擬似 VA 用のまま、DLL は実アドレスを返す（現行は擬似 VA 方式のため、実アドレスを返す形に変える）。`GetModuleHandleW` と `GetModuleFileNameW` の対応 | 中〜大（4〜6 件。runtime の 8 ファイルに触る） |
| W3 | fe-decode / fe-lift の変更 | (a) 複数のコードセクション（`IPPCODE`）、(b) 再配置の指す値（507 個の「関数でないエントリ」）を種にする、(c) `--seeds` と `--pdata` を使った系統の絞り込み（§5.5。既に `--seeds` はある）、(d) 間接 jmp の表（434 スタブ）の解決、(e) 生成する名前空間（`f_<VA>`）が FLEngine と重ならないことの確認 | 中（3〜5 件） |
| W4 | 2 モジュールの生成とリンク | `R[16]`、`FL`、`X[16]`、`fle_dispatch_table`、`fle_dispatch_count`、`fle_resp_table` の重複定義の解消（DLL 側を `extern` にするか、DLL の呼び出しを別の dispatch 表にする）。`build.sh` の 2 モジュール対応（DLL の `DLL=` 変数が 1 つ）、`split_c.py`、wasm の大きさと最大メモリ | 中（3〜4 件） |
| W5 | `call [IAT]` の生成器対応 | 138 か所。応答表に行のある API を `call` から呼べるようにする（現行は `jmp [IAT]` の薄いスタブだけ）。Windows の x64 の呼び出し規約の下で、戻りアドレスをスタックに残す形（`imports_table.rs` の先頭コメントにある形）を `call` にも適用 | 小〜中（2〜3 件） |
| W6 | kernel32 の追加 | 応答表に無い 35 名。ただし ほとんどは CRT の経路で、実際に呼ばれるのは `InitializeCriticalSectionAndSpinCount` ほか数個の見込み（§6.1）。残りは「呼ばれたら名前付きで止まる」で足りる可能性がある（`fle_stop` の流儀。決定 0055 の `SetInputScope`）。仕様の確認（Learn の各ページ）は 1 名ごとに要る | 中（5〜10 件。呼ばれる名前の数に依る） |
| W7 | `gs:` の TEB / PEB のモデル | StackLimit（TEB+0x10）、PEB（`gs:[0x60]` と `[PEB+0x20]`、`[PEB+0xbc]`）。**値の決定が要る**（創作しない。仕様から言えるかを確認）。決定 0032 の `gs:[0x30]` と `0x58` に続く | 小〜中（1〜2 件 + 決定） |
| W8 | 未対応命令 | (1) 方針の決定: SSE2 系統に絞る（§5.5）か AVX を実装するか。(2) SSE2 に絞る場合でも、レガシーの未対応が 4,074 箇所（35 種。`Orps` 773、`Psrldq` 415、`Andnps` 366、`Lddqu` 196、`Movddup` 170、`Movmskps` 155、`Pmovmskb` 129、`Pextrw` 111、`Pcmpeqb` 110 ほか。多くは SSE3 / SSSE3 / SSE4。SSE2 なのに未対応のもの（`Orps`、`Psrldq`、`Pcmpeqb`、`Pmovmskb`、`Movmskps`）が含まれる）。(3) AVX を実装する場合は 361 + 82 種、約 90 万命令（VEX 版）+ EVEX。1 件ずつ決定記録の作法（SDM の確認、テスト）で、既存の `sse_*.rs` と同じ。**ここが最大**（SSE2 に絞って 35 種で 5〜10 件、AVX 全部なら 100 件以上の見込み。推測） | SSE2 に絞って中、AVX は非常に大 |
| W9 | CPUID と XGETBV | 現行のまま（選択値 0）とするか、変えるか。変えるなら XCR0 の値の決定（決定 0047 の条件）。到達確認（選択値が 0 になることを実行で確かめる） | 小（1〜2 件 + 決定） |
| W10 | テスト | 既存の流儀: 生成した C を `clang` でビルドして実行し、値を固定する（`tests/*.rs`）。IPP の関数の**結果の bit 一致**は、実機の値が手元に無いので、native でホストの IPP を呼べないため、**検証の方法自体が未決**（未確認。外部の根拠なしに「正しい」と書けない）。ロード、`GetProcAddress`、DllMain の完走、スタブの選択値、`ippMalloc` などの単純な関数の値は、手計算できる範囲でテストできる | 大（方法の決定を含む） |

- 最小の道筋（A4 の先に進む、が目的の場合）: W1、W2、W3(a)(b)(c)(d)、W4、W5、W6（呼ばれる名前だけ）、W7、W9（現行のまま）、W8(2)。**これで DllMain が通り、420 名の `GetProcAddress` が成功し、ゲストが IPP 関数を呼ぶ経路に届くまで**。IPP の関数を実際に呼ぶ（呼ばれたら結果が正しいか）は、W8 の AVX の方針と W10 の検証方法の決定に依る。
- 依存: W1 → W2、W3 → W4、W5 → W6、W7 と W8 は W4 の後。

## 10. 確認できていないこと（まとめ）

- 実機で `dsp_ippv2_x64.dll` がどう読み込まれ、`f_653f40` が何度呼ばれるか。
- 選択値が実際に 0 になるか（§5.4 の仮説）。実行して確かめていない（DLL を実行する環境が無い）。
- 添字 0〜5 と命令集合（SSE2 / AVX / AVX2 / AVX-512）の対応は、読みと分類の合致から言うもので、IPP の文書では確認していない。
- `xgetbv` 7 か所のうち後ろの 2 つ（0x1b31420、0x1cba0e0）の前提の分岐。
- `.text` の再配置 456 件と、`.text` 内の 57 個の表の対応、507 個の「関数でないエントリ」の内訳の全部。
- `ippMalloc` を含む IPP の公開関数の中身、`0x17cc7e0` の正体。
- `api-ms-win-*` の遅延ロードが実行されるか、`fle_modules[]` の規則で答えられるか。
- ブラウザの wasm の大きさの上限。現行 wasm の実際のメモリ使用量（`HEAPU8` が export されておらず取れなかった。`mem_probe.cjs`）。
- 生成 C の wasm 化とリンクを実際に行っていない（見積もりは推測）。
- IPP の関数の結果の bit 一致を確かめる方法（W10）。
- `ipp_chain` と `ipp-encscan` の FLS / SList の参照数の食い違い（§6.1）。
- 間接 jmp の未解決 617 か所のうち、スタブ形 434 以外の 183 か所の形。

## 11. 作ったファイル（`lift/out-ipp/`。既存のファイルは変更していない）

- ビルド: `target/`（fe-pe、fe-decode、fe-lift の release。`CARGO_TARGET_DIR`）、`target-enc/`（下の Rust の補助ツール）、`tmp/`（空。TMPDIR）。
- 補助ツール（読み取り専用）: `ipp_reloc_scan.py`、`ipp_imports_scan.py`、`ipp_disasm.py`、`ipp_disasm_range.py`、`ipp_reach.py`、`ipp_chain.py`、`ipp_callers.py`、`ipp_stubs_scan.py`、`ipp_unsup_summary.py`、`ipp_dllmain_unsup.py`、`ipp_merge_text.py`（測定用の改変コピーを作る）、`ipp_rebase.py`（同）、`mem_probe.cjs`（取れなかった）、`encscan/`（Rust の補助ツール `ipp-encscan`、`ipp-reach`、`ipp-tables`。`fe-pe` と `fe-decode` を path で参照）。
- 測定の出力: `closure-*.txt`、`lift-*.txt`（`fe-lift` の統計）、`encscan*.txt`、`tables-class.txt`、`stubs-scan.txt`、`stubs-list.tsv`、`reloc-scan.txt`、`imports-scan.txt`、`chain-dllmain.txt`、`reach-*.txt`、`dllmain-*.txt`（逆アセンブル）、`cpu-*.txt`、`init-*.txt`、`unsup-merged.txt`、`legacy-*.txt`、`*-seeds*.txt`。
- 生成 C（大きい。合計約 4.1 GB。単一ファイルは最大 584 MB）: `ipp-pdata.c`、`ipp-merged.c`、`ipp-legacy.c`、`ipp-idx0.c`、`ipp-idx2.c`、`ipp-reloc-max2.c`、`ipp-reloc-max5.c`、`ipp-pdata-tables.c`、`ipp-pdata-reloc.c`、`ipp-rebased-4000000.c`、`flengine-pdata.c`（比較用）。コンパイルはしていない。
- 改変コピー（**実在の DLL ではない**）: `merged-text.dll`、`merged-rebased-2000000.dll`、`merged-rebased-4000000.dll`、`merged-rebased-8000000.dll`。

