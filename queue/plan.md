# 計画: 完成までに必ず達成する 6 つの課題(台帳 L-033)

監査・受付が作る。survey の結果が読めたら、課題と受け入れの条件を直す。

## ① 目的と完成の条件

- 目的: 依頼者の指示 L-033 の 6 つ(C-1〜C-6)を、プロダクトの完成までに全て達成する。今すぐ着手する必要はない(依頼者の指示)。
- 完成の条件: C-1〜C-6 の全てが「受理」されること。受理は、実測の出力つきで、下の受け入れの条件(案)を満たすこと。
- 判定の語は「受理」「差し戻し」「未確認」のいずれか。作業者の報告に「達成」「直った」とは書かない。
- 目標(AGENTS.md): P1 起動 10 秒以内、P2 UI 120 fps、P3 Song Mode で音が途切れない。ぶつかるときは P3、P2、P1、サイズの順。
- 共通の制約: 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(台帳 L-002)。Python は調査と検証だけ(L-008)。製品の実測は Actions の作業ジョブで行う(L-009)。
- 受け入れの条件は、この版では「案」。survey の結果で直す。

## ② 課題(C-1〜C-6)

依頼者の言葉の要点、関係する目標、コードから分かる現状(パスと行番号つき。推定は推定と書く)、実測、前提、受け入れの条件(案)の順に書く。

| 課題 | 依頼者の言葉の要点 | 関係する目標 | 実測 |
|---|---|---|---|
| C-1 | HiDPI に対応しておらず、画面がぼやける | P2 | survey-c1-hidpi |
| C-2 | 画面の描画が音声処理を圧迫する。Worker 分離は完了していないのか | P3(最優先)、P2 | survey-c2-worker-split |
| C-3 | 言語パック・テーマ・プロジェクトテンプレートなど、FL Studio 自体が必要とする物が揃っていない | P1、機能 | survey-c3-fl-resources |
| C-4 | 内蔵のエフェクトとインストゥルメントが 1 つも無い(目標は約 141。L-034) | P1、P3 | survey-c4-builtin-plugins |
| C-5 | Serum 2・OTT など、多数の外部プラグインが無い | P3 | survey-c5-external-plugins |
| C-6 | Tempo・Song Position・Pattern Selector などトップバーのコンテキストメニューが機能しない | P2 | survey-c6-topbar-menus |

### C-1 HiDPI(画面がぼやける)

- 現状(コードから):
  - デスクトップの canvas の描画バッファは、guest 画面の画素数である(app/src/ui/desktop.ts:24〜25)。CSS の大きさは、guest 画面の大きさに拡大率を掛けた値である(同 47〜50 行)。
  - guest 画面は、部屋(stage)の CSS px の大きさに合わせる(app/src/main.ts:260〜276)。注釈は「guest 1 pixel = CSS 1 pixel。ブラウザが canvas を devicePixelRatio で拡大する」(app/src/ui/screen-fit.ts:3〜4。drawScale は 57〜60 行)。
  - devicePixelRatio は表示と記録にだけ使い、描画の大きさには掛けていない(app/src/main.ts:265・275・349。app/shared/perf.ts:331 は記録のみ)。
  - 推定: 拡大率 2 の画面では、canvas がブラウザで 2 倍に引き伸ばされ、ぼやける。
- 実測が要ること: 拡大率 2 の headless Chrome で、canvas の幅 ÷(CSS の幅 × 拡大率)を測る(survey-c1-hidpi)。
- 前提: 画素数を増やすと、描画の所要が増える(推定)。C-2 の後に行う。
- 受け入れの条件(案): 拡大率 2 と 1.5 で、canvas の幅と高さが CSS の大きさ × 拡大率(±1 画素)に一致する。C-2 の受理の後に、P3 の underrun が 0 であることを確かめる。

### C-2 描画が音声を圧迫しない(Worker 分離の完了)

