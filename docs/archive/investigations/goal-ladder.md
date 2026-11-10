# ゴールからの到達段階表（ladder）: 「ブラウザ上で FL Studio が動作する」までの距離

> **2026-11-07 01:23 の更新（決定 0314。PM が取り込み）**: この文書の先頭の節「2026-11-07 の状況欄」が現行（2026-11-07 時点、基準 HEAD f0e5a2f5）。2026-11-04 の版以前の本文は履歴として残してあり、古い記述の訂正の一覧（旧 → 新、出典）は `decisions/0314-milestones-refresh.md`。最新の測定は `milestones/performance-targets.md` §4e の最新の段落。01:06 の注意（本文が古いという警告）は、この更新で解消した。

作成: 2026-10-30（基準 HEAD c76a7c3）。**調査のみ（読み取りと静的分析）。何も決めない**（推奨は書く。決めるのは PM）。実行・ビルドはしていない（cargo／emcc／clang の本ビルドなし。capstone と pefile による静的分析と、既存文書・既存ログの読み）。補助物は `docs/investigations/goal-ladder/`。

> **性能の条件（2026-11-03 追加。依頼者の指示）**: ゴール（G11）には実用的な性能が含まれる。起動 10 秒以内（P1）、所定のプロジェクト `serina1-serum2.zip` で UI 120 fps・遅延ほぼなし（P2）、Song Mode の再生で音が途切れない（P3）。数値の条件を満たすまで、G6（操作感）・G7（実時間）・G8（プロジェクト）を「達成」と数えない。詳細・基準値・計画は [`../milestones/performance-targets.md`](../milestones/performance-targets.md)。
>
> **観測の定義（恒久。2026-11-03 追加）**: この表の「到達」は、lift した実エンジンが実際に実行されて出した結果の観測だけで数える。代役（`engine.kind: surface-table-v1`）、保存した画像の再生、テスト用の期待値、推測は到達に数えない（[`../policies/progress-integrity.md`](../policies/progress-integrity.md)）。
>
> **現在の最高到達段階の読み（2026-11-04 の注記。決定 0198・0199・0207）**: 本文 §1 の表の「現在の状況」の欄と §2 の『現在の最高到達段階: G0』は作成時（2026-10-30）のままで、古い。字義どおりの監査は §2.1（69 項目。**2 通りの読み**: 最も厳しい読み = G0、字義どおりの読み = G4）。§2.1 が挙げた記録の欠け（G0 の現行 HEAD の Chrome 観測、G1-7、G2-7、G3-4、G4-4b）は決定 0199 で埋まった（G1-7・G3-4 は F1xH + opt-in の観測で既定の入口の Worker ログには出ない。G2-7 は app に実装済みで配信サイトは再ビルド待ち。G2-7 は PM が『実反映』を採った=0198 PM）。G7『未達』・G8/G10『未測定』のラベルは古い（0198）。報告は 2 通りの読みを併記する（0198 PM の恒久の書式）。古い記述の一覧: `investigations/docs2/tables/stale-labels.tsv`。この注記は本文を変更しない。

## 2026-11-07 の状況欄（決定 0314。追記。本文の §1 の表・§2・§2.1 は変えていない）

基準 HEAD f0e5a2f5。**docs の読みと整理だけで作った（実行・測定・再測定は 0 件）。段階の判定は変えない**（PM の指示。字義どおりの読みの判定は決定 0198・0199 のまま）。数字は決定文書・`milestones/performance-targets.md` §4e・`docs/README.md` の索引からの転記で、出典（決定番号）を併記する。区分は **実測**（その文書の作業者または PM が出力を見た事実）、**読み**、**未確認**。測定はこの開発用の Linux VM（実 Chrome 156・Firefox 158、headless、模擬入力）で、**依頼者の機械の値は 0 件、実機 Windows との照合は 0 件**。

**現在の最高到達段階（判定は変えない。二つの読み）**: 最も厳しい読み = **G0**、字義どおりの読み = **G4**。G9（同梱プラグイン）は未到達、G10・G11 は未着手（PM の整理）。

**本文の古い記述の訂正の指針**（表の本文は変えない）: §1 の表の『現在の状況』の欄は 10-30 のまま、§2.1 の表は 11-03 の判定で、次が事実と合わない。(1) G7 の『`?audio=1`（opt-in）で条件つきに充足』: 音は既定で ON（0211）。(2) G8 の『既定の入口では File>Open が何も起こさない』: OPFS の窓で File>Open/Save/Import/Export に答える（0216）、参照プロジェクトを開いて再生（0229・0236）。(3) G10 の『出荷構成で Export を完走させる（b2d3 待ち）』: 本番 E13 で完走し参照と一致（22,863,828 B、sha256 `381cdb5a…f979`。E8〜E13 で同一。0277-e13）。(4) G1・G3 の『opt-in の観測』『SHIPH』『ship2』: 0243 でオプションを畳み、本番は単一構成 Ω（8 モジュール）になった。(5) G2 の『配信サイトは再ビルド待ち』: `document.title` は前景窓の題を `app/src/main.ts` が反映する（0199・0223）。

| 段階 | 字義どおりの判定（2026-11-07。判定は 11-03/11-04 のまま） | その後の事実（出典） | 次に観測すべきこと |
|---|---|---|---|
| G0 | 到達 | 本番 Ω（FLEngine + 同梱 5 DLL + oggio + Elastique）は Chrome・Firefox で first idle に達する（E13: Chrome 中央値 4,320 ms、Firefox 4,172 ms。0277-e13）。a4 942/942・895/895・5,310 関数・1,009,737 入口は E12・E13・B1 の木でも基準と同一（native。0312） | なし。最も厳しい読みの連続の起点は変わらない |
| G1 | 記録が揃った（条件つき: F1xH + opt-in の観測。0199） | 0243 で opt-in を畳んだ後の、Worker ログ上の Run 入口の直接観測は文書に見つからない（**未確認**）。下流（first idle・入力・Export・60 s 再生）は本番で成立 | 製品の構成（単一構成 Ω）での Run 入口の 1 行。要否は PM |
| G2 | 記録が揃った（条件つき） | 主窓 @0,0 1920x1080（alpha7 = ShowWindow の最大化の契約。0258、ff5ee429 以降の ci1 の確認）。`document.title` の反映は `app/src/main.ts`（0199・0223）。最小化 → `#taskbar-restore` → 復元が本番で通る（0267・0272・0277-e8）。復元後の playlist の題が `(none)` になるのは FL 自身の挙動（0291） | 本番の `document.title` の実値（主窓の題）の記録 |
| G3 | 記録が揃った（条件つき） | FL の UI の刻みの量は `EnumDisplaySettingsW` の `dmDisplayFrequency` で決まり DwmFlush の周期ではない（0299）。`DwmGetCompositionTimingInfo` の 40 メンバを返す（0302、E13） | 本番での実エンジン自身の `WM_TIMER` の間隔（**未測定**） |
| G4 | 到達（読み A・B1・B2） | 窓の合成は runtime の `fle_desk.c`（Windows の DWM に当たる）で、ページが受け取る画素は 0（0223）。alpha7 の画は E12 と E13 で 0 画素差（0277-e13） | 読み C = 依頼者の実機との比較（依頼者の供給） |
| G5 | 未到達 | 実機のスクリーンショット 0 件。Browser 欄の黒は WebView2 不在（既知。0236）。Problems 窓の一覧欄が空だった原因は OS フォントの入力不足で、代用 3 フォントを足す差分を出した（0236。本番での反映は**未確認**） | 依頼者の実機（Windows、DPI 100%、1920x1080）の起動直後のスクリーンショット。Problems 窓の一覧欄 |
| G6 | 項目は充足。**数えない規則（P2 未達）に該当** | 再生中の File クリックの T の中央値 E7b 79 → E9 22 ms、Esc 41 → 6 ms（gpu1、4e 12:55）。Mixer ドラッグ・idle の入力は E7b と E9 で同等（File 6.9 対 7.4 ms、Mixer 3.4 対 3.3 ms）。画面更新は純再生で 15.6/s（120 の 13%。0297）。E14（present を別 Worker）は、V7 を足した P + V7 の製品形 ABBA 7 run で File・Esc の T の中央値は基準 +1〜2 ms 以内だが、Esc の late が V7 8/147 対 基準 1/110（両側 0.083）でどちらも有意でないが基準と同数でなく、E14 は P + V7 の最終の組で未 push（e2e・Firefox の p3 対 pv7s・最終再検証が残る。first idle・pure は 0277-e14） | P2: 再生中の T の p95、120 fps（headless の rAF は 60 Hz なので依頼者の機械でしか確認できない）。依頼者の画面の Hz |
| G7 | 項目は充足（**音は既定で ON**: 0211）。**数えない規則（P3 未達）に該当** | 本番 E9 の純再生（開始 load ≤8・I/O 圧 0）: 10〜60 s の書き込み率 0.401 → 0.763、underrun 1,075 → 15 quanta、仮想 s/実 s 1.016 → 0.572（0277-e9）。E11 → E12 は検出できる変化なし（0277-e12）。E13 の純再生（静かな窓、12 run すべて数えた run）は E12 と実質の差なし（書き込み率 0.985 → 0.992、仮想 s/実 s 0.806 → 0.811。60 s では音楽 40 s 前後までで重い区間は未検証。0277-e13）。実スレッド版 B1 の 5 分再生は数の基準を満たす（本番ではない。0312） | P3: 曲の位置で揃えた書き込み率（全曲の重い区間を含む 5 分）、実スレッド版の T、G9 後の負荷。耳での確認（記録なし） |
| G8 | 未到達（部分） | File>Open/Save/Import/Export（OPFS の窓。0216）、参照プロジェクトを開いて再生（0229・0236）、Export の WAV は参照と一致（本番）。未達: 内蔵エフェクト 7 種 25 インスタンスと Fruity Wrapper が開かない（『8 plugin(s)』）、書き出しの比較は Wine 10 上の原版（別実装）とだけで、差の主因は同梱プラグインの有無（0307）、残差は ch7・ch35 の時間伸縮（0308・0308b）に局在し、入力統一の A/B（01:47）で RMS 2,031 → 1.0、さらに CPUID を SSE2 に揃えると 0.001（5,715,911 フレーム中 4 フレームが +1 LSB、パッドの出力は byte 一致。03:48）= **完全に閉じた**。差は MP3 復号器と CPU 機能の経路（外部環境）の差で、lift した Elastique の算術の欠陥ではない（0307 の PM の追記）。ACM の MP3 復号器は minimp3 のまま（PM の暫定方針）。実 Windows との差は未確認 | 同梱デモ `NewStuff.flp`（**未確認**）。G9 |
| G9 | 未到達 | native で Fruity Wrapper の load（0183）と Serum 2 の process の出力・出力リングへの到達（0200）。FL 内蔵の 7 種はブラウザで未搭載。入口の見積り（Balance・Filter・Delay 2 は fe-lift が各 1.1〜1.2 s で lift でき、命令レベルの未対応 0）と S1 の比の読み（Balance で 1.848 → 1.94、2 つ目で 2.03）は 0309。実測は Balance 1 つで wasm +7,763,869 B・比 1.859 倍（0313 §6）で、読みは過大。B-FLE の設計は 0312g・0312h・0312i。G9-a（0313）: Balance を実モジュールとして足した test build（wasm 150,851,575 B）の V3 で、first idle に到達、`LoadLibraryW` → 0x6800000、`GetProcAddress("CreatePlugInstance")` → 0x6abdde0、『fle: unsupported』0 行、Problems 窓は 8 → 7 件で Balance の行が消えた（0313 §7、PM が画像で確認）。G9-b（ci1 の Chrome の Export、非計時）は完走し、WAV の sha256 が `94c93666…4090` で基準 `381cdb5a…f979` と違う（11.21% のフレーム。alpha1 の解析〔0310 §6〕では Balance の自動化 clip の内側とその終端の尾だけ）。**Wine の原版 FL（Balance のみ有効）の Export とは 5,715,911 フレーム中 5 フレームだけ ±1 LSB 違い（RMS 0.0007。0307 §2p・0310 §9）= Export に対して原版と一致**。Balance をスロットから開くと窓の枠は出るが client 領域は描かれない（gpu1 の stage 3、PM が画像で確認。出典は 0313 の PM の追記）。**cand1（Filter も実モジュール。TEST・push 前）**: V3 合格、実モジュールは 7 種のうち 2 種、原 FLP の Export は Balance のみと全バイト一致で Wine と 2 フレーム ±1 LSB、**変種 ins1-lp は Wine と一致しない**（差 RMS 5,325.57。0313 の 09:48 の追記）= **G9 は未到達** | G9-c（Balance の GUI の client 描画。0323① の修正で変化なし、`diag-g9-fl-paint` 待ち）→ Fruity Filter の判別（cand1 の V3 は合格。変種 ins1-lp は Wine と不一致で原因未特定）→ 増分の再測定（RT-A の S1 1.8644） |
| G10 | 未到達（PM の整理では未着手。部分の観測あり） | 本番の Export 完走。Save as/Save（未認証で『Buy FL Studio』。0216）。native の走査 8,552 操作で停止率 0.71%（0215）。Split mixer tracks は wasm で `WriteFile: out of memory`（2 GiB の上限。0312j）。Settings のタブ切替は Chrome で測定済み（0225。本番の 8 モジュール版と同一 wasm で Worker の busy 52%: 0249）。ピアノロール: ci1 の場面 d10-pianoroll-f12 は本番 E13 と P + V7 の page で停止なしで通る（窓が出たかの画素確認は未）。MP3 書き出しのブラウザでの観測は文書に見つからない（**未確認**） | Split の OPFS（案 A1）、ピアノロール、MP3 書き出し |
| G11 | 未到達（未着手） | 参照プロジェクトの第三者プラグイン 46 個は供給物が無く『could not be opened』（0229）。Serum 2 は native のエンジンの中で process の出力まで（0200）。ブラウザの FL の中では 0 件 | G9 の後。ライセンスの経路 |

## 0. 構成（サーバー再起動の後に、節ごとに保存しながら書いた。全節記入済み）

| 節 | 状態 |
|---|---|
| 1. ladder（段階と受け入れ条件） | 記入済み |
| 2. 現在の最高到達段階と各段階の測定状況 | 記入済み（§3.9、3.10 で更新） |
| 3. 各段階の距離の見積もり（静的分析） | 記入済み（3.0〜3.10） |
| 4. 最短経路（依存関係）と不確実性 | 記入済み |
| 5. 実験案（優先順、見積もり、結果で計画がどう変わるか） | 記入済み |
| 6. 進捗指標の置き換え案と、M1 の各項目がどの段階を塞ぐか | 記入済み |
| 7. 限界・未確認・規則違反の自己申告 | 記入済み |

## 0.1 凡例

- **実測** = コマンドの出力・実行ログ（出典を併記。この調査自身が行った静的な数え上げを含む。自分で実行したものは「本調査の実測」と書く）。
- **読み** = 逆アセンブル・ソース・文書を読んだ解釈。実行して確かめていない。
- **未測定** = その段階を実行した記録が無い。**未確認** = 調べていない、または調べたが決められない。
- 「引用」は既存文書の記述で、本調査は再測定していない。
- **根拠が無いものは数字にしない**。見積もりの数字には、必ず「実測/読み」と出所を付ける。
- 「仮の応答」は決定ファイルが仮と書く応答（README の `**仮**` の行、47 件）。実機 Windows との照合は 0 件。

## 1. ladder: 「ブラウザ上で FL Studio が動作する」を観測できる条件で 12 段階に分ける

