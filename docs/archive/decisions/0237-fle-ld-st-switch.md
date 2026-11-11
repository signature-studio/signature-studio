# 0237: fle_ld / fle_st を n の switch にする — 可変サイズの memcpy(wasm では呼び出し)を定数サイズの load/store に。語義は不変 — 仮の決定(rt1)

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0237〔仮番号〕、総計 3 時間)、作業者 `rt1`(2026-11-05 04:15〜)。基準は HEAD **2a26d5a6** の `git archive` に、その時点で git に入っていなかった `lift/harness/seeds/os-fonts/*.ttf` 3 本〔PM が 6c60cc22 で追跡済み〕を足した木。差分 `rt1.diff` は HEAD **52034ca9**(最後に確認。fle_runtime.h は 2a26d5a6 から変わっておらず 4977340a にも同じに当たる)の `git archive` に `patch -p1 --dry-run` が当たる(fle_runtime.h の 1 か所と新規テスト 1 ファイル。`-` の行は 4 行で全部読んだ)。リポジトリの追跡ファイルは直接編集していない。

## 結論(先頭の数行)

- **実測したもの**: HEAD 2a26d5a6 の git archive を CI の段スクリプトで冷間ビルドした F2xH7 / F2xH5 の bundle(旧 runtime)と、`fle_runtime.h` だけを変えた新 runtime の bundle を、node 24(JSPI・仮想時計)と隔離 Chrome 156 / Firefox 158 で**実エンジンとして**実行した。この機械・模擬入力で、依頼者の機械ではない。代役・再生・推測は使っていない。
- **現状**: 旧 runtime の F2xH7 で `fle_ld` + `fle_st` は main thread の自己時間の 参照プロジェクト再生 **3.7%**(open 6.4%、全体 4.5%)、Settings タブ **6.8%**、F8 **6.1%**、Playlist ドラッグ 2.6%、idle 1.3%。F2xH5(混成が狭い)は 再生 4.6%、open 7.3%、Settings 8.4%、F8 7.0%。mt2 の非混成 9.4% より小さいが、2% を超える。`fle_ld` / `fle_st` は最終 wasm に各 1 関数。
- **変更と効果**: `fle_ld` / `fle_st` を n の switch(1, 2, 4, 8 は定数サイズの load / store、他は従来の memcpy、`fle_addr` は先頭で 1 回〔引数も順序も不変〕)にした `fle_runtime.h` の 1 か所 + テスト 1 ファイル。新旧を同時に走らせた対 5 組ずつの CPU 秒の新/旧: F2xH7 は再生 **0.948**、open 0.923、Settings **0.925**、F8 **0.896**(中央値 0.911)、参照プロジェクト全体の user 秒 0.954、parity 0.923;idle の窓は 1.004 で**差なし**。F2xH5 は再生 0.943、open 0.889、全体 user 0.932。新で `fle_ld` + `fle_st` の自己時間は 再生 3.7% → 2.1%、Settings 6.8% → 3.0%、F8 6.1% → 3.0%。
- **語義は不変**(ビット単位): 生成 C の sha256 不変、起動 84 行(F2xH7・F2xH5)・参照プロジェクト 55 行(各 5 組)・parity 532 行(F2xH7)が旧新で全行一致、a4 FRUITY 無効 942/942・895/895・5,310・1,009,737、全体テスト **194 results / 2,373 passed / 0 failed**(基準 193 / 2,369 / 0 + 新しいテスト 1 実行ファイル・4 テスト)、n = 0..8 の byte 単位の参照との比較・境界・PROT_NONE・拒否アドレスのテストと mutation **18/18 検出**。最終 wasm で本体が違うのは F2xH7 の 103,005 関数中 **11 個**(F2xH5 は 95,083 中 13 個)、サイズは **+213 B**(どちらも)、first idle(Chrome / Firefox 各 6 回交互)に退行は見えない。
- **分類と判断点**: 方針の『無駄な変換を減らす』(語義不変。FL のクラス・窓・アドレスの分岐は無い)。効果が F2xH7 でも 2% を超えるので**採る提案**。ただし全プリセットの既定の runtime を変えるので PM の判断点で、取り込むと CI の object キャッシュは 1 回冷間になる。
- **未確認**: 依頼者の機械・実機・実操作(P1〜P3 への寄与)、F2xH5 の parity と first idle、出荷形(NO_FS=1)の bundle、CI での冷間ビルド、静かな(load < 10)時間帯の組は 参照プロジェクトと Settings だけ(F8・Playlist は傾向)。詳細は §8。

