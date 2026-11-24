# 実機の終了と失敗の記録と、評価用プロジェクトを開いた時の終了(Issue #1377、2026-11-24)

実記録: `node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf --exits` を実行し、終了コード 0 で、終了・失敗の記録 7 件(7 セッション)の表を観測した。同じ道具の既定の出力は、記録 111 件、読めなかった 0 件、ヘッドレス 0 件。

## 0. 範囲

- 記録: `lift/out-app/host-extra/perf/`(R2 の perf/ の直近 3 日の抽出。commit はしない)。111 件、期間(UTC)2026-11-22 18:53Z から 2026-11-24 06:56Z まで。
- 版: b440e13a623a が 104 件(2026-11-22 18:53Z から 2026-11-24 00:40Z まで)、d7a6e6f392f2 が 7 件(2026-11-24 06:53Z から 06:56Z まで)。
- 直近 48 時間は、道具の実行時刻(2026-11-24 07:56Z)から数えた。記録は全て範囲に入る。
- sid・IP・パス・ユーザー名・ユーザーエージェントは書いていない。セッションは、時刻順に A〜F の記号で呼ぶ(乱数ではない。記号と sid の対応は書かない)。

## 1. 終了と失敗の表(道具の出力)

| 最初 (UTC) | 最後 (UTC) | 版 (bundle.version) | DPR の区分 | 記録数 | engine.state | engine.exit.reason | engine.exit.code | engine.exit.message | error.what | error.where | pageMs (秒) | timeline の最後の印 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-22 18:59Z | 2026-11-22 19:13Z | b440e13a623a | 1.7647 (169 相当) | 22 | exited | finished | 0 | (none) | - | - | 812 | worker_done (812 s) |
| 2026-11-22 19:17Z | 2026-11-22 19:17Z | b440e13a623a | - | 1 | (無し) | - | - | - | failure: left-before-first-idle | - | 1.4 | download_start (0.8 s) |
| 2026-11-22 19:18Z | 2026-11-22 19:19Z | b440e13a623a | 1.7647 (169 相当) | 2 | exited | finished | 0 | (none) | - | - | 30 | worker_done (30 s) |
| 2026-11-22 19:19Z | 2026-11-22 19:20Z | b440e13a623a | 1.7647 (169 相当) | 4 | exited | finished | 0 | (none) | - | - | 85 | worker_done (85 s) |
| 2026-11-22 19:23Z | 2026-11-22 19:24Z | b440e13a623a | 1.7647 (169 相当) | 3 | exited | finished | 0 | (none) | - | - | 43 | worker_done (43 s) |
| 2026-11-22 19:58Z | 2026-11-22 20:00Z | b440e13a623a | 1.75 (169 相当) | 5 | exited | finished | 0 | (none) | - | - | 123 | worker_done (123 s) |
| 2026-11-24 06:53Z | 2026-11-24 06:56Z | d7a6e6f392f2 | 1.7647 (169 相当) | 7 | exited | stopped | 134 | first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8; function entries since the program began: 0 [guest return 0x680e3d2] | - | - | 182 | worker_done (182 s) |

## 2. 件数(reason・code ごと)

| 種類 | reason / code | 件数 | 時刻の範囲 (UTC) | 版 | DPR の区分 |
| --- | --- | --- | --- | --- | --- |
| 終了 | finished / 0 | 5 | 2026-11-22 19:13Z から 20:00Z まで | b440e13a623a(5 件すべて) | 169 相当(1.7647 が 4、1.75 が 1) |
| 停止 | stopped / 134 | 1 | 2026-11-24 06:56Z | d7a6e6f392f2 | 169 相当(1.7647) |
| 失敗 | failure: left-before-first-idle | 1 | 2026-11-22 19:17Z | b440e13a623a | (無し。guest の画面の記録が無い) |

- error の記録(error.what)は 0 件。
- 終了 finished / 0 の message が (none) なのは、コードを読むと、harness が entry から戻った時の停止報告(`returned from entry`)で、捕捉された stderr の行が無かったため。正常な終わりと見える(実測ではなくコードからの読み)。
- 停止 stopped / 134 は、abort 系の終わり(harness の `__wrap_abort` が 134 で終える。`lift/harness/wasm/trace_fruity.c`)。

## 3. 評価用プロジェクトを開いた時の終了

### 3.1 開く窓のあったセッション

`files.engine`(開く窓の数・応答・取り消し)と timeline の `filedlg_request` / `filedlg_answer`(ページ起動からのミリ秒を秒にしたもの)の記録。開く窓の要求と応答は、timeline の印では 1 回目の分だけ見える(印は名前ごとに 1 つ)。

