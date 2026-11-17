# 0182: P2(UI 120 fps・操作遅延ほぼなし)— 画面の更新周期を決める入口 `?hz=`、JS グルーの削減(結果不変)、60 Hz と 120 Hz の実測。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(決定 0182)。調査・設計・実装・測定・テスト・文書は作業者(`p2a`、2026-11-03 JST 14:56〜)。作業は HEAD **7757f60**(ブランチ main)の `git archive`(`lift/out-p2a/base/`)で行い、差分 `p2a.diff` は `app/` の 17 ファイルだけを変える(HEAD は途中で 996bd2d まで進んだが、`app/` では別の担当が `app/scripts/build-engine-bundle.sh` に 13 行足しただけ。適用の確認は 996bd2d で行った)。**`lift/`・wasm・生成 C・`app/worker/`・`app/scripts/`・`app/src/audio/`・`app/src/plugins/`・既定の manifest は変えていない**(ゲストの動作は変わらない)。

**冒頭の三点**:

1. **何を実測したか**: 実エンジン(隔離 headless Chrome 156、この機械、空のプロジェクト、load 3〜22)で、(a) 構成 D(作業開始時の既定の入口のバンドル)に HEAD の前端と本差分の前端を**交互に 5 組**を 2 系列(glueB: 窓一覧の間引き込み、glueC: PM の指示で間引きを既定 OFF にした最終の既定): アイドルと再生の Worker・ページ主スレッドの CPU(idle 1 回あたり µs、`/proc` のスレッド別と CDP プロファイル)、ゲスト時刻列のハッシュ(idle 番号・仮想時計・次の期限)、主窓の SHA-256・窓一覧のダイジェスト(同じ仮想時刻、`?stamp=0` も)。(b) 構成 Dp(`FLE_DWM_HZ` を読む構成)に `?hz=60` と `?hz=120` を**交互に 3 組**: 窓の更新/s、Worker・ページの CPU、単発の移動とクリックの入力→更新、仮想/実、wasm メモリ。さらに Dp の 60/120 Hz で CPU プロファイルとページの `requestAnimationFrame` の間隔、`?hz=auto` の値(headless の rAF)、D に `?hz=120` を付けた run、PM が 16:11 に切り替えた今の既定の入口(F1xn)に `?bundle=` 無しで `?hz=60`・`?hz=120` を付けた run、再生 180 s の wasm メモリ(Dp の 60/120 Hz、0174 前の F1x、0174 入りの F1xn)、主窓のフィルムストリップ。
2. **結論**: 
   (a) **入口**: `?hz=<24〜1000 の整数>` と `?hz=auto` を実装した(`app/` だけ。無指定はエンジンの環境に何も入れず、既定の入口の挙動・面・テストは変わらない。不正値はダウンロードの前にエラー表示)。**構成 D の wasm は `FLE_DWM_HZ` を読まない**ので、D に `?hz=120` を付けても効かない(実測: 再生中の更新 52/s のまま、wasm に文字列 0 件)。`FLE_DWM_HZ` を読む構成 Dp(fin1)に `?hz=60`・`?hz=120` を交互 3 組(空のプロジェクト、load 5〜20): 窓の更新/s(ページの canvas に描いた回数、再生)**59.0 → 117.9**(定常の 12 s では 62.4 → 125.0)、Worker の CPU(再生)0.182 → 0.263 コア・アイドル 0.116 → 0.163 コア、ページ主スレッド 0.105 → 0.106 コア、仮想/実 1.0000 → 1.0000、単発の移動→更新 p50 3.9 → 5.1 ms、クリック→更新 p50 10.0 → 7.4 ms、エンジン処理 p95(移動 18〜20 ms、クリック 34〜36 ms)は 60/120 で同じ。PM が 16:11 に既定の入口を F1xn(`FLE_DWM_HZ` を読む)へ替えたので、`?bundle=` 無しの今の既定の入口でも測った: 窓の更新/s 62.5 → 125.0(アイドルでも)、Worker CPU(アイドル 0.219 → 0.312、再生 0.314 → 0.488 コア)、最初の idle 6.72 → 7.37 s(1 組、load 10〜15)。**発見: F1xn は入力の無いアイドルでも主窓を毎フレーム描き直す(62.5/s、120 Hz で 125/s。D と Dp は 0.7/s)ので、アイドルの CPU が Dp の約 2 倍(Worker)・6 倍(ページ主スレッド)で、PM が §4c に書いた『アイドルの更新 55/s はポインタ移動を含むため』は当たらない**
   (b) **120 fps には『窓の更新』としては届く(125/s)が、『ディスプレイが 120 fps で表示する』は確かめられない**: この機械の headless Chrome の `requestAnimationFrame` は `?hz=` に依らず 60.0〜60.1 回/s(間隔 16.67 ms、p99 も最大も)。最大の障害は **ゲストの CPU**(Worker の idle あたり 79〜89% が wasm)、とくに**入力 1 回あとの『重い周期』**(エンジン処理 p95 が移動 18〜20 ms・クリック 34〜36 ms = 120 Hz のフレーム 2〜4 個分)。JS のグルーとブラウザの描画/転送(ページ主スレッド 0.1 コア前後)は主因ではない。
   (c) **JS グルーは、clk1 の後はもう小さい**: HEAD で Worker の idle 1 回 約 150 µs のうち JS の名前のある関数+GC が約 7%(`(program)` を含む wasm 以外は約 14%)、wasm が 79〜89%。**ui1 の『idle あたり約 0.6 ms』は clk1 の前の値**(訂正)。結果を変えない削減(窓の表の stamp、`timeOrigin`、ページの DOM の書き込み抑制)で、構成 D(旧既定)の idle あたり Worker+ページが アイドル 179.5 → 154.7 µs(対の差の中央値 −17.1、5/5)・再生 165.4 → 147.7(−15.8、5/5)(出荷する既定、交互 5 組)。エンジンの窓一覧の間引き(`?poll=<ms>`)は**効果を実証できなかった**(wasm の対の差の中央値 −7.2 µs、5 組中 3 組)ので、PM の指示で既定 OFF の opt-in。ゲスト時刻列(idle 番号・仮想時計・次の期限)のハッシュは glueB の 10 run で 53/53、glueC の 10 run で 52/52(1 本 50/50)一致(120 Hz は `?hz=120` と手書きの `?env=` が 47/47)、主窓の面は Enter の前(a3000・a4000)で、HEAD の前端・最終・`?stamp=0` の 6 本とも native の基準 `4fa213b07d79` と一致(Enter の後は絵が数枚を巡るので『フィルムストリップ』で順序と時刻を比べた: **区別できなかった**。HEAD の前端・最終の各 3 本のフィルムストリップ(12 s)で、同じ前端どうしでも絵の順序と集合が揃わず(読みが約 26 回/s)、3 本ずつの和集合は前 26 枚・後 25 枚・共通 25 枚、後にしか現れない絵 0 枚、前にしか現れない絵 1 枚(1 本に 1 回だけ)。Enter の後の絵が前後で同じとは言えない(未確認))。
   (d) **再生中の wasm メモリ**: Dp(0174 の前)では wasm メモリが再生の更新の速さに比例して +19〜20% の階段で増える(120 Hz: 再生の約 40〜50 s 後に 1,029 → 1,223 MB、さらに約 75 s 後に 1,468 MB。60 Hz: 約 100〜110 s 後に 1,228 MB)。階段の値は決定 0174 の記録(1,028.9 → 1,227.8 → 1,473.4)と一致し、120 Hz は 60 Hz の約 2.2 倍の速さで進む。120 Hz のままだと約 5〜6 分で上限 2 GiB に届く見込み(読み)。F1x(同じく 0174 の前)も同じ階段(1,030 → 1,229 → 1,475 MB)。**0174 入りの F1xn(= 今の既定の入口)の 120 Hz・180 s: 平坦のまま(1 run)。wasm メモリは最初の標本(起動の 15 s)から 1,033.9 MB で、225 s の最後まで 15 点すべて同じ(階段なし。Dp と F1x は同じ長さで 2 段上がった)。事前の予測の『平坦』は当たり、『931 MB』は外れた(120 Hz の run は起動の 15 s までに 931 → 1,034 MB に上がっていた〔60 Hz の短い run が 931 MB だったことからの読み〕。原因は未調査)。60 Hz の 180 s と、より長い再生・複数 run は未測定**(『平坦』という予測を測定の前に書いた)。
   (e) 途中で見つけて直した誤り: `?hz=auto` の最初の版は rAF をダウンロードと並行して測り、60 Hz の画面を 30 Hz と測った(ページが忙しく rAF が間引かれた)。ダウンロードの前に測る形に直した(起動が約 1.1 s 遅れる、`auto` だけ。直した版は 61 間隔・中央値 16.66 ms = 60.0 Hz と測った)。
3. **未確認**: 所定のプロジェクト(`serina1-serum2.zip`、アプリに FLP を開く経路が無い)での 120 fps。依頼者の機械・120 Hz 以上の実ディスプレイ・実 GPU(headless の rAF は 60 回/s なので、表示側のフレーム落ちは未測定、`?hz=auto` の 120 Hz の画面での値は未実測)。他のブラウザ。隠れたタブでの `?hz=auto`(rAF が止まる場合の 60 Hz へのフォールバックは単体テストのみ)。ゲストの関数単位の CPU を 120 Hz で測り直すこと(名前つきの wasm と node が要る)。窓一覧の間引きの効果(実証できず)と、描画も入力も無い窓の変化が最大 40 ms 遅れて見えることの影響。音(P3)。エンジン側の `fle_idle_lib.js` の削減(再リンクが要る)。 60 Hz と 120 Hz でゲストの描く絵が同じか(同じ仮想時刻の主窓の SHA 比較は未実行)。F1xn(今の既定の入口)でのグルー削減の前後(D で測った)。
F1 系が毎フレーム窓を描き直す理由と、描き直される矩形の大きさ。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実(2026-11-03 JST)。**読み**＝コード・文書を読んだ解釈で、確かめていない。**出典なしの選択**＝根拠が無く作業者が選んだ値(`S-…`、§7 に撤回条件つき)。**未確認**＝確かめていない。**この機械(16 コア、Linux VM、隔離 headless Chrome 156、SwiftShader)の値であって、依頼者の機械ではない。**

## 0. 先に書くこと(進捗の誠実性、AGENTS.md / `docs/policies/progress-integrity.md` に従う)

**(1) 実エンジンが実行された事実(実測)**: 構成 D(既定の入口のバンドル `fl-engine-b1-D`)と構成 Dp(`fl-engine-fin1-Dp`)を、隔離 headless Chrome 156 で**実際に動かして**測った。すべての実行で `engineKind` = `emscripten-fruity-v1`、STAND-IN の帯なし、最初の idle に達し、Enter でウィザードを閉じた後の主ウィンドウ(`TFruityLoopsMainForm`)を観測した。入力は Chrome の CDP `Input.dispatch*Event`(本物のブラウザ入力)。ゲストの時刻列の比較(§4.1)と主窓の SHA は、実エンジンの出力と、決定 0147 が記録した native の SHA(出典つきの記録)との照合である。

**(2) 代役・再生・推測を含む範囲**: 測定値には代役・モック・画像の再生・事前生成物・テスト用の期待値は**無い**。『読み』『推測』『未確認』と書いた所だけが推測(§9)。**所定のプロジェクト(`serina1-serum2.zip`)は開いていない**(アプリに FLP を開く経路が無い)ので、P2 の値は**空のプロジェクト**の主窓のもの。依頼者の PC ではなく**代理環境**(この機械: 16 コア、Linux VM、headless Chrome 156、SwiftShader、localhost)の値。

**(3) 機械の負荷**: この作業の間、他の担当のビルドと走行で 1 分平均の load は 4〜22 だった。各表に load を併記した。**load が 8 を超えた窓の壁時計の値(更新/s、仮想/実、入力遅延)は参考値**で、CPU 秒(スレッド別の `/proc`)と回数・比を主にした。実際には load 15〜20 の窓でも仮想/実は 1.000 を保った(clk1 の駆動)。

**(4) 既定の入口と、規則**: 既定の入口 `/assets/manifest.json` と 5173 のサーバーには触っていない。60/120 の測定は `?bundle=/assets/fl-engine-fin1-Dp/manifest.json&hz=60|120` という明示のバンドル指定で行った(既定の構成 D は `FLE_DWM_HZ` を読まないため。§2)。**この点は指示(『既定のバンドルに ?hz=120 を付けるだけで測る』)と違う**。違う理由は上の通りで、『D では `?hz=` が効かない』ことも実測した(§4.5)。その後 PM が既定の入口を F1xn(`FLE_DWM_HZ` を読む)へ切り替えた(16:11)ので、**最後に、`?bundle=` 無しの今の既定の入口に `?hz=60`・`?hz=120` を付けた run を追加した**(§4.5)。

## 1. 方法と環境

**環境**: この機械(16 コア、Linux VM)、隔離 headless Chrome 156(使い捨てプロファイル、キャッシュ無効、2400x1300、SwiftShader、localhost)、`nice -n 5`、同時 1 本。ページは自前の `vite preview`(`lift/out-p2a/dist-*` を `--outDir` に、`FL_ASSETS_DIR=lift/out-websrv/assets` は読み取りのみ、計測ビーコンのログは `runs/*/perf-log.jsonl`)。**既定の入口 `/assets/manifest.json` と 5173 のサーバーには触っていない。**

**ブラウザの前端(dist)**(`logs/dist-sha.txt` にビルドのハッシュ): `dist-before` = HEAD 7757f60 の `git archive` をそのままビルド。`dist-after-A` = 途中の版(`?hz=`、表の stamp、`timeOrigin`、DOM の書き込み抑制。窓一覧の間引きも `?stamp=` も無い)。`dist-after-B1` = 同じく窓一覧の間引き無し・`?stamp=` 有り(最初のキューの 1 組目)。`dist-after-poll` = **窓一覧の間引きが既定 ON の版**(`dist-final2`、最終のソースで `?poll=40` を付けたのと同じ動作)。`dist-after` = **最終のソース**(PM の指示で間引きは opt-in `?poll=<ms>`、既定 OFF。`?hz=auto` はダウンロードの前に測る)。どの run がどの版か: **hzA**(60/120 の 3 組)= `dist-after-A`。**glueB・profA・hz120hash・soak(Dp 60/120)・D-hz120・auto(最初の版)・sha(最初の 3 本)・abpoll** = 間引きが既定 ON の版(`dist-after-poll` / `dist-final2`)。**glueC・sha2・sha4・soak(F1x、F1xn)・auto2・defE** = 最終のソース `dist-after`(sha3 は時間切れで実行していない)。60 と 120 の比較(hzA、profA、soak)はどれも『同じ版の中』なので、版の違いは比較に入らない。 なお `dist-after` は、`?poll=` の解釈を純関数 `parsePollParam` に切り出して単体テストを付ける前の版(`main.ts` の 1 行だけの違いで、動作は同じ)。

**エンジンのバンドル**:
- `fl-engine-b1-D`(構成 D、**作業の開始時の**既定の入口。wasm `80a5c36f6cbb`): `FLE_DWM_HZ` を読まない。グルーの前後(glueB、glueC)と、`?hz=120` が効かないことの確認に使った。
- `fl-engine-fin1-F1x`(構成 F1x。Blend2D の置き換え、simd128 なし。**決定 0174 の前の runtime**〔fin1 の決定 0176〕): 再生中のメモリの比較に使った。
- `fl-engine-fin1-F1xn`(構成 F1xn。**PM が 16:11 に既定の入口へ切り替えた**。wasm `16bd5d9fe402`、`FLE_DWM_HZ` が 3 件): 最後に、`?bundle=` 無しの既定の入口に `?hz=60`・`?hz=120` を付けた run(defE)に使った。
- `fl-engine-fin1-Dp`(fin1 の構成 Dp、wasm `50ddc8d71430`。決定 0166 の `FLE_DWM_HZ` を含む〔wasm の文字列 3 件〕。**マニフェストの説明は『HEAD 75cfbbf』だが、PM の追記では木は b4f1d3c/07ab9f4 で決定 0174(GDI スロットの再利用、75cfbbf)より前**。どちらが正しいかは私は確かめていない。再生中のメモリの成長〔§4.8〕は PM の読みと整合する〔読み〕): 60/120 の系列(hzA、rafA、profA、soak、auto)。D との違いは dispatch キャッシュ(0163)など。**D と Dp を直接比べた数字は『参考』**(別の構成)。60 と 120 の比較はいつも同じ Dp の中。

**手順**(`app/test/e2e/p2a-run.mjs`。clk1 の `clock-follow.mjs` の写しに、クリック・CDP プロファイル〔Worker とページ主スレッドを同じ区間で〕・rAF の計数・wasm メモリの 15 s の時系列を足した): ページを開く → 最初の idle → 3 s → 段階 `wizard`(ウィザードが出ている間。25 s) → Enter でウィザードを閉じる → 落ち着くまで待つ(`enter-settle`) → `main-idle`(入力なし 60 s) → `mouse`(連続したポインター移動 20 s) → `taps`(離れた位置への単発の移動 30 回、0.7 s おき) → `clicks`(ボタンの押下と離す 20 回、ミキサーの Insert 3・4 の名前、チャンネルラック、プレイリストの格子。ツールバーは決定 0171 の壁で止まるので避けた) → `play`(Space で再生 60 s、Space で停止)。各段階で: ページが窓の面を描いた回数(`PerfRecorder.update` = 方式 B で Worker から来た矩形をページの canvas に `putImageData` した回数)、エンジンの idle 回数、Worker とページ主スレッドの CPU(`/proc/<pid>/task/<tid>/stat` のスレッド別、100 Hz)、仮想時計/実時間(Worker の最後の idle の時計)、`LatencyTracker` の入力→次の更新(clk1 が直した版)、wasm メモリ、load。

**言葉の約束**: 『窓の更新/s』= ページが窓の canvas に矩形を描いた回数/s。ゲストが窓を再描画し、Worker が変換して送り、ページが `putImageData` した回数であって、**ディスプレイが表示したフレーム数ではない**。ページの実フレームは `requestAnimationFrame` の間隔(§4.7)で別に測った。

## 2. 120 Hz の入口(`?hz=`)

**実装した場所**(すべて `app/`。`lift/` は無変更):
- `app/src/engine/hz.ts`(新規、純関数): `parseHzParam`(`?hz=` の解釈)、`roundToStandardHz`、`measureFrameRate`(偽の時計を渡せるフレーム間隔の測定)、`hzEnv`、`hzMismatch`、`applyHz`(要求をエンジン環境へ)。
- `app/src/main.ts`: `?hz=` を読み、不正なら**ダウンロードの前に**エラー画面(`where: URL parameter hz`)。`auto` なら、マニフェストを読んで実エンジンと分かった直後、**ダウンロードを始める前に** `requestAnimationFrame` を約 1.1 s 測る(最初の版はダウンロードと並行して測ったが、ページがダウンロードの展開と検査で忙しく、60 Hz の画面で 30 Hz と測った〔21 間隔、中央値 33.33 ms〕。そのため先に測る形にした。起動が約 1.1 s 遅れる、`auto` だけ)。実エンジンのときは、Worker へマニフェストを渡す直前に `manifest.engine.env` へ `FLE_DWM_HZ` と `FLE_DISPLAYSETTINGS_FREQUENCY` を**同じ値で**入れる(既存の仕組み: `FruityEngine` の `preRun` が `manifest.engine.env` を `ENV` へ写す)。`window.__app.hz` に `{request, hz, measurement, warning}`。
- 形: パラメータ無し = 環境に**何も入れない**(エンジンの 60 のまま。既定の入口の挙動・面・テストは変わらない)/ `?hz=<整数 24〜1000>` / `?hz=auto`。不正値(空、0、23、1001、小数、符号、先頭の 0、空白、16 進、`Auto`、2 回指定)は拒否。
- 2 つの変数が食い違う環境(`?env=FLE_DWM_HZ=120&env=FLE_DISPLAYSETTINGS_FREQUENCY=60` など)はログに warn(ランタイムが stderr に出すのと同じ警告をページでも出す)。DISPLAYSETTINGS だけを 60 以外にした場合も warn(DWM_HZ は既定の 60 なので食い違う)。

