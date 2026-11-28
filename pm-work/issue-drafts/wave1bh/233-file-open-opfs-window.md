---
draft: "233"
title: "File→Open で、ブラウザのネイティブのファイル選択を出さず、OPFS のファイル管理の窓を出す: 窓の答えを FL へ返す"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["verify", "resources"]
size: "L"
priority: "High"
project: "signature-studio/3"
---

## 目的

依頼者の指摘(2026-11-27。本番を確認した結果。原文): 「本番を確認したところ、File→Openをクリックするとネイティブファイル選択ウインドウが出現します。これは明らかに間違った挙動です。OPFSウインドウが出現するよう是正してください。」

FL がファイルを選ぶ時(Open・Import)、ページはブラウザのネイティブのファイル選択を出さず、OPFS のファイル管理の窓(ページ側の窓。FL の画面の上に出る)を出す。窓で選んだ答えを、FL へ返す。保存先を決める時(Save・Export)も、同じ窓で名前を決める。

## 背景

- 読み(`pm-work/findings/fb-2026-11-27.md` の 1 節): 本番でネイティブの窓が出る直接の原因は `app/src/ui/filedlg.ts` の `openChooser` の `input.click()`。ページが非表示の `<input type="file" id="filedlg-input">` を作って、それを開く。
- この経路は #1481(`c409b2db`)で入った。その前の版は `FileDialogView` に `select` の hook があり、`app/src/files/engine-dialog.ts` の `EngineFiles.select` が OPFS の窓を出していた。今は `EngineFiles` と `mountFiles`(`app/src/files/mount.ts`)がどこからも呼ばれない。OPFS の窓のコードは残っていて、配線だけが無い。
- E2E は、ネイティブの選択を `Page.setInterceptFileChooserDialog` で止め、`DOM.setFileInputFiles` で `#filedlg-input` に直接ファイルを渡す(`tools/e2e/lib/flow.mjs` の `openProjectFile`、`tools/e2e/lib/file.mjs` の `openDataFile`)。そのため、この欠陥が E2E で見えなかった。E2E のどこにも `filechooser` のイベントを見る判定は無い。
- Save・Export は今、窓も選択も無い。`app/src/ui/filedlg.ts` の `saveNow` が推定の名前で答え、`SavedFilesView` がブラウザのダウンロードに渡す。この Issue では、保存先の名前を窓で決める形にする。

## 読む物

