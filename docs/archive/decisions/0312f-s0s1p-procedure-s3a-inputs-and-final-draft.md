# 0312f: (1) S0+S1+P の作り方の手順書、(2) S3a の差分の読み直しと n の入力の整理、(3) 0312 の最終の下書き(mt3。PM 指示 22:38。仮。**build・test・Chrome は 0。実測は patch/git/cmp だけ**)

## 1. S0+S1+P の page の作り方(手順書。`tools/mk-s0s1p-page.sh`)

**要点**: P(0311)は `app/` だけの変更で C は変えない。だから **S0+S1+P の bundle = B1 の bundle(`fl-engine-ci-8301687b`)+ 『積み重ね + P』から build した page**。engine の冷間 build(約 25〜35 分〔読み〕)は要らず、page は `vite build`(数十秒〔読み〕)。gpu1 の `app-dist-b1p` が既にその形のはず(gpu1 の作り方は私は見ていない)。以下は PM が同じ手順で検証するためのもの。

| 段 | コマンド | 内容 | 状態 |
|---|---|---|---|
| 1 | `mk-s0s1p-page.sh prepare REPO TREE` | `git archive HEAD app lift`(19 MB)→ TREE、`apply-stack-0312.sh apply TREE`(足場 → 16 件 → 17 件目 → D3a → S1)、続いて P の `0311-present-worker-{src,test}.diff`(`-p0`)と `0311-tsconfig.diff`(`-p1`)。reject を数える。`app/node_modules` は REPO への symlink | **実測(22:41)**: REPO = main **eb2d0b9f**、`stack` rc=0・reject 0・offset は fe-lift `wall_fr15.rs` の 1 hunk だけ、P の src rc=0・失敗 hunk 0(offset は `main.ts` の 1 hunk で 2 行)、test rc=0、tsconfig rc=0、**reject 0**。ログは `tables/0312f-prepare-stack.log`・`tables/0312f-prepare-P.log`。スタックの 80 ファイルは S0+S1 の木と byte 一致(P が触った `main.ts` の 1 ファイルだけ違う = 期待どおり) |
| 2 | `mk-s0s1p-page.sh check TREE` | `tsc --noEmit` を 4 構成(`tsconfig.json`・`tsconfig.webworker.json`・`tsconfig.test.json`・`worker/tsconfig.json`)、`vitest run --maxWorkers=1`(nice 19) | **窓の後**。前回(21:59、P の複製を S0+S1 に当てた、ただし `worker/tsconfig.json` は未): tsc 3 構成 0 エラー、vitest **71 files・1,323 passed・7 skipped・0 failed** |
| 3 | `mk-s0s1p-page.sh build TREE DIST` | `vite build --outDir DIST`(`app/vite.config.ts`: `/assets/` は `FL_ASSETS_DIR` から配る) | **窓の後** |
| 4 | gpu1 の `tools/preview-p.sh DIST NAME PORT`(`test/e2e/preview-logged.mjs --outDir DIST`、`FL_ASSETS_DIR=out-gpu2/assets-NAME`)、assets は `mk-assets.sh NAME ~/work/mt3w-s1/ci-b1/engine/bundle/fl-engine-ci-8301687b` | B1 の wasm + この page | gpu1 の仕事 |

- 重ね順(守ること): **スタック → P**。P の `worker.ts` が import する `stopAudioReport` は S1 の `worker-hook.ts` が export し続ける(0312b)。逆順(P → スタック)でも当たるが、`main.ts` の offset が 0 になるだけ。
- **競合箇所の一覧**(0312b と同じ。file:line): `app/src/main.ts` の S0 の 2 行(`deskWindowsText`、宣言 `:108`・実装 `:176`)と P の hunk(`@@ -555,6 +555,7 @@`)は 2 行ずれるだけで衝突しない。`client.ts`・`worker.ts`・`protocol.ts` は S0・S1 とも触らない。`fruity.ts` は P が触らない。意味の干渉は無い(0312b §2)。
- **P の最新版が変わったら** `P_DIR` の diff を差し替えて `prepare` から再実行(gpu1 の `lift/out-git/gpu8-results/diffs/`。今回の版は src 21:54・test 20:01・tsconfig 19:50)。計器つきの `0311-measure-only-timing-*.diff` と変種 V1〜V3 は含めない。
- 注意: `check` の vitest は『`lift/` の中の C・Rust のソースを読むテスト』を含むので、TREE に `lift/` が要る(`prepare` は `git archive HEAD app lift` で用意する。前回の複製で `lift` が無く 10 件落ちたのはこのため)。入力 DLL が無いテストは skip になる。