## 1. 何を変えたか(語義は不変)

`lift/runtime/fle_runtime.h` の `fle_ld(a, n)` / `fle_st(a, n, v)`(生成 C と runtime の全メモリ読み書きの入口)は、`memcpy(&v, (const void *)fle_addr(a, n, 0), n)` / `memcpy((void *)fle_addr(a, n, 1), &v, n)` だった。n が実行時の値のとき(`FLE_AI` が空の単位では、インライン化されなかった呼び出しごと)、clang/emscripten は wasm の `memory.copy` を出す(emcc 6.0.12 は `-matomics` なしでも既定)。決定 0234(mt2)が node 24 の単独の測定で、n ∈ {1,2,4,8} の可変 memcpy 4.4〜5.0 ns/回に対し switch + 定数サイズは 0.9〜1.5 ns/回と測っている。

変更: `fle_addr(a, n, store)` を**先頭で 1 回、従来と同じ引数で**呼び、n が 1, 2, 4, 8 のときは `uint8_t / uint16_t / uint32_t / uint64_t` の定数サイズ memcpy(= 1 回の load/store)、それ以外(0, 3, 5, 6, 7)は従来の `memcpy(&v, p, n)` に落とす。ロードは下位 n バイトに入れ残りは 0(`v = 0` の初期化と同じ)、ストアは `v` の下位 n バイト(リトルエンディアンのホスト = wasm と x86-64。従来の memcpy と同じ前提)。`#else`(ネイティブ)側の `fle_addr` マクロには触れていない。ネイティブでも同じ関数本体が使われる(S-2)。

**分類(`docs/policies/lift-fidelity.md` の語): 『無駄な変換を減らす』**。FL のクラス名・窓の種類・アドレスによる分岐ではなく、x86 の命令のオペランド幅(n)だけを見る。(a) 余計なロジックの除去でも (b) 外部 API の契約の実装でもなく、同じ結果を出すまま wasm の `memory.copy` 呼び出しを 1 回の定数サイズの load/store にする変換の削減。新しい特別扱いは足していない。

## 2. n の調査

- 生成 C(CI の lift 段の出力、F2xH7 の混成で strip した後の 21 ディレクトリ分): `fle_ld` / `fle_st` の n は**すべて 1, 2, 4, 8 のリテラル**(`tables/n-survey-generated-c.tsv`: ld 8 = 838,225 / 4 = 256,332 / 1 = 63,898 / 2 = 27,053、st 8 = 652,450 / 4 = 136,417 / 2 = 8,364 / 1 = 38,170。F2xH5 の strip 後も同様 `…-H5-strip.tsv`。strip 前の FLEngine の fle-NN.c の取り込み直後の集計 `n-survey-fle-NN-before-strip.tsv` も 1, 2, 4, 8 だけ)。3, 5, 6, 7, 16 は 0 件。
- runtime の `.c` / `.h`(`tables/n-survey-runtime.tsv`): リテラルは 1, 2, 4, 8 のほか、**n = 5 が 1 か所**(`fle_vmap.c:432` `fle_ld(va, IH_SIZE)`、`IH_SIZE` = 5)、変数の `size` を渡す所(`fle_x87.c` の `fle_x87_fst_m`〔size = 4 / 8 / 10 のうち 10 は別の分岐〕と `fle_x87_fist`〔2 / 4 / 8〕の `fle_st(a, size, …)`)。n = 5 は default の memcpy に落ちる(テストで n = 3, 5, 6, 7 を含めて確認)。

## 3. 現状の測定: fle_ld / fle_st の自己時間の割合(F2xH7、旧 runtime)

方法: CI の段スクリプト(`ci/stage-*.sh`)で `git archive` の木から F2xH7 を冷間ビルドした bundle(`tools/rt1-ci-build.sh`; NO_FS=0・COMPRESS=none の FS 形 = node 用。wasm は NO_FS=1 の出荷形より 15 KB 大きい)の link 段だけを `--profiling-funcs` と `wasm-opt -g`(と wasm-merge -g)で作り直した**名前付き版**(出荷の wasm は名前が無いので V8 のプロファイルに関数名が出ない)を node(JSPI・仮想時計)で実行し、V8 の CPU プロファイル(1 ms)の自己時間を、g3-snap の `parity1-maincpu` と プロファイルの終端を合わせた窓で集計(`tools/rt1-prof.py`、mt2 の v8prof-h6.py と同じ合わせ方)。fle_ld / fle_st は最終 wasm に**各 1 関数**(`tools/rt1-names.py`)。

