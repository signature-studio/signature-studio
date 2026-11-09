# 決定 0240(仮): ウインドウ移動とフレームレート — 依頼者の機械の計測は本番では 0 件(原因はビーコンの credentials:'omit')、移動の内訳は『FL 自身の再描画が支配、合成+提示は 20〜46%』、ゴーストは再現せず(ただし窓管理の黙って捨てる欠陥 2 件を契約どおりに直した)、WebGL2 の合成器を唯一の経路として実装し画素同一を確認した(実 GPU の性能は未測定)

状態: **仮の決定(すべての選択)**。作業者 `gpu1`、2026-11-05 JST 06:20〜(UTC 21:20〜)。基準 HEAD は 52034ca9 から始め、差分は PM の指示で**2 本に分割**(互いに独立): `diffs/gpu1-uncover.diff`(runtime の窓管理だけ。HEAD 3688a940 〔決定 0243 opt1・0241 aud1・0242 pick1・0244 alpha1 の取り込み済み〕に対する物で、PM が **1a6383d2** として取り込み済み)と `diffs/gpu1-gl.diff`(GL の合成器とビーコンの 2 ブロック。最新の HEAD **1a6383d2** の `git archive` に `patch -p1 --dry-run` が通る)。`diffs/gpu1.diff` は分割前の古い版(参考。使わない)。区分: **実測**=このセッションのコマンドの出力を見た事実。**読み**=コード・文書の解釈。**未確認**。**出典なしの選択**=`S-gpu1-N`(§9)。**この機械(16 vCPU の Zen 5 VM、実 GPU なし〔SwiftShader〕、headless Chrome 156・Firefox 158、load 5〜45 で他担当と共有)の値で、依頼者の機械(Firefox 159 / Windows 10 / GTX 980)ではない。入力はすべて CDP / WebDriver の模擬で、人の操作ではない。実機 Windows との照合は 0 件。**

## 結論(先頭の数行)

1. **依頼者の本物の機械で測れた事実**: (a) R2 `fl-web-cache` の `perf/` は **0 オブジェクト**(読んだ時点で 265 件、終了時に 291 件、全て `cache/`)。(b) 本番 Worker の `wrangler tail`(読み取り)は、依頼者の Firefox 159 / Windows が 2026-11-04T22:05:48Z にページを読み込んだ(6 要求)のを見たが、その後 tail を閉じるまで(00:37Z。依頼者の要求自体は 22:29:58Z が最後)**POST /__perf は 0 件**。認証なしの POST /__perf は Access の 302(curl)。tail からはセッションが first idle に達したか(= ビーコンが送られる状態だったか)は分からない。**読み**: `app/src/perf/beacon.ts` が `credentials:'omit'` で送るため Access の Cookie が付かず、ビーコンは Worker に届かない。単独の差分は PM が取り込み済み(a61760ba)。(c) 読めた依頼者の機械の値は、旧経路(窓ごとの canvas、bundle cc975e05 / f4a57e84)の 2026-11-03 18:21Z〜11-04 00:07Z の **20 セッション**(Firefox 159 / Windows)だけ: first idle 5.7〜13.3 s(1 件は 114 s)・中央値 10.9 s、エンジン idle 22.9/s(目標 60)、イベント→rAF p50 の中央値 29.8 ms、p95 が 500 ms 以上のセッション 6/17。**現在の経路(デスクトップ合成器)の依頼者の計測は 0 件**。
2. **ウインドウ移動(FL の Mixer パネルのヘッダードラッグ。実エンジン F2xH7、現行 2D 経路)の内訳**: FL は mouse down で SetCapture し、**マウス移動ごとに自分で `SetWindowPos`(flags 0x14)で子窓を動かし**、1 ステップに約 4 回、パネル全面(1920 画面で 1514x500 = 757,000 画素)を **FL 自身が** BeginPaint+BitBlt で再描画する(OS のモーダル移動ループは使われない: ランタイムの窓は非クライアント領域 0、トップレベルは wizard 等のみ。『SC_MOVE の実装が契約どおりか』は対象外)。Worker の CPU は 1 移動 **8.7〜12.2 ms**(Chrome 1920: 8.69、Chrome 3240: 10.5、Firefox 1920: 12.15、Firefox 3240: 10.95。load 6〜20)、うち合成+提示(compose+put)は **20%・26%・33%・46%**(1 移動あたりに直した値)。残りは FL の描画とコピー(ゲスト)。この機械でも Worker は 60 移動/s で Chrome 52〜63%、Firefox 82〜96% 忙しい。**GPU 合成が減らせるのは最大でこの 20〜46%**(下限はアップロード分)。Settings のタブ切替は 1 クリック **Worker CPU 0.37〜0.74 s** で、合成+提示は 9〜29%(ゲストが支配)。
3. **ゴースト(移動の軌跡が残る)は再現しなかった**。Chrome 156 / Firefox 158、1920x1080 / 3240x1946、遅いゲスト(2 CPU に固定)、参照プロジェクトを開いた状態、バースト入力を含む 10 系列(§4)で、Worker の canvas・コンポジタの絵・窓表から描き直した絵・ブラウザのスクリーンショット(1:1)が一致(差は動いている部品の 8〜76 画素だけ)。層の見立て(読み): (B) コンポジタ・(C) この機械の canvas では起きない。ただし**窓管理に『上限を超えると黙って捨てる』欠陥を 2 件見つけ、native の試験で壊れ方を示し、契約どおり(過剰に無効化)に直した**: ① snapshot の窓が `UNC_MAX`=160 を超えると表の後ろの窓を落とす、② 可視領域が `FLE_VISRGN_MAX`=128 矩形を超えるとその窓の露出の無効化を飛ばす(`continue`)。HEAD の runtime では 2 件とも『親/兄弟が無効にならない』(= 軌跡が残る形)、直した runtime では無効になる。実際の Mixer では窓 12〜18・溢れ 0 だったので、**これが依頼者のゴーストの原因とは言えない**(未確認)。
4. **WebGL2 の合成器(案 G)を唯一の経路として実装した**(`?present=GL` は作らない。方針 no-options): C(`fle_desk.c` の `fle_desk_scene`。version 2 = alpha1(決定 0244)のガラス余白 `gx0..gy1` と窓領域の箱 `cl..cb` を含む)は窓の並び・位置・面のアドレス・レイヤー・書かれた矩形だけを書き、JS(`desk-gl.ts`)は窓の知識なしに texSubImage2D(wasm メモリから直接、surface の stride で)と quad 描画を行う。**採用条件(PM 指定、1 か所 `desk-gl.ts webgl2Usable` ← `worker.ts makeDeskPresenter`)**: wasm に `fle_desk_scene` があり、`getContext('webgl2', { failIfMajorPerformanceCaveat: true })` が null でなく、`MAX_TEXTURE_SIZE` が 8192 以上。満たさなければ従来の 2D。**実測(§7.3): この機械の headless Chrome 156(SwiftShader)も Firefox 158(llvmpipe)も、この旗つきで WebGL2 を返す = この条件は『ソフトウェア GL なら 2D』にならない(この機械では常に GL が選ばれる)。** 依頼者の GTX 980 では GL になると読むが、旗が実機で何と答えるかは未確認。**画素同一**: GL の読み戻しと CPU コンポジタの絵は、動いている部品(メーター)以外で 0 画素差(Chrome・Firefox、wizard・主窓・File メニュー・Settings 窓・Mixer 移動後。ブラウザのスクリーンショットと GL の読み戻しは差 0。**この実エンジンでの比較は scene version 1・alpha1 取り込み前・採用条件の変更前の版 F**)。最終版(version 2、ガラス・窓領域つき)は、実 Chrome の WebGL2(SwiftShader)で `GlDeskPresenter` を直接駆動する試験 `app/test/e2e/desk-gl-presenter.mjs`(窓 7 つ: 不透明・constant alpha・colour key・per-pixel alpha・ガラス〔余白あり/シート〕・窓領域つき、strided の面。式は alpha1 の `draw_rect`/`over_premul` の JS 版)で **全 8 段階が 0 画素差**。層状窓(alpha・colorkey・per-pixel alpha)の式は SwiftShader で 16 例・16,384 画素が CPU の式と**完全一致**。**最終の木の実エンジン(Chrome 156・Firefox 158、既定の入口、コミット同士の GL 対 2D、§7.4)**: GL は選ばれ、窓・メニュー・ピッカー・Mixer の移動が出て、GL と CPU の絵は動く部品以外で一致した。Mixer の移動の Worker CPU/移動は Chrome で −13%(1920)・−54%(3240)、Firefox で変化なし(+9%/−1%、ばらつき内)だが、**提示される絵の数/s はソフトウェア GL では 2D より少ない**(Chrome 60→21〜43、Firefox 3240 で約 100→約 40)。**性能は実 GPU が無いので証明できていない**: ソフトウェア GL では Worker の CPU は 1 移動 −25%〜+14%(ばらつき)、3240x1946 では SwiftShader が律速になり提示/s が半減(Chrome 60.8→26.7、Firefox 75→49)。Worker 側で測れる compose+put+draw は 1920 で 1.7→0.96 ms(Chrome)、3240 では 2.7→4.0 ms(影のコピーの比較が O(面積))。
5. **次の手**: ① 実 GPU(依頼者の GTX 980)での計測 — 本番に載せてビーコンの `desk`・`engineRun` ブロック(今回足した読み取り専用の測定)を見る。② 3240 級の大きい矩形での影コピーの費用を減らす(タイルのハッシュ等。S-gpu1-4)。③ (済み)alpha1 の式は取り込み済みなので `fle_scene_win` を version 2 に拡張した。④ `fle_gdi.c` の『可視領域が 128 矩形を超えると名前つきで止まる』(`pixel_stop`)は、BeginPaint の DC については更新矩形の内側だけで領域を作る形に直した(§5)。矩形そのものが 128 を超える描画では最後の手段として止まる(領域の容量を動的にする変更は大きいので未着手)。⑤ 採用条件(§7.3)がソフトウェア GL を 2D にしないので、必要ならレンダラ名の検査(SwiftShader / llvmpipe / Basic Render)を足すか PM が決める(私は足していない: 実機の文字列を確かめられない)。