## 2. S3a の差分の読み直しと、n の入力の整理

**読み直しで直したもの(1 件)**: S3a の `fle_cfg_processors()`(`fle_thread.c`)に前方宣言が無かった → `uint32_t fle_cfg_processors(void);` を足した(`-Wmissing-prototypes` を使う build への備え。`-Wall -Wextra` では出ない)。diff を作り直し、base → S3a → S3b の適用と byte 一致を再確認(`impl/0312e-s3a-…`・`s3b-…` は更新済み)。**ほかに見つけた誤りは無い**(読みのみ。コンパイルは未)。確認した点: `fle_sysinfo.h` の include(`fle_thread.c` の `FLE_GIL` の中)、`FLE_SYS_MAX_PROCESSORS` の定義と全ての使用側が同じヘッダ、`nworkers_cfg` の三項演算子の結合(`e ? A : (n<3 ? 2 : (n>9 ? 8 : n−1))`)、`n ≥ 1` なので `n − 1` が負にならない、`EM_JS` の本体に preprocessor の引数を割る `,` が無い、`static` の `n` は共有メモリ上で全 Worker が見る(`thread_local` ではない)。

**n(guest が見るプロセッサ数)と Worker 数の入力**(誰が何を決めるか):

| | B1(`fl-engine-ci-8301687b`) | S3a | S3b |
|---|---|---|---|
| guest が見る n(GetSystemInfo・GLPI・affinity) | **常に 1**(`FLE_PROCESSORS` は無視される: 16 件の修正が `getenv` を除いた) | `FLE_PROCESSORS` があればそれ(1〜32)、無ければ `navigator.hardwareConcurrency`(1〜32 に clamp) | `navigator.hardwareConcurrency` のみ |
| Worker 数 N | `FLE_WORKERS`(無ければ 0 = **協調**) | `FLE_WORKERS`(0・1 = 協調)、無ければ `clamp(n − 1, 2, 8)` | `clamp(n − 1, 2, 8)` のみ |

- **掃引の組み立て(S3a の bundle)**: 既定(env なし) = host の n と `clamp(n−1,2,8)`。この機械は `nproc` = 16 → n = 16 → 既定 N = 8。**n の効果と N の効果を分けるには**、`FLE_PROCESSORS` を固定して `FLE_WORKERS` を 2/4/8、あるいは `FLE_WORKERS` を固定して `FLE_PROCESSORS` を 1/4/16。協調の arm は `FLE_WORKERS=1`(S3a の既定は実スレッドなので、**manifest に `FLE_WORKERS` が無い run は B1 のときと違い実スレッドになる**。B1 の assets〔`FLE_WORKERS=2`〕との比較のときに注意)。
- **E13 本番との差**: E13(協調)は常に n = 1・N = 1。S3a の既定(env なし)の run は『n = 16・N = 8 の実スレッド』で、B1(n = 1・N = 2)とも違う。**測定の記録には n と N を必ず書く**(`fleThreadReport` の thread table、あるいは GetSystemInfo の値を `engine-log` に出す一時の計器で)。
- 協調の test tree(D1(a′)の複製)と wasm の node テスト: `FLE_GIL` の外は n = 1 なので変わらない。**`FLE_GIL` の wasm を node で走らせるテスト**(`lift/harness/mt2/node-bundle-run-rt.mjs` など)は node の `navigator.hardwareConcurrency`(Node 21 以降にある)を読む → 機械ごとに n が変わる。決定的な出力が要るテストは S3a の `FLE_PROCESSORS` で固定する(S3b の後は node 側で `navigator` を差し替える、またはテストだけ n を固定する host の定義が要る = **S3b の前に決める点**)。

## 3. 0312 の最終の下書き(PM の取り込み用。docs/0312 の現状から何が増えたか)

**区分**: 実測 = patch/git/cmp/native テスト/ビルドの出力を見た事実、読み = コードを読んだだけ、未確認 = 実エンジンでの動作。**実エンジンのブラウザでの実行は 0 件(私の手では)。B1 は gpu1 が測った(PCM の零 3 chunk〔0.036%〕、PM が確認)。**

