# 0315(仮): RT-1 — 実スレッド版を本番に入れる方向の準備(現状の表、衝突、RT 候補の木と build、入力 T、依頼者の条件の再現、新しい関門)(mt3。PM の instruction 08:0x。期限 (1)(2) 09:00)

状態: **仮**。作業者 mt3、2026-11-07。区分: **実測**=この作業のコマンドの出力を見た事実、**引用**=他者の測定を読んだだけ(出典を付ける)、**読み**=コードを読んだだけ、**未確認**。**実エンジン(wasm)のブラウザでの実行は 0 件。Chrome は起動していない。依頼者の機械は 0 件。**

## 0. 冒頭

1. **実際に実行した物(実測、すべて host の道具。engine ではない)**: (a) `apply-stack-0312.sh check` を **PM の木(HEAD 0e4cff02)** に対して実行: 5 本すべて rc 0・**reject 0**。(b) 同じ stack を **cand1 の木(tree-g9f-cand1)** に対して実行: diff 1(rt-on-e11)の 2 hunk だけが reject(`fle_env.c` の fle_stop、`fle_win.c` の include 行。どちらも cand1 の patch が文脈を動かしただけ)、他の 4 本は clean。2 hunk は手で当て(下の §2)、木 **tree-rt1** = cand1 + stack + S3a を作った。(c) その木で `emcc -c -Wall -Wextra -DFLE_GIL -DFLE_TLS_MORE -pthread` を fle_env・fle_win・fle_thread・fle_thread_wasm・fle_sys・fle_sysinfo・fle_audio_transport・fle_gil・fle_callback・fle_vfs・fle_wall_ia2w で: **警告・エラー 0**。(d) page 側: tree-rt1 に P(0311 の src・test・tsconfig)+ V7 を当て(reject 0、main.ts は offset 2 行)、**tsc 3 構成 rc 0、vitest 71 files・1,323 passed・7 skipped・0 failed**(7.1 s)。(e) cand1 → rt1 の差分を 1 本に出す道具(`rt1-tree.sh emit`)は動いたが、**出力は 400 KB を超える**(RT-A の積みが増えたため 103 ファイル・415,617 B)ので置かない。既存の差分(上の表)と `rt1-tree.sh` が正本。
2. **まだ 0 の物**: engine の wasm を作っていない(build は cand1 が走行中)。native の `test.sh full`・`cargo test` は tree-rt1 で未実行。ブラウザの実行・T・PCM は未測定(下の数字はすべて他者の測定の引用)。
3. **結論(読みを含む)**: (i) RT 候補は **1 回の冷間 build**(約 55〜65 分、CI_JOBS=1)で作れる。cand1 の lift 出力は stack の fe-lift の変更でさらに変わるので、objects の再利用は無い。(ii) **T の最小の差分(S2)は、証拠のある物が今は無い**: 私が立てた『P の要求で engine の idle を起こす』仮説は、gpu1 の V5 が既に試して効果なしだった(§5)ので**取り下げ**。最初の一手は新しいコードでなく、**RT の bundle + E14 の page(P+V7)をそのまま ci1 が測る**こと。(iii) **B1(n = 1)だけで新しい関門の T は満たす見込み**(File +9.9、Esc +8.7 ms、引用)だが、マージンは 0.1〜1.3 ms で薄い。本命の効き(O17)は『FL のミキサーが複数スレッドに割れること』= S3a の n > 1 で、**これは一度も走っていない**(§6)。

## 1. 現状の表: 実スレッド版を作る差分と、main(E13)・E14・cand1 に対する状態

基準: PM の木 HEAD **0e4cff02**(`git diff 5b49dcb3..HEAD -- lift app ci tools` は空 = docs だけの変更。E13 のコード = HEAD のコード)。

