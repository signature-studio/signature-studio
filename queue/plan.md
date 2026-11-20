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
| C-4 | 100 以上の内蔵エフェクトとインストゥルメントが 1 つも無い | P1、P3 | survey-c4-builtin-plugins |
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
- 前提: 全件の一覧を作る道具を extract ジョブに足す。ci/ の変更は main の deploy.yml を動かすので、報告に書く。
- 受け入れの条件(案): 全件の一覧と抽出の木の差が 0。起動時の「見つからない」が 0。言語・テーマ・テンプレートの件数が一覧と一致する。

### C-4 内蔵のエフェクトとインストゥルメント 100 以上

- 現状(コードから):
  - lift の対象は 10 DLL(lift/harness/b2d2/modules.tsv)。FL の内蔵は、そのうち Fruity Balance・Fruity Filter・Fruity Delay 2 の 3 つ(同 8〜10 行)。残りはエンジンのライブラリ(dsp・Blend2D・QuickFontCache・FreeType・ILTools・oggio・elastique)。
  - インストゥルメントの DLL は 0 件(ci/extract-dlls.tsv に無い。seed の Plugins\Fruity\Generators は GM CCs.txt の 1 件だけ。seed の 4 行)。
  - 直接 wasm にした関数は 4327 件(lift/harness/hyb/lowered-all.txt。2026-11-21 の push の時点)。
- 未確認: 内蔵の全件の数(インストーラの一覧が runner に無い。C-3 と同じ)。
- 実測が要ること: 表の行数と、抽出の木にある DLL の数(survey-c4-builtin-plugins)。
- 前提: C-3 の全件の一覧。UI から選べるかは、C-6 のメニューが出るようになるまで確かめられない。内蔵を増やすと、lift と lower の段の時間が増える(推定)。
- 受け入れの条件(案): 内蔵のエフェクトとインストゥルメントが 100 以上。全件が lift 済み(表の行数が一致)。全件が起動確認で通る。各 1 件以上が UI から選べる(C-6 の後)。

### C-5 外部プラグイン(Serum 2・OTT など)

- 現状(コードから):
  - 入力は ci/artifacts.lock の 3 件(Serum 2・OTT・Vital。R2 の artifact バケット。6〜8 行)。作業ジョブに、このバケットの鍵は無い(.github/workflows/agent.yml の R2_ARTIFACT_* は extract ジョブにだけある。取るのは FL のインストーラだけ)。
  - 製品の画面からは、プラグインの仕組みを呼んでいない(app/src/plugins は、テストからだけ使われる)。OTT の harness の source と build は、リポジトリに無い(app/src/plugins/ott-host.ts:1〜3 と app/test/unit/plugins-ott.test.ts:2 が指す lift/harness/vst と lift/harness/web/mt1/plugin-worker/build.sh)。
  - VST3 の link(lift/harness/two/wrp1_vst3.c)は、リンクされたときだけ働く。注釈には「opt-in」とある(lift/runtime/fle_modules.h:82、lift/runtime/fle_ole.c:171)。台帳 L-002 により、既定で有効にする。Serum 2 の native の harness は lift/harness/vst3a/main_vst3a.c にある。
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

## ③ 着手の順と時期

- 優先: P3、P2、P1、サイズ(AGENTS.md の目標の順)。
- 順:
  1. C-2(P3)。描画と音声の分離を最初に終える。
  2. C-1(P2)と C-6(P2)。C-1 は C-2 の受理の後に行う(画素数を増やすと、分離の無い engine の Worker の負荷が増えるため。推定)。
  3. C-3(FL の必要物)。
  4. C-4(内蔵の数)。C-3 の一覧と、C-6 の後(UI から選ぶ確認)に行う。lower の進みを見ながら進める。
  5. C-5(外部プラグイン)。C-2 の音声の経路が決まり、入力の取得経路が決まってから行う。
- 時期の目安: 数字の目安は置かない。survey の受理の後、監査・受付が実装のタスクに分けて、順に足す。
- 待ち行列の前提: 待ちは番号の昇順で処理する。ready は約 100 件に保つ(生成器)。2026-11-21 の push の時点では ready 98 件(番号 563〜660)、running 56 件。survey 6 件は、この後ろに並ぶ。作業ジョブは 60 並列、1 件あたり約 1 時間(推定)。ready の 98 件が起動し終えるまで約 2 時間(推定)なので、新しいタスクは数時間以内に起動する(推定。実際の起動は Actions の実行で確かめる)。
- 実装タスクの大きさ: 1 件が約 1 時間で終わる大きさにする。受け入れコマンドを本文の accept: の 1 行に書き、実測の出力を報告に残す。

## ④ 更新の記録

- 2026-11-21: 初版。台帳 L-033 を足した。survey 6 件(survey-c1-hidpi、survey-c2-worker-split、survey-c3-fl-resources、survey-c4-builtin-plugins、survey-c5-external-plugins、survey-c6-topbar-menus)を、待ち行列に入れた(Issue は ready)。survey は測定と読みだけで、製品は変えない。出力は queue/findings/<名前>.md。