**重要な発見(構成 D は `FLE_DWM_HZ` を読まない。その後、既定の入口が読む構成に替わった)**: 作業の開始時の既定の入口の構成 D(`lift/out-websrv/assets/fl-engine-b1-D/engine.80a5c36f6cbb.wasm`)は、決定 0166 の前の HEAD(071c53f + b1.diff)から作られていて、**`FLE_DWM_HZ` を読まない**(wasm の中の文字列 `FLE_DWM_HZ` が 0 件、`FLE_DISPLAYSETTINGS_FREQUENCY` が 0 件)。したがって D のバンドルに `?hz=120` を付けても何も変わらない(§4.5 で実測)。そこで 60/120 の系列は、fin1 の構成 `Dp`(`lift/out-websrv/assets/fl-engine-fin1-Dp/`、`FLE_DWM_HZ` が 3 件)を `?bundle=/assets/fl-engine-fin1-Dp/manifest.json` で指定して測った(既定のマニフェストは触っていない)。**PM は 2026-11-03 16:11 に既定の入口を構成 F1xn(`fl-engine-fin1-F1xn`、wasm `16bd5d9fe402`、`FLE_DWM_HZ` が 3 件)へ切り替えた**(コミット 4933389。私は 17:00 ごろ `git log` で気づいた)。= **今の既定の入口では `?hz=` が効くはず**で、`?bundle=` 無しの既定の入口に `?hz=60`・`?hz=120` を付けた run を最後に追加した(§4.5)。グルー(§3、§4.1〜4.3)の測定は旧既定の D で行った(フロントエンドの変更なので、どのバンドルでも同じ種類の削減になるはずだが、F1xn では測っていない)。

## 3. JS グルーの削減(結果を変えないもの)

### 3.1 何が残っていたか(HEAD の前端、実測、§4.1 の表)

ui1(決定 0166)の「idle 1 回あたり約 0.6 ms(Worker 403 µs + ページ 230 µs)」は clk1(決定 0170)の**前**の値で、idle ごとの `postMessage`(1,000 回/秒)と、窓一覧(エンジンが返すのは最大 32 窓)の `TextDecoder` による読み取りが毎 idle だったことが主だった。clk1 が idle の報告を 40 ms に間引き、窓一覧を `seq` が同じなら読まないようにしたので、HEAD ではグルーは小さい: **Worker は idle 1 回 約 150 µs(うち wasm 129 µs〔ゲスト+ランタイム+JSPI のフレーム〕、`(program)` 10 µs、JS の名前のある関数 9.8 µs、GC 0.7 µs)、ページ主スレッドは 約 21 µs**(CDP プロファイラ、200 µs 間隔、アイドル、load 約 5)。JS のグルー(名前のある関数+GC)は Worker の 7%、残りの 86% は wasm である。

JS の自己時間の内訳(アイドル、µs/idle、HEAD): `setTimeout` 2.0、`now`(performance.now)1.1+0.4、`postMessage` 0.9、`get buffer`(`memory.buffer`)0.9、GC 0.7、`readFields` 0.5、`get i32` 0.4、`wasm-to-js` 0.4、`queueMicrotask` 0.3、`snapshot` 0.3。窓の表を読む関数(`readFields`、`get buffer`、`get i32`、`snapshot`、`entries`、`entryWords`)の合計が約 2.5 µs で、JS の中では最大の塊である。

### 3.2 やったこと

1. **窓の表(surface table)を、変わっていなければ読まない**(`app/src/engine/surface-sync.ts` 新規、`surface-table.ts` に `TableStamp`)。以前は、エンジンの idle ごとと面のイベントごとに `pumpAll()` が、(a) `table.entries()` で全枠(32)を読み、生きている枠ごとにオブジェクトを作り、`JSON.stringify` して前回と比べ(窓の一覧が変わったか)、(b) もう一度 `entries()` を読んで `take()`(ダーティ矩形の取り出し)をしていた。1,000 回/秒のうちほとんどで表は変わっていない。**stamp**: 各枠の `lseq`(語 0)と `gen`(語 17)と、ヘッダの `live` の数だけを `Int32Array` に控え、**前回『完全に見終わった』ときと全部同じなら**この 2 つを丸ごと飛ばす。
   - **同じ結果になる理由**(`lift/harness/fruity_surface.c` の全書き手を読んだ結果。S-stamp-fields): layout の語(1〜15、24〜27)を書くのは 3 か所だけ。(1) `note_write` の「新しい面/大きさの変更」の枝(`lseq` を前後に 1 ずつ増やす)、(2) `refresh()` の枠の解放(`lseq` を前後に増やし `live` を減らす)、(3) `layer_sync`(`lseq` を前後に増やす)。`x`/`y`/`style`/`exstyle`/`visible` を書く `window_fields` は `note_write` からしか呼ばれず、`note_write` は必ず最後に `gen` を増やす。ダーティ矩形を書くのも `note_write` だけで、その前に `gen` を増やす。つまり『`lseq`・`gen`・`live` が同じ』なら layout の語もピクセルも変わっていない。ダーティ矩形は前回の pump が全部 `take()` している。
   - stamp を無効にする場合: 新しい canvas を結びつけたとき(`doBind`、方式 B の `bindNew`)、方式 B の背圧で pump を見送ったとき(`held()`)、書き手が途中(`lseq` が奇数)のとき、最初の 1 回。スタンドインの表は規律を約束していないので使わない(S-stamp-real-only)。
   - **検証**: 単体テスト `surface-sync.test.ts` が、`fruity_surface.c` の書き込みを写した JS のモデル(新規面、サイズ変更、移動、非表示、layer 変更、解放、idle の layer_sync、ピクセルの書き込み、背圧、後から来る canvas の bind)で、**ランダムに 2,000 idle × 12 シード**、stamp あり・なしの 2 つの経路に同じ操作を与え、(i) 表の一覧の通知、(ii) bind、(iii) 描画の列(lid・サイズ・位置・可視・矩形・layer)が**完全に同じ**であることを確かめる(stamp なしは変更前の `pumpAll` と同じ動作)。4 つの変異(`gen` を見ない、`lseq` を見ない、`live` だけ見る、bind で無効にしない)がすべて検出される。
2. **エンジンの窓一覧(`fle_av_poll`)を、変わりうるときだけ読む — opt-in `?poll=<ms>`、既定は従来(毎 idle)。効果は実証できなかった**(`app/src/engine/appview-poll.ts` 新規、`worker.ts`)。`fle_av_poll()`(`lift/runtime/fle_appview.c`)は**呼ぶたびに**構造体を作り直す(最前面から窓を歩き、各窓のクラス名とタイトルを写し、直前の構造体と `memcmp` して番号を付ける)。Worker はこれを**エンジンの idle ごと**(毎秒 1,000〜2,000 回)に呼び、ほとんど何も変わっていないと知るだけである。clk1(決定 0170)は JS 側の読み取り(`TextDecoder`)を `seq` が同じなら省いたが、wasm 側の `fle_av_poll` の実行は毎 idle のままだった(= §4.1 の『wasm』の中の、JS が呼んで起こしている仕事)。窓一覧の使い道はカーソル・ポインター・デスクトップの窓の配置・パネルで、ゲストには届かない。そこで『読むのは、入力が処理中の idle〔`IdleDriver.awaitingCount + queued > 0`〕、表を完全に見た idle〔何か書かれた〕、面のイベント、最初の 1 回、どれも無くても `<ms>` に 1 回』とする形を作り、**PM の指示(中間報告 3 への返答)で既定を OFF にした**: 5 組の測定で wasm の差がゆらぎの範囲(アイドル −7.2 µs〔5 組中 3 組〕、再生 −5.9 µs〔4 組中 3 組〕)で効果を実証できず、窓一覧のページへの到着が描画も入力も無い変化に限り最大 `<ms>` 遅れる=正しさのリスクだけが残るため。`?poll=40`(5〜1000)で opt-in(§4.2 の最初の表〔glueB〕はこの形で測った)。
3. **`performance.timeOrigin` を 1 回だけ読む**(`worker.ts` の `nowAbs()`)。idle ごとに 3〜4 回の `performance.timeOrigin + performance.now()` があり、`timeOrigin` の取得を毎回していた。
4. **ページの DOM は、表示する文字列が前回と同じなら書かない**(`main.ts` の `renderStatus`〔250 ms 毎〕と `renderPhases`〔250 ms 毎〕)。同じ文字列の `textContent` や `replaceChildren` でも、スタイル/レイアウトが無効になり、ページは主フレームを作る。アイドルでエンジンが何も描かない間、これが 8 回/秒の無駄な主フレームだった(読み。§4.2 のページ主スレッドの idle あたりの時間が下がったことと整合するが、因果は他の変更と切り分けていない)。

### 3.3 変えなかったもの
入力の即時注入、report の 40 ms 間引き(clk1 の決定)、`IdleDriver` の駆動、idle ごとの `trace.push`(ゲスト時刻列のハッシュ)、`pollAppView`(`seq` が同じなら窓一覧を読まない: clk1)、`flushLogs`、エンジン側の `fle_idle_lib.js`(idle ごとに `Module['fleIdleDeliver']` と `fleIdleWaiting` のクロージャを作り直し、`new Promise` を作る。`lift/harness/web/fruity/` の変更でエンジンの再リンクが要るので触らなかった。名前のある JS で 0.15 µs/idle と読む)。

## 4. 測定結果

### 4.1 グルーの内訳(HEAD の前端、構成 D、アイドルと再生)と、ui1 の数字の訂正

CDP プロファイラ(200 µs 間隔)で Worker とページ主スレッドを同じ区間(20 s)で測った。構成 D、HEAD の前端(`dist-before`)、load 約 5、1 回ずつ(`runs/prof1`)。

| | アイドル(1,054 idle/s) | 再生(2,033 idle/s) |
|---|---:|---:|
| Worker の idle 1 回あたり | **150.1 µs** | **110.3 µs** |
| ・wasm(ゲスト+ランタイム+JSPI のフレーム) | 129.3(86%) | 89.2(81%) |
| ・`(program)`(JS でも wasm でもない) | 10.3 | 12.3 |
| ・JS の名前のある関数 | 9.8(6.5%) | 8.0(7.3%) |
| ・GC | 0.7 | 0.8 |
| ページ主スレッドの idle 1 回あたり | 20.6 µs(うち `(program)` 18.2) | 37.3 µs(うち `(program)` 32.6) |

**ui1(決定 0166)の『idle 1 回あたり約 0.6 ms(Worker 403 µs + ページ 230 µs)で、ゲストの仕事より大きい』は、clk1(決定 0170)の前の値**である。その時は idle ごとに `postMessage`(1,000 回/秒)をし、窓一覧を毎回読んで `TextDecoder` で 114 窓の文字列を作っていた。clk1 が idle の報告を 40 ms に間引き、窓一覧を `seq` が同じなら読まないようにした後の HEAD では、グルーは Worker の約 7% である。**訂正: 今のグルーは『ゲストの仕事より大きい』ではない。Worker の idle あたりの時間の 81〜86% は wasm(ゲストとランタイム)である。** 残っている JS の自己時間(アイドル、µs/idle): `setTimeout` 2.0、`now` 1.1+0.4、`postMessage` 0.9、`memory.buffer` の getter 0.9、GC 0.7、`readFields` 0.5、`get i32` 0.4、`wasm-to-js` 0.4、`queueMicrotask` 0.3、`snapshot` 0.3。窓の表を読む関数群の合計(`readFields`、`get buffer`、`get i32`、`snapshot`、`entries`、`entryWords`)が JS の中で最大の塊(約 2.5 µs)で、ここが削減の対象になった(§3)。

### 4.2 グルーの前後(各 5 組、交互。構成 D、HEAD の前端 `before` と本差分の前端 `after`)

**2 つの系列がある**: **glueB** は窓一覧の間引き込みの版(`dist-after-poll`。最終のソースで `?poll=40` を付けたのと同じ動作)で測った。その後、PM の指示で**間引きを既定 OFF にした**(効果を実証できなかったため)ので、**最終の既定**(窓の表の stamp、`timeOrigin`、DOM の書き込み抑制。窓一覧は毎 idle 読む)で **glueC** を測り直した。出荷する既定の数字は glueC。

各 run の段階: ウィザードの 25 s、Enter、落ち着くまで、`main-idle` 40 s、`mouse` 8 s、`taps` 8 回、`play` 20 s、そのあとアイドルと再生の CDP プロファイル(Worker とページ主スレッドを同じ 12 s)。1 idle あたりの µs は、スレッド別の CPU 秒(`/proc`、100 Hz の刻み)÷ idle の回数、またはプロファイルの自己時間 ÷ idle の回数。**前後の差は、同じ構成・同じ手順・交互の順序・5 組の中央値〔範囲〕**。

#### glueB(窓一覧の間引き込み = `?poll=40` 相当)

#### glueB: 10 runs (before: 5, after: 5); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| after-1 | ok | 7.37 | 11.7 | 6.9 |
| after-2 | ok | 7.06 | 3.9 | 8.6 |
| after-3 | ok | 6.92 | 6 | 7.7 |
| after-4 | ok | 7.46 | 8.2 | 6.4 |
| after-5 | ok | 7.19 | 5.7 | - |
| before-1 | ok | 7.69 | 8.1 | 8.3 |
| before-2 | ok | 6.97 | 10.8 | 6.5 |
| before-3 | ok | 7.14 | 3.7 | 2.8 |
| before-4 | ok | 6.93 | 3.6 | 8.1 |
| before-5 | ok | 8.92 | 10.2 | 6.4 |

##### phase main-idle: CPU per engine idle callback (microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 155.7 [137.6-162.4] | 144.3 [133.9-158.6] |
| page main thread CPU us / idle (/proc) | 22.8 [21.8-23.7] | 14.2 [13.3-16.1] |
| Worker + page main us / idle | 177.5 [160.4-185.2] | 157.6 [148.1-174.7] |
| engine idle callbacks / s | 1053 [1053-1055] | 1053 [1053-1054] |
| window updates / s | 0.65 [0.62-0.72] | 0.70 [0.62-0.80] |
| virtual / real | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0000] |
| load at the start | 8.1 [3.6-10.8] | 6.0 [3.9-11.7] |

##### phase play: CPU per engine idle callback (microseconds)

| metric | before (n=5) | after (n=4) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 120.1 [111.3-138.6] | 100.4 [99.9-122.4] |
| page main thread CPU us / idle (/proc) | 42.3 [40.6-54.0] | 37.3 [35.4-49.8] |
| Worker + page main us / idle | 161.9 [151.9-192.6] | 137.7 [135.3-172.3] |
| engine idle callbacks / s | 1891 [1890-1895] | 1892 [1887-1893] |
| window updates / s | 53.99 [53.96-54.02] | 54.00 [53.95-54.02] |
| virtual / real | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0001] |
| load at the start | 6.5 [2.8-8.3] | 7.3 [6.4-8.6] |

##### CDP profile, idle (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker busy (all) us / idle | 145.3 [137.8-153.3] | 126.7 [124.9-152.0] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 125.4 [120.1-130.0] | 117.6 [115.8-140.2] |
| Worker JS only (named functions) us / idle | 9.13 [6.96-10.52] | 5.59 [5.16-8.16] |
| Worker (program) us / idle | 10.15 [9.46-15.14] | 3.35 [3.12-4.52] |
| Worker GC us / idle | 0.42 [0.38-0.65] | 0.63 [0.36-1.15] |
| page main busy us / idle | 20.9 [20.3-24.1] | 13.1 [12.2-18.6] |
| page main JS only us / idle | 2.24 [2.19-2.53] | 1.44 [1.17-2.25] |
| page main (program) us / idle | 18.3 [17.5-21.0] | 11.3 [10.6-15.8] |
| idle callbacks / s in the stretch | 1053 [1050-1056] | 1052 [1048-1054] |
| load at the start | 5.9 [4.5-10.6] | 6.2 [5.4-8.8] |

##### CDP profile, play (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=5) | after (n=4) |
|---|---:|---:|
| Worker busy (all) us / idle | 107.8 [103.1-131.5] | 99.2 [93.3-103.9] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 85.3 [80.9-105.5] | 85.3 [81.0-90.3] |
| Worker JS only (named functions) us / idle | 9.60 [7.10-11.09] | 5.97 [5.39-6.79] |
| Worker (program) us / idle | 12.09 [10.20-15.34] | 6.80 [6.40-7.19] |
| Worker GC us / idle | 0.74 [0.71-1.08] | 0.55 [0.48-0.57] |
| page main busy us / idle | 40.2 [37.9-52.7] | 37.3 [35.6-38.2] |
| page main JS only us / idle | 4.54 [4.03-7.57] | 4.28 [3.86-5.02] |
| page main (program) us / idle | 35.3 [33.0-47.2] | 32.3 [31.2-33.3] |
| idle callbacks / s in the stretch | 2022 [2009-2035] | 2025 [2018-2037] |
| load at the start | 5.2 [4.2-9.8] | 5.8 [4.4-7.4] |

##### paired differences (after - before, same repetition; negative = fewer microseconds per engine idle callback)

| metric | rep 1 | rep 2 | rep 3 | rep 4 | rep 5 | median | pairs with after < before |
|---|---:|---:|---:|---:|---:|---:|---:|
| main-idle: Worker CPU us/idle (/proc) | -7.7 | -18.1 | 20.9 | -21.8 | -1.7 | -7.7 | 4/5 |
| main-idle: page main CPU us/idle (/proc) | -9.5 | -9.5 | -6.6 | -7.6 | -8.5 | -8.5 | 5/5 |
| play: Worker CPU us/idle (/proc) | -20.2 | 3.4 | -11.4 | -37.7 | - | -15.8 | 3/4 |
| play: page main CPU us/idle (/proc) | -4.3 | 7.5 | -5.2 | -17.0 | - | -4.7 | 3/4 |
| profile idle: Worker busy us/idle | -26.1 | -16.0 | -0.7 | -1.3 | -18.6 | -16.0 | 5/5 |
| profile idle: Worker JS only us/idle | -5.3 | -2.8 | 1.2 | -2.1 | -4.0 | -2.8 | 4/5 |
| profile idle: Worker (program) us/idle | -6.8 | -6.3 | -5.8 | -10.9 | -6.5 | -6.5 | 5/5 |
| profile idle: Worker wasm us/idle | -14.2 | -7.2 | 3.2 | 11.6 | -7.8 | -7.2 | 3/5 |
| profile idle: Worker non-wasm (JS + program + GC) us/idle | -11.9 | -8.8 | -3.9 | -12.9 | -10.8 | -10.8 | 5/5 |
| profile idle: page main busy us/idle | -7.5 | -8.1 | -6.8 | -5.5 | -11.2 | -7.5 | 5/5 |
| profile play: Worker busy us/idle | -21.6 | -32.0 | -4.3 | -3.9 | - | -13.0 | 4/4 |
| profile play: Worker non-wasm (JS + program + GC) us/idle | -10.3 | -12.6 | -3.9 | -13.3 | - | -11.5 | 4/4 |
| profile play: Worker JS only us/idle | -4.2 | -3.8 | -0.3 | -5.2 | - | -4.0 | 4/4 |
| profile play: page main busy us/idle | -4.6 | -14.5 | 0.3 | -8.9 | - | -6.8 | 3/4 |

##### the guest's time sequence (idle number, virtual clock, next deadline) hashed; equal at every 500th idle up to the first input?

| run | checkpoints compared (n <= first input) | equal | first difference |
|---|---:|---:|---|
| after-1 vs before-1 | 53 | 53 | - |
| after-2 vs before-1 | 53 | 53 | - |
| after-3 vs before-1 | 52 | 52 | - |
| after-4 vs before-1 | 53 | 53 | - |
| after-5 vs before-1 | 53 | 53 | - |
| before-1 vs before-1 | 53 | 53 | - |
| before-2 vs before-1 | 53 | 53 | - |
| before-3 vs before-1 | 53 | 53 | - |
| before-4 vs before-1 | 53 | 53 | - |
| before-5 vs before-1 | 53 | 53 | - |

##### main-window canvas SHA-256 after the welcome wizard (before Enter; the converged picture is `4fa213b0...` in the native baseline)

| run | virtual ms at the read | main window sha (first 12) |
|---|---:|---|
| after-1 | 3627192 | 4fa213b07d79 |
| after-2 | 3627585 | 4fa213b07d79 |
| after-3 | 3626419 | 9b8592d35b26 |
| after-4 | 3627152 | 4fa213b07d79 |
| after-5 | 3627320 | 4fa213b07d79 |
| before-1 | 3626959 | 4fa213b07d79 |
| before-2 | 3627190 | 4fa213b07d79 |
| before-3 | 3627384 | 4fa213b07d79 |
| before-4 | 3627762 | 1da68c630058 |
| before-5 | 3627029 | 4fa213b07d79 |