| 部品 | 中身 | 置き場(`lift/out-git/…`) | HEAD に対して(実測) | cand1 に対して(実測) | 測定(引用) |
|---|---|---|---|---|---|
| S0 の 1 | `rt-on-e11.diff`: 実スレッドの足場(GIL、Worker、JSPI の scheduler、`fle_gil*`、生成 wrapper) | mt3w-results/0301/impl | clean | **2 hunk reject**(fle_env.c の fle_stop、fle_win.c の include。手で当てて解決) | E11 の実スレッド(PCM の零 19.63%) |
| S0 の 2 | `0305-fix-the-16-failures-on-rt-on-e11.diff` | 0305/impl | clean(offset 1 hunk) | clean(offset 1) | — |
| S0 の 3 | `0312-s0-fix17-fruity-diagnostic.diff`(足場の診断ログを除く) | 0312/impl | clean | clean | — |
| S0 の 4 | `0305-d3-remove-instrumentation.diff`(D3a: 計測の旗を除く) | 0305/impl | clean | clean | — |
| S1 | `0312-s1-stage-c.diff`: 音声リング・wr・rdHi を wasm の共有メモリへ(`fle_tp_*` を C に) | 0312/impl | clean | clean | **B1 `fl-engine-ci-8301687b`**: 5 分再生 PCM の零 3 区間・790 フレーム(0.036%、協調 0.038%)、distinct 0.978〜1.000、12288 の割合 0〜2%、CPU/Unlock 最大 66/74 ms、UI Worker 平均 0.583、2 Worker 最大 1.34 コア(gpu1、0311・performance-targets 4e)。T: File 28.0 対 協調 18.1、Esc 13.5 対 4.8 ms |
| S3a | `0312e-s3a-host-cpus-env-kept.diff`: GetSystemInfo/GLPI/affinity が host の `hardwareConcurrency`(1〜32)を返す、Worker 数 `clamp(n−1, 2, 8)`、`FLE_WORKERS`/`FLE_PROCESSORS` は残す | 0312/impl | clean | clean(stack の後) | **B2 `fl-engine-ci-8b2eb5d7`**(wasm 142,986,341 B): ビルド・native・node テストだけ。**ブラウザでの実行 0 件。env なし = n 16・N 8 は一度も走っていない構成** |
| S3b | `0312e-s3b-env-removed.diff`(env を除く最終形) | 0312/impl | S3a の後 | 未テスト | 測定の後 |
| S4 | 旗の除去・協調 scheduler と `rt_mode` の分岐の整理 | **未作成** | — | — | T の関門の後(決定 0312) |
| E14 | P(present を別 Worker): `0311-present-worker-{src,test}.diff` + `0311-tsconfig.diff` + V7 | gpu8-results/diffs | (page のみ。HEAD の app/ に clean〔offset 2〕) | clean(offset 2) | 協調ページ上: P(修正後)= 純再生 21→34/s、File T 17.8→18.4、Esc 4.2→6.0 ms。P+V7 = File +1.7、Esc +1.0 ms、Esc の late 8/147 対 基準 1/110(基準の『同数』は未達)。E14 の最終の組 = P+V7(V4 は不採用、決定 0311)。**B1+P(V7 なし): B 1.2→0.4 ms、present 21→50/s、File T 10.0 ms(協調 17.6)、Esc 23.7 ms(協調 4.4)、Esc の応答 67/70**。**B1 + P + V7 は未測定** |

- **main に足りない物**(RT を製品にするまで): (1) 上の stack + S3a が main に無い(決定 0312: T が関門を通るまで main に出さない → 今回その関門を変えた)。(2) S3b + S4(旗の除去)。(3) 測定が無い物: n の掃引(2/4/8/16)、既定 n = 16・N = 8、first idle(P1)、Export の sha が n に依るか(E13 の `381cdb5a…f979`)、Firefox、1〜2 コアの host、依頼者の機械。
- **E14 の page 側が RT で前提とする物**: P は engine の `WebAssembly.Memory` を受けて scene の記述を読む。RT の build では memory が共有(pthread)で、0312b §2-4 は『S0+S1 では共有メモリで P の hand-over が成立』と読んでいる(読み。実エンジンでは B1+P の測定〔gpu1〕で動いている = 引用)。

## 2. 衝突(実測)