## 冒頭の三点(正直さ)

1. **何を実測したか(実行した本物と範囲)**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → wasm にした物)の出荷構成 F2xH7 を、**CI の段スクリプト**(`ci/stage-*.sh`、`CI_JOBS=4`、`git archive` の木)で HEAD 52034ca9 から**冷間ビルド**した(extract 2 s・felift 7 s・lift 35 s・b2d 31 s・objects 341 s・link 694 s・bundle 374 s = 約 24.7 分、wasm 133,924,267 B)。runtime だけを変えた版(診断入り D、scene 入り E、scene+影の trim+窓管理の直し F)は、その objects を hard link で借りて runtime の objects だけ作り直す増分ビルド(44 単位は全部再利用 = 生成 C は同一)。これらを `vite preview` で配り、隔離 headless Chrome 156(CDP)と Firefox 158 headless(geckodriver)で動かした。参照プロジェクト(`~/ダウンロード/serina1-serum2.zip`、読み取り専用、コミットしない)は、ページのファイル管理窓(OPFS)に 49 ファイルを入れて File > Open で開いた。ビーコンは `wrangler dev` の remote:true の R2 バインディング(list/get のみ)と `wrangler tail`(読み取り)と `curl` の認証なしの POST で読んだ(資格情報は作っていない)。代役・画像の再生・期待値の貼り付けは 0。**最終の木(HEAD 1a6383d2 + gpu1-gl)でも実エンジンを動かした(§7.4)**: CI の段スクリプトで冷間ビルドし(02:00Z 時点で完了、約 48 分、load 20〜47 の共有)、比較用に HEAD(2D)を増分ビルドし(約 33 分)、隔離 headless Chrome 156 と Firefox 158 の**既定の入口**(クエリなし)で、first idle・GL が選ばれること・窓・File メニューの開閉・Mixer の移動・F8 のプラグインピッカー・F10 の設定窓を確かめ、GL の読み戻しと CPU コンポジタの絵が動く部品(メーター・アニメ)以外で 0 画素差であることを確かめた。
2. **結論**: 上の『結論』のとおり。見つけたこと: ビーコンが Access に拒否される不具合(取り込み済み)、窓管理の黙って捨てる 2 件(直した)、移動の内訳、GL 合成器(画素同一、性能は未証明)。**見つからなかったこと**: ゴーストの再現と原因。
3. **未確認**: 依頼者の機械(Firefox 159 / Windows / GTX 980 / DPR 1.7647 / 実マウス)でのゴースト・移動・GL の動作と速さ。実 GPU での GL の費用(ここは SwiftShader)。本番にビーコンが届くこと(PM が次の配備後に tail と `perf/` を読む)。コンテキスト喪失からの復帰を**ブラウザで**確かめること(`Browser.crashGpuProcess` の後の CDP がこの Chrome で応答せず 2 回タイムアウト。復帰の論理は単体試験のみ)。層状窓の GL の画素一致は SwiftShader だけ(実 GPU のブレンドの丸めは実装依存で ±1 の可能性)。実エンジンに層状窓が出る場面では GL と CPU を比べていない(いま FL の既定の画面に層状窓は無かった: §7.1)。全体のテスト基準(152 results・2,118 passed など)は §8 の範囲だけ再実行。最終の木の実エンジンでの確認(§7.4)は、模擬入力・この機械・ソフトウェア GL の範囲。採用条件が依頼者の機械でどう答えるか(旗つきの WebGL2 が取れるか、`MAX_TEXTURE_SIZE`)。リポジトリの e2e 層(`test/e2e/run.mjs`)は最終の木で再実行していない(headless の Chrome は旗つきでも GL が取れるので、製品のページ経由の e2e は今後 GL の経路を通る)。

## 1. 依頼者のビーコン

### 1.1 本番(現在)

- `wrangler dev` に `r2_buckets: [{ binding, bucket_name: "fl-web-cache", remote: true }]` の小さな Worker(`tools/r2read/`。list と get しか呼ばない)で既存の wrangler ログインから読んだ: `perf/` は `[]`、全体 265 オブジェクトは `cache/` のみ(2026-11-03T22:49Z〜11-04T21:16Z の更新)。**実測**。再確認は終わり近く(00:36Z)でも `perf/` 0 件(全 291 件が `cache/`)。
- `wrangler tail fl-studio-web` を 21:3xZ から開いたままにした。依頼者の Firefox 159 / Windows の要求(`tables/worker-tail-requests.tsv`。IP・メール・Cookie 等は集計時に捨て、生ログは削除した)は 22:05:48Z の GET / 200 と静的ファイル 304 の 6 件、22:29:58Z の worker・worklet の 304(Worker の版が c0bfade6 → e5b6adb8 に変わった後)だけ。**POST /__perf は 0 件**。first idle の最初のビーコンも、30 s ごとの周期のビーコンも届いていない(セッションが first idle に達したかは tail から分からない)。
- 認証なしの `POST /__perf` は `https://<CF-VALUE-REMOVED>/cdn-cgi/access/login/...` への 302(**実測**、curl)。
- **読み**: `beacon.ts` の `fetch(..., { credentials: 'omit' })` が Access の Cookie(CF_Authorization)を付けないため、Access が 302 を返し、`connect-src 'self'` の CSP と fetch がそのリダイレクトを拒否する。ページはビーコンに依存しないので何も壊れて見えない。Worker は Access の ID・IP・Cookie を保存しない(既存の `worker.test.ts`)。差分は `diffs/perf-credentials.diff`(PM が a61760ba で取り込み済み。vitest 89/89、mutation 検出)。**配備後に届くことは未確認**(依頼者の新しいセッションが来ていない)。

