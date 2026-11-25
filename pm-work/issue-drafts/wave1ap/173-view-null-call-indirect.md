---
draft: "173"
title: "View メニューの項目(View > #11 > #1)で、null の関数の呼び出し(indirect call to 0)で engine が止まる: 呼び出し元と、null になった理由を特定して直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify", "topbar"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

View メニューの項目 `View > #11 > #1`(道具の id `View/11/1`)を踏むと、engine が `fle: indirect call to 0 is not a known function entry` で止まる。null の関数を呼んだ時に、止まらなくするか、止まるなら、呼び出し元と理由が記録に残るようにする。理由を特定して直す。

この Issue の目標は次の 3 つ。
1. `View/11/1` を DPR 1 で踏んで、止まらない(`ok`)こと。直せない時は、原因を特定し、停止の記録に呼び出し元と最も近い関数の名前を出すこと。
2. `fle_dispatch_miss` の停止の記録に、呼び出し元の番地・レジスタ・最も近い名前つきの関数を入れること(他の停止の原因の特定にも使う)。
3. null の関数の呼び出しを、Windows の挙動(アクセス違反の例外)に沿って扱うかを検討し、決めること。採るなら、ゲストの例外として扱う(ゲストの `try/except` が取れば続く)。

## 背景

- 道具の実測(`pm-work/findings/crash-hunt-2026-11-25.json` の `View/11/1`・DPR 1)。停止の文言は `fle: indirect call to 0 is not a known function entry`(abortKind `fle_dispatch_miss`)。その直前の記録(`logTail`)に、`[err] missing-path: LoadLibraryW 126 "ILSvg_x64.dll"` がある(`ILSvg_x64.dll` の読み込みが失敗。126 は `ERROR_MOD_NOT_FOUND`)。開いていた窓は上部バーの部品(`TWPControl` の多数。`SpacePanel`・`PositionToolbar`・`TimePanel`・`PatternPanel`・`MonitorPanel`・`CPUPanel`・`MenuPanel`・`ViewPLBtn` など)。
- 同じ停止の別の記録: `#1395`(下書き 154。open)。英語以外の言語で起動すると、同じ組み合わせ(`LoadLibraryW 126 "ILSvg_x64.dll"` の後の `indirect call to 0`)で止まる。#1395 の記録では、この bundle が `ILSvg_x64.dll` を lift していないことが原因とされている。
- 推定: この停止は #1395 と同じ原因で、読めなかった DLL の関数の表が NULL のまま呼ばれた。ただし、道具の記録には、呼び出し元の番地も、最も近い関数の名前も無いので、確かめられていない。
- `lift/runtime/fle_dispatch.h` の 47 行付近の `fle_dispatch_miss`: 既知の関数の入口でない値を、推測せずに止める(`abort()`)。その前に、thunk の確認(`fle_win_try_thunk`)をする。
- Windows では、null の関数を呼ぶとアクセス違反(`EXCEPTION_ACCESS_VIOLATION`)が起きる。ゲストが `try/except` を持つ時は、それが例外を取って続くことがある。この VM では、枠の探索(`lift/runtime/fle_seh.c`。`fle_seh_raise` の 527 行付近)が、ゲストの例外を扱える。
- 関係: `#1395` が閉じると、この経路が止まらなくなる見込み。閉じた後に、まず `View/11/1` を踏み直して確かめる。

## 読む物