| 記号 | 版 | DPR | 開く窓: 要求 → 応答 (秒) | 開く窓の結果(files.engine) | 終わり方 |
| --- | --- | --- | --- | --- | --- |
| A | b440e13a623a | 1.7647 | 110.5 → 113.6 | open 1・応答 1 | 終了の記録なし。最後の記録は 219.3 秒の定期の記録(engine-idle) |
| B | b440e13a623a | 1.7647 | 273.1 → 275.8 | open 1・応答 1 | 812.0 秒に finished / 0(応答の 536 秒後)。それまで engine-idle |
| C | b440e13a623a | 1.75 | 55.3 → 58.3 | open 1・応答 1 | 123.4 秒に finished / 0(応答の 65 秒後) |
| D | b440e13a623a | 1.75 | 185.3 → 228.8(1 回目は取り消し) | open 2・応答 1・取り消し 1 | 2 回目の応答(20:19 頃)の後も engine-idle。定期の記録の上限 20 件(604.2 秒)で記録が止まる。終了の記録なし |
| E | b440e13a623a | 1(2220×1271) | 47.2 → 50.5 | open 1・応答 1 | 終了の記録なし。最後の記録は 96.6 秒の定期の記録(engine-idle) |
| F | d7a6e6f392f2 | 1.7647 | 175.8 → 178.6 | open 1・応答 1・取り消し 0 | 182.4 秒に stopped / 134(応答の 3.8 秒後) |

### 3.2 特定の結果

- 結果: **候補は 1 件(セッション F。2026-11-24 06:56:40Z の停止)。推定であり、確定ではない。**
- 根拠:
  1. 開く窓の応答(178.6 秒)の 3.8 秒後に停止した。開く窓はこの回だけで、応答された(取り消し 0)。
  2. DPR 1.7647(169 相当)は、依頼者の実機の DPR 約 1.76 と一致する。
  3. 6 セッションの開く窓のうち、応答の後に stopped の記録があるのは F だけ。b440e13a623a の 5 セッションの応答では、stopped の記録は無い(B と C は finished / 0 で終わり、A・D・E は記録が途切れた)。
  4. timeline: `lazy_done`(102.8 秒)は `filedlg_request`(175.8 秒)より前。`lazy_wait` の印は無く、開く時に lazy の読み込みを待った記録は無い。
  5. #1364 の窓の表(`uiWindows`)が、この記録(F の 7 件すべて)に在る。先頭は TLightweightLayeredControl(584×829)、残りは 0×0 の 7 件で、件数は上限の 8 件に達している(残りは記録に無い)。どの窓が FL の本体かは、この記録からは断定できない。
