# 依頼者の観察(2026-11-07 06:50、06:51)の記録と分類

出典: 依頼者(Windows 11 Pro、Ryzen 9 9950X、RTX 5080、DDR5 128 GB、4K 240 Hz HDR)が自分の機械で製品をブラウザで動かして得た不定形のフィードバック。以下は依頼者の記述の要点を落とさずに書き写した物で、分類・担当・状態は PM(ここ)が付ける。依頼者の機械の値は我々の測定ではない(PM は 0 件のまま)。直近 30 分以内に直った物は無視してよい、との指示つき。

基準(ネイティブ、依頼者の評価用プロジェクト): 再生中の fps 160〜240、発音数 16〜20、再生中に Playlist を激しくドラッグして 60〜100 fps、メモリ約 3750 MB。製品はブラウザで同じ環境上で動作させた。

区分: A = 我々が足した余計な処理(除去する) / B = 外部 API(Win32 など)の契約の不足(契約どおりに拡充する) / C = 資材(Data・Plugins)が無い / D = FL ネイティブ自身の挙動(我々は手を加えない) / E = 未分類(調査)。方針は `docs/policies/lift-fidelity.md`(『動かない物を個別に直さず、全体の仕組みを直す』)。

| 番号 | 依頼者の観察(要点) | 区分(暫定) | 担当 | 状態 |
|---|---|---|---|---|
| O1 | 描画は大きく改善(『重くて作業にならない』ではなくなった)。ただし Playlist で Pattern をドラッグすると、ネイティブでは慣性/重力のようなアニメーションと配置先の骨格が出るのに、製品は残像つきで非常に低いフレームレート。**激しくドラッグするとエンジンが死ぬ** | E(アニメ=タイマーと描画の契約、死ぬ=B の可能性) | gpu1(再現とログ)、alpha1(契約) | 未再現 |
| O2 | 起動アニメーションのフェードアウトが非常に低いフレームレート | E(タイマー・描画頻度) | gpu1 | 未調査 |
| O3 | Settings → About: ネイティブは FL Studio ロゴが浮遊するアニメーション。製品はロゴ画像が全く出ず、スタッフロールも描画されない | E(資材 C か描画の契約 B) | gpu1、opt1(資材の有無) | 未調査 |
| O4 | アニメーション関連が全体的にまだ足りないのでは(推測) | E | gpu1、alpha1 | 調査 |
| O5 | DOM ヘッダの『Monitor >』ボタンで右のドロワーを畳むと**エンジンが止まる** | A(DOM 側の処理が絡むなら除去)または E | gpu1(再現)、ci1 | 未再現 |
| O6 | 数値目標(fps、発音数、メモリ、Playlist ドラッグ中の fps)に近づけること | 性能(P2/P3) | 全体 | 継続 |
| O7 | 画面がぼやける(HiDPI 未対応?) | B/E(ゲストの DPI・画面サイズ・devicePixelRatio の扱い) | gpu1(設計)、alpha1(契約) | 未調査 |
| O8 | Settings → Debug タブのログを Worker が受けているか。製品のログ: `Starting at 12:00:00 AM; 64Bit addressing mode; 3DNow! support: no; SSE yes; SSE2 yes; SSE3 no; SSSE3 no; AVX no; Touchscreen not available; Main GUI scaling factor: 1 (system set to 96PPI); Temporary folder ...C:\Windows\Image-Line\4096-{...}\; FL Studio language: English (US); Windows display language: en-US; Loading tags started ...; Browser caching took 1 milliseconds.; Animation timer set to 8ms, vsync enabled; Mixer track map: 0,1,2,...`。ネイティブ: `Starting at 5:42:38; ... SSE3 yes; SSSE3 yes; AVX yes; Main GUI scaling factor: 1.75 (system set to 168PPI); ...Temp\Image-Line\90120-{...}; FL Studio language: Japanese; Windows display language: ja-JP; Loading tags started...; Loading tags finished.; Browser caching took 148 milliseconds.; Animation timer set to 4ms, vsync enabled; Mixer track map: 0,1,2,...,10,...` | B/E(読み。時刻 12:00:00 AM=ゲスト時計、Loading tags finished が出ない、CPU 特徴は SSE2 のみ、DPI 96/倍率 1、Animation timer 8 ms=画面更新 Hz を約 120 Hz と答えている疑い) | alpha1(契約)、gpu1 | 差の原因を調査。ログが Worker に届くか未確認 |
| O9 | Settings → General は開くが、**スクロールすると描画が崩れる**(縞、空白帯、行の重なり、一部の文字が横に潰れる。ホバーした要素だけ正しく戻る。以前の window ghosting に似る) | B(ScrollWindowEx/ScrollDC/無効領域の契約、フォントの幅)の疑い | alpha1(契約)、gpu1(再現) | 未調査 |
| O10 | テーマ・言語・テンプレートなどの**コアなデータが無い**。意図したものか | C(意図ではない。seed は約 222 ファイルのみ) | opt1(記録)、mt3(配信設計) | 判断済み(下) |
| O11 | FL 内蔵のサンプルが使えない(例: `Data\Patches\Packs\Drums (ModeAudio)\Kicks` に WV Audio *.wv が 349 個) | C+B(WavPack 復号など) | opt1、mt3 | 未着手 |
| O12 | `FL Studio 2026\Data` は 792 MB、`Plugins` は 867 MB。**これらの資材はすべて製品で使えるべき**。読み込み時期(FL 窓を先に描いて背景で取得するか、全 1.62 GB を OPFS に置いてからエンジンを始めるか)は PM が決める | C | PM 決定(下) | 判断済み |
| O13 | 機能の大半が使えない: 内蔵楽器が無い、サンプルが無い、OPFS に置いた音声ファイルを FL が認識しない、Sampler に音声ファイルを指定すると落ちる、コード進行ツールをクリックすると落ちる。**メトロノーム以外に『音を出す』最初の一歩ができていない。1 拍目が鳴らない問題が未解決。音声は常時ストリームか。『無音で切る』処理を書いたなら除去する** | A/B/C | mt3(音声経路の点検)、gpu1(再現)、alpha1 | 点検中 |
| O14 | 変な落ち方がまだある(特定の画面の特定のつまみを回すと落ちる、など)。『動かない物を個別に直さず仕組みを直す』方針は全体に周知・徹底されているか | 方針 | PM | 監査(下) |
| O15 | (追記 06:51)ネイティブでも、再生中に FL の窓を動かすと window ghosting が起きる。ただし依頼者がギリギリ認識できる短時間で解消する。メーター・音声に資源を割く FL 自身の最適化の帰結で、ghosting が出てもメーターのフレームレートは落ちず、音も途切れない | D(FL 自身の挙動。我々は手を加えない) | alpha1(解消の速さの比較) | 記録 |

## PM の判断(2026-11-07 06:55)
1. 資材: `Data` 792 MB と `Plugins` 867 MB の**全体**を製品で使えるようにする。方式は遅延読み込みの仮想ファイルシステム: 起動時にディレクトリ索引(木・サイズ・属性)だけを読み、FindFirstFile 系と GetFileAttributes が全域で効く。中身は要求時に取得(R2 の pack から HTTP range)し、OPFS に保存、背景で先読みする。エンジンの起動は索引と seed が揃った時点(P1 を保つため。1.62 GB を OPFS に置き終えてから始める方式は P1 を壊すので採らない)。事前の調査(opt1): ディレクトリの記録(件数・サイズ・拡張子)、起動時に FL が開くファイル、ブラウザ/テンプレート/テーマ/言語が開くファイル。
2. 方針の周知: `docs/policies/lift-fidelity.md` は存在するが、『1 つの API の停止を 1 件ずつ直す』形が続いていたのは事実。全員に依頼者の文面つきで再通知し、『画面ごとの修正』でなく『契約ごとの網羅(import の census)と停止の系統的な扱い』で進める。監査は PM が行い、結果を本書に追記する。
3. 音声: 『無音で切る』種の処理が app/ や runtime にあるか、1 拍目の欠落の原因を mt3 が点検する(あれば除去)。
4. O15 は FL 自身の挙動なので、我々は再描画や隠蔽の処理を足さない。比べる物は『解消までの速さ』と『メーターの fps・音が落ちないこと』。

## PM の判断の追記(07:21、worker の第 1 回の報告〔0318 未着、0319 alpha1、0320 opt1、0313 §15〜§18 mt3、0321 ci1〕を読んだ後。docs のみ)

読んだ範囲は読み・静的解析・原本の読み取り専用の棚卸しで、依頼者の機械の値は 0 件。実エンジンを動かした観察は gpu1 の Balance の窓(stage 3)と alpha1 の node G3 のみ。

1. **資材(O10〜O12)**: 原本の install tree は 27,822 ファイル・2,053.3 MiB(Data 785.0 + Plugins 867.7 + Shared 205.8 + System 128.9 + Artwork 19.7 + Help 2.8 + 直下 43.4。opt1 の 0320。表の行数と合計を PM が確認)。**テーマは `Artwork\Themes`、言語は `System\Languages` にあり、Data・Plugins の外**。したがって遅延ロードの対象は **install tree 全域**(索引は全域)とする。方式は mt3 の 0313 §18(P-C): chunk を R2 の不変オブジェクトとして置き(小ファイルの流れ 1 MiB・大ファイルの流れ 4 MiB、縮まない組は identity、縮む組は brotli)、起動時は索引(約 0.3〜0.4 MB の見込み)だけを読み、中身は要求時に取得して OPFS に保存、背景で先読み(再生中は止める)。待ちは Atomics.wait + I/O Worker(JSPI 非依存)。既存ファイルへの書き込みは copy-on-write の overlay。lift していない DLL の中身は配らない(暫定。索引にサイズと属性は持たせる。file trace で FL が未 lift の DLL の中身を読むかを確認し、読むなら例外の一覧を作る)。実装は段階(V0 配布物の生成器 = opt1、V1 runtime の lazy node と host TS = mt3 の差分、V2 OPFS・実 R2・Chrome は V1 の後)。差分は push しない。runtime の C が変わるので push は後の G9 の push に載せる。
2. **Debug ログ(O8、alpha1 の 0319)**: ログは FL の内部で組み立てられ、FL 自前の一覧で描かれる。`OutputDebugStringW` は呼ばれても『デバッガ無し』で何もしない契約なので、**Worker・engineLog には届かない**(依頼者の問いへの答え)。差の分類: 時刻 12:00:00 AM = (b) 契約の不足(ホストの実時刻と時差を run の値として渡す。差分の作成を許可)、Temp パス = (b)(初期環境ブロックに TEMP/TMP/USERPROFILE 等を仮想プロファイルの定数から。差分の作成を許可)、GUI scaling 1・96PPI とぼやけ(O7)= (b)(実 DPR をデバイス画素の画面サイズと DPI 96×DPR で run の値として渡し、GetDpiForMonitor・GetDeviceCaps・GetSystemMetrics・EnumDisplaySettingsW・モニタ矩形を同じ出所にする。gpu1 の設計メモと一本化してから実装を許可)、SSE3/SSSE3/AVX no = 決定 0307(Export の決定性・IPP/Elastique)の意図した差として維持(CPUID を lift の命令の表から機械的に出す案と、SSSE3→AVX の順の lift の拡張は B-FLE/G9 の後の宿題)、Animation timer 8 ms = 決定 0268(Hz 上限 120)の意図した差として維持(P2 は 120 fps。120 超は描画を増やさず Worker の負荷だけ増える。提示経路が 120 に届いたら再検討)、言語 en-US・Browser caching 1 ms = 資材(System\Languages と Data)を遅延 VFS に載せた後、Loading tags finished = 診断の後。
3. **Settings→General のスクロールの崩れ(O9)**: 最有力の読みは (b): ScrollWindowEx が『最上位窓の面の中で矩形を動かす』だけで可視領域を使わず、既存の更新矩形に触らない(元の位置だけが無効のまま残る)。直す全体の仕組みは『窓の中身を動かす操作〔SetWindowPos・ScrollWindowEx・ScrollWindow・ScrollDC〕を一つの有効領域のモデルに揃える』。差分の作成を許可(適用しない)。Wine 10 を基準にした小さい Win32 テストで契約の 3 点を確認する。
4. **文字が横に潰れる(O9)**: Win32 の文字 API の誤りではない(GDI の文字系は名前つきの停止)。候補は FL 自前の文字描画への入力(フォントの代用: tahoma→DejaVu Sans が幅広)。native の FL が実際に開くフォントを file trace で確認してから。今は何も足さない。
5. **音声の 1 拍目(O13、mt3 の 0313 §16)**: 『無音で切る』処理は app/ と runtime に見つからない(読み)。最有力の機構は `fle_dsound.c` の再生開始の順序(状態を PLAYING にしてから最初の hand-over)。区分は (a) 我々の順序の除去。観測(gpu1、非計時の Chrome、SKIPPED_FRAMES が再生開始から 100 ms 以内に 0 を超えるか)で裏付けてから、診断入りテスト build に 2 行の入れ替えを同梱して確かめる。push は後の G9 の push に載せる。
6. **native の 160〜240 fps(O6)**: 決定 0268 の Hz 上限 120 のため現設計では届かない。当面維持(上の 2)。
7. **方針の監査(O14)**: ci1 が静的監査(FL のクラス名・窓の種類・位置による分岐、窓管理の代行、画素の補正、『成功を返すだけ』のスタブ、場当たりな軽量化)を `ci1-results/0322/LIFT-FIDELITY-AUDIT.md` に出す(期限 08:30)。結果を本書に追記する。

## 依頼者の追加の所見(2026-11-07 08:02の少し前、『不定期フィードバック』。原文の要点を落とさず記す。画面の写真 1 枚つき: Playlist の Track 3〜8、FL の CPU メーターの表示が 104、メモリ 0 MB、ポリフォニー 13)