### 1.2 読めた依頼者の機械の値(2026-11-03 18:21Z〜11-04 00:07Z。旧経路、`lift/out-websrv/perf-log.jsonl` の remote=tailnet の 125 件 = 20 セッション)

| 指標 | 値(20 セッション。表は `tables/client-beacon-sessions.tsv`) |
|---|---|
| first idle(P1 の根拠) | 5.7〜13.3 s と 114.2 s が 1 件。中央値 10.85 s。10 s 以内は 7/20 |
| エンジン idle/s(ゲストの周期) | 中央値 22.9(10.7〜23.8、目標 60。first idle だけの 3 セッションを除く)= ゲストは約 43 ms 周期 |
| イベント→rAF p50 | 中央値 29.8 ms。200 ms 超が 5 件(485〜14,006 ms) |
| p95 が 500 ms 以上 | 6/17(1,000 ms 以上が 5) |
| 終わり方 | `stopped/134` 12、`stopped/-1` 3(presents 0)、その他は続行中 |
| rAF | 239.5 Hz(240 Hz の画面)、ページの canvas 描画 p50 0.08〜2.9 ms、transit p95 0.2〜23 ms |
| 音(7 セッションに detail。`tables/client-beacon-audio.tsv`) | ctx running・44.1 kHz・outputLatency 22〜32 ms、underrunQuantaSinceFirstIdle は 0〜10,003、engineGapFrames 最大 1,283,528 |

足りない指標のうち、**現在の経路の提示 ms・提示/s・Worker の忙しさ・仮想時計/壁時計**を、ページがすでに持つ値から読み取り専用でビーコンに足した(`gpu1-gl.diff`、§6): `desk`(提示の方式 `gl`/`2d`、Worker のカウンタ〔presents・rects・mpx・composeMs・putMs・drawMs・uploads・draws・whole・ticks・idleTicks・wakes・sleeps〕の累計と**前の送信分からの差**)、`engineRun`(直近 10 s のエンジンの idle 報告から busyFraction・busyMsMedian・busyMsP95・virtualPerWall)。ビーコンは今 4,963 B(中央値)・最大 8,310 B で、足した分は約 0.6 KB(上限 15.8 KB)。窓移動中の『位置が変わる提示の回数/s』はこのカウンタの `since.presents / since.seconds` から読む(移動中の 30 s 窓で近似。移動だけを切り出す指標は足していない)。

## 2. ウインドウ移動の内訳(実エンジン、Chrome 156 / Firefox 158)

操作: ゲストの Mixer のヘッダー(空いた所)を左ボタンで押し、(x,y)↔(x+300,y)(3240 は +500)を 20 歩ずつ往復、約 60 移動/s(ドライバーの限界)、8 s。ゲストの画面は `?screen=` で 1920x1080 / 3240x1946。FL の窓: Mixer = `TFXForm`(lid 64、`TWPControl` lid 16 の子)。表: `tables/window-move-measures.tsv`(series `move3` が最終の A 対 F。`move`/`move2` は先の系列でスレッド名の分類が違う〔`Worker` を含む全スレッド〕ので参考)。CPU は `/proc/<pid>/task/*/stat` の `DedicatedWorker`(Chrome)/`DOM Worker`(Firefox)スレッドの CPU 秒。**load が 6〜20 なので壁時計の速さではなく CPU ミリ秒**。

| 構成(現行 2D、バンドル A) | 移動/s | 提示/移動 | Mpx/提示 | Worker CPU ms/移動 | compose ms/提示 | put ms/提示 | (compose+put)x提示/移動 ÷ Worker | Worker の忙しさ |
|---|---|---|---|---|---|---|---|---|
| Chrome 1920x1080 | 60.2 | 1.00 | 0.68 | 8.69 | 1.13 | 0.59 | 20% | 0.52 |
| Chrome 3240x1946 | 60.1 | 1.01 | 1.27 | 10.50 | 1.81 | 0.91 | 26% | 0.63 |
| Firefox 1920x1080 | 67.3 | 1.00 | 0.68 | 12.15 | 1.79 | 2.26 | 33% | 0.82 |
| Firefox 3240x1946 | 87.8 | 0.86 | 1.27 | 10.95 | 3.60 | 2.28 | 46% | 0.96 |

段ごとの読み(1 ステップ = 1 つの入力イベント):

1. **ページの入力 → ring → ゲスト**: 1 イベントに 1 メッセージ(`FLE_G3_TRACE` で数えた: mouse down 1 → WM_LBUTTONDOWN 1 + WM_MOUSELEAVE 1、その後 WM_MOUSEMOVE が入力 1 件ごとに 1 件。TFXForm がキャプチャ中)。WM_NCLBUTTONDOWN・SC_MOVE・WM_ENTERSIZEMOVE・WM_MOVING は**送られない**(FL が HTCAPTION を返さず自分で動かす)。ページは遅いゲストに対して入力を畳む(バースト 307 件中 288 件が dropped/merged)。
2. **ゲストの実行**: 1 つの処理済みの移動ごとに `SetWindowPos(lid 64, 0x14)` 1 回(診断ビルド: 窓表 12 窓、`copied_px` 約 70 万、コピーの矩形 4)。16 ms(仮想時計)ごとに lid 64 の `BeginPaint`(0,0,1514,500)+メモリ DC からの `BitBlt`(757,000 画素)を約 4 回、lid 17(チャンネルラック)を 5 回(0.36 Mpx/ステップ)、主窓の細い帯を 17 回(`FLE_RASTER_TRACE`)。= **FL 自身の再描画**で、Windows でも同じ(DWM が動かすのはトップレベル窓だけ)。1 移動あたりのゲスト+コピーは Worker CPU から compose と put を引いて約 6〜8 ms。
3. **ランタイムの合成**: 窓表が変わらないのに**親(主窓)の面の画素が変わる**(子窓の移動は面の中のコピーと再描画)ので、dirty の矩形が大きい(0.7〜1.3 Mpx)。compose は BGRX→RGBA 変換+絵との比較+コピー。
4. **提示**: putImageData の dirty rect(1 提示 1 矩形)。

### 2.1 比較: Settings のタブ切替(7 タブ x 3 回、F10 を開いて 21 クリック。`tables/settings-tab-switch-measures.tsv`)

| 構成 | 提示/クリック | Mpx/クリック | compose ms/クリック | put ms/クリック | Worker CPU ms/クリック |
|---|---|---|---|---|---|
| A Chrome 1920 | 17.6 | 4.36 | 25.9 | 7.3 | 368 |
| A Chrome 3240 | 12.4 | 10.6 | 168.9 | 14.2 | 644 |
| A Firefox 1920 | 17.2 | 4.65 | 34.7 | 36.2 | 541 |
| A Firefox 3240 | 10.5 | 11.1 | 170.4 | 47.8 | 745 |

= 1 クリックが Worker の CPU **0.37〜0.74 s**。合成+提示は 9%(Chrome 1920)〜29%(Firefox 3240)で、ゲストの描画が支配(依頼者の『Settings の General をクリックすると落ちる』は別の停止で、ここでは再現しなかった〔決定 0225〕)。**Playlist の中ボタンドラッグ**は、既定の(空の)Playlist でも、参照プロジェクトを開いた Playlist(Chrome 1920、A と F)でも、パンが起きず(提示 1.4〜2/s、Worker CPU/移動 7.5〜8.0 ms、画面の差はツールバーの数値だけ)、**計測にならなかった**(正しい操作を見つけていない。ship5/anim1 の『更新/s』との照合は未了)。