**F2xH7**(旧 runtime = 名前付き版 ciA7-n、新 runtime = ciB7-n。`tables/prof-share-A7.tsv`、`prof-share-B7.tsv`):

| 場面 | 窓 | 旧: 非 idle 時間 | 旧 fle_ld / fle_st(自己時間) | 旧 合計 | 新: 非 idle 時間 | 新 fle_ld / fle_st | 新 合計 |
|---|---|---:|---:|---:|---:|---:|---:|
| playlist | playlist (e03..playlist_mdrag_2) | 1.48 s | 0.03 s / 0.01 s | **2.59%**(0.04 s) | 1.14 s | 0.01 s / 0.01 s | **1.93%**(0.02 s) |
| settings | settings (e03..audio_7) | 13.74 s | 0.61 s / 0.32 s | **6.78%**(0.93 s) | 9.27 s | 0.16 s / 0.11 s | **2.95%**(0.27 s) |
| f8 | f8 (e03..f8_esc_5) | 30.45 s | 1.40 s / 0.47 s | **6.13%**(1.87 s) | 23.80 s | 0.44 s / 0.28 s | **3.02%**(0.72 s) |
| ref1 | ref1-open (e03..open_b) | 6.01 s | 0.28 s / 0.10 s | **6.37%**(0.38 s) | 4.68 s | 0.08 s / 0.06 s | **3.15%**(0.14 s) |
| ref1 | ref1-idle30 (closed..idle30) | 3.08 s | 0.03 s / 0.01 s | **1.31%**(0.04 s) | 2.64 s | 0.04 s / 0.00 s | **1.53%**(0.04 s) |
| ref1 | ref1-play (play_a..play_b) | 15.51 s | 0.45 s / 0.12 s | **3.72%**(0.57 s) | 14.35 s | 0.21 s / 0.09 s | **2.08%**(0.30 s) |
| ref1 | ref1-whole (wiz0..end) | 38.63 s | 1.33 s / 0.40 s | **4.48%**(1.73 s) | 33.17 s | 0.51 s / 0.26 s | **2.30%**(0.77 s) |

注意: 旧と新のプロファイルは別の時刻に取った(旧は load 23〜47、新は load 10〜13)ので、『秒』の列は wall の秒でそのまま比べられない。窓の中の割合(%)で見る。新で fle_ld + fle_st が 2〜3% 残るのは、out-of-line の関数としては残り、その中で実際の load / store をしているため。

**F2xH5**(旧 runtime、名前付き版 ciA5-n。`tables/prof-share-A5.tsv`)。混成が狭いので割合は大きい:

| 場面 | 窓 | 非 idle 時間 | fle_ld | fle_st | 合計 |
|---|---|---:|---:|---:|---:|
| playlist | playlist (e03..playlist_mdrag_2) | 1.03 s | 0.02 s 2.36% | 0.01 s 1.39% | **3.75%** |
| settings | settings (e03..audio_7) | 10.72 s | 0.64 s 6.01% | 0.26 s 2.38% | **8.39%** |
| f8 | f8 (e03..f8_esc_5) | 28.11 s | 1.49 s 5.30% | 0.47 s 1.68% | **6.98%** |
| ref1 | ref1-open (e03..open_b) | 5.04 s | 0.28 s 5.61% | 0.09 s 1.73% | **7.33%** |
| ref1 | ref1-idle30 (closed..idle30) | 2.29 s | 0.02 s 0.77% | 0.01 s 0.53% | **1.31%** |
| ref1 | ref1-play (play_a..play_b) | 15.69 s | 0.57 s 3.64% | 0.15 s 0.97% | **4.62%** |
| ref1 | ref1-whole (wiz0..end) | 35.65 s | 1.41 s 3.96% | 0.45 s 1.25% | **5.21%** |


