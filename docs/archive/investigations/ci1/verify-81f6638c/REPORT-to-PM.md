【ci1 報告2: 本番に出た main 81f6638c（CI run 41956980092）のバンドルを実ブラウザで確認】結果: 全項目 合格。

実エンジンの事実:
- CI の手順（ci/stage-*.sh, F2xH7 / Oz, git archive）をローカルで全段 再実行し、同じ commit から作ったページと一緒に vite preview で配信。既定の入口 `/`（パラメータ無し）を Chrome 156 と Firefox 158 で開いた。
- lift した FLEngine_x64.dll（wasm）が実際に動き、エンジンの窓（TFruityLoopsMainForm など）と画面の画素を読んだ。代役・画像の再生・推測は含まない。
- 未確認（今回は測っていない）: 音、120 fps、操作遅延。

1. バンドルは CI の出力と一致

| 項目 | CI ログ | ローカル再構築 | 一致 |
|---|---|---|---|
| wasm | 133,934,996 B / sha256 15bedd6e…a2436bb | 同じ | ○ |
| glue / seed / dll（sha256 先頭） | 96b2ad2bf1ea / f7726113 / 50206405 | 同じ | ○ |
| br 圧縮（wasm/glue/seed/dll 先頭） | e21f27fc… / 823d2d39… / 6f8cf851… / ffcbd680… | 同じ | ○ |

2. 既定の入口 `/`

| 項目 | Chrome 156 | Firefox 158 |
|---|---|---|
| 最初の idle まで | 8.84 秒 | 6.08 秒 |
| コンソールエラー | 0 | 0 |
| beacon `POST /__perf` | 204 保存済み、`credentials: same-origin` | 同じ |
| beacon の `desk.mode` | gl（今回の全 46 件） | gl |

3. 持ち主の報告 11 場面と今回の修正

| 項目 | Chrome | Firefox（場面ごとに Firefox を起動） |
|---|---|---|
| d1〜d11（11 場面） | 全部 ok | 全部 ok |
| d8 File メニュー 100 回 | 100/100、popup の surface 100 | 100/100、100 |
| `+` > More plugins…: 一覧窓が残る | 残る（TPluginListForm 'Select generator plugin'） | 残る |
| Esc 後の主窓と 2 回目の `+` | 一覧窓が消え、`+` で popup が再び開く | 同じ |
| Add メニューの fade（40 枚） | 197,212,221 → 31,40,45、黒 0 枚 | 137,149,157 → 31,40,45、黒 0 枚 |
| F8 picker の fade-in（25 枚×4 点） | 95,101,109 → 31,39,45 単調、半透明。閉じると 94,101,108 に戻る | 86,94,101 → 31,39,45 単調。閉じると 95,102,110 |

4. picker を開いた状態の Worker CPU（ms/秒。マシン負荷 9〜18 の中で測った。ぶれる）

| 場面 | Chrome | Firefox |
|---|---|---|
| picker 前の idle | 150 | 215 |
| fade-in 0.8 秒 | 560 | 870 |
| picker を開いた定常（3 秒） | 533（present 6/秒） | 430（1.3/秒） |
| fade-out 1.2 秒 | 770 | 1240 |
| picker 後の idle | 145 | 200 |

気づいた点（事実のみ）:
- 定常 533 ms/秒は idle 150 の 3.5 倍。present は 6/秒しかなく、ゲスト（エンジン）が全画面を再描画し続けている分と読める（原因の切り分けは未実施）。
- 左の Browser パネルの一覧領域が黒いままなのは、前回の 1a6383d2 と同じ（既知。今回の変更ではない）。
- 1a6383d2 の Firefox で、同じ Firefox のまま何度かページを開き直すと Files 窓（openProject）が出なくなる現象があった（場面ごとに起動し直すと出る）。SIGABRT（exit 134）が d11 の 207 秒かかった実行で 1 回あり、再現せず（以後 d11 は 10 回 ok）。81f6638c では起きていない。

5. 保存先: `lift/out-git/ci1-results/verify/bundle-81f6638c/`（RESULTS.md、report.json、確認ツール tools/、画像 3 枚）。1a6383d2 の結果は報告1で送付済み。

自己申告の逸脱: 存在しない bash ツールを計 4 回呼んだ、拒否された命令で `&` と sleep を使った、長い git コミットメッセージを端末に出した、/tmp に短時間書いた、並列のツール呼び出しで競合を起こした。いずれも気づいた時点で改め、成果物への影響は見つかっていない。

この後: 私の vite preview（4991/4992）と残プロセスを plain kill で止め、lift/out-ci1 の v2/v3/same8 と wt3 worktree を削除する。
