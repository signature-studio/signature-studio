---
draft: "151"
title: "コード進行ツールの Start creating で、Piano roll にノートが作られる(FL の Python のスクリプトの実行: flpianoroll と標準ライブラリ)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["146"]
labels: ["resources", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

段階 2。#{{146}}(段階 1)で、FL 内蔵の Python の実行環境(`Python\Python312.dll`)が読み込め、engine が Start creating で止まらなくなった後の段階。依頼者の実機で、Piano roll の Tools メニューの「コード進行ツール」を開き、"Start creating" を押すと、コード進行ツールの Python のスクリプトが走り、Piano roll にノートが作られること。この Issue の目標は次の 2 つ。
1. Start creating の押下で、engine が止まらず、Piano roll にノートが 1 以上作られること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

## 背景

- 段階 1 の結果(#{{146}}): `Shared\Python\python312.dll` を real module として読み込み、`LoadLibraryW "Python\Python312.dll"` が成功する。engine は、Start creating で止まらない。ノートは、まだ作られない。
- 依頼者の報告(原文): 「コード進行ツールは "Start creating" を押下した瞬間クラッシュします」。
- 事実 1(配信の一覧 `inputs/fl-installer-files.tsv`): コード進行ツールの実体は、`System\Tools\Chord progression\` の 59 件の FL 内蔵の Python のスクリプト(`cpt.pyscript`・`stack.pyscript`・Python の bytecode の `.pyc` など)。大きさは、この一覧に無い。
- 事実 2(同じ一覧): Python の標準ライブラリは `Shared\Python\python312.zip`、起動の設定は `Shared\Python\python312._pth`、拡張モジュールは `Shared\Python\DLLs\` の `_*.pyd`。
- 事実 3(配信の範囲): 段階 1 の時点で、`Shared\Python\`(37 件)は配信の接頭辞に在り、作業ツリーの `lift/out-app/host-extra/resources/` に置かれる。`System\Tools\Chord progression\`(59 件)も同じ接頭辞の下に在る。ただし、起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、これらの行が無い。段階 1 の lazy の上限(`app/src/bundle/manifest.ts` の maxItems 128)は、`Shared\Python\` の 37 件で 112 件になる。`System\Tools\Chord progression\` の 59 件を足すと、上限を超える。
- 事実 4(FL の機能): FL の本体は、Python に、FL 自身の C のモジュール(`flpianoroll` など)を登録する。コード進行ツールのスクリプトは、そのモジュールで Piano roll のノートを作るとみられる。この事実は未確認で、この Issue の最初の作業で確かめる。
- 仮説(未確認): Start creating は、スクリプトを実行し、そのスクリプトが `flpianoroll` を使って Piano roll のノートを作る。段階 1 の後に、どこで止まるか(モジュールの不在、標準ライブラリの読み込み、スクリプトの探索)は、道具の行で確かめる。

## 読む物

- `inputs/fl-installer-files.tsv`(`System\Tools\Chord progression\`・`Shared\Python\`)
- `pm-work/findings/chord-tool-crash.md`・`pm-work/findings/python312-imports.md`(段階 1 の記録)
- `tools/piano-roll-tools/measure.mjs`(道具。段階 1 で `pythonLoaded` を足し、この Issue で `notes` を足す)
- `lift/runtime/fle_realmod.c`(real module の探索)・`lift/runtime/fle_load.c`(`LoadLibraryW`)
- `app/src/bundle/manifest.ts`(lazy の上限)・`app/src/bundle/lazy.ts`(`@lazy-file:` の仕組み)
- `tools/smoke/welcome-flow.mjs`・`tools/boot-check/boot-bundle.mjs`(`bootBundle` と `afterIdle`)

## 先に行う(PM の作業。この Issue の作業者は `ci/` を変えない)

- 段階 1(#{{146}})が main に入った後に、この Issue の作業を始める。段階 1 の resources の配置(`Shared\Python\` と `lift/plugins/python312/`)を、この Issue の土台にする。
- 作業者は `ci/` を変えない。足りない接頭辞を見つけたら、報告に書いて終える。

## やること

1. Start creating が呼ぶ Python のスクリプトを、実測で特定する。道具 `tools/piano-roll-tools/measure.mjs` の `stopLines` と `missingPaths` と、ファイルを開く行(`open` の行)から、どの `System\Tools\Chord progression\` のスクリプトを読むかを確かめる。確かめたことと推定を分けて書く。
2. Python の初期化を通す: `python312._pth` の読み込み、`python312.zip`(標準ライブラリ)の読み込み、`DLLs\` の拡張モジュールの読み込み。標準ライブラリとスクリプトは、起動の後に取る(`@lazy-file:`)。lazy の上限(128 件)に入らない物は、報告に書いて終える(`app/` の変更と deploy が要るので、作業者は上限を変えない)。
3. FL の `flpianoroll` モジュールを、runtime で提供する(FL の本体が Python に登録する C のモジュールに当たる物。ノートの作成・読み取りの関数)。ほかの FL の内蔵モジュールと同じ方式で作る。
4. スクリプトを実行し、ノートを作らせる。Start creating の押下の後、3 秒待つ。
5. ノートの数(`notes`)を道具で数える: Piano roll のノートの数を、ゲストのメモリの走査か、エンジンの記録で数える。画面の画素の変化だけでは足りない。#{{147}}(Riff machine)も同じ数え方を使う。先に実装した方を使い、もう一方は同じ関数を呼ぶ。
6. 測る: 道具で、DPR 1 と乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool chord-progression --dprs 1,$D > "$RUNNER_TEMP/chord.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&r.pythonLoaded===true&&r.notes>=1&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/chord.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上、`pythonLoaded` が true、`notes` が 1 以上(Piano roll のノートの数。ゲストのメモリの走査か、エンジンの記録で数える)、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行): 各行が、`error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: Start creating が呼んだスクリプトの名前(実測の行)、`flpianoroll` の関数の一覧と件数、直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値、`notes` の数え方。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。FL に内蔵の Python のスクリプトを、ゲストとして動かすのは FL の機能なので、よい。`ci/` は作業者が変えない。
- ベンダー名と Cloudflare の識別子を書かない。ぼやけた・拡大した画面を出さない。175%・DPI 168/169・DPR 1.7647 のような値を、定数・表として埋め込まない。
- OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- この Issue は 1 回の試行で終わらない可能性が高い。次の試行は、その commit から続く(`.prior-work.md` を読む)。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{146}}(段階 1。Python の実行環境の読み込み)。#{{146}} が閉じるまで、この Issue は起動しない。
- #{{147}}(Riff machine)とは、道具 `tools/piano-roll-tools/measure.mjs` の `notes` の数え方を共有する。
- `ci/` の変更は PM が行う(この Issue の作業者は変えない)。