読み: Playlist ドラッグは窓が 1.5 s しかなく、snapshot のハッシュ(`sha_update` 13.5%)が大きい。F2xH7 は混成が広いので、mt2 が非混成で測った 9.4%(fle_ld 7.7% + fle_st 1.7%、参照プロジェクトの再生)より小さい。『名前の付いた関数』は窓の 26.8〜59.4%(idle30 の窓が最小)で、残りは混成の直接 wasm(名前なし)。

## 4. 効果(新旧の対の CPU 秒)

旧(A)と新(B)の bundle を**同じ G3 を同時に**(2 つの node を同じ負荷の下で)走らせ、main thread の CPU 秒(g3-snap の `parity1-maincpu` の差)の新/旧を組ごとに出した(`tools/rt1-pair.sh`、5 組、開始順は組ごとに交代)。load は組の開始時の 1 分 load average。`tables/ab-*.tsv` に組ごとの値。

| bundle | 場面(窓) | 新/旧(平均、〔最小〜最大〕) | load | 備考 |
|---|---|---|---|---|
| F2xH7 | 参照プロジェクトの再生(play_a..play_b、仮想 10 s) | **0.948**(0.927〜0.967) | 6.6〜8.4 | 5 組すべて < 1。旧 10.46 s → 新 9.92 s |
| F2xH7 | 参照プロジェクトを開く(e03..open_b) | 0.923(0.884〜0.952) | 同上 | |
| F2xH7 | 参照プロジェクトの idle 8 s(closed..idle30) | 1.004(0.958〜1.036) | 同上 | **差なし**(5 組中 3 組が 1 以上) |
| F2xH7 | 上の全体(wiz0..end) | 0.947(0.928〜0.960) | 同上 | プロセス全体の user 秒(起動を含む)は 0.954(0.947〜0.961) |
| F2xH7 | Settings タブ 16 回(e03..audio_7) | **0.925**(0.909〜0.941) | 5.9〜9.5 | 全体 user 0.946 |
| F2xH7 | F8 を 6 回(e03..f8_esc_5) | **0.896**(中央値 0.911、0.824〜0.923) | 11.0〜11.2(4 組)、28.9(1 組) | 負荷の高い 1 組(0.824)を除く 4 組の平均 0.914。全体 user 0.917 |
| F2xH7 | Playlist 中ボタンドラッグ 3 回(e03..mdrag_2) | 0.913(0.891〜0.938) | **34.6〜50.3(静かな時間帯ではない)** | 傾向のみ。窓が 1.3 s で短い。全体 user 0.943 |
| F2xH5 | 参照プロジェクトの再生 | 0.943(0.931〜0.956) | 11.4〜16.4 | |
| F2xH5 | 参照プロジェクトを開く | 0.889(0.862〜0.924) | 同上 | |
| F2xH5 | idle 8 s | 1.024(0.955〜1.123) | 同上 | 差なし |
| F2xH5 | 全体(wiz0..end) | 0.920(0.909〜0.936) | 同上 | 全体 user 0.932 |
| F2xH7 | parity 10 セッション(`lift/harness/parity/run.sh`、56 行) | **0.923**(10 セッションすべて < 1、0.779〜0.965) | 18〜27(ゲート自身が 20 超で警告) | 旧 129.01 s → 新 119.11 s(wasm main thread の CPU の合計)。A と B を同時に走らせた 1 ラウンド |

読み: プロファイルで fle_ld+fle_st の自己時間は再生窓で 3.7% だが、CPU の減りは 5.2% で、その割合 × (1 − 1/4)(microbench の 2.4〜4.9 倍から見積もった 3 前後)より大きい。呼び出しごとの `memory.copy`(V8 の組み込み)は呼び出し側の自己時間に入らず `(program)` などに入るためと読む(未確認)。idle の窓は fle_ld+fle_st が 1.3% で、差は測れない。

## 5. 語義が変わっていない証拠