- 現状(コードから。答え: 分離は途中である):
  - ページ(メインスレッド)、エンジンの Worker(fl-engine)、音声の出力(AudioWorklet)は分かれている(app/src/engine/client.ts:75、app/src/audio/page-audio.ts:159)。
  - しかし、エンジン(wasm)、デスクトップの合成と提示(app/src/engine/worker.ts:381 の deskTick)、音声の書き込み(lift/runtime/fle_audio_transport.c:54)、音声の実時間の待ち(同 108 行)は、同じ fl-engine の Worker のイベントループで動く。
  - ゲストの複数のスレッドも、1 つのホストのスレッドで協調して動く(lift/runtime/fle_thread.h:1〜6)。
- 実測が要ること: スレッドの配置(CDP の Target の数と Worker の URL)と、描画の所要(window.__app.deskStats の composeMs・putMs)(survey-c2-worker-split)。P3 の underrun は、所定のプロジェクトが runner に無いので、未確認になる。
- 前提: P3 の所定のプロジェクトは依頼者の個人のファイルで、runner に無い。入力が無いと作業者は報告して止まる。
- 受け入れの条件(案): (a) 描画の合成と提示が、音声の書き込みと別の Worker で動く(実測: Target の数と、Worker の URL)。(b) P3 の実測(所定のプロジェクト、Song Mode、n=3)で underrun が 0。

### C-3 FL 自体が必要とする物(言語パック・テーマ・プロジェクトテンプレート)

- 現状(コードから):
  - seed の表(lift/harness/seeds/vfs-seed-idle1.tsv、247 行)には、言語パックが en.svg の 1 件(239 行)、テーマが Artwork\Skins\Default の 3 件(24〜26 行)、テンプレートは Data\Templates\ のディレクトリだけ(18 行。ファイルは 0 件)、既定のプロジェクトが Data\System\Empty.flp の 1 件(161 行)、デモが NewStuff.flp の 1 件(30 行)。
  - 抽出は app/scripts/extract-fl.mjs。作業ジョブが持つ抽出の木は、DLL の表と seed の行の物だけ(.github/workflows/agent.yml の extract ジョブ)。
- 未確認: インストーラの全件の一覧。runner に無い(R2 の鍵は extract ジョブにだけある)。必要な全件の数。
- 実測が要ること: seed の行と抽出の木の突き合わせ、起動時に見つからないパスの記録(survey-c3-fl-resources)。
- 前提: 共通の前提の inventory(インストーラの一覧)。作る経路と実現可能性は、下の「共通の前提」の節。
- 配信の方針の分類: 起動前に取る物(配信の方針 L-035)。今の bundle では seed pack に入る(app/scripts/pack-seed.mjs の 1〜7 行: 「the guest runs の前にハーネスが読むホストファイル」を 1 つにまとめる。seed pack は 20780949 B。run 44673024703 の判定の出力)。言語・テーマ・テンプレートが、そのうち何バイトかは、survey-c3 で測る(推定: seed の行の中に在る)。
- 受け入れの条件(案): 全件の一覧と抽出の木の差が 0。起動時の「見つからない」が 0。言語・テーマ・テンプレートの件数が一覧と一致する。起動前の取得に含まれ、P1 の 10 秒を超えない。

### C-4 内蔵のエフェクトとインストゥルメント(目標 約 141)

- 依頼者の言葉: 「100 以上の内蔵エフェクトやインストゥルメントは 1 つも存在しない」(L-033)。是正(L-034): 内蔵のエフェクトは `Plugins\Fruity\Effects` に約 90 件、インストゥルメントは `Plugins\Fruity\Generators` に約 51 件ある。依頼者の手元の版の数で、lift 対象の版とは ±2 ほど違いうる。
- 目標: 約 141(効果 約 90 + インストゥルメント 約 51)。OpenGL を使う VFX 系は後回しにしてよい。それ以外の内蔵プラグインは、全て lift の対象。
- 進み具合(目標ではない): lift の対象になっている内蔵は 3 件(Fruity Balance・Fruity Filter・Fruity Delay 2。lift/harness/b2d2/modules.tsv の 8〜10 行)。進み具合は「3 / 約 141」。分母は inventory で確かめる(下の共通の前提)。
- 現状(コードから):
  - lift の対象の表は 10 行(lift/harness/b2d2/modules.tsv)。残りはエンジンのライブラリ(dsp・Blend2D・QuickFontCache・FreeType・ILTools・oggio・elastique)。hyb の対象は 8 行(lift/harness/hyb/hyb-args.tsv)で、Fruity を含まない。
  - インストゥルメントの DLL は 0 件(ci/extract-dlls.tsv に無い。seed の Plugins\Fruity\Generators は GM CCs.txt の 1 件だけ。seed の 4 行)。
  - 直接 wasm にした関数は 4327 件(lift/harness/hyb/lowered-all.txt。2026-11-21 の push の時点)。
