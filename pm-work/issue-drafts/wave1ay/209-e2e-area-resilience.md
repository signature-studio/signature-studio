---
draft: "209"
title: "E2E の領域 resilience: ランダムな操作の列・アイドル・窓の開閉・プロジェクトの開閉・再読み込みの繰り返し・停止からの回復の、耐久のシナリオを足す"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "133"
blocked_by: ["199"]
labels: ["verify", "render"]
size: XL
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(2026-11-26。原文): 「そうではなく、存在する機能について、考えうる限り全ての E2E テストを追加してください。」

依頼者は、FL が「非常にクラッシュしやすい」と指摘した。この領域(resilience)は、その原因を探す走査ではなく、耐久の機能(長く動かしても、操作を繰り返しても、止まらず、止まった時は分かり、回復できること)を、期待する結果つきのシナリオで確かめる。最小のシナリオ数は 40 本(`tools/e2e/features/resilience.min` と同じ)。目標は次の 2 つ。
1. 下の一覧の耐久の機能を、期待する結果(停止 0・例外 0・消えない表示 0・メモリが増え続けない・回復)で確かめるシナリオで覆うこと。
2. 止まった場合は、止まった理由(エンジンの停止の報告)を、報告に書けること。

## 背景

- #{{199}}(E0)が作る物: `defineScenario`・`run.mjs --area`・`--list`・`--shard`・`--budget`・`--repeat`(同じシナリオを n 回)・見本 3 本(`tools/e2e/scenarios/resilience/`)・`lib/detect.mjs` の検出(例外・停止・消えない表示・失敗した要求)。E0 が閉じてから始める。
- 今の検出は、`pageerror`・`console.error`・失敗した要求・消えない表示・エンジンの停止(`running` の 90 秒を超える物を含む)を数える。この領域は、長い時間と多くの操作で、同じ検出が 0 のままであることを確かめる。
- 停止の理由は、`lib/stop.mjs`(製品の `app/src/perf/exit-message.ts` を使う)で読む。
- crash-hunt(`tools/crash-hunt/`)は、メニューの項目を 1 つずつ踏んで停止を見る走査。この領域では、それを置き換えず、ランダムな操作の列という別の形で、耐久を確かめる。

## 読む物

- `tools/e2e/`(E0 の後の姿。lib/detect.mjs・lib/stop.mjs・lib/guest.mjs)
- `tools/crash-hunt/hunt.mjs`・`tools/crash-hunt/run.sh`(走査の方式。停止の見方)
- `tools/boot-hang/`(起動の止まりの見方)・`tools/boot-check/boot-bundle.mjs`
- `pm-work/findings/real-user-hangs-2026-11-24.md`・`pm-work/findings/real-user-exits-2026-11-24.md`・`pm-work/findings/real-user-stops-2026-11-25.md`(実機の止まりの記録。種類の参考)
- `pm-work/findings/boot-hang-1393.md`・`pm-work/findings/dedzii-open-stop-1421.md`(停止の具体例)
- `tools/e2e/features/generated/resilience.tsv`(E0 が作る一覧。無い時は、この領域の表で補う)・`pm-work/findings/e2e-program.md`

## やること

1. **機能の表** `tools/e2e/features/resilience.tsv`(手で書く。見出し行は E0 が作る)。機能ごとに 1 行。列は `id`・`kind`・`name`・`expected`・`how`。
2. **シナリオ**(`tools/e2e/scenarios/resilience/`。1 本 1 ファイル)。次の全てを、機能ごとに 1 本以上。
   - **ランダムな操作の列**: 種を固定した乱数で、操作の列を作る(1 列 200 操作。10 列。種は、シナリオの ID から決める)。操作の種類は、マウスの左右クリック・ダブルクリック・ドラッグ・ホイール・キー(F キー・矢印・Escape・Enter・Ctrl の組み合わせ)。期待: 列の全体で、停止 0・例外 0・消えない表示 0・失敗した要求 0。列ごとに、窓の表が取れ続ける(`dumpRetries` の上限内)。
   - **アイドル 30 分**: 操作無しで 30 分待つ(条件で待つ。上限は 35 分)。期待: 途切れ 0(音が出る状態の時)・メモリの使用量が増え続けない(開始から 30 分後の値が、10 分後の値に対して、基準の比を超えない)・停止 0。
   - **窓を開閉 100 回**: 同じ窓(Channel Rack の窓を F キーで)を、開いて閉じるを 100 回。期待: 毎回、窓の表に出て、消える。停止 0。メモリが増え続けない。
   - **プロジェクトを開いて閉じる 50 回**: demo の曲を 1 本、開いて、閉じる(New で置き換える)を 50 回。期待: 毎回、開く・置き換わる。停止 0。
   - **再読み込みの繰り返し 20 回**: ページを再読み込みし、起動が完了して、窓の表が取れる。期待: 毎回、起動が完了する(`engine-idle`)。停止 0。
   - **タブの停止と再開**: ページのタブを非表示にして、一定の時間の後に表示する。期待: 表示に戻った後、描画と操作が続く。停止 0。
   - **engine が止まった時のページの振る舞い**: 停止を起こすための操作は使わない(製品を変えない)。止まった状態は、既知の欠陥(`expectFail`)の物を使って確かめる。期待: 止まったことが分かる(エンジンの状態が `exited` か `error` として、ページに出る)・再読み込みで回復する(起動が完了する)。
