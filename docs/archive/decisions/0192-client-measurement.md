# 0192: 依頼者のブラウザの「画面側」の実測(rAF の周波数・canvas の実更新・長いタスク・遅い入力・機械の情報)をビーコンに足し、P2 の基準と並べて性能パネルに出し、`tailscale serve` 越しの依頼者を `tailnet` と分類する。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0192、担当 cbeacon1)。実装・テスト・測定・文書は作業者 cbeacon1(2026-11-03 20:44〜、予算 2.5 時間)。作業と測定は HEAD **8c10f48** の `git archive`(`lift/out-cbeacon1/src/app`)から。成果物は `lift/out-git/cbeacon1-results/`。`cbeacon1.diff` は `app/` の 24 ファイル(`README.md` を含む)だけを変え、最新の HEAD f2dded6 の `git archive` に `patch -p1 --dry-run` で当たる(`lift/`・wasm・生成 C・ランタイム・既定のバンドル・`lift/out-websrv/`・5173 のサーバーは 0 行・触っていない)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝`S-cbeacon1-…`(§6)。**実機(依頼者の機械・実機 Windows)との照合は 0 件**。この機械は SwiftShader の 16 コアで、他の担当の仕事で load が 3〜16(各表に併記。壁時計の速度は主張しない)。G 段階は変えない。

## 冒頭の三点(正直さ)

1. **何を実測したか(実行した本物と範囲)**: (a) **実エンジン**を、`lift/out-websrv/assets` の既定のマニフェストを**読み取りだけ**して自分の `vite preview`(別ポート、`FL_PERF_LOG` は自分のファイル)に載せ、隔離した headless Chrome 156 で開いた。作業の始めの既定の入口 **F1xn**(`16bd5d9fe402`)と、作業中に PM が切り替えた今の既定の入口 **SHIPH**(決定 0189、`cc975e05e136`)のそれぞれを、パラメータ無しの既定の入口で 1 回、`?hz=120` を付けて 1 回(計 4 run、すべて 14/14 checks)。ビーコン(first-idle・periodic・exit)がサーバーのログに届き、新しい項目(rAF の間隔と Hz、canvas の実更新、client、hz、sid)が埋まることを見た。ヘッドレスの rAF は 60 Hz の仮想 vsync なので、**「rAF の Hz が 60 と出る = 画面が 60 Hz 相当」が正しく出ること**の確認であって、120 fps を達成した主張ではない。(b) 測定を止めたとき(`?beacon=0`)と既定のときのページの CPU を、`/proc` のスレッド別 CPU 秒で、F1xn と SHIPH のそれぞれ、既定の入口と `?dedupe=1` の静止アイドルを、交互の 5 組ずつ(計 20 組)測った。(c) 主窓の面の SHA-256(仮想時刻 a3000・a4000・入力後 2 点)とゲストの idle の時刻列のハッシュを、測定あり/なしで 2 回ずつ(A1 B1 A2 B2)、F1xn と SHIPH で比べた。(d) 単体・Worker・e2e のテストと、ミューテーション検査。**代役(surface-table)は、既定の入口には置いていない**。e2e の perf-beacon 群・perf-beacon-off 群は代役 `surface-table-v1`(`/assets/animate/…`、明示のパス)で走り、その記録は `standIn: true`、ページには STAND-IN の帯と、パネルの見出し「STAND-IN: …FL Studio の数字ではない」が出る(維持を e2e が検査する)。**依頼者の機械の値は、まだ 1 つも無い**(この文書の数字は全部この機械の headless Chrome のもの)。
2. **結論**: (i) ビーコンの版 2 で、依頼者がページを開くと、依頼者の機械の rAF の間隔(中央値・p95・最大・遅いフレーム・度数分布、直近 10 秒/最初の idle 以降/起動から)と推定した画面の Hz、1 秒あたりの rAF 回数、canvas が実際に更新された画面フレームの数/割合・描画回数・受信→描画の時間、長いタスク、遅い入力、見えていた時間、ブラウザ・OS・表示領域・色深度・コア数(論理)・`?hz=` の値と一致判定が `perf-log.jsonl` に届く仕組みができた。物理コア数は取れないので、項目で「取れない」と理由を明示した。(ii) パネルは同じ値を P2 の基準(120 Hz・p95 ≤ 8.3 ms・遅いフレーム・描画 ≤ 4.2 ms)と並べ、基準を満たさない値を赤にし、rAF が 60 Hz 付近なら「この画面は 60 Hz 相当です: 120 fps は原理的に出せません」と注記する。案内文(送る項目・送らないもの・止め方)が表の上に出る。`?beacon=0` は送信もメーターも止める。(iii) **`tailscale serve` は 127.0.0.1:5173 への逆プロキシなので、依頼者の https の経路では、ソケットの相手が常に loopback になり、これまでは依頼者の記録が「この機械」と区別できなかった**。`X-Forwarded-For`(tailscale が送信元の tailnet アドレスを入れる)で分類する `classifyPeer` を足した。配信中の 5173 の再起動と `dist/` の再ビルドで有効になる。(iv) 測定の負担: 既定の入口(F1xn・SHIPH とも、アイドルでも主窓を毎フレーム描き直す)では、ページ主スレッドの差の中央値は F1xn で −0.006 コア、SHIPH で −0.003 コア(測定ありのほうが小さい。差は雑音の範囲)。**静止したアイドル(`?dedupe=1`)では、rAF のループが Chrome にフレームを作らせるので、ページ主スレッド +0.014 コア(F1xn 0.021 → 0.035、5/5 組。SHIPH 0.022 → 0.040、5/5 組)、Chrome 全体で +0.026〜+0.032 コア**。メーターの JavaScript 自体は 1 フレームあたり 33〜43 ns。(v) 測定ありとなしで、入力前の主窓の面(a3000 `cf92b670c533`・a4000 `9b8592d35b26`)とゲストの idle の時刻列のハッシュ(idle 500〜4500 の 9 点)は、F1xn・SHIPH の 8 回とも同一。入力後は、**Enter が同じ仮想時刻(4613〜4617 ms)に届いた SHIPH の 3 回は、測定あり/なしを問わず面 4 点も idle の時刻列 20 点も同一**で、違った 1 回(測定あり、Enter が 4604 ms)は、対照(同じ構成の別の回)とも同じだけ違う。つまり入力後の違いは、この 4 回では Enter の仮想時刻で分かれ、測定の有無では分かれなかった(4 回なので統計的な主張ではない。F1xn の 4 回は Enter の仮想時刻が全部違い、入力後は比較できない)。
3. **未確認**: **依頼者の機械の値(0 件)**、実際の `tailscale serve` 経由での tailnet の判定(ソースとバイナリの文字列と単体テストまで。PM が 5173 を再起動して確かめる)、120 Hz 以上の実ディスプレイ・可変リフレッシュ・複数ディスプレイ、Firefox・Safari・Edge、隠れた実ブラウザのタブ、GPU のラスタ時間(ページからは見えない)、`?hz=` を読まないバンドルでの `hz.engineHz`、所定のプロジェクトでの値、再生中(窓の更新 120/s)のメーターの費用。

