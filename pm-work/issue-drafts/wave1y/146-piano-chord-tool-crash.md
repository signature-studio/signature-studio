---
draft: "146"
title: "Piano roll のコード進行ツールの Start creating を押すと engine が止まる(crash)を直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、Piano roll の Tools メニューの「コード進行ツール」を開き、"Start creating" を押した瞬間に、FL が落ちる(engine が止まる)。この Issue の目標は次の 2 つ。
1. Piano roll → Tools → コード進行ツール → Start creating の操作で、engine が止まらず、Piano roll にノートが作られること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

## 背景

- 依頼者の報告(原文): 「コード進行ツールは "Start creating" を押下した瞬間クラッシュします」。
- 事実 1(配信の一覧 `inputs/fl-installer-files.tsv`): コード進行ツールの実体は、FL 内蔵の Python のスクリプトに見える。`System\Tools\Chord progression\` に 59 件(`cpt.pyscript`・`stack.pyscript`・Python の bytecode の `.pyc` など)。大きさは、この一覧に無い(dll・exe 以外は空)。
- 事実 2(同じ一覧): Python の実行環境は `Shared\Python\`(37 件。`python312.dll` 7,003,928 B、`python312.zip`、`python312._pth`、`_*.pyd` など)。インストーラに在る。
- 事実 3(配信の範囲 `ci/resources-prefixes.txt`・`ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`): 配信の接頭辞に `System\Tools\` も `Shared\` も無い。ci/extract-dlls.tsv と起動の seed にも、`Shared\Python\` と `System\Tools\Chord progression\` の行は無い。一方、コード進行ツールのテンプレート `Data\Templates\Utility\Create a chord progression\` は配信済み(`Data\Templates\`)。
- 事実 4(読み込みの経路 `lift/runtime/fle_load.c` の 416 行付近・`lift/runtime/fle_wall_ia2.c` の `fle_ia2_path_absent`): 「Python\Python312.dll」の `LoadLibraryW` は、常に「見つからない(126)」と答える。この答えは、記録の上で選んだ choice(`fle_wall_ia2.c` の注)。その注は「インストーラに Python312.dll が無い」と書くが、事実 2 の一覧には `Shared\Python\python312.dll` が在る。食い違い。
- 事実 5(同じ行の記録 `pm-work/findings/blocks-samples-1366.md` の 4.3): Welcome の Default の行を開いた直後に、`[err] missing-path: LoadLibraryW 126 "Python\Python312.dll"` と `[err] fle: RaiseException: exception code 0xeedfade ...` が出て、engine が exit code 134 で止まった(測定の記録。疑う commit は未確認)。
- 仮説(未確認。この Issue の最初の作業で確かめる): コード進行ツールの Start creating は、Python の実行環境を読みに行き、126 で失敗し、処理されない例外(0xeedfade)で engine が止まる。根拠は事実 4 と 5。ほかの原因(窓の作成、未実装の API)の可能性も残す。

## 読む物

- `inputs/fl-installer-files.tsv`(`System\Tools\Chord progression\`・`Shared\Python\`・`Data\Templates\Utility\`)
- `ci/resources-prefixes.txt`・`ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`
- `lift/runtime/fle_load.c`(`LoadLibraryW` の読み込み。`fle_ia2_path_absent` の呼び出し。416 行付近)・`lift/runtime/fle_wall_ia2.c`(`fle_ia2_path_absent`・`fle_imp_MessageBoxW` と、その注)・`lift/runtime/fle_vfs.c`(`missing-path:` の行を出す場所。134 行付近)・`lift/runtime/fle_env.c`(`fle_stop`)・`lift/runtime/fle_proc.c`(`RaiseException`)
- `app/src/engine/fruity-log.ts`(`== stop:` の行の読み)
- `pm-work/findings/blocks-samples-1366.md`(4.3)・`pm-work/findings/missing-paths.md`
- `tools/feat-c6-popup-menus/measure.mjs`(窓の表・右クリック・画面の差分の書き方)・`tools/smoke/welcome-flow.mjs`(`Session`: 窓の表・クリック・Escape・流れの書き方)・`tools/p3-audio-probe/measure.mjs`(`openProject`: ファイルを開く手順)・`tools/boot-check/boot-bundle.mjs`(`bootBundle` と `afterIdle`)

## 先に行う(PM の作業。この Issue の作業者は `ci/` を変えない)

- 配信の接頭辞が足りない。`ci/resources-prefixes.txt` に次の 1 行を足すのは PM の作業: `System\Tools\Chord progression\`(59 件。大きさは一覧に無い)。Python の実行環境を届けるなら、もう 1 行: `Shared\Python\`(37 件。dll・exe の大きさの合計は 14,939,080 B。他の 27 件は一覧に大きさが無い)。
- 作業者は `ci/` を変えない。足りない実物を見つけたら(`missing-path:` の行で確かめる)、報告に書いて終える。

## やること

0. 道具 `tools/piano-roll-tools/measure.mjs`(新規。147 も使う)。
   - 起動は `tools/boot-check/boot-bundle.mjs` の `bootBundle` を使い、`afterIdle` で測る(`tools/feat-c6-popup-menus/measure.mjs` と同じ形)。
   - Piano roll を開く。開き方は実測で選ぶ(F7 か、チャンネルを開く。テンプレート `Data\Templates\Utility\Create a chord progression\Create a chord progression.flp` を開く方法も候補)。窓の位置は窓の表(`window.__app` の窓の一覧)から取る。Tools メニューの項目の位置は、画面の読みで決める。座標は固定の数で書かない。
   - 引数: `<bundle dir> --tool <chord-progression|riff-machine> --dprs <list>`。`--tool` は Tools メニューの項目を選ぶ(`chord-progression` は "Chord progression" の項目。項目名は実測で確かめる。`riff-machine` は "Riff machine")。`--dprs` は拡大率の一覧(1 と、乱数の値)。stdout に、拡大率ごとに 1 行の JSON(JSONL)。stderr に、操作の段ごとの説明。
   - `chord-progression`: Start creating を押し、3 秒待つ。
   - JSON の項目: `tool`・`dpr`・`error`(ページの error。無ければ null)・`engine`(その時の state)・`stopped`(state が stopped・exited・error のいずれか、または `== stop:` の行がログに在る時 true)・`presentedDelta`(3 秒の間の提示の回数の増分。ページが数える値を使う。無ければ、道具で数える方式を報告に書く)・`notes`(Piano roll のノートの数。ゲストのメモリの走査か、エンジンの記録で数える。画面の画素の変化だけでは足りない)・`missingPaths`(起動後の `missing-path:` の行)・`stopLines`(`== stop:` と `fle:` の行)・`windowNew`(147 で使う。新しく現れた最上位の窓の {cls, w, h, visible, painted}。無ければ null)・`elapsedMs`(navigation から最初の engine-idle まで)・`preBootBytes`(`assetsReport` の bytes の合計)・`nproc`・`loadavg`。
   - 製品の既定の動作は変えない。測定のための分岐を、製品に入れない。
1. 再現: 今の main で `--tool chord-progression` を実行し、停止の全文(`missingPaths`・`stopLines`・戻り番地を `lift/out-app/gen/wasm-func-names.tsv` で引いた関数名)を `pm-work/findings/chord-tool-crash.md` に記録する。この commit に含める。
2. 原因: 仮説を、ログの行で確かめる。確かめたことと推定を分けて書く。
3. 直す: 原因に合わせる。候補(実測で選ぶ):
   - (a) 配信: PM が接頭辞を足した後、FL 内蔵の Python の実行環境と、コード進行ツールのスクリプトを、起動の後の取得(`@lazy-file:` の仕組み。`app/src/bundle/lazy.ts`・`app/src/engine/fruity.ts`。#1314・#1342 の方式)で届ける。起動前に取る量は増やさない。
   - (b) 実行: 届けた `python312.dll` を、他の Shared の DLL と同じ経路で lift して動かす。そのとき、`Python\Python312.dll` を「見つからない」と答える choice(`fle_wall_ia2.c` の `fle_ia2_path_absent`)を、既定の動作として外す。オプトインを作らない。
   - (c) 止めないだけの修正(例外を処理する)は、受け入れ(ノートが作られる)を満たさない。単独で直したら、完了とせず、その旨を報告に書く。
   - 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。FL が内蔵する Python のスクリプトを、ゲストとして動かすのは FL の機能なので、よい。
4. 測る: 道具で、DPR 1 と、乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool chord-progression --dprs 1,$D > "$RUNNER_TEMP/chord.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&r.notes>=1&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/chord.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上、`notes` が 1 以上、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行): 各行が、`error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 原因の特定(実測の行。`missing-path:` と `stop` の行)、直した方式と理由、直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値。

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

- blocked_by: なし。
- #{{147}}(Riff machine)は、この Issue が作る道具 `tools/piano-roll-tools/measure.mjs` を使う。147 は、この Issue が閉じるまで起動しない。
- `ci/resources-prefixes.txt` の変更は PM が行う(この Issue の作業者は変えない)。
