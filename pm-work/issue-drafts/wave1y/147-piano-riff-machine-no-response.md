---
draft: "147"
title: "Piano roll のリフ マシーン(Riff machine)が、起動しようとしても何も反応しない(窓が出ない)を直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["157"]
labels: ["resources", "verify"]
size: M
priority: High
project: signature-studio/3
---

## 目的

依頼者の実機で、Piano roll の Tools メニューの「リフ マシーン(Riff machine)」を起動しようとしても、何も反応しない(窓が出ない)。この Issue の目標は次の 2 つ。
1. Piano roll → Tools → Riff machine の操作で、Riff machine の窓が現れ、画面に描かれ、窓の主ボタン(画面の右下の Accept。窓の表に文字は出ない)を押すと、Piano roll にノートが作られること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

## 背景

- 依頼者の報告(原文): 「リフ マシーンは起動しようとしても何も反応しません。」(依頼者の症状は「何も反応しない」。crash ではない)。
- 事実 1(`lift/runtime/fle_wall_ia2.c` の `fle_imp_MessageBoxW` の注。見出しは「wall ia2-W28: Tools > Riff machine」。`lift/harness/seeds/ia2-imports.txt` の 126 行にも同じ記録): Tools > Riff machine は、メッセージボックス(`MessageBoxW`)を出す。エンジンは、窓を出さず、既定のボタンを選んだものとして答える。その時、標準エラーに 1 行(`fle: MessageBoxW NOT SHOWN ...`)を書く。
- 事実 2(同じ注): 既定の答えを選ぶ仕組みは、記録の上で「根拠の無い選択」と明記されている。
- 事実 3(配信の一覧 `inputs/fl-installer-files.tsv`・配信の接頭辞 `ci/resources-prefixes.txt`): Riff machine の資料は次のとおり。
  - `Data\System\Riff machine\` 8 件(17,346 B)。配信の接頭辞 `Data\System\` の下に在る(接頭辞は足さない)。
  - `Data\Patches\Scores\Arpeggiator\Riff machine\` 40 件(8,093 B)。`Data\Patches\Scores\Quantization\Riff machine\` 9 件(3,908 B)。`Data\Patches\Scores\Chopping\Riff machine step 1\` 24 件(6,311 B)と `step 2\` 19 件(2,807 B)。この 4 つの接頭辞は、main の fc686522 で足した済み。
- 事実 4: 配信の一覧に、名前に riff を含む DLL・EXE は 0 件。Riff machine の 8 本のプリセット(`Data\System\Riff machine\*.fst`)は、すべて内蔵の音源 Sytrus を使う。Sytrus の読み込みは、#1400 の作業で直す(この Issue は、#1400 が閉じてから起動する)。

## 前回までの試行で分かったこと

(run 45394680999。job「作業 #1387」。開始 12:35 UTC、engine の作り直し 約 35 分。判定は「差し戻し」。push していない。runner の値。nproc 4、loadavg は JSON の各行)

1. 直す前(HEAD の seed 表)・DPR 1: `MessageBoxW` の「Some required files are missing.」(caption "Error")。窓は無し。`missing-path: FindFirstFileW 3 "<FL のインストール先>\Data\System\Riff machine\*.*"`。
2. seed 表に、Riff machine の行を 9 行足した(フォルダ 1 件と、ファイル 8 件・17,346 B。起動の前に取る)。結果: `MessageBoxW` の行は消え、`enum-dir: 8 ...Riff machine\*.*`(列挙は成功)。`preBootBytes` は 311,111,837 B → 311,132,462 B(+20,625 B)。
3. 次の壁は Sytrus(#1400)。Riff machine の 8 本のプリセットは Sytrus を使う。Sytrus の DLL は seed 表に無く、`LoadLibraryW 126 ""` が失敗して、非表示の警告(`TMsgForm`「Warning」)が出る。本文(ゲストのメモリの UTF-16LE から読んだ): 「The plugin Sytrus couldn't be found or opened. If it has to be unlocked, please make sure that you unlocked it correctly.」と「Tools > Manage cloud plugins. It's free!」。警告は閉じるまで Riff の窓を作らない。
4. 警告を閉じると(道具が Enter を送る。計測の段で、製品の動きではない)、窓「Piano roll - riff machine」(`TPRScoreCreatorForm`)が描かれる(DPR 1 と DPR 2.65 の両方で `visible` と `painted` が true)。窓の主ボタンは、画面の右下の「Accept」(PNG で確かめた。窓の表の文字には出ない)。
5. Accept を押すと、窓は閉じ、Piano roll(「Piano roll - Riff Machine - Squelch」)に F4 付近の音符の帯が画面に出た(画素の読みによる。数えていない)。
6. 残る `missingPaths`: (a) `Data\System\Riff machine\Basic 808 *.wav` の探索(4 件の見本は、配信では `Data\Patches\Packs\Legacy\Drums\Dance\` の下に在る。FL も同じ場所を探すため、見つからないのは正常な応答の可能性がある。推定)。(b) Scores の 4 フォルダ(`Data\Patches\Scores\...\Riff machine\*.*`)の列挙の失敗。seed 表に行が無いため。
7. `notes`(ノートの数)は数えていない。保存で数える経路を試した: Ctrl+S では、保存のダイアログも保存の記録も出ない(タイトルに Trial。試用版では保存が無効と見られる。推定)。画面の画素の変化だけでは、ノートの数の判定には使えない。
8. 道具の不具合を直した(前回の差分に入っている): `windowNew` を Accept の後の画面で取っていた(押す前に取るよう直した)。最上位の窓の `visible` は、表に vis の項目が無いため常に false だった(表に在れば visible とした)。
9. 起動の流れ A〜D: 4 行とも合格(`engine-idle`・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D で `welcomeClosed` true)。`elapsedMs` の中央値 11,538.5 ms。`preBootBytes` の最大 311,132,462 B。
10. `npm test --prefix app` は合格(1374 passed)。`plans_match_recorded_texts` の engine の記録の件は、d14ed548 で直した。
11. 前回の差分には、OTT の扱い(`lift/crates/fe-build/src/main.rs`・`pack.args`・`bundle.txt`)も入っている。main の 4d73666a が同じ問題を別の版で直した。取り込みで `main.rs` が衝突する。**main の版を採る**(OTT の扱いは 4d73666a が正)。
12. 前回の記録(`pm-work/findings/riff-machine-no-response.md`)は、前回の差分として取り込まれる。作業者は同じパスのファイルを作らない(衝突する)。
13. 判定: 差し戻し。理由は、上の 3(Sytrus)と 6(`missingPaths`)と 7(`notes` の数え方が無い)。

## 数え方(PM の決定)

- `notes`(ノートの数): Piano roll の窓の矩形(窓の表から取る)の中で、Accept の後に現れた、ノートの色の連結成分の数。
- ノートの色: 実測で決める(Accept の前の画面と後の画面を比べ、後にだけ現れた色の最頻値)。定数・表として埋め込まない。
- 連結成分: 後にだけ現れた、その色の画素の 4 方向の連結成分のうち、幅 4 px 以上・高さ 4 px 以上の物。
- JSON: `notes`(数)と `noteRects`(各連結成分の x・y・幅・高さの配列)。画素の変化の総数では判定しない。
- 判定の式は、合成データで確かめる(連結成分が 1 以上は通り、0 は落ちる)。報告に、その確かめの出力を書く。
- #1386(コード進行ツール)も、この数え方を使う。先に入った方が `tools/piano-roll-tools/measure.mjs` に実装し、もう一方は同じ関数を呼ぶ。衝突したら両方を残し、報告に書く。

## 先に行う(PM の作業は済み)

- 配信の接頭辞は main に在る(Riff の 4 つ・`Data\System\`)。`ci/` の変更は、この Issue の作業者は行わない。
- #1400(内蔵の音源 Sytrus)が閉じてから起動する(blocked by)。Sytrus が読めると、警告は出なくなり、道具の「警告を閉じる段」は動かなくなる。
- #1386 への依存は外した(この Issue は、道具 `tools/piano-roll-tools/measure.mjs` だけを使う)。

## 読む物

- `tools/piano-roll-tools/measure.mjs`(道具。`--tool riff-machine`・`--log`・`--shot`・窓の表)
- `pm-work/findings/riff-machine-no-response.md`(前回の記録。前回の差分から入る)
- `lift/runtime/fle_wall_ia2.c`(`fle_imp_MessageBoxW` と注)・`lift/harness/seeds/ia2-imports.txt`(W28 の行)
- `inputs/fl-installer-files.tsv`(`Data\Patches\Scores\...\Riff machine\` の行の guest・bytes・sha256)
- `lift/harness/seeds/vfs-seed-idle1.tsv`(既存の行の書式。フォルダの行は `-` と `-` の 2 列の行、実物の行は host を `resources/` で始める)

## 実物の置き場

- 作業の先頭で `mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/` を行い、ビルドの入力の場所へ写す(受け入れの先頭にも在る)。
- 前回の差分で、`Data\System\Riff machine\` の 9 行(フォルダ 1 件と、ファイル 8 件)は seed 表に入る。作業者は、それを確かめる(二重に足さない)。
- 足りない物: Scores の 4 フォルダの行(フォルダの行 4 件と、ファイルの行 92 件・合計 21,119 B。`inputs/fl-installer-files.tsv` の値)。起動の前に取る(`@lazy-file:` にしない)。

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。前回の差分の衝突は、`lift/crates/fe-build/src/main.rs` は main の版を採る(上の 11)。
2. Scores の 4 フォルダを、seed 表に足す(上の「実物の置き場」)。`preBootBytes` が上限(314,368,711 B)を超えないことを確かめる。
3. 窓の確認: Tools > Riff machine を操作し、警告が出ない(Sytrus が読める)ことと、窓が現れて描かれることを、道具で確かめる。警告が出る時だけ、道具の警告の段が動く(受け入れでは警告の数が 0 件であること)。
4. 窓の主ボタン(Accept)の位置は、新しい窓の矩形から画面の読みで取る(座標を固定の数で書かない)。押す前に、窓の表を取る。
5. `notes` を、上の「数え方」で数える。JSON に `notes` と `noteRects` を出す。合成データで判定の式を確かめる。
6. 測る: DPR 1 と乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。
7. 受け入れを通す。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool riff-machine --dprs 1,$D > "$RUNNER_TEMP/riff.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&!!r.windowNew&&r.windowNew.some((w)=>w.visible===true&&w.painted===true)&&r.notes>=1&&!r.stopLines.some((l)=>l.includes("MessageBoxW"))&&!r.missingPaths.some((l)=>l.includes("Riff machine\\*.*"))&&(r.warnings||[]).length===0&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/riff.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上。`windowNew` に、`visible` と `painted` が true の窓が 1 つ以上(窓の矩形の画素が一色でない。窓の矩形で最も多い色の画素が 95% 未満)。
- `stopLines` に `MessageBoxW` の行が 0 件。
- `missingPaths` に、フォルダの列挙(`Riff machine\*.*`)の行が 0 件。個別のファイルの探索(`Basic 808 *.wav` など)の不一致は、判定に入れない(報告には、その行の一覧を書く)。
- `warnings`(道具が記録する、閉じずに残った警告の文字の配列)が空。
- `notes` が 1 以上(上の「数え方」)。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行)。判定は #1386 と同じ(各行の `error`・`state`・`standIn`・`trapped`・`hostErrors`・`opened`、流れ B と D の `welcomeClosed`、4 流れの `elapsedMs` の中央値、`preBootBytes` の最大)。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 窓の表の差分(新しい窓の有無と矩形)、警告の有無、`missingPaths` の行の一覧、Accept の後の `notes` と `noteRects`、数え方の合成データの確かめの出力、直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値、seed 表に足した行の数と合計の大きさ、取り込みの衝突の解消の内容。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。`ci/` は作業者が変えない。
- ベンダー名と Cloudflare の識別子を書かない。ぼやけた・拡大した画面を出さない。特定の拡大率や DPI の値を、定数・表として埋め込まない(DPR は、道具の引数で渡す値と、乱数の値だけ)。
- OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- 警告を隠す・既定の答えを返す、などで窓を出すように見せかけない。Sytrus の読み込みが成功した上で、窓が出る形にする(#1400 の範囲)。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #1400(内蔵の音源 Sytrus。Riff machine のプリセットが使う)。
- #1386(コード進行ツールの段階 2)とは、道具 `tools/piano-roll-tools/measure.mjs` の `notes` の数え方を共有する(上の「数え方」)。
- `ci/resources-prefixes.txt` の変更は PM が行う(この Issue の作業者は変えない)。