## 0. 先に書くこと(進捗の誠実性、AGENTS.md / `docs/policies/progress-integrity.md`)

**(1) 実エンジンが実行された事実(実測)**: 自分のワークツリーの `vite preview`(別ポート、`dist/` は自分のビルド)で、`FL_ASSETS_DIR=<abs>/lift/out-websrv/assets`(読み取りだけ)、`FL_PERF_LOG=<自分の jsonl>`、**パラメータ無しの既定の入口 `/`** を隔離した headless Chrome 156(TCP の非ループバック 0)で開いた(`app/test/e2e/perf-real-engine.mjs`)。**今の既定の入口 SHIPH**(`logs/ship1-manifest-at-measurement.txt`: bundle `fl-engine-Oz` `cc975e05e136`、`/assets/fl-engine-ship1-SHIPH/engine.cc975e05e136.wasm`、`engine.kind` = `emscripten-fruity-v1`)の結果(`logs/real-ship1-default/`、`tables/real-ship1-default.md`): 最初の idle まで **6.1 s**(load 5 の機械。P1 の主張ではない)、STAND-IN の帯なし、サーバーのログに 3 行(first-idle・periodic・exit)、periodic の beacon は 4,831 B。新しい項目: 最初の idle 以降の rAF 間隔 1,801 個、中央値 16.68 ms・p95 16.68 ms・最大 16.67 ms、**推定 60.0 Hz**、遅いフレーム 0;直近 10 秒では窓の矩形の描画 62.5 /s・**canvas が更新された画面フレーム 57.5 /s(600 フレーム中 575、95.8%)**・1 フレームあたり 1.09 回・受信→描画 p95 0.14 ms;`hz.fit` = `match`。`?hz=120`(`logs/real-ship1-hz120/`、`tables/real-ship1-hz120.md`):直近 10 秒の窓の矩形は **124.9 /s**、しかし画面の rAF は 60 回/s なので canvas が更新された画面フレームは **60 /s(600 フレーム全部、1 フレームに 2.08 回描画)**、`hz.fit` = `display-slower`「エンジンは 120 Hz で描画しますが、この画面の rAF は 60 回/s なので、見えるのは最大 60 fps です」。作業の始めの既定の入口 **F1xn** でも同じ手順で 2 run(`logs/real-default/`・`logs/real-hz120b/`、`tables/real-default.md`・`tables/real-hz120.md`)走らせ、同じ形の結果だった(最初の idle 8.5 s、60.0 Hz、canvas 更新 57.5/60、`?hz=120` で 125.0 /s・60 /s・2.08 回)。この 2 つの対が、「窓の更新/s が 120 を超えても、画面が 60 Hz なら見える fps は 60」を数字で分けられることの例になっている。
**(2) 代役・再生・推測を含む範囲**: e2e の perf-beacon 群と perf-beacon-off 群、パネルの単体テストの一部は代役 `surface-table-v1`(または made-up の数字)を使う(上のとおり、STAND-IN の表示を維持し、代役の数字は緑にも赤にもしない)。単体テストの 60 / 120 / 144 Hz は made-up のタイムスタンプ(偽の時計)で、実機の値ではない。tailscale の動作は**ソースの読みと文字列の確認**(実機の serve 経由では未実測)。**これらの数字を G 段階表の到達や性能の達成として読んではならない。P1〜P3 はどれもこの決定で達成されていない。**

## 1. 何を足したか(`app/` だけ。`lift/`・生成 C・エンジンは 0 行)

### 1.1 ビーコンの版 2(`POST /__perf`、同じオリジンだけ。送る時機は決定 0152 のまま: 最初の idle・30 秒ごとに最大 20 回・終了時 1 回)

版 1 の全項目は不変。足したもの(`src/perf/beacon.ts`・`display.ts`・`browser-info.ts`):

| 項目 | 内容 |
|---|---|
| `v` | 2 |
| `sid` | ページを開くたびの乱数 12 桁(ブラウザに保存しない)。ログの記録を「ページ 1 回」にまとめるため。人や機械の識別子ではない |
| `client.browser` | ブラウザ名、`major.minor`、major、OS 名、headless か(UA と `userAgentData.platform` から) |
| `client.colorDepth`・`client.viewport` | `screen.colorDepth`、ページの表示領域(`innerWidth`×`innerHeight`) |
| `client.page` | `secure`(https か)と `hostKind`(`localhost`/`ts.net`/`ip`/`other`。名前そのものは送らない) |
| `client.physicalCores` | **`{ value: null, reason }`**。`hardwareConcurrency`(版 1 の `browser` にある)は論理プロセッサ数で、物理コア数を返す Web API は無い。取れないことを項目で明示する |
| `display.raf` | rAF の間隔(中央値・p95・p99・最大)、そこから推定した Hz(中央値の逆数)と最速の塊の Hz(`peakHz`)、1 秒あたりの rAF 回数(見えている時間あたり)、遅いフレーム数(中央値の 1.5 倍超)、12.5 ms 超の数、10 区分の度数分布。窓は `recent10s`(厳密)・`sinceIdle`(最初の idle 以降)・`total`(ページを開いてから) |
| `display.canvas` | ページが canvas に描いた窓の矩形の数・1 秒あたり、**rAF フレームのうち新しい絵があったものの数/割合/1 秒あたり**、1 フレームあたりの描画回数、**受信 → 描画完了の時間**(p50/p95/最大)、Worker が post してから受け取るまで(2 つの時計の差、目安)、受信数と描画数、Worker が送った数・省いた数(`?dedupe=1` のとき)。表示方式 A では `null`(0 と書かない) |
| `display.visibility` | 見えていた時間・隠れていた時間・切り替え回数。隠れている間の rAF は fps の統計に入れない(間隔を切る) |
| `display.longTasks` | `longtask`(50 ms 超)の回数・最大・合計、最初の idle 以降の回数、直近 30 s の回数。使えないブラウザでは `supported: false` |
| `display.events` | Event Timing: 16 ms 以上かかった入力イベントの数と、その入力遅延・所要時間の p50/p95/最大、最初の入力(`first-input`)、ページが送った入力の総数。使えなければ `supported: false` |
| `hz` | `?hz=` の値、エンジンへ渡した `FLE_DWM_HZ` 相当、`?hz=auto` の測定値、画面の rAF の Hz・最速の Hz、比、判定(`match`/`engine-slower`/`display-slower`/`page-behind`/`unknown`)と日本語の説明 |

1 件の大きさ: 実エンジンの既定の入口の periodic で 4,848 B(上限 16 KB。`beaconText` は 15.5 KB を超えるときだけ、長い文字列を切る → 画面の測定の「最初の idle 以降」の窓と度数分布を落とす → タイムラインを落とす、の順に削る)。最悪の場合(UA 300 字・GPU 名 200 字・タイムライン 600 件・長いエラー)も上限内に収まるテストがある。

### 1.2 画面のメーター(`src/perf/display.ts`、DOM にも網にも触らない純関数+`DisplayMonitor`)