**読み(5 組、load 3〜12、構成 D。再生の 1 本〔after-5〕は Space で再生が始まらなかったので再生の数字から除いた。表の n)**:

| idle 1 回あたり(µs、中央値) | 前 → 後 | 対の差の中央値(after−before、減った対/対) |
|---|---|---|
| アイドル、Worker の CPU(`/proc`) | 155.7 → 144.3 | -7.7(4/5) |
| アイドル、ページ主スレッドの CPU(`/proc`) | 22.8 → 14.2 | -8.5(5/5) |
| アイドル、Worker + ページ(`/proc`) | 177.5 → 157.6 | -17.2(4/5) |
| アイドル、プロファイルの Worker busy | 145.3 → 126.7 | -16.0(5/5) |
| 　・JS の名前のある関数 | 9.1 → 5.6 | -2.8(4/5) |
| 　・`(program)` | 10.2 → 3.4 | -6.5(5/5) |
| 　・wasm | 125.4 → 117.6 | -7.2(3/5) |
| 　・wasm 以外(JS + program + GC) | 19.9 → 9.1 | -10.8(5/5) |
| アイドル、プロファイルのページ主スレッド busy | 20.9 → 13.1 | -7.5(5/5) |
| 再生、Worker の CPU(`/proc`) | 120.1 → 100.4 | -15.8(3/4) |
| 再生、ページ主スレッドの CPU(`/proc`) | 42.3 → 37.3 | -4.7(3/4) |
| 再生、Worker + ページ(`/proc`) | 161.9 → 137.7 | -20.6(3/4) |
| 再生、プロファイルの Worker busy | 107.8 → 99.2 | -13.0(4/4) |
| 　・JS の名前のある関数 | 9.6 → 6.0 | -4.0(4/4) |
| 　・wasm | 85.3 → 85.3 | -5.9(3/4) |
| 　・wasm 以外(JS + program + GC) | 22.6 → 13.5 | -11.5(4/4) |
| 再生、プロファイルのページ主スレッド busy | 40.2 → 37.2 | -6.8(3/4) |

- **削減は idle あたり Worker+ページで 11〜15%(約 20〜24 µs)で、毎秒 1,053〜1,891 回の idle では 約 0.02〜0.04 コア**。もともとグルーは小さかったので、絶対値は小さい。削減の大半は『窓の表を読まない』ことによる Worker の `(program)`(オブジェクトの確保・`JSON.stringify`・型付き配列の作成)と JS(−3〜−4 µs)、ページの DOM の書き込み抑制によるページ主スレッドの `(program)`(−7 µs)。**wasm の差はゆらぎの範囲(アイドル −7.2〔3/5〕、再生 −5.9〔3/4〕)**で、窓一覧の間引き(`fle_av_poll` を毎 idle 呼ばない)の効果は、この系列では実証できていない。
- 窓の表を 1 回読む費用は node のマイクロベンチ(4 枠生存、V8)で 従来 6.4〜7.1 µs/idle → stamp 0.08〜0.21 µs/idle(`tables/bench-sync.txt`)。Chrome のプロファイルで見えた削減(JS+`(program)` で約 9 µs)と同じ桁。
- 最初の idle(P1)は前 6.93〜8.92 s、後 6.92〜7.46 s で、変わらない。

#### glueC(最終の既定 = 間引き無し。出荷する形)

**読み(最終の既定、5 組すべて有効、load 4〜14、構成 D)**:

| idle 1 回あたり(µs、中央値) | 前 → 後 | 対の差の中央値(after−before、減った対/対) |
|---|---|---|
| アイドル、Worker の CPU(`/proc`) | 155.7 → 140.5 | -9.5(4/5) |
| アイドル、ページ主スレッドの CPU(`/proc`) | 22.8 → 13.3 | -8.5(5/5) |
| アイドル、Worker + ページ(`/proc`) | 179.5 → 154.7 | -17.1(5/5) |
| アイドル、プロファイルの Worker busy | 142.1 → 141.0 | -7.1(3/5) |
| 　・JS の名前のある関数 | 8.9 → 6.3 | -2.6(5/5) |
| 　・`(program)` | 10.0 → 4.6 | -6.0(5/5) |
| 　・wasm | 125.0 → 128.4 | +4.3(2/5) |
| 　・wasm 以外(JS + program + GC) | 19.9 → 11.7 | -8.3(5/5) |
| アイドル、プロファイルのページ主スレッド busy | 23.3 → 13.4 | -9.2(5/5) |
| 再生、Worker の CPU(`/proc`) | 123.5 → 110.6 | -11.2(5/5) |
| 再生、ページ主スレッドの CPU(`/proc`) | 42.0 → 36.6 | -5.4(5/5) |
| 再生、Worker + ページ(`/proc`) | 165.4 → 147.7 | -15.8(5/5) |
| 再生、プロファイルの Worker busy | 112.4 → 100.1 | -11.0(4/5) |
| 　・JS の名前のある関数 | 8.5 → 5.3 | -2.3(5/5) |
| 　・wasm | 88.8 → 86.8 | -2.9(4/5) |
| 　・wasm 以外(JS + program + GC) | 20.9 → 13.2 | -7.6(5/5) |
| 再生、プロファイルのページ主スレッド busy | 39.1 → 34.6 | -3.4(4/5) |

- **出荷する既定では、Worker + ページの idle あたりの CPU は、アイドルで 179.5 → 154.7 µs(−14%。対の差の中央値 −17.1、5/5)、再生で 165.4 → 147.7 µs(−11%。−15.8、5/5)**。毎秒 約 1,050〜1,890 回の idle では約 0.02〜0.03 コア。もともとグルーは小さいので絶対値は小さい。
- 前端のコードが決める部分(Worker の wasm 以外 = JS + `(program)` + GC)は、アイドル 19.9 → 11.7(−8.3、5/5)、再生 20.9 → 13.2(−7.6、5/5)。ページ主スレッドは、アイドル 23.3 → 13.4(プロファイル。−9.2、5/5)、再生 −3.4(4/5)。**wasm の差はゆらぎの範囲**(アイドル +4.3〔2/5〕、再生 −2.9〔4/5〕)。= 削減は前端のコードのぶんだけで、ゲストの wasm の時間は変わっていない(変えない約束どおり)。
- 窓一覧の間引きを含む glueB(上の表)と比べて、最終の既定(間引き無し)でも非 wasm の削減は同じ大きさ(−8.3 対 −10.8、アイドル)で、**間引きの寄与は、あっても数 µs で、ゆらぎに埋もれる**。
- ゲスト時刻列のハッシュは 10 run すべてで before-1 と一致(52/52、after-3 だけ 50/50)。最初の idle(P1)は前 7.00〜8.65 s、後 6.98〜11.65 s(after-3 は load 12 の run で 11.65 s。ばらつき)。

#### glueC: 10 runs (before: 5, after: 5); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| after-1 | ok | 7.34 | 5.3 | 5.2 |
| after-2 | ok | 6.98 | 4.9 | 5 |
| after-3 | ok | 11.65 | 7.1 | 5.9 |
| after-4 | ok | 7.41 | 7 | 12.6 |
| after-5 | ok | 7.56 | 6.9 | 6 |
| before-1 | ok | 7.13 | 5 | 4.5 |
| before-2 | ok | 7.00 | 5 | 4.5 |
| before-3 | ok | 8.65 | 11.3 | 11.1 |
| before-4 | ok | 8.63 | 14.8 | 14.4 |
| before-5 | ok | 7.79 | 8.4 | 7.5 |

##### phase main-idle: CPU per engine idle callback (microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 155.7 [135.6-174.6] | 140.5 [126.3-179.3] |
| page main thread CPU us / idle (/proc) | 22.8 [19.0-26.6] | 13.3 [12.3-16.1] |
| Worker + page main us / idle | 179.5 [154.6-201.2] | 154.7 [138.6-195.4] |
| engine idle callbacks / s | 1054 [1053-1054] | 1053 [1053-1054] |
| window updates / s | 0.60 [0.50-0.73] | 0.73 [0.70-0.80] |
| virtual / real | 1.0000 [0.9999-1.0000] | 1.0000 [1.0000-1.0000] |
| load at the start | 8.4 [5.0-14.8] | 6.9 [4.9-7.1] |

##### phase play: CPU per engine idle callback (microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 123.5 [114.6-126.9] | 110.6 [99.4-123.4] |
| page main thread CPU us / idle (/proc) | 42.0 [39.3-44.0] | 36.6 [32.8-39.2] |
| Worker + page main us / idle | 165.4 [153.9-168.9] | 147.7 [132.2-162.6] |
| engine idle callbacks / s | 1859 [1859-1863] | 1861 [1857-1863] |
| window updates / s | 52.12 [52.08-52.23] | 52.15 [52.11-52.20] |
| virtual / real | 1.0000 [0.9999-1.0001] | 1.0000 [1.0000-1.0001] |
| load at the start | 7.5 [4.5-14.4] | 5.9 [5.0-12.6] |

##### CDP profile, idle (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker busy (all) us / idle | 142.1 [115.9-153.2] | 141.0 [125.6-145.9] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 125.0 [96.0-131.4] | 128.4 [116.3-134.2] |
| Worker JS only (named functions) us / idle | 8.89 [7.95-10.22] | 6.30 [4.91-8.77] |
| Worker (program) us / idle | 9.99 [7.60-13.67] | 4.64 [3.45-5.24] |
| Worker GC us / idle | 0.60 [0.41-1.05] | 0.41 [0.37-0.72] |
| page main busy us / idle | 23.3 [20.6-24.8] | 13.4 [12.2-14.5] |
| page main JS only us / idle | 1.83 [1.44-2.23] | 1.45 [1.41-1.53] |
| page main (program) us / idle | 21.1 [18.4-22.1] | 11.4 [10.3-12.6] |
| idle callbacks / s in the stretch | 1052 [1051-1055] | 1053 [1050-1055] |
| load at the start | 8.3 [5.2-12.8] | 5.6 [5.0-13.2] |

##### CDP profile, play (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=5) | after (n=5) |
|---|---:|---:|
| Worker busy (all) us / idle | 112.4 [104.4-113.6] | 100.1 [92.5-115.6] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 88.8 [85.3-92.4] | 86.8 [81.1-97.0] |
| Worker JS only (named functions) us / idle | 8.49 [7.08-10.55] | 5.28 [4.23-8.65] |
| Worker (program) us / idle | 11.41 [10.20-12.24] | 7.13 [6.36-9.43] |
| Worker GC us / idle | 0.70 [0.64-0.82] | 0.53 [0.45-0.77] |
| page main busy us / idle | 39.1 [34.4-41.1] | 34.6 [31.5-42.2] |
| page main JS only us / idle | 4.63 [4.29-5.07] | 4.27 [3.92-5.34] |
| page main (program) us / idle | 33.8 [29.7-35.8] | 29.9 [27.1-36.2] |
| idle callbacks / s in the stretch | 2027 [2017-2040] | 2022 [2016-2028] |
| load at the start | 9.2 [5.7-11.1] | 5.8 [5.2-13.3] |

##### paired differences (after - before, same repetition; negative = fewer microseconds per engine idle callback)

| metric | rep 1 | rep 2 | rep 3 | rep 4 | rep 5 | median | pairs with after < before |
|---|---:|---:|---:|---:|---:|---:|---:|
| main-idle: Worker CPU us/idle (/proc) | -4.6 | -9.5 | -22.8 | 4.7 | -11.4 | -9.5 | 4/5 |
| main-idle: page main CPU us/idle (/proc) | -5.7 | -7.6 | -8.5 | -10.4 | -10.4 | -8.5 | 5/5 |
| play: Worker CPU us/idle (/proc) | -11.2 | -27.5 | -10.4 | -0.1 | -12.2 | -11.2 | 5/5 |
| play: page main CPU us/idle (/proc) | -4.3 | -9.2 | -5.4 | -2.7 | -7.5 | -5.4 | 5/5 |
| profile idle: Worker busy us/idle | -13.8 | 9.7 | 18.6 | -7.1 | -10.3 | -7.1 | 3/5 |
| profile idle: Worker JS only us/idle | -3.0 | -3.8 | -1.4 | -2.6 | -1.5 | -2.6 | 5/5 |
| profile idle: Worker (program) us/idle | -5.2 | -6.5 | -2.4 | -9.0 | -6.0 | -6.0 | 5/5 |
| profile idle: Worker wasm us/idle | -5.5 | 20.3 | 22.6 | 4.3 | -2.6 | 4.3 | 2/5 |
| profile idle: Worker non-wasm (JS + program + GC) us/idle | -8.3 | -10.6 | -4.0 | -11.4 | -7.7 | -8.3 | 5/5 |
| profile idle: page main busy us/idle | -7.4 | -11.1 | -7.4 | -9.2 | -11.2 | -9.2 | 5/5 |
| profile play: Worker busy us/idle | -8.4 | -11.9 | -11.0 | 2.0 | -12.3 | -11.0 | 4/5 |
| profile play: Worker non-wasm (JS + program + GC) us/idle | -5.5 | -7.7 | -10.4 | -2.6 | -7.6 | -7.6 | 5/5 |
| profile play: Worker JS only us/idle | -2.2 | -3.0 | -5.3 | -0.4 | -2.3 | -2.3 | 5/5 |
| profile play: page main busy us/idle | -0.9 | -3.4 | -4.1 | 1.1 | -4.6 | -3.4 | 4/5 |

##### the guest's time sequence (idle number, virtual clock, next deadline) hashed; equal at every 500th idle up to the first input?

| run | checkpoints compared (n <= first input) | equal | first difference |
|---|---:|---:|---|
| after-1 vs before-1 | 52 | 52 | - |
| after-2 vs before-1 | 52 | 52 | - |
| after-3 vs before-1 | 50 | 50 | - |
| after-4 vs before-1 | 52 | 52 | - |
| after-5 vs before-1 | 52 | 52 | - |
| before-1 vs before-1 | 52 | 52 | - |
| before-2 vs before-1 | 52 | 52 | - |
| before-3 vs before-1 | 52 | 52 | - |
| before-4 vs before-1 | 52 | 52 | - |
| before-5 vs before-1 | 52 | 52 | - |

##### main-window canvas SHA-256 after the welcome wizard (before Enter; the converged picture is `4fa213b0...` in the native baseline)

| run | virtual ms at the read | main window sha (first 12) |
|---|---:|---|
| after-1 | 3626865 | 4fa213b07d79 |
| after-2 | 3626626 | 9b8592d35b26 |
| after-3 | 3625675 | 1da68c630058 |
| after-4 | 3626672 | 9b8592d35b26 |
| after-5 | 3626876 | 4fa213b07d79 |
| before-1 | 3626852 | 4fa213b07d79 |
| before-2 | 3627561 | 4fa213b07d79 |
| before-3 | 3626492 | 9b8592d35b26 |
| before-4 | 3626427 | 9b8592d35b26 |
| before-5 | 3627431 | 4fa213b07d79 |


**同じビルド(`dist-final2`、間引きが既定 ON だった時点のもの)の `?stamp=0` 対 既定**(その時点では `?stamp=0` が窓の表の stamp と窓一覧の間引きを一緒に切った。最終のソースでは `?stamp=0` は表の stamp だけを切る。構成 D、1 組、プロファイルだけ):

#### abpoll: the SAME build (dist-final2) with ?stamp=0 (no table stamp, window list read at every idle = the former behaviour) and without (default); bundle D, one run each, load 8-12; CDP profile, microseconds per engine idle callback

| run | stretch | idle/s | Worker busy | wasm | JS only | (program) | GC | page main busy |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| stamp0-1 | idle | 1052.6 | 153.5 | 129.8 | 11.62 | 11.09 | 1.07 | 15 |
| stamp0-1 | play | 2021.6 | 119.2 | 95 | 10.96 | 12.62 | 0.61 | 41.4 |
| stamp1-1 | idle | 1051.4 | 136.8 | 124.9 | 7.43 | 3.75 | 0.67 | 13.7 |
| stamp1-1 | play | 2026.7 | 104.6 | 88.4 | 7.91 | 7.75 | 0.54 | 41.1 |


**間引き(窓一覧の読み取り)を足す前の中間版の 1 組**(`dist-after-B1` = 表の stamp + `timeOrigin` + DOM の書き込み抑制。窓一覧の間引き無し。最初のキューの 1 組目、再走の前に止めた):

#### the first glueB pair, before the window-list poll gating existed: HEAD front end (before-1) and the intermediate build dist-after-B1 (table stamp + timeOrigin + DOM guards; after-1), bundle D, load 7-17, one run each; CDP profile, microseconds per engine idle callback

| run | stretch | idle/s | Worker busy | wasm | JS only | (program) | GC | page main busy |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| after-1 | idle | 1053.5 | 154.1 | 138.3 | 9.1 | 6.49 | 0.22 | 15.8 |
| after-1 | play | 2021.8 | 114 | 97 | 7.78 | 8.53 | 0.7 | 41.4 |
| before-1 | idle | 1049.7 | 168 | 136.1 | 19.88 | 11.29 | 0.74 | 28.3 |
| before-1 | play | 2024.2 | 142.9 | 109.5 | 14.7 | 17.63 | 1.08 | 48.6 |


### 4.3 前後で変わっていないこと

**ゲストが経験する時刻の列**: エンジンの idle ごとに (idle 番号, 仮想時計, 次の期限) を FNV-1a の列ハッシュへ畳み込み、500 回目ごとに記録している(clk1、`idle-trace.ts`)。ゲストの仮想時計はホストの駆動の仕方に依らない(いつ期限を渡すかだけが実時間に依る)ので、入力を入れる前(Enter まで)の列は、前端が違っても一致するはず。上の glueB の表の末尾が、`before-1` と各 run の n ≤ 最初の入力までの checkpoint の一致数。120 Hz でも同じことを確かめた(HEAD の前端に `?env=FLE_DWM_HZ=120&env=FLE_DISPLAYSETTINGS_FREQUENCY=120` を手で付けた run と、本差分の前端に `?hz=120` を付けた run: `?hz=` が渡す環境が手で付けたものと同じであることの確認も兼ねる)。

#### hz120hash: 2 runs (before: 1, after: 1); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| after-1 | ok | 7.31 | 6.6 | 6.7 |
| before-1 | ok | 7.67 | 5.3 | 5.3 |

##### phase main-idle: CPU per engine idle callback (microseconds)

| metric | before (n=1) | after (n=1) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 138.0 [138.0-138.0] | 170.9 [170.9-170.9] |
| page main thread CPU us / idle (/proc) | 23.3 [23.3-23.3] | 17.9 [17.9-17.9] |
| Worker + page main us / idle | 161.3 [161.3-161.3] | 188.8 [188.8-188.8] |
| engine idle callbacks / s | 1116 [1116-1116] | 1118 [1118-1118] |
| window updates / s | 0.60 [0.60-0.60] | 0.80 [0.80-0.80] |
| virtual / real | 1.0000 [1.0000-1.0000] | 0.9999 [0.9999-0.9999] |
| load at the start | 5.3 [5.3-5.3] | 6.6 [6.6-6.6] |

##### phase play: CPU per engine idle callback (microseconds)

| metric | before (n=1) | after (n=1) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 137.2 [137.2-137.2] | 137.9 [137.9-137.9] |
| page main thread CPU us / idle (/proc) | 43.5 [43.5-43.5] | 43.7 [43.7-43.7] |
| Worker + page main us / idle | 180.7 [180.7-180.7] | 181.6 [181.6-181.6] |
| engine idle callbacks / s | 1771 [1771-1771] | 1762 [1762-1762] |
| window updates / s | 87.57 [87.57-87.57] | 87.67 [87.67-87.67] |
| virtual / real | 1.0000 [1.0000-1.0000] | 1.0001 [1.0001-1.0001] |
| load at the start | 5.3 [5.3-5.3] | 6.7 [6.7-6.7] |

##### CDP profile, idle (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=0) | after (n=0) |
|---|---:|---:|
| Worker busy (all) us / idle | - | - |
| Worker wasm (guest + runtime + JSPI frames) us / idle | - | - |
| Worker JS only (named functions) us / idle | - | - |
| Worker (program) us / idle | - | - |
| Worker GC us / idle | - | - |
| page main busy us / idle | - | - |
| page main JS only us / idle | - | - |
| page main (program) us / idle | - | - |
| idle callbacks / s in the stretch | - | - |
| load at the start | - | - |

