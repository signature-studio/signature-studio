# A5 native オラクルを dsp_ippv2_x64.dll（縮小でない・リベース済み）へ適用する

日付: 2026-10-27（作業は 10-25 に開始。ネットワーク障害とサーバ再起動で 2 回中断し、10-27 に完了）。対象は `git archive HEAD` のスナップショット。測定のみで、lift は修正していない。決めない。設計の元は `docs/decisions/0035-a5-oracle.md`、`0036-a5-oracle-v2.md`、`0038-mxcsr-flags.md`、手順の正本は `docs/investigations/a5-oracle-rerun-3.md`（FLEngine 向け）。

凡例: **実測** = この作業でツールを走らせて得た出力。**読み** = ソースや実測からの解釈で、直接は確かめていないもの。**未確認** = 確かめていない。

## 1. 結論（数値）

DLL = `dsp_ippv2_x64.dll`（SHA-256 `497dd4f7…c352`）を `fe-pe rebase` で 0x2000000 にした像（SHA-256 `4fd25502…efc5`、決定 0080 の値と一致）。native も lift もこの像を使う。

| 実行 | 関数 × ケース | bit_exact | mismatch | lift_unsupported | native_fault | ネガコン（出力差 / fault 込み） |
|---|---|---|---|---|---|---|
| **seed 1**（FLEngine と同じ判定基準の純計算 19,635 件から無作為） | 1,000 × 16 = 16,000 | **4,734** | **87**（全件 `lift_timeout`。下の再現で 87/87 が bit_exact） | **1,220**（137 関数） | **9,959**（62.2%） | 83/200 = 41.5% / 169/200 = 84.5% |
| **seed 2**（決定 0075 の 26 ニーモニックを含む純計算 2,817 件から無作為） | 1,000 × 16 = 16,000 | **9,783** | **5**（全件 `lift_timeout`。再現で 5/5 が bit_exact） | **204**（24 関数） | **6,008**（37.6%） | 84/200 = 42.0% / 188/200 = 94.0% |
| seed 3（`pmuludq` か `psubb` を含む純計算 295 件を全部） | 295 × 16 = 4,720 | **2,656** | **0** | **0** | **2,064**（43.7%） | 48/100 / 89/100 |
| 拡張 E1: CPU 振り分けスタブ（0079）× セレクタ値 | 170 スタブ・746 組 × 16 = 11,936 | **8,677** | **0** | **1,930** | **1,329** | 1,103/1,395 ケース（79.1%）、100 組中 96 組で検出 |
| 拡張 E2: MSVC switch（0094）を含む 4 関数（強制） | 168 | **45** | **0** | 0（`lift_stop_other` 39） | **84** | なし |

- **mismatch の再現（アラームを延ばした）**: seed 1 の 87 件（25 関数）を、アラーム 3,600 秒・並列 4 で単独再実行 → **87/87 が bit_exact**（GPR・XMM・イメージ/scratch/スタックのメモリ差分・EFLAGS 定義済みフラグ・MXCSR・x87 が一致）。native 0.09〜8.97 秒（中央値 2.89）に対し lift 14.9〜1,898 秒（中央値 567.5）、比は 95〜394 倍（中央値 173 倍。native 0.3 秒以上の 83 件）。seed 2 の 5 件も 5/5 が bit_exact（lift 59〜467 秒）。**値の不一致（lift_timeout 以外の mismatch）は全測定で 0 件**。**読み**: 92 件はオラクルの時間の上限（2 秒、再実行 10 秒）に lift が届かなかったもので、lift の値の欠陥の証拠ではない（FLEngine の `a5-oracle-rerun-3.md` §6.4 と同じ型）。
- **決定 0075（26 ニーモニック）**: seed 1 の標本に 158 関数が含む（22 種）、seed 2 は 1,000 関数全部（25 種）。bit_exact のケース内で**実行された**ニーモニックは seed 1 で 19 種、seed 2 で 23 種、追加の seed 3 で `pmuludq` と `psubb` が各 1 ケース（§5.9）。純計算に 1 件もない `movnti` は一度も走らせていない。
- **決定 0079（CPU 振り分けスタブ）**: FLEngine と同じ判定基準では純計算に **0 件**（標本にも 0 本）。スタブは `.pdata` の開始でもない。拡張 E1 でセレクタ変数を両側で固定して走らせた: **mismatch 0**、434 スタブ中 170 が（到達先が純計算の組を持つので）走り、167 が bit_exact（§5.10）。
- **決定 0094/0095（MSVC switch）**: 純計算に **0 件**（標本に 0 本）。含む 4 関数を強制して走らせた E2 で、1 サイト（`0x37cc80b`、16 arm）は 31 ケースで bit_exact、3 サイトには bit_exact で届いたケースが 0（§5.11）。HEAD の 0095 が足した DLL の 6 つ目のサイト（`0x37cd3eb`）は**走らせていない**（§3.2、§7）。

## 2. 設計の選択

### 2.1 native 側の像の基底: リベース済み（0x2000000）の像を native でも mmap する

選んだもの: **native も lift も `fe-pe rebase` で作った 0x2000000 の像を `argv[3]` で受け取り、`image.c` が ImageBase にそのまま写す**。元の DLL（ImageBase 0x180000000）は native に渡さない。

理由（読み）:

- lift の生成 C は 0x2000000 系の VA を埋め込む（**実測**: `fle_dispatch_table` に `UINT64_C(0x2001000), f_2001000`、スタブは `fle_ld(UINT64_C(0x4541000), 4)`）。ドライバ（`oracle_main`）は native も lift も同じ `image_load` を呼ぶので、**同じファイルを渡せば両側のメモリ内容と VA が同一**になる。
- したがって「ポインタを値として返す関数」「再配置された絶対参照（`.reloc` の DIR64 15,602 件。**実測**: リベース報告）」「引数・スタックのポインタ」の値が両側で同じになり、VA の違いによる偽の差が出ない。元の 0x180000000 の像を native に渡す案は、`.data`/`.rdata` 内の再配置済みポインタの値が両側で違ってイメージ領域の差分比較（書き込み可能セクション全体）がポインタ値で食い違うので採らなかった。
- 2 モジュール構成（150/942）で実際に動くのは 0x2000000 の像（決定 0080 D1）なので、測るのも実際に動くものになる。
- scratch（0x20000000）とスタック（0x30000000）は DLL の像（0x2000000〜0x461f000）と重ならない（**実測**: `SizeOfImage` 0x261f000）。

限界（読み）: **`fe-pe rebase` 自体の正しさはこのオラクルでは検査されない**（両側が同じリベース済みファイルを読むため）。実 DLL が本来の基底 0x180000000 で動いたときの挙動（ポインタの上位 32 ビットが非 0 であることに依存する演算）は走らせていない（**未確認**）。元の DLL を 0x180000000 のまま lift して比べる案は実装できる見込みだがやっていない（**未確認**）。