- 既定でページを開いた時点から動く。`requestAnimationFrame` のループ(1 フレームあたりのコールバックは 33〜43 ns 程度)、`visibilitychange`、`PerformanceObserver`(`longtask`・`event`〔`durationThreshold: 16`〕・`first-input`)。エンジン・Worker・canvas・仮想時計には触らない(読むだけ・数えるだけ)。`EngineClient.onFrame` に `performance.now()` の読みを 2 回足した(`frameReceived`・`frameDrawn` の第 6 引数)。
- **`?beacon=0`**(これまで無かった): 送信も、メーターも止める(rAF のループも作らない)。Performance パネルを開いたときだけメーターが動く(表示のため。何も送らない)。`?beacon=` の他の値は「分からない」とログに書いて送信のまま。
- パネルが閉じていても、タイトル行に「測定値はこのサーバーのログに送られます(止める: ?beacon=0)」と出る。

### 1.3 Performance パネル(日本語の新しい行 12 本と案内文。既存の英語の行は不変)

`display-hz`(画面の更新周波数。**60 Hz 台なら「この画面は 60 Hz 相当です: 120 fps は原理的に出せません」と赤で注記**)・`raf-rate`・`raf-gaps`(p95 を 1 フレーム 8.3 ms〔+10%〕と)・`raf-dropped`・`canvas-fps`(canvas が実際に更新された画面フレーム/s を 120 と)・`canvas-fresh`・`canvas-draw`・`longtasks`・`slow-events`・`hz-fit`・`visibility`・`machine`。基準を満たさない値は赤(既存の `data-status="miss"`)、満たすと緑、測れない/判定しないは灰色、代役の数字は琥珀(既存の規則。**代役の見出し「STAND-IN: …」は維持**)。案内文は表の上に常に出る: この表の値は依頼者の機械の実測としてサーバーのログに送られる / 送る項目の一覧 / 送らないもの(IP アドレス、Cookie、入力した文字、URL、ファイルの中身、外部への通信)/ 物理コア数は取れない / `?beacon=0` で止める。

### 1.4 サーバー側

- `shared/perf.ts`: `classifyPeer(socket, X-Forwarded-For)`、`beaconProblems(beacon)`(版 2 の新しいブロックの型・範囲・整合〔ビンの合計=n、描画数≤受信数 など〕を検査する純関数。**サーバーは拒否に使わない**: テスト・summary・読み手用)。
- `tools/perf-middleware.ts`: `remote` の分類に `classifyPeer` を使う。**依頼者の https の経路(`tailscale serve` → 127.0.0.1:5173)では、ソケットの相手は常に 127.0.0.1(loopback)**になり、これまでは依頼者の記録が `remote: loopback` と出て、この機械の headless Chrome と区別できなかった(決定 0152 の読みどおり)。tailscale のソース(`ipn/ipnlocal/serve.go` の `addProxyForwardedHeaders`: `X-Forwarded-For` = 送信元アドレスだけ、`X-Forwarded-Proto`、`X-Forwarded-Host`、Host はそのまま)と、手元の `/usr/sbin/tailscaled`(1.104.4)のバイナリにこれらのヘッダ名の文字列があることを確認した。**ソケットの相手が loopback のときだけ**、`X-Forwarded-For` の先頭のアドレスの種類で分類する(読めなければ loopback のまま)。アドレスは保存しない。
- `scripts/perf-log-summary.mjs`: 記録を「ページ 1 回」(`sid`)にまとめ、**機械 × ブラウザ × 画面の Hz の種類**ごとに P1(最初の idle の min/中央値/max、10 秒を満たした回数)・P2(窓の更新/s、rAF の Hz、canvas を更新した画面フレーム/s と 120 を満たした回数、遅いフレームの割合、rAF の間隔の p95、描画時間の p95、入力→更新の p95 の最良、長いタスク、遅い入力、`?hz=` と画面の一致)を出す。`tailnet`(依頼者)と `loopback, headless`(この機械の自動測定)と `loopback, not headless`(古いサーバーの `tailscale serve` 越しかもしれない)を別の組にする。`--machine tailnet|loopback|lan|other` で絞る。版 1 の記録も読む(画面の列は `-`)。

## 2. 定義(`src/perf/display.ts` の冒頭にも同じ文)

- **tick**: ページが見えている間の `requestAnimationFrame` の 1 回のコールバック(時刻はそのコールバックの `DOMHighResTimeStamp` + `timeOrigin`)。
- **interval**: 連続する 2 つの tick の間隔。**隠れた区間をまたぐ間隔、見えている区間の最初の tick から始まる間隔は interval ではない**(rAF が止まるのは遅いフレームではない)。`visibilitychange` で連鎖を切る。隠れている間に来た tick は無視する。同じ時刻が 2 回、または時間が戻った tick はフレームではない。
- **rAF Hz** = 1000 / 間隔の中央値(ページが実際に受けた周波数)。**peak Hz** = 1000 / 間隔の 10 パーセンタイル(最も速い塊。ページが重くて rAF が画面の周期の整数倍に落ちているときの画面の周波数)。peak が rAF Hz の 1.3 倍を超えれば「ページが間に合っていない」(`page-behind`)で、画面の種類は言わない。
- **1 秒あたりの rAF 回数** = tick の数 / 見えていた時間(隠れていた時間は 0 fps ではなく、除く)。
- **遅いフレーム** = 間隔が窓の中央値の 1.5 倍を超えたもの(少なくとも 1 vsync を落とした)。別に「12.5 ms 超」(120 fps の 1.5 フレーム)も数える。
- **fresh tick** = 前の tick からこの tick までの間に、ページが canvas に窓の矩形を描いた(`putImageData` かレイヤー描画)ものがある tick。**「canvas が実際に更新された画面フレーム」**。1 フレームに複数回描いても 1 と数え、`drawsPerFreshTick` に平均回数を出す。
- **受信→描画完了** = Worker からの frame メッセージをページが受け取った時刻(`onFrame` の先頭)から、描画の呼び出しが返るまで(JavaScript の呼び出しの時間。GPU のラスタは見えない)。**transit** = Worker が post した時刻(Worker の時計)から受け取るまで(2 つのスレッドの時計の差なので目安。負は 0)。
- 統計の窓: `recent10s`(直近 10 秒、厳密な値の最近傍順位)、`sinceIdle`(エンジンの最初の idle 以降)、`total`(ページを開いてから)。後の 2 つは幅 0.05 ms のヒストグラムから(誤差は最大 0.025 ms)。
- 中央値・p95 は最近傍順位(決定 0152 と同じ。添字 `ceil(p/100·n) − 1`)。

## 3. 実測

### 3.1 実エンジン、今の既定の入口 SHIPH、headless Chrome 156(§0 (1)。`tables/real-ship1-default.md`、`logs/real-ship1-default/`)

`tables/real-ship1-default.md` に、periodic のビーコンの `display`・`hz`・`client` ブロックと、パネルの全行(ページの文面)を載せた。ここに抜く(**SHIPH**、パラメータ無し):

| 窓 | rAF の間隔 n | 中央値 / p95 / 最大 ms | 推定 Hz | 遅いフレーム | canvas 更新の画面フレーム | 描画 p95 |
|---|---|---|---|---|---|---|
| 直近 10 s | 600 | 16.67 / 16.67 / 16.67 | 60 | 0 | 575 / 600(95.8%)、62.5 描画/s | 0.14 ms |
| 最初の idle 以降 | 1,801 | 16.68 / 16.68 / 16.67 | 60 | 0 | 1,530 / 1,801(85.0%) | 0.23 ms |
| 起動から | 2,058 | 16.68 / 16.68 / 366.65 | 60 | 18 | (起動の間は描画が少ない) | |

