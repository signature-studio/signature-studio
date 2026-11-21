# feat-c1-hidpi: HiDPI の拡大率で画面をぼやけさせない(C-1。P2)

mode: agent
規模: 大

## 目的
依頼者の言葉: 「HiDPI に対応しておらず画面がぼやけている」(台帳 L-033 の (1))。queue/plan.md の C-1 を実装で終える。拡大率 1・1.5・2 で、描画の画素と CSS の大きさが一致し、窓の文字や部品が、拡大率 1 の時と同じ CSS の大きさに見えること。

## 背景(survey の実測。出典: queue/findings/survey-c1-hidpi.md の `## 実測`)
- 拡大率 2 で、canvas[data-desk="1"] は 640×480、CSS は 132×99(room が小さいため guest 画面が下限に丸められた)。窓を 1580×957 にした追加の測定では、canvas 1580×957、CSS 1580×957、ratio = canvas 幅 ÷(CSS 幅 × dpr)= 0.5。描画バッファは CSS 1px に 1 画素で、ブラウザが 2 倍に引き伸ばしている。
- コード: app/src/ui/desktop.ts:24〜25 は canvas の大きさを guest 画面の画素数にする。app/src/ui/screen-fit.ts:3〜4 は「1 guest pixel = 1 CSS pixel」を設計にしている。app/src/main.ts:260〜276 は room を CSS px で測る。dpr は表示と記録にしか使われていない(main.ts:265・275・349、beacon の記録は main.ts:450 で dpi が null)。
- 画面の大きさを engine に渡す既存の経路は、環境変数 FLE_MONITOR_W / FLE_MONITOR_H だけ(app/src/ui/screen-fit.ts:63〜65 の screenEnv。engine.env に入る。lift/runtime/fle_screen.h:6〜13 の注釈)。
- DPI は guest 側で固定: lift/runtime/fle_gdc.h:31 の FLE_GDC_LOGPIXELSX は 96(仮想ディスプレイの 96 DPI。lift/runtime/fle_sysmetrics.h:43〜51 の注釈による)。
- 窓の大きさは、既定の窓では room が小さい(survey の値は 500×99 CSS px)。測定の窓は 1580×957 に固定する。

## 方針(設計の細部は作業者が決める。制約は下の「制約」)
- 案 A(survey-c1 の次の手順 2 の案 A): guest 画面の大きさを room × dpr(物理画素)にし、canvas の大きさも同じにする。engine へは既存の FLE_MONITOR_W / FLE_MONITOR_H に物理画素を渡す。canvas の CSS の大きさは room のまま。
- guest に DPI を伝え、窓の文字や部品が、拡大率 1 の時と同じ CSS の大きさに見えるようにする。DPI の経路は作業者が設計する。FLE_GDC_LOGPIXELSX を実行時の値にする場合、_Static_assert(lift/runtime/fle_gdc.h と fle_screen.h の対応)を壊さないこと。
- 画素数は拡大率の 2 乗で増える(survey の推定: 1580×957 → 3160×1914 は 4 倍)。描画の所要(composeMs・putMs)が増えるので、影響を実測で報告する。

## 制約(守る。台帳の依頼者の指示)
- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない。既存の FLE_MONITOR_W / FLE_MONITOR_H は値を変えて使ってよいが、名前を増やさない。オプトインを作らない(既定で有効。L-002)。
- ベンダー名・Cloudflare のアカウント ID・ホスト名・絶対パスを書かない。wrangler.jsonc を書き換える仕組みを作らない。マスクの命令を置かない。Python を使わない。
- commit は環境の git の名義のまま(git config・git -c・--author・GIT_* を使わない)。commit に Claude の行を付けない。ブランチを作らない。
- app/・lift/ を変える push は main の deploy.yml を動かす。報告に、変えたパスを書く。
- 他のタスク(feat-c2-render-worker・feat-c6-popup-menus)が、同じファイルを変えうる(app/src/ui/desktop.ts・app/src/main.ts・app/src/engine/client.ts・app/src/engine/desk.ts など)。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、下の受け入れをもう一度通してから commit する。