| 番号 | 依頼者の観察(要点) | 区分(暫定) | 担当 | 状態 |
|---|---|---|---|---|
| O16 | 『おそらく日本語フォントにも対応していません。豆腐になっています。』 | (b)(c): 仮想 `C:\Windows\Fonts` に日本語のグリフを持つフォントが無い(alpha1 の読み: arial=Liberation、tahoma=DejaVu Sans、lucon=DejaVu Sans Mono の 3 つ) | alpha1(フォールバックの契約の静的解析)、opt1(Wine の参照と製品用フォントの棚卸し) | 調査 |
| O17 | 『現在のWorkersにおいて、検証用プロジェクトを読み込むことで再生まで行えましたが、音が途切れる、再生が停滞するなどにより、ほとんどリアルタイムでの再生が滞っています。何度か試しましたが、Track 1-5,7,13,15-17,19,20のみのSoloにすることでギリギリでリアルタイムの再生が成立しています。現在一切のインストゥルメントが提供されていないため、サンプルのみの再生となります。今後内蔵インストゥルメントやSerum 2などの外部インストゥルメント、EQなどを追加していく上で、このパフォーマンスは致命的です。』 | P3(協調 1 Worker の飽和)。我々の測定(曲の重い区間でミキサー 1 周の仮想時間 3 ms→165〜270 ms、書き込み率 1/3。0273・0274)と整合 | mt3(RT-1: 実スレッド版の本番化)、ci1(counted の S3a の掃引) | **最優先に引き上げ** |
| O18 | 『ウインドウやノートの端をホバーしたり、トラック内やボタン上をホバーしても、マウスカーソルが変化しません。』 | (b): HCURSOR の絵と hotspot をブラウザの CSS カーソルへ渡していない(`app/src/ui/cursor.ts`、S-module-cursor-default / S-created-cursor-default、決定 0141・0322 の 4-7)。IDC の標準カーソルだけが変換される | ci1(runtime + page の差分) | 差分の作成 |

### PM の判断(O16〜O18)
1. **P3 を最優先に引き上げる。** 本番(協調 1 Worker)は、依頼者の 16 コア級の機械でもサンプルだけの再生で飽和している(定性的。依頼者の機械の値は我々の測定ではない)。FL は本来 ミキサーを複数スレッドに分けるので、**実スレッド版(B1/B2、S3a の Worker 数)を本番に入れる方向に切り替える**(RT-1)。以前の関門『T が協調 +2 ms 以内になるまで main に入れない』(4e 23:44)を外し、新しい関門にする: ①数の基準(5 分再生で PCM の零の連続・distinct 書き込み・underrun・Unlock 間隔)を満たす、②入力の T の中央値が協調の +10 ms 以内(B1 の実測は File クリック +9.9 ms、Esc +8.7 ms)、③ present の rate が協調より落ちない。T は並行して S2(0312c)で縮める。理由: 音が途切れて再生が成立しないことは製品として致命で、+10 ms の入力遅延(60 Hz の 1 フレーム未満)は耐えられる。
2. **計測**: 既存の B2 bundle `fl-engine-ci-8b2eb5d7` の `engine.env` の `FLE_WORKERS` を 2・4・8・16 に変える掃引(再 build 不要、0312c §5c)と対照の E13、条件は参照プロジェクトの 5 分再生(全トラック)と『Solo 12 本』(依頼者の条件)。ci1 が counted の Chrome で行う(Wine の窓の後、gpu1 の cand1 の V3 の合間)。
3. **O18**: 絵と hotspot の受け渡し(CreateIconIndirect の ICONINFO、LoadImage/LoadCursor の resource、ILCursors.ilfont 由来)を runtime が appview に出し、page が handle ごとに CSS の `cursor: url(...) x y` を当てる。サイズ上限と HiDPI(image-set 2x)に注意。FL のクラス名・カーソルの種類による分岐は足さない。
4. **O16**: 日本語のグリフを持つフォント(OFL の Noto Sans CJK JP など。Windows のフォントは再配布しない)を仮想 `C:\Windows\Fonts` に置く案と、FL が選ぶフォールバックの契約(NEWTEXTMETRICEX の FONTSIGNATURE、GetFontUnicodeRanges、FontLink)を、実機の参照(Wine。この機械には Noto CJK があり fontconfig 経由で Wine が見る)と突き合わせて決める。フォントの出荷は遅延 VFS の層(日本語が要った時にだけ取得)。



## PM の追記(2026-11-07 09:14): O5 の有力な手がかり(ci1 の e2e の切り分け。実エンジン E13・実 Chrome 156・非計時)

- **事実**(ci1 の e2e、E13 の bundle `fl-engine-ci-0363b050`。E13 の木でも E14 の木でも同じ): ページが画面を縮める(CDP の Emulation による viewport の縮小 = display change)と、実エンジンが止まる。縮小直後の engine log の最後の行は `[err] fle: fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16): decision 0064`(`lift/runtime/fle_callback.h:21`、`fle_callback.c:41`)。止まった時の状態は Welcome wizard と『Help us improve…』の dialog が開いたまま。縮小の前は正常で、右の欄は `Engine: stopped (stopped, exit code 134)`。写し: `lift/out-git/ci1-results/0277/e14/e2e-resize/`。
- **仮説(未確認)**: O5『DOM ヘッダーの Monitor > ボタンで右のドロワーを畳むとエンジンが止まる』は、ドロワーの開閉で room の大きさが変わる = display change なので、同じ停止かもしれない。gpu1 が O5 を再現できなかったのは、dialog を閉じた状態で試したためかもしれない。**モーダル(wizard・dialog)が無い状態でも起きるかは未確認**。
- **分類(暫定)**: `FLE_CB_MAX_DEPTH 16` は我々の runtime の上限(ホストの wasm の stack を守る安全弁)で、Windows の契約ではない。(a) 我々の制約か、(b) 入れ子を作る窓の送信の契約の実装の問題かは未特定。上限を上げるだけの対症は採らない(lift の忠実性)。担当: alpha1(node G3 で再現し、入れ子の各段の窓・メッセージ・呼び元を出す。期限 10:15)、gpu1(cand1 の run の最後に、ドロワーの開閉を dialog 開いたまま/閉じた後の 2 条件で)。
- **別の発見(同じ e2e)**: `engine-desk` の e2e が、決定 0243 で削除済みの `standin-banner` 要素を読んで例外になり、シナリオが打ち切られる(E13 でも E14 でも)。後続の実エンジンの検査が走っていない = e2e の実エンジンの網羅が空。ci1 がテスト側の修正案を出し、後続の検査が実際に何を言うかを E13 の木で見る。


## PM の追記(2026-11-07 09:40): cand1・Wine の J4/P0 による O3・O5・O9・O16 の切り分け(実エンジン・実 Chrome 156 と Wine 10。非計時の機能の確認。依頼者の機械は 0 件)

| 番号 | 切り分け | 根拠(実ファイル) | 区分 | 次 |
|---|---|---|---|---|
| O5(Monitor > でドロワーを畳むと止まる) | **再現した**: Welcome wizard(モーダル)が開いたままの display change(ドロワーの開閉で room が変わる)で、`fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16)` により SIGABRT(exit 134)。2/2 で再現、stop context の値(guest return 0x6136d6、rsp 0x126f2ad8)も 2 回とも同じ。**wizard を閉じた後の開閉 4 回は止まらない**(5b)。alpha1 の node G3 でも再現(7.1 s)。ページの停止の文言(first non-background pixel … UpdateLayeredWindowIndirect)は古い runtime message で、原因ではない | gpu8-results/0319/logs/cand1-drawer5a・5a2・5b、ci1-results/0277/e14/e2e-resize | 未特定: (a) 我々の安全弁(上限 16)か、(b) 入れ子を作るモーダルの窓の送信の契約の不足か | alpha1 の診断 `diag-g9-cb`(入れ子の各段の窓・メッセージ・呼び元)の bundle で chain を読む(期限 10:15) |
| O3(About のロゴが 3×3、staff roll なし) | ロゴの `TLightweightLayeredControl`(lid 141)は 16×16 で作られ、窓の一覧では 3×3(premultiplied alpha、alpha 255)。**vfs-miss**: `Artwork\\Skins\\Default\\BigFruit.png`(status 2)、`System\\Texts\\About\\AboutTxt.txt`(status 3)。= ロゴの画像と staff roll の本文が seed に無い | cand1-about | C(資材の欠落。API の契約ではない) | mt3 が seed だけを差し替えた実験用 bundle(BigFruit.png ほか)を作り、gpu1 が About で確認。全 install tree の遅延配信で恒久化 |
| O16(日本語が豆腐) | **Wine で確認**: `C:\\Windows\\Fonts\\msgothic.ttc` の有無だけで豆腐 ↔ 日本語。FL の文字は GDI でなく QuickFontCache(AGG + FreeType)で、起動の最初に固定名の fallback 一覧を確認し、msgothic.ttc があれば全体(2,123,288 B)を同期で読む | opt1 の 0320 §13(J4・P0) | C(資材の欠落) | **判断: eager な seed の行にする**(遅延にすると起動が待つ。os-fonts の前例)。再現できる版(recalcTimestamp=False)を正とする |
| O9(Settings→General のスクロールで描画が崩れる) | cand1 の ScrollWindowEx のモデル入りでも E13 の絵とビット単位で同一。崩れは『同じ文字列が、スクロールの 1 段ごとのずれの間隔で複数回残る』形。PM の仮説(子窓の移動で親の空いた領域が無効化されない)は alpha1 の harness(面 M)で**否定**された(8 通りとも契約どおり、SWP_NOREDRAW の時だけ古い絵が残る)。FL の並びを再現した面 N では、親が WS_CLIPCHILDREN なしだと子の新位置に古い絵のコピーが残る(契約どおりの動き) | alpha1 の 0319 §15 | 未特定(FL 側の事実: スクロール容器の style に WS_CLIPCHILDREN が付くか、窓を持たない部品の描画順) | alpha1 の `diag-g9-fl-paint`(窓ごとの vis・show・paint-msg・BeginPaint・draw-discard)を次の bundle に載せて FL の run のログで決める |
| O1(clip のドラッグ) | ImageList 系の呼びは出ていない(`fle: unsupported` 0)。ドラッグ中に元の位置から clip は消えるが、浮いた clip はどこにも描かれず、放した後も見えない。layered の影の窓は作られない(窓の一覧は主窓だけ) | cand1-o1 | 未特定 | alpha1 の DCX_LOCKWINDOWUPDATE・ImageList drag の diff は RT-B の候補。`diag-g9-fl-ulw` を載せて ULW の経路を読む |


## PM の追記(2026-11-07 10:57): 10:00〜10:55 の切り分け(実エンジン・実 Chrome 156・node G3・Wine 10。非計時の機能確認と決定論の Export。依頼者の機械は 0 件)

| 番号 | 状態 | 根拠(実ファイル) | 区分 | 次 |
|---|---|---|---|---|
| O3(About のロゴ・staff roll) | **テスト bundle で解消**: seed に BigFruit.png と AboutTxt.txt の行を足しただけで、フルーツのロゴ・歯車・『FL STUDIO 2026』・スクロールする staff roll が出る(PM が画像で確認、layered control 246×345。cand1 では 3×3) | gpu8-results/0319 logs seed2b-about(`fl-engine-test-rt1-seed2-8128f5be`、page-rt1) | C(資材の欠落) | seed だけの本番変更 0330b(PM が検証して push する) |
| O16(日本語が豆腐) | **テスト bundle で解消**: seed に C:\Windows\Fonts\msgothic.ttc を足すと Playlist 見出し『ぷにぷに電機 › 日本語ボリューム（Solo）』とチャンネル名が日本語の字形で出る。Wine でも msgothic.ttc の有無だけで豆腐 ↔ 日本語(J4)。FL の文字は GDI でなく QuickFontCache(AGG + FreeType)で、起動の最初に全体を同期で読む → eager な seed の行にする判断 | 同上 seed2b-ja、opt1 0320 §13 | C(資材の欠落) | 正本の TTC は opt1 の再現版(sha 1ab00fd8…)。実 Chrome での確認は正本の TTC では未実施(gpu1 の e13-seed3 bundle で行う) |
| O5(Monitor > で畳むと止まる) | **原因を特定し、修正を確認**: Welcome wizard(モーダル)が開いたままの display change で callback の入れ子 17 段(WM_DISPLAYCHANGE → SetWindowPos → WM_WINDOWPOSCHANGED → DefWindowProc の WM_SIZE → 子・兄弟・孫…)が起き、上限 16 の FLE_CB_MAX_DEPTH で abort。入れ子は正当(host の C stack 使用 180,656 B = 8 MiB の 2.2%、1 段最大 34,912 B)で、上限 16 に根拠は無い = (a) 余計な処理。数の上限をやめ、スタックの残りで止める。alpha1 の修正版(0319-callback-stack-limit.diff)を node G3 の wasm で確認(rz-5: 止まらない)、HEAD のコピーで native の全テスト 215 executable: passed 2418、failed 0(alpha1 の測定。PM は dry-run を HEAD と tree-rtb で確認)。**どの build にも未搭載**(RT-B の native テストで最初の版が SEGV と stack 予約の欠陥を持っていたため、直した版を作り直した) | alpha22-results/tables/0319-callback-native-tests-HEAD-fixed-FULL-215.txt、0319-cb-chain-rz4.txt | (a) | RT-B の増分 build で載せる。他スレッドの経路(region − canary)は実スレッド版で未実行 |
| O9(Settings→General のスクロール) | cand1 でも E13 とビット単位で同一の崩れ。PM の仮説(子窓の移動で親の空いた領域が無効化されない)は alpha1 の harness で**否定**(面 M の 8 通りは契約どおり)。残る候補は FL 側の事実(スクロール容器の style に WS_CLIPCHILDREN が付くか、窓を持たない部品の描画順) | alpha1 0319 §15 | 未特定 | paint 診断(diag-g9-fl-paint)を RT-B に載せて FL の run のログで決める |
| O1(clip のドラッグ) | 変わらず。ImageList の呼び・unsupported は無い。alpha1 は GetDCEx(DCX_LOCKWINDOWUPDATE) の契約の穴(runtime は LockWindowUpdate 中の DC を全て捨てる。Learn は DCX_LOCKWINDOWUPDATE で描画を許す)と ImageList の drag 系 23 関数の不足を確定 | alpha1 0319 §13・§14 | (b) | dcx と imagelist-drag を RT-B に載せる |
| G9 の Filter | **Wine と一致しない**(ins1-lp)。Wine の変種は 1・4・16 CPU と再実行で決定的(差 RMS 0.0005〜0.0009、3〜9 サンプル ±1 LSB)。wasm は frame 210,723〜211,680 が厳密に 0、99.77〜100.8 s がほぼ無音で、Wine は同区間を素通し(基準と bit 一致)。wasm の ins1-bp にも同じ区間がある = フィルタの型・係数に依らない = DSP の丸め・MXCSR・x87 ではなく、スロットが有効になる仕組みの差 | opt1 0320、alpha1 0319 §12・§18 | 未特定 | E4(Wine を wasm の registry seed の値で走らせる)→ 結果次第で診断 build(Filter の image への呼びの全列) |