- `pm-work/findings/fb-2026-11-27.md`(1 節と 5 節)
- `app/src/ui/filedlg.ts`(`FileDialogView`: `show`・`buildOpen`・`openChooser`・`chosen`・`saveNow`・`settle`)
- `app/src/files/engine-dialog.ts`(`EngineFiles.select`・`answer`・`pendingSaves`・保存の写し戻し)・`app/src/files/select.ts`・`app/src/files/explorer.ts`(アップロードの入口は 109 行付近の `upload-files`)・`app/src/files/mount.ts`・`app/src/files/opfs.ts`
- `app/src/engine/filedlg.ts`(`FileDlgRequest`・`FileDlgAnswer`・`planOpenFile`・`planSave`)
- `app/src/main.ts`(`fileDlgView` の配線、`fileDlg` の `requests`・`history`)
- `git show c409b2db^:app/src/ui/filedlg.ts` と `git show c409b2db^:app/src/main.ts`(#1481 の前の配線。`select` の hook の書き方)
- `tools/e2e/lib/flow.mjs`・`tools/e2e/lib/file.mjs`・`tools/e2e/scenarios/file/open-box-shows.mjs`・`tools/e2e/scenarios/smoke/file-open-demo.mjs`
- `lift/runtime/fle_filedlg.c`(`browser_picker`。答えの行の形 `OK`・`TYPE`・`R`・`E`・`S`・`D`)・`lift/harness/web/filedlg/fle_filedlg_lib.js`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。`git status` で作業ツリーを確かめる。
1. 赤を先に確かめる。E2E のシナリオ(下の 7)を先に書き、今の本番で `file/open-shows-opfs-window` が赤になることを確かめる(`filechooser` のイベントが 1 件以上。OPFS の窓が無い)。
2. 窓の経路を戻す: `FileDialogView` に `select` の hook を足す(`c409b2db^` の形を参考にする)。`Show` の要求(Open・Import)で hook があれば、`input.click()` を呼ばず、`EngineFiles.select` で OPFS の窓を出す。`app/src/main.ts` で `EngineFiles` を作り、`select: engineFiles.select` を渡す。Save(`request.save`)も hook があれば、窓で保存先の名前を決める。
3. `EngineFiles.select` の答えを、既存の `FileDlgAnswer` の形(`R`・`E`・`S`・`D` の行。`TYPE` の行)に合わせる。Open の答えは OPFS から読んだ `File` で `items` を作り、Worker へ渡す(既存の `client.answerFileDlg`)。
4. `tools/e2e/lib/flow.mjs` の `openProjectFile` と `tools/e2e/lib/file.mjs` の `openDataFile` を直す。`Page.setInterceptFileChooserDialog` で選択を止めて `#filedlg-input` に渡す経路を、OPFS の窓のアップロード(`#fx-input-files`)に変える。ネイティブの選択を迂回しないこと。
5. 窓の中の「Upload files」は、ネイティブの選択を出してよい。FL の Open の押下から、ネイティブの選択を直接出さない。
6. 製品の試験: `app/` の単体試験で、Open の要求に hook があれば `input.click` が呼ばれないことと、hook が無い時の既存の挙動が変わらないことを確かめる。hook は引数として渡す。製品に、テストのための分岐を入れない。
7. E2E のシナリオを `tools/e2e/scenarios/file/` に足す(1 本 1 ファイル。`covers` に機能の ID を付ける):
   - `file/open-shows-opfs-window`: File→Open(Ctrl+O でもよい)を押し、`filechooser` のイベントが 0 件で、OPFS の窓(`#fx-input-files` を含む窓)が出ていること。判定の notes に、窓の class と `filechooser` の件数を書く。DPR 1 と DPR 2 で動く。
   - `file/open-demo-via-opfs`: OPFS の窓で、`Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp` を窓へ入れ、選んで Open する。曲の題名が変わり、engine が止まらないこと。
   - `file/save-shows-opfs-window`: Save As で OPFS の窓が出て、名前を決めて保存すると、OPFS の選んだ場所にファイルが書かれること(`window.__app.saved` と OPFS の中身で確かめる)。
   - 既存の `file/open-box-shows` と `smoke/file-open-demo` は、窓の経路で通るように直す。
8. 報告する(下の「報告の規則」)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --only file/open-shows-opfs-window,file/open-demo-via-opfs,file/save-shows-opfs-window,file/open-box-shows --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e-1.jsonl" && node tools/e2e/run.mjs --only file/open-shows-opfs-window --displays 4k-dpr2 --budget 20m > "$RUNNER_TEMP/e2e-2.jsonl" && node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1 --budget 20m > "$RUNNER_TEMP/e2e-3.jsonl" && node -e 'const fs=require("fs");const L=process.argv.slice(1).flatMap((f)=>fs.readFileSync(f,"utf8").trim().split("\n")).map((l)=>JSON.parse(l));process.exit(L.length===3&&L.every((r)=>r.error===null&&r.scenarios.length>0&&r.scenarios.every((s)=>s.ok===true&&s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e-1.jsonl" "$RUNNER_TEMP/e2e-2.jsonl" "$RUNNER_TEMP/e2e-3.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る(手順 6 の試験を含む)。
- E2E の 3 本の行(`e2e-1`・`e2e-2`・`e2e-3`)が、全て `ok` で、`skipped` が無い。`file/open-shows-opfs-window` の notes に `filechooser` の件数 0 が出る。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の `file/open-shows-opfs-window` の赤の出力の行(`filechooser` の件数)、(b) 配線の差分の要点(`select` の hook・`EngineFiles` の作成の場所)、(c) E2E の 3 行の `ok` の値と、`file/open-shows-opfs-window` の notes の行、(d) 流れ A〜D の 4 行、(e) `npm test --prefix app` の最後の行、(f) `bash guard/scan.sh` の結果、(g) Save・Export の扱いの変更(窓で名前を決めるか、推定名のままか)と理由。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
- #{{235}}(File メニューの「Manage files」)は、この Issue の OPFS の窓を使う。ただし、この Issue が未完でも、既存のファイル管理の窓を出せば進められる。
