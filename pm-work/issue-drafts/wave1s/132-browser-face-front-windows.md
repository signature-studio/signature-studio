---
draft: "132"
title: "Browser の Sounds タブの面(iframe)が、FL のウィンドウの前後関係を守る: 前面の窓・メニュー・ダイアログの下に、面を出さない"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["render", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、Browser の Sounds タブの面(WebView2 の面。ページ側の `iframe`)が、FL の窓より前に出ている。Welcome や Channel Rack などの、Browser より前面の FL の窓が、面に隠れる。File メニューを開くと、メニューの下部が隠れる。

この Issue の目標は次の 3 つ。
1. Browser の面が、Browser より前面にある FL の窓・メニュー・ダイアログの下に出ない。前面の窓の矩形と、面の見える部分が重ならない。
2. 面の見える部分は、Browser の窓の矩形(`put_Bounds`)から、前面の窓の矩形を除いた残り。何も前面に無い時は、矩形と一致する。
3. マウス・キーの入力は、見える部分の中では面が、それ以外では FL の canvas が受ける。

## 背景

- #1362(commit 305bee51。`app/src/ui/webview2-surface.ts`)は、WebView2 の面を、描画の canvas と同じ親の下の `iframe` に写した。`WebView2Face.ensure` は、全面の wrapper(`pointer-events: none`)と、その中の `iframe`(`pointer-events: auto`)を作る。iframe は canvas より上の最前面に在り、`put_Bounds` の矩形を全て覆う。そのため、Browser より前面の FL の窓が、iframe の下に入る。
- 依頼者の実機の報告(2026-11-24): Sounds タブを開いた状態で File メニューを開くと、メニューの下部が隠れる。Welcome・Channel Rack などの FL の窓も、面の下に入る。
- 窓の前後関係は、engine の描画の記述にある(`fle_desk_scene`: 窓を下から上へ並べた表と、各窓の矩形。`lift/runtime/fle_desk.c`。`app/src/engine/desk.ts`・`app/src/engine/desk-gl.ts` が読む)。Browser の窓(`TWPControl`。その子の `TEdgeBrowser`)より後に並ぶ窓が、前面の窓。
- 前面の窓の例: File のメニュー、Welcome(クラス `TWelcomeWizard`)、ダイアログ、ツールチップ、同じ親の兄弟の前面の窓。窓のクラス名は、作業者が窓の表で確かめる。
- #1362 の面の位置・大きさ・表示・資源の仕組み(`put_Bounds`・`put_IsVisible`・`Navigate`)は、そのまま使う。この Issue は、面の見える部分を変える。

## 設計(PM の決定。実測で確かめる)

案 A(既定): 面の `clip-path` で、前面の窓に隠された部分を除く。
- 見える部分 = Browser の窓の矩形(`put_Bounds` を、窓の client の原点で動かした矩形)から、Browser より前面の全ての窓の矩形を、矩形の差(rect subtraction)で除いた残り。残りは互いに重ならない矩形の集まりとし、`clip-path: path(evenodd, …)` で書く(座標は面の CSS 画素。面の原点から)。
- 前面の窓は、描画の記述(窓の表)から取る。窓の変化(開く・閉じる・移動・大きさ・前後関係の変更)に従って更新する。描画の提示のたびに求めてよいが、`style.clipPath` は値が変わった時だけ書く。何も前面に無い時は、`clipPath` を空にする。
- 入力: 見える部分の中では面が受け、それ以外では canvas が受ける(`clip-path` の外は、`iframe` のヒットテストから外れる)。

案 B(採らない): 面を canvas の下に置き、canvas の面の領域を透明に描く。入力は canvas が受けるので、面に届かない(合成した入力は信頼されない)。

## 測定

測定の道具 `tools/browser-panel/measure.mjs` を広げる(既存の項目は残す。判定の基準は変えない)。1 回の起動で、次の 4 つの状態を順に作り、各状態で測り、1 行の JSON にする。順序は作業者が決め、報告に書く。
- `welcome`: Welcome 窓が開いている時(起動の時点)。
- `fileMenu`: File メニューを開いた時(メニューの窓の矩形)。
- `frontWindow`: Browser より前面の、Browser の矩形と重なる FL の窓(例: Channel Rack)を出した時。窓の表から、Browser より前面で重なる窓を選ぶ。
- `none`: 何も前面に無い時(前の状態を閉じた後)。

JSON の項目(トップレベルは既存の `dpr`・`error`・`state`・`standIn`・`elapsedMs`・`preBootBytes`・`boundsRect`・`iframeRect`・`boundsMatch` のまま。その下に `states` を足す):
- `states.<状態>.overlapped`: 前面の窓の矩形と、Browser の矩形が重なる時 true。測定が意味を持つ確認。重ならない時は、窓を Browser の矩形の上へ動かしてから測る(動かした方法を報告に書く)。`none` には無い。
- `states.<状態>.frontRect`: 前面の窓の矩形(CSS 画素。`{x, y, w, h}`)。`none` は null。
- `states.<状態>.overlapPoints`: 前面の窓の矩形を各辺 2 CSS 画素縮めた内側で、2 CSS 画素の格子の点を取り、`document.elementFromPoint` が面の `iframe`(`FACE_ATTR` の印)を返した点の数。目標は 0。
- `states.fileMenu.bottomHit`: メニューの下端の内側 4 CSS 画素の位置で `document.elementFromPoint` が返した要素の tagName。目標は `CANVAS`。
- `states.none.clipMatchesBounds`: `boundsMatch` が true で、かつ面の `style.clipPath` が空の時 true。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-132 && B="$PWD/lift/out-app/bundle/raw/fl-Oz" && node tools/browser-panel/measure.mjs "$B" > ".tmp-132/m.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const s=r.states;const ok=r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.dpr===1&&["welcome","fileMenu","frontWindow"].every((k)=>s[k]&&s[k].overlapped===true&&s[k].overlapPoints===0)&&s.fileMenu.bottomHit==="CANVAS"&&s.none.clipMatchesBounds===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(ok?0:1)' ".tmp-132/m.json" && node tools/smoke/welcome-flow.mjs "$B" > ".tmp-132/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' ".tmp-132/flow.jsonl" && rm -rf .tmp-132 && npm test --prefix app
```

判定の基準(DPR 1):
- 起動が正常(`error` が null・`state` が engine-idle・`standIn` が false)。
- `welcome`・`fileMenu`・`frontWindow` の各状態: `overlapped` が true(測定が意味を持つ)、`overlapPoints` が 0。
- `fileMenu.bottomHit` が `CANVAS`。
- `none.clipMatchesBounds` が true。
- `elapsedMs` が 12500 以下、`preBootBytes` が 314,368,711 以下(#1362 の基準)。
- 流れ A・B・C・D(`tools/smoke/welcome-flow.mjs`)の判定は #1362 と同じ(各行が、`error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true)。
- 単体テスト(`npm test --prefix app`)が通る。