### 取り下げた仮説(記録)
1. 『プロセッサ数(wasm は 1、Wine は 16)が Filter の差を作る』: Wine を 1 CPU(sysinfo.exe で GetSystemInfo=1 を確認)にしても出力は不変(E1、E2 の 4 CPU も同じ)。**否定**。古い 0307 §2c の E1(WINE_CPU_TOPOLOGY=1:0)は、その環境変数がこの Wine 10.0(Ubuntu)に存在せず 16 CPU のままだったので根拠が無かった(opt1 が訂正)。
2. 『子窓の移動で親の空いた領域が無効化されない(O9)』: alpha1 の harness で否定。
3. 『RT-A の Export が Problems 窓が出ないので失敗した = 実スレッド版の機能の差』: ci1 が scenario の読みで否定(Esc で閉じた後の clickWindow の空振り)。PM の読みの誤り。
4. 『RT-A は E13 の page と E14 の page のどちらでも動く』: 誤り。実スレッド版の glue は Blob URL から import すると pthread の Worker の URL が解決できない(Invalid URL)。page-rt1(tree-rt1 の app)が要る。

### 実スレッド版 RT-A の事実(ci1・mt3、PM が実ファイルで確認)
- bundle `fl-engine-test-rt1-91bf78f6`: wasm 158,362,507 B、**S1 全体 = 158,362,507 ÷ 84,942,208 = 1.8644**(hybrid strip 61 units、dropped 3875 functions)。cand1 の 1.966 は strip の印が古いまま skip された build の artifact(C 部分に 3,875 関数が重複)。
- smoke(page-rt1): engine-idle、unsupported 0、n=16 → N=8。first idle 8,256 ms は load 6 前後の単発で測定ではない。
- **Export は N に依る**: FLE_NUM_CPU=2 の Export は cooperative の Export(sha 94c93666…4090)と**バイト一致**。既定(N=8)の Export(sha b8fb623d…)は 22 フレームが最大 1 LSB 違う(最初は frame 538,871、545,827、552,349、557,384、846,497。Wine の elog16 とは 20 フレーム)。= 『Export の sha が n に依らない』の関門(G4)は N=8 では満たされない。N=8 の決定論(再実行)と N=4 は未測定。


## PM の追記(2026-11-07 11:55): E4 の結果(Filter の差に registry は無関係)、seed だけの変更の push、O3/O16 の実 Chrome 確認(依頼者の機械は 0 件)

| 項目 | 状態 | 根拠(実ファイル。PM が WAV・画像を直接読んだ) | 区分 | 次 |
|---|---|---|---|---|
| Filter の差(G9 の阻害要因)の registry 仮説 | **否定**。Wine の prefix-e3 に alpha1 の minimal `.reg`(Image-Line の section 24→2、ASIO・Devices・Shared\General・Shared\Plugins を消す)を入れても ins1-lp の Export は対照と同じ(対照比 5 フレーム ±1 LSB)。[210723,211680) は 0 にならず(rms 1773、wasm は exactly 0)、99.77〜100.8 s は無音にならない(rms 9955、wasm は 1582)。Wine の効果の開始は 211,681 のまま | opt1 の `exports/runs/e4-ctl.wav`・`e4-reg.wav`(5,729,968 フレーム、sha 99ee1147… / 5f5cae9d…)、alpha1 の a22-filter-structure の出力 | 仮説の否定 | 診断 build(alpha1 の `diag-g9f-filter-calls.diff`、RT-B の build の後に 1 本) |
| 957 フレームの正体 | Filter の block 長でも PDC の latency でもなく **tick の端数**: 211,680 = tick 1536(PPQ 96、200 BPM、1 tick = 137.8125 フレーム = 16 拍)。wasm の 0 の区間の終わりと Wine の効果の開始が同じ tick(多数の clip が一斉に始まる)。長さの差はどの変種でも 1 tick(138 フレーム) | alpha1 0319 §20(既存データの数) | 読み(原因は未特定) | 診断 A〜D(Filter への呼びの前後の peak、Filter→外の呼びの target、plug-in object の dump) |
| O3(About のロゴ・staff roll) | **本番相当の bundle で解消を確認**: 本番の wasm・glue・DLL pack のままで seed に BigFruit.png と AboutTxt.txt を足すと、フルーツのロゴ・歯車・『FL STUDIO 2026』・Trial v26.1.7 [build 5653]・staff roll が出る(PM が画像で確認)。unsupported 0 | gpu8 の `rt-e13s3-about/about-28s.png` | C(資材の欠落) | **push 済み(917f5a05)**。GitHub の Deploy(冷間 build 約 60 分)が 11:50 に開始。deploy 後に本番 URL で gpu1 が確認 |
| O16(日本語) | **本番相当の bundle で解消を確認**: 正本の msgothic.ttc(sha 1ab00fd8…)入りの seed で Playlist 見出し『ぷにぷに電機 › 日本語ボリューム（Solo）』とチャンネル『サンプ..リナ』が日本語の字形。**対照**(本番の seed のまま)は同じ箇所が豆腐(□)。unsupported 0 | `rt-e13s3-ja/pic-ja-main-10s.png`、`rt-e13ja-control/pic-ja-main-10s.png` | C(資材の欠落) | 同上 |

### seed だけの本番変更(917f5a05、2026-11-07 11:50 push)
`BigFruit.png`・`AboutTxt.txt`・正本の `msgothic.ttc`(Noto Sans CJK JP の subset、再現可能な生成、sha256 1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69、OFL・COPYRIGHT 付き)の 3 行を vfs-seed-idle1.tsv に追加。pack_seed.py に guest path の重複検査、ci/plan.sh は『seed の資材だけの変更は extract / seed / bundle / prefix の key だけを変える』形に、ci/test-hybrid.sh に検査 3b。`ci/test-hybrid.sh` は all hold。**エンジンのコード(wasm・glue・DLL pack)は変えていない。** この push は plan.sh の key が変わるため冷間 build 1 回(約 60 分)。以後の seed だけの変更は extract → seed → bundle だけ。seed pack は本番比 br(品質 11)+1,300,446 B。依頼者の機械の値は 0 件。deploy が終わるまで他の push はしない。


## 依頼者の追加報告 O19(2026-11-07 12:48): パフォーマンス — Solo の限定条件でも GUI 操作で再生が破綻する

**原文の要旨**: 限られたトラックのみの Solo ならリアルタイム再生がギリギリ成立すると報告したが、『Mixer ウインドウが表示されている』『マウスを動かしている』などの条件でリアルタイム再生が破綻する。つまりオーディオ処理と GUI 系処理が相互に影響し合っていると判断できる。推測: オーディオ処理と GUI 処理を別 Worker で動かす案は優先度がかなり高い。

| 番号 | 依頼者の報告 | PM の読み | 区分 | 担当 | 状態 |
|---|---|---|---|---|---|
| O19 | Solo の限定条件でも、Mixer 表示・マウス移動で再生が破綻する。オーディオと GUI が相互に影響している | P3。協調 1 Worker(E13)の観測として整合する(1 Worker に全部が載る。我々の P3 の run は『play して放置』が主で、依頼者の条件〔Mixer 表示、マウス移動〕を含まず、この違いを測っていない)。RT-A/B(GIL=1、N=8)でも GIL が GUI の重い区間とオーディオ thread を直列化するなら、同じ構造が残る。**未測定: RT で依頼者の条件を再現したか** | P3 の測定の欠落 + 設計(GIL の粒度、audio thread の分離) | ci1(依頼者の条件 4 つ〔放置・Mixer 表示・Mixer 上でマウス往復・Playlist 上でマウス往復〕を E13 と RT で n=2 から。計時の窓で)、mt3(GIL 保持時間の見積り・待ち時間の診断の設計・別 Worker の 3 案。読みのみ、期限 13:20) | 測定と設計の準備中 |

PM の判断: (1) 『オーディオと GUI を別 Worker にする』は、現在の RT(ゲストの thread = 別 Worker)ですでにその形かを実装で確かめるのが先(mt3 の読み)。GIL がオーディオ thread を GUI の重い区間の間止めるなら、別 Worker にしても直列のまま。(2) 測定の目標を『play して放置』から『依頼者の操作を含む条件』に広げる(P3 の gate に追加: Mixer 表示・マウス移動の条件で underrun 0)。(3) これまでの RT-A の『underrun 0』は放置条件のみで、依頼者の条件では未確認。依頼者の機械での値は 0 件のまま。


## 依頼者の追加報告 O19 続き・O20(2026-11-07 13:20。**いずれも本番 E13 での依頼者の観測**。我々の測定ではない。記録のみで、原因は未特定)

| 番号 | 依頼者の観測(要約) | PM の読み(**未確認**) | 区分 | 担当・次 |
|---|---|---|---|---|
| O19 続き | 『Playlist を含む全 FL ウインドウを非表示にしたとき、全トラックを有効化しても曲を通しておよそ 1 割の箇所で破綻した。今までの観測で最も良い』『全窓を非表示にすると改善するのは、GUI 処理がオーディオ処理を圧迫している推測を強める』 | 協調 1 Worker(E13)で、FL の窓の描画が audio の処理時間を奪う構造と整合する。RT(GIL=1)でも GUI の重い API が GIL を保持する間 mixer が待つなら同じ構造が残る(mt3 の 0315f の読み: GIL は runtime の C の実行中だけ、API 1 回ごと。BitBlt・StretchDIBits・fle_raster_wrote は 1 回が数 ms〜10 ms 超)。**『破綻の約 1 割』を E13 と RT で測る**ため、ci1 の O19 の条件に allhidden(全窓非表示)× song を追加し、underrun の割合を表に出す | P3(設計 + 測定の欠落) | ci1(13:40 の計時の窓)、mt3(FLE_DIAG_GIL の診断は RT-D3 候補) |
| O20-1 | Tempo、Song Position、Pattern Selector など、ウインドウ 1・2 行目の要素で**右クリックができない** | 1・2 行目だけか他の窓でも出ないかが要点。ブラウザ側の `contextmenu` / button 2 の扱い(app/src/input/pointer.ts、tracker.ts)か、FL のカスタム窓が使う API(GetKeyState(VK_RBUTTON)、SetCapture、TrackPopupMenu、GetCursorPos)の契約の穴(b) | (b) の疑い | gpu1(再現・log)、alpha1(契約の読み) |
| O20-2 | **Tempo をドラッグで少しでも上下すると、Tempo が確実に 522 になり、以後ドラッグしても 522 から動かない**(一度ドラッグすると二度と動かせない) | 仮説: FL はドラッグ中にカーソルを隠し、毎回 SetCursorPos(中心)で戻して『GetCursorPos − 中心』を増分に使う。runtime の SetCursorPos は静的な位置を持つだけでブラウザの実ポインタは動かない(fle_win.c:5633 `set_pointer`)ため、次の pointermove で増分が大きな一定値になり上限 522 に張り付く、という構造が成り立つか。**この読みは未検証**(alpha1 が Learn/runtime/page を読み、必要なら観測のみの diag を設計) | (b) の疑い(SetCursorPos/GetCursorPos/ShowCursor/ClipCursor とポインタの契約。ブラウザでは Pointer Lock の movementX/Y が候補) | gpu1(再現: 各段の値と pointermove の座標)、alpha1(契約の読み) |
| O20-3 | マウスカーソルがまだ変化しない | O18 の継続(HCURSOR の絵と hotspot をブラウザの CSS カーソルへ渡していない)。ci1 の差分待ち | (b) | ci1 |

PM の判断: (1) O20-2 は『ドラッグした値が一定値に張り付く』という実害が明確で、以後 Tempo を操作できないため、GUI 操作の基本として優先して調べる(gpu1 の再現 → alpha1 の契約の読み)。(2) 『なぜこれまで漏れたか』: 我々の実 Chrome の確認は Tempo のドラッグを含んでいなかった。操作の網羅(1・2 行目の要素のドラッグ・右クリック)を gpu1 の scenario に足す。(3) 依頼者の機械の値は 0 件のまま。

## PM の追記(2026-11-07 13:24): 12:30〜13:20 の切り分け(実 Chrome 156・Wine 10。依頼者の機械は 0 件)