- **生成 C は不変**(runtime だけの変更): `fe-lift lift FLEngine_x64.dll X.c --pdata` の sha256 = `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B(`tables/felift-default-sha256.txt`。lifter は HEAD の木の物で、この差分は lifter のソースに触れない)。
- **a4(FRUITY 無効、`a4-fruity.sh off`、新しい木)**: init_count 942/942、runnable 895/895、functions_distinct **5,310**、function_entries **1,009,737**、`all_units_done=yes`、harness exit 0(`logs/a4off-B.log`)。基準(PM)と一致。
- **全体テスト**(`lift/harness/test.sh full --jobs 3`、新しい木、冷間): **194 results / 2,373 passed / 0 failed**(基準 193 / 2,369 / 0 に対し、新しい実行ファイル `ldst_switch` の 1 results と 4 テストが増えただけ)。壁時計 721 s(他担当と同時、load 約 15〜30)。旧 runtime の木での再実行はしていない(基準は PM の値)。
- **単体テスト `crates/fe-lift/tests/ldst_switch.rs`(新規、4 テスト)**: (1) n = 0..8 × 16 オフセット × 11 個の固定値と乱数で、`fle_ld` は byte 単位の参照(下位 n バイトにリトルエンディアン、残り 0)、`fle_st` は『ちょうど下位 n バイトだけ』(周囲のバイトを変えない)と一致。native の -O0 / -O2 / -Oz / `-DFLE_ALWAYS_INLINE` と wasm(emcc -O2 / -Oz / always-inline、node)。n は volatile を通して実行時の値にした。(2) 64 KiB 境界で終わる / またぐ / 始まる 88 通り。(3) native の PROT_NONE ページの直前で終わる 1〜8 バイトが落ちず、1 バイト手前から始まって境界をまたぐ n = 2..8 の load / store が SIGSEGV(= 必要な幅だけを読み書きする。n = 1 は落ちない)。(4) `FLE_CHECK_HI32` の拒否アドレス(4 GiB 以上)が n = 1..8 の load / store とも止まり、メッセージの `(N-byte load|store)` が変わらず、止まる前後で領域が変化しない。旧ヘッダでも (1)(2) が通る(native 4 設定と wasm -O2 を手で確認。テストが byte 単位の参照と比べているだけで、新旧の差を見分けるのではない)。
- **mutation 18 個**(`tools/rt1-mutation.py`、`tables/mutation-ldst.tsv`): 符号拡張(n = 1, 2, 4)、n = 4 を 3 バイトで読む、n = 8 のマスク、default の欠落 / n−1、n = 2 を 4 バイトで読む(過剰読み)、`fle_addr` の n / store フラグ違い(ld と st)、st の幅違い(1→2、2→1、4→8、8→4、上位半分)、default の書き込み欠落。**18/18 検出**。最初の版は『n = 2 を 4 バイトで読んで下位を返す』が生き残ったので、PROT_NONE のテスト(3)を足した。
- **起動 16 スナップ 84 行**(`cmp-short.g3`、node): F2xH7・F2xH5 とも旧 = 新で全行一致(sha256 `9cde773f…`〔H7〕。ship5 が記録した F2xH5 / F2xH7 の 84 行のファイルとも全行一致)。
- **参照プロジェクト 55 行**(`rt1-ref1-perf-short2.g3`、ref3 の seed): F2xH7・F2xH5 の 5 組ずつすべて旧 = 新(55 行)。
- **parity 10 セッション 532 行**(F2xH7、1 ラウンド): 旧 = 新で全行一致(`tables/parity-identity-H7.txt`)。ゲートは両方 56 行、0 FAIL、1.2 倍の目標を超える行 0(CPU は load が 20 を超えたと警告が出るので参考)。F2xH5 の parity は実行していない。
- **最終 wasm の関数本体**(`tools/rt1-wasmdiff.py`): F2xH7 は 103,005 関数のうち**11 個だけ**、F2xH5 は 95,083 関数のうち 13 個だけが違い、残りはバイト一致。違う関数: `fle_st`(57→132 B)、`fle_ld`(68→135 B)、可変サイズの `fle_st` を使う `fle_x87_fst_m`(+73)と `fle_x87_fist`(+76)、`fle_imp_PathCleanupSpec`(+8)、`fle_imp_WinVerifyTrust`(−20)、C 経路の guest 関数 5 個(f_1d5c910 +3、f_1e875f0 +12、f_478d80 +3、f_634920 −92、f_10df040 +6。インライン判断の差)。F2xH5 は 13 個(guest 関数が 7 個)。
- **サイズ(S1)**: wasm は F2xH7 133,901,932 → 133,902,145 B(**+213 B**)、F2xH5 128,083,884 → 128,084,097 B(**+213 B**)。どちらも NO_FS=0 の FS 形同士(出荷の NO_FS=1 は ship5 の記録で F2xH7 133,886,854 B、F2xH5 128,068,804 B。差は FS 形 +15 KB で、足せば出荷形でも同じ +213 B のはずだが、出荷形はこの作業では作っていない)。wasm オブジェクト 145 個のうち 109 個はバイト一致、36 個が変わり合計 +4,365 B(`tables/objsize-H7-base-vs-new.tsv`)。`FLE_ALWAYS_INLINE` の単位 15 個のうち 10 個はバイト一致、5 個は −92〜+12 B(『n が定数の所は従来と同じ code に畳まれる』が成り立つ)。out-of-line の fle_ld / fle_st の複本を持つ 31 個の通常の単位は各 +150 B 前後で、最終 link で重複が 1 つにまとまり、wasm 全体では +213 B になる。
- **first idle**(隔離 Chrome 156・Firefox 158、F2xH7 の FS 形、旧 A と新 B を同じセッションで交互、各 6 回、load 9〜38、模擬入力): Chrome の中央値 A 8.36 s / B 8.08 s(組ごとの B−A の中央値 +0.03 s、B が遅い組は 6 組中 3 組)、Firefox A 8.24 s / B 7.98 s(B−A の中央値 −0.45 s、遅い組は 0 / 6)。**退行は見えない**が、ばらつきが ±1〜2 s あり小さな差は分解できない(`tables/first-idle-H7-summary.txt`)。F2xH5 の first idle は実行していない。

## 6. 他の可変サイズ memcpy

プロファイル(F2xH7 の旧 runtime、上の 7 窓)で自己時間 ≥ 1% の runtime 関数のうち、可変サイズの `memcpy` / `memmove` を持つ物を調べた(`fle_*.c` / `fle_*.h` の全 memcpy / memmove の引数を読んだ)。

| 候補(窓での最大の自己時間) | 可変サイズの memcpy | 判断 |
|---|---|---|
| `fle_ld` / `fle_st`(2.6〜6.8%) | あり(今回) | 変更済み |
| `fle_x87_fst_m`(再生 1.95%)、`fle_x87_fist` | `fle_st(a, size, …)` の size が変数(4 / 8、2 / 4 / 8) | `fle_st` の switch に含まれる(最終 wasm の本体が +73 / +76 B になった 2 関数がこれ) |
| `fle_r_blit`(F8 7.3%、Settings 2.8%) | 行ごとの `memmove(to, from, w * nb)` は**バルクのコピー**(幅は画素数 × 画素バイト)。画素ごとの `memcpy(&v, p, 4)` は定数 | 対象外(1 回の呼び出しで多数のバイトを動かす物で、定数サイズにできない) |
| `fle_mx_*`(`fle_mx_mulps` 5.4% など)、`round_pack`、`binop`、`fle_op_*`、`fle_cc` | `sizeof` か 16 / 8 / 4 の定数だけ。`fle_unpack` の `esz` は変数だが、出荷構成(`FLE_WASM_SIMD=1`)では SIMD の shuffle の分岐が先に return する | 対象外 |
| `fle_imp_acm_StreamConvert`(open 8.3%)、`fle_imp_VirtualAlloc` | バルクのコピー / API 本体 | 対象外 |

fle_runtime.h 以外は触っていない。この表の『対象外』は ≥1% の関数に同形の可変サイズ memcpy が無いという意味で、1% 未満の関数は調べていない。

## 7. 出典のない選択(S-)

- **S-1 switch か if の連鎖か**: n で `switch`(1, 2, 4, 8 + default)にした。代替: 頻度順の if(n == 4, 8, 2, 1)、`__builtin_constant_p` で分ける、`fle_ld1/2/4/8` に分けて生成側(fe-lift)で選ぶ。switch は mt2 が microbench で測った形で、LLVM が wasm では `br_table` か比較の連鎖にし、生成側を変えないので生成 C が不変のまま(sha 不変)。if の連鎖との比較は測っていない。
- **S-2 ネイティブにも同じ本体**: `#ifdef` で wasm だけにせず、native も同じ switch。代替: wasm だけ。native のテスト(clang -O0 / -O2 / -Oz、PROT_NONE の境界)が wasm と同じ本体を検証でき、native の結果はビット単位で変わらない。native の速度は測っていない。
- **S-3 default は従来の memcpy**: n = 0, 3, 5, 6, 7 は `memcpy(&v, p, n)`(n > 8 は従来どおり未定義)。代替: 3, 5, 6, 7 も個別に。runtime の n = 5(`fle_vmap.c`)が 1 か所あるだけで、生成 C には無い。
- **S-4 テストは Rust の統合テスト + emcc / node(無ければ skip)**: 既存の `wasm_port.rs` / `gs_stacklimit.rs` と同じ形。代替: `lift/harness/` のシェル。全体テスト(`test.sh full`)に入るので採った。
- **S-5 新旧を『同時に』走らせる対**: 『交互に』ではなく、2 つの node を同時に起動して同じ負荷の下に置いた(CPU 秒は main thread の `threadCpuUsage`)。代替: 交互の逐次。同じ負荷が掛かる利点と、SMT / キャッシュを2 プロセスで取り合う欠点がある(新旧とも同じ条件なので比は残ると読むが、単独で走らせた値ではない)。開始順は組ごとに交代。
- **S-6 名前付き版は link 段だけを作り直す**: `--profiling-funcs`、`wasm-opt -g`、`wasm-merge -g`(出荷の wasm は名前が無い)。同じ objects・同じ C 部で、コードは同じ(名前の節だけ増える。wasm は 142,674,014 B)。サイズの比較には使わず、出荷形の bundle の値を使った。代替: 名前付きの全面ビルド。
- **S-7 NO_FS=0・COMPRESS=none の FS 形**: node の道具は FS 形が要る(`NO_FS=1` の出荷形は hostFiles を使う)。ブラウザの first idle も同じ FS 形で測った(出荷形ではない)。wasm は出荷形より 15 KB 大きい。
- **S-8 プロファイルの窓**: g3-snap の `parity1-maincpu`(main thread の累積 CPU 秒)とプロファイルの終端を合わせた(mt2 の v8prof-h6.py と同じ)。harness の snapshot のハッシュ(`sha_update*` / `g3_*`)は別行に出し、合計は両方を載せた。窓の端は snap 名で決めた(再生は play_a..play_b など)。
- **S-9 基準の木**: HEAD 2a26d5a6 の git archive に .ttf 3 本を足した木(PM が 6c60cc22 で git に入れた物と同じファイル。sha は extract 段が検査)。差分は HEAD 4977340a に当たる(fle_runtime.h は 2a26d5a6 から変わっていない)。
- **S-10 参照プロジェクトの seed**: ref3 の `vfs-seed-flat.tsv`(フラット配置、`C:\Users\user\Downloads\serina1-serum2.flp`)。ref1 の seed(入れ子配置 + `oggio_x64.dll`)では出荷構成の bundle が `LoadLibraryW … is not a module of this build`(決定 0233)で止まった。代替: ref1 の seed + `FLE_SOFT_STOP=1`。
- **S-11 pair の回数**: 各 5 組(目標は静かな時間帯の組 ≥ 4)。静かな(load < 10)組は 参照プロジェクト 5、Settings 5、F8 は 0(load 11)、Playlist は 0(load 35 以上)。F8 と Playlist は『静かな時間帯の組』ではない。