### 2.2 そのほかの選択

| 項目 | 選択 | 区分 |
|---|---|---|
| 関数の選択 | `.pdata` の非チェーン開始 ∩ 生成 C の `fle_dispatch_table`。DLL は `.pdata` 50,529 エントリ、非チェーン開始 30,816、生成 C の関数 32,648 | 実測 |
| 純計算の判定 | **FLEngine と同じ基準**（`select_pure.py`: IAT 参照、間接 call/jmp、x87、FS/GS、cpuid 等、lift 済みでない直接 call 先、およびそれらを推移的に呼ぶ関数を除外） | 実測 |
| 引数生成・比較対象・ネガコン | FLEngine と同じ（`argptr.py`、`run_diff2.py`、`oracle2_common.c`）。**変更なし** | 実測 |
| import | native 側に応答を用意しない。IAT を参照する関数は純計算の判定で除外（DLL では 68 件が `iat`、callee 経由が他にある） | 実測 |
| TEB/GS | native は GS を設定しない。GS/FS を持つ関数は除外（`segment` 4 件、callee 経由で 170） | 実測 |
| ビルド | 生成 C を `split_c.py` で 800 関数ごと 41 分割、clang -O0（FLEngine と同じフラグ）。emcc なし | 実測 |
| 並列 | clang ≤ 4。`run_diff2.py` の `--jobs` は seed 1・2 が 12（FLEngine の再実行は 16）、再現は 4 | 実測 |

## 3. DLL に適用するのに必要だった変更

| # | 項目 | FLEngine | DLL | 対応 | 区分 |
|---|---|---|---|---|---|
| 1 | 像のマップ先 | ImageBase 0x400000 | 0x2000000（リベース済み） | 変更不要（`image.c` は PE の ImageBase に mmap） | 実測 |
| 2 | **IAT の範囲** | `.idata`/`.didata` | **どちらも無い**。IAT は `.rdata` 内（データディレクトリ 12: RVA 0x1ebd000、0x248 バイト = 73 スロット、KERNEL32.dll の 72 件） | **`select_pure.py` の `init` を部分編集（+3 行）**: 2 セクションが無いときだけデータディレクトリ 12 を使う。FLEngine の経路は不変 | 実測 |
| 3 | 書き込み可能セクション | `.data`/`.bss`/`.idata`/`.didata` | `.data`（0x4541000〜）と `IPPDATA`（0x45f8000〜）。`pe_parse` は最小〜最大を 1 範囲にするので間の `.pdata` も入る | 変更不要（範囲が広いだけ） | 読み |
| 4 | 実行セクション | `.text` 1 つ | `.text` と `IPPCODE` の 2 つ | 変更不要 | 読み |
| 5 | `run_diff2.py` の TMPDIR | `lift/out-oracle/tmp` | 同じ（`mkdir -p src/lift/out-oracle/tmp` が要る） | 変更不要 | 実測 |
| 6 | 生成 C の大きさ | 215 MB | **537 MB**（6,708,318 行、2GB 級ではない） | `split_c.py` で分割 | 実測 |
| 7 | CPU 振り分けスタブ（0079）と MSVC switch（0094/0095） | なし | スタブ 434、MSVC switch 5（HEAD の 0095 で 6） | 判定基準では間接 jmp として除外される（§5.9）。拡張 E1・E2（新規スクリプト） | 実測 |

**FLEngine 向けの既存の挙動と出力は変えていない**。既存ファイルの変更は `select_pure.py` の 3 行だけ。確認（**実測**）: パッチ前後の `select_pure.py` を FLEngine（SHA-256 `b2c64da4…496d`、生成 C は `lift/out-a5-head/` の `new-part-00.c`）に走らせた `pure.json`（純計算 9,901 件）は、パッチ前・パッチ後・`a5-oracle-rerun-3.md` の `pure.json` が**バイト単位で同一**（`cmp`）。`mp.Pool` がセマフォ作成で `OSError 122` になったため、`mp.Pool` を直列実行に差し替える `dsp_serial_select.py` 経由で `main()` を走らせた（同じコード経路）。

### 3.1 HEAD について

依頼文の HEAD は a951ecc。作業中の HEAD は f1bcba7（a951ecc の次。差は `docs/investigations/bundled-dll-inventory.md` の追加のみ。**実測**: `git diff --stat a951ecc HEAD`）。測定の元になったスナップショットは 5002d98 の `git archive`。5002d98 から HEAD までの `lift/` の変更は `fe-decode/src/{jumptab,lib}.rs`（決定 0095）とテストだけで、`lift/harness/`、`lift/runtime/`、`fe-lift/src/` は不変（**実測**: `git diff --stat 5002d98 HEAD -- lift`）。

### 3.2 生成 C が HEAD の lifter で変わるか（実測）

HEAD（f1bcba7）の lifter で DLL の生成 C を作り直し（`gen/dsp-pdata-head.c`、536,779,902 B、SHA-256 `71111810…4f62`）、5002d98 の lifter の生成 C（`gen/dsp-pdata.c`、536,776,817 B、SHA-256 `5db97dfe…99b9`）と関数本体のハッシュで比べた（`dsp_cmp_gen.py`）。

| 項目 | 結果 |
|---|---|
| 関数の集合 | 32,648 / 32,648、片側だけ 0 |
| 本体が同一 | **32,647** |
| 本体が違う | **1**（`0x37cd2c0`。決定 0095 の新しい形の switch `0x37cd3eb` を含む） |
| プロローグ（dispatch 表、応答表） | 同一 |
| 純計算 19,635 件のうち本体が違うもの | **0**（`0x37cd2c0` は純計算でない: 最初の除外理由 `indirect_jmp`、107 命令、直接 call 先は `0x37cd2a0` のみ） |
| `fe-lift` の統計 | `jumptabs` 439 → 440、`msvc_tables` 5 → 6、`jmp_ind:reg_dispatch` 361 → 360、`unsupported_total` 974,527 のまま |

したがって、**この文書の測定（seed 1・2・3、E1、E2 の 4 関数）に使った lift_drv2 は 5002d98 の lifter の生成 C から作ったが、走らせた関数の本体は HEAD の lifter の生成 C と同一**（読み。測定した全関数の本体が同一であることから）。HEAD の生成 C で `lift_drv2` を作り直さなかった理由は、作業中にディスクの空きが尽きた（§8.2）ためで、`0x37cd2c0`（6 つ目の MSVC switch）は走らせていない。

## 4. 手順

