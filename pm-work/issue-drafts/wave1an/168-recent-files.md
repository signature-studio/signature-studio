---
draft: "168"
title: "FL の Recent(最近開いたファイル)を、ページの再読込をまたいで保持する: FL が Recent をどこへ・いつ書くかを、実測で特定する"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["145"]
labels: ["resources", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の報告(2026-11-24。原文): 「ページを再読込すると Recent が消えるなど、一部のデータは保持されていないように見えます。」

この Issue の目標は 2 つ。
1. FL の Recent(最近開いたファイルの一覧。File メニューの Open recent・Welcome・Browser の Recent などに出る)が、ページの再読込の後も、開いた曲の名前で出ること。
2. そのために、FL が Recent を**いつ・どこに**書くかを、実測で特定すること。特定した書き込みは、既存の保存の経路(`/fl/state/hkcu.log` の journal)に載せる。

前提: #{{145}}(段階 1: 保存したプロジェクト・バックアップの持ち越しと、0 バイトの扱い)が main に入っていること。

## 前回の試行で分かったこと(#{{145}} の run 45499845007。実測。runner の値)

- FL は Recent を、開く・保存(Ctrl+S)・閉じる要求の後のどれでも、保存の記録(journal)に残さなかった。名前を含む記録は 0 件。
- `Settings\Browser\Recent files.scr` は、起動の時に探される(`GetFileAttributesW` が 2 を返す = 無い)。プロジェクトを開いた後も、120 秒待った後も、保存の後も作られない。`Settings\Browser` のフォルダは FL が起動の時に作るので、フォルダの無さは原因ではない(run 45458518648 で確認)。
- レジストリ: プロジェクトを開いた後の `RegSetValueExW` は、起動時の設定の 11 件だけ。ファイル名を含む値は無い(run 45458518648)。
- 閉じる要求(`pagehide` で出す SC_CLOSE)から engine の停止まで 258 ms。保存の無い要求では、その間の書き込みは 0(`bytesBefore` 10,211 → `bytesAfter` 10,211)。「FL は終了の時に Recent を書く」という推定は、この VM では否定された。
- Ctrl+N(New)の試しは、engine の行が出ず、結論が出なかった。Save As・Close project・Browser の Recent タブは、まだ試していない。
- インストーラの一覧(`inputs/fl-installer-files.tsv`)に、Recent の名前の行は 0 件。Settings の行は 1 件。
- 判定の欠陥(#{{145}} で外した): 名前が FL のゲストのメモリに在るかで見ると、保存したプロジェクトのファイルが戻った時にも true になる。Recent の判定は、Recent の表示の文字列で見る(下の「やること」の 4)。

## 背景

- 仮想ボリュームはメモリの上だけにある(`lift/runtime/fle_vfs.h`)。ページを再読込すると、ゲストのファイルは消える。
- 既存の保存の経路: レジストリの journal(`/fl/state/hkcu.log`。キーは値の名前。#{{142}})と、ファイルの保存(記録の op 5・6・7。パスを鍵とする。#{{145}})。起動の前に、`fle_reg_restore_saved`(seed の後)が戻す。
- 仮想のユーザーのフォルダ: `C:\Users\user`。`Settings\Browser\Recent files.scr` の木の場所(FL の設定の下のどこか)は、`GetFileAttributesW` の行で確かめる。
- 起動の記録: FL は起動のたびに `Support\Logs\Startup\Startup <日時>.log` を作る。
- 推測しない。下の「やること」の 2 の書き込みは、実測で埋める。

## 読む物

- `inputs/fl-installer-files.tsv`(FL のインストーラの全件。Settings の既定の物を見る)
- `lift/runtime/fle_vfs.c`・`lift/runtime/fle_vfs.h`(作成・書き込み・名前の変更・削除・列挙)、`lift/runtime/fle_reg.c`(`RegSetValueExW`。保存の記録の書き方)
- `app/src/engine/reg-journal.ts`・`app/src/engine/regstore.ts`(journal の形と読み戻し)、`app/src/engine/worker.ts`・`app/src/engine/fruity.ts`(ページの終了の要求。#{{145}} の段階 1 の形)
- `app/src/files/select.ts`(開く・保存の窓の橋)
- `tools/persist/measure.mjs`(files の段。#{{145}} の段階 1 の版を土台にする)、`tools/smoke/welcome-flow.mjs`、`tools/boot-check/boot-bundle.mjs`
- `tools/feat-c1-hidpi/measure-size.mjs` の `guestText`(ゲストのメモリの文字の走査。#{{117}} の方式)

## やること

0. 作業ツリーの確認。`.prior-work.md` が在れば最初に読む。#{{145}} の段階 1 が main に在ることを確かめる(`app/src/engine/worker.ts` の `pagehide` の終了の要求と、`lift/runtime/fle_vfs.c` の 0 バイトの扱い)。
1. **同じ起動の中で Recent が動くかを確かめる(①)**: 起動 → Welcome を閉じる → File > Open で demo の曲を開く → File メニューの Open recent(または Welcome・Browser の Recent の表示)の文字に、開いた曲の名前が出るかを、`guestText` で読む。OCR を使わない。
   - 出る: Recent は同じ起動の中で作られている。②で書く先を特定し、③の保持を作る。
   - 出ない: FL は同じ起動の中でも Recent を作っていない。`inputs/fl-installer-files.tsv` と、`GetFileAttributesW` の探索の行から、FL の設定の既定の物と `Recent files.scr` の形式を調べる。JSON の `recentInSession` を false にする。
2. **書く操作と場所の特定(②)**: 次の操作ごとに、FL が行った全ての書き込みを、engine のログ(`fle:` の行)から一覧にする。対象は、ファイルの作成・書き込み・名前の変更(`MoveFileExW`・`ReplaceFileW`・`SetFileInformationByHandle`・一時ファイルへ書いて名前を変える)・削除、レジストリの書き込み(`RegSetValueExW` の全て)、`SHAddToRecentDocs`、Windows の Recent フォルダ(`SHGetKnownFolderPath(FOLDERID_Recent)` と `.lnk` の作成)。
   操作: 開く(File > Open・Browser のダブルクリック・Welcome の項目)/ 保存(Save・Save as・Ctrl+S)/ 新規(New。Ctrl+N)/ プロジェクトを閉じる(Close)/ Browser の Recent タブを開く / FL の定期の保存(タイマー)/ 終了の要求。
   ログが足りなければ、`lift/runtime/fle_vfs.c`・`lift/runtime/fle_reg.c` に、常に動く軽い記録を足す(環境変数・`#ifdef`・URL パラメータ・ビルド変数は足さない)。
   結果は JSON の `recentOps`(操作ごとの書き込みの一覧。場所とパス)と、`recentWhere`(Recent が書かれた場所と時点。書かれなければ "none")に出す。
3. **保持(③)**: ②で特定した、Recent の一覧が入る書き込みを、既存の journal(`/fl/state/hkcu.log`。#{{142}}・#{{145}} の形)に載せ、再読込の後に、FL が Recent を読めるようにする。場所(パス)の定数を runtime に入れない(書かれた物を、書かれた木のまま保つ)。`recentInSession` が false の時は、保持を作らず、②の一覧で終える。
4. **判定(④)**: `tools/persist/measure.mjs` の files の段で、セッション A: 起動 → Welcome を閉じる → demo の曲を File > Open で開く → ②で特定した操作 → ページを離れる(`pagehide`)。セッション B(再読込): 起動 → Recent の表示を読む。`recentKept` は、Recent の表示(File メニューの Open recent の文字列、Welcome・Browser の Recent の表示の窓)に、開いた曲の名前が在る時だけ true にする。保存したプロジェクトのファイルの名前では true にならない定義にする(表示の窓の文字列から取る)。
5. **受け入れ**: 下の 1 行を通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/persist/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/persist.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const lang=L.find((r)=>"language" in r);const f=L.find((r)=>r.step==="files");process.exit(lang&&f&&lang.error===null&&lang.elapsedMs<=12500&&lang.preBootBytes<=314368711&&f.error===null&&(f.recentKept===true||(f.recentInSession===false&&Array.isArray(f.recentOps)))&&f.bytes<=104857600?0:1)' "$RUNNER_TEMP/persist.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 言語・起動の段(`step` が `files` ではない行): `error` が null、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- ファイルの段(`step` が `files` の行): `error` が null。次のどちらか。(a) `recentKept` が true(再読込の後、Recent の表示に開いた曲の名前が在る)。(b) `recentInSession` が false で、`recentOps`(②の一覧)が配列。`bytes` が 104,857,600 以下。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)が通る。単体テスト(`npm test --prefix app`)が通る。
- 言語は無作為に 1 つ選ばれる(7 言語)。選ばれた es の時は、#{{154}} の範囲で、再読込の後に engine が `missing-ilsvg` で止まることがある。その時は、受け入れの判定は落ち、選んだ言語を報告に書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: ①の結果(`recentInSession`)、②の一覧(操作ごとの書き込みの場所とパス)、③の保持の有無、④の `recentKept` と、その根拠の行。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。保存は既定の動作にする(オプトインを作らない)。
- ファイルの名前・場所の値を、runtime・app・seed・テストに、定数・表・分岐として入れない(FL が書いた物の木を、そのまま写す)。
- 開く・保存の窓の橋(`app/src/files/select.ts`)の動作を変えない。
- Python を製品に使わない(調査と検証にだけ使う)。OCR を使わない。
- ベンダー名と Cloudflare の識別子を書かない。sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 1 回の試行で、ビルドは数回まで。①の確認・②の記録・③の保持・④の道具は、まとめて 1 回のビルドで行う。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- #{{145}}(段階 1)の後に行う。`tools/persist/measure.mjs` と、ページを離れる時の終了の要求を共有する。
- #{{142}}(レジストリの保持)の journal の形を使う。
