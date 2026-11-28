---
draft: "256"
title: "起動の最中(最初の idle の前)に入力を送ると、Delphi の例外 0xEEDFADE を処理する frame が見つからず、engine が SIGABRT で止まる(間欠。乱数の DPR 2.29 の `context-menu-none`)"
type: "Task"
milestone: "M4 FL に必要な物の配布"
parent: "133"
blocked_by: []
labels: ["verify"]
size: "L"
priority: "Urgent"
project: "signature-studio/3"
---

## 目的

起動の最中(最初の idle の前)に入力を送ると、engine が Delphi の例外(コード 0xEEDFADE、flags 0x1、引数 2 番目が `Exception` のオブジェクト)で止まる(SIGABRT、終了コード 134)。間欠で、乱数の DPR 2.29 の表示 `context-menu-none` の before-idle 領域で 1 回起きた。smoke の判定が落ち、本番への deploy を止めている。

この Issue の目標は次の 3 つ。

1. 最初に投げられた例外 X(種類・メッセージ・発生元の枠)が、止まる時の記録で分かること(既定の動作。オプションにしない)。
2. X の発生元と原因を特定し、起動の最中の入力で X が起きないように直すこと。例外を握りつぶして停止を消すのは直しにしない。
3. 起動・流れ A〜D・`npm test`・`seh_rt` の試験が通ること(回帰が無いこと)。

## 背景

### 停止の記録(依頼の記録の引用。記録の一部は省略)

- 表示 4k-random(DPR 2.29)、scenario `context-menu-none` の before-idle 領域(起動の最初の描画の前に、右クリックを 20 回送る)。`ok` が false。scenario の判定(contextmenu が全て cancel される)は通っていた。`engineStops` が 1。
- 同じ job の他の表示(DPR 2.39・smoke-dpr1・smoke-dpr2)では起きていない。
- 停止の文言(`engine.exit.stop.lastFle`。`lift/runtime/fle_seh.c` の 610 行が作る):

  ```
  fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no frame-based handler took it (11 frames searched); ended at return address 0 (read at 0x3e958818) after frame 11; arg1 class Exception < TObject; first frames 1-11 [411e4d:r0 411edb:r0 d340a7:r3 dc2f37:r2 dbf6ba:r2 dc2aa2:r2 1034956:r2 114cfb4:r0 92d940:r0 976b4f:r0 52a596:r0]
  ```