##### CDP profile, play (200 us sampling; self time per engine idle callback, microseconds)

| metric | before (n=0) | after (n=0) |
|---|---:|---:|
| Worker busy (all) us / idle | - | - |
| Worker wasm (guest + runtime + JSPI frames) us / idle | - | - |
| Worker JS only (named functions) us / idle | - | - |
| Worker (program) us / idle | - | - |
| Worker GC us / idle | - | - |
| page main busy us / idle | - | - |
| page main JS only us / idle | - | - |
| page main (program) us / idle | - | - |
| idle callbacks / s in the stretch | - | - |
| load at the start | - | - |

##### paired differences (after - before, same repetition; negative = fewer microseconds per engine idle callback)

| metric | rep 1 | median | pairs with after < before |
|---|---:|---:|---:|
| main-idle: Worker CPU us/idle (/proc) | 32.9 | 32.9 | 0/1 |
| main-idle: page main CPU us/idle (/proc) | -5.4 | -5.4 | 1/1 |
| play: Worker CPU us/idle (/proc) | 0.7 | 0.7 | 0/1 |
| play: page main CPU us/idle (/proc) | 0.2 | 0.2 | 0/1 |
| profile idle: Worker busy us/idle | - | - | 0/0 |
| profile idle: Worker JS only us/idle | - | - | 0/0 |
| profile idle: Worker (program) us/idle | - | - | 0/0 |
| profile idle: Worker wasm us/idle | - | - | 0/0 |
| profile idle: Worker non-wasm (JS + program + GC) us/idle | - | - | 0/0 |
| profile idle: page main busy us/idle | - | - | 0/0 |
| profile play: Worker busy us/idle | - | - | 0/0 |
| profile play: Worker non-wasm (JS + program + GC) us/idle | - | - | 0/0 |
| profile play: Worker JS only us/idle | - | - | 0/0 |
| profile play: page main busy us/idle | - | - | 0/0 |

##### the guest's time sequence (idle number, virtual clock, next deadline) hashed; equal at every 500th idle up to the first input?

| run | checkpoints compared (n <= first input) | equal | first difference |
|---|---:|---:|---|
| after-1 vs before-1 | 47 | 47 | - |
| before-1 vs before-1 | 47 | 47 | - |

##### main-window canvas SHA-256 after the welcome wizard (before Enter; the converged picture is `4fa213b0...` in the native baseline)

| run | virtual ms at the read | main window sha (first 12) |
|---|---:|---|
| after-1 | 3623308 | 1438f116a7cc |
| before-1 | 3623246 | 1438f116a7cc |


**主窓の面と窓一覧**: 同じ仮想時刻で主窓の canvas の SHA-256 を読み(`p2a-sha.mjs`、clk1 の `sha-at-virtual.mjs` の写し)、決定 0147 が記録した native の基準と比べた。HEAD の前端 / 本差分 / 本差分に `?stamp=0` を付けた(表の stamp が無効)の 3 通り。a3000・a4000 は native の基準 `4fa213b07d79` と一致するかを見る。Enter の後の主窓は**静止せず**、数秒周期で数枚の絵を巡る(`sha`・`sha2` の e3300〜e12000 の値は、どの前端でも `dd416725c74a`・`8561cf14b921`・`b5f5242759a8`・`5a6ea07ecdc8` のどれか)。読み取りは約 100 ms の粒度でしか仮想時刻に合わせられないので、**『同じ仮想時刻の SHA が一致』は Enter の前(a3000・a4000)でしか言えない**: そこでは HEAD の前端 / 本差分 / `?stamp=0` の 6 本すべてが native の基準 `4fa213b07d79` と一致した。Enter の後は、(1) 3 つの前端が同じ絵の集合の中の値を返すこと、(2) **フィルムストリップ**(`sha4`: Enter の後 12 s、主窓の canvas を続けて読み、絵の現れる順番と時刻を比べる)を見た。**(2) は『同じ』と言えなかった**: 最初の 1 組(`before`・`after`)は絵の順序が違い、対照として同じ前端をもう 2 本ずつ(`before2`・`before3`・`after2`・`after3`、計 3 本ずつ)走らせると、**同じ前端どうし(before ↔ before2 など)でも順序も集合も揃わなかった**(共通の絵は 16〜20 枚 / 19〜23 枚)。読み取りは約 26 回/s で、描かれる絵のうち一部しか見えないためと読む(未確認)。3 本ずつの和集合では、前 26 枚・後 25 枚、共通 25 枚で、**後にしか現れない絵は 0 枚**、前にしか現れない絵が 1 枚(`8be1aa0a83fb`、`before` の 1 本に 579 ms で 1 回見えただけ)。= **この方法の分解能では、Enter の後の絵が前後で同じとも違うとも言えない**(区別できなかった)。Enter の後を確かめるには、全フレームを記録する方法(描画のたびに SHA を出す)が要る。

##### (sha) the main window's canvas at the same virtual times (bundle D; native baselines of decision 0147: a3000/a4000 4fa213b07d79, e3300 dd416725c74a)

| sample | after | after-nostamp | before | native baseline |
|---|---|---|---|---|
| a3000-4000 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 |
| a4000 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 |
| e3300 | dd416725c74a | 8561cf14b921 | 8be1aa0a83fb | dd416725c74a |
| e5000 | dd416725c74a | 5a6ea07ecdc8 | dd416725c74a | (none) |

The engine's window list as the page held it at the same samples (digest of the cursor and every toplevel window's flags, place, size, class and title : number of windows):

| sample | after | after-nostamp | before |
|---|---|---|---|
| a3000-4000 | 155a858f:27 | 155a858f:27 | 155a858f:27 |
| a4000 | 155a858f:27 | 155a858f:27 | 155a858f:27 |
| e3300 | 3db97090:27 | 3db97090:27 | 3db97090:27 |
| e5000 | 3db97090:27 | 3db97090:27 | 3db97090:27 |

errors: after: 0, after-nostamp: 0, before: 0

##### (sha2) the main window's canvas at the same virtual times (bundle D; native baselines of decision 0147: a3000/a4000 4fa213b07d79, e3300 dd416725c74a)

| sample | after | after-nostamp | after-nostamp2 | after2 | before | before2 | native baseline |
|---|---|---|---|---|---|---|---|
| a3000-4000 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 |
| a4000 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 | 4fa213b07d79 |
| e3300 | dd416725c74a | 8561cf14b921 | 8561cf14b921 | 8561cf14b921 | 5a6ea07ecdc8 | dd416725c74a | dd416725c74a |
| e5000 | dd416725c74a | b5f5242759a8 | dd416725c74a | dd416725c74a | 8561cf14b921 | dd416725c74a | (none) |
| e7000 | dd416725c74a | dd416725c74a | 8561cf14b921 | 8561cf14b921 | b5f5242759a8 | b5f5242759a8 | (none) |
| e9000 | dd416725c74a | dd416725c74a | b5f5242759a8 | b5f5242759a8 | dd416725c74a | dd416725c74a | (none) |
| e12000 | b5f5242759a8 | dd416725c74a | dd416725c74a | dd416725c74a | dd416725c74a | dd416725c74a | (none) |

The engine's window list as the page held it at the same samples (digest of the cursor and every toplevel window's flags, place, size, class and title : number of windows):

| sample | after | after-nostamp | after-nostamp2 | after2 | before | before2 |
|---|---|---|---|---|---|---|
| a3000-4000 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 |
| a4000 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 | 155a858f:27 |
| e3300 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 |
| e5000 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 |
| e7000 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 |
| e9000 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 |
| e12000 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 | 3db97090:27 |

errors: after: 0, after-nostamp: 0, after-nostamp2: 0, after2: 0, before: 0, before2: 0

##### film strips: the main window read back again and again for 12 s after Enter (bundle D); the pictures in the order they appeared, with the virtual ms after Enter at which each was first read

| run | reads | pictures in order (ms after Enter : first 12 hex digits) |
|---|---:|---|
| after | 310 | 40:4fa213b07d79 → 45:fca9aec99fdf → 50:027386cef77c → 65:c815dd74c6b3 → 81:5ef8a2bd7ce4 → 97:62d4433bdf77 → 129:b319bdda3903 → 145:711b9f5ca634 → 177:535495af08d6 → 193:d5a3ab05b0dd → 209:0aeca3321eab → 233:46f3678a8292 → 233:f86f0b2bb433 → 257:8079cebd015f → 273:0137da5776bf → 291:8d523f112e39 → 321:c3bec9b4a235 → 353:dd416725c74a → 753:19e127807847 → 1761:dd416725c74a → 2706:8561cf14b921 → 3634:5a6ea07ecdc8 → 4607:dd416725c74a → 8450:b5f5242759a8 → 10386:dd416725c74a → 12297:8561cf14b921 |
| after2 | 481 | 0:4fa213b07d79 → 33:fca9aec99fdf → 36:027386cef77c → 36:c815dd74c6b3 → 68:5ef8a2bd7ce4 → 68:e8f60aad3268 → 100:62d4433bdf77 → 100:b319bdda3903 → 132:b3d47954877d → 132:711b9f5ca634 → 164:535495af08d6 → 164:d5a3ab05b0dd → 196:0aeca3321eab → 196:46f3678a8292 → 228:f86f0b2bb433 → 228:0137da5776bf → 260:8d523f112e39 → 292:c3bec9b4a235 → 324:dd416725c74a → 675:ac10a09b27ee → 1724:8561cf14b921 → 3614:19e127807847 → 4623:dd416725c74a → 7490:8561cf14b921 → 8449:19e127807847 → 9407:dd416725c74a |
| after3 | 514 | 0:4fa213b07d79 → 28:fca9aec99fdf → 31:027386cef77c → 31:c815dd74c6b3 → 62:5ef8a2bd7ce4 → 92:e8f60aad3268 → 92:62d4433bdf77 → 110:b319bdda3903 → 110:b3d47954877d → 142:711b9f5ca634 → 142:535495af08d6 → 174:d5a3ab05b0dd → 174:0aeca3321eab → 206:46f3678a8292 → 206:f86f0b2bb433 → 238:8079cebd015f → 238:0137da5776bf → 270:8d523f112e39 → 302:c3bec9b4a235 → 334:dd416725c74a → 558:5a6ea07ecdc8 → 1710:dd416725c74a → 2677:8561cf14b921 → 3636:19e127807847 → 4600:dd416725c74a → 6516:8561cf14b921 → 7441:dd416725c74a → 8443:b5f5242759a8 → 9358:dd416725c74a → 12236:5a6ea07ecdc8 |
| before | 306 | 0:4fa213b07d79 → 52:027386cef77c → 52:c815dd74c6b3 → 83:e8f60aad3268 → 115:62d4433bdf77 → 115:b319bdda3903 → 147:711b9f5ca634 → 179:535495af08d6 → 179:d5a3ab05b0dd → 204:46f3678a8292 → 227:f86f0b2bb433 → 259:0137da5776bf → 291:8d523f112e39 → 323:c3bec9b4a235 → 355:dd416725c74a → 579:8be1aa0a83fb → 1603:b5f5242759a8 → 3571:dd416725c74a → 10313:8561cf14b921 → 11279:5a6ea07ecdc8 |
| before2 | 485 | 0:4fa213b07d79 → 29:fca9aec99fdf → 29:027386cef77c → 43:c815dd74c6b3 → 74:5ef8a2bd7ce4 → 74:e8f60aad3268 → 106:62d4433bdf77 → 106:b319bdda3903 → 138:b3d47954877d → 138:711b9f5ca634 → 170:535495af08d6 → 170:d5a3ab05b0dd → 202:0aeca3321eab → 202:46f3678a8292 → 234:8079cebd015f → 234:0137da5776bf → 266:8d523f112e39 → 298:c3bec9b4a235 → 331:dd416725c74a → 650:b5f5242759a8 → 1717:dd416725c74a → 2687:8561cf14b921 → 3658:5a6ea07ecdc8 → 5558:dd416725c74a → 7475:8561cf14b921 → 8407:b5f5242759a8 → 9370:19e127807847 → 10377:dd416725c74a → 12283:8561cf14b921 |
| before3 | 301 | 0:4fa213b07d79 → 11:fca9aec99fdf → 19:027386cef77c → 36:c815dd74c6b3 → 53:e8f60aad3268 → 83:62d4433bdf77 → 99:b319bdda3903 → 99:b3d47954877d → 132:711b9f5ca634 → 147:535495af08d6 → 163:d5a3ab05b0dd → 179:0aeca3321eab → 195:f86f0b2bb433 → 227:8079cebd015f → 243:0137da5776bf → 259:8d523f112e39 → 291:c3bec9b4a235 → 323:dd416725c74a → 739:ac10a09b27ee → 1587:dd416725c74a → 4554:b5f5242759a8 → 5568:dd416725c74a → 6484:8561cf14b921 → 7444:dd416725c74a → 9349:19e127807847 → 10354:b5f5242759a8 → 11276:dd416725c74a → 12245:8561cf14b921 |

| run | same pictures in the same order as `before` | largest difference of a picture's first-seen time to `before`, ms |
|---|---|---:|
| after | NO | - |
| after2 | NO | - |
| after3 | NO | - |
| before | yes | 0 |
| before2 | NO | - |
| before3 | NO | - |

The reads are about 26 per second, not one per frame, so a run sees only some of the pictures that were drawn (the pictures of the transition show up in one run and not in another). The overlap of the picture sets (pictures seen in both runs / pictures seen in the first run / in the second run); a pair of runs of the SAME front end is the noise of the method:

| pair | common | first | second |
|---|---:|---:|---:|
| after ↔ after2 | 19 | 22 | 22 |
| after ↔ after3 | 22 | 22 | 24 |
| after ↔ before | 17 | 22 | 19 |
| after ↔ before2 | 21 | 22 | 23 |
| after ↔ before3 | 19 | 22 | 22 |
| after2 ↔ after3 | 21 | 22 | 24 |
| after2 ↔ before | 16 | 22 | 19 |
| after2 ↔ before2 | 20 | 22 | 23 |
| after2 ↔ before3 | 20 | 22 | 22 |
| after3 ↔ before | 18 | 24 | 19 |
| after3 ↔ before2 | 23 | 24 | 23 |
| after3 ↔ before3 | 21 | 24 | 22 |
| before ↔ before2 | 17 | 19 | 23 |
| before ↔ before3 | 16 | 19 | 22 |
| before2 ↔ before3 | 20 | 23 | 22 |

Union over the runs of one front end: before (before, before2, before3) 26 pictures, after (after, after2, after3) 25 pictures, in both 25; only in after: (none); only in before: 8be1aa0a83fb.


(Dp の `?hz=60` と `?hz=120` の主窓を同じ仮想時刻で比べる run〔sha3〕は、時間が足りず実行していない。60 と 120 でゲストの描く絵が同じかは未確認。)

### 4.4 60 Hz と 120 Hz(構成 Dp、空のプロジェクト、交互 3 組)

60 Hz と 120 Hz は同じ Dp のバンドルに `?hz=60` と `?hz=120` を付けただけの違い。各 run は同じ手順(§1)。**load が 8 を超えた run がある**(表の load 欄と下の『run ごと』): 60 Hz の 2 本目・3 本目は load 14〜20、120 Hz は 7〜12。壁時計の値は参考で、CPU 秒(スレッド別)と回数・比を主にする。

#### hzA: 6 runs (60: 3, 120: 3); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| 120-1 | ok | 7.81 | 6.2 | 8.9 |
| 120-2 | ok | 7.82 | 6.9 | 6.9 |
| 120-3 | ok | 8.74 | 12.3 | 9.9 |
| 60-1 | ok | 6.89 | 5.5 | 5.4 |
| 60-2 | ok | 9.00 | 20.5 | 13.9 |
| 60-3 | ok | 9.32 | 17.6 | 13.9 |

##### phase main-idle

| metric | 60 Hz (n=3) | 120 Hz (n=3) |
|---|---:|---:|
| (a) window updates / real s | 0.67 [0.67-0.72] | 0.78 [0.73-0.80] |
| engine idle callbacks / real s | 1054 [1053-1054] | 1111 [1111-1112] |
| (b) Worker CPU, cores (CPU s / real s) | 0.116 [0.108-0.120] | 0.163 [0.163-0.173] |
| (b) Worker CPU s / virtual s | 0.116 [0.108-0.120] | 0.163 [0.163-0.173] |
| (c) page main thread, cores | 0.015 [0.015-0.016] | 0.014 [0.014-0.015] |
| all Chrome threads, cores | 0.155 [0.149-0.161] | 0.203 [0.201-0.212] |
| (e) virtual time / real time | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0000] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 1029.0 [1029.0-1029.0] | 1029.3 [1029.3-1029.3] |
| load at the start of the phase | 17.6 [5.5-20.5] | 6.9 [6.2-12.3] |

##### phase mouse

| metric | 60 Hz (n=3) | 120 Hz (n=3) |
|---|---:|---:|
| (a) window updates / real s | 7.44 [7.29-7.65] | 9.58 [9.43-10.07] |
| engine idle callbacks / real s | 1057 [1056-1060] | 1115 [1113-1115] |
| (b) Worker CPU, cores (CPU s / real s) | 0.167 [0.158-0.169] | 0.218 [0.207-0.235] |
| (b) Worker CPU s / virtual s | 0.167 [0.158-0.169] | 0.218 [0.207-0.235] |
| (c) page main thread, cores | 0.141 [0.138-0.143] | 0.142 [0.133-0.143] |
| all Chrome threads, cores | 0.490 [0.488-0.505] | 0.548 [0.529-0.548] |
| (e) virtual time / real time | 1.0000 [1.0000-1.0002] | 1.0000 [0.9999-1.0001] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 1029.0 [1029.0-1029.0] | 1029.3 [1029.3-1029.3] |
| (d) input -> next update p50 ms | 202.3 [201.8-217.4] | 265.0 [193.0-268.3] |
| (d) input -> next update p95 ms | 903.3 [802.6-1202.6] | 2123.2 [1352.8-2204.1] |
| (d) page -> injected into the engine p50 ms | 0.19 [0.18-0.21] | 0.17 [0.14-0.26] |
| (d) engine processing (inject -> idle) p50 ms | 1.27 [1.18-1.31] | 1.16 [0.98-1.33] |
| (d) engine processing p95 ms | 2.6 [2.4-2.9] | 2.4 [2.2-4.3] |
| (d) idle -> first frame on the page p50 ms | 201.08 [200.15-215.57] | 262.79 [191.15-267.11] |
| (d) inputs counted (n) | 398 [397-400] | 394 [389-394] |
| load at the start of the phase | 15.4 [4.9-16.6] | 7.7 [6.4-9.8] |

##### phase taps

| metric | 60 Hz (n=3) | 120 Hz (n=3) |
|---|---:|---:|
| (a) window updates / real s | 7.88 [7.74-7.94] | 13.70 [13.64-13.71] |
| engine idle callbacks / real s | 1053 [1053-1055] | 1112 [1112-1112] |
| (b) Worker CPU, cores (CPU s / real s) | 0.139 [0.136-0.167] | 0.198 [0.189-0.217] |
| (b) Worker CPU s / virtual s | 0.139 [0.136-0.167] | 0.198 [0.189-0.218] |
| (c) page main thread, cores | 0.040 [0.033-0.040] | 0.033 [0.032-0.039] |
| all Chrome threads, cores | 0.231 [0.217-0.267] | 0.278 [0.271-0.312] |
| (e) virtual time / real time | 1.0000 [0.9998-1.0000] | 1.0000 [0.9999-1.0000] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 1029.0 [1029.0-1029.0] | 1029.3 [1029.3-1029.3] |
| (d) input -> next update p50 ms | 3.9 [3.5-6.3] | 5.1 [3.4-7.7] |
| (d) input -> next update p95 ms | 436.1 [106.6-551.1] | 118.5 [20.9-136.9] |
| (d) page -> injected into the engine p50 ms | 0.18 [0.15-0.19] | 0.14 [0.14-0.16] |
| (d) engine processing (inject -> idle) p50 ms | 2.25 [2.13-3.46] | 2.28 [2.24-2.66] |
| (d) engine processing p95 ms | 19.6 [18.7-24.9] | 18.1 [17.3-24.9] |
| (d) idle -> first frame on the page p50 ms | 0.18 [0.17-0.18] | 0.25 [0.14-0.33] |
| (d) inputs counted (n) | 30 [30-30] | 29 [29-30] |
| load at the start of the phase | 14.9 [5.2-15.7] | 8.2 [7.1-11.6] |

