まず、受け入れコマンドが使う道具と入力の一覧を確認します。
次に、インストーラの一覧から `.flp` の候補を探します。
候補は 165 件あります。展開の木の場所と、playlist を確かめる方法を調べます。
展開の木は `runner.temp/host` にあり、テンプレートやプロジェクトは `lift/out-app/host-extra/` に置かれる設計のようです。ワークフローの残りと、設計の該当部分を確かめます。
展開の木 `lift/out-app/host/` があります。その中に demo・テンプレートのプロジェクトがあるか確かめます。
展開の木に 82 件の `.flp` があります。playlist の件数を数える既存の道具が無いか、リポジトリ内を検索します。
リポジトリには `.flp` を読む道具がありません。playlist の件数を数えるため、FLP の形式の仕様を調べます。
イベントの基底は得られました。次に、playlist のイベント ID が定義されているファイルを取得します。
playlist のイベントは ID 233(`DATA + 25`)で、1 件が 32 または 60 バイトの構造体です。`.tmp-*` が無視されるか確かめてから、候補の playlist 件数を数えます。
`.tmp-*` は無視されます。インストーラ一覧の `.flp` ごとに、展開の木の実物を読んで playlist の項目数を数える一時の道具を作ります(完了後に消します)。
出力の整形が崩れたので、1 行 1 件の JSON に直して読み直します。
`tracks` と clip/pattern の分類が 1 バイトずれています。pyflp の定義では `pattern_base` は +4、`item_index` は +6、`track_rvidx` は +12 です。直して数え直します。
playlist の項目を持つ候補は 48 件です。demo の曲(`Data\Demo projects\Demo songs\`)の中で項目数が最多の `Eric Kauffmann & NrgMind - Exoplanet.flp`(4440 項目)を選びます。展開の木のファイルを `.tmp-p3-project.flp` に置き、大きさと SHA-256 がインストーラ一覧と一致するか確かめます。
大きさ 950527 と SHA-256 `76a5cd7a…` が一覧の値と一致しました。`inputs/p3-project.tsv` を書いて commit します。
commit `fd55ddb5` ができました。一時の道具は不要になったので消します。次に、受け入れコマンドが使う道具(依存、cargo、emsdk)が揃っているか確かめます。
依存・cargo・emcc が揃っています。受け入れコマンドを、タスクの記載どおりに実行します(フォアグラウンド、長い時間の上限を指定)。
build と boot は通りました(`state: engine-idle`、`standIn: false`、`error: null`)。次に、受け入れと同じ 3 回の測定を実行します。1 回目が失敗しても残りを取れるように、ループは止めずに回し、判定は最後に受け入れと同じ式で行います。
3 回とも「File > Open のダイアログが 120 秒開かない」で落ちました。道具の `openProject` は「まだ実行していない(Not run yet)」と書かれた経路で、メニューの座標(18,20)・(48,85)を前提にしています。参照元の道具の同じ手順と比べて、原因を調べます。
参照元の `openFile` も同じ座標(18,20)→(48,85)を使っています。ただし参照元は、メニューが開いたことを確かめてから次に進みます(`waitWindow('TQuickPopupMenuWindow')` で最大 3 回再試行)。道具は確かめずに進むので、最初の File クリックでメニューが開いていない可能性があります。画面で確かめるため、道具の一時コピーに各段階のスクリーンショットを入れて 1 回だけ流します(完了後に消します)。
一時コピーを流します(待ち時間は 20 秒に短縮しています)。
Open の項目を押した直後に、エンジンが「stopped (exit code 134)」(異常終了、SIGABRT 相当)になっています。3 枚の画面を見て確かめます。
スクリーンショットで原因が見えました。1 枚目では、File の位置に「Welcome to FL Studio」の窓が重なっています。そのため (18,20) のクリックは、メニューではなく Welcome の窓に当たっています。2 枚目では (48,85) が Welcome の一覧の「Launch Loop Starter」に当たり、エンジンが止まりました。`tools/p2-frames/measure.mjs` は「Welcome の窓は既存の操作(X)で閉じる」としているので、その手順を確かめます。
手順は分かりました。p2 の測定は、起動後に Welcome の窓(`TWelcomeWizard`)を窓の一覧から探し、その X を押して消えるのを待ってから次に進みます。#1303 の probe にはこの段がありません。p2 側の該当コードを確かめます。
p2 の手順(Welcome の X を押し、窓が消えるまで待つ)を #1303 の probe に入れれば、メニューに届くようになるはずです。道具を直す前に、この道具に対応するテストがあるか確かめます。
`tools/` を変えると判定が spec を走らせます。判定の該当部分を読み、`driver.test.sh` が無いことの影響を確かめます。
判定は `tools/` の変更で `spec/guard.test.sh` と `spec/workflow.test.sh` を走らせます。`spec/driver.test.sh` は無いので飛ばされます。原因は確かめたので、診断用の一時コピーは消します。
wizard の X を押してから窓が消えるのを待つ段を、probe に加えます。p2 と同じ読み方(`deskDumpBase64(false).windows`、ガイドのピクセル座標)を使い、結果は JSON の `welcome` に記録します。
`WINDOWS` の定数を、既存の定数の並びに足します。
構文を確かめてから、修正した道具で 1 回だけ流して、File > Open まで進むかを見ます。
X を押しても、15 秒たっても Welcome の窓が消えません。画面は 780×480 なので、窓の右側(X の位置)が画面の外に出ている可能性があります。窓の位置と大きさを、一時コピーで確かめます。
窓は (0,0) から 874×573 で、画面 780×480 より大きいため、X(856,18)は画面の外です。Escape を押すと窓が消えました。Escape 後の画面を確かめます。
Escape で Welcome の窓は消えました。今回の診断では、後段の File > Open もそのまま進んでいたようで、プロジェクトの読み込みで「Problems loading the project」(プラグイン 2 件と音のサンプル 4 件が見つからない)の窓が出ています。診断の結果ファイルを読み、読み込み後の状態を確かめます。
診断の順序を見ると、読み込みの窓は Escape の直後に出ています。そこで、(700,200) のクリックを外し、Escape だけで Welcome が消えるか、プロジェクトが読み込まれないかを切り分けます。main 窓の題名も記録します。
Escape だけで Welcome は消えました。その直後に既定のプロジェクト(FLEX と Emphasizer が開けず、サンプルも無いので「Problems loading the project」)が読み込まれ、`TMissingPluginsForm` の窓が出ます。この窓が開いていると File メニューが使えず、Open に届きません。そこで、この窓の位置を取って、その「Ok」を押せるかを診断します。
Ok に当たる位置(窓の左上から +368, +327)を押すと、窓が閉じました。押した後の画面を確かめます。
Ok を押すと、既定のプロジェクトが読み込まれた状態で File メニューが見えます。これで File > Open の経路に進めます。Welcome は Escape で閉じ、「Problems loading」の窓は測った大きさ(691×359)のときだけ Ok を押す、という形に直します。窓の一覧を読む処理は共通の関数にまとめます。
`closeWelcome` を、Escape で閉じる版に置き換え、共通の窓の読み取りと「Problems loading」の窓の処理を加えます。
プロジェクトの題名を待つ間に「Problems loading」の窓を閉じるようにし、閉じた内容を返します。
出力の欄を 1 つ足し、定数のコメントを新しい関数名に合わせます。
構文を確かめてから、修正版で 1 回流します。
Welcome が閉じ、Ok も押せました。次の段で `evaluate: Uncaught` が出ています。例外の中身が出ていないので、まず `deskDumpBase64` の実装で「ask again」がどこから出るかを確かめます。
`desk-dump-error` は拒否の文言(例: 「ask again」)として返ります。probe の `ev` は例外の説明文を捨てているため、`Uncaught` としか見えていません。例外の説明文も含めるように直し、1 回流して原因を読みます。
今度は「File > Open」の窓は開きましたが、`#filedlg-input` の要素がまだ DOM に無い状態で触っています。この窓の作りを、アプリのソースで確かめます。
`#filedlg-input` は開く用の窓でだけ作られます。保存用や拒否の窓なら要素はありません。実際にどの要求の窓が開いたかを、要求の記録と窓の文字で確かめる一時コピーを流します。
開いた窓は、要求の種類は正しい(Open、.flp)ものの、ブラウザの保管領域のファイル管理(`fx-list` の一覧と `fx-ok` の決定ボタン)です。`#filedlg-input` の経路は今のアプリには無い、と読めます。ソースで確かめます。
現在のアプリは、Open の要求には OPFS のファイル管理の窓(`host.select`、決定 0216)を使います。probe の「ファイル選択の入力」の経路は今は使われません。参照元の `openProject` の手順(アップロードして、一覧の行を選んで OK)を読みます。
現在の経路は、ファイル管理(Files 窓)へ入れてから File > Open で一覧の行を選び、`#fx-ok` を押す形です。probe の `openProject` をこの形に直します。まず現在の関数全体を読みます。
`chooser`(ファイル選択の窓の受け取り)は不要になるので外します。`openProject` を、ファイル管理への追加と一覧からの選択に置き換えます。
main 側のファイル選択の受け取りを外し、呼び出しを新しい形に合わせます。
構文を確かめてから、1 回流します。
ファイル管理から選ぶ経路は通り、今度は題名の待ちで 180 秒切れです。プロジェクトが開いたかどうか、そして `pageTitle()` が何を返すかを確かめます。
`pageTitle()` は前面の窓の題名を返します。プロジェクトの読み込み後に別の窓(「Problems loading」の窓の大きさが違うなど)が前面にあると、題名は一致しません。待ちを 30 秒に縮め、切れた時点の画面と窓の一覧を残す一時コピーで確かめます。
パッチの貼り方が崩れて構文エラーになりました。題名の待ちの終わりだけを差し替える、単純な形で作り直します。
今回は、ファイルの受け付けまで進み(`fd.hist`: answered、950527 バイト)、その後にエンジンが停止しました(exit 134、SIGABRT)。前面は「Song info」の窓です。画面を見て、停止の内容を確かめます。
画面には、エンジンが止まった理由が出ています。「first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect」です。これは、ある層を持つ窓(layered window)へ初めて非背景の画素が書かれた時に停止する仕組みに見えます。決定 0126 の中身を確かめます。
決定 0126 は、エンジンの「wall」(設計上の停止規則)の番号です。停止の文言を、ランタイムとドキュメントで検索して、これが意図した停止か、製品の不具合かを確かめます。
「first non-background pixel」は停止の証拠を書く記録で、止めた原因ではありません。止めたのは別の SIGABRT で、その理由(runtime message)は、エンジンのログに出ているはずです。ログの取り方を、起動確認の道具から確かめます。
页面の `window.__app.engineLog()` でエンジンのログが読めます。停止の直前の行を、同じ診断の窓口で読みます。
ログの末尾には、プラグイン(Fruity Filter の DLL)の読み込みの途中で `LockResource: no PE image is mapped` が出ており、そこで止まっています。原因はこの製品のエンジンのプラグイン読み込みの側にあり、probe の操作ではありません。この候補(Exoplanet)では、今のビルドで読み込みが止まります。

選び直しの判断をする前に、同じ条件(Song Mode の playlist を持つ demo の曲で、項目数が多い順)で次の候補を試します。候補は `DEDZII - NIGHT TERROR.flp`(playlist 3850 項目)です。
DEDZII も題名の待ちで 180 秒切れです。原因を Exoplanet と同じに決めつけず、時間切れの時点の窓の一覧、前面の題名、エンジンのログの末尾、画面を取って確かめます。
DEDZII も同じ形で止まりました。`Song info`(TInfoForm)の窓が出た直後に、ログ `LockResource: no PE image is mapped at 0x400000` で停止します。Exoplanet も同じでした。演奏とは関係なく、この demo の曲のうち「Song info」の付いたものが開けない、と読めます。

テンプレートは候補に入っています(Issue の本文の「demo・テンプレートのプロジェクト」)。テンプレートの中で playlist の項目数が最多の `Data\Templates\Other\Blocks\Blocks.flp`(260 項目)を試します。
Blocks でも題名の待ちで切れます。ここでも窓の一覧と前面の題名とログの末尾を取って、原因を確かめます。
Blocks では、エンジンは生きていて、プロジェクトは読み込まれています。「Problems loading」の窓の大きさが違う(691×287)ため、Ok の位置が合わずに題名が変わりません。窓の下端から見た Ok の位置を、画面で確かめます。
Ok の位置は、窓の下端から約 29 px 上、左端から 368 px でした(Blocks: 96+287-29=354、Exoplanet の窓も同じ式で合います)。窓の高さは内容で変わるので、Ok は「下端から 29 px」で決めます。道具の該当部分を直します。
Blocks で probe が最後まで通りました(`error: null`、Space が届いた `keyAtGuest: true`、再生の quantum 11368、再生中の 10 秒の窓の underrun は `underrunDelta: 0`)。

ただし、受け入れが見る `underrunQuanta` は AudioContext を開始してからの累計で、今回は 961 です。probe は、プロジェクトを開く前に AudioContext を開始しているため、読み込みの間の underrun がこの数に入っています(窓の中は 0)。P3 の定義は「Song Mode の再生で」なので、AudioContext の開始を、プロジェクトを開いた後(Space の直前)に移します。プロジェクト無しの経路は変えません。
構文を確かめて、修正後の流れで Blocks を 1 回流します。
この順でも cumulative は 977 で、読み込みを移す前の 961 とほぼ同じでした。つまり数の大半は AudioContext を開始した直後の立ち上がりの途切れで、プロジェクトの読み込みによるものではありません。AudioContext の開始を後ろへ動かす変更には効果が無いので、元の順序に戻します。代わりに、どの時刻に途切れが起きたかを、ページ側の履歴(`audio.history`)から時刻の範囲として出します。
続けて、窓の終わりで、途切れの時刻の範囲を `audio.history` から出します(AudioContext の開始からの ms)。
この読み取りの定数を、窓の定数の隣に置きます。
出力の欄にも `underrunSpans` を足して、構文を確かめ、Blocks を 1 回流します。
途切れの時刻の範囲が出ました(AudioContext 開始からの ms)。

- 27〜707 ms: 137 quanta
- 917〜4417 ms: 624 quanta
- 12097〜12977 ms: 297 quanta

10 秒の再生の窓では、`underrunDelta` が 0 です。窓の開始時点で累計が既に 1069 なので、途切れは全部、窓より前(開始直後とプロジェクトの読み込み中)にあります。

選ぶ物は `Data\Templates\Other\Blocks\Blocks.flp` にします。demo の上位 2 件は、この版では読み込みでエンジンが止まったためです。インストーラ一覧の値と照らしてから、TSV を直します。
TSV を Blocks に直しました。probe の使い方を説明する冒頭のコメントに、今回の変更(Welcome の閉じ方、ファイル管理からの開き方、JSON の追加欄)を 1 段落足します。
`tools/` を変えたので、判定と同じ 2 本の spec を先に通します。
spec は両方通りました。TSV と道具の変更を commit してから、測定を回します。
commit 済みです。受け入れと同じ 3 回の測定を回します。1 回目が失敗しても 3 回とも取り、最後に受け入れの判定式で読みます。
3 回とも `error` は null、再生の quantum は 11,266〜11,373 で、Space は届いています(`keyAtGuest: true`)。ただし `underrunQuanta` は 1015・1081・1041 で、受け入れの判定(0 であること)は 3 回とも満たしません。判定の式も exit 1 です。途切れの時刻の範囲は、各回の `underrunSpans` にあります。

測定が終わったので、一時ファイルを消します(リポジトリの中の `.tmp-*` と、自分で作った `RUNNER_TEMP` の一時ファイル)。
一時ファイルは消え、作業ツリーは clean です。報告に貼る JSON 1 行を、ファイルから改めて読んで、そのまま写します。
実エンジン: `lift/out-app/bundle/raw/fl-Oz` を `tools/p3-audio-probe/measure.mjs` で起動し、`Data\Templates\Other\Blocks\Blocks.flp` を開いて Space で再生した。3 回とも `error` は null、再生の quantum は 11266〜11373、`keyAtGuest` は true。`underrunQuanta` は 1015・1081・1041 で、受け入れの基準(0)を満たさない。

**判定: 差し戻し**(3 回とも `underrunQuanta` が 0 でない)。耳の確認はしていない。

## 1. 選んだプロジェクト

- 候補: `inputs/fl-installer-files.tsv` の `Data\` 下の `.flp`(165 件)のうち、展開の木 `lift/out-app/host/resources` にある 80 件を読み、Playlist イベント(ID 233)の項目数を数えた。項目を持つのは 48 件。
- 選ぶ過程:
  1. demo の曲で項目数が最多の `Eric Kauffmann & NrgMind - Exoplanet.flp`(4440 項目)は、開くとエンジンが止まった(SIGABRT、exit 134)。停止の直前のログは `fle: LockResource: no PE image is mapped at 0x400000 (no MZ signature)`。
  2. 次の `DEDZII - NIGHT TERROR.flp`(3850 項目)も、同じ形で止まった。
  3. テンプレートで項目数が最多の `Data\Templates\Other\Blocks\Blocks.flp`(260 項目)は、読み込みから再生まで進んだ。これを選んだ。
- `inputs/p3-project.tsv`(commit `d5696f7a`)に、guest・bytes(57737)・sha256(`3d3eee6d…`、一覧と一致)・playlist(260)を書いた。`.tmp-p3-project.flp` は消した。
- 読み込み時の警告: 4 件のサンプル(`DNC_*.wav`)が見つからない。そのため、そのチャンネルは無音になる。

## 2. 測定

runner の値。この機械とは別枠。nproc は 4。loadavg は各回の終わりの値。

1 回目(loadavg 1.33, 0.96, 0.86):
```
{"startedAt":"2026-11-23T21:29:46.192Z","bundle":"fl-Oz","mode":"run","load":"none","project":".tmp-p3-project.flp","state":"engine-idle","standIn":false,"error":null,"gesture":"mouse","consent":"\"No thanks\" at guest (513, 308) of 780x480","welcome":{"wizard":"closed","missing":"Ok at guest (412, 390) of a 691x359 dialog"},"projectDialogs":["Ok at guest (412, 354) of a 691x287 dialog"],"underrunSpans":{"startedAtPage":11410,"spans":[{"fromMs":29,"toMs":4469,"quanta":729},{"fromMs":12359,"toMs":13243,"quanta":280}]},"keyAtGuest":true,"before":{"status":"needs-gesture","ctxState":"suspended","nodeReady":true,"deviceState":"playing","gestures":0,"quantaWhilePlaying":0,"underrunQuanta":0,"engineGapFrames":14112,"lastResume":{"at":2667,"trigger":"start","result":"pending","ms":1500.6,"stateAfter":"suspended"}},"ctxState":"running","status":"running","quantaWhilePlaying":11266,"underrunQuanta":1015,"engineGapFrames":143067,"playingSeconds":32.699501133786846,"deviceState":"playing","presentsDelta":108,"measuredMs":10001,"elapsedMs":46186,"chromeVersion":"Chrome/157.0.8103.78","nproc":4,"loadavg":[1.33,0.96,0.86],"screenDevicePx":null,"dpr":null,"quantaDelta":3449,"bootLast60sAt10s":null,"bootLast60sAt70s":null,"bootLast60sAt90s":null,"maxRefillStallMs":null,"readAtMs":null,"underrunQuantaSinceFirstIdle":1015,"underrunEpisodes":33,"longestRunQuanta":280,"nonSilentQuanta":0,"engineGapFramesLast60s":128955,"underrunDelta":0}
```

2 回目(loadavg 1.04, 0.97, 0.87):
```
{"startedAt":"2026-11-23T21:30:41.105Z","bundle":"fl-Oz","mode":"run","load":"none","project":".tmp-p3-project.flp","state":"engine-idle","standIn":false,"error":null,"gesture":"mouse","consent":"\"No thanks\" at guest (513, 308) of 780x480","welcome":{"wizard":"closed","missing":"Ok at guest (412, 390) of a 691x359 dialog"},"projectDialogs":["Ok at guest (412, 354) of a 691x287 dialog"],"underrunSpans":{"startedAtPage":11160,"spans":[{"fromMs":36,"toMs":4476,"quanta":791},{"fromMs":12106,"toMs":12949,"quanta":283}]},"keyAtGuest":true,"before":{"status":"needs-gesture","ctxState":"suspended","nodeReady":true,"deviceState":"playing","gestures":0,"quantaWhilePlaying":0,"underrunQuanta":0,"engineGapFrames":null,"lastResume":{"at":2506,"trigger":"start","result":"pending","ms":1501.7,"stateAfter":"suspended"}},"ctxState":"running","status":"running","quantaWhilePlaying":11373,"underrunQuanta":1081,"engineGapFrames":151618,"playingSeconds":33.010068027210885,"deviceState":"playing","presentsDelta":108,"measuredMs":10002,"elapsedMs":46344,"chromeVersion":"Chrome/157.0.8103.78","nproc":4,"loadavg":[1.04,0.97,0.87],"screenDevicePx":null,"dpr":null,"quantaDelta":3452,"bootLast60sAt10s":null,"bootLast60sAt70s":null,"bootLast60sAt90s":null,"maxRefillStallMs":null,"readAtMs":null,"underrunQuantaSinceFirstIdle":1081,"underrunEpisodes":32,"longestRunQuanta":283,"nonSilentQuanta":0,"engineGapFramesLast60s":128150,"underrunDelta":0}
```

3 回目(loadavg 0.87, 0.94, 0.87):
```
{"startedAt":"2026-11-23T21:31:36.149Z","bundle":"fl-Oz","mode":"run","load":"none","project":".tmp-p3-project.flp","state":"engine-idle","standIn":false,"error":null,"gesture":"mouse","consent":"\"No thanks\" at guest (513, 308) of 780x480","welcome":{"wizard":"closed","missing":"Ok at guest (412, 390) of a 691x359 dialog"},"projectDialogs":["Ok at guest (412, 354) of a 691x287 dialog"],"underrunSpans":{"startedAtPage":11302,"spans":[{"fromMs":76,"toMs":4357,"quanta":734},{"fromMs":12096,"toMs":12976,"quanta":307}]},"keyAtGuest":true,"before":{"status":"needs-gesture","ctxState":"suspended","nodeReady":true,"deviceState":"playing","gestures":0,"quantaWhilePlaying":0,"underrunQuanta":0,"engineGapFrames":12346,"lastResume":{"at":2623,"trigger":"start","result":"pending","ms":1501.1,"stateAfter":"suspended"}},"ctxState":"running","status":"running","quantaWhilePlaying":11373,"underrunQuanta":1041,"engineGapFrames":150155,"playingSeconds":33.010068027210885,"deviceState":"playing","presentsDelta":109,"measuredMs":10001,"elapsedMs":46324,"chromeVersion":"Chrome/157.0.8103.78","nproc":4,"loadavg":[0.87,0.94,0.87],"screenDevicePx":null,"dpr":null,"quantaDelta":3448,"bootLast60sAt10s":null,"bootLast60sAt70s":null,"bootLast60sAt90s":null,"maxRefillStallMs":null,"readAtMs":null,"underrunQuantaSinceFirstIdle":1041,"underrunEpisodes":29,"longestRunQuanta":307,"nonSilentQuanta":0,"engineGapFramesLast60s":137809,"underrunDelta":0}
```

## 3. underrun の合計と時刻の範囲

時刻は、AudioContext を開始してからの ms(`underrunSpans` の実測)。

| 回 | underrunQuanta(累計) | 再生の窓(Space 後 10 秒)の underrunDelta | 途切れの区間 |
|---|---|---|---|
| 1 | 1015 | 0 | 29〜4469 ms: 729 quanta / 12359〜13243 ms: 280 quanta |
| 2 | 1081 | 0 | 36〜4476 ms: 791 quanta / 12106〜12949 ms: 283 quanta |
| 3 | 1041 | 0 | 76〜4357 ms: 734 quanta / 12096〜12976 ms: 307 quanta |

- 実測: 3 回とも、途切れは窓より前の 2 区間に入り、窓の中の数は 0。窓の開始時点で累計が既に全数と同じだったため(`underrunDelta` が 0)。
- 推定: 2 つ目の区間(12 秒前後)は、Space を送る前後と思われる。この実測では Space の時刻を記録していない。
- 推定: 累計と区間の合計の差(6〜11 quanta)は、履歴の先頭より前の分と思われる。
- 1 つ目の区間(0〜4.5 秒)は、プロジェクトの読み込みと重なる。AudioContext をプロジェクトの読み込みの後に開始する版も試したが、累計は 977 で、同程度だった。原因はまだ特定できていない(推定: 起動直後にエンジンが出力に追いつけていない。#1334 の範囲)。

## 4. 受け入れの出力

- 受け入れの前半(build・cargo・fe-build・bundle・boot-check): 通った。`boot.json` は `state: engine-idle`、`standIn: false`、`error: null`、`elapsedMs: 11522`、`timelineIdle: 11333`。
- P1 の 10 秒の基準について: この実測では engine-idle まで 11.0〜11.8 秒(runner の値)。
- 後半(n=3 の判定の式): exit 1(3 回とも `underrunQuanta !== 0`)。

## 5. 道具の修正(`tools/p3-audio-probe/measure.mjs`、commit `d5696f7a`)

#1303 の道具は、プロジェクトを開く経路を一度も実行していなかった(コメントにもその旨があった)。次の点を直した。

- Welcome の窓は X が画面の外にあるため、Escape で閉じる。
- プロジェクトは、ページのファイル管理(決定 0216)へ入れてから File > Open で選ぶ。旧い「ファイル選択」の経路は今の版に無い。
- 「Problems loading」の窓は、高さが内容で変わる(359 と 287 を確認)。Ok は「下端から 29 px、左端から 368 px」。
- 例外の文言を省かずに出す。
- JSON に `welcome`・`projectDialogs`・`underrunSpans` を足した。

`spec/guard.test.sh` と `spec/workflow.test.sh` は通った(`JUDGE_IN_SPEC=1`)。

## 6. 別に見つかった製品の問題(報告のみ。直していない)

- demo の曲 `Exoplanet`(4440 項目)と `DEDZII`(3850 項目)は、File > Open で開くと、Song info の窓の後にエンジンが止まる(exit 134)。ログの末尾は上の `LockResource` の行。別の Issue にする価値がある。
- 読み込みの `Blocks.flp` で見つからないサンプル 4 件は、展開の木に無い(`%FLStudioFactoryData%` の下)。そのため、この測定は、一部が無音のプロジェクトの再生になる。

## 7. 次に必要なこと

- 受け入れは、`underrunQuanta` の累計が 0 であることを求めている。この版では、AudioContext の開始直後の途切れ(約 1000 quanta)があるため、満たせない。
- 判断を依頼者に返す: (a) 起動直後の途切れを #1334 で直す、または (b) 受け入れの基準を「再生の窓の `underrunDelta` が 0」に改める。(b) は受け入れの定義を変えることなので、依頼者が決める。