## 読む物(実在を確認済み)
- app/src/ui/desktop.ts(21〜27 行: deskCanvas。45〜52 行: apply の CSS の大きさ)
- app/src/ui/screen-fit.ts(3〜4 行の注釈。44〜65 行: chooseGuestScreen・drawScale・screenEnv)
- app/src/main.ts(260〜276 行: measureRoom と applyDisplay。303〜304 行。335 行と 339〜344 行: engine.env への screenEnv。450 行: beacon の dpr と dpi)
- app/src/engine/client.ts(75 行: fl-engine の Worker。86 行: transferControlToOffscreen)
- app/src/engine/desk.ts(80 行: compose。91〜101 行: 共有メモリの分岐)、app/src/engine/desk-gl.ts(208 行: surface の大きさ)
- lift/runtime/fle_screen.h(先頭の注釈 1〜13 行)、lift/runtime/fle_gdc.h(31 行と 41 行)、lift/runtime/fle_sysmetrics.h(43〜51 行)
- tools/boot-check/boot-bundle.mjs(起動と待ちの手順)、tools/boot-check/cdp.mjs(launchChrome の extraArgs)
- queue/findings/survey-c1-hidpi.md(測定の手順と、拡大率 2 の値)

## やること
1. 変更の前に、現状を 1 回測る(拡大率 1 と 2。tools/feat-c1-hidpi/measure.mjs を作る前の手で。出力の行を報告に貼る)。
2. 設計と実装(上の方針)。
3. 測定の道具を作る: tools/feat-c1-hidpi/measure.mjs(JSON の行だけを stdout に出す。ログは stderr)。boot-bundle.mjs と同じ起動(stage-assets、cdp、ISOLATION)で、拡大率 1・1.5・2 の 3 回を起動する。拡大率は extraArgs に `--force-device-scale-factor=<値>` を足す(拡大率 2 は survey で効いた。1.5 は未確認。効かなければ CDP の Emulation.setDeviceMetricsOverride の deviceScaleFactor に替え、報告に書く)。窓は 1580×957 に固定する(--window-size と Emulation の両方)。engine-idle の後、1 行の JSON を出す: {"dpr","state","standIn","error","elapsedMs","canvasW","canvasH","cssW","cssH","composeMs","putMs","presents"}。canvas は canvas[data-desk="1"] の width・height と getBoundingClientRect。composeMs・putMs・presents は window.__app.deskStats。
4. 拡大率 1.5 と 2 の画面をスクリーンショットで保存し(repo の .tmp-* に置き、終わったら消す)、窓の文字の大きさが拡大率 1 と同じ CSS の大きさに見えるかを、画像を読んで報告する(判定には入れない)。
5. 画素数の影響を報告する: 拡大率 1 と 2 の、composeMs と putMs の 1 回あたり(累計 ÷ presents)。推定ではなく、この実行の値。

## 受け入れ
下の accept の行が 0 で終わること。内容は次のとおり。
- 通常のビルド(app・cargo・fe-build engine・fe-build bundle)と、boot.json の起動確認(state が engine-idle、standIn が false、error が null)。
- tools/feat-c1-hidpi/measure.mjs の出力(3 行)を、accept の node -e が読む。3 行の dpr は 1・1.5・2 を含み、各行で state が engine-idle、standIn が false、error が null、canvasW と canvasH が round(cssW × dpr) と round(cssH × dpr) に ±1 画素で一致する。
- elapsedMs は出力に含めるが、判定には入れない。

accept: npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/feat-c1.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=L.length===3&&[1,1.5,2].every((d)=>L.some((r)=>r.dpr===d))&&L.every((r)=>r.state==="engine-idle"&&r.standIn===false&&r.error===null&&Math.abs(r.canvasW-Math.round(r.cssW*r.dpr))<=1&&Math.abs(r.canvasH-Math.round(r.cssH*r.dpr))<=1);process.exit(ok?0:1)' "$RUNNER_TEMP/feat-c1.jsonl"

## 報告の規則
- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、runner の種類(nproc と loadavg)を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 変えたパス(app/・lift/ を含む)、DPI の経路の選び方、拡大率 1.5 が効いたか、拡大率 1 の値が変わっていないこと、画素数の増加の影響(composeMs・putMs の実測)。
- 確認できなかった物は「確認できない」と書く。

## 禁止と注意
- 製品の既定の動作を、測定のためだけに変えない。測定の道具の中だけで、起動の条件(窓の大きさ、拡大率)を決める。
- 測定の値を、accept の判定のために書き換えない(道具の出力の生の値を、accept の node -e が読む)。
- sleep・timeout・nohup・disown・末尾の & は使わない(待ちは JS のタイマー)。
- 一時ファイルは .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。
- ログ・Artifacts・run を削除しない。