起動中(展開・コンパイル)は主スレッドが忙しく、rAF の間隔が 366 ms まで伸びる(18 回が遅い)。**定常は 60.0 Hz で、遅いフレームは 0**。これが「最初の idle 以降」の窓を別に持つ理由。**既定の入口はアイドルでも 62.5 回/s 主窓を描き直す(決定 0187)ので、60 Hz の画面では 62.5 描画/s に対して canvas が更新された画面フレームは 57.5/s(95.8%)**: 描き直しの周期(16 ms のタイマー)と画面の周期のずれで、約 4% の画面フレームには新しい絵が無い(パネルを開いた時点の窓では 89%〔532/599〕。窓によって 85〜96%)。

パネルの行(ページが書いた文面、SHIPH の run、抜粋): `display-hz` **赤** 「60.0 Hz(間隔の中央値 17 ms) — この画面は 60 Hz 相当です: 120 fps は原理的に出せません」/ `raf-rate` 赤 60 /s / `canvas-fps` 赤 53 /s / `canvas-fresh` 赤 89% / `longtasks` 緑 0 回(最初の idle 以降)/ `hz-fit` 緑 「エンジン 60 Hz と画面の rAF 60 Hz は合っています」/ `machine` 「Chrome 156.0(Linux、headless) · 論理コア 16(物理コア数は Web API で取れません) · メモリの目安 32 GB · …」。F1xn の run の表とパネルの行は `tables/real-default.md`(最初の idle 8.5 s、最初の idle 以降 1,801 間隔・遅いフレーム 1・最大 33.33 ms、直近 10 s の canvas 更新 575/600、`hz.fit` = match。同じ形)。

### 3.2 `?hz=120` を付けたとき(`tables/real-ship1-hz120.md`、F1xn: `tables/real-hz120.md`)

SHIPH: エンジンは 120 Hz で描画(直近 10 秒の窓の矩形 **124.9 /s**)、headless の rAF は 60.0 Hz のまま。直近 10 秒で canvas が更新された画面フレームは **60 /s(600 フレーム全部)、1 フレームに平均 2.08 回の描画**(= 描いた矩形の半分は、次の rAF までに上書きされて画面に出ない)。`hz-fit` は赤の `display-slower`。**既存の `p2-rate` 行(窓の更新/s)はこの run で「120 /s(1197 updates)」と言う**(開いた時点の窓では 119.7 /s で、表示は 120 だが比較は数値なので赤)——**60 Hz の画面で見える fps は 60 なのに、既存の行だけを見れば 120 にほぼ届いて見える**。新しい `canvas-fps` 行(58 /s、1 フレームに 2.1 回描画)と `hz-fit` 行(「見えるのは最大 60 fps」)がそれを分ける。F1xn の run も同じ形(125.0 /s・60 /s・2.08 回)。**これは 60 Hz の画面の模擬ではなく、headless の 60 Hz の仮想 vsync**。120 Hz の実画面では `display-hz` が緑、`hz-fit` が「合っている」になるはず(読み。made-up の 120 / 144 Hz のタイムスタンプでの単体テストまで)。

### 3.3 測定の負担(CPU)(`tables/cost-default.md`・`cost-dedupe1.md`〔F1xn〕、`tables/cost-ship1-default.md`・`cost-ship1-dedupe.md`〔SHIPH〕)

方法: 隔離した headless Chrome 156 に実エンジンを既定の入口で載せ、最初の idle の後 15 秒待ち、30 秒間の `/proc` のスレッド別 CPU 時間(ページ主スレッド = renderer の tid == pid のスレッド、compositor スレッド、Worker、renderer 全体、GPU プロセス、browser プロセス)の差を測る(`test/e2e/display-cost.mjs`、`tools/run-cost-batch.sh`)。`?beacon=0`(メーターもビーコンも無い = 変更前と同じ動作)と既定を**交互に 5 組**(off→on、on→off、…)。単位は CPU 秒/秒(1.0 = 1 コア)。load は表に併記(3.4〜11.4)。壁時計の速度は主張しない。

| 構成 | ページ主スレッド off → on(中央値) | 差の中央値(正の組) | compositor | GPU | Chrome 全体 | 読み |
|---|---|---|---|---|---|---|
| **SHIPH**(今の既定の入口)既定 | 0.130 → 0.128 | −0.003(1/5) | −0.001 | −0.000 | −0.005 | **差は雑音の範囲**(組ごとの差 −0.011〜+0.029) |
| **SHIPH** `?dedupe=1`(静止アイドル、窓の更新 0.7/s) | 0.022 → 0.040 | **+0.014(5/5)** | +0.009(5/5) | +0.006(5/5) | +0.026(4/5) | rAF のループが Chrome に毎 vsync フレームを作らせる(読み)。load 8〜11 の組あり(Worker の列が揺れる) |
| F1xn(作業の始めの既定の入口)既定 | 0.102 → 0.096 | −0.006(1/5) | −0.003 | −0.002 | −0.018 | 差は雑音の範囲(組ごとの差 −0.062〜+0.002。load 7〜8 の組が大きく負) |
| F1xn `?dedupe=1` | 0.021 → 0.035 | **+0.014(5/5)** | +0.010(5/5) | +0.007(5/5) | **+0.032(5/5)** | 同上 |

- メーター自体の JavaScript(node の微小測定 `tools/bench-display.mjs`、`logs/bench-display.txt`): tick 1 回 33〜43 ns、受信+描画の記録 35〜42 ns、スナップショット(ビーコン 1 件・パネルの 1 回の更新)0.25〜0.26 ms(2 回測定)。60 Hz で 0.000003 コア。**したがって `?dedupe=1` の増分 +0.014 は、コードではなく「rAF のコールバックがあるとブラウザが毎 vsync に BeginMainFrame を作る」費用**(読み。空のコールバックで確かめてはいない)。
- 既定の入口は元々毎フレーム描き直す(窓の更新 62.5/s)ので、フレームの数が変わらず、増分は雑音の下(検出限界はおよそ ±0.01 コア)。静止した画面(`?dedupe=1`)でのみ見える(Chrome 全体で +0.03 コア = 1 コアの 3%)。再生中(窓の更新 120/s)も同じ理由で増えないはず(読み。再生中は未測定)。
- 止めるには `?beacon=0`(メーターが動かない: rAF のループも PerformanceObserver も作らない。e2e が検査する)。
- 代替(実装していない): 静止している間は rAF のループを止める適応型。静止した画面の rAF の Hz は P2 に関係しない。ただし停止の間を fps の統計から除く会計(隠れた区間と同じ)が要る。`S-cbeacon1-27`。
- 最初に取った 1 組(`logs/cost/`、F1xn 既定、4 組)は、自分のミューテーション検査を重ねて load が 7〜10 になった run を含むので、表には使っていない。取り直した結果が上。