| 組 | 結果 |
|---|---|
| stack ↔ HEAD(PM の木) | 5 本 rc 0・reject 0 |
| stack ↔ cand1 | 4 本 clean。rt-on-e11 の **2 hunk が文脈の不一致**: (1) `fle_env.c` の `fle_stop`: rt-on-e11 が `#ifdef FLE_GIL` の `fle_dbg` を `fprintf` の前に挿入、cand1 の `0319-fle-stop-context.diff` が同じ関数の頭に 2 行(`sp`/`ra`)を足していて文脈が合わない。(2) `fle_win.c` の include: rt-on-e11 が `#include "fle_gil.h"` を `fle_thread.h` の次に挿入、cand1 の `diag-g9-fl.diff` が `fle_test_env.h` をその間に入れた。**どちらも両方の挿入を残して解決**(`tools/rt1-tree.sh` の `build_tree` が python で当てる)。意味の衝突は無い。 |
| stack ↔ cand1 の runtime の意味 | alpha1 の fle_win_scroll・時刻/env・ci1 の 0323(背景の消去・HTTRANSPARENT)・d2 は、**すべて runtime 関数の中の状態**で、実スレッド版では `fle_gil_wrap`(生成)が guest からの入口で GIL を取る = 同じ規則の下に入る〔読み〕。ScrollWindow の import 行(ship6-imports.txt)は生成 wrapper の再生成で自動的に GIL 付きになる〔読み。lock-free の集合 `gen_wrap.py:7-17` に入っていない限り〕。 |
| P/V7 ↔ stack | textual 0(0312b)。tree-rt1 でも clean。tsc 0・vitest 1,323 passed(実測) |
| P/V7 ↔ cand1 の d1(page: `main.ts`・`screen-fit.ts`) | clean(main.ts は offset 2)。tsc 0・vitest 全部通る(実測) |
| 意味の衝突(読み) | (1) P の `servicePresent` は Worker 0 の scheduler 点と idle でだけ呼ばれる(`worker.ts:169-171,239-240`)。RT の Worker 0 は UI スレッド 1 本で、idle の待ちは `Atomics.waitAsync`(`fle_idle_lib.js` の `fle_idle_wait`)。B1+P の Esc の遅れはここの待ちが疑われたが、**V5 が Worker 0 の event loop で `servicePresent(false)` を直接呼んでも効かなかった**(引用)ので、原因は engine 側の『読まれなさ』でなく **P 側の起床の遅れ**(publish → P の loop、2 状態)と読むほうが証拠に合う。(2) FL が processors = n を見て作るスレッド数が増えたとき、`fleThreadReport` の表で Worker に割れる数を見る(S3a の測定項目)。 |

## 3. RT 候補の木と build の見積り(PM の (2))

**木**: `tree-rt1`(`~/work/mt3w-s1`)= **cand1 の木 + 0323b-disabled-input + 0319-scrolldc-model + 0325-cursor-pictures(PM の追加 08:2x、各 dry-run rc 0・FAILED 0・offset 3/0/2)+ stack 5 本 + S3a**。再現: `lift/out-git/mt3w-results/0315/tools/rt1-tree.sh tree [P=1]`(P=1 は page に E14 を足す)。差分 1 本にまとめた物は 400 KB を超えるので置かない(`rt1-tree.sh emit` で作れる)。**FLE_WORKERS の既定が hardwareConcurrency から**: S3a が `nworkers_cfg` の既定を `clamp(n−1, 2, 8)`、GetSystemInfo の n を host の値にする(env があれば優先 = 測定用に残す)。

**build の回数と時間(読み。実績は引用)**:

| 案 | 内容 | 時間 | 根拠 |
|---|---|---|---|
| **RT-A(本命、冷間 1 回)** | tree-rt1 を `GIL=1`、`CI_JOBS=1`、`BINARYEN_CORES=1`、nice 19 で、`extract felift lift b2d objects link seed bundle` | **約 55〜65 分** | objects: GIL 付きの全 runtime + stack で変わった lift の C(fe-lift・fe-wasmgen の変更で生成 C が変わる = a4 で確認済みの 2 import 行 + dispatch 表)+ Balance/Filter の 2 module。cand1 の objects は `-DFLE_GIL`/`-pthread` でなく再利用できない。**実績(引用)**: Filter(協調)の冷間 = objects 23:00 + link 26:07 + lift 38 s + b2d 59 s = 約 51 分(CI_JOBS=1)。B1(CI_JOBS=2、GIL)= objects 53 分〔PM の SIGSTOP 約 35 分込み。引き算で正味 約 18 分〕+ link 11 分。B2(runtime 群だけ)= link 20 分 13 秒(wasm-opt 12:56、CORES=1)。GIL 版の objects は協調より重い見込み(読み) |
| RT-B(増分) | RT-A の work dir を使い、runtime の C だけ変えた再 build(S2 の C 変更や n の既定の調整) | 約 30〜35 分 | B2 の実績(objects 15 s + link 20 分 13 秒 + b2d 54 s + lift 26 s) |
| 診断の木(別) | `patch -R` の D3a(`-DFLE_GIL_STATS -DFLE_CS_HOLD_STATS`)を入れた測定用の木 | 約 30 分(runtime 群 + link) | 0312c §3。**製品に入れない**。C の内訳が要るときだけ |