3. **判定**: 各シナリオは `ok` と、測った値(`results`: 操作の数・停止の数・消えない表示の数・メモリの値の時系列の要約・再読み込みの回数)を返す。`notes` に、判定の根拠を 1 行で書く。
4. **表示**: 既定は DPR 1 だけ(耐久の物は時間が長い)。ランダムな操作の列と、再読み込みの繰り返しだけ、乱数の DPR を 1 点足してよい(理由を `notes` に書く)。
5. **時間**: アイドル 30 分の物は、1 本で 35 分まで使う。受け入れの 1 シャード(1/8)の中に入る物は、1 本だけにする(`--budget 40m` の中に収める)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --area resilience --list > "$RUNNER_TEMP/list.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);process.exit(r.area==="resilience"&&r.scenarios>=40?0:1)' "$RUNNER_TEMP/list.jsonl" && node tools/e2e/run.mjs --area resilience --shard 1/8 --displays 4k-dpr1 --budget 40m > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===1&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.length>0&&r.scenarios.every((s)=>s.skipped!==true))?0:1)' "$RUNNER_TEMP/e2e.jsonl"
```

判定の基準:
- 受け入れの式が通る(流れ A〜D・`npm test --prefix app`・`--list` の resilience の scenarios が 40 以上・1 シャード(1/8)の実行が終了コード 0。`skipped` が無い)。
- 上の一覧の全ての項目に、1 本以上のシナリオがある。
- ランダムな操作の列(10 列)の全てで、停止 0・例外 0・消えない表示 0・失敗した要求 0。この値を、列ごとの表で報告に書く。
- アイドル 30 分の物で、途切れ 0(音が出る状態の時)と、メモリの増加の比を、測った値で報告に書く。
- 停止が起きた時は、その停止を `engineStop`(停止の理由・最後の `fle:` の行)で報告に書き、印 `expectFail: 'new'` を付ける(新しい停止の一覧)。
- 網羅率: 全シャードを合わせて、この領域の機能の 90% 以上が、`ok` かつ `expectFail` 無しのシナリオで覆われる(`judge-coverage.jq`。PM が判定する。報告に覆った数を書く)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に、(a) 追加したシナリオの ID と、各 `covers`、(b) ランダムな操作の列の結果の表(列・操作の数・停止・例外・消えない表示・失敗した要求)、(c) アイドル・開閉・再読み込みの測った値、(d) 印 `expectFail` と `new` の一覧(停止の理由を含む)、(e) 通らなかった物と理由。

## 制約

- 製品(`app/`・`lift/`)は変えない(読むだけ)。製品の欠陥は、シナリオに `expectFail: 'new'` を付けて記録する。直すのは PM が起票する別の Issue。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。テストのための分岐を製品に入れない。
- 領域の作業で作るファイルは、`tools/e2e/scenarios/resilience/`・`tools/e2e/features/resilience.tsv`・`tools/e2e/lib/resilience.mjs`(必要な時だけ)に限る。
- ランダムな操作の列の乱数は、種を固定する(同じ種で、同じ操作の列になる)。種は、シナリオの ID から決め、報告に書く。
- Python を使わない。OCR を使わない。
- ベンダー名・Cloudflare の識別子(アカウント・バケット・ホスト名・アクセス制御の値)を、ファイルに書かない。
- 期待値に、実行時の値(DPR・DPI・窓の位置)を定数で書かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。アイドルは、時間の経過を条件(時刻の差)で確かめる。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。GitHub に、Issue・ラベル・コメントを書かない。

## 作業の進め方

- 開始の時刻を、最初に `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数に 5400000 を渡して待つ。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 長い物(アイドル 30 分・ランダムな操作の列)は、短い物(開閉・再読み込み)の後に足す。1 回で全部は動かさない。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{199}}(E0)。E0 が閉じるまで起動しない。
- 他の領域(#{{200}}〜#{{208}}・#{{210}}〜#{{212}})と、`coverage.json` の母数を共有するが、ファイルは共有しない。
- 停止の修正の Issue が後で来た時は、その Issue に対応するシナリオを 1 本以上足す(この領域の耐久の範囲)。
