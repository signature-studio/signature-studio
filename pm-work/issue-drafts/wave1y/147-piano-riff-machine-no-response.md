---
draft: "147"
title: "Piano roll のリフ マシーン(Riff machine)が、起動しようとしても何も反応しない(窓が出ない)を直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の実機で、Piano roll の Tools メニューの「リフ マシーン(Riff machine)」を起動しようとしても、何も反応しない(窓が出ない)。この Issue の目標は次の 2 つ。
1. Piano roll → Tools → Riff machine の操作で、Riff machine の窓が現れ、画面に描かれ、窓の主ボタン(Generate)を押すと、Piano roll にノートが作られること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

## 背景

- 依頼者の報告(原文): 「リフ マシーンは起動しようとしても何も反応しません。」(依頼者の症状は「何も反応しない」。crash ではない)。
- 事実 1(`lift/runtime/fle_wall_ia2.c` の `fle_imp_MessageBoxW` の注。見出しは「wall ia2-W28: Tools > Riff machine」。`lift/harness/seeds/ia2-imports.txt` の 126 行にも同じ記録): Tools > Riff machine は、メッセージボックス(`MessageBoxW`)を出す。エンジンは、窓を出さず、既定のボタンを選んだものとして答える。その時、標準エラーに 1 行(`fle: MessageBoxW NOT SHOWN ...`。種類・見出し・本文を含む)を書く。
- 事実 2(同じ注): 既定の答えを選ぶ仕組みは、記録の上で「根拠の無い選択」と明記されている。見出しと本文の文字は、記録と、前回の実測の行に在る(下の「前回までに分かったこと」)。
- 事実 3(配信の一覧 `inputs/fl-installer-files.tsv`・配信の接頭辞 `ci/resources-prefixes.txt`): Riff machine の資料は次のとおり。
  - `Data\System\Riff machine\` 8 件(17,346 B)。配信の接頭辞 `Data\System\` の下に在る。接頭辞は足さなくてよい(必要最小は 0 件)。
  - `Data\Patches\Scores\Arpeggiator\Riff machine\` 40 件(8,093 B)。`Data\Patches\Scores\Quantization\Riff machine\` 9 件(3,908 B)。`Data\Patches\Scores\Chopping\Riff machine step 1\` 24 件(6,311 B)と `step 2\` 19 件(2,807 B)。この 4 つの接頭辞は、main の fc686522 で足した済み。
- 事実 4: 配信の一覧に、名前に riff を含む DLL・EXE は 0 件。`inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`・`inputs/external-plugins.tsv` にも riff は 0 件。

## 前回までに分かったこと

(前回の試行は、146 の作業と同じ作業の中で、DPR 1 で道具 `tools/piano-roll-tools/measure.mjs` を回した。道具は main に入った: 05577cf9)

- 操作: Tools のメニューを開き、↓ を 4 回で "Riff machine" の項目に合わせ、Enter。(Chord progression は ↓ を 5 回。)
- 結果(DPR 1): `engine-idle`、`presentedDelta` 27。新しい窓は無し(`windowNew` が空)。停止は無い。
- 出た行(この 2 行が、見える症状の原因の手がかり):

```
fle: MessageBoxW NOT SHOWN, ... caption "Error", text "Some required files are missing."
missing-path: FindFirstFileW 3 "<FL のインストール先>\Data\System\Riff machine\*.*"
```

- 見立て(前回の実測から。推定を含む): FL は `Data\System\Riff machine\` を探すが、VFS に無い。seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)に、`Riff` の行が 0 件、このフォルダの行も 0 件だから。インストーラの一覧には 8 件が在り、接頭辞 `Data\System\` で取り出される(作業ツリーの `lift/out-app/host-extra/resources/` に置かれる)。よって、足りないのは seed 表の行。「何も反応しない」は、このメッセージボックスの既定の答えで、窓が出ないため。
- 道具の判定: `== stop: returned from entry` は正常な起動でも出る。道具の `stopped` は、この行を除いて判定する(05577cf9 で実装済み)。
- 接頭辞の検査: `spec/workflow.test.sh` に (r3) の検査を足した(920f7bc8)。`Data\System\Riff machine\` の 8 件が、接頭辞 `Data\System\` で覆われることを固定する。`ci/resources-prefixes.txt` は変えていない。

## 読む物

- `lift/runtime/fle_wall_ia2.c`(`fle_imp_MessageBoxW` と、その注)・`lift/harness/seeds/ia2-imports.txt`(W28 の行)
- `lift/runtime/fle_vfs.c`(`missing-path:` の行。FindFirstFileW の経路)・`lift/runtime/fle_win.c`(`fle_imp_CreateWindowExW`・`fle_imp_ShowWindow`)
- `inputs/fl-installer-files.tsv`(`Data\System\Riff machine\` の 8 件。guest・bytes・sha256 の列)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(既存の行の書式。フォルダの行は `-` と `-` の 2 列の行、実物の行は host を `resources/` で始める)
- `tools/piano-roll-tools/measure.mjs`(道具。Generate を押す段と、`notes` の数え方を足す)
- `pm-work/findings/chord-tool-crash.md`(前回の記録)

## 先に行う(PM の作業は済み)

- 配信の接頭辞は足りている(`Data\System\` が在る。Riff の 4 つの接頭辞は fc686522 で足した済み)。作業者は `ci/` を変えない。
- #1386(コード進行ツール)への依存は外した(依存を外しただけでは起動しないので、本文を編集して再評価させる)。この Issue は、道具 `tools/piano-roll-tools/measure.mjs` だけを使う。

## 実物の置き場

- ラベル `resources` の extract が取り出し、作業ツリーの `lift/out-app/host-extra/resources/` に置く(`Data\System\Riff machine\` の 8 件を含む)。
- 受け入れの先頭で `mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/` を行い、ビルドの入力の場所へ写す(`fe-build bundle` の `pack-seed.mjs` は `lift/out-app/host/resources/` を読む)。
- 起動の seed 表に、次の行を足す(作業者が足す。`ci/` は変えない): フォルダの行 1 件(`Data\System\Riff machine\` の guest と、host `-`・sha256 `-`)と、ファイルの行 8 件(guest は `Data\System\Riff machine\<名前>`、host は `resources/Data/System/Riff machine/<名前>`、sha256 は一覧の値)。合計は 17,346 B で小さいので、起動の前に取る(`@lazy-file:` にしない)。

## やること

1. 再現と特定: 前回の実測で (i)(iii) は確かめた。残りは (ii) メッセージボックスの本文の全文と、(iv) 項目を選んだ後の画面の差分。道具で `--tool riff-machine` を DPR 1 で実行し、結果を `pm-work/findings/riff-machine-no-response.md` に書く(この commit に含める)。戻り番地は `lift/out-app/gen/wasm-func-names.tsv` で関数名を引く。
2. 実物の行を seed 表に足す(上の「実物の置き場」)。`preBootBytes` が上限(314,368,711 B)を超えないことを確かめる。
3. 窓の確認: 項目を選んだ後に、新しい最上位の窓が現れて描かれるかを、道具で確かめる。現れない場合は、原因を、実測の行で特定する。候補: (a) 欠けた資料の行が残る(seed の行の不足)。(b) 既定の答えの仕組み(`fle_imp_MessageBoxW`)が、窓を出す前に別の処理を止める。(c) 窓は作られたが、見えない(表示の経路)。
4. 既定の答えの仕組み(`fle_imp_MessageBoxW`)は、ほかの窓にも効く。仕組みを変えるなら、変更の範囲(どの窓の挙動が変わるか)を報告に書く。仕組みを変えずに直せるなら、そちらを選ぶ。
5. 道具 `--tool riff-machine` に、窓の主ボタン(Generate)を押す段を足す。ボタンの位置は、新しい窓の矩形から画面の読みで取る(座標を固定の数で書かない)。押してから 3 秒待つ。
6. `notes`(Piano roll のノートの数)を、道具で数える。ゲストのメモリの走査か、エンジンの記録で。画面の画素の変化だけでは足りない。#1394(コード進行ツールの段階 2)と同じ数え方を使う。先に実装された方を使い、もう一方は同じ関数を呼ぶ。衝突したら両方を残し、報告に書く。
7. 測る: 道具で、DPR 1 と、乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool riff-machine --dprs 1,$D > "$RUNNER_TEMP/riff.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&!!r.windowNew&&r.windowNew.some((w)=>w.visible===true&&w.painted===true)&&r.notes>=1&&!r.stopLines.some((l)=>l.includes("MessageBoxW"))&&!r.missingPaths.some((l)=>l.includes("Riff machine"))&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/riff.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上。`windowNew` に、`visible` と `painted` が true の窓が 1 つ以上(窓の矩形の画素が一色でない。窓の矩形で最も多い色の画素が 95% 未満)。`stopLines` に `MessageBoxW` の行が 0 件。`missingPaths` に `Riff machine` を含む行が 0 件。Generate を押した後に `notes` が 1 以上(Piano roll にノートが作られた)。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行)。判定は #1386 と同じ(各行の `error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`、流れ B と D の `welcomeClosed`、4 流れの `elapsedMs` の中央値、`preBootBytes` の最大)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: メッセージボックスの本文の全文、窓の表の差分(新しい窓の有無と矩形)、原因の特定(実測の行)、直した方式と理由、seed 表に足した行の数と合計の大きさ、直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値、`notes` の数え方。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。`ci/` は作業者が変えない。
- ベンダー名と Cloudflare の識別子を書かない。ぼやけた・拡大した画面を出さない。175%・DPI 168/169・DPR 1.7647 のような値を、定数・表として埋め込まない。
- OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし(#1386 への依存は、PM が外した)。
- #1394(コード進行ツールの段階 2)とは、道具 `tools/piano-roll-tools/measure.mjs` の `notes` の数え方を共有する。
- `ci/resources-prefixes.txt` の変更は PM が行う(この Issue の作業者は変えない)。