- 特定できていない点:
  - 開かれたファイルの名前は記録に無い(設計どおり)。`Data\Demo projects\` の下の曲だったかは、確かめられない。
  - 停止の理由は、message からは読めない。message は「捕捉された stderr の最後の行」で、この記録ではその行は decision 0126 の証拠の行(`first non-background pixel ...`)である。
  - この行は、初めて黒以外の画素が描かれた時に 1 回だけ出る。出た時刻は記録に無い。この行の後に捕捉される書き込みが無かったことだけ分かる。
  - `function entries since the program began: 0` は、呼び出しの数を数えていない(web 版では数えていない。0 は情報ではない)。
  - `[guest return 0x680e3d2]` は停止時の guest の戻り番地。関数名への対応は、この作業の場所に symbol の表が無いので引けなかった。
- 既知の停止文言との関係(#1365・#1372):
  - `fle_stop`(と `stopf`・`fle_fmt_stop` などの同類)は、`fle: <api>: <why>` の行を書いてから abort する(`lift/runtime/fle_env.c`)。書く関数は fprintf で、捕捉の対象。よって、その種類の停止なら、最後の行は `fle:` で始まるはず。
  - 記録の最後の行は `fle:` ではない。LockResource(#1365 の `fle: LockResource: no PE image is mapped ...`)と、ExitProcess(#1372)の `fle:` の行は、この記録には無い。
  - したがって、セッション F の停止は、`fle_stop` 系の停止ではない可能性が高い(捕捉の範囲に限る)。

### 3.3 記録の項目の違い(版)

- d7a6e6f392f2 の 7 件には、`timeline` の `lazy_start` / `lazy_done` の印と、`uiWindows` がある。b440e13a623a の 104 件には、どちらも無い。
- lazy は、起動に要らない plugin のデータ(`app/src/bundle/manifest.ts` の定義)。d7a6 の記録で、ページは起動の後に lazy の取得を行っている。
- 版の差(記録の項目の差。コードの差は見ていない)。

## 4. 原因の推定(実測と分ける)

### 4.1 実測(記録から)

- 停止は abort 系(終了コード 134)。停止の直前に捕捉された最後の stderr の行は、decision 0126 の証拠の行。
- 開く窓の応答の 3.8 秒後に停止した(セッション F)。
- この版(d7a6)の記録は 7 件、停止は 1 件。b440 の 104 件には、stopped の記録は無い。

### 4.2 推定(未確認)

- **H1(最も有力。確度は低い): `fle:` の行を書かずに abort が呼ばれた。** 候補は、libc やアロケータの内部の abort(ヒープの破損の検出など)か、harness の外からの呼び出し。`lift/runtime` の `abort()`(67 箇所)は、grep で見た範囲では、直前の数行に表示が無いものは見つからなかった(`fle_stop` と同類は、書いてから abort する)。libc やアロケータの中の abort は、この作業では見ていない。
- **H2(メモリ。支持は弱い)**: 最後の記録の wasm の大きさは 1.17 GB(page が数える上限 2 GiB の 55%)。モジュール自身の上限は、この作業では確かめていない。
- **H3(lazy の plugin データと版の差。未確認)**: d7a6 で初めて lazy の plugin のデータが入った。開く時には lazy の読み込みは終わっていた(`lazy_wait` の印が無い)ので、「読み込みの待ち」ではない。ただし、#121・#128 と同じ系(plugin の像の読み込み)が、開く時に起きる可能性は残る。
- **H4(#1357 の `HostFilesUnsupported`)**: 記録に `fle:` の行が無いので、`fle_stop` の系の停止としては説明できない。silent abort になるかは未確認。

### 4.3 読み取り上の注意(#1372 の見直しに使う場合)

- コードを読むと、`ExitProcess` は `fle_stop` を通り、abort で終わる(`lift/runtime/fle_proc.c`)。そのため、記録上は `stopped` と code 134 になるはず(`finished` と 0 ではない)。
- #128 の「exit 0」は、ExitProcess の引数(guest の終了コード)で、プロセスの終了コードではない。

## 5. 次の Issue の候補(1〜3)

1. **計測: 停止の理由を、message とは別の項目に入れる(ビーコンの任意の項目を足す)。** 候補の項目: 最後の `fle:` の行、`== stop:` の行、`abort kind` の行。証拠の行(decision 0126)は別にする。再現: Actions の web 版で、File > Open で評価用プロジェクト(`Data\Demo projects\` の DEDZII と Exoplanet。#121・#128 の手順)を開き、停止の全文を取る。受け入れ: 同じ停止で、理由の項目に `fle:` の行か `== stop:` の行が入る。
2. **再現の切り分け(Actions): 版 b440 と d7a6 で、同じ評価用プロジェクトを、lazy の読み込みが終わった後に開く。** 停止の全文(`== stop:`・`abort kind`・`functions executed`・`entry log`)を取り、silent abort の呼び出し元を出す。戻り番地 0x680e3d2 の関数名は、symbol の表で引く。受け入れ: 停止の種類が H1〜H4 のどれかを、実測で分けられる。
3. **記録の項目: 開く窓の答えの種類を、名前を入れずに真偽で記録する。** 候補: 答えが `Data\Demo projects\` の下の物かどうか、拡張子が .flp かどうか。開く窓の答えから停止までの秒数は、timeline の印で既に取れる。

## 6. 報告の注意(この作業の外)

- d7a6e6f392f2 は、`lift/harness/web/verified.txt`(HEAD)に無い(この 1 行は b440e13a623a)。本番の manifest が d7a6 に替わった経路は、この作業では確かめられない。`pm-work/findings/real-user-beacons-2026-11-24.md` の節 4 の gate の前提と合わない。監査で確認が要る。
- `tools/perf-beacons/summary.mjs` の「FL の窓の大きさ」の行は、`uiWindows`(#1364)を数えていなかった(0 件と出ていた。実際は 7 件)。この行だけ直した(既定の出力の他の行は、直す前と同じ。diff で確かめた)。

## 7. 記録に無いもの

- 開いたファイルの名前と場所(設計で記録しない)。
- engine のログ全文(記録は message の 1 行だけ)。
- 証拠の行の時刻、停止の直前の関数名、module の memory の上限、版 d7a6e6f392f2 の commit。
- 終了の記録が無いセッション(A・D・E)で、ページが消えた理由(タブを閉じた、タブが落ちた、Worker が落ちた)。記録からは区別できない。
