---
draft: "235"
title: "File メニューに『Manage files』を足す(『Revert to last backup』と『Exit』の間)。選ぶと OPFS のファイル管理の窓が出る"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["topbar-menu", "verify"]
size: "XL"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の質問と指示(2026-11-27。原文): 「質問: ツールバーメニューに任意の項目を追加することは可能ですか？もし可能である場合、File...メニューの"Revert to last backup"と"Exit"の間に"Manage files"メニューを追加してください。このメニューを開くと、OPFSウインドウが出現します。」

依頼者の質問への答え(PM。文章で返した): 「可能と見ている。FL のメニューは FL 自身のコードが組み立て、engine はそれをそのまま動かす。(1) メニューを組み立てる時に項目を足す(FL の項目の構造を engine の側から作って並びに挿す。または項目の配列を書き換える)、(2) 選ばれた時の処理を、ページ側の OPFS の窓を開く関数に結ぶ、の 2 点が要る。項目の並びの中の位置の指定は可能。確認が要るのは、項目の配列を実行時に変えられるかと、クリックの処理がメニューの ID の分岐に固定されていないか」。

この Issue の目標: File メニューに「Manage files」を、「Revert to last backup」と「Exit」の間に足す。選ぶと、OPFS のファイル管理の窓が出る。

## 背景

- 読み(`pm-work/findings/fb-2026-11-27.md` の 3 節): File の見出しは `TNewMenu`(lid 81)。ドロップダウンは `TQuickPopupMenuWindow`(lid 162)で、項目は 10 件(`tools/e2e/features/generated/menus.tsv:2-12`)。
- 項目の名前は、どの記録にも無い。`menus.tsv` は `File > #n` の番号だけ。`tools/e2e/lib/common.mjs` の `menuPath` は、下キーの回数で項目を選ぶ。
- crash-hunt(`pm-work/findings/crash-hunt-2026-11-25.json`)の結果: #8 は確認の窓 1 件と子 1 件、#9 は書き出しのサブメニュー(子 13 件。`TWAVRenderForm[Render options]` を出す)、#10 は Exit(記録に明記)。「Revert to last backup」の番号は確認できない。推定では #8。
- メニューの組み立ての関数・項目の構造・押下の処理は、リポジトリに読みも記録も無い。inline hook(`lift/runtime/fle_vmap.h`)は、lift の時に `--hook-points` で名前付けした関数だけ。
- OPFS のファイル管理の窓: `app/src/files/mount.ts` の `mountFiles` と `FilesHandle.openWindow()` が、公開の入口。今は `mountFiles` を呼ぶ箇所が無い(#1481 で外れた)。

## 読む物

- `pm-work/findings/fb-2026-11-27.md`(3 節)
- `pm-work/findings/crash-hunt-2026-11-25.json`(`probes` の `File/n`)・`tools/crash-hunt/hunt.mjs`(メニューを 1 項目ずつ押す道具)
- `tools/e2e/features/generated/menus.tsv`・`tools/e2e/features/menus.tsv`・`tools/e2e/lib/common.mjs`(`menuPath`・`openMenu`・`readMenuRows`)・`tools/e2e/scenarios/menus/`(`_generate.mjs` と既存のシナリオの書き方)
- `lift/runtime/fle_win.c`・`lift/runtime/fle_vmap.h`・`lift/harness/hooks/`
- `app/src/files/mount.ts`・`app/src/files/explorer.ts`・`app/src/files/engine-dialog.ts`
- FL の File メニューを組み立てるコード(`TNewMenu`・`TQuickPopupMenuWindow` の項目の作成と押下)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 実現の見込みの確認。File メニューの項目を組み立てる処理と、押下の処理を、読める範囲で読む。項目を足す方式を 1 つ決め、理由を報告に書く。候補は次のとおり。
   - (a) 項目の並びに、FL の項目の構造を engine の側から 1 件挿す。
   - (b) 項目の組み立て・押下の関数に、harness の hook を入れる(lift の時の番地の名前付けが要る)。
   - (c) 項目の配列を、実行時に書き換える。
   不可能と分かったら、理由と代替を報告して終える(この Issue の結果は「差し戻し」)。
2. 項目の名前の読み方を決める。項目の名前は記録に無いので、ゲストのメモリの文字列(項目の見出し)を読む方法を決める。OCR は使わない。
3. 項目の追加: 位置は「Revert to last backup」と「Exit」の間。表示の名前は `Manage files`。
4. 押下の処理: 選ばれたら、ページ側の OPFS の窓を開く(`FilesHandle.openWindow()`、または `mountFiles` の窓の入口)。ページ側から窓を出す入口は、engine の側から呼べる形にする(製品の旗を足さない)。
5. 項目の追加で、E2E のメニューの一覧(`tools/e2e/features/generated/menus.tsv`)が変わるので、`tools/e2e/scenarios/menus/_generate.mjs` で作り直し、`node --test tools/e2e/test/*.test.mjs` が通ることを確かめる。
6. 赤を先に確かめる。E2E のシナリオ(下の 7)を先に書き、今の本番で赤になることを確かめる。
7. E2E のシナリオを `tools/e2e/scenarios/menus/` に足す:
   - `menus/manage-files-order`: File メニューを開き、項目の並びで `Manage files` が `Revert to last backup` の次・`Exit` の前にあること(項目の名前は手順 2 の読み方で取る)。DPR 1 と DPR 2。
   - `menus/manage-files-opens-window`: File メニューから `Manage files` を選ぶと、OPFS のファイル管理の窓が出ること(窓の表の class と、窓の中の `#fx-input-files` の存在)。DPR 1 と DPR 2。
8. 報告する(下の「報告の規則」)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node --test tools/e2e/test/*.test.mjs && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --only menus/manage-files-order,menus/manage-files-opens-window --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e-1.jsonl" && node tools/e2e/run.mjs --only menus/manage-files-order,menus/manage-files-opens-window --displays 4k-dpr2 --budget 40m > "$RUNNER_TEMP/e2e-2.jsonl" && node -e 'const fs=require("fs");const L=process.argv.slice(1).flatMap((f)=>fs.readFileSync(f,"utf8").trim().split("\n")).map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.error===null&&r.scenarios.length===2&&r.scenarios.every((s)=>s.ok===true&&s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e-1.jsonl" "$RUNNER_TEMP/e2e-2.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` と `node --test tools/e2e/test/*.test.mjs` が通る。
- E2E の 2 行(DPR 1・DPR 2)の、2 本のシナリオが全て `ok`。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 実現の見込みの読み(方式の候補と、選んだ方式と理由。不可能なら理由と代替)、(b) 項目の名前の読み方、(c) 直す前の `menus/manage-files-order` の赤の出力の行、(d) 項目の位置(Revert と Exit の間に入ったか。前後の項目の名前)、(e) 窓の class(`manage-files-opens-window` の notes)、(f) 流れ A〜D の 4 行、(g) `node --test` の最後の行、(h) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。項目を足すのは既定の動作にする(オプションにしない)。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 段ごと(1 の見込みの確認、2 の項目の追加、3 の押下の処理)に commit する。1 回で終わらない前提で進める。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{233}}(File→Open の OPFS の窓)の窓の入口を使う。233 が未完でも、既存の `mountFiles` の窓を出せば進められる。
