---
draft: "114"
title: "DPI 240 以上(DPR 2.5 以上)で FL が起動しない障害を直す: LOGPIXELSY の値で起きる再帰の閉路を特定し、runtime の応答を Windows と同じにする"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107", "040"]
labels: ["hidpi", "render"]
size: L
priority: High
project: signature-studio/3
---

## 目的

依頼者の要求は、100%〜300% の全ての拡大率で FL が操作できて、ぼやけないこと(`#{{108}}` が全ての拡大率を検証する)。ところが、DPI が 240 以上(DPR 2.5 以上)で、FL が engine-idle に届かず、起動しない。この Issue は、その障害を直す。

この Issue の目標は、DPR 2.5 と 3.0 と、乱数 2 つ(2.25〜3.00)の全てで engine-idle に届くこと。DPR 1・1.7647・2 の結果は変わらないこと。

## 前回の試行で分かったこと

run 45253143948(2026-11-23 19:52Z〜22:49Z、約 3 時間)。判定は差し戻し。製品の差分は無く、残ったのは文書 `pm-work/findings/dpi240-stop-1354.md` だけ(artifact `agent-1354`)。

- 停止の表示: `[js] fle thread 2 failed: RangeError: Maximum call stack size exceeded`。`CreateWindowExW 0x7f990024 ex=0x80 style=0x80000000` の直後に止まる。DPR 2.5 と 3.0 の両方で同じ。
- **原因は窓の応答ではなく、lift が作った呼び出しの形**(実測)。停止時の JS の枠は 9,437 件。その中で、8 枠の閉路が 1,176 回繰り返す(9,408 枠)。
  `fle_dispatch → f_27ad527 → f_27ad54a → f_27ad56d → f_27ad590 → f_27ad5b3 → f_27ad5d6 → f_27ad5f8 → fle_dispatch → …`
- 番地は前回の build のもの。関数名は `lift/out-app/gen/wasm-func-names.tsv` で引いた。閉路の中の番地は全て引けた。底から 60 件のうち引けなかった 5 件は閉路の外。
- 閉路の入口は `f_27ad527`。底からの呼び出し: `fle_wasm_thread_entry → fle_call_guest → f_412ae0 → fle_dispatch → f_5251b0 → fle_dispatch → f_71da40 → … → f_2743af0 → f_27abd80 → f_27acd60 → fle_dispatch → f_27ad527`。`f_27abd80` は `0x27abf2b` で `f_27acd60` を呼ぶ。
- 閉路の中身(生成 C の `gen/src/m1-04.c`): `f_27acd60` は w×h の画素の平均を求めるループ(`1/N` を求め、N 個の 16 バイト要素を `addss` で積算)。後戻りは、バイト表(0x1ecf840)から飛び先のオフセットを引いて `fle_dispatch(R[0]); return;` で飛ぶ。要素は 1 個ずつ別の関数で、末尾は `f_next(); return;`。つまり、ループの 1 回の後戻りが、ネイティブの呼び出しの 1 段になり、反復数に比例して枠が増える。
- 閉路は 1 回で 7 要素を進める。1,176 × 7 = 8,232 要素で枠が尽きた(実測)。240 DPI の N は 8,100 と見込む(推定: 96 DPI の 36×36 画素が DPI に比例して 90×90 になる)。N を作る寸法の出どころ(`f_27abd80` の引数)は未特定。
- 閉路の深さは DPI で増える(推定)。216 DPI(DPR 2.25)は枠の上限に届かず engine-idle になり、240 DPI は届く(`#{{107}}` の結果と、今回の枠の数から)。
- 窓の応答は Windows と同じ(照合済み。直す応答は無い): LOGPIXELSY 240 = MulDiv(96, 250, 100)。SM_CXCURSOR 80 = MulDiv(32, 240, 96)(`fle_sysmetrics.c` の `fle_sm_scaled`)。閉路は窓の応答を読まない。
- 測定(診断の版。runner の値。製品の差分は無い): DPR 1 は engine-idle、guest 1836×1056、dpi 96。DPR 1.32(乱数)は engine-idle、guest 2424×1394、dpi 127。DPR 2.5 と 3.0 は停止。DPR 1・1.7647・2 は今回は測っていない。
- 較正: 部屋 1836×1056 を得る viewport は 2176×1092。1 回目(viewport 1836×1056)の部屋は 1496×1020。2 回目で 1836×1056 になる。道具が 2 回の起動で求める。
- 受け入れの欠陥(直した。前回の受け入れは、この欠陥で DPR に関係なく止まる形だった): 道具は `display` を常に null で出す(`#{{117}}` で `display` は Main GUI Scaling の欄になった)。canvas の寸法は `canvasDisplay`。受け入れは `x.display.w` を読んでいたので、`canvasDisplay` に直した。
- 前回の記録(12 個の戻り番地の並び)は、今回の閉路と一致しない。番地は build のたびに変わる。閉路は関数名で書く。
- 前回の診断の仕方(コミットしない): `lift/out-app/web/engine-glue.js` の `Error.stackTraceLimit` を大きくし、`fe-build bundle` だけで作り直す(約 11 分)。`fle_thread_lib.js` の `fail()` の表示を、枠の全行を出す形に一時的に変える。
- 以前の結果(`#{{107}}` の前後): LOGPIXELSY の値だけを 96 に戻すと、DPR 2.5 が engine-idle に届く。フォントの高さ単独・`GetDpiForMonitor` 単独・物理サイズの仮説は、原因でないと確認した。guest が LOGPIXELSY(90) を読む呼び出し元は 5 か所。値は guest の大域変数(`0x630e924`)に保存され、その読み出しは 7 か所。