### 3.4 測定がエンジンの動作を変えないか(`tables/identity-ship1.md`〔SHIPH〕、`tables/identity.md`〔F1xn〕、`logs/identity*/`)

方法: 既定の入口のマニフェスト(`/assets/manifest.json`、実エンジン)を、`?beacon=0`(A)と既定(B)で 2 回ずつ(A1 B1 A2 B2)走らせ、既存の `sha-at-virtual.mjs`(決定 0170)で、主窓の canvas を読み戻した SHA-256 を、仮想時刻(最初の idle = wiz0 からの ms)の a3000・a4000(入力前)、Enter(仮想 4.6 s 付近)の後の e3300・e5000 で比べ、あわせてゲストの idle の時刻列のハッシュ(`app.idleTrace`: idle の番号・仮想時計・次の期限の FNV-1a の列、500 回ごと、20 点)を比べた。F1xn(8c10f48 の既定)と SHIPH(今の既定)のそれぞれで 4 回。

- **入力前の面は 8 回とも同一**: a3000 `cf92b670c533…`、a4000 `9b8592d35b26…`(F1xn・SHIPH、測定あり・なし)。
- **ゲストの idle の時刻列のハッシュは、入力前の 9 点(idle 500〜4500)が 8 回とも同一**。
- **入力後は、この 4 回では Enter の仮想時刻で分かれ、測定の有無では分かれなかった**(SHIPH。4 回なので統計的な主張ではない): Enter が 4613.1(A1)・4617.1(A2)・4617.1(B2)に届いた 3 回は、測定あり/なしを問わず**面の 4 点も idle の時刻列の 20 点も同一**(A2 vs B2 = 同じ Enter 時刻で測定なし vs あり: 面 4/4・時刻列 20/20 一致)。Enter が 4604.1 に届いた B1 だけが違う(e3300 `99ffc244d19a`、時刻列は 5000 から)。**対照の B1 vs B2(同じ構成)も B1 vs A1・A2 と同じだけ違う**。
- **F1xn の 4 回は Enter の仮想時刻が全部違う**(4636.1・4608.1・4603.1・4630.1)ので、入力後は比較できない(同じ構成の対照も一致しない)。決定 0170 が書いた性質(Enter の仮想時刻に依存する)のとおり。
- 既存の e2e が保証している範囲(この決定の e2e 1008 件は**測定が既定で動いている状態**で走る): 代役(surface-table)の再生の画素が期待値と一致、停止画面の SHA-256 `b12c8ece…`、`stopscreen`/`animate`/`layered` の各表示方式。**実エンジンの 16 スナップショットの比較は e2e に無い**(native のハーネスの基準。ブラウザ側は上の面 4 点と時刻列 20 点まで)。
- 設計上の理由(読み): メーターは `performance.now()` と整数の加算・配列の追加だけで、エンジン・Worker・canvas・仮想時計に触らない。`EngineClient.onFrame` に足したのは時計の読み 2 回と、結果を捨てるイベントの呼び出し 2 回。

### 3.5 テスト

| 層 | 結果(基準 → 本差分) | 備考 |
|---|---|---|
| typecheck | 4/4 緑 | |
| vitest | 1003 passed + 7 skipped → **1140 passed + 7 skipped**(+137) | `perf-display` 新規 43 件(rAF→Hz: 60/75/120/144/240 Hz、ジッタ、1 フレームの長い停止、隠れた穴、隠れている間の tick、窓の切り替え、メモリの上限、canvas の実更新、longtask、Event Timing、`DisplayMonitor` の偽ブラウザ、`hzFit`)、`perf-beacon` +、`perf-panel` +、`perf-server` +、`perf-log-summary` + |
| Worker(`npm run test:worker`) | 43/43 → 43/43 | |
| e2e(隔離 Chrome、dev・preview・Worker の 3 配信先) | 952/952 → **1008/1008**(+56) | perf-beacon 群 77 件(うち新規 25)、perf-beacon-off 群 29 件(新規)、perf-endpoint 群 22 件(`X-Forwarded-For` → tailnet を dev・preview で +2) |
| 実エンジンの perf-real-engine(既定の入口と `?hz=120`) | 既存 8 件 → 14 件(+6) | 新しい項目の埋まり方 6 件を追加(どちらの run も 14/14) |

### 3.6 ミューテーション検査(`tables/mutation.md`:ユニット層 96 行+e2e 層 8 行。コピーの木で 1 つずつ壊す→落ちる→戻す)

コピーの木(`work/mut/app`)で 1 つずつ壊す→テストを走らせる→戻す(`test/mutation/run.mjs`)。

- **ユニット層**: この決定の新規 53 件(Hz の推定を中央値→平均/p95、隠れた区間を数える、隠れている間の tick を数える、見えている時間を経過時間にする、新しい絵の数を描画回数にする、遅いフレームの係数、窓の長さ、ヒストグラムの幅と範囲、リングを捨てない、最初の idle の窓に起動を含める、一致判定の比や許容、周波数の名前の帯、パネルの 60 Hz の注記・赤/緑の閾値・案内文の項目・停止の案内、`?beacon=` の解釈、ビーコンの大きさ上限〔×4〕、版、`X-Forwarded-For` を loopback 以外の相手でも信じる/無視する、スキーマ検査〔物理コア数の数値、ビンの合計〕、ブラウザ名の順序、4 つの数字の版、物理コア数の理由の欠落、ホスト名の漏れ、summary の機械の混同・sid の無視・ソフトウェア GPU・代役の混入)+既存の perf 系 43 件(決定 0152 以降。うち 2 件は `find` を新しい行に合わせた)=**96/96 検出**(`tables/mutation-unit.md`、`logs/mutation-unit-final.log`)。
- 最初の実行(`logs/mutation-unit-run1.log`)では**新規のうち 7 件が検出されなかった**: 2 件は等価な変異だった(`display-gap-tick-counts-fresh`: 隠れた後の最初の tick は間隔を持たないので、描画回数の持ち越しは統計に出ない。`hzclass-throttled-page-named-a-screen`: 周波数の名前の帯は最低でも 56.5 Hz なので 50 Hz の下限は冗長)→ 変異の一覧から外し、ここに記録した。残り 5 件はテストが足りなかった(最速の塊と中央値の取り違え、最初の idle より前に始まった長いタスクが後から届く場合、同じブラウザの 2 つのページの記録が交互に並ぶ場合、UA が普通でも GPU がソフトウェアの場合、代役だけのページ)→ テストを足して検出された(`logs/mutation-unit-run2.log`)。
- **e2e 層**: ページの配線(`src/main.ts`)を壊す 8 件(新規 5〔描画をメーターに渡さない、`?beacon=0` でも送る、`?beacon=0` でメーターを起動する、パネルを開いてもメーターを起動しない、案内文を出さない〕+既存の perf 系 3 件〔1 件は `find` を新しい行に合わせた〕)を、代役の e2e 群(preview、`--only perf-beacon` / `perf-beacon-off` / `default-manifest`)で=**8/8 検出**。検出した検査は意図した物(例: 描画を渡さない → 「received = drawn = ui.updates の合計」の検査が落ちる、`?beacon=0` でも送る → perf-beacon-off の 5 検査が落ちる)(`tables/mutation.md`、`logs/mutation-e2e-final.log`)。
- **等価で外した変異 2 件**と**既存の 3 件**(`find` の文字列が HEAD の時点でソースに無い)は上のとおり。

