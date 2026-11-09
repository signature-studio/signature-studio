【ci1 報告3: 本番の bundle fl-engine-ci-623aba04(8 モジュール、main 26a7cf4e)を、手元の c5ed023b 木の再構築と実 Chrome 156 / Firefox 158 で確認】全項目 合格。

実エンジンの事実: CI の段スクリプト(ci/stage-*.sh、引数なし、CI_JOBS=3、`git archive` の c5ed023b)で手元に作った bundle を、同じ commit のページと vite preview で配信し、既定の入口 `/` を Chrome 156 と Firefox 158 で開いて実際に動かした(Firefox のプロファイルと TMPDIR はディスク上)。lift した FLEngine_x64.dll(wasm)が動き、窓表と画素を読んだ。代役・画像の再生・推測は無い。未測定: 音、120 fps、操作の遅延、依頼者の機械、実 GPU、Worker/Access/R2 を通した配信(本番の URL は Access の内側)。

1. 本番の bundle と手元の再構築はバイト単位で一致(wasm・glue・seed・dll・br)

| 項目 | CI(build run 41987198683、deploy run 41991899343) | 手元 c5ed023b | 一致 |
|---|---|---|---|
| wasm | 141,795,291 B、sha256 3d0279d45493e42eb5d86e189700e5d701cef808010685e63eb3119bb5b5cac3 | 同じ | ○(PM の期待値とも一致) |
| glue | 732,085 B、名前 `engine-glue.6ac38751eb38.js`(sha256 の先頭 12 桁) | 732,085 B、sha256 6ac38751eb383a51…a44a8 | ○ |
| seed | 7,248,243 B、sha256 f7726113ae34d52e…8984 | 同じ | ○ |
| dll(8 ファイル) | 77,444,112 B、sha256 2f494419c1adec95…7d39 | 同じ | ○ |
| br の wasm / glue / seed / dll | 15,533,723 / 65,975 / 3,929,989 / 15,953,166 B、sha256 先頭 2a8ad1ee5c44f3a3 / bc752815cb0901f0 / 6f8cf851197d727b / 3a9645ab01725069 | 同じ | ○ |
| manifest | 3,097 B | 3,086 B | prefix(623aba04 と local)と作成の文言だけ違う |
- 手元の段の秒数(nice 15、load 10〜34): extract 14、felift 40、b2d 89、lift 118、objects 922、link 837、bundle 358(計 40 分)。手元の libblend2d.a は 159 個の wasm オブジェクト全部に +atomics。

2. 前回の失敗(run 41982180403、e840e4d4)の再現。ci/plan.sh と ci/needs.sh を、各 commit の git worktree に、CI のジョブと同じ CI_RUNNER_OS=ubuntu24-x64 で走らせた。出た k_b2d は CI のログの値と同じ

| commit | k_b2d | CI のキャッシュ検索 | needs.sh(H_B2D から) |
|---|---|---|---|
| 5c4b6902(旧い libblend2d.a を保存した版) | fl-b2d-cb54413de1e68dce6457f851637737e5a63edfac | found | - |
| e840e4d4(run 41982180403、link で失敗) | **同じ** cb54413d…(フラグ -matomics -mbulk-memory が鍵に無い) | found | restore_b2d=true、run_b2d=false → 旧い(atomics 無しの)ライブラリが戻り、共有メモリの link が失敗 |
| c5ed023b(run 41986214303) | **違う** fl-b2d-118b3ea31dc4c9910133b6c419398bb1784012cc(鍵に cflags) | not found | restore_b2d=false、run_b2d=true → 作り直し |
- ライブラリの中身(target_features): 旧い物(5c4b6902 の手元ビルド)は +atomics が 0/159、c5ed023b の b2d の段の物は 159/159。ci/test-b2d-key.sh は c5ed023b の木で 6 項目すべて通り、同じ検査を e840e4d4 の木に当てると 3 項目落ちる。表は tables/cache-b2d.md。

3. CI の秒数とメモリ(lift/out-pmprompts/ci-report.sh)

| run(commit) | 内容 | 秒 / 値 |
|---|---|---|
| 41982180403(e840e4d4)、冷間 | felift / lift / wasm objects(8 モジュール、全部) / link(失敗) | 27 / 47 / 1,410 / 296 |
| 41986214303(c5ed023b) | b2d(鍵が変わり作り直し) / objects(キャッシュから、35 s) / link は『lifter のバイナリが戻らない』で失敗 | 86 / 35 / - |
| 41987198683(a6331e92) | link / bundle / build job 全体 | 1,369 / 521 / 33 分 |
| 41987198683 の deploy | bundle のオブジェクトは上がったが、step「R2: the beacons (perf/) expire after 180 days」が AccessDenied で失敗し、Worker も manifest も切り替わらなかった(本番は不変) | 至急報告で直しを渡した |
| 41991899343(26a7cf4e) | build は bundle キャッシュで skipped。deploy 50 秒: lifecycle は警告(『the perf/ expiry is not applied (the deploy goes on)』)、Worker 568b4580-b36b-4778-8a67-568bbe759829 を 100%、既定 manifest を切替、Access で無認証 16 件が 302 | success |
| runner のメモリ(cgroup system.slice memory.peak) | objects の後 4,686,192,640 B(objects は復元) / link の後 7,894,290,432 B(RAM 7,931 MiB の 94.9%)/ swap のピーク 49,152 B / 冷間 objects(run 41982180403)7,764,004,864 B | OOM の行なし |
- 冷間の全体は run をまたいで felift 27+lift 47+objects 1,410+link 1,369+bundle 521 ≒ 57 分(CI)。