## 3. 『Windows の契約どおりか』(移動の処理)

- Learn の移動のモーダルループ(`DefWindowProc` の `WM_SYSCOMMAND SC_MOVE`、`WM_NCLBUTTONDOWN HTCAPTION`)は**システム**が行う。ランタイムの `fle_win.c` には**その実装が無い**(`DefWindowProcW` は `WM_SYSCOMMAND` を扱わず、窓は非クライアント領域 0 〔`nc_zero`、`WM_NCHITTEST` は HTCLIENT/HTNOWHERE だけ〕)。実エンジンでこの経路に入る窓は無かった: FL の窓(Mixer・Channel rack・Playlist・Browser)は主窓の**子窓**で、FL が自分でキャプチャして `SetWindowPos` する(決定 0214 の観察と同じ)。トップレベルの窓(wizard・Settings・メニュー)の移動は未試験。
- 子窓の `SetWindowPos`(決定 0214)は Learn どおり『SWP_NOCOPYBITS なしなら有効な内容を新しい位置へコピー、動かしたことで現れた親・兄弟の部分は無効』。読んだ範囲(`unc_begin`/`unc_end`/`unc_copy_bits`)で、無効な部分を有効としてコピーする所は見つからなかった(`cp0` は自分と子孫の pending を引いてある。親の pending の子への伝播は GAP-U1)。連続した移動でも、診断ビルドの 118〜317 移動で親への無効化は移動量どおりの帯だった。**ただし**次の 2 件は欠陥だった(§5)。

## 4. ゴースト(軌跡)の調査

依頼者の絵(PM の読み: 左側のパネルの同じ像が少しずつずれて水平の帯状に重なる)の窓は Mixer(dB 目盛りとマスターの緑のメーター)。表 `tables/ghost-trials.tsv` の 10 系列 + native 2 件。

- **4 層の比較**(各時点): (i) Worker の OffscreenCanvas の `getImageData` の SHA、(ii) コンポジタの絵(peek)、(iii) 窓表から描き直した絵(dump。**最後**に取る: resync するため)、(iv) ブラウザのスクリーンショット(viewport = guest + 340 で 1:1 にして画素比較)。Chrome 156 と Firefox 158 で、移動中・移動後・遅いゲスト(`taskset` で 2 CPU)・バースト・参照プロジェクトのどれでも、(i)=(ii)、(iv)=(ii)(動いている部品を除く)、旧位置は親の絵で埋まる(目視)。遅いゲストでは移動の 1.2 s 後にまだ差(Firefox で最大 59,905 画素)が見えたが、3.7 s 後には消えた(= 遅いだけで残らない)。
- **依頼者の絵と我々の条件の差**(再現しなかった条件): 実 GPU(GTX 980、Firefox の GPU 合成)、DPR 1.7647、実マウスの高頻度入力、ゲストが約 24 idle/s、Firefox 159、依頼者のプロジェクトの内容。
- **読み(未検証)**: 『ドラッグが終わっても更新されない・ホバーした部品から順に更新』(決定 0214 の症状)は、窓管理の無効化の欠落か提示の取りこぼしで起きる。我々の窓管理は通常の移動列では無効化を正しく行う。**依頼者の環境固有の canvas の部分更新(Firefox の OffscreenCanvas 2D の putImageData の dirty rect、GPU 合成)は、Linux のソフトウェア描画では試せていない**。GL の合成器は dirty rect の putImageData を使わないので、もし原因がそこなら GL への切り替えで消える可能性がある(読み。確かめていない)。

## 5. 窓管理の欠陥 2 件(契約どおりに直した)と、止まる所 1 件

`fle_win.c` の `unc_begin`/`unc_end`(決定 0214 の『動かした子窓が現す領域の無効化』)は、(a) 触れる窓を表の順に `UNC_MAX`=160 件までしか snapshot に入れず、(b) 窓の可視領域が `FLE_VISRGN_MAX`=128 矩形を超えると `fle_win_dc_region` が -1 を返し、`unc_region` が『空』にして、`unc_end` がその窓を `continue` で飛ばしていた。どちらも**黙って捨てる**(32 欄の面の表と同型)。

- **試験**(`present2_uncover.rs` に最初の 2 件、native。期待値は Learn から): ① 親の可視領域を 128 矩形超にする(1 画素高の子 70 個を斜めに置く)と、HEAD の runtime では子を動かしても親が無効にならない(`parent 0 0 0 0 0`)、直した runtime では無効 ② 170 個の孫が動きに触れる状態で、後から作った兄弟が動かした窓の下にあると、HEAD では兄弟が無効にならない。直した runtime で 15/15 通過。HEAD の runtime では新 2 件が失敗(他の 13 件は HEAD の runtime では再実行していない)。
- **直し**(契約 (b)): snapshot が不完全(候補が 160 超、または領域が溢れた)なら、ビットのコピーをやめ、動かした窓の旧・新の矩形の和集合に触れる**全ての可視窓**をその部分で無効にする(Learn SetWindowPos: 『動かしたことで現れた親の部分は無効』の過剰な側 = 安全)。窓・クラス名での分岐は無い。過剰な無効化は再描画の費用だけ。
- **実エンジンでの確認**: 診断ビルドで Mixer の移動 118〜317 回、窓 12〜18、溢れ 0(参照プロジェクトを開いても同じ)。つまり**通常の FL ではこの 2 件は起きていなかった**。直した F のバンドルで画素同一と移動の測定は通った。
- **止まる所と、その直し**: `fle_gdi.c` は、DC の可視領域が 128 矩形を超えると `pixel_stop`(名前つきの停止)した。窓が多い場合に『落ちる』につながり得る。直し(契約 (b)): BeginPaint の DC では、更新矩形の内側だけの領域を `fle_win_dc_region_within`(`fle_win.c`、`fle_visrgn.h` に宣言)で作る — 集合として (A − B) ∩ R = (A ∩ R) − B なので、描く画素の集合は `fle_win_dc_region` を R に切った物と同じ。70 個のクリップされた子を持つ窓を小さい更新矩形の中で描ける試験(`present2_uncover.rs` に 1 件)。`fle_win_dc_region_within` が無い単体試験のリンクのために `fle_gdi.c` の宣言は weak。更新矩形そのものが 128 矩形を超える描画では `pixel_stop` が残る。`present2_uncover` は 16/16(新 3 件は HEAD の runtime では 3 件とも失敗)。

## 6. 差分の内容(`diffs/gpu1-uncover.diff` = `fle_win.c`・`fle_gdi.c`・`fle_visrgn.h`・`present2_uncover.rs`〔基準 3688a940、取り込み済み 1a6383d2〕、`diffs/gpu1-gl.diff` = 以降の行〔基準 1a6383d2〕)

