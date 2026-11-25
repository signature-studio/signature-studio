---
draft: "166"
title: "FL の例外(コード 0xEEDFADE。Delphi の raise と推定)が、枠の探索で取られず、engine が止まる: 実機の停止の上位を直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify", "audio"]
size: L
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、engine が例外で止まる。例外のコードは `0x0EEDFADE`(flags 0x1)で、どの枠にも取られない。この停止は、実機の停止の記録(#1404 の集計)の上位にある。

この Issue の目標は次の 3 つ。
1. 例外が、Windows と同じく、呼び出し元の枠(`try/except` に当たる枠)で取られ、engine が止まらないこと。直せた時は、直した原因と試験を報告に書く。
2. 直せなかった時も、次の実機の停止で原因が分かるように、止まる時の記録を厚くすること。
3. 起動・流れ A〜D・既存の試験が通ること(回帰が無いこと)。

## 背景

- 実機の記録(`pm-work/findings/real-user-stops-2026-11-25.md` の 3.1 と 3.5。全て読むこと)。
  - 版 `2578ea1e23f6` で 3 記録(1 セッション。起動から 47 秒で停止)。停止の文言は次の 1 行(`lift/runtime/fle_seh.c` が作る)。
    `fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no frame-based handler took it (11 frames searched)`
  - 版 `e29a730b932d` で、戻り番地 `0x411e4d`(上と同じ番地)の停止が 2 セッション・13 記録。その版の停止の行は無く、文言は分からない。
  - 戻り番地 `0x411e4d` は、`lift/crates/fe-lift/tests/seh_rt.rs` が、関数 `0x411d20` の中の実機の戻り番地として使っている(187・189 行)。
- 文言の読み(`lift/runtime/fle_seh.c`。コードの読み。実測ではない)。
  - 探索は、枠を 1 つずつ戻り、枠の表(`.pdata` と UNWIND_INFO)でハンドラを探す(`fle_seh_raise`。431–458 行の loop)。探索が終わるのは、戻り番地が 0 になった時(457 行)。文言の `no frame-based handler took it (N frames searched)` は、その時だけ出る(459 行)。
  - ホストの境界は、`fle_call_guest` が積む戻り番地 `FLE_CB_RETURN_ADDRESS`(`0xCB000000`。`lift/runtime/fle_callback.h`)。探索がここに着くと、文言は「ホストの境界を越える例外は未対応」(189–190 行)になり、観測の文言とは違う。試験 `an_exception_that_reaches_a_host_callback_stops_by_name` が、その文言を期待している。
  - よって、観測の探索は**境界を跨がず、戻り番地 0 に着いて終わった**と読める。「探索が境界で止まる」という見込みは、この文言とは合わない。作業者は、この読みを、記録の枠の一覧で確かめてから直す。
  - ゲストの窓の手続き・タイマー・スレッドの開始は、`fle_call_guest` を通る(`fle_win.c` の 317・319 行、`fle_thread.c` の 1006 行。戻り番地は 0xCB000000)。戻り番地 0 を積む入口が、どこに在るかは未確認。
- 例外の性質(推定。実測で確かめる)。
  - `0x0EEDFADE` は、Delphi の RTL が使う例外のコードと推定する。FL は Delphi(VCL)で作られている。リポジトリに定義は無く、試験は値だけを使う。
  - Windows の FL では、呼び出し元の `try/except`(メッセージ処理の枠など)が例外を取り、FL は続く。この VM では、その枠が探索で見つからない。
  - 推定される原因(どれかを、記録で決める)。
    - (a) 境界の扱い: 戻り番地 0 に着く入口(枠の根)が、境界として扱われていない。Windows では、探索は呼び出し元の枠まで続く。
    - (b) 枠の表の読み: 表が無い枠(`find_rf` が 0 を返す)は、葉の関数として扱われる(`fle_seh.c` の 195–201 行)。その時、戻り番地を取り違える。または、ハンドラのフラグ(UNW_EHANDLER)を持つ枠を、見落とす。
    - (c) 例外の引数: 引数 1 つ目(Delphi の例外のオブジェクト)の形の扱い、または、言語固有のハンドラの読み。
- 試験の注意: `seh_rt.rs` は、`FLEngine_x64.dll` が無い時、`SKIPPED` を出して空に通る(`flengine()`。36–50 行)。既定の場所は、リポジトリ直下の `binaries/win64/FLEngine_x64.dll`(`binaries/` は `.gitignore` の対象)。CI では、抽出した DLL は `lift/out-app/host/binaries/win64/FLEngine_x64.dll` に置かれる(`.github/workflows/agent.yml` の「抽出物を lift/out-app/host に置く」)。受け入れは `FLENGINE_PE` で場所を明示する(無ければ panic する)。
- 関連: 決定 0132(枠の処理。決定の文書はこのリポジトリに無く、`fle_seh.c` と `fle_proc.c` の注記で参照されている)。決定 0147(深さ 2 以上で切り替えない。`pm-work/findings/boot-hang-1393.md` の注記)。

## 読む物

- `lift/runtime/fle_seh.c`(全体。特に `unwind_one`(1 枠の巻き戻し)、`fle_seh_raise`(383 行から。探索の loop と 459 行の文言))
- `lift/runtime/fle_callback.c`・`lift/runtime/fle_callback.h`(`fle_call_guest`、`FLE_CB_RETURN_ADDRESS`)
- `lift/runtime/fle_seh.h`(枠の登録と着地)
- `lift/runtime/fle_proc.c`(372–390 行。`fle_imp_RaiseException` と、その注記)
- `lift/crates/fe-lift/tests/seh_rt.rs`(枠組み。`flengine()`・`wrapper`・`stops`・`an_exception_with_no_handler_in_any_frame_stops_by_name_with_the_frame_count`(499 行)・`an_exception_that_reaches_a_host_callback_stops_by_name`(505 行))
- `lift/crates/fe-lift/src/` の枠の表の生成(`grep -rn 'UNW_EHANDLER\|pdata\|RUNTIME_FUNCTION' lift/crates/fe-lift/src` で探す)
- `lift/runtime/fle_win.c`(317・319 行)・`lift/runtime/fle_thread.c`(1006 行)
- `pm-work/findings/real-user-stops-2026-11-25.md`(3.1・3.5)・`pm-work/findings/boot-hang-1393.md`(決定 0147 の注記)
- `git log --oneline -- lift/runtime/fle_seh.c`(枠の処理の変更の経緯)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 上の物を読む。観測の読み(境界を跨がず、戻り番地 0 で終わった)を、コードで確かめる。
2. **試験を先に、red で書く**(`seh_rt.rs` の枠組みと、`CALL(...)`・`wrapper` の形に合わせる)。
   - 観測の形: 境界を跨がず、戻り番地 0 に着く経路で、例外が取られないこと。
   - (a) の形: 窓の手続きの中で `RaiseException(0x0EEDFADE, 1, …)` を投げ、その呼び出し元の枠(ハンドラを持つ)が取る形。今の版は、境界の文言で止まる(red)。
   - (b) の形: 枠の表が無い関数を、探索の途中に挟む経路。
   - 期待値は、Windows の枠の処理(x64 の例外処理の記述)に従う。推定を期待値にしない。
3. **記録を厚くする**(次の実機の停止で、原因が分かるように。ビルドは約 35 分なので、1 回のビルドに、実験と記録をまとめる)。
4. **直す**。実測の原因に従う。
   - (a) なら、境界の記録(`fle_call_guest` が保存する `entry_sp` など)を使い、境界の先の呼び出し元の枠の探索を続ける。Windows と同じ順序で探す。
   - (b) なら、枠の表の読みを直す。(c) なら、引数の扱いを直す。
   - 推定が外れたら、実測で分かった本当の原因を報告に書く。
   - 深さ 2 以上の runtime 枠の状態に気をつける(決定 0147 の注記)。
5. **止まる時の記録を厚くする**(新しい環境変数・`#ifdef`・ビルド変数は足さない)。
   - `fle_seh.c` の 459 行の文言の先頭は、そのまま残す(記録の指紋は先頭 80 字で取るため。試験の期待語も残す)。後ろに足す。
   - 探索した枠の数。各枠の戻り番地(先頭 12 件)、各枠が枠の表を持つか(`has_rf`)、ハンドラの有無(UNW_EHANDLER)。
   - 探索の終わりの理由(戻り番地 0・上限・境界)。
   - 例外の引数 1 つ目が、オブジェクトへのポインタなら、その VMT の ClassName(Delphi の ShortString。読める時だけ)。
   - 長さは `engine.exit.stop.lastFle` に入る 400 字まで(`app/shared/perf.ts` の `PERF_EXIT_LAST_FLE_MAX`)。超える分は落ちるので、重要な物を前に置く。
6. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && FLENGINE_PE="$PWD/lift/out-app/host/binaries/win64/FLEngine_x64.dll" cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test seh_rt && cargo test --locked --manifest-path lift/Cargo.toml -p fe-build && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(DPR 1。`welcome-flow.mjs` の 4 行は流れ A・B・C・D の順):
- 起動確認(`boot.json`)が engine-idle で、`elapsedMs` が 12,500 以下。
- `seh_rt` が実機の DLL で走り、全て通る(新しい試験を含む。SKIPPED で通っていないこと)。
- `fe-build` の試験が通る。
- 流れ A〜D の 4 行が在り、各行が `error` null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。流れ B と D は `welcomeClosed` が true。4 行の `preBootBytes` の最大が 314,368,711 以下。`boot-bundle.mjs` は `preBootBytes` を出さないので、流れの行で見る。
- `npm test --prefix app` が通る。
- 判定の `node -e` は、合成データ(通る組と落ちる組)で先に確かめてから使う。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く。
  1. 観測の読み: 記録の枠の一覧の行(境界を跨いだか、戻り番地 0 で終わったか)。
  2. 原因の特定(実測と推定を分けて)。
  3. 直した方式と、その理由。
  4. 試験の red と green の出力の行。
  5. 止まる時の文言の変更(前後)。
  6. 変えたパス(`lift/` を変えると main の deploy が動く)。
  7. 流れ A〜D の 4 行と、`elapsedMs`・`preBootBytes` の値。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 直しは既定の動作として入れる。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python と OCR は使わない。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 4 時間 30 分で、受け入れが通らなくても、作業ツリーを commit して、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、その行を報告に書いて commit して終える(再試行で続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- `blocked_by` は無い。
- `#1357`(Welcome 窓の回帰。流れ A〜D の道具は、そこで入った)。
- `#1405`(FL の操作の全数の探索)が、同じ枠の処理で止まる場所を見つけうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{167}}`(`RegOpenKeyExW` の空の要素)とは、`lift/runtime/fle_reg.c` で触る場所が違う。