各段階の受け入れ条件は、**ブラウザ（headless Chrome を含む）側で観測できること**で書く。段階の順序は「下の段階が無いと上が観測できない」という依存の向き（厳密な直列ではない。依存は §4）。実機 Windows との照合は全段階で 0 件であり、各段階の「実物と同じ」は別の軸（§6）。

| 段階 | 名前 | ブラウザで観測できる受け入れ条件 | 現在の状況（詳細は §2） |
|---|---|---|---|
| **G0** | エンジン初期化の完走 | Web Worker 内の wasm で DllMain → InitUnits が 942/942 で完走し、a4 の行 7 本が native と一致する | **実測**（native、node、headless Chrome 156。独立再現あり。仮の応答 47 件を含む） |
| **G1** | `CreateFruityInstance` が事前検査と初期化の本体（`f_13751e0`）を通り、Run（アプリのメッセージループ。0x1375950）の入口に着く | Worker のログ（診断フック）に Run の入口への到達が出る。署名検査（`WinVerifyTrust`）、CPU 検査、`BasicInitialization`、`TApplication` の設定、初期化の本体を通過している | **実測（部分。native のみ。HEAD の最後の停止は 38,241 関数入口目の CPU 検査の失敗（決定 0117）。未コミットの作業（`lift/out-fruity5/`、引用）では 38,913〜38,928 入口目の `TApplication` のウィンドウ作成（G2 の入口）まで進んだ）**。初期化の本体 `f_13751e0`（閉包 6,449 関数）は 1 点も実行されていない。ブラウザ（wasm）では未測定 |
| **G2** | メインウィンドウの作成 | 窓表に可視の最上位ウィンドウが作られる（隠しの `TPUtilWindow` を除く）。クラス登録、`CreateWindowExW`、`WM_NCCREATE`〜`WM_CREATE`、`ShowWindow`。サイズとタイトルが JS から読め、canvas のサイズと document.title に反映できる | **未測定**（隠しの補助ウィンドウ 1 つだけ実測。決定 0071）。**`TApplication` の窓（スタイル 0x84ca0000）で止まる地点までは引用の実測**。**G1 の途中（第 8 項）で要求される**ので、G1 と G2 は分離できない |
| **G3** | メッセージループが回り、ブラウザのイベントループへ制御が戻る | エンジンが `PeekMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects` で待つ間もタブが応答し（別スレッドの rAF が維持される）、`WM_TIMER` が等間隔で届く。アイドルで CPU を使い切らない | **未測定**（待ち方の機構は JS 試作と a4 経路の JSPI で実測。実エンジンのループは未実行） |
| **G4** | 最初の 1 フレーム | エンジンが描いた画素が canvas に出る（空白でない。エンジンの GDI 呼び出し結果と画素が一致する） | **未測定** |
| **G5** | 完全な初期画面（文字・アイコンを含む） | 起動直後のメイン画面全体（メニュー、ツールバー、ブラウザ、プレイリスト、ミキサーなど）が、テキストとアイコンを含めて描かれ、実機のスクリーンショットと領域ごとに比較できる | **未測定**（実機のスクリーンショットも未入手） |
| **G6** | 入力の往復 | ブラウザのマウス・キーボード・ホイールが `WM_*` としてエンジンに届き、画面が変わる（ホバーの強調、メニューが開く、ドラッグ）。canvas の差分で観測 | **未測定** |
| **G7** | 音声デバイスと無音の出力 | エンジンが音声出力を開き、空のプロジェクトで再生すると、AudioWorklet が一定のブロックを連続して受け取る（アンダーラン 0 を一定時間）。出力は無音でよい | **未測定**（AudioWorklet 側の受け皿は JS 試作で実測。エンジン側の音声経路は未実行） |
| **G8** | プロジェクトの読み込みと再生（内蔵の音源だけ） | 同梱デモまたは保存済みの `.flp` を読み込み（ファイル I/O。OPFS または MEMFS）、画面に反映され、再生すると AudioWorklet に非ゼロのサンプルが出る。レンダリング結果を実機の書き出しと比較できる | **未測定** |
| **G9** | 同梱プラグイン | `Plugins\Fruity` のプラグイン DLL（160 個）を `LoadLibraryW` + `CreatePlugInstance` で読み、GUI が出て音が出る | **未測定**（棚卸しだけ。m2-inventory） |
| **G10** | UI の広範な操作とダイアログ、保存・書き出し | ピアノロール・ミキサー・設定・ファイルダイアログ・モーダル・ドラッグ・メニューを一通り操作でき、プロジェクトの保存と WAV／MP3 の書き出しができる | **未測定** |
| **G11** | 外部プラグイン・ライセンス・オンライン機能（M2 以降） | VST2／VST3／CLAP の第三者プラグイン、ライセンス検証、更新確認・ネットワークが動く | **未測定** |
> **2026-11-03 の注記（決定 0198）**: 上の表の「現在の状況」の欄は 2026-10-30 の作成時のままで、G1〜G11 の多くが古い（実エンジンは native とブラウザで主窓・入力・メニュー・FLP・WAV 書き出しなどを実行した）。字義どおりの監査は §2.1。古い記述は消していない。

横断の軸（段階ではない）: (a) **実物との同等性**（画面の画素比較、レンダリング音の一致の許容差。実機が無いので全段階で未達）、(b) **性能**（リアルタイムで回るか。a4 の初期化は wasm で約 3 秒だが、描画・音声の負荷は未測定）、(c) **配信**（wasm 389 MB。brotli で 44 MB。プラグイン込みは数 GB の見積もり。m2-inventory）。

## 2. 現在の最高到達段階と各段階の測定状況（§3.9、§3.10 に静的分析の結果を反映）

**現在の最高到達段階: G0**（実測。6 モジュール構成の初期化の完走。native、node、headless Chrome 156 の Web Worker の wasm で a4 の行 7 本が一致。出典: `progress.md` の (1)(1w)、`browser-feasibility.md` §3.1）。

- G1 は部分（native のみ）: `CreateFruityInstance(0xC0, 0xEDE95A, 偽ホスト, &instance)` は、6 モジュールの初期化の後に呼ぶと、署名検査（決定 0117 の主張の表で通過）を越え、**HEAD の決定 0117 の停止点は 38,241 関数入口目の `user32!GetActiveWindow`（エンジンの CPU 検査の失敗によるエラーボックスの経路）**。決定 0118 の作業中（未コミット。`lift/out-fruity5/`、引用。§3.9）では CPU 検査を越え、38,913〜38,928 入口目の `TApplication` の窓の作成（`CreateWindowExW`。スタイル 0x84ca0000）まで進んでいる。Run の入口（0x1375950）にはどちらも届いていない。**ブラウザ（wasm）では `CreateFruityInstance` を一度も呼んでいない**（引用: `jspi-real-wasm.md`: a4 経路にブロッキング API は無い）。
- G2 以降は、実エンジンのコードが実行された記録が無い。G2〜G11 の行は「未測定」で、§3 に静的分析（読み）で距離を書く。
- 「A4 完走」「M1 の 5 項目の平均」は、G0 までの内部の受け入れ基準の進み方であり、G1 以降の段階を 1 つも含まない（§6）。

### 2.1 2026-11-03 時点の監査（決定 0198。追記。上の §2 の記述は消していない）

基準 HEAD 504348b（終わり近くに 8384133 まで進んだ）。§1 の「ブラウザで観測できる受け入れ条件」を字義どおりに 69 項目へ分解し、決定 0116〜0193 の実測と、隔離 Chrome 156 ヘッドレスでの既定の入口（SHIPH、パラメータ無し）の再観測（R1〜R5、A1、A2）に突き合わせた。**項目の表・証拠・環境は `docs/investigations/ladder2/tables/ladder-items.tsv`、判定の語彙・読み・再観測の記録・自己訂正は `docs/decisions/0198-ladder-audit.md`**。判定の語彙: 観測済み（ブラウザ・実エンジン・既定の入口／既定以外の構成）、native/node のみ、間接、未観測、数えない規則に該当（テスト用の刺激・代役・推測・P1〜P3 の注記）、依頼者の供給が必要。実機 Windows・依頼者の機械との照合は 0 件。

**現在の最高到達段階の読み（2 通り。PM が選ぶ）**:

- **最も厳しい読み**（段階の全項目が、ブラウザ・実エンジン・既定の入口で観測され、下位の全段階が到達で、P1〜P3 の注記を適用し、曖昧語は不利な読み）: **G0**。ただし G0 の根拠は 2026-10-28 の FRUITY 無効 wasm の Chrome の観測で、出荷構成の Worker ログは a4 の行を出さない（`a4: init_count=0 of ?`）。
- **字義どおりの読み**（§1 が「厳密な直列ではない」と書くので段階ごとに自分の項目で判定。曖昧語は読み A〔能力〕を採る）: **G4**。ただし G1・G2・G3 に各 1 項目の記録の欠けがある（G1: Worker ログ上の Run 入口、G2: `document.title` への反映〔読みで分かれる〕、G3: 実エンジン自身の `WM_TIMER`）。連続性を要求するなら G0。
- G6（操作感）・G7（実時間）は項目をほぼ満たすが、P2・P3 を満たすまで「達成」と数えない（冒頭の注記）。G5 の比較・G4 の読み C・G8 の書き出しの比較は依頼者の供給が要る。

| 段階 | 字義どおりの判定（2026-11-03） | 次に観測すべきこと |
|---|---|---|
| G0 | 到達(条件つき: ブラウザの観測は 2026-10-28 の FRUITY 無効 wasm。現行 HEAD の Chrome では未再観測) | 現行 HEAD の FRUITY 無効 wasm を Chrome の Worker で 1 回走らせ、a4 の 7 行を native と diff する(別ビルド。決定 0128 の見積もりで約 27 分、RSS 8.8 GB)。または PM が「FRUITY 有効の出荷構成で DllMain が戻り CreateFruityInstance が走る」を G0 の充足と読む。 |
| G1 | 部分(欠け: G1-7 Worker ログ上の Run 入口〔未観測〕。G1-5・G1-6 は native のみ。G1-1〜G1-3 は下流からの間接) | Run(f_86a660 の入口)で 1 行を出す opt-in の runtime フックを wasm に入れ、Chrome の Worker ログに Run 入口が出ることを見る(出荷構成の再ビルドが要る)。 |
| G2 | 部分(欠け: G2-7 document.title〔読み A=JS が題を読める、なら充足〕。G2-3 のメッセージ列は結果からの間接) | PM が G2-7 の読み(能力か実反映か)を決める。実反映なら app/src/main.ts:593 の近くで appView の主窓の題を document.title に反映(数行)し、R1 と同じ手順で確認する。 |
| G3 | 部分(欠け: G3-4 実エンジン自身の WM_TIMER。G3-3 はホスト作成のテスト用タイマーで数えない) | 実エンジンの SetTimer が生きる場面(再生中、ダイアログ、Settings など)を探し、fle_win_timer_log の配送時刻を Chrome の Worker に出して間隔を測る。 |
| G4 | 読み A で到達 / 読み B1〔Chrome == node の基準〕で到達 / 読み B2〔== native〕は出荷系の構成で突き合わせ記録なし / 読み C は依頼者の供給 | 出荷系の構成(F1x/SHIPH)を native -O0/-O2 でも同じ仮想時刻(a3000: lid 14 cf92b670…、lid 48 221e46db…)まで走らせ、面の SHA が Chrome・node と一致するかを見る(G4-4b)。 |
| G5 | 未到達(G5-4 実機との比較は依頼者の供給。既知の欠けあり) | 依頼者が実機(Windows、DPI 100%、1920x1080)の起動直後のスクリーンショットを供給する。並行して Browser 欄の黒い領域の原因を調べる(決定 0134 の PM 注記は WebView2 なしと読む)。 |
| G6 | 項目は充足(マウス・キー・ホイール・ホバー・メニュー・ドラッグ)。数えない規則(P2 未達)に該当 | P2(120 fps、入力→更新 p95 ≤ 17 ms)を満たす。ホイール・ドラッグは画素の差でなく画面上の値(テンポ、フェーダー)の変化として読む。 |
| G7 | 項目は ?audio=1(opt-in)で条件つきに充足(再生中 30 s で 5 quanta の欠け、無音 10 s は 0)。数えない規則(P3 未達)に該当 | PM が「一定時間」の数値を決める(例: 300 s で欠け 0)。?audio=1 を既定にするか。P3。300 s 欠け 0 の独立再現(決定 0175 は 300 s で欠けあり)。 |
| G8 | 未到達(部分: ブラウザで .flp を開くのはテスト用バンドルのみ。既定の入口では File>Open が何も起こさない。再生→AudioWorklet、デモ、実機比較が無い) | 出荷 SHIPH にブラウザのファイル橋(-DFLE_FILEDLG_BROWSER、決定 0193 の Not done)を載せ、Sampler.flp を開いて再生し、AudioWorklet の非ゼロを Chrome で測る。同梱デモ NewStuff.flp も開く。 |
| G9 | 未到達(native で Fruity Wrapper の load のみ) | Serum を使わない Fruity Wrapper の GUI、または同梱ジェネレーターを 1 つ lift して、Chrome で GUI と音を出す(native では Serum 2 の最初の process の SIGSEGV〔決定 0190〕が先)。 |
| G10 | 未到達(部分: メニュー・モーダル・ドラッグ。Settings・ピアノロール・保存・MP3 が無い。WAV は native とテスト構成のみ) | Settings・ピアノロールを開く、プロジェクトを保存する、出荷構成で Export を完走させる(b2d3〔決定 0195〕待ち)。 |
| G11 | 未到達(Serum 2/OTT は FL の外の単体のみ) | native の実エンジンの中で Serum 2 の最初の process の SIGSEGV(決定 0190)を解いて出力を見る。ライセンスの経路と、サードパーティ VST3 の FL の中での Chrome での実行。 |


## 3. 各段階の距離の見積もり（静的分析）

### 3.0 方法（本調査の実測。実行はしていない）

- ツール: `docs/investigations/goal-ladder/fle_index.py`（FLEngine の `.pdata` 57,816 関数を capstone で走査し、直接 call／tail jmp、`call/jmp [IAT]`、`lea` の索引を作る。約 7 秒、RSS 約 0.7 GB）と `fle_paths.py`（索引に対する到達・経路・呼び出し元の問い合わせ）。`imports_vs_table.py`（FLEngine の import 795 名と応答表 445 行の突き合わせと分類）。
- **限界（先に）**: 到達は**直接 call と tail jmp（T1）と、`lea` で関数アドレスを取る辺（T2）**だけを辿る。Delphi の仮想メソッド（vtable）、`WndProc` のメッセージ配送、コールバックは辿れない（K1 の指摘と同じ。例: ILTools の `MoveToEx` は T1／T2 に入らないが実測では最初の壁だった）。したがって**下の到達集合はすべて下限**で、「到達しない」は「実行されない」を意味しない。分類の正規表現は本調査の読みで、API 名による分類であり実行の裏付けではない。
- 応答表の行の有無は `lift/crates/fe-lift/src/imports_table.rs` の `TABLE`（HEAD の 445 行）。**行があることは、どの入力でも通ることを意味しない**（行の中の名前付き停止がある。決定 0111 の区分 C の 69 行は「行はあるが呼ぶと名前付きで止まる」）。

### 3.1 FLEngine の import 795 名（通常 738 + 遅延 57）と応答表 445 行の突き合わせ（本調査の実測。名前の分類は読み）