- **Filter の不一致(G9 の阻害要因)の読みの訂正**: PM は opt1 の fhook3(Wine の Filter の呼びの記録)を読み、『Export の 210,723〜211,680 フレームで Filter は 0 を書き、master の非ゼロは Filter の外から来る』と書いた。**これは PM の誤り**(診断の累積フレーム数 scum を Export のフレーム位置と取り違えた。Wine の scum は Export の前の mixer の空転を含む)。opt1 が時刻で読み直し、Export の 210,723〜211,680 は scum 約 1,223,774〜1,224,731 に当たり、そこでは Filter は非ゼロの入力(src_peak 0.46)を処理して減衰した値(dst_peak 0.031)を出している。wasm の診断 v1 も累積の数え方の欠陥で問題の区間に届かなかった(alpha1 の診断の誤り)。診断 v3(Filter の image へ外から入る blk 型の呼びだけを数える)を RT-D2 の build に載せた(13:00 開始)。レジストリ(E4)・CPU 数・実スレッド・Delay 2・callback の修正は原因から除外済み。
- **RT-B の pthread mutex abort(実 Chrome。gpu1 の 9 本)**: RT-B は o1/o1b(激しいクリップのドラッグ・Edit メニューの後に display change)で 2/2 abort(診断なしの bundle でも)。RT-A(診断 FLE_DIAG_WIN のみ)は o1b で abort しない(n=1)。min(project を開いて drawer の開閉のみ)と clipdrawer(ゆっくり 1 回ドラッグ)は abort しない。stack は Atom API(`fle_glob.c` の `atom_arg`)→ `fle_w_read_alloc` の malloc → dlmalloc の mutex の再入(mt3 の 0315f の読み)。原因は未確定で、次の診断(atom_arg の記録・GIL 待ちの分布・関数名)は RT-D3 候補。
- **O5(wizard を開いたままの display change)**: RT-B で drawer の開閉 ×2 は止まらない(callback の入れ子の最大 9 段、host の stack の残り最小 8,229,984 B)。cand1 は 17 段で abort していた。
- **O9(Settings→General のスクロール)**: RT-B でも不変。**Balance の GUI**: client 領域は未描画のまま。alpha1 の読み = 窓 174〜177 は WS_VISIBLE なしで作られ ShowWindow なしに可視になり WM_PAINT が届かない(class (b)、`upd_show_descendants`)。差分 `0319-show-descendants-paint.diff` は native の面 R で効果を確認したが、O9 は直さない(別の穴)。次の増分の候補。
- **O1(クリップのドラッグ)**: RT-B でも浮いたクリップの絵は出ない(ULW 0、layered 0)。Edit メニューの Undo は『Undo playlist move clip』で FL は移動を内部に登録している。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 13:51): Filter の不一致の切り分け(RT-D2 の診断 Export と Wine fhook4 の突き合わせ)+ F5 + O20 の右クリックの手がかり。依頼者の機械は 0 件

**実際に実行した物**: mt3 の RT-D2 bundle(`fl-engine-test-rtd2-297e2ac6` = RT-B + 診断 v3、TEST・push 前)を ci1 が実 Chrome 156・page-rt1・N=8 で ins1-lp の Export(FLE_DIAG_G9F=1 を manifest のコピーに足しただけ。13:29:48〜13:34:36、rc 0)。opt1 の Wine fhook4(Export 位置基準)。PM は両方のログ・WAV を Python/numpy で読んだ(道具: `docs/investigations/pm-0349/pm-filter-compare.py`、出力 `pm-filter-compare-0349.txt`)。

- **診断は Export を変えていない**: RT-D2 診断 Export の WAV は 5,729,830 フレーム、sha256 `59967806…95c9`。RT-B の ins1-lp との差は 5 フレーム ±1 LSB(ci1 の報告)。engine log は 14,434 行(20,000 行の上限に未達。gzip で `docs/investigations/ci1/0335/rtd2-ins1lp/engine-log.txt.gz`)。
- **wasm の Filter の入力は Wine と同じ位置で初めて非ゼロになる**: wasm の Eff_Render は最初の呼び(scum 0)から Length 138/138/137… の一定列で、Wine の fhook4 と比較できた 2,541 位置で Length の不一致 0(wasm に Export 前の空回しは無く、wasm の scum = Export の位置)。Insert 1 の Filter(wasm の Self 1、Wine の self 0)は **両方とも Export frame 211,680 の呼びで初めて入力が非ゼロ**(wasm src_peak 0.492・dst_peak 0.2355、Wine src_peak 0.566・dst_peak 0.2352)。区間 [210723,211680) の呼びは両者とも入力 0・出力 0。= **Filter の中が 0 を書いているのではない。差は Filter の外**。
- **WAV の事実(numpy)**: wasm の ins1-lp は frame [210342,211680) の 1,338 フレームが厳密に 0。Wine の ins1-lp と wasm・Wine の原 FLP は [210342,210723) の 381 フレームだけ 0 で、210,723 から非ゼロ(rms 1773.5)。wasm の原 FLP と ins1-lp の最初の差は frame 210,723。wasm の ins1-lp にはほかに [4435770,4445280) の 9,510 フレームの厳密な 0 もある(Wine には無い)。= 210,723 から出る音は新しい音源(トラック 3 のボーカル、tick 1529 ≒ frame 210,720、Insert 1)で、car crash の尾は 210,342 で全 WAV とも 0 になる。**opt1 の 13:44 の『区間の音は Insert 2 の尾』は WAV の 381 フレームの零と合わない(ボーカルが主)と PM が訂正を送った**。
- **PM の読み(未確認)**: Wine では Insert 1 の Filter の slot の無音カウンタ(+0xa0)が 176,400(限界)で止まっている間、ボーカルは Filter の入力に出ず、それでも master には出る(迂回)。wasm ではこの区間の master が 0。迂回の経路(止まった slot の処理、Insert の出力・master の sum)のどこかで、lift した命令の意味の誤りか、FL が読む外部の値の差があるかのどちらか。**どちらも未確認**。次: alpha1 が RT-D3 に slot 状態(f_cf2d00)・Eff_Render の先頭 8 サンプル・Insert 出力と master の sum の peak の診断を載せる(14:00 に差分)。
- **訂正の履歴**: 13:20 の PM の『Filter は区間で 0 を書き、master の非ゼロは Filter の外』は scum 基準の誤りで取り下げた。今回は Export 位置基準で、結論の前半(区間で Filter の入力は 0。Filter の外に差がある)は、結果として同じ方向に戻ったが、根拠は別(fhook4 と RT-D2 の同位置の突き合わせ)。
- **F5(新、class b)**: F5 は FL に転送されず(`app/src/input/keymap.ts:60`)ページが再読み込みされ、project が消える(ci1 の `o19-solo12-keys-0-e13/after-F5-playlist.png`: project なしの新しい FL と Welcome wizard)。FL では F5 が Playlist の切替。gpu1 が keymap 全体を調査中(差分は PM の許可後)。F9 の 1 回目は Mixer を前面に出し 2 回目で消える。Playlist は閉じるボタン(1905,97)で消え、主窓と Browser のパネルだけが残る(allhidden)。Alt+F8 は Plugin picker の全面オーバーレイを出す(O19 では使わない)。
- **O20 の再現(gpu1、E13 の本番 wasm、実 Chrome 156)**: Tempo(開いた直後 200.000)は 3 px の小さな上下 1 回で **522.000** になり、以後 80 px 下・20 px 上・小さな上下のどれでも 522.000 から動かない(依頼者の観察と同じ。engine は idle のまま、unsupported 0)。右クリックは Tempo・Song Position・Pattern Selector・Channel rack・Playlist の空き・Mixer の insert 名のすべてでメニューが出ない(1・2 行目だけではない)。左クリックの File メニューは出る。右ボタンの down/up は 6 組すべて guest の正しい screen 座標で engine へ送られている。**PM が `git grep` で確認**: runtime は `fle_input.c:868` で DefWindowProc の msg 0x200〜0x209(WM_RBUTTONUP 0x205 を含む)と 0x007B(WM_CONTEXTMENU)を『結果 0』で終わらせており、WM_RBUTTONUP から WM_CONTEXTMENU を送る所が runtime に無い(0x007B は fle_filedlg.c の GUID と fle_raster.c の表にしか出ない)。= (b) の外部 API の契約の不足の候補(Windows では DefWindowProc が WM_RBUTTONUP から WM_CONTEXTMENU を生成する)。alpha1 に Learn の確認と差分(+ native テスト)を依頼。Tempo の 522 は FL の Tempo の上限と読める(gpu1)。SetCursorPos の仮説は alpha1 が読みで検証中。**いずれも未修正**。
- **pthread abort(mt3 0315f の追補、読みのみ)**: dlmalloc の normal mutex の再入と読める。候補は ①別の API(free/realloc/memalign)の途中で握ったまま suspend/longjmp/proxy が入る、② `emscripten_futex_wait` の `_emscripten_yield` が proxy の仕事を実行しその中で malloc、③ JSPI の suspend が libc の中に入る(入口が無く除外に近い)、④ mutex の語自体が wild write で壊れている(新、未測定)。display change の後に新しく走る malloc/free の経路は imagelist-drag だけ(読み)。④ は停止時に 2 つの mutex の type/lock/waiters を読めば区別できる(page 側の小変更。RT-B 専用の番地)。原因は未確定。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 14:08): O19 ラウンド 1 の測定(counted、n=1)+ alpha1 の RT-D2 の読みの訂正 + O20 の Tempo の方針。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実エンジン(E13 = 本番 wasm と RT-A N=8 = 実スレッドのテスト bundle)を実 Chrome 156・この機械・headless・模擬入力で、参照プロジェクトの song(全トラック)を 6 本、E13 と RT-A を交互に(13:49:48〜14:04:02)。各 run の開始前に load ≤ 8・I/O 圧 0・swap 入出力 0・他の Chrome 0・Wine 0・MemAvailable ≥ 12 GB を記録し、全 run が counted(開始 load 0.62〜3.45、MemAvailable 15.9〜26.8 GB)。PM は表(`tools/o19-table.py` の出力、`docs/investigations/ci1/0336-o19-round1/o19-round1-table.md`)と、各 run の再生前の画像(shown = 全窓、hidden = Mixer だけ消える、allhidden = 主窓と Browser のパネルだけ)を見た。窓 = 再生の 10 秒後からの 58 秒。

| 条件 | エンジン | 窓の underrun(量子 / 窓の量子) | lag cuts | present/s | Worker の CPU(コア) |
|---|---|---|---|---|---|
| shown | E13 | 0 / 20,033 = 0.00% | 1,007 | 15.2 | 0.88 |
| shown | RT-A N=8 | 0 / 20,039 = 0.00% | 0 | 31.9 | 1.96 |
| hidden | E13 | 0 / 20,032 = 0.00% | 0 | 20.6 | 0.83 |
| hidden | RT-A N=8 | 0 / 20,032 = 0.00% | 0 | 39.8 | 1.84 |
| allhidden | E13 | 0 / 20,033 = 0.00% | 0 | 20.5 | 0.72 |
| allhidden | RT-A N=8 | 0 / 20,036 = 0.00% | 0 | 45.7 | 1.73 |

- **読み(n=1。結論にしない)**: 6 本とも窓の underrun は 0。依頼者の『Mixer 表示や全窓表示で破綻』『全窓非表示でも約 1 割破綻』は、**この窓の underrun の量では再現できていない**(E13 でも、shown でも)。差が出たのは lag cuts(E13 shown だけ 1,007)・present/s・Worker CPU。present/s は GUI の窓が少ないほど増え、Worker CPU は減る(E13 0.88 → 0.83 → 0.72、RT-A 1.96 → 1.84 → 1.73)= GUI 描画が CPU を使う、という依頼者の『GUI がオーディオを圧迫』と同じ向き。RT-A は E13 より present/s が約 2 倍。窓は 60 秒で、曲の重い区間(音楽の 40 秒以降と読む)に当たっていない可能性があるため、再生開始後 40〜100 秒の窓、音の数の基準(PCM の零の連続、nonSilent の連続性)、ラウンド 2 を次の窓で測る。
- **訂正(PM)**: 14:00 の私の報告の『E13 song shown の underrun は 4.06%』は誤り。`audio-p3.json` の `detail` は run 全体の値で、再生前の読み込み中の 1,401 量子を含んでいた。窓の値(上表)を使う。
- **alpha1 の RT-D2 の読み(14:07 の report)**: wasm の Filter(Insert 1、Self 1)は Wine の self 0 と 2,541 位置で開始位置も Length も一致(不一致 0)し、入力は区間で 0、expos 211,680 で初めて非ゼロ(出力 0.2355 対 Wine 0.2352)。**差は Filter の外**。訂正 3 点: (A) PM の『wasm に Export 前の空回しは無い』は誤り。wasm にも blk0 の呼びが 844 行ある(Length が 99・38・61 などばらばらを含む。PM が `engine-log.log` で確認: blk0 844 行、合計 112,730 フレーム、source は 0x31c… で blk1 の 0x2b… と別、blk0 は blk1 の最初の行より後に 1 本も無い)。空回しが scum に入らなかったのは診断の blk 判定の上限(0x30000000)による結果で、意図ではない。RT-D3 では診断自身が Export の開始を決めて expos を出す。(B) 211,680 の src_peak が wasm 0.492 / Wine 0.566 なのは定義の違いではなく入力の差(比の中央値 0.580、dst の比の中央値 0.314)。Filter の入力を作る前段(Insert 1 の slot 0 の Balance とそこへの合算)が違う可能性。原因は未確認。(C) wasm ins1-lp の厳密な 0 の連は [210342, 211680) の 1,338 フレーム。**Insert の出力と master の sum を作る関数の VA は確定できていない**(呼び元のループが vtable 経由)。二分探索の hook 点として確実なのは `f_cf2a90`(RVA 0x8f2a90)の戻りで見る task+0x54(slot の最終出力)と task+0x4c(dry)の peak(Wine と wasm で同じ VA)。RT-D3 の診断はこれと expos、src_first8/dst_first8 を載せる。
- **O20 の Tempo の方針(PM の決定、14:03)**: ゲストが SetCursorPos/SetPhysicalCursorPos で動かしたポインタは、次の pointermove でブラウザの実位置に上書きされて消える(`fle_input.c:134`、`fle_win.c:5633`)。FL の drag の 3 関数(VA 0x7f4ca0/0x7f4ec0/0x7f4e20、gpu1 の静的な読み)は、ドラッグ中にカーソルを隠し、窓の端で反対側へ回り込ませ、差 × 倍率で値を動かす。Windows の契約(SetCursorPos の後の入力は、動かした位置からの相対で届く)の不足 = class (b)。第 1 段: runtime のゲストのポインタを『ホストの最新の絶対位置 + オフセット』の仮想ポインタにする(alpha1 が差分と native テストを 14:15 までに)。第 2 段: ゲストが ShowCursor(FALSE) の間の Pointer Lock(movementX/Y)。gpu1 が設計を読む(14:30)。Tempo の 522 は FL の上限、回り込みが毎 move で起きるという仮説は 14:15 以降に gpu1 が実 Chrome で測る。未修正。
- **mt3 の RT-D3 の build script**(`mt3w/0315/tools/rtd3-build.sh`): `FLE_DIAG_ATOM`・`FLE_DIAG_GIL`・alpha1 の slot/master 診断・name section(`--profiling-funcs` と wasm-opt/wasm-merge の `-g`)を RT-D2 の木に足す診断 bundle(TEST のみ、push しない)。GO 条件: 14:15 以降、alpha1 の差分が native テストを通っている、MemAvailable ≥ 8 GB、swap OUT 0。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 14:20): O19 ラウンド 2(n=2 目)の測定と、O20 の Tempo の読み(gpu1)。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実エンジン(E13 と RT-A N=8)を実 Chrome 156・この機械・headless・模擬入力で、song(全トラック)を 6 本、ラウンド 1 と同じ条件・同じ順で(14:05〜14:19。開始 load 1.95〜3.5、swap 入出力 0、counted)。表は `docs/investigations/ci1/0336-o19-round1/o19-round2-table.md`(ラウンド 1 は同ディレクトリの `o19-round1-table.md`)。窓 = 再生の 10 秒後からの 58 秒。