## 8. 未確認

- **依頼者の機械・実機 Windows・実ブラウザの実操作は 0 件**(すべて模擬入力と node の仮想時計)。P1(起動 10 秒以内)・P2(120 fps)・P3(音切れなし)への寄与は測っていない。CPU の減り(再生で 5%、Settings・F8 で 7〜10%)が UI の応答や fps にどれだけ効くかは未確認。
- **F2xH5 の parity、first idle(Chrome / Firefox)**、**出荷形(NO_FS=1)の bundle のサイズ**、**CI での冷間ビルド**(この変更で fle_runtime.h を含む全ての C 単位が再 compile され、CI の object キャッシュは 1 回冷間になる)は実行していない。
- 静かな(load < 10)時間帯の組は 参照プロジェクトと Settings の各 5 組だけ。F8 は 4 組が load 11、Playlist は load 35〜50 で、これらの比は傾向である。idle の窓の 1.004(H7)・1.024(H5)は『差なし』で、わずかな増えも否定できない(新/旧が 1 以上の組は F2xH7 で 3 / 5、F2xH5 で 4 / 5。idle の窓は fle_ld + fle_st が 1.3% しかなく、この窓は『差なし』と読む)。
- PM への第 2 報で書いた『全 25 組で新 < 旧』は誤りで、正しくは idle の窓以外の全ての組(F2xH7 の再生・open・全体・Settings・F8・Playlist の 5 組ずつ、F2xH5 の再生・open・全体)で新 < 旧、idle の窓だけが上のとおり。
- 『呼び出しごとの memory.copy が(program) などに入るので、自己時間の割合より CPU の減りが大きい』は読みで、確かめていない。
- n が 1, 2, 4, 8 以外の呼び出しは、生成 C には無く、runtime に n = 5 が 1 か所(`fle_vmap.c`)と変数 size(x87)があるが、x87 の size = 10 は `fle_st(a, 8, …)` と `fle_st(a + 8, 2, …)` の定数の呼び出し(読み)。`FLE_CHECKED_MEM`(wasm のオプション)の経路はテストしていない(`fle_addr` を呼ぶ位置と引数は変えていない)。
- 旧 runtime の木での全体テストの再実行(基準 193 / 2,369 / 0 は PM の値)、parity の複数ラウンド、F2xH5 の名前付き版でのプロファイルは F2xH5 の新 runtime の名前付き版でのプロファイルは取っていない(旧のみ)。

