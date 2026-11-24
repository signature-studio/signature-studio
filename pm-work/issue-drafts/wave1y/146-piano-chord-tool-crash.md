---
draft: "146"
title: "Piano roll のコード進行ツールの Start creating で止まる: FL 内蔵の Python(python312.dll)を real module として読み込めるようにする(段階 1: LoadLibrary が成功し、止まらない)"
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

依頼者の実機で、Piano roll の Tools メニューの「コード進行ツール」を開き、"Start creating" を押した瞬間に、FL が落ちる(engine が止まる)。この Issue は段階 1 で、次の 2 つを目標にする。
1. Start creating を押しても engine が止まらず、FL 内蔵の Python の実行環境(`Python\Python312.dll`)の読み込み(`LoadLibraryW`)が成功すること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

段階 2(Start creating で Piano roll にノートが作られること)は、別の Issue(下書き 151。題名「コード進行ツールの Start creating で、Piano roll にノートが作られる(FL の Python のスクリプトの実行)」)で扱う。段階 1 だけでは、ノートは作られない。

## 背景

- 依頼者の報告(原文): 「コード進行ツールは "Start creating" を押下した瞬間クラッシュします」。
- 事実 1(配信の一覧 `inputs/fl-installer-files.tsv`): コード進行ツールの実体は、FL 内蔵の Python のスクリプトに見える。`System\Tools\Chord progression\` に 59 件(`cpt.pyscript`・`stack.pyscript`・Python の bytecode の `.pyc` など)。大きさは、この一覧に無い(dll・exe 以外は空)。
- 事実 2(同じ一覧): Python の実行環境は `Shared\Python\`(37 件。`python312.dll` 7,003,928 B、`python312.zip`、`python312._pth`、`_*.pyd` など)。インストーラに在る。
- 事実 3(配信の範囲): 配信の接頭辞 `ci/resources-prefixes.txt` には、`System\Tools\Chord progression\` と `Shared\Python\` を足した済み(main の fc686522)。実物は作業ツリーの `lift/out-app/host-extra/resources/` に置かれる。一方、起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、`Shared\Python\` と `System\Tools\` の行が 0 件。seed に無い物は VFS に載らない。`ci/extract-dlls.tsv` にも `python312.dll` の行は無い。
- 事実 4(読み込みの経路 `lift/runtime/fle_load.c`・`lift/runtime/fle_wall_ia2.c` の `fle_ia2_path_absent`): 「Python\Python312.dll」の `LoadLibraryW` は、常に「見つからない(126)」と答える。この答えは、記録の上で選んだ choice(`fle_wall_ia2.c` の注と、`lift/harness/seeds/ia2-imports.txt` の W21 の行)。その注は「インストーラに Python312.dll が無い」と書くが、事実 2 の一覧には `Shared\Python\python312.dll` が在る。食い違い。この choice は、チャンネルラックの設定アイコン(ia2-W21)の要求にも効いている。
- 事実 5(別の試行の記録 `pm-work/findings/blocks-samples-1366.md` の 4.3): Welcome の Default の行を開いた直後に、`[err] missing-path: LoadLibraryW 126 "Python\Python312.dll"` と `[err] fle: RaiseException: exception code 0xeedfade ...` が出て、engine が exit code 134 で止まった(測定の記録)。
- 仮説(前回の試行で一部を確かめた。下の「前回の試行で分かったこと」): `LoadLibraryW` が 126 で失敗し、その後で呼び出し側が不正な値を読んで止まる。

## 前回の試行で分かったこと

(run 45367751383・failed。作業は保存された。製品は変えていない。道具と記録は main に入った: 05577cf9)

- 症状の再現: 道具 `tools/piano-roll-tools/measure.mjs` で DPR 1 と 1.79 を実行。Start creating の押下の後、engine が exit code 134 で止まる(窓は止まらず開く。Enter で閉じると止まらない)。押下の後のログ(長い path は省略):

```
missing-path: GetFileAttributesW 3 "…\Python\Python312.dll"
missing-path: GetFileAttributesW 3 "Python\Python312.dll"
missing-path: GetFileAttributesW 3 "…\Shared\Python\Python312.dll"
missing-path: LoadLibraryW 126 "Python\Python312.dll"
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: exit code 134 (...)
fle: guest address 0x520000077c (8-byte load) has bits above 32 set: not representable in wasm32 (a native run faults here) [fle_addr_hi32]
```

- 原因(実測): seed 表に `Shared\Python\` と `System\Tools\` の行が無いので、`Python\Python312.dll` が VFS に無く、`LoadLibraryW` が 126 で失敗する。その後、呼び出し側が NULL 由来の不正な値を読んで止まる(推定。戻り番地から関数名は引けていない)。配信の接頭辞と resources の実物は揃っているので、足りないのは VFS の経路と、real module の実装。
- 直し方の検討:
  - (a) 配信だけ(seed 表に `@lazy-file:` で 94 件を足す)は、止まる場所が変わるだけ。lazy の上限(`app/src/bundle/manifest.ts` の maxItems 128、今 75 件)も超えるので、`app/` の変更と deploy が要る。
  - (b) `python312.dll` を real module として lift して動かす: 19 DLL・358 import。runtime が名前を持つのは約 174 件(名前の一致による粗い数)で、残りの約 184 件は未実装と見られる。
  - (c) `fle_ia2_path_absent` を外す: 試していない。ia2-W21 の要求にも効くので、画面の動きが変わる。
- PM の決定: (b) を進める。止めるだけの修正は採らない。
- 道具の判定の訂正: `== stop: returned from entry` は、正常に動く起動でも毎回出る。前回の道具は、この行を除いて `stopped` を判定する(05577cf9 の道具で実装済み)。
- 前回の値(道具の JSON。`missingPaths` は 590 件超のため省く):

```
{"tool":"chord-progression","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":null,"elapsedMs":6745,"preBootBytes":311109738,"room":{"width":1836,"height":1056}}
{"tool":"chord-progression","dpr":1.79,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":null,"elapsedMs":6675,"preBootBytes":311109738,"room":{"width":1836,"height":1056}}
```

- DPR 1.79 の `room` が 1836×1056 のまま(DPR 1.37 では 2515×1447)。原因は未確認。
- 流れ A〜D: 2 回の実行で、起動が first idle に届かず `state running` で失敗する回があった(起動の前に viewport を変えた起動でだけ観測。ほかの 4 回は成功)。原因は未確認。Issue #1393 で扱う。
- まだ無い物: `notes` の数え方(段階 2)。

## 読む物

- `pm-work/findings/chord-tool-crash.md`(前回の記録。停止の全文と道具の値)
- `tools/piano-roll-tools/measure.mjs`(道具。段階 1 で `pythonLoaded` を足す)
- `inputs/fl-installer-files.tsv`(`System\Tools\Chord progression\`・`Shared\Python\`)
- `ci/resources-prefixes.txt`・`ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`
- `lift/runtime/fle_load.c`(`LoadLibraryW`。334 行付近で `fle_realmod_find_path` を呼ぶ)・`lift/runtime/fle_realmod.c`(real module の探索)・`lift/runtime/fle_wall_ia2.c`(`fle_ia2_path_absent` と、その注)・`lift/harness/seeds/ia2-imports.txt`(W21 の行)
- `lift/plugins/` の仕組み(#1369。1 プラグイン 1 ディレクトリ。`app/scripts/extract-fl.mjs` の `--plugins lift/plugins`)
- `lift/runtime/fle_vfs.c`(`missing-path:` の行)・`lift/runtime/fle_proc.c`(`RaiseException`)
- `tools/smoke/welcome-flow.mjs`・`tools/boot-check/boot-bundle.mjs`(`bootBundle` と `afterIdle`)

## 先に行う(PM の作業は済み)

- 配信の接頭辞の追加は、main の fc686522 で済み(`System\Tools\Chord progression\` 59 件・`Shared\Python\` 37 件)。作業者は `ci/` を変えない。`ci/extract-dlls.tsv` に行が要ると分かったら、変えずに、報告に書いて終える(PM が足す)。
- 道具の `pythonLoaded` の項目は、この Issue の作業者が足す(下の「やること」0)。

## 実物の置き場

- ラベル `resources` の extract が、`ci/resources-prefixes.txt` の接頭辞で取り出し、作業ツリーの `lift/out-app/host-extra/resources/` に置く(`Shared\Python\` 37 件を含む)。
- 受け入れの先頭で `mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/` を行い、ビルドの入力の場所へ写す(`fe-build bundle` の `pack-seed.mjs` は `lift/out-app/host/resources/` を読む)。
- 起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)に行が無い物は、VFS に載らない。足りない実物の行は、作業者が seed 表に足してよい。host の欄を `@lazy-file:` で始めると、起動の後に取る(#1345 の方式)。段階 1 の lazy は、今の 75 件に `Shared\Python\` の 37 件を足して 112 件で、上限 128 以内。`System\Tools\Chord progression\` の 59 件は段階 2 で扱い、上限を超えるなら報告に書いて終える。

## やること(段階 1)

0. 道具 `tools/piano-roll-tools/measure.mjs`(main の 05577cf9 の版)に、JSON の項目 `pythonLoaded` を足す。engine のログの `LoadLibraryW` の行から取る(`Python\Python312.dll` の読み込みが非 NULL で成功した時 true)。成功の行がログに無ければ、LoadLibraryW が成功したときに標準エラーへ 1 行書く(記録のみ。動作は変えない)。`notes` は段階 2 で数える。段階 1 では null のままでよい。
1. real module: `Shared\Python\python312.dll` を、#1369 の仕組み(`lift/plugins/python312/` の `plugin.tsv`。1 プラグイン 1 ディレクトリ)で取り出し、lift する。#1369 が main に入った後に作る。
2. import の一覧: 358 件の import を、`pm-work/findings/python312-imports.md` に書く(DLL ごとの名前と、runtime が実装済みか、未実装か)。未実装の約 184 件を、次のどちらかにする。実装する(FL の Python が使う物: ファイル・時刻・環境変数・スレッド・TLS・ヒープ・ロケール・コンソール)か、Windows と同じ失敗を返す stub(ソケット・プロセス・レジストリの一部)。
3. 名前の解決: `LoadLibraryW "Python\Python312.dll"` が、`fle_realmod_find_path`(`lift/runtime/fle_realmod.c`)で real module の `python312.dll` を見つける。seed 表と plugin.tsv の `guest` の値で通す。
4. 起動の後の取得: Python の DLL と標準ライブラリは、起動の後に取る(`@lazy-file:`)。起動前に取る量(`preBootBytes`)を増やさない。
5. `fle_ia2_path_absent`(`Python\Python312.dll` を「見つからない」と答える choice)を外すかは、実測の後で決める。外すなら、ia2-W21(チャンネルラックの設定アイコン)の動作の変化を報告に書く。オプトインは作らない。
6. 測る: 道具で、DPR 1 と乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool chord-progression --dprs 1,$D > "$RUNNER_TEMP/chord.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&r.pythonLoaded===true&&!r.missingPaths.some((l)=>l.startsWith("missing-path: LoadLibraryW")&&l.includes("Python"))&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/chord.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(段階 1):
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上、`pythonLoaded` が true、`LoadLibraryW` の行(Python を含む)が `missingPaths` に 0 件、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。`GetFileAttributesW` の探索の行は判定に入れない(成功の後にも出うる)。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行): 各行が、`error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。
- 段階 2 の「ノートが 1 以上」は、この Issue の受け入れに入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値、`pythonLoaded` の根拠の行、import の一覧の件数(実装・stub・未対応)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。FL に内蔵の Python の DLL を、ゲストとして動かすのは FL の機能なので、よい。`ci/` は作業者が変えない。
- ベンダー名と Cloudflare の識別子を書かない。ぼやけた・拡大した画面を出さない。175%・DPI 168/169・DPR 1.7647 のような値を、定数・表として埋め込まない。
- OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- この Issue は 1 回の試行で終わらない可能性が高い。次の試行は、その commit から続く(`.prior-work.md` を読む)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #1369(プラグインの追加を、1 プラグイン 1 ディレクトリ `lift/plugins/<名前>/` の追加だけにする)。#1369 が閉じるまで、この Issue は起動しない。
- #1387(Riff machine)は、道具 `tools/piano-roll-tools/measure.mjs` だけを使う。この Issue を待たない。
- 段階 2(下書き 151)は、この Issue が閉じるまで起動しない。
- `ci/` の変更は PM が行う(この Issue の作業者は変えない)。
