# 183: DispatchMessage の深度 2 の切り替えの許可と、スレッドの優先度の既定(#1425)の測定

判定: 差し戻し(取り消す)

測定: `node tools/ab/run.mjs p3 <基準> <変更後> 3` と `node tools/ab/run.mjs p2 <基準> <変更後> 3`(1 組 = 基準 1 run と変更後 1 run。3 組。`--dpr 1.75 --play 1 --hz 1000 --mouse 20 --sound`(p3)、`--dpr 1.75 --load a`(p2))。基準は main の版(675a1537)、変更後は次の変更を入れた版。

- `fle_callback.c` / `.h`: 深度 2 の許可の旗 `depth2_allowed`(`fle_callback_depth2_allowed` / `fle_callback_depth2_allow`)。
- `fle_thread.c`: `save_ctx` / `load_ctx` で許可の旗をスレッドごとに保存・復元。`fle_progress_slice` は、深度 2 で許可の旗が立つ時だけ、深度 1 と同じに切り替える。`prio_on` の既定を 1 にする。
- `fle_win.c`: `dispatch_message` が、深度 1 から呼ばれた時だけ、WndProc と TIMERPROC の呼び出しの間に許可を立て、終わったら戻す。
- `fle_seh.c` / `.h`: SEH の飛び先で、許可の旗を登録の時の値に戻す(深度と同じ扱い)。

変更は `lift/runtime/` の上の 4 組。試験(`wall_fr20`・`wall_fr9t`・`callback`)と起動・流れ A〜D は、変更後で通った(受け入れの前半)。判定の前に、`p3` の 3 組を先に測り、判定が「差し戻し」と決まったので、`p2` を続けて測った。

## p3 の 6 回の run(基準 3 run・変更後 3 run。`tools/ab/run.mjs p3`)

| 版 | run | pcmRepeat8192 | pcmEligible | underrunDelta | last60sUnderruns | nonSilentDelta |
|---|---|---|---|---|---|---|
| 基準 | 1 | 0 | 34718 | 22 | 22 | 1146 |
| 基準 | 2 | 0 | 34718 | 8 | 8 | 1146 |
| 基準 | 3 | 0 | 34718 | 8 | 8 | 1146 |
| 変更後 | 1 | 0.2060 | 28223 | 8 | 8 | 1271 |
| 変更後 | 2 | 0.2492 | 25877 | 11 | 11 | 925 |
| 変更後 | 3 | 0.3377 | 30899 | 11 | 11 | 1148 |

error は 6 回とも null、playing・inputRateOk は 6 回とも true。

## p3 の中央値と判定

| 指標 | 基準 | 変更後 | 読み |
|---|---|---|---|
| pcmRepeat8192 | 0 | 0.2492 | 増えた。残す条件(変更後が基準の 0.5 倍以下)を満たさない |
| underrunDelta | 8 | 11 | 増えた(基準以下の条件を満たさない) |
| last60sUnderruns | 8 | 11 | 増えた(同上) |
| nonSilentDelta | 1146 | 1148 | 変わらない |

gates(`tools/ab/gates.mjs`): 変更後の全 run の pcmRepeat8192 が 0.01 未満、の判定が FAIL(3 run とも 0.2 以上)。underrunDelta と last60sUnderruns の変更後の中央値が基準の中央値以下、の判定も FAIL。他の判定は PASS。

## p2 の 6 回の run(`tools/ab/run.mjs p2`)

| 版 | run | minWindowFps | inputDropped | inputToPresentMsP95 (ms) | 結果 |
|---|---|---|---|---|---|
| 基準 | 1 | 6.31 | 106 | 82.335 | error null |
| 基準 | 2 | 4.87 | 94 | 93.125 | error null |
| 基準 | 3 | 6.96 | 86 | 75.945 | error null |
| 変更後 | 1 | (なし) | (なし) | (なし) | error: `evaluate: Error: the picture is being shown: ask again`(計測の道具が画面の取得で落ちた。製品の停止ではない) |
| 変更後 | 2 | (なし) | (なし) | (なし) | error: 同上 |
| 変更後 | 3 | 10 | 0 | 62.17 | error null |