## 読む物

- `lift/crates/fe-lift/src/tailjmp.rs`・`jumptab.rs`・`split.rs`: 飛びの変換と関数の境界。未注釈のバイト表の `jmp reg` は `tailjmp.rs` の `fle_dispatch(R[reg]); return;` になる。注釈された表は `jumptab.rs` が `switch` と `goto` にする(飛び先が関数の入口なら呼び出し)。関数の境界は `split.rs`。
- `lift/crates/fe-decode`(表の注釈。`fe_decode::jumptab`): 閉路のバイト表(0x1ecf840 付近)が注釈されない理由を確かめる。
- `lift/crates/fe-lift/tests/jumptab.rs`・`jt_cpu.rs`・`msvc_switch.rs`(表の変換の既存のテスト)
- `lift/out-app/gen/wasm-func-names.tsv`(`#{{040}}` が出す関数名の表。製品のビルドで書き出す)
- `lift/out-app/gen/src/`(生成 C。閉路は `f_27ad527`〜`f_27ad5f8` と `f_27acd60`。番地は前回の build のもの。名前で引く)
- `tools/feat-c1-hidpi/measure-size.mjs`・`tools/feat-c1-hidpi/read-manifest.mjs`(`#{{107}}` と `#{{117}}` の成果物)
- 照合済み(前回。直す必要は無い): `lift/runtime/fle_sysmetrics.c`・`lift/runtime/fle_screen.h`

## やること

1. 作業ツリーを確かめる。`.prior-work.md` が在れば先に読む(前回の作業の取り込みの結果)。取り込まれた文書は、そのまま使う。製品の差分は前回には無い。
2. 原因を直す(lift が作る呼び出しの形。窓の応答は直さない)。前回の候補:
   - 候補 A: 閉路の要素の間の飛びを、同じ C 関数の中の `goto` にする。要素の関数の境界も、閉路が 1 つの関数に入る形に変える。注釈(`fe-decode`)が閉路のバイト表を拾えるようにするか、拾えない表の `jmp reg` を `tailjmp.rs` で飛び先への `goto` にする。どちらにするかは、読んだ結果で選ぶ。
   - 候補 B(未検証): 末尾呼び出しを wasm の `return_call` にして、ネイティブのスタックを伸ばさない。JSPI・longjmp・`fle_call_guest` との関係を確かめてから採る。製品全体のコンパイル設定の変更になる。
   - どちらでも、閉路を隠す回避(深さの上限・N の抑え・ループの C での置き換え)は採らない。
   - `fe-lift`・`fe-decode` を変えたら、`cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift -p fe-decode` も通す。