`R=/home/ubuntu/work/signature-studio-develop/lift/out-a5dsp`、`H=$R/src/lift/harness/oracle`、`PE=$R/dll/dsp_ippv2_x64.rebased.dll`。全 python に `PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=$R/build/tmp`、`CARGO_TARGET_DIR=$R/target`。

```
# 0. スナップショットとパッチ
git archive HEAD | tar -x -C $R/src/ ; (cd $R/src && git apply $R/patches/a5dsp.diff)   # 測定時は 5002d98 の archive に同じ新規ファイルを置いた
mkdir -p $R/src/lift/out-oracle/tmp                                                       # run_diff2.py が TMPDIR にする
# 1. lifter（6 秒）、リベース、生成 C（6.6 秒、537 MB）
(cd $R/src/lift && cargo build --release -p fe-lift -p fe-pe)
$R/target/release/fe-pe rebase lift/out-ext/dsp_ippv2_x64.dll 0x2000000 $PE
$R/target/release/fe-lift lift $PE $R/gen/dsp-pdata.c --pdata          # 種なし = 全体
# 2. 分割・clang・リンク（1 分 38 秒、並列 4、警告・エラー 0）。lift_drv2 は 624 MB
B=$R/build GEN=$R/gen/dsp-pdata.c JOBS=4 sh $H/build_dsp.sh
# 3. 純計算の選別・ポインタ判定・除外集計（それぞれ約 4 秒）。dispatch 表は part-00 の先頭にある
python3 $H/select_pure.py $PE build/gen/dsp-part-00.c build/results/pure.json
python3 $H/argptr.py $PE build/results/pure.json build/results/argptr.json
python3 $H/exclude_stats.py $PE build/gen/dsp-part-00.c build/results/exclude-stats.json
# 4. 命令の種類の集計
python3 $H/dsp_census.py gen/dsp-pdata.c build/results/census.json
python3 $H/dsp_pool.py $PE build/results/pure.json build/results/census.json build/results/pool --seed 2 --count 1000
python3 $H/dsp_stubs.py $PE gen/dsp-pdata.c build/results/stubs2.json
# 5. 測定（ORACLE_BIN=$R/build/bin）
python3 $H/run_diff2.py --pe $PE --pure build/results/pure.json --argptr A --exclude E --out build/results/seed1 --count 1000 --n 16 --seed 1 --negctl 200 --jobs 12   # 50 分 56 秒
python3 $H/run_diff2.py ... --pure build/results/pool/pool-sse.json --out build/results/seed2b --seed 2 --negctl 200 --jobs 12                                         # 21 分 29 秒
python3 $H/dsp_pool_mn.py build/results/pure.json build/results/pool/classes.json POOL.json pmuludq,psubb,movnti
python3 $H/run_diff2.py ... --pure POOL.json --out build/results/seed3mn --count 295 --seed 3 --negctl 100 --base200 50 --jobs 8
# 6. mismatch の再現（単独・長いアラーム）。ORACLE_TIMEOUT はドライバの環境変数（oracle2_common.c）
python3 $H/dsp_repro.py build/bin $PE ALLCASES.txt IDS.txt OUT.json --native-alarm 120 --lift-alarm 3600 --jobs 4                                                      # seed 1: 3 時間 25 分
# 7. 拡張
python3 $H/dsp_run_stubs.py $PE build/bin build/results/stubs2.json build/results/pure.json build/results/argptr.json build/results/stubs --n 16 --jobs 4 --negctl 100   # 1 分 39 秒
python3 $H/dsp_force_pool.py $PE build/results/pure.json build/results/stubs2.json build/results/pure-force.json ; python3 $H/argptr.py $PE build/results/pure-force.json build/results/argptr-force.json
python3 $H/dsp_run_msvc.py $PE build/bin build/results/stubs2.json build/results/argptr-force.json build/results/msvc --jobs 4                                      # 5 分 18 秒
# 8. 集計
python3 $H/dsp_exec_mn.py $PE RESULTDIR POOLJSON OUT.json ; python3 $H/dsp_sample_stats.py ... ; python3 $H/dsp_fragments.py $PE RESULTDIR...
```

新規ファイル 15 本（すべて `lift/harness/oracle/`）: `build_dsp.sh`(39 行)、`dsp_census.py`(69)、`dsp_cmp_gen.py`(60)、`dsp_exec_mn.py`(88)、`dsp_force_pool.py`(38)、`dsp_fragments.py`(50)、`dsp_pool.py`(179)、`dsp_pool_mn.py`(25)、`dsp_repro.py`(113)、`dsp_run_msvc.py`(120)、`dsp_run_stubs.py`(177)、`dsp_sample_stats.py`(127)、`dsp_select_ext.py`(126)、`dsp_serial_select.py`(30)、`dsp_stubs.py`(94)。既存ファイルの変更は `select_pure.py` の +3 行。合計 +1,338 行、`patches/a5dsp.diff` は 1,439 行。**`lift/harness/{a4.c,a4.h,image.c}`、`lift/crates/`、`lift/runtime/` は diff に含まれない**（実測: `diff --git` の行を全部確認）。

## 5. 測定結果

### 5.1 純計算の一覧（FLEngine と同じ判定基準）

| 項目 | DLL | FLEngine（参考: rerun-3） |
|---|---|---|
| `.pdata` 非チェーン開始 = 候補 | 30,816 | 57,816 |
| **純計算** | **19,635（63.7%）** | 9,901（17.1%） |
| 除外の理由（最初の理由） | `target_not_function` 7,492、callee 経由 2,808、`indirect_call` 345、`indirect_jmp` 202、`cpuid` 150、`iat` 68、`int3` 61、`x87` 33、`wait` 18、`segment` 4 | `callee` 29,331、`indirect_call` 16,461、`target_not_function` 2,036 ほか |
| 純計算のうち生成 C に `fle_unsupported` を持つ（自身 / 呼び先経由） | 4,504 / 5,732 | — |
| 純計算のうち決定 0075 の 26 ニーモニックを含む | 2,817 | — |
| 純計算のうち VEX/EVEX を含む / SSE3 以降の形を含む | 3,112 / 1,325 | — |
| 純計算のうち `SizeOfProlog == 0` の `.pdata` 開始 | 4,017（20.5%） | — |

**読み**: DLL は SIMD の小さな計算関数が多く、純計算の割合が FLEngine より高い。

### 5.2 seed 1（FLEngine と同じ手順・パラメータ）

16,000 ケース: bit_exact 4,734、mismatch_core 87（全件 `lift_timeout`）、mismatch_aux 0、lift_unsupported 1,220、lift_stop_other 0、native_fault 9,959。関数単位: bit_exact_all_comparable 455、has_mismatch_core 25、native_fault_all 436、lift_not_executable 84。ポインタ引数を持つ関数 673（10,768 ケース: bit_exact 4,136、native_fault 5,582、mismatch 9、unsupported 1,041）、持たない関数 327（5,232 ケース: 598 / 4,377 / 78 / 179）。