| 条件 | エンジン | ラウンド 1 / 2 の窓の underrun | lag cuts(1 / 2 回目) | present/s(1 / 2 回目) | Worker の CPU コア(1 / 2 回目) |
|---|---|---|---|---|---|
| shown | E13 | 0 / 0 | 1,007 / 1,031 | 15.2 / 16.4 | 0.88 / 0.87 |
| shown | RT-A N=8 | 0 / 0 | 0 / 0 | 31.9 / 30.2 | 1.96 / 1.99 |
| hidden | E13 | 0 / 0 | 0 / 72 | 20.6 / 17.1 | 0.83 / 0.85 |
| hidden | RT-A N=8 | 0 / 0 | 0 / 0 | 39.8 / 38.8 | 1.84 / 1.93 |
| allhidden | E13 | 0 / 0 | 0 / 0 | 20.5 / 19.1 | 0.72 / 0.79 |
| allhidden | RT-A N=8 | 0 / 0 | 0 / 0 | 45.7 / 43.2 | 1.73 / 1.81 |

- **読み(n=2。結論にしない)**: 12 本とも窓の underrun は 0(窓の量子は各 20,029〜20,040)、engine gap も 0。依頼者の『Mixer 表示や全窓表示で破綻』『全窓非表示でも約 1 割破綻』は、**この窓の underrun の量では 2 回測っても再現できていない**。再現した差は、E13 の shown だけ lag cuts が 1,000 台(1,007 と 1,031)、hidden・allhidden は 0〜72、Worker CPU は窓が少ないほど減る(E13 0.87→0.85→0.79)、RT-A の present/s は E13 の約 2 倍(2 回ともほぼ同じ値)。first idle は E13 が 4.4〜5.3 秒、RT-A が 8.0 秒(n=1 ずつ)で、P1 の 10 秒以内には入るが E13 より約 3.5 秒長い。
- **次の測定(PM の決定)**: A = `audioDump` に worklet の 50 ms ごとの列(peak、nonSilent)を足す(ドライバのみ、ci1)。C = song を最後(最長 300 秒)まで 1 回再生し、10 秒刻みの窓ごとの underrun・lag cuts・engine gap・present/s・無音の連続を出す(14:40 以降、E13 shown、RT-A shown、E13 allhidden、RT-A allhidden、n=1、約 28 分)。窓の外(曲の他の区間)に破綻があるかを見る。
- **O20 の Tempo の読み(gpu1、読みのみ・実行 0、14:19)**: FLEngine の drag の move 関数(VA 0x7f4ec0)は毎 WM_MOUSEMOVE で『値の増減 = (前回位置 − 現在位置) × 倍率』を使い、前回位置が縮めた矩形(モニター矩形 ∩ GetClipCursor を 4 辺とも 32 px 縮めた (32, 32, W−32, H−32))の外なら、前回位置を矩形の幅/高さだけ反対側へずらして SetPhysicalCursorPos で実カーソルを回り込ませる。runtime は次の pointermove でゲストのポインタをブラウザの実位置に上書きする(`fle_input.c:146`)ので、次の差が約 H−64 の一発になり、Tempo の上限 522 に張り付く。Tempo の箱は y=20 < 32 なので、drag の最初の move で必ず回り込みが起き、drag のたびに begin が前回位置を作り直すので 2 回目以降も 522(PM が runtime で確認: `fle_input.c:146`、`fle_cursor.c:115` の GetClipCursor は画面全体を返す)。**反証可能な予測 3 つ**(drag の最初の move の後 pointerY ≈ H−43、y ≥ 32 から始めると飛ばない、pointerX 不変)を 14:25 の実 Chrome で確認する。外れたら読みは誤り。方針: 仮想ポインタ(ゲストの位置 = ホストの絶対位置 + オフセット、SetCursorPos でオフセット更新)を runtime に入れる(alpha1)。`fle_win_pointer_set` の呼び出し元は 2 か所(host move と `apply_display` の丸め直し)なので、仮想ポインタは 3 経路(host move、SetCursorPos、display change)の native テストで確認する。未修正。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 15:47): gpu1 の F5/keymap の読み + alpha1 の O20 の結論(読みのみ)。依頼者の機械は 0 件

**実際に実行した物**: gpu1 の読みは Chrome 0・Wine 0・実行 0（keymap.ts・pointer.ts・Chromium/Firefox の予約表・FL 表の読み）。alpha1 の O20 の結論も読みのみ（Chrome・Wine・native の cargo は 0）。PM が実ファイルで確認: `fle_input.c:146` は host move でゲストのポインタを上書き、`fle_win.c:5621-5633` は Set/GetCursorPos の状態保持。docs の取込: `docs/investigations/gpu8/0335-o19-keymap.md`。

- **F5(keymap の読み、gpu1、未実測)**: `keyPolicy` が forward しない + prevent しないキーは、意図的に手放すものは Tab・F5・Ctrl/Cmd+W の 3 つだけ（他は修飾・IME・unknown-code）。F5 は直接の原因（reload）。Chromium の予約一覧にも Firefox の reserved にも F5/Ctrl+R は無い、という読みではページの preventDefault で止められるはず（未実測、実機の Firefox と Chrome のどちらでも未確認）。F5 を転送 + preventDefault に変えても、デスクトップにフォーカスが無い時の Ctrl+R で再読込は残る。現在の Ctrl+R は転送済みで FL の Export が届く。F5 だけが届かない。差分は PM の許可後。
- **O20 の Tempo の結論(alpha1、読みのみ)**: 仮想ポインタで構造的に説明できる。FL の drag は ShowCursor(FALSE) でカーソルを隠し、毎 move の差で値を決める。runtime が実座標で上書きするので FL の SetPhysicalCursorPos の位置は次の move で消える。仮想ポインタ（ゲスト位置 = ホスト最新 + offset、SetCursorPos で offset 更新、画面に丸める）は FL が置いた位置からの相対をそのままつなぐので構造を消す（クラス/窓の分岐なし）。`apply_display` の 3 経路の注意も PM 確認済み。予測 3 つ（pointerY≈H−43、y≧32 では飛ばない、pointerX 不変）は gpu1 の実 Chrome に委ねる。
- **右クリック（alpha1、読みのみ）**: DefWindowProc が WM_RBUTTONUP を「結果 0 で終了」にして WM_CONTEXTMENU を送っていない = (b) 契約の不足。Learn: DefWindowProc は WM_RBUTTONUP / WM_NCRBUTTONUP の処理時に WM_CONTEXTMENU を生成する（wParam は元 window の HWND、lParam は画面座標）。実装は DefWindowProc の 0x0205 / 0x00A5 から WM_CONTEXTMENU を生成・送信する（Shift+F10 / VK_APPS は対象外）。GetKeyState/SetCapture の穴も候補として残るが、WM_CONTEXTMENU の欠落が先。
- alpha1 には仮想ポインタと WM_CONTEXTMENU の差分 + native テストを指示済み（RT-D3 診断は次増分）。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 16:12): O19 の C(曲全体 10 秒窓、counted、n=1)。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実エンジン(E13 / RT-A N=8)を実 Chrome 156・この機械・headless・模擬入力で、song(全トラック)を最後まで(再生から最長 300 秒)1 回再生した 4 本(15:44〜16:10。開始前 gate: 他 Chrome 0・Wine 0・MemAvailable ≥16 GB・swap I/O 0・I/O 圧 0。開始 load 1.79〜6.49。条件証明は各 run の再生前の画像)。A = `sdriveg2-lat.mjs` の audioDump に worklet の 50 ms post 列(time, peak, nonSilent, n, state, fmin)と audioSnap に nonSilentQuanta を足し、`node --check` 成功、`scn-lint` 654 ops / 0 problems(captured)。腕はエンジン・bundle 不変。表は `docs/investigations/ci1/0337/o19c-table.md`(10 秒窓ごとの underrun 行列と要約)。

| run | underrun>0 窓 / 全窓 | underrun 合計 | lag cuts 合計 | engine gap 合計 | present/s 平均 | 無音連続 ≥100ms / 最長 | Worker CPU s(コア) / N |
|---|---|---|---|---|---|---|---|
| E13 shown | 4/30 | 413 | 4,287 | 47,962 frames | 14.4 | 2 / 74 posts(3.7 s) | 251.52(0.83) / 1 |
| RT-A N=8 shown | 0/30 | 0 | 0 | 0 | 27.8 | 2 / 74 posts(3.7 s) | 563.95(1.87) / 8 |
| E13 allhidden | 0/30 | 0 | 0 | 0 | 19.3 | 2 / 74 posts(3.7 s) | 217.86(0.72) / 1 |
| RT-A N=8 allhidden | 0/30 | 0 | 0 | 0 | 41.4 | 2 / 75 posts(3.75 s) | 520.89(1.73) / 8 |

窓ごとの行列(underrun quanta、行 = 窓、列 = run): 0〜230 秒は 4 run 全部 0。230〜240 は E13 shown 4、240〜250 は E13 shown 139、250〜260 は E13 shown 174、260〜270 は E13 shown 96、他は全部 0。270〜300 は全部 0。

- **読み(n=1。結論にしない)**: E13 shown だけが曲の終盤(約 230〜270 秒)に underrun 413 quanta を出した。E13 allhidden と RT-A の両条件は 0。underrun>0 の窓は E13 shown が 4/30 窓 = 13.3% で、依頼者の『曲を通しておよそ 1 割の破綻』に近い割合だが、固定窓の再現ではなく終盤に集中した。依頼者の『Mixer 表示で GUI がオーディオを圧迫』という観察と整合する方向だが、n=1 なので結論にしない。無音連続(A)は 4 run 実質同一(2 回、3.7 s)で、今回の underrun 差はこの指標では分離できない。E13 shown は lag cuts も 4,287 と他 run と明確に違う(他は 0)。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 16:16): opt1 の fhook5(Wine の slot hook)の結果。依頼者の機械は 0 件

**実際に実行した物**: 実 Wine 10 の FL Studio 2026 で 2 本実行した(opt1)。(a) 起動確認 1 本(16:11:17〜16:12:08、project なし)。# hook cf2d00 と # hook f2a90 の両方が入り、FL64 が 15s 後 SNl・45s 後 RNl を確認。c0000005 なし。(b) 本番 fhook5(16:12:16〜16:14:22、prefix-e2 config A、ins1-lp、FLP sha d6783e08886b4b48)。完了後は wineserver -k のみで終了した。PM が実ファイルで確認: `fhook5-slots.tsv` 5,365 行、`fhook5.wav` の sha 先頭 `db53855f1b75b0e235e3`(22,920,056 B)、Wine プロセス 0。表は `docs/investigations/opt1/0320/tables/fhook5-slots.tsv.gz`(547 KB のため gzip)。

- **WAV の同一性**: fhook5(sha `db53855f1b75b0e2`)は `elog16-ins1-lp.wav`(Wine 08:28)との差が 2 フレーム ±1 LSB(3,700,024・3,806,946)で、fhook4.wav(13:22 Wine)との差は 4 フレーム ±1 LSB(3,700,024・3,806,946・3,839,893・5,416,656)です。指示の「2 フレーム以内」は fhook4 に対しては不成立です(fhook4 も 08:28 と 2 フレーム違いでした)。出力自体は Wine の Export として安定しています。
- **slot ごとの窓(expos 210,000〜212,500)**: Insert 47 の slot(slot_idx 2、ptr 488eec0)は 18 呼びで入力 0・出力 0。Insert 1 の 2 slots(ptr 4921af0 / 4921c70)は expos 211542 を境に 7 呼びの入力が非ゼロになり、出力も非ゼロ(最大 out 0.469/0.206)です。fhook4 の Filter の「frame 211,680 を境に入力が非ゼロ」の動きと一致します(slot の expos の数え方と Eff_Render の expos は定義が異なり、差は 138 フレームほどあります)。
- **Balance の同定はこの表ではできません**(全行が slot_idx 2 のため)。Balance の Eff_Render は `hooktables/balance-effrender.tbl` で次の Wine で取る予定です。
- v9 の起動失敗(c0000005)の原因は、v9 で足した空の汎用 hook の mod が NULL のまま ldr_cb で参照される構造と、x_eval の新 SPEC(mf/d/md)のガード不足でした。X_READQ ガードと空 hook の skip で修正しました(再コンパイル DLL sha 先頭 `9b17052d5b094874`)。静的な修正で Wine は実行していません(counted 中は Wine 0 を維持)。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 16:33): gpu1 の O20 Tempo 検証と mutex(o1b)の読み直し。依頼者の機械は 0 件

