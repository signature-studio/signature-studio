# 決定 0297(仮): P2『UI 120 fps』の実測 — 本番 E11 の bundle で、ブラウザに実際に出る更新の回数と、120 fps との差の分解

状態: **仮の決定・測定(出荷しない)**。作業者 `gpu1`(成果物名は gpu7)、2026-11-06 JST 15:40〜。依頼: PM 15:41(0297、第 1 報 17:00、最終 17:45)、alpha1 の 0299 を前提。基準: **E11 = main 7373cd55 の app(ページ)+ 本番 E11 の engine wasm**(`engine.49bbf63df894.wasm`、PM の通知〔本番 `fl-engine-ci-dc5fae2b`〕の先頭 49bbf63df894fbe1 と一致する opt1 の冷間ビルド `lift/out-opt2/x17/engine/bundle/fl-engine-ci-23e8f5ba` を**読み取り**で使用。manifest・glue は prefix の hash だけ別)。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=解釈、**未確認**。出典なしの選択=`S-gpu7-N`(§8)。**この機械(16 vCPU の Zen 5 VM、実 GPU なし = SwiftShader、headless Chrome 156)の値で、依頼者の機械(GTX 980)の値は 0 件。**

## 冒頭の三点

1. **何を実行したか**: 実 Chrome 156 headless で**実エンジン**(E11 の wasm、協調 1 Worker)に参照プロジェクト(serina1-serum2)を開き、場面 b で (a) idle(再生なし 15 s)、(b) 純再生(Space の 8 s 後から 15 s)、(c) 再生中に Mixer のフェーダーのドラッグ(5 往復)と Playlist の wheel(30 回)(約 20 s)を測った(`tools/scn-p2.json`)。各フェーズは `driverSnap`(IdleDriver・desk の計数・スレッド別 CPU)と Chrome の trace(viz / cc の frame)で挟んだ。4 本: **既定(page が rAF を測って 60 Hz を渡す)`p2-def`、Chrome のフラグ `--disable-frame-rate-limit --disable-gpu-vsync` で rAF を無制限にして page が測定値 185 Hz → 120 Hz を渡す `p2-unl`(製品の経路〔URL パラメータ・旗なし〕で 120 を渡す方法)、V8 プロファイル付きの純再生 20 s `p2-prof`**。各 run は 1 本(再現は未取得)。代役・再生・モックなし。さらに**測定専用ページ(`DESK_FALLBACK_MS = 0` = エンジンの idle ごとに present を試す)の同じ scenario `p2-m`**(FL が画素を変える頻度の上限を見るため。製品の page には入れない)。**上の `def`・`unl`・`prof` は製品の page そのまま**。
2. **結論**: 下。
3. **未確認**: 依頼者の機械・実 GPU の値(0 件)。再現(各 1 本)。実スレッド版(UI 専用 Worker)での 120 Hz と E11 の上の実スレッド。測定専用ページでの compositor の frame(フェーズ c は trace の buffer が溢れた)。Firefox。

## 結論