- 未確認: 約 141 は依頼者の申告で、実測ではない。インストーラの一覧(inventory)で確かめるまで、全件の数は未確認。
- 実測が要ること: 表の行数と、抽出の木にある DLL の数(survey-c4-builtin-plugins)。1 件を足す手数の履歴(同 task の手順 7)。全件数は inventory が要る。
- 前提: 共通の前提の inventory(約 141 の一覧はそこから作る)。UI から選べるかは、C-6 のメニューが出るまで確かめられない。内蔵を増やすと、lift と lower の段の時間が増える(推定)。
- 方式(配信の方針 L-035 に従う): 1 プラグイン 1 wasm で試行し、起動後にバックグラウンドで取る。inventory から、1 DLL につき 1 タスクを作る。lower と同じく、型を queue/kinds/ に足し、生成器の入力に一覧を置く。Fruity の効果は共通の hook 点(lift/harness/hooks/fruity-delphi-effect-hook-points.txt)を使うので、共通の部分を直すと他にも効く可能性がある(推定。survey で確かめる)。
- 論点(解決は survey と inventory の後): 約 141 件を 1 つのエンジンの wasm に入れると、大きさと起動(P1 の 10 秒)に効く。エンジン本体の wasm の大きさは、runner の実測で 165848536 バイト(`engine wasm  165848536 B`。165,848,536 バイト、圧縮 br で 17,510,392 バイト)。出典: run 44673024703(worker lower-0xe40550)の判定の出力(成果物 judge.txt)の `engine wasm` の行。同じ行は Issue #81 のコメントにも在る。同じ行は、run 44672756722(lower-0x6f0530。Issue #45)、44672998002(lower-0x7205c0。Issue #74)、44672617908(lower-0xddf630。Issue #24)でも得られ、範囲は 165,820,907〜165,848,536 バイト。依頼者の見立てでは、lower で直接 wasm にする関数が増えるたびに数十 KB 変わる。この見立ては、関数の数と照合していない(推定)。.gitignore(4 行と 30 行)の 143 MB は、それより前の版の数字とみられる(推定)。配信の方針(L-035)では、プラグインは起動後に取るので、起動前の wasm には入れない。1 プラグイン 1 wasm の分け方(今の engine wasm に、どのモジュールの lift 済みの関数が何件入っているか)は、survey-c4 の読みで決める。優先は P3、P2、P1、サイズの順。
- 後回しの VFX 系(案): DLL の PE の import に OpenGL の DLL(opengl32 など)を含む物。一覧と理由を計画に書き、着手の条件は survey の結果で決める。
- 受け入れの条件(案): (a) 目標の全件(後回しの VFX 系を除く)について、inventory の各 DLL に、ci/extract-dlls.tsv の行(SHA-256 つき)と、lift 対象の表の行と、hook 点が在る。(b) 全件が lift・compile・link を通る。(c) 全件が起動確認で通る(state が engine-idle、standIn が false、error が null)。(d) 各種類で 1 件以上が UI から選べる(C-6 の後)。(e) 後回しの VFX 系は、名前の一覧と理由(OpenGL)が計画に在る。(f) 各プラグインの wasm が、起動後のバックグラウンド取得で届き、起動(P1)を遅らせない。
- 最初の 1 件(配信の方針の試行): 3 件の Fruity(Balance・Filter・Delay 2)のうち、Fruity Balance を案として始める。大きさは未確認。survey-c4 が測って、3 件のうち最も小さい物に替えてよい。lift/harness/hyb/lists/ には Fruity の lowered の一覧が無いので、最初の 1 件は、直接 wasm 化の一覧を作るところから始まる(推定)。

### C-5 外部プラグイン(Serum 2・OTT など)