##### phase clicks

| metric | 60 Hz (n=3) | 120 Hz (n=3) |
|---|---:|---:|
| (a) window updates / real s | 43.40 [43.16-43.43] | 84.69 [84.39-84.72] |
| engine idle callbacks / real s | 1052 [1050-1053] | 1107 [1105-1107] |
| (b) Worker CPU, cores (CPU s / real s) | 0.234 [0.224-0.345] | 0.359 [0.351-0.369] |
| (b) Worker CPU s / virtual s | 0.233 [0.224-0.345] | 0.359 [0.352-0.369] |
| (c) page main thread, cores | 0.093 [0.084-0.105] | 0.085 [0.081-0.093] |
| all Chrome threads, cores | 0.407 [0.405-0.572] | 0.531 [0.530-0.549] |
| (e) virtual time / real time | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0001] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 1029.0 [1029.0-1029.0] | 1029.3 [1029.3-1029.3] |
| (d) input -> next update p50 ms | 10.0 [8.7-15.3] | 7.4 [7.1-10.4] |
| (d) input -> next update p95 ms | 320.8 [233.2-340.8] | 307.7 [223.2-477.2] |
| (d) page -> injected into the engine p50 ms | 0.15 [0.13-0.22] | 0.15 [0.13-0.15] |
| (d) engine processing (inject -> idle) p50 ms | 1.72 [1.68-2.50] | 1.56 [1.53-1.61] |
| (d) engine processing p95 ms | 35.9 [33.8-58.9] | 34.4 [33.7-35.4] |
| (d) idle -> first frame on the page p50 ms | 0.43 [0.21-4.98] | 0.25 [0.19-0.34] |
| (d) inputs counted (n) | 59 [59-60] | 60 [59-60] |
| load at the start of the phase | 14.3 [5.5-14.9] | 9.6 [6.8-9.9] |

##### phase play

| metric | 60 Hz (n=3) | 120 Hz (n=3) |
|---|---:|---:|
| (a) window updates / real s | 59.00 [58.98-59.00] | 117.91 [117.90-117.92] |
| engine idle callbacks / real s | 1972 [1970-1974] | 1985 [1984-1988] |
| (b) Worker CPU, cores (CPU s / real s) | 0.182 [0.165-0.200] | 0.263 [0.255-0.305] |
| (b) Worker CPU s / virtual s | 0.182 [0.165-0.200] | 0.263 [0.255-0.305] |
| (c) page main thread, cores | 0.105 [0.086-0.110] | 0.106 [0.090-0.110] |
| all Chrome threads, cores | 0.349 [0.345-0.389] | 0.439 [0.430-0.492] |
| (e) virtual time / real time | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0000] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 1029.0 [1029.0-1029.0] | 1223.2 [1223.2-1223.2] |
| (d) input -> next update p50 ms | 16.1 [15.4-20.2] | 16.0 [14.4-21.6] |
| (d) input -> next update p95 ms | 560.4 [297.9-955.6] | 371.3 [251.9-1542.7] |
| (d) page -> injected into the engine p50 ms | 0.20 [0.19-0.29] | 0.19 [0.16-0.20] |
| (d) engine processing (inject -> idle) p50 ms | 1.94 [1.39-2.03] | 0.99 [0.96-1.13] |
| (d) engine processing p95 ms | 15.6 [15.4-20.0] | 15.3 [13.9-21.1] |
| (d) idle -> first frame on the page p50 ms | 1.82 [0.22-12.78] | 3.80 [1.92-8.40] |
| (d) inputs counted (n) | 5 [4-5] | 5 [5-5] |
| load at the start of the phase | 13.9 [5.4-13.9] | 8.9 [6.9-9.9] |

##### per run (play)

| run | updates/s | idle/s | Worker core | page main core | virtual/real | wasm MB | load |
|---|---:|---:|---:|---:|---:|---:|---:|
| 120-1 | 117.90 | 1988 | 0.255 | 0.106 | 1.0000 | 1223.2 | 8.9->6.7 |
| 120-2 | 117.91 | 1985 | 0.305 | 0.110 | 1.0000 | 1223.2 | 6.9->13.3 |
| 120-3 | 117.92 | 1984 | 0.263 | 0.090 | 1.0000 | 1223.2 | 9.9->8.8 |
| 60-1 | 58.98 | 1974 | 0.165 | 0.105 | 1.0000 | 1029.0 | 5.4->5.4 |
| 60-2 | 59.00 | 1972 | 0.200 | 0.110 | 1.0000 | 1029.0 | 13.9->19.6 |
| 60-3 | 59.00 | 1970 | 0.182 | 0.086 | 1.0000 | 1029.0 | 13.9->13.4 |


**最初の idle(P1)を ?hz= 別に**(hzA・profA・soak・defE の全 run。load は run ごとに違う):

#### first idle (s after the navigation started) by ?hz= (the same machine, load differs run to run; P1 = 10 s)

| bundle | series | ?hz= | first idle s | load at the start of the idle phase |
|---|---|---:|---:|---:|
| Dp | hzA | 60 | 6.89 | 5.5 |
| Dp | hzA | 60 | 9.00 | 20.5 |
| Dp | hzA | 60 | 9.32 | 17.6 |
| Dp | profA | 60 | 7.88 | 8.8 |
| Dp | profA | 60 | 7.37 | 10.2 |
| Dp | soak | 60 | 8.14 | 12.1 |
| Dp | hzA | 120 | 7.82 | 6.9 |
| Dp | hzA | 120 | 8.74 | 12.3 |
| Dp | hzA | 120 | 7.81 | 6.2 |
| Dp | profA | 120 | 8.43 | 11.7 |
| Dp | profA | 120 | 10.85 | 23.6 |
| Dp | soak | 120 | 8.36 | 11.2 |
| F1xn (default entrance) | defE | 60 | 6.72 | 11.3 |
| F1xn (default entrance) | defE | 120 | 7.37 | 12.1 |

- Dp, 60 Hz: median 8.01 s [6.89-9.32] over 6 runs (median load 11.1)
- Dp, 120 Hz: median 8.40 s [7.81-10.85] over 6 runs (median load 11.4)
- F1xn (default entrance), 60 Hz: median 6.72 s [6.72-6.72] over 1 runs (median load 11.3)
- F1xn (default entrance), 120 Hz: median 7.37 s [7.37-7.37] over 1 runs (median load 12.1)


### 4.5 `?hz=` は構成 D では効かない / 今の既定の入口(F1xn)では効く

既定の入口の構成 D(`fl-engine-b1-D`)に `?hz=120` を付けた run(下)。ページは『`FLE_DWM_HZ=120` と `FLE_DISPLAYSETTINGS_FREQUENCY=120` をエンジンの環境に入れた』とログに出す(`window.__app.hz.hz` = 120)が、**エンジンの wasm はこの環境変数を読まないので、再生中の窓の更新/s は 52.2(この run。D の 60 Hz は glueB の中央値 54.0)、idle は 1,857/s で、60 Hz の値のまま**(`FLE_DWM_HZ` を読む Dp の 120 Hz は 118〜125 更新/s、idle 約 2,000/s)。wasm の中の文字列でも確認した(`FLE_DWM_HZ` 0 件、`FLE_DISPLAYSETTINGS_FREQUENCY` 0 件。Dp は 3 件)。

##### D-hz120

first idle 8.08 s; hz info {"request":"120","hz":120,"measurement":null,"warning":null}

page log lines: ["[info] display refresh rate 120 Hz for the engine (FLE_DWM_HZ=120, FLE_DISPLAYSETTINGS_FREQUENCY=120) from ?hz="]

- main-idle: window updates/s 0.99, idle/s 1054.9, Worker 0.165 cores, virtual/real 1.0003, load 11.3
- play: window updates/s 52.2, idle/s 1857.2, Worker 0.213 cores, virtual/real 1, load 12


**今の既定の入口(PM が 16:11 に構成 F1xn へ切り替えた。`?bundle=` 無し、`/?hz=60`・`/?hz=120` だけ)**: 1 組(60 → 120。短い手順: 起動 → ウィザード 8 s → Enter → アイドル 15 s → マウス 1 s → 単発の移動 8 回 → クリック 6 回 → 再生 20 s)。F1xn は別の構成(Blend2D の置き換え、simd128 なし)なので、Dp や D との直接比較は参考。

**読み**(1 組、load 10〜15。参考):
- **今の既定の入口(F1xn)では `?hz=` が効く**: 窓の更新/s は 60 Hz で 61.7〜63.9、120 Hz で 114〜126(どの段階でも。Enter 前後の立ち上がりの段階は除く)。仮想/実は 1.000(120 Hz の起動直後 25 s だけ 0.58、load 14)。Worker の CPU はアイドル 0.219 → 0.312、再生 0.314 → 0.488 コア(1.4〜1.55 倍)。最初の idle 6.72 → 7.37 s。wasm メモリは 60 Hz 931 MB、120 Hz 1,034 MB(120 Hz の run が起動直後に階段を 1 つ上がっていた。決定 0174 入りの F1xn の再生中の増加は §4.8 の F1xn の表)。
- **発見: 今の既定の入口(F1xn)は、入力の無いアイドルでも主窓を毎フレーム描き直す**。`main-idle`(入力を入れていない 15 s)で窓の更新が **62.5/s(60 Hz)・125.0/s(120 Hz)**。D と Dp のアイドルは 0.7/s。PM が §4c に書いた『アイドルの更新が 55/s なのは、この観測窓がポインタ移動を含むため(読み)』は、入力の無い段階でも 62.5/s だったので当たらない(読みの訂正)。F1x(同じ F1 系)でも同じ(120 Hz のアイドルで 125.1/s)。**アイドルの CPU が重い**: F1xn の Worker 0.219(60 Hz)・0.312(120 Hz)コア、ページ主スレッド 0.092・0.092 コアで、Dp(Worker 0.116・0.163、ページ 0.015・0.014)、D(Worker 0.164、ページ 0.024)の約 2 倍(Worker)・6 倍(ページ)。原因は未調査(F1 系の Blend2D の置き換え〔決定 0167〕などが、画が変わらなくても毎フレーム窓へ書いている、と読む)。毎フレーム何が(どの窓の、どの大きさの矩形が)描き直されるかは測っていない。
- 入力→次の更新は、更新が連続しているので『次の無関係な更新』が 1 フレーム以内に来て小さく出る(`mouse` p50 7.5 ms〔60 Hz〕・4.5 ms〔120 Hz〕、`taps` 3.5・3.9 ms)が、**入力の応答時間ではない**。エンジン処理の p95 は `taps` 29.4〔60〕・17.0 ms〔120〕、`clicks` 47.5〔60〕・33.6 ms〔120〕(Dp より長い)。

#### defE: 2 runs (60: 1, 120: 1); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| 120-1 | ok | 7.37 | 12.1 | 10.8 |
| 60-1 | ok | 6.72 | 11.3 | 15.1 |

##### phase main-idle

| metric | 60 Hz (n=1) | 120 Hz (n=1) |
|---|---:|---:|
| (a) window updates / real s | 62.54 [62.54-62.54] | 125.02 [125.02-125.02] |
| engine idle callbacks / real s | 1051 [1051-1051] | 1107 [1107-1107] |
| (b) Worker CPU, cores (CPU s / real s) | 0.219 [0.219-0.219] | 0.312 [0.312-0.312] |
| (b) Worker CPU s / virtual s | 0.218 [0.218-0.218] | 0.312 [0.312-0.312] |
| (c) page main thread, cores | 0.092 [0.092-0.092] | 0.092 [0.092-0.092] |
| all Chrome threads, cores | 0.384 [0.384-0.384] | 0.483 [0.483-0.483] |
| (e) virtual time / real time | 1.0001 [1.0001-1.0001] | 0.9999 [0.9999-0.9999] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 931.1 [931.1-931.1] | 1033.9 [1033.9-1033.9] |
| load at the start of the phase | 11.3 [11.3-11.3] | 12.1 [12.1-12.1] |

##### phase mouse

| metric | 60 Hz (n=1) | 120 Hz (n=1) |
|---|---:|---:|
| (a) window updates / real s | 63.81 [63.81-63.81] | 125.49 [125.49-125.49] |
| engine idle callbacks / real s | 1051 [1051-1051] | 1084 [1084-1084] |
| (b) Worker CPU, cores (CPU s / real s) | 0.229 [0.229-0.229] | 0.343 [0.343-0.343] |
| (b) Worker CPU s / virtual s | 0.229 [0.229-0.229] | 0.350 [0.350-0.350] |
| (c) page main thread, cores | 0.162 [0.162-0.162] | 0.186 [0.186-0.186] |
| all Chrome threads, cores | 0.571 [0.571-0.571] | 0.735 [0.735-0.735] |
| (e) virtual time / real time | 0.9993 [0.9993-0.9993] | 1.0001 [1.0001-1.0001] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 931.1 [931.1-931.1] | 1033.9 [1033.9-1033.9] |
| (d) input -> next update p50 ms | 7.5 [7.5-7.5] | 4.5 [4.5-4.5] |
| (d) input -> next update p95 ms | 20.4 [20.4-20.4] | 7.9 [7.9-7.9] |
| (d) page -> injected into the engine p50 ms | 0.99 [0.99-0.99] | 0.20 [0.20-0.20] |
| (d) engine processing (inject -> idle) p50 ms | 1.57 [1.57-1.57] | 1.10 [1.10-1.10] |
| (d) engine processing p95 ms | 2.8 [2.8-2.8] | 3.4 [3.4-3.4] |
| (d) idle -> first frame on the page p50 ms | 4.80 [4.80-4.80] | 2.17 [2.17-2.17] |
| (d) inputs counted (n) | 23 [23-23] | 29 [29-29] |
| load at the start of the phase | 13.3 [13.3-13.3] | 11.7 [11.7-11.7] |

##### phase taps

| metric | 60 Hz (n=1) | 120 Hz (n=1) |
|---|---:|---:|
| (a) window updates / real s | 63.89 [63.89-63.89] | 126.33 [126.33-126.33] |
| engine idle callbacks / real s | 1051 [1051-1051] | 1109 [1109-1109] |
| (b) Worker CPU, cores (CPU s / real s) | 0.219 [0.219-0.219] | 0.313 [0.313-0.313] |
| (b) Worker CPU s / virtual s | 0.219 [0.219-0.219] | 0.313 [0.313-0.313] |
| (c) page main thread, cores | 0.089 [0.089-0.089] | 0.099 [0.099-0.099] |
| all Chrome threads, cores | 0.384 [0.384-0.384] | 0.503 [0.503-0.503] |
| (e) virtual time / real time | 0.9991 [0.9991-0.9991] | 1.0000 [1.0000-1.0000] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 931.1 [931.1-931.1] | 1033.9 [1033.9-1033.9] |
| (d) input -> next update p50 ms | 3.5 [3.5-3.5] | 3.9 [3.9-3.9] |
| (d) input -> next update p95 ms | 31.9 [31.9-31.9] | 18.7 [18.7-18.7] |
| (d) page -> injected into the engine p50 ms | 0.60 [0.60-0.60] | 0.14 [0.14-0.14] |
| (d) engine processing (inject -> idle) p50 ms | 2.27 [2.27-2.27] | 2.18 [2.18-2.18] |
| (d) engine processing p95 ms | 29.4 [29.4-29.4] | 17.0 [17.0-17.0] |
| (d) idle -> first frame on the page p50 ms | 0.54 [0.54-0.54] | 0.10 [0.10-0.10] |
| (d) inputs counted (n) | 8 [8-8] | 8 [8-8] |
| load at the start of the phase | 13.3 [13.3-13.3] | 11.7 [11.7-11.7] |

##### phase clicks

| metric | 60 Hz (n=1) | 120 Hz (n=1) |
|---|---:|---:|
| (a) window updates / real s | 63.91 [63.91-63.91] | 126.35 [126.35-126.35] |
| engine idle callbacks / real s | 1048 [1048-1048] | 1090 [1090-1090] |
| (b) Worker CPU, cores (CPU s / real s) | 0.387 [0.387-0.387] | 0.554 [0.554-0.554] |
| (b) Worker CPU s / virtual s | 0.385 [0.385-0.385] | 0.555 [0.555-0.555] |
| (c) page main thread, cores | 0.122 [0.122-0.122] | 0.137 [0.137-0.137] |
| all Chrome threads, cores | 0.611 [0.611-0.611] | 0.815 [0.815-0.815] |
| (e) virtual time / real time | 1.0009 [1.0009-1.0009] | 1.0005 [1.0005-1.0005] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 931.1 [931.1-931.1] | 1033.9 [1033.9-1033.9] |
| (d) input -> next update p50 ms | 13.0 [13.0-13.0] | 16.8 [16.8-16.8] |
| (d) input -> next update p95 ms | 49.1 [49.1-49.1] | 33.8 [33.8-33.8] |
| (d) page -> injected into the engine p50 ms | 0.41 [0.41-0.41] | 0.42 [0.42-0.42] |
| (d) engine processing (inject -> idle) p50 ms | 2.10 [2.10-2.10] | 1.41 [1.41-1.41] |
| (d) engine processing p95 ms | 47.5 [47.5-47.5] | 33.6 [33.6-33.6] |
| (d) idle -> first frame on the page p50 ms | 1.27 [1.27-1.27] | 0.16 [0.16-0.16] |
| (d) inputs counted (n) | 18 [18-18] | 18 [18-18] |
| load at the start of the phase | 14.5 [14.5-14.5] | 11.5 [11.5-11.5] |

##### phase play

| metric | 60 Hz (n=1) | 120 Hz (n=1) |
|---|---:|---:|
| (a) window updates / real s | 62.59 [62.59-62.59] | 124.74 [124.74-124.74] |
| engine idle callbacks / real s | 1878 [1878-1878] | 1891 [1891-1891] |
| (b) Worker CPU, cores (CPU s / real s) | 0.314 [0.314-0.314] | 0.488 [0.488-0.488] |
| (b) Worker CPU s / virtual s | 0.313 [0.313-0.313] | 0.488 [0.488-0.488] |
| (c) page main thread, cores | 0.112 [0.112-0.112] | 0.138 [0.138-0.138] |
| all Chrome threads, cores | 0.517 [0.517-0.517] | 0.728 [0.728-0.728] |
| (e) virtual time / real time | 0.9999 [0.9999-0.9999] | 1.0000 [1.0000-1.0000] |
| (e) lag cuts (times maxLag 1000 ms was hit) | 0 [0-0] | 0 [0-0] |
| (f) wasm memory at the end, MB | 931.1 [931.1-931.1] | 1033.9 [1033.9-1033.9] |
| (d) input -> next update p50 ms | 13.8 [13.8-13.8] | 16.1 [16.1-16.1] |
| (d) input -> next update p95 ms | 21.1 [21.1-21.1] | 24.6 [24.6-24.6] |
| (d) page -> injected into the engine p50 ms | 0.85 [0.85-0.85] | 0.13 [0.13-0.13] |
| (d) engine processing (inject -> idle) p50 ms | 2.42 [2.42-2.42] | 3.45 [3.45-3.45] |
| (d) engine processing p95 ms | 18.5 [18.5-18.5] | 24.0 [24.0-24.0] |
| (d) idle -> first frame on the page p50 ms | 6.81 [6.81-6.81] | 0.47 [0.47-0.47] |
| (d) inputs counted (n) | 5 [5-5] | 5 [5-5] |
| load at the start of the phase | 15.1 [15.1-15.1] | 10.8 [10.8-10.8] |

##### per run (play)

| run | updates/s | idle/s | Worker core | page main core | virtual/real | wasm MB | load |
|---|---:|---:|---:|---:|---:|---:|---:|
| 120-1 | 124.74 | 1891 | 0.488 | 0.138 | 1.0000 | 1033.9 | 10.8->11.6 |
| 60-1 | 62.59 | 1878 | 0.314 | 0.112 | 0.9999 | 931.1 | 15.1->17.1 |


### 4.6 `?hz=auto`(この機械の headless Chrome の rAF)