## やること

0. 作業ツリーの確認。`.prior-work.md` が在れば、最初に読む。
1. 前面の窓を取る経路を作る。描画の記述(`fle_desk_scene`)と、窓の変化の通知が、面を描く側に届いているかを、`app/src/engine/desk.ts`・`desk-gl.ts`・`app/src/engine/worker.ts`・`app/src/engine/protocol.ts` で確かめる。届いていなければ、既存の仕組みに足す(利用者向けのオプションは作らない)。
2. `app/src/ui/webview2-surface.ts`: 見える部分の計算(矩形の差)、`style.clipPath` の更新(値が変わった時だけ)、前面の窓が無い時の扱い。`put_Bounds` の矩形と原点の扱いは今のまま。
3. 測定の道具 `tools/browser-panel/measure.mjs` を広げる(上の 4 つの状態の作り方と、項目)。
4. 1〜3 を、1 回のビルドでまとめて入れる(ビルドは約 35 分。1 回の試行で、数回まで)。
5. 受け入れを通す。通っても通らなくても、作業ツリーを commit する。
6. 報告: 前面の窓をどの経路で取ったか(描画の記述の項目名)、各状態の値、窓を重ねた方法(ドラッグの有無)、入力の確認(面の上の `elementFromPoint` の結果)、半透明の窓の扱い(見える部分の外は canvas の描画になる。その旨を書く)。

## 読む物

- `app/src/ui/webview2-surface.ts`(`WebView2Face`・`faceRect`・`ensure`・`place`)、`app/src/ui/desktop.ts`、`app/src/main.ts`(面の生成)
- `app/src/engine/desk.ts`・`app/src/engine/desk-gl.ts`(窓の記述の読み方)、`lift/runtime/fle_desk.c`(`fle_desk_scene`。窓の表)
- `app/src/engine/worker.ts`・`app/src/engine/protocol.ts`(Worker から面への通知)
- `tools/browser-panel/measure.mjs`(今の測定の項目)、`tools/smoke/welcome-flow.mjs`(流れ A〜D と、窓の表の使い方)
- `pm-work/findings/browser-panel-118.md`(#1362 の記録)

## 他のタスクとの関係

- blocked_by: なし。
- #1362(完了)の面の仕組みを変える。`app/src/ui/webview2-surface.ts` と `tools/browser-panel/measure.mjs` は #1362 と同じファイル。
- 流れ A〜D は、この Issue の変更の後の版で測る。
- 他の Issue が `app/src/engine/` と衝突する場合: 作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 見た目は、`overlapPoints`・`bottomHit`・`clipMatchesBounds` の値で言う。「直った」とは書かない。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、コード・テスト・設定に、分岐や表として入れない。
- ベンダー名と Cloudflare の識別子を書かない。
- `.github/`・`inputs/`・`guard/` は変えない(読むだけ)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。
