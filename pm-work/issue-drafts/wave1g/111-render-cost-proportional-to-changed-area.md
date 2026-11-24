---
draft: "111"
title: "描画の所要を、変化した面積に比例させ、物理画素の画面(6 Mpx)で 120 fps の予算に収める"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["110", "107", "109"]
labels: ["render", "perf"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

依頼者の実機の画面(約 6 Mpx の物理画素。道具の既定の DPR で測る)で、P2 の描画の部分を満たす。提示 1 回の描画の所要を、画面全体の画素数ではなく、変化した面積に比例させる(変化していない面積を描き直さない)。

runner には GPU が無い(ソフトウェアの描画)ため、絶対値は実機と桁が違う。そこで runner の受け入れは、変更の前後の比で判定する。P2 の最終の判定は実機で行う(下の「実機の判定」)。

## 背景

- #{{008}} の実測(`pm-work/findings/p2-measure-1308.md`。runner の値。n = 1): 拡大率 2(4.57 Mpx)で、提示 1 回の compose が 4.9 ms、put が 5.0 ms。合計約 10 ms で、120 fps の予算 8.33 ms を超える。
- 表示方式 D(`app/src/engine/render-worker.ts` の 129 行付近の注釈): runtime が書いた矩形を通知し、render Worker が窓ごとのテクスチャを WebGL2 で描く。`app/src/engine/desk-gl.ts` は、書かれた矩形ごとに `texSubImage2D` で転送する。
- 入力: マウスは 1000 Hz 以上で動く(#{{109}})。入力の処理は #{{109}} の合成(最新の位置だけ)を前提にする。
- 物理画素の画面は #{{107}} で入った。render Worker と canvas の大きさを変える。
- runner の `cpuPerPresentMs` は、レンダラの CPU を提示の回数で割った値。提示の回数が増えると、1 回あたりの値は下がって見える。

## 前回の試行で分かったこと(run 45371958340。`failed`)

- 前回の作業は保存された: artifact `agent-1351` の `work.diff`(commit `50e280c5` の差分。push していない)。次の試行は「前回の作業を取り込む」の段で作業ツリーに載せる。`.prior-work.md` が在れば最初に読み、載った差分を `git status` と `git diff HEAD` で確かめる。
- 前回の変更の中身:
  - `app/src/engine/desk-gl.ts`: 変化した窓の矩形だけを、scissor の箱で消去して描く。キャンバスは `preserveDrawingBuffer: true` で前の絵を保つ。窓の配置・重なり・領域が変わった提示は全面を描く。統計 `drawPixels` を足した。
  - `app/test/unit/desk-gl.test.ts`: 2 件を足した(書かれた矩形だけの描画。領域の変化は全面)。
  - `tools/p2-frames/measure.mjs`: 提示の内訳 `desk` を出す。メトロノームの位置を CSS px から換算する(既定の DPR では、前の位置が再生ボタンに当たっていた)。起動後の警告「Problems loading the project」を、その OK で閉じる。
  - `pm-work/findings/p2-render-1351.md`: 前回の報告。
- 変更の前の値(実測。runner の値。nproc 4。loadavg 1.7〜5.3。n = 3 の平均。JSON は `pm-work/findings/p2-render-1351-before.json`):
  - 既定の DPR・負荷 a: 提示 134(30 秒の窓)・GPU 432 ms/提示・描画の画素 100%(全面)・`inputToPresentMsP95` 254 ms・`cpuPerPresentMsP95` 344 ms。
  - 既定の DPR・負荷 b: 提示 144・GPU 426 ms/提示・描画の画素 100%・`inputToPresentMsP95` 244 ms・`cpuPerPresentMsP95` 244 ms。
  - DPR 1・負荷 a: 提示 435(14.5 回/秒)・GPU 139 ms/提示・描画の画素 40%・`inputToPresentMsP95` 83 ms。
  - DPR 1・負荷 b: 提示 445・GPU 135 ms/提示・描画の画素 25%・`inputToPresentMsP95` 77 ms。
- 変更の後の値(実測。同じ run の n = 3。既定の DPR と DPR 1):
  - 描画の画素の割合: 既定の DPR で負荷 a 10%・負荷 b 34〜35%。DPR 1 で 13〜17%(a)・10〜11%(b)。描画の画素と転送の画素は一致した。→ 「変化した面積に比例」は受理。
  - 提示の回数: 既定の DPR で 387〜395(a)・255〜264(b)。DPR 1 で 1196〜1337(a)・1084〜1110(b)。
  - GPU プロセスの ms/提示: 既定の DPR で 68(a)・171〜177(b)。DPR 1 で 21〜29。
  - `inputToPresentMsP95`: 既定の DPR で 202〜223(a)・139〜144(b)。DPR 1 で 69〜70(a)・34〜40(b)。
  - `cpuPerPresentMsP95`(判定に入れない): 既定の DPR で 171〜175(a)・147〜157(b)。レンダラの CPU の総量は 1 秒あたり約 1.1 → 1.5 コアに増えた。提示の回数の増加が、1 回あたりの値を下げている。
- 縁(`edgeWidth`。`measure-size.mjs`): DPR 1 で 2.261、既定の DPR の行で 2.626。変更とは無関係(#{{107}} の記録と同じ値)。
- 負荷 b の `valid`: 3 回とも false(pointermove が 755〜764 件/秒。変更の前も 735〜737)。道具の測り方の問題の可能性がある(推定)。
- 起動(boot-bundle): engine-idle・standIn false・error null・11184 ms。
- 起動の止まり: 変更の前後に各 1 回、`running` のまま 180 秒 engine-idle に届かない run があった。#{{149}} と同じ症状の可能性がある(推定)。
- 前回の判定が落ちた理由(判定の式の欠陥。製品の失敗ではない):
  1. 絶対値の `cpuPerPresentMsP95` と `inputToPresentMsP95` は、runner では届かない(GPU が無い)。前者は提示の回数で割るため、改善が「下がった」とは見えない。→ 変更の前後の比で判定する(下の (a))。
  2. 負荷 b の `valid`(入力の件数)は、判定から外し、`validB` として報告に出す。
  3. `edgeWidth` の絶対値の基準は、DPR 1 の縁(2 を超える)と両立しない。→ #{{108}} と同じ式(DPR 1 の同じ run の値からの比)。
  4. 120 fps は、表示の上限のある headless では測れない。→ 判定から外す(未確認)。
- 残る要因(推定。実測と分けて書く): 提示 1 回あたりのエンジン(ゲストの wasm)の CPU が大きい。提示が約 13 回/秒では、描画を 0 にしてもエンジンだけで提示 1 回あたり約 100 ms かかる(推定)。実 GPU では `preserveDrawingBuffer` による複写が入りうる(未確認)。転送は窓ごとの外接矩形のため、画面の 10〜39% を含む(runtime の `sh_trim`。区切るには `lift/runtime` の変更が要る)。

## 読む物

- `pm-work/findings/p2-render-1351-before.json`(変更の前の実測。受け入れが読む)
- 前回の差分(作業ツリーの `.prior-work.md` と `git diff HEAD`)
- `tools/p2-frames/measure.mjs`(#{{110}} の道具。`desk` の内訳)、`tools/feat-c1-hidpi/measure-size.mjs`(#{{107}} の道具。縁の遷移)
- `app/src/engine/desk.ts`(`DeskPresentStats`)、`app/src/engine/desk-gl.ts`(テクスチャの更新、`texSubImage2D`、scissor)、`app/src/engine/render-worker.ts`(129 行付近の注釈。提示の時刻 `presentedAt`)
- `app/test/unit/desk-gl.test.ts`
- `lift/runtime/fle_desk.c`(書かれた矩形の記録。`sh_trim`)、`app/src/engine/client.ts`(入力のリングへの書き込み)

## やること

0. 前回の作業を取り込む: `.prior-work.md` を読み、差分が作業ツリーに載ったかを確かめる。載っていれば、それを土台にする(作り直さない)。
1. 受け入れを通す(下の「受け入れ」)。最初の実行の出力(`judge:` の行と、判定の JSON の行)を読む。
2. 落ちたら、出力から切り分ける。(a) 製品の欠陥(描画の比が足りない・音・起動)なら直す。(b) 道具の欠陥なら報告に書く(道具の判定の式は変えない。PM が直す)。
3. 変えてよい範囲: 描画の経路(`app/src/engine/desk.ts`・`desk-gl.ts`・`render-worker.ts`)と、その単体テスト。`lift/runtime` は、転送の区切りを変える必要が出た時だけ(wasm の再ビルドを含む。報告に書く)。
4. 画質を落として速くしない。窓の縁(`edgeWidth`)と色・描き順は変えない。
5. 入力の届き方(参考。判定に入れない): 受け入れの後に `node tools/p3-audio-probe/measure-live.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --play 1 --hz 1000 --mouse 30` を 1 回行い、ページが受けた pointermove の件数(毎秒)を報告に書く(#{{109}} の測定の道具。公称 1200 Hz)。
6. 測定のための一時的な変更は、受け入れの前に取り除く。

## 受け入れ

DPR 2.25 以上は、#{{149}}(起動の間欠の停止)が直るまで、この Issue の受け入れに使わない。#{{149}} の後に、#1348 が確かめる。縁の判定(`measure-size.mjs`)は、DPR を 1 と乱数の 1.25〜2.20 の 2 点に絞って渡す(`--dprs 1 --random 1 --random-range 1.25,2.20`)。道具は、渡した DPR の後に DPR 2.5・3 を参考の行(`highDpr`)として 1 回ずつ起動するが、受け入れは読まず、終了コードにも入らない。

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-1351 && for l in a b; do for i in 1 2 3; do node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --load $l > ".tmp-1351/d-$l-$i.json" || exit 1; node tools/p2-frames/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dpr 1 --load $l > ".tmp-1351/o-$l-$i.json" || exit 1; done; done; node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1 --random 1 --random-range 1.25,2.20 > ".tmp-1351/size.jsonl"; for i in 1 2 3; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --boot > ".tmp-1351/boot-$i.json" || exit 1; done; node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > ".tmp-1351/flow.jsonl"; node -e 'const fs=require("fs"); const rd=(f)=>JSON.parse(fs.readFileSync(f,"utf8").trim()); const rows=(f)=>fs.readFileSync(f,"utf8").trim().split("\n").map((l)=>JSON.parse(l)); const B=rd("pm-work/findings/p2-render-1351-before.json"); const mean=(xs)=>xs.reduce((s,x)=>s+x,0)/xs.length; const runs=(p,l)=>[1,2,3].map((i)=>rd(".tmp-1351/"+p+"-"+l+"-"+i+".json")); const D={a:runs("d","a"),b:runs("d","b")}, O={a:runs("o","a"),b:runs("o","b")}; const ok=(r)=>r.error===null&&r.desk!==null&&typeof r.inputToPresentMsP95==="number"; if(![...D.a,...D.b,...O.a,...O.b].every(ok)){console.log("judge: 測定の値が欠ける(error・desk・inputToPresentMsP95)");process.exit(1);} const sum=(rs)=>({presents:mean(rs.map((r)=>r.desk.presents)),gpu:mean(rs.map((r)=>r.desk.gpuProcessMsPerPresent)),share:mean(rs.map((r)=>r.desk.drawPixelShare)),inP95:mean(rs.map((r)=>r.inputToPresentMsP95))}); const ds={a:sum(D.a),b:sum(D.b)}, os={a:sum(O.a),b:sum(O.b)}; const c={}; for(const l of ["a","b"]){ const bd=B.default[l], bo=B.one[l]; c["presents_"+l]=ds[l].presents>=1.5*bd.presents; c["gpu_"+l]=ds[l].gpu<=0.6*bd.gpuProcessMsPerPresent; c["share_"+l]=ds[l].share<=0.5*bd.drawPixelShare; c["input_"+l]=ds[l].inP95<=1.1*bd.inputToPresentMsP95; c["onePresents_"+l]=os[l].presents>=1.5*bo.presents; } c.validA=[...D.a,...O.a].every((r)=>r.valid===true); c.proportional=[...D.a,...D.b].every((r)=>Math.abs(r.desk.drawPixelShare-r.desk.pixelShare)<=0.01); const S=rows(".tmp-1351/size.jsonl").filter((r)=>typeof r.dpr==="number"); const one=S.find((r)=>r.dpr===1), rnd=S.find((r)=>r.dpr!==1); c.edge=S.length===2&&one!==undefined&&rnd!==undefined&&[one,rnd].every((r)=>r.error===null&&r.state==="engine-idle"&&r.standIn===false&&typeof r.edgeWidth==="number")&&rnd.edgeWidth<=one.edgeWidth*(1+(rnd.dpr-1)/2)+0.5; const P=[1,2,3].map((i)=>rd(".tmp-1351/boot-"+i+".json")); c.boot=P.every((r)=>r.error===null&&r.bootLast60sAt70s===0&&r.bootLast60sAt90s===0&&typeof r.elapsedMs==="number"&&r.elapsedMs<=12500); const F=rows(".tmp-1351/flow.jsonl"); c.flows=F.length===4&&["A","B","C","D"].every((f)=>{const r=F.find((x)=>x.flow===f);return r!==undefined&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&((f!=="B"&&f!=="D")||r.welcomeClosed===true)&&typeof r.elapsedMs==="number"&&r.elapsedMs<=12500&&typeof r.preBootBytes==="number"&&r.preBootBytes<=314368711;}); console.log(JSON.stringify({validB:[...D.b,...O.b].map((r)=>r.valid),ratios:{presentsDefault:{a:ds.a.presents/B.default.a.presents,b:ds.b.presents/B.default.b.presents},presentsOne:{a:os.a.presents/B.one.a.presents,b:os.b.presents/B.one.b.presents},gpuDefault:{a:ds.a.gpu/B.default.a.gpuProcessMsPerPresent,b:ds.b.gpu/B.default.b.gpuProcessMsPerPresent},inputDefault:{a:ds.a.inP95/B.default.a.inputToPresentMsP95,b:ds.b.inP95/B.default.b.inputToPresentMsP95}},checks:c})); const good=Object.values(c).every((x)=>x===true); console.log(good?"judge: 通った":"judge: 通らない"); process.exit(good?0:1);' && npm test --prefix app && rm -rf .tmp-1351
```

## 判定の基準

受け入れの `node -e` が、次を読む(runner の値)。変更の前の値は `pm-work/findings/p2-render-1351-before.json` の実測(n = 3 の平均)。変更の後は、今回の測定の n = 3 の平均と比べる。

- (a) 既定の DPR(道具の既定の点)・負荷 a と b の各々で:
  - (i) 提示の回数(`desk.presents`)の平均が、変更の前の 1.5 倍以上。
  - (ii) GPU プロセスの ms/提示(`desk.gpuProcessMsPerPresent`)の平均が、変更の前の 60% 以下。
  - (iii) 描画の画素の割合(`desk.drawPixelShare`)の平均が、変更の前の 50% 以下。
  - (iv) `inputToPresentMsP95` の平均が、変更の前の 1.1 倍以下(悪化させない)。
- (b) DPR 1(道具の measure-size の既定の点)・負荷 a と b の各々で: 提示の回数の平均が、変更の前の 1.5 倍以上。
- (c) 負荷 a の `valid` が、既定の DPR と DPR 1 の 6 回の全てで true。負荷 b の `valid` は判定に入れず、`validB` として出力に出す。
- (d) 比例: 既定の DPR の全 6 回で、`desk.drawPixelShare`(描画の画素の割合)と `desk.pixelShare`(転送の画素の割合)の差が 0.01 以下。描画の画素 = 転送の画素 = 変化した面積。
- (e) 縁(`tools/feat-c1-hidpi/measure-size.mjs --dprs 1 --random 1 --random-range 1.25,2.20`。DPR 1 の行と、道具が選んだ 1.25〜2.20 の乱数の DPR の行): 両方の行で state が engine-idle・standIn が false・error が null。乱数の DPR の `edgeWidth` が、DPR 1 の `edgeWidth` × (1 + (乱数の DPR − 1) ÷ 2) + 0.5 px 以下。`highDpr` の行は判定に入れない(#{{149}} の範囲)。
- (f) 音と起動: `tools/p3-audio-probe/measure.mjs --boot` の 3 回: error が null、`bootLast60sAt70s` と `bootLast60sAt90s` が 0、`elapsedMs` が 12,500 以下。`tools/smoke/welcome-flow.mjs` の流れ A〜D: 各行が error null・state engine-idle・standIn false・trapped false・hostErrors 0・opened true、流れ B と D は welcomeClosed が true、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- (g) `npm test --prefix app` が通る。

判定に入れないもの(報告に書く): `cpuPerPresentMsP95` の絶対値(runner の CPU。提示の回数で割る)、`inputToPresentMsP95` の絶対値、負荷 b の `valid`(`validB`)、120 fps(未確認)、`highDpr` の行。

閾値の考え方: runner の揺れは、n = 3 の平均と閾値の余白で見込む。閾値は緩めない(提示の回数が 1.2 倍・入力の p95 が 1.3 倍の合成の値は、落ちることを確かめた)。前回の作業の値は全て閾値の内側にある(合成の確認)。

## 実機の判定(P2 の最終)

- runner の受け入れは、変更の前後の比だけを見る。P2 の最終の判定は、依頼者の実機で、次の 2 つで行う: (1) 実機のビーコンの提示の頻度(fps)と、入力から提示までの遅延(#{{155}} で足す項目)、(2) #{{140}}(上部バーの Frame Rate の実測)。
- この Issue の報告に、実機の値は書かない(この機械では測っていない)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 受け入れの `judge:` の行と判定の JSON の行(`validB`・`checks`・`ratios`)、既定の DPR と DPR 1 の変更の前後の提示の回数・GPU・描画の画素の割合・`inputToPresentMsP95` の n = 3 の全て、縁の値(DPR 1 と乱数の DPR)、音と流れ A〜D の行、入力の届き方(参考)、変えたパス(app/・lift/・tools/ を含む。`app/` と `lift/` を変えると main の deploy が動くので、その旨)。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。既定の動作として直す。オプトインを作らない。
- 画質を落として速くしない。
- 受け入れの式に、変更の前の値や特定の DPR の値を定数として書かない(基準は JSON から読む)。
- 道具(`tools/`)の判定の式を、作業者が変えない(欠陥は報告に書く。PM が直す)。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや計測をしない。

## 作業の進め方

- 開始の時刻を `date -u` で記録する。受け入れの最初の実行は、開始から 2 時間 30 分までに終える。4 時間 00 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。受け入れは約 90 分。長いコマンドは、フォアグラウンドで直接実行し、シェルツールの `timeout` 引数に 5400000 を渡す。背景で起動して待たない。
- 受け入れが通っても通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{110}}(測定の道具)、#{{107}}(物理画素の画面)、#{{109}}(入力の合成)。
- #{{040}}(描画が音声を圧迫する経路)と #{{108}}(動的な変化と全ての拡大率)は、同じ描画の経路(`app/src/engine/desk.ts`・`desk-gl.ts`・`render-worker.ts`)を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- 受け入れは、#{{110}}・#{{107}}・#{{109}} の道具と、`tools/smoke/welcome-flow.mjs` の引数に合わせて書いてある。どれかの引数を変えたら、この受け入れも直す。
- 実機の判定の材料: #{{155}}(ビーコンの fps と入力の遅延)と #{{140}}(上部バーの Frame Rate)。どちらも blocked_by ではない。