`?hz=auto` を付けた run(Dp)。ページが rAF を測って丸めた値と、そのときのログ。**最初の版は rAF をダウンロードと並行して測り、60 Hz の画面(rAF は 60.0/s、§4.7)を『21 間隔、中央値 33.33 ms = 30.0 Hz』と測った**(ページがダウンロードの展開と検査で忙しく、フレームが 1 つおきに遅れ、4 s 待ちの打ち切りに当たった。丸めの結果は 60 で偶然正しいが、**120 Hz の画面でも忙しければ 60 と測る、つまり低く測る方向に外れる**〔読み〕)。そのため**ダウンロードの前に測る**形に直した(起動が約 1.1 s 遅れる、`auto` だけ)。直した版(`auto2`)の結果を併記する。**これは headless Chrome(この機械)の値で、120 Hz のディスプレイでの `auto` は未実測**。

##### auto (first version: the frame rate was measured while the bundle downloaded)

first idle 8.94 s; hz info {"request":"auto","hz":60,"measurement":{"measuredHz":30.003000300030017,"intervals":21,"hz":60,"note":"21 frame intervals, median 33.33 ms = 30.0 Hz, rounded to 60 Hz"},"warning":null}

page log lines: ["[info] ?hz=auto: 21 frame intervals, median 33.33 ms = 30.0 Hz, rounded to 60 Hz","[info] display refresh rate 60 Hz for the engine (FLE_DWM_HZ=60, FLE_DISPLAYSETTINGS_FREQUENCY=60)"]

- main-idle: window updates/s 0.99, idle/s 1054.6, Worker 0.119 cores, virtual/real 1, load 12.6
- play: window updates/s 52.16, idle/s 1865.2, Worker 0.172 cores, virtual/real 1, load 13.1

##### auto2/1 (final: measured before the downloads)

first idle 8.08 s; hz info {"request":"auto","hz":60,"measurement":{"measuredHz":60.006000600060034,"intervals":61,"hz":60,"note":"61 frame intervals, median 16.66 ms = 60.0 Hz, rounded to 60 Hz"},"warning":null}

page log lines: ["[info] ?hz=auto: 61 frame intervals, median 16.66 ms = 60.0 Hz, rounded to 60 Hz","[info] display refresh rate 60 Hz for the engine (FLE_DWM_HZ=60, FLE_DISPLAYSETTINGS_FREQUENCY=60)"]

- main-idle: window updates/s 0.99, idle/s 1055.6, Worker 0.097 cores, virtual/real 1, load 11.3
- play: window updates/s 52.08, idle/s 1855.9, Worker 0.144 cores, virtual/real 1, load 10.8

##### auto2/2 (final: measured before the downloads)

first idle 7.84 s; hz info {"request":"auto","hz":60,"measurement":{"measuredHz":60.006000600060034,"intervals":61,"hz":60,"note":"61 frame intervals, median 16.66 ms = 60.0 Hz, rounded to 60 Hz"},"warning":null}

page log lines: ["[info] ?hz=auto: 61 frame intervals, median 16.66 ms = 60.0 Hz, rounded to 60 Hz","[info] display refresh rate 60 Hz for the engine (FLE_DWM_HZ=60, FLE_DISPLAYSETTINGS_FREQUENCY=60)"]

- main-idle: window updates/s 0.6, idle/s 1053.4, Worker 0.097 cores, virtual/real 1, load 6.3
- play: window updates/s 52.18, idle/s 1853.2, Worker 0.152 cores, virtual/real 1, load 6


### 4.7 ページの `requestAnimationFrame`(ページの実フレーム)

ページ自身の `requestAnimationFrame` のコールバックの回数と間隔(アイドルと再生、各 12 s)を、`?hz=60` と `?hz=120` の Dp で数えた(`profA` の run の中。rAF の計数は計測の段階だけで入れる〔毎フレーム呼ぶとページが余計にフレームを作るため〕)。『窓の更新/s』(ページの canvas に矩形を描いた回数)と並べる。**ブラウザが実際に画面に出したフレームではなく、ページが描画の機会を得た回数**(headless は画面を持たない)。

##### the page's requestAnimationFrame (headless Chrome 156, SwiftShader) while the main window idles and while the project plays, 12 s each, Dp at ?hz=60 and ?hz=120

| run | stretch | page rAF callbacks / s | interval p50 ms | p95 ms | p99 ms | max ms | intervals > 20 ms | window updates drawn on the page canvases / s | load |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 120-1 | idle | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 0.8 | 19.1 |
| 120-1 | play | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 125.0 | 16.9 |
| 120-2 | idle | 60.0 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 0.6 | 11.3 |
| 120-2 | play | 60.0 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 125.0 | 11.2 |
| 60-1 | idle | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 0.8 | 8.3 |
| 60-1 | play | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 62.4 | 8.9 |
| 60-2 | idle | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 0.7 | 9 |
| 60-2 | play | 60.1 | 16.67 | 16.67 | 16.67 | 16.7 | 0 | 62.4 | 8.6 |


**読み**: ページの rAF は、8 本すべてで 60.0〜60.1 回/s、間隔は p50・p95・p99・最大とも 16.67 ms(load 8〜19 でも崩れない)。headless Chrome は 60 Hz の仮想 vsync でページの描画の機会を与え、`?hz=` に依らない。**120 Hz の再生で ページが canvas に描いた回数は 125.0/s(60 Hz は 62.4/s)で、1 つの rAF フレームに約 2 回分の更新が載る**(読み)。したがって、この環境で測れるのは『ゲストが 120 Hz で窓を描き直し、ページの canvas がそれに追いついている』ことまでで、**ブラウザがそれを 120 fps で画面に出すか(フレーム落ちを含む)は、120 Hz 以上のディスプレイのブラウザでしか測れない**。

### 4.8 再生中の wasm メモリ(Dp は決定 0174 の前の構成と PM が追記)

Dp で再生を 180 s 続け、30 s ごとに wasm メモリ・ヘープ・速度を記録した(`soak`)。120 Hz は 60 Hz の 2 倍の速さで窓を描き直すので、決定 0174 の漏れ(再生中、更新ごとに GDI のスロットが再利用されずにメモリが階段状に増える)が 2 倍の速さで進むはず(読み)。F1x は fin1 の別構成(Blend2D の置き換え、simd128 なし)で、**決定 0174 の前の runtime で作られたもの**(fin1 の決定 0176 の表: F1x/F2x = 0174 前、**F1xn/F2xn = HEAD b0f989d の runtime = 0174 入り**。マニフェストの『HEAD 75cfbbf』という説明は、作った時点の HEAD を指すだけで、runtime の木ではない)。**今の既定の入口は 0174 入りの F1xn**。Dp との直接比較は参考。

##### play soak (bundle Dp at 60 Hz): wasm memory and rates every 30 s (the samples of the 15 s timer are in the result file)

| at s | wasm MB | page JS heap MB | Worker JS heap MB | virtual / real | idle / s | window updates / s | Worker cores | page main cores |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1029.0 | 2.5 | 12.4 | - | - | - | - | - |
| 30 | 1029.0 | 44.6 | 14.9 | 0.9999 | 2025 | 62.5 | 0.176 | 0.088 |
| 60 | 1029.0 | 1.9 | 16.5 | 1.0000 | 2021 | 62.5 | 0.174 | 0.089 |
| 90 | 1029.0 | 44.2 | 19.3 | 1.0000 | 2025 | 62.5 | 0.183 | 0.098 |
| 120 | 1227.9 | 38.2 | 20.8 | 1.0000 | 2026 | 62.5 | 0.173 | 0.090 |
| 151 | 1227.9 | 30.8 | 13.5 | 1.0000 | 2022 | 62.5 | 0.180 | 0.092 |
| 181 | 1227.9 | 2.9 | 11.6 | 1.0000 | 2029 | 62.5 | 0.172 | 0.090 |

memory series of the 15 s timer: 15s:1029MB(wizard) 30s:1029MB(play) 45s:1029MB(play) 60s:1029MB(play) 75s:1029MB(play) 90s:1029MB(play) 105s:1029MB(play) 120s:1029MB(play) 135s:1029MB(play) 150s:1227.9MB(play) 165s:1227.9MB(play) 180s:1227.9MB(play) 195.1s:1227.9MB(play) 210s:1227.9MB(play) 225.1s:1227.9MB(play)

##### play soak (bundle Dp at 120 Hz): wasm memory and rates every 30 s (the samples of the 15 s timer are in the result file)

| at s | wasm MB | page JS heap MB | Worker JS heap MB | virtual / real | idle / s | window updates / s | Worker cores | page main cores |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1029.3 | 19.5 | 14.5 | - | - | - | - | - |
| 30 | 1029.3 | 2.4 | 17.6 | 1.0000 | 2053 | 125.0 | 0.278 | 0.102 |
| 60 | 1223.2 | 40.2 | 21.4 | 1.0000 | 2045 | 124.8 | 0.286 | 0.106 |
| 90 | 1223.2 | 2.4 | 20.7 | 1.0001 | 2049 | 125.0 | 0.270 | 0.100 |
| 121 | 1467.9 | 43.9 | 12.2 | 1.0000 | 2039 | 125.0 | 0.269 | 0.097 |
| 151 | 1467.9 | 9.7 | 35.1 | 1.0000 | 2042 | 125.0 | 0.279 | 0.103 |
| 181 | 1467.9 | 47.7 | 13.1 | 1.0000 | 2049 | 125.0 | 0.288 | 0.106 |

memory series of the 15 s timer: 15s:1029.3MB(wizard) 30s:1029.3MB(play) 45s:1029.3MB(play) 60s:1029.3MB(play) 75s:1029.3MB(play) 90s:1223.2MB(play) 105s:1223.2MB(play) 120s:1223.2MB(play) 135s:1223.2MB(play) 150s:1223.2MB(play) 165s:1467.9MB(play) 180s:1467.9MB(play) 195s:1467.9MB(play) 210s:1467.9MB(play) 225s:1467.9MB(play)

##### play soak (bundle F1x (before decision 0174) at 120 Hz): wasm memory and rates every 30 s (the samples of the 15 s timer are in the result file)

| at s | wasm MB | page JS heap MB | Worker JS heap MB | virtual / real | idle / s | window updates / s | Worker cores | page main cores |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1030.1 | 6.7 | 12.3 | - | - | - | - | - |
| 30 | 1229.0 | 119.8 | 9.3 | 1.0000 | 2020 | 124.8 | 0.458 | 0.119 |
| 60 | 1229.0 | 96.1 | 25.6 | 1.0000 | 2020 | 124.1 | 0.465 | 0.122 |
| 90 | 1474.8 | 391.1 | 25.7 | 1.0000 | 2015 | 124.2 | 0.453 | 0.117 |
| 120 | 1474.8 | 231.6 | 27.8 | 1.0001 | 2017 | 124.2 | 0.455 | 0.121 |
| 151 | 1474.8 | 145.7 | 21.1 | 0.9999 | 2010 | 124.0 | 0.480 | 0.129 |
| 181 | 1474.8 | 13.8 | 24.3 | 1.0000 | 2009 | 123.1 | 0.585 | 0.147 |

memory series of the 15 s timer: 15s:1030.1MB(wizard) 30s:1030.1MB(play) 45s:1030.1MB(play) 60s:1030.1MB(play) 75s:1229MB(play) 90s:1229MB(play) 105s:1229MB(play) 120s:1229MB(play) 135s:1474.8MB(play) 150s:1474.8MB(play) 165s:1474.8MB(play) 180s:1474.8MB(play) 195s:1474.8MB(play) 210.1s:1474.8MB(play) 225s:1474.8MB(play)

##### play soak (the default entrance = bundle F1xn (decision 0174 included) at 120 Hz): wasm memory and rates every 30 s (the samples of the 15 s timer are in the result file)

| at s | wasm MB | page JS heap MB | Worker JS heap MB | virtual / real | idle / s | window updates / s | Worker cores | page main cores |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1033.9 | 4.5 | 13.8 | - | - | - | - | - |
| 30 | 1033.9 | 133.7 | 19.1 | 1.0000 | 2013 | 124.5 | 0.454 | 0.126 |
| 60 | 1033.9 | 318.0 | 22.8 | 1.0000 | 2019 | 123.8 | 0.492 | 0.137 |
| 90 | 1033.9 | 5.5 | 30.0 | 1.0000 | 2010 | 124.4 | 0.466 | 0.127 |
| 121 | 1033.9 | 130.6 | 32.3 | 1.0000 | 2007 | 124.2 | 0.458 | 0.127 |
| 151 | 1033.9 | 271.4 | 25.6 | 1.0000 | 2011 | 124.6 | 0.439 | 0.117 |
| 181 | 1033.9 | 309.1 | 27.7 | 1.0001 | 2014 | 123.8 | 0.534 | 0.141 |

memory series of the 15 s timer: 15s:1033.9MB(wizard) 30s:1033.9MB(play) 45s:1033.9MB(play) 60s:1033.9MB(play) 75s:1033.9MB(play) 90s:1033.9MB(play) 105s:1033.9MB(play) 120s:1033.9MB(play) 135s:1033.9MB(play) 150s:1033.9MB(play) 165s:1033.9MB(play) 180s:1033.9MB(play) 195s:1033.9MB(play) 210.1s:1033.9MB(play) 225.1s:1033.9MB(play)


**読み**:
- **Dp(決定 0174 の前の構成と PM が追記)では、wasm メモリの階段が再生の更新の速さに比例して進む**。120 Hz(125 更新/s): 再生を始めて約 40〜50 s で 1,029 → 1,223 MB(+194、+19%)、さらに約 75 s 後に → 1,468 MB(+245、+20%)。60 Hz(62.5 更新/s): 約 100〜110 s で最初の階段 1,029 → 1,228 MB(+199)、180 s の間に 2 つ目は無い。= 120 Hz は 60 Hz の約 2.2 倍の速さで階段に達する(更新が 2 倍)。階段の幅(+19〜20%)と値(1,029 → 1,223〜1,228 → 1,468)は、決定 0174 が clk1 の測定として記録したもの(1,028.9 → 1,227.8 → 1,473.4 MB)と一致する。= 0174 の漏れ(GDI のスロットが再利用されず、更新ごとに増える)と同じ原因と読む(PM の読みと整合。未確認)。
- 3 つ目の階段(読み: 約 1,762 MB)は 120 Hz でさらに約 90 s 後、4 つ目(約 2,115 MB)は上限 2,048 MB を超えるので、**120 Hz の再生は約 5〜6 分で止まる**見込み(60 Hz は約 8〜11 分。決定 0174 は 60 Hz で約 8 分と記録)。180 s では止まるところまでは測っていない。
- **予測の経緯**: 16:50 に『F1x は 0174 を含むはず』と思い、『120 Hz の 180 s でも階段は出ず、最初の再利用の後は平坦』と予測して F1x を測ったが、**F1x は 0174 の前の構成だった**(上)ので、予測の前提が誤りだった。F1x の結果は下。**正しい対象は 0174 入りの F1xn(= 今の既定の入口)**で、**17:08 に次の予測を書いてから測った**: F1xn は 120 Hz の 180 s の再生でも wasm メモリが平坦(fin1 は 60 Hz の F2xn で 930.9 MiB・22 標本・増加 0 と測っている)。結果は『F1xn』の表。
- **F1x(0174 の前)の結果**: 120 Hz・再生 180 s で wasm メモリは階段状に増え続けた: 1,030 → 1,229 MB(再生を始めて約 40〜60 s)→ 1,475 MB(さらに約 60 s 後)。Dp とほぼ同じ形。= 0174 の前の構成の漏れが 120 Hz で出ている。(この F1x の run は、私が F1x を 0174 入りと取り違えたためのもので、取り違えは上に書いた。)
- **F1xn(0174 入り = 今の既定の入口)の結果**(`?bundle=` 無し、`?hz=120`、再生 180 s、load 11〜20、1 run): **事前の予測は『平坦』の部分が当たり、『931 MB』の部分は外れた**。wasm メモリは最初の標本(起動の 15 s、ウィザード)から **1,033.9 MB で、225 s の最後の標本まで 15 点すべて同じ**(再生は 30 s 以降の 14 点。階段なし。Dp と F1x は同じ長さで 2 段上がった)。ただし**水準は 931 MB でなく 1,034 MB**(+103 MB、+11%): 同じ入口の 60 Hz の短い run(§4.5)は 931 MB だったので、120 Hz の run では起動の 15 s までに 931 → 1,034 MB に上がっている(読み。Dp・F1x の +19〜20% の階段とは幅が違う)。原因は調べていない。F1xn の 60 Hz の 180 s は測っていない。= **この 1 run では、0174 入りの F1xn は 120 Hz の再生 195 s でメモリが増えなかった**(1 run、180 s。より長い再生と複数 run は未測定。『漏れが直った』と言い切るには足りない)。ページの JS ヘープは 4〜318 MB で上下するだけ(GC)。

### 4.9 Worker とページ主スレッドの CPU の内訳(60 Hz と 120 Hz、Dp)

同じ CDP プロファイル(200 µs 間隔、Worker とページ主スレッドを同じ 15 s)を `?hz=60` と `?hz=120` の Dp で。表の列名 60・120 は Hz。µs/idle は『エンジンの idle 1 回あたり』で、120 Hz では idle の回数が毎秒 約 1,111(アイドル)・1,985(再生)。

#### profA: 4 runs (60: 2, 120: 2); cells are median [min-max]

| run | exit/err | first idle s | load at start of main-idle | load at start of play |
|---|---|---:|---:|---:|
| 120-1 | ok | 10.85 | 23.6 | 21.6 |
| 120-2 | ok | 8.43 | 11.7 | 11.4 |
| 60-1 | ok | 7.88 | 8.8 | 8.6 |
| 60-2 | ok | 7.37 | 10.2 | 9.7 |

##### phase main-idle: CPU per engine idle callback (microseconds)

| metric | 60 (n=2) | 120 (n=2) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 93.8 [91.1-96.6] | 152.8 [133.9-171.6] |
| page main thread CPU us / idle (/proc) | 13.3 [13.3-13.3] | 14.4 [14.3-14.4] |
| Worker + page main us / idle | 107.1 [104.4-109.8] | 167.1 [148.2-186.0] |
| engine idle callbacks / s | 1055 [1054-1056] | 1114 [1107-1120] |
| window updates / s | 0.70 [0.60-0.80] | 0.49 [0.40-0.59] |
| virtual / real | 1.0000 [0.9998-1.0001] | 1.0000 [0.9999-1.0000] |
| load at the start | 9.5 [8.8-10.2] | 17.6 [11.7-23.6] |

##### phase play: CPU per engine idle callback (microseconds)

| metric | 60 (n=2) | 120 (n=2) |
|---|---:|---:|
| Worker thread CPU us / idle (/proc, 100 Hz ticks) | 83.9 [80.3-87.5] | 136.4 [130.8-141.9] |
| page main thread CPU us / idle (/proc) | 35.5 [34.1-36.8] | 45.6 [43.8-47.5] |
| Worker + page main us / idle | 119.3 [114.4-124.3] | 182.0 [174.6-189.4] |
| engine idle callbacks / s | 1735 [1731-1738] | 1763 [1758-1769] |
| window updates / s | 44.06 [44.05-44.08] | 87.55 [87.55-87.55] |
| virtual / real | 1.0000 [1.0000-1.0000] | 1.0000 [1.0000-1.0000] |
| load at the start | 9.1 [8.6-9.7] | 16.5 [11.4-21.6] |

##### CDP profile, idle (200 us sampling; self time per engine idle callback, microseconds)

| metric | 60 (n=2) | 120 (n=2) |
|---|---:|---:|
| Worker busy (all) us / idle | 88.7 [87.3-90.1] | 150.5 [137.6-163.4] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 76.1 [74.2-77.9] | 134.6 [122.3-146.9] |
| Worker JS only (named functions) us / idle | 7.92 [7.78-8.05] | 8.68 [7.35-10.00] |
| Worker (program) us / idle | 4.08 [3.73-4.42] | 6.64 [5.98-7.31] |
| Worker GC us / idle | 0.66 [0.44-0.88] | 0.55 [0.53-0.58] |
| page main busy us / idle | 14.2 [13.9-14.4] | 16.1 [14.0-18.3] |
| page main JS only us / idle | 1.44 [1.41-1.46] | 1.75 [1.75-1.76] |
| page main (program) us / idle | 12.3 [12.1-12.5] | 14.2 [12.0-16.3] |
| idle callbacks / s in the stretch | 1054 [1053-1055] | 1112 [1112-1112] |
| load at the start | 10.1 [8.4-11.7] | 13.0 [10.8-15.2] |

##### CDP profile, play (200 us sampling; self time per engine idle callback, microseconds)