補助指標（**実測**）: EFLAGS 定義済みフラグの差 0（CF・PF・ZF・SF・OF 4,734 全部、AF 4,486 を比較、AF 未定義 248）、x87 の差 0、MXCSR 状態ビットだけの mismatch_aux 0（`mxcsr_lift_ne_reset` と `mxcsr_native_ne_reset` はともに 1,404）、実行された自身の命令の割合 平均 41.3%・中央値 33.8%・全命令実行 161・ゼロ 438（FLEngine は平均 91%）。timeout 再実行 193 件（native 前 timeout 143・後 9、lift 前 138・後 133）。

### 5.3 mismatch 87 件の切り分け（実測）

87 件は 25 関数（`0x20c5b60` から `0x37f4160`。1 関数 1〜7 件）、`mismatch_fields` は `lift_timeout` のみ。値の差（GPR・XMM・メモリ・フラグ）は 0。

再現（`dsp_repro.py`）: 各ケースを native（アラーム 120 秒）→ lift（3,600 秒）の順に単独で実行し、`run_diff2.compare` で比べた。**87/87 が native・lift とも ok で bit_exact**。

| 関数 | 件数 | lift 秒 | native 秒 | | 関数 | 件数 | lift 秒 | native 秒 |
|---|---|---|---|---|---|---|---|---|
| 0x20c5b60 | 2 | 51–282 | 0.24–1.44 | | 0x2823560 | 3 | 341–1898 | 1.15–6.43 |
| 0x22e2060 | 2 | 234–600 | 0.99–2.68 | | 0x28242e0 | 4 | 109–1069 | 0.39–3.59 |
| 0x27cd900 | 4 | 259–1060 | 2.30–8.97 | | 0x2828c40 | 2 | 229–707 | 2.12–6.41 |
| 0x2805420 | 3 | 259–817 | 2.18–7.04 | | 0x283ad40 | 6 | 72–914 | 0.69–8.26 |
| 0x2808020 | 4 | 134–624 | 0.74–3.26 | | 0x283b040 | 3 | 399–809 | 3.79–7.57 |
| 0x2808320 | 5 | 206–821 | 1.10–4.74 | | 0x284c4e0 | 6 | 14–894 | 0.16–8.37 |
| 0x280aba0 | 3 | 327–1395 | 1.28–5.53 | | 0x284c7e0 | 6 | 149–940 | 1.39–8.30 |
| 0x280b1a0 | 4 | 208–1014 | 0.79–3.77 | | 0x2bf0340 | 2 | 96–111 | 0.53 |
| 0x28129a0 | 7 | 343–1274 | 1.20–4.41 | | 0x3515d20 | 1 | 233 | 0.59 |
| 0x2816de0 | 4 | 211–748 | 1.95–6.76 | | 0x3519690 | 1 | 23 | 0.09 |
| 0x2817260 | 6 | 97–889 | 0.95–8.54 | | 0x3777ce0 | 1 | 191 | 0.64 |
| 0x2817e60 | 2 | 866–923 | 8.13–8.73 | | 0x37f4160 | 1 | 289 | 1.61 |
| 0x281a2e0 | 5 | 19–869 | 0.12–3.97 | | | | | |

- lift が 1,200 秒を超えたのは 4 件（`0x2823560` の case 4 が 1,898 秒、`0x280aba0` の case 9 が 1,396 秒、`0x28129a0` の case 15 と 13 が 1,274 と 1,231 秒）。3,600 秒を超えたものは 0 件。**読み**: lift は `-O0 -finstrument-functions` の生成 C で命令ごとに関数呼び出しを経るため native の約 100〜400 倍遅く（FLEngine の同種は 253〜329 倍）、ループが長いケースが 2 秒・10 秒の上限に届かない。所要時間は負荷に依存する: 1 回目の再現（並列 12 の seed 1 の裏で並列 12）では `280b1a0.12`（今回 1,015 秒）と `28129a0.0`（今回 958 秒）が 1,200 秒で止まっていた。
- 1 回目の再現（`build/repro1/ext.json.d`）はサーバ再起動で 69 ケース目までで途切れ、再開後にディスク満杯（§8.2）で一部が 0 バイトの出力になったため、**最終の数値には使っていない**。最終の 87 件は `ext-final.json.d` で最初から同じ条件（アラーム 3,600 秒、並列 4）でやり直したもので、所要時間はすべてプロセス内の計測（mtime からの推定は 0 件）。
- lift の欠陥かオラクルの限界か（**読み**）: オラクルの限界（時間の上限）。lift の値の誤りの証拠は 87 件の中にない。ただし「ループの長い入力で lift が桁違いに遅い」ことは測定値として残る。
- seed 2 の 5 件（`0x3840320` case 8、`0x38415e0` case 14、`0x3841820` case 5、`0x3ba0ae0` case 5、`0x3bba7f0` case 10）も同じ手順で 5/5 が bit_exact（lift 59〜467 秒、native 0.34〜2.99 秒）。

### 5.4 native_fault の内訳（実測）

| 原因 | seed 1 | seed 2 |
|---|---|---|
| `gp_other`（うち `ret` で落ちる） | 2,840（2,828、228 関数） | 135（135、29 関数） |
| `gp_noncanonical_address` | 2,069 | 538 |
| `gp_misaligned_sse_operand`（`movaps`/`movdqa` の整列違反。lift も abort） | 1,277 | 1,216 |
| `null_deref` | 1,178 | 463 |
| `scratch_range_exceeded` | 1,099 | 2,596 |
| `wild_pointer` | 1,079 | 873 |
| `sigbus` | 265 | 123 |
| `control_flow_left_image` | 114 | 28 |
| `stack_range_exceeded` / `sigfpe` / `timeout_2s(loop)` | 18 / 11 / 9 | 14 / 0 / 22 |

**`ret` で落ちるもの（読み）**: FLEngine では目立たなかった型。`.pdata` の開始が本物の関数入口でなく、`add rsp, 0x48; ret` のような**断片**（エピローグ/コールド部）になっているもの。呼ぶと `ret` がスタック上の乱数を戻り先にして native は #GP になり、lift の `ret` はそのまま戻る（`gp_other/ok` 2,701 ケース）ので比べられない。UNWIND_INFO の `SizeOfProlog` が 0 の開始で数えた（`dsp_fragments.py`、**SizeOfProlog 0 は断片の証明ではない**。プロローグのない葉関数も 0）:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| 標本の関数 | 1,000 | 1,000 | 295 |
| うち SizeOfProlog 0 | 192 | 10 | 6 |
| その関数のケース | native_fault 3,040、bit_exact 16、unsupported 16 | native_fault 160 | native_fault 96 |
| その関数の判定 | native_fault_all 190 / 192 | native_fault_all 10 / 10 | 6 / 6 |
| SizeOfProlog > 0 の関数のケース | bit_exact 4,718、native_fault 6,919、mismatch 87、unsupported 1,204 | bit_exact 9,783、native_fault 5,848 ほか | bit_exact 2,656、native_fault 1,968 |