| 項目 | 状態 | 物 |
|---|---|---|
| S0(E13 + 足場 + 16 件 + 17 件目 + D3a) | 実測 | `impl/0312-s0-fix17-fruity-diagnostic.diff`(17 件目)。足場・16 件・D3a は 0301/0305 |
| S1(段階 C) | 実測(unit・wasm テスト・native・a4・B1 の ci build) | `impl/0312-s1-stage-c.diff` |
| 積み重ねの 1 本化 | 実測 | `impl/0312-stack-on-main-e08b4589.diff`(80 ファイル +2,919 −281)、`impl/apply-stack-0312.sh`、`impl/0312-stack-on-main-check.log`・`0312-stack-verify.txt`。main eb2d0b9f でも `prepare` が通る(§1) |
| P(0311)との衝突 | 実測 + 読み | `docs/0312b` |
| S2 の設計・I15 の静的点検 | 読み | `docs/0312c` |
| S3a/S3b の先行実装 | patch の適用のみ実測、コンパイル・テストは未 | `impl/0312e-s3a-…`・`0312e-s3b-…`、`docs/0312e` |
| S0+S1+P の手順 | `prepare` まで実測 | 本書 §1、`tools/mk-s0s1p-page.sh` |

**自己申告(0312 全体)**: ①s0 の基準木で `tar` に `.` と `-T` を併用して repo 全体 86 GB をコピー(すぐ消した)。②`sleep 0` の拒否、`bash` という名前のツールの呼び出しの拒否(いずれも実行されていない)。③20:07〜20:26 に B1 と native test を ci1 の extras と重ねた(『pure end が窓の合図』と誤読)。④B1 の link の `wasm-opt` が 21:04〜21:08 に CPU 379%(『wasm-opt の並列は不可』の期間)。⑤『FLE_PROCESSORS でも n を変えられる』という誤った info(20:08・21:13)→ 22:21 に訂正。⑥0305d の a4 の『汚染された fe-lift』の表現の訂正。⑦P の複製の最初の vitest が 10 件落ちたのは `../lift` の欠如(環境の誤り)。⑧17 件目の修正の原因の推測を事実のように書いた(訂正済み)。⑨積み重ねの 1 本化の最初の `emit` に scratch の `.apply.log` が混入(作り直し)。⑩docs/0312c の行番号の一部が実際とずれていた(直した)。

**次(窓の後、PM の info で)**: ①emcc/native clang の警告、②native `test.sh full`・a4 を S3a 込みで(n = 1 なので変化なしの見込み)、③`fle_sys_processors` の node テストの追加、④S3a の bundle の冷間 build(gpu1 の Chrome の後)→ n の掃引と first idle・Export sha、⑤`mk-s0s1p-page.sh check`/`build`。


## PM の決定と取り込み(2026-11-06 22:43、docs のみ)

取り込み: mt3 の 0312f(S0+S1+P の手順書、S3a の n の入力の整理、0312 の最終の下書き)を置いた。`tools/mk-s0s1p-page.sh` を `docs/investigations/mt3w/0312/tools/`、prepare の記録を `tables/0312f-prepare-*.txt`(元は .log)に置いた。実行は patch・git・cmp だけで、**ビルド・テスト・Chrome・依頼者の機械は 0 件**。

事実として受け取る(mt3 の実測): main eb2d0b9f に対し、積み重ね(`apply-stack-0312.sh`)は rc=0・reject 0、P の src/test/tsconfig も rc=0・reject 0(offset は main.ts の 1 hunk で 2 行)。積み重ねの 80 ファイルは S0+S1 の木と byte 一致。S3a の diff は `fle_cfg_processors()` の前方宣言を足して作り直した(コンパイルは未)。

判断:
1. **S0+S1+P の bundle は B1 の bundle(`fl-engine-ci-8301687b`)に『積み重ね + P』から build した page を組み合わせる**: P は app/ だけの変更で C を変えないので、engine の冷間 build は要らない(読み)。gpu1 の `app-dist-b1p` はこの形のはずなので、gpu1 の ④(B1+P の ABBA)に使う。作り方が mt3 の手順書と同じかは、gpu1 と mt3 が突き合わせて info する。
2. **S3a の測定の記録には n と N を必ず書く**。S3a の既定(env なし)は『実スレッド、n = host(この機械は 16)、N = clamp(n−1,2,8) = 8』で、B1(FLE_WORKERS が無ければ協調)と挙動が違う。n と N の効果を分けるときは FLE_PROCESSORS か FLE_WORKERS の一方を固定し、協調の腕は FLE_WORKERS=1。
3. S3b(env の除去)の前に、node のテストで hardwareConcurrency が機械ごとに変わる問題の扱い(node 側で navigator を差し替える、またはテストの host 定義)を決める。S3a の間は FLE_PROCESSORS で固定できる。
4. 未確認: S3a のコンパイル・テスト・bundle、S0+S1+P の page の build、実エンジンでの動作。