| 変更 | 区分(忠実性の方針) | 理由・測定 |
|---|---|---|
| `fle_desk.h/.c`: `fle_desk_scene_mode`、`fle_desk_scene`(窓の並び・位置・面のアドレスと stride・レイヤー・ガラス余白〔`FLE_SCENE_GLASS`、`gx0..gy1`〕・窓領域の箱〔`cl..cb`〕・書かれた矩形。version 2。窓ごとの影のコピーで同じ画素の書き直しを報告しない)。`fle_desk_wrote` は scene モードで矩形を集めるだけ。`build_desc`(alpha1 の glass/region の説明文つき)と `trim_row`(2D の `paint()` と scene の影が共有する『違う所だけ報告』の 1 つの規則)に切り出し。alpha1 の `draw_rect`・`over_premul` には触れていない | **(c) 変換・通信の削減**: wasm 内の BGRX→RGBA 変換と全面の比較をやめ、面を直接 GPU に渡す。窓の Z 順・位置・可視・レイヤーの『判断』は C に残る(窓表を読むだけ) | native 単体 `desk_scene.rs`(2000 ランダムステップで『ブラウザの鏡』が全窓の面と一致、移動・再積みで画素 0、同じ画素の書き直しで更新 0、ガラス〔シート・余白・窓より大きい余白〕と窓領域〔箱・窓外・反転〕の記述と『画素を送らない』を検査。`fle_desk.c` に手で入れた 4 つの欠陥〔`cl` を 0 に、ガラスの旗を落とす、`gx1` を `gx0` に、`same_win` から領域の比較を外す〕を全部検出)。`desk1_compose.rs` 2/2 通過 |
| `fle_win.c`・`fle_gdi.c`・`fle_visrgn.h`: `unc_*` の不完全な snapshot の扱いと BeginPaint の DC の領域(§5)= `gpu1-uncover.diff` | **(b) 契約の実装**(過剰な無効化、更新矩形の内側だけの領域) | `present2_uncover.rs` 16/16(新 3 件) |
| `desk-gl.ts`(新): scene を読み、窓ごとの texture に texSubImage2D(wasm メモリから、`UNPACK_ROW_LENGTH` = stride/4)、quad を順に描く。窓は 4 モード(不透明、constant alpha / colour key、per-pixel alpha、ガラス〔余白の外は事前乗算 ARGB を下の上に、内側の箱は不透明〕)と、窓領域の箱の外を `discard` | **(c)**。式は alpha1 の `draw_rect`/`over_premul` と同じ。窓の知識・クラス名・位置の判断なし | vitest 16 件(GL の呼び出し列を記録する偽 GL。コンテキスト喪失・復帰、`webgl2Usable`、ガラスと領域の uniform を含む)+ mutation 10 件を検出 |
| `worker.ts`: `makeDeskPresenter`(能力検出 1 か所)、GL のコンテキスト喪失・復帰、読み戻し・peek。`desk-gl.ts`: `webgl2Usable` | 能力検出(no-options §1 の『環境の能力検出による自動の別経路』) | 条件(PM 指定。決定論的): ① `fle_desk_scene` が wasm にある、② 捨て用の OffscreenCanvas(1x1)が `getContext('webgl2', { failIfMajorPerformanceCaveat: true })` で null でないコンテキストを返し、その `MAX_TEXTURE_SIZE` が 8192 以上(窓の面は texture。ゲストの画面は 8K〔7680 幅〕まで)。満たさなければ従来の 2D。デスクトップの canvas は最初のコンテキストの種類で固定されるので、判定は捨て用の canvas で先に行う。URL・設定では選べない。理由と答えは §7.3 |
| `desk.ts`/`fruity.ts`/`protocol.ts`/`client.ts`/`main.ts`: scene の API、`desk-gl-read`(試験用の読み戻し)、`deskGlPixelsBase64`(試験用) | 検査用の関数(no-options 2.1-9: 振る舞いを変えない) | |
| `beacon.ts`/`main.ts`: `desk`・`engineRun` ブロック | 読み取り専用の測定 | vitest 7 件 + mutation 3 件を検出。ビーコンは v3 のまま(任意ブロック) |
| 試験: `desk_scene.rs`、`desk-gl.test.ts`(16 件)、`perf-beacon-desk.test.ts`(7 件)、`desk-gl-presenter.mjs`(実ブラウザで presenter を直接駆動。run.mjs の外。CI には入れない)、`run.mjs` の mutation 13 件(GL 10、ビーコン 3) | | |

'-' 行は `gpu1-gl.diff` が 22 本・`gpu1-uncover.diff` が 7 本、全部読んだ。`gpu1-gl.diff`: `fle_desk_wrote` の宣言の分割 1、`paint()` の trim の 5 行を `trim_row` 呼び出しへ、`fle_desk_compose` の見出しと `build_desc` へ移した説明文の組み立て(alpha1 の glass/region の行を含め、そのまま移動。内容は同じ)、`worker.ts` の import・変数の型・構築・案内文の 4 行、`client.ts` の case 1 行、`main.ts` の import 1 行。新しいファイル(`gpu1-gl.diff` に 5 つ: `desk_scene.rs`・`desk-gl.ts`・`desk-gl.test.ts`・`perf-beacon-desk.test.ts`・`desk-gl-presenter.mjs`)は git に入る(`.gitignore` に当たらない。差分を当てた `git archive` の木で `git check-ignore`、`logs/ignore-check-gl.txt`)。両方の差分の dry-run は新しい `git archive HEAD` の木で OK(`logs/patch-dry-run-gl.txt`・`logs/patch-dry-run.txt`)。

## 7. WebGL2 の合成器の測定(ソフトウェア GL。実 GPU なし)

### 7.1 画素同一(`tables/gl-identity.tsv`、F バンドル + HEAD のページ。Chrome と Firefox)

GL の読み戻し(scene を texture から描き直して readPixels)と CPU コンポジタの絵(窓表から全描き直し)を、wizard・主窓・File メニュー・ドラッグ後・戻した後・Settings 窓で比べた: 差があるのは動いている部品(ツールバーのメーター `[1037,6]-[1121,38]` とその周辺)の 13〜490 画素(最大チャンネル差 154)だけで、それ以外は 0。ブラウザのスクリーンショット(1:1)と GL の読み戻しの差は 0(Chrome)・0〜13(Firefox のメーター)。決定 0243 の取り込み後のページ(HEAD 33349a3d)でも同じ(`engine.desk`・`hostFiles` を manifest から外して起動)。
最終版(version 2)の画素一致: `app/test/e2e/desk-gl-presenter.mjs`(`desk-gl.ts` を esbuild でまとめて本物の Chrome の WebGL2 〔SwiftShader〕で `GlDeskPresenter` を直接駆動。窓 7 つ〔不透明・constant alpha 128・colour key・per-pixel alpha・ガラス〔余白あり〕・ガラス〔シート〕+ 窓領域・constant alpha + 窓領域〕、strided の面、移動・再積み・書かれた矩形・窓を隠す・ガラスの内側の箱と窓領域の変更の 8 段階)で、JS の参照〔alpha1 の `draw_rect`/`over_premul` と div255 の丸め〕との差は**全段階で 0 画素**(`tables/gl-presenter-v2.json`)。ガラスのモードを 0 に、窓領域の `discard` を消す、の 2 つの欠陥を手で入れると 411〜1,869 画素の差が出て検出する。移動・再積み・ガラスの内側の箱/窓領域の変更は texture のアップロード 0。

層状窓: `tools/gl-layer-test.mjs`(本物の WebGL2 文脈で、32x32 の窓を非平坦な背景に重ね、CPU の式〔`fle_desk.c draw_rect`〕の JS 版と画素比較): constant alpha(255,254,200,128,100,77,1,0)・colour key・両方・per-pixel alpha(SCA 255,200,128,33)の **16 例・16,384 画素が完全一致**(SwiftShader)。実 GPU のブレンドの丸めは未確認。

### 7.2 性能(`tables/window-move-measures.tsv` の series `move3`。A = 現行 2D〔HEAD の ciA + HEAD のページ〕、F = scene+影 trim〔ciF + srcG のページ〕。比較はコミット同士)