- `message` は証拠の行(描画の画素の行)で、停止の理由ではない。停止の理由は `lastFle` の行だけ。
- 同じ戻り番地 0x411e4d・同じ文言の停止が、実機の記録にも在る(#1410)。

### 読みの結果(静的な読み。この Issue の前の作業で、実行はしていない)

詳しい表は `pm-work/findings/startup-delphi-exception-stop.md` に在る。本文に要る所だけを書く。

- 11 枠の戻り番地は、全て、直前の call 命令の戻り先と一致する(枠 9 と 11 は間接 call で、先は確かめられない)。つまり、枠の探索(`lift/runtime/fle_seh.c` の `unwind_one`)は、この 11 枠では実際の呼び出しの並びを正しく辿っている。
- 枠 1(0x411d20)は raise 補助で、RaiseException を呼ぶ(戻り 0x411e4d)。枠 2(0x411ea0)は、現在の例外を取り出して同じオブジェクトを投げ直す `raise;`(再 raise)。枠 3(0xd33f00)は try..except で、catch-all(on 節の無い except)の範囲 [0x933f65, 0x93406c) を持つ。再 raise の呼び出し(0xd340a2)は、その except 本体(landing 0x93406c から)の中の、catch-all の範囲の外(a == 0 の範囲 [0x93406d, 0x9340a8))にある。枠 3 は取らず、枠 4〜11 には検索で呼ばれるハンドラ(UNW_EHANDLER)が無い。
- したがって、この停止は、**最初の例外 X を枠 3 の catch-all が一度取り、その except 本体が投げ直した 2 回目の dispatch** と読める(推定。強い)。停止の記録は 2 回目だけを示し、X は記録に無い。arg1 のクラスが Exception(基底)なので、X も同じ種類のオブジェクトと推定する。
- 枠 11(0x52a570)は、`TMessage`(Msg は +0x20、WParam は +0x28、LParam は +0x30、Result は +0x38)を作って、`[r11]` を呼ぶ thunk。枠 10(0x976b30)は `Message.Msg == 0x401` の時だけ枠 9 を呼ぶ。この経路には try..except の枠が無い。0x401 を送る経路は確認できない。
- 根(枠 11 の上)の戻り番地 0 がどこから来たかは確認できない。`fle_call_guest` は戻り番地 0xCB000000 を積む(`lift/runtime/fle_callback.c` 89 行)ので、この根は fle_call_guest からの呼び出しではない。
- 文言に `bytes below the stack base` が無い。根の読みが、TEB の基準より上だったか、基準が 0 だったかは、文言から区別できない。

つまり、この停止は、探索が戻り番地 0 に達したこと自体ではなく、**X が枠 3 で取られて投げ直され、その先に取る枠が無かったこと**による。この Issue の最初の仮説(枠の探索が戻り番地 0 で止まる理由)は、この chain では支持されない。

## 読む物

- `lift/runtime/fle_seh.c`: `fle_seh_raise`(527 行から。探索の loop は 577–609 行)、`unwind_one`(319–440 行)、`record_append`(269–296 行)、停止の文言(610 行)、`fle_seh_unwind_ex`(624 行から。landing に入る所は 698–707 行、`_longjmp` は 707 行)。
- `lift/runtime/fle_callback.c`(`fle_call_guest`。89 行が戻り番地 0xCB000000)と `lift/runtime/fle_callback.h`。
- `lift/runtime/fle_thread.c`(`stack_top` 749 行付近、`thread_main` 1167 行付近、guest thread の初期の rsp 1280 行付近)。
- `lift/crates/fe-lift/tests/seh_rt.rs`(枠組み。`flengine()`・`stops`・`CALL`・`wrapper`。`FLENGINE_PE` を付けないと、この試験は SKIPPED で空に通るので、必ず付ける)。
- 読むだけ: `lift/out-app/host/binaries/win64/FLEngine_x64.dll`(CI の置き場。ImageBase 0x400000)。逆アセンブルは `objdump`(または `llvm-objdump`)でよい。
- `pm-work/findings/startup-delphi-exception-stop.md`(上の読みの詳細)。
- `tools/e2e/scenarios/smoke/context-menu-none.mjs`(before-idle 領域の入力)と `tools/e2e/README.md`(`--repeat`。`--scenarios smoke` は、各シナリオを新しいページで動かす)。

## やること

1. **頻度の実測(直す前に)**。`bash tools/e2e/run.sh --scenarios smoke --only context-menu-none --displays 4k-random --repeat 10` を 1 回走らせ、停止の回数と、各停止の `lastFle` を報告に書く。10 回で 0 回なら、20 回まで増やす。それでも出ない時は、その事実を「確認できない」と書いて、手順 2 へ進む。
2. **最初の例外 X を記録に残す**(既定の動作。新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足さない。オプションにしない)。`fle_seh_unwind_ex` が landing に入る所(`reg->land_ip = target_ip` の直後、`_longjmp` の前)で、次を静的な変数に残す。
   - 取った例外の code と flags。
   - arg1 のクラス名(`class_of` の結果)。
   - Exception のメッセージ(オブジェクトの +8 にある UnicodeString。ポインタと長さ(ポインタの -4 の 32 ビット)を読めた時だけ。読めなければ「読めない」と残す)。
   - pass 2 の枠の一覧(`trail_head`・`trail_n` を写す。X の raise から landing までの枠)。
   
   残した記録を、停止の文言(`fle_seh_raise` の 610 行)の末尾に入れる。文言の先頭(指紋 80 字の `fle: RaiseException: exception code 0xeedfade, flags 0x1` で始まる部分)は変えない。`engine.exit.stop.lastFle` の上限は 400 字(`app/shared/perf.ts` の `PERF_EXIT_LAST_FLE_MAX`)なので、X の記録は前に、長い枠の一覧は後ろに置く。
3. **X の発生元を特定する**。手順 2 の記録で、X のクラスとメッセージと発生元の枠を読む。枠 3 の try 本体の呼び出し(0xd33f77・0xd33f8e・0xd33f9a・0xd33fba・0xd33feb・0xd33ffb・0xd3400b・0xd3401c・0xd34030・0xd3403f・0xd34048・0xd3405d。VA)のどれが X を起こすか、または、その先の枠かを確かめる。そのうえで、起動の最中の入力でだけその経路が通る理由を、コードと記録から読む。実測と推定を分けて報告に書く。
4. **原因を直す**。X が起きる原因を直す(例: VM の API の返り値、起動の最中で初期化されていない物、入力の順序など。記録で決まる)。次は直しにしない: 例外を catch して無視する、handler の戻り値を変えて隠す、停止の判定を外す、試験の期待値を弱める、既知の失敗の一覧に足す。直せない時は、停止を名前つきで残し、何が直せていないかを報告に書く。
5. **試験**。手順 2 の記録が出ることを、`seh_rt.rs` の枠組みで試験にする(先に red を書く)。直した後は、`seh_rt` の全ての試験が通ること(SKIPPED で通っていないこと)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && FLENGINE_PE="$PWD/lift/out-app/host/binaries/win64/FLEngine_x64.dll" cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test seh_rt && (cd tools/e2e && npm ci && npx playwright install chromium) && bash tools/e2e/run.sh --scenarios smoke --only context-menu-none --displays 4k-random --repeat 5 > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const r=L[0];process.exit(L.length===1&&r.servedHeadersMatch===true&&r.error===null&&r.scenarios.length===5&&r.scenarios.every((s)=>s.ok===true&&s.engineStops===0&&s.skipped!==true&&s.dprOk!==false)?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。`seh_rt` が実機の DLL で走り、全て通る(手順 5 の新しい試験を含む)。
- 乱数の DPR での 5 回の実行で、全て `engineStops` が 0 かつ `ok` が true(上の `node -e` の判定)。
- `bash guard/scan.sh` の指摘が 0 件。

判定の式は、合成データ(通る組と落ちる組)で先に確かめてから使う。

## 作業の進め方

- 作業の最初に `date -u` を記録する。**3 時間 15 分**を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは 1 回にして、同じ bundle で確かめる。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 手順 1 は、直す前に行う(頻度を先に知る)。停止が出なくても、その事実を報告に書く。
- 作業の最後に、受け入れが通らなくても作業ツリーを commit する。未 commit の変更が残ると判定は失敗する。`git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。記録の追加は既定の動作として入れる(オプションにしない)。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- AI のベンダー名と Cloudflare の識別子を、ファイル・コミット・報告に書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 例外を握りつぶして停止を消す直し、試験の期待値を弱める直し、既知の失敗の一覧に足す直しは、直しにしない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く。(a) 手順 1 の停止の回数と、各停止の `lastFle`(記録の一部は省略してよい)。(b) 手順 2 で残した X の記録の内容(クラス・メッセージ・発生元の枠)。(c) X の発生元と原因(実測と推定を分けて)。(d) 直した方式と、その理由。(e) 試験の red と green の出力の行。(f) 起動確認の `elapsedMs`、流れ A〜D の 4 行。(g) 乱数の DPR の 5 回の `engineStops` と `ok`。(h) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。確認できなかった物は「確認できない」と書く。

## 他のタスクとの関係

- `blocked_by` は無い。
- #1410(実機の同じ文言・同じ戻り番地 0x411e4d。枠の探索の仮説(a)(b)(c)は、この Issue の読みで、この chain では支持されない。X の記録は #1410 の実機の停止の原因の特定にも使える)。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