純計算の全体では 4,017 件（20.5%）が SizeOfProlog 0（実測）。seed 1 では 16,000 ケースのうち 3,040 ケース（19%）が、native_fault 9,959 ケースのうち約 3 割がこれに当たる。残りは主に、構造体の中身や長さを引数が決める関数（`argptr.py` の近似の限界、0036 §7）と、SSE の 16 バイト整列を要するアクセス。

### 5.5 lift_unsupported の内訳（seed 1: 1,220 ケース・164 サイト・137 関数）

停止の文は全件 `fle: unsupported instruction at 0x…`（生成 C の `fle_unsupported`）。命令はサイトのアドレスから生成 C のコメント（`/* 0x… Mnemonic */`）で引いた。lifter 自身の理由（`fe-lift` 標準出力の `unsupported` 欄）は全件 `mnemonic outside minimal set`。

| 分類 | ケース | サイト | 関数 | 命令（ケース数） |
|---|---|---|---|---|
| **vex**（V*/K*、BMI） | **1,003** | — | 104 | Vmovss 217、Vmovsd 209、Vxorps 176、Vxorpd 130、Vmovupd 59、Vmovapd 32、Vbroadcastf128 29、Vstmxcsr 17、Vsubps 16、Vbroadcastsd 16、Vroundsd 16、Vcvtss2sd 16、Vmovq 14、Vmovdqu 13、Vmovd 7、Vshufps 7、Vperm2f128 7、Vmovdqa 7、Vinsertf128 5、Vunpcklps 5、Vbroadcastss 4、Shlx 1 |
| **sse3p**（SSE3/SSSE3/SSE4） | **193** | — | 31 | Movddup 119、Pshufb 14、Lddqu 14、Addsubps 10、Phaddd 8、Movsldup 7、Movshdup 6、Haddps 6、Ptest 5、Haddpd 4 |
| **other** | **24** | 2 | 2 | **Pushfq 24**（`0x38b5da0` で 13、`0x37f8c60` で 11） |

関数別の全表（137 関数、サイトと件数つき）は `build/results/unsup-by-function.txt`、命令別の表は `build/results/seed1/sample-stats.json`。最多は `Vmovss`（17 関数・19 サイト）、`Vmovsd`（14・18）、`Vxorps`（31・31）。seed 2（24 関数・204 ケース）は全件 sse3p: Movddup 147（12 関数）、Haddps 34（5）、Pshufb 21（6）、Lddqu 2（1）。

切り分け（**読み**）:

- **vex（1,003）と sse3p（193）= lift の未対応で、オラクルの対象外ではない**。native は AVX 対応のホストで実行でき、比較の入力も取れている。決定 0075 §1 は「現行の CPUID 応答は SSE/SSE2 のみで、SSE3 以降・AVX は名乗らない（lift が対応しない機能を主張しない）」と書いていて、SSE3 の 7 ニーモニック（Lddqu、Movddup、Addsubps、Addsubpd、Movshdup、Movsldup、Haddpd）は意図的に実装していない。VEX は決定 0080 D7 が「全部入りは AVX の方針（W8）の決定後」とする。したがって**ゲストの CPUID の下では到達しない経路の可能性が高い**が、実際にどの経路に入るか（セレクタ値と CPUID の対応）は**測っていない**（未確認）。
- **other（24）= Pushfq**: `pushfq` は整数命令で、CPUID に依存しない。この停止を説明する決定文書は見当たらない（`docs/` の grep では `pushfq` が出るのは 0033、0036 の native 側の説明と M1 の文書だけ）。2 関数（24 ケース）の小さな穴だが、理由は**未確認**。
- 純計算の標本で `fle_unsupported` を持つ関数の割合: 自身 236 / 呼び先経由 297（seed 1）。

### 5.6 ネガティブコントロール

| 結果 | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| detected_core | 83 | 84 | 48 |
| detected_aux_only | 0 | 0 | 0 |
| detected_by_native_fault_only | 86 | 104 | 41 |
| undetected_state_changed_but_masked | 16 | 5 | 9 |
| undetected_equivalent_no_state_change | 15 | 7 | 2 |
| 出力の差で検出 | 83/200 = 41.5% | 84/200 = 42.0% | 48/100 = 48% |
| native fault 込み | 169/200 = 84.5% | 188/200 = 94.0% | 89/100 = 89% |

**実測**。FLEngine は出力差 56.7〜60.6%、fault 込み 87.2〜92.1%（rerun-3 §6.6）。DLL の出力差の検出率が低いのは、反転した命令が出力に出ない前に fault する（`detected_by_native_fault_only` 86〜104 件）ものが多いため（読み）。FLEngine にあった「定数返却関数 `mov eax, imm32; ret` の imm 反転 3 件」の健全性確認は、**DLL の純計算プールに該当する関数が 0 件のため行っていない**（`controls_total` は 200 のまま）。

### 5.7 seed 2（SSE/SSE2 優先）

標本の作り方: `dsp_pool.py` が純計算 19,635 件を capstone で分類し、決定 0075 の 26 ニーモニック（xmm 形）を含む 2,817 件を `pool-sse.json` にした（2,817 ≥ 1,000 なので他の legacy SSE 関数は足していない）。`run_diff2.py` が同じアルゴリズムで 1,000 を選ぶ（`Random(seed).sample(200)` + 残りの `shuffle`）。

16,000 ケース: bit_exact 9,783、mismatch_core 5（`lift_timeout`、再現で 5/5 bit_exact）、lift_unsupported 204、native_fault 6,008。関数単位: bit_exact_all_comparable 809、has_mismatch_core 5、native_fault_all 177、lift_not_executable 9。ポインタ引数を持つ関数 968、持たない関数 32。EFLAGS 差 0、x87 差 0、MXCSR 状態ビットだけの mismatch_aux 0（`mxcsr_lift_ne_reset` と `mxcsr_native_ne_reset` はともに 2,826）、実行された命令の割合 平均 54.3%・中央値 63.0%・全命令実行 30・ゼロ 177。**VEX を含む関数は標本に 0**（SSE と VEX を混ぜない関数が選ばれたため）、SSE3 以降の形を含む関数が 139。

### 5.8 seed 3（`pmuludq`・`psubb` を含む関数を全部）

