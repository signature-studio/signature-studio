---
draft: "244"
title: "E2E の file: テンプレートを開いた後の判定が 37 本落ちる(タイトルが読めない・Problems の箱が残る疑い)と、Ctrl+O の箱を前提にした本"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: []
labels: ["verify"]
size: M
priority: Medium
project: signature-studio/3
---

## 目的

`file`(115 本)の E2E で、印の無い失敗が 52 件ある。そのうち 37 件は、テンプレートを開く本(`file/template-*`)で、同じ判定の形(タイトルが `null`・エンジンは `engine-idle`・曲の名前がログに無い)で落ちている。残りは、Ctrl+O の箱を前提にした本(#1519 で File → Open が OPFS の窓に変わった後も、箱を待つ)と、閉じた Issue の印を付けた本(`file/z-persist-recent-list`)。この Issue は、テンプレートの判定を、読める事実で書き直し、箱の前提を今の製品に合わせ、閉じた印を直す。

## 背景

- 全シナリオの E2E(`e2e-full` の最新の run。commit 24aeee99 の版)の結果(実測):
  - テンプレート 37 本: `file/template-advanced-electronic-breakbeat`・`file/template-empty-empty`・`file/template-utility-vocoder-vocoder` など。notes は `title "null" (before "...")`・`state engine-idle`・`the file's name in the engine log false`。曲は開いたように見えるが、判定は偽。
  - 推定: テンプレートを開くと、`Problems loading the project`(`TMissingPluginsForm`)の箱が出て、主の窓が窓の表の上に出ない。そのため `openDataFile` の `title` が `null` になり、`boxLeft` が真になって `opened` が偽になる。結果の JSON には `boxLeft` が残らないので、確認できない。
  - 箱の前提: `file/new-after-demo`(`no open box (#filedlg-input or #fx-input-files) 40 s after Ctrl+O`)・`file/open-box-shows`(`box null shown by Ctrl+O`)・`file/open-upload-multiple`(`2 files given (box null, given false)`)・`file/import-midi`(`box null`)は、ネイティブの箱を待つ。#1519 が閉じている(File → Open を OPFS のファイル管理の窓にする。run の commit の後に入った)ので、この 4 本の前提は、今の製品と合わないと見ている(推定。今の製品での結果は、確かめていない)。
  - 閉じた印: `file/z-persist-recent-list` は印 `#1409`(閉じている)。この run では、再読込の後にエンジンが止まった(状態 `error`)。`#1502`(閉じている。Recent を再読込をまたいで保つ)の後で、印の前提(`#1409`: 書き込み先が見つからない)は古い可能性がある。
  - 同じ仕組みの `drop-downs shown 0` の 5 本(`file/export-submenu` など)は、`menus` の Issue で扱う。
- 確認できない: テンプレートの Problems の箱の有無(結果に記録が無い)。今の製品で、Ctrl+O の箱が出るか(この run の後に製品が変わった)。

## 読む物

- `tools/e2e/lib/file.mjs`(`openDataFile`・`waitOpened`・`mainTitle`・`clearMissingPlugins`・`clearProblems` の使い方)
- `tools/e2e/lib/flow.mjs`(`openProjectFile`・`clearProblems`)
- `tools/e2e/scenarios/file/template-*.mjs`(宣言・`covers`)、`file/new-after-demo.mjs`・`file/open-box-shows.mjs`・`file/open-upload-multiple.mjs`・`file/import-midi.mjs`・`file/z-persist-recent-list.mjs`
- 製品: File → Open の経路(#1519 の変更。読むだけ)、テンプレートを開く経路(読むだけ)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. テンプレートの判定を確かめる: `openDataFile` の結果に、`boxLeft`・Problems の箱の名前・主の窓の有無を書く。`node tools/e2e/run.mjs --only file/template-empty-empty --displays 4k-dpr1 --budget 10m` を走らせて、結果の JSON で、Problems の箱が残っているかを確かめる。
2. 箱が残る時: 箱を閉じてから主の窓の題名を読む(閉じ方は `clearProblems` の既存の手順を使う)。閉じても題名が読めない時は、窓の表の主の窓(`TFruityLoopsMainForm`)の有無を報告に書く。
3. 箱の前提: 上の 4 本の判定を、今の File → Open(OPFS の窓)に合わせて書き直す。箱が出ないことを前提にするのか、OPFS の窓を前提にするのかを、製品の動きで決め、報告に書く。判定の基準(開いたこと・曲の名前)は変えない。
4. 閉じた印 `#1409`: 今の製品での結果を確かめ、印を外すか、開いている Issue の番号に変える。再読込の後にエンジンが止まる場合は、止まりを `new` の印(停止の宣言つき)として、報告に書く。
5. 受け入れを通す。通らない時は、落ちた ID・表示・出力の 1 行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && (cd tools/e2e && npm ci && npx playwright install chromium) && node --test tools/e2e/test/*.test.mjs && node tools/e2e/run.mjs --only file/template-empty-empty,file/template-advanced-electronic-breakbeat,file/template-utility-vocoder-vocoder,file/template-performance-4x4-4x4,file/open-box-shows,file/new-after-demo,file/open-upload-multiple,file/z-persist-recent-list --displays 4k-dpr1 --budget 40m --report-only > "$RUNNER_TEMP/file.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const S=L.flatMap((r)=>r.scenarios);const bad=S.filter((s)=>s.skipped===true||s.error!==null||(s.expectFail===null?s.ok!==true:s.engineStops>0));process.exit(L.every((r)=>r.error===null)&&S.length===8&&bad.length===0?0:1)' "$RUNNER_TEMP/file.jsonl"
```

判定の基準:
- 受け入れの式が、終了コード 0 で終わる(8 本すべてが、印なしなら ok、印つきなら止まらない)。
- テンプレート 4 本の判定が、Problems の箱の有無と、主の窓の題名を結果に書いた上で、開いた事実で読める。
- 箱の前提の 4 本が、今の File → Open(OPFS の窓)に合う形で書かれている(報告に、各本の前提と、今の製品での結果の行)。
- 閉じた印 `#1409` が、印を外すか、開いている Issue の番号になっている。
- 報告に: (a) テンプレートの箱の有無の結果(実測の行)と、判定の直し方、(b) 箱の前提の 4 本の結果、(c) `#1409` の結果と扱い、(d) 8 本の出力の行。確認できない物は「確認できない」と書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には、runner の値であることと、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 確認できなかった物は「確認できない」と書く。
- 段ごとに、やること 0〜5 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。`.github/`・`spec/` は、直す物がある時だけ変える。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。文字は、ゲストのメモリの文字列・窓の表・画素のハッシュで読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&`・先頭の `timeout` は使わない。待つ時は条件で待つ。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。
- commit・PR に、AI が書いたことを示す行を付けない。commit の名義は、環境の設定のまま。
- GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- この作業の job(GitHub Actions の runner)で、受け入れの式の全てを実行する。ビルドと E2E は、この job の中で行う。
- ビルドや E2E の長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- ビルドは 1 回にする。tools だけを直した後は、同じ bundle(`lift/out-app/bundle/raw/fl-Oz`)で E2E を走らせて直す。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する(未 commit の変更が残ると、判定は失敗する)。`git add` は 1 つずつ、自分が変えたパスだけを指定する。

## 他のタスクとの関係

- #1519(File → Open の OPFS の窓)と #1521(File メニューの Manage files)は、この Issue の箱の前提に関わる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、直し、受け入れをもう一度通す。
- `drop-downs shown 0` の 5 本は、#{{241}} で扱う。