1. **120 fps に対して、純再生で画素が変わった frame は 15.6/s(13%)**(既定、60 Hz を渡した場合)。**120 Hz を渡すと 11.7/s(10%)に下がる**(単一 Worker では、FL の刻みが 45.7 → 72.3/s に増えても present は増えず、仮想時計が 0.69 → 0.55 に遅れる)。idle は 0.4〜0.6/s(FL は画素が変わらない刻みを描かない)。再生 + 操作で 16.3〜17.7/s。
2. **差(純再生、既定、120 − 15.6 = 104.4/s)の分解**(§3、測定専用ページ `p2-m` を足した): **(a) FL が描いていない: 74.3/s(71%)** = 渡した Hz が 60 で FL の刻みの上限が 60/仮想 s(60/s 分)+ 単一 Worker が CPU 律速で仮想 s/実 s が 0.69(14.3/s 分)。**(b) FL は刻んだが present されない: 30.1/s(29%)** = 刻み 45.7/s のうち present 15.6/s。これを測定専用ページで分けた: idle ごとに present を試すと(試行 21/s → 315/s)present は 15.6 → **19.4/s(+24%)**に増えるだけ = **(b1) FL が画素を変えない刻みが 約 26/s(25%)、(b2) 変化が present の機会に間に合わなかった分が 約 4/s(4%)**。FL の画素の変化は純再生で **約 19〜20/s(刻みの 40%)**。**ただし再生 + 操作(c)では、既定の present 16.3/s に対し測定専用ページは 35.6/s(刻みの 61%)**で、(b2) が **約 19/s(c の差 103.7 の 19%)**ある = 操作中は既定の page が変化の半分以上を見逃している。**(c) present の実行時間: 1 present あたり JS 2.0 ms(compose 1.43 + put 0.50 + draw 0.07)+ GL 呼び出し後の Worker の idle 約 3.6 ms〔`postMessage` の後の分を含めると 5.8 ms〕(V8 プロファイル)に、**GL 呼び出し直後の native の塊 `(program)` 約 6.0 ms**(§4b)を足して 約 11.6〜13.8 ms = Worker の 18〜21%**(先の 9〜12% は `(program)` の分を数えておらず過小)。SwiftShader の完了待ちが主因で、実 GPU では小さいはず〔読み、未測定〕、ただし 3.6 MB/present のコピーは実 GPU でも残る。
3. **present の回数を決めているのは『単一 Worker が JS に戻る回数』**(§4): rAF は 3.6/s しか Worker に配られず(Worker が飽和)、present の大半(17.7/s)はエンジンの idle からの試行。= **主に飢餓**(page の規則ではない)。純再生では page の規則(rAF の loop、idle の fallback 20 ms = 上限 50/s、sleep)は効いていない(測定専用ページで present +24% だけ)。**操作中(c)は別**: 20 ms のゲートと Worker の飢餓で、FL の変化の半分以上(35.6 → 16.3/s)を見逃す。
4. **120 を渡したときの現れ**(§2): FL の刻みは idle で 64.2 → 127.1/s(**1.98 倍、alpha1 の 0299 の『dmDisplayFrequency に比例』と一致**)、Worker の CPU は idle で 0.23 → 0.48 コア(2.1 倍)。再生では Worker が飽和済み(0.86 → 0.89 コア)なので、刻みの増加は仮想時計の遅れ(0.69 → 0.55)と present の減少(15.6 → 11.7/s)に出る。= **単一 Worker で 120 を渡すのは逆効果**(0268 の『上端 120』は『UI 専用コアがあれば』の前提)。
5. **忠実性(§5)**: (b) の主因は page の present の規則ではなく Worker の飢餓。直す道は **UI を別 Worker に出す(0289 の実スレッド版の前提と同じ)か、present だけを別 Worker に出す**(共有メモリの scene を直接読む = 通信の削減。ただし scene の読みが runtime の表を見るので GIL が要り、実スレッド版の前提と重なる)。FL の挙動を page が代行する分岐は見つからない(§5)。
6. **上限(読み・未測定)**: 実スレッド版(0289、UI Worker が専用コア)では、Hz 60 のまま present 32〜36/s、attempt 49/s(UI の present の機会が増えた)。仮想時計が実時間 = 1.0 なので、120 を渡せば FL の刻みは 120/s、present は『FL が画素を変える割合』(実スレッド E9 の 60 Hz では 刻み 60/s のうち present 34/s ≒ 57%)を掛けて 約 60〜70/s が上限(読み)。**『120 fps で画素が変わる』は、FL 自身の変化の頻度が刻みごとではない(測定専用ページ: 純再生で刻みの 40%、操作中で 61%、約 21〜27 更新/仮想 s、Hz に依らない)ので、理想の環境でも 120 に届かない可能性が高い**(実 Windows の FL が刻みごとに描くかは参照できる値が無い)。

## 1. 方法

- page: main 7373cd55 の `app/` を `vite build`(`srcE11`、無変更)。bundle: 上の opt1 の冷間ビルドの E11(読み取り、`mk-assets.sh` で manifest の圧縮を identity に)。server: `vite preview`(port 4994)。driver: `tools/sdriveg2.mjs`(CDP の模擬入力、`driverSnap` が 10 s ごとでなくフェーズの境目に IdleDriver の統計・desk の計数・スレッド別 CPU を書く)、`tools/scn-p2.json`、`tools/run-p2.sh`、解析 `tools/p2-analyze.py`。
- 測る物と定義: **presents/s** = Worker の `deskPresenter.present()` が画素を変えた回数(= 『画素が変わった frame』。present は scene に更新が無ければ 0 を返して数えない: `desk-gl.ts` の `present()`)。**rAF ticks delivered to the Worker/s** = Worker の rAF の loop の回数(`deskCounters.ticks`)。**presents from the idle/s** = エンジンの idle からの試行(`idleTicks`、`DESK_FALLBACK_MS = 20 ms` で上限 50/s)。**1 present の ms** = compose + put + draw(`composeMs + putMs + drawMs` ÷ presents、JS の壁時計)。**page rAF/s** = page の main thread に注入した rAF の数え(DisplayMonitor は『Worker が描く』方式では canvas を見えない〔`display.ts`: method A〕ので)。**FL の UI の刻み(slice/s)** = 協調エンジンの pulse の thread 0 のスライス数 ÷ 実 s。**仮想 s/実 s** = pulse の `virt` ÷ `real`。**compositor が描いて swap した frame/s** = Chrome の trace の `DirectRenderer::DrawFrame`(software compositing)。CPU = `/proc` の thread。
- 窓: 各フェーズの `driverSnap` の間(idle 15.8 s、純再生 16.1 s、操作つき 27.6 s)。**音楽位置で揃える**(PM の指示、0289 の `matched-music` の方法): 純再生の窓の音楽位置は 既定 6.1〜17.3 s、120 Hz 6.3〜15.1 s で**重なる**(両方 6〜15 s を含む)。ただし 120 Hz の run は仮想時計が遅れて同じ壁時間で進む音楽が短い(8.8 s 対 11.2 s)。