- 1 回で済ませるための注意: RT-A に `S3b` は入れない(env を測定に使う)。page は **E14 あり/なしの 2 種**を別に作る(engine は同じ bundle)。n の掃引・FLE_WORKERS/FLE_PROCESSORS は manifest の `engine.env` を変えるだけ(再 build 不要)。
- **machine の都合**: cand1 の build が走行中(完了の見込み 09:00〜09:05)。同時に走らせない(メモリ・CPU)。RT-A は cand1 の後に始めて **約 10:05 完了の見込み**。順序を変えるなら未決(cand1 を止めて RT-A を先にすることもできるが、cand1 は objects の約 6 割まで進んだ)。

## 4. 入力の T を縮める S2 の最小の差分(PM の (3))

**証拠の整理(引用)**: B1(n = 1 の実スレッド): File 28.0 / Esc 13.5 ms 対 協調 18.1 / 4.8 ms。差は **File +9.9、Esc +8.7**(B が +4.8、C が +5.5)。新しい関門(協調 +10 ms 以内)に対して **File は 0.1 ms、Esc は 1.3 ms の余裕**。B1+P: B は 0.4 ms に消え、File は 10.0 ms(協調 17.6 より良い)、**Esc だけ 23.7 ms(+19.3)で、Esc の答えの present が engine の idle の約 20 ms 後**。V5(P が frame 要求と DONE ごとに engine へメッセージを送り、**Worker の event loop で `servicePresent(false)` を呼ぶ**)は Esc の T が 14.4 対 17.8 ms で効果なし。run は 2 状態(present 50/s で P の起床の遅れ 平均 7〜10 ms、34/s で 平均 1.7 ms)に分かれ、Esc の遅さは前者で出る。

**読み**: V5 は engine Worker の event loop で present の要求を**直接**さばくので、『engine が WANT を読まない』という仮説は V5 で検証済みで外れている。遅れは **P 側**(publish → P の loop の起床、animation frame の位相)にあり、これは**協調でも同じ 2 状態が出て V7 が直した層**(P 層の順序の補正)。V7 は P の内部の順序で、engine の実行モード(協調/実スレッド)に依らない → **B1 + P + V7 で 2 状態が消えるかが次の問い**(未測定)。

**したがって S2 の最小の差分(提案)**:
1. **新しいコードは足さない。まず RT-A の bundle を、E14(P+V7)の page で ci1 が測る**(+ 比較に P なしの page)。『B1 + P + V7』の Esc の T と late を得る。これが関門(+10 ms)を通れば S2 は不要。
2. 通らず、分解して **C(engine の処理、実スレッドで +5.5 ms)**が主なら: 0312c の C1(UI スレッドの GIL の取り直し)を `patch -R` の D3a の計器の木(§3 の診断の木、約 30 分)で File クリック区間の slow acquire 数 × 待ちを取り、L1(lock-free の import を増やす)→ L2 の順。区分は (c) 通信/待ちの削減。
3. **B2(問い合わせの入口で ring を読む、0312c §1)は、P の後に B の中央値が 1.5 ms を超えるときだけ**。B1+P では B は 0.4 ms なので不要の見込み。
4. 私が先に立てた『P の WANT/DONE のとき engine の idle の wait を `Atomics.notify` で起こす』案は、上の V5 の事実で**取り下げ**。

**E14 の P+V7 と組み合わせた T の見込み(読み)**: B1 の Esc 13.5(+8.7)に P の追加(協調で +1.0〜1.8)を足して **約 +10〜11 ms = 境界**。File は P で下がる(10.0 ms)。= 通る保証は無く、測定が要る。

## 5. 依頼者の条件の再現(PM の (4))

- **16 コア**: この機械は `nproc` = 16(実測)。ただし headless Chrome の GL は SwiftShader(CPU)で、依頼者の GPU 付きの機械より CPU を食う(present のコストの上限側に出る)。比較は同じ機械の腕どうしで行い、依頼者の絶対値とは比べない。
- **参照プロジェクト**: `~/ダウンロード/serina1-serum2.zip`(P2 の指定)= 既存の測定と同じ。本機に VST(Serum 2 など)は無く、FL 内蔵エフェクトの負荷(O17 の『サンプルだけ』に近い)。
- **2 条件**: (A) 全トラック。(B) **Solo = Track 1-5, 7, 13, 15-17, 19, 20**(O17 の記述。依頼者の機械で『ギリギリ成立』)。Solo は FLP のイベントに保存される(playlist track の solo)ので、**変種 FLP `solo12.flp` を作るのが再現性の高い道**(0308 の変種 FLP の道具〔alpha1/opt1〕に依頼。私は読みだけ)。UI で Solo を押す操作は sdriveg2 の座標の仕事で不安定になりやすい。
- 測定する量(各条件 × {協調 E13、RT-A page 無し、RT-A + E14 page}): 5 分再生の PCM の零の連続・distinct 書き込み・underrun・Unlock 間隔(数の基準)、仮想 s/実 s、present の rate、UI Worker・2〜n Worker の CPU(コア数)、T(File クリック・Esc の ABBA)。n の掃引は RT-A の manifest env(`FLE_WORKERS` 2/4/8/既定、`FLE_PROCESSORS` 1 と既定)。