## 4. PM の未決の点

1. **配信中の 5173 を再起動し、`dist/` を作り直す**(MERGE-NOTES)。再起動までは依頼者の https 経由の記録が `loopback` と出る(`loopback, not headless` の組に入る。この機械の自動測定は `loopback, headless`)。
2. **既定の動作が変わる点**: (a) ページを開くと rAF のメーターが既定で動く(`?beacon=0` で元の動作)。費用は §3.3: 既定の入口では雑音の下、`?dedupe=1` の静止アイドルで Chrome 全体 +0.032 コア。(b) ビーコンが版 2(約 4.8 KB)。(c) パネルが閉じていてもタイトル行に「測定値はこのサーバーのログに送られます」が出る。(d) 新しい行と案内文は日本語(既存の英語の行は不変、`S-cbeacon1-20`)。
3. **`performance-targets.md` §6 に足す文**(私は触っていない): 「決定 0192: ビーコンは版 2。画面側の実測(rAF の周波数・canvas の実更新・長いタスク・遅い入力・ブラウザ/OS)を載せる。`tailscale serve` 越しの依頼者は `X-Forwarded-For` で `tailnet` と分類される。`?beacon=0` で止まる。`node app/scripts/perf-log-summary.mjs <jsonl> --machine tailnet` が機械×ブラウザ×画面の Hz の種類ごとに P1・P2 を出す。」
4. 依頼者へ案内するときは、「ページを開くだけでよい。右側の Performance の行に、何が送られるか・止め方が書いてある。120 Hz の画面なら `?hz=120` を付けたページも開くと、画面の Hz とエンジンの Hz が合うかが分かる」と伝える(§7)。
5. 既存の 3 件のミューテーション(HEAD の時点で変異を作れない)の整理は別の担当。

## 5. 未確認・限界

1. **依頼者の機械の値は 1 つも無い**。この決定が示すのは「依頼者が https の名前でページを開けば、依頼者のブラウザの rAF の周波数・canvas の実更新・長いタスク・遅い入力・コア数などが `lift/out-websrv/perf-log.jsonl` に届く仕組みが動くこと」だけで、P2(120 fps)が達成されたかどうかは何も言っていない。ヘッドレス Chrome の rAF は 60 Hz の仮想 vsync(`?hz=` に依らない。決定 0182)で、「rAF の Hz が 60 と出る = 画面が 60 Hz 相当」が正しく出ることの確認であり、**120 fps を達成した主張ではない**。
2. **`tailscale serve` 越しの `X-Forwarded-For` による tailnet の判定は、実際の serve 経由では未実測**。確認したのは (a) Go のソース(`serve.go`)に `r.Out.Header.Set("X-Forwarded-For", c.SrcAddr.Addr().String())` があること、(b) 手元の `tailscaled` のバイナリに `X-Forwarded-For`・`X-Forwarded-Proto`・`X-Forwarded-Host` の文字列があること、(c) そのヘッダを付けた要求を中間ウェアに通す単体テスト(`perf-server.test.ts`)。serve の設定は触っていない(依頼者の経路を変えうるため)。PM が 5173 を再起動して依頼者が開いた後、`perf-log-summary.mjs --machine tailnet` に行が出れば確認される。出なければ `remote` が `loopback` のままで、`loopback, not headless` の組に入る。
3. **物理コア数は取れない**。論理コア数(`hardwareConcurrency`)だけ。SMT の有無は分からない(参考: 決定 0186 は「この機械は 8 物理コア × SMT」だが、依頼者の機械は不明)。
4. **画面に実際に出たフレームは、ページからは見えない**。`requestAnimationFrame` の回数は「ブラウザがページに描画の機会を与えた回数」で、ディスプレイが実際に表示した回数ではない(合成や GPU の遅れは見えない)。canvas の「実際に更新された」は「そのフレームの間に `putImageData`/レイヤー描画が呼ばれた」で、GPU がラスタした時刻ではない。`display.canvas.drawMs` は JavaScript の呼び出しの時間。
5. **Event Timing は 16 ms 未満のイベントを報告しない**ので、入力遅延の p50/p95 は「遅かった入力の中」の値。全入力の入力→更新の遅延は版 1 の `input` ブロック(決定 0141)。Firefox・Safari には `longtask` が無い(`supported: false`)。
6. **他のブラウザ**(Firefox・Safari・Edge)と、120 Hz 以上の実ディスプレイ、可変リフレッシュ(VRR)、省電力モード、複数ディスプレイ(ウィンドウが別のディスプレイに動いたときの rAF の変化)は未確認。`parseBrowser` は UA 文字列のテストだけ。
7. **`?hz=` を読まないバンドル**(構成 D など)では `hz.engineHz` が実際のエンジンの描画周波数と合わない(`S-cbeacon1-25`)。今の既定の入口(F1xn)は読む。
8. **実エンジンで `display-hz` が緑になる経路はテストしていない**(ヘッドレスは 60 Hz)。120 Hz の rAF での緑/赤は、パネルの単体テスト(rAF 120 / 144 の made-up 値)と、`DisplayMeter` に 120 Hz のタイムスタンプを与えるテストが保証する範囲。
9. 隠れたタブでの挙動は、偽のホストと偽のタイムスタンプでテストした(rAF が止まる・1 秒に 1 回来る)。実ブラウザの隠れたタブ(e2e の `hidden-timers.mjs` の経路)では、この決定では測っていない。
10. ミューテーション検査の既存の 3 件(`expected-surface-sha-mismatch-ignored`・`client-poll-not-sent-to-the-worker`・`destroyed-window-stays-on-the-desktop`)は HEAD の時点で変異を作れない(`find` の文字列がソースに無い)。この差分の前から。
11. **SHIPH(今の既定の入口、決定 0189、f2dded6)**: 作業中に既定の入口が F1xn から SHIPH に切り替わったので、実エンジンの run(既定と `?hz=120`)・CPU の費用(既定と `?dedupe=1`)・同一性を SHIPH でも取り直した(§3)。コードは `client.ts` の 3 つの hunk が行番号のずれで当たるだけ。測り直さなかった物: SHIPH の `?hz=auto`、再生中(窓の更新 120/s)の費用。F1xn の表は残し、どちらの構成の数字かを表に書いた。
12. e2e の stand-in 群は、`surface-table-v1` のテスト用の表示。実エンジンで `FL_ENGINE_BUNDLE` を使う既存の e2e(F1 の停止画面の SHA 比較)は、この決定では走らせていない(bundle を用意していない)。

## 6. 出典のない選択(`S-cbeacon1-N`。理由、代替、撤回の条件)