## 2. 結果(E11 協調、1 run ずつ、load 4〜6.6)

(`tables/p2-def.md`、`tables/p2-unl.md`、生ログ `logs/p2-*/`)

| 渡した Hz(page が測った値) | フェーズ | 窓 s / 音楽位置 s | presents/s | rAF → Worker /s | idle からの試行 /s | Mpx/s | 1 present の ms | page rAF/s | **FL の刻み slice/s** | 仮想 s/実 s | Worker CPU(コア) | SwiftShader 等(コア) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **60**(59.997 Hz → 60) | a idle | 15.8 / — | **0.4** | 3.0 | 41.5 | 0.00 | 2.85 | 60.0 | **64.2** | 1.00 | 0.23 | 0.02 |
| 60 | b 純再生 | 16.1 / 6.1〜17.3 | **15.6** | 3.6 | 17.7 | 13.9 | 2.00 | 60.0 | **45.7** | 0.69 | 0.86 | 0.95 |
| 60 | c 再生 + 操作 | 27.6 / 17.3〜38.9 | **16.3** | 0.4 | 14.2 | 16.0 | 2.31 | 60.0 | **53.3** | 0.78 | 0.89 | 1.02 |
| **120**(rAF 無制限 → 測定 185.5 Hz → 120) | a idle | 15.9 / — | **0.6** | 67.1 | 44.1 | 0.00 | 3.13 | 60.0 | **127.1** | 1.00 | 0.48 | 0.03 |
| 120 | b 純再生 | 15.9 / 6.3〜15.1 | **11.7** | 7.6 | 16.1 | 10.3 | 1.85 | 60.5 | **72.3** | 0.55 | 0.89 | 0.68 |
| 120 | c 再生 + 操作 | 27.9 / 15.1〜32.6 | **17.7** | 0.1 | 14.6 | 15.9 | 1.92 | 60.3 | **73.4** | 0.64 | 0.89 | 1.01 |

| **60**(測定専用ページ `DESK_FALLBACK_MS = 0`、`p2-m`) | a idle | 15.8 / — | 0.2 | 1.4 | **988.6** | 0.00 | — | 60.0 | 63.4 | 1.00 | 0.25 | 0.01 |
| 60(`p2-m`) | b 純再生 | 16.1 / 6.4〜18.2 | **19.4** | 3.7 | **315.4** | 15.8 | 1.77 | 60.0 | 48.4 | 0.73 | 0.85 | 1.14 |
| 60(`p2-m`) | c 再生 + 操作 | 27.3 / 18.2〜40.9 | **35.6** | 0.0 | **781.1** | 21.0 | 1.20 | 60.0 | 58.8 | 0.84 | 0.81 | **2.11** |

- **測定専用ページ(`p2-m`)の読み**: 試行を idle ごとに増やす(b: 21 → 319/s、c: 14 → 781/s)と、present は **b で +24%(15.6 → 19.4/s)、c で +118%(16.3 → 35.6/s)**。present/FL の刻み = b 40%(19.4 ÷ 48.4)、c 61%(35.6 ÷ 58.8)。= **FL が画素を変える頻度の上限は 純再生で約 19〜20/s、操作中で約 36/s**(刻みの 40〜61%)。SwiftShader 等の CPU は b 1.14、c 2.11 コアと増える(present が増えたぶん)。compositor が swap した frame: 既定 b 18.9〜20.4、測定専用 b 22.2/s(+3.3/s)。フェーズ c は trace の buffer が溢れて下限(既定 10.6、測定専用 6.6)で比べられない。
- compositor が描いて swap した frame/s(trace): 既定 a 0.4、b 18.9〜20.4、c 10.6(trace の buffer が溢れて c は下限)。120 Hz a 0.6、b 18.9。= **b では Worker の present(15.6/s)より多い 19〜20/s が swap される**(present 以外のダメージ〔カーソル・page の他の要素〕が混じる。画面に出る更新はおよそ present に 3〜4/s 加わった程度)。BeginMainFrame は page で 111〜166/s、そのうち中止(ダメージ無し)が 93%。
- **idle での FL の刻み**: Hz 60 → 120 で 64.2 → 127.1 /s(**1.98 倍**)、Worker の CPU 0.23 → 0.48 コア(**2.1 倍**)。alpha1 の 0299(dmDisplayFrequency に比例、κ = 1.92)と一致。
- **ページが渡した Hz**: 既定 = 60(`app.hz`: 20 intervals、median 16.67 ms)。**この機械の headless Chrome が報告する refresh は 60.0 Hz**。120 を渡す製品の経路は、Chrome を `--disable-frame-rate-limit --disable-gpu-vsync` で起動して rAF を無制限にすること(page が 185.5 Hz を測って 120 に丸める。旗・URL パラメータの追加は無い)。
- 120 を渡した run の page rAF/s は 60.5(注入した rAF の loop が測り終えた後は 60 に張り付いた。測定が終わった後の rAF の数え方〔DisplayMonitor の burst〕の差で、Worker の rAF は 67/s〔idle〕まで配られた)。