## 6. 実スレッド版が O17 に効く仕組みと、まだ分かっていない点(読みと引用)

- **B1 が効く仕組み(引用)**: 協調では 1 本の Worker が UI + present の JS(12〜14 ms ×35/s ≒ 0.4〜0.5 コア)+ ミキサー(音声 1 秒あたり 2.17〜2.64 s という Worker CPU は、0274 の native で C/ブロック 1.8〜2.9 と同じ量)を全部持つので飽和する。B1 はミキサーを**別 Worker に出す**ので UI + present と取り合わない(B1 の 2 Worker 最大 1.34 コア、UI Worker 平均 0.583)。B1 の数の基準(PCM の零 0.036%)は『FL 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が無視された負荷』の値。
- **ただし B1(n = 1)ではミキサー自体は 1 スレッド**(FL は processors = 1 と見るので 1 本のミキサー)。0274(alpha1、native、仮想 CPU)の結論: 重い区間でミキサー 1 本が仮想 CPU の 83〜97% を使い、**必要な加速は最悪区間で約 2.9 倍**、他のスレッドは 0 ms。= **1 コアのミキサーでは重い区間に届かない**可能性があり、依頼者の『Solo 12 本でギリギリ』はこれと整合する(読み)。そこで **S3a(n > 1、FL がミキサーを複数スレッドに割る)が本命の効き**で、**FL が GetSystemInfo の n でミキサースレッドの数を決めるか**は私が確認していない(0312e §3-1 も『変わりうる』と書いただけ)。確かめ方: RT-A + n = 16 の run で、`fleThreadReport`/`FLE_DBG_REPORT_S` の thread table の `CreateThread` の数が増え、複数 Worker に割れること。
- **n > 1 が FL のミキサーを割るかの手がかり(引用 + 読み)**: alpha1 の表(`docs/investigations/alpha12/tables/thread-priorities-and-creation.txt`、native N1、processors = 1)では FL は **18 本のスレッド**を作る(開始アドレス 0x412ae0 の TThread 系 15 本〔うち 7 と 18 が TMixerThread_DS、TIME_CRITICAL〕と、QFC のタイマー 3 本)。performance-targets §4e の Wine の FL は **19 本**(16 コア級の機械)。= **作るスレッド数は CPU 数の報告にほとんど比例しない**(1 でも 18、Wine で 19)。つまり『n を増やすと FL がミキサースレッドを増やす』は**支持されない**。可能性は 2 つ: (i) 12 本ほどの TThread のプールは常に作られ、ミキサーが仕事を渡す先は CPU 数に依らず固定(n > 1 の Worker が増えれば実並列になる)、(ii) 渡し方が logical processor 数に依る(n = 1 のとき渡さない)。native N1 の『ミックスの間、他のスレッドのスライスは中央値 0 ms』は processors = 1 のときの事実で、(ii) と整合するが (i) も否定しない。**区別は RT-A + n = 16 の thread table(どのスレッドがミックスの間に走るか)でしか付かない**。**FLEngine の lifted C を読んだ結果(cand1 の work dir の wsrc、読み。実行ではない)**: (1) GetSystemInfo の呼び元は 6 か所。`f_41ca30`(0x41ca30、thunk 0x407230)は `dwNumberOfProcessors`([si+0x20])を返す小関数で、**呼び元は Delphi RTL の System 初期化 `f_41ce10` だけ**(0x41ce7a)。結果は全局 0x15e60b4(= `System.CPUCount`)に入り、**読む所は 3 か所(0x4103ef・0x410669・0x410e27)で、どれも `cmp [0x15e60b4], 1`**(FastMM の『CPU が 1 より多ければ spin、そうでなければ SwitchToThread』の分岐と読む)。= **FL 本体はプロセッサ数を『スレッド数の決定』には使っていない**。(2) 別の GetSystemInfo の呼び元(thunk 0x423700 を 0x494dce・0x666c39・0x69c369・0x8b4379・0x11754b2 から)は `[si+4]`(dwPageSize)を読んでいる例を確認(0x494dce)。(3) GetLogicalProcessorInformation・GetNativeSystemInfo は **FLEngine の IAT に thunk が無い**(宣言が各 fle-NN.c の先頭にあるだけ)。(4) GetProcessAffinityMask(thunk 0x423690)は 0xca995d・0xcae267・0xcb1c42 の 3 か所、フラグ `[obj+0x868]+0x49e` が真のときだけ、直後に GetCurrentProcess + 別の API(第 2 引数 0xf)を呼ぶ形。= 『n を増やせば FL がミキサーを割る』を **S3a が成り立たせる根拠は FL の lifted C にも無い**(GetSystemInfo の n が効くのは FastMM の spin 判断だけ)。FL のミキサーの並列化は FL 自身の設定(Options の Multithreaded mixer / generator processing)で決まるはずで、**仮想 registry/設定でそれが有効かどうか**(native N1 でミックスの間に他のスレッドが 0 ms だった事実はこれが無効、または CPUID 側の情報を見ている可能性を示す)が、RT-A の前に読むべき次の点。CPUID は leaf 1 の EBX = 0(論理プロセッサ数を主張しない、決定 0009/0047)で、トポロジの leaf は leaf 1 のデータを返す(0312e §1)。
- **決定性**: n が変わると FL のミキサーの足し算の順が変わりうる → **Export の sha(E13: `381cdb5a…f979`)が n に依るか**を最初に見る(0312e §3-3)。依存するなら『製品の既定 n』の選び方に影響する(再生は n 大、Export は n 固定、は旗の追加になるので別に決める)。
- **GIL**: n が増えると guest が runtime を呼ぶたびの GIL の取り合いが増える(0312c §5b-1)。lifted の DSP 本体(Elastique が native のミキサー CPU の 70.3%、alpha1 0274)は runtime を呼ばない区間が長いので並列に走る見込み(読み)だが、GIL の待ちの割合は未測定。