## 9. 再現

`git archive HEAD` の木に `patch -p1 < rt1.diff`。CI の段(`tools/rt1-ci-build.sh TREE CIWORK F2xH7|F2xH5`、`tools/rt1-build-bg.sh` が NO_FS=0 COMPRESS=none を付ける)で bundle を作り、`tools/rt1-g3.sh`(84 行)、`tools/rt1-pair.sh`(新旧の対。`rt1-node.sh` が node を起動)、`tools/rt1-abtable.py`、`tools/rt1-names.sh`(名前付き版)+ `tools/rt1-profs.sh` + `tools/rt1-prof.py`(プロファイル)、`tools/rt1-parity.sh`、`tools/rt1-a4off.sh`、`tools/rt1-wasmdiff.py`、`tools/rt1-objsize.sh`、`tools/rt1-ldst-bench.sh`(microbench)、`tools/rt1-mutation.py`、`tools/rt1-idle.sh`(ブラウザ)。詳細は `tools/README.txt`。microbench(node 24、emcc -O2、n が定数でない noinline 呼び出し、旧 / 新を交互 5 回の中央値): ld(n = 8, 4, 2, 1 の巡回)5.37 → 1.10 ns、st 4.28 → 1.76 ns、ld n = 4 固定 5.53 → 0.41 ns、st n = 4 固定 4.13 → 0.51 ns(`tables/ldst-microbench.txt`)。

