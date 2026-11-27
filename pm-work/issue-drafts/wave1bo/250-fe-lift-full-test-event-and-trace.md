---
draft: "250"
title: "`fe-lift` の全体の試験が main で 2 件落ちる: `fle_event.c` の `<unistd.h>`(#1502)と `wall_fr9t` の trace の `rsp` の揺れ"
type: "Task"
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "021"
blocked_by: []
labels: ["plugin-builtin"]
size: "M"
priority: "High"
project: "signature-studio/3"
---

## 目的

`node tools/fe-lift-test.mjs`(引数なしの全体)が main で 2 件、新しく落ちる。この 2 件を直して、全体の試験の「新しい失敗」を 0 件にする。全体の試験を受け入れに持つ Issue は、この 2 件のせいで全て落ちるので、急ぐ。

1. `event` の試験 `source_uses_no_host_clock_thread_process_or_file`: `lift/runtime/fle_event.c` が host の時計・スレッド・プロセス・ファイルを使わないことを検査する。
2. `wall_fr9t` の試験 `the_scheduler_trace_on_stderr_is_the_same_in_two_runs_and_counts_one_switch_per_transfer`: 同じ試験を 2 回走らせ、stderr の `fle-thread:` の行を比べる。

## 背景

### 1. `fle_event.c` の `<unistd.h>`(コードから確かめた事実)