## 7. 新しい関門(PM)と ci1 の測定の手順の案

関門(PM): (G1) 数の基準(5 分再生で PCM の零の連続・distinct 書き込み・underrun・Unlock 間隔)を満たす。(G2) 入力の T の中央値が協調の **+10 ms 以内**(File、Esc)。(G3) present の rate が協調より落ちない。
追加で私が提案する確認(関門でなく採否の前提): (G4) Export の sha が n に依らない(依るなら別判断)。(G5) first idle(P1: 10 s 以内)が n = 既定で伸びない。(G6) Firefox 158 で起動して再生が成立する。

手順(ci1 の counted Chrome、PM の窓の合図の後): ① RT-A の bundle を assets にする(mk-assets)。② 先に **n = 既定で起動するか**(止まる可能性があるので最後に回す、というのが 0312 の PM の順序。今回は既定が本命なので最初に 1 本だけ見て、止まるなら FLE_WORKERS = 2 から)。③ 条件 A/B × {協調, RT-A, RT-A+E14} の 5 分再生 + T の ABBA。④ Export の sha。⑤ 結果は n と N を必ず記録(0312 の PM の指定)。

## 8. 区分と方針への適合

- 実スレッド化は、**FL が本来持つスレッド(ミキサー、UI など)を実スレッドで走らせる = FL の挙動の忠実性**(Windows のスレッドは本物の並列)。協調 scheduler は我々の代用で、除く側(a)。通信・変換の削減ではない。
- **新しい処理・旗・class/アドレスの分岐は、このメモでは足していない**。Worker 数の式 `clamp(n−1, 2, 8)` は 3 区分に当たらない選択(0312e §2 と PM の 22:23 の判断のまま)。
- 進捗の誠実性: 数字は実測と引用を分けた。ブラウザの実行は 0。代役・再生・推測は含まない。

## 9. 自己申告(このメモの作業)

1. 先に立てた仮説『P の要求で engine の idle を起こす』は、V5 の diff(`0311-variant-V5-…srcB1Pm.diff`)を読んで**取り下げ**た(V5 は handler で `servicePresent(false)` を直接呼ぶ)。読む前に提案に書かなかったのは幸いだが、最初の頭の中の仮説は確認前だった。
2. `git pull` を PM の木で 1 回実行(cand1 の info で既に報告)。
3. tool 名 `bash` を 1 回呼んだ(6 回目。cand1 の info に記載)。