| 分類（API 名による。読み） | 行あり | 行なし | 計 |
|---|---:|---:|---:|
| ウィンドウ管理・属性（CreateWindowExW、ShowWindow、SetWindowPos、GetWindowRect ほか） | 7 | 77 | 84 |
| メッセージ・タイマー・フック（GetMessageW、PeekMessageW、DispatchMessageW、SendMessageW、SetTimer ほか） | 1 | 27 | 28 |
| 入力（キー・マウス・キャレット・IME・ドラッグ） | 7 | 30 | 37 |
| メニュー・ダイアログ・ファイル選択 | 1 | 29 | 30 |
| GDI 文字（ExtTextOutW、DrawTextW、GetTextExtentPoint32W、CreateFontIndirectW ほか） | 11 | 7 | 18 |
| GDI ビットマップ・転送・アイコン（BitBlt、StretchDIBits、CreateDIBSection ほか） | 43 | 5 | 48 |
| GDI ベクタ（LineTo、Polygon、Ellipse ほか）・座標変換 | 29 | 4 | 33 |
| GDI 領域・クリップ・矩形 | 23 | 18 | 41 |
| GDI パレット・メタファイル | 18 | 2 | 20 |
| 表示情報・DPI | 8 | 11 | 19 |
| 音声・MIDI・時間（dsound 2、acm 14、midiIn/Out 20、winmm 時間系） | 12 | 28 | 40 |
| スレッド・同期 | 19 | 13 | 32 |
| ファイル I/O・シェルのパス | 38 | 7 | 45 |
| COM・OLE・BSTR・PROPVARIANT | 28 | 16 | 44 |
| ネット・シェル・レジストリ・暗号・セキュリティ | 22 | 31 | 53 |
| ローダー・メモリ・時刻・ロケール・CRT 的なもの | 97 | 56 | 153 |
| その他 | 7 | 63 | 70 |
| **計** | **371** | **424** | **795** |

注: 「行あり 371」は FLEngine の import のうち TABLE に (DLL, 名前) が一致するもの（遅延 import を含む）。TABLE の 445 行には ILTools・QFC・dsp が import する名前が含まれ、FLEngine が使わない行もある。`progress.md` の「IAT 738 のうち行なし 484 → 決定後 390 に減少」とは数え方が違う（こちらは遅延 import 57 を足し、行の有無で数える。**正確な突き合わせは `goal-ladder/fle-imports-cat.tsv`**）。K1 の表（HEAD 3a1050a 時点）は IAT 738 のうち行なし 484。

**読み**: 行なし 424 のうち、**ウィンドウ・メッセージ・入力・メニュー**の 4 分類（計 179 名）が 163 名（77 + 27 + 30 + 29）を占める。つまり G2〜G6 に必要な user32 の層は**ほぼ全く無い**（行ありの 16 名は `CreateWindowExW`／`RegisterClassW`／`DefWindowProcW`／`GetClassInfoW`／`SetWindowLongPtrW`／`GetDC`／`ReleaseDC`、`RegisterWindowMessageW`、カーソル 7 名ほか。いずれも「窓なし」または隠しの補助ウィンドウまでの答え）。一方、**描画の GDI（文字・ビットマップ・ベクタ）は名前の上では行あり 83 名**（43 + 29 + 11）だが、決定 0111 の区分 C により、**行のあるもののうち描画の実体（`ExtTextOutW`、`StretchDIBits`、`StretchBlt`、`GetTextExtentPoint32W`、`GetTextMetricsW`、`LineTo`、`Rectangle` ほか）は名前付き停止**（§3.4）。

### 3.2 メッセージループ、メインウィンドウの作成、描画、音声の位置（本調査の実測＝静的な呼び出し関係。意味づけは読み）

`CreateFruityInstance`（0x1375990、`Flags = 0`）の中から、**直接 call のみ**（T1）で辿った経路（`fle_paths.py path`。関数名はエンジンの VA、シンボルは無い）:

| 目的の API | 直接 call の経路（先頭が API の thunk） | 区分 |
|---|---|---|
| `PeekMessageW`／`TranslateMessage`／`DispatchMessageW` | thunk ← `f_86a0f0`（`ProcessMessage`）← `f_86a250`（`HandleMessage`）← `f_86a660`（`TApplication.Run`）← `f_1375950` ← `CreateFruityInstance` | 実測（呼び出し関係）。関数名は読み |
| `WaitMessage` | thunk ← `f_86b440`（`Idle`）← `f_86a250` ← `f_86a660` ← … | 同上 |
| `MsgWaitForMultipleObjects` | thunk ← `f_867170` ← `f_86bcd0` ← `f_86b440` ← `f_86a250` ← … | 同上 |
| `SetTimer` | thunk ← `f_86b440` ← `f_86a250` ← `f_86a660` | 同上 |
| `CreateWindowExW`（最初に届くもの） | thunk ← `f_867e10`（アプリケーションの窓の作成。`LoadIconW(MAINICON)` の周辺）← `CreateFruityInstance` | 実測（呼び出し関係）。`f_867e10` が `TApplication` の隠しの窓（`TApplication` クラス）を作ると読む（未確認） |
| `BeginPaint` | thunk ← `f_8261a0` ← `f_8205c0` ← `f_7727d0` ← `f_c343f0` ← `f_135d220` ← `f_11a57c0` ← `f_11be0a0` ← `f_13751e0` ← `CreateFruityInstance`（`f_13751e0` は事前検査の後の初期化の本体の入口と読む） | 実測（呼び出し関係） |
| `DirectSoundCreate`（dsound） | thunk ← `f_12afb50`（文字列 "Couldn't create the DirectSound main object."）← `f_1253260` ← `f_1253610` ← **`f_11efbf0`** ← `f_1375950` | 実測（呼び出し関係と文字列） |
| `CreateThread` | `f_412b50` ← `f_525400` ← `f_5252d0` ← （`f_522710` ← `f_525940` ← `f_524e00` ← `f_86b440`（Run のアイドル）、または `f_11ef180` ← `f_11efbf0`）。`f_412b50` はスレッド生成の共通関数 | 実測（呼び出し関係） |
| `midiOutOpen` | thunk ← `f_ba4950` ← `f_117c9d0` ← `f_11d3340` ← `f_1196c00` ← `f_11a57c0` ← `f_11be0a0` ← `f_13751e0` | 実測（呼び出し関係） |
| `ReadFile`／`InternetOpenW`／`CoCreateInstance` | `f_13751e0` 以下に直接 call の経路がある（CoCreateInstance は `f_11efbf0` 以下） | 実測（呼び出し関係） |
| `midiInOpen`、`WaitForMultipleObjects` ほか | 直接 call の経路は見つからない（vtable、コールバック経由と読む） | 実測（経路なし＝下限） |

- **`Flags` のビット 0 の分岐**（`f_1375950`、本調査が逆アセンブルで確認。実測）: `[0x15e3a78]` の指す byte（`Flags` のビット 0 の保存先と読む）が 0 なら `f_86a660`（Run。`TApplication.Run`）、非 0 なら `f_11efbf0`（別のループ関数）。**FL64.exe は 0 なので `f_86a660` の経路**（ホストの寄与は `Flags` 1 つ）。
- **`f_86a660`（Run）の直接 call の閉包は 464 関数で、import 96 名（行あり 53、行なし 43）**。`f_86a660` から直接 call だけでは `DirectSoundCreate`／`CoCreateInstance`／`midiIn/Out`／ファイル I/O には届かない。**Run の閉包は小さく、実際の仕事（ウィンドウのメッセージの処理、描画、音声）は `WndProc` 経由（`DispatchMessageW` → ゲストの `WndProc` → Delphi の VCL）で呼ばれる**（読み。静的には辿れない）。
- **初期化の本体 `f_13751e0`（直接 call の閉包 6,449 関数、import 350 名（行あり 236、行なし 114））は、Run の前に実行される**（`CreateFruityInstance` の順序: 事前検査 → `f_11b32a0`（BasicInitialization の閉包 1,267 関数、import 153（行あり 126、行なし 27）→ `f_13751e0` → Run）。ここに音声デバイスの列挙（`midiOutOpen`）、プロジェクト／データブラウザ、ネットワーク（`InternetOpenW`）、`ReadFile` が入る（読み。到達は実行時の条件分岐に依存し、静的には「届きうる」まで）。
- **`CreateFruityInstance` 全体（直接 call のみ、T1）の閉包は 6,591 関数、import 379 名（行あり 252、行なし 127）。`lea` を含める（T2）と 8,953 関数、import 440 名（行あり 263、行なし 177）**（`fle_paths.py reach 0x1375990`。下限。vtable は含まない）。決定 0117 の停止点（38,241 関数入口目）までに実行された関数は 348（新規）。**直接 call の閉包 6,591 関数のうち、実行が確認された関数は約 350 関数（5%）**。

#### 行なし 127 名（T1 の閉包）の内訳（本調査の実測。分類は読み）

