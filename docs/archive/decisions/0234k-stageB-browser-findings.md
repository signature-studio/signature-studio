# mt3 段階 B(実スレッド)— ブラウザ(Chrome 156)での検証の記録: 見つかった欠陥と直し(進行中。決定番号は PM)

状態: **作業記録(仮)**。段階 B は未完。ここに書いたことはすべて、この機械(16 論理 CPU、他の担当の負荷 11〜33)での実測または読みで、client の機械の値ではない。最終版は測定が済んでから書き直す。

## 0. 先頭: 何が実エンジンで実行され、何が代役か
- **実エンジン**: FLEngine_x64.dll を lift した wasm(F2xH7、非ハイブリッド、NO_FS=1、`GIL=1`、共有メモリ)を、隔離 Chrome 156 の既定の入口(パラメータは `?debug=1` のみ)で実行した。ゲストのスレッドは Worker 1 本につき 1 本(最大 11〜12 本、pthread)。ページは段階 2 のスパイク用(`app5` = t3 の app + 下の 1 の glue 読み込みの 2 行)。
- **代役・再生・推測**: なし(画像の再生やテスト用の期待値は使っていない)。ただし (a) FLE_WORKERS=12 は『Worker の上限』を実験用に環境変数で渡している(成果物の差分では定数にする)、(b) 音声はまだ出ない(下の 4)ので、**P3(音が途切れない)は未達・未測定**、(c) 下の最適化なしの wasm での実行は『動くか』の確認で、速度の値としては使わない。

## 1. 見つかった欠陥と直し(ブラウザの実行で初めて現れたもの。node の実スレッドの 5 分走行では出なかった)
| # | 症状(実測) | 原因 | 直し(状態) |
|---|---|---|---|
| 1 | 最初の idle の 0.1〜0.2 s 後に『エンジンが止まった』(`exit code undefined`、Worker は終了していない)。Chrome で 3 回再現 | JSPI の中断(`fle_idle_wait`、ファイルダイアログ)は emscripten の keep-alive を持たない。pthread の mailbox の処理(`checkMailbox` → `callUserCallback` → `maybeExit`)が、keep-alive 0 のランタイムを **その下で exit する**。単一 Worker では mailbox が無いので出なかった | `fle_idle_lib.js`・`fle_filedlg_lib.js` が中断の間 `runtimeKeepalivePush/Pop`(`EM_ASYNC_JS` が自動でやること)。**Chrome で 20 s 以上止まらないことを確認(直し前 3/3 停止 → 直し後 5/5 無停止)** |
| 2 | (不変条件 I1 の違反。症状は『Worker の起動が遅れる・止まる』の可能性) | emscripten の glue は、pthread からの `fd_write`(= ランタイムの stderr/stdout への出力。ロックを持ったまま、どのスレッドからでも出る)と `pthread_create` を **同期で** 最初の Worker に転送し、結果を待つ。最初の Worker が長いスライスのゲストコードを走らせていれば、ロックを持ったまま待つ | `fd_write__proxy: 'none'`(`fle_thread_lib.js`。出力は Worker 内で `print/printErr` に渡り、emscripten が最初の Worker へ**非同期**に転送する)。`pthread_create` は最初の Worker だけが呼び、他の Worker は `emscripten_proxy_async` で依頼して続行(`fle_thread.c` `wk_start`)。**検査**: `lift/harness/gil/check_i1.py GLUE WASM` が『glue が最初の Worker へ同期転送する import』を列挙し、許可リストに無いものがあれば失敗する(旧 glue は `fd_write` で失敗、新 glue は通る) |
| 3 | pthread Worker は起動が遅れる: 起動時(約 8 s)に作った Worker が実際に動き出すのは、最初の Worker が最初にイベントループへ戻る(最初の idle)とき | Chrome は入れ子の Worker のスクリプト読み込みに親 Worker のイベントループを使う | 設計どおり(CreateThread は待たない)。起動の 10 s 以内(P1)への影響は『最初の idle まで Worker は動かない』= 起動時の CreateThread の先の仕事はそこまで遅れる。測定: 最初の idle 11〜13 s(この機械、最適化 wasm) |
| 4 | **音が出ない**(再生中の AudioWorklet のアンダーラン 38312/38744、非無音 0)。トランスポートの時計が動かない(1:01:00 のまま)。全 Worker の CPU がほぼ 0 | (a) 出力 SharedArrayBuffer は最初の Worker の `globalThis.fleAudioSab` にだけあり、ミキサーが走る pthread Worker には無い(`fle_js_publish` が何もしない)。(b) (仮説、検証中)最初の Worker がページの idle 待ち中に、他の Worker がそのスレッドを ready にしても、待ちを起こす手段が無い(wake word を待つのは Worker の idle wait だけ) | (a) 段階 C の仕事。**暫定(スパイクの glue の前置きだけ。成果物にしない)**: 最初の Worker が pthread Worker の `load` メッセージに SAB を載せる。(b) `fle_idle_wait` が wake word を `Atomics.waitAsync` で待ち、変化で 0 を返す(`fle_idle_lib.js`、`fruity_idle.c`、`fle_gil.h` `fle_idle_wake_*`) |

## 2. Worker の動的割り当て(PM の求め (ii))
`FLE_WORKERS` の固定 11 本ではなく、CreateThread ごとに `pick_worker`(`fle_thread.c`): ① スレッドが終わった起動済みの Worker(pthread は idle。起動費用なし)、② まだ起動していない次の Worker、③ 全部使用中なら生きているスレッドが最少の Worker。上限 `FLE_MAXW` = 32(最初の Worker + 31)。最初の Worker は最初のスレッドだけを走らせる。上限を超えて同じ Worker に 2 本以上のスレッドが載ると、協調スケジューラの切り替え(スライス)に頼るので、FastMM の spin のようなロック待ちが livelock しうる(段階 2 の既知の制限)。**node の 5 分の実走行では 23 本のスレッドが同時に生きていた**ので、31 の上限で足りる見込みだが、超えたときに名前付きの停止にするか共有するかは未決(今は共有)。

## 3. 次
最適化した wasm での Chrome の 34 s 再生(PCM)→ 5 分再生(アンダーラン 0 か)→ Firefox 158。段階 C(音声リングを wasm の共有メモリへ)は 4(a)の暫定を置き換える。