seed 1・2 で標本に入っていても bit_exact のケースでは**実行されなかった**2 ニーモニックを狙った追加測定。純計算で `pmuludq` を含む 263 関数と `psubb` を含む 32 関数（重複なしで 295。`movnti` は 0 関数）を全部、16 ケースずつ。4,720 ケース: bit_exact 2,656、native_fault 2,064、mismatch 0、lift_unsupported 0。この 2 つが実際に実行された bit_exact のケースは、`pmuludq` が **1 ケース・1 関数・1 サイト**（標本内のサイト 476 のうち）、`psubb` が **1 ケース・1 関数・2 サイト**（80 サイトのうち）。**読み**: 命令は静的に多数あるが、このオラクルの入力では届く経路が少ない。

### 5.9 決定 0075 / 0079 / 0094 / 0095 を含む関数が標本に何本入っているか

**数え方（命令の種類で数える）**:

1. **決定 0075 の 26 ニーモニック**: `capstone` で関数を線形に逆アセンブルし、ニーモニックが 26 個（orps、andnps、pcmpeqb、pcmpeqw、pcmpgtb、pcmpgtw、paddb、psubb、psubq、pmaxub、pminub、pmaddwd、pmuludq、psrldq、pslldq、pmovmskb、movmskps、movmskpd、pextrw、pinsrw、movntps、movntpd、movntdq、movnti、rsqrtps、rsqrtss）のどれかで、**オペランドに xmm レジスタを持つ**ものを数える（`dsp_pool.py`。VEX 形はニーモニックが `v…` になるので入らない。MMX 形は xmm を持たないので入らない。`pextrw` は 66 0F C5 の形（オペコードの `0f c5`）だけ数える）。クロスチェック（**実測**）: 生成 C のコメント `/* 0xADDR Mnemonic */` のニーモニック（小文字）で数えると純計算 2,823 関数、capstone では 2,817、両方に出るのは 2,816。差の 6〜7 関数は、同名の MMX 形や SSE4.1 の `pextrw`/`pinsrw` 形のはず（**読み**: 個別には確かめていない）。
2. **実行された**ニーモニック: ネイティブの単一ステップ実行（`ORACLE_TRACE`、`run_diff2.py` が持つ）の実行 rip 集合と、そのニーモニックの命令アドレスの積が空でない bit_exact ケースを数える（`dsp_exec_mn.py`。トレースが 4,096 rip の上限に当たったケースは 0）。
3. **CPU 振り分けスタブ（0079）**: 生成 C の `switch (t)` のうち `(R[10] + (R[0]*8)); switch (t)` の形（`lea r10` の次の `jmp [r10+rax*8]`）を持つ関数（`dsp_census.py`）= 434。バイト列 `41 ff 24 c2` を持つ数と一致（決定 0079 の 434）。
4. **MSVC switch（0094/0095）**: それ以外の `switch (t)`（case が `goto` で関数内ラベルへ飛ぶ形）= 5 サイト・4 関数（5002d98 の lifter）、HEAD の lifter で 6 サイト・5 関数。
5. すべて**静的に含む**ことと**実行された**ことを分けて報告する。

| | 純計算プール 19,635 | seed 1 の標本 1,000 | seed 2 の標本 1,000 | seed 3 の標本 295 |
|---|---|---|---|---|
| **0075 の 26 ニーモニックを含む関数** | **2,817**（25 種。`movnti` なし） | **158**（22 種。なし: movnti、psubq、rsqrtps、rsqrtss） | **1,000**（25 種） | 295（11 種） |
| 0075 の命令サイト（標本内） | — | 1,289 | 8,986 | 1,101 |
| bit_exact のケースで**実行された**サイト / 関数 | — | 604 サイト / 104 関数 | 3,935 / 673 | 240 / 218 |
| bit_exact のケースで実行された**ニーモニックの種類** | — | **19 / 26** | **23 / 26** | 8 / 11（`pmuludq` と `psubb` を含む） |
| 標本に含まれるが bit_exact で実行されなかった種類 | — | movmskpd、pmuludq、psubb | pmuludq、psubb | pcmpgtw、pslldq、psrldq |
| 標本に**無い**種類 | — | movnti、psubq、rsqrtps、rsqrtss | movnti | — |
| **CPU 振り分けスタブ（0079）を含む関数** | **0**（スタブ 434 は全部除外: 間接 jmp） | **0** | **0** | **0** |
| **MSVC switch（0094/0095）を含む関数** | **0**（含む 4〜5 関数は全部除外） | **0** | **0** | **0** |
| VEX/EVEX を含む関数 | 3,112 | 161 | 0 | 0 |
| SSE3 以降の形を含む関数 | 1,325 | 71 | 139 | 17 |
| `fle_unsupported` を持つ関数（自身 / 呼び先経由） | 4,504 / 5,732 | 236 / 297 | 134 / 134 | 17 / 17 |

**FLEngine と同じ判定基準では、0079 のスタブも 0094/0095 の switch も標本に入らない**。これは FLEngine の `a5-oracle-rerun-3.md` §9 と同じ型の空白で、DLL に替えただけでは埋まらない。スタブは `.pdata` の開始でもない（`.pdata` の関数は 16 バイト手前の初期化 `call` から始まり、その先が cpuid に届く）。0075 は DLL に替えたことで**初めて標本に入った**（FLEngine では関数本体が不変で検査にならなかった）。

### 5.10 拡張 E1: CPU 振り分けスタブ（決定 0079）、セレクタ値を固定して走らせる

**方式**: スタブ（`movsxd rax,[V]; lea r10,[T]; jmp [r10+rax*8]`）の入口は lift の関数入口だが `.pdata` の開始ではないので、スタブのアドレスを直接呼ぶ。セレクタ変数 V のファイル上の初期値は 4 つとも `0xffffffff`（**実測**）。V = −1 のままだと native は初期化 `call`（cpuid でホストの CPU に依存）に入り、lift は `fle_jumptab_miss` で止まるので、**両側のマップ済み像で V := k に書き換える**（`ORACLE_PATCH`、4 バイトの XOR。`apply_patch` は native も lift も同じ `oracle_main` で呼ぶ）。native は実表で `jmp [r10+rax*8]` を実行し、lift は生成された `switch` を実行する。到達先（表の k 番目）が純計算プールにある組だけを走らせ、入力は到達先の引数情報（`argptr`）で作る。

**静的な確認（実測）**: 434 スタブすべてで、生成 C の `switch` のケース（スロット VA → 呼び先）が PE の表の実バイト（スロットの qword）と一致し（不一致 0）、スロットは `lea` の表から連続（`dsp_stubs.py`）。表の項目数は 6 が 376、7 が 57、9 が 1（決定 0079 と同じ）、セレクタ変数は `0x4541000`（377 スタブ）、`0x45410a0`（20）、`0x455d050`（33）、`0x454f270`（4）。