DLL 別（本調査の実測）: user32 94、kernel32 8、wininet 6、shell32 4、comctl32 4（FlatSB）、advapi32 3、dsound 2、ほか各 1（wtsapi32、uxtheme、shcore、rpcrt4、ole32、dwmapi）= 127。ウィンドウ・メッセージ・入力が大半（user32 94 名）: `CreateWindowExW` を除く `DestroyWindow`、`ShowWindow`、`SetWindowPos`、`GetWindowRect`、`PeekMessageW`、`TranslateMessage`、`DispatchMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`SendMessageW`、`PostMessageW`、`SetTimer`／`KillTimer`、`BeginPaint`／`EndPaint`、`InvalidateRect`、`SetCapture`、`SetFocus`、`GetKeyState`、`GetCursorPos`、`MessageBoxW`、メニュー 6 ほか。その他: ネット（wininet 6）、スレッド（`CreateThread`、`ResumeThread`、`GetExitCodeThread`）、COM（`CoCreateInstance`）、`DirectSoundCreate`、`DirectSoundEnumerateA`、`ShellExecuteW`／`ShellExecuteExW`、`FormatMessageW`、`GetTokenInformation`／`OpenProcessToken`、`UuidCreate`（遅延）、DPI 関連（遅延）。**一覧は `goal-ladder/reach-cfi-t1-none.txt`**。

### 3.3 描画の実体: Blend2D のビットマップへ書いて BitBlt するだけで済むか、GDI の文字描画・グリフのラスタライズが要るか（**最大の不確実性の 1 つ。結論は出ていない**。本調査の静的な証拠を分けて書く）

**先に結論（読み。実行していない）**: 証拠は「描画の大半は、エンジンが自分で持つメモリ上のビットマップ（DIB セクション）へ書き、最後に `BitBlt`／`SetDIBitsToDevice` で窓へ出す形」を支持する。ただし**文字の描画がどちらを通るか**（Blend2D／FreeType／GDI の `ExtTextOutW`／`DrawTextW`）は静的には決まらず、**実行して確かめる必要がある**。「BitBlt だけで済む」とは言えない。

| # | 証拠 | 区分 | 意味 |
|---|---|---|---|
| E1 | FLEngine の `BitBlt` の直接の呼び出し元は 21 関数で、**その上流（直接 call の逆向きの閉包）は 3,482 関数（FLEngine の `.pdata` 57,816 関数の 6.0%）**。`SetDIBitsToDevice` は直接の呼び出し元 1 関数（`f_6bb1b0`）で、上流 3,332 関数。`CreateDIBSection` は直接 6 関数、上流 4,046 関数（7.0%）。`StretchDIBits` は直接 3 関数、上流 3 関数（0.0%） | 本調査の実測（静的な呼び出し関係） | 描画の「最後の転送」は少数の関数に集まり、そこに上流の 6〜7% が集まる。窓への出力は `BitBlt` と `SetDIBitsToDevice` が主 |
| E2 | `f_6bb1b0`（248 B）は 2 つの画像オブジェクトを取り、`[rcx+0x48]`（所有者）の `+0x496` の byte が非 0 なら **`SetDIBitsToDevice`**、0 なら **`BitBlt`**（rop = `0xcc0020` = SRCCOPY）を呼ぶ。ビットマップのハンドルは `[rax+0x424]`、DIB のビットは `[rcx+0xe0]`、寸法は `[rcx+0x80]`／`+0x84` | 本調査の実測（逆アセンブル）。意味は読み | 「ビットマップを窓の DC へ転送」の末端。**SRCCOPY の同形式 BitBlt なら決定 0111 の最小実装（DIB セクション上の画素コピー）で足りる形**だが、窓の DC（実際の画面）への転送は現状モデルに無い（メモリ DC 間のみ） |
| E3 | `f_6baef0`（484 B）は転送元と先の画像オブジェクトの `+0x70`（ビット深度）を比べ、**深度 32（0x20）と 24（0x18）の組み合わせごとに、4 つの関数ポインタ変数（`.bss` 0x15f3ed0／0x15f3ed8／0x15f3ef0／0x15f3ef8）を `call [..]` で呼ぶ**（ポインタを書くのは `f_6cc700` ほか）。**この 4 変数の指す先は静的に決まらない**（読み。`dsp_ippv2`（IPP の画像関数 `ippi*` 71 種を `GetProcAddress` で引く。トレースの `getprocaddress.tsv`）か、エンジン自身のコードか未確認） | 本調査の実測（呼び出し構造）。指す先は未確認 | 画素の合成（アルファ・ブレンド）は**エンジンと dsp_ippv2（実在モジュール）が CPU で行う**可能性が高い（読み）。a4 の完走でゲストは `ippi*` 71 名を `GetProcAddress` で引き、実在モジュールの VA が返っている（実測） |
| E4 | **`f_6cb0b0`（Blend2D の `AggCreateInstance` を呼ぶ唯一の関数）の直接の呼び出し元は 38 関数、その上流は 2,243 関数（3.9%）**。この 2,243 関数のうち **`BitBlt`／`SetDIBitsToDevice` の上流とも重なるものが 2,154 関数**。**`ExtTextOutW`／`DrawTextW`／`TextOutW` の上流（計 250 関数）と重なるのは 38 関数だけ** | 本調査の実測（静的な呼び出し関係） | 「Blend2D（`Agg2D` ラッパー）で描いてから BitBlt で出す」形は、描画系の少なくとも 2,154 関数に当てはまる（読み）。一方、GDI の文字描画（250 関数）は Blend2D の経路と**ほぼ別**（重なり 38）。**文字は GDI の `DrawTextW`／`ExtTextOutW` を直接使う経路が別にある**可能性が高い（読み） |
| E5 | GDI の文字描画の直接の使い手: `ExtTextOutW` は 6 関数（FLEngine。うち 2 は上流なし＝vtable／コールバック経由。`f_7fd4d0`／`f_7fd520`／`f_7fd580` は options = 2（ETO_OPAQUE?）を渡す薄いラッパー。`f_9f0d10` は 932 B の文字描画ルーチン）、`DrawTextW` は 17 関数（上流 219 関数）、`DrawTextExW` 1、`TextOutW` 1、`GetTextExtentPoint32W` 9（上流 21）。Delphi の `TCanvas` の `TextOut`／`TextRect`／`TextWidth`／`TextExtent` の名前が RTTI にある（本調査の実測: `TextOut` 2、`TextRect` 6、`TextWidth` 1、`TextExtent` 2。`TextHeight` は 102 件だがこれはフォームのリソース中のプロパティ名で、呼び出しの数ではない）。標準コントロールの文字は `TCanvas.TextOut` 経由と読む（未確認） | 本調査の実測（呼び出し関係と文字列）。VCL との対応は読み | **文字描画は GDI の文字 API を通る経路が実在する**（`ExtTextOutW`、`DrawTextW`）。この経路は**グリフのラスタライズを OS（GDI の文字・フォント）に委ねる** |
| E6 | QFC（`FontCache_AcquireFont` ほか 8 名を FLEngine が引く）の閉包（T2。530 関数）は **gdi32 を 1 つも使わない**（user32 も 12 名の窓・メッセージ系のみ）。QFC の `ExtTextOutW` の使い手 2 関数（`f_5138e0`、`f_513a00`）は export から届かない。QFC は FreeType（`FT_Load_Glyph`、`FT_Render_Glyph`、`FT_New_Face`、`FT_Set_Char_Size`、`FT_Get_Kerning` など 14 名を UTF-16 の名前で `GetProcAddress` する）を使う | 本調査の実測（静的）。FreeType の役割は読み | **QFC（フォントキャッシュ）は GDI ではなく FreeType でグリフを作る**。FLEngine の UI フォント（`ILGlyphsEx.ilfont`、`ILCursors.ilfont` は TrueType）はこちらを通る可能性が高い（読み。決定 0110 の `ILTools` のフォントの探索と整合）。**FreeType のグリフ描画（`FT_Render_Glyph`）は、実在モジュール freetype（lift 済み）が CPU で行うので、OS のフォントを使わない** |
| E7 | FLEngine の描画クラス名（Delphi の RTTI。length-prefixed の `T*`）: `TQuickFont`、`TGlyphCache`、`TRenderText`、`TQFHinting`、`TTextLayout`、`TDIB`／`TDIBmp`／`TDIBmpList`、`TQuickPaintBox`、`TVectorDigiPaint` ほか（本調査が文字列で確認。名前は 3,596 個の `T*` 識別子のうち）。**「`TGlyphCache`」「`TQFHinting`」が FLEngine 自身にある**＝FLEngine 自身がグリフのキャッシュとヒンティングを持つ | 本調査の実測（文字列）。役割は名前からの読み | UI の文字は**エンジン自身の文字描画（QFC＋FreeType→グリフ→自前の DIB へ合成）**を通る可能性。GDI の `ExtTextOutW`／`DrawTextW` は標準コントロール（`TCanvas.TextOut` など VCL 由来）が使う可能性。**どちらがメイン画面の文字に使われるかは未確認** |

**不確実性の整理（実験で潰す。§5）**:

1. メイン画面の文字（メニュー、ブラウザ、ミキサーのラベル、ピアノロールの数字）は **QFC＋FreeType（エンジン自前）** か **GDI の `ExtTextOutW`／`DrawTextW`** か、または両方か。
2. GDI の文字描画を実装する場合の規模: グリフのラスタライズ（フォント選択、ヒンティング、ClearType／グレースケール）は**大きなモデル**（§3.7）。ブラウザ側は canvas の `fillText`（フォントはブラウザの物。実機と見た目が一致しない）か、wasm の FreeType を OS フォント代わりに使う（`freetype_x64.dll` は実在モジュールとして既に lift 済み）。
3. 画素の合成の 4 つの関数ポインタ（`.bss` 0x15f3ed0 ほか）が dsp_ippv2 を指すなら、**描画の性能は dsp_ippv2 の lift 結果の wasm 実行速度で決まる**（`dsp_ippv2` は 39.9 MB で、a4 の完走で入られた関数は少数）。
4. Blend2D は **JIT（AsmJit）** を持つ。JIT のコードは lift の対象外で wasm では実行できない（`blend2d-lift-feasibility.md` §4.3。読み）。`Agg2D` ラッパーが JIT を無効にして描くかは**未確認**。無効にできなければ Blend2D の描画は動かない（**最大のブロッカー候補**）。

### 3.4 音声デバイスの経路（G7）: DirectSound／ASIO／WinMM のどれか（本調査の実測＝文字列と呼び出し関係。意味は読み）

| 項目 | 結果 | 区分 |
|---|---|---|
| WASAPI／MMDevice／XAudio／waveOut／Core Audio | **文字列も import も 0 件**（FLEngine） | 本調査の実測 |
| DirectSound | `DirectSoundCreate`（dsound の import 2 名のうち 1。`DirectSoundEnumerateA` も）。使い手は `f_12afb50` の 1 関数（文字列: "Couldn't create the DirectSound main object."／"…set the DirectSound cooperative level."／"…create the primary DirectSound buffer."／"…create the main streaming DirectSound buffer."／"Error while creating the notification events"）。クラスは `TMixerThread_DS`。UI の文字列に "DirectSound devices"、"DirectSound properties"、"Buffer length (DS)"。直接 call の経路: `CreateFruityInstance` ← `f_1375950` ← **`f_11efbf0`（`Flags` のビット 0 が立つときの別ループ）**← `f_1253610` ← `f_1253260` ← `f_12afb50`。**訂正（本調査の実測。先の版の「Flags = 0 では届かない」は不正確だった）**: `f_12afb50` と `f_1253610` は **`f_13751e0`（`CreateFruityInstance` が Run の前に呼ぶ初期化の本体。直接 call の閉包 6,449 関数）の閉包に入る**（`f_11b32a0` の閉包 1,267 関数と Run `f_86a660` の閉包 464 関数には入らない）。`f_1253610` の直接の呼び出し元は 10 関数（`f_c4eac0`、`f_d1af20`、`f_1144cc0`、`f_11c9e70`、`f_11d3340`、`f_11efbf0`、`f_1254890`、`f_127af50`、`f_13582c0`、`f_135b000`）。つまり **FL64.exe（`Flags` = 0）でも、Run の前の初期化の本体の下から DirectSound の初期化に届きうる**（静的に「届きうる」まで。条件分岐で通るかは未測定。設定のレジストリ値（`Software\Image-Line\FL Studio 26\DSTITF` などの有無）で分かれる可能性がある）。デバイスを実際に開く関数が起動時に呼ばれるのか、設定画面から呼ばれるのかは**未確認** | 実測（経路・文字列）、読み（経路の意味） |
| ASIO | クラス `TMixerThread_ASIO`、`TASIOBufferInfo`／`TASIOCallbacks`／`TASIOTime` ほか（ASIO SDK の型）、文字列 "ASIO: CreateBuffers failed with error %d"、"ASIO driver is not open."、"FL Studio ASIO"（`ILWASAPI2ASIO_x64.dll` として同梱。FLEngine に `ILWASAPI2ASIO` の文字列は無い）。ASIO ドライバーは COM（`CoCreateInstance`。FLEngine で 14 関数）で読み込むと読める | 実測（文字列・RTTI）、読み |
| WinMM | `winmm` は import 24 名（`timeGetTime` ほか時間 4 名＝行あり、`midiOut*` 7＝4 行あり、`midiIn*` 10＝3 行あり、**waveOut なし**）。MIDI は `midiOutOpen`（`f_13751e0` 以下に直接 call の経路）、`midiInOpen`（直接経路なし） | 実測 |
| スレッド | `TMixerThread`（音声のミキサースレッド）、`CreateThread` の直接の使い手は共通関数 `f_412b50`（呼び出し元 3 関数）。`TThread` 派生は 33 種（`browser-event-loop.md` §2 の引用。`sync-thread-survey.md`） | 引用・実測 |
| ブラウザ側の受け皿 | AudioWorklet（実時間の `AudioContext` が headless で `running`、48 kHz／44.1 kHz の 128 フレームのブロックを SAB リングで定常にアンダーラン 0 で供給できた。`browser-event-loop.md` §8。**実デバイスへの出力は未確認**）、Worker から SAB を共有するには COOP/COEP が要る（実測）。JSPI で wasm の待ちを中断できる（`jspi-real-wasm.md`） | 引用（実測） |

**読み（G7 の規模）**: エンジンの音声は **DirectSound（`TMixerThread_DS`）と ASIO（COM のドライバー）の 2 系統**で、WASAPI／waveOut は使わない。ブラウザで動かすには **DirectSound の最小モデル**（`DirectSoundCreate`、`IDirectSound::SetCooperativeLevel`、`CreateSoundBuffer`（プライマリとストリーミング）、`SetFormat`、`Lock`／`Unlock`、`Play`、`GetCurrentPosition`、通知イベント（`IDirectSoundNotify`））が必要で、これは **COM のオブジェクトを `IUnknown` の vtable で返すモデル**（`SHGetMalloc` の `IMalloc`（決定 0098）の形が先例。応答表の vtable 行）。DirectSound の再生位置の通知を、AudioWorklet の消費に結び付ける**リングバッファの橋**が要る（読み）。**DirectSound の呼び出し順序と、エンジンが期待するバッファ位置の進み方（実時間に対する）は未測定**で、ここがミキサースレッドの動作（`CreateThread` ＋ `WaitForMultipleObjects` ＋ 通知イベント）に結びつく。**スレッドが要るかは未確認**（`browser-event-loop.md` §2。協調スケジューラ（JSPI）で代替できるか、`pthread`（Worker＋共有メモリ）が要るかは未決）。

### 3.5 プロジェクトの読み込みの経路（G8）（本調査の実測＝呼び出し関係・文字列。意味は読み）

- プロジェクト形式 `.flp` は `FLhd`／`FLdt` のチャンク形式（既知の形式。`m2-inventory.md` §2.4 の `.fst`／`.fsc` の先頭も `FLhd`）。エンジンの `FLhd` の使い手（即値 `0x64684c46` のある関数）は `f_f54900`（860 B。`ExitProcess`／`MessageBoxA`／`WriteFile`／`GetStdHandle` を使う小さな CRT 的関数。コマンドラインの出力？）、`f_f54d70`、**`f_f51fd0`（8,672 B。文字列に `FLhd`、`FLdt`、"Project format is not compatible with target, aborting"、"This project contains invalid data and may cause the program to crash."、`SHFolder.dll`、`SHGetFolderPathW`、`MachineGuid`。本調査の実測）**、`f_aab5a0`（1,125 B。`CreateFileW`、`ReadFile`、`MapViewOfFile`、`GetTempPathW`、`LoadLibraryW`、`SetClipboardData`）。`f_f51fd0` は **FLP の `FLhd`／`FLdt` チャンクを解析する読み込み関数（`TFLProject` 系）と読む**（文字列による。未確認）。その直接 call の閉包（5,217 関数、import 287 名）が大きい（`CreateWindowExW`、`DirectSoundCreate`、`BitBlt`、`CreateThread` を含む）のは、**プロジェクトを開くとプラグインの生成・UI の更新・ミキサーの構築まで 1 回の呼び出しで届く**ことを示す（読み。静的には「届きうる」まで）。**`CreateFruityInstance` の直接 call の経路が存在する**（本調査の実測）: `CreateFruityInstance` → `f_13751e0` → `f_11be0a0` → `f_11a57c0` → `f_e90420` → `f_eae4c0` → `f_f54760` → `f_f51fd0`（起動時に最後のプロジェクトを読み込む、または自動保存の復元の経路と読む。「`CrashLoadingStartupProject`」の文字列が `f_11ef180` の周辺にある（`host-engine-boundary.md` §6.3）。**起動時にプロジェクトを開く条件は未確認**）。
- ファイルを開く直接の使い手: `CreateFileW` 14 関数（上流 4,764 関数）、`ReadFile` 3 関数（上流 2,200）、`WriteFile` 5 関数（上流 36,927。多くは CRT 的な標準出力の書き込みと思われる）、`CreateFileMappingW` 7 関数（上流 3,132）、`FindFirstFileW` 6 関数（上流 15,355。Delphi の RTL の `FindFirst` 経由）。**`GetOpenFileNameW`／`GetSaveFileNameW`／`SHBrowseForFolderW` の直接の呼び出し元は 0 関数**（FLEngine の import にあるが、`lea` で関数ポインタとして取られる thunk のみ）。ダイアログは Delphi の `TOpenDialog`（Vista 以降の `TFileOpenDialog`＝COM の `IFileDialog`。RTTI に `TFileOpenDialog`、`TFileSaveDialog`、`TFavFileOpenDialog`、`TFileDialogWrapper` がある）経由と読む。**ファイルダイアログは COM（`CoCreateInstance`）の大きなモデル**（G10）。
- プロジェクトのクラス: `TFLProject`、`TFlpDataRecord`、`TLoadFlpAction`、`TOpenDemoProjectsTask`、`TProjectDataLoader`、`TSavedProjectInfo`（RTTI。本調査の実測）。デモ曲は `Data\Demo projects`（UTF-16 の文字列 8 件）。インストーラの `.flp` は **165 エントリ（一意 161、87.2 MB）**（`m2-inventory.md` §2.1）。
- **読み**: プロジェクトの読み込みは「ファイルを `CreateFileW`／`ReadFile`（または `CreateFileMappingW`）で読み、FLP のチャンクを解析し、プラグインを `LoadLibraryW` + `CreatePlugInstance` で生成する」の経路で、API としては**既存の応答表の範囲（ファイル I/O、仮想ボリューム）に収まる**見込み（行あり: `CreateFileW`、`ReadFile`、`CreateFileMappingW`、`MapViewOfFile`、`FindFirstFileW` などは行がある。ただし中の名前付き停止の有無は未確認）。**重いのは API ではなく、`.flp` を開いた結果としてエンジンが走らせるコード全体**（プラグイン 160 個の読み込み、サンプル（WavPack、ogg、flac）の復号、ミキサーの構築）。サンプルの復号は同梱 DLL（`wavpackdll_x64`、`oggio_x64`、`ildsp_x64`。**未 lift**。bundled-dll-inventory §6.2）が要る（読み）。

### 3.6 ウィンドウとメッセージの仮想化（G2、G3、G6）: 既にある範囲と、足りない範囲（本調査の実測＝ソースの行数と表。意味は読み）

| 項目 | 現状（HEAD。`lift/runtime/`） | 区分 |
|---|---|---|
| 窓表 | `fle_win.c` 300 行（決定 0071）: `CreateWindowExW`（**1 つの形だけ**: WS_EX_TOOLWINDOW、WS_POPUP、サイズ 0、親なし、ほかは名前付き停止）、`SetWindowLongPtrW(GWLP_WNDPROC)`、`DefWindowProcW`（NCCREATE／NCCALCSIZE／CREATE だけ）。窓は削除されない。`GetWindowLongPtrW`（FLEngine で 66 地点）、`DestroyWindow`、`SendMessageW`（216 地点）、`PostMessageW`（139 地点）、`ShowWindow`（18 関数）、メッセージキュー、`GetMessageW`／`PeekMessageW`／`DispatchMessageW`、タイマーは**どれも無い** | 実測（ソース、`window-design.md` §8 の見積もり） |
| クラス表 | `fle_class.c` 225 行（決定 0070）: ゲスト自身のクラスの `RegisterClassW`／`GetClassInfoW` | 実測 |
| ゲスト関数の呼び出し | `fle_callback.c` 70 行（決定 0064）: `fle_call_guest(va, rcx, rdx, r8, r9)`。入れ子 16 段まで。callee-saved の検査つき | 実測 |
| **WndProc がゲストコードではなく生成コード** | Delphi の `MakeObjectInstance`（`f_52a5b0`）は `VirtualAlloc(PAGE_EXECUTE_READWRITE)` したメモリに **`E8 rel32` と `pop r11; jmp [rip+0]` の機械語を書き**、そのアドレスを `SetWindowLongPtrW(GWL_WNDPROC)` で登録する（決定 0071、`window-design.md` §3.3。**実測**）。**この種の WndProc にメッセージを送るには、生成コードの形を認識して解釈するか、実行可能メモリの機械語を実行する手段が要る**（wasm にはその手段が無い）。`window-design.md` §8 の見積もり: 「トランポリンの解釈は中」（形の認識と共通スタブ `0x52a570` の呼び出し） | 実測（形）、読み（規模） |
| 主ウィンドウ | **未着手**。FLEngine は `CreateWindowExW` を 8 関数が呼ぶ（本調査の実測。`f_425790` ほか）。クラス `TApplication`（`LoadIconW(HInstance, "MAINICON")`、決定 0069 で NULL を返す）、`TTimerWindow`、`TILRemoteWindow`、`TLightweightLayeredControl`（`WS_EX_LAYERED`）、MDI クライアントなど | 実測（呼び出し元の数）、読み |
| 規模の見積もり（読み。`window-design.md` §8 に既存） | `GetWindowLongPtrW` + USERDATA は小、`DestroyWindow` は中（WM_DESTROY／NCDESTROY、子と所有ウィンドウ）、**メッセージキューとループは大（タイマー、WM_QUIT、スレッド、再入）**、トランポリンの解釈は中 | 引用（読み） |

**読み（規模）**: G2〜G3 に要るのは、(a) 窓表の拡張（可視の最上位ウィンドウ、親子、Z 順、フォーカス、キャプチャ、矩形、表示状態。FLEngine の user32 行なし 94 名のうち窓の管理が約 50 名）、(b) **スレッドごとのメッセージキュー**（`PostMessage`／`SendMessage`（同一スレッドは直接呼び出し、他スレッドは配送）／入力キュー／`WM_PAINT`（無効領域から合成）／`WM_TIMER`（`SetTimer`）／`WM_QUIT`）、(c) `PeekMessageW`／`GetMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects` の待ちを **JS のイベントループに戻す仕組み**（JSPI。実測済みで、a4 の経路の 1,007,942 回の全関数入口で中断しても完走。**ただし実エンジンのメッセージループでの深さと中断は未測定**）、(d) 生成 WndProc（`MakeObjectInstance`）の解釈、(e) モーダルのネストしたループ（`MessageBox`、`TrackPopupMenu`、`DoDragDrop`。FLEngine 内で 10＋2＋2 地点）。**API の数は 100 前後だが、本質は「ウィンドウマネージャの状態機械とメッセージ配送の意味論」で、行数の見積もりは数千行規模の読み**（`fle_win.c` 300 行と `fle_gdi.c` 2,494 行の比から。根拠は弱い）。

### 3.7 既に答えがある／名前付き停止／行が無い の件数（本調査の実測＝突き合わせ。分類は決定 0111 の区分表による）

FLEngine の import（通常 738 + 遅延 57 = 795）を応答表 445 行（HEAD）と照合（`goal-ladder/fle-imports-cat.tsv`）:

| 区分 | 件数 | 出所 |
|---|---:|---|
| 行あり（応答表に (DLL, 名前) がある） | 371 | 本調査の実測（`imports_vs_table.py`） |
| 行なし（呼ぶと `unsupported instruction at …` で名前付き停止） | 424 | 同上 |

gdi32 と user32 だけを、決定 0111 の分類（`lift/out-git/g1-results/gdi-bulk-classification.tsv`）で分ける（本調査の実測。分類は決定 0111 の作業者の読み。**FLEngine の import だけ**を数える）:

| DLL | 行あり・答える（A／B／既存） | 行あり・**名前付き停止**（区分 C） | 行なし | 計 |
|---|---:|---:|---:|---:|
| gdi32 | 45 | **59** | 23 | 127 |
| user32 | 38 | 10 | 193（うち遅延 15 を含む） | 242（通常 227 + 遅延 15） |

（user32 は「行あり・分類外」1 を含め 242。）**gdi32 の 59 の名前付き停止には描画の実体が含まれる**: `ExtTextOutW`、`GetTextExtentPoint32W`、`GetTextExtentPointW`、`GetTextMetricsW`、`StretchBlt`、`StretchDIBits`、`SetDIBits`、`SetDIBitsToDevice`、`CreateDIBitmap`、`GetBitmapBits`、`GetPixel`、`SetPixel`、`SetPixelV`、`MaskBlt`、`LineTo`、`Polyline`、`Polygon`、`Rectangle`、`RoundRect`、`Ellipse`、`Arc`、`Pie`、`Chord`、領域・クリップ（`SelectClipRgn`、`IntersectClipRect`、`CombineRgn` ほか 20 名超）、メタファイル 11。user32 の 10 は `DrawTextW`、`DrawTextExW`、`DrawIcon`、`DrawIconEx`、`DrawEdge`、`DrawFocusRect`、`DrawFrameControl`、`FrameRect`、`CreateIcon`、`LoadBitmapW`。

**答える側の中身（読み）**: `BitBlt`／`PatBlt`／`FillRect` は**同形式の DIB セクション間の SRCCOPY ／ソリッド・ブラシだけ**答える（決定 0111 D16）。DDB（画素を持たない）、他のビット数、クリップ領域、他の rop、**画面 DC（`GetDC`）への転送**は名前付き停止。つまり「ビットマップへ書いて BitBlt するだけ」の形のうち、**窓の DC への最終の転送（E2 の `BitBlt`／`SetDIBitsToDevice` の宛先）は現状のモデルに無い**。

## 3.8 大きなモデルが要るもの（一覧と規模。**規模は読みで、実測ではない**。根拠が無いものは数字にしない）

各行の「規模」は、既存の同種の実装の行数（実測）か、`window-design.md` §8 の見積もり（読み）を根拠に、桁だけを書く。**日数・人月には換算しない**（根拠が無い）。

| # | モデル | 必要な段階 | 現状（実測） | 規模の根拠（読み） | ブラウザ側の対応 |
|---|---|---|---|---|---|
| M-1 | **ウィンドウマネージャとメッセージキュー**（窓表の拡張、親子・Z 順・フォーカス・キャプチャ、`SendMessage`／`PostMessage`、`WM_PAINT`／`WM_TIMER`／`WM_QUIT`、入力、モーダルのネスト、生成 WndProc の解釈） | G2、G3、G6 | `fle_win.c` 300 行（1 形だけ） | 行なし user32 94 名のうち窓・メッセージ・入力が大半。`window-design.md` §8: メッセージキューとループは「大」。`fle_gdi.c`（2,494 行）より大きくなる見込み（読み。根拠は弱い） | 窓 = 1 枚の canvas（OffscreenCanvas）＋ JS のイベント（キー・マウス・ホイール・リサイズ）を `WM_*` に変換。待ちは JSPI（実測: 全関数入口の中断で a4 が完走） |
| M-2 | **ブロッキングのメッセージループの JS イベントループへの接続**（G3） | G3 | 機構は a4 経路で実測。**実エンジンのループは未測定** | `browser-event-loop.md`: Worker＋`Atomics.wait`（M-A、往復 75–90 µs）、Asyncify（コード ×1.85。実 wasm への外挿 722 MB で V8 の 1 GiB の 67%。**未実測**）、JSPI（`jspi-real-wasm.md`: SJLJ=wasm の再ビルドが必要。**JSPI のスタック上限 150 kB で通り、145 kB で失敗**。実エンジンのループの深さは未測定）。**描画との関係**: M-A（戻らない Worker）では OffscreenCanvas の自動表示が効かず、`ImageBitmap` の `postMessage` が要る | 引用（実測） |
| M-3 | **GDI のグリフのラスタライズとフォント**（`ExtTextOutW`、`DrawTextW`、`GetTextMetricsW`、`GetTextExtentPoint32W`、`CreateFontIndirectW`／`EnumFontFamiliesExW`、ヒンティング、ClearType） | G4、G5 | フォント作成・列挙は仮の答え（決定 0066/0098）。文字描画と計測は名前付き停止 | **メイン UI の文字が GDI を通るか FreeType（QFC）を通るかが未確認**（§3.3 の E5〜E7）。GDI を通るなら、OS のフォントの形状と寸法を再現する必要があり、**実機と画素が一致しない**。FreeType 側は freetype 実在モジュール（lift 済み、a4 で DllMain まで）が既に実装の本体で、フォントファイルがあれば再現できる | canvas の `fillText`（ブラウザのフォント。見た目は実機と別物）か、wasm の FreeType による自前のラスタライズ（実機の GDI とも一致しない）。**どちらも「実物と変わらず」とは別の問題** |
| M-4 | **画面 DC・ウィンドウ DC への転送、`WM_PAINT`、無効領域、クリップ領域**（領域 API 20 名超） | G4 | 画面 DC は固定値のみ（決定 0053）。メモリ DC 間の転送だけ（決定 0108/0111）。`BeginPaint`／`EndPaint`／`InvalidateRect` は行なし | 窓表（M-1）と同時に作る（読み） | 窓のバックバッファ（DIB）を canvas に `putImageData`（実測: 1920×1080 で CPU 側 0.22 ms／回） |
| M-5 | **音声デバイス: DirectSound（と ASIO の COM）の最小モデル、再生位置と通知イベント、ミキサースレッド** | G7、G8 | 行なし（`DirectSoundCreate`、`CoCreateInstance`）。スレッドは `CreateThread` 行なし（`sync-thread-survey.md` の案 A1（JSPI の協調スケジューラ）、C（fail-closed）、D（pthreads）） | COM のオブジェクト（vtable）を返すモデルは `SHGetMalloc`（決定 0098）が先例。**スレッドが要るか、通知の時間の進み方（実時間か仮想時計か）が未測定**。決定 0017 の「単一スレッド・仮想時計」と衝突する可能性（読み） | AudioWorklet（実測: 128 フレームのブロックを SAB リングで供給。定常アンダーラン 0）。**実デバイスへの出力は未確認** |
| M-6 | **スレッド**（`CreateThread`、`ResumeThread`、`WaitForMultipleObjects`、`SignalObjectAndWait`、タイマーキュー、TLS） | G3、G7、G8 | 1 本目のみ。2 本目の作成は名前付き停止（決定 0017） | `TThread` 派生 33 種。`sync-thread-survey.md` の案 A1／A2／C／D。協調スケジューラは JSPI（実測済みの機構）。pthreads は全オブジェクトの再ビルドと `R`/`X`/`FL` の thread-local 化（引用）で重い | JSPI（推奨は同文書。決めない）または Worker＋共有メモリ（COOP/COEP が要る。実測: 取得可） |
| M-7 | **COM とシェル**（`CoCreateInstance` 14 関数、`IFileDialog`、`IShellItem`、PROPVARIANT、ASIO ドライバー、`ShellExecute`） | G8（ファイルを開く）、G10 | `SHGetMalloc` の `IMalloc` のみ（決定 0098）。`CoCreateInstance` は行なし | 開くダイアログは COM の `IFileDialog` と読む（RTTI に `TFileOpenDialog`）。**ブラウザの File System Access API／`<input type=file>` に写す設計が要る**（ダイアログは同期 API で、JSPI で待つ） | `showOpenFilePicker`／`<input type=file>`、OPFS（実測: 書き込み可、`createSyncAccessHandle` は Worker のみ） |
| M-8 | **プラグインのロードと実行**（`LoadLibraryW` の実在モジュールの一般化、プラグイン 160 個。Delphi 製 132）| G9 | 実在モジュールは 5 つ（`RM_MAX` 8、基底は測定用）。プラグイン DLL は未 lift | `m2-inventory.md`: 共有なしで 2.9〜3.8 GB の wasm（見積もり）、単一 wasm の V8 上限は約 1 GiB。既存 3 モジュールと 89〜99% が共有（下限の測定。共有できるかは未検討） | 動的リンク（複数 wasm モジュールと共有 import）か、単一モジュールに統合するかの設計が要る。**配信量（圧縮後 170 MB 級）** |
| M-9 | **ファイル I/O とサンプル／プロジェクトの供給**（仮想ボリュームにインストールの内容（2.4 GB）をどう置くか） | G8 | 仮想ボリュームは空で始まる（決定 0018）。フォント 2 ファイルのみ種付け（決定 0110） | インストーラの内容を OPFS／MEMFS に置く（配信とキャッシュの設計。製品ファイルをリポジトリに入れない制約）。`FindFirstFileW` の列挙（15,355 関数が上流）に答える必要 | OPFS（quota 4.3 GB（実測、プロファイルが tmpfs のとき）） |
| M-10 | **画素の合成・DSP を担う dsp_ippv2（実在モジュール）と、Blend2D の JIT** | G4〜G8 | dsp_ippv2 は DllMain まで（a4 で 15 関数が locals 超過だが入られない）、Blend2D は DllMain まで（`AggCreateInstance` の実呼び出しは未実行） | **Blend2D の JIT（AsmJit）は lift できず wasm で実行不能**（`blend2d-lift-feasibility.md` §4.3。読み）。JIT を無効化する経路があるかが**未確認**。a4 の完走で実行可能メモリの確保は `MakeObjectInstance` の 2 回のみ（本調査の実測。トレース）で、JIT は起動時には使われない | JIT が必須なら、(a) Blend2D を JIT 無効でビルドし直した別物に置換（「実物と変わらず」から外れる）、(b) JIT の出力を解釈実行する仕組み（大）、(c) 描画を別実装（canvas）に置換 |

**読み（全体）**: M-1〜M-3 が G2〜G5 を決める。M-2（ループの接続）は機構が実測済みで、**最も「実測の裏づけがある」大きなモデル**。M-3（文字）と M-10（Blend2D の JIT）は**実機と同じ画素になるかを決める不確実性**で、ゴールの「実物と変わらず」に直結する。M-5〜M-6（音声とスレッド）は G7 以降の前提で、`sync-thread-survey.md` の判断（A1 を推奨）が未決。

### 3.9 G1 の現在地の更新（引用。他の作業者の未コミットの記録 `lift/out-fruity5/wall-log.md` を読み取りで読んだ。決定 0118 の作業中で、**HEAD にもコミット済みの決定にも無い**。数値は再測定していない）

- 壁 6（CPUID）: EDX の 5 ビット（FPU、CX8、CMOV、MMX、FXSR）を足すと a4 の functions が 5,306 → 5,308、entries が 1,007,942 → 1,007,944（原因は FXSR の 1 ビットだけ）。CPU 検査は通る。壁 2（`GetModuleFileNameW(NULL)` = FL64.exe）も実験（実測）。
- 壁 7（`rpcrt4!UuidCreate`）と壁 8（`LoadIconW(HInstance, "MAINICON")` の資源読み取り）は実装された（ワークツリー。HEAD に無い）。**38,913 関数入口目で `user32!SendMessageW(hWnd = NULL, WM_SETICON, …)`（`TApplication` の `UpdateIcon`）で止まる**（壁 9。**ここで作業者が止めた**: 出典なしの値を決める必要があるため）。
- 実験 E5（NULL の hWnd への `SendMessageW`／`SetClassLongPtrW` が 0 と last error 1400 を返す）を足すと、次の停止は壁 10: **`CreateWindowExW("TApplication", "FL Studio", dwStyle 0x84ca0000, X・Y = 画面の半分, 幅・高さ = 0)`**（`f_867e10`。**38,928 入口**）。決定 0071 の「隠しの WS_POPUP のツールウィンドウだけ」の範囲外で、作業者は「**停止条件 (iii)（大きなモデル）に近い**。ウィンドウ表の拡張で足りるのか、メッセージキューまで要るのかは未調査」と書いた。
- **G1 の「Run の入口（0x1375950）」までの距離（本調査の実測＝`CreateFruityInstance`（0x1375990、3,213 B）の `call` 命令をアドレス順に並べた）**: `CreateFruityInstance` の本体は次の順で呼ぶ（括弧内は本調査の読み）: …`f_4256d0`（WinVerifyTrust）×4 とその間の crypt32 → `f_1264050`（0x1376364。パス設定。決定 0116 の壁 1〜3）→ `f_86a870` → `f_1375230` → `f_13754b0` → **`f_13755b0`（0x1376409。事前検査。CPU 検査＝壁 6）** → **`f_11b32a0`（0x1376459。BasicInitialization）** → `f_86ccd0` → `f_869c90` → `f_869c10` → `f_4250f0`（LoadIconW の thunk 周辺）→ **`f_5d03e0`（0x13764ff。アイコン）** → **`f_867e10`（0x137650e。`TApplication` のウィンドウの作成＝壁 9・10）** → `f_86ccd0` → `f_86a440` → `f_423440`、`f_423be0` → **`f_13751e0`（0x1376545。初期化の本体。閉包 6,449 関数。プロジェクト・音声デバイス列挙・UI の構築が入る）** → **`f_1375950`（0x1376573。Run）**。したがって **Run の入口までに、(1) `f_867e10` の窓の作成（M-1 の入口）、(2) `f_13751e0` の 6,449 関数の閉包（import 350、行なし 114）の実行、が両方残っている**。(2) の閉包に `BeginPaint`（`f_135d220` 経由）、`midiOutOpen`、`ReadFile`、`InternetOpenW`、`f_f51fd0`（FLP 読み込み）が入る。**G1（Run の入口に着く）は、G2（窓）の一部と G4 の描画の一部（`BeginPaint` を含む経路が `f_13751e0` の下にある）を先に要求する可能性が高い**（読み。**到達は実行時の分岐に依存し、未測定**）。実行された関数入口は 38,928 点で、静的な閉包（6,591 関数）に対して、`f_13751e0` の下はまだ 1 点も入っていない（実測: 停止は `f_13751e0` より前）。

### 3.10 各段階の距離（まとめ。各セルに「実測／読み／未確認」と根拠の強さを付ける。**日数は書かない**）

「必要な API」は §3.2〜3.7 の静的な到達（下限）に基づく。「答えがある／名前付き停止／行が無い」の数は、その段階の経路（直接 call の閉包。下限）に入る FLEngine の import について、HEAD の応答表（445 行）との照合（本調査の実測）。**到達の網羅性は保証されない**（vtable 経由は含まない）。

| 段階 | 必要な API・モデル（根拠） | 経路の import の件数（行あり／行なし。静的下限） | 大きなモデル（§3.8） | 根拠の強さ |
|---|---|---|---|---|
| G0 | 達成済み | a4 の完走で 171 種 13,572 回が応答（実測。仮 47 件の決定を含む） | — | **実測**（独立再現）。ブラウザは headless Chrome 156 で一致 |
| G1 | 事前検査（署名: 決定 0117 の主張の表で通過。CPU: 決定 0118 の最中）、`TApplication` の設定（`LoadIconW` 資源: ワークツリーで実装済み・未コミット）、**`f_867e10` の窓の作成（M-1 の入口。壁 10）**、`f_13751e0`（初期化の本体） | `CreateFruityInstance` の直接 call の閉包: 行あり 252／行なし 127（T1）。`f_13751e0`: 236／114。`f_11b32a0`: 126／27 | M-1（窓表）。`f_13751e0` の下で `BeginPaint`（M-4）、`midiOutOpen`、`ReadFile`、`InternetOpenW`、`CreateThread` に届きうる | 実測（停止点まで）＋読み（その先は静的下限）。**`f_13751e0` の先は 1 点も実行されていない** |
| G2 | 窓表の拡張: 可視の最上位（スタイル 0x84ca0000、位置は画面の半分）、`ShowWindow`、`SetWindowPos`、`GetWindowRect`、フォーカス・アクティブ、DPI（`SetThreadDpiAwarenessContext` ほか遅延）、モニタ（`MonitorFromWindow` ほか行なし） | user32 の窓管理で行なし約 50 名（`fle-imports-cat.tsv` の window-msg 分類で行なし 77 の大半） | M-1 | 読み。規模の根拠は弱い |
| G3 | `Run`（`f_86a660`）の直接 call の閉包 464 関数、import 96 名（行あり 53／行なし 43）。`PeekMessageW`／`TranslateMessage`／`DispatchMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects`／`SetTimer`、`SendMessageW`／`PostMessageW`、メッセージキュー、生成 WndProc（`MakeObjectInstance`）の解釈 | 行なし 43（Run の閉包） | M-1、M-2、M-6（アイドルで `CreateThread` に届く: `f_86b440` ← … ← `f_412b50`） | 実測（呼び出し関係）＋引用（待ちの機構: JS 試作 M-A／M-B と a4 経路の JSPI は実測）。**実エンジンのループは未実行** |
| G4 | `WM_PAINT`、`BeginPaint`／`EndPaint`／`InvalidateRect`、画面 DC・窓の DC への `BitBlt`／`SetDIBitsToDevice`、メモリ DC の描画の実体（DIB セクション、`CreateDIBSection`、領域・クリップ）、`StretchBlt`／`StretchDIBits`（名前付き停止中）、Blend2D（`AggCreateInstance`、**JIT**）、dsp_ippv2（`ippi*` の 71 関数。画素の合成の可能性）、文字（§3.3: GDI か FreeType か **未確認**） | 描画系の gdi32: 行あり・答える 45／行あり・名前付き停止 59／行なし 23（FLEngine 全体）。BitBlt の上流 3,482 関数、CreateDIBSection の上流 4,046 関数、Agg2D の上流 2,243 関数（静的） | M-3、M-4、M-10 | **読み**（描画の実体は未実行）。「ビットマップへ書いて BitBlt するだけ」は**部分的に支持される**が確定しない（§3.3） |
| G5 | G4 に加えて、フォント（FreeType による UI フォント：`ILGlyphsEx.ilfont`、`ILCursors.ilfont` は a4 で仮想ボリュームに置いた）、アイコン・カーソル資源（`LoadIconW`／`LoadImageW`／`GetIconInfo`: ワークツリー）、メニュー（`CreatePopupMenu` ほか行なし 29）、テーマ・DPI、レイアウト窓（`TLightweightLayeredControl` の `WS_EX_LAYERED`／`UpdateLayeredWindow`）、**実機のスクリーンショット（未入手）** | user32 のメニュー・描画・テーマの行なしが多い | M-3、M-1 | **未確認**（比較対象が無い） |
| G6 | マウス・キー・ホイール → `WM_*`、`SetCapture`／`GetCursorPos`／`GetKeyState`、キャレット、IME、ドラッグ（`DoDragDrop`＝COM/OLE）、クリップボード（`OpenClipboard` ほか） | input 分類で行なし 30、メニュー・ダイアログで行なし 29 | M-1、M-7（OLE ドラッグ） | 読み。ブラウザ側の入力の往復遅延は実測（75–90 µs、`browser-event-loop.md` §3、§9） |
| G7 | DirectSound の最小モデル（COM の vtable）、再生位置と通知イベント、ミキサースレッド、`timeBeginPeriod`、WinMM の MIDI（`midiOutOpen` ほか）、`CoCreateInstance`（ASIO は `ILWASAPI2ASIO` ドライバー） | `DirectSoundCreate`／`DirectSoundEnumerateA` 行なし、`CreateThread`／`WaitForMultipleObjects` 行なし | M-5、M-6、M-7 | 実測（API の所在）＋読み（モデルの形）。**スレッドの要否が未確認** |
| G8 | G7 に加えて、ファイル I/O（`CreateFileW`／`ReadFile`／`CreateFileMappingW`／`FindFirstFileW`）、FLP の解析（`f_f51fd0`）、サンプルの復号（`wavpackdll`、`oggio`、`ildsp`＝**未 lift の同梱 DLL**）、内蔵の音源プラグイン（G9 の一部）、仮想ボリュームへのデータの供給（デモ 87 MB、`Data\Patches` 数百 MB）、`ShellExecute`／ファイルダイアログ（COM） | ファイル I/O は行あり 38／行なし 7（`GetDiskFreeSpaceW`、`GetFileInformationByHandle`、`SetEndOfFile`、`OpenFileMappingW`、`SHGetKnownFolderPath` ほか） | M-7、M-9、M-8（G9 と重なる）、M-10（dsp_ippv2 の DSP） | 読み＋実測（所在）。**プロジェクトを開いたとき実際に走るコードの範囲は未測定** |
| G9 | `LoadLibraryW`＋`CreatePlugInstance`、同梱プラグイン 160（Delphi 製 132、MSVC 製 27）、プラグインの UI（G4〜G6 と同じ描画経路）、`TFruityPlugHost` の vtable | プラグイン DLL の import は未集計（`m2-inventory` は 313 個の PE の import を表にしている） | M-8（wasm の大きさ: 共有なし 2.9〜3.8 GB の見積もり。単一 wasm の V8 上限 1 GiB） | 読み（m2-inventory の見積もり。新しい DLL に `fe-lift` は未実行） |
| G10 | 設定・ダイアログ・モーダル（`MessageBoxW`、`TrackPopupMenu`、`DoDragDrop`）、保存・書き出し（WAV／MP3＝`LAMEenc_x64`、WavPack、ogg）、ネットワーク（wininet）、`CreateProcess`（ブリッジ）、Python（`python312.dll`＝MSVC 製で 7 MB） | 行なしの wininet 15、shell32 19、comctl32 30 ほか | M-1、M-7、M-8 | 読み。ほぼ未調査 |
| G11 | VST2／VST3／CLAP（VST3 ホストは QFC の `CreateVST3Instance`）、ライセンス（`licence-engine\LicenceEngine.dll`、`MachineGuid`、署名）、オンライン（FL Cloud、更新確認）、ブリッジ（`ILMinihostBridge64.exe` などの別プロセス）、x86 のプラグイン（**x64 の lift の対象外**。FLEngine の他に 32 ビットの DLL がある: `shared-x86-counterpart`、LilyPond、AudioRestore など） | — | M-8、M-7 | 読み（棚卸しのみ） |

## 4. 段階の依存関係（最短経路）と不確実性

### 4.1 依存関係（読み。「先に要る」の向き。実測の裏づけがあるものは根拠を書く）

| 段階 | 先に要る段階・モデル | 依存の根拠 |
|---|---|---|
| G0 | — | 達成 |
| G1 | G0、M-1 の窓の作成（G2 の入口）、`f_13751e0` の下の API | `CreateFruityInstance` の呼び出し順（本調査の実測、§3.9）: `f_867e10`（窓の作成）が `f_13751e0`（初期化の本体）と Run の前にある |
| G2 | G1 の途中、M-1 | 同上。作業者の記録（引用）も、壁 10 を「大きなモデルに近い」と書いた |
| G3 | G2、M-1 のメッセージキュー、M-2（待ちの接続）、生成 WndProc の解釈 | Run（`f_86a660`）の閉包に `PeekMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects`／`SetTimer` がある（実測） |
| G4 | G3（`WM_PAINT` を受ける）、M-4、M-10（Blend2D の JIT・dsp_ippv2）、M-3 の一部 | `BeginPaint` の経路は初期化の本体の下にもある（実測）ので G1 の途中で最初の描画に当たる可能性がある（読み） |
| G5 | G4、M-3（文字）、アイコン・メニュー・テーマ、**実機のスクリーンショット** | — |
| G6 | G3、G4 | 入力は `WM_*` として届き、結果が描画に出る（読み） |
| G7 | G1（`f_13751e0` を通る）、M-5、M-6、M-7（COM） | `f_1253610`／`f_12afb50` は初期化の本体の閉包に入る（実測） |
| G8 | G4、G7、M-9、M-8 の一部（内蔵プラグイン）、サンプルの復号（未 lift の同梱 DLL） | — |
| G9 | G8、M-8 | — |
| G10 | G6、G8 | — |
| G11 | G9、M-7、ライセンス | — |

**図（表）**: `G0 → [M-1 窓の作成] → G1/G2 → [M-1 キュー、M-2、M-6] → G3 → [M-4、M-10、M-3] → G4 → G5`、`G3 → G6`、`G1 → [M-5、M-6、M-7] → G7`、`G4 + G7 → [M-9、サンプル DLL] → G8 → [M-8] → G9 → G10 → G11`。

**クリティカルパス（読み）**: G0 → G1/G2（窓の作成）→ G3（メッセージループ）→ G4（最初のフレーム）。**G5 以降と G7 以降は G4 と G1 を過ぎてから分岐する**。G1 の前に「描画（BeginPaint を含む経路）」と「音声（DirectSound の初期化）」に届く可能性があるので、**G1 の完了（Run の入口に着く）は、実測してみるまで G2・G4・G7 の一部と切り離せない**。したがって、G1 を「Run の入口」で厳密に定義すると、そこへ至る道で M-1 の窓の作成と `f_13751e0` の API（行なし 114）に当たる。

### 4.2 最も不確実な項目（5 件。不確実性の大きい順ではなく「ゴールを左右する度合い」の順。理由と、確実にするために要るものを付ける）

| # | 不確実な項目 | なぜ不確実か（根拠） | ゴールに対する影響 |
|---|---|---|---|
| U1 | **Blend2D の JIT と、描画の本体が Blend2D／dsp_ippv2／GDI のどれか** | JIT は lift できず wasm で実行不能（`blend2d-lift-feasibility.md` §4.3。読み）。`Agg2D` ラッパーが JIT を無効にして描くかは未確認。FLEngine の `f_6cb0b0` は NULL を検査せず vtable を呼ぶ（B10）。Blend2D の上流 2,243 関数と BitBlt の上流の重なり 2,154（読み）。a4 の完走に実行可能メモリの確保は `MakeObjectInstance` の 2 回だけ（実測）で、JIT は起動では使われない | **JIT が必須なら G4 の画面描画の主要部が動かない**（「実物と変わらず」の前提が崩れる）。最大の致命的リスク候補 |
| U2 | **UI の文字は GDI（`ExtTextOutW`／`DrawTextW`）か FreeType（QFC）か** | GDI の文字の使い手 250 関数（上流）と Agg2D の上流の重なりは 38 関数だけ（実測）。QFC は gdi32 を使わず FreeType のみ（実測）。FLEngine は `TGlyphCache`、`TQFHinting` を持つ（実測）。どちらがメインの UI の文字を描くかは静的には決まらない | GDI なら M-3（OS フォントの再現。実機と画素が一致しない）が G4〜G5 の大きな壁。FreeType なら既存の実在モジュールで足りる可能性が高い |
| U3 | **`CreateFruityInstance` の初期化の本体（`f_13751e0`、閉包 6,449 関数）が実際にどの API を踏むか** | 静的な閉包は上界（行なし 114）。実行されたのは 38,928 入口で、`f_13751e0` の下は 0 点（実測）。K1 の指摘: 静的な数え方は、行があって名前付きで止まる壁を数えない。これまでの壁は 9 つ連続し、**どれも静的な数え方に現れなかった種類を含む**（決定 0116、0117 の「限界」） | G1〜G3 の距離そのもの。壁の数は見えていない。**壁を 1 つずつ踏む方式（0116〜0118）では、窓の領域に入ると 1 壁あたりの規模が大きくなる**（壁 10 が M-1 に当たった） |
| U4 | **ブロッキングのメッセージループと JSPI／スレッドの接続（実エンジンのループの深さ・待ち）** | 機構は a4 経路で実測（JSPI＋SJLJ=wasm、全入口の中断で一致）。**実エンジンのメッセージループは未実行**。JSPI のスタックは 150 kB で通り 145 kB で失敗（V8 既定 984 kB）。メッセージループの深さと、モーダル（`MessageBox`、`TrackPopupMenu`、`DoDragDrop`）のネストは未測定。DirectSound のミキサースレッドが要るか未確認 | G3、G7 の成立可否。スレッドが必要なら、JSPI の協調スケジューラ（案 A1）か pthreads（案 D。再ビルド）の判断が要る |
| U5 | **実機との同等性（画面・音）の比較手段が無い** | **API の応答については実機の測定が 1 回届いた**（引用: コミット 2229d99 の `worklog/1/`。Windows 11 Pro 10.0.26300（26H2）、ja-JP、ANSI 932、Hyper-V VM、リモートセッション推測。`win-replay` の 2,144 ケースを実行。`compare.py` の判定: match 352／mismatch 161／reference 55／real-only 1,559／skipped 17。PM の注記: 「測定した機械は Windows 11 26H2 ja-JP で、ランタイムが仮定する Windows 10 en-US ではない」）。**画面のスクリーンショット、FL Studio のレンダリング音、実機での FL Studio の実行記録は無い**。実機の画面・音が無いと G5・G8 の「実物と変わらず」の受け入れ条件を満たしたか判定できない | G5、G8、G10 の受け入れ。**作業では埋まらない**（依頼者側の資源）。API 応答の突き合わせは別軸（§6） |

次点（5 件に入れなかったが記録する）: (a) **配信量と V8 の上限**（プラグイン込みで wasm が 1 GiB を超える。m2-inventory）、(b) **未 lift の同梱 DLL（`ildsp`、`oggio`、`wavpackdll`、`ilvideo`、`speaker`、`python312`、`LAMEenc`）**（サンプルの復号と書き出し。G8、G10）、(c) **x86（32 ビット）のプラグイン・ブリッジ**（G11）、(d) **ライセンス検証の関数が不明**（host-engine-boundary U9）。

## 5. 不確実性を最も早く潰せる実験の提案（**実行していない**。PM が次の作業にする。優先順）

見積もりの「時間」は、既存の同種の測定の所要（実測）を根拠にした桁で、新しい作業の所要の予測ではない。RAM は `/tmp/opencode` の制約（quota 約 2.9 GB。`progress.md` の引用）に対する既存の測定の最大値を引用する。ファイルは新規に作る／変更するものの名前（読み）。**どの実験も「応答の値を創作して先へ進める」ことはせず、観測だけを目的にする**（実験変種の差分はコミットしない流儀。決定 0116／0117 の「実験（別枠）」と同じ）。

### X1（最優先）: Blend2D の JIT の有無と、描画の本体の特定 — 「`AggCreateInstance` を呼び出して、どの経路で画素が書かれるか」を gdb の読み取りで見る

- **何を潰すか**: U1（JIT）と U2（文字）。**描画が「Blend2D（`Agg2D`）→ DIB → BitBlt」で済むのか、JIT の機械語（lift 不能）を通るのか、GDI の文字 API を通るのか**。
- **やり方（実行を伴う。ビルドは既存のものを使う）**: 6 モジュール構成の native（FRUITY 無効。a4 の完走。既存の `a4-multi.sh`、約 1 分 15 秒、最大 RSS 約 2.7 GB。決定 0116 の測定の実測）を作り、**DllMain の完走後に、FLEngine のゲストの関数 `f_6cb0b0`（Agg2D を作る関数。直接の呼び出し元 38 関数）を `fle_call_guest` で直接呼ぶ**小さなハーネス拡張（FRUITY のハーネス `fruity.c` の流儀。`FLE_CALL_FRUITY` と同様の環境変数で opt-in。既定は不変）。`f_6cb0b0` は `rcx` に Agg 保持オブジェクトが要るので、**最小の代用として `f_6cb190`（`[rcx+0x58]` が 0 のとき `f_6cb0b0` を呼ぶ入口。呼び出し元 39 か所の代表）を、0 で埋めた 0x100 バイトのオブジェクトで呼ぶ**。その後、Agg2D の vtable の描画関数（塗りつぶし、線、文字）を 1 つずつ呼び、(a) `VirtualAlloc` の `PAGE_EXECUTE_*` が出るか（JIT の徴候。**既存のトレース機構（gdb の `fle_imp_*` ブレーク）だけで見える**）、(b) 出力のビットマップの画素が変わるか、を見る。
- **見積もり**: ハーネス拡張は `lift/harness/` の小さな C（`fruity.c` の約 770 行の流儀。新規は 200〜300 行と読む。根拠は弱い）。実行は 1 秒未満（DllMain 完走の後。a4 全体が 0.57 秒＝実測）。RSS は 約 190 MB（native の a4 の実測 187 MB）。ビルドは 1 回 約 1〜2 分、RSS 2.7 GB。
- **結果で計画がどう変わるか**:
  - JIT が使われない（`PAGE_EXECUTE_*` が出ず、画素が書かれる）→ **G4 は「ビットマップへ書いて BitBlt」の形で進められる**。U1 が消える。次は X2（文字）。
  - JIT が使われる → lift 可能な描画パスが存在しないことになる。計画は (a) Blend2D の JIT 無効ビルドでの置換（実物から外れる）、(b) JIT 出力の解釈実行、(c) 描画の別実装、のどれかの**決定**（PM）が先に要る。G4 以降の見積もりは、その決定まで書けない。
  - `AggCreateInstance` が NULL を返す／呼べない（B10 の NULL 検査なし）→ Blend2D の DllMain の副作用（TLS、`.xc` の初期化）を実在モジュールで通せているかの見直し。

### X2: メイン UI の文字が GDI／FreeType／Blend2D のどれを通るかを、ブレークポイントで数える

- **何を潰すか**: U2。
- **やり方**: X1 と同じ native ハーネスで、`CreateFruityInstance` を**窓の作成の壁（壁 10）まで進めた構成（`lift/out-fruity5/` の実験 E5 の差分。未コミット。その作業者のものなので、**新しい作業はそれがコミットされてから**）**で、gdb で `ExtTextOutW`／`DrawTextW`／`GetTextExtentPoint32W`／`FT_Render_Glyph`（QFC が引く freetype の関数）／Agg2D の vtable のテキスト関数 の `fle_imp_*`／実在モジュールの入口にブレークを置く。**「壁 10 の先の窓の領域」は未到達なので、このままでは何も数えられない可能性が高い**（`f_13751e0` の下の描画は実行されていない。実測）。**代替**: エンジン内の文字描画の関数（`f_9f0d10`＝ExtTextOutW の使い手 932 B、`f_61b3c0` 等 DrawTextW の使い手）を `fle_call_guest` で直接呼び、GDI の応答が出るか／出力ビットマップがどう変わるかを見る（呼べる最小のオブジェクトを作る必要があり、規模は不明）。
- **見積もり**: 窓の領域に届くまでは待ち（G1/G2 の作業）。代替案は X1 と同程度の規模（200〜300 行の読み。根拠は弱い）。
- **結果で計画がどう変わるか**: GDI の文字が主 → M-3（OS フォントの再現）が G4〜G5 の最大のモデルになり、**画素一致は目標から外す（見た目の近似）か、実機の GDI の出力（スクリーンショットとグリフのビットマップ）の入手が先**。FreeType が主 → 既存の実在モジュールで足り、フォントファイルの供給（既に決定 0110 で 2 つ）と `FT_Render_Glyph` の実行の確認が次。

### X3: 窓の作成と最初の `WM_PAINT` を、エンジンの WndProc に合成メッセージを送って観測する

- **何を潰すか**: U3（初期化の本体より後の壁の数と質）と G2〜G4 の最短経路。
- **やり方**: 実験用の差分（コミットしない）で、仮想窓表に**スタイル・位置・サイズの検査を外した「全部受け入れる」窓**（決定 0071 の範囲を越える。実験の別枠）を足し、`CreateWindowExW("TApplication", …)` を通し、`f_13751e0`／Run まで進める。メッセージキューは作らず、**`PeekMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects` に到達した時点で止め（FLE_FRUITY_STOP_AT。**既存**）、その時点までの実行関数数・踏んだ行なし API の一覧を採る**（次の壁を 1 つずつ踏むのではなく、**行なし API を応答なしで「記録して続行」する census 的な方法（決定 0097）を `CreateFruityInstance` の呼び出しに使えるか**が論点。`create-fruity-instance-measurement.md` §9 J6 は「census は unit を放棄して続けるので、単一の呼び出しには使えない」と読んだ）。
- **見積もり**: 決定 0118 の作業者の続き（`lift/out-fruity5/`）として行うのが自然。1 壁あたりのビルド＋測定は 約 1 分 20 秒、RSS 2.7 GB（決定 0116 の実測）。壁は 9 つ連続し、最初の 3 つ（種付け、`SHGetSpecialFolderLocation`、`GetCPInfo`）で 4,693 → 35,093 入口、後の 3 つで 38,913 入口。**壁の数は見えない**。
- **結果で計画がどう変わるか**: 初期化の本体が窓以外の壁で止まるか、窓の後は静かに Run まで届くかで、G1 の距離が決まる。

### X4: メッセージループの 1 周と、JSPI の中断が実エンジンで通るか（ブラウザ）

- **何を潰すか**: U4（G3）。
- **やり方**: **a4 の wasm（`flengine6.wasm`、SJLJ=wasm、JSPI。コミット c76a7c3 の既存のビルドスクリプト）に、`CreateFruityInstance` を呼ぶ opt-in の入口（`trace.c` に `fruity` 相当の入口を足す。native の `fruity.c` の wasm 版）を足し、Chrome（既存の `lift/harness/web/jspi/`）で、`PeekMessageW`／`WaitMessage`／`MsgWaitForMultipleObjects` の応答を**JS の Promise**にして（`Suspending`）、タブが応答するかを見る**。**これは X3 の窓とキューの仮想化が最低限できてからでないと、ループに入れない**。それまでの代替として、**決定 0117 の停止点（`CreateFruityInstance` が 38,241〜38,928 入口で止まる。native の名前付き停止と同じ停止が wasm でも出るか）を wasm／ブラウザで再現する**（ハーネスの wasm 版だけ。**G1 の「ブラウザでも同じ停止点」の実測になる**。これは小さい）。
- **見積もり**: wasm の再ビルドは約 12 分（`units` 218 s＋`o1` 383〜468 s、最大 RSS 8.8 GB。決定 0115／`browser-feasibility.md`／`jspi-real-wasm.md` の実測）。実行は 3 秒（高速版）〜30 分（未改造ドライバー）。**RSS 8.8 GB は `/tmp/opencode` の 2.9 GB と、この機械の RAM（13 GB）の制約に対して重い**（`jspi-real-wasm.md` は同じビルドを行ったので、可能。ただし他の作業と同時には回せない）。
- **結果で計画がどう変わるか**: native と wasm で停止点が一致するなら、**以後の壁の測定は native だけで進めてよい**（ブラウザの再確認は節目だけ）。食い違うなら、`FLE_LANDING_FORCE`（SJLJ）、ページ保護 fail-open、locals の `-O1` 19 関数のどれかが効いている。

### X5: 実機側の測定の依頼（作業ではなく入手）

- **何を潰すか**: U5（G5、G8 の受け入れ）。
- **依頼内容（読み）**: 実機の Windows 10 22H2 en-US（ランタイムの仮定の環境）での FL Studio 26.1.7.5653 の**起動直後のメイン画面のスクリーンショット**（DPI 100%、1920×1080）、同梱デモ 1 曲の書き出し WAV、**実機の API 呼び出しのトレース**（`CreateFruityInstance` の呼び出し順・引数。ホスト FL64.exe を差し替えず、エンジンの import をフックする `win-replay` とは別の道具）。既に届いた測定は Windows 11 26H2 ja-JP（PM の注記）で、**en-US・Windows 10 の機械が要る**。
- **見積もり**: 作業ではなく依頼者側の資源。本調査は時間を見積もれない。
- **結果で計画がどう変わるか**: スクリーンショットが無いと、G5 の「画面比較」は「それらしく見える」までしか言えない。**書き出し WAV が無いと G8 は「音が出る」までしか言えない**。

### 実験の順序（推奨。決めない）

1. **X1**（Blend2D の JIT と描画の本体）— 実行が最も短く、結果が **G4 以降の全体の方針**（JIT 無効ビルドへの置換が要るか）を決める。ブロッキングのモデル（M-1）を作る前に分かる。
2. **X3**（窓の作成を通して初期化の本体を走らせる）— U3 を潰す。決定 0118 の続き。壁 9・10 の判断（出典なしの値、窓表の拡張）が PM に要る。
3. **X2**（文字）— X1 と X3 の後でないと窓の領域に届かない。X1 の代替案（エンジンの文字描画関数を直接呼ぶ）なら先にできる。
4. **X4**（wasm でも同じ停止点か）— 節目ごとに。X3 の後が自然。
5. **X5**（実機の画面と音）— 並行して依頼者に依頼できる。

## 6. 進捗指標の置き換え案と、M1 の各項目がどの段階を塞ぐか

### 6.1 現行の指標が示していないもの（事実）

- 現行の指標は、M1 の 5 項目（A2a、A2b、A4、A5、A6）の達成度の平均（緩い約 85%、厳しい約 70%。`progress.md`。PM の簡易の指標で、測定でも残り工数の見積もりでもない）。**5 項目のどれも、ladder の G1 以降の段階（窓、メッセージループ、描画、入力、音声、プロジェクト）を含まない**（`progress.md` も、M1 の条件に「FL Studio 自体の動作を含む項目は無い」と書く）。
- 本調査の実測が示す関係: 指標の 85% の時点で、ladder は G0 のみ達成。`CreateFruityInstance` の直接 call の閉包 6,591 関数のうち実行されたのは約 350 関数（約 5%。§3.2）、そのうち初期化の本体 `f_13751e0`（6,449 関数）は 0 関数。**平均が上がっても、ゴールへの距離を表さない**（この指摘は依頼者の指摘と整合する）。

### 6.2 提案: 到達段階ベースの指標（決めるのは PM）

**主指標: 「到達した最高の段階」と「各段階の測定状況」の 2 つを必ず並記する**。数値の平均はしない（段階の幅は同じでない）。

```
ゴール到達: G0 まで（全 12 段階 G0〜G11 のうち、G0 のみ実測で達成）。G1 は部分、G2〜G11 は未測定
```

| 段階 | 状態の種類（固定の凡例） |
|---|---|
| 達成 | 受け入れ条件を、**実測**で満たし、独立再現がある |
| 実測（部分） | 一部の条件だけ実測で満たした（例: G1 は native の停止点のみ） |
| 読みのみ | 静的な読みだけ。実行していない |
| 未測定 | 実行した記録が無い |
| 未確認 | 受け入れ条件の判定手段（実機の画面・音など）が無い |

- **表の例（本調査の時点）**: G0 達成｜G1 実測（部分。native。ブラウザ未測定）｜G2 未測定（窓は 1 形のみ）｜G3 未測定（機構は実測済み）｜G4 読みのみ｜G5 未確認（実機の画面なし）｜G6 読みのみ｜G7 読みのみ｜G8 読みのみ｜G9 読みのみ｜G10 未測定｜G11 未測定。
- **段階ごとの副指標（任意。数えやすく、水増ししにくいものだけ）**:
  1. 実行された関数入口の数（`CreateFruityInstance` の呼び出しの入口数。38,241 → 38,928。**ただし停止点が進んだこと自体は進捗でない**（決定 0101 の趣旨）。参考値として併記するのみで、受け入れには使わない）。
  2. 当該段階の経路（静的な直接 call の閉包）の import の「行あり／名前付き停止／行なし」の件数（§3.10 の形。実測。壁が見えている範囲だけ）。
  3. 実機との照合済みの応答の割合（A2b の P/X の確認。§6.3）。**これは「実物との同等性」の軸で、到達の軸ではない**。
- **到達と同等性を別の軸に分ける**: 到達（その機能が動くか）は G0〜G11、同等性（実機と同じか）は別表。「A2b の P/X 78 種の実機確認」は同等性の軸（§6.3）。

### 6.3 M1 の各項目が、ゴールのどの段階を塞ぐか／塞がないか（本調査の読みと実測）

| M1 の項目 | 現状（引用） | 塞ぐ段階 | 根拠 |
|---|---|---|---|
| **A2a**（命令と制御フローの完全性。`unsupported_total` 390 は IAT 経由 jmp のみ。命令 stub 0） | 達成と読む（仮の解釈） | **G1 以降の「踏んだ命令が無い」前提を支える。ただしゴールの段階を直接塞ぐ未達部分は無い**。HEAD の `unsupported_total` は 379（決定 0117 で 8 thunk 減）。ただし FLEngine 以外の DLL（dsp_ippv2 の LDDQU 975 か所など）は個別に対応が要る（決定 0075 など） | `progress.md` A2a の行。新しい DLL（ildsp、oggio、wavpackdll、プラグイン）の lift で、**新しい命令の壁が出る可能性は未確認**（`m2-inventory.md` §5.6。新しい DLL に `fe-lift` は未実行） |
| **A2b**（import 応答の完全性。呼ばれた応答 171 種のうち仕様確認済み S+N は 41 種、P／X の 78 種は実機または公式資料が無いと数えられない） | 未達 | **塞がない**（**到達の前提ではなく、忠実度の基準**）。根拠: (1) A4 の完走（G0）は、P／X を含んだまま成立している（実測）。(2) `CreateFruityInstance` の壁 1〜9 の解消も、P／X を含む仮の応答で進んだ（決定 0116／0117）。(3) 本調査（実測）: A2b の監査表（`a2b-audit/audit-{GU,K1,K2}.tsv`。172 行。SUMMARY は 171 種と数える）で、クラスが P か X の行は 77、`P;X`／`N;P`／`X;P` の混合 3 行を足すと 80（SUMMARY の「P と X の 78 種」とは 1〜2 行の数え方の違いがある。**どちらが正しいかは未確認**）。この 80 行のうち **51 行が `CreateFruityInstance` の直接 call の閉包（6,591 関数）に入り、29 行は入らない**（`goal-ladder/a2b-px-vs-closure-by-root.tsv`。API 名の (DLL, 名前) の一致による。DLL 別: gdi32 10、user32 13、kernel32 47、advapi32 5、ほか各 1）。G1〜G5 の経路でも**同じ P／X が使われ続ける**ので、実機照合を済ませない限り G1 以降の「実物と同じ」は主張できないが、**照合なしでも G1〜G4 は到達できる（受け入れ条件は「動く」）**。(4) 実機の測定が 1 回届いた（引用: `worklog/1`。match 352、mismatch 161。Windows 11 26H2 ja-JP で、ランタイムの仮定（Windows 10 22H2 en-US）と違う） | **「A2b が塞ぐ段階」は無い**（到達の軸）。**ただし G5、G8、G10 の「実物と変わらず」（同等性の軸）には効く**。実機の測定が既に **161 件の不一致**を示している（GetDIBColorTable の last error、StretchDIBits の戻り値と last error、GetFileAttributesW の last error、SetErrorMode、GetVersionExW ほか。**実機が ja-JP の Windows 11 なので、そのまま「ランタイムの誤り」とは言えない**）ので、A2b の確認作業は **到達を妨げないが、同等性の主張を裏づけるには必要** |
| **A4**（DllMain → InitUnits の完走） | 「A4 完走（仮の応答 47 件を含む）」 | **G0 そのもの**。完走済み。G1 以降は A4 の定義の外 | `progress.md`、決定 0101 |
| **A5**（native オラクルでの bit-exact。純計算 9,901 関数のうち比較できて一致したのは 9,507（元の入力）／9,891（修復入力を含む）。FLEngine の全 59,380 関数では 16.0%） | 部分 | **G4、G8 の「描画・音が正しい」の信頼度を支える**が、**到達は塞がない**。純計算関数の bit-exact は、実際に画素と音を作る関数（DSP の浮動小数、描画の合成）の正しさの下限の証拠。**一方、A5 の対象は「純計算」（IAT 呼び出しや間接 call を持たない 9,901 関数）で、描画・音声・UI の関数の大半（API を呼ぶ・仮想呼び出しを持つ関数、約 47,915 関数）は対象外**（`a5-oracle-run2.md` §3） | 本調査: BitBlt の上流 3,482 関数、CreateDIBSection の上流 4,046 関数、Agg2D の上流 2,243 関数は、**定義上 A5 の対象外**（IAT 呼び出しを持つ関数と、その呼び元）。つまり **A5 の「達成」は G4〜G8 の描画・音の正しさを保証しない**（読み） |
| **A6**（実測値を docs に記録、コミット、push） | push 済み（`progress.md`）。100% は算術 | **塞がない**（運用の項目。ゴールの段階の観測に関わらない） | 指標の A6 = 100% の内訳の根拠は文書に無い（`progress.md` が明記） |

**A3（wasm 化。M1 の完了条件ではない）**: G0 の wasm 版（node と headless Chrome）は実測済み。**G1 の wasm 版は未測定**（§5 X4）。

### 6.4 推奨（意見。決めない）

1. **進捗報告の先頭を ladder の 1 行にする**: 「到達: G0（全 12 段階の最初）。G1 は native で部分。G2 以降は未測定または読みのみ」。M1 の平均は併記してもよいが、**先頭に置かない**。
2. **M1 の項目は「ゴールの前提」と「忠実度の基準」に分けて報告する**。A4 は G0 の前提（達成済み）、A2a は命令の前提。**A2b と A5 は忠実度の基準**（「実物と変わらず」の同等性の軸）で、到達を塞がない。
3. **到達の作業の優先**: 本調査の実験 X1（Blend2D の JIT と描画の本体）、X3（窓の作成を通して初期化の本体を走らせる）を先に。結果が出るまで G4 以降の見積もりは**数字にしない**。
4. **同等性の軸の作業の優先**: 実機のスクリーンショット・書き出し WAV（X5。依頼者側の資源）と、A2b の P／X のうち**描画・窓に関わる 23 行（gdi32 10、user32 13。`goal-ladder/a2b-px-vs-closure-by-root.tsv`）**を実機で確認する（既に届いた `win-replay` の測定は別の機械（Windows 11 ja-JP）なので、Windows 10 22H2 en-US の機械を依頼する選択もある）。

## 7. 限界・未確認・補助ファイル・規則違反の自己申告

### 7.1 本調査の限界（先に）

1. **静的分析は下限**: 到達は直接 call と tail jmp（T1）と `lea`（T2）だけ。Delphi の仮想メソッド、`WndProc` の配送、コールバック、`GetProcAddress` 経由は辿れない。**「Run の閉包に DirectSound が無い」「ExtTextOutW の上流が少ない」は、「実行されない」を意味しない**。実際の描画・音声の関数は、ほとんどが vtable 経由（`DispatchMessageW` → `WndProc` → VCL）で呼ばれる（読み）。
2. **API 名による分類は正規表現（`imports_vs_table.py`）で、本調査の読み**。`Rgn`／`Clip`、`Menu`、`Palette` などの部分一致による。境界の API（たとえば `MoveToEx` を gdi-vector に、`GetStringTypeW` を loader 側に）は分類の判断が入っている。**件数は目安で、分類の細部は検証していない**。
3. **「行あり」＝「通る」ではない**（名前付き停止の行がある。決定 0111 の区分 C の 59＋10 行）。件数の突き合わせは、行の有無（応答表の (DLL, 名前)）だけ。
4. **描画の実体（§3.3）は、実行していないので結論が出ていない**。E1〜E7 は静的な呼び出し関係と文字列からの読み。**Blend2D の JIT の有無、メイン UI の文字の経路は未確認**（実験 X1、X2）。
5. **未コミットの作業（`lift/out-fruity5/wall-log.md`）を読み取りで引用した**。数値は再測定していない。決定 0118 の作業中で、記述が変わりうる。
6. **規模の見積もりに日数・人月を書かなかった**（根拠が無い）。規模の桁は既存の同種の実装の行数（`fle_win.c` 300 行、`fle_gdi.c` 2,494 行ほか）と、`window-design.md` §8 の「小／中／大」からの読みで、弱い。
7. **ブラウザでの測定は、a4 の初期化経路と JS の試作だけ**（`browser-feasibility.md`、`browser-event-loop.md`、`jspi-real-wasm.md`）。Firefox／Safari、実デバイスの音声、実 GPU、実ネットワークは未測定。`CreateFruityInstance` を wasm／ブラウザで一度も呼んでいない。
8. **実機の画面と音は無い**。実機の API の測定は 1 回届いたが、Windows 11 26H2 ja-JP の機械（仮定の Windows 10 22H2 en-US ではない）。
9. **段階の定義（G0〜G11）は本調査の案**。PM が直してよい（依頼どおり）。受け入れ条件のうち G5、G8 の「画面・音の比較」は、実機の素材が無いと判定できない。
10. **逆アセンブルは capstone の線形走査**（`.pdata` の範囲の内側だけ）。Delphi のコードにはデータが混じる（`(bad)`）。`.pdata` に載らない関数（thunk 以外）は索引に入らない（K1 と同じ限界。FLEngine で 58,706 に対し本調査の 57,816）。
11. **自分で訂正した誤り（記録）**: (a) 最初の `com-ole` の正規表現が広すぎ（`Co.*`）、COM でない API を含めた→絞って再実行。(b) `fle_index.py` の IAT 判定で `t - base` を引く誤り→修正。(c) 初回の P/X の集計が `.dll` の表記ゆれで 0 件だった→正規化して 51/80 に。(d) 「DirectSound は Flags = 0 では届かない」を、Run の閉包だけで判断して書いたが、`f_13751e0` の閉包に入っていた→§3.4 を訂正。(e) `f_f51fd0` を「メインプログラム本体」と書いたが、文字列から FLP の解析関数と分かった→訂正。(f) `TextHeight` の 102 件を呼び出し数のように書いた→フォームのリソースのプロパティ名と訂正。

### 7.2 確認できなかったこと

- Blend2D の `Agg2D` ラッパーが JIT を無効にして描くか。ゲストが `AggCreateInstance` を呼んだ後の描画の実行。
- メイン画面の文字が GDI／FreeType／Blend2D のどれを通るか。
- `f_13751e0`（初期化の本体）の下で、実際に踏む行なし API の数と種類。
- 音声のミキサースレッドが要るか、DirectSound のバッファ位置の進み方。
- FLP を開いたときに実際に走るコードの範囲、サンプルの復号に必要な未 lift の同梱 DLL（ildsp、oggio、wavpackdll）の lift の難しさ。
- プラグイン 160 個の import の集計（`m2-inventory` は PE の表。本調査は集計していない）。
- 実機（Windows 10 22H2 en-US）での FL Studio の画面・音・API の呼び出し順。
- ホスト FL64.exe（`Flags = 0`）が、起動時に最後のプロジェクトを開くか、設定のレジストリで DirectSound を初期化するか。
- Issue #1 の測定で、画面・DPI の項目（`GetDeviceCaps`、`GetSystemMetrics`）の実機の値とランタイムの値の差の詳細（本調査は集計値の引用だけ）。

### 7.3 補助ファイル（`docs/investigations/goal-ladder/`。約 90 KB）

| ファイル | 内容 |
|---|---|
| `fle_index.py` | FLEngine の `.pdata` 関数の索引（直接 call、IAT 呼び出し、lea、thunk）を capstone で作る（標準の `pickle` を使う。約 7 秒） |
| `fle_paths.py` | 索引への問い合わせ（`callers`／`reach`／`path`／`fn`） |
| `imports_vs_table.py` | FLEngine の import と応答表の突き合わせと分類（出力は `fle-imports-cat.tsv`） |
| `a2b_px_vs_closure.py` | A2b の監査表の P／X と、各根の直接 call の閉包の突き合わせ（出力は `a2b-px-vs-closure-by-root.tsv`） |
| `fle-imports-cat.tsv` | FLEngine の import 795 名ごとの (DLL, 名前, IAT／遅延, 応答表の行の有無, 分類) |
| `reach-cfi-t1-none.txt` | `CreateFruityInstance` の直接 call の閉包にある、応答表に行が無い import 127 名 |
| `a2b-px-vs-closure-by-root.tsv` | A2b の P／X 80 行が、4 つの根（`CreateFruityInstance`、`f_11b32a0`、`f_13751e0`、Run）の閉包に入るか |

機械語の抜粋はリポジトリに入れていない（本文中の逆アセンブルの記述は、アドレスと命令の意味の要約のみ。引用した命令列は 0 行）。製品のバイナリ・文字列の長い引用は入れていない（引用した文字列は、エンジン自身のエラーメッセージの短い断片のみ）。一時物は `/tmp/opencode/ladder/work/`（RAM。索引の pickle など。再生成は `fle_index.py` で約 7 秒）。

### 7.4 規則違反の自己申告

- **AGENTS.md の「作業は task agent を通じて」**: 私は task ツールを持たないサブエージェントなので、調査・分析・文書作成をすべて自分で行った。
- **読み取り**: `lift/out-fruity5/wall-log.md` を読んだ（依頼どおり読み取りのみ。書いていない）。`git show` で `worklog/1/` の内容を読んだ（読み取りのみ）。
- **実行したもの**: python（`PYTHONDONTWRITEBYTECODE=1` を付けたものと、標準入力から流した短いスクリプト。後者も `PYTHONDONTWRITEBYTECODE=1` を付けて実行した）、`pefile`／`capstone` による静的分析、`awk`／`grep`／`sed`。**cargo／emcc／clang のビルドと、FL Studio のバイナリの実行はしていない**。
- **`rm -f`**: 自分の成果物の中の旧ファイル 1 つ（`goal-ladder/a2b-px-vs-cfi-closure.tsv`。名前を変えて作り直したため）だけ。リポジトリの追跡ファイル、`lift/out-*/`、`lift/out-git/` は削除していない。
- **ファイルの編集**: 編集したのは `docs/investigations/goal-ladder.md` と `docs/investigations/goal-ladder/` の下だけ。追跡ファイルは変更していない。git の変更系操作はしていない。commit していない。
- **サーバー再起動**: 作業の途中で 1 回再起動があった（`/tmp/opencode/ladder/` は空になっていた）。再起動の前に読んだ資料（文書の読み）を再度読み直さずに続けた。成果物は再起動の前には存在しなかったので、重複や破損はない。再起動の前後で HEAD（c76a7c3）と FLEngine の SHA-256（`b2c64da4…9496d`）が同じことを確認した。
- **`/tmp` の使用**: `/tmp/opencode/ladder/work/` だけ。最終の使用量は 5 MB（`du -sm`。索引の pickle 2 つと中間の TSV）。索引の作成中の最大 RSS は約 0.7 GB（`/usr/bin/time -v` の実測）。`/` に書いたのは成果物（約 90 KB）だけ。
- **未追跡のファイルについて**: `git status` に `docs/investigations/win-replay-triage/` が出る。**私が作ったものではない**（他の作業者のもの。触っていない）。
- 資格情報・ブラウザのプロファイルには触れていない。Chrome は起動していない。
- 製品ファイル（FLEngine_x64.dll、インストーラ）をリポジトリに入れていない。インストーラは読んでいない（`lift/out-qfc/quickfontcache_x64.dll`、`lift/out-wallQ/freetype/freetype_x64.dll`、`lift/out-inventory/extracted/ILTools_x64.dll` の既存の抽出物を読み取りで使った）。

### 追加要求: デタッチ窓（低優先度、2026-10-30）

- 要求の趣旨: FL の窓（チャンネルラック、プレイリスト、プラグインなど）は「デタッチ（D）」で FL 外の独立した Windows 窓になり、メイン窓との位置関係を保ち、他窓とのスナップは無効になる（全窓対応かは未検証）。Web では、これを**独立したポップアップウインドウ**として実現すること。
- 調査: `docs/investigations/detach-window.md`（静的分析、窓表への設計制約、`window.open`/Document PiP の headless 実測。実機 Windows の観測は無い）。要点: デタッチは VCL の `Parent` 付け替えで HWND を再作成する（読み）。窓表に親とオーナーの別、親相対の矩形、Z 順、描画面を top-level 単位で持たせる制約を G2 の設計時に入れる。
- **優先度と着手時期は PM が決める**（本節は決めない）。調査は G2〜G6 のどこで何が要るかと、延期時に守る制約（同文書 §3.2、§6）を出しただけ。