## 3. 目標(120 fps)との差の分解

純再生(b、既定、窓 16.1 s)。目標 120/s に対し、画素が変わった frame は 15.6/s。

| 段 | 値(/s) | 失われた分(/s) | 割合(失われた 104.4 に対し) | 原因の分類 | 根拠 |
|---|---|---|---|---|---|
| 目標 | 120 | | | | P2 |
| (b1) 刻みで画素が変わらない | FL の画素の変化 約 19.4(測定専用ページ) | 約 26 | 約 25% | FL が描かない刻み(間引きではない) | 測定専用ページ: idle ごとに試行しても present 19.4/s(刻み 48.4/s の 40%) |
| (b2) 変化が present の機会に間に合わず合体 | present 15.6 | 約 4 | 約 4% | **(b)** 飢餓・page の 20 ms の規則 | 19.4 − 15.6。compositor が実際に swap する frame は 18.9 → 22.2/s(+3.3) |
| (c) present の実行 | — | (Worker の 18〜21%) | — | **(c)** | JS 2.0 ms + GL 直後の `(program)` 約 6.0 ms + GL 後の idle 3.6〜5.8 ms / present(V8 プロファイル、§4・§4b) |

- **(a) が 7 割**: FL に 120 を渡しても、この単一 Worker では刻みが 72.3/s(仮想 0.55)までで、present は逆に減る。**(a) の解消は『Hz を 120 にして、かつ UI の CPU が 1.92 倍を賄えること』(0299 の κ)= UI 専用コア**。
- **(b)**(測定専用ページで分離): 既定の present の機会は Worker が JS に戻る回数(21.3/s、`DESK_FALLBACK_MS` 20 ms のゲートも効く)。**機会を 15 倍(315/s)にしても、純再生では present は +24%(15.6 → 19.4/s)**。= **純再生の (b) の大半は (b1) FL が画素を変えない刻み(約 26/s)で、(b2) 機会不足は約 4/s**。**再生 + 操作(c)では (b2) が大きい**(35.6 − 16.3 = 約 19/s)— 操作で FL の変化が増える(刻みの 61%)のに、既定の page は機会 14/s で見逃す。**ただし機会を増やした present の多くは、同じ数 ms の間に起きた FL の連続する刻みの絵**で、compositor が 1 vsync に 1 frame しか swap しない(b の swap は +3.3/s だけ)ので、**利用者に見える更新の増加は present の増加より小さい**。
- **仮想 1 秒あたりに直すと(音楽位置が重なる純再生、読み)**: present/仮想 s は 既定 15.6 ÷ 0.69 = **22.6**、120 Hz 11.7 ÷ 0.55 = **21.3**(ほぼ同じ)、**測定専用ページ(60 Hz、機会 15 倍)19.4 ÷ 0.73 = 26.6**。present の**機会**/仮想 s は 既定 31 → 120 Hz 43 と 40% 増えたのに present/仮想 s は増えていない。= **この音楽位置(6〜18 s)の FL の『画素が変わる更新』は約 21〜27/仮想 s で、Hz(刻み 2 倍)にほぼ依らない**。刻みは 2 倍になるのに見える更新は同じ = **Hz を上げても FL の見える更新の頻度は増えない可能性が高い**(確認には実スレッド版の同じ音楽位置の測定が要る)。実スレッド E9(Hz 60、音楽 30〜60 s)の present 32〜36/s(仮想 s と同じ)は別の音楽位置で、直接比べられない。
- **(c)**: JS の 2.0 ms は compose(scene の読み)1.43 ms + put(`texSubImage2D` 等)0.50 ms + draw 0.07 ms(`driverSnap` の `composeMs`/`putMs`/`drawMs` ÷ presents、252 present)。`texSubImage2D` の自己時間は 120.8 ms / 244 present(プロファイルの窓)= 1 present あたり 0.50 ms で put と一致。1 present = 更新の矩形 1 つ(約 0.9 Mpx、3.6 MB)。**GL 呼び出しの後に Worker の idle が来る**(§4)。