- 現状(コードから):
  - 入力は ci/artifacts.lock の 3 件(Serum 2・OTT・Vital。R2 の artifact バケット。6〜8 行)。作業ジョブに、このバケットの鍵は無い(.github/workflows/agent.yml の R2_ARTIFACT_* は extract ジョブにだけある。取るのは FL のインストーラだけ)。
  - 製品の画面からは、プラグインの仕組みを呼んでいない(app/src/plugins は、テストからだけ使われる)。OTT の harness の source と build は、リポジトリに無い(app/src/plugins/ott-host.ts:1〜3 と app/test/unit/plugins-ott.test.ts:2 が指す lift/harness/vst と lift/harness/web/mt1/plugin-worker/build.sh)。
  - VST3 の link(lift/harness/two/wrp1_vst3.c)は、リンクされたときだけ働く。注釈には「opt-in」とある(lift/runtime/fle_modules.h:82、lift/runtime/fle_ole.c:171)。台帳 L-002 により、既定で有効にする。Serum 2 の native の harness は lift/harness/vst3a/main_vst3a.c にある。
- 配信の方針(L-035): 外部プラグインも、1 プラグイン 1 wasm で、起動後にバックグラウンドで取る。配るのは、ビルドで lift した wasm と必要な data であり、インストーラ(R2 の artifact)は配らない(推定。survey-c5 で確かめる)。OTT の先例は app/src/plugins/ott-host.ts(1 インスタンス 1 Worker)。
- 実測が要ること: 入力が無いことの確認と、読み(survey-c5-external-plugins)。入力が無いので、実測は行わない。
- 前提: 作業ジョブが入力を取る経路を、監査・受付が決める。C-2 の音声の経路が決まってから、プラグインの処理を入れる。
- 受け入れの条件(案): Serum 2 と OTT が、既定で(オプトイン無しで)読み込まれ、1 ブロックの音声を処理する(実測)。P3 の実測で underrun が 0(n=3)。

### C-6 トップバーのコンテキストメニュー