- `lift/runtime/fle_dispatch.h`(`fle_dispatch_miss`・`fle_jumptab_miss` と、thunk の注記。40〜60 行付近、170〜180 行付近)
- `lift/runtime/fle_win.c`(`fle_win_try_thunk`・`call_wndproc`。237 行と 268 行付近の注記)
- `lift/runtime/fle_seh.c`(`fle_seh_raise` 527 行付近)・`lift/runtime/fle_seh.h`
- `lift/runtime/fle_proc.c`(`fle_imp_RaiseException` 379 行付近)
- `lift/runtime/fle_load.c`・`lift/runtime/fle_vfs.c`(`LoadLibraryW` の失敗の経路。`ILSvg_x64.dll`)
- `pm-work/findings/crash-hunt-2026-11-25.json`(`View/11/1` の `logTail` の全体)
- `pm-work/issue-drafts/wave1ad/154-language-start.md`(#1395 の原因の記録)
- `tools/crash-hunt/hunt.mjs`(`--only` の仕様は `#{{172}}` と同じ。main に在ればそれを使う)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. main を取り込み、`#1395` が閉じているかを `git log` で確かめて報告に書く。その上で、道具で `View/11/1` を DPR 1 で踏む(`bash tools/crash-hunt/run.sh --only View/11/1 ...`)。停止するなら、停止の文言を報告に書く。
2. 診断を足す(試験を先に書く)。`fle_dispatch_miss` の停止の文言を、次の形にする。
   `fle: indirect call to <値> is not a known function entry (caller <戻り番地>, near <最も近い名前つきの関数>)`
   - 戻り番地: 呼び出しの時に積まれた、呼び出し元の番地。取れない時は `caller ?`。
   - `R[0]`〜`R[15]` の値を、停止の時だけ、stderr の 1 行(`fle: regs ...`)に出す。
   - 最も近い名前つきの関数: 戻り番地を、既存の名前の表で引く。表が無い時は `near ?`。
   - 注: 別の Issue の作業が、同じ診断を入れていることがある。main を取り込んでから確かめ、衝突したら両方を残す(同じ行を二重に出さない)。
   - `cargo test -p fe-decode -p fe-lift` に、停止の文言に `caller 0x` が入ることを確かめる試験を足す。
3. 原因の特定。停止の記録と trace から、null の関数を呼んだ呼び出し元と、null になった理由を特定する。候補は次の 3 つ。
   - (i) `GetProcAddress`・`LoadLibrary` が、ゲストの期待する関数を NULL で返した(未実装の API を NULL にしている)。
   - (ii) 仮想関数の表の、読み込まれていないプラグイン・COM のインターフェース。
   - (iii) lift の誤り(間接呼び出しの解決の誤り)。`fe-decode` の `.data` のスロットの扱いの過去の直しと、同じ型かを確かめる(`git log` で探す)。
   - 実測と推定を分けて報告に書く。
4. 直す(原因に応じて)。(i) なら、その API を実装するか、Windows の返り値で失敗させる。(ii) なら、読み込みを足す。(iii) なら、lift を直し、`cargo test -p fe-decode -p fe-lift` に試験を足す。`ILSvg_x64.dll` の扱いは `#1395` の範囲なので、同じ変更を二重に入れない。
5. null の呼び出しの扱いを決める。Windows の挙動に沿い、`fle_dispatch_miss` を、止める代わりに `RaiseException(EXCEPTION_ACCESS_VIOLATION)` と同じ扱いにするかを検討する。採る時は、`fle_imp_RaiseException` と枠の探索(`fle_seh_raise`)を使い、ゲストの `try/except` が取れば続き、取られなければ今と同じく止まる形にする。試験を足す。採らない時は、その理由を報告に書く。
6. 道具で再現し直す。`View/11/1` が `ok` になるか、止まる時は、停止の文言に `caller 0x` が入ることを確かめる。
7. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える(1 回の試行で直せない時の、受け入れの通る形: 原因の特定(`pm-work/findings/` の記録)と、診断の追加まで)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && cargo test --locked --manifest-path lift/Cargo.toml -p fe-decode && node tools/fe-lift-test.mjs && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && mkdir -p "$RUNNER_TEMP/hunt-173" && bash tools/crash-hunt/run.sh --budget 15m --out "$RUNNER_TEMP/hunt-173" --only View/11/1 --bundle "$PWD/lift/out-app/bundle/raw/fl-Oz" && node -e 'const fs=require("fs");const P=JSON.parse(fs.readFileSync(process.argv[1]+"/crash-hunt-state.json","utf8")).probes.filter((p)=>p.id==="View/11/1");process.exit(P.length>=1&&P.every((p)=>p.result==="ok"||/caller 0x[0-9a-fA-F]+/.test((p.stop&&p.stop.lastFle)||""))?0:1)' "$RUNNER_TEMP/hunt-173"
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 12,500 以下。
- `cargo test -p fe-decode -p fe-lift` が全て通る(新しい試験を含む)。
- 流れ A〜D の 4 行の判定(`#1411` と同じ)。
- `npm test --prefix app` が通る。
- 道具の再現: `View/11/1` の probe が、`ok`、または、停止の文言に `caller 0x` が入る(原因が記録に残る)。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: `#1395` が閉じていたか、再現の停止の文言(出力の行)、呼び出し元の番地と近い関数の名前、null になった理由(i)(ii)(iii) のどれか、直した方式、null の呼び出しの扱いの決定と理由、流れ A〜D の 4 行、変えたパス(`lift/` を変えると main の deploy が動く)。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 停止の扱いは、既定の動作として変える。オプトインを作らない。
- Python と OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 資料は、題名と年で書く。URL を書かない。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、その行を報告に書いて commit して終える(再試行で続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。ただし、`#1395`(ILSvg_x64.dll の real module 化)と同じ停止を扱う。#1395 の作業が先に main に入ったら、この Issue は、再現・診断・null の呼び出しの扱いに絞る。
- `#{{172}}`(版情報の探索)とは、触る場所が違う。
- `#{{174}}`(crash-hunt の続き)と `tools/crash-hunt/hunt.mjs` を共有する。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{175}}`(暫定で止めている API の分類)とは、`fle_dispatch.h` と `fle_seh.c` の扱いが重なりうる。