## 4. present が起きる仕組み(コードの読み + プロファイル)

- 起点は 3 つ: (1) Worker の rAF の loop(`worker.ts` `startDeskLoop`: rAF ごとに `deskTick`。`DESK_SLEEP_AFTER_MS` 画素が変わらないと sleep して rAF を止め、idle が起こす)、(2) エンジンの idle(`onEngineIdle`: 前回の tick から 20 ms 超なら `deskTick`。上限 50/s)、(3) 入力に答える inline(L1c)。
- **既定の純再生では (1) の rAF が 3.6/s しか配られない**(Worker が飽和して event loop に戻らない。コメントに『the Worker's requestAnimationFrame is not delivered while the engine keeps the Worker busy』)。present の大半は (2): idle 試行 17.7/s。
- V8 プロファイル(純再生 20 s、`p2-prof`、`tables/p2-prof-idle-ctx.md`): Worker の idle は 14.4%(2,888 ms、231 回、平均 12.5 ms)。**idle の直前は `postMessage`(522 ms)・`texSubImage2D`(512 ms)・`clear`(279 ms)・uniform 等**(= present の GL 呼び出しの後で約 45% の idle)、他は wasm 関数。GL の自己時間は 164 ms(0.8%)と小さい。= **GL を発行した後の GPU(ここでは SwiftShader)の完了待ち**が、1 present あたり約 3.6 ms(GL 呼び出し直後の idle 888 ms ÷ 244 present。`postMessage` の後を含めると 1,410 ms ÷ 244 = 5.8 ms)の Worker の idle(0283 §1 の読みと同じ向き。E11 でも残る)。
- **単一 Worker のため、present(JS)の機会は『エンジンが event loop に戻る回数』**。ミキサーの 1 ブロック(93 ms)の計算中は戻らない。

### 4b. (program) の塊 — alpha1 の 0304 との突き合わせ(PM 17:21 の依頼)

`tools/program-chunks.py`(`tables/p2-prof-program-chunks.md`): V8 の `(program)`(JS / wasm に属さない native の時間)を、連続するサンプルの塊に分けた(`p2-prof`、純再生 20 s、E11 協調、60 Hz)。

| 塊の長さ | 塊の数 | ms | (program) に対する % | 窓(20 s)に対する % |
|---|---|---|---|---|
| < 1 ms | 520 | 156 | 10 | 0.8 |
| 5〜20 ms | 10 | 187 | 12 | 0.9 |
| **20〜50 ms**(平均 27 ms) | **47** | **1,272** | **79** | **6.4** |

- (program) の合計 1,617 ms = 窓の **8.1%**、そのうち **5 ms 以上の塊が 57 個・1,459 ms(窓の 7.3%、(program) の 90%)**。alpha1 の 0304(『5 ms 以上の塊が 87〜92%』)と同じ型。
- **塊の直前のサンプル(5 ms 以上の塊 57 個)**: `texSubImage2D` 25 個(706 ms、48%)、`fle_desk_scene`(wasm 関数の `scene`、`js-to-wasm::i < scene`)12 個(277 ms、19%)、`present` 4 個(6%)、`clear`・`uniform*`・`draw`(いずれも `present` の下の GL 呼び出し)5 個(約 8%)、`postMessage` 2 個(4%)、その他。**present の経路の GL 呼び出しと scene の読みで 約 85%** — alpha1 の『直前の 50〜65% が `fle_desk_scene`、10〜20% が `texSubImage2D`』とは割合が逆だが同じ組(両方の合計は 67〜85%)。1 本の run のサンプルなので割合は run で揺れる。
- **(c) を数え直す**: 1 present の費用 = JS 2.0 ms(compose 1.43 + put 0.50 + draw 0.07)+ **GL 呼び出し直後の native の塊 `(program)` 約 6.0 ms**(1,459 ms ÷ 244 present)+ GL 呼び出し後の Worker の idle 約 3.6 ms(`postMessage` の後を含め 5.8 ms)= **約 11.6〜13.8 ms**。present 15.6/s では **約 180〜215 ms/s = Worker の 18〜21%**(先の『9〜12%』は idle の分だけで **過小だった**。訂正)。**(c) の本体は『texSubImage2D(1 present 0.89 Mpx = 3.6 MB)のあと、native でコマンドバッファの空きや GPU プロセスの処理(ここでは SwiftShader の CPU 上の GL)を待つ時間』**で、`(program)` と `(idle)` の両方に現れる。
- **SwiftShader の artefact か、本質か**: 待ちの長さ(1 回 20〜50 ms)は GL の実行を CPU が行う SwiftShader で膨らんでいる可能性が高い(**読み、未測定**。0283 の微小実験で『切り離した canvas では待たない、DOM に結んだ canvas の提示だけ待つ』)。**本質として残るのは『提示ごとに 3.6 MB をブラウザのコマンドバッファへコピーし、GPU プロセスが texture へ上げる』サイズ**(実 GPU でも 2 回のコピー)。
- **通信/変換の削減の候補(依頼者 2026-11-04 の指示に直結。実装はしていない、読み)**:
  1. **サイズ**: 1 present = 更新の矩形 1 つ(`rects` = presents)で 約 0.89 Mpx(画面の 43%)・3.6 MB、14〜16 Mpx/s(= 約 60 MB/s)。FL の実際の画素の変化(再生ヘッド・メーター)は小さいはず(**読み、未測定**)なので、矩形を細分化(タイル単位の damage)すれば upload が数分の一になり得る。現在は 1 矩形(影比較は 0263 §8 で『採用しない』としたが、**矩形の細分化は影比較の採否と別**)。
  2. **回数**: present を増やす(測定専用ページ)は費用が効果を上回る(SwiftShader の CPU +0.19〜+1.1 コア、見える frame +3.3/s)。減らす側(合体)は idle ごとの試行を減らす現行で既にほぼ。
  3. **経路**: すでに wasm heap → `texSubImage2D` で直接(`postMessage` の画素は 0、0263)。残る 1 回のコピーはブラウザ内部で、**present を別 Worker にしてエンジンの Worker が待たない**(0289 の前提と重なる)か、**サイズを減らす**以外に手が無い。

## 5. (b) は間引きか — 忠実性の点検

- **page の規則**(rAF の loop、`DESK_FALLBACK_MS` 20 ms、sleep)は、compositor(DWM 相当)としての表示の規則で、**FL の挙動の代行ではない**(class・窓の種類・位置の分岐は無い)。純再生の present は 15.6/s で 50/s の上限・rAF の 60/s の上限のどちらにも当たっていない = **規則が効いて間引かれているのではない**(操作中は別、下の測定専用ページの結果)。
- 効いているのは **Worker の飢餓**(上)。直す道は (i) UI を別 Worker にする(0289 の実スレッド版の前提と同じで、その場合 present の機会は 49/s に増え、present 32〜36/s が実測〔E9、Hz 60〕)、(ii) present(scene の読み + GL)だけを別 Worker に出す(`fle_desk_scene` は runtime の表を読むので、実スレッド版と同じ GIL・共有メモリが前提)。(ii) の通信は『pixel の bucket relay』ではなく共有メモリの直読みで、**通信/変換の削減**の範囲(忠実性は保たれる)。**実装はしていない(読み)**。
- **測定専用ページ(`DESK_FALLBACK_MS = 0`)の結果(§2・§3)**: page の**20 ms のゲート(`DESK_FALLBACK_MS`)と Worker の飢餓**は、純再生では present を 4/s(約 20%)しか減らさず、**操作中(フェーズ c)は変化の半分以上(35.6 → 16.3/s)を見逃す**。ゲートを外す(idle ごとに present)のは page の表示の規則(compositor としての提示頻度)の変更で、FL の挙動の代行ではなく、class・窓の種類の分岐は無い。**ただし費用がある**: present が増えるぶん SwiftShader 等の CPU が b で +0.19 コア(0.95 → 1.14)、c で +1.1 コア(1.02 → 2.11)増え(実 GPU では GPU 側へ)、Worker の JS は 1 present 1.2〜1.8 ms を払う。compositor が 1 vsync に 1 frame しか swap しないので、見える更新の増加は小さい(b で +3.3/s)。= **『ゲートを外す』は通信/変換の削減ではなく提示頻度のポリシー**。直すなら『Worker に present を頼む回数』でなく **UI を別 Worker にして飢餓をなくす**(0289)のが本筋、という読み。

## 6. 未取得 / 次の測定(Chrome の窓が取れ次第)

1. 既定の再現 1 本(load と run 間差を知る)。
2. 実スレッド版(0289)の bundle で 120 Hz 1 本(UI 専用 Worker で 120 を渡すとどうなるか。E9 の実スレッド bundle で、E11 ではない。E11 の実スレッド bundle は 17:03 からビルド中)。
3. 測定専用ページの compositor の frame をフェーズ c で取り直す(trace の buffer が溢れた。カテゴリを絞る)。
4. 実 GPU(GTX 980)での (c): 依頼者の機械の beacon(`webglRenderer`)と present の ms。

## 7. 実 GPU の上限(読み、未測定)

- (c) は SwiftShader の GL 完了待ち(1 present あたり JS 2.0 ms + `(program)` 約 6.0 ms + idle 3.6〜5.8 ms = 約 11.6〜13.8 ms、Worker の 18〜21%、§4b)で、実 GPU(GTX 980)では GL の完了待ちが小さい(読み、未測定)。**取り除ければ Worker の約 2 割が戻り、単一 Worker の仮想 s/実 s が上がる方向**(alpha1 の 0296 の式 v ≒ idle の割合 ÷ 0.265 を使えば 0.69 → 0.8 台。読み)。present の回数への効果は +1〜2/s(機会の数は変わらず、FL の変化の頻度で頭打ち)。120 fps の差(104/s)の大半は (a) で、(c) はその主因ではない。
- 120 fps(= 画素が変わる frame が 120/s)の上限は、**FL 自身が刻みごとに画素を変えるか**で決まる。idle では 99% の刻みで変えない(0.4/64.2)。実スレッド E9(Hz 60)では刻み 60/s のうち present 34/s(57%)。= **120 Hz を渡して UI 専用コアで v = 1 にしても、present は 60〜70/s 程度(読み)**。実 Windows の FL が刻みごとに描くのかは参照できる値が無い。

## 8. S-gpu7-N と自己申告

- **S-gpu7-37**: 3 つのフェーズの長さと位置(idle 15 s、純再生は Space の 8 s 後から 15 s、操作はフェーダー 5 往復 + wheel 30 回)、フェーダーの座標(470, 830 → 770)と wheel の座標(1500, 500)は私の選択。
- **S-gpu7-38**: 120 Hz を渡す方法は Chrome の起動フラグ `--disable-frame-rate-limit --disable-gpu-vsync`(0170 の `FL_CHROME_ARGS`)。page は測定値の 185.5 Hz を 120 に丸めた。製品の `hz.ts` に旗・URL パラメータは無い。
- **S-gpu7-39**: FL の刻み(slice/s)は協調エンジンの pulse の thread 0 のスライス数。
- **S-gpu7-40**: 測定専用ページ `srcE11M` は `worker.ts` の `DESK_FALLBACK_MS` を 20 → 0 にしただけ(idle ごとに present を試す)。製品の page・runtime には入れない。
- **S-gpu7-41**: (program) の塊 = 連続する `(program)` サンプルの最大の連なり。『直前』= 塊の前の最後の `(program)`・`(idle)` でないサンプルの関数。
- **S-gpu7-42**: E11 の実スレッド bundle(go/no-go #2 用)は `rt-on-e11.diff`(mt3、0301)を 7373cd55 に当てた木で `GIL=1` の冷間 CI 段階ビルド(`CI_JOBS=3`、nice 19、17:03〜17:21、objects 456 s・link 600 s、load 3〜11)。bundle `fl-engine-ci-03612d9c`(engine wasm `engine.b61382b87de1.wasm` 142,985,941 B、E11 本番 143,086,598 B)、`check_i1.py` rc=0。`out-gpu2/bundle-rt11/` に残した(218 MB)。
- 自己申告: (0) **(c) の最初の見積り(Worker の 9〜12%)は `(program)` の塊を数えておらず過小だった**。PM 17:21 の alpha1 0304 の指摘で数え直し、18〜21% に訂正した(§4b)。
- 自己申告: (1) 各 run 1 本で再現が無い(load は 3.7〜6.6、opt1 の Chrome が最初の run の冒頭に重なった)。(2) trace は buffer が溢れ、フェーズ c の compositor frame は下限。(3) 『present 以外のダメージ』の内訳は未分離。(4) 0289 の最終で出した『税 +40〜70%』の交絡の誤りは別途訂正済み(info 5)。
- 自己申告(続き): (5) ci1 に断って Chrome を 3 窓(16:18〜16:33 の 3 本、16:58〜17:02 の 1 本)で使い、17:03 からのビルドは ci1 の『counted でない run の時間帯』の合意に従った。ビルドの load は 3〜11(自分のビルド + 他の担当)。(6) 後片付けで `runs/*` を消したので、trace の全体は残していない(frame の事象だけを `logs/p2-*/trace-*.frames.json.gz` に残した)。(7) 私の preview server(port 4994、4995)は最後に PID で止める。


## PM の決定と取り込み(2026-11-06 17:28、docs のみ。製品の差分なし)

取り込み: gpu1 の最終(17:26)の文書・表・道具を `docs/investigations/gpu7/0297/` に置いた(生のログ約 4.9 MB は入れない。原本は `lift/out-git/gpu7-results/0297/logs/`)。測定は実 Chrome 156 headless の実エンジン(本番 E11 の wasm = `engine.49bbf63df894`、協調 1 Worker)、この機械の SwiftShader、load 3.7〜6.6。各条件 1 本・再現なし。測定専用ページ(`DESK_FALLBACK_MS = 0`)は製品に入れない。**依頼者の機械・実 GPU は 0 件**。

事実として受け取る:
1. 純再生(60 Hz): 画素が変わった present は 15.6/s(120 の 13%)、FL の UI の刻み 45.7/s、仮想 s/実 s 0.69、Worker 0.86 コア(飽和)。**120 Hz を単一 Worker に伝えると 11.7/s・仮想 0.55 に悪化**(刻み 72.3/s)。idle の刻みは Hz に比例(alpha1 の 0299 と一致)。
2. 差 104.4/s の分解(純再生): (a) FL が描いていない 74.3/s(渡した Hz 60 が 60/s + CPU 律速 14.3/s)、(b1) FL が画素を変えない刻み 約 26/s、(b2) 機会不足 約 4/s(試行を 21 → 315/s にしても present は 15.6 → 19.4/s)、(c) present の実行は **Worker の 18〜21%**(第 1 報の 9〜12% は `(program)` を数えておらず過小 — 訂正)。
3. **再生 + 操作では、既定のページは FL の画素変化の半分以上を見逃す**: 既定 16.3/s に対し測定専用ページ 35.6/s(刻みの 61%)。20 ms のゲート + Worker の飢餓。純再生では既定 15.6/s に対し FL の画素変化は約 19〜20/s(既定は約 80% を提示)。
4. **FL の見える更新は仮想 1 秒あたり約 21〜27 で Hz にほぼ依らない**(読み)。(program) の塊は 20〜50 ms の 47 個が 79%、直前は texSubImage2D 48%・fle_desk_scene 19%・present/GL 約 18%(alpha1 の 0304 の観察と同じ組)。1 present = 3.6 MB(0.89 Mpx、矩形 1 つ)のコピーは実 GPU でも残る(読み)。

判断:
1. **P2 の「120 fps」の測り方**: この機械で測れるのは (i) FL の刻みが表示 Hz に追従するか(追従する、alpha1 0299)、(ii) FL が画素を変えた frame のうち提示された割合(純再生で約 80%、操作中で約 46%)、(iii) 入力の遅延(ci1 の lat)。**FL 自身の画素変化が仮想 1 秒あたり 21〜27 なら、リテラルな「120 回/s の更新」は UI 専用コアと実 GPU があっても FL が出さない**(読み。実スレッド E9 の同じ音楽位置での測定が要る)。これは依頼者の目標を変える判断ではなく、測定の軸の整理。P2 は未達のまま(提示の取りこぼし、入力遅延、Worker の飽和)。
2. **単一 Worker に 120 Hz を渡さない**。製品は rAF を測って 60/75/90/120 に丸めて dmDisplayFrequency に渡す(決定 0268)。**依頼者の機械のモニタが 75 Hz 以上なら、今の製品は FL の刻みを増やして P3 を悪化させる恐れがある**(75 Hz で刻み +25%、120 Hz で +98%)。依頼者の機械の Hz は 0 件で未確認。UI 専用コアが無いうちの扱い(Hz の上限を 60 に固定する等)は忠実性との釣り合いが要る判断で、E13 より前に alpha1 と詰める(別決定)。
3. **改善の候補(実装しない、順序つき)**: (1) present を別 Worker に出してエンジンの Worker が GL の完了を待たない(実スレッド版の前提と重なる、go/no-go #2 の結果で判断)、(2) 矩形のタイル細分化によるコピーの縮小(FL の実変化の大きさは未測定)、(3) 提示の回数を増やすのは費用>効果(SwiftShader +0.19〜+1.1 コア、見える frame は +3.3/s)。経路は既に wasm heap → texSubImage2D 直で、postMessage の画素は 0(依頼者の 2026-11-04 の指示どおり)。
4. gpu1 の次: E11 の実スレッド bundle(`fl-engine-ci-03612d9c`、142,985,941 B、GIL=1、`check_i1.py` rc=0、ビルド 17:03〜17:21)で go/no-go #2 の測定(5 分再生必須、音楽位置を揃えた比較、成功条件は決定 0289 の PM 節)。
5. 未確認: 各条件の再現、実スレッド版の 120 Hz、実 GPU、測定専用ページのフェーズ c の compositor frame(trace の buffer 溢れ)。