**実際に実行した物**: 実 Chrome(画像「私の 1 本」、Chrome 156 headless、16:15〜16:19)、page-rt1 のコピー(`/home/ubuntu/work/gpu8/page-rt1-o20/app`: worker.ts が毎 engine idle で `mark('pointer', {x, y})`、main.ts が履歴 `pointerTrace` を残す。製品/E13/E14 の page には触れていない)、RT-B(`fl-engine-ci-0a33cda7`、診断あり、assets-rtb の manifest コピー)。条件(開始時 16:11): MemAvailable 27.4 GB、load 3.65、swap 入出力 0/0、mt3 の RT-D3 build(wasmopt)継続中(並行)。FORCE=1 なし。`scn-o20-tempo-ptr.json`。画像・数値: `lift/out-git/gpu8-results/0331/logs/o20-tempo-ptr2/`。

- **O20 Tempo 検証完了。予測 3 つは全て成立で、class (b) 仮想ポインタの契約の欠落は本物でした**
  - Tempo: 開始 200.000 → 3 px drag 後 **522.000** → 以後の全部の drag で 522.000 のまま(E13 本番 wasm と同じ。再現で数値は一致)。
  - 予測 ① pointerY ≈ H−43: **成立(正確に一致)**。drag 1(y 21,22,23)の guest pointer は (450,20) → **1037, 1038, 1039** → (450,20)。H=1080 なので H−43=1037。回り込み幅は 1016 = H−64(= 縮めた矩形の高さ 1080−64)。
  - 予測 ③ pointerX 不変: **成立(x=450 のまま。全 trace で x の変化なし)**。
  - 予測 ② y≥32 で回り込まない: **成立(同じ Tempo コントロール内)**。drag 2(80 px 下)y=30(=32 未満)で 1 回だけ 1046=30+1016 へ回り込み、y=40,50,…,100 は host の値をそのまま追随。drag 3(y 10,5,0)は 1026,1021,1016(全て回り込み)。drag 4(19,18,19)は 1035,1034,1035(全て回り込み)。
  - 読解: FL のドラッグ中のポインタ位置とゲストの値がずれるのは、ゲストが SetPhysicalCursorPos で回り込ませた位置を、次の host move がブラウザの実位置で上書きするため。差 = (前回位置 − 現在位置)×scale が毎回ほぼ H 一定になり、Tempo が最大(522)に張り付く。alpha1 の仮想ポインタ差分が入ったら、実 Chrome でこのシナリオで再検証する。
  - PM が trace で確認: `eval-ptr-trace.json` の履歴に 1037/1038/1039、1046、1026/1021/1016、1035/1034/1035 の遷移があり、x は 450 のままです(Browser の実位置 (450,20) の記録も混在)。trace の first/last は (0,0)〜(691,651) で、drag 区間の値は履歴の内部にあります。
- **mutex(o1b)完了。再現した(2/2 → 3/3)。停止時の mutex の語: dlmalloc の mspace mutex を同じ thread が本当に保持していた(self-deadlock)。wild write ではない。**
  - 実 Chrome(私の 1 本、16:26〜16:30)、page-rt1 のコピー(fruity.ts の onAbort で停止時に memory から u32 を 3 語読む。mark 'mutexwords' 経由。製品・E13/E14 の page・wasm には触っていない)、RT-B(`fl-engine-ci-0a33cda7`、診断あり)。条件(開始時 16:26): MemAvailable 約 28 GB(16:11 時点)、load 0.29 → 1.58(終了時)、swap 入出力は pre-run-note で 0(詳細は `lift/out-gpu2/logs/memnote.txt`)。`scn-rtb-o1b-mutex.json`(o1b と同じ操作 + 最後に 1 個の evalPage)。証跡: `lift/out-git/gpu8-results/0331/logs/rtb-o1b-mutex/`。
  - 停止は display change(expand)直後に発生(engine は elaborate animate の repaint で `pthread_mutex_timedlock.c:95` の assertion で停止。abort stack は過去と一字一句同一(`wasm-function[134101] ← 134173 ← 134177 ← 134176 ← 133224 ← 132796 ← 133101 ← 133104`)。state=error、exit=stop。trace の n は measure で正常。
  - **st-collapsed は engine-idle のままで、collapse 自身は無事。expand で停止**(collapse→expand)。
  - mutex の語(3 語: type, lock, waiters): **mspace@0x10f7d4dc = `0x0, 0x2a, 0x0`** (type=0x0 NORMAL、lock=0x2a、waiters=0)、**malloc_global_mutex@0x10f7d50c = `0x0, 0x0, 0x0`** (保持していない)、停止した thread の **tid = 0x2a** (=`pthread_self` 0x10f7c9bc の +24)。
  - PM が `eval-mutex-dump.json` で確認: `mspace: 0x0,0x2a,0x0、global: 0x0,0x0,0x0、tid: 0x2a` です。
  - 読解(mt3 の分類): lock の語の下位 30 bit(`0x2a`)== tid(`0x2a`)、type==0(NORMAL)、waiters==0 → 本当にその thread が持っている(再入。握ったまま別の経路が入った)。wild write の候補ではない。= dlmalloc の mspace mutex を同じ thread が二重に入った(malloc 内での malloc / lock 残量への再入)。
  - これは musl の NORMAL mutex の自己保持の検知である。wild write の否定が得られたので、停止の診断は『malloc の enter/exit』方向(mspace の lock の再入)になる。global mutex は保持していない。誰が二重に入っているかは次段(mt3 + alpha1 のカバー)。
  - 番地は RT-B の link map から確認済み(`_gm_` = 0x10f7d31c。mspace = _gm_ + 448 = 0x10f7d4dc。`malloc_global_mutex` = 0x10f7d50c)。RT-D / RT-D2 では使っていない。
  - 同写し: mt3、alpha1 にも info する(詳細は 0331/logs/rtb-o1b-mutex/eval-mutex-dump.json)。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 16:41): opt1 の fhook6(Balance の Eff_Render hook)の結果。依頼者の機械は 0 件

**実際に実行した物**: 実 Wine 10 の FL Studio 2026 で 2 本実行した(opt1)。(a) 起動確認 1 本(16:35:05〜16:36:02、project なし、OPT1_NOPROJECT=1)。FL64 が 15s 後 SNl・45s 後 RNl を確認。# hook bal は project を開かない起動では Balance DLL 自体がロードされないため出ない。(b) 本番 fhook6(16:36:09〜16:38:16、Export 完了、5,729,968 フレーム、prefix-e2 config A、ins1-lp、FLP sha d6783e08886b4b48)。完了後は wineserver -k のみで終了した。PM が実ファイルで確認: `fhook6-balance.tsv` 182 行、`fhook6.wav` の sha 先頭 `46faf3d6a3017331`(22,920,056 B)、Wine プロセス 0。表は `docs/investigations/opt1/0320/tables/fhook6-balance.tsv` と `hookx-fhook6.txt`。

- **WAV の同一性**: fhook6(sha `46faf3d6a3017331`)は `elog16-ins1-lp.wav`(Wine 08:28)との差が 4 フレーム ±1 LSB(3,779,690・5,325,800・5,325,818・5,396,485)です。fhook4.wav(13:22 Wine)との差は 6 フレーム ±1 LSB(上記 4 + 3,839,893・5,416,656)です。どちらも最大誤差 1 LSB です。指示の「±1 LSB 数フレーム以内」は elog16-ins1-lp に対しては満たすが、fhook4 に対しては 6 フレームです。
- **Balance の Eff_Render の窓(expos 210,000〜212,500)**: hook 行は `# hook bal installed at 0000000079b6efc0 (Fruity Balance_x64.dll base 00000000798b0000)` です。この汎用表は `key=` と `len=` と `win=` を持っていないため、表が生成されたのは区間の 1 本のみ(seq 2a8f、expos 210578、pos 1229458)です。その区間の値: **入力 peak 0・sum 0、出力(入口 d)peak 0・sum 0、出力(出口 d)peak 0・sum 0**(peaks: b=d=0)。その前後の間引き行(expos 206168・208372)も入力 0 です。
- Balance の入力が非ゼロになるのは **expos 212782(seq 2a9f)**で、その際出力 peak = 入力 peak(0.37556)で、素通しになっています。
- つまり、区間 [210,723,211,680) の Balance の出力は 0 です。ただし **不足があります**: 「Balance が 0 を出す」直接の自己無音ではない(Balance の入力自体が frame 211,680 まで 0 のため、出力が 0 で素通しかをこの run だけでは割り切れない)。そのため、区間の Balance の「0 の理由」を言い切るには、Balance の Eff_Render の入力を同定するのに足りる `balance-effrender.tbl` を直し(key= と len=d(rcx+48) と win=210000-212500 を足す)、再実行して区間の全呼びを表に出す必要があります。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 17:15): opt1 の fhook7(Balance の Eff_Render 修正式)の結果。依頼者の機械は 0 件

**実際に実行した物**: 実 Wine 10 の FL Studio 2026 で 2 本実行した(opt1)。(a) 起動確認 1 本(17:07:58〜17:08:48、project なし、OPT1_NOPROJECT=1)。FL64 が 15s 後 SNl・45s 後 RNl を確認。# hook bal は project を開かない起動では Balance DLL 自体がロードされないため出ない。(b) 本番 fhook7(17:08:59〜17:10:59、Export 完了、5,729,968 フレーム、prefix-e2 config A、ins1-lp、FLP sha d6783e08886b4b48)。完了後は wineserver -k のみで終了した。PM が実ファイルで確認: `fhook7-balance.tsv` 1,935 行、`fhook7.wav` の sha 先頭 `ac60933d0c7963ef`(22,920,056 B)、Wine プロセス 0。表は `docs/investigations/opt1/0320/tables/fhook7-balance.tsv.gz`(213 KB のため gzip)。

- **WAV の同一性**: fhook7(sha `ac60933d0c7963ef`)は `elog16-ins1-lp.wav`(Wine 08:28)との差が 4 フレーム ±1 LSB(3,779,690・5,325,800・5,325,818・5,396,485)です。fhook4.wav(13:22 Wine)との差は 6 フレーム ±1 LSB(上記 4 + 3,839,893・5,416,656)です。指示の「±1 LSB 数フレーム以内」は elog16-ins1-lp に対しては満たすが、fhook4 に対しては 6 フレームです。
- **Balance の Eff_Render 修正式(key=rcx、len=r9、win=210000-212500)**: 窓 expos 210,000〜212,500 の区内呼び 18 本のうち、区間 [210026, 211542] の 12 本は入力 peak 0・出力 peak 0 です。**入力が非ゼロになるのは expos 211680(seq 2a67、pos 1225440)**で、その呼びの入力 peak は 0.566111、出力も peak 0.566111 で、素通し(同じ sample 値)です。その後も出力 peak は入力 peak と同じで、Balance は pass-through です。**区間 [210,723,211,680) の Balance の出力は 0**です。
- **Balance について**: Insert 1 の各 slot を個別に同定するのに足りる slot_idx は全件 2(slot 2)で、区間の Balance の「0 の理由」はこの run だけでは割り切れない(Balance の Eff_Render はこの区間では入力自体が 0 であり、この run だけでは Balance が 0 を「出した」ことが分からない)。そのため、区間の Balance の「0 の理由」を言い切るには、Balance の Eff_Render の入力を同定するのに足りる `balance-effrender.tbl` を直し、再実行して区間の全呼びを表に出す必要があります。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 17:16): ci1 の G2/G3/G6 の測定(RT-D3)。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実 Chrome 156(headless/CDP)で E13(16:38:33〜16:42:51)と RT-D3(16:45:37〜16:49:59)を実行した。scenario `tools/scn-lat-ff.json`。各 run は load/free/vmstat を記録し、swap I/O 0・起動 load ≤8 の run のみ counted。latDump JSON は `out/lat-chrome-prev-1/` と `out/lat-chrome-new-1/`。G6 は Firefox 158 で RT-D3 を起動した。表は `docs/investigations/ci1/0338/` に取り込み済み(PM が実データで確認: `g2-table.md` の数値は ci1 の報告と一致)。

- **G2(ユーザー可視 T。post-play window の値のみ。`detail` 全体値は混ぜていない)**: 
  - queue insertion(pointermove、Mixer drag): E13 median 5.14 ms(n=81)、RT-D3 5.13 ms(n=81、差 -0.01 ms)。この指標だけなら pass。
  - ユーザー可視 T(first frame - tPage): idle File click E13 6.29、D3 8.23、差 +1.95 ms(pass)。idle key down E13 1.91、D3 2.00、差 +0.09 ms(pass)。drag pointermove E13 2.66、D3 2.83、差 +0.17 ms(pass)。**playback File click E13 median 19.57、D3 47.93、差 +28.36 ms(fail、p90 24.87 vs 57.09)**。**playback key down E13 median 5.15、D3 19.67、差 +14.52 ms(fail、p90 13.70 vs 32.91)**。
  - **G2 overall: NOT PASS。** pointermove queue 自体は同等だが、再生中の button/key の T が +10 ms を大きく超える。
- **G3(present rate)**: O19 round1/round2 present/s は shown E13 15.2/16.4、RT-A 31.9/30.2、hidden E13 20.6/17.1、RT-A 39.8/38.8、allhidden E13 20.5/19.1、RT-A 45.7/43.2。C full-song mean: shown E13 14.4 / RT-A 27.8、allhidden E13 19.3 / RT-A 41.4。D3 shown 60s: 31.58/s。E13 shown 14.4〜16.4/s を下回らない。**G3 pass**(協調より低下なし。D3 は shown のみ測定)。
- **G6(Firefox 158)**: 起動 ok(RT-D3 engine-idle、first idle 4,621 ms in report)。About は Settings -> About タブ表示は確認。ただし layered logo は 3x3 のままで、staff roll の About 本文は見えない(seed 資材が無い)。ja-names FLP: Channel rack 1行目と Playlist header が tofu box として描かれる。log に msgothic.ttc の seed が無く(arial/tahoma/lucon のみ)、日本語フォントは未解決と記録。Playback は進んだ(audio-p3 playingSeconds 77.4、nonSilentQuanta 5512、underrun 0)。**G6 は partial：起動/About tab/project/playback は動作するが、About seed assets と日本語フォントは未解決。full pass とはしない。**
- **PM の判断**: G2 の playback の fail(+28.36 ms / +14.52 ms)は GIL 保持の長時間 API による T の増加と読める(未確認)。S2(0312c 時間計測)の縮小対象は再生中の button/key の T である。G3 の present は協調より落ちない。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 17:23): RT-E での O20 再検証(仮想ポインタ修正が効いた)。依頼者の機械は 0 件