| 構成 | 提示/s(A→F) | Worker CPU ms/移動(A→F) | compose+put+draw ms/提示(A→F) | 備考 |
|---|---|---|---|---|
| Chrome 1920 | 60.2 → 54.3 | 8.69 → 9.88 | 1.71 → 0.96 | Worker の 1 移動の CPU は増えた(GL の提出で約 2 ms 増え、読みでは SwiftShader 向けの CPU 経路) |
| Chrome 3240 | 60.8 → 26.7 | 10.50 → 7.88 | 2.72 → 4.00 | SwiftShader が律速(提示/s が半減)。影のコピーの比較+コピーが O(面積) |
| Firefox 1920 | 67.4 → 75.6 | 12.15 → 10.01 | 4.05 → 4.97(draw 0.51 含む) | |
| Firefox 3240 | 75.4 → 49.1 | 10.95 → 8.89 | 5.88 → 8.35(draw 0.69 含む) | Firefox の texSubImage2D は 5.5 ms/提示 |

- **アイドル**: 影の trim で提示は 0.7〜1.0/s(A と同じ)。trim なしの E では 36〜59/s(アイドルの再描画を毎回アップロードし、Worker CPU が 0.83→2.63 s/6 s)だったので、trim は必須だった(S-gpu1-4)。
- **読み**: 案 G の構造上の利点(トップレベル窓の移動で画素 0 アップロード)は FL の Mixer には当てはまらない — Mixer は主窓の子窓で、動かすと**主窓の面の画素が変わる**ので dirty rect の texSubImage2D(0.7〜1.3 Mpx = 2.7〜5 MB)は要る。GL が減らせるのは compose の変換・比較(1.1〜3.6 ms)と putImageData の CPU(0.6〜2.3 ms)であり、実 GPU ではアップロード(メモリコピー)と quad の描画だけになるはず。**この機械ではソフトウェア GL が律速で、速くなるとは言えない。実 GPU での値は未測定**。
- **最悪ケース(全画面が毎フレーム変わる。プラグインピッカーのフェード)**は FL の既定の画面では作れず未測定。Settings のタブ切替(0.6 Mpx 級の窓の全面更新)を代表にした: F の Worker CPU/クリックは 293〜659 ms(A は 368〜745)、Mpx/クリックは A の 4.4〜11 に対し F は 1.8〜7.5(影の trim が同じ画素の書き直しを落とす)。

### 7.3 採用条件に対するブラウザの答え(`failIfMajorPerformanceCaveat`。この機械)

| ブラウザ | `getContext('webgl2', { failIfMajorPerformanceCaveat: true })` | 旗なし | `MAX_TEXTURE_SIZE` |
|---|---|---|---|
| Chrome 156 headless(`desk-gl-presenter.mjs` の `usableByThePlatformRule`) | **取れる**(レンダラ ANGLE / Vulkan / SwiftShader) | 取れる | 条件を満たす(試験が通る) |
| Firefox 158 headless(`tools/ff-caveat.mjs`、`logs/firefox-caveat.json`) | **取れる**(レンダラ `llvmpipe, or similar`、Mesa) | 取れる | 16384 |

- **実測**: 両方とも、ソフトウェアのレンダラなのに、旗つきでも WebGL2 を返す。この条件は『ソフトウェア GL なら 2D』という PM の理由(3240x1946 で SwiftShader の提示が半減した)を、この機械のブラウザでは**満たさない**。したがって製品のページをこの機械の headless ブラウザで開くと GL になる(§7.2 の GL の測定もそれと同じ経路)。
- **読み(未確認)**: ソフトウェアのレンダラを『major performance caveat』と数えるかはブラウザとその起動のしかた次第で、この 2 つの headless は数えていない。Windows の実機(GTX 980)は GPU 経由の ANGLE になり、旗つきでも取れると読む。**実機の答えは未確認**。`desk-gl.ts` の旧い説明文は『旗つきならソフトウェアは null』と書いていたが、この測定と食い違うので書き直した(`gpu1-gl.diff` はその版)。
- 足すなら: レンダラ名(`WEBGL_debug_renderer_info`)の SwiftShader / llvmpipe / Microsoft Basic Render Driver の検査。ブラウザが隠す(Firefox の `resistFingerprinting` は一般的な文字列を返す)ことがあり、実機の文字列を確かめられないので私は足していない(PM が決める)。
- コンテキスト喪失の復帰はブラウザでは確かめられず(`Browser.crashGpuProcess` の後で CDP が応答しない)、単体試験のみ。

### 7.4 最終の木(HEAD 1a6383d2 + gpu1-gl)の実エンジン — 既定の入口での確認と、GL 対 2D(コミット同士)

**範囲**: 実エンジン(F2xH7)を CI の段スクリプトで冷間ビルドした 2 つの木: **B** = HEAD 1a6383d2 + `gpu1-gl.diff`(wasm 133,932,785 B、bundle `fl-engine-ci-F2xH7-db478af3`)、**A2** = `git archive HEAD` の 1a6383d2(2D だけ、wasm 133,929,418 B。B の objects を借りた増分ビルド = runtime の objects だけ作り直し、44 単位は全部再利用)。ページも別ビルド(`app-distA` = HEAD、`app-distB` = HEAD + gl)。つまり『2D』は HEAD のコミット、『GL』は HEAD + gpu1-gl のコミット(wasm の差は +3,367 B)。隔離 headless Chrome 156 と Firefox 158(`tools/sdriveg.mjs`)、**クエリなしの既定の入口**(`http://127.0.0.1:PORT/`)、模擬入力、`vite preview` のローカル配信(依頼者の回線ではない)。この機械(実 GPU なし: Chrome は SwiftShader、Firefox は llvmpipe)、load 13〜30 で他担当と共有。**依頼者の機械・実 GPU の値は 0。**

**確認した事実(`tables/final-runs/fin-all-*.json`、`fin-id-chrome.json`、`fin-menu*.json`)**:

| 項目 | Chrome 156 | Firefox 158 |
|---|---|---|
| first idle に達する | 達する(6.5〜8.5 s) | 達する(7.95 s) |
| GL が選ばれる | はい(`glcmp` が GL の読み戻しを返す。測定に `gl` ブロックがある) | はい |
| 窓が出る | wizard → 主窓、File メニュー(`TQuickPopupMenuWindow`、glass)、F8 の `TPlugListForm`(glass、1920x1166 の半透明の暗い面で下の主窓が透ける)、F10 の `TMIDIForm` | 同じ |
| メニューの開閉・F8 の開閉 | 開く窓が表に出て、閉じると主窓だけに戻る(`describe`) | 同じ |
| Mixer の移動 | 移動後の GL 対 CPU の差 0(`f3-after-drag`) | 差 0 |
| GL の読み戻し対 CPU コンポジタの絵(wizard・主窓・メニュー・移動後・ピッカー・設定窓の 8 時点) | 差があるのは画面上部の動く部分だけ: メーター(13〜65 画素、最大差 154)、wizard・メニュー・設定窓が開いている間の上部(wizard 395 画素〔最大差 11〕、メニュー 103、設定窓 119 画素)。移動後は 0。ピッカーは 6 画素(最大差 1)で、半透明の glass が CPU と一致。上部の差が『動く部品』であることは、メニューの状態だけ確かめた(下)。wizard と設定窓の差は読み | メーターの 13/65 画素だけ。残りの時点は 0 |
| ブラウザのスクリーンショット対 GL の絵 | 主窓・ピッカー・移動後は 0。wizard・メニュー・設定窓は上部の動く部分の差(296〜575 画素。読み) | Firefox は 1 時点(65 画素のメーター)以外 0 |