| metric | 60 (n=2) | 120 (n=2) |
|---|---:|---:|
| Worker busy (all) us / idle | 107.9 [79.4-136.3] | 119.8 [114.9-124.7] |
| Worker wasm (guest + runtime + JSPI frames) us / idle | 84.8 [63.8-105.7] | 100.1 [96.4-103.7] |
| Worker JS only (named functions) us / idle | 9.16 [6.66-11.67] | 7.64 [7.18-8.11] |
| Worker (program) us / idle | 13.00 [8.26-17.75] | 11.39 [10.57-12.22] |
| Worker GC us / idle | 0.95 [0.73-1.17] | 0.72 [0.67-0.77] |
| page main busy us / idle | 49.7 [37.2-62.2] | 42.7 [41.2-44.1] |
| page main JS only us / idle | 5.63 [4.60-6.66] | 7.58 [7.18-7.99] |
| page main (program) us / idle | 43.4 [32.2-54.5] | 34.4 [33.5-35.3] |
| idle callbacks / s in the stretch | 2028 [2026-2030] | 2050 [2049-2051] |
| load at the start | 12.1 [10.1-14.2] | 12.0 [10.2-13.8] |

##### the guest's time sequence (idle number, virtual clock, next deadline) hashed; equal at every 500th idle up to the first input?


## 5. P2 の現在地(空のプロジェクト、この機械、代理環境。所定のプロジェクトは未測定)

### 5.1 60 Hz と 120 Hz の (a)〜(f)(構成 Dp、交互 3 組の中央値〔範囲〕。load は §4.4 の表)

| 項目 | Dp 60 Hz(`?hz=60`) | Dp 120 Hz(`?hz=120`) | 備考 |
|---|---:|---:|---|
| (a) 窓の更新/s(ページの canvas に矩形を描いた回数)、再生 | **59.00** [58.98-59.00] | **117.91** [117.90-117.92] | ちょうど 2.00 倍。120 Hz の周期(8.33 ms)の 98%。この 68 s の段階は Space を押して再生が始まるまでの数秒を含む。再生が定常になった 12 s の区間(profA、§4.7)では 60 Hz 62.4/s、120 Hz 125.0/s(矩形の数なので周期より多いことがある) |
| (a) 同、アイドル(何も動かない主窓) | 0.67 | 0.78 | 動くものが無ければ更新は出ない(キャレットの点滅程度) |
| (a) 同、クリックの 20 s の間 | 43.4 | 84.7 | クリックの後、窓が動き続ける区間がある |
| (b) Worker の CPU、再生(コア = CPU 秒/実秒) | 0.182 [0.165-0.200] | 0.263 [0.255-0.305] | 1.45 倍(中央値)。最も静かだった組(load 5.4 と 6.9〜8.9)は 0.165 → 0.255(1.55 倍) |
| (b) 同、アイドル | 0.116 [0.108-0.120] | 0.163 [0.163-0.173] | 1.4 倍 |
| (b) 仮想 1 秒あたりの CPU 秒 | 仮想/実 = 1.000 なので (b) と同じ | 同じ | |
| (c) ページ主スレッド、再生 | 0.105 [0.086-0.110] | 0.106 [0.090-0.110] | 120 Hz でも増えない(描画回数は 2 倍。1 回あたりの費用が小さいため、と読む〔未確認〕) |
| (c) 同、アイドル | 0.015 | 0.014 | |
| (d) 単発のポインター移動 → 次の更新 p50(`taps`、30 回) | 3.9 ms [3.5-6.3] | 5.1 ms [3.4-7.7] | p95 は『見た目が変わらない移動の後、次の無関係な更新を数える』ので意味が薄い(436 / 118 ms) |
| (d) ボタンの押下・離し → 次の更新 p50(`clicks`、約 60 レコード) | 10.0 ms [8.7-15.3] | 7.4 ms [7.1-10.4] | p95 は 321 / 308 ms(同じ理由) |
| (d) エンジンの処理(注入 → 次の idle)p50 / p95、`taps` | 2.25 / 19.6 ms | 2.28 / 18.1 ms | **p95 の約 18〜20 ms が『重い周期』**(ゲストの木の歩き 1〜2 周期分) |
| (d) 同、`clicks` | 1.72 / 35.9 ms | 1.56 / 34.4 ms | クリック後の重い周期 約 34〜36 ms(120 Hz の 4 フレーム分) |
| (d) ページ → Worker → エンジンへ注入するまで p50 | 0.15〜0.20 ms | 0.14〜0.19 ms | 入力は即時注入(clk1) |
| (d) 連続移動(`mouse`)→ 次の更新 p50 | 202 ms | 265 ms | **入力の遅延ではない**: 動かしても見た目が変わらない領域の入力が多く、『次の無関係な更新』までの時間(更新は 7〜10/s)を数えている |
| (e) 仮想時間/実時間(アイドル・マウス・タップ・クリック・再生) | 1.0000 | 1.0000(0.9999〜1.0002) | `maxLag` 1,000 ms に当たった回数(lag cuts)は 0。起動直後の `wizard`(最初の 25 s、ゲストの初期化の山)だけ 60 Hz 0.95〜1.01、120 Hz 0.845(`runs/hzA`) |
| (f) wasm メモリ(再生の終わり) | 1,029.0 MB | **1,223.2 MB**(3 回とも同じ) | 120 Hz の再生で +194 MB の階段。60 Hz は 68 s の再生で増えない。§4.8 |
| 最初の idle(P1) | 中央値 8.01 s [6.89-9.32](6 本、load の中央値 11) | 中央値 8.40 s [7.81-10.85](6 本、load の中央値 11) | 120 Hz は約 +0.4 s(ばらつきの範囲に収まる。load が近い 1 組では 6.89 → 7.81 s)。P1 の 10 s は、load が高い run では超えうる(§4.4 の『最初の idle』の表) |

**参考: 既定の入口の構成 D(60 Hz 固定)**(glueB の 5 本ずつの中央値、同じ手順): 窓の更新/s 再生 54.0、アイドル 0.65〜0.70。Worker CPU は再生 0.227(HEAD の前端)→ 0.190(本差分)、アイドル 0.164 → 0.152 コア。ページ主スレッド 再生 0.080 → 0.071、アイドル 0.024 → 0.015。仮想/実 1.0000。wasm メモリ 1,028.9 MB のまま。Dp の 60 Hz(再生 59.0 更新/s、0.18 コア)より、D の方が再生中の更新が少なく(54.0)、CPU は多い(D は dispatch キャッシュ等の無い構成。別構成なので直接比較は参考)。

### 5.2 『120 fps に届いたか』

- **ゲストが窓を描き直し、ページの canvas に反映した回数は 117.9/s(再生中、空のプロジェクト)**。ゲストの周期 120 Hz の 98%。60 Hz のときの 59.0/s のちょうど 2 倍。= **窓の更新として 120/s に実質届く**(Dp、この機械、空のプロジェクト、再生)。
- ただし『更新/s』は『ゲストが再描画して Worker が送り、ページが `putImageData` した回数』で、**ディスプレイに出たフレームの数ではない**。ページの実フレーム(`requestAnimationFrame` の間隔)は §4.7 にある。**この機械の headless Chrome 156 では、ページの `requestAnimationFrame` は 60 Hz でしか来ない**(60.1 回/s、間隔 p50 16.665 ms・p95 16.670 ms・最大 16.670 ms、20 ms を超えた間隔 0 回。アイドルでも再生中でも、`?hz=60` でも `?hz=120` でも同じ。§4.7)。120 Hz の再生中にページの canvas へ描いた回数は約 125/s(定常の 12 s の区間)で、**画面に出せるのは最大 60/s**(合成は 1 フレームに複数の更新をまとめる、読み)。= **この環境では『ディスプレイが 120 fps で表示する』ことは確かめようがない**。120 Hz 以上のディスプレイでは rAF が 120 回/s 来るはずだが、未測定。
- **入力への反応は、エンジンの処理自体は速い(p50 1.5〜2.3 ms、注入まで 0.2 ms 以下)が、入力 1 回のあと p95 で約 18〜36 ms の『重い周期』が出る**。120 Hz のフレーム(8.33 ms)の 2〜4 個分で、この間は連続性が切れる(読み)。これは 60 Hz でも 120 Hz でも同じ長さ(1 周期の仕事は周期の長さに依らない)。

### 5.3 最大の障害(実測で、何が一番大きいか)

大きい順に(いずれも空のプロジェクト、この機械):