| 番号 | 選択 | 理由 | 代替・撤回 |
|---|---|---|---|
| S-cbeacon1-1 | 「直近」の窓は 10 秒(`RECENT_MS`)。既存の `RECENT_WINDOW_MS`(決定 0152)と同じ | 既存の更新回数/秒の窓と並べて読める | 5 s / 30 s。窓の長さで結論が変わる値は無い(定常の rAF は窓に依らない)。長いタスクだけは 30 s(Chrome の longtask の頻度が低いため) |
| S-cbeacon1-2 | 「遅いフレーム」= rAF の間隔が窓の中央値の 1.5 倍を超えた回数(少なくとも 1 vsync を落とした)。別に「12.5 ms 超」(120 fps の目標の 1.5 フレーム)も数える | 画面が何 Hz でも使える定義と、120 fps に対する定義の両方 | 2 倍(2 vsync 以上)。60 Hz の画面では 12.5 ms 超は全フレームになる(= 120 fps に届かない、が正しい) |
| S-cbeacon1-3 | 画面の最速の周波数(`peakHz`)= 1000 / 間隔の 10 パーセンタイル | ページが重いと rAF は画面の周期の整数倍に落ちる。中央値だけだと 120 Hz の画面を 60 Hz と誤認する。最速の塊が全体の 1/10 以上あれば拾える | 5 / 20 パーセンタイル。全フレームが落ちているページ(最速の塊が無い)は検出できない(画面の Hz は分からない) |
| S-cbeacon1-4 | 全体・最初の idle 以降の統計は幅 0.05 ms のヒストグラム(rAF は 250 ms まで、描画時間は 20 ms まで、transit は 50 ms まで)。直近 10 s だけ厳密な値 | 無制限に値を持たない(240 Hz で 10 分 = 14 万個)。0.05 ms は 120 Hz の 1 フレームの 0.6% | 厳密な全値(メモリが増える)。ビン内の値は中央(誤差は最大 0.025 ms) |
| S-cbeacon1-5 | 「120 fps の前提を満たす画面の Hz」= 114 Hz 以上(120 の 95%) | 120 Hz の画面の rAF は 119.9〜120.1 と出る(headless の 60 Hz は 60.0)。タイムスタンプの揺れの余裕 | 118 Hz など。60 Hz 台と 120 Hz 台の間に値は無い(0.5 の誤差で判定が変わる値は実測に無い) |
| S-cbeacon1-6 | 間隔の p95 の合格線は 8.33 ms の 1.1 倍(9.17 ms) | 既存の更新間隔の行(`p2-interval`)は 8.33 ms ちょうどだが、rAF のタイムスタンプは 120 Hz で 8.33±0.1 ms と揺れる | 1.0 倍。既存の行は変えていない |
| S-cbeacon1-7 | 「落ちたフレーム」の合格線は 1%(間隔の数に対する割合) | 出典なし。1 分 120 Hz で 72 回まで | 0.1% / 5%。依頼者の言う「遅いフレーム」の許容は未指定 |
| S-cbeacon1-8 | 「canvas に新しい絵があった rAF フレームの割合」は 95% 以上を合格、ただし窓の矩形の描画が 5 /s 以上ある(動いている)ときだけ判定。止まっている画面は判定しない(灰色) | 何も動いていない画面(`?dedupe=1` のアイドル)が赤になるのは誤り | 常に判定(アイドルが赤になる)。5 /s は出典なし |
| S-cbeacon1-9 | 受信 → 描画完了の p95 の合格線は 4.2 ms(120 fps の 1 フレーム 8.33 ms の半分) | 1 フレームの半分を超えると 120 Hz で主スレッドの半分以上を描画に使う | 8.3 ms。GPU のラスタ時間はページから見えない |
| S-cbeacon1-10 | 長いタスクの合格線は「最初の idle 以降 0 回」。Chrome の定義(50 ms 超)をそのまま使う | 起動中(展開・コンパイル)は避けられない。操作中に主スレッドが 50 ms 塞がるのは入力の遅れの直接の原因 | 直近 30 s の回数。longtask API が無いブラウザ(Firefox、Safari)は「測れません」 |
| S-cbeacon1-11 | Event Timing は `durationThreshold: 16`(仕様の最小)で観測。入力遅延の p50/p95 は「16 ms 以上かかったイベントの中」の値で、全入力の分布ではない | API は閾値未満のイベントを報告しない。全入力の数は自前で数えた送信数(`inputsSent`)を並べる | 全入力の遅延は取れない。入力→更新の遅延は既存の `input` ブロック(決定 0141)が全イベントで測る |
| S-cbeacon1-12 | 周波数の名前(60 / 75 / 90 / 100 / 120 / 144 / 165 / 240 / 360 Hz 相当)は、比の対数 0.06(約 6%)以内、50 Hz 未満は名前なし(スロットルされたページであって、画面の種類ではない) | 決定 0182 の `STANDARD_HZ`(60〜240)に 100 と 360 を足した。出典なし | `roundToStandardHz`(決定 0182、50 Hz を 60 に丸める)。丸めると省電力で落ちた rAF が 60 Hz の画面に見える |
| S-cbeacon1-13 | `?hz=` と画面の一致: エンジンの Hz / rAF の Hz が 0.9〜1.1 なら一致。1.3 倍の差で「ページが間に合っていない」 | 出典なし。10% は 60 と 64 は一致、60 と 70 は不一致 | 5% / 20% |
| S-cbeacon1-14 | 一致判定に使う rAF の窓は「最初の idle 以降(間隔 30 個以上)」、無ければ全体。パネルの行は直近 10 s(30 個以上で数値) | 起動中は主スレッドが忙しく rAF が落ちる | 直近 10 s のみ |
| S-cbeacon1-15 | ビーコンの版を 2 にした。`sid`(ページを開くたびの乱数 12 桁。ブラウザに保存しない)を足した | 版 1 の全項目は不変だが新しいブロックがあるので版で区別する。記録を「ページ 1 回」にまとめるのに ID が要る(ログには相手のアドレスも無い) | 版 1 のまま(区別が付かない)。sid 無し(まとめは seq 0 の位置で推定、複数の同時ページで誤る) |
| S-cbeacon1-16 | ブラウザの版は `major.minor` の 2 つだけ(`156.0`)。4 つの数字(`156.0.0.0`)は送らない | 4 つの数字はアドレスに見え(ログの「アドレスが無い」の機械検査に引っ掛かる)、縮約された Chrome の UA では情報が無い | 全部(UA に同じ文字列は既にある) |
| S-cbeacon1-17 | 逆プロキシ(`tailscale serve`)の背後では、ソケットの相手が loopback のときだけ `X-Forwarded-For` の先頭のアドレスの種類で分類 | 依頼者の https の経路は 127.0.0.1:5173 への逆プロキシで、ソケットの相手は常に loopback になり、`tailnet` の行が出ない。tailscale のソース(`ipn/ipnlocal/serve.go addProxyForwardedHeaders`)と、手元の `tailscaled` のバイナリの文字列で `X-Forwarded-For` を確認した(実際の serve 経由では未実測) | ヘッダを見ない(依頼者と手元が区別できない)。loopback 以外の相手のヘッダは見ない(相手が種類を選べない) |
| S-cbeacon1-18 | サーバーは新ブロックの形を検査して**拒否しない**(`beaconProblems` はテスト・summary・読み手が使う) | 拒否するとページは以後送らなくなる(決定 0152)。変な記録でも 1 件あるほうがよい | 400 で拒否 |
| S-cbeacon1-19 | `?beacon=0` は送信と画面メーターの両方を止める。パネルを開いたときだけメーターが動く(表示のため、何も送らない) | 「止める」と書いて測り続けるのは不誠実。メーターの rAF はアイドルでもフレームを作らせる | 送信だけ止める |
| S-cbeacon1-20 | 新しい行と案内文は日本語。既存の行(英語)は変えていない | PM の指示は「日本語で既存に合わせる」だったが、既存のパネルは英語。依頼者は日本語 | 全部英語 / 既存も日本語に(既存のテストが多数落ちる) |
| S-cbeacon1-21 | メーターはページを開いた時点で始める(起動の間も測る) | 起動中の rAF(主スレッドが忙しい)も見える。「最初の idle 以降」の窓で定常を別に読める | エンジン開始 / 最初の idle から |
| S-cbeacon1-22 | summary の「この機械」(loopback・headless)は UA の `HeadlessChrome` か、GPU 名に SwiftShader / llvmpipe / Software を含むもの | 古いサーバーの `tailscale serve` 越しの依頼者の記録が loopback と出るので、それとの区別 | UA だけ |
| S-cbeacon1-23 | summary の「1 ページ」の値は、そのページの**最後の**記録の値。画面の Hz の分類もその値 | 最後が定常に一番近い。最初の idle の記録は窓が空 | 全記録の中央値 / 最大 |
| S-cbeacon1-24 | Worker が post してから受け取るまで(transit)は負を 0 に切る。2 つの時計の差の目安 | Worker とページの `timeOrigin` が違う。数 ms の食い違いで負が出る | 補正しない |
| S-cbeacon1-25 | `hz.engineHz` は環境に入れた値、無ければ 60(エンジンの既定)。バンドルが `FLE_DWM_HZ` を読むかは見ない | 今の既定の入口(F1xn)は読む(決定 0182)。構成 D は読まない(その場合は誤り) | バンドルの宣言を見る(宣言が無い) |
| S-cbeacon1-26 | リングは 20,000 個まで、11 s より古いものは捨てる | メモリを抑える(240 Hz で 10 分でも 11 s 分) | なし |
| S-cbeacon1-27 | rAF のループは、見えている間は常に動かす(静止している間も止めない) | 実装が単純で、会計(隠れた区間の除外)が 1 種類で済む。静止した画面での費用は +0.014 コア(§3.3) | 静止している間(窓の描画が数秒無い間)は止める適応型。停止の間も fps の統計から除く会計が要る。`?beacon=0` で止められる |
| S-cbeacon1-28 | 費用の測定は 15 秒待った後の 30 秒、交互 5 組、`/proc` の 10 ms 刻みの CPU 時間 | 窓が 30 秒なら刻みの誤差は 0.0003 コア。組の順序を交互にして順序の影響を消す。5 組は時間の制約 | 組を増やす。load 7〜8 の組は雑音が大きい(既定の入口の組 4・5) |
| S-cbeacon1-29 | 同一性の比較は、入力前の 2 点の面と idle の時刻列に限る | 入力後は Enter の仮想時刻が回ごとにずれて、同じ構成でも一致しない(決定 0170 の性質)。仮想時刻を固定して入力を入れる手段は、この担当の範囲に無い | 仮想時刻で入力を入れる道具を作る(ページの `sendRaw` を Worker の仮想時計に結ぶ) |

