# survey-c6-topbar-menus: C-6(トップバーの Tempo・Song Position・Pattern Selector などのコンテキストメニュー)を、この runner の headless Chrome で測る(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg の値)を添える。
- 代役・再生・事前に作った画像を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行を付ける。推定は「推定」と書き、実測と分ける。
- 「達成」「直った」「メニューが出る」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c6-topbar-menus.md だけ。
- 製品の作業(ビルド・実行)はこの runner の中だけで行う。読むだけの手順は、その旨を書く。
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(この task では何も足さない)。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (6): 「Tempo、Song Position、Pattern Selector などトップバーのコンテキストメニューが機能していない」。queue/plan.md の C-6 を参照。コードの読みでは、トップバーの部品はページに無く、FL 本体の部品がエンジンの窓の中で描かれる。右クリックは、ページからエンジンの窓へ WM_CONTEXTMENU として届くが、メニューを出す TrackPopupMenu は何も表示せず閉じる(lift/runtime/fle_crash1_menu.c:600〜615)。この task は、実際に右クリックして、画面とログに何が起きるかを測る。

## 読む物(実在を確認済み)
- app/index.html(23〜27 行: #toolbar の中身。Tab の送り、表示の情報、Stop の 3 つだけ)
- app/src/input/pointer.ts(68 行: onContext は preventDefault だけ。104 行: contextmenu の登録)
- lift/runtime/fle_input.c(973〜993 行: 右ボタンを離した時に WM_CONTEXTMENU を窓へ送る)
- lift/runtime/fle_crash1_menu.c(600〜615 行: TrackPopupMenu は何も表示しない)と lift/runtime/fle_crash1_menu.h(5 行)
- tools/boot-check/boot-bundle.mjs と tools/boot-check/cdp.mjs(起動と CDP の使い方)

## 先頭の注意(入力)
- 入力は、このリポジトリのビルド物(engine と bundle)だけ。依頼者の個人のファイルは使わない。
- 実測には engine の bundle が要る。ビルドの段が失敗したら、ここで止め、「実エンジン: 実行なし(ビルドの段: <段の名前>、終了コード <n>)」と報告する。

## やること
1. 版と抽出物を確かめる。`grep '^EMSDK_VERSION=' ci/versions.env` の結果と、`find lift/out-app/host -type f | wc -l` の件数を書く。
2. ビルドする(queue/kinds/lower.md の accept と同じ段): `npm ci --prefix app`、`npm run build --prefix app`、`cargo build --release --locked --manifest-path lift/Cargo.toml`、`lift/target/release/fe-build engine`、`lift/target/release/fe-build bundle`。各段の終了コードを書く。
3. 読みを記録する(実行しない): `grep -rn 'Tempo\|Song Position\|Pattern Selector\|songpos\|contextmenu' app/src app/index.html --include=*.ts --include=*.html` の結果を貼る。ページの中に、これらの部品が在るかを書く(無ければ「無い」)。
4. 実測(headless Chrome)。tools/boot-check/boot-bundle.mjs の main を、同じディレクトリの一時ファイル tools/boot-check/.tmp-survey-c6.mjs に写す(import の相対パスを保つため。.tmp-* なので git の無視の対象。終わったら消す)。配信・起動・待ちは boot-bundle.mjs と同じにし、state が engine-idle になるまで待つ。
5. 実測(画面)。engine-idle の後、次の手順で画面を測る。
   a. desk の canvas の位置を取る: `(() => { const c = document.querySelector('canvas[data-desk="1"]'); const r = c.getBoundingClientRect(); return { left: r.left, top: r.top, w: r.width, h: r.height, dpr: window.devicePixelRatio, canvasW: c.width, canvasH: c.height }; })()` の結果を出す。
   b. Page.captureScreenshot(format png)で画面を撮り、repo の中の .tmp-survey-c6/ に保存する。read ツールで画像として開き、Tempo・Song Position・Pattern Selector の位置(CSS の座標)を、画像から読む。読めなければ「未確認(位置が読めない)」と書く。位置は、canvas の左上からの画素で書く。
   c. 右クリックの前に、contextmenu が届いたかを記録する: `window.addEventListener('contextmenu', (e) => { window.__cmPrevented = e.defaultPrevented; }); 0` を評価する。
   d. 各部品について、Input.dispatchMouseEvent で mousePressed(button right、clickCount 1)と mouseReleased(button right)を、その位置に送る。必要なら、先に `document.getElementById('desktop').focus()` を評価する。送った後、1 秒待ってから画面を撮り、直前の画面と PNG のバイト列の sha256 を比べる(同じか違うか)。
   e. 各部品の後に、`window.__cmPrevented` と、`window.__app.engineLog()` の末尾の 10 行を取る。メニューに関係する行(menu や popup などの語)が有るかを書く。
6. 3 回走らせる(手順 4 と 5 の全部)。各回の結果を JSON の行で出し、findings に貼る。
7. 後始末: `rm -rf tools/boot-check/.tmp-survey-c6.mjs .tmp-survey-c6` を行い、残った一時ファイルが無いことを `git status --short` で確かめる。

## 出力
- queue/findings/survey-c6-topbar-menus.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1・2・5・6 の結果(出力の行つき)。部品ごとに、右クリック後の画面の変化(同じ / 違う)、contextmenu が既定の動作を止めたか、ログの末尾の内容を表で書く。
  - `## コードの読み`: 手順 3 の結果と、読む物の要点(パスと行番号つき)。右クリックの経路(ページ → エンジン → TrackPopupMenu)を、順に書く。
  - `## 未確認`: 部品の位置が読めなかったもの、測れなかった項目と理由
  - `## 次の手順の案`: C-6 の実装の案。TrackPopupMenu がメニューを描画の窓として出し、項目を選んだら WM_COMMAND を返す最初の段(TPM_RETURNCMD を含む)。推定と明記する。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c6-topbar-menus.md && grep -q '^## 実測' queue/findings/survey-c6-topbar-menus.md && grep -q '^## 未確認' queue/findings/survey-c6-topbar-menus.md && grep -q '^## 次の手順の案' queue/findings/survey-c6-topbar-menus.md