0. **今の既定の入口(F1xn、PM が 16:11 に切り替え)は、入力の無いアイドルでも主窓を毎フレーム描き直す**(1 組、load 10〜15、§4.5): 窓の更新/s がアイドルで 62.5(`?hz=60`)・125.0(`?hz=120`)、Dp と D は 0.7。Worker の CPU はアイドル 0.219 → 0.312 コア(60 → 120 Hz)・再生 0.314 → 0.488 コア、ページ主スレッドはアイドルで 0.092(Dp 0.015)。= 今の既定の入口で `?hz=120` にすると、**何もしていなくても Worker 約 0.31 コア+ページ約 0.09 コアを使い続ける**(Dp の約 2 倍と約 6 倍)。原因は調べていない(F1 系の Blend2D の置き換えが、画が変わらなくても毎フレーム窓へ書く、と読む。毎フレーム何が描き直されるかは未測定)。120 Hz の再生で Worker 0.49 コアは 1 コアに余裕があるが、**ブラウザの主スレッドと GPU/合成に毎フレームの仕事を回し続ける**ことが、依頼者の機械の 120 fps と入力の遅延に効くかは未測定。
1. **ゲストの CPU(wasm)**。Worker が 1 idle に使う時間のうち wasm(ゲスト+ランタイム+JSPI のフレーム)が 79〜89%(§4.1、§4.9)。120 Hz にすると、Worker の CPU は再生で 0.18 → 0.26 コア、アイドルで 0.116 → 0.163 コアと **1.4〜1.55 倍**(ui1 の外挿は 1.62 倍)。定常の再生は 120 Hz でも余裕がある(Worker 0.26〜0.30 コア)。ui1 の関数別の測定(ノード上、60 Hz の D')では VCL の木の歩き 7 関数が 60% で、今回は名前つきの wasm が無く Chrome 上では関数別に測り直していない。ゲストの wasm は変えない指示だったので、hyb1/hyb3 の直接 wasm 生成部の仕事。
2. **入力 1 回あとの『重い周期』**。エンジンの処理(注入 → 次の idle)の p50 は 1.2〜2.3 ms と速いが、**p95 が移動 18〜20 ms、クリック 34〜36 ms**(60 Hz でも 120 Hz でも同じ長さ)。ゲストが入力のたびに木を歩き直す周期(ui1: マウス移動 1 回で +68k 関数入口 = 1 周期分)で、120 Hz のフレーム(8.33 ms)の 2〜4 個分。この間は更新が途切れる(連続した 120 fps が崩れる)。定常のアイドル/再生にはなく、入力の直後だけ。= ゲスト CPU の項目 1 と同じ原因(木の歩き)と読む(Chrome 上の関数別の測定は無い)。
3. **ブラウザの描画/転送(ページ主スレッド)**: 再生中 0.09〜0.11 コア(60/120 とも、120 Hz で描画の回数が 2 倍でも増えない)、アイドル 0.015 コア。プロファイルでは再生中の idle あたり 35〜62 µs の大半が `(program)`(ブラウザ内部の受信・複製・描画)。**主因ではない**。ただし headless の rAF は 60 回/s で、**表示側が 120 を出せるかは未測定**。
4. **JS のグルー**: Worker の idle あたり 4〜7%(約 6〜10 µs、削減後)。**主因ではない**(§4.1〜4.2)。
5. **仮想時計**: 1.0000 を 60/120 Hz とも保ち、`maxLag` に当たらない(定常)。起動直後の 25 s(ゲストの初期化の山)だけ 120 Hz で 0.845(60 Hz は 0.95〜1.01)。

**CPU の余裕**: ui1 の外挿(『クロック 1.0 倍・120 Hz で再生 約 0.9 コア、余裕 約 1.1 倍』)に対し、実測は Dp で **0.26〜0.30 コア**(Worker)。外挿は clk1 前のグルー(idle あたり 0.6 ms)と load 15〜34 の CPU 秒を含んでいたので悲観的だった(読み)。所定のプロジェクトで木の歩きが何倍になるかは未測定。

## 6. テスト

| 項目 | 結果 | 記録 |
|---|---|---|
| `npm run typecheck`(4 つの tsconfig) | クリーン | `logs/typecheck-vitest.txt` |
| vitest(単体) | 824 passed + 7 skipped(基準 7757f60)→ **955 passed + 7 skipped**(+131、ワークツリー) | `logs/vitest-final.txt` |
| ・`hz.test.ts` | 79: `?hz=` の解釈(受理 5、拒否 21〔空・0・23・1001・小数・符号・先頭の 0・空白・16 進・`Auto`・2 回〕)、標準値への丸め(32 点。境界の 67/68、82/83、103/105、131/132、154/155、198/201 を含む)、偽のフレーム時計での測定(60・75・120・144・240 Hz・30 fps→60、遅いフレームが混じる、±15% のゆらぎ、rAF が止まる、5 fps に絞られる、間隔 0)、環境への書き込み(要求なし → 何も変えない、両変数に同じ値、`?env=` の上書き、食い違いの警告) | |
| ・`surface-sync.test.ts` | 21: `fruity_surface.c` の書き込みを写した JS のモデル(新規面、サイズ変更、移動、非表示、layer 変更、解放、idle の layer_sync、ピクセル、背圧、後から来る canvas の bind)で 2,000 idle × 12 シード、stamp あり/なしが通知・bind・描画列まで完全一致(静かな idle のスキップが 100 回以上あることも確認)。個別: 静かな表のスキップ、変化の検出、背圧で見送った矩形を忘れない、後から bind した canvas に全面、スタンドインは stamp なし、`lseq` が奇数のとき記録しない。変異 4 件を検出 | |
| ・`appview-poll.test.ts` | 27: 既定は毎 idle 読む(間引きは opt-in)、最初の 1 回、`enable(40)` 後の 40 ms の間隔、入力が処理中・表が変わった・面のイベントで待たずに読む、静かなエンジンが 1,000 idle/s で 250 回/10 s、入力の連続、`?poll=` の解釈(受理 4、拒否 14) | |
| ・`client-init.test.ts` | 4: `init` メッセージに `stamp` を載せるのは明示の `false` のとき、`pollMs` は与えたときだけ | |
| 変異検査(`test/mutation/run.mjs --only <22 件> --layers unit`) | **22/22 検出**(最初の版は 11/12 で、先頭の 0 を許す変異が生き残ったためテストを足した) | `logs/mutation-p2a.txt` |
| Worker のテスト(`test/worker/run.mjs`、wrangler dev --local) | 43/43(`app/worker/` は無変更) | `logs/worker-tests.txt` |
| e2e(`node test/e2e/run.mjs`、dev・preview・worker、スタンドイン) | **853/853**(dev 289・preview 289・worker 275。失敗 0。`record:` の記録 26 件は数えない。基準は 691/691 で +162。load 約 13、2 分で完走) | `logs/e2e-final.txt` |
| 差分を HEAD の `git archive` に当てて(`patch -p1`)typecheck と vitest | HEAD 996bd2d の `git archive HEAD app` に `patch -p1 --dry-run` が rc=0(fuzz・offset・reject なし)→ `patch` rc=0 → typecheck(4 つの tsconfig)クリーン → vitest(`APP_REPO_ROOT` をリポジトリに向ける。app だけの archive には `lift/` のファイルが無く、読む 3 つのテストが落ちるため)**954 passed + 8 skipped**。同じ HEAD の素の `app/` は 823 passed + 8 skipped なので **+131**(新しいテストの数と一致)。ワークツリーでは 955 passed + 7 skipped で、1 件の pass/skip が入れ替わる(素の HEAD でも 7757f60 の 824 + 7 から 823 + 8 になっており、私の差分と無関係。どのテストかは特定していない) | `logs/apply-check.txt` |
| lift の全体テスト | 回していない(`lift/` を変えていない) | |

**変異検査で見つかった穴**: 最初の版では『先頭の 0 を許す』変異(`/^[1-9][0-9]{0,3}$/` → `/^[0-9]{1,4}$/`)が生き残った(`'012'` は範囲で弾かれるため)。`'0120'`・`'024'`・`'00060'` を拒否するテストを足して検出するようにした。

## 7. 出典のない選択(S-名。撤回条件つき)

| 名前 | 選択 | 出典 | 代替 | 撤回する条件 |
|---|---|---|---|---|
| S-hz-standard | `?hz=auto` の丸め先は 60/75/90/120/144/165/240 | 無し(市販のディスプレイでよく見る値を依頼文が挙げた) | 測った値をそのまま(整数に丸めて)渡す | 依頼者のディスプレイが 100 Hz/110 Hz 等で、丸めが不利なとき |
| S-hz-round | 丸めは比(`abs(ln(測定値/標準値))` 最小)で選ぶ。60 以下は 60、240 以上は 240 | 無し | 差の絶対値で選ぶ(境界が 105 Hz で 90/120 の同点になり、150 Hz 付近が 144 に偏る) | 360 Hz 等で 240 の上限が不利になったとき(CPU は 1.62 倍/2 倍で増えるので上限は安全側) |
| S-hz-window | 測定窓 1,000 ms(最初の 5 間隔を捨てた後) | 無し | 500 ms / 2 s | 測定が 60 Hz と 59.94 Hz を取り違える等、丸めが揺れたとき(実測では揺れなかった: §4.6) |
| S-hz-skip | 最初の 5 フレーム間隔を捨てる | 無し(ページの起動直後は style/layout/初回描画でフレームが遅れる) | 0 / 10 | 測定値が最初の数フレームで偏るとき |
| S-hz-median | 窓内のフレーム間隔の**中央値**から Hz を出す | 無し(ダウンロード・コンパイル中のメインスレッドの遅れ、タブの一時的なスロットルに強い) | 平均(遅いフレームに引きずられて 120 が 75 になる: 単体テスト `hz-mean-not-median` の変異) | 中央値が外れるほどフレームが不規則なとき |
| S-hz-min | 窓に 20 間隔未満なら測定不能 | 無し | 10 / 40 | - |
| S-hz-giveup | 2,000 ms 待っても足りなければ測定不能(`hz.ts` の既定は 4,000 ms、`main.ts` が 2,000 ms を渡す) | 無し(隠れたタブなど rAF が止まる場合の最大の待ち) | 1 s / 4 s | 隠れたタブで開いたときの起動(P1)の遅れが問題になったとき |
| S-hz-before-download | `?hz=auto` の測定は、ダウンロードの前に行う(起動が約 1.1 s 遅れる) | 並行して測った最初の版が 60 Hz の画面を 30 Hz と測った(実測。ページが忙しい間は rAF の間隔が伸びる) | 並行して測り、結果が怪しければ測り直す / Worker の環境を後から渡す | `auto` の起動の遅れが問題になったとき |
| S-hz-fallback | 測定不能なら 60 Hz(既定)でログに warn | 無し | エラーにする | 依頼者が、測れないときは止めてほしいと言ったとき |
| S-hz-explicit | `?hz=<n>` は 24〜1000 の任意の整数(標準値に限らない)。10 進で符号・先頭の 0・空白なし | ランタイム(fle_thread.c)が受け付ける範囲 | 標準値だけ許す | - |
| S-hz-both | 2 つの環境変数に**同じ値**を渡す(DWM_HZ と DISPLAYSETTINGS_FREQUENCY) | ui1(決定 0166)の方針。2 つを別にすると、ゲストは片方を読んで他方で周期が決まる | 別々のつまみとして出す | ゲストが食い違う値で別の挙動を示すと分かったとき |
| S-hz-wins | `?hz=` は `?env=` とマニフェストの `engine.env` より優先する | 無し | `?env=` を優先 | - |
| S-hz-error | 不正な `?hz=` は、ダウンロードを始める前にエラー画面で拒否する(同じパラメータが 2 回も不正) | 依頼文(不正値は拒否してエラー表示) | 警告して 60 に戻す | - |
| S-hz-standin | スタンドインの bundle では `?hz=` を無視して warn | 無し(スタンドインには環境変数もゲストも無い) | エラー | - |
| S-stamp-real-only | 表の stamp(§3.2)は実エンジンだけで使う。スタンドインは毎回完全に読む | スタンドインの表の書き手は `fruity_surface.c` の規律(layout を変える所は必ず `lseq` か `gen` を増やす)を約束していない | 両方で使う | スタンドインの書き手も同じ規律と確認したとき |
| S-stamp-fields | stamp は各枠の `lseq`(語 0)と `gen`(語 17)と `live` の数だけを見る | `fruity_surface.c` の全書き手を読んだ結果(§3.2) | 全語を比べる(遅い) | `fruity_surface.c` に、`lseq` も `gen` も増やさず layout の語を変える書き手が加わったとき(下の変異検査と単体テストの書き手モデルを更新する) |
| S-dom-guard | ページの `renderStatus`/`renderPhases` は、表示する文字列が前回と同じなら DOM に書かない | 無し | 書き続ける | - |
| S-poll-gap | **opt-in `?poll=<ms>`(既定は毎 idle = 従来)**。指定したとき、何も起きていなければ `<ms>` に 1 回だけ窓一覧(`fle_av_poll`)を読む。測定は 40 | 無し(ページがエンジンの状態を聞くのが元々 40 ms 間隔〔ReportThrottle〕なので、それに合わせた) | 20 ms / 100 ms | 効果が実証できたとき(今回は実証できず、PM の指示で既定 OFF) |
| S-poll-reasons | `?poll=` のとき、次のときは待たずに読む: 入力が処理中(`awaitingCount + queued > 0`)、表を完全に見た(何か書かれた)、面のイベント、最初の 1 回 | 無し(カーソルとポインターは入力で動く。窓の追加・移動・表示は描画を伴うと読んだ) | 時間だけ | 描画を伴わない窓の変化で、ページの表示(窓の配置)が目に見えて遅れたとき |

## 8. 次に効く順の候補(推奨は書く。決めるのは PM)

1. **既定の入口は 16:11 に F1xn へ替わった(`?hz=` が効く構成)**。D のままでは効かなかった。以後、既定の入口を替えるときは、`FLE_DWM_HZ` を読むこと(wasm の文字列で確認できる)と、`?bundle=<新しい構成>&hz=60` が現行の既定と同じ面・同じゲスト時刻列(§4.2〜4.3 の手順: `p2a-run.mjs`・`p2a-sha.mjs` + `summarize.mjs glue`)になることを確かめる。
2. **窓一覧の間引き(`?poll=<ms>`)**: 既定は OFF。効果が実証できなかったので、使うなら先に『窓一覧の更新が 40 ms 遅れても画面が目に見えて遅れないか』と効果を再測定すること。
3. **hz の既定を 60 のままにするか**: 空のプロジェクトでは 120 Hz で Worker は再生中 0.26〜0.30 コア(Dp)で、CPU の余裕は大きい(§4.4、§5.1)。ただし (a) 決定 0174 より前の構成は GDI スロットの消費が 2 倍の速さになり、wasm メモリの階段状の成長と再生の停止(60 Hz 約 8 分〔0174〕→ 120 Hz 約 5〜6 分、読み)が早まる。0174 の前の F1x でも同じ階段が出た。0174 入りの F1xn(今の既定の入口)は §4.8 の結果を見よ。(b) 所定のプロジェクトでの木の歩きの増え方は未測定。(c) 依頼者のディスプレイが 120 Hz 未満なら 120 Hz は無駄。`?hz=auto` はページの rAF から決めるので、依頼者のディスプレイでの値は依頼者のブラウザで初めて分かる(headless の rAF は §4.6、§4.7)。= **既定は 60 のまま、`?hz=auto` を試す価値がある**、が推奨。
4. **ゲストの重い周期(入力 1 回あたり engine 処理 p95 18〜36 ms)**: クリックや単発のポインター移動の後に 1〜4 フレーム分(120 Hz で 8.3 ms 単位)の計算が出る。これは 120 fps の連続性を一時的に壊す最大の項目で、ゲストの CPU(ui1 の木歩き 7 関数 = 60%)。hyb1/hyb3 の直接 wasm 生成部の仕事。グルー(JS)は Worker の 7%、ブラウザの描画/転送(ページ主スレッド)は 0.1 コア前後で、どちらも主因ではない。
5. **所定のプロジェクトで測る**(FLP を開く経路が要る。F5)。木の歩きはコントロール数に比例して増える(読み)ので、空のプロジェクトの余裕(120 Hz で Worker 0.26〜0.30 コア)がそのまま残るとは限らない。
6. **`fle_idle_lib.js`(idle ごとに `Module['fleIdleDeliver']`/`fleIdleWaiting` のクロージャと `new Promise`)の削減**: エンジンの再リンクが要る。名前のある JS で約 0.15 µs/idle と読むので、優先度は低い。
7. **ページの DOM の書き込み抑制を他の周期処理にも**: 今回は `renderStatus`/`renderPhases` だけ。`renderPerf` は性能パネルが閉じている間は早期 return。
8. **`docs/milestones/performance-targets.md` §4a の P2 の行の更新候補**(PM の作業): P2 の窓の更新/s は『再生 62.5/s〔60 Hz の定数が上限〕』に加え『Dp + `?hz=120` で 117.9/s』、入力→更新は『単発の移動 p50 3.9〜5.1 ms、クリック p50 7〜10 ms、エンジンの処理 p95 18〜36 ms』(いずれも空のプロジェクト、この機械、load 5〜14)。ただし (a) 所定のプロジェクトは未測定、(b) 既定の入口の D では `?hz=` が効かない、(c) 『更新/s』はディスプレイのフレームではない、を併記すること。
9. **`?hz=auto` の測定をページの描画で忙しい間に行わない**(S-hz-before-download)。今は起動の前に約 1.1 s を使う。起動(P1)の 1.1 s が惜しい場合は、Worker にエンジンの環境を『開始の直前に』渡す経路(`init` とは別のメッセージ)を足し、ダウンロードの終わりのあと(ページが静かになってから)に測るとよい。
10. **依頼者のブラウザで 120 fps を確かめる経路(計測ビーコン、決定 0152、PM の依頼。設計だけ。実装していない)**: ビーコンにはすでに `ui.updates`(ページが canvas に描いた回数。合計と直近 10 s の間隔の統計)と `browser`(`devicePixelRatio`、画面、UA など)がある。足りないのは『ブラウザが実際にページへ与えた描画の機会』と『`?hz=` の値』。(1) `display: { hzRequest, hzApplied, hzMeasured, rafPerS, rafIntervalP50Ms, rafIntervalP95Ms, rafIntervalMaxMs, rafSampleMs }`(約 150 バイト、上限 16 KB に収まる)を `BeaconPayload` に足す。(2) rAF は常時は回さず(毎フレーム呼ぶとページが余計にフレームを作る)、first-idle のビーコンと periodic の直前に 2 s だけ数える(`measureFrameRate` と同じ純関数に `HzMeasurement` を載せて使い回せる)。(3) 依頼者は `?hz=auto` でページを開くだけで、ビーコンのログ(`/__perf`、`tools/perf-log-summary`)に『ブラウザの rAF は 120 回/s か、ページの canvas への描画は 125/s か、間隔の p95』が残る。(4) サーバー側(`tools/perf-middleware.ts`)は 16 KB・JSON object の検査だけなので、スキーマの変更は要らない見込み(読み。`perf-beacon.test.ts` の期待値は変わる)。既定の入口の挙動は、`display` を足すだけで変わらない。
11. **今の既定の入口(F1xn)がアイドルで毎フレーム主窓を描き直す理由の調査**(新しい発見、§4.5): D と Dp は入力が無ければ更新 0.7/s なのに、F1 系(F1x も)は 62.5/s(60 Hz)・125/s(120 Hz)。アイドルの CPU が Dp の約 2 倍(Worker)・6 倍(ページ主スレッド)。何が描き直されているか(どの窓の、どの矩形か。`fle_raster_wrote` の呼び元、`TableStamp` の `gen` が毎 idle 進むか)を調べ、Blend2D の置き換え(決定 0167)や prune の差で説明できるなら直す。直せば、更新の回数が入力に応じて増減する普通の形に戻り、P2 の CPU の余裕も増える(読み)。逆に、毎フレーム描き直すのがゲストの本来の挙動で D/Dp が描き忘れているのなら、D/Dp 側の問題(面の SHA は一致している)。

## 9. 確認できなかったこと

- **所定のプロジェクト(`serina1-serum2.zip`)は開いていない**(アプリに FLP を開く経路が無い)。すべての値は**空のプロジェクト**の主窓のもの。木の歩きはコントロール数に比例して増える(読み。ui1)ので、所定のプロジェクトでの 120 fps は未測定。
- **依頼者の機械・実ディスプレイ(120 Hz 以上)・実 GPU・実機 Windows**: 未測定。この機械は 16 コア、SwiftShader、headless Chrome 156。headless の `requestAnimationFrame` の周期は §5 で実測した値で、**120 Hz のディスプレイでの `?hz=auto` は未実測**。
- 他のブラウザ(Firefox、Safari)、隠れたタブでの `?hz=auto`(測定不能 → 60 Hz のフォールバックは単体テストのみ、ブラウザでは未確認)。
- `?hz=` を既定の入口で効かせること(既定の構成 D の wasm は `FLE_DWM_HZ` を読まない。構成 Dp 以降への切替は未決)。
- ゲストの関数単位の CPU(ui1 の木歩き 7 関数 = 60%)を 120 Hz の構成 Dp で測り直すこと(名前つきの wasm と node の `--cpu-prof` が要る。ゲストの wasm は変えない指示だった)。
- 音(P3)。AudioWorklet(`?audio=1`)を付けた状態の 120 Hz。
- `fle_idle_lib.js`(エンジン側の idle のグルー)の削減(エンジンの再リンクが要る)。
- **stamp の安全性の限界**: `fruity_surface.c` の全書き手を読んだ結果と、書き手のモデルでの単体テスト、Chrome 走行での面の SHA の一致が根拠。将来 `fruity_surface.c` に、`lseq` も `gen` も増やさずに layout の語を変える書き手が加わると、stamp はそれを見落とす(その場合は `S-stamp-fields` を見直す。`?stamp=0` で従来に戻せる)。
- **窓一覧の間引き(S-poll-gap、opt-in `?poll=<ms>`)**: 既定 OFF。使ったときの効果は実証できず、描画も入力も無い窓一覧の変化(タイトル、フォーカス、描画を伴わない窓の表示/非表示)が最大 `<ms>` 遅れて届くことの影響は測っていない。
- 『更新/s』は、ページの canvas に矩形を描いた回数で、ディスプレイが表示したフレームの数ではない。headless Chrome は画面を持たないので、実際の表示フレームの落ち方は未測定。

## 10. 規則違反・逸脱の自己申告

- **指示との違い(測定の入口)**: 指示は『既定のバンドルに `?hz=120` を付けるだけで測る』だったが、既定の構成 D の wasm は `FLE_DWM_HZ` を読まないので、60/120 の系列は `?bundle=/assets/fl-engine-fin1-Dp/manifest.json` を明示して測った(中間報告 1 で PM に伝え、了解を得た)。D と Dp は別の構成なので、D の数字と Dp の数字は参考としてしか並べていない。
- **コマンドの拒否**: 作業の最初に、背景起動のつもりで末尾 `&` と `sleep 0` を付けたコマンドを 1 回(拒否)、`sleep 1` を 1 回(拒否)、後半にも `sleep 0` を付けたコマンドが 2 回、さらに末尾 `&` が 1 回拒否された(計 5 回、いずれも待つ必要の無い所で癖で打った。拒否されたコマンドは実行されていない)。存在しない `bash` ツールを計 3 回呼んだ(最後の 1 回は 17:40 頃、`echo test` だけ)(shell ツールを使い直した)。その後は `background: true` だけを使い、待ちのループは書いていない。
- **自分の測定のやり直し(3 回)**: (0) 最後に、PM の指示で窓一覧の間引きを既定 OFF にしたため、2 つ目のキューを再走の途中で止めて(自分の bash の PID だけ kill)、最終ビルドでやり直した(glueC)。(1) 系列 hzA の最初の 1 本目(約 2 分、クリックの段階を足す前の版)を、(2) 最初のキュー(sha・glueB の 1 組・…)を glueB の 1 組の後で、窓一覧の読み取りの間引き(`appview-poll.ts`)を足すために、自分で起動した bash・node・Chrome の PID だけ `kill` して止め、やり直した。他の担当のプロセスには触っていない。止めたキューの途中結果は `runs/glueB-pre-poll`・`runs/sha-pre-poll`(間引きの前の版〔`dist-after-B1`〕の 1 組)に残してあり、文書では『中間の版』と書いて使った。
- **負荷**: 他の担当のビルドと走行で load は 4〜22 だった。同時に動かした診断用 Chrome は 1 本。ただし、系列の途中で自分の vitest・変異検査・Worker のテスト(wrangler dev)・node のマイクロベンチを動かした(いずれも数十秒、`nice -n 19`)。その間に走っていた系列の run は CPU 秒に数%の誤差があり得る。
- **p2a-run.mjs の途中差し替え**: 系列 hzA の途中(2 本目の run の最中)で `p2a-run.mjs` に『wasm メモリの 15 s の時系列』と『`--raf-s`』を足した(一時ファイルを作って `mv` で原子的に入れ替え、実行中の run には影響しない。足したのは記録と、引数を渡したときだけ動く段階で、既存の段階の動作は同じ)。hzA の 1 本目と 2 本目は `memSeries` を持たない。
- **待ちの書き方**: 次のキューを前のキューの終わりに続けるため、`tail --pid=<PID> -f /dev/null`(指定したプロセスが終わるまで 1 回ブロックするだけ。ループでもポーリングでもない)を計 3 回使った(キューの連結 2 回と、キューの終わりを待つ 1 回)。ハーネスは受け付けたが、『待ちのための特別な仕掛け』と読まれる可能性があるので申告する。

- **ディスク使用量制限(自分の確認スクリプト)**: 17:40 頃、適用確認の `tools/apply-check.sh` が `git archive HEAD`(リポジトリ全体)を `/tmp/opencode/p2a-apply` に展開し、『ディスク使用量制限を超過しました』で失敗した(約 101 MB 書いたところ)。その run の vitest の 56 件の失敗は不完全な木のためで、無視した。展開先を消し、`git archive HEAD app`(`app/` だけ)に直して、`APP_REPO_ROOT` をリポジトリに向けて再実行した(§6)。その間に他の担当の書き込みを妨げたかは確認していない。
- **e2e の作業ディレクトリ**: `node test/e2e/run.mjs` は既定の作業ディレクトリ(`/tmp/opencode/app1/e2e`)を使った。他の担当が同じ既定で同時に e2e を動かしていれば衝突し得る。私の走行は 853/853 で完走し、`e2e-results.json` の件数(879 件中 失敗 0、`record:` 26)と一致した。
- **取り違え(自分で気づいて直した)**: 構成 F1x を『決定 0174 入り』と取り違え(マニフェストの説明の『HEAD 75cfbbf』を、runtime の木だと読んだ)、『120 Hz の 180 s でも平坦』という予測を F1x に対して書いて測り、外れた。その後 fin1 の決定 0176 を読んで、**F1x は 0174 の前、F1xn が 0174 入り(今の既定の入口)**と分かった。予測の前提の誤りだった。F1xn の予測を測定の前に書き直して(17:08)測った(§4.8)。中間報告 3 にも『F1x は 0174 を含むはず』と書いたので、ここで訂正する。

## 11. 差分と既存の挙動への影響

- `p2a.diff`: **`app/` の 17 ファイル**(新規 9、変更 8)。`lift/`・wasm・生成 C・`app/worker/`・`app/scripts/`・`app/src/audio/`・`app/src/plugins/`・既定の manifest は変えていない。FLEngine の既定の生成 C の sha256 は `cd9107ce…` のまま(lift は無変更なので全体テストは回していない)。
  - 新規: `app/src/engine/hz.ts`、`surface-sync.ts`、`appview-poll.ts`、`app/test/unit/hz.test.ts`、`surface-sync.test.ts`、`appview-poll.test.ts`、`client-init.test.ts`、`app/test/e2e/p2a-run.mjs`、`p2a-sha.mjs`(後ろの 2 つは測定の道具)。
  - 変更: `app/src/main.ts`(`?hz=`、`?stamp=0`、DOM の書き込み抑制)、`app/src/engine/worker.ts`(`SurfaceSync`、`AppViewPoll`、`nowAbs`)、`surface-table.ts`(`TableStamp`、数個の getter)、`idle-driver.ts`(getter 1 つ: `awaitingCount`)、`protocol.ts`・`client.ts`(`init` に任意の `stamp`)、`app/test/e2e/run.mjs`(hz 系シナリオ)、`app/test/mutation/run.mjs`(変異 22 件)。
- **既定の入口の挙動**: `?hz=` が無ければエンジンの環境に何も入れない。ページの DOM は同じ文字列なら書かない(表示される内容は同じ)。Worker は表が変わっていなければ読まない(§3。結果は同じ)。窓一覧は従来どおり毎 idle 読む(opt-in の `?poll=<ms>` を付けたときだけ間引く: S-poll-gap)。ゲストの wasm・時刻列は変わらない(§4.3)。
- **diff の `-` 行は全部読んだ**: 削除した行は、置き換えた行(`performance.timeOrigin + performance.now()` → `nowAbs()`、`pumpAll`/`syncSurfaces` → `SurfaceSync`、`renderStatus`/`renderPhases` の DOM 書き込みに条件、`main.ts` の先頭コメントと `app` の初期値の 1 行に項目を足した)だけで、意図しない削除は無い。

## PM の決定と取り込み(2026-11-03、追記)

取り込み: `p2a.diff`(`app/` の 17 ファイル、基準 HEAD 996bd2d)。PM の検証: `patch -p1 --dry-run` は offset・fuzz・reject なしで当たる。`-` 行 63 本(`init` の引数、`syncSurfaces` の `surface-sync.ts` への移動、`pollAppView` の呼び出し、`now`、URL パラメータの注記、状態表示など)を読み、意図した置換で、aud2〔0175〕・clk1〔0170〕の機能の削除は無いことを確認した。typecheck(rc=0)、vitest **955 passed + 7 skipped**(担当は 954 + 8 skipped と報告。skipped の数の差は mt1 の OTT テストの環境依存で、passed は +1)、Worker 43/43、e2e **853/853**(基準 691/691)。`lift/` には触れていない。PM は Chrome での 60/120 Hz、グルー削減、面の SHA を再測定していない(担当の実測)。進捗(G 段階)には数えない。

判断:

1. **既定 ON**: table stamp・`performance.timeOrigin` の毎回取得の廃止・DOM 書き込み抑制(グルー削減の 3 点。ゲスト時刻列ハッシュが 20 run 一致、Enter 前の主窓の面が native の基準と一致)。**opt-in のまま**: 窓一覧の間引き `?poll=<ms>`(効果を実証できず、窓一覧の更新を遅らせる正しさのリスクだけが残るため)。
2. **`?hz=`**: 取り込み。既定(パラメータ無し)は 60 のまま。既定の入口の構成 F1xn(決定 0176)では効く(62.5→125.0/s)。構成 D では効かない(wasm が `FLE_DWM_HZ` を読まない)。`?hz=auto` は rAF をダウンロードの前に測る(起動 +1.1 s、auto のみ)。headless では 60 に丸まる。
3. **Enter 後の絵の同一性**は『同じ』と言えなかった(読み取りが約 26 回/s で、同じ前端どうしでも順序・集合が揃わない。後にしか無い絵 0 枚、前にしか無い絵 1 枚)。全フレームを記録する方法での確認は未実施(後の作業)。
4. **F1xn(既定の入口)がアイドルでも毎フレーム描き直す異常**(D・Dp は 0.7/s、F1xn は 62.5/s〔60 Hz〕): 決定 0176 の取り込み時に PM が `performance-targets.md` §4c に書いた『アイドルの更新 55/s はポインタ移動を含むため(読み)』は**誤りだった**(訂正は §4c に追記)。原因は idle1(決定 0187)が調べる。結果次第で既定の入口の構成を見直す。
5. **120 fps の表示**: headless の rAF は 60 回/s なので、ページの canvas に描いた回数(125.0/s)と画面に出せる回数は分けて書く。ディスプレイでの確認は依頼者のブラウザだけでできる。ビーコン(rAF の周期と描画回数を `/__perf` に送る)の設計は p2a の recommend 項目 10。実装は後の作業(既定の入口の挙動を変えない形で)。