## 7. 使い方(PM・依頼者向け)

- **依頼者**: `https://<TAILNET-HOST>/` を開くだけ(パラメータ無し)。右側の「Performance vs targets P1-P3 …」の行に「測定値はこのサーバーのログに送られます(止める: ?beacon=0)」と出ている。開くと、表の上に送る項目・送らないもの・止め方が日本語で出る。120 Hz の画面で `?hz=120` を付けた場合と付けない場合を比べたいときは、`https://…/?hz=120` も開く(`hz-fit` の行が「合っている / 合っていない」を言う)。
- **PM(依頼者が開いた後)**: `node app/scripts/perf-log-summary.mjs lift/out-websrv/perf-log.jsonl --machine tailnet`。「tailnet(the owner via the tailnet)| ブラウザ | screen: N Hz class」ごとに、最初の idle(P1)、窓の更新/s・rAF の Hz・canvas を更新した画面フレーム/s(P2)、遅いフレームの割合、`?hz=` と画面の一致が出る。`--json` で機械可読。`loopback, headless` はこの機械の自動測定、`loopback, not headless` は(5173 の再起動前の)`tailscale serve` 越しの記録かもしれないもの。
- **この機械の自動測定**: `cd app && FL_ASSETS_DIR=<abs>/lift/out-websrv/assets FL_PERF_LOG=<自分の jsonl> node test/e2e/perf-real-engine.mjs --out <新規dir> [--query "hz=120"]`。`FL_PERF_LOG` を必ず指定する(指定しないと既定の `perf-log.jsonl` は `FL_ASSETS_DIR` の親、つまり配信中の `lift/out-websrv/perf-log.jsonl` になる)。

## 8. 規則違反の自己申告

- 存在しない `bash` ツールを 1 回呼んだ(shell に読み替えられて実行された)。
- 末尾 `&` を 1 回使った(`tail --pid` をバックグラウンドにしようとした。直後に PID を指定して終了させた。他のプロセスに影響なし)。
- `write` ツールの引数名を 1 回間違えて失敗した(ファイルは作られていない)。
- 同時に走らせたプロセスは、費用の測定の最中に単体のミューテーション検査(vitest)を重ねた時間がある(その測定は表から外し、`logs/cost/` に雑音の run として残した。使った表は重ねずに取り直した)。load が 8 を超えた run が既定の入口の組 4・5 にある(表に load を併記)。
- `FL_PERF_LOG` を指定しない実行はしていない(指定しないと、配信中の `lift/out-websrv/perf-log.jsonl` に書く既定になる。`lift/out-websrv/` と 5173 は読み取り以外していない)。
- 最初の report の送信は 25 分(45 分以内)。

## PM の決定と取り込み（2026-11-03）

- **取り込み範囲:** `cbeacon1.diff`（`app/` の 24 ファイル）。`-` 行 56 行を全部読んだ（既存行の書き換えのみ。削除だけの行は無い）。`.orig` は削除済み。
- **PM の実測（HEAD beeeda9 + 本差分）:** typecheck 通過、vitest 1148 通過（7 スキップ）、Worker 統合 43 通過、e2e 1008/1008（取り込み前は 952）。
- **採用:** (1) 画面側の測定（rAF の周波数、canvas の実更新、long task、遅い入力、ブラウザ・OS）は既定で ON。止めるのは `?beacon=0`。(2) ビーコン版 2（約 4.8 KB）。(3) パネルの日本語の行と注意書き。(4) `classifyPeer` は、直接の相手が loopback のときだけ `X-Forwarded-For` を読んで tailnet と分類する。**ローカルのプロセスは XFF を偽造できる**が、私的な配備では許容する（公開しない）。
- **限界（そのまま記録）:** 依頼者の機械の値は 0 件。`?hz=120` での canvas 60/s は、ヘッドレスの 60 Hz 垂直同期によるもので、**120 fps の達成ではない**。画面側の fps は依頼者のブラウザでしか確認できない。