**実際に実行した物**: 実 Chrome(私の 1 本、Chrome 156 headless)、page-rt1 のコピー(`/home/ubuntu/work/gpu8/page-rt1-o20/app`: worker.ts が毎 engine idle で `mark('pointer', {x, y})`、main.ts が履歴 `pointerTrace` を残す。製品/E13/E14 の page・wasm には触れていない)、RT-E(`fl-engine-test-rte-87af8f55`。assets は `mk-assets-rtb.sh` で manifest をコピー(engine.env = 通常の env + `FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1`。mt3 の mk-rte-assets.sh と同じ)。条件(開始時 17:08〜17:17): MemAvailable 24.0→28.1 GB、load 3.58→0.13、swap si/so 0/0。ci1 の Firefox G6 が終わった後に開始した(17:08 に ps で Firefox の不在を確認)。FORCE=1 なし。`scn-o20-tempo-ptr.json`。画像・数値: `lift/out-gpu2/runs/rt-o20-tempo-rte/` と `rt-o20-rclick-rte/`(PM が copies)。証跡は docs/investigations/gpu8/0331/logs/o20-tempo-rte/ と o20-rclick-rte/。

- **Tempo(予測 3 つ)** — class は (b) 外部 API の契約の実装:
  - **522 に張り付かない**。開始 200.000 → 3 px drag 後 **200.000**(RT-B は 522.000)。80 px 下の drag で **180.000**(-20。RT-B は 522.000)。上への 20 px drag・小さい drag は ±0.75 以下の変化で表示上無変化(表示は 180.000 のまま)。= **値がドラッグを追随し、方向も正しい**。
  - 予測 ①(pointerY ≈ H−43): drag 1 で guest pointer は (450,20)→**1037, 1038, 1039**→(450,20)。**H−43=1037 と正確に一致**。回り込み自体は RT-B と同じ(FL のドラッグ機構が物理カーソルを反対側へ回らせる)。**違うのは、その後の差**: 仮想ポインタの offset により次の move が回り込んだ位置からの相対になり、(prev − cur)×scale が小さな値になる。**値の飛躍が消えた**。
  - 予測 ③(pointerX 不変): **x=450 のまま全期間**。
  - 予測 ②(y≥32 は回り込まない): drag 2 の y=30(=32 未満)で 1 回だけ 1046(=30+1016)へ回り込み、**y=40,50,…,100 では host の値をそのまま追随**(37,47,…,97。+7 の垂直バイアスあり。値の変化は +80px→−20 で正しい)。
  - PM が trace で確認: `eval-ptr-trace.json` の履歴に回り込みの遷移があり、x は 450 のままです(Browser の実位置 (450,20) の記録も混在)。
- **目撃した挙動(参考。今回の再検証の要件の外)**: 2 回目の drag の後に guest pointer は (450,17) へ戻り(FL の drag end の end() が drag 開始点へ復帰)、次の host move(450,20)で guest は 0 へ飛び、その後の drag 3・4 の host には応答しなかった(Tempo は 180 のまま)。= 「top edge を含むその後の drag での再アンカー」は Windows っぽい挙動だが、RT-B では drag 3/4 も動いていたので、**RT-E の仮想ポインタで drag 3/4 が効かない**のは要確認の余地がある。PM の判断対象として記録する(検証条件の外)。
- **右クリック(WM_CONTEXTMENU 差分の検証)** — class (b):
  - **RT-E でも出ない**。6 要素(Tempo (450,20) / Song Position (485,20) / Pattern Selector (615,61) / Channel rack ボタン (520,144) / Playlist 空き (1500,900) / Mixer insert 名 (505,690))の全てで、右クリック後に `TQuickPopupMenuWindow` は現れない(右クリック後の pics の窓一覧は主窓 1 枚だけ)。対照は左クリックの File メニューで **TQuickPopupMenuWindow @4,36 208x333 が出る**(RT-B と同一)。
  - ページの記録(lat-final.json): 6 組の右ボタン down/up は全てエンジンへ送られ、guest の screen 座標も正しい。= 入力自体は届いている。
  - **WM_CONTEXTMENU の差分は開始するのに効果なし**。これは docs/0255-message-audit.md(alpha6)の native A/B の結論と同じ: WM_CONTEXTMENU を作っても CreatePopupMenu / TrackPopupMenu は呼ばれず、窓表・画素は一致。『FL は実機で(Win 上で)WM_CONTEXTMENU からメニューを開かないと推定する』という意味では整合するが、**右クリックでメニューが出るか否かは実機未確認**。右クリックでメニューを開く機序はこの差分だけでは説明が付かない。RT-B(`out-git/gpu8-results/0330b/logs/o20-rclick-e13/`)と比べた結論の変化は無い(全要素 0)。
  - 未確認: FL が実機 Windows で右クリックのメニューを開くか、開く別経路が何か。今回の検証対象(WM_CONTEXTMENU 差分の効果)の判断: **効果なし**。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 17:57): opt1 の fhook8(f_cf2a90 の master-sum hook)の結果。依頼者の機械は 0 件

**実際に実行した物**: 実 Wine 10 の FL Studio 2026 で 2 本実行した(opt1)。起動確認 1 本(17:53:27〜17:54:18、project なし)。# hook msum installed at 000000007b022a90 が shim の log に出た。FL64 が 15s 後 SNl・45s 後 RNl を確認。c0000005 なし。本番 fhook8 1 本(17:54:30〜17:56:42、Export 完了)。prefix-e2 config A、ins1-lp、FLP sha d6783e08886b4b48。ALLOW_CHROME=1 で gpu1 の非計時 Chrome と並行。完了後は wineserver -k のみで終了した。PM が実ファイルで確認: `fhook8-slots.tsv` 5,365 行(547 KB)、`fhook8.wav` の sha 先頭 `57ec045169a9895c`(22,920,056 B)、Wine プロセス 0。表は `docs/investigations/opt1/0320/tables/fhook8-slots.tsv.gz` と `hookx-fhook8-again.txt`。

- **WAV の同一性**: fhook8(sha `57ec045169a9895c`)は `elog16-ins1-lp.wav`(08:28 Wine)との差が 2 フレーム ±1 LSB(5,396,485・5,416,656)です。fhook4.wav(13:22 Wine)との差は 2 フレーム ±1 LSB(3,839,893・5,396,485)です。指定の「±1 LSB 数フレーム以内」は双方に対して満たします。
- **master sum の hook はこの run では取れていない**: f_cf2a90 の入口が出たのは 3 個の slot のみ(Insert 1 と Insert 47)で、Insert 0 の slot は出てこなかった。したがって master の insert(Insert 0)そのものの slot は同定できていない。master sum そのものについても fhook8 では取れていない。表の窓 expos 210,000〜212,500 の区間の呼びは 3 個の slot 各 18 呼びで、Insert 1 / Insert 47 以外の slot は出ていない。
- **判定**: master sum 区間の rms 1773.5 の出所は、f_cf2a90 の入口では見えない。次は別の hook 点(Insert 1 の出力と master の和を作る FLEngine の関数の VA)。RT-D4 の wasm 診断(diag-g9mix)の `f_cf2a90` の入口/出口の観測(Insert 1 の slot 0(Balance)の出力、slot 1(Filter)の出力、dry、Insert 1 の出力 buffer、master の入力の和の peak)を併せて使う。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 17:58): O19 の RT-E の C(曲全体 10 秒窓、counted、n=1)。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実エンジン(E13 協調版と RT-E `fl-engine-ci-87af8f55`)を実 Chrome 156 headless で実行した。順序: E13 shown 17:32:32–17:38:32、RT-E shown 17:38:40–17:44:42、E13 allhidden 17:45:05–17:51:15、RT-E allhidden 17:51:24–17:57:37。scenario `tools/scn-o19c.json`、song 全トラック(all tracks)、再生から 300 s、10 s 刻み窓。各 run 開始前に load/free/vmstat を記録(swap in/out 0/0、MemAvailable 20–26 GB、他 Chrome 0、Wine 0、cargo/wasm-opt 0)。保存先は `lift/out-git/ci1-results/0339/`。表は `docs/investigations/ci1/0339/o19c-table.md` と `o19c-rte-table.md`(PM が実ファイルで確認: 行列は前回と異なり、70–80 の窓だけ RT-E allhidden が 6、他は全部 0)。

| run | windows underrun>0 / windows | underrun quanta total | lag cuts total | engine gap frames total | present/s(mean) | A silent runs ≥100 ms / longest | Worker CPU s(cores) / N |
|---|---|---|---|---|---|---|---|
| E13 shown | 0/30 | 0 | 1,611 | 0 | 22.2 | 2 / 75 posts | 0.80 / 1 |
| RT-E shown | 0/30 | 0 | 0 | 0 | 36.2 | 2 / 74 | 1.76 / 8 |
| E13 allhidden | 0/30 | 0 | 0 | 0 | 20.1 | 2 / 74 | 0.60 / 1 |
| RT-E allhidden | 1/30 | 6(70–80 s 窓) | 0 | 682 | 46.8 | 2 / 74 | 1.55 / 8 |

- **前回の E13 shown 413 quanta(230–240:4, 240–250:139, 250–260:174, 260–270:96)は再現せず、今回の E13 shown は 0 でした。** RT-E shown も 0 で、lag cuts は 0 です。
- 唯一の underrun>0 は RT-E allhidden の 70–80 s の窓の 6 quanta(gap 682 frames)で、他 3 条件は 0 です。
- A の無音連続 ≥100 ms は今回も 4 run 全部で 2 回(最長 74–75 posts)で、前回 C と同じ構造が残っています。
- present/s は RT-E が協調より低くなく、むしろ高いです(shown 36.2 vs 22.2、allhidden 46.8 vs 20.1)。
- **PM の読み(n=1。結論にしない)**: E13 shown の underrun(前回 C の 413)は再現しませんでした。RT-E shown は 0 で、RT-E allhidden だけが 70–80 s の窓に 6 quanta を出しました。依頼者の「Mixer 表示で GUI がオーディオを圧迫」という観察との関係は、n=1 なので結論にしません。E13 shown の lag cuts は今回も 1,611 と他 run と明確に違う(他は 0)ので、協調版の lag cut の傾向は残っています。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 18:58): ci1 の RT-F の G2/G6 再測定。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実 Chrome 156(headless/CDP)で E13(協調版 18:34:23–18:38:26)と RT-F(`fl-engine-ci-d45afb0e` 18:39:35–18:43:40)を実行した。scenario `tools/scn-lat-ff.json`。各 run は load/free/vmstat を記録し、swap I/O 0・起動 load ≤8 の run のみ counted。latDump JSON は `out/lat-chrome-prev-1/` と `out/lat-chrome-new-1/`。G6 は Firefox 158 で RT-F を起動した(About `out/g6-firefox-rtf/` 18:44:07–18:48:43、first idle 4075 ms; ja-names FLP `out/g6-firefox-rtf-ja2/` 18:53:15–18:56:52、first idle 4050 ms、ok/no stop)。表は `docs/investigations/ci1/0340/` に取り込み済み(PM が実ファイルで確認: `g2-table.md` の数値は ci1 の報告と一致)。

- **G2(ユーザー可視 T。post-play window の値のみ)**:
  - pointermove queue insertion(tInject-tEvent、kind=1、Mixer drag): E13 median 5.44 ms、RT-F 5.46 ms、diff +0.02 ms → pass
  - ユーザー可視 T(first frame - tPage): idle File click E13 5.47、RT-F 7.17、diff +1.70 ms(pass)。idle key down E13 1.65、RT-F 1.80、diff +0.15 ms(pass)。drag pointermove answered T E13 2.55、RT-F 2.27、diff -0.29 ms(pass)。
  - **playback File click: E13 16.93、RT-F 40.50、diff +23.58 ms(fail、p90 25.05 vs 50.95)**。
  - **playback key down: E13 3.97、RT-F 17.95、diff +13.98 ms(fail、p90 10.44 vs 27.07)**。
  - **G2 overall: NOT PASS。** RT-D3 の playback +28.36 / +14.52 から若干改善したが、依然 +10 ms を超える。queue insertion 自体は同等(+0.02 ms)なので、劣化は engine 側の処理・frame 提供の待ちにあるとみられる。