4. 既定の入口 `/`(first idle、ms)

| ブラウザ | 単独 | 2 本並列 | コンソールのエラー | beacon | desk.mode |
|---|---|---|---|---|---|
| Chrome 156 | 6,700 | 6,265 | 0 | 204 保存済み、`credentials: same-origin` | gl |
| Firefox 158 | 5,770 | 5,662 | 0 | 204 保存済み、same-origin | gl |
- 5c4b6902 の bundle は単独 5,797 / 4,778(その時 load 約 5、今回は約 10)。P1(10 秒以内)の範囲内。

5. 11 場面と今回の直し

| 項目 | Chrome | Firefox(場面ごとに 1 プロセス、ディスク上のプロファイル) |
|---|---|---|
| d1〜d11 | 全部 ok | 全部 ok |
| d8 File メニュー ×100 | 100/100、popup の surface 100 | 100/100、100 |
| + > More plugins… | 一覧窓が残る→Esc で消える→+ で popup がまた開く | 同じ |
| Add メニューの fade(40 枚) | 134,145,153 → 31,40,45、黒 0 枚 | 168,180,189 → 31,40,45、黒 0 枚 |
| F8 ピッカーの fade-in(25 枚×4 点) | 95,101,109 → 31,39,45、閉じると 95,101,109 | 78,86,93 → 31,39,45、閉じると 95,102,110 |
| Firefox を同じプロセスで 8 回読み込み、Files 窓(d1 の openProject) | - | 8/8 出た(first idle 8.3 / 10.1 / 6.2 / 7.0 / 5.8 / 5.8 / 5.0 / 5.2 s) |

6. ピッカーを開いた定常の Worker CPU(ms/秒、3 回ずつ)

| | Chrome | Firefox |
|---|---|---|
| 定常 | 437 / 473 / 477(present 4.3 / 4.3 / 5.0 回/秒) | 320(1.7 回/秒)/ 660 / 690(6.3 回/秒) |
| 前回(5c4b6902) | 450 / 450 / 493 | 530〜757 |
| 待機(前/後) | 135〜140 / 130〜145 | 165〜225 / 185〜225 |
- Chrome は前回と同じ幅。Firefox は present の回数に比例してばらつく。8 モジュールで定常 CPU は増えていない。

7. 8 モジュール特有(参照プロジェクト serina1-serum2.zip〔53.5 MB〕を Files に入れ、File>Open で zip を二度押し。通常窓の実 OPFS)

| 項目 | Chrome 156 | Firefox 158 |
|---|---|---|
| 「Problems loading the project」窓 | **691x343**(572,422) | **691x343**(572,379) |
| 6 モジュール(5c4b6902)の窓 | - | 691x423(572,339) |
| 中身 | 「8 plugin(s) could not be opened … and the plugin(s) will be ignored.」と 8 行(Fruity Wrapper・FLEX・Frequency Shifter・Fruity Balance・Soundgoodizer・Fruity Parametric EQ 2・Fruity Delay 2・Fruity Filter)。欠けサンプル 2 行(anvil_land.ogg・dltzk glass break 1.wav)とその見出し、「not a serious error」の注意行は無い | 同じ |
| 二度押しから主窓の題 `serina1-serum2.flp - FL Studio 2026` まで | 15.1 s | 20.6 s |
| Space(再生)30 秒、2・5・10・20・30 秒に標本 | 毎回 engine-idle、再生中の画面が 5 種類(時計が進む)、停止なし | 同じ(engine-idle、5 種類、停止なし) |
| Space で停止 | ok | ok |
- 注意(自己申告): 最初の再生検査〔エンジンに画面のハッシュを聞く `deskDumpHash()`〕を使った Firefox の 1 回で、検査が 10 分待った後にページが state error になった。エンジンは再生中に yield するまでその問い合わせに答えないので検査側の問題の公算が大きいが、証明はしていない。検査をページの state と画面だけを読む形に替えた 2 回(Chrome・Firefox)は 30 秒の再生で止まらなかった。

8. 見つけた CI の問題(至急報告済み、PM が 26a7cf4e で取り込み、本番に出た)
- R2 の lifecycle の step がキャッシュ用の認証で AccessDenied(バケットの設定は読めない)。デプロイの前に致命になっていた。AccessDenied は警告にして続行(それ以外のエラーは従来どおり失敗)。`perf/` の 180 日の期限切れは、バケット設定の権限を持つ認証が無い間は掛からないまま(警告が CI に出る)。

保存先: `lift/out-git/ci1-results/c5ed023b/`(tables/RESULTS.md、tables/cache-b2d.md、logs/、tools/、ci1-lifecycle.diff)。手元の木は `lift/out-ci1/c5`(2.2 GB)に残している。

次: alpha3 入りの bundle(CI run 41994297607、約 18:50 に出る見込み)が出たら報告4。

自己申告の逸脱: 依存する 2 つのツール呼び出しを並列にして画像の読み込みが先に走り失敗した(やり直した)、`& wait` を含むスクリプトを書きかけて削除した。