- 現状(コードから):
  - トップバーの部品(Tempo・Song Position・Pattern Selector)は、ページには無い(app/index.html:23〜27 の #toolbar は 3 つの要素だけ)。FL 本体の部品として、エンジンの窓の中で描かれる。
  - 右クリックは、ページが contextmenu の既定の動作を止め(app/src/input/pointer.ts:68・104)、エンジンの窓へ WM_CONTEXTMENU として届く(lift/runtime/fle_input.c:973〜993)。
  - しかし、TrackPopupMenu は何も表示せず、選ばれないまま閉じる(lift/runtime/fle_crash1_menu.c:600〜615。同 fle_crash1_menu.h:5)。これが、メニューが出ない原因の候補である(推定。実測で確かめる)。
- 実測が要ること: headless Chrome で、右クリック後の画面の差と、ログ(survey-c6-topbar-menus)。
- 前提: メニューの描画は、画面の合成(C-2 の分離)の上に載る。C-4 の UI からの選択が、このメニューに依る。
- 受け入れの条件(案): Tempo・Song Position・Pattern Selector の右クリックで、メニューが画面に出る(前後の画面の差が 0 でない)。項目を選ぶと、値が変わる(Tempo の値など)。

## 配信の方針(L-035)

依頼者の提案(台帳 L-035)。実現が難しいと判断した時は、方針を変えてよい(依頼者)。変えたときは、この節と「④ 更新の記録」に書く。

- 起動前に取る物(P1 の 10 秒に入る): FLEngine、言語パック、テーマ、プロジェクトテンプレートなど。FL Studio の起動に必要な wasm が揃った段階で、FL Studio は起動する。
- 起動後にバックグラウンドで取る物: プラグインとアセット。FL の起動処理が始まった時点から、順次取る。プラグインやアセットへのアクセスを起点にしない。
- 1 プラグイン 1 wasm で試行する。1 ページの Worker 数の上限に当たる可能性がある場合も、この限りではない。上限に当たるかを、この計画の中で事前に考えない。
- 現状(コードから。この方針と比べる):
  - 起動時の取得は 4 つで、すべて起動の前にまとめて取る: エンジン wasm・seed pack・DLL pack・glue(app/src/main.ts:714〜717 行)。起動前と起動後の区分は、今は無い。
  - DLL pack は 11 ファイル(FLEngine と 10 モジュール。88675800 B。run 44673024703 の判定の出力の `dll pack` の行)。エンジンへは起動引数(lift/harness/web/pack.args の --args 行)で並べて渡され、Fruity の 3 つを含む。
  - モジュールの PE の扱い: lift/runtime/fle_realmod.h の冒頭は、「ビルドが同じプログラムにリンクし、リベース・lift した第 2 の PE」を real module と呼ぶ。10 モジュールがどちらの扱いか、読みでは分からない(未確認)。
  - lowered の一覧は、モジュール別に lift/harness/hyb/lists/ の 8 件(b2d・dsp・elastique・fle・ft・ilt・oggio・qfc)。Fruity の一覧は無い。
  - 取得の先例: app/src/engine/chunk-io.ts(1〜10 行。decision 0314: 遅延の install tree の chunk を、別の I/O Worker が取り、エンジンは Atomics.wait で待つ)。プラグインごとに Worker を持つ先例: app/src/plugins/plugin-client.ts と app/src/plugins/ott-host.ts(OTT の 1 インスタンス 1 Worker)。
  - 配信: app/worker/index.ts の 2 行目(/assets/* は R2 から、単一の Range・ETag・保存された Content-Encoding で返す)。ページは app/src/bundle/loader.ts の fetchAsset(185 行)と loadPack(219 行)で、manifest の SHA-256 と大きさを検査する(verifyingStream 138 行)。
- 論点(案。依頼者の指示ではない。survey と実装で決める):
  1. 起動後の取得の順序: 何を先に取るか(内蔵の一覧の順、使われやすい物の順など)。
  2. 取得が終わる前にプラグインが使われた時の振る舞い: 待つ、その 1 つを先に取る、など。
  3. 検証と版: 各 wasm の SHA-256 と大きさを、manifest で検査する(今の loader と同じ作り)。ブラウザのキャッシュの扱い。
  4. 起動前の予算: 起動前に取る物の大きさ。今の bundle は、engine wasm 165848536 B(br 17510392 B)、seed pack 20780949 B、DLL pack 88675800 B(同じ判定の出力)。起動前に残す物と、P1 の 10 秒に入るかは、起動の実測(P1)で決める。
  5. 音声への負荷: 取得と wasm のコンパイル・インスタンス化を、音声を作る Worker の外で行う(C-2 の分離と同じ考え)。今は、取得もコンパイルも、ページのメインスレッドで行われている(app/src/main.ts:714〜717 行が loadWasm と loadPack を呼ぶ)。
  6. 1 プラグイン 1 wasm の作り方: fe-build の bundle(lift/crates/fe-build/src/main.rs の pack_step_for)と、wasm の出し方(lift/crates/fe-wasmgen)。survey-c4 で読む。
  7. Worker 数の上限: 1 プラグイン 1 wasm が上限に当たるかは、事前に考えない(依頼者の指示)。試行で当たったら、その時点で方針を見直す。

## 共通の前提: インストーラの一覧(inventory)

- 定義: インストーラ(FL Studio の exe)の中の、インストール先の全ファイルの表。列は、guest のパス、抽出先(lift/out-app/host の中の相対パス)、大きさ(バイト)、SHA-256、(DLL などの実行形式なら)import した DLL の名前。C-3 と C-4 が使う。C-5 の外部プラグインのインストーラにも使えるかは未確認(形式を確かめていない)。
- 経路(案): extract ジョブ(R2 の鍵を持つ)で一覧を作り、キャッシュの木(lift/out-app/host)の中に置く。作業ジョブは、復元された木から読む。commit が要るなら、作業ジョブが queue/inputs/ にコピーして commit する(案。判定の段で scan される)。
- 実現可能性(読みのみ。2026-11-21。この機械では製品を build・実行していない):
  1. 件数と guest パスは、既に作られている。app/scripts/lib/nsis-script.mjs の installTree(102〜124 行)は、section 0 の EXTRACTFILE のうち、インストール先の下の物を [番号, guest パス, ストリームの位置] で返す($PLUGINSDIR と $TEMP は 118 行で除く)。app/scripts/extract-fl.mjs は 69〜70 行でこの表を Map にしているが、出力はしていない。section 0 の外の EXTRACTFILE は、この表に入らない(差の件数は未確認)。
  2. 大きさと SHA-256 は、復号しないと分からない。各ファイルのストリームは独立で(app/scripts/lib/nsis.mjs の streamFile、116〜121 行: 4 バイトの長さ語、LZMA1 の 5 バイトの props、データ)、展開後の長さはストリームに無い。圧縮されていない物(長さ語の最上位ビットが 0)だけは、長さ = clen。復号の所要は、pure JS の app/scripts/lib/lzma1.mjs の速さに依るので、全件の所要は未確認(推定の材料が無い)。
  3. DLL の import は、読める道具が既にある。Rust の fe-pe(lift/crates/fe-pe)の CLI `pe-info FILE` は、import した DLL の名前を出す(lift/crates/fe-pe/src/main.rs の 67〜105 行)。ただし extract ジョブは lift を build しない(agent.yml の extract ジョブは aws と node の段だけで、cargo は動かさない)。extract ジョブで fe-pe だけを cargo build すれば、lift/ を変えずに済む(ビルドの所要は未確認)。
  4. 作る場所: driver/ に新規の 1 ファイル(案: driver/inventory.mjs)。app/scripts/lib/nsis.mjs と nsis-script.mjs を、読むだけで import する。agent.yml の extract ジョブの「インストーラを取り、抽出する」の段で、rm -f の前に呼ぶ。出力は $RUNNER_TEMP/host/inventory.tsv(木の中なので、キャッシュに入り、lift/out-app/host に移る)。
  5. キャッシュの鍵: 既存の木には一覧が無いので、一覧を入れるには鍵を変えて抽出し直すしかない。鍵は agent.yml の 2 か所(64 行: extract ジョブの cache、154 行: run ジョブの restore。どちらも extract-tree-<hashFiles(...)>)にあり、両方に driver/inventory.mjs を足す。run ジョブの restore は fail-on-cache-miss なので、2 か所の鍵がずれると、作業ジョブは全件落ちる。鍵を変えた直後は、同時に並ぶ作業ジョブが全部 cache miss で抽出し直す(重複。推定)。
  6. 失敗時: 一覧の段が落ちると、extract ジョブが失敗する。キャッシュが保存されるのが成功した時だけかは未確認(runs-on/cache の仕様)。保存されなければ、全件が毎回 miss になる。
  7. 触れないもの: app/・lift/・ci/・deploy.yml は変えない。deploy.yml の抽出は、lift/target/release/fe-build extract(deploy.yml の 37 行)で、別の鍵(host-...。32 行)を使う別経路なので、影響しない。ただし、agent.yml の抽出の段は lift/crates/fe-build/src/main.rs の extract_steps(531〜548 行)と同じ手順になるので、2 か所の手順がずれないように注意する。
  8. 結論: agent.yml と driver/ の新規 1 ファイルで、app/・lift/・ci/・deploy.yml に触れずに、一覧は作れる。queue/ の中だけでは置けない(queue/ は待ち行列の状態と入力の置き場所で、道具は置かない)。deploy.yml の paths は app/**・lift/**・ci/**・deploy.yml の 4 つだけ(deploy.yml の 4〜8 行)。
  9. 代案(app/ を変える): app/scripts/extract-fl.mjs に一覧の出力を足すのが一番小さいが、app/** が deploy.yml を動かすので、報告が要る。監査・受付が決める。
- 未確認: 全件の展開の所要と大きさ、section 0 の外の EXTRACTFILE の件数、キャッシュの保存の条件、installTree の件数と extractFiles の件数の差。これらは、一覧の段を入れた後の extract ジョブの計測で測る。
- 実装(2026-11-21。タスク inventory-0。上の 4〜5 の案は、次の形に変えた: extract-tree の鍵と木は変えず、一覧を別のキャッシュに入れる):
  - .github/workflows/agent.yml の extract ジョブに、`if: startsWith(inputs.task, 'inventory')` の 2 段を足した。「一覧(inventory)を作る」は、インストーラを取り直し(既存の抽出と同じ s3 cp)、driver/inventory.mjs で表を作る。「一覧(inventory)を R2 のキャッシュに保存する」は runs-on/cache/save で、キーは inventory-<run_id>-<run_attempt>、path は $RUNNER_TEMP/inventory。run ジョブに同じ if の段を 1 つ足した(runs-on/cache/restore。fail-on-cache-miss: true。同じ path と鍵)。R2_ARTIFACT の鍵は extract の段にだけ渡し、run の段には無い。
  - driver/inventory.mjs(新規): app/scripts/lib/nsis.mjs と nsis-script.mjs を読むだけで使う(app/ は変えない)。installTree の結果から、接頭辞を外した guest パスを作る。表の列は guest・bytes・sha256・kind(dll・exe・other)・imports(dll と exe の PE の import した DLL の名前。小文字、; 区切り)。展開するのは dll・exe と Plugins\・Data\ の下の物だけ(全件の展開の所要は未計測のため)。インストール先の既定値は会社名を含むので、表にもファイルにも書かない。
  - driver/inventory-commit.sh(新規): $RUNNER_TEMP/inventory/fl-installer-files.tsv を、見出しを残して C ロケールで並べ、queue/inputs/fl-installer-files.tsv に置き、1 つの commit にする。
  - queue/tasks/inventory-0.md: mode: script・run: bash driver/inventory-commit.sh・accept: の行(表の行数 100 以上、Effects の dll 20 以上、Generators の dll 10 以上、dll の全行に 64 桁の SHA-256)。
  - 通常のタスク(inventory で始まらない)は、新しい段が全て skip される。spec/driver.test.sh で、inventory の if が 3 つで、他の if: は変更前の 11 件のままであることを検査する。app/・lift/・ci/・deploy.yml は変えない。
  - 未確認: 実インストーラでの動作(NSIS の読み、表の行数と Effects・Generators の dll の件数、展開の所要)。偽の PE と偽の読みで確かめただけ。inventory-0 の run の結果(表の件数)は、その run の後に確かめる。

## タスクの粒度(台帳 L-037)

依頼者の指示(L-037): 2 行足すだけのような機械的な作業に、エージェントを使わない。タスクの最低粒度を、厳し目に設定する。

- エージェントのタスク(`mode: agent`。queue/tasks/ の手書き)は、利用者から見える動作 1 つ、または 1 つの設計課題を終える大きさにする。想定の所要は 3 時間以上(ビルドを何度か回す規模)。本文に `規模: 大` の行を書く。
- 2 行・1 ファイル・1 つの関数・1 つの DLL・1 つの表の行を足すような作業は、エージェントに任せない。
- 同じ型の作業を機械的に増やすときは、型(queue/kinds/。`mode: script`)にし、1 タスクで 30 件以上を扱う(バッチ)。lower は、1 タスク 40 関数のバッチ `lowerb-<k>`(`mode: script`)にする。
- 型は `mode: script` だけ。`mode: script` のタスクは、本文の `run: ` の行のコマンドを、作業ジョブがスクリプトとして実行する(エージェントは動かさない)。

## ③ 着手の順と時期

- 優先: P3、P2、P1、サイズ(AGENTS.md の目標の順)。
- 順(2026-11-21 の改訂。survey の 6 件は完了した。機能の課題は、大きなタスクとして並行で起動する):
  - 5 つを並行で起動する(最低粒度の大きなタスク。P3 の feat-c2-render-worker が最優先だが、独立している物は待たせない)。
  - feat-c2-render-worker(C-2。P3)。描画の合成と提示を、音声と engine の Worker から分ける。
  - feat-c1-hidpi(C-1。P2)と feat-c6-popup-menus(C-6。P2)。
  - feat-delivery-lazy-load(L-035。P1 とサイズ)。最初の 1 件は Fruity Balance(survey-c4 の実測で、3 件のうち最小の 3,718,656 B)。
  - feat-c3-resources-seed(C-3。P1)。今の抽出の木(seed 表と同じ範囲)の中で、起動前の seed の扱いと、見つからなかったファイルの記録を実装する。
  - C-4 と C-5 は、インストーラの一覧(inventory)と lazy-load の最初の 1 件の後に、プラグインのバッチ(型。mode: script)にする。今は作らない。
- 受け入れの条件: 機能のタスクの受け入れは、各タスク(queue/tasks/feat-*.md)の本文の受け入れが正。②の受け入れの条件(案)は、その後で合わせる。
- 時期の目安: 数字の目安は置かない。survey の受理の後、監査・受付が実装のタスクに分けて、順に足す。
- 待ち行列の前提: 非 lower のタスク(survey・計画の課題など)は、lower より先に起動する(driver/fill.sh。台帳 L-036)。どちらも番号の昇順。ready は約 100 件に保つ(生成器)。新しいタスクは、実行中のジョブが 1 件終わった時の fill で起動する(推定。最大で約 1 時間。作業ジョブは 60 並列、1 件あたり約 1 時間(推定))。2026-11-21 の時点では、survey 6 件は ready の lower の後ろに並び、1 件も起動していなかった(起動の確認は、push の後の run と Issue の状態で行う)。
- 実装タスクの大きさ(最低粒度。依頼者の指示): 利用者から見える動作 1 つ、または 1 つの設計課題を終える大きさにする。想定の所要は 3 時間以上で、ビルドを何度か回す規模(本文の `規模: 大`)。作業ジョブの timeout は 350 分。受け入れは本文の accept: の 1 行に書き、判定は作業者の実行とは別にもう 1 回、同じ作業ツリーで行う(冪等に書く)。

## ④ 更新の記録

- 2026-11-21: 初版。台帳 L-033 を足した。survey 6 件(survey-c1-hidpi、survey-c2-worker-split、survey-c3-fl-resources、survey-c4-builtin-plugins、survey-c5-external-plugins、survey-c6-topbar-menus)を、待ち行列に入れた(Issue は ready)。survey は測定と読みだけで、製品は変えない。出力は queue/findings/<名前>.md。
- 2026-11-21(L-034): 依頼者の是正で、C-4 の目標を約 141(効果 約 90 + インストゥルメント 約 51。OpenGL を使う VFX 系は後回し可)に直した。lift 対象の 3 件は、目標ではなく進み具合。共通の前提(inventory)の節を足し、一覧の実現可能性を読みで調べた(agent.yml と driver/ の新規 1 ファイルで可能。app/・lift/・ci/ は変えない)。survey-c4 の本文を直した。C-3 の前提の 1 行を、inventory の節への参照に変えた。
- 2026-11-21(L-035): 依頼者の提案で、配信の方針を入れた(起動前に取る物と、起動後にバックグラウンドで取る物の区分。1 プラグイン 1 wasm の試行。lazy-load)。C-3・C-4・C-5 を、この方針に直した。方針は、実現が難しいと判断した時に変えてよい(依頼者)。
- 2026-11-21(L-036): survey 6 件は、作成から 1 時間以上たっても 1 件も起動しなかった。原因は、driver/fill.sh が ready を番号の昇順で起動し、番号の小さい lower(約 100 件)の後ろに並んでいたこと。fill を直し、非 lower のタスクを先に、lower を後に、それぞれ番号の昇順で起動するようにした(台帳 L-036)。spec/driver.test.sh に、起動の順の検査を 4 件足した。
- 2026-11-21(機能タスク 5 件): survey の 6 件は完了し、実測が入った。依頼者の是正(1800 件のコミットで、HiDPI・右クリック・テンプレートが 1 つも解決していない。タスクの粒度を大きくして開発を加速する)を受けて、機能の課題を大きなタスク 5 件にした: feat-c2-render-worker・feat-c1-hidpi・feat-c6-popup-menus・feat-delivery-lazy-load・feat-c3-resources-seed(queue/tasks/feat-*.md)。Issue は ready で作った(題名は同じ名前)。C-4・C-5 は、inventory と lazy-load の後にバッチにする。feat-c3-resources-seed は、抽出の木が seed 表と同じ範囲(238 件 = 227 + 11)なので、件数の増加は受け入れに入れていない(survey-c3 の実測による)。
- 2026-11-21(inventory): インストーラの全ファイルの一覧の経路を入れた(タスク inventory-0。extract ジョブの 2 段と run ジョブの 1 段が inventory の作業だけで動く。driver/inventory.mjs・driver/inventory-commit.sh・queue/tasks/inventory-0.md)。spec/driver.test.sh は 182 件が通る。queue/tasks の検査は、mode: script なら run: の行、mode: agent なら 規模: 大を要求する形に直した。実インストーラでの動作は未確認。
