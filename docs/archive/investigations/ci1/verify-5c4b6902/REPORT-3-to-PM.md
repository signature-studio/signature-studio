【ci1 報告3: 本番にある e61f80af（main 5c4b6902）を実ブラウザで確認】(1)〜(4) を実施。バイト一致・既定の入口・11 場面・フェード・+ > More plugins… は合格。Firefox の Files 窓は、プロファイルが /tmp(tmpfs) のとき再び出なくなった（ディスク上のプロファイルなら 18/18 出る）。

実エンジンの事実:
- CI と同じ手順（ci/stage-*.sh、F2xH7 / Oz、git archive）の手元再構築を 5c4b6902 と cbf9d1e5 の 2 本作り、同じ commit のページと vite preview で配信。既定の入口 `/`（パラメータ無し）を Chrome 156 と Firefox 158 で開き、lift した FLEngine_x64.dll（wasm）が動いて、エンジンの窓表と画素を読んだ。代役・画像の再生・推測は無い。
- 未測定: 音、120 fps、操作の遅延。

1. 手元の再構築は CI の出力とバイト単位で一致（2 本とも）

| 項目 | 本番 e61f80af（5c4b6902） | 前の本番 3a58cb8c（cbf9d1e5、13:49〜14:36 に稼働） |
|---|---|---|
| wasm | 133,868,905 B / sha256 1f598b9d3a0fa1466cb06d0ce9464a934b1bf83a5af6fef2f819c9ad0c444532 | 133,868,888 B / sha256 04c21ecfb9101408eab14cab88cdb7ceb6d15820e1d304eac91814052b26461f |
| glue / seed / dll（sha256 先頭） | cbdeb1ff2624 / f7726113ae34 / 50206405ba5f | 同じ |
| br の SHA 先頭 16 桁（wasm/glue/seed/dll） | 3b93bdbd781e4cbf / 1b3a65f078ffbc00 / 6f8cf851197d727b / ffcbd680a95ec4e2 | 217494c4c3a1c03b / 同 / 同 / 同 |
| br のサイズ（wasm） | 12,207,167 B | 12,211,588 B |
| 手元との一致 | 全部一致 | 全部一致 |

2. 全面ビルド job（mt4 で生成 C が変わった）は CI で通った

| 項目 | 5c4b6902（run 41964165870） | cbf9d1e5（run 41961969613） |
|---|---|---|
| build job | 45 分 27 秒 | 1 時間 18 分 |
| lift / objects / link（emcc+wasm-opt）/ bundle | 35 s / 906 s / 1,217 s / 449 s | 37 s / 1,060 s / 1,305 s / 463 s |
| runner の RAM | 7,931 MiB | 7,932 MiB |
| メモリのピーク（cgroup system.slice）: objects の後 / link の後〜終了 | 7,281,135,616 B / 7,870,214,144 B（RAM の 94.6%） | 7,401,549,824 B / 7,877,783,552 B |
| swap のピーク | 81,920 B | 49,152 B |
| OOM kill | ログに行なし | 行なし |
| 本番の Worker / 無認証 16 件 | 3284cea2-fc4b-40be-a370-53197c9c51f2 100% / 16 件とも Access の 302 | 16 件 302 |
- link の後にピークに達するのは 81f6638c（7,876,288,512 B）でも同じ。ピークの値は mt4 の前後で変わらない。残りは約 0.45 GB。

3. 既定の入口 `/`（5c4b6902）

| 項目 | Chrome 156 | Firefox 158 |
|---|---|---|
| first idle | 5.80 s | 4.78 s |
| コンソールのエラー | 0 | 0 |
| beacon | 204 保存済み、`credentials: same-origin` | 同じ |
| desk.mode | gl | gl |
- cbf9d1e5 の再構築（負荷 13 以上のとき）: first idle Chrome 8.10 s / Firefox 5.95 s、エラー 0、beacon 204、gl。

4. 11 場面と今回の直し