- **G6(Firefox 158)**: 起動 ok(RT-F engine-idle、first idle 約 4.1 s、opErrors なし)。About は Settings → About タブ表示は確認。ただし layered logo は 3x3 のままで、staff-roll の About 本文は見えない(BigFruit/AboutTxt の seed 資材欠落は未解決、RT-D3/RT-E から不変)。ja-names FLP: Channel rack 1行目が tofu box で描かれる。log に msgothic.ttc の seed が無く(arial/tahoma/lucon のみ)、日本語フォントは未解決(RT-D3/RT-E から不変)。Playback は進んだ(audio-p3 playingSeconds 76.9、nonSilentQuanta 5508、underrun 0)。**G6 は partial**。起動/About tab/project/playback は動作するが、About seed assets と日本語フォントは未解決。full pass とはしない。
- **PM の判断**: G2 の playback の fail(+23.58 ms / +13.98 ms)は engine 側の処理・frame 提供の待ちによる。GIL 保持の長時間 API が有力候補だが、未確認。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-07 05:03): ci1 の RT-F 診断 Export(g9mix)と GIL song、RT-F G2 再測定。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実 Chrome 156 headless で実行した。(1) RT-F 診断 Export(ins1-lp, g9mix): RT-F bundle(`fl-engine-ci-d45afb0e`)の manifest copy に `FLE_DIAG_G9F=1` のみを足した(`assets-rtf-g9f`、port 5130)。run 19:00:56–19:05:34(load 0.52、I/O avg10 0.00、swap 0/0、MemAvail 22866 MB、cargo/wasm-opt 0)。WAV 22,919,504 bytes、sha256 `df212eb06b6b9fa53b9ff8b7f608d397d51f9c1a7425d84fc2ea191803b4cf22`。engine-log は `lift/out-git/ci1-results/0341/rtf-ins1lp/engine-log.log`(50,000 行)。(2) GIL song(300 s, `FLE_DIAG_ATOM=1` + `FLE_DIAG_GIL=1`): `assets-rtf-gil2`(port 5141)で 19:30:47–19:36:49(load 0.30、I/O avg10 0.00、swap 0/0、MemAvail 20544 MB)。比較 run(`assets-rtf-gil`、FLE_DIAG_GIL=1 のみ)19:21:31–19:27:34。(3) RT-F G2 再測定: E13 19:42:04–19:46:05(load 0.08→3.41、swap 0/0、MemAvail 17481 MB)と RT-F 19:46:11–19:50:17(load 3.34→6.33、swap 0/0、MemAvail 27236 MB)。latDump は `out/lat-chrome-prev-1/` と `out/lat-chrome-new-1/`。表は `docs/investigations/ci1/0341/` と `docs/investigations/ci1/0342/` に取り込み済み(PM が実ファイルで確認)。

- **診断 Export(g9mix)の結果**:
  - engine-log は 50,000 行、g9m 行は 40,416(20,000 行の上限超過、cap HIT)、g9f 行は 9,582。
  - g9m entries は 10,104(4 slots × 2,526 calls)。Last scum/expos(max): scum 6,633,478 / expos 5,781,510。
  - **expos window 210,000–212,500: 0 calls**(最初の expos は 215,539、scum は 1,067,507 から)。診断の窓が Export 開始の定義と不一致でした。
  - Insert 1 slot 0 Balance(k=2, slot 0x17679b40): 2,526 calls、nonzero-in 2,351。max in peak 2.34501、max outpost peak 2.34501。first dst8 は src8 と同一(dry passthrough)。
  - Insert 1 slot 1 Filter(k=3, slot 0x17679cc0): 2,526 calls、nonzero-in 2,332。max in peak 2.34501、max outpost peak 1.06217。first out_a8 は `[0.0178409, 0.0282996, …]`(Filter が強く減衰)。
  - その他: k=0(insert=2)は 2,526 calls 全て peak 0(silence 176400→176400)、k=1(insert=47)も 2,526 calls 全て peak 0。
  - **PM の読み**: expos の定義は g9mix 側で修正が必要です(Export 開始の 24 連続 137/138 の呼びの先頭 scum を expos 0 とする形。Wine の fhook4 の Export 開始 scum 1,007,104 と整合する形)。
- **GIL song の結果**:
  - Atom 診断:`fl-diag: atom` 400 行(GlobalAddAtomW / GlobalFindAtomW)。
  - **GIL diagnostics:`fl-diag: gil` 0 行**。954 lines の engine-log に GIL wait/histogram は出ませんでした(「gil」を含む行は g9-diag indirect calls のみ)。
  - **GIL histogram が出ないため、playback File T +23.58 ms / key T +13.98 ms の原因を GIL の待ちから切り分けることは今回の GIL だけではできません**。
  - **PM が実ファイルで確認した原因**: `fle_gil_obs_on` が観測器の内部(`obs_init`)からしか立たず、ゲートが観測器を呼び出すために、永遠に立たない。mt3 の修正差分(`0315-gil-diag-init-fix.diff` 59 行)を docs に取り込み済み(PM が実在確認)。GIL のロックの語・待ち方は一切変えない観測のみです。native テスト 43 binary 648 passed / 0 failed(報告済み、PM 未確認)。
  - RT-D3/RT-E 比較: O19C window は両方とも underrun 0 / lag 0 / gap 0、present/s 36.0 と 36.7、silent runs 2、worker 1.73 cores、N=8。
- **RT-F G2 再測定の結果(post-play window の値のみ)**:
  - pointermove queue insertion(tInject-tEvent、kind=1、Mixer drag): E13 median 5.39 ms、RT-F 5.46 ms、diff +0.06 ms → pass(queue 投入自体は同等)。
  - ユーザー可視 T(first answered frame − `tPage`): idle File click E13 5.50、RT-F 7.20、diff +1.70 ms(pass)。idle key down E13 1.61、RT-F 1.83、diff +0.22 ms(pass)。drag pointermove answered T E13 2.21、RT-F 2.41、diff +0.21 ms(pass)。
  - **playback File click: E13 16.46、RT-F 39.83、diff +23.37 ms(fail、p90 23.12 vs 49.97)**。
  - **playback key down: E13 3.85、RT-F 14.55、diff +10.71 ms(fail、p90 12.10 vs 25.81)**。
  - **G2 overall: NOT PASS。** RT-D3(+28.36 / +14.52)から若干改善したが、まだ +10 ms を超えている。劣化は queue 投入ではなく engine 側の処理・frame 提供の待ちに寄っている。
- **PM の判断**: G2 の playback の fail(+23.37 ms / +10.71 ms)は engine 側の処理・frame 提供の待ちによる。GIL 保持の長時間 API が有力候補だが、FLE_DIAG_GIL が出ないため未確認。diag-g9mix の expos window 210,000–212,500 に 0 calls は expos 定義の修正が必要です(Export 開始の 24 連続 137/138 の呼びの先頭 scum を expos 0 とする形)。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-08 06:22): 再生中 T の内訳(ci1 0343)と、G2 の比較条件の欠陥(PM の発見、未検証の仮説を含む)。依頼者の機械は 0 件

**実際に実行した物**: ci1 が実 Chrome 156 headless(CDP Tracing + v8 cpu_profiler)で E13 と RT-F(`fl-engine-ci-d45afb0e`)を各 2 本、非計時の診断 run(swap 0/0、Wine 0、他 Chrome 0)。再生中の File メニュー開閉 12 回の前後を採取。`docs/investigations/ci1/0343/` に取込(生 trace 86〜113 MB ×4 は除外、`lift/out-ci1/kit/out/tp-{1,2}-{e13,rtf}/` に残る)。

- 再生中 File click の中央値(run1/run2): A tEvent→tPage E13 0.65/0.66、RT-F 0.67/0.76。B tPage→tInject E13 0.35/1.14、RT-F 0.90/2.85。**C tInject→最初の frame(UI Worker の処理)E13 18.96/18.52、RT-F 46.87/44.39(+27.9/+25.9)**。D 最初の frame→rAF E13 3.44/2.00、RT-F 7.05/10.85。増分の大半は C。
- UI Worker は idle でなく処理している(idle E13 0 / RT-F 0.2〜0.3 ms/入力)。phase C の 73〜75% は `g9_hook` の下(包含 34.90/31.82 ms/入力)。`g9_hook` の self は 3.93 ms/入力。GIL 関数(`fle_gil_wake`+`fle_gil_acquire_slow`)の self は合計約 1 ms/入力で、GIL 待ちを主因とする証拠は出ていない(GIL の待ち・保持の分布は `FLE_DIAG_GIL` が 0 行のため未取得)。他の pthread Worker 7 本は多くが CPU を使わず待っている(CPU/wall 0.19〜0.27)。
- **PM の発見(実測)**: E13 本番 bundle(`fl-engine-ci-0363b050`)の wasm は `g9-diag` 文字列 0 件。RT-A(`fl-engine-test-rt1-91bf78f6`)・RT-E(`87af8f55`)・RT-G(`b083dc31`)は 5 件。tree-ci(E13 の木)の `fle_callback.c` に `g9_gui_init` は 0 件、tree-rt1/rtb/rtd/rtd2/rtd3 には 1 件(`__attribute__((constructor)) static void g9_gui_init(void) { fle_dispatch_hook = g9_hook; }`、`fle_callback.c:335`)。したがって RT 系は診断 OFF でも全ゲスト間接呼びが census hook(決定 0097)を通る。**G2 の比較(E13 = フックなし vs RT-F = フックあり)は同条件でなく、G2 の +23 ms の一部あるいは大半がフックの負荷かもしれない(未検証)。** lift-fidelity 方針の (a)「我々の余計な処理」。是正: mt3 に RT-H(診断スイッチ ON のときだけ `fle_dispatch_hook = g9_hook`)を指示、ci1 に RT-A と E13 の G2 を指示。
- **botid の config.json**: 06:00 頃、shell が `botid: cannot read ~/.config/botid/config.json: invalid character ']'` で失敗(`rules` 配列末尾の余分なカンマ)。PM が 06:03 に末尾カンマ 1 文字だけ削除(元の内容は `/tmp/opencode/botid-config.json.orig-20261108-0600`)。ci1 は「06:08:13 に誰かが修正し 06:14 に JSON として読めた」と報告しており、PM の修正時刻と食い違う。誰が・いつ・何度壊したかは未調査。
- **F5(gpu1、実 Chrome 156 と Firefox 158、page-rt1 のコピー+keymap 変更)**: 差分 `docs/investigations/gpu8/diffs/0335-f5-forward.diff`(9,331 B)。F5 4 回(F5×2、Ctrl+F5、Shift+F5)で印が同一=reload なし、`defaultPrevented=true`。変更前 HEAD の page は F5 1 回で reload され読み込んだ project が消える。FL には届く(inputEvents +2 ずつ、Playlist が切り替わる)。unit 変更後 1315 passed / 0 failed(変更前 1313)。未確定: 1 回目の F5 直後の描画遅れと、製品の page(協調 page + E13)での確認(gpu1 に指示済み)。main への push は未実施。
- alpha1 の g9mix 修正差分(expos を Length 137/138 の 24 連続の最初の呼びで決める): `docs/investigations/alpha22/impl/diag-g9mix-expos-fix.diff`(1,452 B)。PM が RT-F の `fle_diag_g9mix.c` に実適用して rc=0・結果 7,760 B・alpha1 の木と cmp 一致を確認。RT-H に取り込む。
- 依頼者の機械は 0 件。


## PM の追記(2026-11-08 07:32): 0344(G2 の比較条件の分離の試み)と取り込みの記録。依頼者の機械は 0 件

**実際に実行した物(ci1、実 Chrome 156 headless、counted)**: `scn-lat-ff.json`、E13(`fl-engine-ci-0363b050`)と RT-A(`fl-engine-test-rt1-91bf78f6`、診断なし・実スレッド・フックあり)を 2 組(組 a・組 b)。組 b を基準。全 run swap 0/0、他 Chrome 0。ただし組 a の E13 の最後の約 26 s に opt1 の Wine が重なった(影響は約 +2 ms、参考扱い)。

- 再生中 File click(中央値): 組 b E13 21.35 → RT-A 45.12(+23.77 ms)。組 a は +26.24 ms。
- 再生中 key down: 組 b 5.82 → 12.91(+7.09 ms)。組 a は +4.98 ms。
- idle File click: +1.98 ms(組 b)。idle key down: +0.34 ms(組 b)。
- **RT-A(フックあり・実スレッド)でも再生中 File click は E13 比 +24〜+26 ms**。RT-F(0342、フックあり)の +23.37 ms と同程度。ゆえに「+23 ms は RT-F 固有」ではなく、フック有無を含む RT 系の共通の差。
- **フック由来か実スレッド由来かは未分離**(RT-A も RT-F もフックを持つ)。分離には診断なし・フックなしの RT-H plain が必要(ci1 に指示済み、保存先 `lift/out-git/ci1-results/0345/`)。
- RT-A の run 1 本(lat-chrome-new-5)が SIGABRT(`DeleteTimerQueueTimer: CompletionEvent is not NULL while the callback is running: waiting for a callback is not modelled`)で停止。counted から除外。原因は未調査。契約不足を指す文面だが、因果は未確認。
- RT-G の GIL 診断(非計時、counted ではない、`FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1` の manifest コピー):累積値で、全 Worker 取得 16,829,040 回のうち 0.1 ms 以上の待ちは 79,555 回(0.473%)。W0 の最長の待ちは 3.74 ms。**File click の phase C の窓だけの GIL 値は未取得**(診断に時刻が無いため)。したがって「+23 ms は GIL 待ちが主因」は、現時点では支持されない(未確定)。

**docs へ取り込んだもの**: `docs/investigations/ci1/0344/`(REPORT-1〜3、解析スクリプト、run 記録の .md・.json。生の trace と 400 KB 超の engine-log は除外)、`docs/investigations/alpha22/tables/fhook9-expos-wav-0319.tsv`、`docs/investigations/alpha22/tools/a22-expos-wav.py`、`docs/decisions/0319-owner-observations-contract-reading.md` 末尾に alpha1 の §26(Insert/右クリック/fhook9)と §27(O20 delta)を追記(既存の PM 節は変更していない)。

**未了(次に確かめる)**:
- opt1 の fhook10(右クリック経路の ra 観測、CreatePopupMenu 0x4248f0 / TrackPopupMenu 0x425620)は未実行。開始条件(counted の Chrome 0、Wine 0、swap si/so 0)を待機中。
- RT-I(gpu1 の keyfocus 診断)の build は mt3 が実行中。実 Chrome の宛先窓の観測は RT-I の bundle 後。
- O20 の delta 方式は native テストのみ。実 Chrome 未確認。
- 右クリックは未修正。経路は未確定(静的の読みでは `f_81d040` 経由と `f_845950` 経由がある)。

**botid の config.json**: 06:00 頃、shell が `~/.config/botid/config.json` の `rules` 末尾カンマで失敗。PM が 06:03 に末尾カンマ 1 文字だけ削除(元の内容は `/tmp/opencode/botid-config.json.orig-20261108-0600`)。誰がいつ壊したかは未調査。