**結果（実測）**: 全 (スタブ, k) 2,664 組のうち到達先が純計算の組は **746（28.0%、170 スタブ）**。11,936 ケース（746 × 16）: bit_exact **8,677**、mismatch **0**、lift_unsupported 1,930（VEX が大半: Vmovsd 422、Vmovss 318、Vxorps 258、Vmovdqu 123、Vmovupd 113。Movddup 101 ほか）、native_fault 1,329。

| k | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|---|
| bit_exact | 1,750 | 1,716 | 1,763 | 1,319 | 1,040 | 1,026 | 63 |
| lift_unsupported | 62 | 66 | 82 | 560 | 566 | 562 | 32 |
| native_fault | 220 | 218 | 203 | 249 | 218 | 220 | 1 |

167 スタブが bit_exact のケースを持ち（走った 170 のうち）、711 組で bit_exact、到達先の関数は 691 種類（うち 0075 のニーモニックを含むのは 1、legacy SSE を含むのは 67、VEX を含むのは 126）。**読み**: k = 3〜5 で lift_unsupported が約 560 に増えるのは、表のそのスロットが AVX 以降の実装を指すため。どの k がゲストの CPUID で選ばれるかは**測っていない**（未確認）。

**ネガティブコントロール（実測）**: lift 側だけセレクタを k2 = (k+1) % 6 にして、native の k の結果と比べた（表の項目が違う 100 組）。1,395 ケース中 **1,103（79.1%）で差が出た**（うち lift が ok で戻らないもの 319）、100 組中 96 組で 1 ケース以上検出。検出 0 の 4 組（`0x27439a0` k2→3、`0x273c660` 3→4、`0x27438b0` 4→5、`0x273c630` 5→0）は到達先が違う関数でも出力が同じだったもの（読み: 等価な到達先）。この確認は、(a) セレクタの書き換えが native にも lift にも効いていること、(b) 表の項目を取り違えたら検出できること、を示す。

**限界**: 434 スタブの 61% は走っていない（到達先が純計算でない）。範囲外のセレクタ値（k ≥ N）と −1 の経路は走らせていない（native は不定、lift は名前付き停止）。

### 5.11 拡張 E2: MSVC switch（決定 0094）を含む関数

純計算の判定では除外される 4 関数（`0x37b2510`、`0x37c6990`、`0x37cc7e0`、`0x37d9630`。5 サイト）を、**判定基準の外で強制して**走らせた（`dsp_run_msvc.py`）。それぞれ別の障害を持つ（実測）: `0x37cc7e0` は lifter が解決できなかった間接 jmp が他に 3 つ（`msvc_rejected`: `0x37cca52`、`0x37ccb82`、`0x37cccf6`）、`0x37d9630` は 1 つ（`0x37d972c`）、`0x37b2510` は直接 jmp の先 `0x37b2761` が lift 済みの関数入口でない、`0x37c6990` は `in` ニーモニック（線形逆アセンブルの結果）を含む。ケース: `gen_cases` の 16 + 添字掃引（非ポインタ引数を全部 v にした 26 ケース、v = 0..25）= 関数あたり 42、計 168。

| 関数 | bit_exact | native_fault | lift_stop_other |
|---|---|---|---|
| 0x37b2510 | 0 | 42 | 0 |
| 0x37c6990 | 1 | 41 | 0 |
| 0x37cc7e0 | **42** | 0 | 0 |
| 0x37d9630 | 2 | 1 | **39**（`indirect call to 0x37d97e2 is not a known function entry` ほか。呼び先が関数入口でない間接 call） |

mismatch は 0。サイト別（native の単一ステップの実行 rip で数えた）:

| サイト | arm | 到達したケース（全部 bit_exact） | 実行 rip に入った arm の先 |
|---|---|---|---|
| **0x37cc80b**（`0x37cc7e0`） | 16 | **31** | **16 / 16** |
| 0x37c69eb（`0x37c6990`） | 11 | 1 | 3 / 11 |
| 0x37b25bd（`0x37b2510`） | 10 | 0 | 0 |
| 0x37d9670（`0x37d9630`） | 15 | 0 | 0 |
| 0x37d986a（`0x37d9630`） | 16 | 0 | 0 |

**読み**: 「arm の先が実行 rip に入った」は、switch で飛んだことの証明ではない（前の arm からのフォールスルーで届くことがある）ので上限。d618380 のコミットログ（決定 0094: `0x37cc80b` で 16 エントリ + 範囲外 8 ケースについて native と生成 C が一致）と矛盾しない（そのログを読んだだけで、その測定は再現していない）。**3 サイトは bit_exact のケースが届いていない**: 検査になっていない。

## 6. 限界

- **FLEngine と同じ判定基準の純計算に限る**（DLL の純計算 19,635 件 = 候補の 63.7%）。seed 1・2・3 の関数の重なりを除いた件数は集計していない（seed 3 は seed 2 のプールの部分集合に当たりうる）。標本は小さい: seed 1 は純計算の 5.1%、全 32,648 関数の 3.1%。`bit_exact` は「その入力で一致した」であって関数全体の等価性ではない。
- **native_fault が多い（seed 1: 62.2%、seed 2: 37.6%）**。FLEngine は約 15%。断片（SizeOfProlog 0）、整列違反、構造体を辿る関数が主因。比較不能なケースは何も主張しない。関数単位でも seed 1 で 436、seed 2 で 177 関数は全ケースが native_fault。
- **lift_unsupported（seed 1: 7.6%）は未検査**: VEX 1,003 ケース・SSE3 以降 193 ケース・`pushfq` 24 ケースは、その命令の lift を検査できていない。
- **時間の上限**: 2 秒（再実行 10 秒）。lift は native の 95〜394 倍遅く、ループの長い入力は lift_timeout になる。上の再現で 92/92 が bit_exact になったが、**native が 10 秒の再実行でも戻らない 9 ケース（seed 1）と 22 ケース（seed 2）は延長して比べていない**。
- **比較しないもの**: 未定義の EFLAGS（AF 248 ケース）、GS/TEB、MM レジスタ、x87 の 80 ビット内容、MXCSR の RC/FTZ/DAZ が非既定の状態（0036、0038）。
- 並列度の違い: seed 1・2 は並列 12。FLEngine の再実行は 16。lift_timeout の数（アラーム 2 秒・10 秒）は負荷に依存する。
- リベース（§2.1）の限界。AMD Ryzen 9 9950X の 1 台のみ（`lscpu` は再確認していない。rerun-3 と同じ機械）、Intel 実機との比較は**未確認**。
- 0075 の `rsqrtps`/`rsqrtss` は近似でホストの結果を写す設計（0075）。seed 2 で実行された（`rsqrtps` 26 ケース・5 関数、`rsqrtss` 7 ケース・3 関数）が、Intel での一致は**未確認**。
- E1・E2 は判定基準の外の拡張。E2 は 4 関数だけ。

