# feat-c6-popup-menus: 右クリックのメニューを画面に出し、項目を選べるようにする(C-6。P2)

mode: agent
規模: 大

## 目的
依頼者の言葉: 「Tempo、Song Position、Pattern Selector などトップバーのコンテキストメニューが機能していない」(台帳 L-033 の (6))。queue/plan.md の C-6 を実装で終える。右クリックの後、メニューが画面に出て、項目を選ぶと、engine に WM_COMMAND(または TrackPopupMenu の戻り値)が届くこと。

## 背景(survey の実測。出典: queue/findings/survey-c6-topbar-menus.md の `## 実測` と `## コードの読み`)
- ページに Tempo・Song Position・Pattern Selector は無い(app/index.html:23〜27 の #toolbar は、Tab の送り・表示の情報・Stop の 3 つだけ)。部品は FL 本体の画面の中にあり、engine が描く。
- 右クリックは、ページが contextmenu の既定の動作を止め(app/src/input/pointer.ts:68・104)、engine の窓へ WM_CONTEXTMENU として届く(lift/runtime/fle_input.c:973〜993)。
- しかし、TrackPopupMenu は何も表示せず、何も送らずに閉じる(lift/runtime/fle_crash1_menu.c:600〜615。注釈は fle_crash1_menu.h:5)。これが、メニューが出ない原因。
- 測定の実測(survey): 起動後の画面には、Welcome 窓と、その上に「Help us improve FL Studio by sending usage data」の確認ダイアログ(I'm in / No thanks)があり、部品は見えなかった。survey は部品の右クリックを送っていない(位置が読めなかった)。参考点の右クリックでは、既定の動作は止まったが、engine のログは増えなかった(322 → 322 行)。

## 方針(設計の細部は作業者が決める。制約は下の「制約」)
- lift/runtime/fle_crash1_menu.c の fle_imp_TrackPopupMenu で、メニューの項目(このファイルの menu の表)を、画面の窓として描く。描き方は、既存の描画の仕組み(lift/runtime/fle_desk.h の合成の注釈が示す窓の描き方)に従う。
- 項目を選んだら、WM_COMMAND(0x0111。wParam の下位 16 ビットが項目の ID、hwnd へ送る)を送る。TPM_RETURNCMD(0x100)のときは WM_COMMAND を送らず、選んだ ID を戻り値にする(選ばなければ 0)。今の 600〜601 行の「何も送らない」を変える。
- 選んだ時と表示した時に、engine のログ(既存の仕組み。stdout/stderr の行。app/src/engine/worker.ts の pushLog)に、WM_COMMAND か TrackPopupMenu の語を含む行を出す。測定の道具がこの行を読む。
- 測定の道具の中だけで、起動時の確認ダイアログを「No thanks」で閉じる。製品の既定の動作は変えない(依頼者の指示)。
- 3 つの部品(Tempo・Song Position・Pattern Selector)の位置は、起動後の画面から作業者が読む。読めない場合は、報告に「未確認」と書く。受け入れは、任意の 1 点の右クリックでメニューが出ることと、項目を選んだ時の WM_COMMAND のログで判定する。

## 制約(守る。台帳の依頼者の指示)
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。オプトインを作らない(既定で有効。L-002)。
- ベンダー名・Cloudflare のアカウント ID・ホスト名・絶対パスを書かない。wrangler.jsonc を書き換える仕組みを作らない。マスクの命令を置かない。Python を使わない。
- commit は環境の git の名義のまま(git config・git -c・--author・GIT_* を使わない)。commit に Claude の行を付けない。ブランチを作らない。
- lift/・app/ を変える push は main の deploy.yml を動かす。報告に、変えたパスを書く。
- 他のタスク(feat-c2-render-worker が描画の経路を変えうる。feat-c1-hidpi が canvas の大きさを変えうる)と衝突しうる。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、下の受け入れをもう一度通してから commit する。

## 読む物(実在を確認済み)
- lift/runtime/fle_crash1_menu.c(600〜615 行: TrackPopupMenu。メニューの表の作り)、lift/runtime/fle_crash1_menu.h(先頭の注釈)
- lift/runtime/fle_input.c(973〜993 行: 右ボタンを離した時の WM_CONTEXTMENU)
- lift/runtime/fle_desk.h(窓を描く仕組みの注釈)、lift/runtime/fle_win.c(窓の表。存在を確かめてから読む)
- app/src/input/pointer.ts(68 行: onContext。104 行: contextmenu の登録)
- app/src/engine/worker.ts(pushLog。engine のログの経路)、app/src/main.ts(engineLog の公開。`window.__app.engineLog()`)
- app/index.html(23〜27 行: #toolbar)
- tools/boot-check/boot-bundle.mjs、tools/boot-check/cdp.mjs
- queue/findings/survey-c6-topbar-menus.md(`## 次の手順の案(推定)` の 1〜4)

## やること
1. 変更の前に、現状を 1 回測る(boot-bundle の起動で、起動後の画面を PNG に保存して読む。部品が見えるか。確認ダイアログの有無)。
2. 実装(上の方針。fle_crash1_menu.c の TrackPopupMenu を中心に。メニューの描き方は既存の窓の仕組みに合わせる)。
3. 測定の道具を作る: tools/feat-c6-popup-menus/measure.mjs(JSON の 1 行だけを stdout に出す)。boot-bundle.mjs と同じ起動で、engine-idle になったら、(a) 確認ダイアログの「No thanks」を CDP のクリックで閉じる(測定の道具の中だけ)、(b) 右クリックの前の画面を PNG で保存する、(c) 任意の 1 点(起動後の画面の中で、窓の内側)に右クリックを送る(mousePressed と mouseReleased を button "right" で)、(d) 1 秒待ち、画面の PNG を保存する、(e) 右クリックの位置の近く(半径 200 px の正方形)で、前後の画像の差のある画素の数を数える(changedNearClick)、(f) 出たメニューの項目の 1 つをクリックする(項目の位置は、メニューの描き方の実装が決める。位置は作業者の道具の中で決め、報告に書く)、(g) engine のログの、クリック前からの増分(logAdded)を取る。
4. 1 行の JSON: {"state","standIn","error","changedNearClick","changedTotal","logAdded":[...]}。
5. 部品の位置の読み取り(報告): Tempo・Song Position・Pattern Selector の位置を、起動後の画面から読み、読めた物は右クリックで試す。読めない物は「未確認」。これは受け入れに入れない。

## 受け入れ
下の accept の行が 0 で終わること。内容は次のとおり。
- 通常のビルド(app・cargo・fe-build engine・fe-build bundle)と、boot.json の起動確認(state が engine-idle、standIn が false、error が null)。
- tools/feat-c6-popup-menus/measure.mjs の出力(1 行)を node -e が読む。state が engine-idle、standIn が false、error が null、changedNearClick が 0 より大きい(右クリックで、その付近に画面の変化が出た)、logAdded に WM_COMMAND または TrackPopupMenu を含む行がある(項目を選んだ時の engine のログ)。

accept: npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c6-popup-menus/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c6.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const ok=r.state==="engine-idle"&&r.standIn===false&&r.error===null&&r.changedNearClick>0&&r.logAdded.some((l)=>/WM_COMMAND|TrackPopupMenu/.test(l));process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c6.json"

## 報告の規則
- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg)を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変えたパス(lift/・app/ を含む)、メニューの描き方の選択と理由、項目を選んだ時の戻り値と WM_COMMAND の ID、3 つの部品の位置が読めたか(読めた物の右クリックの結果を、位置と一緒に)。
- 確認ダイアログを「No thanks」で閉じたのは測定の道具の中だけであること(製品の既定の動作は変えていない)を書く。

## 禁止と注意
- 測定の値を、accept の判定のために書き換えない(道具の出力の生の値を、accept の node -e が読む)。
- 製品の既定の動作を、測定のためだけに変えない。
- sleep・timeout・nohup・disown・末尾の & は使わない(待ちは JS のタイマー)。
- 一時ファイルは .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。
- ログ・Artifacts・run を削除しない。