- **右上の ±1(Chrome のメニュー開時)**: GL と CPU の差ではなく、**面そのものが 100 ms 内に ±1 変わる動き**(メニューが開いている間の主窓の右上のアニメ)。同じ状態で CPU の dump を 100 ms 空けて 2 回取ると、同じ範囲(1854〜1920 x 11〜40)が ±1 変わる(`tools/scn-fin-menu4.json`)。また HEAD の 2D のページでも peek 対 dump に右端の 13 画素の差がある(`fin-menu2d-chrome`)。GL が古い絵を持っているのではない(resync 後の次の比較では 0、`fin-menu3-chrome`)。
- **first idle**: 27 回(この機械、localhost、load 13〜30)で最小 5.1 s・中央値 7.3 s・最大 11.0 s(11.0 s は 1 回、A のページ + B の bundle の Chrome 1920)。依頼者の回線の値ではない。

**GL 対 2D(Mixer のヘッダードラッグ、各セル n=2。`tables/gl-vs-2d-final.tsv`、`tables/final-runs/fin-move-*.json`)**:

| ブラウザ・画面 | 2D の moves/s | 2D の presents/s | 2D の Worker CPU ms/移動 | 2D の Worker busy | GL の moves/s | GL の presents/s | GL の Worker CPU ms/移動 | GL の Worker busy | Worker CPU/移動の変化(平均) |
|---|---|---|---|---|---|---|---|---|---|
| Chrome 1920x1080 | 59.9/60.2 | 60.4/60.4 | 9.46/9.36 | 0.57/0.56 | 59.9/59.9 | 31.3/43.4 | 7.67/8.71 | 0.46/0.52 | **−13%** |
| Chrome 3240x1946 | 60.1/59.9 | 60.6/60.8 | 13.26/11.58 | 0.80/0.69 | 59.6/59.9 | 23.2/20.6 | 5.63/5.77 | 0.34/0.35 | **−54%** |
| Firefox 1920x1080 | 103.1/117.8 | 101.7/114.3 | 10.0/8.49 | 1.03/1.00 | 74.5/110 | 64.3/96.2 | 11.54/8.54 | 0.86/0.94 | +9%(ばらつき内) |
| Firefox 3240x1946 | 114.6/108.4 | 102.6/101.0 | 8.89/9.9 | 1.02/1.07 | 75.2/74.7 | 38.3/41.1 | 9.04/9.48 | 0.68/0.71 | −1% |

- **読み**: この機械ではソフトウェア GL なので『フレームレートが上がる』とは言えない。**Chrome では Worker の CPU/移動が下がる(−13%、3240 では −54%)が、提示された絵の数/s は 2D の 60 から 21〜43 に下がる**(Worker がソフトウェアの GL の提出を待つ。put〔upload〕は 3240 で 3.5〜7.2 ms/提示、2D は 1.1〜1.6 ms)。**Firefox では Worker の CPU/移動は変わらず、3240 で提示/s が 2D の約 100 から GL の約 40 に下がる**。つまりこの機械では GL は 2D より遅いか同等で、**実 GPU でどうなるかは測れていない**(実 GPU なら upload とソフトウェアの quad 描画の待ちが消えるはずだが、読みである)。Firefox の moves/s は WebDriver の速さで 75〜118 とばらつくので、Firefox の CPU/移動の比較は粗い(n=2、驚くほどの差ではない)。
- GL を 2D との比較で『速い』と言える根拠は無いので、**依頼者の GTX 980 での計測(配備後のビーコンの `desk`・`engineRun`)が必要**。

## 8. テスト基準

- 生成 C は不変(増分ビルドで『44 単位は全部再利用、0 単位は object なし』= 単位の C バイト列は ciA と同一。lifter・fe-wasmgen は触っていない)。
- `fe-lift` の native(最終の木 = HEAD 1a6383d2 + gpu1-gl): `present2_uncover` 16/16(新 3 件を含む。新 3 件は HEAD の runtime では失敗)、`desk1_compose` 2/2、`desk_scene` 1/1(2,549 checks、手で入れた 4 つの欠陥を検出)。**`cargo test -p fe-lift` 全体(最終の木): 151 results / 2,127 passed / 0 failed**(`logs/cargo-fe-lift-summary.txt`。古い木〔HEAD 33349a3d + 旧 gpu1.diff〕では 150 results / 2,118 passed / 0 failed)。PM の全体の基準(workspace 全体)とは数え方が違う(fe-lift パッケージだけ)。PM の全体の基準(152 results・2,118 passed)は workspace 全体の数で、私は fe-lift パッケージだけを走らせたので 1 対 1 では比べられない(他のクレートは触っていない)。a4 FRUITY 無効の基準(942/942・895/895・functions 5,310・entries)は lift/ の crates・harness を触っていないので再実行していない(未確認)。
- app(最終の木 = HEAD 1a6383d2 + gpu1-gl): vitest 全体 **1,251 passed / 7 skipped / 0 failed**(64 files + 1 skipped、`logs/vitest-full-final.log`)。mutation(`APP_WORK=/tmp/opencode/gpu1/appwork`): 新 13 件のうち実行した GL 10 件・ビーコン 3 件と、HEAD の既存 2 件(credentials、start-up-in-ms)の **15/15 検出**・復元後の差 0(`logs/mutation-gpu1-final.log`)。tsc 4 本(`npm run typecheck`)通過。
- 16 スナップ(84 行)は未実行(node の G3 は NO_FS=0 の別構成が要る)。runtime の変更は窓の snapshot が不完全なときと scene モードのときだけ振る舞いが変わる(通常の起動では同一と読む。未確認)。

## 9. 出典のない選択(S-gpu1-N)

| 番号 | 選択 | 理由 | 代替 |
|---|---|---|---|
| S-gpu1-1 | WebGL2 の文脈属性: alpha:false、antialias:false、depth/stencil なし、preserveDrawingBuffer:false、powerPreference 'high-performance' | 不透明なデスクトップの 1:1 表示。毎提示で全窓を描き直すので保存不要 | alpha:true(ページの背景と合成: 不要)、preserveDrawingBuffer:true(メモリ・速度) |
| S-gpu1-2 | texture は NEAREST、窓は 1:1 の quad | CPU 合成と同じ画素。拡縮はブラウザが canvas で行う | LINEAR(滲む) |
| S-gpu1-3 | 1 提示の書かれた矩形は『窓ごとに外接矩形 1 つ』 | 実装が小さい。Mixer の移動は 1 矩形、アイドルは trim で小さい | 窓ごとに複数の矩形(アップロードが減る場合がある) |
| S-gpu1-4 | 窓ごとの影のコピー(メモリ = 面の合計)で同じ画素の書き直しを報告しない | CPU 経路の trim(絵との比較)と同じ働き。無いと アイドルの提示が 36〜59/s。大きい矩形では比較+コピーが O(面積)(3240 で 2.2 ms) | タイルのハッシュ(確率的に誤る)、大きい矩形では影を捨て後で全面再コピー、trim なし |
| S-gpu1-5 | scene の上限 64 窓・64 更新 | 既存の `FLE_DESK_MAX_WINDOWS`=64 と同じ | 動的 |
| S-gpu1-6 | 不完全な snapshot は『和集合 ∩ 各可視窓を無効、ビットはコピーしない』 | Learn の過剰側。窓・領域の容量を動的にするより小さい | `FLE_VISRGN_MAX`/`UNC_MAX` を動的にする(`fle_visrgn` を値渡ししている所が多く大きい変更) |
| S-gpu1-7 | GL の採用条件は『`failIfMajorPerformanceCaveat: true` で WebGL2 の文脈が取れ、最大 texture が 8192 以上』(PM の指示。S-gpu1-12) | ソフトウェア GL では遅い(3240 で提示が半減)ので、PM は『ソフトウェアなら 2D』を意図した。**ただしこの機械の Chrome・Firefox は旗つきでもソフトウェアの GL を返す(§7.3)** | レンダラ名の検査を足す(実機の文字列が未確認)、旗なし |
| S-gpu1-8 | ビーコンの `engineRun` は直近 10 s、`desk.since` は前に送ったビーコンとの差 | ビーコンは 30 s 周期。移動中の窓は 30 s に近似 | 移動中だけを切り出す |
| S-gpu1-9 | 測定の CPU は Worker スレッド名(`DedicatedWorker`/`DOM Worker`)だけ、秒は /proc の tick(10 ms 精度) | スレッドプールのワーカーを混ぜない | 全スレッド |
| S-gpu1-10 | GL の『peek』= scene を描き直して readPixels | 検査用。GL には『最後に読んだ絵』が無い | peek 非対応 |
| S-gpu1-12 | GL の採用条件に『`MAX_TEXTURE_SIZE` ≥ 8192』を足した(捨て用の canvas で先に判定) | 窓の面は texture で、8K の画面 + 主窓の余白を収める。WebGL2 の最小保証は 2048 で、足りない GPU で黒くなるのを避ける(2D には後から戻れない) | 閾値なし(小さい GPU で壊れる)、4096(4K 幅 3840 + 余白は入るが 8K は入らない) |
| S-gpu1-13 | `webgl2Usable` の捨て用の文脈は `WEBGL_lose_context.loseContext()` で明示的に手放す | ブラウザの WebGL の文脈数の上限(Chrome は 16 前後と読む)を判定のために食わない。エラーが出ても false を返す | 参照が消えるのを GC に任せる |
| S-gpu1-11 | 診断ビルド(GPU1_DIAG: unc の窓数・溢れ・無効化を stderr に出す)は差分に入れない | 方針(診断は製品に入れない) | 製品に入れる(不可) |