| 項目 | Chrome | Firefox（場面ごとに 1 プロセス） |
|---|---|---|
| d1〜d11 | 全部 ok | 全部 ok |
| d8 File メニュー 100 回（5c4b6902） | 7 回の実行 すべて 100/100 | 4 回 すべて 100/100 |
| + > More plugins…: 一覧窓 | 残る | 残る |
| Esc の後の + | 一覧窓は消え、+ で popup がまた開く | 同じ |
| Add メニューの fade（40 枚） | 161,172,182 → 31,40,45、黒 0 枚 | 116,127,134 → 31,40,45、黒 0 枚 |
| F8 ピッカーの fade-in（25 枚×4 点） | 95,101,109 → 31,39,45 単調（半透明）、閉じると 95,101,109 | 86,94,101 → 31,39,45 単調、閉じると 95,102,110 |
- cbf9d1e5 の再構築（Chrome のみ）: 11 場面のうち d8 が 1 回だけ 97/100（13・15・50 回目で popup が 6 秒以内に出なかった。負荷 13）。同じ bundle の d8 を 3 回続けて 300/300。原因は不明。5c4b6902 では 700/700（Chrome）・400/400（Firefox）で再現せず。

5. ピッカーを開いた定常の Worker CPU（ms/秒、alpha2b 入り。負荷 5〜12）

| ビルド | Chrome | Firefox |
|---|---|---|
| 81f6638c（1 回） | 533（present 6/秒） | 430（1.3/秒） |
| 5c4b6902（単独 3 回） | 450 / 450 / 493（present 3.3〜5.0/秒） | 617 / 530 / 757（3.0〜5.7/秒） |
| 5c4b6902（全項目の実行中、負荷 25） | 410 | 740 |
| 待機（ピッカーの前後） | 125〜165 | 195〜245 |
- Chrome は 81f6638c の 533 に対し 410〜493。差は run ごとのばらつき（present 数に比例）と同じ幅で、alpha2b の効果とは言い切れない。1 秒あたりの合成の時間は 3.4〜5 ms で、CPU の大半はエンジン自身。

6. Firefox を同じプロセスのまま開き直して Files 窓が出るか（d1 の openProject）

| Firefox のプロファイル | ビルド | 開き直し | 出た | 出ない |
|---|---|---|---|---|
| /tmp（tmpfs、80% 使用、空き 760〜800 MB） | 515d97dd | 8 | 2（1〜2 回目） | 6（3〜8 回目） |
| /tmp | 515d97dd | 5 | 4 | 1（5 回目） |
| /tmp | 515d97dd | 8 | 2 | 6 |
| /tmp | 5c4b6902 | 8 | 2（1・8 回目） | 6（2〜7 回目） |
| ディスク上（geckodriver --profile-root） | 515d97dd | 10 | 10 | 0 |
| ディスク上 | 5c4b6902 | 8 | 8 | 0 |
- 出ない回はすべて: state が engine-idle、first idle 4.3〜7.2 秒（出る回と同じ）、最後の mark は first_pixel（4.4〜7.3 秒）、Worker の 3 秒間の CPU は 0.45〜0.73 s（150〜240 ms/秒、待機のエンジンと同じ）。エンジンは止まっていない。
- 出ない理由は Firefox 自身のエラー: Files ボタンに `Browser files are not available: Could not create actor`、起動資産 4 本は `stored copy: failed (io)`。`navigator.storage.estimate()` は /tmp のプロファイルで quota 388,981,145 B（tmpfs の容量のちょうど 10%）、使用量 213.5 MB。ディスクでは quota 10 GiB。
- 言えるのは「プロファイルが小さい tmpfs のとき OPFS が失敗し、ディスクでは失敗しない」まで。使用量は quota より小さいのに Firefox が拒む理由は切り分けていない。人が使う Firefox で、プロファイルのディスクが満杯に近い場合の挙動は未確認。

自己申告の逸脱: 存在しない bash ツールを計 5 回呼んだ、拒否された命令で `( … &)` と sleep を使った、並列のツール呼び出しで競合を起こした（preview の起動が node_modules のリンク作成より先に走り exit 127）、「待っています」と書いてターンを終え、通知が届かないまま時間を失った（PM の指摘で前景待ちに切り替え）。再現のために Firefox を /tmp のプロファイルで 4 セッション動かした（終了時に消える。tmpfs を最大で約 0.5 GB 一時的に使った）。

保存先: `lift/out-git/ci1-results/verify/bundle-5c4b6902/`（RESULTS.md、report.json、道具 tools/、ログ）。道具は sdrive-a1.mjs に、失敗した回の診断（state・timeline・Worker 3 秒 CPU・OPFS）と `FF_PROFILE_ROOT`（Firefox のプロファイルの場所）を足した。

この後: 私の preview（4992/4993/4994）を kill、lift/out-ci1 の v3/v4/v5 を削除。