基準の中央値: minWindowFps 6.31、inputDropped 94、inputToPresentMsP95 82.335。変更後は 1 run しか値が無く、中央値を比べられない。`minWindowFps` の回帰が無いことは、この測定では確かめられなかった。

## 判定

差し戻し(取り消す)。残す条件「`pcmRepeat8192` の基準の中央値が数値(0)で、変更後の中央値が基準の 0.5 倍以下」を満たさない。変更後の中央値は 0.2492 で、3 run とも基準(全 run が 0)より大きい。音の繰り返し(古いバッファの循環)は減らず、増えた。underrun の中央値も 8 から 11 に増えた。

`lift/runtime/` の変更(許可・優先度の既定・SEH の復元)は取り消した。変更は入れない。

測定の結果から、原因は絞れていない。深度 2 の許可と優先度の既定を分けて測っていない。次に試すなら、どちらか 1 つだけを入れて、`p3` を測り直す(新しい Issue)。

## 取り消した後の受け入れ(同じ版どうしの測定)

取り消した版(`lift/` と `app/` が基準と同一。engine と bundle の sha256 は基準と同じ `909275082f8b`)で、受け入れのコマンドを、基準と変更後の組み(3 組)で測り直した。試験・起動・流れ A〜D は通った。

| 指標 | run 1 | run 2 | run 3 | 中央値(基準 / 変更後) |
|---|---|---|---|---|
| p3 pcmRepeat8192(基準 / 変更後) | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| p3 pcmEligible(基準 / 変更後) | 34718 / 34718 | 34718 / 34718 | 34718 / 34718 | — |
| p3 underrunDelta(基準 / 変更後) | 7 / 11 | 11 / 11 | 11 / 11 | 11 / 11 |
| p3 nonSilentDelta(基準 / 変更後) | 1146 / 1146 | 1146 / 1146 | 1146 / 1146 | 1146 / 1146 |
| p2 minWindowFps(基準 / 変更後) | 8.16 / 9 | 7.99 / 7.62 | 3.77 / 7.98 | 7.99 / 7.98 |

p3 の判定: pass が true(全ての gate が PASS)。p2 の判定: pass が true(error null・inputRateOk true・minWindowFps 中央値 7.98 ≥ 0.9 × 7.99)。同じ版どうしでも underrunDelta は 7〜11 の幅で動く(基準 run 1 だけが 7)。この幅が、上の「取り消した版の中央値の判定」(underrun の 8 → 11)が、変更の効果ではなく測定のばらつきの範囲に入り得ることを示す。

## 同じ作業で直した試験と道具(残す)

- fe-lift の試験 `wall_fr20`・`wall_fr9t`・`callback` は、変更の前の main でも link が切れていた(`fle_glob.c` が `fle_imports.c` の `fle_glob_fixed_*` を満たさない。`fle_thread.c` の `__wrap_pthread_mutex_unlock` が `__real_pthread_mutex_unlock` を満たさない。`fle_clip_host_probe` が無い)。試験の link だけを直した(`tests/support/mutex_real_stub.c` を足し、`fle_glob.c` を link に加え、`clip_host_stub.c` に `fle_clip_host_probe` を足した)。
- `wall_fr20` の「0147 の行は最後の行」の試験は、後から足された行(#1355・#1391・#1382)の数を数えていて、古い前提で落ちていた。0147 の行を名前で探すように直した。`tests/known-failures.txt` から 1 件を消し、`spec/fe-lift-test.test.mjs` の件数を 60 から 59 にした。
- `tools/p3-audio-probe/measure-live.mjs` は、`inputs/p3-project.tsv` の最初の列を、`.flp` の path のときだけ案件として読む。#1401 の行は手順の名前(`default-channel-rack-steps`)で、案件のファイルではないため、起動時の Default の案件を使う。これが無いと `tools/ab/run.mjs p3` と `p2` が、最初の run で止まった。