## 10. 方針の点検(lift の忠実性・no-options・依頼者への接し方)と、規則違反の自己申告

- 余計な処理の追加: scene の C は窓表・面・レイヤー状態を**読む**だけで、窓の挙動を決めない。JS は窓を知らない(`grep` で `T[A-Z]` のクラス名・`TFruity`・`TQuick` は差分に 0)。新しい処理は上表のとおり (b) 契約の実装、(c) 変換・通信の削減、測定のどれか。
- オプション: `?present=GL` は作っていない。能力検出は `makeDeskPresenter` の 1 か所。診断の環境変数・旗は差分に入れていない。試験・計測の道具(`tools/`)は製品のコードの外。
- **自己申告**: ① app の mutation ランナーの作業コピーの置き場は既定で共有(`/tmp/opencode/app1`)で、私の mutation 検査 3 回は APP_WORK を指定せずに走らせた。最後の 1 回が別の担当の mutation と重なった形跡があり(PM に連絡済み)、その後は `APP_WORK=/tmp/opencode/gpu1/appwork` で再実行した。② 私のブラウザ試験のポート(preview 4985〜4995、geckodriver 4833〜4990 のうち乱数で選んだ物、r2read 8799)は他担当と重ならないよう確認したが、geckodriver の乱数ポートは検査していない。③ `wrangler tail` を開いたままにして依頼者の要求の行を読んだ(IP 等を含む生ログは集計後に削除)。④ 作業ディレクトリ `lift/out-gpu1` が作業中の上限(8 GiB)を超えた(約 9.3 GB。`noreturn_audit` が『Disk quota exceeded』で 1 回失敗した)。古い木・cargo の成果物を消して復旧し、最後に 500 MB 以下へ片づけた。⑤ Firefox の確認(`tools/ff-caveat.mjs`)の geckodriver のポートは `freePort()` が選んだ空きポートで、他担当の予約ポートとは照合していない。
- 依頼者に操作・資格情報・決定を求める文面は無い。CI に実ブラウザは入れていない。第三者のサービスは使っていない(`wrangler`/`curl` は依頼者のアカウントの既存のログイン・公開 URL)。依頼者の IP・メールは tail の生ログに含まれていたため、集計後に生ログを削除した。

## 11. 再現手順

`tools/` に道具一式(`README.md`)。要点: `bash tools/run-ci-build.sh <git archive HEAD の木> <ciwork> F2xH7` で冷間ビルド、`bash tools/mk-inc.sh NAME TREE` で runtime だけ作り直す増分ビルド、`tools/preview.sh NAME PORT [DIST]` で `vite preview`、`node tools/sdriveg.mjs --browser chrome|firefox --url ... --scenarios tools/scn-*.json --viewport WxH`(`FF_PREFS` に JSON を渡せば Firefox の prefs)、`node tools/gl-layer-test.mjs`、`python3 tools/beacon-tables.py`、`python3 tools/mk-tables.py`、`bash tools/mkdiff.sh`(uncover)、`bash tools/mkdiff-gl.sh`(GL。`srcN3` を既定とする)、`node tools/ff-caveat.mjs APP_DIR WORK_DIR`、`node app/test/e2e/desk-gl-presenter.mjs OUT`(`desk-gl.ts` の presenter を実ブラウザで直接駆動)。

## PM の判断と取り込み(2026-11-05)

取り込んだ基準: HEAD 1a6383d2 に `gpu1-uncover.diff`(取り込み済み 1a6383d2)と `gpu1-gl.diff`(`patch -p1`、offset・fuzz・reject なし、`-` 行 22 本は全部読んだ)。PM が測った値(この機械): FLEngine の既定の生成 C の sha256 `cd9107ce…` 不変、typecheck rc=0、vitest 1,251 passed + 7 skipped、worker 49/49、e2e(エンジン無し)172 checks、全体試験 **198 results / 2,398 passed / 0 failed / 1 ignored**、a4 FRUITY 無効 942/942・895/895・5,310・1,009,737。

1. **GL 合成器を唯一の経路として採る(オプションなし)。** 採用条件は能力検出の 1 か所(`webgl2Usable`): wasm に `fle_desk_scene` がある + `failIfMajorPerformanceCaveat: true` の WebGL2 が取れる + `MAX_TEXTURE_SIZE ≥ 8192`。そうでなければ 2D。**レンダラ名による振り分けは足さない**(プラットフォーム自身の能力の答えに従う。この機械の headless は常に GL になる。ソフトウェア GL の測定は参考値)。
2. **性能は実 GPU が無く未証明。** 依頼者の GTX 980 / Firefox 159 の効果は、ビーコンの `desk` ブロック(gl/2d、Worker のカウンタ)で読む。GL が遅いと分かった場合は、方式を直す(オプションで逃げない)。
3. **ゴーストは再現しなかった**(10 系列)。窓管理の『黙って捨てる』欠陥 2 件は直した(1a6383d2)。依頼者のゴーストの原因とは言えない。
4. **push は、最終の木の実ブラウザでの確認(first idle、GL が選ばれる、窓、メニュー、Mixer の移動、ピッカー)の後。** GL が壊れると依頼者の画面が出ないため。

4'. **最終の木(HEAD 1a6383d2 + gpu1-gl)の実エンジンでの確認(gpu1 の追報、2026-11-05 02:40Z)**: Chrome 156 / Firefox 158 の既定の入口で first idle に達し(27 回、最小 5.1 s・中央値 7.3 s・最大 11.0 s)、GL が選ばれ、wizard・主窓・File メニュー・F8 のピッカー(半透明)・F10・Mixer の移動が動く。GL 読み戻し対 CPU コンポジタは動く部品以外 0 画素差。**この機械(ソフトウェア GL)では GL は 2D より速くない**(Chrome は Worker CPU/移動が −13%〜−54% だが提示/s が 60 から 21〜43 に、Firefox は CPU/移動が同等で 3240 の提示/s が約 100 から約 40)。壊れてはいない。alpha2(0245)との相互作用(影のコピーと『同一内容の書き直しは通知しない』の二重の trim)は測っていない。実 GPU の効果は配備後のビーコン(desk・engineRun)で読む。