## PM の決定と取り込み(2026-11-05)

取り込んだ基準: HEAD 52034ca9 に `rt1.diff`(fle_runtime.h 1 か所 + 新規テスト `ldst_switch.rs`、`-` 4 行は全部読んだ: `fle_ld` / `fle_st` の `memcpy(…, n)` の本体だけ。offset・fuzz・reject なし)と `load2-guard.diff`(決定 0238)を `patch -p1`。PM が測った値(この機械、`lift/out-pmchk-r2/`): FLEngine の既定の生成 C の sha256 `cd9107ce…`(215,811,678 B)不変、全体テスト **196 results / 2,386 passed / 0 failed / ignored 1**、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737。

1. **採る(既定の runtime を変える)。** 分類は『無駄な変換を減らす』(語義不変、x86 のオペランド幅 n だけを見る。FL のクラス・窓・アドレスの分岐なし)。担当の実測(この機械、模擬入力): F2xH7 の再生 CPU 秒が新/旧 0.948、Settings 0.925、F8 0.896、idle は差なし、wasm +213 B、起動 84 行・参照プロジェクト 55 行・parity 532 行は旧新で全行一致、mutation 18/18。
2. **取り込むと CI の object キャッシュが 1 回冷間になる**(runtime の全 C 単位が変わる)。次の CI の実行で F2xH7 の bundle は冷間ビルド(約 38 分)。出荷 wasm は +213 B 前後で変わるはず(CI の実行後に確認する)。
3. 依頼者の機械・Firefox 159・静かな時間帯の組は未測定(§8)。効果は再生 −5%・Settings −7.5% で P2/P3 の主要因ではない(主要因は別: 決定 0234 / 0240)。