3. 任意: N の出どころ(`f_27abd80` の引数)を応答まで辿る。直すのに必要ではない。未特定のままでもよい。
4. 受け入れを走らせる。通れば commit する。
5. 通らなければ、判定の出力の末尾と、DPR ごとの行(状態・guest・dpi・canvas と canvasDisplay)を報告に貼り、作業ツリーを commit する。次の試行は、その続きから行う。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.7647,2,2.5,3 --random 2 --random-range 2.25,3 > "$RUNNER_TEMP/size.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const R=L.filter((x)=>"dpr" in x);const eq=(x,y)=>Math.abs(x-y)<0.0001;const v=R.length?R[0].randomDprs.values:[];const want=[1,1.7647,2,2.5,3,...v];const ok=R.length===7&&v.length===2&&v.every((d)=>d>=2.25&&d<=3)&&want.every((d)=>R.some((x)=>eq(x.dpr,d)))&&R.every((x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&Math.abs(x.canvas.w-x.canvasDisplay.w)<=1&&Math.abs(x.canvas.h-x.canvasDisplay.h)<=1);process.exit(ok?0:1)' "$RUNNER_TEMP/size.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の 7 行(DPR ごとの行)の生の値を、上の node -e が読む。
- (a) 固定の一覧(1・1.7647・2・2.5・3)と乱数 2 つ(2.25〜3.00)が全て在る。全行が正常な起動(`error` が null・`state` が engine-idle・`standIn` が false)。DPR 2.5 と 3.0 の行が engine-idle に届く。
- (b) 全行で、guest の w と h が、部屋 × DPR の round の ±1 画素。dpi が round(96 × DPR) の ±1。canvas と canvasDisplay(`canvas[data-desk="1"]` の矩形 × DPR の round)が ±1 画素で一致する(`#{{107}}` の受け入れの (i)〜(iii) と同じ式)。DPR 1・1.7647・2 の行も、この式を満たす(結果が変わらない)。
- (c) 乱数の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行に、特定の拡大率・DPI・DPR の値(`125`・`150`・`168`・`169`・`175`・`176`・`1.75`・`1.7647`・`0.5667`)が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。
- 単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止時の枠の閉路と、関数名への対応(表で引けた物と、引けなかった物の数)、閉路の入口の関数名、閉路を作っていた飛びの変換(`tailjmp.rs`・`jumptab.rs`・注釈のどれか)、直した箇所(ファイルと行)、DPR ごとの表(状態・guest・dpi・canvas と canvasDisplay)、DPR 1・1.7647・2 の結果が変わらないことの確認、特定の値を用意していないことの確認(source の検査の grep の出力。一致 0 件)。閉路の入口が特定できなければ「未特定」と書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の DPI・DPR の値(240・260・2.5 など)で分岐しない。Windows の式で導く値だけを使う。
- 閉路を隠すだけの回避(再帰の深さの上限を置く、窓の大きさを固定の値にする、DPR を丸めて別の値にする など)で代えない。閉路の原因を直す。
- 拡大して表示する版を作らない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(調査と検証にだけ使う)。

## 作業の進め方

- engine の作り直しは約 30〜35 分、bundle は約 11 分。ビルドは、フォアグラウンドで、シェルツールの `timeout` 引数に大きい値(例 3600000。上限 5400000)を渡して待つ。背景で起動して待たない。
- 前回の試行は、engine を 2 回・bundle を 2 回作った(診断のため)。今回は、直す → engine → 受け入れ、の順に、ビルドを数回に抑える。
- 開始の時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても作業ツリーを commit し、報告を書いて終える。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: `#{{107}}`(測定の道具と、DPI の式)、`#{{040}}`(関数名の表。`#{{040}}` が出す)。どちらも閉じている。
- `#{{113}}`(FL の UI の拡大の実測)と、窓の経路(`fle_win.c`)を変えうる。衝突したら pull して直す。
- `#{{108}}`(動的な変化と全ての拡大率)は、この Issue の後に行う。
- #1349(音が途切れる現象)は、この Issue の後の物理画素の画面で測る。