- `lift/crates/fe-lift/tests/event.rs` の `source_uses_no_host_clock_thread_process_or_file`(679 行付近)は、コメントを除いた `fle_event.c` の本文に禁止の語が 1 つでも在れば落とす。禁止の語には `<unistd.h>` と `printf(` が在る。
- `lift/runtime/fle_event.c` の 13 行の `#include <unistd.h>` と、`ev_trace`(138 行付近)の `write(2, ...)`(146 行)は、commit `19093e76`(#1502)で入った。
- `ev_trace` は 145 行で `snprintf(` を使う。`snprintf(` は `printf(` を含むので、`<unistd.h>` だけを消しても、この検査は次に `printf(` で落ちる(試験の判定式から読める。未実行)。

### 2. `wall_fr9t` の trace の `rsp`(推定。番地の実測は未確認)

- `lift/crates/fe-lift/tests/wall_fr9t.rs` の `trace()`(105 行付近)は、`fle-thread: ` の行を取り出し、`clock=` より後を捨てて、2 回の実行で比べる。
- commit `19093e76`(#1502)で、`lift/runtime/fle_thread.c` の `words_at`(750 行付近)が入り、`wait`・`wait-objects`・`table` の行に ` rsp <値> stack <8 語>` を出すようになった。`git log -S 'rsp %#llx stack' -- lift/runtime/fle_thread.c` で出る commit は 19093e76 だけ。
- `words_at` の `rsp` は guest の rsp(`R[4]`)である。native では guest の番地は host の `mmap(NULL, ...)` で決まる(`lift/runtime/fle_sys.c` の `fle_map_reserve`(787 行付近)。967 行で `VirtualAlloc` の `lpAddress` が 0 のとき `fle_map_reserve(0, ...)` を通る。`FLE_VMEM_ARENA` は emscripten でだけ既定で立つ)。そのため、番地は host の ASLR で実行のたびに変わり得る。
- 起票の前の記録では、同じ試験を 2 回走らせると `wait-objects` の行の `rsp` の値だけが違い、3 回とも落ちた。前の判定では通っていた。推定: 違うのは host の番地(`rsp`)。スタックの 8 語が揺れるかは確かめていない。

## 読む物

- `lift/crates/fe-lift/tests/event.rs`: `source_uses_no_host_clock_thread_process_or_file`(679 行付近)。`grep -rn 'source_uses_no_host_clock_thread_process_or_file' lift/crates/fe-lift/tests`
- `lift/runtime/fle_event.c`: `ev_trace`(138 行付近)と、その呼び出し(`CreateEventW`・`SetEvent`・`ResetEvent`・`WaitForSingleObject`)
- `lift/runtime/fle_host.h`(host の移植層。`fle_host_tid` と `fle_host_size_fits`)
- `lift/runtime/fle_thread.c`: `tlog`(194 行付近)・`words_at`(750 行付近)・`fle_thread_trace_words`(766 行)・`begin_wait`(800 行付近)・`wait-objects` の出力(1473 行付近)・`va_alloc`(1085 行付近)
- `lift/runtime/fle_thread.h`: `FLE_THREAD_REF` の hook の宣言の形(25〜29 行)。`fle_thread_trace_on` と `fle_thread_trace_words` は 234〜236 行
- `lift/crates/fe-lift/tests/wall_fr9t.rs`: `trace()`(105 行付近)と `the_scheduler_trace_on_stderr_…`(228 行付近)
- `git show 19093e76 -- lift/runtime/fle_event.c lift/runtime/fle_thread.c`(#1502 の差分)。`git log --oneline --since=2026-11-24 -- lift/runtime/fle_thread.c`(最近の変更)

## やること

1. `fle_event.c` から、`<unistd.h>` と `printf(` を両方消す。`fle_event.c` の中で host の出力を直接しない。試験の禁止の語の一覧は、削って通さない。
   - `ev_trace` の出力(`fle-event: <what> <handle> <v> thread <n><words>` の 1 行。行の形は変えない)は、`fle_thread.c` の側に置く。例: `fle_thread.c` に `fle_thread_trace_event(const char *what, uint64_t h, uint32_t v)` を足し、`fle_thread.h` に `FLE_THREAD_REF` で宣言する。`fle_event.c` は、その hook が在るときだけ呼ぶ(今の `fle_thread_trace_on` と同じ書き方)。`fle_thread.c` は既に `write(2)` と `snprintf` で trace の行を出している。
   - 別の形も可。ただし `fle_event.c` に `write(`・`printf(`・`<unistd.h>` を書かない。runtime を guest の状態だけで動かす、という `fle_event.c` の先頭の注釈を保つ。
2. `wall_fr9t` の trace の `rsp` を直す。まず、揺れの原因を実測で確かめる。
   - 確かめ方の例: `wall_fr9t` の `run()` が使う scenario の実行ファイルを、`setarch "$(uname -m)" -R`(ASLR を止める)を付けて `ping_pong` で 2 回走らせ、`fle-thread:` の行を比べる。2 回が同じになれば、揺れは host の番地による(実測)。
   - 直し方(優先): trace の行に host の番地を出さない。`rsp` は guest の値にする(例: 現在のスレッドのスタックの先頭からの差。`stack_base` が 0 の main thread の扱いを決める)。または、`rsp` を行から落とす。`words_at` を直せば、`wait`・`wait-objects`・`table` の行と `fle-event:` の行が、同じ方法で直る。
   - スタックの 8 語(`fle_ld(rsp + 8k)`)にも host の番地(ヒープ・TEB など)が入るかを確かめる。入るなら、同じ方法で直す。
   - 試験の `trace()` の比較を弱めて(`rsp` を外して)直すのは、番地が host のものだと実測できた時だけ。その場合は、試験に理由の注釈を書く。
   - 行の数(`a.err.len() == 4003` の試験)と、stdout(guest の出力)は変えない。
3. 直した後、`node tools/fe-lift-test.mjs`(引数なし)の最後の行が「新しい失敗 0 件」になることを確かめる。
4. `lift/crates/fe-lift/tests/known-failures.txt` に足して隠さない。直す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=20000?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node tools/fe-lift-test.mjs > "$RUNNER_TEMP/lift.txt" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n");process.exit(/^fe-lift の試験: .*新しい失敗 0 件/.test(L[L.length-1])?0:1)' "$RUNNER_TEMP/lift.txt" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 20,000 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/fe-lift-test.mjs` が終了コード 0 で、最後の行が「新しい失敗 0 件」。
- `bash guard/scan.sh` の指摘が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書く。
- 報告に書く: (a) 直す前の赤の出力の行(`fle_event.c must not use <unistd.h>` の行と、`wall_fr9t` の trace の差の行。違った `rsp` の 2 つの値)、(b) ASLR を止めた 2 回の比較の結果(同じか違うか)、(c) 直し方と、その理由、(d) 直した後の `node tools/fe-lift-test.mjs` の最後の行、(e) 起動確認の `elapsedMs`、流れ A〜D の 4 行、(f) `bash guard/scan.sh` の結果。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・変数名は書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- `git add` は、変えたファイルを 1 つずつ指定する(`git add -A`・`git add .` は使わない)。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 急ぐ: 全体の試験を受け入れに持つ Issue は、この 2 件で落ちる。最初に、この 2 件を直す。
- ビルドは 1 回にして、同じ bundle で繰り返し確かめて直す。長いコマンドはフォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: なし。