## 7. 未確認

- 決定 0095 の DLL の新しい switch（`0x37cd3eb`、関数 `0x37cd2c0`）を native と lift で比べていない。生成 C は HEAD の lifter で作り直して比較した（§3.2）が、`lift_drv2` の作り直し（`parts/` の .o が約 1.3 GB、リンク先が約 600 MB）がディスクの空き（測定終盤で 220〜300 MB）に入らなかった。
- どのセレクタ値 k がゲストの CPUID で選ばれるか、vex/sse3p の関数がゲストの実行経路で到達するか。
- `pushfq` が lift されない理由。
- 5002d98 の lifter の生成 C から作った `lift_drv2` と、HEAD の lifter の生成 C から作った `lift_drv2` の挙動が、走らせた関数で同じであること（関数本体が同一であることから読んだだけ）。
- 3 サイト（`0x37b25bd`、`0x37d9670`、`0x37d986a`）の switch の検査。
- native の `ret` 落ちが SizeOfProlog 0 の断片であることの個別確認（`0x271d2dc` の 4 命令の例を読んだだけ）。

## 8. 置き場と注意

### 8.1 置き場

生成物はすべて `lift/out-a5dsp/`（`.gitignore` の `lift/out*/`）。約 4.2 GB（`du -sh`）。500 MB を超えるのは `gen/dsp-pdata.c`（537 MB）、`gen/dsp-pdata-head.c`（537 MB）、`build/bin/lift_drv2`（624 MB）、`build/parts/`（1.3 GB、41 個の .o）。2GB 級の単一ファイルはない。

| 内容 | 場所 |
|---|---|
| パッチ（最終） | `patches/a5dsp.diff`（1,439 行。`patches/a5dsp-trial.diff` は途中の試作で、最終版ではない） |
| この文書 | `docs-new/a5-oracle-dsp.md`（`docs-new/a5-oracle-dsp.interim.md` は 10-27 05:17 の途中保存版） |
| リベース済み DLL | `dll/dsp_ippv2_x64.rebased.dll`（`…rebased-head.dll` は HEAD の `fe-pe` で作った同一内容のもの。SHA-256 は同じ） |
| 生成 C | `gen/dsp-pdata.c`（5002d98 の lifter）、`gen/dsp-pdata-head.c`（HEAD の lifter）、`gen/fe-lift-stdout*.txt` |
| ビルド | `build/gen`（41 分割）、`build/parts`、`build/rt`、`build/obj`、`build/bin`、`build/build.txt` |
| 結果 | `build/results/{seed1,seed2b,seed3mn,stubs,msvc}/`（`a5-*.json`、`sample-stats.*`、`exec-mn.*`）、`build/results/{pure,argptr,exclude-stats,census,stubs2,pure-force,argptr-force}.json`、`build/results/pool/`、`build/results/cmp-gen-old-vs-head.json`、`build/results/fleng-check/`、`build/results/unsup-by-function.txt`、`build/results/fragments-all.txt` |
| 再現 | `build/repro1/ext-final.json`（87 件）と `.d`、`build/repro2/ext.json`（seed 2 の 5 件） |
| 失敗・途中の記録（最終の数値には使っていない） | `build/results/seed2/`（ディスク満杯で落ちた 1 回目）、`build/repro1/ext.json*`（途中）、`build/select-try.txt`（152 MB。`select_pure.py` をそのまま DLL に走らせて `.idata` なしで落ち続けたワーカーのエラー）、`build/results/smoke/`（40 関数の動作確認） |

### 8.2 作業中の事故（正直な記録）

- **ネットワーク障害（10-25 22:12 ごろ）とサーバ再起動（10-27 09:46）で 2 回中断**。1 回目は再現の途中で、2 回目は 87 件の再現のうち 69 件目まで進んだところ。
- **ディスクが満杯になった**: 開始時（10-25）の空きは 175 GB だったが、10-27 11:51 には 1.5 GB、以降は 220〜930 MB で推移した。`lift/out-a5dsp/` は 4 GB で、`out-git`（97 GB）、`out-wallB`（46 GB）、`out-wallQ`（40 GB）など他の作業のディレクトリが大きい（私のものではない。rm 禁止なので触っていない）。影響: (1) 0 バイトの case ファイルが 6 件できた（`dsp_repro.py` を直した）、(2) seed 2 の 1 回目（`build/results/seed2/`）が、切れた JSON 行（native のトレース出力）で `JSONDecodeError` になって開始 2 分で落ちた。空きが 439 MB に戻ってから `seed2b` で最初からやり直した（21 分）、(3) `lift_drv2` を HEAD の生成 C で作り直せなかった（§7）、(4) `multiprocessing` のセマフォ作成が `OSError 122` になった（FLEngine 側の同一性確認は直列実行のラッパーに替えた）。
- **seed 2 の 1 回目と再現の並行実行**: 再現（並列 4）と seed 2（並列 8）を `xargs -P 2` で並行させたが、seed 2 は 2 分で落ち、再現は単独で 3 時間 25 分走った。再現の数値は並列 4 のみの負荷。

### 8.3 規則の確認

- `rm`、既存ファイルの全体上書き、git の変更系操作、`/tmp`、sleep、nohup、末尾 `&`、emcc: **使っていない**。clang の並列は 4。巨大な単一ファイル（2GB 級）なし。
- 既存ファイルの変更は `lift/out-a5dsp/src/` の `select_pure.py`（+3 行）だけ。HEAD の作業ツリーは編集していない（`git status --short` の 5 行は私のものではない）。
- 自己申告（文言に触れうるもの）: (1) 2 つの実行を並行させるのに `printf '…\0…\0' | xargs -0 -P 2 -I{} sh -c '{}'` を 2 回使った（末尾 `&` ではない）。(2) `timeout` の語: 環境変数 `ORACLE_TIMEOUT`（ドライバ側の実在の名前）、結果の status 値 `timeout`・`lift_timeout`、Python の変数名、shell ツールの `timeout` パラメータ。`timeout` コマンドは使っていない。(3) 自分で作った新規ファイルの書き換え: `dsp_repro.py` と `dsp_exec_mn.py` を、`edit` や Python の `open(…, "w")` で部分的に直した（どちらもこの作業で作ったファイル）。`docs-new/a5-oracle-dsp.md` は `mv` で `….interim.md` に移してから新規に書いた（削除はしていない）。(4) 出力先に `cp -n` を 1 回使った（警告つき、上書きなし）。以後は `cp --update=none`。(5) `lift/out-a5-head/` は読み取りのみ（FLEngine の生成 C の `new-part-00.c` と `pure.json`）。
